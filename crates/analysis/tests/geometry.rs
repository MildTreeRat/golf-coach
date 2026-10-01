//! The geometry against `spec/vectors/stages/`'s `smoothed`, `phases` and `measure`. [M22 P4]
//!
//! M22 P1 built these twenty-one vectors before there was a line of Rust to run them, for the reason
//! M20 P0 built the audio family first: the first implementation of a stage should have an answer to
//! fail against. This is the phase that finally runs three of the seven, so the sentence in
//! `docs/CONFORMANCE.md` §2 that says **nothing executes these yet** is now true of four stages
//! rather than all seven.
//!
//! # What it does
//!
//! Each stage vector names the engine vector it was derived from, so the input is read from
//! `spec/vectors/{synthetic,corpus}/` and the answers from `spec/vectors/stages/` beside it. The
//! orchestration is `conformance.py::run_stages`', because that is what recorded them:
//!
//! ```text
//! frames  = windowed(face_on.frames, face_on_window)   # engine.py::_windowed
//! smoothed = smooth_keypoints(frames)                  -> the `smoothed` stage
//! phases   = segment_phases(smoothed)                  -> the `phases` stage, window-relative
//! measure  = POSE_MEASUREMENTS[each](smoothed, phases) -> the `measure` stage, unrounded
//! ```
//!
//! [`windowed`] is `engine.py::_windowed` and is **not** P4's module — P6 ports `engine.py`. It is
//! four lines here rather than a dependency on an unwritten module, and `synthetic/windowed` is the
//! one vector that can tell whether they were got right: its window is `[40, 96]` of 136 frames, so
//! every frame index the `phases` stage carries is offset by 40 from the whole-clip answer in the
//! engine vector. A port that forgets the slice fails there and nowhere else.
//!
//! # The rules
//!
//! `docs/CONFORMANCE.md` §3's, and not P2's stricter ones: arithmetic happened here, so floats
//! compare within `RTOL = 1e-9` sized for a different summation order, while a refusal compares equal
//! to nothing but a refusal and a reason string compares exactly. Every difference across all 21
//! vectors is accumulated and reported in one assertion — the template `crates/trigger/tests/
//! conformance.rs` set, and worth more than 21 runs that each name one.
//!
//! **What this file cannot check, and P4's finding.** Not one of the 21 committed vectors refuses a
//! single measurement: all thirteen metrics are measurable on every swing here, so the `measure`
//! stage gates thirteen happy paths and none of the twenty-odd refusal branches — including every
//! `detail` sentence, which reaches a golfer and which §3 compares exactly. Those live in
//! `measure.rs`'s unit tests, and [`every_vector_measures_everything`] pins the *absence* so the hole
//! is a recorded fact rather than something a later reader has to rediscover.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use analysis::measure::POSE_MEASUREMENTS;
use analysis::phases::{segment_phases, LEAD_WRIST};
use analysis::smoothing::{smooth_keypoints, DEFAULT_WINDOW};
use contracts::keypoints::{FrameKeypoints, KeypointsFile};
use contracts::swing::{PhaseSegment, ANALYSIS_VERSION};
use flate2::read::GzDecoder;
use serde::Deserialize;

/// `docs/CONFORMANCE.md` §3's float rule. Sized for a different summation order, not a different
/// rule: the last bits of an f64 are the same measurement added up differently.
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
    /// Every landmark's `x`/`y` after smoothing: `[frame][landmark][x, y]`. The other four fields
    /// are exact pass-throughs and are asserted as a property rather than recorded — see
    /// `docs/CONFORMANCE.md` §2, and [`smoothing_passes_the_untouched_fields_through`] below, which
    /// is this side of that bargain.
    smoothed: Vec<Vec<[f64; 2]>>,
    phases: Vec<PhaseSegment>,
    measure: Vec<MeasureRow>,
}

#[derive(Deserialize)]
struct MeasureRow {
    name: String,
    value: Option<f64>,
    reason: Option<String>,
    detail: String,
}

/// The engine vector's input, of which only these three keys matter to this phase.
#[derive(Deserialize)]
struct EngineVector {
    input: EngineInput,
}

#[derive(Deserialize)]
struct EngineInput {
    face_on: KeypointsFile,
    #[serde(default)]
    face_on_window: Option<[i64; 2]>,
}

fn spec_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/analysis has a grandparent")
        .join("spec")
        .join("vectors")
}

/// Read a vector, gunzipping it when the corpus half's `.json.gz` is what is on disk.
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

/// Every committed stage vector, both halves, sorted so a failure names a stable first offender.
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
            // `provenance.derived_from` is the engine vector's id — "synthetic/windowed" — and the
            // stage family holds its input by reference precisely so none of those 8.6 MB is
            // duplicated. Which means resolving it is this harness's job.
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

/// `engine.py::_windowed`: `(offset, frames)` for a window, clamped into range. An empty window is
/// ignored.
///
/// P6's, not P4's — see the module doc. The offset is returned and deliberately unused here, because
/// the `phases` stage is recorded **window-relative**, before `engine.py::_shifted` puts the indices
/// back into whole-clip coordinates.
fn windowed(keypoints: &[FrameKeypoints], window: Option<[i64; 2]>) -> (usize, &[FrameKeypoints]) {
    let Some([lo, hi]) = window else {
        return (0, keypoints);
    };
    let start = lo.max(0);
    let end = hi.min(keypoints.len() as i64);
    if end - start <= 0 {
        return (0, keypoints);
    }
    (start as usize, &keypoints[start as usize..end as usize])
}

/// §3's float rule, and its `null` rule above it: a refusal compares equal to nothing but a refusal.
fn float_differs(got: Option<f64>, want: Option<f64>) -> bool {
    match (got, want) {
        (None, None) => false,
        // `!(<=)` rather than `>`, because the two differ on a NaN and only the first is the rule:
        // `docs/CONFORMANCE.md` §3 says a float *passes* when it is within tolerance, so anything
        // incomparable is a difference. Written as `partial_cmp` to say that out loud, which is
        // what clippy asks for and what a reader of a conformance comparator needs anyway.
        (Some(a), Some(b)) => !matches!(
            (a - b).abs().partial_cmp(&(ATOL + RTOL * b.abs())),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ),
        _ => true,
    }
}

/// The three stages, on every committed vector, in one report.
#[test]
fn the_geometry_conforms_on_all_twenty_one_vectors() {
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

        let (_, frames) = windowed(&engine.input.face_on.frames, engine.input.face_on_window);
        let smoothed = smooth_keypoints(frames, DEFAULT_WINDOW);
        let phases = segment_phases(&smoothed, LEAD_WRIST);

        // ---- `smoothed`
        if smoothed.len() != stage.stages.smoothed.len() {
            differences.push(format!(
                "{id}: smoothed frame count {} != {}",
                smoothed.len(),
                stage.stages.smoothed.len()
            ));
        } else {
            for (f, (got, want)) in smoothed.iter().zip(&stage.stages.smoothed).enumerate() {
                if got.landmarks.len() != want.len() {
                    differences.push(format!(
                        "{id}: smoothed[{f}] landmark count {} != {}",
                        got.landmarks.len(),
                        want.len()
                    ));
                    continue;
                }
                for (l, (lm, xy)) in got.landmarks.iter().zip(want).enumerate() {
                    for (axis, a, b) in [("x", lm.x, xy[0]), ("y", lm.y, xy[1])] {
                        if float_differs(Some(a), Some(b)) {
                            differences.push(format!("{id}: smoothed[{f}][{l}].{axis} {a} != {b}"));
                        }
                    }
                }
            }
        }

        // ---- `phases`
        if phases.len() != stage.stages.phases.len() {
            differences.push(format!(
                "{id}: {} phases against {}",
                phases.len(),
                stage.stages.phases.len()
            ));
        } else {
            for (got, want) in phases.iter().zip(&stage.stages.phases) {
                let label = format!("{id}: phases[{}]", want.phase.as_str());
                if got.phase != want.phase {
                    differences.push(format!(
                        "{label} is {} not {}",
                        got.phase.as_str(),
                        want.phase.as_str()
                    ));
                }
                if got.start_frame != want.start_frame || got.end_frame != want.end_frame {
                    differences.push(format!(
                        "{label} frames ({}, {}) != ({}, {})",
                        got.start_frame, got.end_frame, want.start_frame, want.end_frame
                    ));
                }
                for (field, a, b) in [
                    ("start_ms", got.start_ms, want.start_ms),
                    ("end_ms", got.end_ms, want.end_ms),
                ] {
                    if float_differs(Some(a), Some(b)) {
                        differences.push(format!("{label}.{field} {a} != {b}"));
                    }
                }
                if got.detected != want.detected {
                    differences.push(format!(
                        "{label}.detected {} != {}",
                        got.detected, want.detected
                    ));
                }
            }
        }

        // ---- `measure`
        if POSE_MEASUREMENTS.len() != stage.stages.measure.len() {
            differences.push(format!(
                "{id}: {} measurements against {} rows",
                POSE_MEASUREMENTS.len(),
                stage.stages.measure.len()
            ));
        } else {
            for ((name, spec), want) in POSE_MEASUREMENTS.iter().zip(&stage.stages.measure) {
                if *name != want.name {
                    differences.push(format!(
                        "{id}: measure row order — {name} against {}",
                        want.name
                    ));
                    continue;
                }
                let got = (spec.measure)(&smoothed, &phases);
                if float_differs(got.value, want.value) {
                    differences.push(format!(
                        "{id}: measure[{name}] {:?} != {:?}",
                        got.value, want.value
                    ));
                }
                let got_reason = got.reason.map(|r| {
                    serde_json::to_value(r)
                        .expect("a reason serializes")
                        .as_str()
                        .expect("to a string")
                        .to_string()
                });
                if got_reason != want.reason {
                    differences.push(format!(
                        "{id}: measure[{name}] reason {got_reason:?} != {:?}",
                        want.reason
                    ));
                }
                if got.detail != want.detail {
                    differences.push(format!(
                        "{id}: measure[{name}] detail {:?} != {:?}",
                        got.detail, want.detail
                    ));
                }
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
}

/// The family is 21 vectors and the exit criterion counts them, so a missing half fails loudly
/// rather than passing a shorter suite.
#[test]
fn all_twenty_one_vectors_are_present() {
    let vectors = stage_vectors();
    assert_eq!(vectors.len(), 21, "the stage family is 21 vectors");
    let synthetic = vectors
        .iter()
        .filter(|(s, _)| s.id.contains("synthetic"))
        .count();
    assert_eq!(synthetic, 6, "6 synthetic and 15 corpus");
}

/// `docs/CONFORMANCE.md` §2 records the trade the `smoothed` stage makes: `z`, `visibility`,
/// `frame_index` and `timestamp_ms` are exact pass-throughs, so recording them would record the
/// input a second time at a cost of 2.3 MB. The vector-builder asserts that at build time; this is
/// the *port's* side of the same bargain, because a Rust `smooth_keypoints` that quietly recomputed
/// one of the four would pass the stage above and be caught nowhere.
///
/// `camera_id` is the fifth field and is deliberately **not** in this list — the Python drops it, and
/// `smoothing.rs`'s own tests pin that.
#[test]
fn smoothing_passes_the_untouched_fields_through() {
    for (stage, engine) in stage_vectors() {
        let (_, frames) = windowed(&engine.input.face_on.frames, engine.input.face_on_window);
        let smoothed = smooth_keypoints(frames, DEFAULT_WINDOW);
        for (raw, got) in frames.iter().zip(&smoothed) {
            assert_eq!(raw.frame_index, got.frame_index, "{}", stage.id);
            assert_eq!(raw.timestamp_ms, got.timestamp_ms, "{}", stage.id);
            for (before, after) in raw.landmarks.iter().zip(&got.landmarks) {
                assert_eq!(before.z, after.z, "{}: z moved", stage.id);
                assert_eq!(
                    before.visibility, after.visibility,
                    "{}: visibility moved",
                    stage.id
                );
            }
        }
    }
}

/// P4's finding, pinned so it reads as a measured fact rather than an oversight.
///
/// Every refusal branch in `measure.rs` — twenty-odd of them, each carrying a sentence
/// `docs/CONFORMANCE.md` §3 compares exactly — is unreachable from this family, because every metric
/// is measurable on every committed swing. **When this test starts failing, that is good news**: a
/// vector that refuses something has been recorded, and the assertion below should be replaced by
/// the stage comparison finally covering that branch rather than relaxed.
#[test]
fn every_vector_measures_everything() {
    let refusals: Vec<String> = stage_vectors()
        .iter()
        .flat_map(|(stage, _)| {
            stage
                .stages
                .measure
                .iter()
                .filter(|row| row.value.is_none())
                .map(|row| format!("{}: {} ({:?})", stage.id, row.name, row.reason))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        refusals.is_empty(),
        "a committed vector now refuses a measurement, which this family could never gate before. \
         Cover its branch in the stage comparison rather than relaxing this:\n{}",
        refusals.join("\n")
    );
}

/// `synthetic/windowed` is the one vector whose frame indices are offset, so the window seam has
/// exactly one witness. If it stops being windowed this gate is testing nothing.
#[test]
fn the_window_seam_still_has_a_witness() {
    let windowed_vectors: Vec<&str> = stage_vectors()
        .iter()
        .filter(|(_, e)| e.input.face_on_window.is_some())
        .map(|(s, _)| Box::leak(s.id.clone().into_boxed_str()) as &str)
        .collect();
    assert_eq!(
        windowed_vectors,
        ["stages/synthetic/windowed"],
        "the window offset has one witness and this is it"
    );
}
