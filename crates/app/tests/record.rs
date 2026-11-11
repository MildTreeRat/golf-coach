//! `record_photo` and `log_read`, driven as Dart drives them: strings in, a card out, against a scratch
//! data root. [M38 P6] And `log_event`, the log beside the reads [M38 P30b].
//!
//! **Everything read off the six captures here is provisional: pasted copies** (rule 12). They are
//! chat-pasted JPEGs and not the originals, by M33's decision of 2026-11-04, and the numbers they are
//! held to are M33's own, in `spikes/vision-ocr/results/C0.json`. Those files are read and never
//! written (rule 5), and a capture is always copied into the scratch root first, because a `Failed`
//! read moves the file it was given.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

use app::api::record::{
    log_event, log_read, record_photo, AppEvent, EventKind, PhotoIn, ReadLog, ReadSource,
    RecordOutcome, RecordStatus, RowState, ShotCard,
};
use app::api::setup::{
    create_golfer, get_state, init_state, set_state, AppState, Handedness, PracticeMode,
    TargetShape,
};
use app::{engine_versions, Dirs};
use contracts::career::{CareerCorpus, ExclusionReason};
use contracts::club::ClubId;
use contracts::intent::PracticeMode as StoredMode;
use contracts::intent::TargetShape as StoredShape;
use contracts::shot::ShotData;
use contracts::Timestamp;
use serde_json::Value;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::read_corpus;
use storage::manifest::{Role, SwingManifest};
use storage::shot_store::ShotStore;
use storage::state::load_shot_analysis;
use tempfile::TempDir;

/// M33's `fs-still` set, each a capture, its committed `raw` boxes and its cells.
const CAPTURES: [&str; 6] = [
    "IMG_2889",
    "IMG_2891",
    "IMG_2892",
    "IMG_2893",
    "IMG_2894",
    "2026-10-07-1",
];

const INSTALL: &str = "a1b2";
const TAKEN: &str = "2026-11-04T20:42:00-05:00";
const NOW: &str = "2026-11-04T20:43:00-05:00";
const SESSION: &str = "2026-11-04-phone-a1b2";

fn spike() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spikes/vision-ocr")
}

fn raw_boxes(id: &str) -> String {
    let path = spike().join("boxes/C0/raw").join(format!("{id}.json"));
    fs::read_to_string(&path).unwrap_or_else(|failed| panic!("{}: {failed}", path.display()))
}

/// M33's verdict on one capture through the `raw` arm: `sets.fs-still.arms.raw.shots.<id>`.
fn m33(id: &str) -> Value {
    let path = spike().join("results/C0.json");
    let results: Value = serde_json::from_slice(&fs::read(path).expect("M33's results"))
        .expect("M33's results are JSON");
    results["sets"]["fs-still"]["arms"]["raw"]["shots"][id].clone()
}

/// A scratch data root with a golfer and a club chosen, as first-run setup leaves it.
struct Phone {
    dir: TempDir,
    root: String,
}

impl Phone {
    fn new() -> Self {
        let phone = Self::unset();
        init_state(phone.root.clone(), INSTALL.to_string()).expect("an install id");
        create_golfer(
            phone.root.clone(),
            "Aaron".to_string(),
            Handedness::Right,
            NOW.to_string(),
        )
        .expect("a golfer");
        phone.choose(|state| {
            state.player_id = Some("aaron".to_string());
            state.club = Some("7i".to_string());
        });
        phone
    }

    /// `<scratch>/processed`, with nothing set up in it.
    fn unset() -> Self {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = dir.path().join("processed").display().to_string();
        Phone { dir, root }
    }

    fn choose(&self, change: impl FnOnce(&mut AppState)) {
        let mut state = get_state(self.root.clone());
        change(&mut state);
        set_state(self.root.clone(), state).expect("a state set_state takes");
    }

    fn dirs(&self) -> Dirs {
        Dirs::of(&self.root)
    }

    fn store(&self) -> SwingBundleStore {
        SwingBundleStore::new(self.dirs().sessions)
    }

    /// A copy of the capture as Swift leaves a photo in `tmp/`, under its own name.
    fn photo(&self, id: &str) -> String {
        let tmp = self.dir.path().join("tmp");
        fs::create_dir_all(&tmp).expect("tmp");
        let to = tmp.join(format!("{id}.jpg"));
        fs::copy(spike().join("captures").join(format!("{id}.jpg")), &to).expect("a capture");
        to.display().to_string()
    }

    fn input(&self, path: String, boxes_json: String) -> PhotoIn {
        PhotoIn {
            root: self.root.clone(),
            path,
            boxes_json,
            captured_at: TAKEN.to_string(),
            now: NOW.to_string(),
            challenge_call: None,
        }
    }

    /// One capture and its committed boxes, as the file holds them.
    fn feed(&self, id: &str) -> RecordOutcome {
        record_photo(self.input(self.photo(id), raw_boxes(id))).expect("a recorded photo")
    }

    fn stored(&self, digest: &str) -> ShotData {
        ShotStore::new(self.dirs().shots)
            .get(digest)
            .expect("a readable shot")
            .expect("a stored shot")
    }

    fn corpus(&self) -> CareerCorpus {
        read_corpus(&self.dirs().sessions, "aaron", engine_versions())
    }

    fn entry(&self, card: &ShotCard) -> SwingManifest {
        self.store()
            .get_swing(&card.session_id, &card.swing_id)
            .expect("the card's entry")
    }

    fn failed_dir(&self) -> PathBuf {
        self.dirs().app.join("failed")
    }
}

fn card(outcome: &RecordOutcome) -> &ShotCard {
    outcome.card.as_ref().expect("a card")
}

fn row<'a>(card: &'a ShotCard, field: &str) -> &'a app::api::record::CardRow {
    card.rows
        .iter()
        .find(|row| row.field == field)
        .unwrap_or_else(|| panic!("no row for {field}"))
}

fn files_under(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| entry.expect("an entry").file_name().display().to_string())
        .collect();
    names.sort();
    names
}

// ------------------------------------------------------------------- M33's six captures, end to end

/// The Done line, by value and with no tolerance: every stored field is what M33's run recorded for
/// that capture's boxes. provisional: pasted copies.
#[test]
fn each_capture_stores_what_m33_read_from_its_boxes_by_value() {
    let phone = Phone::new();
    let (mut read, mut withheld) = (0, 0);
    for id in CAPTURES {
        let outcome = phone.feed(id);
        assert_eq!(outcome.status, RecordStatus::Read, "{id}");
        let expected = m33(id);
        assert_eq!(outcome.boxes, expected["boxes"].as_i64().unwrap(), "{id}");

        let shot = phone.stored(&outcome.photo_sha256);
        let written = serde_json::to_value(&shot).unwrap();
        let cells = expected["cells"].as_object().expect("M33's cells");
        assert_eq!(cells.len(), 12, "{id}");
        for (field, cell) in cells {
            match cell["cell"].as_str().expect("a verdict") {
                "read" => {
                    read += 1;
                    assert_eq!(
                        written[field].as_f64(),
                        cell["value"].as_f64(),
                        "{id} {field}"
                    );
                }
                "withheld" => {
                    withheld += 1;
                    assert!(written[field].is_null(), "{id} {field}: {}", written[field]);
                }
                other => panic!("{id} {field}: M33 recorded no {other} cell on the raw arm"),
            }
            let row = row(card(&outcome), field);
            assert_eq!(row.value, written[field].as_f64(), "{id} {field}");
        }

        let provenance = shot.provenance.as_ref().expect("a read's provenance");
        assert_eq!(provenance.device, "full_swing", "{id}");
        assert_eq!(
            provenance.needs_review,
            expected["flagged"].as_bool().unwrap(),
            "{id}"
        );
        assert_eq!(
            provenance.parse_confidence,
            expected["parse_confidence"].as_f64().unwrap(),
            "{id}"
        );
        let warnings: Vec<&str> = expected["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|warning| warning.as_str().unwrap())
            .collect();
        assert_eq!(provenance.warnings, warnings, "{id}");
        let raw_fields: BTreeMap<String, String> =
            serde_json::from_value(expected["raw_fields"].clone()).unwrap();
        assert_eq!(provenance.raw_fields, raw_fields, "{id}");
    }
    // M33's summary, so this cannot pass by reading fewer cells than the run did.
    assert_eq!((read, withheld), (71, 1));
}

/// provisional: pasted copies.
#[test]
fn the_corpus_lists_all_six_with_no_exclusion() {
    let phone = Phone::new();
    for id in CAPTURES {
        phone.feed(id);
    }
    let corpus = phone.corpus();
    assert_eq!(corpus.swings.len(), 6);
    assert!(corpus.excluded.is_empty(), "{:?}", corpus.excluded);
    let mut swing_ids: Vec<&str> = corpus.swings.iter().map(|s| s.swing_id.as_str()).collect();
    swing_ids.sort();
    assert_eq!(swing_ids, ["1", "2", "3", "4", "5", "6"]);
    assert!(corpus.swings.iter().all(|s| s.session_id == SESSION));
}

/// M33's named case, and the reason a card has a `NotRead` at all: a withheld sign stays an unflagged
/// shot at 1.000 (its log's "For M38" item 5), so review does not surface it and only this row does.
/// provisional: pasted copies.
#[test]
fn img_2893s_withheld_sidespin_shows_not_read_on_an_unflagged_card() {
    let phone = Phone::new();
    let outcome = phone.feed("IMG_2893");
    assert_eq!(outcome.status, RecordStatus::Read);
    assert_eq!(outcome.message, None);

    let card = card(&outcome);
    assert!(!card.flagged);
    assert_eq!(card.confidence, 1.0);
    let sidespin = row(card, "sidespin");
    assert_eq!(sidespin.state, RowState::NotRead);
    assert_eq!(sidespin.value, None);
    assert_eq!(sidespin.display, None);
    assert_eq!(
        card.warnings,
        ["Sidespin Rpm: neither D nor F could be read as lit - sign unknown, not stored"]
    );
    assert_eq!(row(card, "backspin").state, RowState::Read);
}

/// `devices.json`'s order, which is the screen's, less `impact_offset_h`, which has no tile.
/// provisional: pasted copies.
#[test]
fn a_card_has_one_row_per_screen_field_in_the_devices_order() {
    let phone = Phone::new();
    let outcome = phone.feed("IMG_2889");
    let fields: Vec<&str> = card(&outcome)
        .rows
        .iter()
        .map(|r| r.field.as_str())
        .collect();
    assert_eq!(
        fields,
        [
            "total_distance",
            "carry_distance",
            "club_head_speed",
            "ball_speed",
            "club_path",
            "club_face_angle",
            "launch_direction",
            "launch_angle",
            "apex_height",
            "smash_factor",
            "backspin",
            "sidespin",
        ]
    );
    let labels: Vec<&str> = card(&outcome)
        .rows
        .iter()
        .map(|r| r.label.as_str())
        .collect();
    assert_eq!(
        labels,
        [
            "Total Yd",
            "Carry Yd",
            "Club Speed Mph",
            "Ball Speed Mph",
            "Path Deg",
            "Face Deg",
            "Ball Direction Deg",
            "Launch Angle",
            "Apex Ft",
            "Smash Factor",
            "Backspin Rpm",
            "Sidespin Rpm",
        ]
    );
}

/// What a person holds beside the photo: the screen's number and letter, where `value` stays
/// `ShotData`'s. The apex is the case that bites, since the label says feet and the store says yards.
/// provisional: pasted copies.
#[test]
fn a_rows_display_is_what_the_screen_printed_and_its_value_is_what_is_stored() {
    let phone = Phone::new();
    for (id, field, value, display) in [
        ("IMG_2889", "carry_distance", 131.0, "131"),
        ("IMG_2889", "ball_speed", 102.5, "102.5"),
        ("IMG_2889", "club_path", -2.0, "-2"),
        ("IMG_2889", "smash_factor", 1.08, "1.08"),
        ("IMG_2889", "apex_height", 28.400000000000002, "85.2"),
        ("IMG_2893", "apex_height", 5.166666666666667, "15.5"),
        ("IMG_2889", "launch_direction", 10.1, "10.1 R"),
        ("IMG_2891", "launch_direction", -0.8, "0.8 L"),
        ("IMG_2893", "launch_direction", 2.2, "2.2 R"),
        ("IMG_2889", "sidespin", -4199.0, "4199 D"),
        ("2026-10-07-1", "sidespin", 336.0, "336 F"),
    ] {
        let phone_row = {
            let outcome = phone.feed(id);
            row(card(&outcome), field).clone()
        };
        assert_eq!(phone_row.value, Some(value), "{id} {field}");
        assert_eq!(phone_row.display.as_deref(), Some(display), "{id} {field}");
        assert_eq!(phone_row.state, RowState::Read, "{id} {field}");
    }
}

/// provisional: pasted copies.
#[test]
fn only_a_tile_signed_by_a_lit_letter_offers_two_letters_negative_first() {
    let phone = Phone::new();
    let outcome = phone.feed("IMG_2889");
    for row in &card(&outcome).rows {
        let expected = match row.field.as_str() {
            "launch_direction" => Some(("L".to_string(), "R".to_string())),
            "sidespin" => Some(("D".to_string(), "F".to_string())),
            _ => None,
        };
        assert_eq!(row.sign_letters, expected, "{}", row.field);
    }
}

// ---------------------------------------------------------------------------------- what is stored

/// C8, C9 and C17 together, off one entry. provisional: pasted copies.
#[test]
fn a_shot_is_stamped_with_the_photos_time_a_root_relative_path_and_the_sessions_id() {
    let phone = Phone::new();
    let outcome = phone.feed("IMG_2889");
    let card = card(&outcome);
    assert_eq!(
        (card.session_id.as_str(), card.swing_id.as_str()),
        (SESSION, "1")
    );

    let shot = phone.stored(&outcome.photo_sha256);
    assert_eq!(shot.shot_id, format!("{SESSION}-1"));
    assert_eq!(shot.session_id, SESSION);
    assert_eq!(shot.timestamp, Timestamp::parse(TAKEN).unwrap());
    let provenance = shot.provenance.as_ref().unwrap();
    assert_eq!(
        provenance.image_sha256.as_deref(),
        Some(outcome.photo_sha256.as_str())
    );

    let manifest = phone.entry(card);
    let stored = &manifest.roles[&Role::ShotScreen];
    assert_eq!(stored.original_filename, "IMG_2889.jpg");
    assert_eq!(stored.content_sha256, outcome.photo_sha256);
    let relative = provenance.image_path.as_deref().expect("a path");
    assert_eq!(
        relative,
        format!("sessions/{SESSION}/1/{}", stored.filename)
    );

    let on_disk = Path::new(&phone.root).join(relative);
    assert_eq!(
        fs::read(&on_disk).expect("the stored photo"),
        fs::read(spike().join("captures/IMG_2889.jpg")).unwrap()
    );
    let micros = fs::metadata(&on_disk)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    let taken = Timestamp::parse(TAKEN).unwrap().utc_micros();
    assert_eq!(micros, Duration::from_micros(taken.unsigned_abs()));
}

/// 23:30 in New York is already tomorrow in UTC, and the session is the photographer's day (C8).
/// provisional: pasted copies.
#[test]
fn a_session_is_named_by_the_photos_date_in_its_own_offset() {
    let phone = Phone::new();
    let mut photo = phone.input(phone.photo("IMG_2889"), raw_boxes("IMG_2889"));
    photo.captured_at = "2026-11-04T23:30:00-05:00".to_string();
    let outcome = record_photo(photo).unwrap();
    assert_eq!(card(&outcome).session_id, SESSION);

    let mut photo = phone.input(phone.photo("IMG_2891"), raw_boxes("IMG_2891"));
    photo.captured_at = "2026-11-05T04:30:00Z".to_string();
    let outcome = record_photo(photo).unwrap();
    assert_eq!(card(&outcome).session_id, "2026-11-05-phone-a1b2");
}

/// C8: what is current stamps each manifest, so changing clubs reaches only later shots, and no
/// `session.json` exists to conflict at the lab. provisional: pasted copies.
#[test]
fn each_shot_takes_the_golfer_club_and_intent_current_when_it_was_recorded() {
    let phone = Phone::new();
    let first = phone.feed("IMG_2889");
    phone.choose(|state| {
        state.club = Some("pw".to_string());
        state.mode = PracticeMode::ShotShaping;
        state.target_shape = Some(TargetShape::Fade);
    });
    let second = phone.feed("IMG_2891");

    let first = phone.entry(card(&first));
    assert_eq!(first.player_id.as_deref(), Some("aaron"));
    assert_eq!(first.club.map(ClubId::as_str), Some("7i"));
    let goal = first.intent.expect("a stamped intent");
    assert_eq!(
        (goal.mode, goal.target_shape),
        (StoredMode::Fundamentals, None)
    );

    let second = phone.entry(card(&second));
    assert_eq!(second.club.map(ClubId::as_str), Some("pw"));
    let goal = second.intent.expect("a stamped intent");
    assert_eq!(goal.mode, StoredMode::ShotShaping);
    assert_eq!(goal.target_shape, Some(StoredShape::Fade));

    assert_eq!(
        files_under(&phone.dirs().sessions.join(SESSION)),
        ["1", "2"]
    );
}

/// C15: `{shot_shaping, call}` is what is stored, and the mode a challenge restores is left alone.
/// provisional: pasted copies.
#[test]
fn a_challenges_call_stamps_the_shot_and_leaves_the_state_as_it_was() {
    let phone = Phone::new();
    let before = get_state(phone.root.clone());
    let mut photo = phone.input(phone.photo("IMG_2889"), raw_boxes("IMG_2889"));
    photo.challenge_call = Some(TargetShape::Draw);
    let outcome = record_photo(photo).unwrap();

    let goal = phone
        .entry(card(&outcome))
        .intent
        .expect("a stamped intent");
    assert_eq!(goal.mode, StoredMode::ShotShaping);
    assert_eq!(goal.target_shape, Some(StoredShape::Draw));
    assert_eq!(get_state(phone.root.clone()), before);
    assert_eq!(phone.corpus().swings.len(), 1);
}

/// Decision 17 §1's other half: a drill is recorded like any shot, and the corpus is what leaves it
/// out. provisional: pasted copies.
#[test]
fn a_drill_is_analysed_and_the_corpus_leaves_it_out_as_untracked() {
    let phone = Phone::new();
    phone.choose(|state| state.mode = PracticeMode::Drill);
    let outcome = phone.feed("IMG_2889");
    assert_eq!(outcome.status, RecordStatus::Read);

    let corpus = phone.corpus();
    assert!(corpus.swings.is_empty());
    assert_eq!(corpus.excluded.len(), 1);
    assert_eq!(corpus.excluded[0].reason, ExclusionReason::Untracked);
}

/// `analyse_entry` is the last step, so its file agrees with the store and names the photo, the two
/// things the corpus asks before it pools an entry. provisional: pasted copies.
#[test]
fn every_read_leaves_a_shot_analysis_naming_its_photo() {
    let phone = Phone::new();
    let outcome = phone.feed("IMG_2889");
    let card = card(&outcome);
    let swing_dir = phone.dirs().sessions.join(SESSION).join(&card.swing_id);
    let result = load_shot_analysis(&swing_dir).expect("shot_analysis.json");
    assert_eq!(
        result.photo_sha256.as_deref(),
        Some(outcome.photo_sha256.as_str())
    );
}

/// C7: the boxes as given, byte for byte, keyed by the photo. The photo's file is Swift's, and only a
/// failed read moves it. provisional: pasted copies.
#[test]
fn a_read_keeps_the_boxes_as_given_and_leaves_the_photo_it_was_handed() {
    let phone = Phone::new();
    let path = phone.photo("IMG_2889");
    let text = raw_boxes("IMG_2889");
    let outcome = record_photo(phone.input(path.clone(), text.clone())).unwrap();
    let kept = phone
        .dirs()
        .app
        .join("boxes")
        .join(format!("{}.json", outcome.photo_sha256));
    assert_eq!(fs::read_to_string(kept).unwrap(), text);
    assert!(Path::new(&path).exists());
    assert!(!phone.failed_dir().exists());
}

/// M33's files wrap the list in an object, and the plan leaves `ScreenReader`'s shape to P17, so both
/// read. provisional: pasted copies.
#[test]
fn a_bare_list_of_boxes_reads_the_same_as_m33s_file_holding_one() {
    let (wrapped, bare) = (Phone::new(), Phone::new());
    let document: Value = serde_json::from_str(&raw_boxes("IMG_2889")).unwrap();
    let list = serde_json::to_string(&document["boxes"]).unwrap();

    let from_file = wrapped.feed("IMG_2889");
    let from_list = record_photo(bare.input(bare.photo("IMG_2889"), list)).unwrap();

    assert_eq!(from_file.boxes, from_list.boxes);
    assert_eq!(from_file.card, from_list.card);
    assert_eq!(
        wrapped.stored(&from_file.photo_sha256),
        bare.stored(&from_list.photo_sha256)
    );
}

// ------------------------------------------------------------------------------------- the flag

/// A physics check failing is what flags a read, and no photo trips one (`validate.rs`'s doc). So one
/// value box is edited: `2.08` is not this shot's ball speed over its club speed. The shot is stored
/// and analysed all the same, because the card must show what was read.
#[test]
fn a_read_failing_a_check_answers_flagged_and_is_stored() {
    let phone = Phone::new();
    let text = raw_boxes("IMG_2889").replace("\"1.08\"", "\"2.08\"");
    assert_ne!(text, raw_boxes("IMG_2889"), "the edit reached a box");
    let outcome = record_photo(phone.input(phone.photo("IMG_2889"), text)).unwrap();

    assert_eq!(outcome.status, RecordStatus::Flagged);
    assert_eq!(outcome.message, None);
    let card = card(&outcome);
    assert!(card.flagged);
    assert!(card.confidence < 1.0, "{}", card.confidence);
    assert_eq!(row(card, "smash_factor").value, Some(2.08));
    assert!(
        card.warnings.iter().any(|warning| warning.contains("2.08")),
        "{:?}",
        card.warnings
    );
    assert!(
        phone
            .stored(&outcome.photo_sha256)
            .provenance
            .unwrap()
            .needs_review
    );
    assert_eq!(phone.corpus().swings.len(), 1);
}

// ------------------------------------------------------------------------------------ duplicates

/// provisional: pasted copies.
#[test]
fn a_second_feed_of_one_photo_answers_duplicate_and_opens_nothing() {
    let phone = Phone::new();
    let first = phone.feed("IMG_2889");
    let shot = phone.stored(&first.photo_sha256);

    let second = phone.feed("IMG_2889");

    assert_eq!(second.status, RecordStatus::Duplicate);
    assert_eq!(second.card, first.card);
    assert!(second.message.is_some());
    assert_eq!(second.photo_sha256, first.photo_sha256);
    assert_eq!(phone.store().get_session(SESSION).len(), 1);
    assert_eq!(phone.stored(&first.photo_sha256), shot);
    assert_eq!(phone.corpus().swings.len(), 1);
}

/// Those boxes are what the held shot came from, which is what a later re-read of it needs.
/// provisional: pasted copies.
#[test]
fn a_duplicate_does_not_replace_the_boxes_the_held_shot_was_read_from() {
    let phone = Phone::new();
    let first = phone.feed("IMG_2889");
    let kept = phone
        .dirs()
        .app
        .join("boxes")
        .join(format!("{}.json", first.photo_sha256));
    let before = fs::read_to_string(&kept).unwrap();

    let second = record_photo(phone.input(phone.photo("IMG_2889"), "[]".to_string())).unwrap();

    assert_eq!(second.status, RecordStatus::Duplicate);
    assert_eq!(second.boxes, 0);
    assert_eq!(fs::read_to_string(&kept).unwrap(), before);
}

/// A photo with no EXIF date takes its file's date or the clock (C9), so a pasted copy imported on
/// another day names another session. The store is keyed by the photo's hash, so without the corpus's
/// question a `put` here would rewrite the held shot's id to name an entry that is about to be dropped.
/// provisional: pasted copies.
#[test]
fn the_same_photo_under_another_date_answers_the_held_shot_and_leaves_it_alone() {
    let phone = Phone::new();
    let first = phone.feed("IMG_2889");
    let shot = phone.stored(&first.photo_sha256);

    let mut again = phone.input(phone.photo("IMG_2889"), raw_boxes("IMG_2889"));
    again.captured_at = "2026-11-09T09:00:00-05:00".to_string();
    again.now = "2026-11-09T09:00:01-05:00".to_string();
    let second = record_photo(again).unwrap();

    assert_eq!(second.status, RecordStatus::Duplicate);
    assert_eq!(second.card, first.card);
    assert_eq!(phone.store().list_session_ids(), [SESSION]);
    assert_eq!(phone.stored(&first.photo_sha256), shot);
    let corpus = phone.corpus();
    assert_eq!(corpus.swings.len(), 1);
    assert!(corpus.excluded.is_empty(), "{:?}", corpus.excluded);
}

// --------------------------------------------------------------------------------- a failed read

/// C13, off every path a read of nothing could leave something on. provisional: pasted copies.
#[test]
fn an_empty_box_list_answers_failed_with_the_photo_in_failed_no_entry_and_no_shot() {
    let phone = Phone::new();
    let path = phone.photo("IMG_2889");
    let outcome = record_photo(phone.input(path.clone(), "[]".to_string())).unwrap();

    assert_eq!(outcome.status, RecordStatus::Failed);
    assert_eq!(outcome.card, None);
    assert!(outcome.message.as_deref().unwrap().contains("Retake"));
    assert_eq!(outcome.boxes, 0);

    let digest = &outcome.photo_sha256;
    let failed = phone.failed_dir().join(format!("{digest}.jpg"));
    assert_eq!(
        fs::read(&failed).expect("the photo in .app/failed/"),
        fs::read(spike().join("captures/IMG_2889.jpg")).unwrap()
    );
    assert!(
        !Path::new(&path).exists(),
        "the photo moved, and was not copied"
    );
    let boxes = phone
        .dirs()
        .app
        .join("boxes")
        .join(format!("{digest}.json"));
    assert_eq!(fs::read_to_string(boxes).unwrap(), "[]");

    assert!(
        phone.store().list_session_ids().is_empty(),
        "no entry, and no empty session"
    );
    assert!(!ShotStore::new(phone.dirs().shots).has(digest));
    let corpus = phone.corpus();
    assert!(corpus.swings.is_empty() && corpus.excluded.is_empty());
}

/// The photo goes as text the parser finds no tile in, which is the same as nothing. This is the
/// reader returning what it saw on a photo of something else. provisional: pasted copies.
#[test]
fn boxes_holding_no_tile_are_a_failed_read_too() {
    let phone = Phone::new();
    let boxes = r#"[{"text": "A LOVELY DAY", "x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0}]"#;
    let outcome = record_photo(phone.input(phone.photo("IMG_2889"), boxes.to_string())).unwrap();
    assert_eq!(outcome.status, RecordStatus::Failed);
    assert_eq!(outcome.boxes, 1);
    assert!(phone.store().list_session_ids().is_empty());
}

/// A failed read leaves no entry, so it is no duplicate, and a retake of the same bytes reads afresh
/// and replaces the boxes that told us nothing. provisional: pasted copies.
#[test]
fn a_retake_after_a_failed_read_records_normally_with_the_next_swing_id() {
    let phone = Phone::new();
    let failed = record_photo(phone.input(phone.photo("IMG_2889"), "[]".to_string())).unwrap();
    assert_eq!(failed.status, RecordStatus::Failed);

    let retake = phone.feed("IMG_2889");
    assert_eq!(retake.status, RecordStatus::Read);
    assert_eq!(card(&retake).swing_id, "1");
    let kept = phone
        .dirs()
        .app
        .join("boxes")
        .join(format!("{}.json", retake.photo_sha256));
    assert_eq!(fs::read_to_string(kept).unwrap(), raw_boxes("IMG_2889"));
}

/// A failed read among others takes back only its own entry, and leaves the session its neighbours are
/// in. provisional: pasted copies.
#[test]
fn a_failed_read_leaves_the_session_its_neighbours_are_in() {
    let phone = Phone::new();
    phone.feed("IMG_2889");
    let outcome = record_photo(phone.input(phone.photo("IMG_2891"), "[]".to_string())).unwrap();
    assert_eq!(outcome.status, RecordStatus::Failed);
    assert_eq!(phone.store().get_session(SESSION).len(), 1);
    assert_eq!(phone.corpus().swings.len(), 1);
}

/// The name a failed photo keeps is `<digest>.<ext>` (C7), whatever the extension is, or none.
#[test]
fn a_failed_photo_keeps_its_own_extension_or_none() {
    let phone = Phone::new();
    let tmp = phone.dir.path().join("tmp");
    fs::create_dir_all(&tmp).unwrap();
    for (name, bytes) in [("shot.HEIC", "one"), ("noext", "two")] {
        let path = tmp.join(name);
        fs::write(&path, bytes).unwrap();
        let outcome =
            record_photo(phone.input(path.display().to_string(), "[]".to_string())).unwrap();
        let expected = match name {
            "shot.HEIC" => format!("{}.HEIC", outcome.photo_sha256),
            _ => outcome.photo_sha256.clone(),
        };
        assert!(phone.failed_dir().join(expected).exists(), "{name}");
    }
}

// ------------------------------------------------------------------------------------ the refusals

/// Everything that leaves no file behind, because nothing could be stamped: one state at a time, and
/// each error's own words. provisional: pasted copies.
#[test]
fn what_cannot_be_stamped_is_refused_and_writes_nothing() {
    type Change = fn(&mut AppState);
    let cases: [(&str, Change); 5] = [
        ("no golfer", |state| state.player_id = None),
        ("not on this phone", |state| {
            state.player_id = Some("nobody".to_string())
        }),
        ("no club", |state| state.club = None),
        ("needs --shape", |state| {
            state.mode = PracticeMode::ShotShaping;
            state.target_shape = None;
        }),
        ("declares no shape", |state| {
            state.mode = PracticeMode::Fundamentals;
            state.target_shape = Some(TargetShape::Draw);
        }),
    ];
    for (words, change) in cases {
        let phone = Phone::new();
        phone.choose(change);
        let path = phone.photo("IMG_2889");
        let refused =
            record_photo(phone.input(path.clone(), raw_boxes("IMG_2889"))).expect_err(words);
        assert!(refused.contains(words), "{words}: {refused}");
        assert!(Path::new(&path).exists(), "{words}");
        assert!(!phone.dirs().sessions.exists(), "{words}");
        assert!(!phone.dirs().shots.exists(), "{words}");
        assert!(!phone.dirs().app.join("boxes").exists(), "{words}");
    }
}

/// `init_state` has not run, or the file cannot be read, which reads as the defaults: a session named
/// with a guess could never be renamed (C8). provisional: pasted copies.
#[test]
fn no_session_is_named_without_the_install_id() {
    let phone = Phone::unset();
    let refused = record_photo(phone.input(phone.photo("IMG_2889"), raw_boxes("IMG_2889")))
        .expect_err("no install id");
    assert!(refused.contains("install id"), "{refused}");
    assert!(!Path::new(&phone.root).exists());
}

/// The trap C9 and P4's finding 7 name: Dart's `toIso8601String()` on a local time has no offset.
/// provisional: pasted copies.
#[test]
fn a_time_with_no_offset_is_refused_on_either_argument() {
    let phone = Phone::new();
    let mut photo = phone.input(phone.photo("IMG_2889"), raw_boxes("IMG_2889"));
    photo.captured_at = "2026-11-04T20:42:00.123456".to_string();
    let refused = record_photo(photo).expect_err("no offset");
    assert!(refused.contains("offset"), "{refused}");

    let mut photo = phone.input(phone.photo("IMG_2889"), raw_boxes("IMG_2889"));
    photo.now = "2026-11-04T20:42:00".to_string();
    let refused = record_photo(photo).expect_err("no offset");
    assert!(refused.contains("offset"), "{refused}");
    assert!(!phone.dirs().sessions.exists());
}

#[test]
fn a_photo_that_cannot_be_read_is_refused_by_its_path() {
    let phone = Phone::new();
    let missing = phone.dir.path().join("gone.jpg").display().to_string();
    let refused = record_photo(phone.input(missing.clone(), "[]".to_string())).expect_err("gone");
    assert!(refused.contains(&missing), "{refused}");
    assert!(!phone.dirs().sessions.exists());
}

/// A reader gone wrong is not a photo the golfer should retake, so this is a refusal and not `Failed`,
/// and the text stays as the one clue to what it said. provisional: pasted copies.
#[test]
fn text_that_is_not_boxes_is_refused_and_kept_with_no_entry_and_the_photo_left_alone() {
    for text in ["not json", "[{\"text\": 3}]", "{\"no_boxes\": []}"] {
        let phone = Phone::new();
        let path = phone.photo("IMG_2889");
        let refused = record_photo(phone.input(path.clone(), text.to_string())).expect_err(text);
        assert!(refused.contains("not a list of boxes"), "{text}: {refused}");
        assert!(Path::new(&path).exists(), "{text}");
        assert!(!phone.dirs().sessions.exists(), "{text}");
        let boxes = files_under(&phone.dirs().app.join("boxes"));
        assert_eq!(boxes.len(), 1, "{text}");
        let kept = phone.dirs().app.join("boxes").join(&boxes[0]);
        assert_eq!(fs::read_to_string(kept).unwrap(), text);
    }
}

// ------------------------------------------------------------------------------------ one at a time

/// P3's finding 4: two overlapping opens can take one swing id, and the second replaces the first's
/// photo. Nothing serializes bridge calls, so `record_photo` does. provisional: pasted copies.
#[test]
fn overlapping_feeds_each_get_an_entry_of_their_own() {
    let phone = Phone::new();
    let inputs: Vec<PhotoIn> = CAPTURES
        .iter()
        .map(|id| phone.input(phone.photo(id), raw_boxes(id)))
        .collect();
    let outcomes: Vec<RecordOutcome> = std::thread::scope(|scope| {
        let running: Vec<_> = inputs
            .iter()
            .map(|photo| scope.spawn(|| record_photo(photo.clone()).expect("a recorded photo")))
            .collect();
        running
            .into_iter()
            .map(|feed| feed.join().unwrap())
            .collect()
    });
    assert!(outcomes
        .iter()
        .all(|outcome| outcome.status == RecordStatus::Read));
    let mut swing_ids: Vec<String> = outcomes.iter().map(|o| card(o).swing_id.clone()).collect();
    swing_ids.sort();
    assert_eq!(swing_ids, ["1", "2", "3", "4", "5", "6"]);
    let corpus = phone.corpus();
    assert_eq!(corpus.swings.len(), 6);
    assert!(corpus.excluded.is_empty(), "{:?}", corpus.excluded);
}

// --------------------------------------------------------------------------------------- log_read

fn a_line(at: &str, status: RecordStatus) -> ReadLog {
    ReadLog {
        at: at.to_string(),
        photo_sha256: "abc".to_string(),
        source: ReadSource::Camera,
        status,
        boxes: 42,
        ms_vision: 1260,
        ms_rust: 12,
        ms_total: 1400,
    }
}

/// The plan's keys in the plan's order, and the spellings `m38-bay-thresholds.md` reads: `flagged`,
/// `failed` and `duplicate` decide its counts.
#[test]
fn log_read_appends_one_line_per_read_with_the_plans_keys_in_order() {
    let phone = Phone::new();
    log_read(phone.root.clone(), a_line(NOW, RecordStatus::Flagged)).unwrap();
    let mut library = a_line(NOW, RecordStatus::Duplicate);
    library.source = ReadSource::Library;
    log_read(phone.root.clone(), library.clone()).unwrap();
    let mut scanner = a_line(NOW, RecordStatus::Failed);
    scanner.source = ReadSource::Scanner;
    log_read(phone.root.clone(), scanner).unwrap();
    log_read(phone.root.clone(), a_line(NOW, RecordStatus::Read)).unwrap();

    let text = fs::read_to_string(phone.dirs().app.join("reads.jsonl")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4);
    assert!(text.ends_with('\n'));
    assert_eq!(
        lines[0],
        format!(
            "{{\"at\":\"{NOW}\",\"photo_sha256\":\"abc\",\"source\":\"camera\",\
             \"status\":\"flagged\",\"boxes\":42,\"ms_vision\":1260,\"ms_rust\":12,\
             \"ms_total\":1400}}"
        )
    );
    assert_eq!(serde_json::from_str::<ReadLog>(lines[1]).unwrap(), library);
    assert!(
        lines[2].contains("\"source\":\"scanner\"") && lines[2].contains("\"status\":\"failed\"")
    );
    assert!(lines[3].contains("\"status\":\"read\""));
}

/// A first run's first call can be this one, with no `.app/` yet.
#[test]
fn log_read_makes_the_directory_it_needs() {
    let phone = Phone::unset();
    log_read(phone.root.clone(), a_line(NOW, RecordStatus::Read)).unwrap();
    assert!(phone.dirs().app.join("reads.jsonl").exists());
}

#[test]
fn log_read_refuses_a_time_with_no_offset_and_appends_nothing() {
    let phone = Phone::new();
    let refused = log_read(
        phone.root.clone(),
        a_line("2026-11-04T20:42:00", RecordStatus::Read),
    )
    .expect_err("no offset");
    assert!(refused.contains("offset"), "{refused}");
    assert!(!phone.dirs().app.join("reads.jsonl").exists());
}

// -------------------------------------------------------------------------------------- log_event

fn an_event(at: &str, kind: EventKind, detail: &str) -> AppEvent {
    AppEvent {
        at: at.to_string(),
        kind,
        detail: detail.to_string(),
    }
}

/// P30b's three keys in its order, one line per event appended after the last, and the reads' log
/// left alone.
#[test]
fn log_event_appends_one_line_per_event_with_the_plans_keys_in_order() {
    let phone = Phone::new();
    log_event(
        phone.root.clone(),
        an_event(NOW, EventKind::ClubChanged, "7i"),
    )
    .unwrap();
    let saved = an_event(
        NOW,
        EventKind::ReviewSaved,
        "2026-11-04-phone-a1b2/3, 2 edits",
    );
    log_event(phone.root.clone(), saved.clone()).unwrap();

    let text = fs::read_to_string(phone.dirs().app.join("events.jsonl")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(text.ends_with('\n'));
    assert_eq!(
        lines[0],
        format!("{{\"at\":\"{NOW}\",\"kind\":\"club_changed\",\"detail\":\"7i\"}}")
    );
    assert_eq!(serde_json::from_str::<AppEvent>(lines[1]).unwrap(), saved);
    assert!(!phone.dirs().app.join("reads.jsonl").exists());
}

/// Every kind P30b names, spelled as it names them, which is what P30c counts by.
#[test]
fn every_event_kind_is_spelled_as_the_plan_names_it() {
    let kinds = [
        (EventKind::AppStart, "app_start"),
        (EventKind::ClubChanged, "club_changed"),
        (EventKind::ModeChanged, "mode_changed"),
        (EventKind::CameraOpened, "camera_opened"),
        (EventKind::CameraCancelled, "camera_cancelled"),
        (EventKind::LibraryPicked, "library_picked"),
        (EventKind::ReviewOpened, "review_opened"),
        (EventKind::ReviewSaved, "review_saved"),
        (EventKind::ShotDeleted, "shot_deleted"),
        (EventKind::ClubSet, "club_set"),
        (EventKind::IntentSet, "intent_set"),
        (EventKind::MishitSet, "mishit_set"),
        (EventKind::ExportShared, "export_shared"),
        (EventKind::Error, "error"),
    ];
    for (kind, spelled) in kinds {
        assert_eq!(
            serde_json::to_value(kind).unwrap(),
            Value::String(spelled.to_string())
        );
        assert_eq!(
            serde_json::from_value::<EventKind>(Value::String(spelled.to_string())).unwrap(),
            kind
        );
    }
}

/// An error's detail is its message and its stack's first lines, so it holds newlines, quotes and
/// whatever text Dart had. It is stored as given and is still one line, which is what makes the file
/// a log a line at a time.
#[test]
fn an_events_detail_is_stored_as_given_and_a_newline_in_it_stays_inside_its_line() {
    let phone = Phone::new();
    let detail = "Bad state: \"no root\"\n#0      AppModel.root (app_model.dart:62)\n#1      ° é ✓";
    log_event(phone.root.clone(), an_event(NOW, EventKind::Error, detail)).unwrap();
    log_event(phone.root.clone(), an_event(NOW, EventKind::AppStart, "")).unwrap();

    let text = fs::read_to_string(phone.dirs().app.join("events.jsonl")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(
        serde_json::from_str::<AppEvent>(lines[0]).unwrap().detail,
        detail
    );
    assert_eq!(
        serde_json::from_str::<AppEvent>(lines[1]).unwrap().detail,
        ""
    );
}

/// The app's first event is written before anything else is, with no `.app/` yet.
#[test]
fn log_event_makes_the_directory_it_needs() {
    let phone = Phone::unset();
    log_event(
        phone.root.clone(),
        an_event(NOW, EventKind::AppStart, "golfer"),
    )
    .unwrap();
    assert!(phone.dirs().app.join("events.jsonl").exists());
}

#[test]
fn log_event_refuses_a_time_with_no_offset_and_appends_nothing() {
    let phone = Phone::new();
    let refused = log_event(
        phone.root.clone(),
        an_event("2026-11-04T20:42:00", EventKind::AppStart, "ready"),
    )
    .expect_err("no offset");
    assert!(refused.contains("offset"), "{refused}");
    assert!(!phone.dirs().app.join("events.jsonl").exists());
}

/// What `record_photo` answers is what `log_read` needs, which Dart cannot work out for itself (it
/// neither parses the boxes nor hashes the photo). provisional: pasted copies.
#[test]
fn an_outcome_carries_what_its_log_line_needs() {
    let phone = Phone::new();
    let outcome = phone.feed("IMG_2889");
    assert_eq!(outcome.boxes, 42);
    assert_eq!(
        outcome.photo_sha256, "6f22661840a5e698d0b1c72a10fd7214b0b37fa0301f4af4129223f87cff951b",
        "photos.json's digest of the capture"
    );
    assert!(outcome.ms_rust >= 0);
}

// ----------------------------------------------------------------------------------- the timing line

/// The Done line's timing, printed for the plan's findings and judged there, because a limit in a test
/// would be a claim about whichever host runs it. Each entry is a real one, recorded through
/// `record_photo` with a distinct photo. Run it in release:
///
/// `cargo test --release -p app --test record -- --ignored --nocapture timing`
///
/// provisional: pasted copies.
#[test]
#[ignore = "500 real feeds, run by hand in release for the plan's timing line"]
fn timing_a_feed_with_500_entries_in_the_store() {
    let phone = Phone::new();
    let capture = fs::read(spike().join("captures/IMG_2889.jpg")).unwrap();
    let boxes = raw_boxes("IMG_2889");
    let tmp = phone.dir.path().join("tmp");
    fs::create_dir_all(&tmp).unwrap();

    let feed = |n: usize| {
        let path = tmp.join(format!("timing-{n}.jpg"));
        let mut bytes = capture.clone();
        bytes.extend_from_slice(format!("timing-{n}").as_bytes());
        fs::write(&path, bytes).unwrap();
        let photo = phone.input(path.display().to_string(), boxes.clone());
        let started = Instant::now();
        let outcome = record_photo(photo).expect("a recorded photo");
        assert_eq!(outcome.status, RecordStatus::Read);
        started.elapsed()
    };

    let mut at = Vec::new();
    for n in 0..500 {
        let took = feed(n);
        if matches!(n, 0 | 99 | 249 | 499) {
            at.push((n, took));
        }
    }
    let took: Vec<Duration> = (500..510).map(feed).collect();
    let duplicate = {
        let photo = phone.input(
            tmp.join("timing-0.jpg").display().to_string(),
            boxes.clone(),
        );
        let started = Instant::now();
        let outcome = record_photo(photo).expect("a duplicate");
        assert_eq!(outcome.status, RecordStatus::Duplicate);
        started.elapsed()
    };
    let corpus = {
        let started = Instant::now();
        let corpus = phone.corpus();
        assert_eq!(corpus.swings.len(), 510);
        started.elapsed()
    };
    for (n, took) in at {
        println!("timing: feed with {n} entries already stored: {took:?}");
    }
    println!("timing: the ten feeds with 500 and more stored: {took:?}");
    println!("timing: a duplicate of the first, with 510 stored: {duplicate:?}");
    println!("timing: read_corpus alone, with 510 stored: {corpus:?}");
}
