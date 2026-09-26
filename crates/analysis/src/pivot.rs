//! The swing as three points moving — the pivot paths the rotation checks read. [M22 P5b]
//!
//! The port of `analysis/pivot.py`. [`crate::trajectory`] produces the whole body as one long
//! vector for a fitted model; this produces the three points the turn happens *about* — the
//! shoulder-line midpoint, the hip-line midpoint and the hands — plus the orientation of the two
//! lines through the first two. Same resampling onto event time, same shoulder-width ruler, a much
//! smaller shape. Still measuring and not judging: nothing here reads a band, and
//! [`contracts::pivots`] says why these numbers ship as `Measurement`s rather than as
//! `CheckpointScore`s.
//!
//! # One producer, one signature, two cameras
//!
//! `engine.py` segments the face-on clip only — the down-the-line view never holds a
//! `[PhaseSegment]`, it gets a `SwingAnchors` and nothing more — so a phases-shaped producer would
//! have needed a second copy for the second camera, and two copies of a resampler are two things
//! that drift. Anchors are what both views have, so anchors are what [`pivot_observations`] takes.
//! There is deliberately no `from_phases` variant: the face-on caller converts with
//! [`crate::trajectory::anchors_from_phases`].
//!
//! The output is [`contracts::pivots::PivotObservation`] and not a private type here, which is the
//! whole seam: when the fiducial markers land, a calibrated 3-D source becomes a second producer of
//! the same shape and the rule checks that read it do not change (ADR-029 Decision 3).
//!
//! # Handedness is not an argument, and that is a property to preserve
//!
//! Every check below is unsigned — an excursion magnitude, a roughness, a largest-reversal
//! magnitude — so the mirroring a face-on camera applies to a left-handed swing cancels before it
//! reaches a number. Add a *signed* pivot metric later and that stops being true: handedness then
//! has to be threaded to **both** engine call sites, and the face-on one is the one that will be
//! forgotten, because the down-the-line helper is the one that looks unusual.
//! [`the_checks_are_unsigned_so_a_mirrored_swing_scores_the_same`] pins the property the way
//! `tests/analysis/test_pivot.py` does.

use contracts::keypoints::{FrameKeypoints, PoseLandmark};
use contracts::pivots::{
    backswing_span, downswing_span, FrameOfReference, PivotObservation, PIVOT_SAMPLES,
};
use contracts::unscored::UnscoredReason;

use crate::measure::{MeasureOutcome, MIN_DIRECTION_LENGTH, MIN_SHOULDER_WIDTH, MIN_VISIBILITY};
use crate::phases::{LEAD_WRIST, TRAIL_WRIST};
use crate::trajectory::{interpolate_gaps, sample_positions};

/// The landmark pairs the three pivot points are the midpoints of, in [`PivotObservation`]'s order.
///
/// The hands take *both* wrists rather than a `wrist` argument: both hands are on the club, and the
/// midpoint of the pair is the same point whichever one is the lead, so this is the one place in
/// this crate where the lead/trail distinction genuinely does not matter.
const SHOULDER_PAIR: (PoseLandmark, PoseLandmark) =
    (PoseLandmark::LeftShoulder, PoseLandmark::RightShoulder);
const HIP_PAIR: (PoseLandmark, PoseLandmark) = (PoseLandmark::LeftHip, PoseLandmark::RightHip);
const HAND_PAIR: (PoseLandmark, PoseLandmark) = (LEAD_WRIST, TRAIL_WRIST);

/// One landmark at a fractional frame position, or `None` if either bracketing frame is unsure.
///
/// [`crate::trajectory::build_trajectory`]'s inner `read`, named and returning both axes at once.
///
/// [`crate::measure::midpoint_series`] and [`crate::measure::direction_series`] are the per-frame
/// analogues of what this feeds, and they are deliberately *not* reused: both **drop** an
/// unconfident frame instead of reporting it, which throws away the frame index, and an index is
/// exactly what a resampled path needs. What is reused is the pair of gates they apply —
/// [`MIN_VISIBILITY`] here and [`MIN_DIRECTION_LENGTH`] in [`unit`] — imported rather than
/// restated, so a retune moves both.
fn read(frames: &[FrameKeypoints], position: f64, landmark: PoseLandmark) -> Option<(f64, f64)> {
    let low = (position as usize).min(frames.len() - 1);
    let high = (low + 1).min(frames.len() - 1);
    let frac = position - low as f64;
    let a = frames[low].landmark(landmark);
    let b = frames[high].landmark(landmark);
    if a.visibility.min(b.visibility) < MIN_VISIBILITY {
        return None;
    }
    Some((
        a.x * (1.0 - frac) + b.x * frac,
        a.y * (1.0 - frac) + b.y * frac,
    ))
}

/// Both landmarks of a pair at one sampled position — the midpoint *and* the line need both.
///
/// The same shape serves the per-sample orientation pair below ([`unit`] of the shoulders and of
/// the hips), which is two optional readings for one instant either way.
type Pair = (Option<(f64, f64)>, Option<(f64, f64)>);

fn pair(frames: &[FrameKeypoints], position: f64, of: (PoseLandmark, PoseLandmark)) -> Pair {
    (read(frames, position, of.0), read(frames, position, of.1))
}

/// Midpoint of two read landmarks, or `None` if either was unreadable.
fn midpoint(of: Pair) -> Option<(f64, f64)> {
    let (first, second) = (of.0?, of.1?);
    Some(((first.0 + second.0) / 2.0, (first.1 + second.1) / 2.0))
}

/// Unit vector `first -> second`, or `None` where the two have collapsed.
///
/// The `None` is the honest reading and not a gap to be filled: the projected line collapses at one
/// end of every swing in both views (face-on at the top, down-the-line at address), and a direction
/// taken across a collapsed segment is jitter divided by roughly nothing.
///
/// The vector points left landmark to right landmark, but the **line is undirected**: as the
/// shoulders cross the camera axis through a full turn the vector flips sign. A consumer measuring
/// how far the line turned between two samples must therefore fold the angle onto ±90 degrees
/// rather than reading it as ±180 — a flip is the same line, not a half revolution. [`line_delta_deg`]
/// is where that folding lives.
fn unit(of: Pair) -> Option<(f64, f64)> {
    let (first, second) = (of.0?, of.1?);
    let (dx, dy) = (second.0 - first.0, second.1 - first.1);
    // `hypot`, not `(dx*dx + dy*dy).sqrt()`. `measure.rs` carries the same note: they are different
    // functions on the last bits, and the Python picked this one here. Swapping them survives all
    // 21 vectors — the two agree to within an ulp on every magnitude a normalized landmark pair
    // produces, and the only consumer of `length` divides by it. Kept because the port's job is to
    // match an implementation, not to pick the faster of two that happen to agree today.
    let length = dx.hypot(dy);
    if length < MIN_DIRECTION_LENGTH {
        return None;
    }
    Some((dx / length, dy / length))
}

/// One swing as [`PIVOT_SAMPLES`] pivot observations in event time, or `None` if unmeasurable.
///
/// `frames` are expected **smoothed**. `anchors` are `(address, top, impact)` as fractional frame
/// indices, strictly increasing.
///
/// Three normalisations, and each is load-bearing:
///
/// - **Event time.** [`sample_positions`] spaces the samples evenly between the anchors rather than
///   evenly in clock time, so a slow-motion clip, a real-time clip and the two cameras all land on
///   one index space. [`PIVOT_SAMPLES`] is odd precisely so the top of the backswing lands *on* a
///   sample.
/// - **Origin: the hip centre at address**, one point for the whole swing. Not the hip centre per
///   sample, which is what "hip-relative" means in [`crate::trajectory::build_trajectory`] and
///   would be wrong here: it would put the hip at the origin in every sample and zero out the very
///   travel two of the five checks measure.
/// - **Scale: the median per-sample shoulder width**, `build_trajectory`'s ruler rather than
///   [`crate::measure::shoulder_width`]'s mean. That function is written for the address window;
///   averaged across address-to-impact the same quantity is dragged down by the collapse at the top
///   that [`unit`] returns `None` for, and a ruler that shrinks with the turn inflates every
///   excursion measured against it.
///
/// Returns `None` rather than a partial answer when the swing cannot be read at all: fewer than two
/// frames, anchors that do not increase, anchors reaching outside the clip, no usable shoulder
/// width anywhere, or a **shoulder or hip** midpoint missing more of its timeline than
/// [`crate::trajectory::MAX_MISSING`] allows. A per-sample gap inside those limits is bridged; a
/// per-sample *orientation* is never bridged, and stays `None` for the checks to skip.
///
/// **The hands are gated separately and cannot refuse the swing.** They are the worst-tracked of
/// the three points and the only one nothing measures, so a shared gate let them veto the five
/// metrics that ignore them — [`PivotObservation::hands`] carries what that cost.
///
/// # Panics
///
/// On [`FrameOfReference::Calibrated3d`]. The pose producer cannot declare the one frame of
/// reference in which a rotation number could earn a band, and the caller passes a literal — so a
/// wrong one is a wiring bug and not a property of the footage. Python raises `ValueError` here for
/// the same reason [`contracts::pivots::spec_for`] raises `KeyError`.
pub fn pivot_observations(
    frames: &[FrameKeypoints],
    anchors: (f64, f64, f64),
    frame_of_reference: FrameOfReference,
) -> Option<Vec<PivotObservation>> {
    assert!(
        frame_of_reference != FrameOfReference::Calibrated3d,
        "pivot_observations reads pose in an uncalibrated image plane; it cannot produce {} \
         observations",
        FrameOfReference::Calibrated3d,
    );

    if frames.len() < 2 {
        return None;
    }
    if !(anchors.0 < anchors.1 && anchors.1 < anchors.2) {
        return None;
    }

    let anchors = [anchors.0, anchors.1, anchors.2];
    let positions = sample_positions(&anchors, PIVOT_SAMPLES);
    if positions[0] < 0.0 || *positions.last()? > (frames.len() - 1) as f64 {
        return None;
    }

    let mut shoulders: Vec<Option<(f64, f64)>> = Vec::with_capacity(PIVOT_SAMPLES);
    let mut hips: Vec<Option<(f64, f64)>> = Vec::with_capacity(PIVOT_SAMPLES);
    let mut hands: Vec<Option<(f64, f64)>> = Vec::with_capacity(PIVOT_SAMPLES);
    let mut lines: Vec<Pair> = Vec::with_capacity(PIVOT_SAMPLES);
    let mut widths: Vec<f64> = Vec::new();

    for position in &positions {
        let shoulder_pair = pair(frames, *position, SHOULDER_PAIR);
        let hip_pair = pair(frames, *position, HIP_PAIR);
        shoulders.push(midpoint(shoulder_pair));
        hips.push(midpoint(hip_pair));
        hands.push(midpoint(pair(frames, *position, HAND_PAIR)));
        lines.push((unit(shoulder_pair), unit(hip_pair)));

        if let (Some(left), Some(right)) = shoulder_pair {
            // Horizontal span and not the segment length, matching both rulers this one stands
            // beside (`measure::shoulder_width`, `build_trajectory`): the excursions it normalises
            // are horizontal too, so the frame's pixel aspect cancels in the ratio.
            let width = (left.0 - right.0).abs();
            if width >= MIN_SHOULDER_WIDTH {
                widths.push(width);
            }
        }
    }

    if widths.is_empty() {
        return None;
    }
    widths.sort_by(f64::total_cmp);
    let scale = widths[widths.len() / 2];

    let columns = |points: &[Option<(f64, f64)>]| -> Vec<Vec<Option<f64>>> {
        vec![
            points.iter().map(|p| p.map(|p| p.0)).collect(),
            points.iter().map(|p| p.map(|p| p.1)).collect(),
        ]
    };

    // **Two gates, because the three points do not stand or fall together.** The shoulder and hip
    // midpoints are what every check reads, so a swing that cannot bridge them is not an
    // observation and refuses here. The hands are drawn and measured by nothing, and they are also
    // the worst tracked of the three: M14 P3 put face-on hand tracking at 0.63-0.68 over a whole
    // clip against 1.00 over the address window, and on the corpus 44-61% of the resampled samples
    // had no readable wrist pair while the shoulder and hip midpoints read on every single one.
    // Under one shared gate the hands took the swing down with them — a landmark nothing measures
    // holding a veto over the five metrics that ignore it, which is how M17 P5's first corpus run
    // recorded zero face-on rotation numbers on all fifteen swings.
    let mut both = columns(&shoulders);
    both.extend(columns(&hips));
    let filled = interpolate_gaps(&both)?;
    let (shoulder_x, shoulder_y) = (&filled[0], &filled[1]);
    let (hip_x, hip_y) = (&filled[2], &filled[3]);

    let bridged_hands = interpolate_gaps(&columns(&hands));

    // **Which fixed point this is cancels out of all five checks, and the *fixed* part does not.**
    // Measured by mutation: replacing `[0]` with any other single sample passes all 21 vectors,
    // because `axis_drift` subtracts `observations[0]`'s own x, jitter is a second difference and
    // the reversals read orientations — every one of them is invariant under a constant shift.
    // Making the origin *per-sample*, which is what "hip-relative" means in `build_trajectory`, is
    // caught immediately: it zeroes the hip path and the two drift numbers with it. So the address
    // hip is load-bearing for the overlay and for the first signed metric, and today it is the
    // Python's choice reproduced rather than a number any check can see.
    let (origin_x, origin_y) = (hip_x[0], hip_y[0]);
    let placed =
        |xs: &[f64], ys: &[f64], i: usize| ((xs[i] - origin_x) / scale, (ys[i] - origin_y) / scale);

    Some(
        (0..PIVOT_SAMPLES)
            .map(|i| PivotObservation {
                shoulder: placed(shoulder_x, shoulder_y, i),
                hip: placed(hip_x, hip_y, i),
                hands: bridged_hands
                    .as_ref()
                    .map(|hands| placed(&hands[0], &hands[1], i)),
                shoulder_line: lines[i].0,
                hip_line: lines[i].1,
                frame_of_reference,
                club_head: None,
            })
            .collect(),
    )
}

// ----------------------------------------------------------------------------- the rule checks

/// One pivot measurement: a resampled path in, one number or a reason out.
///
/// **Not [`crate::measure::PoseMeasurement`]'s signature**, which takes `[FrameKeypoints]` and would
/// put the producer back inside every check — five checks each resampling the swing from scratch,
/// and five places for a calibrated source to have to arrive. The engine calls
/// [`pivot_observations`] once per view and hands the same slice to all five (ADR-029 Decision 3).
pub type PivotCheckFn = fn(&[PivotObservation]) -> MeasureOutcome;

/// Fewest observations a path statistic will read. Two for an excursion (one point is not a path);
/// three for the roughness below, which is a second difference and needs a middle sample to bend
/// about.
pub const MIN_PATH_SAMPLES: usize = 2;
pub const MIN_JITTER_SAMPLES: usize = 3;

/// How much of a reversal window may carry no shoulder-line orientation before the check refuses.
///
/// Its own constant at [`crate::trajectory::MAX_MISSING`]'s value rather than a use of it, because
/// the two dials govern different things and only one of them bridges: `MAX_MISSING` is how much of
/// a *position* timeline may be interpolated across, while an orientation is never interpolated at
/// all ([`unit`] returns `None` and the check skips the pair). This is a coverage floor — how much
/// of the window the check must actually have read before it is entitled to call a turn monotone.
/// Neither number is set by evidence from this corpus; they match so there is one habit rather than
/// two.
pub const MAX_UNORIENTED: f64 = 0.40;

/// Panic unless every observation is in one frame of reference.
///
/// Two frames of reference are two instruments, and every check below compares observations to each
/// other. Panicking rather than refusing, per this module's existing rule: a mixed slice cannot come
/// out of [`pivot_observations`], so it is a caller assembling one by hand — a wiring bug, not a
/// property of the footage.
fn one_instrument(observations: &[PivotObservation]) {
    let mut spaces: Vec<FrameOfReference> =
        observations.iter().map(|o| o.frame_of_reference).collect();
    spaces.sort_unstable();
    spaces.dedup();
    assert!(
        spaces.len() <= 1,
        "pivot observations mix frames of reference: {:?}",
        spaces.iter().map(|s| s.as_str()).collect::<Vec<_>>()
    );
}

/// Peak lateral excursion of one centre from its own address `x`, in shoulder widths.
///
/// `x` over `x`, so the frame's pixel aspect cancels — which is what makes the two drift metrics the
/// least calibration-sensitive numbers in the milestone and the reason they lead the registry.
///
/// Measured from `observations[0]`'s own `x` rather than from zero. The pose producer happens to put
/// the address *hip* at the origin, so the hip's address `x` is 0 today; the shoulder's is not, and
/// a calibrated producer need not place either there.
fn axis_drift(
    observations: &[PivotObservation],
    point: fn(&PivotObservation) -> (f64, f64),
    name: &str,
) -> MeasureOutcome {
    one_instrument(observations);
    if observations.len() < MIN_PATH_SAMPLES {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::TooFewFrames,
            format!(
                "{} pivot samples; the {name} path needs {MIN_PATH_SAMPLES}",
                observations.len()
            ),
        );
    }
    let address_x = point(&observations[0]).0;
    // `max` over a non-empty slice of finite magnitudes. Python's `max` and Rust's `fold` agree on
    // the *value* here whatever the tie-break — P4's `max`-tie-break edge bites when the winning
    // element's identity matters, and here only the number leaves.
    let peak = observations
        .iter()
        .map(|o| (point(o).0 - address_x).abs())
        .fold(f64::NEG_INFINITY, f64::max);
    MeasureOutcome::measured(peak)
}

/// How far the hip centre slid off the axis it started on. The best-tracked of the three.
pub fn hip_axis_drift(observations: &[PivotObservation]) -> MeasureOutcome {
    axis_drift(observations, |o| o.hip, "hip centre")
}

/// The same excursion for the shoulder centre.
pub fn shoulder_axis_drift(observations: &[PivotObservation]) -> MeasureOutcome {
    axis_drift(observations, |o| o.shoulder, "shoulder centre")
}

/// Sample-to-sample roughness of the hip centre's path, in shoulder widths.
///
/// The **second** difference — `|p[i-1] - 2p[i] + p[i+1]|` — and not the first. A first difference is
/// speed, and a hip that travels smoothly through the swing has plenty of it; what this asks is how
/// much the path changes direction or speed between one sample and the next, so a straight steady
/// slide reads 0 no matter how far it goes. That is also what keeps this independent of
/// [`hip_axis_drift`]: the two would otherwise measure one thing twice.
///
/// Averaged rather than maximised, for [`crate::measure`]'s finish-drift reason: `max` is an
/// extreme-value statistic and one bad sample would become the whole number, on a path whose samples
/// are interpolated from an occlusion-prone landmark.
///
/// **Roughness in event time, and it has a floor at the top.** The samples are evenly spaced between
/// the anchors and not in clock time, so the backswing and the downswing are read at different
/// frames per sample — which means a hip sliding at one constant speed *in frames* changes speed in
/// samples exactly at the top, and reads a small non-zero bend there. Measured on the fixture it is
/// around 3e-4 against a real shake's ~3e-2, so it is a floor rather than a signal; it is not
/// subtracted out and the pair spanning the top is not skipped, because the transition is where a
/// genuine hip jerk would live. It also means the number is tempo-sensitive in a way none of the
/// other four are, which matters the day a band is cut from it.
///
/// Only the hip. The shoulders collapse at one end of every swing in both views and the hands are
/// tracked at 0.63-0.68 face-on (M14 P3), so a roughness on either would report the tracker's noise
/// as the golfer's.
pub fn hip_path_jitter(observations: &[PivotObservation]) -> MeasureOutcome {
    one_instrument(observations);
    if observations.len() < MIN_JITTER_SAMPLES {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::TooFewFrames,
            format!(
                "{} pivot samples; roughness needs {MIN_JITTER_SAMPLES}",
                observations.len()
            ),
        );
    }
    let bends: Vec<f64> = observations
        .windows(3)
        .map(|w| {
            let (first, middle, last) = (w[0].hip, w[1].hip, w[2].hip);
            (first.0 - 2.0 * middle.0 + last.0).hypot(first.1 - 2.0 * middle.1 + last.1)
        })
        .collect();
    MeasureOutcome::measured(bends.iter().sum::<f64>() / bends.len() as f64)
}

/// Signed turn from one shoulder line to the next in degrees, folded onto ±90, or `None`.
///
/// Folded because the stored vector is **directed** and the line it describes is not: [`unit`]
/// points left landmark to right landmark, and that vector flips sign as the shoulders cross the
/// camera axis. Read as ±180 a flip would be a half revolution between two adjacent samples — the
/// largest reversal imaginable, produced by the fullest turns. Folded, it is what it actually is:
/// the same line, and a small move.
///
/// `rem_euclid` rather than `%`, which is the one place the two languages differ on this line:
/// Python's `%` takes the sign of the divisor and Rust's takes the sign of the dividend, so a
/// negative angle lands 180 degrees away under a literal transcription.
fn line_delta_deg(first: Option<(f64, f64)>, second: Option<(f64, f64)>) -> Option<f64> {
    let (first, second) = (first?, second?);
    let cross = first.0 * second.1 - first.1 * second.0;
    let dot = first.0 * second.0 + first.1 * second.1;
    Some((cross.atan2(dot).to_degrees() + 90.0).rem_euclid(180.0) - 90.0)
}

/// Largest against-the-turn move of the shoulder line inside one half of the swing.
///
/// Zero is a monotone turn. The direction the turn is going is taken from the **net** move across
/// the window rather than assumed, which is what lets one implementation serve a backswing and a
/// downswing that turn opposite ways — and, with the folding above, a left-handed golfer whose every
/// sign is mirrored. A window with no net turn at all takes the positive direction, so a line that
/// went out and came back still reports the larger of the two halves rather than 0.
///
/// Only *adjacent* readable samples are compared. Skipping a `None` and pairing across the gap would
/// charge one interval with the whole turn that happened during the collapse, and the collapse sits
/// at the top face-on and at address down-the-line — so the check would fire hardest on the fullest
/// turns, which is exactly the failure mode ADR-029's addendum §4 raised against reading degrees off
/// a projected line in the first place.
fn shoulder_reversal(
    observations: &[PivotObservation],
    span: std::ops::Range<usize>,
    window: &str,
) -> MeasureOutcome {
    one_instrument(observations);
    if observations.len() < PIVOT_SAMPLES {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::TooFewFrames,
            format!(
                "{} pivot samples; the {window} span indexes {PIVOT_SAMPLES}",
                observations.len()
            ),
        );
    }

    let width = span.len();
    let unoriented = span
        .clone()
        .filter(|i| observations[*i].shoulder_line.is_none())
        .count();
    let deltas: Vec<f64> = span
        .take(width - 1)
        .filter_map(|i| {
            line_delta_deg(
                observations[i].shoulder_line,
                observations[i + 1].shoulder_line,
            )
        })
        .collect();
    if deltas.is_empty() || unoriented as f64 / width as f64 > MAX_UNORIENTED {
        return MeasureOutcome::unmeasurable(
            UnscoredReason::LandmarksUnconfident,
            format!("the shoulder line collapsed in {unoriented} of {width} {window} samples"),
        );
    }

    let direction = if deltas.iter().sum::<f64>() >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let worst = deltas
        .iter()
        .map(|delta| -direction * delta)
        .fold(f64::NEG_INFINITY, f64::max);
    MeasureOutcome::measured(0.0_f64.max(worst))
}

/// Address through the top, inclusive of both.
pub fn shoulder_reversal_backswing(observations: &[PivotObservation]) -> MeasureOutcome {
    shoulder_reversal(observations, backswing_span(), "backswing")
}

/// The top through impact. The top belongs to both spans — a reversal is a move *between* samples,
/// so the pair spanning it is the backswing's last and the downswing's first.
pub fn shoulder_reversal_downswing(observations: &[PivotObservation]) -> MeasureOutcome {
    shoulder_reversal(observations, downswing_span(), "downswing")
}

/// Check name -> implementation, keyed by `PivotMeasurementSpec.check` and **never** by metric name.
///
/// Five checks, ten metrics: a face-on spec and its `_dtl` partner name the same check. The view
/// decides the measurement's name and the check does not know which camera it is reading, which is
/// what lets one implementation serve two views without string surgery on the `_dtl` suffix — and
/// what a calibrated third producer will inherit for free.
///
/// A slice rather than a map, matching [`crate::checkpoints::CHECKPOINT_EVALUATORS`]: five entries,
/// looked up once per emitted row, and a `HashMap` would buy nothing but non-determinism.
pub static PIVOT_CHECKS: &[(&str, PivotCheckFn)] = &[
    ("hip_axis_drift", hip_axis_drift),
    ("shoulder_axis_drift", shoulder_axis_drift),
    ("hip_path_jitter", hip_path_jitter),
    ("shoulder_reversal_backswing", shoulder_reversal_backswing),
    ("shoulder_reversal_downswing", shoulder_reversal_downswing),
];

/// The check registered under `name`.
///
/// Panics on a miss, matching [`crate::checkpoints::evaluator_for`]: the name comes off
/// [`contracts::pivots::PIVOT_MEASUREMENT_REGISTRY`], so a miss is the registry and this table
/// having drifted apart — which [`every_registered_check_has_an_implementation`] is what catches.
pub fn check_for(name: &str) -> PivotCheckFn {
    PIVOT_CHECKS
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, check)| *check)
        .unwrap_or_else(|| panic!("no pivot check registered under {name:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::pivots::PIVOT_MEASUREMENT_REGISTRY;

    const FACE_ON_SPACE: FrameOfReference = FrameOfReference::ImagePlaneFaceOn;

    /// A path of `PIVOT_SAMPLES` observations, hips walking a straight line and shoulders still.
    fn path(hip: impl Fn(usize) -> (f64, f64)) -> Vec<PivotObservation> {
        (0..PIVOT_SAMPLES)
            .map(|i| PivotObservation {
                shoulder: (0.0, -1.0),
                hip: hip(i),
                hands: None,
                shoulder_line: Some((1.0, 0.0)),
                hip_line: Some((1.0, 0.0)),
                frame_of_reference: FACE_ON_SPACE,
                club_head: None,
            })
            .collect()
    }

    /// A shoulder line turning by `per_step` degrees each sample, with `overrides` replacing the
    /// turn at the named steps — the shape both reversal checks read.
    fn turning(per_step: f64, overrides: &[(usize, Option<f64>)]) -> Vec<PivotObservation> {
        let mut angle = 0.0_f64;
        let mut out = Vec::with_capacity(PIVOT_SAMPLES);
        for i in 0..PIVOT_SAMPLES {
            let line = match overrides.iter().find(|(at, _)| *at == i) {
                Some((_, None)) => None,
                Some((_, Some(forced))) => {
                    angle = *forced;
                    Some((angle.to_radians().cos(), angle.to_radians().sin()))
                }
                None => Some((angle.to_radians().cos(), angle.to_radians().sin())),
            };
            out.push(PivotObservation {
                shoulder: (0.0, -1.0),
                hip: (0.0, 0.0),
                hands: None,
                shoulder_line: line,
                hip_line: None,
                frame_of_reference: FACE_ON_SPACE,
                club_head: None,
            });
            angle += per_step;
        }
        out
    }

    #[test]
    fn every_registered_check_has_an_implementation_and_every_check_is_named() {
        for spec in PIVOT_MEASUREMENT_REGISTRY {
            check_for(spec.check);
        }
        for (name, _) in PIVOT_CHECKS {
            assert!(
                PIVOT_MEASUREMENT_REGISTRY
                    .iter()
                    .any(|spec| spec.check == *name),
                "{name} is implemented and nothing names it"
            );
        }
    }

    #[test]
    #[should_panic(expected = "no pivot check registered under \"spine_tilt\"")]
    fn an_unregistered_check_panics() {
        check_for("spine_tilt");
    }

    /// The excursion is measured from the *address* x and not from zero, which is the difference
    /// between reading the shoulder centre and reading the hip centre the producer parks at 0.
    #[test]
    fn the_drift_is_measured_from_the_address_sample() {
        let observations = path(|i| (0.5 + i as f64 * 0.01, 0.0));
        let measured = hip_axis_drift(&observations).value.unwrap();
        assert!(
            (measured - 0.40).abs() < 1e-12,
            "40 steps of 0.01, not 0.90 from zero: {measured}"
        );
    }

    #[test]
    fn the_drift_is_unsigned_so_a_leftward_slide_reads_the_same() {
        let right = hip_axis_drift(&path(|i| (i as f64 * 0.01, 0.0)));
        let left = hip_axis_drift(&path(|i| (-(i as f64) * 0.01, 0.0)));
        assert_eq!(right.value, left.value);
    }

    /// A straight steady slide has plenty of speed and no roughness. This is the property that
    /// keeps jitter independent of drift — measure one thing twice and a band cut from either is
    /// cut from both.
    #[test]
    fn a_straight_steady_slide_is_perfectly_smooth() {
        let measured = hip_path_jitter(&path(|i| (i as f64 * 0.02, i as f64 * 0.01)))
            .value
            .unwrap();
        assert!(measured < 1e-12, "{measured}");
    }

    #[test]
    fn a_shake_in_the_hip_path_is_roughness() {
        let shaken = path(|i| ((i % 2) as f64 * 0.03, 0.0));
        assert!(hip_path_jitter(&shaken).value.unwrap() > 0.05);
    }

    #[test]
    fn a_path_shorter_than_three_samples_refuses_roughness_by_name() {
        let two: Vec<PivotObservation> = path(|_| (0.0, 0.0)).into_iter().take(2).collect();
        let refused = hip_path_jitter(&two);
        assert_eq!(refused.reason, Some(UnscoredReason::TooFewFrames));
        assert_eq!(refused.detail, "2 pivot samples; roughness needs 3");
        let one: Vec<PivotObservation> = two.into_iter().take(1).collect();
        assert_eq!(
            hip_axis_drift(&one).detail,
            "1 pivot samples; the hip centre path needs 2"
        );
        assert_eq!(
            shoulder_axis_drift(&one).detail,
            "1 pivot samples; the shoulder centre path needs 2"
        );
    }

    /// A monotone turn reads 0 in whichever direction it goes — the net move decides the sign, so
    /// one implementation serves a backswing and a downswing that turn opposite ways.
    #[test]
    fn a_monotone_turn_reads_zero_in_either_direction() {
        for per_step in [1.5, -1.5] {
            let observations = turning(per_step, &[]);
            assert_eq!(
                shoulder_reversal_backswing(&observations).value,
                Some(0.0),
                "per_step={per_step}"
            );
            assert_eq!(
                shoulder_reversal_downswing(&observations).value,
                Some(0.0),
                "per_step={per_step}"
            );
        }
    }

    /// One sample that goes back against the turn is the whole measurement, and it is a magnitude.
    #[test]
    fn a_step_against_the_turn_is_the_reported_reversal() {
        // Ten steps of 1.5 degrees, then a jump back to 8 at sample 11 — a -7 degree move.
        let observations = turning(1.5, &[(11, Some(8.0))]);
        let measured = shoulder_reversal_backswing(&observations).value.unwrap();
        assert!((measured - 7.0).abs() < 1e-9, "{measured}");
        // ... and it is in the backswing span only.
        assert_eq!(shoulder_reversal_downswing(&observations).value, Some(0.0));
    }

    /// The top belongs to both spans, so a reversal *at* the top is charged to both halves. That is
    /// the contract `BACKSWING_SPAN`/`DOWNSWING_SPAN` overlapping by one sample buys.
    #[test]
    fn a_reversal_at_the_top_is_seen_by_both_windows() {
        let top = PIVOT_SAMPLES / 2;
        let observations = turning(1.5, &[(top + 1, Some(top as f64 * 1.5 - 5.0))]);
        assert!(shoulder_reversal_downswing(&observations).value.unwrap() > 4.0);
        assert_eq!(shoulder_reversal_backswing(&observations).value, Some(0.0));
    }

    /// **The folding is the point.** A shoulder line crossing the camera axis flips the stored unit
    /// vector's sign, and read as ±180 that flip is a half revolution between adjacent samples — the
    /// largest reversal imaginable, produced by the fullest turns. Folded onto ±90 it is the small
    /// move it actually is. A port that dropped the fold passes every monotone case above and fails
    /// here.
    #[test]
    fn a_line_that_flips_sign_is_the_same_line_and_a_small_move() {
        let delta = line_delta_deg(Some((1.0, 0.0)), Some((-0.9962, -0.0872))).unwrap();
        assert!(delta.abs() < 10.0, "a 5-degree move read as {delta}");
    }

    /// `%` in Python takes the sign of the divisor and in Rust the sign of the dividend, so a
    /// literal transcription lands a negative turn 180 degrees away. Pinned in both directions.
    #[test]
    fn the_fold_is_symmetric_about_zero() {
        let up = line_delta_deg(Some((1.0, 0.0)), Some((0.9962, 0.0872))).unwrap();
        let down = line_delta_deg(Some((1.0, 0.0)), Some((0.9962, -0.0872))).unwrap();
        assert!((up + down).abs() < 1e-9, "up={up} down={down}");
        assert!(up > 0.0 && down < 0.0);
    }

    #[test]
    fn an_absent_orientation_yields_no_delta() {
        assert_eq!(line_delta_deg(None, Some((1.0, 0.0))), None);
        assert_eq!(line_delta_deg(Some((1.0, 0.0)), None), None);
    }

    /// A window that collapsed past the coverage floor refuses rather than calling the turn
    /// monotone off the handful of samples it did read.
    #[test]
    fn a_window_past_the_coverage_floor_refuses_with_the_count_in_the_sentence() {
        let missing: Vec<(usize, Option<f64>)> = (0..12).map(|i| (i, None)).collect();
        let refused = shoulder_reversal_backswing(&turning(1.5, &missing));
        assert_eq!(refused.reason, Some(UnscoredReason::LandmarksUnconfident));
        assert_eq!(
            refused.detail,
            "the shoulder line collapsed in 12 of 21 backswing samples"
        );
    }

    /// Exactly at the floor is measured, one sample past it refuses: the gate is `>`, not `>=`.
    #[test]
    fn the_coverage_floor_is_a_strict_comparison() {
        let eight: Vec<(usize, Option<f64>)> = (0..8).map(|i| (i, None)).collect();
        assert!(shoulder_reversal_backswing(&turning(1.5, &eight))
            .value
            .is_some());
        let nine: Vec<(usize, Option<f64>)> = (0..9).map(|i| (i, None)).collect();
        assert!(shoulder_reversal_backswing(&turning(1.5, &nine))
            .value
            .is_none());
    }

    /// A path too short to index the spans refuses by name rather than panicking on the slice.
    #[test]
    fn a_short_path_refuses_both_reversal_windows() {
        let short: Vec<PivotObservation> = turning(1.5, &[]).into_iter().take(10).collect();
        for refused in [
            shoulder_reversal_backswing(&short),
            shoulder_reversal_downswing(&short),
        ] {
            assert_eq!(refused.reason, Some(UnscoredReason::TooFewFrames));
            assert!(refused.detail.contains("10 pivot samples"), "{refused:?}");
        }
    }

    /// **Every check is unsigned, and this is the property that lets handedness stay out of the
    /// signature.** Mirror the swing — negate every `x` and every orientation's `x` — and all five
    /// answers are identical. The day a signed pivot metric lands, this test is what fails.
    #[test]
    fn the_checks_are_unsigned_so_a_mirrored_swing_scores_the_same() {
        let observations = turning(1.5, &[(11, Some(8.0))]);
        let mirrored: Vec<PivotObservation> = observations
            .iter()
            .map(|o| PivotObservation {
                shoulder: (-o.shoulder.0, o.shoulder.1),
                hip: (-o.hip.0, o.hip.1),
                hands: o.hands.map(|h| (-h.0, h.1)),
                shoulder_line: o.shoulder_line.map(|l| (-l.0, l.1)),
                hip_line: o.hip_line.map(|l| (-l.0, l.1)),
                ..*o
            })
            .collect();
        for (_, check) in PIVOT_CHECKS {
            assert_eq!(
                check(&observations).value,
                check(&mirrored).value,
                "a pivot check gained a sign"
            );
        }
    }

    #[test]
    #[should_panic(expected = "mix frames of reference")]
    fn two_instruments_in_one_list_is_a_wiring_bug() {
        let mut observations = path(|_| (0.0, 0.0));
        observations[3].frame_of_reference = FrameOfReference::ImagePlaneDtl;
        hip_axis_drift(&observations);
    }

    #[test]
    #[should_panic(expected = "cannot produce calibrated_3d observations")]
    fn the_pose_producer_refuses_to_claim_a_calibrated_space() {
        pivot_observations(&[], (0.0, 1.0, 2.0), FrameOfReference::Calibrated3d);
    }
}
