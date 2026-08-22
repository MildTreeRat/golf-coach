"""Which *family* of cause a golfer's own numbers point at. [Career mode, step 5]

`PersonalBaseline` says where a golfer sits and how tightly they cluster. This says what that
combination is evidence *for* — and it is the reason career mode is worth building at all.

## The argument, restated

This instrument cannot see why a club face is open. Grip, lead-wrist angle and release timing are
all invisible to it: wrists jitter ~6x more than hips, 14.5% of frames fail the visibility gate, and
the club needs M2. So the causal question looks unanswerable.

It is not, because **the shape of a miss is itself evidence**. A miss that repeats at nearly the
same size every swing is produced by something that is the same every swing — grip, alignment, ball
position, face at address — all of which are checkable *before* you swing. A miss that moves shot to
shot cannot be produced by anything static, so it points at timing and release. Those have
completely different fixes, and separating them needs no view of the body at all: it needs the mean
and the spread of one number over enough swings.

## Two findings, never one verdict

`bias` and `scatter` are answered independently, because a golfer can have both and the pair is more
informative than either:

- **bias** — the center sits further from the target than measurement error explains.
- **scatter** — the shot-to-shot spread is larger than measurement error explains.

Both are decided by an *interval*, never a point estimate: bias needs the mean's 95% CI to clear the
tolerance band entirely, scatter needs the sd's 95% CI lower bound to clear the tolerance. That is
the same safety property the minimum-N floors have — a tolerance set too low surfaces as
`NOT_ESTABLISHED` (an honest "cannot tell"), never as a confident wrong pattern.

`NOT_ESTABLISHED` is not "you are fine". It is "nothing this data can distinguish", and every
sentence rendered from it has to say so.

## Observation and interpretation stay separate

`DispersionPattern` names what was *observed* in the numbers. `PATTERN_READING` carries the
coaching reading, phrased as what to check rather than as a diagnosis, because this system cannot
see any of the things it is pointing at. That split is `Measurement` vs `CheckpointScore` one level
up: a pattern must be structurally incapable of reading as "your grip is wrong".

Stdlib + pydantic only (ADR-008). Produced by `analysis.dispersion.build_dispersion`; reuses
`Interval` and `WithheldClaim` from `contracts.baseline` rather than restating them, since a
dispersion refusal for want of `n` *is* a baseline refusal for want of `n`.
"""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel, Field, model_validator

from golf_coach.contracts.baseline import Interval, WithheldClaim


class Finding(StrEnum):
    """The three answers to "is this established?", and the third is not a failure."""

    #: The interval clears the tolerance. This is evidence, at the 95% level.
    ESTABLISHED = "established"
    #: There is enough data to ask, and the answer is "cannot tell from this". Never read as
    #: "no, there is no bias" — an interval straddling the tolerance is consistent with both.
    NOT_ESTABLISHED = "not_established"
    #: The question could not be asked at all: too few samples (see `withheld`), or the metric
    #: has no declared target to measure a bias against (see `unavailable`).
    WITHHELD = "withheld"


class DispersionPattern(StrEnum):
    """What the mean and the spread together look like. An observation, not a diagnosis."""

    #: A repeatable miss with no established scatter. The half that is checkable before you swing.
    BIASED = "biased"
    #: Scatter with no established bias — the miss moves, so nothing static is producing it.
    SCATTERED = "scattered"
    #: Both. They are separable problems and the repeatable one is the cheaper one to remove.
    BIASED_AND_SCATTERED = "biased_and_scattered"
    #: Neither cleared its tolerance. "Nothing distinguishable", not "nothing wrong".
    NOTHING_ESTABLISHED = "nothing_established"


#: What each pattern is evidence *for*, in the voice a golfer can act on. Deliberately hedged:
#: every one of these names something this system cannot see, so none of them may assert it.
#:
#: **Deliberately metric-agnostic, and that was found by running it.** The first cut named the
#: things to check outright — "grip, alignment, ball position, face at address" — which reads
#: correctly under `face_to_path_deg` and is nonsense under `head_sway_norm`, where it printed
#: unchanged. One reading serves every metric, so it may only name the *class* of cause; naming the
#: specific check is a per-metric job, and the module that has the vocabulary for it is `feedback`.
PATTERN_READING: dict[DispersionPattern, str] = {
    DispersionPattern.BIASED: (
        "A miss that repeats at about the same size every swing. Whatever produces it is the same "
        "every time, which points at something already set before the swing starts rather than at "
        "something that happens during it. None of that is visible here, so the test is empirical: "
        "change one thing at setup, hit another set, and see whether the center moves."
    ),
    DispersionPattern.SCATTERED: (
        "The miss moves shot to shot, which is not what a setup error looks like — anything fixed "
        "before the swing would produce the same error each time. Points at timing and release "
        "rather than at something checkable at address."
    ),
    DispersionPattern.BIASED_AND_SCATTERED: (
        "Both a repeatable miss and real shot-to-shot movement. Work the repeatable half first: it "
        "is checkable before you swing, and removing it makes what is left easier to read."
    ),
    DispersionPattern.NOTHING_ESTABLISHED: (
        "Neither a repeatable miss nor scatter beyond what this instrument's own error explains. "
        "That is not a clean bill — it is that nothing here can be distinguished from noise at "
        "this sample size."
    ),
}

#: The reading when a metric has scatter but no declared target, so only half the pair exists.
SCATTER_ONLY_READING = (
    "This varies more shot to shot than measurement error explains, which points at timing rather "
    "than at anything fixed before the swing. Whether it is centered in the right place is a "
    "separate question, and this metric has no target to answer it against yet."
)


class MetricTarget(BaseModel):
    """What a metric is aiming at, and how close counts as arrived.

    `tolerance` is the smallest difference that can be told apart from this pipeline's own error.
    It does two jobs and both are the same quantity used correctly: it bounds how far the center
    must sit from the target before a bias is real, and — because measurement error inflates
    observed spread (`sd_obs^2 ~ sd_true^2 + sd_err^2`) — it is also the level a spread must exceed
    before the scatter is the golfer's rather than the instrument's.
    """

    metric: str
    target: float | None = Field(
        description=(
            "The value this metric is aiming at, or None when no target is established. None is a "
            "real answer for most of the panel and is never a stand-in for zero — see "
            "`no_target_reason`."
        )
    )
    tolerance: float = Field(
        gt=0.0, description="Measurement error in this metric's own units. See the class docstring."
    )
    provenance: str = Field(description="Where `tolerance` came from, in enough detail to redo it.")
    no_target_reason: str = Field(
        default="",
        description=(
            "Why no target is declared, required whenever `target` is None. A bias claim that is "
            "simply absent is indistinguishable from one nobody thought to make."
        ),
    )

    @model_validator(mode="after")
    def _reason_required_without_target(self) -> MetricTarget:
        if self.target is None and not self.no_target_reason:
            raise ValueError(f"{self.metric}: target is None and no reason was given")
        return self


#: Provenance shared by the six pose tolerances, which are **measured rather than judged**.
#:
#: `scripts/golfdb/tune_spatial_metric.py` already computes exactly the quantity a tolerance needs:
#: `noise` (median |lite - full| at identical labelled instants — two estimators looking at the same
#: frames, so whatever they disagree by is a floor on how well the quantity can be known) plus
#: `bound` (median |labelled - detected| under one estimator — the cost of our own segmentation).
#: The tolerances below are those two columns summed, in each metric's own units.
#:
#: **This is an estimate of our error, not a measurement of it.** GolfDB is broadcast tour footage,
#: ~47% slow-motion and shot on cameras nobody here controls; our bay footage is two hand-held
#: iPhones. It is the best evidence that exists today and it is the first thing a real bay session
#: should revise — the same posture `contracts.baseline.METRIC_MINIMUM_N` takes toward its floors.
_TUNED = (
    "spread/error harness over 461 face-on GolfDB clips, 2026-08-12: "
    "`python scripts/golfdb/tune_spatial_metric.py`, tolerance = noise + bound"
)

#: The judgment behind the two launch-monitor tolerances, and it is judgment.
#:
#: There is **no instrument-error evidence for the OCR path at all** — the estimator-disagreement
#: trick above has no analogue for a photographed screen, which `contracts.baseline` already records
#: about the same two metrics' minimum-N floors. 2 degrees of face-to-path is a shape small enough
#: that no coaching action follows from it, and it sits well clear of any plausible misread on the
#: two angle fields. Erring wide is the safe direction: it costs claims, where erring narrow buys
#: confident claims about the simulator's own noise.
_JUDGED_DEGREES = (
    "judgment, 2026-08-12: no instrument-error evidence exists for the OCR path. 2 degrees is "
    "below the level a coaching action follows from. Revise from a real bay session's repeats"
)

#: The two distance tolerances, and they are judgment for the same reason `_JUDGED_DEGREES` is:
#: there is no instrument-error evidence for the OCR path at all, and the estimator-disagreement
#: trick behind `_TUNED` has no analogue for a photographed screen.
#:
#: 5 yards is below the level a coaching action follows from — nobody changes anything about a
#: swing over it — and it sits well clear of any plausible misread of a printed three-digit number.
#: Erring wide is the safe direction here exactly as it is above: it costs claims, where erring
#: narrow buys confident claims about the simulator's own noise.
#:
#: **A single constant is the wrong shape and this is the deferral, recorded rather than hidden.**
#: Distance error plausibly scales with the club — 5 yards is a different fraction of a driver than
#: of a sand wedge — so the correct tolerance is per club, and it needs a per-club sample this repo
#: does not have yet. M9's `narrow_to(club=)` is what makes that measurable; until then one wide
#: constant, chosen so the wedge end is not judged against the driver's error.
_JUDGED_YARDS = (
    "judgment, 2026-08-21 (M9 P8): no instrument-error evidence exists for the OCR path. 5 yards "
    "is below the level a coaching action follows from. The correct tolerance is per club and "
    "scales with carry; deferred until a per-club sample exists, and erring wide until then"
)

#: The two absolute durations, and why they are not `_TUNED` like the six metrics beside them.
#:
#: `tune_spatial_metric.py` **cannot** score a duration, and it now says so rather than printing a
#: number: its `boundary` column compares a metric at labelled instants against the same metric at
#: detected ones, but labelled phases carry frame indices in `start_ms` by design
#: (`derive_pose_metrics._phases_from_events`, since ~47% of GolfDB is slow-motion and only ratios
#: were ever read from them). For a duration that comparison is frames against milliseconds.
#:
#: So these are composed from the instant errors instead, which M4-REF measured directly and which
#: is the one term a duration's error is made of — a duration is a subtraction of two instants and
#: reads no landmark at all. At the corpus's 30 fps one frame is 33.4 ms:
#:
#:   backswing = top - motion start -> (2 + 7) frames = 300 ms
#:   downswing = impact - top       -> (1 + 2) frames = 100 ms
#:
#: Address dominates the first, at a median 7 frames with 40% of clips over 10, exactly as it
#: dominates `tempo_ratio`'s 0.943 — and the two agree: 300 ms of backswing error on a ~267 ms
#: downswing is about 1.1 of ratio, which is the same size as the tuned figure arrived at
#: independently. That agreement is the only cross-check available and it is worth having.
#:
#: **Both are estimates from a broadcast corpus and the first thing a bay session should revise**,
#: on the same footing as `_TUNED` and `contracts.baseline.METRIC_MINIMUM_N`.
_FROM_INSTANTS = (
    "composed from M4-REF instant errors, 2026-08-20: a duration is a subtraction of two instants, "
    "so its error is theirs summed - address 7 frames, top 2, impact 1, at the corpus's 30 fps. "
    "tune_spatial_metric.py cannot measure these; see ADR-023"
)

#: metric -> what it aims at and how close counts. Every registered metric gets a scatter finding;
#: only the four with a `target` get a bias finding, and the other four say why in the record
#: itself rather than being quietly absent.
#:
#: The split is not squeamishness. Declaring a target is declaring what *good* is, which this repo
#: does in exactly one place — a benchmark band with a derivation behind it (ADR-010 §2). Two of the
#: four targets below are already the repo's stated position (one-sided bands `[0, high]`), and two
#: are physics rather than population. For the rest, a target would be an invention, and
#: `analysis.baseline` was careful for the same reason not to import `benchmarks` at all.
METRIC_TARGETS: dict[str, MetricTarget] = {
    "face_to_path_deg": MetricTarget(
        metric="face_to_path_deg",
        target=0.0,
        tolerance=2.0,
        provenance=_JUDGED_DEGREES,
        # Zero is straight by ball-flight physics, not by a distribution — the face and the path
        # agreeing produces no curvature whoever is swinging. This is the metric the whole
        # milestone was argued from.
    ),
    "start_line_deg": MetricTarget(
        metric="start_line_deg",
        target=0.0,
        tolerance=2.0,
        provenance=_JUDGED_DEGREES,
    ),
    # The two distances, measured since M9 P8 and judged by nothing. **No target, and that is not
    # a gap to be filled in later**: how far a golfer *should* hit a club is not a number this repo
    # has, and the only populations it holds are cut from GolfDB, which is broadcast pose footage
    # with no ball flight in it. A tour carry band would also judge an amateur against a population
    # they are not in, which is a different failure from the one ADR-010 §2 guards and just as bad.
    #
    # They still register, because a tolerance is what buys the *scatter* finding — how repeatable
    # this golfer's distance is, which is answerable without anyone declaring what good is.
    #
    # **Do not add `METRIC_MINIMUM_N` overrides for these.** The defaults are right for distance:
    # nothing about a printed carry is known to need more samples than a pose metric does, and every
    # override in that table was moved off a measured spread/error figure rather than a hunch.
    # Adding one here speculatively would be a refusal nobody derived. `contracts/baseline.py` says
    # the same beside the table itself.
    "carry_distance_yds": MetricTarget(
        metric="carry_distance_yds",
        target=None,
        tolerance=5.0,
        provenance=_JUDGED_YARDS,
        no_target_reason=(
            "how far a golfer should hit a given club is not a number this repo has. Every "
            "distribution here is cut from GolfDB, which contains no ball flight, and a tour "
            "carry band would judge an amateur against a population they are not in"
        ),
    ),
    "total_distance_yds": MetricTarget(
        metric="total_distance_yds",
        target=None,
        tolerance=5.0,
        provenance=_JUDGED_YARDS,
        no_target_reason=(
            "the same as carry, plus one more: the gap between them is roll, which is the ground "
            "rather than the swing - a target would judge a golfer for the mat they hit off"
        ),
    ),
    # The two one-sided magnitudes. "Lower is strictly better" is already this repo's position for
    # both: the shipped bands are `[0.0, 0.43]` and `[0.0, 0.29]`, and `CheckpointScore.one_sided`
    # is the type that says so, so reading a target of 0 off them is not a new claim.
    #
    # **What a bias means here is weaker than it looks, and the difference matters.** Zero head
    # sway is not attainable by a human, so "the center is distinguishable from 0" is established
    # for essentially every golfer. What it actually asserts is *a consistent amount, above
    # measurement error* — which is exactly the half of the contrast this step needs (consistent
    # against scattered) and is **not** a claim that the amount is too much. That question is the
    # tour band's, and `analysis.comparison` answers it (step 6) without a target being invented
    # here: it asks whether the personal center's interval sits inside the band.
    "head_sway_norm": MetricTarget(
        metric="head_sway_norm",
        target=0.0,
        tolerance=0.060,
        provenance=_TUNED,
    ),
    "finish_balance_norm": MetricTarget(
        metric="finish_balance_norm",
        target=0.0,
        tolerance=0.024,
        provenance=_TUNED,
    ),
    "hip_sway_norm": MetricTarget(
        metric="hip_sway_norm",
        target=None,
        tolerance=0.050,
        provenance=_TUNED,
        no_target_reason=(
            "'less is better' is not established for it — some lateral hip travel is a weight "
            "shift rather than a fault, and nothing here knows how much. It has a two-sided band "
            "as of 2026-08-12, which is a range rather than a target: `analysis.comparison` asks "
            "whether the personal center sits inside it, so no point value has to be invented here"
        ),
    ),
    "hip_shift_at_top_norm": MetricTarget(
        metric="hip_shift_at_top_norm",
        target=None,
        tolerance=0.053,
        provenance=_TUNED,
        no_target_reason=(
            "same as hip_sway_norm: it has a one-sided band as of 2026-08-12, but a band edge is "
            "not a target, and the right amount of hip travel going back is not a number this repo "
            "has. Its lower edge is omitted for being unmeasurable, not for being zero"
        ),
    ),
    "head_hip_offset_impact_norm": MetricTarget(
        metric="head_hip_offset_impact_norm",
        target=None,
        tolerance=0.073,
        provenance=_TUNED,
        no_target_reason=(
            "career mode step 4 established that the sign is readable here — a personal corpus "
            "is single-handed by construction, so the camera-relative ambiguity that blocks a tour "
            "band does not apply. A readable sign is not a target: staying behind the ball is "
            "right, and how far behind is not established"
        ),
    ),
    "head_hip_gain_norm": MetricTarget(
        metric="head_hip_gain_norm",
        target=None,
        tolerance=0.080,
        provenance=_TUNED,
        no_target_reason=(
            "it has a band (a two-sided tour range) rather than a value, and the band is the "
            "wrong shape to read a target off: too little head-hip separation is the head "
            "drifting forward with the hips, too much is hanging back through impact, and the "
            "right amount for one golfer is not a number this repo has. `analysis.comparison` "
            "asks the band's own question instead — whether the personal center's interval sits "
            "inside it — which needs no point target invented here. Note the sign convention: "
            "values live in a right-handed camera frame, so a personal center is only meaningful "
            "against a corpus folded the same way"
        ),
    ),
    "tempo_ratio": MetricTarget(
        metric="tempo_ratio",
        # The one metric whose error is entirely our own segmentation: the harness reports noise
        # 0.000, because tempo reads phase instants only and both estimators were handed GolfDB's
        # labelled ones. The whole 0.943 is `bound` — the price of the address instant, which
        # carries a median error of 7 frames with 40% of clips over 10 (M4-REF).
        target=None,
        tolerance=0.943,
        provenance=_TUNED,
        no_target_reason=(
            "its target is a band (2.72-4.71 from 1,399 tour swings), not a value. Reading it "
            "would import `benchmarks` into the personal-baseline path, which is the boundary "
            "`analysis.baseline` holds on purpose (ADR-010 §2). `analysis.comparison` answers the "
            "band's question directly instead — whether the center's interval sits inside it — so "
            "no point target has to be invented here (career mode step 6)"
        ),
    ),
    "backswing_ms": MetricTarget(
        metric="backswing_ms",
        target=None,
        tolerance=300.0,
        provenance=_FROM_INSTANTS,
        no_target_reason=(
            "there is a tour median (about 901 ms) and it is deliberately not a target. Tempo is "
            "already scored once, as a ratio, and declaring a target for each half would put a "
            "second verdict on one fundamental and count it twice — ADR-023 is explicit that these "
            "ship as distributions and never as a band. The tour distribution is still readable "
            "where it belongs, in `analysis/tempo_trainer.py`, which builds a metronome from it "
            "rather than a judgment"
        ),
    ),
    "downswing_ms": MetricTarget(
        metric="downswing_ms",
        target=None,
        tolerance=100.0,
        provenance=_FROM_INSTANTS,
        no_target_reason=(
            "the same reason as `backswing_ms` — one fundamental is scored once. Worth noting what "
            "the corpus says about this half in particular: tour downswing duration barely moves "
            "at all (p10 200 ms, p90 300 ms, and a between-club sd of 6.9 ms against 47.0 ms "
            "between golfers), so it looks more like a constant than any other quantity here. That "
            "is a reason it makes a good metronome, not a reason to score it"
        ),
    ),
}

#: How much larger the pooled spread must be than the within-session spread before it is worth
#: saying that part of it is drift between occasions rather than shot-to-shot scatter.
#:
#: Judgment, and deliberately loose: this only ever *adds a caveat*, never removes a claim, so
#: being wrong about it costs a sentence rather than a verdict.
SESSION_DRIFT_FACTOR = 1.25


def target_for(metric: str) -> MetricTarget | None:
    """What this metric aims at, or None if it is not registered at all.

    An unregistered metric is refused both findings rather than defaulted into one — there is no
    tolerance for it, so neither question can be asked. Same posture as
    `contracts.baseline.minimum_n`'s fallback, one notch more conservative: a metric added tomorrow
    is silent by omission rather than judged against a borrowed error term.
    """
    return METRIC_TARGETS.get(metric)


class MetricDispersion(BaseModel):
    """One golfer, one metric: the shape of their miss, and what that shape is evidence for."""

    name: str
    unit: str
    source: str

    n: int = Field(description="Distinct contributing artifacts. Identical to `MetricBaseline.n`.")
    n_sessions: int

    target: float | None = None
    tolerance: float | None = Field(
        default=None, description="None only when the metric is unregistered — see `target_for`."
    )

    # --- bias -------------------------------------------------------------------------
    bias: Finding = Finding.WITHHELD
    center: float | None = Field(
        default=None,
        description=(
            "The golfer's mean. Carried from `MetricBaseline.mean`, so it is present only when "
            "step 4's CENTER guard allowed it — this contract never recomputes a sealed statistic."
        ),
    )
    center_ci: Interval | None = None
    offset: float | None = Field(
        default=None, description="`center - target`: the size and direction of the miss."
    )

    # --- scatter ----------------------------------------------------------------------
    scatter: Finding = Finding.WITHHELD
    sd: float | None = None
    sd_ci: Interval | None = None
    within_session_sd: float | None = Field(
        default=None,
        description=(
            "Pooled within-session spread, when at least two sessions carry at least two samples "
            "each. Present as evidence beside `sd`, which pools across occasions and so mixes "
            "shot-to-shot scatter with drift between them."
        ),
    )

    # --- the pair ---------------------------------------------------------------------
    pattern: DispersionPattern | None = Field(
        default=None,
        description=(
            "Populated only when **both** findings were answerable. A pattern is a statement about "
            "the pair, and one half of it cannot imply the other — `SCATTERED` asserts that the "
            "bias was looked for and not found, which is untrue when bias was never asked."
        ),
    )
    points_at: str | None = Field(
        default=None, description="What to check, phrased as a check and never as a diagnosis."
    )

    caveats: list[str] = Field(
        default_factory=list, description="Things true of this reading that qualify it."
    )
    withheld: list[WithheldClaim] = Field(
        default_factory=list,
        description="Refusals for want of `n`, inherited whole from the step-4 guard.",
    )
    unavailable: list[str] = Field(
        default_factory=list,
        description=(
            "Refusals no amount of swinging fixes — no declared target, no registered tolerance. "
            "Kept apart from `withheld` because the two need opposite responses: one says book "
            "another bay hour, the other says this metric needs a band before it can speak."
        ),
    )


class GolferDispersion(BaseModel):
    """Every metric's miss-shape for one golfer, or the refusal standing in for it."""

    player_id: str
    metrics: dict[str, MetricDispersion] = Field(
        default_factory=dict, description="Metric name -> dispersion, sorted by name."
    )

    built_from_swings: int = 0
    built_from_sessions: int = 0

    @property
    def patterns_established(self) -> int:
        """Metrics where both findings were answerable, whatever the answers were."""
        return sum(1 for metric in self.metrics.values() if metric.pattern is not None)

    @property
    def nothing_established(self) -> bool:
        """True when not one metric could even be asked — the state on disk today."""
        return self.patterns_established == 0
