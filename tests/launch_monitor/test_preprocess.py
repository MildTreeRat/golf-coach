"""Rectification and orientation, without needing an OCR engine.

`preprocess` is the stage that decides whether OCR gets a clean, upright grid of tiles or
a skewed photo of a room, so it is worth testing on its own. A stub recognizer stands in
for the real engine: it "reads" the profile's labels only when the image is the right way
up, which is exactly the signal the rotation search votes on.

Needs the `vision` extra for OpenCV, not the `ocr` extra — the OCR engine is never called.
"""

from __future__ import annotations

import importlib.util

import pytest

from golf_coach.launch_monitor.screen.profiles import load_profile
from golf_coach.launch_monitor.screen.recognizer import TextBox

pytestmark = pytest.mark.skipif(
    importlib.util.find_spec("cv2") is None, reason="needs the `vision` extra (OpenCV)"
)

# A corner marker painted into the true top-left. After a rotation it is somewhere else,
# which is how the stub recognizer knows the image is not upright.
_MARKER = (0, 0, 255)


@pytest.fixture
def np():
    return pytest.importorskip("numpy")


@pytest.fixture
def cv2():
    return pytest.importorskip("cv2")


def _photo(np, cv2, *, screen_fraction: float = 0.6, rotate: int = 0):
    """A dark 'screen' rectangle on a light 'room' background, optionally rotated."""
    image = np.full((900, 1200, 3), 210, dtype=np.uint8)  # the room
    height, width = image.shape[:2]
    margin_x = int(width * (1 - screen_fraction) / 2)
    margin_y = int(height * (1 - screen_fraction) / 2)
    cv2.rectangle(
        image,
        (margin_x, margin_y),
        (width - margin_x, height - margin_y),
        (30, 30, 30),
        thickness=-1,
    )
    cv2.rectangle(image, (margin_x, margin_y), (margin_x + 40, margin_y + 40), _MARKER, -1)
    if rotate:
        image = cv2.rotate(
            image,
            {90: cv2.ROTATE_90_CLOCKWISE, 180: cv2.ROTATE_180, 270: cv2.ROTATE_90_COUNTERCLOCKWISE}[
                rotate
            ],
        )
    return image


class _OrientationSensitiveRecognizer:
    """Returns the profile's labels only when the marker is back in the top-left."""

    def __init__(self, profile) -> None:
        self._profile = profile
        self.calls = 0

    def recognize(self, image) -> list[TextBox]:
        self.calls += 1
        top_left = image[: image.shape[0] // 3, : image.shape[1] // 3]
        if not (top_left[..., 2] > 200).any():  # the red marker is not up here
            return []
        return [
            TextBox(field.label, 10.0 + index * 60, 10.0, 50.0, 12.0, 0.9)
            for index, field in enumerate(self._profile.stored_fields)
        ]


def test_finds_the_screen_outline_in_a_photo(np, cv2) -> None:
    from golf_coach.launch_monitor.screen.preprocess import find_screen_quad, warp_to_rect

    image = _photo(np, cv2)

    quad = find_screen_quad(image)

    assert quad is not None
    warped = warp_to_rect(image, quad)
    # The crop is the screen, not the room: the light background is gone.
    assert warped.mean() < image.mean()
    assert 0.4 < (warped.shape[1] / image.shape[1]) < 0.85


def test_a_blank_frame_yields_no_quad_rather_than_a_wrong_one(np, cv2) -> None:
    """No crop is recoverable; a fabricated crop warps the tiles into nonsense."""
    from golf_coach.launch_monitor.screen.preprocess import find_screen_quad

    assert find_screen_quad(np.full((600, 800, 3), 128, dtype=np.uint8)) is None


def test_a_contour_tracing_the_whole_frame_is_rejected(np, cv2) -> None:
    from golf_coach.launch_monitor.screen.preprocess import find_screen_quad

    # A "screen" filling 99% of the frame is indistinguishable from the photo border.
    assert find_screen_quad(_photo(np, cv2, screen_fraction=0.995)) is None


@pytest.mark.parametrize("rotation", [0, 90, 180, 270])
def test_orientation_is_recovered_from_the_content(np, cv2, rotation: int) -> None:
    from golf_coach.launch_monitor.screen.preprocess import prepare_screen

    profile = load_profile("hd_golf")
    recognizer = _OrientationSensitiveRecognizer(profile)

    prepared = prepare_screen(_photo(np, cv2, rotate=rotation), recognizer, profile)

    assert prepared.label_ratio == 1.0
    assert len(prepared.boxes) == len(profile.stored_fields)
    if rotation:
        assert any("rotated" in note for note in prepared.notes)


def test_the_upright_case_short_circuits_the_rotation_search(np, cv2) -> None:
    """An upright photo must not pay for four OCR passes."""
    from golf_coach.launch_monitor.screen.preprocess import prepare_screen

    profile = load_profile("hd_golf")
    recognizer = _OrientationSensitiveRecognizer(profile)

    prepare_screen(_photo(np, cv2), recognizer, profile)

    assert recognizer.calls == 1


def test_an_unreadable_screen_is_reported_not_hidden(np, cv2) -> None:
    from golf_coach.launch_monitor.screen.preprocess import prepare_screen

    class _Blind:
        def recognize(self, image) -> list[TextBox]:
            return []

    prepared = prepare_screen(_photo(np, cv2), _Blind(), load_profile("hd_golf"))

    assert prepared.label_ratio == 0.0
    assert any("legible" in note for note in prepared.notes)


# ------------------------------------------------------------------ decoding [HEIC]

# `load_image` is the boundary that decides whether a photo becomes a swing's launch-monitor
# numbers or an exception. It used to be one `cv2.imread`, and an ordinary iPhone photo — HEIC,
# the camera's default — came back as None and raised the same message a truncated file does.


def _heic_header() -> bytes:
    """The first bytes of an ISO-BMFF file whose major brand is `heic`. Not decodable, and not
    meant to be: this pins the *diagnosis*, which is what the old message got wrong."""
    return (0).to_bytes(4, "big") + b"ftypheic" + b"\x00" * 16


def test_is_heif_recognises_an_iphone_photo(tmp_path) -> None:
    from golf_coach.launch_monitor.screen.preprocess import _is_heif

    path = tmp_path / "shot.HEIC"
    path.write_bytes(_heic_header())

    assert _is_heif(path) is True


def test_is_heif_does_not_claim_a_jpeg(tmp_path) -> None:
    from golf_coach.launch_monitor.screen.preprocess import _is_heif

    path = tmp_path / "shot.jpg"
    path.write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 20)

    assert _is_heif(path) is False


def test_is_heif_never_raises_on_a_missing_file(tmp_path) -> None:
    """Advisory only — it runs on the failure path, where raising would replace the real error."""
    from golf_coach.launch_monitor.screen.preprocess import _is_heif

    assert _is_heif(tmp_path / "nope.HEIC") is False


def test_undecodable_heic_names_the_format_and_the_fix(tmp_path, monkeypatch) -> None:
    """The message a golfer in a bay actually has to act on. `cv2.imread` reports "no codec for
    this format" and "this file is corrupt" identically, and the repairs are nothing alike."""
    from golf_coach.launch_monitor.screen import preprocess

    monkeypatch.setattr(preprocess, "_load_via_pillow", lambda _: None)
    path = tmp_path / "shot.HEIC"
    path.write_bytes(_heic_header())

    with pytest.raises(OSError, match="HEIC/HEIF"):
        preprocess.load_image(path)


def test_undecodable_junk_does_not_blame_heic(tmp_path, monkeypatch) -> None:
    from golf_coach.launch_monitor.screen import preprocess

    monkeypatch.setattr(preprocess, "_load_via_pillow", lambda _: None)
    path = tmp_path / "shot.jpg"
    path.write_bytes(b"not an image at all")

    with pytest.raises(OSError, match="no installed decoder"):
        preprocess.load_image(path)


def test_pillow_decodes_what_opencv_declines(tmp_path, np, monkeypatch) -> None:
    """The fallback itself, with OpenCV's decoder forced to decline so the second one is what
    answers. BGR out, matching `cv2.imread` — the caller warps and OCRs it without knowing which
    decoder produced it."""
    from golf_coach.launch_monitor.screen import preprocess

    pytest.importorskip("PIL")
    import cv2

    source = np.zeros((8, 12, 3), dtype=np.uint8)
    source[:, :, 2] = 255  # pure red in BGR
    path = tmp_path / "shot.png"
    cv2.imwrite(str(path), source)

    monkeypatch.setattr(preprocess.cv2, "imread", lambda *a, **k: None)
    loaded = preprocess.load_image(path)

    assert loaded.shape == (8, 12, 3)
    assert loaded[0, 0].tolist() == [0, 0, 255]


def test_a_real_heic_round_trips_when_the_extra_is_installed(tmp_path, np) -> None:
    """End to end on the actual format, skipped where `pillow-heif` is not installed. This is the
    file that failed a whole swing's analysis: `shot_screen.*.HEIC`, straight off a camera roll."""
    pytest.importorskip("pillow_heif")
    import pillow_heif
    from PIL import Image

    from golf_coach.launch_monitor.screen.preprocess import load_image

    pillow_heif.register_heif_opener()
    path = tmp_path / "shot.HEIC"
    Image.fromarray(np.full((16, 24, 3), 200, dtype=np.uint8)).save(path, format="HEIF")

    loaded = load_image(path)

    assert loaded.shape == (16, 24, 3)
