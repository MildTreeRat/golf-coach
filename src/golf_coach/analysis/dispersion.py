"""Mean and spread together, read as evidence about a cause. [Career mode, step 5]

`analysis.baseline` answers *where does this golfer sit and how tightly*. This answers *and what is
that shape evidence for* — which family of cause is worth investigating, given that the causes
themselves are invisible to this instrument.

## The one design decision worth knowing

**Nothing here reads a raw value to decide a finding.** Every statistic comes from the
`MetricBaseline` step 4 already guarded, so when the CENTER claim was refused there is no mean to
test a bias against — not a mean this module declines to use, a mean that does not exist in its
input. `Measurement` made a measurement structurally incapable of reading as a verdict; step 4 made
an under-powered statistic structurally absent; this keeps that property by *consuming* the guarded
shape rather than by re-deriving from `pooled_samples` and remembering to check.

The one place raw samples are read is `_within_session_sd`, which needs the per-session grouping the
baseline flattens away — and it is itself gated on SPREAD being ready, so it cannot become a route
around the seal.

## Why two findings rather than a score

A bias and a scatter are separate facts about a golfer and the pair is the informative thing: the
same 6-degree average miss means opposite things at sd 1 and at sd 9. Collapsing them into one
number would throw away exactly the contrast the milestone exists to read.

Pure functions over contracts, base install only (ADR-008). Imports no `benchmarks`: comparing a
golfer to the tour population lives one layer out in `analysis.comparison` (step 6), and keeping it
out of here means this cannot quietly become a change to how a swing is scored (ADR-010 §2).
`tests/analysis/test_comparison.py` pins that by parsing this file's imports.
"""

from __future__ import annotations

from collections.abc import Sequence

from golf_coach.analysis.baseline import build_baseline, pooled_samples
from golf_coach.analysis.stats import mean_and_sd
from golf_coach.contracts.baseline import BaselineClaim, MetricBaseline, MetricSample
from golf_coach.contracts.career import CareerCorpus
from golf_coach.contracts.dispersion import (
    PATTERN_READING,
    SCATTER_ONLY_READING,
    SESSION_DRIFT_FACTOR,
    DispersionPattern,
    Finding,
    GolferDispersion,
    MetricDispersion,
    target_for,
)


def build_dispersion(corpus: CareerCorpus) -> GolferDispersion:
    """Every metric's miss-shape for one golfer, or the refusal standing in for it.

    Builds the baseline itself rather than taking one, so the guarded statistics and the raw
    per-session samples provably describe the same corpus. Handing in a baseline built from a
    different read would be a way to pair one golfer's spread with another's sessions.
    """
    baseline = build_baseline(corpus)
    samples = pooled_samples(corpus)

    return GolferDispersion(
        player_id=corpus.player_id,
        metrics={
            name: dispersion_for(metric, samples.get(name, []))
            for name, metric in sorted(baseline.metrics.items())
        },
        built_from_swings=baseline.built_from_swings,
        built_from_sessions=baseline.built_from_sessions,
    )


def dispersion_for(
    baseline: MetricBaseline, samples: Sequence[MetricSample] = ()
) -> MetricDispersion:
    """One metric's two findings and, when both were answerable, the pattern they make.

    `samples` is optional and is used for one thing only — separating within-session scatter from
    drift between sessions. Everything that decides a finding comes from `baseline`.
    """
    declared = target_for(baseline.name)

    dispersion = MetricDispersion(
        name=baseline.name,
        unit=baseline.unit,
        source=baseline.source,
        n=baseline.n,
        n_sessions=baseline.n_sessions,
        target=declared.target if declared else None,
        tolerance=declared.tolerance if declared else None,
    )

    if declared is None:
        # No tolerance means neither question can be asked: a finding would be measured against a
        # borrowed error term, which is how a metric acquires a verdict nobody derived for it.
        dispersion.unavailable.append(
            f"{baseline.name} is not registered in METRIC_TARGETS, so there is no measurement "
            "error to judge either a bias or a spread against"
        )
        return dispersion

    _decide_bias(dispersion, baseline, declared.target, declared.tolerance)
    _decide_scatter(dispersion, baseline, declared.tolerance)

    if declared.target is None:
        dispersion.unavailable.append(f"no target for {baseline.name}: {declared.no_target_reason}")

    _carry_refusals(dispersion, baseline)
    _add_session_evidence(dispersion, baseline, samples)
    _read_the_pair(dispersion)
    return dispersion


# --------------------------------------------------------------------------------------
# Internals
# --------------------------------------------------------------------------------------


def _decide_bias(
    dispersion: MetricDispersion,
    baseline: MetricBaseline,
    target: float | None,
    tolerance: float,
) -> None:
    """Is the center displaced from the target by more than measurement error explains?

    Established only when the mean's whole 95% interval sits outside `[target ± tolerance]`. An
    interval that straddles the band is consistent with a real bias *and* with none, so it settles
    nothing — and saying so is the point, since the alternative is a point estimate one sample away
    from the other side of the line.
    """
    if target is None or baseline.mean is None or baseline.mean_ci is None:
        dispersion.bias = Finding.WITHHELD
        return

    dispersion.center = baseline.mean
    dispersion.center_ci = baseline.mean_ci
    dispersion.offset = baseline.mean - target

    low, high = baseline.mean_ci.low, baseline.mean_ci.high
    clears = low > target + tolerance or high < target - tolerance
    dispersion.bias = Finding.ESTABLISHED if clears else Finding.NOT_ESTABLISHED


def _decide_scatter(
    dispersion: MetricDispersion, baseline: MetricBaseline, tolerance: float
) -> None:
    """Is the spread larger than measurement error explains?

    The **lower** bound of the sd interval is what has to clear the tolerance, not the estimate.
    Measurement error inflates observed spread (`sd_obs^2 ~ sd_true^2 + sd_err^2`), so a sample sd
    at the tolerance is exactly what a perfectly repeatable golfer measured by this pipeline would
    produce.
    """
    if baseline.sd is None or baseline.sd_ci is None:
        dispersion.scatter = Finding.WITHHELD
        return

    dispersion.sd = baseline.sd
    dispersion.sd_ci = baseline.sd_ci
    dispersion.scatter = (
        Finding.ESTABLISHED if baseline.sd_ci.low > tolerance else Finding.NOT_ESTABLISHED
    )


def _carry_refusals(dispersion: MetricDispersion, baseline: MetricBaseline) -> None:
    """Bring across the step-4 refusals behind whichever finding is withheld for want of `n`.

    Only the two claims this step reads: a `TREND` refusal is real but is not why a bias or a
    scatter is missing here, and listing it would send someone to book a third session to answer a
    question that needs a longer one.
    """
    needed = {BaselineClaim.CENTER: dispersion.bias, BaselineClaim.SPREAD: dispersion.scatter}
    dispersion.withheld = [
        refusal
        for refusal in baseline.withheld
        if needed.get(refusal.claim) is Finding.WITHHELD
    ]


def _add_session_evidence(
    dispersion: MetricDispersion, baseline: MetricBaseline, samples: Sequence[MetricSample]
) -> None:
    """Separate shot-to-shot scatter from drift between occasions, when the data allows it.

    `sd` pools every sample the golfer has, which mixes two different quantities: how much one
    swing varies from the next, and how much the golfer has moved between sessions. The cause
    reading is about the first — "your release is inconsistent" is a claim about one bay hour — so
    a pooled spread inflated by drift would point at timing when the truth is that something
    changed in between.

    Classification stays on the pooled figure. Splitting them properly is a variance decomposition
    with its own `n` requirements, and there is no corpus to test one against; a caveat that names
    the possibility is the honest amount to say today.
    """
    if baseline.sd is None:
        # SPREAD was refused, so a within-session spread would be a spread statistic reaching the
        # output around the guard rather than through it.
        return

    within = _within_session_sd(samples)
    if within is None:
        return

    dispersion.within_session_sd = within
    if baseline.sd > within * SESSION_DRIFT_FACTOR:
        dispersion.caveats.append(
            f"Pooled spread ({baseline.sd:.3g}) runs wider than the within-session spread "
            f"({within:.3g}), so part of it is movement between sessions rather than shot-to-shot "
            "scatter. Read the per-session breakdown before calling this a timing problem."
        )


def _within_session_sd(samples: Sequence[MetricSample]) -> float | None:
    """Pooled within-session standard deviation, or None when it cannot be formed.

    Needs at least two sessions carrying at least two samples each: one session cannot show drift
    between sessions, and a session of one contributes no within-session spread to pool.
    """
    grouped: dict[str, list[float]] = {}
    for sample in samples:
        grouped.setdefault(sample.session_id, []).append(sample.value)

    usable = [values for values in grouped.values() if len(values) >= 2]
    if len(usable) < 2:
        return None

    squares = 0.0
    degrees = 0
    for values in usable:
        _, sd = mean_and_sd(values)
        if sd is None:
            continue
        squares += (len(values) - 1) * sd**2
        degrees += len(values) - 1
    if degrees == 0:
        return None
    return (squares / degrees) ** 0.5


def _read_the_pair(dispersion: MetricDispersion) -> None:
    """Turn the two findings into a pattern, and the pattern into something to check.

    A pattern needs *both* findings answered. `SCATTERED` asserts that a bias was looked for and
    not found, which is false when no target existed to look against — so a metric with scatter and
    no target gets the reading without the pattern, rather than a pattern that overstates it.
    """
    bias, scatter = dispersion.bias, dispersion.scatter

    if bias is Finding.WITHHELD or scatter is Finding.WITHHELD:
        if scatter is Finding.ESTABLISHED:
            dispersion.points_at = SCATTER_ONLY_READING
        return

    biased = bias is Finding.ESTABLISHED
    scattered = scatter is Finding.ESTABLISHED
    if biased and scattered:
        pattern = DispersionPattern.BIASED_AND_SCATTERED
    elif biased:
        pattern = DispersionPattern.BIASED
    elif scattered:
        pattern = DispersionPattern.SCATTERED
    else:
        pattern = DispersionPattern.NOTHING_ESTABLISHED

    dispersion.pattern = pattern
    dispersion.points_at = PATTERN_READING[pattern]
