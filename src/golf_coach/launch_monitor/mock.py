"""MockShotDataSource — synthetic but realistic shots, no hardware required.

This is what unblocks M3 (and downstream M4-M6) before the Garmin R10 is purchased
(ADR-007). It produces plausible 7-iron-ish numbers with a little random variation so
the analysis engine and MCP server have something real-shaped to chew on.
"""

from __future__ import annotations

import random
from collections.abc import Iterator
from datetime import UTC, datetime

from golf_coach.contracts.shot import ShotData, ShotSource


class MockShotDataSource:
    """Generates deterministic-by-seed synthetic shots for one session."""

    def __init__(self, session_id: str = "mock-session", count: int = 10, seed: int = 0) -> None:
        self._session_id = session_id
        self._count = count
        self._rng = random.Random(seed)

    def _make_shot(self, n: int) -> ShotData:
        chs = self._rng.gauss(85.0, 3.0)  # club head speed, mph
        smash = self._rng.gauss(1.38, 0.02)
        return ShotData(
            shot_id=f"{self._session_id}-{n}",
            session_id=self._session_id,
            timestamp=datetime.now(UTC),
            source=ShotSource.MOCK,
            club_head_speed=round(chs, 1),
            club_face_angle=round(self._rng.gauss(0.0, 2.0), 1),
            club_path=round(self._rng.gauss(0.0, 3.0), 1),
            ball_speed=round(chs * smash, 1),
            launch_angle=round(self._rng.gauss(17.0, 2.0), 1),
            launch_direction=round(self._rng.gauss(0.0, 2.0), 1),
            spin_rate=round(self._rng.gauss(6500, 500), 0),
            spin_axis=round(self._rng.gauss(0.0, 4.0), 1),
            smash_factor=round(smash, 2),
            carry_distance=round(self._rng.gauss(160, 8), 1),
            total_distance=round(self._rng.gauss(170, 8), 1),
            apex_height=round(self._rng.gauss(30, 3), 1),
        )

    def recent(self, count: int) -> list[ShotData]:
        return [self._make_shot(n) for n in range(min(count, self._count))]

    def stream(self) -> Iterator[ShotData]:
        for n in range(self._count):
            yield self._make_shot(n)
