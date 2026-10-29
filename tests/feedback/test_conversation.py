"""The conversation loop, driven by a fake client. [ADR-020]

Runs on a base install: no key, no network, no `llm` extra. That is the point of `ask()` taking
its client, tools and briefing as arguments — everything worth asserting here is reachable without
any of them.

The load-bearing test is `test_a_tool_calling_turn_is_mirrored_verbatim`. A transcript that loses
a thinking block or splits a `tool_use`/`tool_result` pair cannot be replayed, and the way that
failure presents is a 400 on the *third* turn of a conversation that worked twice — long after the
code that caused it.

The stdio route's tests (M29 P16) are the exception to "no extra": they start the real `golf-mcp`
and need the `llm` extra's `mcp` client. Each is skipped by name when either is missing, and the
build that fixes it is in the reason.
"""

from __future__ import annotations

import importlib.util
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import pytest

from golf_coach.contracts.conversation import Transcript
from golf_coach.feedback.conversation import (
    ANSWERING_FOLLOWUPS,
    BUILD_GOLF_MCP,
    MAX_ITERATIONS,
    SEED_PREAMBLE,
    AskOutcome,
    GolfMcpServer,
    answering_followups,
    ask,
    ask_through_golf_mcp,
    brief_digest,
    system_prompt,
)
from golf_coach.storage.transcript_store import new_transcript, visible_turns

MODEL = "claude-opus-5"
BRIEFING = "Swing analysis and launch-monitor data from a home golf simulator."
REPO_ROOT = Path(__file__).resolve().parents[2]


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


class _CallingRunner(_Runner):
    """`_Runner`, except that it runs the tools it was handed, on the thread that iterates it.

    That thread is the point. On the stdio route each tool is the SDK's `mcp_tool`, which reaches
    the client session on the event loop through `anyio.from_thread.run`, and that works only
    from an anyio worker thread. The real runner calls its tools on its own thread as well
    (`_generate_tool_call_response` in the SDK's `_beta_runner.py`).
    """

    def __init__(self, turns: list[_Message], tools: list[Any]) -> None:
        super().__init__(turns)
        self._tools = {tool.name: tool for tool in tools}
        self._answered: dict[int, dict[str, Any]] = {}

    def generate_tool_call_response(self) -> dict[str, Any] | None:
        assert self._current is not None
        calls = [block for block in self._current.content if block.type == "tool_use"]
        if not calls:
            return None
        turn = id(self._current)
        if turn not in self._answered:
            self._answered[turn] = {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": block._data["id"],
                        "content": self._tools[block.name].call(block._data["input"]),
                    }
                    for block in calls
                ],
            }
        return self._answered[turn]


class _CallingClient(_Client):
    def tool_runner(self, **kwargs: Any) -> _Runner:
        super().tool_runner(**kwargs)
        assert isinstance(self._turns, list)
        return _CallingRunner(self._turns, kwargs["tools"])


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
    """Needs the SDK present, because the extra is now checked first and would answer instead."""
    pytest.importorskip("anthropic", reason="the missing-key note is only reached with the SDK")

    outcome = ask("q", transcript=transcript(), tools=[], briefing=BRIEFING, model=MODEL)

    assert isinstance(outcome, AskOutcome)
    assert outcome.text is None
    assert outcome.transcript is None
    assert "no Anthropic API key" in (outcome.note or "")


def test_a_missing_extra_is_not_reported_as_a_missing_key(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """Same ordering rule as `coach.py`: pip is the fix, so pip is what the note has to name."""
    monkeypatch.setattr("golf_coach.feedback.conversation._sdk", lambda: None)

    outcome = ask("q", transcript=transcript(), tools=[], briefing=BRIEFING, model=MODEL)

    assert "llm" in (outcome.note or "")
    assert "API key" not in (outcome.note or "")


def test_an_sdk_error_speaks_this_panels_vocabulary_not_coachings() -> None:
    """`_note_for` is shared with `coach.py`, and its wording defaults to the results page.

    Every other note in this module says "no answer". An SDK error was the one that did not,
    because the prefix was hardcoded — so a rate-limited question replied "no written coaching:"
    directly underneath the question the golfer had just typed.
    """
    anthropic = pytest.importorskip("anthropic")
    exc = anthropic.RateLimitError.__new__(anthropic.RateLimitError)
    Exception.__init__(exc, "boom")

    outcome = ask("q", transcript=transcript(), tools=[], briefing=BRIEFING, model=MODEL,
                  client=_Client(exc))

    assert (outcome.note or "").startswith("no answer: ")
    assert "no written coaching" not in (outcome.note or "")


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


# --------------------------------------------------------------------------- the stdio route


# `golf_mcp`, the built server or a skip naming the build, is `conftest.py`'s, shared with
# `test_coach.py`'s JSON-entry tests (M29 P17).


def _stores(root: Path, *, registry: bool) -> dict[str, Path | None]:
    """Empty stores: every answer the tests need is one a store with nothing in it gives."""
    names = ("sessions", "shots", "golfers") if registry else ("sessions", "shots")
    for name in names:
        (root / name).mkdir()
    return {
        "sessions_dir": root / "sessions",
        "shots_dir": root / "shots",
        "golfers_dir": root / "golfers" if registry else None,
    }


def _server(binary: Path, stores: dict[str, Path | None]) -> GolfMcpServer:
    sessions_dir, shots_dir = stores["sessions_dir"], stores["shots_dir"]
    assert sessions_dir is not None and shots_dir is not None
    return GolfMcpServer(
        binary=binary,
        sessions_dir=sessions_dir,
        shots_dir=shots_dir,
        golfers_dir=stores["golfers_dir"],
    )


def _in_process_tool_names(stores: dict[str, Path | None]) -> list[str]:
    """What frozen Python's route offers over the same stores, for the stdio route to match."""
    from golf_coach.launch_monitor.composite import CompositeShotDataSource
    from golf_coach.mcp.runner_tools import build_tools

    sessions_dir = stores["sessions_dir"]
    assert sessions_dir is not None
    tools = build_tools(
        sessions_dir, CompositeShotDataSource([]), golfers_dir=stores["golfers_dir"]
    )
    return [tool.name for tool in tools]


def test_the_stdio_route_asks_with_golf_mcps_tools_briefing_and_caveat(
    golf_mcp: Path, tmp_path: Path
) -> None:
    """The route end to end over the real server, with only the model faked.

    The tool call is made, not canned: `get_swing` crosses from the runner's thread into the
    event loop and over the pipe, and the miss it answers is the server's own sentence. The
    briefing is Rust's (only Rust's says a misread photo is worth retaking, the M29 plan's C13),
    and the instructions around the server's `ONLY_CHECKPOINTS_ARE_JUDGED` come out exactly as
    the in-process route's, which is what keeping no copy of that sentence has to preserve.
    """
    stores = _stores(tmp_path, registry=True)
    turns = [
        _Message(
            [
                {
                    "type": "tool_use",
                    "id": "toolu_01",
                    "name": "get_swing",
                    "input": {"session_id": "2026-08-10", "swing_id": "2"},
                }
            ],
            stop_reason="tool_use",
        ),
        _Message([{"type": "text", "text": "That swing is not on file."}]),
    ]
    client = _CallingClient(turns)
    stored = transcript()

    outcome = ask_through_golf_mcp(
        "why is my tempo quick?",
        transcript=stored,
        server=_server(golf_mcp, stores),
        model=MODEL,
        client=client,
    )

    assert outcome.text == "That swing is not on file."
    assert outcome.tools_called == ["get_swing"]
    names = [tool.name for tool in client.captured["tools"]]
    assert names == [*_in_process_tool_names(stores), "get_strike_profile"]
    system = client.captured["system"][0]["text"]
    assert system.endswith(f"\n\n{ANSWERING_FOLLOWUPS}")
    # Whitespace collapsed, because the server fills its briefing to a width and may break here.
    assert "`misread` is a photo of the screen worth retaking" in " ".join(system.split())
    result = stored.messages[2]["content"][0]
    assert result["tool_use_id"] == "toolu_01"
    assert json.loads(result["content"][0]["text"]) == {
        "found": False,
        "message": (
            "No swing '2' in session '2026-08-10'. "
            "Call list_sessions to see which sessions and swing ids exist."
        ),
    }


def test_the_stdio_route_without_a_registry_offers_what_the_in_process_route_does(
    golf_mcp: Path, tmp_path: Path
) -> None:
    """`--no-registry` drops every per-golfer tool, `get_strike_profile` with the other five."""
    stores = _stores(tmp_path, registry=False)
    client = _CallingClient(answer())

    outcome = ask_through_golf_mcp(
        "q",
        transcript=transcript(),
        server=_server(golf_mcp, stores),
        model=MODEL,
        client=client,
    )

    assert outcome.text is not None
    assert [tool.name for tool in client.captured["tools"]] == _in_process_tool_names(stores)


@dataclass(frozen=True)
class _Refused(GolfMcpServer):
    """A server started with a flag it refuses, so it exits before the handshake."""

    def args(self) -> list[str]:
        return ["--not-a-flag"]


def test_a_server_that_exits_before_the_handshake_is_a_note(
    golf_mcp: Path, tmp_path: Path
) -> None:
    stored = transcript()
    client = _CallingClient(answer())

    with (tmp_path / "golf-mcp.log").open("w", encoding="utf-8") as errlog:
        outcome = ask_through_golf_mcp(
            "q",
            transcript=stored,
            server=_Refused(binary=golf_mcp, sessions_dir=tmp_path, shots_dir=tmp_path),
            model=MODEL,
            client=client,
            errlog=errlog,
        )

    assert (outcome.note or "").startswith("no answer: golf-mcp could not be used (")
    assert outcome.transcript is None
    assert stored.messages == []
    assert client.captured == {}, "the question was sent with no tools behind it"
    assert "unknown argument: --not-a-flag" in (tmp_path / "golf-mcp.log").read_text("utf-8")


def test_a_golf_mcp_that_is_not_built_is_a_note_naming_the_build(tmp_path: Path) -> None:
    client = _Client(answer())

    outcome = ask_through_golf_mcp(
        "q",
        transcript=transcript(),
        server=GolfMcpServer(
            binary=tmp_path / "golf-mcp", sessions_dir=tmp_path, shots_dir=tmp_path
        ),
        model=MODEL,
        client=client,
    )

    assert outcome.text is None
    assert BUILD_GOLF_MCP in (outcome.note or "")
    assert client.captured == {}


def test_a_missing_key_is_refused_before_golf_mcp_is_looked_for(tmp_path: Path) -> None:
    """Nothing is started for a question that could not be sent, and the note says the real why."""
    pytest.importorskip("anthropic", reason="the missing-key note is only reached with the SDK")

    outcome = ask_through_golf_mcp(
        "q",
        transcript=transcript(),
        server=GolfMcpServer(
            binary=tmp_path / "golf-mcp", sessions_dir=tmp_path, shots_dir=tmp_path
        ),
        model=MODEL,
    )

    assert "no Anthropic API key" in (outcome.note or "")
    assert BUILD_GOLF_MCP not in (outcome.note or "")


def test_the_instructions_quote_the_routes_own_caveat() -> None:
    blocks = system_prompt(BRIEFING, answering_followups("Only the measured is judged"))

    assert "say it was not measured. Only the measured is judged." in blocks[0]["text"]


def test_ask_swing_exits_2_naming_the_build_when_golf_mcp_is_missing(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """Before reading anything: the commonest setup slip answers with the command that fixes it."""
    script = _ask_swing_script()

    code = script.main(
        [
            "2026-08-10/2",
            "why?",
            "--golf-mcp",
            str(tmp_path / "golf-mcp"),
            "--conversations-dir",
            str(tmp_path / "conversations"),
        ]
    )

    assert code == 2
    assert BUILD_GOLF_MCP in capsys.readouterr().err
    assert not (tmp_path / "conversations").exists()


def _ask_swing_script() -> Any:
    spec = importlib.util.spec_from_file_location(
        "ask_swing", REPO_ROOT / "scripts" / "ask_swing.py"
    )
    assert spec is not None and spec.loader is not None
    script = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(script)
    return script


def test_ask_swing_seeds_from_the_brief_the_paragraph_is_written_from(
    golf_mcp: Path, tmp_path: Path, store_swing: Any
) -> None:
    """M29 P17: one rendering of the swing opens both the conversation and the coaching paragraph.

    `coach.brief_through_golf_mcp` is what `python -m golf_coach.feedback.coach` writes from, and
    a swing with no analysis on file is the old sentence, naming the command that makes one.
    """
    from golf_coach.contracts.swing import SwingBundleResult, SwingResult
    from golf_coach.feedback.coach import brief_through_golf_mcp

    stores = _stores(tmp_path, registry=False)
    sessions_dir = stores["sessions_dir"]
    assert sessions_dir is not None
    swing = SwingResult(swing_id="2", session_id="2026-08-10", overall_score=90.0)
    store_swing(
        sessions_dir, SwingBundleResult(swing_id="2", session_id="2026-08-10", swing=swing)
    )
    server = _server(golf_mcp, stores)
    script = _ask_swing_script()

    loaded = script._load_brief(server, "2026-08-10/2")

    assert loaded == (brief_through_golf_mcp(server, "2026-08-10", "2").text, "2026-08-10", "2")
    assert loaded[0].startswith("SWING 2 (session 2026-08-10)\n\noverall_score: 90 out of 100")
    missing = script._load_brief(server, "2026-08-10/9")
    assert missing.startswith("no analyzed swing at '2026-08-10/9' under ")
    assert missing.endswith("Run golf-lab analyze on it first.")
