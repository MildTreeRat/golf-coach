"""Measuring, with nothing judging it: `analysis/measure.py` on synthetic swings.

The point of these tests is the *separation*. A measure function must produce a number with no
band in sight, and must return None only when it genuinely could not measure — never because the
store lacks a range. That property is what lets a new metric be measured across the reference
corpus before any band for it exists, which is the circle that kept this panel at three.
"""

from __future__ import annotations

from conftest import make_swing

from golf_coach.analysis.measure import (
    POSE_MEASUREMENT_DETAIL,
    POSE_MEASUREMENT_UNIT,
    POSE_MEASUREMENTS,
    measure_finish_balance,
    measure_head_hip_offset_impact,
    measure_head_sway,
    measure_hip_shift_at_top,
    measure_hip_sway,
    measure_tempo_ratio,
)
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints

# The fixture's fixed face-on shoulder span: 0.58 - 0.42.
SHOULDER_WIDTH = 0.16


def _analyzed(**kwargs):
    """Smooth then segment, mirroring the engine, and return (smoothed, phases)."""
    smoothed = smooth_keypoints(make_swing(**kwargs))
    return smoothed, segment_phases(smoothed)


def test_tempo_ratio_is_backswing_over_downswing() -> None:
    ratio = measure_tempo_ratio(segment_phases(smooth_keypoints(make_swing(30, 10))))
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


def test_head_sway_is_zero_for_a_still_head() -> None:
    smoothed, phases = _analyzed()
    value = measure_head_sway(smoothed, phases)
    assert value is not None
    assert value < 0.05


def test_head_sway_scales_by_shoulder_width() -> None:
    smoothed, phases = _analyzed(head_sway=0.08)
    value = measure_head_sway(smoothed, phases)
    assert value is not None
    # 0.08 of lateral travel against a 0.16 ruler is half a shoulder width.
    assert 0.4 <= value <= 0.6


def test_hip_sway_is_zero_when_the_hips_hold() -> None:
    smoothed, phases = _analyzed()
    value = measure_hip_sway(smoothed, phases)
    assert value is not None
    assert value < 0.05


def test_hip_sway_sees_a_slide_the_head_does_not() -> None:
    """The case that motivates the metric: hips slide, head stays put.

    Head sway reads clean here and the swing still has a lateral slide in it. This is the
    distinction the three-checkpoint panel could not draw.
    """
    smoothed, phases = _analyzed(head_sway=0.0, hip_sway=0.08)

    head = measure_head_sway(smoothed, phases)
    hips = measure_hip_sway(smoothed, phases)
    assert head is not None and hips is not None
    assert head < 0.05
    assert 0.4 <= hips <= 0.6


def test_hip_shift_at_top_is_between_zero_and_the_full_slide() -> None:
    """Measured at the top, so it sees part of a slide that completes by impact."""
    smoothed, phases = _analyzed(hip_sway=0.08)

    at_top = measure_hip_shift_at_top(smoothed, phases)
    by_impact = measure_hip_sway(smoothed, phases)
    assert at_top is not None and by_impact is not None
    assert 0.0 < at_top <= by_impact + 1e-9


def test_hip_shift_at_top_is_zero_when_the_hips_hold() -> None:
    smoothed, phases = _analyzed()
    value = measure_hip_shift_at_top(smoothed, phases)
    assert value is not None
    assert value < 0.05


def test_head_hip_offset_is_signed() -> None:
    """The one signed metric: a head ahead of the hips and behind them are opposite faults."""
    smoothed, phases = _analyzed(head_sway=0.08)
    ahead = measure_head_hip_offset_impact(smoothed, phases)

    smoothed, phases = _analyzed(head_sway=-0.08)
    behind = measure_head_hip_offset_impact(smoothed, phases)

    assert ahead is not None and behind is not None
    assert ahead > 0.3
    assert behind < -0.3


def test_head_hip_offset_is_zero_when_stacked() -> None:
    smoothed, phases = _analyzed()
    value = measure_head_hip_offset_impact(smoothed, phases)
    assert value is not None
    assert abs(value) < 0.05


def test_finish_balance_is_small_for_a_held_finish() -> None:
    smoothed, phases = _analyzed(followthrough_frames=60)
    value = measure_finish_balance(smoothed, phases)
    assert value is not None
    assert value < 0.1


def test_finish_balance_grows_with_drift() -> None:
    steady, steady_phases = _analyzed(followthrough_frames=60)
    loose, loose_phases = _analyzed(followthrough_frames=60, finish_drift=0.10)

    a = measure_finish_balance(steady, steady_phases)
    b = measure_finish_balance(loose, loose_phases)
    assert a is not None and b is not None
    assert b > a


def test_unusable_phases_measure_nothing_rather_than_guessing() -> None:
    """None means 'could not measure' — the only meaning it is allowed to have here."""
    assert measure_head_sway([], []) is None
    assert measure_hip_sway([], []) is None
    assert measure_hip_shift_at_top([], []) is None
    assert measure_head_hip_offset_impact([], []) is None
    assert measure_finish_balance([], []) is None
    assert measure_tempo_ratio([]) is None


def test_registry_is_complete_and_consistent() -> None:
    """Every registered metric has a unit and a detail string, and the registry runs.

    The registry is what `derive_pose_metrics.py` and `tune_spatial_metric.py` both iterate, so a
    metric added to one dict and forgotten in another would silently lose its provenance.
    """
    smoothed, phases = _analyzed(followthrough_frames=60)

    assert set(POSE_MEASUREMENTS) == set(POSE_MEASUREMENT_UNIT)
    assert set(POSE_MEASUREMENTS) == set(POSE_MEASUREMENT_DETAIL)

    for name, measure in POSE_MEASUREMENTS.items():
        value = measure(smoothed, phases)
        assert value is not None, f"{name} could not measure the ideal synthetic swing"


def test_norm_suffix_marks_the_shoulder_width_metrics() -> None:
    """`derive_reference.py` keys its one-sided band recommendation on the `_norm` suffix."""
    for name, unit in POSE_MEASUREMENT_UNIT.items():
        assert name.endswith("_norm") == (unit == "shoulder_widths")
