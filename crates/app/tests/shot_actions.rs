//! `set_shot_club`, `set_shot_intent`, `set_shot_mishit` and `delete_shot`, driven as Dart drives them,
//! against a scratch data root. [M38 P10]
//!
//! **Everything read off the six captures here is provisional: pasted copies** (rule 12). They are
//! chat-pasted JPEGs and not the originals, by M33's decision of 2026-11-04. Those files are read and
//! never written (rule 5), and a capture is always copied into the scratch root first.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use app::api::history::{
    delete_shot, list_sessions, set_shot_club, set_shot_intent, set_shot_mishit, shot_detail,
    MishitVerdict, ShotDetail, ShotRow,
};
use app::api::record::{record_photo, PhotoIn, RecordOutcome, RecordStatus, ShotCard};
use app::api::review::{correct_shot, FieldEdit};
use app::api::setup::{
    create_golfer, get_state, init_state, set_bag_slot, set_state, AppState, Handedness,
    PracticeMode, TargetShape,
};
use app::{engine_versions, Dirs};
use contracts::career::{CareerCorpus, ExclusionReason};
use contracts::club::{category_of, ClubId};
use contracts::intent::{ClubCategory, PracticeMode as StoredMode, TargetShape as StoredShape};
use contracts::mishit::MishitVerdict as StoredVerdict;
use contracts::shot::ShotData;
use contracts::shot_result::ShotResult;
use contracts::unscored::{spec_for, UnscoredReason};
use contracts::Timestamp;
use golf_core::profile::run_profile;
use serde_json::Value;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::read_corpus;
use storage::manifest::{Role, SwingManifest};
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
const NEXT_DAY: &str = "2026-11-05T09:00:00-05:00";
const NOW: &str = "2026-11-04T20:43:00-05:00";
const LATER: &str = "2026-11-04T20:44:00-05:00";
const SESSION: &str = "2026-11-04-phone-a1b2";
const NEXT_SESSION: &str = "2026-11-05-phone-a1b2";
/// How many writes each side of an overlap makes. With the lock taken out of a retag, the two clubs'
/// test failed at this count.
const OVERLAPS: usize = 40;

fn spike() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spikes/vision-ocr")
}

fn raw_boxes(id: &str) -> String {
    let path = spike().join("boxes/C0/raw").join(format!("{id}.json"));
    fs::read_to_string(&path).unwrap_or_else(|failed| panic!("{}: {failed}", path.display()))
}

/// Every directory and file under `dir`, with each file's bytes.
fn walk(dir: &Path, into: &mut Vec<(PathBuf, Option<Vec<u8>>)>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .expect("a directory")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            into.push((path.clone(), None));
            walk(&path, into);
        } else {
            let bytes = fs::read(&path).expect("a file");
            into.push((path, Some(bytes)));
        }
    }
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

    fn outcome(&self, id: &str, boxes_json: String, taken: &str) -> RecordOutcome {
        let tmp = self.dir.path().join("tmp");
        fs::create_dir_all(&tmp).expect("tmp");
        let to = tmp.join(format!("{id}.jpg"));
        fs::copy(spike().join("captures").join(format!("{id}.jpg")), &to).expect("a capture");
        record_photo(PhotoIn {
            root: self.root.clone(),
            path: to.display().to_string(),
            boxes_json,
            captured_at: taken.to_string(),
            now: NOW.to_string(),
            challenge_call: None,
        })
        .expect("a recorded photo")
    }

    fn feed_at(&self, id: &str, taken: &str) -> ShotCard {
        self.outcome(id, raw_boxes(id), taken).card.expect("a card")
    }

    fn feed(&self, id: &str) -> ShotCard {
        self.feed_at(id, TAKEN)
    }

    /// `IMG_2889` with one value box edited so a physics check flags it, as `tests/record.rs` does,
    /// since no capture trips one.
    fn flagged(&self) -> ShotCard {
        let text = raw_boxes("IMG_2889").replace("\"1.08\"", "\"2.08\"");
        let card = self
            .outcome("IMG_2889", text, TAKEN)
            .card
            .expect("a flagged card");
        assert!(card.flagged);
        card
    }

    fn club(&self, card: &ShotCard, club: &str) -> Result<ShotDetail, String> {
        set_shot_club(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
            club.to_string(),
            LATER.to_string(),
        )
    }

    fn intent(
        &self,
        card: &ShotCard,
        mode: PracticeMode,
        shape: Option<TargetShape>,
    ) -> Result<ShotDetail, String> {
        set_shot_intent(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
            mode,
            shape,
            LATER.to_string(),
        )
    }

    fn verdict(
        &self,
        card: &ShotCard,
        verdict: Option<MishitVerdict>,
    ) -> Result<ShotDetail, String> {
        set_shot_mishit(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
            verdict,
            LATER.to_string(),
        )
    }

    fn delete(&self, card: &ShotCard) -> Result<(), String> {
        delete_shot(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
        )
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

    fn rows(&self, player_id: &str) -> Vec<ShotRow> {
        list_sessions(self.root.clone(), player_id.to_string())
            .expect("a list")
            .into_iter()
            .flat_map(|session| session.rows)
            .collect()
    }

    fn row(&self, card: &ShotCard) -> ShotRow {
        self.rows("aaron")
            .into_iter()
            .find(|row| row.session_id == card.session_id && row.swing_id == card.swing_id)
            .expect("the card's row")
    }

    fn detail(&self, card: &ShotCard) -> ShotDetail {
        shot_detail(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
        )
        .expect("a detail")
    }

    fn sessions_dir(&self) -> PathBuf {
        Dirs::of(&self.root).sessions
    }

    fn swing_dir(&self, card: &ShotCard) -> PathBuf {
        self.sessions_dir()
            .join(&card.session_id)
            .join(&card.swing_id)
    }

    fn manifest(&self, card: &ShotCard) -> SwingManifest {
        SwingBundleStore::new(self.sessions_dir())
            .get_swing(&card.session_id, &card.swing_id)
            .expect("the card's manifest")
    }

    fn analysis(&self, card: &ShotCard) -> ShotResult {
        load_shot_analysis(&self.swing_dir(card)).expect("an analysis")
    }

    fn photo_digest(&self, card: &ShotCard) -> String {
        self.manifest(card).roles[&Role::ShotScreen]
            .content_sha256
            .clone()
    }

    fn shot_store(&self) -> ShotStore {
        ShotStore::new(Dirs::of(&self.root).shots)
    }

    fn stored_shot(&self, card: &ShotCard) -> ShotData {
        self.shot_store()
            .get(&self.photo_digest(card))
            .expect("a shot that reads")
            .expect("a stored shot")
    }

    fn corpus(&self) -> CareerCorpus {
        read_corpus(&self.sessions_dir(), "aaron", engine_versions())
    }

    /// How many shots the graded profile counts for one club, which is what the Profile tab will show.
    fn shots_counted(&self, club: ClubId) -> i64 {
        run_profile(&self.corpus(), None)
            .expect("a profile")
            .clubs
            .iter()
            .find(|graded| graded.grades.club == club)
            .map_or(0, |graded| graded.grades.n_shots)
    }

    fn everything(&self) -> Vec<(PathBuf, Option<Vec<u8>>)> {
        let mut files = Vec::new();
        walk(Path::new(&self.root), &mut files);
        files
    }

    /// A second entry holding `card`'s photo, the manifest alone and on a later date, as
    /// `tests/history.rs` makes one: the phone never opens this itself, and the corpus's rule still
    /// has to hold over it.
    fn duplicate_of(&self, card: &ShotCard) -> ShotCard {
        let copy = self.sessions_dir().join(NEXT_SESSION).join("1");
        fs::create_dir_all(&copy).expect("a directory");
        let mut manifest: Value = serde_json::from_slice(
            &fs::read(self.swing_dir(card).join("manifest.json")).expect("a manifest"),
        )
        .expect("JSON");
        manifest["session_id"] = Value::String(NEXT_SESSION.to_string());
        manifest["swing_id"] = Value::String("1".to_string());
        manifest["created_at"] = Value::String(NEXT_DAY.to_string());
        fs::write(
            copy.join("manifest.json"),
            serde_json::to_vec(&manifest).expect("JSON"),
        )
        .expect("a written manifest");
        ShotCard {
            session_id: NEXT_SESSION.to_string(),
            swing_id: "1".to_string(),
            ..card.clone()
        }
    }

    /// Take every shot out of the store, as P9's entry with no shot has lost it.
    fn lose_every_shot(&self) {
        for entry in fs::read_dir(Dirs::of(&self.root).shots).expect("the store") {
            fs::remove_file(entry.expect("an entry").path()).expect("a removed shot");
        }
    }

    /// Say a stored shot came from a device `devices.json` does not declare, so `run_shot` refuses it.
    fn make_unanalysable(&self, card: &ShotCard) {
        let path = self.shot_store().path_for(&self.photo_digest(card));
        let mut shot: Value =
            serde_json::from_slice(&fs::read(&path).expect("a shot")).expect("JSON");
        shot["provenance"]["device"] = Value::String("no_such_device".to_string());
        fs::write(&path, serde_json::to_vec(&shot).expect("JSON")).expect("a written shot");
    }
}

fn typed(field: &str, printed: &str) -> FieldEdit {
    FieldEdit {
        field: field.to_string(),
        printed: Some(printed.to_string()),
        letter: None,
    }
}

fn later() -> Timestamp {
    Timestamp::parse(LATER).expect("a time")
}

fn now() -> Timestamp {
    Timestamp::parse(NOW).expect("a time")
}

// -------------------------------------------------------------------------------------- the club

/// The one repair every session needs. It reaches all three things that say which club hit a shot: the
/// manifest of record, the analysis, whose category comes from it, and the numbers the corpus keeps.
/// provisional: pasted copies.
#[test]
fn a_new_club_reaches_the_manifest_the_analysis_and_the_corpus() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    assert_eq!(phone.analysis(&card).intent.club, ClubCategory::MidIron);
    assert_eq!(phone.shots_counted(ClubId::SevenIron), 1);

    let detail = phone.club(&card, "driver").expect("a retag");

    assert_eq!(detail.row.club.as_deref(), Some("driver"));
    assert_eq!(phone.manifest(&card).club, Some(ClubId::Driver));
    assert_eq!(phone.manifest(&card).updated_at, later());
    assert_eq!(phone.analysis(&card).intent.club, ClubCategory::Driver);
    assert_eq!(phone.corpus().swings[0].club, Some(ClubId::Driver));
    assert_eq!(phone.shots_counted(ClubId::SevenIron), 0);
    assert_eq!(phone.shots_counted(ClubId::Driver), 1);
}

/// Why a retag has to re-analyse and not only stamp: the club picks the loft the shot flies with. A
/// club with a loft on record flies, the note goes, and going back brings the note back, so it is the
/// analysis that moved each time. provisional: pasted copies.
#[test]
fn a_new_club_flies_the_shot_with_that_clubs_loft() {
    let phone = Phone::new();
    set_bag_slot(
        phone.root.clone(),
        "aaron".to_string(),
        "pw".to_string(),
        Some(46.0),
        NOW.to_string(),
    )
    .expect("a slot with a loft");
    let no_loft = spec_for(UnscoredReason::NoClubLoft).remedy;
    let card = phone.feed("IMG_2891");
    let before = phone.detail(&card);
    assert_eq!(before.notes.len(), 1, "{:?}", before.notes);
    assert!(before.unscored.iter().any(|row| row.reason == no_loft));

    let flown = phone.club(&card, "pw").expect("a retag");
    assert_eq!(flown.notes, Vec::<String>::new());
    assert!(!flown.unscored.iter().any(|row| row.reason == no_loft));
    assert!(
        flown.measurements.len() > before.measurements.len(),
        "{} against {}",
        flown.measurements.len(),
        before.measurements.len()
    );

    let back = phone.club(&card, "7i").expect("a retag");
    assert_eq!(back.notes, before.notes);
    assert_eq!(back.measurements, before.measurements);
}

/// A retag is about the club and nothing else: what a person typed, and a flag no person has looked at,
/// are each still there. Clearing a flag is what a review is for. provisional: pasted copies.
#[test]
fn a_retag_keeps_a_persons_correction_and_a_flag_nobody_has_looked_at() {
    let phone = Phone::new();
    let flagged = phone.flagged();
    let detail = phone.club(&flagged, "pw").expect("a retag");
    assert!(detail.row.flagged);
    assert!(phone.corpus().swings[0].shot_needs_review);

    let card = phone.feed("IMG_2891");
    phone.correct(&card, vec![typed("carry_distance", "150")]);
    let before = phone.stored_shot(&card);
    let detail = phone.club(&card, "pw").expect("a retag");
    assert!(detail.row.corrected);
    assert_eq!(phone.stored_shot(&card), before);
    assert_eq!(phone.analysis(&card).shot.carry_distance, Some(150.0));
}

/// `ClubId`'s own spelling alone, as C2 says: every club Dart holds came from `all_clubs()` or a stored
/// shot. provisional: pasted copies.
#[test]
fn a_club_that_is_not_its_own_spelling_is_refused_and_writes_nothing() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let before = phone.everything();
    for club in ["7 iron", "7I", ""] {
        let refused = phone.club(&card, club).expect_err(club);
        assert!(refused.contains(&format!("'{club}'")), "{refused}");
    }
    assert_eq!(phone.everything(), before);
}

/// Opening the picker and choosing what is already there is not a change, and a stamp would make a
/// re-import at the lab read the entry as differing. provisional: pasted copies.
#[test]
fn choosing_the_club_it_already_has_moves_no_byte() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let before = phone.everything();
    phone.club(&card, "7i").expect("the same club");
    assert_eq!(phone.everything(), before);
}

/// So picking the same club is also a repair: P9's entry whose analysis was lost gets it back.
/// provisional: pasted copies.
#[test]
fn choosing_the_club_it_already_has_analyses_it_again() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let held = phone.swing_dir(&card).join("shot_analysis.json");
    let bytes = fs::read(&held).expect("an analysis");
    fs::remove_file(&held).expect("a removed analysis");
    assert!(!phone.corpus().swings[0].analyzed);
    phone.club(&card, "7i").expect("the same club");
    assert_eq!(fs::read(&held).expect("an analysis"), bytes);
}

// ------------------------------------------------------------------------------------ the intent

/// C15's two rules, which are `intent_action`'s with no second copy, each refused before anything moves.
/// Dart should not offer either, so this is what shows if it does. provisional: pasted copies.
#[test]
fn a_shape_with_no_shot_shaping_and_shot_shaping_with_no_shape_are_refused_by_intent_actions_rule()
{
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let before = phone.everything();

    for mode in [
        PracticeMode::Fundamentals,
        PracticeMode::Performance,
        PracticeMode::Drill,
    ] {
        let refused = phone
            .intent(&card, mode, Some(TargetShape::Draw))
            .expect_err("a shape on another mode");
        assert!(refused.contains("do not go together"), "{refused}");
        assert!(refused.contains("declares no shape"), "{refused}");
    }
    let refused = phone
        .intent(&card, PracticeMode::ShotShaping, None)
        .expect_err("shot shaping with nothing to shape");
    assert!(refused.contains("do not go together"), "{refused}");
    assert!(refused.contains("grades nothing"), "{refused}");

    assert_eq!(phone.everything(), before);
}

/// The goal recorded holds the mode and the shape, and the category stays the analysis's to fill from
/// the club (the M35 plan's call 5), so a retag afterwards cannot leave a stale one on the manifest.
/// provisional: pasted copies.
#[test]
fn a_shape_reaches_the_row_the_manifest_the_analysis_and_the_corpus() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");

    let detail = phone
        .intent(&card, PracticeMode::ShotShaping, Some(TargetShape::Draw))
        .expect("a shape");

    assert_eq!(detail.row.mode, PracticeMode::ShotShaping);
    assert_eq!(detail.row.target_shape, Some(TargetShape::Draw));
    assert!(!detail.row.untracked);

    let manifest = phone.manifest(&card);
    assert_eq!(manifest.updated_at, later());
    let goal = manifest.intent.expect("a goal");
    assert_eq!(goal.mode, StoredMode::ShotShaping);
    assert_eq!(goal.target_shape, Some(StoredShape::Draw));
    assert_eq!(goal.club, ClubCategory::All, "the analysis fills this");

    let analysed = phone.analysis(&card).intent;
    assert_eq!(analysed.mode, StoredMode::ShotShaping);
    assert_eq!(analysed.target_shape, Some(StoredShape::Draw));
    assert_eq!(analysed.club, ClubCategory::MidIron);

    let corpus = phone.corpus();
    assert_eq!(
        corpus.swings[0].intent.as_ref().map(|goal| goal.mode),
        Some(StoredMode::ShotShaping)
    );
}

/// The drill's own sentence says `Change what it was practised as if it should count`, and this is what
/// makes that true, in both directions, over the numbers a person will read.
/// provisional: pasted copies.
#[test]
fn a_drill_leaves_the_aggregates_and_changing_it_back_returns_it() {
    let phone = Phone::new();
    let cards: Vec<ShotCard> = CAPTURES.iter().map(|id| phone.feed(id)).collect();
    assert_eq!(phone.shots_counted(ClubId::SevenIron), 6);
    assert_eq!(phone.corpus().swings.len(), 6);

    let detail = phone
        .intent(&cards[0], PracticeMode::Drill, None)
        .expect("a drill");

    assert!(detail.row.untracked);
    assert_eq!(detail.row.left_out.len(), 1, "{:?}", detail.row.left_out);
    assert!(detail.row.left_out[0].starts_with("Hit as a drill"));
    assert_eq!(phone.shots_counted(ClubId::SevenIron), 5);
    let corpus = phone.corpus();
    assert_eq!(corpus.swings.len(), 5);
    assert_eq!(corpus.excluded.len(), 1);
    assert_eq!(corpus.excluded[0].reason, ExclusionReason::Untracked);
    assert_eq!(
        phone.analysis(&cards[0]).intent.mode,
        StoredMode::Drill,
        "a drill is analysed like any shot, and the corpus is what leaves it out"
    );

    let detail = phone
        .intent(&cards[0], PracticeMode::Fundamentals, None)
        .expect("back");

    assert!(!detail.row.untracked);
    assert_eq!(detail.row.left_out, Vec::<String>::new());
    assert_eq!(phone.shots_counted(ClubId::SevenIron), 6);
    let corpus = phone.corpus();
    assert_eq!(corpus.swings.len(), 6);
    assert_eq!(corpus.excluded.len(), 0);
}

/// As the club's. provisional: pasted copies.
#[test]
fn choosing_the_intent_it_already_has_moves_no_byte() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let before = phone.everything();
    phone
        .intent(&card, PracticeMode::Fundamentals, None)
        .expect("the same goal");
    assert_eq!(phone.everything(), before);

    phone
        .intent(&card, PracticeMode::ShotShaping, Some(TargetShape::Fade))
        .expect("a shape");
    let before = phone.everything();
    phone
        .intent(&card, PracticeMode::ShotShaping, Some(TargetShape::Fade))
        .expect("the same shape");
    assert_eq!(phone.everything(), before);
}

/// A manifest naming no goal reads as `fundamentals`, the lab's before M35 and frozen Python's after a
/// rewrite, so saying that is saying what it already says. provisional: pasted copies.
#[test]
fn fundamentals_on_an_entry_naming_no_goal_moves_no_byte() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    SwingBundleStore::new(phone.sessions_dir())
        .set_intent(&card.session_id, &card.swing_id, None, now())
        .expect("a written manifest")
        .expect("the card's entry");
    let before = phone.everything();
    phone
        .intent(&card, PracticeMode::Fundamentals, None)
        .expect("the same goal");
    assert_eq!(phone.everything(), before);
}

// ------------------------------------------------------------------------------------ the mishit

/// D10. Six carries of one club, one corrected to a top so ADR-028's rule holds it out: each verdict
/// reaches the corpus's own, a verdict overrules the rule either way, and `None` gives it back.
/// provisional: pasted copies.
#[test]
fn each_verdict_reaches_the_corpuss_verdict_and_none_gives_it_back_to_the_rule() {
    let phone = Phone::new();
    let cards: Vec<ShotCard> = CAPTURES.iter().map(|id| phone.feed(id)).collect();
    let topped = phone.correct(&cards[2], vec![typed("carry_distance", "20")]);
    let in_corpus = |phone: &Phone, card: &ShotCard| {
        phone
            .corpus()
            .swings
            .into_iter()
            .find(|swing| swing.swing_id == card.swing_id)
            .expect("a counted shot")
    };
    assert!(in_corpus(&phone, &topped).auto_mishit);
    assert!(phone.row(&topped).mishit);

    let detail = phone
        .verdict(&topped, Some(MishitVerdict::Cleared))
        .expect("a verdict");
    assert_eq!(detail.row.mishit_verdict, Some(MishitVerdict::Cleared));
    assert!(!detail.row.mishit, "a person's verdict beats the rule");
    let swing = in_corpus(&phone, &topped);
    assert_eq!(swing.manual_mishit, Some(StoredVerdict::Cleared));
    assert!(!swing.is_mishit());

    let detail = phone
        .verdict(&cards[0], Some(MishitVerdict::Confirmed))
        .expect("a verdict");
    assert_eq!(detail.row.mishit_verdict, Some(MishitVerdict::Confirmed));
    assert!(detail.row.mishit, "a shot the rule cannot see");
    let swing = in_corpus(&phone, &cards[0]);
    assert_eq!(swing.manual_mishit, Some(StoredVerdict::Confirmed));
    assert!(!swing.auto_mishit);
    assert!(swing.is_mishit());

    let detail = phone.verdict(&topped, None).expect("no verdict");
    assert_eq!(detail.row.mishit_verdict, None);
    assert!(detail.row.mishit, "back to the rule");
    let swing = in_corpus(&phone, &topped);
    assert_eq!(swing.manual_mishit, None);
    assert!(swing.is_mishit());
}

/// A verdict is read off the manifest by the corpus, and by no analysis, so nothing else has any reason
/// to move. provisional: pasted copies.
#[test]
fn a_verdict_moves_the_manifest_and_nothing_else() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let manifest_path = phone.swing_dir(&card).join("manifest.json");
    let without_manifest = |phone: &Phone| {
        let mut files = phone.everything();
        files.retain(|(path, _)| *path != manifest_path);
        files
    };
    let before = without_manifest(&phone);
    phone
        .verdict(&card, Some(MishitVerdict::Confirmed))
        .expect("a verdict");
    assert_eq!(without_manifest(&phone), before);
    assert_eq!(phone.manifest(&card).updated_at, later());
}

/// As the club's and the intent's: the control starts on what is stored, so a tap on it says it again.
/// provisional: pasted copies.
#[test]
fn the_verdict_it_already_has_moves_no_byte() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let before = phone.everything();
    phone
        .verdict(&card, None)
        .expect("no verdict, as it stands");
    assert_eq!(phone.everything(), before);

    phone
        .verdict(&card, Some(MishitVerdict::Cleared))
        .expect("a verdict");
    let before = phone.everything();
    phone
        .verdict(&card, Some(MishitVerdict::Cleared))
        .expect("the same verdict");
    assert_eq!(phone.everything(), before);
}

// ------------------------------------------------------------------------------------ the delete

/// D10's stray photo: gone from History and from the corpus, with no shot left in the store and no empty
/// date left for a list to walk. provisional: pasted copies.
#[test]
fn delete_removes_the_entry_its_shot_and_an_emptied_session() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let digest = phone.photo_digest(&card);
    assert!(phone.shot_store().has(&digest));

    phone.delete(&card).expect("a delete");

    assert!(!phone.swing_dir(&card).exists());
    assert!(!phone.sessions_dir().join(&card.session_id).exists());
    assert!(!phone.shot_store().has(&digest));
    assert_eq!(phone.rows("aaron"), Vec::<ShotRow>::new());
    assert_eq!(phone.corpus().swings.len(), 0);
    assert_eq!(phone.corpus().excluded.len(), 0);
    let refused = shot_detail(
        phone.root.clone(),
        card.session_id.clone(),
        card.swing_id.clone(),
    )
    .expect_err("nothing left to show");
    assert!(refused.contains("is not on this phone"), "{refused}");
}

/// One stray photo in a good session costs nothing else. provisional: pasted copies.
#[test]
fn delete_leaves_the_rest_of_the_session_as_it_was() {
    let phone = Phone::new();
    let kept = phone.feed("IMG_2889");
    let stray = phone.feed("IMG_2891");
    let kept_detail = phone.detail(&kept);
    let kept_manifest = phone.manifest(&kept);
    let kept_analysis = fs::read(phone.swing_dir(&kept).join("shot_analysis.json")).unwrap();

    phone.delete(&stray).expect("a delete");

    assert_eq!(phone.detail(&kept), kept_detail);
    assert_eq!(phone.manifest(&kept), kept_manifest);
    assert_eq!(
        fs::read(phone.swing_dir(&kept).join("shot_analysis.json")).unwrap(),
        kept_analysis
    );
    assert_eq!(phone.rows("aaron").len(), 1);
    assert_eq!(phone.corpus().swings.len(), 1);
}

/// The Done line's last clause. Before the delete the same bytes answer the held shot, and after it they
/// are read as if for the first time, with the newer read's boxes kept, as `keep_boxes` promises.
/// provisional: pasted copies.
#[test]
fn a_deleted_photo_can_be_recorded_afresh() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let digest = phone.photo_digest(&card);
    let again = phone.outcome("IMG_2889", raw_boxes("IMG_2889"), TAKEN);
    assert_eq!(again.status, RecordStatus::Duplicate);

    phone.delete(&card).expect("a delete");

    let boxes = Dirs::of(&phone.root)
        .app
        .join("boxes")
        .join(format!("{digest}.json"));
    assert!(boxes.exists(), "the diagnostics of a read that happened");
    let newer = format!("{}\n", raw_boxes("IMG_2889"));
    let fresh = phone.outcome("IMG_2889", newer.clone(), TAKEN);
    assert_eq!(fresh.status, RecordStatus::Read);
    assert_eq!(fresh.message, None);
    assert_eq!(fs::read_to_string(&boxes).unwrap(), newer);

    let fresh = fresh.card.expect("a card");
    assert_eq!(fresh.rows, card.rows);
    assert_eq!(phone.rows("aaron").len(), 1);
    assert_eq!(phone.corpus().swings.len(), 1);
    assert_eq!(phone.corpus().excluded.len(), 0);
    assert!(phone.shot_store().has(&digest));
}

/// P6 finding 7, now run. `Duplicates` is per golfer, so one photo under two golfers in two sessions is
/// two entries over one record. Whichever goes first, the other keeps its values, and the record goes
/// only with the last. provisional: pasted copies.
#[test]
fn delete_leaves_the_shot_another_golfers_entry_still_holds() {
    for first_goes_first in [true, false] {
        let phone = Phone::new();
        create_golfer(
            phone.root.clone(),
            "Bea".to_string(),
            Handedness::Left,
            NOW.to_string(),
        )
        .expect("a second golfer");
        let aarons = phone.feed("IMG_2889");
        phone.choose(|state| state.player_id = Some("bea".to_string()));
        let beas = phone.feed_at("IMG_2889", NEXT_DAY);
        assert_ne!(aarons.session_id, beas.session_id);
        let digest = phone.photo_digest(&aarons);
        assert_eq!(digest, phone.photo_digest(&beas));

        let (goes, stays) = if first_goes_first {
            (&aarons, &beas)
        } else {
            (&beas, &aarons)
        };
        let rows = phone.detail(stays).card.expect("a card").rows;
        phone.delete(goes).expect("a delete");

        assert!(phone.shot_store().has(&digest), "{first_goes_first}");
        let left = phone.detail(stays);
        assert_eq!(left.no_card, None, "{first_goes_first}");
        assert_eq!(left.card.expect("a card").rows, rows, "{first_goes_first}");
        assert_eq!(left.row.left_out, Vec::<String>::new());

        phone.delete(stays).expect("a delete");
        assert!(!phone.shot_store().has(&digest), "{first_goes_first}");
    }
}

/// The corpus's own duplicate, the same golfer's. Deleting it costs the kept entry nothing, since the
/// two share a record. provisional: pasted copies.
#[test]
fn deleting_a_duplicate_leaves_the_entry_it_duplicates() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let copy = phone.duplicate_of(&card);
    let before = phone.detail(&card);

    phone.delete(&copy).expect("a delete");

    assert!(phone.shot_store().has(&phone.photo_digest(&card)));
    assert_eq!(phone.detail(&card), before);
    assert!(!phone.sessions_dir().join(NEXT_SESSION).exists());
    assert_eq!(phone.corpus().excluded.len(), 0);
}

/// The other way round, the copy is the one left to count, and it was never analysed. Its row says so,
/// and the repair that sentence offers works, because its shot's record stayed.
/// provisional: pasted copies.
#[test]
fn deleting_what_a_duplicate_duplicates_leaves_it_to_count_once_it_is_analysed() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let copy = phone.duplicate_of(&card);
    let digest = phone.photo_digest(&card);

    phone.delete(&card).expect("a delete");

    assert!(phone.shot_store().has(&digest));
    let detail = phone.detail(&copy);
    assert!(detail.card.is_some());
    assert_eq!(detail.row.left_out.len(), 1, "{:?}", detail.row.left_out);
    assert!(detail.row.left_out[0].contains("Saving a review"));

    phone.correct(&copy, Vec::new());
    assert_eq!(phone.detail(&copy).row.left_out, Vec::<String>::new());
    assert_eq!(phone.corpus().swings.len(), 1);
}

/// What is left to do with an entry whose shot is gone, since `set_shot_club` refuses it.
/// provisional: pasted copies.
#[test]
fn an_entry_whose_shot_is_gone_can_still_be_deleted() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    phone.lose_every_shot();
    phone.delete(&card).expect("a delete");
    assert!(!phone.swing_dir(&card).exists());
    assert_eq!(phone.rows("aaron"), Vec::<ShotRow>::new());
}

/// `delete_swing` removes whatever directory it is pointed at, recursively. `..` with a manifest to read
/// there would be every session, so the decoy is what makes this bite: the manifest check alone passes
/// it. provisional: pasted copies.
#[test]
fn delete_refuses_an_id_that_is_not_a_path_segment_and_removes_nothing() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let sessions = phone.sessions_dir();
    fs::copy(
        phone.swing_dir(&card).join("manifest.json"),
        sessions.join("manifest.json"),
    )
    .expect("a decoy");
    let before = phone.everything();

    for (session, swing) in [
        (SESSION, ".."),
        (SESSION, "."),
        ("..", "sessions"),
        (SESSION, "../2026-11-04-phone-a1b2/1"),
        ("", "1"),
        (SESSION, ""),
    ] {
        let refused = delete_shot(phone.root.clone(), session.to_string(), swing.to_string())
            .expect_err(swing);
        assert!(refused.contains("is not one the store names"), "{refused}");
    }
    assert_eq!(phone.everything(), before);
}

/// The user's decision of 2026-11-05, on P10's question: `crate::record`'s module doc says why, and
/// `tests/export_roundtrip.rs` pins what it is for. Deleting the newest entry spends its number, and
/// deleting an older one after it does not bring the floor back down. provisional: pasted copies.
#[test]
fn a_deleted_number_is_never_given_to_another_photo() {
    let phone = Phone::new();
    let first = phone.feed("IMG_2889");
    let second = phone.feed("IMG_2891");
    assert_eq!(second.swing_id, "2");

    phone.delete(&second).expect("a delete");
    let third = phone.feed("IMG_2892");
    assert_eq!(third.swing_id, "3", "the number after the one deleted");

    phone.delete(&third).expect("a delete");
    phone.delete(&first).expect("a delete");
    let fourth = phone.feed("IMG_2893");
    assert_eq!(
        fourth.swing_id, "4",
        "the session holds nothing, and 3 is spent"
    );
}

/// The entry and its session both go, so the number has to live somewhere else, and a date of its own
/// starts at 1. provisional: pasted copies.
#[test]
fn a_number_outlives_its_session_and_binds_no_other_session() {
    let phone = Phone::new();
    let only = phone.feed("IMG_2889");
    phone.delete(&only).expect("a delete");
    assert!(!phone.sessions_dir().join(&only.session_id).exists());

    assert_eq!(phone.feed_at("IMG_2891", NEXT_DAY).swing_id, "1");
    assert_eq!(phone.feed("IMG_2892").swing_id, "2");
}

/// C13's entry, opened and taken back by one call, was never seen by an export, so it spends nothing. A
/// retake keeps the number, as `tests/record.rs` has it, and no file says otherwise. provisional: pasted
/// copies.
#[test]
fn a_read_of_nothing_spends_no_number() {
    let phone = Phone::new();
    let failed = phone.outcome("IMG_2889", "[]".to_string(), TAKEN);
    assert_eq!(failed.status, RecordStatus::Failed);

    assert_eq!(phone.feed("IMG_2891").swing_id, "1");
    assert!(!Dirs::of(&phone.root).app.join("deleted.json").exists());
}

/// A number written after the entry went could be lost to a crash between the two, so it comes first,
/// and if it cannot be written the entry stays: the next photo must not be able to take this number
/// because a delete half happened. provisional: pasted copies.
#[test]
fn a_delete_that_cannot_spend_its_number_removes_nothing() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    fs::create_dir(Dirs::of(&phone.root).app.join("deleted.tmp")).expect("in the way");
    let before = phone.everything();

    let refused = phone
        .delete(&card)
        .expect_err("its number cannot be written");

    assert!(refused.contains("could not delete"), "{refused}");
    assert_eq!(phone.everything(), before);
}

/// `crate::state`'s rule for a file of the app's own: a torn one costs nothing on the screen a person is
/// holding between swings, and the store's own number stands until the next delete writes it whole.
/// provisional: pasted copies.
#[test]
fn a_torn_numbers_file_costs_no_shot_and_the_next_delete_writes_it_whole() {
    let phone = Phone::new();
    let path = Dirs::of(&phone.root).app.join("deleted.json");
    fs::write(&path, b"{\"2026-11-04").expect("torn");

    let card = phone.feed("IMG_2889");
    assert_eq!(card.swing_id, "1");
    phone.delete(&card).expect("a delete");

    assert_eq!(phone.feed("IMG_2891").swing_id, "2");
    let held: Value = serde_json::from_slice(&fs::read(&path).unwrap()).expect("whole");
    assert_eq!(held[&card.session_id], 1);
}

// ------------------------------------------------------------------------------- what is refused

/// Every writer but the delete stamps with `now`, and `parse_now`'s trap is Dart's naive `toIso8601String`.
/// provisional: pasted copies.
#[test]
fn a_time_with_no_offset_is_refused_by_every_writer_that_takes_one_and_writes_nothing() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let before = phone.everything();
    let naive = || "2026-11-04T20:44:00".to_string();
    let (root, session, swing) = (
        || phone.root.clone(),
        || card.session_id.clone(),
        || card.swing_id.clone(),
    );

    for refused in [
        set_shot_club(root(), session(), swing(), "pw".to_string(), naive()),
        set_shot_intent(
            root(),
            session(),
            swing(),
            PracticeMode::Drill,
            None,
            naive(),
        ),
        set_shot_mishit(
            root(),
            session(),
            swing(),
            Some(MishitVerdict::Confirmed),
            naive(),
        ),
    ] {
        let refused = refused.expect_err("a naive time");
        assert!(refused.contains("offset"), "{refused}");
    }
    assert_eq!(phone.everything(), before);
}

/// An entry that is not there is named, nothing is created for it, and a writer that joins ids to a path
/// refuses what is not a segment before it looks. provisional: pasted copies.
#[test]
fn an_entry_that_is_not_here_is_refused_by_name_and_nothing_is_created() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let gone = ShotCard {
        swing_id: "9".to_string(),
        ..card.clone()
    };
    let not_a_segment = ShotCard {
        swing_id: "..".to_string(),
        ..card.clone()
    };
    let before = phone.everything();

    for refused in [
        phone.club(&gone, "pw").map(|_| ()),
        phone.intent(&gone, PracticeMode::Drill, None).map(|_| ()),
        phone
            .verdict(&gone, Some(MishitVerdict::Confirmed))
            .map(|_| ()),
        phone.delete(&gone),
    ] {
        let refused = refused.expect_err("no such entry");
        assert!(refused.contains(&format!("{SESSION}/9")), "{refused}");
    }
    for refused in [
        phone.club(&not_a_segment, "pw").map(|_| ()),
        phone
            .intent(&not_a_segment, PracticeMode::Drill, None)
            .map(|_| ()),
        phone
            .verdict(&not_a_segment, Some(MishitVerdict::Confirmed))
            .map(|_| ()),
    ] {
        let refused = refused.expect_err("not a segment");
        assert!(refused.contains("is not one the store names"), "{refused}");
    }
    assert_eq!(phone.everything(), before);
}

/// `correct_shot`'s refusal, which is now one definition: a duplicate shares its shot's record with the
/// entry it duplicates, and its analysis would be one no reader reads. provisional: pasted copies.
#[test]
fn a_duplicate_cannot_be_retagged_and_says_which_entry_counts() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let copy = phone.duplicate_of(&card);
    let before = phone.everything();

    for refused in [
        phone.club(&copy, "pw").map(|_| ()),
        phone.intent(&copy, PracticeMode::Drill, None).map(|_| ()),
    ] {
        let refused = refused.expect_err("a duplicate");
        assert!(
            refused.contains("is not the entry its photo counts under"),
            "{refused}"
        );
        assert!(refused.contains(&format!("{SESSION}/1")), "{refused}");
    }
    assert_eq!(phone.everything(), before);
}

/// With no shot there is nothing to analyse, so the retag does not half happen. A verdict reads no shot,
/// so it stands. provisional: pasted copies.
#[test]
fn an_entry_whose_shot_is_gone_takes_a_verdict_and_refuses_a_retag() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    phone.lose_every_shot();
    let before = phone.everything();

    for refused in [
        phone.club(&card, "pw").map(|_| ()),
        phone.intent(&card, PracticeMode::Drill, None).map(|_| ()),
    ] {
        let refused = refused.expect_err("no shot");
        assert!(refused.contains("holds no shot"), "{refused}");
    }
    assert_eq!(phone.everything(), before);

    let detail = phone
        .verdict(&card, Some(MishitVerdict::Confirmed))
        .expect("a verdict");
    assert_eq!(detail.row.mishit_verdict, Some(MishitVerdict::Confirmed));
}

/// Analysed before it is stored, so a refused analysis leaves the two agreeing, the stamp included.
/// Shown with the change a person is most likely to make after one. provisional: pasted copies.
#[test]
fn an_analysis_that_is_refused_moves_nothing() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    phone.make_unanalysable(&card);
    let before = phone.everything();

    for refused in [
        phone.club(&card, "pw").map(|_| ()),
        phone.intent(&card, PracticeMode::Drill, None).map(|_| ()),
    ] {
        let refused = refused.expect_err("an analysis refused");
        assert!(refused.contains("could not be analysed"), "{refused}");
    }
    assert_eq!(phone.everything(), before);
}

/// The other side of that order: the analysis has run under the new club, and then the manifest cannot be
/// written. The analysis goes back, so the two never disagree. Writing a manifest makes `manifest.tmp`
/// beside it, so a directory of that name stops it and nothing else. provisional: pasted copies.
#[test]
fn a_manifest_that_cannot_be_written_puts_the_analysis_back() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let blocker = phone.swing_dir(&card).join("manifest.tmp");
    fs::create_dir(&blocker).expect("a directory");
    let before = phone.everything();

    let refused = phone
        .club(&card, "driver")
        .expect_err("no manifest written");
    assert!(refused.contains("could not write"), "{refused}");
    assert!(refused.contains(&format!("{SESSION}/1")), "{refused}");
    assert_eq!(phone.everything(), before);

    fs::remove_dir(&blocker).expect("the directory gone");
    phone.club(&card, "driver").expect("a retag");
    assert_eq!(phone.manifest(&card).club, Some(ClubId::Driver));
    assert_eq!(phone.analysis(&card).intent.club, ClubCategory::Driver);
}

// ----------------------------------------------------------------------------------- the answers

/// Each writer's answer is what the screen would read for itself, so P25 needs no second call. It also
/// shows none of them holds the lock across the read: `std`'s `Mutex` does not re-enter, and this would
/// hang. provisional: pasted copies.
#[test]
fn every_writers_answer_is_the_detail_as_it_now_stands() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    let answer = phone.club(&card, "pw").expect("a retag");
    assert_eq!(answer, phone.detail(&card));
    let answer = phone
        .intent(&card, PracticeMode::ShotShaping, Some(TargetShape::Fade))
        .expect("a shape");
    assert_eq!(answer, phone.detail(&card));
    let answer = phone
        .verdict(&card, Some(MishitVerdict::Cleared))
        .expect("a verdict");
    assert_eq!(answer, phone.detail(&card));
}

// ------------------------------------------------------------------------------------ the overlap

/// Two retags overlapping, as a double tap or a batch beside a detail screen can. The manifest and the
/// analysis have to name one club at the end, whichever won: an analysis left behind by another call's
/// manifest is a shot flown with the wrong loft and nothing to say so. provisional: pasted copies.
#[test]
fn overlapping_retags_leave_the_manifest_and_the_analysis_agreeing() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    thread::scope(|scope| {
        for club in ["driver", "pw"] {
            let (phone, card) = (&phone, &card);
            scope.spawn(move || {
                for _ in 0..OVERLAPS {
                    phone.club(card, club).expect("a retag");
                }
            });
        }
    });
    let club = phone.manifest(&card).club.expect("a club");
    assert_eq!(phone.analysis(&card).intent.club, category_of(club));
}

/// A verdict beside a retag each rewrite one manifest, and neither may lose what the other wrote. This
/// is what they end on, and not what shows the lock: a run with the verdict's taken out still passed,
/// since each answer's read queues behind the retag. The next test is that.
/// provisional: pasted copies.
#[test]
fn a_verdict_and_a_retag_overlapping_both_land() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    thread::scope(|scope| {
        let (first, second) = ((&phone, &card), (&phone, &card));
        scope.spawn(move || {
            for club in ["driver", "pw"].repeat(OVERLAPS / 2) {
                first.0.club(first.1, club).expect("a retag");
            }
        });
        scope.spawn(move || {
            for verdict in [MishitVerdict::Cleared, MishitVerdict::Confirmed].repeat(OVERLAPS / 2) {
                second
                    .0
                    .verdict(second.1, Some(verdict))
                    .expect("a verdict");
            }
        });
    });
    let manifest = phone.manifest(&card);
    assert_eq!(manifest.club, Some(ClubId::PitchingWedge));
    assert_eq!(manifest.mishit, Some(StoredVerdict::Confirmed));
}

/// Every manifest write goes through one `manifest.tmp`, and the store's own lock is per instance, so two
/// verdicts with nothing else between them rename each other's file away. With the lock taken out, a
/// person's tap would be refused with `No such file or directory`: this failed 4 runs of 4 like that.
/// provisional: pasted copies.
#[test]
fn overlapping_verdicts_never_refuse_each_other() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889");
    thread::scope(|scope| {
        for _ in 0..8 {
            let (phone, card) = (&phone, &card);
            scope.spawn(move || {
                for verdict in
                    [MishitVerdict::Cleared, MishitVerdict::Confirmed].repeat(OVERLAPS / 2)
                {
                    phone.verdict(card, Some(verdict)).expect("a verdict");
                }
            });
        }
    });
    assert!(phone.manifest(&card).mishit.is_some());
}
