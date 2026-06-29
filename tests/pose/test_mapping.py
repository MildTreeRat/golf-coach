"""Unit tests for the MediaPipe -> FrameKeypoints mapping (no MediaPipe needed).

The pure `_to_frame_keypoints` helper is the part of pose estimation that carries logic,
so it is tested in isolation with a fake landmark object — the heavy MediaPipe call is not
exercised here.
"""

from __future__ import annotations

from dataclasses import dataclass

from golf_coach.contracts.keypoints import NUM_POSE_LANDMARKS, PoseLandmark
from golf_coach.pose.estimator import _to_frame_keypoints


@dataclass
class _FakeLandmark:
    x: float
    y: float
    z: float
    visibility: float


def test_maps_raw_landmarks_in_order() -> None:
    raw = [
        _FakeLandmark(x=i / 100, y=i / 100, z=0.0, visibility=1.0)
        for i in range(NUM_POSE_LANDMARKS)
    ]
    fk = _to_frame_keypoints(raw, frame_index=5, timestamp_ms=200.0)

    assert fk.frame_index == 5
    assert fk.timestamp_ms == 200.0
    assert len(fk.landmarks) == NUM_POSE_LANDMARKS
    wrist = fk.landmark(PoseLandmark.LEFT_WRIST)
    assert wrist.x == 15 / 100
    assert wrist.visibility == 1.0


def test_visibility_is_clamped_into_contract_range() -> None:
    raw = [
        _FakeLandmark(x=0.5, y=0.5, z=0.0, visibility=1.5) for _ in range(NUM_POSE_LANDMARKS)
    ]
    fk = _to_frame_keypoints(raw, frame_index=0, timestamp_ms=0.0)
    assert all(lm.visibility == 1.0 for lm in fk.landmarks)


def test_missing_body_yields_placeholder_frame() -> None:
    fk = _to_frame_keypoints(None, frame_index=2, timestamp_ms=66.7)

    assert fk.frame_index == 2
    assert len(fk.landmarks) == NUM_POSE_LANDMARKS
    assert all(lm.visibility == 0.0 for lm in fk.landmarks)
