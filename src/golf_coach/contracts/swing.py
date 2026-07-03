"""Swing analysis contract (produced by the `analysis` module, consumed by `feedback`).

`SwingResult` is the merged, analyzed view of one swing: the aligned data streams
plus the phase segmentation, per-checkpoint scores, and overall score. The `analysis`
module is a pure functional core — it turns contracts into this contract with no I/O.
"""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel, Field

from golf_coach.contracts.detections import FrameDetections
from golf_coach.contracts.intent import PracticeGoal
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.shot import ShotData


class SwingPhase(StrEnum):
    """The six segments of a golf swing (ROADMAP M4)."""

    ADDRESS = "address"
    BACKSWING = "backswing"
    TRANSITION = "transition"
    DOWNSWING = "downswing"
    IMPACT = "impact"
    FOLLOW_THROUGH = "follow_through"


class PhaseSegment(BaseModel):
    """A contiguous span of frames belonging to one swing phase."""

    phase: SwingPhase
    start_frame: int = Field(ge=0)
    end_frame: int = Field(ge=0)
    start_ms: float = Field(ge=0.0)
    end_ms: float = Field(ge=0.0)


class CheckpointScore(BaseModel):
    """Result of evaluating one swing checkpoint (e.g. 'address posture')."""

    name: str
    score: float = Field(ge=0.0, le=1.0, description="0=fail .. 1=ideal")
    passed: bool
    observed: float | None = Field(default=None, description="Measured value.")
    expected_low: float | None = None
    expected_high: float | None = None
    message: str = ""


class SwingResult(BaseModel):
    """The complete analyzed result for one swing."""

    swing_id: str
    session_id: str

    phases: list[PhaseSegment] = Field(default_factory=list)
    checkpoint_scores: list[CheckpointScore] = Field(default_factory=list)

    # Dual-axis scoring (ADR-009). The practice intent this swing was judged against,
    # plus the two independent sub-scores. `overall_score` is the policy-weighted blend
    # (for the Fundamentals PoC: overall == mechanics_score, outcome_score is None).
    intent: PracticeGoal | None = None
    mechanics_score: float | None = Field(default=None, ge=0.0, le=100.0)
    outcome_score: float | None = Field(default=None, ge=0.0, le=100.0)
    overall_score: float = Field(ge=0.0, le=100.0, description="0-100 policy-weighted blend.")

    # The merged source data this result was computed from. Optional so a lightweight
    # SwingResult (e.g. from storage, scores only) can omit the heavy streams.
    keypoints: list[FrameKeypoints] = Field(default_factory=list)
    detections: list[FrameDetections] = Field(default_factory=list)
    shot: ShotData | None = None
