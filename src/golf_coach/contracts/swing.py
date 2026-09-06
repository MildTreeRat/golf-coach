"""Swing analysis contract (produced by the `analysis` module, consumed by `feedback`).

`SwingResult` is the merged, analyzed view of one swing: the aligned data streams
plus the phase segmentation, per-checkpoint scores, and overall score. The `analysis`
module is a pure functional core — it turns contracts into this contract with no I/O.
"""

from __future__ import annotations

from enum import StrEnum
from typing import Any, NamedTuple

from pydantic import BaseModel, Field, field_validator

from golf_coach.contracts.alignment import SwingAlignment
from golf_coach.contracts.detections import FrameDetections
from golf_coach.contracts.feedback import FeedbackPayload
from golf_coach.contracts.intent import PracticeGoal
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.unscored import UnscoredCheckpoint, UnscoredReason


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


class CheckpointOutcome(NamedTuple):
    """What one evaluator returns: a score, or the reason there is no score.

    Was `CheckpointScore | None`. The `None` said a checkpoint went missing and nothing else, so
    the engine could only ever record its *name* — see `contracts.unscored` for what that cost
    downstream, and ADR-010's 2026-08-19 addendum for why it went uncorrected for so long.

    A tuple rather than a wider `CheckpointScore` with nullable fields, because a score and a
    refusal are different things and a shape that can be half of each invites code that reads a
    band edge off a checkpoint that has none.
    """

    #: The verdict, or None when the checkpoint could not be scored.
    score: CheckpointScore | None
    #: Which condition stopped it. None exactly when `score` is present.
    reason: UnscoredReason | None = None
    #: Which window, landmark group or lookup failed, for a human.
    detail: str = ""

    @classmethod
    def scored(cls, score: CheckpointScore) -> CheckpointOutcome:
        return cls(score)

    @classmethod
    def unscored_for(cls, reason: UnscoredReason, detail: str) -> CheckpointOutcome:
        return cls(None, reason, detail)


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

    unscored: list[UnscoredCheckpoint] = Field(
        default_factory=list,
        description=(
            "Checkpoints that were attempted but could not be scored, each with the reason — no "
            "benchmark band, unusable landmarks, a boundary that was estimated rather than "
            "detected (ADR-013), or no golfer attributed. Dropping the score is correct (ADR-010 "
            "§2: no score beats a wrong one), but dropping it *silently* is not: `overall_score` "
            "is a mean over whatever survived, so without this list a two-checkpoint swing and a "
            "three-checkpoint swing are indistinguishable. The reason decides what to tell the "
            "golfer, which is why it is carried rather than inferred downstream — see "
            "`contracts.unscored`."
        ),
    )

    @field_validator("unscored", mode="before")
    @classmethod
    def _accept_bare_names(cls, value: Any) -> Any:
        """Read the names-only form every artifact written before 2026-08-19 uses.

        One coercion here rather than a tolerant reader per consumer, and it is the whole
        back-compatibility story for this field: an old `analysis.json` says `["tempo"]` and means
        "tempo went missing and this file does not know why", which is exactly `UNRECORDED`.

        Dropping the entries instead would have been the silent failure — a stored swing judged on
        five fundamentals would start reading as one judged on six, and `overall_score` would gain
        a checkpoint it never had. Note the engine never *writes* `UNRECORDED`; seeing one means
        you are looking at an artifact that predates the reason, and `scripts/reanalyze.py` fixes
        it.
        """
        if not isinstance(value, list):
            return value
        return [
            {"name": entry, "reason": UnscoredReason.UNRECORDED}
            if isinstance(entry, str)
            else entry
            for entry in value
        ]

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
#:   3  2026-08-13 — `head_stays_back` promoted, so `overall_score` is a mean over six. Also the
#:                   first version whose panel size depends on the *swing*: the checkpoint needs
#:                   `Golfer.handedness`, so an unattributed swing is scored over five and says so
#:                   in `unscored`. Two stored distributions moved as well — `head_hip_offset_
#:                   impact_norm` was re-derived with handedness folded and two impossible readings
#:                   dropped (its `sd` was half artifact), though no shipped band was cut from it.
#: 3 -> 4 (2026-08-16, M8.1): three population placements joined `measurements` —
#:                   `tour_joint_distance`, `tour_trajectory_t2` and `tour_trajectory_q`. No
#:                   checkpoint, band or score changed, so `overall_score` is untouched on every
#:                   stored swing; the bump exists because a stored `analysis.json` from version 3
#:                   is *missing* three quantities rather than disagreeing about any, and
#:                   `reanalyze.py` is how it acquires them.
#: 4 -> 5 (2026-08-17, M8.2): the down-the-line clip is segmented on the **trail** wrist. Face-on
#:                   is untouched — same landmark, same rule, same numbers — so no checkpoint,
#:                   band or score moves. What moves is `alignment`: the lead wrist is tracked in
#:                   39% of frames from behind and the shipped rule misses the top on 30% of
#:                   GolfDB's down-the-line clips, against 7% on the trail wrist
#:                   (M4_POSE_BAKEOFF §Phase F).
#: 5 -> 6 (2026-08-17, M8.2): the down-the-line clip gets its own trajectory placement, against its
#:                   own basis, as `tour_trajectory_t2_dtl` / `_q_dtl`. Two cameras answering the
#:                   same question about different planes; deliberately never combined into one
#:                   number. Face-on is untouched and no score moves.
#: 6 -> 7 (2026-08-20, ADR-023): `backswing_ms` and `downswing_ms` joined `measurements` — the two
#:                   halves `tempo_ratio` was already built from and then divided away. No
#:                   checkpoint, band or score changed: they are measured and stored and nothing
#:                   judges them, so `overall_score` is byte-identical on every stored swing. Same
#:                   shape of bump as `3 -> 4`: a version-6 `analysis.json` is *missing* two
#:                   quantities rather than disagreeing about any, and `reanalyze.py` is how it
#:                   acquires them.
#: 7 -> 8 (2026-08-21, M9 P8): `carry_distance_yds` and `total_distance_yds` joined `measurements`
#:                   — two fields the OCR has always read, held out until a swing could say which
#:                   club hit it (`SwingManifest.club`, M9 P4). Nothing judges them: no band, no
#:                   checkpoint, no target, so `overall_score` is byte-identical on every stored
#:                   swing. Same shape of bump as `3 -> 4` and `6 -> 7` — a version-7
#:                   `analysis.json` is *missing* two quantities rather than disagreeing about any,
#:                   and `reanalyze.py` is how it acquires them.
#: 8 -> 9 (2026-08-21, M9 P9): `start_line_offline_yds` joined `measurements` — the start line
#:                   projected out to the carry, so "how many yards right" is answerable in yards
#:                   rather than degrees. Derived from two fields a version-8 artifact already
#:                   carries, which makes this the *cheapest* bump on this list: the quantity was
#:                   recoverable by hand from a version-8 file and is now recorded. Nothing judges
#:                   it against a band, so `overall_score` is byte-identical on every stored swing;
#:                   same shape as `3 -> 4`, `6 -> 7` and `7 -> 8`.
#: 9 -> 10 (2026-08-21, M9 P10): `ball_speed_mph` and `launch_angle_deg` joined `measurements` —
#:                   the two launch conditions, recorded as fitting inputs and judged by nothing:
#:                   no band, no checkpoint, no target, so `overall_score` is byte-identical on
#:                   every stored swing. Same shape of bump as `3 -> 4`, `6 -> 7` and `7 -> 8`, and
#:                   deliberately *not* the shape of `8 -> 9`: these are new reads off `ShotData`
#:                   rather than arithmetic over fields a version-9 artifact already carries, so a
#:                   version-9 file is genuinely missing them and `reanalyze.py` is how it acquires
#:                   them.
#: 10 -> 11 (2026-08-26, M10 P9): **the first bump in a while that adds no field at all.** The six
#:                   checkpoints are the same six, measured the same way, against the same bands —
#:                   over *different frames*. M10 P4-P8 changed which descent the engine calls the
#:                   swing: the down-the-line view is now selected on the trail wrist and matched
#:                   against the face-on view's downswing duration instead of choosing
#:                   independently, and the window always keeps a quiet address in front of the
#:                   takeaway. So a version-10 artifact is not *missing* a quantity the way
#:                   `3 -> 4`, `6 -> 7`, `7 -> 8`, `8 -> 9` and `9 -> 10` were — it **disagrees**,
#:                   which puts this in the same class as `2` and `3`, where `overall_score` moved
#:                   under a reader who had no way to see it. Measured instance: `2026-08-23/8`
#:                   had been scored over its whole 575-frame clip, which carried motion long past
#:                   the finish; on the selected window `finish_balance` goes 0.70 -> 1.00 and the
#:                   swing 86.5 -> 91.7. `reanalyze.py` is how a stored bundle acquires the right
#:                   frames, and until it runs `is_outdated` is the only thing standing between a
#:                   pre-M10 score and a post-M10 one in the same `PersonalBaseline`.
#: 11 -> 12 (2026-08-29, M11 P8): **the first bump whose cause is outside the clip being scored.**
#:                   Both phones record the ball strike, so a bundle that heard it in both views
#:                   has a real shared clock (ADR-025) — and with tau=2 pinned to one sound the two
#:                   downswings become two measurements of one interval, which is what finally says
#:                   *which* view's top is late rather than merely that they disagree. Three things
#:                   move as a result. The window `select_swing` picks can be overruled by a
#:                   descent the microphone heard, so a swing may be scored over different frames;
#:                   the down-the-line impact anchor moves 5-7 frames on four of the eleven
#:                   2026-08-23 bundles, which re-cuts every quantity resampled onto it
#:                   (`tour_trajectory_*_dtl`); and a `tempo` timed from a contradicted top is
#:                   **withdrawn** into `unscored` with `CROSS_VIEW_CONTRADICTED`, so
#:                   `overall_score` becomes a mean over one fewer checkpoint on the bundles that
#:                   carry it. That last one is why this is a `2`/`3`/`10 -> 11`-shaped bump and
#:                   not a `3 -> 4`-shaped one: a version-11 artifact does not *lack* a quantity,
#:                   it **disagrees** — it reports 4.92, 6.08 and 6.09:1 as scored failures where
#:                   this engine reports no tempo at all. `reanalyze.py` is how a stored bundle
#:                   acquires the corrected reading, and `is_outdated` is what keeps the two out of
#:                   one `PersonalBaseline` until it runs.
#: 12 -> 13 (2026-08-30, M11 P10/P11): the same shared clock, read correctly. Version 12 compared a
#:                   sample index against a frame index without noticing that four down-the-line
#:                   clips present their video 105-125 ms after their audio, and it anchored on a
#:                   quiet transient 2-3 frames ahead of the ball on every clip. The two errors ran
#:                   in opposite directions and partly cancelled, which is why neither was visible
#:                   in a score and why they are fixed together. So this is another `10 -> 11`
#:                   shape: no field is added and a version-12 artifact **disagrees** rather than
#:                   lacking anything — its tau=2 is up to 6 frames off in the down-the-line view
#:                   and ~3 frames early in the face-on one, which moves every quantity resampled
#:                   onto impact, the `_DOWNSWING_AGREEMENT` comparison that decides whether
#:                   `tempo` is withdrawn, and — through `select_swing` rule 0 — which descent is
#:                   scored at all. `reanalyze.py` is how a stored bundle acquires the corrected
#:                   frames; `AUDIO_DETECTOR_VERSION` (`contracts/audio.py`) is the matching gate
#:                   one layer down, because the strike lists themselves have to be re-detected
#:                   before this engine can read them right.
#: 13 -> 14 (2026-08-30): a different instrument. `settings.pose_model_variant` moved from `lite`
#:                   to `heavy`, so every landmark under every score is measured by a different
#:                   MediaPipe bundle. Another `10 -> 11` shape: no field is added and nothing is
#:                   missing from a version-13 artifact — it **disagrees**, because the variants do
#:                   not place a landmark identically and the phase instants, the metrics and the
#:                   percentiles all move with them. The bump is what keeps a lite-measured swing
#:                   and a heavy-measured one out of one `PersonalBaseline` until `reanalyze.py`
#:                   has run; `KeypointsFile.pose_estimator` (`contracts/keypoints.py`) is the
#:                   matching gate one layer down, forcing the pose itself to be re-run rather than
#:                   read from a cache keyed only on the clip. Note what this bump does *not* fix:
#:                   the bands are still cut from `mediapipe:lite`, which ADR-012 §4 says makes
#:                   them only approximately comparable to a heavy-measured swing — `api/pipeline.
#:                   py::_band_estimator_note` says so on every affected result until the corpus is
#:                   re-derived.
#: 14 -> 15 (2026-09-06, M15 P13): the `flight_*` family joined `measurements` — six numbers read
#:                   off one simulated ball flight (`analysis/flight_measure.py`'s
#:                   `FLIGHT_MEASUREMENTS`; read the membership there), integrated from the launch
#:                   conditions the shot photo printed. Same shape of bump as `3 -> 4`, `6 -> 7`,
#:                   `7 -> 8` and `9 -> 10`: a version-14 artifact is *missing* them rather than
#:                   disagreeing about anything, nothing judges them — no band, no checkpoint, no
#:                   `METRIC_TARGETS` row — so `overall_score` is byte-identical on every stored
#:                   swing, and `reanalyze.py` is how a stored bundle acquires them.
#:                   **What is new here is the provenance, not the arithmetic**: the six carry
#:                   `model:flight_v1` (ADR-027 §Decision 6), so this is the first version whose
#:                   `measurements` are not all a reading of something. Two of the six record
#:                   conditionally, and a flight the model refuses to fly is named in `unscored`
#:                   under an `INFERENCE_REASONS` reason instead — `refilming_helps` false, and
#:                   never a checkpoint that failed — so a version-15 artifact can carry *fewer*
#:                   than six of them without anything having gone wrong with the clip.
#:                   Measured over the corpus on 2026-09-06: fifteen directories re-analysed, every
#:                   `overall_score` and every `checkpoint_scores` entry byte-identical, five
#:                   artifacts carrying a flight and ten a refusal.
#:                   ⚠️ **It also swept in four measurements that owed a bump of their own.** M14's
#:                   `hand_separation_norm`, `hand_height_norm`, `hand_offset_from_hips_norm` and
#:                   `trail_hand_roll_deg` (2026-09-03) shipped under "no checkpoint, no band, no
#:                   bump" — the first two right, the third against the rule at the top of this
#:                   block and against `3 -> 4`, `6 -> 7`, `7 -> 8` and `9 -> 10`, all of which
#:                   bumped for measurements nothing judges. `is_outdated` compares this number and
#:                   nothing else, so no stored artifact carried them and nothing could tell: their
#:                   honest `n` was 0 until this bump wrote them. **A band and a version answer
#:                   different questions** — *does anything judge this* and *does a stored artifact
#:                   still mean what it says* — and a phase that adds a measurement answers both.
ANALYSIS_VERSION = 15


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
