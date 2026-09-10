"""End-to-end: `analyze_swing_bundle` turns two views plus a shot into one complete verdict.

The load-bearing claims, in order of how much damage getting them wrong would do:

1. The face-on view is scored by `analyze_swing` **unchanged** — the three checkpoints were
   validated against 461 tour clips and a second camera must not put that at risk.
2. A window is a search restriction, not a coordinate system: everything comes back addressing
   the whole clip.
3. Degradation is reported rather than raised — a bundle missing its down-the-line view still
   scores, and says what it lost.
"""

from __future__ import annotations

from datetime import UTC, datetime

from conftest import make_swing

from golf_coach.analysis.engine import analyze_swing, analyze_swing_bundle
from golf_coach.contracts.alignment import AlignmentQuality
from golf_coach.contracts.career import POSE_DTL_SOURCE
from golf_coach.contracts.checkpoints import checkpoint_names
from golf_coach.contracts.keypoints import ClipMetadata, FrameKeypoints, KeypointsFile
from golf_coach.contracts.pivots import PIVOTS_BY_NAME, pivot_measurement_names
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.contracts.swing import ANALYSIS_VERSION, SwingBundleResult
from golf_coach.contracts.unscored import UnscoredReason

_FPS = 100.0


def _file(keypoints: list[FrameKeypoints], camera_id: str | None = None) -> KeypointsFile:
    frames = (
        [frame.model_copy(update={"camera_id": camera_id}) for frame in keypoints]
        if camera_id
        else keypoints
    )
    return KeypointsFile(
        clip=ClipMetadata(fps=_FPS, width=1080, height=1920, frame_count=len(frames)),
        frames=frames,
    )


def _concat(*clips: list[FrameKeypoints]) -> list[FrameKeypoints]:
    joined: list[FrameKeypoints] = []
    for clip in clips:
        for frame in clip:
            index = len(joined)
            joined.append(
                frame.model_copy(update={"frame_index": index, "timestamp_ms": index * 10.0})
            )
    return joined


def _shot(*, needs_review: bool = False, confidence: float = 0.95) -> ShotData:
    return ShotData(
        shot_id="shot-1",
        session_id="session-1",
        timestamp=datetime(2026, 8, 7, tzinfo=UTC),
        source=ShotSource.SCREEN,
        carry_distance=195.9,
        ball_speed=142.1,
        club_head_speed=159.5,
        provenance=ShotProvenance(
            device="hd_golf",
            parse_confidence=confidence,
            needs_review=needs_review,
            warnings=["smash factor does not match the speeds"] if needs_review else [],
        ),
    )


def test_face_on_is_scored_by_analyze_swing_unchanged(swing: list[FrameKeypoints]) -> None:
    """The bundle must not reinterpret the validated checkpoints — same frames, same verdict."""
    bare = analyze_swing("s", "sess", swing)
    bundle = analyze_swing_bundle("s", "sess", _file(swing))

    assert bundle.swing.overall_score == bare.overall_score
    assert bundle.swing.mechanics_score == bare.mechanics_score
    assert bundle.swing.unscored == bare.unscored
    assert [(c.name, c.observed, c.passed) for c in bundle.swing.checkpoint_scores] == [
        (c.name, c.observed, c.passed) for c in bare.checkpoint_scores
    ]


def test_the_result_stamps_the_engine_that_produced_it(swing: list[FrameKeypoints]) -> None:
    """`analysis_version` defaults to 0 so legacy artifacts read as older-than-current; the engine
    is the one thing allowed to claim otherwise, and it has to actually do it."""
    bundle = analyze_swing_bundle("s", "sess", _file(swing))

    assert bundle.analysis_version == ANALYSIS_VERSION
    unstamped = SwingBundleResult(swing_id="s", session_id="sess", swing=bundle.swing)
    assert unstamped.analysis_version == 0


def test_two_views_align(swing: list[FrameKeypoints]) -> None:
    """Two clips of one swing at different frame counts still land on a shared axis."""
    face_on = _file(swing, camera_id="face_on")
    # A second view of the same swing, differently framed — more lead-in, longer tail.
    dtl = _file(_concat([swing[0]] * 40, swing, [swing[-1]] * 25), camera_id="down_the_line")

    bundle = analyze_swing_bundle("s", "sess", face_on, dtl)

    assert bundle.alignment is not None
    assert bundle.alignment.quality.is_aligned
    assert bundle.alignment.a is not None and bundle.alignment.b is not None
    # The same instant, found independently in each clip, offset by exactly the pad.
    assert bundle.alignment.b.anchors.impact - bundle.alignment.a.anchors.impact == 40


def test_without_a_down_the_line_view_it_still_scores_and_says_what_is_missing(
    swing: list[FrameKeypoints],
) -> None:
    bundle = analyze_swing_bundle("s", "sess", _file(swing))

    assert bundle.alignment is None
    assert bundle.swing.overall_score > 0.0
    assert any("no down-the-line view" in note for note in bundle.notes)


def test_shot_is_attached_to_the_swing_result(swing: list[FrameKeypoints]) -> None:
    """`SwingResult.shot` is the field this phase finally populates."""
    bundle = analyze_swing_bundle("s", "sess", _file(swing), shot=_shot())

    assert bundle.swing.shot is not None
    assert bundle.swing.shot.carry_distance == 195.9


def test_shot_is_attached_but_never_scored(swing: list[FrameKeypoints]) -> None:
    """v1 displays the launch-monitor numbers; grading them is M4 proper (ADR-009)."""
    with_shot = analyze_swing_bundle("s", "sess", _file(swing), shot=_shot())
    without = analyze_swing_bundle("s", "sess", _file(swing))

    assert with_shot.swing.outcome_score is None
    assert with_shot.swing.overall_score == without.swing.overall_score
    assert [c.name for c in with_shot.swing.checkpoint_scores] == [
        c.name for c in without.swing.checkpoint_scores
    ]


def test_a_shot_flagged_for_review_is_surfaced_in_notes(swing: list[FrameKeypoints]) -> None:
    """ADR-014: a silently-wrong OCR number is the failure mode the flag exists against."""
    bundle = analyze_swing_bundle(
        "s", "sess", _file(swing), shot=_shot(needs_review=True, confidence=0.4)
    )

    assert any("needs review" in note for note in bundle.notes)
    assert any("0.40" in note for note in bundle.notes)


def test_a_trusted_shot_adds_no_review_note(swing: list[FrameKeypoints]) -> None:
    bundle = analyze_swing_bundle("s", "sess", _file(swing), shot=_shot())
    assert not any("needs review" in note for note in bundle.notes)


def test_a_window_shifts_nothing_into_window_coordinates() -> None:
    """The window decides which frames are *scored*; it must not renumber the result.

    Scoring a windowed clip and scoring that window as a standalone clip must agree on the
    verdict while disagreeing on the frame numbers by exactly the offset — that is what makes
    the phases, the anchors and the video all address the same frames.
    """
    real = make_swing(backswing_frames=70, downswing_frames=25)
    clip = _concat([real[0]] * 150, real)
    offset = 150
    window = (offset, len(clip))

    windowed = analyze_swing_bundle("s", "sess", _file(clip), face_on_window=window)
    standalone = analyze_swing_bundle("s", "sess", _file(real))

    assert windowed.face_on_window == window
    # The frames the result addresses are the whole clip's, not the window's.
    assert len(windowed.swing.keypoints) == len(clip)
    assert windowed.swing.phases[-1].end_frame == len(clip) - 1

    for shifted, plain in zip(windowed.swing.phases, standalone.swing.phases, strict=True):
        assert shifted.phase is plain.phase
        assert shifted.start_frame == plain.start_frame + offset
        assert shifted.end_frame == plain.end_frame + offset

    # Timestamps ride on the frames themselves, so a slice never disturbs them.
    assert windowed.swing.phases[0].start_ms == clip[window[0]].timestamp_ms


def test_a_window_opening_at_frame_zero_still_restores_the_full_clip() -> None:
    """A zero offset is not the same as no window — it still truncates the tail.

    Keying the restore on the offset rather than on whether a window was applied left
    `keypoints` shorter than the phase indices addressing it, which is a coordinate mismatch
    that only shows up on a window starting at frame 0.
    """
    real = make_swing(backswing_frames=70, downswing_frames=25)
    clip = _concat(real, [real[-1]] * 120)
    window = (0, len(real))

    bundle = analyze_swing_bundle("s", "sess", _file(clip), face_on_window=window)

    assert len(bundle.swing.keypoints) == len(clip)
    assert bundle.swing.phases[-1].end_frame < len(bundle.swing.keypoints)


def test_an_impossible_backswing_is_called_out_without_changing_the_score() -> None:
    """A backswing shorter than its own downswing means the boundary is wrong, not the golfer.

    The root cause lives in `phases._motion_start` and is deliberately not patched here; what
    this phase guarantees is that the contradiction cannot be rendered without being seen.
    """
    # A window opening at the top leaves no backswing to measure.
    clip = make_swing(backswing_frames=70, downswing_frames=25)
    bundle = analyze_swing_bundle("s", "sess", _file(clip))
    assert bundle.swing.checkpoint_scores  # the healthy case scores normally
    assert not any("no golf swing does" in note for note in bundle.notes)

    from golf_coach.analysis.alignment import MIN_PLAUSIBLE_TEMPO

    # Reproduce the collapse directly: start the clip a few frames before the top.
    top_ish = len(clip) - 25 - 8
    truncated = _concat(clip[top_ish:])
    collapsed = analyze_swing_bundle("s", "sess", _file(truncated))

    assert any("no golf swing does" in note for note in collapsed.notes), collapsed.notes
    assert any("collapsed onto the top" in note for note in collapsed.notes)
    # The score is whatever analyze_swing said — this reports, it does not re-judge.
    bare = analyze_swing("s", "sess", truncated)
    assert collapsed.swing.overall_score == bare.overall_score
    assert MIN_PLAUSIBLE_TEMPO == 1.0


def test_an_unsegmentable_down_the_line_view_degrades_to_face_on_only(
    swing: list[FrameKeypoints],
) -> None:
    """A clip with no swing in it is reported, not raised (ADR-013)."""
    still = _file([swing[0]] * 40, camera_id="down_the_line")
    bundle = analyze_swing_bundle("s", "sess", _file(swing), still)

    assert bundle.alignment is None
    assert bundle.swing.overall_score > 0.0
    assert any("down-the-line" in note for note in bundle.notes)


def test_serializes_without_the_heavy_streams(swing: list[FrameKeypoints]) -> None:
    """The artifact a results page reads must not inline several hundred frames of landmarks."""
    bundle = analyze_swing_bundle("s", "sess", _file(swing), shot=_shot(needs_review=True))

    payload = bundle.model_dump_json(exclude={"swing": {"keypoints", "detections"}})

    assert '"keypoints"' not in payload
    assert '"parse_confidence"' in payload
    assert '"needs_review":true' in payload
    # Still a valid result: the scores, phases and notes all survive the exclusion.
    assert '"overall_score"' in payload
    assert '"phases"' in payload


def test_quality_below_full_is_named_in_notes(swing: list[FrameKeypoints]) -> None:
    """Rendering two panels implies frame correspondence everywhere; only FULL earns it."""
    face_on = _file(swing, camera_id="face_on")
    dtl = _file(_concat([swing[0]] * 40, swing), camera_id="down_the_line")
    bundle = analyze_swing_bundle("s", "sess", face_on, dtl)

    assert bundle.alignment is not None
    if bundle.alignment.quality is not AlignmentQuality.FULL:
        assert any("alignment degraded" in note for note in bundle.notes)


# --- the ball strike reaches the alignment and stops there ------------------------------ [M11 P6]


def _two_views(swing: list[FrameKeypoints]) -> tuple[KeypointsFile, KeypointsFile]:
    """One swing filmed twice, the second phone rolling 40 frames earlier."""
    return (
        _file(swing, camera_id="face_on"),
        _file(_concat([swing[0]] * 40, swing), camera_id="down_the_line"),
    )


def test_a_heard_strike_moves_the_impact_anchor_and_says_so(
    swing: list[FrameKeypoints],
) -> None:
    """§E4's shape end to end: face-on right, down-the-line seven frames early, both heard.

    The correction is never silent (ADR-010 §2). Five to seven frames is exactly the size of the
    defect M10 handed forward, so a reader comparing two runs of the same bundle has to be able to
    see that the impact frame moved, by how much, and that it is now a measurement.
    """
    face_on, dtl = _two_views(swing)
    inferred = analyze_swing_bundle("s", "sess", face_on, dtl)
    assert inferred.alignment is not None
    assert inferred.alignment.a is not None and inferred.alignment.b is not None
    face_impact = inferred.alignment.a.anchors.impact
    dtl_impact = inferred.alignment.b.anchors.impact
    assert not inferred.alignment.a.anchors.impact_measured

    bundle = analyze_swing_bundle(
        "s", "sess", face_on, dtl,
        face_on_strikes=[face_impact],
        down_the_line_strikes=[dtl_impact + 7],
    )

    assert bundle.alignment is not None and bundle.alignment.b is not None
    assert bundle.alignment.b.anchors.impact == dtl_impact + 7
    assert bundle.alignment.quality is AlignmentQuality.SYNCHRONIZED
    assert any("down-the-line: impact moved +7 frames" in note for note in bundle.notes)
    # A tier above FULL is not a degradation, whatever the anchor count underneath it came to.
    assert not any("alignment degraded" in note for note in bundle.notes)


def test_a_strike_that_confirms_the_pose_impact_moves_nothing_and_says_nothing(
    swing: list[FrameKeypoints],
) -> None:
    """The common case — §E4 found seven of eleven bundles right in both views.

    Worth pinning because the note above must fire on a *correction* and not on the measurement:
    a run that logged "impact moved +0 frames" on every healthy bundle would bury the four that
    matter.
    """
    face_on, dtl = _two_views(swing)
    inferred = analyze_swing_bundle("s", "sess", face_on, dtl)
    assert inferred.alignment is not None
    assert inferred.alignment.a is not None and inferred.alignment.b is not None

    bundle = analyze_swing_bundle(
        "s", "sess", face_on, dtl,
        face_on_strikes=[inferred.alignment.a.anchors.impact],
        down_the_line_strikes=[inferred.alignment.b.anchors.impact],
    )

    assert bundle.alignment is not None
    assert bundle.alignment.quality is AlignmentQuality.SYNCHRONIZED
    assert not any("impact moved" in note for note in bundle.notes)


def test_a_heard_strike_never_reaches_the_score(swing: list[FrameKeypoints]) -> None:
    """The property that lets M11 improve an anchor without re-opening M4's validated scoring.

    `analyze_swing` has already scored the face-on view off its own phases by the time a strike is
    read, so the checkpoints, the phases and the totals are bit-identical with and without audio.
    The down-the-line *trajectory placement* is deliberately not in that list: it is resampled onto
    these anchors, so a better impact makes it a better placement.

    **P8 opened one exception, and this test still holds because it stays outside it.** A strike in
    *both* clips can contradict the face-on top, and a contradicted top withdraws the scores timed
    from it - see the section below. One strike is not a shared clock, so nothing here is decidable
    and nothing here may move.
    """
    face_on, dtl = _two_views(swing)
    inferred = analyze_swing_bundle("s", "sess", face_on, dtl)
    assert inferred.alignment is not None and inferred.alignment.b is not None

    bundle = analyze_swing_bundle(
        "s", "sess", face_on, dtl,
        down_the_line_strikes=[inferred.alignment.b.anchors.impact + 7],
    )

    assert bundle.swing.checkpoint_scores == inferred.swing.checkpoint_scores
    assert bundle.swing.phases == inferred.swing.phases
    assert bundle.swing.unscored == inferred.swing.unscored
    assert bundle.swing.overall_score == inferred.swing.overall_score


def test_one_view_hearing_the_strike_leaves_the_pose_tier_alone(
    swing: list[FrameKeypoints],
) -> None:
    """A shared clock needs the crack in both clips; one is the ordinary state of a bundle."""
    face_on, dtl = _two_views(swing)
    inferred = analyze_swing_bundle("s", "sess", face_on, dtl)
    assert inferred.alignment is not None and inferred.alignment.a is not None

    bundle = analyze_swing_bundle(
        "s", "sess", face_on, dtl, face_on_strikes=[inferred.alignment.a.anchors.impact]
    )

    assert bundle.alignment is not None
    assert bundle.alignment.quality is inferred.alignment.quality
    assert any("a shared clock needs the strike in both" in note for note in bundle.notes)


# --- and the one place it does reach the score ------------------------------------------ [M11 P8]
#
# P7's arbitration knew which top was late and could only say so in prose. These pin the
# consequence: a checkpoint timed from a contradicted top is *withdrawn*, because M10 handed
# forward three `tempo` readings that score, fail, and are not coaching truth
# (docs/M11_ACOUSTIC_SYNC.md §E3). Withdrawn rather than restated — ADR-010 §2, and a corrected
# top belongs in `phases.py` where the boundary is found.


def _disagreeing_views(dtl_downswing: int) -> tuple[KeypointsFile, KeypointsFile]:
    """Two views whose downswings disagree, face-on the shorter — M10's defect, in miniature.

    Face-on descends over 10 frames and the second view over `dtl_downswing`, which at the 100 fps
    these fixtures run at puts the reference duration under the caller's control. That is the whole
    reason the parameter is here: the reference duration is what decides whether the warp
    *corrects* the top, and the finding has to survive it saying no.
    """
    return (
        _file(make_swing(downswing_frames=10), camera_id="face_on"),
        _file(make_swing(downswing_frames=dtl_downswing), camera_id="down_the_line"),
    )


def _heard_in_both(face_on: KeypointsFile, dtl: KeypointsFile) -> SwingBundleResult:
    """The same bundle analyzed twice: once to learn the pose impacts, once anchored on them.

    Confirming strikes rather than correcting ones, so the only thing under test is the top. An
    impact that also moved would make a failure here ambiguous between two different findings.
    """
    inferred = analyze_swing_bundle("s", "sess", face_on, dtl)
    assert inferred.alignment is not None
    assert inferred.alignment.a is not None and inferred.alignment.b is not None
    return analyze_swing_bundle(
        "s", "sess", face_on, dtl,
        face_on_strikes=[inferred.alignment.a.anchors.impact],
        down_the_line_strikes=[inferred.alignment.b.anchors.impact],
    )


def test_a_contradicted_top_withdraws_the_score_timed_from_it() -> None:
    """The finding M10 could not act on, acted on.

    `tempo` divides by the top twice over — it is both halves of the ratio — so a top the other
    view puts eight frames earlier makes the number a comparison between one instant that is right
    and one that is not. It leaves `checkpoint_scores` for `unscored`, and `overall_score` becomes
    a mean over what survived rather than over a reading nothing stands behind.
    """
    face_on, dtl = _disagreeing_views(20)
    inferred = analyze_swing_bundle("s", "sess", face_on, dtl)
    assert "tempo" in {score.name for score in inferred.swing.checkpoint_scores}

    bundle = _heard_in_both(face_on, dtl)

    assert "tempo" not in {score.name for score in bundle.swing.checkpoint_scores}
    withdrawn = next(entry for entry in bundle.swing.unscored if entry.name == "tempo")
    assert withdrawn.reason is UnscoredReason.CROSS_VIEW_CONTRADICTED
    assert "8 frames earlier" in withdrawn.detail
    assert bundle.swing.overall_score != inferred.swing.overall_score


def test_the_withdrawal_survives_a_correction_the_warp_declined() -> None:
    """The §Verification case: 2026-08-23/4 ships 6.08:1 and the warp cannot fix its top.

    A 0.50s reference is no downswing, so `_shared_tops` refuses to impose it and `warp_top` stays
    None — but the *diagnosis* holds, and it is the diagnosis this reads. Keying the withdrawal off
    the correction would leave every bundle in that family shipping the failing score it was
    written to retire.
    """
    face_on, dtl = _disagreeing_views(50)
    bundle = _heard_in_both(face_on, dtl)

    assert bundle.alignment is not None and bundle.alignment.a is not None
    assert bundle.alignment.a.warp_top is None
    assert bundle.alignment.a.top_is_late
    assert "tempo" not in {score.name for score in bundle.swing.checkpoint_scores}


def test_a_withdrawal_is_never_silent() -> None:
    """A score that vanishes between two runs of one bundle has to say why in both places.

    `unscored` is the machine-readable half and `notes` is the half a person reads next to a
    results page that used to show a number here. The note carries what the score *was*, because
    a reader comparing this run against a stored one is looking at 4.92:1 in the other window.
    """
    bundle = _heard_in_both(*_disagreeing_views(20))

    note = next(note for note in bundle.notes if "tempo withdrawn" in note)
    assert "on a shared clock" in note
    assert "8 frames earlier" in note


def test_the_withdrawn_checkpoint_keeps_its_place_in_the_reported_order() -> None:
    """Registry order is the reported order of `unscored` (`contracts.checkpoints` says so).

    `tempo` is the *first* checkpoint, and it is withdrawn last — after `analyze_swing` has already
    built `unscored` around whatever it could not score. Appending would put it behind
    `head_stays_back` and break the one ordering that module asks callers to rely on.
    """
    bundle = _heard_in_both(*_disagreeing_views(20))

    registry = checkpoint_names()
    reported = [entry.name for entry in bundle.swing.unscored]
    assert reported == sorted(reported, key=registry.index)
    assert reported[0] == "tempo"


def test_two_views_that_agree_withdraw_nothing(swing: list[FrameKeypoints]) -> None:
    """The ordinary bundle, and the regression that would cost every swing its tempo.

    Both clips heard the strike and both read the same downswing, so there is nothing to arbitrate
    — a `SYNCHRONIZED` tier on its own must never withdraw anything.
    """
    face_on, dtl = _two_views(swing)
    inferred = analyze_swing_bundle("s", "sess", face_on, dtl)
    bundle = _heard_in_both(face_on, dtl)

    assert bundle.alignment is not None
    assert bundle.alignment.quality is AlignmentQuality.SYNCHRONIZED
    assert bundle.swing.checkpoint_scores == inferred.swing.checkpoint_scores
    assert bundle.swing.unscored == inferred.swing.unscored
    assert bundle.swing.overall_score == inferred.swing.overall_score


def test_a_disagreement_without_a_shared_clock_withdraws_nothing() -> None:
    """Two inferred impacts cannot say which top is late, and a guess may not cost a score.

    The same two clips as the withdrawal above, with no audio. The alignment still reports the
    disagreement in its notes and still falls back to one shared duration; what it must not do is
    take a checkpoint away on the strength of a tie-break (ADR-010 §2).
    """
    face_on, dtl = _disagreeing_views(20)
    bundle = analyze_swing_bundle("s", "sess", face_on, dtl)

    assert bundle.alignment is not None and bundle.alignment.a is not None
    assert not bundle.alignment.a.top_is_late
    assert "tempo" in {score.name for score in bundle.swing.checkpoint_scores}
    assert not any("withdrawn" in note for note in bundle.notes)


# --- the second camera's rotation numbers ----------------------------------------------- [M17 P5]
#
# The face-on half is pinned in `tests/analysis/test_engine.py`. What is only visible here is the
# thing M17 is designed around: two cameras, two scales, ten names that must never be one.


def test_the_second_camera_contributes_its_own_pivot_rows(
    swing: list[FrameKeypoints],
) -> None:
    """The down-the-line five, in registry order, beside the face-on five and never merged.

    Down-the-line is where the turn is least foreshortened, so these are the numbers that matter
    most — and they are still a separate scale. The name is what carries that rule: the suffix is
    part of the identifier and not a `view` field, because `analysis/baseline.py::pooled_samples`
    groups a golfer's history by name and one name for two cameras would pool two instruments.
    """
    face_on, dtl = _two_views(swing)
    bundle = analyze_swing_bundle("s", "sess", face_on, dtl)

    emitted = [m.name for m in bundle.swing.measurements if m.name in PIVOTS_BY_NAME]

    assert emitted == list(pivot_measurement_names())
    assert len(set(emitted)) == len(emitted), "a face-on name collided with a `_dtl` one"


def test_the_dtl_pivots_carry_the_source_that_keys_on_nothing(
    swing: list[FrameKeypoints],
) -> None:
    """The one `pose:` source `CorpusSwing.artifact_key` returns None for, asserted at the emitter.

    `CorpusSwing` carries `face_on_sha256` and no hash for the rear clip, so a `_dtl` row sourced
    `pose:face_on` would assert that two different rear clips over one face-on clip are a single
    reading of it — and the pooled value would be whichever was read first. The two halves are
    checked together because the bug is the pair being equal, not either being wrong alone.
    """
    face_on, dtl = _two_views(swing)
    bundle = analyze_swing_bundle("s", "sess", face_on, dtl)

    by_source: dict[str, set[str]] = {}
    for measurement in bundle.swing.measurements:
        if measurement.name in PIVOTS_BY_NAME:
            by_source.setdefault(measurement.source, set()).add(measurement.name)

    assert by_source[POSE_DTL_SOURCE] == {
        name for name in pivot_measurement_names() if name.endswith("_dtl")
    }
    assert by_source["pose:face_on"].isdisjoint(by_source[POSE_DTL_SOURCE])


def test_a_bundle_with_no_rear_clip_records_only_the_face_on_half(
    swing: list[FrameKeypoints],
) -> None:
    """No second camera, no second reading — the direction that would be silent if it broke.

    A `_dtl` row on a one-view bundle is a number about a clip nobody filmed, and every consumer
    downstream presents it as the view where the turn is best seen.
    """
    bundle = analyze_swing_bundle("s", "sess", _file(swing))

    emitted = [m.name for m in bundle.swing.measurements if m.name in PIVOTS_BY_NAME]

    assert emitted, "the face-on half went missing with the rear clip"
    assert not [name for name in emitted if name.endswith("_dtl")]


def test_the_pivot_rows_move_no_score(swing: list[FrameKeypoints]) -> None:
    """Ten unjudged numbers arrived and the verdict is byte-identical, both views included.

    This is the claim `ANALYSIS_VERSION` 15 -> 16 makes about every stored artifact — that
    `reanalyze.py` adds rows and changes no grade — asserted against the one-view result rather
    than against a recorded constant, so it stays true as the checkpoints themselves move.
    """
    face_on, dtl = _two_views(swing)
    bare = analyze_swing("s", "sess", swing)
    bundle = analyze_swing_bundle("s", "sess", face_on, dtl)

    assert bundle.swing.overall_score == bare.overall_score
    assert bundle.swing.mechanics_score == bare.mechanics_score
    assert [(c.name, c.observed, c.passed) for c in bundle.swing.checkpoint_scores] == [
        (c.name, c.observed, c.passed) for c in bare.checkpoint_scores
    ]
    assert bundle.swing.unscored == bare.unscored
