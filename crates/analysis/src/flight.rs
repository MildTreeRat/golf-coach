//! The ball-flight integrator — RK4 in three dimensions. [M22 P8]
//!
//! The port of `analysis/flight.py`. Given what the launch monitor printed the instant the ball
//! left the face — ball speed, vertical launch angle, spin rate, and the launch direction and spin
//! axis beside them — this returns the path it flew: the polyline, the carry, the curve, how far
//! offline it finished, the apex, the descent angle and the hang time. It is the model
//! [ADR-027](../../../docs/decisions/027-ball-flight-simulation.md) was written for.
//!
//! **The club is not an input** (ADR-027 §Context 1). Loft and lie belong to the impact model —
//! club delivery to launch conditions — which is a different model and is not being built. Once the
//! ball is in the air its path is set by its launch conditions, the air, and its own mass and
//! diameter, and a club bent 2° strong flies exactly as its launch conditions say it does. Loft's
//! entire involvement is choosing between the two candidate spins [`crate::spin_solve`] returns,
//! which is P8b's.
//!
//! Every constant is read from [`crate::benchmarks::flight_model`] and none is defined here, which
//! is the ordering ADR-027 §Decision 2 asks for: an integrator written before its constants has to
//! carry one to run at all, and a constant that works is a constant nobody removes.
//!
//! # The frame, and the sign of everything in it
//!
//! `x` runs downrange along the target line, `y` up, `z` **right** of it — a right-handed frame, and
//! the one `contracts::shot` already signs its fields in (`launch_direction`, `+ = right`). A flight
//! with no launch direction and no spin axis stays in the `z = 0` plane.
//!
//! The one sign that is *not* the contract's is [`LaunchConditions::spin_axis_deg`], and its own doc
//! says why: a fade is right for a right-handed golfer and left for a left-handed one, so a ball's
//! spin axis and a golfer's shot shape are the same quantity only once handedness is known, and a
//! ball has no handedness.
//!
//! # The forces, and where the spin actually enters
//!
//! Three, per unit mass: gravity, `-g` on the vertical axis; **drag**, `-0.5*rho*A*Cd*|v|*v`; and
//! **Magnus lift**, `+0.5*rho*A*Cl*|v|^2*(w_hat x v_hat)` — perpendicular to both the velocity and
//! the spin axis, which needs no special case at the apex or on a curving ball because it is defined
//! off the two vectors rather than off the horizon.
//!
//! The spin axis is computed once at launch and then **held fixed in space**: a golf ball is
//! gyroscopically stable over a flight, so the axis does not follow the velocity round as the ball
//! pitches over. That is what keeps a fade curving after the apex.
//!
//! `Cl` and `Cd` are read against **spin ratio** `S = wR/v` and not against speed (ADR-027
//! §Decision 1), and `w` decays exponentially with *absolute* time — which is why [`derivative`]
//! takes `t` and why each RK4 stage evaluates the spin at its own stage time rather than at the
//! step's start. Note what that means about spin: **`w` reaches the flight only through `S`, and `S`
//! only through `Cl` and `Cd`.** Wherever the coefficient table is being clamped, spin has no
//! remaining route into the answer at all.
//!
//! # What the port had to decide, and what the gate then measured
//!
//! Two numerical choices came across as written and both are load-bearing. **Landing is solved
//! inside the final step, not rounded to it** ([`solve_landing`]) — a whole-step termination
//! overshoots the reference shot's carry by up to 9.5 cm, which is exactly the size of error a
//! tolerance absorbs silently. And **`t_s` accumulates by `+= step_s`** rather than being recomputed
//! as `i * step_s`: the two differ in the last bits, the spin decays against `t_s`, and there are
//! about 900 additions in a flight.
//!
//! The one edge with no counterpart in std is **`math.hypot` with three arguments**, and it turned out
//! to be a real portability edge rather than the accepted `hypot` equivalence P5b recorded. CPython's
//! is a scaled, Neumaier-compensated norm; Rust has only the two-argument [`f64::hypot`], and both
//! obvious stand-ins — chained `hypot` and the naive `(x²+y²+z²).sqrt()` — sit within **1 ulp** of
//! CPython at every point of a committed flight. One ulp is the whole difference: the reference shot's
//! launch speed is `40.546527999999995` to CPython and `40.546528` to both approximations, and
//! [`crate::spin_solve::carry_window`] constructs `high_plateau_min_rpm` so that the launch spin ratio
//! lands **exactly** on the coefficient table's last row — so the first spelling makes
//! `AeroCoefficients::clamped` true at that shoulder and the second makes it false. A flipped clamp is
//! a different flight, and `clamped` is compared exactly.
//!
//! So [`hypot3`] goes through [`crate::pyfmt::hypot`], which is CPython's `vector_norm` transcribed —
//! the fourth edge in a module whose first three all reached a *string*. With it, all 41,287 floats in
//! the five committed flights come back **bit-identical**, not merely inside `RTOL`
//! (`tests/flight.rs`), which is a stronger statement than this phase set out to make and is the one
//! that would notice a fifth edge.
//!
//! # What is not here
//!
//! `VALIDATION_SHOTS`, `GATE_AGREEMENT_FRACTION` and `gate_comparisons()` — M15 P4's gate, the two
//! shots this model can be checked against and how far off it was. They are read by
//! `scripts/simulate_flight.py`, by `flight_infer`'s own gate helper and by the Python tests, none
//! of which is on `conformance.py::run_vector`'s path, so no committed vector holds an answer for
//! one. `FlightResult.sample` goes with them: its callers are the CLI and `api/flight_view.py`,
//! which [ADR-032](../../../docs/decisions/032-the-rust-core.md) §7 retires rather than ports. Same
//! rule as `benchmarks::store`'s `source_date` and `flight_model`'s altitude what-if.
//!
//! Roll-out and wind are out of scope for the milestone entirely (ADR-027 §Deferred) — worth
//! remembering when reading [`FlightResult::landing_offline_m`], because a golfer's ball finishes
//! where the bounce took it and this one stops where it landed.

use crate::benchmarks::flight_model::FlightModel;

/// Exact by definition (NIST), as is the yard at 0.9144 m. Spelled out rather than shared because
/// nothing else in the crate converts these and a units module for two constants would be the
/// abstraction `docs/CODE_STANDARDS.md` R11 declines.
const MPH_TO_M_S: f64 = 0.44704;
/// **A reciprocal that is then multiplied, not a division by 0.9144.** The Python precomputes
/// `1.0 / 0.9144`, and `x * (1/d)` is not `x / d` in the last bit — every yard figure in
/// `spec/vectors/` was recorded through the multiply.
const M_TO_YARDS: f64 = 1.0 / 0.9144;
const RPM_TO_RAD_S: f64 = 2.0 * std::f64::consts::PI / 60.0;

/// Converged four times over: the carry is stable to within 1e-4 yd at every step from 0.02 s down
/// to 0.0005 s. Not a tuning knob — a caller varying it is measuring the integrator rather than the
/// flight. **The step is not where this model's error lives**; the ~2.5% disagreement with HD Golf
/// is three orders larger and comes from the coefficient clamp.
pub const DEFAULT_STEP_S: f64 = 0.005;

/// Newton on `y(theta)` from a linear seed converges in two or three passes on a descending ball.
/// The budget is generous because exhausting it returns the last iterate rather than raising, and a
/// near-horizontal landing is where the extra passes get used.
const LANDING_REFINEMENTS: usize = 8;
/// 1 micrometre. Far below anything reported, and reached long before the budget on a real flight.
const LANDING_TOLERANCE_M: f64 = 1e-6;

/// A validated launch always lands under gravity and drag, so exceeding this is a wiring bug in the
/// integrator and not a flight — `docs/CODE_STANDARDS.md` R8's raising side.
const MAX_FLIGHT_S: f64 = 30.0;

/// Launch conditions there is no flight to integrate from — the port of Python's `ValueError`.
///
/// **A refusal to the caller and a raise in the Python**, and the distinction is P8b's to spend:
/// `flight_measure.fly_shot` catches this and reports `NO_LAUNCH_CONDITIONS` with `str(exc)` as the
/// detail, so the messages below are compared byte for byte the moment a vector reaches one. None
/// of the fifteen does today, which is why they are pinned by unit test here rather than left for
/// the phase that consumes them.
///
/// The one thing Python raises that is *not* this is [`MAX_FLIGHT_S`]'s `RuntimeError`, which
/// `fly_shot` does not catch. It panics here, which is the same reachability: a ball that does not
/// land in 30 s is the integrator being wrong, not the shot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnflyableLaunch {
    /// `str(exc)`, exactly as Python's would read.
    pub message: String,
}

/// What the launch monitor printed, in the units it printed them in.
///
/// mph, degrees and rpm rather than SI, because these arrive from `contracts::shot`'s `ball_speed`,
/// `launch_angle` and `spin_rate` and converting at the call site is how a conversion gets done
/// twice. The integrator works in SI internally and reports both.
///
/// The two lateral fields default to zero in Python, and that default is what makes a planar flight
/// the same object as a curving one. Rust has no default arguments, so [`LaunchConditions::planar`]
/// is the two-field constructor and the struct literal is the five-field one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaunchConditions {
    pub ball_speed_mph: f64,
    pub launch_angle_deg: f64,
    pub spin_rpm: f64,
    /// `contracts::shot`'s `launch_direction`, sign and all: **`+ = right`** of the target line. A
    /// push or a pull, before the ball has curved at all.
    pub launch_direction_deg: f64,
    /// Geometric, and deliberately **not** `ShotData.spin_axis`'s sign. Positive is a right-hand
    /// rotation of the spin axis about the direction of flight, which curves the ball **right**. The
    /// contract signs the same quantity `+ = fade` — and a fade is right for a right-handed golfer
    /// and left for a left-handed one, so the two agree only once a handedness is in hand. This
    /// module does not have one and should not acquire one: it flies a ball, and a ball has no
    /// handedness. P8b resolves the axis (ADR-027 §Decision 5) and owns the flip.
    pub spin_axis_deg: f64,
}

impl LaunchConditions {
    /// The three-argument call, with both lateral fields at their Python defaults. Every planar
    /// flight M15 P3 measured is this, down to the last bit.
    pub fn planar(ball_speed_mph: f64, launch_angle_deg: f64, spin_rpm: f64) -> Self {
        Self {
            ball_speed_mph,
            launch_angle_deg,
            spin_rpm,
            launch_direction_deg: 0.0,
            spin_axis_deg: 0.0,
        }
    }
}

/// One instant of the flight, with the aerodynamic state that produced the next step.
///
/// `spin_ratio` and `clamped` are carried per point rather than summarised once because they are not
/// constant along a flight — `S = wR/v` climbs as the ball sheds speed faster than it sheds spin —
/// and because a viewer that draws a modelled path has to be able to say which part of it was
/// extrapolated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightPoint {
    pub t_s: f64,
    pub x_m: f64,
    pub y_m: f64,
    /// Right of the target line, per the module doc's frame. Exactly 0.0 for the whole flight when
    /// neither lateral launch condition was given.
    pub z_m: f64,
    pub vx_m_s: f64,
    pub vy_m_s: f64,
    pub vz_m_s: f64,
    pub spin_rpm: f64,
    pub spin_ratio: f64,
    /// The coefficients at this point came from holding an end row, not from reading the table.
    pub clamped: bool,
}

impl FlightPoint {
    pub fn speed_m_s(&self) -> f64 {
        hypot3(self.vx_m_s, self.vy_m_s, self.vz_m_s)
    }
}

/// The path and the six numbers read off it, with the extrapolation stated beside them.
///
/// `clamped_points` is not decoration. Every iron in this repo's corpus flies its entire path on the
/// clamped end row of a table measured on a driver, so a carry quoted from here without it is a
/// number whose provenance has been dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct FlightResult {
    pub launch: LaunchConditions,
    pub points: Vec<FlightPoint>,
    /// **Along the launch azimuth, not along the target line** — the landing point rotated into the
    /// frame the ball actually set off in, so that `(carry_m, curvature_m)` are simply its two
    /// coordinates there.
    ///
    /// That choice is M15 P5's one real decision. Turning a whole flight about the vertical cannot
    /// change how far the ball flew, so carry must not move when the shot starts right; the
    /// projection onto the *target* line would have shortened it by `cos(direction)`. Both
    /// validation shots have a launch direction on disk — −5.3° and +4.0° — so that is 0.43% and
    /// 0.24% of carry, a sixth of the gate's tolerance, and it would have entered the spin solve as
    /// a bias shaped like the start line.
    pub carry_m: f64,
    /// Signed deviation of the landing point from the **launch line**, `+ = right`: how far the ball
    /// bent once it was away, with the start direction taken out. Exactly zero whenever the spin
    /// axis is zero, however far offline the shot started — which is what separates a push from a
    /// slice.
    pub curvature_m: f64,
    /// Signed deviation of the landing point from the **target line**, `+ = right`, and the only one
    /// of the three a golfer would recognise. It is the other two added back together:
    /// `carry * sin(direction) + curvature * cos(direction)`. ADR-027 §Decision 6 names it
    /// `flight_landing_offline_yds`.
    pub landing_offline_m: f64,
    pub apex_m: f64,
    pub flight_time_s: f64,
    /// Below the horizontal at landing, read off the landing velocity — and read off it in the same
    /// launch-azimuth frame `carry_m` is in, so the two describe one landing rather than two.
    pub descent_angle_deg: f64,
    pub clamped_points: usize,
    pub spin_ratio_min: f64,
    pub spin_ratio_max: f64,
}

impl FlightResult {
    pub fn carry_yds(&self) -> f64 {
        self.carry_m * M_TO_YARDS
    }

    pub fn apex_yds(&self) -> f64 {
        self.apex_m * M_TO_YARDS
    }

    pub fn curvature_yds(&self) -> f64 {
        self.curvature_m * M_TO_YARDS
    }

    pub fn landing_offline_yds(&self) -> f64 {
        self.landing_offline_m * M_TO_YARDS
    }

    /// True when no point in the flight read an interpolated row. ADR-027's 2026-09-05 addendum
    /// requires this beside any quoted agreement with HD Golf, and it is true on both validation
    /// shots.
    pub fn fully_clamped(&self) -> bool {
        self.clamped_points == self.points.len()
    }

    pub fn landing(&self) -> &FlightPoint {
        &self.points[self.points.len() - 1]
    }
}

/// A point or a direction in the frame the module doc describes: `(downrange, up, right)`.
type Vec3 = (f64, f64, f64);
/// `(x, y, z, vx, vy, vz)`.
type State = [f64; 6];

/// What the derivative needs that does not change from one step to the next.
///
/// One record rather than three arguments travelling together, which is what keeps [`rk4_step`]
/// inside `docs/CODE_STANDARDS.md` R14's five parameters once the spin axis joined them.
///
/// `axis` is the unit spin vector, fixed in space for the whole flight.
struct Integrand<'a> {
    model: &'a FlightModel,
    spin0_rad_s: f64,
    axis: Vec3,
}

/// `math.hypot(a, b, c)`, through [`crate::pyfmt::hypot`] rather than a chain of [`f64::hypot`].
///
/// The module doc records why: the two differ by one ulp on the reference shot's launch velocity, and
/// one ulp flips a `clamped` bool at the shoulder [`crate::spin_solve`] constructs to sit exactly on
/// the coefficient table's last row.
fn hypot3(a: f64, b: f64, c: f64) -> f64 {
    crate::pyfmt::hypot(&[a, b, c])
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    )
}

/// The unit launch velocity, and the unit spin axis it implies.
///
/// The spin axis starts as **pure backspin** — horizontal and square to the launch azimuth, which is
/// `velocity x up` normalised and which falls out independent of the launch angle, as it has to. It
/// is then rotated about the launch velocity by `spin_axis_deg`: Rodrigues with the parallel term
/// dropped, because the two are perpendicular by construction. A golf ball's spin axis is set by the
/// face it came off, and a face cannot impart spin about the line of flight.
///
/// Rotating about the **velocity** rather than about the target line is what makes the axis mean one
/// thing at every launch angle. Tilting it in the vertical plane instead would give a shot launched
/// at 25° a different curve from the same shot launched at 5°, which is not what a launch monitor is
/// reporting when it prints an axis.
fn launch_frame(launch: &LaunchConditions) -> (Vec3, Vec3) {
    let alpha = launch.launch_angle_deg.to_radians();
    let delta = launch.launch_direction_deg.to_radians();
    let beta = launch.spin_axis_deg.to_radians();

    let velocity: Vec3 = (
        alpha.cos() * delta.cos(),
        alpha.sin(),
        alpha.cos() * delta.sin(),
    );
    let backspin: Vec3 = (-delta.sin(), 0.0, delta.cos());
    // The axis of a ball spinning purely sideways: the third leg of the launch triad, and the
    // direction `backspin` tips towards under a positive rotation.
    let sideways = cross(velocity, backspin);
    let (cos_b, sin_b) = (beta.cos(), beta.sin());
    let axis: Vec3 = (
        backspin.0 * cos_b + sideways.0 * sin_b,
        backspin.1 * cos_b + sideways.1 * sin_b,
        backspin.2 * cos_b + sideways.2 * sin_b,
    );
    (velocity, axis)
}

/// Gravity, drag and Magnus lift at one instant, as the six rates of change of the state.
fn derivative(integrand: &Integrand, t_s: f64, state: &State) -> State {
    let model = integrand.model;
    let (vx, vy, vz) = (state[3], state[4], state[5]);
    let speed = hypot3(vx, vy, vz);
    if speed <= 0.0 {
        // A ball at a standstill has no spin ratio (`FlightModel::spin_ratio` returns `None` for
        // exactly this) and therefore no aerodynamic force — only gravity acts.
        return [vx, vy, vz, 0.0, -model.gravity_m_s2, 0.0];
    }

    let spin_rad_s = model.spin_decay.spin_after(integrand.spin0_rad_s, t_s);
    let coefficients = model
        .coefficients_at(speed, spin_rad_s)
        .expect("a moving ball must have a spin ratio");

    // 0.5*rho*A/m, the factor both forces share once divided through by mass.
    let k =
        0.5 * model.atmosphere.density_kg_m3 * model.ball.frontal_area_m2() / model.ball.mass_kg;
    let drag = k * coefficients.cd * speed;
    let lift = k * coefficients.cl * speed;

    // Magnus, `0.5*rho*A*Cl*|v|^2 * (w_hat x v_hat)`, written as `lift * (w_hat x v)` because the
    // extra `|v|` the cross product carries is the one the coefficient form already wanted.
    //
    // **The cross product is not renormalised, and that is the choice worth recording.** Only the
    // spin perpendicular to the velocity makes a Magnus force, so the `sin(theta)` that
    // `|w_hat x v_hat|` carries is the physics rather than an artefact to divide out. At launch the
    // two are square and the factor is exactly 1 — which is why every planar number M15 P3 measured
    // survives the third dimension to the last bit — and on a 21° launch with a 15° axis it has
    // fallen only to 0.94 by landing. Renormalising was the alternative and it is wrong at the
    // limit: it would hold full lift on a ball spinning about its own line of flight, which makes
    // none at all.
    let (lx, ly, lz) = cross(integrand.axis, (vx, vy, vz));

    [
        vx,
        vy,
        vz,
        -drag * vx + lift * lx,
        -drag * vy + lift * ly - model.gravity_m_s2,
        -drag * vz + lift * lz,
    ]
}

/// One classical RK4 step. Stage times are absolute, because the spin decays against them.
///
/// `step_s / 6 * (...)` and not `step_s * (...) / 6`: the divide comes first in the Python and the
/// two orders differ in the last bit of every one of the ~900 steps in a flight.
fn rk4_step(integrand: &Integrand, t_s: f64, state: &State, step_s: f64) -> State {
    let half = step_s / 2.0;
    let k1 = derivative(integrand, t_s, state);
    let s2 = advance(state, &k1, half);
    let k2 = derivative(integrand, t_s + half, &s2);
    let s3 = advance(state, &k2, half);
    let k3 = derivative(integrand, t_s + half, &s3);
    let s4 = advance(state, &k3, step_s);
    let k4 = derivative(integrand, t_s + step_s, &s4);
    let mut out = [0.0; 6];
    for i in 0..6 {
        out[i] = state[i] + step_s / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
    out
}

/// One RK4 stage's trial state.
fn advance(state: &State, rate: &State, step_s: f64) -> State {
    let mut out = [0.0; 6];
    for i in 0..6 {
        out[i] = state[i] + step_s * rate[i];
    }
    out
}

/// Wrap a raw state with the aerodynamic state the integrator saw there.
fn point(integrand: &Integrand, t_s: f64, state: &State) -> FlightPoint {
    let model = integrand.model;
    let speed = hypot3(state[3], state[4], state[5]);
    let spin_rad_s = model.spin_decay.spin_after(integrand.spin0_rad_s, t_s);
    let ratio = model.spin_ratio(speed, spin_rad_s);
    let coefficients = model.coefficients_at(speed, spin_rad_s);
    FlightPoint {
        t_s,
        x_m: state[0],
        y_m: state[1],
        z_m: state[2],
        vx_m_s: state[3],
        vy_m_s: state[4],
        vz_m_s: state[5],
        spin_rpm: spin_rad_s / RPM_TO_RAD_S,
        // A standstill has no spin ratio; it also cannot happen mid-flight, so 0.0 records the
        // absence rather than standing in for a lookup that was never made.
        spin_ratio: ratio.unwrap_or(0.0),
        clamped: coefficients.is_some_and(|c| c.clamped),
    }
}

/// Find `theta` in `(0, step]` where the step that crossed the ground reaches `y = 0`.
///
/// Linear interpolation on `y` seeds it and Newton refines it, both re-running the *integrator* over
/// the short step rather than interpolating the trajectory — a descending ball is curving, and the
/// whole reason this function exists is that the shape of the last fraction of a step is worth more
/// than the step boundary it fell between (ADR-027 §Decision 1).
///
/// Newton walks on `y(theta)` with `dy/dtheta = vy`, which is well conditioned precisely because the
/// ball is descending. It stops early rather than raising if it ever is not: an exhausted budget
/// returns the closest iterate found, because a landing point a micrometre out is not a reason to
/// lose a whole flight.
///
/// Solving on `y` alone stays right in three dimensions. The ground is the `y = 0` plane whatever the
/// ball is doing sideways, and the lateral coordinates arrive with the shortened step rather than
/// being interpolated separately onto it.
fn solve_landing(integrand: &Integrand, t_s: f64, state: &State, step_s: f64) -> (State, f64) {
    let ended = rk4_step(integrand, t_s, state, step_s);
    let span = state[1] - ended[1];
    let mut theta = if span > 0.0 {
        step_s * state[1] / span
    } else {
        step_s / 2.0
    };
    // Bounded into the step it belongs to. The lower bound matters at launch, where `y` is exactly
    // zero and the linear seed degenerates to a zero-length step Newton cannot move off.
    let lo = step_s * 1e-9;
    theta = theta.max(lo).min(step_s);

    for _ in 0..LANDING_REFINEMENTS {
        let landed = rk4_step(integrand, t_s, state, theta);
        if landed[1].abs() <= LANDING_TOLERANCE_M || landed[4] >= 0.0 {
            return (landed, theta);
        }
        theta = (theta - landed[1] / landed[4]).max(lo).min(step_s);
    }
    (rk4_step(integrand, t_s, state, theta), theta)
}

/// Fly a ball from its launch conditions to the ground.
///
/// Refuses launch conditions there is no flight to integrate from, which is Python's `ValueError`
/// and R8's raising side rather than a golfer-facing refusal: the callers that read real shots are
/// the boundary, and one of them handing this a launch angle of zero has skipped a check it owns — a
/// ball on the ground at or below the horizontal does not fly, it rolls, and roll is out of scope
/// for the milestone. A refusal that reached a golfer would have to say something, and "your launch
/// angle was zero" is a sentence about the launch monitor, not about the swing.
///
/// `model` and `step_s` are explicit where Python defaults them, following P4's precedent with
/// `smooth_keypoints`' window: a caller that means the committed model writes
/// `load_flight_model()`, and a defaulted argument in a port is a place the two languages can
/// silently disagree about what the default was.
pub fn simulate_flight(
    launch: &LaunchConditions,
    model: &FlightModel,
    step_s: f64,
) -> Result<FlightResult, UnflyableLaunch> {
    if launch.ball_speed_mph <= 0.0 {
        return Err(refuse(format!(
            "ball speed must be positive to fly, got {} mph",
            crate::pyfmt::repr(launch.ball_speed_mph)
        )));
    }
    if !(launch.launch_angle_deg > 0.0 && launch.launch_angle_deg < 90.0) {
        return Err(refuse(format!(
            "launch angle must be above the horizontal and below the vertical to leave the ground, \
             got {} deg",
            crate::pyfmt::repr(launch.launch_angle_deg)
        )));
    }
    if launch.spin_rpm < 0.0 {
        // Top spin is a real thing and a signed rate is how it would arrive, but the coefficient
        // table is one-sided and clamping a negative ratio to the first row would fly a top-spun
        // ball with lift. Refused here rather than flown wrong.
        return Err(refuse(format!(
            "spin rate must not be negative, got {} rpm",
            crate::pyfmt::repr(launch.spin_rpm)
        )));
    }
    if !(launch.launch_direction_deg > -90.0 && launch.launch_direction_deg < 90.0) {
        // A quarter turn off the target line is already a shank; past it the ball travels back
        // towards the golfer and `carry_m` stops being a distance towards anything. The screen never
        // prints one, so a caller holding one has a parse error rather than a shot.
        return Err(refuse(format!(
            "launch direction must be within a quarter turn of the target line, got {} deg",
            crate::pyfmt::repr(launch.launch_direction_deg)
        )));
    }
    if !(-90.0..=90.0).contains(&launch.spin_axis_deg) {
        // At exactly +/-90 the axis is vertical, the ball spins purely sideways, and that is a
        // flight this model can fly. Past it the axis has tipped over into top spin, refused for the
        // same reason a negative spin rate is: the coefficient table is one-sided.
        return Err(refuse(format!(
            "spin axis must be within a quarter turn of horizontal, got {} deg",
            crate::pyfmt::repr(launch.spin_axis_deg)
        )));
    }
    if step_s <= 0.0 {
        return Err(refuse(format!(
            "step must be positive, got {} s",
            crate::pyfmt::repr(step_s)
        )));
    }

    let speed = launch.ball_speed_mph * MPH_TO_M_S;
    let (direction, axis) = launch_frame(launch);
    let integrand = Integrand {
        model,
        spin0_rad_s: launch.spin_rpm * RPM_TO_RAD_S,
        axis,
    };

    let mut state: State = [
        0.0,
        0.0,
        0.0,
        speed * direction.0,
        speed * direction.1,
        speed * direction.2,
    ];
    let mut t_s = 0.0;
    let mut points = vec![point(&integrand, t_s, &state)];

    loop {
        let stepped = rk4_step(&integrand, t_s, &state, step_s);
        if stepped[1] <= 0.0 {
            let (landed, theta) = solve_landing(&integrand, t_s, &state, step_s);
            points.push(point(&integrand, t_s + theta, &landed));
            break;
        }
        t_s += step_s;
        state = stepped;
        points.push(point(&integrand, t_s, &state));
        assert!(
            t_s <= MAX_FLIGHT_S,
            "{launch:?} did not reach the ground in {MAX_FLIGHT_S} s, which gravity and drag make \
             impossible - the integrator is wrong, not the shot"
        );
    }

    let landing = points[points.len() - 1];
    // The landing point and the landing velocity turned into the frame the ball set off in. One
    // rotation applied twice, so `carry_m`, `curvature_m` and `descent_angle_deg` cannot drift into
    // describing different frames.
    let cos_d = launch.launch_direction_deg.to_radians().cos();
    let sin_d = launch.launch_direction_deg.to_radians().sin();
    Ok(FlightResult {
        launch: *launch,
        carry_m: landing.x_m * cos_d + landing.z_m * sin_d,
        curvature_m: landing.z_m * cos_d - landing.x_m * sin_d,
        landing_offline_m: landing.z_m,
        // The sampled maximum, not an interpolated vertex. `y' = 0` at the apex makes `y` stationary
        // there, so the worst a sample half a step away can miss by is g*(h/2)^2/2 — 0.03 mm at the
        // default step, four orders below anything this number is compared against.
        //
        // `fold` rather than `max_by`: Python's `max` returns the *first* maximum and Rust's
        // `max_by` the last (P4's fourth portability edge). It cannot matter to a value rather than
        // an index, and is written the un-edged way so nobody has to re-derive that.
        apex_m: points.iter().fold(f64::NEG_INFINITY, |a, p| a.max(p.y_m)),
        flight_time_s: landing.t_s,
        descent_angle_deg: (-landing.vy_m_s)
            .atan2(landing.vx_m_s * cos_d + landing.vz_m_s * sin_d)
            .to_degrees(),
        clamped_points: points.iter().filter(|p| p.clamped).count(),
        spin_ratio_min: points
            .iter()
            .fold(f64::INFINITY, |a, p| a.min(p.spin_ratio)),
        spin_ratio_max: points
            .iter()
            .fold(f64::NEG_INFINITY, |a, p| a.max(p.spin_ratio)),
        points,
    })
}

fn refuse(message: String) -> UnflyableLaunch {
    UnflyableLaunch { message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::flight_model::load_flight_model;

    /// M15 P4's first validation shot, whose whole path is committed as
    /// `spec/vectors/stages/corpus/2026-08-10-1`'s `flight` stage. Used here for the properties the
    /// gate cannot state as a number.
    fn reference() -> LaunchConditions {
        LaunchConditions {
            ball_speed_mph: 90.7,
            launch_angle_deg: 20.9,
            spin_rpm: 5991.0,
            launch_direction_deg: -5.3,
            spin_axis_deg: 2.5,
        }
    }

    fn fly(launch: &LaunchConditions) -> FlightResult {
        simulate_flight(launch, load_flight_model(), DEFAULT_STEP_S).expect("a flyable launch")
    }

    /// Every refusal message, byte for byte. **Not one of the twenty-one vectors reaches one**, so
    /// these are the only oracle for the sentences `flight_measure.fly_shot` will put in a
    /// `NO_LAUNCH_CONDITIONS` detail in P8b — and `pyfmt::repr` is what makes `90.0` print as
    /// `90.0` rather than as `90`.
    #[test]
    fn the_six_refusals_read_exactly_as_pythons_value_errors() {
        let model = load_flight_model();
        let cases: [(LaunchConditions, f64, &str); 6] = [
            (
                LaunchConditions::planar(0.0, 20.0, 3000.0),
                DEFAULT_STEP_S,
                "ball speed must be positive to fly, got 0.0 mph",
            ),
            (
                LaunchConditions::planar(90.0, 0.0, 3000.0),
                DEFAULT_STEP_S,
                "launch angle must be above the horizontal and below the vertical to leave the \
                 ground, got 0.0 deg",
            ),
            (
                LaunchConditions::planar(90.0, 20.0, -1.0),
                DEFAULT_STEP_S,
                "spin rate must not be negative, got -1.0 rpm",
            ),
            (
                LaunchConditions {
                    launch_direction_deg: 90.0,
                    ..LaunchConditions::planar(90.0, 20.0, 3000.0)
                },
                DEFAULT_STEP_S,
                "launch direction must be within a quarter turn of the target line, got 90.0 deg",
            ),
            (
                LaunchConditions {
                    spin_axis_deg: -90.5,
                    ..LaunchConditions::planar(90.0, 20.0, 3000.0)
                },
                DEFAULT_STEP_S,
                "spin axis must be within a quarter turn of horizontal, got -90.5 deg",
            ),
            (
                LaunchConditions::planar(90.0, 20.0, 3000.0),
                0.0,
                "step must be positive, got 0.0 s",
            ),
        ];
        for (launch, step, want) in cases {
            let got = simulate_flight(&launch, model, step).expect_err("refused");
            assert_eq!(got.message, want);
        }
    }

    /// The two boundaries the Python comments single out as *flyable*: a vertical spin axis is a
    /// ball spinning purely sideways and this model can fly it, and 89.999° of launch is still a
    /// flight.
    #[test]
    fn the_two_admitted_boundaries_fly_rather_than_refuse() {
        let model = load_flight_model();
        for axis in [-90.0, 90.0] {
            let launch = LaunchConditions {
                spin_axis_deg: axis,
                ..LaunchConditions::planar(90.0, 20.0, 3000.0)
            };
            assert!(
                simulate_flight(&launch, model, DEFAULT_STEP_S).is_ok(),
                "{axis}"
            );
        }
        let steep = LaunchConditions::planar(90.0, 89.999, 3000.0);
        assert!(simulate_flight(&steep, model, DEFAULT_STEP_S).is_ok());
    }

    /// A zero spin axis means **no curve** — the property that separates a push from a slice, and
    /// the one the gate cannot see because no corpus flight carries a start line without an axis.
    ///
    /// **`FlightResult.curvature_m`'s docstring says "exactly zero whenever the spin axis is zero,
    /// however far offline the shot started", and that is overstated.** It is exactly zero only when
    /// the launch direction is zero too; with a start line on it, `z*cos - x*sin` leaves the
    /// rounding residue of a rotation — **-1.9539925233402755e-14 m** on this shot, in *both*
    /// languages to the bit. So the claim holds to 2e-14 m rather than to `0.0`, and this test is
    /// where that is written down: the guarantee a caller can rely on is the planar case below.
    #[test]
    fn a_zero_spin_axis_leaves_no_curve_but_a_rotation_leaves_a_residue() {
        let pushed = LaunchConditions {
            launch_direction_deg: 8.0,
            ..LaunchConditions::planar(90.7, 20.9, 5991.0)
        };
        let flown = fly(&pushed);
        assert_eq!(flown.curvature_m, -1.9539925233402755e-14);
        // The whole path, not just the landing: `z` is a pure rotation of `x` here.
        assert!(flown.points.iter().all(|p| p.z_m != 0.0 || p.x_m == 0.0));

        // A planar launch stays on the `z = 0` line to the bit, which is what makes every number
        // M15 P3 measured survive the third dimension — and it is the only case where the
        // docstring's `0.0` is literal.
        let planar = fly(&LaunchConditions::planar(90.7, 20.9, 5991.0));
        assert!(planar.points.iter().all(|p| p.z_m == 0.0));
        assert_eq!(planar.curvature_m, 0.0);
        assert_eq!(planar.landing_offline_m, 0.0);
    }

    /// Carry is measured along the **launch azimuth**, so turning the whole flight about the
    /// vertical cannot change it. `landing_offline_m` is the number that moves.
    ///
    /// This is the identity `flight_measure` leans on in P8b when it declines to record
    /// `flight_landing_offline_yds` for an undrawn curve — `carry * sin(direction)` is
    /// `shot_measure.measure_start_line_offline` to the last bits.
    #[test]
    fn a_rotation_about_the_vertical_moves_the_offline_and_not_the_carry() {
        let straight = fly(&LaunchConditions::planar(90.7, 20.9, 5991.0));
        let pushed = fly(&LaunchConditions {
            launch_direction_deg: 6.0,
            ..LaunchConditions::planar(90.7, 20.9, 5991.0)
        });
        assert!(
            (pushed.carry_m - straight.carry_m).abs() < 1e-9,
            "{} against {}",
            pushed.carry_m,
            straight.carry_m
        );
        let want = pushed.carry_m * 6.0f64.to_radians().sin();
        assert!(
            (pushed.landing_offline_m - want).abs() < 1e-9,
            "{} against {want}",
            pushed.landing_offline_m
        );
    }

    /// Carry is **even in the spin axis**: a right-handed golfer's fade and a left-handed golfer's
    /// fly the same distance. It is what makes M15 P4's gate indifferent to the sign this module
    /// deliberately does not own.
    #[test]
    fn carry_is_even_in_the_spin_axis_and_curvature_is_odd() {
        let right = fly(&LaunchConditions {
            spin_axis_deg: 9.3,
            ..LaunchConditions::planar(90.5, 23.5, 8100.0)
        });
        let left = fly(&LaunchConditions {
            spin_axis_deg: -9.3,
            ..LaunchConditions::planar(90.5, 23.5, 8100.0)
        });
        assert_eq!(right.carry_m, left.carry_m);
        assert_eq!(right.curvature_m, -left.curvature_m);
        assert!(right.curvature_m > 0.0, "a positive axis curves right");
    }

    /// **The landing is solved inside the final step, not rounded to it.** Without
    /// [`solve_landing`] the last point sits below ground by up to a step's worth of descent — 9.5 cm
    /// of carry on this shot — and that is the size of error a tolerance absorbs in silence.
    #[test]
    fn the_landing_is_solved_to_the_ground_rather_than_stepped_past_it() {
        let flown = fly(&reference());
        let landing = flown.landing();
        assert!(
            landing.y_m.abs() <= LANDING_TOLERANCE_M,
            "landed at y = {}",
            landing.y_m
        );
        // It fell inside the step rather than on its boundary: the last two points are closer
        // together than `DEFAULT_STEP_S`.
        let previous = flown.points[flown.points.len() - 2];
        let theta = landing.t_s - previous.t_s;
        assert!(
            theta > 0.0 && theta < DEFAULT_STEP_S,
            "the landing step was {theta} s"
        );
        // `flight_time_s` is the solved crossing, not the step boundary before it.
        assert_eq!(flown.flight_time_s, landing.t_s);
    }

    /// **The step is not where this model's error lives.** Sixteen-fold refinement moves the carry
    /// by under 1e-4 yd, which is what makes `DEFAULT_STEP_S` a converged choice rather than a
    /// tuned one — and it is the claim ADR-027 §Decision 1 rests the whole tolerance argument on.
    #[test]
    fn the_carry_is_converged_at_the_default_step() {
        let model = load_flight_model();
        let launch = reference();
        let coarse = simulate_flight(&launch, model, 0.02).unwrap().carry_yds();
        let default = simulate_flight(&launch, model, DEFAULT_STEP_S)
            .unwrap()
            .carry_yds();
        let fine = simulate_flight(&launch, model, 0.00125)
            .unwrap()
            .carry_yds();
        assert!(
            (coarse - default).abs() < 1e-4,
            "{coarse} against {default}"
        );
        assert!((fine - default).abs() < 1e-4, "{fine} against {default}");
    }

    /// Every iron in this corpus flies its whole path on a held end row, and the agreement with
    /// HD Golf must never be quoted without that. ADR-027's 2026-09-05 addendum.
    #[test]
    fn the_reference_shot_is_clamped_for_its_entire_path() {
        let flown = fly(&reference());
        assert!(flown.fully_clamped());
        assert_eq!(flown.clamped_points, flown.points.len());
        // `S` climbs through the flight — the ball sheds speed faster than it sheds spin — so the
        // launch point is the minimum and it is already above the table's 0.284 ceiling.
        assert_eq!(flown.spin_ratio_min, flown.points[0].spin_ratio);
        assert!(flown.spin_ratio_min > load_flight_model().coefficients.spin_ratio_max());
        assert!(flown.spin_ratio_max > flown.spin_ratio_min);
    }

    /// M15 P4's gate, re-derived here rather than ported: the two shots, the carry HD Golf printed
    /// beside them, and the **±2.59% spread** that ADR-027's 2026-09-05d addendum pins.
    ///
    /// `gate_comparisons` itself is not ported (see the module doc), but the numbers it reports are
    /// this port's own answers and they are worth one assertion: they are how a re-sourced
    /// coefficient table announces itself.
    #[test]
    fn both_validation_shots_agree_with_hd_golf_to_within_the_pinned_spread() {
        let shots = [
            (
                LaunchConditions {
                    ball_speed_mph: 90.7,
                    launch_angle_deg: 20.9,
                    spin_rpm: 5991.0,
                    launch_direction_deg: -5.3,
                    spin_axis_deg: 2.5,
                },
                125.6,
            ),
            (
                LaunchConditions {
                    ball_speed_mph: 90.5,
                    launch_angle_deg: 23.5,
                    spin_rpm: 8100.0,
                    launch_direction_deg: 4.0,
                    spin_axis_deg: 9.3,
                },
                121.0,
            ),
        ];
        let mut errors = Vec::new();
        for (launch, printed) in shots {
            let flown = fly(&launch);
            assert!(flown.fully_clamped(), "quote the agreement with the clamp");
            errors.push((flown.carry_yds() - printed) / printed);
        }
        assert!(
            errors[0] < 0.0 && errors[1] > 0.0,
            "a spread, not an offset"
        );
        assert!(errors.iter().all(|e| e.abs() <= 0.0259), "{errors:?}");
        // **And the model ranks the two shots the wrong way round**, which the percentage hides:
        // HD Golf has the lower-spin shot flying 4.6 yd further and this model has it flying
        // shorter. Above the clamp spin has no route into the answer at all.
        let low_spin = fly(&shots[0].0).carry_yds();
        let high_spin = fly(&shots[1].0).carry_yds();
        assert!(low_spin < high_spin, "{low_spin} against {high_spin}");
    }

    /// Thin air is not simply longer: it cuts drag and lift by the same density, so the ball flies
    /// further and arrives **flatter**. The altitude what-if is not ported, so this drives the one
    /// input it would have changed by hand.
    #[test]
    fn thinner_air_flies_further_and_flatter() {
        let model = load_flight_model();
        let mut denver = model.clone();
        // ISO 2533 at 1609 m, the figure `scripts/simulate_flight.py --altitude` reaches through
        // `AtmosphereProfile`. Driven directly because that profile stayed in Python.
        denver.atmosphere.density_kg_m3 = 1.0468;
        let sea = fly(&reference());
        let high = simulate_flight(&reference(), &denver, DEFAULT_STEP_S).unwrap();
        assert!(high.carry_yds() > sea.carry_yds());
        assert!(high.apex_m < sea.apex_m);
        assert!(high.flight_time_s < sea.flight_time_s);
        assert!(high.descent_angle_deg < sea.descent_angle_deg);
        assert!(high.curvature_yds().abs() < sea.curvature_yds().abs());
    }

    /// The unit conversions, as constants rather than as arithmetic in a sentence. `M_TO_YARDS` is a
    /// reciprocal that is then multiplied, and the test says so because `x / 0.9144` is a different
    /// last bit.
    #[test]
    fn the_yard_conversion_is_a_multiply_by_a_reciprocal() {
        let flown = fly(&reference());
        assert_eq!(flown.carry_yds(), flown.carry_m * (1.0 / 0.9144));
        assert_eq!(flown.apex_yds(), flown.apex_m * (1.0 / 0.9144));
        assert_eq!(
            flown.landing_offline_yds(),
            flown.landing_offline_m * M_TO_YARDS
        );
        // The launch spin is the first point's, converted back out of rad/s.
        assert!((flown.points[0].spin_rpm - 5991.0).abs() < 1e-9);
    }

    /// A zero third leg does not move the norm: on a planar flight `hypot(vx, vy, 0)` is
    /// `hypot(vx, vy)`, which is the property that lets M15 P3's planar numbers survive the third
    /// dimension.
    ///
    /// **Against [`crate::pyfmt::hypot`] and not against [`f64::hypot`]**, because those two are one
    /// ulp apart here as well — `40.48099063694415` against `40.48099063694414`. The two-argument
    /// call is the same `vector_norm`, so the invariant is about the zero and not about the language.
    #[test]
    fn a_zero_third_leg_does_not_move_the_speed() {
        let flown = fly(&LaunchConditions::planar(90.7, 20.9, 5991.0));
        for p in &flown.points {
            assert_eq!(p.vz_m_s, 0.0);
            assert_eq!(p.speed_m_s(), crate::pyfmt::hypot(&[p.vx_m_s, p.vy_m_s]));
        }
    }
}
