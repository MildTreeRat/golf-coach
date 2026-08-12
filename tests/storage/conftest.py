"""Builders for a sessions directory, with the content hashes under the test's control.

Modelled on `tests/mcp/conftest.py`, which builds the same directory shape by hand rather than by
running the pipeline — the property that keeps these tests on the base install. Copied rather than
imported because `tests/` has no `__init__.py`, so nothing imports across test packages; the same
reason its builders are exposed as fixtures.

The difference that earns a second copy: **every hash is a parameter**. The corpus reader's whole
job is deciding when two directories hold one swing, so a test has to be able to say "these two
swings are the same clip with a different shot photo attached" — which `make_manifest`'s derived
`sha-{session}-{swing}-{role}` cannot express.
"""

from __future__ import annotations

import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import pytest

from golf_coach.api.state import AnalysisState, input_hashes, save_state
from golf_coach.contracts.swing import ANALYSIS_VERSION
from golf_coach.storage.manifest import (
    Role,
    RoleFile,
    SwingManifest,
    manifest_path,
    save_manifest,
)

_WHEN = datetime(2026, 8, 10, 1, 39, tzinfo=UTC)


def measurement(
    name: str, value: float, *, source: str = "pose:face_on", unit: str = "shoulder_widths"
) -> dict[str, Any]:
    """One entry of `SwingResult.measurements` as the pipeline serializes it."""
    return {"name": name, "value": value, "unit": unit, "source": source, "detail": "test"}


def analysis_with(
    measurements: list[dict[str, Any]] | None = None,
    *,
    overall: float = 94.9,
    shot: dict[str, Any] | None = None,
    version: int | None = ANALYSIS_VERSION,
) -> dict[str, Any]:
    """A minimal `analysis.json`, stamped as current unless a test says otherwise.

    `version=None` omits the key entirely, which is what an artifact written before the stamp
    existed actually looks like — not `analysis_version: 0`. Passing an older integer builds the
    other case: an engine that *had* versioning and has since been superseded. Both must read as
    outdated, and only one of them can be constructed by hand at all.
    """
    return {
        **({} if version is None else {"analysis_version": version}),
        "swing": {
            "checkpoint_scores": [
                {
                    "name": "tempo",
                    "score": 0.85,
                    "passed": False,
                    "observed": 2.35,
                    "expected_low": 2.72,
                    "expected_high": 4.71,
                    "message": "Tempo too quick.",
                }
            ],
            "measurements": measurements or [],
            "unscored": [],
            "overall_score": overall,
            "shot": shot,
        },
        "notes": [],
    }


def flagged_shot(*, needs_review: bool = True) -> dict[str, Any]:
    """A `ShotData` whose OCR parse was flagged, as `analysis.json` embeds it (ADR-014)."""
    return {
        "shot_id": "test-1",
        "session_id": "2026-08-10",
        "source": "screen",
        "timestamp": _WHEN.isoformat(),
        "club_face_angle": 8.6,
        "club_path": -4.6,
        "provenance": {
            "device": "hd_golf",
            "parse_confidence": 0.31 if needs_review else 0.95,
            "needs_review": needs_review,
            "warnings": ["low confidence"] if needs_review else [],
        },
    }


def write_swing(
    sessions_dir: Path,
    session_id: str,
    swing_id: str,
    *,
    player_id: str | None = "aaron",
    face_on: str | None = "face-on-hash",
    shot_screen: str | None = "shot-hash",
    down_the_line: str | None = "dtl-hash",
    created_at: datetime = _WHEN,
    analysis: dict[str, Any] | None = None,
    stale: bool = False,
) -> Path:
    """One swing directory. A `None` hash means that role never arrived."""
    swing_dir = sessions_dir / session_id / swing_id
    swing_dir.mkdir(parents=True, exist_ok=True)

    hashes = {
        Role.FACE_ON: face_on,
        Role.DOWN_THE_LINE: down_the_line,
        Role.SHOT_SCREEN: shot_screen,
    }
    manifest = SwingManifest(
        swing_id=swing_id,
        session_id=session_id,
        created_at=created_at,
        updated_at=created_at,
        player_id=player_id,
        roles={
            role: RoleFile(
                role=role,
                filename=f"{role.value}.{digest[:12]}.mov",
                content_sha256=digest,
                original_filename=f"{role.value}.mov",
                content_type="video/quicktime",
                size_bytes=1234,
                received_at=created_at,
            )
            for role, digest in hashes.items()
            if digest is not None
        },
    )
    save_manifest(manifest, manifest_path(swing_dir))

    if analysis is not None:
        (swing_dir / "analysis.json").write_text(json.dumps(analysis), encoding="utf-8")
        inputs = input_hashes(manifest)
        if stale:
            inputs = {role: f"{value}-changed" for role, value in inputs.items()}
        save_state(AnalysisState(status="done", inputs=inputs, score=94.9), swing_dir)

    return swing_dir


@pytest.fixture
def corpus_dir(tmp_path: Path) -> Path:
    """An empty sessions root. Tests populate it with `swing`, which is the point."""
    return tmp_path / "sessions"


@pytest.fixture
def swing():
    """`write_swing` as a fixture. Builders reach tests this way because `tests/` has no
    `__init__.py`, so a plain `import conftest` would collide with `tests/conftest.py`."""
    return write_swing


@pytest.fixture
def metric():
    """`measurement` as a fixture. See `swing`."""
    return measurement


@pytest.fixture
def analysis():
    """`analysis_with` as a fixture. See `swing`."""
    return analysis_with


@pytest.fixture
def shot():
    """`flagged_shot` as a fixture. See `swing`."""
    return flagged_shot


@pytest.fixture
def pose_analysis() -> dict[str, Any]:
    """The commonest input: one analyzed swing carrying one pose measurement."""
    return analysis_with([measurement("head_sway_norm", 0.25)])
