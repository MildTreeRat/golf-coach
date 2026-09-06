"""The ball-flight integrator: the physics it has to obey, and the findings it turned up.

Like `test_flight_model.py` these run against the **real** committed constants rather than a
fixture, on a base install with no extras. A flight is deterministic arithmetic over published
data, so the numbers below are pins on the model as committed — if one moves, either the artifact
was re-sourced or the integrator changed, and both are things a reader should be told about.

The gate itself — agreement with HD Golf's own carry — is **M15 P4** and is not asserted here. What
is asserted here is that the physics is the physics: energy leaves the ball, lift acts where lift
should act, the step size is not where the error lives, and the landing is solved rather than
rounded to.

The **third dimension** joins them at the foot of the file (M15 P5), and its own claim is a
negative one first: the lateral machinery contributes exactly zero to a flight that does not use
it, which is what lets every planar number above stand through the phase that added it.
"""

from __future__ import annotations

import math

import pytest

from golf_coach.analysis.benchmarks import load_flight_model
from golf_coach.analysis.flight import (
    DEFAULT_STEP_S,
    VALIDATION_SHOTS,
    LaunchConditions,
    gate_comparisons,
    simulate_flight,
)

#: The two 2026-08-10 reference shots (ADR-027 §Context 2), and beside each the carry the
#: integrator returns over the committed constants. **These are not HD Golf's carries** — HD Golf
#: printed 125.6 and 121.0 yd, and the ~2.5% between the two columns is P4's subject, not this
#: file's. Pinned so a re-sourced coefficient table cannot move a flight silently.
_REFERENCE = {
    "2026-08-10-1": (LaunchConditions(90.7, 20.9, 5991.0), 122.4110),
    "2026-08-10-2": (LaunchConditions(90.5, 23.5, 8100.0), 124.0747),
}


def test_the_reference_shots_fly_the_carries_this_model_is_pinned_at() -> None:
    for shot, (launch, expected_yds) in _REFERENCE.items():
        result = simulate_flight(launch)
        assert result.carry_yds == pytest.approx(expected_yds, abs=1e-4), shot


def test_a_flight_starts_at_the_tee_and_ends_on_the_ground() -> None:
    result = simulate_flight(_REFERENCE["2026-08-10-1"][0])
    first = result.points[0]
    assert (first.t_s, first.x_m, first.y_m, first.z_m) == (0.0, 0.0, 0.0, 0.0)
    # Solved, not rounded to the step boundary: the landing sits on the ground to well inside a
    # millimetre, which a whole-step termination could not manage (it would be ~9.5 cm long).
    assert abs(result.landing.y_m) < 1e-6
    assert result.landing.vy_m_s < 0.0, "a ball crossing y=0 at the end of a flight is descending"
    assert result.carry_m == result.landing.x_m
    assert result.flight_time_s == result.landing.t_s


def test_the_solved_landing_falls_inside_the_step_that_crossed_the_ground() -> None:
    """The whole point of `_solve_landing`, stated as the property rather than as a number.

    The final interval is a fraction of a step, and the one before it is a whole step. If the
    termination ever regresses to taking the last whole step, this is what says so.
    """
    result = simulate_flight(_REFERENCE["2026-08-10-1"][0])
    final_gap = result.points[-1].t_s - result.points[-2].t_s
    assert 0.0 < final_gap < DEFAULT_STEP_S
    earlier_gap = result.points[-2].t_s - result.points[-3].t_s
    assert earlier_gap == pytest.approx(DEFAULT_STEP_S)


def test_the_carry_is_the_same_at_every_step_size_worth_running() -> None:
    """RK4 is fourth order and these flights are smooth, so the step is not the error term.

    This is the measurement `DEFAULT_STEP_S` was chosen from: four times finer than the coarsest
    step that already converges. It matters because the ~2.5% disagreement with HD Golf must not
    be attributable to the integrator — see the module docstring of `analysis/flight.py`.
    """
    launch = _REFERENCE["2026-08-10-1"][0]
    carries = [simulate_flight(launch, step_s=step).carry_yds for step in (0.02, 0.005, 0.0005)]
    for carry in carries:
        assert carry == pytest.approx(carries[0], abs=1e-4)


def test_the_ball_loses_speed_all_the_way_down() -> None:
    """Drag always opposes the velocity and lift is perpendicular to it, so neither can add speed.

    Gravity can, and on the way down it does — which is why this is asserted over the *ascent*,
    where gravity, drag and the vertical component of the flight all take energy out. A sign error
    in the drag term shows up here and almost nowhere else.
    """
    result = simulate_flight(_REFERENCE["2026-08-10-1"][0])
    apex_index = max(range(len(result.points)), key=lambda i: result.points[i].y_m)
    ascending = result.points[: apex_index + 1]
    speeds = [point.speed_m_s for point in ascending]
    assert speeds == sorted(speeds, reverse=True)
    assert result.landing.speed_m_s < result.points[0].speed_m_s


def test_backspin_holds_the_ball_up_and_no_spin_does_not() -> None:
    """Lift points away from the spin, so removing it must lower the apex and shorten the flight.

    Both flights are otherwise identical, so anything that separates them is the Magnus term. If
    the perpendicular were rotated the wrong way this test fails rather than the carry quietly
    coming out 30 yards short.
    """
    spun = simulate_flight(LaunchConditions(90.7, 20.9, 5991.0))
    unspun = simulate_flight(LaunchConditions(90.7, 20.9, 0.0))
    assert unspun.apex_m < spun.apex_m
    assert unspun.flight_time_s < spun.flight_time_s


def test_the_spin_ratio_climbs_through_a_flight_and_the_spin_itself_falls() -> None:
    """`S = wR/v` rises because the ball sheds speed faster than 4%/s sheds spin.

    That is the mechanism behind M15 P2's central finding — a shot launching above the published
    table only goes further above it — so it is pinned here rather than left to the docstrings.
    """
    result = simulate_flight(_REFERENCE["2026-08-10-2"][0])
    spins = [point.spin_rpm for point in result.points]
    assert spins == sorted(spins, reverse=True)
    assert result.spin_ratio_max > result.spin_ratio_min
    assert result.points[0].spin_ratio == pytest.approx(result.spin_ratio_min)


def test_both_reference_shots_fly_every_step_on_the_clamp() -> None:
    """ADR-027's 2026-09-05 addendum, asserted against the integrator rather than the lookup.

    `test_flight_model.py` pins that both shots *launch* above the table. This pins the
    consequence: they never come back down into it, so `fully_clamped` is true and not one
    coefficient in either flight was read from the measured rows. M15 P4 is required to report
    this beside whatever agreement it finds.
    """
    ceiling = load_flight_model().coefficients.spin_ratio_max
    for shot, (launch, _) in _REFERENCE.items():
        result = simulate_flight(launch)
        assert result.fully_clamped, shot
        assert result.clamped_points == len(result.points)
        assert result.spin_ratio_min > ceiling, shot


def test_above_the_clamp_spin_stops_reaching_the_flight_at_all() -> None:
    """The finding M15 P3 turned up, and the one that reshapes P8. **Read this before writing it.**

    ADR-027's first addendum said a clamped table leaves spin only "the Magnus term's own w". It
    does not. Lift is written with a coefficient — `0.5*rho*A*Cl*v^2` — so `w` reaches the flight
    only through `S`, and `S` only through `Cl` and `Cd`. Hold those and spin has **no** remaining
    route into the answer: two flights differing by 2109 rpm are identical to the last bit.

    §Decision 4's spin solve therefore has a fourth case the ADR did not have — a plateau on which
    every spin reproduces the same carry, which is not two answers but infinitely many.
    """
    slow = simulate_flight(LaunchConditions(90.6, 22.0, 5991.0))
    fast = simulate_flight(LaunchConditions(90.6, 22.0, 8100.0))
    assert slow.fully_clamped and fast.fully_clamped
    assert slow.carry_m == fast.carry_m
    assert slow.apex_m == fast.apex_m
    assert slow.flight_time_s == fast.flight_time_s


def test_below_the_clamp_spin_still_changes_the_carry() -> None:
    """The other half of the finding: the plateau is the clamp's, not the physics'.

    A launch spin low enough to start inside the measured rows flies a different carry from one
    just above it, which is what makes the plateau above a data limit rather than a real feature
    of ball flight — and what leaves M15 P8 a solvable range to work in.
    """
    inside = simulate_flight(LaunchConditions(90.6, 22.0, 3000.0))
    plateau = simulate_flight(LaunchConditions(90.6, 22.0, 8100.0))
    assert not inside.fully_clamped
    assert inside.carry_m != plateau.carry_m


def test_a_steep_enough_launch_lands_behind_the_tee() -> None:
    """A negative carry is a real trajectory, so nothing clamps it to zero on the way out.

    Near-vertical with backspin the lift perpendicular points *backwards*, and the ball comes down
    behind where it started. Nothing on disk looks like this; it is here because a carry that can
    be negative is the case a later reader is most likely to `abs()` away.

    It is also where the descent angle was checked for the obvious follow-on and did **not** find
    it: sweeping 80 deg to 89.9 deg, the ball is always travelling forwards again by the time it
    lands, so the angle stays under 90 deg. Drag kills the backward velocity, and once the ball is
    descending the lift turns forward with it.
    """
    result = simulate_flight(LaunchConditions(90.0, 85.0, 6000.0))
    assert result.carry_m < 0.0
    assert result.landing.vx_m_s > 0.0
    assert 0.0 < result.descent_angle_deg < 90.0


def test_a_shallow_launch_still_produces_a_flight_and_not_a_zero() -> None:
    """3.4 deg is the shallowest launch angle on disk, and it lands inside a handful of steps.

    The seed for the landing solve degenerates when `y` starts at exactly zero, so the shallow end
    is where a naive linear interpolation returns a carry of zero rather than a short flight.
    """
    result = simulate_flight(LaunchConditions(83.8, 3.4, 6000.0))
    assert result.carry_m > 1.0
    assert abs(result.landing.y_m) < 1e-6
    assert result.flight_time_s > 0.0


def test_launch_conditions_that_cannot_fly_are_refused_where_they_arrive() -> None:
    """R8's raising side: the callers reading real shots are the boundary, not this function."""
    with pytest.raises(ValueError, match="ball speed"):
        simulate_flight(LaunchConditions(0.0, 20.0, 6000.0))
    with pytest.raises(ValueError, match="launch angle"):
        simulate_flight(LaunchConditions(90.0, 0.0, 6000.0))
    with pytest.raises(ValueError, match="launch angle"):
        simulate_flight(LaunchConditions(90.0, 90.0, 6000.0))
    with pytest.raises(ValueError, match="spin rate"):
        simulate_flight(LaunchConditions(90.0, 20.0, -100.0))
    with pytest.raises(ValueError, match="step"):
        simulate_flight(LaunchConditions(90.0, 20.0, 6000.0), step_s=0.0)


def test_the_reported_yardages_are_the_metres_converted_once() -> None:
    result = simulate_flight(_REFERENCE["2026-08-10-1"][0])
    assert result.carry_yds == pytest.approx(result.carry_m / 0.9144)
    assert result.apex_yds == pytest.approx(result.apex_m / 0.9144)
    assert result.landing.x_yds == pytest.approx(result.landing.x_m / 0.9144)


def test_the_apex_is_the_highest_point_on_the_path_it_returns() -> None:
    result = simulate_flight(_REFERENCE["2026-08-10-2"][0])
    assert result.apex_m == max(point.y_m for point in result.points)
    assert result.apex_m > 0.0


def test_a_caller_may_pass_the_model_rather_than_have_it_loaded() -> None:
    """The seam M15 P6 varies the atmosphere through, exercised before it has a second value."""
    model = load_flight_model()
    launch = _REFERENCE["2026-08-10-1"][0]
    assert simulate_flight(launch, model=model).carry_m == simulate_flight(launch).carry_m


def test_the_descent_angle_matches_the_landing_velocity_it_was_read_from() -> None:
    result = simulate_flight(_REFERENCE["2026-08-10-1"][0])
    landing = result.landing
    expected = math.degrees(math.atan2(-landing.vy_m_s, landing.vx_m_s))
    assert result.descent_angle_deg == pytest.approx(expected)
    assert 0.0 < result.descent_angle_deg < 90.0


# -------------------------------------------------------------------------------------------
# The third dimension [M15 P5] — the launch direction, the spin axis, and what each one moves
# -------------------------------------------------------------------------------------------

#: The launch directions actually on disk: `2026-08-10-1` started 5.3° left, `2026-08-10-2` 4.0°
#: right, and the widest of the thirteen is 6.5°. Swept rather than picked, because the claim
#: below is that *none* of them touches the carry.
_RECORDED_DIRECTIONS = (-6.7, -5.3, -1.8, 0.0, 2.8, 4.0, 6.5)


def test_the_third_dimension_is_off_unless_a_caller_asks_for_it() -> None:
    """M15 P3's flights are M15 P5's flights, and the default is what guarantees it.

    Not a tautology about default arguments: it says the lateral machinery contributes exactly
    zero — not a rounding error — when it is not used, which is what lets `test_flight.py`'s and
    `test_flight_validation.py`'s planar pins stand unchanged through this phase.
    """
    launch = _REFERENCE["2026-08-10-1"][0]
    assert (launch.launch_direction_deg, launch.spin_axis_deg) == (0.0, 0.0)

    result = simulate_flight(launch)
    assert all(point.z_m == 0.0 and point.vz_m_s == 0.0 for point in result.points)
    assert result.curvature_m == 0.0
    assert result.landing_offline_m == 0.0


def test_a_launch_direction_turns_the_whole_flight_without_changing_it() -> None:
    """Carry cannot depend on where the shot started, and this is the assertion that says so.

    Turning a flight about the vertical axis cannot change how far the ball flew, so defining
    `carry_m` along the launch azimuth makes it *identical* — not approximately equal — across
    every launch direction on disk. Defining it along the target line instead would have shrunk
    it by `cos(direction)`: 0.43% on `2026-08-10-1`'s recorded −5.3°, which is a sixth of
    `GATE_AGREEMENT_FRACTION` and would have gone into M15 P8's spin solve as a bias shaped like
    the start line.

    What the direction does move is where the ball finished, and for an uncurved flight that is
    the whole of it: `offline = carry * sin(direction)`, exactly.
    """
    planar = simulate_flight(_REFERENCE["2026-08-10-1"][0])
    for direction in _RECORDED_DIRECTIONS:
        turned = simulate_flight(
            _REFERENCE["2026-08-10-1"][0]._replace(launch_direction_deg=direction)
        )
        # A nanometre, not bit-equality: the flight is rotated in floating point twice over, once
        # into the launch velocity through a sine and cosine pair and once back out of the landing
        # point. The residual is ~1e-13 m over 110 m, which is the round trip and nothing else.
        assert turned.carry_m == pytest.approx(planar.carry_m, abs=1e-9), direction
        assert turned.apex_m == pytest.approx(planar.apex_m, abs=1e-9), direction
        assert turned.flight_time_s == pytest.approx(planar.flight_time_s, abs=1e-9), direction
        assert turned.descent_angle_deg == pytest.approx(
            planar.descent_angle_deg, abs=1e-9
        ), direction
        assert abs(turned.curvature_m) < 1e-9, direction
        assert turned.landing_offline_m == pytest.approx(
            turned.carry_m * math.sin(math.radians(direction))
        ), direction


def test_the_landing_point_is_the_carry_and_the_curve_in_the_launch_line_frame() -> None:
    """The three lateral numbers are one landing point in two frames, and this is the identity.

    `carry` and `curvature` are the landing point's coordinates along and across the *launch*
    line; `landing_offline` is its distance across the *target* line. Rotating one pair back gives
    the other, and a viewer that draws a push differently from a slice (M15 P16) is reading
    exactly this decomposition.
    """
    launch = LaunchConditions(90.5, 23.5, 8100.0, 4.0, 9.3)
    result = simulate_flight(launch)
    cos_d = math.cos(math.radians(launch.launch_direction_deg))
    sin_d = math.sin(math.radians(launch.launch_direction_deg))

    landing = result.landing
    assert result.landing_offline_m == landing.z_m
    assert result.landing_offline_m == pytest.approx(
        result.carry_m * sin_d + result.curvature_m * cos_d
    )
    assert landing.x_m == pytest.approx(result.carry_m * cos_d - result.curvature_m * sin_d)


def test_a_positive_spin_axis_curves_the_ball_right_and_a_negative_one_mirrors_it() -> None:
    """The sign this module owns, asserted as the direction it bends the ball.

    Positive is a right-hand rotation of the spin axis about the direction of flight, so it tips
    the Magnus force right and the ball goes right. **That is not `ShotData.spin_axis`'s `+ =
    fade`** — the two agree only for a right-handed golfer, and `LaunchConditions` says where the
    flip belongs.

    The mirror is the sharper half: carry is *even* in the axis and curvature is *odd*, so a
    left-handed golfer's fade flies exactly as far as a right-handed golfer's. Which means the
    handedness question above cannot reach the carry at all — only the side it finishes.
    """
    launch = _REFERENCE["2026-08-10-1"][0]
    right = simulate_flight(launch._replace(spin_axis_deg=9.3))
    left = simulate_flight(launch._replace(spin_axis_deg=-9.3))

    assert right.curvature_m > 0.0 and right.landing_offline_m > 0.0
    assert left.curvature_m == pytest.approx(-right.curvature_m, abs=1e-12)
    assert left.carry_m == pytest.approx(right.carry_m, abs=1e-12)
    assert left.apex_m == pytest.approx(right.apex_m, abs=1e-12)


def test_tilting_the_axis_takes_lift_out_of_the_vertical_plane_and_carry_with_it() -> None:
    """Lift is one vector, so what it spends on curving the ball it does not spend holding it up.

    Over the range a golfer produces, more axis means less apex, less carry and more curve, all
    monotonically. This is the mechanism behind M15 P5's finding: on the two validation shots the
    recorded axes are 2.5° and 9.3°, and the shot with the *bigger* axis loses more carry — which
    moves the pair back towards HD Golf's ordering from a source that is not the spin rate at all.
    """
    launch = _REFERENCE["2026-08-10-1"][0]
    flights = [
        simulate_flight(launch._replace(spin_axis_deg=axis))
        for axis in (0.0, 2.5, 9.3, 15.0, 45.0)
    ]

    apexes = [flight.apex_m for flight in flights]
    carries = [flight.carry_m for flight in flights]
    curves = [flight.curvature_m for flight in flights]
    assert apexes == sorted(apexes, reverse=True)
    assert carries == sorted(carries, reverse=True)
    assert curves == sorted(curves)


def test_a_vertical_axis_curves_less_than_a_tilted_one_because_it_falls_out_of_the_sky() -> None:
    """The place the monotonicity above stops, pinned so nobody extends the claim past it.

    At an axis of 90° the ball spins purely sideways: no lift at all at launch, an apex *below*
    the unspun flight's, and 1.9 s less hang time than the backspun one. It curves hard while it
    is up there and then runs out of flight, so it finishes **less** offline than a 45° axis does.
    Curve is not monotone in the axis over the whole quarter turn, only over the part a golf swing
    can reach.
    """
    launch = _REFERENCE["2026-08-10-1"][0]
    tilted = simulate_flight(launch._replace(spin_axis_deg=45.0))
    vertical = simulate_flight(launch._replace(spin_axis_deg=90.0))
    unspun = simulate_flight(launch._replace(spin_rpm=0.0))

    assert vertical.curvature_m < tilted.curvature_m
    assert vertical.apex_m < unspun.apex_m
    assert vertical.flight_time_s < tilted.flight_time_s - 1.0


def test_the_lateral_launch_conditions_are_refused_where_they_stop_being_a_shot() -> None:
    """R8's raising side again, and the axis bound is the coefficient table's rather than golf's.

    A quarter turn of axis is a ball spinning purely sideways, which this model flies. Past it the
    axis has tipped into top spin, refused for the same reason a negative spin rate is: the table
    is one-sided, and clamping a flipped axis onto it would fly a top-spun ball with lift.
    """
    assert simulate_flight(LaunchConditions(90.0, 20.0, 6000.0, 0.0, 90.0)).carry_m > 0.0
    assert simulate_flight(LaunchConditions(90.0, 20.0, 6000.0, 0.0, -90.0)).carry_m > 0.0
    with pytest.raises(ValueError, match="launch direction"):
        simulate_flight(LaunchConditions(90.0, 20.0, 6000.0, 90.0, 0.0))
    with pytest.raises(ValueError, match="launch direction"):
        simulate_flight(LaunchConditions(90.0, 20.0, 6000.0, -90.0, 0.0))
    with pytest.raises(ValueError, match="spin axis"):
        simulate_flight(LaunchConditions(90.0, 20.0, 6000.0, 0.0, 90.1))
    with pytest.raises(ValueError, match="spin axis"):
        simulate_flight(LaunchConditions(90.0, 20.0, 6000.0, 0.0, -90.1))


def test_the_reported_lateral_yardages_are_the_metres_converted_once() -> None:
    result = simulate_flight(LaunchConditions(90.5, 23.5, 8100.0, 4.0, 9.3))
    assert result.curvature_yds == pytest.approx(result.curvature_m / 0.9144)
    assert result.landing_offline_yds == pytest.approx(result.landing_offline_m / 0.9144)
    assert result.landing.z_yds == pytest.approx(result.landing.z_m / 0.9144)


# -------------------------------------------------------------------------------------------
# The altitude what-if [M15 P6] — thinner air, and what it does to the gate's own defect
# -------------------------------------------------------------------------------------------

#: Denver to the nearest metre, and Tactu in Peru — the highest course on earth — as the far end
#: of the profile's range. Neither is a plan; they are the two altitudes worth having a number for.
_DENVER_M = 1609.0
_TACTU_M = 4369.0
#: A course on the Dead Sea shore, and the only reason the profile's lower bound is not zero.
_DEAD_SEA_M = -390.0


def test_flying_through_the_profile_at_sea_level_changes_nothing_at_all() -> None:
    """The negative claim first, as M15 P5 did: the what-if must not perturb the default.

    Point for point, not just carry to carry. `AtmosphereProfile` is written as ratios precisely
    so that this holds by construction — the committed sea-level row is exact in density and
    rounded in viscosity, and an absolute profile would have moved one of them.
    """
    sea_level = load_flight_model().at_altitude(0.0)
    for shot in VALIDATION_SHOTS:
        default = simulate_flight(shot.launch)
        through_the_profile = simulate_flight(shot.launch, model=sea_level)
        assert through_the_profile.points == default.points, shot.shot_id
        assert through_the_profile == default, shot.shot_id


def test_thin_air_flies_the_ball_further_and_thick_air_shorter() -> None:
    """Monotone in altitude, with Denver pinned — the number a golfer would ask for.

    About 3% for a 7 iron at a mile up, not the 10% the driver rule of thumb is quoted at: a
    lofted shot spends much of its carry on lift, and altitude takes lift away with the same
    density it takes drag away with.
    """
    model = load_flight_model()
    shot = VALIDATION_SHOTS[0]

    carries = [
        simulate_flight(shot.launch, model=model.at_altitude(metres)).carry_yds
        for metres in (_DEAD_SEA_M, 0.0, 500.0, _DENVER_M, _TACTU_M)
    ]
    assert carries == sorted(carries)

    sea_level, denver = carries[1], carries[3]
    assert sea_level == pytest.approx(122.3567, abs=1e-4)
    assert denver == pytest.approx(125.9882, abs=1e-4)
    assert denver / sea_level - 1.0 == pytest.approx(0.0297, abs=1e-4)
    assert carries[0] / sea_level - 1.0 == pytest.approx(-0.0083, abs=1e-4)


def test_thin_air_flattens_the_flight_it_lengthens() -> None:
    """Further is not the whole sentence, and the rest of it is the coaching-relevant half.

    Density scales lift and drag together, so a ball at altitude is held up less as well as slowed
    less. It carries further while arriving lower, sooner, shallower and straighter — which is why
    a shot that holds a green at sea level runs through it at Denver, and why the curve a golfer
    plays for gets smaller with the air.
    """
    model = load_flight_model()
    shot = VALIDATION_SHOTS[1]

    sea_level = simulate_flight(shot.launch)
    denver = simulate_flight(shot.launch, model=model.at_altitude(_DENVER_M))

    assert denver.carry_yds > sea_level.carry_yds
    assert denver.apex_yds < sea_level.apex_yds
    assert denver.flight_time_s < sea_level.flight_time_s
    assert denver.descent_angle_deg < sea_level.descent_angle_deg
    assert 0.0 < denver.curvature_yds < sea_level.curvature_yds


def test_altitude_never_brings_a_shot_back_inside_the_measured_table() -> None:
    """A structural fact rather than a lucky one, and it bounds what this phase can be blamed for.

    The spin ratio at launch is `wR/v` — launch conditions only, with no air in it — so no
    atmosphere can move it, and `S` only climbs from there as the ball sheds speed faster than
    spin. Both reference shots launch above the published ceiling, so both stay fully clamped at
    every altitude the profile will produce. Altitude therefore acts on these flights as a pure
    scaling of two held coefficients, and cannot be accused of extrapolating any further than the
    sea-level flight already did.
    """
    model = load_flight_model()
    for shot in VALIDATION_SHOTS:
        launch_ratio = simulate_flight(shot.launch).spin_ratio_min
        for metres in (_DEAD_SEA_M, 0.0, _DENVER_M, _TACTU_M):
            result = simulate_flight(shot.launch, model=model.at_altitude(metres))
            assert result.fully_clamped, f"{shot.shot_id} at {metres} m"
            assert result.spin_ratio_min == pytest.approx(launch_ratio, abs=1e-12)
            assert result.spin_ratio_min > model.coefficients.spin_ratio_max
        # Thinner air holds the ball's speed up, so the ratio climbs *less* by landing — the one
        # place altitude touches the aerodynamics of these shots at all, and it changes nothing
        # because the whole range stays above the ceiling.
        assert (
            simulate_flight(shot.launch, model=model.at_altitude(_TACTU_M)).spin_ratio_max
            < simulate_flight(shot.launch).spin_ratio_max
        )


def test_altitude_widens_the_inverted_ordering_rather_than_closing_it() -> None:
    """M15 P6's finding, and it is a diagnosis of P4's rather than a new fault.

    HD Golf has `2026-08-10-1` — the lower launch, the lower spin — carrying 4.6 yd *further* than
    `2026-08-10-2`. This model has it 1.03 yd shorter at sea level, 1.97 yd shorter at Denver and
    3.78 yd shorter at Tactu. **The gap grows with the altitude**, monotonically and by more than
    the carry does.

    That is worth a test because of what it rules out. Above the clamp the only lever this model
    has between these two shots is 2.6° of launch angle, and thin air amplifies exactly that lever
    — so the inversion behaves like a missing *aerodynamic* term rather than like a mistyped
    launch condition, which would have scaled with the carry instead. It also kills any hope that
    an atmosphere is the free parameter that would fix the gate: every direction of air makes the
    ordering worse or leaves it alone.
    """
    model = load_flight_model()
    first, second = VALIDATION_SHOTS
    assert first.simulator_carry_yds - second.simulator_carry_yds == pytest.approx(4.6)

    gaps = [
        simulate_flight(first.launch, model=model.at_altitude(metres)).carry_yds
        - simulate_flight(second.launch, model=model.at_altitude(metres)).carry_yds
        for metres in (_DEAD_SEA_M, 0.0, _DENVER_M, _TACTU_M)
    ]
    assert all(gap < 0.0 for gap in gaps), "the ordering is inverted at every altitude"
    assert gaps == sorted(gaps, reverse=True), "and it gets worse the higher the ball is hit"
    assert gaps[1] == pytest.approx(-1.0301, abs=1e-4)
    assert gaps[2] == pytest.approx(-1.9651, abs=1e-4)
    assert gaps[3] == pytest.approx(-3.7847, abs=1e-4)


def test_the_gate_is_a_sea_level_gate_and_flying_it_higher_would_be_numerology() -> None:
    """An argument against a mistake, in the shape M15 P4 wrote one for the bias correction.

    `gate_comparisons` accepts a model, so it will happily fly the validation shots at Denver —
    where the first of them agrees with HD Golf to 0.3% instead of 2.6%. That agreement is
    meaningless: HD Golf printed a carry for a ball hit indoors near sea level, so the only air
    the gate can be run in is the air the shot was hit in. Tuning an altitude until the gate
    improved would be fitting the atmosphere to the residual of a clamped coefficient table.

    The ordering test above is the reason this is not merely pedantic — the same altitude that
    flatters shot one makes the model's *ranking* of the two shots visibly worse.
    """
    model = load_flight_model()
    at_sea_level, at_denver = (
        gate_comparisons(model=model.at_altitude(metres)) for metres in (0.0, _DENVER_M)
    )

    assert all(comparison.agrees for comparison in at_sea_level)
    assert abs(at_denver[0].error_fraction) < abs(at_sea_level[0].error_fraction)
    assert abs(at_denver[0].error_fraction) < 0.005
    # And the second shot moves the other way, which is what a spurious agreement looks like.
    assert abs(at_denver[1].error_fraction) > abs(at_sea_level[1].error_fraction)
    assert not at_denver[1].agrees


def test_an_altitude_the_standard_does_not_cover_is_refused_before_a_ball_flies() -> None:
    """The refusal lands on the model, not on the flight — which is where the altitude arrived."""
    model = load_flight_model()
    with pytest.raises(ValueError, match="outside isa_troposphere"):
        simulate_flight(VALIDATION_SHOTS[0].launch, model=model.at_altitude(12_000.0))


# -------------------------------------------------------------------------------------------
# Sampling the path — M15 P14
# -------------------------------------------------------------------------------------------


def test_a_sampled_path_always_ends_on_the_solved_landing() -> None:
    """The landing is solved inside the crossing step, so it is the one point that must survive.

    A sampler that took every nth point would end the flight a few centimetres in the air on most
    counts — 9.5 cm on this shot, which `test_the_landing_is_solved_...` measures — and a page
    drawing a landing point would be drawing the wrong one.
    """
    result = simulate_flight(VALIDATION_SHOTS[0].launch)

    for count in (2, 3, 5, 17, 120):
        sampled = result.sample(count)
        assert sampled[0] is result.points[0]
        assert sampled[-1] is result.points[-1]
        assert len(sampled) <= count


def test_sampling_more_points_than_the_flight_has_returns_the_flight() -> None:
    """The cap is the path itself; asking for a thousand points of a nine-hundred-step flight
    does not interpolate any."""
    result = simulate_flight(VALIDATION_SHOTS[0].launch)

    assert result.sample(len(result.points) * 10) == result.points


def test_a_sample_of_one_is_the_landing_rather_than_the_launch() -> None:
    """Which of the two to keep is a real choice, and the landing is the one carrying the answer."""
    result = simulate_flight(VALIDATION_SHOTS[0].launch)

    assert result.sample(1) == (result.landing,)


def test_the_samples_are_in_flight_order_and_never_repeat_a_point() -> None:
    """Two indices can round together, and the rule drops the duplicate rather than respacing.

    That is why `sample` promises *at most* `count`: spacing the points unevenly to hit a number
    nobody asked for would put a kink in a drawn line to satisfy an arithmetic tidiness.
    """
    result = simulate_flight(VALIDATION_SHOTS[0].launch)

    sampled = result.sample(400)
    times = [point.t_s for point in sampled]

    assert times == sorted(times)
    assert len(set(times)) == len(times)
