"""Measuring, separated from judging. [M6.5 — data infrastructure]

Every function here answers *what is the number* and nothing else. None of them touch
`ranges.json`, none produce a `CheckpointScore`, and a `None` return means strictly **could not
measure** — unusable landmarks, a missing phase, a boundary that was estimated rather than
detected — never "no band exists".

## Why this module exists

`checkpoints/mechanics.py` used to do both jobs in one function: measure the quantity, resolve a
band, score against it, and return `None` if *any* of the three failed. That fused the two, and
the fusion had a consequence nobody set out to design — **a metric with no band could not be
measured at all**. Bands are derived from a population of measurements over the reference corpus
(`scripts/golfdb/derive_pose_metrics.py` -> `derive_reference.py`), so a new metric needed a band
to produce the measurements the band is derived from. That circle, not the difficulty of any
particular metric, is why the panel sat at three checkpoints.

Splitting them breaks it. `derive_pose_metrics.py` can now measure a brand-new quantity across all
461 cached face-on clips, `derive_reference.py` can cut a distribution from that, and only then —
if `scripts/golfdb/tune_spatial_metric.py` says the metric is signal rather than jitter — does
anyone write a band and a checkpoint.

## What is safe to measure face-on, and what is not

The instants are not equally trustworthy: impact carries a median error of 1 frame, the top 2, and
address 7 with 40% of clips over 10 (M4-REF). And the corpus recorded a sharper lesson —
`tune_arm_parallel.py` found that a rule reading *no pose signal at all* beat every pose rule at
locating mid-backswing. But that harness scored **frame indices**: what failed was detecting a new
instant in time, from a single-frame argmin or zero-crossing, where landmark jitter is
unattenuated.

Measuring a *spatial* quantity at an instant that is already validated, averaged over a window, is
the opposite regime — averaging N frames suppresses zero-mean jitter by sqrt(N). That is the
property `head_sway` and `finish_balance` already rely on, and it is the only regime this module
adds to. Every function here reads a window, never a single frame.

Two further constraints shape what is included:

- **`x`-over-`x` is aspect-immune.** A lateral distance divided by a shoulder width cancels the
  16:9 pixel-aspect assumption entirely (recorded for `head_sway_norm` in `ranges.json`). A `y`
  quantity against an `x` ruler does not, so vertical metrics and shoulder tilt are deliberately
  left out for now rather than quietly inheriting a source assumption.
- **Only well-conditioned landmarks.** Hips, shoulders and ears are the structures the bake-off
  found intact even at the finish (`M4_POSE_BAKEOFF.md`); wrists jitter ~6x more, and there is
  *zero* recorded reliability evidence for ankles or knees on this corpus. Nothing here reads them.

Pure functions, stdlib + contracts only (ADR-008).
"""

from __future__ import annotations

from collections.abc import Callable

from golf_coach.analysis.stats import percentile
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import PhaseSegment, SwingPhase

#: A per-frame point extractor over an inclusive frame span — `hip_center_points` and friends.
PointsFn = Callable[[list[FrameKeypoints], int, int], list[tuple[float, float]]]

#: A full pose measurement: smoothed frames + phases in, one number or "could not measure" out.
MeasureFn = Callable[[list[FrameKeypoints], list[PhaseSegment]], float | None]

# Landmarks dimmer than this are treated as unreliable (MediaPipe convention, matches
# phases.py / overlay.py).
MIN_VISIBILITY = 0.5

# Hips at the finish are the worst-conditioned landmarks we read: the golfer has rotated ~90°, so
# from a face-on camera the trail hip sits behind the lead hip and the torso, and the arms and club
# cross the body right there. MediaPipe's `visibility` is a learned logit rather than a calibrated
# probability, so the usual 0.5 gate happily passes confidently-wrong hip estimates — exactly the
# frames `finish_balance` is most sensitive to. Gate them harder than everything else.
MIN_HIP_VISIBILITY = 0.7

# A face-on shoulder width (normalized) below this is degenerate (golfer turned side-on or
# shoulders mis-detected) — we can't form a reliable scale, so the measurement bails.
MIN_SHOULDER_WIDTH = 0.02

# Fewest follow-through frames needed to judge how still the finish settles.
MIN_FINISH_FRAMES = 3

# The posture-sampling window (see `address_sample_bounds`): this fraction of the clip's own
# downswing duration, floored at a handful of frames. Half a downswing is ~4 frames on a real-time
# clip and ~15 in slow motion — long enough that averaging suppresses landmark jitter, short enough
# that it cannot reach back into the golfer walking up to the ball.
ADDRESS_SAMPLE_FRACTION = 0.5
ADDRESS_SAMPLE_MIN_FRAMES = 5

# Finish drift is summarized at this quantile rather than by `max()`. [metric definitions v2]
# `max()` is an extreme-value statistic: one bad frame sets the entire metric, and the frames it
# reads are the most occlusion-prone in the swing (see `MIN_HIP_VISIBILITY`). p90 still captures
# "how far does the body actually stagger" — it is a high quantile, not a central one — while
# needing the drift to persist across several frames before it counts.
FINISH_DRIFT_QUANTILE = 0.90


# --------------------------------------------------------------------------- geometry helpers


def phase_bounds(phases: list[PhaseSegment], phase: SwingPhase) -> tuple[int, int] | None:
    """Inclusive `(start_frame, end_frame)` for a phase, or None if it isn't present."""
    for segment in phases:
        if segment.phase is phase:
            return segment.start_frame, segment.end_frame
    return None


def midpoint_series(
    keypoints: list[FrameKeypoints],
    lo: int,
    hi: int,
    first: PoseLandmark,
    second: PoseLandmark,
    min_visibility: float = MIN_VISIBILITY,
) -> list[tuple[float, float]]:
    """Per-frame midpoint of two landmarks over the inclusive span, confident frames only.

    Both landmarks must clear `min_visibility` for the frame to count — a midpoint built from one
    good and one guessed landmark is worse than no sample at all, because it looks plausible.
    """
    points: list[tuple[float, float]] = []
    for kp in keypoints[lo : hi + 1]:
        a = kp.landmark(first)
        b = kp.landmark(second)
        if a.visibility >= min_visibility and b.visibility >= min_visibility:
            points.append(((a.x + b.x) / 2, (a.y + b.y) / 2))
    return points


def mean_of(points: list[tuple[float, float]]) -> tuple[float, float] | None:
    """Mean `(x, y)` of a point series, or None if it is empty."""
    if not points:
        return None
    return (
        sum(x for x, _ in points) / len(points),
        sum(y for _, y in points) / len(points),
    )


def shoulder_width(keypoints: list[FrameKeypoints], lo: int, hi: int) -> float | None:
    """Mean face-on shoulder width (normalized) over a span — the scale-invariance ruler.

    Returns None if too few confident frames or the width is degenerate (side-on / mis-detect).
    """
    widths = [
        abs(kp.landmark(PoseLandmark.LEFT_SHOULDER).x - kp.landmark(PoseLandmark.RIGHT_SHOULDER).x)
        for kp in keypoints[lo : hi + 1]
        if kp.landmark(PoseLandmark.LEFT_SHOULDER).visibility >= MIN_VISIBILITY
        and kp.landmark(PoseLandmark.RIGHT_SHOULDER).visibility >= MIN_VISIBILITY
    ]
    if not widths:
        return None
    width = sum(widths) / len(widths)
    return width if width >= MIN_SHOULDER_WIDTH else None


def address_sample_bounds(phases: list[PhaseSegment]) -> tuple[int, int] | None:
    """A short window **ending at** the address boundary — where the golfer is actually set up.

    Not the same thing as the ADDRESS phase. That segment runs `[0, motion_start]`, and frame 0 is
    not address, it is wherever the clip happens to begin: across the GolfDB face-on corpus the
    golfer's head travels a median of 0.12 shoulder-widths inside that window and 0.61 at the p90,
    because they are still walking in and settling over the ball. Averaging posture across it
    measures the approach as much as the setup — and `head_sway`'s entire pass band is 0.42
    shoulder-widths, so at the p90 the pre-roll alone outweighs the thing being scored.

    Anchoring a few frames to the *end* of the segment fixes both failure modes at once (ADR-013):
    it excludes the pre-roll, and it makes posture largely indifferent to the address boundary
    being wrong, which matters because that boundary carries a median error of 7 frames. A window
    that moves with the boundary still lands on genuine setup; a window that *starts at frame 0*
    inherits everything before it.

    Length scales with the clip's own downswing duration for the same reason the quiet run does —
    a fixed count would mean 4x different things across a corpus that is ~47% slow-motion — with a
    floor of `ADDRESS_SAMPLE_MIN_FRAMES` so there is always enough to average. Returns None only
    when the phases are unusable.
    """
    address = phase_bounds(phases, SwingPhase.ADDRESS)
    downswing = phase_bounds(phases, SwingPhase.DOWNSWING)
    if address is None or downswing is None:
        return None

    span = max(downswing[1] - downswing[0], 1)
    length = max(ADDRESS_SAMPLE_MIN_FRAMES, round(ADDRESS_SAMPLE_FRACTION * span))

    hi = address[1]
    lo = max(address[0], hi - length)
    # A clip whose takeaway starts almost immediately (or where the boundary was estimated at 0)
    # leaves nothing behind it — widen forward instead, since the frames just after a mis-placed
    # boundary are still far closer to setup than the start of the clip is.
    if hi - lo < ADDRESS_SAMPLE_MIN_FRAMES:
        hi = min(downswing[0], lo + ADDRESS_SAMPLE_MIN_FRAMES)
    return (lo, max(hi, lo))


def hip_center_points(
    keypoints: list[FrameKeypoints], lo: int, hi: int
) -> list[tuple[float, float]]:
    """Per-frame hip-center `(x, y)` over the inclusive span, confident frames only."""
    return midpoint_series(
        keypoints,
        lo,
        hi,
        PoseLandmark.LEFT_HIP,
        PoseLandmark.RIGHT_HIP,
        min_visibility=MIN_HIP_VISIBILITY,
    )


def head_center_points(
    keypoints: list[FrameKeypoints], lo: int, hi: int
) -> list[tuple[float, float]]:
    """Per-frame head-center `(x, y)`: the midpoint of the two ears. [metric definitions v2]

    The ear midpoint approximates the head's **axis of rotation**; the `NOSE` this replaced does
    not. Through a swing the head turns, and a nose on a turning head sweeps several centimetres
    laterally without the head itself going anywhere. That is a *definitional* error, not a noise
    one — no smoothing or better pose model removes it, and on our own clips it was large enough to
    read a stable head as 1.18 shoulder-widths of sway. The ears sit either side of the rotation
    axis, so the turn largely cancels in their midpoint and what survives is true lateral travel.
    """
    return midpoint_series(keypoints, lo, hi, PoseLandmark.LEFT_EAR, PoseLandmark.RIGHT_EAR)


def tempo_timings(phases: list[PhaseSegment]) -> tuple[float, float] | None:
    """Extract (backswing_ms, downswing_ms) from the phase boundaries, or None if unavailable.

    Uses motion start (`BACKSWING.start_ms`), the top of the backswing (center of the
    `TRANSITION` window), and impact (`IMPACT.start_ms`).

    **Address is the dominant error term in this ratio.** It sits directly in the numerator, and it
    is the least accurate instant we have — median 7 frames against 2 for the top and 1 for impact.
    Worth keeping in proportion: a rule that ignores the pose entirely and assumes the tour-median
    tempo locates address to within 11 frames, so the wrist signal contributes real but modest
    information here (M4-REF Phase B6).

    Returns None when the backswing boundary was **estimated rather than detected**. That estimate
    is derived from an assumed tempo ratio, so reporting it would echo that assumption back as an
    observation — a guess dressed as a measurement. Dropping it is ADR-010 §2 and ADR-013; it costs
    ~14% of clips their tempo reading.
    """
    by_phase = {segment.phase: segment for segment in phases}
    backswing = by_phase.get(SwingPhase.BACKSWING)
    transition = by_phase.get(SwingPhase.TRANSITION)
    impact = by_phase.get(SwingPhase.IMPACT)
    if backswing is None or transition is None or impact is None:
        return None
    if not backswing.detected:
        return None

    motion_start_ms = backswing.start_ms
    top_ms = (transition.start_ms + transition.end_ms) / 2
    impact_ms = impact.start_ms

    backswing_ms = top_ms - motion_start_ms
    downswing_ms = impact_ms - top_ms
    if backswing_ms <= 0 or downswing_ms <= 0:
        return None
    return backswing_ms, downswing_ms


def _lateral_travel(
    keypoints: list[FrameKeypoints],
    phases: list[PhaseSegment],
    points_of: PointsFn,
    to_phase: SwingPhase,
) -> float | None:
    """Shared body of the address-to-instant lateral metrics: |Δx| in shoulder widths.

    Both endpoints are means over a window and the ruler comes from the address window, which is
    what makes this family robust and what keeps it aspect-immune (an `x` distance over an `x`
    scale cancels the 16:9 assumption outright).
    """
    setup = address_sample_bounds(phases)
    target = phase_bounds(phases, to_phase)
    if setup is None or target is None:
        return None

    start = mean_of(points_of(keypoints, setup[0], setup[1]))
    end = mean_of(points_of(keypoints, target[0], target[1]))
    width = shoulder_width(keypoints, setup[0], setup[1])
    if start is None or end is None or width is None:
        return None
    return abs(end[0] - start[0]) / width


# --------------------------------------------------------------------------- the measurements


def measure_tempo_ratio(phases: list[PhaseSegment]) -> float | None:
    """Backswing:downswing time ratio. Frame rate cancels, so slo-mo and real-time agree."""
    timings = tempo_timings(phases)
    if timings is None:
        return None
    backswing_ms, downswing_ms = timings
    return backswing_ms / downswing_ms


def measure_head_sway(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> float | None:
    """Lateral head travel, address window -> impact window, in shoulder widths. [v3]"""
    return _lateral_travel(keypoints, phases, head_center_points, SwingPhase.IMPACT)


def measure_hip_sway(keypoints: list[FrameKeypoints], phases: list[PhaseSegment]) -> float | None:
    """Lateral hip travel, address window -> impact window, in shoulder widths.

    The structural twin of `measure_head_sway`, and it answers a different coaching question: a
    head that stays put over hips that slide is a lateral slide, while head and hips moving
    together is a body that swayed. Neither is visible from head sway alone.
    """
    return _lateral_travel(keypoints, phases, hip_center_points, SwingPhase.IMPACT)


def measure_hip_shift_at_top(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> float | None:
    """Lateral hip travel, address window -> transition (top) window, in shoulder widths.

    Magnitude only. Direction would separate a sway off the ball from a reverse pivot, but the sign
    is camera-relative and handedness is not recorded anywhere in this pipeline — see
    `measure_head_hip_offset_impact` for the same problem stated in full. The magnitude is
    interpretable without it: how far the lower body slid going back.

    This is the cleanest test of whether the arm-parallel failure generalizes. It reads a
    *detected* instant (the top, median 2 frames of error) but measures a spatial quantity averaged
    over the transition window, so it inherits none of the single-frame fragility that sank the
    frame-index rules. `tune_spatial_metric.py` is what settles it.
    """
    return _lateral_travel(keypoints, phases, hip_center_points, SwingPhase.TRANSITION)


def measure_head_hip_offset_impact(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> float | None:
    """Signed head-center minus hip-center offset over the impact window, in shoulder widths.

    "Staying behind the ball" — and the only metric here that is signed, because the magnitude
    alone says nothing: a head ahead of the hips at impact and a head behind them are opposite
    faults with the same absolute value.

    **The sign is camera-relative, and that is a known limitation, not an oversight.** Positive
    means the head sits to the right of the hip center *in the image*. Which side that is in swing
    terms depends on handedness, and nothing in this pipeline records or infers it. Two
    consequences to resolve before any band is cut from this: a distribution over a
    mixed-handedness corpus like GolfDB will be **bimodal**, and a band cut from it would be
    meaningless. `tune_spatial_metric.py` reports the population spread and flags bimodality, so it
    shows up there rather than being discovered after a band shipped.

    Measured over the corpus it is *not* bimodal — 458 face-on clips run p10 -0.88 to p90 -0.33,
    consistently head-behind-hips — so a band is derivable. But that is an empirical fact about
    GolfDB's handedness mix, not a guarantee: a left-handed golfer would land on the opposite sign
    and read as a gross fault. Resolve handedness before this becomes a checkpoint.

    **Do not paste `derive_reference.py`'s recommended band for this metric.** That recommendation
    keys on the `_norm` suffix to suggest a one-sided `[0, p90]` band, which assumes non-negative
    values — for this metric it prints `low=0.00 high=-0.33`, an empty interval. The suffix is
    right (it *is* shoulder-width normalized); the heuristic behind the recommendation is what does
    not generalize to a signed quantity.
    """
    setup = address_sample_bounds(phases)
    impact = phase_bounds(phases, SwingPhase.IMPACT)
    if setup is None or impact is None:
        return None

    head = mean_of(head_center_points(keypoints, impact[0], impact[1]))
    hips = mean_of(hip_center_points(keypoints, impact[0], impact[1]))
    width = shoulder_width(keypoints, setup[0], setup[1])
    if head is None or hips is None or width is None:
        return None
    return (head[0] - hips[0]) / width


def measure_finish_balance(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> float | None:
    """Hip-center drift from its own mean through the follow-through, in shoulder widths. [v2]"""
    follow_through = phase_bounds(phases, SwingPhase.FOLLOW_THROUGH)
    setup = address_sample_bounds(phases)
    if follow_through is None or setup is None:
        return None

    points = hip_center_points(keypoints, follow_through[0], follow_through[1])
    width = shoulder_width(keypoints, setup[0], setup[1])
    if len(points) < MIN_FINISH_FRAMES or width is None:
        return None

    center = mean_of(points)
    if center is None:
        return None
    mean_x, mean_y = center
    drifts = [((x - mean_x) ** 2 + (y - mean_y) ** 2) ** 0.5 for x, y in points]
    return percentile(drifts, FINISH_DRIFT_QUANTILE) / width


#: Name -> measure function, for everything derived from face-on pose over a full swing.
#:
#: The registry is the point: `derive_pose_metrics.py` iterates it instead of hardcoding metric
#: names (it hardcoded two, in three places), and `tune_spatial_metric.py` scores whatever is in
#: it. Adding a candidate metric is one line here, and every offline tool picks it up.
#:
#: Names ending `_norm` are shoulder-width-normalized, which `derive_reference.py` keys on to
#: recommend a one-sided band. `tempo_ratio` is deliberately outside that convention.
POSE_MEASUREMENTS: dict[str, MeasureFn] = {
    "tempo_ratio": lambda frames, phases: measure_tempo_ratio(phases),
    "head_sway_norm": measure_head_sway,
    "finish_balance_norm": measure_finish_balance,
    "hip_sway_norm": measure_hip_sway,
    "hip_shift_at_top_norm": measure_hip_shift_at_top,
    "head_hip_offset_impact_norm": measure_head_hip_offset_impact,
}

#: How each measurement is taken, carried onto `Measurement.detail` so a stored number can be
#: re-derived or re-normalized later without reading this module.
POSE_MEASUREMENT_DETAIL = {
    "tempo_ratio": "backswing:downswing time, from phase instants; frame rate cancels",
    "head_sway_norm": "|dx| of ear midpoint, address window -> impact window",
    "finish_balance_norm": "p90 hip-center drift through follow-through",
    "hip_sway_norm": "|dx| of hip midpoint, address window -> impact window",
    "hip_shift_at_top_norm": "|dx| of hip midpoint, address window -> transition window",
    "head_hip_offset_impact_norm": (
        "signed (head - hips) dx over the impact window; + is image-right, "
        "handedness not resolved"
    ),
}

#: Unit per measurement, for `Measurement.unit`.
POSE_MEASUREMENT_UNIT = {
    "tempo_ratio": "ratio",
    "head_sway_norm": "shoulder_widths",
    "finish_balance_norm": "shoulder_widths",
    "hip_sway_norm": "shoulder_widths",
    "hip_shift_at_top_norm": "shoulder_widths",
    "head_hip_offset_impact_norm": "shoulder_widths",
}
