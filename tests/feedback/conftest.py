"""What the two stdio routes' tests share: the built `golf-mcp`, and a stored swing for it to read.

[M29 P16–P17] Both `conversation.py`'s route and `coach.py`'s JSON entry are tested against the real
server rather than a fake one, because what they must hold is what `golf-mcp` actually answers.
"""

from __future__ import annotations

import json
import os
from collections.abc import Callable
from datetime import UTC, datetime
from pathlib import Path

import pytest

from golf_coach.contracts.swing import SwingBundleResult
from golf_coach.feedback.coach import BUILD_GOLF_MCP
from golf_coach.storage.manifest import SwingManifest, manifest_path, save_manifest

REPO_ROOT = Path(__file__).resolve().parents[2]
_EXE = "golf-mcp.exe" if os.name == "nt" else "golf-mcp"


@pytest.fixture
def golf_mcp() -> Path:
    """The built server: `GOLF_MCP`, else the release build, else the debug one `cargo test` makes.

    Skipped by name, with the command that fixes it, when none is built or the `llm` extra's MCP
    client is missing; never passed without the server, because then nothing here was asserted.
    """
    pytest.importorskip("mcp", reason="the stdio route needs the `llm` extra's MCP client")
    pytest.importorskip("anthropic", reason="the stdio route builds its tools with the SDK")
    named = os.environ.get("GOLF_MCP")
    candidates = [Path(named)] if named else []
    candidates += [REPO_ROOT / "target" / profile / _EXE for profile in ("release", "debug")]
    found = next((path for path in candidates if path.is_file()), None)
    if found is None:
        pytest.skip(f"golf-mcp is not built: run `{BUILD_GOLF_MCP}`")
    return found


def _store(sessions_dir: Path, result: SwingBundleResult) -> Path:
    """`result` as the pipeline stores it: a manifest, and `analysis.json` beside it."""
    swing_dir = sessions_dir / result.session_id / result.swing_id
    swing_dir.mkdir(parents=True, exist_ok=True)
    when = datetime(2026, 8, 10, 15, 0, tzinfo=UTC)
    manifest = SwingManifest(
        swing_id=result.swing_id, session_id=result.session_id, created_at=when, updated_at=when
    )
    save_manifest(manifest, manifest_path(swing_dir))
    analysis = result.model_dump(mode="json", exclude={"keypoints", "detections"})
    (swing_dir / "analysis.json").write_text(json.dumps(analysis), encoding="utf-8")
    return swing_dir


@pytest.fixture
def store_swing() -> Callable[[Path, SwingBundleResult], Path]:
    """Write one analyzed swing under a sessions directory, for `golf-mcp` to read back."""
    return _store
