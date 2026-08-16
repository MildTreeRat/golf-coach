"""CaddieSet CSV → one JSON row per view-of-a-shot, with a coverage report. [M8-PAIR]

Usage:
    python scripts/caddieset/ingest.py [--quiet]

Writes `data/reference/caddieset/shots.jsonl` (gitignored, like everything under
`data/reference/`) and prints what the corpus actually contains. The report is the point as much
as the file is: this CSV is sparse by design and dirty by accident, and both facts change what a
study on top of it is allowed to claim.

Nothing here drops a row. Structurally unreadable *cells* become absent (`common.parse_cell`), and
which cells those were is recorded per row in `unreadable` so a later script can decide rather
than guess. See ADR-021 for the corpus decision and its limits.
"""

from __future__ import annotations

import json
import sys
from collections import Counter, defaultdict
from typing import Any

import common


def build(rows: list[dict[str, str]]) -> list[dict[str, Any]]:
    return [common.to_record(row, index) for index, row in enumerate(rows)]


def write(records: list[dict[str, Any]]) -> None:
    common.SHOTS_JSONL.parent.mkdir(parents=True, exist_ok=True)
    with common.SHOTS_JSONL.open("w", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record, separators=(",", ":")) + "\n")


def report(records: list[dict[str, Any]], columns: list[str]) -> None:
    face = common.face_on(records)
    print(
        f"\nrows: {len(records)}  ({len(face)} face-on, "
        f"{len(records) - len(face)} down-the-line)"
    )

    by_view: dict[str, Counter[str]] = defaultdict(Counter)
    for record in records:
        by_view[record["golfer_id"]][record["view"]] += 1
    print(f"\ngolfers: {len(by_view)}")
    for golfer in sorted(by_view, key=int):
        counts = by_view[golfer]
        print(
            f"  golfer {golfer}: {counts[common.VIEW_FACE_ON]:4d} face-on, "
            f"{counts[common.VIEW_DOWN_THE_LINE]:4d} down-the-line"
        )

    clubs = Counter(f"{r['club']} ({r['club_category']})" for r in face)
    print("\nface-on shots by club:")
    for club, count in clubs.most_common():
        print(f"  {club:16s} {count:4d}")

    # The pairing. Two views of one shot share a launch-monitor reading, so the number of distinct
    # shot keys — not the row count — is the honest n for anything that treats a shot as a sample.
    face_keys = {r["shot_key"] for r in face}
    dtl_keys = {r["shot_key"] for r in records if r["view"] == common.VIEW_DOWN_THE_LINE}
    print(
        f"\ndistinct shot keys: {len(face_keys)} face-on, {len(dtl_keys)} down-the-line, "
        f"{len(face_keys & dtl_keys)} seen from both views"
    )
    collisions = len(face) - len(face_keys)
    print(f"  face-on rows sharing a key with another face-on row: {collisions}")

    print(f"\nface-on coverage, {len(columns)} joint columns total:")
    populated = []
    for column in columns:
        count = sum(1 for r in face if column in r["metrics"])
        if count:
            populated.append((column, count))
    for column, count in populated:
        print(f"  {column:26s} {count:4d}  ({100 * count / len(face):5.1f}%)")
    empty = [c for c in columns if all(c not in r["metrics"] for r in face)]
    print(f"\n  {len(populated)} populated, {len(empty)} empty on every face-on row:")
    print(f"  {', '.join(empty)}")

    unreadable = Counter(c for r in records for c in r["unreadable"])
    total_cells = sum(unreadable.values())
    print(f"\nunreadable cells: {total_cells} across {len(unreadable)} columns")
    for column, count in unreadable.most_common():
        print(f"  {column:26s} {count:3d}")

    complete = sum(1 for r in face if not r["unreadable"])
    print(f"\nface-on rows with no unreadable cell: {complete}/{len(face)}")


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print("usage: python scripts/caddieset/ingest.py [--quiet]", file=sys.stderr)
        return 2

    rows = common.load_rows()
    records = build(rows)
    write(records)
    print(f"wrote {len(records)} rows to {common.SHOTS_JSONL}")

    if "--quiet" not in argv:
        report(records, common.joint_columns(rows[0]))

    print("\nNext: python scripts/caddieset/study_panel.py")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
