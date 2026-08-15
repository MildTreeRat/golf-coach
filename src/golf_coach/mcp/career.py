"""Career mode, in the shapes an MCP tool returns. [Career mode, step 6]

The companion to `mcp/query.py`, and a separate module for one reason: that one is documented as
pure reads over the two stores and imports no `analysis` at all. These tools read a corpus and then
run it through three analysis layers — the guarded baseline, the bias/scatter discriminator, and
the tour join — so they belong beside it rather than inside it. Neither imports the MCP SDK; that
stays in `server.py`, which is what lets the interesting half be tested without standing up a
server.

## What is different about handing *this* data to a model

Every other tool in this server returns measurements. These return **claims about a golfer**, and
the guard that decides whether a claim may be made is the entire product (`contracts.baseline`).
A withheld claim ships as `None` — absent, not flagged — which is a property the shapes hold on
their own.

What the shapes cannot hold on their own is the next step. A refusal deliberately keeps its
*evidence* populated: `n`, `n_sessions`, and the per-session counts, because "you have 9 shots over
2 sessions, a trend needs 12 over 3" is what makes a refusal actionable rather than merely silent.
Handed those, a model can compute the mean the guard just refused and narrate it — the numbers are
right there, and nothing in a JSON payload stops arithmetic. So the refusal text says what is
missing, `contracts.caveats` says not to reconstruct it, and both are load-bearing.

## The window and the two sessions are the same operation

`get_shot_trends` narrows a corpus by date and `compare_sessions` narrows it by session id; both
call `storage.corpus.narrow_to` and hand the result to the same `build_baseline`. So a per-session
mean is gated by exactly the guard a pooled mean is gated by, asked of less data, and no second
threshold exists to drift away from the first.

Base install only — no MCP SDK, no vision, no OCR.
"""

from __future__ import annotations

from datetime import UTC, datetime, timedelta
from pathlib import Path

from pydantic import BaseModel, Field

from golf_coach.analysis.baseline import build_baseline
from golf_coach.analysis.comparison import build_standing
from golf_coach.analysis.dispersion import build_dispersion
from golf_coach.contracts.baseline import (
    BaselineClaim,
    Interval,
    MetricBaseline,
    WithheldClaim,
)
from golf_coach.contracts.comparison import MetricComparison
from golf_coach.contracts.dispersion import MetricDispersion
from golf_coach.contracts.golfer import Golfer, slugify
from golf_coach.mcp import query
from golf_coach.storage.corpus import narrow_to, read_corpus
from golf_coach.storage.golfer_store import GolferStore

#: The one sentence every refusal in this module ultimately resolves to. Repeated in the payload
#: rather than left to the tool description, because a description is read once on connect and a
#: payload is read at the moment the model is deciding what to say.
THE_UNBLOCK = (
    "The data does not exist yet — this needs a bay session of 20-30 swings with shots attached. "
    "Report the refusal. Do not estimate the withheld figure from the sample counts below it."
)


class Refusal(BaseModel):
    """A claim the guard declined, flattened from `WithheldClaim` for the wire."""

    claim: str = Field(description="center | spread | trend.")
    have_n: int
    need_n: int
    have_sessions: int
    need_sessions: int
    reason: str = Field(description="What is missing, in a sentence.")


class SessionPoint(BaseModel):
    """One session's contribution to one metric. Evidence, and only conditionally a claim."""

    session_id: str
    captured_at: str = Field(description="ISO 8601, the earliest arrival in that session.")
    n: int = Field(description="Distinct contributing artifacts from this session.")
    mean: float | None = Field(
        default=None,
        description=(
            "This session's mean, present ONLY when the trend guard allowed it. A per-session "
            "mean is a claim about a typical value at even smaller n than the pooled one, so it "
            "is gated where `n` is not."
        ),
    )


class MetricProfile(BaseModel):
    """One metric, everything career mode knows about it, and every refusal standing in the gaps."""

    name: str
    unit: str
    source: str
    n: int = Field(description="Distinct contributing artifacts — never swing directories.")
    n_sessions: int

    # --- what their own history says (contracts.baseline) ---------------------------------
    center: float | None = None
    center_ci_low: float | None = None
    center_ci_high: float | None = None
    median: float | None = None
    sd: float | None = None
    sd_ci_low: float | None = None
    sd_ci_high: float | None = None
    minimum: float | None = None
    maximum: float | None = None

    # --- what the shape of the miss points at (contracts.dispersion) -----------------------
    bias: str = Field(
        default="withheld",
        description=(
            "established | not_established | withheld. The center sits further from the target "
            "than measurement error explains. 'not_established' means the data cannot tell, NOT "
            "that there is no bias."
        ),
    )
    scatter: str = Field(
        default="withheld", description="Same three answers, for the shot-to-shot spread."
    )
    pattern: str | None = Field(
        default=None,
        description="Present only when BOTH findings were answerable — a pattern is a statement "
        "about the pair and half of it cannot imply the other.",
    )
    points_at: str | None = Field(
        default=None, description="What to check, phrased as a check and never as a diagnosis."
    )
    target: float | None = None
    tolerance: float | None = Field(
        default=None, description="This pipeline's own measurement error, in the metric's units."
    )
    within_session_sd: float | None = Field(
        default=None,
        description="Pooled within-session spread, when it can be formed. Beside `sd`, which "
        "pools across occasions and so mixes shot-to-shot scatter with drift between them.",
    )

    # --- where that sits in the tour population (contracts.comparison) ---------------------
    tour_standing: str = Field(
        default="withheld", description="inside | outside | straddles | withheld."
    )
    tour_reading: str | None = None
    tour_percentile: float | None = None
    tour_percentile_clamped: bool = False
    tour_band_low: float | None = None
    tour_band_high: float | None = None
    tour_population_n: int | None = None
    tour_population_players: int | None = None
    outside_by: float | None = Field(
        default=None, description="Signed distance past the nearer band edge, when outside."
    )

    sessions: list[SessionPoint] = Field(default_factory=list)
    caveats: list[str] = Field(default_factory=list)
    withheld: list[Refusal] = Field(
        default_factory=list, description="Refused for want of data. More swings fix these."
    )
    unavailable: list[str] = Field(
        default_factory=list,
        description="Refused for want of a reference. More swings do NOT fix these.",
    )


class GolferProfile(BaseModel):
    """One golfer's whole career picture, or the refusal that is currently standing in for it."""

    player_id: str
    display_name: str
    handedness: str

    distinct_swings: int = Field(
        description="Physical swings, after re-uploads of one clip were collapsed into one."
    )
    swing_dirs_seen: int = Field(
        description="Swing directories on disk for every golfer — the number a naive count "
        "reports. The gap from `distinct_swings` is re-uploads and other golfers."
    )
    sessions: int
    built_from_swings: int = Field(
        description="Swings that contributed at least one value. An unanalyzed, stale or "
        "outdated swing is a real swing of theirs that contributed nothing."
    )

    metrics: list[MetricProfile] = Field(default_factory=list)

    nothing_sayable: bool = Field(
        description="True when the guard refused every claim on every metric."
    )
    note: str = Field(default="", description="What to do about it, when nothing is sayable.")


class MetricTrend(BaseModel):
    """Whether one metric has moved for one golfer, over a window."""

    player_id: str
    metric: str
    unit: str | None = None
    window_days: int | None = Field(
        default=None, description="None means the whole history was used."
    )

    n: int = 0
    n_sessions: int = 0
    ready: bool = Field(
        default=False,
        description="True when the trend guard allowed the per-session means below to be filled "
        "in. False means `mean` is None on every point and no trend may be described.",
    )
    sessions: list[SessionPoint] = Field(default_factory=list)
    withheld: Refusal | None = None
    note: str = ""


class MetricDelta(BaseModel):
    """One metric across two sessions. Every number here is separately gated."""

    metric: str
    unit: str
    n_a: int
    n_b: int
    mean_a: float | None = None
    mean_b: float | None = None
    delta: float | None = Field(
        default=None,
        description="`mean_b - mean_a`, present only when BOTH session means survived the guard. "
        "No significance test is applied to it.",
    )
    withheld: list[Refusal] = Field(default_factory=list)


class SessionsCompared(BaseModel):
    """Two sessions of one golfer's, held against each other."""

    player_id: str
    session_a: str
    session_b: str

    mean_score_a: float | None = Field(
        default=None, description="From get_session_summary — a mean over that session's swings."
    )
    mean_score_b: float | None = None
    swings_a: int = 0
    swings_b: int = 0

    metrics: list[MetricDelta] = Field(default_factory=list)
    note: str = ""


#: Attached to every comparison, whatever the `n`. `overall_score` is a mean over the checkpoints
#: that survived (ADR-013), so two sessions can differ because a checkpoint went unmeasured in one
#: of them rather than because anything about the golfer moved — and nothing here tests a
#: difference for significance.
COMPARE_NOTE = (
    "A difference between two sessions is not by itself evidence that anything changed. Scores "
    "are means over the checkpoints that could be measured, so a session where one went unmeasured "
    "scores on fewer fundamentals than one where it did not, and no significance test is applied "
    "to any difference here. Per-metric means are absent unless each session on its own carried "
    "enough swings to support one."
)


def resolve_golfer(golfers_dir: Path, player: str) -> Golfer | None:
    """A typed name or a stored id to a registered golfer, or None.

    `slugify` folds both forms onto the same id, which is the property that makes "Aaron" and
    "aaron" one golfer rather than two histories with half the data each.
    """
    player_id = slugify(player)
    if not player_id:
        return None
    return GolferStore(golfers_dir).get(player_id)


def golfer_profile(sessions_dir: Path, golfers_dir: Path, player: str) -> GolferProfile | None:
    """Everything one golfer's own history supports saying, and every refusal. None if unknown.

    Builds all three layers over one `read_corpus`, so the baseline, the findings and the tour
    placement provably describe the same swings.
    """
    golfer = resolve_golfer(golfers_dir, player)
    if golfer is None:
        return None

    corpus = read_corpus(sessions_dir, golfer.player_id)
    baseline = build_baseline(corpus)
    dispersion = build_dispersion(corpus)
    standing = build_standing(corpus)

    metrics = [
        _profile(
            metric,
            dispersion.metrics.get(name),
            standing.metrics.get(name),
        )
        for name, metric in sorted(baseline.metrics.items())
    ]

    return GolferProfile(
        player_id=golfer.player_id,
        display_name=golfer.display_name,
        handedness=golfer.handedness.value,
        distinct_swings=corpus.distinct_swings,
        swing_dirs_seen=corpus.swing_dirs_seen,
        sessions=baseline.built_from_sessions,
        built_from_swings=baseline.built_from_swings,
        metrics=metrics,
        nothing_sayable=baseline.nothing_sayable,
        note=THE_UNBLOCK if baseline.nothing_sayable else "",
    )


def shot_trends(
    sessions_dir: Path,
    golfers_dir: Path,
    metric: str,
    player: str,
    days: int | None = None,
) -> MetricTrend | None:
    """Whether one metric has moved across sessions. None if the golfer is unknown.

    A metric this golfer has never recorded is an empty trend rather than a miss — "no samples of
    that" is a real answer, and the same posture `read_corpus` takes toward a golfer with no swings.
    """
    golfer = resolve_golfer(golfers_dir, player)
    if golfer is None:
        return None

    corpus = read_corpus(sessions_dir, golfer.player_id)
    if days is not None:
        corpus = narrow_to(corpus, since=datetime.now(UTC) - timedelta(days=days))

    found = build_baseline(corpus).metrics.get(metric)
    if found is None:
        return MetricTrend(
            player_id=golfer.player_id,
            metric=metric,
            window_days=days,
            note=(
                f"No sample of {metric!r} in this window. Either this golfer has never recorded "
                "it, or every swing carrying it is unanalyzed, stale, or was analyzed by an older "
                "engine. Call get_golfer_profile to see which metrics do have samples."
            ),
        )

    ready = found.supports(BaselineClaim.TREND)
    refusal = next(
        (r for r in found.withheld if r.claim is BaselineClaim.TREND),
        None,
    )
    return MetricTrend(
        player_id=golfer.player_id,
        metric=metric,
        unit=found.unit,
        window_days=days,
        n=found.n,
        n_sessions=found.n_sessions,
        ready=ready,
        sessions=_points(found),
        withheld=_refusal(refusal) if refusal is not None else None,
        note=(
            ""
            if ready
            else "No trend may be described. The per-session counts below are evidence for the "
            "refusal, not a series to read a direction off. " + THE_UNBLOCK
        ),
    )


def compare_sessions(
    sessions_dir: Path,
    golfers_dir: Path,
    session_a: str,
    session_b: str,
    player: str,
) -> SessionsCompared | None:
    """Two sessions of one golfer's, per metric and per score. None if the golfer is unknown.

    Each session is narrowed out of the same corpus and run through `build_baseline` on its own, so
    a per-session mean faces the identical CENTER guard a pooled mean faces. Twelve swings in one
    bay hour support a center; two do not, whichever session they sit in.
    """
    golfer = resolve_golfer(golfers_dir, player)
    if golfer is None:
        return None

    corpus = read_corpus(sessions_dir, golfer.player_id)
    left = build_baseline(narrow_to(corpus, sessions={session_a}))
    right = build_baseline(narrow_to(corpus, sessions={session_b}))

    summary_a = query.get_session_summary(sessions_dir, session_a)
    summary_b = query.get_session_summary(sessions_dir, session_b)

    metrics = [
        _delta(name, left.metrics.get(name), right.metrics.get(name))
        for name in sorted(set(left.metrics) | set(right.metrics))
    ]

    return SessionsCompared(
        player_id=golfer.player_id,
        session_a=session_a,
        session_b=session_b,
        mean_score_a=summary_a.mean_score if summary_a else None,
        mean_score_b=summary_b.mean_score if summary_b else None,
        swings_a=summary_a.analyzed_count if summary_a else 0,
        swings_b=summary_b.analyzed_count if summary_b else 0,
        metrics=metrics,
        note=COMPARE_NOTE + _sampleless(session_a, summary_a, metrics, "a")
        + _sampleless(session_b, summary_b, metrics, "b"),
    )


# --------------------------------------------------------------------------------------
# Internals
# --------------------------------------------------------------------------------------


def _profile(
    baseline: MetricBaseline,
    dispersion: MetricDispersion | None,
    standing: MetricComparison | None,
) -> MetricProfile:
    """Three layers about one metric, flattened into one view.

    The refusals are merged rather than listed per layer: a caller asking "why is there no number
    here" wants the reasons, not which module produced each. Deduplicated on the reason text, since
    a CENTER refusal legitimately reaches this point from all three.
    """
    profile = MetricProfile(
        name=baseline.name,
        unit=baseline.unit,
        source=baseline.source,
        n=baseline.n,
        n_sessions=baseline.n_sessions,
        center=baseline.mean,
        center_ci_low=_low(baseline.mean_ci),
        center_ci_high=_high(baseline.mean_ci),
        median=baseline.median,
        sd=baseline.sd,
        sd_ci_low=_low(baseline.sd_ci),
        sd_ci_high=_high(baseline.sd_ci),
        minimum=baseline.minimum,
        maximum=baseline.maximum,
        sessions=_points(baseline),
        withheld=[_refusal(r) for r in baseline.withheld],
    )

    if dispersion is not None:
        profile.bias = dispersion.bias.value
        profile.scatter = dispersion.scatter.value
        profile.pattern = dispersion.pattern.value if dispersion.pattern else None
        profile.points_at = dispersion.points_at
        profile.target = dispersion.target
        profile.tolerance = dispersion.tolerance
        profile.within_session_sd = dispersion.within_session_sd
        profile.caveats = list(dispersion.caveats)
        profile.unavailable.extend(dispersion.unavailable)

    if standing is not None:
        profile.tour_standing = standing.standing.value
        profile.tour_reading = standing.reading
        profile.tour_percentile = standing.percentile
        profile.tour_percentile_clamped = standing.percentile_clamped
        profile.tour_band_low = standing.band_low
        profile.tour_band_high = standing.band_high
        profile.tour_population_n = standing.population_n
        profile.tour_population_players = standing.population_players
        profile.outside_by = standing.outside_by
        profile.unavailable.extend(standing.unavailable)

    profile.unavailable = list(dict.fromkeys(profile.unavailable))
    return profile


def _sampleless(
    session_id: str,
    summary: query.SessionDetail | None,
    metrics: list[MetricDelta],
    side: str,
) -> str:
    """Explain a session that has scores and contributes no samples, when there is one.

    It looks like a contradiction and is not: swings are deduplicated on the face-on clip's hash,
    so a session made entirely of re-uploads of an earlier session's clip scores normally — the
    files are there and the pipeline ran on them — while contributing nothing to any `n`, because
    the same physical swing already counted once. Left unexplained, a reader sees "1 analyzed
    swing, 0 samples" and concludes the reader is broken. That is exactly what
    `CareerCorpus.excluded` exists to prevent one layer down, said here for a consumer that does
    not see the corpus.
    """
    if summary is None or summary.analyzed_count == 0 or not metrics:
        return ""
    if any((delta.n_a if side == "a" else delta.n_b) > 0 for delta in metrics):
        return ""
    return (
        f" Session {session_id} has analyzed swings but contributes no samples to any metric: its "
        "clips are byte-identical re-uploads of a clip first seen in an earlier session, so they "
        "are the same physical swing and were already counted there."
    )


def _delta(
    metric: str, left: MetricBaseline | None, right: MetricBaseline | None
) -> MetricDelta:
    """One metric across two sessions, with the delta present only when both centers survived."""
    present = left or right
    unit = present.unit if present is not None else ""
    mean_a = left.mean if left else None
    mean_b = right.mean if right else None

    withheld: list[Refusal] = []
    for side in (left, right):
        if side is None:
            continue
        withheld.extend(
            _refusal(r) for r in side.withheld if r.claim is BaselineClaim.CENTER
        )

    return MetricDelta(
        metric=metric,
        unit=unit,
        n_a=left.n if left else 0,
        n_b=right.n if right else 0,
        mean_a=mean_a,
        mean_b=mean_b,
        delta=(mean_b - mean_a) if mean_a is not None and mean_b is not None else None,
        withheld=withheld,
    )


def _points(baseline: MetricBaseline) -> list[SessionPoint]:
    return [
        SessionPoint(
            session_id=session.session_id,
            captured_at=session.captured_at.isoformat(),
            n=session.n,
            mean=session.mean,
        )
        for session in baseline.sessions
    ]


# Every career tool starts by resolving a name to a `player_id`, so every one of them can miss the
# same way — and the *hint* differs each time, because what to do next differs. Kept here rather
# than beside `query.py`'s misses for the same reason the shapes are: this module is the one that
# knows what a golfer is. Two adapters read these (ADR-020), so the prose exists once.


def missing_golfer_profile(player: str) -> query.NotFound:
    return query._missing(
        f"No golfer matching {player!r} is registered.",
        "Names are matched loosely (case and punctuation are folded), so this means nobody "
        "by that name has swings on file yet.",
    )


def missing_golfer_trend(player: str) -> query.NotFound:
    return query._missing(
        f"No golfer matching {player!r} is registered.",
        "Call get_golfer_profile with the name to see whether they have any swings on file.",
    )


def missing_golfer_comparison(player: str) -> query.NotFound:
    return query._missing(
        f"No golfer matching {player!r} is registered.",
        "compare_sessions needs a golfer, because a session's swings can belong to more than "
        "one person. Call get_golfer_profile to see who is on file.",
    )


def _refusal(withheld: WithheldClaim) -> Refusal:
    return Refusal(
        claim=withheld.claim.value,
        have_n=withheld.have_n,
        need_n=withheld.need_n,
        have_sessions=withheld.have_sessions,
        need_sessions=withheld.need_sessions,
        reason=withheld.reason,
    )


def _low(interval: Interval | None) -> float | None:
    return interval.low if interval else None


def _high(interval: Interval | None) -> float | None:
    return interval.high if interval else None
