"""The conversation loop, driven by a fake client. [ADR-020]

Runs on a base install: no key, no network, no `llm` extra. That is the point of `ask()` taking
its client, tools and briefing as arguments — everything worth asserting here is reachable without
any of them.

The load-bearing test is `test_a_tool_calling_turn_is_mirrored_verbatim`. A transcript that loses
a thinking block or splits a `tool_use`/`tool_result` pair cannot be replayed, and the way that
failure presents is a 400 on the *third* turn of a conversation that worked twice — long after the
code that caused it.
"""

from __future__ import annotations

from typing import Any

import pytest

from golf_coach.contracts.conversation import Transcript
from golf_coach.feedback.conversation import (
    ANSWERING_FOLLOWUPS,
    MAX_ITERATIONS,
    SEED_PREAMBLE,
    AskOutcome,
    ask,
    brief_digest,
    system_prompt,
)
from golf_coach.storage.transcript_store import new_transcript, visible_turns

MODEL = "claude-opus-5"
BRIEFING = "Swing analysis and launch-monitor data from a home golf simulator."


# --------------------------------------------------------------------------- fakes


class _Block:
    """A content block with the two things the loop reads: `.type` and `.model_dump()`."""

    def __init__(self, **data: Any) -> None:
        self._data = data
        self.type = data["type"]
        self.name = data.get("name")
        self.text = data.get("text", "")

    def model_dump(self, **_: Any) -> dict[str, Any]:
        return {key: value for key, value in self._data.items() if value is not None}


class _Message:
    def __init__(self, blocks: list[dict[str, Any]], stop_reason: str = "end_turn") -> None:
        self.content = [_Block(**block) for block in blocks]
        self.stop_reason = stop_reason


class _Runner:
    """Yields canned turns and, like the real runner, pairs each with its tool result.

    `generate_tool_call_response` returns `None` on a turn that called no tools — the signal
    `_mirror` uses to know the turn is final — and is cached, which the real one relies on to
    avoid running tools twice.
    """

    def __init__(self, turns: list[_Message]) -> None:
        self._turns = turns
        self._current: _Message | None = None
        self._served: set[int] = set()
        self.captured: dict[str, Any] = {}

    def __iter__(self):
        for turn in self._turns:
            self._current = turn
            yield turn

    def generate_tool_call_response(self) -> dict[str, Any] | None:
        assert self._current is not None
        calls = [block for block in self._current.content if block.type == "tool_use"]
        if not calls:
            return None
        assert id(self._current) not in self._served, "tools were run twice for one turn"
        self._served.add(id(self._current))
        return {
            "role": "user",
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": block._data["id"],
                    "content": '{"overall_score": 93.8}',
                }
                for block in calls
            ],
        }


class _Client:
    def __init__(self, turns: list[_Message] | Exception) -> None:
        self._turns = turns
        self.captured: dict[str, Any] = {}
        self.beta = self

    @property
    def messages(self):  # noqa: D401 - mirrors `client.beta.messages`
        return self

    def tool_runner(self, **kwargs: Any) -> _Runner:
        # Snapshot `messages` rather than aliasing it. The real runner does the same
        # (`{**params, "messages": [m for m in params["messages"]]}`), and it matters here: `ask`
        # keeps appending this turn's replies to the very list it passed in, so a captured
        # reference would show the finished conversation rather than what was sent.
        self.captured = {**kwargs, "messages": list(kwargs.get("messages", []))}
        if isinstance(self._turns, Exception):
            raise self._turns
        return _Runner(self._turns)


def answer(text: str = "Your tempo sits outside the band.") -> list[_Message]:
    return [_Message([{"type": "text", "text": text}])]


def transcript() -> Transcript:
    return new_transcript(model=MODEL, session_id="2026-08-10", swing_id="2")


# --------------------------------------------------------------------------- the mirror


def test_a_tool_calling_turn_is_mirrored_verbatim() -> None:
    """Thinking blocks, the tool call, its result and the answer — in order, nothing dropped.

    Asserted as a whole structure rather than field by field: the property is that *nothing* was
    lost, and an assertion naming the fields it checks would pass over a block type nobody
    thought to name.
    """
    turns = [
        _Message(
            [
                {"type": "thinking", "thinking": "", "signature": "sig-abc"},
                {"type": "text", "text": "Let me look."},
                {
                    "type": "tool_use",
                    "id": "toolu_01",
                    "name": "get_swing",
                    "input": {"session_id": "2026-08-10", "swing_id": "2"},
                },
            ],
            stop_reason="tool_use",
        ),
        _Message([{"type": "text", "text": "Your tempo is quick."}]),
    ]
    stored = transcript()

    outcome = ask(
        "why is my tempo quick?",
        transcript=stored,
        tools=[],
        briefing=BRIEFING,
        model=MODEL,
        client=_Client(turns),
    )

    assert outcome.text == "Your tempo is quick."
    assert outcome.tools_called == ["get_swing"]
    assert [message["role"] for message in stored.messages] == [
        "user",
        "assistant",
        "user",
        "assistant",
    ]
    assert stored.messages[1]["content"] == [
        {"type": "thinking", "thinking": "", "signature": "sig-abc"},
        {"type": "text", "text": "Let me look."},
        {
            "type": "tool_use",
            "id": "toolu_01",
            "name": "get_swing",
            "input": {"session_id": "2026-08-10", "swing_id": "2"},
        },
    ]
    assert stored.messages[2]["content"][0]["tool_use_id"] == "toolu_01"


def test_the_signature_on_a_thinking_block_survives() -> None:
    """Called out separately because it is what makes a replay legal, and it is one field.

    An empty `thinking` text is normal — `display` defaults to omitted — so a mirror that skipped
    "empty" blocks would look reasonable and would break the next turn.
    """
    turns = [
        _Message(
            [
                {"type": "thinking", "thinking": "", "signature": "sig-xyz"},
                {"type": "text", "text": "Answer."},
            ]
        )
    ]
    stored = transcript()

    ask("q", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL, client=_Client(turns))

    thinking = [
        block
        for message in stored.messages
        if isinstance(message["content"], list)
        for block in message["content"]
        if block.get("type") == "thinking"
    ]
    assert thinking == [{"type": "thinking", "thinking": "", "signature": "sig-xyz"}]


def test_a_second_question_grows_the_same_transcript() -> None:
    """Resuming is the feature. The second turn must append, not restart."""
    stored = transcript()
    ask("first?", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL,
        client=_Client(answer("One.")))
    after_first = len(stored.messages)

    ask("second?", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL,
        client=_Client(answer("Two.")))

    assert len(stored.messages) == after_first + 2
    assert visible_turns(stored) == [
        {"role": "user", "text": "first?"},
        {"role": "assistant", "text": "One."},
        {"role": "user", "text": "second?"},
        {"role": "assistant", "text": "Two."},
    ]


def test_the_second_question_replays_the_whole_history() -> None:
    """The API is stateless, so turn two must resend turn one — including the model's blocks."""
    stored = transcript()
    ask("first?", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL,
        client=_Client(answer("One.")))

    client = _Client(answer("Two."))
    ask("second?", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL, client=client)

    sent = client.captured["messages"]
    assert len(sent) == 3
    assert sent[0]["content"].endswith("first?")
    assert sent[1]["content"] == [{"type": "text", "text": "One."}]


# --------------------------------------------------------------------------- seeding


def test_the_first_turn_carries_the_brief_and_records_its_digest() -> None:
    stored = transcript()
    brief = "SWING 2 (session 2026-08-10)\n\noverall_score: 93.8 out of 100"

    ask("why?", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL, brief=brief,
        client=_Client(answer()))

    seeded = stored.messages[0]["content"]
    # Two blocks, not one joined string: the golfer typed the question and not the brief, and
    # `visible_turns` needs a boundary it can render on that no rewording can move.
    assert [block["type"] for block in seeded] == ["text", "text"]
    assert SEED_PREAMBLE in seeded[0]["text"]
    assert brief in seeded[0]["text"]
    assert seeded[1]["text"] == "why?"
    assert stored.brief_digest == brief_digest(brief)


def test_a_seeded_conversation_shows_the_golfer_only_their_question() -> None:
    """The bug this shape exists to prevent: the panel opening with the brief attributed to you."""
    stored = transcript()
    brief = "SWING 2\n\noverall_score: 93.8 out of 100\ntempo: observed 2.35"

    ask("why is my tempo quick?", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL,
        brief=brief, client=_Client(answer("Because the backswing is short.")))

    assert visible_turns(stored) == [
        {"role": "user", "text": "why is my tempo quick?"},
        {"role": "assistant", "text": "Because the backswing is short."},
    ]


def test_a_resumed_conversation_is_not_reseeded() -> None:
    """The brief is already in the history; sending it again would double it every turn."""
    stored = transcript()
    brief = "SWING 2"
    ask("first?", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL, brief=brief,
        client=_Client(answer("One.")))

    ask("second?", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL, brief=brief,
        client=_Client(answer("Two.")))

    assert stored.messages[2]["content"] == "second?"
    assert sum(brief in str(message["content"]) for message in stored.messages) == 1


def test_the_system_prompt_carries_the_briefing_and_is_cacheable() -> None:
    """One block, one breakpoint — identical for every turn of every conversation."""
    blocks = system_prompt(BRIEFING)

    assert len(blocks) == 1
    assert BRIEFING in blocks[0]["text"]
    assert ANSWERING_FOLLOWUPS in blocks[0]["text"]
    assert blocks[0]["cache_control"] == {"type": "ephemeral"}


def test_the_request_asks_for_adaptive_thinking_and_no_sampling_params() -> None:
    """`temperature`, `top_p`, `top_k` and `budget_tokens` are all 400s on Opus 5."""
    client = _Client(answer())
    ask("q", transcript=transcript(), tools=[], briefing=BRIEFING, model=MODEL, client=client)

    assert client.captured["thinking"] == {"type": "adaptive"}
    assert client.captured["output_config"] == {"effort": "medium"}
    assert client.captured["max_iterations"] == MAX_ITERATIONS
    assert not {"temperature", "top_p", "top_k", "budget_tokens"} & set(client.captured)


# --------------------------------------------------------------------------- failures


def test_no_key_is_a_note_not_an_exception() -> None:
    outcome = ask("q", transcript=transcript(), tools=[], briefing=BRIEFING, model=MODEL)

    assert isinstance(outcome, AskOutcome)
    assert outcome.text is None
    assert outcome.transcript is None
    assert "no Anthropic API key" in (outcome.note or "")


def test_an_sdk_failure_becomes_a_note() -> None:
    outcome = ask("q", transcript=transcript(), tools=[], briefing=BRIEFING, model=MODEL,
                  client=_Client(RuntimeError("connection reset")))

    assert outcome.text is None
    assert "connection reset" in (outcome.note or "")


def test_a_refusal_becomes_a_note() -> None:
    turns = [_Message([{"type": "text", "text": ""}], stop_reason="refusal")]

    outcome = ask("q", transcript=transcript(), tools=[], briefing=BRIEFING, model=MODEL,
                  client=_Client(turns))

    assert outcome.text is None
    assert "declined" in (outcome.note or "")


def test_an_exhausted_iteration_budget_says_so() -> None:
    """Still reaching for tools when the budget ran out is a partial thought, not a short answer.

    Reported rather than returned bare, because the text that arrived reads like a complete
    answer and is not one.
    """
    turns = [
        _Message(
            [
                {"type": "text", "text": "Checking one more thing."},
                {"type": "tool_use", "id": "toolu_9", "name": "list_sessions", "input": {}},
            ],
            stop_reason="tool_use",
        )
    ]

    outcome = ask("q", transcript=transcript(), tools=[], briefing=BRIEFING, model=MODEL,
                  client=_Client(turns), max_iterations=1)

    assert "stopped early" in (outcome.note or "")
    assert outcome.tools_called == ["list_sessions"]


def test_a_truncated_answer_says_so() -> None:
    turns = [_Message([{"type": "text", "text": "Your tempo"}], stop_reason="max_tokens")]

    outcome = ask("q", transcript=transcript(), tools=[], briefing=BRIEFING, model=MODEL,
                  client=_Client(turns))

    assert (outcome.text or "").endswith("(This answer was cut off before it finished.)")


def test_a_failed_turn_leaves_the_transcript_untouched() -> None:
    """A question recorded with no answer would replay as an unanswered user turn forever."""
    stored = transcript()
    ask("first?", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL,
        client=_Client(answer("One.")))
    before = list(stored.messages)

    outcome = ask("second?", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL,
                  client=_Client(RuntimeError("boom")))

    assert outcome.transcript is None
    assert stored.messages == before


def test_a_transcript_from_another_model_is_refused_rather_than_replayed() -> None:
    """The failure this avoids is a 400 partway through an old conversation.

    Thinking blocks are bound to the model that produced them, so a config change between turns
    has to reseed. Caught here, before the request, with a sentence a golfer can act on.
    """
    stored = new_transcript(model="claude-opus-4-8")
    client = _Client(answer())

    outcome = ask("q", transcript=stored, tools=[], briefing=BRIEFING, model=MODEL, client=client)

    assert outcome.text is None
    assert "start a new conversation" in (outcome.note or "")
    assert client.captured == {}, "the request was sent anyway"


@pytest.mark.parametrize("stop_reason", ["end_turn", "max_tokens"])
def test_an_answer_with_no_text_is_a_note(stop_reason: str) -> None:
    turns = [_Message([{"type": "thinking", "thinking": "", "signature": "s"}], stop_reason)]

    outcome = ask("q", transcript=transcript(), tools=[], briefing=BRIEFING, model=MODEL,
                  client=_Client(turns))

    assert outcome.text is None
    assert "no text" in (outcome.note or "")
