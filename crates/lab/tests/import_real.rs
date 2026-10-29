//! `golf-lab import-photos`, the binary, over a stored bay photo with the real reader. [M29 P24]
//!
//! `tests/import_photos.rs` drives every branch with a replaying reader; this runs what a user
//! runs, the built binary with `crates/ocr`'s models, on a **scratch copy** of one stored photo
//! (`2026-08-23/2`'s JPEG, whose committed screen vector is `spec/vectors/screen/corpus/2026-08-23-2.json`),
//! so nothing under `data/processed/` is written (the M29 plan's rule for P24). In order:
//!
//! 1. the store-only import reads it (`parsed`), and its shot holds the vector's values, its
//!    `fields_present` and its `needs_review` (the plan's D7, which P7's gate already holds the
//!    reader to; here it is held through the verb);
//! 2. the same run again is `cached`, and a second copy of the photo under another name is too:
//!    the store dedupes by the photo's digest;
//! 3. `--into-session` into a scratch session, over a copy of the golfer registry and a store of its
//!    own, reads the entry's copy, writes its `shot_analysis.json`, and Rust's `read_corpus` pools
//!    the entry.
//!
//! Needs `data/processed/` (gitignored) and `data/models/ocr/`, so it is ignored, and it fails
//! rather than passing when either is missing. Two reads, about 50 s in a debug build.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use contracts::shot::ShotData;
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use serde_json::Value;
use storage::corpus::{read_corpus, EngineVersions};
use storage::shot_store::{hash_image, ShotStore};
use storage::state::load_shot_analysis;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn golf_lab(args: &[&Path]) -> (Option<i32>, String, String) {
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

/// The analysed values and the two flags D7 holds identical, as JSON so a missing key reads.
fn held(shot: &ShotData) -> Value {
    let mut value = serde_json::to_value(shot).unwrap();
    let provenance = value["provenance"].take();
    let object = value.as_object_mut().unwrap();
    for identity in ["shot_id", "session_id", "timestamp"] {
        object.remove(identity);
    }
    object.insert(
        "fields_present".into(),
        provenance["fields_present"].clone(),
    );
    object.insert("needs_review".into(), provenance["needs_review"].clone());
    value
}

#[test]
#[ignore = "needs data/processed/ (gitignored) and data/models/ocr/'s three ONNX files"]
fn a_stored_photo_imports_through_the_binary_and_its_entry_is_pooled() {
    let stored = repo().join("data/processed/sessions/2026-08-23/2/shot_screen.15cf1710a1a7.jpg");
    assert!(stored.is_file(), "{} is missing", stored.display());
    let scratch = tempfile::tempdir().unwrap();
    let path = |relative: &str| scratch.path().join(relative);
    fs::create_dir_all(path("photos")).unwrap();
    let photo = path("photos/shot-2.jpg");
    fs::copy(&stored, &photo).unwrap();
    let digest = hash_image(&fs::read(&photo).unwrap());

    let vector: Value = serde_json::from_str(
        &fs::read_to_string(repo().join("spec/vectors/screen/corpus/2026-08-23-2.json")).unwrap(),
    )
    .unwrap();
    let expected: ShotData = serde_json::from_value(vector["expected"]["shot"].clone()).unwrap();

    // 1. A new import: read, stored, the vector's values.
    let shots = path("shots");
    let store_only = |extra: &[&Path]| {
        let mut args: Vec<&Path> = vec![Path::new("import-photos")];
        args.extend_from_slice(extra);
        args.extend([
            Path::new("--session"),
            Path::new("bay"),
            Path::new("--out"),
            &shots,
        ]);
        golf_lab(&args)
    };
    let (code, out, err) = store_only(&[&photo]);
    assert_eq!(code, Some(0), "{out}\n{err}");
    assert!(
        out.contains("  shot-2.jpg                   parsed  conf="),
        "{out}"
    );
    let shot = ShotStore::new(&shots)
        .get(&digest)
        .unwrap()
        .expect("stored");
    assert_eq!(shot.shot_id, format!("bay-{}", &digest[..12]));
    assert_eq!(held(&shot), held(&expected));

    // 2. Again, and under another name: cached, both.
    let renamed = path("photos/again.jpg");
    fs::copy(&photo, &renamed).unwrap();
    let (code, out, err) = store_only(&[&path("photos")]);
    assert_eq!(code, Some(0), "{out}\n{err}");
    assert_eq!(out.matches(" cached  conf=").count(), 2, "{out}");
    assert_eq!(ShotStore::new(&shots).all().len(), 1);

    // 3. Into a scratch session, with a store of its own so the entry's copy is read.
    let golfers = path("golfers");
    fs::create_dir_all(&golfers).unwrap();
    for entry in fs::read_dir(repo().join("data/processed/golfers")).unwrap() {
        let entry = entry.unwrap().path();
        fs::copy(&entry, golfers.join(entry.file_name().unwrap())).unwrap();
    }
    let (sessions, entry_shots) = (path("sessions"), path("entry-shots"));
    let (code, out, err) = golf_lab(&[
        Path::new("import-photos"),
        &photo,
        Path::new("--into-session"),
        Path::new("2026-10-23"),
        Path::new("--player"),
        Path::new("Aaron"),
        Path::new("--club"),
        Path::new("7i"),
        Path::new("--sessions-dir"),
        &sessions,
        Path::new("--golfers-dir"),
        &golfers,
        Path::new("--out"),
        &entry_shots,
    ]);
    assert_eq!(code, Some(0), "{out}\n{err}");
    assert!(
        out.contains("      2026-10-23/1: wrote shot_analysis.json (fundamentals): "),
        "{out}"
    );
    let entry_shot = ShotStore::new(&entry_shots).get(&digest).unwrap().unwrap();
    assert_eq!(entry_shot.shot_id, "2026-10-23-1");
    assert_eq!(held(&entry_shot), held(&expected));
    let result = load_shot_analysis(&sessions.join("2026-10-23").join("1")).unwrap();
    assert_eq!(result.photo_sha256.as_deref(), Some(digest.as_str()));
    let corpus = read_corpus(
        &sessions,
        "aaron",
        EngineVersions {
            installed: ANALYSIS_VERSION,
            comparable_from: COMPARABLE_FROM,
        },
    );
    assert!(corpus.excluded.is_empty(), "{:?}", corpus.excluded);
    assert_eq!(corpus.swings.len(), 1);
    assert_eq!(
        corpus.swings[0].shot_sha256.as_deref(),
        Some(digest.as_str())
    );
}
