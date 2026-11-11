//! `golf-lab import-export` over a phone's export built by the real writers: a photo-only session
//! with its `shot_analysis.json`, a shot, a golfer and a bag, imported, imported again, refused on a
//! conflict, deduped by photo and read from a zip. [M29 P31] A golfer and a bag that differ from the
//! lab's go by M38's D6. [M38 P12]
//!
//! No Python twin, and no committed vector: the phone does not exist yet (M38). So the export is
//! what M38's phone will write, a `data/processed` subtree as `crates/storage` writes it (the M29
//! plan's D16), made here by `golf-lab import-photos --into-session` over a reader replaying a
//! committed screen vector's boxes, as `tests/import_photos.rs` makes its entries. Every case ends
//! where M38 P4's exit does: on the lab's `read_corpus` reading what was imported as the phone's.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use contracts::bag::Bag;
use contracts::club::parse_club;
use contracts::shot::ShotData;
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::Timestamp;
use golf_lab::import::{import_photos, IntoSession, Request as PhotoRequest};
use golf_lab::import_export::{import_export, Request};
use golf_lab::shot::{PhotoReader, Prepared, ShotOptions};
use screen::profile::DeviceProfile;
use screen::ScreenInput;
use serde_json::Value;
use storage::bag_store::BagStore;
use storage::corpus::{read_corpus, EngineVersions};
use storage::golfer_store::GolferStore;
use storage::manifest::{load_manifest, manifest_path, save_manifest};
use storage::session_meta::set_current_club;
use storage::shot_store::{hash_image, ShotStore};
use zip::write::{FullFileOptions, ZipWriter};
use zip::CompressionMethod;

const PHOTO: &[u8] = b"a stand-in for a bay photo the phone took, which only a reader would open";
const SESSION: &str = "2026-11-02";

/// 2026-11-02T10:00:00Z, whole seconds, so a zip's extended timestamp carries it exactly.
const TAKEN_AT: u64 = 1_793_613_600;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(relative: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(repo().join(relative)).unwrap()).unwrap()
}

fn now() -> Timestamp {
    "2026-11-02T10:05:00Z".parse().unwrap()
}

fn versions() -> EngineVersions {
    EngineVersions {
        installed: ANALYSIS_VERSION,
        comparable_from: COMPARABLE_FROM,
    }
}

/// A reader replaying `2026-08-23-2`'s boxes, which frozen PaddleOCR read off that bay photo.
struct Replay(Prepared);

impl PhotoReader for Replay {
    fn ready(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn prepare(&mut self, _photo: &Path, _profile: &DeviceProfile) -> Result<Prepared, String> {
        Ok(self.0.clone())
    }
}

fn replay() -> Replay {
    let document = read_json("spec/vectors/screen/corpus/2026-08-23-2.json");
    let input: ScreenInput = serde_json::from_value(document["input"].clone()).unwrap();
    Replay(Prepared {
        boxes: input.boxes,
        notes: input.notes,
    })
}

/// `golfers/` holding `aaron`, right-handed, with the bag the storage family records for him.
fn golfers(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("aaron.golfer.json"),
        r#"{"player_id": "aaron", "display_name": "Aaron", "handedness": "right",
            "created_at": "2026-08-07T15:04:05.123456Z"}"#,
    )
    .unwrap();
    let bags = read_json("spec/vectors/storage/stores/bag-reads.json");
    fs::write(
        BagStore::new(dir).path_for("aaron"),
        bags["input"]["files"]["aaron.bag.json"].as_str().unwrap(),
    )
    .unwrap();
}

/// The phone's `data/processed`: `aaron` and his bag, and session `2026-11-02` holding one photo
/// entry analysed into its `shot_analysis.json`, its shot in `shots/`, and the session's club
/// cursor in `session.json`. The photo it was read from stays outside the export.
struct Phone {
    root: tempfile::TempDir,
    _camera: tempfile::TempDir,
}

impl Phone {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let camera = tempfile::tempdir().unwrap();
        let photo = camera.path().join("IMG_0042.jpg");
        fs::write(&photo, PHOTO).unwrap();
        fs::File::options()
            .write(true)
            .open(&photo)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_secs(TAKEN_AT))
            .unwrap();
        golfers(&root.path().join("golfers"));
        let request = PhotoRequest {
            paths: vec![photo],
            session: None,
            options: ShotOptions::new(root.path().join("shots")),
            into: Some(IntoSession {
                session_id: SESSION.to_string(),
                player: "Aaron".to_string(),
                club: "7i".to_string(),
                sessions_dir: root.path().join("sessions"),
                golfers_dir: root.path().join("golfers"),
            }),
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = import_photos(&request, &mut replay(), now(), &mut out, &mut err).unwrap();
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
        set_current_club(
            &root.path().join("sessions").join(SESSION),
            parse_club("7i"),
            now(),
        )
        .unwrap();
        Phone {
            root,
            _camera: camera,
        }
    }

    fn path(&self) -> &Path {
        self.root.path()
    }
}

/// The three stores' directories under `root`, as `--sessions-dir`, `--shots-dir` and
/// `--golfers-dir` would name them.
fn request(export: &Path, lab: &Path) -> Request {
    Request {
        export: export.to_path_buf(),
        sessions_dir: lab.join("sessions"),
        shots_dir: lab.join("shots"),
        golfers_dir: lab.join("golfers"),
        dry_run: false,
    }
}

/// One run: the exit code, stdout and stderr.
fn run(request: &Request) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = import_export(request, &mut out, &mut err).unwrap();
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// Every file under `root` but the bundle store's `.incoming/`, by `/`-joined path, with its bytes.
fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            if name.starts_with('.') {
                continue;
            }
            let rel = format!("{prefix}{name}");
            if entry.file_type().unwrap().is_dir() {
                stack.push((entry.path(), format!("{rel}/")));
            } else {
                found.insert(rel, fs::read(entry.path()).unwrap());
            }
        }
    }
    found
}

fn modified(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

/// The entry's photo, as the phone's store named it.
fn photo_name(sessions: &Path) -> String {
    let manifest = load_manifest(&manifest_path(&sessions.join(SESSION).join("1"))).unwrap();
    manifest.roles[&storage::manifest::Role::ShotScreen]
        .filename
        .clone()
}

fn digest() -> String {
    hash_image(PHOTO)
}

/// The report's line for one unit, as the verb pads its status.
fn line(status: &str, what: &str) -> String {
    format!("\n  {status:<9} {what}")
}

// ----------------------------------------------------------------------------------- the merge

/// Into an empty lab every file is new and copied as the phone's bytes, the photo keeps its time,
/// and the lab's corpus is the phone's: one entry, pooled, nothing excluded.
#[test]
fn a_photo_only_session_imports_whole_and_reads_as_the_phones_corpus() {
    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");

    assert!(
        out.starts_with(&format!(
            "\nImporting the export at {}\n  sessions into {}\n  shots into {}\n  golfers into {}\n",
            phone.path().display(),
            lab.path().join("sessions").display(),
            lab.path().join("shots").display(),
            lab.path().join("golfers").display(),
        )),
        "{out}"
    );
    let expected = [
        line("new", "golfers/aaron.bag.json"),
        line("new", "golfers/aaron.golfer.json"),
        line("new", &format!("shots/{}.shot.json", digest())),
        line("new", &format!("sessions/{SESSION}/session.json")),
        line("new", &format!("sessions/{SESSION}/1 (3 file(s))")),
    ];
    let mut at = 0;
    for want in &expected {
        let found = out[at..].find(want.as_str()).unwrap_or_else(|| {
            panic!("{want:?} after byte {at}, in the write order: {out}");
        });
        at += found + want.len();
    }
    assert!(
        out.ends_with("\n\n7 file(s) copied; 0 identical, 0 left as stored, 0 ignored.\n"),
        "{out}"
    );

    assert_eq!(files(lab.path()), files(phone.path()), "the phone's bytes");
    let photo = photo_name(&phone.path().join("sessions"));
    let copy = lab
        .path()
        .join("sessions")
        .join(SESSION)
        .join("1")
        .join(&photo);
    assert_eq!(
        modified(&copy),
        UNIX_EPOCH + Duration::from_secs(TAKEN_AT),
        "the photo's time, which is its shot's"
    );
    assert!(GolferStore::new(lab.path().join("golfers"))
        .get("aaron")
        .is_some());
    assert!(BagStore::new(lab.path().join("golfers"))
        .get("aaron")
        .is_some());
    let shot = ShotStore::new(lab.path().join("shots"))
        .get(&digest())
        .unwrap()
        .expect("the shot, under its photo's key");
    assert_eq!(shot.shot_id, format!("{SESSION}-1"));

    let corpus = read_corpus(&lab.path().join("sessions"), "aaron", versions());
    assert_eq!(
        corpus,
        read_corpus(&phone.path().join("sessions"), "aaron", versions()),
        "M38 P4's exit: the lab's reader agrees with the phone's on what was exported"
    );
    assert!(corpus.excluded.is_empty(), "{:?}", corpus.excluded);
    let [entry] = corpus.swings.as_slice() else {
        panic!("one entry pooled: {:?}", corpus.swings);
    };
    assert_eq!(
        (entry.session_id.as_str(), entry.swing_id.as_str()),
        (SESSION, "1")
    );
    assert_eq!(entry.face_on_sha256, None);
    assert_eq!(entry.shot_sha256.as_deref(), Some(digest().as_str()));
    assert!(
        entry.shot.is_some(),
        "the entry's shot, from shot_analysis.json"
    );
}

/// The same export twice: the second run finds every file stored identically, copies nothing and
/// touches nothing.
#[test]
fn a_second_import_of_the_same_export_copies_nothing() {
    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    assert_eq!(run(&request(phone.path(), lab.path())).0, 0);
    let manifest = manifest_path(&lab.path().join("sessions").join(SESSION).join("1"));
    let before = (files(lab.path()), modified(&manifest));

    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    for want in [
        line("identical", "golfers/aaron.golfer.json"),
        line("identical", &format!("shots/{}.shot.json", digest())),
        line("identical", &format!("sessions/{SESSION}/1 (3 file(s))")),
    ] {
        assert!(out.contains(&want), "{want:?}: {out}");
    }
    assert!(
        out.ends_with("\n\n0 file(s) copied; 7 identical, 0 left as stored, 0 ignored.\n"),
        "{out}"
    );
    assert_eq!((files(lab.path()), modified(&manifest)), before);
}

/// A dry run judges every file as a run would and writes nothing, not even a store's directory.
#[test]
fn a_dry_run_writes_nothing() {
    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    let mut dry = request(phone.path(), lab.path());
    dry.dry_run = true;
    let (code, out, err) = run(&dry);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains(&line("new", &format!("sessions/{SESSION}/1 (3 file(s))"))));
    assert!(
        out.ends_with(
            "\n\nDry run: 7 file(s) would be copied; 0 identical, 0 left as stored, 0 ignored. \
             Nothing was written.\n"
        ),
        "{out}"
    );
    assert_eq!(fs::read_dir(lab.path()).unwrap().count(), 0);
}

/// Two files stored with other bytes than the export's: both are named, the run exits 1, and
/// nothing is written, the files that were new included.
#[test]
fn one_file_that_differs_refuses_the_whole_import_and_names_each() {
    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    let golfer = lab.path().join("golfers").join("aaron.golfer.json");
    fs::create_dir_all(golfer.parent().unwrap()).unwrap();
    let left_handed = r#"{"player_id": "aaron", "display_name": "Aaron", "handedness": "left",
            "created_at": "2026-08-07T15:04:05.123456Z"}"#;
    fs::write(&golfer, left_handed).unwrap();
    let cursor = lab
        .path()
        .join("sessions")
        .join(SESSION)
        .join("session.json");
    fs::create_dir_all(cursor.parent().unwrap()).unwrap();
    fs::write(&cursor, "{}").unwrap();
    let before = files(lab.path());

    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!(code, 1, "{out}{err}");
    assert_eq!(
        err,
        format!(
            "error: nothing was imported, because 2 file(s) in the export differ from what is \
             stored at the same place:\n  golfers/aaron.golfer.json -> {}\n  \
             sessions/{SESSION}/session.json -> {}\n",
            golfer.display(),
            cursor.display()
        )
    );
    assert!(!out.contains("  new "), "no unit is reported: {out}");
    assert_eq!(files(lab.path()), before, "nothing written");
    assert!(!lab.path().join("shots").exists());
    assert!(!lab.path().join("sessions").join(SESSION).join("1").exists());
}

/// A shot of a photo the store holds a shot of, reading differently, is that photo's and is kept;
/// a photo entry whose photo its session holds, in its own swing or another, is a duplicate and the
/// stored swing stays. Neither is a conflict.
#[test]
fn a_shot_and_an_entry_whose_photo_is_stored_are_left_as_stored() {
    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    assert_eq!(run(&request(phone.path(), lab.path())).0, 0);

    // The lab re-read the photo since, and its entry's analysis moved with it.
    let store = ShotStore::new(lab.path().join("shots"));
    let mut shot: ShotData = store.get(&digest()).unwrap().unwrap();
    shot.provenance.as_mut().unwrap().parse_confidence = 0.25;
    store.put(&shot).unwrap();
    let entry = lab.path().join("sessions").join(SESSION).join("1");
    let analysis = entry.join("shot_analysis.json");
    let mut text = fs::read(&analysis).unwrap();
    text.extend(b"\n");
    fs::write(&analysis, &text).unwrap();
    let before = files(lab.path());

    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    for want in [
        line(
            "kept",
            &format!(
                "shots/{}.shot.json: the store holds a shot of this photo already, which reads \
                 differently, and it stays",
                digest()
            ),
        ),
        line(
            "duplicate",
            &format!(
                "sessions/{SESSION}/1: session {SESSION} holds its photo already, in swing 1, and \
                 that swing stays"
            ),
        ),
    ] {
        assert!(out.contains(&want), "{want:?}: {out}");
    }
    assert!(
        out.ends_with("\n\n0 file(s) copied; 3 identical, 4 left as stored, 0 ignored.\n"),
        "{out}"
    );
    assert_eq!(files(lab.path()), before, "the lab's copies stay");

    // The lab's entry is swing 7 now: the export's swing 1 holds the same photo, so it is still a
    // duplicate, and no swing 1 is opened beside it.
    let seven = lab.path().join("sessions").join(SESSION).join("7");
    fs::rename(&entry, &seven).unwrap();
    let mut manifest = load_manifest(&manifest_path(&seven)).unwrap();
    manifest.swing_id = "7".to_string();
    save_manifest(&manifest, &manifest_path(&seven)).unwrap();
    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains(&line(
        "duplicate",
        &format!(
            "sessions/{SESSION}/1: session {SESSION} holds its photo already, in swing 7, and \
             that swing stays"
        )
    )));
    assert!(!entry.exists());
}

/// A shot keyed by its `shot_id`, with no photo hash, has no photo to be: a difference is a
/// conflict. And a shot is placed under the store's key, whatever the export named it.
#[test]
fn a_shot_with_no_photo_hash_conflicts_and_a_shot_lands_under_its_key() {
    let phone = Phone::new();
    let shots = phone.path().join("shots");
    let mut shot: ShotData = ShotStore::new(&shots).get(&digest()).unwrap().unwrap();
    fs::rename(
        ShotStore::new(&shots).path_for(&digest()),
        shots.join("renamed-on-the-phone.shot.json"),
    )
    .unwrap();
    shot.shot_id = "typed-in-1".to_string();
    if let Some(provenance) = shot.provenance.as_mut() {
        provenance.image_sha256 = None;
    }
    let keyless = ShotStore::new(&shots).put(&shot).unwrap();
    assert_eq!(crate_name(&keyless), "typed-in-1.shot.json");

    let lab = tempfile::tempdir().unwrap();
    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains(&line(
        "new",
        &format!(
            "shots/renamed-on-the-phone.shot.json (stored as {}.shot.json, the store's name for \
             its key)",
            digest()
        )
    )));
    assert!(ShotStore::new(lab.path().join("shots"))
        .get(&digest())
        .unwrap()
        .is_some());

    shot.provenance.as_mut().unwrap().parse_confidence = 0.25;
    ShotStore::new(lab.path().join("shots")).put(&shot).unwrap();
    let (code, _, err) = run(&request(phone.path(), lab.path()));
    assert_eq!(code, 1);
    assert!(
        err.ends_with(&format!(
            "\n  shots/typed-in-1.shot.json -> {}\n",
            lab.path()
                .join("shots")
                .join("typed-in-1.shot.json")
                .display()
        )),
        "{err}"
    );
}

fn crate_name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

/// The same entry stored with a file missing and none different is the same entry: the missing
/// file is copied, and the lab's corpus is the phone's again.
#[test]
fn an_entry_stored_with_a_file_missing_is_filled_in() {
    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    assert_eq!(run(&request(phone.path(), lab.path())).0, 0);
    let entry = lab.path().join("sessions").join(SESSION).join("1");
    fs::remove_file(entry.join("shot_analysis.json")).unwrap();
    let unanalysed = read_corpus(&lab.path().join("sessions"), "aaron", versions());
    assert!(
        unanalysed.swings.iter().all(|swing| !swing.analyzed) && unanalysed.excluded.len() == 1,
        "the entry reads as never analysed: {unanalysed:?}"
    );

    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains(&line(
        "added",
        &format!("sessions/{SESSION}/1 (1 new file(s), 2 identical)")
    )));
    assert!(
        out.ends_with("\n\n1 file(s) copied; 6 identical, 0 left as stored, 0 ignored.\n"),
        "{out}"
    );
    assert_eq!(
        read_corpus(&lab.path().join("sessions"), "aaron", versions()),
        read_corpus(&phone.path().join("sessions"), "aaron", versions())
    );
}

/// Copy the phone's entry 1 to `id`, its manifest saying so, and with a face-on clip when `filmed`.
fn copy_entry(phone: &Phone, id: &str, filmed: bool) {
    let session = phone.path().join("sessions").join(SESSION);
    let to = session.join(id);
    fs::create_dir_all(&to).unwrap();
    for entry in fs::read_dir(session.join("1")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
    }
    let mut manifest = load_manifest(&manifest_path(&to)).unwrap();
    manifest.swing_id = id.to_string();
    if filmed {
        const CLIP: &[u8] = b"a stand-in for a face-on clip";
        let mut clip = manifest.roles[&storage::manifest::Role::ShotScreen].clone();
        clip.role = storage::manifest::Role::FaceOn;
        clip.filename = "face_on.0a1b2c3d4e5f.mp4".to_string();
        clip.content_sha256 = hash_image(CLIP);
        clip.original_filename = "face_on.mp4".to_string();
        clip.content_type = "video/mp4".to_string();
        clip.size_bytes = i64::try_from(CLIP.len()).unwrap();
        fs::write(to.join(&clip.filename), CLIP).unwrap();
        manifest.roles.insert(storage::manifest::Role::FaceOn, clip);
    }
    save_manifest(&manifest, &manifest_path(&to)).unwrap();
}

/// An export's own second entry for one photo is a duplicate of the swing before it, a filmed one
/// included; a filmed swing holding that photo is never one.
#[test]
fn an_entry_its_export_repeats_is_a_duplicate_and_a_filmed_swing_never_is() {
    let phone = Phone::new();
    copy_entry(&phone, "2", true);
    copy_entry(&phone, "10", false);
    let lab = tempfile::tempdir().unwrap();
    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    let expected = [
        line("new", &format!("sessions/{SESSION}/1 (3 file(s))")),
        line("new", &format!("sessions/{SESSION}/2 (4 file(s))")),
        line(
            "duplicate",
            &format!(
                "sessions/{SESSION}/10: session {SESSION} holds its photo already, in a swing \
                 earlier in this export, and that swing stays"
            ),
        ),
    ];
    let mut at = 0;
    for want in &expected {
        let found = out[at..].find(want.as_str()).unwrap_or_else(|| {
            panic!("{want:?} after byte {at}, swings in the store's order: {out}")
        });
        at += found + want.len();
    }
    assert!(!lab
        .path()
        .join("sessions")
        .join(SESSION)
        .join("10")
        .exists());
    assert!(
        out.ends_with("\n\n11 file(s) copied; 0 identical, 3 left as stored, 0 ignored.\n"),
        "{out}"
    );
}

/// A write that fails names its file and what was copied before it, and once its cause is gone a
/// re-run copies the rest, finding the rest stored identically.
#[test]
fn a_write_that_fails_part_way_is_named_and_a_re_run_finishes_it() {
    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    let sessions = lab.path().join("sessions");
    fs::write(&sessions, "a file where the sessions directory goes").unwrap();

    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!(code, 2, "{out}{err}");
    let cursor = sessions.join(SESSION).join("session.json");
    assert!(
        err.starts_with(&format!("error: writing {}: ", cursor.display())),
        "{err}"
    );
    assert!(
        err.ends_with(
            ". 3 of 7 file(s) were copied before it; a re-run copies the rest, since those are \
             stored identically now.\n"
        ),
        "{err}"
    );
    assert!(ShotStore::new(lab.path().join("shots")).has(&digest()));

    fs::remove_file(&sessions).unwrap();
    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.ends_with("\n\n4 file(s) copied; 3 identical, 0 left as stored, 0 ignored.\n"),
        "{out}"
    );
    assert_eq!(files(lab.path()), files(phone.path()));
}

// ----------------------------------------------------------------------------------- M38's D6

/// A golfer record as both sides write one, spaced as `golfers` spaces the phone's.
fn golfer_file(player_id: &str, display_name: &str, handedness: &str, created_at: &str) -> String {
    format!(
        r#"{{"player_id": "{player_id}", "display_name": "{display_name}", "handedness": "{handedness}",
            "created_at": "{created_at}"}}"#
    )
}

/// The phone's `created_at`, from `golfers`.
const PHONES_CREATED_AT: &str = "2026-08-07T15:04:05.123456Z";

/// `text` stored in the lab's `golfers/` at `name`.
fn store_golfers_file(lab: &Path, name: &str, text: &str) {
    let path = lab.join("golfers").join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// The whole import refused on these `golfers/` files alone, exit 1, and nothing written.
fn refused_on(phone: &Phone, lab: &Path, names: &[&str]) {
    let before = files(lab);
    let (code, out, err) = run(&request(phone.path(), lab));
    assert_eq!(code, 1, "{names:?}: {out}{err}");
    let conflicts: String = names
        .iter()
        .map(|name| {
            format!(
                "\n  golfers/{name} -> {}",
                lab.join("golfers").join(name).display()
            )
        })
        .collect();
    assert!(err.ends_with(&format!(":{conflicts}\n")), "{err}");
    assert!(!out.contains("  kept "), "{out}");
    assert_eq!(files(lab), before, "nothing written");
    assert!(!lab.join("shots").exists());
    assert!(!lab.join("sessions").exists());
}

/// D6's first case, the one a phone meets on its first export: the lab knows the golfer, here under
/// another name and creation time. The lab's file stays byte for byte, the line names both fields
/// with both values, the rest of the export lands, and the corpus is still the phone's.
#[test]
fn a_golfer_of_the_same_id_and_handedness_is_kept_as_the_labs_and_says_what_differs() {
    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    let labs = golfer_file("aaron", "Aaron B", "right", "2026-11-02T10:05:00Z");
    store_golfers_file(lab.path(), "aaron.golfer.json", &labs);
    let want = line(
        "kept",
        &format!(
            "golfers/aaron.golfer.json: the store holds this golfer already, and it stays; the \
             export differs in display_name (\"Aaron\", stored \"Aaron B\"), created_at \
             ({PHONES_CREATED_AT}, stored 2026-11-02T10:05:00Z)"
        ),
    );

    let mut dry = request(phone.path(), lab.path());
    dry.dry_run = true;
    let (code, out, err) = run(&dry);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains(&want), "a dry run says so too: {out}");
    assert!(
        out.ends_with(
            "\n\nDry run: 6 file(s) would be copied; 0 identical, 1 left as stored, 0 ignored. \
             Nothing was written.\n"
        ),
        "{out}"
    );

    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains(&want), "{want:?}: {out}");
    assert!(
        out.ends_with("\n\n6 file(s) copied; 0 identical, 1 left as stored, 0 ignored.\n"),
        "{out}"
    );
    let mut expected = files(phone.path());
    expected.insert("golfers/aaron.golfer.json".to_string(), labs.into_bytes());
    assert_eq!(
        files(lab.path()),
        expected,
        "the phone's, but for the golfer"
    );
    assert_eq!(
        read_corpus(&lab.path().join("sessions"), "aaron", versions()),
        read_corpus(&phone.path().join("sessions"), "aaron", versions())
    );

    // A cumulative export meets the same file every time, and copies nothing for it.
    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains(&want), "{out}");
    assert!(
        out.ends_with("\n\n0 file(s) copied; 6 identical, 1 left as stored, 0 ignored.\n"),
        "{out}"
    );
}

/// Only the fields that differ are named, and a file can differ with none of them differing: its
/// spacing, a key the contract does not read, or one instant spelled in another offset.
#[test]
fn a_golfers_line_names_only_the_fields_that_differ_and_may_name_none() {
    let cases = [
        (
            golfer_file("aaron", "Aaron", "right", "2026-11-02T10:05:00Z"),
            format!(
                "the export differs in created_at ({PHONES_CREATED_AT}, stored \
                 2026-11-02T10:05:00Z)"
            ),
        ),
        (
            golfer_file("aaron", "Aaron B", "right", PHONES_CREATED_AT),
            "the export differs in display_name (\"Aaron\", stored \"Aaron B\")".to_string(),
        ),
        (
            golfer_file("aaron", "Aaron", "right", PHONES_CREATED_AT).replace(",\n   ", ","),
            "no field it reads differs, only the file's bytes".to_string(),
        ),
        (
            golfer_file(
                "aaron",
                "Aaron",
                "right",
                "2026-08-07T20:04:05.123456+05:00",
            ),
            "no field it reads differs, only the file's bytes".to_string(),
        ),
        (
            golfer_file("aaron", "Aaron", "right", PHONES_CREATED_AT)
                .replace('}', r#", "nickname": "Ace"}"#),
            "no field it reads differs, only the file's bytes".to_string(),
        ),
    ];
    for (labs, difference) in cases {
        let phone = Phone::new();
        let lab = tempfile::tempdir().unwrap();
        store_golfers_file(lab.path(), "aaron.golfer.json", &labs);
        let (code, out, err) = run(&request(phone.path(), lab.path()));
        assert_eq!((code, err.as_str()), (0, ""), "{labs}: {out}");
        let want = line(
            "kept",
            &format!(
                "golfers/aaron.golfer.json: the store holds this golfer already, and it stays; \
                 {difference}\n"
            ),
        );
        assert!(out.contains(&want), "{want:?}: {out}");
    }
}

/// D6's refusal: a different handedness is two people, or one of them recorded wrong, and so is
/// another id in the same file. Every other field agreeing does not rescue either.
#[test]
fn a_golfer_of_another_handedness_or_id_stays_a_conflict() {
    for labs in [
        golfer_file("aaron", "Aaron", "left", PHONES_CREATED_AT),
        golfer_file("dave", "Aaron", "right", PHONES_CREATED_AT),
    ] {
        let phone = Phone::new();
        let lab = tempfile::tempdir().unwrap();
        store_golfers_file(lab.path(), "aaron.golfer.json", &labs);
        refused_on(&phone, lab.path(), &["aaron.golfer.json"]);
    }
}

/// A golfer that does not read, on either side, cannot be called the same person. Unreadable is what
/// the store's reader says: torn, an older schema, an id that is no slug, a handedness that is no
/// handedness, or a directory at its name.
#[test]
fn a_golfer_unreadable_on_either_side_stays_a_conflict() {
    let good = golfer_file("aaron", "Aaron B", "right", PHONES_CREATED_AT);
    let unreadable = [
        "{\"player_id\": ".to_string(),
        r#"{"player_id": "aaron", "display_name": "Aaron", "handedness": "right"}"#.to_string(),
        golfer_file("Not A Slug", "Aaron", "right", PHONES_CREATED_AT),
        golfer_file("aaron", "Aaron", "sideways", PHONES_CREATED_AT),
    ];
    for bad in &unreadable {
        let phone = Phone::new();
        let lab = tempfile::tempdir().unwrap();
        store_golfers_file(lab.path(), "aaron.golfer.json", bad);
        refused_on(&phone, lab.path(), &["aaron.golfer.json"]);

        let phone = Phone::new();
        let lab = tempfile::tempdir().unwrap();
        store_golfers_file(lab.path(), "aaron.golfer.json", &good);
        fs::write(phone.path().join("golfers").join("aaron.golfer.json"), bad).unwrap();
        refused_on(&phone, lab.path(), &["aaron.golfer.json"]);
    }

    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    fs::create_dir_all(lab.path().join("golfers").join("aaron.golfer.json")).unwrap();
    refused_on(&phone, lab.path(), &["aaron.golfer.json"]);
}

/// D6's bag: the lab's stays byte for byte and the line says the phone's did not land, whatever
/// either side holds, since neither is read. Something that is not a file holds no bag to keep.
#[test]
fn a_bag_that_differs_is_kept_as_the_labs_and_says_the_phones_was_not_applied() {
    let want = line(
        "kept",
        "golfers/aaron.bag.json: the store holds this golfer's bag already, which differs, and it \
         stays; the phone's bag was not applied\n",
    );
    let phones_bag = read_json("spec/vectors/storage/stores/bag-reads.json")["input"]["files"]
        ["aaron.bag.json"]
        .as_str()
        .unwrap()
        .to_string();
    let mut retimed: Value = serde_json::from_str(&phones_bag).unwrap();
    retimed["updated_at"] = "2026-11-02T10:05:00Z".into();
    // A bag the contract reads, so this case is a bag that differs and not only a torn one.
    serde_json::from_value::<Bag>(retimed.clone()).unwrap();
    for labs in [retimed.to_string(), "{\"torn\": ".to_string()] {
        let phone = Phone::new();
        let lab = tempfile::tempdir().unwrap();
        store_golfers_file(lab.path(), "aaron.bag.json", &labs);
        let (code, out, err) = run(&request(phone.path(), lab.path()));
        assert_eq!((code, err.as_str()), (0, ""), "{out}");
        assert!(out.contains(&want), "{want:?}: {out}");
        assert!(
            out.ends_with("\n\n6 file(s) copied; 0 identical, 1 left as stored, 0 ignored.\n"),
            "{out}"
        );
        let mut expected = files(phone.path());
        expected.insert("golfers/aaron.bag.json".to_string(), labs.into_bytes());
        assert_eq!(files(lab.path()), expected, "the phone's, but for the bag");
        assert_eq!(
            read_corpus(&lab.path().join("sessions"), "aaron", versions()),
            read_corpus(&phone.path().join("sessions"), "aaron", versions())
        );
    }

    // A phone's bag that does not read costs nothing, because it is not applied.
    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    store_golfers_file(lab.path(), "aaron.bag.json", &phones_bag);
    fs::write(
        phone.path().join("golfers").join("aaron.bag.json"),
        "{\"torn\": ",
    )
    .unwrap();
    let (code, out, err) = run(&request(phone.path(), lab.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains(&want), "{want:?}: {out}");

    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    fs::create_dir_all(lab.path().join("golfers").join("aaron.bag.json")).unwrap();
    refused_on(&phone, lab.path(), &["aaron.bag.json"]);
}

/// The value rule reaches only what a store lists: a name directly under `golfers/` with a store's
/// suffix. Anything else in the tree goes by bytes, as it did before D6.
#[test]
fn a_file_no_golfers_store_lists_still_goes_by_bytes() {
    let names = ["notes.txt", "old/aaron.bag.json", "old/aaron.golfer.json"];
    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    let golfer = golfer_file("aaron", "Aaron", "right", PHONES_CREATED_AT);
    for name in names {
        let at = phone.path().join("golfers").join(name);
        fs::create_dir_all(at.parent().unwrap()).unwrap();
        fs::write(&at, &golfer).unwrap();
        store_golfers_file(lab.path(), name, &format!("{golfer}\n"));
    }
    refused_on(&phone, lab.path(), &names);
}

/// A golfer's or a bag's entry whose checksum fails is the torn export the module doc refuses, exit
/// 2, though the size alone already told `stored` it differs and neither rule would copy its bytes.
#[test]
fn a_torn_golfer_or_bag_entry_is_refused_though_neither_would_be_copied() {
    let golfer = golfer_file("aaron", "Aaron", "right", PHONES_CREATED_AT);
    let bag = r#"{"Aaron's": "bag"}"#.to_string();
    for (name, text) in [("aaron.golfer.json", golfer), ("aaron.bag.json", bag)] {
        let scratch = tempfile::tempdir().unwrap();
        let lab = tempfile::tempdir().unwrap();
        store_golfers_file(
            lab.path(),
            name,
            &golfer_file("aaron", "Aaron B", "right", PHONES_CREATED_AT),
        );
        let before = files(lab.path());
        let archive = scratch.path().join("export.zip");
        let mut zip = ZipWriter::new(fs::File::create(&archive).unwrap());
        zip.start_file(
            format!("golfers/{name}"),
            FullFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(text.as_bytes()).unwrap();
        zip.finish().unwrap();

        // The archive whole is kept, so what refuses below is the one byte and nothing else.
        let mut dry = request(&archive, lab.path());
        dry.dry_run = true;
        let (code, out, err) = run(&dry);
        assert_eq!((code, err.as_str()), (0, ""), "{name}: {out}");
        assert!(
            out.contains(&format!("\n  kept      golfers/{name}: ")),
            "{out}"
        );

        let mut bytes = fs::read(&archive).unwrap();
        let held = bytes.windows(5).position(|five| five == b"Aaron").unwrap();
        bytes[held] = b'B';
        fs::write(&archive, bytes).unwrap();
        let (code, out, err) = run(&request(&archive, lab.path()));
        assert_eq!(code, 2, "{name}: {out}{err}");
        assert!(
            err.starts_with(&format!(
                "error: nothing was imported, because the export cannot be read as a stored \
                 layout:\n  golfers/{name}: "
            )),
            "{err}"
        );
        assert_eq!(files(lab.path()), before, "nothing written");
    }
}

// ----------------------------------------------------------------------------------- the zip

/// The phone's tree zipped under one folder, deflated, each entry carrying its time in an
/// extended-timestamp field, with a Finder's `__MACOSX/` and `.DS_Store` beside it.
fn zip_of(tree: &Path, into: &Path) -> PathBuf {
    let archive = into.join("export.zip");
    let mut zip = ZipWriter::new(fs::File::create(&archive).unwrap());
    for (rel, bytes) in files(tree) {
        let seconds = modified(&tree.join(&rel))
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut options =
            FullFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut stamp = vec![1u8];
        stamp.extend(u32::try_from(seconds).unwrap().to_le_bytes());
        options.add_extra_data(0x5455, stamp, false).unwrap();
        zip.start_file(format!("processed/{rel}"), options).unwrap();
        zip.write_all(&bytes).unwrap();
    }
    for junk in ["processed/.DS_Store", "__MACOSX/processed/._golfers"] {
        zip.start_file(junk, FullFileOptions::default()).unwrap();
        zip.write_all(b"Finder's").unwrap();
    }
    zip.finish().unwrap();
    archive
}

/// A zip of the export imports as its directory does, read below its one folder, the photo's time
/// from the entry's field, and what no store reads ignored and counted.
#[test]
fn a_zip_imports_as_its_directory_does() {
    let phone = Phone::new();
    let scratch = tempfile::tempdir().unwrap();
    let archive = zip_of(phone.path(), scratch.path());
    let (from_zip, from_dir) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());

    let (code, out, err) = run(&request(&archive, from_zip.path()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.ends_with("\n\n7 file(s) copied; 0 identical, 0 left as stored, 2 ignored.\n"),
        "{out}"
    );
    assert_eq!(run(&request(phone.path(), from_dir.path())).0, 0);
    assert_eq!(files(from_zip.path()), files(from_dir.path()));
    let photo = photo_name(&phone.path().join("sessions"));
    assert_eq!(
        modified(
            &from_zip
                .path()
                .join("sessions")
                .join(SESSION)
                .join("1")
                .join(photo)
        ),
        UNIX_EPOCH + Duration::from_secs(TAKEN_AT)
    );
    assert_eq!(
        read_corpus(&from_zip.path().join("sessions"), "aaron", versions()),
        read_corpus(&phone.path().join("sessions"), "aaron", versions())
    );

    // And again from the zip: identical throughout.
    let (code, out, _) = run(&request(&archive, from_zip.path()));
    assert_eq!(code, 0);
    assert!(out.ends_with("\n\n0 file(s) copied; 7 identical, 0 left as stored, 2 ignored.\n"));
}

// ----------------------------------------------------------------------------------- refusals

/// What is not a stored layout this reads exits 2 with every reason, and writes nothing.
#[test]
fn what_is_not_a_stored_layout_is_refused_by_name() {
    let lab = tempfile::tempdir().unwrap();
    let refused = |export: &Path, lab: &Path| {
        let (code, out, err) = run(&request(export, lab));
        assert_eq!(code, 2, "{out}{err}");
        assert_eq!(fs::read_dir(lab).unwrap().count(), 0, "nothing written");
        err
    };

    // A file outside the three trees, a shot that is not one, and a manifest naming a file its
    // directory lacks, all listed in one refusal.
    let phone = Phone::new();
    fs::write(phone.path().join("notes.txt"), "what the phone also had").unwrap();
    let err = refused(phone.path(), lab.path());
    assert_eq!(
        err,
        "error: nothing was imported, because the export cannot be read as a stored layout:\n  \
         notes.txt: outside sessions/, shots/ and golfers/, which are all an export holds\n"
    );
    fs::remove_file(phone.path().join("notes.txt")).unwrap();
    fs::write(
        phone.path().join("shots").join("torn.shot.json"),
        "{\"shot_id\": ",
    )
    .unwrap();
    let photo = photo_name(&phone.path().join("sessions"));
    fs::remove_file(
        phone
            .path()
            .join("sessions")
            .join(SESSION)
            .join("1")
            .join(&photo),
    )
    .unwrap();
    let err = refused(phone.path(), lab.path());
    assert!(
        err.contains("\n  shots/torn.shot.json: not a shot the shot store reads ("),
        "{err}"
    );
    assert!(
        err.contains(&format!(
            "\n  sessions/{SESSION}/1: manifest.json names {photo}, which the export does not hold\n"
        )),
        "{err}"
    );

    // An archive entry climbing out of the layout, and one with a backslash in its name.
    let scratch = tempfile::tempdir().unwrap();
    let archive = scratch.path().join("evil.zip");
    let mut zip = ZipWriter::new(fs::File::create(&archive).unwrap());
    for name in ["sessions/../../escaped.json", "golfers\\aaron.golfer.json"] {
        zip.start_file(name, FullFileOptions::default()).unwrap();
        zip.write_all(b"{}").unwrap();
    }
    zip.finish().unwrap();
    let err = refused(&archive, lab.path());
    assert!(
        err.contains("\n  sessions/../../escaped.json: not a plain relative path\n"),
        "{err}"
    );
    assert!(
        err.contains("\n  golfers\\aaron.golfer.json: not a plain relative path\n"),
        "{err}"
    );

    // Nothing there, a file that is not a zip, and an export with nothing in its trees.
    let missing = scratch.path().join("no-such-export");
    assert!(refused(&missing, lab.path()).starts_with(&format!("error: {}: ", missing.display())));
    let text = scratch.path().join("export.zip.txt");
    fs::write(&text, "not an archive").unwrap();
    assert!(refused(&text, lab.path()).contains("neither a directory nor a zip archive this reads"));
    let empty = scratch.path().join("empty");
    fs::create_dir_all(empty.join("sessions").join(".incoming")).unwrap();
    fs::write(empty.join("sessions").join(".incoming").join("x.part"), "").unwrap();
    assert!(refused(&empty, lab.path())
        .contains("\n  it holds no file under sessions/, shots/ or golfers/\n"));
}

// ----------------------------------------------------------------------------------- the binary

fn golf_lab(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_golf-lab"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// The verb's flags, argparse's way, and one dry run through the binary over the three flags.
#[test]
fn the_binary_parses_as_argparse_and_imports_through_its_flags() {
    for (args, problem) in [
        (
            &["import-export"][..],
            "import-export: the following arguments are required: EXPORT",
        ),
        (
            &["import-export", "a", "b"][..],
            "import-export: unrecognized arguments: b",
        ),
        (
            &["import-export", "a", "--dry-run=yes"][..],
            "import-export: argument --dry-run: ignored explicit argument \"yes\"",
        ),
        (
            &["import-export", "a", "--shots-dir"][..],
            "import-export: argument --shots-dir: expected one argument",
        ),
        (
            &["import-export", "a", "--force"][..],
            "import-export: unrecognized arguments: --force",
        ),
    ] {
        let (code, _, err) = golf_lab(args);
        assert_eq!(code, Some(2), "{args:?}");
        assert!(
            err.starts_with(&format!("golf-lab: {problem}\n")),
            "{args:?}: {err}"
        );
    }
    let (code, out, _) = golf_lab(&["import-export", "--help"]);
    assert_eq!(code, Some(0));
    assert!(out.contains(
        "golf-lab import-export EXPORT [--sessions-dir D] [--shots-dir D] [--golfers-dir D] \
         [--dry-run]"
    ));
    assert!(!out.contains("arrives"), "every verb has arrived: {out}");

    let phone = Phone::new();
    let lab = tempfile::tempdir().unwrap();
    let dir = |leaf: &str| lab.path().join(leaf).display().to_string();
    let (code, out, err) = golf_lab(&[
        "import-export",
        &phone.path().display().to_string(),
        "--dry-run",
        "--sessions-dir",
        &dir("sessions"),
        "--shots-dir",
        &dir("shots"),
        "--golfers-dir",
        &dir("golfers"),
    ]);
    assert_eq!((code, err.as_str()), (Some(0), ""), "{out}");
    assert!(
        out.contains(&format!("\n  shots into {}\n", dir("shots"))),
        "{out}"
    );
    assert!(out.ends_with("Nothing was written.\n"), "{out}");
    assert_eq!(fs::read_dir(lab.path()).unwrap().count(), 0);
}
