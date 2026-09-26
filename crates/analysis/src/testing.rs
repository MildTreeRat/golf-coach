//! Bodies and phase timelines the tests point a camera at. [M22 P5]
//!
//! Moved out of `measure.rs`'s test module when `checkpoints::mechanics` became the second module
//! that needs them, for the reason `crates/trigger/src/testing.rs` gives about its room tone: a
//! second copy of a fixture is a second thing that drifts, and a judging test has to be reading
//! the *same* body the measuring tests were written against or the two are measuring different
//! golfers.
//!
//! `#[cfg(test)]` and `pub(crate)`: nothing ships, and nothing outside this crate can reach it.

use contracts::keypoints::{FrameKeypoints, Landmark, PoseLandmark, NUM_POSE_LANDMARKS};
use contracts::shot::ShotData;
use contracts::swing::{PhaseSegment, SwingPhase};

/// A stored shot with every optional tile blank — callers set the two or three they read. [M22 P8b]
///
/// **A parse rather than a struct literal**, which is `engine.rs`' own fixture's argument: `ShotData`
/// has twenty optional fields a given test cares about none of, and going through `serde_json` runs
/// the shape's bounds on the way in — ADR-032 §4's rule that a pydantic constraint is behaviour
/// rather than an annotation. A literal would skip them and a fixture is exactly where a value out
/// of bounds would go unnoticed.
pub fn a_shot() -> ShotData {
    serde_json::from_str(
        r#"{"shot_id": "1", "session_id": "s", "timestamp": "2026-09-25T00:00:00Z",
            "source": "screen"}"#,
    )
    .expect("the fixture is a valid ShotData")
}

/// Every landmark parked at the origin-ish and fully confident; callers move what they need.
pub fn frame(index: i64) -> FrameKeypoints {
    FrameKeypoints {
        frame_index: index,
        timestamp_ms: index as f64 * 10.0,
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
    }
}

pub fn put(frame: &mut FrameKeypoints, which: PoseLandmark, x: f64, y: f64, visibility: f64) {
    frame.landmarks[which as usize] = Landmark {
        x,
        y,
        z: 0.0,
        visibility,
    };
}

/// A body standing still with a 0.30 shoulder width, ears and hips centred.
pub fn a_body(n: i64) -> Vec<FrameKeypoints> {
    (0..n)
        .map(|i| {
            let mut f = frame(i);
            put(&mut f, PoseLandmark::LeftShoulder, 0.65, 0.40, 1.0);
            put(&mut f, PoseLandmark::RightShoulder, 0.35, 0.40, 1.0);
            put(&mut f, PoseLandmark::LeftEar, 0.52, 0.20, 1.0);
            put(&mut f, PoseLandmark::RightEar, 0.48, 0.20, 1.0);
            put(&mut f, PoseLandmark::LeftHip, 0.55, 0.60, 1.0);
            put(&mut f, PoseLandmark::RightHip, 0.45, 0.60, 1.0);
            put(&mut f, PoseLandmark::LeftWrist, 0.52, 0.55, 1.0);
            put(&mut f, PoseLandmark::RightWrist, 0.48, 0.55, 1.0);
            put(&mut f, PoseLandmark::RightIndex, 0.48, 0.60, 1.0);
            f
        })
        .collect()
}

pub fn phase(phase: SwingPhase, start: i64, end: i64, detected: bool) -> PhaseSegment {
    PhaseSegment {
        phase,
        start_frame: start,
        end_frame: end,
        start_ms: start as f64 * 10.0,
        end_ms: end as f64 * 10.0,
        detected,
    }
}

/// A complete six-phase timeline over 49 frames, every boundary detected.
pub fn a_segmentation() -> Vec<PhaseSegment> {
    vec![
        phase(SwingPhase::Address, 0, 10, true),
        phase(SwingPhase::Backswing, 10, 27, true),
        phase(SwingPhase::Transition, 27, 33, true),
        phase(SwingPhase::Downswing, 33, 40, true),
        phase(SwingPhase::Impact, 40, 42, true),
        phase(SwingPhase::FollowThrough, 42, 49, true),
    ]
}

/// [`a_body`] with its lead wrist tracing a swing, so `segment_phases` finds one. [M22 P6]
///
/// `a_body` stands perfectly still, which is what most measuring tests want and is exactly wrong for
/// anything that runs the engine end to end: a static wrist has no rising run, so `segment_phases`
/// returns an empty list and every assertion over `SwingResult.phases` becomes vacuous. P6 found two
/// of its own tests passing that way.
///
/// The trace is `phases.rs`' own `a_swing` — still, up (y falls), down, then a finish higher than the
/// top — stretched across `n` frames by nearest-neighbour so a caller can ask for a clip of any
/// length. Nearest-neighbour rather than linear on purpose: interpolating would smooth the reversal at
/// the top into a plateau, and the plateau is what `_top_and_impact`'s tie-break reads.
pub fn a_swinging_body(n: i64) -> Vec<FrameKeypoints> {
    const TRACE: [f64; 17] = [
        0.80, 0.80, 0.80, 0.80, 0.80, // address
        0.70, 0.60, 0.50, 0.40, 0.30, // backswing
        0.40, 0.55, 0.70, 0.82, // downswing to impact
        0.70, 0.50, 0.20, // finish, higher than the top
    ];
    trace_onto(a_body(n), &TRACE)
}

/// `frames` with `pre` copies of its first frame in front, re-indexed and re-timed as one clip.
///
/// A recording that opens before the golfer does, which is what the `*_window` arguments exist for:
/// `phases::select_swing` finds the real swing inside a longer clip and hands `[start, end)` to the
/// engine. Committed as a fixture because `spec/vectors/synthetic/windowed.json` is the only vector
/// anywhere that carries a window, and it cannot show what a window *protects* — only that one was
/// applied.
pub fn with_preroll(pre: i64, frames: &[FrameKeypoints]) -> Vec<FrameKeypoints> {
    let first = frames
        .first()
        .expect("a clip to put a pre-roll in front of");
    (0..pre)
        .map(|_| first.clone())
        .chain(frames.iter().cloned())
        .enumerate()
        .map(|(i, mut frame)| {
            frame.frame_index = i as i64;
            frame.timestamp_ms = i as f64 * 10.0;
            frame
        })
        .collect()
}

/// A swing whose golfer **pauses at the top**, so the motion-start boundary collapses onto it and
/// the backswing measures shorter than the downswing. [M22 P6]
///
/// Not a contrivance: it is the reproducible case `engine::tempo_notes` and
/// `alignment::MIN_PLAUSIBLE_TEMPO` exist for. `phases::motion_start` walks back from the top looking
/// for the last quiet stretch of wrist speed, and a pause hands it one immediately — so the boundary
/// lands a frame or two below the top, `tempo_ratio` reads 0.6 here, and `phases` still reports the
/// segment as detected because from inside one clip nothing about it looks wrong.
///
/// **Nothing in `spec/vectors/` reaches this branch**, which is why the fixture is here: all six
/// synthetic vectors segment into a plausible swing, so the two sentences `tempo_notes` can produce
/// are gated by tests over this body and by no committed answer.
pub fn a_body_that_pauses_at_the_top(n: i64) -> Vec<FrameKeypoints> {
    const TRACE: [f64; 20] = [
        0.80, 0.70, 0.55, 0.40, 0.31, // backswing
        0.30, 0.30, 0.30, 0.30, // the pause, which is the whole point
        0.36, 0.42, 0.48, 0.54, 0.60, 0.66, 0.72, 0.78, 0.84, // a long descent to impact
        0.70, 0.40, // finish
    ];
    trace_onto(a_body(n), &TRACE)
}

/// [`a_swinging_body`] as the **rear** camera sees it: the *trail* wrist traces the swing. [M22 P7]
///
/// A separate fixture rather than an argument, because the difference is the one fact
/// `analyze_swing_bundle` encodes about the second view: it segments the down-the-line clip on
/// `phases::TRAIL_WRIST`, since from behind the lead wrist is the far arm. So a rear clip built from
/// `a_swinging_body` does **not** segment — its trail wrist stands still — and a test that used it
/// would silently exercise the unsegmentable branch while looking like it exercised alignment. That
/// is how P7 first wrote three of its own tests.
///
/// Both wrists trace, not just the trail one, so the clip is also a plausible body for the
/// down-the-line trajectory basis to read rather than one with an arm frozen mid-swing.
pub fn a_rear_swinging_body(n: i64) -> Vec<FrameKeypoints> {
    a_swinging_body(n)
        .into_iter()
        .map(|mut frame| {
            let lead_y = frame.landmarks[PoseLandmark::LeftWrist as usize].y;
            put(&mut frame, PoseLandmark::RightWrist, 0.48, lead_y, 1.0);
            frame
        })
        .collect()
}

/// `frames` with the lead wrist following `trace`, stretched across them by nearest-neighbour.
///
/// Nearest-neighbour rather than linear on purpose: interpolating would smooth the reversal at the
/// top into a plateau, and the plateau is what `_top_and_impact`'s tie-break reads.
fn trace_onto(mut frames: Vec<FrameKeypoints>, trace: &[f64]) -> Vec<FrameKeypoints> {
    let n = frames.len().max(1);
    for (i, frame) in frames.iter_mut().enumerate() {
        let at = ((i * trace.len()) / n).min(trace.len() - 1);
        put(frame, PoseLandmark::LeftWrist, 0.52, trace[at], 1.0);
    }
    frames
}
