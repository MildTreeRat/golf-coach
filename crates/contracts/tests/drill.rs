//! `contracts::drill`: the impact drill's stored strike, each club's place on the ladder, and the
//! ladder's tables. [M46 P6]
//!
//! **No Python twin and no vector**: nothing stores a strike before the drill store (P17) and the
//! facade (P18), so this holds the shapes to what they can be held to on their own (the M46 plan's C7
//! to C14). Every wire name is pinned, because the store writes them to disk and the app and the lab
//! read them back. A whole strike of each kind, and a whole state, round trips with an absent value as
//! an absent key. Each check [`Validate`] makes refuses the case it exists for, assembled in code and
//! read from JSON. Every shipped table is C9's, cut at its label's inscribed radius as `impact::tape`
//! measures it, which is why this crate takes `impact` as a dev-dependency.

use std::collections::BTreeMap;
use std::fmt::Debug;

use contracts::club::{category_of, ClubId};
use contracts::drill::{
    crop_file, label_id, ladder_radii, strike_id, AppliedLabel, ClubDrillState, DrillStage,
    DrillState, DrillTopic, LevelEntry, StrikeCall, StrikeSource, Suggestion, TapeMethod,
    TapeNotRead, TapeStrike, BASELINE_HOLDS, BLOCK_SIZE, DIFFERENTIAL_AFTER_BLOCKS,
    FRESH_BASELINE_AT, MOVE_IN_AT, MOVE_OUT_AT, PATTERN_WINDOW, RECENTRE_FRACTION,
};
use contracts::face_impact::{ImpactMethod, TAPE_READER_VERSION};
use contracts::intent::ClubCategory;
use contracts::{Timestamp, Validate};
use impact::tape::{category_label, CategoryLabel};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

const WHEN: &str = "2026-11-12T20:00:00Z";

fn when() -> Timestamp {
    Timestamp::parse(WHEN).unwrap()
}

// --- Wire names ---------------------------------------------------------------------------------

/// Each value writes as its wire name and reads back from it.
fn pins_wire_names<T>(table: &[(T, &str)])
where
    T: Serialize + DeserializeOwned + PartialEq + Debug + Copy,
{
    for (value, wire) in table {
        assert_eq!(
            serde_json::to_value(value).unwrap(),
            json!(wire),
            "{value:?}"
        );
        assert_eq!(
            serde_json::from_value::<T>(json!(wire)).unwrap(),
            *value,
            "{wire}"
        );
    }
}

/// A variant added to any of the vocabulary's enums fails to compile here until its wire name is in
/// the table below. Rust has no reflection over an enum, so this is the guard on the lists.
#[allow(dead_code)]
fn every_variant_is_listed(
    topic: DrillTopic,
    method: TapeMethod,
    stage: DrillStage,
    call: StrikeCall,
    source: StrikeSource,
    reason: TapeNotRead,
    suggestion: Suggestion,
) {
    match topic {
        DrillTopic::Impact => {}
    }
    match method {
        TapeMethod::StrikeTape => {}
    }
    match stage {
        DrillStage::Baseline
        | DrillStage::Ladder
        | DrillStage::Recentre
        | DrillStage::Differential => {}
    }
    match call {
        StrikeCall::Heel | StrikeCall::Centre | StrikeCall::Toe => {}
    }
    match source {
        StrikeSource::Read | StrikeSource::Picked | StrikeSource::DeclaredOffTape => {}
    }
    match reason {
        TapeNotRead::NoLabel
        | TapeNotRead::NoOrientation
        | TapeNotRead::NoNewMark
        | TapeNotRead::MarkClipped
        | TapeNotRead::DecodeFailed => {}
    }
    match suggestion {
        Suggestion::Recentre
        | Suggestion::Differential
        | Suggestion::ReturnToLadder
        | Suggestion::FreshBaseline => {}
    }
}

const EVERY_REASON: [(TapeNotRead, &str); 5] = [
    (TapeNotRead::NoLabel, "no_label"),
    (TapeNotRead::NoOrientation, "no_orientation"),
    (TapeNotRead::NoNewMark, "no_new_mark"),
    (TapeNotRead::MarkClipped, "mark_clipped"),
    (TapeNotRead::DecodeFailed, "decode_failed"),
];

#[test]
fn every_wire_name_is_pinned() {
    pins_wire_names(&[(DrillTopic::Impact, "impact")]);
    pins_wire_names(&[(TapeMethod::StrikeTape, "strike_tape")]);
    pins_wire_names(&[
        (DrillStage::Baseline, "baseline"),
        (DrillStage::Ladder, "ladder"),
        (DrillStage::Recentre, "recentre"),
        (DrillStage::Differential, "differential"),
    ]);
    pins_wire_names(&[
        (StrikeCall::Heel, "heel"),
        (StrikeCall::Centre, "centre"),
        (StrikeCall::Toe, "toe"),
    ]);
    pins_wire_names(&[
        (StrikeSource::Read, "read"),
        (StrikeSource::Picked, "picked"),
        (StrikeSource::DeclaredOffTape, "declared_off_tape"),
    ]);
    pins_wire_names(&EVERY_REASON);
    pins_wire_names(&[
        (Suggestion::Recentre, "recentre"),
        (Suggestion::Differential, "differential"),
        (Suggestion::ReturnToLadder, "return_to_ladder"),
        (Suggestion::FreshBaseline, "fresh_baseline"),
    ]);
}

/// The topic and the method name the drill's directory, `drills/<player>/impact/strike_tape/` (C7),
/// so the path's spelling and the wire's are one.
#[test]
fn the_directory_names_are_the_wire_names() {
    assert_eq!(
        serde_json::to_value(DrillTopic::Impact).unwrap(),
        json!(DrillTopic::Impact.as_str())
    );
    assert_eq!(
        serde_json::to_value(TapeMethod::StrikeTape).unwrap(),
        json!(TapeMethod::StrikeTape.as_str())
    );
    assert_eq!(DrillTopic::Impact.as_str(), "impact");
    assert_eq!(TapeMethod::StrikeTape.as_str(), "strike_tape");
}

/// A tape drill's readings are `face_impact`'s tape readings, and the two spell it the same.
#[test]
fn a_tape_drill_reads_by_the_tape_method() {
    assert_eq!(
        TapeMethod::StrikeTape.impact_method(),
        ImpactMethod::StrikeTape
    );
    assert_eq!(
        serde_json::to_value(TapeMethod::StrikeTape).unwrap(),
        serde_json::to_value(ImpactMethod::StrikeTape).unwrap()
    );
}

// --- The ids ------------------------------------------------------------------------------------

#[test]
fn the_ids_are_spelled_as_c8_spells_them() {
    assert_eq!(strike_id("3fa2", 17), "3fa2-17");
    assert_eq!(label_id("3fa2", 3), "3fa2-L3");
    assert_eq!(crop_file("3fa2-17"), "crops/3fa2-17.jpg");
}

// --- The strike ---------------------------------------------------------------------------------

/// A ladder strike, read: the record every other strike below is a variation of.
fn read_strike() -> TapeStrike {
    TapeStrike {
        strike_id: "3fa2-17".into(),
        player_id: "aaron".into(),
        club: ClubId::SevenIron,
        category: ClubCategory::MidIron,
        label_type: "strikepro_iron".into(),
        label_id: "3fa2-L3".into(),
        n_on_label: 4,
        captured_at: when(),
        stage: DrillStage::Ladder,
        level_radius_mm: Some(12.0),
        call: None,
        offset_h_mm: Some(8.4),
        offset_v_mm: Some(-2.1),
        source: Some(StrikeSource::Read),
        not_read: None,
        reader_version: TAPE_READER_VERSION,
        crop: Some("crops/3fa2-17.jpg".into()),
    }
}

/// A differential strike the golfer picked, called to the toe.
fn picked_strike() -> TapeStrike {
    TapeStrike {
        stage: DrillStage::Differential,
        call: Some(StrikeCall::Toe),
        offset_h_mm: Some(14.0),
        offset_v_mm: Some(0.5),
        source: Some(StrikeSource::Picked),
        ..read_strike()
    }
}

/// "Missed the tape": counted, with no position, on the crop the `no_new_mark` read left.
fn declared_strike() -> TapeStrike {
    TapeStrike {
        offset_h_mm: None,
        offset_v_mm: None,
        source: Some(StrikeSource::DeclaredOffTape),
        ..read_strike()
    }
}

/// A baseline strike whose photo held no label: no position, no circle and no crop.
fn not_read_strike() -> TapeStrike {
    TapeStrike {
        stage: DrillStage::Baseline,
        level_radius_mm: None,
        offset_h_mm: None,
        offset_v_mm: None,
        source: None,
        not_read: Some(TapeNotRead::NoLabel),
        crop: None,
        ..read_strike()
    }
}

#[test]
fn a_read_strike_writes_every_field_c8_names_by_its_wire_name() {
    let wire = json!({
        "strike_id": "3fa2-17", "player_id": "aaron", "club": "7i", "category": "mid_iron",
        "label_type": "strikepro_iron", "label_id": "3fa2-L3", "n_on_label": 4,
        "captured_at": WHEN, "stage": "ladder", "level_radius_mm": 12.0,
        "offset_h_mm": 8.4, "offset_v_mm": -2.1, "source": "read", "reader_version": 1,
        "crop": "crops/3fa2-17.jpg",
    });
    assert_eq!(serde_json::to_value(read_strike()).unwrap(), wire);
    assert_eq!(
        serde_json::from_value::<TapeStrike>(wire).unwrap(),
        read_strike()
    );
}

/// What was not known is an absent key, never a `null` and never 0 (P5 finding 2's convention): a
/// strike not read writes no offset, no source, no call, no circle and no crop.
#[test]
fn a_strike_not_read_writes_an_absent_value_as_an_absent_key() {
    let wire = json!({
        "strike_id": "3fa2-17", "player_id": "aaron", "club": "7i", "category": "mid_iron",
        "label_type": "strikepro_iron", "label_id": "3fa2-L3", "n_on_label": 4,
        "captured_at": WHEN, "stage": "baseline", "not_read": "no_label", "reader_version": 1,
    });
    assert_eq!(serde_json::to_value(not_read_strike()).unwrap(), wire);
    assert_eq!(
        serde_json::from_value::<TapeStrike>(wire).unwrap(),
        not_read_strike()
    );
}

#[test]
fn a_null_that_arrives_reads_as_absent() {
    let mut wire = serde_json::to_value(not_read_strike()).unwrap();
    for key in [
        "level_radius_mm",
        "call",
        "offset_h_mm",
        "offset_v_mm",
        "source",
        "crop",
    ] {
        wire[key] = Value::Null;
    }
    assert_eq!(
        serde_json::from_value::<TapeStrike>(wire).unwrap(),
        not_read_strike()
    );
}

#[test]
fn every_kind_of_strike_round_trips() {
    for strike in [
        read_strike(),
        picked_strike(),
        declared_strike(),
        not_read_strike(),
    ] {
        assert_eq!(strike.validate(), Ok(()), "{strike:?}");
        let wire = serde_json::to_value(&strike).unwrap();
        assert_eq!(serde_json::from_value::<TapeStrike>(wire).unwrap(), strike);
    }
}

/// C10: read, picked and declared off the tape count; a strike not read does not. Derived from
/// `source`, never stored.
#[test]
fn a_strike_counts_unless_it_was_not_read() {
    assert!(read_strike().counts());
    assert!(picked_strike().counts());
    assert!(declared_strike().counts());
    assert!(!not_read_strike().counts());
    assert_eq!(read_strike().position(), Some((8.4, -2.1)));
    assert_eq!(declared_strike().position(), None);
    assert_eq!(not_read_strike().position(), None);
}

/// The strike is refused, naming `field`.
fn refused(strike: TapeStrike, field: &str) {
    let err = strike.validate().expect_err(&format!("refused at {field}"));
    assert_eq!(err.field, field, "{err}");
}

// The plan's four refusals (P6's Validate line).

#[test]
fn an_offset_on_a_strike_declared_off_the_tape_is_refused() {
    refused(
        TapeStrike {
            offset_h_mm: Some(3.0),
            ..declared_strike()
        },
        "TapeStrike.offset_h_mm",
    );
    refused(
        TapeStrike {
            offset_v_mm: Some(0.0),
            ..declared_strike()
        },
        "TapeStrike.offset_h_mm",
    );
}

#[test]
fn a_read_or_picked_strike_with_no_position_is_refused() {
    for strike in [read_strike(), picked_strike()] {
        refused(
            TapeStrike {
                offset_h_mm: None,
                offset_v_mm: None,
                ..strike.clone()
            },
            "TapeStrike.offset_h_mm",
        );
        // One axis is not a position either: every circle is `hypot(h, v)`.
        refused(
            TapeStrike {
                offset_v_mm: None,
                ..strike.clone()
            },
            "TapeStrike.offset_h_mm",
        );
        refused(
            TapeStrike {
                offset_h_mm: None,
                ..strike
            },
            "TapeStrike.offset_h_mm",
        );
    }
}

#[test]
fn a_call_outside_the_differential_stage_is_refused() {
    for stage in [
        DrillStage::Baseline,
        DrillStage::Ladder,
        DrillStage::Recentre,
    ] {
        let level_radius_mm = (stage != DrillStage::Baseline).then_some(12.0);
        refused(
            TapeStrike {
                stage,
                level_radius_mm,
                call: Some(StrikeCall::Centre),
                ..read_strike()
            },
            "TapeStrike.call",
        );
    }
}

#[test]
fn a_strike_before_its_labels_first_is_refused() {
    refused(
        TapeStrike {
            n_on_label: 0,
            ..read_strike()
        },
        "TapeStrike.n_on_label",
    );
    let err = TapeStrike {
        n_on_label: -1,
        ..read_strike()
    }
    .validate()
    .unwrap_err();
    assert_eq!(err.to_string(), "TapeStrike.n_on_label: -1 is not >= 1");
}

// The rest, each the other half of a rule above or what makes the shape mean one thing.

#[test]
fn a_differential_strike_with_no_call_is_refused() {
    refused(
        TapeStrike {
            call: None,
            ..picked_strike()
        },
        "TapeStrike.call",
    );
}

#[test]
fn a_strike_is_placed_declared_or_not_read_and_only_one() {
    refused(
        TapeStrike {
            not_read: Some(TapeNotRead::NoNewMark),
            ..declared_strike()
        },
        "TapeStrike.not_read",
    );
    refused(
        TapeStrike {
            not_read: None,
            ..not_read_strike()
        },
        "TapeStrike.source",
    );
}

#[test]
fn a_strike_not_read_holding_an_offset_is_refused() {
    refused(
        TapeStrike {
            offset_h_mm: Some(0.0),
            ..not_read_strike()
        },
        "TapeStrike.offset_h_mm",
    );
}

#[test]
fn a_non_finite_offset_is_refused() {
    for bad in [f64::NAN, f64::INFINITY] {
        refused(
            TapeStrike {
                offset_v_mm: Some(bad),
                ..read_strike()
            },
            "TapeStrike.offset_v_mm",
        );
    }
}

#[test]
fn a_circle_is_named_past_the_baseline_and_only_there() {
    refused(
        TapeStrike {
            level_radius_mm: Some(12.0),
            ..not_read_strike()
        },
        "TapeStrike.level_radius_mm",
    );
    refused(
        TapeStrike {
            level_radius_mm: None,
            ..read_strike()
        },
        "TapeStrike.level_radius_mm",
    );
    for bad in [0.0, -4.0, f64::NAN, f64::INFINITY] {
        refused(
            TapeStrike {
                level_radius_mm: Some(bad),
                ..read_strike()
            },
            "TapeStrike.level_radius_mm",
        );
    }
}

/// 0 is not an entry on the reader's ledger (`TAPE_READER_VERSION`'s doc).
#[test]
fn a_strike_naming_no_reader_is_refused() {
    refused(
        TapeStrike {
            reader_version: 0,
            ..read_strike()
        },
        "TapeStrike.reader_version",
    );
}

#[test]
fn a_crop_is_the_strikes_own_and_a_position_has_one() {
    refused(
        TapeStrike {
            crop: Some("crops/3fa2-16.jpg".into()),
            ..read_strike()
        },
        "TapeStrike.crop",
    );
    for strike in [read_strike(), picked_strike()] {
        refused(
            TapeStrike {
                crop: None,
                ..strike
            },
            "TapeStrike.crop",
        );
    }
    // A declaration made on no crop is still a declaration.
    assert_eq!(
        TapeStrike {
            crop: None,
            ..declared_strike()
        }
        .validate(),
        Ok(())
    );
}

/// The drill's directory is named by the player id (C7), so it is held to the slug rule.
#[test]
fn a_player_id_that_is_not_a_slug_is_refused() {
    refused(
        TapeStrike {
            player_id: "../aaron".into(),
            ..read_strike()
        },
        "TapeStrike.player_id",
    );
}

/// Reading a strike from disk applies every check: the store cannot hand the ladder a strike the
/// shape refuses.
#[test]
fn a_refused_strike_is_refused_when_read_from_json() {
    let mut wire = serde_json::to_value(declared_strike()).unwrap();
    wire["offset_h_mm"] = json!(3.0);
    let err = serde_json::from_value::<TapeStrike>(wire).unwrap_err();
    assert!(
        err.to_string().contains("TapeStrike.offset_h_mm"),
        "the refusal names the field: {err}"
    );
}

// --- The reasons --------------------------------------------------------------------------------

/// `retake_helps` is the bit the reading card branches on, and it splits the reasons by whether the
/// camera caused them. A mark run off the label's edge is in every photo of that strike.
#[test]
fn a_retake_helps_exactly_where_the_photo_is_the_problem() {
    let helps: Vec<TapeNotRead> = EVERY_REASON
        .iter()
        .map(|(reason, _)| *reason)
        .filter(|reason| reason.retake_helps())
        .collect();
    assert_eq!(
        helps,
        vec![
            TapeNotRead::NoLabel,
            TapeNotRead::NoOrientation,
            TapeNotRead::NoNewMark,
            TapeNotRead::DecodeFailed,
        ]
    );
}

#[test]
fn every_reason_has_a_sentence_and_only_a_retake_asks_for_one() {
    let mut seen = Vec::new();
    for (reason, _) in EVERY_REASON {
        let sentence = reason.sentence();
        assert!(sentence.ends_with('.'), "{reason:?}: {sentence}");
        assert!(
            !sentence.chars().any(|c| c.is_ascii_digit()),
            "{reason:?}: {sentence}"
        );
        assert!(!seen.contains(&sentence), "{reason:?} repeats a sentence");
        seen.push(sentence);
        assert_eq!(
            sentence.contains("take it again") || sentence.contains("Take it again"),
            reason.retake_helps(),
            "{reason:?}: {sentence}"
        );
    }
}

// --- The state ----------------------------------------------------------------------------------

fn ids(range: std::ops::RangeInclusive<i64>) -> Vec<String> {
    range.map(|n| strike_id("3fa2", n)).collect()
}

/// A 7 iron two judged blocks in, with a suggestion waiting, and a driver in its first baseline.
fn state() -> DrillState {
    let iron = ClubDrillState {
        stage: DrillStage::Ladder,
        level: Some(1),
        block: ids(21..=23),
        suggestion: Some(Suggestion::Recentre),
        history: vec![
            LevelEntry {
                at: when(),
                from: None,
                to: Some(0),
                block: ids(1..=10),
            },
            LevelEntry {
                at: when(),
                from: Some(0),
                to: Some(1),
                block: ids(11..=20),
            },
        ],
        label: Some(AppliedLabel {
            label_id: label_id("3fa2", 3),
            n_on_label: 4,
        }),
    };
    let driver = ClubDrillState {
        stage: DrillStage::Baseline,
        level: None,
        block: Vec::new(),
        suggestion: None,
        history: Vec::new(),
        label: None,
    };
    DrillState {
        player_id: "aaron".into(),
        strikes_issued: 23,
        labels_issued: 3,
        clubs: BTreeMap::from([(ClubId::SevenIron, iron), (ClubId::Driver, driver)]),
        updated_at: when(),
    }
}

#[test]
fn a_state_writes_every_field_c7_names_by_its_wire_name() {
    let wire = json!({
        "player_id": "aaron", "strikes_issued": 23, "labels_issued": 3,
        "clubs": {
            "7i": {
                "stage": "ladder", "level": 1, "block": ["3fa2-21", "3fa2-22", "3fa2-23"],
                "suggestion": "recentre",
                "history": [
                    {"at": WHEN, "to": 0, "block": ids(1..=10)},
                    {"at": WHEN, "from": 0, "to": 1, "block": ids(11..=20)},
                ],
                "label": {"label_id": "3fa2-L3", "n_on_label": 4},
            },
            "driver": {"stage": "baseline", "block": [], "history": []},
        },
        "updated_at": WHEN,
    });
    assert_eq!(serde_json::to_value(state()).unwrap(), wire);
    assert_eq!(serde_json::from_value::<DrillState>(wire).unwrap(), state());
}

/// The clubs write in bag order, as `Bag`'s entries do, so the driver comes before the 7 iron where
/// the alphabet would put it after.
#[test]
fn the_clubs_write_in_bag_order() {
    let text = serde_json::to_string(&state()).unwrap();
    assert!(text.find("\"driver\"").unwrap() < text.find("\"7i\"").unwrap());
}

#[test]
fn a_club_with_no_drill_yet_reads_from_the_fewest_keys() {
    let club: ClubDrillState = serde_json::from_value(json!({"stage": "baseline"})).unwrap();
    assert_eq!(club, state().clubs[&ClubId::Driver]);
}

/// The state is refused, naming `field`.
fn state_refused(state: DrillState, field: &str) {
    let err = state.validate().expect_err(&format!("refused at {field}"));
    assert_eq!(err.field, field, "{err}");
}

fn with_iron(edit: impl FnOnce(&mut ClubDrillState)) -> DrillState {
    let mut state = state();
    edit(state.clubs.get_mut(&ClubId::SevenIron).unwrap());
    state
}

#[test]
fn a_club_past_its_first_baseline_with_no_level_is_refused() {
    for stage in [
        DrillStage::Ladder,
        DrillStage::Recentre,
        DrillStage::Differential,
    ] {
        state_refused(
            with_iron(|club| {
                club.stage = stage;
                club.level = None;
            }),
            "DrillState.clubs[7i] -> ClubDrillState.level",
        );
    }
    // A fresh baseline keeps the level it had until the new one is set.
    let fresh = with_iron(|club| club.stage = DrillStage::Baseline);
    assert_eq!(fresh.validate(), Ok(()));
}

#[test]
fn a_stored_block_of_ten_is_refused() {
    assert_eq!(BLOCK_SIZE, 10);
    let nine = with_iron(|club| club.block = ids(21..=29));
    assert_eq!(nine.validate(), Ok(()));
    state_refused(
        with_iron(|club| club.block = ids(21..=30)),
        "DrillState.clubs[7i] -> ClubDrillState.block",
    );
}

#[test]
fn a_label_with_no_strike_on_it_is_refused() {
    state_refused(
        with_iron(|club| {
            club.label = Some(AppliedLabel {
                label_id: label_id("3fa2", 3),
                n_on_label: 0,
            })
        }),
        "DrillState.clubs[7i] -> ClubDrillState.label -> AppliedLabel.n_on_label",
    );
}

#[test]
fn a_level_past_its_table_is_refused() {
    let tightest = ladder_radii(ClubCategory::MidIron).len() - 1;
    assert_eq!(
        with_iron(|club| club.level = Some(tightest)).validate(),
        Ok(())
    );
    state_refused(
        with_iron(|club| club.level = Some(tightest + 1)),
        "DrillState.clubs[7i].level",
    );
}

/// The putter takes no label (P3 finding 8), so it has no ladder and no drill.
#[test]
fn a_club_with_no_ladder_holds_no_drill() {
    let mut putter = state();
    let driver = putter.clubs[&ClubId::Driver].clone();
    putter.clubs.insert(ClubId::Putter, driver);
    state_refused(putter, "DrillState.clubs[putter]");
}

#[test]
fn a_state_with_a_bad_owner_or_counter_is_refused() {
    state_refused(
        DrillState {
            player_id: "Aaron".into(),
            ..state()
        },
        "DrillState.player_id",
    );
    state_refused(
        DrillState {
            strikes_issued: -1,
            ..state()
        },
        "DrillState.strikes_issued",
    );
    state_refused(
        DrillState {
            labels_issued: -1,
            ..state()
        },
        "DrillState.labels_issued",
    );
}

#[test]
fn a_refused_state_is_refused_when_read_from_json() {
    let mut wire = serde_json::to_value(state()).unwrap();
    wire["clubs"]["7i"]["level"] = json!(99);
    let err = serde_json::from_value::<DrillState>(wire).unwrap_err();
    assert!(
        err.to_string().contains("DrillState.clubs[7i].level"),
        "{err}"
    );
}

// --- The constants and the tables ---------------------------------------------------------------

/// The user's numbers from the M46 interview (D8, C10 to C13), held here so that moving one is a
/// visible change.
#[test]
fn the_constants_are_the_interviews() {
    assert_eq!(BLOCK_SIZE, 10);
    assert_eq!(MOVE_IN_AT, 7);
    assert_eq!(MOVE_OUT_AT, 3);
    assert_eq!(BASELINE_HOLDS, 5);
    assert_eq!(PATTERN_WINDOW, 20);
    assert_eq!(RECENTRE_FRACTION, 1.0 / 3.0);
    assert_eq!(DIFFERENTIAL_AFTER_BLOCKS, 2);
    assert_eq!(FRESH_BASELINE_AT, 7);
    // Moving in and moving out never overlap, and a stay lies between them.
    const { assert!(MOVE_OUT_AT < MOVE_IN_AT && MOVE_IN_AT <= BLOCK_SIZE) };
}

/// Every category, with a guard that fails to compile when `ClubCategory` gains one.
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

#[allow(dead_code)]
fn every_category_is_listed(category: ClubCategory) {
    match category {
        ClubCategory::Driver
        | ClubCategory::Wood
        | ClubCategory::Hybrid
        | ClubCategory::LongIron
        | ClubCategory::MidIron
        | ClubCategory::ShortIron
        | ClubCategory::Wedge
        | ClubCategory::Putter
        | ClubCategory::All => {}
    }
}

/// C9's tables before either cut, the user's (D9), as the module doc lists them. `all` has none.
const UNCUT: [(ClubCategory, &[f64]); 8] = [
    (
        ClubCategory::Driver,
        &[30.0, 26.0, 22.0, 19.0, 16.0, 14.0, 12.0, 10.0],
    ),
    (
        ClubCategory::Wood,
        &[28.0, 24.0, 21.0, 18.0, 15.0, 13.0, 11.0, 9.0],
    ),
    (
        ClubCategory::Hybrid,
        &[26.0, 22.0, 19.0, 16.0, 14.0, 12.0, 10.0, 8.0],
    ),
    (
        ClubCategory::LongIron,
        &[24.0, 21.0, 18.0, 15.0, 13.0, 11.0, 9.0, 8.0, 7.0],
    ),
    (
        ClubCategory::MidIron,
        &[22.0, 19.0, 16.0, 14.0, 12.0, 10.0, 9.0, 8.0, 7.0, 6.0],
    ),
    (
        ClubCategory::ShortIron,
        &[20.0, 17.0, 15.0, 13.0, 11.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0],
    ),
    (
        ClubCategory::Wedge,
        &[18.0, 16.0, 14.0, 12.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0],
    ),
    (
        ClubCategory::Putter,
        &[15.0, 12.0, 10.0, 8.0, 7.0, 6.0, 5.0, 4.0],
    ),
];

/// P4 finding 6: every category is in `impact::tape`'s map, as a label or as named to have none, and
/// a missing row fails here rather than defaulting. A category with a label has a ladder, strictly
/// tightening and never wider than the label can hold, so a strike off the tape is outside every
/// circle (C6). A category with none has no ladder.
#[test]
fn every_table_tightens_and_stays_on_its_label() {
    for category in EVERY_CATEGORY {
        let radii = ladder_radii(category);
        let label = category_label(category.as_str())
            .unwrap_or_else(|| panic!("impact::tape has no row for {}", category.as_str()));
        match label {
            CategoryLabel::Label(label) => {
                assert!(!radii.is_empty(), "{category:?} has a label and no ladder");
                assert!(radii.iter().all(|r| *r > 0.0), "{category:?}");
                assert!(
                    radii.windows(2).all(|pair| pair[0] > pair[1]),
                    "{category:?} does not strictly tighten: {radii:?}"
                );
                assert!(
                    radii[0] <= label.inscribed_radius_mm,
                    "{category:?}'s widest {} is wider than {}'s {} mm",
                    radii[0],
                    label.id,
                    label.inscribed_radius_mm
                );
            }
            CategoryLabel::NoLabel { .. } => {
                assert!(radii.is_empty(), "{category:?} has no label and a ladder")
            }
        }
    }
}

/// What ships is C9's table with the first cut applied and nothing else (the module doc): every
/// radius the label holds is kept, and none it does not. **P29's floor** changes this test when it
/// lands, by removing the radii below twice the measured P95.
#[test]
fn every_table_is_c9s_trimmed_at_its_labels_inscribed_radius() {
    for (category, uncut) in UNCUT {
        let expected: Vec<f64> = match category_label(category.as_str()).unwrap() {
            CategoryLabel::Label(label) => uncut
                .iter()
                .copied()
                .filter(|r| *r <= label.inscribed_radius_mm)
                .collect(),
            CategoryLabel::NoLabel { .. } => Vec::new(),
        };
        assert_eq!(ladder_radii(category), expected, "{category:?}");
    }
    assert!(ladder_radii(ClubCategory::All).is_empty());
    // A fairway wood takes the hybrid label (P3 finding 8), so its table is cut at the hybrid's.
    assert_eq!(ladder_radii(ClubCategory::Wood), [15.0, 13.0, 11.0, 9.0]);
}

/// Every club a golfer can carry, but the putter, has a ladder through its category.
#[test]
fn every_club_but_the_putter_has_a_ladder() {
    for club in ClubId::ALL {
        assert_eq!(
            ladder_radii(category_of(club)).is_empty(),
            club == ClubId::Putter,
            "{club}"
        );
    }
}
