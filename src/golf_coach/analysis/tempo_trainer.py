"""Build a beat sequence from the tour's own swing durations. [ADR-023]

The one thing in `analysis/` that produces something to swing *to* rather than a verdict on a
swing that happened. `contracts/tempo.py` carries the vocabulary and the argument for two
patterns; this builds them.

**Everything comes out of `golfdb_v1.json`.** Not one duration, ratio or tick count is written
here — the tick count in particular resolves to 3 on today's distributions and is still derived,
because it is an output of the medians and would move if they did. `derive_reference.py` is what
put `backswing_ms` and `downswing_ms` in that file; before it, the whole GolfDB pipeline was
deliberately frame-rate free and no row in the repo knew how long a tour backswing takes.

Refuses rather than guesses when the distributions are missing (ADR-010 §2). There is no fallback
constant to fall back to, and a metronome ticking at an invented tempo is worse than no metronome:
a golfer would practice to it.

Two scopes are built here. `build_tempo_plan` fits one swing; `build_career_tempo` at the foot fits
one *golfer*, joining their whole measured history to career mode's guard so a target can follow
their mean rather than their last swing (ADR-023 addendum, 2026-08-22). Both go through
`build_tempo_plan_for`, which is the only place an anchor is chosen and a pace is fitted — the
reason two surfaces cannot disagree about which half of which swing a target was built on.

Stdlib + contracts only (ADR-008).
"""

from __future__ import annotations

from golf_coach.analysis.baseline import build_baseline, pooled_samples
from golf_coach.analysis.benchmarks.distributions import Distribution, load_distribution
from golf_coach.analysis.measure import tempo_timings
from golf_coach.contracts.baseline import (
    BaselineClaim,
    MetricSample,
    PersonalBaseline,
    WithheldClaim,
)
from golf_coach.contracts.career import CareerCorpus
from golf_coach.contracts.swing import PhaseSegment
from golf_coach.contracts.tempo import (
    Beat,
    BeatPattern,
    BeatRole,
    CareerTempo,
    TempoAnchor,
    TempoPattern,
    TempoPlan,
    TempoSwing,
)

#: The distribution rows the plan is built from. Both are unjudged measurements — there is no band
#: for either and ADR-023 says there must not be — so this reads `distributions.py` and never
#: `resolve_range`.
_BACKSWING_METRIC = "backswing_ms"
_DOWNSWING_METRIC = "downswing_ms"


def build_tempo_plan(phases: list[PhaseSegment]) -> TempoPlan | None:
    """Both beat patterns plus the pace to play them at, or None if the reference is unavailable.

    **The target follows the golfer, and their own downswing is how.** Swing speed genuinely does
    change swing duration — LPGA against PGA, driver only and one vote per golfer, the backswing
    runs 1001 ms against 834 and the downswing 267 against 234 (ADR-023's 2026-08-20 addendum).
    What does *not* do it is club: a longer club raises head speed by lengthening the lever rather
    than by rotating faster, which is why the between-club sd is 6.9 ms and the first version of
    this function targeted the tour median for everyone.

    **Which half to fit to is ADR-023's 2026-09-02 addendum, and it is the downswing.** The
    downswing is the half a golfer *feels* — it is how hard they swung — so it is the swing's given
    rather than something to prescribe; the backswing is the half they can deliberately change, so
    it is what the plan asks for. Fitting the other way round, as this function did until M13,
    hands the golfer an instruction about the thing they experience as effort. Read the downswing,
    multiply by the tour ratio, prescribe the backswing.

    Either way the fit needs no speed at all — it is measured directly rather than inferred from a
    proxy — and there is no club-head-speed number available to key on in any case: every stored
    shot reads a smash factor below 1.0 (`analysis/shot_measure.py`).

    **The anchoring is carried entirely by `pace`, and the patterns stay at the tour reference.**
    That is what makes the golfer's pace control mean something: the slider is a multiple of the
    tour median, this function pre-sets it to the fitted value, and moving it is a plain override.
    Had the anchor been baked into the beats instead, the slider would have read 100% for every
    golfer regardless of what was fitted — a control that cannot show the decision it is overriding.

    It also leaves exactly **one** place where a pace is applied. Pre-scaling here while the page
    scaled again by its slider was a double-application waiting for the first caller to pass a
    pace; nothing did, so it never bit. The parameter is gone rather than documented.

    The scaling is exact rather than approximate: `CUES` at the recommended pace plays the golfer's
    own downswing to the millisecond, because `pace` is `anchor / downswing.p50` and that pattern's
    downswing *is* `downswing.p50`. Its top therefore lands at `anchor × tour_ratio`, which is the
    backswing being prescribed and the number `anchor_backswing_ms` carries. The ratio between the
    two is the tour's, which is the quantity the tempo checkpoint judges — so the drill teaches the
    thing being scored.

    Returns the plan with `GRID` first, as the pattern that gives a golfer something to track
    through the backswing; `CUES` is the exact-ratio alternative beside it.
    """
    timings = tempo_timings(phases)
    observed = timings.durations
    return build_tempo_plan_for(
        observed_backswing_ms=observed[0] if observed else None,
        observed_downswing_ms=observed[1] if observed else None,
    )


def build_tempo_plan_for(
    *, observed_backswing_ms: float | None, observed_downswing_ms: float | None
) -> TempoPlan | None:
    """The same plan, from two durations rather than from a segmentation.

    Split out of `build_tempo_plan` above when career mode needed a target anchored to a golfer's
    *mean* rather than to one swing's (ADR-023 addendum, 2026-08-22). The two callers differ in
    exactly one thing — where the observed durations come from — and everything after that point is
    the anchor and the two patterns, which must not exist twice: a second copy would be free to fit
    to the other half, and one of the two pages would say "matched to your own downswing" over a
    drill built on the tour median.

    Keyword-only because the two arguments are the same type in a fixed order, which is the
    boolean trap's numeric cousin (R14): `build_tempo_plan_for(384, 901)` is a plausible-looking
    call that silently builds a target with the halves swapped.
    """
    backswing = load_distribution(_BACKSWING_METRIC)
    downswing = load_distribution(_DOWNSWING_METRIC)
    if backswing is None or downswing is None:
        return None
    if backswing.p50 <= 0 or downswing.p50 <= 0:
        return None

    anchor, anchored = _anchor_downswing(downswing, observed_downswing_ms)

    # The ratio is the tour's, always — it is the anchor's *length* that follows the golfer, never
    # the shape. A golfer whose ratio was already the target would not be reading this page.
    tour_ratio = backswing.p50 / downswing.p50

    source = _source_prose(backswing, downswing, anchored)
    patterns = (
        _grid_pattern(tour_ratio, downswing.p50, source),
        _cues_pattern(backswing.p50, downswing.p50, tour_ratio, source),
    )

    # Reported, never acted on, and together they are the whole of what replaced the old guard.
    # An out-of-range downswing is not refused — under this anchor the downswing is the given and
    # never the fault — so the plan states the fact and carries the edge the golfer *may* snap to,
    # and the golfer decides. `None` when there is nothing to offer, so "is there an offer" and
    # "should the control exist" are one question rather than two that can disagree.
    in_tour_range = observed_downswing_ms is None or (
        downswing.p10 <= observed_downswing_ms <= downswing.p90
    )
    in_range_pace = None if in_tour_range else _nearest_edge_pace(downswing, anchor)

    return TempoPlan(
        patterns=patterns,
        pace=anchor / downswing.p50,
        anchored=anchored,
        anchor_downswing_ms=anchor,
        # Multiplied here rather than by each surface: it needs `tour_ratio`, and a page that knew
        # one would be a second copy of the corpus (`test_the_results_page_writes_no_tempo_number
        # _of_its_own`).
        anchor_backswing_ms=anchor * tour_ratio,
        downswing_in_tour_range=in_tour_range,
        in_range_pace=in_range_pace,
        observed_backswing_ms=observed_backswing_ms,
        observed_downswing_ms=observed_downswing_ms,
    )


def _anchor_downswing(
    downswing: Distribution, observed_downswing_ms: float | None
) -> tuple[float, bool]:
    """`(the downswing the target is built on, whether it is the golfer's own)`.

    The golfer's own downswing whenever one was measured, **with no range check at all** — and the
    absent check is the part worth reading. Its predecessor `_anchor_backswing` refused an observed
    *backswing* outside the tour p10-p90, and had a good reason: an out-of-range backswing is
    plausibly the fault itself, so a drill built on it rehearses the error at a tempo that reads
    correct.

    That reasoning does not survive the flip, and keeping the rule while swapping the metric is the
    mistake this docstring exists to prevent. Under this anchor the downswing is never the fault —
    it is the given, how hard the golfer swung, and the prescription lands on the *other* half. A
    guard here would refuse the 383.9 ms downswing on disk, hand back the tour median, and leave
    exactly the state ADR-023's 2026-09-02 addendum exists to end.

    The protection moves rather than being discarded: `downswing_in_tour_range` says so on the plan
    and `in_range_pace` offers the nearest edge, so a mis-segmented 60 ms downswing becomes a notice
    the golfer can act on instead of a substitution made for them. *"A wrong verdict is read once,
    and a wrong metronome is rehearsed"* still holds; the answer is now theirs rather than ours.

    Positivity is upstream's guarantee and is not re-checked: `tempo_timings` refuses a non-positive
    duration as `TIMING_DEGENERATE`, so a zero arriving here means a corrupt artifact, and
    `anchor_downswing_ms`'s `gt=0` failing loudly beats a second silent fallback to the median.
    """
    if observed_downswing_ms is None:
        return downswing.p50, False
    return observed_downswing_ms, True


def _nearest_edge_pace(downswing: Distribution, observed_ms: float) -> float:
    """The pace that puts the *played* downswing on the tour edge nearest the golfer's own.

    A pace and not a duration, because that is what the control it feeds takes: the page's slider is
    a multiple of the tour median and the snap button only sets it. `edge / p50` is exactly the pace
    at which every pattern's downswing — `downswing.p50` in all of them — lands on that edge.

    Called only when the observed downswing is outside p10-p90, so which edge is "nearest" is
    decided by the side it fell off rather than by comparing two distances.
    """
    edge = downswing.p10 if observed_ms < downswing.p10 else downswing.p90
    return edge / downswing.p50


def _cues_pattern(
    backswing_p50: float, downswing_p50: float, tour_ratio: float, source: str
) -> BeatPattern:
    """Three tones at the tour medians: takeaway, top, impact.

    The exact pattern. At `TempoPlan.pace` its top lands on the anchored backswing to the
    millisecond, because the pace *is* `anchor / backswing_p50` and this backswing is that p50.
    The ratio is the tour's rather than anything recomputed from the beats, so the two cannot
    disagree — and being pace-invariant, it is as true of what the golfer hears as of what is
    stored here.
    """
    backswing_ms = backswing_p50
    downswing_ms = downswing_p50
    return BeatPattern(
        mode=TempoPattern.CUES,
        beats=(
            Beat(at_ms=0.0, role=BeatRole.TAKEAWAY),
            Beat(at_ms=backswing_ms, role=BeatRole.TOP),
            Beat(at_ms=backswing_ms + downswing_ms, role=BeatRole.IMPACT),
        ),
        backswing_ms=backswing_ms,
        downswing_ms=downswing_ms,
        ratio=tour_ratio,
        source=source,
    )


def _grid_pattern(tour_ratio: float, downswing_p50: float, source: str) -> BeatPattern:
    """An even click at the tour downswing, with the backswing snapped to a whole tick count.

    The snap is what the golfer is buying: a steady pulse they can track and loop. What it costs is
    the exact ratio, and the cost is reported rather than hidden — `ratio` here is the tick count,
    which *is* this pattern's true ratio, not the tour figure it was rounded from.

    **The rounding is absorbed by the backswing, never by the downswing**, and that is deliberate.
    The downswing is the *anchor*: at `TempoPlan.pace` the tick lands on the golfer's own measured
    downswing, so rounding it would play them a downswing they did not swing — the one duration in
    the plan that is theirs rather than prescribed. The backswing is the prescription, and it spans
    700 to 1204 across the corpus, so a tick's worth of it is inside the natural spread. Concretely:
    the tick stays on the golfer's downswing and the top lands a little short of the prescribed
    backswing, rather than the top landing exactly and the click drifting off it.

    The code is unchanged from before ADR-023's 2026-09-02 addendum and the argument is inverted —
    it used to read *"the downswing is the near-invariant half and the quantity being corrected"*.
    Both arguments reach the same line, which is why the old one had to go: a reader who finds only
    it will conclude the rounding is on the wrong half and "fix" it.

    `ticks` is floored at 1 so a pathological ratio still yields a playable pattern rather than a
    zero-length backswing.
    """
    tick_ms = downswing_p50
    ticks = max(1, round(tour_ratio))

    beats = [Beat(at_ms=0.0, role=BeatRole.TAKEAWAY)]
    beats.extend(
        Beat(at_ms=index * tick_ms, role=BeatRole.SUBDIVISION) for index in range(1, ticks)
    )
    beats.append(Beat(at_ms=ticks * tick_ms, role=BeatRole.TOP))
    beats.append(Beat(at_ms=(ticks + 1) * tick_ms, role=BeatRole.IMPACT))

    return BeatPattern(
        mode=TempoPattern.GRID,
        beats=tuple(beats),
        backswing_ms=ticks * tick_ms,
        downswing_ms=tick_ms,
        ratio=float(ticks),
        source=source,
    )


def _source_prose(backswing: Distribution, downswing: Distribution, anchored: bool) -> str:
    """Where these numbers came from, in a sentence, from the rows themselves.

    Built from the distributions rather than written out, for the reason `ranges.json` carries
    provenance per row: a sentence naming a sample size is wrong the next time the corpus is
    re-derived, and nobody re-reads prose to check.

    Says which downswing the target was built on, because that is the difference between "the tour
    median" and "your own", and a golfer reading a target should know which one they are hearing.
    """
    anchor_note = (
        "set to your own downswing, at the tour ratio"
        if anchored
        else "set to the tour median downswing"
    )
    return (
        f"tour medians from GolfDB - backswing {backswing.p50:.0f} ms "
        f"(n={backswing.n}, {backswing.n_players} golfers), "
        f"downswing {downswing.p50:.0f} ms (n={downswing.n}, {downswing.n_players} golfers); "
        f"{anchor_note}"
    )


# --------------------------------------------------------------------------------------
# Career scope — one golfer's whole tempo history, and the target it earns them
# --------------------------------------------------------------------------------------

#: The three measurements a career tempo view reads, and the order the anchor prefers them in.
#: `tempo_ratio` is what defines a swing as having a readable tempo — the two halves can be absent
#: on an artifact written before they were measured, and a history that dropped those swings would
#: be shorter than the `n` printed beside it.
_RATIO_METRIC = "tempo_ratio"


def build_career_tempo(corpus: CareerCorpus) -> CareerTempo:
    """One golfer's tempo across every swing on disk, plus a metronome fitted to it. [ADR-023]

    **The measurements and the claims are separated here, not in the page.** `swings` is every
    reading, printed at any `n`; `typical_*` is `PersonalBaseline`'s guarded mean and is `None`
    until the guard lifts. That split is the whole reason this can ship useful today: `tempo_ratio`
    needs 8 samples for a `CENTER` claim (`contracts/baseline.py`, and it is the noisiest metric in
    the panel), so a view that could only show the mean would show a golfer with two swings
    nothing at all — while the two numbers they actually want to see are sitting in the artifacts.

    **The anchor degrades in one direction and says so.** Career mean if the guard allows it, else
    the most recent swing's own downswing, else the tour median. The middle rung is not a mean
    smuggled past the guard: it is one swing's measurement used as a target, which is exactly what
    the results page has done per-swing since ADR-023 shipped, and `TempoAnchor` makes it legible
    rather than leaving the page to infer it from `plan.anchored`.

    **The half selected on is the half fitted to** (M13 P4, ADR-023's 2026-09-02 addendum). Both
    rungs read a `downswing_ms`, which is what `_anchor_downswing` anchors on, so the career page
    and the results page now answer "what should I practise to" off the same measurement. It used
    to select on a backswing while the builder fitted a downswing, and a swing carrying one and not
    the other then reported `LATEST_SWING` over a plan built on the tour median.

    Which does *not* change when this unlocks. Both halves come out of one `tempo_timings` call
    (`analysis/measure.py`), so they are measured together or not at all, share an `n`, and share
    the default `CENTER` floor of 5 — neither carries a `METRIC_MINIMUM_N` override, and the 8 that
    `tempo_ratio` carries has never gated this branch.

    The reported `anchor` is still read back off the built plan rather than predicted, and that
    read-back can no longer fire: a selected downswing is always anchored, because the range guard
    that could refuse one is gone (M13 P2). It stays because it is the discipline that stops a
    second copy of the anchor rule appearing one layer up — the failure ADR-023's 2026-08-22
    addendum names — and because a view claiming "matched to your own swing" over a plan that fell
    back to the median is the disagreement the two-surfaces-one-builder rule exists to prevent.

    Builds its own baseline rather than taking one: `build_baseline` is pure arithmetic over the
    same corpus, so recomputing it costs nothing measurable and removes the one way a caller could
    hand this a baseline describing different swings.
    """
    samples = pooled_samples(corpus)
    baseline = build_baseline(corpus)

    swings = _tempo_swings(samples)
    latest = swings[-1] if swings else None

    typical_ratio = _claimed(baseline, _RATIO_METRIC)
    typical_backswing = _claimed(baseline, _BACKSWING_METRIC)
    typical_downswing = _claimed(baseline, _DOWNSWING_METRIC)

    if typical_downswing is not None:
        anchor, observed_backswing, observed_downswing = (
            TempoAnchor.CAREER_MEAN, typical_backswing, typical_downswing
        )
    elif latest is not None and latest.downswing_ms is not None:
        anchor, observed_backswing, observed_downswing = (
            TempoAnchor.LATEST_SWING, latest.backswing_ms, latest.downswing_ms
        )
    else:
        anchor, observed_backswing, observed_downswing = TempoAnchor.TOUR_MEDIAN, None, None

    plan = build_tempo_plan_for(
        observed_backswing_ms=observed_backswing, observed_downswing_ms=observed_downswing
    )
    if plan is not None and not plan.anchored:
        anchor = TempoAnchor.TOUR_MEDIAN

    return CareerTempo(
        player_id=corpus.player_id,
        swings=swings,
        latest=latest,
        typical_ratio=typical_ratio,
        typical_backswing_ms=typical_backswing,
        typical_downswing_ms=typical_downswing,
        withheld=_center_refusals(baseline),
        anchor=anchor,
        plan=plan,
    )


def _tempo_swings(samples: dict[str, list[MetricSample]]) -> tuple[TempoSwing, ...]:
    """The per-swing history, keyed on the ratio and joined to the two halves by `swing_ref`.

    A join rather than three parallel lists, because the three metrics do not always cover the same
    swings: `backswing_ms` and `downswing_ms` arrived after `tempo_ratio` did, so an artifact from
    an earlier engine carries the ratio alone. Zipping the lists positionally would pair one
    swing's ratio with another's durations the first time that happens, and every number would
    still look plausible.
    """
    halves = {
        metric: {sample.swing_ref: sample.value for sample in samples.get(metric, [])}
        for metric in (_BACKSWING_METRIC, _DOWNSWING_METRIC)
    }
    return tuple(
        TempoSwing(
            swing_ref=sample.swing_ref,
            session_id=sample.session_id,
            captured_at=sample.captured_at,
            ratio=sample.value,
            backswing_ms=halves[_BACKSWING_METRIC].get(sample.swing_ref),
            downswing_ms=halves[_DOWNSWING_METRIC].get(sample.swing_ref),
        )
        for sample in samples.get(_RATIO_METRIC, [])
        if sample.value > 0
    )


def _claimed(baseline: PersonalBaseline, metric: str) -> float | None:
    """One metric's mean, or None — which is the guard's refusal, already applied.

    No `supports(CENTER)` call: `_baseline_for` strips the statistic back out when the claim is
    withheld, so "is there a number" and "may it be shown" are the same question. Asking the
    predicate as well would be a second gate that could one day disagree with the first.
    """
    entry = baseline.metrics.get(metric)
    return entry.mean if entry is not None else None


def _center_refusals(baseline: PersonalBaseline) -> tuple[WithheldClaim, ...]:
    """Why each `typical_*` is absent, deduplicated by reason.

    Deduplicated because the three metrics gate at three floors and frequently refuse in the same
    words — `backswing_ms` and `downswing_ms` share a floor and always share an `n`, so an
    undeduplicated list prints one sentence twice and reads as two different problems. The same
    dedupe `career.html` already does across the baseline, dispersion and standing layers.

    `CENTER` only: `SPREAD` and `TREND` back no field here, and listing their refusals would tell a
    golfer they are waiting for something this view was never going to show them.

    Strictest first, and that ordering is what makes the list readable rather than merely correct.
    `WithheldClaim` carries no metric name — it never needed one on a card already titled with the
    metric — so here the refusals arrive as sentences differing only in a number, and a golfer
    reading "needs 5" above "needs 8" would take the first as the answer. The strictest is the one
    that actually gates the headline ratio.
    """
    refusals: dict[str, WithheldClaim] = {}
    for metric in (_RATIO_METRIC, _BACKSWING_METRIC, _DOWNSWING_METRIC):
        entry = baseline.metrics.get(metric)
        if entry is None:
            continue
        for claim in entry.withheld:
            if claim.claim is BaselineClaim.CENTER:
                refusals.setdefault(claim.reason, claim)
    return tuple(sorted(refusals.values(), key=lambda claim: -claim.need_n))
