"""The MCP adapter: tool definitions over `query.py`. [M3]

Thin by construction. Every tool here validates its arguments, calls one `query` function and
returns its result — no reading, joining or aggregating happens in this file. That is what lets
the interesting half be tested without standing up a server, and what keeps the `llm` extra off
the base install (ADR-008).

`from mcp.server import MCPServer` resolves to the installed SDK, not to this package: Python 3
has no implicit relative imports, so `golf_coach.mcp` never shadows top-level `mcp`. It reads
like a bug and is not one.

**Transport is stdio**, which is what Claude Desktop and Claude Code speak. `settings.mcp_port`
(8081) is left alone rather than wired up — under stdio the client launches this process and
talks over the pipe, so there is no port for anything to connect to. The field becomes real if
an HTTP transport is ever added.
"""

from __future__ import annotations

from pathlib import Path

from mcp.server import MCPServer
from pydantic import BaseModel, Field

from golf_coach.contracts.caveats import (
    READING_THIS_DATA_HONESTLY,
    TWO_AXES,
)
from golf_coach.launch_monitor.source import ShotDataSource
from golf_coach.mcp import query
from golf_coach.mcp.query import (
    SessionDetail,
    SessionSummary,
    ShotView,
    SwingView,
)

SERVER_NAME = "golf-coach"


class NotFound(BaseModel):
    """Nothing matched — said out loud rather than by returning nothing.

    A tool that returns `None` serializes to a result with *no content blocks*, which reads the
    same as a call that silently did nothing and gives the model no way to tell "you have the
    wrong id" from "something broke". Naming the miss, and saying what to do about it, is the
    difference between a useful retry and an invented answer.
    """

    found: bool = Field(default=False, description="Always false. This is the miss case.")
    message: str


def _missing(what: str, hint: str) -> NotFound:
    return NotFound(message=f"{what} {hint}")

#: Shown to the model once, on connect. The place to put the standing caveats, so every tool
#: description does not have to repeat them and no answer has to rediscover them.
#:
#: The data paragraphs are composed from `contracts.caveats` rather than written here: the
#: coaching call (`feedback/coach.py`) needs the identical warnings, and ADR-008 rules out either
#: module importing the other. Only the opening line is local, because only it describes *this
#: server* rather than the data.
_OPENING = "Swing analysis and launch-monitor data from a home golf simulator."

INSTRUCTIONS = "\n\n".join([_OPENING, TWO_AXES, READING_THIS_DATA_HONESTLY]) + "\n"


def build_server(
    sessions_dir: Path,
    shot_source: ShotDataSource,
    *,
    name: str = SERVER_NAME,
) -> MCPServer:
    """Wire the tools to a session directory and a shot source.

    Both are injected rather than read from `settings` here, so a test can point the server at
    a tmp path and a mock source without touching the environment — the same seam
    `api.pipeline` uses for its runner.
    """
    server = MCPServer(name=name, instructions=INSTRUCTIONS)

    @server.tool(
        description=(
            "List golf sessions, newest first, with each swing's score and lead coaching point. "
            "Call this first when the user asks about their golf generally, refers to a session "
            "by day ('yesterday', 'last time'), or asks what data exists — it is the cheapest way "
            "to find the session and swing ids every other tool needs. Returns one entry per "
            "session with its swings; a swing whose status is not 'done' has no score yet, and "
            "'stale' means the video was re-uploaded after the result was computed."
        )
    )
    def list_sessions(limit: int = 20) -> list[SessionSummary]:
        return query.list_sessions(sessions_dir, limit=limit)

    @server.tool(
        description=(
            "Get one swing's full analysis: every scored checkpoint with the tour band it was "
            "judged against and the percentile it sits at, the ranked coaching tips, any "
            "checkpoints that could not be measured, and the launch-monitor shot if one was "
            "attached. Call this whenever the user asks how a specific swing went or what to work "
            "on — it is the only tool that returns mechanics. Needs a session_id and swing_id "
            "from list_sessions; if no such swing exists it says so rather than returning data."
        )
    )
    def get_swing(session_id: str, swing_id: str) -> SwingView | NotFound:
        view = query.get_swing(sessions_dir, session_id, swing_id)
        return view if view is not None else _missing(
            f"No swing {swing_id!r} in session {session_id!r}.",
            "Call list_sessions to see which sessions and swing ids exist.",
        )

    @server.tool(
        description=(
            "Aggregate one session across both axes: mean/best/worst score, how often each "
            "checkpoint passed, which checkpoints went unmeasured, and averages over the "
            "launch-monitor shots attached to that session's swings. Call this when the user asks "
            "how a whole session or day went, rather than fetching every swing individually. "
            "Shots flagged as uncertain are counted but excluded from the averages. A session "
            "with nothing analyzed yet returns zero counts, which is a real answer; only a "
            "session id that does not exist at all reports a miss."
        )
    )
    def get_session_summary(session_id: str) -> SessionDetail | NotFound:
        detail = query.get_session_summary(sessions_dir, session_id)
        return detail if detail is not None else _missing(
            f"No session {session_id!r}.",
            "Call list_sessions to see which sessions exist.",
        )

    @server.tool(
        description=(
            "Get the most recent launch-monitor shots, newest first, across every configured "
            "source. Call this when the user asks about ball flight, distances, club speed or "
            "strike quality without naming a specific swing. Each shot carries needs_review and "
            "parse_confidence, because these numbers are read off a photograph of the simulator "
            "screen by OCR — treat a flagged shot's figures as uncertain rather than as fact."
        )
    )
    def get_recent_shots(count: int = 10) -> list[ShotView]:
        return query.recent_shots(shot_source, count)

    @server.tool(
        description=(
            "Get one launch-monitor shot by its id, with the full set of metrics and the "
            "confidence of the OCR parse that produced them. Call this to follow up on a specific "
            "shot surfaced by get_recent_shots or attached to a swing by get_swing. If no shot "
            "has that id it says so rather than returning data."
        )
    )
    def get_shot_by_id(shot_id: str) -> ShotView | NotFound:
        shot = query.get_shot(shot_source, shot_id)
        return shot if shot is not None else _missing(
            f"No shot with id {shot_id!r}.",
            "Call get_recent_shots to see which shot ids exist.",
        )

    return server


def run(sessions_dir: Path, shot_source: ShotDataSource) -> None:
    """Serve over stdio until the client disconnects."""
    build_server(sessions_dir, shot_source).run(transport="stdio")
