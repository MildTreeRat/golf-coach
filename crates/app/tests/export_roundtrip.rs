//! `export_sessions` and `export_status`, held to what the real `golf-lab import-export` makes of the
//! zip. [M38 P13]
//!
//! **Half of the milestone's exit, on the desk**: a store built by `record_photo`, exported, and imported
//! into empty scratch directories, reads as the phone's under `read_corpus`. The other half is P32's,
//! on a session's real export. Nothing is built by hand on either side: the phone's files come from
//! the facade's own functions, and the lab's from the verb's, so a word either one changes is a failure
//! here. `golf-lab` is a dev-dependency for this file alone, and never ships (`ort_reach` and
//! `crates/lab/tests/edges.rs`).
//!
//! **Everything read off the six captures here is provisional: pasted copies** (rule 12). They are
//! chat-pasted JPEGs and not the originals, by M33's decision of 2026-11-04. Those files are read and
//! never written (rule 5), and each is copied into the scratch root first.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::api::export::{export_sessions, export_status, ExportStatus, ExportSummary};
use app::api::history::{
    delete_shot, set_shot_club, set_shot_intent, set_shot_mishit, MishitVerdict,
};
use app::api::record::{
    log_event, log_read, record_photo, AppEvent, EventKind, PhotoIn, ReadLog, ReadSource,
    RecordOutcome, RecordStatus, ShotCard,
};
use app::api::review::{correct_shot, FieldEdit};
use app::api::setup::{
    create_golfer, get_state, init_state, set_bag_slot, set_state, AppState, Handedness,
    PracticeMode,
};
use app::{engine_versions, Dirs, DEVICE};
use contracts::career::CareerCorpus;
use contracts::golfer::Handedness as StoredHandedness;
use contracts::shot::ShotData;
use contracts::Timestamp;
use golf_lab::import_export::{import_export, Request};
use golf_lab::shot::{shot_for, PhotoReader, Prepared, ShotOptions};
use screen::profile::DeviceProfile;
use serde_json::Value;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::read_corpus;
use storage::golfer_store::GolferStore;
use storage::manifest::{Role, SwingManifest};
use storage::shot_store::{hash_image, ShotStore};
use tempfile::TempDir;
use zip::extra_fields::ExtraField;
use zip::{CompressionMethod, ZipArchive};

/// M33's `fs-still` set, each with a time of its own and over two dates, so the corpus has an order to
/// keep and the export two sessions to carry.
const SHOTS: [(&str, &str); 6] = [
    ("IMG_2889", "2026-11-04T20:42:00-05:00"),
    ("IMG_2891", "2026-11-04T20:44:00-05:00"),
    ("IMG_2892", "2026-11-04T20:46:00-05:00"),
    ("IMG_2893", "2026-11-04T20:48:00-05:00"),
    ("IMG_2894", "2026-11-05T09:00:00-05:00"),
    ("2026-10-07-1", "2026-11-05T09:02:00-05:00"),
];
const FIRST_DATE: &str = "2026-11-04-phone-a1b2";
const RECORDED: &str = "2026-11-05T10:00:00-05:00";
const EXPORTED: &str = "2026-11-05T11:00:00-05:00";
const LATER: &str = "2026-11-05T12:00:00-05:00";

fn spike() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spikes/vision-ocr")
}

fn raw_boxes(id: &str) -> String {
    let path = spike().join("boxes/C0/raw").join(format!("{id}.json"));
    fs::read_to_string(&path).unwrap_or_else(|failed| panic!("{}: {failed}", path.display()))
}

fn at(text: &str) -> Timestamp {
    Timestamp::parse(text).expect("a time")
}

/// A time as a file holds it, for a time in whole seconds.
fn instant(text: &str) -> SystemTime {
    let micros = at(text).utc_micros();
    UNIX_EPOCH + Duration::from_micros(u64::try_from(micros).expect("after 1970"))
}

fn modified(path: &Path) -> SystemTime {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .unwrap_or_else(|failed| panic!("{}: {failed}", path.display()))
}

/// Every file below `root`, by its `/`-joined path, as its bytes. A directory is not listed.
fn files_below(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, into: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                walk(root, &path, into);
            } else {
                let names: Vec<String> = path
                    .strip_prefix(root)
                    .expect("below the root")
                    .components()
                    .map(|name| name.as_os_str().to_string_lossy().into_owned())
                    .collect();
                into.insert(names.join("/"), fs::read(&path).expect("a file"));
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

/// A scratch phone with a golfer and a club chosen, as first-run setup leaves it.
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
            RECORDED.to_string(),
        )
        .expect("a golfer");
        let phone = Phone { dir, root };
        let mut state = get_state(phone.root.clone());
        state.player_id = Some("aaron".to_string());
        state.club = Some("7i".to_string());
        set_state(phone.root.clone(), state).expect("a state");
        phone
    }

    fn with(shots: &[(&str, &str)]) -> Self {
        let phone = Phone::new();
        for (id, taken) in shots {
            phone.feed(id, taken);
        }
        phone
    }

    fn dirs(&self) -> Dirs {
        Dirs::of(&self.root)
    }

    /// One capture and its committed boxes, as Dart passes them.
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
            now: RECORDED.to_string(),
            challenge_call: None,
        })
        .expect("a recorded photo")
    }

    fn feed(&self, id: &str, taken: &str) -> ShotCard {
        self.outcome(id, raw_boxes(id), taken).card.expect("a card")
    }

    /// A read of nothing, which keeps the boxes and the photo as diagnostics and no entry (C13).
    fn fail(&self, id: &str, taken: &str) -> String {
        let failed = self.outcome(id, "[]".to_string(), taken);
        assert_eq!(failed.status, RecordStatus::Failed);
        failed.photo_sha256
    }

    fn log(&self, status: RecordStatus) {
        log_read(
            self.root.clone(),
            ReadLog {
                at: RECORDED.to_string(),
                photo_sha256: "abc".to_string(),
                source: ReadSource::Camera,
                status,
                boxes: 42,
                ms_vision: 1260,
                ms_rust: 12,
                ms_total: 1400,
            },
        )
        .expect("a line");
    }

    /// One line of the events' log (D16), as P30b's model writes one.
    fn event(&self, kind: EventKind, detail: &str) {
        log_event(
            self.root.clone(),
            AppEvent {
                at: RECORDED.to_string(),
                kind,
                detail: detail.to_string(),
            },
        )
        .expect("an event");
    }

    fn zip_path(&self) -> PathBuf {
        self.dir.path().join("tmp").join("export.zip")
    }

    fn export_at(&self, now: &str) -> ExportSummary {
        export_sessions(
            self.root.clone(),
            self.zip_path().display().to_string(),
            now.to_string(),
        )
        .expect("an export")
    }

    fn export(&self) -> PathBuf {
        self.export_at(EXPORTED);
        self.zip_path()
    }

    fn corpus(&self) -> CareerCorpus {
        read_corpus(&self.dirs().sessions, "aaron", engine_versions())
    }

    /// What a lab's stores should hold: the three trees and nothing of `.app/`.
    fn stored(&self) -> BTreeMap<String, Vec<u8>> {
        let mut files = files_below(Path::new(&self.root));
        files.retain(|name, _| !name.starts_with(".app/"));
        files
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

    fn delete(&self, card: &ShotCard) {
        delete_shot(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
        )
        .expect("a delete");
    }

    fn status(&self) -> ExportStatus {
        export_status(self.root.clone())
    }

    /// What P27 does once the share has gone.
    fn save_export(&self, summary: &ExportSummary) {
        let state = AppState {
            last_export_at: Some(summary.exported_at.clone()),
            last_export_shots: Some(summary.shots),
            ..get_state(self.root.clone())
        };
        set_state(self.root.clone(), state).expect("a state");
    }
}

/// A scratch lab, its stores empty unless a test puts something there.
struct Lab {
    dir: TempDir,
    dirs: Dirs,
}

/// What one run of the verb said.
struct Imported {
    code: u8,
    out: String,
    err: String,
}

impl Imported {
    fn clean(&self) {
        assert_eq!((self.code, self.err.as_str()), (0, ""), "{}", self.out);
    }

    fn says(&self, line: &str) {
        assert!(
            self.out.contains(&format!("\n  {line}\n")),
            "no line {line:?} in:\n{}",
            self.out
        );
    }

    fn tally(&self, copied: usize, identical: usize, left: usize, ignored: usize) {
        let tally = format!(
            "\n{copied} file(s) copied; {identical} identical, {left} left as stored, {ignored} \
             ignored.\n"
        );
        assert!(self.out.contains(&tally), "no {tally:?} in:\n{}", self.out);
    }
}

impl Lab {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let dirs = Dirs::of(dir.path().join("processed"));
        Lab { dir, dirs }
    }

    fn import(&self, export: &Path) -> Imported {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = import_export(
            &Request {
                export: export.to_path_buf(),
                sessions_dir: self.dirs.sessions.clone(),
                shots_dir: self.dirs.shots.clone(),
                golfers_dir: self.dirs.golfers.clone(),
                dry_run: false,
            },
            &mut out,
            &mut err,
        )
        .expect("a report");
        Imported {
            code,
            out: String::from_utf8(out).expect("text"),
            err: String::from_utf8(err).expect("text"),
        }
    }

    fn corpus(&self) -> CareerCorpus {
        read_corpus(&self.dirs.sessions, "aaron", engine_versions())
    }

    fn files(&self) -> BTreeMap<String, Vec<u8>> {
        files_below(&self.dir.path().join("processed"))
    }

    fn entry(&self, card: &ShotCard) -> SwingManifest {
        SwingBundleStore::new(&self.dirs.sessions)
            .get_swing(&card.session_id, &card.swing_id)
            .expect("the entry at the lab")
    }

    fn photo_path(&self, card: &ShotCard) -> PathBuf {
        let entry = self.entry(card);
        self.dirs
            .sessions
            .join(&card.session_id)
            .join(&card.swing_id)
            .join(&entry.roles[&Role::ShotScreen].filename)
    }

    fn shot(&self, digest: &str) -> ShotData {
        ShotStore::new(&self.dirs.shots)
            .get(digest)
            .expect("a shot that reads")
            .expect("the shot at the lab")
    }
}

/// The zip's entries in the order they are written.
fn entry_names(zip: &Path) -> Vec<String> {
    let mut archive = ZipArchive::new(fs::File::open(zip).expect("the zip")).expect("a zip");
    (0..archive.len())
        .map(|index| archive.by_index_raw(index).expect("an entry").name().into())
        .collect()
}

fn typed(field: &str, printed: &str) -> FieldEdit {
    FieldEdit {
        field: field.to_string(),
        printed: Some(printed.to_string()),
        letter: None,
    }
}

fn digest_of(id: &str) -> String {
    hash_image(&fs::read(spike().join("captures").join(format!("{id}.jpg"))).expect("a capture"))
}

/// The two lines C16's limit and P10's retag print for a shot the lab already holds.
fn duplicate_line(card: &ShotCard) -> String {
    let (session, swing) = (&card.session_id, &card.swing_id);
    format!(
        "duplicate sessions/{session}/{swing}: session {session} holds its photo already, in \
         swing {swing}, and that swing stays"
    )
}

fn kept_shot_line(digest: &str) -> String {
    format!(
        "kept      shots/{digest}.shot.json: the store holds a shot of this photo already, which \
         reads differently, and it stays"
    )
}

// ------------------------------------------------------------------------------- the exit's half

/// The milestone's exit, on the desk. Equal corpora prove little if both are empty or hold nothing, so
/// it also says what they hold, and that the three trees are the phone's bytes. provisional: pasted
/// copies.
#[test]
fn six_captures_exported_and_imported_read_as_the_phones_corpus() {
    let phone = Phone::with(&SHOTS);
    let lab = Lab::new();
    assert!(lab.corpus().swings.is_empty(), "the lab starts empty");

    lab.import(&phone.export()).clean();

    let corpus = phone.corpus();
    assert_eq!(lab.corpus(), corpus);
    assert_eq!(corpus.swings.len(), SHOTS.len());
    assert_eq!(corpus.excluded.len(), 0);
    assert!(corpus.swings.iter().all(|swing| swing.shot.is_some()));
    assert_eq!(lab.files(), phone.stored());
}

/// The summary is what Settings saves, so each number is held to the zip it describes.
#[test]
fn the_summary_says_what_the_zip_holds() {
    let phone = Phone::with(&SHOTS[..2]);
    phone.log(RecordStatus::Read);
    let summary = phone.export_at(EXPORTED);

    assert_eq!(summary.path, phone.zip_path().display().to_string());
    assert_eq!(summary.exported_at, EXPORTED);
    assert_eq!(summary.shots, 2);
    assert_eq!(
        summary.files,
        i64::try_from(entry_names(&phone.zip_path()).len()).unwrap()
    );
    assert_eq!(
        summary.bytes,
        i64::try_from(fs::metadata(phone.zip_path()).unwrap().len()).unwrap()
    );
}

// ---------------------------------------------------------------------------- what the zip holds

/// C16's list exactly, sorted, with the failed read's two diagnostics in it, and since P30b the events'
/// log (D16). **`state.json` is not in it, nor is the deleted numbers' file**, and since `.app/` is
/// never walked, nor is anything a later phase puts there unnamed. provisional: pasted copies.
#[test]
fn the_zip_holds_c16s_list_sorted_and_nothing_else() {
    let phone = Phone::new();
    let stray = phone.feed("IMG_2891", SHOTS[1].1);
    phone.delete(&stray);
    let card = phone.feed("IMG_2889", SHOTS[0].1);
    let failed = phone.fail("IMG_2892", SHOTS[2].1);
    phone.log(RecordStatus::Read);
    phone.event(EventKind::ShotDeleted, &stray.swing_id);
    fs::write(phone.dirs().app.join("stray.jsonl"), "{}\n").expect("an unnamed file");
    set_bag_slot(
        phone.root.clone(),
        "aaron".to_string(),
        "7i".to_string(),
        Some(34.0),
        RECORDED.to_string(),
    )
    .expect("a slot");
    assert!(phone.dirs().app.join("state.json").is_file());
    assert!(phone.dirs().app.join("deleted.json").is_file());

    let names = entry_names(&phone.export());

    let shot = digest_of("IMG_2889");
    let swing = format!("sessions/{}/{}", card.session_id, card.swing_id);
    let photo = &fs::read_dir(Path::new(&phone.root).join(&swing))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .find(|name| name.ends_with(".jpg"))
        .expect("the stored photo");
    assert_eq!(
        names,
        [
            format!(".app/boxes/{}.json", digest_of("IMG_2891")),
            format!(".app/boxes/{shot}.json"),
            format!(".app/boxes/{failed}.json"),
            ".app/events.jsonl".to_string(),
            format!(".app/failed/{failed}.jpg"),
            ".app/reads.jsonl".to_string(),
            "golfers/aaron.bag.json".to_string(),
            "golfers/aaron.golfer.json".to_string(),
            format!("{swing}/manifest.json"),
            format!("{swing}/{photo}"),
            format!("{swing}/shot_analysis.json"),
            format!("shots/{shot}.shot.json"),
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>(),
        "sorted, and every name accounted for"
    );
}

/// P3 finding 6: a successful open leaves an empty `sessions/.incoming/`. A directory is no entry, so it
/// lands nowhere and counts as nothing. What a crash leaves, in each place one can, stays out as well.
/// provisional: pasted copies.
#[test]
fn an_empty_incoming_directory_and_what_a_crash_left_go_nowhere() {
    let phone = Phone::with(&SHOTS[..1]);
    let dirs = phone.dirs();
    let incoming = dirs.sessions.join(".incoming");
    assert_eq!(fs::read_dir(&incoming).expect("left behind").count(), 0);
    let clean = entry_names(&phone.export());
    assert!(
        clean
            .iter()
            .all(|name| !name.ends_with('/') && !name.contains("incoming")),
        "{clean:?}"
    );

    let lab = Lab::new();
    let imported = lab.import(&phone.zip_path());
    imported.clean();
    let ignored = clean
        .iter()
        .filter(|name| name.starts_with(".app/"))
        .count();
    imported.tally(clean.len() - ignored, 0, 0, ignored);

    for planted in [
        incoming.join("abc.123.part"),
        dirs.sessions
            .join(FIRST_DATE)
            .join("1")
            .join("manifest.tmp"),
        dirs.sessions.join(".DS_Store"),
        dirs.shots.join("abc.tmp"),
        dirs.golfers.join("aaron.tmp"),
        dirs.app.join("boxes").join("abc.tmp"),
        dirs.app.join("state.tmp"),
        dirs.app.join("deleted.tmp"),
    ] {
        fs::write(planted, b"left by a crash").expect("planted");
    }
    assert_eq!(entry_names(&phone.export()), clean);
}

/// The two rules C16 sets for an entry, read back off the real archive, each file's time among them.
/// provisional: pasted copies.
#[test]
fn json_is_deflated_photos_are_stored_and_every_entry_carries_its_files_time() {
    let phone = Phone::with(&SHOTS[..2]);
    phone.fail("IMG_2892", SHOTS[2].1);
    phone.log(RecordStatus::Read);
    let zip = phone.export();

    let mut archive = ZipArchive::new(fs::File::open(&zip).unwrap()).unwrap();
    let (mut deflated, mut stored) = (0, 0);
    for index in 0..archive.len() {
        let entry = archive.by_index_raw(index).unwrap();
        let name = entry.name().to_string();
        if name.ends_with(".json") || name.ends_with(".jsonl") {
            assert_eq!(entry.compression(), CompressionMethod::Deflated, "{name}");
            deflated += 1;
        } else {
            assert_eq!(entry.compression(), CompressionMethod::Stored, "{name}");
            stored += 1;
        }
        let seconds = entry.extra_data_fields().find_map(|field| match field {
            ExtraField::ExtendedTimestamp(stamp) => stamp.mod_time(),
            ExtraField::Ntfs(_) => None,
        });
        let file = modified(&Path::new(&phone.root).join(&name));
        let expected = file.duration_since(UNIX_EPOCH).unwrap().as_secs();
        assert_eq!(seconds.map(u64::from), Some(expected), "{name}");
    }
    assert!(deflated > 0);
    assert_eq!(stored, 3, "two stored photos and the failed one");
}

/// What a person re-exporting would see if the zip moved with the clock, and what a comparison of two
/// exports needs: `now` reaches the answer and no byte of the file.
#[test]
fn exporting_an_unchanged_store_twice_writes_the_same_bytes() {
    let phone = Phone::with(&SHOTS[..2]);
    let first = fs::read(phone.export()).unwrap();
    phone.export_at(LATER);
    assert_eq!(fs::read(phone.zip_path()).unwrap(), first);
}

/// A photo's time crosses as the file's, so a shot's stays its shot's at the lab (C9, C16). A manifest's
/// crosses too, by the same field. provisional: pasted copies.
#[test]
fn a_photo_keeps_its_modification_time_at_the_lab() {
    let phone = Phone::new();
    let cards: Vec<ShotCard> = SHOTS
        .iter()
        .map(|(id, taken)| phone.feed(id, taken))
        .collect();
    let lab = Lab::new();
    lab.import(&phone.export()).clean();

    for (card, (id, taken)) in cards.iter().zip(SHOTS) {
        assert_eq!(modified(&lab.photo_path(card)), instant(taken), "{id}");
    }
    let manifest = |root: &Path| {
        root.join(&cards[0].session_id)
            .join(&cards[0].swing_id)
            .join("manifest.json")
    };
    let phones = modified(&manifest(&phone.dirs().sessions));
    let seconds = phones.duration_since(UNIX_EPOCH).unwrap().as_secs();
    assert_eq!(
        modified(&manifest(&lab.dirs.sessions)),
        UNIX_EPOCH + Duration::from_secs(seconds)
    );
}

/// The field holds whole seconds, so that is what crosses, and the corpus, which reads no file's time,
/// still agrees. **For P20**: Swift's time is free to carry a fraction, and this is what it costs.
/// provisional: pasted copies.
#[test]
fn a_fraction_of_a_second_is_lost_on_the_file_at_the_lab_and_nowhere_else() {
    let phone = Phone::new();
    let card = phone.feed("IMG_2889", "2026-11-04T20:42:00.750-05:00");
    let lab = Lab::new();
    lab.import(&phone.export()).clean();

    assert_eq!(
        modified(&lab.photo_path(&card)),
        instant("2026-11-04T20:42:00-05:00")
    );
    assert_eq!(lab.corpus(), phone.corpus());
}

// ----------------------------------------------------------------------------- what the lab makes

/// C7: the diagnostics travel, the lab counts them, and its stores never hold them, the events' log
/// (P30b) among them. provisional: pasted copies.
#[test]
fn the_apps_files_are_counted_as_ignored_and_copied_nowhere() {
    let phone = Phone::with(&SHOTS[..2]);
    phone.fail("IMG_2892", SHOTS[2].1);
    phone.log(RecordStatus::Read);
    phone.event(EventKind::AppStart, "ready");
    let names = entry_names(&phone.export());
    let ignored = names
        .iter()
        .filter(|name| name.starts_with(".app/"))
        .count();
    assert_eq!(
        ignored, 6,
        "three boxes, a failed photo, the reads and the events"
    );

    let lab = Lab::new();
    let imported = lab.import(&phone.zip_path());
    imported.clean();
    imported.tally(names.len() - ignored, 0, 0, ignored);

    let mut top: Vec<String> = fs::read_dir(lab.dir.path().join("processed"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    top.sort();
    assert_eq!(top, ["golfers", "sessions", "shots"]);
    assert!(
        lab.files().keys().all(|name| !name.contains(".app")
            && !name.contains("reads.jsonl")
            && !name.contains("events.jsonl")
            && !name.contains("boxes")),
        "{:?}",
        lab.files().keys()
    );
}

#[test]
fn a_re_import_copies_nothing() {
    let phone = Phone::with(&SHOTS);
    let lab = Lab::new();
    let names = entry_names(&phone.export());
    let ignored = names
        .iter()
        .filter(|name| name.starts_with(".app/"))
        .count();
    lab.import(&phone.zip_path()).clean();
    let held = lab.files();

    let again = lab.import(&phone.zip_path());

    again.clean();
    again.tally(0, names.len() - ignored, 0, ignored);
    assert_eq!(lab.files(), held);
    assert_eq!(lab.corpus(), phone.corpus());
}

/// Export is cumulative (C16), so the first shots come again and only the new one's files are copied:
/// its manifest, its photo, its analysis and its shot.
#[test]
fn a_later_export_with_one_more_shot_imports_clean() {
    let phone = Phone::with(&SHOTS[..5]);
    let lab = Lab::new();
    lab.import(&phone.export()).clean();
    assert_eq!(lab.corpus().swings.len(), 5);
    let before = entry_names(&phone.zip_path());
    let (id, taken) = SHOTS[5];
    let card = phone.feed(id, taken);

    let names = entry_names(&phone.export());
    let imported = lab.import(&phone.zip_path());

    imported.clean();
    let ignored = names
        .iter()
        .filter(|name| name.starts_with(".app/"))
        .count();
    imported.tally(4, before.len() - (ignored - 1), 0, ignored);
    imported.says(&format!(
        "new       sessions/{}/{} (3 file(s))",
        card.session_id, card.swing_id
    ));
    assert_eq!(lab.corpus(), phone.corpus());
    assert_eq!(lab.corpus().swings.len(), 6);
    assert_eq!(lab.files(), phone.stored());
}

/// D6's point, through the words P12 pinned at its own end. The lab keeps its golfer, the import goes
/// on, and the tally never settles to `identical`, which P12 said and this holds it to. The golfer's
/// `created_at` is on no shot, so the corpus still agrees.
#[test]
fn a_lab_holding_the_same_golfer_with_another_created_at_imports_clean() {
    let phone = Phone::with(&SHOTS[..2]);
    let lab = Lab::new();
    let joined = "2026-01-01T00:00:00Z";
    let store = GolferStore::new(&lab.dirs.golfers);
    store
        .get_or_create("Aaron", StoredHandedness::Right, at(joined))
        .expect("the lab's golfer");
    let golfer = lab.dirs.golfers.join("aaron.golfer.json");
    let lab_copy = fs::read(&golfer).unwrap();
    let names = entry_names(&phone.export());
    let ignored = names
        .iter()
        .filter(|name| name.starts_with(".app/"))
        .count();
    let others = names.len() - ignored - 1;

    let imported = lab.import(&phone.zip_path());

    imported.clean();
    imported.says(&format!(
        "kept      golfers/aaron.golfer.json: the store holds this golfer already, and it stays; \
         the export differs in created_at ({}, stored {})",
        at(RECORDED),
        at(joined)
    ));
    imported.tally(others, 0, 1, ignored);
    assert_eq!(fs::read(&golfer).unwrap(), lab_copy);
    assert_eq!(lab.corpus(), phone.corpus());

    let again = lab.import(&phone.zip_path());
    again.clean();
    again.tally(0, others, 1, ignored);
}

/// D6's second half. A shot's analysis crosses as bytes, so the corpus agrees right after the import
/// even with the lab's lofts differing. P12 finding 8 says what a later re-analysis may do about that.
#[test]
fn a_lab_holding_another_bag_keeps_it_says_so_and_still_reads_the_phones_corpus() {
    let phone = Phone::new();
    set_bag_slot(
        phone.root.clone(),
        "aaron".to_string(),
        "7i".to_string(),
        Some(34.0),
        RECORDED.to_string(),
    )
    .expect("a slot");
    phone.feed(SHOTS[0].0, SHOTS[0].1);
    let lab = Lab::new();
    let bag = lab.dirs.golfers.join("aaron.bag.json");
    fs::create_dir_all(&lab.dirs.golfers).unwrap();
    fs::write(&bag, b"{\"the lab's\": \"own\"}").unwrap();

    let imported = lab.import(&phone.export());

    imported.clean();
    imported.says(
        "kept      golfers/aaron.bag.json: the store holds this golfer's bag already, which \
         differs, and it stays; the phone's bag was not applied",
    );
    assert_eq!(fs::read(&bag).unwrap(), b"{\"the lab's\": \"own\"}");
    assert_eq!(lab.corpus(), phone.corpus());
}

// ------------------------------------------------------- what does not reach the lab after an import

/// **C16's limit, pinned so that it is seen.** The lab keeps what it holds, says so twice, and refuses
/// nothing: the shot as `kept`, and the entry as `duplicate`, since a re-analysis moved its files too
/// (P7 finding 9). The corpora then differ, and that difference is the limit. provisional: pasted copies.
#[test]
fn a_correction_after_the_first_import_prints_kept_and_refuses_nothing() {
    let phone = Phone::new();
    let card = phone.feed(SHOTS[0].0, SHOTS[0].1);
    phone.feed(SHOTS[1].0, SHOTS[1].1);
    let lab = Lab::new();
    lab.import(&phone.export()).clean();
    let held = lab.files();
    let corpus = lab.corpus();

    phone.correct(&card, vec![typed("carry_distance", "150")]);
    let imported = lab.import(&phone.export());

    imported.clean();
    imported.says(&kept_shot_line(&digest_of(SHOTS[0].0)));
    imported.says(&duplicate_line(&card));
    assert_eq!(lab.files(), held, "nothing at the lab moved");
    assert_eq!(lab.corpus(), corpus);
    assert_ne!(lab.corpus(), phone.corpus(), "which is the limit");
}

/// P7 finding 9's other half: a review with nothing to change moves no byte, so the same export says
/// `identical` and neither of the correction's two lines. provisional: pasted copies.
#[test]
fn a_save_that_changed_nothing_leaves_identical() {
    let phone = Phone::new();
    let card = phone.feed(SHOTS[0].0, SHOTS[0].1);
    assert!(!card.flagged, "the empty save only moves no byte on these");
    let names = entry_names(&phone.export());
    let ignored = names
        .iter()
        .filter(|name| name.starts_with(".app/"))
        .count();
    let lab = Lab::new();
    lab.import(&phone.zip_path()).clean();
    let stored = phone.stored();

    phone.correct(&card, Vec::new());
    assert_eq!(phone.stored(), stored);
    let imported = lab.import(&phone.export());

    imported.clean();
    imported.tally(0, names.len() - ignored, 0, ignored);
    assert!(!imported.out.contains("kept "), "{}", imported.out);
    assert!(!imported.out.contains("duplicate "), "{}", imported.out);
}

/// P10 finding 6's first limit, as P10 finding 7 asked: a retag, an intent and a verdict each print
/// `duplicate`, refuse nothing and move nothing, and the shot's record is `identical`, since it holds
/// no club. It is C16's limit under another word. provisional: pasted copies.
#[test]
fn a_retag_an_intent_or_a_verdict_after_an_import_prints_duplicate_and_moves_nothing() {
    type Change = fn(&Phone, &ShotCard) -> Result<(), String>;
    let club: Change = |phone, card| {
        set_shot_club(
            phone.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
            "pw".to_string(),
            LATER.to_string(),
        )
        .map(|_| ())
    };
    let intent: Change = |phone, card| {
        set_shot_intent(
            phone.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
            PracticeMode::Drill,
            None,
            LATER.to_string(),
        )
        .map(|_| ())
    };
    let verdict: Change = |phone, card| {
        set_shot_mishit(
            phone.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
            Some(MishitVerdict::Confirmed),
            LATER.to_string(),
        )
        .map(|_| ())
    };

    for (what, change) in [("club", club), ("intent", intent), ("verdict", verdict)] {
        let phone = Phone::new();
        let card = phone.feed(SHOTS[0].0, SHOTS[0].1);
        let lab = Lab::new();
        lab.import(&phone.export()).clean();
        let held = lab.files();

        change(&phone, &card).unwrap_or_else(|refused| panic!("{what}: {refused}"));
        let imported = lab.import(&phone.export());

        imported.clean();
        imported.says(&duplicate_line(&card));
        imported.says(&format!(
            "identical shots/{}.shot.json",
            digest_of(SHOTS[0].0)
        ));
        assert_eq!(lab.files(), held, "{what}");
        assert_ne!(lab.corpus(), phone.corpus(), "{what}");
    }
}

/// A photo the store holds keeps what a person typed through a re-read at the lab, because the store is
/// keyed by the photo's hash and the file crossed as it was (C12, P8 finding 7). The replay reads the
/// capture's own boxes, which disagree with the correction, so a re-read that lost it would show.
/// provisional: pasted copies.
#[test]
fn a_correction_that_reached_the_lab_survives_the_labs_re_read() {
    struct Replay(Prepared);
    impl PhotoReader for Replay {
        fn ready(&mut self) -> Result<(), String> {
            Ok(())
        }
        fn prepare(&mut self, _: &Path, _: &DeviceProfile) -> Result<Prepared, String> {
            Ok(self.0.clone())
        }
    }

    let phone = Phone::new();
    let (id, taken) = SHOTS[0];
    let card = phone.feed(id, taken);
    let digest = digest_of(id);
    let read = ShotStore::new(&phone.dirs().shots)
        .get(&digest)
        .unwrap()
        .unwrap();
    phone.correct(&card, vec![typed("carry_distance", "150")]);
    let lab = Lab::new();
    lab.import(&phone.export()).clean();
    let corrected = lab.shot(&digest);
    assert_ne!(corrected.carry_distance, read.carry_distance);

    let boxes: Value = serde_json::from_str(&raw_boxes(id)).unwrap();
    let mut reader = Replay(Prepared {
        boxes: serde_json::from_value(boxes["boxes"].clone()).unwrap(),
        notes: Vec::new(),
    });
    let options = ShotOptions {
        device: DEVICE.to_string(),
        force_ocr: true,
        ..ShotOptions::new(&lab.dirs.shots)
    };
    let swing_dir = lab
        .dirs
        .sessions
        .join(&card.session_id)
        .join(&card.swing_id);
    let answer = shot_for(
        &swing_dir,
        &lab.entry(&card),
        &options,
        &mut reader,
        &mut |_| {},
    )
    .expect("a re-read");

    let reread = lab.shot(&digest);
    assert_eq!(
        reread.carry_distance, corrected.carry_distance,
        "{:?}",
        answer.origin
    );
    let map = |shot: &ShotData| shot.provenance.as_ref().unwrap().corrections.clone();
    assert_eq!(map(&reread), map(&corrected));
    assert!(map(&reread).contains_key("carry_distance"));
}

// --------------------------------------------------------- a number is never given out twice

/// **The user's decision of 2026-11-05, on P10's question.** A shot the lab holds is deleted on the
/// phone, another photo comes under the same date, and the next export imports: the new photo takes a
/// number the lab has never seen. The deleted one stays at the lab, since a delete does not travel
/// (P10 finding 6). provisional: pasted copies.
#[test]
fn a_shot_deleted_after_its_import_leaves_the_next_export_importing_clean() {
    let phone = Phone::with(&SHOTS[..1]);
    let second = phone.feed(SHOTS[1].0, SHOTS[1].1);
    assert_eq!(second.swing_id, "2");
    let lab = Lab::new();
    lab.import(&phone.export()).clean();

    phone.delete(&second);
    let third = phone.feed(SHOTS[2].0, SHOTS[2].1);
    assert_ne!(third.swing_id, second.swing_id);
    let imported = lab.import(&phone.export());

    imported.clean();
    imported.says(&format!(
        "new       sessions/{}/{} (3 file(s))",
        third.session_id, third.swing_id
    ));
    assert_eq!(lab.entry(&second).swing_id, second.swing_id);
    assert_eq!(
        lab.shot(&digest_of(SHOTS[2].0)).carry_distance,
        ShotStore::new(&phone.dirs().shots)
            .get(&digest_of(SHOTS[2].0))
            .unwrap()
            .unwrap()
            .carry_distance
    );

    // And again, with the number after it, so a floor kept once is not the whole of the fix.
    phone.delete(&third);
    phone.feed(SHOTS[3].0, SHOTS[3].1);
    lab.import(&phone.export()).clean();
}

/// The session's only shot takes its directory with it, so a floor kept beside the entries would have
/// gone too. It is the same refusal if the number comes back. provisional: pasted copies.
#[test]
fn a_sessions_only_shot_deleted_after_its_import_leaves_the_next_export_importing_clean() {
    let phone = Phone::new();
    let only = phone.feed(SHOTS[0].0, SHOTS[0].1);
    let lab = Lab::new();
    lab.import(&phone.export()).clean();

    phone.delete(&only);
    assert!(!phone.dirs().sessions.join(&only.session_id).exists());
    let next = phone.feed(SHOTS[1].0, SHOTS[1].1);

    assert_eq!(next.session_id, only.session_id);
    assert_ne!(next.swing_id, only.swing_id);
    lab.import(&phone.export()).clean();
}

// ------------------------------------------------------------------------------- what is refused

/// Every time crossing from Dart is held to this (`parse_now`), and this one reaches Settings' last
/// export. It is refused before any file is touched. provisional: pasted copies.
#[test]
fn a_time_with_no_offset_is_refused_and_nothing_is_written() {
    let phone = Phone::with(&SHOTS[..1]);
    let refused = export_sessions(
        phone.root.clone(),
        phone.zip_path().display().to_string(),
        "2026-11-05T11:00:00".to_string(),
    )
    .expect_err("a naive time");
    assert!(refused.contains("offset"), "{refused}");
    assert!(!phone.zip_path().exists());
}

/// In a tree, the next export would carry this one, and a zip inside a zip inside a zip is what a
/// wrong path in P27 would become. provisional: pasted copies.
#[test]
fn a_path_inside_the_data_is_refused_and_nothing_is_written() {
    let phone = Phone::with(&SHOTS[..1]);
    let before = files_below(Path::new(&phone.root));
    for inside in ["sessions/export.zip", "export.zip", ".app/export.zip"] {
        let path = Path::new(&phone.root).join(inside);
        let refused = export_sessions(
            phone.root.clone(),
            path.display().to_string(),
            EXPORTED.to_string(),
        )
        .expect_err(inside);
        assert!(refused.contains("inside the data"), "{refused}");
    }
    assert_eq!(files_below(Path::new(&phone.root)), before);
}

/// The last export is what the share sheet may still hold, so a failure writing the next one leaves it
/// whole, and nothing half-made at the path. Its `.part` taken by a directory is a failure every OS
/// agrees on. provisional: pasted copies.
#[test]
fn a_failed_export_leaves_the_last_one_whole() {
    let phone = Phone::with(&SHOTS[..1]);
    let last = fs::read(phone.export()).unwrap();
    phone.feed(SHOTS[1].0, SHOTS[1].1);
    let part = PathBuf::from(format!("{}.part", phone.zip_path().display()));
    fs::create_dir(&part).unwrap();

    let refused = export_sessions(
        phone.root.clone(),
        phone.zip_path().display().to_string(),
        EXPORTED.to_string(),
    )
    .expect_err("its .part is taken");

    assert!(refused.contains("could not write"), "{refused}");
    assert_eq!(fs::read(phone.zip_path()).unwrap(), last);
}

/// A directory at the path is what cannot be replaced by a rename, and the `.part` written first goes
/// again rather than sitting in `tmp/` for the rest of the phone's life. provisional: pasted copies.
#[test]
fn a_rename_that_fails_leaves_no_part_behind() {
    let phone = Phone::with(&SHOTS[..1]);
    let taken = phone.dir.path().join("taken");
    fs::create_dir(&taken).unwrap();

    let refused = export_sessions(
        phone.root.clone(),
        taken.display().to_string(),
        EXPORTED.to_string(),
    )
    .expect_err("a directory at the path");

    assert!(refused.contains("could not write"), "{refused}");
    assert!(taken.is_dir());
    assert!(!phone.dir.path().join("taken.part").exists());
}

/// An export moves nothing on the phone, `last_export_at` included, because P27 saves that only once
/// the share has gone (C16). provisional: pasted copies.
#[test]
fn an_export_moves_nothing_on_the_phone() {
    let phone = Phone::with(&SHOTS[..2]);
    phone.log(RecordStatus::Read);
    let before = files_below(Path::new(&phone.root));
    phone.export();
    assert_eq!(files_below(Path::new(&phone.root)), before);
    assert_eq!(get_state(phone.root.clone()).last_export_at, None);
}

/// A fresh phone has no store directories at all. Its zip would hold nothing under the three trees, and
/// the lab refuses that with exit 2, so the phone says so first and hands the share sheet nothing.
#[test]
fn a_phone_with_nothing_on_it_refuses_rather_than_write_what_the_lab_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("processed").display().to_string();
    let zip = dir.path().join("tmp").join("export.zip");

    let refused = export_sessions(root, zip.display().to_string(), EXPORTED.to_string())
        .expect_err("nothing to export");

    assert!(refused.contains("nothing on this phone"), "{refused}");
    assert!(!zip.exists());
}

/// A session of nothing but failed reads still leaves: its diagnostics are the ones most wanted, and
/// the golfer is what makes it a layout the lab reads. The lab takes the golfer and counts the rest.
/// provisional: pasted copies.
#[test]
fn a_golfer_with_only_failed_reads_exports_and_imports_clean() {
    let phone = Phone::new();
    let failed = phone.fail(SHOTS[0].0, SHOTS[0].1);
    let summary = phone.export_at(EXPORTED);

    assert_eq!((summary.shots, summary.files), (0, 3));
    let names = entry_names(&phone.zip_path());
    assert!(
        names.contains(&format!(".app/failed/{failed}.jpg")),
        "{names:?}"
    );
    let lab = Lab::new();
    let imported = lab.import(&phone.zip_path());
    imported.clean();
    imported.tally(1, 0, 0, 2);
    assert_eq!(lab.corpus(), phone.corpus());
}

/// Nothing serializes bridge calls, and a shot torn in the zip refuses the whole import at the lab, so
/// the export holds the writers' lock. Each zip cut during a run of records and deletes must import
/// clean into a lab of its own, with no export refused by a file vanishing under it.
///
/// **`.app/failed/` is made heavy so the race is a race.** It sorts before `sessions/`, so each export
/// lists an entry, spends its time copying these, and only then opens the entry's files. A writer
/// gets a whole record and delete in there, if nothing stops it. provisional: pasted copies.
#[test]
fn exports_overlapping_records_and_deletes_are_each_whole() {
    let phone = Phone::with(&SHOTS[..2]);
    let heavy = phone.dirs().app.join("failed");
    fs::create_dir_all(&heavy).unwrap();
    for name in 0..5 {
        fs::write(heavy.join(format!("{name}.jpg")), vec![7u8; 8 << 20]).unwrap();
    }
    let zip = phone.zip_path();
    thread::scope(|scope| {
        let writer = scope.spawn(|| {
            for _ in 0..8 {
                let card = phone.feed(SHOTS[2].0, SHOTS[2].1);
                phone.delete(&card);
            }
        });
        let mut round = 0;
        while !writer.is_finished() || round < 8 {
            export_sessions(
                phone.root.clone(),
                zip.display().to_string(),
                EXPORTED.to_string(),
            )
            .unwrap_or_else(|refused| panic!("round {round}: {refused}"));
            Lab::new().import(&zip).clean();
            round += 1;
        }
    });
}

// ------------------------------------------------------------------------------ what Settings says

#[test]
fn before_any_export_every_shot_is_since_and_no_last_export_is_named() {
    let phone = Phone::with(&SHOTS[..2]);
    assert_eq!(
        phone.status(),
        ExportStatus {
            last_export_at: None,
            last_export_display: None,
            last_export_shots: None,
            shots: 2,
            shots_since: 2,
        }
    );
}

/// The summary's two values through `set_state`, as P27 saves them, are what the status reads back. A
/// shot recorded later counts, by its entry's own time. provisional: pasted copies.
#[test]
fn a_saved_export_leaves_nothing_since_until_the_next_shot() {
    let phone = Phone::with(&SHOTS[..2]);
    let summary = phone.export_at(EXPORTED);
    assert_eq!(phone.status().shots_since, 2, "not saved yet");
    phone.save_export(&summary);
    assert_eq!(
        phone.status(),
        ExportStatus {
            last_export_at: Some(EXPORTED.to_string()),
            last_export_display: Some("2026-11-05 11:00".to_string()),
            last_export_shots: Some(2),
            shots: 2,
            shots_since: 0,
        }
    );

    record_photo(PhotoIn {
        now: LATER.to_string(),
        ..photo_in(&phone, SHOTS[2])
    })
    .expect("a shot");
    let status = phone.status();
    assert_eq!((status.shots, status.shots_since), (3, 1));
    assert_eq!(status.last_export_shots, Some(2));
}

/// By subtraction this would read 0: two exported, one deleted, one more, and two on the phone. The one
/// that has never left is the number a person is asking for. provisional: pasted copies.
#[test]
fn a_stray_deleted_and_a_photo_retaken_reads_one_since_and_not_none() {
    let phone = Phone::with(&SHOTS[..1]);
    let stray = phone.feed(SHOTS[1].0, SHOTS[1].1);
    let summary = phone.export_at(EXPORTED);
    phone.save_export(&summary);

    phone.delete(&stray);
    record_photo(PhotoIn {
        now: LATER.to_string(),
        ..photo_in(&phone, SHOTS[2])
    })
    .expect("a shot");

    let status = phone.status();
    assert_eq!((status.shots, status.shots_since), (2, 1));
    assert_eq!(status.last_export_shots, Some(2));
}

/// `set_state` refuses this, so only a file edited by hand holds it, and a read here never refuses
/// (`crate::state`'s doc). Claiming no last export overstates what is unsent, which is the safe way
/// round: it sends a person to export, and an export is cumulative. provisional: pasted copies.
#[test]
fn a_last_export_time_that_does_not_read_is_no_last_export() {
    let phone = Phone::with(&SHOTS[..1]);
    let path = phone.dirs().app.join("state.json");
    let mut state: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    state["last_export_at"] = Value::String("yesterday".to_string());
    fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();

    let status = phone.status();
    assert_eq!(status.last_export_at, None);
    assert_eq!(status.last_export_display, None);
    assert_eq!(status.shots_since, 1);
}

/// One capture as Dart passes it, for a test that needs a `now` of its own.
fn photo_in(phone: &Phone, (id, taken): (&str, &str)) -> PhotoIn {
    let tmp = phone.dir.path().join("tmp");
    fs::create_dir_all(&tmp).expect("tmp");
    let to = tmp.join(format!("{id}.jpg"));
    fs::copy(spike().join("captures").join(format!("{id}.jpg")), &to).expect("a capture");
    PhotoIn {
        root: phone.root.clone(),
        path: to.display().to_string(),
        boxes_json: raw_boxes(id),
        captured_at: taken.to_string(),
        now: RECORDED.to_string(),
        challenge_call: None,
    }
}
