//! The writer against every committed vector's landmarks. [M23 P6]
//!
//! **What this gate can prove without MediaPipe, and why it is worth having beside the real run.**
//! P6's headline verification is a 4K clip posed through `golf-pose` and diffed against
//! `data/processed/sessions/2026-08-23/11/face_on.keypoints.json` — but that clip is 38 MB and not in
//! git, so a test that needed it would be a test that only passes on this box. What *is* committed is
//! `spec/vectors/synthetic/`, whose `input.face_on` is a real `KeypointsFile` recorded by the Python
//! engine, nulls and all. Pushing each one through [`pose::writer`] and reading it back asks the
//! question the writer can get wrong: **does a value survive being written**.
//!
//! The synthetic family specifically, and that is not laziness about the 15 corpus vectors: those are
//! gzipped and reading them would cost this crate a `flate2` dev-dependency, while the nulls that
//! make this interesting are *only* in the synthetic six — 391 `camera_id` and 6 `clip.source_sha256`
//! spelled `null`, against zero in the corpus half. The corpus vectors' keypoints are already read
//! and round-tripped by `crates/contracts/tests/round_trip.rs`; what is new here is the null
//! handling, and this is where it lives.
//!
//! The comparison is on the **parsed** value and not the text, and here that is unavoidable rather
//! than merely prudent: a vector is written by `conformance.py`'s `json.dumps`, which is CPython, and
//! CPython writes `-1.636758133827243e-05` where `serde_json` writes `-0.00001636758133827243` for
//! the identical f64 (ADR-032's second addendum, 77 floats). A `*.keypoints.json` is a different
//! writer with a different answer — `model_dump_json` serializes inside pydantic-core, which is Rust
//! — and [`pose::writer`]'s own docs carry that measurement. This gate compares values because the
//! material it has is vector text.

use std::fs;
use std::path::{Path, PathBuf};

use contracts::keypoints::KeypointsFile;
use serde_json::Value;

/// `spec/vectors/synthetic`, two levels up from this crate, as `round_trip.rs` resolves it.
fn synthetic_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/vectors/synthetic")
        .canonicalize()
        .expect(
            "spec/vectors/synthetic is missing — run `python scripts/conformance.py regenerate`",
        )
}

/// Every synthetic vector's `input.face_on`, as `(id, raw JSON, parsed)`.
///
/// Discovered rather than listed, the choice `conformance.vector_paths` and `round_trip.rs` both
/// make: a vector that exists on disk and in no index is a vector nothing runs.
fn face_on_keypoints() -> Vec<(String, Value, KeypointsFile)> {
    let mut paths: Vec<PathBuf> = fs::read_dir(synthetic_dir())
        .expect("read spec/vectors/synthetic")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no synthetic vectors under {paths:?}");

    paths
        .iter()
        .map(|path| {
            let text = fs::read_to_string(path).expect("read vector");
            let vector: Value = serde_json::from_str(&text).expect("parse vector");
            let id = vector["id"].as_str().expect("vector has an id").to_string();
            let raw = vector["input"]["face_on"].clone();
            let parsed: KeypointsFile =
                serde_json::from_value(raw.clone()).unwrap_or_else(|e| panic!("{id}: {e}"));
            (id, raw, parsed)
        })
        .collect()
}

/// Write each one, read it back, and accumulate every disagreement before asserting.
///
/// One assertion at the end rather than a panic on the first vector: a report naming four vectors is
/// worth four runs that each name one. `crates/trigger/tests/conformance.rs` is the template and
/// `round_trip.rs` follows it too.
#[test]
fn every_committed_keypoints_file_survives_the_writer() {
    let dir = std::env::temp_dir().join(format!("golf-pose-writer-{}", std::process::id()));
    let mut report: Vec<String> = Vec::new();
    let mut vectors = 0usize;
    let mut frames = 0usize;

    for (id, raw, parsed) in face_on_keypoints() {
        let target = pose::keypoints_path(&dir, &id);
        pose::writer::write(&parsed, &target).unwrap_or_else(|e| panic!("{id}: writing: {e}"));
        let text =
            fs::read_to_string(&target).unwrap_or_else(|e| panic!("{id}: reading back: {e}"));

        // No null anywhere, which is the property `save_keypoints`' `exclude_none=True` has and the
        // one the six synthetic vectors can actually distinguish — they are the only committed
        // keypoints that carry an explicit one.
        if text.contains("null") {
            report.push(format!("{id}: the written file contains a null"));
        }
        // And the keys it dropped were exactly the null ones: `camera_id` is absent from the text but
        // the frames are all still there.
        match serde_json::from_str::<KeypointsFile>(&text) {
            Ok(back) if back == parsed => {}
            Ok(back) => report.push(format!(
                "{id}: read back different: {} frames vs {}, estimator {:?} vs {:?}, clip {:?} vs {:?}",
                back.frames.len(),
                parsed.frames.len(),
                back.pose_estimator,
                parsed.pose_estimator,
                back.clip,
                parsed.clip,
            )),
            Err(e) => report.push(format!("{id}: the written file does not parse: {e}")),
        }
        // The raw vector text is the other side of the same claim: a key spelled `null` there must
        // have parsed to `None`, or the equality above would be comparing two wrong values.
        if raw["frames"]
            .as_array()
            .is_some_and(|frames| frames.iter().any(|frame| frame["camera_id"].is_null()))
            && parsed.frames.iter().any(|frame| frame.camera_id.is_some())
        {
            report.push(format!("{id}: a null camera_id parsed to a value"));
        }

        vectors += 1;
        frames += parsed.frames.len();
    }

    let _ = fs::remove_dir_all(&dir);
    assert!(
        report.is_empty(),
        "{} vectors differ:\n  {}",
        report.len(),
        report.join("\n  ")
    );
    // A gate that silently ran one of six would report the same green as one that ran all of them.
    assert_eq!(vectors, 6, "expected 6 synthetic vectors, wrote {vectors}");
    assert_eq!(
        frames, 391,
        "expected 391 frames of landmarks, wrote {frames}"
    );
}
