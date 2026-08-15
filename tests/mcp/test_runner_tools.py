"""The tool-runner adapter, and the seam that keeps it honest against the MCP one. [ADR-020]

The load-bearing test here is `test_both_adapters_expose_the_same_tools`. Everything else checks
that this adapter works; that one checks that it cannot *drift* from the adapter an external
client sees. A model must not be able to tell which one it is talking to — the descriptions route
it and the miss messages tell it what to do next, so a difference in either is a difference in
behaviour that no other test would catch.

Both adapters need the `llm` extra, so the whole module skips without it.
"""

from __future__ import annotations

import json
from collections.abc import Awaitable
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import pytest

from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.tool_descriptions import CAREER_TOOL_NAMES, TOOL_DESCRIPTIONS

pytest.importorskip("anthropic", reason="needs the `llm` extra")
pytest.importorskip("mcp", reason="needs the `llm` extra")

import anyio  # noqa: E402

from golf_coach.mcp.runner_tools import build_tools  # noqa: E402
from golf_coach.mcp.server import build_server  # noqa: E402


def run(coro_fn: Any, *args: Any) -> Any:
    """`anyio.run` takes an async callable, not a coroutine — same shim as `test_server.py`."""

    async def _call() -> Any:
        result: Awaitable[Any] = coro_fn(*args)
        return await result

    return anyio.run(_call)


class _EmptySource:
    def recent(self, count: int) -> list[ShotData]:
        return []

    def stream(self):
        return iter(())


@pytest.fixture
def tools(sessions_dir: Path, golfers_dir: Path):
    return {
        tool.name: tool
        for tool in build_tools(sessions_dir, _EmptySource(), golfers_dir=golfers_dir)
    }


def call(tool: Any, **kwargs: Any) -> Any:
    """Invoke a tool the way the runner does, and parse what the model would receive."""
    result = tool.call(kwargs)
    assert isinstance(result, str), (
        f"{tool.name} returned {type(result).__name__}; the runner puts the return value straight "
        "into a tool_result block, which takes a string or content blocks — not a model"
    )
    return json.loads(result)


# --------------------------------------------------------------------------- the seam


def test_both_adapters_expose_the_same_tools(sessions_dir: Path, golfers_dir: Path) -> None:
    """Names *and* descriptions, against the live MCP server rather than against the constants.

    Comparing both adapters to `TOOL_DESCRIPTIONS` separately would pass if one of them quietly
    stopped using it. Comparing them to each other is the property that actually matters.
    """
    server = build_server(sessions_dir, _EmptySource(), golfers_dir=golfers_dir)
    served = {tool.name: tool.description for tool in run(server.list_tools)}
    runner = {
        tool.name: tool.description
        for tool in build_tools(sessions_dir, _EmptySource(), golfers_dir=golfers_dir)
    }

    assert set(runner) == set(served)
    assert set(runner) == set(TOOL_DESCRIPTIONS)
    assert runner == served, "the two adapters describe the same tool differently"


def test_every_runner_tool_carries_its_full_description(tools) -> None:
    """`beta_tool` reads `__doc__` at decoration time.

    Decorating first and assigning `.description` afterwards yields an empty string with no error,
    so this asserts the whole paragraph arrived rather than merely that something did — an empty
    or truncated description is a tool the model silently never calls.
    """
    for name, tool in tools.items():
        assert tool.description == TOOL_DESCRIPTIONS[name]
        assert len(tool.description) > 150


def test_the_args_block_becomes_parameter_descriptions(tools) -> None:
    """The `Args:` section is not decoration — it is the schema a model fills in."""
    schema = tools["get_swing"].to_dict()["input_schema"]

    assert set(schema["required"]) == {"session_id", "swing_id"}
    for name in ("session_id", "swing_id"):
        assert schema["properties"][name]["description"], f"{name} reached the model undescribed"
    assert "Args:" not in tools["get_swing"].description, "the Args block leaked into the prose"


# --------------------------------------------------------------------------- behaviour


def test_the_career_tools_are_absent_without_a_registry(sessions_dir: Path) -> None:
    """Absent, not broken. A tool that answers "unknown golfer" to every name is worse."""
    names = {tool.name for tool in build_tools(sessions_dir, _EmptySource())}

    assert names == set(TOOL_DESCRIPTIONS) - CAREER_TOOL_NAMES
    assert not (names & CAREER_TOOL_NAMES)


def test_the_career_tools_are_present_with_one(tools) -> None:
    assert CAREER_TOOL_NAMES <= set(tools)


def test_list_sessions_returns_the_sessions_on_disk(tools) -> None:
    sessions = call(tools["list_sessions"])

    assert [s["session_id"] for s in sessions] == ["2026-08-10", "2026-08-09", "2026-08-08"]


def test_get_swing_returns_the_analysis(tools) -> None:
    swing = call(tools["get_swing"], session_id="2026-08-10", swing_id="2")

    assert swing["overall_score"] == 93.8
    assert [c["name"] for c in swing["checkpoints"]] == ["tempo", "head_sway"]


def test_a_miss_reads_identically_to_the_mcp_server(
    tools, sessions_dir: Path, golfers_dir: Path
) -> None:
    """The other half of the seam: a wrong id must produce the same words either way.

    `NotFound` moved to `query.py` for exactly this — the hint is what turns a dead end into a
    retry, and two copies of it would drift the first time one was reworded.
    """
    server = build_server(sessions_dir, _EmptySource(), golfers_dir=golfers_dir)
    result = run(server.call_tool, "get_swing", {"session_id": "nope", "swing_id": "9"})
    served = "".join(block.text for block in result.content if block.type == "text")

    through_runner = call(tools["get_swing"], session_id="nope", swing_id="9")

    assert through_runner["found"] is False
    assert "Call list_sessions" in through_runner["message"]
    assert through_runner["message"] in served, (
        "the two adapters word the same miss differently — the hint is what turns a dead end "
        "into a retry, and a model must get the identical one either way"
    )


def test_an_unknown_golfer_is_a_named_miss(tools) -> None:
    profile = call(tools["get_golfer_profile"], player="Nobody")

    assert profile["found"] is False
    assert "Nobody" in profile["message"]


# --------------------------------------------------------------------------- the null rule


@pytest.fixture
def refusing_tools(tmp_path: Path, golfers_dir: Path, swing_writer, career_writer):
    """Two analyzed swings across two sessions — the shape actually on disk today.

    Enough for the guard to have something to refuse *about*, which the base `sessions_dir`
    fixture does not give: its swings carry no `measurements`, so every metric is simply absent
    rather than withheld, and a null test over it would pass without proving anything.
    """
    root = tmp_path / "career-sessions"
    for index in range(2):
        swing_writer(
            root,
            f"2026-08-{index + 1:02d}",
            "1",
            player_id="aaron",
            created_at=datetime(2026, 8, index + 1, 12, tzinfo=UTC),
            analysis=career_writer({"tempo_ratio": 5.4 + index, "head_sway_norm": 0.05}),
        )
    return {
        tool.name: tool for tool in build_tools(root, _EmptySource(), golfers_dir=golfers_dir)
    }


def test_a_withheld_claim_survives_serialisation_as_null(refusing_tools) -> None:
    """The single most important property of `_json`, and the easiest one to optimise away.

    Career mode ships a refusal as `None`, and `caveats.READING_A_PERSONAL_HISTORY` tells the
    model that "null here means the guard refused the claim". Serialising with `exclude_none`
    would be the natural way to trim these payloads and it would delete the refusal, leaving only
    the per-session counts that justified it — handing a model exactly the arithmetic the guard
    exists to prevent, with nothing left in the payload saying not to do it.
    """
    profile = call(refusing_tools["get_golfer_profile"], player="Aaron")

    assert "found" not in profile, "Aaron is registered; this should not be a miss"
    assert profile["metrics"], "no metric reached the payload — nothing to withhold"

    withheld = [
        (metric["name"], key)
        for metric in profile["metrics"]
        for key, value in metric.items()
        if value is None
    ]
    assert withheld, (
        "every field survived as a value at n=2, so either the guard stopped withholding or "
        "serialisation dropped the nulls — both delete the refusal the caveats describe"
    )
    # `center` is the headline claim, and the one a model would otherwise state as a fact.
    assert any(key == "center" for _, key in withheld), (
        "the golfer's typical value was published at n=2 — that is the claim the guard exists "
        "to withhold, and it is the one a coaching answer would lead on"
    )


def test_the_trend_tool_refuses_rather_than_inventing_a_series(refusing_tools) -> None:
    """At the sample size on disk, `ready` is false and no per-session mean may be published."""
    trend = call(refusing_tools["get_shot_trends"], metric="tempo_ratio", player="Aaron")

    assert trend["ready"] is False
    assert all(session["mean"] is None for session in trend["sessions"]), (
        "a per-session mean was published while the trend was not ready — those counts are the "
        "evidence for the refusal, not a series to average"
    )
