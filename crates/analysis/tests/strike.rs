//! `analysis::strike`: the per-shot judgments, each at and either side of its edge (M37 P2), one
//! club's grades built from them (P6), and the player's pooled over the bag (P7).
//!
//! **No Python twin.** The profile family (`spec/vectors/profile/hand/`, M37 P5) is worked by hand
//! and gated through `crates/core` once `run_profile` exists (P9). The first half of this file holds
//! each rule alone, against the M37 plan's decisions: decision 4's radius, call 10's grid, decision
//! 7's low point, call 5's shape boundary, call 6's landing basis, decision 8's on-line tolerance, and
//! decisions 1 and 6's consistency. Every edge is inclusive except low point's, which is strictly
//! ahead.
//!
//! The shots are built on the `synthetic` device (`crates/contracts/devices.json`, M37 P1), which
//! analyses every reading, so a judgment here is limited only by what its shot holds. The tests
//! after them hold the other half: a reading a device does not print, or prints only to show, never
//! reaches a judgment.
//!
//! The second half is [`build_club_grades`]: which shots a topic sees, when a topic exists, the
//! typical shot, the grid and the blend, each on a small corpus of its own. Then
//! [`build_player_grades`] and [`build_grades`]: each shot pooled by its own club's criterion, the
//! untagged shots reaching only the topics that need no club, a photo judged once however it is
//! filed, and the player's blend. Then the hand cases as a **preview**: every committed case's
//! grades, each club's and the player's, are built here and compared with the case's expected
//! ones, ahead of P9's gate, which compares the whole profile with its lists.

use std::fs;
use std::path::{Path, PathBuf};

use analysis::club_profile::build_bag_profile;
use analysis::flight_measure::{FLIGHT_MEASUREMENTS, FLIGHT_SOURCE};
use analysis::shot_measure::{measure_carry_distance, measure_start_line_offline};
use analysis::stats::{mean_and_sd, mean_ci};
use analysis::strike::{
    build_club_grades, build_grades, build_player_grades, carry_and_offline, impact_cell,
    is_centered, is_consistent, low_point_ahead, offline_basis, on_line, shape_of, AnalysedShot,
    CarryAndOffline, Landing, FLIGHT_REFUSAL_ROW, IMPACT_OFFSET_H, PRINTED_LANDING_FIELD,
    PROJECTED_LANDING_ROW,
};
use contracts::bag::Bag;
use contracts::capability::{devices, FieldUse};
use contracts::career::CareerCorpus;
use contracts::club::ClubId;
use contracts::intent::{ClubCategory, TargetShape};
use contracts::shot::ShotData;
use contracts::strike::{
    is_club_independent, low_point_graded_for, ClubGrades, FaceColumn, FaceRow, GradedProfile,
    ImpactCell, OfflineBasis, PlayerGrades, Share, Topic, TopicGrade, UnscoredCount,
};
use contracts::swing::Measurement;
use contracts::unscored::{UnscoredCheckpoint, UnscoredReason};
use contracts::Validate;
use serde_json::{json, Value};

// --- Fixtures -----------------------------------------------------------------------------------

/// A screen shot whose provenance names `device`, holding `fields`. Unstamped (`fields_present` not
/// recorded) unless `fields` sets it, so it prints what its device declares of what it holds.
fn shot_from(device: &str, fields: Value) -> ShotData {
    let mut document = json!({
        "shot_id": "s",
        "session_id": "d",
        "timestamp": "2026-10-10T00:00:00Z",
        "source": "screen",
        "provenance": {"device": device, "parse_confidence": 1.0},
    });
    let object = document.as_object_mut().expect("an object");
    for (key, value) in fields.as_object().expect("fields are an object") {
        if key == "fields_present" {
            object["provenance"]["fields_present"] = value.clone();
        } else {
            object.insert(key.clone(), value.clone());
        }
    }
    serde_json::from_value(document).expect("a valid shot")
}

/// The analysed view of a `synthetic` shot holding `fields`.
fn synthetic(fields: Value) -> AnalysedShot {
    AnalysedShot::of(&shot_from("synthetic", fields)).expect("synthetic is declared")
}

/// A shot struck at (`h`, `v`) mm, or at `h` alone.
fn struck(h: f64, v: Option<f64>) -> AnalysedShot {
    synthetic(json!({"impact_offset_h": h, "impact_offset_v": v}))
}

/// A shot whose face sits `face` degrees open and whose path `path` degrees in-to-out.
fn faced(face: f64, path: f64) -> AnalysedShot {
    synthetic(json!({"club_face_angle": face, "club_path": path}))
}

/// The flight's projected landing row, as the engine records it.
fn projected(offline_yds: f64) -> Measurement {
    Measurement {
        name: PROJECTED_LANDING_ROW.to_string(),
        value: offline_yds,
        unit: "yards".to_string(),
        source: FLIGHT_SOURCE.to_string(),
        detail: String::new(),
    }
}

fn refused(name: &str, reason: UnscoredReason) -> UnscoredCheckpoint {
    UnscoredCheckpoint {
        name: name.to_string(),
        reason,
        detail: String::new(),
    }
}

const EVERY_CATEGORY: [ClubCategory; 9] = [
    ClubCategory::Driver,
    ClubCategory::Wood,
    ClubCategory::Hybrid,
    ClubCategory::LongIron,
    ClubCategory::MidIron,
    ClubCategory::ShortIron,
    ClubCategory::Wedge,
    ClubCategory::Putter,
    ClubCategory::All,
];

// --- Where on the face: decision 4 and call 10 --------------------------------------------------

/// `hypot(h, v) ≤ 10`, inclusive: (6, 8) is exactly 10 mm out, in every quadrant.
#[test]
fn centered_is_inclusive_at_the_radius_with_both_offsets() {
    for (h, v) in [
        (6.0, 8.0),
        (-6.0, 8.0),
        (6.0, -8.0),
        (-8.0, -6.0),
        (0.0, 10.0),
        (10.0, 0.0),
    ] {
        assert_eq!(is_centered(&struck(h, Some(v))), Some(true), "({h}, {v})");
    }
    for (h, v) in [(6.0, 8.1), (0.0, -10.1), (10.1, 0.0)] {
        assert_eq!(is_centered(&struck(h, Some(v))), Some(false), "({h}, {v})");
    }
}

/// `|h| ≤ 10` where the vertical holds no value: judged on the axis printed, never assumed centered
/// on the other.
#[test]
fn centered_on_the_horizontal_alone_is_inclusive_at_the_radius() {
    for h in [0.0, 10.0, -10.0] {
        assert_eq!(is_centered(&struck(h, None)), Some(true), "{h}");
    }
    for h in [10.1, -10.1] {
        assert_eq!(is_centered(&struck(h, None)), Some(false), "{h}");
    }
}

/// Strike location's one required input is the horizontal offset.
#[test]
fn no_horizontal_offset_judges_no_strike() {
    let vertical_only = synthetic(json!({"impact_offset_v": 0.0}));
    assert_eq!(is_centered(&vertical_only), None);
    assert_eq!(impact_cell(&vertical_only), None);
    assert_eq!(is_centered(&synthetic(json!({}))), None);
    assert_eq!(Topic::StrikeLocation.inputs(), ["impact_offset_h"]);
}

/// Each axis binned at ±10, inclusive in the center, and no row where the vertical holds no value.
#[test]
fn the_grid_bins_each_axis_at_the_radius() {
    let cases = [
        (
            (10.0, Some(-10.0)),
            (FaceColumn::Center, Some(FaceRow::Center)),
        ),
        (
            (-10.0, Some(10.0)),
            (FaceColumn::Center, Some(FaceRow::Center)),
        ),
        ((10.1, Some(-10.1)), (FaceColumn::Toe, Some(FaceRow::Low))),
        ((-10.1, Some(10.1)), (FaceColumn::Heel, Some(FaceRow::High))),
        ((-25.0, None), (FaceColumn::Heel, None)),
        ((0.0, None), (FaceColumn::Center, None)),
        ((10.1, None), (FaceColumn::Toe, None)),
    ];
    for ((h, v), cell) in cases {
        assert_eq!(impact_cell(&struck(h, v)), Some(cell), "({h}, {v:?})");
    }
}

/// The square and the circle differ at the corners on purpose (`ImpactMap`'s doc): a strike at
/// (9, 9) mm is in the center cell and is not centered.
#[test]
fn the_center_cell_is_a_square_and_centered_is_a_circle() {
    let corner = struck(9.0, Some(9.0));
    assert_eq!(
        impact_cell(&corner),
        Some((FaceColumn::Center, Some(FaceRow::Center)))
    );
    assert_eq!(is_centered(&corner), Some(false));
}

// --- Low point: decision 7 ----------------------------------------------------------------------

/// Strictly ahead: at the ball is a miss.
#[test]
fn low_point_ahead_is_strictly_ahead_of_the_ball() {
    let at = |inches: f64| {
        low_point_ahead(
            &synthetic(json!({"low_point": inches})),
            ClubCategory::MidIron,
        )
    };
    assert_eq!(at(0.0), Some(false));
    assert_eq!(at(0.1), Some(true));
    assert_eq!(at(-0.1), Some(false));
    assert_eq!(at(3.0), Some(true));
    assert_eq!(
        low_point_ahead(&synthetic(json!({})), ClubCategory::MidIron),
        None
    );
}

/// Judged for an iron, a wedge or a hybrid; for a driver, a wood, a putter or no club it judges
/// nothing, so a builder that asked anyway would count an unscored shot and never a miss.
#[test]
fn low_point_judges_only_the_categories_it_is_graded_for() {
    let printed = synthetic(json!({"low_point": 2.0}));
    for category in EVERY_CATEGORY {
        let answer = low_point_ahead(&printed, category);
        if low_point_graded_for(category) {
            assert_eq!(answer, Some(true), "{category:?}");
        } else {
            assert_eq!(answer, None, "{category:?}");
        }
    }
    assert!(!low_point_graded_for(ClubCategory::Driver));
    assert!(low_point_graded_for(ClubCategory::Wedge));
}

// --- Shape: call 5 ------------------------------------------------------------------------------

/// Straight within ±2.0°, inclusive; a fade beyond on the open side, a draw beyond on the closed.
#[test]
fn the_shape_boundary_is_inclusive_at_the_straight_tolerance() {
    let cases = [
        (1.9, TargetShape::Straight),
        (2.0, TargetShape::Straight),
        (2.1, TargetShape::Fade),
        (-2.0, TargetShape::Straight),
        (-2.1, TargetShape::Draw),
        (0.0, TargetShape::Straight),
    ];
    for (face_to_path, shape) in cases {
        assert_eq!(
            shape_of(&faced(face_to_path, 0.0)),
            Some(shape),
            "{face_to_path}"
        );
    }
}

/// Face minus path, not either angle alone: a face 3° open to a 1° in-to-out path is 2° open to it.
#[test]
fn the_shape_is_the_face_against_the_path() {
    assert_eq!(shape_of(&faced(3.0, 1.0)), Some(TargetShape::Straight));
    assert_eq!(shape_of(&faced(1.0, 3.5)), Some(TargetShape::Draw));
    assert_eq!(shape_of(&faced(-1.0, -3.5)), Some(TargetShape::Fade));
    assert_eq!(shape_of(&synthetic(json!({"club_face_angle": 5.0}))), None);
    assert_eq!(shape_of(&synthetic(json!({"club_path": 5.0}))), None);
}

/// Nothing reads handedness. A left-hander's face open to the path is a fade by its sign, as a
/// right-hander's is (the M37 plan's finding 5); which side of the line it finishes on is the
/// projected landing's, read as it stands; and a landing withheld for want of a hand is named so.
#[test]
fn a_left_handed_shot_is_classified_as_its_sign_says() {
    let fade = faced(1.0, -1.5);
    assert_eq!(shape_of(&fade), Some(TargetShape::Fade));
    // A left-hander's fade curves left: the projection carries that, and on line reads it as is.
    let landing = offline_basis(&fade, &[projected(-7.0)], &[]).expect("projected");
    assert_eq!(landing.offline_yds, -7.0);
    assert!(on_line(landing.offline_yds));

    let draw = faced(-1.5, 1.0);
    assert_eq!(shape_of(&draw), Some(TargetShape::Draw));
    let landing = offline_basis(&draw, &[projected(12.0)], &[]).expect("projected");
    assert!(!on_line(landing.offline_yds));

    let unattributed = [refused(PROJECTED_LANDING_ROW, UnscoredReason::NoHandedness)];
    assert_eq!(
        offline_basis(&draw, &[], &unattributed),
        Err(UnscoredReason::NoHandedness)
    );
}

// --- Where it landed: call 6 and decision 8 -----------------------------------------------------

/// ±9 yd of the line, inclusive.
#[test]
fn on_line_is_inclusive_at_nine_yards() {
    for offline in [0.0, 9.0, -9.0, 8.9] {
        assert!(on_line(offline), "{offline}");
    }
    for offline in [9.1, -9.1, 30.0] {
        assert!(!on_line(offline), "{offline}");
    }
}

/// A printed landing wherever the view holds one, the projection otherwise.
#[test]
fn the_landing_is_printed_where_printed_and_projected_otherwise() {
    let printed = synthetic(json!({"carry_offline": 4.0}));
    assert_eq!(
        offline_basis(&printed, &[projected(-11.0)], &[]),
        Ok(Landing {
            offline_yds: 4.0,
            basis: OfflineBasis::Printed
        })
    );
    let unprinted = synthetic(json!({}));
    assert_eq!(
        offline_basis(&unprinted, &[projected(-11.0)], &[]),
        Ok(Landing {
            offline_yds: -11.0,
            basis: OfflineBasis::Projected
        })
    );
    // A blank printed tile falls through to the projection, as "used wherever it exists" says.
    let blank = [refused(PRINTED_LANDING_FIELD, UnscoredReason::PrintedBlank)];
    assert_eq!(
        offline_basis(&unprinted, &[projected(-11.0)], &blank).map(|landing| landing.basis),
        Ok(OfflineBasis::Projected)
    );
}

/// No landing: the reason the artifact named, the printed tile's before the projection's before a
/// refused flight's, and `unrecorded` when nothing says. Entries under other names say nothing.
#[test]
fn a_shot_with_no_landing_is_refused_with_the_reason_the_artifact_named() {
    let shot = synthetic(json!({}));
    let refuse = |entries: &[UnscoredCheckpoint]| offline_basis(&shot, &[], entries);
    assert_eq!(
        refuse(&[refused(
            PROJECTED_LANDING_ROW,
            UnscoredReason::SpinAxisUnresolved
        )]),
        Err(UnscoredReason::SpinAxisUnresolved)
    );
    assert_eq!(
        refuse(&[refused(
            FLIGHT_REFUSAL_ROW,
            UnscoredReason::CarryUnreachable
        )]),
        Err(UnscoredReason::CarryUnreachable)
    );
    assert_eq!(
        refuse(&[
            refused(FLIGHT_REFUSAL_ROW, UnscoredReason::NoLaunchConditions),
            refused(PROJECTED_LANDING_ROW, UnscoredReason::SpinAxisUnresolved),
            refused(PRINTED_LANDING_FIELD, UnscoredReason::Misread),
        ]),
        Err(UnscoredReason::Misread)
    );
    assert_eq!(
        refuse(&[
            refused(FLIGHT_REFUSAL_ROW, UnscoredReason::NoClubLoft),
            refused(PROJECTED_LANDING_ROW, UnscoredReason::NoHandedness),
        ]),
        Err(UnscoredReason::NoHandedness)
    );
    assert_eq!(refuse(&[]), Err(UnscoredReason::Unrecorded));
    assert_eq!(
        refuse(&[
            refused("tempo", UnscoredReason::BoundaryEstimated),
            refused("spin_axis", UnscoredReason::PrintedBlank),
        ]),
        Err(UnscoredReason::Unrecorded)
    );
}

/// The two flight names the landing reads are the flight registry's, so a renamed row fails here
/// rather than turning every projected landing into `unrecorded`.
#[test]
fn the_flight_names_the_landing_reads_are_the_flight_registrys() {
    let names: Vec<&str> = FLIGHT_MEASUREMENTS.iter().map(|(name, _)| *name).collect();
    assert!(names.contains(&PROJECTED_LANDING_ROW), "{names:?}");
    assert!(names.contains(&FLIGHT_REFUSAL_ROW), "{names:?}");
}

// --- Consistency: decisions 1 and 6, call 4 -----------------------------------------------------

const SEVEN_IRON: CarryAndOffline = CarryAndOffline {
    carry_yds: 150.0,
    offline_yds: 2.0,
};

fn went(carry_yds: f64, offline_yds: f64) -> CarryAndOffline {
    CarryAndOffline {
        carry_yds,
        offline_yds,
    }
}

/// ±10% of the median carry on carry, inclusive: 15 yd either side of 150.
#[test]
fn consistency_is_inclusive_at_ten_percent_of_the_median_carry() {
    for carry in [165.0, 135.0, 150.0] {
        assert!(
            is_consistent(went(carry, 2.0), SEVEN_IRON, false),
            "{carry}"
        );
    }
    for carry in [165.1, 134.9] {
        assert!(
            !is_consistent(went(carry, 2.0), SEVEN_IRON, false),
            "{carry}"
        );
    }
}

/// And the same 15 yd either side of the median offline, laterally: the window is a fraction of
/// the carry, not of the offline.
#[test]
fn the_lateral_window_is_ten_percent_of_the_median_carry() {
    for offline in [17.0, -13.0] {
        assert!(
            is_consistent(went(150.0, offline), SEVEN_IRON, false),
            "{offline}"
        );
    }
    for offline in [17.1, -13.1] {
        assert!(
            !is_consistent(went(150.0, offline), SEVEN_IRON, false),
            "{offline}"
        );
    }
    // Scaled per club: ±25 yd for a 250-yard driver, ±10 for a 100-yard wedge.
    let driver = went(250.0, 0.0);
    assert!(is_consistent(went(250.0, 25.0), driver, false));
    assert!(!is_consistent(went(250.0, 25.1), driver, false));
    let wedge = went(100.0, 0.0);
    assert!(is_consistent(went(100.0, -10.0), wedge, false));
    assert!(!is_consistent(went(100.0, -10.1), wedge, false));
}

/// Both windows, not either.
#[test]
fn consistency_needs_both_windows() {
    assert!(!is_consistent(went(165.1, 2.0), SEVEN_IRON, false));
    assert!(!is_consistent(went(150.0, 17.1), SEVEN_IRON, false));
    assert!(!is_consistent(went(170.0, 20.0), SEVEN_IRON, false));
}

/// Decision 6: a mishit is a miss whatever its numbers, even the typical shot's own.
#[test]
fn a_mishit_inside_both_windows_is_not_consistent() {
    assert!(is_consistent(SEVEN_IRON, SEVEN_IRON, false));
    assert!(!is_consistent(SEVEN_IRON, SEVEN_IRON, true));
    assert!(!is_consistent(went(151.0, 1.0), SEVEN_IRON, true));
}

/// Consistency reads the carry and the start-line offline by the functions that record their rows.
#[test]
fn consistency_reads_the_carry_and_the_start_line_offline() {
    let fields = json!({"carry_distance": 150.0, "launch_direction": 3.0});
    let view = synthetic(fields.clone());
    let raw = shot_from("synthetic", fields);
    let went = carry_and_offline(&view).expect("both held");
    assert_eq!(went.carry_yds, measure_carry_distance(&raw).unwrap());
    assert_eq!(went.offline_yds, measure_start_line_offline(&raw).unwrap());
    assert!((went.offline_yds - 150.0 * 3f64.to_radians().sin()).abs() < 1e-12);
    assert_eq!(
        carry_and_offline(&synthetic(json!({"carry_distance": 150.0}))),
        None
    );
    assert_eq!(
        carry_and_offline(&synthetic(json!({"launch_direction": 3.0}))),
        None
    );
    assert_eq!(
        Topic::Consistency.inputs(),
        ["carry_distance", "launch_direction"]
    );
}

// --- Nothing outside printed ∩ analysed reaches a judgment --------------------------------------

/// Every `ShotData` key any judgment reads: the topics' inputs, the vertical offset the grid adds a
/// row with, and the printed landing.
fn judged_readings() -> Vec<&'static str> {
    let mut keys: Vec<&str> = Topic::ALL
        .iter()
        .flat_map(|topic| topic.inputs().iter().copied())
        .chain(["impact_offset_v", PRINTED_LANDING_FIELD])
        .collect();
    keys.sort_unstable();
    keys.dedup();
    keys
}

/// HD Golf prints no impact offset, no low point and no landing: a shot holding all three is judged
/// on none of them, and on the shape and consistency it does print.
#[test]
fn an_undeclared_reading_never_reaches_a_judgment() {
    let shot = shot_from(
        "hd_golf",
        json!({
            "impact_offset_h": 0.0, "impact_offset_v": 0.0, "low_point": 2.0,
            "carry_offline": 0.0, "club_face_angle": 1.0, "club_path": 0.0,
            "carry_distance": 150.0, "launch_direction": 1.0,
        }),
    );
    let view = AnalysedShot::of(&shot).expect("hd_golf is declared");
    assert_eq!(is_centered(&view), None);
    assert_eq!(impact_cell(&view), None);
    assert_eq!(low_point_ahead(&view, ClubCategory::MidIron), None);
    assert_eq!(
        offline_basis(&view, &[], &[]),
        Err(UnscoredReason::Unrecorded)
    );
    assert_eq!(
        offline_basis(&view, &[projected(3.0)], &[]).map(|landing| landing.basis),
        Ok(OfflineBasis::Projected)
    );
    assert_eq!(shape_of(&view), Some(TargetShape::Straight));
    assert!(carry_and_offline(&view).is_some());
}

/// A stamped shot whose parse did not locate a tile prints nothing under it, whatever it holds.
#[test]
fn a_reading_the_screen_did_not_print_never_reaches_a_judgment() {
    let located = synthetic(json!({
        "impact_offset_h": 0.0, "low_point": 2.0,
        "fields_present": ["impact_offset_h", "low_point"],
    }));
    assert_eq!(is_centered(&located), Some(true));
    assert_eq!(low_point_ahead(&located, ClubCategory::Wedge), Some(true));
    let unlocated = synthetic(json!({
        "impact_offset_h": 0.0, "low_point": 2.0,
        "fields_present": ["carry_distance"],
    }));
    assert_eq!(is_centered(&unlocated), None);
    assert_eq!(low_point_ahead(&unlocated, ClubCategory::Wedge), None);
}

/// For every declared device, each judged reading survives the view exactly when the device
/// declares it analysed: undeclared and shown-only both arrive as `None`. No device declares a
/// judged reading shown-only today, so the shown-only half is the view's rule, held here for the
/// device that first does; `hd_golf`'s shown-only club speed is the live case of the rule.
#[test]
fn a_reading_reaches_the_judgments_only_where_its_device_analyses_it() {
    let judged = judged_readings();
    let holding: serde_json::Map<String, Value> = judged
        .iter()
        .map(|key| (key.to_string(), json!(1.0)))
        .collect();
    let mut walked = 0;
    for device in devices() {
        let view = AnalysedShot::of(&shot_from(&device.device, Value::Object(holding.clone())))
            .expect("a declared device");
        let written = serde_json::to_value(view.shot()).expect("serialize");
        for key in &judged {
            let reaches = !written[*key].is_null();
            let analysed = device.use_of(key) == Some(FieldUse::Analysed);
            assert_eq!(reaches, analysed, "{} {key}", device.device);
            walked += 1;
        }
    }
    assert!(walked >= 3 * judged.len(), "hd_golf, mock and synthetic");

    let shown_only = shot_from(
        "hd_golf",
        json!({"club_head_speed": 103.6, "ball_speed": 90.7}),
    );
    let view = AnalysedShot::of(&shown_only).expect("hd_golf is declared");
    assert_eq!(view.shot().club_head_speed, None);
    assert_eq!(view.shot().ball_speed, Some(90.7));
}

/// A device nobody declared is refused before anything is judged.
#[test]
fn an_undeclared_device_is_refused() {
    let r10: ShotData = serde_json::from_value(json!({
        "shot_id": "s", "session_id": "d", "timestamp": "2026-10-10T00:00:00Z", "source": "r10",
        "impact_offset_h": 0.0,
    }))
    .expect("a valid shot");
    assert!(AnalysedShot::of(&r10).is_err());
}

// --- One club's grades: the builder (P6) --------------------------------------------------------

/// Swing `i` of a builder fixture, as `crates/storage::read_corpus` writes one: a 7 iron counting
/// toward metrics, with a photo of its own, carrying a `device` shot holding `fields`. `extra`
/// overrides any key of the swing (`club`, `intent`, `stale`, `manual_mishit`, `shot_unscored` …).
fn swing(i: usize, device: &str, fields: Value, extra: Value) -> Value {
    let mut swing = json!({
        "player_id": "p",
        "session_id": "2026-09-01",
        "swing_id": i.to_string(),
        "captured_at": format!("2026-09-01T12:{:02}:00Z", i % 60),
        "club": "7i",
        "shot_sha256": format!("photo-{i}"),
        "analyzed": true,
        "analysis_version": 17,
        "shot": serde_json::to_value(shot_from(device, fields)).expect("a shot serializes"),
    });
    for (key, value) in extra.as_object().expect("extra is an object") {
        swing[key] = value.clone();
    }
    swing
}

fn corpus(swings: Vec<Value>) -> CareerCorpus {
    serde_json::from_value(json!({"player_id": "p", "swings": swings})).expect("a valid corpus")
}

/// `club`'s grades from `corpus`, beside the `ClubProfile` `build_bag_profile` makes of it, and
/// held to every rule `ClubGrades` checks.
fn grades_of(corpus: &CareerCorpus, club: ClubId) -> ClubGrades {
    let bag = build_bag_profile(corpus, None);
    let profile = bag.profile_for(club).expect("the club was hit");
    let grades = build_club_grades(corpus, profile);
    grades
        .validate()
        .unwrap_or_else(|e| panic!("{club}: {e:?}"));
    grades
}

fn topic_of(grades: &ClubGrades, topic: Topic) -> Option<&TopicGrade> {
    grades.topics.iter().find(|grade| grade.topic == topic)
}

fn topics_of(grades: &ClubGrades) -> Vec<Topic> {
    grades.topics.iter().map(|grade| grade.topic).collect()
}

/// `(successes, n)` of a graded share; panics on a withheld one.
fn graded(share: &Share) -> (i64, i64) {
    let rate = share.graded().expect("graded");
    (rate.successes, rate.n)
}

/// A 7-iron shot carrying `carry` yards, started `launch` degrees right.
fn carried(carry: f64, launch: f64) -> Value {
    json!({"carry_distance": carry, "launch_direction": launch})
}

/// The key strike location reads is the one the builder floors the lean at.
#[test]
fn the_impact_offset_key_is_strike_locations_input() {
    assert_eq!(Topic::StrikeLocation.inputs(), [IMPACT_OFFSET_H]);
}

/// A topic exists where its every input is analysed on one of the club's shots, and is absent, not
/// withheld, where none is: HD Golf prints no impact offset and no low point, so a shot holding
/// both is graded on consistency alone (decision 3, call 7).
#[test]
fn a_topic_exists_only_where_its_inputs_are_analysed() {
    let held = json!({
        "carry_distance": 150.0, "launch_direction": 0.0,
        "impact_offset_h": 0.0, "low_point": 2.0,
    });
    let on_hd_golf = corpus(
        (1..=3)
            .map(|i| swing(i, "hd_golf", held.clone(), json!({})))
            .collect(),
    );
    let grades = grades_of(&on_hd_golf, ClubId::SevenIron);
    assert_eq!(topics_of(&grades), [Topic::Consistency]);
    assert!(grades.strike.impacts.is_none());

    let on_synthetic = corpus(
        (1..=3)
            .map(|i| swing(i, "synthetic", held.clone(), json!({})))
            .collect(),
    );
    let grades = grades_of(&on_synthetic, ClubId::SevenIron);
    assert_eq!(
        topics_of(&grades),
        [Topic::StrikeLocation, Topic::Consistency, Topic::LowPoint]
    );
    assert!(grades.strike.impacts.is_some());

    // One shot printing an input is enough for the topic to exist; the shots not printing it are
    // neither judged nor unscored, since an unstamped shot prints only what it holds.
    let one_printed = corpus(vec![
        swing(1, "synthetic", json!({"impact_offset_h": 0.0}), json!({})),
        swing(2, "synthetic", json!({}), json!({})),
    ]);
    let grades = grades_of(&one_printed, ClubId::SevenIron);
    let strike = topic_of(&grades, Topic::StrikeLocation).expect("printed on one shot");
    assert_eq!((strike.judged, strike.unscored.len()), (1, 0));
    assert_eq!(grades.n_shots, 2);
}

/// Low point is graded for an iron and absent for a driver though the driver prints it (decision 7).
#[test]
fn low_point_is_absent_for_a_driver_that_prints_it() {
    let swings = (1..=4)
        .map(|i| {
            let club = if i <= 2 { "7i" } else { "driver" };
            swing(
                i,
                "synthetic",
                json!({"low_point": i as f64 - 2.5}),
                json!({"club": club}),
            )
        })
        .collect();
    let printed = corpus(swings);
    let iron = grades_of(&printed, ClubId::SevenIron);
    let low_point = topic_of(&iron, Topic::LowPoint).expect("graded for an iron");
    assert_eq!(low_point.judged, 2);
    assert!(topic_of(&grades_of(&printed, ClubId::Driver), Topic::LowPoint).is_none());
}

/// A shape exists only where a shot of the club was declared `shot_shaping` with it (call 7): a
/// `target_shape` under another mode declares nothing, and a shape never declared is absent.
#[test]
fn a_shape_exists_only_where_a_shot_was_declared_shot_shaping_with_it() {
    let faced = json!({"club_face_angle": 3.0, "club_path": 0.0});
    let fundamentals = json!({"intent": {"mode": "fundamentals", "target_shape": "fade"}});
    let not_declared = corpus(vec![swing(1, "synthetic", faced.clone(), fundamentals)]);
    let grades = grades_of(&not_declared, ClubId::SevenIron);
    assert!(topics_of(&grades).iter().all(|t| t.shape().is_none()));

    let shaping = json!({"intent": {"mode": "shot_shaping", "target_shape": "fade"}});
    let declared = corpus(vec![
        swing(1, "synthetic", faced.clone(), shaping),
        swing(2, "synthetic", faced, json!({})),
    ]);
    let grades = grades_of(&declared, ClubId::SevenIron);
    assert_eq!(topics_of(&grades), [Topic::Fade]);
    let fade = topic_of(&grades, Topic::Fade).expect("declared");
    // Only the declared shot is judged on it; the fundamentals one is not a fade attempt.
    assert_eq!(fade.judged, 1);
    let shares = fade.shape.as_ref().expect("a shape carries its shares");
    assert_eq!(shares.hit_shape.n(), 1);
    // No landing on either basis and nothing named: unrecorded, out of the on-line denominator.
    assert_eq!(
        shares.landing_unscored,
        [UnscoredCount {
            reason: UnscoredReason::Unrecorded,
            n: 1
        }]
    );
    assert_eq!(fade.grade.n(), 0);
}

/// A shape's two shares: "hit the shape" over every judged shot, and "hit it and finished on line"
/// over those with a landing, printed counted apart from projected (calls 5 and 6).
#[test]
fn a_shape_grades_the_shape_and_the_landing() {
    let shaping = json!({"intent": {"mode": "shot_shaping", "target_shape": "draw"}});
    let draw = |i: usize, face: f64, fields: Value, extra: Value| {
        let mut held = json!({"club_face_angle": face, "club_path": 0.0});
        for (key, value) in fields.as_object().expect("an object") {
            held[key] = value.clone();
        }
        let mut swing = swing(i, "synthetic", held, shaping.clone());
        for (key, value) in extra.as_object().expect("an object") {
            swing[key] = value.clone();
        }
        swing
    };
    let projected_row = |value: f64| json!([{"name": PROJECTED_LANDING_ROW, "value": value, "unit": "yards", "source": FLIGHT_SOURCE, "detail": ""}]);
    let mut swings = Vec::new();
    // Eight printed landings: six hit the draw and finish on line, one hits it and misses the line,
    // one is straight (-2.0, inclusive) and on line.
    for i in 1..=6 {
        swings.push(draw(i, -3.0, json!({"carry_offline": 9.0}), json!({})));
    }
    swings.push(draw(7, -3.0, json!({"carry_offline": -9.1}), json!({})));
    swings.push(draw(8, -2.0, json!({"carry_offline": 0.0}), json!({})));
    // Two projected, both draws on line; one with no landing, its spin axis unresolved.
    for i in 9..=10 {
        swings.push(draw(
            i,
            -2.1,
            json!({}),
            json!({"measurements": projected_row(-4.0)}),
        ));
    }
    swings.push(draw(
        11,
        -5.0,
        json!({}),
        json!({"shot_unscored": [{"name": PROJECTED_LANDING_ROW, "reason": "spin_axis_unresolved", "detail": ""}]}),
    ));
    let grades = grades_of(&corpus(swings), ClubId::SevenIron);
    let topic = topic_of(&grades, Topic::Draw).expect("declared");
    let shares = topic.shape.as_ref().expect("a shape");
    assert_eq!(topic.judged, 11);
    assert_eq!(graded(&shares.hit_shape), (10, 11));
    assert_eq!((shares.printed, shares.projected), (8, 2));
    assert_eq!(graded(&topic.grade), (8, 10));
    assert_eq!(
        shares.landing_unscored,
        [UnscoredCount {
            reason: UnscoredReason::SpinAxisUnresolved,
            n: 1
        }]
    );
}

/// A tile printed and holding no value is unscored under the reason the artifact named for it,
/// else `printed_blank`, and never a miss (call 8). The reasons list in declaration order.
#[test]
fn a_blank_input_is_unscored_under_its_reason_and_never_a_miss() {
    let stamped = |fields: Value| {
        let mut fields = fields;
        fields["fields_present"] = json!([IMPACT_OFFSET_H]);
        fields
    };
    let mut swings: Vec<Value> = (1..=10)
        .map(|i| {
            swing(
                i,
                "synthetic",
                stamped(json!({"impact_offset_h": 1.0})),
                json!({}),
            )
        })
        .collect();
    swings.push(swing(
        11,
        "synthetic",
        stamped(json!({})),
        json!({"shot_unscored": [{"name": IMPACT_OFFSET_H, "reason": "misread", "detail": ""}]}),
    ));
    swings.push(swing(12, "synthetic", stamped(json!({})), json!({})));
    let grades = grades_of(&corpus(swings), ClubId::SevenIron);
    let strike = topic_of(&grades, Topic::StrikeLocation).expect("printed");
    assert_eq!(graded(&strike.grade), (10, 10));
    assert_eq!(
        strike.unscored,
        [
            UnscoredCount {
                reason: UnscoredReason::PrintedBlank,
                n: 1
            },
            UnscoredCount {
                reason: UnscoredReason::Misread,
                n: 1
            },
        ]
    );
    let impacts = grades
        .strike
        .impacts
        .as_ref()
        .expect("strike location exists");
    assert_eq!(impacts.heel_toe.n, 10);
}

/// A topic judges each photo once, on its first swing in corpus order, and only from swings that
/// count: a stale, outdated, unanalysed or flagged swing, and a shot from a device nobody declared,
/// are not judged. `n_shots` is still every photo, since it is evidence and not a judgment.
#[test]
fn each_photo_is_judged_once_and_only_from_a_swing_that_counts() {
    let held = carried(150.0, 0.0);
    let mut swings = vec![
        swing(1, "synthetic", held.clone(), json!({})),
        // A second clip of the same photo: the photo is judged once.
        swing(
            2,
            "synthetic",
            held.clone(),
            json!({"shot_sha256": "photo-1"}),
        ),
        swing(3, "synthetic", held.clone(), json!({"stale": true})),
        swing(4, "synthetic", held.clone(), json!({"outdated": true})),
        swing(5, "synthetic", held.clone(), json!({"analyzed": false})),
        swing(
            6,
            "synthetic",
            held.clone(),
            json!({"shot_needs_review": true}),
        ),
        swing(7, "synthetic", held.clone(), json!({})),
    ];
    let mut undeclared = swing(8, "synthetic", held, json!({}));
    undeclared["shot"]["source"] = json!("r10");
    undeclared["shot"]["provenance"] = Value::Null;
    swings.push(undeclared);
    let grades = grades_of(&corpus(swings), ClubId::SevenIron);
    let consistency = topic_of(&grades, Topic::Consistency).expect("printed");
    assert_eq!(consistency.judged, 2);
    assert_eq!(grades.n_shots, 7);
}

/// Decision 6 and call 4: a mishit inside both windows is a miss and stays out of the typical
/// shot, and a club whose every judged shot is a mishit has no typical shot and grades 0 of n,
/// rather than dropping its shots.
#[test]
fn a_mishit_is_a_miss_and_a_club_of_mishits_grades_zero() {
    let confirmed = json!({"manual_mishit": "confirmed"});
    let mut swings: Vec<Value> = (1..=10)
        .map(|i| swing(i, "synthetic", carried(150.0, 0.0), json!({})))
        .collect();
    // Inside both windows, and a miss; far out, and left out of the median.
    swings.push(swing(
        11,
        "synthetic",
        carried(150.0, 0.0),
        confirmed.clone(),
    ));
    swings.push(swing(
        12,
        "synthetic",
        carried(40.0, 0.0),
        confirmed.clone(),
    ));
    let grades = grades_of(&corpus(swings), ClubId::SevenIron);
    let consistency = topic_of(&grades, Topic::Consistency).expect("printed");
    assert_eq!(graded(&consistency.grade), (10, 12));

    let all_mishits = corpus(
        (1..=10)
            .map(|i| swing(i, "synthetic", carried(150.0, 0.0), confirmed.clone()))
            .collect(),
    );
    let grades = grades_of(&all_mishits, ClubId::SevenIron);
    let consistency = topic_of(&grades, Topic::Consistency).expect("printed");
    assert_eq!(graded(&consistency.grade), (0, 10));
    assert_eq!(consistency.judged, 10);
}

/// Below ten judged shots a share is withheld and named, and a club with nothing graded has no
/// blend (decision 2, call 11). At ten it grades, and the blend averages the graded topics and
/// names the withheld ones as excluded.
#[test]
fn a_thin_topic_is_withheld_and_the_blend_averages_what_graded() {
    let held = |i: usize| {
        let mut fields = carried(150.0, 0.0);
        if i <= 9 {
            fields["impact_offset_h"] = json!(if i <= 3 { 15.0 } else { 0.0 });
        }
        fields
    };
    let nine = corpus(
        (1..=9)
            .map(|i| swing(i, "synthetic", held(i), json!({})))
            .collect(),
    );
    let grades = grades_of(&nine, ClubId::SevenIron);
    for topic in &grades.topics {
        assert!(matches!(topic.grade, Share::Withheld(_)), "{topic:?}");
        assert_eq!(topic.grade.n(), 9);
    }
    assert!(grades.blend.is_none());

    let ten = corpus(
        (1..=10)
            .map(|i| swing(i, "synthetic", held(i), json!({})))
            .collect(),
    );
    let grades = grades_of(&ten, ClubId::SevenIron);
    assert!(matches!(
        topic_of(&grades, Topic::StrikeLocation)
            .expect("printed")
            .grade,
        Share::Withheld(_)
    ));
    assert_eq!(
        graded(
            &topic_of(&grades, Topic::Consistency)
                .expect("printed")
                .grade
        ),
        (10, 10)
    );
    let blend = grades.blend.as_ref().expect("consistency graded");
    assert_eq!(blend.averaged, [Topic::Consistency]);
    assert_eq!(blend.excluded, [Topic::StrikeLocation]);
    assert_eq!(blend.mean, 1.0);
}

/// The grid lists the cells struck, row by row with the horizontal-only row first, and heel/toe
/// are its outer columns. The lean shows from the `Center` floor of 5 and not below it, as
/// `stats` computes it over the offsets in corpus order.
#[test]
fn the_grid_and_the_lean() {
    let at = |h: f64, v: Option<f64>| json!({"impact_offset_h": h, "impact_offset_v": v});
    let strikes = [
        at(-12.0, None),
        at(0.0, Some(0.0)),
        at(11.0, Some(-11.0)),
        at(0.0, None),
    ];
    let four = corpus(
        strikes
            .iter()
            .enumerate()
            .map(|(i, fields)| swing(i + 1, "synthetic", fields.clone(), json!({})))
            .collect(),
    );
    let grades = grades_of(&four, ClubId::SevenIron);
    let impacts = grades
        .strike
        .impacts
        .as_ref()
        .expect("strike location exists");
    let cell = |horizontal, vertical, n| ImpactCell {
        horizontal,
        vertical,
        n,
    };
    assert_eq!(
        impacts.grid,
        [
            cell(FaceColumn::Heel, None, 1),
            cell(FaceColumn::Center, None, 1),
            cell(FaceColumn::Toe, Some(FaceRow::Low), 1),
            cell(FaceColumn::Center, Some(FaceRow::Center), 1),
        ]
    );
    let lean = &impacts.heel_toe;
    assert_eq!((lean.n, lean.heel, lean.toe), (4, 1, 1));
    assert!(lean.mean_offset_mm.is_none() && lean.mean_offset_ci.is_none());

    let mut five: Vec<Value> = strikes
        .iter()
        .enumerate()
        .map(|(i, fields)| swing(i + 1, "synthetic", fields.clone(), json!({})))
        .collect();
    five.push(swing(5, "synthetic", at(3.0, Some(1.0)), json!({})));
    let grades = grades_of(&corpus(five), ClubId::SevenIron);
    let lean = &grades.strike.impacts.as_ref().expect("exists").heel_toe;
    let offsets = [-12.0, 0.0, 11.0, 0.0, 3.0];
    assert_eq!(
        lean.mean_offset_mm,
        mean_and_sd(&offsets).map(|(mean, _)| mean)
    );
    let ci = lean.mean_offset_ci.as_ref().expect("shown with its mean");
    assert_eq!(Some((ci.low, ci.high)), mean_ci(&offsets));
}

// --- The player's grades: the pool (P7) ---------------------------------------------------------

/// The player's grades from `corpus`, held to every rule `PlayerGrades` checks.
fn player_of(corpus: &CareerCorpus) -> PlayerGrades {
    let player = build_player_grades(corpus);
    player.validate().unwrap_or_else(|e| panic!("{e:?}"));
    player
}

fn player_topic(player: &PlayerGrades, topic: Topic) -> Option<&TopicGrade> {
    player.topics.iter().find(|grade| grade.topic == topic)
}

/// Each shot pools as its own club's grade judged it (ADR-034 §5.4). A 7 iron 14 yd short of its
/// club's 150 is inside the 7 iron's 15 yd window and far outside the driver's; a drive 24 yd short
/// of 250 is inside the driver's 25. One median over all twenty would sit at 188 and fail every
/// shot, so the player's 20 of 20 is only reachable club by club, and it is the clubs' sum.
#[test]
fn a_shot_pools_as_consistent_by_its_own_clubs_median() {
    let mut swings: Vec<Value> = (1..=10)
        .map(|i| {
            let carry = if i == 1 { 136.0 } else { 150.0 };
            swing(i, "synthetic", carried(carry, 0.0), json!({}))
        })
        .collect();
    for i in 11..=20 {
        let carry = if i == 11 { 226.0 } else { 250.0 };
        swings.push(swing(
            i,
            "synthetic",
            carried(carry, 0.0),
            json!({"club": "driver"}),
        ));
    }
    let corpus = corpus(swings);
    for club in [ClubId::SevenIron, ClubId::Driver] {
        let grades = grades_of(&corpus, club);
        let consistency = topic_of(&grades, Topic::Consistency).expect("printed");
        assert_eq!(graded(&consistency.grade), (10, 10), "{club}");
    }
    let player = player_of(&corpus);
    let consistency = player_topic(&player, Topic::Consistency).expect("pooled");
    assert_eq!(graded(&consistency.grade), (20, 20));
    assert_eq!((player.n_shots, player.untagged_shots), (20, 0));
}

/// The untagged shots are an owner with no club (call 9): they reach strike location and a shape,
/// whose criteria need no club, and never consistency or low point, though every one of them holds
/// a carry, a start line and a low point. Without a tagged shot, those two topics do not exist for
/// the player at all.
#[test]
fn untagged_shots_enter_strike_location_and_the_shapes_only() {
    let held = json!({
        "carry_distance": 150.0, "launch_direction": 0.0, "impact_offset_h": 0.0,
        "low_point": 1.0, "club_face_angle": 3.0, "club_path": 0.0, "carry_offline": 0.0,
    });
    let fade = json!({"mode": "shot_shaping", "target_shape": "fade"});
    let tagged = (1..=10).map(|i| swing(i, "synthetic", held.clone(), json!({"intent": fade})));
    let untagged: Vec<Value> = (11..=14)
        .map(|i| {
            swing(
                i,
                "synthetic",
                held.clone(),
                json!({"club": null, "intent": fade}),
            )
        })
        .collect();

    let both = corpus(tagged.chain(untagged.iter().cloned()).collect());
    let player = player_of(&both);
    assert_eq!((player.n_shots, player.untagged_shots), (14, 4));
    let judged = |topic| player_topic(&player, topic).map(|grade| grade.judged);
    assert_eq!(judged(Topic::StrikeLocation), Some(14));
    assert_eq!(judged(Topic::Fade), Some(14));
    assert_eq!(judged(Topic::Consistency), Some(10));
    assert_eq!(judged(Topic::LowPoint), Some(10));
    // All fourteen were declared a fade, hit one and landed on line: the on-line share is over every
    // landing, the untagged four's included.
    let fade_grade = player_topic(&player, Topic::Fade).expect("declared");
    assert_eq!(graded(&fade_grade.grade), (14, 14));
    let impacts = player
        .strike
        .impacts
        .as_ref()
        .expect("strike location exists");
    assert_eq!(impacts.heel_toe.n, 14);

    let alone = player_of(&corpus(untagged));
    assert_eq!((alone.n_shots, alone.untagged_shots), (4, 4));
    let topics: Vec<Topic> = alone.topics.iter().map(|grade| grade.topic).collect();
    assert_eq!(topics, [Topic::StrikeLocation, Topic::Fade]);
}

/// A photo filed under two clubs, a tagging slip, is judged by each club's grade and **once** by the
/// player's, on its first swing in corpus order. Added up, the clubs would judge 11 shots of the
/// player's 10, which `PlayerGrades` refuses.
#[test]
fn a_photo_filed_under_two_clubs_is_judged_once_for_the_player() {
    let mut swings: Vec<Value> = (1..=10)
        .map(|i| swing(i, "synthetic", carried(150.0, 0.0), json!({})))
        .collect();
    swings.push(swing(
        11,
        "synthetic",
        carried(150.0, 0.0),
        json!({"club": "driver", "shot_sha256": "photo-1"}),
    ));
    let corpus = corpus(swings);
    let judged = |grades: &ClubGrades| {
        topic_of(grades, Topic::Consistency)
            .expect("printed")
            .judged
    };
    assert_eq!(judged(&grades_of(&corpus, ClubId::SevenIron)), 10);
    assert_eq!(judged(&grades_of(&corpus, ClubId::Driver)), 1);
    let player = player_of(&corpus);
    assert_eq!(player.n_shots, 10);
    let consistency = player_topic(&player, Topic::Consistency).expect("pooled");
    assert_eq!(graded(&consistency.grade), (10, 10));
}

/// The player's blend is the mean of the player's own graded topics (call 11), not of the clubs'
/// blends: it averages consistency, names strike location (four struck shots) as excluded, and holds
/// nothing of low point, which the drives print and no driver is graded on, so it is absent. Below
/// the floor everywhere, there is no blend at all, never a 0.
#[test]
fn the_player_blend_averages_its_own_graded_topics() {
    let mut swings: Vec<Value> = (1..=10)
        .map(|i| {
            let mut held = carried(150.0, 0.0);
            if i <= 4 {
                held["impact_offset_h"] = json!(0.0);
            }
            swing(i, "synthetic", held, json!({}))
        })
        .collect();
    // Against their own median of 250 and its 25 yd window, the 200 and the 300 miss: 3 of 5.
    for (i, carry) in (11..).zip([250.0, 250.0, 250.0, 200.0, 300.0]) {
        let mut held = carried(carry, 0.0);
        held["low_point"] = json!(1.0);
        swings.push(swing(i, "synthetic", held, json!({"club": "driver"})));
    }
    let corpus = corpus(swings);
    let iron = grades_of(&corpus, ClubId::SevenIron);
    assert_eq!(iron.blend.as_ref().map(|blend| blend.mean), Some(1.0));
    assert!(grades_of(&corpus, ClubId::Driver).blend.is_none());

    let player = player_of(&corpus);
    let topics: Vec<Topic> = player.topics.iter().map(|grade| grade.topic).collect();
    assert_eq!(topics, [Topic::StrikeLocation, Topic::Consistency]);
    let consistency = player_topic(&player, Topic::Consistency).expect("pooled");
    assert_eq!(graded(&consistency.grade), (13, 15));
    let blend = player.blend.as_ref().expect("consistency graded");
    assert_eq!(blend.averaged, [Topic::Consistency]);
    assert_eq!(blend.excluded, [Topic::StrikeLocation]);
    let rate = consistency.grade.graded().expect("graded");
    assert_eq!(blend.mean, rate.share);
    assert!(blend.mean < 1.0, "not the 7 iron's blend carried up");

    let nine = corpus_of_nine();
    let player = player_of(&nine);
    assert!(player
        .topics
        .iter()
        .all(|grade| matches!(grade.grade, Share::Withheld(_))));
    assert!(player.blend.is_none());
}

fn corpus_of_nine() -> CareerCorpus {
    corpus(
        (1..=9)
            .map(|i| swing(i, "synthetic", carried(150.0, 0.0), json!({})))
            .collect(),
    )
}

/// The player's raw numbers are club-independent only (call 17): no ball speed, and the dispersion of
/// the two angles over the whole corpus, never the carry's, though every swing records one.
#[test]
fn the_players_raw_numbers_are_club_independent() {
    // The baseline and the dispersion read measurement rows, as `read_corpus` lifts them, so each
    // swing carries the rows its shot would record.
    let rows = |i: usize| {
        let row = |name: &str, value: f64, unit: &str| {
            json!({"name": name, "value": value, "unit": unit,
                   "source": "launch_monitor:synthetic", "detail": ""})
        };
        json!([
            row("face_to_path_deg", (i % 4) as f64, "degrees"),
            row("start_line_deg", (i % 3) as f64 - 1.0, "degrees"),
            row("carry_distance_yds", 150.0 + i as f64, "yards"),
            row("ball_speed_mph", 110.0 + (i % 2) as f64, "mph"),
        ])
    };
    let corpus = corpus(
        (1..=12)
            .map(|i| {
                swing(
                    i,
                    "synthetic",
                    carried(150.0 + i as f64, 0.0),
                    json!({"measurements": rows(i)}),
                )
            })
            .collect(),
    );
    let bag = build_bag_profile(&corpus, None);
    let player = player_of(&corpus);
    assert!(player.strike.ball_speed.is_none());
    let keys: Vec<&str> = player
        .strike
        .dispersion
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["face_to_path_deg", "start_line_deg"]);
    assert!(keys.iter().all(|metric| is_club_independent(metric)));
    // The club keeps its carry spread and its ball speed, copied from its `ClubProfile`.
    let club = build_club_grades(&corpus, bag.profile_for(ClubId::SevenIron).expect("hit"));
    assert!(club.strike.ball_speed.is_some());
    assert!(club.strike.dispersion.contains_key("carry_distance_yds"));
}

/// [`build_grades`] grades only a club with a shot photo, in bag order: a club the bag declares and
/// nobody hit, and a club hit only on clips with no photo, are profiled by `build_bag_profile` and
/// are not clubs of the grades.
#[test]
fn build_grades_lists_only_the_clubs_with_a_photo() {
    let mut swings: Vec<Value> = (1..=3)
        .map(|i| {
            swing(
                i,
                "synthetic",
                carried(250.0, 0.0),
                json!({"club": "driver"}),
            )
        })
        .collect();
    swings.extend((4..=6).map(|i| swing(i, "synthetic", carried(150.0, 0.0), json!({}))));
    swings.push(swing(
        7,
        "synthetic",
        json!({}),
        json!({"club": "9i", "shot_sha256": null, "shot": null}),
    ));
    let corpus = corpus(swings);
    let bag: Bag = serde_json::from_value(json!({
        "player_id": "p",
        "entries": {"pw": {"club": "pw", "loft_deg": 46.0, "recorded_at": "2026-08-20T12:00:00Z"}},
        "updated_at": "2026-08-20T12:00:00Z",
    }))
    .expect("a valid bag");
    let bag_profile = build_bag_profile(&corpus, Some(&bag));
    let profiled: Vec<ClubId> = bag_profile.clubs.iter().map(|club| club.club).collect();
    assert_eq!(
        profiled,
        [
            ClubId::Driver,
            ClubId::SevenIron,
            ClubId::NineIron,
            ClubId::PitchingWedge
        ]
    );
    let grades = build_grades(&corpus, &bag_profile);
    let graded_clubs: Vec<ClubId> = grades.clubs.iter().map(|club| club.club).collect();
    assert_eq!(graded_clubs, [ClubId::Driver, ClubId::SevenIron]);
    assert_eq!(grades.player, build_player_grades(&corpus));
    assert_eq!(grades.player.n_shots, 6);
}

// --- The hand cases, as a preview of P9's gate ----------------------------------------------------

fn hand_cases() -> Vec<(String, Value)> {
    let dir: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/profile/hand");
    let mut cases: Vec<(String, Value)> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .map(|path| {
            let text =
                fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let vector: Value =
                serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let id = vector["id"].as_str().expect("an id").to_string();
            (id, vector)
        })
        .collect();
    cases.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!cases.is_empty(), "no hand case under {}", dir.display());
    cases
}

/// Every hand case's grades through [`build_grades`], compared with the case's expected profile: the
/// clubs it lists, each club's strike profile, topics and blend, and the player's. The lists beside
/// them are `feedback`'s, which this crate cannot reach, so they are compared by the family's gate,
/// `crates/core/tests/profile.rs`, which holds the whole profile to `run_profile` (P9).
#[test]
fn every_hand_cases_grades_are_built_as_worked() {
    let cases = hand_cases();
    let mut compared = 0;
    for (id, vector) in &cases {
        let corpus: CareerCorpus =
            serde_json::from_value(vector["input"]["corpus"].clone()).expect("input.corpus");
        let bag: Option<Bag> =
            serde_json::from_value(vector["input"]["bag"].clone()).expect("input.bag");
        let expected: GradedProfile =
            serde_json::from_value(vector["expected"]["profile"].clone()).expect("expected");
        let grades = build_grades(&corpus, &build_bag_profile(&corpus, bag.as_ref()));
        let listed: Vec<ClubId> = grades.clubs.iter().map(|club| club.club).collect();
        let wanted: Vec<ClubId> = expected.clubs.iter().map(|club| club.grades.club).collect();
        assert_eq!(listed, wanted, "{id}: the clubs graded");
        for (built, want) in grades.clubs.iter().zip(&expected.clubs) {
            built
                .validate()
                .unwrap_or_else(|e| panic!("{id}: {}: {e:?}", built.club));
            assert_eq!(
                serde_json::to_value(built).expect("serializes"),
                serde_json::to_value(&want.grades).expect("serializes"),
                "{id}: {}",
                built.club
            );
            compared += 1;
        }
        grades
            .player
            .validate()
            .unwrap_or_else(|e| panic!("{id}: player: {e:?}"));
        assert_eq!(
            serde_json::to_value(&grades.player).expect("serializes"),
            serde_json::to_value(&expected.player.grades).expect("serializes"),
            "{id}: player"
        );
    }
    // Every case lists at least one club, so a case whose clubs went uncompared shows here.
    assert!(
        compared >= cases.len(),
        "{compared} clubs compared across {} cases",
        cases.len()
    );
}

/// Mechanics enters no grade (Decision 12): every pose row of the hand case that carries them,
/// changed or removed, builds the same club grades and the same player's grades.
#[test]
fn no_pose_measurement_moves_a_grade() {
    let (id, vector) = hand_cases()
        .into_iter()
        .find(|(id, _)| id.ends_with("the-list-beside-a-drill-and-pose"))
        .expect("the case carrying pose rows");
    let corpus: CareerCorpus =
        serde_json::from_value(vector["input"]["corpus"].clone()).expect("input.corpus");
    let is_pose = |row: &Measurement| row.source.starts_with("pose:");
    assert!(
        corpus
            .swings
            .iter()
            .any(|swing| swing.measurements.iter().any(is_pose)),
        "{id} carries pose rows"
    );
    let mut moved = corpus.clone();
    let mut removed = corpus.clone();
    for swing in &mut moved.swings {
        for row in swing.measurements.iter_mut().filter(|row| is_pose(row)) {
            row.value = row.value * 7.0 + 3.0;
        }
    }
    for swing in &mut removed.swings {
        swing.measurements.retain(|row| !is_pose(row));
    }
    for club in [ClubId::Driver, ClubId::SevenIron] {
        let baseline = grades_of(&corpus, club);
        assert_eq!(
            grades_of(&moved, club),
            baseline,
            "{id}: {club}, pose rows moved"
        );
        assert_eq!(
            grades_of(&removed, club),
            baseline,
            "{id}: {club}, pose rows removed"
        );
    }
    let player = player_of(&corpus);
    assert_eq!(player_of(&moved), player, "{id}: player, pose rows moved");
    assert_eq!(
        player_of(&removed),
        player,
        "{id}: player, pose rows removed"
    );
}
