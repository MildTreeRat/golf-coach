"""The four sentences a drawn flight is never quoted without. [M15 P14]

What is pinned here is that the numbers inside them are *derived* and that the composition rule is
one rule. The prose itself is not pinned word for word — a caveat is meant to be rewritten as the
model's limits become better understood, and a test asserting an exact paragraph would make every
such rewrite a two-file edit for no protection. What would actually rot is a threshold typed into
a sentence, so that is what these read back off the model.

They also pin the shape the JSON surface depends on: one paragraph, no line breaks. The CLI wraps
these to 72 columns and `api/flight_view.py` sends them as strings, and a caveat carrying this
repo's own source indentation into a `<p>` would arrive on the page as ragged whitespace.
"""

from __future__ import annotations

import pytest

from golf_coach.analysis.benchmarks import load_flight_model
from golf_coach.analysis.flight import (
    GATE_AGREEMENT_FRACTION,
    VALIDATION_SHOTS,
    LaunchConditions,
    simulate_flight,
)
from golf_coach.analysis.flight_caveats import (
    altitude_caveat,
    caveats_for,
    clamp_caveat,
    gate_caveat,
    inferred_caveat,
    planar_caveat,
)
from golf_coach.analysis.flight_infer import SpinSource, flight_for_shot
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.shot import ShotData, ShotSource

#: The lower-spin validation shot, which is the one every other flight test starts from.
_GATE_SHOT = VALIDATION_SHOTS[0]


def _shot(**fields) -> ShotData:
    """One parsed screen. Defaults are `2026-08-23-7`'s, a real 3 wood with no spin printed."""
    base = {
        "shot_id": "t-1",
        "session_id": "t",
        "timestamp": "2026-08-23T10:00:00Z",
        "source": ShotSource.SCREEN,
        "ball_speed": 100.9,
        "launch_angle": 17.0,
        "carry_distance": 144.5,
        "launch_direction": 4.6,
        "shot_type": "FADE",
    }
    return ShotData.model_validate(base | fields)


def test_the_clamp_threshold_is_read_off_the_table_and_not_typed() -> None:
    """A re-sourced coefficient table has to move this sentence, or the sentence is a lie.

    Both validation shots fly their whole path above the published rows (ADR-027's first
    addendum), so the fully-clamped branch is the one every real shot on this corpus takes.
    """
    model = load_flight_model()
    result = simulate_flight(_GATE_SHOT.launch)
    assert result.fully_clamped

    said = clamp_caveat(result, model)

    assert f"{model.coefficients.spin_ratio_max:g}" in said
    assert "driver" in said


def test_the_gate_caveat_carries_the_pinned_agreement_and_the_inversion() -> None:
    """Never the percentage on its own: the two shots pass individually and rank backwards."""
    said = gate_caveat()

    assert f"{GATE_AGREEMENT_FRACTION:.2%}" in said
    assert "backwards" in said


def test_the_inferred_caveat_quotes_this_shot_s_own_cap() -> None:
    """The cap is per shot — `carry_window` measures it — so it cannot be a constant in prose."""
    resolved = flight_for_shot(_shot(), loft_deg=21.0, handedness=Handedness.RIGHT)
    assert resolved.spin is not None

    said = inferred_caveat(resolved)

    assert f"{resolved.spin.cap_rpm:.0f}" in said
    assert "not measured" in said


def test_the_altitude_caveat_names_the_altitude_in_both_units() -> None:
    said = altitude_caveat(1609.0)

    assert "1609 m" in said
    assert "5279 ft" in said
    assert "sea-level" in said


def test_every_caveat_is_one_paragraph() -> None:
    """The JSON surface's requirement, and the reason `_said` exists at all."""
    model = load_flight_model()
    result = simulate_flight(_GATE_SHOT.launch)

    for said in (
        clamp_caveat(result, model),
        gate_caveat(),
        planar_caveat(),
        altitude_caveat(500.0),
    ):
        assert "\n" not in said
        assert "  " not in said
        assert said == said.strip()


def test_a_measured_spin_drawn_with_its_axis_owes_exactly_the_two_standing_caveats() -> None:
    """The floor: every flight owes the clamp and the gate, and this one owes nothing else.

    `2026-08-10-1` is the only shape in the corpus that reaches it — a printed spin *and* a
    printed axis — which is why the assertion is about a gate shot and not about a bay shot.
    """
    shot = _shot(
        ball_speed=90.7,
        launch_angle=20.9,
        spin_rate=5991.0,
        carry_distance=125.6,
        launch_direction=-5.3,
        spin_axis=2.5,
    )
    resolved = flight_for_shot(shot, loft_deg=30.5, handedness=Handedness.RIGHT)
    assert resolved.launch is not None
    assert resolved.spin_source is SpinSource.MEASURED
    assert resolved.curve_is_drawn

    said = caveats_for(simulate_flight(resolved.launch), resolved, load_flight_model())

    assert len(said) == 2


def test_a_planar_flight_with_a_solved_spin_owes_all_four() -> None:
    """The corpus's own commonest shape, and the two extra sentences are why it is not the floor.

    No axis on the screen, so the curve is not drawn; no spin either, so the number on the line
    was solved for. Both are facts a viewer must not have to derive from the numbers themselves.
    """
    resolved = flight_for_shot(_shot(), loft_deg=21.0, handedness=Handedness.RIGHT)
    assert resolved.launch is not None
    assert resolved.spin_source is SpinSource.INFERRED
    assert not resolved.curve_is_drawn

    said = caveats_for(simulate_flight(resolved.launch), resolved, load_flight_model())

    assert len(said) == 4
    assert any("vertical plane" in s for s in said)
    assert any("not measured" in s for s in said)


def test_the_altitude_caveat_joins_the_list_only_when_the_flight_left_sea_level() -> None:
    model = load_flight_model()
    resolved = flight_for_shot(_shot(), loft_deg=21.0, handedness=Handedness.RIGHT)
    assert resolved.launch is not None
    result = simulate_flight(resolved.launch)

    assert len(caveats_for(result, resolved, model, altitude_m=1609.0)) == 5
    assert len(caveats_for(result, resolved, model, altitude_m=0.0)) == 4


@pytest.mark.parametrize("count,expected", [(1, "this flight is"), (7, "7 of these flights are")])
def test_the_planar_caveat_agrees_with_itself_about_number(count, expected) -> None:
    """`--shots` says it once for a whole corpus and the route says it about one flight."""
    assert planar_caveat(count).startswith(expected)


def test_a_launch_below_the_table_reads_measured_rows_and_says_so() -> None:
    """The third clamp branch, which one shot on disk reaches and no synthetic test otherwise would.

    A low spin at a high ball speed keeps the spin ratio inside the published range, which is the
    only way to exercise the branch that says the coefficients were *read*.
    """
    model = load_flight_model()
    result = simulate_flight(
        LaunchConditions(ball_speed_mph=150.0, launch_angle_deg=12.0, spin_rpm=1800.0)
    )
    assert not result.fully_clamped

    said = clamp_caveat(result, model)

    assert "measured row" in said or "fell outside" in said
