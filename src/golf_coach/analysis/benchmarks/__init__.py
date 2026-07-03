"""Benchmark range store — the public surface is the resolver. [M4-PoC]

Ranges are versioned data with provenance (ADR-010); consumers ask `resolve_range` rather
than reading constants. See `store.py` for the fallback semantics.
"""

from golf_coach.analysis.benchmarks.store import ResolvedRange, resolve_range

__all__ = ["resolve_range", "ResolvedRange"]
