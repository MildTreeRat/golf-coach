"""Launch monitor module — shot data + MCP server (the imperative shell's LM edge).

Defines the ShotDataSource *port* with two adapters: a MockShotDataSource (works today,
no hardware) and the R10 adapter (later). The MCP server exposes whichever source is
wired in. Swapping hardware only changes the adapter, never the consumers (ADR-006/007).
"""

from golf_coach.launch_monitor.mock import MockShotDataSource
from golf_coach.launch_monitor.source import ShotDataSource

__all__ = ["ShotDataSource", "MockShotDataSource"]
