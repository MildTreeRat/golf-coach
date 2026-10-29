//! Both views' windows, the confident view picked first and the other matched against it: the
//! pipeline's ordering around `analysis::phases`' clip-choosing rules. [M29 P25]
//!
//! **Twin: `api/pipeline.py`'s `_pick_swing`, `_downswing_seconds`, `_narrate_choice` and
//! `_auto_windows`**, ported in Python's order, with every log line and note in Python's words,
//! because the selection family (`spec/vectors/selection/`) recorded them from frozen Python and
//! [`crate::selection_family::run_selection`] holds this module to every recorded `_auto_windows`
//! answer, windows, notes, log and picks, by value.
//!
//! # Why this is in `golf-core` and not in `crates/lab`
//!
//! `crates/lab` is the pipeline's home, and it calls [`auto_windows`] from its bundle assembly. But
//! nothing may depend on `golf-lab`, and `golf-core rerecord` re-records the selection family under
//! an engine declaration, so a port in the lab would have left the re-record carrying every
//! `_auto_windows` answer as committed, unable to move one with the picks it is built from (the
//! M29 plan's P19 finding 6 named three ways out; this is the second, "put `_auto_windows`
//! somewhere `core` can reach"). Here, the family's gate, its re-record and the lab run one
//! definition. It is pure: frames, frame rates and strike lists in, windows and sentences out, so
//! it costs the phone's shell (M38) nothing to carry.
//!
//! # What a view is
//!
//! A [`View`] is what selection reads of a keypoints file: its frames, its `clip.fps` (`None` with
//! no clip block, as `keypoints.clip.fps if keypoints.clip else None`), and the strikes heard in
//! **its own** clip, in its own frame numbering. `None` strikes is a view nobody listened to, and
//! `Some(&[])` one listened to and found silent; the rules treat the two alike (`phases::struck`),
//! and [`PickCall`] keeps them apart because Python's picks do.

use analysis::phases::{select_matching_swing, select_swing, SwingChoice, LEAD_WRIST, TRAIL_WRIST};
use analysis::smoothing::{smooth_keypoints, DEFAULT_WINDOW};
use contracts::keypoints::{FrameKeypoints, PoseLandmark};
use storage::manifest::Role;

/// One view, as selection reads it. The module doc says what each field means.
#[derive(Debug, Clone, Copy)]
pub struct View<'a> {
    pub frames: &'a [FrameKeypoints],
    pub fps: Option<f64>,
    pub strikes: Option<&'a [i64]>,
}

/// One `_pick_swing` call [`auto_windows`] made, in the order it made them: which view, which
/// wrist, the reference it matched against (`None` for a pick made alone) and the strikes it was
/// handed. The selection family records these as each answer's `picks`.
#[derive(Debug, Clone, PartialEq)]
pub struct PickCall {
    pub role: Role,
    pub wrist: PoseLandmark,
    pub reference_downswing_s: Option<f64>,
    pub strike_frames: Option<Vec<i64>>,
}

/// [`auto_windows`]' answer: Python's `(window_face_on, window_dtl)`, and the picks behind them.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoWindows {
    pub face_on: Option<(i64, i64)>,
    pub down_the_line: Option<(i64, i64)>,
    pub picks: Vec<PickCall>,
}

/// `_pick_swing`: one view's swing, chosen alone or against a duration the other view already
/// measured. The frames smoothed at [`DEFAULT_WINDOW`] first, then [`select_matching_swing`] with a
/// reference and [`select_swing`] without one, at the clip's own fps.
///
/// Selection only, no log and no notes, which is what lets [`auto_windows`] try face-on twice. There
/// is deliberately no "matched, else alone" fallback, as Python's docstring says: both rules end at
/// the same lone-candidate escape, so `select_matching_swing` declines exactly where `select_swing`
/// would, and a fallback would be unreachable.
pub fn pick_swing(
    frames: &[FrameKeypoints],
    fps: Option<f64>,
    wrist: PoseLandmark,
    reference_downswing_s: Option<f64>,
    strike_frames: Option<&[i64]>,
) -> Option<SwingChoice> {
    let frames = smooth_keypoints(frames, DEFAULT_WINDOW);
    match reference_downswing_s {
        Some(reference) => select_matching_swing(&frames, fps, reference, wrist, strike_frames),
        None => select_swing(&frames, fps, wrist, strike_frames),
    }
}

/// `_downswing_seconds`: the chosen descent's duration, the whole of what one view offers the other.
/// `None` with no choice, no positive frame rate, or a descent measuring no time, because
/// `select_matching_swing` declines a non-positive reference and passing one on would quietly turn
/// "no reference" into "no window".
pub fn downswing_seconds(choice: Option<&SwingChoice>, fps: Option<f64>) -> Option<f64> {
    let (choice, fps) = (choice?, fps?);
    if fps <= 0.0 {
        return None;
    }
    let seconds = choice.downswing.seconds(fps);
    (seconds > 0.0).then_some(seconds)
}

/// `_narrate_choice`: one view's final pick said out loud, its window or why it has none. A decline
/// is a note as well as a log line, because scoring a whole clip that holds practice swings gives
/// numbers that look fine and describe the wrong motion. Called once per view, after the last
/// attempt at it, so a note never describes an attempt that was superseded.
fn narrate_choice(
    label: &str,
    choice: Option<&SwingChoice>,
    fps: Option<f64>,
    wrist: PoseLandmark,
    log: &mut dyn FnMut(&str),
    notes: &mut Vec<String>,
) -> Option<(i64, i64)> {
    let Some(choice) = choice else {
        // Named from the constant rather than the landmark, which would say "right wrist" for the
        // trail wrist of a right-handed golfer and be wrong for a left-handed one: Python's reason.
        let landmark = if wrist == TRAIL_WRIST {
            "trail wrist"
        } else {
            "lead wrist"
        };
        let why = if fps.is_none() {
            "the keypoints file records no fps"
        } else {
            "no plausible downswing"
        };
        log(&format!(
            "  {label}: could not pick a swing ({why}, on the {landmark}) — using the whole clip. \
             Run --list-swings and pass a window if this clip holds more than one swing"
        ));
        notes.push(format!(
            "could not pick a swing in the {label} view ({why}, reading the {landmark}) — the whole \
             clip was scored, so these numbers describe every motion in it, not one swing"
        ));
        return None;
    };
    log(&format!("  {label}: {}", choice.reason));
    Some(window(choice))
}

/// A choice's `[start, end)`, as the engine takes a window.
fn window(choice: &SwingChoice) -> (i64, i64) {
    let (start, end) = choice.window;
    (start as i64, end as i64)
}

/// `_auto_windows`: both views' windows, the confident view first and the other matched to it.
///
/// Face-on leads because it is the view that works; down the line, the last plausible descent is
/// routinely a move made after the ball was gone. **The pick is mutual**: when face-on declines,
/// the down-the-line view is picked alone and its duration becomes the reference for a second
/// attempt at face-on, with a note saying the basis is weaker. A window given by hand is not a
/// reference, so the other view is then picked alone and told so. `None` for `down_the_line` is a
/// bundle without that view; a given `window_dtl` means it is not picked at all. Python's docstring
/// has the measurements behind each rule (M10 §A1–§A2).
pub fn auto_windows(
    face_on: View<'_>,
    down_the_line: Option<View<'_>>,
    window_face_on: Option<(i64, i64)>,
    window_dtl: Option<(i64, i64)>,
    log: &mut dyn FnMut(&str),
    notes: &mut Vec<String>,
) -> AutoWindows {
    let mut picks = Vec::new();
    let mut pick = |role: Role, view: &View<'_>, wrist: PoseLandmark, reference: Option<f64>| {
        picks.push(PickCall {
            role,
            wrist,
            reference_downswing_s: reference,
            strike_frames: view.strikes.map(<[i64]>::to_vec),
        });
        pick_swing(view.frames, view.fps, wrist, reference, view.strikes)
    };

    let face_on_given = window_face_on.is_some();
    // Nothing to pick for the down-the-line view when it is absent or its window was given: an
    // explicit window always wins.
    let dtl = if window_dtl.is_none() {
        down_the_line
    } else {
        None
    };

    let mut face_on_choice = if face_on_given {
        None
    } else {
        pick(Role::FaceOn, &face_on, LEAD_WRIST, None)
    };
    let reference = downswing_seconds(face_on_choice.as_ref(), face_on.fps);

    // The trail wrist down the line, matching the engine's own down-the-line segmentation, because
    // this window is what that segmentation then runs over (M10 §A1).
    let dtl_choice = dtl
        .as_ref()
        .and_then(|view| pick(Role::DownTheLine, view, TRAIL_WRIST, reference));

    let mut matched_in_reverse = false;
    if !face_on_given && face_on_choice.is_none() {
        if let Some(view) = &dtl {
            if let Some(back_reference) = downswing_seconds(dtl_choice.as_ref(), view.fps) {
                face_on_choice = pick(Role::FaceOn, &face_on, LEAD_WRIST, Some(back_reference));
                matched_in_reverse = face_on_choice.is_some();
            }
        }
    }

    let mut window_face_on = window_face_on;
    if !face_on_given {
        window_face_on = narrate_choice(
            "face-on",
            face_on_choice.as_ref(),
            face_on.fps,
            LEAD_WRIST,
            log,
            notes,
        );
        if matched_in_reverse {
            notes.push(
                "the face-on swing could not be picked from that clip alone and was chosen by \
                 matching the down-the-line view instead — a weaker basis than usual, so check the \
                 aligned video shows the swing and not a practice move"
                    .to_string(),
            );
        }
    }
    let mut window_dtl = window_dtl;
    if let Some(view) = &dtl {
        window_dtl = narrate_choice(
            "down-the-line",
            dtl_choice.as_ref(),
            view.fps,
            TRAIL_WRIST,
            log,
            notes,
        );
        if dtl_choice.is_some() && reference.is_none() {
            let why = if face_on_given {
                "the face-on window was given rather than measured"
            } else {
                "the face-on view has no measured downswing to match against"
            };
            notes.push(format!(
                "the down-the-line swing was picked from that clip alone ({why}), with nothing to \
                 cross-check it — on a clip that keeps rolling past impact the last plausible \
                 descent is often a move made after the ball was gone"
            ));
        }
    }
    AutoWindows {
        face_on: window_face_on,
        down_the_line: window_dtl,
        picks,
    }
}
