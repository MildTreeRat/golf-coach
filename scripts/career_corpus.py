"""Dev CLI: the honest-`n` counter for one golfer's career corpus. [Career mode, step 2]

Usage:
    python scripts/career_corpus.py                    # every golfer in the registry
    python scripts/career_corpus.py --name Aaron       # one, by the name you would type at the bay
    python scripts/career_corpus.py --player-id aaron  # one, by stored id
    python scripts/career_corpus.py --verbose          # list every excluded swing individually

Prints what `storage.corpus.read_corpus` found: how many swing directories were read, how many of
them are actually distinct swings, and the sample size behind each metric.

**The gap between those first two numbers is the reason this exists.** A baseline built by counting
directories would report a golfer's numbers as many times as their clips were re-uploaded, and
repeating one measurement does not just inflate `n` — it drives the variance toward zero, which is
the single quantity career mode is being built to read (a tight spread points at a static cause,
a wide one at timing). This prints the number a minimum-N guard should be allowed to see.

Exits 0 always: an `n` of zero is a finding, not a failure.
"""

from __future__ import annotations

import argparse
import sys

from golf_coach.config import settings
from golf_coach.contracts.career import CareerCorpus
from golf_coach.contracts.golfer import slugify
from golf_coach.contracts.swing import ANALYSIS_VERSION
from golf_coach.storage.corpus import read_corpus
from golf_coach.storage.golfer_store import GolferStore


def _report(corpus: CareerCorpus, display_name: str, *, verbose: bool) -> None:
    print(f"\n{display_name}  ({corpus.player_id})")
    print("-" * 72)
    print(
        f"  {corpus.swing_dirs_seen} swing director"
        f"{'y' if corpus.swing_dirs_seen == 1 else 'ies'} across "
        f"{corpus.sessions_scanned} session{'' if corpus.sessions_scanned == 1 else 's'}"
        f"  ->  {corpus.distinct_swings} distinct swing"
        f"{'' if corpus.distinct_swings == 1 else 's'}, {corpus.distinct_shots} distinct shot"
        f"{'' if corpus.distinct_shots == 1 else 's'}"
    )

    if corpus.duplicates_collapsed:
        print(f"\n  Collapsed {corpus.duplicates_collapsed} re-upload(s):")
        for swing in corpus.swings:
            if swing.duplicates:
                print(
                    f"    face_on {swing.face_on_sha256[:12]}  kept {swing.ref}"
                    f"  <- {', '.join(swing.duplicates)}"
                )

    if corpus.shot_conflicts:
        print(f"\n  {corpus.shot_conflicts} swing(s) whose re-uploads disagree on the shot photo:")
        for swing in corpus.swings:
            if swing.conflicting_shots:
                others = ", ".join(sha[:12] for sha in swing.conflicting_shots)
                kept = (swing.shot_sha256 or "none")[:12]
                print(f"    {swing.ref}  kept {kept}, also saw {others}")
        print("    One swing has one ball flight, so one of these is misattached. Not counted.")

    print("\n  Honest n per metric:")
    if corpus.metric_counts:
        width = max(len(name) for name in corpus.metric_counts)
        for name, count in corpus.metric_counts.items():
            print(f"    {name:<{width}}  n = {count}")
    else:
        print("    (none — no distinct swing carries a measurement yet)")

    if corpus.outdated_swings:
        versions = sorted({s.analysis_version for s in corpus.swings if s.outdated})
        seen = ", ".join(str(v) for v in versions)
        print(
            f"\n  {corpus.outdated_swings} distinct swing(s) analyzed by an older engine "
            f"(version {seen}; {ANALYSIS_VERSION} is installed)."
        )
        print("    Their numbers are not comparable with a swing analyzed today, so they are")
        print("    reported rather than pooled. Repair with: python scripts/reanalyze.py")

    if corpus.analyzed_without_measurements:
        print(
            f"\n  {corpus.analyzed_without_measurements} current-engine swing(s) carrying no "
            "measurement at all."
        )
        print("    Every metric returned None on them — a measurement failure, not an old")
        print("    artifact. Their checkpoint `observed` values are deliberately not read as")
        print("    measurements: it would mix two derivation paths under one metric name.")

    if corpus.untagged_swings:
        print(
            f"\n  {corpus.untagged_swings} distinct swing(s) naming no club "
            f"(of {corpus.distinct_swings})."
        )
        print("    They still count toward every metric above and toward every whole-bag")
        print("    number — the club was never an input to measuring head sway — and are")
        print("    absent only from per-club views. Repair them one at a time on the upload")
        print("    page: a session has many clubs, so there is no bulk backfill (ADR-024 §5).")

    if corpus.unknown_sources:
        print(f"\n  Unrecognised measurement sources: {', '.join(corpus.unknown_sources)}")
        print("    Counted per swing rather than per artifact — check the dedupe key still fits.")

    if corpus.excluded:
        print(f"\n  Contributing no sample ({len(corpus.excluded)}):")
        by_reason: dict[str, list[str]] = {}
        for entry in corpus.excluded:
            by_reason.setdefault(entry.reason.value, []).append(entry.ref)
        for reason, refs in sorted(by_reason.items()):
            print(f"    {reason:<14} {len(refs):>3}  {', '.join(refs)}")
        if verbose:
            print()
            for entry in corpus.excluded:
                print(f"    {entry.ref}: {entry.reason.value} — {entry.detail}")

    if corpus.other_golfers:
        print(f"\n  ({corpus.other_golfers} swing(s) on disk belong to someone else.)")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--name", help="Golfer's name as typed, e.g. 'Aaron'.")
    parser.add_argument("--player-id", help="Stored id, e.g. 'aaron'.")
    parser.add_argument(
        "--verbose", action="store_true", help="Print the reason for each excluded swing."
    )
    args = parser.parse_args(argv)

    golfers = GolferStore(settings.golfers_dir)

    if args.name or args.player_id:
        player_id = args.player_id or slugify(args.name)
        if not player_id:
            print(f"'{args.name}' has no usable characters for an id.")
            return 2
        golfer = golfers.get(player_id)
        # An unregistered id is still worth reading: swings can name a golfer whose registry
        # record was deleted, and a corpus of zero says that out loud rather than erroring.
        targets = [(player_id, golfer.display_name if golfer else f"{player_id} (not registered)")]
    else:
        targets = [(g.player_id, g.display_name) for g in golfers.list_all()]
        if not targets:
            print(f"No golfers registered in {settings.golfers_dir}.")
            return 0

    for player_id, display_name in targets:
        _report(
            read_corpus(settings.sessions_dir, player_id), display_name, verbose=args.verbose
        )
    print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
