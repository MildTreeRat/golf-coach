//! `golf-lab import-photos` against a reader that replays a committed screen vector: the store-only
//! import, `--into-session`'s photo entries, and the exit codes. [M29 P24]
//!
//! The reader is `tests/shot_lookup.rs`'s [`Replay`] again: `spec/vectors/screen/corpus/2026-08-23-2.json`'s
//! boxes and notes, which frozen PaddleOCR read off that bay photo, so the parse, the validation and
//! the stamp are `crates/screen`'s real ones and the "photos" here are stand-in bytes no decoder
//! opens. These run in `cargo test` on any machine; `tests/import_real.rs` runs the binary over a
//! stored photo with the models.
//!
//! The store-only cases name the line of `import_shot_screens.py` or `importer.py::import_screen`
//! they port and check the printed table as text, because a lab run must read as the script's did.
//! The `--into-session` cases have no Python twin, and end where the plan's P24 says they must: on
//! an entry Rust's `read_corpus` admits.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use contracts::shot::{ShotData, SCREEN_PARSER_VERSION};
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::Timestamp;
use golf_lab::import::{import_photos, IntoSession, Request};
use golf_lab::shot::{import_photo, Imported, NewImport, PhotoReader, Prepared, ShotOptions};
use screen::profile::{load_profile, DeviceProfile};
use screen::ScreenInput;
use serde_json::Value;
use storage::bag_store::BagStore;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::{read_corpus, EngineVersions};
use storage::manifest::Role;
use storage::shot_store::{hash_image, ShotStore};
use storage::state::load_shot_analysis;

const PHOTO: &[u8] = b"a stand-in for a bay photo, which only the reader would open";
const OTHER_PHOTO: &[u8] = b"a second stand-in, so a second photo has a digest of its own";
const SESSION: &str = "2026-10-23";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(relative: &str) -> Value {
    let path = repo().join(relative);
    serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap()
}

/// The vector's input and its expected shot.
fn vector() -> (ScreenInput, ShotData) {
    let document = read_json("spec/vectors/screen/corpus/2026-08-23-2.json");
    let input: ScreenInput = serde_json::from_value(document["input"].clone()).unwrap();
    let shot = serde_json::from_value(document["expected"]["shot"].clone()).unwrap();
    (input, shot)
}

fn now() -> Timestamp {
    "2026-10-23T08:00:00Z".parse().unwrap()
}

/// A reader replaying the vector's boxes, or refusing as it is told, counting its reads.
struct Replay {
    ready: Result<(), String>,
    prepared: Result<Prepared, String>,
    prepares: u32,
}

impl Replay {
    fn vector() -> Self {
        let (input, _) = vector();
        Self::answering(Ok(Prepared {
            boxes: input.boxes,
            notes: input.notes,
        }))
    }

    fn answering(prepared: Result<Prepared, String>) -> Self {
        Replay {
            ready: Ok(()),
            prepared,
            prepares: 0,
        }
    }

    /// No box at all, so the parse is empty: Python's `failed`.
    fn blank() -> Self {
        Self::answering(Ok(Prepared {
            boxes: Vec::new(),
            notes: Vec::new(),
        }))
    }
}

impl PhotoReader for Replay {
    fn ready(&mut self) -> Result<(), String> {
        self.ready.clone()
    }

    fn prepare(&mut self, _photo: &Path, profile: &DeviceProfile) -> Result<Prepared, String> {
        assert_eq!(profile.device, "hd_golf");
        self.prepares += 1;
        self.prepared.clone()
    }
}

/// A scratch root: `photos/` with the stand-ins, an empty `shots/`, and `golfers/` holding `aaron`
/// (right-handed) with the bag the storage family records for him, whose 7i has a loft.
struct Scratch {
    root: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let photos = root.path().join("photos");
        fs::create_dir_all(&photos).unwrap();
        fs::write(photos.join("shot-1.jpg"), PHOTO).unwrap();
        let golfers = root.path().join("golfers");
        fs::create_dir_all(&golfers).unwrap();
        fs::write(
            golfers.join("aaron.golfer.json"),
            r#"{"player_id": "aaron", "display_name": "Aaron", "handedness": "right",
                "created_at": "2026-08-07T15:04:05.123456Z"}"#,
        )
        .unwrap();
        let bags = read_json("spec/vectors/storage/stores/bag-reads.json");
        fs::write(
            BagStore::new(&golfers).path_for("aaron"),
            bags["input"]["files"]["aaron.bag.json"].as_str().unwrap(),
        )
        .unwrap();
        Scratch { root }
    }

    /// `relative` under the root, joined a component at a time so it is spelled as the verb
    /// spells a path (`\` on Windows), which is what the printed lines are compared with.
    fn path(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(self.root.path().to_path_buf(), |path, part| path.join(part))
    }

    fn photo(&self) -> PathBuf {
        self.path("photos/shot-1.jpg")
    }

    fn options(&self) -> ShotOptions {
        ShotOptions::new(self.path("shots"))
    }

    fn store(&self) -> ShotStore {
        ShotStore::new(self.path("shots"))
    }

    fn sessions(&self) -> SwingBundleStore {
        SwingBundleStore::new(self.path("sessions"))
    }

    fn request(&self, paths: &[PathBuf], edit: impl FnOnce(&mut Request)) -> Request {
        let mut request = Request {
            paths: paths.to_vec(),
            session: Some("bay".to_string()),
            options: self.options(),
            into: None,
        };
        edit(&mut request);
        request
    }

    fn entries(&self, player: &str, club: &str) -> IntoSession {
        IntoSession {
            session_id: SESSION.to_string(),
            player: player.to_string(),
            club: club.to_string(),
            sessions_dir: self.path("sessions"),
            golfers_dir: self.path("golfers"),
        }
    }
}

/// One run: the exit code, stdout and stderr.
fn run(request: &Request, reader: &mut dyn PhotoReader) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = import_photos(request, reader, now(), &mut out, &mut err).unwrap();
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// The table's row for the vector's shot, as `_format_row` prints it.
fn vector_row(name: &str, status: &str) -> String {
    format!(
        "  {name:<28} {status:<7} conf=0.88\n      carry=33.6 total=69.4 ball=83.8 club=93.7 \
         smash=0.89 path=-7.7 face=7.6"
    )
}

fn profile() -> &'static DeviceProfile {
    load_profile("hd_golf").unwrap()
}

// ---------------------------------------------------------------------------- import_photo

/// `import_screen`'s new import: read, stamped `{session}-{digest[:12]}` with the path as given,
/// stored under the photo's digest; then `cached` with no read, and `force` reads it again.
#[test]
fn a_new_photo_is_parsed_and_stored_then_cached_and_force_reads_it_again() {
    let scratch = Scratch::new();
    let (_, expected) = vector();
    let digest = hash_image(PHOTO);
    let mut reader = Replay::vector();
    let new = NewImport {
        session_id: "bay",
        shot_id: None,
    };

    let first = import_photo(
        &scratch.photo(),
        new,
        profile(),
        &scratch.options(),
        &mut reader,
    )
    .unwrap();
    assert_eq!(first.status(), "parsed");
    let Imported::Read {
        shot,
        replacing: None,
        written: Some(written),
    } = &first
    else {
        panic!("a new import that was written: {first:?}");
    };
    assert_eq!(written, &scratch.store().path_for(&digest));
    assert_eq!(shot.shot_id, format!("bay-{}", &digest[..12]));
    assert_eq!(shot.session_id, "bay");
    let provenance = shot.provenance.as_ref().unwrap();
    assert_eq!(provenance.image_sha256.as_deref(), Some(digest.as_str()));
    assert_eq!(
        provenance.image_path.as_deref(),
        Some(scratch.photo().display().to_string().as_str())
    );
    assert_eq!(provenance.parser_version, SCREEN_PARSER_VERSION);
    assert_eq!(shot.carry_distance, expected.carry_distance);
    assert_eq!(
        provenance.fields_present,
        expected.provenance.unwrap().fields_present
    );
    assert_eq!(
        scratch.store().get(&digest).unwrap().as_ref(),
        Some(&**shot)
    );

    let again = import_photo(
        &scratch.photo(),
        new,
        profile(),
        &scratch.options(),
        &mut reader,
    )
    .unwrap();
    assert_eq!(again.status(), "cached");
    assert_eq!(again.shot(), Some(&**shot));
    assert_eq!(
        reader.prepares, 1,
        "a current stored shot is not read again"
    );

    let forced = ShotOptions {
        force_ocr: true,
        ..scratch.options()
    };
    let other = NewImport {
        session_id: "range",
        shot_id: None,
    };
    let read = import_photo(&scratch.photo(), other, profile(), &forced, &mut reader).unwrap();
    assert_eq!(read.status(), "parsed", "--force is a new import");
    assert_eq!(reader.prepares, 2);
    assert_eq!(
        read.shot().unwrap().shot_id,
        format!("range-{}", &digest[..12])
    );
}

/// The shot store's stale shot, which Python serves as `cached`, is re-read keeping its identity:
/// `shot_for`'s rule.
#[test]
fn a_stale_stored_shot_is_re_read_and_keeps_its_identity() {
    let scratch = Scratch::new();
    let digest = hash_image(PHOTO);
    let mut stale: ShotData = vector().1;
    stale.shot_id = "2026-08-23-2".to_string();
    stale.session_id = "2026-08-23".to_string();
    let provenance = stale.provenance.as_mut().unwrap();
    provenance.parser_version = 0;
    provenance.fields_present = None;
    provenance.image_sha256 = Some(digest.clone());
    provenance.image_path = Some("C:\\bay\\shot-2.jpg".to_string());
    scratch.store().put(&stale).unwrap();

    let mut reader = Replay::vector();
    let new = NewImport {
        session_id: "bay",
        shot_id: None,
    };
    let read = import_photo(
        &scratch.photo(),
        new,
        profile(),
        &scratch.options(),
        &mut reader,
    )
    .unwrap();
    assert_eq!(read.status(), "re-read");
    let Imported::Read {
        shot,
        replacing: Some(replacing),
        written: Some(_),
    } = &read
    else {
        panic!("a re-read that was written: {read:?}");
    };
    assert_eq!(**replacing, stale);
    assert_eq!(
        (shot.shot_id.as_str(), shot.session_id.as_str()),
        ("2026-08-23-2", "2026-08-23")
    );
    assert_eq!(shot.timestamp, stale.timestamp);
    let provenance = shot.provenance.as_ref().unwrap();
    assert_eq!(
        provenance.image_path.as_deref(),
        Some("C:\\bay\\shot-2.jpg")
    );
    assert_eq!(provenance.parser_version, SCREEN_PARSER_VERSION);
    assert_eq!(
        scratch.store().get(&digest).unwrap().as_ref(),
        Some(&**shot)
    );
}

/// `dry_run=True`: read, answered `dry-run`, nothing written. A read of nothing is `failed` and
/// leaves the store as it was; a reader's error is the reader's message, unwrapped.
#[test]
fn a_dry_run_writes_nothing_and_a_blank_or_broken_read_stores_nothing() {
    let scratch = Scratch::new();
    let new = NewImport {
        session_id: "bay",
        shot_id: None,
    };
    let dry = ShotOptions {
        dry_run: true,
        ..scratch.options()
    };
    let read = import_photo(
        &scratch.photo(),
        new,
        profile(),
        &dry,
        &mut Replay::vector(),
    )
    .unwrap();
    assert_eq!(read.status(), "dry-run");
    assert!(!scratch.path("shots").exists(), "a dry run writes nothing");

    let blank = import_photo(
        &scratch.photo(),
        new,
        profile(),
        &scratch.options(),
        &mut Replay::blank(),
    )
    .unwrap();
    assert_eq!((blank.status(), blank.shot()), ("failed", None));
    assert!(scratch.store().all().is_empty());

    let why = "could not read image x.jpg: it is a progressive JPEG, and only baseline ...";
    let broken = import_photo(
        &scratch.photo(),
        new,
        profile(),
        &scratch.options(),
        &mut Replay::answering(Err(why.to_string())),
    );
    assert_eq!(broken, Err(why.to_string()));
    let missing = import_photo(
        &scratch.path("photos/gone.jpg"),
        new,
        profile(),
        &scratch.options(),
        &mut Replay::vector(),
    )
    .unwrap_err();
    assert!(missing.starts_with("could not read image "), "{missing}");
}

// ---------------------------------------------------------------------- the store-only verb

/// The script's run, line for line: the header, a row per photo found (a directory's sorted, a
/// non-photo warned about), the store, and the last line; then the same run again is `cached`.
#[test]
fn the_store_only_run_prints_the_scripts_table_and_exits_zero() {
    let scratch = Scratch::new();
    fs::write(scratch.path("photos/notes.txt"), "not a photo").unwrap();
    fs::write(scratch.path("photos/Shot-0.PNG"), OTHER_PHOTO).unwrap();
    let request = scratch.request(
        &[scratch.path("photos"), scratch.path("photos/notes.txt")],
        |_| {},
    );
    let mut reader = Replay::vector();
    let (code, out, err) = run(&request, &mut reader);
    let store = scratch.path("shots");
    assert_eq!(
        out,
        format!(
            "\nImporting 2 image(s) as session 'bay' [hd_golf]\n{}\n{}\n\nStore: {}\n\n2 shot(s) \
             imported cleanly.\n",
            vector_row("Shot-0.PNG", "parsed"),
            vector_row("shot-1.jpg", "parsed"),
            store.display()
        )
    );
    assert_eq!(
        err,
        format!(
            "warning: skipping {} (not a JPEG, PNG or HEIC photo)\n",
            scratch.path("photos/notes.txt").display()
        )
    );
    assert_eq!(code, 0);
    assert_eq!(scratch.store().all().len(), 2);

    let (code, out, _) = run(&request, &mut reader);
    assert_eq!(code, 0);
    assert!(out.contains(&vector_row("shot-1.jpg", "cached")), "{out}");
    assert_eq!(reader.prepares, 2, "the second run read nothing");
}

/// `--session` absent is `screens-{today}` in UTC; `--dry-run` prints no store line and writes
/// nothing.
#[test]
fn the_default_session_is_todays_and_a_dry_run_names_no_store() {
    let scratch = Scratch::new();
    let request = scratch.request(&[scratch.photo()], |request| {
        request.session = None;
        request.options.dry_run = true;
    });
    let (code, out, _) = run(&request, &mut Replay::vector());
    assert_eq!(code, 0);
    assert!(
        out.starts_with("\nImporting 1 image(s) as session 'screens-2026-10-23' [hd_golf]\n"),
        "{out}"
    );
    assert!(out.contains(&vector_row("shot-1.jpg", "dry-run")));
    assert!(!out.contains("Store:"), "{out}");
    assert!(!scratch.path("shots").exists());
}

/// A path given with `/` and a `.` (Git Bash passes `C:/...`) is spelled as Python's `str(Path)`
/// spells it, in the table's store line and in the `image_path` the shot is stamped with.
#[test]
fn a_path_is_spelled_as_pythons_str_path() {
    let scratch = Scratch::new();
    let root = scratch.root.path().display().to_string();
    let given = PathBuf::from(format!("{root}/photos/./shot-1.jpg"));
    let request = scratch.request(&[given], |request| {
        request.options.shots_dir = PathBuf::from(format!("{root}/shots/."));
    });
    let (code, out, _) = run(&request, &mut Replay::vector());
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(&format!("\nStore: {}\n", scratch.path("shots").display())),
        "{out}"
    );
    let shot = scratch.store().get(&hash_image(PHOTO)).unwrap().unwrap();
    assert_eq!(
        shot.provenance.unwrap().image_path,
        Some(scratch.photo().display().to_string())
    );
}

/// Exit 1 for a photo read as nothing, a flagged shot (listed with its warnings), no photo found,
/// and a reader that cannot run; exit 2 for a profile nothing names, with the `KeyError`'s `repr`.
#[test]
fn the_scripts_exit_codes() {
    let scratch = Scratch::new();
    let request = scratch.request(&[scratch.photo()], |_| {});

    let (code, out, _) = run(&request, &mut Replay::blank());
    assert_eq!(code, 1);
    assert!(
        out.contains("  shot-1.jpg                   failed\n"),
        "{out}"
    );
    assert!(
        out.ends_with("\n1 unreadable, 0 flagged for review.\n"),
        "{out}"
    );

    let (code, out, err) = run(
        &request,
        &mut Replay::answering(Err("could not read image: it is broken".to_string())),
    );
    assert_eq!(code, 1);
    assert!(
        out.ends_with("\n1 unreadable, 0 flagged for review.\n"),
        "{out}"
    );
    assert_eq!(err, "error: could not read image: it is broken\n");

    let strict = scratch.request(&[scratch.photo()], |request| {
        request.options.min_confidence = 0.95;
    });
    let (code, out, _) = run(&strict, &mut Replay::vector());
    assert_eq!(code, 1);
    let review = format!("  {} needs review:\n    - ", scratch.photo().display());
    assert!(out.contains(&review), "{out}");
    assert!(out.contains("conf=0.88  REVIEW\n"), "{out}");
    assert!(
        out.ends_with("\n0 unreadable, 1 flagged for review.\n"),
        "{out}"
    );

    let nothing = scratch.request(&[scratch.path("empty")], |_| {});
    fs::create_dir_all(scratch.path("empty")).unwrap();
    let (code, out, err) = run(&nothing, &mut Replay::vector());
    assert_eq!((code, out.as_str()), (1, ""));
    assert_eq!(
        err,
        format!(
            "error: no images found in {}\n",
            scratch.path("empty").display()
        )
    );

    let unknown = scratch.request(&[scratch.photo()], |request| {
        request.options.device = "trackman".to_string();
    });
    let (code, _, err) = run(&unknown, &mut Replay::vector());
    assert_eq!(code, 2);
    assert!(
        err.starts_with("error: \"Unknown launch-monitor profile 'trackman'. Known profiles: "),
        "{err}"
    );

    let mut not_ready = Replay::vector();
    not_ready.ready = Err("model det: data/models/ocr/det.onnx is missing".to_string());
    let (code, out, err) = run(&request, &mut not_ready);
    assert_eq!((code, out.as_str()), (1, ""));
    assert_eq!(
        err,
        "error: model det: data/models/ocr/det.onnx is missing\n"
    );
    assert_eq!(not_ready.prepares, 0, "it fails before any photo");
}

// ------------------------------------------------------------------------- --into-session

fn versions() -> EngineVersions {
    EngineVersions {
        installed: ANALYSIS_VERSION,
        comparable_from: COMPARABLE_FROM,
    }
}

/// The photo becomes entry `S/1`, golfer and club the flags', its shot stamped `{S}-1` off the
/// entry's own copy, its `shot_analysis.json` written, and Rust's corpus pools it.
#[test]
fn into_a_session_the_photo_is_an_entry_the_corpus_admits() {
    let scratch = Scratch::new();
    let request = scratch.request(&[scratch.photo()], |request| {
        request.session = None;
        request.into = Some(scratch.entries("Aaron", "7 iron"));
    });
    let (code, out, err) = run(&request, &mut Replay::vector());
    assert_eq!(err, "");
    assert_eq!(code, 0, "{out}");
    assert!(
        out.starts_with(
            "\nImporting 1 image(s) into session '2026-10-23' as photo entries (golfer aaron, \
             club 7i) [hd_golf]\n"
        ),
        "{out}"
    );
    assert!(out.contains(&vector_row("shot-1.jpg", "parsed")), "{out}");
    assert!(
        out.contains("\n      2026-10-23/1: wrote shot_analysis.json (fundamentals): "),
        "{out}"
    );
    assert!(out.contains(&format!(
        "\nSession: {}\n",
        scratch.path("sessions").join(SESSION).display()
    )));

    let manifest = scratch.sessions().get_swing(SESSION, "1").expect("entry 1");
    assert_eq!(manifest.player_id.as_deref(), Some("aaron"));
    assert_eq!(manifest.club.map(|club| club.as_str()), Some("7i"));
    assert_eq!(manifest.roles.len(), 1, "a photo and no clip");
    let photo = &manifest.roles[&Role::ShotScreen];
    assert_eq!(photo.content_sha256, hash_image(PHOTO));
    assert_eq!(photo.original_filename, "shot-1.jpg");
    assert_eq!(photo.content_type, "image/jpeg");
    let copy = scratch
        .path("sessions")
        .join(SESSION)
        .join("1")
        .join(&photo.filename);
    assert_eq!(fs::read(&copy).unwrap(), PHOTO);
    assert_eq!(
        fs::metadata(&copy).unwrap().modified().unwrap(),
        fs::metadata(scratch.photo()).unwrap().modified().unwrap(),
        "the copy keeps the photo's time, which is the shot's"
    );
    assert!(fs::read_dir(scratch.path("sessions/.incoming"))
        .unwrap()
        .next()
        .is_none());

    let shot = scratch.store().get(&hash_image(PHOTO)).unwrap().unwrap();
    assert_eq!(
        (shot.shot_id.as_str(), shot.session_id.as_str()),
        ("2026-10-23-1", SESSION)
    );
    assert_eq!(
        shot.provenance.unwrap().image_path,
        Some(copy.display().to_string())
    );
    let result = load_shot_analysis(&scratch.path("sessions").join(SESSION).join("1"))
        .expect("shot_analysis.json");
    assert_eq!(
        result.photo_sha256.as_deref(),
        Some(hash_image(PHOTO).as_str())
    );

    let corpus = read_corpus(&scratch.path("sessions"), "aaron", versions());
    assert!(corpus.excluded.is_empty(), "{:?}", corpus.excluded);
    let [entry] = corpus.swings.as_slice() else {
        panic!("one entry pooled: {:?}", corpus.swings);
    };
    assert_eq!(
        (entry.session_id.as_str(), entry.swing_id.as_str()),
        (SESSION, "1")
    );
    assert_eq!(entry.face_on_sha256, None);
    assert_eq!(
        entry.shot_sha256.as_deref(),
        Some(hash_image(PHOTO).as_str())
    );
}

/// The same photo again is skipped, under any name; `--force` reads it again and re-analyses the
/// entry, opening no second one.
#[test]
fn a_photo_the_session_holds_is_skipped_unless_forced() {
    let scratch = Scratch::new();
    fs::write(scratch.path("photos/renamed.png"), PHOTO).unwrap();
    let into = |force| {
        scratch.request(
            &[scratch.photo(), scratch.path("photos/renamed.png")],
            |request| {
                request.session = None;
                request.options.force_ocr = force;
                request.into = Some(scratch.entries("aaron", "7i"));
            },
        )
    };
    let mut reader = Replay::vector();
    let (code, out, _) = run(&into(false), &mut reader);
    assert_eq!(code, 0);
    assert!(
        out.contains(&format!(
        "{}\n      2026-10-23/1: skipped: this session already holds the photo (--force reads it \
         again and re-analyses the entry)\n",
        vector_row("renamed.png", "skipped")
    )),
        "{out}"
    );
    assert_eq!(reader.prepares, 1);

    let (code, out, _) = run(&into(true), &mut reader);
    assert_eq!(code, 0, "{out}");
    assert_eq!(reader.prepares, 3, "both names read again");
    assert_eq!(
        out.matches("2026-10-23/1: wrote shot_analysis.json")
            .count(),
        2
    );
    let swings: Vec<String> = scratch
        .sessions()
        .get_session(SESSION)
        .into_iter()
        .map(|manifest| manifest.swing_id)
        .collect();
    assert_eq!(swings, ["1"]);

    // Forced again, and nothing is read this time: the entry and its result stay as they were.
    let (code, out, _) = run(&into(true), &mut Replay::blank());
    assert_eq!(code, 1);
    assert!(
        out.contains(
            "      2026-10-23/1: left as it was, since no shot was read from the photo this time\n"
        ),
        "{out}"
    );
    let entry = scratch.path("sessions").join(SESSION).join("1");
    assert!(load_shot_analysis(&entry).is_some());
    assert!(scratch.store().get(&hash_image(PHOTO)).unwrap().is_some());
}

/// A session holding a filmed swing with no photo yet: the photo opens entry 2 by name rather than
/// being slotted into swing 1, which the store's automatic rule would do; a swing's own photo is
/// never touched, forced or not.
#[test]
fn an_entry_is_opened_by_name_and_a_swings_photo_is_left_alone() {
    let scratch = Scratch::new();
    let swing = scratch.path("sessions").join(SESSION).join("1");
    fs::create_dir_all(&swing).unwrap();
    let clip_sha = hash_image(b"a face-on clip");
    let manifest = serde_json::json!({
        "swing_id": "1", "session_id": SESSION,
        "created_at": "2026-10-23T07:00:00Z", "updated_at": "2026-10-23T07:00:00Z",
        "roles": {"face_on": {
            "role": "face_on", "filename": format!("face_on.{}.MOV", &clip_sha[..12]),
            "content_sha256": clip_sha, "original_filename": "front.MOV",
            "content_type": "video/quicktime", "size_bytes": 14,
            "received_at": "2026-10-23T07:00:00Z", "warnings": []}},
        "player_id": "aaron", "club": "7i"
    });
    fs::write(swing.join("manifest.json"), manifest.to_string()).unwrap();

    let request = scratch.request(&[scratch.photo()], |request| {
        request.session = None;
        request.into = Some(scratch.entries("aaron", "7i"));
    });
    let (code, out, _) = run(&request, &mut Replay::vector());
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("      2026-10-23/2: wrote shot_analysis.json"),
        "{out}"
    );
    let swing_one = scratch.sessions().get_swing(SESSION, "1").unwrap();
    assert!(
        !swing_one.roles.contains_key(&Role::ShotScreen),
        "swing 1 untouched"
    );

    // Now make swing 2's photo swing 1's, and a forced import leaves it alone.
    let mut filmed = scratch.sessions().get_swing(SESSION, "2").unwrap();
    filmed
        .roles
        .insert(Role::FaceOn, swing_one.roles[&Role::FaceOn].clone());
    fs::write(
        scratch
            .path("sessions")
            .join(SESSION)
            .join("2")
            .join("manifest.json"),
        serde_json::to_string(&filmed).unwrap(),
    )
    .unwrap();
    let forced = scratch.request(&[scratch.photo()], |request| {
        request.session = None;
        request.options.force_ocr = true;
        request.into = Some(scratch.entries("aaron", "7i"));
    });
    let mut reader = Replay::vector();
    let (code, out, _) = run(&forced, &mut reader);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(
            "      2026-10-23/2: skipped: this session already holds the photo, in a swing with a \
         face-on clip, whose shot is read and analysed with it\n"
        ),
        "{out}"
    );
    assert_eq!(reader.prepares, 0);
}

/// A photo nothing is read from opens no entry, and a session this run made and left empty goes
/// with it; a dry run opens, writes and analyses nothing.
#[test]
fn an_unread_photo_opens_no_entry_and_a_dry_run_opens_none() {
    let scratch = Scratch::new();
    let request = scratch.request(&[scratch.photo()], |request| {
        request.session = None;
        request.into = Some(scratch.entries("aaron", "7i"));
    });
    let (code, out, _) = run(&request, &mut Replay::blank());
    assert_eq!(code, 1);
    assert!(out.contains(
        "  shot-1.jpg                   failed\n      2026-10-23: no photo entry opened, since no \
         shot was read from the photo\n"
    ), "{out}");
    assert!(
        out.ends_with("\n1 unreadable, 0 flagged for review, 0 not analysed.\n"),
        "{out}"
    );
    assert!(!scratch.path("sessions").join(SESSION).exists());

    let dry = scratch.request(&[scratch.photo()], |request| {
        request.session = None;
        request.options.dry_run = true;
        request.into = Some(scratch.entries("aaron", "7i"));
    });
    let (code, out, _) = run(&dry, &mut Replay::vector());
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(&format!(
            "{}\n      2026-10-23: would open a photo entry for it (a dry run writes nothing)\n",
            vector_row("shot-1.jpg", "dry-run")
        )),
        "{out}"
    );
    assert!(!scratch.path("sessions").join(SESSION).exists());
    assert!(!scratch.path("shots").exists());
}

/// The flags' refusals, each exit 2 before any photo: a golfer the registry lacks (named with who
/// it holds), a name with no id in it, text that is not a club, and a session id that is a path.
#[test]
fn into_session_refuses_an_unknown_golfer_a_non_club_and_a_path() {
    let scratch = Scratch::new();
    for (into, says) in [
        (
            scratch.entries("Aron", "7i"),
            format!(
                "error: golfer 'aron' is not in the registry at {} (it holds aaron)\n",
                scratch.path("golfers").display()
            ),
        ),
        (
            scratch.entries("!!", "7i"),
            "error: name '!!' has no usable characters for an id\n".to_string(),
        ),
        (
            scratch.entries("aaron", "wedge"),
            "error: 'wedge' is not a club. Try '7i', '7 iron', 'driver' or 'pw'.\n'wedge' and \
             'iron' name a category rather than a club, so neither is accepted.\n"
                .to_string(),
        ),
        (
            IntoSession {
                session_id: "../elsewhere".to_string(),
                ..scratch.entries("aaron", "7i")
            },
            "error: --into-session takes a session id, one directory name such as 2026-10-23, \
             not '../elsewhere'\n"
                .to_string(),
        ),
    ] {
        let request = scratch.request(&[scratch.photo()], |request| request.into = Some(into));
        let mut reader = Replay::vector();
        let (code, out, err) = run(&request, &mut reader);
        assert_eq!((code, out.as_str(), err.as_str()), (2, "", says.as_str()));
        assert_eq!(reader.prepares, 0);
    }
    assert!(!scratch.path("sessions").exists());
}

// ---------------------------------------------------------------------------- the binary

fn golf_lab(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_golf-lab"))
        .args(args)
        .output()
        .expect("golf-lab runs");
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// What argparse refuses exits 2 with the usage, `--help` exits 0, and a models directory with no
/// models is the reader's refusal, exit 1, before any photo is read.
#[test]
fn the_binary_parses_as_argparse_and_refuses_missing_models() {
    let (code, out, _) = golf_lab(&["import-photos", "--help"]);
    assert_eq!(code, Some(0));
    assert!(out.starts_with("usage: golf-lab import-photos"), "{out}");

    for (args, says) in [
        (
            &["import-photos", "--colour"][..],
            "unrecognized arguments: --colour",
        ),
        (
            &["import-photos", "--session"][..],
            "argument --session: expected one argument",
        ),
        (
            &["import-photos", "--min-confidence", "high"][..],
            "argument --min-confidence: invalid float value: 'high'",
        ),
        (
            &["import-photos", "--force=yes"][..],
            "ignored explicit argument \"yes\"",
        ),
        (
            &["import-photos", "--into-session", "s", "--player", "aaron"][..],
            "--into-session S, --player P and --club C go together",
        ),
        (
            &["import-photos", "--club", "7i"][..],
            "--into-session S, --player P and --club C go together",
        ),
        (
            &[
                "import-photos",
                "--into-session=s",
                "--player=a",
                "--club=7i",
                "--session=t",
            ][..],
            "argument --session: not allowed with argument --into-session",
        ),
        (
            &["import-photos", "--golfers-dir", "g"][..],
            "--sessions-dir and --golfers-dir go with --into-session",
        ),
    ] {
        let (code, out, err) = golf_lab(args);
        assert_eq!(code, Some(2), "{args:?}: {err}");
        assert!(out.is_empty(), "{args:?}");
        assert!(err.contains(says), "{args:?}: {err}");
        assert!(err.contains("usage: golf-lab import-photos"), "{args:?}");
    }

    let scratch = Scratch::new();
    let empty = scratch.path("no-models");
    fs::create_dir_all(&empty).unwrap();
    let photo = scratch.photo();
    let out_dir = scratch.path("shots");
    let (code, out, err) = golf_lab(&[
        "import-photos",
        photo.to_str().unwrap(),
        "--out",
        out_dir.to_str().unwrap(),
        "--models",
        empty.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(out, "");
    assert!(err.starts_with("error: "), "{err}");
    assert!(!out_dir.exists());
}
