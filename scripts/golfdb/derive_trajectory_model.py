"""Fit the tour *trajectory* model — the whole motion, not six snapshots. [M8.1 step 3]

Usage:
    python scripts/golfdb/derive_trajectory_model.py [--steps 40] [--components 6]
                                                     [--axes xy|xyz] [--view face-on|down-the-line]
                                                     [--landmarks face-on|down-the-line]
                                                     [--anchors detected|annotated] [--write]

Fits **one model per camera view**, to separate artifacts. The two views do not share a landmark
list: from behind, the lead arm is hidden by the torso, and handing the face-on twelve to a
down-the-line fit discards 72% of the corpus (M4_POSE_BAKEOFF §Phase H).

Defaults are the shipped configuration, and every one of them was chosen by measurement rather
than taste — the sweep is in `docs/M4_POSE_BAKEOFF.md` §Phase E.

Needs the `research` extra (numpy). Reads the `mediapipe-lite` Tier 1 cache.

### What this is for

`joint_model_v1.json` reads six scalars at four instants. The cache behind it holds **461 face-on
clips as full 33-landmark time series**, so that model ignores almost everything already on disk.
More importantly, six scalars cannot answer *when*: a golfer told "your hip number is unusual"
still does not know whether it went wrong at the top or through impact, and "when" is most of what
a coach actually says.

So this fits the shape of the tour swing **as a path through time**, and reports two numbers that
mean different things:

- **T² — distance inside the model's own subspace.** An unusual *combination* of the modes tour
  swings do vary along. The trajectory analogue of what `joint.py` already does.
- **Q — reconstruction residual, the distance *off* the subspace.** A shape the tour basis cannot
  represent at all. A swing can sit at a perfectly ordinary T² and a huge Q, and that is the more
  interesting failure: not "an unusual amount of a normal thing" but "a thing this population does
  not do". Six scalars have no way to express it and neither does `joint.py`.

### Why PCA, and why the dimension count is not a free choice

12 landmarks × 2 axes × 40 timesteps is 960 numbers per swing, against 415 face-on swings (510
down-the-line). Fitting anything in that space directly is impossible — more dimensions than
samples means the covariance is singular and no distance exists. PCA is what makes the problem
well-posed: project onto the handful of directions the population actually varies along, where a
few hundred samples is tens per dimension.

In that projected space the components are uncorrelated by construction, so the joint model
degenerates pleasantly: T² is just the sum of squared component scores divided by their own
standard deviations. No matrix to invert, and the artifact stays readable.

### The `z` question, settled here rather than asserted — and the answer was no

`tune_z_channel.py` passed `z` at a median ratio of 2.69 against `x` 9.86 and `y` 18.09, while
saying plainly that the number flattered it: lite and full are one architecture at two sizes, so
they agree with each other about depth while both guessing. That screen could not settle whether
`z` *helps*. This could, and it did.

Fitting both ways, `z` **lost on every measure at once**: variance explained fell (62.2% → 66.3% in
its absence at 10 components), leave-one-player-out calibration got worse on both statistics, and
the artifact grew by half. 480 extra dimensions of inferred depth dilute the basis rather than
extend it. **`x/y` ships.** Keeping `--axes xyz` costs nothing and means the finding stays
re-runnable instead of becoming folklore.

Like `derive_reference.py`, this **prints before it writes** and touches no committed band.
"""

from __future__ import annotations

import json
import sys
from datetime import date
from typing import Any

import common
import derive_reference
import numpy as np

from golf_coach.analysis import trajectory
from golf_coach.contracts.keypoints import (
    FrameKeypoints,
    Landmark,
    PoseLandmark,
)

SCHEMA_VERSION = 1
_OUTPUT_NAMES = {
    common.VIEW_FACE_ON: "trajectory_model_v1.json",
    "down-the-line": "trajectory_model_dtl_v1.json",
}

_ESTIMATOR_SLUG = "mediapipe-lite"
_ESTIMATOR_NAME = "mediapipe:lite"

# The anchors the time axis is defined against — what makes a slow-motion clip and a real-time clip
# comparable at all, since the corpus is ~47% slow-motion and absolute durations across it are
# meaningless.
#
# **Two sets, and the default is the smaller one for a reason that nearly went unnoticed.** GolfDB
# hand-annotates eight events, and fitting on all eight produces a better-looking model — but four
# of them (`toe_up`, `mid_backswing`, `mid_downswing`, `mid_follow_through`) are annotations this
# project cannot produce. `segment_phases()` emits six phases off its own detection, and the
# instants it locates well are **address, top and impact** (median error 7, 2 and 1 frames,
# `docs/M4_ADDRESS_DETECTION.md`). A model anchored on events that exist only in the corpus can
# never score a golfer's swing; it would be a research artifact wearing a product's name.
#
# So `detected` is the shippable anchor set and the default. `annotated` is kept because it is the
# ceiling — the difference between the two is the cost of our own segmentation, which is worth
# being able to measure rather than assume.
ANCHOR_SETS: dict[str, tuple[str, ...]] = {
    "detected": ("address", "top", "impact"),
    "annotated": common.EVENT_NAMES[1:-1],
}

# Twelve landmarks in six left/right pairs, so mirroring a left-handed golfer is a swap plus a sign
# flip. MediaPipe's face detail (eyes, mouth) and hand detail (pinky, index, thumb) are dropped:
# nothing in the panel reads them and the hands are the noisiest points on the body.
_PAIRS: tuple[tuple[str, PoseLandmark, PoseLandmark], ...] = (
    ("ear", PoseLandmark.LEFT_EAR, PoseLandmark.RIGHT_EAR),
    ("shoulder", PoseLandmark.LEFT_SHOULDER, PoseLandmark.RIGHT_SHOULDER),
    ("elbow", PoseLandmark.LEFT_ELBOW, PoseLandmark.RIGHT_ELBOW),
    ("wrist", PoseLandmark.LEFT_WRIST, PoseLandmark.RIGHT_WRIST),
    ("hip", PoseLandmark.LEFT_HIP, PoseLandmark.RIGHT_HIP),
    ("knee", PoseLandmark.LEFT_KNEE, PoseLandmark.RIGHT_KNEE),
)
_LANDMARKS = [lm for _, left, right in _PAIRS for lm in (left, right)]
_LANDMARK_NAMES = [
    f"{side}_{name}" for name, _, _ in _PAIRS for side in ("left", "right")
]

# Per-view landmark sets. **Not the same twelve, and that is measured rather than stylistic**
# (M4_POSE_BAKEOFF §Phase G): from down-the-line the lead arm crosses the body and the torso hides
# it, so the lead elbow and wrist track in 0.46 and 0.47 of frames against 0.87 and 0.86 on the
# trail side. Two of the face-on twelve are therefore unusable from behind. Ankles take their
# place — visible in ~1.00 of frames from either camera.
#
# Left/right order within each pair is preserved so `analysis/trajectory.py::_mirror` can still
# find a landmark's partner by name. `right_elbow`/`right_wrist` have no left counterpart in the
# DTL set, and `_mirror` maps those to themselves, which is correct: there is nothing to swap with.
LANDMARK_SETS: dict[str, list[str]] = {
    common.VIEW_FACE_ON: list(_LANDMARK_NAMES),
    "down-the-line": [
        "left_ear",
        "right_ear",
        "left_shoulder",
        "right_shoulder",
        "left_hip",
        "right_hip",
        "left_knee",
        "right_knee",
        "left_ankle",
        "right_ankle",
        "right_elbow",
        "right_wrist",
    ],
}

_MIN_VISIBILITY = 0.5
_MIN_SHOULDER_WIDTH = 0.02
# A landmark missing more than this fraction of its timeline is interpolated across too far to
# trust, and the swing is dropped rather than invented.
_MAX_MISSING = 0.40


def _load_frames(source_id: str) -> list[dict[str, Any]] | None:
    path = common.KEYPOINTS_DIR / _ESTIMATOR_SLUG / f"{source_id}.keypoints.json"
    if not path.exists():
        return None
    payload = json.loads(path.read_text(encoding="utf-8"))
    return payload["frames"] if isinstance(payload, dict) else payload


def _event_frames(swing: Any, n_frames: int, events: tuple[str, ...]) -> list[float] | None:
    """The eight event indices, rebased onto the trimmed clip.

    GolfDB stores absolute frame numbers in the source video while the cached clips are trimmed, so
    they must be rebased on `start` exactly as upstream's dataloader does. Getting this wrong does
    not raise — the indices simply run off the end and the corpus quietly empties.
    """
    start = swing.events.get("start")
    if start is None:
        return None
    frames = []
    for event in events:
        absolute = swing.events.get(event)
        if absolute is None:
            return None
        index = absolute - start
        if index < 0 or index >= n_frames:
            return None
        frames.append(float(index))
    # Non-monotonic events mean a mislabelled clip; interpolating across them would smear one
    # phase into another.
    if any(b <= a for a, b in zip(frames, frames[1:], strict=False)):
        return None
    return frames


def _sample_axis(events: list[float], steps: int) -> list[float]:
    """Frame indices for `steps` points evenly spaced in *event* time, not clock time.

    Phase-time runs 0..7 with the integers landing on the eight annotated events; each sample is
    mapped back to a (fractional) frame by interpolating between the two events bracketing it. A
    slow-motion clip and a real-time clip of the same swing therefore produce the same sample
    positions, which is the entire reason this model can pool a corpus that is half slow-motion.
    """
    out = []
    span = len(events) - 1
    for i in range(steps):
        t = span * i / (steps - 1)
        low = min(int(t), span - 1)
        frac = t - low
        out.append(events[low] + frac * (events[low + 1] - events[low]))
    return out


def _interpolate_gaps(series: np.ndarray) -> np.ndarray | None:
    """Linear interpolation across low-visibility gaps, per column.

    Returns None when any column is missing more than `_MAX_MISSING` of its timeline — past that
    the interpolation is inventing the swing rather than bridging a blink.
    """
    filled = series.copy()
    steps = series.shape[0]
    for column in range(series.shape[1]):
        values = filled[:, column]
        missing = np.isnan(values)
        if missing.all() or missing.mean() > _MAX_MISSING:
            return None
        if missing.any():
            index = np.arange(steps)
            values[missing] = np.interp(index[missing], index[~missing], values[~missing])
    return filled


def build_trajectory(
    swing: Any,
    frames: list[dict[str, Any]],
    steps: int,
    axes: tuple[str, ...],
    events: tuple[str, ...],

    landmarks: list[str],
) -> np.ndarray | None:
    """One swing as a flat feature vector, via the package's own builder.

    **The transform itself lives in `analysis/trajectory.py` and is imported, not reimplemented.**
    A feature vector built one way here and another way at scoring time would produce a model
    evaluated against numbers that were never fitted to it — plausible output, no error, nothing to
    catch it. So this function's whole job is the two things that are *corpus* facts rather than
    transform facts: rebasing GolfDB's absolute event indices onto the trimmed clip, and correcting
    pixel aspect.

    `videos_160` resizes a non-square bounding box to a square, so `x` and `y` land on different
    scales per clip (ADR-012's third accepted limit). A metric built from ratios of `x` distances
    cancels that; this model mixes the axes in every component, so an uncorrected `y` would put
    GolfDB's cropping into the basis and the PCA would spend components describing it.
    `derive_pose_metrics.py::_apply_pixel_aspect` does the same for the scalar metrics.
    """
    anchors = _event_frames(swing, len(frames), events)
    if anchors is None:
        return None

    aspect = float(getattr(swing, "pixel_aspect", 1.0) or 1.0)
    keypoints = [
        FrameKeypoints(
            frame_index=frame["frame_index"],
            timestamp_ms=frame["timestamp_ms"],
            landmarks=[
                Landmark(
                    x=lm["x"], y=lm["y"] * aspect, z=lm["z"], visibility=lm["visibility"]
                )
                for lm in frame["landmarks"]
            ],
        )
        for frame in frames
    ]

    vector = trajectory.build_trajectory(
        keypoints, tuple(anchors), steps, list(landmarks), list(axes)
    )
    return None if vector is None else np.array(vector)


def collect(
    swings: list[Any],
    steps: int,
    axes: tuple[str, ...],
    lefties: set[str],
    events: tuple[str, ...],
    landmarks: list[str],
) -> tuple[np.ndarray, list[str], int]:
    """Trajectories for every usable face-on clip, mirrored onto one handedness."""
    rows, subjects, skipped = [], [], 0
    for swing in swings:
        frames = _load_frames(swing.source_id)
        if frames is None:
            skipped += 1
            continue
        vector = build_trajectory(swing, frames, steps, axes, events, landmarks)
        if vector is None:
            skipped += 1
            continue
        if swing.subject in lefties:
            # Re-run through the shared builder with the mirror applied, rather than folding the
            # flat vector here — the swap-and-negate rule is part of the transform and belongs
            # in one place.
            keypoints_vector = build_trajectory_mirrored(
                swing, frames, steps, axes, events, landmarks
            )
            if keypoints_vector is None:
                skipped += 1
                continue
            vector = keypoints_vector
        rows.append(vector)
        subjects.append(swing.subject or "")
    return np.array(rows), subjects, skipped


def build_trajectory_mirrored(
    swing: Any,
    frames: list[dict[str, Any]],
    steps: int,
    axes: tuple[str, ...],
    events: tuple[str, ...],

    landmarks: list[str],
) -> np.ndarray | None:
    """`build_trajectory` with the left-handed fold applied."""
    anchors = _event_frames(swing, len(frames), events)
    if anchors is None:
        return None
    aspect = float(getattr(swing, "pixel_aspect", 1.0) or 1.0)
    keypoints = [
        FrameKeypoints(
            frame_index=frame["frame_index"],
            timestamp_ms=frame["timestamp_ms"],
            landmarks=[
                Landmark(x=lm["x"], y=lm["y"] * aspect, z=lm["z"], visibility=lm["visibility"])
                for lm in frame["landmarks"]
            ],
        )
        for frame in frames
    ]
    vector = trajectory.build_trajectory(
        keypoints, tuple(anchors), steps, list(landmarks), list(axes), mirror=True
    )
    return None if vector is None else np.array(vector)


def fit_pca(matrix: np.ndarray, components: int) -> dict[str, Any]:
    """Mean, basis and component scales. SVD rather than an eigendecomposition of the covariance —
    the covariance here is 1,440 x 1,440 and singular by construction, and never forming it is both
    faster and the only numerically sane route."""
    mean = matrix.mean(axis=0)
    centered = matrix - mean
    _, singular, basis = np.linalg.svd(centered, full_matrices=False)
    basis = basis[:components]
    scores = centered @ basis.T
    return {
        "mean": mean,
        "basis": basis,
        "scale": scores.std(axis=0, ddof=1),
        "explained": float((singular[:components] ** 2).sum() / (singular**2).sum()),
    }


def score(matrix: np.ndarray, model: dict[str, Any]) -> tuple[np.ndarray, np.ndarray]:
    """`(T2, Q)` — distance inside the subspace, and the residual off it."""
    centered = matrix - model["mean"]
    scores = centered @ model["basis"].T
    t2 = np.sqrt(((scores / model["scale"]) ** 2).sum(axis=1))
    residual = centered - scores @ model["basis"]
    return t2, np.sqrt((residual**2).sum(axis=1))


def validate_by_player(
    matrix: np.ndarray, subjects: list[str], components: int
) -> tuple[float, float]:
    """Leave-one-player-out exceedance of the refitted model's own p90, for T² and Q.

    The question a calibration number answers: does a golfer the fit never saw land where the fit
    says they should? ~10% should exceed p90. Far more and the basis has memorised these 122
    players; far less and it is too loose to say anything.
    """
    people = np.array(subjects)
    t2_exceeded = q_exceeded = total = 0
    for player in sorted(set(subjects)):
        held = people == player
        if held.all() or held.sum() == 0:
            continue
        model = fit_pca(matrix[~held], components)
        train_t2, train_q = score(matrix[~held], model)
        held_t2, held_q = score(matrix[held], model)
        t2_exceeded += int((held_t2 > np.quantile(train_t2, 0.90)).sum())
        q_exceeded += int((held_q > np.quantile(train_q, 0.90)).sum())
        total += int(held.sum())
    return t2_exceeded / total, q_exceeded / total


def build_payload(
    matrix: np.ndarray,
    subjects: list[str],
    model: dict[str, Any],
    steps: int,
    axes: tuple[str, ...],
    validation: tuple[float, float],
    events: tuple[str, ...],
    landmarks: list[str],
    view: str,
) -> dict[str, Any]:
    t2, q = score(matrix, model)
    quantiles = (0.10, 0.25, 0.50, 0.75, 0.90)
    return {
        "schema_version": SCHEMA_VERSION,
        "dataset": {
            "name": "GolfDB",
            "citation": "McNally et al., GolfDB: A Video Database for Golf Swing Sequencing, "
            "CVPR Workshops 2019",
            "url": "https://github.com/wmcnally/golfdb",
            "license_note": (
                "Aggregate statistics only. A mean trajectory and a principal-component basis "
                "fitted over 122 golfers carry no player name, clip id or frame index, and are "
                "not a substantial reproduction of the dataset — the same test the percentiles "
                "pass. See ADR-012 and ADR-022."
            ),
            "pose_estimator": _ESTIMATOR_NAME,
            "metric_definitions_version": 3,
            "min_samples": derive_reference.MIN_SAMPLES,
            "derived_on": date.today().isoformat(),
            "pipeline_commit": derive_reference._pipeline_commit(),
        },
        "model": {
            "kind": "pca_trajectory",
            "landmarks": list(landmarks),
            "view": view,
            "axes": list(axes),
            "events": list(events),
            "steps": steps,
            "n": int(matrix.shape[0]),
            "n_players": len(set(subjects)),
            "dimensions": int(matrix.shape[1]),
            "components": int(model["basis"].shape[0]),
            "explained_variance": round(model["explained"], 4),
            "mean": [round(float(v), 5) for v in model["mean"]],
            "scale": [round(float(v), 5) for v in model["scale"]],
            "basis": [[round(float(v), 5) for v in row] for row in model["basis"]],
            "t2_quantiles": {
                f"p{int(q_ * 100)}": round(float(np.quantile(t2, q_)), 5) for q_ in quantiles
            },
            "q_quantiles": {
                f"p{int(q_ * 100)}": round(float(np.quantile(q, q_)), 5) for q_ in quantiles
            },
            # Shipped *inside* the artifact rather than left in a doc, because these two numbers
            # are what a consumer needs to know how far to trust each statistic. 10% is calibrated.
            # T2 sits close to it; Q does not, and over-flags golfers the basis never saw — a new
            # person's idiosyncrasies land in the residual by construction, which is a fact about
            # what Q measures rather than a tuning failure.
            "leave_one_player_out_exceedance": {
                "target": 0.10,
                "t2": round(validation[0], 4),
                "q": round(validation[1], 4),
            },
        },
    }


def _arg(argv: list[str], flag: str, default: int) -> int:
    return int(argv[argv.index(flag) + 1]) if flag in argv else default


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print(
            "usage: python scripts/golfdb/derive_trajectory_model.py "
            "[--steps N] [--components N] [--axes xyz|xy] "
            "[--anchors detected|annotated] [--view V] [--landmarks V] [--write]",
            file=sys.stderr,
        )
        return 2

    steps = _arg(argv, "--steps", 40)
    # 6 components, x/y only. Both settled by measurement rather than taste — see the sweep in
    # docs/M4_POSE_BAKEOFF.md §Phase E. `z` *lowered* variance explained while enlarging the
    # artifact, and T2's leave-one-player-out calibration peaks at 6 (9.9% against a 10% target)
    # with Q at its best there too.
    #
    # It was 10 before the pixel-aspect correction landed. Removing that per-clip distortion took
    # a source of variance out of the data, so **fewer** components now describe more of it — a
    # good reason never to carry a component count across a change in how the features are built.
    components = _arg(argv, "--components", 6)
    axes: tuple[str, ...] = ("x", "y", "z") if "xyz" in argv else ("x", "y")
    anchors = "annotated" if "annotated" in argv else "detected"
    events = ANCHOR_SETS[anchors]
    view = argv[argv.index("--view") + 1] if "--view" in argv else common.VIEW_FACE_ON
    if view not in LANDMARK_SETS:
        print(f"unknown view {view!r}; expected one of {sorted(LANDMARK_SETS)}", file=sys.stderr)
        return 2
    # `--landmarks` exists to run the comparison the screen could not settle: whether dropping
    # the lead arm actually helps, or only looks principled. Defaults to this view's own set.
    chosen = argv[argv.index("--landmarks") + 1] if "--landmarks" in argv else view
    if chosen not in LANDMARK_SETS:
        print(f"unknown landmark set {chosen!r}", file=sys.stderr)
        return 2
    landmarks = LANDMARK_SETS[chosen]

    swings = common.load_swings()
    derive_reference._drop_implausible(swings)
    lefty_list, _ = derive_reference._normalize_handedness(swings)
    lefties = set(lefty_list)
    in_view = [s for s in swings if s.view == view]

    matrix, subjects, skipped = collect(in_view, steps, axes, lefties, events, landmarks)
    if len(matrix) < 100:
        print(f"only {len(matrix)} usable clips — expected ~450", file=sys.stderr)
        return 1

    print(
        f"axes={''.join(axes)}  steps={steps}  components={components}\n"
        f"{len(matrix)} clips from {len(set(subjects))} golfers "
        f"({skipped} skipped, {len(lefties)} left-handed golfers mirrored)"
    )
    print(f"raw dimensions: {matrix.shape[1]}  ->  {components} components")

    model = fit_pca(matrix, components)
    print(f"variance explained: {model['explained']:.1%}")

    t2_rate, q_rate = validate_by_player(matrix, subjects, components)
    print("\nleave-one-player-out exceedance of the fit's own p90 (10% is calibrated):")
    print(f"  T2 (inside the subspace): {t2_rate:.1%}")
    print(f"  Q  (residual off it):     {q_rate:.1%}")

    payload = build_payload(
        matrix, subjects, model, steps, axes, (t2_rate, q_rate), events, landmarks, view
    )
    size = len(json.dumps(payload)) / 1024
    print(f"\nartifact would be {size:,.0f} KB")

    if "--write" not in argv:
        print(f"\nDRY RUN — pass --write to update {_OUTPUT_NAMES[view]}")
        return 0

    output = common.BENCHMARKS_DIR / _OUTPUT_NAMES[view]
    output.write_text(json.dumps(payload) + "\n", encoding="utf-8")
    print(f"\nwrote {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
