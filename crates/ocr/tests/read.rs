//! The OCR gate (the M29 plan's D7): the Rust reader end to end on the 13 bay photos, held to the
//! shots frozen PaddleOCR's boxes parse as (M29 P7).
//!
//! Each committed screen vector (`spec/vectors/screen/corpus/<shot>.json`) holds one stored bay
//! photo's boxes as frozen `prepare_screen` and PaddleOCR found them, and the shot
//! `crates/screen`'s reader parses from them (`expected.shot`, re-recorded through the Rust reader
//! at M34 P10). The gate reads the photo the vector names with [`ocr::read_photo`] (this crate's
//! decode, quad, rectification, vote and recognizer, nothing of frozen Python's), parses the boxes
//! and notes it returns through [`screen::read`] with the vector's own identity and threshold, and
//! compares the two shots.
//!
//! The photo is the vector's `input.image_path`, checked against its `input.image_sha256`, rather
//! than looked up through its bundle's manifest: the vector names the file and its digest, so the
//! digest is what proves the photo is the one the boxes were read from.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ocr::models::Models;
use screen::profile::load_profile;
use screen::ScreenInput;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// The 13 corpus vectors, in file-name order.
fn corpus_vectors(root: &Path) -> Vec<(String, Value)> {
    let dir = root
        .join("spec")
        .join("vectors")
        .join("screen")
        .join("corpus");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let id = path.file_stem().unwrap().to_string_lossy().into_owned();
            (id, json(&path))
        })
        .collect()
}

/// A shot's values: every key but `provenance`, which D7 reads two keys of.
fn values(shot: &Value) -> Value {
    let mut values = shot.as_object().unwrap().clone();
    values.remove("provenance");
    Value::Object(values)
}

/// A value this crate reads differently from the committed vector, on purpose and by name.
struct DeclaredDifference {
    /// The vector's shot, its file stem.
    shot: &'static str,
    /// The shot's key.
    key: &'static str,
    /// What this crate reads, as JSON.
    ours: &'static str,
    /// What the committed vector holds, as JSON.
    vector: &'static str,
}

impl DeclaredDifference {
    fn holds(&self, ours: &Value, vector: &Value) -> bool {
        serde_json::from_str::<Value>(self.ours).unwrap() == *ours
            && serde_json::from_str::<Value>(self.vector).unwrap() == *vector
    }
}

/// The one value D5's reader reads differently from the five-model reading the vectors hold.
///
/// On 2026-08-10-1 frozen PaddleOCR's UVDoc unwarping reads the `Impact Position V` label as
/// `Impact Position`, so the vector's parse saw two labels tie and withheld both tiles; this crate
/// reads the `V`, so the tie is gone and the tile under `Impact Position` reads `CENTER`, which is
/// what the photo shows (its `V` tile is `---`). `tests/recognizer.rs`'s `UVDOC_MOVES_VALUES`
/// names the same photo one stage earlier. **Decided by the user on 2026-10-21** (the M29 plan's
/// P7, option (a)): the reader stays without UVDoc rather than gaining it to reproduce a misread,
/// and the committed screen vector stays as it is, since this plan leaves the screen family alone.
/// The lab's re-read of this photo carries the difference into the shot store (the plan's P25).
const DECLARED_DIFFERENCES: [DeclaredDifference; 1] = [DeclaredDifference {
    shot: "2026-08-10-1",
    key: "impact_position",
    ours: r#""CENTER""#,
    vector: "null",
}];

fn strings(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

/// The D7 gate on the 13 bay photos.
///
/// **What it holds, per photo** (D7): the shot read from this crate's boxes has **every value** of
/// the vector's expected shot, its **`fields_present`** and its **`needs_review`**. A read of
/// nothing where the vector read a shot, or the reverse, fails. Per-box confidences, box geometry
/// and the warnings' text may differ, so `parse_confidence`, `raw_fields` and `warnings` are
/// printed for the plan's finding and held to nothing.
///
/// **And `prepare_screen`'s own answer**: the notes [`ocr::read_photo`] hands the parser are the
/// vector's `input.notes`, word for word. They say whether the screen was found (P2's quad gate is
/// exact on all 13) and whether the vote turned the photo or found it illegible, so a change in any
/// of the three shows here, and they lead the shot's warnings.
///
/// **Less what it names** ([`DECLARED_DIFFERENCES`]): one value on 2026-08-10-1, where frozen
/// PaddleOCR's UVDoc unwarping, which D5 leaves out, misread a label and this crate does not. A
/// declared difference is held exactly, both sides of it, and the gate fails if one is not seen,
/// so a reader that comes to agree with the vector, or disagrees in another way, says so. Every
/// other value, and `fields_present`, `needs_review` and the notes, agree on all 13.
#[test]
#[ignore = "needs the 13 bay photos in data/processed/, ffmpeg 7.1 or later (GOLF_FFMPEG or PATH) and the ONNX models in data/models/ocr/"]
fn every_bay_photo_reads_as_the_shot_frozen_paddleocrs_boxes_parse_as() {
    let root = repo_root();
    let mut models = Models::open(&root.join(ocr::DEFAULT_DIR)).unwrap();
    let vectors = corpus_vectors(&root);
    assert_eq!(vectors.len(), 13, "the 13 stored bay photos");
    let mut failures = Vec::new();
    let mut declared_seen = BTreeSet::new();
    let mut heif_photos = 0;
    for (id, vector) in &vectors {
        let input = &vector["input"];
        let expected = &vector["expected"];
        let path = root.join(input["image_path"].as_str().unwrap());
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{id}: {}: {e}", path.display()));
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            input["image_sha256"].as_str().unwrap(),
            "{id}: {} is not the photo the vector was read from",
            path.display()
        );
        heif_photos += usize::from(ocr::decode::is_heif(&path));

        let profile = load_profile(input["device"].as_str().unwrap()).unwrap();
        let prepared =
            ocr::read_photo(&mut models, &path, profile).unwrap_or_else(|e| panic!("{id}: {e}"));
        let mut screen_input: ScreenInput = serde_json::from_value(input.clone()).unwrap();
        let frozen_notes = std::mem::take(&mut screen_input.notes);
        screen_input.boxes = prepared.boxes.clone();
        screen_input.notes = prepared.notes.clone();
        if prepared.notes != frozen_notes {
            failures.push(format!(
                "{id}: notes {:?}, frozen {frozen_notes:?}",
                prepared.notes
            ));
        }
        let ours = serde_json::to_value(screen::read(&screen_input).unwrap()).unwrap();
        let theirs = &expected["shot"];

        eprintln!(
            "{id}: {} boxes (frozen {}), turned {}, label ratio {} (frozen {}), {}",
            prepared.boxes.len(),
            input["boxes"].as_array().unwrap().len(),
            prepared.rotation.degrees(),
            prepared.label_ratio,
            expected["label_ratio"],
            if prepared.rectified() {
                "rectified"
            } else {
                "uncropped"
            }
        );
        match (ours.is_null(), theirs.is_null()) {
            (true, true) => continue,
            (false, false) => {}
            (ours_null, _) => {
                failures.push(format!(
                    "{id}: {} where the vector {}",
                    if ours_null {
                        "nothing read"
                    } else {
                        "a shot read"
                    },
                    if ours_null {
                        "read a shot"
                    } else {
                        "read nothing"
                    }
                ));
                continue;
            }
        }

        // D7's three, less what is declared.
        let (ours_values, theirs_values) = (values(&ours), values(theirs));
        for (key, want) in theirs_values.as_object().unwrap() {
            let got = &ours_values[key];
            if got == want {
                continue;
            }
            match DECLARED_DIFFERENCES
                .iter()
                .find(|d| d.shot == id && d.key == key)
            {
                Some(declared) if declared.holds(got, want) => {
                    eprintln!("  {key} = {got}, the vector's {want}: declared");
                    declared_seen.insert((declared.shot, declared.key));
                }
                _ => failures.push(format!("{id}: {key} = {got}, the vector's {want}")),
            }
        }
        let (ours_provenance, theirs_provenance) = (&ours["provenance"], &theirs["provenance"]);
        for key in ["fields_present", "needs_review"] {
            if ours_provenance[key] != theirs_provenance[key] {
                failures.push(format!(
                    "{id}: provenance.{key} = {}, the vector's {}",
                    ours_provenance[key], theirs_provenance[key]
                ));
            }
        }

        // What D7 lets differ, printed for the finding.
        if ours_provenance["parse_confidence"] != theirs_provenance["parse_confidence"] {
            eprintln!(
                "  parse_confidence {} (frozen {})",
                ours_provenance["parse_confidence"], theirs_provenance["parse_confidence"]
            );
        }
        let (ours_raw, theirs_raw) = (
            ours_provenance["raw_fields"].as_object().unwrap(),
            theirs_provenance["raw_fields"].as_object().unwrap(),
        );
        for label in ours_raw
            .keys()
            .chain(theirs_raw.keys())
            .collect::<BTreeSet<_>>()
        {
            if ours_raw.get(label) != theirs_raw.get(label) {
                eprintln!(
                    "  raw_fields[{label:?}] {:?} (frozen {:?})",
                    ours_raw.get(label),
                    theirs_raw.get(label)
                );
            }
        }
        let (ours_warnings, theirs_warnings) = (
            strings(&ours_provenance["warnings"]),
            strings(&theirs_provenance["warnings"]),
        );
        for warning in ours_warnings.difference(&theirs_warnings) {
            eprintln!("  warning only here: {warning}");
        }
        for warning in theirs_warnings.difference(&ours_warnings) {
            eprintln!("  warning only frozen: {warning}");
        }
    }
    assert_eq!(heif_photos, 1, "the bay photos hold one HEIC");
    for declared in &DECLARED_DIFFERENCES {
        if !declared_seen.contains(&(declared.shot, declared.key)) {
            failures.push(format!(
                "{}: the declared difference ({} = {}, the vector's {}) was not seen: \
                 DECLARED_DIFFERENCES is out of date",
                declared.shot, declared.key, declared.ours, declared.vector
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
