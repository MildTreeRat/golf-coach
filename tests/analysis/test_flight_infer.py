"""The loft prior, the axis fallback, and the two things a number must be reported beside.

Like `test_spin_solve.py` these run against the **real** committed constants on a base install, so
a re-sourced coefficient table moves them and a reader is told about it.

Three findings this file exists to hold in place, none of them behaviour:

- **the branch rule does not depend on where its one constant sits.** Everything at fairway-wood
  loft and above takes the falling branch, everything at driver loft refuses, and there is no club
  in between — so `LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG` may be anywhere in the gap. That is the
  honest form of a coarse prior, and it is asserted rather than claimed in a comment.
- **the corpus mostly refuses, and M15 P10 corrected why.** No *shot* carries a club — a launch
  monitor reports a ball — but the swing it arrived with does, and eleven of the thirteen on disk
  are tagged. What is missing is a declared loft on the bag entry behind the tag, so `NO_CLUB_LOFT`
  is still what the two-branch case usually produces; the loft path is exercised here against
  synthetic lofts, because `analysis/` may not open an artifact to find a real one.
- **the axis fallback resolves a direction and not an axis.** A face-to-path with no measured axis
  beside it must never come back with a number, however tempting the degrees sitting right there.

`honest_test` and `gate_ordering` each cost a handful of flights and `carry_window` costs about
forty, so windows are measured once and shared.
"""

from __future__ import annotations

from datetime import UTC, datetime
from functools import cache
from math import radians, sin

import pytest

from golf_coach.analysis.flight import VALIDATION_SHOTS, simulate_flight
from golf_coach.analysis.flight_infer import (
    LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG,
    GateOrdering,
    InferredSpin,
    ShotFlight,
    SpinAxisSource,
    SpinSource,
    flight_for_shot,
    gate_ordering,
    honest_test,
    infer_spin,
    infer_spin_axis,
)
from golf_coach.analysis.shot_measure import measure_start_line_offline
from golf_coach.analysis.spin_solve import CarryWindow, SpinSolveCase, UnspunLaunch, carry_window
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.intent import TargetShape
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.unscored import (
    INFERENCE_REASONS,
    UNSCORED_REASONS,
    UnscoredReason,
)

#: `2026-08-10-1` with its spin taken back off — the one set of launch conditions on disk that
#: lands in the two-solution band, which is the only case a loft prior has anything to do in.
_REFERENCE = UnspunLaunch.from_launch(VALIDATION_SHOTS[0].launch)

#: A 7 iron's book loft, from `aaron.bag.json` (M15 P1). Typed rather than read: this package does
#: not open artifacts, and the point of the constant here is that it is comfortably above the
#: threshold rather than that it is this golfer's.
_IRON_LOFT_DEG = 30.5
#: A driver, the one club whose own spin sits near the peak.
_DRIVER_LOFT_DEG = 10.5


@cache
def _window() -> CarryWindow:
    return carry_window(_REFERENCE)


def _solve(carry_yds: float, loft_deg: float | None) -> InferredSpin:
    return infer_spin(_REFERENCE, carry_yds, loft_deg, window=_window())


# ---------------------------------------------------------------------------------------------
# The loft prior
# ---------------------------------------------------------------------------------------------


def test_a_lofted_club_takes_the_falling_branch_and_says_which_one_it_did_not() -> None:
    """The one case that produces a number, and the branch not taken stays in the detail.

    A reader meeting 3,185 rpm has no way to tell it apart from a measurement unless the record
    says a second spin flew the same carry. The alternative is in `detail` for that reason, and the
    solve itself keeps both on `InferredSpin.solution`.
    """
    inferred = _solve(VALIDATION_SHOTS[0].simulator_carry_yds, _IRON_LOFT_DEG)
    assert inferred.reason is None
    assert inferred.spin_rpm == pytest.approx(3185.0, abs=1.0)
    assert inferred.solution.case is SpinSolveCase.TWO_BRANCHES
    assert inferred.solution.rising_rpm == pytest.approx(1904.6, abs=1.0)
    assert "1905 rpm is the branch not taken" in inferred.detail


def test_the_branch_chosen_does_not_depend_on_where_the_loft_floor_sits() -> None:
    """The prior is coarse and the margin is enormous, and this is what makes that checkable.

    Drivers are built to about 12 degrees and fairway woods start at about 15, so the threshold
    lives in a gap no club occupies. Everything above the gap must answer the same way and
    everything below it must refuse the same way — which means moving the constant anywhere inside
    the gap changes nothing, and no test has to be edited when it moves.
    """
    assert 12.0 < LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG < 15.0

    carry = VALIDATION_SHOTS[0].simulator_carry_yds
    falling = _solve(carry, _IRON_LOFT_DEG).spin_rpm
    for loft in (15.0, 18.0, 21.0, 30.5, 46.0, 58.0):
        assert _solve(carry, loft).spin_rpm == falling, f"{loft} deg took a different branch"
    for loft in (8.0, 9.5, 10.5, 12.0):
        assert _solve(carry, loft).reason is UnscoredReason.SPIN_NOT_RECOVERABLE


def test_a_driver_refuses_because_its_own_spin_sits_at_the_peak() -> None:
    """Not "no answer" but "both answers", which is a different sentence and a different detail.

    The two candidates straddle a driver's own spin rate, so the comparison the prior is built on
    has no sign. ADR-010 §2 applied to an inference: no number beats a wrong one.
    """
    inferred = _solve(VALIDATION_SHOTS[0].simulator_carry_yds, _DRIVER_LOFT_DEG)
    assert inferred.spin_rpm is None
    assert inferred.reason is UnscoredReason.SPIN_NOT_RECOVERABLE
    assert "spins about as fast as" in inferred.detail


def test_two_branches_with_no_loft_is_the_one_refusal_a_golfer_can_clear() -> None:
    """And it is what every two-branch shot on disk does, because none of them carries a club.

    `NO_CLUB_LOFT` is separated from the other three inference reasons for exactly this: its remedy
    is an action on the results page, and collapsing it into "nothing to fix" would hide the one
    thing that is.
    """
    inferred = _solve(VALIDATION_SHOTS[0].simulator_carry_yds, None)
    assert inferred.spin_rpm is None
    assert inferred.reason is UnscoredReason.NO_CLUB_LOFT
    assert not UNSCORED_REASONS[inferred.reason].refilming_helps
    assert "no loft is on record" in inferred.detail


# ---------------------------------------------------------------------------------------------
# The six cases that never reach the prior
# ---------------------------------------------------------------------------------------------


def test_a_carry_no_spin_can_fly_is_a_finding_at_either_end() -> None:
    """Above the peak and below the floor are the same reason and different details.

    They are one member because the reader does the same thing about both — nothing — and two
    details because they are opposite statements about the shot. `SpinSolveCase` keeps them apart
    where a consumer that cares can branch on it.
    """
    window = _window()
    above = _solve(window.peak_yds + 5.0, _IRON_LOFT_DEG)
    assert above.reason is UnscoredReason.CARRY_UNREACHABLE
    assert above.solution.case is SpinSolveCase.ABOVE_PEAK
    assert "peak of what these launch conditions can fly" in above.detail

    below = _solve(window.floor_yds - 5.0, _IRON_LOFT_DEG)
    assert below.reason is UnscoredReason.CARRY_UNREACHABLE
    assert below.solution.case is SpinSolveCase.BELOW_FLOOR
    assert "floor of what these launch conditions can fly" in below.detail


def test_neither_plateau_invents_a_representative_spin() -> None:
    """The flat intervals are where a plausible-looking answer is always available to return.

    Infinitely many spins fly these carries, so `docs/CODE_STANDARDS.md` R7 says the honest answer
    is the case rather than a member of the set — and a loft prior cannot rescue it either, because
    a prior narrows a set of two and not a continuum. Passing one changes nothing here, which is
    the part worth pinning.
    """
    window = _window()
    for carry, case in (
        (window.low_plateau_yds, SpinSolveCase.ON_LOW_PLATEAU),
        (window.high_plateau_yds, SpinSolveCase.ON_HIGH_PLATEAU),
    ):
        for loft in (None, _IRON_LOFT_DEG):
            inferred = _solve(carry, loft)
            assert inferred.spin_rpm is None
            assert inferred.solution.case is case
            assert inferred.reason is UnscoredReason.SPIN_NOT_RECOVERABLE


def test_the_unique_answer_is_refused_against_the_peak_and_not_against_the_case_name() -> None:
    """ADR-027's 2026-09-05f addendum asked for this specifically, and the detail is the assertion.

    Its predecessor refused `BETWEEN_PLATEAUS` on the ground that the band lies at spins no 7 iron
    makes, and then found a shot where the band reaches 4,572 rpm and the answer is an ordinary
    number. So the argument has to be made from where the answer sits. It sits on the rising branch
    below the peak, and the prior says a lofted club spins above it — a stated comparison that a
    differently-shaped curve could fail, rather than a branch on the enum.
    """
    window = _window()
    between = 0.5 * (window.low_plateau_yds + window.high_plateau_yds)
    inferred = _solve(between, _IRON_LOFT_DEG)

    assert inferred.solution.case is SpinSolveCase.BETWEEN_PLATEAUS
    assert inferred.solution.falling_rpm is None, "the case is the falling branch not reaching"
    assert inferred.spin_rpm is None
    assert inferred.reason is UnscoredReason.SPIN_NOT_RECOVERABLE
    assert "below the" in inferred.detail and "rpm peak" in inferred.detail
    assert f"{window.peak_rpm:.0f}" in inferred.detail


def test_the_peak_itself_needs_no_prior_because_there_is_no_branch_to_choose() -> None:
    """One spin flies it, so loft has nothing to do — and the answer comes back without a loft.

    Never seen on a real shot; it is here because a knife-edge case that silently required a loft
    would refuse every stored shot for a reason that was not true of it.
    """
    inferred = _solve(_window().peak_yds + 0.02, None)
    assert inferred.solution.case is SpinSolveCase.AT_PEAK
    assert inferred.reason is None
    assert inferred.spin_rpm == pytest.approx(_window().peak_rpm)


def test_every_refusal_this_module_makes_is_one_nobody_should_be_re_filmed_for() -> None:
    """ADR-027 §Decision 5's last paragraph, as an assertion over the cases rather than a promise.

    A screen that withheld a spin field is not a camera problem. This walks the reasons the module
    can actually emit rather than the enum, because the enum contains plenty that are.
    """
    window = _window()
    carries = (
        window.peak_yds + 5.0,
        window.floor_yds - 5.0,
        window.low_plateau_yds,
        window.high_plateau_yds,
        0.5 * (window.low_plateau_yds + window.high_plateau_yds),
        VALIDATION_SHOTS[0].simulator_carry_yds,
    )
    emitted = {
        _solve(carry, loft).reason
        for carry in carries
        for loft in (None, _DRIVER_LOFT_DEG, _IRON_LOFT_DEG)
    } - {None}
    assert emitted <= INFERENCE_REASONS
    assert all(not UNSCORED_REASONS[reason].refilming_helps for reason in emitted)


# ---------------------------------------------------------------------------------------------
# The cap, the honest test and the gate
# ---------------------------------------------------------------------------------------------


def test_the_cap_is_reported_on_a_refusal_as_well_as_on_an_answer() -> None:
    """Because the refusals are where it explains the most.

    A shot refused for sitting on the high plateau is refused *by* the cap; a consumer that could
    only read the cap off a successful answer would have nothing to say about the case where it did
    all the work.
    """
    window = _window()
    for carry in (window.high_plateau_yds, VALIDATION_SHOTS[0].simulator_carry_yds):
        assert _solve(carry, _IRON_LOFT_DEG).cap_rpm == pytest.approx(5154.1, abs=1.0)
    assert not _solve(VALIDATION_SHOTS[0].simulator_carry_yds, _IRON_LOFT_DEG).at_cap


def test_the_honest_test_still_fails_and_this_is_the_shape_of_the_failure() -> None:
    """ADR-027 §Decision 3 checked against the only two shots that can check it.

    One refusal and one answer 46.8% low. **If a future change ever makes these agree, it is the
    labelling that has to be revisited and not this test** — an inferred spin that happened to be
    right on two shots is still not a measurement of spin.
    """
    results = {result.shot_id: result for result in honest_test(loft_deg=_IRON_LOFT_DEG)}

    first = results["2026-08-10-1"]
    assert first.measured_spin_rpm == 5991.0
    assert first.inferred.spin_rpm == pytest.approx(3185.0, abs=1.0)
    assert first.error_fraction == pytest.approx(-0.468, abs=0.001)

    second = results["2026-08-10-2"]
    assert second.inferred.spin_rpm is None
    assert second.inferred.reason is UnscoredReason.CARRY_UNREACHABLE
    assert second.error_fraction is None

    # The mechanism rather than the size: above the cap the carry has stopped depending on spin, so
    # no solve of any quality recovers either of these.
    assert all(result.measured_above_cap for result in results.values())


def test_the_honest_test_defaults_to_the_corpus_state_which_is_no_loft() -> None:
    """No shot on disk carries a club, so the default has to refuse for want of one.

    A default loft here would have made the two-branch shot look answerable on a corpus where it
    is not, which is the same class of mistake as filling in a missing measurement.
    """
    results = {result.shot_id: result for result in honest_test()}
    assert results["2026-08-10-1"].inferred.reason is UnscoredReason.NO_CLUB_LOFT
    assert results["2026-08-10-2"].inferred.reason is UnscoredReason.CARRY_UNREACHABLE


def test_the_gate_ranks_the_shots_backwards_and_the_residual_beats_the_effect() -> None:
    """The reading `GATE_AGREEMENT_FRACTION` on its own conceals, measured rather than quoted.

    HD Golf has the lower-spin shot flying 4.6 yd further; this model has it 1.03 yd shorter. The
    residual is therefore larger than the difference it is failing to reproduce, which is what
    makes "accurate to 2.6%" the wrong sentence to print beside an inferred spin.
    """
    ordering = gate_ordering()
    assert ordering.inverted
    assert ordering.simulator_gap_yds == pytest.approx(4.6, abs=1e-9)
    assert ordering.model_gap_yds == pytest.approx(-1.030, abs=0.01)
    assert ordering.residual_yds == pytest.approx(5.630, abs=0.01)
    assert ordering.residual_yds > ordering.simulator_gap_yds


def test_the_recorded_spin_axis_closes_about_a_tenth_of_the_inversion() -> None:
    """M15 P5's finding, re-measured here rather than restated.

    It is the reason P4's "±2.5% is the size of the missing *spin* effect" was an
    over-attribution: a tenth of the gap was never spin's to explain, and M15 P6 ruled out the
    atmosphere as the free parameter that would close the rest. Both halves are re-flown by
    `gate_ordering`, so a re-sourced table moves them together.
    """
    ordering = gate_ordering()
    assert ordering.planar_model_gap_yds == pytest.approx(-1.664, abs=0.01)
    assert ordering.axis_share == pytest.approx(0.101, abs=0.005)
    assert ordering.planar_residual_yds > ordering.residual_yds


def test_the_ordering_is_measured_from_flights_and_not_from_stored_numbers() -> None:
    """A guard on the property `gate_comparisons` was written to have.

    Flying at a coarser step must move every field a little and none of them a lot; a field that
    did not move at all would be a constant that had been typed in at some point.
    """
    coarse = gate_ordering(step_s=0.02)
    fine = gate_ordering()
    assert coarse != fine
    assert isinstance(coarse, GateOrdering)
    assert coarse.residual_yds == pytest.approx(fine.residual_yds, abs=0.01)


# ---------------------------------------------------------------------------------------------
# The spin axis
# ---------------------------------------------------------------------------------------------


def _shot(**fields: object) -> ShotData:
    """A `ShotData` carrying only the tiles the axis resolution reads."""
    return ShotData(
        shot_id="t", session_id="s", timestamp=datetime(2026, 9, 5, tzinfo=UTC), **fields
    )


def test_a_measured_axis_is_flipped_by_handedness_and_not_before() -> None:
    """`+ = fade` is the golfer's sign and `+ = right` is the ball's; only handedness relates them.

    `LaunchConditions.spin_axis_deg` names M15 P9 as the owner of this flip, and
    `contracts/dispersion.py` is the precedent for why it is not pedantry — a camera-relative sign
    meeting a mixed-handedness corpus read every left-handed golfer as a gross fault.
    """
    shot = _shot(spin_axis=9.3, shot_type="FADE", club_face_angle=8.6, club_path=-4.6)

    right = infer_spin_axis(shot, Handedness.RIGHT)
    assert right.spin_axis_deg == pytest.approx(9.3)
    assert right.source is SpinAxisSource.MEASURED
    assert right.reason is None

    left = infer_spin_axis(shot, Handedness.LEFT)
    assert left.spin_axis_deg == pytest.approx(-9.3)
    assert left.curve_direction is TargetShape.FADE, "a fade is a fade whichever side you swing"


def test_an_axis_with_no_golfer_refuses_rather_than_assuming_right_handed() -> None:
    inferred = infer_spin_axis(_shot(spin_axis=9.3, shot_type="FADE"), None)
    assert inferred.spin_axis_deg is None
    assert inferred.reason is UnscoredReason.NO_HANDEDNESS
    assert inferred.curve_direction is TargetShape.FADE


def test_face_to_path_gives_a_direction_and_never_an_axis() -> None:
    """This phase's correction to ADR-027 §Decision 5, and the tempting number is right there.

    Face-to-path is in degrees and sits beside a spin axis in degrees, so a fallback that returned
    it would look entirely reasonable. The two shots where both are on disk are what rule it out:
    10.9 deg of face-to-path against a 2.5 deg axis on one and 13.2 against 9.3 on the other, which
    is three times the tilt per degree — and in the wrong direction, since the second shot spins
    more and more backspin tilts an axis less.
    """
    inferred = infer_spin_axis(
        _shot(shot_type="SLIGHT FADE", club_face_angle=3.1, club_path=-9.8), Handedness.RIGHT
    )
    assert inferred.spin_axis_deg is None
    assert inferred.source is None
    assert inferred.reason is UnscoredReason.SPIN_AXIS_UNRESOLVED
    assert inferred.curve_direction is TargetShape.FADE
    assert "no magnitude" in inferred.detail


def test_with_neither_tile_there_is_not_even_a_direction() -> None:
    inferred = infer_spin_axis(_shot(shot_type="FADE", club_path=-4.5), Handedness.RIGHT)
    assert inferred.reason is UnscoredReason.SPIN_AXIS_UNRESOLVED
    assert inferred.curve_direction is None
    assert inferred.screen_shape is TargetShape.FADE, "the screen still said which way it went"
    assert not inferred.sign_disagrees, "nothing to compare is not a disagreement"


def test_a_sign_that_contradicts_the_screen_is_flagged_and_never_overwritten() -> None:
    """One shot on disk does this — 114.8 mph, printed `SLIGHT FADE`, face-to-path -1.3 deg.

    ADR-027 §Decision 5 asks for a warning rather than a silent correction, because resolving a
    disagreement quietly is how ADR-014's original sign inversion survived as long as it did. So
    the derived direction stays as derived and the screen's word stays beside it.
    """
    inferred = infer_spin_axis(
        _shot(shot_type="SLIGHT FADE", club_face_angle=0.8, club_path=2.1), Handedness.RIGHT
    )
    assert inferred.curve_direction is TargetShape.DRAW
    assert inferred.screen_shape is TargetShape.FADE
    assert inferred.sign_disagrees


def test_the_two_recorded_axes_agree_with_the_word_on_the_screen() -> None:
    """Which is ADR-027 §Decision 5's first branch working, and M15 P5 noticed it before this.

    ADR-014's addendum recorded the axis as stored sign-inverted; both shots on disk now read
    `+` against a printed fade, so the inversion is gone and the branch is live.
    """
    for spin_axis, shot_type in ((2.5, "CENTER SLIGHT FADE"), (9.3, "FADE")):
        inferred = infer_spin_axis(
            _shot(spin_axis=spin_axis, shot_type=shot_type), Handedness.RIGHT
        )
        assert not inferred.sign_disagrees
        assert inferred.spin_axis_deg == pytest.approx(spin_axis)


# ---------------------------------------------------------------------------------------------
# One stored shot, resolved into something that can be flown
# ---------------------------------------------------------------------------------------------

#: `2026-08-10-1` as `ShotData` holds it — the tiles the HD Golf screen printed for that shot, in
#: the contract's own signs. Typed rather than read off disk for the reason `_IRON_LOFT_DEG` is:
#: `analysis/` opens no artifacts. That it reproduces `VALIDATION_SHOTS[0]` exactly is the point of
#: the first test below.
_REFERENCE_TILES: dict[str, object] = {
    "ball_speed": 90.7,
    "launch_angle": 20.9,
    "spin_rate": 5991.0,
    "launch_direction": -5.3,
    "spin_axis": 2.5,
    "carry_distance": 125.6,
    "shot_type": "CENTER SLIGHT FADE",
}


def _resolve(
    loft_deg: float | None = None, **overrides: object
) -> tuple[ShotData, ShotFlight]:
    """`flight_for_shot` over the reference tiles, right-handed unless a test says otherwise."""
    hand = overrides.pop("handedness", Handedness.RIGHT)
    shot = _shot(**{**_REFERENCE_TILES, **overrides})
    return shot, flight_for_shot(shot, loft_deg=loft_deg, handedness=hand)


def test_a_stored_shot_resolves_to_the_launch_conditions_the_gate_was_typed_from() -> None:
    """The check M15 P4's constant has never had, and it costs nothing.

    `VALIDATION_SHOTS` was typed by hand off two screens and is this model's only external
    reference. The same screens were parsed into `ShotData`, so resolving one through the ordinary
    path must land on the constant — every field, both lateral ones included. A mistyped digit in
    either would move a gate that has been quoted in five addenda.
    """
    _, resolved = _resolve()

    assert resolved.launch == VALIDATION_SHOTS[0].launch
    assert resolved.spin_source is SpinSource.MEASURED
    assert resolved.spin is None
    assert resolved.curve_is_drawn


def test_a_printed_spin_is_never_re_solved_however_solvable_the_carry_is() -> None:
    """The solve is for a missing number, not a second opinion.

    This shot's carry is solvable and its printed spin is 46.8% away from what the solve would say
    (`honest_test`), so a resolution order that ran the solve anyway would silently replace a
    measurement with a model output — the exact substitution `SpinSource` exists to make visible.
    """
    _, resolved = _resolve(loft_deg=_IRON_LOFT_DEG)

    assert resolved.launch is not None
    assert resolved.launch.spin_rpm == 5991.0
    assert resolved.spin_source is SpinSource.MEASURED


def test_a_missing_spin_is_solved_and_carries_the_solve_that_produced_it() -> None:
    """The one shot on disk this happens for, in the shape it happens in."""
    _, resolved = _resolve(loft_deg=_IRON_LOFT_DEG, spin_rate=None)

    assert resolved.spin_source is SpinSource.INFERRED
    assert resolved.spin is not None
    assert resolved.launch is not None
    assert resolved.launch.spin_rpm == resolved.spin.spin_rpm
    # The cap travels with it. A spin shown without one reads as a low-spin diagnosis.
    assert resolved.spin.cap_rpm > resolved.launch.spin_rpm


def test_a_refused_solve_produces_no_launch_and_keeps_the_case_that_refused_it() -> None:
    """A refusal is a finding, so it has to arrive with enough to say which finding."""
    _, resolved = _resolve(loft_deg=_IRON_LOFT_DEG, spin_rate=None, carry_distance=400.0)

    assert resolved.launch is None
    assert resolved.reason is UnscoredReason.CARRY_UNREACHABLE
    assert resolved.spin is not None
    assert resolved.spin.solution.case is SpinSolveCase.ABOVE_PEAK


def test_the_loft_the_join_could_not_find_refuses_the_flight_and_not_the_shot() -> None:
    """`NO_CLUB_LOFT` reaching here is the corpus's commonest solvable state (six 3 wood shots)."""
    _, resolved = _resolve(spin_rate=None)

    assert resolved.launch is None
    assert resolved.reason is UnscoredReason.NO_CLUB_LOFT


def test_an_unresolved_axis_flies_the_ball_flat_rather_than_refusing_to_fly_it() -> None:
    """ADR-027 §Decision 5's third branch: the two refusals are not the same kind of refusal.

    A missing axis costs the curve. A missing spin costs the flight. Collapsing them would throw
    away a carry, an apex and a descent angle over a lateral number nobody asked for.
    """
    _, resolved = _resolve(spin_axis=None, club_face_angle=8.6, club_path=-4.6)

    assert resolved.launch is not None
    assert resolved.launch.spin_axis_deg == 0.0
    assert resolved.reason is None
    assert resolved.axis.reason is UnscoredReason.SPIN_AXIS_UNRESOLVED
    assert not resolved.curve_is_drawn
    # The direction survives even though the magnitude did not — a fade drawn straight is still
    # known to be a fade.
    assert resolved.axis.curve_direction is TargetShape.FADE


def test_a_planar_flights_offline_is_the_start_line_measurement_by_a_longer_route() -> None:
    """**Why `flight_landing_offline_yds` must not be recorded on a planar flight** (M15 P11).

    With the axis at zero the landing offline is `carry * sin(start line)` to the last bit — which
    is `shot_measure.measure_start_line_offline`, a quantity recorded since M9. Two names for one
    number is exactly the pooling hazard ADR-027 §Decision 6 exists to prevent, and this is the
    measurement of it rather than the worry about it.
    """
    shot, resolved = _resolve(spin_axis=None)
    assert resolved.launch is not None
    result = simulate_flight(resolved.launch)

    assert result.curvature_yds == pytest.approx(0.0, abs=1e-9)
    assert result.landing_offline_yds == pytest.approx(
        result.carry_yds * sin(radians(resolved.launch.launch_direction_deg)), abs=1e-9
    )
    # And against the number this repo already stores for the same shot, which differs only by the
    # model's own disagreement with the printed carry.
    printed = measure_start_line_offline(shot)
    assert printed is not None
    assert result.landing_offline_yds == pytest.approx(printed, rel=0.03)


def test_a_left_handed_golfers_fade_curves_the_other_way_before_it_is_flown() -> None:
    """The flip `LaunchConditions` names M15 P9 as the owner of, reaching the integrator."""
    _, right = _resolve()
    _, left = _resolve(handedness=Handedness.LEFT)

    assert right.launch is not None and left.launch is not None
    assert right.launch.spin_axis_deg == 2.5
    assert left.launch.spin_axis_deg == -2.5


def test_a_screen_that_withheld_the_launch_itself_says_so_rather_than_crashing() -> None:
    """Fires on nothing in this corpus, and the OCR drops tiles one at a time (one shot on disk
    is already missing its face angle), so the path exists rather than the crash."""
    for missing in ({"ball_speed": None}, {"launch_angle": None}):
        _, resolved = _resolve(**missing)
        assert resolved.launch is None
        assert resolved.reason is UnscoredReason.NO_LAUNCH_CONDITIONS


def test_a_ball_that_rolls_is_refused_here_rather_than_raised_two_frames_later() -> None:
    """`simulate_flight` names this check as the caller's to own, and this is the caller."""
    _, resolved = _resolve(launch_angle=0.0)

    assert resolved.reason is UnscoredReason.NO_LAUNCH_CONDITIONS
    assert "rolls" in resolved.detail


def test_no_spin_and_no_carry_is_a_shot_with_nothing_to_solve_from() -> None:
    """The carry is the solve's only input, so its absence is a launch problem and not a solve
    that failed — and reporting it as `carry_unreachable` would blame a physics disagreement for a
    tile that never parsed."""
    _, resolved = _resolve(loft_deg=_IRON_LOFT_DEG, spin_rate=None, carry_distance=None)

    assert resolved.reason is UnscoredReason.NO_LAUNCH_CONDITIONS
    assert resolved.spin is None


def test_every_refusal_a_stored_shot_can_produce_is_still_one_nobody_re_films_for() -> None:
    """The same guarantee as the solve's, over the reasons this function can emit.

    A launch monitor withholding a tile is not a camera fault, and a golfer told to re-shoot a
    swing over it would be re-shooting the wrong thing — the screen has cleared by then anyway.
    """
    emitted = {
        _resolve(**case)[1].reason
        for case in (
            {},
            {"spin_rate": None},
            {"loft_deg": _IRON_LOFT_DEG, "spin_rate": None, "carry_distance": 400.0},
            {"ball_speed": None},
            {"launch_angle": 0.0},
        )
    } - {None}

    assert emitted <= INFERENCE_REASONS
    assert all(not UNSCORED_REASONS[reason].refilming_helps for reason in emitted)
