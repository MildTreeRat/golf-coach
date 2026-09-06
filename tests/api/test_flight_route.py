"""`GET /api/sessions/{id}/swings/{id}/flight` — the route M15 P15's page draws from.

Three things are pinned here and the third is the one that matters.

**The join**: a swing reaches a flight through the shot photo's sha256, its golfer's handedness
and its bag's loft, and each of those going missing produces a different answer rather than the
same shrug. **The boundary**: a shot the model cannot fly is a 200 carrying a reason, and a swing
with no shot at all is a 404 — ten of the thirteen shots on disk are the first kind, so a route
that treated a refusal as an error would report this repo as broken every time it was read
honestly. **The honesty fields**: the spin's source, the axis's refusal and the caveats are what
M15 P16 renders, and a payload that dropped one of them would make that phase a route change.

The physics is not re-tested here. `tests/analysis/test_flight.py` and
`test_flight_validation.py` own the numbers; what this file asserts about a carry is that the
route serves the same one the integrator produces, which is a statement about plumbing.

Artifacts are seeded onto disk directly rather than by running a pipeline — the standing shape in
`tests/api/`, and what keeps this on the base install with no worker, no OCR and no cv2.
"""

from __future__ import annotations

import pytest
from fastapi.testclient import TestClient

from golf_coach.analysis.flight import simulate_flight
from golf_coach.analysis.flight_infer import flight_for_shot
from golf_coach.analysis.flight_measure import FLIGHT_MEASUREMENTS
from golf_coach.api.app import create_app
from golf_coach.config import settings
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.launch_monitor.screen.store import ShotStore
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.golfer_store import GolferStore
from golf_coach.storage.manifest import Role

_ROLES = ("face_on", "down_the_line", "shot_screen")

#: `2026-08-10-1`, the lower-spin validation shot: the only shape on this corpus that carries a
#: printed spin *and* a printed axis, so it is the one shot that flies with nothing inferred.
_PRINTED = {
    "ball_speed": 90.7,
    "launch_angle": 20.9,
    "spin_rate": 5991.0,
    "carry_distance": 125.6,
    "launch_direction": -5.3,
    "spin_axis": 2.5,
    "shot_type": "CENTER SLIGHT FADE",
}

#: `2026-08-23-7`, a real 3 wood from the bay session: no spin printed, and a carry the solve can
#: reach on both branches — so what decides it is a loft, and the bag is where a loft comes from.
_TWO_BRANCH = {
    "ball_speed": 100.9,
    "launch_angle": 17.0,
    "carry_distance": 144.5,
    "launch_direction": 4.6,
    "shot_type": "FADE",
}

#: `2026-08-23-1`: a printed carry above anything this model can fly at that ball speed. The
#: commonest refusal on disk, and never reported as an OCR fault (ADR-027's 2026-09-05f addendum).
_UNREACHABLE = {
    "ball_speed": 88.9,
    "launch_angle": 19.5,
    "carry_distance": 121.8,
    "launch_direction": -6.7,
    "shot_type": "SLIGHT FADE",
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
def shots(tmp_path, monkeypatch):
    """The parsed-shot store the route reads, pointed away from the repo's real one.

    `create_app` takes no `shots_dir`: the store is not derivable from a bundle root the way a bag
    is derivable from a golfer root, and the route reads `settings` at request time exactly as
    `api/pipeline.py` does. `test_conversation_routes.py` patches its directory the same way.
    """
    root = tmp_path / "shots"
    monkeypatch.setattr(settings, "shots_dir", root)
    return ShotStore(root)


@pytest.fixture
def client(store, golfers, shots):
    return TestClient(create_app(store=store, golfers=golfers, token=None, worker=None))


def _seed(client, store, shots, *, fields, club="7i", tag="a", with_shot=True, parsed=True):
    """One uploaded swing, with its shot screen parsed into the store under the photo's own hash.

    The digest is read back off the manifest rather than hashed here: the join the route performs
    is `SwingManifest.roles[SHOT_SCREEN].content_sha256` against `ShotProvenance.image_sha256`, and
    a test that computed its own sha256 would agree with itself while the two sides drifted.
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
    session_id, swing_id = res.json()["session_id"], res.json()["swing_id"]

    manifest = store.get_swing(session_id, swing_id)
    assert manifest is not None
    role_file = manifest.roles.get(Role.SHOT_SCREEN)
    if parsed and role_file is not None:
        shots.put(
            ShotData(
                shot_id=f"{session_id}-{swing_id}",
                session_id=session_id,
                timestamp="2026-08-23T10:00:00Z",
                source=ShotSource.SCREEN,
                provenance=ShotProvenance(
                    device="hd_golf",
                    parse_confidence=0.98,
                    image_sha256=role_file.content_sha256,
                ),
                **fields,
            )
        )
    return session_id, swing_id


def _flight(client, session_id, swing_id, **params):
    return client.get(f"/api/sessions/{session_id}/swings/{swing_id}/flight", params=params)


def _reflown(store, shots, session_id, swing_id, *, loft_deg=30.5):
    """The same shot flown outside the route, to compare what the route served against.

    Nothing in this file types a carry or a landing point. A re-sourced coefficient table moves
    the model and both sides of every comparison here move with it, which keeps these assertions
    about the route rather than about the physics — those are `tests/analysis/test_flight.py`'s.
    """
    manifest = store.get_swing(session_id, swing_id)
    shot = shots.get(manifest.roles[Role.SHOT_SCREEN].content_sha256)
    resolved = flight_for_shot(shot, loft_deg=loft_deg, handedness=Handedness.RIGHT)
    assert resolved.launch is not None
    return simulate_flight(resolved.launch)


# ------------------------------------------------------------------- a flight that flew


def test_a_printed_spin_flies_and_the_carry_is_the_integrator_s_own(client, store, shots) -> None:
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED)

    body = _flight(client, session_id, swing_id).json()

    assert body["flew"] is True
    assert body["reason"] is None
    assert body["spin"]["source"] == "measured"
    assert body["spin"]["rpm"] == 5991.0

    carry = next(m for m in body["measurements"] if m["name"] == "flight_carry_yds")
    assert carry["value"] == pytest.approx(_reflown(store, shots, session_id, swing_id).carry_yds)
    assert carry["unit"] == "yards"


def test_a_flown_shot_serves_every_simulated_name_the_artifact_would_have(
    client, store, shots
) -> None:
    """The registry is the definition, so the route and `analysis.json` cannot drift apart.

    All six land on this shot because it is the one shape that owes none of the two conditional
    withholdings: the axis was printed, so the curve is drawn, and — the other way round — the
    spin was printed, so `flight_spin_rpm` is *not* recorded. Five of the six, and the sixth is
    the one that says a solved spin is not a measured one.
    """
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED)

    body = _flight(client, session_id, swing_id).json()
    names = {m["name"] for m in body["measurements"]}

    assert names == set(FLIGHT_MEASUREMENTS) - {"flight_spin_rpm"}
    assert body["source"] == "model:flight_v1"
    assert body["axis"]["curve_is_drawn"] is True
    assert body["unscored"] == []


def test_the_measured_numbers_travel_beside_the_simulated_ones(client, store, shots) -> None:
    """M15 P16 draws the measured landing point beside the simulated one; this is that point."""
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED)

    body = _flight(client, session_id, swing_id).json()
    measured = {m["name"]: m["value"] for m in body["measured"]}

    assert measured["carry_distance_yds"] == 125.6
    assert "start_line_offline_yds" in measured
    # The printed carry and the simulated one are two different numbers under two different names,
    # which is the whole of ADR-027 §Decision 6.
    simulated = {m["name"] for m in body["measurements"]}
    assert not simulated & set(measured)


def test_the_comparison_pairs_each_number_with_the_printed_one_and_says_what_the_gap_is(
    client, store, shots
) -> None:
    """M15 P16 renders this; it does not derive it (`flight_measure.compare_to_printed`).

    Two rows on this shot and they are opposite kinds of thing. The carry is a check — the screen
    printed the spin, so the carry beside it was measured by something that did not use it. The
    offline is not a check at all: the two numbers are where the ball started and where it
    finished, and only one of them was measured.
    """
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED)

    served = _flight(client, session_id, swing_id).json()["comparison"]
    rows = {row["quantity"]: row for row in served}

    assert set(rows) == {"down_range", "offline"}
    assert rows["down_range"]["comparable"] is True
    assert rows["down_range"]["measured"] == 125.6
    assert rows["offline"]["comparable"] is False
    assert all(row["reading"] for row in rows.values()), "a row without its sentence is two numbers"


def test_a_solved_spin_makes_its_own_carry_row_not_a_check(client, store, shots) -> None:
    """⚠️ The row this phase refuses to serve as a validation.

    One bag edit turns the refusal above into a flight, and that flight's carry is the printed one
    reproduced — because the printed one is what the solve inverted. Two numbers agreeing to a
    thousandth of a yard is the most convincing thing on the page and evidence of nothing.
    """
    session_id, swing_id = _seed(client, store, shots, fields=_TWO_BRANCH, club="3w")
    client.post("/api/golfers/aaron/bag/3w", json={"loft_deg": 15.0})

    body = _flight(client, session_id, swing_id).json()
    carry = next(row for row in body["comparison"] if row["quantity"] == "down_range")

    assert body["spin"]["source"] == "inferred"
    assert carry["comparable"] is False
    assert abs(carry["difference"]) < 0.01
    assert "by construction" in carry["reading"]


def test_a_refused_flight_has_nothing_to_set_beside_the_printed_numbers(
    client, store, shots
) -> None:
    """`measured` still travels — the screen printed those — but there is no simulated half."""
    session_id, swing_id = _seed(client, store, shots, fields=_UNREACHABLE)

    body = _flight(client, session_id, swing_id).json()

    assert body["flew"] is False
    assert body["comparison"] == []
    assert {m["name"] for m in body["measured"]} >= {"carry_distance_yds"}


def test_the_caveats_are_the_ones_the_flight_owes(client, store, shots) -> None:
    """Composed by `analysis/flight_caveats.caveats_for`, which is also what the CLI prints."""
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED)

    said = _flight(client, session_id, swing_id).json()["caveats"]

    assert len(said) == 2
    assert any("driver" in s for s in said)
    assert any("backwards" in s for s in said)
    assert all("\n" not in s for s in said)


# ------------------------------------------------------------------- the path itself


def test_the_path_is_sampled_and_keeps_the_solved_landing(client, store, shots) -> None:
    """The landing is the one point the integrator solved for, so it is never the one dropped."""
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED)

    body = _flight(client, session_id, swing_id, points=5).json()
    path = body["path"]

    assert len(path["points"]) == 5
    assert path["steps"] > 500
    assert path["points"][0]["t_s"] == 0.0

    # The last sample is the point the integrator *solved* for, to the bit — not the last whole
    # step, which would end the flight a few centimetres in the air. Compared against the flight's
    # own landing rather than against the carry, because `carry_yds` is that point projected onto
    # the launch azimuth and this shot started 5.3 deg left of the target line.
    landing = _reflown(store, shots, session_id, swing_id).landing
    assert path["points"][-1]["y_yds"] == pytest.approx(0.0, abs=1e-6)
    assert path["points"][-1]["x_yds"] == pytest.approx(landing.x_yds)
    assert path["points"][-1]["z_yds"] == pytest.approx(landing.z_yds)


def test_the_path_carries_the_clamp_flag_per_point(client, store, shots) -> None:
    """Every iron on this corpus flies its whole path on a held end row, and the page has to say
    which part was extrapolated — which is per point, because the spin ratio climbs as the ball
    slows."""
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED)

    path = _flight(client, session_id, swing_id).json()["path"]

    assert path["fully_clamped"] is True
    assert all(p["clamped"] for p in path["points"])
    assert path["spin_ratio_max"] > path["spin_ratio_min"]


def test_a_points_count_below_two_is_refused_rather_than_silently_widened(
    client, store, shots
) -> None:
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED)

    assert _flight(client, session_id, swing_id, points=1).status_code == 422


# ------------------------------------------------------------------- refusals are 200s


def test_a_carry_this_model_cannot_fly_is_a_200_with_a_reason(client, store, shots) -> None:
    """The commonest state on disk. A 500 here would report the corpus as a server fault."""
    session_id, swing_id = _seed(client, store, shots, fields=_UNREACHABLE)

    res = _flight(client, session_id, swing_id)
    body = res.json()

    assert res.status_code == 200
    assert body["flew"] is False
    assert body["reason"] == "carry_unreachable"
    assert body["path"] is None
    assert body["measurements"] == []
    # One entry and not five: `get_session_summary` counts unscored by name, and one missing
    # flight counted six times would report a session as six times more broken than it is.
    assert [u["name"] for u in body["unscored"]] == ["flight_carry_yds"]
    assert body["unscored"][0]["refilming_helps"] is False


def test_a_refusal_keeps_the_solve_it_ran(client, store, shots) -> None:
    """Which of the seven cases the carry landed on is most of the answer when there is none."""
    session_id, swing_id = _seed(client, store, shots, fields=_UNREACHABLE)

    body = _flight(client, session_id, swing_id).json()

    assert body["spin"]["rpm"] is None
    assert body["spin"]["case"] is not None
    assert body["spin"]["cap_rpm"] > 0
    assert body["caveats"] == []


def test_an_undeclared_loft_refuses_and_names_the_bag_page(client, store, shots) -> None:
    """P10's finding, served: two of the ten refusals are the bag page and not a bay session."""
    session_id, swing_id = _seed(client, store, shots, fields=_TWO_BRANCH, club="3w")

    body = _flight(client, session_id, swing_id).json()

    assert body["reason"] == "no_club_loft"
    assert body["shot"]["club"] == "3w"
    assert body["shot"]["loft_deg"] is None
    assert "bag page" in body["shot"]["loft_remedy"]


def test_declaring_the_loft_turns_that_refusal_into_a_flight(client, store, shots) -> None:
    """The same shot, the same route, one bag edit apart — and no re-analysis in between.

    This is why the route re-flies rather than serving `analysis.json`'s stored numbers: the bag
    is editable, and a stored refusal would go on being drawn until someone re-ran the engine.
    """
    session_id, swing_id = _seed(client, store, shots, fields=_TWO_BRANCH, club="3w")
    assert _flight(client, session_id, swing_id).json()["flew"] is False

    client.post("/api/golfers/aaron/bag/3w", json={"loft_deg": 15.0})

    body = _flight(client, session_id, swing_id).json()
    assert body["flew"] is True
    assert body["shot"]["loft_deg"] == 15.0
    assert body["spin"]["source"] == "inferred"
    # The number is capped and the cap travels with it, never one without the other.
    assert body["spin"]["rpm"] <= body["spin"]["cap_rpm"]


def test_a_remedy_is_not_offered_to_a_flight_that_never_wanted_a_loft(
    client, store, shots
) -> None:
    """A printed spin never reaches for a loft, so "add the club on the bag page" would be a
    repair for a problem this golfer does not have."""
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED, club="9i")

    body = _flight(client, session_id, swing_id).json()

    assert body["flew"] is True
    assert body["shot"]["loft_deg"] is None
    assert body["shot"]["loft_remedy"] is None


def test_an_unresolved_axis_draws_the_flight_planar_and_withholds_the_landing_offline(
    client, store, shots
) -> None:
    """A refused axis does not refuse the flight (ADR-027 §Decision 5's third branch).

    What it costs is one measurement, and the reason it costs that one is P10's identity: a planar
    flight's landing offline is `carry * sin(start line)`, which the repo already records as
    `start_line_offline_yds`.
    """
    session_id, swing_id = _seed(client, store, shots, fields=_TWO_BRANCH, club="3w")
    client.post("/api/golfers/aaron/bag/3w", json={"loft_deg": 15.0})

    body = _flight(client, session_id, swing_id).json()

    assert body["flew"] is True
    assert body["axis"]["curve_is_drawn"] is False
    assert body["axis"]["reason"] == "spin_axis_unresolved"
    assert "flight_landing_offline_yds" not in {m["name"] for m in body["measurements"]}
    assert [u["name"] for u in body["unscored"]] == ["flight_landing_offline_yds"]
    assert any("vertical plane" in s for s in body["caveats"])


def test_the_screen_s_own_word_is_carried_beside_the_derived_direction(
    client, store, shots
) -> None:
    """Flagged, never overwritten: quietly picking one is how ADR-014's sign inversion survived."""
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED)

    axis = _flight(client, session_id, swing_id).json()["axis"]

    assert axis["screen_shape"] == "fade"
    assert axis["curve_direction"] == "fade"
    assert axis["sign_disagrees"] is False


# ------------------------------------------------------------------- and 404s are not refusals


def test_a_swing_with_no_shot_screen_has_no_flight_resource(client, store, shots) -> None:
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED, with_shot=False)

    res = _flight(client, session_id, swing_id)

    assert res.status_code == 404
    assert "no shot-screen photo" in res.json()["detail"]


def test_an_unread_shot_screen_names_the_two_things_that_read_it(client, store, shots) -> None:
    """No OCR in a request handler — the route reads the store and says so when it is empty."""
    session_id, swing_id = _seed(client, store, shots, fields=_PRINTED, parsed=False)

    res = _flight(client, session_id, swing_id)

    assert res.status_code == 404
    assert "import_shot_screens" in res.json()["detail"]


def test_an_unknown_swing_404s(client) -> None:
    assert _flight(client, "2026-01-01", "9").status_code == 404


@pytest.mark.parametrize("segment", ["..", "../etc", ".hidden", "a/b", "with space"])
def test_path_traversal_and_junk_segments_are_rejected(client, segment) -> None:
    assert _flight(client, segment, "1").status_code in (400, 404)
    assert _flight(client, "2026-01-01", segment).status_code in (400, 404)


def test_the_route_is_behind_the_upload_token(store, golfers, shots) -> None:
    """Funnel makes every `/api/` route publicly reachable, so this one is gated like the rest."""
    client = TestClient(create_app(store=store, golfers=golfers, token="s3cret", worker=None))

    res = client.get("/api/sessions/2026-01-01/swings/1/flight")

    assert res.status_code == 401
