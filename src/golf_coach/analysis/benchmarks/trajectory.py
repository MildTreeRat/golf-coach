"""The tour trajectory model — is this the *shape* of a swing that works? [M8.1]

`joint.py` places six scalars as a combination. This places the **whole motion**: every tracked
landmark, over normalised event time, against a basis fitted on 415 face-on tour swings from 116
golfers.

Two statistics, and they answer different questions:

- **T² — distance inside the model's own subspace.** An unusual *amount* of the things tour swings
  do vary in. Calibrated: leave-one-player-out exceedance of p90 is 9.9% against a 10% target.
- **Q — the reconstruction residual, distance *off* the subspace.** A shape the tour basis cannot
  represent at all. This is the one nothing else here can express: not "too much of a normal
  thing" but "a thing this population does not do".

**Q is not calibrated, and the artifact says so.** Its leave-one-player-out exceedance is 12%
against the same 10% target, because a golfer the basis never saw has idiosyncrasies that land in
the residual *by construction*. Q therefore partly measures "a different person" rather than "a
worse swing". Both figures ship inside the artifact under
`leave_one_player_out_exceedance`; read them before quoting either number at a golfer.

Everything ADR-012 says about the bands applies here: the population is tour professionals only, so
this is "how far from tour", never "how wrong". Off the scoring path, like every other placement
(ADR-010 §2).

Stdlib + pydantic. Fitting is `scripts/golfdb/derive_trajectory_model.py` under the `research`
extra; what ships is the basis and the arithmetic to project onto it (ADR-022).
"""

from __future__ import annotations

import json
import math
from functools import lru_cache
from importlib import resources

from pydantic import BaseModel, TypeAdapter

from golf_coach.analysis.trajectory import anchors_from_phases, build_trajectory
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.swing import PhaseSegment

_MODEL_FILE = "trajectory_model_v1.json"


class TrajectoryPlacement(BaseModel):
    """Where one swing's motion sits against the tour basis."""

    t2: float
    t2_percentile: float
    t2_percentile_clamped: bool
    q: float
    q_percentile: float
    q_percentile_clamped: bool
    population_n: int
    population_players: int
    #: Share of the squared reconstruction error falling in each anchor interval, largest first.
    #: This is the "when" — an unusual shape that is all in `top->impact` is a different fault from
    #: the same magnitude spread evenly, and no scalar checkpoint can make that distinction.
    residual_by_interval: dict[str, float]


class TrajectoryModel(BaseModel):
    """Mean trajectory, principal-component basis, and the population's own distance quantiles."""

    kind: str
    landmarks: list[str]
    axes: list[str]
    events: list[str]
    steps: int
    n: int
    n_players: int
    dimensions: int
    components: int
    explained_variance: float
    mean: list[float]
    scale: list[float]
    basis: list[list[float]]
    t2_quantiles: dict[str, float]
    q_quantiles: dict[str, float]
    leave_one_player_out_exceedance: dict[str, float]

    def placement_for(
        self,
        keypoints: list[FrameKeypoints],
        phases: list[PhaseSegment],
        *,
        left_handed: bool = False,
    ) -> TrajectoryPlacement | None:
        """Project one swing onto the basis, or `None` when it cannot be built.

        Returns `None` rather than a guess whenever the anchors collapse or a landmark is missing
        too much of its timeline — ADR-010 §2's rule, applied to a vector instead of a scalar.
        """
        anchors = anchors_from_phases(phases)
        if anchors is None:
            return None
        vector = build_trajectory(
            keypoints, anchors, self.steps, self.landmarks, self.axes, mirror=left_handed
        )
        if vector is None or len(vector) != self.dimensions:
            return None

        centered = [v - m for v, m in zip(vector, self.mean, strict=True)]
        scores = [sum(b * c for b, c in zip(row, centered, strict=True)) for row in self.basis]

        t2 = math.sqrt(sum((s / sd) ** 2 for s, sd in zip(scores, self.scale, strict=True)))

        reconstructed = [
            sum(scores[k] * self.basis[k][d] for k in range(len(scores)))
            for d in range(self.dimensions)
        ]
        residual = [c - r for c, r in zip(centered, reconstructed, strict=True)]
        q = math.sqrt(sum(r * r for r in residual))

        t2_pct, t2_clamped = _percentile(t2, self.t2_quantiles)
        q_pct, q_clamped = _percentile(q, self.q_quantiles)
        return TrajectoryPlacement(
            t2=round(t2, 4),
            t2_percentile=round(t2_pct, 1),
            t2_percentile_clamped=t2_clamped,
            q=round(q, 4),
            q_percentile=round(q_pct, 1),
            q_percentile_clamped=q_clamped,
            population_n=self.n,
            population_players=self.n_players,
            residual_by_interval=self._by_interval(residual),
        )

    def _by_interval(self, residual: list[float]) -> dict[str, float]:
        """Split the squared residual across the anchor intervals it falls in.

        The vector is row-major by timestep, so a timestep's slice is contiguous and each sample's
        position in event time says which interval it belongs to.
        """
        per_step = len(self.landmarks) * len(self.axes)
        spans = len(self.events) - 1
        totals = dict.fromkeys(
            [f"{a}->{b}" for a, b in zip(self.events, self.events[1:], strict=False)], 0.0
        )
        names = list(totals)
        for step in range(self.steps):
            t = spans * step / (self.steps - 1)
            interval = names[min(int(t), spans - 1)]
            chunk = residual[step * per_step : (step + 1) * per_step]
            totals[interval] += sum(v * v for v in chunk)

        grand = sum(totals.values())
        if grand <= 0:
            return dict.fromkeys(names, 0.0)
        shares = {k: round(v / grand, 4) for k, v in totals.items()}
        return dict(sorted(shares.items(), key=lambda kv: -kv[1]))


def _percentile(value: float, quantiles: dict[str, float]) -> tuple[float, bool]:
    """Where `value` falls among the population's own, in `[10, 90]`, clamped at the edges.

    Same five stored points and the same clamp as `Distribution.percentile_of`: the tails were
    never stored, so past them the honest report is "at least this unusual" rather than a
    precise-looking extrapolation.
    """
    points = [(float(p), quantiles[f"p{p}"]) for p in (10, 25, 50, 75, 90)]
    if value <= points[0][1]:
        return 10.0, True
    if value >= points[-1][1]:
        return 90.0, True
    for (low_pct, low_val), (high_pct, high_val) in zip(points, points[1:], strict=False):
        if low_val <= value <= high_val:
            if high_val == low_val:
                return low_pct, False
            return low_pct + (value - low_val) / (high_val - low_val) * (high_pct - low_pct), False
    return 50.0, False


class TrajectoryDatasetInfo(BaseModel):
    """Provenance for the artifact — what produced it, and under what terms."""

    name: str
    citation: str
    url: str
    license_note: str
    pose_estimator: str | list[str] | None = None
    metric_definitions_version: int
    min_samples: int
    derived_on: str
    pipeline_commit: str


_MODEL_ADAPTER = TypeAdapter(TrajectoryModel)


@lru_cache(maxsize=1)
def _load() -> tuple[TrajectoryDatasetInfo, TrajectoryModel]:
    raw = resources.files(__package__).joinpath(_MODEL_FILE).read_text(encoding="utf-8")
    payload = json.loads(raw)
    return (
        TrajectoryDatasetInfo.model_validate(payload["dataset"]),
        _MODEL_ADAPTER.validate_python(payload["model"]),
    )


def trajectory_dataset_info() -> TrajectoryDatasetInfo:
    """Provenance of the corpus this basis was fitted on."""
    return _load()[0]


def load_trajectory_model() -> TrajectoryModel:
    """The fitted tour trajectory model."""
    return _load()[1]


def trajectory_placement_for(
    keypoints: list[FrameKeypoints],
    phases: list[PhaseSegment],
    *,
    left_handed: bool = False,
) -> TrajectoryPlacement | None:
    """Convenience: place one swing's motion against the tour basis."""
    return load_trajectory_model().placement_for(keypoints, phases, left_handed=left_handed)
