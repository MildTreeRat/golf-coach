"""Physics cross-checks that catch a plausible-looking misread.

The failure this guards against is silent: OCR drops a digit, the number is still
well-formed, and it quietly skews every session average it lands in. The screen prints
redundant metrics, so the redundancy is the test.
"""

from __future__ import annotations

from conftest import SCREEN_2738, SCREEN_2739, build_screen

from golf_coach.launch_monitor.screen.parser import parse_screen
from golf_coach.launch_monitor.screen.validate import validate_parse


def _parse(rows, profile):
    return parse_screen(build_screen(rows), profile)


def test_a_clean_screen_passes_untouched(profile) -> None:
    parsed = _parse(SCREEN_2738, profile)

    validated = validate_parse(parsed)

    assert validated.needs_review is False
    assert validated.warnings == []
    assert validated.confidence == parsed.confidence


def test_an_implausible_but_self_consistent_shot_still_passes(profile) -> None:
    """IMG_2739 shows a 159.5 mph club speed and a 0.89 smash factor.

    Odd numbers, but the screen's own arithmetic checks out — so the *parse* is right, and
    that is all this validator judges. Flagging it would train the reviewer to ignore flags.
    """
    validated = validate_parse(_parse(SCREEN_2739, profile))

    assert validated.needs_review is False
    assert validated.warnings == []


def test_a_fade_stored_as_a_draw_is_caught_by_the_shape_word(profile) -> None:
    """The one misread the arithmetic checks cannot see: a correct magnitude, wrong sign.

    Every other cross-check here compares numbers, and a flipped sign leaves all of them
    satisfied — the shot is simply reported as curving the other way. HD Golf names the
    shape in the tile beside the axis, so the screen carries its own answer.
    """
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[1][4] = ("Shot Type", ["FADE"])
    rows[1][6] = ("Spin Axis", ["9.3 ° L"])  # a left tilt is a draw, not a fade

    validated = validate_parse(_parse(rows, profile))

    assert validated.needs_review is True
    assert any("disagrees with shot type" in w for w in validated.warnings)


def test_the_axis_and_the_shape_word_agreeing_is_silent(profile) -> None:
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[1][4] = ("Shot Type", ["FADE"])
    rows[1][6] = ("Spin Axis", ["-9.3 °"])  # printed inverted; stored +9.3 = fade

    validated = validate_parse(_parse(rows, profile))

    assert validated.values["spin_axis"] == 9.3
    assert validated.needs_review is False
    assert validated.warnings == []


def test_a_near_zero_axis_is_not_judged_against_the_shape_word(profile) -> None:
    """The device still prints 'SLIGHT FADE' for a ball that curved a foot."""
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[1][4] = ("Shot Type", ["SLIGHT", "FADE"])
    rows[1][6] = ("Spin Axis", ["0.4 ° L"])

    validated = validate_parse(_parse(rows, profile))

    assert validated.needs_review is False
    assert not any("disagrees" in w for w in validated.warnings)


def test_a_straight_shot_gives_the_axis_nothing_to_disagree_with(profile) -> None:
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[1][4] = ("Shot Type", ["STRAIGHT"])
    rows[1][6] = ("Spin Axis", ["-9.3 °"])

    validated = validate_parse(_parse(rows, profile))

    assert not any("disagrees" in w for w in validated.warnings)


def test_a_dropped_digit_in_carry_breaks_the_distance_identity(profile) -> None:
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[0][1] = ("Carry", ["28.1", "yds"])  # 128.1 misread

    validated = validate_parse(_parse(rows, profile))

    assert validated.needs_review is True
    assert any("carry + bounce & roll" in w for w in validated.warnings)
    assert validated.confidence < 0.9


def test_a_misread_speed_breaks_the_smash_identity(profile) -> None:
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[0][3] = ("Ball Speed", ["10.5", "mph"])  # 101.5 misread

    validated = validate_parse(_parse(rows, profile))

    assert validated.needs_review is True
    assert any("smash factor" in w for w in validated.warnings)


def test_an_inserted_digit_is_caught_by_range(profile) -> None:
    rows = [list(SCREEN_2738[0]), list(SCREEN_2738[1])]
    rows[0][3] = ("Ball Speed", ["1015", "mph"])

    validated = validate_parse(_parse(rows, profile))

    assert validated.needs_review is True
    assert any("ball_speed" in w and "plausible range" in w for w in validated.warnings)


def test_low_confidence_alone_flags_a_review(profile) -> None:
    """Coverage and consistency fail independently, so either one is enough."""
    partial = _parse([SCREEN_2738[0]], profile)  # top row only: nothing to cross-check

    validated = validate_parse(partial, min_confidence=0.9)

    assert validated.needs_review is True


def test_validation_does_not_mutate_the_parse(profile) -> None:
    parsed = _parse(SCREEN_2738, profile)
    before = len(parsed.warnings)

    validate_parse(parsed, min_confidence=1.0)

    assert parsed.needs_review is False
    assert len(parsed.warnings) == before
