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
    wrong for two milestones.
  - `SwingResult.unscored` — `overall_score` is a mean over *survivors*, so a two-checkpoint and
    a three-checkpoint swing print the same number. The checkpoint was dropped, not failed, and
    the names say which (ADR-013).

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

from golf_coach.api.state import load_analysis, load_state
from golf_coach.contracts.alignment import AlignmentQuality
from golf_coach.contracts.caveats import (
    ALIGNMENT_CAVEAT,
    ONLY_CHECKPOINTS_ARE_JUDGED,
    PLACEMENTS_ARE_NOT_SCORES,
)
from golf_coach.contracts.placements import PLACEMENTS_BY_NAME
from golf_coach.contracts.shot import ShotData
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


class SwingView(BaseModel):
    """Everything the coach needs about one swing: mechanics, outcome, and what to distrust."""

    swing_id: str
    session_id: str
    status: str
    overall_score: float | None = None
    mechanics_score: float | None = None
    outcome_score: float | None = Field(
        default=None,
        description="None until M4 scores the shot numbers; they are attached but not judged.",
    )
    headline: str | None = None
    checkpoints: list[CheckpointView] = Field(default_factory=list)
    tips: list[TipView] = Field(default_factory=list)
    unscored: list[str] = Field(
        default_factory=list,
        description=(
            "Checkpoints that could not be measured on this swing. NOT failures — they are "
            "excluded from `overall_score` rather than counted as zero, so a swing with an "
            "unscored checkpoint is scored on fewer fundamentals than one without."
        ),
    )
    measurements: dict[str, float] = Field(
        default_factory=dict,
        description=(
            "Quantities measured off this swing that are NOT judged: no benchmark band exists for "
            "them yet, so there is no good or bad value and no percentile. Report them only if "
            f"asked for a raw number, and never say whether one is good — "
            f"{ONLY_CHECKPOINTS_ARE_JUDGED}. `face_to_path_deg` is the exception worth knowing: "
            "positive means the club face was open to its path (a fade shape), negative closed "
            "(a draw), and zero is straight regardless of either angle alone."
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
    if analysis is None:
        return SwingView(swing_id=swing_id, session_id=session_id, status=status)

    swing = _dict(analysis.get("swing"))
    feedback = _dict(analysis.get("feedback"))
    alignment = _dict(analysis.get("alignment"))
    quality, caveat = _alignment_view(alignment)

    shot_raw = swing.get("shot")
    measurements, placements = _measurements(swing)
    return SwingView(
        swing_id=swing_id,
        session_id=session_id,
        status=status,
        overall_score=_number(swing.get("overall_score")),
        mechanics_score=_number(swing.get("mechanics_score")),
        outcome_score=_number(swing.get("outcome_score")),
        headline=_text(feedback.get("headline")),
        checkpoints=[_checkpoint(c) for c in _list(swing.get("checkpoint_scores"))],
        tips=[_tip(t) for t in _list(feedback.get("tips"))],
        unscored=[str(name) for name in _list(swing.get("unscored"))],
        measurements=measurements,
        population=placements,
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
        for name in view.unscored:
            unscored_counts[name] = unscored_counts.get(name, 0) + 1
        if view.shot is not None:
            shots_attached += 1
            if view.shot.needs_review:
                shots_flagged += 1
                continue  # a flagged parse must not move an average it would silently poison
            for key, value in view.shot.metrics.items():
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


def _measurements(swing: dict[str, Any]) -> tuple[dict[str, float], list[PlacementView]]:
    """Split `SwingResult.measurements` into the flat metrics and the population placements.

    The unit and detail strings are dropped from the flat half deliberately. They are provenance
    for a derivation step, not context a coaching answer can use, and carrying them here would pad
    every reply with text the model has no way to act on.

    The placements are the exception and get their own shape, because their detail is not
    provenance but meaning — see `PlacementView`. Membership comes from
    `contracts.placements.PLACEMENTS_BY_NAME` rather than a name prefix: a `tour_` test would have
    silently reclassified anything later named that way, in the direction that loses the caveat.
    """
    flat: dict[str, float] = {}
    placements: list[PlacementView] = []
    for entry in _list(swing.get("measurements")):
        if not isinstance(entry, dict):
            continue
        name, value = entry.get("name"), _number(entry.get("value"))
        if not isinstance(name, str) or value is None:
            continue
        spec = PLACEMENTS_BY_NAME.get(name)
        if spec is None:
            flat[name] = value
            continue
        placements.append(
            PlacementView(
                name=name,
                value=value,
                # Off the stored entry, not off the spec: a swing analyzed by an older engine is
                # read back here, and this reports what that engine wrote rather than what the
                # current registry would have written.
                unit=_text(entry.get("unit")) or spec.unit,
                view=spec.view,
                calibrated=spec.calibrated,
                detail=_text(entry.get("detail")) or "",
            )
        )
    return flat, placements


def _alignment_view(alignment: dict[str, Any]) -> tuple[str | None, str | None]:
    """The alignment tier and, below `FULL`, the sentence a reader needs beside it."""
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
