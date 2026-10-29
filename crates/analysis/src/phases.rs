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
//! # The clip-*choosing* half, and what gates it [M29 P19]
//!
//! [`candidate_downswings`], [`select_swing`], [`select_matching_swing`] and [`window_around`] are
//! the half of the Python module that decides *which* descent in a clip is the swing, before the
//! engine sees a frame. Nothing `scripts/conformance.py::run_vector` reaches calls them —
//! `api/pipeline.py` does, and `alignment.py` reaches only [`segment_phases`] and three constants —
//! so M22 left them out under the rule M22 P2 set for the `contracts/` registries: a port lands with
//! the gate that proves it. That gate is the `selection` family (`spec/vectors/selection/`), recorded
//! once from frozen Python by M29 P18 and answered here through `golf_core::selection_family`. Its
//! `_pick_swing` calls are this module's rules after [`crate::smoothing::smooth_keypoints`], as the
//! pipeline calls them; its `_auto_windows` calls are the pipeline's ordering around those picks,
//! which is `crates/lab`'s to port (M29 P25), not this module's.
//!
//! **Every reason is a sentence a golfer reads** (the pipeline logs it and writes it into
//! `analysis.json`'s notes), so its numbers go through `pyfmt`: `.2f` as [`pyfmt::fixed`], and the
//! band through [`pyfmt::g`], which writes `0.8` where `{:.2}` would write `0.80`.

use contracts::keypoints::{FrameKeypoints, PoseLandmark};
use contracts::swing::{PhaseSegment, SwingPhase};

use pyfmt::{fixed, g, round_index};

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
/// Here rather than in [`crate::alignment`] because this is its Python home, and module names carry
/// over one for one (ADR-032 §1). It has **two** Python readers, both ported: `alignment`'s
/// `with_measured_impact` (M22 P6), and `phases._struck` ([`select_swing`]'s rule 0, M29 P19), where
/// it decides which descents ended at a ball strike.
pub const STRIKE_TOLERANCE_S: f64 = 0.20;

/// The band a real downswing lasts, in seconds — `(low, high)`. [M22 P7]
///
/// Expressed in seconds, the one rule in this module that is (ADR-013): a downswing lasts about the
/// same time for every golfer at every frame rate, which is what makes an absolute bound meaningful
/// here where the rest of the module works in the clip's own time base.
///
/// **A sanity bound and nothing else** (the Python's comment carries the corpus sweep behind both
/// edges): it rejects descents that are not swing-shaped at all, and it must never be what decides
/// which of two real swings a clip is windowed on — the strike, then ordering, then the other camera
/// do that ([`select_swing`], [`select_matching_swing`]). Its readers here are those two rules (M29
/// P19) and `alignment::shared_tops`, which reads it only to *warn* that the shared top it is about
/// to impose is outside anything a golfer does — it used to veto the correction, and that veto is
/// what left four bundles replaying a panel at up to 3.11x (M11 P7). Python's last reader, the tempo
/// trainer, is not ported. Public, as in Python, for that reader's reason.
pub const POSSIBLE_DOWNSWING_S: (f64, f64) = (0.12, 0.80);

/// How far a descent may sit from the swing another camera already found, in seconds: the tolerance
/// [`select_matching_swing`] matches on. [M29 P19]
///
/// Two jobs: it admits a descent [`POSSIBLE_DOWNSWING_S`] rejects when the *other view* vouches for
/// it, and it is the distance "nearest the reference" is measured with. 0.12 sits inside the plateau
/// the 15 stored bundles give (0.085–0.25 all pick the same swing), about two frames at 60 fps above
/// the binding case. **Rejected: a tolerance relative to the reference**, because what it absorbs is
/// `top_and_impact` bracketing one descent a few frames early or late — a frame count, not a share
/// of the swing.
const MATCH_TOLERANCE_S: f64 = 0.12;

/// How much clip to keep around the chosen swing, in downswing-lengths before the top and after
/// impact — the clip's own time base (ADR-013). [M29 P19]
///
/// The lead is sized from [`FALLBACK_TEMPO_RATIO`]: a tour backswing runs about 3.5 downswings, and
/// motion start then needs a stretch of quiet address before that to walk back to. 5 is the smallest
/// lead that detected motion start on all six real clips it was measured on.
const WINDOW_LEAD: i64 = 5;
const WINDOW_TRAIL: i64 = 3;

/// A floor under [`WINDOW_LEAD`]'s lead, in seconds, because a short measured downswing shrinks the
/// lead exactly when it must not: the address goes, motion start goes with it, and the alignment
/// falls back to an estimate that multiplies that same short downswing by 3.5 (M10 §A4). [M29 P19]
///
/// **In seconds where the lead is in downswings, and that is the finding**: over the 14 stored
/// face-on clips, the lead `motion_start` needs is 0.92–1.27 s — near-constant in real time, and 3x
/// to 7x in downswing-lengths, because it tracks the *backswing*. 1.5 s clears the worst requirement
/// by 0.23 s. **Rejected: raising [`WINDOW_LEAD`] to 7**, which fixes the same three clips but gives
/// them the thinnest margins and widens every working clip for nothing.
const MIN_ADDRESS_LEAD_S: f64 = 1.5;

/// How far a descent must fall, as a share of the clip's largest, to be a candidate swing at all.
/// [M29 P19]
///
/// Deliberately looser than [`MAJOR_RISE_FRACTION`]: on `aaron-1-back` the largest descent is a
/// *bystander* at 0.417, so 0.80 would cut at 0.334 and drop the real swing (0.257) before duration
/// got to look. Public, as in Python, because `scripts/align_swings.py --list-swings` shares it.
pub const CANDIDATE_MIN_RISE: f64 = 0.45;

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

// ------------------------------------------------- the clip-choosing half [M29 P19]

/// One candidate downswing: the hands descending from `top` to `impact`. [M29 P19]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Downswing {
    pub top: usize,
    pub impact: usize,
    /// How far the tracked wrist fell, in normalized image units.
    pub rise: f64,
}

impl Downswing {
    /// `impact - top`, as Python's int subtraction: never negative for a descent [`rising_runs`]
    /// found, but [`window_around`] takes any downswing it is handed.
    fn frames(&self) -> i64 {
        self.impact as i64 - self.top as i64
    }

    /// The descent's length in seconds, `(impact - top) / fps`: an int difference made a float by
    /// true division, which is what `as f64` and `/` are here.
    pub fn seconds(&self, fps: f64) -> f64 {
        self.frames() as f64 / fps
    }
}

/// Every descent of the hands in the clip, earliest first. [M29 P19]
///
/// [`segment_phases`] locates *one* swing and is right to: it was validated against 461 GolfDB
/// clips, each holding exactly one. A phone clip does not — someone takes two practice swings and
/// then hits — and `top_and_impact`'s "earliest major run" rule, which exists to stop a *finish*
/// being read as the top, will return the first practice swing. This does not change that rule. It
/// exposes the runs [`rising_runs`] already finds, so a caller can see how many swings the clip
/// holds and choose one.
///
/// `min_fraction` is the share of the largest descent a run must reach to be listed; at
/// [`MAJOR_RISE_FRACTION`], Python's default, the first entry is the swing [`segment_phases`] would
/// choose. Python's keyword default is not carried over, because every caller ported passes
/// [`CANDIDATE_MIN_RISE`]. `wrist` is the camera's question, as in [`segment_phases`]: the window
/// this listing feeds decides which frames get scored, and one chosen on the other wrist describes
/// a different swing (M10 §A1).
pub fn candidate_downswings(
    keypoints: &[FrameKeypoints],
    min_fraction: f64,
    wrist: PoseLandmark,
) -> Vec<Downswing> {
    if keypoints.len() < MIN_FRAMES {
        return Vec::new();
    }
    let ys: Vec<f64> = wrist_xy(keypoints, wrist)
        .into_iter()
        .map(|(_, y)| y)
        .collect();
    let runs = rising_runs(&ys, &wrist_confident(keypoints, wrist));
    // `max(rise for …)`: a value, so which of two equal rises Python's `max` keeps is moot.
    let largest = runs
        .iter()
        .map(|&(rise, _, _)| rise)
        .fold(f64::NEG_INFINITY, f64::max);
    let threshold = min_fraction * largest;
    runs.into_iter()
        .filter(|&(rise, _, _)| rise >= threshold)
        .map(|(rise, top, impact)| Downswing { top, impact, rise })
        .collect()
}

/// Which descent in a multi-swing clip is the actual swing, and why. [M29 P19]
#[derive(Debug, Clone, PartialEq)]
pub struct SwingChoice {
    /// `[start, end)` in the clip's frames: [`window_around`] the chosen descent.
    pub window: (usize, usize),
    pub downswing: Downswing,
    /// **Every** descent found, not only the ones the choice was made among: once a ball strike has
    /// narrowed the field the choice is made among the struck descents, and this listing exists for
    /// a person checking whether the right one was taken, which a filtered listing cannot answer.
    pub candidates: Vec<Downswing>,
    /// The sentence the pipeline logs and writes into `analysis.json`'s notes.
    pub reason: String,
}

/// The `[start, end)` window holding one swing plus its takeaway, address and finish. [M29 P19]
///
/// With `fps`, the lead is floored at [`MIN_ADDRESS_LEAD_S`], so a fast downswing cannot squeeze
/// the address out of the window and take motion start with it. Without it, the lead is
/// [`WINDOW_LEAD`] downswings: a clip whose frame rate was never recorded degrades to that
/// arithmetic rather than refusing (ADR-013), as `alignment`'s `TOP_IMPACT` fallback keeps its
/// no-fps path. The floor's seconds become frames through Python's `round`, half-to-even
/// ([`round_index`]).
pub fn window_around(downswing: &Downswing, fps: Option<f64>) -> (usize, usize) {
    let frames = downswing.frames().max(1);
    let mut lead = WINDOW_LEAD * frames;
    if let Some(fps) = fps.filter(|&fps| fps > 0.0) {
        lead = lead.max(round_index(MIN_ADDRESS_LEAD_S * fps));
    }
    let start = (downswing.top as i64 - lead).max(0);
    let end = downswing.impact as i64 + WINDOW_TRAIL * frames;
    (start as usize, end as usize)
}

/// The candidates whose impact lands on a transient heard in that same clip. [M29 P19]
///
/// Evidence of a different kind from every other rule here: those read the pose stream and ask
/// whether a motion *looks* like a swing, and this asks whether a ball was hit. Frames in this
/// clip's own numbering, so no clip-to-clip offset appears.
///
/// **Empty means "audio has nothing to say", never "no swing".** `None` (nobody listened) and an
/// empty list (the detector heard nothing) both come back empty, and both leave the remaining rules
/// to judge the clip as they did before audio existed.
fn struck(candidates: &[Downswing], strike_frames: Option<&[i64]>, fps: f64) -> Vec<Downswing> {
    let Some(strikes) = strike_frames.filter(|strikes| !strikes.is_empty()) else {
        return Vec::new();
    };
    let tolerance = round_index(STRIKE_TOLERANCE_S * fps).max(1);
    candidates
        .iter()
        .copied()
        .filter(|swing| {
            strikes
                .iter()
                .any(|&frame| (swing.impact as i64 - frame).abs() <= tolerance)
        })
        .collect()
}

/// The "one candidate wins on its own" escape both selection rules share. [M29 P19]
///
/// `None` unless exactly one descent survived. Both rules arrive here having filtered everything
/// away, and both then owe the same answer: with nothing to choose between, a bound that only
/// exists to reject non-swings has no job. `pool` is what survived and `candidates` every descent
/// found ([`SwingChoice::candidates`]' doc). Python's `candidates` defaults to `pool`, and both of
/// its callers pass it, so the default is not carried over.
///
/// The reason says which justification applies — the band as a tie-break with no tie, or the band
/// *overruled* by a ball strike — and says "rather than the usual" rather than "outside the
/// possible", which stays true when a coarse frame rate brackets a real swing loosely.
fn lone_candidate_choice(
    pool: &[Downswing],
    fps: f64,
    candidates: &[Downswing],
    struck: bool,
) -> Option<SwingChoice> {
    let [only] = pool else {
        return None;
    };
    let (low, high) = POSSIBLE_DOWNSWING_S;
    let (lead, why) = if struck {
        (
            "1 descent ends at a ball strike, and its",
            "since a ball was struck there",
        )
    } else {
        (
            "1 descent, and its",
            "since there is nothing to choose between",
        )
    };
    Some(SwingChoice {
        window: window_around(only, Some(fps)),
        downswing: *only,
        candidates: candidates.to_vec(),
        reason: format!(
            "{lead} downswing measures {}s rather than the usual {}-{}s — taking it anyway, {why} \
             (top {}, impact {})",
            fixed(only.seconds(fps), 2),
            g(low),
            g(high),
            only.top,
            only.impact
        ),
    })
}

/// Pick the real swing out of a clip that holds several. [M29 P19]
///
/// [`segment_phases`] takes the **earliest** major descent, which is right for GolfDB and wrong for
/// a bay clip: on all four real clips it measured it picks a setup move, and the window decides
/// which frames get scored. Four rules, in order:
///
/// 0. **A ball was struck here** ([`struck`]): when a transient lands where a descent ends, the rules
///    below judge those descents and no others. With no strikes, or none near any descent, the
///    rules judge the whole field: a stray transient is not evidence against every candidate.
/// 1. **Swing-shaped at all** ([`POSSIBLE_DOWNSWING_S`]) — a sanity bound, never a discriminator.
/// 2. **Last, not first**: nobody takes a practice swing *after* hitting the ball.
/// 3. **One candidate wins on its own** ([`lone_candidate_choice`]).
///
/// `None` — never a guess — with no frame rate to read durations with, no descent at all, or several
/// and none plausible; a caller then falls back to [`segment_phases`]' own choice. `wrist` is the
/// camera's question, as everywhere in this module: on session 10 the lead wrist measures a 9.7 s
/// "downswing" from down-the-line, while the trail wrist finds the swing at 0.40 s (M10 §A1).
///
/// Python declines `fps <= 0.0` and would carry on with a NaN; this declines anything not above
/// zero, NaN included. No caller can hand it one: JSON has no NaN and `ClipMetadata` bounds its fps.
pub fn select_swing(
    keypoints: &[FrameKeypoints],
    fps: Option<f64>,
    wrist: PoseLandmark,
    strike_frames: Option<&[i64]>,
) -> Option<SwingChoice> {
    let fps = fps.filter(|&fps| fps > 0.0)?;
    let candidates = candidate_downswings(keypoints, CANDIDATE_MIN_RISE, wrist);
    let struck = struck(&candidates, strike_frames, fps);
    // `struck or candidates`: a strike list that lands near no descent leaves the duration rules
    // judging the whole field, rather than declining a clip over a stray noise.
    let pool = if struck.is_empty() {
        &candidates
    } else {
        &struck
    };
    let (low, high) = POSSIBLE_DOWNSWING_S;
    let plausible: Vec<Downswing> = pool
        .iter()
        .copied()
        .filter(|swing| {
            let seconds = swing.seconds(fps);
            low <= seconds && seconds <= high
        })
        .collect();
    let Some(&chosen) = plausible.last() else {
        return lone_candidate_choice(pool, fps, &candidates, !struck.is_empty());
    };

    let duration = fixed(chosen.seconds(fps), 2);
    let mut found = format!("{} descent(s)", candidates.len());
    if !struck.is_empty() {
        found.push_str(&format!(", {} ending at a ball strike", struck.len()));
    }
    let reason = if plausible.len() == 1 {
        format!(
            "{found}, 1 with a plausible downswing ({duration}s) — top {}, impact {}",
            chosen.top, chosen.impact
        )
    } else {
        format!(
            "{found}, {} with a plausible downswing; took the last ({duration}s, top {}, impact \
             {}). The clip holds more than one real-looking swing — confirm with --list-swings",
            plausible.len(),
            chosen.top,
            chosen.impact
        )
    };
    Some(SwingChoice {
        window: window_around(&chosen, Some(fps)),
        downswing: chosen,
        candidates,
        reason,
    })
}

/// Pick the descent that matches a swing already chosen in the *other* view. [M29 P19]
///
/// [`select_swing`] judges a clip alone, and on a down-the-line clip that is not enough: that phone
/// keeps rolling 15–24 s past impact, so "the last plausible descent" is routinely something that
/// happened after the ball was gone (M10 §A2). Once face-on's swing is chosen, the other view's is
/// no longer a free choice. Four rules, rule 0 [`select_swing`]'s:
///
/// 1. **Swing-shaped, or vouched for**: in [`POSSIBLE_DOWNSWING_S`], **or** within
///    [`MATCH_TOLERANCE_S`] of the reference — another camera looking at the same swing.
/// 2. **Nearest the reference, not last**, with "last" surviving only as the tie-break: Python's
///    `min` over the reversed list keeps the first of equals, which is the *later* candidate.
/// 3. **Decline rather than guess**, except that a lone candidate still wins.
///
/// `None` without a positive frame rate (NaN as in [`select_swing`]) or a positive reference: a
/// reference nobody measured is not one to match against.
pub fn select_matching_swing(
    keypoints: &[FrameKeypoints],
    fps: Option<f64>,
    reference_downswing_s: f64,
    wrist: PoseLandmark,
    strike_frames: Option<&[i64]>,
) -> Option<SwingChoice> {
    let fps = fps.filter(|&fps| fps > 0.0)?;
    if reference_downswing_s <= 0.0 {
        return None;
    }
    let candidates = candidate_downswings(keypoints, CANDIDATE_MIN_RISE, wrist);
    let struck = struck(&candidates, strike_frames, fps);
    let pool = if struck.is_empty() {
        &candidates
    } else {
        &struck
    };
    let (low, high) = POSSIBLE_DOWNSWING_S;
    let off = |seconds: f64| (seconds - reference_downswing_s).abs();
    let matched: Vec<(Downswing, f64)> = pool
        .iter()
        .map(|&swing| (swing, swing.seconds(fps)))
        .filter(|&(_, seconds)| {
            (low <= seconds && seconds <= high) || off(seconds) <= MATCH_TOLERANCE_S
        })
        .collect();
    // `min(reversed(matched), key=…)`: a later candidate replaces the best only when strictly
    // nearer, so a tie goes to the last one in clip order.
    let mut nearest: Option<(Downswing, f64)> = None;
    for &(swing, seconds) in matched.iter().rev() {
        if nearest.is_none_or(|(_, best)| off(seconds) < off(best)) {
            nearest = Some((swing, seconds));
        }
    }
    let Some((chosen, duration)) = nearest else {
        return lone_candidate_choice(pool, fps, &candidates, !struck.is_empty());
    };

    let admitted = if low <= duration && duration <= high {
        "in band".to_string()
    } else {
        format!("outside the usual {}-{}s band", g(low), g(high))
    };
    let mut found = format!("{} descent(s)", candidates.len());
    if !struck.is_empty() {
        found.push_str(&format!(", {} ending at a ball strike", struck.len()));
    }
    Some(SwingChoice {
        window: window_around(&chosen, Some(fps)),
        downswing: chosen,
        reason: format!(
            "{found}, {} matching the {}s reference from the other view; took the nearest ({}s, \
             {admitted}, off by {}s — top {}, impact {})",
            matched.len(),
            fixed(reference_downswing_s, 2),
            fixed(duration, 2),
            fixed(off(duration), 2),
            chosen.top,
            chosen.impact
        ),
        candidates,
    })
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

    // ------------------------------------------------- the clip-choosing half [M29 P19]
    //
    // The `selection` family is the gate (`golf_core::selection_family`); these pin the edges it
    // may not reach on purpose: the two rounding sites' ties, the tie-break, and `%g`'s band.

    /// Frames for a wrist `y` trace, every frame confident.
    fn trace(ys: &[f64]) -> Vec<FrameKeypoints> {
        ys.iter()
            .enumerate()
            .map(|(i, &y)| frame(i as i64, 0.5, y, 1.0))
            .collect()
    }

    /// Two identical swings back to back: up 10 frames, down 30, twice, then a drawdown that
    /// closes the second run. Both descents rise the same and last the same, so every rule that
    /// reads duration ties them.
    fn two_identical_swings() -> Vec<FrameKeypoints> {
        // The second backswing starts below the first impact, not level with it: a frame back at
        // the same height would extend the first run by one (`y >= peak` moves the peak).
        let back: Vec<f64> = (1..=10).map(|i| 0.8 - 0.05 * i as f64).collect();
        let down: Vec<f64> = (1..=30).map(|i| 0.3 + 0.5 * i as f64 / 30.0).collect();
        let mut ys = vec![0.8];
        for _ in 0..2 {
            ys.extend(&back);
            ys.extend(&down);
        }
        ys.extend([0.7, 0.6, 0.5]);
        trace(&ys)
    }

    #[test]
    fn two_identical_swings_are_two_candidates_of_equal_length() {
        let candidates =
            candidate_downswings(&two_identical_swings(), CANDIDATE_MIN_RISE, LEAD_WRIST);
        assert_eq!(candidates.len(), 2, "{candidates:?}");
        assert_eq!(candidates[0].frames(), 30);
        assert_eq!(candidates[1].frames(), 30);
        assert_eq!(candidates[0].rise, candidates[1].rise);
    }

    /// `min` over the reversed list keeps the first of equals, which is the **later** candidate.
    /// Rust's `min_by` over the list in order would take the earlier one: a different swing, and a
    /// window 41 frames away.
    #[test]
    fn a_tie_against_the_reference_goes_to_the_later_candidate() {
        let frames = two_identical_swings();
        let choice = select_matching_swing(&frames, Some(100.0), 0.30, LEAD_WRIST, None)
            .expect("two matching candidates");
        assert_eq!(choice.downswing, choice.candidates[1]);
        assert_eq!(
            choice.reason,
            format!(
                "2 descent(s), 2 matching the 0.30s reference from the other view; took the \
                 nearest (0.30s, in band, off by 0.00s — top {}, impact {})",
                choice.downswing.top, choice.downswing.impact
            )
        );
        // ...and judged alone, "last" is the rule rather than the tie-break, with the same answer.
        let alone = select_swing(&frames, Some(100.0), LEAD_WRIST, None).expect("two plausible");
        assert_eq!(alone.downswing, choice.downswing);
        assert!(alone
            .reason
            .contains("2 with a plausible downswing; took the last (0.30s"));
    }

    /// The band prints through `%g`: `0.12-0.8s`, where `{:.2}` would write `0.80`.
    #[test]
    fn a_lone_descent_outside_the_band_is_taken_and_says_so_in_g() {
        let back: Vec<f64> = (0..=10).map(|i| 0.8 - 0.05 * i as f64).collect();
        let down: Vec<f64> = (1..=100).map(|i| 0.3 + 0.5 * i as f64 / 100.0).collect();
        let mut ys = back;
        ys.extend(down);
        let frames = trace(&ys);
        let choice = select_swing(&frames, Some(100.0), LEAD_WRIST, None).expect("the lone one");
        assert_eq!(
            choice.reason,
            "1 descent, and its downswing measures 1.00s rather than the usual 0.12-0.8s — \
             taking it anyway, since there is nothing to choose between (top 10, impact 110)"
        );
        assert_eq!(
            select_swing(&frames, None, LEAD_WRIST, None),
            None,
            "no fps"
        );
        assert_eq!(
            select_swing(&frames, Some(0.0), LEAD_WRIST, None),
            None,
            "a zero fps"
        );
        assert_eq!(
            select_matching_swing(&frames, Some(100.0), 0.0, LEAD_WRIST, None),
            None,
            "no reference"
        );
    }

    /// `window_around`'s floor is `round(1.5 * fps)`, one of the rounding sites ADR-032 §3 names: at
    /// 7 fps it is `round(10.5)`, which Python answers 10 and `f64::round` 11.
    #[test]
    fn the_address_floor_rounds_half_to_even() {
        let swing = Downswing {
            top: 20,
            impact: 21,
            rise: 0.5,
        };
        assert_eq!(
            window_around(&swing, None),
            (15, 24),
            "five downswings of lead"
        );
        assert_eq!(
            window_around(&swing, Some(7.0)),
            (10, 24),
            "round(10.5) is 10"
        );
        assert_eq!(
            window_around(&swing, Some(0.0)),
            (15, 24),
            "a zero fps is no fps"
        );
        let backwards = Downswing {
            top: 21,
            impact: 20,
            rise: 0.5,
        };
        assert_eq!(
            window_around(&backwards, None),
            (16, 23),
            "a downswing of no frames counts as one"
        );
    }

    /// The strike tolerance is `max(1, round(0.20 * fps))`: at 12.5 fps that is `round(2.5)`, 2,
    /// so a strike three frames from impact is not this descent's — 3 under half-away-from-zero.
    #[test]
    fn the_strike_tolerance_rounds_half_to_even() {
        let swing = Downswing {
            top: 0,
            impact: 10,
            rise: 0.5,
        };
        assert!(struck(&[swing], Some(&[13]), 12.5).is_empty());
        assert_eq!(struck(&[swing], Some(&[12]), 12.5), [swing]);
        assert_eq!(
            struck(&[swing], Some(&[11]), 1.0),
            [swing],
            "never under a frame"
        );
        assert!(
            struck(&[swing], Some(&[]), 60.0).is_empty(),
            "heard nothing"
        );
        assert!(struck(&[swing], None, 60.0).is_empty(), "nobody listened");
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
