"""Is there joint structure the six independent bands are missing? The gate for M8-JOINT.

Usage:
    python scripts/golfdb/tune_joint_structure.py

Needs the `research` extra (numpy). Reads `data/reference/golfdb/swings.jsonl`.

### The question this decides

Today a swing is judged by six scalars against six independent bands
(`contracts/checkpoints.py::CHECKPOINT_REGISTRY` → `checkpoints/mechanics.py::_score_within_range`).
That shape can say each number is normal. It cannot say the *combination* is one no tour player
produces — a wide hip slide is fine if the head stays back and a fault if it does not, and six
separate range checks have no way to express the "if".

Whether that missing "if" is worth building depends entirely on whether the metrics are actually
correlated in the tour population. If they are near-independent, a joint model is arithmetic
dressed up as insight: the Mahalanobis distance collapses to a sum of squared z-scores and says
nothing the bands do not already say. So this is a gate, and it is allowed to say no — the same
job `tune_arm_parallel.py` did when it killed two candidate checkpoints before they were written.

### Two things measured alongside, because the data is already open

- **Player-clustered band edges.** 458 clips come from 122 golfers, so they are not 458
  independent samples: several clips are often cut from one broadcast of one swing motion. A
  bootstrap that resamples *players* rather than clips gives the honest interval on p10/p90. This
  needs no new data and puts an error bar on every band in `ranges.json`.
- **Per-club stratum sizes** on the face-on subset, against `derive_reference.py`'s existing
  `MIN_SAMPLES = 30`. ROADMAP's "per-club percentiles" box is open and this says which clubs could
  ever fill it.

Nothing here writes to `ranges.json` or to `golfdb_v1.json`. It prints, a human reads (ADR-010).
"""

from __future__ import annotations

import random
import sys
from typing import Any

import common
import derive_reference
import numpy as np

from golf_coach.analysis.stats import percentile

# The six metrics behind the six checkpoints, in registry order. `head_hip_offset_impact_norm` is
# measured on the same clips and deliberately left out: its sign is camera-relative and this
# corpus mixes handedness, which is exactly why ADR-010's 2026-08-12 addendum refused to promote
# it to a checkpoint. Correlating a quantity whose sign means two different things across the
# population would produce a number, and the number would be meaningless.
METRICS: tuple[str, ...] = (
    "tempo_ratio",
    "head_sway_norm",
    "finish_balance_norm",
    "hip_sway_norm",
    "hip_shift_at_top_norm",
    "head_hip_gain_norm",
)

# Below this the joint model has nothing to add over six independent bands. Chosen as the point
# where a correlation starts to move a joint distance materially: at r = 0.2 the ellipse's axes
# differ from the axis-aligned box by a few percent, which is not worth a new artifact, a new
# loader and an ANALYSIS_VERSION bump.
_MIN_INTERESTING_R = 0.2

_BOOTSTRAP_ROUNDS = 2000
_SEED = 20260816


def face_on_rows(swings: list[Any]) -> list[Any]:
    """Face-on clips that carry every one of the six metrics, screened as the bands were.

    **The two preprocessing steps are borrowed rather than reimplemented**, in `derive_reference`'s
    order, because this script's whole claim is to describe the population the committed bands were
    cut from. Skipping them was the first version of this script and it disagreed with
    `ranges.json`: `head_hip_gain_norm`'s p90 came out at -0.111 against the committed -0.137,
    because four left-handed subjects — Mickelson and Watson among them — still had their signed
    metrics mirrored. An unfolded corpus also *attenuates* every correlation involving a signed
    metric, so the structure this gate exists to detect would have been understated.

    Listwise complete rather than per-metric: a correlation matrix assembled from a different
    subset of swings per cell is not a covariance matrix and cannot be meaningfully inverted later.
    """
    derive_reference._drop_implausible(swings)
    derive_reference._normalize_handedness(swings)
    return [
        s
        for s in swings
        if s.view == common.VIEW_FACE_ON and all(m in s.metrics for m in METRICS)
    ]


def matrix_of(rows: list[Any]) -> np.ndarray:
    return np.array([[float(r.metrics[m]) for m in METRICS] for r in rows])


def report_correlations(matrix: np.ndarray) -> float:
    correlation = np.corrcoef(matrix, rowvar=False)
    print(f"\n{'=' * 78}\nCORRELATION AMONG THE SIX BANDED METRICS\n{'=' * 78}\n")

    width = max(len(m) for m in METRICS)
    print(" " * (width + 2) + "".join(f"{m[:11]:>12s}" for m in METRICS))
    for i, name in enumerate(METRICS):
        cells = "".join(f"{correlation[i, j]:>12.2f}" for j in range(len(METRICS)))
        print(f"  {name:<{width}s}{cells}")

    off_diagonal = [
        (abs(correlation[i, j]), correlation[i, j], METRICS[i], METRICS[j])
        for i in range(len(METRICS))
        for j in range(i + 1, len(METRICS))
    ]
    off_diagonal.sort(reverse=True)
    print("\n  strongest pairings:")
    for magnitude, signed, left, right in off_diagonal[:6]:
        mark = "  <- above gate" if magnitude >= _MIN_INTERESTING_R else ""
        print(f"    {left:24s} x {right:24s} r = {signed:+.3f}{mark}")

    # Eigenvalues of the *correlation* matrix, not the covariance: `tempo_ratio` is a ratio near
    # 3.4 while the others are shoulder-width fractions near 0.2, so an unstandardised covariance
    # would report "the first component explains 87%" when all it had found was that tempo is
    # measured in bigger numbers. A joint model standardises before it inverts, and so does this.
    eigenvalues = np.linalg.eigvalsh(correlation)
    condition = float(eigenvalues.max() / eigenvalues.min())
    explained = eigenvalues.max() / eigenvalues.sum()
    print(f"\n  correlation-matrix condition number: {condition:,.1f}")
    print(f"  variance explained by the first standardised component: {explained:.1%}")
    return max(magnitude for magnitude, *_ in off_diagonal)


def report_clustered_bands(rows: list[Any]) -> None:
    """Band edges with an interval that respects who swung.

    Two bootstraps, printed side by side, and the gap between them is the whole point: the naive
    one resamples clips and believes it has 458 independent observations, the clustered one
    resamples golfers and carries the fact that one golfer contributes several clips. Where the
    clustered interval is materially wider, the band is less certain than `ranges.json`'s single
    number looks.
    """
    print(f"\n{'=' * 78}\nBAND EDGES, CLIP BOOTSTRAP vs PLAYER-CLUSTERED BOOTSTRAP\n{'=' * 78}")

    by_player: dict[str, list[Any]] = {}
    for row in rows:
        by_player.setdefault(row.subject or "", []).append(row)
    players = sorted(by_player)
    print(f"\n  {len(rows)} clips from {len(players)} golfers")
    per_golfer = sorted(len(v) for v in by_player.values())
    print(
        f"  clips per golfer: median {per_golfer[len(players) // 2]}, max {per_golfer[-1]}"
    )
    # Five of the six point estimates below reproduce `ranges.json` to its rounding. `tempo_ratio`
    # will not, and that is correct rather than a discrepancy: its committed band is cut from all
    # 1,399 clips because a ratio of frame counts needs no pose and therefore no face-on view,
    # while everything here is the 458-clip face-on subset the spatial metrics live on. The
    # widening factor still reads, since both bootstraps run over the same rows.
    print("  (tempo_ratio's committed band is cut from all 1,399 clips, not this subset)")

    # `analysis.stats.percentile` takes q in [0, 1] and is used rather than a numpy quantile on
    # purpose: it is the same function `derive_reference.py` cuts the committed bands with, and a
    # confidence interval computed under a different percentile convention than the point estimate
    # it brackets would be quietly wrong at exactly the edges that matter.
    rng = random.Random(_SEED)
    for metric in METRICS:
        values = [float(r.metrics[metric]) for r in rows]
        point_low, point_high = percentile(values, 0.10), percentile(values, 0.90)

        naive_low, naive_high = [], []
        clustered_low, clustered_high = [], []
        for _ in range(_BOOTSTRAP_ROUNDS):
            sample = [values[rng.randrange(len(values))] for _ in range(len(values))]
            naive_low.append(percentile(sample, 0.10))
            naive_high.append(percentile(sample, 0.90))

            drawn: list[float] = []
            for _ in range(len(players)):
                picked = by_player[players[rng.randrange(len(players))]]
                drawn.extend(float(r.metrics[metric]) for r in picked)
            clustered_low.append(percentile(drawn, 0.10))
            clustered_high.append(percentile(drawn, 0.90))

        def interval(samples: list[float]) -> tuple[float, float]:
            return percentile(samples, 0.025), percentile(samples, 0.975)

        n_lo, n_hi = interval(naive_low)
        c_lo, c_hi = interval(clustered_low)
        widening_low = (c_hi - c_lo) / (n_hi - n_lo) if n_hi > n_lo else float("nan")
        n_lo2, n_hi2 = interval(naive_high)
        c_lo2, c_hi2 = interval(clustered_high)
        widening_high = (c_hi2 - c_lo2) / (n_hi2 - n_lo2) if n_hi2 > n_lo2 else float("nan")

        print(f"\n  {metric}")
        print(f"    p10 {point_low:8.4f}   clip 95% [{n_lo:8.4f},{n_hi:8.4f}]   "
              f"player 95% [{c_lo:8.4f},{c_hi:8.4f}]   x{widening_low:.2f} wider")
        print(f"    p90 {point_high:8.4f}   clip 95% [{n_lo2:8.4f},{n_hi2:8.4f}]   "
              f"player 95% [{c_lo2:8.4f},{c_hi2:8.4f}]   x{widening_high:.2f} wider")


def report_strata(rows: list[Any]) -> None:
    """Which per-club face-on strata could ever clear `derive_reference.py`'s MIN_SAMPLES."""
    print(f"\n{'=' * 78}\nPER-CLUB FACE-ON STRATA\n{'=' * 78}\n")
    counts: dict[str, int] = {}
    players: dict[str, set[str]] = {}
    for row in rows:
        counts[row.club] = counts.get(row.club, 0) + 1
        players.setdefault(row.club, set()).add(row.subject or "")
    for club, count in sorted(counts.items(), key=lambda kv: -kv[1]):
        verdict = "clears 30" if count >= 30 else "BELOW the min-sample gate"
        print(f"  {club:10s} {count:4d} clips, {len(players[club]):3d} golfers   {verdict}")


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print("usage: python scripts/golfdb/tune_joint_structure.py", file=sys.stderr)
        return 2

    rows = face_on_rows(common.load_swings())
    if not rows:
        print("no face-on rows carry all six metrics — run derive_pose_metrics.py", file=sys.stderr)
        return 1

    matrix = matrix_of(rows)
    print(f"face-on clips with all six metrics: {len(rows)}")

    strongest = report_correlations(matrix)
    report_strata(rows)
    report_clustered_bands(rows)

    print(f"\n{'=' * 78}\nGATE\n{'=' * 78}")
    if strongest >= _MIN_INTERESTING_R:
        print(f"  PASS — strongest |r| = {strongest:.3f} >= {_MIN_INTERESTING_R}")
        print("  The metrics are not independent, so a joint model can say something six")
        print("  separate range checks cannot. Proceed to derive_joint_model.py.")
    else:
        print(f"  FAIL — strongest |r| = {strongest:.3f} < {_MIN_INTERESTING_R}")
        print("  The six metrics are near-independent in this population. A joint model would")
        print("  reduce to the bands already shipped. Record it and stop.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
