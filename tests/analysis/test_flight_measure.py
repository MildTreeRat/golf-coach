"""The six simulated measurements, and the two refusals that stand in for them. [M15 P11]

`test_flight.py` pins the physics and `test_flight_infer.py` pins the branch rule. What is tested
here is the **recording decision**: which of a flight's numbers becomes a `Measurement`, under what
name, and what is said when there is no flight at all. Those are the questions ADR-027 §Decision 6
answers, and two of its answers are corrections this phase measured rather than inherited.

Shots are built from `contracts` directly — no disk, no store — so a recording failure can never be
mistaken for a reader failure. The one shot flown end to end is `VALIDATION_SHOTS[0]`, which is
hand-typed from a screen and re-flown rather than remembered.
"""

from __future__ import annotations

from datetime import UTC, datetime
from math import cos, radians, sin

import pytest

from golf_coach.analysis.flight import VALIDATION_SHOTS
from golf_coach.analysis.flight_measure import (
    _COMPARISON_PAIRS,
    FLIGHT_MEASUREMENTS,
    FLIGHT_SOURCE,
    circular_carry_note,
    compare_to_printed,
    flight_unscored,
    fly_shot,
)
from golf_coach.analysis.shot_measure import SHOT_MEASUREMENTS, measure_start_line_offline
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.shot import ShotData, ShotSource
from golf_coach.contracts.unscored import INFERENCE_REASONS, UnscoredReason

#: The 7 iron's book loft, from `aaron.bag.json` as M15 P1 corrected it. Typed here rather than
#: read, for the reason `VALIDATION_SHOTS` is: a test that opens the bag is testing the bag.
_SEVEN_IRON_LOFT = 30.5


def _shot(**fields) -> ShotData:
    """A shot carrying only what a test names. Everything else is the screen printing nothing."""
    base = {
        "shot_id": "test-1",
        "session_id": "test",
        "source": ShotSource.SCREEN,
        "timestamp": datetime(2026, 8, 10, 12, tzinfo=UTC),
    }
    return ShotData(**{**base, **fields})


def _measured(shot: ShotData, **kwargs) -> dict[str, float]:
    """name -> value for whatever this shot records, through the registry rather than by hand."""
    flown = fly_shot(shot, **{"loft_deg": None, "handedness": None, **kwargs})
    return {
        name: value
        for name, (read, _unit, _detail) in FLIGHT_MEASUREMENTS.items()
        if (value := read(flown)) is not None
    }


# --------------------------------------------------------------------------- the happy path


def test_a_shot_with_a_printed_spin_records_the_four_model_outputs() -> None:
    """The screen printed everything, so nothing is solved and the flight is simply flown."""
    reference = VALIDATION_SHOTS[0]
    shot = _shot(
        ball_speed=reference.launch.ball_speed_mph,
        launch_angle=reference.launch.launch_angle_deg,
        spin_rate=reference.launch.spin_rpm,
        launch_direction=reference.launch.launch_direction_deg,
        spin_axis=reference.launch.spin_axis_deg,
        carry_distance=reference.simulator_carry_yds,
    )

    values = _measured(shot, handedness=Handedness.RIGHT)

    assert set(values) == {
        "flight_carry_yds",
        "flight_apex_yds",
        "flight_descent_angle_deg",
        "flight_time_s",
        "flight_landing_offline_yds",
    }
    # P5's re-flown gate figure, to the digit. Re-flown here rather than asserted loosely, because
    # the whole point of `gate_comparisons` re-flying is that a re-sourced table moves the number.
    assert values["flight_carry_yds"] == pytest.approx(122.3567, abs=1e-3)
    assert values["flight_apex_yds"] > 0.0
    assert 0.0 < values["flight_descent_angle_deg"] < 90.0
    assert values["flight_time_s"] > 0.0


def test_the_simulated_carry_is_not_the_printed_one() -> None:
    """The reason the names differ at all, asserted on the shot the gate is made of.

    `pooled_samples` groups by name; if these two agreed to the yard the distinction would look
    like pedantry. They differ by about 3 yards — ADR-027's gate is ±2.59% — and a personal mean
    over both would be a mean over a measurement and a model.
    """
    reference = VALIDATION_SHOTS[0]
    shot = _shot(
        ball_speed=reference.launch.ball_speed_mph,
        launch_angle=reference.launch.launch_angle_deg,
        spin_rate=reference.launch.spin_rpm,
        launch_direction=reference.launch.launch_direction_deg,
        spin_axis=reference.launch.spin_axis_deg,
        carry_distance=reference.simulator_carry_yds,
    )

    values = _measured(shot, handedness=Handedness.RIGHT)

    assert values["flight_carry_yds"] != pytest.approx(reference.simulator_carry_yds, abs=0.5)


def test_every_registered_name_carries_a_unit_and_says_it_is_simulated() -> None:
    """The detail string is the one line about a model output a reader is guaranteed to see.

    `analysis.baseline` averages by name and unit and knows nothing about a `source`, so a reader
    meeting `flight_carry_yds` beside `carry_distance_yds` has the detail and nothing else to tell
    a simulated number from a printed one.
    """
    for name, (_read, unit, detail) in FLIGHT_MEASUREMENTS.items():
        assert name.startswith("flight_"), f"{name} must be namespaced away from the measured ones"
        assert unit, f"{name} has no unit"
        assert "SIMULATED" in detail or "SOLVED" in detail, f"{name} does not say it is modelled"


# --------------------------------------------------------------------------- the two withholdings


def test_a_planar_flight_withholds_its_landing_offline() -> None:
    """⚠️ The identity M15 P10 measured on the corpus, and the reason for the rule.

    With the axis unresolved the ball is flown in the vertical plane, so its landing offline is
    `simulated carry * sin(start line)` — and `measure_start_line_offline` is the same sine over the
    *printed* carry. On a shot whose spin was solved **from** that printed carry the two carries are
    the same number by construction, so the two offlines agree to five decimals: two names for one
    quantity, which is the hazard ADR-027 §Decision 6 exists to prevent. So the assertion is in two
    halves — the number is withheld, *and* it would have been a duplicate.

    The shot is `2026-08-23-4`, the one shot on disk the solve can invert, typed from the screen the
    way `VALIDATION_SHOTS` is. Its axis is refused because the screen printed none: eleven of the
    thirteen stored shots are in exactly this state, which is what makes this the ordinary case
    rather than an edge one.
    """
    shot = _shot(
        ball_speed=89.8,
        launch_angle=22.4,
        launch_direction=2.8,
        carry_distance=126.1,
        club_face_angle=3.9,
        club_path=-1.1,
        shot_type="SLIGHT FADE",
    )

    flown = fly_shot(shot, loft_deg=_SEVEN_IRON_LOFT, handedness=Handedness.RIGHT)
    values = _measured(shot, loft_deg=_SEVEN_IRON_LOFT, handedness=Handedness.RIGHT)

    assert flown.flew and not flown.curve_is_drawn
    assert "flight_landing_offline_yds" not in values
    # What it would have been, had it recorded: the M9 number, to five decimal places.
    assert flown.flight is not None
    assert flown.flight.landing_offline_yds == pytest.approx(
        measure_start_line_offline(shot), abs=1e-4
    )


def test_a_measured_spin_makes_the_two_offlines_disagree_instead_of_duplicate() -> None:
    """⚠️ P11's refinement of that finding, and it argues the *same* rule harder.

    M15 P10 measured the identity on `2026-08-23-4` and read 2.6e-5 yd. That is not structural — it
    holds because a solved spin pins the simulated carry to the printed one. Where the spin was
    *measured* the two carries differ by the gate's own ±2.59%, and so do the two offlines: 0.29 yd
    apart on the shot below, with nothing available to say which of them is right.

    So the case for withholding is stronger on this half of the corpus, not weaker. A duplicate
    number pools badly; two numbers named for one quantity that quietly disagree is what
    `measure_total_distance` declines to create in so many words.
    """
    shot = _shot(
        ball_speed=90.7,
        launch_angle=20.9,
        spin_rate=5991.0,
        launch_direction=-5.3,
        carry_distance=125.6,
    )

    flown = fly_shot(shot, loft_deg=None, handedness=Handedness.RIGHT)
    values = _measured(shot, handedness=Handedness.RIGHT)

    assert flown.flew and not flown.curve_is_drawn
    assert "flight_landing_offline_yds" not in values
    assert flown.flight is not None
    printed = measure_start_line_offline(shot)
    assert printed is not None
    assert abs(flown.flight.landing_offline_yds - printed) == pytest.approx(0.29, abs=0.05)


def test_a_printed_spin_is_never_recorded_as_a_solved_one() -> None:
    """⚠️ P11's correction to §Decision 6, which named the measurement without splitting the two.

    A spin off the screen is a reading of a ball; a solved one is the spin this integrator needs to
    agree with a printed carry. Pooling them under one name is the thing the `flight_` prefix was
    introduced to stop, one layer in — on the corpus it would average a measured 5,991 rpm with a
    solved 2,924.
    """
    printed = _shot(ball_speed=90.7, launch_angle=20.9, spin_rate=5991.0, carry_distance=125.6)

    values = _measured(printed, handedness=Handedness.RIGHT)

    assert "flight_spin_rpm" not in values
    assert values["flight_carry_yds"] > 0.0, "the other outputs still record — they are the model's"


def test_a_solved_spin_is_recorded_under_its_own_name() -> None:
    """The other half: no spin on the screen, a loft to pick the branch, and a number comes back.

    `2026-08-23-4` again, and it is the *only* shot on disk this can be asserted on — the other ten
    spin-less shots print a carry above the peak of what their own launch conditions can fly. The
    number is P10's: 2,924 rpm under a 5,103 rpm cap.
    """
    solved = _shot(
        ball_speed=89.8,
        launch_angle=22.4,
        launch_direction=2.8,
        carry_distance=126.1,
    )

    values = _measured(solved, loft_deg=_SEVEN_IRON_LOFT, handedness=Handedness.RIGHT)

    assert values["flight_spin_rpm"] == pytest.approx(2924.0, abs=5.0)
    # The solve's own success criterion, restated as the reason the name is honest: this carry is
    # the printed one, reproduced, and not an independent prediction of it.
    assert values["flight_carry_yds"] == pytest.approx(126.1, abs=0.05)


# --------------------------------------------------------------------------- the refusals


def test_a_refused_flight_is_one_entry_and_not_five() -> None:
    """`get_session_summary` counts unscored entries by name, so five copies of one cause would
    report a session as five times more broken than it is."""
    no_loft = _shot(ball_speed=110.0, launch_angle=17.0, carry_distance=170.0)

    flown = fly_shot(no_loft, loft_deg=None, handedness=Handedness.RIGHT)
    entries = flight_unscored(flown)

    assert not flown.flew
    assert [entry.name for entry in entries] == ["flight_carry_yds"]
    assert entries[0].reason in INFERENCE_REASONS
    assert "none of the flight_* measurements recorded" in entries[0].detail


def test_a_refused_axis_is_a_second_entry_and_does_not_stop_the_flight() -> None:
    """ADR-027 §Decision 5's third branch, as it reaches `unscored`.

    The asymmetry is the whole reason `ShotFlight` carries two reason fields: a spin refusal costs
    the flight, an axis refusal costs one measurement.
    """
    shot = _shot(
        ball_speed=90.7,
        launch_angle=20.9,
        spin_rate=5991.0,
        launch_direction=-5.3,
        carry_distance=125.6,
    )

    flown = fly_shot(shot, loft_deg=None, handedness=Handedness.RIGHT)
    entries = flight_unscored(flown)

    assert flown.flew and flown.reason is None
    assert [entry.name for entry in entries] == ["flight_landing_offline_yds"]
    assert entries[0].reason is UnscoredReason.SPIN_AXIS_UNRESOLVED


def test_a_shot_with_no_launch_conditions_refuses_rather_than_raising() -> None:
    """The screen drops tiles one at a time, and a missing ball speed is not an exception."""
    flown = fly_shot(_shot(carry_distance=170.0), loft_deg=None, handedness=Handedness.RIGHT)

    assert not flown.flew
    assert flown.reason is UnscoredReason.NO_LAUNCH_CONDITIONS
    assert flight_unscored(flown)[0].name == "flight_carry_yds"


def test_an_unflyable_parse_is_caught_rather_than_taking_the_swing_down() -> None:
    """⚠️ The guard this phase exists to add, and the one `flight_for_shot` does not own.

    `simulate_flight` raises on a launch direction past a quarter turn, and names the caller as the
    boundary. Reaching `analyze_swing` that would discard pose that had already run and every
    checkpoint that had already scored — the failure `api.pipeline._shot_for` was widened to
    prevent, one layer in. A tile nobody can parse must not cost a golfer their swing.
    """
    absurd = _shot(
        ball_speed=90.7,
        launch_angle=20.9,
        spin_rate=5991.0,
        launch_direction=140.0,
        carry_distance=125.6,
    )

    flown = fly_shot(absurd, loft_deg=None, handedness=Handedness.RIGHT)

    assert not flown.flew
    assert flown.reason is UnscoredReason.NO_LAUNCH_CONDITIONS
    assert "quarter turn" in flown.detail


def test_a_flight_that_drew_its_curve_reports_nothing_unscored() -> None:
    """The empty case, which on the corpus today is no shot at all — worth pinning anyway, because
    `unscored` growing an entry on a complete flight is a silent way to make a swing look worse."""
    complete = _shot(
        ball_speed=90.7,
        launch_angle=20.9,
        spin_rate=5991.0,
        launch_direction=-5.3,
        spin_axis=2.5,
        carry_distance=125.6,
    )

    flown = fly_shot(complete, loft_deg=None, handedness=Handedness.RIGHT)

    assert flown.flew and flown.curve_is_drawn
    assert flight_unscored(flown) == []


# --------------------------------------------------------------------------- the provenance


def test_the_source_is_the_versioned_model_and_not_an_instrument() -> None:
    """The fourth `Measurement.source` prefix, and the first that names no instrument.

    Versioned with the artifact it evaluates, so a re-sourced coefficient table takes a new name
    rather than silently changing what the old one meant.
    """
    assert FLIGHT_SOURCE == "model:flight_v1"
    assert not FLIGHT_SOURCE.startswith(("pose:", "launch_monitor:", "population:"))


# ------------------------------------------------------- beside what the screen printed [M15 P16]


def test_every_pair_names_a_measurement_in_both_registries() -> None:
    """The pairing is two registry names, and a rename in either must not go quiet.

    `compare_to_printed` looks both up rather than reading a stored value, so a renamed measurement
    raises here rather than dropping a row on a page. The `quantity` beside them is what a surface
    places the printed number by, and there are exactly two axes to draw on.
    """
    for simulated_name, measured_name, quantity in _COMPARISON_PAIRS:
        assert simulated_name in FLIGHT_MEASUREMENTS
        assert measured_name in SHOT_MEASUREMENTS
        assert quantity in {"down_range", "offline"}


def test_a_printed_spin_makes_the_carry_an_independent_check() -> None:
    """Half the flights this corpus can produce, and the only half where a gap is an error.

    The screen printed the spin, so the carry beside it was measured by something that did not use
    it — which is the gate, per shot. `2026-08-10-1` disagrees by about 3 yd.
    """
    reference = VALIDATION_SHOTS[0]
    shot = _shot(
        ball_speed=reference.launch.ball_speed_mph,
        launch_angle=reference.launch.launch_angle_deg,
        spin_rate=reference.launch.spin_rpm,
        launch_direction=reference.launch.launch_direction_deg,
        spin_axis=reference.launch.spin_axis_deg,
        carry_distance=reference.simulator_carry_yds,
    )

    flown = fly_shot(shot, loft_deg=None, handedness=Handedness.RIGHT)
    carry = next(
        row for row in compare_to_printed(flown, shot) if row.quantity == "down_range"
    )

    assert carry.comparable
    assert carry.difference == pytest.approx(-3.24, abs=0.05)
    assert "error" in carry.reading


def test_a_solved_spin_makes_the_carry_no_check_at_all() -> None:
    """⚠️ The row this phase exists to refuse to draw as a validation.

    The printed carry is the *input* to the solve, so the flown carry reproduces it to the root
    search's own tolerance. Two numbers agreeing to a thousandth of a yard is the most convincing
    thing on the page and it is evidence of nothing — `2026-08-23-4` is the shot, and the sentence
    beside the row is `circular_carry_note`, which the CLI has printed since M15 P7.
    """
    solved = _shot(
        ball_speed=89.8, launch_angle=22.4, launch_direction=2.8, carry_distance=126.1
    )

    flown = fly_shot(solved, loft_deg=_SEVEN_IRON_LOFT, handedness=Handedness.RIGHT)
    carry = next(
        row for row in compare_to_printed(flown, solved) if row.quantity == "down_range"
    )

    assert not carry.comparable
    assert abs(carry.difference) < 0.01, "the solve reproduces its own input, as it must"
    assert "by construction" in carry.reading


def test_the_two_offline_numbers_are_never_a_comparison() -> None:
    """⚠️ They are two different points, and the gap between them decomposes exactly.

    `start_line_offline_yds` is where the ball *started*, projected out to the printed carry;
    `flight_landing_offline_yds` is where the model has it finishing. So the gap is the bend the
    screen never measured **plus** the carry row's own disagreement leaning on the start line, and
    the reading splits it that way rather than calling all of it curve. The identity below is
    `FlightResult.landing_offline_m` inverted, and it is what makes the split exact.
    """
    reference = VALIDATION_SHOTS[1]
    shot = _shot(
        ball_speed=reference.launch.ball_speed_mph,
        launch_angle=reference.launch.launch_angle_deg,
        spin_rate=reference.launch.spin_rpm,
        launch_direction=reference.launch.launch_direction_deg,
        spin_axis=reference.launch.spin_axis_deg,
        carry_distance=reference.simulator_carry_yds,
    )

    flown = fly_shot(shot, loft_deg=None, handedness=Handedness.RIGHT)
    offline = next(row for row in compare_to_printed(flown, shot) if row.quantity == "offline")

    assert not offline.comparable
    assert flown.flight is not None
    direction = radians(flown.flight.launch.launch_direction_deg)
    bend = flown.flight.curvature_yds * cos(direction)
    lean = (flown.flight.carry_yds - reference.simulator_carry_yds) * sin(direction)
    assert offline.difference == pytest.approx(bend + lean, abs=1e-9)
    # And the bend is most of it, which is why calling the whole gap an error would be wrong twice.
    assert abs(bend) > abs(lean) * 10


def test_a_planar_flight_offers_no_offline_row() -> None:
    """Nothing to compare, because the model withheld its half of the pair.

    A row here would have to print `flight_landing_offline_yds` after `FLIGHT_MEASUREMENTS`
    declined to record it — the page's comparison block saying what the artifact refused to.
    """
    shot = _shot(ball_speed=90.7, launch_angle=20.9, spin_rate=5991.0, carry_distance=125.6)

    flown = fly_shot(shot, loft_deg=None, handedness=Handedness.RIGHT)
    rows = compare_to_printed(flown, shot)

    assert flown.flew and not flown.curve_is_drawn
    assert [row.quantity for row in rows] == ["down_range"]


def test_a_refused_flight_compares_nothing() -> None:
    """There is no simulated number to set anything beside, and `reason` is what it owes instead."""
    unflyable = _shot(ball_speed=89.8, launch_angle=22.4, carry_distance=126.1)

    flown = fly_shot(unflyable, loft_deg=None, handedness=Handedness.RIGHT)

    assert not flown.flew
    assert compare_to_printed(flown, unflyable) == ()


def test_the_circular_carry_sentence_has_one_definition() -> None:
    """A column of them and one row of them are the same claim, so they are one function.

    The CLI prints the column form under `--shots`; the flight route sends the row form to
    `flight.html`. `analysis/flight_caveats.py`'s docstring is the standing argument — prose this
    load-bearing existing twice is prose that goes stale in one place.
    """
    column, row = circular_carry_note(), circular_carry_note(0.001)

    claim = "the printed carry is the *input* to the spin solve"
    assert claim in column and claim in row
    assert "no error column" in column
    assert "+0.001 yd" in row
    assert "\n" not in column and "\n" not in row, "one paragraph; the terminal owns the wrapping"
