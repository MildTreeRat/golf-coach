"""What the overlay draws, and what it must not. [M14 P1]

The defect this pins is invisible to every other test in the repo: the dot loop and the bone
list were two independent lists, and a wrong dot set renders a slightly-cluttered picture that
still encodes and still plays. Nothing downstream reads an `aligned.mp4` back, so the only
thing that can catch a drifted dot set is an assertion about the dot set.

The topology tests need no OpenCV — `overlay` is import-cheap by contract
(`tests/api/test_pipeline_imports.py`), and that is exactly what lets them run on a base
install. Only the ink test asks for the vision extra.
"""

from __future__ import annotations

import pytest

from golf_coach.contracts.keypoints import (
    NUM_POSE_LANDMARKS,
    FrameKeypoints,
    Landmark,
    PoseLandmark,
)
from golf_coach.pose.overlay import _BONES, _JOINTS, draw_skeleton

#: The thirteen landmarks the overlay used to draw with no bone touching them and no checkpoint
#: reading them — nine of the eleven head points (the two ears are the exception, below) and the
#: four foot points. Named rather than derived: deriving this list from `_BONES` would make the test
#: restate the implementation and pass no matter what `_BONES` said.
_NEVER_DRAWN = (
    PoseLandmark.NOSE,
    PoseLandmark.LEFT_EYE_INNER,
    PoseLandmark.LEFT_EYE,
    PoseLandmark.LEFT_EYE_OUTER,
    PoseLandmark.RIGHT_EYE_INNER,
    PoseLandmark.RIGHT_EYE,
    PoseLandmark.RIGHT_EYE_OUTER,
    PoseLandmark.MOUTH_LEFT,
    PoseLandmark.MOUTH_RIGHT,
    PoseLandmark.LEFT_HEEL,
    PoseLandmark.RIGHT_HEEL,
    PoseLandmark.LEFT_FOOT_INDEX,
    PoseLandmark.RIGHT_FOOT_INDEX,
)


def test_every_dot_is_a_bone_endpoint() -> None:
    """The derivation itself — the two lists can no longer drift apart."""
    assert set(_JOINTS) == {lm for bone in _BONES for lm in bone}


def test_the_unmeasured_head_and_foot_landmarks_are_not_drawn() -> None:
    """Eyes, mouth, nose and feet: extracted, stored, and read by nothing."""
    for landmark in _NEVER_DRAWN:
        assert landmark not in _JOINTS, f"{landmark.name} is drawn but nothing measures it"


def test_the_ears_survive_because_head_sway_is_measured_between_them() -> None:
    """The two head points that are *not* clutter.

    `measure.head_center_points` takes the midpoint of these two as the head centre, so this is
    the one head bone that shows a scored quantity. A future tidy-up that removes the head
    entirely fails here.
    """
    assert (PoseLandmark.LEFT_EAR, PoseLandmark.RIGHT_EAR) in _BONES
    assert PoseLandmark.LEFT_EAR in _JOINTS
    assert PoseLandmark.RIGHT_EAR in _JOINTS


@pytest.mark.parametrize(
    ("wrist", "fingers"),
    [
        (
            PoseLandmark.LEFT_WRIST,
            (PoseLandmark.LEFT_INDEX, PoseLandmark.LEFT_PINKY, PoseLandmark.LEFT_THUMB),
        ),
        (
            PoseLandmark.RIGHT_WRIST,
            (PoseLandmark.RIGHT_INDEX, PoseLandmark.RIGHT_PINKY, PoseLandmark.RIGHT_THUMB),
        ),
    ],
)
def test_each_hand_is_a_three_spoke_fan_off_its_wrist(
    wrist: PoseLandmark, fingers: tuple[PoseLandmark, ...]
) -> None:
    """Structure, not specks. A fan can be reviewed by eye; three loose dots cannot.

    Reviewability is what M14 P3 needs before its reliability numbers are worth believing.
    """
    for finger in fingers:
        assert (wrist, finger) in _BONES


def test_the_overlay_leaves_no_ink_on_a_landmark_it_does_not_draw() -> None:
    """The behavioural half: a drifted dot set is caught in pixels, not just in a tuple.

    Everything drawn is parked at frame centre and the nose is sent to a far corner, so any mark
    in that corner can only have come from drawing the nose.
    """
    np = pytest.importorskip("numpy")
    pytest.importorskip("cv2")

    landmarks = [Landmark(x=0.5, y=0.5, visibility=1.0) for _ in range(NUM_POSE_LANDMARKS)]
    landmarks[PoseLandmark.NOSE] = Landmark(x=0.1, y=0.1, visibility=1.0)
    landmarks[PoseLandmark.RIGHT_WRIST] = Landmark(x=0.5, y=0.5, visibility=1.0)
    landmarks[PoseLandmark.RIGHT_INDEX] = Landmark(x=0.8, y=0.5, visibility=1.0)
    frame = FrameKeypoints(frame_index=0, timestamp_ms=0.0, landmarks=landmarks)

    canvas = draw_skeleton(np.zeros((100, 100, 3), dtype=np.uint8), frame)

    assert not canvas[5:16, 5:16].any(), "the nose was drawn"
    # ... and the fan really is rendered, so the assertion above is not passing by drawing nothing.
    assert canvas[50, 70].any(), "the wrist-to-index bone was not drawn"
