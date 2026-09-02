"""The three routes the bag page reads and writes through. [M9 P19]

`/api/golfers/{player_id}/bag` is the third surface over `analysis/club_profile.py` — after
`scripts/club_profile.py` and the MCP tools — and **the only one that writes**. Nothing in this
repo had ever declared a bag before this phase, so `BagStore.set_entry` and `remove_entry` had no
caller outside their own unit tests; half of what is pinned here is those two finally reached
through HTTP.

What is asserted is the envelope, the gates and the writes, never the statistics: those are pinned
in `tests/analysis/test_club_profile_builder.py` and `tests/contracts/test_club_profile.py` over
contracts built in memory, and duplicating them through HTTP would test pydantic's serializer twice
and the guard not at all.

Artifacts are seeded onto disk directly rather than by running a pipeline, which is what keeps this
file — like `test_career_route.py` and `test_results.py` — on the base install with no worker, no
threads and no cv2. The seed helper is this file's own rather than a shared fixture, which is the
standing shape in `tests/api/`: every route file builds the disk state its own route cares about.
"""

from __future__ import annotations

import json

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.api.state import AnalysisState, input_hashes, save_state
from golf_coach.contracts.club import ClubId, category_of
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.swing import ANALYSIS_VERSION
from golf_coach.storage.bag_store import BagStore
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.golfer_store import GolferStore

_TOKEN = "s3cret-token"
_ROLES = ("face_on", "down_the_line", "shot_screen")

#: One pose metric and one launch-monitor metric, because the two count differently: a pose
#: measurement is counted per face-on clip and a launch-monitor one per shot photograph. The
#: divergence test below turns on exactly that.
_MEASUREMENTS = [
    {"name": "head_sway_norm", "value": 0.31, "unit": "shoulder_widths",
     "source": "pose:face_on", "detail": "test"},
    {"name": "carry_distance_yds", "value": 154.4, "unit": "yards",
     "source": "launch_monitor:hd_golf", "detail": "test"},
]


def _analysis(*, with_shot: bool = True) -> dict:
    """One stored artifact. Without a shot photo it carries the pose measurement only.

    Not a cosmetic switch: `CorpusSwing.artifact_key` returns None for a launch-monitor measurement
    on a swing with no `shot_sha256`, so a carry written here anyway would contribute nothing and
    the fixture would be describing a swing that cannot exist.
    """
    measurements = _MEASUREMENTS if with_shot else _MEASUREMENTS[:1]
    return {
        "analysis_version": ANALYSIS_VERSION,
        "swing": {
            "overall_score": 86.0,
            "checkpoint_scores": [],
            "unscored": [],
            "measurements": measurements,
        },
        "notes": [],
        "feedback": {"headline": "Work on tempo first.", "tips": []},
    }


@pytest.fixture
def store(tmp_path):
    return SwingBundleStore(tmp_path / "sessions")


@pytest.fixture
def golfers(tmp_path):
    store = GolferStore(tmp_path / "golfers")
    store.get_or_create("Aaron", Handedness.RIGHT)
    return store


@pytest.fixture
def bags(golfers):
    """The store the routes write through, read back directly to check what landed on disk.

    Derived from the golfer store's root rather than constructed on a path of its own, because
    `create_app` derives it the same way — a bag lives beside the golfer it belongs to, and a test
    pointing the two at different directories would be checking a bag the app never wrote.
    """
    return BagStore(golfers.root)


@pytest.fixture
def client(store, golfers):
    return TestClient(create_app(store=store, golfers=golfers, token=None, worker=None))


def _seed_swing(
    client, store, *, club: str, tag: str, with_shot: bool = True
) -> tuple[str, str]:
    """One analyzed swing, attributed to Aaron and tagged with `club`.

    `tag` varies every file's bytes, which is what makes each call a *distinct* swing: `read_corpus`
    groups on the face-on hash, so a shared tag would collapse two calls into one swing plus a
    duplicate.

    `with_shot=False` leaves the shot screen unphotographed — the swing is filmed and analyzed and
    contributes no launch-monitor sample. That is the real shape of the `n_swings > n_shots` gap;
    two swings cannot *share* one photo here, because `assign_from_path` dedupes an identical
    upload back onto the swing that already holds it rather than filing the same bytes twice.
    """
    client.post("/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"})
    client.post("/api/sessions/current/club", json={"club": club})
    roles = _ROLES if with_shot else _ROLES[:2]
    for role in roles:
        res = client.post(
            "/api/uploads",
            params={"role": role, "filename": f"{role}.mov"},
            content=f"{role}-{tag}".encode(),
        )
    payload = res.json()
    session_id, swing_id = payload["session_id"], payload["swing_id"]
    swing_dir = store.root / session_id / swing_id
    (swing_dir / "analysis.json").write_text(
        json.dumps(_analysis(with_shot=with_shot)), encoding="utf-8"
    )

    manifest = store.get_swing(session_id, swing_id)
    assert manifest is not None
    save_state(AnalysisState(status="done", inputs=input_hashes(manifest), score=86.0), swing_dir)
    return session_id, swing_id


def _clubs(body: dict) -> list[str]:
    return [club["club"] for club in body["clubs"]]


# ------------------------------------------------------------------------ the envelope


def test_the_route_lists_every_club_hit_or_declared_in_bag_order(client, store) -> None:
    """Canonical bag order, which is `ClubId`'s declaration order and never alphabetical.

    The set is P14's and `test_bag.py`'s, chosen because insertion order (`pw 7i driver 3w`),
    alphabetical order (`3w 7i driver pw`) and bag order all differ — an assertion over a set where
    two of the three agree would pass on a sorted list.
    """
    _seed_swing(client, store, club="pw", tag="a")
    _seed_swing(client, store, club="7i", tag="b")
    _seed_swing(client, store, club="driver", tag="c")
    client.post("/api/golfers/aaron/bag/3w", json={"loft_deg": 15.0})

    body = client.get("/api/golfers/aaron/bag").json()

    assert _clubs(body) == ["driver", "3w", "7i", "pw"]
    assert body["clubs_used"] == 3, "the 3 wood is declared and unhit"
    assert body["clubs_declared"] == 1


def test_the_category_travels_and_is_derived_not_stored(client, store) -> None:
    """`ClubProfile.category` is a plain property, so the route projects it (M9 P14).

    Asserted against `category_of` rather than a written-out mapping: a literal here would be the
    second copy of `CLUB_CATEGORY` that projecting it server-side exists to prevent, and it would
    pass on the day a club's family changes in one place and not the other.
    """
    _seed_swing(client, store, club="7i", tag="a")
    _seed_swing(client, store, club="driver", tag="b")

    body = client.get("/api/golfers/aaron/bag").json()

    assert body["clubs"], "nothing to check if no club came back"
    for club in body["clubs"]:
        assert club["category"] == category_of(ClubId(club["club"])).value


def test_a_withheld_claim_arrives_as_absent_with_its_reason(client, store) -> None:
    """The property the milestone rests on, one cut further in than the career route pins it.

    A statistic shipped beside a `ready: false` is one forgotten conditional away from being
    rendered; `null` is what makes the page structurally unable to print a carry average over one
    shot. Per club these refusals are the normal case rather than the edge — five 7-iron shots is a
    much higher bar than five shots.
    """
    _seed_swing(client, store, club="7i", tag="a")

    body = client.get("/api/golfers/aaron/bag").json()
    carry = body["clubs"][0]["metrics"]["carry_distance_yds"]

    assert carry["mean"] is None and carry["sd"] is None and carry["mean_ci"] is None
    assert carry["n"] == 1
    assert {r["claim"] for r in carry["withheld"]} == {"center", "spread", "trend"}
    assert all(r["reason"] for r in carry["withheld"]), "a refusal with no sentence is a blank cell"


def test_untagged_swings_travel_and_reach_no_club(client, store) -> None:
    """The history no club row can see, and the number the empty state is built from.

    Every swing on disk today is in here, because they all predate the club tag. Leaving it out
    would make a bag look complete when most of a golfer's swings are simply missing from it.
    """
    _seed_swing(client, store, club="7i", tag="a")
    session_id, swing_id = _seed_swing(client, store, club="pw", tag="b")
    # Strip the club back off the second manifest, which is the only way to make a swing that
    # predates M9 — the upload route refuses an untagged one (P6).
    manifest_path = store.root / session_id / swing_id / "manifest.json"
    raw = json.loads(manifest_path.read_text(encoding="utf-8"))
    raw["club"] = None
    manifest_path.write_text(json.dumps(raw), encoding="utf-8")

    body = client.get("/api/golfers/aaron/bag").json()

    assert _clubs(body) == ["7i"], "the untagged swing reaches no club row"
    assert body["untagged_swings"] == 1


def test_the_two_evidence_counters_diverge_when_a_clip_has_no_shot_photo(client, store) -> None:
    """Two swings with one photo is two swings of history and a carry ceiling of one.

    This gap is the whole reason `ClubProfile` carries `n_swings` *and* `n_shots` (M9 P14): one
    counter would either undercount the history or overstate what a distance statistic can be built
    from, and a carry average is the number nobody audits.
    """
    _seed_swing(client, store, club="7i", tag="a")
    _seed_swing(client, store, club="7i", tag="b", with_shot=False)

    profile = client.get("/api/golfers/aaron/bag").json()["clubs"][0]

    assert profile["n_swings"] == 2
    assert profile["n_shots"] == 1
    assert profile["metrics"]["head_sway_norm"]["n"] == 2, "counted per face-on clip"
    assert profile["metrics"]["carry_distance_yds"]["n"] == 1, "counted per shot photograph"


# ------------------------------------------------------------------------ the gates


def test_an_unregistered_golfer_is_a_404_on_every_verb(client) -> None:
    assert client.get("/api/golfers/nobody/bag").status_code == 404
    assert client.post("/api/golfers/nobody/bag/7i", json={}).status_code == 404
    assert client.delete("/api/golfers/nobody/bag/7i").status_code == 404


def test_a_player_id_that_is_not_a_slug_is_rejected_before_it_reaches_a_path(client) -> None:
    """`player_id` becomes a filesystem path in both directions here — `read_corpus` walks the
    sessions for it and `BagStore` names a file after it — so `_safe` validates it rather than
    trusting it, on the writers as well as the reader.

    Asserted on the **character class** rather than on an encoded `..%2F..%2F`, and that is not a
    weaker test: httpx normalises the encoded form away before it is sent, so a route asserting
    against it gets a routing 404 and never exercises `_safe` at all. A 400 here is the guard
    speaking, and it is the same guard a literal `..` segment would meet.
    """
    for verb in (client.get, client.delete):
        assert verb("/api/golfers/aaron:evil/bag").status_code in (400, 405)
    assert client.get("/api/golfers/.hidden/bag").status_code == 400
    assert client.post("/api/golfers/aa ron/bag/7i", json={}).status_code == 400
    assert client.delete("/api/golfers/aaron:evil/bag/7i").status_code == 400


def test_a_club_that_is_not_a_club_is_a_400_naming_what_was_rejected(client) -> None:
    """Refused rather than nudged toward a nearest match, which is `parse_club`'s own posture.

    'wedge' is the case worth pinning beside the junk one: it is a *category*, so a route that
    guessed would file a lob wedge's shots under a pitching wedge's history where nothing
    downstream could ever detect it (ADR-024 §5).
    """
    for text in ("wedge", "banana"):
        res = client.post(f"/api/golfers/aaron/bag/{text}", json={})
        assert res.status_code == 400
        assert text in res.json()["detail"]


def test_the_shaft_vocabularies_are_parsed_at_the_boundary_and_refuse(client) -> None:
    """`"S"` off the shaft band saves; `"regular-ish"` is a 400 and not a silent `REGULAR`. [M12 P2]

    The reason `BagEntryRequest` types these two as `str` rather than as their enums: pydantic
    would 422 on `"S"` before `parse_shaft_flex` ever ran, and the letter code is the only spelling
    most golfers have ever seen their flex written in. `ClubRequest` states the same argument for
    `"7 iron"`, which is the pattern this follows.

    The refusal matters in the other direction for the same asymmetry `parse_club` names: a hedge
    recorded as a flex loses the hedge, and a graphite shaft filed as steel is a wrong label nothing
    downstream can detect.
    """
    def entry_for(body: dict, club: str) -> dict:
        # The response is the whole bag in bag order, so index by club rather than by position.
        return next(c for c in body["clubs"] if c["club"] == club)["bag_entry"]

    saved = client.post(
        "/api/golfers/aaron/bag/7i",
        json={"shaft_flex": "S", "shaft_material": "Carbon Fiber"},
    )

    assert saved.status_code == 200
    entry = entry_for(saved.json(), "7i")
    assert entry["shaft_flex"] == "stiff", "'S' is the spelling on the band"
    assert entry["shaft_material"] == "graphite"

    refused = client.post("/api/golfers/aaron/bag/8i", json={"shaft_flex": "regular-ish"})
    assert refused.status_code == 400
    assert "regular-ish" in refused.json()["detail"]

    blank = client.post("/api/golfers/aaron/bag/9i", json={"make": "Ping"})
    assert blank.status_code == 200, "an undeclared flex is not a refused one"
    assert entry_for(blank.json(), "9i")["shaft_flex"] is None


def test_every_verb_is_behind_the_token(store, golfers) -> None:
    """Funnel makes these routes publicly reachable (ADR-016), and the two writers are the first
    thing on this server that lets a stranger edit stored state rather than only read it."""
    client = TestClient(create_app(store=store, golfers=golfers, token=_TOKEN, worker=None))
    headers = {"X-Upload-Token": _TOKEN}

    assert client.get("/api/golfers/aaron/bag").status_code == 401
    assert client.post("/api/golfers/aaron/bag/7i", json={}).status_code == 401
    assert client.delete("/api/golfers/aaron/bag/7i").status_code == 401
    assert client.get("/api/golfers/aaron/bag", headers=headers).status_code == 200


# ------------------------------------------------------------------------ the writes


def test_declaring_a_club_puts_it_in_the_bag_and_the_response_is_a_fresh_read(client) -> None:
    """The writers return the whole bag, not the row they saved — one render path for the page.

    Asserted as equality with a following GET rather than field by field, because the property is
    not "the loft came back", it is that a save and a reload cannot disagree. P16's caveat is
    created by declaring an entry, so a response assembled from the request would be the one place
    it could go missing.
    """
    saved = client.post(
        "/api/golfers/aaron/bag/7 iron",
        json={"loft_deg": 34.0, "make": "Titleist", "model": "T150", "shaft_model": "Modus 105"},
    )

    assert saved.status_code == 200
    assert saved.json() == client.get("/api/golfers/aaron/bag").json()

    profile = saved.json()["clubs"][0]
    assert profile["bag_entry"]["club"] == "7i", "'7 iron' is parsed here as it is at the bay"
    assert profile["bag_entry"]["loft_deg"] == 34.0
    assert profile["bag_entry"]["make"] == "Titleist"
    assert profile["in_bag"] is True
    assert profile["n_swings"] == 0, "declared is not hit"


def test_saving_an_unedited_row_does_not_move_the_declared_date(client) -> None:
    """**The load-bearing write test.** P3 built this short-circuit for this exact route.

    Someone opens the bag page and presses save on a row they did not touch. If `recorded_at`
    moved, P16 would raise a bag-changed caveat over shots that were all hit with the same physical
    club — a sentence telling the golfer their carry average pools two clubs, produced by the act
    of looking at the page.
    """
    body = {"loft_deg": 34.0, "make": "Titleist", "model": "T150"}
    first = client.post("/api/golfers/aaron/bag/7i", json=body).json()
    again = client.post("/api/golfers/aaron/bag/7i", json=body).json()

    assert (
        first["clubs"][0]["bag_entry"]["recorded_at"]
        == again["clubs"][0]["bag_entry"]["recorded_at"]
    )


def test_replacing_a_club_retires_the_old_one_and_caveats_the_history(client, store, bags) -> None:
    """Where the write path and P16's caveat meet, which is the pair nothing else exercises.

    The swing is hit first and the entry declared after, so all of the history predates it — the
    state every golfer is in the day they declare a bag. The caveat says so and the statistics are
    untouched, which is the half that would be easy to get wrong by cutting the history back to
    post-entry swings.
    """
    _seed_swing(client, store, club="7i", tag="a")
    client.post("/api/golfers/aaron/bag/7i", json={"loft_deg": 34.0, "make": "Titleist"})
    body = client.post("/api/golfers/aaron/bag/7i", json={"loft_deg": 30.5, "make": "Mizuno"})

    profile = body.json()["clubs"][0]
    assert profile["bag_entry"]["make"] == "Mizuno"
    assert profile["caveats"], "a bag entry recorded after these swings has to say so"
    assert profile["n_swings"] == 1, "the caveat qualifies the history, it never withholds it"

    shelf = bags.get("aaron").retired_for(ClubId.SEVEN_IRON)
    assert [entry.make for entry in shelf] == ["Titleist"]
    assert shelf[0].retired_at is not None


def test_removing_a_club_keeps_every_shot_it_ever_hit(client, store, bags) -> None:
    """`in_bag` and `n_swings > 0` are different questions, and this is where that pays.

    A club sold last year still answers "how far did I hit it". Removing it takes it out of the
    *current* bag and destroys nothing — the entry goes to `Bag.retired`, so the measured loft
    survives, which is the loss ADR-024 called unrecoverable after the fact.
    """
    _seed_swing(client, store, club="7i", tag="a")
    client.post("/api/golfers/aaron/bag/7i", json={"loft_deg": 34.0, "make": "Titleist"})

    body = client.delete("/api/golfers/aaron/bag/7i").json()

    profile = body["clubs"][0]
    assert profile["in_bag"] is False
    assert profile["bag_entry"] is None, "no bag entry, and never the shelf's — that is P15's rule"
    assert profile["n_swings"] == 1 and profile["metrics"], "the history is untouched"
    assert body["clubs_used"] == 1 and body["clubs_declared"] == 0

    shelf = bags.get("aaron").retired_for(ClubId.SEVEN_IRON)
    assert [entry.make for entry in shelf] == ["Titleist"]


def test_removing_a_club_that_is_not_in_the_bag_is_a_404(client) -> None:
    """`remove_entry` returning None. A 200 would tell the page its request changed something."""
    res = client.delete("/api/golfers/aaron/bag/7i")

    assert res.status_code == 404
    assert "7i" in res.json()["detail"]


# ------------------------------------------------------------- the whole specification [M12 P5]


def test_the_request_carries_every_spec_field_and_the_three_it_must_not(client) -> None:
    """**The R6 pin.** `BagEntryRequest` is a hand-written copy of `ClubSpec`'s field list, and
    this is what makes that safe to be.

    The failure it catches is silent and expensive: a field added to `ClubSpec` that nobody adds
    here is accepted by pydantic, dropped by the route on the way to the contract, and answered
    with a 200 — the page shows a saved value that went nowhere. That is `mcp/query.py`'s
    `_METRIC_FIELDS` failure, which this repo has already paid for once.

    The three deliberate differences are asserted as such rather than skipped, because each is a
    decision: `club` is the path segment (so the route editing a 7 iron cannot be handed a payload
    claiming to be a wedge), and `provenance` is added because it is a fact about the declaration
    rather than about the club.
    """
    from golf_coach.api.app import BagEntryRequest
    from golf_coach.contracts.club_spec import ClubSpec

    body = set(BagEntryRequest.model_fields)

    assert body == (set(ClubSpec.model_fields) - {"club"}) | {"provenance"}
    assert "recorded_at" not in body, "the store owns the clock (`bag_store.py`)"
    assert "retired_at" not in body, "a club leaves the bag through DELETE, not through a field"


def test_a_whole_specification_round_trips_through_the_save_route(client) -> None:
    """Every group of `ClubSpec` posted at once, read back through a fresh `GET`.

    Field by field rather than as a lump, because the failure this catches is one field silently
    lost in the middle of a body that otherwise works — a 200 looks identical either way. The two
    tuple-shaped and enum-shaped fields are in here on purpose: `loft_range_deg` is the only
    non-scalar on the contract and `shaft_flex` is one of the two that arrive as text.
    """
    posted = {
        "make": "TaylorMade",
        "model": "Stealth 2",
        "model_year": 2023,
        "head_type": "titanium driver head, carbon crown",
        "set_composition": "single club",
        "loft_deg": 10.5,
        "lie_deg": 56.0,
        "bounce_deg": None,
        "grind": "",
        "offset_mm": 2.0,
        "face_angle_deg": 0.0,
        "head_weight_g": 198.0,
        "adjustable_hosel": True,
        "loft_range_deg": [8.5, 12.5],
        "shaft_model": "Ventus TR Red",
        "shaft_material": "Carbon Fiber",
        "shaft_flex": "S",
        "shaft_weight_g": 55.0,
        "shaft_torque_deg": 4.6,
        "shaft_kick_point": "mid-high",
        "length_in": 45.75,
        "swing_weight": "D3",
        "total_weight_g": 310.0,
        "grip": "Golf Pride Tour Velvet 360",
        "cor": 0.83,
        "moi_g_cm2": 5100.0,
        "usga_conforming": True,
    }

    saved = client.post("/api/golfers/aaron/bag/driver", json=posted)
    assert saved.status_code == 200
    assert saved.json() == client.get("/api/golfers/aaron/bag").json()

    entry = saved.json()["clubs"][0]["bag_entry"]
    for name, value in posted.items():
        expected = {"shaft_material": "graphite", "shaft_flex": "stiff"}.get(name, value)
        assert entry[name] == expected, f"{name} did not survive the round trip"

    assert entry["face_angle_deg"] == 0.0, "a declared zero is a value, not an omission"
    assert entry["bounce_deg"] is None and entry["grind"] == "", "undeclared stays undeclared"


def test_a_looked_up_provenance_travels_onto_the_entry_and_a_typed_one_is_stamped(client) -> None:
    """ADR-026 §7 at the surface that writes: a model's answer never looks like a typed one.

    The block is echoed back from the lookup response rather than composed here, which is what
    carries the model id **and its own confidence notes** — usually the reason a field beside them
    is blank — into the bag. A body with no provenance is stamped `"typed"`, so the M9 hand-filled
    form keeps working and still says where its numbers came from.
    """
    echoed = {
        "source": "llm:claude-opus-5",
        "retrieved_at": "2026-08-31T12:00:00Z",
        "notes": "the T150 iron loft chart is not something I can confirm.",
    }

    looked_up = client.post(
        "/api/golfers/aaron/bag/7i",
        json={"make": "Titleist", "model": "T150", "provenance": echoed},
    )
    typed = client.post("/api/golfers/aaron/bag/8i", json={"make": "Titleist", "model": "T150"})

    def entry_for(body: dict, club: str) -> dict:
        return next(c for c in body["clubs"] if c["club"] == club)["bag_entry"]

    assert looked_up.status_code == 200 and typed.status_code == 200
    assert entry_for(looked_up.json(), "7i")["provenance"]["source"] == "llm:claude-opus-5"
    assert "loft chart" in entry_for(looked_up.json(), "7i")["provenance"]["notes"]
    assert entry_for(typed.json(), "8i")["provenance"]["source"] == "typed"


def test_a_second_save_of_a_looked_up_club_does_not_retire_it(client, bags) -> None:
    """**ADR-026 §4, reached through HTTP.** `same_club_as` excludes `provenance`, and this route
    stamps a fresh `retrieved_at` on every save — so without that exclusion, pressing save twice
    would retire a perfectly good entry and hand M9 P16 a bag-changed caveat over shots that were
    every one of them hit with the same club.

    Nothing raises when it goes wrong and the bag still reads correctly, which is why it is pinned
    at the route as well as on the contract: the only symptom is a sentence about a club that never
    changed.
    """
    body = {"make": "Titleist", "model": "T150", "lie_deg": 61.5}

    first = client.post("/api/golfers/aaron/bag/7i", json=body).json()
    second = client.post("/api/golfers/aaron/bag/7i", json=body).json()

    declared = [c for c in second["clubs"] if c["club"] == "7i"][0]["bag_entry"]["recorded_at"]
    assert declared == [c for c in first["clubs"] if c["club"] == "7i"][0]["bag_entry"][
        "recorded_at"
    ]
    assert bags.get("aaron").retired_for(ClubId.SEVEN_IRON) == ()


def test_a_confirmed_club_is_written_into_the_catalogue_and_a_nameless_one_is_not(client) -> None:
    """Confirming is what makes a specification worth serving to the next lookup (ADR-026 §5).

    The nameless half is the silent refusal `remember` is built on: a row under an empty key would
    be served for every make-less lookup thereafter, so it declines and the save still succeeds.
    A 500 here would cost the golfer the bag entry over a cache write.
    """
    from golf_coach.clubs import catalogue

    with_name = client.post(
        "/api/golfers/aaron/bag/7i",
        json={"make": "Titleist", "model": "T150", "model_year": 2023, "lie_deg": 61.5},
    )
    without = client.post("/api/golfers/aaron/bag/8i", json={"loft_deg": 38.0})

    assert with_name.status_code == 200 and without.status_code == 200
    assert list(catalogue.load_catalogue()) == ["titleist/t150/2023/7i"]
