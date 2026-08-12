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
from golf_coach.contracts.keypoints import ClipMetadata, FrameKeypoints, KeypointsFile
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.contracts.swing import ANALYSIS_VERSION, SwingBundleResult

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
