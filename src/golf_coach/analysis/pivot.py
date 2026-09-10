"""The swing as three points moving — the pivot paths the rotation checks read. [M17]

`trajectory.py` produces the whole body as one long vector for a fitted model. This produces the
three points the turn happens *about* — the shoulder-line midpoint, the hip-line midpoint and the
hands — plus the orientation of the two lines through the first two. Same resampling onto event
time, same shoulder-width ruler, a much smaller shape. Still measuring and not judging: nothing
here reads a band, and `contracts/pivots.py` says why these numbers ship as `Measurement`s rather
than as `CheckpointScore`s.

**This module is the single implementation**, the way `trajectory.py` is, and it has **one
producer with one signature serving both cameras**. `analysis/engine.py` segments the face-on clip
only — the down-the-line view never holds a `list[PhaseSegment]`, it gets a `SwingAnchors` and
nothing more — so a phases-shaped producer would have needed a second copy for the second camera,
and two copies of a resampler are two things that drift. Anchors are what both views have, so
anchors are what this takes. There is deliberately no `from_phases` variant: the face-on caller
converts with `trajectory.anchors_from_phases`.

The output is `contracts.PivotObservation` and not a private type here, which is the whole seam:
when the fiducial markers land, a calibrated 3-D source becomes a second producer of the same
shape and the rule checks that read it do not change (ADR-029 Decision 3).

**Handedness is not an argument, and that is a property to preserve rather than an omission.**
Every check below is unsigned — an excursion magnitude, a roughness, a largest-reversal magnitude
— so the mirroring `engine.analyze_swing`'s docstring warns about ("a face-on camera sees a
left-handed swing mirrored") cancels before it reaches a number. Add a *signed* pivot metric later
and that stops being true: `handedness` then has to be threaded to **both** engine call sites, and
the face-on one is the one that will be forgotten, because the down-the-line helper is the one
that looks unusual. `tests/analysis/test_pivot.py` pins the unsigned property by scoring a
mirrored swing.

Stdlib + `contracts/` only (ADR-008).
"""

from __future__ import annotations

import math
from collections.abc import Callable

from golf_coach.analysis.measure import (
    MIN_DIRECTION_LENGTH,
    MIN_SHOULDER_WIDTH,
    MIN_VISIBILITY,
    MeasureOutcome,
)
from golf_coach.analysis.phases import LEAD_WRIST, TRAIL_WRIST

# `_interpolate_gaps` is private to `trajectory.py` by name and shared by intent: it is the rule for
# how much of a landmark's timeline may be bridged before the bridge is an invention, and this is
# the second resampler in the package rather than a module outside it. A copy here would be a second
# `MAX_MISSING` to keep in step.
from golf_coach.analysis.trajectory import _interpolate_gaps, sample_positions
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.pivots import (
    BACKSWING_SPAN,
    DOWNSWING_SPAN,
    PIVOT_SAMPLES,
    FrameOfReference,
    PivotObservation,
)
from golf_coach.contracts.unscored import UnscoredReason

#: The landmark pairs the three pivot points are the midpoints of, in `PivotObservation`'s order.
#: The hands take *both* wrists rather than a `wrist` argument: both hands are on the club, and the
#: midpoint of the pair is the same point whichever one is the lead, so this is the one place in
#: `analysis/` where the lead/trail distinction genuinely does not matter.
_SHOULDER_PAIR = (PoseLandmark.LEFT_SHOULDER, PoseLandmark.RIGHT_SHOULDER)
_HIP_PAIR = (PoseLandmark.LEFT_HIP, PoseLandmark.RIGHT_HIP)
_HAND_PAIR = (LEAD_WRIST, TRAIL_WRIST)


def _read(
    frames: list[FrameKeypoints], position: float, landmark: PoseLandmark
) -> tuple[float, float] | None:
    """One landmark at a fractional frame position, or None if either bracketing frame is unsure.

    `build_trajectory`'s inner `read`, named and returning both axes at once.

    `measure.midpoint_series` and `measure.direction_series` are the per-frame analogues of what
    this feeds, and they are deliberately *not* reused: both **drop** an unconfident frame instead
    of reporting it, which throws away the frame index, and an index is exactly what a resampled
    path needs. What is reused is the pair of gates they apply — `MIN_VISIBILITY` here and
    `MIN_DIRECTION_LENGTH` in `_unit` — imported rather than restated, so a retune moves both.

    Both bracketing frames must clear the gate: interpolating from one good and one guessed
    landmark produces a plausible number, which is worse than no sample at all.
    """
    low = int(position)
    high = min(low + 1, len(frames) - 1)
    frac = position - low
    a = frames[low].landmark(landmark)
    b = frames[high].landmark(landmark)
    if min(a.visibility, b.visibility) < MIN_VISIBILITY:
        return None
    return (a.x * (1 - frac) + b.x * frac, a.y * (1 - frac) + b.y * frac)


def _pair(
    frames: list[FrameKeypoints], position: float, pair: tuple[PoseLandmark, PoseLandmark]
) -> tuple[tuple[float, float] | None, tuple[float, float] | None]:
    """Both landmarks of a pair at one sampled position — the midpoint *and* the line need both."""
    return (_read(frames, position, pair[0]), _read(frames, position, pair[1]))


def _midpoint(
    first: tuple[float, float] | None, second: tuple[float, float] | None
) -> tuple[float, float] | None:
    """Midpoint of two read landmarks, or None if either was unreadable."""
    if first is None or second is None:
        return None
    return ((first[0] + second[0]) / 2, (first[1] + second[1]) / 2)


def _unit(
    first: tuple[float, float] | None, second: tuple[float, float] | None
) -> tuple[float, float] | None:
    """Unit vector `first -> second`, or None where the two have collapsed.

    The `None` is the honest reading and not a gap to be filled: the projected line collapses at
    one end of every swing in both views (face-on at the top, down-the-line at address), and a
    direction taken across a collapsed segment is jitter divided by roughly nothing —
    `measure.direction_series`' rule, applied to a resampled instant.

    The vector points left landmark to right landmark, but the **line is undirected**: as the
    shoulders cross the camera axis through a full turn the vector flips sign. A consumer measuring
    how far the line turned between two samples must therefore fold the angle onto +/-90 degrees
    rather than reading it as +/-180 — a flip is the same line, not a half revolution.
    """
    if first is None or second is None:
        return None
    dx, dy = second[0] - first[0], second[1] - first[1]
    length = math.hypot(dx, dy)
    if length < MIN_DIRECTION_LENGTH:
        return None
    return (dx / length, dy / length)


def pivot_observations(
    frames: list[FrameKeypoints],
    anchors: tuple[float, float, float],
    *,
    frame_of_reference: FrameOfReference,
) -> list[PivotObservation] | None:
    """One swing as `PIVOT_SAMPLES` pivot observations in event time, or None if unmeasurable.

    `frames` are expected **smoothed** — `analyze_swing` smooths once and hands the result to
    everything, and `_dtl_placements` smooths the second camera's frames at its own call site.
    `anchors` are `(address, top, impact)` as fractional frame indices, strictly increasing.

    Three normalisations, and each is load-bearing:

    - **Event time.** `sample_positions` spaces the samples evenly between the anchors rather than
      evenly in clock time, so a slow-motion clip, a real-time clip and the two cameras all land on
      one index space. `PIVOT_SAMPLES` is odd precisely so the top of the backswing lands *on* a
      sample; `contracts/pivots.py` carries that contract and the two spans that read it.
    - **Origin: the hip centre at address**, one point for the whole swing. Not the hip centre
      per sample, which is what "hip-relative" means in `build_trajectory` and would be wrong here:
      it would put the hip at the origin in every sample and zero out the very travel two of the
      five checks measure. A fixed origin still buys what the per-sample one buys — a golfer
      standing anywhere in frame produces the same numbers — while leaving the motion in.
    - **Scale: the median per-sample shoulder width**, `build_trajectory`'s ruler rather than
      `measure.shoulder_width`'s mean. That function is written for the address window, where the
      shoulder line is square to a face-on camera; averaged across address-to-impact the same
      quantity is dragged down by the collapse at the top that `_unit` returns `None` for, and a
      ruler that shrinks with the turn inflates every excursion measured against it. The median
      still is not immune to how far the golfer turns, which is one of the reasons every pivot
      measurement ships with `PivotMeasurementSpec.interim_reason` attached.

    Returns None rather than a partial answer when the swing cannot be read at all: fewer than two
    frames, anchors that do not increase (a collapsed detection window — the down-the-line caller
    builds its three from a `SwingAnchors` that nothing has checked), anchors reaching outside the
    clip, no usable shoulder width anywhere, or a **shoulder or hip** midpoint missing more of its
    timeline than `trajectory.MAX_MISSING` allows. A per-sample gap inside those limits is bridged;
    a per-sample *orientation* is never bridged, and stays `None` for the checks to skip.

    **The hands are gated separately and cannot refuse the swing.** They are the worst-tracked of
    the three points and the only one nothing measures, so a shared gate let them veto the five
    metrics that ignore them — `PivotObservation.hands` carries what that cost. `hands` is `None` on
    every observation of a swing whose wrists were too sparse to bridge, and the shoulder and hip
    midpoints are still exactly what they would have been.
    """
    # The pose producer cannot declare the one frame of reference in which a rotation number could
    # earn a band. Raised rather than returned, per `contracts.pivots.spec_for`'s reasoning: the
    # caller passes a literal, so a wrong one is a wiring bug and not a property of the footage.
    if frame_of_reference is FrameOfReference.CALIBRATED_3D:
        raise ValueError(
            "pivot_observations reads pose in an uncalibrated image plane; it cannot produce "
            f"{FrameOfReference.CALIBRATED_3D} observations"
        )

    if len(frames) < 2:
        return None
    if not (anchors[0] < anchors[1] < anchors[2]):
        return None

    positions = sample_positions(anchors, PIVOT_SAMPLES)
    if positions[0] < 0 or positions[-1] > len(frames) - 1:
        return None

    points: dict[str, list[tuple[float, float] | None]] = {"shoulder": [], "hip": [], "hands": []}
    lines: list[tuple[tuple[float, float] | None, tuple[float, float] | None]] = []
    widths: list[float] = []

    for position in positions:
        shoulders = _pair(frames, position, _SHOULDER_PAIR)
        hips = _pair(frames, position, _HIP_PAIR)
        points["shoulder"].append(_midpoint(*shoulders))
        points["hip"].append(_midpoint(*hips))
        points["hands"].append(_midpoint(*_pair(frames, position, _HAND_PAIR)))
        lines.append((_unit(*shoulders), _unit(*hips)))

        left_shoulder, right_shoulder = shoulders
        if left_shoulder is not None and right_shoulder is not None:
            # Horizontal span and not the segment length, matching both rulers this one stands
            # beside (`measure.shoulder_width`, `build_trajectory`): the excursions it normalises
            # are horizontal too, so the frame's pixel aspect cancels in the ratio.
            width = abs(left_shoulder[0] - right_shoulder[0])
            if width >= MIN_SHOULDER_WIDTH:
                widths.append(width)

    if not widths:
        return None
    widths.sort()
    scale = widths[len(widths) // 2]

    def columns(*names: str) -> list[list[float | None]]:
        return [
            [None if point is None else point[axis] for point in points[name]]
            for name in names
            for axis in (0, 1)
        ]

    # **Two gates, because the three points do not stand or fall together.** The shoulder and hip
    # midpoints are what every check reads, so a swing that cannot bridge them is not an observation
    # and refuses here. The hands are drawn and measured by nothing, and they are also the worst
    # tracked of the three: M14 P3 put face-on hand tracking at 0.63-0.68 over a whole clip against
    # 1.00 over the address window, and on the corpus 44-61% of the resampled samples had no
    # readable wrist pair while the shoulder and hip midpoints read on every single one. Under one
    # shared gate the hands took the swing down with them — a landmark nothing measures holding a
    # veto over the five metrics that ignore it, which is how M17 P5's first corpus run recorded
    # zero face-on rotation numbers on all fifteen swings.
    filled = _interpolate_gaps(columns("shoulder", "hip"))
    if filled is None:
        return None
    shoulder_x, shoulder_y, hip_x, hip_y = filled

    hands = _interpolate_gaps(columns("hands"))

    origin_x, origin_y = hip_x[0], hip_y[0]

    def placed(xs: list[float], ys: list[float], i: int) -> tuple[float, float]:
        return ((xs[i] - origin_x) / scale, (ys[i] - origin_y) / scale)

    return [
        PivotObservation(
            shoulder=placed(shoulder_x, shoulder_y, i),
            hip=placed(hip_x, hip_y, i),
            hands=None if hands is None else placed(hands[0], hands[1], i),
            shoulder_line=lines[i][0],
            hip_line=lines[i][1],
            frame_of_reference=frame_of_reference,
        )
        for i in range(PIVOT_SAMPLES)
    ]


# ------------------------------------------------------------------------------ the rule checks


#: One pivot measurement: a resampled path in, one number or a reason out.
#:
#: **Not `measure.MeasureFn`**, which takes `list[FrameKeypoints]` and would put the producer back
#: inside every check — five checks each resampling the swing from scratch, and five places for a
#: calibrated source to have to arrive. The engine calls `pivot_observations` once per view and
#: hands the same list to all five (ADR-029 Decision 3 and its addendum §1).
PivotCheckFn = Callable[[list[PivotObservation]], MeasureOutcome]

#: Fewest observations a path statistic will read. Two for an excursion (one point is not a path);
#: three for the roughness below, which is a second difference and needs a middle sample to bend
#: about.
MIN_PATH_SAMPLES = 2
MIN_JITTER_SAMPLES = 3

#: How much of a reversal window may carry no shoulder-line orientation before the check refuses.
#:
#: Its own constant at `trajectory.MAX_MISSING`'s value rather than an import of it, because the
#: two dials govern different things and only one of them bridges: `MAX_MISSING` is how much of a
#: *position* timeline may be interpolated across, while an orientation is never interpolated at
#: all (`_unit` returns None and the check skips the pair). This is a coverage floor — how much of
#: the window the check must actually have read before it is entitled to call a turn monotone.
#: Neither number is set by evidence from this corpus; they match so there is one habit rather than
#: two.
MAX_UNORIENTED = 0.40


def _one_instrument(observations: list[PivotObservation]) -> None:
    """Raise unless every observation is in one frame of reference.

    Two frames of reference are two instruments, and every check below compares observations to
    each other (`contracts.pivots.PivotObservation.frame_of_reference`). Raised rather than
    refused, per this module's existing rule: a mixed list cannot come out of `pivot_observations`,
    so it is a caller assembling one by hand — a wiring bug, not a property of the footage.
    """
    spaces = {o.frame_of_reference for o in observations}
    if len(spaces) > 1:
        raise ValueError(f"pivot observations mix frames of reference: {sorted(spaces)}")


def _axis_drift(
    observations: list[PivotObservation],
    point: Callable[[PivotObservation], tuple[float, float]],
    name: str,
) -> MeasureOutcome:
    """Peak lateral excursion of one centre from its own address `x`, in shoulder widths.

    `x` over `x`, so the frame's pixel aspect cancels — which is what makes the two drift metrics
    the least calibration-sensitive numbers in the milestone and the reason they lead the registry.

    Measured from `observations[0]`'s own `x` rather than from zero. The pose producer happens to
    put the address *hip* at the origin, so the hip's address `x` is 0 today; the shoulder's is
    not, and a calibrated producer need not place either there.
    """
    _one_instrument(observations)
    if len(observations) < MIN_PATH_SAMPLES:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.TOO_FEW_FRAMES,
            f"{len(observations)} pivot samples; the {name} path needs {MIN_PATH_SAMPLES}",
        )
    address_x = point(observations[0])[0]
    return MeasureOutcome.measured(max(abs(point(o)[0] - address_x) for o in observations))


def hip_axis_drift(observations: list[PivotObservation]) -> MeasureOutcome:
    """How far the hip centre slid off the axis it started on. The best-tracked of the three."""
    return _axis_drift(observations, lambda o: o.hip, "hip centre")


def shoulder_axis_drift(observations: list[PivotObservation]) -> MeasureOutcome:
    """The same excursion for the shoulder centre."""
    return _axis_drift(observations, lambda o: o.shoulder, "shoulder centre")


def hip_path_jitter(observations: list[PivotObservation]) -> MeasureOutcome:
    """Sample-to-sample roughness of the hip centre's path, in shoulder widths.

    The **second** difference — `|p[i-1] - 2p[i] + p[i+1]|` — and not the first. A first difference
    is speed, and a hip that travels smoothly through the swing has plenty of it; what this asks is
    how much the path changes direction or speed between one sample and the next, so a straight
    steady slide reads 0 no matter how far it goes. That is also what keeps this independent of
    `hip_axis_drift`: the two would otherwise measure one thing twice.

    Averaged rather than maximised, for `measure.FINISH_DRIFT_QUANTILE`'s reason: `max()` is an
    extreme-value statistic and one bad sample would become the whole number, on a path whose
    samples are interpolated from an occlusion-prone landmark.

    **Roughness in event time, and it has a floor at the top.** The samples are evenly spaced
    between the anchors and not in clock time, so the backswing and the downswing are read at
    different frames per sample — which means a hip sliding at one constant speed *in frames*
    changes speed in samples exactly at the top, and reads a small non-zero bend there. Measured on
    the fixture it is around 3e-4 against a real shake's ~3e-2, so it is a floor rather than a
    signal; it is not subtracted out and the pair spanning the top is not skipped, because the
    transition is where a genuine hip jerk would live and dropping it to buy a cleaner zero would
    drop the most interesting sample in the swing. It also means the number is tempo-sensitive in a
    way none of the other four are, which matters the day a band is cut from it.

    Only the hip. The shoulders collapse at one end of every swing in both views and the hands are
    tracked at 0.63-0.68 face-on (M14 P3), so a roughness on either would report the tracker's
    noise as the golfer's (`contracts.pivots.PivotObservation.hands`).
    """
    _one_instrument(observations)
    if len(observations) < MIN_JITTER_SAMPLES:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.TOO_FEW_FRAMES,
            f"{len(observations)} pivot samples; roughness needs {MIN_JITTER_SAMPLES}",
        )
    bends = [
        math.hypot(
            first.hip[0] - 2 * middle.hip[0] + last.hip[0],
            first.hip[1] - 2 * middle.hip[1] + last.hip[1],
        )
        for first, middle, last in zip(
            observations, observations[1:], observations[2:], strict=False
        )
    ]
    return MeasureOutcome.measured(sum(bends) / len(bends))


def _line_delta_deg(
    first: tuple[float, float] | None, second: tuple[float, float] | None
) -> float | None:
    """Signed turn from one shoulder line to the next in degrees, folded onto +/-90, or None.

    Folded because the stored vector is **directed** and the line it describes is not: `_unit`
    points left landmark to right landmark, and that vector flips sign as the shoulders cross the
    camera axis. Read as +/-180 a flip would be a half revolution between two adjacent samples —
    the largest reversal imaginable, produced by the fullest turns. Folded, it is what it actually
    is: the same line, and a small move.
    """
    if first is None or second is None:
        return None
    cross = first[0] * second[1] - first[1] * second[0]
    dot = first[0] * second[0] + first[1] * second[1]
    return (math.degrees(math.atan2(cross, dot)) + 90.0) % 180.0 - 90.0


def _shoulder_reversal(
    observations: list[PivotObservation], span: range, window: str
) -> MeasureOutcome:
    """Largest against-the-turn move of the shoulder line inside one half of the swing.

    Zero is a monotone turn. The direction the turn is going is taken from the **net** move across
    the window rather than assumed, which is what lets one implementation serve a backswing and a
    downswing that turn opposite ways — and, with the folding above, a left-handed golfer whose
    every sign is mirrored. A window with no net turn at all takes the positive direction, so a
    line that went out and came back still reports the larger of the two halves rather than 0.

    Only *adjacent* readable samples are compared. Skipping a `None` and pairing across the gap
    would charge one interval with the whole turn that happened during the collapse, and the
    collapse sits at the top face-on and at address down-the-line — so the check would fire hardest
    on the fullest turns, which is exactly the failure mode ADR-029's addendum §4 raised against
    reading degrees off a projected line in the first place.
    """
    _one_instrument(observations)
    if len(observations) < PIVOT_SAMPLES:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.TOO_FEW_FRAMES,
            f"{len(observations)} pivot samples; the {window} span indexes {PIVOT_SAMPLES}",
        )

    unoriented = sum(1 for i in span if observations[i].shoulder_line is None)
    steps = [
        _line_delta_deg(observations[i].shoulder_line, observations[i + 1].shoulder_line)
        for i in span[:-1]
    ]
    deltas = [delta for delta in steps if delta is not None]
    if not deltas or unoriented / len(span) > MAX_UNORIENTED:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.LANDMARKS_UNCONFIDENT,
            f"the shoulder line collapsed in {unoriented} of {len(span)} {window} samples",
        )

    direction = 1.0 if sum(deltas) >= 0 else -1.0
    return MeasureOutcome.measured(max(0.0, max(-direction * delta for delta in deltas)))


def shoulder_reversal_backswing(observations: list[PivotObservation]) -> MeasureOutcome:
    """Address through the top, inclusive of both."""
    return _shoulder_reversal(observations, BACKSWING_SPAN, "backswing")


def shoulder_reversal_downswing(observations: list[PivotObservation]) -> MeasureOutcome:
    """The top through impact. The top belongs to both spans — a reversal is a move *between*
    samples, so the pair spanning it is the backswing's last and the downswing's first."""
    return _shoulder_reversal(observations, DOWNSWING_SPAN, "downswing")


#: Check name -> implementation, keyed by `PivotMeasurementSpec.check` and **never** by metric name.
#:
#: Five checks, ten metrics: a face-on spec and its `_dtl` partner name the same check. The view
#: decides the measurement's name and the check does not know which camera it is reading, which is
#: what lets one implementation serve two views without string surgery on the `_dtl` suffix — and
#: what a calibrated third producer will inherit for free.
#:
#: `tests/analysis/test_pivot.py` pins these keys against the registry's `check` fields, in both
#: directions: a spec naming a check that does not exist and a check nothing names are the same
#: kind of silent gap.
PIVOT_CHECKS: dict[str, PivotCheckFn] = {
    "hip_axis_drift": hip_axis_drift,
    "shoulder_axis_drift": shoulder_axis_drift,
    "hip_path_jitter": hip_path_jitter,
    "shoulder_reversal_backswing": shoulder_reversal_backswing,
    "shoulder_reversal_downswing": shoulder_reversal_downswing,
}
