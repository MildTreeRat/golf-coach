"""`POST /api/sessions/{session_id}/swings/{swing_id}/mishit` — closed at M29. [M16 P5, M29 P32]

The golfer's verdict on a shot is recorded by `golf-core flag-mishit` now (the M29 plan's D11), so
this route answers 410 naming that command for the swing it was asked about, and writes nothing.
What the route used to check is held elsewhere: the exclusion arithmetic over contracts in
`tests/storage/test_corpus.py` and `tests/analysis/test_club_profile_builder.py`,
`bundle_store.set_mishit` in `tests/storage/test_bundle_store.py`, and the verb in
`crates/core/tests/reports.rs` against the career family's recorded `flag_mishit.py` answers.
`tests/api/test_uploads.py` holds every closed route to the same refusal; this file holds the one
property only a verdict has, that a verdict already on disk is left as it was.
"""

from __future__ import annotations

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.mishit import MishitVerdict
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.golfer_store import GolferStore


@pytest.fixture
def store(tmp_path) -> SwingBundleStore:
    return SwingBundleStore(tmp_path / "sessions")


@pytest.fixture
def golfers(tmp_path) -> GolferStore:
    store = GolferStore(tmp_path / "golfers")
    store.get_or_create("Aaron", Handedness.RIGHT)
    return store


@pytest.fixture
def client(store, golfers) -> TestClient:
    return TestClient(create_app(store=store, golfers=golfers, token=None, worker=None))


@pytest.mark.parametrize("verdict", ["confirmed", "cleared", None, "topped"])
def test_the_route_refuses_naming_flag_mishit_and_leaves_the_verdict(
    client, store, file_swing, verdict
) -> None:
    for role in ("face_on", "down_the_line", "shot_screen"):
        filed = file_swing(store, role, f"{role}-a".encode(), player_id="aaron")
    session_id, swing_id = filed.session_id, filed.swing_id
    store.set_mishit(session_id, swing_id, MishitVerdict.CONFIRMED)
    before = store.get_swing(session_id, swing_id)

    res = client.post(
        f"/api/sessions/{session_id}/swings/{swing_id}/mishit", json={"verdict": verdict}
    )

    assert res.status_code == 410
    assert (
        f"golf-core flag-mishit {session_id}/{swing_id} --confirm, --clear or --auto"
        in res.json()["detail"]
    )
    assert store.get_swing(session_id, swing_id) == before
