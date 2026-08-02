"""Fetch the GolfDB annotation database into the local reference cache. [M4-REF]

Usage:
    python scripts/golfdb/fetch.py

Downloads *only* the annotation files (`golfDB.pkl`, `golfDB.mat` — together under a megabyte)
from the upstream repo into `data/reference/golfdb/upstream/`, which is gitignored. Videos are
not fetched: they are ~2 GB and come from a separate Google Drive archive (see ADR-012 §Sourcing).

**We deliberately do not vendor any of this into the repo.** GolfDB's code is CC BY-NC 4.0, the
dataset carries no stated license, and the underlying clips are third-party broadcast footage.
Only *aggregate* statistics derived from these annotations are committed, under
`src/golf_coach/analysis/benchmarks/`. See ADR-012 for the full licensing posture.

Idempotent: files already present are left alone. Pass `--force` to re-download.
"""

from __future__ import annotations

import sys
import urllib.error
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
UPSTREAM_DIR = REPO_ROOT / "data" / "reference" / "golfdb" / "upstream"

_RAW_BASE = "https://raw.githubusercontent.com/wmcnally/golfdb/master/data"

# The pickle is a pre-built pandas DataFrame and is what upstream's own tooling reads. The .mat is
# the same annotations in MATLAB form and is the documented fallback: the pickle was written with
# a long-obsolete pandas and does not always load on modern versions (upstream issue #14).
FILES = {
    "golfDB.pkl": f"{_RAW_BASE}/golfDB.pkl",
    "golfDB.mat": f"{_RAW_BASE}/golfDB.mat",
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

    print(f"\nAnnotations cached under {UPSTREAM_DIR}")
    print("Reminder: this directory is gitignored and must stay that way (ADR-012).")
    return 0


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print("usage: python scripts/golfdb/fetch.py [--force]", file=sys.stderr)
        return 2
    return fetch(force="--force" in argv)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
