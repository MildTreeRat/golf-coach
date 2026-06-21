"""Dev CLI: start the launch-monitor MCP server. [M3]

Runs against the MockShotDataSource by default (no hardware), switching to the R10
adapter once available (ADR-007). Requires the `llm` extra for the MCP SDK.
"""

from __future__ import annotations


def main() -> int:
    raise NotImplementedError("M3: start MCP server backed by a ShotDataSource.")


if __name__ == "__main__":
    raise SystemExit(main())
