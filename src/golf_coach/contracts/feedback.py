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
    headline: str | None = Field(
        default=None,
        description=(
            "The one thing to work on, or a clean bill of health naming the closest call. A score "
            "and a list of tips do not tell a golfer where to start; this does. None when nothing "
            "could be scored, which a UI should render as 'no verdict' rather than as 'fine'."
        ),
    )
    tips: list[Tip] = Field(
        default_factory=list,
        description=(
            "Ranked most-actionable first: failures by overshoot, then passes by percentile."
        ),
    )
    coaching_text: str | None = Field(default=None, description="LLM (Claude) response.")
    annotated_video_path: str | None = Field(
        default=None, description="Path to the rendered overlay video, if generated."
    )
