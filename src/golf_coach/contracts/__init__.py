"""Shared data contracts — the decoupling seam of the whole project (ADR-008).

Every module depends on these data shapes; modules never import each other. Data
flows producer -> contract -> consumer, which is what lets each section be built
and tested independently (e.g. analysis against mock keypoints before pose exists).

This package depends only on pydantic + the stdlib. Keep it that way.
"""

from golf_coach.contracts.detections import (
    BoundingBox,
    Detection,
    FrameDetections,
    ObjectClass,
)
from golf_coach.contracts.feedback import FeedbackPayload, Severity, Tip
from golf_coach.contracts.keypoints import (
    NUM_POSE_LANDMARKS,
    FrameKeypoints,
    Landmark,
    PoseLandmark,
)
from golf_coach.contracts.shot import ShotData, ShotSource
from golf_coach.contracts.swing import (
    CheckpointScore,
    PhaseSegment,
    SwingPhase,
    SwingResult,
)

__all__ = [
    # keypoints
    "FrameKeypoints",
    "Landmark",
    "PoseLandmark",
    "NUM_POSE_LANDMARKS",
    # detections
    "FrameDetections",
    "Detection",
    "BoundingBox",
    "ObjectClass",
    # shot
    "ShotData",
    "ShotSource",
    # swing
    "SwingResult",
    "PhaseSegment",
    "SwingPhase",
    "CheckpointScore",
    # feedback
    "FeedbackPayload",
    "Tip",
    "Severity",
]
