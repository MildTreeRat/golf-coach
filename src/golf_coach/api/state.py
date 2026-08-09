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
"""

from __future__ import annotations

import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, Field

from golf_coach.storage.manifest import SwingManifest

STATE_NAME = "analysis.state.json"

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
