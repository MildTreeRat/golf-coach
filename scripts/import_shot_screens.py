"""Dev CLI: read shot data off photos of the launch-monitor screen. [M3 / ADR-014]

Usage:
    python scripts/import_shot_screens.py data/raw/shot_screens
    python scripts/import_shot_screens.py shot.jpg --session 2026-08-04-range
    python scripts/import_shot_screens.py data/raw/shot_screens --dry-run

Point it at an image or a directory of them. Each photo is rectified, oriented, OCR'd,
parsed against the device profile, cross-checked, and written to the shot store — after
which `ScreenShotDataSource` serves it to the analysis engine with no extras installed.

Parsing is content-addressed, so re-running is cheap: photos already in the store are
skipped unless `--force`. Exits non-zero if any shot came out flagged for review, so this
is safe to put in front of an analysis step.

Requires the `ocr` extra: pip install -e '.[ocr]'
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
from datetime import UTC, datetime
from pathlib import Path

from golf_coach.config import settings
from golf_coach.contracts.shot import ShotData
from golf_coach.launch_monitor.screen.parser import parse_screen, to_shot_data
from golf_coach.launch_monitor.screen.profiles import available_profiles, load_profile
from golf_coach.launch_monitor.screen.store import ShotStore, hash_image
from golf_coach.launch_monitor.screen.validate import validate_parse

_IMAGE_SUFFIXES = {".jpg", ".jpeg", ".png", ".bmp", ".webp"}

# Metrics worth eyeballing in the summary table; the rest live in the stored JSON.
_SUMMARY_FIELDS = (
    ("carry", "carry_distance"),
    ("total", "total_distance"),
    ("ball", "ball_speed"),
    ("club", "club_head_speed"),
    ("smash", "smash_factor"),
    ("path", "club_path"),
    ("face", "club_face_angle"),
)


def _images_in(paths: list[Path]) -> list[Path]:
    found: list[Path] = []
    for path in paths:
        if path.is_dir():
            found.extend(
                child
                for child in sorted(path.iterdir())
                if child.suffix.lower() in _IMAGE_SUFFIXES
            )
        elif path.suffix.lower() in _IMAGE_SUFFIXES:
            found.append(path)
        else:
            print(f"warning: skipping {path} (not an image)", file=sys.stderr)
    return found


def _build_recognizer(engine: str):  # noqa: ANN202 - concrete type needs the `ocr` extra
    """Construct the OCR engine, failing fast and clearly when the extra is missing.

    The adapter loads its model lazily (constructing it downloads weights), so without
    this up-front check the missing dependency would surface as a traceback partway
    through the first image rather than as an instruction before any work starts.
    """
    if engine != "paddle":
        raise SystemExit(f"error: unknown OCR engine {engine!r} (known: paddle)")

    missing = [name for name in ("cv2", "paddleocr") if importlib.util.find_spec(name) is None]
    if missing:
        raise SystemExit(
            f"error: {' and '.join(missing)} not installed - importing screenshots needs "
            "the OCR extra.\n       pip install -e '.[ocr]'"
        )

    from golf_coach.launch_monitor.screen.paddle import PaddleOCRRecognizer

    return PaddleOCRRecognizer()


def _shot_timestamp(image_path: Path) -> datetime:
    """When the photo was taken, so stored shots keep the order they were hit in."""
    return datetime.fromtimestamp(image_path.stat().st_mtime, tz=UTC)


def _format_row(name: str, status: str, shot: ShotData | None) -> str:
    if shot is None:
        return f"  {name:<28} {status}"
    metrics = " ".join(
        f"{label}={value:g}"
        for label, attribute in _SUMMARY_FIELDS
        if (value := getattr(shot, attribute)) is not None
    )
    confidence = shot.provenance.parse_confidence if shot.provenance else 1.0
    flag = "  REVIEW" if shot.provenance and shot.provenance.needs_review else ""
    return f"  {name:<28} {status:<7} conf={confidence:.2f}{flag}\n      {metrics}"


def _import_one(
    image_path: Path,
    *,
    recognizer,  # noqa: ANN001 - concrete type needs the `ocr` extra
    profile,  # noqa: ANN001 - DeviceProfile, imported lazily alongside preprocess
    store: ShotStore,
    session_id: str,
    min_confidence: float,
    force: bool,
    dry_run: bool,
) -> tuple[str, ShotData | None]:
    from golf_coach.launch_monitor.screen.preprocess import load_image, prepare_screen

    data = image_path.read_bytes()
    digest = hash_image(data)

    if not force:
        cached = store.get(digest)
        if cached is not None:
            return "cached", cached

    prepared = prepare_screen(load_image(image_path), recognizer, profile)
    parsed = parse_screen(prepared.boxes, profile)
    parsed.warnings[:0] = prepared.notes  # preprocessing notes belong first, they explain the rest
    parsed = validate_parse(parsed, min_confidence=min_confidence)

    if parsed.is_empty:
        return "failed", None

    shot = to_shot_data(
        parsed,
        shot_id=f"{session_id}-{digest[:12]}",
        session_id=session_id,
        timestamp=_shot_timestamp(image_path),
        image_sha256=digest,
        image_path=str(image_path),
    )
    if not dry_run:
        store.put(shot)
    return ("parsed" if not dry_run else "dry-run"), shot


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="import_shot_screens",
        description="Parse launch-monitor shot data from photos of the SHOT DATA screen.",
    )
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        help="image files or directories (default: the configured shot_screens_dir)",
    )
    parser.add_argument("--session", help="session id to file these shots under")
    parser.add_argument(
        "--device",
        default=settings.launch_monitor_profile,
        help=f"launch-monitor profile (known: {', '.join(available_profiles())})",
    )
    parser.add_argument("--out", type=Path, default=settings.shots_dir, help="shot store directory")
    parser.add_argument(
        "--min-confidence",
        type=float,
        default=settings.ocr_min_confidence,
        help="below this a parse is flagged for review",
    )
    parser.add_argument("--force", action="store_true", help="re-parse images already stored")
    parser.add_argument("--dry-run", action="store_true", help="parse and report, write nothing")
    args = parser.parse_args(argv)

    paths = args.paths or [settings.shot_screens_dir]
    images = _images_in(paths)
    if not images:
        print(f"error: no images found in {', '.join(str(p) for p in paths)}", file=sys.stderr)
        return 1

    try:
        profile = load_profile(args.device)
    except KeyError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    session_id = args.session or f"screens-{datetime.now(tz=UTC):%Y-%m-%d}"
    store = ShotStore(args.out)
    recognizer = _build_recognizer(settings.ocr_engine)

    print(f"\nImporting {len(images)} image(s) as session {session_id!r} [{profile.device}]")
    flagged: list[ShotData] = []
    failed = 0

    for image_path in images:
        status, shot = _import_one(
            image_path,
            recognizer=recognizer,
            profile=profile,
            store=store,
            session_id=session_id,
            min_confidence=args.min_confidence,
            force=args.force,
            dry_run=args.dry_run,
        )
        print(_format_row(image_path.name, status, shot))
        if shot is None:
            failed += 1
        elif shot.provenance and shot.provenance.needs_review:
            flagged.append(shot)

    for shot in flagged:
        assert shot.provenance is not None
        print(f"\n  {shot.provenance.image_path} needs review:")
        for warning in shot.provenance.warnings:
            print(f"    - {warning}")

    if not args.dry_run:
        print(f"\nStore: {store.root}")
    if failed or flagged:
        print(f"\n{failed} unreadable, {len(flagged)} flagged for review.")
        return 1
    print(f"\n{len(images)} shot(s) imported cleanly.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
