"""Synthetic-swing fixtures for the analysis spine (no MediaPipe, no clips).

Fabricates a `FrameKeypoints` timeline whose **lead wrist** rises then falls — the signal
`phases.segment_phases` and the tempo checkpoint read. It also parks a few body landmarks so
the pose-only mechanics checkpoints have something to measure: separated **shoulders** (the
scale ruler), a **nose** that can drift sideways (head sway), and a **hip-center** that can
drift through the finish (balance). `make_swing` lets a test dial the backswing/downswing frame
counts (tempo) plus optional `head_sway` / `finish_drift`, so one builder drives the ideal and
the fault cases deterministically.
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

# Fixed face-on shoulder span (normalized x) — the scale-invariance ruler (~0.16 wide).
_SHOULDER_LEFT_X = 0.42
_SHOULDER_RIGHT_X = 0.58
_NOSE_BASE_X = 0.5
_HIP_BASE_X = 0.5


def _frame(index: int, wrist_y: float, nose_x: float, hip_x: float) -> FrameKeypoints:
    """One frame: body parked mid-frame, with lead wrist, nose, shoulders, hips placed."""
    landmarks = [
        Landmark(x=0.5, y=0.5, z=0.0, visibility=1.0) for _ in range(NUM_POSE_LANDMARKS)
    ]
    landmarks[PoseLandmark.LEFT_WRIST] = Landmark(x=0.5, y=wrist_y, z=0.0, visibility=1.0)
    landmarks[PoseLandmark.NOSE] = Landmark(x=nose_x, y=0.2, z=0.0, visibility=1.0)
    landmarks[PoseLandmark.LEFT_SHOULDER] = Landmark(x=_SHOULDER_LEFT_X, y=0.4, visibility=1.0)
    landmarks[PoseLandmark.RIGHT_SHOULDER] = Landmark(x=_SHOULDER_RIGHT_X, y=0.4, visibility=1.0)
    landmarks[PoseLandmark.LEFT_HIP] = Landmark(x=hip_x, y=0.6, z=0.0, visibility=1.0)
    landmarks[PoseLandmark.RIGHT_HIP] = Landmark(x=hip_x, y=0.6, z=0.0, visibility=1.0)
    return FrameKeypoints(
        frame_index=index,
        timestamp_ms=index * _MS_PER_FRAME,
        landmarks=landmarks,
    )


def _ramp(start: float, end: float, count: int) -> list[float]:
    """`count` values stepping from just past `start` to `end` (endpoint inclusive)."""
    step = (end - start) / count
    return [start + step * (k + 1) for k in range(count)]


def _lerp(start: float, end: float, count: int) -> list[float]:
    """`count` values evenly from `start` to `end` inclusive (constant for count == 1)."""
    if count <= 1:
        return [end] * count
    return [start + (end - start) * k / (count - 1) for k in range(count)]


def make_swing(
    backswing_frames: int = 30,
    downswing_frames: int = 10,
    head_sway: float = 0.0,
    finish_drift: float = 0.0,
) -> list[FrameKeypoints]:
    """Build a synthetic swing with the given tempo and optional head-sway / finish-drift.

    Default 30:10 → a ~3:1 tempo, steady head, held finish (all three checkpoints pass). The
    wrist sits at address, rises over `backswing_frames` to the top, falls back to address
    height over `downswing_frames` (impact), then rises again. `head_sway` shifts the nose `x`
    from address to impact (in normalized units); `finish_drift` slides the hip-center sideways
    across the follow-through.
    """
    swing_frames = backswing_frames + downswing_frames

    ys: list[float] = [_ADDRESS_Y] * _ADDRESS_FRAMES
    ys += _ramp(_ADDRESS_Y, _TOP_Y, backswing_frames)
    ys += _ramp(_TOP_Y, _ADDRESS_Y, downswing_frames)
    ys += _ramp(_ADDRESS_Y, _FOLLOWTHROUGH_Y, _FOLLOWTHROUGH_FRAMES)

    # Nose steady through address, then drifts to its swayed position by impact, then holds.
    nose_xs: list[float] = [_NOSE_BASE_X] * _ADDRESS_FRAMES
    nose_xs += _lerp(_NOSE_BASE_X, _NOSE_BASE_X + head_sway, swing_frames)
    nose_xs += [_NOSE_BASE_X + head_sway] * _FOLLOWTHROUGH_FRAMES

    # Hips steady through the swing, then drift across the follow-through (balance signal).
    hip_xs: list[float] = [_HIP_BASE_X] * (_ADDRESS_FRAMES + swing_frames)
    hip_xs += _lerp(_HIP_BASE_X, _HIP_BASE_X + finish_drift, _FOLLOWTHROUGH_FRAMES)

    return [
        _frame(index, y, nose_x, hip_x)
        for index, (y, nose_x, hip_x) in enumerate(zip(ys, nose_xs, hip_xs, strict=True))
    ]


@pytest.fixture
def swing() -> list[FrameKeypoints]:
    """A ~3:1 (ideal-tempo) synthetic swing: steady head, held finish."""
    return make_swing()
