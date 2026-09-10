"""The swing as three moving points, and the seam a calibrated source will produce instead. [M17]

`placements.py`'s counterpart for rotation. A **pivot observation** is one instant of the swing
reduced to the three points the turn happens about — the shoulder-line midpoint, the hip-line
midpoint and the hands — plus the orientation of the two lines through the first two. A **pivot
measurement** is one number computed off the whole path of those points: how far a centre slid off
its axis, how rough its path was, whether the shoulder line ever turned back on itself.

**Why the observation is a contract and not an implementation detail.** True rotation is 3-D and
ADR-011 says this capture tier cannot recover it: two hand-held phones, no calibration, no shared
clock. So M17 ships an explicitly interim instrument — 2-D, per view, never blended — behind the
shape a *calibrated* source would fill in. The golfer is printing fiducial markers for the hitting
area; when those land, the calibrated producer becomes a second producer of `PivotObservation` in
`CALIBRATED_3D`, and the rule checks in `analysis/pivot.py`, this registry and the overlay do not
change. Only the `interim_reason` prose flips. That is the whole reason this module exists rather
than the observations being a private type inside `analysis/pivot.py` (ADR-029 Decision 3).

The consequence to preserve: **the rule checks read `PivotObservation` and never `FrameKeypoints`.**
A check typed over frames is a check only the pose producer can ever feed, and the seam buys
nothing.

**Order is load-bearing**, the same way it is in `placements.py`: `analysis/engine.py` emits these
in registry order and `feedback/coach.py` renders them in it. Append, never insert.

Nothing here is judged. These are `Measurement`s and not `CheckpointScore`s — no band, no
`passed`, no contribution to `overall_score` — because a rotation checkpoint needs a calibrated
instrument and a population, and M17 has neither (ADR-010 §2). They are also registered nowhere
near `METRIC_TARGETS`, so career mode reports them `unavailable` rather than computing a
repeatability statistic over the camera's noise.

Stdlib only, like every other contract (ADR-008).
"""

from __future__ import annotations

from enum import StrEnum
from typing import NamedTuple

from golf_coach.contracts.placements import DOWN_THE_LINE, FACE_ON

#: How many samples one swing is resampled to before any check reads it.
#:
#: **Odd, and that is a contract rather than a taste.** `analysis/trajectory.sample_positions`
#: walks event time as `t = span·i/(steps−1)`, which reaches the middle anchor at an integer `i`
#: only when `steps` is odd — so an even count puts the top of the backswing *between* two samples
#: and the two spans below could not meet on it. 41 is the odd neighbour of the fitted trajectory
#: models' 40 steps, so the two resamplings read the swing at the same temporal resolution.
PIVOT_SAMPLES = 41

#: Address through the top, and the top through impact, as indices into a `PivotObservation` list.
#:
#: Event time is the index space and the anchors are integers in it: sample 0 is address, the
#: middle sample is the top, the last is impact. That is why the checks need no phase lookup —
#: and why they must not attempt one. `measure.phase_bounds` returns *frame* indices off a
#: `list[PhaseSegment]`; these are sample indices off a resampled path, the down-the-line view has
#: no phases at all, and the two index spaces look alike enough to be swapped by accident.
#:
#: The top belongs to both spans: a reversal is a move *between* consecutive samples, so the pair
#: spanning the top is the backswing's last and the downswing's first.
BACKSWING_SPAN = range(PIVOT_SAMPLES // 2 + 1)
DOWNSWING_SPAN = range(PIVOT_SAMPLES // 2, PIVOT_SAMPLES)


class FrameOfReference(StrEnum):
    """Which space a `PivotObservation`'s coordinates live in.

    The seam, in one field. Two image-plane values ship today and carry no metric scale: an
    uncalibrated phone's pixels, foreshortened by wherever it happened to stand. `CALIBRATED_3D` is
    what a fiducial-calibrated source will declare, and the only value under which a rotation
    number could ever earn a band.

    Two image-plane members rather than one because a face-on and a down-the-line observation are
    **two scales that must never be blended** — the same rule the `_dtl` name suffix carries into
    the registry below.
    """

    IMAGE_PLANE_FACE_ON = "image_plane_face_on"
    IMAGE_PLANE_DTL = "image_plane_dtl"
    CALIBRATED_3D = "calibrated_3d"


class PivotObservation(NamedTuple):
    """One resampled instant of the swing: three points, two line orientations, one space.

    A `NamedTuple` for the same reason `PlacementSpec` is one — it is arithmetic, not a boundary
    shape; nothing parses one from JSON. A swing is `PIVOT_SAMPLES` of these in event time, so they
    are allocated in the thousands per analysis run.
    """

    #: Shoulder-line midpoint, `(x, y)` in `frame_of_reference`.
    shoulder: tuple[float, float]
    #: Hip-line midpoint, `(x, y)` in `frame_of_reference`. The best-tracked of the three.
    hip: tuple[float, float]
    #: Midpoint of both wrists, `(x, y)`, or `None` where the pair is tracked too sparsely to be
    #: bridged at all. Drawn by the overlay and deliberately unmeasured: M14 P3 put face-on hand
    #: tracking at 0.63-0.68 over a whole clip, so a through-swing hand-path number would report the
    #: tracker's noise as the golfer's.
    #:
    #: **Optional for exactly that reason, and it took a corpus run to see it** (M17 P5). The two
    #: line orientations above go `None` where the projection collapses; this one goes `None` where
    #: the tracker was never confident. When the three points shared one interpolation gate, the
    #: hands failing it refused the *whole swing* — the landmark nothing measures holding a veto
    #: over the five metrics that ignore it, and every face-on swing in the corpus recorded no
    #: rotation number at all. A consumer that wants a hand path must handle its absence; the
    #: shoulder and hip midpoints are never None, because a swing that cannot produce them is not an
    #: observation.
    hands: tuple[float, float] | None
    #: Shoulder-line orientation as a **unit** vector, or `None` where the two landmarks collapsed
    #: within `measure.MIN_DIRECTION_LENGTH`.
    #:
    #: Not an angle in degrees, and a later reader will want to "simplify" it to one. A 2-D line
    #: angle here passes through a **projection singularity**: face-on the shoulder line is
    #: full-width at address and collapses toward zero width at the top, down-the-line it is the
    #: reverse, and `atan2` is worst-conditioned exactly where the segment is shortest — which is
    #: also where MediaPipe is estimating an occluded shoulder. A reversal check reading raw
    #: degrees fires hardest on the *cleanest* turns. `measure.direction_series` already states the
    #: rule and returns unit vectors for it; keeping the gate in the shape means five checks cannot
    #: each forget it. Degrees are derived once, at the measurement boundary.
    shoulder_line: tuple[float, float] | None
    #: Hip-line orientation, same rule.
    hip_line: tuple[float, float] | None
    #: Which space the coordinates above are in. Every consumer that compares two observations must
    #: check this first — two frames of reference are two instruments.
    frame_of_reference: FrameOfReference
    #: Left for a future club detector to write. Nothing in M17 populates it, no measurement reads
    #: it, and the overlay draws no club: the one detection spike (ADR-017) was a no-go on exposure
    #: time. The slot is here so that detector needs no contract change.
    club_head: tuple[float, float] | None = None


class PivotMeasurementSpec(NamedTuple):
    """One pivot measurement's identity, and the two facts that decide how it may be read."""

    #: The `Measurement.name` this is recorded under; consumers match on it, so it is an
    #: identifier. The `_dtl` suffix is part of the name and not a `view` field on the measurement,
    #: for `placements.py`'s reason: `analysis/baseline.py::pooled_samples` groups a golfer's
    #: history by name, and one name for two cameras would pool two scales into one baseline.
    #:
    #: The `pivot_` prefix is load-bearing in the small way that matters in an append-only
    #: registry: `PIVOTS_BY_NAME` is the lookup, but a reader scanning a measurement list should
    #: not need one to see what a row is.
    name: str
    #: The same quantity in a sentence, for prose a model reads.
    label: str
    #: Which camera produced it — `FACE_ON` or `DOWN_THE_LINE`, spelled as `placements.py` spells
    #: them. Never compare a value against the other view's: two cameras, two scales.
    view: str
    #: `Measurement.unit`, so the registry and the emitted measurement cannot disagree.
    unit: str
    #: `Measurement.detail` — how and where it was taken, and the sign convention. Fixed prose, and
    #: that is the difference from `PlacementSpec`, whose detail is written at the engine call site
    #: because it interpolates a percentile. Nothing here varies per swing.
    detail: str
    #: The unsuffixed key of the rule in `analysis/pivot.PIVOT_CHECKS` that computes it. The
    #: face-on and `_dtl` members of a pair name the **same** check: the view decides the name and
    #: the check does not know which camera it is reading, which is what lets one implementation
    #: serve two views without string surgery on the `_dtl` suffix.
    check: str
    #: The sentence a `CALIBRATED_3D` producer would clear — why this number is provisional, said
    #: in terms of what is missing rather than as a hedge. Derived into caveat prose (P6) rather
    #: than typed into it, so growing the registry cannot leave the prose behind: that is the M6.5
    #: failure `placements.py` was written to stop.
    interim_reason: str


# The two positional metrics are ratios of a horizontal excursion to a horizontal shoulder width,
# so the frame's pixel aspect cancels; what does not cancel is the foreshortening, which changes
# with where the phone stood. Said once here and quoted by four specs.
_POSITIONAL_INTERIM = (
    "measured in one uncalibrated phone's image plane: the ratio cancels the pixel aspect but not "
    "the foreshortening, which changes with where the camera stood, so it compares within a camera "
    "position and not across sessions or views. Fiducial calibration clears it."
)

# The angle metrics carry the projection singularity as well, and it sits at opposite ends of the
# swing in the two views — which is why each view states its own half. This narrows the general
# "down-the-line sees rotation best" reading, which holds for the centre paths and not for these.
_ANGLE_INTERIM_FACE_ON = (
    "a projected 2-D line angle, not a turn: face-on the shoulder line collapses toward zero width "
    "at the top of the backswing, so this reads the half of the swing its camera sees squarely and "
    "is worst-conditioned at the other end. Its down-the-line partner is conditioned the opposite "
    "way; read the pair, never one alone. Fiducial calibration clears it."
)
_ANGLE_INTERIM_DTL = (
    "a projected 2-D line angle, not a turn: down-the-line the shoulder line is collapsed at "
    "address and opens through the turn, so this reads the half of the swing its camera sees "
    "squarely and is worst-conditioned at the other end. Its face-on partner is conditioned the "
    "opposite way; read the pair, never one alone. Fiducial calibration clears it."
)

#: Every pivot measurement that ships, in the order `analysis/engine.py` records them: the face-on
#: view, then the down-the-line one, matching how `placements.py` groups its own.
#:
#: Five checks, ten metrics. Backswing and downswing reversal are two names rather than one because
#: `analysis/baseline.py::pooled_samples` groups by name — one name for two windows pools two
#: distributions, and a firing value could not say which half of the swing produced it.
#:
#: Jitter is on the hip centre only. It is the best-tracked of the three pivot points; the hands
#: are drawn and unmeasured, on purpose (see `PivotObservation.hands`).
PIVOT_MEASUREMENT_REGISTRY: tuple[PivotMeasurementSpec, ...] = (
    PivotMeasurementSpec(
        "pivot_hip_axis_drift_norm",
        "hip centre drift off its axis",
        FACE_ON,
        "shoulder_widths",
        "peak lateral excursion of the hip centre from its address x, over the resampled swing "
        "(address -> impact); unsigned, so handedness does not enter it",
        "hip_axis_drift",
        _POSITIONAL_INTERIM,
    ),
    PivotMeasurementSpec(
        "pivot_shoulder_axis_drift_norm",
        "shoulder centre drift off its axis",
        FACE_ON,
        "shoulder_widths",
        "peak lateral excursion of the shoulder centre from its address x, over the resampled "
        "swing (address -> impact); unsigned",
        "shoulder_axis_drift",
        _POSITIONAL_INTERIM,
    ),
    PivotMeasurementSpec(
        "pivot_hip_path_jitter_norm",
        "hip centre path roughness",
        FACE_ON,
        "shoulder_widths",
        "sample-to-sample roughness of the hip centre's path over the resampled swing; unsigned, "
        "0 is a perfectly smooth path",
        "hip_path_jitter",
        _POSITIONAL_INTERIM,
    ),
    PivotMeasurementSpec(
        "pivot_shoulder_reversal_backswing_deg",
        "shoulder line reversal in the backswing",
        FACE_ON,
        "degrees",
        "largest against-the-turn move of the shoulder line between consecutive samples within "
        "the backswing (address -> top); magnitude only, 0 is a monotone turn",
        "shoulder_reversal_backswing",
        _ANGLE_INTERIM_FACE_ON,
    ),
    PivotMeasurementSpec(
        "pivot_shoulder_reversal_downswing_deg",
        "shoulder line reversal in the downswing",
        FACE_ON,
        "degrees",
        "largest against-the-turn move of the shoulder line between consecutive samples within "
        "the downswing (top -> impact); magnitude only, 0 is a monotone turn",
        "shoulder_reversal_downswing",
        _ANGLE_INTERIM_FACE_ON,
    ),
    PivotMeasurementSpec(
        "pivot_hip_axis_drift_norm_dtl",
        "down-the-line hip centre drift off its axis",
        DOWN_THE_LINE,
        "shoulder_widths",
        "peak lateral excursion of the hip centre from its address x, over the resampled swing "
        "(address -> impact); unsigned",
        "hip_axis_drift",
        _POSITIONAL_INTERIM,
    ),
    PivotMeasurementSpec(
        "pivot_shoulder_axis_drift_norm_dtl",
        "down-the-line shoulder centre drift off its axis",
        DOWN_THE_LINE,
        "shoulder_widths",
        "peak lateral excursion of the shoulder centre from its address x, over the resampled "
        "swing (address -> impact); unsigned",
        "shoulder_axis_drift",
        _POSITIONAL_INTERIM,
    ),
    PivotMeasurementSpec(
        "pivot_hip_path_jitter_norm_dtl",
        "down-the-line hip centre path roughness",
        DOWN_THE_LINE,
        "shoulder_widths",
        "sample-to-sample roughness of the hip centre's path over the resampled swing; unsigned, "
        "0 is a perfectly smooth path",
        "hip_path_jitter",
        _POSITIONAL_INTERIM,
    ),
    PivotMeasurementSpec(
        "pivot_shoulder_reversal_backswing_deg_dtl",
        "down-the-line shoulder line reversal in the backswing",
        DOWN_THE_LINE,
        "degrees",
        "largest against-the-turn move of the shoulder line between consecutive samples within "
        "the backswing (address -> top); magnitude only, 0 is a monotone turn",
        "shoulder_reversal_backswing",
        _ANGLE_INTERIM_DTL,
    ),
    PivotMeasurementSpec(
        "pivot_shoulder_reversal_downswing_deg_dtl",
        "down-the-line shoulder line reversal in the downswing",
        DOWN_THE_LINE,
        "degrees",
        "largest against-the-turn move of the shoulder line between consecutive samples within "
        "the downswing (top -> impact); magnitude only, 0 is a monotone turn",
        "shoulder_reversal_downswing",
        _ANGLE_INTERIM_DTL,
    ),
)

#: Name -> spec, for the consumers that hold a `Measurement` and need to know whether it is a pivot
#: row. Built once rather than scanned per lookup: `api/state.py::resolve_pivots` and
#: `feedback/coach.py` both partition a whole measurement list through it.
PIVOTS_BY_NAME: dict[str, PivotMeasurementSpec] = {
    spec.name: spec for spec in PIVOT_MEASUREMENT_REGISTRY
}


def pivot_measurement_names() -> tuple[str, ...]:
    """Registered pivot measurement names, in the order they are recorded."""
    return tuple(spec.name for spec in PIVOT_MEASUREMENT_REGISTRY)


def spec_for(name: str) -> PivotMeasurementSpec:
    """The spec registered under `name`.

    Raises `KeyError` rather than returning `None`, matching `placements.spec_for` and
    `checkpoints.spec_for`: a caller here holds a name that came out of the registry, so a miss is
    a wiring bug and not a data condition. Use `PIVOTS_BY_NAME.get` for the "is this a pivot row?"
    question, which *is* a data one.
    """
    spec = PIVOTS_BY_NAME.get(name)
    if spec is None:
        raise KeyError(f"no pivot measurement registered under {name!r}")
    return spec
