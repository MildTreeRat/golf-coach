"""Mechanics checkpoints: tempo, head/hip sway, finish balance on synthetic swings."""

from __future__ import annotations

from conftest import make_swing

from golf_coach.analysis.benchmarks import resolve_range
from golf_coach.analysis.checkpoints import (
    CHECKPOINT_EVALUATORS,
    evaluate_finish_balance,
    evaluate_head_sway,
    evaluate_hip_shift_at_top,
    evaluate_hip_sway,
    evaluate_tempo,
)
from golf_coach.analysis.checkpoints.mechanics import (
    _ADDRESS_SAMPLE_MIN_FRAMES,
    _address_sample_bounds,
)
from golf_coach.analysis.measure import POSE_MEASUREMENTS
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.checkpoints import CHECKPOINT_REGISTRY
from golf_coach.contracts.intent import ClubCategory
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import SwingPhase


def _tempo(backswing_frames: int, downswing_frames: int):
    swing = make_swing(backswing_frames, downswing_frames)
    return evaluate_tempo(segment_phases(swing)).score


def _analyzed(head_sway: float = 0.0, finish_drift: float = 0.0, hip_sway: float = 0.0):
    """Smooth then segment, mirroring the engine, and return (smoothed, phases)."""
    smoothed = smooth_keypoints(
        make_swing(30, 10, head_sway=head_sway, finish_drift=finish_drift, hip_sway=hip_sway)
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
    cp = evaluate_tempo(segment_phases(swing)).score
    assert cp is not None
    assert cp.observed >= 2.5  # not collapsed toward the ~1.5:1 the vertical rise alone gives
    assert cp.passed is True


def test_steady_head_passes_sway() -> None:
    smoothed, phases = _analyzed(head_sway=0.0)
    cp = evaluate_head_sway(smoothed, phases).score
    assert cp is not None
    assert cp.name == "head_sway"
    assert cp.passed is True
    assert cp.score == 1.0
    assert cp.observed <= cp.expected_high


def test_large_head_sway_fails() -> None:
    # A full shoulder-width of lateral drift (sway ~1.0) is well past the 0.5 band.
    smoothed, phases = _analyzed(head_sway=0.16)
    cp = evaluate_head_sway(smoothed, phases).score
    assert cp is not None
    assert cp.passed is False
    assert cp.observed > cp.expected_high
    assert cp.score < 1.0


def test_held_finish_passes_balance() -> None:
    smoothed, phases = _analyzed(finish_drift=0.0)
    cp = evaluate_finish_balance(smoothed, phases).score
    assert cp is not None
    assert cp.name == "finish_balance"
    assert cp.passed is True
    assert cp.score == 1.0


def test_staggered_finish_fails_balance() -> None:
    smoothed, phases = _analyzed(finish_drift=0.3)
    cp = evaluate_finish_balance(smoothed, phases).score
    assert cp is not None
    assert cp.passed is False
    assert cp.observed > cp.expected_high


def test_checkpoints_return_none_when_unsegmentable() -> None:
    empty: list[FrameKeypoints] = []
    assert evaluate_head_sway(empty, []).score is None
    assert evaluate_finish_balance(empty, []).score is None
    assert evaluate_hip_sway(empty, []).score is None
    assert evaluate_hip_shift_at_top(empty, []).score is None


# --- the hip checkpoints, promoted 2026-08-12 ----------------------------------------------
# The point of these two is not that they are more checkpoints, it is that they have *different
# band shapes for different reasons*. hip_sway is two-sided because "less is better" is not
# established for hip travel; hip_shift_at_top is one-sided because its lower edge sits below
# the pipeline's own measurement error. Both refusals are easy to lose in a later refactor that
# "tidies" the panel into one shape, so each is pinned below.


def test_hip_sway_passes_inside_the_band() -> None:
    smoothed, phases = _analyzed(hip_sway=0.03)  # ~0.19 shoulder-widths
    cp = evaluate_hip_sway(smoothed, phases).score
    assert cp is not None
    assert cp.name == "hip_sway"
    assert cp.passed is True
    assert cp.score == 1.0
    assert cp.expected_low <= cp.observed <= cp.expected_high


def test_hip_sway_fails_when_the_hips_barely_move() -> None:
    """The whole reason this band is two-sided: a body that does not move laterally is a fault.

    Under a `[0, p90]` band -- the shape every other spatial checkpoint uses, and the one
    `derive_reference.py` recommends for any `_norm` metric -- this swing would score a perfect
    1.0. The tour population says otherwise: p10 is 0.14, so 90% of tour swings move the hips
    further than this one does.
    """
    smoothed, phases = _analyzed(hip_sway=0.0)
    cp = evaluate_hip_sway(smoothed, phases).score
    assert cp is not None
    assert cp.passed is False
    assert cp.observed < cp.expected_low
    assert cp.score < 1.0


def test_hip_sway_fails_when_the_hips_slide_too_far() -> None:
    smoothed, phases = _analyzed(hip_sway=0.12)  # ~0.74 shoulder-widths
    cp = evaluate_hip_sway(smoothed, phases).score
    assert cp is not None
    assert cp.passed is False
    assert cp.observed > cp.expected_high
    assert cp.score < 1.0


def test_hip_sway_is_two_sided_and_hip_shift_is_not() -> None:
    """The contrast, on one swing: motionless hips fail one checkpoint and pass the other.

    Not a redundant restatement of the two tests above -- it pins that the shapes were chosen per
    metric rather than copied, which is the property a later "make the panel consistent" change
    would silently undo. `one_sided` also rides out on `CheckpointScore` for `feedback` to rank
    with and for the caveats to warn about, so the flags are asserted, not just the verdicts.
    """
    smoothed, phases = _analyzed(hip_sway=0.0)

    sway = evaluate_hip_sway(smoothed, phases).score
    shift = evaluate_hip_shift_at_top(smoothed, phases).score
    assert sway is not None and shift is not None

    assert sway.passed is False and sway.one_sided is False
    assert shift.passed is True and shift.one_sided is True
    assert sway.expected_low > 0.0
    assert shift.expected_low == 0.0


def test_hip_shift_at_top_passes_a_centered_backswing() -> None:
    smoothed, phases = _analyzed(hip_sway=0.03)  # ~0.13 shoulder-widths by the top
    cp = evaluate_hip_shift_at_top(smoothed, phases).score
    assert cp is not None
    assert cp.name == "hip_shift_at_top"
    assert cp.passed is True
    assert cp.score == 1.0


def test_hip_shift_at_top_fails_a_lateral_slide_going_back() -> None:
    smoothed, phases = _analyzed(hip_sway=0.08)  # ~0.36 shoulder-widths by the top
    cp = evaluate_hip_shift_at_top(smoothed, phases).score
    assert cp is not None
    assert cp.passed is False
    assert cp.observed > cp.expected_high
    assert cp.score < 1.0


def test_the_hip_messages_never_claim_a_direction_or_a_rotation() -> None:
    """Both metrics are unsigned magnitudes, and neither sees the hips *turn*.

    `measure_hip_shift_at_top` drops the sign because it is camera-relative and handedness is not
    resolved on the analysis path, so a message naming a side would be right for half of golfers.
    Rotation is not measured at all -- the standing caveats forbid inferring it -- and "the hips
    slid instead of turning" is the sentence a coach reaches for first.
    """
    smoothed, phases = _analyzed(hip_sway=0.12)
    messages = [
        cp.message or ""
        for cp in (
            evaluate_hip_sway(smoothed, phases).score,
            evaluate_hip_shift_at_top(smoothed, phases).score,
        )
        if cp is not None
    ]
    assert len(messages) == 2
    for message in messages:
        lowered = message.lower()
        assert "rotat" not in lowered and "turning" not in lowered
        for side in ("left", "right", "away from the target", "toward the target"):
            assert side not in lowered


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
    cp = evaluate_head_sway(smoothed, segment_phases(smoothed)).score
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
    clean = evaluate_finish_balance(smoothed, segment_phases(smoothed)).score
    assert clean is not None
    assert clean.observed == 0.0

    dirty_swing = _blow_out_one_hip_frame(
        make_swing(30, 10, followthrough_frames=_REALISTIC_FOLLOWTHROUGH)
    )
    smoothed = smooth_keypoints(dirty_swing)
    cp = evaluate_finish_balance(smoothed, segment_phases(smoothed)).score
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
    cp = evaluate_finish_balance(smoothed, segment_phases(smoothed)).score
    assert cp is not None
    assert cp.observed > 0.4, "expected the short window to let one bad frame through"


def test_finish_balance_still_catches_sustained_drift() -> None:
    """p90 must not have blunted the checkpoint into never firing."""
    cp = evaluate_finish_balance(*_analyzed(finish_drift=0.3)).score
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
    assert evaluate_finish_balance(smoothed, segment_phases(smoothed)).score is None


def _with_pre_roll(keypoints: list[FrameKeypoints], frames: int) -> list[FrameKeypoints]:
    """Prepend still frames — the golfer standing over the ball before anything happens.

    `make_swing` opens with an 8-frame address dwell, which is far shorter than real footage: the
    GolfDB face-on corpus carries a median of 59 frames between the clip starting and the labelled
    address, and 254 at the p90. Without modelling that, the sampling window is bounded by how few
    frames exist rather than by its own length, and a test of its scaling measures nothing.
    """
    padded = [keypoints[0]] * frames + list(keypoints)
    return [
        frame.model_copy(update={"frame_index": index, "timestamp_ms": index * 10.0})
        for index, frame in enumerate(padded)
    ]


def test_address_sample_window_ends_at_the_boundary_and_excludes_pre_roll() -> None:
    """Posture is sampled from a short window ending at address, not from `[0, motion_start]`.

    The ADDRESS phase starts at frame 0, which is not address — it is wherever the clip begins.
    On GolfDB that window holds a median 59 frames of the golfer walking up to the ball, and their
    head travels 0.61 shoulder-widths across it at the p90 against a `head_sway` pass band of 0.42
    in total. See `_address_sample_bounds` and ADR-013.
    """
    smoothed = smooth_keypoints(_with_pre_roll(make_swing(30, 10, takeaway_frames=12), 40))
    phases = segment_phases(smoothed)
    address = next(p for p in phases if p.phase is SwingPhase.ADDRESS)

    lo, hi = _address_sample_bounds(phases)

    assert hi == address.end_frame, "the window must end exactly at the address boundary"
    assert lo > address.start_frame, "it must not reach back to frame 0"
    assert hi - lo + 1 >= _ADDRESS_SAMPLE_MIN_FRAMES
    # The 40 frames of pre-roll are exactly what the old `[0, motion_start]` window averaged over.
    assert lo > 40 - hi, "most of the pre-roll must fall outside the window"


def test_address_sample_window_scales_with_the_downswing() -> None:
    """A slow-motion clip gets a proportionally longer window, for the same reason the stall does.

    A fixed frame count would mean four times different things across a corpus that is ~47%
    slow-motion. Measured on GolfDB the window is 6 frames on real-time clips (the floor) and 12
    on slow-motion ones. Both fixtures get generous pre-roll so the window is limited by its own
    length rather than by the end of the clip.
    """
    fast = segment_phases(smooth_keypoints(_with_pre_roll(make_swing(20, 8), 60)))
    slow = segment_phases(smooth_keypoints(_with_pre_roll(make_swing(60, 40), 60)))

    fast_lo, fast_hi = _address_sample_bounds(fast)
    slow_lo, slow_hi = _address_sample_bounds(slow)

    assert (fast_hi - fast_lo) < (slow_hi - slow_lo)


def test_address_sample_window_survives_a_boundary_at_frame_zero() -> None:
    """When the boundary is 0 there is nothing behind it, so the window widens forward instead.

    Frames just after a mis-placed boundary are still far closer to setup than the start of a clip
    is, and 11% of GolfDB clips left fewer than 5 frames behind the boundary.
    """
    moving = make_swing(20, 14, takeaway_frames=60)[8:]
    phases = segment_phases(smooth_keypoints(moving))
    lo, hi = _address_sample_bounds(phases)

    assert 0 <= lo <= hi
    assert hi - lo + 1 >= _ADDRESS_SAMPLE_MIN_FRAMES


def test_tempo_is_dropped_when_the_boundary_was_only_estimated() -> None:
    """No score beats a wrong one (ADR-010 §2): a guessed address must not become a tempo reading.

    The fallback estimate is derived from an assumed tempo ratio, so scoring it would report that
    assumption straight back as an observation.
    """
    moving = make_swing(20, 14, takeaway_frames=60)[8:]
    phases = segment_phases(smooth_keypoints(moving))

    assert not next(p for p in phases if p.phase is SwingPhase.BACKSWING).detected
    assert evaluate_tempo(phases).score is None
    # Posture still scores — it only needs the boundary to place a window, not to divide by.
    smoothed = smooth_keypoints(moving)
    assert evaluate_head_sway(smoothed, phases).score is not None


def test_every_registered_checkpoint_has_an_evaluator_and_a_band() -> None:
    """The pin that catches a checkpoint added to the registry and nowhere else.

    `contracts.checkpoints` is now the single declaration of which checkpoints exist, which only
    helps if the three things a name has to line up with — an evaluator, a measurement, and a
    benchmark band — are checked against it rather than assumed. Adding a spec without a band is
    the specific mistake that would otherwise ship a checkpoint that silently lands in `unscored`
    on every swing (ADR-010 §2 makes it return `None`, so nothing else would complain).
    """
    for spec in CHECKPOINT_REGISTRY:
        assert spec.name in CHECKPOINT_EVALUATORS, (
            f"{spec.name} is registered but has no evaluator bound in "
            "analysis.checkpoints.CHECKPOINT_EVALUATORS"
        )
        assert spec.metric in POSE_MEASUREMENTS, (
            f"{spec.name} judges {spec.metric}, which nothing measures"
        )
        assert resolve_range(spec.metric, ClubCategory.ALL) is not None, (
            f"{spec.name} has no band in benchmarks/ranges.json, so it would be unscored on "
            "every swing"
        )

    assert set(CHECKPOINT_EVALUATORS) == {spec.name for spec in CHECKPOINT_REGISTRY}, (
        "an evaluator is bound to a name that is not registered — the engine walks the registry, "
        "so it would never be called"
    )


def test_band_shape_agrees_with_the_registry() -> None:
    """`one_sided` is a claim about the band, so the band is what it gets checked against.

    A one-sided band is `[0, high]` — only overshoot is judged. The flag rides out on every
    `CheckpointScore` and `caveats.py` builds the "less is not better" warning by partitioning on
    it, so a spec that disagreed with its own band would teach a model the wrong direction for a
    checkpoint. The ADR-010 addendum of 2026-08-12 is the rule these encode.
    """
    for spec in CHECKPOINT_REGISTRY:
        band = resolve_range(spec.metric, ClubCategory.ALL)
        assert band is not None
        assert spec.one_sided == (band.low == 0.0), (
            f"{spec.name} is registered one_sided={spec.one_sided} but its band is "
            f"[{band.low}, {band.high}]"
        )
