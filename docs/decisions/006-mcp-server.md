# ADR-006: MCP Server for Launch Monitor Data

## Status
Accepted

## Date
2026-03-16

## Context
Need to expose launch monitor shot data to both the analysis engine and the LLM coaching layer (Claude API). MCP (Model Context Protocol) provides a standard way for LLMs to call tools — this is a natural fit for letting Claude query shot history and correlate it with swing analysis.

## Options Considered

### Option A: MCP Server exposing shot data as tools
- **Pros**: Claude can directly query shot data during coaching conversations via tool use. Standard protocol. Clean separation — the MCP server owns all launch monitor data access. Useful learning exercise for MCP.
- **Cons**: Adds a service to run. MCP is relatively new — less community support.

### Option B: Direct database access from analysis engine + manual prompt stuffing for LLM
- **Pros**: Simpler. No extra service.
- **Cons**: LLM can't dynamically query data — you'd have to anticipate what data to include in the prompt. Loses the interactive coaching ability.

### Option C: REST API instead of MCP
- **Pros**: More familiar pattern. Easier to test with curl/Postman.
- **Cons**: Claude can't call REST APIs natively during conversations (without custom tool definitions). MCP is purpose-built for this.

## Decision
**MCP Server**. It's the right abstraction for LLM-accessible data, and it's a valuable skill to learn. The server will also serve as the single source of truth for all shot data, used by both the analysis engine and Claude.

## Tool Definitions (planned)

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `get_recent_shots` | Last N shots | `count: int` | `List[ShotData]` |
| `get_shot_by_id` | Single shot details | `shot_id: str` | `ShotData` |
| `get_session_summary` | Averages for a session | `session_id: str` | `SessionSummary` |
| `compare_sessions` | Compare two sessions | `session_a: str, session_b: str` | `ComparisonResult` |
| `get_shot_trends` | Metric trends over time | `metric: str, days: int` | `List[DataPoint]` |

## Consequences
- MCP server is a standalone Python service (port 8081).
- Analysis engine queries MCP tools programmatically.
- Claude API calls include MCP server URL, enabling Claude to pull shot data during coaching.
- Launch monitor data parsing is isolated inside the MCP server — swapping hardware only changes the parser, not the tool interface.

---

## Addendum (2026-08-05): parsing moved out of the server, and that is a better shape

The last Consequence above — "launch monitor data parsing is isolated **inside the MCP
server**" — is no longer how the system is built, and correcting the record matters because it
changes what the MCP server is *for*.

[ADR-008](008-project-structure.md) introduced the `ShotDataSource` port, and
[ADR-014](014-screen-capture-shot-ingestion.md) built the real parser behind it: screen
rectification, OCR, tile parsing, sign conventions and physics validation all live in
`src/golf_coach/launch_monitor/screen/`, with `CompositeShotDataSource` mixing adapters. The
MCP server is therefore a **consumer of the port, not the owner of the parsing** — one of
several, alongside `scripts/import_shot_screens.py` and (later) the analysis engine.

**Why this is an improvement rather than a drift to be corrected back:**

- The original goal — "swapping hardware only changes the parser, not the tool interface" — is
  *better* served by the port. It is now swappable for every consumer, not just for the MCP
  server's tools.
- Shot data became usable **before** the MCP server existed. `ScreenShotDataSource` serves
  parsed shots on the base install with no OCR stack and no running service, which is what let
  M3 deliver value while its server half is still unwritten.
- Parsing is testable without standing up a service. The launch-monitor suite runs in-process.

**What is unchanged:** the decision to build an MCP server, the tool list above, and the
rationale for MCP over REST. The server remains the intended interface for Claude to query
shot history during coaching (M6). It is simply a thin adapter over the port rather than the
place the parsing lives.

**Status of the server itself:** not built. `scripts/run_mcp_server.py` raises
`NotImplementedError`, and it is the only remaining M3 item.

---

## Addendum (2026-08-10): the tool list was written before the swing analysis existed

The addendum above says "what is unchanged: the decision to build an MCP server, **the tool
list above**, and the rationale for MCP over REST." Two of those three still hold. The tool
list does not, and this is the correction.

**Why it went stale.** Every tool in the 2026-03-16 table is shot-only. That was the whole
system then: M3 was the launch-monitor milestone and nothing else produced a queryable
artifact. Since, the pose-only track delivered a scored swing — three checkpoints with
benchmark bands ([ADR-010](010-benchmark-ranges.md)), tour percentiles from 458 face-on
swings ([ADR-012](012-golfdb-reference-data.md)), ranked coaching tips (M5-FB) — and M7
Phase 4 joined it to the shot and started writing `analysis.json` per swing.

So the shot metrics are now the *less* differentiated half of what this repo holds. They are
the HD Golf simulator's own readout, photographed and OCR'd; the simulator already displays
them on a screen in front of the golfer. What only this system can say is where a swing sits
against a tour population. A server built to the table above would let Claude report "club
speed 98.3, carried 121 yards" and leave it unable to say "your head sway sits higher than
83% of 458 tour swings" — which inverts the point of a coaching interface.

**The revised tool set.** Both axes of [ADR-009](009-swing-scoring-model.md)'s model, not just
the outcome one:

| Tool | Description | Output |
|------|-------------|--------|
| `list_sessions` | Sessions newest-first, with per-swing score and headline | `list[SessionSummary]` |
| `get_swing` | One swing: checkpoints with band + percentile, ranked tips, alignment quality | `SwingView` |
| `get_recent_shots` | Last N shots | `list[ShotData]` |
| `get_shot_by_id` | Single shot with full provenance | `ShotData` |
| `get_session_summary` | Per-session aggregates across both axes | `SessionDetail` |

**Two tools from the original table are deliberately not built yet**, and the reason is
sample size rather than effort. `get_shot_trends` and `compare_sessions` both invite Claude to
narrate a trend; `data/processed/shots/` currently holds **three** parsed shots across three
sessions. A trend tool over n=3 does not report a trend, it reports noise with a confident
voice — the same failure mode M5-FB's percentile work exists to prevent, and the same one
[ADR-012](012-golfdb-reference-data.md) found when a 120-clip result vanished at 461. They go
in when a real range session's worth of shots exists to trend over. This is an omission by
decision, not by oversight.

**Location diverges from ADR-008.** [ADR-008](008-project-structure.md)'s tree puts the MCP
server inside `launch_monitor/` ("ShotDataSource port + mock/r10 adapters + MCP server"),
which was correct for a server that only ever read shots. A server that also reads swing
analysis, session bundles and benchmark percentiles does not belong inside the launch-monitor
module — it would make `launch_monitor` import `analysis` and `storage` to serve tools that
have nothing to do with a launch monitor. It goes in a new top-level `src/golf_coach/mcp/`,
which is what the dependency direction actually wants.

**What is still unchanged**, now for the third time: the decision to build an MCP server, the
rationale for MCP over REST, and the addendum above's correction that parsing lives behind the
`ShotDataSource` port rather than inside the server. Nothing here re-decides any of that, which
is why this is an addendum and not ADR-017. The next free number stays 017.
