"""Launch monitor module — shot data + MCP server (the imperative shell's LM edge).

Defines the ShotDataSource *port* with three adapters: a MockShotDataSource (works today,
no hardware), a ScreenShotDataSource (shots parsed off photos of the simulator's SHOT DATA
screen, ADR-014), and the R10 adapter (later). CompositeShotDataSource fans a single port
out across several of them, so a session can mix sources. The MCP server exposes whichever
source is wired in — swapping hardware only changes the adapter, never the consumers
(ADR-006/007).

`ScreenShotDataSource` is imported from `golf_coach.launch_monitor.screen` rather than
re-exported here, so importing the port stays free of the OCR/vision extras (ADR-008).
"""

from golf_coach.launch_monitor.composite import CompositeShotDataSource
from golf_coach.launch_monitor.mock import MockShotDataSource
from golf_coach.launch_monitor.source import ShotDataSource

__all__ = ["ShotDataSource", "MockShotDataSource", "CompositeShotDataSource"]
