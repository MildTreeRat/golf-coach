"""Where a golfer's own center sits in the tour population. [Career mode, step 6]

`PersonalBaseline` says where this golfer sits. `Distribution` says where 122 tour players sit.
This is the join, and it is the last piece of reasoning career mode needs: it turns "a consistent
0.31 of head sway" into "and that sits inside the range tour swings occupy".

The join was designed for from the start — `MetricBaseline` mirrors `Distribution`'s shape
deliberately, so this is a lookup rather than a redesign — but it was kept out of steps 4 and 5 on
purpose. `analysis.baseline` and `analysis.dispersion` both import no `benchmarks` at all, which is
what stops a personal statistic from quietly becoming a change to how a swing is scored
(ADR-010 §2). Landing the join in its own module keeps that property: the boundary moved by one
layer, it did not dissolve.

## A tour band is a description, not a target

The reason career mode exists at all is that *"a 15-handicap held to a tour p10-p90 fails
everything forever"* (ROADMAP). So `OUTSIDE` must not read as a fault, and nothing here carries a
`score` or a `passed` — the same structural refusal `Measurement` and `MetricDispersion` hold. What
this contract asserts is placement in a population and nothing else.

## Decided by the interval, like everything else in this milestone

`Standing` is read off the mean's 95% CI, never off the mean. A center that sits a hair outside p90
with an interval straddling it has not been shown to be outside anything, and `STRADDLES` says so.
That is the same discipline `analysis.dispersion` applies to bias and scatter, and it has the same
payoff: too small an `n` surfaces as an honest "cannot tell" rather than as a confident placement.

## Three metrics are refused, for two different reasons

Six of the eight career metrics have a stored distribution, and only five may use it. The refusals
are in `unavailable` rather than absent, because a comparison nobody made and a comparison this
repo declines to make are different facts.

Stdlib + pydantic only (ADR-008). Reuses `Interval` and `WithheldClaim` from `contracts.baseline`
rather than restating them: a standing refused for want of `n` *is* a baseline refused for want of
`n`, and it should print the same sentence.
"""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel, Field

from golf_coach.contracts.baseline import Interval, WithheldClaim
from golf_coach.contracts.career import LAUNCH_MONITOR_SOURCE_PREFIX


class Standing(StrEnum):
    """Where the golfer's center sits relative to the tour band. Placement, never a verdict."""

    #: The whole 95% interval sits within p10-p90. This golfer's typical value is one the tour
    #: population routinely produces.
    INSIDE = "inside"
    #: The whole interval sits beyond one edge. **Not a fault** — see `STANDING_READING`.
    OUTSIDE = "outside"
    #: The interval crosses an edge, so the data does not settle which side of it the center is on.
    #: An honest "cannot tell", exactly like `Finding.NOT_ESTABLISHED`.
    STRADDLES = "straddles"
    #: The question could not be asked: no center survived the step-4 guard (see `withheld`), or
    #: this metric has no population it may be compared against (see `unavailable`).
    WITHHELD = "withheld"


#: What each standing means, in the voice a golfer can act on — and deliberately hedged, because
#: the most likely misreading of this whole module is that a tour band is a target.
#:
#: **Metric-agnostic on purpose.** One reading serves every metric, so it may name only the class
#: of statement being made. Step 5 shipped a `BIASED` text naming "grip, alignment, ball position"
#: which was correct under `face_to_path_deg` and nonsense under `head_sway_norm`, where it printed
#: unchanged; the fix was to stop the shared reading naming any specific check, and the same rule
#: binds here.
STANDING_READING: dict[Standing, str] = {
    Standing.INSIDE: (
        "This golfer's typical value sits inside the range tour swings occupy. That is a statement "
        "about where the number falls in a population, not a pass — the band describes what tour "
        "players do, and being inside it is not evidence that nothing here is worth working on."
    ),
    Standing.OUTSIDE: (
        "This golfer's typical value sits outside the middle 80% of tour swings. Read it as a "
        "distance from a population, not as a fault: the band describes what tour professionals "
        "do, and holding an amateur to it means failing everything forever. What makes it useful "
        "is direction — `outside_by` says which way this metric would move to look more like that "
        "population, and by how much. On a one-sided magnitude, below the band is the good side."
    ),
    Standing.STRADDLES: (
        "The interval around this golfer's typical value crosses the edge of the tour range, so "
        "which side of it they sit on is not settled by this much data. Not 'borderline' — "
        "unresolved."
    ),
}


#: Metrics that **have** a stored distribution and still may not be compared against it.
#:
#: One entry, and it is the mirror image of a step-4 finding rather than a new rule.
#: `head_hip_offset_impact_norm` is the one metric a *personal* baseline can interpret and a tour
#: band cannot: its sign is camera-relative, a personal corpus is single-handed by construction so
#: the sign is consistent within it, and the GolfDB population is not. M6.5 blocked this metric from
#: becoming a checkpoint for exactly that reason. A stored distribution existing is not the same as
#: a stored distribution meaning something.
TOUR_COMPARISON_BLOCKED: dict[str, str] = {
    "head_hip_offset_impact_norm": (
        "its sign is camera-relative, and the tour distribution is cut from GolfDB's mixed-"
        "handedness population — the reason M6.5 blocked this metric from becoming a checkpoint. "
        "A personal baseline reads the sign because one golfer swings from one side; averaging "
        "left- and right-handed swings puts the two signs in one number, so placing a personal "
        "center in that population would compare a real quantity against a meaningless one"
    ),
}

#: Why a launch-monitor metric has no population here, and it is not an oversight to be filled in.
#: Every distribution in this repo is derived from GolfDB, which is pose estimated off broadcast
#: video — there is no ball flight in it at all. `face_to_path_deg` and `start_line_deg` would need
#: a launch-monitor reference corpus, which is a different acquisition problem, not a missing row.
NO_LAUNCH_MONITOR_POPULATION = (
    "no tour population exists for it. Every distribution in this repo comes from GolfDB, which is "
    "pose estimated from broadcast video and contains no ball flight — a launch-monitor reference "
    "would have to be acquired, not derived"
)

#: The generic case: a pose metric nobody cut a distribution for.
NO_POPULATION = "no reference distribution is stored for it"

#: Why the spread is never compared to the tour spread, though both are called `sd` and sit one
#: field apart. **This is a category error, and it is recorded rather than computed.**
#:
#: `Distribution.sd` is *between-player* variation: 458 clips over 122 players, under four swings
#: each, so it measures how much tour players differ from one another. A personal `sd` is
#: *within-player* — how much one golfer varies shot to shot, which is the quantity this whole
#: milestone was built to read. "Your spread is tighter than the tour's" would be comparing a
#: golfer's repeatability against a population's diversity and would come out flattering for
#: everyone, since one person always varies less than 122 people do.
SPREAD_NOT_COMPARABLE = (
    "the tour sd measures how much 122 players differ from each other, while a personal sd "
    "measures how much one golfer varies shot to shot. They are different quantities and "
    "comparing them would flatter every golfer alive"
)


class MetricComparison(BaseModel):
    """One golfer, one metric: where their center sits in the tour population, or why it cannot."""

    name: str
    unit: str
    source: str

    n: int = Field(description="Distinct contributing artifacts. Identical to `MetricBaseline.n`.")
    n_sessions: int

    # --- the golfer's side, carried and never recomputed ---------------------------------
    center: float | None = Field(
        default=None,
        description=(
            "The golfer's mean, carried from `MetricBaseline.mean` — so it is present only when "
            "step 4's CENTER guard allowed it. This contract never recomputes a sealed statistic."
        ),
    )
    center_ci: Interval | None = None

    # --- the population's side ------------------------------------------------------------
    standing: Standing = Standing.WITHHELD
    reading: str | None = Field(
        default=None, description="`STANDING_READING` for the standing, when there is one."
    )

    percentile: float | None = Field(
        default=None,
        description=(
            "Where `center` falls in the tour population, from `Distribution.percentile_of`. "
            "Informational only and deliberately off the scoring path (ADR-010 §2)."
        ),
    )
    percentile_clamped: bool = Field(
        default=False,
        description=(
            "True when the percentile landed on a rail. The stored quantiles stop at p10/p90, so a "
            "center past either is reported as 'at least this extreme' rather than extrapolated "
            "into a precise-looking number the data cannot support. Said out loud because a bare "
            "90 reads as a rank."
        ),
    )

    band_low: float | None = Field(default=None, description="The population's p10.")
    band_high: float | None = Field(default=None, description="The population's p90.")
    outside_by: float | None = Field(
        default=None,
        description=(
            "Signed distance from the nearer band edge, populated only when `standing` is "
            "OUTSIDE — negative below p10, positive above p90. `MetricDispersion.offset`'s "
            "counterpart, and present for the same reason: 'outside' on its own is a fact nobody "
            "can size, and `percentile` cannot supply it because it clamps at the rails. This is "
            "what makes the direction in `STANDING_READING[OUTSIDE]` a promise the shape keeps."
        ),
    )
    population_n: int | None = Field(default=None, description="Clips behind the distribution.")
    population_players: int | None = Field(
        default=None,
        description=(
            "Distinct golfers behind it. Carried beside `population_n` because they are different "
            "sample sizes and the smaller one binds — the same reasoning "
            "`contracts.baseline.MINIMUM_SESSIONS` applies to a personal corpus."
        ),
    )

    withheld: list[WithheldClaim] = Field(
        default_factory=list,
        description="Refusals for want of `n`, inherited whole from the step-4 CENTER guard.",
    )
    unavailable: list[str] = Field(
        default_factory=list,
        description=(
            "Refusals no amount of swinging fixes — no population, or one this metric may not be "
            "placed in. Kept apart from `withheld` because the two need opposite responses: one "
            "says book another bay hour, the other says this needs a reference corpus first."
        ),
    )


class GolferStanding(BaseModel):
    """Every metric's placement in the tour population for one golfer, or the refusal for it."""

    player_id: str
    metrics: dict[str, MetricComparison] = Field(
        default_factory=dict, description="Metric name -> comparison, sorted by name."
    )

    built_from_swings: int = 0
    built_from_sessions: int = 0

    @property
    def placements(self) -> int:
        """Metrics whose center could actually be placed, wherever it landed."""
        return sum(
            1 for metric in self.metrics.values() if metric.standing is not Standing.WITHHELD
        )

    @property
    def nothing_placed(self) -> bool:
        """True when not one metric could be placed — the state on disk today."""
        return self.placements == 0


def no_population_reason(metric: str, source: str) -> str:
    """Why `metric` has no usable tour population, in the voice a refusal has to explain itself in.

    Keyed on `Measurement.source` rather than on a list of metric names, so a launch-monitor metric
    added tomorrow inherits the right sentence instead of the generic one — the same reason
    `CorpusSwing.artifact_key` dispatches on the prefix rather than on a hardcoded pair.
    """
    if source.startswith(LAUNCH_MONITOR_SOURCE_PREFIX):
        return f"{metric}: {NO_LAUNCH_MONITOR_POPULATION}"
    return f"{metric}: {NO_POPULATION}"
