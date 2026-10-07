//! `analysis/measure.py`. Measuring, separated from judging. [M22 P4]
//!
//! Every function here answers *what is the number* and nothing else. None of them touch
//! `ranges.json`, none produce a `CheckpointScore`, and a [`MeasureOutcome`] with no value means
//! strictly **could not measure** — unusable landmarks, a missing phase, a boundary that was
//! estimated rather than detected — never "no band exists". That split is what lets a new metric be
//! measured across the corpus before a band for it exists, and it is the reason the Python module
//! exists at all.
//!
//! Which of the six conditions failed is carried out rather than discarded, in
//! [`MeasureOutcome::reason`]. The Python draws it from `contracts.unscored.MEASUREMENT_REASONS`,
//! the subset of the vocabulary that describes a *measurement* failing — `NoBand` and
//! `NoHandedness` are in the vocabulary and deliberately not in that subset, which is this module's
//! split with the judging layer made checkable.
//!
//! # What gates this, and the hole in the gate
//!
//! `spec/vectors/stages/`'s `measure` stage records each [`POSE_MEASUREMENTS`] entry's *unrounded*
//! value or its refusal reason, on all 21 vectors. That is the numbers covered exactly.
//!
//! **Not one of those 21 vectors refuses a single measurement.** All thirteen metrics are
//! measurable on every committed swing, so the committed family gates thirteen happy paths and
//! **none** of the twenty-odd refusal branches below — including every `detail` sentence, which
//! reaches a golfer through `SwingResult.unscored` and which `docs/CONFORMANCE.md` §3 compares
//! **exactly**. A port that got every refusal string wrong would pass this phase's gate cleanly.
//! The unit tests at the bottom of this file are therefore load-bearing rather than decorative:
//! they are the only thing standing between a wrong sentence and a golfer, until a vector that
//! refuses something is recorded.
//!
//! # The two portability edges that reach this module
//!
//! [`address_sample_bounds`] rounds a **frame count**, so it goes through
//! [`pyfmt::round_index`] — one of ADR-032 §3's seventeen. And [`tempo_timings`] formats two
//! floats with `.0f` into a refusal sentence, so that goes through [`pyfmt::fixed`]. Both are
//! solved in `pyfmt` rather than here, which is what P3 bought.

use contracts::keypoints::{FrameKeypoints, PoseLandmark};
use contracts::swing::{PhaseSegment, SwingPhase};
use contracts::unscored::UnscoredReason;

use crate::phases::TRAIL_WRIST;
use crate::stats::percentile;
use pyfmt::{fixed, round_index};

/// Landmarks dimmer than this are treated as unreliable (MediaPipe convention, matches
/// [`crate::phases`]).
pub const MIN_VISIBILITY: f64 = 0.5;

/// Hips at the finish are the worst-conditioned landmarks read anywhere here: the golfer has
/// rotated ~90°, so face-on the trail hip sits behind the lead hip and the torso, and the arms and
/// club cross the body right there. MediaPipe's `visibility` is a learned logit rather than a
/// calibrated probability, so the usual 0.5 gate happily passes confidently-wrong hip estimates —
/// exactly the frames `finish_balance` is most sensitive to.
pub const MIN_HIP_VISIBILITY: f64 = 0.7;

/// A face-on shoulder width (normalized) below this is degenerate — golfer turned side-on, or
/// shoulders mis-detected — so no reliable scale can be formed and the measurement bails.
pub const MIN_SHOULDER_WIDTH: f64 = 0.02;

/// Fewest follow-through frames needed to judge how still the finish settles.
const MIN_FINISH_FRAMES: usize = 3;

/// The posture-sampling window: this fraction of the clip's own downswing duration, floored at a
/// handful of frames. Half a downswing is ~4 frames on a real-time clip and ~15 in slow motion.
const ADDRESS_SAMPLE_FRACTION: f64 = 0.5;
const ADDRESS_SAMPLE_MIN_FRAMES: i64 = 5;

/// Finish drift is summarized at this quantile rather than by a maximum: `max()` is an
/// extreme-value statistic, so one bad frame would set the whole metric — and these are the most
/// occlusion-prone frames in the swing (see [`MIN_HIP_VISIBILITY`]).
const FINISH_DRIFT_QUANTILE: f64 = 0.90;

/// Below this length (normalized frame units) a wrist-to-knuckle vector is treated as collapsed and
/// the frame is dropped: its direction is then pure landmark jitter, and normalizing it divides by
/// roughly nothing.
pub const MIN_DIRECTION_LENGTH: f64 = 3e-4;

/// How much the per-frame directions must agree before their circular mean is reported: the length
/// of the mean unit vector, 1.0 when every frame points the same way and 0.0 when they cancel. Low,
/// because this is a sentinel guard and not a steadiness band.
const MIN_DIRECTION_CONSENSUS: f64 = 0.05;

/// One number, or the reason there isn't one.
///
/// The Python replaced a bare `float | None` with this: the `None` was never ambiguous about
/// *whether* the measurement failed, but it threw away *which* of six conditions failed — and that
/// was the one thing every consumer downstream wanted and had to guess at. The reason costs nothing
/// to produce here and cannot be recovered anywhere else.
///
/// A struct rather than a Rust `Result`, and the difference is not cosmetic: a `Result::Err` reads
/// as a failure of the *program*, and this is a successful reading of a clip that does not contain
/// the thing asked for. ADR-010 §2 is the rule — a refusal is an answer — and a shape that carries a
/// `detail` beside the reason is what makes it one.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureOutcome {
    /// The measurement, or `None` if it could not be taken. Never a sentinel, never a zero.
    pub value: Option<f64>,
    /// Which condition failed, or `None` when `value` is present.
    pub reason: Option<UnscoredReason>,
    /// Which window or landmark group, for a human. `value` is what code reads.
    pub detail: String,
}

impl MeasureOutcome {
    pub fn measured(value: f64) -> Self {
        Self {
            value: Some(value),
            reason: None,
            detail: String::new(),
        }
    }

    /// `detail` is required, not optional — a reason with no window named is half an answer.
    pub fn unmeasurable(reason: UnscoredReason, detail: impl Into<String>) -> Self {
        Self {
            value: None,
            reason: Some(reason),
            detail: detail.into(),
        }
    }
}

// --------------------------------------------------------------------------- geometry helpers

/// Inclusive `(start_frame, end_frame)` for a phase, or `None` if it isn't present.
pub fn phase_bounds(phases: &[PhaseSegment], phase: SwingPhase) -> Option<(i64, i64)> {
    phases
        .iter()
        .find(|s| s.phase == phase)
        .map(|s| (s.start_frame, s.end_frame))
}

/// The Python's `keypoints[lo : hi + 1]` where `lo`/`hi` are inclusive frame indices.
///
/// Python slices clamp and never panic; Rust's do neither, and the bounds here come out of phase
/// boundaries that a degenerate clip can push past the end of the timeline. Clamping in one place
/// keeps that from being twelve chances to index out of range.
fn inclusive(keypoints: &[FrameKeypoints], lo: i64, hi: i64) -> &[FrameKeypoints] {
    let n = keypoints.len() as i64;
    let start = lo.clamp(0, n) as usize;
    let end = (hi + 1).clamp(start as i64, n) as usize;
    &keypoints[start..end]
}

/// Per-frame midpoint of two landmarks over the inclusive span, confident frames only.
///
/// **Both** landmarks must clear `min_visibility` for the frame to count — a midpoint built from one
/// good and one guessed landmark is worse than no sample at all, because it looks plausible.
pub fn midpoint_series(
    keypoints: &[FrameKeypoints],
    lo: i64,
    hi: i64,
    first: PoseLandmark,
    second: PoseLandmark,
    min_visibility: f64,
) -> Vec<(f64, f64)> {
    inclusive(keypoints, lo, hi)
        .iter()
        .filter_map(|kp| {
            let (a, b) = (kp.landmark(first), kp.landmark(second));
            (a.visibility >= min_visibility && b.visibility >= min_visibility)
                .then(|| ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0))
        })
        .collect()
}

/// Mean `(x, y)` of a point series, or `None` if it is empty.
pub fn mean_of(points: &[(f64, f64)]) -> Option<(f64, f64)> {
    if points.is_empty() {
        return None;
    }
    let n = points.len() as f64;
    let sx: f64 = points.iter().map(|&(x, _)| x).sum();
    let sy: f64 = points.iter().map(|&(_, y)| y).sum();
    Some((sx / n, sy / n))
}

/// Mean face-on shoulder width (normalized) over a span — the scale-invariance ruler.
///
/// `None` if too few confident frames or the width is degenerate (side-on / mis-detect).
pub fn shoulder_width(keypoints: &[FrameKeypoints], lo: i64, hi: i64) -> Option<f64> {
    let widths: Vec<f64> = inclusive(keypoints, lo, hi)
        .iter()
        .filter_map(|kp| {
            let left = kp.landmark(PoseLandmark::LeftShoulder);
            let right = kp.landmark(PoseLandmark::RightShoulder);
            (left.visibility >= MIN_VISIBILITY && right.visibility >= MIN_VISIBILITY)
                .then(|| (left.x - right.x).abs())
        })
        .collect();
    if widths.is_empty() {
        return None;
    }
    let width = widths.iter().sum::<f64>() / widths.len() as f64;
    (width >= MIN_SHOULDER_WIDTH).then_some(width)
}

/// A short window **ending at** the address boundary — where the golfer is actually set up.
///
/// Not the same thing as the address *phase*. That segment runs `[0, motion_start]`, and frame 0 is
/// not address, it is wherever the clip happens to begin: across the GolfDB face-on corpus the
/// golfer's head travels a median of 0.12 shoulder-widths inside that window and 0.61 at the p90,
/// because they are still walking in and settling over the ball — and `head_sway`'s entire pass band
/// is 0.42 shoulder-widths, so at the p90 the pre-roll alone outweighs the thing being scored.
///
/// Anchoring a few frames to the *end* of the segment fixes both failure modes at once (ADR-013): it
/// excludes the pre-roll, and it makes posture largely indifferent to the address boundary being
/// wrong, which matters because that boundary carries a median error of 7 frames.
///
/// Length scales with the clip's own downswing duration for the same reason the quiet run does.
/// `None` only when the phases are unusable.
pub fn address_sample_bounds(phases: &[PhaseSegment]) -> Option<(i64, i64)> {
    let address = phase_bounds(phases, SwingPhase::Address)?;
    let downswing = phase_bounds(phases, SwingPhase::Downswing)?;

    let span = (downswing.1 - downswing.0).max(1);
    // One of ADR-032 §3's seventeen frame-index roundings.
    let length = ADDRESS_SAMPLE_MIN_FRAMES.max(round_index(ADDRESS_SAMPLE_FRACTION * span as f64));

    let mut hi = address.1;
    let lo = address.0.max(hi - length);
    // A clip whose takeaway starts almost immediately (or where the boundary was estimated at 0)
    // leaves nothing behind it — widen forward instead, since the frames just after a mis-placed
    // boundary are still far closer to setup than the start of the clip is.
    if hi - lo < ADDRESS_SAMPLE_MIN_FRAMES {
        hi = downswing.0.min(lo + ADDRESS_SAMPLE_MIN_FRAMES);
    }
    Some((lo, hi.max(lo)))
}

/// Per-frame hip-center `(x, y)` over the inclusive span, confident frames only.
pub fn hip_center_points(keypoints: &[FrameKeypoints], lo: i64, hi: i64) -> Vec<(f64, f64)> {
    midpoint_series(
        keypoints,
        lo,
        hi,
        PoseLandmark::LeftHip,
        PoseLandmark::RightHip,
        MIN_HIP_VISIBILITY,
    )
}

/// Per-frame head-center `(x, y)`: the midpoint of the two **ears**.
///
/// The ear midpoint approximates the head's axis of rotation; the nose it replaced does not. Through
/// a swing the head turns, and a nose on a turning head sweeps several centimetres laterally without
/// the head itself going anywhere — a *definitional* error no smoothing removes, and on this repo's
/// own clips large enough to read a stable head as 1.18 shoulder-widths of sway.
pub fn head_center_points(keypoints: &[FrameKeypoints], lo: i64, hi: i64) -> Vec<(f64, f64)> {
    midpoint_series(
        keypoints,
        lo,
        hi,
        PoseLandmark::LeftEar,
        PoseLandmark::RightEar,
        MIN_VISIBILITY,
    )
}

/// The two durations, or the reason they could not be read.
///
/// A second shape beside [`MeasureOutcome`] rather than a reuse of it, because this carries a *pair*
/// and that one carries a number. Both exist for the same reason: the three ways tempo goes missing
/// are genuinely different clips, and merging them into one `None` is what forced every consumer to
/// guess.
#[derive(Debug, Clone, PartialEq)]
pub struct TempoTimings {
    /// `(backswing_ms, downswing_ms)`, or `None` if they could not be read.
    pub durations: Option<(f64, f64)>,
    pub reason: Option<UnscoredReason>,
    pub detail: String,
}

/// Extract `(backswing_ms, downswing_ms)` from the phase boundaries, or say why not.
///
/// Uses motion start (backswing's `start_ms`), the top of the backswing (centre of the transition
/// window) and impact (impact's `start_ms`).
///
/// Refuses when the backswing boundary was **estimated rather than detected**. That estimate is
/// derived from an assumed tempo ratio, so reporting it would echo the assumption back as an
/// observation — a guess dressed as a measurement. It costs ~14% of clips their tempo reading, which
/// is why it gets its own reason rather than sharing one with an unreadable clip: the footage was
/// fine.
pub fn tempo_timings(phases: &[PhaseSegment]) -> TempoTimings {
    let find = |p: SwingPhase| phases.iter().find(|s| s.phase == p);
    let backswing = find(SwingPhase::Backswing);
    let transition = find(SwingPhase::Transition);
    let impact = find(SwingPhase::Impact);

    let missing: Vec<&str> = [
        ("backswing", backswing.is_none()),
        ("transition", transition.is_none()),
        ("impact", impact.is_none()),
    ]
    .into_iter()
    .filter_map(|(name, absent)| absent.then_some(name))
    .collect();

    let (Some(backswing), Some(transition), Some(impact)) = (backswing, transition, impact) else {
        return TempoTimings {
            durations: None,
            reason: Some(UnscoredReason::PhaseNotSegmented),
            detail: format!(
                "tempo needs the backswing, transition and impact phases; missing {}",
                missing.join(", ")
            ),
        };
    };
    if !backswing.detected {
        return TempoTimings {
            durations: None,
            reason: Some(UnscoredReason::BoundaryEstimated),
            detail: "the start of the backswing was estimated from an assumed tempo, not detected"
                .to_string(),
        };
    }

    let motion_start_ms = backswing.start_ms;
    let top_ms = (transition.start_ms + transition.end_ms) / 2.0;
    let impact_ms = impact.start_ms;

    let backswing_ms = top_ms - motion_start_ms;
    let downswing_ms = impact_ms - top_ms;
    if backswing_ms <= 0.0 || downswing_ms <= 0.0 {
        return TempoTimings {
            durations: None,
            reason: Some(UnscoredReason::TimingDegenerate),
            detail: format!(
                "backswing {} ms and downswing {} ms; both must be positive",
                fixed(backswing_ms, 0),
                fixed(downswing_ms, 0)
            ),
        };
    }
    TempoTimings {
        durations: Some((backswing_ms, downswing_ms)),
        reason: None,
        detail: String::new(),
    }
}

/// A per-frame point extractor over an inclusive frame span — [`hip_center_points`] and friends.
type PointsFn = fn(&[FrameKeypoints], i64, i64) -> Vec<(f64, f64)>;

/// Shared body of the address-to-instant lateral metrics: `|Δx|` in shoulder widths.
///
/// Both endpoints are means over a window and the ruler comes from the address window, which is what
/// makes this family robust and what keeps it aspect-immune — an `x` distance over an `x` scale
/// cancels the 16:9 assumption outright.
///
/// `landmarks` is the human name of what `points_of` reads ("ear midpoint") and exists only for
/// `detail`. Deriving it from the function would mean matching on a function identity, which is the
/// kind of cleverness that goes stale the first time someone passes a closure.
fn lateral_travel(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
    points_of: PointsFn,
    to_phase: SwingPhase,
    landmarks: &str,
) -> MeasureOutcome {
    let Some(setup) = address_sample_bounds(phases) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::PhaseNotSegmented,
            "the address sampling window needs the address and downswing phases",
        );
    };
    let Some(target) = phase_bounds(phases, to_phase) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::PhaseNotSegmented,
            format!("no {} phase to measure travel to", to_phase.as_str()),
        );
    };

    // Scale first: without a ruler the endpoints are unusable even when both are present, and
    // reporting "landmarks unconfident" for a golfer who simply stood side-on would send them off to
    // fix the wrong thing.
    let Some(width) = shoulder_width(keypoints, setup.0, setup.1) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::ScaleUnavailable,
            "no confident, non-degenerate shoulder width across the address window",
        );
    };

    let Some(start) = mean_of(&points_of(keypoints, setup.0, setup.1)) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            format!("no confident {landmarks} frames in the address window"),
        );
    };
    let Some(end) = mean_of(&points_of(keypoints, target.0, target.1)) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            format!(
                "no confident {landmarks} frames in the {} window",
                to_phase.as_str()
            ),
        );
    };
    MeasureOutcome::measured((end.0 - start.0).abs() / width)
}

/// The address window and its shoulder-width ruler, or the refusal that stopped both.
///
/// Shared by the metrics read entirely at setup, in [`lateral_travel`]'s order: window first, then
/// scale, because a golfer who stood side-on has no ruler and telling them their landmarks were
/// unconfident sends them off to fix the wrong thing.
///
/// Deliberately **not** retrofitted onto [`lateral_travel`], [`measure_head_hip_offset_impact`] and
/// [`measure_finish_balance`], which open with the same two guards. Each interleaves a second
/// [`phase_bounds`] call between them, so folding this in would reorder their guards and change
/// which reason a clip missing that phase reports.
fn address_ruler(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
) -> Result<((i64, i64), f64), MeasureOutcome> {
    let Some(setup) = address_sample_bounds(phases) else {
        return Err(MeasureOutcome::unmeasurable(
            UnscoredReason::PhaseNotSegmented,
            "the address sampling window needs the address and downswing phases",
        ));
    };
    let Some(width) = shoulder_width(keypoints, setup.0, setup.1) else {
        return Err(MeasureOutcome::unmeasurable(
            UnscoredReason::ScaleUnavailable,
            "no confident, non-degenerate shoulder width across the address window",
        ));
    };
    Ok((setup, width))
}

/// Per-frame distance between two landmarks over the inclusive span, confident frames only.
///
/// [`midpoint_series`]'s sibling and gated the same way, for the same reason: a distance with one
/// guessed endpoint is not a shorter distance, it is a made-up one.
///
/// Distances per frame, to be averaged by the caller — *not* the distance between the two mean
/// positions. They coincide only when nothing moves, and they answer different questions when
/// something does.
pub fn separation_series(
    keypoints: &[FrameKeypoints],
    lo: i64,
    hi: i64,
    first: PoseLandmark,
    second: PoseLandmark,
) -> Vec<f64> {
    inclusive(keypoints, lo, hi)
        .iter()
        .filter_map(|kp| {
            let (a, b) = (kp.landmark(first), kp.landmark(second));
            (a.visibility >= MIN_VISIBILITY && b.visibility >= MIN_VISIBILITY)
                // `((dx)**2 + (dy)**2) ** 0.5` in the Python, not `math.hypot`. Kept literally —
                // see `phases::wrist_speed` for why the two are not interchangeable.
                .then(|| ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt())
        })
        .collect()
}

/// Per-frame **unit** vector from `origin` to `tip` over the inclusive span.
///
/// The third member of [`midpoint_series`]'s family and gated the same way, plus one gate of its
/// own: a frame whose two landmarks have collapsed to within [`MIN_DIRECTION_LENGTH`] is dropped
/// rather than normalized, because its direction is jitter divided by roughly nothing.
///
/// Unit vectors rather than raw ones, so a caller averaging them gets a **circular mean** of the
/// directions instead of a length-weighted one. The distinction matters exactly when it is least
/// visible: a frame where the pose model placed the knuckle twice as far out would otherwise count
/// twice toward the answer, on no evidence that its angle was any better.
pub fn direction_series(
    keypoints: &[FrameKeypoints],
    lo: i64,
    hi: i64,
    origin: PoseLandmark,
    tip: PoseLandmark,
) -> Vec<(f64, f64)> {
    let mut directions = Vec::new();
    for kp in inclusive(keypoints, lo, hi) {
        let (a, b) = (kp.landmark(origin), kp.landmark(tip));
        if a.visibility < MIN_VISIBILITY || b.visibility < MIN_VISIBILITY {
            continue;
        }
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        // `math.hypot` here, where the Python calls `math.hypot` — unlike `separation_series` above.
        let length = dx.hypot(dy);
        if length >= MIN_DIRECTION_LENGTH {
            directions.push((dx / length, dy / length));
        }
    }
    directions
}

// --------------------------------------------------------------------------- the measurements

/// Backswing:downswing time ratio. Frame rate cancels, so slo-mo and real-time agree.
pub fn measure_tempo_ratio(phases: &[PhaseSegment]) -> MeasureOutcome {
    let timings = tempo_timings(phases);
    let Some((backswing_ms, downswing_ms)) = timings.durations else {
        // `tempo_timings` already decided which of the three conditions failed; re-deciding here
        // would be a second opinion on the same evidence.
        return MeasureOutcome::unmeasurable(
            timings.reason.expect("a refusal always carries its reason"),
            timings.detail,
        );
    };
    MeasureOutcome::measured(backswing_ms / downswing_ms)
}

/// Backswing duration in milliseconds — motion start to the top.
///
/// Unlike [`measure_tempo_ratio`], frame rate does **not** cancel here, so this is only meaningful
/// on a real-time clip. That is a property of the capture rather than of this function, which is why
/// it is not gated here: the phases carry millisecond timestamps derived from the clip's own fps, and
/// a slow-motion clip arrived with slow-motion timestamps long before this point.
pub fn measure_backswing_ms(phases: &[PhaseSegment]) -> MeasureOutcome {
    tempo_duration(phases, true)
}

/// Downswing duration in milliseconds — the top to impact. See [`measure_backswing_ms`].
pub fn measure_downswing_ms(phases: &[PhaseSegment]) -> MeasureOutcome {
    tempo_duration(phases, false)
}

/// One half of [`tempo_timings`], as a measurement.
///
/// Both halves and the ratio come from the same call, so all three inherit its three refusal paths
/// and cannot disagree about whether a swing was timeable.
fn tempo_duration(phases: &[PhaseSegment], backswing: bool) -> MeasureOutcome {
    let timings = tempo_timings(phases);
    let Some(durations) = timings.durations else {
        return MeasureOutcome::unmeasurable(
            timings.reason.expect("a refusal always carries its reason"),
            timings.detail,
        );
    };
    MeasureOutcome::measured(if backswing { durations.0 } else { durations.1 })
}

/// Lateral head travel, address window -> impact window, in shoulder widths.
pub fn measure_head_sway(keypoints: &[FrameKeypoints], phases: &[PhaseSegment]) -> MeasureOutcome {
    lateral_travel(
        keypoints,
        phases,
        head_center_points,
        SwingPhase::Impact,
        "ear midpoint",
    )
}

/// Lateral hip travel, address window -> impact window, in shoulder widths.
///
/// The structural twin of [`measure_head_sway`], and it answers a different coaching question: a head
/// that stays put over hips that slide is a lateral slide, while head and hips moving together is a
/// body that swayed. Neither is visible from head sway alone.
pub fn measure_hip_sway(keypoints: &[FrameKeypoints], phases: &[PhaseSegment]) -> MeasureOutcome {
    lateral_travel(
        keypoints,
        phases,
        hip_center_points,
        SwingPhase::Impact,
        "hip midpoint",
    )
}

/// Lateral hip travel, address window -> transition (top) window, in shoulder widths.
///
/// Magnitude only. Direction would separate a sway off the ball from a reverse pivot, but the sign is
/// camera-relative — see [`measure_head_hip_offset_impact`] for the same problem stated in full.
pub fn measure_hip_shift_at_top(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
) -> MeasureOutcome {
    lateral_travel(
        keypoints,
        phases,
        hip_center_points,
        SwingPhase::Transition,
        "hip midpoint",
    )
}

/// Signed head-center minus hip-center offset over the impact window, in shoulder widths.
///
/// "Staying behind the ball" — and the only metric here that is signed, because the magnitude alone
/// says nothing: a head ahead of the hips at impact and a head behind them are opposite faults with
/// the same absolute value.
///
/// **The sign is camera-relative, and that is a known limitation rather than an oversight.** Positive
/// means the head sits to the right of the hip center *in the image*, and which side that is in swing
/// terms depends on handedness. The judging layer resolves it; this records the camera-frame number.
pub fn measure_head_hip_offset_impact(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
) -> MeasureOutcome {
    let Some(setup) = address_sample_bounds(phases) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::PhaseNotSegmented,
            "the address sampling window needs the address and downswing phases",
        );
    };
    let Some(impact) = phase_bounds(phases, SwingPhase::Impact) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::PhaseNotSegmented,
            "no impact phase to read the offset at",
        );
    };
    let Some(width) = shoulder_width(keypoints, setup.0, setup.1) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::ScaleUnavailable,
            "no confident, non-degenerate shoulder width across the address window",
        );
    };

    let head = mean_of(&head_center_points(keypoints, impact.0, impact.1));
    let hips = mean_of(&hip_center_points(keypoints, impact.0, impact.1));
    let (Some(head), Some(hips)) = (head, hips) else {
        let missing = if head.is_none() {
            "ear midpoint"
        } else {
            "hip midpoint"
        };
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            format!("no confident {missing} frames in the impact window"),
        );
    };
    MeasureOutcome::measured((head.0 - hips.0) / width)
}

/// Change in the signed head-hip offset from address to impact, in shoulder widths.
///
/// The *same* quantity as [`measure_head_hip_offset_impact`], read as a change across the swing
/// rather than as an absolute position: how much rearward separation the golfer's motion actually
/// created.
///
/// **Why this exists beside the absolute, rather than instead of it.** Shoulder-width normalization
/// removes the `1/Z` scale but not **yaw**: a camera off-square to the target line converts the
/// head/hip *depth* difference into apparent horizontal offset, and at impact the hips have rotated
/// open while the head has not. Measured directly, this repo's own bay clips carry a 0.32
/// shoulder-width disagreement with GolfDB *at address*, where the body is square and there is no
/// swing yet to disagree about — about 4x this metric's own measurement error. Subtracting the
/// address reading removes the static term by construction.
///
/// Both readings share **one ruler**, the shoulder width measured over the address window, so the
/// subtraction is of two comparable quantities. Using each instant's own width would put a second
/// difference into the result and defeat the point.
pub fn measure_head_hip_gain(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
) -> MeasureOutcome {
    let Some(setup) = address_sample_bounds(phases) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::PhaseNotSegmented,
            "the address sampling window needs the address and downswing phases",
        );
    };
    let Some(impact) = phase_bounds(phases, SwingPhase::Impact) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::PhaseNotSegmented,
            "no impact phase to difference the offset against",
        );
    };
    let Some(width) = shoulder_width(keypoints, setup.0, setup.1) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::ScaleUnavailable,
            "no confident, non-degenerate shoulder width across the address window",
        );
    };

    let mut offsets = Vec::with_capacity(2);
    for (window, (lo, hi)) in [("address", setup), ("impact", impact)] {
        let head = mean_of(&head_center_points(keypoints, lo, hi));
        let hips = mean_of(&hip_center_points(keypoints, lo, hi));
        let (Some(head), Some(hips)) = (head, hips) else {
            let missing = if head.is_none() {
                "ear midpoint"
            } else {
                "hip midpoint"
            };
            return MeasureOutcome::unmeasurable(
                UnscoredReason::LandmarksUnconfident,
                format!("no confident {missing} frames in the {window} window"),
            );
        };
        offsets.push((head.0 - hips.0) / width);
    }
    MeasureOutcome::measured(offsets[1] - offsets[0])
}

/// Hip-center drift from its own mean through the follow-through, in shoulder widths.
pub fn measure_finish_balance(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
) -> MeasureOutcome {
    let Some(follow_through) = phase_bounds(phases, SwingPhase::FollowThrough) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::PhaseNotSegmented,
            "no follow-through phase — the clip may end at impact",
        );
    };
    let Some(setup) = address_sample_bounds(phases) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::PhaseNotSegmented,
            "the address sampling window needs the address and downswing phases",
        );
    };
    let Some(width) = shoulder_width(keypoints, setup.0, setup.1) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::ScaleUnavailable,
            "no confident, non-degenerate shoulder width across the address window",
        );
    };

    // `TooFewFrames` rather than `LandmarksUnconfident` even at zero: the hips are gated at
    // `MIN_HIP_VISIBILITY` here, harder than anywhere else, so "not enough of them" is the honest
    // description of both a short follow-through and an occluded one.
    let points = hip_center_points(keypoints, follow_through.0, follow_through.1);
    if points.len() < MIN_FINISH_FRAMES {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::TooFewFrames,
            format!(
                "{} confident hip frames through the follow-through, need {MIN_FINISH_FRAMES}",
                points.len()
            ),
        );
    }

    // The Python keeps a `None` guard here that the length check above has already made
    // unreachable, and says so. `expect` is that same statement in Rust's grammar.
    let (mean_x, mean_y) = mean_of(&points).expect("past the length check there are points");
    let drifts: Vec<f64> = points
        .iter()
        .map(|&(x, y)| ((x - mean_x).powi(2) + (y - mean_y).powi(2)).sqrt())
        .collect();
    let p90 = percentile(&drifts, FINISH_DRIFT_QUANTILE).expect("drifts is non-empty");
    MeasureOutcome::measured(p90 / width)
}

/// Mean wrist-to-wrist distance across the address window, in shoulder widths.
///
/// Both hands are on one grip, so this is close to a **constant of the golfer** rather than a
/// description of the swing. That is what makes it the hand pair's canary: a swing whose separation
/// reads far from the golfer's own usual value did not have a wider grip in it — the wrists were
/// mis-placed, and any other hand number from the same clip is suspect for the same reason.
///
/// **It mixes `x` and `y`, so unlike the lateral family it does not cancel the frame's pixel
/// aspect** — the two wrists are separated mostly along the shaft, which face-on is mostly vertical,
/// so an `|dx|` version would measure the small residual and be dominated by jitter.
pub fn measure_hand_separation(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
) -> MeasureOutcome {
    let ((lo, hi), width) = match address_ruler(keypoints, phases) {
        Ok(ready) => ready,
        Err(refusal) => return refusal,
    };

    let separations = separation_series(
        keypoints,
        lo,
        hi,
        PoseLandmark::LeftWrist,
        PoseLandmark::RightWrist,
    );
    if separations.is_empty() {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            "no address-window frames with both wrists confident",
        );
    }
    let mean = separations.iter().sum::<f64>() / separations.len() as f64;
    MeasureOutcome::measured(mean / width)
}

/// How far the hands hang below the shoulders at address, in shoulder widths.
///
/// Signed `y` of the wrist midpoint minus the shoulder midpoint, so **positive is hands below
/// shoulders** — which is every address position a golfer has ever taken, making the sign a sanity
/// check rather than a distinction.
///
/// **The sign is not camera-relative and no handedness question arises.** Down is down in an image
/// whichever way the golfer faces. What it is not free of is the frame's pixel aspect — a `y`
/// quantity over an `x` ruler — and *that* is the reason there is no band yet.
pub fn measure_hand_height(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
) -> MeasureOutcome {
    let ((lo, hi), width) = match address_ruler(keypoints, phases) {
        Ok(ready) => ready,
        Err(refusal) => return refusal,
    };

    let Some(hands) = mean_of(&midpoint_series(
        keypoints,
        lo,
        hi,
        PoseLandmark::LeftWrist,
        PoseLandmark::RightWrist,
        MIN_VISIBILITY,
    )) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            "no confident wrist midpoint frames in the address window",
        );
    };
    // The ruler above needed these two landmarks over this same window, so this branch is the
    // Python's documented `pragma: no cover` — kept because the type says it can happen.
    let Some(shoulders) = mean_of(&midpoint_series(
        keypoints,
        lo,
        hi,
        PoseLandmark::LeftShoulder,
        PoseLandmark::RightShoulder,
        MIN_VISIBILITY,
    )) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            "no confident shoulder midpoint frames in the address window",
        );
    };
    MeasureOutcome::measured((hands.1 - shoulders.1) / width)
}

/// Signed wrist-midpoint minus hip-center offset at address, in shoulder widths.
///
/// Where the hands sit along the body at setup — the 2D shadow of shaft lean. Hands ahead of the hip
/// center and hands behind it are opposite setups with the same absolute value, so like
/// [`measure_head_hip_offset_impact`] this is stored **signed**.
///
/// Unlike the other two hand metrics this one is `x`-over-`x`, so it **does** cancel the frame's
/// pixel aspect. It is unjudged for the handedness reason, not for that one.
pub fn measure_hand_offset_from_hips(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
) -> MeasureOutcome {
    let ((lo, hi), width) = match address_ruler(keypoints, phases) {
        Ok(ready) => ready,
        Err(refusal) => return refusal,
    };

    let Some(hands) = mean_of(&midpoint_series(
        keypoints,
        lo,
        hi,
        PoseLandmark::LeftWrist,
        PoseLandmark::RightWrist,
        MIN_VISIBILITY,
    )) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            "no confident wrist midpoint frames in the address window",
        );
    };
    let Some(hips) = mean_of(&hip_center_points(keypoints, lo, hi)) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            "no confident hip midpoint frames in the address window",
        );
    };
    MeasureOutcome::measured((hands.0 - hips.0) / width)
}

/// Angle of the trail wrist-to-index-knuckle vector off image-vertical, in degrees.
///
/// **A proxy, and labelled one everywhere it appears.** Grip rotation is a three-dimensional quantity
/// about the shaft axis; this is two image points on the back of one hand, seen from the camera worst
/// placed to see rotation *in* the swing plane. It is stored so "does this golfer's trail hand sit the
/// same way every swing?" becomes answerable from stored numbers.
///
/// **No shoulder ruler is taken, deliberately.** An angle is already scale-free, so calling
/// [`address_ruler`] would refuse a side-on clip for `ScaleUnavailable` over a ruler this measurement
/// never divides by — a refusal for a reason that was not the problem.
///
/// The average is a **circular mean** — the direction of the summed unit vectors — not the mean of
/// the per-frame angles. Angles wrap at ±180°, which for this vector is the hand pointing at the top
/// of the frame, so a pair of mis-detected frames either side of the wrap would average to a
/// confident straight-down.
pub fn measure_trail_hand_roll(
    keypoints: &[FrameKeypoints],
    phases: &[PhaseSegment],
) -> MeasureOutcome {
    let Some((lo, hi)) = address_sample_bounds(phases) else {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::PhaseNotSegmented,
            "the address sampling window needs the address and downswing phases",
        );
    };

    let directions = direction_series(keypoints, lo, hi, TRAIL_WRIST, PoseLandmark::RightIndex);
    if directions.is_empty() {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            "no address-window frames with a readable trail wrist-to-index-knuckle vector",
        );
    }
    let n = directions.len() as f64;
    let dx = directions.iter().map(|&(x, _)| x).sum::<f64>() / n;
    let dy = directions.iter().map(|&(_, y)| y).sum::<f64>() / n;
    if dx.hypot(dy) < MIN_DIRECTION_CONSENSUS {
        // `atan2(0.0, 0.0)` is 0.0, and 0 degrees is a perfectly plausible reading here. Directions
        // that cancel this completely are landmarks pointing every which way, not a hand pointing
        // straight down, and ADR-010 §2 says refuse rather than report the sentinel.
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            "the trail hand's direction did not survive averaging across the address window",
        );
    }
    // `math.degrees(math.atan2(dx, dy))` — note the argument order, which is `atan2(x, y)` and not
    // the usual `atan2(y, x)`: the angle is measured off image-**vertical**, not off horizontal.
    MeasureOutcome::measured(dx.atan2(dy).to_degrees())
}

/// How to take one pose measurement, and what the resulting number is.
///
/// The Python was three dicts keyed by the same names — the function, the unit and the detail — which
/// is three places to edit and two chances to add a metric that measures fine and then stores with
/// the wrong unit.
pub struct PoseMeasurement {
    /// Takes the smoothed frames and the phase segmentation, returns a [`MeasureOutcome`].
    pub measure: fn(&[FrameKeypoints], &[PhaseSegment]) -> MeasureOutcome,
    /// Goes onto `Measurement.unit`.
    pub unit: &'static str,
    /// How the measurement is taken, carried onto `Measurement.detail` so a stored number can be
    /// re-derived or re-normalized later without reading this module.
    pub detail: &'static str,
}

/// Name -> how to measure it, for everything derived from face-on pose over a full swing.
///
/// **A slice, not a map, and that is ADR-032 §3's third edge.** The Python is a `dict`, which since
/// 3.7 iterates in insertion order, and the engine walks it to build `Measurement` rows whose order
/// `docs/CONFORMANCE.md` §3 compares **structurally** — same list, same order. A `HashMap` here would
/// destroy that outright and a `BTreeMap` would replace it with alphabetical, which is a different
/// wrong answer. The order below is the Python's, and the gate that catches a reordering is the
/// `measure` stage, which records the rows positionally.
///
/// Names ending `_norm` are shoulder-width-normalized, which the band-derivation script keys on.
/// `tempo_ratio` is deliberately outside that convention.
///
/// Not every metric here is judged. A name gains a checkpoint by appearing in
/// `contracts/checkpoints.py`'s registry and a band in `benchmarks/ranges.json`; until then it is
/// measured and stored and nothing else, which is the order M6.5 exists to allow.
pub static POSE_MEASUREMENTS: &[(&str, PoseMeasurement)] = &[
    (
        "tempo_ratio",
        PoseMeasurement {
            measure: |_, phases| measure_tempo_ratio(phases),
            unit: "ratio",
            detail: "backswing:downswing time, from phase instants; frame rate cancels",
        },
    ),
    // The two halves the ratio above is built from, kept rather than divided away. Nothing judges
    // them — no band, no checkpoint, no placement — because tempo is already scored once and a
    // second verdict over the same two instants would double-count it (ADR-023). They are here so a
    // stored swing can say "your downswing took 384 ms", which the ratio cannot recover.
    (
        "backswing_ms",
        PoseMeasurement {
            measure: |_, phases| measure_backswing_ms(phases),
            unit: "ms",
            detail: "motion start -> top, from phase instants; real-time clips only, fps does not cancel",
        },
    ),
    (
        "downswing_ms",
        PoseMeasurement {
            measure: |_, phases| measure_downswing_ms(phases),
            unit: "ms",
            detail: "top -> impact, from phase instants; real-time clips only, fps does not cancel",
        },
    ),
    (
        "head_sway_norm",
        PoseMeasurement {
            measure: measure_head_sway,
            unit: "shoulder_widths",
            detail: "|dx| of ear midpoint, address window -> impact window",
        },
    ),
    (
        "finish_balance_norm",
        PoseMeasurement {
            measure: measure_finish_balance,
            unit: "shoulder_widths",
            detail: "p90 hip-center drift through follow-through",
        },
    ),
    (
        "hip_sway_norm",
        PoseMeasurement {
            measure: measure_hip_sway,
            unit: "shoulder_widths",
            detail: "|dx| of hip midpoint, address window -> impact window",
        },
    ),
    (
        "hip_shift_at_top_norm",
        PoseMeasurement {
            measure: measure_hip_shift_at_top,
            unit: "shoulder_widths",
            detail: "|dx| of hip midpoint, address window -> transition window",
        },
    ),
    (
        "head_hip_offset_impact_norm",
        PoseMeasurement {
            measure: measure_head_hip_offset_impact,
            unit: "shoulder_widths",
            detail: "signed (head - hips) dx over the impact window; + is image-right, camera-frame — \
carries a static camera bias, see head_hip_gain_norm",
        },
    ),
    (
        "head_hip_gain_norm",
        PoseMeasurement {
            measure: measure_head_hip_gain,
            unit: "shoulder_widths",
            detail: "change in signed (head - hips) dx, address window -> impact window, one shared \
shoulder-width ruler; + is image-right, camera-frame, handedness resolved when judged",
        },
    ),
    // The two hand metrics [M14 P4]. Both read the address window and nothing wider — face-on the
    // wrists track in 1.00 of address frames against about 0.65 over the whole clip. Both mix `x`
    // and `y` and say so, because a stored number that quietly carries the frame shape cannot be
    // re-normalized later and one that declares it can.
    (
        "hand_separation_norm",
        PoseMeasurement {
            measure: measure_hand_separation,
            unit: "shoulder_widths",
            detail: "mean per-frame wrist-to-wrist distance over the address window; mixes x and y, so it \
carries the frame's pixel aspect where the lateral metrics cancel it",
        },
    ),
    (
        "hand_height_norm",
        PoseMeasurement {
            measure: measure_hand_height,
            unit: "shoulder_widths",
            detail: "signed (wrists - shoulders) dy over the address window, + is hands below shoulders; a y \
quantity over an x ruler, so it carries the frame's pixel aspect. Not handedness-relative",
        },
    ),
    // M14 P5's pair, and the two differ in every way the P4 pair matched. One is x-over-x and
    // cancels the pixel aspect; the other is an angle and never had a ruler.
    (
        "hand_offset_from_hips_norm",
        PoseMeasurement {
            measure: measure_hand_offset_from_hips,
            unit: "shoulder_widths",
            detail: "signed (wrists - hips) dx over the address window; + is image-right, camera-frame, \
handedness resolved when judged. x over x, so the frame's pixel aspect cancels",
        },
    ),
    (
        "trail_hand_roll_deg",
        PoseMeasurement {
            measure: measure_trail_hand_roll,
            unit: "degrees",
            detail: "PROXY: circular-mean angle of the trail wrist-to-index-knuckle vector off image-vertical \
at address, + is image-right. Two points on one hand, not grip rotation about the shaft",
        },
    ),
];

/// The measurements that are an absolute time, and therefore **not** frame-rate invariant.
///
/// Every other metric in the registry is a ratio or a shoulder-width, which is why the GolfDB tooling
/// can treat a slow-motion clip and a real-time clip as the same kind of evidence. Derived from the
/// unit rather than listed, exactly as the Python derives it, so a third duration metric is covered
/// the day it is added rather than the day someone remembers this set exists.
pub fn fps_dependent_measurements() -> Vec<&'static str> {
    POSE_MEASUREMENTS
        .iter()
        .filter(|(_, m)| m.unit == "ms")
        .map(|&(name, _)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    // Moved to `crate::testing` in M22 P5, when `checkpoints::mechanics` became the second module
    // that needs a body to point a camera at. Same fixtures, one copy — the argument
    // `crates/trigger/src/testing.rs` already makes about its room tone.
    use crate::testing::{a_body, a_segmentation, phase, put};

    /// Address 0-10, backswing 10-27, transition 27-33, downswing 33-40, impact 40-42, finish 42-49.
    // ------------------------------------------------------------------ the registry

    #[test]
    fn the_registry_holds_thirteen_metrics_in_the_python_order() {
        let names: Vec<&str> = POSE_MEASUREMENTS.iter().map(|&(n, _)| n).collect();
        assert_eq!(
            names,
            [
                "tempo_ratio",
                "backswing_ms",
                "downswing_ms",
                "head_sway_norm",
                "finish_balance_norm",
                "hip_sway_norm",
                "hip_shift_at_top_norm",
                "head_hip_offset_impact_norm",
                "head_hip_gain_norm",
                "hand_separation_norm",
                "hand_height_norm",
                "hand_offset_from_hips_norm",
                "trail_hand_roll_deg",
            ]
        );
    }

    #[test]
    fn the_duration_metrics_are_derived_from_their_unit() {
        assert_eq!(
            fps_dependent_measurements(),
            ["backswing_ms", "downswing_ms"]
        );
    }

    // ------------------------------------------------------------------ the helpers

    #[test]
    fn a_missing_phase_has_no_bounds() {
        let only_address = [phase(SwingPhase::Address, 0, 4, true)];
        assert_eq!(
            phase_bounds(&only_address, SwingPhase::Address),
            Some((0, 4))
        );
        assert_eq!(phase_bounds(&only_address, SwingPhase::Impact), None);
    }

    #[test]
    fn a_midpoint_needs_both_landmarks_confident() {
        let mut frames = a_body(3);
        put(&mut frames[1], PoseLandmark::LeftEar, 0.52, 0.20, 0.1);
        let points = head_center_points(&frames, 0, 2);
        assert_eq!(
            points.len(),
            2,
            "the half-seen frame is dropped, not halved"
        );
        assert_eq!(points[0], (0.5, 0.20));
    }

    #[test]
    fn hips_are_gated_harder_than_everything_else() {
        let mut frames = a_body(2);
        // 0.6 clears MIN_VISIBILITY and not MIN_HIP_VISIBILITY.
        put(&mut frames[0], PoseLandmark::LeftHip, 0.55, 0.60, 0.6);
        put(&mut frames[0], PoseLandmark::RightHip, 0.45, 0.60, 0.6);
        assert_eq!(hip_center_points(&frames, 0, 1).len(), 1);
        assert_eq!(
            midpoint_series(
                &frames,
                0,
                1,
                PoseLandmark::LeftHip,
                PoseLandmark::RightHip,
                MIN_VISIBILITY
            )
            .len(),
            2
        );
    }

    #[test]
    fn a_degenerate_shoulder_width_has_no_ruler() {
        let mut frames = a_body(2);
        // `0.65 - 0.35` is `0.30000000000000004`, which is the whole reason §3 compares floats
        // within a tolerance and this assertion does too.
        let width = shoulder_width(&frames, 0, 1).unwrap();
        assert!((width - 0.30).abs() < 1e-12, "got {width}");
        for f in frames.iter_mut() {
            put(f, PoseLandmark::LeftShoulder, 0.505, 0.40, 1.0);
            put(f, PoseLandmark::RightShoulder, 0.495, 0.40, 1.0);
        }
        assert_eq!(
            shoulder_width(&frames, 0, 1),
            None,
            "0.01 is below the floor"
        );
    }

    /// A phase boundary past the end of the timeline clamps rather than panicking — Python's slice
    /// semantics, which this port has to supply by hand.
    #[test]
    fn an_out_of_range_span_clamps_like_a_python_slice() {
        let frames = a_body(3);
        assert_eq!(head_center_points(&frames, 0, 99).len(), 3);
        assert_eq!(head_center_points(&frames, 5, 99).len(), 0);
        assert_eq!(head_center_points(&frames, -4, 1).len(), 2);
    }

    #[test]
    fn the_address_window_ends_at_the_address_boundary() {
        // Downswing spans 7 frames, so the length is max(5, round(0.5 * 7)) = max(5, 4) = 5.
        assert_eq!(address_sample_bounds(&a_segmentation()), Some((5, 10)));
    }

    /// The forward-widening branch: a takeaway that starts at frame 1 leaves nothing behind it.
    #[test]
    fn an_immediate_takeaway_widens_the_window_forward() {
        let mut phases = a_segmentation();
        phases[0] = phase(SwingPhase::Address, 0, 1, true);
        assert_eq!(address_sample_bounds(&phases), Some((0, 5)));
    }

    #[test]
    fn a_missing_downswing_leaves_no_address_window() {
        let phases = [phase(SwingPhase::Address, 0, 10, true)];
        assert_eq!(address_sample_bounds(&phases), None);
    }

    /// The window length is a rounded frame count — ADR-032 §3's first edge, at this module's one
    /// site. `round(0.5 * 21)` is `round(10.5)`, which is **10** in Python and 11 in a
    /// half-away-from-zero rule.
    #[test]
    fn the_window_length_rounds_half_to_even() {
        let mut phases = a_segmentation();
        phases[3] = phase(SwingPhase::Downswing, 33, 54, true);
        // length = max(5, round(10.5)) = 10, so lo = 10 - 10 = 0.
        assert_eq!(address_sample_bounds(&phases), Some((0, 10)));
    }

    // ------------------------------------------------------------------ tempo

    #[test]
    fn tempo_reads_the_top_as_the_centre_of_the_transition_window() {
        let timings = tempo_timings(&a_segmentation());
        // top_ms = (270 + 330) / 2 = 300; backswing = 300 - 100 = 200; downswing = 400 - 300 = 100.
        assert_eq!(timings.durations, Some((200.0, 100.0)));
        assert_eq!(measure_tempo_ratio(&a_segmentation()).value, Some(2.0));
        assert_eq!(measure_backswing_ms(&a_segmentation()).value, Some(200.0));
        assert_eq!(measure_downswing_ms(&a_segmentation()).value, Some(100.0));
    }

    /// One of the refusal sentences no committed vector exercises. See the module doc.
    #[test]
    fn an_estimated_backswing_boundary_refuses_all_three_tempo_metrics() {
        let mut phases = a_segmentation();
        phases[1] = phase(SwingPhase::Backswing, 10, 27, false);
        for outcome in [
            measure_tempo_ratio(&phases),
            measure_backswing_ms(&phases),
            measure_downswing_ms(&phases),
        ] {
            assert_eq!(outcome.value, None);
            assert_eq!(outcome.reason, Some(UnscoredReason::BoundaryEstimated));
            assert_eq!(
                outcome.detail,
                "the start of the backswing was estimated from an assumed tempo, not detected"
            );
        }
    }

    #[test]
    fn missing_phases_are_named_in_the_order_tempo_needs_them() {
        let phases = [phase(SwingPhase::Transition, 0, 4, true)];
        let outcome = measure_tempo_ratio(&phases);
        assert_eq!(outcome.reason, Some(UnscoredReason::PhaseNotSegmented));
        assert_eq!(
            outcome.detail,
            "tempo needs the backswing, transition and impact phases; missing backswing, impact"
        );
    }

    /// The `.0f` interpolation — ADR-032 §3's second edge, at this module's one site.
    #[test]
    fn a_degenerate_timing_formats_its_two_numbers_the_python_way() {
        let phases = vec![
            phase(SwingPhase::Backswing, 10, 27, true),
            // Transition centred *before* the backswing start, so the backswing duration is
            // negative; impact then sits before the top, so the downswing is negative too.
            PhaseSegment {
                phase: SwingPhase::Transition,
                start_frame: 5,
                end_frame: 6,
                start_ms: 49.5,
                end_ms: 50.5,
                detected: true,
            },
            phase(SwingPhase::Impact, 4, 5, true),
        ];
        let outcome = measure_tempo_ratio(&phases);
        assert_eq!(outcome.reason, Some(UnscoredReason::TimingDegenerate));
        // backswing = 50 - 100 = -50; downswing = 40 - 50 = -10.
        assert_eq!(
            outcome.detail,
            "backswing -50 ms and downswing -10 ms; both must be positive"
        );
    }

    // ------------------------------------------------------------------ the spatial family

    #[test]
    fn a_body_that_never_moves_has_no_sway() {
        let frames = a_body(50);
        let phases = a_segmentation();
        assert_eq!(measure_head_sway(&frames, &phases).value, Some(0.0));
        assert_eq!(measure_hip_sway(&frames, &phases).value, Some(0.0));
        assert_eq!(measure_hip_shift_at_top(&frames, &phases).value, Some(0.0));
        assert_eq!(measure_head_hip_gain(&frames, &phases).value, Some(0.0));
    }

    #[test]
    fn head_sway_is_lateral_travel_over_the_shoulder_width() {
        let mut frames = a_body(50);
        // Slide the ears 0.06 right across the impact window: 0.06 / 0.30 = 0.2 shoulder widths.
        for f in frames.iter_mut().take(43).skip(40) {
            put(f, PoseLandmark::LeftEar, 0.58, 0.20, 1.0);
            put(f, PoseLandmark::RightEar, 0.54, 0.20, 1.0);
        }
        let value = measure_head_sway(&frames, &a_segmentation()).value.unwrap();
        assert!((value - 0.2).abs() < 1e-12, "got {value}");
    }

    #[test]
    fn the_offset_at_impact_is_signed_and_the_sway_is_not() {
        let mut frames = a_body(50);
        // Ears 0.06 to the image-*left* of the hips through impact.
        for f in frames.iter_mut().take(43).skip(40) {
            put(f, PoseLandmark::LeftEar, 0.46, 0.20, 1.0);
            put(f, PoseLandmark::RightEar, 0.42, 0.20, 1.0);
        }
        let phases = a_segmentation();
        let signed = measure_head_hip_offset_impact(&frames, &phases)
            .value
            .unwrap();
        let unsigned = measure_head_sway(&frames, &phases).value.unwrap();
        assert!((signed + 0.2).abs() < 1e-12, "got {signed}");
        assert!((unsigned - 0.2).abs() < 1e-12, "got {unsigned}");
    }

    #[test]
    fn the_scale_guard_fires_before_the_landmark_guard() {
        let mut frames = a_body(50);
        for f in frames.iter_mut() {
            // No ruler *and* no ears: the refusal must name the ruler.
            put(f, PoseLandmark::LeftShoulder, 0.505, 0.40, 1.0);
            put(f, PoseLandmark::RightShoulder, 0.495, 0.40, 1.0);
            put(f, PoseLandmark::LeftEar, 0.52, 0.20, 0.0);
            put(f, PoseLandmark::RightEar, 0.48, 0.20, 0.0);
        }
        let outcome = measure_head_sway(&frames, &a_segmentation());
        assert_eq!(outcome.reason, Some(UnscoredReason::ScaleUnavailable));
        assert_eq!(
            outcome.detail,
            "no confident, non-degenerate shoulder width across the address window"
        );
    }

    #[test]
    fn an_unseen_ear_in_the_impact_window_names_that_window() {
        let mut frames = a_body(50);
        for f in frames.iter_mut().take(43).skip(40) {
            put(f, PoseLandmark::LeftEar, 0.52, 0.20, 0.0);
        }
        let outcome = measure_head_sway(&frames, &a_segmentation());
        assert_eq!(outcome.reason, Some(UnscoredReason::LandmarksUnconfident));
        assert_eq!(
            outcome.detail,
            "no confident ear midpoint frames in the impact window"
        );
    }

    #[test]
    fn a_missing_transition_names_the_phase_travel_was_measured_to() {
        let phases: Vec<_> = a_segmentation()
            .into_iter()
            .filter(|s| s.phase != SwingPhase::Transition)
            .collect();
        let outcome = measure_hip_shift_at_top(&a_body(50), &phases);
        assert_eq!(outcome.reason, Some(UnscoredReason::PhaseNotSegmented));
        assert_eq!(outcome.detail, "no transition phase to measure travel to");
    }

    #[test]
    fn head_hip_gain_names_which_of_its_two_windows_failed() {
        let mut frames = a_body(50);
        for f in frames.iter_mut().take(43).skip(40) {
            put(f, PoseLandmark::LeftHip, 0.55, 0.60, 0.0);
        }
        let outcome = measure_head_hip_gain(&frames, &a_segmentation());
        assert_eq!(outcome.reason, Some(UnscoredReason::LandmarksUnconfident));
        assert_eq!(
            outcome.detail,
            "no confident hip midpoint frames in the impact window"
        );
    }

    // ------------------------------------------------------------------ finish balance

    #[test]
    fn a_still_finish_drifts_nowhere() {
        assert_eq!(
            measure_finish_balance(&a_body(50), &a_segmentation()).value,
            Some(0.0)
        );
    }

    #[test]
    fn the_finish_is_summarized_at_p90_and_not_at_the_maximum() {
        let mut frames = a_body(50);
        // One wild frame at the very end of the follow-through.
        put(&mut frames[49], PoseLandmark::LeftHip, 0.95, 0.60, 1.0);
        put(&mut frames[49], PoseLandmark::RightHip, 0.85, 0.60, 1.0);
        let value = measure_finish_balance(&frames, &a_segmentation())
            .value
            .unwrap();
        // The single outlier sits 0.4 off centre before normalizing; p90 must not reach it.
        assert!(value < 0.4 / 0.30, "p90 {value} was dragged to the outlier");
    }

    #[test]
    fn too_few_confident_hip_frames_says_how_many_there_were() {
        let mut frames = a_body(50);
        for f in frames.iter_mut().skip(43) {
            put(f, PoseLandmark::LeftHip, 0.55, 0.60, 0.0);
        }
        let outcome = measure_finish_balance(&frames, &a_segmentation());
        assert_eq!(outcome.reason, Some(UnscoredReason::TooFewFrames));
        assert_eq!(
            outcome.detail,
            "1 confident hip frames through the follow-through, need 3"
        );
    }

    #[test]
    fn a_clip_that_ends_at_impact_has_no_finish_to_judge() {
        let phases: Vec<_> = a_segmentation()
            .into_iter()
            .filter(|s| s.phase != SwingPhase::FollowThrough)
            .collect();
        let outcome = measure_finish_balance(&a_body(50), &phases);
        assert_eq!(outcome.reason, Some(UnscoredReason::PhaseNotSegmented));
        assert_eq!(
            outcome.detail,
            "no follow-through phase — the clip may end at impact"
        );
    }

    // ------------------------------------------------------------------ the hand family

    #[test]
    fn hand_separation_is_the_mean_per_frame_distance_not_the_distance_of_the_means() {
        let mut frames = a_body(50);
        // Wrists 0.04 apart at address, travelling together — the distance of the means would be
        // unchanged by the travel and so would this, but only one of them is the right reason.
        for (i, f) in frames.iter_mut().enumerate().take(11).skip(5) {
            let shift = i as f64 * 0.01;
            put(f, PoseLandmark::LeftWrist, 0.52 + shift, 0.55, 1.0);
            put(f, PoseLandmark::RightWrist, 0.48 + shift, 0.55, 1.0);
        }
        let value = measure_hand_separation(&frames, &a_segmentation())
            .value
            .unwrap();
        assert!((value - 0.04 / 0.30).abs() < 1e-12, "got {value}");
    }

    #[test]
    fn hand_height_is_positive_when_the_hands_hang_below_the_shoulders() {
        // Shoulders at y = 0.40, wrists at 0.55: 0.15 / 0.30 = 0.5.
        let value = measure_hand_height(&a_body(50), &a_segmentation())
            .value
            .unwrap();
        assert!((value - 0.5).abs() < 1e-12, "got {value}");
    }

    #[test]
    fn hand_offset_from_hips_is_signed_about_image_right() {
        let mut frames = a_body(50);
        for f in frames.iter_mut() {
            put(f, PoseLandmark::LeftWrist, 0.61, 0.55, 1.0);
            put(f, PoseLandmark::RightWrist, 0.57, 0.55, 1.0);
        }
        // Wrist midpoint 0.59, hip centre 0.50: +0.09 / 0.30 = +0.3.
        let value = measure_hand_offset_from_hips(&frames, &a_segmentation())
            .value
            .unwrap();
        assert!((value - 0.3).abs() < 1e-12, "got {value}");
    }

    #[test]
    fn a_hand_metric_with_no_ruler_refuses_on_the_ruler() {
        let mut frames = a_body(50);
        for f in frames.iter_mut() {
            put(f, PoseLandmark::LeftShoulder, 0.505, 0.40, 1.0);
            put(f, PoseLandmark::RightShoulder, 0.495, 0.40, 1.0);
        }
        for outcome in [
            measure_hand_separation(&frames, &a_segmentation()),
            measure_hand_height(&frames, &a_segmentation()),
            measure_hand_offset_from_hips(&frames, &a_segmentation()),
        ] {
            assert_eq!(outcome.reason, Some(UnscoredReason::ScaleUnavailable));
        }
    }

    #[test]
    fn unseen_wrists_are_named_by_each_metric_in_its_own_words() {
        let mut frames = a_body(50);
        for f in frames.iter_mut() {
            put(f, PoseLandmark::LeftWrist, 0.52, 0.55, 0.0);
        }
        assert_eq!(
            measure_hand_separation(&frames, &a_segmentation()).detail,
            "no address-window frames with both wrists confident"
        );
        assert_eq!(
            measure_hand_height(&frames, &a_segmentation()).detail,
            "no confident wrist midpoint frames in the address window"
        );
        assert_eq!(
            measure_hand_offset_from_hips(&frames, &a_segmentation()).detail,
            "no confident wrist midpoint frames in the address window"
        );
    }

    // ------------------------------------------------------------------ trail hand roll

    #[test]
    fn a_hand_pointing_straight_down_reads_zero_degrees() {
        // The trail wrist is at (0.48, 0.55) and its index knuckle at (0.48, 0.60): straight down.
        let value = measure_trail_hand_roll(&a_body(50), &a_segmentation())
            .value
            .unwrap();
        assert_eq!(value, 0.0);
    }

    #[test]
    fn the_angle_is_measured_off_image_vertical_and_positive_is_image_right() {
        let mut frames = a_body(50);
        for f in frames.iter_mut() {
            // 45° to the image-right of straight down.
            put(f, PoseLandmark::RightIndex, 0.53, 0.60, 1.0);
        }
        let value = measure_trail_hand_roll(&frames, &a_segmentation())
            .value
            .unwrap();
        assert!((value - 45.0).abs() < 1e-12, "got {value}");
    }

    /// The circular mean is the point of `direction_series`: two frames either side of the ±180°
    /// wrap must not average to a confident straight-down.
    #[test]
    fn directions_that_cancel_are_refused_rather_than_read_as_zero() {
        let mut frames = a_body(50);
        for (i, f) in frames.iter_mut().enumerate() {
            let y = if i % 2 == 0 { 0.60 } else { 0.50 };
            put(f, PoseLandmark::RightIndex, 0.48, y, 1.0);
        }
        let outcome = measure_trail_hand_roll(&frames, &a_segmentation());
        assert_eq!(outcome.value, None);
        assert_eq!(outcome.reason, Some(UnscoredReason::LandmarksUnconfident));
        assert_eq!(
            outcome.detail,
            "the trail hand's direction did not survive averaging across the address window"
        );
    }

    #[test]
    fn a_collapsed_vector_is_dropped_rather_than_normalized() {
        let mut frames = a_body(50);
        for f in frames.iter_mut() {
            put(f, PoseLandmark::RightIndex, 0.48, 0.55 + 1e-5, 1.0);
        }
        let outcome = measure_trail_hand_roll(&frames, &a_segmentation());
        assert_eq!(outcome.reason, Some(UnscoredReason::LandmarksUnconfident));
        assert_eq!(
            outcome.detail,
            "no address-window frames with a readable trail wrist-to-index-knuckle vector"
        );
    }

    /// An angle is scale-free, so this metric must *not* borrow the ruler the others need.
    #[test]
    fn the_roll_survives_a_clip_with_no_shoulder_ruler() {
        let mut frames = a_body(50);
        for f in frames.iter_mut() {
            put(f, PoseLandmark::LeftShoulder, 0.505, 0.40, 1.0);
            put(f, PoseLandmark::RightShoulder, 0.495, 0.40, 1.0);
        }
        assert_eq!(
            measure_trail_hand_roll(&frames, &a_segmentation()).value,
            Some(0.0)
        );
    }
}
