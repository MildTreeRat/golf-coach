"""Dev CLI: how far one golfer hits each club — and where their history refuses to say. [M9 P17]

Usage:
    python scripts/club_profile.py                    # every golfer in the registry
    python scripts/club_profile.py --name Aaron       # one, by the name you'd type at the bay
    python scripts/club_profile.py --player-id aaron  # one, by stored id
    python scripts/club_profile.py --club "7 iron"    # one club, in any spelling parse_club takes
    python scripts/club_profile.py --verbose          # the per-session counts behind a refusal

The fourth career-shaped CLI, and the first reader of anything M9 built. The other three ask one
corpus three questions: `career_corpus.py` prints the honest `n`, `career_baseline.py` prints what
that `n` buys, `career_dispersion.py` prints what the numbers are evidence for. All three read the
whole bag. This one cuts the same corpus by club, which is the only cut on which a distance means
anything — a mean carry pooled over a driver and a wedge describes nobody's shot.

**The correct output today is the empty state, not a table of refusals.** The phase list expected
refusals; there are none to print, because a club with no swings and no bag entry gets no row at
all and every swing on disk predates the club tag. So `_report_empty` says that in full — how many
untagged swings there are, and where the tag comes from. A golfer's name followed by a blank would
read as a bug in this CLI when the finding is that nothing on disk names a club.

Exits 0 always, except on input this cannot use: a name that slugs to nothing, or club text that is
not a club. An `n` of zero is a finding, not a failure.
"""

from __future__ import annotations

import argparse
import sys

from golf_coach.analysis.club_profile import build_bag_profile
from golf_coach.config import settings
from golf_coach.contracts.bag import BagEntry
from golf_coach.contracts.baseline import BaselineClaim, MetricBaseline
from golf_coach.contracts.club import ClubId, parse_club
from golf_coach.contracts.club_profile import BagProfile, ClubProfile
from golf_coach.contracts.dispersion import Finding, MetricDispersion
from golf_coach.contracts.golfer import slugify
from golf_coach.storage.bag_store import BagStore
from golf_coach.storage.corpus import read_corpus
from golf_coach.storage.golfer_store import GolferStore

#: How each finding prints, copied from `career_dispersion.py` with its reasoning intact:
#: `NOT_ESTABLISHED` is not "no", it is "there is enough data to ask and the data does not settle
#: it", and a column reading "no" beside a real number would be read as a clean bill.
_FINDING_LABEL = {
    Finding.ESTABLISHED: "yes",
    Finding.NOT_ESTABLISHED: "cannot tell",
    Finding.WITHHELD: "withheld",
}

#: Unit -> decimal places. **This is the one helper the career CLIs could not be copied from.**
#: Their rule is `1dp if unit == "degrees" else 3dp`, written when every other metric in the repo
#: was shoulder-width-normalized. M9 P8-P10 put `yards` and `mph` into `measurements` and `ms` was
#: already there, so that rule prints "152.000 yards" and "384.000 ms" — precision the launch
#: monitor does not have and nobody reads.
_PRECISION = {
    "degrees": 1,
    "yards": 1,
    "mph": 1,
    "ms": 1,
    "ratio": 3,
    "shoulder_widths": 3,
}

#: An unlisted unit takes three decimals rather than one, deliberately. The two failures are not
#: symmetric: too much precision is noise a reader can see and complain about, too little is a
#: digit that quietly went missing. Same asymmetry `parse_club` argues from — the cheap failure
#: wins.
_DEFAULT_PRECISION = 3

#: Width of the label column, from `career_dispersion.py`. "untagged" is the longest here.
_LABEL_WIDTH = 10


def _fmt(value: float, unit: str) -> str:
    return f"{value:.{_PRECISION.get(unit, _DEFAULT_PRECISION)}f}"


def _plural(count: int, noun: str) -> str:
    return f"{count} {noun}{'' if count == 1 else 's'}"


def _label(label: str, indent: int = 6) -> str:
    """The label column, so every row in a block lines up.

    Takes an indent because this CLI nests one level deeper than the career ones: a metric sits
    inside a club, which sits inside a golfer.
    """
    return f"{' ' * indent}{label:<{_LABEL_WIDTH}}"


def _print_block(label: str, text: str, indent: int = 6) -> None:
    """A labelled paragraph, wrapped and hanging-indented under its label.

    The reasons here are sentences, not fragments — a refusal has to say what it is waiting for —
    and several run past 200 characters, which is unreadable as one line in a terminal.
    """
    for i, line in enumerate(_wrap(text, width=94 - indent - _LABEL_WIDTH)):
        print(_label(label if i == 0 else "", indent) + line)


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


# --------------------------------------------------------------------------------------
# One metric
# --------------------------------------------------------------------------------------


def _report_metric(
    baseline: MetricBaseline, dispersion: MetricDispersion | None, *, verbose: bool
) -> None:
    """One metric for one club: what it says, what it will not say, and what it is evidence for.

    Merges what the two career CLIs each print half of. They are separate there because a baseline
    and a miss-shape are separate questions about the whole bag; per club they are one row of one
    table, and asking someone to run two commands to read one club would be the worse split.
    """
    unit = baseline.unit
    print(f"\n    {baseline.name}  ({unit})")
    print(f"      n = {baseline.n} over {_plural(baseline.n_sessions, 'session')}")

    # Note what these conditions are *not*: a call to `supports()`. A withheld claim leaves no
    # number behind, so "is there a number" and "may it be shown" are the same question — the
    # property `contracts/baseline.py` exists to give every consumer, this one included.
    if baseline.mean is not None and baseline.median is not None:
        line = _label("center") + _fmt(baseline.mean, unit)
        if baseline.mean_ci is not None:
            line += (
                f"   95% CI {_fmt(baseline.mean_ci.low, unit)}"
                f" .. {_fmt(baseline.mean_ci.high, unit)}"
            )
        print(f"{line}   (median {_fmt(baseline.median, unit)})")

    if baseline.sd is not None and baseline.minimum is not None and baseline.maximum is not None:
        line = _label("spread") + f"sd {_fmt(baseline.sd, unit)}"
        if baseline.sd_ci is not None:
            line += (
                f"   95% CI {_fmt(baseline.sd_ci.low, unit)}"
                f" .. {_fmt(baseline.sd_ci.high, unit)}"
            )
        print(f"{line}   (range {_fmt(baseline.minimum, unit)} .. {_fmt(baseline.maximum, unit)})")

    if baseline.supports(BaselineClaim.TREND):
        print(_label("trend") + "per session:")
        for session in baseline.sessions:
            mean = "—" if session.mean is None else _fmt(session.mean, unit)
            print(_label("") + f"  {session.session_id}  n={session.n}  mean {mean}")

    if dispersion is not None:
        _report_findings(dispersion)

    # These sentences explain the absence of everything above them, so they follow it. `waiting`
    # before `blocked`, as `career_dispersion.py` orders them and for its reason: the two need
    # opposite responses, and the one no amount of swinging fixes reads best as the last word.
    for refusal in baseline.withheld:
        _print_block(refusal.claim.value, refusal.reason)
    if dispersion is not None:
        for reason in dispersion.unavailable:
            _print_block("blocked", reason)

    if verbose and not baseline.supports(BaselineClaim.TREND):
        # The evidence behind the trend refusal. Counts only: a per-session mean is itself a claim,
        # and it stays sealed with the rest of them.
        print(_label("") + "sessions so far:")
        for session in baseline.sessions:
            print(_label("") + f"  {session.session_id}  n={session.n}")


def _report_findings(metric: MetricDispersion) -> None:
    """The bias/scatter pair — a repeatable miss against a scattered one.

    **`metric.withheld` is deliberately not printed.** `analysis/dispersion.py`'s `_carry_refusals`
    builds that list by *filtering* the baseline's own, so every sentence in it is already printed
    verbatim by the caller. On today's output — which is nothing but refusals — rendering both
    would double the length of every metric block to say each thing twice. Nothing is hidden by the
    omission: the finding labels below still read "withheld", and `unavailable` is a different list
    saying a different thing, so the caller prints it — after the refusals, not here.
    """
    line = _label("finding") + f"bias {_FINDING_LABEL[metric.bias]}"
    line += f"    scatter {_FINDING_LABEL[metric.scatter]}"
    if metric.target is not None and metric.tolerance is not None:
        line += (
            f"    target {_fmt(metric.target, metric.unit)}"
            f" +/- {_fmt(metric.tolerance, metric.unit)}"
        )
    print(line)

    # Only worth a column when it is not the center restated: every target in the table today is
    # either 0.0 or absent, so this stays quiet until a nonzero one is declared.
    if metric.offset is not None and metric.target:
        print(_label("") + f"off target by {_fmt(metric.offset, metric.unit)}")
    if metric.within_session_sd is not None:
        print(_label("") + f"within-session sd {_fmt(metric.within_session_sd, metric.unit)}")
    if metric.pattern is not None:
        print(_label("pattern") + metric.pattern.value.replace("_", " "))
    if metric.points_at:
        _print_block("check", metric.points_at)

    for caveat in metric.caveats:
        _print_block("caveat", caveat)


# --------------------------------------------------------------------------------------
# One club
# --------------------------------------------------------------------------------------


def _report_club(profile: ClubProfile, *, verbose: bool) -> None:
    print(f"\n  {profile.club.value}  ({profile.category.value.replace('_', ' ')})")
    _print_bag_entry(profile.bag_entry)

    # All three counters, never one. `contracts/club_profile.py` opens with why they diverge per
    # club: a 7 iron filmed six times with two shot-screen photos is six swings of history and a
    # carry ceiling of two, because every launch-monitor claim dedupes on the photo's hash.
    evidence = (
        f"{_plural(profile.n_swings, 'swing')}, "
        f"{_plural(profile.n_shots, 'shot photo')}, "
        f"{_plural(profile.n_sessions, 'session')}"
    )
    if profile.n_shots < profile.n_swings:
        evidence += " — so every distance and launch number below is capped at the shot count"
    _print_block("evidence", evidence, 4)

    for caveat in profile.caveats:
        _print_block("caveat", caveat, 4)

    if not profile.metrics:
        _print_block("history", _no_metrics_reason(profile), 4)
        return

    for name, baseline in profile.metrics.items():
        _report_metric(baseline, profile.dispersion.get(name), verbose=verbose)


def _no_metrics_reason(profile: ClubProfile) -> str:
    """Why a profile exists with nothing in it — and the two cases need opposite responses.

    A declared club nobody has hit is not a failure of anything; it is the state every club is in
    the day it is added to the bag, and the profile exists so the club is *visible* before it has
    history. A club with swings but no measurement is a pipeline problem, and `career_corpus.py`
    is where the reason is recorded.
    """
    if profile.n_swings == 0:
        return (
            "In the bag, nothing hit with it yet. No statistics, and that is not an error — the "
            "profile is here so the club is visible before it has any history."
        )
    return (
        "These swings carry no measurement. Run `python scripts/career_corpus.py` for why — the "
        "commonest causes are swings that were never analyzed, and swings analyzed by an older "
        "engine."
    )


def _print_bag_entry(entry: BagEntry | None) -> None:
    """The physical club in this slot, or what its absence costs.

    An undeclared entry is spelled out rather than left blank because the two halves of it need
    saying together: it takes nothing away from the distance statistics, and it makes every loft or
    fitting question about this club refuse. That is `SwingResult.unscored`'s per-input refusal
    applied to a different missing input (ADR-024 §2), and a blank cell would read as a zero loft.
    """
    if entry is None:
        _print_block(
            "club",
            "No bag entry declared. Every statistic below is unaffected; a loft or fitting "
            "question about this club has to refuse, and no catalogue default is substituted for "
            "a loft nobody measured.",
            4,
        )
        return

    described = " ".join(part for part in (entry.make, entry.model) if part)
    line = described or "make and model not recorded"
    line += (
        f", {entry.loft_deg:g} deg loft" if entry.loft_deg is not None else ", loft not measured"
    )
    if entry.shaft:
        line += f", {entry.shaft} shaft"
    if entry.length_in is not None:
        line += f", {entry.length_in:g} in"
    line += f"  (declared {entry.recorded_at:%Y-%m-%d})"
    _print_block("club", line, 4)


# --------------------------------------------------------------------------------------
# One golfer
# --------------------------------------------------------------------------------------


def _report(profile: BagProfile, display_name: str, *, verbose: bool, club: ClubId | None) -> None:
    print(f"\n{display_name}  ({profile.player_id})")
    print("-" * 72)

    if club is not None:
        # `BagProfile.profile_for`, never a hand-rolled scan: P14 put the lookup on the contract so
        # this and P18's `get_club_profile` cannot disagree about what "no such club" means.
        one = profile.profile_for(club)
        if one is None:
            print(f"  {club.value}: never hit and not in the bag — there is no history to report.")
        else:
            _report_club(one, verbose=verbose)
        _print_untagged(profile)
        return

    if not profile.clubs:
        _report_empty(profile)
        return

    print(
        f"  {_plural(len(profile.clubs), 'club')} with history or a declared bag entry: "
        f"{len(profile.clubs_used)} hit, {len(profile.clubs_declared)} in the bag."
    )
    for one in profile.clubs:
        _report_club(one, verbose=verbose)
    _print_untagged(profile)


def _report_empty(profile: BagProfile) -> None:
    """No club hit and none declared — **the state on disk today, and a finding not a blank page.**

    Its own branch because "no clubs" and "clubs that cannot say anything yet" look identical from
    a distance and need opposite responses. A table of refusals means go and hit balls; an empty
    bag profile means nothing on disk names a club at all, which no bay session fixes on its own.
    The phase list predicted the first and this is the second, so the message has to carry the
    untagged count and where a tag comes from or it says nothing at all.
    """
    print("  No club has been hit or declared — there is nothing to profile yet.")

    if profile.untagged_swings:
        # Labelled rather than left as a bare paragraph, and the label is the same word the
        # populated path uses: this *is* `_print_untagged`'s number, printed here because the
        # empty state returns before reaching it.
        _print_block(
            "untagged",
            f"All {_plural(profile.untagged_swings, 'swing')} on record name no club. That is "
            "real history, and no per-club number can be cut from it until each one is tagged.",
            2,
        )
    else:
        print("  No swings on record either.")

    _print_block(
        "next",
        "New swings get a club at upload — `python scripts/run_server.py`, then the picker on the "
        "upload page. A swing already stored is retagged one at a time, from the change control on "
        "its row in that page's swing list or with POST "
        "/api/sessions/<session>/swings/<swing>/club.",
        2,
    )


def _print_untagged(profile: BagProfile) -> None:
    """The history no club profile above can see. Printed last, and never folded into a club.

    `BagProfile.untagged_swings` is read off the whole corpus, so it is the one number here that
    every per-club narrowing reports as zero by construction. Leaving it out would make a bag look
    complete when most of the golfer's swings are missing from it.
    """
    if not profile.untagged_swings:
        return
    print()
    _print_block(
        "untagged",
        f"{_plural(profile.untagged_swings, 'swing')} on record name no club, so nothing above "
        "counts them. Retag one at a time, from the change control on the swing's row in the "
        "upload page or with POST /api/sessions/<session>/swings/<swing>/club.",
        2,
    )


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--name", help="Golfer's name as typed, e.g. 'Aaron'.")
    parser.add_argument("--player-id", help="Stored id, e.g. 'aaron'.")
    parser.add_argument("--club", help="One club: '7i', '7 iron', 'seven iron', 'driver', 'pw'.")
    parser.add_argument(
        "--verbose", action="store_true", help="Show the per-session counts behind a refusal."
    )
    args = parser.parse_args(argv)

    club: ClubId | None = None
    if args.club:
        club = parse_club(args.club)
        if club is None:
            # Refused rather than nudged toward a nearest match, which is `parse_club`'s own
            # posture and its argument: a retype costs one line, and a wrong club pools a wedge's
            # carries into a 7 iron's where nothing downstream will ever flag it.
            print(f"'{args.club}' is not a club. Try '7i', '7 iron', 'driver' or 'pw'.")
            print("'wedge' and 'iron' name a category rather than a club, so neither is accepted.")
            return 2

    golfers = GolferStore(settings.golfers_dir)
    # Same directory as the golfer records — a bag sits beside the golfer it belongs to, as
    # `api/app.py` builds it. `get` is tolerant: an absent or unreadable bag reads as None, and
    # None is not a degraded mode, it costs the loft and nothing else.
    bags = BagStore(settings.golfers_dir)

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
        profile = build_bag_profile(corpus, bags.get(player_id))
        _report(profile, display_name, verbose=args.verbose, club=club)
    print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
