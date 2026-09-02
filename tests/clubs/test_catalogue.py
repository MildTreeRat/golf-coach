"""The catalogue: one key per club however it is spelled, and no refusal that loses a row.

Every pin here guards a failure that is invisible in normal use, because a cache that quietly does
nothing looks exactly like a cache that was never asked:

- a key that splits on punctuation — "T150" and "T-150" as two rows — never raises. The catalogue
  simply misses, the model is called again, and a second row lands beside the first. That is
  `slugify`'s founding failure (one golfer, two baselines) with a club in place of a name;
- a `remember` that overwrites a catalogue it could not read replaces every checked row with the
  one it was asked to add, and the loss is only visible to someone who remembers what was in there;
- `exclude_defaults` on the way to disk is right only while the defaults are blanks: the day it
  drops a real `0.0` — a blade's zero offset — the row reads as "unknown" and the golfer's club
  gets re-looked-up forever;
- provenance rewritten to `"catalogue"` on the way out would make an LLM's recollection
  indistinguishable from a number typed off the manufacturer's page, which is the one distinction
  ADR-026 §7 asks this file to keep.

The packaged catalogue is patched away for every test by an autouse fixture, so nothing here can
write into the committed file — except the one test that deliberately reads it.
"""

from __future__ import annotations

import json
from collections.abc import Iterator
from datetime import UTC, datetime
from pathlib import Path

import pytest

from golf_coach.clubs import catalogue
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.club_spec import ClubSpec, ShaftFlex, ShaftMaterial, SpecProvenance

_WHEN = datetime(2026, 8, 31, 12, 0, tzinfo=UTC)
_FROM_MODEL = SpecProvenance(source="llm:claude-opus-5", retrieved_at=_WHEN)


@pytest.fixture(autouse=True)
def catalogue_file(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Iterator[Path]:
    """Point the catalogue at a throwaway file, for every test in this module.

    Autouse rather than opt-in: `remember` writes beside its own module, so a test that forgot the
    fixture would edit the repo's committed catalogue and pass while doing it. The cost of that
    mistake is a dirty working tree nobody attributes to a test run, which is worth one line here.
    """
    path = tmp_path / "club_catalogue.json"
    monkeypatch.setattr(catalogue, "_catalogue_path", lambda: path)
    yield path


def _spec(**overrides: object) -> ClubSpec:
    """A T150 7 iron, and the identity fields are the only ones that matter to most pins."""
    fields: dict[str, object] = {
        "club": ClubId.SEVEN_IRON,
        "make": "Titleist",
        "model": "T150",
        "model_year": 2023,
        "loft_deg": 30.5,
    }
    fields.update(overrides)
    return ClubSpec.model_validate(fields)


# ------------------------------------------------------------------------------- the key


@pytest.mark.parametrize("model", ["T150", "T-150", "t 150", "  t150  ", "T.150"])
def test_every_spelling_of_a_model_is_one_key(model: str) -> None:
    """The catalogue's founding pin, and it is `slugify`'s argument with a club in the name slot.

    A manufacturer writes "T150", a retailer writes "T-150" and a golfer types "t 150". If those
    are three keys, the third lookup of one club is still the first: three model calls, three rows,
    and no symptom anywhere — the catalogue reports a miss, which is what a catalogue is allowed
    to do.
    """
    assert catalogue.catalogue_key("Titleist", model, 2023, ClubId.SEVEN_IRON) == (
        "titleist/t150/2023/7i"
    )


@pytest.mark.parametrize("make", ["TaylorMade", "Taylor Made", "taylor-made", "TAYLORMADE"])
def test_a_make_written_two_ways_is_one_key(make: str) -> None:
    """The same fold on the make, where the space is the spelling people actually disagree on."""
    key = catalogue.catalogue_key(make, "Stealth 2", None, ClubId.DRIVER)
    assert key == "taylormade/stealth2//driver"


def test_a_different_year_is_a_different_key() -> None:
    """Makers reuse a model name across generations with different lofts.

    Two `P790` 7 irons four years apart are two specifications, so they are two rows. Collapsing
    them would serve one generation's lofts for the other and nothing downstream could tell.
    """
    keys = {
        catalogue.catalogue_key("TaylorMade", "P790", year, ClubId.SEVEN_IRON)
        for year in (2019, 2023, None)
    }
    assert len(keys) == 3


def test_a_different_slot_is_a_different_key() -> None:
    """The unit is a slot of a model, not a model (ADR-026 §2) — a set has a loft per slot."""
    seven = catalogue.catalogue_key("Titleist", "T150", 2023, ClubId.SEVEN_IRON)
    eight = catalogue.catalogue_key("Titleist", "T150", 2023, ClubId.EIGHT_IRON)
    assert seven != eight


# ------------------------------------------------------------------- remember and read back


def test_a_remembered_spec_comes_back_through_lookup(catalogue_file: Path) -> None:
    """The round trip the whole phase exists for: `remember` -> `load_catalogue` -> `lookup`."""
    spec = _spec(shaft_material=ShaftMaterial.STEEL, shaft_flex=ShaftFlex.STIFF)

    key = catalogue.remember(spec, _FROM_MODEL)

    assert key == "titleist/t150/2023/7i"
    assert catalogue_file.exists()
    assert set(catalogue.load_catalogue()) == {key}

    row = catalogue.lookup("titleist", "T-150", 2023, ClubId.SEVEN_IRON)
    assert row is not None
    assert row.spec == spec
    assert row.provenance == _FROM_MODEL


def test_the_provenance_is_not_rewritten_on_the_way_out(catalogue_file: Path) -> None:
    """A row keeps the provenance of whatever produced it (ADR-026 §7).

    Stamping `"catalogue"` on it would be true and useless: the catalogue is where an answer was
    kept, not where it came from, and a reader that cannot tell a model's recollection from a
    number typed off the manufacturer's page has to distrust both equally.
    """
    catalogue.remember(_spec(), _FROM_MODEL)

    row = catalogue.lookup("Titleist", "T150", 2023, ClubId.SEVEN_IRON)
    assert row is not None
    assert row.provenance.source == "llm:claude-opus-5"


def test_a_blank_field_stays_blank_and_a_real_zero_survives(catalogue_file: Path) -> None:
    """`exclude_defaults` is safe only because every default here is the blank.

    A `lie_deg` nobody supplied is absent from the file and reads back `None`, which is the same
    statement written shorter. A `0.0` offset is a *measured fact about a blade* and must survive
    — if it were ever dropped, the club would read as unspecified and be looked up forever.
    """
    catalogue.remember(_spec(lie_deg=None, offset_mm=0.0), _FROM_MODEL)

    written = json.loads(catalogue_file.read_text(encoding="utf-8"))
    assert "lie_deg" not in written["clubs"][0]["spec"]
    assert written["clubs"][0]["spec"]["offset_mm"] == 0.0

    row = catalogue.lookup("Titleist", "T150", 2023, ClubId.SEVEN_IRON)
    assert row is not None
    assert row.spec.lie_deg is None
    assert row.spec.offset_mm == 0.0


def test_remembering_the_same_club_twice_leaves_one_row(catalogue_file: Path) -> None:
    """A corrected spec supersedes the one it corrects rather than landing beside it.

    Appending would leave the catalogue with two answers for one club and no rule about which
    wins, which is the state a cache is least able to explain to whoever reads the diff.
    """
    catalogue.remember(_spec(loft_deg=30.5), _FROM_MODEL)
    catalogue.remember(_spec(loft_deg=30.0), _FROM_MODEL)

    written = json.loads(catalogue_file.read_text(encoding="utf-8"))
    assert len(written["clubs"]) == 1
    assert written["clubs"][0]["spec"]["loft_deg"] == 30.0


def test_rows_are_written_in_key_order(catalogue_file: Path) -> None:
    """This is committed data, and a diff over it is how anyone reviews what was learned.

    Insertion order would reshuffle the file on every save and make a two-line change unreadable.
    """
    for club in (ClubId.NINE_IRON, ClubId.SEVEN_IRON, ClubId.EIGHT_IRON):
        catalogue.remember(_spec(club=club), _FROM_MODEL)

    written = json.loads(catalogue_file.read_text(encoding="utf-8"))
    keys = [row["spec"]["club"] for row in written["clubs"]]
    assert keys == ["7i", "8i", "9i"]


def test_a_spec_with_no_make_or_model_is_not_remembered(catalogue_file: Path) -> None:
    """Nothing could key it, and a row under the empty key answers every make-less lookup.

    The refusal is silent — this is a cache, and the save that triggered it is not allowed to fail
    over a cache write — so it reports itself by returning `None` and leaving no file behind.
    """
    assert catalogue.remember(_spec(make=""), _FROM_MODEL) is None
    assert catalogue.remember(_spec(model="  "), _FROM_MODEL) is None
    assert not catalogue_file.exists()


def test_a_lookup_with_no_make_or_model_misses(catalogue_file: Path) -> None:
    """The other half of the same rule: no row can be keyed on emptiness, so none is returned."""
    catalogue.remember(_spec(), _FROM_MODEL)

    assert catalogue.lookup("", "T150", 2023, ClubId.SEVEN_IRON) is None
    assert catalogue.lookup("Titleist", "", 2023, ClubId.SEVEN_IRON) is None
    assert catalogue.lookup("Titleist", "T150", 2023, ClubId.EIGHT_IRON) is None


# --------------------------------------------------------------- what a broken file costs


def test_a_truncated_catalogue_reads_as_empty_with_no_exception(catalogue_file: Path) -> None:
    """A corrupt catalogue costs a cache hit and never a lookup.

    The consumer of this is P5's route: a golfer looking a club up gets the model's answer and no
    error at all, because the catalogue's failure is not their problem to see.
    """
    catalogue_file.write_text('{"schema_version": 1, "clubs": [{"spec": ', encoding="utf-8")

    assert catalogue.load_catalogue() == {}
    assert catalogue.lookup("Titleist", "T150", 2023, ClubId.SEVEN_IRON) is None


def test_remember_will_not_write_over_a_catalogue_it_cannot_read(catalogue_file: Path) -> None:
    """`BagStore._load_for_write`'s rule, and the reason the tolerant reader is not enough.

    A writer that treats unreadable as empty replaces forty checked rows with the single row it was
    asked to add. Nothing raises, the file looks fine, and the loss is visible only to someone who
    remembers what used to be in it — so the write is refused and the damaged file is left exactly
    as found, for a person to look at.
    """
    broken = '{"schema_version": 1, "clubs": [{"spec": '
    catalogue_file.write_text(broken, encoding="utf-8")

    assert catalogue.remember(_spec(), _FROM_MODEL) is None
    assert catalogue_file.read_text(encoding="utf-8") == broken


def test_a_catalogue_from_a_future_schema_is_neither_read_nor_overwritten(
    catalogue_file: Path,
) -> None:
    """An unknown `schema_version` is not this shape, and guessing at it is the one thing a cache
    may not do: a row misread under the wrong reader hands a golfer a specification assembled out
    of fields that moved. It is also not this reader's file to replace.
    """
    future = json.dumps({"schema_version": 2, "clubs": [{"anything": True}]})
    catalogue_file.write_text(future, encoding="utf-8")

    assert catalogue.load_catalogue() == {}
    assert catalogue.remember(_spec(), _FROM_MODEL) is None
    assert catalogue_file.read_text(encoding="utf-8") == future


def test_one_unreadable_row_does_not_cost_the_others(catalogue_file: Path) -> None:
    """Row-level tolerance, because a hand-edited typo should not empty a checked catalogue."""
    catalogue.remember(_spec(), _FROM_MODEL)
    payload = json.loads(catalogue_file.read_text(encoding="utf-8"))
    payload["clubs"].insert(0, {"spec": {"club": "not-a-club"}, "provenance": {}})
    catalogue_file.write_text(json.dumps(payload), encoding="utf-8")

    rows = catalogue.load_catalogue()

    assert set(rows) == {"titleist/t150/2023/7i"}


def test_an_absent_catalogue_is_an_empty_one(catalogue_file: Path) -> None:
    """The state this milestone starts in: no file, no rows, and a `remember` that creates it."""
    assert not catalogue_file.exists()
    assert catalogue.load_catalogue() == {}
    assert catalogue.lookup("Titleist", "T150", 2023, ClubId.SEVEN_IRON) is None
    assert catalogue.remember(_spec(), _FROM_MODEL) == "titleist/t150/2023/7i"


def test_the_shipped_catalogue_parses_and_every_row_in_it_validates(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """Somebody has to read the committed file, because the tolerant reader will not complain.

    A stray comma in `club_catalogue.json` turns the cache off for everyone and produces no symptom
    beyond a model call that was supposed to be free. This is the only test that looks at the real
    file, and it asserts against the file's own row count so it stays true as the catalogue grows.
    """
    packaged = Path(catalogue.__file__).with_name("club_catalogue.json")
    monkeypatch.setattr(catalogue, "_catalogue_path", lambda: packaged)

    raw = json.loads(packaged.read_text(encoding="utf-8"))

    assert raw["schema_version"] == 1
    assert len(catalogue.load_catalogue()) == len(raw["clubs"])
