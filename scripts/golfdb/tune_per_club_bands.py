"""Are per-club benchmark bands worth cutting? The gate for M8's last open box.

Usage:
    python scripts/golfdb/tune_per_club_bands.py

Needs no extra: stdlib only, reading `data/reference/golfdb/swings.jsonl`.

### The question this decides

`resolve_range` has taken a `ClubCategory` since ADR-010 §3 and `analyze_bundle.py --club` reaches
it end to end, but `ranges.json` has no per-club row, so every swing lands on `(all, all)`.
`tune_joint_structure.py` costed the strata that *could* be cut — driver 341 clips, iron 69,
fairway 32 clearing `derive_reference.MIN_SAMPLES` — and ROADMAP has carried "per-club bands are
now costed" as an open box ever since.

**Costed is not gated.** That a stratum has enough clips to cut a band from says nothing about
whether the band that comes out differs from the one already shipped. This script asks the second
question, and it is allowed to say no — the job `tune_arm_parallel.py` did when it killed two
candidate checkpoints before they were written.

### The test, in two parts, because the first part is not enough

1. **Does the per-club edge move by more than the instrument can resolve?** Each metric's
   noise+boundary error is read from `contracts.dispersion.METRIC_TARGETS[...].tolerance` rather
   than restated here, and the bar is `tune_spatial_metric.py`'s 2.0x. An edge that moves less than
   the measurement error changes verdicts at random.

2. **Does that shift survive a player-clustered bootstrap?** This is the part the naive screen gets
   wrong, and ADR-010's 2026-08-18 addendum is why it is here: clip counts overstate independence,
   so a stratum of 32 clips from 14 golfers is not 32 observations. Part 1 runs on point estimates;
   part 2 asks whether the all-club edge sits inside the per-club edge's own 95% interval. Where it
   does, the two bands are one band wearing different sample noise.

`MIN_SAMPLES = 30` is a **clip** gate and does not catch this: fairway clears it on 32 clips while
carrying only 14 golfers.

### A vocabulary mismatch that outlives whatever this gate decides

`ClubCategory` is `driver, wood, hybrid, long_iron, mid_iron, short_iron, wedge, putter, all`. The
corpus labels clubs `driver, iron, fairway, wedge, hybrid` — `contracts/reference.py` says so in as
many words ("Club as the corpus names it, not our ClubCategory; mapping is the consumer's") and
nothing has ever had to write that mapping. It is not writable as things stand: the corpus cannot
tell a long iron from a short one, so three `ClubCategory` members would have to be seeded from one
undifferentiated stratum, and `resolve_range` matches `club_category` exactly. Three rows claiming
to be three strata while being one is the shape of thing ADR-010's provenance rule exists to stop.

Nothing here writes to `ranges.json`. It prints, a human reads (ADR-010).
"""

from __future__ import annotations

import random
import statistics
import sys
from collections import defaultdict
from collections.abc import Callable
from typing import Any

import common
import derive_reference

from golf_coach.analysis.stats import percentile
from golf_coach.contracts.dispersion import METRIC_TARGETS

#: The six metrics behind the six shipped checkpoints, in registry order.
METRICS: tuple[str, ...] = (
    "tempo_ratio",
    "head_sway_norm",
    "finish_balance_norm",
    "hip_sway_norm",
    "hip_shift_at_top_norm",
    "head_hip_gain_norm",
)

#: `tempo_ratio` is a ratio of frame counts, so it needs no pose and its committed band is cut from
#: all 1,399 labelled swings. Every other metric here exists only on the face-on subset.
ALL_VIEW_METRICS = frozenset({"tempo_ratio"})

#: Clubs as the *corpus* names them, not as `ClubCategory` does. See the module docstring.
CORPUS_CLUBS: tuple[str, ...] = ("driver", "iron", "fairway", "hybrid", "wedge")

#: `tune_spatial_metric.py`'s screening bar: a quantity clears its own error by this much before it
#: is worth acting on.
BAR = 2.0

_BOOTSTRAP_ROUNDS = 4000
_SEED = 20260818


def _rows_for(metric: str, swings: list[Any]) -> list[Any]:
    """The population a metric's band is actually cut from (see `ALL_VIEW_METRICS`)."""
    usable = [s for s in swings if metric in s.metrics]
    if metric in ALL_VIEW_METRICS:
        return usable
    return [s for s in usable if s.view == common.VIEW_FACE_ON]


def _by_player(rows: list[Any], metric: str) -> dict[str, list[float]]:
    grouped: dict[str, list[float]] = defaultdict(list)
    for row in rows:
        grouped[row.subject or "?"].append(float(row.metrics[metric]))
    return grouped


def _clustered_interval(
    grouped: dict[str, list[float]], quantile: float, rng: random.Random
) -> tuple[float, float]:
    """95% interval on a band edge, resampling **golfers** rather than clips.

    `percentile` rather than a numpy quantile for the reason `tune_joint_structure.py` gives: it is
    the function `derive_reference.py` cuts the committed bands with, and an interval computed under
    a different convention than the point estimate it brackets is wrong exactly at the edges that
    decide this.
    """
    players = sorted(grouped)
    edges = []
    for _ in range(_BOOTSTRAP_ROUNDS):
        drawn: list[float] = []
        for _ in range(len(players)):
            drawn.extend(grouped[players[rng.randrange(len(players))]])
        if drawn:
            edges.append(percentile(drawn, quantile))
    return percentile(edges, 0.025), percentile(edges, 0.975)


def report_gate(swings: list[Any], rng: random.Random) -> list[tuple[str, str, float]]:
    """Every metric x club stratum, screened then bootstrapped. Returns what survived both."""
    print(f"\n{'=' * 78}\nPER-CLUB BANDS: DOES THE EDGE MOVE MORE THAN THE INSTRUMENT?\n{'=' * 78}")
    print(f"\n  bar is {BAR}x the metric's own noise+boundary error (contracts.dispersion)")
    print("  the shift is measured on p90, the edge every one-sided band asserts\n")

    survivors: list[tuple[str, str, float]] = []
    for metric in METRICS:
        floor = METRIC_TARGETS[metric].tolerance
        rows = _rows_for(metric, swings)
        overall = percentile([float(r.metrics[metric]) for r in rows], 0.90)
        print(f"  {metric}  (floor {floor:g}, all-club p90 {overall:+.4f}, n={len(rows)})")

        for club in CORPUS_CLUBS:
            grouped = _by_player([r for r in rows if r.club == club], metric)
            values = [v for vs in grouped.values() for v in vs]
            if len(values) < derive_reference.MIN_SAMPLES:
                print(
                    f"    {club:<9} {len(values):>4} clips, {len(grouped):>3} golfers"
                    f"   below MIN_SAMPLES, not cut"
                )
                continue

            edge = percentile(values, 0.90)
            ratio = abs(edge - overall) / floor
            line = (
                f"    {club:<9} {len(values):>4} clips, {len(grouped):>3} golfers"
                f"   p90 {edge:+.4f}   shift {ratio:4.2f}x"
            )
            if ratio < BAR:
                print(f"{line}   screened out")
                continue

            low, high = _clustered_interval(grouped, 0.90, rng)
            if low <= overall <= high:
                print(f"{line}   player 95% [{low:+.4f},{high:+.4f}]  NOT distinguishable")
                continue
            worst = min(abs(low - overall), abs(high - overall)) / floor
            verdict = "SURVIVES" if worst >= BAR else f"fails on the bound ({worst:.2f}x)"
            print(f"{line}   player 95% [{low:+.4f},{high:+.4f}]  {verdict}")
            if worst >= BAR:
                survivors.append((metric, club, worst))
        print()
    return survivors


def report_pooled(swings: list[Any], rng: random.Random) -> None:
    """The same test with every non-driver club pooled.

    Whatever per-club effect exists points one way — a shorter club is swung with less movement —
    and each single stratum is too thin to see it. Pooling tests that physical hypothesis instead of
    the club labels, on roughly triple the golfers. It is a *measurement*, not a proposal: there is
    no `ClubCategory` for "not a driver", so nothing could be keyed on this without a contract
    change and the mapping the module docstring says is unwritable.
    """
    print(f"\n{'=' * 78}\nTHE SAME TEST, NON-DRIVER CLUBS POOLED\n{'=' * 78}\n")
    short = set(CORPUS_CLUBS) - {"driver"}
    for metric in METRICS:
        floor = METRIC_TARGETS[metric].tolerance
        rows = _rows_for(metric, swings)
        overall = percentile([float(r.metrics[metric]) for r in rows], 0.90)
        grouped = _by_player([r for r in rows if r.club in short], metric)
        values = [v for vs in grouped.values() for v in vs]
        edge = percentile(values, 0.90)
        ratio = abs(edge - overall) / floor
        low, high = _clustered_interval(grouped, 0.90, rng)
        if low <= overall <= high:
            verdict = "NOT distinguishable"
        else:
            worst = min(abs(low - overall), abs(high - overall)) / floor
            verdict = (
                f"SURVIVES ({worst:.2f}x)" if worst >= BAR else f"fails the bound ({worst:.2f}x)"
            )
        print(
            f"  {metric:<22} {len(values):>4} clips, {len(grouped):>3} golfers"
            f"   p90 {edge:+.4f} vs {overall:+.4f}   shift {ratio:4.2f}x   {verdict}"
        )


def report_tempo_axis(swings: list[Any]) -> None:
    """Is tempo a club property at all? Variance between golfers against variance between clubs.

    Asked because tempo screens out at every club while being the metric a coach would most expect
    to move with club length. The decomposition answers *why*, on the axis the question should have
    been put: if a golfer's tempo is their own signature, the club is the wrong key and a per-club
    tempo band would be a worse-resolved version of career mode's personal baseline.
    """
    print(f"\n{'=' * 78}\nTEMPO: IS THE CLUB EVEN THE RIGHT AXIS?\n{'=' * 78}\n")
    rows = _rows_for("tempo_ratio", swings)
    values = [float(r.metrics["tempo_ratio"]) for r in rows]
    print(
        f"  {len(rows)} swings, {len({r.subject for r in rows})} golfers"
        f"   overall sd {statistics.stdev(values):.4f}"
    )

    def spread(key: Callable[[Any], str], label: str, floor_n: int) -> float:
        grouped: dict[str, list[float]] = defaultdict(list)
        for row in rows:
            grouped[key(row)].append(float(row.metrics["tempo_ratio"]))
        kept = {k: v for k, v in grouped.items() if len(v) >= floor_n}
        between = statistics.stdev([statistics.mean(v) for v in kept.values()])
        within = statistics.median([statistics.stdev(v) for v in kept.values() if len(v) >= 2])
        print(
            f"  by {label:<8} {len(kept):>3} groups   between-group sd of means {between:.4f}"
            f"   typical within-group sd {within:.4f}"
        )
        return between

    by_player = spread(lambda r: r.subject or "?", "GOLFER", 3)
    by_club = spread(lambda r: r.club, "CLUB", 10)
    tempo_floor = METRIC_TARGETS["tempo_ratio"].tolerance
    print(f"\n  golfer effect is {by_player / by_club:.1f}x the club effect")
    print(
        f"  and the club effect ({by_club:.4f}) is {tempo_floor / by_club:.1f}x smaller than our"
        f" own tempo error ({tempo_floor})"
    )


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print(__doc__)
        return 2

    swings = common.load_swings()
    derive_reference._drop_implausible(swings)
    derive_reference._normalize_handedness(swings)
    if not swings:
        print("no swings — run ingest_labels.py then derive_pose_metrics.py", file=sys.stderr)
        return 1

    rng = random.Random(_SEED)
    survivors = report_gate(swings, rng)
    report_pooled(swings, rng)
    report_tempo_axis(swings)

    print(f"\n{'=' * 78}\nVERDICT\n{'=' * 78}\n")
    if not survivors:
        print("  No per-club band clears its own measurement error on a player-clustered bound.")
    else:
        print(f"  {len(survivors)} stratum/strata survive both tests:")
        for metric, club, worst in sorted(survivors, key=lambda t: -t[2]):
            print(f"    {metric} x {club}   worst-case {worst:.2f}x floor")
    print("\n  Shipping any of them needs three things this repo does not have (see ADR-010's")
    print("  2026-08-18 per-club addendum): a corpus-to-ClubCategory mapping that is not a")
    print("  fiction, something that records which club was swung, and `_population_placement`")
    print("  drawing its percentile from the stratum the band actually resolved to.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
