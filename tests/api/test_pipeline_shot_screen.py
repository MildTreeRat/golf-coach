"""The shot screen is the optional third of a bundle, and this pins that it stays optional.

`api/pipeline` promises graceful degradation in its own docstring — "a shot that could not be
read" is meant to become a line in `result.notes`, not an exception. `_shot_for` did not keep
that promise: it guarded `build_recognizer` and left `import_screen` bare, so a HEIC photo raised
`OSError` out of `preprocess.load_image` and took the whole analysis down with it. Pose had
already run on both clips and every mechanics checkpoint had already scored; all of it was
discarded over a photo.

These run on the base install. `_shot_for` is exercised directly with the OCR seam stubbed, which
is the point — the failure being pinned belongs to third-party decoders and OCR engines, so the
test must not need them present to reproduce it.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.api.pipeline import PipelineOptions, _shot_for
from golf_coach.storage.manifest import Role, RoleFile, SwingManifest


@pytest.fixture
def bundle(tmp_path):
    """One swing directory with a shot-screen file in it, and the manifest that names it."""
    swing_dir = tmp_path / "2026-08-23" / "1"
    swing_dir.mkdir(parents=True)
    (swing_dir / "shot_screen.abc123.HEIC").write_bytes(b"not really an image")
    now = datetime.now(tz=UTC)
    manifest = SwingManifest(
        swing_id="1",
        session_id="2026-08-23",
        created_at=now,
        updated_at=now,
        roles={
            Role.SHOT_SCREEN: RoleFile(
                role=Role.SHOT_SCREEN,
                filename="shot_screen.abc123.HEIC",
                content_sha256="abc123",
                original_filename="shot-1.HEIC",
                content_type="application/octet-stream",
                size_bytes=19,
                received_at=now,
            )
        },
    )
    return swing_dir, manifest, PipelineOptions(shots_dir=tmp_path / "shots")


def _stub_importer(monkeypatch, raiser):
    """Stand in for the `ocr` extra. `_shot_for` imports these lazily from the importer module,
    so patching the module's attributes is what the function will actually pick up."""
    from golf_coach.launch_monitor.screen import importer

    monkeypatch.setattr(importer, "build_recognizer", lambda _: object(), raising=False)
    monkeypatch.setattr(importer, "import_screen", raiser, raising=False)


@pytest.mark.parametrize(
    "error",
    [
        OSError("Could not read image: shot_screen.abc123.HEIC"),
        ValueError("bad profile geometry"),
        RuntimeError("the OCR engine died"),
    ],
)
def test_an_unreadable_shot_screen_is_a_note_and_not_an_exception(bundle, monkeypatch, error):
    """Deliberately broad, and these three are why: the exceptions come from OpenCV, Pillow and
    PaddleOCR, so an unanticipated type is exactly the bug. Losing the swing is never the answer.
    """
    swing_dir, manifest, options = bundle

    def raiser(*args, **kwargs):
        raise error

    _stub_importer(monkeypatch, raiser)

    shot, note = _shot_for(swing_dir, manifest, options=options, log=lambda _: None)

    assert shot is None
    assert "could not be read" in note
    # The reason travels with the note. `notes` is part of `analysis.json`, and a reader of the
    # result is the only person who can act on "your shot screen was a HEIC".
    assert str(error) in note


def test_a_readable_shot_screen_still_attaches(bundle, monkeypatch):
    """The guard must not swallow the success path — the try wraps one call, not the function."""
    from golf_coach.contracts.shot import ShotData

    swing_dir, manifest, options = bundle
    shot = ShotData(shot_id="2026-08-23-1", session_id="2026-08-23", timestamp=datetime.now(tz=UTC))
    _stub_importer(monkeypatch, lambda *a, **k: ("parsed", shot))

    result, note = _shot_for(swing_dir, manifest, options=options, log=lambda _: None)

    assert result is shot
    assert note is None


def test_a_failed_parse_is_still_reported_as_a_parse(bundle, monkeypatch):
    """`import_screen` returning `(status, None)` is a *readable* photo it could not parse, and
    it keeps its own message — a wrong number is worse than a missing one (ADR-014)."""
    swing_dir, manifest, options = bundle
    _stub_importer(monkeypatch, lambda *a, **k: ("failed", None))

    shot, note = _shot_for(swing_dir, manifest, options=options, log=lambda _: None)

    assert shot is None
    assert "(failed)" in note
