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

import math

import pytest
from conftest import (
    _ADDRESS_Y,
    _GRIP_OFFSET_X,
    _GRIP_OFFSET_Y,
    _INDEX_FAN,
    _SHOULDER_LEFT_X,
    _SHOULDER_Y,
    make_swing,
)

from golf_coach.analysis.measure import (
    FPS_DEPENDENT_MEASUREMENTS,
    POSE_MEASUREMENTS,
    measure_backswing_ms,
    measure_downswing_ms,
    measure_finish_balance,
    measure_hand_height,
    measure_hand_offset_from_hips,
    measure_hand_separation,
    measure_head_hip_gain,
    measure_head_hip_offset_impact,
    measure_head_sway,
    measure_hip_shift_at_top,
    measure_hip_sway,
    measure_tempo_ratio,
    measure_trail_hand_roll,
)
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import PoseLandmark
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


# --------------------------------------------------------------------------- the hand pair [M14]

#: What the fixture places between the two wrists — a grip's width down the shaft, held for the
#: whole swing. Imported rather than retyped, so a retuned grip offset moves the expectation with
#: it instead of failing here.
_GRIP_LENGTH = math.hypot(_GRIP_OFFSET_X, _GRIP_OFFSET_Y)


def test_hand_separation_is_the_distance_the_fixture_holds_the_wrists_apart() -> None:
    """A grip's width over the shoulder ruler, to the digit — not a range.

    The value is exact because both wrists carry the same trajectory offset by a constant, so the
    5-frame smoothing window shifts them identically and cancels out of their distance. Anything
    else means the metric picked up a landmark that is not travelling with the club.
    """
    smoothed, phases = _analyzed()
    value = measure_hand_separation(smoothed, phases).value
    assert value is not None
    assert value == pytest.approx(_GRIP_LENGTH / SHOULDER_WIDTH, abs=1e-6)


def test_hand_separation_is_the_same_number_whatever_the_swing_does() -> None:
    """The property that makes it the pair's canary: it describes the golfer, not the swing.

    Both hands are on one grip, so a takeaway that drags the wrists across the frame, a swaying
    head and a drifting finish must all leave this untouched. A metric that moved with them would
    be reading something other than two hands on a club — which is the failure it exists to catch
    in a *stored* number, where no frame is available to look at.
    """
    values = [
        measure_hand_separation(*_analyzed(**case)).value
        for case in (
            {},
            {"takeaway_frames": 6},
            {"head_sway": 0.08, "hip_sway": 0.08},
            {"followthrough_frames": 60, "finish_drift": 0.10},
        )
    ]
    assert all(value is not None for value in values)
    assert values[1:] == [pytest.approx(values[0], abs=1e-6)] * 3


def test_hand_height_puts_the_hands_below_the_shoulders() -> None:
    """Positive is hands below shoulders, and the size is the reach down to the ball.

    The fixture stands the golfer with shoulders at `y=0.4` and the lead wrist at `y=0.85`, so the
    wrist midpoint hangs a shade under 0.46 below them — a shade, because the trail wrist sits a
    grip's width further down the shaft. Asserted against the fixture's own constants rather than
    a literal, and loosely enough to absorb the smoothing window reaching into the takeaway at the
    far end of the address span.
    """
    smoothed, phases = _analyzed()
    value = measure_hand_height(smoothed, phases).value

    expected = ((_ADDRESS_Y + _GRIP_OFFSET_Y / 2) - _SHOULDER_Y) / SHOULDER_WIDTH
    assert value is not None
    assert value == pytest.approx(expected, abs=0.02)
    assert value > 0, "positive must mean hands below shoulders — image y grows downward"


def test_hand_height_grows_when_the_hands_hang_lower() -> None:
    """Posture, not a constant: reaching further down to the ball reads as a larger number.

    Driven by lowering the wrists after smoothing rather than by a fixture knob, because the
    fixture's address height is shared with every tempo and phase expectation in this directory
    and moving it would be a change to all of them.
    """
    smoothed, phases = _analyzed()
    before = measure_hand_height(smoothed, phases).value

    drop = 0.08  # half a shoulder width, the same size `head_sway` uses
    for frame in smoothed:
        for side in (PoseLandmark.LEFT_WRIST, PoseLandmark.RIGHT_WRIST):
            frame.landmarks[side] = frame.landmarks[side].model_copy(
                update={"y": frame.landmarks[side].y + drop}
            )
    after = measure_hand_height(smoothed, phases).value

    assert before is not None and after is not None
    assert after - before == pytest.approx(drop / SHOULDER_WIDTH, abs=1e-6)


def test_dim_wrists_measure_nothing_rather_than_a_plausible_number() -> None:
    """The refusal M14 P2 exists to make reachable, on the landmarks the pair actually reads.

    Dimmed *after* segmentation, not before: `segment_phases` tracks the lead wrist to find the
    top, so a swing with dim wrists from the start has no phases either and both metrics would
    refuse for `PHASE_NOT_SEGMENTED` — the right answer to a different question. Handing good
    phases to bad frames isolates the visibility gate, which is the thing under test.

    `LANDMARKS_UNCONFIDENT` and not a number is the whole point (ADR-010 §2). The wrists are still
    sitting at a perfectly plausible address position; only their confidence says otherwise, and
    that has to be enough.
    """
    smoothed, phases = _analyzed()
    for frame in smoothed:
        for side in (PoseLandmark.LEFT_WRIST, PoseLandmark.RIGHT_WRIST):
            frame.landmarks[side] = frame.landmarks[side].model_copy(update={"visibility": 0.4})

    for measure in (measure_hand_separation, measure_hand_height):
        outcome = measure(smoothed, phases)
        assert outcome.value is None, measure.__name__
        assert outcome.reason is UnscoredReason.LANDMARKS_UNCONFIDENT
        assert "address window" in outcome.detail or "address-window" in outcome.detail


# ------------------------------------------------------------------ the offset and the roll [M14]

#: The trail index knuckle's placement in the fixture, resolved from shaft-relative units to the
#: `(dx, dy)` a reader of the frame would see. Derived here rather than typed, for `_INDEX_FAN`'s
#: reason: retuning the grip offset must move the expectation with it.
_INDEX_DX = _INDEX_FAN[0] * _GRIP_OFFSET_X + _INDEX_FAN[1] * _GRIP_OFFSET_Y
_INDEX_DY = _INDEX_FAN[0] * _GRIP_OFFSET_Y - _INDEX_FAN[1] * _GRIP_OFFSET_X


def _place_trail_knuckle(smoothed, dx: float, dy: float) -> None:
    """Put the trail index knuckle at a fixed offset from its wrist, in every frame.

    After smoothing, like the other landmark edits here: the window would otherwise average the
    placement against its neighbours and the angle under test would not be the angle asked for.
    """
    for frame in smoothed:
        wrist = frame.landmarks[PoseLandmark.RIGHT_WRIST]
        frame.landmarks[PoseLandmark.RIGHT_INDEX] = frame.landmarks[
            PoseLandmark.RIGHT_INDEX
        ].model_copy(update={"x": wrist.x + dx, "y": wrist.y + dy})


def test_hand_offset_from_hips_is_the_signed_gap_the_fixture_holds() -> None:
    """Wrist midpoint against hip center, signed, exact.

    Both are parked at `x = 0.5` for the whole address dwell and the wrists carry a constant grip
    offset, so what survives is half that offset over the ruler. Exact rather than approximate
    because nothing in `x` moves across the smoothing window on the default swing.
    """
    smoothed, phases = _analyzed()
    value = measure_hand_offset_from_hips(smoothed, phases).value
    assert value is not None
    assert value == pytest.approx((_GRIP_OFFSET_X / 2) / SHOULDER_WIDTH, abs=1e-6)


def test_hand_offset_from_hips_flips_sign_with_the_hands() -> None:
    """Signed and camera-relative: hands to the image-left of the hips is a negative number.

    The whole point of storing the sign, and `head_hip_offset_impact_norm`'s reason one window
    earlier. A magnitude would call these two setups identical.
    """
    smoothed, phases = _analyzed()
    shift = 0.05
    for frame in smoothed:
        for side in (PoseLandmark.LEFT_WRIST, PoseLandmark.RIGHT_WRIST):
            frame.landmarks[side] = frame.landmarks[side].model_copy(
                update={"x": frame.landmarks[side].x - shift}
            )
    value = measure_hand_offset_from_hips(smoothed, phases).value

    assert value is not None
    assert value < 0
    assert value == pytest.approx((_GRIP_OFFSET_X / 2 - shift) / SHOULDER_WIDTH, abs=1e-6)


def test_hand_offset_from_hips_refuses_when_the_hips_go_dim() -> None:
    """The hip branch is reachable, unlike `hand_height_norm`'s shoulder one.

    `hip_center_points` gates at `MIN_HIP_VISIBILITY`, which is stricter than the ruler's gate, so
    a clip can carry a perfectly good shoulder width and still have no hips to measure against.
    0.6 is the value that proves it: above `MIN_VISIBILITY`, below `MIN_HIP_VISIBILITY`.
    """
    smoothed, phases = _analyzed()
    for frame in smoothed:
        for side in (PoseLandmark.LEFT_HIP, PoseLandmark.RIGHT_HIP):
            frame.landmarks[side] = frame.landmarks[side].model_copy(update={"visibility": 0.6})

    outcome = measure_hand_offset_from_hips(smoothed, phases)
    assert outcome.value is None
    assert outcome.reason is UnscoredReason.LANDMARKS_UNCONFIDENT
    assert "hip" in outcome.detail


def test_trail_hand_roll_reads_the_fan_the_fixture_placed() -> None:
    """The angle off image-vertical of the trail wrist-to-index-knuckle vector, exactly.

    Exact for `hand_separation_norm`'s reason: both landmarks carry the same trajectory offset by
    a constant, so the smoothing window shifts them identically and cancels out of their
    difference. A drifting value here means the metric read a landmark not riding with the hand.
    """
    smoothed, phases = _analyzed()
    value = measure_trail_hand_roll(smoothed, phases).value
    assert value is not None
    assert value == pytest.approx(math.degrees(math.atan2(_INDEX_DX, _INDEX_DY)), abs=1e-6)


@pytest.mark.parametrize(
    ("dx", "dy", "expected"),
    [
        (0.0, 0.03, 0.0),  # straight down the frame
        (0.03, 0.03, 45.0),  # down and to the image-right
        (-0.03, 0.03, -45.0),  # down and to the image-left, sign kept rather than folded away
        (0.03, 0.0, 90.0),  # horizontal, image-right
    ],
)
def test_trail_hand_roll_measures_the_angle_off_vertical(dx, dy, expected) -> None:
    """Zero is straight down and positive is image-right, on four placements that say so.

    Pinned against hand-computed angles rather than against the fixture's own arithmetic, because
    the convention is the thing being fixed: a version measuring off *horizontal*, or with the
    sign the other way, would still pass a test that recomputed it the way the code does.
    """
    smoothed, phases = _analyzed()
    _place_trail_knuckle(smoothed, dx, dy)
    assert measure_trail_hand_roll(smoothed, phases).value == pytest.approx(expected, abs=1e-6)


def test_trail_hand_roll_averages_around_the_wrap_not_through_it() -> None:
    """Why the mean is circular: two frames pointing nearly straight *up* must not read as down.

    Alternate frames at +179 and -179 degrees describe a hand pointing at the top of the frame in
    both. A mean of the per-frame angles gives 0 - straight down, the opposite direction, and a
    perfectly plausible-looking number. The mean of the unit vectors gives 180, which is what the
    frames actually show. This is the test that fails if `direction_series` stops normalizing or
    the caller starts averaging angles.
    """
    smoothed, phases = _analyzed()
    length = 0.03
    for i, frame in enumerate(smoothed):
        angle = math.radians(179.0 if i % 2 == 0 else -179.0)
        wrist = frame.landmarks[PoseLandmark.RIGHT_WRIST]
        frame.landmarks[PoseLandmark.RIGHT_INDEX] = frame.landmarks[
            PoseLandmark.RIGHT_INDEX
        ].model_copy(
            update={
                "x": wrist.x + length * math.sin(angle),
                "y": wrist.y + length * math.cos(angle),
            }
        )

    value = measure_trail_hand_roll(smoothed, phases).value
    assert value is not None
    assert abs(value) == pytest.approx(180.0, abs=1.0)


def test_trail_hand_roll_needs_no_shoulder_ruler() -> None:
    """An angle is scale-free, so a clip with no usable ruler still has a roll.

    Both shoulders collapsed onto one `x` is what `MIN_SHOULDER_WIDTH` rejects - a golfer turned
    side-on. `hand_offset_from_hips_norm` divides by that width and must say `SCALE_UNAVAILABLE`;
    the roll never asks for it, and refusing anyway would report a reason that was not the problem.
    """
    smoothed, phases = _analyzed()
    for frame in smoothed:
        for side in (PoseLandmark.LEFT_SHOULDER, PoseLandmark.RIGHT_SHOULDER):
            frame.landmarks[side] = frame.landmarks[side].model_copy(update={"x": _SHOULDER_LEFT_X})

    offset = measure_hand_offset_from_hips(smoothed, phases)
    assert offset.reason is UnscoredReason.SCALE_UNAVAILABLE
    assert measure_trail_hand_roll(smoothed, phases).value is not None


def test_trail_hand_roll_refuses_a_collapsed_hand_rather_than_reporting_zero() -> None:
    """A knuckle sitting on its wrist has no direction, and `atan2(0, 0)` is a confident 0.0.

    The failure this guards is specific and silent: 0 degrees is "the hand points straight down",
    an entirely ordinary address reading, so a fabricated one is indistinguishable from a measured
    one downstream. `MIN_DIRECTION_LENGTH` drops those frames; with none left the metric refuses.
    """
    smoothed, phases = _analyzed()
    _place_trail_knuckle(smoothed, 0.0, 0.0)

    outcome = measure_trail_hand_roll(smoothed, phases)
    assert outcome.value is None
    assert outcome.reason is UnscoredReason.LANDMARKS_UNCONFIDENT


def test_dim_trail_hand_measures_no_roll() -> None:
    """The visibility gate on the landmarks M14 P2 placed, which nothing read until now."""
    smoothed, phases = _analyzed()
    for frame in smoothed:
        frame.landmarks[PoseLandmark.RIGHT_INDEX] = frame.landmarks[
            PoseLandmark.RIGHT_INDEX
        ].model_copy(update={"visibility": 0.4})

    outcome = measure_trail_hand_roll(smoothed, phases)
    assert outcome.value is None
    assert outcome.reason is UnscoredReason.LANDMARKS_UNCONFIDENT
    assert "trail wrist" in outcome.detail
