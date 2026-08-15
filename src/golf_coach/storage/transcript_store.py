"""Reading and writing conversation transcripts. [M6, ADR-020]

One JSON file per conversation under `settings.conversations_dir`, flat. Deliberately **beside**
`sessions_dir` rather than inside a swing directory: a conversation seeded from one swing can
range across sessions ("was that worse than Tuesday?"), so filing it under the swing it started
from would put it in a directory it has outgrown by turn two. Same reasoning that put
`golfers_dir` beside sessions rather than in them — a golfer outlives any one session.

The read/write shape is `api/state.py`'s, on purpose:

  - **`load_transcript` is tolerant.** A half-written or older-schema file reads as *absent*, never
    raises. A conversation is the least important thing on disk; losing one costs a re-ask, and a
    corrupt one must not take down the results page it is rendered on.
  - **`save_transcript` is atomic.** The web route rewrites a transcript while a `GET` may be
    reading it.

Nothing here imports the Anthropic SDK. The blocks it moves are opaque dicts (see
`contracts/conversation.py`), which is what lets the store be tested on a base install.
"""

from __future__ import annotations

import re
import secrets
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from golf_coach.contracts.conversation import Transcript

#: `conv-<date>-<random>`. The date makes a directory listing legible to a human scanning for
#: "the one from yesterday"; the suffix is random rather than sequential because two conversations
#: can be started in the same second by the CLI and the web UI, and a counter would need a lock.
_ID = re.compile(r"^conv-\d{4}-\d{2}-\d{2}-[0-9a-f]{8}$")


def new_conversation_id(*, now: datetime | None = None) -> str:
    stamp = (now or datetime.now(tz=UTC)).strftime("%Y-%m-%d")
    return f"conv-{stamp}-{secrets.token_hex(4)}"


def is_conversation_id(value: str) -> bool:
    """Does this look like an id this module minted?

    The guard for anything that turns a caller-supplied string into a path. `api/app.py` has
    `_safe` for path segments generally; this is the narrower question — a transcript id has one
    shape, so accepting only that shape is cheaper than sanitising an arbitrary one.
    """
    return bool(_ID.match(value))


def transcript_path(conversations_dir: Path, conversation_id: str) -> Path:
    return conversations_dir / f"{conversation_id}.json"


def load_transcript(conversations_dir: Path, conversation_id: str) -> Transcript | None:
    """The stored conversation, or None if there is none, it is unreadable, or the id is not ours.

    Tolerant on purpose, mirroring `api.state.load_state` and `storage.manifest.load_manifest`.
    The id check is first because it is the only one of the three failures that can be *hostile*:
    without it a `conversation_id` of `../../secrets` reads a file this module never wrote.
    """
    if not is_conversation_id(conversation_id):
        return None
    path = transcript_path(conversations_dir, conversation_id)
    try:
        return Transcript.model_validate_json(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def save_transcript(transcript: Transcript, conversations_dir: Path) -> Path:
    """Write atomically, stamping `updated_at`. Returns the path written."""
    transcript.updated_at = datetime.now(tz=UTC)
    path = transcript_path(conversations_dir, transcript.conversation_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".tmp")
    tmp.write_text(transcript.model_dump_json(indent=2), encoding="utf-8")
    tmp.replace(path)
    return path


def list_transcripts(conversations_dir: Path, *, limit: int = 20) -> list[Transcript]:
    """Conversations newest-first by `updated_at`, unreadable ones skipped.

    Sorted on the parsed field rather than mtime: a transcript copied between machines keeps its
    own history, and mtime would reorder the whole directory on a `cp -r`.
    """
    if not conversations_dir.exists():
        return []
    found = [
        transcript
        for path in conversations_dir.glob("conv-*.json")
        if (transcript := load_transcript(conversations_dir, path.stem)) is not None
    ]
    found.sort(key=lambda t: t.updated_at, reverse=True)
    return found[: max(0, limit)]


def new_transcript(
    *,
    model: str,
    session_id: str | None = None,
    swing_id: str | None = None,
    brief_digest: str | None = None,
    conversation_id: str | None = None,
) -> Transcript:
    """An empty transcript stamped now. The one place `created_at`/`updated_at` start equal."""
    now = datetime.now(tz=UTC)
    return Transcript(
        conversation_id=conversation_id or new_conversation_id(now=now),
        created_at=now,
        updated_at=now,
        model=model,
        session_id=session_id,
        swing_id=swing_id,
        brief_digest=brief_digest,
    )


def visible_turns(transcript: Transcript) -> list[dict[str, Any]]:
    """The conversation as a reader sees it: `[{"role", "text"}]`, oldest first.

    The **only** place a transcript is flattened, and it is a rendering — the stored blocks are
    untouched (`contracts/conversation.py` explains why that matters). Thinking blocks are
    dropped, and so are tool blocks: a `tool_result` is a JSON payload of a shape a golfer has no
    use for, and the answer that quotes what mattered from it is the next text block along.

    **The seeded first turn shows only its question.** A seeded conversation opens with the whole
    swing brief in the first user message (`feedback.conversation.seed_turn`), and the golfer did
    not type that — rendering it whole opens the panel with several hundred words of measurements
    in a bubble labelled as theirs. `brief_digest` is what says the first turn was seeded, and the
    question is its last block, which is why `seed_turn` keeps the two as separate blocks rather
    than one joined string.

    A turn whose text is empty renders as nothing rather than as a blank bubble — that is what an
    assistant turn consisting only of tool calls actually is.
    """
    out: list[dict[str, Any]] = []
    for index, message in enumerate(transcript.messages):
        role = message.get("role")
        if role not in ("user", "assistant"):
            continue
        blocks = [
            block
            for block in _blocks(message.get("content"))
            if block.get("type") == "text" and isinstance(block.get("text"), str)
        ]
        if index == 0 and role == "user" and transcript.brief_digest and len(blocks) > 1:
            blocks = blocks[-1:]
        text = "\n".join(block["text"] for block in blocks).strip()
        if text:
            out.append({"role": role, "text": text})
    return out


def _blocks(content: Any) -> list[dict[str, Any]]:
    """Content as a block list.

    A message's `content` is either a list of blocks or a bare string — the API accepts both, and
    a seeded user turn is written the short way. Normalising here keeps that convenience from
    reaching every reader.
    """
    if isinstance(content, str):
        return [{"type": "text", "text": content}]
    if isinstance(content, list):
        return [block for block in content if isinstance(block, dict)]
    return []
