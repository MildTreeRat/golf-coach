//! The selection family's runner: one case under `spec/vectors/selection/` in, an answer per call
//! out, in `expected.results`' shape. [M29 P19]
//!
//! **Twin: `scripts/conformance_vectors.py::_run_selection`**, the definition frozen Python recorded
//! the family with (M29 P18, the plan's decision 14), over `analysis/phases.py`'s
//! `select_swing`, `select_matching_swing` and `window_around` and `api/pipeline.py`'s `_pick_swing`
//! and `_auto_windows`. The family ages on `ANALYSIS_VERSION` (the M29 plan's call 1), recorded at
//! frozen Python's, so `golf-core rerecord` walks it under an engine declaration, after the shot
//! family (`rerecord::walk_selection`).
//!
//! # One definition, two callers
//!
//! `tests/selection.rs` is the family's gate and `golf-core rerecord` its re-record, and both answer
//! a case through [`run_selection`], as [`crate::profile_family`]'s runner serves its gate and the
//! re-record. Here rather than in `analysis` for [`crate::stages`]' reason: what a committed file
//! holds is vector plumbing, and the engine stays a library that knows nothing about `spec/`.
//!
//! # The case
//!
//! The recorder's section header in `scripts/conformance_vectors.py` is the encoding, in full. In
//! short: `input.clips` maps a name to `{fps, frames, tracks}`, `tracks` holding each read wrist's
//! `y` and `visibility`, one value per frame; `input.calls` are answered in order, one
//! `expected.results` entry each.
//!
//! - **`select_swing`, `select_matching_swing`**: the rule over the clip's frames *as given* (a
//!   `rules/` clip is stored already smoothed), `fps` an argument.
//! - **`window_around`**: `[start, end]`, and the one call that reaches the window's no-fps
//!   arithmetic.
//! - **`_pick_swing`**: [`auto_windows::pick_swing`], smoothing and then the rule, with the clip's
//!   own fps — matching when the call carries a reference, alone when it does not.
//! - **`_auto_windows`**: [`auto_windows::auto_windows`], the pipeline's ordering around its picks,
//!   answered `{windows, notes, log, picks}`. It is in this crate rather than in `crates/lab`, whose
//!   bundle assembly calls it, so that this runner and the re-record answer it too (M29 P25, and
//!   [`crate::auto_windows`]' module doc). Each answer's `picks` are the `_pick_swing` calls the case
//!   lists ahead of it, so the lab's picks are the gated composition and not a second copy.
//!
//! Since P25 every call is answered and [`NOT_YET`] is empty. It stays, as `career_family::CARRIED`
//! stayed once it was empty (M36 P16), because the gate and the re-record share it: a call recorded
//! before its runner lands is skipped by name in the one and carried in the other.
//!
//! **A clip is rebuilt as `_selection_file` rebuilds it**: `frames` frames of 33 landmarks, all
//! zeros (visibility too, `Landmark`'s Python default) but the tracked wrists' `y` and
//! `visibility`, `frame_index` the position and `timestamp_ms` 0. The recorder proved, per case,
//! that selection reads nothing else, by answering every call over the whole clip as well and
//! refusing a case whose answers differed.

use std::collections::BTreeMap;

use analysis::phases::{
    select_matching_swing, select_swing, window_around, Downswing, SwingChoice,
};
use contracts::keypoints::{FrameKeypoints, Landmark, PoseLandmark, NUM_POSE_LANDMARKS};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use storage::manifest::Role;

use crate::auto_windows::{auto_windows, pick_swing, View};

/// `provenance.kind` on every selection case, and the family's directory under `spec/vectors/`.
pub const KIND: &str = "selection";

/// The calls this crate does not answer yet, each with the M29 phase that will. Empty since P25,
/// which answered the last of them, `_auto_windows` (the module doc says why the table stays).
pub const NOT_YET: [(&str, &str); 0] = [];

/// The phase that answers `fn`, when [`run_selection`] does not.
pub fn not_yet(name: &str) -> Option<&'static str> {
    NOT_YET
        .iter()
        .find(|(call, _)| *call == name)
        .map(|&(_, phase)| phase)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    clips: BTreeMap<String, Clip>,
    calls: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Clip {
    fps: Option<f64>,
    frames: usize,
    tracks: BTreeMap<String, Track>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Track {
    y: Vec<f64>,
    visibility: Vec<f64>,
}

/// `select_swing`'s call.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    clip: String,
    fps: Option<f64>,
    wrist: String,
    strike_frames: Option<Vec<i64>>,
}

/// `select_matching_swing`'s call: [`Rule`] and a reference, which this rule always has.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Matching {
    clip: String,
    fps: Option<f64>,
    reference_downswing_s: f64,
    wrist: String,
    strike_frames: Option<Vec<i64>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Window {
    downswing: Descent,
    fps: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descent {
    top: usize,
    impact: usize,
    rise: f64,
}

/// `_pick_swing`'s call: no fps, because the pipeline reads the clip's.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pick {
    clip: String,
    wrist: String,
    reference_downswing_s: Option<f64>,
    strike_frames: Option<Vec<i64>>,
}

/// `_auto_windows`' call: both views by clip name, the windows given by hand, and each view's
/// strikes, `null` for a view nobody listened to.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Windows {
    views: PerView<Option<String>>,
    window_face_on: Option<(i64, i64)>,
    window_dtl: Option<(i64, i64)>,
    strikes: PerView<Option<Vec<i64>>>,
}

/// One value per view, keyed as the recorder keys them, by `Role`'s wire name.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PerView<T> {
    face_on: T,
    down_the_line: T,
}

/// A case's clip rebuilt: its frame rate, and its frames as `_selection_file` writes them.
struct Rebuilt {
    fps: Option<f64>,
    frames: Vec<FrameKeypoints>,
}

/// Every call of one case's `input` answered in order: `Some` with the value `expected.results`
/// holds at that index, or `None` for a call [`NOT_YET`] names. `Err` names what in the input does
/// not read: an unknown call, a clip no call can find, a landmark that is not a wrist, a track of
/// another length than its clip.
pub fn run_selection(input: &Value) -> Result<Vec<Option<Value>>, String> {
    let input = Input::deserialize(input).map_err(|e| format!("input: {e}"))?;
    let clips: BTreeMap<&str, Rebuilt> = input
        .clips
        .iter()
        .map(|(name, clip)| Ok((name.as_str(), rebuild(name, clip)?)))
        .collect::<Result<_, String>>()?;
    let clip = |name: &str| {
        clips
            .get(name)
            .ok_or_else(|| format!("no clip is named {name:?}"))
    };

    let answer = |call: &Value| -> Result<Option<Value>, String> {
        let name = call["fn"].as_str().unwrap_or_default();
        let answer = match name {
            "select_swing" => {
                let call: Rule = arguments(call)?;
                let frames = &clip(&call.clip)?.frames;
                let wrist = wrist_named(&call.wrist)?;
                choice(select_swing(
                    frames,
                    call.fps,
                    wrist,
                    call.strike_frames.as_deref(),
                ))
            }
            "select_matching_swing" => {
                let call: Matching = arguments(call)?;
                let frames = &clip(&call.clip)?.frames;
                let wrist = wrist_named(&call.wrist)?;
                choice(select_matching_swing(
                    frames,
                    call.fps,
                    call.reference_downswing_s,
                    wrist,
                    call.strike_frames.as_deref(),
                ))
            }
            "window_around" => {
                let call: Window = arguments(call)?;
                let descent = Downswing {
                    top: call.downswing.top,
                    impact: call.downswing.impact,
                    rise: call.downswing.rise,
                };
                let (start, end) = window_around(&descent, call.fps);
                json!([start, end])
            }
            "_pick_swing" => {
                let call: Pick = arguments(call)?;
                let clip = clip(&call.clip)?;
                let wrist = wrist_named(&call.wrist)?;
                choice(pick_swing(
                    &clip.frames,
                    clip.fps,
                    wrist,
                    call.reference_downswing_s,
                    call.strike_frames.as_deref(),
                ))
            }
            "_auto_windows" => {
                let call: Windows = arguments(call)?;
                windows_answer(&call, &clip)?
            }
            other if not_yet(other).is_some() => return Ok(None),
            other => return Err(format!("no selection call is named {other:?}")),
        };
        Ok(Some(answer))
    };
    input
        .calls
        .iter()
        .enumerate()
        .map(|(index, call)| answer(call).map_err(|e| format!("call {index}: {e}")))
        .collect()
}

/// One `_auto_windows` call answered as `_auto_windows_answer` writes it: the windows, the notes and
/// log lines appended, and each pick as `{clip, wrist, reference_downswing_s, strike_frames}`, the
/// clip named as the call names the view it was made on.
fn windows_answer<'c>(
    call: &Windows,
    clip: &impl Fn(&str) -> Result<&'c Rebuilt, String>,
) -> Result<Value, String> {
    fn view<'v>(rebuilt: &'v Rebuilt, strikes: &'v Option<Vec<i64>>) -> View<'v> {
        View {
            frames: &rebuilt.frames,
            fps: rebuilt.fps,
            strikes: strikes.as_deref(),
        }
    }
    let face_on_name = call
        .views
        .face_on
        .as_deref()
        .ok_or("_auto_windows: every bundle has its face-on view")?;
    let face_on = view(clip(face_on_name)?, &call.strikes.face_on);
    let down_the_line = match call.views.down_the_line.as_deref() {
        Some(name) => Some(view(clip(name)?, &call.strikes.down_the_line)),
        None => None,
    };

    let mut log = Vec::new();
    let mut notes = Vec::new();
    let answer = auto_windows(
        face_on,
        down_the_line,
        call.window_face_on,
        call.window_dtl,
        &mut |line| log.push(line.to_string()),
        &mut notes,
    );
    let picks: Vec<Value> = answer
        .picks
        .iter()
        .map(|pick| {
            let clip = match pick.role {
                Role::DownTheLine => call.views.down_the_line.as_deref(),
                _ => Some(face_on_name),
            };
            json!({
                "clip": clip,
                "wrist": wrist_name(pick.wrist),
                "reference_downswing_s": pick.reference_downswing_s,
                "strike_frames": pick.strike_frames,
            })
        })
        .collect();
    let pair = |window: Option<(i64, i64)>| window.map(|(start, end)| json!([start, end]));
    Ok(json!({
        "windows": {
            "face_on": pair(answer.face_on),
            "down_the_line": pair(answer.down_the_line),
        },
        "notes": notes,
        "log": log,
        "picks": picks,
    }))
}

/// A call's arguments, the `fn` key aside, each held to its shape (`deny_unknown_fields`), so an
/// argument the recorder adds is an error here rather than one this runner silently ignores.
fn arguments<T: DeserializeOwned>(call: &Value) -> Result<T, String> {
    let mut arguments = call.clone();
    if let Some(fields) = arguments.as_object_mut() {
        fields.remove("fn");
    }
    serde_json::from_value(arguments).map_err(|e| format!("{}: {e}", call["fn"]))
}

/// The two landmarks the family tracks, by their `PoseLandmark` names (the `#[repr(u8)]` 15 and
/// 16). Any other is refused: the recorder keeps only the wrist each call reads.
fn wrist_named(name: &str) -> Result<PoseLandmark, String> {
    match name {
        "LEFT_WRIST" => Ok(PoseLandmark::LeftWrist),
        "RIGHT_WRIST" => Ok(PoseLandmark::RightWrist),
        other => Err(format!(
            "{other:?} is not a wrist the selection family tracks"
        )),
    }
}

/// [`wrist_named`] the other way, for the picks an `_auto_windows` answer lists. Only the two wrists
/// reach it: [`auto_windows`] reads `LEAD_WRIST` and `TRAIL_WRIST` and no other landmark.
fn wrist_name(wrist: PoseLandmark) -> &'static str {
    match wrist {
        PoseLandmark::LeftWrist => "LEFT_WRIST",
        PoseLandmark::RightWrist => "RIGHT_WRIST",
        other => unreachable!("selection reads a wrist, not {other:?}"),
    }
}

/// `_selection_file`: the clip's frames, zeros everywhere but its tracked wrists.
fn rebuild(name: &str, clip: &Clip) -> Result<Rebuilt, String> {
    let mut tracked = Vec::with_capacity(clip.tracks.len());
    for (landmark, track) in &clip.tracks {
        let landmark = wrist_named(landmark).map_err(|e| format!("clip {name:?}: {e}"))?;
        if track.y.len() != clip.frames || track.visibility.len() != clip.frames {
            return Err(format!(
                "clip {name:?}: {landmark:?}'s track is {} and {} frames, not {}",
                track.y.len(),
                track.visibility.len(),
                clip.frames
            ));
        }
        tracked.push((landmark as usize, track));
    }
    let blank = Landmark {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        visibility: 0.0,
    };
    let frames = (0..clip.frames)
        .map(|index| {
            let mut landmarks = vec![blank.clone(); NUM_POSE_LANDMARKS];
            for &(slot, track) in &tracked {
                landmarks[slot] = Landmark {
                    x: 0.0,
                    y: track.y[index],
                    z: 0.0,
                    visibility: track.visibility[index],
                };
            }
            FrameKeypoints {
                frame_index: index as i64,
                timestamp_ms: 0.0,
                landmarks,
                camera_id: None,
            }
        })
        .collect();
    Ok(Rebuilt {
        fps: clip.fps,
        frames,
    })
}

/// A choice as `_choice` writes it, or `null` for a decline.
fn choice(choice: Option<SwingChoice>) -> Value {
    let Some(choice) = choice else {
        return Value::Null;
    };
    json!({
        "window": [choice.window.0, choice.window.1],
        "downswing": descent(&choice.downswing),
        "candidates": choice.candidates.iter().map(descent).collect::<Vec<_>>(),
        "reason": choice.reason,
    })
}

/// `Downswing._asdict()`.
fn descent(downswing: &Downswing) -> Value {
    json!({
        "top": downswing.top,
        "impact": downswing.impact,
        "rise": downswing.rise,
    })
}
