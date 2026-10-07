//! `analysis/phases.py`. Swing phase segmentation. [M22 P4]
//!
//! Splits a [`FrameKeypoints`] timeline into the six [`SwingPhase`] spans by tracking the **lead
//! wrist** vertical position in a face-on view. In image coordinates `y` grows *downward*, so
//! through the swing the wrist traces: rest (high `y`) -> rises through the backswing (`y` falls to
//! a minimum at the top) -> falls back through the downswing (`y` climbs) -> impact near address
//! height -> follow-through (rises again).
//!
//! Three instants are located: **motion start**, **top of backswing** and **impact**. Top and impact
//! are found together, as the two ends of the downswing — the longest sustained stretch of the hands
//! coming down — and the rule takes the **earliest** such run within a fraction of the size of the
//! largest, because a full finish leaves the hands higher than the top and nothing in a golf swing
//! descends before the downswing does. The measurements behind every constant here are in the
//! Python module's comments and in `docs/M4_POSE_BAKEOFF.md`; they are pointed at rather than
//! copied, per `CLAUDE.md`.
//!
//! # What this port has to get right that the Python never had to think about
//!
//! **Python's `max` returns the first maximum and Rust's [`Iterator::max_by`] returns the last.**
//! `_top_and_impact`'s no-runs fallback is `max(range(top, n), key=ys.__getitem__)`, and on a clip
//! whose wrist sits at the same `y` for two frames those two answers are different impact frames —
//! which is ADR-032 §3's frame-index edge arriving from a direction that section does not name.
//! [`first_argmax`] is that asymmetry solved once; the minimum needed no help, because both
//! languages take the first one.
//!
//! **`round` is half-to-even.** Two sites here round a frame count — the stall length and the tempo
//! fallback — and both go through [`pyfmt::round_index`].
//!
//! # What is not ported yet, and why
//!
//! `candidate_downswings`, `select_swing`, `select_matching_swing` and `window_around` are the
//! clip-*choosing* half of the Python module, and nothing `scripts/conformance.py::run_vector`
//! reaches calls them: `api/pipeline.py` does, which ADR-032 §8 leaves in Python for M22, and
//! `alignment.py` reaches only [`segment_phases`] and three constants. They have no committed gate
//! in `spec/vectors/stages/`, so under the rule M22 P2 set for the `contracts/` registries they land
//! with a phase that can prove them — P7 for the constants alignment reads, and §M29 for the lab's.

use contracts::keypoints::{FrameKeypoints, PoseLandmark};
use contracts::swing::{PhaseSegment, SwingPhase};

use pyfmt::round_index;

/// The wrist to track from a face-on camera. Measured, not assumed: over 1,045 labelled GolfDB
/// clips the rule does better on the lead wrist face-on (9%/7% failure against 17%/10%) even though
/// the trail wrist is tracked in more frames.
pub const LEAD_WRIST: PoseLandmark = PoseLandmark::LeftWrist;

/// The wrist to track from down-the-line, where the lead wrist is the **far** arm, occluded by the
/// torso through the top and tracked in only 39% of frames. Public for the same reason its Python
/// twin is: a pair where only one half can be said by name is a pair whose other half gets written
/// as a literal landmark index somewhere this module cannot see it drift.
pub const TRAIL_WRIST: PoseLandmark = PoseLandmark::RightWrist;

/// Frames whose tracked-wrist visibility is below this are unreliable; the last good `y` is held
/// rather than a low-confidence jump trusted (MediaPipe convention).
const MIN_VISIBILITY: f64 = 0.5;

/// A rising run tolerates a dip of this fraction of the rise it has already accumulated before it is
/// considered over. A real downswing is not perfectly monotone in a 2D track.
const DRAWDOWN_TOLERANCE: f64 = 0.25;

/// ...but a fraction of the run's *own accumulated* rise is near zero at the start of a run, so the
/// drawdown must also clear an absolute floor, expressed as a fraction of the clip's own vertical
/// wrist range so it stays resolution-, framing- and fps-invariant (ADR-013). 0.012 is the largest
/// floor the 461-clip sweep leaves the long-tail count unmoved at.
const DRAWDOWN_FLOOR: f64 = 0.012;

/// A rising run counts as a candidate downswing at this fraction of the largest rise in the clip.
/// The centre of a plateau across 0.75-0.85, not an argmin.
const MAJOR_RISE_FRACTION: f64 = 0.80;

/// Motion start is velocity-anchored: the takeaway begins just after the last sustained *quiet*
/// stretch, a run of frames slower than this fraction of the swing's peak wrist speed.
const MOTION_QUIET_FRAC: f64 = 0.05;

/// How long that quiet run must be, as a fraction of `(impact - top)` — the clip's own time base
/// rather than a frame count, which is ADR-013 and the difference between reading a 240fps phone
/// clip and a broadcast slow-motion replay the same way.
const MOTION_STALL_FRACTION: f64 = 0.25;

/// Floor for the quiet run. Two frames is the shortest stretch that can distinguish a dwell from a
/// single smoothed sample; it binds only on very short real-time clips.
const MOTION_STALL_MIN_FRAMES: i64 = 2;

/// When the wrist never settles there is no quiet run to find, and frame 0 is not a neutral answer —
/// GolfDB clips carry a median 59 frames of pre-roll. Falling back to the tour-median tempo ratio
/// bounds the damage. GolfDB's own median over 1,399 clips (ADR-012), not a book number.
///
/// The two segments this boundary defines are marked `detected = false` when it fires, because the
/// estimate is circular for anything that divides by it: tempo would report ~3.5:1 by construction.
pub const FALLBACK_TEMPO_RATIO: f64 = 3.5;

/// How far from the pose-estimated impact a **heard** ball strike may sit and still be taken as a
/// correction of it, in seconds. [M22 P6]
///
/// Sized from the measurements in `docs/M11_ACOUSTIC_SYNC.md` that bound the error: ±0.125 s is the
/// worst pose-impact error the corpus holds, +0.145 s is the ball-to-screen gap (so the *nearest*
/// transient to a correct impact may be the screen strike rather than the ball), and +0.018 s is the
/// detector's own onset convention. 0.20 s clears the worst of those with room to spare, and nothing
/// it must reject is anywhere near it — the post-impact descents the rule excludes sit 15-24 s past
/// the swing. There is no value between "covers the measurement error" and "admits a decoy" on this
/// corpus.
///
/// **Rejected: an asymmetric window**, on the grounds that a crack cannot precede the contact. That
/// is true of the physics and wrong about the *measurement*: four bundles have the pose impact
/// landing ~0.1 s after the audio, and an asymmetric rule would refuse exactly the ones M11 exists to
/// repair.
///
/// `pub(crate)` and here rather than in [`crate::alignment`] because this is its Python home, and
/// module names carry over one for one (ADR-032 §1). It has **two** Python readers and only one of
/// them is ported: `alignment::with_measured_impact`, and `phases._struck`, which belongs to the
/// clip-*choosing* half P4 left in Python because only `api/pipeline.py` and `scripts/` reach it.
pub(crate) const STRIKE_TOLERANCE_S: f64 = 0.20;

/// The band a real downswing lasts, in seconds — `(low, high)`. [M22 P7]
///
/// Expressed in seconds, the one rule in this module that is (ADR-013): a downswing lasts about the
/// same time for every golfer at every frame rate, which is what makes an absolute bound meaningful
/// here where the rest of the module works in the clip's own time base.
///
/// **Its one ported reader decides nothing with it.** `alignment::shared_tops` reads it to *warn*
/// that the shared top it is about to impose is outside anything a golfer does — it used to veto the
/// correction, and that veto is what left four bundles replaying a panel at up to 3.11x
/// (M11 P7). Python's other two readers are the clip-*choosing* half P4 left behind
/// (`candidate_downswings`, `select_swing`) and the tempo trainer, neither of which is ported.
pub(crate) const POSSIBLE_DOWNSWING_S: (f64, f64) = (0.12, 0.80);

/// Half-widths (in frames) of the transition window straddling the top and of the impact window
/// straddling the return to address height. Small, symmetric, heuristic.
const TRANSITION_HALF_FRAMES: i64 = 3;
const IMPACT_HALF_FRAMES: i64 = 2;

/// Below this many frames there is no swing to segment.
const MIN_FRAMES: usize = 6;

/// Python's `max(range(...), key=...)`, which returns the **first** maximum.
///
/// [`Iterator::max_by`] returns the last one, so this is not a style preference: on a clip whose
/// wrist `y` repeats, the two disagree about which frame impact is, and every measurement after it
/// inherits the difference. `None` on an empty range, which the one caller cannot hand it.
fn first_argmax(values: &[f64], from: usize) -> Option<usize> {
    let mut best: Option<usize> = None;
    for i in from..values.len() {
        if best.is_none_or(|b| values[i] > values[b]) {
            best = Some(i);
        }
    }
    best
}

/// Python's `min(range(n), key=...)`, which returns the first minimum — as does `min_by`, so this
/// exists for symmetry with [`first_argmax`] and to keep the tie rule visible at both call sites.
fn first_argmin(values: &[f64]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for i in 0..values.len() {
        if best.is_none_or(|b| values[i] < values[b]) {
            best = Some(i);
        }
    }
    best
}

/// Tracked-wrist `(x, y)` per frame, holding the last confident value through dim frames.
fn wrist_xy(keypoints: &[FrameKeypoints], wrist: PoseLandmark) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(keypoints.len());
    let mut last_good: Option<(f64, f64)> = None;
    for frame in keypoints {
        let lm = frame.landmark(wrist);
        if lm.visibility >= MIN_VISIBILITY || last_good.is_none() {
            last_good = Some((lm.x, lm.y));
        }
        points.push(last_good.expect("the first frame always sets a last-good position"));
    }
    points
}

/// Per-frame mask: was the tracked wrist actually seen, or is [`wrist_xy`] holding?
///
/// Held frames are excluded from run detection so a stretch of lost tracking cannot bound a descent
/// — worth about 1.5 frames of mean top error on the GolfDB face-on set. It masks with the
/// **tracked** wrist's own visibility, which the Python did not always do: it read the lead wrist in
/// its body while taking a `wrist` argument, so a trail-wrist caller got trail positions masked by
/// the visibility of the arm that wrist was chosen to avoid.
fn wrist_confident(keypoints: &[FrameKeypoints], wrist: PoseLandmark) -> Vec<bool> {
    keypoints
        .iter()
        .map(|f| f.landmark(wrist).visibility >= MIN_VISIBILITY)
        .collect()
}

/// One near-monotone stretch of *falling hands*: `(rise, start_frame, end_frame)`.
type Run = (f64, usize, usize);

/// Near-monotone stretches of falling hands. `y` grows downward, so a rising `y` is the hands
/// coming down — a descent of the club.
///
/// Each run ends when `y` gives back more than [`DRAWDOWN_TOLERANCE`] of the rise it has accumulated
/// **and** more than [`DRAWDOWN_FLOOR`] of the clip's whole wrist range. Only confidently-tracked
/// frames participate.
fn rising_runs(ys: &[f64], confident: &[bool]) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut start: Option<usize> = None;
    let mut peak_at = 0usize;
    let mut peak = 0.0f64;

    let tracked: Vec<f64> = ys
        .iter()
        .zip(confident)
        .filter(|(_, &ok)| ok)
        .map(|(&y, _)| y)
        .collect();
    let floor = if tracked.is_empty() {
        0.0
    } else {
        let hi = tracked.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let lo = tracked.iter().copied().fold(f64::INFINITY, f64::min);
        DRAWDOWN_FLOOR * (hi - lo)
    };

    for (index, &y) in ys.iter().enumerate() {
        if !confident[index] {
            continue;
        }
        let Some(open) = start else {
            start = Some(index);
            peak = y;
            peak_at = index;
            continue;
        };
        if y >= peak {
            peak = y;
            peak_at = index;
            continue;
        }

        let rise = peak - ys[open];
        if rise > 0.0 && (peak - y) > (DRAWDOWN_TOLERANCE * rise).max(floor) {
            runs.push((rise, open, peak_at));
            start = Some(index);
            peak = y;
            peak_at = index;
        } else if y < ys[open] {
            // Still descending toward a lower turning point — restart from here.
            start = Some(index);
            peak = y;
            peak_at = index;
        }
    }

    if let Some(open) = start {
        if peak > ys[open] {
            runs.push((peak - ys[open], open, peak_at));
        }
    }
    runs
}

/// The top of the backswing and impact, as the two ends of the downswing.
///
/// Takes the **earliest** rising run within [`MAJOR_RISE_FRACTION`] of the largest, rather than the
/// largest outright. That one word is what makes this correct on real swings: a full finish puts
/// the hands higher than they were at the top and degraded tracking after it adds spurious
/// excursions, either of which can rival the true downswing — but neither can happen *before* it.
/// Ordering is the one piece of structure every golf swing has, and unlike a threshold it needs no
/// calibrating per camera, per player or per fps.
///
/// Falls back to the global argmin only when no run is found at all, where any answer is a guess.
fn top_and_impact(ys: &[f64], confident: &[bool], n: usize) -> (usize, usize) {
    let runs = rising_runs(ys, confident);
    if runs.is_empty() {
        let top = first_argmin(&ys[..n]).expect("a clip past MIN_FRAMES has frames");
        let impact = first_argmax(&ys[..n], top).expect("top is in range");
        return (top, impact);
    }

    let largest = runs
        .iter()
        .map(|&(rise, _, _)| rise)
        .fold(f64::NEG_INFINITY, f64::max);
    // `min(major, key=lambda run: run[1])` — the earliest start. Both languages take the first
    // minimum, and the starts are distinct anyway.
    let (_, top, impact) = runs
        .iter()
        .filter(|&&(rise, _, _)| rise >= MAJOR_RISE_FRACTION * largest)
        .min_by_key(|&&(_, start, _)| start)
        .copied()
        .expect("the largest run always clears its own fraction");
    (top, impact)
}

/// Per-frame 2D wrist speed (frame-to-frame displacement); `0.0` at the first frame.
fn wrist_speed(xy: &[(f64, f64)]) -> Vec<f64> {
    let mut speeds = vec![0.0];
    for window in xy.windows(2) {
        let ((x0, y0), (x1, y1)) = (window[0], window[1]);
        // `((dx)**2 + (dy)**2) ** 0.5` in the Python, not `math.hypot` — and the two are not the
        // same function. `hypot` is scaled to avoid intermediate overflow and is correctly rounded;
        // the naive form is a multiply, an add and a sqrt. On coordinates in `[0, 1]` neither
        // overflows, but they can land on different last bits, so this keeps the Python's. Where
        // the Python *does* call `math.hypot` — `measure.direction_series` — the port calls
        // `f64::hypot`.
        speeds.push(((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt());
    }
    speeds
}

/// Motion start: the frame the sustained takeaway begins, walking back from the top.
///
/// Returns `(frame, detected)`. `detected` is false when the wrist never settles and the frame is
/// the bounded estimate at [`FALLBACK_TEMPO_RATIO`] rather than something found in the signal —
/// callers that divide by this boundary must drop their result instead of using it.
///
/// Speed rather than wrist *height*, because the early takeaway is near-horizontal and a height rule
/// misses it; a *run* rather than a single frame, so one slow smoothed frame mid-takeaway cannot end
/// it early.
fn motion_start(xy: &[(f64, f64)], top: usize, impact: usize) -> (usize, bool) {
    if top == 0 {
        return (0, true);
    }
    let speeds = wrist_speed(xy);
    // `max(speeds[1 : top + 1], default=0.0)`. A value, not a position, so the tie rule is moot.
    let peak = speeds[1..=top].iter().copied().fold(0.0f64, f64::max);
    if peak <= 0.0 {
        return (0, true);
    }
    let quiet_threshold = peak * MOTION_QUIET_FRAC;
    let downswing = (impact as i64 - top as i64).max(1);
    let stall = MOTION_STALL_MIN_FRAMES.max(round_index(MOTION_STALL_FRACTION * downswing as f64));

    let mut quiet: i64 = 0;
    for i in (0..=top).rev() {
        if speeds[i] < quiet_threshold {
            quiet += 1;
            if quiet >= stall {
                // First moving frame above the quiet run = takeaway start. Clamped to the top,
                // which a long stall near the top can otherwise overshoot.
                return ((i + quiet as usize).min(top), true);
            }
        } else {
            quiet = 0;
        }
    }

    let estimate = top as i64 - round_index(FALLBACK_TEMPO_RATIO * downswing as f64);
    (estimate.max(0) as usize, false)
}

fn segment(phase: SwingPhase, start: i64, end: i64, ts: &[f64], detected: bool) -> PhaseSegment {
    PhaseSegment {
        phase,
        start_frame: start,
        end_frame: end,
        start_ms: ts[start as usize],
        end_ms: ts[end as usize],
        detected,
    }
}

/// Segment a keypoint timeline into the six swing phases, in canonical order.
///
/// Returns an empty list for a clip too short to contain a swing. The returned segments are
/// contiguous and their frame indices are monotonic non-decreasing, so a consumer can read phase
/// timings straight off the boundaries.
///
/// **`wrist` is the camera's question, not the golfer's.** [`LEAD_WRIST`] is right for face-on and
/// is what every band and every stored analysis was produced with; a down-the-line caller passes
/// [`TRAIL_WRIST`].
pub fn segment_phases(keypoints: &[FrameKeypoints], wrist: PoseLandmark) -> Vec<PhaseSegment> {
    let n = keypoints.len();
    if n < MIN_FRAMES {
        return Vec::new();
    }

    let ts: Vec<f64> = keypoints.iter().map(|f| f.timestamp_ms).collect();
    let xy = wrist_xy(keypoints, wrist);
    let ys: Vec<f64> = xy.iter().map(|&(_, y)| y).collect();
    let confident = wrist_confident(keypoints, wrist);

    let (top, impact) = top_and_impact(&ys, &confident, n);
    let (motion_start_frame, found) = motion_start(&xy, top, impact);

    // Bracket a small symmetric window around the top (transition) and after impact, then clamp
    // everything into a monotonic, non-overlapping boundary chain.
    let (top, impact, n) = (top as i64, impact as i64, n as i64);
    let b0 = 0;
    let b1 = motion_start_frame as i64;
    let b2 = b1.max(top - TRANSITION_HALF_FRAMES);
    let b3 = impact.min(top + TRANSITION_HALF_FRAMES).max(b2);
    let b4 = impact.max(b3);
    let b5 = (n - 1).min(b4 + IMPACT_HALF_FRAMES).max(b4);
    let b6 = n - 1;

    vec![
        segment(SwingPhase::Address, b0, b1, &ts, found),
        segment(SwingPhase::Backswing, b1, b2, &ts, found),
        segment(SwingPhase::Transition, b2, b3, &ts, true),
        segment(SwingPhase::Downswing, b3, b4, &ts, true),
        segment(SwingPhase::Impact, b4, b5, &ts, true),
        segment(SwingPhase::FollowThrough, b5, b6, &ts, true),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::keypoints::{Landmark, NUM_POSE_LANDMARKS};

    /// A frame whose lead wrist sits at `(x, y)`; every other landmark is parked and confident.
    fn frame(index: i64, x: f64, y: f64, visibility: f64) -> FrameKeypoints {
        let mut landmarks = vec![
            Landmark {
                x: 0.5,
                y: 0.5,
                z: 0.0,
                visibility: 1.0,
            };
            NUM_POSE_LANDMARKS
        ];
        landmarks[LEAD_WRIST as usize] = Landmark {
            x,
            y,
            z: 0.0,
            visibility,
        };
        FrameKeypoints {
            frame_index: index,
            timestamp_ms: index as f64 * 10.0,
            landmarks,
            camera_id: None,
        }
    }

    /// A wrist trace: still, up (y falls), down (y rises), then a finish higher than the top.
    fn a_swing() -> Vec<FrameKeypoints> {
        let ys = [
            0.80, 0.80, 0.80, 0.80, 0.80, // address
            0.70, 0.60, 0.50, 0.40, 0.30, // backswing
            0.40, 0.55, 0.70, 0.82, // downswing to impact
            0.70, 0.50, 0.20, // finish, higher than the top
        ];
        ys.iter()
            .enumerate()
            .map(|(i, &y)| frame(i as i64, 0.5, y, 1.0))
            .collect()
    }

    #[test]
    fn a_clip_too_short_to_hold_a_swing_segments_to_nothing() {
        let frames: Vec<_> = (0..MIN_FRAMES as i64 - 1)
            .map(|i| frame(i, 0.5, 0.5, 1.0))
            .collect();
        assert!(segment_phases(&frames, LEAD_WRIST).is_empty());
    }

    #[test]
    fn the_earliest_major_descent_wins_over_the_finish() {
        let phases = segment_phases(&a_swing(), LEAD_WRIST);
        // The top is the wrist's minimum at frame 9, not the deeper finish excursion at 16.
        assert_eq!(phases[2].phase, SwingPhase::Transition);
        let top_window = (phases[2].start_frame, phases[2].end_frame);
        assert!(
            top_window.0 <= 9 && 9 <= top_window.1,
            "the transition window {top_window:?} should straddle frame 9"
        );
        assert_eq!(
            phases[4].start_frame, 13,
            "impact is the end of the descent"
        );
    }

    /// ...and `a_swing`'s real downswing is also its *largest* descent, so the test above passes
    /// under "take the largest" as well — as do all 21 committed vectors, which is why raising
    /// [`MAJOR_RISE_FRACTION`] to 1.00 survived both gates. The rule earns its keep only when a
    /// **later** descent is bigger, which is the tracking-junk-after-the-finish case it was written
    /// for. Here the real downswing rises 0.45 and the post-finish excursion 0.50: at 0.80 both are
    /// major and the earliest wins; at 1.00 only the junk qualifies and the top lands 6 frames late.
    #[test]
    fn a_bigger_descent_after_the_finish_does_not_steal_the_top() {
        let ys = [
            0.80, 0.80, 0.80, 0.80, // address
            0.70, 0.60, 0.45, 0.30, // backswing
            0.45, 0.60, 0.75, // the downswing: rises 0.45
            0.60, 0.40, 0.20, // the finish carries the hands higher than the top
            0.40, 0.55, 0.70, // and tracking then falls away 0.50 — a bigger "descent"
        ];
        let frames: Vec<_> = ys
            .iter()
            .enumerate()
            .map(|(i, &y)| frame(i as i64, 0.5, y, 1.0))
            .collect();

        let runs = rising_runs(&ys, &[true; 17]);
        assert_eq!(runs.len(), 2, "two descents: {runs:?}");
        assert!(
            runs[1].0 > runs[0].0,
            "the later one is the larger: {runs:?}"
        );

        let phases = segment_phases(&frames, LEAD_WRIST);
        assert_eq!(
            phases[4].start_frame, 10,
            "impact ends the *earliest* major descent, not the largest one at frame 16"
        );
    }

    #[test]
    fn the_six_phases_are_contiguous_and_monotonic() {
        let frames = a_swing();
        let phases = segment_phases(&frames, LEAD_WRIST);
        assert_eq!(phases.len(), 6);
        for pair in phases.windows(2) {
            assert_eq!(pair[0].end_frame, pair[1].start_frame);
            assert!(pair[0].start_frame <= pair[0].end_frame);
        }
        assert_eq!(phases[0].start_frame, 0);
        assert_eq!(phases[5].end_frame, frames.len() as i64 - 1);
    }

    /// The Python marks the two boundaries the estimate defines, and only those two.
    #[test]
    fn an_undetected_motion_start_rides_on_address_and_backswing_alone() {
        // A wrist that never stops moving: no quiet run exists to walk back to.
        let mut ys: Vec<f64> = (0..24).map(|i| 0.8 - (i as f64 * 0.03)).collect();
        ys.extend((0..14).map(|i| 0.1 + (i as f64 * 0.05)));
        let frames: Vec<_> = ys
            .iter()
            .enumerate()
            .map(|(i, &y)| frame(i as i64, 0.5 + i as f64 * 0.01, y, 1.0))
            .collect();
        let phases = segment_phases(&frames, LEAD_WRIST);
        assert!(!phases[0].detected);
        assert!(!phases[1].detected);
        for later in &phases[2..] {
            assert!(later.detected, "{:?} should stay detected", later.phase);
        }
    }

    #[test]
    fn dim_frames_hold_the_last_confident_position() {
        let frames = vec![
            frame(0, 0.1, 0.9, 1.0),
            frame(1, 0.7, 0.2, 0.1),
            frame(2, 0.3, 0.7, 1.0),
        ];
        let xy = wrist_xy(&frames, LEAD_WRIST);
        assert_eq!(xy[1], (0.1, 0.9), "the dim frame holds frame 0");
        assert_eq!(xy[2], (0.3, 0.7));
        assert_eq!(
            wrist_confident(&frames, LEAD_WRIST),
            vec![true, false, true]
        );
    }

    /// If the *first* frame is dim there is nothing to hold, so it seeds itself.
    #[test]
    fn a_dim_first_frame_seeds_the_hold() {
        let frames = vec![frame(0, 0.4, 0.4, 0.0), frame(1, 0.6, 0.6, 0.0)];
        assert_eq!(wrist_xy(&frames, LEAD_WRIST), vec![(0.4, 0.4), (0.4, 0.4)]);
    }

    /// ADR-032 §3's frame-index edge, arriving through `max` rather than through `round`.
    #[test]
    fn the_first_maximum_wins_not_the_last() {
        let ys = [0.0, 0.9, 0.5, 0.9, 0.1];
        assert_eq!(first_argmax(&ys, 0), Some(1));
        assert_eq!(first_argmin(&ys), Some(0));
        // Rust's own `max_by` would answer 3 here, which is the divergence this guards.
        let rust_default = ys
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i);
        assert_eq!(rust_default, Some(3));
    }

    #[test]
    fn a_wrist_that_never_descends_has_no_runs() {
        let ys = [0.9, 0.8, 0.7, 0.6];
        assert!(rising_runs(&ys, &[true; 4]).is_empty());
    }

    #[test]
    fn held_frames_cannot_bound_a_descent() {
        let ys = [0.1, 0.5, 0.9, 0.2];
        let with_all = rising_runs(&ys, &[true; 4]);
        let without_middle = rising_runs(&ys, &[true, false, true, true]);
        assert_eq!(with_all[0].0, 0.8);
        assert_eq!(without_middle[0].0, 0.8, "the run spans the dim frame");
        assert_eq!(without_middle[0].1, 0, "and still starts where it started");
    }

    /// The stall length is a rounded frame count, so it is one of the seventeen sites ADR-032 §3
    /// names. The discriminating cases are the exact `.5` ties, where Python answers even.
    #[test]
    fn the_stall_length_rounds_half_to_even() {
        assert_eq!(
            round_index(MOTION_STALL_FRACTION * 10.0),
            2,
            "round(2.5) is 2"
        );
        assert_eq!(
            round_index(MOTION_STALL_FRACTION * 6.0),
            2,
            "round(1.5) is 2"
        );
        assert_eq!(
            round_index(MOTION_STALL_FRACTION * 14.0),
            4,
            "round(3.5) is 4"
        );
    }

    /// ...and the test above only proves `round_index` is right, not that [`motion_start`] *uses*
    /// it. Swapping it for `f64::round` survived both the table above and all 21 vectors, which is
    /// the gap this closes: a downswing of **10** frames puts the stall on `round(2.5)`, so the two
    /// rules ask for a 2-frame quiet run and a 3-frame one, and the trace below has exactly one
    /// 2-frame quiet patch near the top with a longer one further back.
    #[test]
    fn a_stall_on_an_exact_tie_takes_the_even_answer() {
        // speeds, by index: [0, 0, 0, 1.0, 0, 0, .5, .5, .5] — quiet at 4-5, moving at 3.
        let xs = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.5, 2.0, 2.5];
        let mut xy: Vec<(f64, f64)> = xs.iter().map(|&x| (x, 0.0)).collect();
        xy.resize(19, (2.5, 0.0));
        let (top, impact) = (8, 18);
        assert_eq!(round_index(MOTION_STALL_FRACTION * 10.0), 2);
        assert_eq!(
            motion_start(&xy, top, impact),
            (6, true),
            "a 2-frame stall stops at the quiet patch just below the top; a 3-frame one \
             (half-away-from-zero) walks past it to frame 3"
        );
    }

    #[test]
    fn a_top_at_frame_zero_needs_no_walk_back() {
        assert_eq!(motion_start(&[(0.0, 0.0); 8], 0, 4), (0, true));
    }

    #[test]
    fn a_wrist_that_never_moves_starts_at_zero() {
        // Every speed is 0.0, so the peak is 0.0 and there is no threshold to be quiet against.
        assert_eq!(motion_start(&[(0.5, 0.5); 12], 6, 9), (0, true));
    }

    /// The no-runs fallback, and the one place the first-versus-last maximum actually decides an
    /// answer rather than a helper's return value.
    ///
    /// A flat timeline has no rising run, so `top_and_impact` falls through to argmin/argmax — and
    /// every `y` ties, so Python's `max` answers frame 0 and Rust's `max_by` would answer frame 9.
    /// That is a nine-frame difference in where impact is, and no committed vector reaches this
    /// branch at all: swapping [`first_argmax`] for `max_by` survives all 21.
    #[test]
    fn a_timeline_of_identical_frames_still_segments() {
        let frames: Vec<_> = (0..10).map(|i| frame(i, 0.5, 0.5, 1.0)).collect();
        let phases = segment_phases(&frames, LEAD_WRIST);
        assert_eq!(phases.len(), 6);
        assert_eq!(phases[0].start_frame, 0);
        assert_eq!(
            (phases[4].start_frame, phases[4].end_frame),
            (0, 2),
            "impact is the *first* maximum, frame 0 — `max_by` would put it at frame 9"
        );
    }
}
