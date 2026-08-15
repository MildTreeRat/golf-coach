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
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass, field
from typing import Any

from golf_coach.contracts.caveats import ONLY_CHECKPOINTS_ARE_JUDGED
from golf_coach.contracts.conversation import Transcript

# Shared with `coach.py` on purpose: one seam onto the SDK, one vocabulary for an expected
# failure. A second `_note_for` would drift the first time the SDK renamed an exception, and the
# symptom would be a golfer reading "an error occurred" where the other path says which one.
from golf_coach.feedback.coach import _note_for, _sdk, _text_from

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

ANSWERING_FOLLOWUPS = f"""\
You are a golf coach answering follow-up questions about swings this system has measured. The
data above is what you may reason from; the tools fetch more of the same.

How to answer:

- Answer the question that was asked, in a few plain sentences. No headers, no bullet lists, no
  markdown, no preamble like "Based on the data". Write as if speaking.
- Look things up rather than guessing. If a question needs a session, a shot or a golfer's history
  you were not given, call a tool for it. If a tool reports a miss, say so and say what you would
  need — do not answer from what you would expect to be true.
- Never state a measurement that is not in the data you were given or that a tool returned. If
  something was not measured, say it was not measured. {ONLY_CHECKPOINTS_ARE_JUDGED}.
- When a figure is flagged as uncertain or was withheld, say so in the sentence that uses it. A
  withheld figure is a refusal to report, not an invitation to work it out from what is beside it.
- Address the golfer as "you". Be direct and encouraging; do not pad.
- If you cannot answer from what is measured here, say that plainly. That is a useful answer; an
  invented one is not."""


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


def system_prompt(briefing: str) -> list[dict[str, Any]]:
    """The briefing plus this module's own instructions, as one cacheable system block.

    Identical for every turn of every conversation, which is the whole point: one `cache_control`
    breakpoint here and a session's second question reads the prefix from cache. Worth checking
    against `usage.cache_read_input_tokens` rather than trusting — Opus 5 will not cache a prefix
    under 512 tokens and writes nothing silently.
    """
    return [
        {
            "type": "text",
            "text": f"{briefing}\n\n{ANSWERING_FOLLOWUPS}",
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
) -> AskOutcome:
    """Ask one question and return the answer, with the transcript grown by this turn.

    Never raises for an expected failure. `client` is the injection seam the tests use — anything
    exposing `beta.messages.tool_runner` works, so the loop and the mirroring are asserted with no
    network and without the `llm` extra installed.

    `transcript` is mutated only on success. A turn that could not happen leaves the stored
    conversation exactly as it was, so a failed ask is never a question recorded with no answer.
    """
    sdk = _sdk()
    if client is None:
        if not api_key:
            return AskOutcome(
                note=(
                    "no answer: no Anthropic API key is configured, so the question was not sent "
                    "(set GOLF_ANTHROPIC_API_KEY). The stored analysis is unaffected."
                )
            )
        if sdk is None:
            return AskOutcome(
                note=(
                    "no answer: the `llm` extra is not installed (pip install -e '.[llm]'). "
                    "The stored analysis is unaffected."
                )
            )
        client = sdk.Anthropic(api_key=api_key)

    if not transcript.replayable_by(model):
        return AskOutcome(
            note=(
                f"no answer: this conversation was recorded against {transcript.model!r} and the "
                f"configured model is now {model!r}. A model's own reasoning cannot be replayed "
                "into a different one, so start a new conversation about this swing."
            )
        )

    first_turn = not transcript.messages
    seeded = first_turn and brief is not None
    opening: Any = seed_turn(brief, question) if seeded and brief else question
    messages: list[dict[str, Any]] = [
        *transcript.messages,
        {"role": "user", "content": opening},
    ]

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
            system=system_prompt(briefing),
            tools=tools,
            messages=messages,
        )
        last, tools_called = _mirror(runner, messages)
    except Exception as exc:  # narrowed in `_note_for`; the SDK may be absent entirely
        return AskOutcome(note=_note_for(exc, sdk))

    if last is None:
        return AskOutcome(note="no answer: the model returned nothing for this question.")

    stop_reason = getattr(last, "stop_reason", None)
    if stop_reason == "refusal":
        return AskOutcome(note="no answer: the model declined to answer this question.")

    text = _text_from(last)
    if stop_reason == "tool_use":
        # The loop hit `max_iterations` with the model still reaching for tools. Say so: whatever
        # text arrived is a partial thought, not an answer that happens to be short.
        note = (
            "the answer stopped early — it was still looking things up after "
            f"{max_iterations} steps. Ask a narrower question, or ask again."
        )
        return AskOutcome(text=text or None, tools_called=tools_called, note=note)
    if not text:
        return AskOutcome(note="no answer: the model returned no text for this question.")
    if stop_reason == "max_tokens":
        text += "\n\n(This answer was cut off before it finished.)"

    transcript.messages = messages
    if first_turn and brief is not None:
        transcript.brief_digest = brief_digest(brief)
    return AskOutcome(text=text, transcript=transcript, tools_called=tools_called)


def _mirror(runner: Any, messages: list[dict[str, Any]]) -> tuple[Any, list[str]]:
    """Run the loop, appending every turn to `messages` as it goes.

    The runner keeps its own copy of the history and does not expose it, so the only way to end up
    with a replayable transcript is to rebuild it here. `generate_tool_call_response` is cached by
    the runner — calling it does not run the tools a second time — and returns `None` on a turn
    that called no tools, which is how the last turn is recognised.

    Returns the final message and the tools called along the way, in order.
    """
    last: Any = None
    tools_called: list[str] = []

    for message in runner:
        last = message
        messages.append({"role": "assistant", "content": _plain(message.content)})
        tools_called += [
            block.name for block in message.content if getattr(block, "type", None) == "tool_use"
        ]
        response = runner.generate_tool_call_response()
        if response is not None:
            messages.append(_plain(response))

    return last, tools_called
