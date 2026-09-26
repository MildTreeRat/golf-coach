//! Spin solved backwards out of a printed carry — the inverse of [`crate::flight`]. [M22 P8]
//!
//! The port of `analysis/spin_solve.py`. Eleven of the thirteen shots on disk record no spin at all:
//! the bay's screen profile prints `Spin:` with nothing under it, and only the two 2026-08-10
//! reference shots carry a number
//! ([ADR-027](../../../docs/decisions/027-ball-flight-simulation.md) §Context 2). This module is
//! what stands in — given ball speed, launch angle and the carry the simulator printed beside them,
//! it looks for the spin rate that makes the integrator agree.
//!
//! **What comes back is not a measurement of spin, and the strength of that sentence is the whole
//! point of the module.** ADR-027 §Decision 3 wrote it down as the milestone's weakest claim before
//! any of this ran; the 2026-09-05b addendum then ran the only honest test available — the two shots
//! where a real spin *is* on disk — and found one refusal and one answer 47% low. What survives is
//! what §Decision 3 said the number was for: **drawing a path whose carry matches the measured
//! carry**. It is a path-drawing device. It must never be shown to a golfer as their spin rate, and
//! P8b owns the labelling that keeps that true.
//!
//! # Why this is a separate module from the integrator
//!
//! [`crate::flight`] is the forward model and this is the inverse problem over it, called as a black
//! box — it holds no physics, no constant of its own, and no second copy of anything in
//! [`crate::benchmarks::flight_model`]. Keeping them apart is also what keeps the forward model's own
//! property obvious: every number here re-flies rather than remembering.
//!
//! # The shape of the curve being inverted
//!
//! Carry against launch spin, at fixed ball speed and launch angle, is **five segments**:
//!
//! ```text
//! low plateau ──▁▂▃ rising ▄▅ peak ▅▄ falling ▃▂▁── high plateau
//! (flat)                                            (flat)
//! ```
//!
//! Both plateaus are the coefficient table being clamped, and they are flat because `w` reaches the
//! flight only through `S = wR/v`, and `S` only through `Cl` and `Cd`. Hold those and spin has left
//! the problem entirely — carry is *bit-identical* at 5,200 and 30,000 rpm on the reference shot.
//! The high plateau starts where the whole flight launches at or above the table's last row; the low
//! one ends where the flight first climbs into its first row (`S` rises through a flight, so a
//! low-spin ball can start under the table and enter it later).
//!
//! That geometry gives the seven cases in [`SpinSolveCase`], which are §Decision 4's five with two
//! of them split by what the 2026-09-05c addendum measured.
//!
//! # What this module deliberately does not do
//!
//! It does not pick a branch — §Decision 3 gives that job to the loft prior, which is P8b's, along
//! with the refusal reasons from `contracts::unscored`. It does not read anything off disk. And it
//! never invents a representative from an infinite set: on either plateau the honest answer is the
//! case itself, not a spin chosen from a flat interval (`docs/CODE_STANDARDS.md` R7).
//!
//! # What the committed vectors gate, and what they do not
//!
//! `spec/vectors/stages/`'s `flight` stage records `resolved.spin.solution` on the **eleven** corpus
//! shots whose spin was not printed, and that is a whole [`SpinSolution`] each: the case, the two
//! branch spins, the target carry, and all five numbers of the [`CarryWindow`] underneath.
//! `crates/analysis/tests/flight.rs` runs them.
//!
//! They land on **three of the seven cases** — `above_peak`, `between_plateaus` and `two_branches`.
//! The other four, `at_peak`, `on_high_plateau`, `on_low_plateau` and `below_floor`, reach no
//! committed answer and are gated by the unit tests at the foot of this file. `below_floor` is the
//! one to know about: it is the case §Decision 4 did not know existed, because it assumed carry
//! falls without bound past the peak when in fact it *floors*.

use crate::benchmarks::flight_model::FlightModel;
use crate::flight::{simulate_flight, FlightResult, LaunchConditions, UnflyableLaunch};

/// The spin the calibration flight is flown at, purely to read a launch spin ratio off it. Any
/// positive value works — `S = wR/v` is linear in `w` at a fixed ball speed, so one flight fixes the
/// whole scale — and this one is a real 7-iron number so a failure prints something plausible.
const CALIBRATION_RPM: f64 = 5000.0;

/// How close two spins have to be before the search stops splitting them. Half an rpm is four orders
/// below the 47% error the solve was measured to make on the one shot where the truth is known, so
/// the iteration count here is not what limits the answer.
const ROOT_TOLERANCE_RPM: f64 = 0.5;
/// Bisection halves the bracket each pass, so this covers a 6,500 rpm range down to the tolerance
/// above with room to spare. Exhausting it returns the midpoint rather than raising: the bracket is
/// already known to hold a root, so the last iterate is a worse answer and not a wrong one.
const ROOT_ITERATIONS: usize = 24;

/// The peak's *location* barely matters — carry is stationary there, so 2 rpm of slop moves the peak
/// carry by well under a thousandth of a yard — but its *value* decides whether a printed carry is
/// reachable at all, which is why the search is run to a tolerance rather than a fixed pass count.
const PEAK_TOLERANCE_RPM: f64 = 2.0;
const PEAK_ITERATIONS: usize = 40;

/// Half of the last digit HD Golf prints. The carry tiles round to 1 dp, so a target within this of
/// a plateau value is *indistinguishable from it at the precision it was read*, and treating it as a
/// solvable point would be inventing a spin out of the rounding.
const PRINTED_PRECISION_YDS: f64 = 0.05;

/// 1/phi, the ratio that lets each golden-section pass reuse one of the previous pass's two
/// evaluations. Spelled to the digit the Python spells it, because the search path — and so the
/// flights taken — depends on it exactly.
const GOLDEN_RATIO: f64 = 0.618_033_988_749_894_9;

/// [`LaunchConditions`] with the one field this module is looking for left out.
///
/// A separate shape rather than a `LaunchConditions` whose `spin_rpm` is ignored, because an ignored
/// field is a trap: every caller here holds launch conditions that genuinely have no spin on them,
/// and a solve that quietly overwrote a spin the caller *did* pass would be impossible to see at the
/// call site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnspunLaunch {
    pub ball_speed_mph: f64,
    pub launch_angle_deg: f64,
    /// Both lateral fields carry [`LaunchConditions`]' signs and defaults exactly — see it for what
    /// `+` means and for why the spin axis is not the contract's `spin_axis` sign.
    pub launch_direction_deg: f64,
    pub spin_axis_deg: f64,
}

impl UnspunLaunch {
    /// Drop the spin off a full set of launch conditions.
    ///
    /// Used to re-solve the two validation shots as if their spin had never been printed, which is
    /// the only honest test ADR-027 §Decision 3 can be given.
    pub fn from_launch(launch: &LaunchConditions) -> Self {
        Self {
            ball_speed_mph: launch.ball_speed_mph,
            launch_angle_deg: launch.launch_angle_deg,
            launch_direction_deg: launch.launch_direction_deg,
            spin_axis_deg: launch.spin_axis_deg,
        }
    }

    /// Put a candidate spin back on, giving something the integrator can fly.
    pub fn at_spin(&self, spin_rpm: f64) -> LaunchConditions {
        LaunchConditions {
            ball_speed_mph: self.ball_speed_mph,
            launch_angle_deg: self.launch_angle_deg,
            spin_rpm,
            launch_direction_deg: self.launch_direction_deg,
            spin_axis_deg: self.spin_axis_deg,
        }
    }
}

/// Every carry these launch conditions can produce, and the spins at the edges of that set.
///
/// This is the module's real product. The solve returns a spin only in three of its seven cases, but
/// the window is measured every time and says *why* — a printed carry 2 yd above the peak and one
/// sitting exactly on the high plateau both come back without a number, and they are entirely
/// different findings about the shot.
///
/// Read `low_plateau_yds` and `high_plateau_yds` as values of the flat segments, and the two rpm
/// fields as the edges of the flat intervals that produce them: every spin at or below
/// `low_plateau_max_rpm` flies `low_plateau_yds`, and every spin at or above `high_plateau_min_rpm`
/// flies `high_plateau_yds`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarryWindow {
    pub launch: UnspunLaunch,
    /// What the ball flies when the spin is too low for the coefficient table to see it at all. Not
    /// zero-lift: the first row is *held*, so this is a real flight with `Cl = 0.141` on it.
    pub low_plateau_yds: f64,
    /// The most spin that still flies `low_plateau_yds` — the point where the flight first climbs
    /// into the table's measured rows on its way down.
    pub low_plateau_max_rpm: f64,
    pub peak_yds: f64,
    pub peak_rpm: f64,
    /// What the ball flies once every step of the flight is above the table's last row. Every iron
    /// in this repo's corpus is here for its whole path.
    pub high_plateau_yds: f64,
    /// The least spin that flies `high_plateau_yds` — analytic, not searched: the launch spin ratio
    /// is the smallest one in a flight, so it alone decides whether the whole path is clamped.
    /// **This is also the cap on every inferred spin**, and P8b has to report it: an answer there
    /// means the carry stopped responding, not that the golfer spun the ball this fast.
    pub high_plateau_min_rpm: f64,
}

impl CarryWindow {
    /// The shortest carry these conditions can fly, whichever plateau it falls on.
    ///
    /// The 2026-09-05c addendum's correction, as code: the bottom of the range is the *low*-spin
    /// clamp on every shot measured so far, and not — as the addendum before it assumed — the
    /// high-spin one that the falling branch runs into.
    pub fn floor_yds(&self) -> f64 {
        self.low_plateau_yds.min(self.high_plateau_yds)
    }

    /// Floor to peak: the room a printed carry has to be wrong in before the case changes.
    pub fn width_yds(&self) -> f64 {
        self.peak_yds - self.floor_yds()
    }
}

/// Which of the curve's segments the target carry landed on, and how many spins produce it.
///
/// ADR-027 §Decision 4's five cases, with two of them split by what M15 P4 measured — the plateau
/// case is two different plateaus with different meanings, and the two-solution case has a band
/// between the plateaus where only one branch reaches down that far.
///
/// A `StrEnum` in the Python because P8b stores this beside the answer and the flight route serves
/// it: the case is the part of the result that stays true even when there is no number. Here that is
/// [`SpinSolveCase::as_str`], and `docs/CONFORMANCE.md` §3 compares it exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinSolveCase {
    /// Two spins fly this carry, one either side of the peak. ADR-027 §Decision 3's loft prior is
    /// what chooses; with no loft on record there is no branch rule and no answer.
    TwoBranches,
    /// Exactly one spin flies it, because the target sits between the two plateau values and the
    /// falling branch never comes down this far. **Unique is not automatically the good news it
    /// looks like** — on the reference shot the band runs 1,129-1,538 rpm, which is not a spin a
    /// 7 iron produces. But it is not always implausible either: on `2026-08-23-2`, launched at
    /// 3.4°, it runs 1,323-4,762 rpm and the answer lands at 2,307. So the refusal P8b owes this
    /// case has to be argued from where the answer sits, not from the case's name.
    BetweenPlateaus,
    /// The target is at the peak, to within the precision it was printed at — one spin, on a knife
    /// edge, and the answer is the peak itself rather than a branch.
    AtPeak,
    /// Infinitely many: every spin at or above `high_plateau_min_rpm` flies exactly this carry. The
    /// common case for a well-struck iron, and the reason a carry cannot recover an iron's spin.
    OnHighPlateau,
    /// Infinitely many at the other end: every spin from zero up to `low_plateau_max_rpm`. Nothing
    /// in that interval is a plausible struck-iron spin, which is the difference between this and
    /// the case above.
    OnLowPlateau,
    /// No spin flies the ball this far. A finding rather than a failure — but read it beside the
    /// model's own ~2.6% disagreement with HD Golf before calling it an OCR fault, because seven of
    /// the eleven printed carries clear the peak by less than that.
    AbovePeak,
    /// No spin flies the ball this *short* — the opposite refusal, and the one ADR-027 §Decision 4
    /// did not know existed, because it assumed carry falls without bound past the peak. It floors
    /// instead. This is what happens to `2026-08-10-2`, whose printed 121.0 yd is below anything
    /// those launch conditions can fly.
    BelowFloor,
}

impl SpinSolveCase {
    /// The `StrEnum` value, which is what reaches a payload and what §3 compares.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TwoBranches => "two_branches",
            Self::BetweenPlateaus => "between_plateaus",
            Self::AtPeak => "at_peak",
            Self::OnHighPlateau => "on_high_plateau",
            Self::OnLowPlateau => "on_low_plateau",
            Self::AbovePeak => "above_peak",
            Self::BelowFloor => "below_floor",
        }
    }
}

/// What the carry could and could not say about the spin.
///
/// The case is always present; the two spins are present only where they exist. Nothing here is
/// filled in with a plausible value when it is absent (`docs/CODE_STANDARDS.md` R7) — in particular
/// neither plateau case returns a representative spin, because choosing one out of a flat interval
/// is exactly the invention this module is written not to make.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinSolution {
    pub case: SpinSolveCase,
    pub target_carry_yds: f64,
    pub window: CarryWindow,
    /// The low-spin answer, below the peak. Present for `TwoBranches`, for `BetweenPlateaus` when
    /// the low plateau is the lower of the two, and for `AtPeak`, where both branches meet.
    pub rising_rpm: Option<f64>,
    /// The high-spin answer, above the peak — the branch a struck iron is actually on, and the one
    /// capped by `window.high_plateau_min_rpm`.
    pub falling_rpm: Option<f64>,
}

impl SpinSolution {
    /// The distinct spins that fly the target carry.
    ///
    /// Empty where none does — and equally where infinitely many do.
    ///
    /// `AtPeak` sets both branches to the same value on purpose, so it comes back as the one answer
    /// it is rather than as two.
    pub fn candidates(&self) -> Vec<f64> {
        let found: Vec<f64> = [self.rising_rpm, self.falling_rpm]
            .into_iter()
            .flatten()
            .collect();
        if found.len() == 2 && found[0] == found[1] {
            return vec![found[0]];
        }
        found
    }
}

/// Measure the whole carry-against-spin curve for one set of launch conditions.
///
/// Costs about forty flights, which is why it is a separate function: a caller that wants both the
/// window and a solve against it should measure it once and hand it back to
/// [`solve_spin_from_carry`].
///
/// Refuses whatever [`simulate_flight`] refuses for launch conditions there is no flight to
/// integrate from — a launch angle at or below the horizontal is the caller's check to have made,
/// and [`crate::flight`] says why that is a raise and not a golfer-facing refusal.
pub fn carry_window(
    launch: &UnspunLaunch,
    model: &FlightModel,
    step_s: f64,
) -> Result<CarryWindow, UnflyableLaunch> {
    let fly = |spin_rpm: f64| -> Result<FlightResult, UnflyableLaunch> {
        simulate_flight(&launch.at_spin(spin_rpm), model, step_s)
    };
    let carry = |spin_rpm: f64| -> Result<f64, UnflyableLaunch> { Ok(fly(spin_rpm)?.carry_yds()) };

    // The launch spin ratio is `wR/v` with the launch speed in it, so at a fixed ball speed it is
    // simply proportional to the spin. One flight therefore fixes the scale exactly, and no unit
    // conversion is re-typed here from `flight.rs` — the constants stay in one place (R4).
    let calibration = fly(CALIBRATION_RPM)?;
    let ratio_per_rpm = calibration.points[0].spin_ratio / CALIBRATION_RPM;
    let high_plateau_min_rpm = model.coefficients.spin_ratio_max() / ratio_per_rpm;

    let low_plateau_yds = carry(0.0)?;
    let high_plateau_yds = carry(high_plateau_min_rpm)?;
    let low_plateau_max_rpm = low_plateau_shoulder(
        &fly,
        model.coefficients.spin_ratio_min(),
        high_plateau_min_rpm,
    )?;

    let (mut peak_rpm, mut peak_yds) = peak(&carry, low_plateau_max_rpm, high_plateau_min_rpm)?;
    // The interior search assumes one hump between the shoulders; the plateau values are known
    // exactly, so taking the best of the three costs nothing and means launch conditions whose carry
    // only ever rises come back with a peak at the end rather than with a golden-section artefact
    // somewhere in the middle. **This is not defensive**: `2026-08-23-2` is on disk and does exactly
    // that, and its peak is its high plateau to the last bit — so it has no falling branch at all
    // and no two-solution band.
    if low_plateau_yds >= peak_yds {
        (peak_rpm, peak_yds) = (low_plateau_max_rpm, low_plateau_yds);
    }
    if high_plateau_yds >= peak_yds {
        (peak_rpm, peak_yds) = (high_plateau_min_rpm, high_plateau_yds);
    }

    Ok(CarryWindow {
        launch: *launch,
        low_plateau_yds,
        low_plateau_max_rpm,
        peak_yds,
        peak_rpm,
        high_plateau_yds,
        high_plateau_min_rpm,
    })
}

/// Find the spin — or the spins, or neither — that fly `target_carry_yds` from these conditions.
///
/// The target is the carry the launch monitor printed, which is the output of *its* flight model.
/// Solving against it fits this model to that one rather than to reality, and ADR-027 §Decision 3 is
/// on the record about it: what comes back is "the spin our integrator needs in order to agree with
/// the simulator" and is not a measurement of spin.
///
/// `window` is Python's optional keyword, and passing `None` measures one — which is the path
/// `flight_infer` takes on every shot.
pub fn solve_spin_from_carry(
    launch: &UnspunLaunch,
    target_carry_yds: f64,
    model: &FlightModel,
    step_s: f64,
    window: Option<CarryWindow>,
) -> Result<SpinSolution, UnflyableLaunch> {
    let window = match window {
        Some(window) => window,
        None => carry_window(launch, model, step_s)?,
    };
    let carry = |spin_rpm: f64| -> Result<f64, UnflyableLaunch> {
        Ok(simulate_flight(&launch.at_spin(spin_rpm), model, step_s)?.carry_yds())
    };

    let solution = |case: SpinSolveCase, rising: Option<f64>, falling: Option<f64>| SpinSolution {
        case,
        target_carry_yds,
        window,
        rising_rpm: rising,
        falling_rpm: falling,
    };

    if target_carry_yds > window.peak_yds + PRINTED_PRECISION_YDS {
        return Ok(solution(SpinSolveCase::AbovePeak, None, None));
    }
    if target_carry_yds > window.peak_yds {
        // Above the peak but by less than the printed carry's own last digit. The two branches have
        // collapsed onto each other here, so the peak is the answer rather than either of them.
        return Ok(solution(
            SpinSolveCase::AtPeak,
            Some(window.peak_rpm),
            Some(window.peak_rpm),
        ));
    }
    if (target_carry_yds - window.low_plateau_yds).abs() <= PRINTED_PRECISION_YDS {
        return Ok(solution(SpinSolveCase::OnLowPlateau, None, None));
    }
    if (target_carry_yds - window.high_plateau_yds).abs() <= PRINTED_PRECISION_YDS {
        return Ok(solution(SpinSolveCase::OnHighPlateau, None, None));
    }
    if target_carry_yds < window.floor_yds() {
        return Ok(solution(SpinSolveCase::BelowFloor, None, None));
    }

    let rising_rpm = root(
        &carry,
        window.low_plateau_max_rpm,
        window.peak_rpm,
        target_carry_yds,
    )?;
    let falling_rpm = root(
        &carry,
        window.peak_rpm,
        window.high_plateau_min_rpm,
        target_carry_yds,
    )?;
    let found = [rising_rpm, falling_rpm].into_iter().flatten().count();
    assert!(
        found > 0,
        // The target is inside the window the same window said it was inside, and neither branch
        // brackets it. That is the curve not having the shape measured above, which is a wiring bug
        // in this module rather than anything about the shot (R8) — Python's `RuntimeError`, which
        // nothing catches, so a panic is the same reachability.
        "{target_carry_yds} yd sits inside {}-{} yd for {launch:?}, but neither branch brackets it \
         - the carry curve is not the shape carry_window measured",
        window.floor_yds(),
        window.peak_yds
    );
    let case = if found == 2 {
        SpinSolveCase::TwoBranches
    } else {
        SpinSolveCase::BetweenPlateaus
    };
    Ok(solution(case, rising_rpm, falling_rpm))
}

/// The most spin whose flight never once climbs into the table's measured rows.
///
/// Bisected on the *mechanism* rather than on the carry. The two agree — the plateau is flat because
/// the coefficients are held — but `spin_ratio_max` rises with spin by construction, while a carry
/// that happens to be flat over some other interval would fool an equality test. `S` is largest at
/// landing, so the whole flight is under the table exactly when the landing point is.
fn low_plateau_shoulder(
    fly: &dyn Fn(f64) -> Result<FlightResult, UnflyableLaunch>,
    table_spin_ratio_min: f64,
    high_plateau_min_rpm: f64,
) -> Result<f64, UnflyableLaunch> {
    let (mut low, mut high) = (0.0, high_plateau_min_rpm);
    for _ in 0..ROOT_ITERATIONS {
        if high - low <= ROOT_TOLERANCE_RPM {
            break;
        }
        let mid = 0.5 * (low + high);
        if fly(mid)?.spin_ratio_max <= table_spin_ratio_min {
            low = mid;
        } else {
            high = mid;
        }
    }
    Ok(low)
}

/// Golden-section search for the top of the hump between the two plateaus.
///
/// Golden section rather than a derivative or a sampled sweep: carry has no closed form here, each
/// evaluation is a whole flight, and a sweep fine enough to place the peak to a couple of rpm would
/// cost thousands of them.
///
/// It assumes one hump between the shoulders, which is true of every normally-launched shot on disk
/// and **false of at least one real one** — `2026-08-23-2`, launched at 3.4°, rises all the way to
/// the high plateau and never falls. Golden section walks to that edge rather than inventing an
/// interior maximum, and [`carry_window`] then takes the best of the three known points, so the
/// caller and not this helper is where that case is closed.
fn peak(
    carry: &dyn Fn(f64) -> Result<f64, UnflyableLaunch>,
    low_rpm: f64,
    high_rpm: f64,
) -> Result<(f64, f64), UnflyableLaunch> {
    let (mut a, mut b) = (low_rpm, high_rpm);
    let mut c = b - GOLDEN_RATIO * (b - a);
    let mut d = a + GOLDEN_RATIO * (b - a);
    let (mut fc, mut fd) = (carry(c)?, carry(d)?);
    for _ in 0..PEAK_ITERATIONS {
        if b - a <= PEAK_TOLERANCE_RPM {
            break;
        }
        if fc > fd {
            (b, d, fd) = (d, c, fc);
            c = b - GOLDEN_RATIO * (b - a);
            fc = carry(c)?;
        } else {
            (a, c, fc) = (c, d, fd);
            d = a + GOLDEN_RATIO * (b - a);
            fd = carry(d)?;
        }
    }
    let peak_rpm = 0.5 * (a + b);
    Ok((peak_rpm, carry(peak_rpm)?))
}

/// Bisect for the spin that flies `target_yds`, on a branch known to be monotone.
///
/// Orientation-free — it reads the sign at each end rather than being told which way the branch runs
/// — so the same helper serves the rising and the falling branch, and neither call site has a flag on
/// it saying which (`docs/CODE_STANDARDS.md` R14).
///
/// Returns `None` when the bracket does not straddle the target. That is the ordinary way a branch
/// reports that it never reaches the carry being asked for, and it is what separates
/// `BetweenPlateaus` from `TwoBranches`.
fn root(
    carry: &dyn Fn(f64) -> Result<f64, UnflyableLaunch>,
    a_rpm: f64,
    b_rpm: f64,
    target_yds: f64,
) -> Result<Option<f64>, UnflyableLaunch> {
    let (mut a, mut b) = (a_rpm, b_rpm);
    let (mut fa, mut fb) = (carry(a)? - target_yds, carry(b)? - target_yds);
    if fa == 0.0 {
        return Ok(Some(a));
    }
    if fb == 0.0 {
        return Ok(Some(b));
    }
    if (fa > 0.0) == (fb > 0.0) {
        return Ok(None);
    }
    for _ in 0..ROOT_ITERATIONS {
        if b - a <= ROOT_TOLERANCE_RPM {
            break;
        }
        let mid = 0.5 * (a + b);
        let fm = carry(mid)? - target_yds;
        if (fm > 0.0) == (fa > 0.0) {
            (a, fa) = (mid, fm);
        } else {
            (b, fb) = (mid, fm);
        }
    }
    let _ = fb;
    Ok(Some(0.5 * (a + b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::flight_model::load_flight_model;
    use crate::flight::DEFAULT_STEP_S;

    /// M15 P4's first validation shot, whose printed carry is 125.6 yd.
    fn reference() -> UnspunLaunch {
        UnspunLaunch {
            ball_speed_mph: 90.7,
            launch_angle_deg: 20.9,
            launch_direction_deg: -5.3,
            spin_axis_deg: 2.5,
        }
    }

    fn window_of(launch: &UnspunLaunch) -> CarryWindow {
        carry_window(launch, load_flight_model(), DEFAULT_STEP_S).expect("a flyable launch")
    }

    fn solve(launch: &UnspunLaunch, target: f64) -> SpinSolution {
        solve_spin_from_carry(launch, target, load_flight_model(), DEFAULT_STEP_S, None)
            .expect("a flyable launch")
    }

    /// The five segments are in the order the module doc draws them, and the two plateau *values*
    /// bracket everything the conditions can fly.
    #[test]
    fn the_window_has_the_five_segment_shape_the_module_describes() {
        let window = window_of(&reference());
        assert!(window.low_plateau_max_rpm > 0.0);
        assert!(window.low_plateau_max_rpm < window.peak_rpm);
        assert!(window.peak_rpm < window.high_plateau_min_rpm);
        assert!(window.peak_yds >= window.low_plateau_yds);
        assert!(window.peak_yds >= window.high_plateau_yds);
        assert_eq!(
            window.floor_yds(),
            window.low_plateau_yds.min(window.high_plateau_yds)
        );
        assert_eq!(window.width_yds(), window.peak_yds - window.floor_yds());
        // The 2026-09-05c correction: the floor is the *low*-spin clamp on this shot, not the high.
        assert_eq!(window.floor_yds(), window.low_plateau_yds);
    }

    /// **Both plateaus are flat to the bit**, which is the property that makes a carry unable to
    /// recover an iron's spin: `w` reaches the flight only through `S`, and `S` only through the
    /// coefficients, so once they are held spin has left the problem.
    #[test]
    fn the_high_plateau_is_bit_identical_across_a_25000_rpm_span() {
        let model = load_flight_model();
        let launch = reference();
        let window = window_of(&launch);
        let at = |rpm: f64| {
            simulate_flight(&launch.at_spin(rpm), model, DEFAULT_STEP_S)
                .unwrap()
                .carry_yds()
        };
        assert_eq!(at(window.high_plateau_min_rpm), window.high_plateau_yds);
        assert_eq!(at(5200.0), at(30000.0));
        assert_eq!(at(30000.0), window.high_plateau_yds);
        // And the low plateau likewise, from a standstill up to its shoulder.
        assert_eq!(at(0.0), window.low_plateau_yds);
        assert_eq!(at(window.low_plateau_max_rpm), window.low_plateau_yds);
    }

    /// `high_plateau_min_rpm` is **analytic, not searched**: the launch spin ratio is the smallest in
    /// a flight, so it alone decides whether the whole path is clamped. Checked against the
    /// coefficient table's own ceiling rather than against a second search.
    #[test]
    fn the_high_shoulder_is_the_launch_spin_ratio_reaching_the_tables_ceiling() {
        let model = load_flight_model();
        let launch = reference();
        let window = window_of(&launch);
        let flown = simulate_flight(
            &launch.at_spin(window.high_plateau_min_rpm),
            model,
            DEFAULT_STEP_S,
        )
        .unwrap();
        assert!((flown.points[0].spin_ratio - model.coefficients.spin_ratio_max()).abs() < 1e-12);
        assert!(flown.fully_clamped());
    }

    /// The three cases the committed vectors reach, driven here off the window rather than off a
    /// recorded shot — so a failure says which *branch* moved rather than which vector did.
    #[test]
    fn a_target_above_the_peak_and_one_inside_it_land_on_the_right_cases() {
        let launch = reference();
        let window = window_of(&launch);

        let above = solve(&launch, window.peak_yds + 1.0);
        assert_eq!(above.case, SpinSolveCase::AbovePeak);
        assert_eq!(above.candidates(), Vec::<f64>::new());

        // Inside the two-branch band: between the high plateau and the peak, both branches reach.
        let mid = 0.5 * (window.high_plateau_yds + window.peak_yds);
        let two = solve(&launch, mid);
        assert_eq!(two.case, SpinSolveCase::TwoBranches);
        assert_eq!(two.candidates().len(), 2);
        let (rising, falling) = (two.rising_rpm.unwrap(), two.falling_rpm.unwrap());
        assert!(rising < window.peak_rpm && falling > window.peak_rpm);

        // Between the two plateau values, where only the rising branch comes down that far.
        let between = 0.5 * (window.low_plateau_yds + window.high_plateau_yds);
        let one = solve(&launch, between);
        assert_eq!(one.case, SpinSolveCase::BetweenPlateaus);
        assert_eq!(one.candidates().len(), 1);
        assert!(one.falling_rpm.is_none());
    }

    /// **The four cases no committed vector reaches**, which is why they are here.
    ///
    /// `at_peak` is the knife edge inside the printed carry's own last digit; the two plateau cases
    /// refuse rather than pick a representative from a flat interval; and `below_floor` is the one
    /// ADR-027 §Decision 4 did not know existed.
    #[test]
    fn the_four_ungated_cases_behave_as_their_variants_say() {
        let launch = reference();
        let window = window_of(&launch);

        // Just above the peak, by less than half the last printed digit: both branches collapse
        // onto the peak, so it is one answer rather than two.
        let at_peak = solve(&launch, window.peak_yds + PRINTED_PRECISION_YDS / 2.0);
        assert_eq!(at_peak.case, SpinSolveCase::AtPeak);
        assert_eq!(at_peak.candidates(), vec![window.peak_rpm]);
        assert_eq!(at_peak.rising_rpm, at_peak.falling_rpm);

        for target in [
            window.high_plateau_yds,
            window.high_plateau_yds + PRINTED_PRECISION_YDS,
        ] {
            let on = solve(&launch, target);
            assert_eq!(on.case, SpinSolveCase::OnHighPlateau, "{target}");
            assert!(on.candidates().is_empty(), "no representative is invented");
        }
        let low = solve(&launch, window.low_plateau_yds);
        assert_eq!(low.case, SpinSolveCase::OnLowPlateau);
        assert!(low.candidates().is_empty());

        let below = solve(&launch, window.floor_yds() - 1.0);
        assert_eq!(below.case, SpinSolveCase::BelowFloor);
        assert!(below.candidates().is_empty());
    }

    /// **The plateau checks run before the floor check, and the order is load-bearing.** A target
    /// exactly on the low plateau is also *not* below the floor, but one a hair under it is both
    /// on-plateau and below-floor — and it must report the plateau, because that is what the carry
    /// was read to 1 dp as. Swapping the two would turn an unrecoverable spin into a refusal about
    /// the model's range.
    #[test]
    fn a_target_inside_the_printed_precision_of_the_floor_reads_as_a_plateau() {
        let launch = reference();
        let window = window_of(&launch);
        let just_under = window.floor_yds() - PRINTED_PRECISION_YDS / 2.0;
        assert!(just_under < window.floor_yds());
        assert_eq!(solve(&launch, just_under).case, SpinSolveCase::OnLowPlateau);
    }

    /// `2026-08-23-2` is on disk and its carry curve **only rises** — launched at 3.4°, it runs to
    /// the high plateau and never falls, so its peak *is* its high plateau to the last bit and it has
    /// no two-solution band at all. The committed window for it says exactly that, and this is the
    /// mechanism behind it.
    #[test]
    fn a_shallow_launch_peaks_on_its_high_plateau_and_has_no_falling_branch() {
        let shallow = UnspunLaunch {
            ball_speed_mph: 83.8,
            launch_angle_deg: 3.4,
            launch_direction_deg: -0.1,
            spin_axis_deg: 0.0,
        };
        let window = window_of(&shallow);
        assert_eq!(window.peak_yds, window.high_plateau_yds);
        assert_eq!(window.peak_rpm, window.high_plateau_min_rpm);
        // So every reachable target between the plateaus has one answer and not two.
        let between = 0.5 * (window.low_plateau_yds + window.high_plateau_yds);
        let solved = solve(&shallow, between);
        assert_eq!(solved.case, SpinSolveCase::BetweenPlateaus);
        assert!(solved.falling_rpm.is_none());
    }

    /// The solve **re-flies rather than remembering**: handing it a window it already measured gives
    /// the same answer as letting it measure its own. That is the property that keeps a re-sourced
    /// coefficient table from being quietly ignored.
    #[test]
    fn a_handed_in_window_gives_the_same_answer_as_a_measured_one() {
        let launch = reference();
        let window = window_of(&launch);
        let target = 0.5 * (window.high_plateau_yds + window.peak_yds);
        let measured = solve(&launch, target);
        let handed = solve_spin_from_carry(
            &launch,
            target,
            load_flight_model(),
            DEFAULT_STEP_S,
            Some(window),
        )
        .unwrap();
        assert_eq!(measured, handed);
    }

    /// The honest test ADR-027 §Decision 3 can be given: re-solve the one shot whose real spin is on
    /// disk as if it had never been printed. **One refusal and one answer 47% low** is what the
    /// 2026-09-05b addendum measured, and the refusal is this one — 121.0 yd printed against a floor
    /// these conditions cannot come down to.
    #[test]
    fn the_second_validation_shot_prints_a_carry_below_its_own_floor() {
        let launch = UnspunLaunch::from_launch(&LaunchConditions {
            ball_speed_mph: 90.5,
            launch_angle_deg: 23.5,
            spin_rpm: 8100.0,
            launch_direction_deg: 4.0,
            spin_axis_deg: 9.3,
        });
        let solved = solve(&launch, 121.0);
        assert_eq!(solved.case, SpinSolveCase::BelowFloor);
        assert!(solved.window.floor_yds() > 121.0);

        // And the **other** shot is the 47%: its printed 125.6 yd sits in the two-branch band, and
        // the branch a loft prior would not have to choose — the rising one — comes back at
        // 1904.6 rpm against a printed 5991. Both branches are under the truth, which is why
        // ADR-027 §Decision 3 calls this a path-drawing device and not a spin.
        let first = solve(&reference(), 125.6);
        assert_eq!(first.case, SpinSolveCase::TwoBranches);
        let rising = first.rising_rpm.expect("the rising branch reaches");
        let falling = first.falling_rpm.expect("the falling branch reaches");
        assert!((rising - 1904.5532565634658).abs() < 1e-9, "{rising}");
        assert!(
            falling < 0.6 * 5991.0,
            "{falling} rpm against a printed 5991"
        );
    }

    /// `from_launch` drops the spin and keeps the other four fields; `at_spin` puts one back. The
    /// round trip is what lets a printed shot be re-solved without a second literal.
    #[test]
    fn dropping_and_restoring_the_spin_is_a_round_trip_on_the_other_four_fields() {
        let full = LaunchConditions {
            ball_speed_mph: 90.5,
            launch_angle_deg: 23.5,
            spin_rpm: 8100.0,
            launch_direction_deg: 4.0,
            spin_axis_deg: 9.3,
        };
        assert_eq!(UnspunLaunch::from_launch(&full).at_spin(8100.0), full);
    }

    /// An unflyable launch refuses out of the window rather than panicking, which is what
    /// `flight_measure.fly_shot` catches in P8b: the solve flies the ball dozens of times looking for
    /// a carry, so the raise comes from in there rather than from the final `simulate_flight`.
    #[test]
    fn an_unflyable_launch_refuses_out_of_the_solve_rather_than_the_flight() {
        let flat = UnspunLaunch {
            ball_speed_mph: 88.0,
            launch_angle_deg: 0.0,
            launch_direction_deg: 0.0,
            spin_axis_deg: 0.0,
        };
        let refused =
            carry_window(&flat, load_flight_model(), DEFAULT_STEP_S).expect_err("refused");
        assert_eq!(
            refused.message,
            "launch angle must be above the horizontal and below the vertical to leave the ground, \
             got 0.0 deg"
        );
        assert!(
            solve_spin_from_carry(&flat, 120.0, load_flight_model(), DEFAULT_STEP_S, None).is_err()
        );
    }
}
