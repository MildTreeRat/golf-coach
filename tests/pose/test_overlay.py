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
from golf_coach.pose.overlay import (
    _BONES,
    _GUIDE_COLOR,
    _GUIDE_LINES,
    _JOINT_COLOR,
    _JOINTS,
    _PIVOT_COLOR,
    _PIVOT_POINTS,
    draw_skeleton,
)

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


# --- the rotation picture [M17 P1] ------------------------------------------------------------


def test_the_guide_lines_are_the_shoulder_and_the_hip_segment() -> None:
    """Named, not derived: deriving the expectation would pass whatever `_GUIDE_LINES` said.

    These two and no others, because these two are the segments whose rotation *is* the turn.
    """
    assert _GUIDE_LINES == (
        (PoseLandmark.LEFT_SHOULDER, PoseLandmark.RIGHT_SHOULDER),
        (PoseLandmark.LEFT_HIP, PoseLandmark.RIGHT_HIP),
    )


def test_the_guide_lines_are_still_bones() -> None:
    """The overdraw is on purpose, and a tidy-up that removes it fails here.

    Dropping these pairs from `_BONES` to stop drawing them twice would re-derive `_JOINTS` and
    silently take the shoulder and hip dots with it.
    """
    for pair in _GUIDE_LINES:
        assert pair in _BONES


def test_every_guide_line_is_also_a_pivot_pair() -> None:
    """The derivation itself, the way `_JOINTS` is derived from `_BONES`.

    A guide line the golfer cannot see the centre of is half the picture, so the two lists cannot
    be allowed to drift apart.
    """
    for pair in _GUIDE_LINES:
        assert pair in _PIVOT_POINTS


def test_the_hands_are_the_third_pivot_point_and_carry_no_line() -> None:
    """Drawn, deliberately unmeasured (M14 P3's 0.63-0.68 hand tracking), and not an axis."""
    hands = (PoseLandmark.LEFT_WRIST, PoseLandmark.RIGHT_WRIST)
    assert hands in _PIVOT_POINTS
    assert hands not in _GUIDE_LINES
    assert len(_PIVOT_POINTS) == len(_GUIDE_LINES) + 1


def _rotation_frame(dim: PoseLandmark | None = None) -> FrameKeypoints:
    """A pose with only the six pivot landmarks visible, at known and distinct pixels.

    On a 200x200 canvas: shoulders (60, 60) and (140, 60), hips (70, 120) and (130, 120), wrists
    (80, 90) and (120, 90) — so all three midpoints share x=100 and no two marks collide.
    """
    landmarks = [Landmark(x=0.5, y=0.5, visibility=0.0) for _ in range(NUM_POSE_LANDMARKS)]
    placed = {
        PoseLandmark.LEFT_SHOULDER: (0.3, 0.3),
        PoseLandmark.RIGHT_SHOULDER: (0.7, 0.3),
        PoseLandmark.LEFT_HIP: (0.35, 0.6),
        PoseLandmark.RIGHT_HIP: (0.65, 0.6),
        PoseLandmark.LEFT_WRIST: (0.4, 0.45),
        PoseLandmark.RIGHT_WRIST: (0.6, 0.45),
    }
    for landmark, (x, y) in placed.items():
        visibility = 0.0 if landmark is dim else 1.0
        landmarks[landmark] = Landmark(x=x, y=y, visibility=visibility)
    return FrameKeypoints(frame_index=0, timestamp_ms=0.0, landmarks=landmarks)


def test_the_rotation_marks_land_where_the_swing_turns() -> None:
    """The behavioural half: three midpoint markers, and a shoulder axis that runs past the joints.

    Marker colour at a midpoint is also the "last, on top" pin — a marker drawn before the joints
    or the guide line would be painted over at the hip centre, which sits on its own guide line.
    """
    np = pytest.importorskip("numpy")
    pytest.importorskip("cv2")

    canvas = draw_skeleton(np.zeros((200, 200, 3), dtype=np.uint8), _rotation_frame())

    for label, (y, x) in {"shoulder": (60, 100), "hip": (120, 100), "hands": (90, 100)}.items():
        assert canvas[y, x].tolist() == list(_PIVOT_COLOR), f"no marker at the {label} centre"

    # The hands marker sits *between* the wrists, which are themselves already ordinary joints.
    assert canvas[90, 80].tolist() == list(_JOINT_COLOR), "the left wrist lost its joint dot"
    assert canvas[90, 120].tolist() == list(_JOINT_COLOR), "the right wrist lost its joint dot"

    # The shoulder line, over the white bone it repeats and out past the shoulder at x=60 — the
    # overshoot is what makes it read as an axis rather than as one more bone.
    assert canvas[60, 80].tolist() == list(_GUIDE_COLOR), "the shoulder bone was not emphasised"
    assert canvas[60, 45].tolist() == list(_GUIDE_COLOR), "the shoulder axis stopped at the joint"


def test_a_half_seen_line_is_not_drawn_at_an_invented_angle() -> None:
    """One dim endpoint takes out that pair's line and its marker, and nothing else."""
    np = pytest.importorskip("numpy")
    pytest.importorskip("cv2")

    canvas = draw_skeleton(
        np.zeros((200, 200, 3), dtype=np.uint8), _rotation_frame(dim=PoseLandmark.RIGHT_HIP)
    )

    assert not canvas[120, 100].any(), "a hip centre was drawn from one visible hip"
    assert not canvas[120, 58].any(), "a hip axis was drawn from one visible hip"
    # ... while the whole pairs are untouched, so this is not passing on a blank frame.
    assert canvas[60, 100].tolist() == list(_PIVOT_COLOR), "the shoulder centre went with it"
