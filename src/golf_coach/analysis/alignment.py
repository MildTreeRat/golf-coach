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

from golf_coach.analysis.phases import (
    _FALLBACK_TEMPO_RATIO,
    _PLAUSIBLE_DOWNSWING_S,
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


# The swing and nothing else: from a little before the takeaway to one downswing past impact.
# Widen it to render more of the clip.
DEFAULT_TAU_RANGE = (-0.4, 3.0)


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

    # The hard anchors get their own check. If the two views disagree about how long the downswing
    # lasted, pinning both to tau=1 resamples one panel to catch up — see `_DOWNSWING_AGREEMENT`.
    top_a, top_b = _shared_tops(a, b, notes)
    if top_a is not None and top_b is not None:
        quality = AlignmentQuality.IMPACT_ONLY
        motion_a = max(0, top_a - round(_FALLBACK_TEMPO_RATIO * (a.impact - top_a)))
        motion_b = max(0, top_b - round(_FALLBACK_TEMPO_RATIO * (b.impact - top_b)))

    return SwingAlignment(
        a=_clip_alignment(a, motion_a, top_a),
        b=_clip_alignment(b, motion_b, top_b),
        quality=quality,
        notes=notes,
        overlap=_overlap(a, motion_a, top_a, b, motion_b, top_b),
    )


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
    return (
        "Most likely the two clips are showing DIFFERENT swings; check for a practice swing in one "
        "of them"
    )


def _shared_tops(
    a: SwingAnchors, b: SwingAnchors, notes: list[str]
) -> tuple[int | None, int | None]:
    """A tau=1 anchor for each clip at a *shared* downswing duration, or `(None, None)`.

    Returns None for both unless the two views disagree about the downswing by more than
    `_DOWNSWING_AGREEMENT` — the common case is that they agree and the detected tops stand. When
    they do not, the reference duration is the face-on clip's, because that is the view the phase
    detector was tuned on (docs/M4_POSE_BAKEOFF.md is a face-on corpus) and the only one scored
    (ADR-015). Each clip converts that duration through its *own* fps, so both panels then advance
    at their native rate and meet at impact.
    """
    if not a.fps or not b.fps:
        return None, None

    seconds_a = a.downswing_frames / a.fps
    seconds_b = b.downswing_frames / b.fps
    if _relative_gap(seconds_a, seconds_b) <= _DOWNSWING_AGREEMENT:
        return None, None

    reference = seconds_a if a.camera_id == "face_on" else (
        seconds_b if b.camera_id == "face_on" else seconds_a
    )
    label_a, label_b = a.camera_id or "a", b.camera_id or "b"
    low, high = _PLAUSIBLE_DOWNSWING_S
    if not low <= reference <= high:
        notes.append(
            f"downswing durations disagree ({label_a} {seconds_a:.3f}s vs {label_b} "
            f"{seconds_b:.3f}s) and the reference view's {reference:.3f}s is not a possible "
            "downswing — leaving the warp in place rather than holding both panels to it"
        )
        return None, None
    notes.append(
        f"downswing durations disagree ({label_a} {seconds_a:.3f}s vs {label_b} {seconds_b:.3f}s) "
        f"— one view's top is wrong. Holding both panels to {reference:.3f}s back from impact so "
        "neither is replayed at the wrong speed; the tops may sit a frame or two apart on screen"
    )
    # Clamp to a downswing of at least one frame: `frame_of_tau` divides by it.
    top_a = min(a.impact - 1, max(0, a.impact - round(reference * a.fps)))
    top_b = min(b.impact - 1, max(0, b.impact - round(reference * b.fps)))
    return top_a, top_b


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

    schedule: list[FramePairing] = []
    for step in range(first, last + 1):
        tau = tau_of_frame(
            lead.anchors, step, motion_start=lead.warp_motion_start, top=lead.warp_top
        )
        follow = alignment.b if reference == "a" else alignment.a
        mapped = _clamp_index(
            frame_of_tau(
                follow.anchors, tau, motion_start=follow.warp_motion_start, top=follow.warp_top
            ),
            count_b if reference == "a" else count_a,
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
    anchors: SwingAnchors, motion_start: int, top: int | None = None
) -> ClipAlignment:
    last = (anchors.frame_count - 1) if anchors.frame_count else anchors.impact
    return ClipAlignment(
        anchors=anchors,
        warp_motion_start=motion_start,
        warp_top=top,
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
