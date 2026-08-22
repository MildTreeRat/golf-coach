"""The two follow-up routes. [ADR-020]

Seeded the way `test_results.py` seeds — artifacts written straight to disk, no worker, no
pipeline — and with the conversation loop replaced by a stub. What is under test here is the
*route*: does it resume the right transcript, seed a new one from the stored analysis, persist
what came back, and turn every expected failure into a 200 with a note rather than a 500?

The loop itself is tested in `tests/feedback/test_conversation.py` against a fake client, so
nothing here needs a key, a network or the `llm` extra.
"""

from __future__ import annotations

import json
from datetime import UTC, datetime, timedelta

import pytest
from fastapi.testclient import TestClient

from golf_coach.api.app import create_app
from golf_coach.api.state import AnalysisState, save_state
from golf_coach.config import settings
from golf_coach.feedback.conversation import AskOutcome
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.transcript_store import (
    list_transcripts,
    load_transcript,
    new_transcript,
    save_transcript,
    transcript_path,
)

_ROLES = ("face_on", "down_the_line", "shot_screen")

_ANALYSIS = {
    "swing_id": "1",
    "session_id": "seeded",
    "swing": {
        "swing_id": "1",
        "session_id": "seeded",
        "overall_score": 86.0,
        "mechanics_score": 86.0,
        "checkpoint_scores": [
            {
                "name": "tempo", "score": 0.58, "passed": False, "observed": 1.89,
                "expected_low": 2.72, "expected_high": 4.71, "percentile": 10.0,
                "population_n": 1399, "one_sided": False, "message": "Tempo too quick",
            }
        ],
        "unscored": [],
    },
    "notes": [],
    "feedback": {
        "swing_id": "1", "overall_score": 86.0, "headline": "Work on tempo first.",
        "tips": [{"checkpoint": "tempo", "text": "Tempo too quick", "severity": "minor"}],
    },
}


@pytest.fixture
def conversations_dir(tmp_path, monkeypatch):
    """Point the store at a tmp dir — the routes read `settings` rather than taking a path."""
    root = tmp_path / "conversations"
    monkeypatch.setattr(settings, "conversations_dir", root)
    return root


@pytest.fixture
def store(tmp_path):
    return SwingBundleStore(tmp_path / "sessions")


@pytest.fixture
def client(store, conversations_dir):
    return TestClient(create_app(store=store, token=None, worker=None))


@pytest.fixture
def asked(monkeypatch):
    """Replace the loop with a stub that records what it was handed and answers.

    Patched where it is *used* rather than where it is defined, because the route imports it
    inside the handler — the lazy import that keeps `app.py` startable without the `llm` extra.
    """
    calls: list[dict] = []

    def _fake_ask(question, *, transcript, tools, briefing, model, brief=None, **kwargs):
        calls.append(
            {
                "question": question,
                "transcript": transcript,
                "briefing": briefing,
                "brief": brief,
                "tool_names": [tool.name for tool in tools],
            }
        )
        transcript.messages = [
            *transcript.messages,
            {"role": "user", "content": question},
            {"role": "assistant", "content": [{"type": "text", "text": "Because tempo."}]},
        ]
        return AskOutcome(
            text="Because tempo.", transcript=transcript, tools_called=["get_swing"]
        )

    monkeypatch.setattr("golf_coach.feedback.conversation.ask", _fake_ask)
    return calls


def _seed(client, store, *, analysis: bool = True) -> str:
    # M9 P6: an upload 409s without a club cursor; these tests are not about the club.
    client.post("/api/sessions/current/club", json={"club": "7i"})
    for role in _ROLES:
        res = client.post(
            "/api/uploads", params={"role": role, "filename": f"{role}.mov"},
            content=role.encode(),
        )
    session_id = res.json()["session_id"]
    swing_dir = store.root / session_id / "1"
    if analysis:
        (swing_dir / "analysis.json").write_text(json.dumps(_ANALYSIS), encoding="utf-8")
    save_state(
        AnalysisState(status="done", inputs={}, score=86.0, headline="Work on tempo first."),
        swing_dir,
    )
    return session_id


# --------------------------------------------------------------------------- asking


def test_a_first_question_seeds_from_the_stored_analysis(
    client, store, asked, conversations_dir
) -> None:
    """The brief is `build_brief` over this swing's own `analysis.json`.

    One rendering, shared with the coaching paragraph already on the page — so the conversation
    and the verdict above it cannot describe the same swing differently.
    """
    session_id = _seed(client, store)

    body = client.post(
        f"/api/sessions/{session_id}/swings/1/ask", json={"question": "why?"}
    ).json()

    assert body["text"] == "Because tempo."
    assert body["tools_called"] == ["get_swing"]
    assert body["note"] is None

    assert "Tempo too quick" in asked[0]["brief"]
    assert "overall_score: 86" in asked[0]["brief"]
    assert list_transcripts(conversations_dir)[0].conversation_id == body["conversation_id"]


def test_the_answer_is_persisted(client, store, asked, conversations_dir) -> None:
    session_id = _seed(client, store)

    body = client.post(
        f"/api/sessions/{session_id}/swings/1/ask", json={"question": "why?"}
    ).json()
    stored = load_transcript(conversations_dir, body["conversation_id"])

    assert stored is not None
    assert stored.session_id == session_id
    assert stored.swing_id == "1"
    assert len(stored.messages) == 2


def test_a_second_question_continues_the_same_conversation(
    client, store, asked, conversations_dir
) -> None:
    session_id = _seed(client, store)
    first = client.post(
        f"/api/sessions/{session_id}/swings/1/ask", json={"question": "why?"}
    ).json()

    second = client.post(
        f"/api/sessions/{session_id}/swings/1/ask",
        json={"question": "and now?", "conversation_id": first["conversation_id"]},
    ).json()

    assert second["conversation_id"] == first["conversation_id"]
    assert len(list_transcripts(conversations_dir)) == 1
    stored = load_transcript(conversations_dir, first["conversation_id"])
    assert stored is not None and len(stored.messages) == 4
    # A resumed turn is not reseeded — the brief is already in the history.
    assert asked[1]["brief"] is None


def test_the_tools_offered_match_the_configured_stores(client, store, asked) -> None:
    session_id = _seed(client, store)

    client.post(f"/api/sessions/{session_id}/swings/1/ask", json={"question": "why?"})

    assert "get_swing" in asked[0]["tool_names"]
    assert "list_sessions" in asked[0]["tool_names"]


def test_a_swing_with_no_analysis_yet_still_answers(client, store, asked) -> None:
    """Nothing to seed from, so the model looks everything up through the tools instead."""
    session_id = _seed(client, store, analysis=False)

    body = client.post(
        f"/api/sessions/{session_id}/swings/1/ask", json={"question": "why?"}
    ).json()

    assert body["text"] == "Because tempo."
    assert asked[0]["brief"] is None


# --------------------------------------------------------------------------- refusals


def test_an_unknown_swing_is_a_404(client, store, asked) -> None:
    res = client.post("/api/sessions/nope/swings/1/ask", json={"question": "why?"})

    assert res.status_code == 404


def test_an_unknown_conversation_is_a_404_not_a_new_thread(client, store, asked) -> None:
    """A typo must not silently start a fresh conversation the user thinks is their old one."""
    session_id = _seed(client, store)

    res = client.post(
        f"/api/sessions/{session_id}/swings/1/ask",
        json={"question": "why?", "conversation_id": "conv-2026-08-15-deadbeef"},
    )

    assert res.status_code == 404


def test_a_traversing_conversation_id_is_a_404(client, store, asked) -> None:
    session_id = _seed(client, store)

    res = client.post(
        f"/api/sessions/{session_id}/swings/1/ask",
        json={"question": "why?", "conversation_id": "../../../etc/passwd"},
    )

    assert res.status_code == 404


@pytest.mark.parametrize("question", ["", "   "])
def test_an_empty_question_is_a_400(client, store, asked, question: str) -> None:
    session_id = _seed(client, store)

    res = client.post(
        f"/api/sessions/{session_id}/swings/1/ask", json={"question": question}
    )

    assert res.status_code == 400


def test_a_failed_ask_is_a_200_with_a_note(client, store, monkeypatch) -> None:
    """Coaching must never be able to break the page a score is rendered on.

    A 500 here would take out the results view of a swing whose analysis is fine — so no key, no
    extra, a rate limit and a refusal all arrive as a note beside a null answer.
    """
    session_id = _seed(client, store)
    monkeypatch.setattr(
        "golf_coach.feedback.conversation.ask",
        lambda *a, **k: AskOutcome(note="no answer: the coaching request was rate limited."),
    )

    res = client.post(f"/api/sessions/{session_id}/swings/1/ask", json={"question": "why?"})

    assert res.status_code == 200
    assert res.json()["text"] is None
    assert "rate limited" in res.json()["note"]


def test_a_failed_ask_stores_nothing(client, store, monkeypatch, conversations_dir) -> None:
    session_id = _seed(client, store)
    monkeypatch.setattr(
        "golf_coach.feedback.conversation.ask",
        lambda *a, **k: AskOutcome(note="no answer: no Anthropic API key is configured."),
    )

    client.post(f"/api/sessions/{session_id}/swings/1/ask", json={"question": "why?"})

    assert list_transcripts(conversations_dir) == []


# --------------------------------------------------------------------------- reading back


def test_a_conversation_renders_its_turns(client, conversations_dir) -> None:
    transcript = new_transcript(model="claude-opus-5", session_id="s", swing_id="1")
    transcript.messages = [
        {"role": "user", "content": "why?"},
        {
            "role": "assistant",
            "content": [
                {"type": "thinking", "thinking": "", "signature": "sig"},
                {"type": "tool_use", "id": "t1", "name": "get_swing", "input": {}},
            ],
        },
        {
            "role": "user",
            "content": [{"type": "tool_result", "tool_use_id": "t1", "content": "{}"}],
        },
        {"role": "assistant", "content": [{"type": "text", "text": "Because tempo."}]},
    ]
    save_transcript(transcript, conversations_dir)

    body = client.get(f"/api/conversations/{transcript.conversation_id}").json()

    assert body["turns"] == [
        {"role": "user", "text": "why?"},
        {"role": "assistant", "text": "Because tempo."},
    ]
    assert body["model"] == "claude-opus-5"


def test_reading_an_unknown_conversation_is_a_404(client, conversations_dir) -> None:
    assert client.get("/api/conversations/conv-2026-08-15-deadbeef").status_code == 404


def test_reading_a_traversing_id_is_a_404(client, conversations_dir) -> None:
    assert client.get("/api/conversations/..%2F..%2Fsecret").status_code == 404


# ------------------------------------------------------ resuming a swing's thread on page load


def _seed_conversation(
    conversations_dir, *, session_id: str, swing_id: str, updated_at: datetime, text: str
) -> str:
    """A stored two-turn conversation for a swing, with `updated_at` forced past the writer.

    `save_transcript` always stamps `updated_at` to now, so the file is rewritten with the value
    the test needs — the same trick `tests/storage/test_transcript_store.py` uses.
    """
    transcript = new_transcript(model="claude-opus-5", session_id=session_id, swing_id=swing_id)
    transcript.messages = [
        {"role": "user", "content": "why?"},
        {"role": "assistant", "content": [{"type": "text", "text": text}]},
    ]
    save_transcript(transcript, conversations_dir)
    stored = load_transcript(conversations_dir, transcript.conversation_id)
    assert stored is not None
    stored.updated_at = updated_at
    transcript_path(conversations_dir, stored.conversation_id).write_text(
        stored.model_dump_json(indent=2), encoding="utf-8"
    )
    return stored.conversation_id


def test_a_swings_latest_conversation_is_returned_for_resume(client, conversations_dir) -> None:
    """The results page holds no id after a reload; this is how it finds the thread to resume."""
    base = datetime(2026, 8, 15, 12, 0, tzinfo=UTC)
    _seed_conversation(
        conversations_dir, session_id="s", swing_id="1", updated_at=base, text="Older answer."
    )
    newest = _seed_conversation(
        conversations_dir, session_id="s", swing_id="1",
        updated_at=base + timedelta(hours=1), text="Newer answer.",
    )

    body = client.get("/api/sessions/s/swings/1/conversation").json()

    assert body["conversation_id"] == newest
    assert body["turns"] == [
        {"role": "user", "text": "why?"},
        {"role": "assistant", "text": "Newer answer."},
    ]


def test_a_swing_with_no_conversation_is_an_empty_thread_not_a_404(
    client, conversations_dir
) -> None:
    """First-visit state: the panel renders empty either way, so a 200 rather than a 404."""
    res = client.get("/api/sessions/s/swings/1/conversation")

    assert res.status_code == 200
    assert res.json() == {"conversation_id": None, "turns": []}


def test_resume_is_scoped_to_the_swing_not_the_clock(client, conversations_dir) -> None:
    """A newer thread on a *different* swing must not be picked up by this one."""
    base = datetime(2026, 8, 15, 12, 0, tzinfo=UTC)
    mine = _seed_conversation(
        conversations_dir, session_id="s", swing_id="1", updated_at=base, text="Mine."
    )
    _seed_conversation(
        conversations_dir, session_id="s", swing_id="2",
        updated_at=base + timedelta(hours=1), text="Another swing.",
    )

    body = client.get("/api/sessions/s/swings/1/conversation").json()

    assert body["conversation_id"] == mine
