"""Measuring, separated from judging. [M6.5 — data infrastructure]

Every function here answers *what is the number* and nothing else. None of them touch
`ranges.json`, none produce a `CheckpointScore`, and a `MeasureOutcome` with no value means
strictly **could not measure** — unusable landmarks, a missing phase, a boundary that was
estimated rather than detected — never "no band exists".

Which of those it was is carried out rather than discarded: `MeasureOutcome.reason` is drawn from
`contracts.unscored.MEASUREMENT_REASONS`, the subset of the vocabulary that describes a
measurement failing. `NO_BAND` and `NO_HANDEDNESS` are in that vocabulary and are *not* in the
subset, which is this module's split with `checkpoints/mechanics.py` made checkable.

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
from typing import NamedTuple

from golf_coach.analysis.stats import percentile
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import PhaseSegment, SwingPhase
from golf_coach.contracts.unscored import UnscoredReason

#: A per-frame point extractor over an inclusive frame span — `hip_center_points` and friends.
PointsFn = Callable[[list[FrameKeypoints], int, int], list[tuple[float, float]]]


class MeasureOutcome(NamedTuple):
    """One number, or the reason there isn't one.

    Replaced a bare `float | None`. The `None` was never ambiguous about *whether* the measurement
    failed, but it threw away *which* of six conditions failed — and that was the one thing every
    consumer downstream wanted and had to guess at (`contracts.unscored` tells that story).

    The reason costs nothing to produce here and cannot be recovered anywhere else: by the time a
    `None` reaches `feedback`, the window it came from, the landmark group it was reading and the
    gate it failed are all gone.
    """

    #: The measurement, or None if it could not be taken. Never a sentinel, never a zero.
    value: float | None
    #: Which condition failed, or None when `value` is present. Always in `MEASUREMENT_REASONS`.
    reason: UnscoredReason | None = None
    #: Which window or landmark group, for a human. `MeasureOutcome.value` is what code reads.
    detail: str = ""

    @classmethod
    def measured(cls, value: float) -> MeasureOutcome:
        return cls(value)

    @classmethod
    def unmeasurable(cls, reason: UnscoredReason, detail: str) -> MeasureOutcome:
        """`detail` is required, not optional — a reason with no window named is half an answer."""
        return cls(None, reason, detail)


#: A full pose measurement: smoothed frames + phases in, one number or a reason out.
MeasureFn = Callable[[list[FrameKeypoints], list[PhaseSegment]], MeasureOutcome]

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


class TempoTimings(NamedTuple):
    """The two durations, or the reason they could not be read.

    A second shape beside `MeasureOutcome` rather than a reuse of it, because this carries a *pair*
    and that one carries a number. Both exist for the same reason: the three ways tempo goes
    missing are genuinely different clips, and merging them into one `None` is what forced every
    consumer to guess.
    """

    #: `(backswing_ms, downswing_ms)`, or None if they could not be read.
    durations: tuple[float, float] | None
    reason: UnscoredReason | None = None
    detail: str = ""


def tempo_timings(phases: list[PhaseSegment]) -> TempoTimings:
    """Extract (backswing_ms, downswing_ms) from the phase boundaries, or say why not.

    Uses motion start (`BACKSWING.start_ms`), the top of the backswing (center of the
    `TRANSITION` window), and impact (`IMPACT.start_ms`).

    **Address is the dominant error term in this ratio.** It sits directly in the numerator, and it
    is the least accurate instant we have — median 7 frames against 2 for the top and 1 for impact.
    Worth keeping in proportion: a rule that ignores the pose entirely and assumes the tour-median
    tempo locates address to within 11 frames, so the wrist signal contributes real but modest
    information here (M4-REF Phase B6).

    Refuses when the backswing boundary was **estimated rather than detected**
    (`BOUNDARY_ESTIMATED`). That estimate is derived from an assumed tempo ratio, so reporting it
    would echo that assumption back as an observation — a guess dressed as a measurement. Dropping
    it is ADR-010 §2 and ADR-013; it costs ~14% of clips their tempo reading, which is why it gets
    its own reason rather than sharing one with an unreadable clip: the footage was fine.
    """
    by_phase = {segment.phase: segment for segment in phases}
    backswing = by_phase.get(SwingPhase.BACKSWING)
    transition = by_phase.get(SwingPhase.TRANSITION)
    impact = by_phase.get(SwingPhase.IMPACT)
    missing = [
        name
        for name, segment in (
            ("backswing", backswing),
            ("transition", transition),
            ("impact", impact),
        )
        if segment is None
    ]
    if backswing is None or transition is None or impact is None:
        return TempoTimings(
            None,
            UnscoredReason.PHASE_NOT_SEGMENTED,
            "tempo needs the backswing, transition and impact phases; "
            f"missing {', '.join(missing)}",
        )
    if not backswing.detected:
        return TempoTimings(
            None,
            UnscoredReason.BOUNDARY_ESTIMATED,
            "the start of the backswing was estimated from an assumed tempo, not detected",
        )

    motion_start_ms = backswing.start_ms
    top_ms = (transition.start_ms + transition.end_ms) / 2
    impact_ms = impact.start_ms

    backswing_ms = top_ms - motion_start_ms
    downswing_ms = impact_ms - top_ms
    if backswing_ms <= 0 or downswing_ms <= 0:
        return TempoTimings(
            None,
            UnscoredReason.TIMING_DEGENERATE,
            f"backswing {backswing_ms:.0f} ms and downswing {downswing_ms:.0f} ms; "
            "both must be positive",
        )
    return TempoTimings((backswing_ms, downswing_ms))


def _lateral_travel(
    keypoints: list[FrameKeypoints],
    phases: list[PhaseSegment],
    points_of: PointsFn,
    to_phase: SwingPhase,
    landmarks: str,
) -> MeasureOutcome:
    """Shared body of the address-to-instant lateral metrics: |Δx| in shoulder widths.

    Both endpoints are means over a window and the ruler comes from the address window, which is
    what makes this family robust and what keeps it aspect-immune (an `x` distance over an `x`
    scale cancels the 16:9 assumption outright).

    `landmarks` is the human name of what `points_of` reads ("ear midpoint"), and exists only for
    `detail`. Deriving it from the callable would mean matching on a function identity, which is
    the kind of cleverness that goes stale the first time someone passes a lambda.
    """
    setup = address_sample_bounds(phases)
    if setup is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.PHASE_NOT_SEGMENTED,
            "the address sampling window needs the address and downswing phases",
        )
    target = phase_bounds(phases, to_phase)
    if target is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.PHASE_NOT_SEGMENTED,
            f"no {to_phase.value} phase to measure travel to",
        )

    # Scale first: without a ruler the endpoints are unusable even when both are present, and
    # reporting "landmarks unconfident" for a golfer who simply stood side-on would send them off
    # to fix the wrong thing.
    width = shoulder_width(keypoints, setup[0], setup[1])
    if width is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.SCALE_UNAVAILABLE,
            "no confident, non-degenerate shoulder width across the address window",
        )

    start = mean_of(points_of(keypoints, setup[0], setup[1]))
    if start is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.LANDMARKS_UNCONFIDENT,
            f"no confident {landmarks} frames in the address window",
        )
    end = mean_of(points_of(keypoints, target[0], target[1]))
    if end is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.LANDMARKS_UNCONFIDENT,
            f"no confident {landmarks} frames in the {to_phase.value} window",
        )
    return MeasureOutcome.measured(abs(end[0] - start[0]) / width)


# --------------------------------------------------------------------------- the measurements


def measure_tempo_ratio(phases: list[PhaseSegment]) -> MeasureOutcome:
    """Backswing:downswing time ratio. Frame rate cancels, so slo-mo and real-time agree."""
    timings = tempo_timings(phases)
    if timings.durations is None:
        # `tempo_timings` already decided which of the three conditions failed; re-deciding here
        # would be a second opinion on the same evidence.
        assert timings.reason is not None
        return MeasureOutcome.unmeasurable(timings.reason, timings.detail)
    backswing_ms, downswing_ms = timings.durations
    return MeasureOutcome.measured(backswing_ms / downswing_ms)


def measure_backswing_ms(phases: list[PhaseSegment]) -> MeasureOutcome:
    """Backswing duration in milliseconds — motion start to the top.

    Unlike `measure_tempo_ratio`, frame rate does **not** cancel here, so this is only meaningful
    on a real-time clip. That is a property of the capture rather than of this function, which is
    why it is not gated here: `phases` carries millisecond timestamps that the pipeline already
    derived from the clip's own fps, and a slow-motion clip would have arrived with slow-motion
    timestamps long before this point.
    """
    return _tempo_duration(phases, backswing=True)


def measure_downswing_ms(phases: list[PhaseSegment]) -> MeasureOutcome:
    """Downswing duration in milliseconds — the top to impact. See `measure_backswing_ms`."""
    return _tempo_duration(phases, backswing=False)


def _tempo_duration(phases: list[PhaseSegment], *, backswing: bool) -> MeasureOutcome:
    """One half of `tempo_timings`, as a measurement.

    Both halves and the ratio come from the same call, so all three inherit its three refusal
    paths and cannot disagree about whether a swing was timeable — a clip whose backswing boundary
    was estimated has no tempo *and* no duration, for the one reason.
    """
    timings = tempo_timings(phases)
    if timings.durations is None:
        assert timings.reason is not None
        return MeasureOutcome.unmeasurable(timings.reason, timings.detail)
    return MeasureOutcome.measured(timings.durations[0 if backswing else 1])


def measure_head_sway(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> MeasureOutcome:
    """Lateral head travel, address window -> impact window, in shoulder widths. [v3]"""
    return _lateral_travel(
        keypoints, phases, head_center_points, SwingPhase.IMPACT, "ear midpoint"
    )


def measure_hip_sway(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> MeasureOutcome:
    """Lateral hip travel, address window -> impact window, in shoulder widths.

    The structural twin of `measure_head_sway`, and it answers a different coaching question: a
    head that stays put over hips that slide is a lateral slide, while head and hips moving
    together is a body that swayed. Neither is visible from head sway alone.
    """
    return _lateral_travel(
        keypoints, phases, hip_center_points, SwingPhase.IMPACT, "hip midpoint"
    )


def measure_hip_shift_at_top(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> MeasureOutcome:
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
    return _lateral_travel(
        keypoints, phases, hip_center_points, SwingPhase.TRANSITION, "hip midpoint"
    )


def measure_head_hip_offset_impact(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> MeasureOutcome:
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
    if setup is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.PHASE_NOT_SEGMENTED,
            "the address sampling window needs the address and downswing phases",
        )
    impact = phase_bounds(phases, SwingPhase.IMPACT)
    if impact is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.PHASE_NOT_SEGMENTED, "no impact phase to read the offset at"
        )

    width = shoulder_width(keypoints, setup[0], setup[1])
    if width is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.SCALE_UNAVAILABLE,
            "no confident, non-degenerate shoulder width across the address window",
        )

    head = mean_of(head_center_points(keypoints, impact[0], impact[1]))
    hips = mean_of(hip_center_points(keypoints, impact[0], impact[1]))
    if head is None or hips is None:
        missing = "ear midpoint" if head is None else "hip midpoint"
        return MeasureOutcome.unmeasurable(
            UnscoredReason.LANDMARKS_UNCONFIDENT,
            f"no confident {missing} frames in the impact window",
        )
    return MeasureOutcome.measured((head[0] - hips[0]) / width)


def measure_head_hip_gain(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> MeasureOutcome:
    """Change in the signed head-hip offset from address to impact, in shoulder widths. [M6.5]

    The *same* quantity as `measure_head_hip_offset_impact`, read as a change across the swing
    rather than as an absolute position: how much rearward separation between head and hips the
    golfer's motion actually created. Negative means the offset moved head-behind-hips during the
    swing, which is what a tour swing does — the hips shift toward the target while the head stays
    put. A value near zero means head and hips travelled together.

    **Why this exists beside the absolute, rather than instead of it.** The absolute offset is the
    one metric in this module that is a *static difference between two body parts at one instant*;
    every other one is a difference of a single landmark across time. That distinction is not
    stylistic — it decides whether a camera-geometry bias cancels. Shoulder-width normalization
    removes the `1/Z` scale, so distance from the camera is handled, but it does not remove **yaw**:
    a camera off-square to the target line converts the head/hip *depth* difference into apparent
    horizontal offset, and at impact the hips have rotated open while the head has not, so the two
    sit at genuinely different depths exactly where the absolute is read.

    `scripts/golfdb/check_metric_transfer.py` measured that directly and it is not small. Against
    the GolfDB corpus our own bay clips carry a **0.32 shoulder-width** disagreement *at address*,
    where the body is square and there is no swing yet to disagree about — 55% of the total gap at
    impact, and about 4x this metric's own measurement error. Subtracting the address reading
    removes the static term by construction and leaves the part the swing produced, which is also
    the part the coaching concept ("stay behind the ball") is actually about.

    Both readings share **one ruler** — the shoulder width measured over the address window — so
    the subtraction is of two comparable quantities. Using each instant's own width would put a
    second difference into the result and defeat the point.

    Signed and camera-relative, exactly like the absolute: positive is image-right, and which side
    that is in swing terms depends on handedness. `analysis` records the raw camera-frame number
    here and the judging layer normalizes it — see `checkpoints.mechanics.evaluate_head_stays_back`.
    """
    setup = address_sample_bounds(phases)
    if setup is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.PHASE_NOT_SEGMENTED,
            "the address sampling window needs the address and downswing phases",
        )
    impact = phase_bounds(phases, SwingPhase.IMPACT)
    if impact is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.PHASE_NOT_SEGMENTED, "no impact phase to difference the offset against"
        )

    width = shoulder_width(keypoints, setup[0], setup[1])
    if width is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.SCALE_UNAVAILABLE,
            "no confident, non-degenerate shoulder width across the address window",
        )

    offsets: list[float] = []
    for window, lo, hi in (("address", *setup), ("impact", *impact)):
        head = mean_of(head_center_points(keypoints, lo, hi))
        hips = mean_of(hip_center_points(keypoints, lo, hi))
        if head is None or hips is None:
            missing = "ear midpoint" if head is None else "hip midpoint"
            return MeasureOutcome.unmeasurable(
                UnscoredReason.LANDMARKS_UNCONFIDENT,
                f"no confident {missing} frames in the {window} window",
            )
        offsets.append((head[0] - hips[0]) / width)
    return MeasureOutcome.measured(offsets[1] - offsets[0])


def measure_finish_balance(
    keypoints: list[FrameKeypoints], phases: list[PhaseSegment]
) -> MeasureOutcome:
    """Hip-center drift from its own mean through the follow-through, in shoulder widths. [v2]"""
    follow_through = phase_bounds(phases, SwingPhase.FOLLOW_THROUGH)
    if follow_through is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.PHASE_NOT_SEGMENTED,
            "no follow-through phase — the clip may end at impact",
        )
    setup = address_sample_bounds(phases)
    if setup is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.PHASE_NOT_SEGMENTED,
            "the address sampling window needs the address and downswing phases",
        )

    width = shoulder_width(keypoints, setup[0], setup[1])
    if width is None:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.SCALE_UNAVAILABLE,
            "no confident, non-degenerate shoulder width across the address window",
        )

    # `TOO_FEW_FRAMES` rather than `LANDMARKS_UNCONFIDENT` even at zero: the hips are gated at
    # `MIN_HIP_VISIBILITY` here, harder than anywhere else, so "not enough of them" is the honest
    # description of both a short follow-through and an occluded one. `mean_of` below cannot then
    # return None, and the guard stays only because the type says it can.
    points = hip_center_points(keypoints, follow_through[0], follow_through[1])
    if len(points) < MIN_FINISH_FRAMES:
        return MeasureOutcome.unmeasurable(
            UnscoredReason.TOO_FEW_FRAMES,
            f"{len(points)} confident hip frames through the follow-through, "
            f"need {MIN_FINISH_FRAMES}",
        )

    center = mean_of(points)
    if center is None:  # pragma: no cover - unreachable past the length check above
        return MeasureOutcome.unmeasurable(
            UnscoredReason.TOO_FEW_FRAMES, "no confident hip frames through the follow-through"
        )
    mean_x, mean_y = center
    drifts = [((x - mean_x) ** 2 + (y - mean_y) ** 2) ** 0.5 for x, y in points]
    return MeasureOutcome.measured(percentile(drifts, FINISH_DRIFT_QUANTILE) / width)


class PoseMeasurement(NamedTuple):
    """How to take one pose measurement, and what the resulting number is.

    Was three dicts keyed by the same seven names — the function, the unit and the detail string —
    which is three places to edit and two chances to add a metric that measures fine and then
    stores with the wrong unit. Nothing enforced that they agreed except a test asserting their key
    sets matched, which caught a missing entry but never a misaligned one.

    Unpacks positionally as `(measure, unit, detail)`, which is the shape `shot_measure`'s registry
    already had, so the engine really does build both families the same way.
    """

    #: Takes the smoothed frames and the phase segmentation, returns a `MeasureOutcome` — the
    #: number, or which condition stopped it being taken.
    measure: MeasureFn
    #: Goes onto `Measurement.unit`.
    unit: str
    #: How the measurement is taken, carried onto `Measurement.detail` so a stored number can be
    #: re-derived or re-normalized later without reading this module.
    detail: str


#: Name -> how to measure it, for everything derived from face-on pose over a full swing.
#:
#: The registry is the point: `derive_pose_metrics.py` iterates it instead of hardcoding metric
#: names (it hardcoded two, in three places), and `tune_spatial_metric.py` scores whatever is in
#: it. Adding a candidate metric is one line here, and every offline tool picks it up.
#:
#: Names ending `_norm` are shoulder-width-normalized, which `derive_reference.py` keys on to
#: recommend a one-sided band. `tempo_ratio` is deliberately outside that convention.
#:
#: Not every metric here is judged. A name gains a checkpoint by appearing in
#: `contracts.checkpoints.CHECKPOINT_REGISTRY` and a band in `benchmarks/ranges.json`; until then
#: it is measured and stored and nothing else, which is the order M6.5 exists to allow.
POSE_MEASUREMENTS: dict[str, PoseMeasurement] = {
    "tempo_ratio": PoseMeasurement(
        lambda frames, phases: measure_tempo_ratio(phases),
        "ratio",
        "backswing:downswing time, from phase instants; frame rate cancels",
    ),
    # The two halves the ratio above is built from, kept rather than divided away. Nothing judges
    # them — no band, no checkpoint, no placement — because tempo is already scored once and a
    # second verdict over the same two instants would double-count it (ADR-023). They are here so
    # a stored swing can say "your downswing took 384 ms", which the ratio cannot recover, and as
    # the groundwork for diagnosing *which half* is off. M6.5's measure-now-judge-later order.
    "backswing_ms": PoseMeasurement(
        lambda frames, phases: measure_backswing_ms(phases),
        "ms",
        "motion start -> top, from phase instants; real-time clips only, fps does not cancel",
    ),
    "downswing_ms": PoseMeasurement(
        lambda frames, phases: measure_downswing_ms(phases),
        "ms",
        "top -> impact, from phase instants; real-time clips only, fps does not cancel",
    ),
    "head_sway_norm": PoseMeasurement(
        measure_head_sway,
        "shoulder_widths",
        "|dx| of ear midpoint, address window -> impact window",
    ),
    "finish_balance_norm": PoseMeasurement(
        measure_finish_balance,
        "shoulder_widths",
        "p90 hip-center drift through follow-through",
    ),
    "hip_sway_norm": PoseMeasurement(
        measure_hip_sway,
        "shoulder_widths",
        "|dx| of hip midpoint, address window -> impact window",
    ),
    "hip_shift_at_top_norm": PoseMeasurement(
        measure_hip_shift_at_top,
        "shoulder_widths",
        "|dx| of hip midpoint, address window -> transition window",
    ),
    "head_hip_offset_impact_norm": PoseMeasurement(
        measure_head_hip_offset_impact,
        "shoulder_widths",
        "signed (head - hips) dx over the impact window; + is image-right, camera-frame — "
        "carries a static camera bias, see head_hip_gain_norm",
    ),
    "head_hip_gain_norm": PoseMeasurement(
        measure_head_hip_gain,
        "shoulder_widths",
        "change in signed (head - hips) dx, address window -> impact window, one shared "
        "shoulder-width ruler; + is image-right, camera-frame, handedness resolved when judged",
    ),
}


#: The measurements that are an absolute time, and therefore **not** frame-rate invariant.
#:
#: Every other metric in the registry is a ratio or a shoulder-width, which is why the GolfDB
#: tooling can treat a slow-motion clip and a real-time clip as the same kind of evidence: about
#: 47% of that corpus is slow-motion and a ratio survives it. These two do not.
#:
#: That matters to the offline scripts specifically, because
#: `derive_pose_metrics._phases_from_events` builds labelled phases with **frame indices in the
#: `start_ms` field** — deliberately, and documented there, since only ratios were ever read from
#: them. A duration measured off those phases is a frame count wearing a millisecond's name. It
#: would be written into `swings.jsonl` beside real milliseconds and cut into a band, and nothing
#: in `tests/` covers those scripts.
#:
#: Derived from the unit rather than listed, so a third duration metric is covered the day it is
#: added rather than the day someone remembers this set exists.
FPS_DEPENDENT_MEASUREMENTS: frozenset[str] = frozenset(
    name for name, measurement in POSE_MEASUREMENTS.items() if measurement.unit == "ms"
)
