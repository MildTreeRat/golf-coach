"""ShotDataSource port: the interface every shot-data source implements.

Adapters:
  - MockShotDataSource (launch_monitor/mock.py) — synthetic shots          [today]
  - R10Source          (launch_monitor/r10.py)  — Garmin R10 over BLE      [needs hardware]

Both yield identical `ShotData` contracts (ADR-007).
"""

from __future__ import annotations

from collections.abc import Iterator
from typing import Protocol, runtime_checkable

from golf_coach.contracts.shot import ShotData


@runtime_checkable
class ShotDataSource(Protocol):
    """A source of launch-monitor shots."""

    def recent(self, count: int) -> list[ShotData]:
        """Return the most recent `count` shots."""
        ...

    def stream(self) -> Iterator[ShotData]:
        """Yield shots as they arrive (real-time for R10; finite for mock)."""
        ...
