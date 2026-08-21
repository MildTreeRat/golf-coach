"""Shared GolfDB plumbing for the reference-data scripts. [M4-REF]

The **pandas boundary**. `load_database()` is the only place in this project that reads GolfDB's
pickled DataFrame; it returns plain dicts, and every script downstream is stdlib + pydantic. That
keeps the `research` extra off the analysis core (ADR-008) and means a future corpus only has to
produce the same dicts.

Imported by its siblings as `import common` — running `python scripts/golfdb/<script>.py` puts
this directory on `sys.path`, the same convention `tests/analysis/conftest.py` relies on.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
REFERENCE_DIR = REPO_ROOT / "data" / "reference" / "golfdb"
UPSTREAM_DIR = REFERENCE_DIR / "upstream"
KEYPOINTS_DIR = REFERENCE_DIR / "keypoints"
SWINGS_JSONL = REFERENCE_DIR / "swings.jsonl"

BENCHMARKS_DIR = REPO_ROOT / "src" / "golf_coach" / "analysis" / "benchmarks"

SOURCE_NAME = "golfdb"

# GolfDB stores ten frame indices per clip: the clip bounds plus the eight annotated events, in
# swing order. Index 0 is `start`, NOT `address` — conflating them is the classic mis-read of this
# dataset (upstream's own dataloader rebases with `events -= events[0]`), and it silently turns
# every tempo ratio into nonsense because it counts the pre-swing setup as backswing.
EVENT_NAMES: tuple[str, ...] = (
    "start",
    "address",
    "toe_up",
    "mid_backswing",
    "top",
    "mid_downswing",
    "impact",
    "mid_follow_through",
    "finish",
    "end",
)

VIEW_FACE_ON = "face-on"


def load_database() -> list[dict[str, Any]]:
    """Read the GolfDB annotation DataFrame into plain dicts. The only pandas call we make.

    Prefers the pre-built pickle. Upstream wrote it with a long-obsolete pandas and it does not
    always load on current versions (upstream issue #14), so a `.mat` fallback is documented in
    `fetch.py`; if the pickle ever stops loading, convert it there rather than teaching every
    downstream script about two formats.
    """
    import pandas as pd

    pickle_path = UPSTREAM_DIR / "golfDB.pkl"
    if not pickle_path.exists():
        raise FileNotFoundError(
            f"{pickle_path} not found — run `python scripts/golfdb/fetch.py` first."
        )

    frame = pd.read_pickle(pickle_path)
    records: list[dict[str, Any]] = []
    for _, row in frame.iterrows():
        records.append(
            {
                "id": str(int(row["id"])),
                "youtube_id": str(row["youtube_id"]),
                "player": str(row["player"]),
                "sex": str(row["sex"]),
                "club": str(row["club"]),
                "view": str(row["view"]),
                "slow": bool(int(row["slow"])),
                "events": [int(v) for v in row["events"]],
                "bbox": [float(v) for v in row["bbox"]],
            }
        )
    return records


def event_map(events: list[int]) -> dict[str, int]:
    """Zip GolfDB's ten frame indices onto their names."""
    if len(events) != len(EVENT_NAMES):
        raise ValueError(f"expected {len(EVENT_NAMES)} event frames, got {len(events)}")
    return dict(zip(EVENT_NAMES, events, strict=True))


def load_swings() -> list[Any]:
    """Read `swings.jsonl` (Tier 2) as `ReferenceSwing` rows."""
    from golf_coach.contracts.reference import ReferenceSwing

    if not SWINGS_JSONL.exists():
        raise FileNotFoundError(
            f"{SWINGS_JSONL} not found — run `python scripts/golfdb/ingest_labels.py` first."
        )
    with SWINGS_JSONL.open(encoding="utf-8") as handle:
        return [ReferenceSwing.model_validate_json(line) for line in handle if line.strip()]


def bakeoff_sample(swings: list[Any], count: int) -> list[Any]:
    """Face-on clips spread across as many distinct source videos as possible.

    GolfDB often cuts several clips from one broadcast, so taking the first N by id would
    over-weight a handful of videos and golfers. Round-robin across `group_id` maximizes
    independent samples. Deterministic, and **shared** by `extract_pose.py` and `bakeoff.py` so
    every estimator is extracted and scored on exactly the same clips — a comparison across
    different clip sets would measure the clips, not the estimators.
    """
    face_on = [s for s in swings if s.view == VIEW_FACE_ON]
    groups: dict[str, list[Any]] = {}
    for swing in sorted(face_on, key=lambda s: (s.group_id or "", int(s.source_id))):
        groups.setdefault(swing.group_id or "", []).append(swing)

    chosen: list[Any] = []
    depth = 0
    while len(chosen) < count:
        added = False
        for key in sorted(groups):
            if depth < len(groups[key]):
                chosen.append(groups[key][depth])
                added = True
                if len(chosen) == count:
                    break
        if not added:
            break
        depth += 1
    return chosen


def clip_path(videos_dir: Path, source_id: str) -> Path:
    """Path to a clip in the `videos_160` archive, which names files `{id}.mp4`."""
    return videos_dir / f"{source_id}.mp4"


def keypoints_path(source_id: str, estimator_slug: str) -> Path:
    """Where this clip's extracted keypoints live (Tier 1 cache, gitignored).

    The cache is partitioned **by estimator** — `extract_pose.py` writes under
    `KEYPOINTS_DIR / estimator_slug(name)` so the bake-off can hold four estimators' output for
    the same clip at once. This helper used to omit that level and return a path that has never
    existed; it had no callers, so nothing failed and the bug sat here looking like the way to do
    it. Pass `extract_pose.estimator_slug(name)`, or one of the slugs already on disk.
    """
    return KEYPOINTS_DIR / estimator_slug / f"{source_id}.keypoints.json"
