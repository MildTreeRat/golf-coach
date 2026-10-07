//! Event-anchored alignment of two views of one swing. `analysis/alignment.py`. [M22 P7]
//!
//! Two cameras film one swing and agree on nothing: not the frame rate, not the clip length, not
//! the moment either one started recording. There is no shared clock to appeal to and no
//! calibration, so triangulation is off the table permanently (ADR-011's 2026-08-05 addendum). What
//! both cameras *do* see is the swing, so the correspondence is built on the swing itself: each clip
//! is segmented **independently** and its three instants define a normalized axis,
//!
//! ```text
//! tau = 0  motion start        tau = 1  top        tau = 2  impact
//! ```
//!
//! and composing one clip's `tau -> frame` map with the other's inverse is the whole algorithm.
//!
//! **Not every anchor is worth the same.** Against 461 GolfDB clips impact lands within a median of
//! 1 frame and the top within 2, but motion start is out by a median of 7 with 40% of clips over 10.
//! It is therefore a **soft** anchor, used only when both clips found it independently *and* the two
//! agree about the backswing — as a tempo ratio, and again in seconds, because a ratio cannot see an
//! error the two views share.
//!
//! # What is here, and what has no gate
//!
//! P6 brought the three items a bundle with **no** second view reaches; P7 brings the rest of the
//! reachable module: [`anchors_from_keypoints`], [`tau_of_frame`] and [`align_swings`] with its nine
//! helpers, gated by the `alignment` stage on the fifteen corpus vectors.
//!
//! The *rendering* half deliberately did not come, on the rule P4 applied to `phases.py`'s
//! clip-choosing half: `frame_of_tau`, `map_frame`, `warp_speeds`, `pair_frames`,
//! `DEFAULT_TAU_RANGE` and `_MAX_WARP_SPEED_ERROR` are reached only from `api/pipeline.py`,
//! `scripts/` and `pose/side_by_side.py`, so `conformance.py::run_vector` never calls them and no
//! committed vector holds an answer for one. They would ship ungated — and §M29 retires their
//! callers into the Flutter shell, which is where the schedule they build belongs.
//!
//! That leaves one asymmetry worth stating out loud, because it looks like an omission:
//! [`tau_of_frame`] is here and its exact inverse is not. `tau_of_frame` has a reader inside
//! [`align_swings`] — [`clip_alignment`] and [`overlap`] both call it — where `frame_of_tau` has
//! none until a renderer exists.
//!
//! # The coverage the fifteen corpus vectors actually buy, which is less than "green" suggests
//!
//! Measured, not assumed, and each figure is pinned by a test in `tests/alignment.rs`:
//!
//! - **All fifteen report `Synchronized`**, because every corpus clip pair heard the strike. So
//!   `Full`, `TopImpact` and `ImpactOnly` never reach a committed answer and [`synchronized`]'s
//!   half-pair note is unreachable. The soft-anchor decision *is* gated, through
//!   `warp_motion_start` and the notes, but the tier it sets is not.
//! - **`warp_top` is `None` on all fifteen and `top_late_by` is too**, because the two views'
//!   downswings agree to within 27.6% against [`DOWNSWING_AGREEMENT`]'s 30%. So [`shared_tops`]
//!   returns `(None, None)` every time and [`arbitrate_tops`] returns `None` every time — which
//!   takes [`Arbitration`], [`top_at`], [`tempo_restated`], the `ImpactOnly` tier and
//!   `engine::without_contradicted_scores` with them. The defect M11 P7 built the arbiter for is
//!   *fixed*: on the unpinned anchors the widest gap is 21.7%, and pinning tau=2 to the strike
//!   moves it to 27.6% — so the corpus no longer contains a pair the arbiter can decide.
//! - **`motion_start_detected` is true on all thirty clips**, so the first refusal in
//!   [`align_swings`] is unreachable, as is [`estimated_motion_start`] — reached only when a clip
//!   lacks fps, and every corpus clip has one.
//! - **No corpus vector passes a `down_the_line_window`**, so [`anchors_from_keypoints`]' window
//!   branch has no vector behind it either.
//!
//! Where a stage cannot see a branch, this module's own unit tests are the gate. That is
//! `docs/CONFORMANCE.md` §2's rule, and it does more work in this phase than in any before it.

use contracts::alignment::{
    AlignmentQuality, ClipAlignment, SwingAlignment, SwingAnchors, TAU_TOP,
};
use contracts::keypoints::{ClipMetadata, FrameKeypoints, PoseLandmark};
use contracts::swing::{PhaseSegment, SwingPhase};
use contracts::Validate;

use crate::phases::{
    segment_phases, FALLBACK_TEMPO_RATIO, LEAD_WRIST, POSSIBLE_DOWNSWING_S, STRIKE_TOLERANCE_S,
};
use crate::smoothing::{smooth_keypoints, DEFAULT_WINDOW};
use pyfmt::{fixed, round_index};

/// The `camera_id` the face-on view records, which two tie-breaks below compare against.
///
/// **Not `contracts::placements::FACE_ON`, which is the string `"face-on"`.** The two spellings name
/// the same camera and are different values: a `camera_id` is free-form and comes off whatever the
/// capture source was told it is, while the placement view keys a fitted basis. Python writes the
/// literal `"face_on"` at both sites in this module, and substituting the placement constant would
/// make both tie-breaks silently never fire — the reference would fall through to `a` every time,
/// which is the same answer on every committed vector because `a` *is* the face-on clip there.
const FACE_ON_CAMERA: &str = "face_on";

/// How far apart two clips' backswing:downswing ratios may sit before the soft anchor is refused.
///
/// Frame rate cancels out of a ratio, so two views of ONE swing must agree here whatever the cameras
/// were set to; disagreement means at least one `motion_start` is wrong, or — worse, and the reason
/// this check is not merely a nicety — the two clips locked onto different swings entirely.
///
/// 0.35 is deliberately loose. `motion_start`'s median error is 7 frames, which on a ~15-frame
/// downswing is most of a tempo unit all by itself, so a tight bound would reject honest pairs. It is
/// sized to catch a *category* error (3.1 against 1.4), not to grade agreement.
const TEMPO_AGREEMENT: f64 = 0.35;

/// The cross-check the constant above is structurally unable to make: how far apart the two views'
/// *backswing durations* may sit, in seconds, before the soft anchor is refused.
///
/// A ratio divides out the downswing. So when both views mismeasure one swing in the same direction
/// — the common case, since they are watching the same motion — the ratios agree while the durations
/// do not, and [`TEMPO_AGREEMENT`] waves through a pair that is visibly apart on screen. That is not
/// a loose bound, it is a blind one: no value of [`TEMPO_AGREEMENT`] catches this, which is why there
/// is a second constant here rather than a tighter first one.
///
/// Sized on the four bundles that reported `full` on disk (M10 P3, docs/M10_ALIGNMENT_ACCURACY.md
/// B2):
///
/// ```text
/// session  9   1.084s vs 0.851s   0.233s apart   tempo 4.06 / 4.25   must fail
/// session  6   0.984s vs 0.784s   0.200s apart   tempo 2.36 / 2.14   must fail
/// session 11   0.834s vs 0.700s   0.133s apart   tempo 2.50 / 2.00   must pass
/// session  8   0.884s vs 0.867s   0.017s apart   tempo 1.89 / 2.60   must pass
/// ```
///
/// leaving the band (0.133, 0.200). 0.167 is ten frames at 60 fps and sits at its log midpoint, a
/// third of the width clear either side. Read sessions 9 and 8 against each other: the pair fourteen
/// times further apart in real time is the pair whose tempo ratios agree more closely. That inversion
/// is the entire argument for measuring this in seconds.
///
/// Absolute seconds, where the other two agreement bounds are relative. A relative bound here would
/// re-import the scale-blindness being fixed, allowing a slow backswing more real drift than a fast
/// one — and the viewer sees the same daylight between the panels either way, because what is wrong
/// at tau=0 is an offset and not a rate.
const BACKSWING_AGREEMENT_S: f64 = 0.167;

/// How far apart the two views' *downswing durations* may sit, relative, before the top is refused as
/// an anchor too. Unlike the tempo cross-check this is a check on the **hard** anchors, so it runs
/// whatever happened to the soft one.
///
/// The warp's central assumption is that between two anchors the views progress through the swing
/// proportionally (ADR-015). That is exactly true only if the instants are exactly right. When they
/// are not, forcing both panels to reach tau=1 and tau=2 together does not hide the disagreement — it
/// converts it into *playback speed*, resampling the follower to catch up by impact. A viewer reads
/// that as one camera running fast, which is worse than a visible seam: it silently misrepresents
/// tempo and sequencing, the two things the side-by-side exists to show.
///
/// 0.30 sits between the two regimes with room on both sides. Top and impact are located to a median
/// of 2 and 1 frames, so honest disagreement on a ~15-frame downswing runs to maybe 20%; the failure
/// this was sized to catch measured 0.234 s against 0.400 s, a gap of 0.42.
///
/// **It is the whole reason four of this module's functions are unreachable from the committed
/// vectors**, which is P7's sharpest finding and not a fact about the constant: the widest gap the
/// fifteen corpus pairs produce is 0.276. See the module doc.
const DOWNSWING_AGREEMENT: f64 = 0.30;

/// Below this backswing:downswing ratio the `motion_start` boundary is wrong rather than the swing
/// being unusual, and no distribution is needed to say so — the slowest credible amateur is well
/// above 1:1, and a backswing that measures *shorter* than its own downswing is not a golf swing.
///
/// It fires on real phone footage for a reproducible reason: `phases::motion_start` walks back from
/// the top looking for the last quiet stretch of wrist speed, and a golfer who pauses at the top
/// hands it one immediately, so the boundary lands a frame or two below the top and the backswing
/// measures near zero. The estimate is still returned as detected, because from inside `phases` it
/// looks fine.
///
/// Public because `engine::analyze_swing_bundle` applies the same floor to say out loud that the
/// *tempo checkpoint* is untrustworthy on such a swing. The two uses are one fact read twice: a
/// wrong `motion_start` is useless as an alignment anchor and makes the tempo ratio derived from it
/// wrong. [`align_swings`] is the alignment half, and it is the reader that refuses the anchor.
pub const MIN_PLAUSIBLE_TEMPO: f64 = 1.0;

/// `anchors` with tau=2 pinned to a ball strike **heard** in this same clip. [M11 P6]
///
/// The one anchor in the system that can be measured rather than inferred. Frames, and frames in
/// *this clip's own numbering* — the convention `phases::struck` takes, and for the same reason: no
/// clip-to-clip offset appears anywhere in `analysis/`, because each view is anchored against what
/// its own microphone heard and the shared clock falls out of that.
///
/// **The earliest candidate wins, and that rule does the work the window cannot.** The bay makes
/// four transients per shot and the loudest is the ball hitting the impact screen 85-145 ms later,
/// while the correction this exists to make runs to 125 ms — so no window wide enough to admit the
/// real error is narrow enough to exclude the screen strike, and picking the *nearest* transient
/// would take the screen on any clip whose pose impact was already right. Ordering resolves what
/// distance cannot: the ball is the first sound a shot makes.
///
/// Returns `anchors` **unchanged** when there is nothing to pin to — no strikes, no fps to size the
/// window with, or no transient inside it — and `impact_measured` then stays false, which is the
/// honest reading: not measured is not the same as measured at the pose estimate (ADR-010 §2).
///
/// # The two guards the type system does not give for free
///
/// `SwingAnchors` requires impact strictly after the top and inside the clip, and Python enforces
/// that in the *filter* rather than leaving it to the validator, because `model_copy` does not
/// re-run one. The port has the same shape for a sharper reason — nothing re-validates a struct
/// literal either — and both conditions are live rather than defensive: on a fast downswing the
/// window reaches back past the top all by itself (face-on measures 0.183 s of downswing on M10's
/// four offenders, against a 0.20 s window).
pub fn with_measured_impact(anchors: &SwingAnchors, strike_frames: Option<&[i64]>) -> SwingAnchors {
    // `not strike_frames or not anchors.fps` — an empty list is as falsy as a missing one, and a
    // zero fps is as falsy as no fps. The second can only arrive by a struct literal, since a
    // parsed `SwingAnchors` carries `gt(fps, 0.0)`.
    let strikes = strike_frames.unwrap_or_default();
    let Some(fps) = anchors.fps.filter(|rate| *rate != 0.0) else {
        return anchors.clone();
    };
    if strikes.is_empty() {
        return anchors.clone();
    }

    // Python's one-argument `round`, which is half-to-even: ADR-032 §3's rounding edge at one of
    // `alignment.py`'s ten frame-index sites.
    //
    // **Swapping it for Rust's `f64::round` survives every test in this workspace, and provably
    // so** (M22 P6's mutation sweep, one of two survivors). A tie needs `0.20 * fps == n + 0.5`,
    // so `fps == 5n + 2.5` — a frame rate with a half in it, which no camera reports: 24, 25, 30,
    // 30000/1001, 50, 60, 60000/1001, 120 and 240 all miss it, and at 29.97 the product is
    // 5.994005994. So the two rules agree on every rate this repo can be handed, and
    // `round_index` is here because the *site* is one of the seventeen that round a frame index,
    // not because this particular multiplication needs it. A fps that did tie would move the
    // window by a frame and change which transient is admitted.
    let window = round_index(STRIKE_TOLERANCE_S * fps);
    let last = anchors
        .frame_count
        .filter(|count| *count != 0)
        .map(|c| c - 1);

    let earliest = strikes
        .iter()
        .copied()
        .filter(|frame| (frame - anchors.impact).abs() <= window)
        .filter(|frame| *frame > anchors.top)
        .filter(|frame| last.is_none_or(|last| *frame <= last))
        .min();
    let Some(impact) = earliest else {
        return anchors.clone();
    };

    SwingAnchors {
        impact,
        impact_measured: true,
        ..anchors.clone()
    }
}

/// The three anchors, read off the boundary chain `segment_phases` produces.
///
/// Returns `None` for a clip that could not be segmented at all, or whose segmentation is
/// degenerate (no downswing to divide by) — reported, not raised (ADR-013).
///
/// `top` is the midpoint of the `Transition` segment, which recovers the detected top exactly unless
/// the ±3-frame window around it was clamped by a neighbouring boundary; that clamp only binds on
/// clips too short to hold the window, which are degenerate for alignment anyway. It is the same
/// derivation the overlay banners use, so the frame a banner is stamped on and the frame the warp
/// pins to tau=1 cannot drift apart.
///
/// `offset` shifts every index back into the coordinates of the *unsliced* clip, for callers that
/// segmented a window of a multi-swing recording. `engine::analyze_swing_bundle` passes `0` and
/// shifts the phases instead, which is why nothing in this crate exercises a non-zero one yet.
///
/// # The lookup takes the first segment of each phase, where Python's dict takes the last
///
/// Python builds `{segment.phase: segment for segment in phases}`, and a dict comprehension keeps
/// the **last** value written for a repeated key. This scans for the first. The two differ only on a
/// phase list that names one phase twice, which `segment_phases` cannot produce — it emits the six
/// phases once each, in canonical order — and both call sites in the engine pass its output
/// unchanged or shifted. So they agree on the whole reachable domain, which is the standard P5 and
/// P5b set for a deliberate divergence that survives.
///
/// `trajectory::anchors_from_phases` is the same lookup over the same three phases and carries the
/// same note. It is **not** the same function: that one returns fractional instants for a resampler
/// and this one returns integer frames for a warp, and its midpoint is `/ 2.0` where this one
/// floor-divides.
pub fn anchors_from_phases(
    phases: &[PhaseSegment],
    clip: Option<&ClipMetadata>,
    camera_id: Option<&str>,
    offset: i64,
) -> Option<SwingAnchors> {
    let of = |phase: SwingPhase| phases.iter().find(|segment| segment.phase == phase);
    let address = of(SwingPhase::Address)?;
    let transition = of(SwingPhase::Transition)?;
    let impact = of(SwingPhase::Impact)?;

    // Python's `//`, which floors rather than truncating. Both frames are non-negative here — a
    // `PhaseSegment` carries `ge(start_frame, 0)` — so Rust's truncating `/` agrees, and the
    // division is written this way rather than through a helper because a negative frame index is
    // refused two layers up rather than handled.
    let top = (transition.start_frame + transition.end_frame) / 2;
    if impact.start_frame <= top {
        return None;
    }

    let anchors = SwingAnchors {
        motion_start: address.end_frame + offset,
        top: top + offset,
        impact: impact.start_frame + offset,
        motion_start_detected: address.detected,
        impact_measured: false,
        camera_id: camera_id.map(str::to_string),
        frame_count: clip.and_then(|clip| clip.frame_count),
        fps: clip.and_then(|clip| clip.fps),
    };
    // Pydantic validates on construction and Python therefore *raises* here rather than returning
    // `None`: the two guards above cover the degenerate downswing, and a `motion_start` past the top
    // would mean `segment_phases` emitted an address segment ending after the transition's midpoint,
    // which is a broken segmenter rather than a bad clip. The panic says the same thing the
    // `ValidationError` does, at the same moment — the choice `CheckpointOutcome::scored` already
    // made in this crate.
    anchors
        .validate()
        .unwrap_or_else(|e| panic!("segment_phases produced anchors no swing can have: {e}"));
    Some(anchors)
}

/// Smooth, segment and extract anchors — the whole per-clip path in one call.
///
/// Smoothing first is not optional: [`segment_phases`] expects it, and raw MediaPipe landmarks
/// jitter enough to move the top.
///
/// `window` restricts the search to `[start, end)` so a clip containing practice swings can be
/// pointed at the real one. The slice is segmented on its own and the resulting indices are shifted
/// back, so every frame number this returns is in the original clip's coordinates and a caller never
/// has to track the offset itself.
///
/// `wrist` is which wrist to track, and it is the *only* place the two views are told apart:
/// `engine::analyze_swing_bundle` passes the **trail** wrist for the down-the-line clip, because
/// from behind the lead wrist is the far arm and is tracked in 39% of frames.
///
/// # An empty window refuses here and is ignored one module over
///
/// `engine::windowed` returns the whole clip for a window that selects nothing; this returns `None`.
/// The two are not inconsistent — that one is choosing frames to *score* and has a clip either way,
/// while a window naming no frames here has no swing to find anchors in — but they are one line
/// apart in behaviour and the difference is easy to port away by accident.
pub fn anchors_from_keypoints(
    keypoints: &[FrameKeypoints],
    clip: Option<&ClipMetadata>,
    window: Option<(i64, i64)>,
    wrist: Option<PoseLandmark>,
) -> Option<SwingAnchors> {
    let mut frames = keypoints;
    let mut offset = 0;
    if let Some((lo, hi)) = window {
        let start = lo.max(0);
        let end = hi.min(keypoints.len() as i64);
        if end - start <= 0 {
            return None;
        }
        frames = &keypoints[start as usize..end as usize];
        offset = start;
    }

    let camera_id = frames.iter().find_map(|frame| frame.camera_id.as_deref());

    // **`frame_count` becomes the length of the whole keypoint list, not the window's.** Python's
    // comment at this line says "the window's own length", and the code says `len(keypoints)`; the
    // code is right and the comment is stale, because `offset` has already put every index back into
    // whole-clip coordinates and that is what `frame_count` clamps. What the copy *does* replace is
    // the container's reported count, which a truncated or re-encoded clip can disagree with.
    let replaced;
    let metadata = match (window, clip) {
        (Some(_), Some(clip)) => {
            replaced = ClipMetadata {
                frame_count: Some(keypoints.len() as i64),
                ..clip.clone()
            };
            Some(&replaced)
        }
        _ => clip,
    };

    let smoothed = smooth_keypoints(frames, DEFAULT_WINDOW);
    let phases = segment_phases(&smoothed, wrist.unwrap_or(LEAD_WRIST));
    anchors_from_phases(&phases, metadata, camera_id, offset)
}

/// What swing-instant this frame shows — the exact inverse of `frame_of_tau`.
///
/// Piecewise-linear through the anchors, and linear outside them: past impact at the downswing rate,
/// before motion start at the backswing rate.
///
/// `motion_start` overrides the tau=0 anchor, which is how the soft-anchor fallback substitutes the
/// same estimate into both clips. `top` overrides tau=1 the same way, which is how the `ImpactOnly`
/// tier gives both clips a shared downswing duration measured back from impact.
///
/// `frame` is a float because the forward map produces one; every caller in this crate passes an
/// integer frame index.
pub fn tau_of_frame(
    anchors: &SwingAnchors,
    frame: f64,
    motion_start: Option<i64>,
    top: Option<i64>,
) -> f64 {
    let zero = motion_start.unwrap_or(anchors.motion_start);
    let pivot = top.unwrap_or(anchors.top) as f64;
    let downswing = anchors.impact as f64 - pivot;
    let backswing = pivot - zero as f64;

    if frame >= pivot {
        return TAU_TOP + (frame - pivot) / downswing;
    }
    // With no backswing to measure (a clip opening at the top), fall back to the downswing rate
    // rather than dividing by zero — degraded, and flagged by the quality tier.
    let rate = if backswing > 0.0 {
        backswing
    } else {
        downswing
    };
    TAU_TOP - (pivot - frame) / rate
}

/// Put two clips of one swing on a shared tau axis. [M11 P7/P8]
///
/// Both clips are always aligned on **top and impact** — the two anchors the bake-off says are worth
/// trusting. Motion start joins them only when both clips detected it independently and the two
/// backswings agree twice over: as tempo ratios, and — when both clips reported a frame rate — as
/// durations in seconds, which is the disagreement a ratio is blind to.
///
/// When it is refused, both clips take the tour-median estimate off **one shared downswing
/// duration** and convert it through their own fps, so the pre-top region degrades by the same
/// number of *seconds* in each panel. A clip without fps is the exception and still degrades off its
/// own downswing — see [`shared_motion_starts`].
///
/// Above all of that sits one case that is not a count of anchors at all: when both clips arrive with
/// `impact_measured`, the pair has a real shared clock and reports `Synchronized`. See
/// [`synchronized`].
///
/// That shared clock is also what lets a disagreement about the *top* be settled rather than merely
/// reported: with tau=2 fixed to one instant in real time, the shorter downswing is the late top and
/// both panels are held to the longer one. See [`arbitrate_tops`].
///
/// **The finding outlives the correction.** Which top is late is recorded on each clip as
/// `ClipAlignment::top_is_late` whether or not the warp went on to move it, because the two answer
/// different questions — the warp asks *can this be replayed honestly*, and the flag asks *was the
/// instant a checkpoint was timed from contradicted*. A pair with no shared clock therefore carries a
/// corrected warp and no finding, which is right: the render is fixed and nothing has been proved
/// about the golfer. It is the flag rather than the warp that `engine::without_contradicted_scores`
/// reads before retiring a score.
///
/// # `a` is the scored view, by convention rather than by type
///
/// `engine::analyze_swing_bundle` passes the face-on anchors first, and it then reads `alignment.a`
/// as *the clip the checkpoints were measured on*. Nothing here enforces that — the function is
/// symmetric apart from the two `"face_on"` tie-breaks below — so swapping the arguments produces a
/// valid alignment that means something different downstream.
pub fn align_swings(a: &SwingAnchors, b: &SwingAnchors) -> SwingAlignment {
    let mut notes: Vec<String> = Vec::new();
    let mut quality = AlignmentQuality::Full;

    // Neither loop below breaks: two clips can each fail, and each one that does says so. Python's
    // `(a, "a"), (b, "b")` pairs the fallback label with the anchors rather than deriving it from a
    // position, which is what keeps the note readable on a clip with no `camera_id`.
    let mut use_soft = true;
    for (anchors, side) in [(a, "a"), (b, "b")] {
        if !anchors.motion_start_detected {
            use_soft = false;
            let label = anchors.camera_id.as_deref().unwrap_or(side);
            notes.push(format!("{label}: motion start was estimated, not detected"));
        }
    }

    if use_soft {
        for (anchors, side) in [(a, "a"), (b, "b")] {
            let label = anchors.camera_id.as_deref().unwrap_or(side);
            match anchors.tempo_ratio() {
                None => {
                    use_soft = false;
                    notes.push(format!(
                        "{label}: no measurable backswing; motion start dropped as an anchor"
                    ));
                }
                Some(ratio) if ratio < MIN_PLAUSIBLE_TEMPO => {
                    use_soft = false;
                    notes.push(format!(
                        "{label}: backswing measures {} downswings, which no golf swing does — \
                         motion start has collapsed onto the top (the pause at the top reads as \
                         the 'quiet' stretch the takeaway is measured back to). Using the \
                         tour-median estimate instead",
                        fixed(ratio, 2)
                    ));
                }
                Some(_) => {}
            }
        }
    }

    if use_soft {
        // Both were just checked, so Python asserts here rather than re-branching. The expect is
        // that assert: unreachable unless the two loops above stop agreeing with each other.
        let ratio_a = a.tempo_ratio().expect("checked in the loop above");
        let ratio_b = b.tempo_ratio().expect("checked in the loop above");
        if relative_gap(ratio_a, ratio_b) > TEMPO_AGREEMENT {
            use_soft = false;
            notes.push(tempo_disagreement_note(a, b, ratio_a, ratio_b));
        } else if let (Some(fps_a), Some(fps_b)) =
            (a.fps.filter(|r| *r != 0.0), b.fps.filter(|r| *r != 0.0))
        {
            // The ratio check's blind spot, measured in real time — see `BACKSWING_AGREEMENT_S`.
            // `else if`, because a pair that already failed on tempo has been refused and a second
            // note about the same disagreement would only crowd the first. Both frame rates or
            // nothing: without one there is no duration to compare, so the soft anchor stands on the
            // ratio alone exactly as it did before (ADR-013, reported not raised).
            let backswing_a = a.backswing_frames() as f64 / fps_a;
            let backswing_b = b.backswing_frames() as f64 / fps_b;
            if (backswing_a - backswing_b).abs() > BACKSWING_AGREEMENT_S {
                use_soft = false;
                notes.push(backswing_disagreement_note(a, b, backswing_a, backswing_b));
            }
        }
    }

    let (mut motion_a, mut motion_b) = if use_soft {
        (a.motion_start, b.motion_start)
    } else {
        quality = AlignmentQuality::TopImpact;
        shared_motion_starts(a, b)
    };

    // Which top the shared clock says is late — computed here rather than inside [`shared_tops`]
    // because the two outputs part company: the warp only *moves* a top whose reference duration is
    // a possible downswing, while the finding that one of them is late holds either way and is what
    // `engine::without_contradicted_scores` retires a checkpoint on. Deciding it once is also what
    // stops the warp and `ClipAlignment::top_late_by` disagreeing about the same pair.
    let arbitration = arbitrate_tops(a, b);

    // The hard anchors get their own check. If the two views disagree about how long the downswing
    // lasted, pinning both to tau=1 resamples one panel to catch up — see `DOWNSWING_AGREEMENT`.
    let (top_a, top_b) = shared_tops(a, b, &mut notes, arbitration.as_ref());
    if let (Some(pinned_a), Some(pinned_b)) = (top_a, top_b) {
        quality = AlignmentQuality::ImpactOnly;
        motion_a =
            (pinned_a - round_index(FALLBACK_TEMPO_RATIO * (a.impact - pinned_a) as f64)).max(0);
        motion_b =
            (pinned_b - round_index(FALLBACK_TEMPO_RATIO * (b.impact - pinned_b) as f64)).max(0);
    }

    quality = synchronized(a, b, quality, &mut notes);

    // Python writes `late_by if late is a else None` per side, over an identity comparison against
    // the very object the arbitration holds. `late_is_a` is that decision recorded instead of
    // re-derived: it is `seconds_a < seconds_b`, the branch that chose `late`. The two differ only if
    // `a` and `b` are the same object, and an aliased pair cannot reach here — its two durations are
    // equal, so the gap is zero and [`arbitrate_tops`] has already returned `None`.
    let (late_a, late_b) = match &arbitration {
        Some(arbitration) if arbitration.late_is_a => (Some(arbitration.frames_late), None),
        Some(arbitration) => (None, Some(arbitration.frames_late)),
        None => (None, None),
    };

    let alignment = SwingAlignment {
        a: Some(clip_alignment(a, motion_a, top_a, late_a)),
        b: Some(clip_alignment(b, motion_b, top_b, late_b)),
        quality,
        notes,
        overlap: Some(overlap(a, motion_a, top_a, b, motion_b, top_b)),
    };
    // `ClipAlignment` carries `ge(warp_motion_start, 0)`, `ge(warp_top, 0)` and `gt(top_late_by, 0)`,
    // and pydantic runs all three at construction. Rust has no constructor hook, so the check is the
    // explicit call [`anchors_from_phases`] already makes for the same reason (ADR-032 §4).
    alignment
        .validate()
        .unwrap_or_else(|e| panic!("align_swings assembled an invalid SwingAlignment: {e}"));
    alignment
}

/// `Synchronized` when both clips heard the strike, else `quality` untouched. [M11 P6]
///
/// **Last, and overwriting whatever the anchor count came to.** A measured tau=2 is better evidence
/// than three inferred anchors, so a synchronized pair must not go on reporting `ImpactOnly` — that
/// tier is the *worst* non-failing one precisely because its single anchor was a guess.
///
/// **This function still only sets the label.** The warp is decided above it, and what a measured
/// impact does *there* is give [`shared_tops`] an arbiter for a disagreement about the top. Both
/// facts come off the same `impact_measured` pair and are deliberately read in two places, because
/// the tier is a claim about evidence and the warp is a claim about frames.
///
/// Half a pair is worth a note and no tier. One clip anchored on sound and the other on pose does not
/// make a shared clock, and the interesting case is the ordinary one — a phone across the bay that
/// heard nothing — so name the view that could not contribute rather than leaving a silent `Full`.
fn synchronized(
    a: &SwingAnchors,
    b: &SwingAnchors,
    quality: AlignmentQuality,
    notes: &mut Vec<String>,
) -> AlignmentQuality {
    if a.impact_measured && b.impact_measured {
        return AlignmentQuality::Synchronized;
    }
    if a.impact_measured || b.impact_measured {
        let (heard, deaf) = if a.impact_measured { (a, b) } else { (b, a) };
        notes.push(format!(
            "{}'s impact was measured on the ball strike but {}'s was not, so the two clips are \
             aligned on inferred instants as before — a shared clock needs the strike in both",
            heard.camera_id.as_deref().unwrap_or("one view"),
            deaf.camera_id.as_deref().unwrap_or("the other"),
        ));
    }
    quality
}

/// Why two views of one swing came out at different tempos.
fn tempo_disagreement_note(
    a: &SwingAnchors,
    b: &SwingAnchors,
    ratio_a: f64,
    ratio_b: f64,
) -> String {
    format!(
        "tempo ratios disagree ({} vs {}) — frame rate cancels out of a ratio, so two views of one \
         swing should not. {}",
        fixed(ratio_a, 2),
        fixed(ratio_b, 2),
        which_half_is_wrong(a, b, ratio_a < ratio_b),
    )
}

/// Why a pair whose tempo ratios agree is refused anyway. [M10 P3]
///
/// This note has to say more than the tempo one, because the reader has just been told nothing is
/// wrong: the ratios matched. Lead with the arithmetic reason they could match — a ratio divides out
/// the downswing — or the refusal reads as the check being fussy about a pair it already approved.
fn backswing_disagreement_note(
    a: &SwingAnchors,
    b: &SwingAnchors,
    seconds_a: f64,
    seconds_b: f64,
) -> String {
    format!(
        "the two views' backswings are {}s apart ({}s and {}s) even though their tempo ratios agree \
         — a ratio divides out the downswing, so it cannot see two views whose errors scale \
         together. {}",
        fixed((seconds_a - seconds_b).abs(), 3),
        fixed(seconds_a, 3),
        fixed(seconds_b, 3),
        which_half_is_wrong(a, b, seconds_a < seconds_b),
    )
}

/// Which of the two boundaries to doubt — the denominator tells you.
///
/// Both refusals above are a disagreement about the backswing, so it lives in either the takeaway
/// boundary or the top. If the clips also disagree about the *downswing*, the tops are on different
/// events and "different swings" is the likeliest reading. But when the downswings agree, the two
/// views are demonstrably watching the same motion and the whole difference sits in the takeaway:
/// one clip's motion start is late. Saying "different swings" there sends the reader to check
/// something that is fine.
///
/// **The "different swings" reading is the one this corpus has never once justified**, and it is now
/// unreachable from the committed vectors for a second reason: it needs the two downswings to
/// disagree past `DOWNSWING_AGREEMENT`, and none of the fifteen does. It stays as the last resort
/// because a pair that heard nothing still cannot rule it out.
///
/// Shared by both notes rather than written twice, because the branch is the same judgement and a
/// second copy is a second thing to drift.
fn which_half_is_wrong(a: &SwingAnchors, b: &SwingAnchors, shorter_backswing_is_a: bool) -> String {
    if let (Some(fps_a), Some(fps_b)) = (a.fps.filter(|r| *r != 0.0), b.fps.filter(|r| *r != 0.0)) {
        let seconds_a = a.downswing_frames() as f64 / fps_a;
        let seconds_b = b.downswing_frames() as f64 / fps_b;
        if relative_gap(seconds_a, seconds_b) <= DOWNSWING_AGREEMENT {
            let late = if shorter_backswing_is_a {
                a.camera_id.as_deref().unwrap_or("a")
            } else {
                b.camera_id.as_deref().unwrap_or("b")
            };
            return format!(
                "The two downswings agree ({}s and {}s), so this is the takeaway boundary, not two \
                 different swings — {late} is finding its motion start late. Dropping it as an \
                 anchor; the swing itself is fine",
                fixed(seconds_a, 3),
                fixed(seconds_b, 3),
            );
        }
        if let Some(arbitration) = arbitrate_tops(a, b) {
            return format!(
                "The two downswings disagree ({}s and {}s) — but both views heard the strike, so \
                 tau=2 is one instant in real time and this is one swing filmed twice, not two. \
                 {}'s top is the late one, by {} frames",
                fixed(seconds_a, 3),
                fixed(seconds_b, 3),
                arbitration.late_label(),
                arbitration.frames_late,
            );
        }
    }
    "Most likely the two clips are showing DIFFERENT swings; check for a practice swing in one of \
     them"
        .to_string()
}

/// Which of two disagreeing tops is the wrong one, once a shared clock makes that decidable.
struct Arbitration<'a> {
    /// The view whose top landed late. Its downswing is the shorter of the two.
    late: &'a SwingAnchors,
    /// The view whose downswing the pair is held to.
    sound: &'a SwingAnchors,
    /// Whether `late` is [`align_swings`]' first argument. See the note at its one reader: this is
    /// `seconds_a < seconds_b` recorded, standing in for Python's `late is a`.
    late_is_a: bool,
    /// `sound`'s downswing, in seconds — the reference both clips convert through their own fps.
    seconds: f64,
    /// Where that duration puts `late`'s top, in `late`'s own frame numbering.
    corrected_top: i64,
    /// How far `late`'s detected top sits past `corrected_top`, in its own frames.
    frames_late: i64,
}

impl Arbitration<'_> {
    fn late_label(&self) -> &str {
        self.late.camera_id.as_deref().unwrap_or("one view")
    }

    fn sound_label(&self) -> &str {
        self.sound.camera_id.as_deref().unwrap_or("the other view")
    }
}

/// Which view's top is wrong, when both clips heard the strike. `None` when undecidable.
///
/// M10 closed the windowing and handed one defect forward: the face-on top landed late on five of the
/// eleven bundles then on disk, measuring 0.183-0.267 s of downswing where down-the-line measured
/// 0.367-0.484 s of the same swing. Every consumer inherited it — three of those five scored a
/// `tempo` that *failed* — and until M11 nothing could say which view was wrong, because the two
/// disagreeing durations were measured against two independently inferred impacts.
///
/// **The strike is the arbiter.** With tau=2 pinned in both clips to a sound both microphones heard,
/// the two downswings become two measurements of one interval in real time. `impact_measured` on both
/// is therefore the entry condition and not a nicety.
///
/// **The shorter downswing is the late top, and the asymmetry behind that is mechanical rather than
/// statistical.** `phases::top_and_impact` puts the top at the start of the major rising run, and the
/// failure the drawdown floor documents is that run *fragmenting*: one wobble splits the descent and
/// the later fragment is taken, which shortens the downswing. Nothing in the rule can move a top the
/// other way, because a candidate run must carry 80% of the largest rise in the clip.
///
/// Returns `None` rather than guessing when either clip is anchored on pose, when either has no fps
/// to compare in real time, or when the two durations already agree inside `DOWNSWING_AGREEMENT` —
/// the ordinary case, where there is nothing to arbitrate (ADR-013).
///
/// **Every one of the fifteen corpus vectors takes that last exit**, which is the P7 finding worth
/// carrying: the defect this was built for has been repaired upstream. See the module doc.
fn arbitrate_tops<'a>(a: &'a SwingAnchors, b: &'a SwingAnchors) -> Option<Arbitration<'a>> {
    if !(a.impact_measured && b.impact_measured) {
        return None;
    }
    let fps_a = a.fps.filter(|r| *r != 0.0)?;
    let fps_b = b.fps.filter(|r| *r != 0.0)?;

    let seconds_a = a.downswing_frames() as f64 / fps_a;
    let seconds_b = b.downswing_frames() as f64 / fps_b;
    if relative_gap(seconds_a, seconds_b) <= DOWNSWING_AGREEMENT {
        return None;
    }

    let late_is_a = seconds_a < seconds_b;
    let (late, sound, seconds) = if late_is_a {
        (a, b, seconds_b)
    } else {
        (b, a, seconds_a)
    };
    let corrected_top = top_at(late, seconds);
    Some(Arbitration {
        late,
        sound,
        late_is_a,
        seconds,
        corrected_top,
        frames_late: late.top - corrected_top,
    })
}

/// The frame sitting `seconds` back from this clip's impact, in its own frame numbering.
///
/// Clamped to a downswing of at least one frame, because [`tau_of_frame`] divides by it. Converting a
/// duration rather than copying a frame index is what lets one reference serve two clips filmed at
/// different rates (ADR-013).
///
/// One of `alignment.py`'s ten frame-index roundings, so [`round_index`] rather than `f64::round`
/// (ADR-032 §3). Unlike [`with_measured_impact`]'s, this one multiplies a *measured* duration by a
/// frame rate and there is no argument that it cannot tie.
fn top_at(anchors: &SwingAnchors, seconds: f64) -> i64 {
    (anchors.impact - round_index(seconds * anchors.fps.unwrap_or(0.0)))
        .max(0)
        .min(anchors.impact - 1)
}

/// What the late view's tempo ratio reads on the arbitrated top, when it has one.
///
/// The alignment scores nothing and this does not change that — `SwingResult` keeps the ratio the
/// face-on phases produced, and M11 P7's brief was to *report* the correction rather than substitute
/// it silently. But the late top is precisely why three bundles reported 4.92, 6.08 and 6.09:1: the
/// ratio's denominator is the very duration being corrected here, so a note that moves the top
/// without saying what that does to the number leaves the reader to redo the arithmetic against a
/// score the results page is showing them in red.
///
/// Empty string when there is no before-and-after to state — no measurable backswing on one side of
/// the correction or the other — rather than a sentence about nothing.
fn tempo_restated(arbitration: &Arbitration) -> String {
    let backswing = arbitration.corrected_top - arbitration.late.motion_start;
    let downswing = arbitration.late.impact - arbitration.corrected_top;
    let Some(before) = arbitration.late.tempo_ratio() else {
        return String::new();
    };
    if backswing <= 0 || downswing <= 0 {
        return String::new();
    }
    format!(
        ", where its backswing reads {}:1 rather than {}:1",
        fixed(backswing as f64 / downswing as f64, 2),
        fixed(before, 2),
    )
}

/// A tau=1 anchor for each clip at a *shared* downswing duration, or `(None, None)`.
///
/// Returns `None` for both unless the two views disagree about the downswing by more than
/// `DOWNSWING_AGREEMENT` — the common case is that they agree and the detected tops stand. When they
/// do not, both clips are held to one duration, converted through each clip's *own* fps so that both
/// panels advance at their native rate and meet at impact.
///
/// **This is the repair, and `pair_frames`' guard is the backstop** — the guard being one of the
/// things this phase did not port, because nothing in `conformance.py::run_vector` reaches it. This
/// rule can move an anchor, so it needs to know which one to move and fires only where that is
/// decidable; the guard cannot repair anything and fires wherever the resulting playback speed would
/// be wrong. Neither subsumes the other, and a pair this one declines is still rendered at native
/// rate.
///
/// **Which duration depends on whether the pair has a shared clock.** With one, [`arbitrate_tops`]
/// names the late top and the reference becomes the *other* view's, so the correction moves the top
/// that is wrong. Without one there is nothing to decide with, and the reference stays the face-on
/// clip's on the original grounds: that is the view the phase detector was tuned on and the only one
/// scored (ADR-015).
///
/// **There is no plausibility veto on the reference, and removing it is what fixed the renders.**
/// `phases::POSSIBLE_DOWNSWING_S` used to guard both routes, which is right about the *anchor* and
/// backwards about the *render*: declining re-imposes the two detected tops, which are known to
/// disagree, and the schedule then has no way to express that except as playback speed — measured at
/// 1.48x, 2.08x, 2.70x and 3.11x on four bundles. So the constant is read here and decides nothing;
/// it only warns.
///
/// `arbitration` is decided by the caller and passed in rather than taken here, because
/// [`align_swings`] also stamps it onto both clips and the two must not be able to disagree about the
/// same pair. [M11 P8]
fn shared_tops(
    a: &SwingAnchors,
    b: &SwingAnchors,
    notes: &mut Vec<String>,
    arbitration: Option<&Arbitration>,
) -> (Option<i64>, Option<i64>) {
    let (Some(fps_a), Some(fps_b)) = (a.fps.filter(|r| *r != 0.0), b.fps.filter(|r| *r != 0.0))
    else {
        return (None, None);
    };

    let seconds_a = a.downswing_frames() as f64 / fps_a;
    let seconds_b = b.downswing_frames() as f64 / fps_b;
    // Deliberately **not** the render tolerance, though the gap between two durations is exactly the
    // follower's playback speed error. Returning non-`None` here sends [`align_swings`] to
    // `ImpactOnly`, and that tier is a claim about *evidence* — a pair whose downswings sit 25% apart
    // has still had its top independently found in both views. Lowering this trigger re-labelled real
    // bundles to buy a fix the render schedule makes for free.
    if relative_gap(seconds_a, seconds_b) <= DOWNSWING_AGREEMENT {
        return (None, None);
    }

    let label_a = a.camera_id.as_deref().unwrap_or("a");
    let label_b = b.camera_id.as_deref().unwrap_or("b");
    let opening = format!(
        "downswing durations disagree ({label_a} {}s vs {label_b} {}s)",
        fixed(seconds_a, 3),
        fixed(seconds_b, 3),
    );

    let (reference, diagnosis, remedy) = match arbitration {
        None => {
            let reference = if a.camera_id.as_deref() == Some(FACE_ON_CAMERA) {
                seconds_a
            } else if b.camera_id.as_deref() == Some(FACE_ON_CAMERA) {
                seconds_b
            } else {
                seconds_a
            };
            (
                reference,
                "one view's top is wrong".to_string(),
                format!(
                    "Holding both panels to {}s back from impact so neither is replayed at the \
                     wrong speed; the tops may sit a frame or two apart on screen",
                    fixed(reference, 3)
                ),
            )
        }
        Some(arbitration) => (
            arbitration.seconds,
            format!(
                "both views heard the strike, so tau=2 is one instant in real time and the tops are \
                 the only thing left to disagree — {}'s is {} frames late",
                arbitration.late_label(),
                arbitration.frames_late,
            ),
            format!(
                "Holding both panels to {}'s {}s back from impact, which moves {}'s top to frame \
                 {}{}",
                arbitration.sound_label(),
                fixed(arbitration.seconds, 3),
                arbitration.late_label(),
                arbitration.corrected_top,
                tempo_restated(arbitration),
            ),
        ),
    };

    // A correction that does not fit inside the clip is not one this rule can make: [`top_at`]
    // clamps, and a top clamped to frame 0 is the start of the recording rather than the instant
    // asked for. Declining is safe here in a way it was not before the render schedule grew its own
    // guard — that guard holds the pair at native rate whatever this returns, so what is given up is
    // the banner placement and never the playback speed.
    if [a, b]
        .into_iter()
        .any(|anchors| anchors.impact - round_index(reference * anchors.fps.unwrap_or(0.0)) <= 0)
    {
        notes.push(format!(
            "{opening} — {diagnosis}. But {}s back from impact falls outside one of the clips, so \
             there is no shared top to impose — the panels are held at their native rate instead \
             and the tops will sit apart on screen",
            fixed(reference, 3)
        ));
        return (None, None);
    }

    let (low, high) = POSSIBLE_DOWNSWING_S;
    let caution = if low <= reference && reference <= high {
        String::new()
    } else {
        format!(
            ". Note that {}s is not a downswing any golfer makes, so the top banner is likely wrong \
             in both panels — but holding them to it is what keeps either from being replayed at a \
             speed its camera never shot",
            fixed(reference, 3)
        )
    };
    notes.push(format!("{opening} — {diagnosis}. {remedy}{caution}"));
    (Some(top_at(a, reference)), Some(top_at(b, reference)))
}

/// One clip's place on the shared tau axis. `_clip_alignment`.
fn clip_alignment(
    anchors: &SwingAnchors,
    motion_start: i64,
    top: Option<i64>,
    top_late_by: Option<i64>,
) -> ClipAlignment {
    let last = anchors
        .frame_count
        .filter(|count| *count != 0)
        .map_or(anchors.impact, |count| count - 1);
    ClipAlignment {
        anchors: anchors.clone(),
        warp_motion_start: motion_start,
        warp_top: top,
        // `> 0` rather than a presence test: [`arbitrate_tops`] only fires past
        // `DOWNSWING_AGREEMENT`, so a zero here would be a rounding artefact of [`top_at`] rather
        // than a top that is late by nothing — and the field's `gt(0)` would reject it anyway.
        top_late_by: top_late_by.filter(|frames| *frames > 0),
        tau_start: tau_of_frame(anchors, 0.0, Some(motion_start), top),
        tau_end: tau_of_frame(anchors, last as f64, Some(motion_start), top),
    }
}

/// The bounded tour-median estimate `phases::motion_start` falls back to, applied here too.
///
/// Reusing that constant rather than picking a second one keeps a clip whose motion start was
/// estimated by the detector and one whose anchor was refused here on the *same* footing.
fn estimated_motion_start(anchors: &SwingAnchors) -> i64 {
    (anchors.top - round_index(FALLBACK_TEMPO_RATIO * anchors.downswing_frames() as f64)).max(0)
}

/// The tau=0 fallback for both clips at a *shared* backswing duration.
///
/// Applying [`estimated_motion_start`] to each clip separately shares the tour-median **ratio**,
/// which is not the same thing as degrading symmetrically: the two views routinely disagree about the
/// downswing by 10-40%, and [`FALLBACK_TEMPO_RATIO`] multiplies that disagreement by 3.5 before it
/// reaches the screen. Over the bundles on disk that put the two panels' tau=0 up to 0.300 s apart,
/// and the render opens *before* tau=0, so it began with the gap grown another 40%.
///
/// So derive one duration in seconds and convert it through each clip's own fps, exactly as
/// [`shared_tops`] does for the tops. The reference is the face-on clip for the same reason it is
/// there: that is the view the phase detector was tuned on and the only one scored (ADR-015).
///
/// Without fps on both clips there is no duration to share, and this falls back to the per-clip ratio
/// — degraded further, but reported rather than raised (ADR-013). **No committed vector reaches that
/// fallback**: every corpus clip carries a frame rate.
fn shared_motion_starts(a: &SwingAnchors, b: &SwingAnchors) -> (i64, i64) {
    let (Some(fps_a), Some(fps_b)) = (a.fps.filter(|r| *r != 0.0), b.fps.filter(|r| *r != 0.0))
    else {
        return (estimated_motion_start(a), estimated_motion_start(b));
    };

    // Python's `a.camera_id == "face_on" or b.camera_id != "face_on"`, which is [`shared_tops`]'
    // three-way choice written as one condition: take `a` when it is the face-on view, or when
    // neither is.
    let reference = if a.camera_id.as_deref() == Some(FACE_ON_CAMERA)
        || b.camera_id.as_deref() != Some(FACE_ON_CAMERA)
    {
        a.downswing_frames() as f64 / fps_a
    } else {
        b.downswing_frames() as f64 / fps_b
    };
    let backswing_seconds = FALLBACK_TEMPO_RATIO * reference;
    (
        (a.top - round_index(backswing_seconds * fps_a)).max(0),
        (b.top - round_index(backswing_seconds * fps_b)).max(0),
    )
}

/// Symmetric relative difference, so the comparison does not depend on argument order.
fn relative_gap(x: f64, y: f64) -> f64 {
    let larger = x.abs().max(y.abs());
    if larger > 0.0 {
        (x - y).abs() / larger
    } else {
        0.0
    }
}

/// The tau range both clips actually cover.
///
/// `f64::max`/`f64::min` rather than Python's `max`/`min`, which return the *first* of two equal
/// arguments — the P4 tie-break edge. It cannot bite here: the two rules differ only on a signed-zero
/// tie, and `tau_of_frame(_, 0)` is `1.0 - pivot/rate`, which reaches `0.0` and never `-0.0`.
fn overlap(
    a: &SwingAnchors,
    motion_a: i64,
    top_a: Option<i64>,
    b: &SwingAnchors,
    motion_b: i64,
    top_b: Option<i64>,
) -> (f64, f64) {
    let last_a = a
        .frame_count
        .filter(|count| *count != 0)
        .map_or(a.impact, |count| count - 1);
    let last_b = b
        .frame_count
        .filter(|count| *count != 0)
        .map_or(b.impact, |count| count - 1);
    let low = tau_of_frame(a, 0.0, Some(motion_a), top_a).max(tau_of_frame(
        b,
        0.0,
        Some(motion_b),
        top_b,
    ));
    let high = tau_of_frame(a, last_a as f64, Some(motion_a), top_a).min(tau_of_frame(
        b,
        last_b as f64,
        Some(motion_b),
        top_b,
    ));
    (low, high)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phases::TRAIL_WRIST;
    use crate::testing::{a_rear_swinging_body, a_swinging_body, phase, put, with_preroll};

    fn anchors() -> SwingAnchors {
        SwingAnchors {
            motion_start: 10,
            top: 40,
            impact: 50,
            motion_start_detected: true,
            impact_measured: false,
            camera_id: None,
            frame_count: Some(100),
            fps: Some(60.0),
        }
    }

    /// A segmentation whose three read phases sit where `segment_phases` puts them.
    fn segmentation() -> Vec<PhaseSegment> {
        vec![
            phase(SwingPhase::Address, 0, 10, true),
            phase(SwingPhase::Backswing, 10, 37, true),
            phase(SwingPhase::Transition, 37, 43, true),
            phase(SwingPhase::Downswing, 43, 50, true),
            phase(SwingPhase::Impact, 50, 52, true),
            phase(SwingPhase::FollowThrough, 52, 70, true),
        ]
    }

    #[test]
    fn the_anchors_are_the_boundary_chains_own_three_instants() {
        let clip = ClipMetadata {
            fps: Some(59.94),
            width: None,
            height: None,
            frame_count: Some(70),
            source_sha256: None,
        };
        let got = anchors_from_phases(&segmentation(), Some(&clip), Some("face-on"), 0)
            .expect("the fixture segments");
        assert_eq!(got.motion_start, 10);
        assert_eq!(got.top, 40); // (37 + 43) / 2, the transition's midpoint
        assert_eq!(got.impact, 50);
        assert_eq!(got.camera_id.as_deref(), Some("face-on"));
        assert_eq!(got.frame_count, Some(70));
        assert_eq!(got.fps, Some(59.94));
        assert!(!got.impact_measured);
        assert!(got.motion_start_detected);
    }

    /// `motion_start_detected` is `address.detected` and not a constant, which **nothing in P6 can
    /// see**: its only reader is `align_swings`, and a bundle with one camera serializes
    /// `alignment: null`, so the field never reaches a compared payload. Hard-coding it `true`
    /// passes every vector and every other test here.
    ///
    /// It matters because it inverts an ADR-013 disclosure: `false` means the boundary was the
    /// tour-median *estimate* rather than something found in the signal, and the whole point of
    /// carrying it is that two clips guessing separately do not agree, so the warp may not use it as
    /// a shared anchor. P7 gates it; this stands there until then.
    #[test]
    fn an_estimated_motion_start_is_carried_as_estimated() {
        let mut phases = segmentation();
        phases[0] = phase(SwingPhase::Address, 0, 10, false);
        let got = anchors_from_phases(&phases, None, None, 0).expect("still segments");
        assert!(!got.motion_start_detected);
        // And the estimate is still returned — it is the best available number, it just may not be
        // used as a shared anchor.
        assert_eq!(got.motion_start, 10);
    }

    /// A clip with no clip metadata still yields anchors — they just carry no fps, which is what
    /// makes [`with_measured_impact`] a no-op on them.
    #[test]
    fn missing_clip_metadata_costs_the_fps_and_nothing_else() {
        let got = anchors_from_phases(&segmentation(), None, None, 0).expect("still segments");
        assert_eq!(got.fps, None);
        assert_eq!(got.frame_count, None);
        assert_eq!(with_measured_impact(&got, Some(&[49])), got);
    }

    #[test]
    fn a_missing_phase_refuses_rather_than_guessing_an_instant() {
        for absent in [
            SwingPhase::Address,
            SwingPhase::Transition,
            SwingPhase::Impact,
        ] {
            let phases: Vec<PhaseSegment> = segmentation()
                .into_iter()
                .filter(|segment| segment.phase != absent)
                .collect();
            assert!(
                anchors_from_phases(&phases, None, None, 0).is_none(),
                "{absent:?}"
            );
        }
    }

    /// A downswing of zero length is the denominator of the whole tau axis, so it is a refusal
    /// rather than a fixup. The guard is `impact.start_frame <= top`, and the boundary case — impact
    /// landing exactly on the midpoint — is the one worth pinning.
    #[test]
    fn a_collapsed_downswing_is_a_refusal() {
        let mut phases = segmentation();
        phases[4] = phase(SwingPhase::Impact, 40, 42, true);
        assert!(anchors_from_phases(&phases, None, None, 0).is_none());
        phases[4] = phase(SwingPhase::Impact, 41, 42, true);
        assert!(anchors_from_phases(&phases, None, None, 0).is_some());
    }

    #[test]
    fn an_offset_moves_every_index_into_whole_clip_coordinates() {
        let got = anchors_from_phases(&segmentation(), None, None, 40).expect("segments");
        assert_eq!((got.motion_start, got.top, got.impact), (50, 80, 90));
    }

    /// The pass-through, which is the only branch the six synthetic vectors reach: no strikes means
    /// the anchors come back untouched and `impact_measured` stays false.
    #[test]
    fn nothing_to_pin_to_leaves_the_estimate_alone() {
        let base = anchors();
        for strikes in [None, Some(&[][..])] {
            let got = with_measured_impact(&base, strikes);
            assert_eq!(got, base);
            assert!(!got.impact_measured);
        }
        // And no fps means no window to size, whatever was heard.
        let unclocked = SwingAnchors {
            fps: None,
            ..base.clone()
        };
        assert_eq!(with_measured_impact(&unclocked, Some(&[49])), unclocked);
    }

    /// The earliest candidate inside the window wins, not the nearest — the ball is the first sound
    /// a shot makes and the screen strike is the loudest one after it.
    #[test]
    fn the_earliest_strike_wins_over_the_nearest() {
        // At 60 fps the window is 12 frames, so 44 and 51 are both candidates. 51 is nearer.
        let got = with_measured_impact(&anchors(), Some(&[51, 44]));
        assert_eq!(got.impact, 44);
        assert!(got.impact_measured);
    }

    /// A transient that would break `SwingAnchors`' own ordering is a different event, not a
    /// correction — so it is filtered rather than left to a validator that `model_copy` skips.
    #[test]
    fn a_strike_at_or_before_the_top_is_not_a_correction() {
        let base = anchors();
        assert_eq!(with_measured_impact(&base, Some(&[40])), base);
        assert_eq!(with_measured_impact(&base, Some(&[39])), base);
        assert_eq!(with_measured_impact(&base, Some(&[41])).impact, 41);
    }

    /// Nor is one past the last frame of the clip.
    #[test]
    fn a_strike_past_the_end_of_the_clip_is_refused() {
        let short = SwingAnchors {
            frame_count: Some(52),
            ..anchors()
        };
        assert_eq!(with_measured_impact(&short, Some(&[55])), short);
        assert_eq!(with_measured_impact(&short, Some(&[51])).impact, 51);
        // `frame_count == 52` means frame 51 is the last one, so the bound is inclusive there.
        assert_eq!(with_measured_impact(&short, Some(&[52])), short);
    }

    /// Outside the tolerance window the pose estimate stands. At 60 fps the window is
    /// `round(0.20 * 60) = 12` frames, so 62 is in and 63 is out.
    #[test]
    fn the_window_is_the_strike_tolerance_in_frames() {
        let base = anchors();
        assert_eq!(with_measured_impact(&base, Some(&[62])).impact, 62);
        assert_eq!(with_measured_impact(&base, Some(&[63])), base);
    }

    /// A zero `frame_count` is falsy in Python and must not become "the last frame is -1", which
    /// would refuse every candidate. Unreachable from a parsed artifact and one character away.
    #[test]
    fn a_zero_frame_count_is_no_frame_count() {
        let unknown = SwingAnchors {
            frame_count: Some(0),
            ..anchors()
        };
        assert_eq!(with_measured_impact(&unknown, Some(&[49])).impact, 49);
    }

    // ------------------------------------------------------------------- P7, the second view
    //
    // The `alignment` stage gates the anchors and the warp on the fifteen corpus vectors, and the
    // module doc lists what that leaves out. Everything below stands in for one of those absences:
    // three of the five quality tiers, both halves of `shared_tops`, all of `arbitrate_tops` and the
    // sentences hanging off them, `estimated_motion_start`, and `anchors_from_keypoints`' window.

    /// One clip's anchors at 60 fps, which makes a frame 1/60 s and every duration below readable.
    fn view(camera: &str, motion_start: i64, top: i64, impact: i64) -> SwingAnchors {
        SwingAnchors {
            motion_start,
            top,
            impact,
            motion_start_detected: true,
            impact_measured: false,
            camera_id: Some(camera.to_string()),
            frame_count: Some(200),
            fps: Some(60.0),
        }
    }

    fn heard(anchors: &SwingAnchors) -> SwingAnchors {
        SwingAnchors {
            impact_measured: true,
            ..anchors.clone()
        }
    }

    /// A pair the soft anchor is accepted on: same tempo, same backswing, same downswing.
    fn agreeing_pair() -> (SwingAnchors, SwingAnchors) {
        (
            view("face_on", 10, 40, 50),
            view("down_the_line", 11, 41, 51),
        )
    }

    /// A pair whose downswings disagree by 37.5%, which is what everything past
    /// [`DOWNSWING_AGREEMENT`] needs — and what no committed vector supplies.
    ///
    /// `face_on` measures 10 frames of downswing where `down_the_line` measures 16, so at 60 fps
    /// that is 0.167 s against 0.267 s.
    fn disagreeing_pair() -> (SwingAnchors, SwingAnchors) {
        (
            view("face_on", 10, 40, 50),
            view("down_the_line", 8, 40, 56),
        )
    }

    // ----------------------------------------------------------------- anchors_from_keypoints

    /// A window is segmented on its own and every index comes back in the **whole clip's**
    /// coordinates, so a caller never tracks the offset.
    #[test]
    fn a_window_is_segmented_alone_and_shifted_back() {
        let swing = a_swinging_body(60);
        let clip = with_preroll(40, &swing);
        let windowed = anchors_from_keypoints(&clip, None, Some((40, 100)), None)
            .expect("the window holds the swing");

        // The windowed answer is the 60-frame swing's own anchors with 40 added to each, which is
        // the whole claim: the slice is segmented alone and then shifted.
        let bare = anchors_from_keypoints(&swing, None, None, None).expect("the swing segments");
        assert_eq!(windowed.motion_start, bare.motion_start + 40);
        assert_eq!(windowed.top, bare.top + 40);
        assert_eq!(windowed.impact, bare.impact + 40);
        // **The unwindowed clip finds the same three instants here**, because one pre-roll and one
        // swing gives `segment_phases` nothing else to lock onto. So what this test can check is the
        // shift, and "the window picked the right swing out of several" is what it cannot —
        // `spec/vectors/synthetic/windowed.json` has exactly the same limitation one module over.
        let whole = anchors_from_keypoints(&clip, None, None, None).expect("it still segments");
        assert_eq!(
            (whole.motion_start, whole.top, whole.impact),
            (windowed.motion_start, windowed.top, windowed.impact)
        );
    }

    /// The window's `frame_count` is the **whole** keypoint list's length, not the window's.
    ///
    /// Python's comment at that line says the opposite and its code says this; the code is right,
    /// because `offset` has already put every index into whole-clip coordinates and `frame_count` is
    /// what those are clamped against. A port that trusted the comment would clamp a frame-95 impact
    /// against a 60-frame window.
    #[test]
    fn a_window_keeps_the_whole_clips_frame_count() {
        let clip = with_preroll(40, &a_swinging_body(60));
        let metadata = ClipMetadata {
            fps: Some(59.94),
            width: None,
            height: None,
            // Deliberately wrong, so the answer shows which of the two it took.
            frame_count: Some(7),
            source_sha256: None,
        };
        let got = anchors_from_keypoints(&clip, Some(&metadata), Some((40, 100)), None)
            .expect("the window holds the swing");
        assert_eq!(got.frame_count, Some(clip.len() as i64));
        // And the rest of the metadata is copied rather than rebuilt.
        assert_eq!(got.fps, Some(59.94));
        // With no window the container's own count stands, wrong or not.
        let unwindowed = anchors_from_keypoints(&clip, Some(&metadata), None, None)
            .expect("the whole clip segments");
        assert_eq!(unwindowed.frame_count, Some(7));
    }

    /// An empty or inverted window **refuses**, where `engine::windowed` ignores one.
    ///
    /// The two are one line apart in behaviour and the difference is easy to port away by accident:
    /// that one is choosing frames to score and has a clip either way, while a window naming no
    /// frames here has no swing to find anchors in.
    #[test]
    fn an_empty_window_refuses_rather_than_falling_back_to_the_clip() {
        let clip = with_preroll(40, &a_swinging_body(60));
        for empty in [(20, 20), (60, 10), (900, 901)] {
            assert!(
                anchors_from_keypoints(&clip, None, Some(empty), None).is_none(),
                "{empty:?}"
            );
        }
        // A negative start clamps instead of refusing.
        assert!(anchors_from_keypoints(&clip, None, Some((-5, 100)), None).is_some());
    }

    /// The wrist argument is the **only** place the two views are told apart, and it changes the
    /// answer: a rear clip whose trail wrist swings segments on `TRAIL_WRIST` and not on the default.
    #[test]
    fn the_wrist_argument_decides_which_view_this_is() {
        let rear: Vec<FrameKeypoints> = a_rear_swinging_body(60)
            .into_iter()
            .map(|mut frame| {
                // Freeze the lead wrist, which is what the torso does to it from behind.
                put(&mut frame, PoseLandmark::LeftWrist, 0.52, 0.55, 1.0);
                frame
            })
            .collect();
        assert!(anchors_from_keypoints(&rear, None, None, None).is_none());
        assert!(anchors_from_keypoints(&rear, None, None, Some(TRAIL_WRIST)).is_some());
    }

    /// `camera_id` is the first frame **inside the window** that recorded one.
    #[test]
    fn the_camera_id_comes_off_the_windowed_frames() {
        let mut clip = with_preroll(40, &a_swinging_body(60));
        for (at, frame) in clip.iter_mut().enumerate() {
            frame.camera_id = Some(if at < 40 { "preroll" } else { "down_the_line" }.to_string());
        }
        let got = anchors_from_keypoints(&clip, None, Some((40, 100)), None).expect("segments");
        assert_eq!(got.camera_id.as_deref(), Some("down_the_line"));
    }

    // ------------------------------------------------------------------------- tau_of_frame

    /// The axis is pinned at its three anchors and linear between and beyond them.
    #[test]
    fn tau_is_zero_one_and_two_at_the_three_anchors() {
        let anchors = view("face_on", 10, 40, 50);
        assert!((tau_of_frame(&anchors, 10.0, None, None) - 0.0).abs() < 1e-12);
        assert!((tau_of_frame(&anchors, 40.0, None, None) - 1.0).abs() < 1e-12);
        assert!((tau_of_frame(&anchors, 50.0, None, None) - 2.0).abs() < 1e-12);
        // Past impact runs on at the downswing rate, which is the whole reason the axis is defined
        // this way: one unit of tau is one downswing there too.
        assert!((tau_of_frame(&anchors, 60.0, None, None) - 3.0).abs() < 1e-12);
        // And before motion start at the backswing rate, so a clip that rolls early reads negative.
        assert!((tau_of_frame(&anchors, 0.0, None, None) + 1.0 / 3.0).abs() < 1e-12);
    }

    /// A clip that opens **at** the top has no backswing to measure, so the pre-top region runs at
    /// the downswing rate rather than dividing by zero.
    #[test]
    fn no_backswing_falls_back_to_the_downswing_rate() {
        let anchors = view("face_on", 40, 40, 50);
        assert!((tau_of_frame(&anchors, 30.0, None, None) - 0.0).abs() < 1e-12);
        assert!(tau_of_frame(&anchors, 0.0, None, None).is_finite());
    }

    /// Both overrides substitute for their anchor and for nothing else, which is how the soft-anchor
    /// fallback and the `ImpactOnly` tier reuse one function.
    #[test]
    fn the_overrides_replace_the_anchors_they_name() {
        let anchors = view("face_on", 10, 40, 50);
        assert!((tau_of_frame(&anchors, 20.0, Some(20), None) - 0.0).abs() < 1e-12);
        assert!((tau_of_frame(&anchors, 34.0, None, Some(34)) - 1.0).abs() < 1e-12);
    }

    // -------------------------------------------------------------------------- align_swings

    /// The ordinary pair: both anchors kept, nothing to say, and `a` is the clip it was handed first.
    #[test]
    fn an_agreeing_pair_keeps_every_anchor_and_says_nothing() {
        let (a, b) = agreeing_pair();
        let got = align_swings(&a, &b);
        assert_eq!(got.quality, AlignmentQuality::Full);
        assert!(got.notes.is_empty());
        let clip_a = got.a.as_ref().expect("both clips are present");
        assert_eq!(clip_a.warp_motion_start, a.motion_start);
        assert_eq!(clip_a.warp_top, None);
        assert_eq!(clip_a.top_late_by, None);
        assert_eq!(got.b.as_ref().unwrap().warp_motion_start, b.motion_start);
    }

    /// An **estimated** motion start on either clip drops the soft anchor for both, and each clip
    /// that estimated says so under its own name.
    ///
    /// `motion_start_detected` is true on all thirty committed clips, so this branch has no vector
    /// behind it — and hard-coding the field `true` in `anchors_from_phases` passes every one of
    /// them, which P6 already recorded one layer down.
    #[test]
    fn an_estimated_motion_start_drops_the_soft_anchor_for_both_clips() {
        let (a, b) = agreeing_pair();
        let estimated = SwingAnchors {
            motion_start_detected: false,
            ..b.clone()
        };
        let got = align_swings(&a, &estimated);
        assert_eq!(got.quality, AlignmentQuality::TopImpact);
        assert_eq!(
            got.notes,
            vec!["down_the_line: motion start was estimated, not detected"]
        );
        // Both clips move, not just the one that estimated: the point is that they degrade by the
        // same number of *seconds*.
        assert_ne!(got.a.as_ref().unwrap().warp_motion_start, a.motion_start);

        // And both clips estimating produces both notes, in argument order.
        let both = align_swings(
            &SwingAnchors {
                motion_start_detected: false,
                ..a.clone()
            },
            &estimated,
        );
        assert_eq!(both.notes.len(), 2);
        assert!(both.notes[0].starts_with("face_on:"));
    }

    /// A clip with no measurable backswing drops the soft anchor and names itself.
    #[test]
    fn a_clip_with_no_backswing_drops_the_soft_anchor() {
        let (a, b) = agreeing_pair();
        let collapsed = view("down_the_line", 41, 41, 51);
        let got = align_swings(&a, &collapsed);
        assert_eq!(got.quality, AlignmentQuality::TopImpact);
        assert_eq!(
            got.notes,
            vec!["down_the_line: no measurable backswing; motion start dropped as an anchor"]
        );
        let _ = b;
    }

    /// A backswing shorter than its own downswing is not a golf swing, and the sentence says why.
    ///
    /// This one **is** gated — four corpus vectors produce it, all on the rear clip — so it is pinned
    /// here byte for byte as the readable copy of an answer that otherwise only exists gzipped.
    #[test]
    fn an_implausible_tempo_drops_the_soft_anchor_with_the_reason() {
        // Both clips measure 20 frames of downswing, so this refusal is the *only* note: a fixture
        // whose downswings also disagreed would add `shared_tops`' sentence underneath and the
        // equality below would be testing two things at once.
        let a = view("face_on", 10, 60, 80);
        // 1 frame of backswing against 20 of downswing: 0.05 downswings, which is what four corpus
        // vectors read on their rear clip.
        let collapsed = view("down_the_line", 39, 40, 60);
        let got = align_swings(&a, &collapsed);
        assert_eq!(
            got.notes,
            vec![
                "down_the_line: backswing measures 0.05 downswings, which no golf swing does — \
                 motion start has collapsed onto the top (the pause at the top reads as the \
                 'quiet' stretch the takeaway is measured back to). Using the tour-median estimate \
                 instead"
            ]
        );
    }

    /// Tempo ratios that disagree past 35% refuse the soft anchor, and the note says which boundary
    /// to doubt: the downswings agree, so it is the takeaway.
    #[test]
    fn disagreeing_tempo_ratios_blame_the_takeaway_when_the_downswings_agree() {
        let a = view("face_on", 10, 40, 50); // 3.00:1
        let b = view("down_the_line", 30, 40, 51); // 0.91... no — see below
        let b = SwingAnchors {
            motion_start: 29,
            ..b
        }; // 11 frames of backswing against 11 of downswing: 1.00:1
        let got = align_swings(&a, &b);
        assert_eq!(got.quality, AlignmentQuality::TopImpact);
        assert_eq!(got.notes.len(), 1);
        assert_eq!(
            got.notes[0],
            "tempo ratios disagree (3.00 vs 1.00) — frame rate cancels out of a ratio, so two views \
             of one swing should not. The two downswings agree (0.167s and 0.183s), so this is the \
             takeaway boundary, not two different swings — down_the_line is finding its motion \
             start late. Dropping it as an anchor; the swing itself is fine"
        );
    }

    /// The ratio check's blind spot: two views whose ratios agree and whose backswings are a fifth of
    /// a second apart in real time.
    ///
    /// Three corpus vectors produce this sentence, which makes it the second of the four gated ones.
    #[test]
    fn agreeing_ratios_with_disagreeing_backswings_are_still_refused() {
        // **The two ratios agree within the bound rather than exactly, and they have to.** If they
        // were equal *and* the downswings agreed in seconds, the backswings would agree too — the
        // ratio is the quotient. So the reachable case is a pair inside `TEMPO_AGREEMENT`'s 0.35
        // whose backswings are still a quarter of a second apart, which is the shape all three
        // corpus vectors that produce this sentence have.
        let a = view("face_on", 0, 60, 80); // 1.000s backswing, 0.333s downswing, 3.00:1
        let b = view("down_the_line", 15, 60, 82); // 0.750s backswing, 0.367s downswing, 2.05:1
        let got = align_swings(&a, &b);
        assert_eq!(got.quality, AlignmentQuality::TopImpact);
        assert_eq!(
            got.notes,
            vec![
                "the two views' backswings are 0.250s apart (1.000s and 0.750s) even though their \
                 tempo ratios agree — a ratio divides out the downswing, so it cannot see two views \
                 whose errors scale together. The two downswings agree (0.333s and 0.367s), so this \
                 is the takeaway boundary, not two different swings — down_the_line is finding its \
                 motion start late. Dropping it as an anchor; the swing itself is fine"
            ]
        );
    }

    /// A pair already refused on tempo does not collect a second note about the same disagreement.
    ///
    /// Python's `elif`, and the reason it is one: the backswing check would fire on most pairs the
    /// tempo check rejects, and two sentences about one fact only crowd the first.
    #[test]
    fn a_pair_refused_on_tempo_is_not_refused_twice() {
        let a = view("face_on", 10, 40, 50);
        let b = SwingAnchors {
            motion_start: 29,
            ..view("down_the_line", 30, 40, 51)
        };
        assert_eq!(align_swings(&a, &b).notes.len(), 1);
    }

    /// Without a frame rate on both clips there is no duration to compare, so the soft anchor stands
    /// on the ratio alone — reported, not raised (ADR-013).
    #[test]
    fn a_clip_with_no_frame_rate_skips_the_duration_cross_check() {
        let a = view("face_on", 10, 40, 50);
        let b = SwingAnchors {
            fps: None,
            ..view("down_the_line", 10, 40, 50)
        };
        // Identical ratios and no way to notice the durations differ, so this passes where the test
        // above fails.
        let got = align_swings(&a, &b);
        assert_eq!(got.quality, AlignmentQuality::Full);
        assert!(got.notes.is_empty());
    }

    /// Both clips hearing the strike is `Synchronized`, and it **overwrites** the anchor count.
    #[test]
    fn a_pair_that_both_heard_the_strike_is_synchronized() {
        let (a, b) = agreeing_pair();
        let got = align_swings(&heard(&a), &heard(&b));
        assert_eq!(got.quality, AlignmentQuality::Synchronized);
        assert!(!got.quality.is_degraded());

        // Even from the worst non-failing tier, which is the inversion `_synchronized` exists to
        // stop: one measured anchor is better evidence than three estimated ones.
        let (a, b) = disagreeing_pair();
        let degraded = align_swings(&a, &b);
        assert_eq!(degraded.quality, AlignmentQuality::ImpactOnly);
        assert_eq!(
            align_swings(&heard(&a), &heard(&b)).quality,
            AlignmentQuality::Synchronized
        );
    }

    /// Half a pair is worth a note and no tier, and the note names the view that could not
    /// contribute.
    ///
    /// Unreachable from `spec/vectors/`: all fifteen corpus pairs heard the strike in both clips.
    #[test]
    fn one_clip_hearing_the_strike_is_a_note_and_not_a_shared_clock() {
        let (a, b) = agreeing_pair();
        let got = align_swings(&heard(&a), &b);
        assert_eq!(got.quality, AlignmentQuality::Full, "no tier is earned");
        assert_eq!(
            got.notes,
            vec![
                "face_on's impact was measured on the ball strike but down_the_line's was not, so \
                 the two clips are aligned on inferred instants as before — a shared clock needs \
                 the strike in both"
            ]
        );
        // The other way round names the other view.
        assert!(align_swings(&a, &heard(&b)).notes[0].starts_with("down_the_line's impact was"));
    }

    /// Downswings that disagree past 30% hold **both** clips to one duration, which is the
    /// `ImpactOnly` tier — and no committed vector reaches it.
    ///
    /// Without a shared clock there is nothing to arbitrate, so the reference is the face-on clip's
    /// on ADR-015's grounds: that is the view the detector was tuned on and the only one scored.
    #[test]
    fn disagreeing_downswings_impose_a_shared_top() {
        let (a, b) = disagreeing_pair();
        let got = align_swings(&a, &b);
        assert_eq!(got.quality, AlignmentQuality::ImpactOnly);
        // The face-on clip's 10-frame downswing is the reference, so both tops land 10 frames back
        // from their own impacts.
        assert_eq!(got.a.as_ref().unwrap().warp_top, Some(40));
        assert_eq!(got.b.as_ref().unwrap().warp_top, Some(46));
        // Nothing was arbitrated, so no clip carries a finding — the render is fixed and nothing has
        // been proved about the golfer.
        assert_eq!(got.a.as_ref().unwrap().top_late_by, None);
        assert_eq!(got.b.as_ref().unwrap().top_late_by, None);
        assert!(got.notes[0].starts_with(
            "downswing durations disagree (face_on 0.167s vs down_the_line 0.267s) — one view's \
             top is wrong. Holding both panels to 0.167s back from impact"
        ));
    }

    /// With a shared clock the disagreement is **settled**: the shorter downswing is the late top,
    /// the other view's duration is the reference, and the note restates the tempo it fixes.
    ///
    /// The whole of `arbitrate_tops`, `top_at` and `tempo_restated` in one sentence, pinned byte for
    /// byte because every one of them is unreachable from `spec/vectors/`.
    #[test]
    fn a_shared_clock_names_the_late_top_and_restates_its_tempo() {
        let (a, b) = disagreeing_pair();
        let got = align_swings(&heard(&a), &heard(&b));

        // `face_on` measures the shorter downswing, so its top is the late one — by 6 frames, since
        // 0.267 s back from its own impact is frame 34 and it detected 40.
        assert_eq!(got.a.as_ref().unwrap().top_late_by, Some(6));
        assert_eq!(got.b.as_ref().unwrap().top_late_by, None);
        assert!(got.a.as_ref().unwrap().top_is_late());
        assert_eq!(got.a.as_ref().unwrap().warp_top, Some(34));
        assert_eq!(got.b.as_ref().unwrap().warp_top, Some(40));
        assert_eq!(
            got.notes,
            vec![
                "downswing durations disagree (face_on 0.167s vs down_the_line 0.267s) — both \
                 views heard the strike, so tau=2 is one instant in real time and the tops are the \
                 only thing left to disagree — face_on's is 6 frames late. Holding both panels to \
                 down_the_line's 0.267s back from impact, which moves face_on's top to frame 34, \
                 where its backswing reads 1.50:1 rather than 3.00:1"
            ]
        );
    }

    /// The finding lands on whichever clip is late, not on `a`.
    ///
    /// Python decides it with `late is a` over an object identity; the port records
    /// `seconds_a < seconds_b` instead, and this is the half of that equivalence a reversed pair
    /// checks.
    #[test]
    fn the_finding_follows_the_late_clip_rather_than_the_first_argument() {
        let (a, b) = disagreeing_pair();
        let got = align_swings(&heard(&b), &heard(&a));
        assert_eq!(got.a.as_ref().unwrap().top_late_by, None);
        assert_eq!(got.b.as_ref().unwrap().top_late_by, Some(6));
    }

    /// A reference that does not fit inside one of the clips is declined, with a note saying so.
    ///
    /// `top_at` clamps, and a top clamped to frame 0 is the start of the recording rather than the
    /// instant asked for — so the panels are held at their native rate instead.
    #[test]
    fn a_reference_that_falls_outside_a_clip_is_declined() {
        // The rear clip's impact is at frame 12, and the face-on reference is 16 frames.
        let a = view("down_the_line", 0, 2, 12);
        let b = view("face_on", 10, 40, 56);
        let got = align_swings(&a, &b);
        assert_eq!(got.a.as_ref().unwrap().warp_top, None);
        assert_eq!(got.b.as_ref().unwrap().warp_top, None);
        assert_ne!(
            got.quality,
            AlignmentQuality::ImpactOnly,
            "declining leaves the tier where the anchor count put it"
        );
        assert!(
            got.notes.iter().any(|note| {
                note.contains(
                "0.267s back from impact falls outside one of the clips, so there is no shared top \
                 to impose"
            )
            }),
            "{:?}",
            got.notes
        );
    }

    /// A reference outside the band a downswing can occupy is **warned about and used anyway**.
    ///
    /// The old rule vetoed the correction here, and that is exactly what left four bundles replaying
    /// a panel at up to 3.11x: declining re-imposes two tops that are known to disagree. So
    /// `phases::POSSIBLE_DOWNSWING_S` is read and decides nothing.
    #[test]
    fn an_impossible_reference_earns_a_caution_and_is_imposed_regardless() {
        // 3 frames of downswing at 60 fps is 0.05 s, under the 0.12 s floor.
        let a = view("face_on", 10, 40, 43);
        let b = view("down_the_line", 10, 40, 50);
        let got = align_swings(&a, &b);
        assert_eq!(got.a.as_ref().unwrap().warp_top, Some(40));
        assert_eq!(got.b.as_ref().unwrap().warp_top, Some(47));
        assert!(
            got.notes.iter().any(|note| note.ends_with(
                ". Note that 0.050s is not a downswing any golfer makes, so the top banner is \
                 likely wrong in both panels — but holding them to it is what keeps either from \
                 being replayed at a speed its camera never shot"
            )),
            "{:?}",
            got.notes
        );
    }

    /// When the downswings disagree too and nothing can arbitrate them, the note falls back to
    /// "different swings" — the reading this corpus has never once justified, and the one it keeps
    /// because a pair that heard nothing cannot rule it out.
    #[test]
    fn disagreeing_downswings_with_no_shared_clock_suspect_two_swings() {
        let a = view("face_on", 10, 40, 50); // 3.00:1, 0.167s downswing
        let b = view("down_the_line", 24, 40, 56); // 1.00:1, 0.267s downswing
        let got = align_swings(&a, &b);
        assert!(
            got.notes[0].ends_with(
                "Most likely the two clips are showing DIFFERENT swings; check for a practice swing \
                 in one of them"
            ),
            "{:?}",
            got.notes[0]
        );
        // And a pair with no frame rate at all takes the same exit, for want of a duration.
        let unclocked = SwingAnchors {
            fps: None,
            ..b.clone()
        };
        assert!(align_swings(&a, &unclocked).notes[0].ends_with("in one of them"));
    }

    /// The tau=0 fallback shares one **duration** rather than one ratio, converted through each
    /// clip's own fps.
    ///
    /// Applying the tour-median ratio per clip shares the ratio, which is not the same thing as
    /// degrading symmetrically: the two views disagree about the downswing by 10-40%, and 3.5x
    /// multiplies that before it reaches the screen.
    #[test]
    fn the_shared_motion_start_is_one_duration_through_two_frame_rates() {
        let a = view("face_on", 10, 40, 50);
        let b = SwingAnchors {
            fps: Some(120.0),
            ..view("down_the_line", 20, 80, 100)
        };
        // Refuse the soft anchor on an estimated boundary, so the fallback runs.
        let got = align_swings(
            &a,
            &SwingAnchors {
                motion_start_detected: false,
                ..b.clone()
            },
        );
        // The face-on downswing is 10 frames at 60 fps = 0.167 s; 3.5 of those is 0.583 s, which is
        // 35 frames for `a` and 70 for `b`.
        assert_eq!(got.a.as_ref().unwrap().warp_motion_start, 40 - 35);
        assert_eq!(got.b.as_ref().unwrap().warp_motion_start, 80 - 70);
    }

    /// Without fps on both clips there is no duration to share, so each falls back to the tour-median
    /// **ratio** off its own downswing — degraded further, and reported rather than raised.
    ///
    /// `estimated_motion_start` is reachable through nothing else, and every corpus clip carries a
    /// frame rate.
    #[test]
    fn without_a_frame_rate_each_clip_degrades_off_its_own_downswing() {
        let a = SwingAnchors {
            fps: None,
            motion_start_detected: false,
            ..view("face_on", 10, 40, 50)
        };
        let b = SwingAnchors {
            fps: None,
            ..view("down_the_line", 10, 80, 100)
        };
        let got = align_swings(&a, &b);
        // 3.5 * 10 frames = 35, and 3.5 * 20 = 70 — each off its own downswing.
        assert_eq!(got.a.as_ref().unwrap().warp_motion_start, 40 - 35);
        assert_eq!(got.b.as_ref().unwrap().warp_motion_start, 80 - 70);
        // And the estimate is clamped at frame 0 rather than going negative.
        let short = SwingAnchors {
            fps: None,
            motion_start_detected: false,
            ..view("face_on", 0, 5, 50)
        };
        assert_eq!(
            align_swings(&short, &b)
                .a
                .as_ref()
                .unwrap()
                .warp_motion_start,
            0
        );
    }

    /// Neither clip being the face-on view falls through to `a`, which is the third arm of a
    /// tie-break written as one condition in Python.
    ///
    /// The literal is `"face_on"` and **not** `contracts::placements::FACE_ON`, which is
    /// `"face-on"`. Substituting the placement constant makes both tie-breaks silently never fire —
    /// and passes every committed vector, because `a` is the face-on clip on all fifteen.
    #[test]
    fn the_face_on_tie_break_is_the_camera_id_and_not_the_placement_view() {
        let a = view("face-on", 10, 40, 50);
        let b = view("face_on", 10, 40, 56);
        // `b` is the face-on view here; its 16-frame downswing is 0.267 s and becomes the reference.
        let got = align_swings(&a, &b);
        assert_eq!(got.b.as_ref().unwrap().warp_top, Some(40));
        assert_eq!(got.a.as_ref().unwrap().warp_top, Some(34));
        let held = |notes: &[String], seconds: &str| {
            let clause = format!("Holding both panels to {seconds}s");
            notes.iter().any(|note| note.contains(&clause))
        };
        // Scanned rather than indexed: this pair's tempo ratios disagree too, so `shared_tops`'
        // sentence is the second note and not the first.
        assert!(held(&got.notes, "0.267"), "{:?}", got.notes);
        // With the hyphenated spelling on both, neither matches and `a` wins by fallthrough.
        let neither = align_swings(&a, &view("down-the-line", 10, 40, 56));
        assert!(held(&neither.notes, "0.167"), "{:?}", neither.notes);
    }

    /// `tau_end` is read off the last frame, and a clip with no `frame_count` uses its impact
    /// instead — which is zero-safe, because a zero count is no count.
    #[test]
    fn a_clip_without_a_frame_count_ends_at_its_impact() {
        let (a, b) = agreeing_pair();
        let unknown = SwingAnchors {
            frame_count: None,
            ..a.clone()
        };
        let got = align_swings(&unknown, &b);
        let clip = got.a.as_ref().unwrap();
        assert!((clip.tau_end - 2.0).abs() < 1e-12);
        // A zero is falsy in Python and must not become "the last frame is -1".
        let zero = SwingAnchors {
            frame_count: Some(0),
            ..a.clone()
        };
        let got = align_swings(&zero, &b);
        assert!((got.a.as_ref().unwrap().tau_end - 2.0).abs() < 1e-12);
    }

    /// **`top_at` rounds half-to-even, and the two rules disagree on a reachable frame rate.**
    ///
    /// ADR-032 §3's first edge at one of `alignment.py`'s ten frame-index sites. `with_measured_impact`
    /// multiplies a *constant* by the frame rate and P6 proved that product cannot tie; this one
    /// multiplies a **measured** duration, so it can, and the fixture below is what a tie looks like:
    /// 6.5 frames, where Python's `round` answers 6 and Rust's `f64::round` answers 7. One frame of
    /// difference in where tau=1 is pinned, which moves both panels' top banner and the whole pre-top
    /// region of the warp.
    ///
    /// No committed vector reaches it — the corpus never reaches `top_at` at all — so this is the
    /// site's only coverage.
    #[test]
    fn the_shared_top_rounds_half_to_even() {
        let anchors = SwingAnchors {
            fps: Some(13.0),
            ..view("face_on", 0, 10, 20)
        };
        // 0.5 s at 13 fps is exactly 6.5 frames, and 6.5 is exactly representable.
        assert_eq!(top_at(&anchors, 0.5), 20 - 6, "half-to-even, not half-away");
        // 7.5 ties the other way and both rules agree there, which is what makes a sweep worthless.
        let faster = SwingAnchors {
            fps: Some(15.0),
            ..anchors.clone()
        };
        assert_eq!(top_at(&faster, 0.5), 20 - 8);
        // Both clamps: at least one frame of downswing, and never before frame 0.
        assert_eq!(
            top_at(&anchors, 0.0),
            19,
            "a zero-length downswing is refused"
        );
        assert_eq!(top_at(&anchors, 100.0), 0);
    }

    /// **`estimated_motion_start` rounds half-to-even too**, and a three-frame downswing is a tie.
    ///
    /// `3.5 * 3` is 10.5: Python answers 10 and `f64::round` answers 11. The second site of the same
    /// edge, reachable only through the no-fps fallback, which no committed vector takes either.
    #[test]
    fn the_estimated_motion_start_rounds_half_to_even() {
        let anchors = SwingAnchors {
            fps: None,
            ..view("face_on", 0, 20, 23)
        };
        assert_eq!(estimated_motion_start(&anchors), 20 - 10);
        // A two-frame downswing gives 7.0 and does not tie, so the two rules agree — the difference
        // is confined to odd downswings, which is the enumerable shape P3 recorded for this edge.
        let even = SwingAnchors {
            impact: 22,
            ..anchors.clone()
        };
        assert_eq!(estimated_motion_start(&even), 20 - 7);
    }

    /// The tau=0 fallback's reference is the **face-on** clip's downswing, whichever argument it is.
    ///
    /// The same three-way tie-break `shared_tops` makes, written in Python as one condition — take
    /// `a` when it is the face-on view, or when neither is. Substituting "always `a`" passes every
    /// committed vector, because `a` is the face-on clip on all fifteen.
    #[test]
    fn the_shared_motion_start_reference_is_the_face_on_clip_either_way() {
        // Two clips filmed at different rates whose downswings agree to 17% — inside
        // [`DOWNSWING_AGREEMENT`], so `shared_tops` stands aside and this fallback is what decides
        // tau=0. The rear clip measures 0.200 s of downswing and the face-on one 0.167 s.
        let rear = SwingAnchors {
            fps: Some(120.0),
            ..view("down_the_line", 100, 200, 224)
        };
        let face_on = view("face_on", 50, 100, 110);
        let refused = |anchors: &SwingAnchors| SwingAnchors {
            motion_start_detected: false,
            ..anchors.clone()
        };
        let starts = |warp: &SwingAlignment| {
            (
                warp.a.as_ref().unwrap().warp_motion_start,
                warp.b.as_ref().unwrap().warp_motion_start,
            )
        };

        // The face-on clip's 0.167 s is the reference either way: 3.5 of those is 0.583 s, which is
        // 70 frames at 120 fps and 35 at 60.
        assert_eq!(starts(&align_swings(&refused(&rear), &face_on)), (130, 65));
        assert_eq!(starts(&align_swings(&refused(&face_on), &rear)), (65, 130));

        // With neither clip named `face_on` the first argument wins by fallthrough, so the reference
        // becomes the rear clip's 0.200 s — 0.700 s of backswing, 84 frames and 42.
        let anonymous = |anchors: &SwingAnchors| SwingAnchors {
            camera_id: None,
            ..anchors.clone()
        };
        assert_eq!(
            starts(&align_swings(
                &refused(&anonymous(&rear)),
                &anonymous(&face_on)
            )),
            (116, 58)
        );
    }

    /// A correction that lands **on** the late clip's own motion start restates nothing.
    ///
    /// `tempo_restated` needs a backswing and a downswing on both sides of the corrected top, and a
    /// corrected top sitting exactly on tau=0 leaves no backswing to divide. Writing `< 0` instead of
    /// `<= 0` renders ", where its backswing reads 0.00:1 rather than 3.00:1" — a sentence about
    /// nothing, in a note a golfer reads.
    #[test]
    fn a_correction_onto_the_motion_start_restates_no_tempo() {
        let late = heard(&view("face_on", 34, 40, 50));
        let sound = heard(&view("down_the_line", 8, 40, 56));
        let arbitration = arbitrate_tops(&late, &sound).expect("the pair disagrees enough");
        // 0.267 s back from frame 50 at 60 fps is frame 34, which is this clip's motion start.
        assert_eq!(arbitration.corrected_top, late.motion_start);
        assert_eq!(tempo_restated(&arbitration), "");

        // One frame earlier and the clause appears, so the guard is the only thing suppressing it.
        let with_room = heard(&view("face_on", 33, 40, 50));
        let arbitration = arbitrate_tops(&with_room, &sound).expect("still disagrees");
        assert_eq!(
            tempo_restated(&arbitration),
            ", where its backswing reads 0.06:1 rather than 0.70:1"
        );
    }

    /// A top late by **nothing** is carried as no finding rather than as a zero.
    ///
    /// `arbitrate_tops` only fires past [`DOWNSWING_AGREEMENT`], so a zero here is a rounding
    /// artefact of [`top_at`] and not a top that is late by nothing — and `ClipAlignment.top_late_by`
    /// carries `gt(0)`, so an unfiltered zero would make [`align_swings`]' own `validate` panic
    /// instead of answering. The fixture is contrived because the case is: it needs a one-frame
    /// downswing in the late view and a rear clip whose duration rounds back onto that same frame.
    #[test]
    fn a_top_late_by_nothing_is_no_finding() {
        // 1 frame of downswing at 60 fps: 0.0167 s.
        let late = heard(&view("face_on", 39, 40, 41));
        // 49 frames at 2000 fps: 0.0245 s, a 32% disagreement — and 0.0245 * 60 rounds to 1, so
        // 0.0245 s back from frame 41 is frame 40, which is exactly the detected top.
        let sound = heard(&SwingAnchors {
            fps: Some(2000.0),
            ..view("down_the_line", 100, 200, 249)
        });
        let arbitration = arbitrate_tops(&late, &sound).expect("a 32% disagreement");
        assert_eq!(arbitration.frames_late, 0);

        let got = align_swings(&late, &sound);
        assert_eq!(got.a.as_ref().unwrap().top_late_by, None);
        assert!(!got.a.as_ref().unwrap().top_is_late());
        // The *warp* still moves, which is the separation `ClipAlignment` documents: the render is
        // held to one duration whenever the two disagree, and the finding needs a named culprit.
        assert_eq!(got.a.as_ref().unwrap().warp_top, Some(40));
    }

    /// The overlap is the tau range **both** clips cover — the later start and the earlier end.
    #[test]
    fn the_overlap_is_the_intersection_of_the_two_clips() {
        let a = view("face_on", 10, 40, 50);
        let b = SwingAnchors {
            frame_count: Some(60),
            ..view("down_the_line", 10, 40, 50)
        };
        let (low, high) = align_swings(&a, &b)
            .overlap
            .expect("both clips are present");
        // Both open at frame 0, so the low end is shared; `b` is the shorter clip, so it decides the
        // high end — frame 59 against `a`'s 199.
        assert!((low + 1.0 / 3.0).abs() < 1e-12);
        assert!((high - tau_of_frame(&b, 59.0, Some(10), None)).abs() < 1e-12);
    }
}
