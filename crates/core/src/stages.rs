//! The stage document, produced in Rust. [M32 P2]
//!
//! `spec/vectors/stages/` records `analyze_swing_bundle`'s intermediates at the seven function
//! boundaries it already has, and until M32 only Python could write one: `conformance.py::run_stages`
//! assembled them and `cmd_regenerate --stages-only` wrote them. ADR-035 clause 3 moves the oracle to
//! Rust, and a re-record that cannot produce a stage vector cannot re-record that family — so
//! [`run_stages`] is that function, port for port, returning the same `stages` object a committed
//! stage vector holds.
//!
//! # Why here and not in `analysis`
//!
//! **It is vector plumbing.** The seven keys, the group names and the `[x, y]`-only `smoothed` are
//! choices about what a committed file holds, and `analysis` stays a library that knows nothing
//! about `spec/` (§M32 "The stage document gets a Rust producer"). The same argument decides the
//! `flight` stage's shape: `FlownShot`, `ShotFlight` and `FlightResult` carry no `Serialize`, and
//! deriving one on them would put the vector's JSON into the engine's types. The shape is built by
//! hand below instead, field for field against what `_jsonable` writes.
//!
//! # Why it calls the engine's own functions, private ones included
//!
//! For the reason `run_stages`' docstring gives: a second copy of `_windowed`'s clamping or of
//! `_measurements`' grouping, living here, is a thing that drifts from the definition it is supposed
//! to be recording. So `engine`'s `windowed`, `camera_id` and `anchored_on_strike` are `pub` for this
//! module (M32's decision 3), as Python's `run_stages` imports `E._windowed` and the rest — and
//! `shifted` is, for [`verify_compose`]. What this module owns is the *orchestration* — which calls,
//! in which order — and that is the one thing it can still get wrong.
//!
//! **It is a second copy of that orchestration**, beside the one each of `crates/analysis/tests/`'
//! stage gates carries. That is deliberate: those gate the port one stage at a time and stay
//! independent of this file, and the committed vectors are what keeps the two copies in step.
//!
//! # Why a stage document is checked against the bundle, not only against itself [M32 P3]
//!
//! A stage vector that agrees with [`run_stages`] says only that `run_stages` is deterministic. From
//! M32 this module also *writes* the family (the re-record), so the objection
//! `_verify_stages_compose` was written to answer is now Rust's to answer: a recorder that drifted
//! from `analyze_swing_bundle` would record its drift, and every later comparison would agree with
//! it. [`verify_compose`] is the answer, ported — each stage that reaches the artifact is composed
//! forward and compared with the engine vector's `expected` under the same [`compare`] a port is
//! judged by, so the recorder is held to the *other* copy of the orchestration, the engine's own.
//!
//! # Three things a port of this gets wrong
//!
//! - **Every frame index is window-relative.** The face-on clip is sliced first and nothing here
//!   shifts it back, which is what `analyze_swing_bundle` does *after* these calls. Only
//!   `synthetic/windowed` carries a window, so it is the one vector that fails a recorder which
//!   forgets — or one which "helpfully" shifts.
//! - **`measurements` is a positional partition of one list, never a grouping by `source`.** The
//!   face-on pivot rows carry `pose:face_on` like the pose rows, and both placement families carry
//!   `population:golfdb`; `source` answers "which instrument read this", which is not "which
//!   function appended it". See `partition`, below.
//! - **`smoothed` records `x` and `y` and nothing else.** `smooth_keypoints` copies `z`,
//!   `visibility`, `frame_index` and `timestamp_ms` through untouched, so recording them would record
//!   the input twice; `run_stages` says what that cost (2.3 MB gzipped) and made the trade.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use analysis::alignment::{align_swings, anchors_from_keypoints, anchors_from_phases};
use analysis::benchmarks::flight_model::load_flight_model;
use analysis::checkpoints::{evaluator_for, CheckpointOutcome};
use analysis::engine::{
    anchored_on_strike, camera_id, dtl_placements, face_on_measurements, flight_measurements,
    measurements, pivot_measurements, shifted, shot_measurements, windowed,
};
use analysis::flight::{FlightPoint, FlightResult, LaunchConditions, DEFAULT_STEP_S};
use analysis::flight_infer::{InferredSpin, InferredSpinAxis, ShotFlight};
use analysis::flight_measure::{flight_unscored, fly_shot, FlownShot};
use analysis::measure::POSE_MEASUREMENTS;
use analysis::phases::{segment_phases, LEAD_WRIST, TRAIL_WRIST};
use analysis::pyfmt::round_to;
use analysis::smoothing::{smooth_keypoints, DEFAULT_WINDOW};
use analysis::spin_solve::{CarryWindow, SpinSolution, UnspunLaunch};
use contracts::checkpoints::CHECKPOINT_REGISTRY;
use contracts::keypoints::{FrameKeypoints, KeypointsFile};
use contracts::placements::DOWN_THE_LINE;
use contracts::swing::{Measurement, PhaseSegment};

use crate::compare::{compare, Difference};
use crate::VectorInput;

/// The seven keys of a stage vector's `stages` object. `conformance.STAGE_NAMES`.
///
/// `feedback` is not one, for the reason `STAGE_NAMES`' comment gives: `build_feedback` takes the
/// assembled `SwingResult`, which no stage produces, so the engine vector's `expected.feedback`
/// already gates it.
pub const STAGE_NAMES: [&str; 7] = [
    "smoothed",
    "phases",
    "measure",
    "measurements",
    "checkpoints",
    "alignment",
    "flight",
];

/// How `measurements` is grouped, in the order the engine concatenates the groups.
/// `conformance.MEASUREMENT_GROUPS`.
///
/// An array rather than a map because the order *is* part of the answer: `swing.measurements` is one
/// flat list compared positionally, and the stage records it as a list of `{group, rows}` objects
/// precisely so the order survives a writer that sorts keys. The first five are `_measurements`'
/// own; the two `_dtl` groups are `analyze_swing_bundle`'s, appended after it.
pub const MEASUREMENT_GROUPS: [&str; 7] = [
    "pose",
    "placements",
    "pivot_face_on",
    "shot",
    "flight",
    "placements_dtl",
    "pivot_dtl",
];

/// Run one engine vector's input and return the engine's intermediates, keyed by stage.
/// `conformance.py::run_stages`.
///
/// The result is the `stages` object of the stage vector derived from that engine vector, and
/// `tests/stages.rs` holds it to every committed one under `docs/CONFORMANCE.md` §3's rules.
pub fn run_stages(input: &VectorInput) -> Value {
    let intent = input.intent.clone().unwrap_or_default();
    let handedness = input.handedness;

    let (_start, frames) = windowed(&input.face_on.frames, input.face_on_window);
    let smoothed = smooth_keypoints(frames, DEFAULT_WINDOW);
    let phases = segment_phases(&smoothed, LEAD_WRIST);

    // The unrounded pose numbers and the reason for each that refused — both things the bundle
    // cannot show, because `_measurements` rounds to four places on the way into a `Measurement`
    // and drops a refusal with no record of which gate failed. This is the only committed evidence
    // of the pre-rounding float, which is what makes ADR-032 §3's banker's-rounding edge diagnosable
    // rather than merely detectable.
    let measure: Vec<Value> = POSE_MEASUREMENTS
        .iter()
        .map(|(name, pose)| {
            let outcome = (pose.measure)(&smoothed, &phases);
            json!({
                "name": name,
                "value": outcome.value,
                "reason": outcome.reason,
                "detail": outcome.detail,
            })
        })
        .collect();

    // A scored row carries `reason: null` and `detail: ""` because Python's
    // `CheckpointOutcome.scored(score)` leaves both at their defaults. `CheckpointOutcome` here is an
    // enum and has no detail to read on that arm, so the empty string is written rather than read —
    // and it is what all 21 committed vectors hold for every scored row.
    let checkpoints: Vec<Value> = CHECKPOINT_REGISTRY
        .iter()
        .map(|spec| {
            match evaluator_for(spec.name)(&smoothed, &phases, handedness, intent.club, None) {
                CheckpointOutcome::Scored(score) => json!({
                    "name": spec.name,
                    "score": score,
                    "reason": null,
                    "detail": "",
                }),
                CheckpointOutcome::Unscored { reason, detail } => json!({
                    "name": spec.name,
                    "score": null,
                    "reason": reason,
                    "detail": detail,
                }),
            }
        })
        .collect();

    // Flown once and read twice, by the `flight` group and by `flight_unscored`, as `analyze_swing`
    // does — two flights would be two answers nothing guarantees are the same.
    let flown = input.shot.as_ref().map(|shot| {
        fly_shot(
            shot,
            input.loft_deg,
            handedness,
            load_flight_model(),
            DEFAULT_STEP_S,
        )
    });
    let every = measurements(
        &smoothed,
        &phases,
        input.shot.as_ref(),
        handedness,
        flown.as_ref(),
    );
    let [pose, placements, pivot_face_on, shot, flight] =
        partition(&every, &smoothed, &phases, input, flown.as_ref());

    // The two views' anchors, before and after the ball strike moves the impact one. The pinned pair
    // is what `align_swings` receives; the unpinned pair is where `alignment`'s frame-index roundings
    // live, so recording both tells a port *which half* it got wrong. The notes the pinning writes
    // are discarded, as `run_stages` discards its own: they are the bundle's, and its `notes` gate
    // them.
    let mut notes: Vec<String> = Vec::new();
    let face_from_phases =
        anchors_from_phases(&phases, input.face_on.clip.as_ref(), camera_id(frames), 0);
    let face_anchors = anchored_on_strike(
        face_from_phases.clone(),
        input.face_on_strikes.as_deref(),
        "face-on",
        &mut notes,
    );
    let dtl_from_keypoints = input.down_the_line.as_ref().and_then(|dtl| {
        anchors_from_keypoints(
            &dtl.frames,
            dtl.clip.as_ref(),
            input.down_the_line_window,
            Some(TRAIL_WRIST),
        )
    });
    let dtl_anchors = anchored_on_strike(
        dtl_from_keypoints.clone(),
        input.down_the_line_strikes.as_deref(),
        "down-the-line",
        &mut notes,
    );

    // The rear clip's two groups, off one smoothing pass and one anchor tuple shared by both readers
    // — `analyze_swing_bundle`'s construction, so the trajectory and pivot paths resample onto the
    // same numbers here as they do there.
    let (mut placements_dtl, mut pivot_dtl) = (Vec::new(), Vec::new());
    if let (Some(dtl), Some(anchors)) = (input.down_the_line.as_ref(), dtl_anchors.as_ref()) {
        let dtl_frames = smooth_keypoints(&dtl.frames, DEFAULT_WINDOW);
        let events = (
            anchors.motion_start as f64,
            anchors.top as f64,
            anchors.impact as f64,
        );
        placements_dtl = dtl_placements(&dtl_frames, events, handedness);
        pivot_dtl = pivot_measurements(&dtl_frames, events, DOWN_THE_LINE);
    }

    let clip_alignment = match (face_anchors.as_ref(), dtl_anchors.as_ref()) {
        (Some(face), Some(dtl)) => Some(align_swings(face, dtl)),
        _ => None,
    };

    let groups: [&[Measurement]; 7] = [
        pose,
        placements,
        pivot_face_on,
        shot,
        flight,
        &placements_dtl,
        &pivot_dtl,
    ];
    let smoothed_xy: Vec<Vec<[f64; 2]>> = smoothed
        .iter()
        .map(|frame| frame.landmarks.iter().map(|lm| [lm.x, lm.y]).collect())
        .collect();

    json!({
        "smoothed": smoothed_xy,
        "phases": phases,
        "measure": measure,
        "measurements": MEASUREMENT_GROUPS
            .iter()
            .zip(groups)
            .map(|(group, rows)| json!({"group": group, "rows": rows}))
            .collect::<Vec<_>>(),
        "checkpoints": checkpoints,
        "alignment": {
            "face_anchors_from_phases": face_from_phases,
            "face_anchors": face_anchors,
            "dtl_anchors_from_keypoints": dtl_from_keypoints,
            "dtl_anchors": dtl_anchors,
            "clip_alignment": clip_alignment,
        },
        "flight": {
            "flown": flown.as_ref().map(flown_json),
            "unscored": flown.as_ref().map(flight_unscored).unwrap_or_default(),
        },
    })
}

/// `measurements`' one list cut into its first five groups, by the sub-calls' **lengths**.
///
/// The rows come off the one real list in the one real order; the sub-calls are run only to be
/// counted. That is `run_stages`' rule, and it found the reason the hard way: grouping by `source`
/// put eighteen rows in `pose` against thirteen pose measurements.
///
/// **Every cut is measured, where Python's `pose` is the remainder.** Python's closing assert is
/// therefore true by construction; here the five lengths have to add up to the list's, so the assert
/// below is a check that `measurements` is still exactly the concatenation of these five calls. The
/// shot cut is `shot_measurements(shot).len()` rather than Python's count of rows whose `source` is
/// `launch_monitor:{device}`, which keeps the device rule out of this file entirely: restating it
/// here would be a second place to update when the rule moves.
fn partition<'a>(
    every: &'a [Measurement],
    smoothed: &[FrameKeypoints],
    phases: &[PhaseSegment],
    input: &VectorInput,
    flown: Option<&FlownShot>,
) -> [&'a [Measurement]; 5] {
    let (pose, placed, pivots) = face_on_measurements(smoothed, phases, input.handedness);
    let cuts = [
        pose.len(),
        placed.len(),
        pivots.len(),
        input
            .shot
            .as_ref()
            .map_or(0, |shot| shot_measurements(shot).len()),
        flown.map_or(0, |flown| flight_measurements(flown).len()),
    ];
    assert_eq!(
        cuts.iter().sum::<usize>(),
        every.len(),
        "the measurement groups do not partition `measurements`: {cuts:?} against {} rows",
        every.len()
    );
    let mut at = 0;
    cuts.map(|size| {
        let group = &every[at..at + size];
        at += size;
        group
    })
}

// --------------------------------------------------------------------------- the compose check

/// Check that a stage document adds back up to its engine vector's bundle answer.
/// `conformance_vectors._verify_stages_compose`, check for check and in its order.
///
/// `engine_vector` is a whole engine vector: `input` for the clip and its window, `expected` for the
/// answer. When the re-record calls this it passes the engine document **as it will be written**,
/// not as committed (M32's call 9) — in M32 the two agree on everything read here, and the order is
/// the right one the first time a declaration moves a composed value.
///
/// `Err` is one sentence naming the vector, the stage and the first place it failed to land, as the
/// Python raises on its first difference. Every refusal goes through one formatter, `refused`, so the
/// stage is always named: the sentence says *where* the recorder and the engine part ways, which is
/// the first thing a reader of a red re-record needs, rather than only that they do.
///
/// Two stages cannot be composed whole and are named rather than skipped silently, as the Python
/// names them: `smoothed` reaches `expected` only through `phases` and `measure`, so its pass-through
/// property is asserted directly instead; `alignment`'s four anchor tuples reach it only through
/// `clip_alignment`, which *is* compared.
pub fn verify_compose(stages: &Value, engine_vector: &Value) -> Result<(), String> {
    let id = engine_vector["id"]
        .as_str()
        .unwrap_or("<an engine vector with no id>");
    let given = &engine_vector["input"];
    let expected = &engine_vector["expected"];
    let swing = &expected["swing"];

    // `smoothed`: the four fields it does not record are checked to be copies, which is what makes
    // not recording them honest rather than merely cheap. `camera_id` is *not* among them —
    // `smooth_keypoints` drops it (`smoothing.rs`' module doc), and nothing reads it back, because
    // `engine::camera_id` asks the raw frames. Only the frame count of what *was* recorded is checked
    // here, as in the Python: the `x`/`y` values are `run_stages`' gate's business, not this one's.
    let face_on: KeypointsFile = parse(&given["face_on"]).map_err(|e| {
        refused(
            id,
            "smoothed",
            format!("the face-on clip does not parse: {e}"),
        )
    })?;
    let window: Option<(i64, i64)> = parse(&given["face_on_window"]).map_err(|e| {
        refused(
            id,
            "smoothed",
            format!("the face-on window does not parse: {e}"),
        )
    })?;
    let (start, frames) = windowed(&face_on.frames, window);
    let smoothed = smooth_keypoints(frames, DEFAULT_WINDOW);
    let recorded = list(stages, "smoothed").map_err(|why| refused(id, "smoothed", why))?;
    if recorded.len() != frames.len() || smoothed.len() != frames.len() {
        return Err(refused(
            id,
            "smoothed",
            format!(
                "{} frames recorded and {} smoothed, from a window of {}",
                recorded.len(),
                smoothed.len(),
                frames.len()
            ),
        ));
    }
    for (raw, got) in frames.iter().zip(&smoothed) {
        if (got.frame_index, got.timestamp_ms) != (raw.frame_index, raw.timestamp_ms) {
            return Err(refused(
                id,
                "smoothed",
                format!(
                    "smoothing moved frame {}'s index or timestamp, which it does not do",
                    raw.frame_index
                ),
            ));
        }
        if got.camera_id.is_some() {
            return Err(refused(
                id,
                "smoothed",
                "smoothing is expected to drop camera_id",
            ));
        }
        let copied = got.landmarks.len() == raw.landmarks.len()
            && raw
                .landmarks
                .iter()
                .zip(&got.landmarks)
                .all(|(a, b)| (a.z, a.visibility) == (b.z, b.visibility));
        if !copied {
            return Err(refused(
                id,
                "smoothed",
                format!(
                    "smoothing moved a z or a visibility on frame {}, which it does not do — the \
                     stage records x/y only and that is no longer safe",
                    raw.frame_index
                ),
            ));
        }
    }

    // `phases`: window-relative in the stage, whole-clip in the artifact. Shifting them back is what
    // exercises `engine::shifted`, and `synthetic/windowed` is the only vector where it moves them.
    let mut whole_clip = Vec::new();
    for phase in list(stages, "phases").map_err(|why| refused(id, "phases", why))? {
        let segment: PhaseSegment = parse(phase)
            .map_err(|e| refused(id, "phases", format!("a row does not parse: {e}")))?;
        whole_clip
            .push(serde_json::to_value(shifted(&segment, start)).expect("a phase serializes"));
    }
    lands(
        id,
        "phases",
        "phases",
        &swing["phases"],
        &Value::Array(whole_clip),
    )?;

    // `measure`: the unrounded float, rounded the way `_measurements` rounds it, must be the row that
    // reached the artifact. Python's `round(x, 4)` is `pyfmt::round_to` and never a scaled
    // `f64::round` — ADR-032 §3's banker's-rounding edge, which this puts under test on every vector
    // rather than leaving for a port to discover.
    let groups = list(stages, "measurements").map_err(|why| refused(id, "measurements", why))?;
    let pose_rows: BTreeMap<&str, &Value> = groups
        .iter()
        .filter(|group| group["group"] == "pose")
        .flat_map(|group| group["rows"].as_array().into_iter().flatten())
        .map(|row| (row["name"].as_str().unwrap_or_default(), row))
        .collect();
    for row in list(stages, "measure").map_err(|why| refused(id, "measure", why))? {
        let name = row["name"].as_str().unwrap_or_default();
        let stored = pose_rows.get(name);
        if row["value"].is_null() {
            if stored.is_some() {
                return Err(refused(
                    id,
                    "measure",
                    format!("{name} refused in `measure` but reached `measurements`"),
                ));
            }
            if !matches!(&row["reason"], Value::String(reason) if !reason.is_empty()) {
                return Err(refused(
                    id,
                    "measure",
                    format!("{name} refused with no reason"),
                ));
            }
            continue;
        }
        let Some(stored) = stored else {
            return Err(refused(
                id,
                "measure",
                format!("{name} measured but absent from `measurements`"),
            ));
        };
        let Some(value) = row["value"].as_f64() else {
            return Err(refused(
                id,
                "measure",
                format!("{name}'s value is not a number: {}", row["value"]),
            ));
        };
        lands(
            id,
            "measure",
            &format!("measure.{name}"),
            &stored["value"],
            &json!(round_to(value, 4)),
        )?;
    }

    // `measurements`: the groups are a partition of one flat list, so concatenating them in the
    // recorded order is the whole check.
    let flat: Vec<Value> = groups
        .iter()
        .flat_map(|group| group["rows"].as_array().into_iter().flatten())
        .cloned()
        .collect();
    lands(
        id,
        "measurements",
        "measurements",
        &swing["measurements"],
        &Value::Array(flat),
    )?;

    // `checkpoints`: containment rather than equality, and deliberately. Two things happen to this
    // stage's answer after it — `flight_unscored` extends `unscored`, and a late top withdraws a score
    // under `cross_view_contradicted` (`engine`'s `without_contradicted_scores`). Composing those
    // forward here would be a third copy of the engine; checking that every outcome landed somewhere
    // it is allowed to land tests the same thing without one.
    let scored = by_name(&swing["checkpoint_scores"]);
    let unscored = by_name(&swing["unscored"]);
    for row in list(stages, "checkpoints").map_err(|why| refused(id, "checkpoints", why))? {
        let name = row["name"].as_str().unwrap_or_default();
        let at = format!("checkpoints.{name}");
        if !row["score"].is_null() {
            match scored.get(name) {
                Some(landed) => lands(id, "checkpoints", &at, landed, &row["score"])?,
                None if unscored
                    .get(name)
                    .is_some_and(|entry| entry["reason"] == "cross_view_contradicted") => {}
                None => {
                    return Err(refused(
                        id,
                        "checkpoints",
                        format!(
                            "{name} scored, but is neither in `checkpoint_scores` nor withdrawn \
                             by a late top"
                        ),
                    ))
                }
            }
            continue;
        }
        let Some(landed) = unscored.get(name) else {
            return Err(refused(
                id,
                "checkpoints",
                format!("{name} refused but is not in `unscored`"),
            ));
        };
        lands(
            id,
            "checkpoints",
            &format!("{at}.reason"),
            &landed["reason"],
            &row["reason"],
        )?;
    }

    // `alignment`: the anchors are not in the artifact, but the warp built from them is.
    lands(
        id,
        "alignment",
        "alignment",
        &expected["alignment"],
        &stages["alignment"]["clip_alignment"],
    )?;

    // `flight`: the refusals it produces are appended to `unscored` verbatim.
    let flight_unscored = stages["flight"]["unscored"]
        .as_array()
        .ok_or_else(|| refused(id, "flight", "`unscored` is missing or not a list"))?;
    for entry in flight_unscored {
        let name = entry["name"].as_str().unwrap_or_default();
        let Some(landed) = unscored.get(name) else {
            return Err(refused(
                id,
                "flight",
                format!("the flight refused {name} and the artifact does not say so"),
            ));
        };
        lands(
            id,
            "flight",
            &format!("flight.unscored.{name}"),
            landed,
            entry,
        )?;
    }
    Ok(())
}

/// One refusal, in the form every check above uses: vector, stage, why.
fn refused(id: &str, stage: &str, why: impl std::fmt::Display) -> String {
    format!("{id}: the `{stage}` stage does not compose onto `expected` — {why}")
}

/// `expected` and `actual` agree under §3's rules, or the first difference refused under `stage`.
///
/// `at` prefixes the difference's path, as `compare_results`' third argument does, so the sentence
/// says `measure.tempo_ratio` rather than `<root>`. The prefix is for the reader only: [`compare`]
/// is called on the two sub-documents, so its own paths stay in call 3's spelling.
fn lands(id: &str, stage: &str, at: &str, expected: &Value, actual: &Value) -> Result<(), String> {
    let differences = compare(expected, actual);
    let Some(first) = differences.first() else {
        return Ok(());
    };
    let path = match first.path.as_str() {
        "" => at.to_string(),
        index if index.starts_with('[') => format!("{at}{index}"),
        key => format!("{at}.{key}"),
    };
    let more = match differences.len() {
        1 => String::new(),
        n => format!(" (and {} more)", n - 1),
    };
    let first = Difference {
        path,
        kind: first.kind.clone(),
    };
    Err(refused(id, stage, format!("{first}{more}")))
}

/// A stage that must be a list, or why it is not.
fn list<'a>(stages: &'a Value, stage: &str) -> Result<&'a Vec<Value>, String> {
    stages[stage]
        .as_array()
        .ok_or_else(|| format!("the stage is missing or not a list: {}", stages[stage]))
}

/// A list of `{name, …}` objects keyed by name — the Python's dict comprehension, later rows winning.
fn by_name(rows: &Value) -> BTreeMap<&str, &Value> {
    rows.as_array()
        .into_iter()
        .flatten()
        .map(|row| (row["name"].as_str().unwrap_or_default(), row))
        .collect()
}

/// A contract type read out of a borrowed `Value` through its **trait** `Deserialize`.
///
/// Written generically because the contract types derive `#[serde(remote = "Self")]` and get their
/// validation from `contracts::validated!`'s trait impl: the derive also leaves an *inherent*
/// `T::deserialize`, which method resolution prefers at a concrete call site and which skips
/// `Validate`. Through a type parameter only the trait method is in scope.
fn parse<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, serde_json::Error> {
    T::deserialize(value)
}

// --------------------------------------------------------------------------- the `flight` stage
//
// `_jsonable`'s expansion of the three NamedTuples and what they hold, by hand. Python's `_asdict`
// recurses into every field, so every field is here: a field left out is a removed key under §3, and
// that is how this half would fail rather than silently thin out. Enums are their wire strings —
// `UnscoredReason` and `TargetShape` through their serde names, the three analysis-side enums
// through their `as_str`, which is the same string `StrEnum.value` writes.

fn flown_json(flown: &FlownShot) -> Value {
    json!({
        "resolved": flown.resolved.as_ref().map(shot_flight_json),
        "flight": flown.flight.as_ref().map(flight_json),
        "reason": flown.reason,
        "detail": flown.detail,
    })
}

fn shot_flight_json(resolved: &ShotFlight) -> Value {
    json!({
        "launch": resolved.launch.as_ref().map(launch_json),
        "spin_source": resolved.spin_source.map(|source| source.as_str()),
        "spin": resolved.spin.as_ref().map(spin_json),
        "axis": axis_json(&resolved.axis),
        "reason": resolved.reason,
        "detail": resolved.detail,
    })
}

fn axis_json(axis: &InferredSpinAxis) -> Value {
    json!({
        "spin_axis_deg": axis.spin_axis_deg,
        "source": axis.source.map(|source| source.as_str()),
        "reason": axis.reason,
        "detail": axis.detail,
        "curve_direction": axis.curve_direction,
        "screen_shape": axis.screen_shape,
    })
}

fn spin_json(spin: &InferredSpin) -> Value {
    json!({
        "spin_rpm": spin.spin_rpm,
        "reason": spin.reason,
        "detail": spin.detail,
        "solution": solution_json(&spin.solution),
    })
}

fn solution_json(solution: &SpinSolution) -> Value {
    json!({
        "case": solution.case.as_str(),
        "target_carry_yds": solution.target_carry_yds,
        "window": window_json(&solution.window),
        "rising_rpm": solution.rising_rpm,
        "falling_rpm": solution.falling_rpm,
    })
}

fn window_json(window: &CarryWindow) -> Value {
    json!({
        "launch": unspun_json(&window.launch),
        "low_plateau_yds": window.low_plateau_yds,
        "low_plateau_max_rpm": window.low_plateau_max_rpm,
        "peak_yds": window.peak_yds,
        "peak_rpm": window.peak_rpm,
        "high_plateau_yds": window.high_plateau_yds,
        "high_plateau_min_rpm": window.high_plateau_min_rpm,
    })
}

fn unspun_json(launch: &UnspunLaunch) -> Value {
    json!({
        "ball_speed_mph": launch.ball_speed_mph,
        "launch_angle_deg": launch.launch_angle_deg,
        "launch_direction_deg": launch.launch_direction_deg,
        "spin_axis_deg": launch.spin_axis_deg,
    })
}

fn launch_json(launch: &LaunchConditions) -> Value {
    json!({
        "ball_speed_mph": launch.ball_speed_mph,
        "launch_angle_deg": launch.launch_angle_deg,
        "spin_rpm": launch.spin_rpm,
        "launch_direction_deg": launch.launch_direction_deg,
        "spin_axis_deg": launch.spin_axis_deg,
    })
}

/// The whole path rather than its summary, which is the stage's own choice (M22 P1): a divergence is
/// then *located* — a wrong Magnus sign shows in the first fifty points, not as a carry three yards
/// out.
fn flight_json(flight: &FlightResult) -> Value {
    json!({
        "launch": launch_json(&flight.launch),
        "points": flight.points.iter().map(point_json).collect::<Vec<_>>(),
        "carry_m": flight.carry_m,
        "curvature_m": flight.curvature_m,
        "landing_offline_m": flight.landing_offline_m,
        "apex_m": flight.apex_m,
        "flight_time_s": flight.flight_time_s,
        "descent_angle_deg": flight.descent_angle_deg,
        "clamped_points": flight.clamped_points,
        "spin_ratio_min": flight.spin_ratio_min,
        "spin_ratio_max": flight.spin_ratio_max,
    })
}

fn point_json(point: &FlightPoint) -> Value {
    json!({
        "t_s": point.t_s,
        "x_m": point.x_m,
        "y_m": point.y_m,
        "z_m": point.z_m,
        "vx_m_s": point.vx_m_s,
        "vy_m_s": point.vy_m_s,
        "vz_m_s": point.vz_m_s,
        "spin_rpm": point.spin_rpm,
        "spin_ratio": point.spin_ratio,
        "clamped": point.clamped,
    })
}
