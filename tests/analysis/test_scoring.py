"""Scoring policies: Fundamentals grades mechanics only; other modes are seams."""

from __future__ import annotations

import pytest

from golf_coach.analysis.scoring import AxisScores, FundamentalsPolicy, policy_for
from golf_coach.contracts.intent import PracticeMode
from golf_coach.contracts.swing import CheckpointScore


def _cp(score: float) -> CheckpointScore:
    return CheckpointScore(name="tempo", score=score, passed=score >= 0.999)


def test_fundamentals_overall_equals_mechanics_and_outcome_is_none() -> None:
    policy = FundamentalsPolicy()
    result = policy.combine([_cp(1.0), _cp(0.5)], outcome=[])
    assert isinstance(result, AxisScores)
    assert result.mechanics == pytest.approx(75.0)
    assert result.overall == pytest.approx(75.0)
    assert result.outcome is None


def test_empty_mechanics_scores_zero() -> None:
    result = FundamentalsPolicy().combine([], outcome=[])
    assert result.mechanics == 0.0
    assert result.overall == 0.0


def test_policy_for_fundamentals_and_unimplemented_modes() -> None:
    assert isinstance(policy_for(PracticeMode.FUNDAMENTALS), FundamentalsPolicy)
    with pytest.raises(NotImplementedError):
        policy_for(PracticeMode.SHOT_SHAPING)
