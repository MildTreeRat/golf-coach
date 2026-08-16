"""Shared CaddieSet plumbing for the paired mechanics/outcome scripts. [M8-PAIR]

**The CSV boundary.** `load_rows()` is the only place that reads CaddieSet's raw CSV; it returns
plain dicts and every script downstream is stdlib. Same shape of decision as
`scripts/golfdb/common.py::load_database` — one place knows the upstream format, so a second
paired corpus later only has to produce the same dicts.

Imported by its siblings as `import common` — running `python scripts/caddieset/<script>.py` puts
this directory on `sys.path`, the same convention `scripts/golfdb/` already relies on.

### What this corpus is, in one paragraph

1,757 rows, 924 of them face-on, from **eight** golfers of mixed skill hitting into a camera-based
launch monitor. Each row is one *view* of one shot: per-phase joint metrics named
`{phase}-{METRIC}` over the same eight swing events GolfDB annotates, plus the ball's flight. The
joint metrics are CaddieSet's own definitions, computed by CaddieSet's own pipeline — **they are
not our `metric_definitions_version: 3` measurements and their values may never become bands
here** (ADR-012 §4 is the governing rule: a band is only comparable to a swing measured the same
way). What transfers is which features carry signal and in which direction.
"""

from __future__ import annotations

import csv
import hashlib
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
REFERENCE_DIR = REPO_ROOT / "data" / "reference" / "caddieset"
UPSTREAM_CSV = REFERENCE_DIR / "upstream" / "CaddieSet.csv"
SHOTS_JSONL = REFERENCE_DIR / "shots.jsonl"

SOURCE_NAME = "caddieset"

# Normalised to GolfDB's vocabulary (`scripts/golfdb/common.py::VIEW_FACE_ON`) so the two corpora
# can be filtered by the same string. CaddieSet has no equivalent of GolfDB's "other" bucket.
VIEW_FACE_ON = "face-on"
VIEW_DOWN_THE_LINE = "down-the-line"
_VIEWS = {"FACEON": VIEW_FACE_ON, "DTL": VIEW_DOWN_THE_LINE}

# GolfDB's club vocabulary, so a future stratum can span both corpora. CaddieSet ships only
# driver, fairway wood and irons — no hybrid, no wedge — which is worth knowing before anyone
# expects a per-club band out of it.
_CLUB_CATEGORIES = {"W1": "driver", "W3": "fairway"} | {f"I{n}": "iron" for n in range(1, 10)}

# The launch monitor's columns, verbatim. Units are metric and undeclared upstream: `Carry` and
# `Distance` are metres (the driver mean is 169, which is 185 yards — an amateur, not a tour
# player, and that gradation is the entire reason this corpus is here), `BallSpeed` is m/s,
# the two spin columns are rpm and the two angle columns are degrees.
OUTCOME_FIELDS = (
    "Distance",
    "Carry",
    "LrDistanceOut",
    "DirectionAngle",
    "SpinBack",
    "SpinSide",
    "SpinAxis",
    "BallSpeed",
)

# The eight swing events, in the order CaddieSet's `{phase}-{METRIC}` prefixes index them. Same
# sequence GolfDB annotates, minus GolfDB's clip-bound `start`/`end` — so phase 3 is the top of
# the backswing and phase 5 is impact, which is what makes the two corpora talk to each other.
PHASE_NAMES: tuple[str, ...] = (
    "address",
    "toe_up",
    "mid_backswing",
    "top",
    "mid_downswing",
    "impact",
    "mid_follow_through",
    "finish",
)


def load_rows() -> list[dict[str, str]]:
    """Read the raw CSV into plain dicts. The only `csv` call we make.

    `utf-8-sig`, not `utf-8`: the file carries a BOM, which would otherwise ride along on the
    first column name and make `row["View"]` a `KeyError` that reads like a schema change.
    """
    if not UPSTREAM_CSV.exists():
        raise FileNotFoundError(
            f"{UPSTREAM_CSV} not found — run `python scripts/caddieset/fetch.py` first."
        )
    with UPSTREAM_CSV.open(encoding="utf-8-sig", newline="") as handle:
        return list(csv.DictReader(handle))


def joint_columns(row: dict[str, str]) -> list[str]:
    """The `{phase}-{METRIC}` columns, in file order. Everything else is metadata or outcome."""
    return [c for c in row if c[:1].isdigit()]


def view_of(row: dict[str, str]) -> str:
    return _VIEWS[row["View"]]


def club_category(club_type: str) -> str:
    return _CLUB_CATEGORIES.get(club_type, "unknown")


def parse_cell(raw: str) -> float | None:
    """One joint cell to a float, or `None` when the cell cannot be trusted.

    Three ways a cell fails, all of them present in the shipped file and all of them silent if
    you let `float()` see them or coerce a blank to zero:

    - **empty** — the honest majority. A metric is only defined at the phases and the view where
      it means something, so 29 of the 69 joint columns are blank on every face-on row. Absent is
      not zero, and this repo has a rule about that (ADR-010 §2: no score beats a wrong one).
    - **`#NAME?`** — 11 cells. That is Excel's unknown-name error, so the file has been through a
      spreadsheet at some point in its life. It is a hole, not a value.
    - **non-finite** — 11 cells hold an infinity, from a division by a zero-length body segment
      when pose failed on that frame.

    Outliers are deliberately *not* handled here. Structural validity is an ingest question;
    "is 33 shoulder-widths of head movement plausible" is a modelling question with a threshold
    that depends on the column's units, and it belongs to the study that sets it. Same split
    `scripts/golfdb/` makes, where `_drop_implausible` lives in `derive_reference.py` rather than
    in `ingest_labels.py`.
    """
    raw = raw.strip()
    if not raw:
        return None
    try:
        value = float(raw)
    except ValueError:
        return None
    return value if math.isfinite(value) else None


def shot_key(row: dict[str, str]) -> str:
    """A stable id for the *physical shot*, so the two views of it can be recognised as one.

    CaddieSet ships one row per view, and 803 of the face-on rows have a down-the-line row with a
    byte-identical launch-monitor reading. Treating 1,757 rows as 1,757 shots would therefore
    double-count most of the corpus — the same mistake `CorpusSwing.artifact_key` exists to stop
    on our own data, and the same fix: key on the thing that was measured once.

    The key is the golfer, the club and all eight ball columns. Three face-on rows still collide
    under it, which is the expected rate for genuinely identical readings and is why this returns
    a key to group by rather than silently merging rows.
    """
    material = "|".join([row["GolferId"], row["ClubType"], *(row[f] for f in OUTCOME_FIELDS)])
    return hashlib.sha1(material.encode("utf-8")).hexdigest()[:16]


def to_record(row: dict[str, str], row_index: int) -> dict[str, Any]:
    """One CSV row to the JSONL shape written by `ingest.py`.

    Deliberately *not* a `contracts.reference.ReferenceSwing`. That type has no slot for a shot
    outcome, and widening a shape in `contracts/` for a corpus whose value is not yet proven is
    the expensive kind of speculative change (CLAUDE.md L3). Nothing in `src/` reads per-shot
    reference rows — only aggregates ship — so this shape stays inside `scripts/`.
    """
    columns = joint_columns(row)
    metrics: dict[str, float] = {}
    unreadable: list[str] = []
    for column in columns:
        value = parse_cell(row[column])
        if value is not None:
            metrics[column] = value
        elif row[column].strip():
            # Distinguished from blank on purpose: a blank column is this view not defining the
            # metric, an unreadable one is a cell that was supposed to hold a number.
            unreadable.append(column)

    return {
        "source": SOURCE_NAME,
        "row_index": row_index,
        "shot_key": shot_key(row),
        "golfer_id": row["GolferId"],
        "view": view_of(row),
        "club": row["ClubType"],
        "club_category": club_category(row["ClubType"]),
        "outcome": {f: float(row[f]) for f in OUTCOME_FIELDS},
        "metrics": metrics,
        "unreadable": unreadable,
    }


def load_shots() -> list[dict[str, Any]]:
    """Read `shots.jsonl` back. Mirrors `scripts/golfdb/common.py::load_swings`."""
    if not SHOTS_JSONL.exists():
        raise FileNotFoundError(
            f"{SHOTS_JSONL} not found — run `python scripts/caddieset/ingest.py` first."
        )
    with SHOTS_JSONL.open(encoding="utf-8") as handle:
        return [json.loads(line) for line in handle if line.strip()]


def face_on(shots: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [s for s in shots if s["view"] == VIEW_FACE_ON]
