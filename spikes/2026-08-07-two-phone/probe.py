"""M7 Phase 0 field-spike probe — THROWAWAY. Not part of the `golf_coach` package.

Answers the three Phase 0 questions against real iPhone footage (see
docs/M7_TWO_PHONE_SPIKE.md):

1. Does `segment_phases()` find sane top/impact on a **down-the-line** clip? It tracks
   `LEFT_WRIST` y and was tuned on 461 **face-on** GolfDB clips.
2. Can OpenCV decode iPhone HEVC `.mov`?
3. What does `CAP_PROP_FPS` report, normal vs slo-mo?

**This file reaches into `analysis/phases.py` privates on purpose.** Measuring the *shipped*
rule is the whole point — a re-implementation here could pass while the real detector fails.
Reaching through the seam is acceptable precisely because this code is discarded once the
findings land in the doc (spikes/README.md: fold in the findings, not the code).

Subcommands::

    probe.py inspect  <video>...                     # Q2 + Q3: container, codec, fps, decode
    probe.py frames   <video> --out <dir> [--every N] # index-burned PNGs for hand-labelling
    probe.py pose     <video> --out <kp.json>        # STREAMING pose (no run_pose.py OOM)
    probe.py wrist    <kp.json>                      # Q1 mechanism: visibility + continuity
    probe.py variants <kp.json>                      # Q1 remedy: 5 candidate signals
    probe.py score    <truth.json>                   # error tables + per-question verdict

`inspect`, `frames` and `pose` need the `vision` extra; `wrist`, `variants` and `score` run on
the base install.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
from pathlib import Path

from pydantic import TypeAdapter

from golf_coach.analysis.phases import (
    _MIN_VISIBILITY,
    _top_and_impact,
    segment_phases,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import SwingPhase

_KEYPOINTS_ADAPTER = TypeAdapter(list[FrameKeypoints])

# A "teleport" is MediaPipe swapping left for right — a real risk from behind the golfer, where
# the model has far less training data than it does face-on. Its signature is a jump that
# **snaps back**: the landmark leaps across the body and returns within a few frames. A merely
# *large* step is not evidence of anything, because the hands genuinely travel a sixth of the
# frame between two 60fps frames through impact — an early version of this that flagged any step
# above 5x the clip median reported 158 teleports on a clean face-on clip, all of them real
# motion. The median is set by the long quiet stretch at address, so it is the wrong ruler.
_TELEPORT_STEP_FLOOR = 0.08  # normalized units; below this nothing is worth calling a jump
_TELEPORT_P95_FACTOR = 3.0
_TELEPORT_RETURN_FRAMES = 3
_TELEPORT_RETURN_FRACTION = 0.5

# Below this mean intensity a frame is treated as black — the signature of a decoder that
# opened the file, reported frames, and handed back nothing usable (the silent Q2 failure).
_BLACK_THRESHOLD = 1.0


# --------------------------------------------------------------------------------------
# Q1 — candidate wrist signals
#
# Each returns `(ys, confident)` in exactly the shape `_top_and_impact` consumes, so every
# variant is judged by the shipped rule with only the *signal* swapped. The pattern is
# scripts/golfdb/tune_address.py's: keep the losers runnable so the evidence outlives the
# decision.
# --------------------------------------------------------------------------------------


def _series(
    keypoints: list[FrameKeypoints], indices: tuple[PoseLandmark, ...]
) -> tuple[list[float], list[bool]]:
    """Mean `y` over `indices`, holding the last confident value through dim frames.

    Mirrors `_lead_wrist_xy` / `_wrist_confident` but over a landmark *set*, so one helper
    covers the single-landmark and averaged variants alike.
    """
    ys: list[float] = []
    confident: list[bool] = []
    last_good: float | None = None
    for frame in keypoints:
        marks = [frame.landmark(i) for i in indices]
        ok = all(m.visibility >= _MIN_VISIBILITY for m in marks)
        if ok or last_good is None:
            last_good = sum(m.y for m in marks) / len(marks)
        ys.append(last_good)
        confident.append(ok)
    return ys, confident


def _best_visible_wrist(keypoints: list[FrameKeypoints]) -> tuple[list[float], list[bool]]:
    """Per frame, whichever wrist MediaPipe is more confident about."""
    ys: list[float] = []
    confident: list[bool] = []
    last_good: float | None = None
    for frame in keypoints:
        left = frame.landmark(PoseLandmark.LEFT_WRIST)
        right = frame.landmark(PoseLandmark.RIGHT_WRIST)
        best = left if left.visibility >= right.visibility else right
        ok = best.visibility >= _MIN_VISIBILITY
        if ok or last_good is None:
            last_good = best.y
        ys.append(last_good)
        confident.append(ok)
    return ys, confident


# Name -> signal builder. `lead_wrist` reproduces the shipped detector and is the baseline
# every other variant has to beat before it is worth a line in the doc.
VARIANTS = {
    "lead_wrist": lambda kp: _series(kp, (PoseLandmark.LEFT_WRIST,)),
    "trail_wrist": lambda kp: _series(kp, (PoseLandmark.RIGHT_WRIST,)),
    "wrist_mean": lambda kp: _series(kp, (PoseLandmark.LEFT_WRIST, PoseLandmark.RIGHT_WRIST)),
    "hand_mid": lambda kp: _series(kp, (PoseLandmark.LEFT_INDEX, PoseLandmark.RIGHT_INDEX)),
    "best_visible": _best_visible_wrist,
}


def _predict(keypoints: list[FrameKeypoints], variant: str) -> tuple[int, int]:
    """`(top, impact)` for one variant. Smooths first, exactly as `engine.analyze_swing` does."""
    smoothed = smooth_keypoints(keypoints)
    ys, confident = VARIANTS[variant](smoothed)
    return _top_and_impact(ys, confident, len(smoothed))


def _shipped_instants(keypoints: list[FrameKeypoints]) -> dict[str, int]:
    """What the shipped pipeline actually reports, read off the phase boundaries.

    Same derivation as `scripts/analyze_swing.py:_instants`, so the pre-flight check can
    assert the probe and the CLI agree.
    """
    phases = segment_phases(smooth_keypoints(keypoints))
    out: dict[str, int] = {}
    for segment in phases:
        if segment.phase is SwingPhase.ADDRESS:
            out["motion_start"] = segment.end_frame
            out["motion_start_detected"] = int(segment.detected)
        elif segment.phase is SwingPhase.TRANSITION:
            out["top"] = (segment.start_frame + segment.end_frame) // 2
        elif segment.phase is SwingPhase.IMPACT:
            out["impact"] = segment.start_frame
    return out


# --------------------------------------------------------------------------------------
# Subcommands
# --------------------------------------------------------------------------------------


def cmd_inspect(args: argparse.Namespace) -> int:
    """Q2 + Q3: what the container claims, versus what actually decodes."""
    import cv2

    rows = []
    for path in _expand(args.videos):
        cap = cv2.VideoCapture(str(path))
        if not cap.isOpened():
            rows.append({"file": path.name, "opened": False})
            print(f"!! OpenCV could NOT open {path}", file=sys.stderr)
            continue

        # Read this before the decode loop — `getBackendName()` asserts on a released capture.
        backend = cap.getBackendName()
        fourcc_raw = int(cap.get(cv2.CAP_PROP_FOURCC))
        fourcc = "".join(chr((fourcc_raw >> (8 * i)) & 0xFF) for i in range(4)).strip()
        reported_fps = cap.get(cv2.CAP_PROP_FPS)
        reported_count = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
        width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
        height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))

        # Decode the whole file. The dangerous Q2 outcome is not "won't open" — it is a
        # decoder that opens, reports a frame count, and quietly stops a third of the way in.
        started = time.perf_counter()
        decoded = black = 0
        while True:
            ok, image = cap.read()
            if not ok:
                break
            decoded += 1
            if image.mean() < _BLACK_THRESHOLD:
                black += 1
        elapsed = time.perf_counter() - started
        cap.release()

        rows.append(
            {
                "file": path.name,
                "opened": True,
                "backend": backend,
                "fourcc": fourcc,
                "reported_fps": round(reported_fps, 3),
                "reported_frames": reported_count,
                "decoded_frames": decoded,
                "short_by": reported_count - decoded,
                "size": f"{width}x{height}",
                "mb": round(path.stat().st_size / 1024**2, 1),
                "container_seconds": round(decoded / reported_fps, 2) if reported_fps else None,
                "black_frames": black,
                "decode_seconds": round(elapsed, 1),
                "decode_fps": round(decoded / elapsed, 1) if elapsed else None,
            }
        )

    _print_table(rows)
    _emit_json(args.json, rows)

    # The precondition for Q2 meaning anything at all.
    codecs = {r.get("fourcc") for r in rows if r.get("opened")}
    if not codecs & {"hvc1", "hev1"}:
        print(
            "\nNOTE: no clip reports hvc1/hev1 — HEVC was never actually tested. Something "
            "transcoded (phone Format setting, Photos export, or the upload path). Probe the "
            "cable-copied control clip to find out which.",
            file=sys.stderr,
        )
    return 0


def cmd_frames(args: argparse.Namespace) -> int:
    """Dump frames with the index burned in, for hand-labelling top and impact."""
    import cv2

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    cap = cv2.VideoCapture(str(args.video))
    if not cap.isOpened():
        print(f"error: could not open {args.video}", file=sys.stderr)
        return 1

    index = written = 0
    while True:
        ok, image = cap.read()
        if not ok:
            break
        if index % args.every == 0 and args.start <= index <= (args.end or 1 << 30):
            cv2.putText(
                image, str(index), (20, 60),
                cv2.FONT_HERSHEY_SIMPLEX, 2.0, (0, 0, 255), 4, cv2.LINE_AA,
            )
            cv2.imwrite(str(out_dir / f"{index:05d}.png"), image)
            written += 1
        index += 1
    cap.release()
    print(f"Wrote {written} frames (of {index}) -> {out_dir}")
    return 0


def cmd_pose(args: argparse.Namespace) -> int:
    """Streaming pose pass — the spike's working path.

    `scripts/run_pose.py:40` materializes every decoded BGR frame (a 1080p60 8s clip is
    ~2.9 GB; 4K60 is ~15 GB). Fixing that is M7 Phase 1's commit, not the spike's, so the
    spike simply does not depend on it: `estimate_pose` takes an iterable, so the generator
    goes straight in and no frame outlives its iteration.
    """
    from golf_coach.capture.file import FileVideoSource
    from golf_coach.pose.estimator import estimate_pose

    started = time.perf_counter()
    with FileVideoSource(args.video) as source:
        fps = source.fps
        keypoints = estimate_pose(source.frames())

    if not keypoints:
        print(f"error: no frames read from {args.video}", file=sys.stderr)
        return 1

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(_KEYPOINTS_ADAPTER.dump_json(keypoints, indent=2))

    found = sum(
        1 for f in keypoints if any(lm.visibility > 0.0 for lm in f.landmarks)
    )
    elapsed = time.perf_counter() - started
    print(
        f"{len(keypoints)} frames @ {fps:g}fps reported | body found in {found} "
        f"({found / len(keypoints):.1%}) | {elapsed:.1f}s -> {out}"
    )
    return 0


def cmd_wrist(args: argparse.Namespace) -> int:
    """Q1 mechanism: is the lead wrist actually tracked through the swing, or hallucinated?

    A verdict without this is unattributable. If down-the-line errors are large *and*
    visibility collapses through the top, the cause is occlusion and a different landmark can
    fix it. If errors are large while visibility stays high, the signal itself is wrong from
    that angle and no landmark swap will help.
    """
    keypoints = _load(args.keypoints)
    smoothed = smooth_keypoints(keypoints)
    instants = _shipped_instants(keypoints)
    top, impact = instants.get("top", 0), instants.get("impact", len(smoothed) - 1)

    rows = []
    for name, landmark in (
        ("lead_wrist (LEFT)", PoseLandmark.LEFT_WRIST),
        ("trail_wrist (RIGHT)", PoseLandmark.RIGHT_WRIST),
    ):
        vis = [f.landmark(landmark).visibility for f in smoothed]
        window = vis[max(0, top - 10) : impact + 1] or vis
        track = [(f.landmark(landmark).x, f.landmark(landmark).y) for f in smoothed]
        steps = [_distance(a, b) for a, b in zip(track, track[1:], strict=False)]
        p95 = _percentile(steps, 0.95)
        dim = sum(1 for v in window if v < _MIN_VISIBILITY) / len(window)
        rows.append(
            {
                "landmark": name,
                "vis_mean_all": round(statistics.fmean(vis), 3),
                "vis_mean_top_to_impact": round(statistics.fmean(window), 3),
                "vis_min_top_to_impact": round(min(window), 3),
                "pct_below_gate": f"{dim:.1%}",
                "p95_step": round(p95, 4),
                "teleports": _count_teleports(track, steps, p95),
            }
        )

    print(f"\n{Path(args.keypoints).name}  frames={len(smoothed)}  "
          f"top={top} impact={impact}  (gate={_MIN_VISIBILITY})")
    _print_table(rows)
    _emit_json(args.json, rows)
    return 0


def cmd_variants(args: argparse.Namespace) -> int:
    """Q1 remedy: run every candidate signal through the shipped `_top_and_impact`."""
    keypoints = _load(args.keypoints)
    shipped = _shipped_instants(keypoints)

    rows = []
    for name in VARIANTS:
        top, impact = _predict(keypoints, name)
        rows.append({"variant": name, "top": top, "impact": impact, "downswing": impact - top})

    print(f"\n{Path(args.keypoints).name}  frames={len(keypoints)}")
    print(f"shipped pipeline: {shipped}")
    _print_table(rows)

    # Pre-flight assertion: the baseline variant must reproduce the shipped answer, or every
    # number this probe produces later is worthless.
    baseline = next(r for r in rows if r["variant"] == "lead_wrist")
    if "top" in shipped and abs(baseline["top"] - shipped["top"]) > 3:
        print(
            f"!! lead_wrist top {baseline['top']} disagrees with the shipped pipeline "
            f"{shipped['top']} by more than the TRANSITION half-window. The probe is wrong.",
            file=sys.stderr,
        )
        return 1
    _emit_json(args.json, rows)
    return 0


def cmd_score(args: argparse.Namespace) -> int:
    """Join predictions to hand labels and print the error tables + verdicts.

    Error is normalized against the clip's **own** downswing, never raw frames: a 60fps clip
    and a 240fps clip do not share a unit. This is ADR-013's clip-relative principle applied
    to the validation of the thing ADR-013 governs.
    """
    truth = json.loads(Path(args.truth).read_text(encoding="utf-8"))
    root = Path(args.truth).parent

    rows = []
    for clip in truth["clips"]:
        kp_path = Path(clip["keypoints"])
        if not kp_path.is_absolute():
            kp_path = root / kp_path
        if not kp_path.exists():
            print(f"skip {clip['name']}: {kp_path} missing", file=sys.stderr)
            continue

        keypoints = _load(kp_path)
        span = clip["truth_impact"] - clip["truth_top"]
        if span <= 0:
            print(f"skip {clip['name']}: truth_impact must exceed truth_top", file=sys.stderr)
            continue

        for name in VARIANTS:
            top, impact = _predict(keypoints, name)
            rows.append(
                {
                    "clip": clip["name"],
                    "angle": clip["angle"],
                    "variant": name,
                    "e_top": round(abs(top - clip["truth_top"]) / span, 3),
                    "e_impact": round(abs(impact - clip["truth_impact"]) / span, 3),
                    "err_top_frames": top - clip["truth_top"],
                    "err_impact_frames": impact - clip["truth_impact"],
                    "label_uncertainty": clip.get("label_uncertainty_frames"),
                }
            )

    if not rows:
        print("no clips scored", file=sys.stderr)
        return 1

    _print_table(rows)
    _emit_json(args.json, rows)
    print()
    _print_verdicts(rows)
    return 0


def _print_verdicts(rows: list[dict]) -> None:
    """Per-angle, per-variant medians and the Q1 verdict (thresholds from the spike plan).

    GREEN  median e_top <= 0.20, median e_impact <= 0.15, no clip above 0.50,
           and DTL median <= 2x the face-on median on the same swings.
    AMBER  nothing above e = 1.0, but a GREEN threshold missed.
    RED    any clip above e = 1.0 — the instant landed outside the true downswing entirely,
           the "found the finish" failure `_top_and_impact` exists to prevent.
    """
    summary: dict[tuple[str, str], dict] = {}
    for (angle, variant) in {(r["angle"], r["variant"]) for r in rows}:
        subset = [r for r in rows if r["angle"] == angle and r["variant"] == variant]
        summary[(angle, variant)] = {
            "angle": angle,
            "variant": variant,
            "n": len(subset),
            "med_e_top": round(statistics.median(r["e_top"] for r in subset), 3),
            "med_e_impact": round(statistics.median(r["e_impact"] for r in subset), 3),
            "max_e": round(max(max(r["e_top"], r["e_impact"]) for r in subset), 3),
        }

    ordered = sorted(summary.values(), key=lambda s: (s["angle"], s["variant"]))
    _print_table(ordered)

    if not any(s["angle"] == "down_the_line" for s in ordered):
        print("\n  (no down_the_line clips labelled — Q1 has nothing to rule on yet)")
        return

    for stats in ordered:
        if stats["angle"] != "down_the_line":
            continue
        face = summary.get(("face_on", stats["variant"]))
        relative_ok = face is None or (
            stats["med_e_top"] <= 2 * max(face["med_e_top"], 1e-9)
            and stats["med_e_impact"] <= 2 * max(face["med_e_impact"], 1e-9)
        )
        if stats["max_e"] > 1.0:
            verdict = "RED   (an instant landed outside the true downswing)"
        elif (
            stats["med_e_top"] <= 0.20
            and stats["med_e_impact"] <= 0.15
            and stats["max_e"] <= 0.50
            and relative_ok
        ):
            verdict = "GREEN"
        else:
            verdict = "AMBER (bounded, but a GREEN threshold missed)"
        print(f"  down_the_line / {stats['variant']:<13} n={stats['n']:<3} {verdict}")

    print(
        "\n  Verdicts cover Q1 only, and only the thresholds — check them against each clip's "
        "label_uncertainty before quoting a margin near the noise floor."
    )


# --------------------------------------------------------------------------------------
# Plumbing
# --------------------------------------------------------------------------------------


def _distance(a: tuple[float, float], b: tuple[float, float]) -> float:
    return ((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) ** 0.5


def _percentile(values: list[float], q: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(q * len(ordered)))]


def _count_teleports(
    track: list[tuple[float, float]], steps: list[float], p95: float
) -> int:
    """Jumps that snap back — the left/right-swap signature, not merely fast motion.

    A step counts only if it is both large in absolute terms and large for this clip, *and*
    the landmark returns near where it started within a few frames. Genuine hand speed
    through impact produces large steps that keep going; a swap produces an out-and-back.
    """
    bar = max(_TELEPORT_STEP_FLOOR, _TELEPORT_P95_FACTOR * p95)
    count = 0
    for i, step in enumerate(steps, start=1):
        if step < bar:
            continue
        origin = track[i - 1]
        horizon = min(len(track), i + _TELEPORT_RETURN_FRAMES + 1)
        if any(
            _distance(origin, track[j]) < _TELEPORT_RETURN_FRACTION * step
            for j in range(i + 1, horizon)
        ):
            count += 1
    return count


def _load(path: str | Path) -> list[FrameKeypoints]:
    # Takes both the clip-metadata envelope run_pose.py writes since M7 Phase 1 and the bare
    # arrays that predate it — including this probe's own `pose` output.
    from golf_coach.storage.keypoints_io import load_keypoints

    return load_keypoints(path).frames


def _expand(patterns: list[str]) -> list[Path]:
    """Expand globs ourselves — PowerShell does not do it for us the way a POSIX shell does."""
    found: list[Path] = []
    for pattern in patterns:
        path = Path(pattern)
        if path.exists():
            found.append(path)
            continue
        anchor = Path(path.anchor) if path.anchor else Path()
        relative = str(path)[len(path.anchor) :] if path.anchor else str(path)
        found.extend(sorted(anchor.glob(relative)))
    return found


def _print_table(rows: list[dict]) -> None:
    if not rows:
        print("(no rows)")
        return
    columns = list(dict.fromkeys(k for row in rows for k in row))
    widths = {
        c: max(len(c), max(len(str(row.get(c, ""))) for row in rows)) for c in columns
    }
    print("  ".join(c.ljust(widths[c]) for c in columns))
    print("  ".join("-" * widths[c] for c in columns))
    for row in rows:
        print("  ".join(str(row.get(c, "")).ljust(widths[c]) for c in columns))


def _emit_json(path: str | None, rows: list[dict]) -> None:
    if path:
        Path(path).write_text(json.dumps(rows, indent=2), encoding="utf-8")
        print(f"\n-> {path}")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("inspect", help="container / codec / fps / decode integrity (Q2, Q3)")
    p.add_argument("videos", nargs="+")
    p.add_argument("--json")
    p.set_defaults(func=cmd_inspect)

    p = sub.add_parser("frames", help="dump index-burned frames for hand-labelling")
    p.add_argument("video")
    p.add_argument("--out", required=True)
    p.add_argument("--every", type=int, default=1)
    p.add_argument("--start", type=int, default=0)
    p.add_argument("--end", type=int)
    p.set_defaults(func=cmd_frames)

    p = sub.add_parser("pose", help="streaming MediaPipe pass -> keypoints JSON")
    p.add_argument("video")
    p.add_argument("--out", required=True)
    p.set_defaults(func=cmd_pose)

    p = sub.add_parser("wrist", help="lead/trail wrist visibility + track continuity (Q1)")
    p.add_argument("keypoints")
    p.add_argument("--json")
    p.set_defaults(func=cmd_wrist)

    p = sub.add_parser("variants", help="candidate signals through the shipped rule (Q1)")
    p.add_argument("keypoints")
    p.add_argument("--json")
    p.set_defaults(func=cmd_variants)

    p = sub.add_parser("score", help="error tables + verdicts against hand labels")
    p.add_argument("truth")
    p.add_argument("--json")
    p.set_defaults(func=cmd_score)

    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
