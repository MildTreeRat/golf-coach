"""Aggregate reference swings into committed benchmark distributions. [M4-REF Phase A3]

Usage:
    python scripts/golfdb/derive_reference.py

Reads `data/reference/golfdb/swings.jsonl` (Tier 2, gitignored, per-clip) and writes
`src/golf_coach/analysis/benchmarks/golfdb_v1.json` (Tier 3, committed, aggregate).

**Tier 3 is the only GolfDB-derived artifact that enters git**, and that is a licensing decision,
not a size one: percentiles over a thousand clips are not a substantial reproduction of anyone's
dataset, whereas per-clip rows keyed to named tour players arguably are. Nothing here carries a
player name, a clip id, or a frame index. See ADR-012.

Stdlib only — the `research` extra's pandas is confined to `common.load_database()`, and the
percentile convention is shared with the checkpoint evaluators via `analysis.stats` so a band and
the swing measured against it can never be cut with different definitions of "p90".

This script does **not** edit `ranges.json`. It prints the recommended bands and a human moves
them across: ADR-010 makes every benchmark row an auditable, human-attested claim, and a script
silently rewriting the committed store would undercut that.
"""

from __future__ import annotations

import json
import statistics
import subprocess
import sys
from datetime import date
from typing import Any

import common

from golf_coach.analysis.stats import percentile
from golf_coach.contracts.reference import ReferenceSwing

SCHEMA_VERSION = 1
OUTPUT_PATH = common.BENCHMARKS_DIR / "golfdb_v1.json"

# Below this a "distribution" is an anecdote. Thin strata are dropped rather than published with a
# confident-looking p10/p90 cut from a handful of swings.
MIN_SAMPLES = 30

QUANTILES = (0.10, 0.25, 0.50, 0.75, 0.90)

#: Metrics whose sign is camera-relative, so a mixed-handedness corpus records the same swing
#: position under two opposite signs. Listed explicitly rather than inferred: "is this quantity
#: signed?" is a fact about what the metric means, and a heuristic over the observed values would
#: have to guess it back from data that the guess then conditions.
SIGNED_METRICS = frozenset({"head_hip_offset_impact_norm", "head_hip_gain_norm"})

#: Which signed metric decides a subject's handedness for all of them. The absolute offset, because
#: its population sits furthest from zero (median -0.62 against the gain's -0.40), so the fewest
#: golfers straddle the sign boundary where the test is a coin flip.
_HANDEDNESS_CLASSIFIER = "head_hip_offset_impact_norm"

#: Noise + boundary error for `_HANDEDNESS_CLASSIFIER`, from `tune_spatial_metric.py`. A subject
#: whose median sits inside this is classified on evidence weaker than the instrument, and is
#: reported rather than silently trusted.
_CLASSIFIER_ERROR = 0.082

#: Largest believable shoulder-width-normalized value. Above this the *ruler* has collapsed, not
#: the golfer moved: `shoulder_width` bottoms out at `MIN_SHOULDER_WIDTH` (0.02) when a golfer turns
#: side-on or the shoulders mis-detect, and every `_norm` metric divides by it, so a near-floor
#: width inflates whatever sits on top. Two clips of one driver swing read 5.44 and 6.13
#: shoulder-widths of head-hip offset — six shoulder widths is not a body.
#:
#: Applies to every `_norm` metric because they all share that denominator. It is a no-op for the
#: other four today (their largest values are 2.64 and 2.16), which is the point: it removes two
#: known-impossible readings and leaves every shipped band where it was. Quantiles were already
#: robust to these, so the row this actually corrects is `sd` — inflated from 0.249 to 0.504 by
#: those two clips alone.
MAX_PLAUSIBLE_NORM = 3.0

CITATION = (
    "McNally et al., GolfDB: A Video Database for Golf Swing Sequencing, CVPR Workshops 2019"
)
LICENSE_NOTE = (
    "Upstream code is CC BY-NC 4.0; the dataset itself states no license and the source clips are "
    "third-party broadcast footage. Only aggregate statistics are reproduced here - no clips, no "
    "annotations, no per-player rows. See ADR-012."
)


def _load_swings() -> list[ReferenceSwing]:
    if not common.SWINGS_JSONL.exists():
        raise FileNotFoundError(
            f"{common.SWINGS_JSONL} not found — run `python scripts/golfdb/ingest_labels.py` first."
        )
    with common.SWINGS_JSONL.open(encoding="utf-8") as handle:
        return [ReferenceSwing.model_validate_json(line) for line in handle if line.strip()]


def _drop_implausible(swings: list[ReferenceSwing]) -> dict[str, int]:
    """Remove `_norm` readings past `MAX_PLAUSIBLE_NORM`. Returns per-metric drop counts.

    Mutates the in-memory rows only — `swings.jsonl` is written by `derive_pose_metrics.py` and
    stays the faithful record of what was measured. Screening belongs to the aggregation step,
    where it can be reported, not to the measurement step, where it would erase evidence.
    """
    dropped: dict[str, int] = {}
    for swing in swings:
        for metric in list(swing.metrics):
            if not metric.endswith("_norm"):
                continue
            if abs(swing.metrics[metric]) > MAX_PLAUSIBLE_NORM:
                del swing.metrics[metric]
                dropped[metric] = dropped.get(metric, 0) + 1
    return dropped


def _normalize_handedness(swings: list[ReferenceSwing]) -> tuple[list[str], list[str]]:
    """Fold left-handed golfers onto the right-handed sign convention.

    Returns `(lefties, weakly_classified)`.

    **The metric's own sign is the handedness label, and it checks out by name.** A face-on camera
    sees a left-handed golfer's swing mirrored, so every signed quantity flips; a golfer's *median*
    is therefore a far more robust handedness signal than any single clip, and GolfDB carries no
    handedness column to read instead.

    Exactly four of 122 subjects come out positive, and two of them are Phil Mickelson and Bubba
    Watson — the two best-known left-handers in professional golf, identified by a test that knows
    nothing about them. That is the validation this has instead of a label, and it is why the
    classification is per *subject* rather than per clip: at the clip level the same test would
    reclassify a golfer's one outlying swing as a change of handedness.

    **One golfer gets one label, from one metric.** Classifying each signed metric independently
    is the obvious implementation and it is wrong: it gave `TOBY KEITH` opposite handedness on the
    two metrics, from medians of -0.110 and +0.015 that are both inside measurement error. A person
    does not swing right-handed under one measurement and left-handed under another, so handedness
    is resolved once — from `_HANDEDNESS_CLASSIFIER`, whose population sits furthest from zero
    (median -0.62) and therefore has the least ambiguous sign — and that verdict is applied to every
    signed metric.

    Subjects whose evidence is inside measurement error are still classified, because the sign test
    puts Mickelson on the correct side even at +0.011 and folding a near-zero value barely moves it
    either way. They are *reported* rather than silently trusted, which is the part that matters.

    Called after `_drop_implausible`, deliberately. A blown-up ruler produces large values of
    arbitrary sign, and enough of them under one subject would flip that subject's median — so
    screening has to come first or the classifier reads its own artifacts.
    """
    by_subject: dict[str, list[float]] = {}
    for swing in swings:
        if swing.subject and _HANDEDNESS_CLASSIFIER in swing.metrics:
            by_subject.setdefault(swing.subject, []).append(
                swing.metrics[_HANDEDNESS_CLASSIFIER]
            )

    medians = {subject: statistics.median(values) for subject, values in by_subject.items()}
    lefties = {subject for subject, median in medians.items() if median > 0}
    weak = sorted(s for s in medians if abs(medians[s]) < _CLASSIFIER_ERROR)

    for swing in swings:
        if swing.subject not in lefties:
            continue
        for metric in SIGNED_METRICS & swing.metrics.keys():
            swing.metrics[metric] = -swing.metrics[metric]
    return sorted(lefties), weak


def _strata(swings: list[ReferenceSwing]) -> list[tuple[str, str, str, list[ReferenceSwing]]]:
    """`(club, sex, view, members)` groups: the overall population plus one-axis marginals.

    Marginals only, not the full cross-product. Five clubs x two sexes x three views would mostly
    produce cells under `MIN_SAMPLES`, and publishing a band per empty-ish cell invites reading
    noise as a finding. Cross-strata can be added when a checkpoint actually needs one.
    """
    groups: list[tuple[str, str, str, list[ReferenceSwing]]] = [("all", "all", "all", swings)]

    for club in sorted({s.club for s in swings if s.club}):
        groups.append((club, "all", "all", [s for s in swings if s.club == club]))
    for sex in sorted({s.sex for s in swings if s.sex}):
        groups.append(("all", sex, "all", [s for s in swings if s.sex == sex]))
    for view in sorted({s.view for s in swings if s.view}):
        groups.append(("all", "all", view, [s for s in swings if s.view == view]))
    return groups


def _summarize(
    metric: str, club: str, sex: str, view: str, members: list[ReferenceSwing]
) -> dict[str, Any] | None:
    values = [s.metrics[metric] for s in members if metric in s.metrics]
    if len(values) < MIN_SAMPLES:
        return None

    # Distinct golfers, not just clips. A band drawn from 200 clips of 6 players is far weaker
    # than one drawn from 200 players, and nothing else in the file would reveal the difference.
    players = len({s.subject for s in members if s.subject and metric in s.metrics})

    summary: dict[str, Any] = {
        "metric": metric,
        "club": club,
        "sex": sex,
        "view": view,
        "n": len(values),
        "n_players": players,
        "mean": round(statistics.fmean(values), 4),
        "sd": round(statistics.stdev(values), 4) if len(values) > 1 else 0.0,
    }
    for q in QUANTILES:
        summary[f"p{int(q * 100)}"] = round(percentile(values, q), 4)
    return summary


def _pipeline_commit() -> str:
    try:
        return subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=common.REPO_ROOT,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    except (subprocess.CalledProcessError, OSError):
        return "unknown"


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print("usage: python scripts/golfdb/derive_reference.py", file=sys.stderr)
        return 2

    swings = _load_swings()
    print(f"Loaded {len(swings)} reference swings")

    dropped = _drop_implausible(swings)
    for metric, count in sorted(dropped.items()):
        print(f"  dropped {count} reading(s) of {metric} past {MAX_PLAUSIBLE_NORM} shoulder-widths")

    lefties, weak = _normalize_handedness(swings)
    if lefties:
        print(
            f"  handedness (from {_HANDEDNESS_CLASSIFIER}): folded {len(lefties)} "
            f"left-handed subject(s) — {', '.join(lefties)}"
        )
    if weak:
        print(
            f"  {len(weak)} subject(s) classified on a median inside the "
            f"{_CLASSIFIER_ERROR} error floor — {', '.join(weak)}"
        )

    metrics = sorted({name for s in swings for name in s.metrics})
    estimators = sorted({s.pose_estimator for s in swings if s.pose_estimator})
    versions = sorted({s.metric_definitions_version for s in swings})
    if len(versions) > 1:
        print(
            f"error: rows carry mixed metric_definitions_version {versions}; "
            "re-run the ingest scripts so every row is measured the same way.",
            file=sys.stderr,
        )
        return 1

    distributions = [
        summary
        for metric in metrics
        for club, sex, view, members in _strata(swings)
        if (summary := _summarize(metric, club, sex, view, members)) is not None
    ]

    payload = {
        "schema_version": SCHEMA_VERSION,
        "dataset": {
            "name": "GolfDB",
            "citation": CITATION,
            "url": "https://github.com/wmcnally/golfdb",
            "license_note": LICENSE_NOTE,
            "pose_estimator": estimators[0] if len(estimators) == 1 else (estimators or None),
            "metric_definitions_version": versions[0],
            "min_samples": MIN_SAMPLES,
            "derived_on": date.today().isoformat(),
            "pipeline_commit": _pipeline_commit(),
        },
        "distributions": distributions,
    }

    OUTPUT_PATH.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {len(distributions)} distributions -> {OUTPUT_PATH}")

    _report(distributions)
    return 0


def _report(distributions: list[dict[str, Any]]) -> None:
    """Print the overall row per metric and the band it implies, for the human doing the edit."""
    everything = ("all", "all", "all")
    overall = [d for d in distributions if (d["club"], d["sex"], d["view"]) == everything]
    if not overall:
        return

    print("\nOverall (all clubs / sexes / views):")
    print(f"  {'metric':<22} {'n':>5} {'players':>8} {'p10':>7} {'p50':>7} {'p90':>7}")
    for row in overall:
        print(
            f"  {row['metric']:<22} {row['n']:>5} {row['n_players']:>8} "
            f"{row['p10']:>7.2f} {row['p50']:>7.2f} {row['p90']:>7.2f}"
        )

    print("\nRecommended ranges.json bands (paste by hand — see module docstring):")
    for row in overall:
        low, high, shape = _recommended_band(row)
        print(
            f'  {row["metric"]:<26} low={low:<7.2f} high={high:<7.2f} '
            f'{shape:<10} (n={row["n"]}, {row["n_players"]} golfers)'
        )
    print(
        "\nA recommendation is a starting point, not a band. ADR-010's addendum (2026-08-12) asks\n"
        "for one more check before any edge ships: assert an edge only where it clears the\n"
        "instrument — compare it against the metric's noise+boundary error from\n"
        "tune_spatial_metric.py, and drop the edge if the two are comparable."
    )


def _recommended_band(row: dict[str, Any]) -> tuple[float, float, str]:
    """`(low, high, shape)` for one overall row.

    **A `_norm` suffix does not mean "lower is better".** It means shoulder-width normalized, and
    the old rule read it as the former: one-sided `[0, p90]` for anything ending `_norm`. That is
    right for head sway and finish drift, wrong for the hip metrics (some lateral travel is the
    weight shift a swing needs — ADR-010 addendum 2026-08-12), and *incoherent* for a signed
    quantity, where it printed `low=0.00 high=-0.33` — an empty interval — because it assumed
    values are non-negative. `measure.py` carried a warning telling readers not to paste that,
    which is a note where a fix belonged.

    The distribution itself settles the only part a script can settle: a population sitting mostly
    below zero cannot have a band starting at zero. Whether a *non-negative* metric deserves a lower
    edge stays a human call, so those keep the one-sided default and the caller is told to check it.
    """
    if row["p90"] < 0 or row["p10"] < 0:
        return row["p10"], row["p90"], "two-sided"
    if row["metric"].endswith("_norm"):
        return 0.0, row["p90"], "one-sided"
    return row["p10"], row["p90"], "two-sided"


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
