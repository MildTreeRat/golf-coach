"""The tour joint-distribution model — how unusual is this *combination*? [M8-JOINT]

`ranges.json` says whether each of six numbers is in band. `golfdb_v1.json` says where each one
sits in the tour population. Neither can say anything about the six **together**, and that is the
gap this closes: a swing can pass all six bands individually while sitting at a combination no
tour player produces, because six independent range checks have no way to express "this is fine
only if that is also true".

There is something to express. Over the 458 face-on clips the bands were cut from,
`head_sway_norm` and `hip_shift_at_top_norm` correlate at +0.44 and `head_sway_norm` and
`head_hip_gain_norm` at -0.39 (`scripts/golfdb/tune_joint_structure.py`, the gate that had to pass
before this file was written).

**Unusual is not bad.** Every clip behind the model is a tour professional, so it knows the shape
of swings that work and nothing at all about swings that don't — the same limit ADR-012 records
for the bands. The output is therefore a percentile within the tour population, never a score, and
`placement_for` is not on the scoring path any more than `percentile_of` is (ADR-010 §2).

Stdlib + pydantic. The fitting lives in `scripts/golfdb/derive_joint_model.py` under the `research`
extra; what ships is the fitted artifact as committed JSON and the arithmetic to evaluate it, which
is the same division `ranges.json` and `golfdb_v1.json` already follow (ADR-022).
"""

from __future__ import annotations

import json
import math
from collections.abc import Mapping
from functools import lru_cache
from importlib import resources

from pydantic import BaseModel, TypeAdapter

_MODEL_FILE = "joint_model_v1.json"


class JointPlacement(BaseModel):
    """Where one swing's six metrics sit, as a combination, in the tour population."""

    distance: float
    percentile: float
    percentile_clamped: bool
    population_n: int
    population_players: int
    #: Metric -> share of the squared distance it accounts for, largest first. Shares sum to 1.0
    #: and may be negative: a negative share means that metric is pulling the swing *towards* the
    #: population given the others, which is a real thing a correlated model can say and a band
    #: cannot.
    contributions: dict[str, float]


class JointModel(BaseModel):
    """A robust center, a robust scale, and the inverse correlation of the tour population."""

    kind: str
    metrics: list[str]
    n: int
    n_players: int
    clip_z: float
    center: list[float]
    scale: list[float]
    precision: list[list[float]]
    distance_quantiles: dict[str, float]

    def _standardize(self, observed: Mapping[str, float]) -> list[float] | None:
        """Robust z per metric, clipped as the fit was, or `None` if any metric is absent.

        **All six or nothing.** A Mahalanobis distance over a subset is a different quantity —
        the marginal distribution of those metrics — and computing it would need the covariance
        re-inverted for that subset. Returning `None` and letting the caller name what was missing
        is the same choice `unscored` makes everywhere else: no number beats a wrong one
        (ADR-010 §2). In practice this abstains most often when tempo is absent, which is the
        ~14% of clips where address detection declines to guess.
        """
        values: list[float] = []
        for index, metric in enumerate(self.metrics):
            value = observed.get(metric)
            if value is None:
                return None
            z = (value - self.center[index]) / self.scale[index]
            values.append(max(-self.clip_z, min(self.clip_z, z)))
        return values

    def placement_for(self, observed: Mapping[str, float]) -> JointPlacement | None:
        """Distance, percentile and per-metric decomposition, or `None` if a metric is missing."""
        z = self._standardize(observed)
        if z is None:
            return None

        # precision @ z, then z . (precision @ z) — the quadratic form, term by term so each
        # metric's share of it falls out of the same arithmetic rather than a second pass.
        weighted = [
            sum(self.precision[i][j] * z[j] for j in range(len(z))) for i in range(len(z))
        ]
        terms = [z[i] * weighted[i] for i in range(len(z))]
        squared = sum(terms)
        distance = math.sqrt(max(squared, 0.0))

        shares = (
            {m: round(t / squared, 4) for m, t in zip(self.metrics, terms, strict=True)}
            if squared > 0
            else dict.fromkeys(self.metrics, 0.0)
        )
        percentile, clamped = self._percentile_of(distance)
        return JointPlacement(
            distance=round(distance, 4),
            percentile=round(percentile, 1),
            percentile_clamped=clamped,
            population_n=self.n,
            population_players=self.n_players,
            contributions=dict(sorted(shares.items(), key=lambda kv: -abs(kv[1]))),
        )

    def _percentile_of(self, distance: float) -> tuple[float, bool]:
        """Where this distance falls among tour swings' own distances, in `[10, 90]`.

        Same five stored quantiles and the same clamp as `Distribution.percentile_of`, for the
        same reason: the tails were never stored, so a swing past them is reported as "at least
        this unusual" rather than extrapolated. The clamp matters more here than there — a swing
        far outside the population is exactly the interesting case, and reporting it as 90 is a
        floor on its strangeness, not a measurement of it.
        """
        points = [
            (10.0, self.distance_quantiles["p10"]),
            (25.0, self.distance_quantiles["p25"]),
            (50.0, self.distance_quantiles["p50"]),
            (75.0, self.distance_quantiles["p75"]),
            (90.0, self.distance_quantiles["p90"]),
        ]
        if distance <= points[0][1]:
            return 10.0, True
        if distance >= points[-1][1]:
            return 90.0, True
        for (low_pct, low_val), (high_pct, high_val) in zip(points, points[1:], strict=False):
            if low_val <= distance <= high_val:
                if high_val == low_val:
                    return low_pct, False
                span = (distance - low_val) / (high_val - low_val)
                return low_pct + span * (high_pct - low_pct), False
        return 50.0, False


class JointDatasetInfo(BaseModel):
    """Provenance for the artifact — what produced these numbers, and under what terms."""

    name: str
    citation: str
    url: str
    license_note: str
    pose_estimator: str | list[str] | None = None
    metric_definitions_version: int
    min_samples: int
    derived_on: str
    pipeline_commit: str


_MODEL_ADAPTER = TypeAdapter(JointModel)


@lru_cache(maxsize=1)
def _load() -> tuple[JointDatasetInfo, JointModel]:
    raw = resources.files(__package__).joinpath(_MODEL_FILE).read_text(encoding="utf-8")
    payload = json.loads(raw)
    return (
        JointDatasetInfo.model_validate(payload["dataset"]),
        _MODEL_ADAPTER.validate_python(payload["model"]),
    )


def joint_dataset_info() -> JointDatasetInfo:
    """Provenance of the corpus this model was fitted on."""
    return _load()[0]


def load_joint_model() -> JointModel:
    """The fitted tour joint-distribution model."""
    return _load()[1]


def placement_for(observed: Mapping[str, float]) -> JointPlacement | None:
    """Convenience: place one swing's measured metrics in the tour joint distribution."""
    return load_joint_model().placement_for(observed)
