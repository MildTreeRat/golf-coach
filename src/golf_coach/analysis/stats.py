"""Small statistical helpers for the analysis core — pure, stdlib only. [M4-REF]

One definition of "percentile", shared by the checkpoint evaluators and by the GolfDB
reference-aggregation script (`scripts/golfdb/derive_reference.py`). Two drifting definitions of
`p90` — one cutting a benchmark band, the other measuring the swing scored against it — would
produce a systematic bias that is almost impossible to spot after the fact, so they share this.

Linear interpolation between the two nearest ranks (the same convention as `numpy.percentile`'s
default and R's type-7), chosen so the result is continuous in `q` and matches what anyone
cross-checking these numbers in numpy or pandas will compute.

No numpy: the analysis core runs on the base install (ADR-008).
"""

from __future__ import annotations

from collections.abc import Iterable


def percentile(values: Iterable[float], q: float) -> float:
    """The `q`-quantile of `values` by linear interpolation between adjacent ranks.

    `q` is a fraction in `[0, 1]` (so `0.9` is the 90th percentile). Raises `ValueError` on an
    empty input — an empty sample has no percentile, and silently returning `0.0` would quietly
    poison a benchmark band.
    """
    ordered = sorted(values)
    if not ordered:
        raise ValueError("percentile() requires at least one value")
    if not 0.0 <= q <= 1.0:
        raise ValueError(f"q must be in [0, 1], got {q}")
    if len(ordered) == 1:
        return ordered[0]

    position = q * (len(ordered) - 1)
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = position - lower
    return ordered[lower] + (ordered[upper] - ordered[lower]) * fraction
