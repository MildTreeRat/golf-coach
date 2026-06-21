"""MediaPipe pose estimation. [M1]

Implementation lands in Milestone 1. Signature is fixed now so downstream code can be
written against it. Requires the `vision` extra (`pip install -e '.[vision]'`).
"""

from __future__ import annotations

from collections.abc import Iterable

from golf_coach.capture.source import Frame
from golf_coach.contracts.keypoints import FrameKeypoints


def estimate_pose(frames: Iterable[Frame]) -> list[FrameKeypoints]:
    """Run MediaPipe Pose over frames and return one FrameKeypoints per frame."""
    raise NotImplementedError("M1: implement MediaPipe pose estimation.")
