"""Mechanics checkpoints — pose-based, intent-independent. [M4-PoC+]

**This module judges; it no longer measures.** The quantities themselves come from
`analysis/measure.py`, and everything here is about turning one into a verdict: resolve a band,
score against it, place it in the reference population, phrase it for a golfer. The split exists
because the fused version could not measure a metric that had no band yet — and bands are derived
from populations of measurements, so nothing new could ever acquire one. See `measure.py`.

Every checkpoint here is measured from **face-on 2D pose** (the canonical pose-camera placement,
ADR-003 addendum) — deliberately the ones this single view reads well. Which of them ship, and in
what order, is `contracts.checkpoints.CHECKPOINT_REGISTRY`; this list is what each one *means*:

- **tempo** — backswing:downswing time ratio, from phase timings.
- **head_sway** — lateral (`x`) head travel from address to impact, in shoulder-widths.
- **finish_balance** — how still the body settles through the follow-through, in shoulder-widths.
- **hip_sway** — lateral (`x`) hip travel from address to impact, in shoulder-widths. [2026-08-12]
- **hip_shift_at_top** — lateral hip travel from address to the top, in shoulder-widths.
  [2026-08-12]
- **head_stays_back** — how much head-behind-hips separation the swing created, address to impact,
  in shoulder-widths. **Signed**, so it needs `Handedness` and returns `None` without it — the only
  checkpoint here that depends on who swung rather than only on what they did. [2026-08-13]

**Every checkpoint here differences one landmark across time, and that is load-bearing rather than
incidental.** `head_stays_back` was very nearly the exception: the obvious form of "staying behind
the ball" is the head-hip offset *at impact*, a static difference between two body parts at one
instant, and `check_metric_transfer.py` showed what that costs. Our own bay clips disagree with the
reference corpus by 0.32 shoulder-widths **at address** — square body, no swing yet, ~4x the
metric's error and 55% of the whole gap at impact — because a camera off-square to the target line
converts head/hip *depth* into apparent horizontal offset, and shoulder-width normalization does not
remove it. Differencing address from impact cancels the static term. The rule worth carrying: a band
cut from broadcast footage transfers to a phone only for quantities where the camera cancels.

The last two were measured for a milestone before they were judged (M6.5), which is the order this
module's split exists to allow: a band is cut from a population of measurements, so the measuring
has to come first. `tune_spatial_metric.py` cleared both — spread/error 7.1 and 3.6 — and the
bands come from the same 458 face-on GolfDB swings the other two spatial bands do.

**A band edge is asserted only where it clears the instrument.** That rule is why the two new
checkpoints have different *shapes*, and it is worth stating because the obvious default gets both
wrong. `derive_reference.py` recommends a one-sided `[0, p90]` band for anything named `_norm`,
which encodes "less is better" — established for head sway and finish drift, and **not** established
for either hip metric, since some lateral hip travel is the weight shift a swing needs (the same
finding that denied both metrics a bias target in career mode step 5). So:

- `hip_sway_norm` is **two-sided** `[0.14, 0.50]`. The tour p10 is 0.14, meaning 90% of tour swings
  show *more* hip travel than that — not the shape of a quantity to minimise — and 0.14 sits 2.8x
  the metric's own noise+boundary error (0.050) above zero, so "too little" is a distinction this
  pipeline can actually make. That 2.8x is a claim about *resolution* and M8's player-clustered
  bootstrap leaves it untouched; what the bootstrap widened is where the tour p10 *sits* (95%
  interval reaching 0.0801). Weighed 2026-08-18 and the edge kept — see ADR-010's addendum.
- `hip_shift_at_top_norm` is **one-sided** `[0, 0.21]`, and not because less is better. Its p10 is
  0.015 against an error floor of 0.053, so a lower edge there would separate golfers the pipeline
  cannot tell apart. Only overshoot is judged — the half the instrument resolves.

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
    measure_head_hip_gain,
    measure_head_sway,
    measure_hip_shift_at_top,
    measure_hip_sway,
    measure_tempo_ratio,
)
from golf_coach.contracts.checkpoints import spec_for
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.intent import ClubCategory, PlayerProfile
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.swing import CheckpointScore, PhaseSegment

# Kept as module attributes under their original private names: `tests/analysis/test_checkpoints.py`
# imports them from here, and the window definition is still what this module's bands were cut
# against even though the code that computes it moved.
_ADDRESS_SAMPLE_MIN_FRAMES = ADDRESS_SAMPLE_MIN_FRAMES
_address_sample_bounds = address_sample_bounds

# Name, band key and band shape all come off the spec rather than being retyped here.
# `contracts.checkpoints` owns the pairing because `caveats.py` has to build prose out of it and
# cannot import this package (ADR-008); keeping a second copy is what let the shipped caveat text
# claim five checkpoints while six ran. The names stay module attributes because they are public
# vocabulary — `SwingResult.unscored`, `feedback/rules.py` and the tests all match on them.
_TEMPO = spec_for("tempo")
TEMPO_CHECKPOINT = _TEMPO.name
_TEMPO_RANGE_KEY = _TEMPO.metric

_HEAD_SWAY = spec_for("head_sway")
HEAD_SWAY_CHECKPOINT = _HEAD_SWAY.name
_HEAD_SWAY_RANGE_KEY = _HEAD_SWAY.metric

_FINISH_BALANCE = spec_for("finish_balance")
FINISH_BALANCE_CHECKPOINT = _FINISH_BALANCE.name
_FINISH_BALANCE_RANGE_KEY = _FINISH_BALANCE.metric

_HIP_SWAY = spec_for("hip_sway")
HIP_SWAY_CHECKPOINT = _HIP_SWAY.name
_HIP_SWAY_RANGE_KEY = _HIP_SWAY.metric

_HIP_SHIFT_AT_TOP = spec_for("hip_shift_at_top")
HIP_SHIFT_AT_TOP_CHECKPOINT = _HIP_SHIFT_AT_TOP.name
_HIP_SHIFT_AT_TOP_RANGE_KEY = _HIP_SHIFT_AT_TOP.metric

_HEAD_STAYS_BACK = spec_for("head_stays_back")
HEAD_STAYS_BACK_CHECKPOINT = _HEAD_STAYS_BACK.name
_HEAD_STAYS_BACK_RANGE_KEY = _HEAD_STAYS_BACK.metric


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
        one_sided=_TEMPO.one_sided,
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
        one_sided=_HEAD_SWAY.one_sided,
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
        one_sided=_FINISH_BALANCE.one_sided,
    )


def evaluate_hip_sway(
    keypoints: list[FrameKeypoints],
    phases: list[PhaseSegment],
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> CheckpointScore | None:
    """Score lateral hip travel from address to impact — the **two-sided** checkpoint.

    The structural twin of `evaluate_head_sway` measuring a different thing: head sway asks whether
    the golfer stayed centred, hip sway asks how much the lower body moved laterally into the shot.
    Neither is visible from the other — a head that stays put over hips that slide is a lateral
    slide, while head and hips moving together is a body that swayed.

    **Two-sided, and that is the substance of this checkpoint rather than a detail.** Every other
    spatial band here is `[0, p90]`, which asserts that less is better. That is not established for
    hip travel: some of it is the weight shift a swing needs, and the tour population shows it —
    the p10 is 0.14, so 90% of tour swings move the hips *further* than that. A `[0, p90]` band
    would score a golfer who barely moves their lower body as perfect. The lower edge is safe to
    assert because it clears the instrument by 2.8x (see the module docstring) — a resolution the
    player-clustered interval on the tour p10 does not narrow (ADR-010 addendum 2026-08-18).

    Both endpoints are means over a window, so zero-mean landmark jitter suppresses by √N. Returns
    `None` if the phases/landmarks are unusable or the store has no band.
    """
    observed = measure_hip_sway(keypoints, phases)
    if observed is None:
        return None

    band = resolve_range(_HIP_SWAY_RANGE_KEY, club, profile)
    if band is None:
        return None

    score = _score_within_range(observed, band.low, band.high)
    passed = band.low <= observed <= band.high
    target = f"tour range {band.low:g}-{band.high:g}"
    if passed:
        message = (
            f"Good lower-body movement - {observed:.2f} shoulder-widths of lateral hip travel "
            f"to impact (inside the {target})."
        )
    elif observed < band.low:
        # Deliberately phrased as a fact about the population rather than as a cause. This
        # pipeline measures lateral hip position; it does not see weight, pressure or rotation,
        # and the standing caveats forbid inferring them.
        message = (
            f"Little hip movement - {observed:.2f} shoulder-widths of lateral hip travel to "
            f"impact, under the {target}. Almost every tour swing moves the hips further into "
            "the shot than this."
        )
    else:
        message = (
            f"Hip slide - {observed:.2f} shoulder-widths of lateral hip travel to impact, past "
            f"the {target}. The hips are travelling sideways more than tour swings do."
        )

    pct, population_n = _population_placement(_HIP_SWAY_RANGE_KEY, observed)
    if pct is not None and population_n is not None:
        clause = _placement_clause(
            pct, population_n, "less hip travel than", "more hip travel than"
        )
        message += f" That is {clause}."

    return CheckpointScore(
        name=HIP_SWAY_CHECKPOINT,
        score=score,
        passed=passed,
        observed=round(observed, 2),
        expected_low=band.low,
        expected_high=band.high,
        message=message,
        percentile=pct,
        population_n=population_n,
        one_sided=_HIP_SWAY.one_sided,
    )


def evaluate_hip_shift_at_top(
    keypoints: list[FrameKeypoints],
    phases: list[PhaseSegment],
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> CheckpointScore | None:
    """Score lateral hip travel from address to the top of the backswing.

    **One-sided, for a reason that is not "less is better".** That claim is no better established
    here than for `evaluate_hip_sway`. The lower edge is omitted because it is *unmeasurable*: the
    tour p10 is 0.015 against a noise+boundary error of 0.053, so a band edge there would separate
    golfers this pipeline cannot tell apart. Judging only overshoot judges only the half the
    instrument resolves — a distinction worth keeping, because a reader who assumes the usual
    `[0, p90]` reasoning will draw the usual conclusion from it.

    **Magnitude only, so this says nothing about direction.** Sign would separate a slide away from
    the target from a reverse pivot toward it, and the sign is camera-relative while handedness is
    not resolved on the analysis path (see `measure_head_hip_offset_impact`). The message therefore
    describes how far the hips travelled, never which way.

    This reads a *detected* instant (the top, median 2 frames of error) but measures a spatial
    quantity averaged over the transition window, so it inherits none of the single-frame fragility
    that sank the arm-parallel candidates in M5-FB. Returns `None` if the phases/landmarks are
    unusable or the store has no band.
    """
    observed = measure_hip_shift_at_top(keypoints, phases)
    if observed is None:
        return None

    band = resolve_range(_HIP_SHIFT_AT_TOP_RANGE_KEY, club, profile)
    if band is None:
        return None

    score = _score_within_range(observed, band.low, band.high)
    passed = band.low <= observed <= band.high
    if passed:
        message = (
            f"Steady hips going back - {observed:.2f} shoulder-widths of lateral hip travel to "
            "the top (centered over the ball at the top)."
        )
    else:
        message = (
            f"Hip slide going back - {observed:.2f} shoulder-widths of lateral hip travel to the "
            f"top. The hips are travelling sideways during the backswing rather than staying "
            f"centered (aim under {band.high})."
        )

    pct, population_n = _population_placement(_HIP_SHIFT_AT_TOP_RANGE_KEY, observed)
    if pct is not None and population_n is not None:
        clause = _placement_clause(
            pct, population_n, "less hip slide going back than", "more hip slide going back than"
        )
        message += f" That is {clause}."

    return CheckpointScore(
        name=HIP_SHIFT_AT_TOP_CHECKPOINT,
        score=score,
        passed=passed,
        observed=round(observed, 2),
        expected_low=band.low,
        expected_high=band.high,
        message=message,
        percentile=pct,
        population_n=population_n,
        one_sided=_HIP_SHIFT_AT_TOP.one_sided,
    )


def evaluate_head_stays_back(
    keypoints: list[FrameKeypoints],
    phases: list[PhaseSegment],
    handedness: Handedness | None = None,
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> CheckpointScore | None:
    """Score how much rearward head-hip separation the swing created, address to impact.

    "Staying behind the ball", and the first checkpoint here whose *sign* carries meaning — which is
    why it is also the first that cannot be scored without knowing who swung.

    **Returns `None` when `handedness` is None, and that is the feature.** A face-on camera sees a
    left-handed swing mirrored, so the same body position lands on the opposite sign. Defaulting to
    right-handed would read a left-handed golfer's perfectly ordinary impact position as a gross
    fault — silently, and in the direction nothing downstream can detect. An unscored checkpoint is
    reported by name in `SwingResult.unscored`; a wrongly scored one is not reported at all. Same
    posture `api.app` takes when it refuses to invent a handedness for a golfer nobody stated one
    for (`contracts.golfer` records why the field is captured at all).

    **Why the delta rather than the offset at impact.** The absolute head-hip offset is the one pose
    quantity in this module that is a static difference between two body parts at a single instant;
    every other one differences a single landmark across time, which is what makes a camera-geometry
    bias cancel. `check_metric_transfer.py` measured the difference that makes: our bay clips
    disagree with the reference corpus by **0.32 shoulder-widths at address**, where the body is
    square and no swing has happened yet — 55% of the whole gap at impact, and ~4x this metric's own
    error. Scoring the absolute would have billed that to the golfer, and it would have become the
    worst checkpoint on every swing on disk. Differencing the two instants under one shared ruler
    removes the static term and leaves what the swing actually did. See `measure_head_hip_gain`.

    **Two-sided**, and neither edge is "less is better". Too little separation is the head drifting
    forward with the hips; too much is hanging back behind the ball through impact. Both edges clear
    the instrument at 3.4x the 0.080 noise+boundary error, which is what ADR-010's 2026-08-12
    addendum asks before an edge is asserted.
    """
    raw = measure_head_hip_gain(keypoints, phases)
    if raw is None or handedness is None:
        return None

    # Into the right-handed camera frame the band is cut in. The corpus was folded the same way
    # (derive_reference.py), so `observed`, the band and the stored distribution all share one
    # convention — the percentile below would otherwise read a mirrored value against an
    # unmirrored population and quietly return a plausible, wrong rank.
    observed = raw if handedness is Handedness.RIGHT else -raw

    band = resolve_range(_HEAD_STAYS_BACK_RANGE_KEY, club, profile)
    if band is None:
        return None

    score = _score_within_range(observed, band.low, band.high)
    passed = band.low <= observed <= band.high
    # Phrased as a magnitude in the golfer's own terms. The stored number is negative because the
    # band lives in a camera frame, and "-0.16 shoulder-widths of separation" is not a sentence.
    travel = abs(observed)
    if passed:
        message = (
            f"Head stayed behind the ball - {travel:.2f} shoulder-widths of separation opened up "
            "between head and hips through impact (inside the tour range)."
        )
    elif observed > band.high:
        message = (
            f"Head drifting forward - only {travel:.2f} shoulder-widths of separation between "
            "head and hips by impact. The head is travelling toward the target with the hips "
            f"rather than staying back (tour swings open up {abs(band.high):g}-"
            f"{abs(band.low):g})."
        )
    else:
        message = (
            f"Hanging back - {travel:.2f} shoulder-widths of separation between head and hips by "
            f"impact, more than tour swings show (they open up {abs(band.high):g}-"
            f"{abs(band.low):g}). The upper body is staying behind the ball through impact rather "
            "than moving through the shot."
        )

    pct, population_n = _population_placement(_HEAD_STAYS_BACK_RANGE_KEY, observed)
    if pct is not None and population_n is not None:
        # The population is stored in the same negative frame, so a *low* percentile is the
        # hanging-back end and a *high* one is the drifting-forward end. Labels follow the frame,
        # not the prose above.
        clause = _placement_clause(
            pct,
            population_n,
            "more head-behind separation than",
            "less head-behind separation than",
        )
        message += f" That is {clause}."

    return CheckpointScore(
        name=HEAD_STAYS_BACK_CHECKPOINT,
        score=score,
        passed=passed,
        observed=round(observed, 2),
        expected_low=band.low,
        expected_high=band.high,
        message=message,
        percentile=pct,
        population_n=population_n,
        one_sided=_HEAD_STAYS_BACK.one_sided,
    )
