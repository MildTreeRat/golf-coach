"""Swing analysis contract (produced by the `analysis` module, consumed by `feedback`).

`SwingResult` is the merged, analyzed view of one swing: the aligned data streams
plus the phase segmentation, per-checkpoint scores, and overall score. The `analysis`
module is a pure functional core — it turns contracts into this contract with no I/O.
"""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel, Field

from golf_coach.contracts.alignment import SwingAlignment
from golf_coach.contracts.detections import FrameDetections
from golf_coach.contracts.feedback import FeedbackPayload
from golf_coach.contracts.intent import PracticeGoal
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.shot import ShotData


class SwingPhase(StrEnum):
    """The six segments of a golf swing (ROADMAP M4)."""

    ADDRESS = "address"
    BACKSWING = "backswing"
    TRANSITION = "transition"
    DOWNSWING = "downswing"
    IMPACT = "impact"
    FOLLOW_THROUGH = "follow_through"


class PhaseSegment(BaseModel):
    """A contiguous span of frames belonging to one swing phase."""

    phase: SwingPhase
    start_frame: int = Field(ge=0)
    end_frame: int = Field(ge=0)
    start_ms: float = Field(ge=0.0)
    end_ms: float = Field(ge=0.0)
    detected: bool = Field(
        default=True,
        description=(
            "False when this boundary is an estimate rather than something found in the signal. "
            "A detector that fails should say so rather than return a plausible-looking number "
            "(ADR-013): the estimate is good enough to place a measurement window, but a consumer "
            "that *divides* by the boundary — tempo does — must drop its score instead of "
            "reporting one. Defaults True so a segment nobody flagged reads as detected."
        ),
    )


class CheckpointScore(BaseModel):
    """Result of evaluating one swing checkpoint (e.g. 'address posture')."""

    name: str
    score: float = Field(ge=0.0, le=1.0, description="0=fail .. 1=ideal")
    passed: bool
    observed: float | None = Field(default=None, description="Measured value.")
    expected_low: float | None = None
    expected_high: float | None = None
    message: str = ""

    # Where `observed` sits in the reference population (M4-REF `golfdb_v1.json`), as opposed to
    # `score`, which only says how far outside the *band* it fell. These are **informational and
    # never affect `score` or `passed`** — scoring reads `ranges.json` and nothing else (ADR-010 §2
    # and its percentile addendum). They exist because `score` is not comparable across checkpoints:
    # `_score_within_range` decays in band-widths, and the bands are 1.99, 0.43 and 0.29 wide, so a
    # 0.6 means three different real-world things. A percentile is the common currency that lets
    # `feedback` rank tips and say which fault to work on first.
    percentile: float | None = Field(
        default=None,
        ge=0.0,
        le=100.0,
        description=(
            "Percentile of `observed` within the reference population, or None when no "
            "distribution covers this metric. Clamped to [10, 90] by `Distribution.percentile_of` "
            "because the tails were never stored — read 90 as 'at or beyond the 90th', not as a "
            "precise rank."
        ),
    )
    population_n: int | None = Field(
        default=None,
        ge=0,
        description="Sample size behind `percentile`, so a reader can weigh how much it is worth.",
    )
    one_sided: bool = Field(
        default=False,
        description=(
            "True when lower is strictly better (head sway, finish balance) and the band is "
            "`[0, high]`; False when both tails are faults (tempo). Consumers need this to turn a "
            "percentile into a distance from *ideal* — at the 10th percentile a one-sided metric "
            "is excellent and a two-sided one is as wrong as it is at the 90th."
        ),
    )


class Measurement(BaseModel):
    """A quantity measured off one swing, carrying no claim about whether it is good.

    Deliberately **not** a `CheckpointScore`: no band, no `passed`, no `score`. The distinction is
    the point. A metric earns a verdict only once a population distribution exists to judge it
    against (ADR-010 §2 — no score beats a wrong one), and until then it is data. Making that a
    *type* rather than a convention means a measurement is structurally incapable of being rendered
    as a fault, however it travels.

    Recording them before they can be judged is what breaks the chicken-and-egg that kept this
    panel at three checkpoints: bands are derived from a population of measurements
    (`scripts/golfdb/derive_pose_metrics.py` -> `derive_reference.py`), so a metric that cannot be
    measured without a band can never acquire one. It is also why a session's swings stay worth
    re-analyzing later — the measurement is on disk even though nothing yet knows what it means.
    """

    name: str
    value: float
    unit: str = Field(
        description="'shoulder_widths' | 'degrees' | 'ratio' | 'mph' | 'yards' | 'rpm'."
    )
    source: str = Field(description="Where it came from: 'pose:face_on', 'launch_monitor:hd_golf'.")
    detail: str = Field(
        default="",
        description=(
            "How and where it was taken — 'address window -> impact window', and any sign "
            "convention. A bare number cannot be re-derived or re-normalized later; this is what "
            "makes it auditable when a band is eventually cut from it."
        ),
    )


class SwingResult(BaseModel):
    """The complete analyzed result for one swing."""

    swing_id: str
    session_id: str

    phases: list[PhaseSegment] = Field(default_factory=list)
    checkpoint_scores: list[CheckpointScore] = Field(default_factory=list)

    measurements: list[Measurement] = Field(
        default_factory=list,
        description=(
            "Quantities measured off this swing that are **not** judged — no band, no score, no "
            "pass/fail. A consumer must never render these as verdicts or infer one from them; "
            "they exist so bands can be derived from a real population later, and so a swing "
            "captured today is still worth re-reading once they are. Independent of "
            "`checkpoint_scores`: a metric can be measured here and scored there, or measured "
            "here and scored nowhere."
        ),
    )

    unscored: list[str] = Field(
        default_factory=list,
        description=(
            "Checkpoints that were attempted but could not be measured — no benchmark band, "
            "unusable landmarks, or a boundary that was estimated rather than detected (ADR-013). "
            "Dropping the score is correct (ADR-010 §2: no score beats a wrong one), but dropping "
            "it *silently* is not: `overall_score` is a mean over whatever survived, so without "
            "this list a two-checkpoint swing and a three-checkpoint swing are indistinguishable. "
            "Names only — the reason would need the evaluators to return one instead of None."
        ),
    )

    # Dual-axis scoring (ADR-009). The practice intent this swing was judged against,
    # plus the two independent sub-scores. `overall_score` is the policy-weighted blend
    # (for the Fundamentals PoC: overall == mechanics_score, outcome_score is None).
    intent: PracticeGoal | None = None
    mechanics_score: float | None = Field(default=None, ge=0.0, le=100.0)
    outcome_score: float | None = Field(default=None, ge=0.0, le=100.0)
    overall_score: float = Field(ge=0.0, le=100.0, description="0-100 policy-weighted blend.")

    # The merged source data this result was computed from. Optional so a lightweight
    # SwingResult (e.g. from storage, scores only) can omit the heavy streams.
    keypoints: list[FrameKeypoints] = Field(default_factory=list)
    detections: list[FrameDetections] = Field(default_factory=list)
    shot: ShotData | None = None


#: The generation of the analysis engine. Stamped onto every `SwingBundleResult` the pipeline
#: writes, so a stored artifact can say which code produced it.
#:
#: **Bump it whenever the engine's output changes *meaning*** — a new measurement, a corrected
#: phase instant, a re-cut benchmark band. Not for refactors, and not for anything that leaves
#: every number where it was.
#:
#: The reason this exists is career mode, where a `PersonalBaseline` reads a golfer's *spread*
#: across sessions. Two swings analyzed by different generations of this engine are not
#: comparable, and the difference is invisible in the artifact: the 2026-08-09 `_DRAWDOWN_FLOOR`
#: fix moved a stored tempo from 0.43 to 2.42 without changing the shape of anything. Mixing them
#: would manufacture variance out of a code change — the same class of error as counting a
#: re-uploaded clip twice, which `storage.corpus` already refuses.
#:
#:   0  the artifact predates versioning (no `analysis_version` key at all)
#:   1  2026-08-12 — first versioned engine: M6.5 `measurements` present
#:   2  2026-08-12 — `hip_sway` and `hip_shift_at_top` promoted to scored checkpoints, so
#:                   `overall_score` is a mean over five and no longer comparable with a mean
#:                   over three. The measurements did not move; what they *mean* did.
ANALYSIS_VERSION = 2


class SwingBundleResult(BaseModel):
    """One swing analyzed from the two camera views plus its shot photo. [M7 Phase 4]

    A *bundle* is what the storage layer assembles from two phones and a screen photo: a face-on
    clip, a down-the-line clip, and a picture of the launch monitor. This is the whole verdict on
    one, and it is what gets serialized for a results page to render.

    The two views are not equals and this shape says so. **Only the face-on view is scored** —
    it is the canonical pose angle the three checkpoints were validated against, and the GolfDB
    corpus the benchmark bands came from is face-on, so a down-the-line metric would have no
    reference data to be judged against. The down-the-line clip contributes alignment anchors
    and nothing else (ADR-015; M7 Phase 4).

    Every frame index in here — `swing.phases`, `alignment`'s anchors — addresses the **whole**
    clip, even when a window was used to find the swing inside a longer recording. The window is
    a search restriction, not a coordinate system.
    """

    swing_id: str
    session_id: str

    swing: SwingResult = Field(
        description="The face-on view's scored result, with the shot attached if one was found."
    )

    analysis_version: int = Field(
        default=0,
        description=(
            "Which generation of the engine produced this (`ANALYSIS_VERSION` above). **Defaults "
            "to 0, not to the current version, and that is load-bearing**: this field is read "
            "back by parsing artifacts written before it existed, and a default of 'current' "
            "would make every legacy file claim to be up to date — the one wrong answer that "
            "cannot be recovered from, since it is indistinguishable from a correct one. The "
            "pipeline sets it explicitly; anything that reads 0 was written by an older engine "
            "and should be re-run rather than compared against."
        ),
    )

    alignment: SwingAlignment | None = Field(
        default=None,
        description=(
            "How the two views correspond, or None when there was no usable down-the-line clip. "
            "Read `alignment.quality` before presenting the side-by-side video as synchronized — "
            "rendering two panels implies frame correspondence everywhere and only FULL earns it."
        ),
    )

    face_on_window: tuple[int, int] | None = Field(
        default=None,
        description=(
            "The `[start, end)` frame range the swing was found in, when the clip held more than "
            "one. Recorded because it is not cosmetic: it decides which frames were scored."
        ),
    )
    down_the_line_window: tuple[int, int] | None = None

    notes: list[str] = Field(
        default_factory=list,
        description=(
            "Everything that degraded, in order of discovery — a missing view, an unmeasurable "
            "anchor, a shot flagged for review, a checkpoint whose input is not physically "
            "possible. A consumer that renders the score and ignores this is exactly the silent "
            "failure ADR-013 and ADR-014 were written against."
        ),
    )

    feedback: FeedbackPayload | None = Field(
        default=None,
        description=(
            "Ranked coaching tips. Left None by `analysis`, which must not import `feedback` "
            "(ADR-008: modules depend on `contracts` and never on each other) — the caller fills "
            "it in with `feedback.build_feedback(result.swing)` so the serialized artifact is the "
            "complete result rather than something a reader has to recompute. Same shape of seam "
            "as `SwingResult.shot`."
        ),
    )
