"""Synthetic-swing fixtures for the analysis spine (no MediaPipe, no clips).

Fabricates a `FrameKeypoints` timeline whose **lead wrist** rises then falls — the only
signal `phases.segment_phases` and the tempo checkpoint actually read. `make_swing` lets a
test dial the backswing/downswing frame counts to hit a target tempo ratio, so the same
builder drives the ideal, too-quick, and too-slow cases deterministically.
"""

from __future__ import annotations

import pytest

from golf_coach.contracts.keypoints import (
    NUM_POSE_LANDMARKS,
    FrameKeypoints,
    Landmark,
    PoseLandmark,
)

_FPS = 100.0
_MS_PER_FRAME = 1000.0 / _FPS

# Lead-wrist `y` (image coords, grows downward): high at address, low at the top of backswing.
_ADDRESS_Y = 0.85
_TOP_Y = 0.15
_FOLLOWTHROUGH_Y = 0.35

_ADDRESS_FRAMES = 8
_FOLLOWTHROUGH_FRAMES = 8


def _frame(index: int, wrist_y: float) -> FrameKeypoints:
    """One frame: every landmark parked mid-frame except the lead wrist at `wrist_y`."""
    landmarks = [
        Landmark(x=0.5, y=0.5, z=0.0, visibility=1.0) for _ in range(NUM_POSE_LANDMARKS)
    ]
    landmarks[PoseLandmark.LEFT_WRIST] = Landmark(x=0.5, y=wrist_y, z=0.0, visibility=1.0)
    return FrameKeypoints(
        frame_index=index,
        timestamp_ms=index * _MS_PER_FRAME,
        landmarks=landmarks,
    )


def _ramp(start_y: float, end_y: float, count: int) -> list[float]:
    """`count` values stepping from just past `start_y` to `end_y` (endpoint inclusive)."""
    step = (end_y - start_y) / count
    return [start_y + step * (k + 1) for k in range(count)]


def make_swing(backswing_frames: int = 30, downswing_frames: int = 10) -> list[FrameKeypoints]:
    """Build a synthetic swing with the given backswing/downswing lengths.

    Default 30:10 → a ~3:1 tempo. The wrist sits at address, rises over `backswing_frames` to
    the top, falls back to address height over `downswing_frames` (impact), then rises again.
    """
    ys: list[float] = [_ADDRESS_Y] * _ADDRESS_FRAMES
    ys += _ramp(_ADDRESS_Y, _TOP_Y, backswing_frames)
    ys += _ramp(_TOP_Y, _ADDRESS_Y, downswing_frames)
    ys += _ramp(_ADDRESS_Y, _FOLLOWTHROUGH_Y, _FOLLOWTHROUGH_FRAMES)
    return [_frame(index, y) for index, y in enumerate(ys)]


@pytest.fixture
def swing() -> list[FrameKeypoints]:
    """A ~3:1 (ideal-tempo) synthetic swing."""
    return make_swing()
