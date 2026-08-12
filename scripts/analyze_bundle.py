"""Dev CLI: an assembled swing bundle in, the complete result out. [M7 Phase 4]

Usage:
    python scripts/analyze_bundle.py 2026-08-07/1          # SESSION/SWING in the bundle store
    python scripts/analyze_bundle.py path/to/swing_dir     # or the directory itself

    python scripts/analyze_bundle.py 2026-08-07/1 --list-swings          # what's in the clips
    python scripts/analyze_bundle.py 2026-08-07/1 --window-face-on 676:760

A *bundle* is what two phones and a screen photo produce: a face-on clip, a down-the-line clip
and a picture of the launch monitor, grouped into one swing by `storage.bundle_store`.

The pipeline itself lives in `golf_coach.api.pipeline` — the same code the upload server's
background worker runs, so what you see here and what a phone sees on the results page cannot
drift apart (M7 Phase 5). This file is the terminal's half of it: flags in, report out. Note that
the report prints *after* the render rather than before it, so the score lands next to your
prompt instead of scrolled off the top by a few minutes of encoding.

Needs the `vision` extra to run pose or render, and `ocr` only to read a shot photo that is not
already in the store. It does **not** need the `api` extra — `pipeline` imports no web framework.

Exit codes: 0 clean, 1 produced a result with something flagged (shot needs review, a role
missing), 2 no result.
"""

from __future__ import annotations

import argparse
import sys
import textwrap
from pathlib import Path

from golf_coach.analysis.phases import (
    CANDIDATE_MIN_RISE,
    candidate_downswings,
    window_around,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.api.pipeline import (
    VIEWS,
    PipelineOptions,
    analyze_swing_dir,
    keypoints_for,
    resolve_swing_dir,
)
from golf_coach.config import settings
from golf_coach.contracts.feedback import FeedbackPayload
from golf_coach.contracts.intent import ClubCategory
from golf_coach.contracts.keypoints import KeypointsFile
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import SwingBundleResult
from golf_coach.storage.manifest import Role, load_manifest, manifest_path

# Metrics worth printing; the rest live in the stored JSON. Mirrors import_shot_screens.py.
_SHOT_FIELDS = (
    ("carry", "carry_distance"),
    ("total", "total_distance"),
    ("ball", "ball_speed"),
    ("club", "club_head_speed"),
    ("smash", "smash_factor"),
    ("path", "club_path"),
    ("face", "club_face_angle"),
    ("launch", "launch_angle"),
    ("shape", "shot_type"),
    ("strike", "impact_position"),
)


def _parse_window(value: str | None) -> tuple[int, int] | None:
    """`--window-face-on 676:760` -> (676, 760)."""
    if value is None:
        return None
    try:
        start, _, end = value.partition(":")
        return int(start), int(end)
    except ValueError:
        raise argparse.ArgumentTypeError(
            f"window must look like START:END, got {value!r}"
        ) from None


def _parse_tau(value: str | None) -> tuple[float, float] | None:
    """`--tau -0.4:3.0` -> (-0.4, 3.0); None leaves the pipeline's default in place."""
    if value is None:
        return None
    try:
        low, _, high = value.partition(":")
        return float(low), float(high)
    except ValueError:
        raise argparse.ArgumentTypeError(f"tau must look like LO:HI, got {value!r}") from None


def _print_swings(label: str, flag: str, keypoints: KeypointsFile) -> None:
    """Every descent of the hands in one view — how you find a swing the rule missed."""
    fps = keypoints.clip.fps if keypoints.clip else None
    swings = candidate_downswings(
        smooth_keypoints(keypoints.frames), min_fraction=CANDIDATE_MIN_RISE
    )
    print(f"\n{label}: {len(keypoints.frames)} frames, {len(swings)} candidate descent(s)")
    if not swings:
        print("  (none - no detectable descent of the hands)")
        return
    print(f"  {'#':>2}  {'at':>7}  {'top':>6}  {'impact':>6}  {'downswing':>10}  select with")
    for i, swing in enumerate(swings):
        frames = swing.impact - swing.top
        at = f"{swing.top / fps:6.1f}s" if fps else f"{swing.top:6d}f"
        duration = f"{frames / fps:8.2f}s" if fps else f"{frames:7d}f"
        start, end = window_around(swing)
        print(
            f"  {i:>2}  {at:>7}  {swing.top:>6}  {swing.impact:>6}  {duration:>10}"
            f"  {flag} {start}:{end}"
        )
    print("  a real downswing runs ~0.2-0.3s; much longer is a rehearsal or a setup move.")


def _print_report(result: SwingBundleResult) -> None:
    swing = result.swing
    print(f"\nSwing {result.swing_id}  (session {result.session_id})")
    overall = f"Overall score: {swing.overall_score:.0f}/100"
    if swing.mechanics_score is not None:
        overall += f"  (mechanics {swing.mechanics_score:.0f}/100)"
    print(overall)

    print("\nCheckpoints:")
    if not swing.checkpoint_scores:
        print("  (none scored - no benchmark band matched, or the swing was unsegmentable)")
    for cp in swing.checkpoint_scores:
        band = (
            f"[{cp.expected_low} - {cp.expected_high}]"
            if cp.expected_low is not None
            else "[no band]"
        )
        place = f"  tour pct={cp.percentile:g}" if cp.percentile is not None else ""
        print(
            f"  {cp.name:<16} observed={cp.observed}  band={band}  "
            f"score={cp.score:.0%}  {'PASS' if cp.passed else 'MISS'}{place}"
        )
    if swing.unscored:
        print(f"  not measured: {', '.join(swing.unscored)}")

    if result.alignment is not None and result.alignment.a and result.alignment.b:
        a, b = result.alignment.a.anchors, result.alignment.b.anchors
        print(f"\nAlignment: {result.alignment.quality.summary}")
        print(f"  face-on        top {a.top}  impact {a.impact}")
        print(f"  down-the-line  top {b.top}  impact {b.impact}")

    _print_shot(result.swing.shot)

    # Before the tips, deliberately. A note can say a checkpoint's own input was impossible, and
    # a headline built on that number must not be read without it.
    if result.notes:
        print("\nNotes:")
        for note in result.notes:
            print(f"  ! {note}")

    if result.feedback is not None:
        if result.feedback.headline:
            print(f"\n>> {result.feedback.headline}")
        print("\nTips (most actionable first):")
        for tip in result.feedback.tips:
            print(f"  [{tip.severity.value.upper():<5}] {tip.text}")
        _print_coaching(result.feedback)


def _print_coaching(feedback: FeedbackPayload) -> None:
    """The written coaching, under a heading that says who wrote it (M6).

    Attribution above the prose rather than below it: everything else this report prints is a
    measurement, and a reader scanning down has to know the register changed before they read it.
    """
    if not feedback.coaching_text or feedback.coaching is None:
        return
    print(f"\nCoach ({feedback.coaching.model}, written from the numbers above):")
    for line in textwrap.wrap(feedback.coaching_text, width=88):
        print(f"  {line}")


def _print_shot(shot: ShotData | None) -> None:
    """The HD Golf numbers, and how much to trust them (ADR-014)."""
    print("\nShot data:")
    if shot is None:
        print("  (none attached)")
        return

    provenance = shot.provenance
    confidence = provenance.parse_confidence if provenance else 1.0
    flag = "  ** NEEDS REVIEW **" if provenance and provenance.needs_review else ""
    print(f"  {shot.shot_id}   source={shot.source.value}  conf={confidence:.2f}{flag}")
    metrics = "  ".join(
        f"{label}={value}" if isinstance(value, str) else f"{label}={value:g}"
        for label, attribute in _SHOT_FIELDS
        if (value := getattr(shot, attribute)) is not None
    )
    print(f"  {metrics}")
    if provenance and provenance.warnings:
        for warning in provenance.warnings:
            print(f"    - {warning}")


def _list_swings(swing_dir: Path, *, force_pose: bool) -> int:
    """`--list-swings`: the candidate descents in each view, then stop."""
    manifest = load_manifest(manifest_path(swing_dir))
    if manifest is None:
        print(f"error: {swing_dir} has no readable manifest", file=sys.stderr)
        return 2
    for role, label in VIEWS:
        keypoints = keypoints_for(swing_dir, manifest, role, force=force_pose, log=print)
        if keypoints is not None:
            flag = "--window-face-on" if role is Role.FACE_ON else "--window-dtl"
            _print_swings(label, flag, keypoints)
    print()
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="analyze_bundle",
        description="Analyze one assembled swing bundle: two views + a shot photo (M7 Phase 4).",
    )
    parser.add_argument("swing", help="SESSION/SWING in the bundle store, or a swing directory")
    parser.add_argument("--sessions-dir", type=Path, default=settings.sessions_dir)
    parser.add_argument("--shots-dir", type=Path, default=settings.shots_dir)
    parser.add_argument("--window-face-on", default=None, help="restrict face-on to START:END")
    parser.add_argument("--window-dtl", default=None, help="restrict down-the-line to START:END")
    parser.add_argument(
        "--no-auto-window",
        action="store_true",
        help="score the whole clip instead of picking the swing by downswing duration",
    )
    parser.add_argument(
        "--list-swings",
        action="store_true",
        help="list every candidate swing in each view and exit — use when the pick looks wrong",
    )
    parser.add_argument("--no-video", action="store_true", help="skip the side-by-side render")
    parser.add_argument(
        "--no-coaching",
        action="store_true",
        help="skip the Claude coaching call (it is skipped anyway with no API key configured)",
    )
    parser.add_argument("--force-pose", action="store_true", help="re-run pose, ignoring the cache")
    parser.add_argument("--force-ocr", action="store_true", help="re-read the shot screen")
    parser.add_argument("--skip-ocr", action="store_true", help="do not OCR an unstored photo")
    parser.add_argument(
        "--club",
        type=ClubCategory,
        default=ClubCategory.ALL,
        choices=list(ClubCategory),
        help="club used, for club-keyed benchmark bands",
    )
    parser.add_argument(
        "--min-confidence", type=float, default=settings.ocr_min_confidence,
        help="below this an OCR parse is flagged for review",
    )
    parser.add_argument(
        "--tau", default=None,
        help="swing-time range to render, LO:HI (0=motion start, 1=top, 2=impact)",
    )
    args = parser.parse_args(argv)

    swing_dir = resolve_swing_dir(args.swing, args.sessions_dir)
    if swing_dir is None:
        print(
            f"error: no swing bundle at {args.swing!r} (looked in {args.sessions_dir})",
            file=sys.stderr,
        )
        return 2

    if args.list_swings:
        return _list_swings(swing_dir, force_pose=args.force_pose)

    tau_range = _parse_tau(args.tau)
    defaults = PipelineOptions()
    options = PipelineOptions(
        shots_dir=args.shots_dir,
        club=args.club,
        min_confidence=args.min_confidence,
        window_face_on=_parse_window(args.window_face_on),
        window_down_the_line=_parse_window(args.window_dtl),
        auto_window=not args.no_auto_window,
        render_video=not args.no_video,
        coaching=defaults.coaching and not args.no_coaching,
        force_pose=args.force_pose,
        force_ocr=args.force_ocr,
        skip_ocr=args.skip_ocr,
        tau_range=tau_range if tau_range is not None else defaults.tau_range,
    )

    outcome = analyze_swing_dir(swing_dir, options=options, log=print)
    if outcome.result is None:
        print(f"error: {outcome.error}", file=sys.stderr)
        return 2

    _print_report(outcome.result)
    print()
    return 1 if outcome.flagged else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
