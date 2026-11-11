//! `list_sessions` and `shot_detail`, driven as Dart drives them: a golfer's id in, plain rows out,
//! against a scratch data root. [M38 P9]
//!
//! **Everything read off the six captures here is provisional: pasted copies** (rule 12). They are
//! chat-pasted JPEGs and not the originals, by M33's decision of 2026-11-04. Those files are read and
//! never written (rule 5), and a capture is always copied into the scratch root first.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use app::api::history::{list_sessions, shot_detail, MishitVerdict, SessionView, ShotRow};
use app::api::record::{record_photo, CardRow, PhotoIn, RecordStatus, RowState, ShotCard};
use app::api::review::{correct_shot, FieldEdit};
use app::api::setup::{
    create_golfer, get_state, init_state, set_bag_slot, set_state, AppState, Handedness,
    PracticeMode, TargetShape,
};
use app::{engine_versions, Dirs};
use contracts::career::{CareerCorpus, ExclusionReason};
use contracts::mishit::MishitVerdict as StoredVerdict;
use contracts::shot_result::ShotResult;
use contracts::unscored::{spec_for, UnscoredReason};
use contracts::Timestamp;
use serde_json::Value;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::read_corpus;
use storage::shot_store::ShotStore;
use storage::state::load_shot_analysis;
use tempfile::TempDir;

const CAPTURES: [&str; 6] = [
    "IMG_2889",
    "IMG_2891",
    "IMG_2892",
    "IMG_2893",
    "IMG_2894",
    "2026-10-07-1",
];
const TAKEN: &str = "2026-11-04T20:42:00-05:00";
const NOW: &str = "2026-11-04T20:43:00-05:00";
const LATER: &str = "2026-11-04T20:44:00-05:00";
const SESSION: &str = "2026-11-04-phone-a1b2";
/// How many saves a read is raced against. With its lock taken out, a list failed 5 runs of 5 at this
/// count, inside 2 s, and a detail 2 of 5. It cannot fail with the lock in, so more would only cost time.
const SAVES: u32 = 60;

fn spike() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spikes/vision-ocr")
}

fn raw_boxes(id: &str) -> String {
    let path = spike().join("boxes/C0/raw").join(format!("{id}.json"));
    fs::read_to_string(&path).unwrap_or_else(|failed| panic!("{}: {failed}", path.display()))
}

/// A scratch data root with a golfer and a club chosen, as first-run setup leaves it.
struct Phone {
    dir: TempDir,
    root: String,
}

impl Phone {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = dir.path().join("processed").display().to_string();
        init_state(root.clone(), "a1b2".to_string()).expect("an install id");
        create_golfer(
            root.clone(),
            "Aaron".to_string(),
            Handedness::Right,
            NOW.to_string(),
        )
        .expect("a golfer");
        let phone = Phone { dir, root };
        phone.choose(|state| {
            state.player_id = Some("aaron".to_string());
            state.club = Some("7i".to_string());
        });
        phone
    }

    fn choose(&self, change: impl FnOnce(&mut AppState)) {
        let mut state = get_state(self.root.clone());
        change(&mut state);
        set_state(self.root.clone(), state).expect("a state set_state takes");
    }

    /// One capture read from `boxes_json`, taken at `taken`.
    fn feed_text(&self, id: &str, boxes_json: String, taken: &str) -> ShotCard {
        let tmp = self.dir.path().join("tmp");
        fs::create_dir_all(&tmp).expect("tmp");
        let to = tmp.join(format!("{id}.jpg"));
        fs::copy(spike().join("captures").join(format!("{id}.jpg")), &to).expect("a capture");
        let outcome = record_photo(PhotoIn {
            root: self.root.clone(),
            path: to.display().to_string(),
            boxes_json,
            captured_at: taken.to_string(),
            now: NOW.to_string(),
            challenge_call: None,
        })
        .expect("a recorded photo");
        assert_ne!(outcome.status, RecordStatus::Failed, "{id}");
        outcome.card.expect("a card")
    }

    fn feed_at(&self, id: &str, taken: &str) -> ShotCard {
        self.feed_text(id, raw_boxes(id), taken)
    }

    fn feed_boxes(&self, id: &str, boxes_json: String) -> ShotCard {
        self.feed_text(id, boxes_json, TAKEN)
    }

    /// `IMG_2889` with one value box edited so a physics check flags it, as `tests/record.rs` does,
    /// since no capture trips one.
    fn flagged(&self) -> ShotCard {
        let text = raw_boxes("IMG_2889").replace("\"1.08\"", "\"2.08\"");
        let card = self.feed_boxes("IMG_2889", text);
        assert!(card.flagged);
        card
    }

    fn feed(&self, id: &str) -> ShotCard {
        self.feed_at(id, TAKEN)
    }

    fn sessions(&self) -> Vec<SessionView> {
        list_sessions(self.root.clone(), "aaron".to_string()).expect("a list")
    }

    /// The row of one card, from the whole list.
    fn row(&self, card: &ShotCard) -> ShotRow {
        self.sessions()
            .into_iter()
            .flat_map(|session| session.rows)
            .find(|row| row.session_id == card.session_id && row.swing_id == card.swing_id)
            .expect("the card's row")
    }

    fn detail(&self, card: &ShotCard) -> app::api::history::ShotDetail {
        shot_detail(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
        )
        .expect("a detail")
    }

    fn correct(&self, card: &ShotCard, edits: Vec<FieldEdit>) -> ShotCard {
        correct_shot(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
            edits,
            LATER.to_string(),
        )
        .expect("a saved review")
    }

    fn swing_dir(&self, card: &ShotCard) -> PathBuf {
        Dirs::of(&self.root)
            .sessions
            .join(&card.session_id)
            .join(&card.swing_id)
    }

    fn analysis(&self, card: &ShotCard) -> ShotResult {
        load_shot_analysis(&self.swing_dir(card)).expect("an analysis")
    }

    fn corpus(&self) -> CareerCorpus {
        read_corpus(&Dirs::of(&self.root).sessions, "aaron", engine_versions())
    }

    fn set_verdict(&self, card: &ShotCard, verdict: Option<StoredVerdict>) {
        SwingBundleStore::new(Dirs::of(&self.root).sessions)
            .set_mishit(
                &card.session_id,
                &card.swing_id,
                verdict,
                Timestamp::parse(LATER).expect("a time"),
            )
            .expect("a written verdict")
            .expect("the card's entry");
    }

    /// Rewrite an entry's analysis file by hand, as an older build or another reader would leave it.
    fn rewrite_analysis(&self, card: &ShotCard, change: impl FnOnce(&mut Value)) {
        let path = self.swing_dir(card).join("shot_analysis.json");
        let mut held: Value = serde_json::from_slice(&fs::read(&path).expect("an analysis"))
            .expect("an analysis is JSON");
        change(&mut held);
        fs::write(&path, serde_json::to_vec(&held).expect("JSON")).expect("a written analysis");
    }
}

fn typed(field: &str, printed: &str) -> FieldEdit {
    FieldEdit {
        field: field.to_string(),
        printed: Some(printed.to_string()),
        letter: None,
    }
}

fn cleared(field: &str) -> FieldEdit {
    FieldEdit {
        field: field.to_string(),
        printed: None,
        letter: None,
    }
}

fn card_row<'a>(card: &'a ShotCard, field: &str) -> &'a CardRow {
    card.rows
        .iter()
        .find(|row| row.field == field)
        .unwrap_or_else(|| panic!("no row for {field}"))
}

// ------------------------------------------------------------------------------------- the order

/// Six captures on three days, recorded out of date order, as a library import can: the newest day
/// first, each with its own count, and every shot once. provisional: pasted copies.
#[test]
fn sessions_answer_newest_first_with_their_dates_and_counts() {
    let phone = Phone::new();
    for (id, taken) in [
        ("IMG_2889", "2026-11-04T10:00:00-05:00"),
        ("IMG_2891", "2026-11-03T10:00:00-05:00"),
        ("IMG_2892", "2026-11-05T10:00:00-05:00"),
        ("IMG_2893", "2026-11-03T11:00:00-05:00"),
        ("IMG_2894", "2026-11-04T11:00:00-05:00"),
        ("2026-10-07-1", "2026-11-03T12:00:00-05:00"),
    ] {
        phone.feed_at(id, taken);
    }

    let sessions = phone.sessions();
    let shape: Vec<(&str, &str, i64, usize)> = sessions
        .iter()
        .map(|session| {
            (
                session.session_id.as_str(),
                session.date.as_str(),
                session.count,
                session.rows.len(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            ("2026-11-05-phone-a1b2", "2026-11-05", 1, 1),
            ("2026-11-04-phone-a1b2", "2026-11-04", 2, 2),
            ("2026-11-03-phone-a1b2", "2026-11-03", 3, 3),
        ]
    );
}

/// By when each photo was taken and not when it arrived, since C9's time is the photo's: the last one
/// recorded here was taken first, so it reads last. provisional: pasted copies.
#[test]
fn rows_answer_newest_first_by_when_the_photo_was_taken() {
    let phone = Phone::new();
    for (id, taken) in [
        ("IMG_2889", "2026-11-04T10:00:00-05:00"),
        ("IMG_2891", "2026-11-04T10:05:00-05:00"),
        ("IMG_2892", "2026-11-04T10:10:00-05:00"),
        ("IMG_2893", "2026-11-04T10:15:00-05:00"),
        ("IMG_2894", "2026-11-04T10:20:00-05:00"),
        ("2026-10-07-1", "2026-11-04T09:00:00-05:00"),
    ] {
        phone.feed_at(id, taken);
    }
    let sessions = phone.sessions();
    assert_eq!(sessions.len(), 1);
    let order: Vec<(&str, &str)> = sessions[0]
        .rows
        .iter()
        .map(|row| (row.swing_id.as_str(), row.time_display.as_str()))
        .collect();
    assert_eq!(
        order,
        [
            ("5", "10:20"),
            ("4", "10:15"),
            ("3", "10:10"),
            ("2", "10:05"),
            ("1", "10:00"),
            ("6", "09:00"),
        ]
    );
}

/// A batch can hold photos stamped alike, by C9's fallbacks. The later entry still reads first, and
/// the order stays numeric past nine, where a sort on the id as text would not.
/// provisional: pasted copies.
#[test]
fn photos_taken_at_one_instant_read_newest_entry_first() {
    let phone = Phone::new();
    for id in CAPTURES {
        phone.feed(id);
    }
    let order: Vec<String> = phone.sessions()[0]
        .rows
        .iter()
        .map(|row| row.swing_id.clone())
        .collect();
    assert_eq!(order, ["6", "5", "4", "3", "2", "1"]);
}

// ---------------------------------------------------------------------------------------- a row

/// Everything a row says of an ordinary read, and that its numbers are the card's on all six: a person
/// looking from one to the other must not see two spellings. provisional: pasted copies.
#[test]
fn every_row_shows_what_its_card_shows() {
    let phone = Phone::new();
    for id in CAPTURES {
        let card = phone.feed(id);
        let row = phone.row(&card);
        assert_eq!(
            row.carry_display.as_ref(),
            card_row(&card, "carry_distance").display.as_ref(),
            "{id}"
        );
        assert_eq!(
            row.ball_speed_display.as_ref(),
            card_row(&card, "ball_speed").display.as_ref(),
            "{id}"
        );
        assert_eq!(row.carry_label, card_row(&card, "carry_distance").label);
        assert_eq!(row.ball_speed_label, card_row(&card, "ball_speed").label);
        assert_eq!(row.club.as_deref(), Some("7i"), "{id}");
        assert_eq!(row.mode, PracticeMode::Fundamentals, "{id}");
        assert_eq!(row.target_shape, None, "{id}");
        assert!(!row.flagged && !row.corrected && !row.mishit && !row.untracked);
        assert_eq!(row.mishit_verdict, None, "{id}");
        assert_eq!(row.left_out, Vec::<String>::new(), "{id}");
    }
    let row = phone.row(&phone.feed("IMG_2893"));
    assert_eq!(row.carry_label, "Carry Yd");
    assert_eq!(row.carry_display.as_deref(), Some("184"));
    assert_eq!(row.ball_speed_label, "Ball Speed Mph");
    assert_eq!(row.ball_speed_display.as_deref(), Some("134"));
}

/// C2's time crosses with its offset, and the clock beside it is the one the photo was taken under,
/// so neither needs a Dart `DateTime`. provisional: pasted copies.
#[test]
fn a_rows_time_keeps_the_photos_offset() {
    let phone = Phone::new();
    let row = phone.row(&phone.feed("IMG_2889"));
    assert_eq!(row.time, TAKEN);
    assert_eq!(row.time_display, "20:42");
}

/// C17: a stored path is relative to the root because a container's changes on reinstall, so the
/// absolute one is built here, through the manifest, and it opens the bytes that were read.
/// provisional: pasted copies.
#[test]
fn a_rows_photo_path_is_absolute_and_opens_the_photo() {
    let phone = Phone::new();
    let row = phone.row(&phone.feed("IMG_2889"));
    let path = PathBuf::from(row.photo_path.expect("a photo"));
    assert!(path.is_absolute(), "{}", path.display());
    assert_eq!(
        fs::read(&path).expect("the stored photo"),
        fs::read(spike().join("captures/IMG_2889.jpg")).expect("the capture")
    );
}

/// C8: what stamped a shot is the manifest's, so changing clubs afterwards moves no earlier row.
/// provisional: pasted copies.
#[test]
fn a_row_says_what_its_own_entry_was_stamped_with() {
    let phone = Phone::new();
    let first = phone.feed("IMG_2889");
    phone.choose(|state| {
        state.club = Some("driver".to_string());
        state.mode = PracticeMode::ShotShaping;
        state.target_shape = Some(TargetShape::Draw);
    });
    let second = phone.row(&phone.feed("IMG_2891"));
    assert_eq!(second.club.as_deref(), Some("driver"));
    assert_eq!(second.mode, PracticeMode::ShotShaping);
    assert_eq!(second.target_shape, Some(TargetShape::Draw));

    let first = phone.row(&first);
    assert_eq!(first.club.as_deref(), Some("7i"));
    assert_eq!(first.mode, PracticeMode::Fundamentals);
    assert_eq!(first.target_shape, None);
}

/// D9's mark and C11's. `flagged` lifts once a person has looked, `corrected` once they changed one.
/// provisional: pasted copies.
#[test]
fn a_row_marks_a_flagged_shot_and_a_corrected_one() {
    let phone = Phone::new();
    let card = phone.flagged();
    let row = phone.row(&card);
    assert!(row.flagged && !row.corrected);

    let card = phone.correct(&card, Vec::new());
    let row = phone.row(&card);
    assert!(!row.flagged && !row.corrected, "a person has now looked");

    phone.correct(&card, vec![typed("smash_factor", "1.08")]);
    let row = phone.row(&card);
    assert!(!row.flagged && row.corrected);
}

// ------------------------------------------------------------------------------------- the drill

/// Decision 17 §1's other half, on the surface it matters: a drill counts in nothing and is still a shot a
/// person hit and can find. The corpus's own sentence names a verb no phone has, which is why this one
/// is not that one. provisional: pasted copies.
#[test]
fn a_drill_is_listed_marked_untracked_and_says_why_in_words_true_on_a_phone() {
    let phone = Phone::new();
    let counted = phone.feed("IMG_2889");
    phone.choose(|state| state.mode = PracticeMode::Drill);
    let drill = phone.feed("IMG_2891");

    let sessions = phone.sessions();
    assert_eq!(sessions[0].count, 2, "a drill is in the count");

    let corpus = phone.corpus();
    assert_eq!(corpus.swings.len(), 1, "and in none of the numbers");
    assert_eq!(corpus.excluded[0].reason, ExclusionReason::Untracked);
    assert!(corpus.excluded[0].detail.contains("golf-core set-intent"));

    let row = phone.row(&drill);
    assert!(row.untracked);
    assert_eq!(row.mode, PracticeMode::Drill);
    assert_eq!(row.left_out.len(), 1);
    assert!(row.left_out[0].starts_with("Hit as a drill"), "{row:?}");
    assert!(!row.left_out[0].contains("golf-core"), "{row:?}");
    assert_eq!(phone.detail(&drill).row, row);

    let row = phone.row(&counted);
    assert!(!row.untracked);
    assert_eq!(row.left_out, Vec::<String>::new());
}

// ------------------------------------------------------------------------------------ the mishit

/// ADR-028 reaches a row through the corpus. Six carries of one club, one corrected to a top: the rule
/// holds it out with no verdict on it, a verdict overrules the rule either way, and clearing the
/// verdict gives it back. provisional: pasted copies.
#[test]
fn a_rows_mishit_is_the_corpus_and_its_verdict_is_the_persons() {
    let phone = Phone::new();
    let cards: Vec<ShotCard> = CAPTURES.iter().map(|id| phone.feed(id)).collect();
    let topped = phone.correct(&cards[2], vec![typed("carry_distance", "20")]);

    let row = phone.row(&topped);
    assert!(row.mishit, "the automatic rule");
    assert_eq!(row.mishit_verdict, None);
    assert_eq!(row.left_out, Vec::<String>::new(), "held out of one number");

    phone.set_verdict(&topped, Some(StoredVerdict::Cleared));
    let row = phone.row(&topped);
    assert!(!row.mishit, "a person's verdict wins");
    assert_eq!(row.mishit_verdict, Some(MishitVerdict::Cleared));

    phone.set_verdict(&cards[0], Some(StoredVerdict::Confirmed));
    let row = phone.row(&cards[0]);
    assert!(row.mishit, "a shot the rule cannot see");
    assert_eq!(row.mishit_verdict, Some(MishitVerdict::Confirmed));

    phone.set_verdict(&topped, None);
    let row = phone.row(&topped);
    assert!(row.mishit, "back to the automatic rule");
    assert_eq!(row.mishit_verdict, None);
}

/// A drill has no swing in the corpus, so nothing automatic can be said of it, and a person's own word
/// is all there is. provisional: pasted copies.
#[test]
fn a_verdict_on_a_shot_the_corpus_holds_no_swing_for_still_shows() {
    let phone = Phone::new();
    phone.choose(|state| state.mode = PracticeMode::Drill);
    let drill = phone.feed("IMG_2889");
    assert!(!phone.row(&drill).mishit);
    phone.set_verdict(&drill, Some(StoredVerdict::Confirmed));
    let row = phone.row(&drill);
    assert!(row.mishit);
    assert_eq!(row.mishit_verdict, Some(MishitVerdict::Confirmed));
}

// ------------------------------------------------------------------------------- what is left out

/// P6 keeps an entry whose analysis failed, because the read was good. So it lists, with its numbers
/// and its reason, and the repair its sentence offers is real. provisional: pasted copies.
#[test]
fn an_entry_whose_analysis_file_is_missing_still_lists_with_its_reason() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2893");
    let whole = phone.detail(&card);
    fs::remove_file(phone.swing_dir(&card).join("shot_analysis.json")).unwrap();

    let sessions = phone.sessions();
    assert_eq!(sessions[0].count, 1);
    let row = phone.row(&card);
    assert_eq!(row.carry_display.as_deref(), Some("184"));
    assert_eq!(row.left_out.len(), 1);
    assert!(row.left_out[0].contains("No analysis is stored"), "{row:?}");
    assert!(!row.left_out[0].contains("golf-core"), "{row:?}");

    let detail = phone.detail(&card);
    assert_eq!(detail.row, row);
    assert_eq!(detail.card.as_ref(), Some(&card), "the shot is the store's");
    assert_eq!(detail.no_card, None);
    assert!(detail.measurements.is_empty());
    assert!(detail.unscored.is_empty());
    assert!(detail.notes.is_empty());

    phone.correct(&card, Vec::new());
    let detail = phone.detail(&card);
    assert_eq!(detail, whole, "a saved review analyses it again");
}

/// The corpus decides a drill before it looks for a file, so it never says this one is missing, and the
/// empty lists on the screen would be unexplained. provisional: pasted copies.
#[test]
fn a_drill_with_no_analysis_says_both_things_on_its_detail_screen() {
    let phone = Phone::new();
    phone.choose(|state| state.mode = PracticeMode::Drill);
    let card = phone.feed("IMG_2889");
    fs::remove_file(phone.swing_dir(&card).join("shot_analysis.json")).unwrap();

    assert_eq!(phone.row(&card).left_out.len(), 1, "the corpus's one");
    let said = phone.detail(&card).row.left_out;
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[0].starts_with("Hit as a drill"), "{said:?}");
    assert!(said[1].contains("No analysis is stored"), "{said:?}");
}

/// An entry can be left out for two reasons at once, so `left_out` is a list. And both of these are
/// repaired by the same save their sentences offer. provisional: pasted copies.
#[test]
fn a_stale_entry_from_an_older_engine_says_both_and_a_save_repairs_them() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    phone.rewrite_analysis(&card, |held| {
        held["photo_sha256"] = Value::Null;
        held["analysis_version"] = serde_json::json!(0);
    });

    let said = phone.row(&card).left_out;
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[0].contains("names another photo"), "{said:?}");
    assert!(said[1].contains("older build"), "{said:?}");
    for sentence in &said {
        assert!(!sentence.contains("golf-core"), "{sentence}");
    }

    phone.correct(&card, Vec::new());
    assert_eq!(phone.row(&card).left_out, Vec::<String>::new());
}

/// The same photo under another session's id, which `record_photo` refuses and a restored backup could
/// still hold. The corpus's sentence is the one that says which entry was kept, and it names no verb.
/// provisional: pasted copies.
#[test]
fn a_duplicate_says_which_entry_was_kept() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let sessions = Dirs::of(&phone.root).sessions;
    let copy = sessions.join("2026-11-05-phone-a1b2").join("1");
    fs::create_dir_all(&copy).unwrap();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(phone.swing_dir(&card).join("manifest.json")).unwrap())
            .unwrap();
    manifest["session_id"] = Value::String("2026-11-05-phone-a1b2".to_string());
    manifest["swing_id"] = Value::String("1".to_string());
    manifest["created_at"] = Value::String("2026-11-05T09:00:00-05:00".to_string());
    fs::write(
        copy.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();

    let list = phone.sessions();
    assert_eq!(list[0].session_id, "2026-11-05-phone-a1b2");
    let said = &list[0].rows[0].left_out;
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].starts_with("Not counted here: "), "{said:?}");
    assert!(said[0].contains(&format!("{SESSION}/1")), "{said:?}");
    assert!(!said[0].contains("golf-core"), "{said:?}");
    assert_eq!(list[1].rows[0].left_out, Vec::<String>::new());
}

/// P6 finding 6: a shot the store does not hold has no card, and the screen needs something of its own
/// to say. The row still lists, with nothing read rather than with a 0. provisional: pasted copies.
#[test]
fn an_entry_whose_shot_is_gone_still_lists_and_its_detail_says_so() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let sha = phone
        .analysis(&card)
        .photo_sha256
        .expect("the analysis names its photo");
    let shots = Dirs::of(&phone.root).shots;
    assert!(ShotStore::new(&shots).has(&sha));
    for entry in fs::read_dir(&shots).unwrap() {
        fs::remove_file(entry.unwrap().path()).unwrap();
    }

    let row = phone.row(&card);
    assert_eq!(row.carry_display, None);
    assert_eq!(row.ball_speed_display, None);
    assert!(!row.flagged && !row.corrected);
    assert_eq!(row.time, NOW, "the entry's arrival, with no shot to ask");

    let detail = phone.detail(&card);
    assert_eq!(detail.card, None);
    let said = detail.no_card.expect("a sentence");
    assert!(said.contains("No shot is stored"), "{said}");
}

/// A file that is not a shot is not the same as no file, and `shot_store` says what is wrong with it.
/// The row still lists. provisional: pasted copies.
#[test]
fn an_unreadable_shot_lists_and_its_detail_says_what_is_wrong() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let shots = Dirs::of(&phone.root).shots;
    for entry in fs::read_dir(&shots).unwrap() {
        fs::write(entry.unwrap().path(), "not a shot").unwrap();
    }

    assert_eq!(phone.row(&card).carry_display, None);
    let detail = phone.detail(&card);
    assert_eq!(detail.card, None);
    assert!(detail.no_card.is_some());
}

// --------------------------------------------------------------------------------- the detail

/// P7's finding: the card a detail screen shows is the one a read answered, the same path whichever
/// surface asks. provisional: pasted copies.
#[test]
fn a_detail_holds_the_card_a_read_answered_and_the_row_the_list_shows() {
    let phone = Phone::new();
    for id in CAPTURES {
        let card = phone.feed(id);
        let detail = phone.detail(&card);
        assert_eq!(detail.card.as_ref(), Some(&card), "{id}");
        assert_eq!(detail.no_card, None, "{id}");
        assert_eq!(detail.row, phone.row(&card), "{id}");
    }
}

/// In the stored order, the shot's rows then the flight's, each with its number as text and the
/// sentence that stops it being misread. provisional: pasted copies.
#[test]
fn measurements_answer_in_stored_order_with_their_text_and_unit() {
    let phone = Phone::new();
    for id in CAPTURES {
        let card = phone.feed(id);
        let stored = phone.analysis(&card).measurements;
        assert!(!stored.is_empty(), "{id}");
        let shown = phone.detail(&card).measurements;
        assert_eq!(shown.len(), stored.len(), "{id}");
        for (shown, stored) in shown.iter().zip(&stored) {
            assert_eq!(shown.name, stored.name, "{id}");
            assert_eq!(shown.unit, stored.unit, "{id}");
            assert_eq!(shown.detail, stored.detail, "{id}");
            assert_eq!(shown.display, pyfmt::g(stored.value), "{id}");
        }
    }
    let shown = phone.detail(&phone.feed("IMG_2893")).measurements;
    let carry = shown
        .iter()
        .find(|row| row.name == "carry_distance_yds")
        .expect("a carry row");
    assert_eq!(
        (carry.display.as_str(), carry.unit.as_str()),
        ("184", "yards")
    );
}

/// A tile that printed nothing and one that printed what no number can be read from, each named by the
/// tile's own label, each with the contract's sentence for its reason. Those two are the phone's own
/// refusals, and no capture trips them, so the boxes are edited. provisional: pasted copies.
#[test]
fn an_unscored_tile_carries_the_contracts_sentence_under_its_own_label() {
    let phone = Phone::new();
    let card = phone.feed_boxes(
        "IMG_2889",
        raw_boxes("IMG_2889")
            .replace("\"131\"", "\"---\"")
            .replace("\"102.5\"", "\"XX\""),
    );
    let unscored = phone.detail(&card).unscored;
    let of = |name: &str| {
        unscored
            .iter()
            .find(|row| row.name == name)
            .unwrap_or_else(|| panic!("no {name} in {unscored:?}"))
    };

    assert_eq!(of("carry_distance").label, "Carry Yd");
    assert_eq!(
        of("carry_distance").reason,
        spec_for(UnscoredReason::PrintedBlank).remedy
    );
    assert_eq!(of("ball_speed").label, "Ball Speed Mph");
    assert_eq!(
        of("ball_speed").reason,
        spec_for(UnscoredReason::Misread).remedy
    );
}

/// What all six really hold is the flight's refusal, in two of the contract's reasons. Each shown row is
/// the stored one's, by its reason and in its place, and a flight row has no tile, so it keeps the
/// contract's name. provisional: pasted copies.
#[test]
fn every_unscored_row_of_the_six_captures_is_the_contracts_sentence() {
    let phone = Phone::new();
    let mut reasons = Vec::new();
    for id in CAPTURES {
        let card = phone.feed(id);
        let stored = phone.analysis(&card).unscored;
        let shown = phone.detail(&card).unscored;
        assert!(!stored.is_empty(), "{id}");
        assert_eq!(shown.len(), stored.len(), "{id}");
        for (shown, stored) in shown.iter().zip(&stored) {
            assert_eq!(shown.name, stored.name, "{id}");
            assert_eq!(shown.label, stored.name, "{id}: no tile, no label");
            assert_eq!(shown.reason, stored.spec().remedy, "{id}");
            assert!(!shown.reason.contains('\n'), "{}", shown.reason);
            reasons.push(stored.reason);
        }
    }
    assert!(reasons.contains(&UnscoredReason::CarryUnreachable));
    assert!(reasons.contains(&UnscoredReason::NoClubLoft));
}

/// M33's named case, and what it does not do: `IMG_2893`'s sidespin is withheld, and sidespin is
/// `shown_only` on this device (`devices.json`), so `tile_refusals` asks nothing of it. The card's
/// `NotRead` row is all that says so, which is right: no number was going to be measured from it.
/// provisional: pasted copies.
#[test]
fn a_withheld_tile_nothing_analyses_is_on_the_card_and_in_no_unscored_list() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2893");
    assert_eq!(card_row(&card, "sidespin").state, RowState::NotRead);
    assert!(!card.flagged);
    let detail = phone.detail(&card);
    assert!(
        !detail.unscored.iter().any(|row| row.name == "sidespin"),
        "{:?}",
        detail.unscored
    );
}

/// ADR-010 §2, at each place a number could appear: a value that was withheld, and one a person cleared,
/// read as not there, on the row, on the card and in the measurements, and nothing stands in for them.
/// provisional: pasted copies.
#[test]
fn a_number_with_no_value_never_reads_as_0() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2893");

    let sidespin = card_row(&card, "sidespin");
    assert_eq!(sidespin.state, RowState::NotRead);
    assert_eq!((sidespin.value, sidespin.display.as_ref()), (None, None));

    let detail = phone.detail(&card);
    let measured: Vec<&str> = detail
        .measurements
        .iter()
        .map(|row| row.name.as_str())
        .collect();
    assert!(
        !measured.iter().any(|name| name.contains("sidespin")),
        "{measured:?}"
    );

    let card = phone.correct(&card, vec![cleared("carry_distance")]);
    let row = phone.row(&card);
    assert_eq!(row.carry_display, None, "and not Some(\"0\")");
    assert_eq!(
        row.carry_label, "Carry Yd",
        "the label stays to say what is missing"
    );

    let detail = phone.detail(&card);
    let carry = card_row(detail.card.as_ref().expect("a card"), "carry_distance");
    assert_eq!((carry.value, carry.display.as_ref()), (None, None));
    assert!(
        !detail
            .measurements
            .iter()
            .any(|row| row.name == "carry_distance_yds"),
        "{:?}",
        detail.measurements
    );
}

/// A real zero is a number and a missing one is not, so the two must not read alike in either direction.
/// provisional: pasted copies.
#[test]
fn a_number_a_person_typed_as_0_reads_as_0() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let card = phone.correct(&card, vec![typed("carry_distance", "0")]);
    assert_eq!(phone.row(&card).carry_display.as_deref(), Some("0"));
}

/// P7 finding 6, handled where the words are: clearing a tile writes `misread` into the analysis, with
/// a sentence blaming the photo for what a person chose. The row's `Corrected` state is the truth. A
/// tile nobody touched still says why, and so does a flight row the clear caused.
/// provisional: pasted copies.
#[test]
fn a_cleared_tile_shows_as_corrected_and_not_as_a_misread_photo() {
    let phone = Phone::new();
    let card = phone.feed_boxes(
        "IMG_2889",
        raw_boxes("IMG_2889").replace("\"102.5\"", "\"---\""),
    );
    let card = phone.correct(&card, vec![cleared("carry_distance")]);

    assert!(
        phone
            .analysis(&card)
            .unscored
            .iter()
            .any(|entry| entry.name == "carry_distance" && entry.reason == UnscoredReason::Misread),
        "the stored file still says it"
    );

    let detail = phone.detail(&card);
    let names: Vec<&str> = detail
        .unscored
        .iter()
        .map(|row| row.name.as_str())
        .collect();
    assert!(!names.contains(&"carry_distance"), "{names:?}");
    assert!(names.contains(&"ball_speed"), "{names:?}");
    assert!(names.contains(&"flight_carry_yds"), "{names:?}");
    let shown = detail.card.expect("a card");
    assert_eq!(
        card_row(&shown, "carry_distance").state,
        RowState::Corrected
    );
}

/// As stored, because they are `analyse_entry`'s free text with nothing to translate them by. A flagged
/// read has one, ADR-014's, and a review takes it away. provisional: pasted copies.
#[test]
fn notes_answer_as_stored() {
    let phone = Phone::new();
    let card = phone.flagged();
    let notes = phone.detail(&card).notes;
    assert_eq!(notes, phone.analysis(&card).notes);
    assert!(
        notes
            .iter()
            .any(|note| note.starts_with("shot data needs review")),
        "{notes:?}"
    );

    phone.correct(&card, Vec::new());
    assert!(!phone
        .detail(&card)
        .notes
        .iter()
        .any(|note| note.starts_with("shot data needs review")));
}

/// Two of M33's six carry a loft note under a club with no loft, so a person would read a verb their
/// phone does not have. The phone's words go in that clause alone, the file keeps the lab's, and what
/// they offer works. provisional: pasted copies.
#[test]
fn a_loft_note_says_what_a_phone_can_do_and_doing_it_clears_the_note() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2891");
    let stored = phone.analysis(&card).notes;
    assert_eq!(stored.len(), 1, "{stored:?}");
    assert!(
        stored[0].ends_with(golf_core::session::RERUN_SHOT),
        "{stored:?}"
    );

    let shown = phone.detail(&card).notes;
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert!(
        shown[0].starts_with("club 7i is not in this golfer's bag"),
        "{shown:?}"
    );
    assert!(
        shown[0].ends_with("then save a review of this shot"),
        "{shown:?}"
    );
    assert!(!shown[0].contains("golf-core"), "{shown:?}");

    set_bag_slot(
        phone.root.clone(),
        "aaron".to_string(),
        "7i".to_string(),
        Some(34.0),
        NOW.to_string(),
    )
    .expect("a slot with a loft");
    phone.correct(&card, Vec::new());

    let detail = phone.detail(&card);
    assert_eq!(detail.notes, Vec::<String>::new());
    assert!(
        !detail
            .unscored
            .iter()
            .any(|row| row.reason == spec_for(UnscoredReason::NoClubLoft).remedy),
        "{:?}",
        detail.unscored
    );
}

// ------------------------------------------------------------------------------------ the edges

/// "This golfer has no swings yet" is a real answer, and the first one every golfer has.
#[test]
fn nothing_recorded_lists_nothing_and_refuses_nothing() {
    let phone = Phone::new();
    assert_eq!(phone.sessions(), Vec::<SessionView>::new());
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("nothing-here").display().to_string();
    assert_eq!(
        list_sessions(root, "aaron".to_string()),
        Ok(Vec::<SessionView>::new())
    );
}

/// `list_sessions` is one golfer's. A session holding only another's does not show as an empty date,
/// and a shared one counts only theirs. provisional: pasted copies.
#[test]
fn a_list_is_one_golfers_and_a_session_with_none_of_theirs_is_not_shown() {
    let phone = Phone::new();
    phone.feed_at("IMG_2889", "2026-11-03T10:00:00-05:00");
    phone.feed_at("IMG_2891", "2026-11-04T10:00:00-05:00");
    create_golfer(
        phone.root.clone(),
        "Pat".to_string(),
        Handedness::Left,
        NOW.to_string(),
    )
    .unwrap();
    phone.choose(|state| state.player_id = Some("pat".to_string()));
    phone.feed_at("IMG_2892", "2026-11-04T11:00:00-05:00");
    phone.feed_at("IMG_2893", "2026-11-05T11:00:00-05:00");

    let mine: Vec<(String, i64)> = phone
        .sessions()
        .into_iter()
        .map(|session| (session.date, session.count))
        .collect();
    assert_eq!(
        mine,
        [("2026-11-04".to_string(), 1), ("2026-11-03".to_string(), 1)]
    );

    let theirs: Vec<(String, i64)> = list_sessions(phone.root.clone(), "pat".to_string())
        .unwrap()
        .into_iter()
        .map(|session| (session.date, session.count))
        .collect();
    assert_eq!(
        theirs,
        [("2026-11-05".to_string(), 1), ("2026-11-04".to_string(), 1)]
    );

    assert_eq!(
        list_sessions(phone.root.clone(), "nobody".to_string()),
        Ok(Vec::<SessionView>::new())
    );
}

#[test]
fn a_detail_of_an_entry_that_is_not_here_is_refused_by_name() {
    let phone = Phone::new();
    let refused = shot_detail(phone.root.clone(), SESSION.to_string(), "1".to_string())
        .expect_err("nothing recorded");
    assert!(refused.contains(&format!("{SESSION}/1")), "{refused}");
}

/// Reading moves nothing: History opens on every tab change, and the lab's `identical` on a re-import
/// rests on an untouched file. provisional: pasted copies.
#[test]
fn looking_writes_nothing() {
    fn walk(dir: &Path, into: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)
            .expect("a directory")
            .map(|entry| entry.expect("an entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, into);
            } else {
                into.push((path.clone(), fs::read(&path).expect("a file")));
            }
        }
    }
    let phone = Phone::new();
    let card = phone.feed("IMG_2893");
    let before = {
        let mut files = Vec::new();
        walk(Path::new(&phone.root), &mut files);
        files
    };
    phone.sessions();
    phone.detail(&card);
    let after = {
        let mut files = Vec::new();
        walk(Path::new(&phone.root), &mut files);
        files
    };
    assert_eq!(before, after);
}

/// One read, over and over, for as long as another thread keeps saving a review of the same shot.
///
/// **One kind of read to a race.** A locked read beside an unlocked one lines the loop up behind each
/// save and hides the tear: with both in one loop and either lock taken out, 8 runs of 8 passed.
fn race_a_save(read: impl Fn(&Phone, &ShotCard)) {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let root = phone.root.clone();
    let (session_id, swing_id) = (card.session_id.clone(), card.swing_id.clone());
    let saver = thread::spawn(move || {
        for n in 0..SAVES {
            correct_shot(
                root.clone(),
                session_id.clone(),
                swing_id.clone(),
                vec![typed("carry_distance", &format!("{}", 130 + n % 2))],
                LATER.to_string(),
            )
            .expect("a saved review");
        }
    });
    while !saver.is_finished() {
        read(&phone, &card);
    }
    saver.join().expect("the saver");
}

/// The module doc's "one moment": `ShotStore::put` writes in place, so a read landing inside a save
/// finds a torn file and shows a real value as not read. provisional: pasted copies.
#[test]
fn a_list_taken_during_a_save_never_shows_a_held_value_as_not_read() {
    race_a_save(|phone, card| {
        let row = phone.row(card);
        assert!(row.carry_display.is_some(), "{row:?}");
    });
}

/// The same for the detail screen, whose card would go. provisional: pasted copies.
#[test]
fn a_detail_taken_during_a_save_never_loses_its_card() {
    race_a_save(|phone, card| {
        let detail = phone.detail(card);
        assert_eq!(detail.no_card, None, "{:?}", detail.row);
    });
}

/// The row's text held to the numbers M33 read, once, rather than only to the card it shares a path
/// with. provisional: pasted copies.
#[test]
fn a_rows_numbers_are_the_shots_as_the_screen_printed_them() {
    let phone = Phone::new();
    for (id, carry, ball_speed) in [
        ("IMG_2889", "131", "102.5"),
        ("IMG_2891", "162", "112.8"),
        ("IMG_2892", "80", "77.4"),
        ("IMG_2893", "184", "134"),
        ("IMG_2894", "105", "87.2"),
        ("2026-10-07-1", "279", "158.6"),
    ] {
        let row = phone.row(&phone.feed(id));
        assert_eq!(row.carry_display.as_deref(), Some(carry), "{id}");
        assert_eq!(row.ball_speed_display.as_deref(), Some(ball_speed), "{id}");
    }
}
