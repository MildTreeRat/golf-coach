"""Feedback contract (produced by the `feedback` module, consumed by the UI/API).

The final user-facing payload: the score, structured rule-based tips, the LLM
coaching text, and a pointer to the annotated replay video.
"""

from __future__ import annotations

from datetime import datetime
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


class CoachingProvenance(BaseModel):
    """Where the coaching prose came from. Present if and only if `coaching_text` is. [M6]

    The same idea as `ShotProvenance`, for the same reason: coaching is *inferred* from the
    measurements rather than measured, and a reader who cannot tell which is which will read a
    sentence a model wrote as a number this repo stands behind. A UI that finds `coaching_text`
    without this should decline to render it as coaching rather than present it unattributed.
    """

    model: str = Field(description="Exact model id that produced the text, e.g. 'claude-opus-5'.")
    generated_at: datetime
    input_digest: str | None = Field(
        default=None,
        description=(
            "sha256 of the brief the text was generated from. The same trick "
            "`analysis.state.json` uses on its inputs: a stored result can say whether it still "
            "describes the swing beside it, or whether the numbers moved underneath it."
        ),
    )


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
    coaching_text: str | None = Field(
        default=None,
        description=(
            "Claude's coaching prose, written from the measured numbers in this payload (M6). "
            "None when coaching was disabled, unconfigured, or failed — never a placeholder, so "
            "an absent verdict never reads as a neutral one. Always carries `coaching`."
        ),
    )
    coaching: CoachingProvenance | None = Field(
        default=None,
        description="Which model wrote `coaching_text`, and when. None iff `coaching_text` is.",
    )
    annotated_video_path: str | None = Field(
        default=None, description="Path to the rendered overlay video, if generated."
    )
