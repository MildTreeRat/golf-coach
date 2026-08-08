"""Swing phase segmentation — pure, stdlib only. [M4-PoC]

Splits a `FrameKeypoints` timeline into the six `SwingPhase` spans by tracking the
**lead wrist** (`LEFT_WRIST`) vertical position in a face-on view (the canonical pose-camera
placement — see M1 findings / ADR-003 addendum). In image coordinates `y` grows *downward*,
so during the swing the wrist traces: rest (high `y`) → rises through the backswing (`y`
falls to a minimum at the top) → falls back through the downswing (`y` climbs) → impact near
address height → follow-through (rises again).

We only need three instants for the tempo checkpoint — **motion start**, **top of backswing**, and
**impact**.

**Top and impact are found together, as the two ends of the downswing.** The downswing is the
longest sustained stretch of the hands *coming down* (rising `y`), so we scan for near-monotone
rising runs and take the **earliest** one within half the size of the largest. Ordering carries the
decision: a full finish leaves the hands higher than they were at the top, and lost tracking after
the finish adds spurious excursions, so "the biggest descent" can easily be something that is not
the downswing — but nothing in a golf swing descends before the downswing does. That is also why
the rule needs no address baseline, no fps assumption, and no per-camera threshold.

An earlier version anchored on the global minimum of wrist `y` ("highest hands"). Validated against
GolfDB's hand-annotated events it mislocated the top by a **median of 26 frames**, always late,
failing on 80% of tour clips — it was finding the finish. The rule here reduces that to a median of
**2 frames** with a 7% failure rate; the residual cases are clips containing a practice swing before
the real one, which no amount of pose accuracy resolves. See docs/M4_POSE_BAKEOFF.md.

**Motion start** is then found *relative* to the top, by 2D wrist speed: walk backward and take the
takeaway to begin just after the last sustained *quiet* stretch. Using speed rather than wrist
height is what makes tempo believable — the early takeaway is near-horizontal, so a height rule
missed it, landed motion-start mid-takeaway, and collapsed tempo toward ~1:1 (M4 findings).

How long that quiet stretch must be is expressed as a fraction of the clip's own downswing
duration rather than as a frame count, which is what lets the same rule read a real-time clip and a
4x slow-motion one (M4-REF Phase B6, ADR-013). Motion start is by a wide margin the least accurate
of the three instants — median 7 frames against 2 and 1 — so it reports whether it actually found
anything: `_motion_start` returns a `detected` flag that rides on the ADDRESS and BACKSWING
segments, and `evaluate_tempo` drops its score rather than divide by an estimate.

Counting the downswing to ball contact matches how Tour Tempo defines it (ADR-010), so the
benchmark stays calibrated. Tempo depends only on start/top/impact *timing*, never on posture.
No numpy, no MediaPipe — just lists, so it runs on the base install (ADR-008).

**Expects smoothed input.** `engine.analyze_swing` runs `smoothing.smooth_keypoints` before
calling this, so the wrist `y` series is already denoised and the top/impact instants are
stable frame-to-frame — this is the real robustness win over the raw-landmark first pass
(which misread a jittery clip as ~1.1:1). It still works on raw keypoints, just noisier.

**Rejected (M4-REF, 2026-08-01): sub-frame parabola refinement of the top.** The textbook remedy
for a flat extremum is to fit a parabola near the `argmin` and take its vertex, on the theory that
the neighbouring samples pin the vertex better than the single lowest one does. It does not apply
here. The top of a golf swing is not a *symmetric* flat extremum — it is an asymmetric reversal,
and a symmetric-window fit is pulled toward whichever side has the shallower slope. Measured on our
two clips the lead wrist approaches the top at ~-0.006/frame and leaves it at ~+0.002/frame, and
the fit moved the top **+1 frame later**; on a synthetic swing with the asymmetry reversed it moved
it *earlier*. A correction whose sign depends on the local shape of the trajectory is a
data-dependent bias, not noise reduction — and it would differ between broadcast reference footage
and phone clips, which is precisely where we need the two to stay comparable. The raw `argmin` on
the smoothed series is left in place; how well it actually locates the top is measured against
GolfDB's ground-truth event labels rather than guessed at (see docs/M4_POSE_BAKEOFF.md).

HARDWARE-REVALIDATE: top/impact are pose-only *proxies* for ball contact. When club/ball
detection (M2) and launch-monitor timing (M3) land, validate them against real impact timing
and against the annotated overlay (see docs/M4_FUNDAMENTALS_PANEL.md).
"""

from __future__ import annotations

from typing import NamedTuple

from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import PhaseSegment, SwingPhase

_LEAD_WRIST = PoseLandmark.LEFT_WRIST

# Frames whose lead-wrist visibility is below this are treated as unreliable; we hold the
# last good `y` rather than trust a low-confidence jump (MediaPipe convention).
_MIN_VISIBILITY = 0.5

# A rising run tolerates a dip of this fraction of the rise it has already accumulated before it
# is considered over. A real downswing is not perfectly monotone in a 2D track; testing for strict
# monotonicity shatters it into fragments and ends up scoring noise instead of the swing.
_DRAWDOWN_TOLERANCE = 0.25

# A rising run counts as a candidate downswing at this fraction of the largest rise in the clip.
# Tuned against GolfDB ground truth over the full 461-clip face-on corpus (see
# `docs/M4_POSE_BAKEOFF.md`): mean top error is flat at ~10.5 frames across 0.75-0.85 and rises on
# both sides, so this is the centre of a plateau rather than an argmin. An earlier 97-clip sweep
# put the optimum at 0.50; that was sample size, not signal.
_MAJOR_RISE_FRACTION = 0.80

# Motion start is velocity-anchored. The lead wrist is *still* at setup (and momentarily between
# waggle bobs) but moves continuously once the takeaway begins — including the early, near-
# horizontal part a wrist-*height* rule misses. So we measure 2D wrist speed and, walking back
# from the top, take the takeaway to begin just after the last sustained *quiet* stretch: a run of
# frames slower than `_MOTION_QUIET_FRAC` of the swing's peak wrist speed.
#
# `_MOTION_QUIET_FRAC` is a fraction of the clip's own peak, so it is already scale- and
# fps-invariant; the 461-clip sweep puts it on a plateau across 0.04-0.05.
#
# **The run length is not a frame count** (M4-REF Phase B6, ADR-013). It used to be
# `_MOTION_STALL_FRAMES = 4` — the one fps-dependent absolute left anywhere in the address path,
# and the reason this instant failed the way it did. A downswing is ~8 frames in a real-time clip
# and ~30 in a broadcast slow-motion one, so "4 quiet frames" meant something four times different
# across the two halves of the corpus, and a slow takeaway spends long stretches below any
# fraction of its own peak. The walk therefore stopped *mid-takeaway*: signed error median +4 with
# 66% of clips late, and 17 frames of median error on slow-motion clips against 5 on real-time.
# Expressing the run as a fraction of `(impact - top)` — the clip's own time base, measured from
# the two instants we locate accurately — cut median error 9 -> 8 and mean 27.2 -> 24.1.
_MOTION_QUIET_FRAC = 0.05
_MOTION_STALL_FRACTION = 0.25

# Floor for the quiet run. Two frames is the shortest stretch that can distinguish a dwell from a
# single smoothed sample; it binds only on very short real-time clips.
_MOTION_STALL_MIN_FRAMES = 2

# When the wrist never settles — continuous fidgeting through setup, or a golfer still walking in —
# there is no quiet run to find. This used to return frame 0, which is not a neutral answer: GolfDB
# clips carry a median 59 frames of pre-roll (p90 254), so it fired on 11% of clips and cost them a
# median of 31 frames. Falling back to the tour-median tempo ratio instead bounds the damage
# (median 9 -> 7, mean 27.2 -> 22.9 overall). The value is GolfDB's own median over 1,399 clips
# (ADR-012), not a book number.
#
# The segment is marked `detected=False` when this fires, because the estimate is circular for
# anything that divides by it: `evaluate_tempo` would report ~3.5:1 by construction. Posture
# checkpoints still use it, since they only need it to place a sampling window (ADR-013).
_FALLBACK_TEMPO_RATIO = 3.5

# Address is still the weakest instant by a wide margin (median 7 frames, 40% of clips over 10)
# and this does not fix that. It is intrinsically hard — GolfDB's own SwingNet reaches only 31.7%
# PCE here — because the takeaway onset is a gradual departure from stillness rather than a
# direction change like the top or a contact event like impact. Six alternative signal families
# were tried and all lost to lead-wrist speed; `scripts/golfdb/tune_address.py` keeps them
# runnable and docs/M4_POSE_BAKEOFF.md Phase B6 records why each fails. The honest ceiling marker:
# a rule using *no pose signal at all* scores a median of 11 frames, so what the wrist contributes
# here is real but small.

# Half-widths (in frames) of the transition window straddling the top of the backswing and
# of the impact window straddling the return to address height. Small, symmetric, heuristic.
_TRANSITION_HALF_FRAMES = 3
_IMPACT_HALF_FRAMES = 2

# Below this many frames there is no swing to segment.
_MIN_FRAMES = 6


def _lead_wrist_xy(keypoints: list[FrameKeypoints]) -> list[tuple[float, float]]:
    """Lead-wrist `(x, y)` per frame, holding the last confident value through dim frames."""
    points: list[tuple[float, float]] = []
    last_good: tuple[float, float] | None = None
    for frame in keypoints:
        wrist = frame.landmark(_LEAD_WRIST)
        if wrist.visibility >= _MIN_VISIBILITY or last_good is None:
            last_good = (wrist.x, wrist.y)
        points.append(last_good)
    return points


def _wrist_confident(keypoints: list[FrameKeypoints]) -> list[bool]:
    """Per-frame mask: was the lead wrist actually tracked, or is `_lead_wrist_xy` holding?

    `_lead_wrist_xy` carries the last confident position through dim frames, which is right for a
    continuous series to smooth but wrong as evidence of where the hands went. Held frames are
    excluded from run detection so a stretch of lost tracking cannot bound a descent — worth about
    1.5 frames of mean top error on the GolfDB face-on set (docs/M4_POSE_BAKEOFF.md).
    """
    return [frame.landmark(_LEAD_WRIST).visibility >= _MIN_VISIBILITY for frame in keypoints]


def _rising_runs(ys: list[float], confident: list[bool]) -> list[tuple[float, int, int]]:
    """Near-monotone stretches of *falling hands* as `(rise, start_frame, end_frame)`.

    `y` grows downward, so a rising `y` is the hands coming down — a descent of the club. The
    downswing is the largest such stretch in a normal swing; the takeaway and the follow-through
    run the other way. Each run ends when `y` gives back more than `_DRAWDOWN_TOLERANCE` of the
    rise it has accumulated, and only confidently-tracked frames participate.
    """
    runs: list[tuple[float, int, int]] = []
    start = peak_at = -1
    peak = 0.0

    for index, y in enumerate(ys):
        if not confident[index]:
            continue
        if start < 0:
            start, peak, peak_at = index, y, index
            continue
        if y >= peak:
            peak, peak_at = y, index
            continue

        rise = peak - ys[start]
        if rise > 0.0 and (peak - y) > _DRAWDOWN_TOLERANCE * rise:
            runs.append((rise, start, peak_at))
            start, peak, peak_at = index, y, index
        elif y < ys[start]:
            # Still descending toward a lower turning point — restart from here.
            start, peak, peak_at = index, y, index

    if start >= 0 and peak > ys[start]:
        runs.append((peak - ys[start], start, peak_at))
    return runs


def _top_and_impact(ys: list[float], confident: list[bool], n: int) -> tuple[int, int]:
    """Locate the top of the backswing and impact as the ends of the downswing.

    Takes the **earliest** rising run within `_MAJOR_RISE_FRACTION` of the largest one, rather than
    the largest outright. That one word is what makes this correct on real swings: a full finish
    puts the hands *higher* than they were at the top, and after the finish tracking often degrades
    into large spurious excursions — either can produce a rise that rivals the true downswing. What
    they cannot do is happen *before* it. Ordering is the one piece of structure every golf swing
    has, and unlike a threshold it does not need calibrating per camera, per player, or per fps.

    Falls back to the old global-argmin behaviour only when no run is found at all (a clip with no
    detectable descent), where any answer is a guess anyway.
    """
    runs = _rising_runs(ys, confident)
    if not runs:
        top = min(range(n), key=ys.__getitem__)
        return top, max(range(top, n), key=ys.__getitem__)

    largest = max(rise for rise, _, _ in runs)
    major = [run for run in runs if run[0] >= _MAJOR_RISE_FRACTION * largest]
    _, top, impact = min(major, key=lambda run: run[1])
    return top, impact


class Downswing(NamedTuple):
    """One candidate downswing: the hands descending from `top` to `impact`."""

    top: int
    impact: int
    rise: float  # how far the lead wrist fell, in normalized image units


def candidate_downswings(
    keypoints: list[FrameKeypoints], *, min_fraction: float = _MAJOR_RISE_FRACTION
) -> list[Downswing]:
    """Every descent of the hands in the clip, earliest first. [M7 Phase 2]

    `segment_phases` locates *one* swing and is right to: it was validated against 461 GolfDB
    clips, each of which contains exactly one. A phone clip does not. Someone takes two practice
    swings, settles, and then hits — and `_top_and_impact`'s "earliest major run" rule, which
    exists to stop a *finish* being read as the top, will happily return the first **practice**
    swing instead. That failure is already documented at the top of this module as the residual 7%
    of GolfDB clips; on hand-held phone footage it stops being an edge case.

    This does not change that rule. It exposes the runs `_rising_runs` already finds so a caller
    can *see* how many swings are in the clip and choose one, instead of discovering by eye that
    the wrong one was picked. `analysis.alignment` uses it to warn when two clips of "the same"
    swing disagree, and `scripts/align_swings.py --list-swings` prints it.

    `min_fraction` is the share of the largest descent a run must reach to be listed. It defaults
    to the same `_MAJOR_RISE_FRACTION` `segment_phases` itself applies, so the *first* entry of the
    default listing is exactly the swing `segment_phases` would have chosen. Lower it to see the
    near-misses — a lazy practice swing often descends less far than the real one.
    """
    n = len(keypoints)
    if n < _MIN_FRAMES:
        return []

    xy = _lead_wrist_xy(keypoints)
    ys = [y for _, y in xy]
    runs = _rising_runs(ys, _wrist_confident(keypoints))
    if not runs:
        return []

    largest = max(rise for rise, _, _ in runs)
    threshold = min_fraction * largest
    return [
        Downswing(top=start, impact=peak, rise=rise)
        for rise, start, peak in runs
        if rise >= threshold
    ]


# How long a real downswing takes, in seconds — the discriminator that separates a golf swing
# from everything else a phone clip contains. This is a **selection** aid, not a measurement, and
# it is the one rule in this module expressed in seconds rather than in the clip's own time base
# (ADR-013): a downswing is ~0.2-0.3 s for every golfer at every frame rate, which is exactly what
# makes it usable to tell swings apart from setup moves.
#
# Measured on the four real bay clips (`--list-swings`, M7 Phase 2 footage):
#
#   real swings    0.23  0.38  0.40  0.42        <- one per clip, ground-truth confirmed on aaron-1
#   setup moves    0.48  0.50  0.50  0.50  0.53  <- the hands being lowered into address
#   rehearsals     1.57  3.10  3.37  6.60  ...   <- practice swings and idle motion
#   tracking junk  0.08                          <- 5 frames at the very end of a clip
#
# The upper bound sits in a 0.06 s gap (0.42 real against 0.48 decoy), which is *thin*. That is why
# `select_swing` always reports what it chose, always yields to an explicit window, and declines
# rather than guesses when nothing lands in the band. Down-the-line durations run longer than
# face-on ones for the same swing because the DTL top reads early (the lead wrist is the far,
# occluded arm from behind — see WORKLOG 2026-08-07), which is what the 0.45 ceiling accommodates.
#
# **Every one of those numbers came from a 60 fps clip.** A 30 fps recording of the same swing
# brackets the descent more coarsely and reads *longer* — the 2026-08-07 bundle's face-on view
# measures 0.60 s for its only swing. Widening the band to admit that would swallow the whole
# setup-move cluster at 60 fps, so the band stays where the evidence put it and the
# single-candidate case is handled separately instead (see `select_swing`).
_PLAUSIBLE_DOWNSWING_S = (0.15, 0.45)

# How much clip to keep around the chosen swing, in downswing-lengths before the top and after
# impact — the clip's own time base, so this reads a 30 fps clip and a 240 fps one alike
# (ADR-013). Shared with `scripts/align_swings.py --list-swings` so the window the listing prints
# and the window `select_swing` picks are the same arithmetic.
#
# The lead is sized from `_FALLBACK_TEMPO_RATIO`: a tour backswing runs ~3.5 downswings, and
# `_motion_start` then needs a stretch of *quiet* address before that to walk back to. 2 was
# enough to frame a swing for viewing (which is all Phase 2 asked of it) but not to measure one
# — at 2 the window lands inside the backswing and motion start goes undetected on two of the
# six real clips, which drops the tempo checkpoint and degrades the alignment to top-and-impact.
# Measured across leads 2-8 on all six: 5 is the smallest value that detects motion start on
# every clip while leaving top and impact exactly where they are without a window at all.
_WINDOW_LEAD = 5
_WINDOW_TRAIL = 3

# How far a descent must fall, as a share of the clip's largest, to be considered a swing at all.
# Deliberately looser than `_MAJOR_RISE_FRACTION`, and it is not a nicety: on `aaron-1-back` the
# largest descent is a *bystander* at 0.417, so the default 0.80 threshold puts the cut at 0.334
# and the real swing (0.257) is not even a candidate. Duration is what identifies the swing here,
# so this rule's job is only to avoid discarding it before duration gets to look. Shared with
# `scripts/align_swings.py --list-swings` so the set a human chooses from and the set
# `select_swing` chooses from are identical.
CANDIDATE_MIN_RISE = 0.45


class SwingChoice(NamedTuple):
    """Which descent in a multi-swing clip is the actual swing, and why. [M7 Phase 4]"""

    window: tuple[int, int]
    downswing: Downswing
    candidates: list[Downswing]
    reason: str


def window_around(downswing: Downswing) -> tuple[int, int]:
    """The `[start, end)` frame window holding one swing plus its takeaway and finish."""
    frames = max(1, downswing.impact - downswing.top)
    return max(0, downswing.top - _WINDOW_LEAD * frames), downswing.impact + _WINDOW_TRAIL * frames


def select_swing(
    keypoints: list[FrameKeypoints], *, fps: float | None
) -> SwingChoice | None:
    """Pick the real swing out of a clip that contains several. [M7 Phase 4]

    `segment_phases` takes the **earliest** major descent, which is right for the single-swing
    GolfDB corpus it was validated against and wrong for a bay clip: on all four real clips it
    picks a *setup move*, and that is not merely a framing problem — the window decides which
    frames get scored, so an unwindowed clip is graded on the wrong swing (whole-clip
    `aaron-1-front` scores 58/100 with tempo unscored; the actual swing scores 67/100).

    Three rules, in order:

    1. **Duration.** Keep only candidates whose downswing lasts a plausible time
       (`_PLAUSIBLE_DOWNSWING_S`). This is what does the work — it is uniquely correct on all four
       multi-swing bay clips, because setup moves cluster tightly at ~0.5 s and rehearsals run
       whole seconds.
    2. **Last, not first.** Among survivors take the latest: nobody takes a practice swing *after*
       hitting the ball. Applied on its own this rule is wrong on both down-the-line clips (the
       DTL phone keeps rolling 15-24 s past impact, on the busy side of the bay), which is why it
       runs second rather than first.
    3. **One candidate wins on its own.** If the clip holds exactly one descent, take it whatever
       it measures. The duration band exists to *choose between* candidates; with nothing to
       choose between it is only a filter with no job, and applying it anyway throws away a
       perfectly good window — which is how a 30 fps single-swing clip (0.60 s, above the band
       derived from 60 fps footage) ended up scored over its whole length including dead air.
       This can never be worse than declining: `segment_phases` would pick that same lone descent
       regardless, so the choice is only whether to measure it in isolation or with the rest of
       the clip mixed in.

    Returns None — never a guess — when it has no pick to offer: no fps to read durations with
    (legacy keypoints files record `clip=None`, and a duration in seconds is meaningless without
    one), no descent at all, or several descents and none plausible. A caller that gets None
    falls back to `segment_phases`' own choice and should show the candidate listing, which
    prints the same durations this rule judged on.

    Note what this deliberately does **not** claim: a plausible downswing duration is evidence
    about *which descent is a swing*, not evidence that two clips show the *same* swing. Only
    `alignment.align_swings`' tempo cross-check speaks to that, and it does not fire in every
    case — it is skipped once the soft anchor has already been refused for another reason.
    """
    if fps is None or fps <= 0.0:
        return None

    candidates = candidate_downswings(keypoints, min_fraction=CANDIDATE_MIN_RISE)
    low, high = _PLAUSIBLE_DOWNSWING_S
    plausible = [
        swing for swing in candidates if low <= (swing.impact - swing.top) / fps <= high
    ]
    if not plausible:
        if len(candidates) != 1:
            return None
        only = candidates[0]
        return SwingChoice(
            window=window_around(only),
            downswing=only,
            candidates=candidates,
            reason=(
                f"1 descent, and its downswing measures "
                f"{(only.impact - only.top) / fps:.2f}s rather than the usual "
                f"{low:g}-{high:g}s — taking it anyway, since there is nothing to choose "
                f"between (top {only.top}, impact {only.impact})"
            ),
        )

    chosen = plausible[-1]
    duration = (chosen.impact - chosen.top) / fps
    if len(plausible) == 1:
        reason = (
            f"{len(candidates)} descent(s), 1 with a plausible downswing "
            f"({duration:.2f}s) — top {chosen.top}, impact {chosen.impact}"
        )
    else:
        reason = (
            f"{len(candidates)} descent(s), {len(plausible)} with a plausible downswing; took "
            f"the last ({duration:.2f}s, top {chosen.top}, impact {chosen.impact}). The clip "
            "holds more than one real-looking swing — confirm with --list-swings"
        )
    return SwingChoice(
        window=window_around(chosen), downswing=chosen, candidates=candidates, reason=reason
    )


def _wrist_speed(xy: list[tuple[float, float]]) -> list[float]:
    """Per-frame 2D lead-wrist speed (frame-to-frame displacement); `0.0` at the first frame."""
    speeds = [0.0]
    for (x0, y0), (x1, y1) in zip(xy, xy[1:], strict=False):
        speeds.append(((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5)
    return speeds


def _motion_start(xy: list[tuple[float, float]], top: int, impact: int) -> tuple[int, bool]:
    """Motion start: the frame the sustained takeaway begins, walking back from the top.

    Returns `(frame, detected)`. `detected` is False when the wrist never settles and the frame is
    the bounded estimate described at `_FALLBACK_TEMPO_RATIO` rather than something found in the
    signal — callers that divide by this boundary must drop their result instead of using it.

    The lead wrist is *still* at setup (and momentarily between waggle bobs) but moves continuously
    once the takeaway begins. We measure 2D wrist speed and walk backward from the top; the
    takeaway begins just after the last **quiet** stretch — a run of frames slower than
    `_MOTION_QUIET_FRAC` of the swing's peak wrist speed. Requiring a *run* keeps a single slow
    smoothed frame mid-takeaway from ending it early, and using speed (not wrist height) catches
    the near-horizontal early takeaway an earlier height rule missed.

    **The run length scales with the clip, not with the frame rate.** It is
    `_MOTION_STALL_FRACTION` of the downswing duration `(impact - top)` — the clip's own time base,
    taken from the two instants we locate to within 2 and 1 frames. That is what makes this rule
    read a 240fps phone clip and a broadcast slow-motion replay the same way; see ADR-013.

    Needs `top > 0` and reads cleanly because `engine` smooths first.
    """
    if top <= 0:
        return 0, True
    speeds = _wrist_speed(xy)
    peak = max(speeds[1 : top + 1], default=0.0)
    if peak <= 0.0:
        return 0, True
    quiet_threshold = peak * _MOTION_QUIET_FRAC
    downswing = max(impact - top, 1)
    stall = max(_MOTION_STALL_MIN_FRAMES, round(_MOTION_STALL_FRACTION * downswing))

    quiet = 0
    for i in range(top, -1, -1):
        if speeds[i] < quiet_threshold:
            quiet += 1
            if quiet >= stall:
                # First moving frame above the quiet run = takeaway start. Clamped to the top,
                # which a long stall near the top can otherwise overshoot.
                return min(i + quiet, top), True
        else:
            quiet = 0

    return max(0, top - round(_FALLBACK_TEMPO_RATIO * downswing)), False


def _segment(
    phase: SwingPhase, start: int, end: int, ts: list[float], detected: bool = True
) -> PhaseSegment:
    return PhaseSegment(
        phase=phase,
        start_frame=start,
        end_frame=end,
        start_ms=ts[start],
        end_ms=ts[end],
        detected=detected,
    )


def segment_phases(keypoints: list[FrameKeypoints]) -> list[PhaseSegment]:
    """Segment a keypoint timeline into the six swing phases (in canonical order).

    Returns an empty list for a clip too short to contain a swing. The returned segments are
    contiguous and their frame indices are monotonic non-decreasing, so a consumer can read
    phase timings straight off the boundaries.
    """
    n = len(keypoints)
    if n < _MIN_FRAMES:
        return []

    ts = [frame.timestamp_ms for frame in keypoints]
    xy = _lead_wrist_xy(keypoints)
    ys = [y for _, y in xy]
    confident = _wrist_confident(keypoints)

    # Top and impact are found together, as the two ends of the downswing (see `_top_and_impact`).
    top, impact = _top_and_impact(ys, confident, n)

    # Motion start: the frame the sustained takeaway begins (see `_motion_start`) — anchored on 2D
    # wrist speed so the near-horizontal early takeaway isn't missed and a waggle isn't mistaken
    # for the backswing. `found` is False when the wrist never settled and the boundary is an
    # estimate; it rides on the two segments that boundary defines.
    motion_start, found = _motion_start(xy, top, impact)

    # Bracket a small symmetric window around the top (transition) and after impact, then
    # clamp everything into a monotonic, non-overlapping boundary chain.
    b0 = 0
    b1 = motion_start
    b2 = max(b1, top - _TRANSITION_HALF_FRAMES)
    b3 = min(impact, top + _TRANSITION_HALF_FRAMES)
    b3 = max(b3, b2)
    b4 = max(impact, b3)
    b5 = min(n - 1, b4 + _IMPACT_HALF_FRAMES)
    b6 = n - 1
    b5 = max(b5, b4)

    return [
        _segment(SwingPhase.ADDRESS, b0, b1, ts, found),
        _segment(SwingPhase.BACKSWING, b1, b2, ts, found),
        _segment(SwingPhase.TRANSITION, b2, b3, ts),
        _segment(SwingPhase.DOWNSWING, b3, b4, ts),
        _segment(SwingPhase.IMPACT, b4, b5, ts),
        _segment(SwingPhase.FOLLOW_THROUGH, b5, b6, ts),
    ]
