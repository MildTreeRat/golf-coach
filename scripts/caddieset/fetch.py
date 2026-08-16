"""Fetch the CaddieSet dataset into the local reference cache. [M8-PAIR]

Usage:
    python scripts/caddieset/fetch.py [--force]

Downloads `CaddieSet.csv` (~500 KB) and the upstream LICENSE from
<https://github.com/damilab/CaddieSet> into `data/reference/caddieset/upstream/`.

**Why this corpus exists here at all.** GolfDB gives us a tour population with no ball flight —
ADR-012 says it plainly: no handicap, no shot outcome, no good/bad, and no gradation even within
the pro pool. CaddieSet is the other half: 1,757 shots from eight golfers of *mixed* skill, each
row carrying per-phase joint metrics **and** the launch monitor's reading of where the ball went.
It is the only public dataset we found with both sides, which makes it the only thing on hand that
can answer "does this checkpoint predict anything" rather than "is this checkpoint unusual".

**It is MIT-licensed, so unlike GolfDB we *could* vendor it.** We don't, deliberately: keeping
every third-party corpus under one gitignored root means the licensing boundary is a directory
rather than a per-file judgement someone has to remember. The file is one HTTP GET away and this
script is idempotent, so reproducibility loses nothing. What gets committed is the same thing that
gets committed for GolfDB — aggregates under `src/golf_coach/analysis/benchmarks/`.

Idempotent: files already present are left alone. Pass `--force` to re-download.
"""

from __future__ import annotations

import sys
import urllib.error
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
UPSTREAM_DIR = REPO_ROOT / "data" / "reference" / "caddieset" / "upstream"

_RAW_BASE = "https://raw.githubusercontent.com/damilab/CaddieSet/main"

# The LICENSE travels with the data on purpose. ADR-012 §2 made every committed artifact state its
# terms inline so they cannot drift away from the thing they govern; the same reasoning applies to
# a cached corpus, where the alternative is a licence known only to whoever ran the download.
FILES = {
    "CaddieSet.csv": f"{_RAW_BASE}/data/CaddieSet.csv",
    "LICENSE": f"{_RAW_BASE}/LICENSE",
}


def fetch(force: bool = False) -> int:
    UPSTREAM_DIR.mkdir(parents=True, exist_ok=True)
    failures = 0

    for name, url in FILES.items():
        target = UPSTREAM_DIR / name
        if target.exists() and not force:
            print(f"  {name}: already present ({target.stat().st_size:,} bytes)")
            continue
        try:
            urllib.request.urlretrieve(url, target)  # trusted https, public repo
        except (urllib.error.URLError, OSError) as exc:
            print(f"  {name}: FAILED — {exc}", file=sys.stderr)
            failures += 1
            continue
        print(f"  {name}: downloaded ({target.stat().st_size:,} bytes)")

    if failures:
        print(f"\n{failures} file(s) failed to download.", file=sys.stderr)
        return 1

    print(f"\nCaddieSet cached under {UPSTREAM_DIR}")
    print("Next: python scripts/caddieset/ingest.py")
    return 0


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print("usage: python scripts/caddieset/fetch.py [--force]", file=sys.stderr)
        return 2
    return fetch(force="--force" in argv)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
