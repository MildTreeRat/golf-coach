"""Solving spin backwards out of a carry: the shape of the curve, and what it refuses.

Like `test_flight.py` these run against the **real** committed constants on a base install. The
numbers are pins on the model as committed — a coefficient table that gets re-sourced moves them,
and that is exactly the event a reader should be told about.

Two things this file is written to hold in place, because both are findings rather than behaviour:

- **the honest test of ADR-027 §Decision 3.** The two 2026-08-10 shots carry a measured spin *and*
  a printed carry, so the solve can be checked against a truth. It returns 3,185 rpm against a
  recorded 5,991 on one and refuses the other outright. That is one fabricated answer and one
  refusal out of two, and if a future change ever makes those numbers agree, it is the *labelling*
  around the result that has to be revisited, not this test.
- **that no case invents a spin it does not have.** Four of the seven cases return nothing at all,
  and two of those four have infinitely many solutions rather than none. `docs/CODE_STANDARDS.md`
  R7 is the rule; the plateau cases are where it is easiest to break, because a plausible-looking
  representative is always available to return.

A window costs about forty flights (~0.5 s), so the two reference windows are measured once here
and shared, and the tests that need a *different* set of launch conditions say why they do.
"""

from __future__ import annotations

from functools import cache

import pytest

from golf_coach.analysis.benchmarks import load_flight_model
from golf_coach.analysis.flight import VALIDATION_SHOTS, simulate_flight
from golf_coach.analysis.spin_solve import (
    CarryWindow,
    SpinSolveCase,
    UnspunLaunch,
    carry_window,
    solve_spin_from_carry,
)

#: The two 2026-08-10 reference shots with their spin taken back off, which is the state the other
#: eleven shots on disk arrive in (ADR-027 §Context 2).
_UNSPUN = {shot.shot_id: UnspunLaunch.from_launch(shot.launch) for shot in VALIDATION_SHOTS}
_PRINTED_CARRY = {shot.shot_id: shot.simulator_carry_yds for shot in VALIDATION_SHOTS}
_MEASURED_SPIN = {shot.shot_id: shot.launch.spin_rpm for shot in VALIDATION_SHOTS}

#: `2026-08-23-2`: 83.8 mph off a **3.4°** launch, the shallow outlier of the corpus. It is here
#: because it is the one set of launch conditions on disk whose carry never falls — it rises to the
#: high plateau and stops — so it is the real-data case for the degenerate branch of the peak
#: search. Typed rather than read off disk: this package does not read artifacts (M15 P10 does).
_SHALLOW = UnspunLaunch(83.8, 3.4)


@cache
def _window(shot_id: str) -> CarryWindow:
    return carry_window(_UNSPUN[shot_id])


def test_the_carry_window_is_the_five_segments_the_addenda_measured() -> None:
    window = _window("2026-08-10-1")
    # Pinned to 1e-4 yd, four orders below the ~3 yd the model disagrees with HD Golf by.
    assert window.low_plateau_yds == pytest.approx(117.9927, abs=1e-4)
    assert window.peak_yds == pytest.approx(127.3982, abs=1e-4)
    assert window.high_plateau_yds == pytest.approx(122.3567, abs=1e-4)
    assert window.low_plateau_max_rpm == pytest.approx(1128.7, abs=1.0)
    assert window.peak_rpm == pytest.approx(2545.8, abs=2.0)
    assert window.high_plateau_min_rpm == pytest.approx(5154.1, abs=1.0)
    # The high plateau is the *end* of the fall and not the bottom of the range — ADR-027's
    # 2026-09-05c correction, which is the whole reason `floor_yds` is not simply the plateau.
    assert window.floor_yds == window.low_plateau_yds
    assert window.high_plateau_yds < window.peak_yds


def test_the_high_plateau_starts_exactly_where_the_launch_ratio_reaches_the_table() -> None:
    """The one edge that is derived rather than searched, so it is checked against the row itself.

    The launch spin ratio is the smallest in a flight, so it alone decides whether every step is
    clamped — which is what makes this edge analytic while the other one has to be bisected.
    """
    model = load_flight_model()
    window = _window("2026-08-10-1")
    launch = _UNSPUN["2026-08-10-1"]

    at_edge = simulate_flight(launch.at_spin(window.high_plateau_min_rpm))
    assert at_edge.spin_ratio_min == pytest.approx(model.coefficients.spin_ratio_max, abs=1e-9)
    assert at_edge.fully_clamped

    just_under = simulate_flight(launch.at_spin(window.high_plateau_min_rpm * 0.999))
    assert not just_under.fully_clamped, "a shade less spin has to read one measured row"


def test_the_low_plateau_ends_where_the_flight_first_climbs_into_the_table() -> None:
    model = load_flight_model()
    window = _window("2026-08-10-1")
    launch = _UNSPUN["2026-08-10-1"]

    at_edge = simulate_flight(launch.at_spin(window.low_plateau_max_rpm))
    assert at_edge.spin_ratio_max <= model.coefficients.spin_ratio_min
    assert at_edge.carry_yds == pytest.approx(window.low_plateau_yds, abs=1e-6)

    # Five rpm more and the ball touches the measured rows on the way down, which is what stops the
    # carry being flat. The move is tiny — 0.007 yd — and that is the point: this shoulder is found
    # on the mechanism (`spin_ratio_max`) rather than on a carry difference a tolerance could miss.
    above = simulate_flight(launch.at_spin(window.low_plateau_max_rpm + 5.0))
    assert above.spin_ratio_max > model.coefficients.spin_ratio_min
    assert above.carry_yds > window.low_plateau_yds


def test_both_plateaus_are_flat_rather_than_nearly_flat() -> None:
    """Every spin on a plateau flies one carry to the last bit, which is why neither is solvable."""
    window = _window("2026-08-10-1")
    launch = _UNSPUN["2026-08-10-1"]

    low = [0.0, window.low_plateau_max_rpm * 0.5, window.low_plateau_max_rpm]
    assert {simulate_flight(launch.at_spin(rpm)).carry_yds for rpm in low} == {
        window.low_plateau_yds
    }

    high = [window.high_plateau_min_rpm, window.high_plateau_min_rpm * 2.0, 30000.0]
    assert {simulate_flight(launch.at_spin(rpm)).carry_yds for rpm in high} == {
        window.high_plateau_yds
    }


def test_the_solve_returns_the_spin_a_flight_was_flown_at() -> None:
    """The one property here that is arithmetic and not a finding: fly a spin, solve its carry,
    get the spin back."""
    launch = _UNSPUN["2026-08-10-1"]
    window = _window("2026-08-10-1")
    for spin_rpm in (2800.0, 3500.0, 4500.0):
        carry_yds = simulate_flight(launch.at_spin(spin_rpm)).carry_yds
        solution = solve_spin_from_carry(launch, carry_yds, window=window)
        assert solution.case is SpinSolveCase.TWO_BRANCHES
        assert solution.falling_rpm == pytest.approx(spin_rpm, abs=1.0)
        # The other answer is real too, and is the reason a loft prior exists at all: the same
        # carry off the same face comes from a much lower spin on the near side of the peak.
        assert solution.rising_rpm is not None
        assert solution.rising_rpm < window.peak_rpm < solution.falling_rpm


def test_the_honest_test_of_decision_3_still_fails_the_way_it_failed() -> None:
    """ADR-027's own "honest test": the two shots where the true spin is on disk beside the carry.

    One answer 47% low and one refusal. This is not a regression pin on a bug — it is the evidence
    behind every sentence in the ADR that calls the inferred value a path-drawing device, and if it
    ever changes, that labelling is what has to be re-argued.
    """
    solution = solve_spin_from_carry(
        _UNSPUN["2026-08-10-1"], _PRINTED_CARRY["2026-08-10-1"], window=_window("2026-08-10-1")
    )
    assert solution.case is SpinSolveCase.TWO_BRANCHES
    assert solution.falling_rpm == pytest.approx(3185.0, abs=1.0)
    measured = _MEASURED_SPIN["2026-08-10-1"]
    assert solution.falling_rpm is not None
    assert (solution.falling_rpm - measured) / measured == pytest.approx(-0.468, abs=0.001)
    # And the truth is not merely off the answer — it is outside the range the carry can speak
    # about at all, which is the mechanism rather than the size of the error.
    assert measured > solution.window.high_plateau_min_rpm

    refused = solve_spin_from_carry(
        _UNSPUN["2026-08-10-2"], _PRINTED_CARRY["2026-08-10-2"], window=_window("2026-08-10-2")
    )
    assert refused.case is SpinSolveCase.BELOW_FLOOR
    assert refused.candidates == ()
    # 121.0 yd printed against a floor of 122.62: short by 1.6 yd, not by the 2.0 the 2026-09-05c
    # addendum measured, because P5's spin axis moved the floor and not the printed carry.
    short_by = refused.window.floor_yds - _PRINTED_CARRY["2026-08-10-2"]
    assert short_by == pytest.approx(1.62, abs=0.01)


def test_no_case_that_cannot_name_a_spin_returns_one() -> None:
    """R7, where it is easiest to break: two of these have infinitely many answers, not none."""
    launch = _UNSPUN["2026-08-10-1"]
    window = _window("2026-08-10-1")
    cases = {
        SpinSolveCase.ON_LOW_PLATEAU: window.low_plateau_yds,
        SpinSolveCase.ON_HIGH_PLATEAU: window.high_plateau_yds,
        SpinSolveCase.ABOVE_PEAK: window.peak_yds + 5.0,
        SpinSolveCase.BELOW_FLOOR: window.floor_yds - 5.0,
    }
    for case, target_yds in cases.items():
        solution = solve_spin_from_carry(launch, target_yds, window=window)
        assert solution.case is case
        assert solution.candidates == ()
        assert (solution.rising_rpm, solution.falling_rpm) == (None, None)


def test_a_carry_indistinguishable_from_a_plateau_is_not_solved_out_of_the_rounding() -> None:
    """HD Golf prints carry to 1 dp, so a target inside half a digit of a plateau says nothing
    about spin."""
    launch = _UNSPUN["2026-08-10-1"]
    window = _window("2026-08-10-1")
    for offset in (-0.04, 0.04):
        assert (
            solve_spin_from_carry(launch, window.high_plateau_yds + offset, window=window).case
            is SpinSolveCase.ON_HIGH_PLATEAU
        )
    # Just outside it, the two branches come back — the falling one within a few hundred rpm of the
    # cap, which is what M15 P9 has to report beside any answer this close to the plateau.
    outside = solve_spin_from_carry(launch, window.high_plateau_yds + 0.06, window=window)
    assert outside.case is SpinSolveCase.TWO_BRANCHES
    assert outside.falling_rpm is not None
    assert outside.falling_rpm < window.high_plateau_min_rpm


def test_the_peak_is_one_answer_and_not_two_collapsed_together() -> None:
    launch = _UNSPUN["2026-08-10-1"]
    window = _window("2026-08-10-1")
    solution = solve_spin_from_carry(launch, window.peak_yds + 0.02, window=window)
    assert solution.case is SpinSolveCase.AT_PEAK
    assert solution.candidates == (window.peak_rpm,)
    assert solution.rising_rpm == solution.falling_rpm == window.peak_rpm


def test_between_the_plateaus_one_branch_answers_and_the_other_never_reaches() -> None:
    """The band ADR-027's 2026-09-05c addendum found, and why a unique answer is not good news."""
    launch = _UNSPUN["2026-08-10-1"]
    window = _window("2026-08-10-1")
    solution = solve_spin_from_carry(
        launch, 0.5 * (window.low_plateau_yds + window.high_plateau_yds), window=window
    )
    assert solution.case is SpinSolveCase.BETWEEN_PLATEAUS
    assert len(solution.candidates) == 1
    assert solution.falling_rpm is None, "the fall stops at the high plateau, well above this carry"
    assert solution.rising_rpm is not None
    # 1,369 rpm: unique, and not a spin a 7 iron produces. The band it sits in runs from the low
    # plateau's shoulder to about 1,540 rpm.
    assert window.low_plateau_max_rpm < solution.rising_rpm < 1600.0


def test_a_shot_whose_carry_never_falls_has_no_two_branch_band_at_all() -> None:
    """`2026-08-23-2` at 3.4°, the corpus outlier — its peak *is* its high plateau.

    Two things ride on this one shot. It is the real-data case for the peak search walking to the
    edge of its interval rather than finding an interior hump, and it is the counter-example to
    reading `BETWEEN_PLATEAUS` as "implausibly low spin": here the band runs to 4,572 rpm and the
    answer lands at 2,307, which is an ordinary iron number.
    """
    window = carry_window(_SHALLOW)
    assert window.peak_yds == window.high_plateau_yds
    assert window.peak_rpm == window.high_plateau_min_rpm

    solution = solve_spin_from_carry(_SHALLOW, 33.6, window=window)
    assert solution.case is SpinSolveCase.BETWEEN_PLATEAUS
    assert solution.rising_rpm == pytest.approx(2306.6, abs=1.0)
    # And the correction it makes to the 2026-09-05b addendum: 33.6 yd was called "25% below
    # anything the model can fly" against a floor of 44.8, which was the high plateau. The real
    # floor is the low-spin clamp, 26.9 yd, and the printed carry is inside the range after all.
    assert window.low_plateau_yds == pytest.approx(26.89, abs=0.01)
    assert window.floor_yds < 33.6 < window.peak_yds


def test_the_answer_does_not_move_with_the_integrator_step() -> None:
    """Halving and doubling the step changes nothing: the solve reads the model, not the RK4."""
    launch = _UNSPUN["2026-08-10-1"]
    answers = [
        solve_spin_from_carry(launch, 125.6, step_s=step).falling_rpm for step in (0.0025, 0.01)
    ]
    assert answers[0] == pytest.approx(3185.0, abs=1.0)
    assert answers[1] == pytest.approx(answers[0], abs=1.0)


def test_a_measured_window_can_be_handed_back_instead_of_being_re_flown() -> None:
    launch = _UNSPUN["2026-08-10-1"]
    with_window = solve_spin_from_carry(launch, 125.6, window=_window("2026-08-10-1"))
    without = solve_spin_from_carry(launch, 125.6)
    assert with_window == without


def test_dropping_the_spin_off_launch_conditions_keeps_everything_else() -> None:
    for shot in VALIDATION_SHOTS:
        unspun = UnspunLaunch.from_launch(shot.launch)
        assert unspun.at_spin(shot.launch.spin_rpm) == shot.launch
        # The lateral pair survives the round trip, which is what stops a solve quietly flying a
        # planar approximation of a shot that curved — the mistake M15 P5 found in the gate.
        assert (unspun.launch_direction_deg, unspun.spin_axis_deg) != (0.0, 0.0)


def test_launch_conditions_that_cannot_fly_are_refused_by_the_integrator_that_owns_the_check() -> (
    None
):
    """No second copy of `simulate_flight`'s validation — the solve inherits it by calling it."""
    with pytest.raises(ValueError, match="launch angle"):
        carry_window(UnspunLaunch(90.0, 0.0))
    with pytest.raises(ValueError, match="ball speed"):
        carry_window(UnspunLaunch(0.0, 20.0))
