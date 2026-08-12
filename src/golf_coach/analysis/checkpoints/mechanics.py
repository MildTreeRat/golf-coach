"""Mechanics checkpoints — pose-based, intent-independent. [M4-PoC+]

**This module judges; it no longer measures.** The quantities themselves come from
`analysis/measure.py`, and everything here is about turning one into a verdict: resolve a band,
score against it, place it in the reference population, phrase it for a golfer. The split exists
because the fused version could not measure a metric that had no band yet — and bands are derived
from populations of measurements, so nothing new could ever acquire one. See `measure.py`.

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
from golf_coach.analysis.measure import (
    ADDRESS_SAMPLE_MIN_FRAMES,
    address_sample_bounds,
    measure_finish_balance,
    measure_head_sway,
    measure_tempo_ratio,
)
from golf_coach.contracts.intent import ClubCategory, PlayerProfile
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.swing import CheckpointScore, PhaseSegment

# Kept as module attributes under their original private names: `tests/analysis/test_checkpoints.py`
# imports them from here, and the window definition is still what this module's bands were cut
# against even though the code that computes it moved.
_ADDRESS_SAMPLE_MIN_FRAMES = ADDRESS_SAMPLE_MIN_FRAMES
_address_sample_bounds = address_sample_bounds

TEMPO_CHECKPOINT = "tempo"
_TEMPO_RANGE_KEY = "tempo_ratio"

HEAD_SWAY_CHECKPOINT = "head_sway"
_HEAD_SWAY_RANGE_KEY = "head_sway_norm"

FINISH_BALANCE_CHECKPOINT = "finish_balance"
_FINISH_BALANCE_RANGE_KEY = "finish_balance_norm"


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
    """
    if pct <= 50.0:
        share, label, at_rail = 100.0 - pct, below_label, pct <= 10.0
    else:
        share, label, at_rail = pct, above_label, pct >= 90.0
    qualifier = "at least " if at_rail else ""
    return f"{label} {qualifier}{share:.0f}% of {n} tour swings"


def evaluate_tempo(
    phases: list[PhaseSegment],
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> CheckpointScore | None:
    """Score swing tempo (backswing:downswing ratio) against the benchmark band.

    Returns `None` when the phases don't yield a usable tempo or when the store has no band
    for this checkpoint — in both cases the caller simply omits a tempo score.
    """
    observed = measure_tempo_ratio(phases)
    if observed is None:
        return None

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
    observed = measure_head_sway(keypoints, phases)
    if observed is None:
        return None

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
    observed = measure_finish_balance(keypoints, phases)
    if observed is None:
        return None

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
