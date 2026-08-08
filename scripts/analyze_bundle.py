"""Dev CLI: an assembled swing bundle in, the complete result out. [M7 Phase 4]

Usage:
    python scripts/analyze_bundle.py 2026-08-07/1          # SESSION/SWING in the bundle store
    python scripts/analyze_bundle.py path/to/swing_dir     # or the directory itself

    python scripts/analyze_bundle.py 2026-08-07/1 --list-swings          # what's in the clips
    python scripts/analyze_bundle.py 2026-08-07/1 --window-face-on 676:760

A *bundle* is what two phones and a screen photo produce: a face-on clip, a down-the-line clip
and a picture of the launch monitor, grouped into one swing by `storage.bundle_store`. This runs
the whole pipeline over one — pose per view, OCR the shot screen, score, rank tips, align the two
views — and leaves three artifacts in the swing directory:

    analysis.json    the complete result, for a results page to render
    aligned.mp4      the two views side by side, banners landing together
    <role>.keypoints.json   pose output per view, cached so a re-run is free

**Everything expensive is cached and content-addressed.** Pose on a 4K60 clip is minutes, so
keypoints are reused whenever their recorded `source_sha256` still matches the manifest's, and
the shot is looked up in the shot store by the photo's hash before any OCR is considered — which
means an already-imported shot attaches with no `ocr` extra installed at all.

Degradation is graceful and always narrated: a bundle with no down-the-line clip still scores
and still writes JSON, it just has no alignment and no video. Only a missing face-on clip is
fatal — that is the view every checkpoint is measured from.

Needs the `vision` extra to run pose or render, and `ocr` only to read a shot photo that is not
already in the store.

Exit codes: 0 clean, 1 produced a result with something flagged (shot needs review, a role
missing), 2 no result.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from golf_coach.analysis.alignment import DEFAULT_TAU_RANGE, pair_frames
from golf_coach.analysis.engine import analyze_swing_bundle
from golf_coach.analysis.phases import (
    CANDIDATE_MIN_RISE,
    candidate_downswings,
    select_swing,
    window_around,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.config import settings
from golf_coach.contracts.intent import ClubCategory, PracticeGoal
from golf_coach.contracts.keypoints import ClipMetadata, KeypointsFile
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import SwingBundleResult
from golf_coach.feedback.rules import build_feedback
from golf_coach.launch_monitor.screen.store import ShotStore
from golf_coach.storage.keypoints_io import load_keypoints, save_keypoints
from golf_coach.storage.manifest import Role, SwingManifest, load_manifest, manifest_path

_ANALYSIS_NAME = "analysis.json"
_ALIGNED_NAME = "aligned.mp4"

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

_VIEWS: tuple[tuple[Role, str], ...] = (
    (Role.FACE_ON, "face-on"),
    (Role.DOWN_THE_LINE, "down-the-line"),
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


def _parse_tau(value: str | None) -> tuple[float, float]:
    """`--tau -0.4:3.0` -> (-0.4, 3.0); the default is the swing and nothing else."""
    if value is None:
        return DEFAULT_TAU_RANGE
    try:
        low, _, high = value.partition(":")
        return float(low), float(high)
    except ValueError:
        raise argparse.ArgumentTypeError(f"tau must look like LO:HI, got {value!r}") from None


def _resolve_swing_dir(target: str, sessions_dir: Path) -> Path | None:
    """Accept either `SESSION/SWING` inside the store or a path to the swing directory."""
    direct = Path(target)
    if direct.is_dir() and manifest_path(direct).exists():
        return direct
    parts = target.replace("\\", "/").split("/")
    if len(parts) == 2:
        candidate = sessions_dir / parts[0] / parts[1]
        if manifest_path(candidate).exists():
            return candidate
    return None


def _keypoints_for(
    swing_dir: Path, manifest: SwingManifest, role: Role, *, force: bool
) -> KeypointsFile | None:
    """Pose output for one view, from cache when the clip behind it has not changed.

    The cache key is the clip's own sha256, recorded in the keypoints envelope by M7 Phase 1 and
    in the manifest by the upload route. Comparing them is what makes reuse safe rather than
    merely fast: a re-uploaded clip under the same role invalidates itself.
    """
    role_file = manifest.roles.get(role)
    if role_file is None:
        return None

    cache_path = swing_dir / f"{role.value}.keypoints.json"
    if cache_path.exists() and not force:
        cached = load_keypoints(cache_path)
        if cached.clip is not None and cached.clip.source_sha256 == role_file.content_sha256:
            print(f"  {role.value}: {len(cached.frames)} frames from cache")
            return cached
        print(f"  {role.value}: cached keypoints are for a different clip, re-running pose")

    video_path = swing_dir / role_file.filename
    if not video_path.exists():
        print(f"  {role.value}: {video_path.name} is missing from the swing directory")
        return None

    try:
        from golf_coach.capture.file import FileVideoSource
        from golf_coach.pose.estimator import estimate_pose
    except ImportError:
        print(f"  {role.value}: pose needs the vision extra — pip install -e '.[vision]'")
        return None

    print(f"  {role.value}: running pose over {role_file.original_filename} ...")
    # The generator goes straight into estimate_pose, so no decoded frame outlives its
    # iteration and the clip's resolution stops mattering to memory (M7 Phase 1).
    with FileVideoSource(video_path, camera_id=role.value) as source:
        fps, width, height = source.fps, source.width, source.height
        reported = source.frame_count
        frames = estimate_pose(source.frames())

    if not frames:
        print(f"  {role.value}: no frames decoded")
        return None
    if reported and reported != len(frames):
        # A file that opens cleanly and then decodes short produces a plausible-looking analysis
        # of an incomplete swing (docs/M7_TWO_PHONE_SPIKE.md, Q2).
        print(
            f"  {role.value}: warning - clip reports {reported} frames but decoded {len(frames)}",
            file=sys.stderr,
        )

    keypoints = KeypointsFile(
        clip=ClipMetadata(
            fps=fps,
            width=width or None,
            height=height or None,
            frame_count=len(frames),
            source_sha256=role_file.content_sha256,
        ),
        frames=frames,
    )
    save_keypoints(keypoints, cache_path)
    print(f"  {role.value}: {len(frames)} frames -> {cache_path.name}")
    return keypoints


def _shot_for(
    swing_dir: Path,
    manifest: SwingManifest,
    *,
    session_id: str,
    swing_id: str,
    shots_dir: Path,
    min_confidence: float,
    skip_ocr: bool,
    force: bool,
) -> tuple[ShotData | None, str | None]:
    """The launch-monitor shot for this swing. Returns `(shot, note_if_absent)`.

    Cache first, and that is not merely an optimisation: the manifest already records the
    photo's sha256 and `ShotStore` is keyed on exactly that, so a shot imported earlier — by
    this or by `import_shot_screens.py` — attaches on the base install without OCR, without
    OpenCV, and without opening the image.
    """
    role_file = manifest.roles.get(Role.SHOT_SCREEN)
    if role_file is None:
        return None, "no shot-screen photo in this bundle — no launch-monitor data to attach"

    store = ShotStore(shots_dir)
    if not force:
        cached = store.get(role_file.content_sha256)
        if cached is not None:
            print(f"  shot_screen: {cached.shot_id} from the shot store")
            return cached, None

    if skip_ocr:
        return None, "shot-screen OCR skipped by request — no launch-monitor data attached"

    image_path = swing_dir / role_file.filename
    if not image_path.exists():
        return None, f"shot-screen photo {role_file.filename} is missing from the swing directory"

    from golf_coach.launch_monitor.screen.importer import (
        MissingOCRExtra,
        build_recognizer,
        import_screen,
    )
    from golf_coach.launch_monitor.screen.profiles import load_profile

    try:
        recognizer = build_recognizer(settings.ocr_engine)
    except (MissingOCRExtra, ValueError) as exc:
        return None, f"no launch-monitor data attached: {exc}"

    print(f"  shot_screen: reading {role_file.original_filename} ...")
    status, shot = import_screen(
        image_path,
        recognizer=recognizer,
        profile=load_profile(settings.launch_monitor_profile),
        store=store,
        session_id=session_id,
        min_confidence=min_confidence,
        force=force,
        shot_id=f"{session_id}-{swing_id}",
    )
    if shot is None:
        return None, f"the shot screen could not be read ({status}) — no numbers attached"
    print(f"  shot_screen: {status}")
    return shot, None


def _auto_window(label: str, keypoints: KeypointsFile) -> tuple[int, int] | None:
    """`select_swing`'s pick for one view, narrating what it chose or why it declined."""
    fps = keypoints.clip.fps if keypoints.clip else None
    choice = select_swing(smooth_keypoints(keypoints.frames), fps=fps)
    if choice is None:
        why = "the keypoints file records no fps" if fps is None else "no plausible downswing"
        print(f"  {label}: could not pick a swing ({why}) — using the whole clip. Run "
              "--list-swings and pass a window if this clip holds more than one swing")
        return None
    print(f"  {label}: {choice.reason}")
    return choice.window


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


def _render(result: SwingBundleResult, swing_dir: Path, manifest: SwingManifest,
            views: dict[Role, KeypointsFile], tau_range: tuple[float, float]) -> None:
    """Write the side-by-side MP4, streaming both clips."""
    from contextlib import ExitStack

    alignment = result.alignment
    if alignment is None or alignment.a is None or alignment.b is None:
        print("\nNo aligned video: the two views could not be aligned.")
        return

    face_on, dtl = views[Role.FACE_ON], views[Role.DOWN_THE_LINE]
    schedule = pair_frames(
        alignment, len(face_on.frames), len(dtl.frames), reference="a", tau_range=tau_range
    )
    if not schedule:
        print("\nNo aligned video: the two clips share no overlapping swing time.")
        return

    try:
        from golf_coach.capture.file import FileVideoSource
        from golf_coach.pose.side_by_side import Panel, render_side_by_side
    except ImportError:
        print("\nNo aligned video: rendering needs the vision extra — pip install -e '.[vision]'")
        return

    out_path = swing_dir / _ALIGNED_NAME
    with ExitStack() as stack:
        sources = {
            role: stack.enter_context(FileVideoSource(swing_dir / manifest.roles[role].filename))
            for role, _ in _VIEWS
            if (swing_dir / manifest.roles[role].filename).exists()
        }
        written = render_side_by_side(
            out_path,
            schedule,
            Panel(face_on.frames, "face-on", face_on.clip,
                  sources[Role.FACE_ON].frames() if Role.FACE_ON in sources else ()),
            Panel(dtl.frames, "down-the-line", dtl.clip,
                  sources[Role.DOWN_THE_LINE].frames()
                  if Role.DOWN_THE_LINE in sources else ()),
            fps=alignment.a.anchors.fps or 60.0,
            quality=alignment.quality,
        )
    print(f"\nWrote {written} aligned frames -> {out_path}")


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

    swing_dir = _resolve_swing_dir(args.swing, args.sessions_dir)
    if swing_dir is None:
        print(
            f"error: no swing bundle at {args.swing!r} (looked in {args.sessions_dir})",
            file=sys.stderr,
        )
        return 2
    manifest = load_manifest(manifest_path(swing_dir))
    if manifest is None:
        print(f"error: {swing_dir} has no readable manifest", file=sys.stderr)
        return 2

    missing = manifest.missing_roles()
    print(f"\nBundle {manifest.session_id}/{manifest.swing_id}  [{manifest.status()}]")
    if missing:
        print(f"  missing: {', '.join(role.value for role in missing)}")

    print("\nPose:")
    views: dict[Role, KeypointsFile] = {}
    for role, label in _VIEWS:
        keypoints = _keypoints_for(swing_dir, manifest, role, force=args.force_pose)
        if keypoints is not None:
            views[role] = keypoints
        elif role is Role.FACE_ON:
            print(
                f"error: no usable {label} view — that is the view every checkpoint is "
                "measured from, so there is nothing to score",
                file=sys.stderr,
            )
            return 2

    if args.list_swings:
        for role, label in _VIEWS:
            if role in views:
                flag = "--window-face-on" if role is Role.FACE_ON else "--window-dtl"
                _print_swings(label, flag, views[role])
        print()
        return 0

    window_face_on = _parse_window(args.window_face_on)
    window_dtl = _parse_window(args.window_dtl)
    if not args.no_auto_window:
        print("\nSwing selection:")
        if window_face_on is None:
            window_face_on = _auto_window("face-on", views[Role.FACE_ON])
        if window_dtl is None and Role.DOWN_THE_LINE in views:
            window_dtl = _auto_window("down-the-line", views[Role.DOWN_THE_LINE])

    print("\nShot data:")
    shot, shot_note = _shot_for(
        swing_dir,
        manifest,
        session_id=manifest.session_id,
        swing_id=manifest.swing_id,
        shots_dir=args.shots_dir,
        min_confidence=args.min_confidence,
        skip_ocr=args.skip_ocr,
        force=args.force_ocr,
    )
    if shot_note:
        print(f"  {shot_note}")

    result = analyze_swing_bundle(
        swing_id=manifest.swing_id,
        session_id=manifest.session_id,
        face_on=views[Role.FACE_ON],
        down_the_line=views.get(Role.DOWN_THE_LINE),
        shot=shot,
        intent=PracticeGoal(club=args.club),
        face_on_window=window_face_on,
        down_the_line_window=window_dtl,
    )
    if shot_note:
        result.notes.append(shot_note)
    # `analysis` must not import `feedback` (ADR-008), so the shell joins the two — which is
    # what makes analysis.json the complete result instead of something to recompute.
    result.feedback = build_feedback(result.swing)

    _print_report(result)

    # The heavy streams are excluded deliberately: the keypoints already sit beside this file in
    # their own per-view JSON, and inlining several hundred frames of 33 landmarks would make
    # the artifact a results page has to fetch tens of megabytes for nothing.
    out_path = swing_dir / _ANALYSIS_NAME
    out_path.write_text(
        result.model_dump_json(indent=2, exclude={"swing": {"keypoints", "detections"}}),
        encoding="utf-8",
    )
    print(f"\nWrote {out_path}")

    if not args.no_video and Role.DOWN_THE_LINE in views:
        _render(result, swing_dir, manifest, views, _parse_tau(args.tau))

    print()
    flagged = shot is not None and shot.provenance is not None and shot.provenance.needs_review
    return 1 if (flagged or missing) else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
