"""What one golfer's own history says about them — and what it is not yet allowed to say.
[Career mode, step 4]

`golfdb_v1.json` says what good looks like across 122 tour players. This says what *normal* looks
like for the person actually swinging: their typical value, their spread, their movement across
sessions. It is the other half of the scoring model, and the half that makes a fix testable —
baseline the golfer, change one thing, measure whether it moved.

**The guard is the product here, not the statistics.** Computing a mean and a standard deviation
over a list of floats is not the hard part and never was; career mode was deferred for one reason,
recorded in ROADMAP: *"building it now would produce a confident-looking trend line over three
points."* `storage/corpus.py` stopped `n` being inflated by re-uploads. This module stops a
correct `n` of 2 from being rendered as a baseline anyway.

## Withheld means absent, not flagged

Every statistic here is gated behind a claim, and when a claim is withheld **the field is `None`**.
Not populated-with-a-warning, not shipped beside a `ready: false` a caller might forget to read —
absent, so there is no number to render.

This is `Measurement`'s principle applied one level up. That contract carries no `score` and no
`passed` so that a measurement is *structurally* incapable of reading as a verdict however it
travels; the same reasoning says a baseline built from two swings must be structurally incapable of
reading as a baseline. A consumer that has to remember to check a flag is a consumer that will
eventually forget, and the thing it renders will be a confident sentence about a golfer's tendency
built out of two numbers.

What is *always* populated is the evidence: `n`, `n_sessions`, and the per-session breakdown. Those
are facts about how much data exists, not claims about the golfer, and they are exactly what a
refusal needs in order to be actionable — "you have 9 shots over 2 sessions, a trend needs 12 over
3" tells someone to book another bay hour, where a bare `None` tells them nothing.

## Three claims, not one verdict

The claims have genuinely different appetites for `n`, so a single per-metric threshold would
either block a defensible mean or ship a spread the data cannot support:

- `CENTER` — "your typical face-to-path is +9°". Needs the fewest samples.
- `SPREAD` — "your face angle is repeatable". Needs more: the sample standard deviation is a
  noisier estimator than the sample mean, with a relative error of `1 / sqrt(2(n-1))` — 71% at
  n=2 and still 24% at n=10.
- `TREND` — "it has moved since last month". Needs the most, **and needs repeated occasions
  rather than repeated swings**, which is why `n_sessions` is gated separately.

Stdlib + pydantic only (ADR-008). Produced by `analysis.baseline.build_baseline` from the
`CareerCorpus` that `storage.corpus` reads; `storage` and `analysis` meet here and never import
each other.
"""

from __future__ import annotations

from datetime import datetime
from enum import StrEnum

from pydantic import BaseModel, Field


class BaselineClaim(StrEnum):
    """The three things a personal history can be asked to assert, each gated separately."""

    #: Where this golfer's values sit — the mean, its interval, and the median.
    CENTER = "center"
    #: How tightly they cluster. The claim career mode exists for: a tight spread points at a
    #: *static* cause (grip, setup — checkable before you swing), a wide one at timing/release,
    #: and those have completely different fixes. Also the claim most easily faked by too small
    #: an `n`, since repeating one measurement drives the variance toward zero.
    SPREAD = "spread"
    #: Whether the center has moved across sessions.
    TREND = "trend"


#: Default minimum samples per claim, and the criterion behind each.
#:
#: **These are judgment, and they are documented as judgment.** The obvious place to derive them
#: from is M6.5's spread/error ratios (`scripts/golfdb/tune_spatial_metric.py`: finish_balance 8.2
#: down to tempo 2.4), and that would be wrong. That ratio is *population* spread over *instrument*
#: error, while what binds a personal baseline is the golfer's own shot-to-shot variability —
#: unmeasured, and much larger than the instrument's. Deriving from tempo's r = 2.4 yields a
#: usefully-resolved personal mean at n ≈ 3, which is absurd on its face.
#:
#: What makes judgment safe here is the interval. A floor set too low shows up as a visibly wide
#: confidence interval rather than as a confident wrong number, so the failure mode of getting
#: these wrong is "obviously uncertain", not "quietly misleading".
#:
#: The stated criterion is the sample sd's own relative error, `1 / sqrt(2(n-1))`:
#: 71% at n=2, 35% at n=5, 24% at n=10, 19% at n=15, 14% at n=25.
#:
#: - `CENTER` 5 — SEM is then ~0.45 sd, so the mean is resolved to about half a personal spread,
#:   and t(4) = 2.78 keeps the reported interval honestly wide.
#: - `SPREAD` 10 — sd known to ~24%: enough to tell a tight spread from a wide one (a 2x
#:   difference), not enough to rank two similar ones against each other.
#: - `TREND` 12 — with the session floor below doing the real work.
DEFAULT_MINIMUM_N: dict[BaselineClaim, int] = {
    BaselineClaim.CENTER: 5,
    BaselineClaim.SPREAD: 10,
    BaselineClaim.TREND: 12,
}

#: Minimum **distinct sessions** per claim. `TREND` is 3 because a trend is a claim about repeated
#: occasions, and twelve swings hit in one bay hour are one occasion however many of them there
#: are. The same reasoning `Distribution.n_players` encodes for the tour bands — 200 clips from 6
#: players is not a sample of 200 — pointed at the axis a personal corpus actually varies on.
#:
#: Two sessions can only ever draw a line; three is the fewest that can disagree with one.
MINIMUM_SESSIONS: dict[BaselineClaim, int] = {
    BaselineClaim.CENTER: 1,
    BaselineClaim.SPREAD: 1,
    BaselineClaim.TREND: 3,
}

#: Per-metric overrides, for the metrics with recorded reasons to be noisier than the default.
#:
#: - `tempo_ratio` — the noisiest instrument in the panel (spread/error 2.4, against 8.2 for
#:   `finish_balance`) and the only metric whose measurement depends on the **address** instant,
#:   which carries a median error of 7 frames with 40% of clips over 10 (M4-REF). It also goes
#:   missing outright on ~14% of clips (ADR-013), so its `n` grows more slowly than every other
#:   metric's from the same swings.
#: - `hip_shift_at_top_norm` — spread/error 3.6, the second-noisiest.
#:
#: **Every launch-monitor metric takes the defaults**, and that is the least-informed row in this
#: table: there is no instrument-error evidence for the OCR path at all — the estimator-disagreement
#: trick `tune_spatial_metric.py` uses has no analogue for a photographed screen. They are the first
#: floors a real bay session should revise. M9 P8 added two distances under that same silence and
#: deliberately did not override them either: the defaults are not known to be wrong for a printed
#: carry, and a floor moved without evidence is a refusal nobody derived.
METRIC_MINIMUM_N: dict[str, dict[BaselineClaim, int]] = {
    "tempo_ratio": {
        BaselineClaim.CENTER: 8,
        BaselineClaim.SPREAD: 15,
        BaselineClaim.TREND: 18,
    },
    "hip_shift_at_top_norm": {
        BaselineClaim.CENTER: 6,
        BaselineClaim.SPREAD: 12,
        BaselineClaim.TREND: 14,
    },
}


def minimum_n(metric: str, claim: BaselineClaim) -> int:
    """Samples this metric needs before it may make this claim."""
    return METRIC_MINIMUM_N.get(metric, {}).get(claim, DEFAULT_MINIMUM_N[claim])


def minimum_sessions(claim: BaselineClaim) -> int:
    """Distinct sessions this claim needs, regardless of metric."""
    return MINIMUM_SESSIONS[claim]


class Interval(BaseModel):
    """A confidence interval. One shape for both the mean's and the standard deviation's.

    Carried rather than computed by the consumer because a point estimate without its uncertainty
    reads as a fact, and the whole difference between a baseline over 6 swings and one over 60 is
    the width of this.
    """

    low: float
    high: float
    confidence: float = Field(default=0.95, gt=0.0, lt=1.0)

    @property
    def width(self) -> float:
        return self.high - self.low


class WithheldClaim(BaseModel):
    """A claim the guard refused, and everything needed to know when it will be allowed.

    The `ExcludedSwing` analogue, for the same reason: a claim that is simply missing is
    indistinguishable from a claim nobody thought to make, and from a bug in the guard.
    """

    claim: BaselineClaim
    have_n: int
    need_n: int
    have_sessions: int
    need_sessions: int = Field(
        default=1, description="1 for the claims that do not gate on sessions at all."
    )
    reason: str = Field(description="The sentence a human needs, naming what is still missing.")


class SessionSample(BaseModel):
    """What one session contributed to one metric.

    Always present, even while `TREND` is withheld — this *is* the evidence behind the refusal, and
    a golfer who can see "2 sessions, 4 and 5 swings" knows what to do about it. `mean` is the one
    field that is a claim rather than a count, so it alone is gated.
    """

    session_id: str
    captured_at: datetime
    n: int
    mean: float | None = Field(
        default=None,
        description="This session's mean. Populated only when `TREND` is ready — a per-session "
        "mean is a center claim at even smaller `n` than the pooled one.",
    )


class MetricSample(BaseModel):
    """One value of one metric, with the artifact it was derived from.

    The unit of pooling. `artifact_key` is what makes the pooling honest: three re-uploads of one
    clip share a key and contribute one sample, and two swings sharing a shot photo contribute two
    pose samples but one launch-monitor sample. See `CorpusSwing.artifact_key`.
    """

    metric: str
    value: float
    unit: str
    source: str
    session_id: str
    captured_at: datetime
    artifact_key: str
    swing_ref: str = Field(description="`session/swing` of the swing that carried it.")


class MetricBaseline(BaseModel):
    """One golfer, one metric: what their own history supports saying, and what it does not.

    Read the `None`s as refusals, not as missing data — every one of them is itemised in
    `withheld` with the `n` it is waiting for.
    """

    name: str
    unit: str
    source: str = Field(description="Carried from `Measurement.source`, so provenance survives.")

    n: int = Field(description="Distinct contributing artifacts, never swing directories.")
    n_sessions: int = Field(description="Distinct sessions those artifacts came from.")

    # --- CENTER ------------------------------------------------------------------------
    mean: float | None = None
    mean_ci: Interval | None = None
    median: float | None = None

    # --- SPREAD ------------------------------------------------------------------------
    sd: float | None = None
    sd_ci: Interval | None = None
    minimum: float | None = None
    maximum: float | None = None

    # --- TREND -------------------------------------------------------------------------
    sessions: list[SessionSample] = Field(
        default_factory=list,
        description="Oldest first. Present whatever the guard says; only the per-session `mean` "
        "inside is gated.",
    )

    ready: list[BaselineClaim] = Field(default_factory=list)
    withheld: list[WithheldClaim] = Field(default_factory=list)

    def supports(self, claim: BaselineClaim) -> bool:
        """May this metric make this claim?"""
        return claim in self.ready


class PersonalBaseline(BaseModel):
    """Every metric one golfer's own history has an opinion about — or is refusing to have one."""

    player_id: str

    metrics: dict[str, MetricBaseline] = Field(
        default_factory=dict, description="Metric name -> baseline, sorted by name."
    )

    built_from_swings: int = Field(
        default=0,
        description=(
            "Distinct swings that contributed at least one value. Not `CareerCorpus"
            ".distinct_swings`: an unanalyzed, stale or outdated swing is a real swing of this "
            "golfer's that contributed nothing, and this is the count that explains the `n`s."
        ),
    )
    built_from_sessions: int = Field(
        default=0, description="Distinct sessions those contributing swings came from."
    )

    @property
    def claims_ready(self) -> int:
        return sum(len(metric.ready) for metric in self.metrics.values())

    @property
    def nothing_sayable(self) -> bool:
        """True when the guard refused every claim on every metric — the state on disk today."""
        return self.claims_ready == 0
