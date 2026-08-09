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
from contextlib import ExitStack
from pathlib import Path

from golf_coach.analysis.alignment import (
    align_swings,
    anchors_from_keypoints,
    pair_frames,
)
from golf_coach.analysis.phases import (
    CANDIDATE_MIN_RISE,
    candidate_downswings,
    select_swing,
    window_around,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.alignment import SwingAlignment, SwingAnchors
from golf_coach.contracts.keypoints import FrameKeypoints, KeypointsFile
from golf_coach.storage.keypoints_io import load_keypoints


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
    # descends less far than the real one, and the point here is to SEE it. Shared with
    # `select_swing` so the set you choose from and the set it chooses from are identical.
    swings = candidate_downswings(smoothed, min_fraction=CANDIDATE_MIN_RISE)
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
        # tell a swing from a rehearsal or from the hands simply being lowered into address, and
        # it is the rule `select_swing` automates.
        duration = f"{frames / fps:8.2f}s" if fps else f"{frames:7d}f"
        start, end = window_around(swing)
        window = f"--window {start}:{end}"
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
    """Open the clips and hand them to the renderer, which streams both."""
    from golf_coach.capture.file import FileVideoSource
    from golf_coach.pose.side_by_side import (
        BROWSER_HOSTILE_CODECS,
        Panel,
        render_side_by_side,
    )

    schedule = pair_frames(
        alignment, len(kp_a), len(kp_b), reference=reference, tau_range=tau_range
    )
    if not schedule:
        print("error: the two clips share no overlapping swing time", file=sys.stderr)
        return

    lead = alignment.a if reference == "a" else alignment.b
    assert lead is not None  # pair_frames returns [] unless both sides are present

    with ExitStack() as stack:
        source_a = stack.enter_context(FileVideoSource(video_a)) if video_a else None
        source_b = stack.enter_context(FileVideoSource(video_b)) if video_b else None
        render = render_side_by_side(
            out_path,
            schedule,
            Panel(kp_a, label_a, file_a.clip, source_a.frames() if source_a else ()),
            Panel(kp_b, label_b, file_b.clip, source_b.frames() if source_b else ()),
            fps=lead.anchors.fps or 60.0,
            quality=alignment.quality,
        )

    print(f"Wrote {render.frames} aligned frames ({render.codec}) -> {out_path}")
    if render.codec in BROWSER_HOSTILE_CODECS:
        print(f"  note: {render.codec} plays in VLC but not in most browsers - see README")


def _auto_window(
    label: str, keypoints: list[FrameKeypoints], file: KeypointsFile
) -> tuple[int, int] | None:
    """`select_swing`'s pick for one clip, narrating what it chose or why it declined."""
    fps = file.clip.fps if file.clip else None
    choice = select_swing(smooth_keypoints(keypoints), fps=fps)
    if choice is None:
        reason = "the keypoints file records no fps" if fps is None else "no plausible downswing"
        print(f"  {label}: auto-window declined ({reason}) — using the whole clip")
        return None
    print(f"  {label}: {choice.reason}")
    return choice.window


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
    parser.add_argument(
        "--auto-window",
        action="store_true",
        help=(
            "pick each clip's swing automatically by downswing duration (analysis.phases."
            "select_swing) instead of using the whole clip. An explicit --window-a/-b still wins"
        ),
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

    window_a = _parse_window(args.window_a)
    window_b = _parse_window(args.window_b)
    if args.auto_window:
        print("\nAuto-window:")
        if window_a is None:
            window_a = _auto_window(name_a, kp_a, file_a)
        if window_b is None:
            window_b = _auto_window(name_b, kp_b, file_b)

    anchors_a = anchors_from_keypoints(kp_a, clip=file_a.clip, window=window_a)
    anchors_b = anchors_from_keypoints(kp_b, clip=file_b.clip, window=window_b)
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
