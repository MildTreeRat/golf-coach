"""A personal center, placed in the tour population. [Career mode, step 6]

`analysis.baseline` answers *where does this golfer sit*. `analysis.dispersion` answers *what is
that shape evidence for*. This answers the third question, and the one everything else in the repo
was already able to ask about a single swing: *and how does that compare to the tour*.

## Why this is a separate module and not three lines in `baseline.py`

Because of what it imports. `analysis.baseline` and `analysis.dispersion` both state in their
docstrings that they pull in no `benchmarks`, and that is not stylistic — it is what stops a
personal statistic from becoming a change to how a swing is scored (ADR-010 §2). The join has to
import `benchmarks.distributions` by definition, so it lands in its own module: the boundary moves
out by one layer instead of dissolving, and the two guarded modules keep the property a test can
still pin.

## It reads the guarded baseline, never the samples

Same posture as `analysis.dispersion`: when step 4 refused `CENTER` there is no mean *in the input*
to place against a population. Not a mean this module declines to use — a mean that does not exist
in what it was handed. That is how "withheld means absent" survives being consumed one more time.

## The stratum is the one the bands were cut from

`load_distribution(metric)` with its `("all", "all", "all")` defaults, exactly as
`checkpoints.mechanics._population_placement` does, and for the reason recorded there:
`tempo_ratio`'s face-on stratum has p90 5.00 against the all-view 4.71, so drawing a placement from
a narrower stratum than the band would let one number read "inside the band" and "past the 90th
percentile" at once. Moving to per-club strata is a later change that has to move the band and the
placement together.

Pure functions over contracts, base install only (ADR-008). No I/O beyond the packaged reference
data: the corpus arrives assembled.
"""

from __future__ import annotations

from golf_coach.analysis.baseline import build_baseline
from golf_coach.analysis.benchmarks import load_distribution
from golf_coach.contracts.baseline import BaselineClaim, Interval, MetricBaseline
from golf_coach.contracts.career import CareerCorpus
from golf_coach.contracts.comparison import (
    SPREAD_NOT_COMPARABLE,
    STANDING_READING,
    TOUR_COMPARISON_BLOCKED,
    GolferStanding,
    MetricComparison,
    Standing,
    no_population_reason,
)


def build_standing(corpus: CareerCorpus) -> GolferStanding:
    """Every metric's placement in the tour population for one golfer, or the refusal for it.

    Builds the baseline itself rather than taking one, for the reason `build_dispersion` does:
    handing in a baseline read from a different corpus would be a way to place one golfer's center
    in a population using another golfer's `n`.
    """
    baseline = build_baseline(corpus)

    return GolferStanding(
        player_id=corpus.player_id,
        metrics={
            name: comparison_for(metric) for name, metric in sorted(baseline.metrics.items())
        },
        built_from_swings=baseline.built_from_swings,
        built_from_sessions=baseline.built_from_sessions,
    )


def comparison_for(baseline: MetricBaseline) -> MetricComparison:
    """One metric's placement, or the refusal standing in for it.

    Two kinds of refusal and they return by different routes. A metric with no usable population
    returns early carrying only that, because no amount of swinging changes it and printing "needs
    5 samples" beside it would send someone to the bay to answer a question that will still be
    unanswerable. A metric with a population but no center carries the step-4 refusal instead,
    which *is* actionable.
    """
    comparison = MetricComparison(
        name=baseline.name,
        unit=baseline.unit,
        source=baseline.source,
        n=baseline.n,
        n_sessions=baseline.n_sessions,
    )

    blocked = TOUR_COMPARISON_BLOCKED.get(baseline.name)
    if blocked is not None:
        comparison.unavailable.append(
            f"{baseline.name} has a stored distribution and may not be placed in it: {blocked}"
        )
        return comparison

    distribution = load_distribution(baseline.name)
    if distribution is None:
        comparison.unavailable.append(no_population_reason(baseline.name, baseline.source))
        return comparison

    comparison.band_low = distribution.p10
    comparison.band_high = distribution.p90
    comparison.population_n = distribution.n
    comparison.population_players = distribution.n_players

    # Said only once the golfer actually has a spread. Before that the question is not askable and
    # the `withheld` list already explains why; after it, the absence of an obvious-looking
    # `sd` vs `sd` line is the thing that needs a reason attached.
    if baseline.sd is not None:
        comparison.unavailable.append(
            f"{baseline.name}: the spread is not placed against the tour spread — "
            f"{SPREAD_NOT_COMPARABLE}"
        )

    if baseline.mean is None or baseline.mean_ci is None:
        comparison.withheld = [
            refusal for refusal in baseline.withheld if refusal.claim is BaselineClaim.CENTER
        ]
        return comparison

    comparison.center = baseline.mean
    comparison.center_ci = baseline.mean_ci

    placement = distribution.percentile_of(baseline.mean)
    comparison.percentile = round(placement, 1)
    # `percentile_of` clamps at the stored quantiles, so a rail value is a floor on how extreme the
    # center is rather than its rank. Flagged rather than inferred by the reader: `contracts.swing`
    # records that a bare 90 has been misread as a rank before.
    comparison.percentile_clamped = placement <= 10.0 or placement >= 90.0

    comparison.standing = _place(baseline.mean_ci, distribution.p10, distribution.p90)
    comparison.reading = STANDING_READING[comparison.standing]
    if comparison.standing is Standing.OUTSIDE:
        # Measured from the center rather than from the interval: the interval decided *whether*
        # this is outside, and the size of the gap is a statement about the golfer's typical value.
        comparison.outside_by = (
            baseline.mean - distribution.p90
            if baseline.mean > distribution.p90
            else baseline.mean - distribution.p10
        )
    return comparison


# --------------------------------------------------------------------------------------
# Internals
# --------------------------------------------------------------------------------------


def _place(center_ci: Interval, band_low: float, band_high: float) -> Standing:
    """Where the *interval* sits relative to `[p10, p90]`.

    Read off the interval and never the point estimate, which is the same rule
    `dispersion._decide_bias` follows. A center sitting a hair past p90 with an interval that
    crosses it has not been shown to be outside anything, and the difference between saying so and
    reporting `OUTSIDE` is the difference between a placement that survives the next swing and one
    that flips on it.
    """
    if center_ci.low >= band_low and center_ci.high <= band_high:
        return Standing.INSIDE
    if center_ci.high < band_low or center_ci.low > band_high:
        return Standing.OUTSIDE
    return Standing.STRADDLES
