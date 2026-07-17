"""Landmark smoothing: denoises x/y, preserves metadata, respects visibility."""

from __future__ import annotations

from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import (
    NUM_POSE_LANDMARKS,
    FrameKeypoints,
    Landmark,
    PoseLandmark,
)

_W = PoseLandmark.LEFT_WRIST


def _frame(index: int, wrist_x: float, visibility: float = 1.0) -> FrameKeypoints:
    landmarks = [Landmark(x=0.5, y=0.5, visibility=1.0) for _ in range(NUM_POSE_LANDMARKS)]
    landmarks[_W] = Landmark(x=wrist_x, y=0.5, visibility=visibility)
    return FrameKeypoints(frame_index=index, timestamp_ms=index * 10.0, landmarks=landmarks)


def _variance(values: list[float]) -> float:
    mean = sum(values) / len(values)
    return sum((v - mean) ** 2 for v in values) / len(values)


def test_smoothing_reduces_jitter() -> None:
    xs = [0.5, 0.65, 0.4, 0.62, 0.38, 0.55, 0.45]  # zig-zag around ~0.5
    smoothed = smooth_keypoints([_frame(i, x) for i, x in enumerate(xs)], window=5)
    out = [f.landmark(_W).x for f in smoothed]
    assert _variance(out) < _variance(xs)


def test_preserves_index_timestamp_and_count() -> None:
    frames = [_frame(i, 0.5) for i in range(6)]
    smoothed = smooth_keypoints(frames)
    assert len(smoothed) == len(frames)
    for original, result in zip(frames, smoothed, strict=True):
        assert result.frame_index == original.frame_index
        assert result.timestamp_ms == original.timestamp_ms
        assert len(result.landmarks) == NUM_POSE_LANDMARKS


def test_low_visibility_outlier_is_down_weighted() -> None:
    # A single wild but zero-confidence sample must not drag its neighbours' smoothed value.
    frames = [
        _frame(0, 0.5),
        _frame(1, 0.5),
        _frame(2, 5.0, visibility=0.0),
        _frame(3, 0.5),
        _frame(4, 0.5),
    ]
    smoothed = smooth_keypoints(frames, window=5)
    assert abs(smoothed[2].landmark(_W).x - 0.5) < 0.1


def test_all_zero_visibility_keeps_raw_value() -> None:
    frames = [_frame(i, 0.3, visibility=0.0) for i in range(5)]
    smoothed = smooth_keypoints(frames)
    assert smoothed[2].landmark(_W).x == 0.3  # no confident samples -> keep the raw coordinate


def test_window_one_returns_equivalent_copy() -> None:
    frames = [_frame(i, 0.1 * i) for i in range(4)]
    smoothed = smooth_keypoints(frames, window=1)
    for original, result in zip(frames, smoothed, strict=True):
        assert result.landmark(_W).x == original.landmark(_W).x


def test_empty_timeline() -> None:
    assert smooth_keypoints([]) == []
