//! The graded profile's shapes and constants, before anything builds one. [M37 P1]
//!
//! **No Python twin and no vector yet**: the hand-worked profile family arrives in M37 P5, and its
//! gate in P9. So this holds what a shape can be held to on its own. A whole profile, built here with
//! every variant in it, round trips, and writes an absent value as an absent key. Each check
//! [`Validate`] makes refuses the case it exists for, so a builder that drifts from one of the M37
//! plan's decisions fails at the boundary rather than rendering. The constants are the ones the
//! interview decided, and the borrowed tolerances are the rows it named. The stat-scope table covers
//! every shot number a committed vector records.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use contracts::baseline::{Interval, MetricBaseline};
use contracts::capability::{capability_for, FieldUse, NOT_READINGS};
use contracts::club::{category_of, ClubId};
use contracts::dispersion::{target_for, MetricDispersion};
use contracts::intent::TargetShape;
use contracts::shot::ShotData;
use contracts::strike::{
    is_club_independent, low_point_graded_for, scope_of, Blend, ClubGrades, FaceColumn, FaceRow,
    FindingKind, GradedClub, GradedPlayer, GradedProfile, HeelToe, ImpactCell, ImpactMap,
    OfflineBasis, PlayerGrades, ProfileFinding, RateEstimate, ShapeShares, Share, StatScope,
    StrikeProfile, Topic, TopicGrade, UnscoredCount, WithheldShare, CENTERED_RADIUS_MM,
    CONSISTENCY_FRACTION, ON_LINE_TOLERANCE_ROW, RATE_MINIMUM_N, STAT_SCOPES,
    STRAIGHT_TOLERANCE_ROW,
};
use contracts::unscored::UnscoredReason;
use contracts::Validate;
use serde_json::{json, Value};

// --- Builders -----------------------------------------------------------------------------------

fn interval(low: f64, high: f64) -> Interval {
    Interval {
        low,
        high,
        confidence: Interval::DEFAULT_CONFIDENCE,
    }
}

/// A rate with an interval that holds it. **Not Wilson**: the arithmetic is P2's, and any interval
/// holding the share inside [0, 1] is a valid shape.
fn rate(successes: i64, n: i64) -> RateEstimate {
    let share = successes as f64 / n as f64;
    RateEstimate {
        successes,
        n,
        share,
        interval: interval((share - 0.2).max(0.0), (share + 0.2).min(1.0)),
    }
}

fn graded(successes: i64, n: i64) -> Share {
    Share::Graded(rate(successes, n))
}

fn withheld(have_n: i64) -> Share {
    Share::Withheld(WithheldShare {
        have_n,
        need_n: RATE_MINIMUM_N,
    })
}

fn count(reason: UnscoredReason, n: i64) -> UnscoredCount {
    UnscoredCount { reason, n }
}

fn topic(topic: Topic, judged: i64, grade: Share) -> TopicGrade {
    TopicGrade {
        topic,
        judged,
        grade,
        unscored: Vec::new(),
        shape: None,
    }
}

fn shape_topic(topic: Topic, judged: i64, grade: Share, shares: ShapeShares) -> TopicGrade {
    TopicGrade {
        topic,
        judged,
        grade,
        unscored: Vec::new(),
        shape: Some(shares),
    }
}

fn cell(horizontal: FaceColumn, vertical: Option<FaceRow>, n: i64) -> ImpactCell {
    ImpactCell {
        horizontal,
        vertical,
        n,
    }
}

/// Twelve strikes: two with no vertical offset, ten placed on the 3 x 3 grid, one each off the heel
/// and the toe, leaning 1.5 mm toward the toe.
fn impacts() -> ImpactMap {
    ImpactMap {
        grid: vec![
            cell(FaceColumn::Center, None, 2),
            cell(FaceColumn::Heel, Some(FaceRow::Low), 1),
            cell(FaceColumn::Center, Some(FaceRow::Center), 8),
            cell(FaceColumn::Toe, Some(FaceRow::High), 1),
        ],
        heel_toe: HeelToe {
            n: 12,
            heel: 1,
            toe: 1,
            mean_offset_mm: Some(1.5),
            mean_offset_ci: Some(interval(-1.0, 4.0)),
        },
    }
}

fn baseline(name: &str) -> MetricBaseline {
    serde_json::from_value(json!({
        "name": name, "unit": "mph", "source": "launch_monitor:synthetic",
        "n": 14, "n_sessions": 2, "mean": 101.0, "sd": 1.8,
    }))
    .expect("a baseline")
}

fn dispersion(name: &str) -> MetricDispersion {
    serde_json::from_value(json!({
        "name": name, "unit": "degrees", "source": "launch_monitor:synthetic",
        "n": 14, "n_sessions": 2,
    }))
    .expect("a dispersion")
}

fn finding(metric: &str, kind: FindingKind, ratio: f64) -> ProfileFinding {
    ProfileFinding {
        metric: metric.to_string(),
        kind,
        ratio,
    }
}

/// A 7 iron with four topics in every state a topic can be in: strike location and consistency
/// graded, low point withheld, a fade graded with some landings printed, some projected and two
/// with none.
fn seven_iron() -> ClubGrades {
    let topics = vec![
        TopicGrade {
            unscored: vec![count(UnscoredReason::PrintedBlank, 2)],
            ..topic(Topic::StrikeLocation, 12, graded(9, 12))
        },
        topic(Topic::Consistency, 14, graded(10, 14)),
        TopicGrade {
            unscored: vec![count(UnscoredReason::Misread, 1)],
            ..topic(Topic::LowPoint, 9, withheld(9))
        },
        shape_topic(
            Topic::Fade,
            12,
            graded(6, 10),
            ShapeShares {
                hit_shape: graded(9, 12),
                printed: 4,
                projected: 6,
                landing_unscored: vec![
                    count(UnscoredReason::NoHandedness, 1),
                    count(UnscoredReason::SpinAxisUnresolved, 1),
                ],
            },
        ),
    ];
    let mean = (9.0 / 12.0 + 10.0 / 14.0 + 6.0 / 10.0) / 3.0;
    ClubGrades {
        club: ClubId::SevenIron,
        n_shots: 14,
        strike: StrikeProfile {
            impacts: Some(impacts()),
            ball_speed: Some(baseline("ball_speed_mph")),
            dispersion: BTreeMap::from([(
                "face_to_path_deg".to_string(),
                dispersion("face_to_path_deg"),
            )]),
        },
        topics,
        blend: Some(Blend {
            mean,
            averaged: vec![Topic::StrikeLocation, Topic::Consistency, Topic::Fade],
            excluded: vec![Topic::LowPoint],
        }),
    }
}

/// A driver hit three times on HD Golf: nothing printed that a topic reads, so no topic, no
/// impacts, no blend, and nothing on its list.
fn driver() -> ClubGrades {
    ClubGrades {
        club: ClubId::Driver,
        n_shots: 3,
        strike: StrikeProfile {
            impacts: None,
            ball_speed: None,
            dispersion: BTreeMap::new(),
        },
        topics: Vec::new(),
        blend: None,
    }
}

fn player() -> PlayerGrades {
    PlayerGrades {
        n_shots: 20,
        untagged_shots: 3,
        strike: StrikeProfile {
            impacts: Some(impacts()),
            ball_speed: None,
            dispersion: BTreeMap::from([(
                "face_to_path_deg".to_string(),
                dispersion("face_to_path_deg"),
            )]),
        },
        topics: vec![
            topic(Topic::StrikeLocation, 12, graded(9, 12)),
            shape_topic(
                Topic::Straight,
                4,
                withheld(3),
                ShapeShares {
                    hit_shape: withheld(4),
                    printed: 0,
                    projected: 3,
                    landing_unscored: vec![count(UnscoredReason::SpinAxisUnresolved, 1)],
                },
            ),
        ],
        blend: Some(Blend {
            mean: 0.75,
            averaged: vec![Topic::StrikeLocation],
            excluded: vec![Topic::Straight],
        }),
    }
}

fn profile() -> GradedProfile {
    GradedProfile {
        player_id: "aaron".to_string(),
        clubs: vec![
            GradedClub {
                grades: driver(),
                list: Vec::new(),
            },
            GradedClub {
                grades: seven_iron(),
                list: vec![
                    finding("face_to_path_deg", FindingKind::Bias, 1.8),
                    finding("face_to_path_deg", FindingKind::Scatter, 1.2),
                    finding("start_line_offline_yds", FindingKind::Strength, 0.6),
                ],
            },
        ],
        player: GradedPlayer {
            grades: player(),
            list: vec![finding("start_line_deg", FindingKind::Scatter, 1.3)],
        },
    }
}

/// The refusal's text, from whichever moment caught it.
fn refusal<T: Validate>(value: &T) -> String {
    value.validate().expect_err("refused").to_string()
}

fn assert_refused<T: Validate>(value: &T, says: &str) {
    let message = refusal(value);
    assert!(message.contains(says), "{message:?} does not say {says:?}");
}

// --- The shapes ---------------------------------------------------------------------------------

#[test]
fn a_whole_profile_validates_and_round_trips() {
    let profile = profile();
    profile.validate().expect("the profile is valid");
    let written = serde_json::to_string(&profile).expect("serialize");
    let read: GradedProfile = serde_json::from_str(&written).expect("parse");
    assert_eq!(read, profile);
    assert_eq!(
        serde_json::to_value(&read).unwrap(),
        serde_json::to_value(&profile).unwrap()
    );
}

/// Every `null` in the written profile, except inside the two shapes carried whole from the career
/// family, which write `null` for a withheld statistic as frozen Python's pydantic does and are held
/// to that by the career vectors.
fn nulls(value: &Value, path: &str, out: &mut Vec<String>) {
    match value {
        Value::Null => out.push(path.to_string()),
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                nulls(item, &format!("{path}[{i}]"), out);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                if key != "ball_speed" && key != "dispersion" {
                    nulls(item, &format!("{path}.{key}"), out);
                }
            }
        }
        _ => {}
    }
}

/// ADR-034 §5.5's absent, on the wire: an absent value writes no key, so a reader never meets a
/// `null` it has to remember means "not here". A `null` that arrives is still read as absent.
#[test]
fn an_absent_value_is_an_absent_key_and_never_null() {
    let written = serde_json::to_value(profile()).unwrap();
    let mut found = Vec::new();
    nulls(&written, "", &mut found);
    assert!(found.is_empty(), "{found:?}");

    let driver = &written["clubs"][0]["grades"];
    assert!(driver.get("blend").is_none(), "no blend, no key");
    assert!(driver["strike"].get("impacts").is_none());
    assert!(driver["strike"].get("ball_speed").is_none());
    let seven = &written["clubs"][1]["grades"];
    assert!(
        seven["topics"][1].get("shape").is_none(),
        "consistency is not a shape"
    );
    assert!(seven["topics"][3].get("shape").is_some(), "a fade is");
    assert!(seven["strike"]["impacts"]["grid"][0]
        .get("vertical")
        .is_none());

    let mut spelled = written.clone();
    spelled["clubs"][0]["grades"]["blend"] = Value::Null;
    spelled["clubs"][0]["grades"]["strike"]["impacts"] = Value::Null;
    spelled["clubs"][1]["grades"]["topics"][1]["shape"] = Value::Null;
    spelled["clubs"][1]["grades"]["strike"]["impacts"]["grid"][0]["vertical"] = Value::Null;
    let read: GradedProfile = serde_json::from_value(spelled).expect("null reads as absent");
    assert_eq!(read, profile());
}

#[test]
fn a_rate_refuses_more_successes_than_shots() {
    let mut over = rate(10, 10);
    over.successes = 11;
    assert_refused(&over, "11 successes is not between 0 and n = 10");
    let parsed = serde_json::from_value::<RateEstimate>(json!({
        "successes": 11, "n": 10, "share": 1.1, "interval": {"low": 0.9, "high": 1.0},
    }));
    assert!(parsed.unwrap_err().to_string().contains("11 successes"));

    let mut none = rate(0, 1);
    none.n = 0;
    assert_refused(&none, "a share of no shots");
    let mut negative = rate(0, 10);
    negative.successes = -1;
    assert_refused(&negative, "-1 successes");
}

/// `share` is the quotient, bit for bit; the interval holds it, inside [0, 1].
#[test]
fn a_rate_is_its_quotient_inside_an_interval_that_holds_it() {
    let mut off = rate(7, 10);
    off.share = 0.7000000000000001;
    assert_refused(&off, "RateEstimate.share");
    let seven_twelfths = rate(7, 12);
    assert_eq!(seven_twelfths.share.to_string(), "0.5833333333333334");
    seven_twelfths.validate().unwrap();

    for (low, high) in [(0.75, 0.9), (0.5, 0.65), (-0.01, 0.9), (0.5, 1.01)] {
        let mut loose = rate(7, 10);
        loose.interval = interval(low, high);
        assert_refused(&loose, "RateEstimate.interval");
    }
    for every in [rate(0, 10), rate(10, 10)] {
        every.validate().expect("the ends hold");
    }
}

/// Decision 2: a share is shown at ten judged shots and withheld below.
#[test]
fn a_graded_share_needs_the_floor_and_a_withheld_one_falls_short() {
    graded(5, RATE_MINIMUM_N).validate().unwrap();
    withheld(RATE_MINIMUM_N - 1).validate().unwrap();
    withheld(0).validate().unwrap();
    assert_refused(&graded(5, RATE_MINIMUM_N - 1), "below the floor");
    assert_refused(&withheld(RATE_MINIMUM_N), "WithheldShare.have_n");
    assert_refused(
        &Share::Withheld(WithheldShare {
            have_n: 3,
            need_n: 5,
        }),
        "WithheldShare.need_n",
    );
    let parsed = serde_json::from_value::<Share>(json!({"graded": {
        "successes": 5, "n": 9, "share": 5.0 / 9.0, "interval": {"low": 0.3, "high": 0.8},
    }}));
    assert!(parsed.unwrap_err().to_string().contains("below the floor"));
    assert_eq!(graded(5, 12).n(), 12);
    assert_eq!(withheld(4).n(), 4);
    assert!(withheld(4).graded().is_none());
}

#[test]
fn the_blend_averages_the_graded_topics_and_is_absent_when_none_is() {
    let mut no_blend = seven_iron();
    no_blend.blend = None;
    assert_refused(&no_blend, "absent while");

    let mut wrong = seven_iron();
    wrong.blend.as_mut().unwrap().averaged.push(Topic::LowPoint);
    assert_refused(&wrong, "where the graded topics are");

    let mut unnamed = seven_iron();
    unnamed.blend.as_mut().unwrap().excluded.clear();
    assert_refused(&unnamed, "where the graded topics are");

    let mut nothing = driver();
    nothing.blend = Some(Blend {
        mean: 0.0,
        averaged: Vec::new(),
        excluded: Vec::new(),
    });
    assert_refused(&nothing, "a blend of nothing");

    let mut over = seven_iron();
    over.blend.as_mut().unwrap().mean = 1.5;
    assert_refused(&over, "Blend.mean");
}

#[test]
fn topics_are_listed_once_each_in_topic_order() {
    let mut swapped = seven_iron();
    swapped.topics.swap(0, 1);
    assert_refused(&swapped, "in topic order");

    let mut twice = seven_iron();
    let again = twice.topics[1].clone();
    twice.topics.insert(1, again);
    assert_refused(&twice, "in topic order");

    let mut over = seven_iron();
    over.n_shots = 11;
    assert_refused(&over, "judged 12 of 11 shots");
}

#[test]
fn a_shape_topic_carries_its_shares_and_no_other_topic_does() {
    let mut bare = seven_iron();
    bare.topics[3].shape = None;
    assert_refused(&bare, "a shape topic carries its shape shares");

    let mut stray = seven_iron();
    stray.topics[1].shape = stray.topics[3].shape.clone();
    assert_refused(&stray, "only a shape topic");

    let mut split = seven_iron();
    split.topics[3].shape.as_mut().unwrap().projected = 5;
    assert_refused(
        &split,
        "the printed and projected landings are the grade's shots",
    );

    let mut lost = seven_iron();
    lost.topics[3]
        .shape
        .as_mut()
        .unwrap()
        .landing_unscored
        .pop();
    assert_refused(&lost, "the grade's shots and the landing refusals");

    let mut hit = seven_iron();
    hit.topics[3].shape.as_mut().unwrap().hit_shape = graded(9, 11);
    assert_refused(&hit, "hit_shape is over the judged shots");

    let mut uneven = seven_iron();
    uneven.topics[1].judged = 13;
    assert_refused(&uneven, "the grade is over 14 shots and 13 were judged");
}

#[test]
fn unscored_counts_are_each_reason_once_in_declaration_order() {
    let mut reordered = seven_iron();
    reordered.topics[3]
        .shape
        .as_mut()
        .unwrap()
        .landing_unscored
        .reverse();
    assert_refused(&reordered, "in declaration order");

    let mut empty = seven_iron();
    empty.topics[0].unscored = vec![count(UnscoredReason::PrintedBlank, 0)];
    assert_refused(&empty, "UnscoredCount.n");
}

/// Call 10: the grid is a picture of where, and heel/toe is its outer columns added up.
#[test]
fn the_grid_and_the_lean_agree() {
    let mut heavy = impacts();
    heavy.heel_toe.heel = 2;
    assert_refused(&heavy, "heel_toe's heel");

    let mut total = impacts();
    total.heel_toe.n = 13;
    assert_refused(&total, "heel_toe's n");

    let mut twice = impacts();
    twice.grid.insert(1, cell(FaceColumn::Center, None, 1));
    assert_refused(&twice, "row by row");

    let mut swapped = impacts();
    swapped.grid.swap(1, 2);
    assert_refused(&swapped, "row by row");

    let mut empty = impacts();
    empty.grid[0].n = 0;
    assert_refused(&empty, "ImpactCell.n");
}

/// The lean's mean is a center claim, so it waits for the center floor and comes with its interval.
#[test]
fn the_lean_is_shown_with_its_interval_and_only_from_the_center_floor() {
    let thin = ImpactMap {
        grid: vec![cell(FaceColumn::Center, None, 4)],
        heel_toe: HeelToe {
            n: 4,
            heel: 0,
            toe: 0,
            mean_offset_mm: Some(0.5),
            mean_offset_ci: Some(interval(-3.0, 4.0)),
        },
    };
    assert_refused(&thin, "below the center floor of 5");

    let mut alone = impacts();
    alone.heel_toe.mean_offset_ci = None;
    assert_refused(&alone, "together or not at all");

    let mut outside = impacts();
    outside.heel_toe.mean_offset_ci = Some(interval(2.0, 4.0));
    assert_refused(&outside, "does not hold the mean");

    let mut unclaimed = impacts();
    unclaimed.heel_toe.mean_offset_mm = None;
    unclaimed.heel_toe.mean_offset_ci = None;
    unclaimed.validate().expect("withheld is always allowed");
}

#[test]
fn strike_location_and_the_impact_map_come_together() {
    let mut no_map = seven_iron();
    no_map.strike.impacts = None;
    assert_refused(&no_map, "present together or not at all");

    let mut no_topic = driver();
    no_topic.strike.impacts = Some(impacts());
    assert_refused(&no_topic, "present together or not at all");
}

#[test]
fn the_strike_profile_carries_only_its_own_metrics() {
    let mut wrong_speed = seven_iron();
    wrong_speed.strike.ball_speed = Some(baseline("club_head_speed_mph"));
    assert_refused(&wrong_speed, "StrikeProfile.ball_speed");

    let mut stray = seven_iron();
    stray
        .strike
        .dispersion
        .insert("head_sway_norm".to_string(), dispersion("head_sway_norm"));
    assert_refused(&stray, "StrikeProfile.dispersion");

    let mut misfiled = seven_iron();
    misfiled
        .strike
        .dispersion
        .insert("start_line_deg".to_string(), dispersion("face_to_path_deg"));
    assert_refused(&misfiled, "StrikeProfile.dispersion");
}

/// Decision 7, over every club in the vocabulary, and the refusal it backs.
#[test]
fn low_point_is_graded_for_irons_wedges_and_hybrids_and_nothing_else() {
    let graded_for: Vec<&str> = ClubId::ALL
        .into_iter()
        .filter(|club| low_point_graded_for(category_of(*club)))
        .map(|club| club.as_str())
        .collect();
    let not: Vec<&str> = ClubId::ALL
        .into_iter()
        .filter(|club| !low_point_graded_for(category_of(*club)))
        .map(|club| club.as_str())
        .collect();
    assert_eq!(not, ["driver", "3w", "5w", "7w", "putter"]);
    assert!(graded_for.contains(&"4h") && graded_for.contains(&"7i") && graded_for.contains(&"lw"));

    let mut driver = driver();
    driver.topics = vec![topic(Topic::LowPoint, 3, withheld(3))];
    driver.blend = None;
    assert_refused(&driver, "low point is not graded for a driver");
}

/// Call 17 and ADR-034 §4: the player's raw numbers are club-independent, and so is its list.
#[test]
fn the_player_holds_club_independent_numbers_only() {
    let mut speed = player();
    speed.strike.ball_speed = Some(baseline("ball_speed_mph"));
    assert_refused(&speed, "ball_speed_mph is club-only");

    let mut carry = player();
    carry.strike.dispersion.insert(
        "carry_distance_yds".to_string(),
        dispersion("carry_distance_yds"),
    );
    assert_refused(&carry, "carry_distance_yds is club-only");

    let mut untagged = player();
    untagged.untagged_shots = 21;
    assert_refused(&untagged, "21 untagged of 20 shots");

    let listed = GradedPlayer {
        grades: player(),
        list: vec![finding("carry_distance_yds", FindingKind::Scatter, 1.4)],
    };
    assert_refused(&listed, "carry_distance_yds is club-only");
}

/// ADR-034 §5.7's list: weaknesses then strengths, a metric established one way only, and only a
/// launch-monitor metric with a tolerance, so never a pose one (Decision 12).
#[test]
fn the_list_is_weaknesses_then_strengths_and_never_names_a_pose_metric() {
    let club = |list: Vec<ProfileFinding>| GradedClub {
        grades: seven_iron(),
        list,
    };
    assert_refused(
        &club(vec![
            finding("start_line_offline_yds", FindingKind::Strength, 0.6),
            finding("face_to_path_deg", FindingKind::Bias, 1.8),
        ]),
        "listed after a strength",
    );
    assert_refused(
        &club(vec![
            finding("face_to_path_deg", FindingKind::Bias, 1.8),
            finding("face_to_path_deg", FindingKind::Strength, 0.4),
        ]),
        "both a strength and a weakness",
    );
    assert_refused(
        &club(vec![
            finding("face_to_path_deg", FindingKind::Bias, 1.8),
            finding("face_to_path_deg", FindingKind::Bias, 1.7),
        ]),
        "listed twice",
    );
    // A pose metric has a tolerance and no scope; a flight row has a scope and no tolerance.
    assert!(target_for("head_sway_norm").is_some());
    assert_refused(
        &club(vec![finding("head_sway_norm", FindingKind::Scatter, 1.5)]),
        "is not a launch-monitor metric",
    );
    assert_refused(
        &club(vec![finding("flight_carry_yds", FindingKind::Scatter, 1.5)]),
        "is not a launch-monitor metric",
    );
    assert_refused(
        &club(vec![finding(
            "face_to_path_deg",
            FindingKind::Bias,
            f64::NAN,
        )]),
        "is not a size",
    );
}

#[test]
fn clubs_are_listed_once_each_in_bag_order() {
    let mut reversed = profile();
    reversed.clubs.reverse();
    assert_refused(&reversed, "in bag order");

    let mut twice = profile();
    let again = twice.clubs[1].clone();
    twice.clubs.push(again);
    assert_refused(&twice, "in bag order");
}

// --- The constants ------------------------------------------------------------------------------

/// The interview's numbers (decisions 1, 2 and 4), so moving one is a change to this test as well.
#[test]
fn the_constants_are_the_ones_the_interview_decided() {
    assert_eq!(RATE_MINIMUM_N, 10);
    assert_eq!(CENTERED_RADIUS_MM, 10.0);
    assert_eq!(CONSISTENCY_FRACTION, 0.10);
}

/// Decisions 5 and 8: the straight and on-line tolerances are `METRIC_TARGETS` rows reused, centered
/// on zero, at the 2.0 degrees and 9 yards the interview confirmed.
#[test]
fn the_borrowed_tolerances_are_the_rows_the_interview_named() {
    for (row, tolerance) in [(STRAIGHT_TOLERANCE_ROW, 2.0), (ON_LINE_TOLERANCE_ROW, 9.0)] {
        let target = target_for(row).unwrap_or_else(|| panic!("{row} is not a METRIC_TARGETS row"));
        assert_eq!(target.target, Some(0.0), "{row}");
        assert_eq!(target.tolerance, tolerance, "{row}");
    }
}

#[test]
fn the_wire_names() {
    let topics: Vec<Value> = Topic::ALL
        .into_iter()
        .map(|topic| serde_json::to_value(topic).unwrap())
        .collect();
    assert_eq!(
        topics,
        [
            "strike_location",
            "consistency",
            "low_point",
            "fade",
            "draw",
            "straight"
        ]
        .map(|name| json!(name))
    );
    for topic in Topic::ALL {
        assert_eq!(serde_json::to_value(topic).unwrap(), json!(topic.as_str()));
    }
    for basis in [OfflineBasis::Printed, OfflineBasis::Projected] {
        assert_eq!(serde_json::to_value(basis).unwrap(), json!(basis.as_str()));
    }
    for kind in [
        FindingKind::Bias,
        FindingKind::Scatter,
        FindingKind::Strength,
    ] {
        assert_eq!(serde_json::to_value(kind).unwrap(), json!(kind.as_str()));
    }
    assert_eq!(
        serde_json::to_value(StatScope::ClubIndependent).unwrap(),
        json!("club_independent")
    );
}

/// The three shapes are three topics, a shape names its topic both ways, and only consistency and
/// low point need a club (call 9).
#[test]
fn the_shapes_are_three_topics_and_two_topics_need_a_club() {
    for shape in [TargetShape::Fade, TargetShape::Draw, TargetShape::Straight] {
        assert_eq!(Topic::for_shape(shape).shape(), Some(shape));
    }
    let shapes = Topic::ALL
        .iter()
        .filter(|topic| topic.shape().is_some())
        .count();
    assert_eq!(shapes, 3);
    let need: Vec<Topic> = Topic::ALL
        .into_iter()
        .filter(|topic| topic.needs_a_club())
        .collect();
    assert_eq!(need, [Topic::Consistency, Topic::LowPoint]);
}

/// The keys a serialized `ShotData` writes, read off an instance so the struct is the only list.
fn shot_data_keys() -> BTreeSet<String> {
    let shot: ShotData = serde_json::from_value(json!({
        "shot_id": "s", "session_id": "d", "timestamp": "2026-10-10T00:00:00Z", "source": "screen",
    }))
    .expect("a valid shot");
    serde_json::to_value(shot)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

/// Call 7's inputs are readings, `synthetic` analyses every one, and HD Golf prints neither strike
/// location's nor low point's, which is decision 3 and finding 3: on every stored shot those two
/// topics are absent.
#[test]
fn every_topic_input_is_a_reading_and_hd_golf_prints_no_strike_or_low_point() {
    let keys = shot_data_keys();
    let synthetic = capability_for("synthetic").expect("declared");
    let hd_golf = capability_for("hd_golf").expect("declared");
    for topic in Topic::ALL {
        for input in topic.inputs() {
            assert!(keys.contains(*input), "{input} is not a ShotData key");
            assert!(!NOT_READINGS.contains(input), "{input} is not a reading");
            assert_eq!(synthetic.use_of(input), Some(FieldUse::Analysed), "{input}");
        }
        let hd_golf_prints = topic
            .inputs()
            .iter()
            .all(|input| hd_golf.use_of(input) == Some(FieldUse::Analysed));
        let expected = !matches!(topic, Topic::StrikeLocation | Topic::LowPoint);
        assert_eq!(hd_golf_prints, expected, "{topic:?} on hd_golf");
    }
}

// --- The stat scopes ----------------------------------------------------------------------------

/// Call 17's table as decided, once each.
#[test]
fn the_scope_table_is_call_17() {
    let names: Vec<&str> = STAT_SCOPES.iter().map(|(name, _)| *name).collect();
    let distinct: BTreeSet<&str> = names.iter().copied().collect();
    assert_eq!(names.len(), distinct.len(), "a name twice: {names:?}");

    let independent: BTreeSet<&str> = STAT_SCOPES
        .iter()
        .filter(|(_, scope)| *scope == StatScope::ClubIndependent)
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(
        independent,
        BTreeSet::from([
            "face_to_path_deg",
            "start_line_deg",
            "impact_offset_h",
            "impact_offset_v"
        ])
    );
    for club_only in [
        "carry_distance_yds",
        "total_distance_yds",
        "ball_speed_mph",
        "launch_angle_deg",
        "start_line_offline_yds",
        "flight_spin_rpm",
    ] {
        assert_eq!(
            scope_of(club_only),
            Some(StatScope::ClubOnly),
            "{club_only}"
        );
    }
    // The offsets are keyed by their ShotData names, there being no measurement row for either.
    let keys = shot_data_keys();
    assert!(keys.contains("impact_offset_h") && keys.contains("impact_offset_v"));
    // A pose metric has no scope, so no player-level number.
    for pose in ["head_sway_norm", "tempo_ratio", "hip_sway_norm"] {
        assert_eq!(scope_of(pose), None, "{pose}");
        assert!(!is_club_independent(pose));
    }
}

/// Every shot and flight row the committed shot family records has a scope. `analysis`'s
/// `shot_measure` tests hold the registries themselves to the table; this holds it to the numbers on
/// disk, from the side of the crate that cannot see `analysis`.
#[test]
fn every_shot_number_a_committed_vector_records_has_a_scope() {
    let mut recorded = BTreeSet::new();
    for path in common::vector_files("shot") {
        let vector = common::read_vector(&path);
        for row in vector["expected"]["measurements"]
            .as_array()
            .unwrap_or_else(|| panic!("{}: no expected.measurements", path.display()))
        {
            recorded.insert(row["name"].as_str().expect("a row name").to_string());
        }
    }
    let unscoped: Vec<&String> = recorded
        .iter()
        .filter(|name| scope_of(name).is_none())
        .collect();
    assert!(unscoped.is_empty(), "{unscoped:?}");
    assert!(
        recorded.contains("face_to_path_deg") && recorded.contains("flight_carry_yds"),
        "the check is not vacuous: {recorded:?}"
    );
}
