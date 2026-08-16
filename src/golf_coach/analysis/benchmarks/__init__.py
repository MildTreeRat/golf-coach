"""Benchmark range store — the public surface is the resolver. [M4-PoC]

Ranges are versioned data with provenance (ADR-010); consumers ask `resolve_range` rather
than reading constants. See `store.py` for the fallback semantics.

`distributions` exposes the reference population the bands were cut from, at full resolution
(M4-REF). Scoring does not read it — it is for saying *how far* off a swing is, and for re-cutting
bands without re-deriving them. See `distributions.py`.

`joint` answers the question neither of the other two can: whether the six metrics **together**
form a combination the tour population produces. Also off the scoring path (M8-JOINT, ADR-022).
See `joint.py`.
"""

from golf_coach.analysis.benchmarks.distributions import (
    DatasetInfo,
    Distribution,
    dataset_info,
    load_distribution,
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

__all__ = [
    "DatasetInfo",
    "Distribution",
    "JointDatasetInfo",
    "JointModel",
    "JointPlacement",
    "ResolvedRange",
    "dataset_info",
    "joint_dataset_info",
    "load_distribution",
    "load_joint_model",
    "placement_for",
    "resolve_range",
]
