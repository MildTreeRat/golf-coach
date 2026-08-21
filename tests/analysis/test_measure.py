"""Measuring, with nothing judging it: `analysis/measure.py` on synthetic swings.

The point of these tests is the *separation*. A measure function must produce a number with no
band in sight, and must refuse only when it genuinely could not measure — never because the store
lacks a range. That property is what lets a new metric be measured across the reference corpus
before any band for it exists, which is the circle that kept this panel at three.

Since 2026-08-19 a refusal also carries *which* condition failed, and `MEASUREMENT_REASONS` is
what keeps that from becoming a back door into judging: the reasons this module may report are a
strict subset of the vocabulary, and `NO_BAND` is not in it.
"""

from __future__ import annotations

import pytest
from conftest import make_swing

from golf_coach.analysis.measure import (
    FPS_DEPENDENT_MEASUREMENTS,
    POSE_MEASUREMENTS,
    measure_backswing_ms,
    measure_downswing_ms,
    measure_finish_balance,
    measure_head_hip_gain,
    measure_head_hip_offset_impact,
    measure_head_sway,
    measure_hip_shift_at_top,
    measure_hip_sway,
    measure_tempo_ratio,
)
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.swing import SwingPhase
from golf_coach.contracts.unscored import MEASUREMENT_REASONS, UnscoredReason

# The fixture's fixed face-on shoulder span: 0.58 - 0.42.
SHOULDER_WIDTH = 0.16


def _analyzed(**kwargs):
    """Smooth then segment, mirroring the engine, and return (smoothed, phases)."""
    smoothed = smooth_keypoints(make_swing(**kwargs))
    return smoothed, segment_phases(smoothed)


def test_tempo_ratio_is_backswing_over_downswing() -> None:
    ratio = measure_tempo_ratio(segment_phases(smooth_keypoints(make_swing(30, 10)))).value
    assert ratio is not None
    assert 2.5 <= ratio <= 3.5


def test_tempo_needs_no_band_to_be_measured() -> None:
    """The whole reason this module exists: a measurement never consults `ranges.json`.

    `evaluate_tempo` would return None with no band. `measure_tempo_ratio` cannot, because it has
    no idea bands exist.
    """
    import golf_coach.analysis.measure as measure_module

    assert "resolve_range" not in dir(measure_module)
    assert "benchmarks" not in measure_module.__doc__.lower()


def test_the_two_durations_are_the_halves_the_ratio_is_built_from() -> None:
    """The whole of Part 2 in one assertion: nothing new is computed, something stops being lost.

    `tempo_timings` had both numbers all along and divided them away, so no stored swing could say
    "your downswing took 384 ms" — a ratio of 2.35 is the same whether both halves are slow or
    both are quick, and those need opposite advice.
    """
    phases = segment_phases(smooth_keypoints(make_swing(30, 10)))

    backswing = measure_backswing_ms(phases).value
    downswing = measure_downswing_ms(phases).value
    ratio = measure_tempo_ratio(phases).value

    assert backswing is not None and downswing is not None
    assert backswing > 0 and downswing > 0
    assert ratio == pytest.approx(backswing / downswing)


def test_all_three_tempo_refusals_reach_both_durations() -> None:
    """One `tempo_timings` call behind all three, so they cannot disagree about a clip.

    Each case is a genuinely different swing and gets a different reason, which is what decides
    whether the golfer is told to re-film — `BOUNDARY_ESTIMATED` in particular means the footage
    was fine.
    """
    good = segment_phases(smooth_keypoints(make_swing(30, 10)))

    estimated = [
        p.model_copy(update={"detected": False}) if p.phase is SwingPhase.BACKSWING else p
        for p in good
    ]
    # Impact dragged back before the top, which is every phase present and disagreeing.
    top_ms = next(p for p in good if p.phase is SwingPhase.TRANSITION).start_ms
    degenerate = [
        p.model_copy(update={"start_ms": top_ms - 50.0}) if p.phase is SwingPhase.IMPACT else p
        for p in good
    ]

    cases = [
        ([], UnscoredReason.PHASE_NOT_SEGMENTED),
        (estimated, UnscoredReason.BOUNDARY_ESTIMATED),
        (degenerate, UnscoredReason.TIMING_DEGENERATE),
    ]
    for phases, expected in cases:
        for measure in (measure_backswing_ms, measure_downswing_ms, measure_tempo_ratio):
            outcome = measure(phases)
            assert outcome.value is None
            assert outcome.reason is expected, f"{measure.__name__} on {expected}"
            assert outcome.detail


def test_the_durations_are_the_only_metrics_that_depend_on_frame_rate() -> None:
    """The set the offline scripts refuse to touch, and why it is derived from the unit.

    `derive_pose_metrics.py` measures at GolfDB's labelled instants, whose phases carry *frame
    indices* in `start_ms` because ~47% of that corpus is slow-motion. A duration read off them is
    a frame count wearing a millisecond's name, and written to `swings.jsonl` it would be cut into
    a band beside real milliseconds. Every other metric is a ratio or a shoulder-width and is
    immune, which is the property this asserts.
    """
    assert FPS_DEPENDENT_MEASUREMENTS == {"backswing_ms", "downswing_ms"}

    for name, pose in POSE_MEASUREMENTS.items():
        assert (name in FPS_DEPENDENT_MEASUREMENTS) == (pose.unit == "ms"), name


def test_head_sway_is_zero_for_a_still_head() -> None:
    smoothed, phases = _analyzed()
    value = measure_head_sway(smoothed, phases).value
    assert value is not None
    assert value < 0.05


def test_head_sway_scales_by_shoulder_width() -> None:
    smoothed, phases = _analyzed(head_sway=0.08)
    value = measure_head_sway(smoothed, phases).value
    assert value is not None
    # 0.08 of lateral travel against a 0.16 ruler is half a shoulder width.
    assert 0.4 <= value <= 0.6


def test_hip_sway_is_zero_when_the_hips_hold() -> None:
    smoothed, phases = _analyzed()
    value = measure_hip_sway(smoothed, phases).value
    assert value is not None
    assert value < 0.05


def test_hip_sway_sees_a_slide_the_head_does_not() -> None:
    """The case that motivates the metric: hips slide, head stays put.

    Head sway reads clean here and the swing still has a lateral slide in it. This is the
    distinction the three-checkpoint panel could not draw.
    """
    smoothed, phases = _analyzed(head_sway=0.0, hip_sway=0.08)

    head = measure_head_sway(smoothed, phases).value
    hips = measure_hip_sway(smoothed, phases).value
    assert head is not None and hips is not None
    assert head < 0.05
    assert 0.4 <= hips <= 0.6


def test_hip_shift_at_top_is_between_zero_and_the_full_slide() -> None:
    """Measured at the top, so it sees part of a slide that completes by impact."""
    smoothed, phases = _analyzed(hip_sway=0.08)

    at_top = measure_hip_shift_at_top(smoothed, phases).value
    by_impact = measure_hip_sway(smoothed, phases).value
    assert at_top is not None and by_impact is not None
    assert 0.0 < at_top <= by_impact + 1e-9


def test_hip_shift_at_top_is_zero_when_the_hips_hold() -> None:
    smoothed, phases = _analyzed()
    value = measure_hip_shift_at_top(smoothed, phases).value
    assert value is not None
    assert value < 0.05


def test_head_hip_offset_is_signed() -> None:
    """The one signed metric: a head ahead of the hips and behind them are opposite faults."""
    smoothed, phases = _analyzed(head_sway=0.08)
    ahead = measure_head_hip_offset_impact(smoothed, phases).value

    smoothed, phases = _analyzed(head_sway=-0.08)
    behind = measure_head_hip_offset_impact(smoothed, phases).value

    assert ahead is not None and behind is not None
    assert ahead > 0.3
    assert behind < -0.3


def test_head_hip_offset_is_zero_when_stacked() -> None:
    smoothed, phases = _analyzed()
    value = measure_head_hip_offset_impact(smoothed, phases).value
    assert value is not None
    assert abs(value) < 0.05


def test_finish_balance_is_small_for_a_held_finish() -> None:
    smoothed, phases = _analyzed(followthrough_frames=60)
    value = measure_finish_balance(smoothed, phases).value
    assert value is not None
    assert value < 0.1


def test_finish_balance_grows_with_drift() -> None:
    steady, steady_phases = _analyzed(followthrough_frames=60)
    loose, loose_phases = _analyzed(followthrough_frames=60, finish_drift=0.10)

    a = measure_finish_balance(steady, steady_phases).value
    b = measure_finish_balance(loose, loose_phases).value
    assert a is not None and b is not None
    assert b > a


def test_unusable_phases_measure_nothing_rather_than_guessing() -> None:
    """No value means 'could not measure' — the only meaning it is allowed to have here.

    With no phases at all every one of these fails at the same place, and they all say so: the
    segmentation never produced the window. The reason is asserted rather than just the absence,
    because a `None` that arrived for the wrong reason is indistinguishable from one that did not.
    """
    for outcome in (
        measure_head_sway([], []),
        measure_hip_sway([], []),
        measure_hip_shift_at_top([], []),
        measure_head_hip_offset_impact([], []),
        measure_head_hip_gain([], []),
        measure_finish_balance([], []),
        measure_tempo_ratio([]),
    ):
        assert outcome.value is None
        assert outcome.reason is UnscoredReason.PHASE_NOT_SEGMENTED
        assert outcome.detail, "a reason with no window named is half an answer"


def test_no_measurement_ever_reports_a_judging_reason() -> None:
    """`measure.py` measures; it does not judge, and this is that split made checkable.

    A `NO_BAND` or `NO_HANDEDNESS` coming out of this module would mean the fusion M6.5 spent a
    milestone undoing had quietly reformed — the measuring layer would be reaching for a band
    again. Driven over both a good swing and an empty one so the pass path and every refusal path
    are covered.
    """
    smoothed, phases = _analyzed(followthrough_frames=60)

    for name, pose in POSE_MEASUREMENTS.items():
        for outcome in (pose.measure(smoothed, phases), pose.measure([], [])):
            if outcome.reason is not None:
                assert outcome.reason in MEASUREMENT_REASONS, (
                    f"{name} reported {outcome.reason}, which is a judging failure"
                )


def test_a_short_follow_through_is_too_few_frames_not_a_missing_phase() -> None:
    """The distinction that decides what a golfer is told to do differently.

    `finish_balance` is the checkpoint that goes unscored most often, and "your clip stopped at
    impact" and "the swing could not be segmented" are different clips with different fixes. Both
    used to be one `None`.
    """
    smoothed, phases = _analyzed(followthrough_frames=1)
    outcome = measure_finish_balance(smoothed, phases)

    assert outcome.value is None
    assert outcome.reason is UnscoredReason.TOO_FEW_FRAMES
    assert "follow-through" in outcome.detail


def test_registry_is_complete_and_consistent() -> None:
    """Every registered metric carries a unit and a detail string, and the registry runs.

    The registry is what `derive_pose_metrics.py` and `tune_spatial_metric.py` both iterate. The
    three parallel dicts this replaced could disagree about which names they held — a test caught
    that — but never about which unit belonged to which metric, because nothing tied a row
    together. One record per metric is what makes the misalignment unrepresentable.
    """
    smoothed, phases = _analyzed(followthrough_frames=60)

    for name, pose in POSE_MEASUREMENTS.items():
        assert pose.unit, f"{name} has no unit"
        assert pose.detail, f"{name} has no detail string"
        outcome = pose.measure(smoothed, phases)
        assert outcome.value is not None, f"{name} could not measure the ideal synthetic swing"


def test_norm_suffix_marks_the_shoulder_width_metrics() -> None:
    """`derive_reference.py` keys its one-sided band recommendation on the `_norm` suffix."""
    for name, pose in POSE_MEASUREMENTS.items():
        assert name.endswith("_norm") == (pose.unit == "shoulder_widths")
