"""`analysis.state.json` — what the worker did to a swing, and whether it is still true.

Deliberately a **sidecar**, not a field on `SwingManifest`. The manifest records what arrived
from a phone; this records what a machine later derived from it. Keeping them in separate files
means an analysis run never rewrites ingestion truth, and a corrupt or deleted state file costs
a re-analysis rather than a swing.

Two jobs beyond bookkeeping:

**Knowing when a result went stale.** `inputs` is the role -> content_sha256 map as it stood
when the run started. Re-upload a clip under the same role and its hash changes, so the recorded
inputs stop matching the manifest and the result invalidates itself. Same trick
`pipeline.keypoints_for` uses for the pose cache, applied to the whole bundle.

**Keeping the status poll cheap.** `score` and `headline` are denormalised copies. The upload
page polls every 5 seconds with every swing of the day on screen; without these, each poll would
open and parse a 7 KB `analysis.json` per swing to render one number.

Denormalised copies drift, so the invariant is enforced one level up: `api.pipeline.record_state`
writes this file as part of writing `analysis.json`, and is the only thing that writes a terminal
status. Before that, only the worker did — and a CLI re-run left the sidecar quoting a score its
own `analysis.json` had stopped agreeing with, invisibly, because `matches()` compares *inputs*
and the inputs had not changed.

This module is also where the **tolerant readers for `analysis.json` itself** live —
`load_analysis`, `stored_analysis_version`, `is_outdated`. They sit beside the state readers
rather than in `storage/` because they answer the same question from two sides: is what we
recorded about this swing still true? (`storage.corpus` imports them; see the note there.)
"""

from __future__ import annotations

import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, Field, TypeAdapter, ValidationError

from golf_coach.analysis.tempo_trainer import build_tempo_plan
from golf_coach.contracts.checkpoints import CHECKPOINT_REGISTRY
from golf_coach.contracts.placements import PLACEMENTS_BY_NAME
from golf_coach.contracts.swing import ANALYSIS_VERSION, PhaseSegment
from golf_coach.storage.manifest import SwingManifest

STATE_NAME = "analysis.state.json"

#: Re-hydrates `swing.phases` off a stored artifact. Module-level because `TypeAdapter` builds a
#: validator on construction and `resolve_tempo_plan` is called once per swing detail request.
_PHASE_LIST_ADAPTER = TypeAdapter(list[PhaseSegment])

Status = Literal["queued", "running", "done", "failed"]


class AnalysisState(BaseModel):
    """One swing's analysis lifecycle. Written at every transition, read by every poll."""

    status: Status
    #: role -> content_sha256 at the moment the run started. The staleness key.
    inputs: dict[str, str] = Field(default_factory=dict)
    queued_at: datetime | None = None
    started_at: datetime | None = None
    completed_at: datetime | None = None
    duration_seconds: float | None = None
    #: Present only when `status == "failed"`. The message a human needs, not a traceback.
    error: str | None = None
    #: True when the bundle was analyzed without all three roles — always via an explicit
    #: "analyze anyway", never automatically.
    partial: bool = False
    missing_roles: list[str] = Field(default_factory=list)
    #: Filename of the rendered video, relative to the swing dir; None when none was produced.
    video: str | None = None
    #: "avc1" plays in a browser; "mp4v" does not. The results page warns on the latter.
    video_codec: str | None = None
    score: float | None = None
    headline: str | None = None

    def matches(self, manifest: SwingManifest) -> bool:
        """Does this state still describe the files currently in the swing directory?"""
        return self.inputs == input_hashes(manifest)


def input_hashes(manifest: SwingManifest) -> dict[str, str]:
    """The role -> sha256 map that identifies exactly which bytes an analysis was run over."""
    return {role.value: file.content_sha256 for role, file in sorted(manifest.roles.items())}


def state_path(swing_dir: Path) -> Path:
    return swing_dir / STATE_NAME


def load_state(swing_dir: Path) -> AnalysisState | None:
    """The recorded state, or None if there is none or it is unreadable.

    Tolerant on purpose, mirroring `storage.manifest.load_manifest`: a half-written or
    older-schema state file should cost a re-analysis, not a 500 on the status page.
    """
    path = state_path(swing_dir)
    try:
        return AnalysisState.model_validate_json(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def save_state(state: AnalysisState, swing_dir: Path) -> None:
    """Write atomically — the status route reads this file while the worker rewrites it."""
    path = state_path(swing_dir)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".tmp")
    tmp.write_text(state.model_dump_json(indent=2), encoding="utf-8")
    tmp.replace(path)


def now() -> datetime:
    return datetime.now(tz=UTC)


def load_analysis(swing_dir: Path) -> dict | None:
    """The stored `analysis.json` as plain JSON, for handing straight to the results page.

    Deliberately not re-validated into `SwingBundleResult`: the page renders whatever the
    pipeline wrote, and a schema change should not make an old result unservable.
    """
    from golf_coach.api.pipeline import ANALYSIS_NAME

    try:
        loaded = json.loads((swing_dir / ANALYSIS_NAME).read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    return loaded if isinstance(loaded, dict) else None


def stored_analysis_version(analysis: dict | None) -> int:
    """Which generation of the engine wrote this artifact. `0` for anything before versioning.

    Read off the raw dict rather than through `SwingBundleResult` for the same reason
    `load_analysis` returns one: an artifact whose shape has moved on must stay readable. A
    non-integer or negative value is treated as 0 — "old enough that I cannot tell" and
    "definitely old" want the same repair.
    """
    if not isinstance(analysis, dict):
        return 0
    version = analysis.get("analysis_version", 0)
    if isinstance(version, bool) or not isinstance(version, int) or version < 0:
        return 0
    return version


def is_outdated(analysis: dict | None) -> bool:
    """Was this written by an engine older than the one installed now?

    The question `storage.corpus` asks before letting a swing into a metric count, and the one
    `scripts/reanalyze.py` picks its targets by. A missing analysis is **not** outdated — it is
    absent, which is a different repair with a different report (`ExclusionReason.NOT_ANALYZED`).
    """
    return analysis is not None and stored_analysis_version(analysis) < ANALYSIS_VERSION


def _measurement_entries(analysis: dict | None) -> list[dict]:
    """`swing.measurements` off a stored artifact, or empty for anything that is not that."""
    if not isinstance(analysis, dict):
        return []
    swing = analysis.get("swing")
    if not isinstance(swing, dict):
        return []
    entries = swing.get("measurements")
    if not isinstance(entries, list):
        return []
    return [entry for entry in entries if isinstance(entry, dict)]


def resolve_placements(analysis: dict | None) -> list[dict]:
    """The population placements on a stored swing, each carrying what makes it readable.

    A placement shipped as a bare float is the failure `contracts/placements.py` exists to stop:
    `tour_trajectory_q_dtl: 11.06` looks like this swing's most alarming number, when on the
    stored corpus it is a mis-detected anchor. The percentile, the size of the population and the
    "NOT calibrated" warning all live in `detail`, and none of them is recoverable from the value.

    **One resolver, two channels.** `mcp/query.py` wraps this in `PlacementView` and `api/app.py`
    hands it to the results page, so the rule for turning a stored measurement entry into a
    readable placement has exactly one definition. It lives here beside `load_analysis` for the
    reason this module's docstring gives — the tolerant readers for `analysis.json` sit together —
    and it returns plain dicts rather than a model because its two callers need different shapes
    and `contracts` holds no dict-readers.

    Membership is `PLACEMENTS_BY_NAME` and never a `tour_` prefix test: a prefix would silently
    reclassify a future name in the direction that loses the caveat.
    """
    resolved: list[dict] = []
    for entry in _measurement_entries(analysis):
        name = entry.get("name")
        if not isinstance(name, str):
            continue
        spec = PLACEMENTS_BY_NAME.get(name)
        if spec is None:
            continue
        value = entry.get("value")
        if isinstance(value, bool) or not isinstance(value, int | float):
            continue
        unit = entry.get("unit")
        detail = entry.get("detail")
        resolved.append(
            {
                "name": name,
                "value": float(value),
                # Unit and detail come off the **stored entry**, view and calibrated off the spec.
                # An artifact written by an older engine is read back here, and the two halves
                # differ in what they are: the first pair is what that engine recorded, the second
                # is what this repo knows about the model. Falling back to the spec's unit keeps a
                # row renderable when an old artifact omitted one.
                "unit": unit if isinstance(unit, str) and unit else spec.unit,
                "detail": detail if isinstance(detail, str) else "",
                "view": spec.view,
                "calibrated": spec.calibrated,
            }
        )
    return resolved


def resolve_tempo_plan(analysis: dict | None) -> dict | None:
    """A metronome for this swing, built from the tour's durations. None if it cannot be.

    Built from the stored **phases**, not from the stored measurements, and that is what makes it
    work on an artifact written before `backswing_ms` and `downswing_ms` existed (`ANALYSIS_VERSION`
    6 and earlier). A version-6 swing has every instant this needs; it simply never wrote the two
    subtractions down.

    Returns a plain dict for the same reason `resolve_placements` does — this is a route payload,
    and `contracts` holds no dict-readers. The plan itself is a real `TempoPlan`; only the
    serialization is loose.

    Tolerant in the one direction that matters: a stored `phases` list that will not validate
    yields None rather than an exception, because a results page failing to render a *whole swing*
    over an optional practice aid would be the wrong trade.
    """
    if not isinstance(analysis, dict):
        return None
    swing = analysis.get("swing")
    if not isinstance(swing, dict):
        return None
    raw_phases = swing.get("phases")
    if not isinstance(raw_phases, list):
        return None
    try:
        phases = _PHASE_LIST_ADAPTER.validate_python(raw_phases)
    except ValidationError:
        return None

    plan = build_tempo_plan(phases)
    return plan.model_dump(mode="json") if plan is not None else None


def judged_metrics(analysis: dict | None) -> list[str]:
    """Which measurement names on this swing are already shown as scored checkpoints.

    `measurements` holds every quantity the engine recorded, including the ones backing the
    checkpoint panel — so a consumer rendering it whole under "measured, not yet judged" says
    something false about the entries that *are* judged, in the table directly above it.

    Membership comes from this swing's own `checkpoint_scores` rather than from the registry
    alone, because the honest claim is "these exact numbers are judged on this page" — a
    checkpoint the engine could not score contributes nothing here. The name -> metric mapping is
    `CheckpointSpec.metric`, the one name for the number across measuring, banding and storing.
    """
    if not isinstance(analysis, dict):
        return []
    swing = analysis.get("swing")
    if not isinstance(swing, dict):
        return []
    scores = swing.get("checkpoint_scores")
    scored = {
        entry.get("name")
        for entry in (scores if isinstance(scores, list) else [])
        if isinstance(entry, dict)
    }
    return [spec.metric for spec in CHECKPOINT_REGISTRY if spec.name in scored]
