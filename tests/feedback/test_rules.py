"""Rule-based feedback: a swing result becomes a FeedbackPayload with a tempo tip."""

from __future__ import annotations

from golf_coach.contracts.feedback import Severity
from golf_coach.contracts.swing import CheckpointScore, SwingResult
from golf_coach.feedback.rules import build_feedback


def _result_with(checkpoint: CheckpointScore, overall: float) -> SwingResult:
    return SwingResult(
        swing_id="swing-1",
        session_id="session-1",
        checkpoint_scores=[checkpoint],
        overall_score=overall,
    )


def test_passing_tempo_yields_info_tip() -> None:
    cp = CheckpointScore(name="tempo", score=1.0, passed=True, message="Good tempo.")
    payload = build_feedback(_result_with(cp, 100.0))

    assert payload.swing_id == "swing-1"
    assert payload.overall_score == 100.0
    assert len(payload.tips) == 1
    tip = payload.tips[0]
    assert tip.checkpoint == "tempo"
    assert tip.severity is Severity.INFO
    assert tip.text == "Good tempo."


def test_bad_tempo_is_flagged_by_severity() -> None:
    cp = CheckpointScore(name="tempo", score=0.1, passed=False, message="Tempo too quick.")
    payload = build_feedback(_result_with(cp, 10.0))
    assert payload.tips[0].severity is Severity.MAJOR
