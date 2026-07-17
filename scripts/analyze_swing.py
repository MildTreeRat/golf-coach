"""Dev CLI: run the analysis spine over a keypoints JSON and report the result. [M4-PoC+]

Usage:
    python scripts/analyze_swing.py data/processed/my_swing.keypoints.json
    python scripts/analyze_swing.py data/processed/my_swing.keypoints.json \
        --overlay data/raw/my_swing.mov

Runs `analyze_swing` + `build_feedback` on an existing M1 keypoints file and prints a text
report — per-checkpoint scores, the overall score, plain-English tips, and the detected
address / top / impact frames. With `--overlay <video>` it also renders an annotated clip that
stamps those instants on the frames plus a score HUD, so you can *visually verify* the
segmentation landed on the right moments. This is the no-hardware substitute for ground-truth
accuracy (see docs/M4_FUNDAMENTALS_PANEL.md).
"""

from __future__ import annotations

import sys
from pathlib import Path

from pydantic import TypeAdapter

from golf_coach.analysis.engine import analyze_swing
from golf_coach.contracts.keypoints import FrameKeypoints
from golf_coach.contracts.swing import PhaseSegment, SwingPhase, SwingResult
from golf_coach.feedback.rules import build_feedback

_KEYPOINTS_ADAPTER = TypeAdapter(list[FrameKeypoints])

# How many frames each side of an instant the overlay banner stays up, so it is visible.
_BANNER_HALF_FRAMES = 2


def _bounds(phases: list[PhaseSegment], phase: SwingPhase) -> tuple[int, int] | None:
    for segment in phases:
        if segment.phase is phase:
            return segment.start_frame, segment.end_frame
    return None


def _instants(result: SwingResult) -> dict[str, int]:
    """Detected address / top / impact frame indices from the phase boundaries."""
    instants: dict[str, int] = {}
    address = _bounds(result.phases, SwingPhase.ADDRESS)
    transition = _bounds(result.phases, SwingPhase.TRANSITION)
    impact = _bounds(result.phases, SwingPhase.IMPACT)
    if address is not None:
        instants["ADDRESS"] = address[1]
    if transition is not None:
        instants["TOP OF BACKSWING"] = (transition[0] + transition[1]) // 2
    if impact is not None:
        instants["IMPACT"] = impact[0]
    return instants


def _print_report(result: SwingResult, instants: dict[str, int]) -> None:
    print(f"\nSwing: {result.swing_id}   frames: {len(result.keypoints)}")
    overall = f"Overall score: {result.overall_score:.0f}/100"
    if result.mechanics_score is not None:
        overall += f"  (mechanics {result.mechanics_score:.0f}/100)"
    print(overall)
    print(
        "Detected instants (frame): "
        + ", ".join(f"{label} @ {frame}" for label, frame in instants.items())
    )

    print("\nCheckpoints:")
    if not result.checkpoint_scores:
        print("  (none scored - no benchmark band matched, or the swing was unsegmentable)")
    for cp in result.checkpoint_scores:
        band = (
            f"[{cp.expected_low} - {cp.expected_high}]"
            if cp.expected_low is not None
            else "[no band]"
        )
        flag = "PASS" if cp.passed else "MISS"
        print(f"  {cp.name:<16} observed={cp.observed}  band={band}  "
              f"score={cp.score:.0%}  {flag}")

    print("\nTips:")
    feedback = build_feedback(result)
    for tip in feedback.tips:
        print(f"  [{tip.severity.value.upper():<5}] {tip.text}")
    print()


def _write_overlay(
    keypoints: list[FrameKeypoints],
    result: SwingResult,
    instants: dict[str, int],
    video_path: Path,
    out_path: Path,
) -> None:
    import cv2

    from golf_coach.capture.file import FileVideoSource
    from golf_coach.pose.overlay import annotate_frame

    with FileVideoSource(video_path) as source:
        fps = source.fps
        frames = [frame.image for frame in source.frames()]

    if len(frames) != len(keypoints):
        print(
            f"warning: video has {len(frames)} frames but keypoints has {len(keypoints)}; "
            "overlaying the common prefix.",
            file=sys.stderr,
        )

    # frame index -> banner label, held for a small window around each instant.
    banners: dict[int, str] = {}
    for label, center in instants.items():
        for f in range(center - _BANNER_HALF_FRAMES, center + _BANNER_HALF_FRAMES + 1):
            banners.setdefault(f, label)

    hud = tuple(
        f"{cp.name}: {cp.observed} ({'PASS' if cp.passed else 'MISS'})"
        for cp in result.checkpoint_scores
    ) + (f"overall {result.overall_score:.0f}/100",)

    height, width = frames[0].shape[:2]
    fourcc = cv2.VideoWriter_fourcc(*"mp4v")
    writer = cv2.VideoWriter(str(out_path), fourcc, fps, (width, height))
    try:
        for i, (image, kp) in enumerate(zip(frames, keypoints, strict=False)):
            writer.write(annotate_frame(image, kp, banner=banners.get(i), hud_lines=hud))
    finally:
        writer.release()
    print(f"Wrote annotated overlay -> {out_path}")


def main(argv: list[str]) -> int:
    if not argv or argv[0] in {"-h", "--help"}:
        print(
            "usage: python scripts/analyze_swing.py <keypoints.json> [--overlay <video>]",
            file=sys.stderr,
        )
        return 2

    keypoints_path = Path(argv[0])
    overlay_video: Path | None = None
    if "--overlay" in argv:
        idx = argv.index("--overlay")
        if idx + 1 >= len(argv):
            print("error: --overlay requires a video path", file=sys.stderr)
            return 2
        overlay_video = Path(argv[idx + 1])

    if not keypoints_path.exists():
        print(f"error: {keypoints_path} not found", file=sys.stderr)
        return 1

    keypoints = _KEYPOINTS_ADAPTER.validate_json(keypoints_path.read_bytes())
    # run_pose.py names files "<clip>.keypoints.json"; drop the trailing ".keypoints" so the
    # swing id and outputs read as the original clip name.
    stem = keypoints_path.stem.removesuffix(".keypoints")
    result = analyze_swing(
        swing_id=stem,
        session_id="cli",
        keypoints=keypoints,
    )
    instants = _instants(result)
    _print_report(result, instants)

    if overlay_video is not None:
        if not overlay_video.exists():
            print(f"error: overlay video {overlay_video} not found", file=sys.stderr)
            return 1
        out_path = keypoints_path.with_name(f"{stem}.analysis.mp4")
        _write_overlay(keypoints, result, instants, overlay_video, out_path)

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
