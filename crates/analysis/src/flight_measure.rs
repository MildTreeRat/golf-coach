//! The six numbers a simulated flight contributes, and the two ways it declines to. [M22 P8b]
//!
//! [`crate::flight_infer::flight_for_shot`] resolves a stored shot as far as the screen and the bag
//! allow and **stops at the launch conditions**; [`crate::flight::simulate_flight`] flies them. This
//! module is the join, and it is the layer that decides what of the result becomes a `Measurement` —
//! a narrower question than what a viewer may draw, and the reason the two are not the same function.
//!
//! ADR-027 §Decision 6 names the six: `flight_carry_yds`, `flight_apex_yds`,
//! `flight_descent_angle_deg`, `flight_time_s`, `flight_landing_offline_yds` and `flight_spin_rpm`,
//! under the source [`FLIGHT_SOURCE`] — the fourth, after `pose:face_on`, `launch_monitor:*` and
//! `population:golfdb`.
//!
//! # Distinct names, and the rule underneath them
//!
//! `baseline.pooled_samples` groups by name, so a predicted carry sharing `carry_distance_yds` would
//! build a personal mean over a mixture of a measurement and a model output. The `flight_` prefix is
//! what prevents that, and the rule it encodes is **one provenance per name** — which costs a number
//! twice here rather than buying one:
//!
//! - **`flight_landing_offline_yds` is recorded only when the curve was actually drawn.** With the
//!   spin axis unresolved the flight is flown in the vertical plane and its landing offline comes out
//!   as `carry * sin(start line)` to the last bit, which is `start_line_offline_yds` reached by forty
//!   flights instead of one sine. [`crate::flight_infer::ShotFlight::curve_is_drawn`] is the bit that
//!   says when.
//! - **`flight_spin_rpm` is recorded only when the spin was inferred.** ⚠️ A correction to §Decision
//!   6, which named the measurement without distinguishing the two spins that can fill it. A spin the
//!   screen printed is a reading off a launch monitor; a solved one is *the spin this integrator
//!   needs in order to agree with HD Golf's carry*. Pooling the two under one name would average a
//!   measured 5,991 rpm with a solved 2,924 rpm and print the result as one golfer's spin rate.
//!
//! The other four are model outputs however the spin arrived, so they record on every flight.
//!
//! # The two refusals, and why they are two entries and not six
//!
//! A refused flight withholds five names at once, for one cause. It is reported as **one**
//! `UnscoredCheckpoint` named `flight_carry_yds` — the quantity the model exists to produce — because
//! `mcp.query.get_session_summary` counts unscored entries by name, and one missing flight counted
//! six times would report a session as six times more broken than it is.
//!
//! The axis is the second entry and genuinely separate: **a refused axis does not stop the flight**,
//! so the other five record and only `flight_landing_offline_yds` goes missing.
//!
//! **Nothing here is a checkpoint** and nothing enters `overall_score` — no band, no `ranges.json`
//! row, no `CHECKPOINT_REGISTRY` entry. `UnscoredCheckpoint` is reused because the absence being
//! *named* is the whole point of that shape.
//!
//! # What is gated
//!
//! The `flight` stage's `unscored` list on all fifteen corpus vectors, and the `measurements` stage's
//! `flight` group. Between them the fifteen reach: five flights (four printed spins, one solved), ten
//! refusals under `flight_carry_yds`, and one `flight_landing_offline_yds` entry — the vector where
//! the spin solved and the axis did not. **`flight_spin_rpm` records on exactly one of the twenty-one
//! vectors**, and `flight_landing_offline_yds` on four, so the two conditions that make this module
//! more than a tuple unpack are each gated by a single-digit sample and by the unit tests below.
//!
//! # Why the `Err` arms exist
//!
//! `simulate_flight` refuses launch conditions there is no flight to integrate from, and names the
//! caller as the boundary that owns the check. `flight_for_shot` owns two of the five guards, and the
//! other three — a negative spin, a launch direction or a spin axis past a quarter turn — can still
//! arrive from a parse rather than from a swing. Reaching `analyze_swing` they would take the *whole*
//! swing down, discarding pose that had already run and every checkpoint that had already scored. So
//! they are turned into a stated refusal here. Nothing about a launch monitor's tile justifies losing
//! the swing.
//!
//! # The printed comparison, ported with its first Rust caller [M29 P14]
//!
//! [`compare_to_printed`], [`PrintedComparison`], `_reading` and [`circular_carry_note`] are M15
//! P16's viewer-honesty layer: a simulated number set beside the printed one it is about, with the
//! sentence saying whether the gap is a check. `conformance.py::run_vector` reaches none of them, so
//! M22 P8b left them with their callers (ADR-032 §7), then `scripts/simulate_flight.py`, the flight
//! page and, unnoted there, `mcp/flight.py`, whose `comparison` block is built from them.
//! `golf-mcp`'s `simulate_flight` is that third caller ported, so they are here now, held to the
//! recorded answers in `spec/vectors/mcp/`; `golf-lab flight` (the M29 plan's P28) is the second.
//! The program plan's inventory (R5) still counts them among the 130 lines that are not ported.
//! That row predates the MCP port.

use contracts::unscored::{UnscoredCheckpoint, UnscoredReason};

use crate::benchmarks::flight_model::FlightModel;
use crate::flight::{simulate_flight, FlightResult, GATE_AGREEMENT_FRACTION};
use crate::flight_infer::{flight_for_shot, ShotFlight, SpinSource};
use crate::shot_measure::SHOT_MEASUREMENTS;
use contracts::golfer::Handedness;
use contracts::shot::ShotData;

/// `Measurement.source` for everything this module produces.
///
/// The fourth provenance in the repo, and the first that is not a reading of anything — `pose:`,
/// `launch_monitor:` and `population:` all name an instrument or a corpus, and this one names a
/// *model*, versioned with the artifact it evaluates (`benchmarks/flight_model_v1.json`) so a
/// re-sourced coefficient table gets a new name rather than silently changing what the old one meant.
pub const FLIGHT_SOURCE: &str = "model:flight_v1";

/// One stored shot flown, or the reason it was not — with the resolution kept either way.
///
/// [`Self::resolved`] survives a refusal on purpose: it carries which of the seven spin-solve shapes
/// the carry landed on and the cap the answer would have sat under, and that is most of what a reader
/// wants when there is no number.
#[derive(Debug, Clone, PartialEq)]
pub struct FlownShot {
    /// `None` only when resolving the launch conditions itself refused — a parse this model cannot
    /// fly. Present on every ordinary refusal.
    pub resolved: Option<ShotFlight>,
    /// The flight, or `None` when [`Self::reason`] says why there is none.
    pub flight: Option<FlightResult>,
    /// One of `contracts::unscored::INFERENCE_REASONS`. Never `SpinAxisUnresolved`, which is a flight
    /// drawn straight rather than a flight refused.
    pub reason: Option<UnscoredReason>,
    pub detail: String,
}

impl FlownShot {
    pub fn flew(&self) -> bool {
        self.flight.is_some()
    }

    /// Whether this flight bends, and so whether its landing point is a landing point.
    pub fn curve_is_drawn(&self) -> bool {
        self.resolved
            .as_ref()
            .is_some_and(ShotFlight::curve_is_drawn)
    }
}

/// Resolve one stored shot's launch conditions and fly them, or say why neither happened. `fly_shot`.
///
/// `loft_deg` and `handedness` have no defaults for the reason `flight_for_shot` gives: both have a
/// plausible-looking wrong answer and both come from a different artifact than the shot does.
///
/// **This is where the Python's two `except ValueError` arms live**, and in Rust they are the two
/// `Err` matches below. The first covers the solve, which flies the ball dozens of times looking for
/// a carry, so an unflyable start line or spin axis surfaces from in there rather than from the
/// `simulate_flight` after it.
pub fn fly_shot(
    shot: &ShotData,
    loft_deg: Option<f64>,
    handedness: Option<Handedness>,
    model: &FlightModel,
    step_s: f64,
) -> FlownShot {
    let resolved = match flight_for_shot(shot, loft_deg, handedness, model, step_s) {
        Ok(resolved) => resolved,
        Err(refusal) => {
            return FlownShot {
                resolved: None,
                flight: None,
                reason: Some(UnscoredReason::NoLaunchConditions),
                detail: refusal.message,
            }
        }
    };

    let Some(launch) = resolved.launch else {
        // Python asserts that a shot with no launch conditions says why. Here the assert has no
        // counterpart to write: `flight_for_shot` returns `reason: Some(_)` on every path that
        // leaves `launch` empty, and the `unwrap_or` below is the only thing that could paper over a
        // regression — so it names `Unrecorded`, which is the one reason that is *not* an inference
        // reason and would therefore be visible in a vector rather than plausible.
        let reason = resolved.reason.unwrap_or(UnscoredReason::Unrecorded);
        return FlownShot {
            detail: resolved.detail.clone(),
            resolved: Some(resolved),
            flight: None,
            reason: Some(reason),
        };
    };

    match simulate_flight(&launch, model, step_s) {
        Ok(flight) => FlownShot {
            detail: resolved.detail.clone(),
            resolved: Some(resolved),
            flight: Some(flight),
            reason: None,
        },
        Err(refusal) => FlownShot {
            resolved: Some(resolved),
            flight: None,
            reason: Some(UnscoredReason::NoLaunchConditions),
            detail: refusal.message,
        },
    }
}

fn carry(flown: &FlownShot) -> Option<f64> {
    flown.flight.as_ref().map(FlightResult::carry_yds)
}

fn apex(flown: &FlownShot) -> Option<f64> {
    flown.flight.as_ref().map(FlightResult::apex_yds)
}

fn descent_angle(flown: &FlownShot) -> Option<f64> {
    flown.flight.as_ref().map(|f| f.descent_angle_deg)
}

fn flight_time(flown: &FlownShot) -> Option<f64> {
    flown.flight.as_ref().map(|f| f.flight_time_s)
}

/// Where the ball **finished**, and only when the model actually bent it.
///
/// The module doc says why a planar flight's offline is withheld rather than recorded: it is
/// `start_line_offline_yds` again, to the last bit, under a second name.
fn landing_offline(flown: &FlownShot) -> Option<f64> {
    if !flown.curve_is_drawn() {
        return None;
    }
    flown.flight.as_ref().map(FlightResult::landing_offline_yds)
}

/// The solved spin, and never a printed one — the module doc says why.
///
/// Reads `spin_source` rather than `ShotData.spin_rate` so there is one definition of which spin the
/// flight was drawn with, and it is the one `flight_infer` chose.
fn spin(flown: &FlownShot) -> Option<f64> {
    flown.flight.as_ref()?;
    let resolved = flown.resolved.as_ref()?;
    if resolved.spin_source != Some(SpinSource::Inferred) {
        return None;
    }
    resolved.spin_rpm()
}

/// How to read one number off a flown shot, and what the resulting number is.
///
/// [`crate::shot_measure::ShotMeasurement`]'s shape with a different argument, and the argument is
/// the point: the readers take the **flown shot** rather than the launch conditions, because six
/// numbers off one flight must not fly the ball six times — `SHOT_MEASUREMENTS`' one-function-per-
/// number shape would re-integrate the whole trajectory six times and the spin solve behind it dozens
/// more.
pub struct FlightMeasurement {
    pub measure: fn(&FlownShot) -> Option<f64>,
    pub unit: &'static str,
    /// **Every detail string says the number is simulated**, in the string itself rather than by
    /// relying on the `flight_` prefix or the source: `analysis.baseline` averages by name, unit and
    /// detail without knowing what a source is, and `mcp.query` prints the detail beside the value.
    pub detail: &'static str,
}

/// Name -> how to read it, for everything one simulated flight contributes.
///
/// A slice and not a map, in the Python dict's insertion order, for the reason
/// [`crate::measure::POSE_MEASUREMENTS`] gives. None of them is judged: no band, no `ranges.json`
/// row, no `CHECKPOINT_REGISTRY` entry, and no `contracts.dispersion.METRIC_TARGETS` row either — a
/// model output must not draw a scatter finding.
pub static FLIGHT_MEASUREMENTS: &[(&str, FlightMeasurement)] = &[
    (
        "flight_carry_yds",
        FlightMeasurement {
            measure: carry,
            unit: "yards",
            detail: "SIMULATED carry from the printed launch conditions; not the launch monitor's \
                     own carry, which is carry_distance_yds",
        },
    ),
    (
        "flight_apex_yds",
        FlightMeasurement {
            measure: apex,
            unit: "yards",
            detail: "SIMULATED peak height above the tee; the screen prints no apex tile at all",
        },
    ),
    (
        "flight_descent_angle_deg",
        FlightMeasurement {
            measure: descent_angle,
            unit: "degrees",
            detail:
                "SIMULATED angle below the horizontal at landing, read off the landing velocity",
        },
    ),
    // Seconds rather than the `ms` the two pose durations use, which is the name ADR-027 gave it and
    // is also what keeps it out of `measure.FPS_DEPENDENT_MEASUREMENTS` — that set is derived from
    // the unit `ms`, and a hang time integrated by a model has no frame rate to be dependent on.
    (
        "flight_time_s",
        FlightMeasurement {
            measure: flight_time,
            unit: "seconds",
            detail: "SIMULATED hang time from launch to landing",
        },
    ),
    (
        "flight_landing_offline_yds",
        FlightMeasurement {
            measure: landing_offline,
            unit: "yards",
            detail: "SIMULATED yards right (+) or left of target the ball FINISHED; recorded only \
                     when the spin axis resolved, so unlike start_line_offline_yds this is a \
                     landing point",
        },
    ),
    (
        "flight_spin_rpm",
        FlightMeasurement {
            measure: spin,
            unit: "rpm",
            detail: "SOLVED backspin - the spin this model needs to agree with the printed carry, \
                     capped and recorded only when the screen printed none; a printed spin is on \
                     the shot, not here",
        },
    ),
];

/// The one or two entries naming what this flight could not produce, and why. `flight_unscored`.
///
/// Empty when the flight flew and its curve was drawn, which on the corpus today is no shot at all.
/// See the module doc for why a refused flight is one entry under `flight_carry_yds` and not five.
pub fn flight_unscored(flown: &FlownShot) -> Vec<UnscoredCheckpoint> {
    if let Some(reason) = flown.reason {
        return vec![UnscoredCheckpoint {
            name: "flight_carry_yds".to_string(),
            reason,
            // The other four names are withheld by the same cause; saying so here is what stops a
            // reader taking their absence for a second, unexplained failure.
            detail: format!(
                "{} - no flight, so none of the flight_* measurements recorded",
                flown.detail
            ),
        }];
    }
    if let Some(resolved) = flown.resolved.as_ref() {
        if let Some(reason) = resolved.axis.reason {
            return vec![UnscoredCheckpoint {
                name: "flight_landing_offline_yds".to_string(),
                reason,
                detail: resolved.axis.detail.clone(),
            }];
        }
    }
    Vec::new()
}

/// Which simulated name may be set beside which printed one, and on which drawing axis the quantity
/// lives. `_COMPARISON_PAIRS`. [M29 P14]
///
/// **Two pairs and no third, and the four missing are a fact about the screen**: `flight_apex_yds`
/// has `apex_height` opposite it and no stored shot carries one, `flight_descent_angle_deg` and
/// `flight_time_s` have no tile at all, and `flight_spin_rpm` records only a solved spin, so a shot
/// with a printed spin to check it against is exactly a shot where it is not recorded ([`spin`]).
/// The quantity is here rather than on a page because down range and offline are facts about the
/// numbers, and a surface deciding them from a name would be pattern-matching on `_yds`.
pub const COMPARISON_PAIRS: [(&str, &str, &str); 2] = [
    ("flight_carry_yds", "carry_distance_yds", "down_range"),
    (
        "flight_landing_offline_yds",
        "start_line_offline_yds",
        "offline",
    ),
];

/// One simulated number beside the printed one it is about, and whether that is a check.
/// `PrintedComparison`, field for field.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintedComparison {
    pub simulated_name: &'static str,
    pub measured_name: &'static str,
    /// `"down_range"` or `"offline"` ([`COMPARISON_PAIRS`]).
    pub quantity: &'static str,
    pub simulated: f64,
    /// What the launch monitor printed, read through [`SHOT_MEASUREMENTS`], so "measured" means what
    /// it means everywhere else.
    pub measured: f64,
    /// `simulated - measured`. A number; [`Self::comparable`] says whether it is an *error*.
    pub difference: f64,
    /// False when the difference cannot be read as agreement or disagreement: the printed number
    /// was the simulation's input, or the two are not one quantity.
    pub comparable: bool,
    /// The sentence a surface must print beside the row, with its numbers derived here.
    pub reading: String,
}

/// Every simulated number this shot printed a counterpart for, with what the gap means.
/// `compare_to_printed`. [M29 P14]
///
/// **The carry is a check on exactly half the flights the corpus can produce**: on a printed spin
/// the printed carry is independent, and on a solved one it was the solve's input, which the flight
/// reproduces to the root search's tolerance ([`circular_carry_note`]). **The offline pair is never
/// a check**: the printed side is the start line projected out, and the gap is the curve. Empty for
/// a refused flight, which owes its `reason` instead.
pub fn compare_to_printed(flown: &FlownShot, shot: &ShotData) -> Vec<PrintedComparison> {
    let Some(flight) = flown.flight.as_ref() else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for (simulated_name, measured_name, quantity) in COMPARISON_PAIRS {
        let read_simulated = registry_row(FLIGHT_MEASUREMENTS, simulated_name).measure;
        let read_measured = registry_row(SHOT_MEASUREMENTS, measured_name).measure;
        let (Some(simulated), Some(measured)) = (read_simulated(flown), read_measured(shot)) else {
            continue;
        };
        let (comparable, reading) = reading(simulated_name, flown, flight, simulated, measured);
        rows.push(PrintedComparison {
            simulated_name,
            measured_name,
            quantity,
            simulated,
            measured,
            difference: simulated - measured,
            comparable,
            reading,
        });
    }
    rows
}

/// A registry row by its name. The pairs name rows both registries hold, which the unit test pins,
/// so a miss is a renamed row, and a panic naming it is the answer a renamer needs.
fn registry_row<'a, T>(registry: &'a [(&str, T)], name: &str) -> &'a T {
    registry
        .iter()
        .find(|(row, _)| *row == name)
        .map(|(_, row)| row)
        .unwrap_or_else(|| panic!("{name} is a comparison pair's name and no registry row"))
}

/// The sentence for one pair, and whether the difference in it is an error at all. `_reading`.
///
/// Python's `{:+.2f}` is [`pyfmt::signed_fixed`], `{:g}` is [`pyfmt::g`], `{:.2%}` is
/// [`pyfmt::percent`], and `{:+.2%}` is the last with the sign, by [`signed_percent`].
fn reading(
    simulated_name: &str,
    flown: &FlownShot,
    flight: &FlightResult,
    simulated: f64,
    measured: f64,
) -> (bool, String) {
    let difference = simulated - measured;
    if simulated_name == "flight_landing_offline_yds" {
        // Split rather than attributed whole to the curve: the two numbers are read off different
        // carries, so the start line leans the carry row's own disagreement into this one.
        // `landing_offline_m` is `carry * sin(direction) + curvature * cos(direction)`, and the
        // second term is what is left once the first is taken out, so the split is exact.
        // `to_radians` is CPython's `math.radians`, one multiply by `pi / 180`.
        let direction = flight.launch.launch_direction_deg;
        let bend = flight.curvature_yds() * direction.to_radians().cos();
        return (
            false,
            format!(
                "not an error and not a comparison: the printed number is where the ball \
                 *started*, projected out to the carry, and the simulated one is where this model \
                 has it finishing. Most of the gap is the bend the screen never measured, {} yd of \
                 it; the remaining {} yd is the carry row above leaning on a start line {} deg off \
                 target, and it is not a second finding.",
                pyfmt::signed_fixed(bend, 2),
                pyfmt::signed_fixed(difference - bend, 2),
                pyfmt::g(direction)
            ),
        );
    }
    let printed_spin = flown
        .resolved
        .as_ref()
        .is_some_and(|resolved| resolved.spin_source == Some(SpinSource::Measured));
    if !printed_spin {
        return (false, circular_carry_note(Some(difference)));
    }
    (
        true,
        format!(
            "the screen printed the spin this was flown on, so the carry beside it is an \
             independent measurement and this difference is the model's error on this shot: {} \
             yd, {} against a gate measured at +/-{}.",
            pyfmt::signed_fixed(difference, 2),
            signed_percent(difference / measured, 2),
            pyfmt::percent(GATE_AGREEMENT_FRACTION, 2)
        ),
    )
}

/// Python's `f"{x:+.N%}"`: [`pyfmt::percent`] with the sign always written, composed from
/// [`pyfmt::signed_fixed`] as `percent` is from `fixed`, which is CPython's own order (multiply by
/// 100, render `+.Nf`, append `%`). Composed here rather than added to `pyfmt`, because every
/// `pyfmt` function is gated by a `format/` table and that family does not change in M29; the
/// digits are `signed_fixed`'s, which `fixed.json` already gates. Public since M29 P28, because
/// `golf-lab flight` prints the same `+.2%` in its gate and stored-shot tables, and a second copy
/// of the composition is a second place the order could slip.
pub fn signed_percent(x: f64, precision: usize) -> String {
    format!("{}%", pyfmt::signed_fixed(x * 100.0, precision))
}

/// Why a solved flight's carry is not a check, for one row or for a whole column.
/// `circular_carry_note`. [M29 P14]
///
/// `Some(difference)` is one flight's row, and quotes the residual, because a reader looking at
/// `+0.001 yd` deserves to be told what it is before deciding it is agreement. `None` is a column
/// of them, `golf-lab flight --shots`' footer.
pub fn circular_carry_note(difference: Option<f64>) -> String {
    const CLAIM: &str = "the printed carry is the *input* to the spin solve, so the flown carry \
                         reproduces it by construction";
    match difference {
        None => format!(
            "there is no error column here and there cannot be one: {CLAIM}, and a column of \
             zeros would read as a validation of the thing that was assumed."
        ),
        Some(difference) => format!(
            "{CLAIM} ({} yd, which is the root search's own tolerance and not an agreement). \
             Nothing here checks the model; what the solve produced is a spin, and its cap is what \
             bounds that.",
            pyfmt::signed_fixed(difference, 3)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::flight_model::load_flight_model;
    use crate::flight::DEFAULT_STEP_S;
    use crate::testing::a_shot;

    fn a_flyable_shot() -> ShotData {
        let mut shot = a_shot();
        shot.ball_speed = Some(89.8);
        shot.launch_angle = Some(22.4);
        shot.launch_direction = Some(2.8);
        shot.carry_distance = Some(126.1);
        shot
    }

    fn flown(shot: &ShotData, loft_deg: Option<f64>, handedness: Option<Handedness>) -> FlownShot {
        fly_shot(
            shot,
            loft_deg,
            handedness,
            load_flight_model(),
            DEFAULT_STEP_S,
        )
    }

    /// The registry's names and their order, which `docs/CONFORMANCE.md` §3 compares positionally.
    #[test]
    fn the_registry_is_adr_027s_six_in_the_pythons_order() {
        let names: Vec<&str> = FLIGHT_MEASUREMENTS.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            [
                "flight_carry_yds",
                "flight_apex_yds",
                "flight_descent_angle_deg",
                "flight_time_s",
                "flight_landing_offline_yds",
                "flight_spin_rpm",
            ]
        );
        // And every detail says SIMULATED or SOLVED in the string itself, which is the one line about
        // a model output a reader is guaranteed to see.
        for (name, row) in FLIGHT_MEASUREMENTS {
            assert!(
                row.detail.starts_with("SIMULATED") || row.detail.starts_with("SOLVED"),
                "{name}: {}",
                row.detail
            );
        }
    }

    /// A printed spin with a printed axis: five of the six record, and the sixth is the *spin*,
    /// withheld because it was read rather than solved.
    #[test]
    fn a_measured_spin_records_five_numbers_and_never_the_spin() {
        let mut shot = a_flyable_shot();
        shot.spin_rate = Some(5991.0);
        shot.spin_axis = Some(2.5);
        let flown = flown(&shot, None, Some(Handedness::Right));
        assert!(flown.flew() && flown.curve_is_drawn());
        let recorded: Vec<&str> = FLIGHT_MEASUREMENTS
            .iter()
            .filter(|(_, row)| (row.measure)(&flown).is_some())
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(
            recorded,
            [
                "flight_carry_yds",
                "flight_apex_yds",
                "flight_descent_angle_deg",
                "flight_time_s",
                "flight_landing_offline_yds",
            ]
        );
        assert!(flight_unscored(&flown).is_empty());
    }

    /// A solved spin with no axis: the spin records and the offline does not, which is the pair of
    /// conditions this module exists for and the only corpus vector that shows either.
    #[test]
    fn a_solved_spin_records_the_spin_and_withholds_the_offline() {
        let flown = flown(&a_flyable_shot(), Some(30.5), None);
        assert!(flown.flew());
        assert!(!flown.curve_is_drawn(), "no axis was printed");
        assert_eq!(landing_offline(&flown), None);
        let solved = spin(&flown).expect("the spin was solved, so it is a model output");
        assert!(solved > 0.0);
        // The offline the flight *did* compute is not zero-ish nonsense — it is simply the start line
        // projected out, which is why recording it under a second name is the hazard.
        assert!(flown.flight.as_ref().unwrap().landing_offline_yds() != 0.0);
    }

    /// **A refused flight is one entry, not five**, and its detail says the other four went with it.
    #[test]
    fn a_refused_flight_is_one_unscored_entry_under_the_carry() {
        let mut shot = a_flyable_shot();
        shot.carry_distance = Some(400.0);
        let flown = flown(&shot, None, None);
        assert!(!flown.flew());
        for (name, row) in FLIGHT_MEASUREMENTS {
            assert_eq!((row.measure)(&flown), None, "{name}");
        }
        let unscored = flight_unscored(&flown);
        assert_eq!(unscored.len(), 1);
        assert_eq!(unscored[0].name, "flight_carry_yds");
        assert_eq!(unscored[0].reason, UnscoredReason::CarryUnreachable);
        assert!(unscored[0]
            .detail
            .ends_with(" - no flight, so none of the flight_* measurements recorded"));
        assert!(unscored[0].detail.starts_with(&flown.detail));
    }

    /// And a flown shot whose axis was refused is the *other* entry, which is the asymmetry the two
    /// reason fields exist for: the flight stands and only the offline goes missing.
    #[test]
    fn a_refused_axis_is_its_own_entry_and_the_flight_still_counts() {
        let mut shot = a_flyable_shot();
        shot.spin_rate = Some(5991.0);
        let flown = flown(&shot, None, Some(Handedness::Right));
        assert!(flown.flew());
        let unscored = flight_unscored(&flown);
        assert_eq!(unscored.len(), 1);
        assert_eq!(unscored[0].name, "flight_landing_offline_yds");
        assert_eq!(unscored[0].reason, UnscoredReason::SpinAxisUnresolved);
        assert_eq!(
            unscored[0].detail,
            flown.resolved.as_ref().unwrap().axis.detail
        );
    }

    /// **The refusal precedence is the flight's, not the axis's.** Both are refused on ten of the
    /// fifteen corpus vectors and only the carry entry is recorded — a port that checked the axis
    /// first would emit the wrong name on two thirds of the corpus.
    #[test]
    fn a_flight_refused_with_an_unresolved_axis_reports_only_the_carry() {
        let mut shot = a_flyable_shot();
        shot.carry_distance = Some(400.0);
        let flown = flown(&shot, None, None);
        assert_eq!(
            flown.resolved.as_ref().unwrap().axis.reason,
            Some(UnscoredReason::SpinAxisUnresolved)
        );
        let unscored = flight_unscored(&flown);
        assert_eq!(unscored.len(), 1);
        assert_eq!(unscored[0].name, "flight_carry_yds");
    }

    /// **An unflyable parse is a stated refusal and not a lost swing**, which is why this module
    /// catches at all. A spin axis past a quarter turn is one of the three guards `flight_for_shot`
    /// does not own, and it surfaces from inside the *solve* — dozens of flights in.
    #[test]
    fn an_unflyable_parse_refuses_the_flight_rather_than_taking_the_swing_down() {
        let mut shot = a_flyable_shot();
        shot.spin_axis = Some(120.0);
        let flown = flown(&shot, None, Some(Handedness::Right));
        assert_eq!(flown.resolved, None, "the resolution itself refused");
        assert_eq!(flown.reason, Some(UnscoredReason::NoLaunchConditions));
        assert!(
            !flown.detail.is_empty(),
            "the integrator's own message carries through"
        );
        // And it still produces the one entry a reader is owed.
        let unscored = flight_unscored(&flown);
        assert_eq!(unscored.len(), 1);
        assert_eq!(unscored[0].name, "flight_carry_yds");
    }

    // --- the printed comparison [M29 P14] ------------------------------------------------------
    //
    // Each expected row is frozen Python's `compare_to_printed` on the same shot, read off
    // `golf_coach.analysis.flight_measure` (M29 P14): the floats exactly, and the sentences to the
    // character.

    /// Both pairs name a row their registry holds, so `registry_row` cannot miss.
    #[test]
    fn every_comparison_pair_names_a_row_of_both_registries() {
        for (simulated, measured, quantity) in COMPARISON_PAIRS {
            assert!(FLIGHT_MEASUREMENTS
                .iter()
                .any(|(name, _)| *name == simulated));
            assert!(SHOT_MEASUREMENTS.iter().any(|(name, _)| *name == measured));
            assert!(matches!(quantity, "down_range" | "offline"));
        }
    }

    /// A printed spin and axis: the carry is a check, signed in yards and in percent, and the
    /// offline row is split into the bend and the start line's lean.
    #[test]
    fn a_printed_flight_is_checked_on_the_carry_and_never_on_the_offline() {
        let mut shot = a_flyable_shot();
        shot.spin_rate = Some(5991.0);
        shot.spin_axis = Some(2.5);
        let rows = compare_to_printed(&flown(&shot, None, Some(Handedness::Right)), &shot);
        assert_eq!(rows.len(), 2);
        let carry = &rows[0];
        assert_eq!(
            (carry.simulated_name, carry.measured_name, carry.quantity),
            ("flight_carry_yds", "carry_distance_yds", "down_range")
        );
        assert_eq!(carry.simulated, 121.82030959863656);
        assert_eq!(carry.measured, 126.1);
        assert_eq!(carry.difference, -4.279690401363439);
        assert!(carry.comparable);
        assert_eq!(
            carry.reading,
            "the screen printed the spin this was flown on, so the carry beside it is an \
             independent measurement and this difference is the model's error on this shot: \
             -4.28 yd, -3.39% against a gate measured at +/-2.59%."
        );
        let offline = &rows[1];
        assert_eq!(
            (
                offline.simulated_name,
                offline.measured_name,
                offline.quantity
            ),
            (
                "flight_landing_offline_yds",
                "start_line_offline_yds",
                "offline"
            )
        );
        assert_eq!(offline.simulated, 7.662057811979329);
        assert_eq!(offline.measured, 6.1599559712268315);
        assert_eq!(offline.difference, 1.5021018407524975);
        assert!(!offline.comparable);
        assert_eq!(
            offline.reading,
            "not an error and not a comparison: the printed number is where the ball *started*, \
             projected out to the carry, and the simulated one is where this model has it \
             finishing. Most of the gap is the bend the screen never measured, +1.71 yd of it; \
             the remaining -0.21 yd is the carry row above leaning on a start line 2.8 deg off \
             target, and it is not a second finding."
        );
    }

    /// A solved spin: the carry row is the circular note with its residual, and there is no
    /// offline row, because a planar flight records no landing offline to pair.
    #[test]
    fn a_solved_flight_says_its_carry_is_its_input() {
        let shot = a_flyable_shot();
        let rows = compare_to_printed(&flown(&shot, Some(30.5), None), &shot);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].difference, 0.0005416737286481066);
        assert!(!rows[0].comparable);
        assert_eq!(
            rows[0].reading,
            circular_carry_note(Some(rows[0].difference))
        );
        assert_eq!(
            rows[0].reading,
            "the printed carry is the *input* to the spin solve, so the flown carry reproduces it \
             by construction (+0.001 yd, which is the root search's own tolerance and not an \
             agreement). Nothing here checks the model; what the solve produced is a spin, and its \
             cap is what bounds that."
        );
        assert_eq!(
            circular_carry_note(None),
            "there is no error column here and there cannot be one: the printed carry is the \
             *input* to the spin solve, so the flown carry reproduces it by construction, and a \
             column of zeros would read as a validation of the thing that was assumed."
        );
    }

    /// A refused flight has nothing to set beside the screen.
    #[test]
    fn a_refused_flight_compares_nothing() {
        let mut shot = a_flyable_shot();
        shot.carry_distance = Some(400.0);
        assert!(compare_to_printed(&flown(&shot, None, None), &shot).is_empty());
    }

    /// The same guard reached with a printed spin, where there is no solve in front of it: the
    /// refusal comes out of `simulate_flight` instead, and lands on the same reason.
    #[test]
    fn an_unflyable_printed_shot_refuses_out_of_the_integrator() {
        let mut shot = a_flyable_shot();
        shot.spin_rate = Some(-10.0);
        let flown = flown(&shot, None, None);
        assert!(
            flown.resolved.is_some(),
            "the resolution succeeded; the flight did not"
        );
        assert_eq!(flown.reason, Some(UnscoredReason::NoLaunchConditions));
        assert!(flown.flight.is_none());
    }
}
