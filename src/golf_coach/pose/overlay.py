"""Skeleton overlay rendering. [M1]

Draws a `FrameKeypoints` skeleton onto a BGR frame for visual accuracy review. Works purely
off our own contract (no MediaPipe): it denormalizes landmark coordinates back to pixels and
connects them with a small bone list. This proves `FrameKeypoints` carries everything a
consumer needs to visualize a pose. OpenCV is imported lazily so importing this module stays
cheap.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark

if TYPE_CHECKING:
    import numpy as np

# Minimal skeleton topology: which landmarks to connect with a bone. Enough to read body
# posture through a swing without drawing all of MediaPipe's edges.
_BONES: tuple[tuple[PoseLandmark, PoseLandmark], ...] = (
    # arms
    (PoseLandmark.LEFT_SHOULDER, PoseLandmark.LEFT_ELBOW),
    (PoseLandmark.LEFT_ELBOW, PoseLandmark.LEFT_WRIST),
    (PoseLandmark.RIGHT_SHOULDER, PoseLandmark.RIGHT_ELBOW),
    (PoseLandmark.RIGHT_ELBOW, PoseLandmark.RIGHT_WRIST),
    # shoulders + torso
    (PoseLandmark.LEFT_SHOULDER, PoseLandmark.RIGHT_SHOULDER),
    (PoseLandmark.LEFT_SHOULDER, PoseLandmark.LEFT_HIP),
    (PoseLandmark.RIGHT_SHOULDER, PoseLandmark.RIGHT_HIP),
    (PoseLandmark.LEFT_HIP, PoseLandmark.RIGHT_HIP),
    # legs
    (PoseLandmark.LEFT_HIP, PoseLandmark.LEFT_KNEE),
    (PoseLandmark.LEFT_KNEE, PoseLandmark.LEFT_ANKLE),
    (PoseLandmark.RIGHT_HIP, PoseLandmark.RIGHT_KNEE),
    (PoseLandmark.RIGHT_KNEE, PoseLandmark.RIGHT_ANKLE),
)

# A landmark dimmer than this is treated as not-confidently-seen and skipped.
_MIN_VISIBILITY = 0.5

_JOINT_COLOR = (0, 255, 0)  # BGR green
_BONE_COLOR = (255, 255, 255)  # BGR white


def draw_skeleton(
    image: np.ndarray[Any, Any], keypoints: FrameKeypoints
) -> np.ndarray[Any, Any]:
    """Draw the skeleton for one frame onto a copy of `image` (BGR). Returns the copy."""
    import cv2

    canvas = image.copy()
    height, width = canvas.shape[:2]

    def pixel(which: PoseLandmark) -> tuple[int, int] | None:
        lm = keypoints.landmark(which)
        if lm.visibility < _MIN_VISIBILITY:
            return None
        return int(lm.x * width), int(lm.y * height)

    # Bones first so joints render on top.
    for a, b in _BONES:
        pa, pb = pixel(a), pixel(b)
        if pa is not None and pb is not None:
            cv2.line(canvas, pa, pb, _BONE_COLOR, 2)

    for landmark in PoseLandmark:
        p = pixel(landmark)
        if p is not None:
            cv2.circle(canvas, p, 3, _JOINT_COLOR, -1)

    return canvas
