"""Rule-based feedback. [M4-PoC]

Maps each `CheckpointScore` in a `SwingResult` to a plain-English `Tip`. Pure function, no
I/O. The PoC covers the tempo checkpoint; the fuller rules catalogue (one rule set per
checkpoint, phrasing tuned by phase) is M5, and LLM coaching is M6. Severity is derived from
the checkpoint result so the UI can rank tips without re-deriving it.
"""

from __future__ import annotations

from golf_coach.contracts.feedback import FeedbackPayload, Severity, Tip
from golf_coach.contracts.swing import CheckpointScore, SwingResult

# A checkpoint that failed but still scored at/above this is a minor miss; below it, major.
_MINOR_SCORE_FLOOR = 0.5


def _severity_for(checkpoint: CheckpointScore) -> Severity:
    if checkpoint.passed:
        return Severity.INFO
    return Severity.MINOR if checkpoint.score >= _MINOR_SCORE_FLOOR else Severity.MAJOR


def _tip_for(checkpoint: CheckpointScore) -> Tip:
    text = checkpoint.message or f"{checkpoint.name}: score {checkpoint.score:.0%}."
    return Tip(checkpoint=checkpoint.name, text=text, severity=_severity_for(checkpoint))


def build_feedback(result: SwingResult) -> FeedbackPayload:
    """Produce rule-based tips from a swing result (LLM coaching added in M6)."""
    return FeedbackPayload(
        swing_id=result.swing_id,
        overall_score=result.overall_score,
        tips=[_tip_for(checkpoint) for checkpoint in result.checkpoint_scores],
    )
