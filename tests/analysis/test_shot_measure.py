"""Derived launch-monitor quantities, and the ones deliberately left out.

The exclusions are as load-bearing as the inclusions here. This simulator prints a smash factor
below 1.0 and a club speed that does not match its own carry, and it prints a spin axis whose sign
contradicts the contract. Recording those as measurements would hand a future band-derivation step
a device artifact wearing a provenance string, so the tests pin their absence.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.analysis.shot_measure import (
    SHOT_MEASUREMENTS,
    measure_carry_distance,
    measure_face_to_path,
    measure_start_line,
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


def test_the_distances_are_registered_in_yards() -> None:
    """The unit is the whole interface for a pass-through metric — there is no arithmetic to check.

    A carry stored under `degrees`, or under no unit at all, reaches `analysis.baseline` and gets
    averaged anyway; the unit is all any reader has to tell 125.6 yards from 125.6 of anything.
    """
    for name in ("carry_distance_yds", "total_distance_yds"):
        assert SHOT_MEASUREMENTS[name][1] == "yards"


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
    )
    for name, (measure, unit, detail) in SHOT_MEASUREMENTS.items():
        assert unit and detail, f"{name} is missing a unit or a detail string"
        assert measure(shot) is not None
