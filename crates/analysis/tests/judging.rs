//! The judging against `spec/vectors/stages/`'s `checkpoints`. [M22 P5]
//!
//! The fourth of the seven stages to acquire a runner, and the first that compares a **sentence**.
//! `docs/CONFORMANCE.md` §3 holds `CheckpointScore.message` to an exact match, so this file is
//! where the port's formatting work stops being theoretical: a wrong `round`, a `{}` where CPython
//! writes `str()`, a `:g` reimplemented by hand — each lands as a byte in a string a golfer reads,
//! and each fails here rather than as a float a tolerance would have swallowed.
//!
//! # What it does
//!
//! The orchestration is `conformance.py::run_stages`', because that is what recorded the answers:
//!
//! ```text
//! frames   = windowed(face_on.frames, face_on_window)          # engine.py::_windowed
//! smoothed = smooth_keypoints(frames)                          # P4
//! phases   = segment_phases(smoothed)                          # P4
//! for spec in CHECKPOINT_REGISTRY:
//!     CHECKPOINT_EVALUATORS[spec.name](smoothed, phases, handedness, intent.club, None)
//! ```
//!
//! Note the `None` profile: `run_stages` passes one and so does `engine.py`'s own checkpoint loop,
//! so the `skill_level` half of `resolve_range`'s fallback has no committed evidence in either
//! language. [`crate`]'s `PlayerProfile` doc records that as a hole rather than hiding it.
//!
//! # The rules
//!
//! `docs/CONFORMANCE.md` §3's: floats within `RTOL = 1e-9`, and **strings, bools and ints
//! exactly**. Every difference across all 21 vectors is accumulated and reported in one assertion,
//! the template `crates/trigger/tests/conformance.rs` set.
//!
//! # What this file cannot check, and P5's finding
//!
//! Across all 21 vectors there are **126 checkpoint evaluations and exactly one refusal** — a
//! single `no_handedness` on the one vector with no golfer attributed. So this stage gates 125
//! happy paths, one refusal branch, and **none** of the others: not `NO_BAND`, and not one of the
//! twenty-odd `measure` refusals that `_no_measurement` carries outward. The same shape P4 found
//! one layer down, and for a sharper reason here — `resolve_range` is asked for `club=all` or the
//! default on every committed vector, so the per-club fallback that produces `NO_BAND` in
//! production is never walked. [`every_vector_scores_almost_everything`] pins the absence, so it
//! is a recorded fact rather than something a later reader rediscovers; the refusal branches
//! themselves live in `mechanics.rs`'s and `store.rs`'s unit tests.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use analysis::checkpoints::{evaluator_for, CheckpointOutcome};
use analysis::phases::{segment_phases, LEAD_WRIST};
use analysis::smoothing::{smooth_keypoints, DEFAULT_WINDOW};
use contracts::checkpoints::CHECKPOINT_REGISTRY;
use contracts::golfer::Handedness;
use contracts::intent::PracticeGoal;
use contracts::keypoints::{FrameKeypoints, KeypointsFile};
use contracts::swing::{CheckpointScore, ANALYSIS_VERSION};
use contracts::unscored::UnscoredReason;
use flate2::read::GzDecoder;
use serde::Deserialize;

/// `docs/CONFORMANCE.md` §3's float rule, and the same constants `tests/geometry.rs` uses.
const RTOL: f64 = 1e-9;
const ATOL: f64 = 1e-12;

#[derive(Deserialize)]
struct StageVector {
    id: String,
    analysis_version: i64,
    provenance: Provenance,
    stages: Stages,
}

#[derive(Deserialize)]
struct Provenance {
    derived_from: String,
}

#[derive(Deserialize)]
struct Stages {
    checkpoints: Vec<CheckpointRow>,
    /// Read only by [`the_left_handed_vector_cannot_gate_the_mirror`], which needs the *unrounded*
    /// measurement rather than the `observed` the score carries.
    measure: Vec<MeasureRow>,
}

#[derive(Deserialize)]
struct MeasureRow {
    name: String,
    value: Option<f64>,
}

/// One recorded verdict. `score` and `reason` are the two arms of Python's `CheckpointOutcome`,
/// which serializes as a flat row rather than as a tagged union — so the discriminator here is
/// which of the two is `null`, and a row with both or neither is a malformed vector.
///
/// `reason` is read as the **enum**, not as the string on disk. That makes serde's wire-name table
/// part of what this gate checks: a port that spelled `no_handedness` differently fails at the
/// parse rather than at a string compare, and the comparison below is then between two reasons
/// rather than between two renderings of one.
#[derive(Deserialize)]
struct CheckpointRow {
    name: String,
    score: Option<CheckpointScore>,
    reason: Option<UnscoredReason>,
    detail: String,
}

#[derive(Deserialize)]
struct EngineVector {
    input: EngineInput,
}

#[derive(Deserialize)]
struct EngineInput {
    face_on: KeypointsFile,
    #[serde(default)]
    face_on_window: Option<[i64; 2]>,
    /// `"right"`, `"left"` or absent. `conformance.py::_handedness` is the same coercion, and it
    /// is not cosmetic: the sign of `head_hip_gain_norm` is read off it.
    #[serde(default)]
    handedness: Option<Handedness>,
    #[serde(default)]
    intent: Option<PracticeGoal>,
}

fn spec_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/analysis has a grandparent")
        .join("spec")
        .join("vectors")
}

fn read_json(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    if path.extension().is_some_and(|e| e == "gz") {
        let mut text = String::new();
        GzDecoder::new(&bytes[..])
            .read_to_string(&mut text)
            .unwrap_or_else(|e| panic!("gunzip {path:?}: {e}"));
        text
    } else {
        String::from_utf8(bytes).unwrap_or_else(|e| panic!("utf-8 {path:?}: {e}"))
    }
}

/// Every committed stage vector with the engine vector it was derived from, sorted so a failure
/// names a stable first offender. The same resolution `tests/geometry.rs` does, and duplicated
/// rather than shared for the reason a test helper crate would have to exist to share it: two
/// integration tests in one crate are separate binaries, and a third file whose only job is to be
/// imported by both is more structure than forty lines of reading earns.
fn stage_vectors() -> Vec<(StageVector, EngineVector)> {
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
    assert!(!paths.is_empty(), "no stage vectors under {root:?}");

    paths
        .into_iter()
        .map(|path| {
            let stage: StageVector = serde_json::from_str(&read_json(&path))
                .unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
            let mut engine_path = spec_dir().join(&stage.provenance.derived_from);
            engine_path.set_extension("json");
            if !engine_path.exists() {
                engine_path.set_extension("json.gz");
            }
            let engine: EngineVector = serde_json::from_str(&read_json(&engine_path))
                .unwrap_or_else(|e| panic!("parse {engine_path:?}: {e}"));
            (stage, engine)
        })
        .collect()
}

/// `engine.py::_windowed`, as in `tests/geometry.rs`: P6's function, four lines here rather than a
/// dependency on an unwritten module.
fn windowed(keypoints: &[FrameKeypoints], window: Option<[i64; 2]>) -> &[FrameKeypoints] {
    let Some([lo, hi]) = window else {
        return keypoints;
    };
    let start = lo.max(0);
    let end = hi.min(keypoints.len() as i64);
    if end - start <= 0 {
        return keypoints;
    }
    &keypoints[start as usize..end as usize]
}

fn float_differs(got: Option<f64>, want: Option<f64>) -> bool {
    match (got, want) {
        (None, None) => false,
        (Some(a), Some(b)) => !matches!(
            (a - b).abs().partial_cmp(&(ATOL + RTOL * b.abs())),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ),
        _ => true,
    }
}

/// Every field of a `CheckpointScore`, under §3's rules: floats within tolerance, everything else
/// exact.
///
/// **`message` is compared byte for byte and is the point of this whole file.** It carries a
/// `:.1f`, a `:.2f`, a `:.0f`, a `:g` and a bare `str()` between the six checkpoints, so it is the
/// one field that fails on any of `pyfmt`'s five edges — and the only one where being wrong is
/// visible to a golfer rather than to a diff.
fn compare_score(
    id: &str,
    name: &str,
    got: &CheckpointScore,
    want: &CheckpointScore,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut exact = |field: &str, a: String, b: String| {
        if a != b {
            out.push(format!("{id}: {name}.{field} {a:?} != {b:?}"));
        }
    };
    exact("name", got.name.clone(), want.name.clone());
    exact("passed", got.passed.to_string(), want.passed.to_string());
    exact(
        "one_sided",
        got.one_sided.to_string(),
        want.one_sided.to_string(),
    );
    exact(
        "population_n",
        format!("{:?}", got.population_n),
        format!("{:?}", want.population_n),
    );
    exact("message", got.message.clone(), want.message.clone());

    for (field, a, b) in [
        ("score", Some(got.score), Some(want.score)),
        ("observed", got.observed, want.observed),
        ("expected_low", got.expected_low, want.expected_low),
        ("expected_high", got.expected_high, want.expected_high),
        ("percentile", got.percentile, want.percentile),
    ] {
        if float_differs(a, b) {
            out.push(format!("{id}: {name}.{field} {a:?} != {b:?}"));
        }
    }
    out
}

/// The `checkpoints` stage, on every committed vector, in one report.
#[test]
fn the_judging_conforms_on_all_twenty_one_vectors() {
    let mut differences: Vec<String> = Vec::new();
    let vectors = stage_vectors();

    for (stage, engine) in &vectors {
        let id = &stage.id;
        assert_eq!(
            stage.analysis_version, ANALYSIS_VERSION,
            "{id}: recorded at v{} against a port claiming v{ANALYSIS_VERSION} — \
             re-record the vectors with `golf-core rerecord` in the change that bumped it",
            stage.analysis_version
        );

        let frames = windowed(&engine.input.face_on.frames, engine.input.face_on_window);
        let smoothed = smooth_keypoints(frames, DEFAULT_WINDOW);
        let phases = segment_phases(&smoothed, LEAD_WRIST);
        let intent = engine.input.intent.clone().unwrap_or_default();

        // The registry walk *is* the answer's order, so the rows are zipped against it rather than
        // looked up by name: a port that evaluated the same six in a different order would produce
        // the same set and a different `unscored`, and `docs/CONFORMANCE.md` §3 compares list order
        // exactly.
        if stage.stages.checkpoints.len() != CHECKPOINT_REGISTRY.len() {
            differences.push(format!(
                "{id}: checkpoints row count {} != {}",
                stage.stages.checkpoints.len(),
                CHECKPOINT_REGISTRY.len()
            ));
            continue;
        }

        for (spec, want) in CHECKPOINT_REGISTRY.iter().zip(&stage.stages.checkpoints) {
            if spec.name != want.name {
                differences.push(format!(
                    "{id}: checkpoint order — registry has {}, vector has {}",
                    spec.name, want.name
                ));
                continue;
            }
            let judged = evaluator_for(spec.name)(
                &smoothed,
                &phases,
                engine.input.handedness,
                intent.club,
                None,
            );
            match (&judged, &want.score, &want.reason) {
                (CheckpointOutcome::Scored(got), Some(expected), None) => {
                    differences.extend(compare_score(id, spec.name, got, expected));
                }
                (CheckpointOutcome::Unscored { reason, detail }, None, Some(expected)) => {
                    // A reason is a *word*, not a number: §3 compares it exactly, and so is the
                    // `detail` beside it, which names the window or the club that missed.
                    if reason != expected {
                        differences.push(format!(
                            "{id}: {}.reason {reason:?} != {expected:?}",
                            spec.name
                        ));
                    }
                    if detail != &want.detail {
                        differences.push(format!(
                            "{id}: {}.detail {detail:?} != {:?}",
                            spec.name, want.detail
                        ));
                    }
                }
                (CheckpointOutcome::Scored(_), None, Some(expected)) => differences.push(format!(
                    "{id}: {} scored, but the vector refuses it with {expected:?}",
                    spec.name
                )),
                (CheckpointOutcome::Unscored { reason, .. }, Some(_), None) => {
                    differences.push(format!(
                        "{id}: {} refused with {reason:?}, but the vector scores it",
                        spec.name
                    ))
                }
                _ => differences.push(format!(
                    "{id}: {} — the vector row carries both a score and a reason, or neither",
                    spec.name
                )),
            }
        }
    }

    assert!(
        differences.is_empty(),
        "{} difference(s) across {} vectors:\n{}",
        differences.len(),
        vectors.len(),
        differences.join("\n")
    );
    println!("judging: {} vectors conform", vectors.len());
}

/// The hole this stage has, pinned as a fact rather than left to be rediscovered.
///
/// **This test is meant to fail the day a vector refuses something new**, which is the good
/// failure: the answer is to extend [`the_judging_conforms_on_all_twenty_one_vectors`]'s refusal
/// comparison to cover the new branch, not to relax this. `tests/geometry.rs`'s
/// `every_vector_measures_everything` is the same device one layer down.
#[test]
fn every_vector_scores_almost_everything() {
    let mut refusals: Vec<(String, String, UnscoredReason)> = Vec::new();
    let mut evaluations = 0;
    for (stage, _) in stage_vectors() {
        for row in &stage.stages.checkpoints {
            evaluations += 1;
            if let Some(reason) = row.reason {
                refusals.push((stage.id.clone(), row.name.clone(), reason));
            }
        }
    }
    assert_eq!(evaluations, 126, "21 vectors times six checkpoints");
    let reasons: Vec<UnscoredReason> = refusals.iter().map(|(_, _, r)| *r).collect();
    assert_eq!(
        reasons,
        [UnscoredReason::NoHandedness],
        "the committed corpus's refusals changed: {refusals:?}"
    );
}

/// **The corpus has a left-handed vector and it cannot gate the mirror**, which is the sharpest
/// thing this phase found.
///
/// `evaluate_head_stays_back` re-signs its observation for a left-handed golfer — the whole reason
/// the checkpoint takes a `Handedness` at all, and the difference between reading a left-handed
/// swing correctly and reading an ordinary impact position as a gross fault. Exactly one of the 21
/// vectors is left-handed, and its `head_hip_gain_norm` is **exactly `0.0`**, so `-raw == raw` and
/// a port that deleted the mirror outright passes all 21. Measured by mutation, not assumed.
///
/// A first version of this test asserted only that both handednesses appear in the corpus. That is
/// true, and it is the kind of reassurance that stops someone looking — so it is written out here
/// as the hole it is. `mechanics.rs`'s
/// `the_handedness_mirror_flips_the_observation_and_the_verdict` is what actually holds the line.
///
/// It fails the day a left-handed vector with a non-zero gain is captured, which is the good
/// failure: the answer then is to delete this test and note that the stage covers the mirror.
#[test]
fn the_left_handed_vector_cannot_gate_the_mirror() {
    let mut right = 0;
    let mut absent = 0;
    let mut left_gains: Vec<(String, Option<f64>)> = Vec::new();
    for (stage, engine) in stage_vectors() {
        match engine.input.handedness {
            Some(Handedness::Right) => right += 1,
            None => absent += 1,
            Some(Handedness::Left) => {
                let gain = stage
                    .stages
                    .measure
                    .iter()
                    .find(|row| row.name == "head_hip_gain_norm")
                    .and_then(|row| row.value);
                left_gains.push((stage.id.clone(), gain));
            }
        }
    }
    assert!(right > 0 && absent > 0, "right={right} absent={absent}");
    assert_eq!(
        left_gains.len(),
        1,
        "the corpus's left-handed half changed: {left_gains:?}"
    );
    assert_eq!(
        left_gains[0].1,
        Some(0.0),
        "a left-handed vector now has a non-zero gain — the mirror *is* gated, so retire this test"
    );
}
