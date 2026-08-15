"""What each tool is for, in the words a model picks it by. [M6]

The second odd member of `contracts/`, for the same reason as its neighbour `caveats.py`: not a
data shape, but prose *about* the shapes here that two modules need to be identical. `mcp/server.py`
hands these to an MCP client over stdio; `mcp/runner_tools.py` hands them to the in-process tool
runner (ADR-020). ADR-008 forbids either importing the other, and a second copy of load-bearing
prose is prose that goes stale in one place — this repo has been bitten by that three times.

**A model picks a tool by reading its description and nothing else.** It does not read the
implementation, the ADR, or the changelog. So these strings carry three things beyond what the
tool does:

  - **when to call it**, in the user's language rather than the schema's ("yesterday", "last
    time", "how did I hit it today") — routing text, and the one place calibrated urgency belongs;
  - **what a miss looks like**, so a wrong id reads as an answer to retry rather than a fault;
  - **the refusals**, for the career tools. Those withhold any figure the sample size cannot
    support, and a withheld figure sitting beside the counts that justify it is exactly what an
    eager reader reconstructs. `caveats.READING_A_PERSONAL_HISTORY` says so once at the top of the
    conversation; these say it again at the point of use, because that is where it gets ignored.

Kept as plain strings rather than derived from anything. Nothing here states a count or a band —
that is deliberate, and it is what keeps this file out of the failure `caveats.py` guards against
with `CHECKPOINT_REGISTRY`. If a description ever needs to name how many checkpoints exist, derive
it there and interpolate; do not type the number here.
"""

from __future__ import annotations

LIST_SESSIONS = (
    "List golf sessions, newest first, with each swing's score and lead coaching point. "
    "Call this first when the user asks about their golf generally, refers to a session "
    "by day ('yesterday', 'last time'), or asks what data exists — it is the cheapest way "
    "to find the session and swing ids every other tool needs. Returns one entry per "
    "session with its swings; a swing whose status is not 'done' has no score yet, and "
    "'stale' means the video was re-uploaded after the result was computed."
)

GET_SWING = (
    "Get one swing's full analysis: every scored checkpoint with the tour band it was "
    "judged against and the percentile it sits at, the ranked coaching tips, any "
    "checkpoints that could not be measured, and the launch-monitor shot if one was "
    "attached. Call this whenever the user asks how a specific swing went or what to work "
    "on — it is the only tool that returns mechanics. Needs a session_id and swing_id "
    "from list_sessions; if no such swing exists it says so rather than returning data."
)

GET_SESSION_SUMMARY = (
    "Aggregate one session across both axes: mean/best/worst score, how often each "
    "checkpoint passed, which checkpoints went unmeasured, and averages over the "
    "launch-monitor shots attached to that session's swings. Call this when the user asks "
    "how a whole session or day went, rather than fetching every swing individually. "
    "Shots flagged as uncertain are counted but excluded from the averages. A session "
    "with nothing analyzed yet returns zero counts, which is a real answer; only a "
    "session id that does not exist at all reports a miss."
)

GET_RECENT_SHOTS = (
    "Get the most recent launch-monitor shots, newest first, across every configured "
    "source. Call this when the user asks about ball flight, distances, club speed or "
    "strike quality without naming a specific swing. Each shot carries needs_review and "
    "parse_confidence, because these numbers are read off a photograph of the simulator "
    "screen by OCR — treat a flagged shot's figures as uncertain rather than as fact."
)

GET_SHOT_BY_ID = (
    "Get one launch-monitor shot by its id, with the full set of metrics and the "
    "confidence of the OCR parse that produced them. Call this to follow up on a specific "
    "shot surfaced by get_recent_shots or attached to a swing by get_swing. If no shot "
    "has that id it says so rather than returning data."
)

GET_GOLFER_PROFILE = (
    "Get one golfer's own baseline: their typical value and spread for each measured "
    "metric, whether their miss is repeatable or scattered, and where their typical value "
    "sits in the tour population. Call this whenever the user asks what they usually do, "
    "what their tendency is, whether something is improving, or why a fault keeps "
    "happening — it is the only tool that judges a golfer against themselves rather than "
    "against tour bands. Takes a name as typed ('Aaron') or a stored id ('aaron'). "
    "IMPORTANT: any figure the sample size cannot support is absent rather than flagged, "
    "and the `withheld` list says what it is waiting for. Report those refusals as "
    "refusals; do not compute the missing figure from the per-session counts beside it."
)

GET_SHOT_TRENDS = (
    "Get one metric's per-session history for one golfer, oldest first, to answer whether "
    "it has moved. Covers the pose metrics (tempo, head sway, hip sway, finish balance and "
    "the rest) as well as the launch-monitor ones, despite the name. Call this for "
    "'is my tempo getting better', 'has my face angle changed since last month'. Optional "
    "`days` limits the window; omit it for the whole history. IMPORTANT: `ready` is false "
    "until there are enough swings AND enough separate sessions for a trend to mean "
    "anything, and while it is false every per-session mean is absent and no direction may "
    "be described — the session counts are the evidence for the refusal, not a series."
)

COMPARE_SESSIONS = (
    "Hold two of one golfer's sessions against each other: each session's mean score, and "
    "each metric's sample count and mean in both. Call this for 'was today better than "
    "last time' or 'compare Tuesday to Thursday'. Needs two session ids from "
    "list_sessions. IMPORTANT: a per-metric mean is present only when that session on its "
    "own carried enough swings to support one, so at small sample sizes the honest answer "
    "is a pair of counts and no delta. No difference here is tested for significance, and "
    "a session can score well while contributing no samples at all — read the `note`."
)

#: tool name -> description, derived from the constants above rather than retyped.
#:
#: Exists so a consumer can iterate the surface and so `tests/contracts/test_tool_descriptions.py`
#: can assert both adapters expose exactly these names. The names are the wire identifiers a model
#: calls, so they belong beside the prose they identify.
TOOL_DESCRIPTIONS: dict[str, str] = {
    "list_sessions": LIST_SESSIONS,
    "get_swing": GET_SWING,
    "get_session_summary": GET_SESSION_SUMMARY,
    "get_recent_shots": GET_RECENT_SHOTS,
    "get_shot_by_id": GET_SHOT_BY_ID,
    "get_golfer_profile": GET_GOLFER_PROFILE,
    "get_shot_trends": GET_SHOT_TRENDS,
    "compare_sessions": COMPARE_SESSIONS,
}

#: The three that only exist when a golfer registry is configured (`build_server`'s `golfers_dir`).
#: Named here so both adapters agree on which tools that flag adds, rather than each keeping a list.
CAREER_TOOL_NAMES: frozenset[str] = frozenset(
    {"get_golfer_profile", "get_shot_trends", "compare_sessions"}
)
