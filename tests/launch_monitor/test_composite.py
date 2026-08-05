"""Fanning one ShotDataSource out across several.

A session realistically mixes sources — screen captures imported from the range, mock
shots while testing, an R10 feed later. Consumers must keep seeing one port, and "the
last N shots" must mean the last N *overall*.
"""

from __future__ import annotations

from collections.abc import Iterator
from datetime import UTC, datetime, timedelta

from golf_coach.contracts.shot import ShotData, ShotSource
from golf_coach.launch_monitor import CompositeShotDataSource, MockShotDataSource
from golf_coach.launch_monitor.source import ShotDataSource

_BASE = datetime(2026, 8, 4, 12, 0, tzinfo=UTC)


class _FakeSource:
    """A fixed list of shots, standing in for any adapter."""

    def __init__(self, shots: list[ShotData]) -> None:
        self._shots = sorted(shots, key=lambda s: s.timestamp, reverse=True)

    def recent(self, count: int) -> list[ShotData]:
        return self._shots[:count]

    def stream(self) -> Iterator[ShotData]:
        yield from reversed(self._shots)


def _shot(shot_id: str, minutes: int, source: ShotSource = ShotSource.SCREEN) -> ShotData:
    return ShotData(
        shot_id=shot_id,
        session_id="mixed",
        timestamp=_BASE + timedelta(minutes=minutes),
        source=source,
        carry_distance=float(100 + minutes),
    )


def test_composite_satisfies_the_port() -> None:
    composite = CompositeShotDataSource([MockShotDataSource(count=3)])

    assert isinstance(composite, ShotDataSource)


def test_recent_interleaves_sources_by_time() -> None:
    screens = _FakeSource([_shot("screen-1", 0), _shot("screen-2", 20)])
    mock = _FakeSource([_shot("mock-1", 10, ShotSource.MOCK)])

    composite = CompositeShotDataSource([screens, mock])

    assert [s.shot_id for s in composite.recent(3)] == ["screen-2", "mock-1", "screen-1"]


def test_recent_truncates_across_the_whole_merge_not_per_source() -> None:
    a = _FakeSource([_shot("a1", 1), _shot("a2", 2)])
    b = _FakeSource([_shot("b1", 3), _shot("b2", 4)])

    assert [s.shot_id for s in CompositeShotDataSource([a, b]).recent(2)] == ["b2", "b1"]


def test_first_source_wins_on_a_duplicate_id() -> None:
    """Ordering sources by trust is meaningful: hardware before mock."""
    trusted = _FakeSource([_shot("same-shot", 0, ShotSource.R10)])
    fallback = _FakeSource([_shot("same-shot", 0, ShotSource.MOCK)])

    merged = CompositeShotDataSource([trusted, fallback]).recent(10)

    assert len(merged) == 1
    assert merged[0].source is ShotSource.R10


def test_stream_drains_sources_in_order_without_repeats() -> None:
    a = _FakeSource([_shot("a1", 1), _shot("shared", 2)])
    b = _FakeSource([_shot("shared", 2), _shot("b1", 3)])

    streamed = [s.shot_id for s in CompositeShotDataSource([a, b]).stream()]

    assert streamed == ["a1", "shared", "b1"]


def test_empty_and_degenerate_cases() -> None:
    assert CompositeShotDataSource([]).recent(5) == []
    assert list(CompositeShotDataSource([]).stream()) == []
    assert CompositeShotDataSource([_FakeSource([_shot("a", 0)])]).recent(0) == []
