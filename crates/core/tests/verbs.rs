//! `golf-core run-shot --session` and `golf-core set-intent`, the verbs that write into a session.
//! [M35 P11]
//!
//! **Never on `data/`.** Each test builds a scratch tree from a hand-worked storage case
//! (`spec/vectors/storage/hand/`, M35 P9), with its `shot_analysis.json` files taken out so that the
//! verb is what writes them, plus a registered golfer, that golfer's bag (the storage family's
//! `stores/bag-reads`, a 7 iron at 34 degrees) and a shot store. The shots put in the store are two
//! committed shot cases' (`spec/vectors/shot/`), chosen because each needs exactly one of the two
//! inputs the shell resolves:
//!
//! - [`NO_AXIS`], `2026-08-23-4`'s shot: no spin and no spin axis printed, so it flies only with a
//!   loft and never asks for a handedness;
//! - [`AXIS`], `2026-08-10-1`'s: spin and spin axis printed, so it flies with no loft and its landing
//!   offline needs a handedness.
//!
//! That is what lets one walk show each gap note appended when the engine reached for the input, and
//! left off when it did not (`golf_core::session`'s doc).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use contracts::career::ExclusionReason;
use contracts::club::ClubId;
use contracts::golfer::Handedness;
use contracts::intent::{PracticeGoal, PracticeMode, TargetShape};
use contracts::shot::ShotData;
use contracts::shot_result::ShotResult;
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::unscored::UnscoredReason;
use contracts::Timestamp;
use golf_core::session::{
    intent_action, run_shot_session, set_intent, IntentAction, IntentTarget, ShotDirs,
};
use golf_core::{run_shot, ShotInput};
use serde_json::Value;
use storage::bag_store::BagStore;
use storage::corpus::{read_corpus, EngineVersions};
use storage::flight_inputs::{loft_for_club, LoftGap};
use storage::manifest::{load_manifest, manifest_path};
use storage::session_meta::load_session_meta;
use storage::shot_store::ShotStore;
use storage::state::{load_shot_analysis, shot_analysis_path, SHOT_ANALYSIS_NAME};

/// `2026-08-23-4`'s shot: needs a loft, never a handedness.
const NO_AXIS: &str = "no-loft.json";
/// `2026-08-10-1`'s shot: needs a handedness, never a loft.
const AXIS: &str = "no-handedness.json";

const SESSION: &str = "2026-08-20";

/// A scratch directory under the system temp dir, removed on drop. `reports.rs`' own, restated
/// because each file under `tests/` is a crate of its own and `tempfile` is not this crate's
/// dependency (the M36 plan's P13 finding 6).
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "golf-core-verbs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("a scratch directory");
        Scratch(path)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("parent directories");
        fs::write(&path, text).expect("a scratch file");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/vectors")
        .canonicalize()
        .expect("spec/vectors/ is committed")
}

fn read_vector(relative: &str) -> Value {
    let path = vectors_dir().join(relative);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

const GOLFER: &str = r#"{"player_id": "aaron", "display_name": "Aaron", "handedness": "right",
    "created_at": "2026-08-07T15:04:05.123456Z"}"#;

/// A hand case's sessions with every `shot_analysis.json` left out, golfer `aaron` (right-handed)
/// and the bag `stores/bag-reads` records for him, and an empty shot store.
fn tree(case: &str) -> (Scratch, ShotDirs, Value) {
    let vector = read_vector(&format!("storage/hand/{case}"));
    let scratch = Scratch::new();
    for (relative, text) in vector["input"]["files"].as_object().expect("input.files") {
        if !relative.ends_with(SHOT_ANALYSIS_NAME) {
            scratch.write(
                &format!("sessions/{relative}"),
                text.as_str().expect("a file's text"),
            );
        }
    }
    scratch.write("golfers/aaron.golfer.json", GOLFER);
    let bags = read_vector("storage/stores/bag-reads.json");
    let dirs = ShotDirs {
        sessions: scratch.0.join("sessions"),
        golfers: scratch.0.join("golfers"),
        shots: scratch.0.join("shots"),
    };
    fs::write(
        BagStore::new(&dirs.golfers).path_for("aaron"),
        bags["input"]["files"]["aaron.bag.json"]
            .as_str()
            .expect("aaron's bag"),
    )
    .expect("the bag");
    (scratch, dirs, vector)
}

/// A committed shot case's shot, as frozen Python's import would have stored it for `photo`.
fn shot_of(case: &str, photo: &str) -> ShotData {
    let mut shot: ShotData =
        serde_json::from_value(read_vector(&format!("shot/{case}"))["input"]["shot"].clone())
            .expect("a shot");
    shot.provenance
        .as_mut()
        .expect("a photo's shot has provenance")
        .image_sha256 = Some(photo.to_string());
    shot
}

fn put(dirs: &ShotDirs, photo: &str, case: &str) -> ShotData {
    let shot = shot_of(case, photo);
    ShotStore::new(&dirs.shots).put(&shot).expect("the store");
    shot
}

fn versions() -> EngineVersions {
    EngineVersions {
        installed: ANALYSIS_VERSION,
        comparable_from: COMPARABLE_FROM,
    }
}

fn now() -> Timestamp {
    "2026-10-10T12:30:00.250000Z".parse().expect("a timestamp")
}

fn swing_dir(dirs: &ShotDirs, swing: &str) -> PathBuf {
    dirs.sessions.join(SESSION).join(swing)
}

fn written(dirs: &ShotDirs, swing: &str) -> ShotResult {
    load_shot_analysis(&swing_dir(dirs, swing))
        .unwrap_or_else(|| panic!("{SESSION}/{swing}: no shot_analysis.json that reads back"))
}

fn every_shot_analysis(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == SHOT_ANALYSIS_NAME) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The line the verb prints for a result it wrote.
fn wrote_line(swing: &str, result: &ShotResult) -> String {
    let s = |n: usize| if n == 1 { "" } else { "s" };
    format!(
        "{SESSION}/{swing}: wrote shot_analysis.json (fundamentals): {} measurement{}, {} \
         unscored, {} note{}",
        result.measurements.len(),
        s(result.measurements.len()),
        result.unscored.len(),
        result.notes.len(),
        s(result.notes.len()),
    )
}

fn reasons(result: &ShotResult) -> Vec<UnscoredReason> {
    result.unscored.iter().map(|entry| entry.reason).collect()
}

/// **`run-shot --session` over `mixed-session`**: every photo entry with a stored parse is analysed
/// and written, every swing is skipped and gets no file, a photo with no parse is named with the
/// frozen import and skipped, and each written result is exactly the seam's answer for what the
/// stores resolve, with the gap notes the engine asked for and no others. A second run writes the
/// same bytes and prints the same lines, and the corpus then pools the written entries.
#[test]
fn run_shot_writes_each_photo_entry_and_no_swing_and_a_rerun_writes_the_same_bytes() {
    let (_scratch, dirs, _) = tree("mixed-session.json");
    let shot_2 = put(&dirs, "photo-20-2", NO_AXIS);
    let shot_4 = put(&dirs, "photo-20-4", AXIS);
    let shot_7 = put(&dirs, "photo-20-7", AXIS);
    let shot_8 = put(&dirs, "photo-20-8", NO_AXIS);

    let answer = run_shot_session(SESSION, None, &dirs, versions());
    assert_eq!((answer.stderr.as_str(), answer.code), ("", 0));

    // What the stores resolve: aaron is right-handed with a 7i at 34 degrees; 7 names no golfer, so
    // it has neither; brian (8) is not registered and has no bag.
    let seam = |swing: &str, shot: &ShotData, handedness, loft_deg, photo: &str| {
        run_shot(&ShotInput {
            swing_id: swing.to_string(),
            session_id: SESSION.to_string(),
            shot: shot.clone(),
            intent: None,
            club: Some(ClubId::SevenIron),
            handedness,
            loft_deg,
            photo_sha256: Some(photo.to_string()),
        })
        .expect("the seam analyses it")
    };
    let expected_2 = seam(
        "2",
        &shot_2,
        Some(Handedness::Right),
        Some(34.0),
        "photo-20-2",
    );
    let expected_4 = seam(
        "4",
        &shot_4,
        Some(Handedness::Right),
        Some(34.0),
        "photo-20-4",
    );
    let mut expected_7 = seam("7", &shot_7, None, None, "photo-20-7");
    let mut expected_8 = seam("8", &shot_8, None, None, "photo-20-8");

    // Both gaps are real on both entries, and the engine asks for one of them on each.
    let golfers = Some(dirs.golfers.as_path());
    assert_eq!(
        loft_for_club(None, Some(ClubId::SevenIron), golfers),
        Err(LoftGap::NoBagEntry)
    );
    assert_eq!(
        loft_for_club(Some("brian"), Some(ClubId::SevenIron), golfers),
        Err(LoftGap::NoBagEntry)
    );
    assert!(reasons(&expected_7).contains(&UnscoredReason::NoHandedness));
    assert!(!reasons(&expected_7).contains(&UnscoredReason::NoClubLoft));
    assert!(reasons(&expected_8).contains(&UnscoredReason::NoClubLoft));
    assert!(!reasons(&expected_8).contains(&UnscoredReason::NoHandedness));

    let got_7 = written(&dirs, "7");
    let got_8 = written(&dirs, "8");
    assert_eq!(got_7.notes.len(), 1, "{:?}", got_7.notes);
    assert!(got_7.notes[0]
        .starts_with("no golfer is attributed to this shot, so this shot's landing offline"));
    assert_eq!(got_8.notes.len(), 1, "{:?}", got_8.notes);
    assert!(got_8.notes[0].starts_with("club 7i is not in this golfer's bag"));
    expected_7.notes.extend(got_7.notes.clone());
    expected_8.notes.extend(got_8.notes.clone());

    for (swing, expected) in [
        ("2", &expected_2),
        ("4", &expected_4),
        ("7", &expected_7),
        ("8", &expected_8),
    ] {
        assert_eq!(&written(&dirs, swing), expected, "{SESSION}/{swing}");
    }
    assert!(expected_2.notes.is_empty() && expected_4.notes.is_empty());

    let swing_line = "skipped: a swing with a face-on clip, whose shot is analysed with it into \
                      analysis.json";
    let lines: Vec<&str> = answer.stdout.lines().collect();
    assert_eq!(lines.len(), 8, "{}", answer.stdout);
    assert_eq!(lines[0], format!("{SESSION}/1: {swing_line}"));
    assert_eq!(lines[1], wrote_line("2", &expected_2));
    assert_eq!(lines[2], format!("{SESSION}/3: {swing_line}"));
    assert_eq!(lines[3], wrote_line("4", &expected_4));
    assert_eq!(lines[4], format!("{SESSION}/5: {swing_line}"));
    let photo_6 = swing_dir(&dirs, "6").join("shot_screen.photo-20-6.jpg");
    assert_eq!(
        lines[5],
        format!(
            "{SESSION}/6: skipped: the shot store at {} holds no parse of shot photo photo-20-6; \
             frozen Python reads it with python scripts/import_shot_screens.py {} --session \
             {SESSION} --out {}, and then this verb analyses it",
            dirs.shots.display(),
            photo_6.display(),
            dirs.shots.display()
        )
    );
    assert_eq!(lines[6], wrote_line("7", &expected_7));
    assert_eq!(lines[7], wrote_line("8", &expected_8));

    let files = every_shot_analysis(&dirs.sessions);
    let wanted: Vec<PathBuf> = ["2", "4", "7", "8"]
        .iter()
        .map(|swing| shot_analysis_path(&swing_dir(&dirs, swing)))
        .collect();
    assert_eq!(
        files, wanted,
        "a file for each analysed photo entry and none else"
    );
    for swing in ["1", "3", "5", "6"] {
        assert!(!shot_analysis_path(&swing_dir(&dirs, swing)).exists());
    }

    let bytes: Vec<Vec<u8>> = files.iter().map(|f| fs::read(f).expect("bytes")).collect();
    let again = run_shot_session(SESSION, None, &dirs, versions());
    assert_eq!(again, answer, "a second run prints the same");
    for (file, before) in files.iter().zip(&bytes) {
        assert_eq!(
            &fs::read(file).expect("bytes"),
            before,
            "{}",
            file.display()
        );
    }

    // The corpus pools both of aaron's written entries, read from the file the verb wrote, and
    // names the one with no parse as not analysed, with the verb that would analyse it.
    let corpus = read_corpus(&dirs.sessions, "aaron", versions());
    for (swing, expected) in [("2", &expected_2), ("4", &expected_4)] {
        let entry = corpus
            .swings
            .iter()
            .find(|entry| entry.swing_id == swing)
            .unwrap_or_else(|| panic!("photo entry {swing} is pooled"));
        assert!(entry.analyzed && entry.face_on_sha256.is_none());
        assert_eq!(entry.measurements, expected.measurements, "{swing}");
    }
    let not_analyzed: Vec<&str> = corpus
        .excluded
        .iter()
        .filter(|entry| entry.reason != ExclusionReason::Unattributed)
        .map(|entry| entry.swing_id.as_str())
        .collect();
    assert_eq!(not_analyzed, ["6"]);
    assert!(corpus.excluded.iter().any(|entry| entry.swing_id == "6"
        && entry.reason == ExclusionReason::NotAnalyzed
        && entry.detail.contains("golf-core run-shot over its session")));
}

/// A photo entry the corpus counts as a duplicate is skipped with the corpus's own recorded
/// sentence and gets no file, whether the walk reaches it or `--swing` names it: the photo-only copy
/// of a swing's photo (decision 4), and the later of two photo entries holding one photo.
#[test]
fn a_photo_entry_the_corpus_counts_as_a_duplicate_is_skipped_with_its_sentence() {
    for case in [
        "photo-only-copy-of-a-swing.json",
        "photo-entries-one-photo.json",
    ] {
        let (_scratch, dirs, vector) = tree(case);
        let session = vector["input"]["files"]
            .as_object()
            .expect("files")
            .keys()
            .next()
            .and_then(|key| key.split('/').next())
            .expect("a session")
            .to_string();
        let store = storage::bundle_store::SwingBundleStore::new(&dirs.sessions);
        for manifest in store.get_session(&session) {
            if let Some(photo) = manifest.roles.get(&storage::manifest::Role::ShotScreen) {
                put(&dirs, &photo.content_sha256, NO_AXIS);
            }
        }
        let duplicates: Vec<(String, String)> = vector["expected"]["corpus"]["excluded"]
            .as_array()
            .expect("excluded")
            .iter()
            .filter(|entry| entry["reason"] == "duplicate")
            .map(|entry| {
                (
                    entry["swing_id"].as_str().expect("an id").to_string(),
                    entry["detail"].as_str().expect("a sentence").to_string(),
                )
            })
            .collect();
        assert_eq!(duplicates.len(), 1, "{case}");

        let answer = run_shot_session(&session, None, &dirs, versions());
        assert_eq!(answer.code, 0, "{case}: {}", answer.stderr);
        for (swing, detail) in &duplicates {
            let line = format!("{session}/{swing}: skipped: {detail}");
            assert!(
                answer.stdout.lines().any(|printed| printed == line),
                "{case}: {line}\n{}",
                answer.stdout
            );
            let named = run_shot_session(&session, Some(swing), &dirs, versions());
            assert_eq!((named.stdout, named.code), (format!("{line}\n"), 0));
            let dir = dirs.sessions.join(&session).join(swing);
            assert!(!shot_analysis_path(&dir).exists(), "{case}: {swing}");
        }
        let written_count = every_shot_analysis(&dirs.sessions).len();
        let photo_entries = store
            .get_session(&session)
            .iter()
            .filter(|m| !m.roles.contains_key(&storage::manifest::Role::FaceOn))
            .count();
        assert_eq!(written_count, photo_entries - duplicates.len(), "{case}");
    }
}

/// The refusals exit 2 and write nothing; a failed analysis is named on stderr, exits 1 and does
/// not stop the walk; a manifest with no photo is skipped; an empty session is said to be one.
#[test]
fn the_refusals_write_nothing_and_a_failed_entry_does_not_stop_the_walk() {
    let (_scratch, dirs, _) = tree("mixed-session.json");
    put(&dirs, "photo-20-2", NO_AXIS);
    let mut undeclared = shot_of(NO_AXIS, "photo-20-6");
    undeclared.provenance.as_mut().expect("provenance").device = "no_such_device".into();
    ShotStore::new(&dirs.shots)
        .put(&undeclared)
        .expect("the store");
    let manifest_3 = fs::read_to_string(manifest_path(&swing_dir(&dirs, "3"))).expect("3");
    let mut no_photo: Value = serde_json::from_str(&manifest_3).expect("JSON");
    no_photo["swing_id"] = "9".into();
    no_photo["roles"] = serde_json::json!({});
    fs::create_dir_all(swing_dir(&dirs, "9")).expect("9");
    fs::write(
        manifest_path(&swing_dir(&dirs, "9")),
        serde_json::to_string_pretty(&no_photo).expect("JSON"),
    )
    .expect("9's manifest");

    for (session, swing, says) in [
        (
            "2026-01-01",
            None,
            "golf-core run-shot: no session 2026-01-01 in ",
        ),
        (
            SESSION,
            Some("1"),
            "golf-core run-shot: 2026-08-20/1 is a swing with a face-on clip",
        ),
        (
            SESSION,
            Some("99"),
            "golf-core run-shot: no swing 2026-08-20/99\n",
        ),
        (
            SESSION,
            Some("9"),
            "golf-core run-shot: 2026-08-20/9 has no shot photo, so there is no shot to analyse\n",
        ),
    ] {
        let refused = run_shot_session(session, swing, &dirs, versions());
        assert_eq!(refused.code, 2, "{swing:?}");
        assert!(refused.stdout.is_empty(), "{swing:?}");
        assert!(refused.stderr.starts_with(says), "{}", refused.stderr);
    }
    assert!(every_shot_analysis(&dirs.sessions).is_empty());
    assert!(!dirs.sessions.join("2026-01-01").exists());

    let walked = run_shot_session(SESSION, None, &dirs, versions());
    assert_eq!(walked.code, 1);
    assert!(
        walked
            .stderr
            .starts_with("golf-core run-shot: 2026-08-20/6: not analysed: ")
            && walked.stderr.contains("no_such_device"),
        "{}",
        walked.stderr
    );
    assert!(walked
        .stdout
        .contains("2026-08-20/9: skipped: no shot photo, so there is no shot to analyse\n"));
    assert!(walked
        .stdout
        .contains("2026-08-20/2: wrote shot_analysis.json"));
    assert!(shot_analysis_path(&swing_dir(&dirs, "2")).exists());
    assert!(!shot_analysis_path(&swing_dir(&dirs, "6")).exists());

    fs::create_dir_all(dirs.sessions.join("2026-09-01")).expect("an empty session");
    let empty = run_shot_session("2026-09-01", None, &dirs, versions());
    assert_eq!(
        (empty.stdout.as_str(), empty.code),
        ("2026-09-01: no swings\n", 0)
    );
}

fn action(target: IntentTarget, goal: Option<PracticeGoal>) -> IntentAction {
    IntentAction {
        session_id: SESSION.to_string(),
        target,
        goal,
    }
}

fn goal(mode: PracticeMode, target_shape: Option<TargetShape>) -> PracticeGoal {
    PracticeGoal {
        mode,
        target_shape,
        ..PracticeGoal::default()
    }
}

/// The manifest's recorded mode is one of the members the committed manifest schema lists, the
/// schema frozen Python's `PracticeGoal` was exported into: what `flag-mishit`'s test checks of a
/// verdict, for the intent.
fn assert_the_schema_names_the_mode(written: &Value) {
    let schema: Value = serde_json::from_str(
        &fs::read_to_string(vectors_dir().join("../schemas/swing_manifest.schema.json"))
            .expect("the committed manifest schema"),
    )
    .expect("the schema is JSON");
    let members = schema["$defs"]["PracticeMode"]["enum"]
        .as_array()
        .expect("PracticeMode's members");
    assert!(members.contains(&written["intent"]["mode"]), "{written}");
}

/// **`set-intent`**: a goal on one swing, overwritten, then cleared to no key at all; the session's
/// cursor set and cleared; a missing swing or session refused with nothing written or created; and
/// only the one file each write names is touched.
#[test]
fn set_intent_stamps_a_swing_and_the_session_default_and_clears_both() {
    let (_scratch, dirs, vector) = tree("mixed-session.json");
    let untouched: Vec<(String, String)> = vector["input"]["files"]
        .as_object()
        .expect("files")
        .iter()
        .filter(|(relative, _)| {
            relative.as_str() != "2026-08-20/2/manifest.json"
                && !relative.ends_with(SHOT_ANALYSIS_NAME)
        })
        .map(|(relative, text)| (relative.clone(), text.as_str().expect("text").to_string()))
        .collect();
    let manifest_2 = manifest_path(&swing_dir(&dirs, "2"));
    let raw = || -> Value {
        serde_json::from_str(&fs::read_to_string(&manifest_2).expect("2")).expect("JSON")
    };

    for (goal, said) in [
        (
            goal(PracticeMode::Drill, None),
            "= drill, which no club or player stat counts",
        ),
        (
            goal(PracticeMode::ShotShaping, Some(TargetShape::Fade)),
            "= shot_shaping (fade)",
        ),
    ] {
        let set = set_intent(
            &action(IntentTarget::Swing("2".into()), Some(goal.clone())),
            &dirs.sessions,
            now(),
        );
        assert_eq!(
            (set.stdout.as_str(), set.code),
            (format!("{SESSION}/2: intent {said}\n").as_str(), 0)
        );
        let read = load_manifest(&manifest_2).expect("the manifest reads back");
        assert_eq!(read.intent, Some(goal));
        assert_eq!(read.updated_at.to_string(), now().to_string());
        assert_the_schema_names_the_mode(&raw());
    }
    let cleared = set_intent(
        &action(IntentTarget::Swing("2".into()), None),
        &dirs.sessions,
        now(),
    );
    assert_eq!(
        cleared.stdout,
        format!("{SESSION}/2: intent cleared, so it reads as fundamentals\n")
    );
    assert_eq!(load_manifest(&manifest_2).expect("2").intent, None);
    assert!(
        raw().get("intent").is_none(),
        "a cleared intent writes no key"
    );

    let session_dir = dirs.sessions.join(SESSION);
    let drill = goal(PracticeMode::Drill, None);
    let cursor = set_intent(
        &action(IntentTarget::SessionDefault, Some(drill.clone())),
        &dirs.sessions,
        now(),
    );
    assert_eq!(cursor.code, 0);
    assert!(cursor
        .stdout
        .starts_with("2026-08-20: session intent = drill, which no club or player stat counts"));
    assert_eq!(load_session_meta(&session_dir).intent, Some(drill));
    let cleared = set_intent(
        &action(IntentTarget::SessionDefault, None),
        &dirs.sessions,
        now(),
    );
    assert_eq!(cleared.stdout, "2026-08-20: session intent cleared\n");
    assert_eq!(load_session_meta(&session_dir).intent, None);
    let meta: Value = serde_json::from_str(
        &fs::read_to_string(session_dir.join("session.json")).expect("session.json"),
    )
    .expect("JSON");
    assert!(meta.get("intent").is_none());

    let no_swing = set_intent(
        &action(
            IntentTarget::Swing("99".into()),
            Some(goal(PracticeMode::Drill, None)),
        ),
        &dirs.sessions,
        now(),
    );
    assert_eq!(
        (no_swing.stderr.as_str(), no_swing.code),
        ("golf-core set-intent: no swing 2026-08-20/99\n", 2)
    );
    let mut elsewhere = action(
        IntentTarget::SessionDefault,
        Some(goal(PracticeMode::Drill, None)),
    );
    elsewhere.session_id = "2026-01-01".into();
    let no_session = set_intent(&elsewhere, &dirs.sessions, now());
    assert_eq!(no_session.code, 2);
    assert!(no_session.stderr.contains("no session 2026-01-01"));
    assert!(
        !dirs.sessions.join("2026-01-01").exists(),
        "--default created a session"
    );
    assert!(!swing_dir(&dirs, "99").exists());

    for (relative, text) in &untouched {
        let now = fs::read_to_string(dirs.sessions.join(relative)).expect("still there");
        assert_eq!(&now, text, "{relative} was written");
    }
    // And the refusals call 5 makes, through the same decision the binary takes.
    assert!(intent_action(
        Some(SESSION.into()),
        Some("2".into()),
        false,
        Some("drill".into()),
        Some("draw".into()),
        false
    )
    .is_err());
    assert!(intent_action(
        Some(SESSION.into()),
        Some("2".into()),
        false,
        Some("shot_shaping".into()),
        None,
        false
    )
    .is_err());
}

fn golf_core(args: &[&str], dirs: &ShotDirs) -> Output {
    Command::new(env!("CARGO_BIN_EXE_golf-core"))
        .args(args)
        .env_remove("GOLF_SESSIONS_DIR")
        .env_remove("GOLF_GOLFERS_DIR")
        .env("GOLF_SHOTS_DIR", &dirs.shots)
        .output()
        .expect("golf-core runs")
}

/// **A drill end to end, through the binary**: `set-intent` records it, `run-shot --session`
/// analyses and writes it like any shot, and the corpus leaves it out as `untracked` while the
/// tracked entry beside it pools (the M35 plan's decision 1). The shot store is found through
/// `GOLF_SHOTS_DIR` here, and the other two directories through their flags.
#[test]
fn a_drill_set_by_set_intent_is_analysed_by_run_shot_and_absent_from_the_corpus() {
    let (_scratch, dirs, _) = tree("mixed-session.json");
    put(&dirs, "photo-20-2", NO_AXIS);
    put(&dirs, "photo-20-4", AXIS);
    let sessions = format!("--sessions-dir={}", dirs.sessions.display());
    let golfers = dirs.golfers.display().to_string();

    let set = golf_core(
        &[
            "set-intent",
            "--session",
            SESSION,
            "--swing",
            "2",
            "--mode",
            "drill",
            sessions.as_str(),
        ],
        &dirs,
    );
    assert_eq!(
        set.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&set.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&set.stdout),
        "2026-08-20/2: intent = drill, which no club or player stat counts\n"
    );

    let ran = golf_core(
        &[
            "run-shot",
            "--session",
            SESSION,
            sessions.as_str(),
            "--golfers-dir",
            golfers.as_str(),
        ],
        &dirs,
    );
    let stdout = String::from_utf8_lossy(&ran.stdout);
    assert_eq!(
        ran.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert!(
        stdout.contains(
            "2026-08-20/2: wrote shot_analysis.json (drill, a drill that no club or player stat \
             counts): "
        ),
        "{stdout}"
    );
    let drill = written(&dirs, "2");
    assert_eq!(drill.intent.mode, PracticeMode::Drill);
    assert!(!drill.measurements.is_empty(), "a drill is analysed");

    let corpus = read_corpus(&dirs.sessions, "aaron", versions());
    assert!(corpus.swings.iter().all(|entry| entry.swing_id != "2"));
    assert!(corpus
        .swings
        .iter()
        .any(|entry| entry.swing_id == "4" && entry.analyzed));
    assert!(corpus
        .excluded
        .iter()
        .any(|entry| entry.swing_id == "2" && entry.reason == ExclusionReason::Untracked));
}

/// The flags each verb refuses exit 2 with the usage on stderr and nothing on stdout, and write
/// nothing; `--help` prints the usage and exits 0.
#[test]
fn the_binary_refuses_what_argparse_would() {
    let (_scratch, dirs, _) = tree("mixed-session.json");
    let sessions = format!("--sessions-dir={}", dirs.sessions.display());
    for args in [
        &[
            "set-intent",
            "--session",
            SESSION,
            "--swing",
            "2",
            "--mode",
            "drill",
            "--shape",
            "draw",
        ][..],
        &[
            "set-intent",
            "--session",
            SESSION,
            "--swing",
            "2",
            "--mode",
            "shot_shaping",
        ][..],
        &[
            "set-intent",
            "--session",
            SESSION,
            "--swing",
            "2",
            "--default",
            "--clear",
        ][..],
        &[
            "set-intent",
            "--session",
            SESSION,
            "--default=yes",
            "--clear",
        ][..],
        &["set-intent", "--swing", "2", "--clear"][..],
        &["set-intent", "--session", SESSION, "--swing", "2", "--mode"][..],
        &["run-shot", "--swing", "2"][..],
        &["run-shot", "--session"][..],
        &["run-shot", "--session", SESSION, "--colour"][..],
    ] {
        // The directory first, so a flag left without its value is the case's last argument.
        let mut with_dir = vec![args[0], sessions.as_str()];
        with_dir.extend_from_slice(&args[1..]);
        let refused = golf_core(&with_dir, &dirs);
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert_eq!(refused.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(refused.stdout.is_empty(), "{args:?}");
        assert!(stderr.contains("usage:"), "{args:?}: {stderr}");
    }
    assert!(every_shot_analysis(&dirs.sessions).is_empty());
    assert_eq!(
        load_manifest(&manifest_path(&swing_dir(&dirs, "2")))
            .expect("2")
            .intent,
        None
    );
    for verb in ["run-shot", "set-intent"] {
        let help = golf_core(&[verb, "--help"], &dirs);
        assert_eq!(help.status.code(), Some(0));
        assert!(String::from_utf8_lossy(&help.stdout).contains("golf-core set-intent --session"));
    }
}
