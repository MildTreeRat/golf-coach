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


# --- the one place this file asserts a literal, and why -----------------------------------------


def test_the_hip_sway_lower_edge_carries_its_clustered_interval() -> None:
    """A decision to *keep* a band edge leaves no behaviour behind to pin — only a reason.

    `hip_sway_norm`'s lower edge was revisited on 2026-08-18 and kept: M8's player-clustered
    bootstrap puts p10's 95% interval down at 0.0801, which widens where the edge *sits* without
    touching the 2.8x resolution claim that admitted it (ADR-010 addendum 2026-08-18). Nothing
    executable moved, so the only thing that can silently regress is the argument going missing
    from the row — and a session reading only the confident half is free to tighten this band on a
    number it does not know has an error bar.

    **This deliberately breaks the module docstring's rule against literals.** That rule exists to
    keep re-sourcing a band a cheap data edit, and it does not conflict here: a re-derivation
    replaces this interval rather than dropping it, so this test failing on one is the alarm
    working rather than friction.
    """
    band = resolve_range("hip_sway_norm")
    assert band is not None
    assert "0.0801" in band.source, "the clustered interval is no longer recorded on the row"
