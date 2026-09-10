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
    with_measured_impact,
)
from golf_coach.analysis.benchmarks.joint import placement_for as joint_placement
from golf_coach.analysis.benchmarks.trajectory import (
    placement_from_anchors,
    trajectory_placement_for,
)
from golf_coach.analysis.checkpoints import CHECKPOINT_EVALUATORS
from golf_coach.analysis.flight_measure import (
    FLIGHT_MEASUREMENTS,
    FLIGHT_SOURCE,
    FlownShot,
    flight_unscored,
    fly_shot,
)
from golf_coach.analysis.measure import POSE_MEASUREMENTS
from golf_coach.analysis.phases import TRAIL_WRIST, segment_phases
from golf_coach.analysis.pivot import PIVOT_CHECKS, pivot_observations
from golf_coach.analysis.scoring import policy_for
from golf_coach.analysis.shot_measure import SHOT_MEASUREMENTS
from golf_coach.analysis.smoothing import smooth_keypoints

# Aliased because this package holds **two** functions called `anchors_from_phases` and they answer
# different questions: `alignment`'s builds a `SwingAnchors` for two clips to be warped onto each
# other, while this one returns the three fractional frame positions the resamplers read. Imported
# under a name that says which, rather than shadowing the one imported above.
from golf_coach.analysis.trajectory import anchors_from_phases as event_time_anchors
from golf_coach.contracts.alignment import ClipAlignment, SwingAnchors
from golf_coach.contracts.career import POSE_DTL_SOURCE
from golf_coach.contracts.checkpoints import (
    CHECKPOINT_REGISTRY,
    CONTRADICTED_BY_A_LATE_TOP,
    checkpoint_names,
)
from golf_coach.contracts.detections import FrameDetections
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.intent import PracticeGoal
from golf_coach.contracts.keypoints import FrameKeypoints, KeypointsFile
from golf_coach.contracts.pivots import PIVOT_MEASUREMENT_REGISTRY, FrameOfReference
from golf_coach.contracts.placements import DOWN_THE_LINE, FACE_ON
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
from golf_coach.contracts.unscored import UnscoredCheckpoint, UnscoredReason


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
    smoothed: list[FrameKeypoints],
    anchors: tuple[float, float, float],
    handedness: Handedness | None,
) -> list[Measurement]:
    """The down-the-line view's own trajectory placement, against its own basis.

    Separate names rather than a `view` field on the existing ones, because `measurements` is keyed
    by name everywhere downstream — `analysis/baseline.py::pooled_samples` groups by it, so two
    entries called `tour_trajectory_t2` would silently pool a face-on and a down-the-line number
    into one personal baseline.

    Takes the smoothed frames and the three instants rather than the raw clip and a `SwingAnchors`,
    because M17 P5 gave the rear clip a second reader: denoising and building the anchor tuple at
    the one call site is what keeps the trajectory and the pivot paths resampling onto literally the
    same numbers, instead of two copies of that conversion drifting apart.
    """
    placement = placement_from_anchors(
        smoothed,
        anchors,
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


#: view -> the space its pivot coordinates live in, and the `Measurement.source` its rows carry.
#:
#: One table rather than two lookups, because the pair has to move together. The source is what
#: `CorpusSwing.artifact_key` dedupes on: a `_dtl` row borrowing `pose:face_on` would be counted as
#: a reading of a clip it was never taken from, so the second camera's rows carry
#: `POSE_DTL_SOURCE` — the one `pose:` source that keys on nothing, because `CorpusSwing` holds no
#: hash for the rear clip (ADR-029's 2026-09-09b addendum §5).
#:
#: `CALIBRATED_3D` is deliberately absent and is not an omission: `pivot_observations` raises on it.
#: A calibrated source arrives as a second *producer* of `PivotObservation`, not as a third row
#: here.
_PIVOT_VIEWS: dict[str, tuple[FrameOfReference, str]] = {
    FACE_ON: (FrameOfReference.IMAGE_PLANE_FACE_ON, "pose:face_on"),
    DOWN_THE_LINE: (FrameOfReference.IMAGE_PLANE_DTL, POSE_DTL_SOURCE),
}


def _pivot_measurements(
    smoothed: list[FrameKeypoints],
    anchors: tuple[float, float, float],
    view: str,
) -> list[Measurement]:
    """One view's rotation numbers: the swing read as three moving points. [M17 P5]

    One helper for both cameras, because `pivot_observations` has one signature for both — it takes
    the three anchors rather than a `list[PhaseSegment]`, which the down-the-line clip never has
    (nothing segments it). The face-on caller converts its phases with `event_time_anchors`; the
    bundle already holds the rear clip's three instants for the warp.

    **Recorded, not judged**, behind the same firewall as `_placements`: no band, no
    `CheckpointScore`, nothing that reaches `overall_score`. A rotation checkpoint needs a
    calibrated instrument and a population and M17 has neither (ADR-010 §2), so each row carries its
    `PivotMeasurementSpec.interim_reason` into the derived caveat prose instead.

    **Two ways to record nothing, and they are not the same one.** An unreadable swing — collapsed
    anchors, no usable shoulder width, a landmark missing too much of its timeline — yields no
    observations and therefore no rows at all; a single check that refuses drops its own row and
    leaves the other four. Neither is reported in `unscored`, which is a list of
    `UnscoredCheckpoint`s and nothing here is a checkpoint: `Measurement.value` is a required float,
    so a refusal ships as an absence. Five per view is a ceiling, not a count.

    Name, unit and detail come off the registry rather than being written here, for the reason
    `contracts/pivots.py` exists — `caveats.py` derives the prose every MCP client reads from those
    same rows, and a name typed at this call site could be renamed without the warning following it.
    The spec is in hand from walking the registry, so there is no `spec_for` lookup to do.
    """
    space, source = _PIVOT_VIEWS[view]
    observations = pivot_observations(smoothed, anchors, frame_of_reference=space)
    if observations is None:
        return []

    out: list[Measurement] = []
    for spec in PIVOT_MEASUREMENT_REGISTRY:
        if spec.view != view:
            continue
        # Keyed by `spec.check` and never by `spec.name`: a face-on spec and its `_dtl` partner name
        # the same implementation, and the view is already decided by the caller.
        value = PIVOT_CHECKS[spec.check](observations).value
        if value is None:
            continue
        out.append(
            Measurement(
                name=spec.name,
                value=round(value, 4),
                unit=spec.unit,
                source=source,
                detail=spec.detail,
            )
        )
    return out


def _measurements(
    smoothed: list[FrameKeypoints],
    phases: list[PhaseSegment],
    shot: ShotData | None,
    handedness: Handedness | None = None,
    flown: FlownShot | None = None,
) -> list[Measurement]:
    """Every quantity we can measure off this swing, judged by nothing.

    Deliberately independent of the checkpoint loop above. A metric appears here whether or not a
    band exists for it, which is the whole point: bands are cut from populations of measurements,
    so a metric that could only be measured once it had a band could never acquire one. Recording
    them now is also what makes a swing captured today worth re-reading after the bands land.

    Order is pose first, then shot, then the simulated flight, and within each the registry's
    order — stable across runs so a diff of two `analysis.json` files is readable. The flight goes
    last because it is the only family that *consumes* another: `flight_infer` reads the shot's
    tiles through `shot_measure`'s own extractors, so a reader meeting `flight_carry_yds` has
    already met the `carry_distance_yds` it was solved from.

    **`flown` is passed in rather than computed here**, because a refused flight is reported in
    `unscored` and this function does not build that list. `analyze_swing` flies the ball once and
    hands the result to both.
    """
    out: list[Measurement] = []
    pose_values: dict[str, float] = {}

    for name, pose in POSE_MEASUREMENTS.items():
        # The reason a measurement is missing is not recorded here. `measurements` is the
        # unjudged half — a metric absent from it has no band to be unscored *against*, and
        # the reasons that matter are the ones attached to a checkpoint below.
        value = pose.measure(smoothed, phases).value
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

    # The face-on rotation numbers, after the placements because they are the same kind of thing —
    # measured off the whole swing and judged by nothing. `None` is a swing that could not be
    # segmented into three usable instants, which is already the reason `_placements` recorded no
    # trajectory placement above; the pivot rows simply go missing with it.
    face_on_anchors = event_time_anchors(phases)
    if face_on_anchors is not None:
        out.extend(_pivot_measurements(smoothed, face_on_anchors, FACE_ON))

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

    if flown is not None:
        # One flight, six readings. The registry's functions take the flown shot rather than the
        # launch conditions for exactly that reason — `SHOT_MEASUREMENTS`' one-function-per-number
        # shape would re-integrate the whole trajectory six times, and the spin solve behind it
        # dozens more.
        for name, (read_flight, unit, detail) in FLIGHT_MEASUREMENTS.items():
            flight_value = read_flight(flown)
            if flight_value is None:
                continue
            out.append(
                Measurement(
                    name=name,
                    value=round(flight_value, 4),
                    unit=unit,
                    source=FLIGHT_SOURCE,
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
    loft_deg: float | None = None,
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

    Passing `None` costs the swing that one checkpoint, reported in `unscored` with reason
    `NO_HANDEDNESS`, and costs it nothing else. `analysis` stays pure: resolving a `player_id` to
    a `Golfer` is the shell's job (`api.pipeline`), and nothing here imports the golfer registry.

    **`loft_deg` is a third argument of that same kind** [M15 P11]. It is the club's declared loft,
    and its entire involvement is choosing between the two candidate spins the ball-flight solve
    returns (ADR-027 §Decision 3) — loft is not an input to ball flight, so a club bent 2° strong
    changes the inferred spin and not the path. It arrives here rather than on `PracticeGoal`
    because `intent.club` is a *slot* (`7i`), and what a 7 iron is bent to is a fact about this
    golfer's bag: `storage/flight_inputs.py` reads it and `api.pipeline` passes it. `None` is the
    corpus's own state on most shots and costs the flight measurements, reported in `unscored`
    with reason `NO_CLUB_LOFT` — never guessed, because a 7 iron's loft is exactly the
    plausible-looking wrong answer.
    """
    intent = intent or PracticeGoal()

    # Denoise once, up front, so phase instants and every checkpoint read a stable signal
    # (raw MediaPipe landmarks jitter frame-to-frame). SwingResult still keeps the raw
    # keypoints below as the source data this result was computed from.
    smoothed = smooth_keypoints(keypoints)
    phases = segment_phases(smoothed)

    # Each evaluator refuses rather than guesses when it cannot score — no band, unusable
    # landmarks, a boundary that was estimated rather than detected, or no golfer attributed
    # (ADR-010 §2, ADR-013). That is right, but a dropped score still has to be *reported*:
    # `overall_score` is a mean over whatever survived, so a two-checkpoint swing and a
    # three-checkpoint swing otherwise print the same number with nothing to distinguish them.
    # Walking the registry is what lets `unscored` say which one went missing — the name comes
    # off the spec, so it cannot disagree with the `CheckpointScore` the same spec's evaluator
    # produced — and the evaluator supplies *why*, which nothing downstream could recover.
    #
    # Registry order is the reported order (`contracts.checkpoints` says so), and every evaluator is
    # called through the one adapted signature, so the two that read a narrower set of arguments —
    # tempo takes no keypoints, `head_stays_back` is the only one that needs handedness — do not
    # each need a line here.
    mechanics: list[CheckpointScore] = []
    unscored: list[UnscoredCheckpoint] = []
    for spec in CHECKPOINT_REGISTRY:
        # Not `outcome` — that name is the outcome *axis* a dozen lines down (ADR-009), and the
        # two mean opposite things.
        judged = CHECKPOINT_EVALUATORS[spec.name](
            smoothed, phases, handedness, intent.club, None
        )
        if judged.score is not None:
            mechanics.append(judged.score)
        else:
            assert judged.reason is not None, f"{spec.name} produced neither a score nor a reason"
            unscored.append(
                UnscoredCheckpoint(name=spec.name, reason=judged.reason, detail=judged.detail)
            )

    # The simulated flight [M15 P11]. Flown once, read by both halves below: six `flight_*`
    # measurements, and one or two `unscored` entries when it could not be drawn. It is appended
    # *after* the checkpoint loop rather than inside it because nothing here is a checkpoint —
    # ADR-027 §Decision 6 gives the flight its own measurement names and no `CHECKPOINT_REGISTRY`
    # entry — and registry order is what `contracts.checkpoints` promises for the entries above.
    flown = (
        fly_shot(shot, loft_deg=loft_deg, handedness=handedness) if shot is not None else None
    )
    if flown is not None:
        unscored.extend(flight_unscored(flown))

    # Pose-only PoC: no outcome checkpoints yet (needs M2 detection / M3 shot data).
    outcome: list[CheckpointScore] = []

    scores = policy_for(intent.mode).combine(mechanics, outcome)

    return SwingResult(
        swing_id=swing_id,
        session_id=session_id,
        phases=phases,
        checkpoint_scores=mechanics + outcome,
        measurements=_measurements(smoothed, phases, shot, handedness, flown),
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
    face_on_strikes: list[int] | None = None,
    down_the_line_strikes: list[int] | None = None,
    handedness: Handedness | None = None,
    loft_deg: float | None = None,
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

    The `*_strikes` arguments are the ball strikes heard in each view's own clip, as frame indices
    in that clip's own numbering (`api.pipeline.audio_for` produces them). They move **only the
    alignment**: each view's impact anchor is pinned to the strike it heard, and a pair that
    both heard one reports `AlignmentQuality.SYNCHRONIZED`. No checkpoint changes, because
    `analyze_swing` above has already scored the face-on view off its own phases by the time these
    are read — the property that lets M11 improve the anchor without re-opening M4's scoring.

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
        loft_deg=loft_deg,
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

    # Before the strike is read, on purpose: this note is about the *tempo checkpoint*, and the
    # checkpoint was scored off the pose phases. Pinning impact to the sound first would have it
    # quote a ratio no checkpoint on this swing was measured from.
    notes.extend(_tempo_notes(face_anchors))

    face_anchors = _anchored_on_strike(face_anchors, face_on_strikes, "face-on", notes)

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

    dtl_anchors = _anchored_on_strike(dtl_anchors, down_the_line_strikes, "down-the-line", notes)

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
    # The pivot paths ride along [M17 P5]: the second camera is where the turn is least
    # foreshortened, so its five rotation numbers are the ones that matter most — and they are still
    # never combined with the face-on five, for the reason stated just above. The `_dtl` suffix in
    # `PIVOT_MEASUREMENT_REGISTRY` carries that rule the same way it does here.
    if down_the_line is not None and dtl_anchors is not None:
        # Denoised once and the three instants built once, for both readers of this clip. Two
        # constructions of the same anchor tuple would be two things to keep in step, and what is
        # worth keeping is that the trajectory and the pivot paths resample onto the same numbers.
        dtl_frames = smooth_keypoints(down_the_line.frames)
        dtl_events = (
            float(dtl_anchors.motion_start),
            float(dtl_anchors.top),
            float(dtl_anchors.impact),
        )
        swing = swing.model_copy(
            update={
                "measurements": [
                    *swing.measurements,
                    *_dtl_placements(dtl_frames, dtl_events, handedness),
                    *_pivot_measurements(dtl_frames, dtl_events, DOWN_THE_LINE),
                ]
            }
        )

    alignment = None
    if face_anchors is not None and dtl_anchors is not None:
        alignment = align_swings(face_anchors, dtl_anchors)
        if alignment.quality.is_degraded:
            notes.append(f"alignment degraded: {alignment.quality.summary}")
        notes.extend(f"alignment: {note}" for note in alignment.notes)

        # The one thing in this function that reaches back into a score `analyze_swing` already
        # produced, and it is deliberately the *last* thing: everything above it is measured from
        # one clip, and this is the only finding that does not exist inside either clip alone.
        # `align_swings` is passed `face_anchors` as `a`, so `alignment.a` is always the scored
        # view.
        if alignment.a is not None and alignment.a.top_is_late:
            swing = _without_contradicted_scores(swing, alignment.a, notes)

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


def _anchored_on_strike(
    anchors: SwingAnchors | None, strikes: list[int] | None, label: str, notes: list[str]
) -> SwingAnchors | None:
    """`anchors` with tau=2 pinned to the ball strike this view heard, saying so when it moved.

    Never substitutes silently (ADR-010 §2). A correction of five to seven frames is exactly the
    size of the defect M10 handed forward, so a reader comparing this run against an older one has
    to be able to see that the impact frame changed and by how much — and a reader who *only* has
    this run has to be able to see that the number is a measurement rather than an estimate.

    Runs on `None` too, so the two call sites do not each need the guard: a view that could not be
    segmented has no anchor to pin.
    """
    if anchors is None:
        return None
    measured = with_measured_impact(anchors, strikes)
    moved = measured.impact - anchors.impact
    if measured.impact_measured and moved != 0:
        # `impact_measured` is only ever True when fps was known — `with_measured_impact` sizes its
        # window with it — so the conversion to seconds below cannot divide by None.
        assert measured.fps is not None
        notes.append(
            f"{label}: impact moved {moved:+d} frames ({moved / measured.fps:+.3f}s) onto the ball "
            f"strike heard in this clip — pose had it "
            f"{'early' if moved > 0 else 'late'}. tau=2 is now a measured instant, not an estimate"
        )
    return measured


def _without_contradicted_scores(
    swing: SwingResult, clip: ClipAlignment, notes: list[str]
) -> SwingResult:
    """`swing` with every score a late top invalidates withdrawn into `unscored`. [M11 P8]

    **The only place a cross-view finding re-opens a face-on score**, and it exists because M10
    handed forward three `tempo` readings that score, *fail* at 4.92-6.09:1 against a 4.71 ceiling,
    and are not coaching truth — the top they divide by landed late, which shortens the downswing
    and lengthens the backswing at once. From inside the face-on clip nothing about that is
    visible; `phases.py` reports the boundary as detected and is not wrong to. It takes the other
    camera, on a clock both of them heard, to know (docs/M11_ACOUSTIC_SYNC.md §E3).

    **Withdrawn rather than restated, and the distinction is ADR-010 §2.** `align_swings` can say
    what the ratio reads on the corrected top, and on 2026-08-23 bundle 2 that is 2.35:1 — a pass.
    Writing it into `CheckpointScore.observed` would mean a score whose number came from the
    alignment and whose band came from the engine, measured over frames `segment_phases` never
    agreed to; and where the two views disagree without a shared clock to arbitrate them there is
    no restatement to write at all. No score beats a wrong one, and a corrected top belongs in
    `phases.py` where the boundary is found, not patched in at the seam that noticed.

    `mechanics` and `outcome` are split back apart by registry membership rather than by position,
    because `combine` weighs the two axes differently and `checkpoint_scores` is their
    concatenation with nothing marking the join (ADR-009). Today the outcome list is empty and this
    is a no-op; it is written this way so it stays right when it is not.
    """
    withdrawn = [
        score for score in swing.checkpoint_scores if score.name in CONTRADICTED_BY_A_LATE_TOP
    ]
    if not withdrawn:
        return swing

    names = {score.name for score in withdrawn}
    kept = [score for score in swing.checkpoint_scores if score.name not in names]
    registered = set(checkpoint_names())
    scores = policy_for((swing.intent or PracticeGoal()).mode).combine(
        [score for score in kept if score.name in registered],
        [score for score in kept if score.name not in registered],
    )

    detail = (
        f"the other view, synchronized on the ball strike, puts the top {clip.top_late_by} frames "
        "earlier than this clip did"
    )
    for score in withdrawn:
        observed = "" if score.observed is None else f" at {score.observed:.2f}"
        notes.append(
            f"{score.name} withdrawn: it scored {score.score:.2f}{observed} off a top the "
            f"down-the-line view puts {clip.top_late_by} frames earlier on a shared clock"
        )

    # Registry order is the reported order of `unscored` (`contracts.checkpoints` says so), so the
    # withdrawn entries are sorted into place rather than appended — an appended `tempo` would put
    # the first checkpoint last and quietly break the one ordering that module asks for.
    order = {name: index for index, name in enumerate(checkpoint_names())}
    unscored = [
        *swing.unscored,
        *(
            UnscoredCheckpoint(
                name=score.name,
                reason=UnscoredReason.CROSS_VIEW_CONTRADICTED,
                detail=detail,
            )
            for score in withdrawn
        ),
    ]
    return swing.model_copy(
        update={
            "checkpoint_scores": kept,
            "unscored": sorted(unscored, key=lambda entry: order.get(entry.name, len(order))),
            "mechanics_score": scores.mechanics,
            "outcome_score": scores.outcome,
            "overall_score": scores.overall,
        }
    )


def _tempo_notes(anchors: SwingAnchors | None) -> list[str]:
    """Say out loud when the tempo checkpoint's own input is not physically possible.

    A backswing that measures shorter than its own downswing is not a golf swing — the same fact
    `alignment.align_swings` refuses a motion-start anchor over, read here against the checkpoint
    that *divides* by that boundary. It happens on real footage for a specific reason: a golfer
    who pauses at the top hands `phases._motion_start` a quiet stretch immediately, so the
    boundary lands a frame or two below the top and the backswing measures near zero.

    `phases.py` reports `detected=True` and is not wrong to — from inside one clip nothing about
    it looks wrong — so `evaluate_tempo` scores it and `feedback` will lead with *"work on tempo
    first … take it back longer"*, prescribing a backswing target off a downswing that was never
    measured against a real top. On `aaron-1` that reads 0.43:1.

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
