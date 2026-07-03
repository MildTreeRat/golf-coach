"""Tempo checkpoint: ideal ~3:1 passes; too-quick / too-slow score lower and fail."""

from __future__ import annotations

from conftest import make_swing

from golf_coach.analysis.checkpoints import evaluate_tempo
from golf_coach.analysis.phases import segment_phases


def _tempo(backswing_frames: int, downswing_frames: int):
    swing = make_swing(backswing_frames, downswing_frames)
    return evaluate_tempo(segment_phases(swing))


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
