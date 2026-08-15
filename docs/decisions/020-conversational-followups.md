# ADR-020: Conversational follow-ups — a transcript store, and the tool runner over `query.py`

## Status
Accepted

## Date
2026-08-15

## Context

M6's coaching call is **one shot**. `feedback/coach.py` renders a brief, asks Claude for a
paragraph, stamps it with `CoachingProvenance`, and stops. That was the right shape for what it
does — the paragraph is the last thing that happens to a swing and the least important, so it
must never be able to cost a golfer their score.

But a golfer who reads *"your tempo is quick"* has nowhere to ask **why**, or **is that worse than
last week**, or **what should I feel**. Answering those needs three things the one-shot path does
not have:

1. **Data the brief does not contain.** "Worse than last week" is a different session; "how do I
   usually miss" is the career reader. The brief is one swing by construction — it excludes
   keypoints and phases deliberately, and it knows nothing about any other swing.
2. **The ability to look things up mid-answer**, rather than being handed a fixed payload.
3. **Memory across turns.** The second question is almost always about the first answer.

The lookup half already exists and is proven. `mcp/query.py` and `mcp/career.py` hold eight
read-only functions, and ADR-006's tool surface was driven over a real stdio client on 2026-08-14:
`initialize` negotiated, all eight tools advertised with schemas, `call_tool` served live queries
including the not-found path. What does not exist is anywhere to keep a conversation.

That is the whole of this ADR: **the missing piece is a transcript store**, and the decision is
where the loop that fills it lives.

### Why a stored transcript is not optional

The Messages API is stateless — every turn resends the whole history. On `claude-opus-5`
(`settings.coaching_model`) two properties make "just rebuild the context each time" wrong rather
than merely wasteful:

- **Thinking is on by default**, and thinking blocks must be replayed **unchanged** on the same
  model. A transcript that stores extracted text and reconstructs the rest cannot replay.
- **Tool use spans turns.** A `tool_use` block and its `tool_result` are a matched pair; dropping
  or re-deriving either breaks the turn.

So the transcript has to hold the model's own content blocks verbatim, not a rendering of them.

## Options Considered

### Option A: stateless re-ask — rebuild the brief, prepend a summary of the last answer
- **Pros**: no new storage, no new shapes. The existing `build_brief` does all the work.
- **Cons**: cannot replay thinking blocks, so every turn pays a cold start and the model loses its
  own reasoning. Worse, "summarize the last answer" is a lossy re-rendering of exactly the prose
  this repo is careful about — the caveats that survived into turn one get summarized away by turn
  three. And it still cannot answer a cross-session question, because the brief is one swing.

### Option B: the SDK tool runner driving the **stdio MCP server** as a subprocess
- **Pros**: one tool implementation, already proven over the wire. The Python SDK even ships
  `anthropic.lib.tools.mcp` helpers to convert MCP tools for the runner.
- **Cons**: a process spawn and a wire round trip per tool call, to reach functions already
  importable in-process. That round trip exists for **external** clients — Claude Desktop, Claude
  Code — and re-paying it for an in-app call buys nothing. It also makes the `mcp` SDK a runtime
  dependency of the web server, and makes every follow-up answer depend on a subprocess launching
  correctly.

### Option C: the SDK tool runner over `mcp/query.py`'s functions directly, plus a transcript store (chosen)
- **Pros**: in-process calls, no subprocess, no wire. The runner (`client.beta.messages.tool_runner`)
  supplies the loop, so there is no hand-written `while stop_reason == "tool_use"`. `max_iterations`
  bounds it. The same query functions serve both adapters, so the MCP server and the in-app
  conversation can never disagree about what a swing scored.
- **Cons**: a second set of tool *definitions* over the same functions, which is a drift risk — see
  the tool-description seam below, which is the mitigation and the reason this ADR is not shorter.
  The runner is a beta SDK surface.

### Option D: Managed Agents — let Anthropic hold the session state
- **Pros**: no transcript store to build; server-side compaction and memory come free.
- **Cons**: moves this repo's data to a hosted session and makes a home-lab feature depend on a
  network round trip for its *state*, not just its inference. The whole system is local-first
  (ADR-016), the analysis core is stdlib-only, and the artifacts on disk are the record. Storing a
  golfer's conversation history off-machine to avoid writing one JSON reader is the wrong trade.

## Decision

**Option C.** Concretely:

- **`contracts/conversation.py`** holds `Transcript`: the conversation id, the `model` it was
  recorded against, the swing it was seeded from, a `brief_digest`, and `messages` — a list of
  **verbatim** content-block dicts, thinking blocks included and unmodified.
- **`storage/transcript_store.py`** reads and writes them under `settings.conversations_dir`
  (`data/processed/conversations/`), one JSON file per conversation, with the tolerant-read and
  atomic-write shape `api/state.py` established.
- **`mcp/runner_tools.py`** builds the eight tools for the runner, beside `mcp/server.py` and over
  the same `query.py` / `career.py` functions. ADR-008's 2026-08-13 addendum already records `mcp`
  as a second imperative shell; this is a second adapter inside it, not a new edge.
- **`feedback/conversation.py`** owns the loop and the `anthropic` seam. It takes `tools` and
  `briefing` as **arguments** — the same injection seam `build_server` uses — so it never imports
  `mcp`, and its interesting assertions run with a fake client on a base install.
- **`scripts/ask_swing.py`** and two routes on `api/app.py` are the two entry points.

### The tool descriptions move to `contracts/`, and that is the load-bearing part

`mcp/server.py` carries eight hand-written tool descriptions — the prose telling a model *when* to
call each tool and how to read the result honestly (`get_shot_trends`'s "`ready` is false until…",
`get_golfer_profile`'s "report those refusals as refusals"). The runner needs the identical text,
and ADR-008 forbids `feedback` importing `mcp`.

This is the same problem `contracts/caveats.py` was created to solve, so it gets the same answer:
**`contracts/tool_descriptions.py` is the single source, and both adapters read it.** Prose that
exists twice is prose that goes stale in one copy — this repo has been bitten by that three times,
and the last time it told every coaching call a false fact for a milestone.

The mechanism is worth recording because it is not obvious. `@beta_tool` derives a tool's
description from the function's `__doc__` **at decoration time**, so:

```python
def _get_swing(session_id: str, swing_id: str) -> SwingView | NotFound: ...
_get_swing.__doc__ = f"{GET_SWING}\n\nArgs:\n    session_id: ...\n    swing_id: ...\n"
get_swing_tool = beta_tool(_get_swing)          # description carries in full
```

Decorating first and assigning `.description` afterwards **silently produces an empty description**
(verified against `anthropic` 0.121.0, which is why it is written down). The `Args:` section still
becomes per-parameter schema descriptions, so nothing is lost by routing the prose through
`__doc__`.

### Model parameters

- `output_config={"effort": "medium"}`, **not** `coach.py`'s `EFFORT = "low"`. Coaching is
  summarization over data already in hand; a follow-up is multi-hop tool selection
  (`list_sessions` → `compare_sessions`). Effort is the first lever if the loop under- or
  over-calls tools — reach for it before adding prompt text.
- `max_iterations` on the runner is the loop bound. Exhausting it returns a note, not an exception.
- The briefing carries `cache_control`; it is identical across every turn and every conversation.

## Consequences

- **A conversation can reach data the coaching call cannot**, including the career tools. That is
  the point, and it is also the main new risk: those tools withhold any claim the sample size
  cannot support, and a withheld figure sitting next to per-session counts is exactly what an eager
  reader reconstructs. `READING_A_PERSONAL_HISTORY` is in the briefing for this reason, and
  **"does it refuse the cross-session question at n=2" is the acceptance test for this feature** —
  no assertion can cover it, so it is checked by hand against real swings.
- **`ask()` never raises for an expected failure.** No key, no `llm` extra, a rate limit, a refusal,
  an exhausted iteration budget each return a note. Same discipline as `generate_coaching`, and
  `_note_for` is reused rather than reimplemented.
- **The transcript is model-bound.** Thinking blocks recorded under one model cannot be replayed
  into another, so `Transcript.model` is recorded and a resume under a changed `coaching_model`
  reseeds rather than replaying. Without this the failure is a 400 on turn three of an old
  conversation, long after the config changed.
- **A stored transcript can outlive the numbers it discusses.** `brief_digest` is the same trick
  `CoachingProvenance.input_digest` plays: a conversation seeded from a swing that has since been
  re-analyzed can say so instead of quietly discussing stale figures.
- **The web route blocks.** The runner loop is multi-second and synchronous, so the POST handler is
  a plain `def` and Starlette runs it in a threadpool — it cannot stall the event loop or the
  analysis worker. If turns get long enough to risk an HTTP timeout the answer is a background job
  polled like `AnalysisWorker`; that is deliberately **not** where this starts.
- **Two tool definitions over one implementation.** The descriptions are shared and pinned, and the
  *names* are pinned to match, but the two signatures are still written twice. That is accepted:
  a signature is checked by mypy and the tests, prose is not.
- **Compaction is out of scope, and is the known next step.** These conversations are a handful of
  turns against a 1M-token window. If one ever approaches the limit, the answer is server-side
  compaction (`compact-2026-01-12`), which requires appending `response.content` — including
  compaction blocks — back to the transcript. Storing blocks verbatim, as decided above, is what
  keeps that door open; truncating the history is never the answer.
- **Server-side refusal fallbacks are deliberately not used.** The `claude-opus-5` guidance is to
  opt into `fallbacks` by default, but golf coaching will not trip a safety classifier, and a
  refusal already becomes a golfer-readable note. Revisit if one is ever actually observed.
- **What this does not do**: it does not let a conversation change anything. Every tool is a read.
  There is no "log this drill", no annotation, no write path — and adding one would need its own
  decision, because it would put a model's output into the artifacts the analysis reads back.
