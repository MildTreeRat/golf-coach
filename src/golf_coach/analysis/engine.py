"""Swing analysis entry point. [M4-PoC, bundles M7 Phase 4]

Pure function: merged data streams + intent in, `SwingResult` out. Orchestrates the analysis
spine — segment phases → evaluate mechanics checkpoints → combine via the intent's scoring
policy — with no I/O, no hardware, no network. The PoC covers the pose-only Fundamentals path
(tempo checkpoint); the `detections` / `shot` inputs and the `outcome` axis are the named
seams where full M4 adds club detection and launch-monitor scoring (ADR-009).

`analyze_swing_bundle` is the two-view entry point on top of it: same spine, run over the
face-on clip, with the down-the-line clip contributing alignment anchors only.
"""

from __future__ import annotations

from golf_coach.analysis.alignment import (
    MIN_PLAUSIBLE_TEMPO,
    align_swings,
    anchors_from_keypoints,
    anchors_from_phases,
)
from golf_coach.analysis.checkpoints import (
    FINISH_BALANCE_CHECKPOINT,
    HEAD_SWAY_CHECKPOINT,
    TEMPO_CHECKPOINT,
    evaluate_finish_balance,
    evaluate_head_sway,
    evaluate_tempo,
)
from golf_coach.analysis.measure import (
    POSE_MEASUREMENT_DETAIL,
    POSE_MEASUREMENT_UNIT,
    POSE_MEASUREMENTS,
)
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.scoring import policy_for
from golf_coach.analysis.shot_measure import SHOT_MEASUREMENTS
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.alignment import AlignmentQuality, SwingAnchors
from golf_coach.contracts.detections import FrameDetections
from golf_coach.contracts.intent import PracticeGoal
from golf_coach.contracts.keypoints import FrameKeypoints, KeypointsFile
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import (
    CheckpointScore,
    Measurement,
    PhaseSegment,
    SwingBundleResult,
    SwingResult,
)


def _measurements(
    smoothed: list[FrameKeypoints],
    phases: list[PhaseSegment],
    shot: ShotData | None,
) -> list[Measurement]:
    """Every quantity we can measure off this swing, judged by nothing.

    Deliberately independent of the checkpoint loop above. A metric appears here whether or not a
    band exists for it, which is the whole point: bands are cut from populations of measurements,
    so a metric that could only be measured once it had a band could never acquire one. Recording
    them now is also what makes a swing captured today worth re-reading after the bands land.

    Order is pose first then shot, and within each the registry's order — stable across runs so a
    diff of two `analysis.json` files is readable.
    """
    out: list[Measurement] = []

    for name, measure in POSE_MEASUREMENTS.items():
        value = measure(smoothed, phases)
        if value is None:
            continue
        out.append(
            Measurement(
                name=name,
                value=round(value, 4),
                unit=POSE_MEASUREMENT_UNIT[name],
                source="pose:face_on",
                detail=POSE_MEASUREMENT_DETAIL[name],
            )
        )

    if shot is not None:
        device = shot.provenance.device if shot.provenance else shot.source.value
        for name, (measure_shot, unit, detail) in SHOT_MEASUREMENTS.items():
            value = measure_shot(shot)
            if value is None:
                continue
            out.append(
                Measurement(
                    name=name,
                    value=round(value, 4),
                    unit=unit,
                    source=f"launch_monitor:{device}",
                    detail=detail,
                )
            )

    return out


def analyze_swing(
    swing_id: str,
    session_id: str,
    keypoints: list[FrameKeypoints],
    detections: list[FrameDetections] | None = None,
    shot: ShotData | None = None,
    intent: PracticeGoal | None = None,
) -> SwingResult:
    """Analyze one swing from its data streams, judged against a practice intent.

    `intent` defaults to Fundamentals (grade mechanics only). Checkpoints that can't be
    scored (e.g. no benchmark band) are dropped, so `overall_score` reflects only what was
    judged.
    """
    intent = intent or PracticeGoal()

    # Denoise once, up front, so phase instants and every checkpoint read a stable signal
    # (raw MediaPipe landmarks jitter frame-to-frame). SwingResult still keeps the raw
    # keypoints below as the source data this result was computed from.
    smoothed = smooth_keypoints(keypoints)
    phases = segment_phases(smoothed)

    # Each evaluator returns `None` rather than guess when it cannot measure — no band, unusable
    # landmarks, or a boundary that was estimated rather than detected (ADR-010 §2, ADR-013). That
    # is right, but a dropped score still has to be *reported*: `overall_score` is a mean over
    # whatever survived, so a two-checkpoint swing and a three-checkpoint swing otherwise print the
    # same number with nothing to distinguish them. Carrying the name alongside each call is what
    # lets `unscored` say which one went missing.
    mechanics: list[CheckpointScore] = []
    unscored: list[str] = []
    for name, checkpoint in (
        (TEMPO_CHECKPOINT, evaluate_tempo(phases, club=intent.club)),
        (HEAD_SWAY_CHECKPOINT, evaluate_head_sway(smoothed, phases, club=intent.club)),
        (FINISH_BALANCE_CHECKPOINT, evaluate_finish_balance(smoothed, phases, club=intent.club)),
    ):
        if checkpoint is not None:
            mechanics.append(checkpoint)
        else:
            unscored.append(name)

    # Pose-only PoC: no outcome checkpoints yet (needs M2 detection / M3 shot data).
    outcome: list[CheckpointScore] = []

    scores = policy_for(intent.mode).combine(mechanics, outcome)

    return SwingResult(
        swing_id=swing_id,
        session_id=session_id,
        phases=phases,
        checkpoint_scores=mechanics + outcome,
        measurements=_measurements(smoothed, phases, shot),
        unscored=unscored,
        intent=intent,
        mechanics_score=scores.mechanics,
        outcome_score=scores.outcome,
        overall_score=scores.overall,
        keypoints=keypoints,
        detections=detections or [],
        shot=shot,
    )


def analyze_swing_bundle(
    swing_id: str,
    session_id: str,
    face_on: KeypointsFile,
    down_the_line: KeypointsFile | None = None,
    shot: ShotData | None = None,
    intent: PracticeGoal | None = None,
    face_on_window: tuple[int, int] | None = None,
    down_the_line_window: tuple[int, int] | None = None,
) -> SwingBundleResult:
    """Analyze one assembled swing bundle: two camera views plus the launch-monitor shot.

    **The face-on view is scored by `analyze_swing` above, called unmodified.** That is the whole
    design: the three checkpoints were validated against 461 face-on tour clips and there is no
    reason for a second camera to put that at risk. The down-the-line clip is segmented only to
    produce alignment anchors — no checkpoint is measured from it, because the reference corpus
    behind every benchmark band is face-on and a down-the-line metric would have nothing to be
    judged against (ADR-012, ADR-015).

    `shot` is passed straight through to `SwingResult.shot`. It is **attached and reported, never
    scored**: outcome checkpoints need per-club benchmark bands `ranges.json` does not have, and
    grading the ball flight is M4 proper (ADR-009).

    The `*_window` arguments restrict each view to `[start, end)`, which is how a clip containing
    practice swings is pointed at the real one (`phases.select_swing` finds them). This is not
    cosmetic — the window decides which frames get *scored*, not merely which get rendered.

    **Everything comes back in whole-clip coordinates.** A window is a search restriction, not a
    coordinate system: the face-on phases are shifted back by the window offset and the result
    carries the full frame list, so `swing.phases[i].start_frame` indexes `swing.keypoints` and
    the alignment anchors address the same frames the video does. A caller never tracks an offset.

    Degradation is reported, not raised (ADR-013): a missing or unsegmentable down-the-line view
    leaves `alignment` None with a note saying so, and the face-on result stands on its own.
    """
    start, frames = _windowed(face_on.frames, face_on_window)
    swing = analyze_swing(
        swing_id=swing_id,
        session_id=session_id,
        keypoints=frames,
        shot=shot,
        intent=intent,
    )

    # Back into whole-clip coordinates, and re-attach the frames the window sliced away. Keyed
    # on whether a window was actually applied rather than on `start`, because a window opening
    # at frame 0 still truncates the tail and would otherwise leave `keypoints` shorter than the
    # phase indices addressing it.
    if frames is not face_on.frames:
        swing = swing.model_copy(
            update={
                "phases": [_shifted(segment, start) for segment in swing.phases],
                "keypoints": face_on.frames,
            }
        )

    notes: list[str] = []

    # The anchors come from the phases `analyze_swing` just computed rather than from a second
    # segmentation pass. Two reasons, and the second is the important one: it is free, and it
    # makes it *impossible* for the frame a checkpoint was measured on and the frame the warp
    # pins to tau=1 to disagree.
    face_anchors = anchors_from_phases(
        swing.phases,
        clip=face_on.clip,
        camera_id=_camera_id(face_on.frames),
    )
    if face_anchors is None:
        notes.append(
            "face-on: the swing could not be segmented into usable anchors, so the two views "
            "cannot be aligned"
        )

    notes.extend(_tempo_notes(face_anchors))

    dtl_anchors: SwingAnchors | None = None
    if down_the_line is None:
        notes.append("no down-the-line view in this bundle — face-on analysis only")
    else:
        dtl_anchors = anchors_from_keypoints(
            down_the_line.frames, clip=down_the_line.clip, window=down_the_line_window
        )
        if dtl_anchors is None:
            notes.append(
                "down-the-line: the clip could not be segmented into a swing, so the two views "
                "cannot be aligned"
            )

    alignment = None
    if face_anchors is not None and dtl_anchors is not None:
        alignment = align_swings(face_anchors, dtl_anchors)
        if alignment.quality is not AlignmentQuality.FULL:
            notes.append(f"alignment degraded: {alignment.quality.summary}")
        notes.extend(f"alignment: {note}" for note in alignment.notes)

    if shot is not None and shot.provenance is not None and shot.provenance.needs_review:
        notes.append(
            f"shot data needs review (parse confidence "
            f"{shot.provenance.parse_confidence:.2f}) — the numbers below were read off a "
            "photograph and at least one check on them failed (ADR-014)"
        )

    return SwingBundleResult(
        swing_id=swing_id,
        session_id=session_id,
        swing=swing,
        alignment=alignment,
        face_on_window=face_on_window,
        down_the_line_window=down_the_line_window,
        notes=notes,
    )


def _windowed(
    keypoints: list[FrameKeypoints], window: tuple[int, int] | None
) -> tuple[int, list[FrameKeypoints]]:
    """`(offset, frames)` for a window, clamped into range. An empty window is ignored."""
    if window is None:
        return 0, keypoints
    start = max(0, window[0])
    end = min(len(keypoints), window[1])
    if end - start <= 0:
        return 0, keypoints
    return start, keypoints[start:end]


def _shifted(segment: PhaseSegment, offset: int) -> PhaseSegment:
    """One phase boundary moved from window coordinates into whole-clip coordinates.

    Only the frame indices move. `start_ms` / `end_ms` are read off the frames themselves, which
    carry the original clip's timestamps through a slice untouched, so they are already right.
    """
    return segment.model_copy(
        update={
            "start_frame": segment.start_frame + offset,
            "end_frame": segment.end_frame + offset,
        }
    )


def _camera_id(keypoints: list[FrameKeypoints]) -> str | None:
    return next((frame.camera_id for frame in keypoints if frame.camera_id is not None), None)


def _tempo_notes(anchors: SwingAnchors | None) -> list[str]:
    """Say out loud when the tempo checkpoint's own input is not physically possible.

    A backswing that measures shorter than its own downswing is not a golf swing — the same fact
    `alignment.align_swings` refuses a motion-start anchor over, read here against the checkpoint
    that *divides* by that boundary. It happens on real footage for a specific reason: a golfer
    who pauses at the top hands `phases._motion_start` a quiet stretch immediately, so the
    boundary lands a frame or two below the top and the backswing measures near zero.

    `phases.py` reports `detected=True` and is not wrong to — from inside one clip nothing about
    it looks wrong — so `evaluate_tempo` scores it and `feedback` will lead with *"work on tempo
    first, the downswing is rushing the backswing"*. On `aaron-1` that reads 0.43:1.

    **The score is deliberately left alone here.** Dropping the checkpoint would mean this
    function disagreeing with `analyze_swing` about the same frames, and fixing the boundary
    belongs in `phases._motion_start` where the bug is — measured against the GolfDB corpus the
    tempo band was derived from, not patched downstream. What this does is make the contradiction
    impossible to render without seeing it.
    """
    if anchors is None:
        return []
    ratio = anchors.tempo_ratio
    if ratio is None:
        return [
            "face-on: no measurable backswing, so any tempo reading on this swing is meaningless"
        ]
    if ratio < MIN_PLAUSIBLE_TEMPO:
        return [
            f"face-on: the backswing measures {ratio:.2f} downswings, which no golf swing does — "
            "the motion-start boundary has collapsed onto the top (a pause at the top reads as "
            "the quiet stretch the takeaway is measured back to). Any tempo score on this swing "
            "is measuring that artifact, not the golfer; ignore it"
        ]
    return []
