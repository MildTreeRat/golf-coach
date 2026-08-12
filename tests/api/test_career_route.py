"""The route both career surfaces read. [Career mode, step 6]

`/api/golfers/{player_id}/career` feeds the career page and the "Against your own history" block on
the swing page. One route rather than two, so the number rendered in one place and the number
rendered in the other cannot disagree — and so this file is the only place the web contract for
career mode has to be pinned.

What is asserted here is the envelope and the gates, not the statistics: those are pinned in
`tests/analysis/test_baseline.py`, `test_dispersion.py` and `test_comparison.py` over contracts
built in memory. Duplicating them through HTTP would test pydantic's serializer twice and the
guard not at all.

Artifacts are seeded onto disk directly rather than by running a pipeline — the same property that
keeps `test_results.py` on the base install with no worker, no threads and no cv2.
"""

from __future__ import annotations

import json

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.api.state import AnalysisState, input_hashes, save_state
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.swing import ANALYSIS_VERSION
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.golfer_store import GolferStore

_TOKEN = "s3cret-token"
_ROLES = ("face_on", "down_the_line", "shot_screen")

_MEASUREMENTS = [
    {"name": "head_sway_norm", "value": 0.31, "unit": "shoulder_widths",
     "source": "pose:face_on", "detail": "test"},
    {"name": "tempo_ratio", "value": 2.42, "unit": "ratio",
     "source": "pose:face_on", "detail": "test"},
    {"name": "face_to_path_deg", "value": 8.6, "unit": "degrees",
     "source": "launch_monitor:hd_golf", "detail": "test"},
]


def _analysis() -> dict:
    return {
        "analysis_version": ANALYSIS_VERSION,
        "swing": {
            "overall_score": 86.0,
            "checkpoint_scores": [],
            "unscored": [],
            "measurements": _MEASUREMENTS,
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
def client(store, golfers):
    return TestClient(create_app(store=store, golfers=golfers, token=None, worker=None))


def _seed(client, store, *, player: str | None = "aaron") -> str:
    """One analyzed swing carrying measurements, attributed to a golfer."""
    if player:
        client.post("/api/sessions/current/golfer", json={"name": "Aaron", "handedness": "right"})
    for role in _ROLES:
        res = client.post(
            "/api/uploads", params={"role": role, "filename": f"{role}.mov"},
            content=f"{role}-bytes".encode(),
        )
    session_id = res.json()["session_id"]
    swing_dir = store.root / session_id / "1"
    (swing_dir / "analysis.json").write_text(json.dumps(_analysis()), encoding="utf-8")

    manifest = store.get_swing(session_id, "1")
    assert manifest is not None
    save_state(
        AnalysisState(status="done", inputs=input_hashes(manifest), score=86.0), swing_dir
    )
    return session_id


# ------------------------------------------------------------------------ the envelope


def test_the_route_serves_all_three_layers_over_one_corpus(client, store) -> None:
    _seed(client, store)

    body = client.get("/api/golfers/aaron/career").json()

    assert body["player_id"] == "aaron"
    assert body["display_name"] == "Aaron"
    assert body["handedness"] == "right"
    assert set(body) >= {"corpus", "baseline", "dispersion", "standing"}

    # The three layers describe the same swings, which is the property `build_standing` and
    # `build_dispersion` each build their own baseline to guarantee.
    for layer in ("baseline", "dispersion", "standing"):
        assert sorted(body[layer]["metrics"]) == sorted(m["name"] for m in _MEASUREMENTS)


def test_the_corpus_block_carries_the_evidence_behind_every_n(client, store) -> None:
    """A refusal is only actionable beside the reason the `n` is what it is.

    For the corpus on disk that reason is almost never "you have not swung enough" — it is
    re-uploads collapsing, or a swing nobody attributed — so the counts and the itemised
    exclusions travel with it.
    """
    _seed(client, store)

    corpus = client.get("/api/golfers/aaron/career").json()["corpus"]

    assert corpus["distinct_swings"] == 1
    assert corpus["swing_dirs_seen"] == 1
    assert corpus["metric_counts"] == {m["name"]: 1 for m in _MEASUREMENTS}
    assert "excluded" in corpus


def test_withheld_arrives_as_absent_over_the_wire(client, store) -> None:
    """The property the whole milestone rests on, surviving JSON.

    A statistic shipped beside a `ready: false` is one forgotten conditional away from being
    rendered. `null` is what makes the page structurally unable to print a baseline over one swing,
    and `withheld` is what makes the refusal say when it will lift.
    """
    _seed(client, store)

    body = client.get("/api/golfers/aaron/career").json()
    metric = body["baseline"]["metrics"]["head_sway_norm"]

    assert metric["mean"] is None and metric["sd"] is None and metric["mean_ci"] is None
    assert metric["n"] == 1
    assert {r["claim"] for r in metric["withheld"]} == {"center", "spread", "trend"}
    assert all(r["reason"] for r in metric["withheld"])

    assert body["standing"]["metrics"]["head_sway_norm"]["standing"] == "withheld"
    assert body["dispersion"]["metrics"]["head_sway_norm"]["bias"] == "withheld"


def test_a_metric_with_no_tour_population_says_so_rather_than_being_silent(
    client, store
) -> None:
    _seed(client, store)

    standing = client.get("/api/golfers/aaron/career").json()["standing"]["metrics"]

    assert standing["tempo_ratio"]["band_low"] is not None, "tempo has a band, just not the n"
    blocked = standing["face_to_path_deg"]["unavailable"]
    assert blocked and "no tour population exists" in blocked[0]


# ------------------------------------------------------------------------ the gates


def test_a_golfer_with_no_swings_is_an_answer_not_an_error(client, golfers) -> None:
    """The first true answer about every golfer, and `read_corpus` takes the same posture."""
    golfers.get_or_create("Bev", Handedness.LEFT)

    res = client.get("/api/golfers/bev/career")

    assert res.status_code == 200
    body = res.json()
    assert body["corpus"]["distinct_swings"] == 0
    assert body["baseline"]["metrics"] == {}


def test_an_unregistered_golfer_is_a_404(client) -> None:
    assert client.get("/api/golfers/nobody/career").status_code == 404


def test_a_path_traversal_attempt_is_rejected(client) -> None:
    """`player_id` is joined to a filesystem path, so it is validated rather than trusted —
    the same `_SAFE_SEGMENT` guard every other path parameter on this app goes through."""
    assert client.get("/api/golfers/..%2F..%2Fetc/career").status_code == 404


def test_the_route_is_behind_the_token(store, golfers) -> None:
    """Funnel makes these routes publicly reachable (ADR-016), and a career payload is the most
    personal thing this server holds — it is one golfer's whole history rather than one swing."""
    client = TestClient(create_app(store=store, golfers=golfers, token=_TOKEN, worker=None))

    assert client.get("/api/golfers/aaron/career").status_code == 401
    assert client.get(
        "/api/golfers/aaron/career", headers={"X-Upload-Token": _TOKEN}
    ).status_code == 200
