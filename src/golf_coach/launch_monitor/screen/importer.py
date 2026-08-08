"""One photo of the SHOT DATA screen -> one stored `ShotData`. [ADR-014, M7 Phase 4]

The slow, extras-dependent half of the pipeline: rectify, OCR, parse, cross-check, cache. It
lives here rather than in `scripts/import_shot_screens.py` because there are now two entry
points into it — the bulk import CLI and the swing-bundle analysis, which has one shot photo
sitting beside two clips — and a second copy of this is exactly how a sign convention or a
confidence threshold drifts between them.

**Importing stays separate from reading.** `ScreenShotDataSource` serves what is already in the
store with no extras at all; only getting a *new* photo into the store needs OpenCV and an OCR
engine. Importing this module is safe on the base install — the heavy imports are inside the
functions that need them, which is also what makes the missing-extra message an instruction
rather than a traceback from halfway through the first image.

Parsing is content-addressed, so re-running is cheap: a photo already in the store is served
from cache under any filename, and re-photographing a *different* shot never collides.
"""

from __future__ import annotations

import importlib.util
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING

from golf_coach.contracts.shot import ShotData
from golf_coach.launch_monitor.screen.parser import parse_screen, to_shot_data
from golf_coach.launch_monitor.screen.store import ShotStore, hash_image
from golf_coach.launch_monitor.screen.validate import validate_parse

if TYPE_CHECKING:
    from golf_coach.launch_monitor.screen.profiles import DeviceProfile
    from golf_coach.launch_monitor.screen.recognizer import TextRecognizer

_ENGINES = ("paddle",)


class MissingOCRExtra(RuntimeError):
    """The `ocr` extra is not installed, so a new photo cannot be read.

    Raised rather than exited so callers choose what it means: the bulk CLI turns it into a
    `SystemExit` with an install instruction, while bundle analysis carries on without the shot
    and says so — a swing is still worth scoring when the launch-monitor numbers are missing.
    """


def build_recognizer(engine: str = "paddle") -> TextRecognizer:
    """Construct the OCR engine, failing fast and clearly when the extra is missing.

    The adapter loads its model lazily (constructing it downloads weights), so without this
    up-front check the missing dependency would surface as a traceback partway through the first
    image rather than as an instruction before any work starts.
    """
    if engine not in _ENGINES:
        raise ValueError(f"unknown OCR engine {engine!r} (known: {', '.join(_ENGINES)})")

    missing = [name for name in ("cv2", "paddleocr") if importlib.util.find_spec(name) is None]
    if missing:
        raise MissingOCRExtra(
            f"{' and '.join(missing)} not installed - reading a shot screen needs the OCR "
            "extra.\n       pip install -e '.[ocr]'"
        )

    from golf_coach.launch_monitor.screen.paddle import PaddleOCRRecognizer

    return PaddleOCRRecognizer()


def screen_timestamp(image_path: Path) -> datetime:
    """When the photo was taken, so stored shots keep the order they were hit in."""
    return datetime.fromtimestamp(image_path.stat().st_mtime, tz=UTC)


def import_screen(
    image_path: Path,
    *,
    recognizer: TextRecognizer,
    profile: DeviceProfile,
    store: ShotStore,
    session_id: str,
    min_confidence: float,
    force: bool = False,
    dry_run: bool = False,
    shot_id: str | None = None,
) -> tuple[str, ShotData | None]:
    """Read one screen photo into a `ShotData`, caching by the image's content hash.

    Returns `(status, shot)` where status is one of `cached` / `parsed` / `dry-run` / `failed`.
    A `failed` parse returns None rather than a half-populated shot: a wrong number is worse
    than a missing one, which is the whole thesis of ADR-014.

    `shot_id` overrides the generated id, so a bundle can file the shot under the swing it
    belongs to instead of a bare digest.
    """
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
        shot_id=shot_id or f"{session_id}-{digest[:12]}",
        session_id=session_id,
        timestamp=screen_timestamp(image_path),
        image_sha256=digest,
        image_path=str(image_path),
    )
    if not dry_run:
        store.put(shot)
    return ("parsed" if not dry_run else "dry-run"), shot
