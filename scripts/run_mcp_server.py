"""Dev CLI: start the golf-coach MCP server. [M3]

Usage:
    python scripts/run_mcp_server.py
    python scripts/run_mcp_server.py --sessions-dir path/to/sessions --shots-dir path/to/shots

Exposes analyzed swings and launch-monitor shots to Claude as MCP tools (ADR-006, and its
2026-08-10 addendum for why the tool set covers swings and not only shots).

**Speaks stdio, not a port.** The client launches this process and talks to it over the pipe,
so there is nothing to point a browser at and `settings.mcp_port` is not used. To register it
with Claude Desktop, add to `claude_desktop_config.json`:

    {
      "mcpServers": {
        "golf-coach": {
          "command": "<repo>/.venv/Scripts/python.exe",
          "args": ["<repo>/scripts/run_mcp_server.py"]
        }
      }
    }

Claude Code: `claude mcp add golf-coach -- <python> <repo>/scripts/run_mcp_server.py`.

Reads only. Nothing here analyzes a swing or imports a shot — point it at data that
`scripts/analyze_bundle.py` (or the upload worker) and `scripts/import_shot_screens.py` have
already produced.

Requires the `llm` extra: pip install -e '.[llm]'
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from golf_coach.config import settings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--sessions-dir",
        type=Path,
        default=settings.sessions_dir,
        help="Directory of analyzed swing bundles (default: %(default)s).",
    )
    parser.add_argument(
        "--shots-dir",
        type=Path,
        default=settings.shots_dir,
        help="Directory of parsed launch-monitor shots (default: %(default)s).",
    )
    args = parser.parse_args()

    # Imported here, not at module scope, so `--help` and the path checks below work on an
    # install without the `llm` extra — and so a missing SDK produces an instruction rather
    # than an ImportError traceback out of a process the client is holding a pipe to.
    try:
        from golf_coach.mcp.server import run
    except ImportError as exc:  # pragma: no cover - depends on which extras are installed
        print(
            f"MCP SDK not available ({exc}). Install it with: pip install -e '.[llm]'",
            file=sys.stderr,
        )
        return 2

    from golf_coach.launch_monitor.composite import CompositeShotDataSource
    from golf_coach.launch_monitor.screen.source import ScreenShotDataSource

    # Warn rather than fail: a missing directory is an empty dataset, and the tools answer
    # "nothing recorded yet" perfectly well. Exiting here would make the client show a broken
    # server instead, which is a worse diagnosis of the same situation.
    for label, path in (("sessions", args.sessions_dir), ("shots", args.shots_dir)):
        if not path.exists():
            print(f"warning: {label} directory does not exist: {path}", file=sys.stderr)

    source = CompositeShotDataSource([ScreenShotDataSource(args.shots_dir)])
    print(
        f"golf-coach MCP server on stdio — sessions={args.sessions_dir} shots={args.shots_dir}",
        file=sys.stderr,
    )
    run(args.sessions_dir, source)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
