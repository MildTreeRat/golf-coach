"""The ball-flight integrator — RK4 in three dimensions. [M15 P3, third dimension M15 P5]

Given what the launch monitor printed the instant the ball left the face — ball speed, vertical
launch angle, spin rate, and the launch direction and spin axis beside them — this returns the
path it flew: the polyline, the carry, the curve, how far offline it finished, the apex, the
descent angle and the hang time. It is the model
[ADR-027](../../../docs/decisions/027-ball-flight-simulation.md) was written for, and the reader
`analysis/shot_measure.py` has been storing ball speed and launch angle *for* since M9 while
describing it in the negative — *"fitting inputs... for a model that does not exist yet"*.

**The club is not an input** (ADR-027 §Context 1). Loft and lie belong to the impact model — club
delivery to launch conditions — which is a different model and is not being built. Once the ball
is in the air its path is set by its launch conditions, the air, and its own mass and diameter, and
a club bent 2° strong flies exactly as its launch conditions say it does.

Every constant is read from `benchmarks/flight_model.py` and none is defined here. That ordering is
deliberate and is the reason P2 shipped first: an integrator written before its constants has to
carry one to run at all, and a constant that works is a constant nobody removes
(ADR-027 §Decision 2).

## The frame, and the sign of everything in it

`x` runs downrange along the target line, `y` up, `z` **right** of it — a right-handed frame, and
the one `contracts/shot.py` already signs its fields in (`launch_direction`, `+ = right`). A flight
with no launch direction and no spin axis stays in the `z = 0` plane, which is M15 P3's whole
model and every number it measured.

The one sign that is *not* the contract's is `spin_axis_deg`, and `LaunchConditions` says why: a
fade is right for a right-handed golfer and left for a left-handed one, so a ball's spin axis and
a golfer's shot shape are the same quantity only once handedness is known, and a ball has no
handedness.

## The forces, and where the spin actually enters

Three, per unit mass:

- gravity, `-g` on the vertical axis;
- **drag**, `-0.5*rho*A*Cd*|v|*v`, opposing the velocity;
- **Magnus lift**, `+0.5*rho*A*Cl*|v|^2*(w_hat x v_hat)` — perpendicular to *both* the velocity and
  the spin axis, which needs no special case at the apex, on the way down, or on a curving ball,
  because it is defined off the two vectors rather than off the horizon.

The spin axis is computed once at launch and then **held fixed in space**: a golf ball is
gyroscopically stable over a flight, so the axis does not follow the velocity round as the ball
pitches over. That is what keeps a fade curving after the apex, and it is also why the axis is
square to the velocity only at launch — see `_derivative` for what this module does about that,
which is deliberately nothing.

`Cl` and `Cd` are read against **spin ratio** `S = wR/v` and not against speed
(ADR-027 §Decision 1), and `w` decays exponentially with *absolute* time — which is why the
derivative takes `t` and why each RK4 stage evaluates the spin at its own stage time rather than at
the step's start.

Note what that means about spin, because it decides more than it looks like it does: **`w` reaches
the flight only through `S`, and `S` only through `Cl` and `Cd`.** There is no free `w` left in the
lift term once lift is written with a coefficient. So wherever the coefficient table is being
clamped, spin has no remaining route into the answer at all — see `flight_model.py`, and the
2026-09-05b addendum this module's own measurements put on ADR-027.

## Two numerical choices, both measured rather than assumed

**Landing is solved inside the final step, not rounded to it.** When a whole step ends below the
ground, the crossing is found by re-taking that step at a shortened length — seeded by linear
interpolation on `y`, then refined by Newton on `y(theta)` using the vertical velocity as the
derivative. The reference shot lands doing 23.9 m/s, 19.1 of them horizontal, so a whole-step
termination at the default step overshoots the carry by up to 9.5 cm — and that is exactly the kind
of error a tolerance absorbs silently (ADR-027 §Decision 1). On that shot the solved crossing falls
2.5 ms into the step it landed in.

**The step is not where this model's error lives.** RK4 is fourth order, and over these flights the
carry is converged to within 1e-4 yd at *every* step from 0.02 s down to 0.0005 s — the two
validation shots return 122.4110 yd and 124.0747 yd identically across that whole range.
`DEFAULT_STEP_S` is 0.005 s, four times finer than the coarsest step already measured to converge,
which costs ~900 steps and a few milliseconds. The real disagreement with HD Golf is ~2.5%, three
orders of magnitude larger, and it comes from the coefficient clamp rather than from the
integration.

## The gate, and how to quote it

`VALIDATION_SHOTS`, `GATE_AGREEMENT_FRACTION` and `gate_comparisons()` at the foot of this module
are M15 P4, re-flown on complete launch conditions by M15 P5: the two shots that carry launch
conditions *and* an independent carry, and how far this model lands from that carry. **±2.59%, and
it is a spread rather than an offset** — the two errors are −3.24 yd and +2.39 yd.

Quote that agreement only with two things beside it, both of which the gate's own tests pin. The
first is `fully_clamped`, true on both flights: not one coefficient in either was read from a
measured row. The second is sharper, and it is what the percentage hides — **this model ranks the
two shots the wrong way round.** HD Golf has the lower-spin shot flying 4.6 yd further; this model
has it flying 1.0 yd shorter. Above the clamp the only differences it can see between the two are
2.6° of launch angle and 6.8° of spin axis, and reproducing HD's ordering needs 5.6 yd of
separation out of a spin term that, at these launch conditions, has no route into the answer at
all.

That 1.0 yd was 1.7 yd before P5 flew the recorded spin axes, which is worth stating precisely,
because it is the one piece of the inversion anybody has closed: the axis recovered 0.63 yd of the
6.26 yd, a tenth of it, from a launch condition that is not the spin rate. P4's reading — that
±2.5% is the size of the missing *spin* effect — is therefore an over-attribution, and M15 P9 must
not repeat it.

## The air is an input, and it is the model's rather than this function's

`simulate_flight` takes a `model`, and the air is a block of it — so the altitude what-if is
`simulate_flight(launch, model=load_flight_model().at_altitude(1609))` and this function learns no
new argument (M15 P6). An `altitude_m` keyword beside `model` would have been a second way to say
one thing, and the two would drift.

Thin air is not simply longer. It cuts drag and lift by the same density, so the ball flies
further and arrives **flatter** — lower apex, shorter hang time, shallower descent, and less curve
for the same spin axis. A 7 iron that carries 122.4 yd here carries 126.0 at Denver, and its
curvature falls from 1.71 yd to 1.53.

## What this module deliberately does not do

Roll-out and wind are out of scope for the milestone entirely (ADR-027 §Deferred) — which is worth
remembering when reading `landing_offline_m`, because a golfer's ball finishes where the bounce
took it and this one stops where it landed.

Stdlib and `benchmarks/` only, per ADR-008 and `docs/CODE_STANDARDS.md` R2. **No numpy** — this is
the module in the repo most likely to attract one, and M15 P6's subprocess pin in
`tests/api/test_pipeline_imports.py` fails if one ever appears.
"""

from __future__ import annotations

import math
from typing import NamedTuple

from golf_coach.analysis.benchmarks.flight_model import FlightModel, load_flight_model

#: Exact by definition (NIST), as is the yard at 0.9144 m. Spelled out rather than imported
#: because nothing else in the package converts these and a shared units module for two constants
#: would be the abstraction R11 declines.
_MPH_TO_M_S = 0.44704
_M_TO_YARDS = 1.0 / 0.9144
_RPM_TO_RAD_S = 2.0 * math.pi / 60.0

#: Converged four times over — see the module docstring. Not a tuning knob; a caller varying it is
#: measuring the integrator rather than the flight.
DEFAULT_STEP_S = 0.005

#: Newton on `y(theta)` from a linear seed converges in two or three passes on a descending ball.
#: The budget is generous because exhausting it returns the last iterate rather than raising, and a
#: near-horizontal landing is where the extra passes get used.
_LANDING_REFINEMENTS = 8
#: 1 micrometre. Far below anything reported, and reached long before the budget on a real flight.
_LANDING_TOLERANCE_M = 1e-6

#: A validated launch always lands under gravity and drag, so exceeding this is a wiring bug in the
#: integrator and not a flight — R8's raising side.
_MAX_FLIGHT_S = 30.0


class LaunchConditions(NamedTuple):
    """What the launch monitor printed, in the units it printed them in.

    mph, degrees and rpm rather than SI, because these arrive from `contracts/shot.py`'s
    `ball_speed`, `launch_angle` and `spin_rate` and converting at the call site is how a
    conversion gets done twice. The integrator works in SI internally and reports both.

    The two lateral fields default to zero, and that default is what makes a planar flight the
    same object as a curving one: every call written against M15 P3 still describes the flight it
    described then, down to the last bit.
    """

    ball_speed_mph: float
    launch_angle_deg: float
    spin_rpm: float
    #: `contracts/shot.py`'s `launch_direction`, sign and all: **`+ = right`** of the target line.
    #: A push or a pull, before the ball has curved at all.
    launch_direction_deg: float = 0.0
    #: Geometric, and deliberately **not** `ShotData.spin_axis`'s sign. Positive is a right-hand
    #: rotation of the spin axis about the direction of flight, which curves the ball **right**.
    #: The contract signs the same quantity `+ = fade` — and a fade is right for a right-handed
    #: golfer and left for a left-handed one, so the two agree only once a `Golfer.handedness` is
    #: in hand. This module does not have one and should not acquire one: it flies a ball, and a
    #: ball has no handedness. M15 P9 resolves the axis (ADR-027 §Decision 5) and owns the flip.
    #: `contracts/dispersion.py` is the precedent for why that is not pedantry — a camera-relative
    #: sign meeting a mixed-handedness corpus read every left-handed golfer as a gross fault.
    spin_axis_deg: float = 0.0


class FlightPoint(NamedTuple):
    """One instant of the flight, with the aerodynamic state that produced the next step.

    `spin_ratio` and `clamped` are carried per point rather than summarised once because they are
    not constant along a flight — `S = wR/v` climbs as the ball sheds speed faster than it sheds
    spin — and because a viewer that draws a modelled path (M15 P16) has to be able to say which
    part of it was extrapolated.
    """

    t_s: float
    x_m: float
    y_m: float
    #: Right of the target line, per the module docstring's frame. Exactly 0.0 for the whole
    #: flight when neither lateral launch condition was given.
    z_m: float
    vx_m_s: float
    vy_m_s: float
    vz_m_s: float
    spin_rpm: float
    spin_ratio: float
    #: The coefficients at this point came from holding an end row, not from reading the table.
    clamped: bool

    @property
    def speed_m_s(self) -> float:
        return math.hypot(self.vx_m_s, self.vy_m_s, self.vz_m_s)

    @property
    def x_yds(self) -> float:
        return self.x_m * _M_TO_YARDS

    @property
    def y_yds(self) -> float:
        return self.y_m * _M_TO_YARDS

    @property
    def z_yds(self) -> float:
        return self.z_m * _M_TO_YARDS


class FlightResult(NamedTuple):
    """The path and the six numbers read off it, with the extrapolation stated beside them.

    `clamped_points` is not decoration. Every iron in this repo's corpus flies its entire path on
    the clamped end row of a table measured on a driver, so a carry quoted from here without it is
    a number whose provenance has been dropped — which is the one thing M15 P4 is told not to do.
    """

    launch: LaunchConditions
    points: tuple[FlightPoint, ...]
    #: **Along the launch azimuth, not along the target line** — the landing point rotated into the
    #: frame the ball actually set off in, so that `(carry_m, curvature_m)` are simply its two
    #: coordinates there.
    #:
    #: That choice is the phase's one real decision. Turning a whole flight about the vertical
    #: cannot change how far the ball flew, so carry must not move when the shot starts right; the
    #: projection onto the *target* line would have shortened it by `cos(direction)`. Both
    #: validation shots have a launch direction on disk — −5.3° and +4.0° — so that is 0.43% and
    #: 0.24% of carry, a sixth of `GATE_AGREEMENT_FRACTION`, and it would have entered M15 P8's
    #: spin solve as a bias shaped like the start line. At a launch direction of zero this is
    #: exactly `landing.x_m`, which is the number M15 P3 and P4 measured.
    carry_m: float
    #: Signed deviation of the landing point from the **launch line**, `+ = right`: how far the
    #: ball bent once it was away, with the start direction taken out. Exactly zero whenever the
    #: spin axis is zero, however far offline the shot started — which is what separates a push
    #: from a slice, and what M15 P16 needs in order to draw one differently from the other.
    curvature_m: float
    #: Signed deviation of the landing point from the **target line**, `+ = right`, and the only
    #: one of the three a golfer would recognise. It is the other two added back together:
    #: `carry * sin(direction) + curvature * cos(direction)`. ADR-027 §Decision 6 names it
    #: `flight_landing_offline_yds`.
    landing_offline_m: float
    apex_m: float
    flight_time_s: float
    #: Below the horizontal at landing, read off the landing velocity — and read off it in the
    #: same launch-azimuth frame `carry_m` is in, so the two describe one landing rather than two.
    #: It would exceed 90° for a ball still travelling backwards as it lands, and swept launch
    #: angles from 80° to 89.9° found none: drag kills the backward velocity a steep Magnus lift
    #: builds, and the lift itself turns forward once the ball is descending. `carry_m` does go
    #: negative there — a ball landing behind the tee is a real trajectory rather than an error,
    #: and nothing here clamps it to zero.
    descent_angle_deg: float
    clamped_points: int
    spin_ratio_min: float
    spin_ratio_max: float

    @property
    def carry_yds(self) -> float:
        return self.carry_m * _M_TO_YARDS

    @property
    def apex_yds(self) -> float:
        return self.apex_m * _M_TO_YARDS

    @property
    def curvature_yds(self) -> float:
        return self.curvature_m * _M_TO_YARDS

    @property
    def landing_offline_yds(self) -> float:
        return self.landing_offline_m * _M_TO_YARDS

    @property
    def fully_clamped(self) -> bool:
        """True when no point in the flight read an interpolated row."""
        return self.clamped_points == len(self.points)

    @property
    def landing(self) -> FlightPoint:
        return self.points[-1]

    def sample(self, count: int) -> tuple[FlightPoint, ...]:
        """At most `count` points along the path, launch and landing always among them.

        **Sampled by index rather than by time, so the landing is always the last row.** It is the
        one point the integrator *solved* for rather than stepped to (see `_land`), and a sampler
        that rounded it away would show the flight ending a few centimetres in the air — which on
        a page that draws a landing point is the whole subject of the drawing.

        Fewer than `count` come back where two indices round together, and that is left alone: the
        alternative is spacing them unevenly to hit a count nobody specified.

        Here rather than in either caller because there are two — `scripts/simulate_flight.py`
        prints a table of these and [M15 P14]'s flight route sends them as a polyline — and a
        second copy of the rule is a second place the landing can be dropped.
        """
        n = len(self.points)
        count = min(count, n)
        if count < 2:
            return (self.points[-1],)
        indices = sorted({round(i * (n - 1) / (count - 1)) for i in range(count)})
        return tuple(self.points[i] for i in indices)


#: A point or a direction in the frame the module docstring describes: `(downrange, up, right)`.
_Vec = tuple[float, float, float]
#: `(x, y, z, vx, vy, vz)`.
_State = tuple[float, float, float, float, float, float]


class _Integrand(NamedTuple):
    """What the derivative needs that does not change from one step to the next.

    One record rather than three arguments travelling together, which is what keeps `_rk4_step`
    inside `docs/CODE_STANDARDS.md` R14's five parameters once the spin axis joined them — R5's
    argument applied to a signature instead of to a collection.

    `axis` is the unit spin vector, fixed in space for the whole flight. See the module docstring
    for why it does not follow the velocity round.
    """

    model: FlightModel
    spin0_rad_s: float
    axis: _Vec


def _cross(a: _Vec, b: _Vec) -> _Vec:
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def _launch_frame(launch: LaunchConditions) -> tuple[_Vec, _Vec]:
    """The unit launch velocity, and the unit spin axis it implies.

    The spin axis starts as **pure backspin** — horizontal and square to the launch azimuth, which
    is `velocity x up` normalised and which falls out independent of the launch angle, as it has
    to. It is then rotated about the launch velocity by `spin_axis_deg`: Rodrigues with the
    parallel term dropped, because the two are perpendicular by construction. A golf ball's spin
    axis is set by the face it came off, and a face cannot impart spin about the line of flight.

    Rotating about the **velocity** rather than about the target line is what makes the axis mean
    one thing at every launch angle. Tilting it in the vertical plane instead would give a shot
    launched at 25° a different curve from the same shot launched at 5°, which is not what a
    launch monitor is reporting when it prints an axis.
    """
    alpha = math.radians(launch.launch_angle_deg)
    delta = math.radians(launch.launch_direction_deg)
    beta = math.radians(launch.spin_axis_deg)

    velocity: _Vec = (
        math.cos(alpha) * math.cos(delta),
        math.sin(alpha),
        math.cos(alpha) * math.sin(delta),
    )
    backspin: _Vec = (-math.sin(delta), 0.0, math.cos(delta))
    # The axis of a ball spinning purely sideways: the third leg of the launch triad, and the
    # direction `backspin` tips towards under a positive rotation.
    sideways = _cross(velocity, backspin)
    cos_b, sin_b = math.cos(beta), math.sin(beta)
    axis: _Vec = (
        backspin[0] * cos_b + sideways[0] * sin_b,
        backspin[1] * cos_b + sideways[1] * sin_b,
        backspin[2] * cos_b + sideways[2] * sin_b,
    )
    return velocity, axis


def _derivative(integrand: _Integrand, t_s: float, state: _State) -> _State:
    """Gravity, drag and Magnus lift at one instant, as the six rates of change of the state."""
    model = integrand.model
    _, _, _, vx, vy, vz = state
    speed = math.hypot(vx, vy, vz)
    if speed <= 0.0:
        # A ball at a standstill has no spin ratio (`flight_model.spin_ratio` returns None for
        # exactly this) and therefore no aerodynamic force — only gravity acts.
        return (vx, vy, vz, 0.0, -model.gravity_m_s2, 0.0)

    spin_rad_s = model.spin_decay.spin_after(integrand.spin0_rad_s, t_s)
    coefficients = model.coefficients_at(speed, spin_rad_s)
    if coefficients is None:  # pragma: no cover - speed > 0 was just checked
        raise RuntimeError("a moving ball must have a spin ratio")

    # 0.5*rho*A/m, the factor both forces share once divided through by mass.
    k = 0.5 * model.atmosphere.density_kg_m3 * model.ball.frontal_area_m2 / model.ball.mass_kg
    drag = k * coefficients.cd * speed
    lift = k * coefficients.cl * speed

    # Magnus, `0.5*rho*A*Cl*|v|^2 * (w_hat x v_hat)`, written as `lift * (w_hat x v)` because the
    # extra `|v|` the cross product carries is the one the coefficient form already wanted.
    #
    # **The cross product is not renormalised, and that is the choice worth recording.** Only the
    # spin perpendicular to the velocity makes a Magnus force, so the `sin(theta)` that
    # `|w_hat x v_hat|` carries is the physics rather than an artefact to divide out. At launch
    # the two are square and the factor is exactly 1 — which is why every planar number M15 P3
    # measured survives this phase to the last bit — and on a 21° launch with a 15° axis it has
    # fallen only to 0.94 by landing. Renormalising was the alternative and it is wrong at the
    # limit: it would hold full lift on a ball spinning about its own line of flight, which makes
    # none at all.
    lx, ly, lz = _cross(integrand.axis, (vx, vy, vz))

    return (
        vx,
        vy,
        vz,
        -drag * vx + lift * lx,
        -drag * vy + lift * ly - model.gravity_m_s2,
        -drag * vz + lift * lz,
    )


def _rk4_step(integrand: _Integrand, t_s: float, state: _State, step_s: float) -> _State:
    """One classical RK4 step. Stage times are absolute, because the spin decays against them."""
    half = step_s / 2
    k1 = _derivative(integrand, t_s, state)
    s2 = _advance(state, k1, half)
    k2 = _derivative(integrand, t_s + half, s2)
    s3 = _advance(state, k2, half)
    k3 = _derivative(integrand, t_s + half, s3)
    s4 = _advance(state, k3, step_s)
    k4 = _derivative(integrand, t_s + step_s, s4)
    return (
        state[0] + step_s / 6 * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0]),
        state[1] + step_s / 6 * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1]),
        state[2] + step_s / 6 * (k1[2] + 2 * k2[2] + 2 * k3[2] + k4[2]),
        state[3] + step_s / 6 * (k1[3] + 2 * k2[3] + 2 * k3[3] + k4[3]),
        state[4] + step_s / 6 * (k1[4] + 2 * k2[4] + 2 * k3[4] + k4[4]),
        state[5] + step_s / 6 * (k1[5] + 2 * k2[5] + 2 * k3[5] + k4[5]),
    )


def _advance(state: _State, rate: _State, step_s: float) -> _State:
    """One RK4 stage's trial state. Spelled out rather than zipped, so `mypy` keeps the arity."""
    return (
        state[0] + step_s * rate[0],
        state[1] + step_s * rate[1],
        state[2] + step_s * rate[2],
        state[3] + step_s * rate[3],
        state[4] + step_s * rate[4],
        state[5] + step_s * rate[5],
    )


def _point(integrand: _Integrand, t_s: float, state: _State) -> FlightPoint:
    """Wrap a raw state with the aerodynamic state the integrator saw there."""
    model = integrand.model
    x, y, z, vx, vy, vz = state
    speed = math.hypot(vx, vy, vz)
    spin_rad_s = model.spin_decay.spin_after(integrand.spin0_rad_s, t_s)
    ratio = model.spin_ratio(speed, spin_rad_s)
    coefficients = model.coefficients_at(speed, spin_rad_s)
    return FlightPoint(
        t_s=t_s,
        x_m=x,
        y_m=y,
        z_m=z,
        vx_m_s=vx,
        vy_m_s=vy,
        vz_m_s=vz,
        spin_rpm=spin_rad_s / _RPM_TO_RAD_S,
        # A standstill has no spin ratio; it also cannot happen mid-flight, so 0.0 records the
        # absence rather than standing in for a lookup that was never made.
        spin_ratio=0.0 if ratio is None else ratio,
        clamped=False if coefficients is None else coefficients.clamped,
    )


def _solve_landing(
    integrand: _Integrand, t_s: float, state: _State, step_s: float
) -> tuple[_State, float]:
    """Find `theta` in `(0, step]` where the step that crossed the ground reaches `y = 0`.

    Linear interpolation on `y` seeds it and Newton refines it, both re-running the *integrator*
    over the short step rather than interpolating the trajectory — a descending ball is curving,
    and the whole reason this function exists is that the shape of the last fraction of a step is
    worth more than the step boundary it fell between (ADR-027 §Decision 1).

    Newton walks on `y(theta)` with `dy/dtheta = vy`, which is well conditioned precisely because
    the ball is descending. It stops early rather than raising if it ever is not: an exhausted
    budget returns the closest iterate found, because a landing point a micrometre out is not a
    reason to lose a whole flight.

    Solving on `y` alone stays right in three dimensions. The ground is the `y = 0` plane whatever
    the ball is doing sideways, and the lateral coordinates arrive with the shortened step rather
    than being interpolated separately onto it.
    """
    ended = _rk4_step(integrand, t_s, state, step_s)
    span = state[1] - ended[1]
    theta = step_s * state[1] / span if span > 0.0 else step_s / 2
    # Bounded into the step it belongs to. The lower bound matters at launch, where `y` is exactly
    # zero and the linear seed degenerates to a zero-length step Newton cannot move off.
    lo = step_s * 1e-9
    theta = min(max(theta, lo), step_s)

    for _ in range(_LANDING_REFINEMENTS):
        landed = _rk4_step(integrand, t_s, state, theta)
        if abs(landed[1]) <= _LANDING_TOLERANCE_M or landed[4] >= 0.0:
            return landed, theta
        theta = min(max(theta - landed[1] / landed[4], lo), step_s)
    return _rk4_step(integrand, t_s, state, theta), theta


def simulate_flight(
    launch: LaunchConditions,
    *,
    model: FlightModel | None = None,
    step_s: float = DEFAULT_STEP_S,
) -> FlightResult:
    """Fly a ball from its launch conditions to the ground.

    Raises `ValueError` on launch conditions there is no flight to integrate from. That is R8's
    raising side rather than a refusal: the callers that read real shots (M15 P7-P10) are the
    boundary, and one of them handing this a launch angle of zero has skipped a check it owns —
    a ball on the ground at or below the horizontal does not fly, it rolls, and roll is out of
    scope for the milestone. A refusal that reached a golfer would have to say something, and
    "your launch angle was zero" is a sentence about the launch monitor, not about the swing.
    """
    if launch.ball_speed_mph <= 0.0:
        raise ValueError(f"ball speed must be positive to fly, got {launch.ball_speed_mph} mph")
    if not 0.0 < launch.launch_angle_deg < 90.0:
        raise ValueError(
            "launch angle must be above the horizontal and below the vertical to leave the "
            f"ground, got {launch.launch_angle_deg} deg"
        )
    if launch.spin_rpm < 0.0:
        # Top spin is a real thing and a signed rate is how it would arrive, but the coefficient
        # table is one-sided and clamping a negative ratio to the first row would fly a top-spun
        # ball with lift. Refused here rather than flown wrong.
        raise ValueError(f"spin rate must not be negative, got {launch.spin_rpm} rpm")
    if not -90.0 < launch.launch_direction_deg < 90.0:
        # A quarter turn off the target line is already a shank; past it the ball travels back
        # towards the golfer and `carry_m` stops being a distance towards anything. The screen
        # never prints one, so a caller holding one has a parse error rather than a shot.
        raise ValueError(
            "launch direction must be within a quarter turn of the target line, got "
            f"{launch.launch_direction_deg} deg"
        )
    if not -90.0 <= launch.spin_axis_deg <= 90.0:
        # At exactly +/-90 the axis is vertical, the ball spins purely sideways, and that is a
        # flight this model can fly. Past it the axis has tipped over into top spin, refused for
        # the same reason a negative spin rate is: the coefficient table is one-sided.
        raise ValueError(
            f"spin axis must be within a quarter turn of horizontal, got {launch.spin_axis_deg} deg"
        )
    if step_s <= 0.0:
        raise ValueError(f"step must be positive, got {step_s} s")

    model = load_flight_model() if model is None else model
    speed = launch.ball_speed_mph * _MPH_TO_M_S
    direction, axis = _launch_frame(launch)
    integrand = _Integrand(model, launch.spin_rpm * _RPM_TO_RAD_S, axis)

    state: _State = (
        0.0,
        0.0,
        0.0,
        speed * direction[0],
        speed * direction[1],
        speed * direction[2],
    )
    t_s = 0.0
    points = [_point(integrand, t_s, state)]

    while True:
        stepped = _rk4_step(integrand, t_s, state, step_s)
        if stepped[1] <= 0.0:
            landed, theta = _solve_landing(integrand, t_s, state, step_s)
            points.append(_point(integrand, t_s + theta, landed))
            break
        t_s += step_s
        state = stepped
        points.append(_point(integrand, t_s, state))
        if t_s > _MAX_FLIGHT_S:
            raise RuntimeError(
                f"{launch} did not reach the ground in {_MAX_FLIGHT_S} s, which gravity and drag "
                "make impossible - the integrator is wrong, not the shot"
            )

    landing = points[-1]
    # The landing point and the landing velocity turned into the frame the ball set off in. One
    # rotation applied twice, so `carry_m`, `curvature_m` and `descent_angle_deg` cannot drift
    # into describing different frames.
    cos_d = math.cos(math.radians(launch.launch_direction_deg))
    sin_d = math.sin(math.radians(launch.launch_direction_deg))
    return FlightResult(
        launch=launch,
        points=tuple(points),
        carry_m=landing.x_m * cos_d + landing.z_m * sin_d,
        curvature_m=landing.z_m * cos_d - landing.x_m * sin_d,
        landing_offline_m=landing.z_m,
        # The sampled maximum, not an interpolated vertex. `y' = 0` at the apex makes `y`
        # stationary there, so the worst a sample half a step away can miss by is g*(h/2)^2/2 —
        # 0.03 mm at the default step, four orders below anything this number is compared against.
        apex_m=max(point.y_m for point in points),
        flight_time_s=landing.t_s,
        descent_angle_deg=math.degrees(
            math.atan2(-landing.vy_m_s, landing.vx_m_s * cos_d + landing.vz_m_s * sin_d)
        ),
        clamped_points=sum(1 for point in points if point.clamped),
        spin_ratio_min=min(point.spin_ratio for point in points),
        spin_ratio_max=max(point.spin_ratio for point in points),
    )


# ---------------------------------------------------------------------------------------------
# The gate [M15 P4] — the two shots this model can be checked against, and how far off it was
# ---------------------------------------------------------------------------------------------


class ValidationShot(NamedTuple):
    """A shot where the launch conditions *and* an independent carry were both recorded.

    There are two, both from 2026-08-10, and there will not be a third until the bay's screen
    profile starts printing spin (ADR-027 §Context 2). `simulator_carry_yds` is HD Golf's own
    number, read off the same screen as the launch conditions beside it. That makes it independent
    of everything in this package and *dependent on HD Golf's flight model* — it is not a tape
    measure, and §Decision 3 turns on the difference.
    """

    shot_id: str
    launch: LaunchConditions
    simulator_carry_yds: float


#: ADR-027 §Context 2's table, as data. It was hand-typed into four documents before this constant
#: existed; this is the copy anything executable reads (`docs/CODE_STANDARDS.md` R4).
#:
#: **M15 P5 completed these rows and the gate moved.** Until it, the two shots were flown planar
#: because the integrator had no third dimension to fly them in — but the launch direction and the
#: spin axis were on disk all along, and HD Golf's printed carry is the carry of the curving shot,
#: not of a planar approximation of it. The direction cannot move a carry defined along the launch
#: azimuth (`FlightResult.carry_m` says why); the **axis can and does**, because lift is one vector
#: and what it spends bending the ball it does not spend holding it up.
#:
#: The axis sign is the one thing here that is not simply the contract's, and it does not matter to
#: this gate: carry is *even* in the spin axis, so a right-handed golfer's fade and a left-handed
#: golfer's fly the same distance. What handedness decides is which side the ball finishes, and
#: that is M15 P9's to resolve (ADR-027 §Decision 5). The magnitudes are straight OCR reads and
#: `screen/validate.py` cross-checked both against the `Shot Type` tile beside them.
VALIDATION_SHOTS: tuple[ValidationShot, ...] = (
    ValidationShot("2026-08-10-1", LaunchConditions(90.7, 20.9, 5991.0, -5.3, 2.5), 125.6),
    ValidationShot("2026-08-10-2", LaunchConditions(90.5, 23.5, 8100.0, 4.0, 9.3), 121.0),
)

#: **Measured, then pinned — never chosen.** ADR-027 and `ROADMAP.md`'s M15 section both say the
#: tolerance is whatever the gate achieves, because M15 P9 bakes it into every inferred spin and a
#: budget widened until it passes would be baked in just as silently. The achieved worst case is
#: 2.58224%; this is that rounded up in the fourth decimal, so there is no slack in it for a
#: regression to hide in — `test_flight_validation.py` pins the tightness as well as the pass.
#:
#: **M15 P5 moved this from 0.0255, and the direction of the move is the point.** Flying the two
#: shots with their recorded spin axes made one worse (−2.539% → −2.582%) and the other markedly
#: better (+2.541% → +1.973%), so the pin rose 0.0004 while the mean absolute error *fell*. It is
#: not a tolerance widened until a model passed; it is the same tolerance rule applied to the shots
#: HD Golf actually measured. See ADR-027's 2026-09-05d addendum for what that cost P4's
#: cancellation argument.
#:
#: Read it with `FlightResult.fully_clamped`, which is true on both shots, and never without:
#: neither flight read one measured coefficient row, so this is the agreement between HD Golf and
#: a *held end row*, not a validation of the published table (ADR-027's 2026-09-05 addendum).
GATE_AGREEMENT_FRACTION = 0.0259


class GateComparison(NamedTuple):
    """One validation shot flown, beside the carry the simulator printed for it."""

    shot: ValidationShot
    flight: FlightResult

    @property
    def error_yds(self) -> float:
        """Signed: positive where this model flies the ball further than HD Golf did."""
        return self.flight.carry_yds - self.shot.simulator_carry_yds

    @property
    def error_fraction(self) -> float:
        return self.error_yds / self.shot.simulator_carry_yds

    @property
    def agrees(self) -> bool:
        return abs(self.error_fraction) <= GATE_AGREEMENT_FRACTION


def gate_comparisons(
    *, model: FlightModel | None = None, step_s: float = DEFAULT_STEP_S
) -> tuple[GateComparison, ...]:
    """Fly both validation shots and report the disagreement with the simulator.

    A function rather than a stored table of results, so the gate is re-run rather than remembered:
    a re-sourced coefficient table moves these numbers and nothing that merely quoted them would
    notice. M15 P9 and P16 both have to state this disagreement beside an inferred spin, and this
    is where they get it.
    """
    model = load_flight_model() if model is None else model
    return tuple(
        GateComparison(shot, simulate_flight(shot.launch, model=model, step_s=step_s))
        for shot in VALIDATION_SHOTS
    )
