"""MCP server exposing swings and shots to Claude. [M3]

Two modules, and the split is load-bearing:

  - `query.py`  — reads sessions, swings and shots off disk. Imports no MCP SDK, so it
                  installs, imports and tests on the base install.
  - `server.py` — the MCP adapter. Declares tools, delegates every call to `query`.

Same shape as `api/pipeline.py` (imports no fastapi) beside `api/app.py`, and pinned the
same way by a subprocess import test. See ADR-006's 2026-08-10 addendum for why this lives
here rather than in `launch_monitor/` as ADR-008's tree originally placed it: a server that
reads swing analysis, session bundles and tour percentiles is not a launch-monitor concern.
"""
