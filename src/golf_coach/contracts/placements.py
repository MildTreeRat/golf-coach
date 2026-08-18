"""Which population placements exist, and how each may be spoken. [M8.3]

`checkpoints.py`'s counterpart for the quantities that are deliberately **not** checkpoints. A
placement says where one swing sits against a population of tour professionals — as a combination
of six metrics (the joint model), and as a motion (the trajectory models, one basis per camera).
None of them has a band, none touches `overall_score`, and that is by construction rather than by
omission: the corpus behind them is entirely tour swings, so the models know the shape of swings
that work and nothing whatever about swings that do not. *Unusual is not bad* is the whole reading
rule, and it is the opposite of how a distance-from-the-tour-population number looks.

**Why this half is in `contracts/`, and it is the same argument `checkpoints.py` makes.** M8 fitted
three models and surfaced five numbers onto `SwingResult.measurements`, and nothing in `caveats.py`
said a word about them — so `mcp/query.py` handed an MCP client `tour_trajectory_q_dtl: 11.06` as a
bare float under a field description promising "no percentile", with no way for a reader to know it
is an *uncalibrated residual* whose large value on that swing is a broken anchor rather than a
flaw in the golfer. That is the M6.5 failure repeating in a new channel: prose that has to name a
set, hand-typed, going stale the moment the set grew. Deriving the sentence from a registry deletes
the failure mode instead of fixing this instance of it.

**Order is load-bearing** in the same way: `analysis/engine.py` emits placements in this order and
`feedback/coach.py` renders them in it, so a diff of two briefs stays readable. Append, never
insert.

The view strings live here rather than in `analysis/benchmarks/trajectory.py` because both this
registry and that module need them and `contracts` cannot import `analysis` (ADR-008). Note they
are *not* `storage.manifest.Role`'s vocabulary (`face_on`, underscored): that enum names a file
uploaded from a phone, this names a fitted basis. Two different things that a shared spelling would
quietly merge.
"""

from __future__ import annotations

from typing import NamedTuple

#: The two camera positions that have a fitted population behind them, spelled as
#: `analysis/benchmarks/trajectory.py` keys its artifacts.
FACE_ON = "face-on"
DOWN_THE_LINE = "down-the-line"


class PlacementSpec(NamedTuple):
    """One population placement's identity, and the two facts that decide how it may be read.

    A `NamedTuple` for the same reason `CheckpointSpec` is one: a compile-time constant, never
    parsed from JSON, never validated at a boundary.
    """

    #: The `Measurement.name` this is recorded under. Consumers match on it, so it is an
    #: identifier. The `_dtl` suffix is part of the name rather than a `view` field on the
    #: measurement because `analysis/baseline.py::pooled_samples` groups a golfer's history by
    #: name — one name for two cameras would pool two bases into one personal baseline.
    #:
    #: That pooling happens today and is a **deferred decision, not an endorsement**: ADR-022's
    #: fourth addendum records why a placement may not be a personal quantity at all, and that it
    #: starts publishing a center the first bay session that carries `n` past 5.
    name: str
    #: The same quantity in a sentence, for prose a model reads.
    label: str
    #: Which fitted basis produced it — `FACE_ON` or `DOWN_THE_LINE`. Never compare a value
    #: against the other view's: two cameras, two populations, two scales.
    view: str
    #: `Measurement.unit`, so the registry and the emitted measurement cannot disagree.
    unit: str
    #: Whether its own exceedance rate was validated against the target on held-out players. False
    #: means the number is real but its *rate* is not: the residual models over-flag any golfer the
    #: tour basis never saw, which is every golfer using this system. An uncalibrated placement is
    #: read beside its calibrated partner and never alone.
    calibrated: bool


#: Every population placement that ships, in the order `analysis/engine.py` records them.
#:
#: The two Q entries are uncalibrated on purpose and ship that way rather than being withheld: a
#: residual off a fitted basis is a genuinely different question from a distance inside it ("shape
#: the basis cannot represent" vs "an unusual amount of shape it can"), and the honest handling of
#: a quantity that cannot be calibrated yet is to record it and label it — the same call ADR-010 §2
#: makes for a checkpoint that cannot be measured.
POPULATION_PLACEMENT_REGISTRY: tuple[PlacementSpec, ...] = (
    PlacementSpec(
        "tour_joint_distance",
        "joint distance",
        FACE_ON,
        "sd_units",
        calibrated=True,
    ),
    PlacementSpec(
        "tour_trajectory_t2",
        "trajectory T2",
        FACE_ON,
        "sd_units",
        calibrated=True,
    ),
    PlacementSpec(
        "tour_trajectory_q",
        "trajectory Q",
        FACE_ON,
        "shoulder_widths",
        calibrated=False,
    ),
    PlacementSpec(
        "tour_trajectory_t2_dtl",
        "down-the-line trajectory T2",
        DOWN_THE_LINE,
        "sd_units",
        calibrated=True,
    ),
    PlacementSpec(
        "tour_trajectory_q_dtl",
        "down-the-line trajectory Q",
        DOWN_THE_LINE,
        "shoulder_widths",
        calibrated=False,
    ),
)

#: Name -> spec, for the consumers that hold a `Measurement` and need to know whether it is a
#: placement. Built once rather than scanned per lookup, because `mcp/query.py` and
#: `feedback/coach.py` both partition a whole measurement list through it.
PLACEMENTS_BY_NAME: dict[str, PlacementSpec] = {
    spec.name: spec for spec in POPULATION_PLACEMENT_REGISTRY
}


def placement_names() -> tuple[str, ...]:
    """Registered placement names, in the order they are recorded."""
    return tuple(spec.name for spec in POPULATION_PLACEMENT_REGISTRY)


def spec_for(name: str) -> PlacementSpec:
    """The spec registered under `name`.

    Raises `KeyError` rather than returning `None`, matching `checkpoints.spec_for`: callers here
    hold a name that came out of the registry, so a miss is a wiring bug and not a data condition.
    Use `PLACEMENTS_BY_NAME.get` for the "is this a placement?" question, which *is* a data one.
    """
    spec = PLACEMENTS_BY_NAME.get(name)
    if spec is None:
        raise KeyError(f"no population placement registered under {name!r}")
    return spec
