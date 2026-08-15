"""The same tools, for the in-process tool runner instead of the wire. [M6, ADR-020]

`server.py`'s sibling. Both are thin adapters over `query.py` and `career.py`; the difference is
only who is calling. `server.py` answers an **external** client — Claude Desktop, Claude Code —
over stdio, and that round trip is the point there. This module answers *this application*, which
already has the functions importable, so it calls them directly and skips the subprocess.

Three things are deliberately shared with `server.py` rather than restated, because a model
must not be able to tell which adapter it is talking to:

  - the **descriptions**, from `contracts/tool_descriptions.py` — a model picks a tool by reading
    these and nothing else;
  - the **miss shapes**, from `query.missing_*` and `career.missing_golfer_*` — a wrong id has to
    read as an answer to retry, identically, either way;
  - the **briefing**, from `server.instructions()`, which the caller passes to the conversation.

What is *not* shared is the schema: each adapter derives its own from its own function signature,
which mypy and the tests check. That duplication is accepted and the reasoning is in ADR-020 — a
signature is checkable, prose is not.

**Tool functions must return `str`.** The runner puts the return value straight into the
`tool_result` block's `content`, which accepts a string or content blocks — not a pydantic model.
So every tool here serialises its own view, and `_json` is the only place that happens.

This module imports `anthropic` at module scope, the same way `server.py` imports the MCP SDK.
Both are the `llm` extra, and both are kept out of `query.py` so the reading half still runs on a
base install — `tests/api/test_pipeline_imports.py` pins that.
"""

from __future__ import annotations

import json
from collections.abc import Sequence
from pathlib import Path
from typing import Any

from anthropic import beta_tool
from anthropic.lib.tools import BetaFunctionTool
from pydantic import BaseModel

from golf_coach.contracts.tool_descriptions import (
    COMPARE_SESSIONS,
    GET_GOLFER_PROFILE,
    GET_RECENT_SHOTS,
    GET_SESSION_SUMMARY,
    GET_SHOT_BY_ID,
    GET_SHOT_TRENDS,
    GET_SWING,
    LIST_SESSIONS,
)
from golf_coach.launch_monitor.source import ShotDataSource
from golf_coach.mcp import career, query


def _json(value: BaseModel | Sequence[BaseModel]) -> str:
    """One view, or a list of them, as the JSON string the runner puts in a `tool_result`.

    **`None` fields are kept.** Dropping them would be the natural way to trim a payload and it
    would be a bug: career mode ships a withheld claim as `None`, and `caveats` tells the model
    that "null here means the guard refused the claim". Excluding nulls would delete the refusal
    and leave only the counts that justified it — which is precisely the arithmetic the refusal
    exists to prevent.
    """
    if isinstance(value, BaseModel):
        return value.model_dump_json()
    return json.dumps([item.model_dump(mode="json") for item in value])


def _tool(fn: Any, description: str, **params: str) -> BetaFunctionTool[Any]:
    """Attach the shared description, then build the tool.

    The `__doc__` assignment has to happen **before** `beta_tool` runs: the description is read at
    decoration time, so decorating first and setting `.description` afterwards produces an empty
    description with no error. Verified against `anthropic` 0.121.0 and recorded in ADR-020,
    because the failure mode is a tool the model never calls and nothing that says why.

    The `Args:` block becomes the per-parameter schema descriptions. Those are written here rather
    than in `contracts/tool_descriptions.py` because the MCP adapter derives its schema from its
    own signature and never sees them — they describe *this* function's arguments, not the tool.
    """
    args = "".join(f"    {name}: {text}\n" for name, text in params.items())
    fn.__doc__ = f"{description}\n\nArgs:\n{args}" if args else description
    return beta_tool(fn)


def build_tools(
    sessions_dir: Path,
    shot_source: ShotDataSource,
    *,
    golfers_dir: Path | None = None,
) -> list[BetaFunctionTool[Any]]:
    """The tools a conversation may call, bound to one set of stores.

    Injected rather than read from `settings` here, exactly as `build_server` does it, so a test
    points at a tmp path and a mock source without touching the environment.

    **`golfers_dir=None` removes the three career tools rather than breaking them**, the same
    contract `build_server` documents: every career tool starts by resolving a name to a
    `player_id`, and a tool that exists and answers "unknown golfer" to every name is worse than
    one that is not offered — the model just keeps retrying the spelling.
    """

    def list_sessions(limit: int = 20) -> str:
        return _json(query.list_sessions(sessions_dir, limit=limit))

    def get_swing(session_id: str, swing_id: str) -> str:
        view = query.get_swing(sessions_dir, session_id, swing_id)
        return _json(view if view is not None else query.missing_swing(session_id, swing_id))

    def get_session_summary(session_id: str) -> str:
        detail = query.get_session_summary(sessions_dir, session_id)
        return _json(detail if detail is not None else query.missing_session(session_id))

    def get_recent_shots(count: int = 10) -> str:
        return _json(query.recent_shots(shot_source, count))

    def get_shot_by_id(shot_id: str) -> str:
        shot = query.get_shot(shot_source, shot_id)
        return _json(shot if shot is not None else query.missing_shot(shot_id))

    tools = [
        _tool(
            list_sessions,
            LIST_SESSIONS,
            limit="How many sessions to return, newest first.",
        ),
        _tool(
            get_swing,
            GET_SWING,
            session_id="Session id from list_sessions, e.g. '2026-08-10'.",
            swing_id="Swing id within that session, e.g. '2'.",
        ),
        _tool(
            get_session_summary,
            GET_SESSION_SUMMARY,
            session_id="Session id from list_sessions.",
        ),
        _tool(
            get_recent_shots,
            GET_RECENT_SHOTS,
            count="How many shots to return, newest first.",
        ),
        _tool(
            get_shot_by_id,
            GET_SHOT_BY_ID,
            shot_id="Shot id from get_recent_shots, or from a swing's attached shot.",
        ),
    ]

    if golfers_dir is not None:
        tools += _career_tools(sessions_dir, golfers_dir)
    return tools


def _career_tools(
    sessions_dir: Path, golfers_dir: Path
) -> list[BetaFunctionTool[Any]]:
    """The three that judge a golfer against their own history.

    Every one of them can refuse: the guard behind them withholds any figure the sample size
    cannot support, and at the sample size on disk today all of them do. That is working, not
    broken — see `caveats.READING_A_PERSONAL_HISTORY`, which is in the briefing precisely because
    a refusal sitting next to the counts that justify it is trivially reconstructible arithmetic.
    """

    def get_golfer_profile(player: str) -> str:
        profile = career.golfer_profile(sessions_dir, golfers_dir, player)
        return _json(profile if profile is not None else career.missing_golfer_profile(player))

    def get_shot_trends(metric: str, player: str, days: int | None = None) -> str:
        trend = career.shot_trends(sessions_dir, golfers_dir, metric, player, days)
        return _json(trend if trend is not None else career.missing_golfer_trend(player))

    def compare_sessions(session_a: str, session_b: str, player: str) -> str:
        compared = career.compare_sessions(
            sessions_dir, golfers_dir, session_a, session_b, player
        )
        return _json(
            compared if compared is not None else career.missing_golfer_comparison(player)
        )

    return [
        _tool(
            get_golfer_profile,
            GET_GOLFER_PROFILE,
            player="Golfer name as typed ('Aaron') or a stored id ('aaron').",
        ),
        _tool(
            get_shot_trends,
            GET_SHOT_TRENDS,
            metric="Metric name, e.g. 'tempo', 'head_sway', 'club_head_speed'.",
            player="Golfer name as typed, or a stored id.",
            days="Optional window in days; omit for the whole history.",
        ),
        _tool(
            compare_sessions,
            COMPARE_SESSIONS,
            session_a="First session id from list_sessions.",
            session_b="Second session id from list_sessions.",
            player="Golfer name as typed, or a stored id.",
        ),
    ]
