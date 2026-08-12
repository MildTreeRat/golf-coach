"""Shared data contracts — the decoupling seam of the whole project (ADR-008).

Every module depends on these data shapes; modules never import each other. Data
flows producer -> contract -> consumer, which is what lets each section be built
and tested independently (e.g. analysis against mock keypoints before pose exists).

This package depends only on pydantic + the stdlib. Keep it that way.
"""

from golf_coach.contracts.alignment import (
    TAU_IMPACT,
    TAU_MOTION_START,
    TAU_TOP,
    AlignmentQuality,
    ClipAlignment,
    SwingAlignment,
    SwingAnchors,
)
from golf_coach.contracts.career import (
    CareerCorpus,
    CorpusSwing,
    ExcludedSwing,
    ExclusionReason,
)
from golf_coach.contracts.detections import (
    BoundingBox,
    Detection,
    FrameDetections,
    ObjectClass,
)
from golf_coach.contracts.feedback import FeedbackPayload, Severity, Tip
from golf_coach.contracts.golfer import Golfer, Handedness, slugify
from golf_coach.contracts.intent import (
    ClubCategory,
    PlayerProfile,
    PracticeGoal,
    PracticeMode,
    TargetShape,
)
from golf_coach.contracts.keypoints import (
    NUM_POSE_LANDMARKS,
    FrameKeypoints,
    Landmark,
    PoseLandmark,
)
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.contracts.swing import (
    ANALYSIS_VERSION,
    CheckpointScore,
    Measurement,
    PhaseSegment,
    SwingBundleResult,
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
    "ShotProvenance",
    # intent
    "PracticeGoal",
    "PracticeMode",
    "TargetShape",
    "ClubCategory",
    "PlayerProfile",
    # golfer
    "Golfer",
    "Handedness",
    "slugify",
    # swing
    "SwingResult",
    "SwingBundleResult",
    "PhaseSegment",
    "SwingPhase",
    "CheckpointScore",
    "Measurement",
    "ANALYSIS_VERSION",
    # career
    "CareerCorpus",
    "CorpusSwing",
    "ExcludedSwing",
    "ExclusionReason",
    # feedback
    "FeedbackPayload",
    "Tip",
    "Severity",
    # alignment
    "SwingAnchors",
    "SwingAlignment",
    "ClipAlignment",
    "AlignmentQuality",
    "TAU_MOTION_START",
    "TAU_TOP",
    "TAU_IMPACT",
]
