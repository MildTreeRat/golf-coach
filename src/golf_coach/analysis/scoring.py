"""Scoring policies — intent selects how sub-scores combine. [M4-PoC]

Per [ADR-009](../../docs/decisions/009-swing-scoring-model.md): a swing has two independent
axes — **mechanics** (pose) and **outcome** (ball flight) — and the golfer's *practice mode*
picks a **ScoringPolicy** (Strategy) that weights them into the overall score. The engine
passes the two checkpoint lists separately, so a policy never needs to know which axis a
`CheckpointScore` came from.

The PoC implements only `FundamentalsPolicy` (mechanics 100%, outcome ignored). The other
modes are named seams: `policy_for` raises `NotImplementedError` for them until full M4.
"""

from __future__ import annotations

from typing import NamedTuple, Protocol

from golf_coach.contracts.intent import PracticeMode
from golf_coach.contracts.swing import CheckpointScore


class AxisScores(NamedTuple):
    """The two sub-scores and their blended overall (all on a 0-100 scale)."""

    mechanics: float | None
    outcome: float | None
    overall: float


class ScoringPolicy(Protocol):
    """Combines per-axis checkpoint scores into `AxisScores` (Strategy)."""

    def combine(
        self, mechanics: list[CheckpointScore], outcome: list[CheckpointScore]
    ) -> AxisScores: ...


def _mean_percent(scores: list[CheckpointScore]) -> float:
    """Mean of 0..1 checkpoint scores, scaled to 0..100 (0.0 for an empty list)."""
    if not scores:
        return 0.0
    return sum(score.score for score in scores) / len(scores) * 100.0


class FundamentalsPolicy:
    """Grade mechanics only; outcome is informational (ADR-009 §PoC boundary)."""

    def combine(
        self, mechanics: list[CheckpointScore], outcome: list[CheckpointScore]
    ) -> AxisScores:
        mechanics_score = _mean_percent(mechanics)
        return AxisScores(mechanics=mechanics_score, outcome=None, overall=mechanics_score)


def policy_for(mode: PracticeMode) -> ScoringPolicy:
    """Select the scoring policy for a practice mode.

    Only `FUNDAMENTALS` is wired in the PoC; the other modes are documented seams that arrive
    with the outcome axis in full M4.
    """
    if mode is PracticeMode.FUNDAMENTALS:
        return FundamentalsPolicy()
    raise NotImplementedError(f"scoring policy for {mode!r} lands in full M4")
