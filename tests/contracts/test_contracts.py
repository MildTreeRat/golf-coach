"""Exercise the contracts seam end-to-end (no heavy deps required).

If these pass, the decoupling backbone is real: every module can import and round-trip
the shared data shapes, and the mock shot source produces valid ShotData.
"""

from __future__ import annotations

from golf_coach.contracts import (
    NUM_POSE_LANDMARKS,
    BoundingBox,
    ClubCategory,
    Detection,
    FrameDetections,
    FrameKeypoints,
    Landmark,
    ObjectClass,
    PoseLandmark,
    PracticeGoal,
    PracticeMode,
    ShotData,
    SwingResult,
    TargetShape,
)
from golf_coach.launch_monitor import MockShotDataSource


def _metrics(shots: list[ShotData]) -> list[dict]:
    """Dump shots without the wall-clock timestamp, for determinism checks."""
    return [s.model_dump(exclude={"timestamp"}) for s in shots]


def _full_landmarks() -> list[Landmark]:
    return [Landmark(x=0.5, y=0.5, z=0.0, visibility=1.0) for _ in range(NUM_POSE_LANDMARKS)]


def test_frame_keypoints_named_access() -> None:
    frame = FrameKeypoints(frame_index=0, timestamp_ms=0.0, landmarks=_full_landmarks())
    wrist = frame.landmark(PoseLandmark.LEFT_WRIST)
    assert wrist is frame.landmarks[15]
    assert len(frame.landmarks) == 33


def test_detection_center_and_filter() -> None:
    club = Detection(
        object_class=ObjectClass.CLUB_HEAD,
        bbox=BoundingBox(x1=0.4, y1=0.4, x2=0.6, y2=0.6),
        confidence=0.9,
    )
    frame = FrameDetections(frame_index=0, timestamp_ms=0.0, detections=[club])
    assert frame.of(ObjectClass.CLUB_HEAD) == [club]
    assert frame.of(ObjectClass.BALL) == []
    assert club.bbox.center == (0.5, 0.5)


def test_contract_json_roundtrip() -> None:
    frame = FrameKeypoints(frame_index=3, timestamp_ms=25.0, landmarks=_full_landmarks())
    restored = FrameKeypoints.model_validate_json(frame.model_dump_json())
    assert restored == frame


def test_practice_goal_defaults_to_fundamentals() -> None:
    goal = PracticeGoal()
    assert goal.mode is PracticeMode.FUNDAMENTALS
    assert goal.club is ClubCategory.ALL
    assert goal.target_shape is None


def test_swing_result_dual_axis_roundtrip() -> None:
    goal = PracticeGoal(mode=PracticeMode.FUNDAMENTALS, target_shape=TargetShape.FADE)
    result = SwingResult(
        swing_id="s1",
        session_id="sess1",
        overall_score=82.5,
        mechanics_score=82.5,
        outcome_score=None,
        intent=goal,
    )
    restored = SwingResult.model_validate_json(result.model_dump_json())
    assert restored == result
    assert restored.intent is not None
    assert restored.intent.target_shape is TargetShape.FADE
    assert restored.outcome_score is None


def test_mock_shot_source_is_deterministic_and_valid() -> None:
    a = MockShotDataSource(seed=42).recent(5)
    b = MockShotDataSource(seed=42).recent(5)
    assert len(a) == 5
    assert all(isinstance(s, ShotData) for s in a)
    # Metrics are deterministic by seed; timestamp is wall-clock, so exclude it.
    assert _metrics(a) == _metrics(b)
    assert a[0].smash_factor is not None
