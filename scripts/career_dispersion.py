"""Dev CLI: what the *shape* of a golfer's miss points at. [Career mode, step 5]

Usage:
    python scripts/career_dispersion.py                    # every golfer in the registry
    python scripts/career_dispersion.py --name Aaron       # one, by the name you'd type at the bay
    python scripts/career_dispersion.py --player-id aaron  # one, by stored id
    python scripts/career_dispersion.py --verbose          # show the tolerance and its provenance

The third of the three career CLIs, and they answer three different questions about one corpus:
`career_corpus.py` prints the honest `n`, `career_baseline.py` prints what that `n` buys, and this
prints what the numbers are *evidence for* — a repeatable miss (look at what is fixed before the
swing) against a scattered one (look at timing and release).

Today it prints refusals on every metric, and that is the intended output: two swings cannot
distinguish a grip problem from a timing problem, and a mechanism that pretended otherwise would be
worse than none. Every refusal says which of the two kinds it is — waiting for swings, or waiting
for a band — so the output doubles as a worklist.

Exits 0 always: an `n` of zero is a finding, not a failure.
"""

from __future__ import annotations

import argparse
import sys

from golf_coach.analysis.dispersion import build_dispersion
from golf_coach.config import settings
from golf_coach.contracts.dispersion import (
    Finding,
    GolferDispersion,
    MetricDispersion,
    target_for,
)
from golf_coach.contracts.golfer import slugify
from golf_coach.storage.corpus import read_corpus
from golf_coach.storage.golfer_store import GolferStore

#: How each finding prints. `NOT_ESTABLISHED` is deliberately not "no" — it is the answer "there is
#: enough data to ask and the data does not settle it", and a column reading "no" beside a mean of
#: 2.5 degrees would be read as a clean bill.
_FINDING_LABEL = {
    Finding.ESTABLISHED: "yes",
    Finding.NOT_ESTABLISHED: "cannot tell",
    Finding.WITHHELD: "withheld",
}


def _fmt(value: float, unit: str) -> str:
    """One decimal for degrees, three for the normalized metrics. Matches career_baseline.py."""
    return f"{value:.1f}" if unit == "degrees" else f"{value:.3f}"


def _report_metric(metric: MetricDispersion, *, verbose: bool) -> None:
    print(f"\n  {metric.name}  ({metric.unit})")

    header = (
        f"    n = {metric.n} over {metric.n_sessions} session"
        f"{'' if metric.n_sessions == 1 else 's'}"
    )
    if metric.target is not None and metric.tolerance is not None:
        header += (
            f"    target {_fmt(metric.target, metric.unit)}"
            f" +/- {_fmt(metric.tolerance, metric.unit)}"
        )
    print(header)

    # As in career_baseline.py, these conditions are not calls to a "may I show this" predicate.
    # A finding that was never established leaves no number behind, so "is there a number" and
    # "may it be shown" are the same question.
    bias = _label("bias") + _FINDING_LABEL[metric.bias]
    if metric.center is not None:
        bias += f"    center {_fmt(metric.center, metric.unit)}"
        if metric.center_ci is not None:
            bias += (
                f" (95% CI {_fmt(metric.center_ci.low, metric.unit)}"
                f" .. {_fmt(metric.center_ci.high, metric.unit)})"
            )
        # Only worth a column when it is not the center restated: every target in the table today
        # is either 0.0 or absent, so this stays quiet until a nonzero one is declared.
        if metric.offset is not None and metric.target:
            bias += f"    off target by {_fmt(metric.offset, metric.unit)}"
    print(bias)

    scatter = _label("scatter") + _FINDING_LABEL[metric.scatter]
    if metric.sd is not None:
        scatter += f"    sd {_fmt(metric.sd, metric.unit)}"
        if metric.sd_ci is not None:
            scatter += (
                f" (95% CI {_fmt(metric.sd_ci.low, metric.unit)}"
                f" .. {_fmt(metric.sd_ci.high, metric.unit)})"
            )
    if metric.within_session_sd is not None:
        scatter += f"    within-session {_fmt(metric.within_session_sd, metric.unit)}"
    print(scatter)

    if metric.pattern is not None:
        print(_label("pattern") + metric.pattern.value.replace("_", " "))
    for line in _wrap(metric.points_at):
        print(_label("") + line)
    for caveat in metric.caveats:
        _print_block("caveat", caveat)

    for refusal in metric.withheld:
        _print_block("waiting", refusal.reason)
    for reason in metric.unavailable:
        _print_block("blocked", reason)

    if verbose and metric.tolerance is not None:
        declared = target_for(metric.name)
        if declared is not None:
            _print_block("tolerance", f"{declared.tolerance:g} — {declared.provenance}")


#: Width of the label column. "tolerance" is the longest and needs a gap after it — at 9 it
#: butted straight up against the value and printed "tolerance2".
_LABEL_WIDTH = 10


def _label(label: str) -> str:
    """The label column, so every row in a metric block lines up at the same column."""
    return f"    {label:<{_LABEL_WIDTH}}"


def _print_block(label: str, text: str) -> None:
    """A labelled paragraph, wrapped and hanging-indented under its label.

    The reasons here are sentences, not fragments — a refusal has to say what it is waiting for —
    and several run past 200 characters, which is unreadable as one line in a terminal.
    """
    for i, line in enumerate(_wrap(text)):
        print(_label(label if i == 0 else "") + line)


def _wrap(text: str | None, width: int = 78) -> list[str]:
    """Naive greedy wrap. Not textwrap.fill, because the indent differs per caller."""
    if not text:
        return []
    lines: list[str] = []
    current = ""
    for word in text.split():
        if current and len(current) + 1 + len(word) > width:
            lines.append(current)
            current = word
        else:
            current = f"{current} {word}".strip()
    if current:
        lines.append(current)
    return lines


def _report(dispersion: GolferDispersion, display_name: str, *, verbose: bool) -> None:
    print(f"\n{display_name}  ({dispersion.player_id})")
    print("-" * 72)

    if not dispersion.metrics:
        print("  No measurement on any swing yet — nothing to read a miss-shape from.")
        print("  Check `python scripts/career_corpus.py` for why.")
        return

    print(
        f"  Built from {dispersion.built_from_swings} distinct swing"
        f"{'' if dispersion.built_from_swings == 1 else 's'} across "
        f"{dispersion.built_from_sessions} session"
        f"{'' if dispersion.built_from_sessions == 1 else 's'}."
    )

    if dispersion.nothing_established:
        print(
            "\n  No metric can be asked yet. A repeatable miss and a scattered one are the same\n"
            "  two numbers until there are enough of them to tell apart."
        )

    for metric in dispersion.metrics.values():
        _report_metric(metric, verbose=verbose)

    if dispersion.nothing_established:
        print("\n  The unblock is a bay session: 20-30 swings with shots attached.")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--name", help="Golfer's name as typed, e.g. 'Aaron'.")
    parser.add_argument("--player-id", help="Stored id, e.g. 'aaron'.")
    parser.add_argument(
        "--verbose", action="store_true", help="Show each tolerance and where it came from."
    )
    args = parser.parse_args(argv)

    golfers = GolferStore(settings.golfers_dir)

    if args.name or args.player_id:
        player_id = args.player_id or slugify(args.name)
        if not player_id:
            print(f"'{args.name}' has no usable characters for an id.")
            return 2
        golfer = golfers.get(player_id)
        targets = [(player_id, golfer.display_name if golfer else f"{player_id} (not registered)")]
    else:
        targets = [(g.player_id, g.display_name) for g in golfers.list_all()]
        if not targets:
            print(f"No golfers registered in {settings.golfers_dir}.")
            return 0

    for player_id, display_name in targets:
        corpus = read_corpus(settings.sessions_dir, player_id)
        _report(build_dispersion(corpus), display_name, verbose=args.verbose)
    print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
