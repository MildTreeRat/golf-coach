"""Follow-up questions about a swing, over the tool runner. [M6, ADR-020]

`coach.py` writes one paragraph from one brief and stops. This holds a conversation: the golfer
asks *why*, or *is that worse than last week*, and the model looks the answer up rather than being
handed a fixed payload.

## What this module does not import

`tools` and `briefing` arrive as **arguments**, never as imports. ADR-008 forbids `feedback`
importing `mcp`, and this is the same injection seam `build_server(sessions_dir, shot_source)`
uses — the caller (a CLI or an API route) owns the wiring. The payoff is that the interesting
assertions here — does the transcript mirror faithfully? does a refusal become a note? — run with
a fake client and fake tools on a base install, with no key and no network.

## Two routes to the tools [M29 P16, ADR-020's 2026-09-30 addendum]

`ask` does not know which route built its tools, and that is what makes the second one a change of
caller rather than of loop.

  - **In process** (ADR-020's Option C): `mcp/runner_tools.build_tools` and
    `mcp/server.instructions`, frozen Python's eleven tools and briefing. The frozen results
    page's chat panel (`api/app.py`) keeps this route until M40 deletes Python `mcp/`.
  - **Over stdio** (Option B): `ask_through_golf_mcp` starts the Rust server, `golf-mcp`, and
    takes everything the model reads from it: the tools from its `tools/list` through the SDK's
    `mcp_tool`, the briefing from its `initialize` answer, and `ONLY_CHECKPOINTS_ARE_JUDGED` from
    its `golf-coach://caveats` resource. **No caveat sentence is copied on this route** (the M29
    plan's D10): the only Python copy is `contracts/caveats.py`, which is frozen and feeds the
    other route. `scripts/ask_swing.py` takes this route.

`mcp_tool` is synchronous and reaches the async client session through `anyio.from_thread.run`, so
the session lives on an event loop and `ask` runs on an anyio worker thread beside it, where the
runner's tool calls can reach back into that loop. One session serves one question, so a question
pays `golf-mcp`'s spawn once and each tool call pays a pipe round trip, which is what every
external client of the server pays too.

## The loop

`client.beta.messages.tool_runner` supplies the agentic loop, so there is no hand-written
`while stop_reason == "tool_use"` here and no chance of getting the tool-result pairing wrong.
What this module owns instead is **mirroring**: the runner keeps its own message list and does not
expose it, so the transcript is rebuilt turn by turn as the loop runs. That is the documented
pattern, and `_mirror` is the whole of it.

Everything is bounded. `max_iterations` stops a confused model spinning through tools; exhausting
it returns a note rather than raising, exactly as `generate_coaching` does for a rate limit. This
is still the least important thing that happens to a swing: it must never be able to cost a golfer
their score, or take down the page their score is rendered on.

## Why the blocks are stored raw

`settings.coaching_model` is `claude-opus-5`, where thinking is on by default and thinking blocks
must be replayed **unchanged** on the same model. So `_mirror` serialises the model's own content
blocks and stores them verbatim — see `contracts/conversation.py`, which is where that rule and
its consequences are written down.

## The call log [M43 P5]

Each message the tool runner yields answers one API request, so every one of them is a line in the
call log (ADR-036 §2, §4), and a question that calls two tools is three lines under one turn id.
`_mirror` hands each message to the question's `_Turn` as it takes it, which is where the lines are
timed and written. A request's latency runs from the loop asking the runner for its next message to
the message arriving, so the tools that run in between are not in it. A line waits until it is
known whether its message was the last: when the next one arrives the loop went on, so it is `ok`;
the last one carries the turn's kind once `ask` has read the answer, which is how the last line of a
loop that ran out of iterations says `exhausted`. A request that raised is a line of its own, with
its error class. The refusals before a request (no extra, no key, a transcript from another model)
write nothing, because nothing was sent. The log is observation only: `llm_log.record` never
raises, so no answer comes back different for it.
"""

from __future__ import annotations

import hashlib
import shutil
import sys
from dataclasses import dataclass, field, replace
from functools import partial
from time import perf_counter
from typing import Any, TextIO
from uuid import uuid4

from golf_coach import llm_log

# The in-process route's copy only: `ask_through_golf_mcp` reads this sentence from `golf-mcp`.
from golf_coach.contracts.caveats import ONLY_CHECKPOINTS_ARE_JUDGED
from golf_coach.contracts.conversation import Transcript

# Shared with `coach.py` on purpose: one seam onto the SDK, one vocabulary for an expected
# failure. A second `_note_for` would drift the first time the SDK renamed an exception, and the
# symptom would be a golfer reading "an error occurred" where the other path says which one. The
# stdio pieces live there too (M29 P17), because its JSON entry reads `golf-mcp` as this route does
# and this module imports that one, never the reverse.
from golf_coach.feedback.coach import (
    BUILD_GOLF_MCP,
    CAVEATS_URI,
    READ_TIMEOUT_SECONDS,
    GolfMcpServer,
    _caveat,
    _innermost,
    _kind_for,
    _note_for,
    _sdk,
    _text_from,
)

#: The vocabulary this module puts in front of a shared failure sentence. `_note_for` defaults to
#: coaching's wording, which is right for the results page and wrong under a question — every
#: other note here already says "no answer", and an SDK error used to be the one that did not.
_NOTE_PREFIX = "no answer: "

#: Room for adaptive thinking, several tool calls and an answer. On Opus 5 `max_tokens` bounds
#: thinking *and* response text together, so this is not "how long may the answer be" — sizing it
#: to the prose would cut answers off mid-sentence after a couple of tool calls.
MAX_TOKENS = 8192

#: Higher than `coach.py`'s `low`, and the difference is the workload. Coaching is summarization
#: over data already in hand; a follow-up is multi-hop tool selection — "was that worse than
#: Tuesday" is `list_sessions` then `compare_sessions`. This is the first lever to reach for if
#: the loop under- or over-calls tools; tune it before adding prompt text.
EFFORT = "medium"

#: The loop bound. Twelve is generous for the deepest real chain (list -> profile -> compare) and
#: still finite. Hitting it is a note, not an exception — an answer that ran out of budget is
#: reported as one rather than being silently truncated into something that reads complete.
MAX_ITERATIONS = 12

#: Prepended to the brief on the first turn, so the model knows the brief is context it was handed
#: rather than something a tool returned — and that it may go looking for anything else.
SEED_PREAMBLE = """\
Here is the swing this conversation is about. It has already been analyzed; these are the stored
results, not something you need to look up. Use the tools when the question needs anything beyond
this swing — another session, this golfer's own history, or shot data not attached here."""

def answering_followups(only_checkpoints_are_judged: str) -> str:
    """This module's own instructions, around the one caveat sentence they quote.

    A function of that sentence rather than a constant over an import, because the two routes take
    it from different places: the in-process route from frozen `contracts/caveats.py`, the stdio
    route from `golf-mcp`'s caveats resource. Both sentences are the same to the character today
    (the M29 plan's P9 finding 5 compared them), so both routes send the same instructions.
    """
    return f"""\
You are a golf coach answering follow-up questions about swings this system has measured. The
data above is what you may reason from; the tools fetch more of the same.

How to answer:

- Answer the question that was asked, in a few plain sentences. No headers, no bullet lists, no
  markdown, no preamble like "Based on the data". Write as if speaking.
- Look things up rather than guessing. If a question needs a session, a shot or a golfer's history
  you were not given, call a tool for it. If a tool reports a miss, say so and say what you would
  need — do not answer from what you would expect to be true.
- Never state a measurement that is not in the data you were given or that a tool returned. If
  something was not measured, say it was not measured. {only_checkpoints_are_judged}.
- When a figure is flagged as uncertain or was withheld, say so in the sentence that uses it. A
  withheld figure is a refusal to report, not an invitation to work it out from what is beside it.
- Address the golfer as "you". Be direct and encouraging; do not pad.
- If you cannot answer from what is measured here, say that plainly. That is a useful answer; an
  invented one is not."""


#: The in-process route's instructions, from frozen Python's copy of the caveat.
ANSWERING_FOLLOWUPS = answering_followups(ONLY_CHECKPOINTS_ARE_JUDGED)


@dataclass(frozen=True)
class AskOutcome:
    """What one question produced. `text is None` means it did not happen, and why.

    Mirrors `CoachingOutcome`, for the same reason: the caller's job is to record an expected
    failure and carry on, not to handle an exception.
    """

    text: str | None = None
    #: The transcript including this turn. `None` when the turn did not happen at all, so a
    #: caller never persists a conversation that recorded a question and no attempt to answer it.
    transcript: Transcript | None = None
    tools_called: list[str] = field(default_factory=list)
    note: str | None = None


def system_prompt(briefing: str, answering: str = ANSWERING_FOLLOWUPS) -> list[dict[str, Any]]:
    """The briefing plus this module's own instructions, as one cacheable system block.

    Identical for every turn of every conversation, which is the whole point: one `cache_control`
    breakpoint here and a session's second question reads the prefix from cache. Worth checking
    against `usage.cache_read_input_tokens` rather than trusting — Opus 5 will not cache a prefix
    under 512 tokens and writes nothing silently.

    `answering` is `answering_followups` over the route's own caveat sentence.
    """
    return [
        {
            "type": "text",
            "text": f"{briefing}\n\n{answering}",
            "cache_control": {"type": "ephemeral"},
        }
    ]


def seed_turn(brief: str, question: str) -> list[dict[str, Any]]:
    """The first user turn: the swing, then the question — as **two** text blocks.

    Two rather than one concatenated string so the question stays separable afterwards. The
    golfer typed the question; they did not type the brief, and a renderer that cannot tell them
    apart opens the conversation with the whole brief in a bubble attributed to *them*. Splitting
    a joined string back apart would mean pattern-matching on the preamble, which breaks the first
    time the wording changes; a block boundary does not.

    `storage.transcript_store.visible_turns` is the renderer that relies on this.
    """
    return [
        {"type": "text", "text": f"{SEED_PREAMBLE}\n\n{brief}"},
        {"type": "text", "text": question},
    ]


def brief_digest(brief: str) -> str:
    """sha256 of the seeded brief, so a resumed conversation can tell when the numbers moved."""
    return hashlib.sha256(brief.encode("utf-8")).hexdigest()


def _plain(value: Any) -> Any:
    """A content block, or a list of them, as plain JSON-safe data.

    `exclude_none=True` is deliberate and narrow: it drops the optional fields the SDK models
    carry as `None` (a text block's `citations`, say), which keeps a replayed message to the
    fields the API expects. It cannot drop anything load-bearing — a thinking block's `signature`
    is always present, and its `thinking` text is `""` rather than `None` when display is omitted.

    Note this is the opposite call from `mcp/runner_tools._json`, where nulls are the refusal and
    must survive. Different direction, different rule: that is a payload *to* the model, this is
    the model's own words on their way to disk.
    """
    if isinstance(value, list):
        return [_plain(item) for item in value]
    if hasattr(value, "model_dump"):
        return value.model_dump(mode="json", exclude_none=True)
    if isinstance(value, dict):
        return {key: _plain(item) for key, item in value.items()}
    return value


def ask(
    question: str,
    *,
    transcript: Transcript,
    tools: list[Any],
    briefing: str,
    model: str,
    brief: str | None = None,
    api_key: str | None = None,
    client: Any | None = None,
    max_iterations: int = MAX_ITERATIONS,
    answering: str = ANSWERING_FOLLOWUPS,
) -> AskOutcome:
    """Ask one question and return the answer, with the transcript grown by this turn.

    Never raises for an expected failure. `client` is the injection seam the tests use — anything
    exposing `beta.messages.tool_runner` works, so the loop and the mirroring are asserted with no
    network and without the `llm` extra installed.

    `transcript` is mutated only on success. A turn that could not happen leaves the stored
    conversation exactly as it was, so a failed ask is never a question recorded with no answer.

    `tools`, `briefing` and `answering` are whichever route's (the module doc's *Two routes*).
    """
    sdk = _sdk()
    client, refused = _sendable(transcript, model, api_key, client)
    if refused is not None:
        return refused

    first_turn = not transcript.messages
    seeded = first_turn and brief is not None
    opening: Any = seed_turn(brief, question) if seeded and brief else question
    messages: list[dict[str, Any]] = [
        *transcript.messages,
        {"role": "user", "content": opening},
    ]

    turn = _Turn(model)
    try:
        runner = client.beta.messages.tool_runner(
            model=model,
            max_tokens=MAX_TOKENS,
            max_iterations=max_iterations,
            # Thinking is on by default on Opus 5; saying so keeps this request honest if the
            # default moves. `budget_tokens` is removed on this model and 400s, as do temperature
            # / top_p / top_k — none of them belong here.
            thinking={"type": "adaptive"},
            output_config={"effort": EFFORT},
            system=system_prompt(briefing, answering),
            tools=tools,
            messages=messages,
        )
        last, tools_called = _mirror(runner, messages, turn)
    except Exception as exc:  # narrowed in `_note_for`; the SDK may be absent entirely
        turn.failed(exc, _kind_for(exc, sdk))
        return AskOutcome(note=_note_for(exc, sdk, prefix=_NOTE_PREFIX))

    if last is None:
        # The runner yielded nothing, so it sent no request and there is no line to write.
        return AskOutcome(note="no answer: the model returned nothing for this question.")

    answered, kind = _answered_from(last, tools_called, max_iterations)
    turn.finished(kind)
    if answered.note is not None:
        return answered

    transcript.messages = messages
    if first_turn and brief is not None:
        transcript.brief_digest = brief_digest(brief)
    return replace(answered, transcript=transcript)


def _answered_from(
    last: Any, tools_called: list[str], max_iterations: int
) -> tuple[AskOutcome, str]:
    """The loop's last message as what the golfer is shown, and the kind its log line records.

    The kind is `ok` for an answer and otherwise names the note in snake_case, the vocabulary
    `clubs/lookup.py::_read` set and `coach.py::_written_from` shares. It is returned beside the
    answer rather than worked out from the note's wording, so rewording a note cannot silently
    move a logged outcome. An answer comes back without its transcript, which `ask` attaches only
    to an answer with no note.
    """
    stop_reason = getattr(last, "stop_reason", None)
    if stop_reason == "refusal":
        return AskOutcome(note="no answer: the model declined to answer this question."), "refusal"

    text = _text_from(last)
    if stop_reason == "tool_use":
        # The loop hit `max_iterations` with the model still reaching for tools. Say so: whatever
        # text arrived is a partial thought, not an answer that happens to be short.
        note = (
            "the answer stopped early — it was still looking things up after "
            f"{max_iterations} steps. Ask a narrower question, or ask again."
        )
        return AskOutcome(text=text or None, tools_called=tools_called, note=note), "exhausted"
    if not text:
        no_text = "no answer: the model returned no text for this question."
        return AskOutcome(note=no_text), "no_text"
    if stop_reason == "max_tokens":
        # Served with its admission, as `coach.py`'s `truncated` is, where the lookup's serves
        # nothing. The line's `route` tells the two apart.
        text += "\n\n(This answer was cut off before it finished.)"
        return AskOutcome(text=text, tools_called=tools_called), "truncated"
    return AskOutcome(text=text, tools_called=tools_called), "ok"


def _sendable(
    transcript: Transcript, model: str, api_key: str | None, client: Any | None
) -> tuple[Any, AskOutcome | None]:
    """The client a question goes out on, or the note saying why it cannot go out at all.

    One helper for both routes, so that the stdio route refuses before it starts anything: a
    question with no extra, no key or a transcript bound to another model is not worth spawning
    `golf-mcp` for, and the two routes say so in the same words.
    """
    if client is None:
        sdk = _sdk()
        # Extra before key, for the reason spelled out in `coach.py`'s matching guard: only one of
        # the two can be reported, and a missing package is the one that makes the other moot.
        if sdk is None:
            return None, AskOutcome(
                note=(
                    "no answer: the `llm` extra is not installed (pip install -e '.[llm]'). "
                    "The stored analysis is unaffected."
                )
            )
        if not api_key:
            return None, AskOutcome(
                note=(
                    "no answer: no Anthropic API key is configured, so the question was not sent "
                    "(set GOLF_ANTHROPIC_API_KEY). The stored analysis is unaffected."
                )
            )
        client = sdk.Anthropic(api_key=api_key)

    if not transcript.replayable_by(model):
        return None, AskOutcome(
            note=(
                f"no answer: this conversation was recorded against {transcript.model!r} and the "
                f"configured model is now {model!r}. A model's own reasoning cannot be replayed "
                "into a different one, so start a new conversation about this swing."
            )
        )
    return client, None


def ask_through_golf_mcp(
    question: str,
    *,
    transcript: Transcript,
    server: GolfMcpServer,
    model: str,
    brief: str | None = None,
    api_key: str | None = None,
    client: Any | None = None,
    max_iterations: int = MAX_ITERATIONS,
    errlog: TextIO | None = None,
) -> AskOutcome:
    """`ask`, with the tools, the briefing and the caveat all taken from `golf-mcp` over stdio.

    ADR-020's Option B. Never raises for an expected failure: a server that is not built, will not
    start, or answers the handshake with something unusable is a note, as a rate limit is, and
    the transcript is then untouched. `errlog` is where the server's stderr goes (its one-line
    banner naming the stores, and any warning), `sys.stderr` when not given.
    """
    client, refused = _sendable(transcript, model, api_key, client)
    if refused is not None:
        return refused

    binary = shutil.which(str(server.binary))
    if binary is None:
        return AskOutcome(
            note=f"no answer: golf-mcp is not built at {server.binary} (run `{BUILD_GOLF_MCP}`)."
        )

    try:
        import anyio
        import anyio.to_thread
        from anthropic.lib.tools.mcp import mcp_tool
        from mcp import ClientSession, StdioServerParameters
        from mcp.client.stdio import stdio_client
    except ImportError as exc:
        return AskOutcome(
            note=(
                f"no answer: the MCP client is not installed ({exc}). It is part of the `llm` "
                "extra (pip install -e '.[llm]'). The stored analysis is unaffected."
            )
        )

    # A list rather than a return value, so that an answer survives a failure after it: once `ask`
    # has answered, the transcript holds the turn, and a server that then fails to shut down
    # cleanly has not made the answer any less given.
    answered: list[AskOutcome] = []

    async def converse() -> None:
        parameters = StdioServerParameters(command=binary, args=server.args())
        async with (
            stdio_client(parameters, errlog=errlog or sys.stderr) as (read, write),
            ClientSession(read, write, read_timeout_seconds=READ_TIMEOUT_SECONDS) as session,
        ):
            hello = await session.initialize()
            if not hello.instructions:
                raise ValueError("golf-mcp's initialize answer carries no briefing")
            listed = await session.list_tools()
            caveats = await session.read_resource(CAVEATS_URI)
            caveat = _caveat(caveats, "ONLY_CHECKPOINTS_ARE_JUDGED")
            tools = [mcp_tool(tool, session) for tool in listed.tools]
            outcome = await anyio.to_thread.run_sync(
                partial(
                    ask,
                    question,
                    transcript=transcript,
                    tools=tools,
                    briefing=hello.instructions,
                    model=model,
                    brief=brief,
                    client=client,
                    max_iterations=max_iterations,
                    answering=answering_followups(caveat),
                )
            )
            answered.append(outcome)

    try:
        anyio.run(converse)
    except Exception as exc:  # the spawn, the pipe and the handshake; `ask` itself never raises
        if answered:
            return answered[0]
        return AskOutcome(note=f"no answer: golf-mcp could not be used ({_innermost(exc)}).")
    return answered[0]


def _mirror(runner: Any, messages: list[dict[str, Any]], turn: _Turn) -> tuple[Any, list[str]]:
    """Run the loop, appending every turn to `messages` and handing every message to `turn`.

    The runner keeps its own copy of the history and does not expose it, so the only way to end up
    with a replayable transcript is to rebuild it here. `generate_tool_call_response` is cached by
    the runner — calling it does not run the tools a second time — and returns `None` on a turn
    that called no tools, which is how the last turn is recognised.

    The runner sends a request each time the loop asks it for a message, so `turn.asking` marks
    each ask and `turn.received` each answer: one log line per message (ADR-036 §4). The next ask
    is marked after the tools have answered, so a request's latency holds no tool's time.

    Returns the final message and the tools called along the way, in order.
    """
    last: Any = None
    tools_called: list[str] = []

    turn.asking()
    for message in runner:
        turn.received(message)
        last = message
        messages.append({"role": "assistant", "content": _plain(message.content)})
        tools_called += [
            block.name for block in message.content if getattr(block, "type", None) == "tool_use"
        ]
        response = runner.generate_tool_call_response()
        if response is not None:
            messages.append(_plain(response))
        turn.asking()

    return last, tools_called


class _Turn:
    """One question's lines in the call log, one per request the tool runner sends. [M43 P5]

    `_mirror` marks when the loop asks the runner for a message (`asking`), which is when the
    runner sends its request, and when the message arrives (`received`). A line is held until it
    is known whether its message was the last. When the next one arrives the loop went on, so the
    held line is `ok`. When the loop ends, `ask` reads the answer and `finished` writes the last
    line with the turn's kind. When the loop raises, `failed` writes what is held and then a line
    for the request that raised.

    Iterations count from 1, as the runner's `max_iterations` does, so a loop that ran out of
    iterations ends on the line whose `iteration` is that bound. Nothing here raises, because
    `llm_log.record` never does: the log cannot change an answer (ADR-036 §4).
    """

    def __init__(self, model: str) -> None:
        #: One per question, shared by every request the question's loop sends.
        self.turn_id = uuid4().hex
        self._model = model
        self._iteration = 0
        self._asked_at = 0.0
        #: True from an ask until its message arrives: a request is out and unanswered.
        self._in_flight = False
        #: The newest message, its latency and its iteration, until its outcome is known.
        self._held: tuple[Any, float, int] | None = None

    def asking(self) -> None:
        """The loop is asking the runner for its next message, so the next request goes out."""
        self._asked_at = perf_counter()
        self._in_flight = True

    def received(self, message: Any) -> None:
        """A message arrived. The one held before it was not the last, so the loop went on."""
        latency_s = perf_counter() - self._asked_at
        self._in_flight = False
        self._release("ok")
        self._iteration += 1
        self._held = (message, latency_s, self._iteration)

    def failed(self, exc: Exception, kind: str) -> None:
        """The loop raised `exc`, which the golfer is told about as a note of this kind.

        A request that was out when it raised is a line of its own, with no response. One that had
        been answered was not the request that raised, so its own line carries the error. With
        neither, no request was sent (the runner could not be built), and nothing is written.
        """
        if self._in_flight:
            latency_s = perf_counter() - self._asked_at
            self._in_flight = False
            self._release("ok")
            self._iteration += 1
            self._write(None, latency_s, self._iteration, kind, exc)
        else:
            self._release(kind, exc)

    def finished(self, kind: str) -> None:
        """`ask` has read the last message as this kind, so the last line can be written."""
        self._release(kind)

    def _release(self, outcome: str, error: Exception | None = None) -> None:
        if self._held is None:
            return
        message, latency_s, iteration = self._held
        self._held = None
        self._write(message, latency_s, iteration, outcome, error)

    def _write(
        self,
        response: Any,
        latency_s: float,
        iteration: int,
        outcome: str,
        error: Exception | None,
    ) -> None:
        llm_log.record(
            route="conversation",
            requested_model=self._model,
            latency_s=latency_s,
            response=response,
            outcome=outcome,
            error=error,
            turn_id=self.turn_id,
            iteration=iteration,
        )
