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
start is found by **2D wrist speed** — walking **backward** from the top and taking the takeaway
to begin just after the last sustained *quiet* (low-speed) stretch — and impact is the first
frame back at/above address height walking **forward** from the top — the wrist returning to
roughly ball height (fallback: the deepest point of the descent). This matches how Tour Tempo
counts the downswing (to ball contact, ADR-010), so the ~3:1 benchmark stays calibrated.
Anchoring on the top keeps a long pre-swing setup/waggle from being mistaken for the backswing.
Using *speed* (not wrist height) to find the start is what makes tempo believable: the early
takeaway is near-horizontal — the lead wrist moves back at roughly constant height — so an
earlier rule that keyed on the wrist *rising* past address height missed that horizontal move,
landed motion-start mid-takeaway, and collapsed tempo toward ~1:1 (see M4 findings). It is
intentionally simple: tempo only depends on start/top/impact *timing*, not on precise posture.
No numpy, no MediaPipe — just lists, so it runs on the base install (ADR-008).

**Expects smoothed input.** `engine.analyze_swing` runs `smoothing.smooth_keypoints` before
calling this, so the wrist `y` series is already denoised and the top/impact instants are
stable frame-to-frame — this is the real robustness win over the raw-landmark first pass
(which misread a jittery clip as ~1.1:1). It still works on raw keypoints, just noisier.

HARDWARE-REVALIDATE: top/impact are pose-only *proxies* for ball contact. When club/ball
detection (M2) and launch-monitor timing (M3) land, validate them against real impact timing
and against the annotated overlay (see docs/M4_FUNDAMENTALS_PANEL.md).
"""

from __future__ import annotations

from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import PhaseSegment, SwingPhase

_LEAD_WRIST = PoseLandmark.LEFT_WRIST

# Frames whose lead-wrist visibility is below this are treated as unreliable; we hold the
# last good `y` rather than trust a low-confidence jump (MediaPipe convention).
_MIN_VISIBILITY = 0.5

# The address baseline is the mean lead-wrist `y` over the first few frames (the golfer is
# still at setup). Used to locate impact (the wrist's return to ball height).
_ADDRESS_SAMPLE_FRAMES = 5

# Motion start is velocity-anchored. The lead wrist is *still* at setup (and momentarily between
# waggle bobs) but moves continuously once the takeaway begins — including the early, near-
# horizontal part a wrist-*height* rule misses. So we measure 2D wrist speed and, walking back
# from the top, take the takeaway to begin just after the last sustained *quiet* stretch: at least
# `_MOTION_STALL_FRAMES` consecutive frames slower than `_MOTION_QUIET_FRAC` of the swing's peak
# wrist speed. Both are scale- and fps-invariant (a fraction of the swing's own peak speed; a
# frame count small enough to sit inside a real setup dwell). 8% cleanly separates a still setup
# (with small waggle jitter, ~3% of peak on the aaron-swing-2 clip) from the takeaway onset
# (which jumps past ~12%); tuned against that clip's speed profile and the overlay.
_MOTION_QUIET_FRAC = 0.08
_MOTION_STALL_FRAMES = 3

# Half-widths (in frames) of the transition window straddling the top of the backswing and
# of the impact window straddling the return to address height. Small, symmetric, heuristic.
_TRANSITION_HALF_FRAMES = 3
_IMPACT_HALF_FRAMES = 2

# Below this many frames there is no swing to segment.
_MIN_FRAMES = 6


def _lead_wrist_xy(keypoints: list[FrameKeypoints]) -> list[tuple[float, float]]:
    """Lead-wrist `(x, y)` per frame, holding the last confident value through dim frames."""
    points: list[tuple[float, float]] = []
    last_good: tuple[float, float] | None = None
    for frame in keypoints:
        wrist = frame.landmark(_LEAD_WRIST)
        if wrist.visibility >= _MIN_VISIBILITY or last_good is None:
            last_good = (wrist.x, wrist.y)
        points.append(last_good)
    return points


def _impact_frame(ys: list[float], top: int, n: int, address_y: float) -> int:
    """Impact instant: first frame after the top where the wrist is back at/above address height.

    `y` grows downward, so the descending wrist returns to ~ball height as it climbs back to
    `address_y`. Falls back to the deepest point of the descent (max `y` after the top) when
    the wrist never quite reaches the baseline (body shift, a short clip). Reads cleanly
    because `engine` smooths the series first.
    """
    return next(
        (i for i in range(top + 1, n) if ys[i] >= address_y),
        max(range(top, n), key=ys.__getitem__),
    )


def _wrist_speed(xy: list[tuple[float, float]]) -> list[float]:
    """Per-frame 2D lead-wrist speed (frame-to-frame displacement); `0.0` at the first frame."""
    speeds = [0.0]
    for (x0, y0), (x1, y1) in zip(xy, xy[1:], strict=False):
        speeds.append(((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5)
    return speeds


def _motion_start(xy: list[tuple[float, float]], top: int) -> int:
    """Motion start: the frame the sustained takeaway begins, walking back from the top.

    The lead wrist is *still* at setup (and momentarily between waggle bobs) but moves
    continuously once the takeaway begins. We measure 2D wrist speed and walk backward from the
    top; the takeaway begins just after the last **quiet** stretch — `_MOTION_STALL_FRAMES`
    consecutive frames slower than `_MOTION_QUIET_FRAC` of the swing's peak wrist speed. Requiring
    a *run* of quiet frames keeps a single slow smoothed frame mid-takeaway from ending it early,
    and using speed (not wrist height) catches the near-horizontal early takeaway an earlier
    height rule missed. Reads cleanly because `engine` smooths first; falls back to frame 0 when
    the wrist never settles (no detectable setup / continuous motion from the first frame).
    """
    if top <= 0:
        return 0
    speeds = _wrist_speed(xy)
    peak = max(speeds[1 : top + 1], default=0.0)
    if peak <= 0.0:
        return 0
    quiet_threshold = peak * _MOTION_QUIET_FRAC

    quiet = 0
    for i in range(top, -1, -1):
        if speeds[i] < quiet_threshold:
            quiet += 1
            if quiet >= _MOTION_STALL_FRAMES:
                return i + quiet  # first moving frame above the quiet run = takeaway start
        else:
            quiet = 0
    return 0


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
    xy = _lead_wrist_xy(keypoints)
    ys = [y for _, y in xy]

    # Top of backswing: highest hands = global minimum `y`. The clearest anchor.
    top = min(range(n), key=ys.__getitem__)

    # Address baseline from the still frames at the start of the clip.
    sample = min(_ADDRESS_SAMPLE_FRAMES, max(top, 1))
    address_y = sum(ys[:sample]) / sample

    # Motion start: the frame the sustained takeaway begins (see `_motion_start`) — anchored on 2D
    # wrist speed so the near-horizontal early takeaway isn't missed and a waggle isn't mistaken
    # for the backswing.
    motion_start = _motion_start(xy, top)

    # Impact: the wrist's return to address height after the top (see `_impact_frame`).
    impact = _impact_frame(ys, top, n, address_y)

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
