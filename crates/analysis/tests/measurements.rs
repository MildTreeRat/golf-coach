//! `_measurements`' five groups against `spec/vectors/stages/`'s `measurements`. [M22 P5b, P8b]
//!
//! The fifth of the seven stages to acquire a runner, and P8b completes it: **five of the stage's
//! seven groups** — `pose`, `placements` and `pivot_face_on`, which are the rows a face-on clip alone
//! produces, plus `shot` and `flight`, which are the rows a launch-monitor tile produces. The two
//! `_dtl` groups are `analyze_swing_bundle`'s rather than `_measurements`', so they belong to
//! `crates/core`'s whole-bundle gate and this file deliberately does not touch them.
//!
//! # What it does
//!
//! The orchestration is `conformance.py::run_stages`', because that is what recorded the answers:
//!
//! ```text
//! frames   = windowed(face_on.frames, face_on_window)      # engine.py::_windowed
//! smoothed = smooth_keypoints(frames)                      # P4
//! phases   = segment_phases(smoothed)                      # P4
//! flown    = fly_shot(shot, loft_deg, handedness)          # P8b, once
//! every    = _measurements(smoothed, phases, shot, handedness, flown)
//! groups   = every sliced by [len(pose), len(_placements), len(_pivot_measurements), …]
//! ```
//!
//! `run_stages` **slices one real list** rather than rebuilding it group by group, and its docstring
//! records why: grouping by `Measurement.source` is the obvious way and is wrong, because the
//! face-on pivot rows carry `pose:face_on` too and both placement families carry
//! `population:golfdb`. `engine::face_on_measurements` returns its three groups separately for the
//! same reason in reverse — the partition is *positional*, so a port that produced the right rows
//! under the wrong grouping would still concatenate to the right answer, and this file checks both
//! the groups and their concatenation.
//!
//! `shot` and `flight` are the two groups `source` *would* have separated cleanly, and they are still
//! read positionally: a partition that is right for four of six groups by accident is not a rule.
//!
//! # The rules
//!
//! `docs/CONFORMANCE.md` §3's: floats within `RTOL = 1e-9`, and **strings, bools and ints exactly**.
//! `Measurement.detail` is a string, and three of the rows here interpolate a `%g` percentile and a
//! dict key into one — so `pyfmt::g` and the insertion-order edge both land in this file as bytes
//! rather than as tolerances.
//!
//! # What this file cannot check, and P5b's findings
//!
//! Two holes, both measured rather than assumed, and both pinned below so they are recorded facts
//! rather than something a later reader rediscovers.
//!
//! **Every one of the fifteen corpus vectors refuses the trajectory placement**, always on the same
//! landmarks: the trail elbow and trail wrist are missing 45-70% of their resampled timeline against
//! `trajectory::MAX_MISSING`'s 40%. So T² and Q are gated by the **six synthetic vectors only**, and
//! the fifteen real swings gate the *refusal* — which is the opposite of the coverage one would
//! assume from "21 vectors, all green". See [`the_corpus_never_reaches_the_trajectory_basis`].
//!
//! **Five of the ten pivot rows are never emitted here.** They are the `_dtl` half, whose walker is
//! `_dtl_placements` — `analyze_swing_bundle`'s and not `_measurements`'. Their answers are committed
//! on every corpus vector and P7 runs them from `tests/alignment.rs` and the whole-bundle gate; what
//! this file pins is that the group it *does* emit is the face-on five in registry order. See
//! [`the_dtl_half_of_the_registry_is_gated_from_the_bundle_and_not_from_here`].
//!
//! **The two outcome groups are thin where they look thick.** Fifteen vectors carry a shot, so the
//! `shot` group's seven names are gated 104 rows deep — but only *one* of them ever goes missing
//! (`2026-08-23-3` printed no face angle), so [`analysis::shot_measure::measure_face_to_path`]'s
//! two-input refusal is gated in one direction and nothing else in the group is gated in either. The
//! `flight` group is thinner still: **25 rows across five flights**, of which
//! `flight_landing_offline_yds` is four and `flight_spin_rpm` is **one**. Those two names are the only
//! conditional readers in `FLIGHT_MEASUREMENTS`, so the whole of what makes that registry more than a
//! tuple unpack rests on five committed rows. See
//! [`the_outcome_groups_are_gated_by_a_handful_of_rows`].

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use analysis::benchmarks::flight_model::load_flight_model;
use analysis::engine::{face_on_measurements, flight_measurements, shot_measurements};
use analysis::flight::DEFAULT_STEP_S;
use analysis::flight_measure::fly_shot;
use analysis::phases::{segment_phases, LEAD_WRIST};
use analysis::smoothing::{smooth_keypoints, DEFAULT_WINDOW};
use contracts::golfer::Handedness;
use contracts::keypoints::{FrameKeypoints, KeypointsFile};
use contracts::pivots::PIVOT_MEASUREMENT_REGISTRY;
use contracts::placements::{DOWN_THE_LINE, FACE_ON};
use contracts::shot::ShotData;
use contracts::swing::{Measurement, ANALYSIS_VERSION};
use flate2::read::GzDecoder;
use serde::Deserialize;

/// `docs/CONFORMANCE.md` §3's float rule, and the same constants `tests/geometry.rs` uses.
const RTOL: f64 = 1e-9;
const ATOL: f64 = 1e-12;

/// The five groups `_measurements` itself produces, in the order `conformance.MEASUREMENT_GROUPS`
/// lists them. The two `_dtl` groups after them are `analyze_swing_bundle`'s, not this call's.
const OWNED: [&str; 5] = ["pose", "placements", "pivot_face_on", "shot", "flight"];

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
    measurements: Vec<Group>,
}

/// One recorded group. The rows are read as `Measurement`s, so the contract's own bounds and its
/// `detail` default apply at the parse — a vector whose row is missing `detail` reads as `""` here
/// exactly as it does in Python.
#[derive(Deserialize)]
struct Group {
    group: String,
    rows: Vec<Measurement>,
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
    /// The sign the mirror reads. `conformance.py::_handedness` is the same coercion, and a swing
    /// with no golfer attributed gets no mirror rather than a guessed one.
    #[serde(default)]
    handedness: Option<Handedness>,
    /// The two the outcome groups need [M22 P8b]. Both absent on all six synthetic vectors; the shot
    /// present on all fifteen corpus ones and the loft on five of them.
    #[serde(default)]
    shot: Option<ShotData>,
    #[serde(default)]
    loft_deg: Option<f64>,
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
/// names a stable first offender. The same resolution `tests/geometry.rs` and `tests/judging.rs`
/// do, duplicated rather than shared for the reason they give: two integration tests in one crate
/// are separate binaries, and a third file whose only job is to be imported is more structure than
/// forty lines of reading earns.
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

/// `engine.py::_windowed`, as in the two sibling gates: P6's function, four lines here rather than
/// a dependency on an unwritten module.
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

fn float_differs(got: f64, want: f64) -> bool {
    !matches!(
        (got - want).abs().partial_cmp(&(ATOL + RTOL * want.abs())),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    )
}

/// Every field of a `Measurement` under §3's rules: the float within tolerance, the four strings
/// exactly.
///
/// **`detail` is compared byte for byte and it is where this file earns its keep.** Two of the
/// three placement rows interpolate a `%g` percentile, one interpolates a dict key chosen by a
/// stable sort over an insertion-ordered map, and all five pivot rows carry fixed registry prose. A
/// wrong `pyfmt::g`, a `HashMap` where the Python has a `dict`, or a registry sentence retyped at a
/// call site each land here as a byte a golfer reads.
fn compare_row(
    id: &str,
    group: &str,
    at: usize,
    got: &Measurement,
    want: &Measurement,
) -> Vec<String> {
    let mut out = Vec::new();
    for (field, a, b) in [
        ("name", &got.name, &want.name),
        ("unit", &got.unit, &want.unit),
        ("source", &got.source, &want.source),
        ("detail", &got.detail, &want.detail),
    ] {
        if a != b {
            out.push(format!("{id}: {group}[{at}].{field} {a:?} != {b:?}"));
        }
    }
    if float_differs(got.value, want.value) {
        out.push(format!(
            "{id}: {group}[{at}].value {} != {}",
            got.value, want.value
        ));
    }
    out
}

/// The five `_measurements` groups, on every committed vector, in one report.
#[test]
fn the_measurements_conform_on_all_twenty_one_vectors() {
    let mut differences: Vec<String> = Vec::new();
    let vectors = stage_vectors();

    for (stage, engine) in &vectors {
        let id = &stage.id;
        assert_eq!(
            stage.analysis_version, ANALYSIS_VERSION,
            "{id}: recorded at v{} against a port claiming v{ANALYSIS_VERSION} — \
             regenerate the vectors in the change that bumped it",
            stage.analysis_version
        );

        let frames = windowed(&engine.input.face_on.frames, engine.input.face_on_window);
        let smoothed = smooth_keypoints(frames, DEFAULT_WINDOW);
        let phases = segment_phases(&smoothed, LEAD_WRIST);
        let (pose, placements, pivots) =
            face_on_measurements(&smoothed, &phases, engine.input.handedness);
        let (shot, flight) = outcome_measurements(engine);

        for (name, got) in OWNED
            .iter()
            .zip([&pose, &placements, &pivots, &shot, &flight])
        {
            let Some(want) = stage
                .stages
                .measurements
                .iter()
                .find(|group| group.group == *name)
            else {
                differences.push(format!("{id}: no {name} group recorded"));
                continue;
            };
            if got.len() != want.rows.len() {
                differences.push(format!(
                    "{id}: {name} row count {} != {} ({:?} against {:?})",
                    got.len(),
                    want.rows.len(),
                    got.iter().map(|r| &r.name).collect::<Vec<_>>(),
                    want.rows.iter().map(|r| &r.name).collect::<Vec<_>>(),
                ));
                continue;
            }
            for (at, (got, want)) in got.iter().zip(&want.rows).enumerate() {
                differences.extend(compare_row(id, name, at, got, want));
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
    println!("measurements: {} vectors conform", vectors.len());
}

/// The `shot` and `flight` groups, off the engine vector's own three arguments. [M22 P8b]
///
/// **The flight is flown once and read twice**, as it is in `analyze_swing`: `flight_measurements`
/// and `flight_unscored` take the same `FlownShot`, and a harness that flew it per reader would be
/// testing two integrations against one recorded answer. `tests/flight.rs` owns the resolution
/// itself; what this file adds is the row it becomes.
fn outcome_measurements(engine: &EngineVector) -> (Vec<Measurement>, Vec<Measurement>) {
    let Some(shot) = engine.input.shot.as_ref() else {
        return (Vec::new(), Vec::new());
    };
    let flown = fly_shot(
        shot,
        engine.input.loft_deg,
        engine.input.handedness,
        load_flight_model(),
        DEFAULT_STEP_S,
    );
    (shot_measurements(shot), flight_measurements(&flown))
}

/// **The groups are a positional partition of one flat list, not a set of independent answers.**
///
/// `run_stages` slices `_measurements`' single list at five offsets, so the port's five groups
/// concatenated must be a prefix of the recorded flat list in exactly that order — the two `_dtl`
/// groups are what follows. A port that got every row right and emitted the pivots before the
/// placements, or the flight before the shot, would pass the group-by-group comparison above and
/// fail here, and `docs/CONFORMANCE.md` §3 compares `swing.measurements` positionally, so that
/// reordering is a wrong answer and not a formatting difference.
#[test]
fn the_five_groups_concatenate_in_the_recorded_order() {
    for (stage, engine) in stage_vectors() {
        let id = &stage.id;
        let frames = windowed(&engine.input.face_on.frames, engine.input.face_on_window);
        let smoothed = smooth_keypoints(frames, DEFAULT_WINDOW);
        let phases = segment_phases(&smoothed, LEAD_WRIST);
        let (pose, placements, pivots) =
            face_on_measurements(&smoothed, &phases, engine.input.handedness);
        let (shot, flight) = outcome_measurements(&engine);

        let ours: Vec<&str> = pose
            .iter()
            .chain(&placements)
            .chain(&pivots)
            .chain(&shot)
            .chain(&flight)
            .map(|row| row.name.as_str())
            .collect();
        let recorded: Vec<&str> = stage
            .stages
            .measurements
            .iter()
            .flat_map(|group| &group.rows)
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(
            ours,
            recorded[..ours.len()],
            "{id}: the five groups are not the flat list's prefix"
        );
    }
}

/// **The fifteen real swings never reach the trajectory basis**, which is the sharpest thing this
/// phase measured.
///
/// `build_trajectory` refuses a swing whose trail elbow and trail wrist are missing more than 40% of
/// their resampled timeline, and face-on they always are: the trail arm crosses the torso through
/// most of the swing. So every corpus vector records the joint placement alone, and T², Q, the
/// `%g` percentile in their sentences and the interval name in Q's are gated by the **six synthetic
/// vectors** — which have perfect visibility by construction.
///
/// This is meant to fail the day a corpus vector produces a trajectory row, which is the good
/// failure: the answer then is to note that the stage covers the real footage too, not to relax it.
#[test]
fn the_corpus_never_reaches_the_trajectory_basis() {
    let mut with_trajectory: Vec<String> = Vec::new();
    let mut without: Vec<String> = Vec::new();
    for (stage, _) in stage_vectors() {
        let placements = stage
            .stages
            .measurements
            .iter()
            .find(|group| group.group == "placements")
            .expect("every vector records a placements group");
        let names: Vec<&str> = placements.rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names.first(),
            Some(&"tour_joint_distance"),
            "{}: the joint placement is measurable on every committed swing",
            stage.id
        );
        if names.len() > 1 {
            with_trajectory.push(stage.id.clone());
        } else {
            without.push(stage.id.clone());
        }
    }
    assert_eq!(
        with_trajectory.len(),
        6,
        "only the synthetic half reaches the basis: {with_trajectory:?}"
    );
    assert!(
        with_trajectory.iter().all(|id| id.contains("synthetic")),
        "{with_trajectory:?}"
    );
    assert_eq!(
        without.len(),
        15,
        "all fifteen corpus swings refuse it: {without:?}"
    );
    assert!(
        without.iter().all(|id| id.contains("corpus")),
        "{without:?}"
    );
}

/// One of the six does exercise the mirror, and it is worth knowing which.
///
/// P5 found that the corpus's one left-handed vector could not gate `head_stays_back`'s mirror,
/// because its `head_hip_gain_norm` is exactly zero. The trajectory mirror is in better shape:
/// `synthetic/face-on-only` is left-handed **and** is one of the six that reach the basis, so
/// `build_trajectory`'s left/right swap and `x` negation are both on the gated path. Deleting the
/// mirror changes that vector's T² and Q.
#[test]
fn the_left_handed_vector_does_gate_the_trajectory_mirror() {
    let mut left_handed_with_trajectory: Vec<String> = Vec::new();
    for (stage, engine) in stage_vectors() {
        if engine.input.handedness != Some(Handedness::Left) {
            continue;
        }
        let rows = stage
            .stages
            .measurements
            .iter()
            .find(|group| group.group == "placements")
            .map(|group| group.rows.len())
            .unwrap_or(0);
        if rows > 1 {
            left_handed_with_trajectory.push(stage.id.clone());
        }
    }
    assert_eq!(
        left_handed_with_trajectory.len(),
        1,
        "the corpus's left-handed half changed: {left_handed_with_trajectory:?}"
    );
}

/// **Half the pivot registry is answered here and run elsewhere**, pinned rather than left implicit.
///
/// The five `_dtl` specs come in with `contracts::pivots` because a registry lands with its walker,
/// and the walker for *these* five is `_dtl_placements`, which `analyze_swing_bundle` calls and
/// `_measurements` does not. Their answers are on disk — every corpus vector records a five-row
/// `pivot_dtl` group — and P7's `tests/alignment.rs` and `crates/core`'s whole-bundle gate are what
/// run them. What this file owes is the other half: that the group it emits is the face-on five, in
/// registry order, on all 21.
///
/// The arithmetic behind both halves is covered on all 21 either way, because a face-on spec and its
/// `_dtl` partner name the same implementation — which is the whole point of
/// `PivotMeasurementSpec::check`.
#[test]
fn the_dtl_half_of_the_registry_is_gated_from_the_bundle_and_not_from_here() {
    let face_on: Vec<&str> = PIVOT_MEASUREMENT_REGISTRY
        .iter()
        .filter(|spec| spec.view == FACE_ON)
        .map(|spec| spec.name)
        .collect();
    let dtl: Vec<&str> = PIVOT_MEASUREMENT_REGISTRY
        .iter()
        .filter(|spec| spec.view == DOWN_THE_LINE)
        .map(|spec| spec.name)
        .collect();
    assert_eq!(face_on.len(), 5);
    assert_eq!(dtl.len(), 5);

    let mut two_camera = 0;
    for (stage, _) in stage_vectors() {
        let rows = |group_name: &str| -> Vec<String> {
            stage
                .stages
                .measurements
                .iter()
                .filter(|group| group.group == group_name)
                .flat_map(|group| &group.rows)
                .map(|row| row.name.clone())
                .collect()
        };
        assert_eq!(
            rows("pivot_face_on"),
            face_on,
            "{}: `_measurements`' rows are the face-on five, in registry order",
            stage.id
        );
        let recorded = rows("pivot_dtl");
        if !recorded.is_empty() {
            assert_eq!(
                recorded, dtl,
                "{}: the bundle's rows, in registry order",
                stage.id
            );
            two_camera += 1;
        }
    }
    assert_eq!(
        two_camera, 15,
        "every corpus vector has a second camera; the six synthetic ones do not"
    );
}

/// **What the two outcome groups actually gate**, counted rather than inferred from "fifteen
/// vectors". [M22 P8b]
///
/// The `shot` group looks thoroughly covered and is: 104 rows, seven names, fifteen swings. But six
/// of the seven are straight reads present on every one, so the only *conditional* in the group —
/// [`analysis::shot_measure::measure_face_to_path`]'s two-input refusal — rests on the single vector
/// that printed a path and no face angle.
///
/// The `flight` group is the opposite shape: 25 rows across five flights, and the two names that are
/// not unconditional are four rows and one row. `flight_spin_rpm` records on **one** of twenty-one
/// vectors, so the `SpinSource::Inferred` check that ADR-027 §Decision 6 was corrected for is gated
/// by a single row — which is why `flight_measure`'s unit tests carry the rest of it.
///
/// Written as exact counts so that a re-recorded corpus which lost one of them fails here with the
/// number, rather than passing with a gate that has quietly become vacuous.
#[test]
fn the_outcome_groups_are_gated_by_a_handful_of_rows() {
    let mut shot_rows = 0;
    let mut with_shot = 0;
    let mut face_to_path = 0;
    let mut flight_rows = 0;
    let mut flights = 0;
    let mut offline = 0;
    let mut solved_spin = 0;

    for (stage, _) in stage_vectors() {
        let rows = |name: &str| -> Vec<String> {
            stage
                .stages
                .measurements
                .iter()
                .filter(|group| group.group == name)
                .flat_map(|group| &group.rows)
                .map(|row| row.name.clone())
                .collect()
        };
        let shot = rows("shot");
        if !shot.is_empty() {
            with_shot += 1;
            shot_rows += shot.len();
            face_to_path += usize::from(shot.iter().any(|n| n == "face_to_path_deg"));
        }
        let flight = rows("flight");
        if !flight.is_empty() {
            flights += 1;
            flight_rows += flight.len();
            offline += usize::from(flight.iter().any(|n| n == "flight_landing_offline_yds"));
            solved_spin += usize::from(flight.iter().any(|n| n == "flight_spin_rpm"));
        }
    }

    assert_eq!((with_shot, shot_rows, face_to_path), (15, 104, 14));
    assert_eq!((flights, flight_rows, offline, solved_spin), (5, 25, 4, 1));
}

/// Thirteen pose metrics on every one of the 21, which is what makes the `pose` group's coverage
/// complete and its *refusal* coverage zero — the same hole `tests/geometry.rs` pins one layer down,
/// restated here because this is the group that turns a refusal into a missing row.
#[test]
fn every_vector_records_all_thirteen_pose_rows() {
    for (stage, _) in stage_vectors() {
        let pose = stage
            .stages
            .measurements
            .iter()
            .find(|group| group.group == "pose")
            .expect("every vector records a pose group");
        assert_eq!(
            pose.rows.len(),
            13,
            "{}: a vector that refuses a pose metric would gate the absent-row path",
            stage.id
        );
    }
}
