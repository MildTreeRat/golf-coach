"""Synthetic-swing fixtures for the analysis spine (no MediaPipe, no clips).

Fabricates a `FrameKeypoints` timeline whose **lead wrist** rises then falls — the signal
`phases.segment_phases` and the tempo checkpoint read. It also parks a few body landmarks so
the pose-only mechanics checkpoints have something to measure: separated **shoulders** (the
scale ruler), a **head** (nose + both ears) that can drift sideways (head sway), and a
**hip-center** that can drift through the finish (balance). `make_swing` lets a test dial the
backswing/downswing frame counts (tempo) plus optional `head_sway` / `finish_drift`, so one
builder drives the ideal and the fault cases deterministically.

The head moves as a **rigid unit** — nose and both ears translate together — because
`evaluate_head_sway` measures the *ear midpoint*, not the nose (metric definitions v2). Moving
only the nose would model a head that rotates in place, which is the very thing the ear-midpoint
definition is designed to ignore.

It can also prepend a **near-horizontal takeaway** (`takeaway_frames` / `takeaway_x`): the lead
wrist slides sideways at address *height* before the vertical rise begins. That models the early
takeaway a wrist-*height* rule can't see, so tests can exercise the 2D-speed motion-start
detector that must count it as part of the backswing.
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
_HEAD_BASE_X = 0.5
_HIP_BASE_X = 0.5
_WRIST_BASE_X = 0.5

# Half the ear-to-ear span. The ears straddle `head_x` symmetrically, so their midpoint is exactly
# `head_x` — a rigid head translation, with no rotation component for the metric to cancel.
_EAR_HALF_SPAN = 0.03


def _frame(
    index: int, wrist_y: float, wrist_x: float, head_x: float, hip_x: float
) -> FrameKeypoints:
    """One frame: body parked mid-frame, with lead wrist, head, shoulders, hips placed."""
    landmarks = [
        Landmark(x=0.5, y=0.5, z=0.0, visibility=1.0) for _ in range(NUM_POSE_LANDMARKS)
    ]
    landmarks[PoseLandmark.LEFT_WRIST] = Landmark(x=wrist_x, y=wrist_y, z=0.0, visibility=1.0)
    landmarks[PoseLandmark.NOSE] = Landmark(x=head_x, y=0.2, z=0.0, visibility=1.0)
    landmarks[PoseLandmark.LEFT_EAR] = Landmark(
        x=head_x - _EAR_HALF_SPAN, y=0.2, z=0.0, visibility=1.0
    )
    landmarks[PoseLandmark.RIGHT_EAR] = Landmark(
        x=head_x + _EAR_HALF_SPAN, y=0.2, z=0.0, visibility=1.0
    )
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
    takeaway_frames: int = 0,
    takeaway_x: float = 0.16,
    finish_y: float = _FOLLOWTHROUGH_Y,
    followthrough_frames: int = _FOLLOWTHROUGH_FRAMES,
) -> list[FrameKeypoints]:
    """Build a synthetic swing with the given tempo and optional head-sway / finish-drift.

    Default 30:10 → a ~3:1 tempo, steady head, held finish (all three checkpoints pass). The
    wrist sits at address, rises over `backswing_frames` to the top, falls back to address
    height over `downswing_frames` (impact), then rises again. `head_sway` shifts the whole head
    (nose and both ears together) in `x` from address to impact (in normalized units);
    `finish_drift` slides the hip-center sideways across the follow-through.

    With `takeaway_frames > 0`, a near-horizontal takeaway is inserted between the address dwell
    and the vertical rise: the lead wrist slides sideways by `takeaway_x` (normalized) at address
    *height*. This is the part of the backswing a wrist-height rule misses — the true backswing is
    `takeaway_frames + backswing_frames` long, so counting only the vertical rise reads as a
    too-quick tempo.

    `finish_y` sets how high the hands end up (smaller = higher, image coordinates). Passing a
    value **above** `_TOP_Y` models a full tour-style finish, where the hands wrap higher than they
    ever were at the top of the backswing — the swing shape that breaks any "top = highest hands"
    rule. Default keeps the modest finish of the clips this fixture was originally built from.

    `followthrough_frames` sets how long the finish is held. The default 8 is **far shorter than
    real footage** — the GolfDB face-on corpus measures p10 50 / p50 89 frames — and is kept only so
    existing tests are unaffected. Any test about `finish_balance`'s *statistical* behaviour must
    pass a realistic value: `evaluate_finish_balance` summarizes drift at p90, and
    `smooth_keypoints` smears one bad frame across its 5-frame window, so p90 can only reject an
    isolated outlier once 5/n < 0.10 — i.e. beyond ~50 follow-through frames. Below that the window
    is too short for the statistic to have rejection power, and a test using it would silently
    measure nothing.
    """
    swing_frames = backswing_frames + downswing_frames
    lead_pad = _ADDRESS_FRAMES + takeaway_frames  # frames before the vertical rise begins
    takeaway_dx = takeaway_x if takeaway_frames > 0 else 0.0

    ys: list[float] = [_ADDRESS_Y] * lead_pad  # address dwell + flat (horizontal) takeaway
    ys += _ramp(_ADDRESS_Y, _TOP_Y, backswing_frames)
    ys += _ramp(_TOP_Y, _ADDRESS_Y, downswing_frames)
    ys += _ramp(_ADDRESS_Y, finish_y, followthrough_frames)

    # Lead wrist steady in `x` at address, slides sideways through the takeaway, then holds.
    wrist_xs: list[float] = [_WRIST_BASE_X] * _ADDRESS_FRAMES
    wrist_xs += _lerp(_WRIST_BASE_X, _WRIST_BASE_X + takeaway_dx, takeaway_frames)
    wrist_xs += [_WRIST_BASE_X + takeaway_dx] * (swing_frames + followthrough_frames)

    # Head steady through address + takeaway, then drifts to its swayed position by impact, holds.
    head_xs: list[float] = [_HEAD_BASE_X] * lead_pad
    head_xs += _lerp(_HEAD_BASE_X, _HEAD_BASE_X + head_sway, swing_frames)
    head_xs += [_HEAD_BASE_X + head_sway] * followthrough_frames

    # Hips steady through the swing, then drift across the follow-through (balance signal).
    hip_xs: list[float] = [_HIP_BASE_X] * (lead_pad + swing_frames)
    hip_xs += _lerp(_HIP_BASE_X, _HIP_BASE_X + finish_drift, followthrough_frames)

    return [
        _frame(index, y, wrist_x, head_x, hip_x)
        for index, (y, wrist_x, head_x, hip_x) in enumerate(
            zip(ys, wrist_xs, head_xs, hip_xs, strict=True)
        )
    ]


@pytest.fixture
def swing() -> list[FrameKeypoints]:
    """A ~3:1 (ideal-tempo) synthetic swing: steady head, held finish."""
    return make_swing()
