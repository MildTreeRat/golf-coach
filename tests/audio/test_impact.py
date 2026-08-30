"""Strike detection and clip-to-clip offset, on waveforms built to the bay's measured shape.

Synthetic, the way `tests/analysis/conftest.py` fabricates wrist tracks rather than decoding
clips: a stated shape, with the reason for every wobble in a comment. What is stated here is
docs/M11_ACOUSTIC_SYNC.md §E5's measurement of what a simulator bay actually emits per shot —

  club-ball contact,
  the club hitting the mat **15-20 ms** later, inseparable from the ball at a 5 ms hop,
  the ball hitting the impact screen **85-145 ms** after that, and **louder**,
  the simulator's own ball-flight audio a beat behind all of it,

— because those four transients, and specifically the screen being the loud one, are what decide
whether M11 P5 can ever be built on this module. An earlier draft of the milestone guessed 3 ms
and 60-80 ms and had the screen quieter; every number here is the corrected, measured one.

Room tone is white noise at a level well below the transients. That is a fair model for what
matters — these detectors threshold against the clip's own noise floor (ADR-013), so what the
floor is *made of* changes nothing, only how far the transients stand above it.
"""

from __future__ import annotations

from array import array

import pytest

np = pytest.importorskip("numpy")

from golf_coach.audio.impact import (  # noqa: E402  (after importorskip)
    detect_strikes,
    offset_between,
)

_RATE = 48_000

# One frame at the corpus's 60 fps — the unit every claim about *when* is worth stating in.
_FRAME_S = 1 / 60

# §E5's four transients, as offsets from the ball and amplitudes in int16 counts. The screen is
# 1.5x the ball here where §E5 measured 3-4x in flux; a smaller margin is the harder test, and it
# is still enough to put the screen top of the ranking.
_MAT_AFTER_S = 0.018
_SCREEN_AFTER_S = 0.115
_FLIGHT_AFTER_S = 0.600
_BALL_AMPLITUDE = 9_000
_MAT_AMPLITUDE = 6_000
_SCREEN_AMPLITUDE = 14_000
_FLIGHT_AMPLITUDE = 2_000

_ROOM_TONE = 200


def _room(seconds: float, rng: np.random.Generator) -> np.ndarray:
    """An empty bay: the clip a rehearsal swing produces."""
    return rng.normal(0.0, _ROOM_TONE, int(seconds * _RATE))


def _add_transient(
    track: np.ndarray, at_s: float, *, amplitude: float, decay_ms: float, rng: np.random.Generator
) -> None:
    """A broadband crack that decays away: noise under an exponential envelope.

    Broadband rather than a tone because that is what the flux envelope is built to catch — a
    strike is energy arriving in every bin at once — and decaying rather than a click because a
    click is what the corpus does *not* contain: a real strike stays elevated for tens of
    milliseconds (which is why the detector suppresses neighbours 50 ms either side).
    """
    start = int(at_s * _RATE)
    length = int(0.25 * _RATE)
    decay = np.exp(-np.arange(length) / _RATE / (decay_ms / 1000))
    track[start : start + length] += amplitude * rng.normal(0.0, 1.0, length) * decay


def _add_shot(track: np.ndarray, ball_at_s: float, rng: np.random.Generator) -> None:
    """All four of §E5's transients, at one shot's worth of spacing."""
    _add_transient(track, ball_at_s, amplitude=_BALL_AMPLITUDE, decay_ms=25, rng=rng)
    _add_transient(
        track, ball_at_s + _MAT_AFTER_S, amplitude=_MAT_AMPLITUDE, decay_ms=30, rng=rng
    )
    _add_transient(
        track, ball_at_s + _SCREEN_AFTER_S, amplitude=_SCREEN_AMPLITUDE, decay_ms=60, rng=rng
    )
    _add_transient(
        track, ball_at_s + _FLIGHT_AFTER_S, amplitude=_FLIGHT_AMPLITUDE, decay_ms=300, rng=rng
    )


def _shot_clip(ball_at_s: float = 2.0, *, seconds: float = 5.0, seed: int = 3) -> np.ndarray:
    """A whole clip: room tone with one shot in it, quantised the way a decode hands it over."""
    rng = np.random.default_rng(seed)
    track = _room(seconds, rng)
    _add_shot(track, ball_at_s, rng)
    return np.clip(track, -32_768, 32_767).astype(np.int16)


def _seconds(sample: int) -> float:
    return sample / _RATE


def _by_time(strikes: list) -> list:
    return sorted(strikes, key=lambda strike: strike.sample)


def test_the_ball_and_the_screen_are_both_found_within_a_frame() -> None:
    """The two loudest candidates are the two events, each landing on the right frame."""
    strikes = detect_strikes(_shot_clip(), _RATE)

    ball, screen = _by_time(strikes[:2])
    assert _seconds(ball.sample) == pytest.approx(2.0, abs=_FRAME_S)
    assert _seconds(screen.sample) == pytest.approx(2.0 + _SCREEN_AFTER_S, abs=_FRAME_S)


def test_the_loudest_transient_is_the_screen_and_not_the_ball() -> None:
    """The trap this module exists to keep open, pinned so nobody closes it by taking [0].

    On the real corpus the screen outranks the ball on roughly half the clips, so a caller that
    reads `strikes[0]` as "impact" is wrong about half the time and confidently so. The ranking
    is by prominence and the *earlier* of the top two is the ball.
    """
    strikes = detect_strikes(_shot_clip(), _RATE)

    assert _seconds(strikes[0].sample) == pytest.approx(2.0 + _SCREEN_AFTER_S, abs=_FRAME_S)
    assert strikes[0].prominence > strikes[1].prominence
    assert _seconds(min(strikes[0].sample, strikes[1].sample)) == pytest.approx(2.0, abs=_FRAME_S)


def test_the_club_mat_transient_is_absorbed_into_the_ball() -> None:
    """18 ms after the ball is inside the 50 ms suppression, deliberately (`_MIN_SEPARATION_S`).

    The mat is not separable from the ball at a 5 ms hop and no consumer in this milestone has a
    question that needs it, so it must not arrive as a second candidate competing with the ball
    for the caller's attention.
    """
    strikes = detect_strikes(_shot_clip(), _RATE)

    near_the_ball = [s for s in strikes if abs(_seconds(s.sample) - 2.0) < 0.040]
    assert len(near_the_ball) == 1


def test_a_rehearsal_makes_no_crack() -> None:
    """An empty bay is an empty list — the result M11 P5 reads, not a gap in the data."""
    rng = np.random.default_rng(5)
    tone = np.clip(_room(20.0, rng), -32_768, 32_767).astype(np.int16)

    assert detect_strikes(tone, _RATE) == []


def test_digital_silence_has_no_floor_to_measure_against() -> None:
    """Zero flux everywhere means no clip-relative threshold exists, so nothing is claimed.

    An absolute threshold would answer here, and would mean something different on every clip
    (ADR-013). Declining is the same choice `_MIN_PROMINENCE_Z` makes in the ordinary case.
    """
    assert detect_strikes(np.zeros(5 * _RATE, dtype=np.int16), _RATE) == []


def test_a_clip_too_short_to_hold_two_windows_is_not_a_measurement() -> None:
    """Flux is a *difference* between windows, so one window's worth of audio has none."""
    a_single_window = _shot_clip()[: int(_RATE * 1024 / 48_000)]

    assert detect_strikes(a_single_window, _RATE) == []


def test_the_frame_is_left_for_the_caller_that_knows_the_fps() -> None:
    """Pins `contracts/audio.py`'s reason for `AudioStrike.frame` being optional.

    This function is handed a waveform and a rate; it has never seen the video. Filling in a
    frame would mean inventing an fps, and an underivable number is None (ADR-010 §2).
    """
    assert all(strike.frame is None for strike in detect_strikes(_shot_clip(), _RATE))


def test_the_stdlib_array_the_port_hands_over_is_accepted() -> None:
    """`AudioClip.samples` is an `array("h")`, not a numpy array (audio/source.py).

    The seam only works if the detector takes what the decoder produces without a conversion
    step in between that each caller would otherwise have to remember.
    """
    clip = _shot_clip()

    assert detect_strikes(array("h", clip.tolist()), _RATE) == detect_strikes(clip, _RATE)


def test_a_rate_of_zero_is_a_bug_and_not_a_missing_measurement() -> None:
    with pytest.raises(ValueError):
        detect_strikes(_shot_clip(), 0)


# --- offset_between ---------------------------------------------------------------------------


def _bay(seconds: float = 12.0, shots: tuple[float, ...] = (4.0,), seed: int = 9) -> np.ndarray:
    """One continuous recording of the bay that several 'phones' can each be a window onto.

    It carries clatter as well as shots — a club dropped in the rack, someone setting a bag down
    — because the offset is recovered from the *pattern* of everything both phones heard, and a
    fixture with nothing in it but the shot would let a broken cross-correlation pass.
    """
    rng = np.random.default_rng(seed)
    track = _room(seconds, rng)
    for at in shots:
        _add_shot(track, at, rng)
    for at in (0.7, 1.9, 2.6, 9.4):
        if at < seconds:
            _add_transient(track, at, amplitude=2_500, decay_ms=40, rng=rng)
    return track


def _phone(bay: np.ndarray, start_s: float, seconds: float, seed: int) -> np.ndarray:
    """One phone's view of the bay: a window onto it, with that phone's own noise added.

    The independent tone matters — two phones do not record the same noise, and an offset that
    only survives when they do would be an artefact of the fixture.
    """
    rng = np.random.default_rng(seed)
    start = int(start_s * _RATE)
    window = bay[start : start + int(seconds * _RATE)].copy()
    window += _room(len(window) / _RATE, rng)[: len(window)]
    return np.clip(window, -32_768, 32_767).astype(np.int16)


def test_the_same_moment_recorded_twice_recovers_the_offset_and_its_sign() -> None:
    """`seconds` is `t_a - t_b`: add it to a time in b to land on the same instant in a."""
    bay = _bay()
    # b's phone was started 2.5 s before a's, so everything lands 2.5 s later on b's clock.
    a = _phone(bay, start_s=3.0, seconds=6.0, seed=21)
    b = _phone(bay, start_s=0.5, seconds=9.0, seed=22)

    offset = offset_between(a, b, _RATE)

    assert offset is not None
    assert offset.seconds == pytest.approx(-2.5, abs=2 * _FRAME_S)
    assert offset.r > 0.45
    assert offset.r - offset.runner_up_r > 0.10


def test_the_offset_needs_no_transient_identified() -> None:
    """The robust half: the same answer with the ball, the mat and the screen all unnamed.

    The detector above can be wrong about which peak was the ball without this being wrong about
    the offset, which is why M11 P6 can be built before M11 P5 exists.
    """
    bay = _bay()
    a = _phone(bay, start_s=3.0, seconds=6.0, seed=21)
    b = _phone(bay, start_s=0.5, seconds=9.0, seed=22)

    offset = offset_between(a, b, _RATE)
    ball_in_a = min(s.sample for s in detect_strikes(a, _RATE)[:2])
    ball_in_b = min(s.sample for s in detect_strikes(b, _RATE)[:2])

    assert offset is not None
    assert offset.seconds == pytest.approx(_seconds(ball_in_a) - _seconds(ball_in_b), abs=_FRAME_S)


def test_two_clips_with_nothing_in_common_are_declined() -> None:
    """Two empty bays correlate on noise, and noise is not an offset (ADR-010 §2)."""
    a = np.clip(_room(6.0, np.random.default_rng(31)), -32_768, 32_767).astype(np.int16)
    b = np.clip(_room(6.0, np.random.default_rng(32)), -32_768, 32_767).astype(np.int16)

    assert offset_between(a, b, _RATE) is None


def test_a_repeated_shot_is_declined_rather_than_guessed() -> None:
    """Two equally good answers is not one good answer.

    This is the synthetic form of what the corpus does for real: bundle 8's 80-second
    down-the-line clip holds two shots, and the face-on clip matches the wrong one *decisively*
    (r = 0.923 against 0.672). Where the two candidates are genuinely alike, as here, the margin
    rule catches it — and the next test is what catches it when they are not.
    """
    rng = np.random.default_rng(41)
    shot = _room(3.0, rng)
    _add_shot(shot, 1.5, np.random.default_rng(42))

    a = np.clip(shot, -32_768, 32_767).astype(np.int16)
    twice = np.concatenate([shot, _room(1.0, rng), shot])
    b = np.clip(twice, -32_768, 32_767).astype(np.int16)

    assert offset_between(a, b, _RATE) is None


def test_a_bound_on_the_search_resolves_what_the_margin_rule_only_declines() -> None:
    """The same ambiguous pair, answered — which is why `max_lag_s` exists and P6 must pass it.

    Declining and being wrong are both failures; a caller that already knows roughly where the
    swing is can rule the impostor out of the search instead of arguing with it afterwards.
    """
    rng = np.random.default_rng(41)
    shot = _room(3.0, rng)
    _add_shot(shot, 1.5, np.random.default_rng(42))

    a = np.clip(shot, -32_768, 32_767).astype(np.int16)
    twice = np.concatenate([shot, _room(1.0, rng), shot])
    b = np.clip(twice, -32_768, 32_767).astype(np.int16)

    offset = offset_between(a, b, _RATE, max_lag_s=1.0)

    assert offset is not None
    assert offset.seconds == pytest.approx(0.0, abs=2 * _FRAME_S)
    assert offset.r > 0.45


def test_a_sliver_of_overlap_is_not_a_comparison() -> None:
    """Below a second of shared audio there is not enough to correlate, at any lag."""
    bay = _bay(seconds=6.0, shots=(2.0,))
    a = _phone(bay, start_s=0.0, seconds=0.5, seed=51)
    b = _phone(bay, start_s=3.0, seconds=0.5, seed=52)

    assert offset_between(a, b, _RATE) is None


def test_an_offset_rate_of_zero_is_a_bug_too() -> None:
    with pytest.raises(ValueError):
        offset_between(_shot_clip(), _shot_clip(), 0)
