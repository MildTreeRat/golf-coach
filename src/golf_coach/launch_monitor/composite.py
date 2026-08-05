"""CompositeShotDataSource — one `ShotDataSource` over several underlying ones.

Shots arrive from more than one place at once: screen captures imported from a range
session, synthetic shots while testing, and (later) a live R10 feed. Rather than teach
every consumer about that fan-in, this adapter *is* a `ShotDataSource` and hides it —
the MCP server and analysis engine still see a single port (ADR-007/008).

Merging rules:
  - `shot_id` is the identity. First source wins on a collision, so ordering the
    sources by trust (real hardware before mock) is meaningful.
  - `recent()` sorts newest-first across all sources, so "the last 10 shots" means the
    last 10 *overall*, not 10 from whichever source answered first.
"""

from __future__ import annotations

from collections.abc import Iterator, Sequence

from golf_coach.contracts.shot import ShotData
from golf_coach.launch_monitor.source import ShotDataSource


class CompositeShotDataSource:
    """Fans one `ShotDataSource` request out across several. Implements the port."""

    def __init__(self, sources: Sequence[ShotDataSource]) -> None:
        self._sources = tuple(sources)

    @property
    def sources(self) -> tuple[ShotDataSource, ...]:
        return self._sources

    def recent(self, count: int) -> list[ShotData]:
        """The most recent `count` shots across every source, newest first.

        Each source is asked for `count` because any one of them could hold all of the
        newest shots; the merged list is then sorted and truncated.
        """
        if count <= 0:
            return []
        merged: dict[str, ShotData] = {}
        for source in self._sources:
            for shot in source.recent(count):
                merged.setdefault(shot.shot_id, shot)
        newest_first = sorted(merged.values(), key=lambda s: s.timestamp, reverse=True)
        return newest_first[:count]

    def stream(self) -> Iterator[ShotData]:
        """Yield shots from each source in turn, skipping ids already seen.

        Deliberately sequential rather than time-merged: a live source's `stream()` may
        never end, so draining sources in order is the only well-defined behaviour.
        """
        seen: set[str] = set()
        for source in self._sources:
            for shot in source.stream():
                if shot.shot_id in seen:
                    continue
                seen.add(shot.shot_id)
                yield shot
