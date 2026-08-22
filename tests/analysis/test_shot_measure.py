"""Derived launch-monitor quantities, and the ones deliberately left out.

The exclusions are as load-bearing as the inclusions here. This simulator prints a smash factor
below 1.0 and a club speed that does not match its own carry, and it prints a spin axis whose sign
contradicts the contract. Recording those as measurements would hand a future band-derivation step
a device artifact wearing a provenance string, so the tests pin their absence.

M9 P10 made that a discrimination rather than a blanket refusal: ball speed is recorded and club
speed is not, off the same screen and the same smash-factor finding, so there is a pin for the
pair as well as for the absence.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.analysis.shot_measure import (
    SHOT_MEASUREMENTS,
    measure_ball_speed,
    measure_carry_distance,
    measure_face_to_path,
    measure_launch_angle,
    measure_start_line,
    measure_start_line_offline,
    measure_total_distance,
    normalize_shot_shape,
)
from golf_coach.contracts.intent import TargetShape
from golf_coach.contracts.shot import ShotData, ShotSource


def _shot(**kwargs) -> ShotData:
    return ShotData(
        shot_id="s1",
        session_id="2026-08-10",
        timestamp=datetime(2026, 8, 10, tzinfo=UTC),
        source=ShotSource.SCREEN,
        **kwargs,
    )


# The three real shots on disk, with the shape HD Golf itself printed for each.
@pytest.mark.parametrize(
    ("face", "path", "expected", "printed_shape"),
    [
        (8.6, -4.6, 13.2, "FADE"),
        (2.8, -8.1, 10.9, "CENTER SLIGHT FADE"),
        (-5.0, 6.8, -11.8, "DRAW"),
    ],
)
def test_face_to_path_matches_the_simulators_own_shape(
    face: float, path: float, expected: float, printed_shape: str
) -> None:
    """Face-to-path sign must agree with the shape the screen printed, on all three real shots.

    This is a genuine cross-check rather than a tautology: the two angles are OCR'd with different
    sign vocabularies (`OPEN`/`CLOSED` for the face, `I>O`/`O>I` for the path), so agreement means
    both were resolved correctly.
    """
    shot = _shot(club_face_angle=face, club_path=path, shot_type=printed_shape)

    value = measure_face_to_path(shot)
    assert value is not None
    assert value == pytest.approx(expected, abs=0.01)

    shape = normalize_shot_shape(shot)
    if shape is TargetShape.FADE:
        assert value > 0, "a fade needs the face open to the path"
    elif shape is TargetShape.DRAW:
        assert value < 0, "a draw needs the face closed to the path"


def test_face_to_path_needs_both_angles() -> None:
    assert measure_face_to_path(_shot(club_face_angle=5.0)) is None
    assert measure_face_to_path(_shot(club_path=5.0)) is None
    assert measure_face_to_path(_shot()) is None


def test_start_line_is_carried_signed() -> None:
    assert measure_start_line(_shot(launch_direction=4.0)) == 4.0
    assert measure_start_line(_shot(launch_direction=-5.3)) == -5.3
    assert measure_start_line(_shot()) is None


def test_the_distances_are_carried_as_printed() -> None:
    """A real pair out of the shot store; nothing is rounded, scaled or converted."""
    shot = _shot(carry_distance=125.6, total_distance=131.0)

    assert measure_carry_distance(shot) == 125.6
    assert measure_total_distance(shot) == 131.0


def test_a_missing_distance_is_none_rather_than_zero() -> None:
    """A shot the screen printed no carry for did not carry zero yards.

    Same rule as `SwingResult.unscored` one layer down: a measurement that is absent stays absent,
    because a 0.0 pooled into a mean carry is a duff that never happened.
    """
    assert measure_carry_distance(_shot()) is None
    assert measure_total_distance(_shot()) is None
    assert measure_carry_distance(_shot(total_distance=131.0)) is None
    assert measure_total_distance(_shot(carry_distance=125.6)) is None


def test_the_launch_conditions_are_carried_as_printed() -> None:
    """A real pair out of the shot store; nothing rounded, scaled or converted.

    Straight reads, like the distances above — what they buy is a row in `measurements`, which is
    what `storage.corpus` pools. The fitting model these exist for does not exist yet, so this test
    is the whole of what can be asserted about them today.
    """
    shot = _shot(ball_speed=90.7, launch_angle=20.9)

    assert measure_ball_speed(shot) == 90.7
    assert measure_launch_angle(shot) == 20.9


def test_a_missing_launch_condition_is_none_rather_than_zero() -> None:
    """A screen that printed no ball speed did not record a ball that left at 0 mph.

    Each field is missing independently of the other, which is the case worth pinning: the two are
    read from separate tiles and the OCR can lose one of them, so a shot with a launch angle and no
    ball speed is a real state and neither field may stand in for the other.
    """
    assert measure_ball_speed(_shot()) is None
    assert measure_launch_angle(_shot()) is None
    assert measure_ball_speed(_shot(launch_angle=20.9)) is None
    assert measure_launch_angle(_shot(ball_speed=90.7)) is None


def test_offline_is_the_start_line_projected_to_the_carry() -> None:
    """The arithmetic, on a round number chosen so the answer can be checked by hand.

    150 yards at two degrees is 150 x sin(2 deg) = 5.23 yards right. Two degrees is not an
    incidental choice: it is the tolerance `contracts/dispersion.py` puts on the start line, so
    this case also says what that tolerance is worth in yards at a mid-iron carry.
    """
    shot = _shot(carry_distance=150.0, launch_direction=2.0)

    value = measure_start_line_offline(shot)
    assert value is not None
    assert value == pytest.approx(5.23, abs=0.01)


def test_offline_keeps_the_sign_of_the_start_line() -> None:
    """Positive is right of target, and it has to stay the same right as `start_line_deg`'s.

    Two readings of one quantity in two units is exactly where a sign flip hides: nothing else in
    the pipeline compares them, so a `-` dropped in the projection would produce a coherent-looking
    artifact that says the golfer misses the other way. This pins them to each other rather than
    each to a constant.
    """
    for angle in (4.0, -4.0, 0.5, -12.3):
        shot = _shot(carry_distance=150.0, launch_direction=angle)
        offline = measure_start_line_offline(shot)
        start_line = measure_start_line(shot)
        assert offline is not None and start_line is not None
        assert (offline > 0) == (start_line > 0)
        assert (offline < 0) == (start_line < 0)


def test_a_straight_start_is_offline_by_zero() -> None:
    """Zero degrees is zero yards at every carry - the one place a 0.0 here is a real measurement.

    It is worth pinning against the `None` cases below, because they are the same falsy value to
    any reader that forgets to check: a ball that started dead straight is offline by nothing,
    while a shot with no carry printed is offline by *unknown*.
    """
    assert measure_start_line_offline(_shot(carry_distance=150.0, launch_direction=0.0)) == 0.0
    assert measure_start_line_offline(_shot(carry_distance=250.0, launch_direction=0.0)) == 0.0


def test_offline_needs_both_inputs() -> None:
    """Either half missing means the projection is not available, never that it was zero.

    The carry direction is the one that matters most: a start line without a carry is a real
    reading of a different quantity, and multiplying it by an assumed distance would be a fitted
    parameter wearing a measurement's name.
    """
    assert measure_start_line_offline(_shot(carry_distance=150.0)) is None
    assert measure_start_line_offline(_shot(launch_direction=2.0)) is None
    assert measure_start_line_offline(_shot()) is None


def test_offline_is_registered_in_yards() -> None:
    """Degrees in, yards out - the unit is the whole point of the metric existing beside its angle.

    `start_line_deg` already records the same fact in degrees. If this row were also stored as
    degrees, the two would be indistinguishable to `analysis.baseline`, which averages by name and
    unit and has no idea one is a projection of the other.
    """
    assert SHOT_MEASUREMENTS["start_line_offline_yds"][1] == "yards"
    assert SHOT_MEASUREMENTS["start_line_deg"][1] == "degrees"


def test_the_distances_are_registered_in_yards() -> None:
    """The unit is the whole interface for a pass-through metric — there is no arithmetic to check.

    A carry stored under `degrees`, or under no unit at all, reaches `analysis.baseline` and gets
    averaged anyway; the unit is all any reader has to tell 125.6 yards from 125.6 of anything.
    """
    for name in ("carry_distance_yds", "total_distance_yds"):
        assert SHOT_MEASUREMENTS[name][1] == "yards"


def test_the_launch_conditions_are_registered_in_their_own_units() -> None:
    """mph and degrees - and the degrees one shares a unit with a metric in the other plane.

    `launch_angle_deg` is vertical and `start_line_deg` is horizontal, and to `analysis.baseline`
    they are two names with the same unit and no way to tell them apart. Nothing downstream needs
    to tell them apart today, which is precisely why the distinction has to live in the name and
    the detail string rather than in a reader's head.
    """
    assert SHOT_MEASUREMENTS["ball_speed_mph"][1] == "mph"
    assert SHOT_MEASUREMENTS["launch_angle_deg"][1] == "degrees"
    assert SHOT_MEASUREMENTS["start_line_deg"][1] == "degrees"
    assert "vertical" in SHOT_MEASUREMENTS["launch_angle_deg"][2]


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        ("FADE", TargetShape.FADE),
        ("CENTER SLIGHT FADE", TargetShape.FADE),
        ("DRAW", TargetShape.DRAW),
        ("STRAIGHT", TargetShape.STRAIGHT),
        ("CENTER", TargetShape.STRAIGHT),
        ("SLICE", TargetShape.FADE),
        ("HOOK", TargetShape.DRAW),
    ],
)
def test_shape_normalization(text: str, expected: TargetShape) -> None:
    assert normalize_shot_shape(_shot(shot_type=text)) is expected


def test_curvature_beats_start_line_in_a_compound_label() -> None:
    """'CENTER SLIGHT FADE' is a fade that started centered, not a straight shot.

    Regression: the first ordering of `_SHAPE_TOKENS` checked CENTER before FADE and classified
    this — an actual recorded shot — as STRAIGHT, silently discarding the only part of the label
    worth extracting.
    """
    assert normalize_shot_shape(_shot(shot_type="CENTER SLIGHT FADE")) is TargetShape.FADE
    assert normalize_shot_shape(_shot(shot_type="CENTER SLIGHT DRAW")) is TargetShape.DRAW


def test_unknown_vocabulary_is_none_rather_than_a_guess() -> None:
    assert normalize_shot_shape(_shot(shot_type="PUSH")) is None
    assert normalize_shot_shape(_shot(shot_type="")) is None
    assert normalize_shot_shape(_shot()) is None


def test_device_artifacts_are_not_measured() -> None:
    """smash / club speed / spin axis stay out — see the module docstring for each reason.

    Every shot on disk reads smash 0.89-1.00, i.e. ball speed below club speed, which no real
    strike produces. A band cut from that would be a band on a simulator bug.
    """
    excluded = {"smash", "club_head_speed", "club_speed", "spin_axis"}
    for name in SHOT_MEASUREMENTS:
        assert not any(term in name for term in excluded), f"{name} should not be measured"


def test_ball_speed_is_measured_where_club_speed_is_not() -> None:
    """The two halves of the smash factor are treated differently on purpose [M9 P10].

    The test above pins the absence; this pins the discrimination, because the two numbers come off
    the same screen under the same finding and the obvious reading of that finding excludes both.
    A smash below 1.0 says at least one of the pair is wrong and cannot say which. The two shots on
    disk can: 90.7 and 90.5 mph of ball speed for 125.6 and 121.0 yards of carry, against club
    speeds of 91.0 and 98.3 - the same ball speed and near enough the same carry, 7.3 mph apart on
    the field nothing else corroborates.
    """
    assert "ball_speed_mph" in SHOT_MEASUREMENTS
    assert not any("club" in name for name in SHOT_MEASUREMENTS)


def test_registry_entries_are_well_formed() -> None:
    # Every field any registered measurement reads. A metric added to the registry and not to this
    # shot fails here, which is the point — it is the cheapest place to notice that a new
    # measurement has no test of its own.
    shot = _shot(
        club_face_angle=8.6,
        club_path=-4.6,
        launch_direction=4.0,
        carry_distance=125.6,
        total_distance=131.0,
        ball_speed=90.7,
        launch_angle=20.9,
    )
    for name, (measure, unit, detail) in SHOT_MEASUREMENTS.items():
        assert unit and detail, f"{name} is missing a unit or a detail string"
        assert measure(shot) is not None
