"""Dev CLI: what one golfer's own history supports saying — and what it refuses to.
[Career mode, steps 4 and 6]

Usage:
    python scripts/career_baseline.py                    # every golfer in the registry
    python scripts/career_baseline.py --name Aaron       # one, by the name you'd type at the bay
    python scripts/career_baseline.py --player-id aaron  # one, by stored id
    python scripts/career_baseline.py --verbose          # show the per-session breakdown too

The companion to `scripts/career_corpus.py`. That one prints the honest `n`; this one prints what
that `n` buys. Today it buys nothing — every claim on every metric is refused — and that is the
output this step was built to produce. A baseline that printed a mean over two swings would look
like progress and be the exact failure career mode was deferred to avoid.

Every refusal names what it is waiting for, so the output doubles as a worklist: it says how many
more swings, and over how many more sessions, before each metric can speak.

**Step 6 added the `vs tour` row**, and it belongs here rather than in a fourth career CLI because
a standing is a statement about the *center* — which is what this CLI is about. It is also the one
row that can be blocked for a reason no bay session fixes: three of the eight metrics have no tour
population they may be placed in, and each says which kind of refusal it is.

Exits 0 always: an `n` of zero is a finding, not a failure.
"""

from __future__ import annotations

import argparse
import sys

from golf_coach.analysis.baseline import build_baseline
from golf_coach.analysis.comparison import build_standing
from golf_coach.config import settings
from golf_coach.contracts.baseline import BaselineClaim, MetricBaseline, PersonalBaseline
from golf_coach.contracts.comparison import GolferStanding, MetricComparison, Standing
from golf_coach.contracts.golfer import slugify
from golf_coach.storage.corpus import read_corpus
from golf_coach.storage.golfer_store import GolferStore

#: How each standing prints. Two deliberate choices, the second found by rendering a corpus that
#: could actually speak:
#:
#: - `straddles` is not "borderline". The interval crosses the band edge, so which side the golfer
#:   is on is unresolved rather than nearly-decided.
#: - `outside` is never printed bare, because it reads as a fault and for half the panel it is the
#:   opposite. `finish_balance_norm` sits on a one-sided `[0, high]` band, so a center below p10 is
#:   *better balanced* than the tour population — and "outside tour range" rendered that identically
#:   to a center above p90. Naming the side is factual where the bare word is a verdict.
_STANDING_LABEL = {
    Standing.INSIDE: "inside tour range",
    Standing.STRADDLES: "cannot tell",
    Standing.WITHHELD: "withheld",
}


def _standing_label(standing: MetricComparison) -> str:
    if standing.standing is Standing.OUTSIDE:
        below = standing.outside_by is not None and standing.outside_by < 0
        return "below tour range" if below else "above tour range"
    return _STANDING_LABEL[standing.standing]


def _fmt(value: float, unit: str) -> str:
    """Three significant-ish decimals for normalized metrics, one for degrees."""
    return f"{value:.1f}" if unit == "degrees" else f"{value:.3f}"


def _report_standing(standing: MetricComparison, unit: str) -> None:
    """Where the center sits in the tour population, or why it cannot be placed.

    Prints the band whenever one exists, even while the standing itself is withheld: "we have a
    reference for this and are waiting on your `n`" and "there is no reference and never will be
    from this data" are the two situations, and the band is what tells them apart at a glance.
    """
    line = f"    vs tour  {_standing_label(standing)}"

    if standing.band_low is not None and standing.band_high is not None:
        line += (
            f"   tour {_fmt(standing.band_low, unit)} .. {_fmt(standing.band_high, unit)}"
        )
        if standing.population_players is not None:
            line += f" ({standing.population_players} players)"

    if standing.percentile is not None:
        # "at least" at the rails: `percentile_of` clamps at the stored quantiles, so a 90 there
        # is a floor on how extreme the center is rather than its rank.
        qualifier = "at least " if standing.percentile_clamped else ""
        line += f"   {qualifier}{standing.percentile:.0f}th pct"

    if standing.outside_by is not None:
        sign = "+" if standing.outside_by > 0 else ""
        line += f"   {sign}{_fmt(standing.outside_by, unit)} past the edge"

    print(line)

    for reason in standing.unavailable:
        print(f"    {'':<8} blocked  — {reason}")


def _report_metric(
    metric: MetricBaseline, standing: MetricComparison | None, *, verbose: bool
) -> None:
    print(f"\n  {metric.name}  ({metric.unit})")
    print(f"    n = {metric.n} over {metric.n_sessions} session"
          f"{'' if metric.n_sessions == 1 else 's'}")

    # Note what these conditions are *not*: a call to `supports()`. A withheld claim leaves no
    # number behind, so "is there a number" and "may it be shown" are the same question — which is
    # the property the contract exists to give every consumer, this one included.
    if metric.mean is not None and metric.median is not None:
        line = f"    center   {_fmt(metric.mean, metric.unit)}"
        if metric.mean_ci is not None:
            line += (
                f"   95% CI {_fmt(metric.mean_ci.low, metric.unit)}"
                f" .. {_fmt(metric.mean_ci.high, metric.unit)}"
            )
        print(f"{line}   (median {_fmt(metric.median, metric.unit)})")

    if metric.sd is not None and metric.minimum is not None and metric.maximum is not None:
        line = f"    spread   sd {_fmt(metric.sd, metric.unit)}"
        if metric.sd_ci is not None:
            line += (
                f"   95% CI {_fmt(metric.sd_ci.low, metric.unit)}"
                f" .. {_fmt(metric.sd_ci.high, metric.unit)}"
            )
        print(f"{line}   (range {_fmt(metric.minimum, metric.unit)}"
              f" .. {_fmt(metric.maximum, metric.unit)})")

    if metric.supports(BaselineClaim.TREND):
        print("    trend    per session:")
        for session in metric.sessions:
            mean = "—" if session.mean is None else _fmt(session.mean, metric.unit)
            print(f"               {session.session_id}  n={session.n}  mean {mean}")

    for refusal in metric.withheld:
        print(f"    {refusal.claim.value:<8} withheld — {refusal.reason}")

    # Last, because a standing is derived from the center: printing it above the row that says
    # there is no center reads as the two being unrelated.
    if standing is not None:
        _report_standing(standing, metric.unit)

    if verbose and not metric.supports(BaselineClaim.TREND):
        # The evidence behind the trend refusal. Counts only: the per-session mean is itself a
        # claim, and it stays sealed with the rest of them.
        print("             sessions so far:")
        for session in metric.sessions:
            print(f"               {session.session_id}  n={session.n}")


def _report(
    baseline: PersonalBaseline,
    standing: GolferStanding,
    display_name: str,
    *,
    verbose: bool,
) -> None:
    print(f"\n{display_name}  ({baseline.player_id})")
    print("-" * 72)

    if not baseline.metrics:
        print("  No measurement on any swing yet — nothing to build a baseline from.")
        print("  Check `python scripts/career_corpus.py` for why: the commonest causes are")
        print("  swings that were never analyzed, and swings analyzed by an older engine.")
        return

    print(
        f"  Built from {baseline.built_from_swings} distinct swing"
        f"{'' if baseline.built_from_swings == 1 else 's'} across "
        f"{baseline.built_from_sessions} session"
        f"{'' if baseline.built_from_sessions == 1 else 's'}."
    )

    if baseline.nothing_sayable:
        print("\n  Nothing is sayable yet. Every claim below is refused, with what it needs.")

    for metric in baseline.metrics.values():
        _report_metric(metric, standing.metrics.get(metric.name), verbose=verbose)

    if baseline.nothing_sayable:
        print("\n  The unblock is a bay session: 20-30 swings with shots attached.")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--name", help="Golfer's name as typed, e.g. 'Aaron'.")
    parser.add_argument("--player-id", help="Stored id, e.g. 'aaron'.")
    parser.add_argument(
        "--verbose", action="store_true", help="Show the per-session breakdown behind a refusal."
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
        _report(
            build_baseline(corpus), build_standing(corpus), display_name, verbose=args.verbose
        )
    print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
