"""Benchmark range store — the public surface is the resolver. [M4-PoC]

Ranges are versioned data with provenance (ADR-010); consumers ask `resolve_range` rather
than reading constants. See `store.py` for the fallback semantics.

`distributions` exposes the reference population the bands were cut from, at full resolution
(M4-REF). Scoring does not read it — it is for saying *how far* off a swing is, and for re-cutting
bands without re-deriving them. See `distributions.py`.

`joint` answers the question neither of the other two can: whether the six metrics **together**
form a combination the tour population produces. Also off the scoring path (M8-JOINT, ADR-022).
See `joint.py`.

`flight_model` is the odd one out: it holds no tour population at all. It is the published
aerodynamic constants `analysis/flight.py` integrates a ball with — nothing fitted, nothing
scored, and nothing about a golfer (M15 P2, ADR-027). See `flight_model.py`.
"""

from golf_coach.analysis.benchmarks.distributions import (
    DatasetInfo,
    Distribution,
    dataset_info,
    load_distribution,
)
from golf_coach.analysis.benchmarks.flight_model import (
    AeroCoefficients,
    AeroRow,
    AeroTable,
    Atmosphere,
    AtmosphereProfile,
    BallSpec,
    FlightDatasetInfo,
    FlightModel,
    SourceNote,
    SpinDecay,
    coefficients_for,
    flight_dataset_info,
    load_flight_model,
)
from golf_coach.analysis.benchmarks.joint import (
    JointDatasetInfo,
    JointModel,
    JointPlacement,
    joint_dataset_info,
    load_joint_model,
    placement_for,
)
from golf_coach.analysis.benchmarks.store import ResolvedRange, resolve_range
from golf_coach.analysis.benchmarks.trajectory import (
    TrajectoryDatasetInfo,
    TrajectoryModel,
    TrajectoryPlacement,
    load_trajectory_model,
    trajectory_dataset_info,
    trajectory_placement_for,
)

__all__ = [
    "AeroCoefficients",
    "AeroRow",
    "AeroTable",
    "Atmosphere",
    "AtmosphereProfile",
    "BallSpec",
    "DatasetInfo",
    "Distribution",
    "FlightDatasetInfo",
    "FlightModel",
    "JointDatasetInfo",
    "JointModel",
    "JointPlacement",
    "ResolvedRange",
    "SourceNote",
    "SpinDecay",
    "TrajectoryDatasetInfo",
    "TrajectoryModel",
    "TrajectoryPlacement",
    "coefficients_for",
    "dataset_info",
    "flight_dataset_info",
    "joint_dataset_info",
    "load_distribution",
    "load_flight_model",
    "load_joint_model",
    "load_trajectory_model",
    "placement_for",
    "resolve_range",
    "trajectory_dataset_info",
    "trajectory_placement_for",
]
