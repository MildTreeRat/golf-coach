"""Mechanics checkpoints: tempo, head sway, and finish balance on synthetic swings."""

from __future__ import annotations

from conftest import make_swing

from golf_coach.analysis.checkpoints import (
    evaluate_finish_balance,
    evaluate_head_sway,
    evaluate_tempo,
)
from golf_coach.analysis.phases import segment_phases
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints


def _tempo(backswing_frames: int, downswing_frames: int):
    swing = make_swing(backswing_frames, downswing_frames)
    return evaluate_tempo(segment_phases(swing))


def _analyzed(head_sway: float = 0.0, finish_drift: float = 0.0):
    """Smooth then segment, mirroring the engine, and return (smoothed, phases)."""
    smoothed = smooth_keypoints(
        make_swing(30, 10, head_sway=head_sway, finish_drift=finish_drift)
    )
    return smoothed, segment_phases(smoothed)


def test_ideal_tempo_passes_inside_band() -> None:
    cp = _tempo(30, 10)  # ~3:1
    assert cp is not None
    assert cp.name == "tempo"
    assert cp.passed is True
    assert cp.expected_low <= cp.observed <= cp.expected_high
    assert cp.score == 1.0


def test_too_quick_tempo_fails_and_scores_lower() -> None:
    cp = _tempo(10, 12)  # backswing shorter than downswing → well under 2.7
    assert cp is not None
    assert cp.passed is False
    assert cp.observed < cp.expected_low
    assert cp.score < 1.0


def test_too_slow_tempo_fails_and_scores_lower() -> None:
    cp = _tempo(44, 8)  # long backswing → well over 3.3
    assert cp is not None
    assert cp.passed is False
    assert cp.observed > cp.expected_high
    assert cp.score < 1.0


def test_steady_head_passes_sway() -> None:
    smoothed, phases = _analyzed(head_sway=0.0)
    cp = evaluate_head_sway(smoothed, phases)
    assert cp is not None
    assert cp.name == "head_sway"
    assert cp.passed is True
    assert cp.score == 1.0
    assert cp.observed <= cp.expected_high


def test_large_head_sway_fails() -> None:
    # A full shoulder-width of lateral drift (sway ~1.0) is well past the 0.5 band.
    smoothed, phases = _analyzed(head_sway=0.16)
    cp = evaluate_head_sway(smoothed, phases)
    assert cp is not None
    assert cp.passed is False
    assert cp.observed > cp.expected_high
    assert cp.score < 1.0


def test_held_finish_passes_balance() -> None:
    smoothed, phases = _analyzed(finish_drift=0.0)
    cp = evaluate_finish_balance(smoothed, phases)
    assert cp is not None
    assert cp.name == "finish_balance"
    assert cp.passed is True
    assert cp.score == 1.0


def test_staggered_finish_fails_balance() -> None:
    smoothed, phases = _analyzed(finish_drift=0.3)
    cp = evaluate_finish_balance(smoothed, phases)
    assert cp is not None
    assert cp.passed is False
    assert cp.observed > cp.expected_high


def test_checkpoints_return_none_when_unsegmentable() -> None:
    empty: list[FrameKeypoints] = []
    assert evaluate_head_sway(empty, []) is None
    assert evaluate_finish_balance(empty, []) is None
