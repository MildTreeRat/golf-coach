"""`POST /api/clubs/lookup` — the catalogue, then the model, and never a write. [M12 P5]

Three properties shape every pin here, and none of them is about the specification's *content*
(that is `tests/clubs/test_lookup.py`, over the parse, and `tests/clubs/test_catalogue.py`, over
the key):

- **the route writes nothing** (ADR-026 §5). A lookup that saved would make a hallucinated lie
  angle indistinguishable from a typed one, and the confirm step this milestone is built on would
  be decoration;
- **the second lookup of a confirmed club reaches no model.** That is the catalogue's whole claim,
  and it is asserted by injecting a stand-in that *raises* if it is called — a spy's call count
  would pass just as well against a route that called and threw the answer away;
- **every failure is a 200 with a note.** No key, no `llm` extra, an API error: the golfer still
  gets a form to type into, which is the M9 state this milestone improves on.

The model is reached by monkeypatching `look_up_club` / `look_up_set` on `api.app`, the shape
`tests/api/test_conversation_routes.py` already uses for `ask`. Injecting a fake `client=` would
mean widening `create_app` with a seam only the tests want; patching the two names the route calls
tests the composition, which is the only thing this phase built.

The catalogue writes into a throwaway file here, from `tests/conftest.py`'s repo-wide fixture
rather than one of this module's own — see that fixture for the row a missing redirect committed.
"""

from __future__ import annotations

from datetime import UTC, datetime
from typing import Any

import pytest
from fastapi.testclient import TestClient

from golf_coach.api import app as app_module
from golf_coach.api.app import create_app
from golf_coach.clubs import catalogue
from golf_coach.clubs.lookup import ClubLookupOutcome
from golf_coach.config import settings
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.club_spec import ClubSpec, ShaftFlex, ShaftMaterial, SpecProvenance
from golf_coach.contracts.golfer import Handedness
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.golfer_store import GolferStore

_TOKEN = "s3cret-token"
_WHEN = datetime(2026, 8, 31, 12, 0, tzinfo=UTC)
_FROM_MODEL = SpecProvenance(
    source="llm:claude-opus-5",
    retrieved_at=_WHEN,
    notes="I cannot confirm the published iron loft chart, so it is null rather than guessed.",
)


@pytest.fixture
def client(tmp_path):
    golfers = GolferStore(tmp_path / "golfers")
    golfers.get_or_create("Aaron", Handedness.RIGHT)
    return TestClient(
        create_app(
            store=SwingBundleStore(tmp_path / "sessions"),
            golfers=golfers,
            token=None,
            worker=None,
        )
    )


def _spec(club: ClubId = ClubId.SEVEN_IRON, **overrides: Any) -> ClubSpec:
    """A T150 7 iron with the fields P4's real lookup filled, and the one it refused."""
    fields: dict[str, Any] = {
        "club": club,
        "make": "Titleist",
        "model": "T150",
        "model_year": 2023,
        # Blank on purpose: P4 measured that the model refuses every iron loft, so the shape this
        # route has to move most often is a spec with the headline field missing.
        "loft_deg": None,
        "lie_deg": 61.5,
        "length_in": 37.0,
        "shaft_material": ShaftMaterial.STEEL,
        "shaft_flex": ShaftFlex.STIFF,
    }
    return ClubSpec(**{**fields, **overrides})


def _answering(*specs: ClubSpec, note: str | None = None):
    """A stand-in for `look_up_club` / `look_up_set` that records what the route asked it for."""
    calls: list[dict] = []

    def fake(make, model, model_year, club_or_slots, **kwargs):
        calls.append(
            {"make": make, "model": model, "model_year": model_year, "clubs": club_or_slots}
        )
        return ClubLookupOutcome(
            specs=specs, provenance=_FROM_MODEL if specs else None, note=note
        )

    fake.calls = calls  # type: ignore[attr-defined]
    return fake


def _refusing(*_args: Any, **_kwargs: Any) -> ClubLookupOutcome:
    """Any call at all is the failure — used where the catalogue is supposed to answer alone."""
    raise AssertionError("the model was called for a club the catalogue already knew")


@pytest.fixture
def looking_up(monkeypatch: pytest.MonkeyPatch):
    """Patch both entry points at once, so a test never has to know which one the route picks."""

    def patch(single=_refusing, whole_set=_refusing):
        monkeypatch.setattr(app_module, "look_up_club", single)
        monkeypatch.setattr(app_module, "look_up_set", whole_set)

    return patch


# ------------------------------------------------------------------------ the boundary


def test_a_lookup_without_a_make_or_a_model_is_refused_before_it_costs_a_call(
    client, looking_up
) -> None:
    """The make and model are the catalogue key, so an answer keyed on nothing is unrememberable.

    Buying it anyway would mean the catalogue calling the API forever while looking exactly like it
    was working — `catalogue_key`'s own argument, applied one layer up where it can still 400.
    """
    looking_up()

    for body in ({"model": "T150", "club": "7i"}, {"make": "Titleist", "club": "7i"}):
        res = client.post("/api/clubs/lookup", json=body)
        assert res.status_code == 400
        assert "make" in res.json()["detail"]

    punctuation = client.post(
        "/api/clubs/lookup", json={"make": "Titleist", "model": "---", "club": "7i"}
    )
    assert punctuation.status_code == 400, "a model that slugifies to nothing is not a model"


def test_a_lookup_naming_no_club_is_a_400(client, looking_up) -> None:
    looking_up()

    res = client.post("/api/clubs/lookup", json={"make": "Titleist", "model": "T150"})

    assert res.status_code == 400
    assert "club" in res.json()["detail"]


def test_every_slot_is_parsed_at_the_boundary_and_a_bad_one_refuses(client, looking_up) -> None:
    """`slots` goes through `_resolve_club` exactly as `club` does.

    An unparsed slot would render a filled-in row the save route then refuses — a form showing the
    golfer a club they cannot keep, which is a worse failure than a 400 on the way in.
    """
    looking_up()

    res = client.post(
        "/api/clubs/lookup",
        json={"make": "Titleist", "model": "T150", "slots": ["7 iron", "wedge"]},
    )

    assert res.status_code == 400
    assert "wedge" in res.json()["detail"], "a category is not a club (ADR-024 §5)"


def test_the_route_is_behind_the_token(tmp_path) -> None:
    """Funnel makes this route publicly reachable (ADR-016), and it spends money when it runs."""
    guarded = TestClient(
        create_app(
            store=SwingBundleStore(tmp_path / "sessions"),
            golfers=GolferStore(tmp_path / "golfers"),
            token=_TOKEN,
            worker=None,
        )
    )

    assert guarded.post("/api/clubs/lookup", json={}).status_code == 401


# ------------------------------------------------------------------------ the lookup


def test_one_club_is_looked_up_and_comes_back_with_its_provenance(client, looking_up) -> None:
    """The identity is echoed from the request, and the refused loft arrives blank and not zero."""
    single = _answering(_spec())
    looking_up(single=single)

    res = client.post(
        "/api/clubs/lookup",
        json={"make": "Titleist", "model": "T150", "model_year": 2023, "club": "7 iron"},
    )

    assert res.status_code == 200
    body = res.json()
    assert body["note"] is None
    assert single.calls == [
        {"make": "Titleist", "model": "T150", "model_year": 2023, "clubs": ClubId.SEVEN_IRON}
    ]

    candidate = body["candidates"][0]
    assert candidate["served_from"] == "lookup"
    assert candidate["spec"]["club"] == "7i", "'7 iron' is parsed here as it is at the bay"
    assert candidate["spec"]["loft_deg"] is None, "a refusal is blank, never a zero loft"
    assert candidate["spec"]["lie_deg"] == 61.5
    assert candidate["provenance"]["source"] == "llm:claude-opus-5"
    assert "cannot confirm" in candidate["provenance"]["notes"]


def test_a_set_is_one_call_and_returns_one_candidate_per_slot_in_order(client, looking_up) -> None:
    """Several slots go to `look_up_set`, which asks for them together so the progression holds."""
    slots = (ClubId.SEVEN_IRON, ClubId.EIGHT_IRON, ClubId.NINE_IRON)
    whole_set = _answering(*(_spec(club=slot) for slot in slots))
    looking_up(whole_set=whole_set)

    res = client.post(
        "/api/clubs/lookup",
        json={"make": "Titleist", "model": "T150", "slots": ["7i", "8 iron", "9i", "7i"]},
    )

    assert res.status_code == 200
    assert len(whole_set.calls) == 1, "a set is one call, not one per slot (ADR-026 §6)"
    assert whole_set.calls[0]["clubs"] == slots, "the duplicate collapsed before the call"
    assert [c["spec"]["club"] for c in res.json()["candidates"]] == ["7i", "8i", "9i"]


def test_a_lookup_that_cannot_run_is_a_200_carrying_its_note(client) -> None:
    """No key, no extra, a rate limit: the page still renders and the golfer still types.

    Asserted through the *real* `look_up_club` with the key cleared rather than through a fake, so
    this pins the composition the route actually performs — including that nothing between an empty
    outcome and the response turns it into a 500. The note's wording is deliberately not asserted:
    it differs between an install with the `llm` extra and one without, and both are correct.
    """
    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(settings, "anthropic_api_key", None)
        res = client.post(
            "/api/clubs/lookup", json={"make": "Titleist", "model": "T150", "club": "7i"}
        )

    assert res.status_code == 200
    body = res.json()
    assert body["candidates"] == []
    assert body["note"], "an empty answer has to say why"
    assert body["make"] == "Titleist" and body["model"] == "T150", (
        "the identity is echoed, so a noted empty answer still names the club that was asked about"
    )


def test_the_lookup_writes_nothing(client, looking_up) -> None:
    """ADR-026 §5, asserted as an absence: no bag entry, and no catalogue row either.

    The catalogue is the one that would look like a feature. Remembering here rather than on
    confirm would serve a model's unreviewed proposal to the next lookup as though a person had
    accepted it.
    """
    looking_up(single=_answering(_spec()))

    client.post("/api/clubs/lookup", json={"make": "Titleist", "model": "T150", "club": "7i"})

    assert catalogue.load_catalogue() == {}
    assert client.get("/api/golfers/aaron/bag").json()["clubs"] == []


# ------------------------------------------------------------------------ the catalogue


def test_a_confirmed_club_is_served_from_the_catalogue_with_no_model_call(
    client, looking_up
) -> None:
    """**The milestone's payoff, end to end**: look up, save, look up again — and the second one
    never reaches a model.

    The saved row's own provenance travels back out rather than the word "catalogue": the catalogue
    is where an answer was kept and not where it came from, and flattening the two would lose
    exactly the distinction ADR-026 §7 asks it to preserve.
    """
    looking_up(single=_answering(_spec()))
    looked_up = client.post(
        "/api/clubs/lookup",
        json={"make": "Titleist", "model": "T150", "model_year": 2023, "club": "7i"},
    ).json()["candidates"][0]

    saved = client.post(
        "/api/golfers/aaron/bag/7i",
        json={
            **{name: value for name, value in looked_up["spec"].items() if name != "club"},
            "provenance": looked_up["provenance"],
        },
    )
    assert saved.status_code == 200

    # Both entry points now raise if they are reached at all.
    looking_up()
    again = client.post(
        "/api/clubs/lookup",
        json={"make": "Titleist", "model": "T150", "model_year": 2023, "club": "7i"},
    )

    assert again.status_code == 200
    candidate = again.json()["candidates"][0]
    assert candidate["served_from"] == "catalogue"
    assert candidate["spec"]["lie_deg"] == 61.5
    assert candidate["provenance"]["source"] == "llm:claude-opus-5", (
        "the row keeps the source that produced it"
    )


def test_a_saved_row_carries_the_spec_and_not_the_declaration(client, looking_up) -> None:
    """`BagEntry` is a `ClubSpec`, so the catalogue would happily store the subclass whole.

    `recorded_at`, `retired_at` and `provenance` are facts about one golfer's declaration; a row
    carrying them would say a T150 7 iron was declared on a Tuesday, which is true of nobody but
    the person who declared it. `_spec_of` is what drops them.
    """
    looking_up()
    client.post(
        "/api/golfers/aaron/bag/7i",
        json={"make": "Titleist", "model": "T150", "model_year": 2023, "lie_deg": 61.5},
    )

    row = catalogue.load_catalogue()["titleist/t150/2023/7i"]

    assert row.spec.lie_deg == 61.5
    assert not hasattr(row.spec, "recorded_at")
    assert row.provenance.source == "typed", "a golfer reading the maker's page is a real source"


def test_a_half_known_set_asks_the_model_only_for_the_rest(client, looking_up) -> None:
    """The catalogue is consulted per slot, which is the shape a bag filled in a club at a time
    actually produces."""
    looking_up()
    client.post(
        "/api/golfers/aaron/bag/7i",
        json={"make": "Titleist", "model": "T150", "model_year": 2023, "lie_deg": 61.5},
    )
    whole_set = _answering(_spec(club=ClubId.EIGHT_IRON), _spec(club=ClubId.NINE_IRON))
    looking_up(whole_set=whole_set)

    res = client.post(
        "/api/clubs/lookup",
        json={
            "make": "Titleist",
            "model": "T150",
            "model_year": 2023,
            "slots": ["7i", "8i", "9i"],
        },
    )

    assert res.status_code == 200
    assert whole_set.calls[0]["clubs"] == (ClubId.EIGHT_IRON, ClubId.NINE_IRON), (
        "the club already confirmed was not bought a second time"
    )
    served = {c["spec"]["club"]: c["served_from"] for c in res.json()["candidates"]}
    assert served == {"7i": "catalogue", "8i": "lookup", "9i": "lookup"}


def test_the_year_is_part_of_the_key_and_a_different_one_misses(client, looking_up) -> None:
    """Makers reuse a model name across generations with different lofts, so a 2023 row must not
    answer a 2019 question — `catalogue_key`'s "an unknown year is its own key, not a wildcard"."""
    looking_up()
    client.post(
        "/api/golfers/aaron/bag/7i",
        json={"make": "Titleist", "model": "T150", "model_year": 2023, "lie_deg": 61.5},
    )
    single = _answering(_spec(model_year=2019))
    looking_up(single=single)

    res = client.post(
        "/api/clubs/lookup",
        json={"make": "Titleist", "model": "T150", "model_year": 2019, "club": "7i"},
    )

    assert res.status_code == 200
    assert len(single.calls) == 1, "a different generation is a different club"
    assert res.json()["candidates"][0]["served_from"] == "lookup"


def test_the_catalogue_key_folds_the_spellings_of_one_model(client, looking_up) -> None:
    """T150, T-150 and t 150 are one club, which is what makes the second lookup free."""
    looking_up()
    client.post(
        "/api/golfers/aaron/bag/7i",
        json={"make": "Titleist", "model": "T150", "model_year": 2023, "lie_deg": 61.5},
    )

    res = client.post(
        "/api/clubs/lookup",
        json={"make": "titleist", "model": "T-150", "model_year": 2023, "club": "7i"},
    )

    assert res.status_code == 200
    assert res.json()["candidates"][0]["served_from"] == "catalogue"
