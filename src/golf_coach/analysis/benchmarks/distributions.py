"""Reference distributions behind the benchmark bands. [M4-REF]

`ranges.json` says *whether* a swing is in band. This says **where in the population it sits** —
the same GolfDB-derived data the bands were cut from, kept at full resolution (n, distinct
golfers, mean, sd, p10/p25/p50/p75/p90) rather than collapsed to two numbers.

Two reasons it is worth storing more than `low`/`high`:

- **Feedback with a sense of scale.** "Your tempo is quicker than 90% of tour swings" is a
  different sentence from "outside 2.72-4.71", and only one of them tells a golfer how far off
  they are. `percentile_of` turns an observed value into that statement.
- **Re-cutting bands without re-deriving them.** Tightening a band to p25-p75 becomes a lookup
  rather than a re-run of the corpus.

Deliberately **not** on the `resolve_range` hot path — scoring reads `ranges.json` and nothing
here — so this file can grow richer without touching how a swing is scored (ADR-010 §2).

Stdlib + pydantic, loaded from the package via `importlib.resources` exactly like `store.py`.
"""

from __future__ import annotations

import json
from functools import lru_cache
from importlib import resources

from pydantic import BaseModel, TypeAdapter

_DISTRIBUTIONS_FILE = "golfdb_v1.json"

_ANY = "all"


class Distribution(BaseModel):
    """The observed spread of one metric over one stratum of the reference corpus."""

    metric: str
    club: str
    sex: str
    view: str

    n: int
    n_players: int
    mean: float
    sd: float
    p10: float
    p25: float
    p50: float
    p75: float
    p90: float

    def percentile_of(self, observed: float) -> float:
        """Roughly where `observed` falls in this population, as a percentile in `[0, 100]`.

        Interpolates between the stored quantiles and clamps outside p10/p90 — the tails aren't
        stored, so a value past them is reported as "at least this extreme" rather than
        extrapolated into a precise-looking number the data can't support.
        """
        points = ((10.0, self.p10), (25.0, self.p25), (50.0, self.p50), (75.0, self.p75),
                  (90.0, self.p90))
        if observed <= self.p10:
            return 10.0
        if observed >= self.p90:
            return 90.0
        for (low_pct, low_val), (high_pct, high_val) in zip(points, points[1:], strict=False):
            if low_val <= observed <= high_val:
                if high_val == low_val:
                    return low_pct
                span = (observed - low_val) / (high_val - low_val)
                return low_pct + span * (high_pct - low_pct)
        return 50.0


class DatasetInfo(BaseModel):
    """Provenance for the whole file — what produced these numbers, and under what terms."""

    name: str
    citation: str
    url: str
    license_note: str
    pose_estimator: str | list[str] | None = None
    metric_definitions_version: int
    min_samples: int
    derived_on: str
    pipeline_commit: str


_DISTRIBUTION_LIST_ADAPTER = TypeAdapter(list[Distribution])


@lru_cache(maxsize=1)
def _load() -> tuple[DatasetInfo, list[Distribution]]:
    raw = resources.files(__package__).joinpath(_DISTRIBUTIONS_FILE).read_text(encoding="utf-8")
    payload = json.loads(raw)
    return (
        DatasetInfo.model_validate(payload["dataset"]),
        _DISTRIBUTION_LIST_ADAPTER.validate_python(payload["distributions"]),
    )


def dataset_info() -> DatasetInfo:
    """Provenance of the reference corpus these distributions came from."""
    return _load()[0]


def load_distribution(
    metric: str,
    club: str = _ANY,
    sex: str = _ANY,
    view: str = _ANY,
) -> Distribution | None:
    """The distribution for `metric`, most-specific stratum first, or None if we have none.

    Falls back one axis at a time — `(club, sex, view)`, then each single axis, then the overall
    population — mirroring `resolve_range`'s specific-to-general walk. Returning `None` rather than
    a guess keeps a missing stratum from being reported as a confident percentile.

    Note the corpus stores **marginals**, not the full cross-product, so a fully-specified query
    will normally land on a single-axis fallback. That is intentional: see `derive_reference.py`.
    """
    _, distributions = _load()
    candidates = [
        (club, sex, view),
        (club, _ANY, _ANY),
        (_ANY, sex, _ANY),
        (_ANY, _ANY, view),
        (_ANY, _ANY, _ANY),
    ]
    rows = [d for d in distributions if d.metric == metric]
    for want in candidates:
        for row in rows:
            if (row.club, row.sex, row.view) == want:
                return row
    return None
