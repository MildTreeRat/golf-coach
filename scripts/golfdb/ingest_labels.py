"""Derive label-only swing metrics from GolfDB annotations. [M4-REF Phase A2]

Usage:
    python scripts/golfdb/ingest_labels.py

Reads the GolfDB annotation database and writes one `ReferenceSwing` per clip to
`data/reference/golfdb/swings.jsonl` (Tier 2, gitignored).

**No video is touched.** Everything here comes from the eight hand-annotated event frames, so it
runs in a second and needs neither the ~2 GB clip archive nor a pose estimator. Pose-derived
metrics are merged into the same rows later by `extract_pose.py`.

Every metric is a **frame ratio**, which buys two things that matter:

- *fps-invariance*: GolfDB is roughly half slow-motion, and a ratio of frame counts is unchanged by
  the capture rate. Absolute durations would be meaningless across the corpus; ratios are not.
- *no model in the loop*: these are human labels. SwingNet's own weak spots (it detects Address and
  Finish at ~30%) are irrelevant because we never run it.
"""

from __future__ import annotations

import sys

import common

from golf_coach.contracts.reference import ReferenceSwing

# These definitions are what the derived benchmark bands mean. Changing one invalidates every band
# cut from it, so they are versioned alongside the checkpoint definitions in `mechanics.py`.
#
# v3 (M4-REF Phase B6): `head_sway_norm` and `finish_balance_norm` now read their address endpoint
# and their shoulder-width ruler from `_address_sample_bounds` — a short window ending at the
# address boundary — instead of averaging across the whole ADDRESS phase, which began at frame 0
# and so averaged over the golfer walking into shot. Both p90s moved (0.42 -> 0.43 sway,
# 0.28 -> 0.29 balance), which is small but is exactly the kind of drift ADR-012 §4 exists to stop
# going unrecorded.
METRIC_DEFINITIONS_VERSION = 3

# GolfDB's `bbox` is normalized to the *source* video, whose pixel dimensions we do not have —
# only the clips resized from it. Nearly all of the 580 sources are YouTube broadcast footage, so
# 16:9 is the overwhelmingly likely frame shape and the only assumption available. It affects one
# metric (`finish_balance_norm`, the sole one mixing x and y); `derive_pose_metrics.py` reports the
# corrected and uncorrected distributions side by side so the assumption's weight is visible rather
# than trusted.
_ASSUMED_SOURCE_ASPECT = 16.0 / 9.0


def _pixel_aspect(bbox: list[float]) -> float:
    """How much to scale normalized `y` so it shares units with normalized `x`.

    `videos_160` crops each golfer's bounding box and resizes it to a square, so a box that was
    taller than it was wide gets squashed differently in each axis. `bbox` is `(x, y, w, h)` in
    fractions of the source frame, so the crop's true pixel shape is `(w * W) x (h * H)` and the
    ratio we need is `(h * H) / (w * W)`.
    """
    _, _, width, height = bbox
    if width <= 0 or height <= 0:
        return 1.0
    return (height / width) / _ASSUMED_SOURCE_ASPECT


def _ratios(events: dict[str, int]) -> dict[str, float] | None:
    """Unitless timing metrics for one swing, or None if its labels can't support them."""
    address, top, impact = events["address"], events["top"], events["impact"]
    backswing = top - address
    downswing = impact - top
    if backswing <= 0 or downswing <= 0:
        return None

    metrics = {
        # The headline: what "Tour Tempo ~3:1" actually refers to.
        "tempo_ratio": backswing / downswing,
        # Where the intermediate instants fall *within* each half of the swing. These say
        # something tempo alone cannot — whether the backswing is evenly paced or rushed early.
        "toe_up_frac": (events["toe_up"] - address) / backswing,
        "mid_backswing_frac": (events["mid_backswing"] - address) / backswing,
        "mid_downswing_frac": (events["mid_downswing"] - top) / downswing,
    }

    finish = events["finish"] - impact
    if finish > 0:
        metrics["follow_through_ratio"] = finish / backswing
    return metrics


def _is_monotonic(events: dict[str, int]) -> bool:
    """Events must run in swing order; anything else is an annotation error, not a fast swing."""
    ordered = [events[name] for name in common.EVENT_NAMES]
    return all(a <= b for a, b in zip(ordered, ordered[1:], strict=False))


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print("usage: python scripts/golfdb/ingest_labels.py", file=sys.stderr)
        return 2

    records = common.load_database()
    print(f"Loaded {len(records)} GolfDB clips")

    swings: list[ReferenceSwing] = []
    skipped_disordered = 0
    skipped_degenerate = 0

    for record in records:
        events = common.event_map(record["events"])
        if not _is_monotonic(events):
            skipped_disordered += 1
            continue
        metrics = _ratios(events)
        if metrics is None:
            skipped_degenerate += 1
            continue

        swings.append(
            ReferenceSwing(
                source=common.SOURCE_NAME,
                source_id=record["id"],
                group_id=record["youtube_id"],
                subject=record["player"],
                sex=record["sex"],
                club=record["club"],
                view=record["view"],
                slow_motion=record["slow"],
                pixel_aspect=_pixel_aspect(record["bbox"]),
                events=events,
                metrics=metrics,
                pose_estimator=None,
                metric_definitions_version=METRIC_DEFINITIONS_VERSION,
            )
        )

    common.REFERENCE_DIR.mkdir(parents=True, exist_ok=True)
    with common.SWINGS_JSONL.open("w", encoding="utf-8") as handle:
        for swing in swings:
            handle.write(swing.model_dump_json() + "\n")

    print(f"Wrote {len(swings)} rows -> {common.SWINGS_JSONL}")
    if skipped_disordered:
        print(f"  skipped {skipped_disordered} clip(s) with out-of-order event labels")
    if skipped_degenerate:
        print(f"  skipped {skipped_degenerate} clip(s) with a zero-length backswing or downswing")

    players = {s.subject for s in swings}
    videos = {s.group_id for s in swings}
    face_on = sum(1 for s in swings if s.view == common.VIEW_FACE_ON)
    print(
        f"  {len(players)} distinct golfers, {len(videos)} source videos, "
        f"{face_on} face-on clips"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
