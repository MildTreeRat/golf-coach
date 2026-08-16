"""Fit the tour joint-distribution model and write the committed artifact. [M8-JOINT]

Usage:
    python scripts/golfdb/derive_joint_model.py [--write]

Needs the `research` extra (numpy, scikit-learn). Reads `data/reference/golfdb/swings.jsonl` and
writes `src/golf_coach/analysis/benchmarks/joint_model_v1.json`.

### What it fits, and why that is a model rather than a band

Six independent bands can say each of a swing's six numbers is normal. They cannot say the
*combination* is one no tour player produces, because six range checks have no way to express
"this is fine only if that is also true". `tune_joint_structure.py` is the gate that established
there is something to express: `head_sway_norm` and `hip_shift_at_top_norm` correlate at +0.44,
`head_sway_norm` and `head_hip_gain_norm` at -0.39.

So this fits the **shape** of the tour population — a robust center, a robust scale and the
correlation structure between the six — and stores its inverse. A swing then gets a Mahalanobis
distance: how far it sits from the tour population *accounting for the fact that these quantities
travel together*. The distance is decomposable, so it can also name which metric and which pairing
carried the departure.

**It says unusual, not bad.** Every clip behind it is a tour professional's swing, so the model
knows what a working swing looks like and nothing whatsoever about what a broken one looks like.
That is the same limit ADR-012 records for the bands, and it is why the output is a percentile
within the tour population rather than a score.

### Why it may be committed at all

A mean vector, a scale vector and a 6x6 matrix are anonymous aggregates — no player name, no clip
id, no frame index — which is exactly the test the percentiles in `golfdb_v1.json` already pass
under ADR-012 §2. The corpus stays gitignored; its shape ships.

Like `derive_reference.py`, this **prints before it writes** and never touches `ranges.json`.
Pass `--write` to update the artifact; without it the run is a dry run.
"""

from __future__ import annotations

import json
import sys
from datetime import date
from typing import Any

import common
import derive_reference
import numpy as np
import tune_joint_structure

SCHEMA_VERSION = 1
OUTPUT_PATH = common.BENCHMARKS_DIR / "joint_model_v1.json"

# Order is load-bearing: it indexes the center, scale and precision matrix that ship, and a
# reordering here without a re-derive would silently pair each metric with another's statistics.
# Taken from the gate rather than restated, so the two cannot drift apart.
METRICS = tune_joint_structure.METRICS

# Standardised values past this are pose failures rather than swings, and a single one moves a
# correlation more than a hundred real clips do. `_drop_implausible` already removes `_norm`
# readings past 3.0 in raw units; this is the second net, in robust-z units, and it catches
# `tempo_ratio`, which has no `_norm` suffix and therefore no first net.
_CLIP_Z = 4.0

# Where a distance sits in the tour population. Same five-point shape `Distribution` stores, for
# the same reason: the tails are not stored, so a swing past them is reported as "at least this
# unusual" rather than extrapolated into a precise-looking number.
_QUANTILES = (0.10, 0.25, 0.50, 0.75, 0.90)


def robust_standardize(matrix: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Median/MAD standardisation, returning `(z, center, scale)`.

    Median and MAD rather than mean and sd because the corpus is pose over broadcast footage and
    its tails are instrument failures, not athletes. 1.4826 rescales MAD to estimate sigma for
    normally distributed data, so a robust z is comparable to an ordinary one.
    """
    center = np.median(matrix, axis=0)
    mad = np.median(np.abs(matrix - center), axis=0)
    scale = np.where(mad > 0, mad * 1.4826, 1.0)
    return (matrix - center) / scale, center, scale


def fit(matrix: np.ndarray) -> dict[str, Any]:
    """Center, scale, and the inverse correlation of the clipped standardised values."""
    z, center, scale = robust_standardize(matrix)
    clipped = np.clip(z, -_CLIP_Z, _CLIP_Z)
    correlation = np.corrcoef(clipped, rowvar=False)
    precision = np.linalg.inv(correlation)
    return {
        "center": center,
        "scale": scale,
        "correlation": correlation,
        "precision": precision,
        "z": clipped,
    }


def distances(z: np.ndarray, precision: np.ndarray) -> np.ndarray:
    """Mahalanobis distance of each row, in the standardised frame."""
    return np.sqrt(np.einsum("ij,jk,ik->i", z, precision, z))


def _compare_with_mcd(matrix: np.ndarray, correlation: np.ndarray) -> tuple[float, bool]:
    """How much would a formal robust covariance estimator disagree? Returns `(gap, converged)`.

    Median/MAD plus a clip is the estimator that ships, because it is four lines of arithmetic
    anyone can check against the committed numbers. `MinCovDet` is the textbook answer and is run
    as an audit of that choice.

    **It does not converge on this corpus**, and that is worth more than the number it returns.
    MCD looks for the tightest elliptical core in the data; its determinant warning says it could
    not find one, which means this population is not well described by an ellipse out in the tails.
    The consequence is specific and bounded: a Mahalanobis distance here may **not** be read
    against a chi-square distribution, the way textbook treatments do. It is still a perfectly good
    ordering, which is why the artifact calibrates against stored empirical quantiles of the tour
    population's own distances instead of assuming a parametric tail.
    """
    import warnings

    from sklearn.covariance import MinCovDet

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        robust = MinCovDet(random_state=0).fit(matrix)
        converged = not any("Determinant has increased" in str(w.message) for w in caught)

    covariance = robust.covariance_
    deviation = np.sqrt(np.diag(covariance))
    mcd_correlation = covariance / np.outer(deviation, deviation)
    return float(np.abs(mcd_correlation - correlation).max()), converged


def _validate_by_player(rows: list[Any], matrix: np.ndarray) -> tuple[float, float]:
    """Leave-one-player-out: does the shape generalise, or has it memorised these 122 golfers?

    A model fitted on everyone and scored on everyone will always look calibrated. The honest
    question is whether a golfer the fit never saw lands where the fit says they should, so each
    player is held out, the model refitted on the rest, and the held-out swings scored against
    *that* model's own p90. Roughly 10% should exceed it. Far more means the shape does not
    transfer; far fewer means it is too loose to say anything.

    Returns `(exceedance, in_sample_exceedance)`.
    """
    players = np.array([r.subject or "" for r in rows])
    exceeded = total = 0
    for player in sorted(set(players)):
        held = players == player
        if held.all():
            continue
        model = fit(matrix[~held])
        train_distance = distances(model["z"], model["precision"])
        threshold = float(np.quantile(train_distance, 0.90))

        z = (matrix[held] - model["center"]) / model["scale"]
        held_distance = distances(np.clip(z, -_CLIP_Z, _CLIP_Z), model["precision"])
        exceeded += int((held_distance > threshold).sum())
        total += int(held.sum())

    full = fit(matrix)
    in_sample = distances(full["z"], full["precision"])
    in_sample_rate = float((in_sample > np.quantile(in_sample, 0.90)).mean())
    return exceeded / total, in_sample_rate


def build_payload(rows: list[Any], model: dict[str, Any]) -> dict[str, Any]:
    distance = distances(model["z"], model["precision"])
    players = {r.subject or "" for r in rows}

    return {
        "schema_version": SCHEMA_VERSION,
        "dataset": {
            "name": "GolfDB",
            "citation": "McNally et al., GolfDB: A Video Database for Golf Swing Sequencing, "
            "CVPR Workshops 2019",
            "url": "https://github.com/wmcnally/golfdb",
            "license_note": (
                "Aggregate statistics only. GolfDB's code is CC BY-NC 4.0, the dataset states no "
                "license, and the clips are third-party broadcast footage. A center, a scale and "
                "a correlation matrix carry no player name, clip id or frame index and are not a "
                "substantial reproduction of the dataset. See ADR-012 and ADR-022."
            ),
            "pose_estimator": "mediapipe:lite",
            "metric_definitions_version": 3,
            "min_samples": derive_reference.MIN_SAMPLES,
            "derived_on": date.today().isoformat(),
            "pipeline_commit": derive_reference._pipeline_commit(),
        },
        "model": {
            "kind": "mahalanobis",
            "metrics": list(METRICS),
            "n": len(rows),
            "n_players": len(players),
            "clip_z": _CLIP_Z,
            "center": [round(float(v), 6) for v in model["center"]],
            "scale": [round(float(v), 6) for v in model["scale"]],
            "precision": [[round(float(v), 6) for v in row] for row in model["precision"]],
            "distance_quantiles": {
                f"p{int(q * 100)}": round(float(np.quantile(distance, q)), 6) for q in _QUANTILES
            },
        },
    }


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print("usage: python scripts/golfdb/derive_joint_model.py [--write]", file=sys.stderr)
        return 2

    rows = tune_joint_structure.face_on_rows(common.load_swings())
    matrix = tune_joint_structure.matrix_of(rows)
    players = {r.subject or "" for r in rows}
    print(f"fitting over {len(rows)} face-on clips from {len(players)} golfers")

    model = fit(matrix)
    distance = distances(model["z"], model["precision"])

    print("\n  metric                       center      scale")
    for i, metric in enumerate(METRICS):
        print(f"    {metric:<24s} {model['center'][i]:9.4f}  {model['scale'][i]:9.4f}")

    print("\n  distance quantiles over the tour population itself:")
    for q in _QUANTILES:
        print(f"    p{int(q * 100):<3d} {float(np.quantile(distance, q)):.3f}")

    disagreement, converged = _compare_with_mcd(matrix, model["correlation"])
    if converged:
        print(f"\n  max |correlation| disagreement vs MinCovDet: {disagreement:.3f}")
    else:
        print("\n  MinCovDet did NOT converge on this corpus, so its disagreement is not a")
        print("  usable audit. The population has no tight elliptical core, which is why the")
        print("  artifact calibrates on empirical quantiles rather than a chi-square tail.")

    held_out, in_sample = _validate_by_player(rows, matrix)
    print(f"\n  leave-one-player-out exceedance of the fit's own p90: {held_out:.1%}")
    print(f"  same figure in-sample: {in_sample:.1%}   (10% is the calibrated target)")
    if abs(held_out - 0.10) > 0.05:
        print("  WARNING: the shape does not transfer to unseen golfers. Do not ship this.")

    payload = build_payload(rows, model)
    if "--write" not in argv:
        print(f"\n  DRY RUN — pass --write to update {OUTPUT_PATH.name}")
        return 0

    OUTPUT_PATH.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"\n  wrote {OUTPUT_PATH}")
    print("  This does not touch ranges.json. No band moves without a human (ADR-010).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
