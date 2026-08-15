"""Parsing a launch-monitor screen into ShotData fields.

The cases that matter are the ones where a wrong answer still looks like a right one:
a sign flipped because the direction word was ignored, a `---` read as zero, or a value
claimed by the tile next door. Those are what these tests pin down.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest
from conftest import SCREEN_2738, SCREEN_2739, build_screen

from golf_coach.contracts.shot import ShotSource
from golf_coach.launch_monitor.screen.parser import parse_screen, to_shot_data
from golf_coach.launch_monitor.screen.recognizer import TextBox


def test_reads_every_tile_of_a_real_screen(profile) -> None:
    parsed = parse_screen(build_screen(SCREEN_2738), profile)

    assert parsed.values == {
        "total_distance": 151.5,
        "carry_distance": 128.1,
        "bounce_and_roll": 23.4,
        "ball_speed": 101.5,
        "launch_angle": 10.2,
        "club_head_speed": 88.6,
        "club_path": -1.6,  # printed "1.6 ° O>I" — out-to-in is negative
        "club_face_angle": -1.1,  # printed "1.1 ° Closed" — closed is negative
        "smash_factor": 1.15,
        "impact_position": "CENTER",
        "shot_type": "SLIGHT DRAW",  # two stacked lines, rejoined
        "launch_direction": -2.6,  # printed "2.6 ° L" — left is negative
    }
    assert parsed.warnings == []
    assert parsed.confidence > 0.9


def test_reads_the_opposite_signs_on_the_second_screen(profile) -> None:
    parsed = parse_screen(build_screen(SCREEN_2739), profile)

    assert parsed.values["club_path"] == 6.8  # "6.8 ° I>O" — in-to-out is positive
    assert parsed.values["club_face_angle"] == -5.0  # "5.0 ° Closed"
    assert parsed.values["launch_direction"] == 1.8  # "1.8 ° R" — right is positive
    assert parsed.values["shot_type"] == "DRAW"
    assert parsed.warnings == []


def test_blank_tiles_become_none_not_zero(profile) -> None:
    """`---` means the device didn't measure it. Zero would be a fabricated reading."""
    parsed = parse_screen(build_screen(SCREEN_2738), profile)

    assert "spin_rate" not in parsed.values
    assert "spin_axis" not in parsed.values
    assert parsed.raw_fields["Spin"] == "---"  # ...but we still record what was on screen

    shot = to_shot_data(
        parsed, shot_id="s1", session_id="sess", timestamp=datetime.now(tz=UTC)
    )
    assert shot.spin_rate is None
    assert shot.spin_axis is None


def test_raw_text_is_kept_for_every_tile(profile) -> None:
    """The audit trail: what was on screen, before any sign or unit interpretation."""
    parsed = parse_screen(build_screen(SCREEN_2738), profile)

    assert parsed.raw_fields["Club Path"] == "1.6 ° O>I"
    assert parsed.raw_fields["Club Face Angle"] == "1.1 ° Closed"
    assert parsed.raw_fields["Carry"] == "128.1 yds"


def test_split_value_boxes_are_rejoined_in_reading_order(profile) -> None:
    """OCR often splits '1.6 ° O>I' into separate boxes; the cell must reassemble it."""
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[0][6] = ("Club Path", ["1.6"])
    boxes = build_screen(rows)
    # Drop the degree/qualifier onto the same line, to the right of the number.
    number = next(b for b in boxes if b.text == "1.6")
    boxes.append(TextBox("° O>I", number.right + 4, number.y, 40.0, number.height, 0.9))

    parsed = parse_screen(boxes, profile)

    assert parsed.values["club_path"] == -1.6


def test_missing_direction_word_is_reported_not_guessed(profile) -> None:
    """A magnitude with no qualifier is ambiguous — say so rather than assume a sign."""
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[0][6] = ("Club Path", ["1.6 °"])

    parsed = parse_screen(build_screen(rows), profile)

    assert parsed.values["club_path"] == 1.6
    assert any("Club Path" in w and "sign unknown" in w for w in parsed.warnings)


def test_a_printed_sign_is_a_known_sign_not_a_missing_word(profile) -> None:
    """HD Golf prints the spin axis signed, and its polarity is the contract's inverted.

    `-9.3 °` on screen is a fade, and the contract is `+ = fade`, so the stored value is
    +9.3. The sign came from the digits and the profile's stated convention, so there is
    nothing to warn about — the old message called this "sign unknown" on every shot.
    """
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[1][6] = ("Spin Axis", ["-9.3 °"])

    parsed = parse_screen(build_screen(rows), profile)

    assert parsed.values["spin_axis"] == 9.3
    assert parsed.warnings == []


def test_a_direction_word_still_beats_a_printed_sign(profile) -> None:
    """If the device ever prints both, the word states the convention and wins."""
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[1][6] = ("Spin Axis", ["-9.3 ° R"])

    parsed = parse_screen(build_screen(rows), profile)

    assert parsed.values["spin_axis"] == 9.3  # R = right = fade = positive
    assert parsed.warnings == []


def test_an_unsigned_number_with_no_word_is_still_reported(profile) -> None:
    """The `printed_sign` convention must not swallow the genuinely ambiguous case."""
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[1][6] = ("Spin Axis", ["9.3 °"])

    parsed = parse_screen(build_screen(rows), profile)

    assert parsed.values["spin_axis"] == 9.3
    assert any("Spin Axis" in w and "sign unknown" in w for w in parsed.warnings)


def test_the_title_is_matched_without_its_spaces(profile) -> None:
    """OCR returns the tracked-out banner as `SHOTDATA`; the profile says `SHOT DATA`."""
    parsed = parse_screen(build_screen(SCREEN_2738, title="SHOTDATA Aaron"), profile)

    assert parsed.warnings == []


def test_a_title_cropped_out_of_frame_is_not_reported_as_missing(profile) -> None:
    """`rectify` can crop to the tile grid, leaving the banner outside the frame.

    Absence of a title from a frame that starts at the first tile row is a fact about the
    crop, not about the page — and warning on it means warning on every correctly-parsed
    photo, which is how the repo's real shots all came to carry the message.
    """
    boxes = [b for b in build_screen(SCREEN_2738) if "SHOT" not in b.text.upper()]

    parsed = parse_screen(boxes, profile)

    assert parsed.values["carry_distance"] == 128.1  # the page parsed fine
    assert not any("title" in w for w in parsed.warnings)


def test_a_missing_title_is_reported_when_we_could_have_seen_it(profile) -> None:
    """The check still earns its keep: text above the grid, but not the title."""
    boxes = build_screen(SCREEN_2738, title="PUTTING ANALYSIS")

    parsed = parse_screen(boxes, profile)

    assert any("title" in w and "right page" in w for w in parsed.warnings)


def test_unreadable_screen_yields_nothing_rather_than_garbage(profile) -> None:
    boxes = [TextBox("BALL TRAJECTORY", 0.0, 0.0, 200.0, 20.0, 0.9)]

    parsed = parse_screen(boxes, profile)

    assert parsed.is_empty
    assert parsed.confidence == 0.0
    assert any("no" in w and "labels recognized" in w for w in parsed.warnings)


def test_partial_screen_lowers_confidence(profile) -> None:
    """Half a screen parses, but must not present itself as trustworthy."""
    full = parse_screen(build_screen(SCREEN_2738), profile)
    half = parse_screen(build_screen([SCREEN_2738[0]]), profile)

    assert half.values["carry_distance"] == 128.1
    assert half.confidence < full.confidence
    assert any("Smash Factor" in w for w in half.warnings)


@pytest.mark.parametrize("screen", [SCREEN_2738, SCREEN_2739])
def test_to_shot_data_carries_the_parse_provenance(screen, profile) -> None:
    parsed = parse_screen(build_screen(screen), profile)

    shot = to_shot_data(
        parsed,
        shot_id="shot-1",
        session_id="range-2026-08-04",
        timestamp=datetime(2026, 8, 4, 12, 0, tzinfo=UTC),
        image_sha256="abc123",
        image_path="data/raw/shot_screens/IMG.jpeg",
    )

    assert shot.source is ShotSource.SCREEN
    assert shot.provenance is not None
    assert shot.provenance.device == "hd_golf"
    assert shot.provenance.image_sha256 == "abc123"
    assert shot.provenance.raw_fields["Carry"].startswith(("128.1", "195.9"))
    # A ShotData built this way must survive the contract seam intact.
    assert shot.model_validate_json(shot.model_dump_json()) == shot
