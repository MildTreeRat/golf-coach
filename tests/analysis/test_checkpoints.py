"""Mechanics checkpoints: tempo, head sway, and finish balance on synthetic swings."""

from __future__ import annotations

from conftest import make_swing

from golf_coach.analysis.checkpoints import (
    evaluate_finish_balance,
    evaluate_head_sway,
    evaluate_tempo,
)
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark


def _tempo(backswing_frames: int, downswing_frames: int):
    swing = make_swing(backswing_frames, downswing_frames)
    return evaluate_tempo(segment_phases(swing))


def _analyzed(head_sway: float = 0.0, finish_drift: float = 0.0):
    """Smooth then segment, mirroring the engine, and return (smoothed, phases)."""
    smoothed = smooth_keypoints(
        make_swing(30, 10, head_sway=head_sway, finish_drift=finish_drift)
    )
    return smoothed, segment_phases(smoothed)


def test_ideal_tempo_passes_inside_band() -> None:
    cp = _tempo(30, 10)  # ~3:1
    assert cp is not None
    assert cp.name == "tempo"
    assert cp.passed is True
    assert cp.expected_low <= cp.observed <= cp.expected_high
    assert cp.score == 1.0


def test_too_quick_tempo_fails_and_scores_lower() -> None:
    cp = _tempo(10, 12)  # backswing shorter than downswing → well under 2.7
    assert cp is not None
    assert cp.passed is False
    assert cp.observed < cp.expected_low
    assert cp.score < 1.0


def test_too_slow_tempo_fails_and_scores_lower() -> None:
    cp = _tempo(44, 8)  # long backswing → well over 3.3
    assert cp is not None
    assert cp.passed is False
    assert cp.observed > cp.expected_high
    assert cp.score < 1.0


def test_tempo_counts_horizontal_takeaway() -> None:
    # The vertical rise alone (20) vs downswing (13) reads ~1.5:1 ("too quick"), but the
    # near-horizontal takeaway is part of the backswing. Counting it (20 takeaway + 20 rise)
    # restores a believable ~3:1 instead of the collapsed reading a wrist-height rule produced.
    swing = make_swing(20, 13, takeaway_frames=20)
    cp = evaluate_tempo(segment_phases(swing))
    assert cp is not None
    assert cp.observed >= 2.5  # not collapsed toward the ~1.5:1 the vertical rise alone gives
    assert cp.passed is True


def test_steady_head_passes_sway() -> None:
    smoothed, phases = _analyzed(head_sway=0.0)
    cp = evaluate_head_sway(smoothed, phases)
    assert cp is not None
    assert cp.name == "head_sway"
    assert cp.passed is True
    assert cp.score == 1.0
    assert cp.observed <= cp.expected_high


def test_large_head_sway_fails() -> None:
    # A full shoulder-width of lateral drift (sway ~1.0) is well past the 0.5 band.
    smoothed, phases = _analyzed(head_sway=0.16)
    cp = evaluate_head_sway(smoothed, phases)
    assert cp is not None
    assert cp.passed is False
    assert cp.observed > cp.expected_high
    assert cp.score < 1.0


def test_held_finish_passes_balance() -> None:
    smoothed, phases = _analyzed(finish_drift=0.0)
    cp = evaluate_finish_balance(smoothed, phases)
    assert cp is not None
    assert cp.name == "finish_balance"
    assert cp.passed is True
    assert cp.score == 1.0


def test_staggered_finish_fails_balance() -> None:
    smoothed, phases = _analyzed(finish_drift=0.3)
    cp = evaluate_finish_balance(smoothed, phases)
    assert cp is not None
    assert cp.passed is False
    assert cp.observed > cp.expected_high


def test_checkpoints_return_none_when_unsegmentable() -> None:
    empty: list[FrameKeypoints] = []
    assert evaluate_head_sway(empty, []) is None
    assert evaluate_finish_balance(empty, []) is None


# --- metric definitions v2 (M4-REF) -------------------------------------------------------
# The two definition changes below are the reason GolfDB-derived bands are valid; if either
# regresses, every band cut against them silently becomes wrong. These tests pin the *property*
# each change was made for, not just its current number.


def test_head_sway_ignores_pure_head_rotation() -> None:
    """A head that turns in place must not read as sway. [v2: ear midpoint, not NOSE]

    This is the whole reason the metric moved off the nose. Here the ears stay put and only the
    nose swings across — a head rotating about its own axis, which is what every golfer's head
    does through impact. Under the v1 nose definition this scored as a full-blown sway fault.
    """
    swing = make_swing(30, 10)
    for index, frame in enumerate(swing):
        nose = frame.landmarks[PoseLandmark.NOSE]
        # Sweep the nose a long way across the face without moving the ears at all.
        frame.landmarks[PoseLandmark.NOSE] = nose.model_copy(
            update={"x": nose.x + 0.12 * index / (len(swing) - 1)}
        )

    smoothed = smooth_keypoints(swing)
    cp = evaluate_head_sway(smoothed, segment_phases(smoothed))
    assert cp is not None
    assert cp.observed == 0.0
    assert cp.passed is True


# A realistic held finish. The corpus this checkpoint's band is cut from measures p10 50 / p50 89
# follow-through frames; the fixture default of 8 is far too short for a p90 to reject anything.
_REALISTIC_FOLLOWTHROUGH = 90


def _blow_out_one_hip_frame(swing: list[FrameKeypoints]) -> list[FrameKeypoints]:
    """Wreck a single follow-through frame's hips — a plausible occlusion mis-detect."""
    bad = swing[-3]
    for side in (PoseLandmark.LEFT_HIP, PoseLandmark.RIGHT_HIP):
        bad.landmarks[side] = bad.landmarks[side].model_copy(update={"x": 0.95})
    return swing


def test_finish_balance_survives_a_single_bad_frame() -> None:
    """One mis-detected hip frame must not move the *observed value*. [v2: p90, not max]

    Follow-through is where the trail hip disappears behind the torso, so an occasional wild hip
    estimate is expected rather than exceptional. Under `max()` a single such frame *was* the
    metric. p90 requires the drift to persist before it counts.

    Asserted on `observed`, not on `passed`. Checking the verdict would couple this property to
    whatever `ranges.json` currently says, and a band change would then look like a regression in
    the metric — which is exactly what happened when the GolfDB recalibration tightened the band
    from 0.6 to 0.28. The robustness claim is about the statistic and is testable on its own.

    The claim is *attenuation*, not immunity: the outlier still drags the mean the drift is measured
    from, so it leaks through at a few hundredths. What must not happen is the single frame setting
    the metric, which is what the companion short-window test shows `max`-like behaviour doing.
    """
    clean_swing = make_swing(30, 10, followthrough_frames=_REALISTIC_FOLLOWTHROUGH)
    smoothed = smooth_keypoints(clean_swing)
    clean = evaluate_finish_balance(smoothed, segment_phases(smoothed))
    assert clean is not None
    assert clean.observed == 0.0

    dirty_swing = _blow_out_one_hip_frame(
        make_swing(30, 10, followthrough_frames=_REALISTIC_FOLLOWTHROUGH)
    )
    smoothed = smooth_keypoints(dirty_swing)
    cp = evaluate_finish_balance(smoothed, segment_phases(smoothed))
    assert cp is not None
    assert cp.observed is not None and clean.observed is not None
    assert cp.observed - clean.observed < 0.05, (
        f"one bad frame moved the metric {clean.observed} -> {cp.observed}"
    )


def test_finish_balance_rejection_needs_a_long_enough_window() -> None:
    """p90 has no rejection power on a short finish, and that limit is worth pinning down.

    `smooth_keypoints` smears one blown-out frame across its 5-frame window, so p90 only discards
    it once 5/n < 0.10 — beyond roughly 50 follow-through frames. Real footage clears that easily
    (the GolfDB face-on corpus is p10 50 / p50 89), but a *test* using the 8-frame fixture default
    would measure nothing while appearing to assert robustness. This test exists so that trap is
    recorded rather than rediscovered.
    """
    short = _blow_out_one_hip_frame(make_swing(30, 10, followthrough_frames=8))
    smoothed = smooth_keypoints(short)
    cp = evaluate_finish_balance(smoothed, segment_phases(smoothed))
    assert cp is not None
    assert cp.observed > 0.4, "expected the short window to let one bad frame through"


def test_finish_balance_still_catches_sustained_drift() -> None:
    """p90 must not have blunted the checkpoint into never firing."""
    cp = evaluate_finish_balance(*_analyzed(finish_drift=0.3))
    assert cp is not None
    assert cp.passed is False


def test_low_confidence_hips_are_rejected() -> None:
    """Hips below the stricter v2 gate are dropped, not trusted. [v2: _MIN_HIP_VISIBILITY]"""
    swing = make_swing(30, 10)
    for frame in swing:
        for side in (PoseLandmark.LEFT_HIP, PoseLandmark.RIGHT_HIP):
            # 0.6 clears the old 0.5 gate but not the 0.7 hip gate.
            frame.landmarks[side] = frame.landmarks[side].model_copy(update={"visibility": 0.6})

    smoothed = smooth_keypoints(swing)
    assert evaluate_finish_balance(smoothed, segment_phases(smoothed)) is None
