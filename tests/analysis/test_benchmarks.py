"""Benchmark store: the seeded Tour Tempo band resolves, with correct fallback semantics."""

from __future__ import annotations

from golf_coach.analysis.benchmarks import ResolvedRange, resolve_range
from golf_coach.contracts.intent import ClubCategory, PlayerProfile


def test_tour_tempo_band_resolves() -> None:
    band = resolve_range("tempo_ratio")
    assert isinstance(band, ResolvedRange)
    assert (band.low, band.high) == (2.7, 3.3)
    assert "Tour Tempo" in band.source


def test_specific_club_and_skill_fall_back_to_all() -> None:
    # Tempo is seeded only at (all, all); a specific club + skill must still resolve to it.
    band = resolve_range("tempo_ratio", ClubCategory.DRIVER, PlayerProfile(skill_level="10hcp"))
    assert band is not None
    assert (band.low, band.high) == (2.7, 3.3)


def test_missing_checkpoint_yields_none() -> None:
    # No wrong score for an unseeded checkpoint — the resolver returns None (ADR-010 §2).
    assert resolve_range("hip_rotation") is None
