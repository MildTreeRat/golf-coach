"""Shared test fixtures.

Sample clips and other binary fixtures live under tests/fixtures/ (gitignored if large).
Most tests should be able to run with only the base install (no vision/ML deps), because
the `contracts` seam and the mock sources have no heavy dependencies.
"""

import os
from collections.abc import Iterator
from pathlib import Path

import pytest

from golf_coach import llm_log
from golf_coach.clubs import catalogue
from golf_coach.feedback.coach import BUILD_GOLF_MCP

REPO_ROOT = Path(__file__).resolve().parents[1]
_EXE = "golf-mcp.exe" if os.name == "nt" else "golf-mcp"


@pytest.fixture(autouse=True)
def _throwaway_club_catalogue(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Iterator[Path]:
    """Point the club catalogue at a throwaway file for **every** test in the suite. [M12 P5]

    `catalogue.remember` writes `club_catalogue.json` beside its own module, so a test that
    triggers a write without redirecting the path edits the repo's committed data and passes while
    doing it. That is not hypothetical: P5 gave the bag save route a `remember` call, and the first
    run of `tests/api/test_bag_route.py` after it committed a Titleist T150 row to the shipped
    catalogue — a test fixture's club, provenanced `"typed"`, in the file the package ships.

    Repo-wide rather than per-module because the set of modules that can reach a write is no longer
    enumerable by eye: any test exercising the bag route writes, and so will P6's. The cost of
    forgetting is a dirty working tree nobody attributes to a test run, and the only symptom of
    *not* noticing is a catalogue row that then answers every lookup of that club.

    `tests/clubs/test_catalogue.py` keeps its own fixture and its own reasons; setting the same
    attribute twice is harmless, and its `test_the_shipped_catalogue_parses…` deliberately points
    back at the real file, which still works because it re-patches after this one.
    """
    path = tmp_path / "club_catalogue.json"
    monkeypatch.setattr(catalogue, "_catalogue_path", lambda: path)
    yield path


@pytest.fixture(autouse=True)
def _throwaway_llm_log(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Iterator[Path]:
    """Point the Claude call log at a throwaway file for **every** test in the suite. [M43 P3]

    The catalogue's reason, one milestone on. Each Claude route calls `llm_log.record` once per
    request, and with no `path` that appends to `llm_calls.jsonl` under the real
    `settings.processed_dir`. So every route test that hands a route a fake client would write a
    line about a call that never happened into the log of the user's own use, and
    `llm_log summary` would count it.

    `_log_path` is the seam `llm_log` keeps for this. A test that reads the log reads this
    fixture's value. `tests/test_llm_log.py`'s test of the default path puts the real function
    back, as `test_catalogue.py` does for the shipped catalogue.
    """
    path = tmp_path / "llm_calls.jsonl"
    monkeypatch.setattr(llm_log, "_log_path", lambda: path)
    yield path


@pytest.fixture
def golf_mcp() -> Path:
    """The built server: `GOLF_MCP`, else the release build, else the debug one `cargo test` makes.

    Skipped by name, with the command that fixes it, when none is built or the `llm` extra's MCP
    client is missing; never passed without the server, because then nothing here was asserted.

    `tests/feedback/conftest.py`'s until M43 P15 moved it here, so that `tests/evals/` reaches the
    same server the stdio routes' tests do: the coach eval's cases are tested by being served.
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
