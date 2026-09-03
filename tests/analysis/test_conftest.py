"""What the synthetic swing fixture places, and where. [M14 P2]

`conftest.make_swing` initializes all 33 landmarks at frame centre with `visibility=1.0` and
then overwrites only the ones a current metric reads. That is fine for a landmark nothing
reads — right up until something does, at which point the unplaced landmark clears the
confidence gate while sitting nowhere near the joint it names, and a metric computed from it
returns a plausible non-`None` number with no relationship to the swing. Its test goes green.

Nothing reads landmarks 17-22 yet (M14 P4 is the first thing that will), so no other test in
this repo can notice if their placement is dropped or reverted. That is what these pin.
"""

from __future__ import annotations

import math

from conftest import _GRIP_OFFSET_X, _GRIP_OFFSET_Y, make_swing

from golf_coach.contracts.keypoints import PoseLandmark

_HANDS = (
    (PoseLandmark.LEFT_WRIST, PoseLandmark.LEFT_INDEX),
    (PoseLandmark.LEFT_WRIST, PoseLandmark.LEFT_PINKY),
    (PoseLandmark.LEFT_WRIST, PoseLandmark.LEFT_THUMB),
    (PoseLandmark.RIGHT_WRIST, PoseLandmark.RIGHT_INDEX),
    (PoseLandmark.RIGHT_WRIST, PoseLandmark.RIGHT_PINKY),
    (PoseLandmark.RIGHT_WRIST, PoseLandmark.RIGHT_THUMB),
)

#: A grip's width — the fixture's one unit of hand-scale distance.
_GRIP_LENGTH = math.hypot(_GRIP_OFFSET_X, _GRIP_OFFSET_Y)


def test_hand_landmarks_are_placed_off_frame_centre() -> None:
    """17-22 are somewhere other than the initializer's `(0.5, 0.5)`, within a hand of their wrist.

    The top of the backswing is the frame to check: the lead wrist is at `y=0.15` there, so a
    hand landmark still parked at frame centre is 0.35 away from the wrist it belongs to — and
    an assertion about *distance from the wrist* catches that, where an assertion about the
    value `(0.5, 0.5)` would not once the wrist happens to pass through the middle of the frame.
    """
    swing = make_swing()
    top = min(swing, key=lambda frame: frame.landmarks[PoseLandmark.LEFT_WRIST].y)

    for wrist, part in _HANDS:
        hand = top.landmarks[part]
        anchor = top.landmarks[wrist]
        reach = math.hypot(hand.x - anchor.x, hand.y - anchor.y)
        assert 0.0 < reach <= _GRIP_LENGTH, f"{part.name} sits {reach:.3f} from {wrist.name}"


def test_hand_landmarks_ride_with_their_own_wrist() -> None:
    """The offset from wrist to hand landmark is the same in every frame of the swing.

    Both hands stay on the club for the whole swing, so a fan that drifts relative to its wrist
    describes a golfer whose fingers are coming off the grip. Checked as a constant offset rather
    than a fixed position, because the wrists themselves move — that is the signal the fixture
    exists to produce.
    """
    swing = make_swing(takeaway_frames=6, head_sway=0.05, finish_drift=0.04)

    for wrist, part in _HANDS:
        offsets = {
            (
                round(frame.landmarks[part].x - frame.landmarks[wrist].x, 9),
                round(frame.landmarks[part].y - frame.landmarks[wrist].y, 9),
            )
            for frame in swing
        }
        assert len(offsets) == 1, f"{part.name} drifts relative to {wrist.name}: {offsets}"


def test_the_six_hand_landmarks_are_six_distinct_points() -> None:
    """No two of 17-22 coincide, and none coincides with a wrist.

    A fan collapsed onto one point still passes both tests above, and would let a metric that
    reads the wrong landmark of the six agree with the right one.
    """
    frame = make_swing()[0]
    points = {
        (round(frame.landmarks[part].x, 9), round(frame.landmarks[part].y, 9))
        for part in (
            PoseLandmark.LEFT_WRIST,
            PoseLandmark.RIGHT_WRIST,
            *(part for _, part in _HANDS),
        )
    }
    assert len(points) == 8
