"""The MCP adapter: the extras boundary, and the tool declarations themselves.

The tool *behaviour* is `query`'s and is tested there. What is left here is the contract the
model actually sees — names, schemas, descriptions — plus the guarantee that needing the SDK to
serve does not mean needing it to read.

The SDK's tool API is async and this repo has no async test plumbing, so the handful of
coroutines here are driven through `anyio.run` rather than pulling in a pytest plugin for five
tests. `anyio` is guaranteed present wherever these run: the MCP SDK depends on it, and the
whole module skips without the SDK.
"""

from __future__ import annotations

import subprocess
import sys
from collections.abc import Awaitable
from pathlib import Path
from typing import Any, TypeVar

import pytest

from golf_coach.contracts.checkpoints import CHECKPOINT_REGISTRY
from golf_coach.contracts.shot import ShotData

pytest.importorskip("mcp", reason="needs the `llm` extra")

import anyio  # noqa: E402

from golf_coach.mcp.server import build_server  # noqa: E402

T = TypeVar("T")


def run(coro_fn: Any, *args: Any) -> Any:
    """`anyio.run` takes an async callable, not a coroutine — wrap the call site once."""

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
def server(sessions_dir: Path):
    return build_server(sessions_dir, _EmptySource())


def test_every_planned_tool_is_declared(server) -> None:
    names = {tool.name for tool in run(server.list_tools)}

    assert names == {
        "list_sessions",
        "get_swing",
        "get_session_summary",
        "get_recent_shots",
        "get_shot_by_id",
    }


def test_every_tool_has_a_substantive_description(server) -> None:
    """A thin description is the most common cause of a tool never being called.

    The bar is deliberately about *when to call*, not just what it returns, so a one-liner
    fails it.
    """
    for tool in run(server.list_tools):
        assert tool.description, f"{tool.name} has no description"
        assert len(tool.description) > 150, f"{tool.name}'s description is too thin to route on"
        assert "Call this" in tool.description, f"{tool.name} never says when to call it"


def test_tool_schemas_require_the_ids_they_cannot_default(server) -> None:
    by_name = {tool.name: tool for tool in run(server.list_tools)}

    assert by_name["get_swing"].input_schema["required"] == ["session_id", "swing_id"]
    assert by_name["get_session_summary"].input_schema["required"] == ["session_id"]
    assert by_name["get_shot_by_id"].input_schema["required"] == ["shot_id"]
    # The two listing tools are callable with no arguments at all.
    assert not by_name["list_sessions"].input_schema.get("required")
    assert not by_name["get_recent_shots"].input_schema.get("required")


def test_get_swing_round_trips_through_the_server(server) -> None:
    result = run(server.call_tool, "get_swing", {"session_id": "2026-08-10", "swing_id": "1"})

    text = "".join(block.text for block in result.content if block.type == "text")
    assert '"overall_score": 94.9' in text
    assert "tempo" in text


def test_a_miss_says_so_instead_of_returning_nothing(server) -> None:
    """A `None` return serializes to zero content blocks, which reads as a call that did nothing.

    The model cannot tell that from a broken tool, so a miss has to name itself and point at
    the tool that would produce a valid id.
    """
    result = run(server.call_tool, "get_swing", {"session_id": "2026-08-10", "swing_id": "99"})

    assert not result.is_error
    text = "".join(block.text for block in result.content if block.type == "text")
    assert text, "a miss returned no content at all"
    assert '"found": false' in text
    assert "list_sessions" in text


def test_every_miss_points_at_the_tool_that_lists_valid_ids(server) -> None:
    misses = {
        "get_swing": {"session_id": "nope", "swing_id": "nope"},
        "get_session_summary": {"session_id": "nope"},
        "get_shot_by_id": {"shot_id": "nope"},
    }

    for name, args in misses.items():
        result = run(server.call_tool, name, args)
        text = "".join(block.text for block in result.content if block.type == "text")
        assert '"found": false' in text, f"{name} did not report a miss"
        assert "Call " in text, f"{name}'s miss does not say what to do next"


def test_the_instructions_carry_the_standing_caveats(server) -> None:
    """These are the misreadings no single tool description can prevent."""
    instructions = server.instructions or ""

    assert "unscored" in instructions
    assert "needs_review" in instructions
    assert "percentile" in instructions
    # The panel is a fixed set of face-on checkpoints; the model must not invent the rest.
    assert "swing plane" in instructions

    # The count and the names come off `CHECKPOINT_REGISTRY`, and this is the assertion that says
    # what breaks if they stop: these instructions are what an MCP client tells its model the
    # system measures. They claimed five checkpoints for the whole of M6.5 while six ran.
    for spec in CHECKPOINT_REGISTRY:
        assert spec.label in instructions, (
            f"{spec.name} ships as a checkpoint but the MCP instructions never name it"
        )


def test_reading_the_data_does_not_require_the_mcp_sdk() -> None:
    """`query` must import on the base install — the same boundary `api.pipeline` holds.

    A subprocess, because this test module has already imported the SDK and an in-process
    `sys.modules` check would pass regardless.
    """
    code = "import golf_coach.mcp.query, sys; print('mcp' in sys.modules)"

    out = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True, check=True)

    assert out.stdout.strip() == "False", (
        "importing golf_coach.mcp.query pulled in the MCP SDK — that breaks the query layer "
        "on an install without the `llm` extra"
    )
