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
from golf_coach.analysis.benchmarks.joint import placement_for as joint_placement
from golf_coach.analysis.benchmarks.trajectory import (
    DOWN_THE_LINE,
    placement_from_anchors,
    trajectory_placement_for,
)
from golf_coach.analysis.checkpoints import CHECKPOINT_EVALUATORS
from golf_coach.analysis.measure import POSE_MEASUREMENTS
from golf_coach.analysis.phases import TRAIL_WRIST, segment_phases
from golf_coach.analysis.scoring import policy_for
from golf_coach.analysis.shot_measure import SHOT_MEASUREMENTS
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.alignment import AlignmentQuality, SwingAnchors
from golf_coach.contracts.checkpoints import CHECKPOINT_REGISTRY
from golf_coach.contracts.detections import FrameDetections
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.intent import PracticeGoal
from golf_coach.contracts.keypoints import FrameKeypoints, KeypointsFile
from golf_coach.contracts.placements import spec_for as placement_spec
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import (
    ANALYSIS_VERSION,
    CheckpointScore,
    Measurement,
    PhaseSegment,
    SwingBundleResult,
    SwingResult,
)


def _placements(
    smoothed: list[FrameKeypoints],
    phases: list[PhaseSegment],
    pose_values: dict[str, float],
    handedness: Handedness | None,
) -> list[Measurement]:
    """Where this swing sits against the tour *population*, as recorded quantities.

    Three numbers that no single checkpoint can produce, because each is about the swing as a
    whole: how unusual the six metrics are **as a combination** (ADR-022's joint model), and how
    far the motion sits from the tour shape both **inside** the fitted subspace (T²) and **off** it
    entirely (Q).

    **Recorded, not judged.** They ride on `measurements` rather than `checkpoint_scores`, so they
    cannot touch `overall_score` — the same firewall ADR-010 §2 puts around percentiles, and the
    same "measure now, judge later" ordering M6.5 established. A band for any of them would have to
    be earned separately, against a population of these values that does not exist yet.

    Each returns `None` rather than a guess when its inputs are incomplete: the joint model needs
    all six metrics, the trajectory model needs three usable phase anchors.

    Name and unit come off `contracts.placements` rather than being typed here, for the reason that
    module exists: `caveats.py` names these five in the prose every MCP client and every coaching
    call reads, and a name that lived only at this call site could be renamed without the warning
    about it following.
    """
    out: list[Measurement] = []

    joint = joint_placement(pose_values)
    if joint is not None:
        leading = next(iter(joint.contributions), "?")
        spec = placement_spec("tour_joint_distance")
        out.append(
            Measurement(
                name=spec.name,
                value=round(joint.distance, 4),
                unit=spec.unit,
                source="population:golfdb",
                detail=(
                    f"Mahalanobis distance of the six metrics as a combination, against "
                    f"{joint.population_n} face-on tour swings; more unusual than "
                    f"{joint.percentile:g}% of them. Largest contributor: {leading}"
                ),
            )
        )

    placement = trajectory_placement_for(
        smoothed, phases, left_handed=handedness == Handedness.LEFT
    )
    if placement is not None:
        interval = next(iter(placement.residual_by_interval), "?")
        t2_spec, q_spec = placement_spec("tour_trajectory_t2"), placement_spec("tour_trajectory_q")
        out.append(
            Measurement(
                name=t2_spec.name,
                value=round(placement.t2, 4),
                unit=t2_spec.unit,
                source="population:golfdb",
                detail=(
                    f"distance from the tour swing shape inside the fitted subspace; more "
                    f"unusual than {placement.t2_percentile:g}% of "
                    f"{placement.population_n} tour swings"
                ),
            )
        )
        out.append(
            Measurement(
                name=q_spec.name,
                value=round(placement.q, 4),
                unit=q_spec.unit,
                source="population:golfdb",
                detail=(
                    f"residual off the tour subspace — shape the basis cannot represent; most of "
                    f"it falls in {interval}. NOT calibrated: it over-flags golfers the basis "
                    f"never saw, so read it beside T2 rather than alone"
                ),
            )
        )

    return out


def _dtl_placements(
    frames: list[FrameKeypoints],
    anchors: SwingAnchors,
    handedness: Handedness | None,
) -> list[Measurement]:
    """The down-the-line view's own trajectory placement, against its own basis.

    Separate names rather than a `view` field on the existing ones, because `measurements` is keyed
    by name everywhere downstream — `analysis/baseline.py::pooled_samples` groups by it, so two
    entries called `tour_trajectory_t2` would silently pool a face-on and a down-the-line number
    into one personal baseline.
    """
    placement = placement_from_anchors(
        smooth_keypoints(frames),
        (float(anchors.motion_start), float(anchors.top), float(anchors.impact)),
        left_handed=handedness == Handedness.LEFT,
        view=DOWN_THE_LINE,
    )
    if placement is None:
        return []

    interval = next(iter(placement.residual_by_interval), "?")
    t2_spec = placement_spec("tour_trajectory_t2_dtl")
    q_spec = placement_spec("tour_trajectory_q_dtl")
    return [
        Measurement(
            name=t2_spec.name,
            value=round(placement.t2, 4),
            unit=t2_spec.unit,
            source="population:golfdb",
            detail=(
                f"down-the-line: distance from the tour swing shape inside its fitted subspace; "
                f"more unusual than {placement.t2_percentile:g}% of "
                f"{placement.population_n} tour swings. A separate basis from the face-on one — "
                f"never combine the two"
            ),
        ),
        Measurement(
            name=q_spec.name,
            value=round(placement.q, 4),
            unit=q_spec.unit,
            source="population:golfdb",
            detail=(
                f"down-the-line: residual off that basis; most of it falls in {interval}. NOT "
                f"calibrated, same caveat as the face-on Q"
            ),
        ),
    ]


def _measurements(
    smoothed: list[FrameKeypoints],
    phases: list[PhaseSegment],
    shot: ShotData | None,
    handedness: Handedness | None = None,
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
    pose_values: dict[str, float] = {}

    for name, pose in POSE_MEASUREMENTS.items():
        value = pose.measure(smoothed, phases)
        if value is None:
            continue
        pose_values[name] = value
        out.append(
            Measurement(
                name=name,
                value=round(value, 4),
                unit=pose.unit,
                source="pose:face_on",
                detail=pose.detail,
            )
        )

    out.extend(_placements(smoothed, phases, pose_values, handedness))

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
    handedness: Handedness | None = None,
) -> SwingResult:
    """Analyze one swing from its data streams, judged against a practice intent.

    `intent` defaults to Fundamentals (grade mechanics only). Checkpoints that can't be
    scored (e.g. no benchmark band) are dropped, so `overall_score` reflects only what was
    judged.

    **`handedness` is identity, not intent, which is why it is a separate argument.** Every signed
    quantity this engine measures is camera-relative — a face-on camera sees a left-handed swing
    mirrored — so `head_stays_back` cannot be scored without it. `PracticeGoal` was the tempting
    place to put it and is the wrong one: intent is what the golfer was *trying to do* and is chosen
    per session, while handedness is *who they are* and is recorded once (`contracts.golfer` states
    the distinction and why the field is captured at capture time).

    Passing `None` costs the swing that one checkpoint, reported by name in `unscored`, and costs it
    nothing else. `analysis` stays pure: resolving a `player_id` to a `Golfer` is the shell's job
    (`api.pipeline`), and nothing here imports the golfer registry.
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
    # same number with nothing to distinguish them. Walking the registry is what lets `unscored`
    # say which one went missing — the name comes off the spec, so it cannot disagree with the
    # `CheckpointScore` the same spec's evaluator produced.
    #
    # Registry order is the reported order (`contracts.checkpoints` says so), and every evaluator is
    # called through the one adapted signature, so the two that read a narrower set of arguments —
    # tempo takes no keypoints, `head_stays_back` is the only one that needs handedness — do not
    # each need a line here.
    mechanics: list[CheckpointScore] = []
    unscored: list[str] = []
    for spec in CHECKPOINT_REGISTRY:
        checkpoint = CHECKPOINT_EVALUATORS[spec.name](
            smoothed, phases, handedness, intent.club, None
        )
        if checkpoint is not None:
            mechanics.append(checkpoint)
        else:
            unscored.append(spec.name)

    # Pose-only PoC: no outcome checkpoints yet (needs M2 detection / M3 shot data).
    outcome: list[CheckpointScore] = []

    scores = policy_for(intent.mode).combine(mechanics, outcome)

    return SwingResult(
        swing_id=swing_id,
        session_id=session_id,
        phases=phases,
        checkpoint_scores=mechanics + outcome,
        measurements=_measurements(smoothed, phases, shot, handedness),
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
    handedness: Handedness | None = None,
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
        handedness=handedness,
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
        # The trail wrist, not the lead one — this is the only place the two views are told
        # apart, and it is worth the special case. From behind, the lead wrist is the far arm and
        # is tracked in 39% of frames; the shipped rule then misses the top on 30% of GolfDB's
        # down-the-line clips and impact on 35%. On the trail wrist it misses 7% and 2%, which is
        # better than face-on manages. Measured over 584 labelled clips — M4_POSE_BAKEOFF §Phase F.
        dtl_anchors = anchors_from_keypoints(
            down_the_line.frames,
            clip=down_the_line.clip,
            window=down_the_line_window,
            wrist=TRAIL_WRIST,
        )
        if dtl_anchors is None:
            notes.append(
                "down-the-line: the clip could not be segmented into a swing, so the two views "
                "cannot be aligned"
            )

    # The down-the-line view gets its own trajectory placement, against its own basis, and the two
    # are **never combined into one number**. They are two cameras answering the same question
    # about different planes of the same swing, and blending them would be exactly the mistake
    # ADR-009 avoided by keeping mechanics and outcome as separate axes. Disagreement between them
    # is a finding rather than a defect: a swing that looks ordinary face-on and unusual from
    # behind has departed in the plane the face-on camera cannot see.
    #
    # Built from `dtl_anchors` rather than by segmenting again — those are already the three
    # instants this model resamples onto, so reusing them makes it impossible for the frames the
    # trajectory reads and the frames the warp pins to disagree.
    if down_the_line is not None and dtl_anchors is not None:
        swing = swing.model_copy(
            update={
                "measurements": [
                    *swing.measurements,
                    *_dtl_placements(down_the_line.frames, dtl_anchors, handedness),
                ]
            }
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
        # Set explicitly, because the field defaults to 0 so that artifacts written before it
        # existed read as older-than-current rather than as current. This is the one place that
        # gets to claim otherwise, and it earns it by being the thing that just did the work.
        analysis_version=ANALYSIS_VERSION,
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
