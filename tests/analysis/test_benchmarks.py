"""Benchmark store: the seeded tempo band resolves, with correct fallback semantics.

These assert the store's *semantics* — a band resolves, it is ordered, it carries provenance, and
fallback lands on the right row — rather than literal thresholds. ADR-010 makes re-sourcing a band
a routine data edit; tests that hardcode `2.7` turn every such edit into a test failure that says
nothing about whether the resolver still works. The numbers themselves are checked where they are
actually derived, in `test_distributions.py`.
"""

from __future__ import annotations

from golf_coach.analysis.benchmarks import ResolvedRange, resolve_range
from golf_coach.contracts.intent import ClubCategory, PlayerProfile


def test_tempo_band_resolves_with_provenance() -> None:
    band = resolve_range("tempo_ratio")
    assert isinstance(band, ResolvedRange)
    assert band.low < band.high
    # Every threshold is auditable to a named source (ADR-010 §1).
    assert band.source.strip()


def test_tempo_band_brackets_the_classic_three_to_one() -> None:
    """Whatever the band is sourced from, ~3:1 must sit inside it or something is badly wrong."""
    band = resolve_range("tempo_ratio")
    assert band is not None
    assert band.low <= 3.0 <= band.high


def test_specific_club_and_skill_fall_back_to_all() -> None:
    # Tempo is seeded only at (all, all); a specific club + skill must still resolve to that row.
    band = resolve_range("tempo_ratio", ClubCategory.DRIVER, PlayerProfile(skill_level="10hcp"))
    generic = resolve_range("tempo_ratio")
    assert band is not None and generic is not None
    assert band == generic


def test_missing_checkpoint_yields_none() -> None:
    # No wrong score for an unseeded checkpoint — the resolver returns None (ADR-010 §2).
    assert resolve_range("hip_rotation") is None
