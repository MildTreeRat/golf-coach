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
        # Two-sided for a ratio that can err in either direction; one-sided from zero for the
        # "lower is better" pose metrics, matching `_score_within_range(low=0.0, ...)`.
        one_sided = row["metric"].endswith("_norm")
        low = 0.0 if one_sided else row["p10"]
        print(
            f'  {row["metric"]:<22} low={low:<7.2f} high={row["p90"]:<7.2f} '
            f'(n={row["n"]}, {row["n_players"]} golfers)'
        )


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
