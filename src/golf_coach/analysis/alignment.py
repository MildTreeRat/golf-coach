"""Event-anchored alignment of two views of one swing — pure, stdlib only. [M7 Phase 2]

Two people hold two phones. Nothing about the recordings matches: not the frame rate, not the
clip length, not the moment either person pressed record. There is no shared clock to appeal to,
no clapper, and — for hand-held phones — no calibration, so triangulation is off the table
permanently (ADR-011's 2026-08-05 addendum). What both cameras *do* see is the swing.

So we align on the swing itself. Each clip is segmented **independently** by the existing
`segment_phases()`, which yields three instants, and those instants define a normalized axis:

    tau = 0  motion start        tau = 1  top        tau = 2  impact

Mapping `tau -> frame` is piecewise-linear between the anchors and linear past impact at the
downswing rate. Composing one clip's map with the other's inverse is the whole algorithm. It is
ADR-011's Option C used standalone rather than as a refinement of Option B, and it is immune by
construction to everything two consumer phones will disagree about — including iPhone slo-mo,
which stores 120/240 fps behind a stretched playback rate and would defeat any timestamp-based
approach (docs/M7_TWO_PHONE_SPIKE.md, Q3).

**Not every anchor is worth the same.** Against 461 GolfDB clips (docs/M4_POSE_BAKEOFF.md) impact
lands within a median of 1 frame and the top within 2, but motion start is out by a median of 7
with 40% of clips over 10 frames and an outright fallback on ~14%. It is therefore a **soft**
anchor: used only when both clips found it independently *and* the two agree about the backswing —
as a tempo ratio, and again in seconds, because a ratio cannot see an error the two views share.
Otherwise both clips fall back to the same tour-median estimate and the result says so through
`AlignmentQuality`, rather than rendering a video that implies precision nobody measured.

**Multi-swing clips are handled by selection, not by cleverness.** A phone clip often contains
practice swings; `segment_phases` locates the earliest major descent and will pick a practice one
if it comes first (see `candidate_downswings`). Rather than guess, this module takes an optional
frame `window` per clip and cross-checks the two clips' tempo ratios — a mismatch means the two
views almost certainly locked onto *different* swings, which is the one failure mode that would
otherwise produce a confident, plausible-looking, completely wrong video.

No numpy, no OpenCV, no I/O — contracts in, contracts out, so this imports and tests on the base
install (ADR-008).
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import NamedTuple

from golf_coach.analysis.phases import (
    _FALLBACK_TEMPO_RATIO,
    _POSSIBLE_DOWNSWING_S,
    _STRIKE_TOLERANCE_S,
    segment_phases,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.alignment import (
    TAU_TOP,
    AlignmentQuality,
    ClipAlignment,
    FramePairing,
    SwingAlignment,
    SwingAnchors,
)
from golf_coach.contracts.keypoints import ClipMetadata, FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import PhaseSegment, SwingPhase

# How far apart two clips' backswing:downswing ratios may sit before the soft anchor is refused.
# Frame rate cancels out of a ratio, so two views of ONE swing must agree here whatever the phones
# were set to; disagreement means at least one `motion_start` is wrong, or — worse, and the reason
# this check is not merely a nicety — the two clips locked onto different swings entirely.
#
# 0.35 is deliberately loose. `motion_start`'s median error is 7 frames, which on a ~15-frame
# downswing is most of a tempo unit all by itself, so a tight bound would reject honest pairs. It
# is sized to catch a *category* error (3.1 against 1.4), not to grade agreement.
_TEMPO_AGREEMENT = 0.35

# The cross-check the constant above is structurally unable to make: how far apart the two views'
# *backswing durations* may sit, in seconds, before the soft anchor is refused.
#
# A ratio divides out the downswing. So when both views mismeasure one swing in the same direction
# — the common case, since they are watching the same motion — the ratios agree while the durations
# do not, and `_TEMPO_AGREEMENT` waves through a pair that is visibly apart on screen. That is not a
# loose bound, it is a blind one: no value of `_TEMPO_AGREEMENT` catches this, which is why there is
# a second constant here rather than a tighter first one.
#
# Sized on the four bundles that report `full` on disk (docs/M10_ALIGNMENT_ACCURACY.md B2):
#
#   session  9   1.084s vs 0.851s   0.233s apart   tempo 4.06 / 4.25   must fail
#   session  6   0.984s vs 0.784s   0.200s apart   tempo 2.36 / 2.14   must fail
#   session 11   0.834s vs 0.700s   0.133s apart   tempo 2.50 / 2.00   must pass
#   session  8   0.884s vs 0.867s   0.017s apart   tempo 1.89 / 2.60   must pass
#
# leaving the band (0.133, 0.200). 0.167 is ten frames at 60fps and sits at its log midpoint, a
# third of the width clear either side. Read sessions 9 and 8 against each other: the pair fourteen
# times further apart in real time is the pair whose tempo ratios agree more closely. That inversion
# is the entire argument for measuring this in seconds.
#
# Absolute seconds, where `_TEMPO_AGREEMENT` and `_DOWNSWING_AGREEMENT` are both relative. A
# relative bound here would re-import the scale-blindness being fixed, allowing a slow backswing
# more real drift than a fast one — and the viewer sees the same daylight between the panels either
# way, because what is wrong at tau=0 is an offset and not a rate.
_BACKSWING_AGREEMENT_S = 0.167

# A backswing shorter than its own downswing is not a golf swing. Tour tempo is ~3:1 and the
# slowest credible amateur is still well above 1:1, so a ratio under this means the `motion_start`
# boundary is wrong rather than the swing being unusual — no distribution needed to say so.
#
# It fires on real phone footage for a specific, reproducible reason: `_motion_start` walks back
# from the top looking for the last *quiet* stretch of wrist speed, and a golfer who pauses at the
# top hands it one immediately. The boundary then lands a frame or two below the top and the
# "backswing" measures near zero. The estimate is still returned with `detected=True`, because
# from inside `phases.py` nothing about it looks wrong — it takes two views, or this check, to see
# that it is.
#
# Public because `engine.analyze_swing_bundle` applies the same floor to say out loud that the
# *tempo checkpoint* is untrustworthy on such a swing. The two uses are the same fact read twice:
# a backswing that measures shorter than its downswing means the motion-start boundary is wrong,
# which makes it useless as an alignment anchor and makes the tempo ratio derived from it wrong.
MIN_PLAUSIBLE_TEMPO = 1.0

# How far apart the two views' *downswing durations* may sit, in seconds, before the top is refused
# as an anchor too. Unlike the tempo cross-check above this is a check on the **hard** anchors, so
# it runs whatever happened to the soft one.
#
# The warp's central assumption is that between two anchors the views progress through the swing
# proportionally (ADR-015). That is exactly true only if the instants are exactly right. When they
# are not, forcing both panels to reach tau=1 and tau=2 together does not hide the disagreement — it
# converts it into *playback speed*, resampling the follower to catch up by impact. A viewer reads
# that as one camera running fast, which is worse than a visible seam: it silently misrepresents
# tempo and sequencing, the two things the side-by-side exists to show.
#
# 0.30 sits between the two regimes with room on both sides. Top and impact are located to a median
# of 2 and 1 frames, so honest disagreement on a ~15-frame downswing runs to maybe 20%; the failure
# this catches measured 0.234s against 0.400s, a gap of 0.42.
_DOWNSWING_AGREEMENT = 0.30

# How far the follower's implied *playback speed* may sit from 1.0 before the warp is refused and
# both panels are held to one duration instead. Measured as `_relative_gap` against 1.0, so 0.10 is
# a follower running 1.11x fast or 0.90x slow.
#
# **This is a separate constant from `_DOWNSWING_AGREEMENT` on purpose, and the two thresholds
# answer different questions.** That one decides whether a disagreement can be *blamed* on a named
# view — it gates `_arbitrate_tops`, and through `ClipAlignment.top_late_by` it decides whether
# `analysis.engine` retires a checkpoint. This one decides only whether the disagreement may be
# rendered as *speed*, which is never. Sharing one number made the looser scoring question set the
# tighter playback one, and 0.30 admits a panel replayed 1.4x fast in silence.
#
# 0.10 is the anchor precision the bake-off reports, restated as a rate: impact lands within a
# median of 1 frame and the top within 2, so on a ~20-frame 60fps downswing two honest views
# disagree by ~2 frames, which is 10%. Below that the disagreement is not distinguishable from
# anchor noise and warping to it would be chasing it.
#
# The corpus agrees and leaves the value room. Follower playback speed on the 15 stored bundles
# sits at 1.00, 1.04 and 0.95 (gaps 0.00-0.046) and then jumps to 1.18, 1.27, 0.75, 1.48, 2.08,
# 2.70, 3.11 (gaps 0.153-0.68). Nothing lands between 0.046 and 0.153.
_MAX_WARP_SPEED_ERROR = 0.10


# The swing and nothing else: from a little before the takeaway to one downswing past impact.
# Widen it to render more of the clip.
DEFAULT_TAU_RANGE = (-0.4, 3.0)


def with_measured_impact(
    anchors: SwingAnchors, strike_frames: Sequence[int] | None
) -> SwingAnchors:
    """`anchors` with tau=2 pinned to a ball strike heard in this same clip. [M11 P6]

    The one anchor in the system that can be *measured* rather than inferred. Everything else here
    is a pose estimate, including the impact this replaces — and on four of the eleven bundles on
    disk that estimate is 5.7 to 7.5 frames early in the down-the-line view while the face-on view
    is right to within a frame (docs/M11_ACOUSTIC_SYNC.md §E4). Two clips that each pin tau=2 to
    the sound they heard are on a real shared clock, which is what `align_swings` then reports as
    `AlignmentQuality.SYNCHRONIZED`.

    Frames, and frames in **this clip's own numbering** — the same convention `phases._struck`
    takes, and for the same reason: no clip-to-clip offset appears anywhere in `analysis/`, because
    each view is anchored against what its own microphone heard and the shared clock falls out of
    that rather than being computed.

    **The earliest candidate wins, and that rule is doing the work the window cannot.** The bay
    makes four transients per shot and the loudest of them is the ball hitting the impact screen
    85-145 ms later (§E5), while the correction this exists to make runs to 125 ms — so no window
    wide enough to admit the real error is narrow enough to exclude the screen strike, and picking
    the *nearest* transient would take the screen on any clip whose pose impact was already right.
    Ordering resolves what distance cannot: the ball is the first sound a shot makes. The
    club-and-mat pair that precedes it by 15-20 ms is below `audio/impact.py`'s own 50 ms
    separation floor and has already been merged into one onset by the time it arrives here.

    **Anything ahead of the ball wins here by construction, so what is offered matters as much as
    this rule does.** M11 P9 measured a quiet precursor 2-3 frames ahead of the strike being taken
    instead of it, on every clip in the corpus. That is fixed where it was made — `audio/impact.py`
    now drops any candidate under a quarter of the clip's loudest (M11 P11) — and it could only be
    fixed alongside the second error underneath it, a video edit-list offset that ran the other way
    on four clips and had been partly cancelling it (M11 P10, in `api/pipeline.py`).
    With both landed, this rule picks a frame that agrees with contact by eye to within one on all
    ten clips checked frame by frame. Nothing here changed for either fix, and that is the point of
    the seam: this function takes frames and has no opinion about how they were measured. Story and
    frame numbers in docs/M11_ACOUSTIC_SYNC.md §Addendum, P10 and P11.

    Returns `anchors` **unchanged** when there is nothing to pin to — no strikes, no fps to size
    the window with, or no transient inside it — and `impact_measured` then stays False, which is
    the honest reading: not measured is not the same as measured at the pose estimate (ADR-010 §2).
    """
    if not strike_frames or not anchors.fps:
        return anchors

    window = round(_STRIKE_TOLERANCE_S * anchors.fps)
    last = (anchors.frame_count - 1) if anchors.frame_count else None
    candidates = [
        frame
        for frame in strike_frames
        if abs(frame - anchors.impact) <= window
        # `SwingAnchors` requires impact strictly after the top and inside the clip. A transient
        # that would break either is not a correction, it is a different event: on a fast downswing
        # the window reaches back past the top all by itself (face-on measures 0.183 s of downswing
        # on M10's four offenders, against a 0.20 s window), so this is a live branch and not a
        # defensive one.
        and frame > anchors.top
        and (last is None or frame <= last)
    ]
    if not candidates:
        return anchors

    # `model_copy` does not re-run the validator, which is why the two ordering conditions are
    # enforced in the filter above rather than left to it.
    return anchors.model_copy(update={"impact": min(candidates), "impact_measured": True})


def anchors_from_phases(
    phases: list[PhaseSegment],
    *,
    clip: ClipMetadata | None = None,
    camera_id: str | None = None,
    offset: int = 0,
) -> SwingAnchors | None:
    """The three anchors, read off the boundary chain `segment_phases()` produces.

    Returns None for a clip that could not be segmented at all, or whose segmentation is
    degenerate (no downswing to divide by) — reported, not raised (ADR-013).

    `top` is the midpoint of the TRANSITION segment, which recovers the detected top exactly
    unless the +/-3-frame window around it was clamped by a neighbouring boundary (`phases.py`,
    `_TRANSITION_HALF_FRAMES`); that clamp only binds on clips too short to hold the window, which
    are degenerate for alignment anyway. It is the same derivation `scripts/analyze_swing.py` uses
    to place its overlay banners, and both now read it from here — so the frame the banner is
    stamped on and the frame the warp pins to tau=1 cannot drift apart.

    `offset` shifts every index back into the coordinates of the *unsliced* clip, for callers that
    segmented a window of a multi-swing recording.
    """
    by_phase = {segment.phase: segment for segment in phases}
    address = by_phase.get(SwingPhase.ADDRESS)
    transition = by_phase.get(SwingPhase.TRANSITION)
    impact = by_phase.get(SwingPhase.IMPACT)
    if address is None or transition is None or impact is None:
        return None

    top = (transition.start_frame + transition.end_frame) // 2
    if impact.start_frame <= top:
        return None

    return SwingAnchors(
        motion_start=address.end_frame + offset,
        top=top + offset,
        impact=impact.start_frame + offset,
        motion_start_detected=address.detected,
        camera_id=camera_id,
        frame_count=clip.frame_count if clip is not None else None,
        fps=clip.fps if clip is not None else None,
    )


def anchors_from_keypoints(
    keypoints: list[FrameKeypoints],
    *,
    clip: ClipMetadata | None = None,
    window: tuple[int, int] | None = None,
    wrist: PoseLandmark | None = None,
) -> SwingAnchors | None:
    """Smooth, segment and extract anchors — the whole per-clip path in one call.

    Smoothing first is not optional: `segment_phases` expects it (`engine.analyze_swing` does the
    same before calling it), and raw MediaPipe landmarks jitter enough to move the top.

    `window` restricts the search to `[start, end)` so a clip containing practice swings can be
    pointed at the real one. The slice is segmented on its own and the resulting indices are
    shifted back, so every frame number this returns is in the original clip's coordinates and a
    caller never has to track the offset itself.
    """
    frames = keypoints
    offset = 0
    if window is not None:
        start, end = window
        start = max(0, start)
        end = min(len(keypoints), end)
        if end - start <= 0:
            return None
        frames = keypoints[start:end]
        offset = start

    camera_id = next((f.camera_id for f in frames if f.camera_id is not None), None)
    # The window's own length is what indices are clamped against downstream, not the whole clip.
    metadata = clip
    if window is not None and clip is not None:
        metadata = clip.model_copy(update={"frame_count": len(keypoints)})

    return anchors_from_phases(
        segment_phases(smooth_keypoints(frames), wrist) if wrist is not None
        else segment_phases(smooth_keypoints(frames)),
        clip=metadata,
        camera_id=camera_id,
        offset=offset,
    )


def frame_of_tau(
    anchors: SwingAnchors, tau: float, *, motion_start: int | None = None, top: int | None = None
) -> float:
    """Where `tau` falls in this clip, as a (fractional) frame index.

    Piecewise-linear through the anchors, and linear outside them: **past impact at the downswing
    rate**, before motion start at the backswing rate. Extrapolating the follow-through at the
    downswing rate is the honest choice — it is the last rate actually measured, and the hands do
    keep moving at broadly that speed into the finish. It is also the region where alignment
    matters least for viewing.

    `motion_start` overrides the tau=0 anchor, which is how the soft-anchor fallback substitutes
    the same estimate into both clips. `top` overrides tau=1 the same way, which is how the
    IMPACT_ONLY tier gives both clips a shared downswing duration measured back from impact.
    """
    zero = anchors.motion_start if motion_start is None else motion_start
    pivot = anchors.top if top is None else top
    downswing = float(anchors.impact - pivot)
    backswing = float(pivot - zero)

    if tau >= TAU_TOP:
        # Covers [1, 2] and everything past impact with one expression: both run at the
        # downswing rate, which is exactly why the axis is defined this way.
        return pivot + (tau - TAU_TOP) * downswing

    # Below the top. With no backswing to measure (a clip opening at the top), fall back to the
    # downswing rate rather than dividing by zero — degraded, and flagged by the quality tier.
    rate = backswing if backswing > 0.0 else downswing
    return pivot - (TAU_TOP - tau) * rate


def tau_of_frame(
    anchors: SwingAnchors, frame: float, *, motion_start: int | None = None, top: int | None = None
) -> float:
    """The exact inverse of `frame_of_tau` — what swing-instant this frame shows."""
    zero = anchors.motion_start if motion_start is None else motion_start
    pivot = anchors.top if top is None else top
    downswing = float(anchors.impact - pivot)
    backswing = float(pivot - zero)

    if frame >= pivot:
        return TAU_TOP + (frame - pivot) / downswing

    rate = backswing if backswing > 0.0 else downswing
    return TAU_TOP - (pivot - frame) / rate


def align_swings(a: SwingAnchors, b: SwingAnchors) -> SwingAlignment:
    """Put two clips of one swing on a shared tau axis.

    Both clips are always aligned on **top and impact** — the two anchors the bake-off says are
    worth trusting. Motion start joins them only when both clips detected it independently and the
    two backswings agree twice over: as tempo ratios, and — when both clips reported a frame rate —
    as durations in seconds, which is the disagreement a ratio is blind to.

    When it is refused, both clips take the tour-median estimate off **one shared downswing
    duration** and convert it through their own fps, so the pre-top region degrades by the same
    number of *seconds* in each panel. A clip without fps is the exception and still degrades off
    its own downswing — see `_shared_motion_starts`.

    Above all of that sits one case that is not a count of anchors at all: when both clips arrive
    with `impact_measured` — tau=2 pinned to a strike each one *heard*, via `with_measured_impact`
    — the pair has a real shared clock and reports `SYNCHRONIZED`. See `_synchronized`.

    That shared clock is also what lets a disagreement about the *top* be settled rather than
    merely reported: with tau=2 fixed to one instant in real time, the shorter downswing is the
    late top and both panels are held to the longer one. See `_arbitrate_tops`. [M11 P7]

    **The finding outlives the correction.** Which top is late is recorded on each clip as
    `ClipAlignment.top_is_late` whether or not the warp went on to move it, because the two answer
    different questions — the warp asks *can this be replayed honestly*, and the flag asks *was the
    instant a checkpoint was timed from contradicted*. What makes them come apart is the shared
    clock: the warp holds both panels to one duration on either route, while the flag is set only
    where two *measured* impacts let `_arbitrate_tops` name which view is wrong. A pair with no
    shared clock therefore carries a corrected warp and no finding, which is right — the render is
    fixed and nothing has been proved about the golfer. It is the flag rather than the warp that
    `analysis.engine` reads before retiring a score
    (`contracts.unscored.CROSS_VIEW_CONTRADICTED`). [M11 P8]
    """
    notes: list[str] = []
    quality = AlignmentQuality.FULL

    use_soft = True
    for anchors, side in ((a, "a"), (b, "b")):
        if not anchors.motion_start_detected:
            use_soft = False
            label = anchors.camera_id or side
            notes.append(f"{label}: motion start was estimated, not detected")

    if use_soft:
        for anchors, side in ((a, "a"), (b, "b")):
            ratio = anchors.tempo_ratio
            label = anchors.camera_id or side
            if ratio is None:
                use_soft = False
                notes.append(f"{label}: no measurable backswing; motion start dropped as an anchor")
            elif ratio < MIN_PLAUSIBLE_TEMPO:
                use_soft = False
                notes.append(
                    f"{label}: backswing measures {ratio:.2f} downswings, which no golf swing "
                    "does — motion start has collapsed onto the top (the pause at the top reads "
                    "as the 'quiet' stretch the takeaway is measured back to). Using the "
                    "tour-median estimate instead"
                )

    if use_soft:
        ratio_a, ratio_b = a.tempo_ratio, b.tempo_ratio
        assert ratio_a is not None and ratio_b is not None  # both checked just above
        if _relative_gap(ratio_a, ratio_b) > _TEMPO_AGREEMENT:
            use_soft = False
            notes.append(_tempo_disagreement_note(a, b, ratio_a, ratio_b))
        elif a.fps and b.fps:
            # The ratio check's blind spot, measured in real time — see `_BACKSWING_AGREEMENT_S`.
            # `elif`, because a pair that already failed on tempo has been refused and a second
            # note about the same disagreement would only crowd the first. Both frame rates or
            # nothing: without one there is no duration to compare, so the soft anchor stands on
            # the ratio alone exactly as it did before (ADR-013, reported not raised).
            backswing_a = a.backswing_frames / a.fps
            backswing_b = b.backswing_frames / b.fps
            if abs(backswing_a - backswing_b) > _BACKSWING_AGREEMENT_S:
                use_soft = False
                notes.append(_backswing_disagreement_note(a, b, backswing_a, backswing_b))

    if use_soft:
        motion_a, motion_b = a.motion_start, b.motion_start
    else:
        quality = AlignmentQuality.TOP_IMPACT
        motion_a, motion_b = _shared_motion_starts(a, b)

    # Which top the shared clock says is late — computed here rather than inside `_shared_tops`
    # because the two outputs part company: the warp only *moves* a top whose reference duration is
    # a possible downswing, while the finding that one of them is late holds either way and is what
    # `analysis.engine` retires a checkpoint on. Deciding it once is also what stops the warp and
    # `ClipAlignment.top_is_late` disagreeing about the same pair. [M11 P8]
    arbitration = _arbitrate_tops(a, b)

    # The hard anchors get their own check. If the two views disagree about how long the downswing
    # lasted, pinning both to tau=1 resamples one panel to catch up — see `_DOWNSWING_AGREEMENT`,
    # and `pair_frames` for the guard that catches whatever this rule declines.
    top_a, top_b = _shared_tops(a, b, notes, arbitration)
    if top_a is not None and top_b is not None:
        quality = AlignmentQuality.IMPACT_ONLY
        motion_a = max(0, top_a - round(_FALLBACK_TEMPO_RATIO * (a.impact - top_a)))
        motion_b = max(0, top_b - round(_FALLBACK_TEMPO_RATIO * (b.impact - top_b)))

    quality = _synchronized(a, b, quality, notes)

    late = None if arbitration is None else arbitration.late
    late_by = None if arbitration is None else arbitration.frames_late
    return SwingAlignment(
        a=_clip_alignment(a, motion_a, top_a, top_late_by=late_by if late is a else None),
        b=_clip_alignment(b, motion_b, top_b, top_late_by=late_by if late is b else None),
        quality=quality,
        notes=notes,
        overlap=_overlap(a, motion_a, top_a, b, motion_b, top_b),
    )


def _synchronized(
    a: SwingAnchors, b: SwingAnchors, quality: AlignmentQuality, notes: list[str]
) -> AlignmentQuality:
    """`SYNCHRONIZED` when both clips heard the strike, else `quality` untouched. [M11 P6]

    **Last, and overwriting whatever the anchor count came to.** A measured tau=2 is better
    evidence than three inferred anchors, so a synchronized pair must not go on reporting
    `IMPACT_ONLY` — that tier is the *worst* non-failing one precisely because its single anchor
    was a guess, and §Design of docs/M11_ACOUSTIC_SYNC.md names the inversion this creates.

    **This function still only sets the label.** The warp is decided above it, and what a measured
    impact does *there* is give `_shared_tops` an arbiter for a disagreement about the top (M11
    P7): the two downswings become two measurements of one interval in real time, so the shorter
    one is the late top rather than merely the face-on one. Both facts come off the same
    `impact_measured` pair and they are deliberately read in two places, because the tier is a
    claim about evidence and the warp is a claim about frames.

    Half a pair is worth a note and no tier. One clip anchored on sound and the other on pose does
    not make a shared clock, and the interesting case is the ordinary one — a phone across the bay
    that heard nothing — so name the view that could not contribute rather than leaving a silent
    `full`.
    """
    if a.impact_measured and b.impact_measured:
        return AlignmentQuality.SYNCHRONIZED
    if a.impact_measured or b.impact_measured:
        heard, deaf = (a, b) if a.impact_measured else (b, a)
        notes.append(
            f"{heard.camera_id or 'one view'}'s impact was measured on the ball strike but "
            f"{deaf.camera_id or 'the other'}'s was not, so the two clips are aligned on inferred "
            "instants as before — a shared clock needs the strike in both"
        )
    return quality


def _tempo_disagreement_note(
    a: SwingAnchors, b: SwingAnchors, ratio_a: float, ratio_b: float
) -> str:
    """Why two views of one swing came out at different tempos."""
    opening = (
        f"tempo ratios disagree ({ratio_a:.2f} vs {ratio_b:.2f}) — frame rate cancels out of a "
        "ratio, so two views of one swing should not. "
    )
    return opening + _which_half_is_wrong(a, b, shorter_backswing_is_a=ratio_a < ratio_b)


def _backswing_disagreement_note(
    a: SwingAnchors, b: SwingAnchors, seconds_a: float, seconds_b: float
) -> str:
    """Why a pair whose tempo ratios agree is refused anyway. [M10 P3]

    This note has to say more than the tempo one, because the reader has just been told nothing is
    wrong: the ratios matched. Lead with the arithmetic reason they could match — a ratio divides
    out the downswing — or the refusal reads as the check being fussy about a pair it already
    approved.
    """
    opening = (
        f"the two views' backswings are {abs(seconds_a - seconds_b):.3f}s apart "
        f"({seconds_a:.3f}s and {seconds_b:.3f}s) even though their tempo ratios agree — a ratio "
        "divides out the downswing, so it cannot see two views whose errors scale together. "
    )
    return opening + _which_half_is_wrong(a, b, shorter_backswing_is_a=seconds_a < seconds_b)


def _which_half_is_wrong(
    a: SwingAnchors, b: SwingAnchors, *, shorter_backswing_is_a: bool
) -> str:
    """Which of the two boundaries to doubt — the denominator tells you.

    Both refusals above are a disagreement about the backswing, so it lives in either the takeaway
    boundary or the top. If the clips also disagree about the *downswing*, the tops are on different
    events and "different swings" is the likeliest reading — a practice swing in one clip is the
    classic cause. But when the downswings agree, the two views are demonstrably watching the same
    motion and the whole difference sits in the takeaway: one clip's motion start is late. Saying
    "different swings" there sends the reader to check something that is fine.

    **The "different swings" reading is the one this corpus has never once justified.** It fires on
    2026-08-23 bundles 4, 7 and 9, and on all three the two views' audio cross-correlates to a
    single strike at r = 0.76-0.83 — one swing, filmed twice, with a bad boundary in one view
    (docs/M11_ACOUSTIC_SYNC.md §E3). It stays as the last resort because a pair that heard nothing
    still cannot rule it out; a pair that heard the *same* strike can, which is the branch below.

    Shared by both notes rather than written twice, because the branch is the same judgement and a
    second copy is a second thing to drift.
    """
    if a.fps and b.fps:
        seconds_a = a.downswing_frames / a.fps
        seconds_b = b.downswing_frames / b.fps
        if _relative_gap(seconds_a, seconds_b) <= _DOWNSWING_AGREEMENT:
            late = (a.camera_id or "a") if shorter_backswing_is_a else (b.camera_id or "b")
            return (
                f"The two downswings agree ({seconds_a:.3f}s and {seconds_b:.3f}s), so this is the "
                f"takeaway boundary, not two different swings — {late} is finding its motion start "
                "late. Dropping it as an anchor; the swing itself is fine"
            )
        arbitration = _arbitrate_tops(a, b)
        if arbitration is not None:
            return (
                f"The two downswings disagree ({seconds_a:.3f}s and {seconds_b:.3f}s) — but both "
                "views heard the strike, so tau=2 is one instant in real time and this is one "
                f"swing filmed twice, not two. {arbitration.late_label}'s top is the late one, by "
                f"{arbitration.frames_late} frames"
            )
    return (
        "Most likely the two clips are showing DIFFERENT swings; check for a practice swing in one "
        "of them"
    )


class _Arbitration(NamedTuple):
    """Which of two disagreeing tops is the wrong one, once a shared clock makes that decidable."""

    #: The view whose top landed late. Its downswing is the shorter of the two.
    late: SwingAnchors
    #: The view whose downswing the pair is held to.
    sound: SwingAnchors
    #: `sound`'s downswing, in seconds — the reference both clips convert through their own fps.
    seconds: float
    #: Where that duration puts `late`'s top, in `late`'s own frame numbering.
    corrected_top: int
    #: How far `late`'s detected top sits past `corrected_top`, in its own frames.
    frames_late: int

    @property
    def late_label(self) -> str:
        return self.late.camera_id or "one view"

    @property
    def sound_label(self) -> str:
        return self.sound.camera_id or "the other view"


def _arbitrate_tops(a: SwingAnchors, b: SwingAnchors) -> _Arbitration | None:
    """Which view's top is wrong, when both clips heard the strike. `None` when undecidable.

    M10 closed the windowing and handed one defect forward: the face-on top lands late on five of
    the eleven bundles on disk, measuring 0.183-0.267s of downswing where down-the-line measures
    0.367-0.484s of the same swing (docs/M11_ACOUSTIC_SYNC.md §E3). Every consumer inherits it —
    three of those five score a `tempo` that *fails* at 4.92-6.09:1 against a 4.71 ceiling — and
    until now nothing could say which view was wrong, because the two disagreeing durations were
    measured against two independently inferred impacts. Two wrong anchors, one gap, no arbiter.

    **The strike is the arbiter.** With tau=2 pinned in both clips to a sound both microphones
    heard (`with_measured_impact`), the two downswings become two measurements of one interval in
    real time. `impact_measured` on both is therefore the entry condition and not a nicety: on a
    pair of inferred impacts this is exactly the comparison M10 could not settle.

    **The shorter downswing is the late top, and the asymmetry behind that is mechanical rather
    than statistical.** `phases._top_and_impact` puts the top at the start of the major rising run,
    and the failure `_DRAWDOWN_FLOOR` documents is that run *fragmenting* — a golfer who hovers at
    the top gives the lead wrist a nearly-flat stretch, one wobble splits the descent, and the
    later fragment is taken. That shortens the downswing. Nothing in the rule can move a top the
    other way: `_MAJOR_RISE_FRACTION` requires 80% of the largest rise in the clip before a run is
    a candidate at all, so a pre-top wobble cannot be mistaken for the descent. The ground truth
    agrees — on 2026-08-09 swing 2 the two down-the-line wrists read 24 and 25 frames while face-on
    read 14, and the floor moved face-on to 24.

    Returns `None` rather than guessing when either clip is anchored on pose, when either has no
    fps to compare in real time, or when the two durations already agree inside
    `_DOWNSWING_AGREEMENT` — the ordinary case, where there is nothing to arbitrate (ADR-013).
    """
    if not (a.impact_measured and b.impact_measured) or not a.fps or not b.fps:
        return None

    seconds_a = a.downswing_frames / a.fps
    seconds_b = b.downswing_frames / b.fps
    if _relative_gap(seconds_a, seconds_b) <= _DOWNSWING_AGREEMENT:
        return None

    late, sound, seconds = (a, b, seconds_b) if seconds_a < seconds_b else (b, a, seconds_a)
    corrected = _top_at(late, seconds)
    return _Arbitration(late, sound, seconds, corrected, late.top - corrected)


def _top_at(anchors: SwingAnchors, seconds: float) -> int:
    """The frame sitting `seconds` back from this clip's impact, in its own frame numbering.

    Clamped to a downswing of at least one frame, because `frame_of_tau` divides by it. Converting
    a duration rather than copying a frame index is what lets one reference serve two clips filmed
    at different rates (ADR-013).
    """
    return min(anchors.impact - 1, max(0, anchors.impact - round(seconds * (anchors.fps or 0.0))))


def _tempo_restated(arbitration: _Arbitration) -> str:
    """What the late view's tempo ratio reads on the arbitrated top, when it has one.

    The alignment scores nothing and this does not change that — `SwingResult` keeps the ratio the
    face-on phases produced, and P7's brief is to *report* the correction rather than substitute it
    silently. But the late top is precisely why three bundles on disk report 4.92, 6.08 and 6.09:1
    (docs/M11_ACOUSTIC_SYNC.md §E3): the ratio's denominator is the very duration being corrected
    here, so a note that moves the top without saying what that does to the number leaves the
    reader to redo the arithmetic against a score the results page is showing them in red.

    Empty string when there is no before-and-after to state — no measurable backswing on one side
    of the correction or the other — rather than a sentence about nothing.
    """
    before = arbitration.late.tempo_ratio
    backswing = arbitration.corrected_top - arbitration.late.motion_start
    downswing = arbitration.late.impact - arbitration.corrected_top
    if before is None or backswing <= 0 or downswing <= 0:
        return ""
    return f", where its backswing reads {backswing / downswing:.2f}:1 rather than {before:.2f}:1"


def _shared_tops(
    a: SwingAnchors,
    b: SwingAnchors,
    notes: list[str],
    arbitration: _Arbitration | None,
) -> tuple[int | None, int | None]:
    """A tau=1 anchor for each clip at a *shared* downswing duration, or `(None, None)`.

    Returns None for both unless the two views disagree about the downswing by more than
    `_DOWNSWING_AGREEMENT` — the common case is that they agree and the detected tops stand. When
    they do not, both clips are held to one duration, converted through each clip's *own* fps so
    that both panels advance at their native rate and meet at impact.

    **This is the repair, and `pair_frames`' guard is the backstop.** This rule can move an anchor,
    so it needs to know which one to move and fires only where that is decidable; the guard cannot
    repair anything and fires wherever the resulting playback speed would be wrong, including the
    pre-top region this rule never touches. Neither subsumes the other, and a pair this one
    declines is still rendered at native rate.

    **Which duration depends on whether the pair has a shared clock.** With one — both impacts
    pinned to a heard strike — `_arbitrate_tops` names the late top and the reference becomes the
    *other* view's, so the correction moves the top that is wrong and leaves the sound one where it
    was detected. Without one there is nothing to decide with, and the reference stays the face-on
    clip's on the original grounds: that is the view the phase detector was tuned on
    (docs/M4_POSE_BAKEOFF.md is a face-on corpus) and the only one scored (ADR-015). The two rules
    point opposite ways on the bundles this exists for — face-on is the late view on all five of
    them — which is the whole of what P7 changes. [M11 P7]

    **There is no plausibility veto on the reference, and removing it is what fixed the renders.**
    `_POSSIBLE_DOWNSWING_S` used to guard both routes: a reference outside 0.15-0.45s meant the
    sound view's top was suspect too, so neither was imposed and the warp stood. That reasoning is
    right about the *anchor* and backwards about the *render*. Declining is not neutral here — it
    re-imposes the two detected tops, which are known to disagree, and `pair_frames` then has no
    way to express that disagreement except as playback speed. Measured on 2026-08-23/9, whose
    reference missed the 0.45s ceiling by 0.3 ms: the down-the-line panel ran at 1.00x to the top
    and 2.08x from the top to the end of the clip. Bundles 1, 4 and 5 did the same at 1.48x, 3.11x
    and 2.70x.

    So a reference nobody can make is still the better of the two things to hold both panels to,
    because holding them to it costs a top banner a few frames of accuracy and *not* holding them
    to it costs the viewer the tempo and sequencing the side-by-side exists to show. What the old
    veto was protecting — the finding that one view's top is wrong — was never carried by the warp
    in the first place: `ClipAlignment.top_late_by` carries it, `_arbitrate_tops` sets it on its own
    threshold, and `analysis.engine` reads it rather than `warp_top`. No score moved when this
    changed.

    `arbitration` is decided by the caller and passed in rather than taken here, because
    `align_swings` also stamps it onto both clips and the two must not be able to disagree about
    the same pair. [M11 P8]
    """
    if not a.fps or not b.fps:
        return None, None

    seconds_a = a.downswing_frames / a.fps
    seconds_b = b.downswing_frames / b.fps
    # Deliberately **not** `_MAX_WARP_SPEED_ERROR`, though the gap between two durations is exactly
    # the follower's playback speed error. Returning non-None here sends `align_swings` to
    # `IMPACT_ONLY`, and that tier is a claim about *evidence* — a pair whose downswings sit 25%
    # apart has still had its top independently found in both views. Lowering this trigger to the
    # render tolerance re-labelled real bundles (session 8, which reports `full` on disk) to buy a
    # fix `pair_frames` makes for free. The two rules are layered instead: this one repairs the
    # anchor where it can, that one refuses the speed whatever this decided.
    if _relative_gap(seconds_a, seconds_b) <= _DOWNSWING_AGREEMENT:
        return None, None

    label_a, label_b = a.camera_id or "a", b.camera_id or "b"
    opening = (
        f"downswing durations disagree ({label_a} {seconds_a:.3f}s vs {label_b} {seconds_b:.3f}s)"
    )

    if arbitration is None:
        reference = seconds_a if a.camera_id == "face_on" else (
            seconds_b if b.camera_id == "face_on" else seconds_a
        )
        diagnosis = "one view's top is wrong"
        remedy = (
            f"Holding both panels to {reference:.3f}s back from impact so neither is replayed at "
            "the wrong speed; the tops may sit a frame or two apart on screen"
        )
    else:
        reference = arbitration.seconds
        diagnosis = (
            "both views heard the strike, so tau=2 is one instant in real time and the tops are "
            f"the only thing left to disagree — {arbitration.late_label}'s is "
            f"{arbitration.frames_late} frames late"
        )
        remedy = (
            f"Holding both panels to {arbitration.sound_label}'s {reference:.3f}s back from "
            f"impact, which moves {arbitration.late_label}'s top to frame "
            f"{arbitration.corrected_top}{_tempo_restated(arbitration)}"
        )

    # A correction that does not fit inside the clip is not one this rule can make: `_top_at`
    # clamps, and a top clamped to frame 0 is the start of the recording rather than the instant
    # asked for. Declining is safe here in a way it was not before `pair_frames` grew its own
    # guard — that guard holds the pair at native rate whatever this returns, so what is given up
    # is the banner placement and never the playback speed.
    if any(anchors.impact - round(reference * (anchors.fps or 0.0)) <= 0 for anchors in (a, b)):
        notes.append(
            f"{opening} — {diagnosis}. But {reference:.3f}s back from impact falls outside one of "
            "the clips, so there is no shared top to impose — the panels are held at their native "
            "rate instead and the tops will sit apart on screen"
        )
        return None, None

    # `_POSSIBLE_DOWNSWING_S` is read here and *decides nothing* — it used to veto the correction
    # and that is exactly what left four bundles replaying a panel at up to 3.11x (see the
    # docstring). What it is still good for is warning the reader: a reference outside the band
    # means the top being imposed is probably wrong even though the playback speed is now right.
    low, high = _POSSIBLE_DOWNSWING_S
    caution = (
        ""
        if low <= reference <= high
        else (
            f". Note that {reference:.3f}s is not a downswing any golfer makes, so the top banner "
            "is likely wrong in both panels — but holding them to it is what keeps either from "
            "being replayed at a speed its camera never shot"
        )
    )
    notes.append(f"{opening} — {diagnosis}. {remedy}{caution}")
    return _top_at(a, reference), _top_at(b, reference)


def map_frame(alignment: SwingAlignment, frame: int, *, source: str = "a") -> int | None:
    """The frame of the *other* clip showing the same swing instant as `frame`.

    Rounded to the nearest frame and clamped into the target clip's range. None when there is no
    alignment to map through.
    """
    if alignment.a is None or alignment.b is None:
        return None
    if source not in {"a", "b"}:
        raise ValueError(f"source must be 'a' or 'b', got {source!r}")

    origin, target = (
        (alignment.a, alignment.b) if source == "a" else (alignment.b, alignment.a)
    )
    tau = tau_of_frame(
        origin.anchors, frame, motion_start=origin.warp_motion_start, top=origin.warp_top
    )
    mapped = frame_of_tau(
        target.anchors, tau, motion_start=target.warp_motion_start, top=target.warp_top
    )
    return _clamp(round(mapped), target.anchors.frame_count)


def _segment_rates(clip: ClipAlignment) -> tuple[float, float]:
    """`(pre_top, post_top)` — the frames-per-tau-unit `frame_of_tau` uses on either side of tau=1.

    Mirrors that function's own fallback rather than re-deriving it: a clip that opens at the top
    has no backswing to measure and runs the pre-top region at the downswing rate.
    """
    down = float(clip.anchors.impact - clip.top)
    back = float(clip.top - clip.warp_motion_start)
    return (back if back > 0.0 else down), down


def warp_speeds(alignment: SwingAlignment, *, reference: str = "a") -> dict[str, float]:
    """The follower panel's implied playback speed, per linear segment of the warp.

    1.0 is honest — one second of the follower's footage fills one second of output. 2.08 is what
    `2026-08-23/9` shipped through its downswing before `_MAX_WARP_SPEED_ERROR` existed, and it is
    what a viewer reads as one camera running fast.

    **This is the only quantity in the module a viewer can see directly**, which is why it is
    computed here rather than inferred from the anchors at each call site: `pair_frames` guards on
    it and `api.pipeline` reports it, and a second copy of the arithmetic is a second thing to
    drift (the same reason `_which_half_is_wrong` is shared).

    Keys are the two rates `frame_of_tau` actually uses — `"backswing"` below the top, `"downswing"`
    at or above it, which is also the region past impact. Empty when the speed cannot be measured:
    no alignment, or a clip with no fps, because a rate in real time needs one. Empty means *not
    measured*, never *measured at 1.0* (ADR-010 §2).
    """
    if alignment.a is None or alignment.b is None:
        return {}
    if reference not in {"a", "b"}:
        raise ValueError(f"reference must be 'a' or 'b', got {reference!r}")

    lead, follow = (
        (alignment.a, alignment.b) if reference == "a" else (alignment.b, alignment.a)
    )
    fps_lead, fps_follow = lead.anchors.fps, follow.anchors.fps
    if not fps_lead or not fps_follow:
        return {}

    speeds: dict[str, float] = {}
    for name, lead_frames, follow_frames in zip(
        ("backswing", "downswing"), _segment_rates(lead), _segment_rates(follow), strict=True
    ):
        if lead_frames <= 0.0 or follow_frames <= 0.0:
            continue
        speeds[name] = (follow_frames / fps_follow) / (lead_frames / fps_lead)
    return speeds


def pair_frames(
    alignment: SwingAlignment,
    count_a: int,
    count_b: int,
    *,
    reference: str = "a",
    tau_range: tuple[float, float] = DEFAULT_TAU_RANGE,
) -> list[FramePairing]:
    """The output schedule for a side-by-side render: which two frames show each instant.

    The reference clip drives the timeline one frame at a time — its motion stays at its native
    rate — and the other clip is sampled at whatever frame shows the same `tau`. Every entry
    carries that single shared `tau`, so a renderer draws one banner decision per output frame
    and the two panels cannot disagree.

    The schedule is clamped to `tau_range` intersected with what both clips actually cover. The
    honest overlap of two long clips is mostly dead air — a golfer standing over the ball, or the
    bay after they walk off — and rendering all of it buries the thing the video exists to show.
    It is also the region the warp describes worst whenever the soft anchor was refused.

    **A warp that would replay the follower at the wrong speed is refused here, and this is the
    only place it can be.** Everything upstream reasons about anchors; playback speed does not
    exist until frames are put on a schedule, so an anchor disagreement that survives
    `_shared_tops` arrives as an instruction to resample one panel. When `warp_speeds` says any
    segment would run outside `_MAX_WARP_SPEED_ERROR`, the follower is mapped **rigidly** instead:
    pinned to the lead at tau=2 and advancing at its own native rate on either side of it, so the
    two panels drift apart at the top rather than one of them running fast. That drift is a
    visible seam and it is the intended outcome — a seam says "these two instants disagree", which
    is true, where a speed-up says "this golfer swung twice as fast after the top", which is not.

    `_shared_tops` fixes what it can before this ever runs, by moving a top so both panels reach
    impact together at native rate; it needs to know *which* top to move, so it only acts on the
    downswing where a shared clock can tell it. This guard needs to know nothing — it cannot
    repair an anchor, only decline to express the disagreement as speed — which is why it is the
    backstop and not the primary rule. On the fifteen stored bundles it is the pre-top region it
    catches, where an accepted soft anchor still admits `_BACKSWING_AGREEMENT_S` of drift.

    Empty when there is no alignment to map through or the two clips share no overlapping swing
    time — reported, not raised (ADR-013).
    """
    if alignment.a is None or alignment.b is None or alignment.overlap is None:
        return []
    if reference not in {"a", "b"}:
        raise ValueError(f"reference must be 'a' or 'b', got {reference!r}")

    lead, count_lead = (
        (alignment.a, count_a) if reference == "a" else (alignment.b, count_b)
    )
    low = max(alignment.overlap[0], tau_range[0])
    high = min(alignment.overlap[1], tau_range[1])
    first = max(
        0,
        int(
            frame_of_tau(
                lead.anchors, low, motion_start=lead.warp_motion_start, top=lead.warp_top
            )
        ),
    )
    last = min(
        count_lead - 1,
        int(
            frame_of_tau(
                lead.anchors, high, motion_start=lead.warp_motion_start, top=lead.warp_top
            )
        ),
    )
    if last <= first:
        return []

    follow = alignment.b if reference == "a" else alignment.a
    count_follow = count_b if reference == "a" else count_a
    speeds = warp_speeds(alignment, reference=reference)
    # An unmeasurable speed is not a fast one: with no fps there is nothing to guard on, and
    # `warp_speeds` returning {} leaves the warp exactly as it was before this existed.
    rigid = any(_relative_gap(speed, 1.0) > _MAX_WARP_SPEED_ERROR for speed in speeds.values())
    # Native rate for the follower, in follower frames per lead frame. tau=2 is the pin because it
    # is the best-located anchor in the system and, on a `SYNCHRONIZED` pair, the only measured one.
    native = (
        (follow.anchors.fps / lead.anchors.fps)
        if (lead.anchors.fps and follow.anchors.fps)
        else 1.0
    )

    schedule: list[FramePairing] = []
    for step in range(first, last + 1):
        tau = tau_of_frame(
            lead.anchors, step, motion_start=lead.warp_motion_start, top=lead.warp_top
        )
        mapped = _clamp_index(
            follow.anchors.impact + (step - lead.anchors.impact) * native
            if rigid
            else frame_of_tau(
                follow.anchors, tau, motion_start=follow.warp_motion_start, top=follow.warp_top
            ),
            count_follow,
        )
        pairing = (
            FramePairing(tau=tau, frame_a=step, frame_b=mapped)
            if reference == "a"
            else FramePairing(tau=tau, frame_a=mapped, frame_b=step)
        )
        schedule.append(pairing)
    return schedule


def _clamp_index(frame: float, count: int) -> int:
    return max(0, min(count - 1, round(frame)))


def _clip_alignment(
    anchors: SwingAnchors,
    motion_start: int,
    top: int | None = None,
    *,
    top_late_by: int | None = None,
) -> ClipAlignment:
    last = (anchors.frame_count - 1) if anchors.frame_count else anchors.impact
    return ClipAlignment(
        anchors=anchors,
        warp_motion_start=motion_start,
        warp_top=top,
        # `> 0` rather than truthiness: `_arbitrate_tops` only fires past `_DOWNSWING_AGREEMENT`, so
        # a zero here would be a rounding artefact of `_top_at`, not a top that is late by nothing,
        # and the field's `gt=0` would reject it anyway.
        top_late_by=top_late_by if top_late_by and top_late_by > 0 else None,
        tau_start=tau_of_frame(anchors, 0, motion_start=motion_start, top=top),
        tau_end=tau_of_frame(anchors, last, motion_start=motion_start, top=top),
    )


def _estimated_motion_start(anchors: SwingAnchors) -> int:
    """The bounded tour-median estimate `phases._motion_start` falls back to, applied here too.

    Reusing that constant rather than picking a second one keeps a clip whose motion start was
    estimated by the detector and one whose anchor was refused here on the *same* footing.
    """
    return max(0, anchors.top - round(_FALLBACK_TEMPO_RATIO * anchors.downswing_frames))


def _shared_motion_starts(a: SwingAnchors, b: SwingAnchors) -> tuple[int, int]:
    """The tau=0 fallback for both clips at a *shared* backswing duration.

    Applying `_estimated_motion_start` to each clip separately shares the tour-median **ratio**,
    which is not the same thing as degrading symmetrically: the two views routinely disagree about
    the downswing by 10–40%, and `_FALLBACK_TEMPO_RATIO` multiplies that disagreement by 3.5 before
    it reaches the screen. Over the bundles on disk that put the two panels' tau=0 up to 0.300s
    apart, and `DEFAULT_TAU_RANGE` opens *before* tau=0, so the render began with the gap grown
    another 40% — the drift the viewer actually complained about (docs/M10_ALIGNMENT_ACCURACY.md
    §B1).

    So derive one duration in seconds and convert it through each clip's own fps, exactly as
    `_shared_tops` does for the tops. The reference is the face-on clip for the same reason it is
    there: that is the view the phase detector was tuned on and the only one scored (ADR-015).

    Without fps on both clips there is no duration to share, and this falls back to today's
    per-clip ratio — degraded further, but reported rather than raised (ADR-013).
    """
    if not a.fps or not b.fps:
        return _estimated_motion_start(a), _estimated_motion_start(b)

    reference = (
        a.downswing_frames / a.fps
        if a.camera_id == "face_on" or b.camera_id != "face_on"
        else b.downswing_frames / b.fps
    )
    backswing_seconds = _FALLBACK_TEMPO_RATIO * reference
    return (
        max(0, a.top - round(backswing_seconds * a.fps)),
        max(0, b.top - round(backswing_seconds * b.fps)),
    )


def _relative_gap(x: float, y: float) -> float:
    """Symmetric relative difference, so the comparison does not depend on argument order."""
    larger = max(abs(x), abs(y))
    return abs(x - y) / larger if larger > 0.0 else 0.0


def _overlap(
    a: SwingAnchors,
    motion_a: int,
    top_a: int | None,
    b: SwingAnchors,
    motion_b: int,
    top_b: int | None,
) -> tuple[float, float]:
    """The tau range both clips actually cover."""
    last_a = (a.frame_count - 1) if a.frame_count else a.impact
    last_b = (b.frame_count - 1) if b.frame_count else b.impact
    low = max(
        tau_of_frame(a, 0, motion_start=motion_a, top=top_a),
        tau_of_frame(b, 0, motion_start=motion_b, top=top_b),
    )
    high = min(
        tau_of_frame(a, last_a, motion_start=motion_a, top=top_a),
        tau_of_frame(b, last_b, motion_start=motion_b, top=top_b),
    )
    return (low, high)


def _clamp(frame: int, frame_count: int | None) -> int:
    if frame < 0:
        return 0
    if frame_count and frame > frame_count - 1:
        return frame_count - 1
    return frame
