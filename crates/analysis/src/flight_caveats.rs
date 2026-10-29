//! What a simulated flight may not be quoted without. `src/golf_coach/analysis/flight_caveats.py`.
//! [M29 P14]
//!
//! Five sentences, of which every surface that draws a flight owes the first two always and the other
//! three when they apply. Python wrote them for `scripts/simulate_flight.py` and moved them here once
//! the flight had a second reader, because load-bearing prose that exists twice goes stale in one
//! place. Their Rust callers are `golf-mcp`'s `simulate_flight` (M29 P14) and `golf-lab flight`
//! (P28), so they live where both can reach them: here, beside the shapes they read
//! ([`FlightResult`], [`FlightModel`], [`ShotFlight`]), which `contracts` may not hold.
//!
//! **Every number in them is derived, never typed** (`docs/CODE_STANDARDS.md` R4): the clamp edges
//! come off the coefficient table, the agreement off [`GATE_AGREEMENT_FRACTION`], the spin cap off
//! the `CarryWindow` that shot measured. A re-sourced table moves all three and the prose follows.
//! The *47% low* in [`inferred_caveat`] is the one number Python typed, and it is typed here too:
//! it is M15 P9's measurement of a shot, not a quantity this module can derive.
//!
//! # They compose, and the composition is the shared part
//!
//! [`caveats_for`] is what a surface calls, not the builders one by one: *which* caveats a flight
//! owes is a rule about the flight, and a second surface deriving it again is a second place to
//! derive it wrongly. Each is one paragraph with no line breaks, Python's `_said`, because a JSON
//! payload and a terminal want different shapes of one sentence; wrapping is the terminal's.
//!
//! # Python's formatting, to the character
//!
//! `{:g}` is [`pyfmt::g`], `{:.0f}` is [`pyfmt::fixed`] and `{:.2%}` is [`pyfmt::percent`], so the
//! digits are the ones `spec/vectors/format/` gates. Python's triple-quoted paragraphs are written
//! here with `\` continuations, which drop the line break and the next line's indent, so each
//! sentence is already what `" ".join(body.split())` leaves: one space between words, none at
//! either end. `spec/vectors/mcp/` holds four of the five sentences as frozen Python wrote them
//! (fully clamped, the gate, planar and inferred); the unit tests below hold the rest to
//! Python's text, worked from the frozen module.

use crate::benchmarks::flight_model::FlightModel;
use crate::flight::{FlightResult, GATE_AGREEMENT_FRACTION};
use crate::flight_infer::{ShotFlight, SpinSource};

/// Exact, as the international foot is 0.3048 m. A reciprocal multiplied, as Python's
/// `_M_TO_FEET = 1.0 / 0.3048` is, rather than a division by 0.3048, which differs in the last bit.
const M_TO_FEET: f64 = 1.0 / 0.3048;

/// Whether this flight read the published table or was flown off the end of it. `clamp_caveat`.
pub fn clamp_caveat(result: &FlightResult, model: &FlightModel) -> String {
    let table = &model.coefficients;
    let (low, high) = (
        pyfmt::g(table.spin_ratio_min()),
        pyfmt::g(table.spin_ratio_max()),
    );
    if result.fully_clamped() {
        return format!(
            "every point of this flight sat above the published table, which stops at a spin \
             ratio of {high} and was measured on a *driver*. Its end row was held rather than \
             extended, so lift and drag were one constant pair the whole way — and since spin \
             reaches the flight only through the spin ratio and the spin ratio only through those \
             two coefficients, the spin above the clamp does not reach this carry at all. The same \
             ball at 5,200 rpm and at 30,000 rpm flies exactly this far."
        );
    }
    if result.clamped_points > 0 {
        return format!(
            "{} of {} points fell outside the published table ({low} to {high}) and read a held \
             end row rather than a measured one. Over that part of the flight, spin has no route \
             into the answer.",
            result.clamped_points,
            result.points.len()
        );
    }
    format!(
        "every point read a measured row of the table ({low} to {high}), which one shot on disk \
         manages and no other — and that shot is the one whose spin was *solved* rather than \
         printed. The two go together rather than coinciding: a carry the solve can invert is a \
         carry still responding to spin, and spin still reaching the answer is exactly a flight \
         below the clamp."
    )
}

/// The agreement and the thing the agreement hides, which are never quoted apart. `gate_caveat`.
pub fn gate_caveat() -> String {
    format!(
        "the model agrees with HD Golf to +/-{} on the two shots where a full launch-condition set \
         and an independent carry are both on disk (`--gate`) — while ranking those two shots \
         backwards. That percentage is therefore the size of what this model cannot see, not a \
         calibration error, and a carry from here is not \"accurate to {}\".",
        pyfmt::percent(GATE_AGREEMENT_FRACTION, 2),
        pyfmt::percent(GATE_AGREEMENT_FRACTION, 1)
    )
}

/// What a flight with no spin axis does and does not say about where the ball finished.
/// `planar_caveat`, whose `count` is a table's: `golf-mcp` always says it of one flight.
pub fn planar_caveat(count: usize) -> String {
    let subject = if count > 1 {
        format!("{count} of these flights are")
    } else {
        "this flight is".to_string()
    };
    format!(
        "{subject} drawn in the vertical plane, because nothing on the screen fixes how far the \
         spin axis was tilted (ADR-027 §Decision 5's third branch). The curvature is then zero by \
         construction and the landing offline reduces to `carry * sin(start line)` — which is \
         `shot_measure`'s `start_line_offline_yds` reached by a much longer route, and not a \
         landing point. The curve is missing from it, and on a shot the screen called a fade the \
         curve is most of the lateral miss."
    )
}

/// What an inferred spin is, said where the number is printed rather than in a footnote.
/// `inferred_caveat`. Empty for a flight whose spin was not solved, as Python's guard returns,
/// though [`caveats_for`] only asks it of an inferred one.
pub fn inferred_caveat(resolved: &ShotFlight) -> String {
    let Some(spin) = resolved.spin.as_ref() else {
        return String::new();
    };
    format!(
        "the spin on this flight was not measured. It is the spin this integrator needs in order \
         to fly the carry HD Golf printed, capped at {} rpm where carry stops responding to spin \
         at all, and chosen between two candidates by the club's loft. Both shots whose real spin \
         is on disk sit above their own cap, and the one the solve answers on comes back 47% low \
         — so this is a path-drawing device and not a spin rate the golfer produced.",
        pyfmt::fixed(spin.window().high_plateau_min_rpm, 0)
    )
}

/// That the gate was measured at sea level and this flight was not flown there. `altitude_caveat`.
pub fn altitude_caveat(altitude_m: f64) -> String {
    format!(
        "the gate above is a *sea-level* gate: HD Golf printed its carries for a ball hit indoors \
         near sea level, and this flight was flown at {} m ({} ft). The agreement measured there \
         does not transfer, and an altitude picked because the agreement improved would be fitting \
         the atmosphere to the residual of a held coefficient (ADR-027's 2026-09-05e addendum).",
        pyfmt::g(altitude_m),
        pyfmt::fixed(altitude_m * M_TO_FEET, 0)
    )
}

/// Everything one flown shot has to be read with, in the order a reader needs them. `caveats_for`.
///
/// The clamp first because it is about this flight, the gate second because it is about the model,
/// then the ones only sometimes owed. `altitude_m` is Python's keyword default of `0.0` made
/// explicit, and *truthy* as Python reads it: any altitude but zero, a NaN included, owes the
/// altitude caveat.
pub fn caveats_for(
    result: &FlightResult,
    resolved: &ShotFlight,
    model: &FlightModel,
    altitude_m: f64,
) -> Vec<String> {
    let mut said = vec![clamp_caveat(result, model), gate_caveat()];
    if !resolved.curve_is_drawn() {
        said.push(planar_caveat(1));
    }
    if resolved.spin_source == Some(SpinSource::Inferred) {
        said.push(inferred_caveat(resolved));
    }
    if altitude_m != 0.0 {
        said.push(altitude_caveat(altitude_m));
    }
    said
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::flight_model::load_flight_model;
    use crate::flight::{simulate_flight, LaunchConditions, DEFAULT_STEP_S};
    use crate::flight_infer::flight_for_shot;
    use crate::testing::a_shot;
    use contracts::golfer::Handedness;

    fn flown(launch: LaunchConditions) -> FlightResult {
        simulate_flight(&launch, load_flight_model(), DEFAULT_STEP_S).expect("a flyable launch")
    }

    /// Every sentence is one paragraph, as `_said` leaves it: no line break, no doubled space, and
    /// nothing at either end. A `\` continuation written with no space before it would join two
    /// words, which only this catches.
    fn assert_said(text: &str) {
        assert_eq!(pyfmt::split(text).collect::<Vec<_>>().join(" "), text);
        assert!(!text.is_empty());
    }

    /// The gate, to the character as frozen Python writes it: `.2%` and `.1%` of 0.0259.
    #[test]
    fn the_gate_caveat_is_pythons_and_derives_both_percentages() {
        let said = gate_caveat();
        assert_said(&said);
        assert_eq!(
            said,
            "the model agrees with HD Golf to +/-2.59% on the two shots where a full \
             launch-condition set and an independent carry are both on disk (`--gate`) — while \
             ranking those two shots backwards. That percentage is therefore the size of what this \
             model cannot see, not a calibration error, and a carry from here is not \"accurate \
             to 2.6%\"."
        );
    }

    /// The three clamp branches, each held to frozen Python's sentence for the same launch: the
    /// validation shot (every point above the table), a partly clamped one, and a low-spin one
    /// that reads a measured row the whole way. Python's texts were read off
    /// `flight_caveats.clamp_caveat` for these launches (M29 P14).
    #[test]
    fn the_clamp_caveat_takes_each_of_its_three_branches_as_python_does() {
        let model = load_flight_model();
        let full = flown(LaunchConditions {
            ball_speed_mph: 90.7,
            launch_angle_deg: 20.9,
            spin_rpm: 5991.0,
            launch_direction_deg: -5.3,
            spin_axis_deg: 2.5,
        });
        assert!(full.fully_clamped());
        assert!(clamp_caveat(&full, model).starts_with(
            "every point of this flight sat above the published table, which stops at a spin \
             ratio of 0.284 and was measured on a *driver*."
        ));

        // Python flies this one 1,413 points, 556 of them past the table's last row.
        let part = flown(LaunchConditions::planar(150.0, 12.0, 4000.0));
        assert_eq!((part.clamped_points, part.points.len()), (556, 1413));
        assert_eq!(
            clamp_caveat(&part, model),
            "556 of 1413 points fell outside the published table (0.085 to 0.284) and read a held \
             end row rather than a measured one. Over that part of the flight, spin has no route \
             into the answer."
        );

        let none = flown(LaunchConditions::planar(150.0, 12.0, 2600.0));
        assert_eq!(none.clamped_points, 0);
        assert_eq!(
            clamp_caveat(&none, model),
            "every point read a measured row of the table (0.085 to 0.284), which one shot on \
             disk manages and no other — and that shot is the one whose spin was *solved* rather \
             than printed. The two go together rather than coinciding: a carry the solve can \
             invert is a carry still responding to spin, and spin still reaching the answer is \
             exactly a flight below the clamp."
        );
    }

    /// The count is the subject, and one flight is "this flight", never "1 of these flights".
    /// The three-flight sentence is frozen Python's `planar_caveat(3)`.
    #[test]
    fn the_planar_caveat_names_one_flight_or_a_count() {
        assert!(planar_caveat(1).starts_with("this flight is drawn in the vertical plane,"));
        assert!(planar_caveat(0).starts_with("this flight is drawn"));
        assert_eq!(
            planar_caveat(3),
            "3 of these flights are drawn in the vertical plane, because nothing on the screen \
             fixes how far the spin axis was tilted (ADR-027 §Decision 5's third branch). The \
             curvature is then zero by construction and the landing offline reduces to `carry * \
             sin(start line)` — which is `shot_measure`'s `start_line_offline_yds` reached by a \
             much longer route, and not a landing point. The curve is missing from it, and on a \
             shot the screen called a fade the curve is most of the lateral miss."
        );
    }

    /// `{:g}` metres and `{:.0f}` feet, through the reciprocal Python multiplies by: frozen
    /// Python's `altitude_caveat(1609.0)`, and the half-metre one's `(2 ft)`.
    #[test]
    fn the_altitude_caveat_formats_the_altitude_as_python_does() {
        assert_eq!(
            altitude_caveat(1609.0),
            "the gate above is a *sea-level* gate: HD Golf printed its carries for a ball hit \
             indoors near sea level, and this flight was flown at 1609 m (5279 ft). The agreement \
             measured there does not transfer, and an altitude picked because the agreement \
             improved would be fitting the atmosphere to the residual of a held coefficient \
             (ADR-027's 2026-09-05e addendum)."
        );
        assert!(altitude_caveat(0.5).contains("at 0.5 m (2 ft)."));
    }

    fn a_flyable_shot() -> contracts::shot::ShotData {
        let mut shot = a_shot();
        shot.ball_speed = Some(89.8);
        shot.launch_angle = Some(22.4);
        shot.launch_direction = Some(2.8);
        shot.carry_distance = Some(126.1);
        shot
    }

    /// The composition is the shared rule: clamp, gate, then planar for an unbent flight, inferred
    /// for a solved spin, altitude for any altitude but zero, in that order.
    #[test]
    fn caveats_for_composes_in_the_order_a_reader_needs_them() {
        let model = load_flight_model();
        // A solved spin on a 7 iron's loft, with no axis printed: planar and inferred both owed.
        let resolved = flight_for_shot(&a_flyable_shot(), Some(30.5), None, model, DEFAULT_STEP_S)
            .expect("resolves");
        let result = flown(resolved.launch.expect("the spin solved"));
        let said = caveats_for(&result, &resolved, model, 0.0);
        assert_eq!(said.len(), 4);
        assert_eq!(said[0], clamp_caveat(&result, model));
        assert_eq!(said[1], gate_caveat());
        assert_eq!(said[2], planar_caveat(1));
        assert_eq!(said[3], inferred_caveat(&resolved));
        assert!(said[3].contains(" rpm where carry stops responding to spin at all"));
        for sentence in &said {
            assert_said(sentence);
        }
        let high = caveats_for(&result, &resolved, model, 1609.0);
        assert_eq!(high.len(), 5);
        assert_eq!(high[4], altitude_caveat(1609.0));

        // A printed spin and axis: the flight bends and nothing was solved, so the two always owed
        // are all it owes, and the inferred builder has nothing to say of it.
        let mut shot = a_flyable_shot();
        shot.spin_rate = Some(5991.0);
        shot.spin_axis = Some(2.5);
        let resolved = flight_for_shot(&shot, None, Some(Handedness::Right), model, DEFAULT_STEP_S)
            .expect("resolves");
        let result = flown(resolved.launch.expect("printed"));
        assert_eq!(caveats_for(&result, &resolved, model, 0.0).len(), 2);
        assert_eq!(inferred_caveat(&resolved), "");
    }
}
