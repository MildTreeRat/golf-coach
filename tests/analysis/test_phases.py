"""Phase segmentation: a synthetic swing yields the six phases, in order, contiguous."""

from __future__ import annotations

from conftest import _ADDRESS_FRAMES, make_swing

from golf_coach.analysis.phases import _rising_runs, segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.swing import SwingPhase


def test_six_phases_in_canonical_order(swing: list[FrameKeypoints]) -> None:
    phases = segment_phases(swing)
    assert [p.phase for p in phases] == [
        SwingPhase.ADDRESS,
        SwingPhase.BACKSWING,
        SwingPhase.TRANSITION,
        SwingPhase.DOWNSWING,
        SwingPhase.IMPACT,
        SwingPhase.FOLLOW_THROUGH,
    ]


def test_segments_are_contiguous_and_monotonic(swing: list[FrameKeypoints]) -> None:
    phases = segment_phases(swing)
    # Each segment is well-formed and the chain covers the whole clip without gaps.
    for segment in phases:
        assert segment.start_frame <= segment.end_frame
        assert segment.start_ms <= segment.end_ms
    # Sliding pairwise window (intentionally uneven lengths), so strict=False.
    for earlier, later in zip(phases, phases[1:], strict=False):
        assert earlier.end_frame == later.start_frame
    assert phases[0].start_frame == 0
    assert phases[-1].end_frame == len(swing) - 1


def test_too_short_clip_returns_no_phases() -> None:
    assert segment_phases([]) == []


def test_top_detected_near_true_top_on_smoothed_swing() -> None:
    # make_swing(30, 10): 8 address frames + 30-frame backswing => the top (min wrist-y)
    # sits at ~frame 37. Smoothing (as engine does) must keep the detected top close to it.
    smoothed = smooth_keypoints(make_swing(30, 10))
    phases = segment_phases(smoothed)
    transition = next(p for p in phases if p.phase is SwingPhase.TRANSITION)
    top = (transition.start_frame + transition.end_frame) // 2
    assert 34 <= top <= 41


def test_top_survives_a_finish_higher_than_the_top() -> None:
    """The regression that GolfDB exposed. [M4-REF]

    A full tour finish leaves the hands **higher** than they ever were at the top of the backswing,
    so "top = global minimum of wrist y" locates the *finish* instead. Measured against GolfDB's
    hand-annotated events, the old rule was a median of 26 frames late and wrong on 80% of tour
    clips — and no clip we owned could have shown it, because none of them finish that high.

    Here the finish (`0.05`) is well above the top (`_TOP_Y == 0.15`). The detected top must stay
    on the backswing, not jump to the end of the clip.
    """
    smoothed = smooth_keypoints(make_swing(30, 10, finish_y=0.05))
    phases = segment_phases(smoothed)
    transition = next(p for p in phases if p.phase is SwingPhase.TRANSITION)
    top = (transition.start_frame + transition.end_frame) // 2
    assert 34 <= top <= 41, f"top landed at {top} — likely the finish, not the top of backswing"

    # And the swing must still segment sanely around it, rather than collapsing to a sliver.
    impact = next(p for p in phases if p.phase is SwingPhase.IMPACT)
    assert top < impact.start_frame < len(smoothed) - 1


def test_rising_runs_ignore_untracked_frames() -> None:
    """Held-forward values must not manufacture a descent. [M4-REF]

    `_lead_wrist_xy` carries the last confident position through dim frames to keep the series
    continuous. That is right for smoothing and wrong for finding extrema: the held stretch is not
    evidence of where the hands went. Here the untracked middle sits far below the tracked frames,
    which without the confidence mask would read as by far the largest descent in the clip.
    """
    ys = [0.50, 0.40, 0.30, 0.95, 0.95, 0.95, 0.32, 0.40, 0.50]
    confident = [True, True, True, False, False, False, True, True, True]

    runs = _rising_runs(ys, confident)

    assert runs, "the genuine 0.30 -> 0.50 descent should still be found"
    assert max(rise for rise, _, _ in runs) < 0.30, (
        "a run spanning the untracked plateau was scored — the 0.30 -> 0.95 jump is an artefact "
        "of holding the last good value, not a descent of the hands"
    )
    assert all(not 3 <= end <= 5 for _, _, end in runs)


def test_motion_start_includes_horizontal_takeaway() -> None:
    # A 20-frame near-horizontal takeaway (lead wrist sliding sideways at address height) sits
    # between the 8-frame address dwell and the vertical rise (~frame 28+). A wrist-*height* rule
    # can't see that sideways move and anchors motion-start at the vertical-rise start, collapsing
    # the backswing; the 2D-speed rule must anchor it back near the takeaway start (~frame 8).
    smoothed = smooth_keypoints(make_swing(20, 13, takeaway_frames=20))
    phases = segment_phases(smoothed)
    backswing = next(p for p in phases if p.phase is SwingPhase.BACKSWING)
    assert backswing.start_frame <= _ADDRESS_FRAMES + 4  # not the ~frame-28 vertical-rise start
