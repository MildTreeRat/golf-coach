"""Benchmark range store — versioned data with provenance. [M4-PoC]

Per [ADR-010](../../../docs/decisions/010-benchmark-ranges.md): the "correct" range for a
checkpoint is *data*, not a magic constant in code. Every row records its `source` so each
threshold is auditable and swappable, and rows are keyed by `(checkpoint, club, skill)` so
they can be parameterized later without touching the resolver.

`resolve_range` falls back most-specific → least-specific so the store can stay sparse, and
returns `None` when nothing matches — a missing benchmark yields *no* score for that
checkpoint rather than a wrong one (ADR-010 §2). The PoC seeds exactly one row (Tour Tempo).

Pure stdlib + pydantic: the data ships inside the package (`ranges.json`) and is read via
`importlib.resources`, so the analysis core stays install-light (ADR-008).
"""

from __future__ import annotations

import json
from functools import lru_cache
from importlib import resources
from typing import NamedTuple

from pydantic import BaseModel, TypeAdapter

from golf_coach.contracts.intent import ClubCategory, PlayerProfile

_RANGES_FILE = "ranges.json"


class BenchmarkRange(BaseModel):
    """One seeded benchmark row (internal — not a cross-module contract)."""

    checkpoint: str
    club_category: ClubCategory
    skill_level: str
    low: float
    high: float
    source: str
    source_date: str
    added: str


class ResolvedRange(NamedTuple):
    """The resolved band for a checkpoint, carrying its provenance."""

    low: float
    high: float
    source: str


_RANGE_LIST_ADAPTER = TypeAdapter(list[BenchmarkRange])


@lru_cache(maxsize=1)
def _load_ranges() -> list[BenchmarkRange]:
    """Load and validate the seeded ranges from the packaged JSON (cached)."""
    raw = resources.files(__package__).joinpath(_RANGES_FILE).read_text(encoding="utf-8")
    payload = json.loads(raw)
    return _RANGE_LIST_ADAPTER.validate_python(payload["ranges"])


def resolve_range(
    checkpoint: str,
    club: ClubCategory = ClubCategory.ALL,
    profile: PlayerProfile | None = None,
) -> ResolvedRange | None:
    """Resolve the expected band for a checkpoint, most-specific → least-specific.

    Fallback order (ADR-010 §2): the requested `(club, skill)` first, then progressively
    generalized to `(club, all)`, `(all, skill)`, and finally `(all, all)`. Returns the
    first match, or `None` if the store has no row for this checkpoint at any specificity —
    the caller then produces no score for it (never a wrong one).
    """
    skill = (profile.skill_level if profile else "all")
    candidates = [
        (club, skill),
        (club, "all"),
        (ClubCategory.ALL, skill),
        (ClubCategory.ALL, "all"),
    ]
    rows = [r for r in _load_ranges() if r.checkpoint == checkpoint]
    for want_club, want_skill in candidates:
        for row in rows:
            if row.club_category == want_club and row.skill_level == want_skill:
                return ResolvedRange(row.low, row.high, row.source)
    return None
