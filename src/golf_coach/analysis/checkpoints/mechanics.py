"""Mechanics checkpoints — pose-based, intent-independent. [M4-PoC+]

Three checkpoints, all measured from **face-on 2D pose** (the canonical pose-camera placement,
ADR-003 addendum) — deliberately the ones this single view reads well:

- **tempo** — backswing:downswing time ratio, from phase timings.
- **head_sway** — lateral (`x`) head travel from address to impact, in shoulder-widths.
- **finish_balance** — how still the body settles through the follow-through, in shoulder-widths.

Each compares an observed value against a benchmark band resolved from the store (ADR-010) and
returns a `CheckpointScore`, or `None` when the data is unusable or the store has no band — no
score beats a wrong one (ADR-010 §2). Pure functions, stdlib only. Checkpoints needing depth
(spine tilt, hip rotation, swing plane) are *not* here — they need a down-the-line/synced view
(ADR-011) and stay deferred; see docs/M4_FUNDAMENTALS_PANEL.md.

Each score also carries **where it sits in the reference population** (`percentile`,
`population_n`), read from `golfdb_v1.json` via `_population_placement`. That is strictly
informational — it never touches `score` or `passed`, which stay on `ranges.json` alone (ADR-010 §2
and its percentile addendum). It exists because `score` is 1.0 for *every* passing checkpoint and
so cannot rank them: a swing can score 100/100 and still have one fundamental sitting higher than
83% of tour swings, which is exactly the thing worth telling a golfer.

**Metric definitions version 3** (M4-REF Phase B6, 2026-08-02). A band is only meaningful against
the definition it was cut from, so each generation is numbered and the bands re-derived with it:

- **v2** — head sway measured from the ear midpoint rather than the `NOSE`, and finish drift
  summarized at p90 rather than `max()`.
- **v3** — `head_sway`'s address endpoint and *both* checkpoints' shoulder-width ruler read
  `_address_sample_bounds` (a short window ending at the address boundary) instead of averaging
  across the whole ADDRESS phase, which began at frame 0 and so averaged over the golfer walking
  into shot. Moved `head_sway_norm` p90 0.42 → 0.43 and `finish_balance_norm` p90 0.28 → 0.29.

See `docs/M4_POSE_BAKEOFF.md` for the before/after and `golfdb_v1.json`'s
`metric_definitions_version` for which definition a band belongs to.

HARDWARE-REVALIDATE: `head_sway` / `finish_balance` bands are now derived from 458 face-on tour
swings rather than eyeballed (ADR-012), but a tour population says what *good* looks like, not what
*this camera* measures. Re-check against our own captured ground truth, and revisit the deferred
depth checkpoints, when the down-the-line camera / 3D fusion land (ADR-011).
"""

from __future__ import annotations

from golf_coach.analysis.benchmarks import load_distribution, resolve_range
from golf_coach.analysis.stats import percentile
from golf_coach.contracts.intent import ClubCategory, PlayerProfile
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import CheckpointScore, PhaseSegment, SwingPhase

TEMPO_CHECKPOINT = "tempo"
_TEMPO_RANGE_KEY = "tempo_ratio"

HEAD_SWAY_CHECKPOINT = "head_sway"
_HEAD_SWAY_RANGE_KEY = "head_sway_norm"

FINISH_BALANCE_CHECKPOINT = "finish_balance"
_FINISH_BALANCE_RANGE_KEY = "finish_balance_norm"

# Landmarks dimmer than this are treated as unreliable (MediaPipe convention, matches
# phases.py / overlay.py).
_MIN_VISIBILITY = 0.5

# Hips at the finish are the worst-conditioned landmarks we read: the golfer has rotated ~90°, so
# from a face-on camera the trail hip sits behind the lead hip and the torso, and the arms and club
# cross the body right there. MediaPipe's `visibility` is a learned logit rather than a calibrated
# probability, so the usual 0.5 gate happily passes confidently-wrong hip estimates — exactly the
# frames `finish_balance` is most sensitive to. Gate them harder than everything else.
_MIN_HIP_VISIBILITY = 0.7

# A face-on shoulder width (normalized) below this is degenerate (golfer turned side-on or
# shoulders mis-detected) — we can't form a reliable scale, so the checkpoint bails.
_MIN_SHOULDER_WIDTH = 0.02

# Fewest follow-through frames needed to judge how still the finish settles.
_MIN_FINISH_FRAMES = 3

# The posture-sampling window (see `_address_sample_bounds`): this fraction of the clip's own
# downswing duration, floored at a handful of frames. Half a downswing is ~4 frames on a real-time
# clip and ~15 in slow motion — long enough that averaging suppresses landmark jitter, short enough
# that it cannot reach back into the golfer walking up to the ball.
_ADDRESS_SAMPLE_FRACTION = 0.5
_ADDRESS_SAMPLE_MIN_FRAMES = 5

# Finish drift is summarized at this quantile rather than by `max()`. [metric definitions v2]
# `max()` is an extreme-value statistic: one bad frame sets the entire metric, and the frames it
# reads are the most occlusion-prone in the swing (see `_MIN_HIP_VISIBILITY`). p90 still captures
# "how far does the body actually stagger" — it is a high quantile, not a central one — while
# needing the drift to persist across several frames before it counts.
_FINISH_DRIFT_QUANTILE = 0.90


def _score_within_range(observed: float, low: float, high: float) -> float:
    """1.0 inside `[low, high]`, decaying linearly (by band-widths) to 0.0 outside.

    Shared scorer for every mechanics checkpoint in this module. For a one-sided "lower is
    better" metric (sway, finish drift) pass `low=0.0`, so any non-negative value inside the
    band scores 1.0 and only overshoot past `high` is penalised.
    """
    if low <= observed <= high:
        return 1.0
    width = high - low
    if width <= 0:
        return 0.0
    distance = (low - observed) if observed < low else (observed - high)
    return max(0.0, 1.0 - distance / width)


def _population_placement(range_key: str, observed: float) -> tuple[float | None, int | None]:
    """Where `observed` sits in the reference population: `(percentile, n)`, or `(None, None)`.

    **Informational only — never feeds `score` or `passed`.** Scoring reads `ranges.json` and
    nothing else (ADR-010 §2, and the percentile addendum that admits this call). The band answers
    *is this a fault*; the percentile answers *how far off, in units comparable to the other
    checkpoints*, which is what lets `feedback` rank tips instead of listing them.

    Queries the **same stratum the band was cut from** — `(club, sex, view)` all `"all"`, the
    `load_distribution` defaults. That consistency is load-bearing, not laziness: `tempo_ratio`'s
    face-on stratum has p90 5.00 against the all-view 4.71, so drawing the percentile from a
    narrower stratum than the band would let a swing read "inside the band" and "past the 90th
    percentile" at once. Per-club strata are a later change that has to move the band *and* the
    percentile together.
    """
    distribution = load_distribution(range_key)
    if distribution is None:
        return None, None
    return round(distribution.percentile_of(observed), 1), distribution.n


def _placement_clause(pct: float, n: int, below_label: str, above_label: str) -> str:
    """Plain-English population placement, phrased for a golfer rather than a statistician.

    `pct` is the share of the population *below* the observed value, so a swing under the median is
    described against the share it undercuts (`below_label`) and one over it against the share it
    exceeds (`above_label`). Callers supply both labels because the comparison only reads naturally
    in the metric's own vocabulary — "a quicker tempo than" and "more head movement than" are not
    the same sentence with a sign flipped.

    `Distribution.percentile_of` clamps to `[10, 90]` because the tails were never stored, so at
    the rails this says "at least" rather than quoting a rank the stored quantiles cannot support.

    (Named `pct`, not `percentile`, so it cannot shadow the `stats.percentile` imported above.)
    """
    if pct <= 50.0:
        share, label, at_rail = 100.0 - pct, below_label, pct <= 10.0
    else:
        share, label, at_rail = pct, above_label, pct >= 90.0
    qualifier = "at least " if at_rail else ""
    return f"{label} {qualifier}{share:.0f}% of {n} tour swings"


def _phase_bounds(phases: list[PhaseSegment], phase: SwingPhase) -> tuple[int, int] | None:
    """Inclusive `(start_frame, end_frame)` for a phase, or None if it isn't present."""
    for segment in phases:
        if segment.phase is phase:
            return segment.start_frame, segment.end_frame
    return None


def _midpoint_series(
    keypoints: list[FrameKeypoints],
    lo: int,
    hi: int,
    first: PoseLandmark,
    second: PoseLandmark,
    min_visibility: float = _MIN_VISIBILITY,
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


def _mean_of(points: list[tuple[float, float]]) -> tuple[float, float] | None:
    """Mean `(x, y)` of a point series, or None if it is empty."""
    if not points:
        return None
    return (
        sum(x for x, _ in points) / len(points),
        sum(y for _, y in points) / len(points),
    )


def _shoulder_width(keypoints: list[FrameKeypoints], lo: int, hi: int) -> float | None:
    """Mean face-on shoulder width (normalized) over a span — the scale-invariance ruler.

    Returns None if too few confident frames or the width is degenerate (side-on / mis-detect).
    """
    widths = [
        abs(kp.landmark(PoseLandmark.LEFT_SHOULDER).x - kp.landmark(PoseLandmark.RIGHT_SHOULDER).x)
        for kp in keypoints[lo : hi + 1]
        if kp.landmark(PoseLandmark.LEFT_SHOULDER).visibility >= _MIN_VISIBILITY
        and kp.landmark(PoseLandmark.RIGHT_SHOULDER).visibility >= _MIN_VISIBILITY
    ]
    if not widths:
        return None
    width = sum(widths) / len(widths)
    return width if width >= _MIN_SHOULDER_WIDTH else None


def _address_sample_bounds(phases: list[PhaseSegment]) -> tuple[int, int] | None:
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
    floor of `_ADDRESS_SAMPLE_MIN_FRAMES` so there is always enough to average. Returns None only
    when the phases are unusable.
    """
    address = _phase_bounds(phases, SwingPhase.ADDRESS)
    downswing = _phase_bounds(phases, SwingPhase.DOWNSWING)
    if address is None or downswing is None:
        return None

    span = max(downswing[1] - downswing[0], 1)
    length = max(_ADDRESS_SAMPLE_MIN_FRAMES, round(_ADDRESS_SAMPLE_FRACTION * span))

    hi = address[1]
    lo = max(address[0], hi - length)
    # A clip whose takeaway starts almost immediately (or where the boundary was estimated at 0)
    # leaves nothing behind it — widen forward instead, since the frames just after a mis-placed
    # boundary are still far closer to setup than the start of the clip is.
    if hi - lo < _ADDRESS_SAMPLE_MIN_FRAMES:
        hi = min(downswing[0], lo + _ADDRESS_SAMPLE_MIN_FRAMES)
    return (lo, max(hi, lo))


def _hip_center_points(
    keypoints: list[FrameKeypoints], lo: int, hi: int
) -> list[tuple[float, float]]:
    """Per-frame hip-center `(x, y)` over the inclusive span, confident frames only."""
    return _midpoint_series(
        keypoints,
        lo,
        hi,
        PoseLandmark.LEFT_HIP,
        PoseLandmark.RIGHT_HIP,
        min_visibility=_MIN_HIP_VISIBILITY,
    )


def _head_center_points(
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
    return _midpoint_series(
        keypoints, lo, hi, PoseLandmark.LEFT_EAR, PoseLandmark.RIGHT_EAR
    )


def _tempo_timings(phases: list[PhaseSegment]) -> tuple[float, float] | None:
    """Extract (backswing_ms, downswing_ms) from the phase boundaries, or None if unavailable.

    Uses motion start (`BACKSWING.start_ms`), the top of the backswing (center of the
    `TRANSITION` window), and impact (`IMPACT.start_ms`).

    **Address is the dominant error term in this ratio.** It sits directly in the numerator, and it
    is the least accurate instant we have — median 7 frames against 2 for the top and 1 for impact.
    Worth keeping in proportion: a rule that ignores the pose entirely and assumes the tour-median
    tempo locates address to within 11 frames, so the wrist signal contributes real but modest
    information here (M4-REF Phase B6).

    Returns None when the backswing boundary was **estimated rather than detected**. That estimate
    is derived from an assumed tempo ratio, so scoring it would report that assumption back as an
    observation — a wrong score dressed as a measurement. Dropping the checkpoint is ADR-010 §2 and
    ADR-013; it costs ~14% of clips their tempo reading.
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


def evaluate_tempo(
    phases: list[PhaseSegment],
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> CheckpointScore | None:
    """Score swing tempo (backswing:downswing ratio) against the benchmark band.

    Returns `None` when the phases don't yield a usable tempo or when the store has no band
    for this checkpoint — in both cases the caller simply omits a tempo score.
    """
    timings = _tempo_timings(phases)
    if timings is None:
        return None
    backswing_ms, downswing_ms = timings
    observed = backswing_ms / downswing_ms

    band = resolve_range(_TEMPO_RANGE_KEY, club, profile)
    if band is None:
        return None

    score = _score_within_range(observed, band.low, band.high)
    passed = band.low <= observed <= band.high
    # Quote the resolved band rather than a hardcoded "~3:1". The band is sourced data (ADR-010)
    # and has already been re-cut once — from Novosel's 2.7-3.3 estimate to the p10-p90 of the
    # GolfDB tour population — so a literal here would have started lying the moment it moved.
    target = f"tour range {band.low:g}-{band.high:g}:1"
    if passed:
        message = f"Good tempo - {observed:.1f}:1 backswing:downswing (inside the {target})."
    elif observed < band.low:
        message = (
            f"Tempo too quick - {observed:.1f}:1. The downswing is rushing the backswing; "
            f"feel a smoother, fuller backswing (aim for the {target})."
        )
    else:
        message = (
            f"Tempo too slow - {observed:.1f}:1. The backswing is dragging relative to the "
            f"downswing; let the downswing flow a touch quicker (aim for the {target})."
        )

    pct, population_n = _population_placement(_TEMPO_RANGE_KEY, observed)
    if pct is not None and population_n is not None:
        clause = _placement_clause(
            pct, population_n, "a quicker tempo than", "a slower tempo than"
        )
        message += f" You swing with {clause}."

    return CheckpointScore(
        name=TEMPO_CHECKPOINT,
        score=score,
        passed=passed,
        observed=round(observed, 2),
        expected_low=band.low,
        expected_high=band.high,
        message=message,
        percentile=pct,
        population_n=population_n,
        one_sided=False,
    )


def evaluate_head_sway(
    keypoints: list[FrameKeypoints],
    phases: list[PhaseSegment],
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> CheckpointScore | None:
    """Score lateral head stability: `x` travel of the head center from address to impact.

    The head center is the **ear midpoint** (see `_head_center_points`), measured in
    shoulder-widths so it is independent of the golfer's distance from the camera. Face-on is the
    ideal view for side-to-side sway. Both endpoints are means over a window, which is what makes
    this checkpoint robust: averaging N frames suppresses zero-mean landmark jitter by √N, so the
    remaining error is dominated by *definition*, not by the pose model.

    The address endpoint reads `_address_sample_bounds`, **not** the full ADDRESS phase — averaging
    over the whole phase averaged over the golfer walking into frame. See that helper for why.

    Returns `None` if the phases/landmarks are unusable or the store has no band (the caller then
    omits a sway score).
    """
    setup = _address_sample_bounds(phases)
    impact = _phase_bounds(phases, SwingPhase.IMPACT)
    if setup is None or impact is None:
        return None

    head_address = _mean_of(_head_center_points(keypoints, setup[0], setup[1]))
    head_impact = _mean_of(_head_center_points(keypoints, impact[0], impact[1]))
    width = _shoulder_width(keypoints, setup[0], setup[1])
    if head_address is None or head_impact is None or width is None:
        return None

    observed = abs(head_impact[0] - head_address[0]) / width

    band = resolve_range(_HEAD_SWAY_RANGE_KEY, club, profile)
    if band is None:
        return None

    score = _score_within_range(observed, band.low, band.high)
    passed = band.low <= observed <= band.high
    if passed:
        message = (
            f"Good head stability - {observed:.2f} shoulder-widths of lateral head movement "
            "to impact (nicely centered over the ball)."
        )
    else:
        message = (
            f"Head sway - {observed:.2f} shoulder-widths of lateral movement to impact. "
            f"Keep your head centered over the ball (aim under {band.high})."
        )

    pct, population_n = _population_placement(_HEAD_SWAY_RANGE_KEY, observed)
    if pct is not None and population_n is not None:
        clause = _placement_clause(
            pct, population_n, "less head movement than", "more head movement than"
        )
        message += f" That is {clause}."

    return CheckpointScore(
        name=HEAD_SWAY_CHECKPOINT,
        score=score,
        passed=passed,
        observed=round(observed, 2),
        expected_low=band.low,
        expected_high=band.high,
        message=message,
        percentile=pct,
        population_n=population_n,
        one_sided=True,
    )


def evaluate_finish_balance(
    keypoints: list[FrameKeypoints],
    phases: list[PhaseSegment],
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> CheckpointScore | None:
    """Score finish balance: how far the hip-center drifts from its own mean through follow-through.

    A balanced swing settles into a held finish (small drift); an off-balance one keeps
    staggering. Measured in shoulder-widths for scale-invariance.

    The shoulder-width ruler comes from `_address_sample_bounds` rather than the full ADDRESS
    phase: measured over the whole phase it was off by more than 10% on 12% of GolfDB clips, and it
    divides the metric, so that error lands straight on the score.

    Drift is summarized at the **p90** of the per-frame series rather than its `max`
    (`_FINISH_DRIFT_QUANTILE`, metric definitions v2). `max` made this the one checkpoint where a
    single mis-detected frame passed straight through to the score unattenuated — the opposite of
    how `head_sway` averages over a window — and it read hips at the most occluded moment in the
    swing. Hip landmarks are additionally gated at `_MIN_HIP_VISIBILITY`.

    Returns `None` if there are too few confident follow-through frames or the store has no band.
    """
    follow_through = _phase_bounds(phases, SwingPhase.FOLLOW_THROUGH)
    setup = _address_sample_bounds(phases)
    if follow_through is None or setup is None:
        return None

    points = _hip_center_points(keypoints, follow_through[0], follow_through[1])
    width = _shoulder_width(keypoints, setup[0], setup[1])
    if len(points) < _MIN_FINISH_FRAMES or width is None:
        return None

    center = _mean_of(points)
    if center is None:
        return None
    mean_x, mean_y = center
    drifts = [((x - mean_x) ** 2 + (y - mean_y) ** 2) ** 0.5 for x, y in points]
    observed = percentile(drifts, _FINISH_DRIFT_QUANTILE) / width

    band = resolve_range(_FINISH_BALANCE_RANGE_KEY, club, profile)
    if band is None:
        return None

    score = _score_within_range(observed, band.low, band.high)
    passed = band.low <= observed <= band.high
    if passed:
        message = (
            f"Balanced finish - the body settles within {observed:.2f} shoulder-widths through "
            "the follow-through (held and steady)."
        )
    else:
        message = (
            f"Unbalanced finish - {observed:.2f} shoulder-widths of drift after impact. "
            f"Swing to a held, balanced finish (aim under {band.high})."
        )

    pct, population_n = _population_placement(_FINISH_BALANCE_RANGE_KEY, observed)
    if pct is not None and population_n is not None:
        clause = _placement_clause(
            pct, population_n, "a steadier finish than", "a looser finish than"
        )
        message += f" That is {clause}."

    return CheckpointScore(
        name=FINISH_BALANCE_CHECKPOINT,
        score=score,
        passed=passed,
        observed=round(observed, 2),
        expected_low=band.low,
        expected_high=band.high,
        message=message,
        percentile=pct,
        population_n=population_n,
        one_sided=True,
    )
