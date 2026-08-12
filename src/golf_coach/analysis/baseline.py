"""One golfer's own history, turned into what it is allowed to say. [Career mode, step 4]

`storage.corpus` answers *how many distinct swings does this golfer have*. This answers *and what
does that support claiming* — which, at the `n` currently on disk, is nothing at all, and printing
nothing is the whole feature.

## The pooling has to agree with the counting

`CareerCorpus.metric_counts` is computed by the reader; the values behind those counts are pooled
here. If the two used different rules the printed `n` would not be the number of values averaged
under it — and the disagreement would appear only in the cases the rule exists for (a re-uploaded
clip, one shot photo across two swings), which is to say only where it is invisible and load-
bearing at once.

So neither side owns the rule: both call `CorpusSwing.artifact_key`. `build_baseline` keeps one
value per `(metric, artifact key)`, and `tests/storage/test_corpus.py` pins
`corpus.metric_counts[name] == baseline.metrics[name].n` over a corpus read off disk.

## Where the judgment lives, and where it does not

The thresholds are data (`contracts.baseline.METRIC_MINIMUM_N`), the statistics are arithmetic
(`analysis.stats`), and what is left here is the assembly. That split is deliberate: revising a
floor once a bay session lands 20-30 swings should be an edit to a table with a comment, not a
change to any logic.

Nothing here imports `benchmarks.distributions`. Comparing a personal baseline to the tour band —
"personal bands beat tour bands for feedback" — is real, and it lives one layer out in
`analysis.comparison` (step 6). Keeping it out of this module means step 4 cannot quietly become a
change to how a swing is scored (ADR-010 §2); `tests/analysis/test_comparison.py` pins it by
parsing this file's imports, since a runtime check cannot see the property.

Pure functions over contracts, base install only (ADR-008). No I/O: the corpus arrives assembled.
"""

from __future__ import annotations

from golf_coach.analysis.stats import mean_and_sd, mean_ci, percentile, sd_ci
from golf_coach.contracts.baseline import (
    BaselineClaim,
    Interval,
    MetricBaseline,
    MetricSample,
    PersonalBaseline,
    SessionSample,
    WithheldClaim,
    minimum_n,
    minimum_sessions,
)
from golf_coach.contracts.career import CareerCorpus


def pooled_samples(corpus: CareerCorpus) -> dict[str, list[MetricSample]]:
    """metric -> the deduplicated values behind its `n`, oldest first.

    Public because step 5 (dispersion as a cause discriminator) needs the values themselves rather
    than the guarded summary of them, and should not have to re-derive the dedupe rule to get at
    them. The guard governs what a `PersonalBaseline` *asserts*; it is not an attempt to keep the
    numbers away from the rest of the codebase, which would be security theatre against ourselves.
    """
    samples: dict[str, list[MetricSample]] = {}
    seen: set[tuple[str, str]] = set()

    # `corpus.swings` is already oldest-`captured_at` first, so first-wins keeps the earliest
    # reading of a repeated artifact — the same survivor the reader chose when it collapsed the
    # re-uploads, rather than a second, differently-chosen one.
    for swing in corpus.swings:
        if not swing.counts_toward_metrics():
            continue
        for measurement in swing.measurements:
            key = swing.artifact_key(measurement)
            if key is None or (measurement.name, key) in seen:
                continue
            seen.add((measurement.name, key))
            samples.setdefault(measurement.name, []).append(
                MetricSample(
                    metric=measurement.name,
                    value=measurement.value,
                    unit=measurement.unit,
                    source=measurement.source,
                    session_id=swing.session_id,
                    captured_at=swing.captured_at,
                    artifact_key=key,
                    swing_ref=swing.ref,
                )
            )
    return samples


def build_baseline(corpus: CareerCorpus) -> PersonalBaseline:
    """Everything this golfer's history supports saying about them, and every refusal.

    An empty corpus is an empty baseline rather than an error — "this golfer has no swings yet" is
    the first true answer about every golfer, and `read_corpus` takes the same posture.
    """
    samples = pooled_samples(corpus)

    contributing_swings = {
        sample.swing_ref for metric_samples in samples.values() for sample in metric_samples
    }
    contributing_sessions = {
        sample.session_id for metric_samples in samples.values() for sample in metric_samples
    }

    return PersonalBaseline(
        player_id=corpus.player_id,
        metrics={name: _baseline_for(name, samples[name]) for name in sorted(samples)},
        built_from_swings=len(contributing_swings),
        built_from_sessions=len(contributing_sessions),
    )


def refuse(metric: str, claim: BaselineClaim, n: int, n_sessions: int) -> WithheldClaim | None:
    """The guard. None means the claim may be made.

    Public because step 5 (`analysis.dispersion`) gates on the same two claims and must produce the
    same refusals. A second copy would be step 4's own lesson repeating: the dedupe rule was private
    until pooling nearly disagreed with counting, and a guard that disagrees with itself fails in
    the same invisible way — two floors drifting apart, with the looser one deciding what gets said.
    """
    need_n = minimum_n(metric, claim)
    need_sessions = minimum_sessions(claim)
    if n >= need_n and n_sessions >= need_sessions:
        return None

    shortfalls: list[str] = []
    if n < need_n:
        shortfalls.append(f"{need_n} samples (there are {n})")
    if n_sessions < need_sessions:
        shortfalls.append(
            f"{need_sessions} sessions (there are {n_sessions}) — "
            "repeated swings in one bay hour are one occasion, not several"
        )

    return WithheldClaim(
        claim=claim,
        have_n=n,
        need_n=need_n,
        have_sessions=n_sessions,
        need_sessions=need_sessions,
        reason=f"{_CLAIM_PHRASING[claim]} needs {' and '.join(shortfalls)}",
    )


#: What each claim would be asserting, in the voice a refusal has to explain itself in.
_CLAIM_PHRASING: dict[BaselineClaim, str] = {
    BaselineClaim.CENTER: "a typical value for this golfer",
    BaselineClaim.SPREAD: "a claim about how repeatable this is",
    BaselineClaim.TREND: "a claim that this has moved",
}


# --------------------------------------------------------------------------------------
# Internals
# --------------------------------------------------------------------------------------


def _baseline_for(name: str, samples: list[MetricSample]) -> MetricBaseline:
    """One metric's statistics, with every claim the guard refused stripped back out.

    Computed first and *then* removed, rather than guarded before computing. It reads backwards but
    it is the safer order: one place decides what survives, so a statistic can never reach the
    output by a path that forgot to ask.
    """
    values = [sample.value for sample in samples]
    sessions = _sessions_of(samples)
    n, n_sessions = len(values), len(sessions)

    ready: list[BaselineClaim] = []
    withheld: list[WithheldClaim] = []
    for claim in BaselineClaim:
        refusal = refuse(name, claim, n, n_sessions)
        if refusal is None:
            ready.append(claim)
        else:
            withheld.append(refusal)

    mean, sd = mean_and_sd(values)
    baseline = MetricBaseline(
        name=name,
        unit=samples[0].unit,
        source=samples[0].source,
        n=n,
        n_sessions=n_sessions,
        sessions=sessions,
        ready=ready,
        withheld=withheld,
    )

    if BaselineClaim.CENTER in ready:
        baseline.mean = mean
        baseline.mean_ci = _interval(mean_ci(values))
        baseline.median = percentile(values, 0.5)

    if BaselineClaim.SPREAD in ready:
        baseline.sd = sd
        baseline.sd_ci = _interval(sd_ci(values))
        baseline.minimum = min(values)
        baseline.maximum = max(values)

    if BaselineClaim.TREND in ready:
        means = _session_means(samples)
        for session in baseline.sessions:
            session.mean = means[session.session_id]

    return baseline


def _sessions_of(samples: list[MetricSample]) -> list[SessionSample]:
    """Per-session counts, oldest first. `mean` stays None until `TREND` opens it."""
    counts: dict[str, int] = {}
    first_seen: dict[str, MetricSample] = {}
    for sample in samples:
        counts[sample.session_id] = counts.get(sample.session_id, 0) + 1
        if sample.session_id not in first_seen:
            first_seen[sample.session_id] = sample

    ordered = sorted(first_seen.values(), key=lambda s: (s.captured_at, s.session_id))
    return [
        SessionSample(
            session_id=sample.session_id,
            captured_at=sample.captured_at,
            n=counts[sample.session_id],
        )
        for sample in ordered
    ]


def _session_means(samples: list[MetricSample]) -> dict[str, float]:
    grouped: dict[str, list[float]] = {}
    for sample in samples:
        grouped.setdefault(sample.session_id, []).append(sample.value)
    return {session_id: sum(values) / len(values) for session_id, values in grouped.items()}


def _interval(bounds: tuple[float, float] | None) -> Interval | None:
    if bounds is None:
        return None
    return Interval(low=bounds[0], high=bounds[1])
