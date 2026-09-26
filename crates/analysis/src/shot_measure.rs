//! Derived launch-monitor quantities. Measure, never judge. [M22 P8b]
//!
//! `ShotData` carries what the HD Golf screen printed. This module computes the things *derived*
//! from those numbers and worth recording in their own right — chiefly **face-to-path**, which the
//! screen does not print but which is the quantity that actually explains the ball's curvature.
//!
//! Nothing here scores anything and nothing here needs a reference population. Face-to-path relates
//! to shape by ball-flight physics rather than by a tour distribution, so it is measurable today
//! whereas carry and spin *norms* are blocked on data nobody has acquired (ADR-010 §4).
//!
//! # Five of the seven are pass-throughs, and a pass-through still earns a registry row
//!
//! `carry_distance_yds`, `total_distance_yds`, `ball_speed_mph`, `launch_angle_deg` and
//! `start_line_deg` read a `ShotData` field and return it. What [`SHOT_MEASUREMENTS`] holds is what
//! `storage.corpus` pools and what `analysis.baseline` builds a personal mean from, and a field that
//! stays on `ShotData` reaches neither. The Python module docstring argues each exclusion —
//! `smash_factor`, `club_head_speed`, `spin_axis`, dispersion — and none of that reasoning is
//! re-litigated here; what a port owes is the seven rows and their order.
//!
//! # What is gated, and what is not
//!
//! The `measurements` stage's `shot` group, on the fifteen corpus vectors, which covers all seven
//! names, units, `detail` sentences and values — 104 rows.
//!
//! **Exactly one refusal is gated, and it is the useful one.** `2026-08-23-3` printed a club path and
//! no face angle, so it is the single vector anywhere in `spec/` with fourteen face-to-path readings
//! against fifteen shots — which means a port reading `and` for `or` in [`measure_face_to_path`] fails
//! on one of twenty-one. That is the whole of the refusal coverage here: the other six names are
//! straight reads present on all fifteen, and the reverse mistake (a face angle with no path) is
//! reached by no vector at all. The unit tests below are what stands there.
//!
//! [`normalize_shot_shape`] is reached only through [`crate::flight_infer`], never through a
//! measurement row, and its token order is the load-bearing part: the screen prints compound labels
//! like `"CENTER SLIGHT FADE"` and matching `CENTER` first classifies a recorded fade as straight.

use contracts::intent::TargetShape;
use contracts::shot::ShotData;

/// Tokens the HD Golf screen uses in its free-text `Shot Type` tile.
///
/// **Order is load-bearing: curvature words are checked before centering words.** `"CENTER SLIGHT
/// FADE"` has `CENTER` describing the *start line* and `FADE` the *curve*; matching `CENTER` first
/// classified a real recorded fade as `Straight`. A slice rather than a map for the same reason
/// [`crate::measure::POSE_MEASUREMENTS`] is one — the iteration order is part of the answer.
static SHAPE_TOKENS: &[(&str, TargetShape)] = &[
    // Curvature — these decide the shape.
    ("DRAW", TargetShape::Draw),
    ("HOOK", TargetShape::Draw),
    ("FADE", TargetShape::Fade),
    ("SLICE", TargetShape::Fade),
    // Only reached when nothing curved.
    ("STRAIGHT", TargetShape::Straight),
    ("CENTER", TargetShape::Straight),
];

/// `club_face_angle - club_path`, in degrees. The quantity that explains curvature.
///
/// Positive means the face is **open to the path** (a fade/slice shape for a right-handed golfer);
/// negative means closed to it. Zero is a straight shot regardless of where either number sits
/// individually, which is exactly why the difference is the observable and neither angle is on its
/// own (ADR-009 §Concepts).
///
/// `None` if either angle is missing — and *either*, not both: a face angle with no path is not a
/// face-to-path of the face angle.
pub fn measure_face_to_path(shot: &ShotData) -> Option<f64> {
    Some(shot.club_face_angle? - shot.club_path?)
}

/// Initial horizontal launch direction, in degrees; positive is right of target.
///
/// Where the ball *started*, as opposed to how it curved afterwards. Kept separate from
/// face-to-path because the two together are what distinguish a pull-fade from a push-fade.
pub fn measure_start_line(shot: &ShotData) -> Option<f64> {
    shot.launch_direction
}

/// Carry, in yards, exactly as the screen printed it. No arithmetic at all.
pub fn measure_carry_distance(shot: &ShotData) -> Option<f64> {
    shot.carry_distance
}

/// Carry plus roll, in yards, as printed.
///
/// Kept beside carry rather than folded into it because the gap between them is the ground, not the
/// swing. `ShotData.bounce_and_roll` already prints that gap, so `total - carry` is not recorded: a
/// third number free to disagree with the two it came from.
pub fn measure_total_distance(shot: &ShotData) -> Option<f64> {
    shot.total_distance
}

/// How many yards right or left the ball **started**, in yards. Positive is right of target.
///
/// `carry * sin(start line)` — exact trigonometry over two printed numbers, with no physics invented
/// and no parameter fitted.
///
/// **This is not where the ball landed.** The HD Golf screen prints no offline tile at all, so a true
/// offline is not available to be read; the curve is a separate reading in separate units
/// ([`measure_face_to_path`], in degrees). A golfer who starts it straight and slices 30 yards reads
/// about 0 here, and any prose rendering this must say *started* and never *finished*.
///
/// Reads both inputs through the registry's own extractors rather than off `ShotData`, so there is
/// one definition of which field is the carry and which is the start line.
pub fn measure_start_line_offline(shot: &ShotData) -> Option<f64> {
    let carry = measure_carry_distance(shot)?;
    let start_line = measure_start_line(shot)?;
    Some(carry * start_line.to_radians().sin())
}

/// Ball speed off the face, in mph, exactly as the screen printed it. Judged by nothing.
///
/// A fitting input rather than a coaching number. It survives the smash-factor exclusion that keeps
/// club speed out because the printed smash implicates the *pair* and the two shots on disk isolate
/// the fault to club speed — the Python module docstring has the measurement.
pub fn measure_ball_speed(shot: &ShotData) -> Option<f64> {
    shot.ball_speed
}

/// Vertical launch angle, in degrees, as printed. The other half of the launch-condition pair.
///
/// **Vertical**, where [`measure_start_line`] is horizontal — two separate tiles on the screen and
/// two separate rows here, which is what the registry's `detail` string exists to say.
pub fn measure_launch_angle(shot: &ShotData) -> Option<f64> {
    shot.launch_angle
}

/// Map the simulator's free-text shape (`"CENTER SLIGHT FADE"`) onto [`TargetShape`].
///
/// Coarse by design: the sim's qualifiers ("SLIGHT") describe magnitude, and magnitude is what
/// [`measure_face_to_path`] reports in degrees. This answers only *which way did it curve*.
///
/// A compound label resolves to its **curvature** word, not its start-line word — see
/// [`SHAPE_TOKENS`] for the ordering that enforces it. `None` when nothing matches, rather than
/// guessing `Straight`: an unrecognized vocabulary is a fact worth surfacing.
pub fn normalize_shot_shape(shot: &ShotData) -> Option<TargetShape> {
    // Python's `if not shot.shot_type`, which is falsy on `""` as well as on `None`. The filter is
    // kept rather than dropped: scanning six tokens against an empty string reaches the same answer
    // today, and would stop doing so the moment a token were ever the empty string.
    let text = shot
        .shot_type
        .as_deref()
        .filter(|tile| !tile.is_empty())?
        .to_uppercase();
    SHAPE_TOKENS
        .iter()
        .find(|(token, _)| text.contains(token))
        .map(|(_, shape)| *shape)
}

/// How to take one shot measurement, and what the resulting number is.
///
/// [`crate::measure::PoseMeasurement`]'s shape, for the reason given there: the Python is three
/// parallel dicts keyed by the same names, which is three places to edit and two chances to add a
/// metric that measures fine and stores with the wrong unit.
pub struct ShotMeasurement {
    /// Takes the stored shot, returns the number or the absence of one.
    pub measure: fn(&ShotData) -> Option<f64>,
    /// Goes onto `Measurement.unit`.
    pub unit: &'static str,
    /// How the measurement is taken, carried onto `Measurement.detail`.
    pub detail: &'static str,
}

/// Name -> how to measure it, for everything derived from one launch-monitor shot.
///
/// A slice and not a map, and the order is the Python dict's insertion order:
/// [`crate::engine::shot_measurements`] walks it to build rows that `docs/CONFORMANCE.md` §3 compares
/// positionally. Only numeric measurements live here — [`normalize_shot_shape`] returns a category
/// and is consumed directly.
pub static SHOT_MEASUREMENTS: &[(&str, ShotMeasurement)] = &[
    (
        "face_to_path_deg",
        ShotMeasurement {
            measure: measure_face_to_path,
            unit: "degrees",
            detail: "club face angle minus club path; + is face open to path (fade shape)",
        },
    ),
    (
        "start_line_deg",
        ShotMeasurement {
            measure: measure_start_line,
            unit: "degrees",
            detail: "initial horizontal launch direction; + is right of target",
        },
    ),
    // The two distances are **pooled whole-bag** by everything that reads them until
    // `storage.corpus.narrow_to(club=)` lands, which is what makes a mean carry mean anything.
    // Nothing false ships in the meantime — `contracts.baseline.DEFAULT_MINIMUM_N` refuses a centre
    // below 5 samples — but the guard that saves this is a sample count, not an argument about clubs.
    (
        "carry_distance_yds",
        ShotMeasurement {
            measure: measure_carry_distance,
            unit: "yards",
            detail: "carry as printed by the launch monitor; meaningful per club, not across a bag",
        },
    ),
    (
        "total_distance_yds",
        ShotMeasurement {
            measure: measure_total_distance,
            unit: "yards",
            detail: "carry plus roll as printed; the gap to carry is the ground, not the swing",
        },
    ),
    // Derived, not read: the screen prints no offline tile, so this is the start line projected out
    // to the carry. The detail string says *started* on purpose — it is the one line about this
    // metric a reader is guaranteed to see, and "finished" is the misreading it exists to prevent.
    (
        "start_line_offline_yds",
        ShotMeasurement {
            measure: measure_start_line_offline,
            unit: "yards",
            detail: "yards right (+) or left of target the ball started, projected to the carry; \
                     not where it finished - the curve is face_to_path_deg",
        },
    ),
    // The two launch conditions. Recorded for a model that does not exist yet and scored by nothing.
    (
        "ball_speed_mph",
        ShotMeasurement {
            measure: measure_ball_speed,
            unit: "mph",
            detail: "ball speed off the face as printed by the launch monitor; a fitting input, \
                     judged by nothing",
        },
    ),
    (
        "launch_angle_deg",
        ShotMeasurement {
            measure: measure_launch_angle,
            unit: "degrees",
            detail: "vertical launch angle as printed; start_line_deg is the horizontal one",
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::a_shot;

    /// The registry's names and their order, which `docs/CONFORMANCE.md` §3 compares positionally.
    #[test]
    fn the_registry_is_the_pythons_seven_in_the_pythons_order() {
        let names: Vec<&str> = SHOT_MEASUREMENTS.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            [
                "face_to_path_deg",
                "start_line_deg",
                "carry_distance_yds",
                "total_distance_yds",
                "start_line_offline_yds",
                "ball_speed_mph",
                "launch_angle_deg",
            ]
        );
    }

    /// Face-to-path is the one row that is arithmetic, and the sign is the whole of what it says.
    ///
    /// The three numbers are the recorded agreements the Python docstring quotes: +13.2 on a FADE,
    /// +10.9 on a CENTER SLIGHT FADE, -11.8 on a DRAW.
    #[test]
    fn face_to_path_is_the_face_angle_less_the_path() {
        let mut shot = a_shot();
        for (face, path, want) in [(4.0, -9.2, 13.2), (5.6, -5.3, 10.9), (-7.1, 4.7, -11.8)] {
            shot.club_face_angle = Some(face);
            shot.club_path = Some(path);
            let got = measure_face_to_path(&shot).expect("both angles are present");
            assert!((got - want).abs() < 1e-12, "{got} != {want}");
        }
    }

    /// **Either missing refuses, not both.** No committed vector is missing one of the two, so this
    /// is the only thing standing between a port that reads `and` for `or` and a wrong number.
    #[test]
    fn a_face_to_path_needs_both_angles() {
        let mut shot = a_shot();
        shot.club_face_angle = Some(4.0);
        shot.club_path = None;
        assert_eq!(measure_face_to_path(&shot), None);
        shot.club_face_angle = None;
        shot.club_path = Some(-9.2);
        assert_eq!(measure_face_to_path(&shot), None);
        shot.club_face_angle = None;
        assert_eq!(measure_face_to_path(&shot), None);
    }

    /// The start-line offline is the projection and not the landing point — negative start line,
    /// negative offline, and its magnitude below the carry.
    #[test]
    fn the_start_line_offline_is_the_carry_projected_through_the_start_line() {
        let mut shot = a_shot();
        shot.carry_distance = Some(125.6);
        shot.launch_direction = Some(-5.3);
        let got = measure_start_line_offline(&shot).expect("both tiles printed");
        let want = 125.6 * (-5.3f64).to_radians().sin();
        assert!((got - want).abs() < 1e-12);
        assert!(got < 0.0 && got.abs() < 125.6);
    }

    /// And it refuses on either input, through the two extractors rather than the two fields.
    #[test]
    fn the_start_line_offline_refuses_without_a_carry_or_a_start_line() {
        let mut shot = a_shot();
        shot.carry_distance = None;
        shot.launch_direction = Some(-5.3);
        assert_eq!(measure_start_line_offline(&shot), None);
        shot.carry_distance = Some(125.6);
        shot.launch_direction = None;
        assert_eq!(measure_start_line_offline(&shot), None);
    }

    /// **The compound label is the reason the token order exists**, and no committed vector has one
    /// that would catch a reordering: all fifteen print a bare `FADE` or `DRAW`.
    #[test]
    fn a_compound_label_resolves_to_its_curvature_word() {
        let mut shot = a_shot();
        for (tile, want) in [
            ("CENTER SLIGHT FADE", TargetShape::Fade),
            ("CENTER SLIGHT DRAW", TargetShape::Draw),
            ("FADE", TargetShape::Fade),
            ("SLICE", TargetShape::Fade),
            ("HOOK", TargetShape::Draw),
            ("STRAIGHT", TargetShape::Straight),
            ("CENTER", TargetShape::Straight),
            // Lower case reaches the same answer: the tile is upper-cased before the scan.
            ("center slight fade", TargetShape::Fade),
        ] {
            shot.shot_type = Some(tile.to_string());
            assert_eq!(normalize_shot_shape(&shot), Some(want), "{tile}");
        }
    }

    /// An unrecognized vocabulary is a fact, not a default — and an empty tile is the same absence
    /// as no tile, which is Python's falsy test rather than its `is None` one.
    #[test]
    fn an_unknown_or_empty_tile_refuses_rather_than_guessing_straight() {
        let mut shot = a_shot();
        for tile in [None, Some(String::new()), Some("PUSH-PULL".to_string())] {
            shot.shot_type = tile.clone();
            assert_eq!(normalize_shot_shape(&shot), None, "{tile:?}");
        }
    }

    /// The five pass-throughs read the field they name and nothing else, which is a wiring check:
    /// carry and total are adjacent tiles and swapping them is invisible to every other test here.
    #[test]
    fn the_pass_throughs_read_the_tile_they_are_named_for() {
        let mut shot = a_shot();
        shot.launch_direction = Some(-5.3);
        shot.carry_distance = Some(125.6);
        shot.total_distance = Some(131.0);
        shot.ball_speed = Some(90.7);
        shot.launch_angle = Some(20.9);
        assert_eq!(measure_start_line(&shot), Some(-5.3));
        assert_eq!(measure_carry_distance(&shot), Some(125.6));
        assert_eq!(measure_total_distance(&shot), Some(131.0));
        assert_eq!(measure_ball_speed(&shot), Some(90.7));
        assert_eq!(measure_launch_angle(&shot), Some(20.9));
    }
}
