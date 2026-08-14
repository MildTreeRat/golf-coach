"""YOLOv8 club/ball detection. [M2 — gated on M1.5 spike]

Requires the `vision` extra. Implementation strategy (pure-ML vs marker-assisted vs
fusion) is decided by the M1.5 detectability spike before this is built.
"""

from __future__ import annotations

from collections.abc import Iterable
from typing import TYPE_CHECKING

from golf_coach.contracts.detections import FrameDetections

if TYPE_CHECKING:
    # Type-only: `Frame` holds a numpy image and so cannot live in `contracts/`. Kept out of the
    # runtime graph so this module stays importable without the `vision` extra — see
    # `pose/estimator.py`'s note.
    from golf_coach.capture.source import Frame


def detect_objects(frames: Iterable[Frame]) -> list[FrameDetections]:
    """Run YOLOv8 over frames and return per-frame club/ball detections."""
    raise NotImplementedError("M2: implement YOLOv8 detection (after M1.5 spike).")
