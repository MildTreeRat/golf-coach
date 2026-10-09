//! The shell: `conformance.py::run_vector`'s seam, in Rust [M22 P6] — and from M32 the recorder of
//! the vectors that seam is judged by.
//!
//! One vector's `input` in, the serialized `SwingBundleResult` out — which is *the whole of what a
//! port has to reimplement* (M19). `docs/CONFORMANCE.md` §4 calls `run` "the cross-language seam: a
//! Rust implementation is diffed by a shell pipeline, with no Python in the loop but the reference",
//! and this crate plus [`bin/golf-core`](../golf_core/index.html) is that implementation's end of
//! the pipe.
//!
//! # Why this is a third crate
//!
//! **Two calls, not one, and the second is not optional.** `analysis` may not import `feedback`
//! (ADR-008), so `analyze_swing_bundle` leaves `SwingBundleResult.feedback` as `None` and
//! `api/pipeline.py:1259` fills it in immediately afterwards — which means the artifact this repo
//! actually writes has ranked tips in it and a bare engine call does not. ADR-032 §1 asks cargo to
//! enforce that separation as a dependency edge, so `analysis` and `feedback` cannot reach each
//! other and something above both has to make the two calls. In Python that something is `api/` and
//! `scripts/`; here it is this crate.
//!
//! §1 says *two* crates and this makes four. The amendment is recorded in ADR-032's M22 P6 addendum
//! rather than left as a surprise: the two the ADR names are the two that hold the **port**, and the
//! shell is neither of them — it is the counterpart of `api/pipeline.py`, which §7 retires into the
//! Flutter shell instead of porting.
//!
//! # What the comparison is, and what it is not
//!
//! Structural, over parsed values, and never byte-for-byte. Python writes
//! `-1.636758133827243e-05` where `serde_json` writes `-0.00001636758133827243` for the identical
//! f64, and seventy-seven distinct floats in the committed vectors take the exponent form (M22 P2).
//! So **`run`'s stdout seam needs no formatter** — `docs/CONFORMANCE.md` §4 says so — and the
//! diffing is `compare_results`' job on whichever side runs it. [`compare`] implements §3's rules
//! for the Rust side, as library code since M32 because the re-record gates on its answers, and
//! `tests/engine.rs` is the gate that runs it over the committed vectors.
//!
//! # The stage document
//!
//! [`stages::run_stages`] is `conformance.py::run_stages`' counterpart: the same input, the
//! engine's seven intermediates out, as a stage vector's `stages` object (M32). It is here rather
//! than in `analysis` because what a committed file holds is vector plumbing, and the engine stays a
//! library that knows nothing about `spec/`. `tests/stages.rs` holds it to every committed stage
//! vector.
//!
//! # The re-record
//!
//! [`rerecord`] is how the committed engine and stage vectors are re-recorded from M32 (ADR-035
//! clause 3), and `golf-core rerecord` is its verb. Its rules are pure functions over
//! `serde_json::Value`: [`compare`]'s typed differences matched against declared paths, applied onto
//! the *committed* document so every undeclared value keeps the bits Python recorded, and written
//! down in the vector's own `provenance`. Its run, [`rerecord::plan`] and [`rerecord::Run::write`],
//! walks `spec/vectors/` with them — every vector gated and composed before any file is written.
//! Since M34 P7 it re-records the screen family too, through [`rerecord::run_screen`], one family
//! per run as the declaration's version key says, and since M36 P13 the storage and career
//! families together, under `career_version`.
//!
//! # The many-shot layer's runners
//!
//! [`storage_family::run_storage`] and [`career_family::run_career`] answer one vector of the
//! storage and career families (M36). Each is the definition its gate (`tests/storage.rs`,
//! `tests/career.rs`) and the re-record share, so a re-record writes what the gate checks. They are
//! why this crate depends on `storage`, which is the one crate allowed to hold it beside `analysis`
//! (ADR-008 as cargo edges).
//!
//! # The career reports and their verbs
//!
//! [`reports`] renders what the career scripts print (M36 P15–P16), from the aggregates alone, so
//! [`career_family::run_career`] records the text into the career family and the verbs print it over
//! `data/`: `golf-core career-corpus`, `career-baseline`, `career-dispersion` and `club-profile`,
//! and `flag-mishit`, the one that writes. The verbs' `main` (who to report on, which data
//! directories, and `flag-mishit`'s flag and listing) is there too; `bin/golf_core.rs` parses flags
//! and prints.
//!
//! # `ANALYSIS_VERSION` is stamped here once, and only through the gate
//!
//! `analyze_swing_bundle` sets the answer's, because it is the thing that did the work. A vector's
//! *top-level* `analysis_version` says which engine recorded the file, and only the re-record writes
//! it: it substitutes the constant into Rust's side of the comparison, so the move reaches the file
//! only if the declaration names it, as every other move does. The staleness check — a vector
//! recorded by another version than this build's — belongs to the gates, `tests/engine.rs` and
//! `tests/stages.rs`.

pub mod career_family;
pub mod compare;
pub mod reports;
pub mod rerecord;
pub mod stages;
pub mod storage_family;

use serde::Deserialize;

use analysis::engine::{analyze_swing_bundle, BundleRequest};
use contracts::golfer::Handedness;
use contracts::intent::PracticeGoal;
use contracts::keypoints::KeypointsFile;
use contracts::shot::ShotData;
use feedback::rules::build_feedback;

/// One engine vector's `input` object: the thirteen arguments `analyze_swing_bundle` takes, as JSON.
///
/// The field set and the defaults are `conformance.py::run_vector`'s reads of `given`, one for one.
/// Two of its coercions have no counterpart here because serde already does them: `_handedness`
/// turns `"right"` into the enum, and `_tuple` turns a two-element list into a tuple — JSON has
/// neither an enum nor a tuple, and both are declared types on this struct instead of conversions at
/// the call site.
///
/// **An absent key and an explicit `null` both read as `None`, and here that is right.** P2's
/// round-trip harness needed `Option<Option<T>>` to tell them apart, because it wrote the input back
/// out and ten corpus vectors carry `"loft_deg": null` where `windowed.json` simply has no `shot`.
/// This struct is consumed and never re-emitted, so the distinction has nothing to preserve.
///
/// One difference from Python worth naming rather than hiding: `run_vector` tests `if given.get(…)`,
/// which is *falsy* rather than `is None`, so an empty `{}` for `shot` or `down_the_line` would be
/// read as absent there and as present-and-invalid here. No committed vector carries one, and the
/// Python behaviour is an accident of `or` rather than a rule anybody wrote down.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorInput {
    pub swing_id: String,
    pub session_id: String,
    pub face_on: KeypointsFile,
    #[serde(default)]
    pub down_the_line: Option<KeypointsFile>,
    #[serde(default)]
    pub shot: Option<ShotData>,
    #[serde(default)]
    pub intent: Option<PracticeGoal>,
    #[serde(default)]
    pub face_on_window: Option<(i64, i64)>,
    #[serde(default)]
    pub down_the_line_window: Option<(i64, i64)>,
    #[serde(default)]
    pub face_on_strikes: Option<Vec<i64>>,
    #[serde(default)]
    pub down_the_line_strikes: Option<Vec<i64>>,
    #[serde(default)]
    pub handedness: Option<Handedness>,
    #[serde(default)]
    pub loft_deg: Option<f64>,
}

/// A whole vector file, of which only `input` is read. `expected` is the gate's business.
#[derive(Debug, Clone, Deserialize)]
pub struct Vector {
    pub input: VectorInput,
}

/// What `api/pipeline.py::analyze_swing_dir` drops before writing `analysis.json`.
///
/// Restated here because a port has to make the same drop and has no reason to guess at it — the
/// same restatement `conformance.py`'s `EXCLUDED_FROM_RESULT` is, and M19's finding about it stands:
/// **this lives at a call site and in no schema**, so `spec/schemas/swing_bundle_result.json`
/// describes a shape that carries keypoints and every committed vector's `expected` does not.
///
/// Keypoints are the *input*, echoed back on the result as the data it was computed from;
/// round-tripping them through the comparison would make every vector 30x larger and check nothing.
const EXCLUDED_FROM_SWING: &[&str] = &["keypoints", "detections"];

/// Run one vector's input through this build and return the serialized result. `run_vector`.
///
/// Everything else in this crate is plumbing around these four lines, which is the shape
/// `conformance.py`'s own docstring claims for its version.
pub fn run(input: &VectorInput) -> serde_json::Value {
    let mut result = analyze_swing_bundle(&bundle_request(input));
    result.feedback = Some(build_feedback(&result.swing));
    serialize(&result)
}

/// One vector's input as the engine's argument struct — thirteen fields, borrowed.
///
/// Split out of [`run`] so it can be *checked*: a `None` substituted for the intent, the shot or
/// either strike list passes all six synthetic vectors, because all six carry none of them, and
/// nothing about the serialized result would look wrong. `every_input_field_reaches_the_engine`
/// compares the two structs field by field instead, which is the only form of this check that stays
/// true when a fourteenth field lands.
fn bundle_request<'a>(input: &'a VectorInput) -> BundleRequest<'a> {
    BundleRequest {
        swing_id: &input.swing_id,
        session_id: &input.session_id,
        face_on: &input.face_on,
        down_the_line: input.down_the_line.as_ref(),
        shot: input.shot.as_ref(),
        intent: input.intent.as_ref(),
        face_on_window: input.face_on_window,
        down_the_line_window: input.down_the_line_window,
        face_on_strikes: input.face_on_strikes.as_deref(),
        down_the_line_strikes: input.down_the_line_strikes.as_deref(),
        handedness: input.handedness,
        loft_deg: input.loft_deg,
    }
}

/// The shell's serialization, and the one a port is asked to match. `_serialize`.
///
/// Pydantic's `exclude={"swing": {"keypoints", "detections"}}` has no serde equivalent that can be
/// switched on per call — `skip_serializing_if` is a property of the *type*, and this shape is also
/// written whole by `storage/`, which keeps both fields. So the drop happens on the `Value`, after
/// the fact, which is what `model_dump_json(exclude=…)` is doing anyway.
fn serialize(result: &contracts::swing::SwingBundleResult) -> serde_json::Value {
    let mut value = serde_json::to_value(result).expect("a SwingBundleResult serializes");
    let swing = value
        .get_mut("swing")
        .and_then(serde_json::Value::as_object_mut)
        .expect("a serialized bundle carries a swing object");
    for field in EXCLUDED_FROM_SWING {
        swing
            .remove(*field)
            .expect("the excluded field was already absent, so the drop is not what it says");
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::keypoints::{FrameKeypoints, Landmark, NUM_POSE_LANDMARKS};

    fn a_clip(n: i64) -> KeypointsFile {
        KeypointsFile {
            clip: None,
            frames: (0..n)
                .map(|i| FrameKeypoints {
                    frame_index: i,
                    timestamp_ms: i as f64 * 10.0,
                    camera_id: None,
                    landmarks: vec![
                        Landmark {
                            x: 0.5,
                            y: 0.5,
                            z: 0.0,
                            visibility: 1.0,
                        };
                        NUM_POSE_LANDMARKS
                    ],
                })
                .collect(),
            pose_estimator: None,
        }
    }

    fn an_input(face_on: KeypointsFile) -> VectorInput {
        VectorInput {
            swing_id: "1".to_string(),
            session_id: "s".to_string(),
            face_on,
            down_the_line: None,
            shot: None,
            intent: None,
            face_on_window: None,
            down_the_line_window: None,
            face_on_strikes: None,
            down_the_line_strikes: None,
            handedness: None,
            loft_deg: None,
        }
    }

    /// The two heavy streams are gone and nothing else is — in particular `shot` survives, which
    /// sits on the same object and is *not* in the exclusion set.
    #[test]
    fn the_serialized_swing_drops_the_input_streams_and_keeps_everything_else() {
        let value = run(&an_input(a_clip(40)));
        let swing = value["swing"].as_object().expect("a swing object");
        assert!(!swing.contains_key("keypoints"));
        assert!(!swing.contains_key("detections"));
        for kept in [
            "swing_id",
            "session_id",
            "phases",
            "checkpoint_scores",
            "measurements",
            "unscored",
            "intent",
            "mechanics_score",
            "outcome_score",
            "overall_score",
            "shot",
        ] {
            assert!(swing.contains_key(kept), "{kept} went missing");
        }
    }

    /// The bundle's own keys, including the two a `None` must reach the wire as `null` for —
    /// pydantic emits them and `docs/CONFORMANCE.md` §3 compares the key set exactly.
    #[test]
    fn an_absent_alignment_serializes_as_null_rather_than_going_missing() {
        let value = run(&an_input(a_clip(40)));
        assert!(value.get("alignment").is_some());
        assert!(value["alignment"].is_null());
        assert!(value["down_the_line_window"].is_null());
        assert!(value["face_on_window"].is_null());
        assert_eq!(
            value["analysis_version"],
            serde_json::json!(contracts::swing::ANALYSIS_VERSION)
        );
    }

    /// **The second call is what this crate exists for.** A bare engine call leaves `feedback` null,
    /// which is the vector M19 P1 first recorded and corrected — a spec telling a port to ship a
    /// results page with no coaching on it.
    #[test]
    fn the_result_carries_feedback_and_not_a_null() {
        let value = run(&an_input(a_clip(40)));
        assert!(!value["feedback"].is_null(), "the second call did not run");
        assert!(value["feedback"]["tips"].is_array());
        // And the sidecar's half is still absent, as it is on all 21 vectors.
        assert!(value["feedback"]["coaching"].is_null());
        assert!(value["feedback"]["coaching_text"].is_null());
    }

    /// **Every field of `VectorInput` reaches `analyze_swing_bundle`.**
    ///
    /// Thirteen pass-throughs and nothing in this file would notice one going missing — a `None`
    /// substituted for the intent, the shot or the strike list passes all six synthetic vectors,
    /// because all six carry none of the three. (The corpus fifteen carry all three, which is why
    /// those pass-throughs are gated at P7 and P8b rather than never.) So the check here is
    /// structural: build a request the way [`run`] does and compare it field by field against the
    /// input it came from.
    ///
    /// Written as a `match`-free field sweep rather than a set of `run` calls because that is what
    /// makes it total — a field added to `VectorInput` and forgotten in [`run`] fails here.
    #[test]
    fn every_input_field_reaches_the_engine() {
        let face_on = a_clip(40);
        let shot: ShotData = serde_json::from_str(
            r#"{"shot_id": "7", "session_id": "s", "timestamp": "2026-09-25T00:00:00Z",
                "source": "screen", "ball_speed": 130.0}"#,
        )
        .expect("a valid ShotData");
        let intent = PracticeGoal {
            club: contracts::intent::ClubCategory::MidIron,
            ..PracticeGoal::default()
        };
        let input = VectorInput {
            swing_id: "swing-7".to_string(),
            session_id: "session-3".to_string(),
            face_on: face_on.clone(),
            down_the_line: None,
            shot: Some(shot.clone()),
            intent: Some(intent.clone()),
            face_on_window: Some((5, 35)),
            down_the_line_window: Some((1, 2)),
            face_on_strikes: Some(vec![20, 22]),
            down_the_line_strikes: Some(vec![21]),
            handedness: Some(Handedness::Left),
            loft_deg: Some(30.5),
        };

        let request = bundle_request(&input);
        assert_eq!(request.swing_id, "swing-7");
        assert_eq!(request.session_id, "session-3");
        assert_eq!(request.face_on, &face_on);
        assert_eq!(request.down_the_line, None);
        assert_eq!(request.shot, Some(&shot));
        assert_eq!(request.intent, Some(&intent));
        assert_eq!(request.face_on_window, Some((5, 35)));
        assert_eq!(request.down_the_line_window, Some((1, 2)));
        assert_eq!(request.face_on_strikes, Some(&[20, 22][..]));
        assert_eq!(request.down_the_line_strikes, Some(&[21][..]));
        assert_eq!(request.handedness, Some(Handedness::Left));
        assert_eq!(request.loft_deg, Some(30.5));
    }

    /// And the three the six vectors carry none of, checked through `run` rather than through the
    /// request — the shot lands on the result, and the intent is what the swing was judged against.
    #[test]
    fn the_shot_and_the_intent_reach_the_serialized_result() {
        let mut input = an_input(a_clip(40));
        input.shot = Some(
            serde_json::from_str(
                r#"{"shot_id": "7", "session_id": "s", "timestamp": "2026-09-25T00:00:00Z",
                    "source": "screen", "ball_speed": 130.0}"#,
            )
            .expect("a valid ShotData"),
        );
        input.intent = Some(PracticeGoal {
            club: contracts::intent::ClubCategory::MidIron,
            ..PracticeGoal::default()
        });
        let value = run(&input);
        assert_eq!(value["swing"]["shot"]["shot_id"], serde_json::json!("7"));
        assert_eq!(
            value["swing"]["intent"]["club"],
            serde_json::json!("mid_iron")
        );
    }

    /// The window shape JSON has no type for: in as a two-element array, out as one.
    #[test]
    fn a_window_survives_the_crossing_as_a_two_element_array() {
        let parsed: VectorInput = serde_json::from_value(serde_json::json!({
            "swing_id": "1",
            "session_id": "s",
            "face_on": {"frames": []},
            "face_on_window": [40, 96],
        }))
        .expect("the input parses");
        assert_eq!(parsed.face_on_window, Some((40, 96)));

        let mut input = an_input(a_clip(120));
        input.face_on_window = Some((40, 96));
        assert_eq!(run(&input)["face_on_window"], serde_json::json!([40, 96]));
    }

    /// An absent key and an explicit `null` are the same `None` here, which is the thing P2's
    /// harness could *not* do and this one must.
    #[test]
    fn an_explicit_null_reads_as_an_absent_key() {
        let absent: VectorInput = serde_json::from_value(serde_json::json!({
            "swing_id": "1", "session_id": "s", "face_on": {"frames": []},
        }))
        .expect("parses");
        let nulled: VectorInput = serde_json::from_value(serde_json::json!({
            "swing_id": "1", "session_id": "s", "face_on": {"frames": []},
            "shot": null, "loft_deg": null, "handedness": null, "face_on_window": null,
        }))
        .expect("parses");
        assert!(absent.loft_deg.is_none() && nulled.loft_deg.is_none());
        assert!(absent.shot.is_none() && nulled.shot.is_none());
        assert!(absent.handedness.is_none() && nulled.handedness.is_none());
    }
}
