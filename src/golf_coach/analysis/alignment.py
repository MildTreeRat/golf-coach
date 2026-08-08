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
anchor: used only when both clips found it independently *and* the two agree on the swing's tempo.
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

from golf_coach.analysis.phases import _FALLBACK_TEMPO_RATIO, segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.alignment import (
    TAU_TOP,
    AlignmentQuality,
    ClipAlignment,
    FramePairing,
    SwingAlignment,
    SwingAnchors,
)
from golf_coach.contracts.keypoints import ClipMetadata, FrameKeypoints
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
        segment_phases(smooth_keypoints(frames)),
        clip=metadata,
        camera_id=camera_id,
        offset=offset,
    )


def frame_of_tau(anchors: SwingAnchors, tau: float, *, motion_start: int | None = None) -> float:
    """Where `tau` falls in this clip, as a (fractional) frame index.

    Piecewise-linear through the anchors, and linear outside them: **past impact at the downswing
    rate**, before motion start at the backswing rate. Extrapolating the follow-through at the
    downswing rate is the honest choice — it is the last rate actually measured, and the hands do
    keep moving at broadly that speed into the finish. It is also the region where alignment
    matters least for viewing.

    `motion_start` overrides the tau=0 anchor, which is how the soft-anchor fallback substitutes
    the same estimate into both clips.
    """
    zero = anchors.motion_start if motion_start is None else motion_start
    downswing = float(anchors.impact - anchors.top)
    backswing = float(anchors.top - zero)

    if tau >= TAU_TOP:
        # Covers [1, 2] and everything past impact with one expression: both run at the
        # downswing rate, which is exactly why the axis is defined this way.
        return anchors.top + (tau - TAU_TOP) * downswing

    # Below the top. With no backswing to measure (a clip opening at the top), fall back to the
    # downswing rate rather than dividing by zero — degraded, and flagged by the quality tier.
    rate = backswing if backswing > 0.0 else downswing
    return anchors.top - (TAU_TOP - tau) * rate


def tau_of_frame(anchors: SwingAnchors, frame: float, *, motion_start: int | None = None) -> float:
    """The exact inverse of `frame_of_tau` — what swing-instant this frame shows."""
    zero = anchors.motion_start if motion_start is None else motion_start
    downswing = float(anchors.impact - anchors.top)
    backswing = float(anchors.top - zero)

    if frame >= anchors.top:
        return TAU_TOP + (frame - anchors.top) / downswing

    rate = backswing if backswing > 0.0 else downswing
    return TAU_TOP - (anchors.top - frame) / rate


def align_swings(a: SwingAnchors, b: SwingAnchors) -> SwingAlignment:
    """Put two clips of one swing on a shared tau axis.

    Both clips are always aligned on **top and impact** — the two anchors the bake-off says are
    worth trusting. Motion start joins them only when both clips detected it independently and
    their tempo ratios agree; otherwise both clips take the same tour-median estimate, so the
    pre-top region degrades symmetrically instead of in one panel only.
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
            notes.append(
                f"tempo ratios disagree ({ratio_a:.2f} vs {ratio_b:.2f}) — frame rate cancels out "
                "of a ratio, so two views of one swing should not. Most likely the two clips are "
                "showing DIFFERENT swings; check for a practice swing in one of them"
            )

    if use_soft:
        motion_a, motion_b = a.motion_start, b.motion_start
    else:
        quality = AlignmentQuality.TOP_IMPACT
        motion_a = _estimated_motion_start(a)
        motion_b = _estimated_motion_start(b)

    return SwingAlignment(
        a=_clip_alignment(a, motion_a),
        b=_clip_alignment(b, motion_b),
        quality=quality,
        notes=notes,
        overlap=_overlap(a, motion_a, b, motion_b),
    )


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
    tau = tau_of_frame(origin.anchors, frame, motion_start=origin.warp_motion_start)
    mapped = frame_of_tau(target.anchors, tau, motion_start=target.warp_motion_start)
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
    first = max(0, int(frame_of_tau(lead.anchors, low, motion_start=lead.warp_motion_start)))
    last = min(
        count_lead - 1,
        int(frame_of_tau(lead.anchors, high, motion_start=lead.warp_motion_start)),
    )
    if last <= first:
        return []

    schedule: list[FramePairing] = []
    for step in range(first, last + 1):
        tau = tau_of_frame(lead.anchors, step, motion_start=lead.warp_motion_start)
        follow = alignment.b if reference == "a" else alignment.a
        mapped = _clamp_index(
            frame_of_tau(follow.anchors, tau, motion_start=follow.warp_motion_start),
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


def _clip_alignment(anchors: SwingAnchors, motion_start: int) -> ClipAlignment:
    last = (anchors.frame_count - 1) if anchors.frame_count else anchors.impact
    return ClipAlignment(
        anchors=anchors,
        warp_motion_start=motion_start,
        tau_start=tau_of_frame(anchors, 0, motion_start=motion_start),
        tau_end=tau_of_frame(anchors, last, motion_start=motion_start),
    )


def _estimated_motion_start(anchors: SwingAnchors) -> int:
    """The bounded tour-median estimate `phases._motion_start` falls back to, applied here too.

    Reusing that constant rather than picking a second one keeps a clip whose motion start was
    estimated by the detector and one whose anchor was refused here on the *same* footing.
    """
    return max(0, anchors.top - round(_FALLBACK_TEMPO_RATIO * anchors.downswing_frames))


def _relative_gap(x: float, y: float) -> float:
    """Symmetric relative difference, so the comparison does not depend on argument order."""
    larger = max(abs(x), abs(y))
    return abs(x - y) / larger if larger > 0.0 else 0.0


def _overlap(
    a: SwingAnchors, motion_a: int, b: SwingAnchors, motion_b: int
) -> tuple[float, float]:
    """The tau range both clips actually cover."""
    last_a = (a.frame_count - 1) if a.frame_count else a.impact
    last_b = (b.frame_count - 1) if b.frame_count else b.impact
    low = max(
        tau_of_frame(a, 0, motion_start=motion_a), tau_of_frame(b, 0, motion_start=motion_b)
    )
    high = min(
        tau_of_frame(a, last_a, motion_start=motion_a),
        tau_of_frame(b, last_b, motion_start=motion_b),
    )
    return (low, high)


def _clamp(frame: int, frame_count: int | None) -> int:
    if frame < 0:
        return 0
    if frame_count and frame > frame_count - 1:
        return frame_count - 1
    return frame
