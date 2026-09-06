"""The gate: this model against the only two independent carries anyone has. [M15 P4, P5]

`test_flight.py` pins the physics and says explicitly that the gate is not its subject. This is the
gate. Its job is not to show that the model agrees — it does, to ±2.59% — but to record *what kind*
of agreement that is, because M15 P9 bakes the number into every inferred spin and M15 P16 has to
draw it beside a measured landing point.

**M15 P5 re-flew it on complete launch conditions and every number here moved.** P4 flew the two
shots planar because the integrator had no third dimension; their recorded launch directions and
spin axes were on disk the whole time, and HD Golf's printed carry belongs to the shot that curved.
Three of the four things pinned below say something different as a result, and one of them —
P4's cancellation argument — turned out to have been partly an artefact of flying the wrong shot.

Four things are pinned here that a bare tolerance would let rot:

- the pin has **no slack**, so a regression cannot hide inside a budget that was never a budget;
- both flights are **fully clamped**, so the pass is agreement with a held end row and not a
  validation of the published table (ADR-027's 2026-09-05 addendum);
- the model still **ranks the two shots the wrong way round**, which is the fact the percentage
  hides and the reason ADR-027 §Decision 3 is a path-drawing device rather than a measurement;
- and the recorded **spin axis closed a tenth of that inversion**, which is what stops P4's
  "±2.5% is the size of the missing spin effect" from being quotable as it stands.

Run against the real committed constants on a base install, like their siblings.
"""

from __future__ import annotations

import pytest

from golf_coach.analysis.flight import (
    GATE_AGREEMENT_FRACTION,
    VALIDATION_SHOTS,
    gate_comparisons,
    simulate_flight,
)

#: What HD Golf printed, what this model flies, and the gap. All three columns, so a reader of a
#: failure can see which one moved. The middle column is **no longer** the pin `test_flight.py`
#: carries: that one is the planar flight, this one is the same shot flown with the launch
#: direction and spin axis beside it, and the 0.05 yd and 0.69 yd between the two files are the
#: third dimension. A change moving one without the other is a finding either way.
_EXPECTED = {
    "2026-08-10-1": (125.6, 122.3567, -3.2433),
    "2026-08-10-2": (121.0, 123.3868, +2.3868),
}


def test_the_gate_passes_and_these_are_the_numbers_it_passed_with() -> None:
    """M15 P4's whole claim, in one assertion per shot. **±2.59%, in both directions.**"""
    for comparison in gate_comparisons():
        simulator, model_carry, error = _EXPECTED[comparison.shot.shot_id]
        assert comparison.shot.simulator_carry_yds == simulator
        assert comparison.flight.carry_yds == pytest.approx(model_carry, abs=1e-4)
        assert comparison.error_yds == pytest.approx(error, abs=1e-4)
        assert comparison.agrees, comparison.shot.shot_id


def test_the_gate_flies_the_shot_the_simulator_measured_and_not_a_planar_one() -> None:
    """M15 P5's correction to the gate, stated as the thing that would undo it.

    Dropping the recorded spin axis is the tempting simplification — it is the one launch
    condition `analysis/shot_measure.py` still refuses to record as a measurement — and it changes
    the answer by 0.05 yd on one shot and 0.69 yd on the other. Dropping the launch direction
    changes nothing at all, and that is not luck: `carry_m` is defined along the launch azimuth
    precisely so that where a shot started cannot move how far it flew.
    """
    for comparison in gate_comparisons():
        launch = comparison.shot.launch
        assert launch.spin_axis_deg != 0.0, comparison.shot.shot_id
        assert launch.launch_direction_deg != 0.0, comparison.shot.shot_id

        without_axis = simulate_flight(launch._replace(spin_axis_deg=0.0))
        assert abs(without_axis.carry_yds - comparison.flight.carry_yds) > 0.04

        without_direction = simulate_flight(launch._replace(launch_direction_deg=0.0))
        assert without_direction.carry_m == pytest.approx(comparison.flight.carry_m, abs=1e-9)


def test_the_pinned_agreement_has_no_slack_to_hide_a_regression_in() -> None:
    """The tolerance was measured and then pinned, which only means something if it is tight.

    A budget chosen in advance can absorb a change silently, and `ROADMAP.md`'s M15 section is
    explicit that this one must not be *"a number to widen until it passes"*. So the worst achieved
    error has to sit within a whisker of the constant: 2.5822% against a pin of 2.59%.

    P5 raised the pin from 0.0255, and this test is the check that it was raised the honest way —
    to the fourth decimal above what was achieved, with nothing left over.
    """
    worst = max(abs(comparison.error_fraction) for comparison in gate_comparisons())
    assert worst <= GATE_AGREEMENT_FRACTION
    assert worst > GATE_AGREEMENT_FRACTION - 0.0002


def test_the_gate_is_passed_by_flights_that_never_read_the_published_table() -> None:
    """The caveat that has to travel with the number, asserted in the same file as the number.

    ADR-027's 2026-09-05 addendum: the coefficient table is a driver's and both these shots are an
    iron's, flying the whole way above its last row. So ±2.59% is the agreement between HD Golf and
    a *held* coefficient pair. Quoting it as a validation of the patent's measurements would be
    quoting it as evidence for data that was never consulted.
    """
    for comparison in gate_comparisons():
        assert comparison.flight.fully_clamped, comparison.shot.shot_id


def test_the_model_ranks_the_two_shots_the_wrong_way_round() -> None:
    """The finding P4 exists to surface, and the one ±2.59% conceals.

    HD Golf has `2026-08-10-1` — the lower launch, the lower spin — carrying 4.6 yd *further* than
    `2026-08-10-2`. This model has it 1.0 yd shorter. The sign of the difference between the only
    two shots that can be checked is inverted, and the mechanism is P3's: above the clamp the
    2,109 rpm that separates them reaches the flight nowhere at all.

    A per-shot tolerance cannot catch this, because each shot passes on its own.
    """
    first, second = gate_comparisons()
    simulator_gap = second.shot.simulator_carry_yds - first.shot.simulator_carry_yds
    model_gap = second.flight.carry_yds - first.flight.carry_yds
    assert simulator_gap == pytest.approx(-4.6, abs=1e-9)
    assert model_gap == pytest.approx(+1.030, abs=1e-3)
    assert simulator_gap * model_gap < 0.0, "the gate passed while the ordering inverted"


def test_the_spin_axis_closes_a_tenth_of_the_inversion_and_the_spin_rate_closes_none() -> None:
    """M15 P5's finding, and the reason P4's headline sentence cannot be repeated as written.

    P4 read ±2.5% as *"the size of the missing spin effect"*, because at the time the only thing
    separating the two shots that the model could not see was their 2,109 rpm. It was seeing an
    incomplete shot. The recorded spin axes differ by 6.8°, the shot with the larger one loses more
    carry to lateral lift, and flying them closes 0.63 yd of the 6.26 yd inversion — a tenth of it,
    from a launch condition that is not the spin rate at all.

    Nine tenths remain and the ordering is still inverted, so P4's conclusion survives. Its
    *attribution* does not, and M15 P9 must report the residual rather than the whole.
    """
    first, second = gate_comparisons()
    planar = [
        simulate_flight(shot.launch._replace(launch_direction_deg=0.0, spin_axis_deg=0.0))
        for shot in VALIDATION_SHOTS
    ]
    planar_gap = planar[1].carry_yds - planar[0].carry_yds
    flown_gap = second.flight.carry_yds - first.flight.carry_yds

    assert planar_gap == pytest.approx(+1.664, abs=1e-3)
    assert flown_gap == pytest.approx(+1.030, abs=1e-3)
    # Both gaps sit the wrong side of HD Golf's −4.6, so "closed" means closer, not fixed.
    assert flown_gap < planar_gap
    assert (planar_gap - flown_gap) / (planar_gap + 4.6) == pytest.approx(0.10, abs=0.01)


def test_the_two_errors_no_longer_cancel_and_that_was_the_point_of_flying_them_properly() -> None:
    """P4 pinned a mean error of −0.06 yd as a *warning*. P5 turned the warning into a measurement.

    The two errors used to be −3.19 and +3.07, averaging to −0.06 yd — which P4 called the most
    flattering way to state this model's accuracy and the least honest, since two near-equal errors
    of opposite sign at `n = 2` are the signature of a model that cannot separate two shots rather
    than of one that is unbiased. Flying the recorded spin axes gives −3.24 and +2.39, a mean of
    −0.43 yd, and the cancellation is gone: it was partly an artefact of flying both shots planar.

    The pin stays, inverted. A bias correction fitted to two shots would still be fitting noise
    (ADR-022 §2's argument for interpretable estimators at small `n`, at the smallest `n` there
    is) — but nobody can now reach for the mean as evidence that there is no bias to correct.
    """
    errors = [comparison.error_yds for comparison in gate_comparisons()]
    mean_error = sum(errors) / len(errors)
    assert mean_error == pytest.approx(-0.4282, abs=1e-3)
    assert min(errors) < 0.0 < max(errors)
    assert abs(mean_error) > 0.15 * min(abs(error) for error in errors)


def test_the_disagreement_is_far_larger_than_the_precision_the_tiles_print_at() -> None:
    """HD Golf prints one decimal place, so the inputs carry ±0.05 of rounding. That is not this.

    The obvious first explanation for a 3 yd disagreement is that the launch conditions were read
    to a coarser precision than the carry, and it is wrong by an order of magnitude: sweeping both
    inputs across their full rounding interval moves the carry ~0.3 yd against errors of 3.2 and
    2.4 yd. Checked here so nobody re-checks it, and so the ±2.59% cannot be explained away as
    rounding.

    The sweep varies ball speed and launch angle only. The other two tiles are swept nowhere
    because they cannot rescue the disagreement either: the spin rate has no route into a clamped
    flight at all, and the launch direction cannot move a carry measured along the launch azimuth.
    """
    for comparison in gate_comparisons():
        launch = comparison.shot.launch
        low = simulate_flight(
            launch._replace(
                ball_speed_mph=launch.ball_speed_mph - 0.05,
                launch_angle_deg=launch.launch_angle_deg - 0.05,
            )
        ).carry_yds
        high = simulate_flight(
            launch._replace(
                ball_speed_mph=launch.ball_speed_mph + 0.05,
                launch_angle_deg=launch.launch_angle_deg + 0.05,
            )
        ).carry_yds
        assert high - low < 0.35
        assert high - low < abs(comparison.error_yds) / 8


def test_one_target_is_reachable_at_a_fabricated_spin_and_the_other_at_no_spin_at_all() -> None:
    """What the gate means for M15 P8, measured rather than inferred from the pass.

    Sweeping spin at each shot's own launch conditions gives the whole set of carries this model
    can produce there. `2026-08-10-1`'s printed 125.6 yd is inside its set — reachable, but only at
    **3,185 rpm against a recorded 5,991**, which is ADR-027 §Decision 3's honest test and its
    failure. `2026-08-10-2`'s printed 121.0 yd is below the lowest carry the model produces at any
    spin, so the solve refuses it. One fabricated answer and one refusal, on the two shots where
    the true answer is known, is the whole of P8's evidence base.

    P5 moved that fabricated answer by 16 rpm, from P4's 3,201. The third dimension changes the
    verdict on neither shot, which is worth knowing before P8 hopes it might.

    The sweep is coarse on purpose: it is bounding a set, and a coarser grid can only understate
    the peak, which makes the reachability claims conservative rather than optimistic.
    """
    reachable = {}
    for shot in VALIDATION_SHOTS:
        carries = [
            simulate_flight(shot.launch._replace(spin_rpm=spin)).carry_yds
            for spin in range(0, 12001, 750)
        ]
        reachable[shot.shot_id] = min(carries) <= shot.simulator_carry_yds <= max(carries)
    assert reachable == {"2026-08-10-1": True, "2026-08-10-2": False}


def test_the_gate_reruns_rather_than_remembering() -> None:
    """`gate_comparisons` flies the shots, so a re-sourced coefficient table moves these numbers.

    The alternative — storing the results beside the constants — was declined because
    `flight_model_v1.json` says of itself that every number in it is a published measurement and
    that nothing in it was derived from this repo's corpus. A model output over two shots on disk
    is neither, and putting it there would have made the file's central claim false to keep one
    number tidy. Varying the step is what shows the gate is computed: the disagreement is the
    model's, not the integrator's, so it survives a four-times-coarser step unchanged.
    """
    for coarse, default in zip(gate_comparisons(step_s=0.02), gate_comparisons(), strict=True):
        assert coarse.error_yds == pytest.approx(default.error_yds, abs=1e-4)


def test_the_gate_is_indifferent_to_which_hand_the_golfer_swings_with() -> None:
    """The one place `LaunchConditions.spin_axis_deg`'s unresolved sign could have reached a number.

    It cannot. Carry is even in the spin axis, so flipping every axis in `VALIDATION_SHOTS` — which
    is exactly what reading these shots as a left-handed golfer's would do — leaves the gate where
    it was, to a nanometre over 112 m. (Not to the last bit: the launch direction rotates the
    flight through a sine and cosine pair on the way in and back out again, and one ULP survives
    that round trip.) What handedness decides is the side the ball finishes on, and M15 P9 owns
    that (ADR-027 §Decision 5).

    Pinned because the alternative is a reader concluding that the gate rests on a sign this repo
    has already had wrong once (`screen/profiles.json`'s `printed_sign`, and ADR-014's addendum).
    """
    mirrored = [
        simulate_flight(shot.launch._replace(spin_axis_deg=-shot.launch.spin_axis_deg))
        for shot in VALIDATION_SHOTS
    ]
    for flown, comparison in zip(mirrored, gate_comparisons(), strict=True):
        assert flown.carry_m == pytest.approx(comparison.flight.carry_m, abs=1e-9)
        assert flown.landing_offline_m != comparison.flight.landing_offline_m
