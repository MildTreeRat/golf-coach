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
