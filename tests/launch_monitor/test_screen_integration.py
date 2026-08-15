"""End-to-end: two real photos of the simulator screen -> two correct `ShotData`. [ADR-014]

Everything else in this directory runs on synthetic `TextBox`es, which proves the parsing
rules but says nothing about whether OCR can read a photograph taken at an angle, under
ceiling glare, one of them upside-down. This test is the one that answers that — so it is
also the one to run after touching `preprocess.py`.

Skipped unless the `ocr` extra is installed *and* the reference photos are present, since
`data/` is gitignored and the base test suite must pass with no extras at all (ADR-008).
"""

from __future__ import annotations

import importlib.util
from datetime import UTC, datetime
from pathlib import Path

import pytest

from golf_coach.launch_monitor.screen.parser import parse_screen, to_shot_data
from golf_coach.launch_monitor.screen.profiles import load_profile
from golf_coach.launch_monitor.screen.store import hash_image
from golf_coach.launch_monitor.screen.validate import validate_parse

_SCREENS = Path(__file__).resolve().parents[2] / "data" / "raw" / "shot_screens"

_HAS_OCR = all(importlib.util.find_spec(m) is not None for m in ("cv2", "paddleocr"))

pytestmark = pytest.mark.skipif(
    not _HAS_OCR or not _SCREENS.exists(),
    reason="needs the `ocr` extra and the reference photos in data/raw/shot_screens/",
)

# Transcribed by eye from the photos. Distances in yards, speeds in mph, angles in signed
# degrees per the ShotData contract (+ = in-to-out / open / right).
EXPECTED = {
    "IMG_2738.jpeg": {
        "total_distance": 151.5,
        "carry_distance": 128.1,
        "bounce_and_roll": 23.4,
        "ball_speed": 101.5,
        "launch_angle": 10.2,
        "club_head_speed": 88.6,
        "club_path": -1.6,
        "club_face_angle": -1.1,
        "smash_factor": 1.15,
        "launch_direction": -2.6,
        "impact_position": "CENTER",
        "shot_type": "SLIGHT DRAW",
    },
    # Displays upside-down in viewers that ignore EXIF; cv2.imread honours the tag, so
    # this arrives upright. Orientation robustness is covered in test_preprocess.py.
    "IMG_2739.jpeg": {
        "total_distance": 226.3,
        "carry_distance": 195.9,
        "bounce_and_roll": 30.4,
        "ball_speed": 142.1,
        "launch_angle": 5.4,
        "club_head_speed": 159.5,
        "club_path": 6.8,
        "club_face_angle": -5.0,
        "smash_factor": 0.89,
        "launch_direction": 1.8,
        "impact_position": "CENTER",
        "shot_type": "DRAW",
    },
}


@pytest.fixture(scope="module")
def recognizer():
    from golf_coach.launch_monitor.screen.paddle import PaddleOCRRecognizer

    return PaddleOCRRecognizer()


@pytest.mark.parametrize("filename", sorted(EXPECTED))
def test_parses_a_real_photo(filename: str, recognizer) -> None:
    from golf_coach.launch_monitor.screen.preprocess import load_image, prepare_screen

    path = _SCREENS / filename
    if not path.exists():
        pytest.skip(f"{filename} not present")

    profile = load_profile("hd_golf")
    prepared = prepare_screen(load_image(path), recognizer, profile)
    parsed = validate_parse(parse_screen(prepared.boxes, profile))

    mismatches = {
        field: (parsed.values.get(field), expected)
        for field, expected in EXPECTED[filename].items()
        if parsed.values.get(field) != expected
    }
    assert not mismatches, (
        f"{filename}: field(s) misread (got, expected) = {mismatches}\n"
        f"raw tiles: {parsed.raw_fields}\nnotes: {prepared.notes + parsed.warnings}"
    )
    assert parsed.needs_review is False, parsed.warnings

    shot = to_shot_data(
        parsed,
        shot_id=f"integration-{filename}",
        session_id="integration",
        timestamp=datetime.now(tz=UTC),
        image_sha256=hash_image(path.read_bytes()),
        image_path=str(path),
    )
    assert shot.spin_rate is None  # HD Golf leaves the spin tile blank
    assert shot.provenance is not None and shot.provenance.parse_confidence > 0.6


@pytest.mark.parametrize("filename", sorted(EXPECTED))
def test_a_correct_parse_does_not_warn_about_the_title(filename: str, recognizer) -> None:
    """The pin the synthetic fixtures could not provide.

    This message was on every shot the repo had ever parsed, for two compounding reasons
    only a real photo shows: PaddleOCR returns the banner as one token (`SHOTDATA`) where
    the profile says `SHOT DATA`, and `rectify` crops these two photos to the tile grid,
    below where the banner sits at all. A warning that fires on every correct parse is
    worse than no warning — it is what teaches a reader, human or model, to skip them.
    """
    from golf_coach.launch_monitor.screen.preprocess import load_image, prepare_screen

    path = _SCREENS / filename
    if not path.exists():
        pytest.skip(f"{filename} not present")

    profile = load_profile("hd_golf")
    prepared = prepare_screen(load_image(path), recognizer, profile)
    parsed = validate_parse(parse_screen(prepared.boxes, profile))

    assert not [w for w in parsed.warnings if "title" in w], parsed.warnings


@pytest.mark.parametrize("filename", sorted(EXPECTED))
def test_the_screen_is_cropped_out_of_the_photo(filename: str, recognizer) -> None:
    """Both photos are off-axis with the room visible around the monitor.

    Without the crop, OCR runs on a downscaled full frame where the tile text is a
    fraction of its size and the background competes for attention — so `rectified`
    failing here is the leading indicator for a drop in parse accuracy.
    """
    from golf_coach.launch_monitor.screen.preprocess import load_image, prepare_screen

    path = _SCREENS / filename
    if not path.exists():
        pytest.skip(f"{filename} not present")

    prepared = prepare_screen(load_image(path), recognizer, load_profile("hd_golf"))

    assert prepared.rectified is True
    assert prepared.label_ratio >= 0.6
