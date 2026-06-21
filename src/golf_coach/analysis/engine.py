"""Swing analysis entry point. [M4]

Pure function: takes the merged data streams and produces a SwingResult. Internally
this will call merge -> segment phases -> evaluate checkpoints -> score (each its own
module under analysis/). Stubbed now; the contract is what downstream code targets.
"""

from __future__ import annotations

from golf_coach.contracts.detections import FrameDetections
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import SwingResult


def analyze_swing(
    swing_id: str,
    session_id: str,
    keypoints: list[FrameKeypoints],
    detections: list[FrameDetections] | None = None,
    shot: ShotData | None = None,
) -> SwingResult:
    """Analyze one swing from its merged data streams."""
    raise NotImplementedError("M4: implement merge -> phases -> checkpoints -> scoring.")
