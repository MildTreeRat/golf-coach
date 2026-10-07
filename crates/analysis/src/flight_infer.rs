//! The two launch conditions the screen did not print, and what it takes to stand in. [M22 P8b]
//!
//! [`crate::flight`] flies a ball from five numbers. The HD Golf screen prints three of them on every
//! shot, the fourth (spin) on two of thirteen, and the fifth (spin axis) on the same two — and
//! ADR-027 §Decisions 3 and 5 are the two answers to that. This module is both of them, and it is the
//! first place in the outcome half where a refusal is a *result* rather than an exception:
//!
//! - **spin** comes from [`crate::spin_solve::solve_spin_from_carry`], which returns up to two
//!   candidates, and §Decision 3 gives loft the job of choosing between them. That is loft's entire
//!   involvement in ball flight — an involvement in the *inference*, not in the flight, which is why
//!   a club bent 2° strong changes the inferred spin and not the path the ball takes.
//! - **the spin axis** comes from the measured `spin_axis` where the screen printed one, and
//!   otherwise from `Shot Type` and face-to-path.
//!
//! Everything unresolvable comes back as an `UnscoredReason`, all of them with `refilming_helps`
//! false, because a launch monitor withholding a number is not a camera problem and a golfer must
//! never be sent back to the bay over one.
//!
//! # What the corpus gates, measured rather than assumed
//!
//! Of the fifteen corpus vectors, **four printed a spin** and skip the solve entirely; **one** infers
//! one and reaches [`InferredSpin`]'s answer; and **ten refuse**. Between them the eleven solves reach
//! four of the twelve sentences this module can produce: `above_peak` (seven), `two_branches` with no
//! loft (two), `two_branches` taking the falling branch (one), and `between_plateaus` with a loft
//! above the floor (one).
//!
//! **The loft prior is gated, contrary to what M15 P9 recorded.** Five of the fifteen carry
//! `loft_deg: 30.5` — `conformance_vectors._identity` resolves it from the swing manifest's club,
//! which is a thing that did not exist when the Python module doc said no shot on disk carries one —
//! so [`LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG`] is compared against a real value on two committed
//! vectors. What is *not* gated is the floor's position: every one of the five is 30.5°, seventeen
//! degrees clear of it, so a port that moved the constant anywhere below 30.5 would pass all 21.
//!
//! Three of the five axis branches are gated too: the measured read (four vectors), face-to-path for
//! the direction (ten), and **neither** — `2026-08-23-3`, which printed no axis and no face angle.
//!
//! What no vector reaches, and so what the unit tests below are the only gate for: a **left-handed**
//! shot (the mirror `handedness` exists for), a printed axis with **no golfer attributed**
//! (`NoHandedness`), `AtPeak`, `BelowFloor`, both plateau cases, and the three loft sentences a club
//! *below* the floor produces.
//!
//! One thing the corpus contains and the stage vector cannot see: `2026-08-23-11` derives a **draw**
//! from its face-to-path against a `FADE` on the screen, so [`InferredSpinAxis::sign_disagrees`] is
//! true on a committed swing — and it is a method rather than a field, so `run_stages` records
//! neither it nor anything computed from it. The disagreement is in `spec/` and the bit is not.
//!
//! # What is not ported
//!
//! `honest_test`, `HonestTestResult`, `gate_ordering` and `GateOrdering`. They re-solve the two
//! validation shots and measure the gate's inversion, and their only callers are
//! `scripts/simulate_flight.py` and the flight page — `conformance.py::run_vector` reaches none of
//! them. ADR-032 §8's rule, and the same cut P8 made on `flight_model.py`'s altitude what-if.

use contracts::golfer::Handedness;
use contracts::intent::TargetShape;
use contracts::shot::ShotData;
use contracts::unscored::UnscoredReason;

use crate::benchmarks::flight_model::FlightModel;
use crate::flight::{LaunchConditions, UnflyableLaunch};
use crate::shot_measure::{
    measure_ball_speed, measure_carry_distance, measure_face_to_path, measure_launch_angle,
    measure_start_line, normalize_shot_shape,
};
use crate::spin_solve::{
    solve_spin_from_carry, CarryWindow, SpinSolution, SpinSolveCase, UnspunLaunch,
};
use pyfmt::{fixed, signed_fixed};

/// The loft above which a club spins the ball faster than any peak-carry spin this model produces,
/// so the falling branch is the answer and the rising one is not.
///
/// **Placed in a gap in the equipment rather than fitted to anything.** `carry_window` measures the
/// peak per shot, and on every two-branch shot on disk it lands at 2,402-2,781 rpm — a driver's own
/// spin. A driver is therefore the only club whose two candidates genuinely straddle it; a fairway
/// wood and everything above spins harder than the peak on any published club average. Drivers are
/// built to about 12° and fairway woods start at about 15°, and this sits between, so no real club is
/// near enough the threshold for its exact value to matter.
///
/// **Its presence is exercised and its value is not.** The Python module doc says no shot on disk
/// carries a club, and that has stopped being true of the *vectors*: five of the fifteen carry a
/// 30.5° loft resolved from the swing manifest, and two of them reach this comparison. All five are
/// seventeen degrees above it, so the committed set proves the prior fires and proves nothing at all
/// about where the threshold sits — moving this constant anywhere in `(0, 30.5)` passes all 21, and
/// `the_loft_floor_sits_in_a_gap_no_real_club_is_near` below is what stands there.
pub const LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG: f64 = 13.0;

/// Where a resolved spin axis came from. ADR-027 §Decision 5's order, as far as it reaches.
///
/// There is no `FaceToPath` member, and its absence is M15 P9's finding rather than an omission:
/// face-to-path resolves the *direction* of the curve, which [`InferredSpinAxis::curve_direction`]
/// reports whether or not an axis came out of it, and a direction is not an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinAxisSource {
    /// The `spin_axis` tile, flipped out of the contract's golfer-relative sign into the
    /// integrator's geometric one. The only source that has ever produced a number.
    Measured,
}

impl SpinAxisSource {
    /// The wire name, for the stage vector's `source` field. [`contracts::intent::ClubCategory`]'s
    /// argument: a serializer round trip would allocate and quote, and a second table would drift.
    pub fn as_str(self) -> &'static str {
        match self {
            SpinAxisSource::Measured => "measured",
        }
    }
}

/// Where the spin a flight is drawn with came from — and the two are not interchangeable.
///
/// A measured spin is a reading off the screen. An inferred one is *the spin this integrator needs in
/// order to agree with HD Golf's carry*, which is a different kind of quantity wearing the same unit,
/// so nothing may show one without saying which it had.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinSource {
    Measured,
    Inferred,
}

impl SpinSource {
    pub fn as_str(self) -> &'static str {
        match self {
            SpinSource::Measured => "measured",
            SpinSource::Inferred => "inferred",
        }
    }
}

/// The spin the flight will be drawn with, or the reason there is none — and the cap either way.
///
/// [`Self::solution`] is present on a refusal too, because *which* of [`SpinSolveCase`]'s seven shapes
/// the carry landed on is most of the answer: a printed carry two yards above the peak and one
/// sitting exactly on the high plateau both come back without a number and are entirely different
/// findings about the shot. [`Self::reason`] collapses those seven onto the four things a reader can
/// *do*, and [`Self::detail`] keeps the case.
#[derive(Debug, Clone, PartialEq)]
pub struct InferredSpin {
    pub spin_rpm: Option<f64>,
    /// `None` exactly when [`Self::spin_rpm`] is not. One of `contracts::unscored::INFERENCE_REASONS`
    /// — never a reason whose `refilming_helps` is true.
    pub reason: Option<UnscoredReason>,
    pub detail: String,
    pub solution: SpinSolution,
}

impl InferredSpin {
    pub fn window(&self) -> &CarryWindow {
        &self.solution.window
    }

    /// The most spin this carry could ever have named, and **it must be reported beside the number,
    /// not instead of it**.
    ///
    /// Above this the whole flight launches past the coefficient table's last row, the held end row
    /// makes carry independent of spin, and the inverse problem has nothing left to invert. Both
    /// shots whose real spin is known sit above their own cap — 5,991 and 8,100 rpm against 5,154 and
    /// 5,143 — which is why one of them is refused and the other comes back 46.8% low.
    pub fn cap_rpm(&self) -> f64 {
        self.window().high_plateau_min_rpm
    }

    /// Whether the number produced is the cap itself, to within the root search's tolerance.
    ///
    /// Never true on this corpus, and worth surfacing rather than assuming: an answer sitting on the
    /// cap is the solve saying "at least this much and I cannot see further".
    pub fn at_cap(&self) -> bool {
        self.spin_rpm.is_some_and(|rpm| rpm >= self.cap_rpm() - 1.0)
    }
}

/// How far the spin axis was tilted, or the reason it is unknown — plus which way it curved.
///
/// The two halves are separate on purpose. [`Self::spin_axis_deg`] is refused far more often than
/// [`Self::curve_direction`] is, and a consumer that has to draw a straight flight still wants to be
/// able to say *"this was a fade and the model is not drawing the curve"* rather than saying nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct InferredSpinAxis {
    /// Geometric, in `LaunchConditions::spin_axis_deg`'s sign — **positive curves the ball right** —
    /// which is the contract's `+ = fade` only for a right-handed golfer. A missing handedness
    /// refuses here rather than defaulting: `contracts/dispersion.py` is the precedent, where a
    /// camera-relative sign meeting a mixed-handedness corpus read every left-handed golfer as a
    /// gross fault.
    pub spin_axis_deg: Option<f64>,
    pub source: Option<SpinAxisSource>,
    pub reason: Option<UnscoredReason>,
    pub detail: String,
    /// Which way the ball curved, golfer-relative, from whichever evidence was available — the
    /// measured axis where there is one, else the sign of face-to-path. Independent of whether an
    /// axis magnitude resolved.
    pub curve_direction: Option<TargetShape>,
    /// What the `Shot Type` tile said, through [`normalize_shot_shape`]. The cross-check, and the
    /// reason ADR-027 §Decision 5 could resolve a sign ADR-014 could not: no *numeric* tile carries
    /// the direction and a *text* one does.
    pub screen_shape: Option<TargetShape>,
}

impl InferredSpinAxis {
    /// Whether the derived direction contradicts the word on the screen.
    ///
    /// **A warning and never a silent overwrite** (ADR-027 §Decision 5). A disagreement is
    /// information about the parse, and resolving it quietly is how ADR-014's original sign inversion
    /// survived as long as it did. `None` on either side is not a disagreement — there is simply
    /// nothing to compare.
    pub fn sign_disagrees(&self) -> bool {
        match (self.curve_direction, self.screen_shape) {
            (Some(derived), Some(printed)) => derived != printed,
            _ => false,
        }
    }
}

/// Solve the carry for a spin and let the club's loft choose the branch. `infer_spin`.
///
/// `target_carry_yds` is the carry the launch monitor printed. It is a required positional rather
/// than something read off a `ShotData` here, because a shot with no printed carry has nothing to
/// solve and that absence belongs to the caller.
///
/// Pass `window` when the caller already measured one; it costs about forty flights and both the
/// solve and any report of the cap want the same one.
///
/// **`Result` where Python raises**, as everywhere else in this crate: the solve flies the ball dozens
/// of times, so an unflyable start line or spin axis surfaces from in here rather than from the
/// caller's own `simulate_flight`, and [`crate::flight_measure::fly_shot`] is the boundary that turns
/// it into a stated refusal.
pub fn infer_spin(
    launch: &UnspunLaunch,
    target_carry_yds: f64,
    loft_deg: Option<f64>,
    model: &FlightModel,
    step_s: f64,
    window: Option<CarryWindow>,
) -> Result<InferredSpin, UnflyableLaunch> {
    let solution = solve_spin_from_carry(launch, target_carry_yds, model, step_s, window)?;
    let case = solution.case;
    let where_ = format!("{} yd, case {}", fixed(target_carry_yds, 1), case.as_str());

    let refuse = |reason: UnscoredReason, detail: String| InferredSpin {
        spin_rpm: None,
        reason: Some(reason),
        detail,
        solution,
    };

    Ok(match case {
        SpinSolveCase::AbovePeak | SpinSolveCase::BelowFloor => {
            let above = case == SpinSolveCase::AbovePeak;
            let edge = if above { "peak" } else { "floor" };
            let edge_yds = if above {
                solution.window.peak_yds
            } else {
                solution.window.floor_yds()
            };
            refuse(
                UnscoredReason::CarryUnreachable,
                format!(
                    "{where_}: {} yd is the {edge} of what these launch conditions can fly",
                    fixed(edge_yds, 2)
                ),
            )
        }
        // Infinitely many spins fly exactly this carry. Choosing a representative out of a flat
        // interval is the one invention `spin_solve` is written not to make, and the loft prior
        // cannot rescue it either — a prior narrows a set of two, not a continuum.
        SpinSolveCase::OnLowPlateau | SpinSolveCase::OnHighPlateau => {
            let end = if case == SpinSolveCase::OnLowPlateau {
                "low"
            } else {
                "high"
            };
            refuse(
                UnscoredReason::SpinNotRecoverable,
                format!(
                    "{where_}: every spin on the {end} plateau flies this carry, so none of them \
                     is the answer"
                ),
            )
        }
        // One spin, on a knife edge, and no branch to choose — so this is the one answer loft is not
        // needed for. It has never occurred on a real shot.
        SpinSolveCase::AtPeak => InferredSpin {
            spin_rpm: Some(solution.window.peak_rpm),
            reason: None,
            detail: format!("{where_}: the peak itself"),
            solution,
        },
        SpinSolveCase::BetweenPlateaus => between_plateaus(solution, loft_deg, &where_),
        SpinSolveCase::TwoBranches => two_branches(solution, loft_deg, &where_)?,
    })
}

/// The unique answer, refused — and **argued from where it sits, not from the case's name**.
///
/// ADR-027's 2026-09-05c addendum asked for a blanket refusal here on the ground that the band lies
/// at spins no 7 iron produces, and its 2026-09-05f successor found a shot where it does not: on
/// `2026-08-10-1` the band runs 1,129-1,538 rpm and on `2026-08-23-2` it runs 1,323-4,572 with the
/// answer at 2,307, an ordinary number. So the argument is made against the peak instead.
///
/// A unique answer here is on the *rising* branch by construction — the falling branch never comes
/// down this far, which is what made it unique — so it is below `peak_rpm`. The loft prior says a club
/// above the loft floor spins *above* `peak_rpm`. The two cannot both be right, and the prior is the
/// one built on a club rather than on a printed carry, so the answer is refused. It comes out as a
/// refusal in every case, but for a stated reason that a driver, or a differently-shaped curve, would
/// not satisfy.
fn between_plateaus(solution: SpinSolution, loft_deg: Option<f64>, where_: &str) -> InferredSpin {
    let answer = solution.rising_rpm;
    let peak_rpm = solution.window.peak_rpm;
    let detail = match loft_deg {
        None => {
            let named = match answer {
                None => "it".to_string(),
                Some(rpm) => format!("{} rpm", fixed(rpm, 0)),
            };
            format!(
                "{where_}: one spin flies this carry, and no loft is on record to say whether \
                 {named} is a spin this club makes"
            )
        }
        Some(loft) if loft >= LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG && answer.is_some() => format!(
            "{where_}: the only spin that flies it is {} rpm, below the {} rpm peak, and a {} deg \
             club spins above the peak",
            fixed(answer.expect("the guard above tested it"), 0),
            fixed(peak_rpm, 0),
            fixed(loft, 1),
        ),
        Some(loft) => format!(
            "{where_}: a {} deg club spins about as fast as the {} rpm peak, so nothing here says \
             which side of it this shot was on",
            fixed(loft, 1),
            fixed(peak_rpm, 0),
        ),
    };
    let reason = if loft_deg.is_none() {
        UnscoredReason::NoClubLoft
    } else {
        UnscoredReason::SpinNotRecoverable
    };
    InferredSpin {
        spin_rpm: None,
        reason: Some(reason),
        detail,
        solution,
    }
}

/// Two spins fly the carry, and this is the only place loft does anything.
///
/// The choice is a single comparison against the peak, and the module doc argues why that needs no
/// loft-to-spin model: the peak is measured, it lands at driver spin, and every club with more loft
/// than a driver beats it.
///
/// **Python's `raise RuntimeError` for a two-branch case missing a branch is an `Err` here**, not a
/// panic. The Python marks it `pragma: no cover` — the case is *defined* by both existing — and a
/// panic would be the one failure mode in this crate that takes a whole swing down rather than
/// refusing a flight, which is exactly what `flight_measure`'s `ValueError` catch exists to prevent.
fn two_branches(
    solution: SpinSolution,
    loft_deg: Option<f64>,
    where_: &str,
) -> Result<InferredSpin, UnflyableLaunch> {
    let (Some(rising), Some(falling)) = (solution.rising_rpm, solution.falling_rpm) else {
        return Err(UnflyableLaunch {
            message: format!(
                "{} without two branches: {solution:?}",
                solution.case.as_str()
            ),
        });
    };
    let peak_rpm = solution.window.peak_rpm;

    Ok(match loft_deg {
        None => InferredSpin {
            spin_rpm: None,
            reason: Some(UnscoredReason::NoClubLoft),
            detail: format!(
                "{where_}: {} and {} rpm both fly this carry and no loft is on record to choose \
                 between them",
                fixed(rising, 0),
                fixed(falling, 0),
            ),
            solution,
        },
        // A driver's own spin sits at the peak, so its two candidates straddle it and the prior has
        // nothing to say. Refusing is the whole of ADR-010 §2 applied to an inference.
        Some(loft) if loft < LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG => InferredSpin {
            spin_rpm: None,
            reason: Some(UnscoredReason::SpinNotRecoverable),
            detail: format!(
                "{where_}: a {} deg club spins about as fast as the {} rpm peak, so neither {} nor \
                 {} rpm is the one it was",
                fixed(loft, 1),
                fixed(peak_rpm, 0),
                fixed(rising, 0),
                fixed(falling, 0),
            ),
            solution,
        },
        Some(loft) => InferredSpin {
            spin_rpm: Some(falling),
            reason: None,
            detail: format!(
                "{where_}: the falling branch, because a {} deg club spins above the {} rpm peak; \
                 {} rpm is the branch not taken",
                fixed(loft, 1),
                fixed(peak_rpm, 0),
                fixed(rising, 0),
            ),
            solution,
        },
    })
}

/// Resolve the spin axis, and the direction of the curve, in ADR-027 §Decision 5's order.
///
/// The measured `spin_axis` first, flipped into the integrator's geometric sign; then face-to-path,
/// which resolves the direction and — M15 P9's finding — not the magnitude; then nothing.
///
/// `handedness` is `Golfer.handedness` and is required rather than defaulted, because the flip it
/// controls is exactly the one ADR-014's addendum got wrong for a milestone.
pub fn infer_spin_axis(shot: &ShotData, handedness: Option<Handedness>) -> InferredSpinAxis {
    let screen_shape = normalize_shot_shape(shot);
    let face_to_path = measure_face_to_path(shot);
    let curve_direction = direction_of(shot.spin_axis, face_to_path);

    let Some(printed) = shot.spin_axis else {
        let detail = match face_to_path {
            Some(f2p) => format!(
                "no spin axis printed; face-to-path {} deg gives the direction and no magnitude",
                signed_fixed(f2p, 1)
            ),
            None => "no spin axis printed, and no face angle to take a direction from either"
                .to_string(),
        };
        return InferredSpinAxis {
            spin_axis_deg: None,
            source: None,
            reason: Some(UnscoredReason::SpinAxisUnresolved),
            detail,
            curve_direction,
            screen_shape,
        };
    };

    let Some(handedness) = handedness else {
        return InferredSpinAxis {
            spin_axis_deg: None,
            source: None,
            reason: Some(UnscoredReason::NoHandedness),
            detail: format!(
                "a {} deg axis was printed, but '+ = fade' is a right-handed reading and no golfer \
                 is attributed to this shot",
                signed_fixed(printed, 1)
            ),
            curve_direction,
            screen_shape,
        };
    };

    // `+ = fade` is `+ = right` for a right-handed golfer and `+ = left` for a left-handed one; the
    // integrator's sign is geometric and knows nothing about either.
    let geometric = if handedness == Handedness::Right {
        printed
    } else {
        -printed
    };
    InferredSpinAxis {
        spin_axis_deg: Some(geometric),
        source: Some(SpinAxisSource::Measured),
        reason: None,
        detail: format!(
            "{} deg printed, read {}-handed",
            signed_fixed(printed, 1),
            handedness.as_str()
        ),
        curve_direction,
        screen_shape,
    }
}

/// Which way the ball curved, golfer-relative, from whichever evidence exists.
///
/// Both inputs carry the same sign convention for this purpose — `ShotData.spin_axis` is `+ = fade`
/// and a positive face-to-path is a face open to the path, which is a fade — so this is a sign read
/// and not a conversion. Exact zero is `Straight` rather than a coin toss; nothing on disk is exactly
/// zero and a shot that were would deserve the honest label.
fn direction_of(spin_axis: Option<f64>, face_to_path: Option<f64>) -> Option<TargetShape> {
    let value = spin_axis.or(face_to_path)?;
    Some(if value > 0.0 {
        TargetShape::Fade
    } else if value < 0.0 {
        TargetShape::Draw
    } else {
        TargetShape::Straight
    })
}

/// One stored shot, resolved as far as the screen and the bag allow — or the reason it is not.
///
/// It stops at the launch conditions rather than flying them, and that is deliberate: the caller
/// chooses the model, and [`crate::flight::simulate_flight`] owns the physical guards on what may be
/// flown. What this resolves is only the two fields the screen did not print.
///
/// **The axis refusal does not stop the flight, and the spin refusal does.** With no axis the ball is
/// flown in the vertical plane and its landing offline is withheld — ADR-027 §Decision 5's third
/// branch — so [`Self::reason`] stays `None` and `axis.reason` carries the refusal. That asymmetry is
/// the whole reason the two are separate fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ShotFlight {
    /// Ready to fly, or `None` when [`Self::reason`] says why not. `spin_axis_deg` is zero on it
    /// whenever `axis.spin_axis_deg` is `None`, which is a flight drawn planar and not an axis of
    /// zero measured.
    pub launch: Option<LaunchConditions>,
    pub spin_source: Option<SpinSource>,
    /// The solve, whenever one ran — including when it refused, because [`InferredSpin::solution`]
    /// carries which of the seven cases the carry landed on and the cap the answer sits under. `None`
    /// only when the screen printed a spin, or when there was nothing to solve from.
    pub spin: Option<InferredSpin>,
    pub axis: InferredSpinAxis,
    /// Why there is no flight. One of `contracts::unscored::INFERENCE_REASONS`; never
    /// `SpinAxisUnresolved`, which is a flight drawn straight rather than a flight refused.
    pub reason: Option<UnscoredReason>,
    pub detail: String,
}

impl ShotFlight {
    /// The spin the flight is drawn with, whichever way it was arrived at.
    pub fn spin_rpm(&self) -> Option<f64> {
        self.launch.as_ref().map(|launch| launch.spin_rpm)
    }

    /// Whether this flight bends — and so whether its landing point is a landing point.
    ///
    /// False whenever the axis was refused. **A planar flight's `landing_offline_yds` is not new
    /// information**: with the axis at zero it comes out as `carry * sin(start line)` to the last bit,
    /// which is [`crate::shot_measure::measure_start_line_offline`] reached by forty flights instead
    /// of one sine. That identity is why ADR-027 §Decision 6's `flight_landing_offline_yds` must not
    /// be recorded on a shot whose axis is unresolved: two names for one number is exactly the
    /// pooling hazard that section exists to prevent.
    pub fn curve_is_drawn(&self) -> bool {
        self.axis.spin_axis_deg.is_some()
    }
}

/// Turn one stored shot into launch conditions, solving for the spin where the screen has none.
/// `flight_for_shot`.
///
/// `loft_deg` and `handedness` have no defaults, because both have a plausible-looking wrong answer —
/// a 7 iron's loft and a right-handed golfer — and both come from a different artifact than the shot
/// does. Passing `None` for either is the corpus's own state on most shots and produces a stated
/// refusal rather than a guess.
///
/// Every field is read through [`crate::shot_measure`]'s extractors rather than off `ShotData`
/// directly, so there is one definition of which tile is the carry and which is the start line.
pub fn flight_for_shot(
    shot: &ShotData,
    loft_deg: Option<f64>,
    handedness: Option<Handedness>,
    model: &FlightModel,
    step_s: f64,
) -> Result<ShotFlight, UnflyableLaunch> {
    let axis = infer_spin_axis(shot, handedness);
    // A refused axis is flown planar, not refused: ADR-027 §Decision 5's third branch. The zero here
    // is `LaunchConditions`' own default arriving explicitly, and `curve_is_drawn` is what stops a
    // reader taking the resulting zero offline for a measurement.
    let axis_deg = axis.spin_axis_deg.unwrap_or(0.0);

    let refuse = |axis: InferredSpinAxis, detail: String| ShotFlight {
        launch: None,
        spin_source: None,
        spin: None,
        axis,
        reason: Some(UnscoredReason::NoLaunchConditions),
        detail,
    };

    let ball_speed = measure_ball_speed(shot);
    let launch_angle = measure_launch_angle(shot);
    let (Some(ball_speed), Some(launch_angle)) = (ball_speed, launch_angle) else {
        // Python's `' and no '.join(missing)` over a list built in this order, so a shot missing both
        // reads "no ball speed and no launch angle" and one missing either names only that one.
        let missing: Vec<&str> = [("ball speed", ball_speed), ("launch angle", launch_angle)]
            .iter()
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| *name)
            .collect();
        return Ok(refuse(
            axis,
            format!("the screen printed no {}", missing.join(" and no ")),
        ));
    };
    if !(0.0 < launch_angle && launch_angle < 90.0) {
        // `simulate_flight` names this check as the caller's to own: a ball at or below the
        // horizontal rolls, and roll is out of scope. Refused here, where there is a shot to name,
        // rather than raised out of the integrator two frames later.
        return Ok(refuse(
            axis,
            format!(
                "{} deg of launch angle is a ball that rolls rather than flies",
                fixed(launch_angle, 1)
            ),
        ));
    }

    // Python's `measure_start_line(shot) or 0.0`, which is **falsy** rather than `is None` — so a
    // printed start line of exactly 0.0 also reads as 0.0 and the two agree. Kept as `unwrap_or`
    // because the values coincide; a transform where they did not would need the `or` spelled out.
    let start_line = measure_start_line(shot).unwrap_or(0.0);
    let unspun = UnspunLaunch {
        ball_speed_mph: ball_speed,
        launch_angle_deg: launch_angle,
        launch_direction_deg: start_line,
        spin_axis_deg: axis_deg,
    };

    if let Some(printed) = shot.spin_rate {
        return Ok(ShotFlight {
            launch: Some(unspun.at_spin(printed)),
            spin_source: Some(SpinSource::Measured),
            spin: None,
            axis,
            reason: None,
            detail: format!("{} rpm printed on the screen", fixed(printed, 0)),
        });
    }

    let Some(carry) = measure_carry_distance(shot) else {
        return Ok(refuse(
            axis,
            "no spin printed, and no carry to solve one from either".to_string(),
        ));
    };

    let spin = infer_spin(&unspun, carry, loft_deg, model, step_s, None)?;
    Ok(match spin.spin_rpm {
        None => ShotFlight {
            launch: None,
            spin_source: None,
            reason: spin.reason,
            detail: spin.detail.clone(),
            spin: Some(spin),
            axis,
        },
        Some(rpm) => ShotFlight {
            launch: Some(unspun.at_spin(rpm)),
            spin_source: Some(SpinSource::Inferred),
            reason: None,
            detail: spin.detail.clone(),
            spin: Some(spin),
            axis,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::flight_model::load_flight_model;
    use crate::flight::DEFAULT_STEP_S;
    use crate::testing::a_shot;

    /// A shot the corpus never supplies: both launch tiles, a carry, and whatever else a test sets.
    fn a_flyable_shot() -> ShotData {
        let mut shot = a_shot();
        shot.ball_speed = Some(89.8);
        shot.launch_angle = Some(22.4);
        shot.launch_direction = Some(2.8);
        shot.carry_distance = Some(126.1);
        shot
    }

    // -------------------------------------------------------------------------------------------
    // The axis — five branches, of which the committed vectors reach two
    // -------------------------------------------------------------------------------------------

    /// The measured branch, right-handed: the printed sign passes through unchanged.
    #[test]
    fn a_printed_axis_is_read_geometrically_for_a_right_handed_golfer() {
        let mut shot = a_shot();
        shot.spin_axis = Some(2.5);
        shot.shot_type = Some("FADE".to_string());
        let axis = infer_spin_axis(&shot, Some(Handedness::Right));
        assert_eq!(axis.spin_axis_deg, Some(2.5));
        assert_eq!(axis.source, Some(SpinAxisSource::Measured));
        assert_eq!(axis.reason, None);
        assert_eq!(axis.detail, "+2.5 deg printed, read right-handed");
        assert_eq!(axis.curve_direction, Some(TargetShape::Fade));
        assert_eq!(axis.screen_shape, Some(TargetShape::Fade));
        assert!(!axis.sign_disagrees());
    }

    /// **The mirror, which no committed vector reaches**: a left-handed fade curves the ball left, so
    /// the geometric sign flips and the golfer-relative direction does not.
    #[test]
    fn a_printed_axis_is_flipped_for_a_left_handed_golfer() {
        let mut shot = a_shot();
        shot.spin_axis = Some(2.5);
        let axis = infer_spin_axis(&shot, Some(Handedness::Left));
        assert_eq!(axis.spin_axis_deg, Some(-2.5));
        assert_eq!(axis.detail, "+2.5 deg printed, read left-handed");
        // The *direction* is read off the printed sign and is golfer-relative, so it does not flip.
        assert_eq!(axis.curve_direction, Some(TargetShape::Fade));
    }

    /// A printed axis with no golfer attributed refuses rather than assuming right-handed — the one
    /// place `NoHandedness` reaches this module, and it is not an `INFERENCE_REASON`.
    #[test]
    fn a_printed_axis_with_no_golfer_refuses_rather_than_assuming_a_side() {
        let mut shot = a_shot();
        shot.spin_axis = Some(-3.0);
        let axis = infer_spin_axis(&shot, None);
        assert_eq!(axis.spin_axis_deg, None);
        assert_eq!(axis.source, None);
        assert_eq!(axis.reason, Some(UnscoredReason::NoHandedness));
        assert_eq!(
            axis.detail,
            "a -3.0 deg axis was printed, but '+ = fade' is a right-handed reading and no golfer \
             is attributed to this shot"
        );
        assert_eq!(axis.curve_direction, Some(TargetShape::Draw));
    }

    /// **And the refusal reaches the flight**, which is the half the axis test above cannot see.
    ///
    /// A mutation sweep found this: `axis.spin_axis_deg.or(shot.spin_axis)` in [`flight_for_shot`]
    /// survives all 21 vectors and every other test here, because the only shots with a printed axis
    /// in `spec/` also have a golfer. A port that reached past the refusal would fly this ball on a
    /// `+ = fade` reading for a golfer whose handedness is unknown — the exact inversion ADR-014's
    /// addendum recorded, arriving one layer further in.
    #[test]
    fn a_refused_axis_is_flown_planar_and_never_on_the_number_it_refused() {
        let mut shot = a_flyable_shot();
        shot.spin_axis = Some(-3.0);
        shot.spin_rate = Some(5991.0);
        let resolved = flight_for_shot(&shot, None, None, load_flight_model(), DEFAULT_STEP_S)
            .expect("a flyable shot");
        assert_eq!(resolved.axis.reason, Some(UnscoredReason::NoHandedness));
        assert_eq!(
            resolved
                .launch
                .expect("the flight still stands")
                .spin_axis_deg,
            0.0,
            "the printed axis reached the integrator through a refusal"
        );
        assert!(!resolved.curve_is_drawn());
    }

    /// The branch eleven of the fifteen take: no axis, a face angle, so a direction and no magnitude.
    #[test]
    fn no_printed_axis_gives_the_direction_from_face_to_path_and_no_magnitude() {
        let mut shot = a_shot();
        shot.club_face_angle = Some(4.0);
        shot.club_path = Some(-9.2);
        let axis = infer_spin_axis(&shot, Some(Handedness::Right));
        assert_eq!(axis.spin_axis_deg, None);
        assert_eq!(axis.reason, Some(UnscoredReason::SpinAxisUnresolved));
        assert_eq!(
            axis.detail,
            "no spin axis printed; face-to-path +13.2 deg gives the direction and no magnitude"
        );
        assert_eq!(axis.curve_direction, Some(TargetShape::Fade));
    }

    /// And with neither, there is nothing to take a direction from — a sentence no vector reaches.
    #[test]
    fn no_axis_and_no_face_angle_leaves_even_the_direction_unknown() {
        let axis = infer_spin_axis(&a_shot(), Some(Handedness::Right));
        assert_eq!(
            axis.detail,
            "no spin axis printed, and no face angle to take a direction from either"
        );
        assert_eq!(axis.curve_direction, None);
        assert_eq!(axis.screen_shape, None);
        assert!(
            !axis.sign_disagrees(),
            "nothing to compare is not a disagreement"
        );
    }

    /// Exact zero is `Straight` and not a coin toss, on either input.
    #[test]
    fn an_axis_of_exactly_zero_is_straight() {
        let mut shot = a_shot();
        shot.spin_axis = Some(0.0);
        assert_eq!(
            infer_spin_axis(&shot, Some(Handedness::Right)).curve_direction,
            Some(TargetShape::Straight)
        );
        shot.spin_axis = None;
        shot.club_face_angle = Some(3.0);
        shot.club_path = Some(3.0);
        assert_eq!(
            infer_spin_axis(&shot, Some(Handedness::Right)).curve_direction,
            Some(TargetShape::Straight)
        );
    }

    /// **The axis outranks the face angle**, which is ADR-027 §Decision 5's order: a shot whose two
    /// pieces of evidence disagree reports the axis's direction and flags the contradiction.
    #[test]
    fn a_derived_direction_that_contradicts_the_screen_is_flagged_and_not_overwritten() {
        let mut shot = a_shot();
        shot.spin_axis = Some(2.5);
        shot.club_face_angle = Some(-8.0);
        shot.club_path = Some(2.0);
        shot.shot_type = Some("DRAW".to_string());
        let axis = infer_spin_axis(&shot, Some(Handedness::Right));
        assert_eq!(axis.curve_direction, Some(TargetShape::Fade));
        assert_eq!(axis.screen_shape, Some(TargetShape::Draw));
        assert!(axis.sign_disagrees());
        // And the value is untouched: a warning, never a silent overwrite.
        assert_eq!(axis.spin_axis_deg, Some(2.5));
    }

    // -------------------------------------------------------------------------------------------
    // The spin — the loft prior, which no committed vector reaches at all
    // -------------------------------------------------------------------------------------------

    /// The two-branch shot the corpus has, with a loft: the falling branch, and the sentence names
    /// the branch not taken.
    #[test]
    fn a_lofted_club_takes_the_falling_branch_and_says_which_one_it_did_not() {
        let unspun = UnspunLaunch {
            ball_speed_mph: 89.8,
            launch_angle_deg: 22.4,
            launch_direction_deg: 2.8,
            spin_axis_deg: 0.0,
        };
        let spin = infer_spin(
            &unspun,
            126.1,
            Some(30.5),
            load_flight_model(),
            DEFAULT_STEP_S,
            None,
        )
        .expect("these launch conditions fly");
        assert_eq!(spin.solution.case, SpinSolveCase::TwoBranches);
        let rpm = spin.spin_rpm.expect("a loft above the floor chooses one");
        assert!(spin
            .detail
            .contains("the falling branch, because a 30.5 deg club spins above the"));
        assert!(spin.detail.contains("rpm is the branch not taken"));
        // The falling branch is the *higher* spin: carry falls away as spin rises past the peak,
        // which is what the loft prior is choosing when it says a lofted club spins above it.
        assert!(rpm > spin.solution.window.peak_rpm);
        assert_eq!(Some(rpm), spin.solution.falling_rpm);
        assert!(!spin.at_cap());
    }

    /// **The branch is invariant anywhere in 12.0-15.0**, which is the honest form of "this number is
    /// coarse and the margin is enormous" — the Python pins the same invariance.
    #[test]
    fn the_loft_floor_sits_in_a_gap_no_real_club_is_near() {
        let unspun = UnspunLaunch {
            ball_speed_mph: 89.8,
            launch_angle_deg: 22.4,
            launch_direction_deg: 2.8,
            spin_axis_deg: 0.0,
        };
        let at = |loft: f64| {
            infer_spin(
                &unspun,
                126.1,
                Some(loft),
                load_flight_model(),
                DEFAULT_STEP_S,
                None,
            )
            .expect("these launch conditions fly")
            .spin_rpm
        };
        // A driver refuses; everything from a fairway wood up takes the falling branch.
        assert_eq!(at(12.0), None);
        assert_eq!(at(12.9), None);
        assert!(at(13.0).is_some());
        assert_eq!(at(15.0), at(30.5));
    }

    /// No loft on record is `NoClubLoft` and names both candidates, which is the corpus's own state
    /// on the two-branch shots.
    #[test]
    fn two_branches_with_no_loft_names_both_and_refuses() {
        let unspun = UnspunLaunch {
            ball_speed_mph: 89.8,
            launch_angle_deg: 22.4,
            launch_direction_deg: 2.8,
            spin_axis_deg: 0.0,
        };
        let spin = infer_spin(
            &unspun,
            126.1,
            None,
            load_flight_model(),
            DEFAULT_STEP_S,
            None,
        )
        .expect("these launch conditions fly");
        assert_eq!(spin.spin_rpm, None);
        assert_eq!(spin.reason, Some(UnscoredReason::NoClubLoft));
        assert!(spin
            .detail
            .contains("rpm both fly this carry and no loft is on record to choose between them"));
    }

    /// A driver's own spin sits at the peak, so its two candidates straddle it and the prior refuses
    /// rather than guessing — ADR-010 §2 applied to an inference.
    #[test]
    fn a_driver_refuses_because_its_candidates_straddle_the_peak() {
        let unspun = UnspunLaunch {
            ball_speed_mph: 89.8,
            launch_angle_deg: 22.4,
            launch_direction_deg: 2.8,
            spin_axis_deg: 0.0,
        };
        let spin = infer_spin(
            &unspun,
            126.1,
            Some(10.5),
            load_flight_model(),
            DEFAULT_STEP_S,
            None,
        )
        .expect("these launch conditions fly");
        assert_eq!(spin.reason, Some(UnscoredReason::SpinNotRecoverable));
        assert!(spin
            .detail
            .contains("a 10.5 deg club spins about as fast as the"));
        assert!(spin.detail.contains("rpm is the one it was"));
    }

    /// **`AtPeak` — the one case where a spin comes back with no loft consulted**, reached by nothing
    /// in `spec/` and, until a mutation sweep said so, by nothing here either.
    ///
    /// It is the sliver between the peak and the peak plus the printed carry's own last digit: the two
    /// branches have collapsed onto each other, so the peak *is* the answer rather than either of
    /// them. Constructed by measuring the window and asking for a carry just inside that sliver, which
    /// is the only way to reach it — a hand-picked target would be a guess at where the peak sits.
    #[test]
    fn a_carry_a_hair_above_the_peak_answers_with_the_peak_itself() {
        let unspun = UnspunLaunch {
            ball_speed_mph: 89.8,
            launch_angle_deg: 22.4,
            launch_direction_deg: 2.8,
            spin_axis_deg: 0.0,
        };
        let model = load_flight_model();
        let window = crate::spin_solve::carry_window(&unspun, model, DEFAULT_STEP_S)
            .expect("these launch conditions fly");
        let spin = infer_spin(
            &unspun,
            window.peak_yds + 0.01,
            None,
            model,
            DEFAULT_STEP_S,
            None,
        )
        .expect("these launch conditions fly");

        assert_eq!(spin.solution.case, SpinSolveCase::AtPeak);
        assert_eq!(spin.spin_rpm, Some(window.peak_rpm));
        assert_eq!(spin.reason, None, "no loft is needed and none is missed");
        assert!(spin.detail.ends_with(": the peak itself"));
    }

    /// **The `between_plateaus` loft comparison is `>=`, and only a loft of exactly the floor can tell.**
    ///
    /// The other survivor of the same sweep: turning it into `>` passes all 21 vectors and every other
    /// test here, because every loft in `spec/` is 30.5. This is the boundary itself, on the corpus's
    /// own between-plateaus shot — a club built exactly to the floor takes the first sentence, not the
    /// "spins about as fast as the peak" one.
    #[test]
    fn a_club_at_exactly_the_loft_floor_is_above_it() {
        let unspun = UnspunLaunch {
            ball_speed_mph: 83.8,
            launch_angle_deg: 3.4,
            launch_direction_deg: -0.1,
            spin_axis_deg: 0.0,
        };
        let at = |loft: f64| {
            let spin = infer_spin(
                &unspun,
                33.6,
                Some(loft),
                load_flight_model(),
                DEFAULT_STEP_S,
                None,
            )
            .expect("these launch conditions fly");
            assert_eq!(spin.solution.case, SpinSolveCase::BetweenPlateaus);
            spin.detail
        };
        assert!(at(LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG)
            .contains("and a 13.0 deg club spins above the peak"));
        assert!(
            at(LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG - 0.1).contains("spins about as fast as the")
        );
    }

    /// A carry past the peak is `CarryUnreachable`, and the sentence quotes the edge to two decimals
    /// — the branch nine of the fifteen corpus vectors take.
    #[test]
    fn a_carry_above_the_peak_names_the_peak_it_could_not_reach() {
        let unspun = UnspunLaunch {
            ball_speed_mph: 89.8,
            launch_angle_deg: 22.4,
            launch_direction_deg: 2.8,
            spin_axis_deg: 0.0,
        };
        let spin = infer_spin(
            &unspun,
            400.0,
            None,
            load_flight_model(),
            DEFAULT_STEP_S,
            None,
        )
        .expect("these launch conditions fly");
        assert_eq!(spin.reason, Some(UnscoredReason::CarryUnreachable));
        assert!(spin.detail.starts_with("400.0 yd, case above_peak: "));
        assert!(spin
            .detail
            .ends_with(" yd is the peak of what these launch conditions can fly"));
    }

    /// And a carry below the floor names the floor, which is the same sentence with two words moved.
    /// **No committed vector reaches it** — the corpus misses high, never low.
    #[test]
    fn a_carry_below_the_floor_names_the_floor() {
        let unspun = UnspunLaunch {
            ball_speed_mph: 89.8,
            launch_angle_deg: 22.4,
            launch_direction_deg: 2.8,
            spin_axis_deg: 0.0,
        };
        let spin = infer_spin(
            &unspun,
            1.0,
            None,
            load_flight_model(),
            DEFAULT_STEP_S,
            None,
        )
        .expect("these launch conditions fly");
        assert_eq!(spin.reason, Some(UnscoredReason::CarryUnreachable));
        assert!(spin.detail.starts_with("1.0 yd, case below_floor: "));
        assert!(spin
            .detail
            .ends_with(" yd is the floor of what these launch conditions can fly"));
    }

    /// The cap is the high plateau's lower edge, reported beside the number and never instead of it.
    #[test]
    fn the_cap_is_the_high_plateaus_own_edge() {
        let unspun = UnspunLaunch {
            ball_speed_mph: 89.8,
            launch_angle_deg: 22.4,
            launch_direction_deg: 2.8,
            spin_axis_deg: 0.0,
        };
        let spin = infer_spin(
            &unspun,
            126.1,
            None,
            load_flight_model(),
            DEFAULT_STEP_S,
            None,
        )
        .expect("these launch conditions fly");
        assert_eq!(spin.cap_rpm(), spin.solution.window.high_plateau_min_rpm);
        assert!(
            !spin.at_cap(),
            "a refusal has no number and cannot sit on the cap"
        );
    }

    // -------------------------------------------------------------------------------------------
    // The join — one shot in, launch conditions or a stated refusal out
    // -------------------------------------------------------------------------------------------

    /// A printed spin short-circuits the solve entirely: no `spin`, source `measured`, and the
    /// detail is the printed number. The branch four of the fifteen take.
    #[test]
    fn a_printed_spin_is_flown_as_printed_and_never_solved() {
        let mut shot = a_flyable_shot();
        shot.spin_rate = Some(5991.0);
        shot.spin_axis = Some(2.5);
        let resolved = flight_for_shot(
            &shot,
            None,
            Some(Handedness::Right),
            load_flight_model(),
            DEFAULT_STEP_S,
        )
        .expect("a flyable shot");
        assert_eq!(resolved.spin_source, Some(SpinSource::Measured));
        assert_eq!(
            resolved.spin, None,
            "nothing was solved, so there is no solution to keep"
        );
        assert_eq!(resolved.detail, "5991 rpm printed on the screen");
        assert_eq!(resolved.spin_rpm(), Some(5991.0));
        assert!(resolved.curve_is_drawn());
        let launch = resolved.launch.expect("ready to fly");
        assert_eq!(launch.spin_axis_deg, 2.5);
        assert_eq!(launch.launch_direction_deg, 2.8);
    }

    /// **A refused axis does not stop the flight** — ADR-027 §Decision 5's third branch, and the
    /// asymmetry the two `reason` fields exist for. The axis is zero on the launch and
    /// `curve_is_drawn` is what says that zero is not a measurement.
    #[test]
    fn a_refused_axis_is_flown_planar_rather_than_refused() {
        let mut shot = a_flyable_shot();
        shot.spin_rate = Some(5991.0);
        let resolved = flight_for_shot(
            &shot,
            None,
            Some(Handedness::Right),
            load_flight_model(),
            DEFAULT_STEP_S,
        )
        .expect("a flyable shot");
        assert_eq!(resolved.reason, None, "the flight stands");
        assert_eq!(
            resolved.axis.reason,
            Some(UnscoredReason::SpinAxisUnresolved)
        );
        assert_eq!(resolved.launch.expect("ready to fly").spin_axis_deg, 0.0);
        assert!(!resolved.curve_is_drawn());
    }

    /// Both missing launch tiles are named in one sentence, in the registry's order.
    #[test]
    fn a_shot_with_no_launch_tiles_names_every_one_it_is_missing() {
        let mut shot = a_shot();
        let resolved = |shot: &ShotData| {
            flight_for_shot(shot, None, None, load_flight_model(), DEFAULT_STEP_S)
                .expect("a refusal is not an error")
        };
        assert_eq!(
            resolved(&shot).detail,
            "the screen printed no ball speed and no launch angle"
        );
        shot.ball_speed = Some(89.8);
        assert_eq!(resolved(&shot).detail, "the screen printed no launch angle");
        shot.ball_speed = None;
        shot.launch_angle = Some(22.4);
        assert_eq!(resolved(&shot).detail, "the screen printed no ball speed");
        assert_eq!(
            resolved(&shot).reason,
            Some(UnscoredReason::NoLaunchConditions)
        );
    }

    /// A ball at or below the horizontal rolls, and roll is out of scope — refused here, where there
    /// is a shot to name, rather than raised out of the integrator.
    #[test]
    fn a_launch_angle_at_or_below_the_horizontal_is_a_ball_that_rolls() {
        let mut shot = a_flyable_shot();
        for angle in [0.0, -3.0, 90.0, 91.0] {
            shot.launch_angle = Some(angle);
            let resolved = flight_for_shot(&shot, None, None, load_flight_model(), DEFAULT_STEP_S)
                .expect("a refusal is not an error");
            assert_eq!(
                resolved.reason,
                Some(UnscoredReason::NoLaunchConditions),
                "{angle}"
            );
            assert!(
                resolved
                    .detail
                    .ends_with("of launch angle is a ball that rolls rather than flies"),
                "{}",
                resolved.detail
            );
        }
    }

    /// No spin and no carry leaves nothing to solve from, which is its own sentence.
    #[test]
    fn no_spin_and_no_carry_has_nothing_to_solve_from() {
        let mut shot = a_flyable_shot();
        shot.carry_distance = None;
        let resolved = flight_for_shot(&shot, None, None, load_flight_model(), DEFAULT_STEP_S)
            .expect("a refusal is not an error");
        assert_eq!(
            resolved.detail,
            "no spin printed, and no carry to solve one from either"
        );
        assert_eq!(resolved.spin, None);
    }

    /// **A refused solve keeps its solution**, which is most of what a reader wants when there is no
    /// number: the case says whether the carry was past the peak or on a plateau.
    #[test]
    fn a_refused_solve_still_carries_the_case_it_landed_on() {
        let mut shot = a_flyable_shot();
        shot.carry_distance = Some(400.0);
        let resolved = flight_for_shot(&shot, None, None, load_flight_model(), DEFAULT_STEP_S)
            .expect("a refusal is not an error");
        assert_eq!(resolved.launch, None);
        assert_eq!(resolved.reason, Some(UnscoredReason::CarryUnreachable));
        let spin = resolved.spin.expect("the solve ran and refused");
        assert_eq!(spin.solution.case, SpinSolveCase::AbovePeak);
        assert_eq!(resolved.detail, spin.detail);
    }

    /// An inferred spin reaches the launch conditions, and `spin_source` is what says it was solved
    /// rather than read — the one corpus vector that gets here.
    #[test]
    fn an_inferred_spin_is_flown_and_labelled_as_solved() {
        let mut shot = a_flyable_shot();
        let resolved =
            flight_for_shot(&shot, Some(30.5), None, load_flight_model(), DEFAULT_STEP_S)
                .expect("a flyable shot");
        assert_eq!(resolved.spin_source, Some(SpinSource::Inferred));
        let solved = resolved.spin_rpm().expect("the solve produced one");
        assert_eq!(
            Some(solved),
            resolved.spin.as_ref().and_then(|s| s.spin_rpm),
            "the launch is flown on the spin the solve chose and not a second number"
        );
        shot.spin_rate = Some(5991.0);
    }

    /// The start line is the launch direction, and an absent one is planar rather than a refusal —
    /// Python's `or 0.0`, which is falsy and agrees with this on every value it can take.
    #[test]
    fn an_absent_start_line_flies_straight_down_the_target_line() {
        let mut shot = a_flyable_shot();
        shot.launch_direction = None;
        shot.spin_rate = Some(5991.0);
        let resolved = flight_for_shot(&shot, None, None, load_flight_model(), DEFAULT_STEP_S)
            .expect("a flyable shot");
        assert_eq!(
            resolved.launch.expect("ready to fly").launch_direction_deg,
            0.0
        );
    }
}
