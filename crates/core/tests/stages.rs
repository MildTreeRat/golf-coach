//! The stage document, produced in Rust, against every committed stage vector. [M32 P2]
//!
//! `golf_core::stages::run_stages` on each engine vector's input, compared with the `stages` object
//! of the stage vector derived from it, under `docs/CONFORMANCE.md` §3's rules through
//! [`golf_core::compare`]. **Zero differences is the pass, not a tolerance budget**: the re-record
//! (M32) writes a stage vector by gating this function's answer against the committed one, so a
//! difference here today would be an undeclared move on the first run of the verb.
//!
//! What this adds over `crates/analysis/tests/`' seven stage gates is the *document*: those compare
//! the fields each stage owns through hand-written structs, and none of them can see a key the
//! recorder adds, drops or misspells. This compares the whole object, keys included, which is the
//! question the re-record asks.
//!
//! And the other half [M32 P3]: `golf_core::stages::verify_compose` holds every committed stage
//! vector to its engine vector's `expected`, which is what stops the family agreeing only with
//! whatever `run_stages` happens to do — and is shown here to refuse a moved value, naming the stage.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use contracts::swing::ANALYSIS_VERSION;
use flate2::read::GzDecoder;
use golf_core::compare::compare;
use golf_core::stages::{run_stages, verify_compose, STAGE_NAMES};
use golf_core::VectorInput;
use serde_json::{json, Value};

fn spec_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/core has a grandparent")
        .join("spec")
        .join("vectors")
}

fn read_json(path: &Path) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let text = if path.extension().is_some_and(|e| e == "gz") {
        let mut text = String::new();
        GzDecoder::new(&bytes[..])
            .read_to_string(&mut text)
            .unwrap_or_else(|e| panic!("gunzip {path:?}: {e}"));
        text
    } else {
        String::from_utf8(bytes).unwrap_or_else(|e| panic!("utf-8 {path:?}: {e}"))
    };
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

/// Every committed stage vector with the engine vector it was derived from, joined by
/// `provenance.derived_from` and sorted so a failure names a stable first offender. Duplicated from
/// `crates/analysis/tests/`' readers for the reason they give each other: integration tests are
/// separate binaries.
fn stage_vectors() -> Vec<(Value, Value)> {
    let root = spec_dir().join("stages");
    let mut paths: Vec<PathBuf> = ["synthetic", "corpus"]
        .iter()
        .flat_map(|half| {
            fs::read_dir(root.join(half))
                .unwrap_or_else(|e| panic!("list {:?}: {e}", root.join(half)))
                .map(|entry| entry.expect("a readable directory entry").path())
                .collect::<Vec<_>>()
        })
        .collect();
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let stage = read_json(&path);
            let derived_from = stage["provenance"]["derived_from"]
                .as_str()
                .unwrap_or_else(|| panic!("{path:?} names no engine vector"));
            let mut engine_path = spec_dir().join(derived_from);
            engine_path.set_extension("json");
            if !engine_path.exists() {
                engine_path.set_extension("json.gz");
            }
            (stage, read_json(&engine_path))
        })
        .collect()
}

/// **Every committed stage vector, reproduced with zero differences, in one report.**
#[test]
fn the_stage_document_reproduces_every_committed_stage_vector() {
    let vectors = stage_vectors();
    assert_eq!(
        vectors.len(),
        21,
        "the committed stage family changed size; it has one vector per engine vector"
    );

    let mut differences: Vec<String> = Vec::new();
    for (stage, engine) in &vectors {
        let id = stage["id"].as_str().expect("a stage vector carries an id");
        assert_eq!(
            stage["analysis_version"].as_i64(),
            Some(ANALYSIS_VERSION),
            "{id}: recorded at v{} against a port claiming v{ANALYSIS_VERSION} — re-record the \
             vectors in the change that bumped it",
            stage["analysis_version"]
        );

        let input: VectorInput = serde_json::from_value(engine["input"].clone())
            .unwrap_or_else(|e| panic!("{id}: input does not parse into the ported shapes: {e}"));
        let ours = run_stages(&input);

        let keys: Vec<&str> = ours
            .as_object()
            .expect("the stage document is an object")
            .keys()
            .map(String::as_str)
            .collect();
        let mut named = STAGE_NAMES.to_vec();
        named.sort_unstable();
        assert_eq!(keys, named, "{id}: the document is not the seven stages");

        differences.extend(
            compare(&stage["stages"], &ours)
                .iter()
                .map(|difference| format!("{id}, under stages: {difference}")),
        );
    }

    assert!(
        differences.is_empty(),
        "{} difference(s) across {} stage vectors:\n{}",
        differences.len(),
        vectors.len(),
        differences.join("\n")
    );
    println!("the stage document reproduces {} vectors", vectors.len());
}

/// **The gate above can see a forgotten window**, which is the mistake a recorder of this document
/// is most likely to make and the one only one vector can catch.
///
/// `synthetic/windowed` is the sole committed input with a `face_on_window`, so every other vector
/// passes whether or not the recorder slices. Dropping the window from its input must move the
/// recorded frame indices — if it did not, the zero above would be saying nothing about windowing.
#[test]
fn the_windowed_vector_fails_a_recorder_that_forgets_the_window() {
    let (stage, engine) = stage_vectors()
        .into_iter()
        .find(|(stage, _)| stage["id"] == "stages/synthetic/windowed")
        .expect("the windowed stage vector is committed");

    let mut input: VectorInput =
        serde_json::from_value(engine["input"].clone()).expect("the input parses");
    assert!(input.face_on_window.is_some(), "the vector lost its window");
    input.face_on_window = None;

    let differences = compare(&stage["stages"], &run_stages(&input));
    assert!(
        differences
            .iter()
            .any(|difference| difference.path.starts_with("phases[")),
        "an unwindowed run left every phase boundary where it was: {differences:?}"
    );
}

/// **Every committed stage vector composes onto its engine vector's committed answer.**
///
/// Python runs the same check at build time and again in `tests/test_conformance.py`, because a
/// build-time-only guard stops running the moment nobody regenerates. This is Rust's, and the one
/// the re-record runs before it writes anything (M32's call 9). Every refusal is collected rather
/// than the first, so a red run says how far the drift reaches.
#[test]
fn every_committed_stage_vector_composes_onto_its_bundle_answer() {
    let vectors = stage_vectors();
    let refusals: Vec<String> = vectors
        .iter()
        .filter_map(|(stage, engine)| verify_compose(&stage["stages"], engine).err())
        .collect();
    assert!(
        refusals.is_empty(),
        "{} of {} stage vectors do not compose:\n{}",
        refusals.len(),
        vectors.len(),
        refusals.join("\n")
    );
    println!(
        "{} stage vectors compose onto their bundle answers",
        vectors.len()
    );
}

/// **The compose check can see a moved value, and says which stage moved.**
///
/// A check that passes everything passes the test above too, so each of three stages has one value
/// moved by hand — a phase boundary, an unrounded pose number, a checkpoint's score — and each must
/// be refused under its own stage's name. `synthetic/windowed` carries all three, and it is the one
/// vector whose phases need `engine::shifted` to land, which is asserted first: on any other vector a
/// compose check that forgot the shift would still pass.
#[test]
fn the_compose_check_refuses_a_moved_value_and_names_the_stage() {
    let (stage, engine) = stage_vectors()
        .into_iter()
        .find(|(stage, _)| stage["id"] == "stages/synthetic/windowed")
        .expect("the windowed stage vector is committed");
    let stages = &stage["stages"];
    assert_eq!(verify_compose(stages, &engine), Ok(()));
    assert!(
        !compare(&engine["expected"]["swing"]["phases"], &stages["phases"]).is_empty(),
        "the windowed vector's phases land unshifted, so the shift is no longer under test"
    );

    // The stage that must be named, and the edit that moves one of its values.
    type Perturbation = (&'static str, fn(&mut Value));
    let perturbations: [Perturbation; 3] = [
        ("phases", |stages| {
            let end = &mut stages["phases"][0]["end_frame"];
            *end = json!(end.as_i64().expect("a frame index is an int") + 1);
        }),
        ("measure", |stages| {
            let row = stages["measure"]
                .as_array_mut()
                .expect("`measure` is a list")
                .iter_mut()
                .find(|row| row["value"].is_number())
                .expect("the windowed swing measures something");
            row["value"] = json!(row["value"].as_f64().expect("a number") + 0.01);
        }),
        ("checkpoints", |stages| {
            let row = stages["checkpoints"]
                .as_array_mut()
                .expect("`checkpoints` is a list")
                .iter_mut()
                .find(|row| !row["score"].is_null())
                .expect("the windowed swing scores a checkpoint");
            let score = &mut row["score"]["score"];
            *score = json!(score.as_f64().expect("a score is a number") + 0.25);
        }),
    ];
    for (named, perturb) in perturbations {
        let mut moved = stages.clone();
        perturb(&mut moved);
        let Err(refusal) = verify_compose(&moved, &engine) else {
            panic!("a moved `{named}` value composed onto the bundle answer");
        };
        assert!(
            refusal.contains(&format!("the `{named}` stage")),
            "a moved `{named}` value was refused under another name: {refusal}"
        );
        println!("{refusal}");
    }
}
