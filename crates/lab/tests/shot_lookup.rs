//! `shot_for` against a reader that replays a committed screen vector: every branch of the shot
//! lookup. [M29 P23]
//!
//! The reader is [`Replay`], which answers with `spec/vectors/screen/corpus/2026-08-23-2.json`'s
//! boxes and notes, the ones frozen PaddleOCR read off that bay photo. So the parse, the validation
//! and the stamp are `crates/screen`'s real ones, and a re-read's shot can be held to the vector's
//! `expected.shot` exactly, with no model, no photo and no OCR here: these run in `cargo test` on
//! any machine. The models and the stored photos are `tests/import_real.rs`' and
//! `crates/ocr/tests/read.rs`', and the stored shots `tests/reread.rs`'.
//!
//! Each case names the line of `api/pipeline.py::_shot_for` it ports, or says it is the stale
//! branch Python has no twin of, and checks the log and the note as text, because the Python
//! branches are Python's wording and a lab run must read the same.

use std::fs;
use std::path::{Path, PathBuf};

use contracts::shot::{ShotData, SCREEN_PARSER_VERSION};
use golf_lab::shot::{
    older_shot_note, shot_for, OcrReader, Origin, PhotoReader, Prepared, ShotFor, ShotOptions,
};
use screen::profile::DeviceProfile;
use screen::ScreenInput;
use serde_json::Value;
use storage::manifest::SwingManifest;
use storage::shot_store::{hash_image, ShotStore};
use storage::StoreError;

const PHOTO_BYTES: &[u8] = b"a stand-in for the bay photo, which only the reader would open";
/// The swing the bundle is filed as. Not the vector's `2`, so a new import's stamp
/// (`2026-08-23-7`) and a kept identity (`2026-08-23-2`) can be told apart.
const SWING: &str = "7";
const SESSION: &str = "2026-08-23";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The vector's input and its expected shot.
fn vector() -> (ScreenInput, Value) {
    let path = repo().join("spec/vectors/screen/corpus/2026-08-23-2.json");
    let document: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let input: ScreenInput = serde_json::from_value(document["input"].clone()).unwrap();
    (input, document["expected"]["shot"].clone())
}

/// A reader replaying the vector's boxes, or refusing as it is told, counting what it was asked.
struct Replay {
    ready: Result<(), String>,
    prepared: Result<Prepared, String>,
    readies: u32,
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
            readies: 0,
            prepares: 0,
        }
    }

    fn not_ready(why: &str) -> Self {
        Replay {
            ready: Err(why.to_string()),
            ..Self::vector()
        }
    }
}

impl PhotoReader for Replay {
    fn ready(&mut self) -> Result<(), String> {
        self.readies += 1;
        self.ready.clone()
    }

    fn prepare(&mut self, _photo: &Path, profile: &DeviceProfile) -> Result<Prepared, String> {
        assert_eq!(
            profile.device, "hd_golf",
            "read with the configured profile"
        );
        self.prepares += 1;
        self.prepared.clone()
    }
}

/// A scratch root holding one bundle with a stand-in photo, and an empty shot store.
struct Bundle {
    _root: tempfile::TempDir,
    swing_dir: PathBuf,
    shots_dir: PathBuf,
    manifest: SwingManifest,
}

impl Bundle {
    fn new() -> Self {
        Self::with_manifest_sha(&hash_image(PHOTO_BYTES))
    }

    /// A bundle whose manifest records `sha` for the photo, whatever the photo's bytes hash to.
    fn with_manifest_sha(sha: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let swing_dir = root.path().join("sessions").join(SESSION).join(SWING);
        fs::create_dir_all(&swing_dir).unwrap();
        let filename = format!("shot_screen.{}.jpg", &sha[..12]);
        fs::write(swing_dir.join(&filename), PHOTO_BYTES).unwrap();
        let manifest = serde_json::from_value(serde_json::json!({
            "swing_id": SWING, "session_id": SESSION,
            "created_at": "2026-08-23T05:29:40.000000Z", "updated_at": "2026-08-23T05:29:50.000000Z",
            "roles": {"shot_screen": {
                "role": "shot_screen", "filename": filename, "content_sha256": sha,
                "original_filename": "shot-2.jpg", "content_type": "image/jpeg",
                "size_bytes": PHOTO_BYTES.len(), "received_at": "2026-08-23T05:29:50.000000Z",
                "warnings": []}}
        }))
        .unwrap();
        Bundle {
            shots_dir: root.path().join("shots"),
            swing_dir,
            manifest,
            _root: root,
        }
    }

    fn sha(&self) -> String {
        self.manifest
            .roles
            .values()
            .next()
            .unwrap()
            .content_sha256
            .clone()
    }

    fn photo(&self) -> PathBuf {
        let role = self.manifest.roles.values().next().unwrap();
        self.swing_dir.join(&role.filename)
    }

    fn store(&self) -> ShotStore {
        ShotStore::new(&self.shots_dir)
    }

    fn options(&self) -> ShotOptions {
        ShotOptions::new(&self.shots_dir)
    }

    /// The vector's shot as a store would hold it under this bundle's photo, read by `version`:
    /// at 0 frozen Python's, unstamped and with no `fields_present`, and one value off, so a
    /// re-read that served it instead would show.
    fn stored(&self, version: i64) -> ShotData {
        self.stored_under(&self.sha(), version)
    }

    /// [`Self::stored`], filed under `sha` rather than the manifest's.
    fn stored_under(&self, sha: &str, version: i64) -> ShotData {
        let mut record = expected(sha);
        let provenance = record["provenance"].as_object_mut().unwrap();
        provenance.insert("parser_version".into(), version.into());
        if version == 0 {
            provenance.insert("fields_present".into(), Value::Null);
        }
        record["carry_distance"] = 99.9.into();
        let shot: ShotData = serde_json::from_value(record).unwrap();
        self.store().put(&shot).unwrap();
        shot
    }

    fn run(&self, options: &ShotOptions, reader: &mut dyn PhotoReader) -> (ShotFor, Vec<String>) {
        let mut lines = Vec::new();
        let answer = shot_for(
            &self.swing_dir,
            &self.manifest,
            options,
            reader,
            &mut |line| lines.push(line.to_string()),
        )
        .unwrap();
        (answer, lines)
    }

    fn store_bytes(&self) -> Option<Vec<u8>> {
        fs::read(self.store().path_for(&self.sha())).ok()
    }
}

/// The vector's expected shot, filed under `sha`: what a re-read keeping the vector's identity is.
fn expected(sha: &str) -> Value {
    let (_, mut shot) = vector();
    shot["provenance"]["image_sha256"] = sha.into();
    shot
}

fn json(shot: &ShotData) -> Value {
    serde_json::to_value(shot).unwrap()
}

const STALE_LINE: &str = "  shot_screen: 2026-08-23-2 in the shot store was read by an \
                          unversioned screen parser, re-reading with screen parser 1";

#[test]
fn the_stale_line_names_this_builds_parser() {
    assert_eq!(SCREEN_PARSER_VERSION, 1, "STALE_LINE spells the version");
}

#[test]
fn a_bundle_with_no_shot_screen_attaches_nothing_and_asks_no_reader() {
    // `if role_file is None: return None, "no shot-screen photo in this bundle …"`.
    let mut bundle = Bundle::new();
    bundle.manifest.roles.clear();
    let mut reader = Replay::vector();
    let (answer, lines) = bundle.run(&bundle.options(), &mut reader);
    assert_eq!(answer.shot, None);
    assert_eq!(answer.origin, Origin::Nothing);
    assert_eq!(
        answer.note.as_deref(),
        Some("no shot-screen photo in this bundle — no launch-monitor data to attach")
    );
    assert!(lines.is_empty());
    assert_eq!(reader.readies, 0);
}

#[test]
fn a_current_stored_shot_is_served_and_no_photo_is_read() {
    // `cached = store.get(sha); if cached is not None: log(…from the shot store); return cached`,
    // now only for a parse as new as this parser.
    let bundle = Bundle::new();
    let stored = bundle.stored(SCREEN_PARSER_VERSION);
    let mut reader = Replay::vector();
    let (answer, lines) = bundle.run(&bundle.options(), &mut reader);
    assert_eq!(answer.shot, Some(stored));
    assert_eq!(answer.origin, Origin::Stored);
    assert_eq!(answer.note, None);
    assert_eq!(lines, ["  shot_screen: 2026-08-23-2 from the shot store"]);
    assert_eq!((reader.readies, reader.prepares), (0, 0));
}

#[test]
fn a_stale_stored_shot_is_re_read_keeping_its_identity_and_stamped() {
    // The branch Python has no twin of: an unstamped shot re-read from the bundle's photo, filed
    // under the stored shot's id, session, time and path rather than this bundle's.
    let bundle = Bundle::new();
    let stale = bundle.stored(0);
    let mut reader = Replay::vector();
    let (answer, lines) = bundle.run(&bundle.options(), &mut reader);
    let shot = answer.shot.expect("the re-read");
    assert_eq!(
        json(&shot),
        expected(&bundle.sha()),
        "the vector's shot, its identity kept"
    );
    assert_eq!(
        shot.provenance.as_ref().unwrap().parser_version,
        SCREEN_PARSER_VERSION
    );
    assert_eq!(answer.note, None);
    assert_eq!(
        answer.origin,
        Origin::Read {
            replacing: Some(Box::new(stale)),
            written: Some(bundle.store().path_for(&bundle.sha())),
        }
    );
    assert_eq!(
        bundle.store().get(&bundle.sha()).unwrap(),
        Some(shot),
        "stored"
    );
    assert_eq!(
        lines,
        [
            STALE_LINE,
            "  shot_screen: reading shot-2.jpg ...",
            "  shot_screen: parsed"
        ]
    );
    // A second run finds it current.
    let (again, lines) = bundle.run(&bundle.options(), &mut reader);
    assert_eq!(again.origin, Origin::Stored);
    assert_eq!(lines, ["  shot_screen: 2026-08-23-2 from the shot store"]);
    assert_eq!(reader.prepares, 1);
}

#[test]
fn a_dry_run_re_reads_and_writes_nothing() {
    let bundle = Bundle::new();
    let stale = bundle.stored(0);
    let before = bundle.store_bytes();
    let options = ShotOptions {
        dry_run: true,
        ..bundle.options()
    };
    let (answer, lines) = bundle.run(&options, &mut Replay::vector());
    assert_eq!(json(answer.shot.as_ref().unwrap()), expected(&bundle.sha()));
    assert_eq!(
        answer.origin,
        Origin::Read {
            replacing: Some(Box::new(stale)),
            written: None,
        }
    );
    assert_eq!(
        lines.last().map(String::as_str),
        Some("  shot_screen: dry-run")
    );
    assert_eq!(bundle.store_bytes(), before, "the store is untouched");
}

#[test]
fn a_stale_shot_that_cannot_be_re_read_is_kept_with_a_note_saying_why() {
    // Each of Python's ways a photo goes unread, met with an older parser's shot in the store:
    // the shot is kept, as `keypoints_for` keeps an older pose run, and the note names the parser
    // and the reason.
    let skipped = |bundle: &Bundle| {
        let options = ShotOptions {
            skip_ocr: true,
            ..bundle.options()
        };
        bundle.run(&options, &mut Replay::vector())
    };
    let missing = |bundle: &Bundle| {
        fs::remove_file(bundle.photo()).unwrap();
        bundle.run(&bundle.options(), &mut Replay::vector())
    };
    let not_ready =
        |bundle: &Bundle| bundle.run(&bundle.options(), &mut Replay::not_ready("no models"));
    let raised = |bundle: &Bundle| {
        let mut reader = Replay::answering(Err("the photo would not decode".to_string()));
        bundle.run(&bundle.options(), &mut reader)
    };
    type Case<'a> = (
        &'a dyn Fn(&Bundle) -> (ShotFor, Vec<String>),
        String,
        &'a [&'a str],
    );
    let read_line = "  shot_screen: reading shot-2.jpg ...";
    let cases: [Case; 4] = [
        (
            &skipped,
            "shot-screen OCR was skipped by request".into(),
            &[STALE_LINE],
        ),
        (
            &missing,
            "shot-screen photo shot_screen.{sha12}.jpg is missing from the swing directory".into(),
            &[STALE_LINE],
        ),
        (&not_ready, "no models".into(), &[STALE_LINE]),
        (
            &raised,
            "the photo would not decode".into(),
            &[STALE_LINE, read_line],
        ),
    ];
    for (run, because, log) in cases {
        let bundle = Bundle::new();
        let stale = bundle.stored(0);
        let before = bundle.store_bytes();
        let because = because.replace("{sha12}", &bundle.sha()[..12]);
        let (answer, lines) = run(&bundle);
        assert_eq!(answer.shot.as_ref(), Some(&stale), "{because}");
        assert_eq!(answer.origin, Origin::KeptOlder, "{because}");
        assert_eq!(answer.note, Some(older_shot_note(&stale, &because)));
        assert_eq!(lines, log, "{because}");
        assert_eq!(
            bundle.store_bytes(),
            before,
            "{because}: the store is untouched"
        );
    }
}

#[test]
fn a_machine_with_no_models_says_which_and_opens_them_once() {
    // `build_recognizer` raising `MissingOCRExtra`, which is a missing or mismatched model here.
    let bundle = Bundle::new();
    let empty = tempfile::tempdir().unwrap();
    let mut reader = OcrReader::new(empty.path());
    let (answer, lines) = bundle.run(&bundle.options(), &mut reader);
    let note = answer.note.unwrap();
    assert!(
        note.starts_with("no launch-monitor data attached: ") && note.contains(" is missing."),
        "{note}"
    );
    assert!(note.contains(&empty.path().display().to_string()), "{note}");
    assert_eq!((answer.shot, answer.origin), (None, Origin::Nothing));
    assert!(lines.is_empty(), "{lines:?}");
    assert!(reader.opened(), "asked once");
    let (again, _) = bundle.run(&bundle.options(), &mut reader);
    assert_eq!(
        again.note.as_deref(),
        Some(note.as_str()),
        "the refusal is remembered"
    );
    assert_eq!(reader.reads(), 0);
}

#[test]
fn a_re_read_of_nothing_attaches_nothing_and_leaves_the_older_file() {
    // `if shot is None: return None, "the shot screen could not be read (failed) …"`, which holds
    // for a stale shot too: the reader read this photo and found nothing in it.
    let bundle = Bundle::new();
    let stale = bundle.stored(0);
    let before = bundle.store_bytes();
    let mut reader = Replay::answering(Ok(Prepared {
        boxes: Vec::new(),
        notes: Vec::new(),
    }));
    let (answer, lines) = bundle.run(&bundle.options(), &mut reader);
    assert_eq!(answer.shot, None);
    assert_eq!(
        answer.note.as_deref(),
        Some("the shot screen could not be read (failed) — no numbers attached")
    );
    assert_eq!(
        answer.origin,
        Origin::Read {
            replacing: Some(Box::new(stale)),
            written: None,
        }
    );
    assert_eq!(lines, [STALE_LINE, "  shot_screen: reading shot-2.jpg ..."]);
    assert_eq!(bundle.store_bytes(), before);
}

#[test]
fn a_store_miss_is_pythons_every_branch() {
    let note_of = |bundle: &Bundle, options: &ShotOptions, reader: &mut dyn PhotoReader| {
        let (answer, _) = bundle.run(options, reader);
        assert_eq!((&answer.shot, &answer.origin), (&None, &Origin::Nothing));
        answer.note.unwrap()
    };
    let bundle = Bundle::new();
    // `if options.skip_ocr: …`
    let skip = ShotOptions {
        skip_ocr: true,
        ..bundle.options()
    };
    assert_eq!(
        note_of(&bundle, &skip, &mut Replay::vector()),
        "shot-screen OCR skipped by request — no launch-monitor data attached"
    );
    // `except (MissingOCRExtra, ValueError) as exc: …`
    assert_eq!(
        note_of(
            &bundle,
            &bundle.options(),
            &mut Replay::not_ready("no models")
        ),
        "no launch-monitor data attached: no models"
    );
    // The broad `except Exception as exc`, around the read and `load_profile` both.
    let (answer, lines) = bundle.run(
        &bundle.options(),
        &mut Replay::answering(Err("the photo would not decode".to_string())),
    );
    assert_eq!(
        answer.note.as_deref(),
        Some("the shot screen could not be read — no numbers attached: the photo would not decode")
    );
    assert_eq!(lines, ["  shot_screen: reading shot-2.jpg ..."]);
    let unknown = ShotOptions {
        device: "nope".to_string(),
        ..bundle.options()
    };
    let (answer, _) = bundle.run(&unknown, &mut Replay::vector());
    assert_eq!(
        answer.note.as_deref(),
        Some(
            "the shot screen could not be read — no numbers attached: Unknown launch-monitor \
             profile 'nope'. Known profiles: hd_golf"
        )
    );
    // `if not image_path.exists(): …`
    fs::remove_file(bundle.photo()).unwrap();
    assert_eq!(
        note_of(&bundle, &bundle.options(), &mut Replay::vector()),
        format!(
            "shot-screen photo shot_screen.{}.jpg is missing from the swing directory",
            &bundle.sha()[..12]
        )
    );
}

#[test]
fn a_store_miss_is_read_as_a_new_import_stamped_from_this_bundle() {
    // `import_screen(…, session_id=manifest.session_id, shot_id=f"{session}-{swing}")`: the id is
    // this bundle's, the time the photo's, the path the one read.
    let bundle = Bundle::new();
    let mut reader = Replay::vector();
    let (answer, lines) = bundle.run(&bundle.options(), &mut reader);
    let shot = answer.shot.unwrap();
    let mut want = expected(&bundle.sha());
    want["shot_id"] = format!("{SESSION}-{SWING}").into();
    want["timestamp"] =
        serde_json::to_value(ocr::screen_timestamp(&bundle.photo()).unwrap()).unwrap();
    want["provenance"]["image_path"] = bundle.photo().display().to_string().into();
    assert_eq!(json(&shot), want);
    assert_eq!(
        answer.origin,
        Origin::Read {
            replacing: None,
            written: Some(bundle.store().path_for(&bundle.sha())),
        }
    );
    assert_eq!(answer.note, None);
    assert_eq!(
        lines,
        [
            "  shot_screen: reading shot-2.jpg ...",
            "  shot_screen: parsed"
        ]
    );
}

#[test]
fn force_ocr_reads_even_a_current_shot_as_a_new_import_and_keeps_nothing_on_failure() {
    // `if not options.force_ocr: cached = …`: the store is not consulted, so there is no older shot
    // to keep, as `keypoints_for(force=True)` has no superseded run.
    let bundle = Bundle::new();
    bundle.stored(SCREEN_PARSER_VERSION);
    let force = ShotOptions {
        force_ocr: true,
        ..bundle.options()
    };
    let (answer, lines) = bundle.run(&force, &mut Replay::vector());
    let shot = answer.shot.unwrap();
    assert_eq!(shot.shot_id, format!("{SESSION}-{SWING}"));
    assert!(matches!(
        answer.origin,
        Origin::Read {
            replacing: None,
            ..
        }
    ));
    assert_eq!(
        lines,
        [
            "  shot_screen: reading shot-2.jpg ...",
            "  shot_screen: parsed"
        ]
    );

    let (answer, _) = bundle.run(&force, &mut Replay::not_ready("no models"));
    assert_eq!(answer.shot, None);
    assert_eq!(
        answer.note.as_deref(),
        Some("no launch-monitor data attached: no models")
    );
}

#[test]
fn a_photo_whose_bytes_are_not_the_manifests_is_a_new_import_and_noted() {
    // Python reads the file and files the shot under its digest without a word. Here the stale
    // shot under the manifest's digest is left alone, the read is stamped as a new import, and the
    // note says the photo changed.
    let recorded = "0".repeat(64);
    let bundle = Bundle::with_manifest_sha(&recorded);
    let stale = bundle.stored(0);
    let before = bundle.store_bytes();
    let (answer, _) = bundle.run(&bundle.options(), &mut Replay::vector());
    let digest = hash_image(PHOTO_BYTES);
    let shot = answer.shot.unwrap();
    assert_eq!(shot.shot_id, format!("{SESSION}-{SWING}"));
    assert_eq!(
        shot.provenance.as_ref().unwrap().image_sha256.as_deref(),
        Some(digest.as_str())
    );
    assert_eq!(answer.note, Some(hash_note(&recorded)));
    assert_eq!(
        answer.origin,
        Origin::Read {
            replacing: None,
            written: Some(bundle.store().path_for(&digest)),
        }
    );
    assert_eq!(
        bundle.store_bytes(),
        before,
        "the stale shot's file is untouched"
    );
    assert_ne!(stale.shot_id, shot.shot_id);
}

/// The note for a photo whose bytes are not the `recorded` digest its manifest holds.
fn hash_note(recorded: &str) -> String {
    format!(
        "the shot-screen photo on disk hashes to {}, not to the {recorded} its manifest records, \
         so it was read as a new photo",
        hash_image(PHOTO_BYTES)
    )
}

#[test]
fn a_photo_whose_bytes_have_a_current_shot_of_their_own_is_pythons_cached() {
    // `import_screen`'s own lookup, under the bytes' digest: `if not force: cached =
    // store.get(digest); if cached is not None: return "cached", cached`, and `_shot_for` logs the
    // status. The stale shot under the manifest's digest is another photo's, and is not kept.
    let recorded = "0".repeat(64);
    let bundle = Bundle::with_manifest_sha(&recorded);
    bundle.stored(0);
    let current = bundle.stored_under(&hash_image(PHOTO_BYTES), SCREEN_PARSER_VERSION);
    let mut reader = Replay::vector();
    let (answer, lines) = bundle.run(&bundle.options(), &mut reader);
    assert_eq!(answer.shot, Some(current));
    assert_eq!(answer.origin, Origin::Stored);
    assert_eq!(answer.note, Some(hash_note(&recorded)));
    assert_eq!(
        lines,
        [
            STALE_LINE,
            "  shot_screen: reading shot-2.jpg ...",
            "  shot_screen: cached"
        ]
    );
    assert_eq!(reader.prepares, 0);
}

#[test]
fn a_photo_whose_bytes_have_a_stale_shot_of_their_own_re_reads_it_keeping_its_identity() {
    let recorded = "0".repeat(64);
    let bundle = Bundle::with_manifest_sha(&recorded);
    let digest = hash_image(PHOTO_BYTES);
    let stale = bundle.stored_under(&digest, 0);
    let (answer, lines) = bundle.run(&bundle.options(), &mut Replay::vector());
    assert_eq!(json(answer.shot.as_ref().unwrap()), expected(&digest));
    assert_eq!(answer.note, Some(hash_note(&recorded)));
    assert_eq!(
        answer.origin,
        Origin::Read {
            replacing: Some(Box::new(stale)),
            written: Some(bundle.store().path_for(&digest)),
        }
    );
    assert_eq!(
        lines,
        [
            "  shot_screen: reading shot-2.jpg ...",
            STALE_LINE,
            "  shot_screen: parsed"
        ]
    );
}

#[test]
fn a_stored_file_that_is_not_a_shot_stops_the_bundle() {
    // `ShotStore.get` raising out of `model_validate_json`, outside every `except` in `_shot_for`.
    let bundle = Bundle::new();
    fs::create_dir_all(&bundle.shots_dir).unwrap();
    fs::write(bundle.store().path_for(&bundle.sha()), "{not json").unwrap();
    let outcome = shot_for(
        &bundle.swing_dir,
        &bundle.manifest,
        &bundle.options(),
        &mut Replay::vector(),
        &mut |_| {},
    );
    assert!(
        matches!(outcome, Err(StoreError::Unreadable { .. })),
        "{outcome:?}"
    );
}

#[test]
fn a_store_that_cannot_be_written_is_the_broad_except() {
    // `store.put(shot)` sits inside `import_screen`, inside the broad `except`: the shot was read
    // and is not attached, which is Python's answer, ported as it stands.
    let bundle = Bundle::new();
    fs::write(
        &bundle.shots_dir,
        "a file where the store's directory should be",
    )
    .unwrap();
    let (answer, lines) = bundle.run(&bundle.options(), &mut Replay::vector());
    assert_eq!(answer.shot, None);
    let note = answer.note.unwrap();
    assert!(
        note.starts_with("the shot screen could not be read — no numbers attached: "),
        "{note}"
    );
    assert_eq!(lines, ["  shot_screen: reading shot-2.jpg ..."]);
}
