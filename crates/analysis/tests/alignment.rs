//! The second view against `spec/vectors/stages/`'s `alignment` and its two `_dtl` groups. [M22 P7]
//!
//! The sixth of the seven stages to acquire a runner, and the first whose answers live almost
//! entirely in the **corpus** half: the six synthetic vectors carry one camera, so they record four
//! nulls and nothing else, and every non-trivial answer here comes off the fifteen real swings.
//!
//! # What it does
//!
//! Two gates in one file, because they read the same two things — the rear clip and the anchors
//! built from it — and splitting them would mean segmenting every corpus clip twice.
//!
//! The orchestration is `conformance.py::run_stages`', because that is what recorded the answers:
//!
//! ```text
//! frames    = windowed(face_on.frames, face_on_window)          # engine.py::_windowed
//! phases    = segment_phases(smooth_keypoints(frames))          # P4
//! face_from = anchors_from_phases(phases, face_on.clip, camera_id(frames))
//! face      = anchored_on_strike(face_from, face_on_strikes)
//! dtl_from  = anchors_from_keypoints(dtl.frames, dtl.clip, dtl_window, TRAIL_WRIST)
//! dtl       = anchored_on_strike(dtl_from, down_the_line_strikes)
//! warp      = align_swings(face, dtl)                           # when both exist
//!
//! dtl_frames = smooth_keypoints(dtl.frames)                     # once, for both readers
//! dtl_events = (motion_start, top, impact) as floats
//! placements_dtl = dtl_placements(dtl_frames, dtl_events, handedness)
//! pivot_dtl      = pivot_measurements(dtl_frames, dtl_events, DOWN_THE_LINE)
//! ```
//!
//! **The unpinned pair is recorded alongside the pinned one on purpose.** `alignment.py` holds ten
//! of ADR-032 §3's seventeen frame-index roundings, and a port that got the warp wrong needs to know
//! *which half* — the anchors, or the strike that moved one of them. `face_anchors_from_phases` and
//! `dtl_anchors_from_keypoints` are that split, and they are compared here in full.
//!
//! # The rules
//!
//! `docs/CONFORMANCE.md` §3's: floats within `RTOL = 1e-9`, and **strings, bools and ints exactly**.
//! Every `note` in a `SwingAlignment` is a string, and four of them interpolate a `.2f` or a `.3f`
//! — so `pyfmt::fixed` lands in this file as bytes rather than as a tolerance.
//!
//! # What this file cannot check, which is most of the module
//!
//! Measured rather than assumed, and each figure has a test below.
//! [`alignment`](analysis::alignment)'s module doc carries the same list with the reasons.
//!
//! - **Fifteen of fifteen report `synchronized`.** So three of the five `AlignmentQuality` tiers
//!   never reach a committed answer, and returning the constant `Synchronized` from `align_swings`
//!   passes this gate.
//! - **`warp_top` and `top_late_by` are `None` on all thirty clips**, because the widest downswing
//!   disagreement the corpus produces is 27.6% against the 30% threshold. `shared_tops` and
//!   `arbitrate_tops` therefore return nothing on every vector, and `engine`'s
//!   `without_contradicted_scores` is unreachable through this gate in either language.
//! - **Four of the fifteen reach the down-the-line trajectory basis and eleven refuse it** — the
//!   mirror image of P5b's face-on finding, where all fifteen refused.
//! - **All fifteen are right-handed**, so the down-the-line mirror is ungated here. P5b's
//!   counterpart *was* gated, by `synthetic/face-on-only`; there is no left-handed two-camera vector.
//! - **No vector passes a `down_the_line_window`**, so `anchors_from_keypoints`' window branch has
//!   no committed answer.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use analysis::alignment::{align_swings, anchors_from_keypoints, anchors_from_phases};
use analysis::engine::{dtl_placements, pivot_measurements};
use analysis::phases::{segment_phases, LEAD_WRIST, TRAIL_WRIST};
use analysis::smoothing::{smooth_keypoints, DEFAULT_WINDOW};
use contracts::alignment::{AlignmentQuality, ClipAlignment, SwingAlignment, SwingAnchors};
use contracts::golfer::Handedness;
use contracts::keypoints::{FrameKeypoints, KeypointsFile};
use contracts::placements::DOWN_THE_LINE;
use contracts::swing::{Measurement, ANALYSIS_VERSION};
use flate2::read::GzDecoder;
use serde::Deserialize;

/// `docs/CONFORMANCE.md` §3's float rule, and the same constants the three sibling gates use.
const RTOL: f64 = 1e-9;
const ATOL: f64 = 1e-12;

/// The two `measurements` groups this phase owns, in the order `conformance.MEASUREMENT_GROUPS`
/// lists them. They are the *last* two of the seven, because `analyze_swing_bundle` appends them to
/// a list `analyze_swing` had already finished.
const OWNED: [&str; 2] = ["placements_dtl", "pivot_dtl"];

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
    alignment: RecordedAlignment,
    measurements: Vec<Group>,
}

/// The stage's five keys. `Option<SwingAnchors>` rather than `Option<Option<_>>`: unlike P2's
/// harness input these five are always *present*, and the four anchor slots are `null` on a
/// single-camera vector rather than absent.
#[derive(Deserialize)]
struct RecordedAlignment {
    face_anchors_from_phases: Option<SwingAnchors>,
    face_anchors: Option<SwingAnchors>,
    dtl_anchors_from_keypoints: Option<SwingAnchors>,
    dtl_anchors: Option<SwingAnchors>,
    clip_alignment: Option<RecordedWarp>,
}

/// `SwingAlignment` with its serialized computed field kept, which `SwingAlignment`'s own
/// `Deserialize` drops. The sentence is in the payload a port has to match, so it is compared here
/// rather than trusted to follow from `quality`.
#[derive(Deserialize)]
struct RecordedWarp {
    #[serde(flatten)]
    warp: SwingAlignment,
    quality_summary: String,
}

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
    down_the_line: Option<KeypointsFile>,
    #[serde(default)]
    face_on_window: Option<[i64; 2]>,
    #[serde(default)]
    down_the_line_window: Option<[i64; 2]>,
    #[serde(default)]
    face_on_strikes: Option<Vec<i64>>,
    #[serde(default)]
    down_the_line_strikes: Option<Vec<i64>>,
    #[serde(default)]
    handedness: Option<Handedness>,
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
/// names a stable first offender. The same resolution the three sibling gates do, duplicated rather
/// than shared for the reason they give: two integration tests in one crate are separate binaries,
/// and a fifth file whose only job is to be imported is more structure than forty lines of reading
/// earns.
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

/// `engine.py::_windowed`, as in the three sibling gates: four lines here rather than a dependency
/// on a private function.
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

/// `engine.py::_camera_id` — the first frame that recorded one.
///
/// `run_stages` reads it off the **windowed** slice where `analyze_swing_bundle` reads it off the
/// whole clip. The two answer differently only on a clip whose first frames record no `camera_id`
/// and whose later ones do, which no vector is; this file follows `run_stages`, because that is what
/// produced the committed answer.
fn camera_id(keypoints: &[FrameKeypoints]) -> Option<&str> {
    keypoints
        .iter()
        .find_map(|frame| frame.camera_id.as_deref())
}

fn float_differs(got: f64, want: f64) -> bool {
    !matches!(
        (got - want).abs().partial_cmp(&(ATOL + RTOL * want.abs())),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    )
}

/// One `Option<f64>` under §3's null rule: a refusal compares equal to nothing but a refusal, and is
/// reported as a type difference rather than tested numerically.
fn compare_opt_float(at: &str, got: Option<f64>, want: Option<f64>, out: &mut Vec<String>) {
    match (got, want) {
        (Some(got), Some(want)) if float_differs(got, want) => {
            out.push(format!("{at} {got} != {want}"));
        }
        (Some(_), Some(_)) | (None, None) => {}
        _ => out.push(format!("{at} {got:?} != {want:?}")),
    }
}

/// Every field of a `SwingAnchors`: the five ints and bools exactly, the `camera_id` exactly, the
/// `fps` within tolerance.
fn compare_anchors(at: &str, got: &SwingAnchors, want: &SwingAnchors, out: &mut Vec<String>) {
    for (field, got, want) in [
        ("motion_start", got.motion_start, want.motion_start),
        ("top", got.top, want.top),
        ("impact", got.impact, want.impact),
    ] {
        if got != want {
            out.push(format!("{at}.{field} {got} != {want}"));
        }
    }
    for (field, got, want) in [
        (
            "motion_start_detected",
            got.motion_start_detected,
            want.motion_start_detected,
        ),
        ("impact_measured", got.impact_measured, want.impact_measured),
    ] {
        if got != want {
            out.push(format!("{at}.{field} {got} != {want}"));
        }
    }
    if got.camera_id != want.camera_id {
        out.push(format!(
            "{at}.camera_id {:?} != {:?}",
            got.camera_id, want.camera_id
        ));
    }
    if got.frame_count != want.frame_count {
        out.push(format!(
            "{at}.frame_count {:?} != {:?}",
            got.frame_count, want.frame_count
        ));
    }
    compare_opt_float(&format!("{at}.fps"), got.fps, want.fps, out);
}

fn compare_opt_anchors(
    at: &str,
    got: Option<&SwingAnchors>,
    want: Option<&SwingAnchors>,
    out: &mut Vec<String>,
) {
    match (got, want) {
        (Some(got), Some(want)) => compare_anchors(at, got, want, out),
        (None, None) => {}
        _ => out.push(format!(
            "{at} {} != {}",
            if got.is_some() { "anchors" } else { "null" },
            if want.is_some() { "anchors" } else { "null" },
        )),
    }
}

fn compare_clip(at: &str, got: &ClipAlignment, want: &ClipAlignment, out: &mut Vec<String>) {
    compare_anchors(&format!("{at}.anchors"), &got.anchors, &want.anchors, out);
    if got.warp_motion_start != want.warp_motion_start {
        out.push(format!(
            "{at}.warp_motion_start {} != {}",
            got.warp_motion_start, want.warp_motion_start
        ));
    }
    for (field, got, want) in [
        ("warp_top", got.warp_top, want.warp_top),
        ("top_late_by", got.top_late_by, want.top_late_by),
    ] {
        if got != want {
            out.push(format!("{at}.{field} {got:?} != {want:?}"));
        }
    }
    compare_opt_float(
        &format!("{at}.tau_start"),
        Some(got.tau_start),
        Some(want.tau_start),
        out,
    );
    compare_opt_float(
        &format!("{at}.tau_end"),
        Some(got.tau_end),
        Some(want.tau_end),
        out,
    );
}

/// The whole `SwingAlignment`, including the serialized `quality_summary` and every note byte for
/// byte.
fn compare_warp(id: &str, got: &SwingAlignment, want: &RecordedWarp, out: &mut Vec<String>) {
    for (side, got, want) in [
        ("a", got.a.as_ref(), want.warp.a.as_ref()),
        ("b", got.b.as_ref(), want.warp.b.as_ref()),
    ] {
        match (got, want) {
            (Some(got), Some(want)) => {
                compare_clip(&format!("{id}: clip_alignment.{side}"), got, want, out);
            }
            (None, None) => {}
            _ => out.push(format!("{id}: clip_alignment.{side} presence differs")),
        }
    }
    if got.quality != want.warp.quality {
        out.push(format!(
            "{id}: clip_alignment.quality {:?} != {:?}",
            got.quality, want.warp.quality
        ));
    }
    if got.quality.summary() != want.quality_summary {
        out.push(format!(
            "{id}: clip_alignment.quality_summary {:?} != {:?}",
            got.quality.summary(),
            want.quality_summary
        ));
    }
    if got.notes.len() != want.warp.notes.len() {
        out.push(format!(
            "{id}: clip_alignment.notes {} != {} ({:?} against {:?})",
            got.notes.len(),
            want.warp.notes.len(),
            got.notes,
            want.warp.notes,
        ));
    } else {
        for (at, (got, want)) in got.notes.iter().zip(&want.warp.notes).enumerate() {
            if got != want {
                out.push(format!(
                    "{id}: clip_alignment.notes[{at}]\n  got  {got:?}\n  want {want:?}"
                ));
            }
        }
    }
    match (got.overlap, want.warp.overlap) {
        (Some((got_low, got_high)), Some((want_low, want_high))) => {
            compare_opt_float(
                &format!("{id}: clip_alignment.overlap.0"),
                Some(got_low),
                Some(want_low),
                out,
            );
            compare_opt_float(
                &format!("{id}: clip_alignment.overlap.1"),
                Some(got_high),
                Some(want_high),
                out,
            );
        }
        (None, None) => {}
        _ => out.push(format!("{id}: clip_alignment.overlap presence differs")),
    }
}

/// Every field of a `Measurement` under §3's rules, as `tests/measurements.rs` does it: the float
/// within tolerance, the four strings exactly. `detail` is where this earns its keep — the two
/// `placements_dtl` rows interpolate a `%g` percentile and a dict key chosen by a stable sort.
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

/// What the port produces for one vector's second view: the four anchor slots, the warp, and the two
/// `_dtl` measurement groups.
struct Ran {
    face_from_phases: Option<SwingAnchors>,
    face: Option<SwingAnchors>,
    dtl_from_keypoints: Option<SwingAnchors>,
    dtl: Option<SwingAnchors>,
    warp: Option<SwingAlignment>,
    groups: [Vec<Measurement>; 2],
}

/// `run_stages`' orchestration for this stage, and nothing else — no comparison, so the three tests
/// below read the same run rather than three slightly different ones.
///
/// **`anchored_on_strike` is not called**, because it is private to `engine` and its whole
/// contribution here is `with_measured_impact` plus a note this stage does not record.
/// `alignment::with_measured_impact` is what the recorded pinning is, and P6 already gated the
/// pass-through; calling it directly is also what makes the unpinned/pinned split above readable.
fn run(input: &EngineInput) -> Ran {
    use analysis::alignment::with_measured_impact;

    let frames = windowed(&input.face_on.frames, input.face_on_window);
    let smoothed = smooth_keypoints(frames, DEFAULT_WINDOW);
    let phases = segment_phases(&smoothed, LEAD_WRIST);
    let face_from_phases =
        anchors_from_phases(&phases, input.face_on.clip.as_ref(), camera_id(frames), 0);
    let face = face_from_phases
        .as_ref()
        .map(|anchors| with_measured_impact(anchors, input.face_on_strikes.as_deref()));

    let dtl_from_keypoints = input.down_the_line.as_ref().and_then(|clip| {
        anchors_from_keypoints(
            &clip.frames,
            clip.clip.as_ref(),
            input.down_the_line_window.map(|[lo, hi]| (lo, hi)),
            Some(TRAIL_WRIST),
        )
    });
    let dtl = dtl_from_keypoints
        .as_ref()
        .map(|anchors| with_measured_impact(anchors, input.down_the_line_strikes.as_deref()));

    let warp = match (face.as_ref(), dtl.as_ref()) {
        (Some(face), Some(dtl)) => Some(align_swings(face, dtl)),
        _ => None,
    };

    let mut groups = [Vec::new(), Vec::new()];
    if let (Some(clip), Some(anchors)) = (input.down_the_line.as_ref(), dtl.as_ref()) {
        let dtl_frames = smooth_keypoints(&clip.frames, DEFAULT_WINDOW);
        let events = (
            anchors.motion_start as f64,
            anchors.top as f64,
            anchors.impact as f64,
        );
        groups[0] = dtl_placements(&dtl_frames, events, input.handedness);
        groups[1] = pivot_measurements(&dtl_frames, events, DOWN_THE_LINE);
    }

    Ran {
        face_from_phases,
        face,
        dtl_from_keypoints,
        dtl,
        warp,
        groups,
    }
}

/// The `alignment` stage, on every committed vector, in one report.
#[test]
fn the_alignment_conforms_on_all_twenty_one_vectors() {
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

        let ran = run(&engine.input);
        let want = &stage.stages.alignment;
        for (at, got, want) in [
            (
                "face_anchors_from_phases",
                ran.face_from_phases.as_ref(),
                want.face_anchors_from_phases.as_ref(),
            ),
            (
                "face_anchors",
                ran.face.as_ref(),
                want.face_anchors.as_ref(),
            ),
            (
                "dtl_anchors_from_keypoints",
                ran.dtl_from_keypoints.as_ref(),
                want.dtl_anchors_from_keypoints.as_ref(),
            ),
            ("dtl_anchors", ran.dtl.as_ref(), want.dtl_anchors.as_ref()),
        ] {
            compare_opt_anchors(&format!("{id}: {at}"), got, want, &mut differences);
        }

        match (ran.warp.as_ref(), want.clip_alignment.as_ref()) {
            (Some(got), Some(want)) => compare_warp(id, got, want, &mut differences),
            (None, None) => {}
            (got, _) => differences.push(format!(
                "{id}: clip_alignment {} != {}",
                if got.is_some() { "a warp" } else { "null" },
                if want.clip_alignment.is_some() {
                    "a warp"
                } else {
                    "null"
                },
            )),
        }
    }

    assert!(
        differences.is_empty(),
        "{} difference(s) across {} vectors:\n{}",
        differences.len(),
        vectors.len(),
        differences.join("\n")
    );
    println!("alignment: {} vectors conform", vectors.len());
}

/// The `measurements` stage's last two groups, which are the rear clip's own rows.
///
/// They are checked here rather than in `tests/measurements.rs` because they hang off `dtl_anchors`,
/// which is this stage's product: a file that owned the groups without the anchors would have to
/// build them, and that is a second copy of `analyze_swing_bundle`'s orchestration.
#[test]
fn the_dtl_measurement_groups_conform_on_all_twenty_one_vectors() {
    let mut differences: Vec<String> = Vec::new();
    let vectors = stage_vectors();

    for (stage, engine) in &vectors {
        let id = &stage.id;
        let ran = run(&engine.input);
        for (name, got) in OWNED.iter().zip(&ran.groups) {
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
                    got.iter().map(|row| &row.name).collect::<Vec<_>>(),
                    want.rows.iter().map(|row| &row.name).collect::<Vec<_>>(),
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
}

/// **Every corpus pair reports `synchronized`, so three of the five tiers are ungated.**
///
/// `synchronized` overwrites whatever the anchor count came to, and every corpus clip pair heard the
/// strike — so `full`, `top_impact` and `impact_only` never reach a committed answer. What *is*
/// gated is the soft-anchor decision underneath: it reaches the payload through
/// `warp_motion_start` and through the notes, and this test checks both halves are actually
/// exercised so the tier's invisibility is not mistaken for the decision's.
///
/// Meant to fail the day a vector reports another tier, which is the good failure: the answer then
/// is to note that the ladder is covered, not to relax it.
#[test]
fn every_two_camera_vector_is_synchronized_and_the_tiers_below_are_not_gated() {
    let mut aligned = 0;
    let mut soft_accepted = 0;
    let mut soft_refused = 0;

    for (stage, _) in stage_vectors() {
        let Some(warp) = stage.stages.alignment.clip_alignment else {
            continue;
        };
        aligned += 1;
        assert_eq!(
            warp.warp.quality,
            AlignmentQuality::Synchronized,
            "{}: a second tier reached a committed answer",
            stage.id
        );
        let clip = warp
            .warp
            .a
            .as_ref()
            .expect("an aligned pair has both clips");
        // The soft anchor was accepted exactly when tau=0 is still the detected `motion_start`.
        if clip.warp_motion_start == clip.anchors.motion_start {
            soft_accepted += 1;
            assert!(
                warp.warp.notes.is_empty(),
                "{}: an accepted soft anchor explains nothing",
                stage.id
            );
        } else {
            soft_refused += 1;
            assert!(
                !warp.warp.notes.is_empty(),
                "{}: a refused soft anchor says why",
                stage.id
            );
        }
        // And the tops always stand, which is the finding below this one.
        assert_eq!(warp.warp.a.as_ref().and_then(|c| c.warp_top), None);
        assert_eq!(warp.warp.b.as_ref().and_then(|c| c.warp_top), None);
        assert_eq!(warp.warp.a.as_ref().and_then(|c| c.top_late_by), None);
        assert_eq!(warp.warp.b.as_ref().and_then(|c| c.top_late_by), None);
    }

    assert_eq!(aligned, 15, "the six synthetic vectors carry one camera");
    assert_eq!(
        (soft_accepted, soft_refused),
        (7, 8),
        "both sides of the soft-anchor decision are gated, and by how many"
    );
}

/// **The corpus no longer contains a pair `arbitrate_tops` can decide**, which is the P7 finding
/// worth carrying forward.
///
/// M11 P7 built the arbiter for five bundles whose face-on view measured 0.183-0.267 s of downswing
/// where down-the-line measured 0.367-0.484 s of the same swing. The corpus as recorded does not
/// hold that: the widest *relative* disagreement is 27.6% against the 30% threshold, and pinning
/// tau=2 to the heard strike is part of why — it moves the widest unpinned gap of 21.7% to 27.6%,
/// still short. So `shared_tops`, `arbitrate_tops`, `top_at`, `tempo_restated`, the `impact_only`
/// tier and `engine::without_contradicted_scores` are all unreachable from `spec/vectors/`, in
/// either language, and their unit tests are the whole of their coverage.
///
/// This measures the margin rather than asserting the outcome, so it fails *with a number* the day
/// a vector gets close — which is the useful failure, because the answer then is that the ladder
/// has become gated.
#[test]
fn no_committed_pair_disagrees_enough_about_the_downswing_to_be_arbitrated() {
    fn relative_gap(x: f64, y: f64) -> f64 {
        let larger = x.abs().max(y.abs());
        if larger > 0.0 {
            (x - y).abs() / larger
        } else {
            0.0
        }
    }
    fn downswing_seconds(anchors: &SwingAnchors) -> f64 {
        (anchors.impact - anchors.top) as f64 / anchors.fps.expect("every corpus clip carries fps")
    }

    let mut widest_pinned: f64 = 0.0;
    let mut widest_unpinned: f64 = 0.0;
    for (stage, _) in stage_vectors() {
        let recorded = &stage.stages.alignment;
        let (Some(face), Some(dtl)) = (&recorded.face_anchors, &recorded.dtl_anchors) else {
            continue;
        };
        widest_pinned = widest_pinned.max(relative_gap(
            downswing_seconds(face),
            downswing_seconds(dtl),
        ));
        let unpinned = (
            recorded.face_anchors_from_phases.as_ref(),
            recorded.dtl_anchors_from_keypoints.as_ref(),
        );
        if let (Some(face), Some(dtl)) = unpinned {
            widest_unpinned = widest_unpinned.max(relative_gap(
                downswing_seconds(face),
                downswing_seconds(dtl),
            ));
        }
        assert!(
            face.impact_measured && dtl.impact_measured,
            "{}: the arbiter's entry condition is a shared clock, and every corpus pair has one",
            stage.id
        );
    }

    assert!(
        widest_pinned < 0.30,
        "a pair now reaches the arbiter at a gap of {widest_pinned:.3} — the ladder is gated, so \
         say so rather than relaxing this"
    );
    assert!((widest_pinned - 0.276).abs() < 0.001, "{widest_pinned:.4}");
    assert!(
        (widest_unpinned - 0.217).abs() < 0.001,
        "{widest_unpinned:.4}"
    );
}

/// **Four of the fifteen reach the down-the-line trajectory basis; eleven refuse it.**
///
/// The mirror image of P5b's finding rather than a repeat of it. Face-on, `build_trajectory` refuses
/// every one of the fifteen because the *trail* arm crosses the torso; from behind it is the *lead*
/// arm that hides, and the down-the-line fit drops it. So the same footage that cannot be placed
/// face-on can sometimes be placed from the rear — and the four that manage it are the only
/// committed evidence for that artifact's arithmetic, its `%g` percentile and its interval name.
///
/// The five `pivot_dtl` rows are a different story and are checked here too: all fifteen produce the
/// full five, because `pivot_observations` needs three instants and a shoulder width rather than a
/// whole resampled timeline per landmark.
#[test]
fn four_corpus_vectors_reach_the_dtl_basis_and_every_one_reaches_the_pivots() {
    let mut with_trajectory: Vec<String> = Vec::new();
    let mut without: Vec<String> = Vec::new();
    let mut with_pivots = 0;

    for (stage, _) in stage_vectors() {
        let rows = |name: &str| -> usize {
            stage
                .stages
                .measurements
                .iter()
                .filter(|group| group.group == name)
                .map(|group| group.rows.len())
                .sum()
        };
        if stage.stages.alignment.dtl_anchors.is_none() {
            assert_eq!(rows("placements_dtl"), 0, "{}", stage.id);
            assert_eq!(rows("pivot_dtl"), 0, "{}", stage.id);
            continue;
        }
        match rows("placements_dtl") {
            0 => without.push(stage.id.clone()),
            2 => with_trajectory.push(stage.id.clone()),
            other => panic!("{}: T² and Q arrive together, not {other}", stage.id),
        }
        assert_eq!(
            rows("pivot_dtl"),
            5,
            "{}: the rotation numbers need three instants, not a clean timeline",
            stage.id
        );
        with_pivots += 1;
    }

    assert_eq!(with_pivots, 15);
    assert_eq!(
        with_trajectory.len(),
        4,
        "the rear basis is reached by these and no others: {with_trajectory:?}"
    );
    assert_eq!(without.len(), 11, "{without:?}");
}

/// **No two-camera vector is left-handed, so the rear mirror is ungated here.**
///
/// P5b found the face-on mirror *was* gated, by `synthetic/face-on-only` — left-handed and one of
/// the six that reach the basis. There is no left-handed two-camera vector, so `build_trajectory`'s
/// left/right swap and `x` negation run against the down-the-line basis on no committed swing at
/// all. `crate::trajectory`'s own mirror tests are what stands there.
///
/// It is also the narrowest hole of the four this file records, because the mirror is one shared
/// implementation: what is untested is the swap *against this basis's landmark list*, which differs
/// from the face-on one — it drops the lead arm and adds both ankles.
#[test]
fn no_two_camera_vector_is_left_handed() {
    for (stage, engine) in stage_vectors() {
        if stage.stages.alignment.dtl_anchors.is_none() {
            continue;
        }
        assert_eq!(
            engine.input.handedness,
            Some(Handedness::Right),
            "{}: a left-handed two-camera vector would gate the rear mirror — say so",
            stage.id
        );
    }
}

/// **No vector carries a `down_the_line_window`**, so `anchors_from_keypoints`' window branch has no
/// committed answer.
///
/// The one committed window anywhere is `synthetic/windowed.json`'s face-on `[40, 96]`, and that
/// goes through `engine::windowed` rather than through this function — which clamps differently and
/// refuses an empty window where the engine ignores one. Both differences are pinned by unit tests
/// in `analysis::alignment`.
#[test]
fn the_rear_window_branch_has_no_committed_answer() {
    for (_, engine) in stage_vectors() {
        assert_eq!(engine.input.down_the_line_window, None);
    }
}
