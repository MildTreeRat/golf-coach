"""The tempo trainer's vocabulary — a beat sequence a golfer can swing to. [ADR-023]

Every other surface in this project *judges* a swing that already happened. This one is the first
that asks for a swing back: `evaluate_tempo` says "tempo too quick" and stops, which is a verdict
a golfer cannot act on. A `TempoPlan` is the actionable half — the tour's own durations, laid out
as instants to hear.

## Why two patterns rather than one

There is no single comfortable pulse that marks both the top and impact. Tour medians put the
backswing near 900 ms and the downswing near 267 ms, so the two intervals differ by about 3.4x: a
pulse slow enough to settle into (~67 BPM) cannot mark impact, and one fast enough to mark impact
(~225 BPM) is not a groove. Tour Tempo ships tones rather than a metronome for exactly this reason.

Both answers ship, and they are **not two renderings of one sequence**:

- `GRID` snaps the backswing to a whole number of downswing-length ticks, buying a steady,
  trackable, loopable click at the cost of a ratio that is no longer exactly the tour median's.
- `CUES` keeps the exact ratio and pays with ~900 ms of silence through the backswing.

That difference is why the durations and the ratio live on `BeatPattern` rather than on
`TempoPlan`. One `ratio` field beside two patterns would be false for one of them, and it would be
false quietly — which is the shape of bug this repo's contracts exist to make impossible.

## Why it is in `contracts/`

`feedback` and `api` both need this vocabulary and neither may import `analysis` (ADR-008). Same
argument `unscored.py` and `placements.py` make: shared vocabulary, no shared module.

**Read-only, and that is a decision.** Nothing here records that a golfer practiced, and the mode
toggle is a client-side preference that persists nowhere. ADR-020 says a write path needs its own
decision; this respects that rather than stretching it.

Stdlib + pydantic only (ADR-008).
"""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel, Field


class TempoPattern(StrEnum):
    """Which of the two beat layouts a pattern is. See the module docstring for why both exist."""

    #: Evenly spaced ticks, one downswing-length apart, with the backswing snapped to a whole
    #: number of them. Steady enough to track and to loop; its ratio is an integer by construction.
    GRID = "grid"

    #: Three tones at the exact tour medians — takeaway, top, impact — and silence between. Keeps
    #: the true ratio; gives the golfer nothing to track through the backswing.
    CUES = "cues"


class BeatRole(StrEnum):
    """What one beat marks.

    Carried rather than inferred from position: the page pitches and captions a beat by role, and
    counting positions would break the moment `GRID` resolves to a different number of ticks —
    which it does, because the tick count is derived from the distributions and not written down.
    """

    #: Start the takeaway. Always the first beat, always at 0 ms.
    TAKEAWAY = "takeaway"
    #: The top of the backswing.
    TOP = "top"
    #: Impact. Always the last beat.
    IMPACT = "impact"
    #: Grid filler between the swing instants — `GRID` only. Marks no instant, and exists to keep
    #: the pulse audible; the page renders these quieter and lower so they never sound like a cue.
    SUBDIVISION = "subdivision"


#: The roles that mark a real swing instant, as opposed to keeping the pulse going. `GRID` and
#: `CUES` disagree about how many beats there are and agree exactly about these three.
SWING_INSTANTS: frozenset[BeatRole] = frozenset(
    {BeatRole.TAKEAWAY, BeatRole.TOP, BeatRole.IMPACT}
)


class Beat(BaseModel):
    """One click, and what it means."""

    at_ms: float = Field(
        ge=0.0,
        description="Milliseconds from the start of the sequence. The first beat is always 0.",
    )
    role: BeatRole


class BeatPattern(BaseModel):
    """One playable sequence, with the durations it actually encodes.

    The durations and ratio are the pattern's own — `GRID` rounds the backswing to a whole tick
    count and `CUES` does not, so the two carry different numbers and a reader must be able to see
    which they are hearing.

    **Every time here is at `TempoPlan.pace` == 1.0**, which is the tour reference and not
    necessarily what the golfer is played. Multiply by the plan's `pace` for that. `ratio` is the
    exception and needs no scaling: a pace moves both halves, so it is equally true of both.
    """

    mode: TempoPattern
    beats: tuple[Beat, ...] = Field(
        description="In time order, first at 0 ms, last at impact. Never empty."
    )

    backswing_ms: float = Field(gt=0.0, description="This pattern's takeaway-to-top interval.")
    downswing_ms: float = Field(gt=0.0, description="This pattern's top-to-impact interval.")
    ratio: float = Field(
        gt=0.0,
        description=(
            "`backswing_ms / downswing_ms` for *this* pattern. Exactly an integer for `GRID`, "
            "which is what the snap buys; the unrounded quotient of the tour medians for `CUES`."
        ),
    )

    source: str = Field(
        description=(
            "Which distributions these came from, in prose - the reference corpus, the stratum "
            "and the sample size. Provenance travels with the numbers rather than being looked up "
            "again by whoever renders them (ADR-010 §1)."
        )
    )


class TempoPlan(BaseModel):
    """Everything the results page needs to render a metronome for one swing.

    Derived at read time from a stored `SwingResult` rather than stored on it: nothing here is a
    measurement of the swing, and deriving it means already-stored swings get a trainer without a
    re-analysis or a contract change.
    """

    patterns: tuple[BeatPattern, ...] = Field(
        description=(
            "The playable patterns, **default first** - the order `POPULATION_PLACEMENT_REGISTRY` "
            "already sets for anything a surface renders in sequence. Append, never insert."
        )
    )

    pace: float = Field(
        default=1.0,
        gt=0.0,
        description=(
            "**The multiplier a surface must apply to every beat time**, and where the fitting "
            "lives: it is `anchor_backswing_ms / <tour median backswing>`, so 1.11 means this "
            "golfer is played 11% slower than the tour median. The patterns themselves are always "
            "the tour reference, never pre-scaled - one place applies a pace, and it is the "
            "renderer.\n\n"
            "That split is what lets the golfer's pace control mean something. It is a multiple of "
            "the tour median, pre-set to this value and freely movable from there; had the anchor "
            "been baked into the beats the control would read 100% for everyone, showing nothing "
            "of the decision it overrides. Scaling both halves leaves every `ratio` untouched."
        ),
    )

    anchored: bool = Field(
        default=False,
        description=(
            "Whether the target follows this golfer's own backswing rather than the tour median. "
            "**Swing speed does change swing duration** - LPGA against PGA, driver only, the "
            "backswing runs 1001 ms against 834 and the downswing 267 against 234 - so a single "
            "target would hand a slower golfer a faster golfer's swing. Anchoring captures that "
            "without needing a club-head speed, which is fortunate: every stored shot reads a "
            "smash factor below 1.0, so no usable speed exists to key on (ADR-023 addendum)."
        ),
    )
    anchor_backswing_ms: float = Field(
        gt=0.0,
        description=(
            "The backswing every pattern was built from - the golfer's own when `anchored`, the "
            "tour median otherwise. Carried rather than left implicit because the two cases "
            "produce different targets and a reader cannot tell them apart from the beats alone."
        ),
    )

    observed_backswing_ms: float | None = Field(
        default=None,
        description=(
            "This golfer's own backswing, when tempo was measurable. Optional because no target "
            "depends on it - it makes the prose concrete ('yours took 384 ms') and nothing else. "
            "None means the swing had no timeable tempo, which is a real state, not a gap."
        ),
    )
    observed_downswing_ms: float | None = Field(
        default=None, description="This golfer's own downswing. See `observed_backswing_ms`."
    )

    @property
    def default_pattern(self) -> BeatPattern:
        """The pattern a surface should play unless the golfer picked the other one."""
        return self.patterns[0]
