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
import sys
from datetime import UTC, datetime
from pathlib import Path

from golf_coach.config import settings
from golf_coach.contracts.shot import ShotData
from golf_coach.launch_monitor.screen.importer import (
    MissingOCRExtra,
    build_recognizer,
    import_screen,
)
from golf_coach.launch_monitor.screen.profiles import available_profiles, load_profile
from golf_coach.launch_monitor.screen.store import ShotStore

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
    try:
        recognizer = build_recognizer(settings.ocr_engine)
    except (MissingOCRExtra, ValueError) as exc:
        raise SystemExit(f"error: {exc}") from None

    print(f"\nImporting {len(images)} image(s) as session {session_id!r} [{profile.device}]")
    flagged: list[ShotData] = []
    failed = 0

    for image_path in images:
        status, shot = import_screen(
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
