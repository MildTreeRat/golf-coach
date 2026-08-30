"""Strike detection and clip-to-clip offset: what the bay's audio can be asked. [M11 P3]

Two functions, deliberately separate, because they answer questions of very different difficulty:

- `detect_strikes` says *when the transients are* in one clip. It is the hard half — a simulator
  bay produces four transients per shot and the ball is not reliably the loudest of them — so it
  returns every candidate and lets the caller decide, rather than returning "the impact".
- `offset_between` says *how far apart two clips' clocks are*, and identifies no transient at all.
  It is the robust half: every transient appears in both recordings, so cross-correlating the two
  envelopes recovers the offset without anyone having to know which peak was the ball. M11 P6's
  synchronization needs only this, which is why it does not depend on strike identification.

numpy lives here and nowhere downstream. This module sits behind the `audio` extra alongside the
decoder; what crosses into `analysis/` is a sample index and a confidence — plain data on a
pydantic model (`contracts/audio.py`), the same seam pose crosses at (ADR-008).

**Thresholds are clip-relative, per ADR-013.** Every decision below is expressed against the
clip's own flux floor (a robust z-score), never against an absolute amplitude: a phone two metres
from the mat and one six metres away record the same swing at very different levels, and an
absolute threshold would silently mean something different on each.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Any

import numpy as np

from golf_coach.contracts.audio import AudioStrike

# --- the envelope -----------------------------------------------------------------------------

# Hop and window for the short-time spectral flux everything here reads. 5 ms is the hop M11 P0
# measured the corpus with (docs/M11_ACOUSTIC_SYNC.md §E4-E5), kept so the numbers in this file
# and in that document describe the same signal; it is a third of a frame at 60 fps, which is
# finer than any consumer needs. The window is 1024 samples at the corpus's 48 kHz — expressed in
# seconds so a differently-sampled clip gets the same *time* resolution, not the same array shape.
#
# A shorter window was tried and rejected: at a 2.5 ms hop over a 256-sample window the envelope
# of a real strike breaks into a dozen jittering sub-peaks (measured on 2026-08-23/2 face-on) and
# the ball-versus-screen structure this module exists to expose stops being legible.
_HOP_S = 0.005
_WINDOW_S = 1024 / 48_000

# STFT frames per block. The whole-clip matrix would be len(samples)/hop x window floats — 134 MB
# on the corpus's longest clip (80 s) and unbounded on a longer one — where the envelope it
# produces is a few tens of kB. Blocking keeps the peak allocation flat.
_FRAME_BLOCK = 4096

# --- strike detection -------------------------------------------------------------------------

# How far above the clip's flux floor a peak must stand to be listed at all, in robust z (median
# absolute deviations from the median). Measured over the 22 clips of the 2026-08-23 session: the
# ball and screen strikes land at z = 113-599, while 20 s of pure room tone — and of amplitude-
# modulated noise standing in for a voice — never exceeds z = 4.4. So this floor is an order of
# magnitude below every real strike and roughly twice the loudest thing an empty bay produces. It
# is set low on purpose: this function's job is to hand over candidates, not to pick one.
_MIN_PROMINENCE_Z = 8.0

# Peaks closer together than this are one transient seen twice. A real strike's flux does not
# arrive as a spike but as a burst that stays elevated for tens of milliseconds, and 50 ms is the
# largest suppression that still keeps the ball and the screen apart: §E5 measures that gap at
# 85-145 ms. The deliberate cost is that the club-mat transient, 15-20 ms after the ball, is
# absorbed into the ball's candidate rather than listed separately — which is the right trade,
# because no consumer in this milestone has a question that needs the mat.
_MIN_SEPARATION_S = 0.050

# The z at which `confidence` reads 0.5. Chosen from the same 22-clip measurement so the numbers
# it produces are readable rather than arbitrary: the real strikes (z = 113-599) land at
# 0.85-0.97, the listing floor above lands at 0.29, and the loudest peak in an empty bay near 0.18.
_CONFIDENCE_HALF_Z = 20.0

# --- clip-to-clip offset ----------------------------------------------------------------------

# How well the two flux envelopes must line up before an offset is reported, and by how much the
# winning lag must beat the best rival elsewhere in the clip. Measured whole-clip over the eleven
# 2026-08-23 bundles: the winning lag scores r = 0.838-0.923 against a best rival of 0.125-0.672,
# and P0 measured r = 0.59-0.85 against 0.15-0.47 on the +/-1.5 s windows it used. Both sets clear
# these thresholds comfortably; a pair that does not is declined rather than guessed (ADR-010 §2).
_MIN_CORRELATION = 0.45
_MIN_MARGIN = 0.10

# How far from the winning lag a peak has to be before it counts as a *rival* rather than as the
# shoulder of the same match. Wider than §E5's 85-145 ms ball-to-screen gap on purpose: that gap
# puts a real secondary peak either side of every correct answer — P0 saw it at +/-6-8 frames in
# the correlation function — and a guard narrower than it would make every honest offset look
# ambiguous against its own alias.
_RIVAL_GUARD_S = 0.25

# A lag whose overlap is a sliver of one clip can correlate on almost nothing. Requiring half of
# the shorter envelope keeps the comparison a comparison; the one-second floor is for the
# degenerate case where "half" is still too little to mean anything.
_MIN_OVERLAP_FRACTION = 0.5
_MIN_OVERLAP_S = 1.0


@dataclass(frozen=True)
class ClipOffset:
    """How far apart two clips' clocks are, with the evidence that says so.

    Richer than the bare `float` this function was planned to return, and for a reason ADR-010 §2
    makes structural: the caller (M11 P6) has to *write down* why it trusted or declined an
    offset, and a lone number can say neither how good the match was nor how close the runner-up
    came. The two correlations are the whole audit trail.
    """

    seconds: float
    """`t_a - t_b` for any event visible in both clips: add it to a time in `b` to get the same
    instant on `a`'s clock. Positive means `b` was started earlier, so the event lands later in
    `a`'s own timeline."""

    r: float
    """Pearson correlation of the two flux envelopes at the winning lag, over their overlap."""

    runner_up_r: float
    """The best correlation at any lag more than `_RIVAL_GUARD_S` away — how alone the winner is.

    A high `r` with a `runner_up_r` right behind it is not a strong match; it is an ambiguous one.
    """


def detect_strikes(
    samples: Sequence[int] | np.ndarray[Any, Any],
    rate: int,
    *,
    min_prominence_z: float = _MIN_PROMINENCE_Z,
    min_separation_s: float = _MIN_SEPARATION_S,
) -> list[AudioStrike]:
    """Every transient in one decoded clip, loudest first. [M11 P3]

    **The first entry is not the ball.** A simulator bay produces four transients per shot —
    club-ball contact, the club hitting the mat 15-20 ms later, the ball hitting the impact screen
    85-145 ms after that, and the simulator's own ball-flight audio a moment later — and across
    the 22 clips of the 2026-08-23 session the screen strike outranks the ball on roughly half of
    them. Any rule of the form "take the loudest peak" is therefore wrong about half the time,
    which is exactly why this returns the list rather than a verdict.
    `analysis.phases.candidate_downswings` made the same choice for the same reason: seeing how
    many candidates there were beats discovering by eye that the wrong one was taken.

    Ordering is by `prominence`, descending — the ranking, not the timeline. Sort by `sample` if
    you want time order.

    **A floor relative to the clip's loudest transient works and must not land on its own.** M11 P9
    specified one to drop the quiet onsets a few frames ahead of the ball; it was built on
    2026-08-30, measured over all 30 cached clips (it wants 0.25 of the clip maximum — precursors
    run 0.02-0.10 and the ball never falls below 0.61), and it picks the ball on every one of them.
    It was reverted anyway, because watching the renders found a second defect underneath it: on the
    four down-the-line clips carrying a video edit list (§E2 — bundles 1, 7, 9, 11), the video
    decode ignores a 90 ms presentation offset that the audio decode honours, and the precursor
    error was accidentally cancelling most of it. Removing the precursor alone leaves the anchor
    right in *audio* time and 6 frames late in *video* time, which is the time tau=2 is measured in.
    Fix the edit list first, then floor the candidates. Evidence, frame by frame, in
    docs/M11_ACOUSTIC_SYNC.md §Addendum.

    `frame` is left `None` on every strike: this function is handed a waveform and a sample rate
    and has never seen the video, so it cannot know the fps (`contracts/audio.py`). The caller
    that holds the manifest fills it in.

    Returns an empty list for a clip with no transient above the floor — which is a *result*, and
    the one M11 P5 reads: a rehearsal swing makes no crack. It is also what a clip too short to
    hold two analysis windows returns, and a clip of digital silence, where there is no floor to
    measure anything against.
    """
    if rate <= 0:
        raise ValueError(f"rate must be positive, got {rate}")

    envelope, hop, _ = _flux_envelope(samples, rate)
    if envelope.size == 0:
        return []

    median = float(np.median(envelope))
    # Median absolute deviation, scaled to be a standard deviation on normally distributed data.
    # Robust rather than the plain sd because the thing being measured — a handful of enormous
    # transients — is precisely what would inflate an sd and then hide itself behind it.
    scale = float(np.median(np.abs(envelope - median))) * 1.4826
    if scale <= 0.0:
        # Digital silence, or a flux envelope flat enough to have no floor. There is no
        # clip-relative threshold to apply, and an absolute one would mean something different on
        # every clip (ADR-013), so decline rather than invent one.
        return []

    z = (envelope - median) / scale
    separation = max(1, round(min_separation_s * rate / hop))

    strikes: list[AudioStrike] = []
    chosen: list[int] = []
    for index in np.argsort(z)[::-1]:
        peak = int(index)
        if z[peak] < min_prominence_z:
            break
        if any(abs(peak - other) < separation for other in chosen):
            continue
        chosen.append(peak)
        strikes.append(
            AudioStrike(
                # The first sample of the analysis window whose new energy produced this rise.
                # Calibrated against the raw waveform on all 22 clips of the 2026-08-23 session:
                # this convention runs +18 ms late on average (sd 25 ms) against where the
                # amplitude actually starts climbing, most of that spread being the crudeness of
                # the reference rather than of the estimate. The bias is a property of the window,
                # so it is the same in both views of a bundle and cancels in any comparison
                # between them — which is the only comparison M11 makes.
                sample=(peak + 1) * hop,
                confidence=float(z[peak] / (z[peak] + _CONFIDENCE_HALF_Z)),
                prominence=float(envelope[peak] - median),
            )
        )
    return strikes


def offset_between(
    a: Sequence[int] | np.ndarray[Any, Any],
    b: Sequence[int] | np.ndarray[Any, Any],
    rate: int,
    *,
    max_lag_s: float | None = None,
    min_correlation: float = _MIN_CORRELATION,
    min_margin: float = _MIN_MARGIN,
) -> ClipOffset | None:
    """How far apart two recordings of the same moment are, or None if it cannot be told. [M11 P3]

    Cross-correlates the two clips' flux envelopes and returns the lag that lines them up. It
    identifies no transient and does not care which peak was the ball: every sound in the bay
    reaches both phones, so the *pattern* is shared even when no single event in it can be named.
    That is what makes this the robust half of the module, and what lets M11 P6 use it without M11
    P5 existing.

    Both clips must be at the same `rate` — decode through `audio/ffmpeg.py` and they are.

    `max_lag_s` bounds the search, and a caller with any prior at all should pass it. Measured
    whole-clip over the eleven 2026-08-23 bundles, this recovers the offset to within 0-20 ms
    (0-1.2 frames at 60 fps) on ten of them — and gets bundle 8 confidently, decisively wrong: its
    80-second down-the-line clip holds *two* shots, and the face-on clip's single shot matches the
    second of them at r = 0.923 where the correct answer scores 0.672. The margin rule below does
    not catch that and no margin rule can, because both answers are real matches; only a bound on
    the search does. Pass one derived from the pose anchors you already have.

    Returns None when the winning lag is weak in absolute terms, when a rival elsewhere comes
    within `min_margin` of it, or when no lag leaves the two clips enough overlap to compare — an
    offset that cannot be trusted is not reported at all (ADR-010 §2).
    """
    if rate <= 0:
        raise ValueError(f"rate must be positive, got {rate}")

    env_a, hop, _ = _flux_envelope(a, rate)
    env_b, _, _ = _flux_envelope(b, rate)
    if env_a.size == 0 or env_b.size == 0:
        return None

    hop_s = hop / rate
    lags, r = _correlation_by_lag(env_a, env_b, min_overlap=_min_overlap(env_a, env_b, hop_s))
    if max_lag_s is not None:
        r[np.abs(lags) * hop_s > max_lag_s] = np.nan
    if not bool(np.any(np.isfinite(r))):
        return None

    winner = int(np.nanargmax(r))
    best = float(r[winner])

    guard = max(1, round(_RIVAL_GUARD_S / hop_s))
    rivals = r.copy()
    rivals[max(0, winner - guard) : winner + guard + 1] = np.nan
    runner_up = float(np.nanmax(rivals)) if bool(np.any(np.isfinite(rivals))) else 0.0

    if best < min_correlation or best - runner_up < min_margin:
        return None
    return ClipOffset(seconds=float(lags[winner]) * hop_s, r=best, runner_up_r=runner_up)


def _flux_envelope(
    samples: Sequence[int] | np.ndarray[Any, Any], rate: int
) -> tuple[np.ndarray[Any, Any], int, int]:
    """Half-wave-rectified spectral flux, one value per hop, with the hop and window it used.

    Flux — the sum of how much each frequency bin *grew* since the previous window — rather than
    plain energy, because it answers the question actually being asked. A ball strike is a sudden
    broadband arrival, and it is the arrival that marks the instant; loudness alone peaks somewhere
    in the middle of the ringing that follows, and where in it depends on the room, the microphone
    and how far away the phone was standing.
    """
    x = np.asarray(samples, dtype=np.float64)
    hop = max(1, round(rate * _HOP_S))
    window_len = max(2, round(rate * _WINDOW_S))
    frames = 1 + (x.size - window_len) // hop if x.size >= window_len else 0
    if frames < 2:
        return np.empty(0), hop, window_len

    taper = np.hanning(window_len)
    envelope = np.empty(frames - 1)
    previous: np.ndarray[Any, Any] | None = None
    for start in range(0, frames, _FRAME_BLOCK):
        stop = min(frames, start + _FRAME_BLOCK)
        offsets = np.arange(window_len)[None, :] + hop * np.arange(start, stop)[:, None]
        magnitude = np.abs(np.fft.rfft(x[offsets] * taper, axis=1))
        if previous is not None:
            # The one difference that straddles the block boundary; without it the envelope would
            # read zero exactly where a strike happened to fall on a multiple of _FRAME_BLOCK.
            envelope[start - 1] = np.maximum(magnitude[0] - previous, 0.0).sum()
        envelope[start : stop - 1] = np.maximum(np.diff(magnitude, axis=0), 0.0).sum(axis=1)
        previous = magnitude[-1]
    return envelope, hop, window_len


def _min_overlap(a: np.ndarray[Any, Any], b: np.ndarray[Any, Any], hop_s: float) -> int:
    """How many hops two envelopes must share before a lag between them is worth scoring."""
    return max(round(_MIN_OVERLAP_S / hop_s), round(_MIN_OVERLAP_FRACTION * min(a.size, b.size)))


def _correlation_by_lag(
    a: np.ndarray[Any, Any], b: np.ndarray[Any, Any], *, min_overlap: int
) -> tuple[np.ndarray[Any, Any], np.ndarray[Any, Any]]:
    """Pearson correlation of `a` against `b` at every lag, NaN where the overlap is too small.

    Pearson over the *overlap* at each lag, not one global normalization, because the two clips
    are different lengths and start at different moments: a lag that lines up ten seconds of
    shared bay noise and one that lines up half a second of it would otherwise be scored on the
    same scale. Prefix sums make the per-lag means and variances exact at no extra pass over the
    data, so this stays one FFT and a handful of subtractions.

    Lag `k` means an event at index `i` in `a` sits at index `i - k` in `b`.
    """
    n, m = a.size, b.size
    size = 1 << int(np.ceil(np.log2(n + m)))
    spectrum = np.fft.rfft(a, size) * np.conj(np.fft.rfft(b, size))
    circular = np.fft.irfft(spectrum, size)
    # irfft wraps the negative lags around the end of the buffer; unwrap them onto a plain axis.
    lags = np.arange(-(m - 1), n)
    products = np.concatenate([circular[size - (m - 1) :], circular[:n]])

    sum_a = np.concatenate([[0.0], np.cumsum(a)])
    sum_aa = np.concatenate([[0.0], np.cumsum(a * a)])
    sum_b = np.concatenate([[0.0], np.cumsum(b)])
    sum_bb = np.concatenate([[0.0], np.cumsum(b * b)])

    lo_a = np.maximum(0, lags)
    hi_a = np.minimum(n, m + lags)
    count = hi_a - lo_a
    lo_b, hi_b = lo_a - lags, hi_a - lags
    safe = np.maximum(count, 1)

    total_a, total_aa = sum_a[hi_a] - sum_a[lo_a], sum_aa[hi_a] - sum_aa[lo_a]
    total_b, total_bb = sum_b[hi_b] - sum_b[lo_b], sum_bb[hi_b] - sum_bb[lo_b]
    covariance = products - total_a * total_b / safe
    spread = np.sqrt(
        np.maximum(total_aa - total_a * total_a / safe, 0.0)
        * np.maximum(total_bb - total_b * total_b / safe, 0.0)
    )
    r = np.where(spread > 0.0, covariance / np.where(spread > 0.0, spread, 1.0), 0.0)
    r[count < min_overlap] = np.nan
    return lags, r
