"""Phase segmentation: a synthetic swing yields the six phases, in order, contiguous."""

from __future__ import annotations

from conftest import make_swing

from golf_coach.analysis.phases import segment_phases
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
