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
    camera_id: str | None = Field(
        default=None,
        description=(
            "Which camera produced this frame (ADR-011). None means not recorded — the state of "
            "every file written before M7 Phase 1. Free-form by design: the value is whatever the "
            "`VideoSource` was told it is. M7 uses the same words as `storage.manifest.Role` "
            "('face_on', 'down_the_line') by convention, not by import."
        ),
    )

    def landmark(self, which: PoseLandmark) -> Landmark:
        """Convenience accessor: ``frame.landmark(PoseLandmark.LEFT_WRIST)``."""
        return self.landmarks[which]


class ClipMetadata(BaseModel):
    """What the source clip was, recorded alongside the pose extracted from it.

    Every field is optional: these are facts about a video file, and a `FrameKeypoints` list can
    legitimately exist without one (synthetic test fixtures, a corpus whose videos are long gone).
    None means unknown, never zero.

    **`fps` is the container's claim, not measured real time.** It is `CAP_PROP_FPS` as reported,
    which iPhone slo-mo is known to stretch — see docs/M7_TWO_PHONE_SPIKE.md Q3. If that spike shows
    the reported rate does not describe real time, a companion real-time rate lands here as a pure
    field addition; nothing already written needs migrating.
    """

    fps: float | None = Field(default=None, gt=0.0, description="Container-reported frames/second.")
    width: int | None = Field(default=None, gt=0)
    height: int | None = Field(default=None, gt=0)
    frame_count: int | None = Field(
        default=None,
        ge=0,
        description=(
            "Frames actually DECODED, which is the honest number — a container can claim more "
            "frames than it yields, and a silently-short decode is the failure mode this records."
        ),
    )
    source_sha256: str | None = Field(
        default=None, description="Content hash of the source video, tying pose back to its clip."
    )


class KeypointsFile(BaseModel):
    """The on-disk shape of a `*.keypoints.json`: the frames plus what clip they came from.

    Files written before M7 Phase 1 are a bare JSON array of frames with no envelope. Read through
    `storage.keypoints_io.load_keypoints`, which accepts both and reports `clip=None` for the old
    shape, rather than parsing this model directly.
    """

    clip: ClipMetadata | None = None
    frames: list[FrameKeypoints]
