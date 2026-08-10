"""Fixtures building a sessions directory the way the upload worker leaves one.

Written by hand rather than by running the pipeline: these tests are about what the query
layer *reads*, so the input has to be constructible without pose, video or OCR — which is the
same property that keeps them running on the base install.
"""

from __future__ import annotations

import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import pytest

from golf_coach.api.state import AnalysisState, input_hashes, save_state
from golf_coach.storage.manifest import (
    Role,
    RoleFile,
    SwingManifest,
    manifest_path,
    save_manifest,
)

_WHEN = datetime(2026, 8, 10, 1, 39, tzinfo=UTC)


def make_manifest(session_id: str, swing_id: str, *, roles: tuple[Role, ...] = ()) -> SwingManifest:
    return SwingManifest(
        swing_id=swing_id,
        session_id=session_id,
        created_at=_WHEN,
        updated_at=_WHEN,
        roles={
            role: RoleFile(
                role=role,
                filename=f"{role.value}.abc123.mov",
                content_sha256=f"sha-{session_id}-{swing_id}-{role.value}",
                original_filename=f"{role.value}.mov",
                content_type="video/quicktime",
                size_bytes=1234,
                received_at=_WHEN,
            )
            for role in roles
        },
    )


def make_analysis(
    session_id: str,
    swing_id: str,
    *,
    overall: float = 94.9,
    quality: str | None = "top_impact",
    unscored: list[str] | None = None,
    shot: dict[str, Any] | None = None,
) -> dict[str, Any]:
    return {
        "swing_id": swing_id,
        "session_id": session_id,
        "swing": {
            "swing_id": swing_id,
            "session_id": session_id,
            "checkpoint_scores": [
                {
                    "name": "tempo",
                    "score": 0.85,
                    "passed": False,
                    "observed": 2.42,
                    "expected_low": 2.72,
                    "expected_high": 4.71,
                    "message": "Tempo too quick - 2.4:1.",
                    "percentile": 10.0,
                    "population_n": 1399,
                    "one_sided": False,
                },
                {
                    "name": "head_sway",
                    "score": 1.0,
                    "passed": True,
                    "observed": 0.12,
                    "expected_low": 0.0,
                    "expected_high": 0.43,
                    "message": "Head stays steady.",
                    "percentile": 35.5,
                    "population_n": 458,
                    "one_sided": True,
                },
            ],
            "unscored": unscored or [],
            "mechanics_score": overall,
            "outcome_score": None,
            "overall_score": overall,
            "shot": shot,
        },
        "alignment": ({"quality": quality} if quality else None),
        "notes": ["alignment degraded: aligned on top and impact"] if quality else [],
        "feedback": {
            "swing_id": swing_id,
            "overall_score": overall,
            "headline": "Work on tempo first. Tempo too quick - 2.4:1.",
            "tips": [
                {"checkpoint": "tempo", "text": "Tempo too quick - 2.4:1.", "severity": "minor"}
            ],
        },
    }


def write_swing(
    sessions_dir: Path,
    session_id: str,
    swing_id: str,
    *,
    roles: tuple[Role, ...] = (Role.FACE_ON, Role.DOWN_THE_LINE, Role.SHOT_SCREEN),
    analysis: dict[str, Any] | None = None,
    state: bool = True,
    stale: bool = False,
) -> Path:
    """One swing directory. `state=False` mimics a CLI-analyzed swing from before Phase 5."""
    swing_dir = sessions_dir / session_id / swing_id
    swing_dir.mkdir(parents=True, exist_ok=True)

    manifest = make_manifest(session_id, swing_id, roles=roles)
    save_manifest(manifest, manifest_path(swing_dir))

    if analysis is not None:
        (swing_dir / "analysis.json").write_text(json.dumps(analysis), encoding="utf-8")

    if state and analysis is not None:
        hashes = input_hashes(manifest)
        if stale:
            hashes = {role: f"{value}-changed" for role, value in hashes.items()}
        save_state(
            AnalysisState(
                status="done",
                inputs=hashes,
                score=analysis["swing"]["overall_score"],
                headline=analysis["feedback"]["headline"],
            ),
            swing_dir,
        )
    return swing_dir


@pytest.fixture
def analysis_factory():
    """`make_analysis` as a fixture — `tests/` has no `__init__.py`, so nothing imports across
    test modules; the builders reach the tests the way pytest already passes everything else."""
    return make_analysis


@pytest.fixture
def swing_writer():
    """`write_swing` as a fixture. See `analysis_factory`."""
    return write_swing


@pytest.fixture
def sessions_dir(tmp_path: Path) -> Path:
    """Three sessions: two analyzed swings, one CLI-analyzed, one never analyzed."""
    root = tmp_path / "sessions"
    write_swing(root, "2026-08-10", "1", analysis=make_analysis("2026-08-10", "1", overall=94.9))
    write_swing(root, "2026-08-10", "2", analysis=make_analysis("2026-08-10", "2", overall=93.8))
    # No state sidecar — analyzed by scripts/analyze_bundle.py before M7 Phase 5 existed.
    write_swing(
        root,
        "2026-08-09",
        "1",
        analysis=make_analysis("2026-08-09", "1", overall=86.1),
        state=False,
    )
    # Uploaded, never analyzed, and still missing a role.
    write_swing(root, "2026-08-08", "1", roles=(Role.FACE_ON,), analysis=None)
    return root
