"""ScreenShotDataSource — the `ShotDataSource` adapter over parsed screen captures.

Reading is deliberately separate from importing: `scripts/import_shot_screens.py` does the
slow, extras-dependent work (OCR a photo, validate it, write it to the store), and this
source just serves what is already there. So the analysis engine and MCP server can consume
screen-derived shots on the base install, with no OpenCV, no OCR engine, and no images on
hand — exactly the property that lets `CompositeShotDataSource` mix this with the mock and
(later) R10 feeds behind one port (ADR-007/008).
"""

from __future__ import annotations

from collections.abc import Iterator
from pathlib import Path

from golf_coach.contracts.shot import ShotData
from golf_coach.launch_monitor.screen.store import ShotStore


class ScreenShotDataSource:
    """Serves shots previously parsed off launch-monitor screens. Implements the port."""

    def __init__(
        self,
        store: ShotStore | Path | str,
        *,
        session_id: str | None = None,
        include_needs_review: bool = True,
    ) -> None:
        """`session_id` filters to one range session; `include_needs_review` can hide
        low-confidence parses from consumers that should only see trusted numbers."""
        self._store = store if isinstance(store, ShotStore) else ShotStore(Path(store))
        self._session_id = session_id
        self._include_needs_review = include_needs_review

    def recent(self, count: int) -> list[ShotData]:
        if count <= 0:
            return []
        return self._shots()[:count]

    def stream(self) -> Iterator[ShotData]:
        """Yield stored shots oldest-first — the order they were actually hit in."""
        yield from reversed(self._shots())

    def _shots(self) -> list[ShotData]:
        shots = self._store.all()  # already newest-first
        if self._session_id is not None:
            shots = [s for s in shots if s.session_id == self._session_id]
        if not self._include_needs_review:
            shots = [s for s in shots if not (s.provenance and s.provenance.needs_review)]
        return shots
