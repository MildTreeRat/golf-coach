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
