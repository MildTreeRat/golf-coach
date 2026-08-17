"""The swing as a path through time — the feature vector the trajectory model reads. [M8.1]

`measure.py` produces scalars at instants. This produces the **shape of the whole motion**: every
tracked landmark, resampled onto a normalised event axis, hip-relative and shoulder-width scaled.
It is still measuring, not judging — nothing here reads a band, a basis or a quantile, and
`benchmarks/trajectory.py` is what turns the vector into a placement.

**This module is the single implementation, and that is deliberate.** The fitting script
(`scripts/golfdb/derive_trajectory_model.py`) imports it rather than keeping a numpy copy, because
a feature vector built two ways is a model scored against numbers that were never fitted to it —
a failure that produces plausible output and no error. The script's only extra step is GolfDB's
pixel-aspect correction, which belongs to that corpus rather than to this transform.

Stdlib only, like the rest of `analysis/` (ADR-008).
"""

from __future__ import annotations

from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import PhaseSegment, SwingPhase

#: Landmark name -> index, for the twelve the trajectory model tracks. Names rather than raw
#: indices because the shipped artifact stores names: an artifact that said `[7, 8, 11, ...]` would
#: be unreadable, and worse, would silently survive a landmark being swapped out from under it.
LANDMARK_INDEX: dict[str, PoseLandmark] = {
    "left_ear": PoseLandmark.LEFT_EAR,
    "right_ear": PoseLandmark.RIGHT_EAR,
    "left_shoulder": PoseLandmark.LEFT_SHOULDER,
    "right_shoulder": PoseLandmark.RIGHT_SHOULDER,
    "left_elbow": PoseLandmark.LEFT_ELBOW,
    "right_elbow": PoseLandmark.RIGHT_ELBOW,
    "left_wrist": PoseLandmark.LEFT_WRIST,
    "right_wrist": PoseLandmark.RIGHT_WRIST,
    "left_hip": PoseLandmark.LEFT_HIP,
    "right_hip": PoseLandmark.RIGHT_HIP,
    "left_knee": PoseLandmark.LEFT_KNEE,
    "right_knee": PoseLandmark.RIGHT_KNEE,
    # Ankles are here for the down-the-line set rather than the face-on one. From behind, the lead
    # arm is hidden by the torso (elbow and wrist track in under half of frames,
    # M4_POSE_BAKEOFF §Phase G), so that model drops the lead arm and takes the feet instead —
    # visible in ~1.00 of frames from either camera. A name absent from this map makes
    # `build_trajectory` return None for every swing, which is silent rather than loud, so the map
    # must cover the union of every view's list and not just the one that came first.
    "left_ankle": PoseLandmark.LEFT_ANKLE,
    "right_ankle": PoseLandmark.RIGHT_ANKLE,
}

MIN_VISIBILITY = 0.5
MIN_SHOULDER_WIDTH = 0.02
#: A landmark missing more of its timeline than this is being invented rather than bridged.
MAX_MISSING = 0.40


def anchors_from_phases(phases: list[PhaseSegment]) -> tuple[float, float, float] | None:
    """The three instants the model is anchored on — address, top, impact — or None.

    Recovered from the boundary chain `segment_phases` builds rather than re-detected:

    - **address** is the end of the ADDRESS segment, which *is* `motion_start` — the frame the
      sustained takeaway begins, and the closest thing this pipeline has to GolfDB's "address".
    - **top** is the midpoint of TRANSITION, which brackets the detected top symmetrically. Reading
      the midpoint rather than a boundary means the clamping at either end (against motion start or
      against impact) degrades the estimate instead of destroying it.
    - **impact** is the start of the IMPACT segment, which *is* the detected impact frame.

    The other four events GolfDB annotates — toe-up, mid-backswing, mid-downswing,
    mid-follow-through — are deliberately not here. This pipeline does not detect them, and a model
    anchored on instants it cannot produce could never score a real swing.
    """
    by_phase = {segment.phase: segment for segment in phases}
    address = by_phase.get(SwingPhase.ADDRESS)
    transition = by_phase.get(SwingPhase.TRANSITION)
    impact = by_phase.get(SwingPhase.IMPACT)
    if address is None or transition is None or impact is None:
        return None

    anchors = (
        float(address.end_frame),
        (transition.start_frame + transition.end_frame) / 2.0,
        float(impact.start_frame),
    )
    # Strictly increasing or the resampling below divides by zero and smears one phase into
    # another. A collapsed window means detection failed, which is a refusal rather than a fixup.
    if not (anchors[0] < anchors[1] < anchors[2]):
        return None
    return anchors


def sample_positions(anchors: tuple[float, ...], steps: int) -> list[float]:
    """Frame positions for `steps` samples evenly spaced in *event* time, not clock time.

    Event-time runs 0..len(anchors)-1 with the integers landing on the anchors; each sample maps
    back to a fractional frame by interpolating between the two anchors bracketing it. A
    slow-motion clip and a real-time clip of the same swing therefore yield the same sample
    positions — which is the only reason a corpus that is ~47% slow-motion can be pooled at all,
    and the reason a phone at 60fps can be compared to broadcast footage.
    """
    span = len(anchors) - 1
    out = []
    for i in range(steps):
        t = span * i / (steps - 1)
        low = min(int(t), span - 1)
        out.append(anchors[low] + (t - low) * (anchors[low + 1] - anchors[low]))
    return out


def _interpolate_gaps(columns: list[list[float | None]]) -> list[list[float]] | None:
    """Bridge low-visibility gaps per column, or None if any column is too holey to trust."""
    filled: list[list[float]] = []
    for column in columns:
        known = [i for i, v in enumerate(column) if v is not None]
        if not known or (len(column) - len(known)) / len(column) > MAX_MISSING:
            return None
        # Known values pulled out first so the interpolation below works over plain floats and
        # needs no narrowing dance — the alternative is three `type: ignore`s guarding arithmetic
        # that is only ambiguous to the checker.
        anchors: list[tuple[int, float]] = [(i, v) for i, v in enumerate(column) if v is not None]
        out: list[float] = []
        for i, value in enumerate(column):
            if value is not None:
                out.append(value)
                continue
            before = [(k, v) for k, v in anchors if k < i]
            after = [(k, v) for k, v in anchors if k > i]
            if not before:
                out.append(after[0][1])  # leading gap: hold the first known value
            elif not after:
                out.append(before[-1][1])  # trailing gap: hold the last known value
            else:
                (lo_i, lo_v), (hi_i, hi_v) = before[-1], after[0]
                out.append(lo_v + (i - lo_i) / (hi_i - lo_i) * (hi_v - lo_v))
        filled.append(out)
    return filled


def build_trajectory(
    keypoints: list[FrameKeypoints],
    anchors: tuple[float, ...],
    steps: int,
    landmarks: list[str],
    axes: list[str],
    *,
    mirror: bool = False,
) -> list[float] | None:
    """One swing as a flat `steps * len(landmarks) * len(axes)` vector, or None if unmeasurable.

    Row-major by timestep, so the vector reads as "the whole body at t=0, then t=1, …" and a slice
    of it is a moment rather than a landmark's history. The shipped basis is fitted in this order;
    changing it silently re-pairs every coefficient with the wrong coordinate.

    `mirror` folds a left-handed swing onto the right-handed convention a face-on camera implies.
    That is a sign flip on `x` **and** a swap of every left/right pair — flipping without swapping
    would put the lead arm on the trail side and score the golfer against a shape nobody swings.
    """
    if len(keypoints) < 2 or steps < 2:
        return None

    if any(name not in LANDMARK_INDEX for name in landmarks):
        return None
    indices = [LANDMARK_INDEX[name] for name in landmarks]

    positions = sample_positions(anchors, steps)
    if positions[0] < 0 or positions[-1] > len(keypoints) - 1:
        return None

    columns: list[list[float | None]] = [[] for _ in range(len(landmarks) * len(axes))]
    widths: list[float] = []

    for position in positions:
        low = int(position)
        high = min(low + 1, len(keypoints) - 1)
        frac = position - low

        def read(index: PoseLandmark, axis: str, lo: int = low, hi: int = high, f: float = frac):
            a = keypoints[lo].landmark(index)
            b = keypoints[hi].landmark(index)
            if min(a.visibility, b.visibility) < MIN_VISIBILITY:
                return None
            return getattr(a, axis) * (1 - f) + getattr(b, axis) * f

        origin: dict[str, float | None] = {}
        for axis in axes:
            left = read(PoseLandmark.LEFT_HIP, axis)
            right = read(PoseLandmark.RIGHT_HIP, axis)
            origin[axis] = None if left is None or right is None else (left + right) / 2

        left_shoulder = read(PoseLandmark.LEFT_SHOULDER, "x")
        right_shoulder = read(PoseLandmark.RIGHT_SHOULDER, "x")
        if left_shoulder is not None and right_shoulder is not None:
            width = abs(left_shoulder - right_shoulder)
            if width >= MIN_SHOULDER_WIDTH:
                widths.append(width)

        for i, index in enumerate(indices):
            for j, axis in enumerate(axes):
                value = read(index, axis)
                base = origin[axis]
                column = i * len(axes) + j
                columns[column].append(None if value is None or base is None else value - base)

    if not widths:
        return None
    # One ruler for the whole swing rather than one per frame. A per-frame width would renormalise
    # away the very torso motion this is trying to see, and is noisier besides.
    widths.sort()
    scale = widths[len(widths) // 2]

    filled = _interpolate_gaps([[None if v is None else v / scale for v in c] for c in columns])
    if filled is None:
        return None

    if mirror:
        filled = _mirror(filled, landmarks, axes)

    return [
        filled[i * len(axes) + j][t]
        for t in range(steps)
        for i in range(len(landmarks))
        for j in range(len(axes))
    ]


def _mirror(columns: list[list[float]], landmarks: list[str], axes: list[str]) -> list[list[float]]:
    """Swap every left/right pair and negate `x`."""
    partner = {}
    for i, name in enumerate(landmarks):
        if name.startswith("left_"):
            other = name.replace("left_", "right_", 1)
        else:
            other = name.replace("right_", "left_", 1)
        partner[i] = landmarks.index(other) if other in landmarks else i

    out = [list(c) for c in columns]
    for i in range(len(landmarks)):
        for j, axis in enumerate(axes):
            source = columns[partner[i] * len(axes) + j]
            target = i * len(axes) + j
            out[target] = [-v for v in source] if axis == "x" else list(source)
    return out
