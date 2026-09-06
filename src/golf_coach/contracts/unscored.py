"""Why a checkpoint produced no score. The vocabulary, and the remedy each cause deserves.

ADR-010 §2 says no score beats a wrong one, so an evaluator that cannot measure returns nothing
rather than a guess. ADR-013 added the disclosure half: the checkpoint is *named* in
`SwingResult.unscored`, because `overall_score` is a mean over survivors and a swing judged on
five fundamentals must not print the same shape of number as one judged on six.

This module is the third part of that argument — **which** of the things that can go wrong did.

## Why a name alone was not enough

Names-only left every consumer to reconstruct the cause, and two of them did. `feedback/rules.py`
inferred it from a side channel: if the metric appeared in `measurements` then the footage was
readable, so a missing `head_stays_back` score must be the handedness case. That worked for
exactly one checkpoint and could never work for a second, because the inference is not about the
failure — it is about what happened to be measurable beside it. `api/pipeline.py` narrated the
same case again in free text. Neither was the source, and the source had the answer all along:
every `return None` in `analysis/measure.py` and `analysis/checkpoints/mechanics.py` knows exactly
which condition it just failed.

The distinction that actually reaches a golfer is `refilming_helps`. "Film it again" is the right
advice for an unreadable clip and actively wrong for a missing form field — it sends someone back
to the bay over something they can fix from the results page in one click.

## Why the vocabulary lives in `contracts/`

`feedback` may not import `analysis` (ADR-008), which is why `rules.py` used to hold checkpoint
names as bare string literals with a comment apologising for it. Same problem `contracts.caveats`
solves for the standing warnings and `contracts.dispersion.METRIC_TARGETS` for the error floors:
shared vocabulary, no shared module. `feedback`, `mcp`, `api` and `caveats` all read the one table
below.

## Three families, not one

The vocabulary started as "why a *checkpoint* produced no score" and has outgrown the noun twice.
`MEASUREMENT_REASONS` are what `analysis.measure` may report, their complement was the judging
half, and M15 P9 added `INFERENCE_REASONS` — why a *simulated* flight could not recover an input
the launch monitor never printed. Nothing about that third family is a checkpoint: ADR-027
§Decision 6 gives the flight its own measurement names and no `CHECKPOINT_REGISTRY` entry. What it
shares with the other two, and the only thing this module has ever really been about, is that the
absence is *named* instead of being filled in.

`ExcludedSwing` in `contracts.career` is the shape this mirrors deliberately — reason plus a
human-readable `detail`, on the same principle that nothing is dropped silently. The two answer
the same question one level apart: that one says why a *swing* is not in a corpus, this says why a
*checkpoint* is not in a score.

Stdlib + pydantic only (ADR-008).
"""

from __future__ import annotations

from enum import StrEnum
from typing import NamedTuple

from pydantic import BaseModel, Field


class UnscoredReason(StrEnum):
    """Every way a checkpoint can fail to produce a score. Closed, and reported rather than guessed.

    The members are not a taxonomy designed top-down — each one is a condition that already existed
    as a `return None`, given a name. Adding a member means a new failure condition was written; it
    does not mean this list was incomplete.
    """

    #: A phase the checkpoint reads is absent from the segmentation, so its window was never
    #: located. `measure.phase_bounds` / `measure.address_sample_bounds` / `measure.tempo_timings`.
    PHASE_NOT_SEGMENTED = "phase_not_segmented"

    #: The start of the backswing was **estimated rather than detected** (ADR-013). That estimate
    #: is derived from an assumed tempo ratio, so a tempo reported from it would echo the
    #: assumption back as an observation. Its own cause because it is the common one — about 14%
    #: of clips — and because the clip was otherwise fine, which changes what you tell the golfer.
    BOUNDARY_ESTIMATED = "boundary_estimated"

    #: The instants landed in an order that gives a non-positive backswing or downswing duration.
    #: Distinct from `PHASE_NOT_SEGMENTED`: every phase is present, they just disagree.
    TIMING_DEGENERATE = "timing_degenerate"

    #: The landmarks this checkpoint reads never cleared the visibility gate across the window it
    #: needs. Hips at the finish are the usual one — they are gated harder than everything else
    #: (`measure.MIN_HIP_VISIBILITY`) precisely because a confidently-wrong hip is worse than none.
    LANDMARKS_UNCONFIDENT = "landmarks_unconfident"

    #: The window held usable frames, but fewer than the minimum the statistic needs
    #: (`measure.MIN_FINISH_FRAMES`). Separate from `LANDMARKS_UNCONFIDENT` because "a few good
    #: frames, not enough of them" and "no good frames at all" are different clips.
    TOO_FEW_FRAMES = "too_few_frames"

    #: Shoulder width — the ruler every `_norm` metric divides by — was unmeasurable or degenerate
    #: (`measure.MIN_SHOULDER_WIDTH`: golfer turned side-on, or shoulders mis-detected). The
    #: quantity may well have been readable; there was no scale to express it in.
    SCALE_UNAVAILABLE = "scale_unavailable"

    #: The swing was measured fine and no benchmark band resolved for this checkpoint at this club
    #: and profile. Not a capture problem, and not the golfer's to fix — the number is still
    #: recorded in `SwingResult.measurements`, which is the order M6.5 exists to allow.
    NO_BAND = "no_band"

    #: No golfer is attributed to this swing, and this checkpoint's sign is camera-relative
    #: (`head_stays_back`). Guessing right-handed would score a left-handed golfer's ordinary
    #: impact position as a gross fault, so it refuses. Fixable from the results page.
    NO_HANDEDNESS = "no_handedness"

    #: The other view, on a shared clock, contradicts the instants this checkpoint was timed
    #: from. Only reachable on a bundle whose two clips **both heard the ball strike**: with tau=2
    #: pinned to one sound the two downswings become two measurements of one interval, and the
    #: shorter one is the late top (`analysis.alignment._arbitrate_tops`, M11 P7). The face-on
    #: clip alone looks fine, which is the point — nothing inside it could have found this.
    CROSS_VIEW_CONTRADICTED = "cross_view_contradicted"

    #: No spin at all reproduces the carry the launch monitor printed, from the launch conditions
    #: it printed beside it (`analysis.spin_solve`: `ABOVE_PEAK` or `BELOW_FLOOR`). A finding about
    #: the two models rather than about the golfer — read it beside this model's own ~2.6%
    #: disagreement with HD Golf before calling it an OCR fault, because most of the shots on disk
    #: that land here miss by less than that (ADR-027's 2026-09-05f addendum).
    CARRY_UNREACHABLE = "carry_unreachable"

    #: Two spins fly that carry, one either side of the peak, and no loft is on record to choose
    #: between them (ADR-027 §Decision 3: loft's only job in ball flight is this branch). Not a
    #: capture problem and **not a dead end** — it is the one reason in this family a golfer can
    #: clear, and M15 P10 found which half of the repair is actually outstanding here: every
    #: 2026-08-23 swing already carries a club tag, so what is missing is the *declared loft* on
    #: the bag entry that tag points at. Six shots on disk say `3w` and no 3 wood has ever been
    #: declared; the repair is the bag page, not the bay.
    NO_CLUB_LOFT = "no_club_loft"

    #: The carry does not pin a spin the club could have produced. Three of `SpinSolveCase`'s
    #: shapes arrive here and `detail` says which: either plateau, where infinitely many spins fly
    #: exactly that carry because the coefficient table is clamped and spin has left the problem;
    #: and the unique answer between the plateaus, which sits on the rising branch below the
    #: peak-carry spin while the loft prior says the club spins above it.
    SPIN_NOT_RECOVERABLE = "spin_not_recoverable"

    #: Nothing on the screen fixes how far the spin axis is tilted, so the flight is simulated in
    #: the vertical plane and its landing offline is withheld (ADR-027 §Decision 5's third branch).
    #: The *direction* of the curve is usually known — `Shot Type` prints it in words and
    #: face-to-path agrees with it — and a direction with no magnitude is not an axis.
    SPIN_AXIS_UNRESOLVED = "spin_axis_unresolved"

    #: The screen printed too little of the launch to fly anything — no ball speed, no launch
    #: angle, or a launch angle at or below the horizontal, which is a ball that rolls rather than
    #: one that flies (`analysis.flight.simulate_flight` names that guard as the caller's to own).
    #: Fires on nothing in this repo's corpus today and exists because the OCR drops tiles one at a
    #: time rather than all at once: one stored shot is already missing its face angle, and the
    #: same gap in the ball-speed tile would reach the integrator as a crash instead of a reason.
    NO_LAUNCH_CONDITIONS = "no_launch_conditions"

    #: The stored result predates reasons being recorded at all. Never produced by the engine — it
    #: exists so a tolerant reader can say "this artifact does not know" instead of inventing a
    #: cause or dropping the entry, which would understate how many fundamentals a swing was judged
    #: on. Fixed by re-running the pipeline, not by re-filming.
    UNRECORDED = "unrecorded"


class ReasonSpec(NamedTuple):
    """What one reason means, and what to tell the golfer about it."""

    #: One clause, for a brief or a table cell. No leading capital and no full stop — callers
    #: embed it in a sentence of their own.
    summary: str
    #: The golfer-facing sentence. This is the whole point of the module: it is chosen by cause
    #: rather than by inference, so it can be specific without being a guess.
    remedy: str
    #: Whether shooting the clip again could plausibly fix this. The one bit every consumer needs
    #: and none could previously derive — `NO_BAND` and `NO_HANDEDNESS` are not capture problems,
    #: and telling those golfers to steady their camera answers a question they did not ask.
    refilming_helps: bool


#: Reason -> what it means and what to do. One table, so the prose cannot drift between the tip,
#: the brief, the MCP view and the results page. `tests/contracts/test_unscored.py` pins that every
#: member has a row.
UNSCORED_REASONS: dict[UnscoredReason, ReasonSpec] = {
    UnscoredReason.PHASE_NOT_SEGMENTED: ReasonSpec(
        summary="the swing could not be split into the phases this checkpoint reads",
        remedy=(
            "The swing could not be broken into its phases, so the part of it this checkpoint "
            "looks at was never located. Try a clip with the whole swing in frame, from address "
            "through the finish, and the camera steady."
        ),
        refilming_helps=True,
    ),
    UnscoredReason.BOUNDARY_ESTIMATED: ReasonSpec(
        summary="the start of the backswing was estimated rather than detected",
        remedy=(
            "The swing itself was readable - but the moment the backswing starts had to be "
            "estimated rather than detected, and timing measured from an estimate would only "
            "repeat the assumption back to you. A clip that starts with you already settled over "
            "the ball gives it something to find."
        ),
        refilming_helps=True,
    ),
    UnscoredReason.TIMING_DEGENERATE: ReasonSpec(
        summary="the detected instants give a non-positive phase duration",
        remedy=(
            "The moments this checkpoint times came out in an impossible order, which usually "
            "means a practice swing or a second movement in the clip was picked up instead. Try a "
            "clip containing one swing."
        ),
        refilming_helps=True,
    ),
    UnscoredReason.LANDMARKS_UNCONFIDENT: ReasonSpec(
        summary="the body landmarks it reads were never confidently visible in that window",
        remedy=(
            "The body points this checkpoint tracks were never clearly visible for long enough to "
            "measure. Better light, and nothing between you and the camera, is what usually fixes "
            "it."
        ),
        refilming_helps=True,
    ),
    UnscoredReason.TOO_FEW_FRAMES: ReasonSpec(
        summary="the window held fewer usable frames than the measurement needs",
        remedy=(
            "There were some usable frames but not enough of them to measure this reliably. Let "
            "the clip keep running through the finish rather than cutting it at impact."
        ),
        refilming_helps=True,
    ),
    UnscoredReason.SCALE_UNAVAILABLE: ReasonSpec(
        summary="shoulder width, the ruler this metric is expressed in, was unmeasurable",
        remedy=(
            "This checkpoint is measured in shoulder widths, and your shoulder width could not be "
            "read from this clip - usually a camera that is not square to you, or shoulders the "
            "pose model lost. Film face-on, with the camera side-on to the target line."
        ),
        refilming_helps=True,
    ),
    UnscoredReason.NO_BAND: ReasonSpec(
        summary="no benchmark band exists for it yet",
        remedy=(
            "The swing was measured fine - there is just no tour benchmark for this checkpoint "
            "yet, so there is nothing to judge the number against. Nothing to fix; the "
            "measurement is recorded either way."
        ),
        refilming_helps=False,
    ),
    UnscoredReason.NO_HANDEDNESS: ReasonSpec(
        summary=(
            "no golfer is attributed, and this checkpoint's sign depends on which side you swing "
            "from"
        ),
        remedy=(
            "The swing itself was measured fine - this checkpoint also needs to know which side "
            "you swing from, and no golfer is attributed to this swing. Pick a golfer for the "
            "session and it will score without re-filming."
        ),
        refilming_helps=False,
    ),
    UnscoredReason.CROSS_VIEW_CONTRADICTED: ReasonSpec(
        summary="the other camera, synchronized on the ball strike, contradicts its timing",
        remedy=(
            "Both cameras heard the strike, so the two clips share a real clock - and on that "
            "clock they disagree about when the backswing ended. The face-on view puts the top "
            "later than the other camera does, which would make this reading a comparison "
            "between one instant that is right and one that is not. Nothing to re-film: both "
            "clips are good, and it is the swing's own pause at the top that makes the moment "
            "hard to place."
        ),
        refilming_helps=False,
    ),
    UnscoredReason.CARRY_UNREACHABLE: ReasonSpec(
        summary="no spin flies the ball the distance the launch monitor printed",
        remedy=(
            "The launch conditions and the carry the simulator printed do not fit together: no "
            "spin rate at all makes this model fly the ball that far. Nothing to re-film and "
            "probably nothing you did - the two flight models disagree by about as much as this "
            "shot misses by."
        ),
        refilming_helps=False,
    ),
    UnscoredReason.NO_CLUB_LOFT: ReasonSpec(
        summary="two spins fly that carry and no club loft is on record to choose between them",
        remedy=(
            "Two different spin rates fly this ball exactly as far as the simulator says it went, "
            "and the club's loft is what decides which one it was. Tag this swing with the club "
            "you hit, and fill that club's loft into your bag, and it resolves without "
            "re-filming."
        ),
        refilming_helps=False,
    ),
    UnscoredReason.SPIN_NOT_RECOVERABLE: ReasonSpec(
        summary="the printed carry does not pin a spin the club could have produced",
        remedy=(
            "The carry is reachable, but not by a spin rate this club plausibly makes - either "
            "every spin above a threshold flies exactly this far, or the only one that fits is "
            "far below anything a struck golf shot spins at. Nothing to fix; a launch monitor "
            "that prints spin is what settles it."
        ),
        refilming_helps=False,
    ),
    UnscoredReason.SPIN_AXIS_UNRESOLVED: ReasonSpec(
        summary="the screen printed which way the ball curved but not how far the axis is tilted",
        remedy=(
            "Which way this shot curved is known - the simulator prints it in words - but nothing "
            "on the screen says how much the spin axis was tilted, so the flight is drawn "
            "straight and where it finished sideways is left blank. Nothing to re-film."
        ),
        refilming_helps=False,
    ),
    UnscoredReason.NO_LAUNCH_CONDITIONS: ReasonSpec(
        summary="the screen did not print enough of the launch for a flight to be simulated",
        remedy=(
            "Drawing the ball's path needs the ball speed and the launch angle, and this shot "
            "did not arrive with both of them - so there is no flight to draw. Nothing about the "
            "swing: the number never came off the photo of the simulator's screen, and re-reading "
            "that photo is what would recover it."
        ),
        refilming_helps=False,
    ),
    UnscoredReason.UNRECORDED: ReasonSpec(
        summary="this result was produced before the reason was recorded",
        remedy=(
            "This result was analyzed before the reason was recorded, so all we know is that the "
            "checkpoint could not be scored. Re-running the analysis will say why."
        ),
        refilming_helps=False,
    ),
}


#: The reasons a *measurement* may report. Everything else is produced above it — by a checkpoint
#: banding the number (`NO_BAND`, `NO_HANDEDNESS`), by a tolerant reader (`UNRECORDED`), or, since
#: M11 P8, by the two-view seam in `analysis.engine` that can see what one clip cannot
#: (`CROSS_VIEW_CONTRADICTED`).
#:
#: `analysis/measure.py` answers "what is the number" and nothing else, and this frozenset is what
#: makes that checkable rather than aspirational: a `NO_BAND` coming out of a measure function
#: would mean the split between measuring and judging had quietly closed again, which is the exact
#: fusion M6.5 spent a milestone undoing. `tests/analysis/test_measure.py` pins it.
MEASUREMENT_REASONS: frozenset[UnscoredReason] = frozenset(
    {
        UnscoredReason.PHASE_NOT_SEGMENTED,
        UnscoredReason.BOUNDARY_ESTIMATED,
        UnscoredReason.TIMING_DEGENERATE,
        UnscoredReason.LANDMARKS_UNCONFIDENT,
        UnscoredReason.TOO_FEW_FRAMES,
        UnscoredReason.SCALE_UNAVAILABLE,
    }
)


#: The reasons the **ball-flight inference** may report (ADR-027 §Decisions 3 and 5, M15 P9).
#: Nothing about a swing produces one of these; they come from `analysis.flight_infer` failing to
#: recover a launch condition the launch monitor's screen did not print.
#:
#: They are named here rather than in `analysis/` for the reason the whole module is here — `mcp`
#: and `api` have to render them and may not import `analysis` (ADR-008) — and they are a *set*
#: rather than a loose handful so the partition below stays a partition. Before M15 P9 the
#: complement of `MEASUREMENT_REASONS` was exactly the judging reasons, and
#: `tests/contracts/test_unscored.py` said so by name; a third family arriving with no name of its
#: own would have turned that assertion into a list that grows silently.
#:
#: **Four reasons over `SpinSolveCase`'s seven shapes, and the split is deliberate.** The criterion
#: here is what the *reader* must do, and for three of the four the answer is "nothing" — so the
#: seven cases collapse onto them and `UnscoredCheckpoint.detail` carries which one it was.
#: `analysis.spin_solve.SpinSolveCase` already names all seven and travels beside the result; a
#: second copy of that taxonomy in this file is a second thing to keep in step.
INFERENCE_REASONS: frozenset[UnscoredReason] = frozenset(
    {
        UnscoredReason.CARRY_UNREACHABLE,
        UnscoredReason.NO_CLUB_LOFT,
        UnscoredReason.SPIN_NOT_RECOVERABLE,
        UnscoredReason.SPIN_AXIS_UNRESOLVED,
        UnscoredReason.NO_LAUNCH_CONDITIONS,
    }
)


class UnscoredCheckpoint(BaseModel):
    """One checkpoint that was attempted and produced no score, with the reason in both forms."""

    name: str = Field(
        description=(
            "The registered checkpoint name, taken from `CHECKPOINT_REGISTRY` rather than retyped, "
            "so it cannot disagree with the `CheckpointScore` the same spec would have produced."
        )
    )
    reason: UnscoredReason
    detail: str = Field(
        default="",
        description=(
            "Which window or landmark group, in the words of the code that failed - 'no confident "
            "ear frames in the impact window'. The reason is what a consumer branches on; this is "
            "the part a human needs to tell two clips apart."
        ),
    )

    @property
    def spec(self) -> ReasonSpec:
        """The prose for this reason. A property so no consumer indexes the table by hand."""
        return UNSCORED_REASONS[self.reason]
