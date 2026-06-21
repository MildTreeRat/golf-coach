"""Feedback contract (produced by the `feedback` module, consumed by the UI/API).

The final user-facing payload: the score, structured rule-based tips, the LLM
coaching text, and a pointer to the annotated replay video.
"""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel, Field


class Severity(StrEnum):
    INFO = "info"
    MINOR = "minor"
    MAJOR = "major"


class Tip(BaseModel):
    """A single rule-based coaching tip tied to a checkpoint."""

    checkpoint: str
    text: str
    severity: Severity = Severity.INFO


class FeedbackPayload(BaseModel):
    """Everything the UI needs to render feedback for one swing."""

    swing_id: str
    overall_score: float = Field(ge=0.0, le=100.0)
    tips: list[Tip] = Field(default_factory=list)
    coaching_text: str | None = Field(default=None, description="LLM (Claude) response.")
    annotated_video_path: str | None = Field(
        default=None, description="Path to the rendered overlay video, if generated."
    )
