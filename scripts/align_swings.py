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
never on a clock — see `analysis/alignment.py` and ADR-015. `--auto-window` is the one thing that
is not independent, and only in *which* swing it points each clip at: since M10 P8 the face-on
clip is chosen first and the other is matched to its downswing duration.

The visible correctness claim is that the ADDRESS / TOP / IMPACT banners appear on both panels on
the same output frame; they are drawn from a single `tau` per output frame, so if the alignment is
right they cannot disagree.

Videos are optional. Without them the skeletons render on a black canvas, which still proves the
alignment and works on any keypoints pair — including files whose clips are long gone.

Needs the `vision` extra only to render; the text report runs on the base install.
"""

from __future__ import annotations

import argparse
import sys
from contextlib import ExitStack
from pathlib import Path
from typing import NamedTuple

from golf_coach.analysis.alignment import (
    align_swings,
    anchors_from_keypoints,
    pair_frames,
)
from golf_coach.analysis.phases import (
    CANDIDATE_MIN_RISE,
    LEAD_WRIST,
    TRAIL_WRIST,
    SwingChoice,
    candidate_downswings,
    select_matching_swing,
    select_swing,
    window_around,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.contracts.alignment import SwingAlignment, SwingAnchors
from golf_coach.contracts.keypoints import FrameKeypoints, KeypointsFile, PoseLandmark
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


def _selection_wrist(keypoints: list[FrameKeypoints]) -> PoseLandmark:
    """Which landmark this clip is read on: the trail wrist from behind, else the lead wrist.

    This script takes two clips called A and B and is told nothing about which view either is, so
    it asks the frames — the same `camera_id` `main()` already reads to decide which clip drives
    the output timeline, and that `alignment.anchors_from_keypoints` reads to stamp the anchors.
    The field is free-form by contract (`contracts/keypoints.py`); M7 writes
    `storage.manifest.Role`'s own words into it by convention, which is what this matches. A file
    recording no `camera_id` — everything written before M7 Phase 1 — gets the lead wrist, which
    is what it has always been read on.

    Why per view at all: from down-the-line the lead wrist is the far arm, occluded by the torso
    through the top and tracked in 39% of frames (`phases.TRAIL_WRIST`). `engine` has segmented
    that view on the trail wrist since M4 §Phase F. This script did not, so its report of a
    down-the-line clip disagreed with what the pipeline produces from the same file.
    """
    return TRAIL_WRIST if _camera_id(keypoints) == "down_the_line" else LEAD_WRIST


def _camera_id(keypoints: list[FrameKeypoints]) -> str | None:
    """Which view this clip says it is, or `None` for a file written before M7 Phase 1."""
    return next((f.camera_id for f in keypoints if f.camera_id is not None), None)


def _wrist_name(wrist: PoseLandmark) -> str:
    """The repo's word for a landmark. Not `wrist.name`, which says "left wrist" — true of the
    index and wrong for a left-handed golfer, whose lead wrist is the right one."""
    return "trail wrist" if wrist is TRAIL_WRIST else "lead wrist"


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
    wrist = _selection_wrist(keypoints)
    # A lower threshold than segment_phases' own, deliberately: a lazy practice swing often
    # descends less far than the real one, and the point here is to SEE it. Shared with
    # `select_swing` — same threshold and same landmark — so the set you choose from and the set
    # it chooses from are identical.
    swings = candidate_downswings(smoothed, min_fraction=CANDIDATE_MIN_RISE, wrist=wrist)
    print(
        f"\n{label}: {len(keypoints)} frames, {len(swings)} candidate descent(s)"
        f"  [read on the {_wrist_name(wrist)}]"
    )
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
        start, end = window_around(swing, fps=fps)
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

    print(
        f"Wrote {render.frames} aligned frames ({render.codec}) -> {out_path}"
        f"  [reads back {render.frames_read}]"
    )
    if render.frames_read is not None and render.frames_read != render.frames:
        print(
            f"  note: the file decodes {render.frames_read} of {render.frames} frames written - "
            "re-render before trusting it"
        )
    if render.codec in BROWSER_HOSTILE_CODECS:
        print(f"  note: {render.codec} plays in VLC but not in most browsers - see README")


class _Clip(NamedTuple):
    """One of the two files this script was handed, in the three shapes the rest of it wants."""

    name: str
    keypoints: list[FrameKeypoints]
    file: KeypointsFile


def _pick_swing(clip: _Clip, *, reference_downswing_s: float | None = None) -> SwingChoice | None:
    """This clip's swing: chosen alone, or against the duration the other clip already measured."""
    fps = clip.file.clip.fps if clip.file.clip else None
    frames = smooth_keypoints(clip.keypoints)
    wrist = _selection_wrist(clip.keypoints)
    if reference_downswing_s is not None:
        return select_matching_swing(
            frames, fps=fps, reference_downswing_s=reference_downswing_s, wrist=wrist
        )
    return select_swing(frames, fps=fps, wrist=wrist)


def _downswing_seconds(choice: SwingChoice | None, clip: _Clip) -> float | None:
    """The chosen descent's duration — all one clip offers the other. `None` if it has none."""
    fps = clip.file.clip.fps if clip.file.clip else None
    if choice is None or fps is None or fps <= 0.0:
        return None
    seconds = (choice.downswing.impact - choice.downswing.top) / fps
    return seconds if seconds > 0.0 else None


def _narrate_choice(clip: _Clip, choice: SwingChoice | None) -> tuple[int, int] | None:
    """What one clip ended up with — printed after the last attempt at it, never before."""
    wrist = _selection_wrist(clip.keypoints)
    if choice is None:
        fps = clip.file.clip.fps if clip.file.clip else None
        reason = "the keypoints file records no fps" if fps is None else "no plausible downswing"
        print(
            f"  {clip.name}: auto-window declined ({reason}, on the {_wrist_name(wrist)})"
            " — using the whole clip"
        )
        return None
    print(f"  {clip.name}: {choice.reason}  [read on the {_wrist_name(wrist)}]")
    return choice.window


def _auto_windows(
    clip_a: _Clip,
    clip_b: _Clip,
    window_a: tuple[int, int] | None,
    window_b: tuple[int, int] | None,
) -> tuple[tuple[int, int] | None, tuple[int, int] | None]:
    """Both clips' windows: the face-on clip picked first, the other matched against it.

    The same order `api.pipeline._auto_windows` runs, and that is the point of doing it here at
    all. A window printed by this script is one a human copies into `--window-a/-b`, so a script
    that chose independently would report a different swing than the pipeline scores from the same
    file — on six of the fifteen stored bundles it did (M10 §A2, P8). The *rule* is shared,
    `phases.select_matching_swing`; only the narration is written twice, because this one prints
    where the pipeline's writes notes.

    An explicit window always wins, and is never a reference: it is a frame range rather than a
    downswing duration, and it may deliberately point at a different swing than the selector would
    have picked.
    """
    # Face-on leads because it is the view "the last plausible descent" is right about, while a
    # down-the-line phone keeps rolling 15-24 s past impact. Same test as `main`'s `b_leads`.
    swapped = (
        _camera_id(clip_b.keypoints) == "face_on" and _camera_id(clip_a.keypoints) != "face_on"
    )
    lead, follow = (clip_b, clip_a) if swapped else (clip_a, clip_b)
    lead_window, follow_window = (window_b, window_a) if swapped else (window_a, window_b)
    if _camera_id(lead.keypoints) != "face_on":
        print(f"  note: neither clip says it is face-on — taking {lead.name} as the reference")

    lead_choice = None if lead_window is not None else _pick_swing(lead)
    reference = _downswing_seconds(lead_choice, lead)
    follow_choice = (
        None
        if follow_window is not None
        else _pick_swing(follow, reference_downswing_s=reference)
    )
    # The reverse: when the reference clip declines, the other one's pick becomes the reference
    # for a second attempt at it. `api.pipeline._auto_windows` records what that buys and risks.
    if lead_window is None and lead_choice is None and follow_choice is not None:
        back_reference = _downswing_seconds(follow_choice, follow)
        if back_reference is not None:
            lead_choice = _pick_swing(lead, reference_downswing_s=back_reference)

    if lead_window is None:
        lead_window = _narrate_choice(lead, lead_choice)
    if follow_window is None:
        follow_window = _narrate_choice(follow, follow_choice)
        if follow_choice is not None and reference is None:
            print(
                f"  note: nothing to cross-check {follow.name} against — {lead.name} declined or "
                "was given by hand, so this is the pick P8 exists to stop relying on"
            )
    return (follow_window, lead_window) if swapped else (lead_window, follow_window)


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
            "pick each clip's swing automatically instead of using the whole clip: the face-on "
            "clip by downswing duration (analysis.phases.select_swing), the other one by matching "
            "it (select_matching_swing). An explicit --window-a/-b still wins"
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
        window_a, window_b = _auto_windows(
            _Clip(name_a, kp_a, file_a), _Clip(name_b, kp_b, file_b), window_a, window_b
        )

    # The same landmark the window was picked on, for the reason `engine.analyze_swing_bundle`
    # passes one here: the window decides which frames these anchors are measured over, so a
    # window chosen on one wrist and anchors segmented on the other describe two different swings
    # (M10 §A1). Until M10 P5 this passed no wrist at all, so a down-the-line clip was reported
    # here on the lead wrist and scored in the pipeline on the trail one.
    anchors_a = anchors_from_keypoints(
        kp_a, clip=file_a.clip, window=window_a, wrist=_selection_wrist(kp_a)
    )
    anchors_b = anchors_from_keypoints(
        kp_b, clip=file_b.clip, window=window_b, wrist=_selection_wrist(kp_b)
    )
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
