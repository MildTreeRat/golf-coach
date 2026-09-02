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

## Two scopes, one vocabulary

`TempoPlan` is one swing's target. `CareerTempo` at the foot is one *golfer's* — their whole
measured history plus the target it earns them, which is the thing ADR-023 left under *Deferred, by
choice* until a personal baseline existed over the two durations. It shares the beat vocabulary
above rather than restating it, and carries a `TempoPlan` whole.

Stdlib + pydantic only (ADR-008).
"""

from __future__ import annotations

from datetime import datetime
from enum import StrEnum

from pydantic import BaseModel, Field, model_validator

# The one import out of this module, and it is sideways rather than upward: `CareerTempo` reports
# the baseline guard's refusals verbatim, so it needs the guard's own shape. Restating it would be
# the second refusal vocabulary in a repo whose whole career-mode argument is that a withheld claim
# must look identical wherever it surfaces.
from golf_coach.contracts.baseline import WithheldClaim


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
            "lives: it is `anchor_downswing_ms / <tour median downswing>`, so 1.44 means this "
            "golfer is played 44% slower than the tour median. The patterns themselves are always "
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
            "Whether the target follows this golfer's own measured swing rather than the tour "
            "median. **Swing speed does change swing duration** - LPGA against PGA, driver only, "
            "the backswing runs 1001 ms against 834 and the downswing 267 against 234 - so a "
            "single target would hand a slower golfer a faster golfer's swing. Anchoring captures "
            "that without needing a club-head speed, which is fortunate: every stored shot reads "
            "a smash factor below 1.0, so no usable speed exists to key on (ADR-023 addendum)."
        ),
    )
    anchor_downswing_ms: float = Field(
        gt=0.0,
        description=(
            "**The downswing every pattern is built to land on**, and the half the fit is taken "
            "from - the golfer's own when `anchored`, the tour median otherwise. It is the half a "
            "golfer *feels*: it is how hard they swung, so it is the swing's given rather than "
            "something to prescribe, and the target is fitted to it (ADR-023's 2026-09-02 "
            "addendum). Required for the same reason `anchor_backswing_ms` is: a plan that cannot "
            "say what it was fitted to cannot be told apart from the tour-median case."
        ),
    )
    anchor_backswing_ms: float = Field(
        gt=0.0,
        description=(
            "**The backswing this plan prescribes** - `anchor_downswing_ms` times the tour ratio, "
            "and so the half the golfer is being asked to change. Derived, and carried anyway "
            "rather than left for the page to multiply: it is the number a surface prints in "
            "words ('take it back over about 1300 ms'), and deriving it per surface would put the "
            "tour ratio into pages that are not allowed to know one."
        ),
    )

    downswing_in_tour_range: bool = Field(
        default=True,
        description=(
            "Whether the anchor sits inside the reference downswing's p10-p90. **False is a "
            "notice, never a refusal.** Nothing may be substituted for a downswing outside that "
            "range: under this anchor the downswing is the given and never the fault, so the "
            "guard that used to substitute the tour median for one is deleted (ADR-023's "
            "2026-09-02 addendum; M13 P2 is where it goes). The golfer is told, offered "
            "`in_range_pace`, and decides. Defaults True because the tour "
            "median is trivially inside its own p10-p90: with nothing measured there is nothing "
            "to notice."
        ),
    )
    in_range_pace: float | None = Field(
        default=None,
        gt=0.0,
        description=(
            "The pace that would put the played downswing on the nearest tour edge, or None when "
            "there is nothing to offer. A surface renders its snap control off exactly this "
            "None-ness, so 'is there an offer' and 'should the button exist' are one question - "
            "the shape `CareerTempo.plan` already uses for a target that cannot be given. The "
            "golfer opting in is the point: it overrides their own measurement, which is theirs "
            "to override and was previously done for them by the server."
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

    @model_validator(mode="after")
    def _snap_only_offered_outside_the_range(self) -> TempoPlan:
        """There is nothing to snap a downswing to that is already inside the tour range.

        Pinned in the model rather than left to convention because the two fields are one
        statement read by two different controls - the notice reads the bool, the snap button
        reads the None-ness. A plan claiming both would draw a button that moves a golfer off
        their own measured downswing with no notice above it saying why, and neither surface has
        what it needs to spot the contradiction.

        The converse is deliberately not enforced: `False` with no pace is a coherent state for a
        caller that can see a downswing is out of range and has no edge to offer it.
        """
        if self.downswing_in_tour_range and self.in_range_pace is not None:
            raise ValueError(
                "in_range_pace is an offer to a downswing outside the tour range, and "
                "downswing_in_tour_range says this one is inside it"
            )
        return self

    @property
    def default_pattern(self) -> BeatPattern:
        """The pattern a surface should play unless the golfer picked the other one."""
        return self.patterns[0]


class TempoAnchor(StrEnum):
    """Which measurement a career-level target was fitted to. [ADR-023 addendum 2026-08-22]

    `TempoPlan.anchored` is a bool because a single swing has only two answers: its own downswing
    or the tour's. A career view has three, and the middle one is the whole reason this enum
    exists — a golfer whose baseline is still withheld is not in the same position as one with no
    measurement at all, and a page that could not tell them apart would either print a mean it is
    forbidden to print or refuse a golfer a target it can honestly give them.
    """

    #: The golfer's own mean downswing, and so a claim their history supports.
    CAREER_MEAN = "career_mean"
    #: Their most recent measured downswing. Not a claim about the golfer — a fact about one
    #: swing, which is what makes it sayable at an `n` the `CENTER` guard refuses.
    LATEST_SWING = "latest_swing"
    #: Neither was usable, so the target is the tour median. Also where the anchor lands when the
    #: builder reports back that it anchored nothing — still read off the plan and never predicted,
    #: though since M13 P4 the career branch selects on the same half the builder fits to, so that
    #: path can no longer be taken. The read-back stays as the thing that keeps it that way.
    #: (Until M13 P2 this was also where an out-of-range backswing landed; that guard is deleted,
    #: and an out-of-range *downswing* is now reported on the plan rather than substituted for.)
    TOUR_MEDIAN = "tour_median"


class TempoSwing(BaseModel):
    """One recorded swing's tempo, as measured.

    **A measurement, never a claim about the golfer**, which is exactly why it may be printed at
    any `n`. `PersonalBaseline` withholds a mean over two swings because a mean asserts a tendency;
    "this swing took 901 ms back and 384 ms down" asserts nothing beyond that swing and is the
    evidence the refusal is built out of. Same split `SessionSample` already draws — the counts
    travel while the per-session mean is gated.
    """

    swing_ref: str = Field(description="`session/swing`, matching `MetricSample.swing_ref`.")
    session_id: str
    captured_at: datetime

    ratio: float = Field(gt=0.0, description="`backswing_ms / downswing_ms` for this swing.")
    backswing_ms: float | None = Field(
        default=None,
        description=(
            "Takeaway to top. Optional because the two durations are a later measurement than the "
            "ratio (ADR-023 §Consequences): a swing analyzed by an engine that wrote `tempo_ratio` "
            "and not the halves has a ratio and no halves, and dropping it would shorten the "
            "history for no reason a reader could see."
        ),
    )
    downswing_ms: float | None = Field(default=None, description="Top to impact. See above.")


class CareerTempo(BaseModel):
    """One golfer's tempo across their whole history, and the target it earns them. [ADR-023]

    The career-mode counterpart to the per-swing `TempoPlan`. The results page answers "what did
    *this* swing do"; this answers "what do I usually do, and what should I be practising to" —
    which is the question a golfer standing in the bay actually has, and the one ADR-023 left under
    *Deferred, by choice* until a personal baseline existed over the two durations.

    **Three layers, and they are gated differently on purpose:**

    - `swings` and `latest` are measurements. Never withheld.
    - `typical_*` are claims, and come straight from `PersonalBaseline` — `None` when its `CENTER`
      guard refused, with the refusal itemised in `withheld`. `tempo_ratio` needs 8 samples rather
      than the default 5 (`contracts/baseline.py`), so this stays `None` for longer than the rest
      of the panel and that is the metric's own noise, not an oversight.
    - `plan` is a target, and `anchor` says which of the two it was fitted to.

    Nothing here is stored. It is derived at read time from the corpus, for the reason ADR-023
    rejected storing `TempoPlan` on `SwingResult`: none of it is a measurement *of* a swing, and
    deriving it means the history a golfer sees can never lag the artifacts on disk.
    """

    player_id: str

    swings: tuple[TempoSwing, ...] = Field(
        default=(),
        description="Every distinct swing with a readable tempo, **oldest first**. Deduplicated "
        "by `CorpusSwing.artifact_key`, so three re-uploads of one clip appear once.",
    )
    latest: TempoSwing | None = Field(
        default=None, description="The last entry in `swings`, or None when there are none."
    )

    typical_ratio: float | None = None
    typical_backswing_ms: float | None = None
    typical_downswing_ms: float | None = None

    withheld: tuple[WithheldClaim, ...] = Field(
        default=(),
        description=(
            "Every `CENTER` refusal behind a `None` above, deduplicated by reason. The three "
            "metrics gate separately and at different floors, so a page that printed one refusal "
            "would be silent about the other two."
        ),
    )

    anchor: TempoAnchor = TempoAnchor.TOUR_MEDIAN
    plan: TempoPlan | None = Field(
        default=None,
        description=(
            "The metronome, or None when the reference distributions are unavailable — the same "
            "refusal `build_tempo_plan` makes, for the same reason (ADR-010 §2). `plan.anchored` "
            "and `anchor` always agree: the anchor is chosen in the plan builder and this reports "
            "that decision rather than predicting it."
        ),
    )

    @property
    def n(self) -> int:
        """Distinct swings behind this. The evidence a refusal is read against."""
        return len(self.swings)
