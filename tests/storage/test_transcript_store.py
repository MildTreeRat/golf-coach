"""The transcript store: round-trip fidelity, tolerance, and the traversal guard. [ADR-020]

The load-bearing test in here is `test_a_round_trip_preserves_thinking_and_tool_blocks`. Every
other property is convenience; that one is correctness. A transcript that loses a thinking block
or splits a `tool_use`/`tool_result` pair cannot be replayed, and the way that failure presents
is a 400 on the *third* turn of a conversation that worked twice.
"""

from __future__ import annotations

import json
from datetime import UTC, datetime, timedelta
from pathlib import Path

import pytest

from golf_coach.contracts.conversation import TRANSCRIPT_VERSION, Transcript
from golf_coach.storage.transcript_store import (
    is_conversation_id,
    list_transcripts,
    load_transcript,
    new_conversation_id,
    new_transcript,
    save_transcript,
    transcript_path,
    visible_turns,
)

MODEL = "claude-opus-5"


@pytest.fixture
def conversations_dir(tmp_path: Path) -> Path:
    return tmp_path / "conversations"


#: One turn's worth of the real wire shape: a thinking block, an answer, a tool call, its result.
#: Written out rather than fetched so the assertion holds without a network or the `llm` extra.
TURN = [
    {
        "role": "user",
        "content": [{"type": "text", "text": "why is my tempo quick?"}],
    },
    {
        "role": "assistant",
        "content": [
            {"type": "thinking", "thinking": "", "signature": "abc123=="},
            {"type": "text", "text": "Let me look at the swing."},
            {
                "type": "tool_use",
                "id": "toolu_01",
                "name": "get_swing",
                "input": {"session_id": "2026-08-10", "swing_id": "2"},
            },
        ],
    },
    {
        "role": "user",
        "content": [
            {
                "type": "tool_result",
                "tool_use_id": "toolu_01",
                "content": '{"overall_score": 71.2}',
            }
        ],
    },
    {
        "role": "assistant",
        "content": [{"type": "text", "text": "Your tempo sits outside the band."}],
    },
]


def test_a_round_trip_preserves_thinking_and_tool_blocks(conversations_dir: Path) -> None:
    """The reason this store stores blocks rather than prose.

    Compared against the input structure rather than field-by-field: the point is that *nothing*
    was dropped, normalised or reordered, and an assertion naming the fields it checks would pass
    over a block type nobody thought to name.
    """
    transcript = new_transcript(model=MODEL, session_id="2026-08-10", swing_id="2")
    transcript.messages = json.loads(json.dumps(TURN))

    save_transcript(transcript, conversations_dir)
    loaded = load_transcript(conversations_dir, transcript.conversation_id)

    assert loaded is not None
    assert loaded.messages == TURN

    signatures = [
        block.get("signature")
        for message in loaded.messages
        for block in message["content"]
        if block.get("type") == "thinking"
    ]
    assert signatures == ["abc123=="], "the thinking block's signature did not survive the trip"


def test_save_stamps_updated_at_without_touching_created_at(conversations_dir: Path) -> None:
    transcript = new_transcript(model=MODEL)
    created = transcript.created_at
    transcript.updated_at = created - timedelta(hours=1)

    save_transcript(transcript, conversations_dir)
    loaded = load_transcript(conversations_dir, transcript.conversation_id)

    assert loaded is not None
    assert loaded.created_at == created
    assert loaded.updated_at >= created


def test_an_unreadable_transcript_reads_as_absent(conversations_dir: Path) -> None:
    """Tolerant like `api.state.load_state`: a corrupt file costs a re-ask, not a 500."""
    conversation_id = new_conversation_id()
    path = transcript_path(conversations_dir, conversation_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("{ half a write", encoding="utf-8")

    assert load_transcript(conversations_dir, conversation_id) is None


def test_a_missing_transcript_reads_as_absent(conversations_dir: Path) -> None:
    assert load_transcript(conversations_dir, new_conversation_id()) is None


@pytest.mark.parametrize(
    "hostile",
    [
        "../../../etc/passwd",
        "conv-2026-08-15-../../secret",
        "..",
        "",
        "conv-2026-08-15-XYZ",
        "arbitrary-name",
    ],
)
def test_an_id_that_is_not_ours_never_reaches_the_filesystem(
    conversations_dir: Path, hostile: str
) -> None:
    """`conversation_id` arrives from a URL path on the web route.

    The other two `load_transcript` failures are accidents; this one is the hostile case, which is
    why the shape check runs before the path is built rather than after.
    """
    assert not is_conversation_id(hostile)
    assert load_transcript(conversations_dir, hostile) is None


def test_minted_ids_are_accepted_and_unique() -> None:
    ids = {new_conversation_id() for _ in range(50)}

    assert len(ids) == 50, "two conversations minted the same id"
    assert all(is_conversation_id(value) for value in ids)


def test_listing_is_newest_first_and_skips_unreadable(conversations_dir: Path) -> None:
    base = datetime(2026, 8, 15, 12, 0, tzinfo=UTC)
    for index in range(3):
        transcript = new_transcript(model=MODEL)
        save_transcript(transcript, conversations_dir)
        # Rewrite `updated_at` under the atomic writer, which always stamps it to now.
        stored = load_transcript(conversations_dir, transcript.conversation_id)
        assert stored is not None
        stored.updated_at = base + timedelta(hours=index)
        transcript_path(conversations_dir, stored.conversation_id).write_text(
            stored.model_dump_json(indent=2), encoding="utf-8"
        )

    (conversations_dir / "conv-2026-08-15-deadbeef.json").write_text("nonsense", encoding="utf-8")

    listed = list_transcripts(conversations_dir)

    assert len(listed) == 3, "the unreadable file was counted"
    assert [t.updated_at for t in listed] == sorted(
        (t.updated_at for t in listed), reverse=True
    )


def test_listing_an_absent_directory_is_empty(tmp_path: Path) -> None:
    assert list_transcripts(tmp_path / "never-created") == []


def test_replayable_by_rejects_another_model_and_an_older_schema() -> None:
    """Both halves of the replay guard, because both fail the same way and late.

    Thinking blocks are bound to the model that produced them, so the cost of getting this wrong
    is a 400 partway through an old conversation rather than at the point the config changed.
    """
    transcript = new_transcript(model=MODEL)

    assert transcript.replayable_by(MODEL)
    assert not transcript.replayable_by("claude-opus-4-8")

    transcript.transcript_version = TRANSCRIPT_VERSION - 1
    assert not transcript.replayable_by(MODEL)


def test_visible_turns_renders_prose_and_drops_the_machinery() -> None:
    """What a golfer reads. Thinking and tool blocks are not it."""
    transcript = new_transcript(model=MODEL)
    transcript.messages = json.loads(json.dumps(TURN))

    assert visible_turns(transcript) == [
        {"role": "user", "text": "why is my tempo quick?"},
        {"role": "assistant", "text": "Let me look at the swing."},
        {"role": "assistant", "text": "Your tempo sits outside the band."},
    ]


def test_visible_turns_accepts_a_bare_string_content() -> None:
    """A seeded user turn is written the short way; the API accepts both shapes."""
    transcript = new_transcript(model=MODEL)
    transcript.messages = [{"role": "user", "content": "here is the brief"}]

    assert visible_turns(transcript) == [{"role": "user", "text": "here is the brief"}]


def test_a_tool_only_assistant_turn_renders_as_nothing(conversations_dir: Path) -> None:
    """Not a blank bubble — an assistant turn that only called a tool said nothing to read."""
    transcript = new_transcript(model=MODEL)
    transcript.messages = [
        {
            "role": "assistant",
            "content": [
                {"type": "tool_use", "id": "toolu_02", "name": "list_sessions", "input": {}}
            ],
        }
    ]

    assert visible_turns(transcript) == []


def test_the_stored_file_is_json_a_human_can_read(conversations_dir: Path) -> None:
    """Indented, like every other artifact this repo writes. Someone will open one in an editor."""
    transcript = new_transcript(model=MODEL, session_id="2026-08-10", swing_id="2")
    path = save_transcript(transcript, conversations_dir)

    text = path.read_text(encoding="utf-8")

    assert "\n  " in text
    assert json.loads(text)["conversation_id"] == transcript.conversation_id
    assert not list(conversations_dir.glob("*.tmp")), "the atomic write left its temp file behind"


def test_a_transcript_validates_from_plain_json(conversations_dir: Path) -> None:
    """The shape is a contract, so a hand-written file with the documented fields must load."""
    conversation_id = new_conversation_id()
    transcript_path(conversations_dir, conversation_id).parent.mkdir(parents=True, exist_ok=True)
    transcript_path(conversations_dir, conversation_id).write_text(
        json.dumps(
            {
                "conversation_id": conversation_id,
                "transcript_version": TRANSCRIPT_VERSION,
                "created_at": "2026-08-15T12:00:00Z",
                "updated_at": "2026-08-15T12:00:00Z",
                "model": MODEL,
                "messages": [{"role": "user", "content": "hello"}],
            }
        ),
        encoding="utf-8",
    )

    loaded = load_transcript(conversations_dir, conversation_id)

    assert isinstance(loaded, Transcript)
    assert loaded.notes == []
    assert loaded.brief_digest is None
