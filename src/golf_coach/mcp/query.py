"""Reads what the pipeline wrote, in the shapes an MCP tool returns. [M3]

Pure reads over the two stores that already exist — `sessions_dir` for analyzed swing bundles
and the `ShotDataSource` port for launch-monitor shots. Nothing here parses video, runs pose,
or touches the MCP SDK, which is what keeps it testable on the base install.

**The views are not passthroughs, and the difference is the point.** An LLM will present
whatever it is handed as fact, so anything this repo knows to be provisional has to survive the
trip out or it becomes a confident lie in a coaching answer. Three things are carried
deliberately rather than flattened away:

  - `ShotProvenance.needs_review` — a shot read off a photograph is a measurement of a
    measurement (ADR-014). OCR can drop a digit and produce a number that is wrong and perfectly
    plausible, so a flagged parse arrives labelled or not at all.
  - `AlignmentQuality` — rendering two panels implies frame correspondence everywhere and only
    `FULL` earns it (ADR-015). `alignment_caveat` spells that out in a sentence rather than
    leaving a reader to infer it from an enum value, which is exactly what the results page did
    wrong for two milestones. `SYNCHRONIZED` ranks *above* `FULL` and still carries the caveat
    (M11 P6): a measured ball strike pins one instant on a shared clock and says nothing about
    the ones between the anchors, which is precisely what the sentence warns about.
  - `SwingResult.unscored` — `overall_score` is a mean over *survivors*, so a two-checkpoint and
    a three-checkpoint swing print the same number. The checkpoint was dropped, not failed, and
    each entry says which *and why* (ADR-013, `contracts.unscored`). The why matters here more
    than anywhere: a model told only that a checkpoint is missing will supply the likeliest
    explanation itself, and "your camera moved" is a confident wrong answer to give a golfer
    whose clip was fine and whose band simply does not exist yet.

**Shots are joined by photo hash, not by session id.** `analysis.json` attaches a shot by the
sha256 of the screen photo, so a swing can legitimately carry a shot whose own `session_id`
names the session the photo was first *imported* in — session `2026-08-10` swing 1 holds a shot
stamped `2026-08-07-aaron1-1` for exactly this reason. So `get_session_summary` aggregates the
shots *attached to that session's swings* rather than querying the store by session id; the two
are not the same set, and the attached one is the one that answers "how did I hit it today".
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from pydantic import BaseModel, Field

from golf_coach.api.state import load_analysis, load_state, resolve_pivots, resolve_placements
from golf_coach.contracts.alignment import AlignmentQuality
from golf_coach.contracts.career import MODEL_SOURCE_PREFIX
from golf_coach.contracts.caveats import (
    ALIGNMENT_CAVEAT,
    ONLY_CHECKPOINTS_ARE_JUDGED,
    PIVOTS_ARE_INTERIM,
    PLACEMENTS_ARE_NOT_SCORES,
)
from golf_coach.contracts.mishit import MISHIT_EXCLUDED_METRICS, MishitVerdict
from golf_coach.contracts.pivots import PIVOTS_BY_NAME
from golf_coach.contracts.placements import PLACEMENTS_BY_NAME
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.unscored import UnscoredCheckpoint, UnscoredReason
from golf_coach.launch_monitor.source import ShotDataSource
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.manifest import load_manifest, manifest_path

# Alignment tiers below FULL get a caveat attached. Its text lives in `contracts.caveats` because
# the coaching call needs the identical sentence and ADR-008 rules out importing this module.
_CAVEAT = ALIGNMENT_CAVEAT


class NotFound(BaseModel):
    """Nothing matched — said out loud rather than by returning nothing.

    A tool that returns `None` serializes to a result with *no content blocks*, which reads the
    same as a call that silently did nothing and gives the model no way to tell "you have the
    wrong id" from "something broke". Naming the miss, and saying what to do about it, is the
    difference between a useful retry and an invented answer.

    Lives here rather than in `server.py` because since ADR-020 there are two adapters over these
    functions — the stdio server and the in-process tool runner — and a miss must read identically
    to a model whichever one it arrived through. The `missing_*` helpers below are the reason: the
    hint each one carries is the prose that turns a dead end into a retry, and it exists once.
    """

    found: bool = Field(default=False, description="Always false. This is the miss case.")
    message: str


def _missing(what: str, hint: str) -> NotFound:
    return NotFound(message=f"{what} {hint}")


def missing_swing(session_id: str, swing_id: str) -> NotFound:
    return _missing(
        f"No swing {swing_id!r} in session {session_id!r}.",
        "Call list_sessions to see which sessions and swing ids exist.",
    )


def missing_session(session_id: str) -> NotFound:
    return _missing(
        f"No session {session_id!r}.",
        "Call list_sessions to see which sessions exist.",
    )


def missing_shot(shot_id: str) -> NotFound:
    return _missing(
        f"No shot with id {shot_id!r}.",
        "Call get_recent_shots to see which shot ids exist.",
    )


class SwingSummary(BaseModel):
    """One swing as it appears in a listing — cheap enough to render a whole session from."""

    swing_id: str
    session_id: str
    status: str = Field(description="queued | running | done | failed | not analyzed.")
    score: float | None = Field(default=None, description="Overall 0-100, None until analyzed.")
    headline: str | None = Field(default=None, description="The one thing to work on first.")
    roles: list[str] = Field(default_factory=list, description="Which of the three files landed.")
    partial: bool = Field(default=False, description="Analyzed without all three roles.")


class SessionSummary(BaseModel):
    """A day at the bay, as a listing entry."""

    session_id: str
    swing_count: int
    analyzed_count: int
    swings: list[SwingSummary] = Field(default_factory=list)


class CheckpointView(BaseModel):
    """One scored fundamental, with both the band it was judged against and where it sits."""

    name: str
    observed: float | None = None
    expected_low: float | None = None
    expected_high: float | None = None
    passed: bool | None = None
    score: float | None = Field(default=None, description="0-1 within-band score.")
    percentile: float | None = Field(
        default=None,
        description=(
            "Where this sits in the tour reference population. Informational only — it is "
            "deliberately kept off the scoring path (ADR-010), and it clamps at the band edges, "
            "so every failing checkpoint reports 90 whether it missed by a hair or by triple."
        ),
    )
    population_n: int | None = Field(default=None, description="Size of that reference population.")
    message: str | None = Field(default=None, description="The plain-English coaching line.")


class TipView(BaseModel):
    checkpoint: str
    text: str
    severity: str | None = None


class ShotView(BaseModel):
    """A launch-monitor shot with its trustworthiness hoisted to the top level.

    `needs_review` and `warnings` live inside `ShotData.provenance` and are easy to miss there.
    A consumer that quotes `carry_distance` without noticing the parse was flagged is the exact
    failure ADR-014's provenance block exists to prevent, so they are surfaced here.
    """

    shot_id: str
    session_id: str
    source: str
    needs_review: bool = False
    warnings: list[str] = Field(default_factory=list)
    parse_confidence: float | None = None
    metrics: dict[str, float | str] = Field(default_factory=dict)


class PlacementView(BaseModel):
    """One population placement, with the sentence that makes it readable attached to it.

    The only member of `measurements` that keeps its `detail`, and the exception is the point.
    `_measurements` drops unit and detail because for a pose metric they are provenance for a
    derivation step — `address window -> impact window` tells a coaching answer nothing. For a
    placement the detail *is* the meaning: it carries the percentile, the size of the population
    and which part of the swing the number came from, none of which can be recovered from a bare
    float. Shipped as a float alone, `tour_trajectory_q_dtl: 11.06` reads as this swing's most
    alarming number, when on the stored corpus it is a mis-detected anchor.

    The fields are filled by `api.state.resolve_placements`, which the results page reads through
    as well — this is the MCP-shaped wrapper around it and not a second implementation. Which half
    of a row comes off the stored artifact and which off the registry is decided there.
    """

    name: str
    value: float
    unit: str
    view: str = Field(description="Which camera's fitted basis produced it — never compare across.")
    calibrated: bool = Field(
        description=(
            "False means the number is real but its rate is not: an uncalibrated placement "
            "over-flags any golfer the tour basis never saw. Read it beside its calibrated "
            "partner, never alone."
        )
    )
    detail: str


class PivotView(BaseModel):
    """One rotation measurement, with the sentence that says it is interim attached to it.

    `PlacementView`'s sibling and one step short of it: a placement is a distance from a
    reference population that exists; a pivot row has none — only the fact that a calibrated
    (fiducial-marker) source would replace it. Shipped as a float alone, `pivot_shoulder_reversal
    _backswing_deg: 41.2` reads as a measured turn, when it is a 2-D image-plane line angle at
    exactly the point ADR-029 §9 says that projection is worst-conditioned.

    The fields are filled by `api.state.resolve_pivots`, which the results page reads through as
    well and `feedback/coach.py` partitions the same list for — three surfaces, one rule, the same
    shape `PlacementView` repeats.
    """

    name: str
    value: float
    unit: str
    view: str = Field(description="Which camera produced it — never compare across.")
    interim_reason: str = Field(
        description=(
            "Why this number is provisional, in terms of what is missing — never a hedge. "
            "Fiducial calibration is the only thing that clears it."
        )
    )
    detail: str


class SimulatedView(BaseModel):
    """One number this repo modelled rather than measured, with the sentence that says so. [M15 P17]

    The second exception to `_measurements`' rule that a row is worth a name and a float, and it
    is `PlacementView`'s reason arriving from the other direction. There the detail carries the
    *meaning* a bare number loses; here it carries the **provenance**, and losing that is worse
    than losing meaning: a simulated carry reduced to `flight_carry_yds: 122.36` is
    indistinguishable from something a launch monitor printed, and the whole of ADR-027 §Decision 6
    is the argument that those two must never be poolable.

    ⚠️ **This shape exists because they were poolable here for a milestone.** M15 P11 put the six
    `flight_*` numbers into `SwingResult.measurements` with `source` and `detail` carrying the
    provenance, and P13 wrote them onto every stored artifact — but `_measurements` flattens to
    name -> value, so what reached a coaching model was six bare floats under a field description
    that calls them "quantities measured off this swing". The page had said SIMULATED on every row
    since P15 and the CLI since P7; this surface, the one that talks, said nothing.

    Membership is decided by `Measurement.source` through `contracts.career.MODEL_SOURCE_PREFIX`,
    not by the `flight_` name prefix — the same choice `_measurements` records for placements, and
    for the same reason: a name test silently reclassifies whatever gets named that way later, in
    the direction that drops the caveat. The prefix is already the repo's answer to "which
    artifact is this a reading of" (`CorpusSwing.artifact_key`), so a second model's numbers land
    here the day they exist.
    """

    name: str
    value: float
    unit: str
    source: str = Field(
        description=(
            "Which model produced it, versioned with the artifact it evaluates — "
            "`model:flight_v1`. Never an instrument."
        )
    )
    detail: str = Field(
        description=(
            "What the number is and what it is not, in the words the engine recorded it with. "
            "Read it before describing the number; SIMULATED and SOLVED are not the same claim."
        )
    )


class UnscoredView(BaseModel):
    """One checkpoint that produced no score, and why — carried, never inferred.

    `why` is prose rather than only the enum token because a coaching model is going to render a
    sentence either way; giving it the repo's sentence is what stops it writing its own. The
    `reason` stays alongside so a consumer can branch without parsing English.
    """

    name: str
    reason: str = Field(
        description=(
            "Stable token from `contracts.unscored.UnscoredReason` - branch on this, not on `why`."
        )
    )
    why: str = Field(description="What went wrong, as a clause that can be read to a golfer.")
    refilming_helps: bool = Field(
        description=(
            "Whether shooting the clip again could fix it. False means it is not a capture "
            "problem - no band exists yet, or no golfer is attributed - and telling the golfer to "
            "re-film would be answering a question they did not ask."
        )
    )


class SwingView(BaseModel):
    """Everything the coach needs about one swing: mechanics, outcome, and what to distrust."""

    swing_id: str
    session_id: str
    status: str
    club: str | None = Field(
        default=None,
        description=(
            "Which club hit this swing, or None when nothing recorded one. None is not a data "
            "error: the club tag is newer than some of the swings on file, and an untagged swing "
            "is still a real swing whose mechanics were measured normally — the club was never an "
            "input to any of them. It is only absent from per-club answers, which is what "
            "get_club_profile reports on."
        ),
    )
    mishit: str | None = Field(
        default=None,
        description=(
            "The golfer's verdict on the shot this swing hit: `confirmed` (a top or duff, so its "
            "carry and total distance are meaningless and every per-club average leaves them "
            "out), `cleared` (a real shot the automatic rule flagged, put back), or None (no "
            "verdict — the automatic rule in get_bag_profile decides). Set only through the mishit "
            "repair route or flag_mishit.py, never inferred. The mechanics are a fair sample "
            "either way; only the distance is in question (ADR-028)."
        ),
    )
    overall_score: float | None = None
    mechanics_score: float | None = None
    outcome_score: float | None = Field(
        default=None,
        description="None until M4 scores the shot numbers; they are attached but not judged.",
    )
    headline: str | None = None
    checkpoints: list[CheckpointView] = Field(default_factory=list)
    tips: list[TipView] = Field(default_factory=list)
    unscored: list[UnscoredView] = Field(
        default_factory=list,
        description=(
            "Checkpoints that could not be scored on this swing, each with the reason. NOT "
            "failures — they are excluded from `overall_score` rather than counted as zero, so a "
            "swing with an unscored checkpoint is scored on fewer fundamentals than one without. "
            "Use the reason rather than guessing at one: `refilming_helps` is false when the "
            "cause is not the golfer's camera."
        ),
    )
    measurements: dict[str, float] = Field(
        default_factory=dict,
        description=(
            "Quantities MEASURED off this swing that are NOT judged: no benchmark band exists for "
            "them, so there is no good or bad value and no percentile. Report them only if "
            f"asked for a raw number, and never say whether one is good — "
            f"{ONLY_CHECKPOINTS_ARE_JUDGED}. `face_to_path_deg` is the exception worth knowing: "
            "positive means the club face was open to its path (a fade shape), negative closed "
            "(a draw), and zero is straight regardless of either angle alone. `backswing_ms` and "
            "`downswing_ms` are a second kind of exception: they will never have a band, because "
            "tempo is already scored as the ratio of the two and a band on each half would judge "
            "one fundamental three times (ADR-023). They are still the honest answer to which "
            "*half* is off, which the ratio alone cannot tell you. Nothing simulated is in here — "
            "see `simulated`."
        ),
    )
    simulated: list[SimulatedView] = Field(
        default_factory=list,
        description=(
            "Numbers a MODEL produced for this swing rather than an instrument measuring "
            "anything: the ball flight ADR-027 integrates from the launch conditions the "
            "simulator printed. Split out of `measurements` rather than mixed into it because a "
            "simulated carry and a measured one are different kinds of quantity wearing the same "
            "unit, and the difference does not survive being reduced to a bare number. Say a "
            "number here is simulated whenever you quote one, read each row's `detail` before "
            "describing it, and never pool one with a printed figure or average the two. "
            "`simulate_flight` is the tool that returns the same flight with what it must be read "
            "with."
        ),
    )
    population: list[PlacementView] = Field(
        default_factory=list,
        description=(
            "Where this swing sits against a corpus of tour swings, as a whole rather than "
            f"checkpoint by checkpoint: {PLACEMENTS_ARE_NOT_SCORES}. Split out of `measurements` "
            "rather than mixed into it because these are the entries whose meaning does not "
            "survive being reduced to a number — read each one's `detail`, `calibrated` and "
            "`view` before saying anything about it."
        ),
    )
    rotation: list[PivotView] = Field(
        default_factory=list,
        description=(
            "How the shoulder line, the hip line and their centres moved: "
            f"{PIVOTS_ARE_INTERIM}. Split out of `measurements` for `population`'s reason — read "
            "each row's `detail` and `interim_reason` before saying anything about it, and never "
            "compare a `view` against the other one."
        ),
    )
    analysis_version: int | None = Field(
        default=None,
        description=(
            "Which generation of the engine wrote this result. Not a quality score: an older "
            "number means the swing has not been re-analyzed since the engine moved on, which is "
            "what `status: stale` cannot tell you, because staleness is about the uploads."
        ),
    )
    alignment_quality: str | None = None
    alignment_caveat: str | None = Field(
        default=None,
        description="Present whenever the two views were aligned on less than all three instants.",
    )
    shot: ShotView | None = None
    notes: list[str] = Field(
        default_factory=list, description="What the pipeline wants a reader of this result to know."
    )


class SessionDetail(BaseModel):
    """Per-session aggregates across both axes of ADR-009's model."""

    session_id: str
    swing_count: int
    analyzed_count: int
    mean_score: float | None = None
    best_score: float | None = None
    worst_score: float | None = None
    checkpoint_pass_rates: dict[str, str] = Field(
        default_factory=dict, description="checkpoint -> 'passed/measured'."
    )
    unscored_counts: dict[str, int] = Field(default_factory=dict)
    shots_attached: int = 0
    shots_needing_review: int = Field(
        default=0, description="Attached shots whose OCR parse was flagged (ADR-014)."
    )
    mishits_excluded: int = Field(
        default=0,
        description=(
            "Attached shots whose carry and total distance were left out of `shot_averages` "
            "because the golfer confirmed the swing was a mishit. Only a manual `confirmed` "
            "verdict does this here: a single session rarely holds enough of one club to spot an "
            "outlier, so the automatic rule get_bag_profile uses is not applied (ADR-028 §3). "
            "Ball speed, launch and the rest still count the shot."
        ),
    )
    shot_averages: dict[str, float] = Field(
        default_factory=dict, description="Mean of each metric over attached, trusted shots."
    )
    headlines: list[str] = Field(
        default_factory=list, description="Each analyzed swing's lead coaching point, in order."
    )


# --------------------------------------------------------------------------------------
# Sessions and swings
# --------------------------------------------------------------------------------------


def list_sessions(sessions_dir: Path, *, limit: int = 20) -> list[SessionSummary]:
    """Sessions newest-first, each with its swings.

    Reads `score` and `headline` from the `analysis.state.json` sidecar rather than opening
    `analysis.json` per swing — that denormalisation exists precisely so a listing costs one
    small read each instead of parsing an 8 KB document per row.
    """
    if not sessions_dir.exists():
        return []
    store = SwingBundleStore(sessions_dir)
    session_ids = sorted(store.list_session_ids(), reverse=True)
    out: list[SessionSummary] = []
    for session_id in session_ids[: max(0, limit)]:
        swings = [
            _summarize(manifest, sessions_dir / session_id / manifest.swing_id)
            for manifest in store.get_session(session_id)
        ]
        out.append(
            SessionSummary(
                session_id=session_id,
                swing_count=len(swings),
                analyzed_count=sum(1 for s in swings if s.score is not None),
                swings=swings,
            )
        )
    return out


def get_swing(sessions_dir: Path, session_id: str, swing_id: str) -> SwingView | None:
    """One swing's full analysis, or None if there is no such swing."""
    swing_dir = sessions_dir / session_id / swing_id
    store = SwingBundleStore(sessions_dir)
    manifest = store.get_swing(session_id, swing_id)
    if manifest is None:
        return None

    state = load_state(swing_dir)
    status = _status_of(state, swing_dir)
    analysis = load_analysis(swing_dir)
    # Off the manifest, not the analysis: the club and the mishit verdict are both stamped on the
    # swing rather than derived by the engine, so an unanalyzed swing carries them too. Both
    # returns carry them for that reason.
    club = manifest.club.value if manifest.club is not None else None
    mishit = manifest.mishit.value if manifest.mishit is not None else None

    if analysis is None:
        return SwingView(
            swing_id=swing_id, session_id=session_id, status=status, club=club, mishit=mishit
        )

    swing = _dict(analysis.get("swing"))
    feedback = _dict(analysis.get("feedback"))
    alignment = _dict(analysis.get("alignment"))
    quality, caveat = _alignment_view(alignment)

    shot_raw = swing.get("shot")
    # Resolved off the whole artifact rather than off `swing`, because `state.resolve_placements`
    # is the one definition the results page reads through too — see its docstring.
    placements = [PlacementView(**row) for row in resolve_placements(analysis)]
    pivots = [PivotView(**row) for row in resolve_pivots(analysis)]
    return SwingView(
        swing_id=swing_id,
        session_id=session_id,
        status=status,
        club=club,
        mishit=mishit,
        overall_score=_number(swing.get("overall_score")),
        mechanics_score=_number(swing.get("mechanics_score")),
        outcome_score=_number(swing.get("outcome_score")),
        headline=_text(feedback.get("headline")),
        checkpoints=[_checkpoint(c) for c in _list(swing.get("checkpoint_scores"))],
        tips=[_tip(t) for t in _list(feedback.get("tips"))],
        unscored=[_unscored(entry) for entry in _list(swing.get("unscored"))],
        measurements=_measurements(swing),
        simulated=_simulated(swing),
        population=placements,
        rotation=pivots,
        analysis_version=_version(analysis),
        alignment_quality=quality,
        alignment_caveat=caveat,
        shot=_shot_view_from_dict(shot_raw) if isinstance(shot_raw, dict) else None,
        notes=[str(n) for n in _list(analysis.get("notes"))],
    )


def get_session_summary(sessions_dir: Path, session_id: str) -> SessionDetail | None:
    """Aggregates over a session's analyzed swings and the shots attached to them.

    Returns None only when the session directory does not exist — an existing session with
    nothing analyzed yet is a real answer ("you uploaded four swings and none finished"), not
    a missing one.
    """
    if not (sessions_dir / session_id).exists():
        return None

    store = SwingBundleStore(sessions_dir)
    manifests = store.get_session(session_id)

    scores: list[float] = []
    headlines: list[str] = []
    measured: dict[str, int] = {}
    passed: dict[str, int] = {}
    unscored_counts: dict[str, int] = {}
    metric_totals: dict[str, list[float]] = {}
    shots_attached = 0
    shots_flagged = 0
    mishits_excluded = 0

    for manifest in manifests:
        view = get_swing(sessions_dir, session_id, manifest.swing_id)
        if view is None or view.overall_score is None:
            continue
        scores.append(view.overall_score)
        if view.headline:
            headlines.append(view.headline)
        for checkpoint in view.checkpoints:
            measured[checkpoint.name] = measured.get(checkpoint.name, 0) + 1
            if checkpoint.passed:
                passed[checkpoint.name] = passed.get(checkpoint.name, 0) + 1
        for entry in view.unscored:
            unscored_counts[entry.name] = unscored_counts.get(entry.name, 0) + 1
        if view.shot is not None:
            shots_attached += 1
            if view.shot.needs_review:
                shots_flagged += 1
                continue  # a flagged parse must not move an average it would silently poison
            # The manual verdict only. A session is too small to hold one club's distribution, so
            # the automatic rule read_corpus applies is deliberately not run here (ADR-028 §3) —
            # and it is scoped to the two distance keys, so this shot's ball speed and launch
            # still count.
            topped = manifest.mishit is MishitVerdict.CONFIRMED
            if topped:
                mishits_excluded += 1
            for key, value in view.shot.metrics.items():
                if topped and key in _MISHIT_SKIP:
                    continue
                if isinstance(value, float):
                    metric_totals.setdefault(key, []).append(value)

    return SessionDetail(
        session_id=session_id,
        swing_count=len(manifests),
        analyzed_count=len(scores),
        mean_score=round(sum(scores) / len(scores), 1) if scores else None,
        best_score=round(max(scores), 1) if scores else None,
        worst_score=round(min(scores), 1) if scores else None,
        checkpoint_pass_rates={
            name: f"{passed.get(name, 0)}/{count}" for name, count in sorted(measured.items())
        },
        unscored_counts=dict(sorted(unscored_counts.items())),
        shots_attached=shots_attached,
        shots_needing_review=shots_flagged,
        mishits_excluded=mishits_excluded,
        shot_averages={
            key: round(sum(values) / len(values), 1)
            for key, values in sorted(metric_totals.items())
        },
        headlines=headlines,
    )


# --------------------------------------------------------------------------------------
# Shots
# --------------------------------------------------------------------------------------


def recent_shots(source: ShotDataSource, count: int) -> list[ShotView]:
    """The most recent `count` shots across every configured source, newest first."""
    return [_shot_view(shot) for shot in source.recent(max(0, count))]


def get_shot(source: ShotDataSource, shot_id: str) -> ShotView | None:
    """One shot by id, or None.

    A linear scan of `stream()`: the port offers no lookup by id, and the screen store is keyed
    by *image* hash rather than shot id, so there is no index to consult. Fine at the scale this
    runs at (one JSON file per shot, a range session's worth at a time); revisit if the store
    ever moves to SQLite.
    """
    for shot in source.stream():
        if shot.shot_id == shot_id:
            return _shot_view(shot)
    return None


# --------------------------------------------------------------------------------------
# Internals
# --------------------------------------------------------------------------------------

#: `ShotData` fields that are metrics rather than identity/provenance bookkeeping.
#:
#: Read through `getattr(shot, field, None)`, so neither mypy nor ruff can see the coupling to
#: `contracts.shot.ShotData` — a field added there is silently absent from every MCP payload and
#: never averaged, with the suite green. `_NON_METRIC_FIELDS` below exists so the split can be
#: asserted *exhaustively* against the model (`tests/mcp/test_query.py`); without it a pin could
#: only check that these fields still exist, which is not the direction that breaks.
_METRIC_FIELDS = (
    "club_head_speed",
    "club_face_angle",
    "club_path",
    "ball_speed",
    "launch_angle",
    "launch_direction",
    "spin_rate",
    "spin_axis",
    "smash_factor",
    "carry_distance",
    "total_distance",
    "bounce_and_roll",
    "apex_height",
    "shot_type",
    "impact_position",
)

#: The rest of `ShotData` — identity, provenance and the source enum. These reach a client through
#: `ShotView`'s own named fields rather than through `metrics`, which is why they are excluded
#: rather than missing.
_NON_METRIC_FIELDS = (
    "shot_id",
    "session_id",
    "timestamp",
    "source",
    "provenance",
)

#: The `ShotView.metrics` keys a CONFIRMED mishit's shot is held out of a session average on
#: (ADR-028 §3). `contracts.mishit` names the metrics with a `_yds` suffix because those are
#: *measurement* names; the launch-monitor shot fields they are read off do not carry it. Derived
#: by stripping the suffix rather than re-typed, so this and `MISHIT_EXCLUDED_METRICS` cannot name
#: two different sets — `tests/mcp/test_query.py` pins that every name here is a real shot metric.
_MISHIT_SKIP = frozenset(name.removesuffix("_yds") for name in MISHIT_EXCLUDED_METRICS)


def _summarize(manifest: Any, swing_dir: Path) -> SwingSummary:
    state = load_state(swing_dir)
    if state is not None:
        return SwingSummary(
            swing_id=manifest.swing_id,
            session_id=manifest.session_id,
            status=_status_of(state, swing_dir),
            score=state.score,
            headline=state.headline,
            roles=sorted(role.value for role in manifest.roles),
            partial=state.partial,
        )

    # No sidecar. Either nothing has been analyzed, or it was analyzed by
    # `scripts/analyze_bundle.py` before M7 Phase 5 introduced the state file — the sessions
    # from 2026-08-07 are exactly that. Falling back to `analysis.json` costs one 8 KB parse
    # on those swings only, and the alternative is a listing that reports "not analyzed" for a
    # swing whose full result `get_swing` will happily return a moment later.
    analysis = _dict(load_analysis(swing_dir))
    return SwingSummary(
        swing_id=manifest.swing_id,
        session_id=manifest.session_id,
        status="done" if analysis else "not analyzed",
        score=_number(_dict(analysis.get("swing")).get("overall_score")),
        headline=_text(_dict(analysis.get("feedback")).get("headline")),
        roles=sorted(role.value for role in manifest.roles),
    )


def _status_of(state: Any, swing_dir: Path) -> str:
    """The swing's analysis status, with staleness folded in.

    `AnalysisState.matches` is what makes a re-uploaded clip invalidate its own result: the
    recorded input hashes stop matching the manifest, and the stored score stops describing
    the files on disk. Reporting `done` there would hand out a number for footage that is no
    longer present.
    """
    if state is None:
        return "not analyzed" if load_analysis(swing_dir) is None else "done"

    manifest = load_manifest(manifest_path(swing_dir))
    if manifest is not None and not state.matches(manifest):
        return "stale — inputs changed since this was analyzed"
    return state.status


def _measurements(swing: dict[str, Any]) -> dict[str, float]:
    """Flatten the non-placement half of `SwingResult.measurements` to name -> value.

    The unit and detail strings are dropped here deliberately. They are provenance for a
    derivation step, not context a coaching answer can use, and carrying them would pad every
    reply with text the model has no way to act on.

    The placements are the exception and leave through `state.resolve_placements` instead, because
    their detail is not provenance but meaning — see `PlacementView`. Membership comes from
    `contracts.placements.PLACEMENTS_BY_NAME` rather than a name prefix: a `tour_` test would have
    silently reclassified anything later named that way, in the direction that loses the caveat.

    **The pivot rows are the same exception for the same reason**, one step short of a placement:
    they carry no reference population at all, only the fact that they are interim. Membership is
    `contracts.pivots.PIVOTS_BY_NAME`, never a `pivot_` prefix test, for the identical reason.

    **The simulated rows are a third exception, and they leave for the opposite reason** (M15
    P17): what a bare float loses there is not the meaning but the fact that nothing measured it.
    See `SimulatedView`. A row with no `source` at all stays here — that is every artifact written
    before provenance was recorded, and calling one of those simulated on no evidence would be the
    same error pointing the other way.
    """
    flat: dict[str, float] = {}
    for entry in _list(swing.get("measurements")):
        if not isinstance(entry, dict):
            continue
        name, value = entry.get("name"), _number(entry.get("value"))
        if not isinstance(name, str) or value is None:
            continue
        if name in PLACEMENTS_BY_NAME or name in PIVOTS_BY_NAME:
            continue
        if _is_modelled(entry):
            continue
        flat[name] = value
    return flat


def _is_modelled(entry: dict[str, Any]) -> bool:
    """Whether one stored measurement row is a model's output rather than a reading."""
    source = entry.get("source")
    return isinstance(source, str) and source.startswith(MODEL_SOURCE_PREFIX)


def _simulated(swing: dict[str, Any]) -> list[SimulatedView]:
    """The `model:`-sourced half of `measurements`, with the provenance a float cannot carry.

    Unit and detail are kept here precisely where `_measurements` drops them: for a modelled
    number the detail is the sentence that stops it being read as a measurement, and it is the
    same string the results page prints and the engine stored.

    A row missing its unit or detail is still returned, with what it has. This reads artifacts,
    and an artifact from a build that recorded less is not a reason to withhold the number — it is
    a reason not to claim more about it than is on disk.
    """
    rows: list[SimulatedView] = []
    for entry in _list(swing.get("measurements")):
        if not isinstance(entry, dict) or not _is_modelled(entry):
            continue
        name, value = entry.get("name"), _number(entry.get("value"))
        if not isinstance(name, str) or value is None:
            continue
        rows.append(
            SimulatedView(
                name=name,
                value=value,
                unit=str(entry.get("unit", "")),
                source=str(entry.get("source", "")),
                detail=str(entry.get("detail", "")),
            )
        )
    return rows


def _version(analysis: dict[str, Any]) -> int | None:
    """The engine generation that wrote this artifact, or None when it does not say.

    `api.state.stored_analysis_version` reads the same field and coerces an unreadable one to 0,
    which is right for a staleness check ("old enough that I cannot tell"). Here the honest answer
    is `None`: a model told a version of 0 would report it as a number, and there is no engine 0.
    """
    version = analysis.get("analysis_version")
    return version if isinstance(version, int) and not isinstance(version, bool) else None


def _alignment_view(alignment: dict[str, Any]) -> tuple[str | None, str | None]:
    """The alignment tier and, on every tier but `FULL`, the sentence a reader needs beside it.

    `SYNCHRONIZED` is above `FULL` on the ladder and still gets the caveat — see the module
    docstring, and `AlignmentQuality.is_degraded` for why that is a different question from
    whether anything went wrong.
    """
    raw = alignment.get("quality")
    if not isinstance(raw, str):
        return None, None
    try:
        quality = AlignmentQuality(raw)
    except ValueError:
        return raw, None
    summary = _text(alignment.get("quality_summary")) or quality.summary
    if quality is AlignmentQuality.FULL:
        return quality.value, None
    return quality.value, _CAVEAT.format(summary=summary)


def _checkpoint(raw: Any) -> CheckpointView:
    data = _dict(raw)
    return CheckpointView(
        name=str(data.get("name", "")),
        observed=_number(data.get("observed")),
        expected_low=_number(data.get("expected_low")),
        expected_high=_number(data.get("expected_high")),
        passed=data.get("passed") if isinstance(data.get("passed"), bool) else None,
        score=_number(data.get("score")),
        percentile=_number(data.get("percentile")),
        population_n=data["population_n"] if isinstance(data.get("population_n"), int) else None,
        message=_text(data.get("message")),
    )


def _tip(raw: Any) -> TipView:
    data = _dict(raw)
    return TipView(
        checkpoint=str(data.get("checkpoint", "")),
        text=str(data.get("text", "")),
        severity=_text(data.get("severity")),
    )


def _unscored(raw: Any) -> UnscoredView:
    """Read one unscored entry, in either the current shape or the names-only one.

    Two old forms reach this. A bare `"tempo"` is every artifact written before 2026-08-19 and
    means "this file does not know why", which is `UNRECORDED` — the same coercion
    `SwingResult.unscored` does, restated here because this layer reads raw JSON and never builds
    the model (that is the point of a tolerant reader). A reason string this build does not
    recognise gets the same treatment rather than an exception: an unknown token from a *newer*
    artifact is still an honest "not scored, and this reader cannot say why".

    Dropping either would be the one unacceptable outcome — a swing judged on five fundamentals
    would start reading as one judged on six.
    """
    if isinstance(raw, str):
        entry = UnscoredCheckpoint(name=raw, reason=UnscoredReason.UNRECORDED)
    else:
        data = _dict(raw)
        raw_reason = str(data.get("reason", ""))
        reason = (
            UnscoredReason(raw_reason)
            if raw_reason in set(UnscoredReason)
            else UnscoredReason.UNRECORDED
        )
        entry = UnscoredCheckpoint(name=str(data.get("name", "")), reason=reason)
    return UnscoredView(
        name=entry.name,
        reason=entry.reason.value,
        why=entry.spec.summary,
        refilming_helps=entry.spec.refilming_helps,
    )


def _shot_view(shot: ShotData) -> ShotView:
    provenance = shot.provenance
    metrics: dict[str, float | str] = {}
    for field in _METRIC_FIELDS:
        value = getattr(shot, field, None)
        if isinstance(value, int | float) and not isinstance(value, bool):
            metrics[field] = float(value)
        elif isinstance(value, str):
            metrics[field] = value
    return ShotView(
        shot_id=shot.shot_id,
        session_id=shot.session_id,
        source=shot.source.value,
        needs_review=bool(provenance and provenance.needs_review),
        warnings=list(provenance.warnings) if provenance else [],
        parse_confidence=provenance.parse_confidence if provenance else None,
        metrics=metrics,
    )


def _shot_view_from_dict(raw: dict[str, Any]) -> ShotView | None:
    """The shot embedded in `analysis.json`, re-read through `ShotData`.

    Validated rather than hand-mapped so the metric list and the provenance rules stay in one
    place; a stored shot that no longer validates is dropped rather than half-reported.
    """
    try:
        return _shot_view(ShotData.model_validate(raw))
    except ValueError:
        return None


def _dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def _list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def _number(value: Any) -> float | None:
    return float(value) if isinstance(value, int | float) and not isinstance(value, bool) else None


def _text(value: Any) -> str | None:
    return value if isinstance(value, str) and value else None
