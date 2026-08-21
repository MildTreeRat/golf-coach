"""Golfer registry — identity stability and the two silent-drift guards.

The properties under test here are the ones whose failure mode is invisible: a name that resolves
to two golfers halves a career baseline without erroring, and a flipped handedness inverts every
signed metric without erroring. Both are pinned rather than trusted.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.contracts.golfer import Golfer, Handedness, slugify
from golf_coach.storage.golfer_store import GolferStore


@pytest.fixture
def golfers(tmp_path):
    return GolferStore(tmp_path / "golfers")


# --------------------------------------------------------------------------- slug


@pytest.mark.parametrize(
    ("name", "expected"),
    [
        ("Aaron", "aaron"),
        ("  Aaron  ", "aaron"),
        ("AARON", "aaron"),
        ("Aaron Sierra", "aaron-sierra"),
        ("Aaron  Sierra", "aaron-sierra"),
        ("O'Brien", "o-brien"),
        ("Player 2", "player-2"),
        ("!!!", ""),
        ("", ""),
        # Accents fold to the bare letter rather than to a separator. Without this "María" and
        # "Maria" are two golfers with half a history each, which nothing downstream can detect.
        ("María", "maria"),
        ("Maria", "maria"),
        ("Björn", "bjorn"),
        ("Éamon", "eamon"),
    ],
)
def test_slugify_folds_names_to_a_stable_id(name, expected) -> None:
    assert slugify(name) == expected


def test_accented_and_unaccented_spellings_are_the_same_golfer(golfers) -> None:
    """`index.html` mirrors `slugify` including this pass; parity is checked there by hand."""
    first = golfers.get_or_create("María", Handedness.RIGHT)

    assert golfers.get_or_create("Maria", Handedness.RIGHT).player_id == first.player_id
    assert len(golfers.list_all()) == 1


@pytest.mark.parametrize("bad_id", ["Aaron Sierra", "Aaron", "aaron sierra", "../aaron", "-aaron"])
def test_player_id_must_be_a_slug(bad_id) -> None:
    """The id becomes a filename and a URL path segment; the model refuses anything else."""
    with pytest.raises(ValueError):
        Golfer(
            player_id=bad_id,
            display_name="Aaron",
            handedness=Handedness.RIGHT,
            created_at=datetime(2026, 8, 12, tzinfo=UTC),
        )


def test_a_well_formed_golfer_round_trips() -> None:
    golfer = Golfer(
        player_id="aaron-sierra",
        display_name="Aaron Sierra",
        handedness=Handedness.LEFT,
        created_at=datetime(2026, 8, 12, tzinfo=UTC),
    )

    assert Golfer.model_validate_json(golfer.model_dump_json()) == golfer


# --------------------------------------------------------------------------- registry


def test_get_or_create_creates_then_returns_the_same_golfer(golfers) -> None:
    created = golfers.get_or_create("Aaron", Handedness.RIGHT)
    again = golfers.get_or_create("Aaron", Handedness.RIGHT)

    assert created.player_id == "aaron"
    assert again == created
    assert len(golfers.list_all()) == 1


def test_a_retyped_name_resolves_to_the_existing_golfer(golfers) -> None:
    """The identity-stability property. Two spellings must not become two career baselines."""
    first = golfers.get_or_create("Aaron", Handedness.RIGHT)
    second = golfers.get_or_create("  aaron ", Handedness.RIGHT)

    assert second.player_id == first.player_id
    assert len(golfers.list_all()) == 1


def test_handedness_of_a_known_golfer_is_never_overwritten(golfers) -> None:
    """A stray form submission must not silently invert every signed metric's frame of reference."""
    golfers.get_or_create("Aaron", Handedness.RIGHT)

    resolved = golfers.get_or_create("aaron", Handedness.LEFT)

    assert resolved.handedness is Handedness.RIGHT
    assert golfers.get("aaron").handedness is Handedness.RIGHT


def test_display_name_keeps_the_original_spelling(golfers) -> None:
    golfers.get_or_create("Aaron", Handedness.RIGHT)

    assert golfers.get_or_create("aaron", Handedness.RIGHT).display_name == "Aaron"


def test_a_nameless_golfer_is_refused_rather_than_given_a_placeholder(golfers) -> None:
    with pytest.raises(ValueError):
        golfers.get_or_create("!!!", Handedness.RIGHT)
    assert golfers.list_all() == []


def test_unknown_and_corrupt_records_read_as_absent(golfers) -> None:
    assert golfers.get("nobody") is None

    golfers.get_or_create("Aaron", Handedness.RIGHT)
    golfers.path_for("aaron").write_text("{not json", encoding="utf-8")

    assert golfers.get("aaron") is None
    assert golfers.list_all() == []


def test_list_all_is_sorted_by_display_name(golfers) -> None:
    golfers.get_or_create("Zoe", Handedness.LEFT)
    golfers.get_or_create("aaron", Handedness.RIGHT)

    assert [g.display_name for g in golfers.list_all()] == ["aaron", "Zoe"]
