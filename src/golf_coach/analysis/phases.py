"""Swing phase segmentation — pure, stdlib only. [M4-PoC]

Splits a `FrameKeypoints` timeline into the six `SwingPhase` spans by tracking the
**lead wrist** (`LEFT_WRIST`) vertical position in a face-on view (the canonical pose-camera
placement — see M1 findings / ADR-003 addendum). In image coordinates `y` grows *downward*,
so during the swing the wrist traces: rest (high `y`) → rises through the backswing (`y`
falls to a minimum at the top) → falls back through the downswing (`y` climbs) → impact near
address height → follow-through (rises again).

We only need three instants for the tempo checkpoint — **motion start**, **top of
backswing**, and **impact**. The top (highest hands = global minimum `y`) is the cleanest,
least ambiguous signal, so we anchor on it and derive the other two *relative* to it: motion
start is the last frame at/above address height while walking **backward** from the top (the
start of the final rise), and impact is the first frame back at address height walking
forward. Anchoring on the top keeps a long pre-swing setup/waggle from being mistaken for the
backswing (see M4 findings). It is intentionally simple and a little noisy: tempo only depends
on start/top/impact *timing*, not on precise posture. No numpy, no MediaPipe — just lists, so
it runs on the base install (ADR-008).
"""

from __future__ import annotations

from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import PhaseSegment, SwingPhase

_LEAD_WRIST = PoseLandmark.LEFT_WRIST

# Frames whose lead-wrist visibility is below this are treated as unreliable; we hold the
# last good `y` rather than trust a low-confidence jump (MediaPipe convention).
_MIN_VISIBILITY = 0.5

# The address baseline is the mean lead-wrist `y` over the first few frames (the golfer is
# still at setup). Motion start is where the wrist last sat within this tolerance of that
# baseline before rising into the top.
_ADDRESS_SAMPLE_FRAMES = 5
_MOTION_EPSILON = 0.01

# Half-widths (in frames) of the transition window straddling the top of the backswing and
# of the impact window straddling the return to address height. Small, symmetric, heuristic.
_TRANSITION_HALF_FRAMES = 3
_IMPACT_HALF_FRAMES = 2

# Below this many frames there is no swing to segment.
_MIN_FRAMES = 6


def _lead_wrist_y(keypoints: list[FrameKeypoints]) -> list[float]:
    """Lead-wrist `y` per frame, holding the last confident value through low-visibility frames."""
    ys: list[float] = []
    last_good: float | None = None
    for frame in keypoints:
        wrist = frame.landmark(_LEAD_WRIST)
        if wrist.visibility >= _MIN_VISIBILITY or last_good is None:
            last_good = wrist.y
        ys.append(last_good)
    return ys


def _segment(phase: SwingPhase, start: int, end: int, ts: list[float]) -> PhaseSegment:
    return PhaseSegment(
        phase=phase,
        start_frame=start,
        end_frame=end,
        start_ms=ts[start],
        end_ms=ts[end],
    )


def segment_phases(keypoints: list[FrameKeypoints]) -> list[PhaseSegment]:
    """Segment a keypoint timeline into the six swing phases (in canonical order).

    Returns an empty list for a clip too short to contain a swing. The returned segments are
    contiguous and their frame indices are monotonic non-decreasing, so a consumer can read
    phase timings straight off the boundaries.
    """
    n = len(keypoints)
    if n < _MIN_FRAMES:
        return []

    ts = [frame.timestamp_ms for frame in keypoints]
    ys = _lead_wrist_y(keypoints)

    # Top of backswing: highest hands = global minimum `y`. The clearest anchor.
    top = min(range(n), key=ys.__getitem__)

    # Address baseline from the still frames at the start of the clip.
    sample = min(_ADDRESS_SAMPLE_FRAMES, max(top, 1))
    address_y = sum(ys[:sample]) / sample

    # Motion start: walking back from the top, the last frame the wrist was still at/above
    # address height — i.e. the start of the final rise, not the earlier setup/waggle.
    motion_start = next(
        (i for i in range(top - 1, -1, -1) if ys[i] >= address_y - _MOTION_EPSILON),
        0,
    )

    # Impact: first frame after the top where the wrist has returned to address height
    # (fallback: the lowest point — max `y` — after the top).
    impact = next(
        (i for i in range(top + 1, n) if ys[i] >= address_y),
        max(range(top, n), key=ys.__getitem__),
    )

    # Bracket a small symmetric window around the top (transition) and after impact, then
    # clamp everything into a monotonic, non-overlapping boundary chain.
    b0 = 0
    b1 = motion_start
    b2 = max(b1, top - _TRANSITION_HALF_FRAMES)
    b3 = min(impact, top + _TRANSITION_HALF_FRAMES)
    b3 = max(b3, b2)
    b4 = max(impact, b3)
    b5 = min(n - 1, b4 + _IMPACT_HALF_FRAMES)
    b6 = n - 1
    b5 = max(b5, b4)

    return [
        _segment(SwingPhase.ADDRESS, b0, b1, ts),
        _segment(SwingPhase.BACKSWING, b1, b2, ts),
        _segment(SwingPhase.TRANSITION, b2, b3, ts),
        _segment(SwingPhase.DOWNSWING, b3, b4, ts),
        _segment(SwingPhase.IMPACT, b4, b5, ts),
        _segment(SwingPhase.FOLLOW_THROUGH, b5, b6, ts),
    ]
