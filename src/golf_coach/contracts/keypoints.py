"""Body pose contracts (produced by the `pose` module, consumed by `analysis`).

These mirror the MediaPipe Pose output: 33 landmarks per frame. Nothing in here
depends on MediaPipe itself — the `pose` module maps MediaPipe's raw output into
these plain data shapes so consumers never import MediaPipe.
"""

from __future__ import annotations

from enum import IntEnum

from pydantic import BaseModel, Field


class PoseLandmark(IntEnum):
    """The 33 MediaPipe Pose landmarks, in MediaPipe's canonical index order.

    Use these names instead of magic indices, e.g. ``frame.landmarks[PoseLandmark.LEFT_WRIST]``.
    The wrist landmarks matter most for golf — they anchor the club shaft when we
    fuse pose with club-head detections (see ADR-007 / M1.5 spike).
    """

    NOSE = 0
    LEFT_EYE_INNER = 1
    LEFT_EYE = 2
    LEFT_EYE_OUTER = 3
    RIGHT_EYE_INNER = 4
    RIGHT_EYE = 5
    RIGHT_EYE_OUTER = 6
    LEFT_EAR = 7
    RIGHT_EAR = 8
    MOUTH_LEFT = 9
    MOUTH_RIGHT = 10
    LEFT_SHOULDER = 11
    RIGHT_SHOULDER = 12
    LEFT_ELBOW = 13
    RIGHT_ELBOW = 14
    LEFT_WRIST = 15
    RIGHT_WRIST = 16
    LEFT_PINKY = 17
    RIGHT_PINKY = 18
    LEFT_INDEX = 19
    RIGHT_INDEX = 20
    LEFT_THUMB = 21
    RIGHT_THUMB = 22
    LEFT_HIP = 23
    RIGHT_HIP = 24
    LEFT_KNEE = 25
    RIGHT_KNEE = 26
    LEFT_ANKLE = 27
    RIGHT_ANKLE = 28
    LEFT_HEEL = 29
    RIGHT_HEEL = 30
    LEFT_FOOT_INDEX = 31
    RIGHT_FOOT_INDEX = 32


NUM_POSE_LANDMARKS = 33


class Landmark(BaseModel):
    """A single body landmark in normalized image coordinates.

    x/y are normalized to [0, 1] relative to image width/height (MediaPipe convention).
    z is depth relative to the hips (roughly normalized, smaller = closer to camera).
    visibility is MediaPipe's confidence the landmark is present and not occluded.
    """

    x: float
    y: float
    z: float = 0.0
    visibility: float = Field(default=0.0, ge=0.0, le=1.0)


class FrameKeypoints(BaseModel):
    """All body landmarks for one video frame."""

    frame_index: int = Field(ge=0)
    timestamp_ms: float = Field(ge=0.0, description="Milliseconds from start of clip.")
    landmarks: list[Landmark] = Field(
        description=f"Exactly {NUM_POSE_LANDMARKS} landmarks, indexed by PoseLandmark.",
    )

    def landmark(self, which: PoseLandmark) -> Landmark:
        """Convenience accessor: ``frame.landmark(PoseLandmark.LEFT_WRIST)``."""
        return self.landmarks[which]
