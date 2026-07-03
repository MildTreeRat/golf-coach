"""End-to-end: analyze_swing turns a synthetic swing into a scored SwingResult."""

from __future__ import annotations

from golf_coach.analysis import analyze_swing
from golf_coach.contracts.intent import PracticeGoal, PracticeMode
from golf_coach.contracts.keypoints import FrameKeypoints


def test_analyze_swing_produces_tempo_checkpoint_and_score(swing: list[FrameKeypoints]) -> None:
    result = analyze_swing("swing-1", "session-1", swing)

    assert result.swing_id == "swing-1"
    assert len(result.phases) == 6

    names = {cp.name for cp in result.checkpoint_scores}
    assert "tempo" in names

    # Fundamentals: overall == mechanics, outcome axis untouched.
    assert result.intent is not None
    assert result.intent.mode is PracticeMode.FUNDAMENTALS
    assert result.outcome_score is None
    assert result.mechanics_score == result.overall_score
    assert 0.0 <= result.overall_score <= 100.0
    assert result.overall_score > 0.0  # an ideal-tempo swing scores well


def test_analyze_swing_defaults_intent_to_fundamentals(swing: list[FrameKeypoints]) -> None:
    explicit = analyze_swing("s", "sess", swing, intent=PracticeGoal())
    implied = analyze_swing("s", "sess", swing)
    assert explicit.overall_score == implied.overall_score
