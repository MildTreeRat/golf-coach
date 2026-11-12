# ADR-036: LLM evaluation and observability — where the call log and the evals live, and the rules they keep

## Status
**Accepted** 2026-11-09 — M43 P1 ([the plan](../plans/m43-llm-evals.md)), approved by the user as
written, every clause as proposed. Built in M43 P2–P21 as decided. What the two evals measured,
what that says about `coaching_model` (a recommendation to the user, not a change), and the places
where the build reads §2 and §5 more exactly than they are written are the 2026-11-11 addendum at
the foot.

It adds new Python inside [ADR-035](035-rust-everywhere-python-where-required.md) clause 1's LLM
exception. That clause names every surviving file, so the new ones are named there too, by an
addendum in ADR-035 that routes back to clause 1 below. It is not a third exception.

## Date
2026-11-09

## Context

**Nothing measures the model.** Claude answers three routes: the club-specification lookup
(`clubs/lookup.py`), the coaching paragraph (`feedback/coach.py`, in process and over `golf-mcp`)
and the follow-up conversation (`feedback/conversation.py`). Every test of them runs on a canned
response handed in through the route's `client=` seam. Those tests prove the code handles what the
model sends, and say nothing about whether the answer is right. No request's cost or latency is
recorded anywhere. `settings.coaching_model` is `claude-opus-5`, and three newer models
(`claude-opus-5-5`, `claude-sonnet-5-5`, `claude-haiku-5-5`) have never been compared with it.

**What the routes already give an eval.** Each takes a `client=`, and with one supplied it never
reads the key. Every expected failure (no key, an SDK error, a refusal, truncation, an exhausted
tool loop) comes back as a **note** rather than an exception, and a 429 is one of them. Every
route sends adaptive thinking with an explicit effort, none forces `tool_choice`, and none sends
`fallbacks`.

**The rules this lands under.**
- ADR-035 clause 1: Python stays only for MediaPipe pose and the LLM, the files that survive are
  named, and "nothing else survives". New files need naming.
- [ADR-022](022-learned-artifacts-as-committed-data.md): committed data carries its source.
- [ADR-026](026-club-specification-lookup.md) §6: a null beats a guessed spec, and a set is one
  request. §7: `clubs/club_catalogue.json` is Claude's own past lookups, so it is the thing under
  test and can never be the answer key.
- [ADR-019](019-secret-handling.md): the set of files that unwrap a secret is pinned
  (`tests/test_config.py::SANCTIONED_UNWRAP_SITES`), and widening it takes an addendum there.
- The `claude-api` skill's eval guides (`shared/evals/build-eval.md`, `eval-audit.md` and
  `report/SCHEMA.md`) define the eval contract: rows, error rows, variants, the harness gate and
  the two sign-offs.

**Facts on disk.** `data/processed/` is git-ignored (`.gitignore:15`). `evals/` and
`.claude/hillclimb/` are not. The repository is public.

## Options Considered

### Option A: Read answers by hand and pick a model
- **Pros**: no code.
- **Cons**: no number to compare, no cost, and nothing to re-run when a prompt changes.

### Option B: Log every call, and stop there
- **Pros**: cost and latency for every request, cheaply.
- **Cons**: measures what an answer costs and not whether it is right, which is the question.

### Option C: A Python call log and a Python eval runner inside the LLM exception, on the skill's contract *(chosen)*
- **Pros**: one language beside the routes it measures, and it calls the real route functions
  through their existing `client=` seams. build-eval.md Step 3 allows the project's own language, so
  long as the runner keeps every property of its `runner-scaffold.mjs`.
- **Cons**: the runner's properties are re-implemented rather than reused, so they need tests of
  their own.

### Option D: The skill's Node runner scaffold
- **Pros**: written already.
- **Cons**: a third language in a repo whose rule is Rust everywhere and Python where required.
  Declined (the plan's Decision 5).

### Option E: Evals as pytest tests behind a marker
- **Pros**: one entry point.
- **Cons**: a test passes or fails and an eval measures. A paid call would sit one flag away from
  every `pytest`, which is the wrong distance.

## Decision

**Option C.** The clauses below are numbered so that P2–P23 and the ADR-035 addendum can cite them.

### 1. The layout, and its import closure

| What | Where |
|---|---|
| The call log | `src/golf_coach/llm_log.py` |
| The price table, with its source and the date it was checked | `src/golf_coach/llm_prices.json` |
| The log file | `llm_calls.jsonl` under `settings.processed_dir` (git-ignored) |
| The runner, the flows and the judge | the `golf_coach.evals` package (`src/golf_coach/evals/`) |
| Eval inputs: answer key and coach cases | `evals/<flow>/` at the repo root |
| Eval runs | `.claude/hillclimb/<flow>/` |
| Tests | `tests/test_llm_log.py`, `tests/evals/` |

**This ADR names the package and the two data directories, not each file in them.** A file inside
`golf_coach.evals`, `evals/` or `.claude/hillclimb/` needs no further decision.

**The closure.** `llm_log` imports nothing from `anthropic`, because `feedback/coach.py` imports it
and must still import clean without the `llm` extra (`tests/api/test_pipeline_imports.py`). The
runner, the flows and the judge import from the standard library, the `anthropic` SDK,
`config.py`, `llm_log` and clause 1's LLM survivors, and from nothing else in `golf_coach`. So they
reach the frozen lab only through what those survivors already import, and they add nothing to the
closure that M40 has to cut.

The coach-case generator is the one exception that might exist. If it lives in `golf_coach.evals`
and needs the frozen lab's shapes, it is a recorder in ADR-035 §4's sense. It writes committed data
that the Rust `golf-mcp` serves, nothing on the run path imports it, and it goes when its imports
go, leaving the committed cases behind. The frozen lab itself gains nothing.

### 2. The call log: one line per API request

`llm_log.record(...)` appends one JSON object per API request to `llm_calls.jsonl`. It logs each
request, not each route call: one conversation turn that calls two tools is three lines. Each line
carries:

- the timestamp (UTC), `origin` (clause 3) and the route (`club_lookup`, `coach` or
  `conversation`);
- the requested model, and the served model as the response names it;
- `usage` (input, output, cache read, cache write), `cost_usd`, `latency_s` and `stop_reason`;
- the outcome (`ok`, or the kind of note the route returned), and the tool-call count;
- for the conversation loop, a turn id and the iteration number;
- an error class when the request failed, and the request id when there is one.

**Metadata only.** A line holds no prompt, no response text and no key. The log stays in the
git-ignored `data/processed/` because it records the user's own use.

**Cost is priced from measured `usage`** with `llm_prices.json`. That file carries, per model, the
input, output, cache-read and cache-write rates and any long-prompt tier (Haiku 5.5's above 100K),
plus a `source` URL and a `checked` date (ADR-022). A model the table does not know is recorded
with `cost_usd: null` and one warning on stderr, **never a zero**. A summary counts and names such
rows and never sums them as zero.

*As built: the coach eval's judge writes no line, so its spend is on the eval's rows (the
2026-11-11 addendum, item 4).*

### 3. The `origin` tag

Every line carries `origin`. It is `app` by default and `eval:<flow>/<variant>` when the runner sets
it with `llm_log.set_origin(...)`. The tag is process-wide, because one eval run is one process and
one origin. `python -m golf_coach.llm_log summary` reports `app` unless asked for `eval` or `all`.
So eval calls go to the same log as real use, and they never pollute what real use costs.

### 4. Logging never changes an outcome

Logging is observation. A failed log write is swallowed with one line to stderr. It never turns a
paragraph into a note, a note into a crash, or a lookup into a different answer. Each route logs at
the one place its request is made: `clubs/lookup.py::_look_up`, `feedback/coach.py::_write` (the
one place both coaching routes reach) and `feedback/conversation.py::_mirror` (one line per message
the tool runner yields). Note paths log too. No key means no request, so it writes no line. No
route's request changes: no parameter, effort, model or `fallbacks` moves.

### 5. Evals are opt-in commands, not tests

A run is `python -m golf_coach.evals.runner --flow … --variant … --model …`, a command the user
approves. The default `pytest` runs the runner's own logic on fake flows and fake clients, and
never makes a call. A test that needs `golf-mcp` skips by name without it, as
`tests/feedback/conftest.py::golf_mcp` does.

**The runner keeps build-eval.md Step 3's contract.** Rows are written as each case completes. A
failed attempt is an `errors.jsonl` row with a failure class and never a zero. A run resumes at the
(case, rep) key. A served model that differs from the requested one is an error. Backoff is the
runner's (`max_retries=0` on its client), keyed on the route's note kind because a route turns a 429
into a note. A per-case wall-clock ceiling records a timeout. A variant directory is `baseline` or
`v<N>`, and a `v<N>` needs its `change.md`. **A zero is a bug until proven otherwise**: a `$0.00`
cost, a `0.0` latency or a constant metric on a successful row fails the run loudly.

*As built: a constant metric is a warning and the zeros stop the run; the backoff keys on the
exception behind the note; and a metric may be absent from a grade where it has nothing to divide
by (the 2026-11-11 addendum, items 1–3).*

**The key.** The runner builds its own `anthropic.Anthropic(max_retries=0)` and hands it to each
route through `client=`, so the route's own key gate never runs. The client resolves
`ANTHROPIC_API_KEY` or an `ant auth` profile, as the SDK does, or is given
`settings.anthropic_api_key`. That last case unwraps a secret at the runner's entry, which makes
`evals/runner.py` an unwrap site. The phase that writes it adds it to `SANCTIONED_UNWRAP_SITES`
with an ADR-019 addendum, the same shape as `feedback/coach.py`'s 2026-10-22 entry: the entry point
unwraps at the call, and nothing below it knows how the key is held.

### 6. Consent: paid runs and the harness gate

**Every paid call has the user's yes, in the phase that makes it.** Before the first call of a
pilot or a full run, the phase shows the estimate with its formula, from measured pilot usage where
there is one. It proceeds only on the user's yes. A run's cost is reported from measured `usage`,
never from a guess. An unattended session stops there and reports `BLOCKED`.

**`--approve-harness` is the user's flag, never the agent's.** The runner keeps a sha over itself
and over `_state.json`'s `harness_paths`, and exits 2 when any of them changes, until the user
passes the flag (build-eval.md Step 3). An agent that could pass it would be grading its own
harness.

### 7. The club-spec answer key never comes from a model

The answer key is built from manufacturer spec pages, or the maker's archived spec sheet for an
older model. A retailer page is used only when the maker's is gone, and is tagged as one. Every
value carries the URL it was read from and the date it was read (ADR-022). A field the source does
not print is recorded as **published-blank**, with the URL of the table that lacks it. Neither
`club_catalogue.json` nor any model's memory is a source. **The user verifies every set before
anything is graded against it.**

The key includes made-up models whose right answer is all blanks. ADR-026 §6 says a blank beats a
guess, so the headline metric is the **invented rate**: a field filled where nothing was published.

**The repository is public.** The user's bag enters `evals/club_spec/` as specifications only:
make, model, slots and the published values. The user's review of those sets is also the user's yes
to publishing them. The coach cases are synthetic swings, generated and committed, and no real
swing enters `evals/`.

### 8. The judge, and how its scores are read

The coach eval's judge is `claude-opus-5-5`, chosen knowing it is also one of the models under test
(the plan's Decision 8). Three rules follow.

- **It is blind.** It never sees which model wrote a paragraph. The brief and the paragraph reach
  it as data.
- **The code checks lead.** The checks are the coach eval's primary metric and the judge's score is
  secondary. The checks are: every quoted number is in the brief, every caveat is carried, nothing
  unmeasured is claimed, and the rule-based verdict is not contradicted.
- **Self-preference is read, not removed.** When the judge favours Opus 5.5's rows by more than the
  code checks do, the findings report that as possible self-preference, not as quality.

The judge request sends **no `fallbacks`**, because a fallback would swap the judge mid-run. A judge
refusal is therefore an `errors.jsonl` row. This declines the skill's default of adding fallbacks
to Opus 5.5 code, and the phase that writes the request tells the user so. Before the judge grades
anything real, it must fail an empty paragraph, "I don't know", and a confident paragraph about a
different swing (build-eval.md).

### 9. Where runs are written, and what is committed

Runs go under `.claude/hillclimb/<flow>/`, with `_state.json`, `baseline/` and `v<N>/`, as
build-eval.md lays them out. Their calls also land in the call log, tagged `eval:…`. **What of a
run is committed is left to M43's close (P23)**, where the user is asked. The recommendation is
everything but `traces/`, with `**/traces/`, `report.html` and `trajectory/` git-ignored. The call
log is never committed. `evals/` (the answer key and the coach cases) is committed whole.

### 10. What this does not change

- `settings.coaching_model` stays `claude-opus-5` through M43. The evals inform that choice, and the
  user makes it afterwards.
- No Rust changes, no contract changes and no `config.py` field. The frozen lab gains nothing
  (ADR-035 §4).
- No route gains `fallbacks`, and no route's request or outcome moves (clause 4).

## Consequences

- **Easier**: what real use costs, per route and model, is one command (`llm_log summary`). The four
  models are compared on measured accuracy, cost and latency rather than reputation. A later prompt
  change has a baseline to beat (`/claude-api hillclimb`).
- **Harder**: every new Claude call site must log, at the one place its request is made. The price
  table goes stale, and its `checked` date is how a reader knows. The answer key ages as makers take
  spec pages down, so a value's URL may need an archive. The judge's self-preference can only be
  read, not removed.
- **For M40**: `llm_log` writes beneath `settings.processed_dir`. So M40's cut of `config.py` to
  "what the two workers read" (ADR-035 clause 1) keeps that field, or gives `llm_log` its path some
  other way.

## Deferred, by choice

These stay out of M43 (the plan's Decision 10): switching `coaching_model`, which is the user's call
from the two full runs; a tool-use eval of the follow-up conversation; a red-team set; live
thumbs-up and thumbs-down on a paragraph (the phone holds no key, so it waits for M38); hill-climbing
a prompt against these evals; and adding `fallbacks` to any route.

## Addendum (2026-11-11, M43 P22): what the two evals measured, what that says about `coaching_model`, and where the build is more exact than §2 and §5

M43 built every clause as decided (P2–P21). Both evals have a baseline and three model variants on
disk under `.claude/hillclimb/`, and the user signed off both graders (P13 the club spec, P20 the
coach). This addendum records what the runs measured. Each number's derivation, and the reading of
the rows behind it, is in [the plan's](../plans/m43-llm-evals.md) P14 and P21 findings.

### The build, against §2 and §5

No clause changed. In four places the build is more exact than the clause, and a reader of the
clause needs to know them:

1. **A constant metric is a warning, not a failure** (P7, the user's call on 2026-11-09). §5 says a
   constant metric "fails the run loudly". An oracle run and a null run are constant by design, so
   the runner prints a constant metric as a warning in its summary. The zeros that **stop** a run
   are on one answered request: no `usage`, zero prompt or output tokens, a cost not above $0, a
   `0.0` latency, or no `model` on the response. Each is a `harness_error` line with no row, and the
   run exits 3.
2. **The backoff keys on the exception behind the note, not on the note** (P8). §5 says "keyed on
   the route's note kind". The runner's `RecordingClient` keeps each request's exception and
   re-raises it unchanged, so the route still meets it and still returns its note. The runner then
   reads the cause off the recorded call. A 408, 409, 429 or 5xx, a connection error or a timeout
   is a `serving_error`, retried with jittered backoff up to five tries. No other note is retried.
3. **A metric can be absent from a grade** (P12, the user's call `absent`). Each club-spec metric is
   defined only where it has a denominator. A made-up set has no published value, so it has no
   `accuracy` or `coverage`. A set that prints every field has no `invented` rate. A real set has no
   `abstention`. An answer that filled no published value has no `accuracy`. The coach's four code
   checks are absent on a refusal (P17). Such a metric is declared `Metric(always=False)`, and a
   grade leaves it out rather than holding a stand-in number, which would move every mean it
   joined. This departs from build-eval.md's rule that every metric id is on every case's grade,
   for those metrics only. The summary and the report average the rows that hold one, so each
   metric's case count is its own.
4. **The coach eval's judge writes no call-log line** (P18). §2's three routes stand, and
   `llm_log.Route` is still those three. The judge's spend is on the eval's own rows (`judge_model`,
   `judge_usage`, and `meta.judge_cost_usd`, which `meta.cost_usd` includes) and on its error lines.
   So "every Claude request writes a log line" holds for the three routes, and the judge's requests
   are the runner's to measure. `llm_log summary --origin eval` leaves a coach run's judge spend
   out, so it is cross-checked against `meta.cost_usd - meta.judge_cost_usd`.

### The spend, measured

All of it is from measured `usage`, priced from `llm_prices.json`. On every variant the rows and
`llm_log summary --origin eval` agree to the cent, with the judge's spend taken out for the coach.

| run | cost |
|---|---|
| club spec: pilot (P13) and full run (P14) | $0.7886 + $4.2143 = $5.0029 |
| the judge's sanity checks (P19) | $0.1362 |
| coach: pilot (P20) and full run (P21) | $0.7198 + $6.0924 = $6.8122, of which P21's judge was $4.375 |
| **M43 in all** | **$11.95** |

### The club-spec eval (P14)

Each model ran 14 sets × 2 reps: 11 real sets, and 3 made-up ones whose right answer is all
blanks. Loft, lie, length, offset and bounce are graded. Coverage is over the 11 real sets, and the
invented rate is over the 8 real sets with unpublished fields plus the 3 made-up ones.

| | Opus 5 (`baseline`) | Opus 5.5 (v1) | Sonnet 5.5 (v2) | Haiku 5.5 (v3) |
|---|---|---|---|---|
| invented, pooled | 0 of 340 | 0 of 340 | 0 of 340 | 0 of 340 |
| coverage, pooled | 88 of 690 | 24 of 690 | 3 of 690 | 2 of 690 |
| accuracy, pooled | 73 of 88 | 21 of 24 | 3 of 3 | 2 of 2 |
| made-up sets left blank | 220 of 220 | 220 of 220 | 220 of 220 | 220 of 220 |
| refusals | 0 | 0 | 0 | 0 |
| latency, mean | 35.8 s | 19.6 s | 14.5 s | 9.9 s |
| cost per case, 2 reps | $0.179 | $0.118 | $0.058 | $0.0032 |

- **The headline cannot separate the models.** No model invented a value, and every model left the
  made-up sets blank. As P13's audit warned, a blank answer scores perfectly on the headline, so
  only coverage separates the models, with accuracy on the few values a model gave.
- **Opus 5 answers the most, and it is wrong the most.** 14 of its 15 wrong values are lofts exactly
  1° from the published loft, twice the ±0.5° tolerance, which looks like a neighbouring
  generation's table. Opus 5.5's 3 wrong values are among those same ones.
- **Most of the smaller models' coverage is an echo.** The TaylorMade R9's loft is in the brief's
  model name. Without it, coverage per case is 0.109, 0.040, 0.001 and 0, in the table's order.
  Every model filled the ungraded text fields (head type, conformity, shaft) and left the numbers
  blank, with notes saying it would not guess.
- **The ranking is suggested, not established.** Paired by case, none of Opus 5's coverage leads
  clears zero: +0.069 ± 0.135 over Opus 5.5, up to +0.109 ± 0.152 over Haiku 5.5. Each lead rests
  mostly on one set, the P790. Rep noise is not the limit, at ±0.05 or less on coverage. Which sets
  are in the key is the limit, so more real sets would narrow the comparison and more reps would
  not.
- **For the user**: the P790's 5i and 6i lofts are the two cells where Opus 5 and Opus 5.5 agree on
  the same 1° difference from the key. The key stands as the user verified it in P11. Those two
  cells are worth a second look, because TaylorMade sold more than one P790 under that name.

### The coach eval (P21)

Each model ran 16 synthetic swings × 2 reps, two swings per coverage tag. The code checks are the
primary metric and the blind judge, `claude-opus-5-5`, is secondary (§8). `all_checks` is never
quoted without `judge_all` and `judge_actionable` beside it (P20's audit), because a paragraph can
pass every code check and still be wrong.

| metric | Opus 5 (`baseline`) | Opus 5.5 (v1) | Sonnet 5.5 (v2) | Haiku 5.5 (v3) |
|---|---|---|---|---|
| `all_checks`, as graded | 0.781 | 0.969 | 0.781 | 0.875 |
| `all_checks`, grader errors removed | 0.938 | 1.000 | 0.906 | 0.906 |
| `judge_all` | 0.594 | 0.594 | 0.594 | **0.062** |
| `judge_actionable` | 1.000 | 1.000 | 1.000 | 0.844 |
| `numbers_in_brief` | 0.938 | 1.000 | 0.906 | 0.906 |
| `caveats_carried` | 1.000 | 0.969 | 1.000 | 1.000 |
| `nothing_unmeasured` | 0.938 | 1.000 | 0.844 | 0.969 |
| `verdict_kept` | 0.906 | 1.000 | 0.969 | 1.000 |
| `judge_accurate` | 0.750 | 0.719 | 0.688 | **0.156** |
| `judge_top_fault` | 0.906 | 0.938 | 0.969 | 0.906 |
| `judge_no_invented_claims` | 0.719 | 0.875 | 0.812 | 0.531 |
| refusals, truncations | 0, 0 | 0, 0 | 0, 0 | 0, 0 |
| coach latency, mean | 8.12 s | 8.83 s | 4.29 s | 3.19 s |
| cost per case, 2 reps | $0.1364 | $0.1202 | $0.0900 | $0.0746 |
| of which the coach | $0.0521 | $0.0477 | $0.0195 | $0.0011 |

- **13 of the 21 failing code-check verdicts are grader errors**, read by hand. That is far past
  eval-audit.md's "roughly one in ten". They are of three kinds:
  - "rotate" in a drill cue ("so the hips rotate rather than slide"), read as a claim that rotation
    was measured. These are all 8 of `nothing_unmeasured`'s failures, and without them that check
    is 1.000 on every model;
  - a sentence that praises the whole swing while naming one miss, or that lists several
    checkpoints, read as speaking of one. These are all 4 of `verdict_kept`'s;
  - a caveat carried in the paragraph's own words rather than the check's markers, which is
    `caveats_carried`'s one.

  `numbers_in_brief`'s 8 failures are correct by the rule: each is a backswing target in
  milliseconds that the brief does not print. So the raw `all_checks` puts Opus 5.5 0.19 above
  Opus 5, and the corrected one 0.06, which is inside the noise. **The code checks' ranking of the
  four models is mostly grader error.** The lite report draws only `all_checks` as graded, so it
  shows Opus 5.5 ahead, and this table is what corrects it.
- **Only Haiku 5.5's drop on the judge is past the noise.** It fails the judge on 30 of 32 rows,
  and read by hand the failures hold. It miscounts the passes ("five of six pass" with two misses),
  reverses the club path's sign, gives the hip slide a direction the prompt says is never
  measured, and calls a value at a band's edge "comfortably inside". The code checks pass most of
  those paragraphs. Opus 5, Opus 5.5 and Sonnet 5.5 tie on `judge_all`, and their corrected
  `all_checks` and every judge criterion differ by less than the noise floors, which reach ±0.24
  at 2 reps (from each model's within-case variance).
- **Self-preference is read, not removed (§8).** On the whole, the judge favours Opus 5.5 less than
  the code checks do: `judge_all` gives it +0.000 over Opus 5, against `all_checks`' +0.188 as
  graded and +0.062 corrected. On one criterion, `judge_no_invented_claims`, the judge favours it by
  +0.156, where the matching check (`nothing_unmeasured`) shows no margin once its errors are
  removed. That is **possible self-preference, not quality**, and it is at the noise floor. Read
  by hand, the failures on both sides are of one kind, a causal link from tempo to the hip slide
  that the brief does not draw. The judge failed Opus 5.5's links of that kind too, and Sonnet 5.5
  gains +0.094 on the same criterion without being the judge.
- **The judge has no human calibration.** It failed the three must-fail paragraphs (empty, "I
  don't know", and another swing's coaching) on every criterion in both reps (P19), and
  `python -m golf_coach.evals.judge_sanity` repeats that check after any change to the judge. The
  user accepted all 50 of the pilot's judge verdicts as graded (P20). That is a sign-off and not an
  independent labelling, so it is not an agreement rate.

### What the evals say about `coaching_model`: a recommendation to the user, not a change

`coaching_model` is still `claude-opus-5` (§10), and M43 changes no route. It is one setting for
all three routes: the club lookup (`api/app.py`'s lookup route), the coach and the follow-up
conversation. These two evals measure the first two. The conversation is not evaluated (Deferred,
above).

- **On the coach, Opus 5.5 and Sonnet 5.5 are indistinguishable from Opus 5** at 2 reps. Sonnet
  5.5 matches it at 0.37× the coach's cost per case and about half the latency. Opus 5.5 matches it
  at 0.92× the cost and 1.1× the latency. **Haiku 5.5 is not a candidate for the coach**, at a
  judge pass rate of 0.06.
- **On the club lookup, the choice is between coverage and abstention.** Opus 5 fills the most
  published values (88 of 690), and 15 of them are wrong. The newer models invent nothing and are
  nearly always right, but they fill almost nothing (24, 3 and 2 values), so the golfer fills the
  numbers in. ADR-026 §6 says a blank beats a guess, so their blanks are on the safe side of that rule.
  This result holds for the lookup's present prompt, which no M43 phase changed.
- **The recommendation.** On the coach, **Sonnet 5.5 is the change the data supports**: the same
  measured quality at about a third of the coach's cost and half its latency. Because one setting
  serves all three routes, the switch also gives up the lookup's coverage. Keeping Opus 5 keeps
  that coverage, with about one value in six wrong. Opus 5.5 is in between: tied on the coach at
  nearly Opus 5's cost, with a quarter of Opus 5's lookup coverage, 21 of its 24 values right. A
  model per route would need a `config.py` field, which is a structural change of its own. Any
  switch makes a stored conversation reseed rather than replay, because thinking blocks are bound
  to the model that wrote them (`contracts/conversation.py`'s `model`). The decision is the
  user's.

### For whoever runs these again

- **The coach grader's three false-positive kinds** above are the list for a fix. A fix changes
  `src/golf_coach/evals/coach_feedback.py`, and so the harness sha, and the next run needs the
  user's `--approve-harness`.
- **A change to `feedback/coach.py::brief_from_view`'s rendering stops every coach attempt** until
  `python -m golf_coach.evals.coach_cases` rewrites `evals/coach_feedback/cases.md` and the user
  approves it again. That is deliberate, because the user approved the briefs as written (P16).
- **More real sets would narrow the club-spec comparison**, and more reps would not.
- **A prompt hill-climb** against either eval starts from these baselines, with
  `/claude-api hillclimb`.
