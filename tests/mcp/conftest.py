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
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.swing import ANALYSIS_VERSION
from golf_coach.storage.golfer_store import GolferStore
from golf_coach.storage.manifest import (
    Role,
    RoleFile,
    SwingManifest,
    manifest_path,
    save_manifest,
)

_WHEN = datetime(2026, 8, 10, 1, 39, tzinfo=UTC)


def make_manifest(
    session_id: str,
    swing_id: str,
    *,
    roles: tuple[Role, ...] = (),
    player_id: str | None = None,
    created_at: datetime = _WHEN,
) -> SwingManifest:
    return SwingManifest(
        swing_id=swing_id,
        session_id=session_id,
        created_at=created_at,
        updated_at=created_at,
        player_id=player_id,
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
    measurements: list[dict[str, Any]] | None = None,
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
            "measurements": measurements or [],
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
    player_id: str | None = None,
    created_at: datetime = _WHEN,
) -> Path:
    """One swing directory. `state=False` mimics a CLI-analyzed swing from before Phase 5.

    `player_id` and `created_at` exist for the career tools (step 6): a swing naming no golfer is
    excluded from every corpus as `UNATTRIBUTED`, and `created_at` is what a trend is keyed on, so
    both have to be settable per swing rather than fixed at `_WHEN`.
    """
    swing_dir = sessions_dir / session_id / swing_id
    swing_dir.mkdir(parents=True, exist_ok=True)

    manifest = make_manifest(
        session_id, swing_id, roles=roles, player_id=player_id, created_at=created_at
    )
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


# --------------------------------------------------------------------------------------
# Career mode (step 6): a corpus with a golfer attached, and one big enough to speak
# --------------------------------------------------------------------------------------

#: The eight metrics an analyzed swing records, with the provenance each one's `n` is keyed on.
#: `source` is not decoration here — it decides whether a sample counts distinct face-on clips or
#: distinct shot photos (`CorpusSwing.artifact_key`).
CAREER_METRICS: dict[str, tuple[str, str]] = {
    "head_sway_norm": ("pose:face_on", "shoulder_widths"),
    "hip_sway_norm": ("pose:face_on", "shoulder_widths"),
    "hip_shift_at_top_norm": ("pose:face_on", "shoulder_widths"),
    "head_hip_offset_impact_norm": ("pose:face_on", "shoulder_widths"),
    "finish_balance_norm": ("pose:face_on", "shoulder_widths"),
    "tempo_ratio": ("pose:face_on", "ratio"),
    "face_to_path_deg": ("launch_monitor:hd_golf", "degrees"),
    "start_line_deg": ("launch_monitor:hd_golf", "degrees"),
}


def career_analysis(values: dict[str, float], *, overall: float = 91.0) -> dict[str, Any]:
    """An `analysis.json` carrying measurements and stamped with the current engine version.

    Both are required for a swing to reach a baseline at all: `measurements` is what
    `build_baseline` pools, and a missing `analysis_version` reads as **0**, which `read_corpus`
    correctly excludes as OUTDATED. A career fixture without the stamp tests the exclusion path
    and nothing else, silently.
    """
    payload = make_analysis("s", "1", overall=overall)
    payload["analysis_version"] = ANALYSIS_VERSION
    payload["swing"]["measurements"] = [
        {
            "name": name,
            "value": value,
            "unit": CAREER_METRICS[name][1],
            "source": CAREER_METRICS[name][0],
            "detail": "test",
        }
        for name, value in values.items()
    ]
    return payload


@pytest.fixture
def golfers_dir(tmp_path: Path) -> Path:
    """A registry holding one golfer, created the way the upload page creates one."""
    root = tmp_path / "golfers"
    GolferStore(root).get_or_create("Aaron", Handedness.RIGHT)
    return root


@pytest.fixture
def career_writer():
    """`career_analysis` as a fixture. See `analysis_factory`."""
    return career_analysis


@pytest.fixture
def career_metrics() -> dict[str, tuple[str, str]]:
    """`CAREER_METRICS` as a fixture. See `analysis_factory`."""
    return CAREER_METRICS
