"""Dev CLI: align two views of one swing and render them side by side. [M7 Phase 2]

Usage:
    # what the detector found in each clip, and how well they align — no video needed
    python scripts/align_swings.py face_on.keypoints.json down_the_line.keypoints.json

    # the proof: one MP4, two panels, banners landing simultaneously by construction
    python scripts/align_swings.py face_on.keypoints.json down_the_line.keypoints.json \
        --video-a face_on.MOV --video-b down_the_line.MOV --out aligned.mp4

    # a clip with practice swings in it: see what's there, then point at the real one
    python scripts/align_swings.py a.keypoints.json b.keypoints.json --list-swings
    python scripts/align_swings.py a.keypoints.json b.keypoints.json --window-b 1800:2400

The two clips are segmented **independently** and aligned on the swing instants each produces,
never on a clock — see `analysis/alignment.py` and ADR-015. The visible correctness claim is that
the ADDRESS / TOP / IMPACT banners appear on both panels on the same output frame; they are drawn
from a single `tau` per output frame, so if the alignment is right they cannot disagree.

Videos are optional. Without them the skeletons render on a black canvas, which still proves the
alignment and works on any keypoints pair — including files whose clips are long gone.

Needs the `vision` extra only to render; the text report runs on the base install.
"""

from __future__ import annotations

import argparse
import sys
from collections.abc import Iterator
from contextlib import ExitStack
from pathlib import Path
from typing import TYPE_CHECKING, Any

from golf_coach.analysis.alignment import (
    align_swings,
    anchors_from_keypoints,
    frame_of_tau,
    tau_of_frame,
)
from golf_coach.analysis.phases import candidate_downswings
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.alignment import (
    TAU_IMPACT,
    TAU_MOTION_START,
    TAU_TOP,
    ClipAlignment,
    SwingAlignment,
    SwingAnchors,
)
from golf_coach.contracts.keypoints import FrameKeypoints, KeypointsFile
from golf_coach.storage.keypoints_io import load_keypoints

if TYPE_CHECKING:
    import numpy as np

# Height each panel is scaled to before the two are joined. 4K portrait phone footage is 3840 tall;
# writing that verbatim produces a gigantic file nobody can scrub through, and the alignment is
# just as visible at 720.
_PANEL_HEIGHT = 720

# Fallback canvas when there is no video and the keypoints file never recorded a size.
_DEFAULT_CANVAS = (1280, 720)  # (height, width) is transposed below

# How wide a tau window each banner stays up for, as a fraction of one downswing. ~0.13 is two
# frames at a typical 15-frame 60fps downswing — long enough to see when scrubbing, short enough
# that TOP and IMPACT never overlap.
_BANNER_HALF_TAU = 0.13

_BANNERS: tuple[tuple[float, str], ...] = (
    (TAU_MOTION_START, "ADDRESS"),
    (TAU_TOP, "TOP OF BACKSWING"),
    (TAU_IMPACT, "IMPACT"),
)


def _parse_window(value: str | None) -> tuple[int, int] | None:
    """`--window-a 1800:2400` -> (1800, 2400)."""
    if value is None:
        return None
    try:
        start, _, end = value.partition(":")
        return int(start), int(end)
    except ValueError:
        raise argparse.ArgumentTypeError(
            f"window must look like START:END, got {value!r}"
        ) from None


def _load(path: Path) -> KeypointsFile:
    return load_keypoints(path)


def _override(anchors: SwingAnchors, top: int | None, impact: int | None) -> SwingAnchors:
    """Apply manual anchor overrides, if any.

    This exists because down-the-line anchor detection is the biggest open risk in M7
    (docs/M7_TWO_PHONE_SPIKE.md, Q1) and because a clip full of practice swings can defeat any
    automatic choice. It produces the same `SwingAnchors` the detector does, so the manual path is
    a *parameter* of the alignment rather than a second route through it.
    """
    if top is None and impact is None:
        return anchors
    return anchors.model_copy(
        update={
            "top": anchors.top if top is None else top,
            "impact": anchors.impact if impact is None else impact,
        }
    )


def _print_swings(label: str, keypoints: list[FrameKeypoints], fps: float | None) -> None:
    """List every descent of the hands in the clip — how you find a practice swing."""
    smoothed = smooth_keypoints(keypoints)
    # A lower threshold than segment_phases' own, deliberately: a lazy practice swing often
    # descends less far than the real one, and the point here is to SEE it.
    swings = candidate_downswings(smoothed, min_fraction=0.45)
    print(f"\n{label}: {len(keypoints)} frames, {len(swings)} candidate descent(s)")
    if not swings:
        print("  (none - no detectable descent of the hands)")
        return

    header = f"  {'#':>2}  {'at':>7}  {'top':>6}  {'impact':>6}  {'downswing':>10}  {'drop':>6}"
    print(f"{header}  select with")
    for i, swing in enumerate(swings):
        frames = swing.impact - swing.top
        at = f"{swing.top / fps:6.1f}s" if fps else f"{swing.top:6d}f"
        # A real downswing is ~0.2-0.3 s whoever is swinging; this is by far the easiest way to
        # tell a swing from a rehearsal or from the hands simply being lowered into address.
        duration = f"{frames / fps:8.2f}s" if fps else f"{frames:7d}f"
        pad = max(0, swing.top - 2 * frames)
        window = f"--window {pad}:{swing.impact + frames * 3}"
        print(
            f"  {i:>2}  {at:>7}  {swing.top:>6}  {swing.impact:>6}  {duration:>10}"
            f"  {swing.rise:>6.3f}  {window}"
        )

    print("  row 0 is what segment_phases() picks on its own - right only if it IS the swing.")
    if fps:
        print("  a real downswing runs ~0.2-0.3s; much longer is a rehearsal or a setup move.")


def _print_report(alignment: SwingAlignment, name_a: str, name_b: str) -> None:
    print(f"\nAlignment: {alignment.quality.summary}  [{alignment.quality.value}]")
    for note in alignment.notes:
        print(f"  ! {note}")

    if alignment.a is None or alignment.b is None:
        return

    left, right = alignment.a, alignment.b
    rows: tuple[tuple[str, str, str], ...] = (
        ("motion start (frame)", str(left.anchors.motion_start), str(right.anchors.motion_start)),
        ("top (frame)", str(left.anchors.top), str(right.anchors.top)),
        ("impact (frame)", str(left.anchors.impact), str(right.anchors.impact)),
        (
            "downswing (frames)",
            str(left.anchors.downswing_frames),
            str(right.anchors.downswing_frames),
        ),
        ("tempo ratio", _fmt(left.anchors.tempo_ratio), _fmt(right.anchors.tempo_ratio)),
        ("fps", _fmt(left.anchors.fps), _fmt(right.anchors.fps)),
        ("tau at first frame", f"{left.tau_start:.2f}", f"{right.tau_start:.2f}"),
        ("tau at last frame", f"{left.tau_end:.2f}", f"{right.tau_end:.2f}"),
    )
    print(f"\n  {'':<22}{name_a:>18}{name_b:>18}")
    for label, va, vb in rows:
        print(f"  {label:<22}{va:>18}{vb:>18}")

    if alignment.overlap is not None:
        low, high = alignment.overlap
        print(f"\n  both clips cover tau {low:.2f} -> {high:.2f}", end="")
        print("  (tau: 0=motion start, 1=top, 2=impact)")


def _fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:.2f}"


class _FramePuller:
    """Pull frames from a one-shot decoder at non-decreasing indices, holding the last.

    The warp is monotone, so the follower clip is only ever asked to move forward — which means
    both clips can stream rather than being buffered. That matters: two 1080p60 clips held in
    memory is ~5.8 GB, and 4K is far worse. It is the same bug M7 Phase 1 removed from
    `run_pose.py`, and re-introducing it here would undo that.

    A repeated index re-serves the frame already in hand, which is what happens whenever the
    reference clip is the faster of the two.

    **It always decodes from frame 0**, so a swing 3,000 frames into a 4K clip costs a minute of
    decoding before the first output frame appears. `VideoSource` is a forward-only port with no
    seek, deliberately (a live camera cannot seek), and adding one is a capture-layer change rather
    than an alignment one. Streaming is what keeps memory flat; the wait is the price.
    """

    def __init__(self, frames: Iterator[Any], blank: np.ndarray[Any, Any]) -> None:
        self._frames = frames
        self._current: np.ndarray[Any, Any] = blank
        self._index = -1

    def at(self, index: int) -> np.ndarray[Any, Any]:
        while self._index < index:
            frame = next(self._frames, None)
            if frame is None:
                break  # clip ended early; hold the last frame we did get
            self._current = frame.image
            self._index += 1
        return self._current


def _tau(clip: ClipAlignment, frame: int) -> float:
    """This clip's frame -> the shared axis."""
    return tau_of_frame(clip.anchors, frame, motion_start=clip.warp_motion_start)


def _frame(clip: ClipAlignment, tau: float) -> float:
    """The shared axis -> this clip's frame."""
    return frame_of_tau(clip.anchors, tau, motion_start=clip.warp_motion_start)


def _banner_for(tau: float) -> str | None:
    for anchor_tau, label in _BANNERS:
        if abs(tau - anchor_tau) <= _BANNER_HALF_TAU:
            return label
    return None


def _canvas_size(keypoints_file: KeypointsFile) -> tuple[int, int]:
    """(height, width) for the black-canvas fallback, from the clip metadata when it exists."""
    clip = keypoints_file.clip
    if clip is not None and clip.width and clip.height:
        return clip.height, clip.width
    return _DEFAULT_CANVAS


def _render(
    alignment: SwingAlignment,
    kp_a: list[FrameKeypoints],
    kp_b: list[FrameKeypoints],
    file_a: KeypointsFile,
    file_b: KeypointsFile,
    video_a: Path | None,
    video_b: Path | None,
    reference: str,
    out_path: Path,
    label_a: str,
    label_b: str,
    tau_range: tuple[float, float],
) -> None:
    """Write the side-by-side MP4, streaming both clips."""
    import cv2
    import numpy as np

    from golf_coach.capture.file import FileVideoSource
    from golf_coach.pose.overlay import annotate_frame

    assert alignment.a is not None and alignment.b is not None
    assert alignment.overlap is not None

    ref: ClipAlignment = alignment.a if reference == "a" else alignment.b
    fps = ref.anchors.fps or 60.0

    # The reference clip drives the output timeline, so its OWN length is the bound — reading the
    # other clip's here would truncate or overrun whenever the two differ, which is always.
    ref_frames = len(kp_a) if reference == "a" else len(kp_b)

    # Clamp the render to the part of the axis that is actually a swing. The honest overlap of two
    # long clips is mostly dead air — a golfer standing over the ball, or the bay after they walk
    # off — and rendering all of it buries the thing the video exists to show. It is also the
    # region the warp describes worst whenever the soft anchor was refused.
    low = max(alignment.overlap[0], tau_range[0])
    high = min(alignment.overlap[1], tau_range[1])
    first = max(0, int(_frame(ref, low)))
    last = min(ref_frames - 1, int(_frame(ref, high)))
    if last <= first:
        print("error: the two clips share no overlapping swing time", file=sys.stderr)
        return

    blank_a = np.zeros((*_canvas_size(file_a), 3), dtype=np.uint8)
    blank_b = np.zeros((*_canvas_size(file_b), 3), dtype=np.uint8)
    writer: cv2.VideoWriter | None = None

    with ExitStack() as stack:
        source_a = stack.enter_context(FileVideoSource(video_a)) if video_a else None
        source_b = stack.enter_context(FileVideoSource(video_b)) if video_b else None
        pull_a = _FramePuller(source_a.frames() if source_a else iter(()), blank_a)
        pull_b = _FramePuller(source_b.frames() if source_b else iter(()), blank_b)

        for step in range(first, last + 1):
            # ONE tau per output frame, used for both panels. This is what makes the banners
            # land simultaneously by construction rather than by coincidence.
            if reference == "a":
                frame_a = step
                tau = _tau(alignment.a, step)
                frame_b = _clamped(_frame(alignment.b, tau), len(kp_b))
            else:
                frame_b = step
                tau = _tau(alignment.b, step)
                frame_a = _clamped(_frame(alignment.a, tau), len(kp_a))

            # Scale FIRST, then annotate. Landmarks are normalized to [0, 1], so the skeleton
            # lands identically either way — but `annotate_frame` draws text at a fixed pixel
            # size, and text drawn on a 3840-tall 4K frame is unreadable once the panel is
            # shrunk to 720. Annotating the scaled frame keeps the HUD legible at output size.
            banner = _banner_for(tau)
            panel_a = annotate_frame(
                _scaled(pull_a.at(frame_a)),
                kp_a[min(frame_a, len(kp_a) - 1)],
                banner=banner,
                hud_lines=(f"{label_a}  frame {frame_a}",),
            )
            panel_b = annotate_frame(
                _scaled(pull_b.at(frame_b)),
                kp_b[min(frame_b, len(kp_b) - 1)],
                banner=banner,
                hud_lines=(
                    f"{label_b}  frame {frame_b}",
                    f"tau {tau:+.2f}  {alignment.quality.summary}",
                ),
            )

            joined = np.hstack([panel_a, panel_b])
            if writer is None:
                height, width = joined.shape[:2]
                writer = cv2.VideoWriter(
                    str(out_path), cv2.VideoWriter_fourcc(*"mp4v"), fps, (width, height)
                )
            writer.write(joined)

        if writer is not None:
            writer.release()

    print(f"Wrote {last - first + 1} aligned frames -> {out_path}")


def _scaled(panel: np.ndarray[Any, Any]) -> np.ndarray[Any, Any]:
    import cv2

    height, width = panel.shape[:2]
    scale = _PANEL_HEIGHT / height
    return cv2.resize(panel, (max(1, int(width * scale)), _PANEL_HEIGHT))


def _clamped(frame: float, count: int) -> int:
    return max(0, min(count - 1, round(frame)))


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Align two views of one swing on a normalized swing-time axis (M7 Phase 2)."
    )
    parser.add_argument("keypoints_a", type=Path, help="first clip's keypoints JSON")
    parser.add_argument("keypoints_b", type=Path, help="second clip's keypoints JSON")
    parser.add_argument("--video-a", type=Path, default=None, help="pixels for panel A (optional)")
    parser.add_argument("--video-b", type=Path, default=None, help="pixels for panel B (optional)")
    parser.add_argument("--out", type=Path, default=None, help="write a side-by-side MP4 here")
    parser.add_argument(
        "--reference",
        choices=("a", "b"),
        default=None,
        help="which clip drives the output timeline; defaults to the face-on clip, else A",
    )
    parser.add_argument("--window-a", default=None, help="restrict clip A to START:END frames")
    parser.add_argument("--window-b", default=None, help="restrict clip B to START:END frames")
    parser.add_argument("--top-a", type=int, default=None, help="override clip A's top frame")
    parser.add_argument("--impact-a", type=int, default=None, help="override clip A's impact frame")
    parser.add_argument("--top-b", type=int, default=None, help="override clip B's top frame")
    parser.add_argument("--impact-b", type=int, default=None, help="override clip B's impact frame")
    parser.add_argument(
        "--tau",
        default="-0.4:3.0",
        help=(
            "swing-time range to render, LO:HI (0=motion start, 1=top, 2=impact). The default "
            "runs from a little before the takeaway to one downswing past impact, i.e. the swing "
            "and nothing else; widen it to see more of the clip."
        ),
    )
    parser.add_argument(
        "--list-swings",
        action="store_true",
        help="list every candidate swing in each clip and exit — use on clips with practice swings",
    )
    args = parser.parse_args(argv)

    for path in (args.keypoints_a, args.keypoints_b):
        if not path.exists():
            print(f"error: {path} not found", file=sys.stderr)
            return 1

    file_a, file_b = _load(args.keypoints_a), _load(args.keypoints_b)
    kp_a, kp_b = file_a.frames, file_b.frames
    name_a = args.keypoints_a.stem.removesuffix(".keypoints")
    name_b = args.keypoints_b.stem.removesuffix(".keypoints")

    if args.list_swings:
        _print_swings(name_a, kp_a, file_a.clip.fps if file_a.clip else None)
        _print_swings(name_b, kp_b, file_b.clip.fps if file_b.clip else None)
        return 0

    anchors_a = anchors_from_keypoints(kp_a, clip=file_a.clip, window=_parse_window(args.window_a))
    anchors_b = anchors_from_keypoints(kp_b, clip=file_b.clip, window=_parse_window(args.window_b))
    if anchors_a is None or anchors_b is None:
        missing = name_a if anchors_a is None else name_b
        print(f"error: {missing} could not be segmented into a swing", file=sys.stderr)
        return 1

    anchors_a = _override(anchors_a, args.top_a, args.impact_a)
    anchors_b = _override(anchors_b, args.top_b, args.impact_b)

    alignment = align_swings(anchors_a, anchors_b)
    _print_report(alignment, name_a, name_b)

    if args.out is not None:
        # The face-on clip leads by default: it is the view the three validated checkpoints are
        # measured from, so it is the one whose motion should stay at its native rate.
        reference = args.reference
        if reference is None:
            b_leads = anchors_b.camera_id == "face_on" and anchors_a.camera_id != "face_on"
            reference = "b" if b_leads else "a"
        for video in (args.video_a, args.video_b):
            if video is not None and not video.exists():
                print(f"error: video {video} not found", file=sys.stderr)
                return 1
        tau_lo, _, tau_hi = args.tau.partition(":")
        _render(
            alignment, kp_a, kp_b, file_a, file_b,
            args.video_a, args.video_b, reference, args.out,
            anchors_a.camera_id or name_a, anchors_b.camera_id or name_b,
            (float(tau_lo), float(tau_hi)),
        )
    print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
