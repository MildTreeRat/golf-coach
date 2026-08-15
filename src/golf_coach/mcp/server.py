"""The MCP adapter: tool definitions over `query.py`. [M3]

Thin by construction. Every tool here validates its arguments, calls one `query` function and
returns its result — no reading, joining or aggregating happens in this file. That is what lets
the interesting half be tested without standing up a server, and what keeps the `llm` extra off
the base install (ADR-008).

**The tool descriptions are not here.** They live in `contracts/tool_descriptions.py` because
`mcp/runner_tools.py` needs the identical text for the in-process tool runner (ADR-020), and a
model picks a tool by reading that description and nothing else. Same reasoning as `caveats.py`
one import below, and the same three-time-repeated failure behind it.

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

from golf_coach.contracts.caveats import (
    READING_A_PERSONAL_HISTORY,
    READING_THIS_DATA_HONESTLY,
    TWO_AXES,
)
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
from golf_coach.mcp.career import GolferProfile, MetricTrend, SessionsCompared
from golf_coach.mcp.query import (
    NotFound,
    SessionDetail,
    SessionSummary,
    ShotView,
    SwingView,
)

SERVER_NAME = "golf-coach"


#: Shown to the model once, on connect. The place to put the standing caveats, so every tool
#: description does not have to repeat them and no answer has to rediscover them.
#:
#: The data paragraphs are composed from `contracts.caveats` rather than written here: the
#: coaching call (`feedback/coach.py`) needs the identical warnings, and ADR-008 rules out either
#: module importing the other. Only the opening line is local, because only it describes *this
#: server* rather than the data.
_OPENING = "Swing analysis and launch-monitor data from a home golf simulator."


def instructions(*, career_tools: bool) -> str:
    """The connect-time briefing, matched to the tools this server actually offers.

    `READING_A_PERSONAL_HISTORY` is added only when the career tools are, because it is entirely
    about how to read a withheld claim — and a briefing carrying rules for tools that are not
    present is how a model learns that the briefing describes something other than this server.
    """
    blocks = [_OPENING, TWO_AXES, READING_THIS_DATA_HONESTLY]
    if career_tools:
        blocks.append(READING_A_PERSONAL_HISTORY)
    return "\n\n".join(blocks) + "\n"


#: The briefing without the career tools. Kept as a name because it was one, and because the
#: no-registry server is still the shape ADR-006's addendum describes.
INSTRUCTIONS = instructions(career_tools=False)


def build_server(
    sessions_dir: Path,
    shot_source: ShotDataSource,
    *,
    golfers_dir: Path | None = None,
    name: str = SERVER_NAME,
) -> MCPServer:
    """Wire the tools to a session directory, a shot source, and optionally a golfer registry.

    All three are injected rather than read from `settings` here, so a test can point the server
    at a tmp path and a mock source without touching the environment — the same seam
    `api.pipeline` uses for its runner.

    **`golfers_dir` is optional and its absence removes the three career tools rather than
    breaking them.** Every career tool starts by resolving a name to a `player_id`, which is a
    registry lookup; without one there is no golfer to build a history for, and a tool that exists
    and answers "unknown golfer" to every name is worse than a tool that is not offered — the
    model would keep retrying the spelling.
    """
    server = MCPServer(name=name, instructions=instructions(career_tools=golfers_dir is not None))

    @server.tool(description=LIST_SESSIONS)
    def list_sessions(limit: int = 20) -> list[SessionSummary]:
        return query.list_sessions(sessions_dir, limit=limit)

    @server.tool(description=GET_SWING)
    def get_swing(session_id: str, swing_id: str) -> SwingView | NotFound:
        view = query.get_swing(sessions_dir, session_id, swing_id)
        return view if view is not None else query.missing_swing(session_id, swing_id)

    @server.tool(description=GET_SESSION_SUMMARY)
    def get_session_summary(session_id: str) -> SessionDetail | NotFound:
        detail = query.get_session_summary(sessions_dir, session_id)
        return detail if detail is not None else query.missing_session(session_id)

    @server.tool(description=GET_RECENT_SHOTS)
    def get_recent_shots(count: int = 10) -> list[ShotView]:
        return query.recent_shots(shot_source, count)

    @server.tool(description=GET_SHOT_BY_ID)
    def get_shot_by_id(shot_id: str) -> ShotView | NotFound:
        shot = query.get_shot(shot_source, shot_id)
        return shot if shot is not None else query.missing_shot(shot_id)

    if golfers_dir is not None:
        _add_career_tools(server, sessions_dir, golfers_dir)

    return server


def _add_career_tools(server: MCPServer, sessions_dir: Path, golfers_dir: Path) -> None:
    """The three tools that judge a golfer against their own history. [Career mode, step 6]

    Two of them — `get_shot_trends` and `compare_sessions` — are ADR-006's original table finally
    landing. They were deliberately deferred rather than dropped, and the reason was sample size:
    *"a trend tool over n=3 reports noise in a confident voice"*. What makes them safe to offer now
    is not more data (there still is not any) but the per-(metric, claim) guard behind them: a claim
    the `n` cannot support arrives absent, so the confident voice has nothing to say.

    `get_shot_trends` keeps its name though it now covers the seven pose metrics as well as the two
    launch-monitor ones. The description says so, because a model picks a tool by reading that and
    not by reading a changelog.
    """

    @server.tool(description=GET_GOLFER_PROFILE)
    def get_golfer_profile(player: str) -> GolferProfile | NotFound:
        profile = career.golfer_profile(sessions_dir, golfers_dir, player)
        return profile if profile is not None else career.missing_golfer_profile(player)

    @server.tool(description=GET_SHOT_TRENDS)
    def get_shot_trends(
        metric: str, player: str, days: int | None = None
    ) -> MetricTrend | NotFound:
        trend = career.shot_trends(sessions_dir, golfers_dir, metric, player, days)
        return trend if trend is not None else career.missing_golfer_trend(player)

    @server.tool(description=COMPARE_SESSIONS)
    def compare_sessions(
        session_a: str, session_b: str, player: str
    ) -> SessionsCompared | NotFound:
        compared = career.compare_sessions(
            sessions_dir, golfers_dir, session_a, session_b, player
        )
        return compared if compared is not None else career.missing_golfer_comparison(player)


def run(
    sessions_dir: Path, shot_source: ShotDataSource, golfers_dir: Path | None = None
) -> None:
    """Serve over stdio until the client disconnects."""
    build_server(sessions_dir, shot_source, golfers_dir=golfers_dir).run(transport="stdio")
