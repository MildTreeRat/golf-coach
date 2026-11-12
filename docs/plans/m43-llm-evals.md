# Plan: M43 — LLM evaluation and observability

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-11-09, **revised** the same day (a second `/plan-phases` pass split every phase
that had two gates, and settled the runner, the log's origin tag, the answer key's size, the coach
case count and the judge). **Milestone**: [ROADMAP §M43](../../ROADMAP.md#m43-llm-evaluation-and-observability).
This file is the phase list, and where it and the ROADMAP section disagree, **this file wins**.

**Why**: every test of the three Claude routes runs on canned responses, so they prove the code
handles the model and say nothing about whether the model's answers are any good. This milestone
measures that, logs what each call costs, and compares models on accuracy against cost and latency.

**Governing decisions**: [ADR-035](../decisions/035-rust-everywhere-python-where-required.md) §1
(the LLM stays Python, and the files that survive are named, so new ones need a decision of their
own: P1's ADR-036), [ADR-022](../decisions/022-learned-artifacts-as-committed-data.md) (committed
data carries its source), [ADR-026](../decisions/026-club-specification-lookup.md) §6 (a set is one
request), and the `claude-api` skill's `shared/evals/build-eval.md` and `eval-audit.md` (the eval
contract and the two sign-offs).

**Level: L2.** New Python inside the LLM exception, no Rust change, no contract change, no
`config.py` change (that would make it L3), no change to any route's behaviour. Every phase reads
its own list below rather than the repo.

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot, and address anything a later phase
   must know to that phase by number ("**For P12**: …");
3. stop, and tell the user to `/clear` and run `/next-phase` (or `/orchestrate` for the rest).

**Nothing is committed until P23.** Every phase leaves its work in the working tree.

**Phases marked 💲 call the paid API.** Each one stops before its first call, shows the user the
estimate (from measured pilot usage where one exists), and proceeds only on the user's yes. An
unattended run reports `BLOCKED` there instead of guessing. So do P11 and P16, which end on the
user's review.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document, the ROADMAP row and section | ✅ Done *(2026-11-09)* |
| **P1** | ADR-036: where the log and the evals live, and the rules they keep | ✅ Done *(2026-11-09)* |
| **P2** | The call log: `llm_log.record` and the price table | ✅ Done *(2026-11-09)* |
| **P3** | The club lookup writes the log | ✅ Done *(2026-11-09)* |
| **P4** | The coach writes the log | ✅ Done *(2026-11-09)* |
| **P5** | The follow-up conversation writes the log | ✅ Done *(2026-11-09)* |
| **P6** | `llm_log summary` | ✅ Done *(2026-11-09)* |
| **P7** | Eval runner core, on a fake flow | ✅ Done *(2026-11-09)* |
| **P8** | Eval runner hardening: backoff, ceiling, harness gate, report | ✅ Done *(2026-11-09)* |
| **P9** | Club-spec answer key, part 1: the user's bag and about six real sets | ✅ Done *(2026-11-09)* |
| **P10** | Club-spec answer key, part 2: about five real sets and three made-up ones | ✅ Done *(2026-11-09)* |
| **P11** | Club-spec answer key review: the user verifies every value | ✅ Done *(2026-11-10)* |
| **P12** | Club-spec flow and grader, offline | ✅ Done *(2026-11-11)* |
| **P13** | 💲 Club-spec pilot, signed off | ✅ Done *(2026-11-11)* |
| **P14** | 💲 Club-spec full run across the four models | ✅ Done *(2026-11-11)* |
| **P15** | Coach cases, part 1: the generator and eight swings | ✅ Done *(2026-11-11)* |
| **P16** | Coach cases, part 2: eight more swings, approved by the user | ✅ Done *(2026-11-11)* |
| **P17** | Coach code checks, offline | ✅ Done *(2026-11-11)* |
| **P18** | Coach judge request, offline | ✅ Done *(2026-11-11)* |
| **P19** | 💲 Judge sanity checks | ✅ Done *(2026-11-11)* |
| **P20** | 💲 Coach pilot, signed off | ✅ Done *(2026-11-11)* |
| **P21** | 💲 Coach full run across the four models | ✅ Done *(2026-11-11)* |
| **P22** | Docs | ✅ Done *(2026-11-11)* |
| **P23** | Close: verify, WORKLOG, ROADMAP, the one commit | ✅ Done *(2026-11-12)* |

---

## Rules every phase keeps

- **Invoke the `claude-api` skill before touching any code that builds a request or reads a
  response**, as `config.py`'s comment on `coaching_model` already asks. Its files (the eval guides,
  `report/SCHEMA.md`, the lite report builder) live under a **session-scoped** base directory that
  the skill prints when it loads, so invoke it in the phase that needs a path; never reuse a path
  from an earlier session. Model ids are the exact strings in its table, never with a date suffix.
- **No route changes behaviour.** Logging is observation: a failed log write is swallowed with one
  line to stderr and never turns a paragraph into a note or a note into a crash.
- **`llm_log` imports nothing from `anthropic`**, and `feedback/coach.py` still does not import
  `anthropic` at module scope (`tests/api/test_pipeline_imports.py`).
- **New Python only where ADR-036 says.** The frozen lab gains nothing (ADR-035 §4), no Rust
  changes in this milestone, and no `config.py` field.
- **`settings.coaching_model` does not change in M43.** The evals inform that decision; the user
  makes it afterwards.
- **No eval runs in the default `pytest`.** The runner's own logic is tested offline with fakes; a
  real run is a command the user approves. A test that needs `golf-mcp` skips by name without it, as
  `tests/feedback/conftest.py::golf_mcp` does.
- **The answer key never comes from a model.** `clubs/club_catalogue.json` holds Claude's own past
  lookups, so it is the thing under test and never the gold. Every answer-key value carries the URL
  it was read from and the date (ADR-022).
- **Every paid call has the user's yes in that phase** (💲 above), and a run's cost is reported from
  measured `usage`, never guessed.
- **`--approve-harness` is the user's flag, never the agent's** (build-eval.md, Step 3).
- **A zero is a bug until proven otherwise.** A `$0.00` cost, a `0.0` latency or a constant metric
  on a successful row fails the run loudly (build-eval.md, "Trusting a zero").
- **Stay under ~250k context.** A phase that passes ~200k stops at a clean point, records what is
  done, and addresses the rest to the next phase by number. That matters most in P9, P10, P15 and
  P16, whose work comes in countable pieces.

## Verify — the commands, and when each runs

```bash
.venv/Scripts/python.exe -m pytest                 # every phase that touches Python
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
.venv/Scripts/python.exe scripts/conformance.py check   # P23, and any phase touching feedback/
cargo build --release -p golf-mcp                   # before any test or run that reads a coach case
cargo test                                          # P23 only; nothing in Rust should move
```

The report builder ships with the `claude-api` skill, not this repo, and only the lite builder is
extracted (no `build-report.mjs` beside it, checked 2026-11-09):
`node "<skill base>/shared/evals/report/build-report-lite.mjs" .claude/hillclimb/<flow>/`, where
`<skill base>` is what the skill prints when invoked in that session. `node` is on PATH on this box.

## Decisions

Taken in the first interview (2026-11-09):

1. **Scope**: per-call logging, the club-spec eval, and the coach-feedback eval with an LLM judge.
2. **Club-spec answer key**: gathered by Claude from manufacturer spec pages, a source URL per
   value, and **verified by the user entry by entry** before anything is graded against it.
3. **Models compared**: `claude-opus-5` (today's `coaching_model`) against `claude-opus-5-5`,
   `claude-sonnet-5-5` and `claude-haiku-5-5`, on accuracy, cost and latency.
4. **Coach eval swings are synthetic**, so the whole eval can be committed.

Taken in the revision interview (2026-11-09):

5. **The runner is Python**, in `golf_coach.evals`, keeping every property of the skill's
   `runner-scaffold.mjs` (build-eval.md Step 3 allows the project's own language). The Node
   scaffold was declined: it would add a third language to a repo whose rule is Rust everywhere and
   Python where required.
6. **Eval calls go to the same call log, tagged.** Every record carries `origin`: `app` by default,
   `eval:<flow>/<variant>` when the runner set it. `llm_log summary` reports `app` unless asked.
7. **The answer key is about 15 sets**: the user's bag, about 11 real sets (current and older
   models, across makes), and 3 made-up models whose right answer is all blanks. Gathered over two
   phases (P9, P10), then reviewed by the user in a third (P11). About 600 values to verify.
8. **The judge is `claude-opus-5-5`**, chosen knowing it is also under test. So: the judge never
   sees which model wrote a paragraph; the code checks are the coach eval's primary metric and the
   judge's score is secondary; and when the judge favours Opus 5.5's rows by more than the code
   checks do, the findings say so as possible self-preference rather than reporting it as quality.
   **No `fallbacks` on the judge request**: a fallback would swap the judge mid-run, so a judge
   refusal is an `errors.jsonl` row (the skill's default of adding fallbacks to Opus 5.5 code is
   declined here, and P18 tells the user so).
9. **16 coach cases**, two per coverage tag.
10. **Not in M43**: switching `coaching_model` (the user's call, from P14 and P21); a tool-use eval
    of the follow-up conversation; a red-team set; live thumbs-up/down (the phone holds no key, so
    it waits for M38); hill-climbing a prompt against these evals (`/claude-api hillclimb` is the
    tool when wanted); adding `fallbacks` to any route.

## What the planning read found (2026-11-09)

- **The three routes and their entry points.** `clubs/lookup.py::look_up_set` (and `look_up_club`)
  take `llm_model` as a keyword, plus `api_key=` and `client=`; `_look_up` makes the one request
  (around line 396). `feedback/coach.py`: `_write` (line 521) makes the request for both the
  in-process route (`api/pipeline.py` → `generate_coaching`) and the stdio one
  (`coach_through_golf_mcp`, line 1127, which `main` and `golf-lab analyze` use), and
  `_coaching_client` (line 485) takes a `client=` too. `feedback/conversation.py::ask` (line 225)
  runs `client.beta.messages.tool_runner`, and **`_mirror` (line 433) iterates it: each message it
  yields is one API request**, so that is where P5 logs.
- **The key.** The routes gate on an explicit `api_key` (`settings.anthropic_api_key`, env
  `GOLF_ANTHROPIC_API_KEY`) and return a "no key" note without one, so an `ant auth` profile never
  reaches them. The runner therefore builds its own client (`anthropic.Anthropic(max_retries=0)`,
  which resolves `ANTHROPIC_API_KEY` or an `ant auth` profile, or is given the settings key) and
  passes it through each route's `client=` seam. `max_retries=0` makes retries the runner's, so
  they are visible (build-eval.md Step 3). Note that a route turns a 429 into a **note**, not an
  exception, so the runner's backoff keys on the note's kind (P8, P12).
- **Request shape on all three**: `thinking={"type": "adaptive"}`, an explicit
  `output_config={"effort": ...}` (lookup `high` and a format, coach `low`, conversation `medium`),
  `MAX_TOKENS`, and a `max_tokens` stop branch. **No forced `tool_choice`** anywhere, so the 5.5
  family's 400 on forced tool use does not bite, and every route already sends effort explicitly
  (Opus 5.5 and Haiku 5.5 default to `medium`). No route sends `fallbacks`, so a refusal on a 5.5
  model is measured as the route would meet it today: failure class `refusal`.
- **Expected failures already become notes** (no key, SDK error, refusal, truncation, exhausted
  loop): `tests/feedback/test_conversation.py`, `test_coach.py` and `tests/clubs/test_lookup.py`
  pin them. The log records the note's kind as the outcome.
- **`data/processed/` is git-ignored** (`.gitignore:15`), so a log there never reaches the repo.
  `evals/` and `.claude/hillclimb/` are not ignored.
- **Prices**, from the `claude-api` skill's table (cached 2026-10-06), per million tokens in/out:
  Opus 5 $5/$25, Opus 5.5 $4/$20, Sonnet 5.5 $2/$10, Haiku 5.5 $0.10/$0.50 for prompts up to 100K
  ($0.50/$2.50 beyond). Cache writes 1.25x input. Cache reads: Opus 5.5 and Sonnet 5.5 $0.20,
  otherwise 0.1x input. **P2 checks these against the live pricing page** before committing them.
- **A coach case is a sessions directory**: `<session>/<swing>/` holding a manifest and an
  `analysis.json` (a `SwingBundleResult` dump without keypoints and detections), as
  `tests/feedback/conftest.py::_store` writes it. `tests/` is not a package (no `__init__.py`), so
  its helpers (`_store`, `tests/analysis/conftest.py::make_swing`) cannot be imported from `src/`.
- **The user's bag**: `data/processed/golfers/aaron.bag.json`, through `storage/bag_store.py`.
- **The eval guides on disk** (skill 2.1.317): `build-eval.md`, `eval-audit.md`,
  `eval-hillclimb.md`, `report/SCHEMA.md`, `report/build-report-lite.mjs`,
  `report/runner-scaffold.mjs`. No full `build-report.mjs`, so the runner prices its own rows (the
  full viewer is what would otherwise derive `cost_usd`).

## Layout (P1's ADR settles it)

| What | Where |
|---|---|
| Call log module | `src/golf_coach/llm_log.py` |
| Price table, with source and date checked | `src/golf_coach/llm_prices.json` |
| Log file | `data/processed/llm_calls.jsonl` (from `settings.processed_dir`; git-ignored) |
| Eval runner and flows | `src/golf_coach/evals/`: `runner.py`, `club_spec.py`, `coach_feedback.py`, `coach_judge.py`, and the coach-case generator if P15 puts it there |
| Eval inputs | `evals/club_spec/` (`cases.json`, `cases.md`), `evals/coach_feedback/` (`sessions/`, `cases.json`, `cases.md`) |
| Eval runs | `.claude/hillclimb/<flow>/` with `_state.json`, `baseline/` and `v<N>/`, as build-eval.md lays out |
| Tests | `tests/test_llm_log.py`, `tests/evals/` |

ADR-036 names the package and the two data directories, not each file, so a file inside them
needs no further decision.

---

## Phases

### P0 — This plan document, the ROADMAP row and section

Done in the planning sessions: this file, M43's row in "Status at a glance", and its section.

### P1 — ADR-036: where the log and the evals live, and the rules they keep

- **Goal**: one decision record so the new Python is named, as ADR-035 §1 requires.
- **Read**: ADR-035 §1 and §4; this plan above `## Phases`; `docs/README.md`'s ADR index;
  `tests/test_docs_truth.py` (what it checks about ADRs and the index).
- **Touches**: `docs/decisions/036-llm-evaluation-and-observability.md` (new); an addendum on
  ADR-035 naming `llm_log.py`, `llm_prices.json`, the `golf_coach.evals` package and the `evals/`
  data directory; the ADR index row in `docs/README.md`.
- **The ADR decides**: the layout above; the log record's fields (P2's list) and the `origin` tag;
  that logging never changes an outcome; that evals are opt-in commands, not tests; the paid-run
  consent rule and `--approve-harness`; the answer key's source rule; the judge and its self-bias
  reading rule (Decision 8); where runs are written, with what of them is committed left to P23.
- **Precondition (the user's)**: approval of the ADR. Unattended, leave it **Proposed** and report
  `BLOCKED`.
- **Done when**: the ADR is Accepted with the user's approval, and the docs tests pass.
- **Verify**: `pytest tests/test_docs_truth.py`.

### P2 — The call log: `llm_log.record` and the price table

- **Goal**: `llm_log.record(...)` writes one JSON line per API request, with its cost.
- **Read**: ADR-036 (P1); the `claude-api` skill (invoke it: model table, Haiku 5.5's 100K tier,
  and the Pricing row in `shared/live-sources.md`); `clubs/lookup.py` lines 358–470 (what a
  response carries: `.model`, `.usage`, `.stop_reason`, `._request_id`); how
  `clubs/catalogue.py` loads `club_catalogue.json` (load the price table the same way).
- **Record fields**: timestamp (UTC); `origin`; route (`club_lookup` | `coach` | `conversation`);
  requested model; served model (from the response); `usage` (input, output, cache read, cache
  write); `cost_usd`; `latency_s`; `stop_reason`; outcome (`ok` or the note's kind); tool-call
  count; for the loop, a turn id and iteration number; an error class when the call failed; the
  request id when there is one.
- **Origin**: `llm_log.set_origin(label)`, process-wide (one eval run is one process and one
  origin), default `app`.
- **Prices**: `llm_prices.json` carries per model in, out, cache-read and cache-write rates, the
  Haiku 5.5 over-100K tier, a `source` URL and a `checked` date. Check them against the live
  pricing page (WebFetch); if it cannot be fetched, record the skill's cached table and its date as
  the source and say so in findings.
- **Log path**: `settings.processed_dir / "llm_calls.jsonl"`, an argument in tests. No `config.py`
  field.
- **Touches**: `src/golf_coach/llm_log.py`, `src/golf_coach/llm_prices.json`,
  `tests/test_llm_log.py`.
- **Done when**: tests cover the cost per model including cache reads and writes; Haiku's tier; an
  unknown model (recorded with `cost_usd: null` and one stderr warning, never a fake zero); a failed
  write (swallowed, one stderr line); the origin tag; and that `llm_log` imports no `anthropic`.
- **Verify**: pytest, ruff, mypy.

### P3 — The club lookup writes the log

- **Goal**: every request from `clubs/lookup.py` is logged, notes included.
- **Read**: P2's findings; `lookup.py::_look_up` (lines 358–470) and `_note_for` (line 633);
  `tests/clubs/test_lookup.py`'s fakes.
- **Touches**: `src/golf_coach/clubs/lookup.py`, `tests/clubs/test_lookup.py`.
- **Done when**: with the existing fake clients, the route writes exactly one line per request on
  the happy path and on each note path (refusal, truncation, SDK error); no key writes none,
  because no request was made. Latency is measured around the `messages.create` call. Every
  existing test passes unchanged.
- **Verify**: pytest (including `tests/api/test_pipeline_imports.py`), ruff, mypy.

### P4 — The coach writes the log

- **Goal**: every coaching request is logged, notes included, on both routes.
- **Read**: P3's findings; `coach.py::_write` (line 521), `_coaching_client` (line 485) and
  `_note_for` (line 592); `tests/feedback/test_coach.py`'s fakes.
- **Note**: `_write` is the one place both coaching routes reach, so log there and nowhere else.
- **Touches**: `src/golf_coach/feedback/coach.py`, `tests/feedback/test_coach.py`.
- **Done when**: as P3, for the coach. `coach.py` still imports no `anthropic` at module scope.
- **Verify**: pytest (including `tests/api/test_pipeline_imports.py`), ruff, mypy, conformance check.

### P5 — The follow-up conversation writes the log

- **Goal**: each request inside `conversation.py`'s loop is logged with its turn id and iteration.
- **Read**: P4's findings; `conversation.py::ask` (line 225) and `_mirror` (line 433);
  `tests/feedback/test_conversation.py`.
- **How**: each message `_mirror` takes from the tool runner is one request, so log there: latency
  from the previous yield (the first from the call), the iteration index, and one turn id per
  `ask`. An exception mid-loop logs a line with its error class.
- **Touches**: `src/golf_coach/feedback/conversation.py`, `tests/feedback/test_conversation.py`.
- **Done when**: a two-tool-call turn logs three lines under one turn id; an exhausted loop's last
  line says so; every existing test passes unchanged.
- **Verify**: pytest, ruff, mypy, conformance check.

### P6 — `llm_log summary`

- **Goal**: `python -m golf_coach.llm_log summary [--since DATE] [--origin app|eval|all]` (default
  `app`) prints per route × model: calls, outcome counts and failure rate, latency p50/p95, tokens,
  cache-read share, total and per-call cost. Rows with an unknown cost are counted and named, never
  summed as zero.
- **Read**: P2's findings; `llm_log.py`.
- **Touches**: `src/golf_coach/llm_log.py`, `tests/test_llm_log.py` (on a fixture log).
- **Done when**: the summary of a fixture log matches hand-worked numbers in the test.
- **Verify**: pytest, ruff, mypy.

### P7 — Eval runner core, on a fake flow

- **Goal**: `python -m golf_coach.evals.runner --flow <name> --variant <baseline|vN> --model <id>
  --reps N --limit K` runs a flow's cases, grades them and writes rows to build-eval.md's contract.
- **Read**: the `claude-api` skill's `shared/evals/build-eval.md` Step 3 and
  `shared/evals/report/SCHEMA.md` (invoke the skill for the path); `eval-audit.md` §2–§3; ADR-036.
- **The runner has**: a flow protocol (load cases, run one case, grade one output, declare metrics);
  bounded concurrency with a thread pool (the routes are synchronous); the client it builds and
  passes through each route's `client=` (see "The key" above); `set_origin("eval:<flow>/<variant>")`
  at start; rows written to `results.jsonl` as each case completes, with `prompt_id`, `prompt`,
  `tags` (`tags[0]` the primary grouping), `model` from the response, `usage`, `latency_s`,
  `stop_reason`, `status` (`ok` or `truncated`), `grade` as a `{metric_id: number}` dict, and the
  cost (priced through `llm_log`'s table) in `meta`; `traces/<id>_rep<k>.json` in the documented
  turn format; resume at the (case, rep) key; `errors.jsonl` for failed attempts with a failure
  class (refusal, harness or serving error, timeout, genuine failure), the retry count, and
  `model`/`usage` when the call completed; a served-model assertion; variant directories named
  exactly `baseline` or `v<N>`, a `v<N>` refused without its `change.md`; and a summary line with
  the headline metric and its 95% interval.
- **Touches**: `src/golf_coach/evals/__init__.py`, `src/golf_coach/evals/runner.py`,
  `tests/evals/test_runner.py` (the fake flow lives in the test).
- **Done when**: a fake flow's oracle scores ~100% and its null ~0% through the whole runner; an
  induced error lands in `errors.jsonl` and not as a zero; resume skips finished rows and re-runs
  errored ones; a served-model mismatch is an error row.
- **Verify**: pytest, ruff, mypy.

### P8 — Eval runner hardening: backoff, ceiling, harness gate, report

- **Goal**: the rest of build-eval.md's runner properties.
- **Read**: P7's findings; build-eval.md Step 3's four properties and its report-builder section;
  `runner-scaffold.mjs`'s harness gate, for the exact sha rule to mirror.
- **The runner gains**: jittered backoff on transient failures, keyed on the route's note kind
  (rate limit, overloaded, connection), with the retry count recorded and strict per-attempt
  scoring; a hard per-case wall-clock ceiling (`--timeout-s`) that records a timeout, never a zero;
  and the harness-integrity gate (a sha over the runner and `_state.json.harness_paths`; exit 2 until
  the **user** passes `--approve-harness`).
- **Touches**: `src/golf_coach/evals/runner.py`, `tests/evals/test_runner.py`.
- **Done when**: tests show two transient notes then success give an `ok` row with two retries; a
  slow fake becomes a timeout error row; a changed harness file exits 2 until approved; and the lite
  report builder renders a fake run (in a temporary flow directory, not `.claude/hillclimb/`) with
  no blank metric column.
- **Verify**: pytest, ruff, mypy; the lite report builder on the fake run.

### P9 — Club-spec answer key, part 1: the user's bag and about six real sets

- **Goal**: the first ~7 sets, each slot's published loft, lie and length (and offset or bounce
  where printed), every value with its source URL and the date read.
- **Read**: `contracts/club_spec.py` (the fields); `clubs/lookup.py::_brief` (line 472) and
  `_response_schema` (line 488), for what the model is asked and can answer;
  `storage/bag_store.py` and `data/processed/golfers/aaron.bag.json` (the user's bag).
- **How**: WebSearch and WebFetch the manufacturer's spec page, or its archived spec sheet for an
  older model; a retailer page only when the maker's is gone, tagged as such. A field the source
  does not print is recorded as published-blank, with the URL that shows the table without it.
  Never `club_catalogue.json`, never a model's memory. Tag each case (`own-bag`, `current`,
  `older`), and mix makes.
- **Touches**: `evals/club_spec/cases.json`, `evals/club_spec/cases.md` (one table per set, for
  review), `tests/evals/test_club_spec_cases.py` (the schema check: every value has a URL and a
  date, tags are known, slots are real `ClubId`s).
- **Done when**: ~7 sets, every value sourced, the schema test passes. If context nears the limit
  first, stop and address the remaining sets to P10.
- **Verify**: pytest on the schema test, ruff.
- **The user's answers (2026-11-09), after an unattended P9 attempt was blocked** because its
  agent had no WebSearch or WebFetch, and titleist.com returned 403 to plain curl:
  - **Sources: use curl (as P2 did) plus the Wayback Machine.** Fetch with curl's plain user agent,
    never a spoofed browser one; the permission classifier refuses that as bot-check
    circumvention. When a maker's page refuses curl, read its `web.archive.org` copy and cite the
    archive URL along with the original page's URL. The rest of **How** still applies.
  - **The user's bag**: the irons are **Takomo 201 Mk II** ("Takomo 201 mark 2s"), and the driver
    is "a Ping driver of sorts". The user did not name the Ping model, so do not guess one. Check
    both against `aaron.bag.json`, and if they disagree, write down the difference for P11 rather
    than choosing one. The blocked attempt treated Titleist as the bag's maker; the user's answer
    is newer than that.
- **For P22**: the user accepted P7's reading of ADR-036 §5 (see the P7 findings). Add a note under
  §5 so the ADR says the same thing.

### P10 — Club-spec answer key, part 2: about five real sets and three made-up ones

- **Goal**: ~15 sets in all.
- **Read**: P9's findings (including anything addressed to P10); P9's section above for the rules.
- **Adds**: ~5 real sets, balancing P9's makes and ages; **3 made-up models**, with plausible names
  checked by a search to be absent, whose right answer is all blanks, tagged `fabricated`.
- **Touches**: `evals/club_spec/cases.json`, `cases.md`.
- **Done when**: ~15 sets, every value sourced, the schema test passes, `cases.md` shows every case.
- **Verify**: pytest on the schema test, ruff.

### P11 — Club-spec answer key review: the user verifies every value

- **Precondition (the user's)**: the user is present to review. Unattended, report `BLOCKED`.
- **Goal**: the user's yes on every set, as Decision 2 requires.
- **Read**: P10's findings; `evals/club_spec/cases.md`.
- **Do**: present the sets a few at a time; apply the user's corrections to `cases.json` and
  `cases.md`; mark each set `verified` with the date.
- **Done when**: every set is verified, and the findings record it.
- **Verify**: pytest on the schema test (extended to require `verified` on every set).
- **Decided 2026-11-10 (the user, after `/orchestrate` stopped here)**: P11 runs **attended**, the
  user going through the sets a few at a time in their own session (`/next-phase`), not skipped.
  `/orchestrate` cannot run this phase, and resumes from P12 once it is ticked.

### P12 — Club-spec flow and grader, offline

- **Precondition**: P11 done (every set verified). Without it, stop and ask.
- **Goal**: the `club_spec` flow: load the cases, call the real `look_up_set` with the run's model
  through the runner's client, grade each field.
- **Read**: P7, P8 and P11's findings; `lookup.py::look_up_set` (line 329) and its notes;
  `SCHEMA.md`, for declaring a lower-is-better metric (if it cannot, declare `not_invented`).
- **Grading, per field**: right (within a tolerance P13 confirms, proposed ±0.5° loft and lie,
  ±0.25" length), wrong, blank where a value was published (missed), blank where none was (correct
  abstention), or filled where none was (**invented**).
- **Metrics, in this order**: invented rate (the headline), accuracy on answered fields, coverage,
  correct-abstention rate on the made-up models. The route's note kinds map to failure classes,
  transient ones to P8's backoff.
- **Touches**: `src/golf_coach/evals/club_spec.py`, `tests/evals/test_club_spec.py`,
  `.claude/hillclimb/club_spec/_state.json` (metrics, labels ≤14 characters, prices,
  `harness_paths`).
- **Done when**: the answer key fed back as the model's output scores 100% accuracy and 0% invented;
  all blanks scores 0 coverage and 0 invented; a hand-built mixed case matches hand-worked grades.
- **Verify**: pytest, ruff, mypy.

### P13 — 💲 Club-spec pilot, signed off

- **Preconditions (the user's)**: an API credential the runner's client resolves
  (`GOLF_ANTHROPIC_API_KEY`, `ANTHROPIC_API_KEY`, or `ant auth status`); the user approves the
  harness (`--approve-harness`) and the pilot's spend.
- **Goal**: 5 cases × 2 reps on `claude-opus-5`, the report, and the user's yes on the grading.
- **Read**: P12's findings; `eval-audit.md`; build-eval.md "Before the first paid call".
- **Do**: run `eval-audit.md` against the build first (oracle, null, an induced error). Run the
  pilot, read one row end to end (model, usage, cost and latency non-zero), build the report, put
  the pilot's noise floor beside the differences the comparison hopes to see, and ask the user
  whether they would grade any case differently. Iterate until no.
- **Done when**: the user signs off the grading and tolerances, recorded in findings with the
  pilot's measured cost and wall-clock per case. **For P14**: the cost per case, and an estimate
  per model scaled by the price table (labelled as an estimate).
- **Verify**: the report renders, with no blank metric column.
- **The user's answer to P13's BLOCKED (2026-11-11)**: *ran*. The user approved the pilot's spend
  and asked the orchestrating session to run it for them. It ran the command exactly as the
  BLOCKED gave it (`--flow club_spec --model claude-opus-5 --reps 2 --limit 5 --approve-harness`),
  which approved and recorded harness sha `9b23640d87f9` in `.claude/hillclimb/club_spec/_state.json`.
  Exit 0. 10 of 10 (case, rep) scored, 0 truncated, 0 failed attempts, 0 backoff retries,
  **$0.7886 measured**. Rows are in `.claude/hillclimb/club_spec/baseline/`. The runner's summary:
  Invented 0.000 (4 cases), Accuracy 0.938 (2 cases), Coverage 0.108 (4 cases), Made-up blank
  1.000 (1 case). It also warned that `invented` and `abstention` are constant across every scored
  row ("a bug until proven otherwise"), and the next session must account for that. Coverage of
  0.108 is low enough to check too. **The pilot is not to be re-run.** What is left of P13: read
  one row end to end, build the report, set the noise floor beside the differences the comparison
  hopes to see, explain the two constant-metric warnings, and ask the user to sign off the grading.
- **The user's answer to P13's second BLOCKED (2026-11-11)**: **yes**. The user signs off the
  club-spec grading and tolerances as the pilot applied them: the R9's loft 10.5, the one value it
  answered, stays graded as answered even though its model name carries that value, and the i230's
  two lengths on the ±0.25" edge stay right. Length stays ±0.25". No grader edit, so no regrade and
  no re-run. What is left of P13: record the sign-off with the pilot's measured cost ($0.7886) and
  wall-clock (94 s for 10 rows) as the findings' last entry says, then tick P13. The P14 estimate
  there (about $4.80, ceiling $21.65) has **not** been approved. P14 still asks for it.

### P14 — 💲 Club-spec full run across the four models

- **Precondition (the user's)**: the user approves the estimate (cases × reps × four models, from
  P13's measured cost), stated with its formula before the first call.
- **Goal**: `baseline` on `claude-opus-5`, then `v1` = Opus 5.5, `v2` = Sonnet 5.5, `v3` = Haiku 5.5,
  each with a `change.md` naming the model and saying no prompt changed. Reps: 2 unless P13's noise
  floor says otherwise.
- **Read**: P13's findings.
- **Done when**: every variant is complete with no unexplained errors; the findings carry each
  model's invented rate, accuracy, coverage, abstention, refusals, mean latency and cost per case,
  absolute numbers first; the measured spend replaces the estimate.
- **Verify**: the report shows four variants.
- **The user's answer to P14's BLOCKED (2026-11-11)**: **yes, 2 reps**. The user approves the
  spend for all four runs at 2 reps, estimated at about $4.80 with a $21.65 ceiling, as the BLOCKED
  in the findings ("P14 (2026-11-11)") worked it out. Run the four commands that entry gives, in
  its order: baseline, then v1, v2 and v3. Do not pass `--approve-harness` unless the harness sha
  has changed. If the measured spend heads past the $21.65 ceiling, stop and report `BLOCKED`
  rather than finishing.

### P15 — Coach cases, part 1: the generator and eight swings

- **Goal**: one generator, and the first 8 swings (one per tag), each a sessions directory that
  `golf-mcp`'s `get_swing` serves.
- **Read**: `tests/feedback/conftest.py` (`_store`, `golf_mcp`); `tests/analysis/conftest.py::make_swing`;
  `contracts/swing.py` (`SwingBundleResult`); `coach.py::build_brief` (line 304) and
  `brief_from_view` (line 779), for what reaches the model; `coach_through_golf_mcp` (line 1127).
- **Tags**: a clean swing; an unscored metric; an uncalibrated placement; an interim pivot row; a
  flagged shot; partial alignment; no handedness; a swing with nothing to fix.
- **Generator**: committed, re-runnable, and byte-identical on a rerun (fixed timestamps). **No
  second copy of `make_swing` or `_store`**: `tests/` is not a package, so if the generator needs
  them, move them to one importable home and have the conftests import from it. Decide between
  `src/golf_coach/evals/coach_cases.py` and a test-side generator by where that home must be, and
  say which in findings.
- **Touches**: the generator; `evals/coach_feedback/sessions/` (committed swing directories),
  `cases.json` (id, tags, generator inputs), `cases.md` (each case's brief as the model sees it);
  `tests/evals/test_coach_cases.py`.
- **Done when**: `golf-mcp` serves all 8 (a test through the `golf_mcp` fixture), and regenerating
  leaves no diff.
- **Verify**: `cargo build --release -p golf-mcp`, then pytest, ruff, mypy.

### P16 — Coach cases, part 2: eight more swings, approved by the user

- **Goal**: the second case per tag, varied (different values, handedness, club), and the user's
  approval of all 16.
- **Read**: P15's findings; the generator; `evals/coach_feedback/cases.md`.
- **Touches**: `evals/coach_feedback/` (8 more sessions, `cases.json`, `cases.md`).
- **Done when**: `golf-mcp` serves all 16, and **the user approves the inputs**, recorded in
  findings. Unattended, the approval step reports `BLOCKED`.
- **Verify**: pytest on `tests/evals/test_coach_cases.py`.
- **The user's answer to P16's BLOCKED (2026-11-11)**: **yes**. The user approves all 16 coach
  cases in `evals/coach_feedback/cases.md` as written, without the band-estimator note ("bands cut
  from mediapipe:lite"). No pose estimator measured these synthetic swings, so the note would be
  false. The user also accepts the frozen-engine quirk that both no-handedness cases show:
  `flight_landing_offline_yds` is listed with the checkpoints excluded from the score. No case
  changes. What is left of P16: record the approval in findings and tick P16.

### P17 — Coach code checks, offline

- **Precondition**: P16's inputs approved.
- **Goal**: the `coach_feedback` flow: call the real coach entry (`coach_through_golf_mcp`, with the
  run's model, the runner's client, and a `GolfMcpServer` over `evals/coach_feedback/sessions/`),
  then run the code checks.
- **Read**: P7, P8 and P16's findings; `coach.py::coaching_system_prompt` (line 98), for the list
  of what is not measured; `feedback/rules.py`, for the rule-based verdict.
- **Code checks**: every number the paragraph quotes is in the brief (within its rounding); every
  caveat in the brief is carried; none of the things the system prompt says are not measured
  (spine angle, hip rotation, swing plane, club path; take the list from the prompt rather than
  retyping it) is claimed; it does not contradict the rule-based verdict.
- **Touches**: `src/golf_coach/evals/coach_feedback.py`, `tests/evals/test_coach_feedback.py`,
  `.claude/hillclimb/coach_feedback/_state.json`.
- **Done when**: the checks pass a hand-written good paragraph and fail hand-written bad ones (a
  wrong number, a dropped caveat, a claim about swing plane, a contradicted verdict).
- **Verify**: pytest, ruff, mypy.

### P18 — Coach judge request, offline

- **Goal**: the judge that scores each paragraph on a rubric, built and parsed offline.
- **Read**: P17's findings; the `claude-api` skill (invoke it: Opus 5.5's section, structured
  outputs, and `eval-audit.md` §4 "When the grader is an LLM judge").
- **The judge**: `claude-opus-5-5`, adaptive thinking, an explicit effort (`high`; Opus 5.5
  defaults to `medium`), structured output via `output_config.format`, **no `fallbacks`**
  (Decision 8; tell the user, as the skill asks). A rubric of concrete, checkable criteria
  (accurate to the brief, actionable, prioritises the top fault, no invented drill claims), each
  scored with an explanation. The brief and paragraph are passed as data, with nothing naming the
  model that wrote it. Rows carry `judge_model` and `judge_usage`.
- **Touches**: `src/golf_coach/evals/coach_judge.py`, `tests/evals/test_coach_judge.py`, the coach
  flow's metrics in `_state.json`.
- **Done when**: the request is built and parsed with a fake client, and a malformed or refused
  judge answer becomes an error row, not a zero.
- **Verify**: pytest, ruff, mypy.

### P19 — 💲 Judge sanity checks

- **Precondition (the user's)**: the spend approved (six judge calls, measured after).
- **Goal**: the three checks build-eval.md requires of a judge, before it grades anything real.
- **Read**: P18's findings.
- **Do**: feed the judge an empty paragraph, "I don't know", and a confident paragraph about a
  different swing, twice each. All six must fail.
- **Done when**: all six fail; findings carry the measured cost per judge call. **For P20**: that
  cost.
- **Verify**: the six rows on disk, each failing.
- **The user's answer to P19's BLOCKED (2026-11-11)**: **yes**. The user approves about $0.25
  (ceiling $2.06) for the six judge calls on `claude-opus-5-5`, as the findings' "P19 (2026-11-11)"
  entry works it out. Run `.venv/Scripts/python.exe -m golf_coach.evals.judge_sanity` once, with no
  `--approve-harness` and no golf-mcp rebuild. If the measured spend heads past $2.06, stop and
  report `BLOCKED`. Do not re-run past the six calls without asking.

### P20 — 💲 Coach pilot, signed off

- **Preconditions (the user's)**: the pilot's spend approved, and the harness approved.
- **Goal**: 5 cases × 2 reps on `claude-opus-5`, judged, with the report and the user's yes.
- **Read**: P19's findings; `eval-audit.md`.
- **Do**: as P13: the audit first, then the pilot, one row read end to end, the report, and the
  user's grading review, iterating until no.
- **Done when**: the user signs off; findings carry the measured cost per case, judge included.
  **For P21**: the estimate per model.
- **Verify**: the report renders, with no blank metric column.
- **The user's answer to P20's BLOCKED (2026-11-11)**: *ran*. The user approved the pilot's spend
  and harness sha `df41c54a485a`, and asked the orchestrating session to run it for them. It ran
  the command exactly as the BLOCKED gave it (`--flow coach_feedback --model claude-opus-5 --reps 2
  --limit 5 --approve-harness`), which recorded the sha in `.claude/hillclimb/coach_feedback/_state.json`.
  Exit 0. 10 of 10 (case, rep) scored, 0 truncated, 0 failed attempts, 0 unscored, 1 backoff
  retry, **$0.7198 measured**. Rows are in `.claude/hillclimb/coach_feedback/baseline/`. The
  runner's summary on 5 cases: All checks 80.0%, Numbers 90.0%, Caveats 100.0%, No unmeasured
  90.0%, Verdict kept 100.0%, Judge: all 50.0%, J: accurate 60.0%, J: actionable 100.0%, J: top
  fault 100.0%, J: no invented 70.0%. It warned that four metrics are constant across every scored
  row: `caveats_carried`, `verdict_kept`, `judge_actionable` and `judge_top_fault` ("a bug until
  proven otherwise"), and the next session must account for each. **The pilot is not to be
  re-run.** What is left of P20: read one row end to end (judge included), build the report, set
  the noise floor beside the differences the comparison hopes to see, explain the four
  constant-metric warnings, and ask the user to sign off the grading. That includes P17's and
  P18's choices that were left for this sign-off.
- **The user's answer to P20's second BLOCKED (2026-11-11)**: *as graded*. The user signs off the
  coach eval's grading as the pilot graded it. `nothing_unmeasured` is **not** narrowed, so
  `coach_feedback.py` and harness sha `df41c54a485a` stand, with no regrade and no re-run. **For
  P21**: read every `nothing_unmeasured` failure by hand and report the false positives per model
  (the flagged-shot rep 0 drill cue is the known one). The user marked no judge verdicts, so the
  two borderline `no_invented_claims` verdicts (uncalibrated-placement rep 0, unscored-finish rep 1)
  stand as the judge gave them. P21's spend (about $6.4, ceiling $51.52) is **not** approved by this
  answer, and P21 still stops for it. What is left of P20: record this sign-off in its findings
  and tick its row. No further spend and no code change.

### P21 — 💲 Coach full run across the four models

- **Precondition (the user's)**: the estimate approved, judge spend included.
- **Goal**: as P14, for the coach flow: `baseline` plus three variants.
- **Read**: P20's findings.
- **Done when**: every variant is complete; the findings carry each model's code-check pass rate
  (per check), judge score (per criterion), refusals, latency and cost per case, and the
  self-preference reading of Decision 8.
- **Verify**: the report shows four variants.
- **The user's answer to P21's BLOCKED (2026-11-11)**: *2 reps*. The user approves P21's spend at
  2 reps, judge included: about $6.4, worst case $51.52, about 17 minutes, ±0.18 noise. Run the
  four commands in the P21 findings entry exactly as given there, in the order given, with
  `--reps 2`, and check the running total between commands. Stop before the total passes
  $51.52. This approval covers only that run. It does not cover a re-run, a third rep or any other
  spend.
- **The user's answer to P21's second BLOCKED, the API credit running out (2026-11-11)**: *add
  credit and resume*. The user is adding credit to the same account, and the 2-rep approval
  stands. That includes the overrun to about $6.5 in all, of which about $0.07 redoes the three
  coach answers whose judge call failed. Rerun v1's command unchanged, so that only its 11 missing
  rows run, then v2's and v3's unchanged. The $51.52 ceiling still applies to P21's total,
  including the $2.80 already spent. If a call still fails with "credit balance is too low",
  report `BLOCKED` again and do not retry in a loop.

### P22 — Docs

- **Goal**: `CLAUDE.md`'s commands gain the log summary, the runner and the report builder;
  `docs/ARCHITECTURE.md` names the log and the evals; ADR-036 gains a results addendum (the numbers
  from P14 and P21, and what they say about `coaching_model`, as a recommendation to the user, not a
  change).
- **Read**: P14 and P21's findings; ADR-036.
- **Verify**: `pytest tests/test_docs_truth.py`.

### P23 — Close: verify, WORKLOG, ROADMAP, the one commit

- **Do**: run every verify command, `cargo test` and the conformance check included; ask the user
  what of `.claude/hillclimb/` to commit (build-eval.md, "Make it durable": recommended, everything
  but `traces/`, with `**/traces/`, `report.html` and `trajectory/` added to `.gitignore`); WORKLOG
  entry; ROADMAP row and section to done; one commit onto `main`.
- **The user's answer to P23's BLOCKED (2026-11-12)**: *everything but the traces* (option (a),
  ADR-036 §9). Commit both `_state.json` files, each variant's `results.jsonl` and `change.md`,
  `coach_feedback/v1/errors.jsonl` and `_judge_sanity/results.jsonl`, about 543 KB. Add
  `**/traces/`, `report.html` and `trajectory/` to `.gitignore`, so none of those three is
  committed. Finish the WORKLOG line that was waiting on this answer, then make the one commit onto
  `main` as the plan says. Do not push.

## Exit

M43 is done when both evals have a baseline and three model variants on disk, the user has signed
off both graders, and every Claude request writes a log line. What stays out is Decision 10.

---

## Phase findings

### P0 (2026-11-09)

Plan written from the first 2026-11-09 interview, then revised the same day in a second
`/plan-phases` pass: 16 phases became 23 (each phase with two gates split), and Decisions 5–9 were
taken. Nothing else changed. The plan and the ROADMAP row and section are uncommitted, as every
phase's work is until P23.

### P1 (2026-11-09): written, BLOCKED on the user's approval, then approved and Accepted

- **The user's answer (2026-11-09): approved ADR-036 as written, all clauses as proposed.**
  That includes §5 (the runner falls back to `settings.anthropic_api_key`, so P7 adds the ADR-019
  addendum and the `tests/test_config.py` sanctioned site) and §7 (the bag enters `evals/club_spec/`
  as specs only, and P11's review is the user's yes to publishing them). The next session
  finishes P1 using the "To finish P1" steps below and does not ask again.
- **On disk, uncommitted**: `docs/decisions/036-llm-evaluation-and-observability.md` (**Proposed**,
  clauses 1–10); ADR-035's addendum "(2026-11-09, M43): the LLM exception gains the call log and
  the evals", plus a sentence in ADR-035's Status pointing at it; `docs/README.md` with row 036
  (addenda `—`), ADR-035's row at **2**, and "36 decisions, 121 addenda". Docs-truth: 91 passed.
- **To finish P1** (the session that has the user's yes): set ADR-036's Status to **Accepted**
  with the date. Then drop "awaiting the user's approval" from README row 036, the italic "Proposed
  with ADR-036…" line under ADR-035's M43 addendum, and "proposed with" in ADR-035's Status sentence
  and README's ADR-035 row. If the user strikes or changes a clause, README row 036 summarises it
  too. Then tick P1.
- **Clause numbers for later phases to cite**: §1 layout and import closure, §2 log record, §3
  `origin`, §4 logging never changes an outcome, §5 opt-in commands, the runner contract and the
  key, §6 consent and `--approve-harness`, §7 answer key (and the public repo), §8 judge, §9 runs,
  §10 what does not change.
- **The ADR settles two things the plan did not spell out.** Both are flagged for the user's review.
  1. **For P7**: when the runner falls back to `settings.anthropic_api_key`, it unwraps a
     `SecretStr`, and `tests/test_config.py::SANCTIONED_UNWRAP_SITES` pins every file that does
     that. ADR-036 §5 makes `src/golf_coach/evals/runner.py` a sanctioned site. So P7 also touches
     `tests/test_config.py`, a new ADR-019 addendum (the shape of its 2026-10-22 `coach.py` one), and
     `docs/README.md`: ADR-019's row count, and the addenda total 121 → 122. If the user rejects
     this, the runner resolves only `ANTHROPIC_API_KEY` or `ant auth`, and P13's precondition loses
     `GOLF_ANTHROPIC_API_KEY`.
  2. **For P9 and P11**: the repository is public (`.gitignore`'s `.env*` comment). ADR-036 §7
     says the user's bag enters `evals/club_spec/` as specifications only (make, model, slots and
     published values), and P11's review is also the user's yes to publishing them.
- **For P15**: ADR-036 §1 lets the generator live in `golf_coach.evals` and read the frozen lab's
  shapes, as a recorder that nothing on the run path imports. The runner, the flows and the judge
  import nothing from `golf_coach` beyond `config.py`, `llm_log` and the LLM survivors. Note that
  `feedback/coach.py` itself imports `contracts/swing.py` and other non-surviving contracts today,
  so the rule is about the eval modules' own imports and not the survivors' closure.
- **For P2**: the record fields are ADR-036 §2, and a record holds no prompt, no response text and
  no key.
- **For P22**: `CLAUDE.md`'s "Python stays only where it is required" bullet says §1 "names every
  file that survives". It now needs "and its M43 addendum". When ADR-036 gains its results
  addendum, do not count its addenda in words in its Status: `test_an_adr_that_counts_its_own_addenda…`
  pins any `**<word> addenda` it finds.
- **For P23**: the doc-count pin reads `git ls-files`, so "85 markdown documents" stays until the
  files are staged. After `git add` it should read 87 documents, 74 in `docs/`, 37 ADRs and 12 in
  `plans/` (this plan and ADR-036). Check it then, as M42's close did.
- Checked against the code while writing: all three routes skip their key gate when handed a
  `client=` (`lookup.py::_look_up`, `coach.py::_coaching_client`, `conversation.py::_sendable`).
  ADR-026 §6 does carry "a set is one request". `evals/` and `.claude/hillclimb/` are not ignored,
  and `data/processed/llm_calls.jsonl` is (`.gitignore:15`).
- **Resumed and finished (2026-11-09 14:02)**, from the user's recorded yes above. The files
  changed since the 13:52 marker were the three named on disk above, and they were kept as they
  were but for the "To finish P1" edits: ADR-036's Status reads **Accepted** 2026-11-09, approved
  as written; README row 036 reads **Accepted** with "awaiting the user's approval" gone; ADR-035's
  M43 addendum lost its italic "Proposed with ADR-036…" line, and "proposed with" became "with" in
  ADR-035's Status and README's ADR-035 row. No clause changed, so row 036's summary stands.
  Docs-truth: 91 passed. ROADMAP's M43 row still reads "0/23 phases after P0", left for P23.

### P2 (2026-11-09)

- **Built**: `src/golf_coach/llm_log.py` (`record`, `set_origin`, `origin`, `cost_usd`),
  `src/golf_coach/llm_prices.json` and `tests/test_llm_log.py` (31 tests). **One file outside the
  Touches line**: `tests/test_conformance.py::_PACKAGE_DATA` gains `llm_prices.json` (ADR-036 §2).
  `test_every_on_disk_artifact_has_a_schema` scrapes every `*.json` filename constant in `src/`
  and fails on one that has not been classified, and the price table is package data like
  `club_catalogue.json`. Full pytest: 2114 passed, and that test, the one failure, then passed.
  ruff and mypy are clean.
- **The prices were checked against the live page.** `curl` of
  `https://platform.claude.com/docs/en/about-claude/pricing.md` worked on 2026-11-09. There was no
  WebFetch tool, and `live-sources.md` names curl as a way to fetch. **One correction to the
  planning read: Sonnet 5.5's cache read is $0.10/MTok (0.05x), not $0.20.** Opus 5.5 reads at
  $0.20 (0.05x), and the others at 0.1x. A 5-minute write costs 1.25x input and **a 1-hour write
  2x**. The table carries both, and `record` prices a 1-hour split from
  `usage.cache_creation.ephemeral_1h_input_tokens`. Haiku 5.5: a request with **more than 100,000
  prompt tokens, counting cache reads and writes**, pays $0.50/$2.50 on every token (read $0.05,
  writes $0.625/$1). The rates are standard, global and first-party: no batch, no fast mode, and no
  `inference_geo` us 1.1x, because no route sends any of them (the table's `basis` says so).
- **The `claude-api` skill**: this session had no Skill tool. The skill's reference files were read
  from its extracted bundle, `%LOCALAPPDATA%/Temp/claude/bundled-skills/2.1.319/<hash>/claude-api/`
  (`shared/models.md`, `shared/live-sources.md`). There is **no `SKILL.md` on disk**, so its Current
  Models table was not read, though `models.md` carries the same ids and prices. **For P7, P8, P12,
  P18**: an orchestrated session may lack the Skill tool too. `shared/evals/` (build-eval.md,
  eval-audit.md, `report/`) is in that bundle. `ls` it rather than reusing this path.
- **For P3, P4, P5, the API**: `llm_log.record(*, route, requested_model, latency_s,
  response=None, outcome="ok", error=None, turn_id=None, iteration=None, path=None) -> None`. It
  never raises (anything wrong is one `llm_log: …` line on stderr). Pass the response as the SDK
  returned it, or the message the tool runner yielded. When the request raised, pass `error=exc`
  with no response: the line gets `error_class` (the class name), the SDK's `request_id`, and
  `usage`, `cost_usd` and `tool_calls` null. `latency_s` is the caller's `time.perf_counter()`
  difference. Fakes are safe: a `MagicMock` response logs `usage: null` with no warning (pinned),
  and a fake with no `.model` is priced as the requested model.
- **For P3 (it applies to P4 and P5 too): redirect the log in tests before wiring a route in.**
  Once a route calls `record`, every route test appends to the real
  `data/processed/llm_calls.jsonl`. Add an autouse fixture to `tests/conftest.py`, the shape of
  `_throwaway_club_catalogue`, that sets `llm_log._log_path` to a tmp file. That is the seam it
  exists for, and P3's Touches line does not name `tests/conftest.py`. No real log exists yet.
- **For P3, P4, P5 and P8, outcomes**: `outcome` is a free string, `ok` or the note's kind. Use one
  snake_case vocabulary across the three routes, and give a 429 its own kind (`rate_limited`
  suggested; `overloaded` for a 529), because P8's backoff keys on the note kind.
- **For P6**: a line is compact JSON with exactly these keys: `ts` (UTC, ms, `Z`), `origin`,
  `route`, `requested_model`, `served_model`, `usage`
  (`input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`),
  `cost_usd`, `latency_s`, `stop_reason`, `outcome`, `tool_calls`, `turn_id`, `iteration`,
  `error_class` and `request_id`. `cost_usd` is null in two cases, told apart by `usage`: an
  unpriced model (usage present) and a failed request (usage null). Only the first should be
  "counted and named". `cost_usd(model, usage)` re-prices a line's usage dict at the 5-minute
  write rate. The unknown-model warning fires once per model per process (`_warned_unpriced`).
- **For P7**: `set_origin` raises `ValueError` on anything but `app` or `eval:<flow>/<variant>`,
  where the flow matches `[a-z][a-z0-9_]*` and the variant is `baseline` or `v<N>`. Call it once,
  before the first request.
- **For P18**: `Route` is `Literal["club_lookup", "coach", "conversation"]`, ADR-036 §2's list. A
  judge request is none of these. If the judge's calls should reach the log, the list needs a
  `judge` value, and that widens §2. Ask, or leave the judge's cost to the runner's own rows.

### P3 (2026-11-09)

- **Built**: `clubs/lookup.py::_look_up` times the `messages.create` call with
  `time.perf_counter` (imported as `perf_counter`) and calls `llm_log.record` once per request: on
  the exception path with `error=exc`, and after the read with `response=`. The response handling
  moved unchanged into a new `_read(...) -> (ClubLookupOutcome, kind)`. Every note's wording is
  the same as before. The module docstring names the `llm_log` import (ADR-036 §4) beside
  ADR-008's "contracts only". No-extra, no-key and no-slots return before the clock starts, so
  they write nothing. 12 new tests (23 cases) in `tests/clubs/test_lookup.py`, and none of the
  old ones was edited. Full pytest: 2138 passed. ruff and mypy are clean. No
  `data/processed/llm_calls.jsonl` exists after the run.
- **Two files outside the Touches line**, both following P2's finding. `tests/conftest.py` gains
  the autouse `_throwaway_llm_log`, which patches `llm_log._log_path` to
  `tmp_path / "llm_calls.jsonl"`. That changed `tests/test_llm_log.py` too: its
  `test_the_default_log_is_llm_calls_jsonl_under_the_processed_dir` would have passed **by
  coincidence**, because it sets `processed_dir` to the same `tmp_path`. It now captures
  `_REAL_LOG_PATH = llm_log._log_path` at import, puts it back, and points `processed_dir` at
  `tmp_path / "processed"`. **For P4, P5**: the fixture is already in place. A route test reads
  the log at `llm_log._log_path()` (see `test_lookup.py::_logged`).
- **For P4, P5 and P8, the outcome vocabulary as it now stands.** Reuse it, and add a kind only for
  a note the lookup does not have:
  - from a response: `ok` (any specs served, partial included), `refusal`, `truncated`
    (`max_tokens`), `unreadable` (the JSON did not parse) and `no_clubs` (lookup only);
  - from an SDK exception, via `_kind_for(exc, sdk)`, which follows `_note_for`'s branch order:
    `model_not_found` (NotFoundError), `auth_failed`, `rate_limited` (429), `overloaded` (529),
    `api_error` (any other `APIStatusError`), `connection_failed` (`APIConnectionError`, timeouts
    included) and `request_failed` (anything else, or no SDK).
  `rate_limited` and `overloaded` are subclasses of `APIStatusError`, so order matters, and a test
  pins it. The class name always goes in `error_class` beside the kind. `_kind_for` is
  **duplicated, not shared**, the same as `_note_for`: `clubs/` may not import `feedback/`
  (ADR-008), so coach.py and conversation.py each need their own copy over their own `_note_for`.
  The lookup's module docstring says a third copy should go to `contracts/`. Lifting it there, or
  into `llm_log`, is outside P4's and P5's Touches lines, so either one needs the user's yes.
- **For P4**: coach.py keeps a truncated paragraph and serves it with an admission. So
  `truncated` there means "served, with an admission", where on the lookup it means "nothing
  served". If that difference matters to P6 or P17, give coach's its own kind.
- **For P8 and P12**: a lookup note does not carry its kind, because `ClubLookupOutcome` has no
  kind field and adding one would change the route. The runner either reads the kind back from
  the log line, which has the same origin and is the last one written, or matches the note's
  wording ("rate limited", "overloaded"). Choose one in P8.

### P4 (2026-11-09)

- **Built**: `feedback/coach.py::_write` times `messages.create` with `perf_counter` (imported
  as `from time import perf_counter`, so a test patches `golf_coach.feedback.coach.perf_counter`)
  and calls `llm_log.record(route="coach", …)` once per request: on the exception path with
  `error=exc`, and after the read with `response=`. `_write` is the only place that logs, and both
  routes reach it, so `generate_coaching` and `coach_through_golf_mcp` are unchanged. The response
  handling moved into `_written_from(response, model) -> (_Written, kind)`, with every note's
  wording and the request's kwargs exactly as before. The module docstring has a new "The call
  log [M43 P4]" section. `coach.py` imports `llm_log` at module scope (ADR-036 §1 sanctions that)
  and still no `anthropic`, and `tests/api/test_pipeline_imports.py` passes.
- **Tests**: 13 new tests (25 cases) in `tests/feedback/test_coach.py`, and no old test was edited.
  Two run the stdio route on the real `golf-mcp` (built, not skipped): one line for a paragraph,
  none for a miss. Before any request, no extra, no key and an unbuilt server write nothing. Full
  pytest: 2163 passed (P3's 2138 + 25). ruff, mypy and the conformance check are clean, and no
  `data/processed/llm_calls.jsonl` exists after the run. Nothing changed outside the Touches line.
  (Under the repo's `addopts = "-q"`, adding `-q` makes `-qq`, which prints no "N passed" line, so
  read the exit code.)
- **The `claude-api` skill was not invoked.** There was no Skill tool, and the request was not
  touched: no parameter moved, and the existing request-shape tests pin that.
- **For P5, P6, P8 and P17: the vocabulary gains one kind, `no_text`** (the model returned no
  text). That covers the case where thinking spent all of `max_tokens` before any text. The
  line's `stop_reason` tells the two apart (`end_turn` against `max_tokens`). Coach outcomes are
  `ok`, `refusal`, `no_text`, `truncated` and P3's seven SDK kinds.
- **For P6 and P17: on route `coach`, `truncated` means a paragraph was served**, with
  "(This note was cut off before it finished.)" appended. On `club_lookup` it means nothing was
  served. It was kept as one kind because every line carries its `route`. So P6 should count
  `truncated` as served per route, not lump it into failures across routes. And P17's code checks
  see the admission sentence at the end of such a paragraph.
- **For P5**: `coach.py` now has `_kind_for(exc, sdk)` and `_ERROR_KINDS`, in `_note_for`'s
  branch order. `conversation.py` already imports `_note_for` from `coach.py`, so it can import
  `_kind_for` the same way. That needs no third copy and no change to `coach.py`. It answers P3's
  "each needs its own copy" for conversation.py.
- **For P8, P12 and P20**: the coach's answer does not carry its kind either. `CoachingOutcome`
  has text, provenance and note, and the stdio entry's dict is `{text, provenance}` or `{note}`.
  So whatever P8 chooses for the lookup (read the log line back, or match the wording) applies
  here too. The coach's notes say "rate limited" and "overloaded" as the lookup's do.

### P5 (2026-11-09)

- **Built**: `feedback/conversation.py` logs every request the tool runner sends, one line per
  message `_mirror` takes, through a new `_Turn` (one per `ask`, `turn_id = uuid4().hex`).
  `_mirror(runner, messages, turn)` marks each ask of the runner (`turn.asking()`, which starts the
  clock) and each message that arrives (`turn.received(message)`). The reading after the loop moved
  unchanged into `_answered_from(last, tools_called, max_iterations) -> (AskOutcome, kind)`, the
  shape of coach's `_written_from`. `ask` attaches the transcript with `dataclasses.replace` only to
  an answer with no note, as before. `_kind_for` is imported from `coach.py` (P4's finding), so
  there is no third copy and `coach.py` is unchanged. The request's kwargs did not move. The module
  docstring has a new "The call log [M43 P5]" section. There are 19 new test cases in
  `tests/feedback/test_conversation.py`, and none of the 28 old ones was edited. Two of the new
  cases start the real `golf-mcp` (built, not skipped). Full pytest: 2182 passed (P4's 2163 + 19).
  ruff, mypy and the conformance check are clean, and no `data/processed/llm_calls.jsonl` exists
  after the run. Nothing changed outside the Touches line. The `claude-api` skill was not invoked,
  because there was no Skill tool and the request was not touched.
- **Two refinements of the plan's "How"**, both inside ADR-036 §4:
  1. **Latency runs from the loop's ask to the message, not from the previous yield.** The tools
     run between a yield and the next ask (`generate_tool_call_response`, which crosses the pipe
     on the stdio route). Timing "from the previous yield" would add every round of tool calls to
     the request after it. That is P4's rule (the clock brackets the request and nothing of ours),
     and `test_latency_is_each_request_and_not_the_tools_between` pins it. The first request is
     timed from `_mirror`'s first ask, just after `tool_runner(...)`. Building the runner sends
     nothing: the SDK's runner is lazy, and `_handle_request` calls `messages.parse` during
     iteration (`anthropic/lib/tools/_beta_runner.py`).
  2. **The last line is written from `ask`, not from `_mirror`.** When a message arrives, nothing
     says whether the runner will stop after it, so `_Turn` holds each line until that is known.
     The next message's arrival makes it `ok`. After the loop, `ask` reads the answer and
     `turn.finished(kind)` writes the last line with the turn's kind. That is how an exhausted
     loop's last line says so. `_mirror` still hands over every message, and `_Turn` writes
     every line.
- **For P6, conversation lines**:
  - `iteration` counts from 1, as the SDK's `max_iterations` does. An exhausted loop's last line
    has `iteration == max_iterations`. A turn is every line with one `turn_id`, and its outcome is
    the outcome of the line with the highest iteration. Every earlier line is `ok` with
    `stop_reason: tool_use`. So a per-request failure rate on route `conversation` counts those
    `ok`s in its denominator. Group by `turn_id` if a per-question view is wanted.
  - **New kind: `exhausted`.** The loop hit `max_iterations`, the golfer gets the partial text with
    the "stopped early" note, and the transcript is not saved. Conversation outcomes are `ok`,
    `refusal`, `no_text`, `truncated` (served with its admission, as on coach), `exhausted`, and
    P3's seven SDK kinds.
  - A request that raised mid-loop is a line of its own at the next iteration, with `usage`,
    `served_model`, `stop_reason` and `cost_usd` null, `error_class` set and the kind from
    `_kind_for`. The line before it is `ok`. If our own handling raises after a message has
    arrived, which has not been seen in practice because the SDK catches tool exceptions, the
    error goes on that message's own line, beside its usage.
  - No line is written when the runner yielded nothing or could not be built, because no request
    was sent.
- **For tests touching the conversation's failures**: the old fake `_Client(exc)` raises from
  `tool_runner(...)` itself, which the real runner never does for a failed request. Under that
  fake, a failure writes no line, and `test_a_question_that_was_never_sent_logs_nothing` pins
  that. A failed request is modelled by the new `_ScriptedRunner(turns, raises=exc)`, which raises
  from its iteration as the real one does.
- **For P8**: `AskOutcome` carries no kind either (it has text, transcript, tools_called and note).
  Decision 10 keeps the conversation out of M43's evals, so P8 does not have to handle it.

### P6 (2026-11-09)

- **Built**: `src/golf_coach/llm_log.py` gains `read_log(path) -> (lines, skipped)`,
  `summarize(lines, *, origin="app", since=None) -> list[Summary]`, `render(...)`, `main(argv)` and
  an `if __name__ == "__main__"`. So `python -m golf_coach.llm_log summary [--since DATE] [--origin
  app|eval|all]` works. The module docstring has a new "The summary [M43 P6]" section that says
  what each number counts. `tests/test_llm_log.py` gains 10 tests (11 cases) on a fixture log of
  19 lines written out by hand, not through `record`, with every expected number worked by hand
  in the test (`EXPECTED_APP`). One test runs the real `python -m` in a subprocess with
  `GOLF_PROCESSED_DIR` pointed at a tmp dir, and asserts that stderr is empty. Full pytest:
  2193 passed (P5's 2182 + 11). ruff and mypy are clean. `ruff format` was applied to the two
  touched files, and it changed only the new code. No `data/processed/llm_calls.jsonl` exists
  after the run. Nothing changed outside the Touches line.
- **Choices the plan left open, each pinned by a test:**
  1. **A group is origin × route × requested model.** Under the default `app` that is exactly the
     plan's route × model. Under `eval` or `all`, each `eval:<flow>/<variant>` is its own group,
     so a variant never folds into another one or into real use. The requested model is the
     group, because it is the route's choice. A call another model answered is priced as that
     model (`record` already does this) and named in a "served by another model" row.
  2. **Failed means nothing was served, per route** (`_SERVED`): `club_lookup` serves `ok` only,
     while `coach` and `conversation` serve `ok` and `truncated` (P4's finding). `exhausted` is a
     failure. A route the table does not know serves only `ok`, so a future `judge` route would
     summarise without a change here.
  3. **Latency is over the answered requests** (a line with `usage`), and the percentiles are
     nearest-rank (p50 is the ceil(n/2)-th smallest, p95 the ceil(0.95n)-th), so each one is a
     latency that was measured. A 429's 0.5 s is a failed call, not the model's speed.
  4. **Cost sums the logged `cost_usd` and re-prices nothing.** Per-call cost is over the priced
     calls. **Unpriced** means `usage` with a null cost: it is counted, named by the served model,
     and kept out of the total, and both the group and the total line say "not in the ... total".
     A request that raised is neither priced nor unpriced. A group with nothing priced prints
     "none priced", never `$0`.
  5. **Conversation groups add a turns row**: a turn is one `turn_id`, and its outcome is the line
     with the highest `iteration` (P5), not the last line in the file. The fixture interleaves
     turns as two worker threads would write them.
  6. **Cache-read share** is cache reads over input + cache reads + cache writes.
  7. A missing log prints "No calls logged yet", and a filter that matches nothing says so. Both
     exit 0. Unreadable lines (bad JSON, not an object, a field of the wrong type, a bad `ts`) are
     skipped and counted in one `llm_log: skipped N unreadable line(s)` stderr line. The output
     is ASCII only.
- **For P7 and P8**: the runner can read its own spend back with `llm_log.read_log` and
  `summarize(..., origin="eval")`, but that returns every eval origin. For one run, filter
  `line["origin"] == "eval:<flow>/<variant>"` first. The CLI has no per-variant filter and no log
  path option. It reads `llm_log._log_path()`, which is `settings.processed_dir`
  (`GOLF_PROCESSED_DIR`). The runner's own rows remain the measure of a run (build-eval.md), and
  the summary is a cross-check.
- **For P14 and P21**: `python -m golf_coach.llm_log summary --origin eval` gives each variant's
  measured calls, failure rate, latency and cost from the log, as a cross-check on the report.
- **For P18**: the summary needs no change for a `judge` route (see 2), but `Route` and ADR-036
  §2 still do, as P2 said.
- **For P22**: the command and what each number means are in `llm_log`'s module docstring. A doc
  should point there rather than restate it.

### P7 (2026-11-09)

- **The user's answer (2026-11-09): accepted that a constant metric is only a warning.** Zero
  cost, latency or tokens still stop the run (exit 3). ADR-036 §5 still says "fails", so P22 adds
  a note under §5 to match.
- **Built**: `src/golf_coach/evals/__init__.py`, `src/golf_coach/evals/runner.py` and
  `tests/evals/test_runner.py` (31 tests, 68 cases, on a fake arithmetic flow and a fake client).
  The command is `python -m golf_coach.evals.runner --flow F --model M [--variant baseline|v<N>]
  [--reps N] [--limit K] [--concurrency N] [--runs-dir DIR]`. The module docstring says what each
  file holds, the failure classes, the key and the exit codes (0 all scored, 1 an attempt failed,
  2 refused before any request, 3 stopped on a zero). Full pytest: 2261 passed (P6's 2193 + 68). ruff and mypy
  are clean. Nothing was written to `.claude/hillclimb/` or `data/processed/llm_calls.jsonl`, and
  the `claude-api` skill's files (build-eval.md Step 3, SCHEMA.md, eval-audit.md §2–§3 and
  `runner-scaffold.mjs`) were read from the 2.1.319 bundle, because this session had no Skill tool.
- **Three files outside the Touches line**, all from P1's finding 1 (ADR-036 §5):
  `tests/test_config.py::SANCTIONED_UNWRAP_SITES` gains `src/golf_coach/evals/runner.py`, ADR-019
  gains its 2026-11-09 addendum, and `docs/README.md`'s ADR-019 row reads **3**, with the total at
  122 addenda. The pin failed on the runner first, as it should. Docs-truth passes.
- **How a flow plugs in (for P12, P17, P18).** A flow is a `runner.Flow` (an ABC): `metrics` in
  report order, `judge_model`, `load_cases()`, `run(case, ctx)` and `grade(case, output, ctx) ->
  Grade`. The module `golf_coach.evals.<flow>` exposes it as `FLOW`, so **P12 and P17 add `FLOW =
  …` and do not touch the runner**. `main(argv, flows=..., client=...)` is the test seam.
  `ctx.client` is a `RecordingClient`, one per attempt over the run's client. It has
  `messages.create` and nothing else, and both routes call nothing else on their client
  (`lookup.py:411`, `coach.py:547`). Pass it as the route's `client=` and `ctx.model` as its model.
  The runner reads the row's `model`, `usage`, `stop_reason`, `latency_s` (the requests' own time,
  not the flow's), `tool_calls` and the trace off the recorded calls, so a flow returns only what
  it grades. A grader that calls a model uses `ctx.judge_client` and declares `judge_model`. Its
  calls are measured apart (`judge_model`, `judge_usage`, `meta.judge_cost_usd`, with the total in
  `meta.cost_usd`) and kept out of the trace.
- **For P8, and the note-kind question P3 and P4 left open**: the recorder keeps the SDK exception
  that the route swallowed into a note. So the runner classifies **from the exception**, not from
  the log line or the note's wording (`_failure_class_of`). Status 408, 409, 429 or 5xx (529
  included), or a connection or timeout error, is `serving_error`. Any other status, or anything
  else, is `harness_error`. P8's backoff retries `serving_error` only. Rows carry `meta.retries: 0`
  and error lines `retries: 0` for P8 to fill, keeping strict per-attempt scoring.
- **Failure classes**: `serving_error`, `harness_error`, `serving_substitution` (the scaffold's
  name for a served-model mismatch), `refusal`, `genuine_failure` and `timeout`, which is reserved
  for P8. **A refusal on the app's call is a graded row** marked `meta.failure_class: "refusal"`,
  because eval-audit.md §2 says a refusal is the model's answer. An `errors.jsonl` `refusal` is
  what a grader raises with `AttemptFailed("refusal", …)` (the judge, ADR-036 §8). A flow raises
  `AttemptFailed("genuine_failure", …)` when there is nothing to grade and the model caused it.
  **For P12**: that covers the lookup's `unreadable`. A served-model mismatch carries the billed
  `model`, `usage` and `cost_usd`, priced on the served model.
- **A truncated row** (the last app call stopped at `max_tokens`) is written with `grade: {}` and
  never graded or averaged. **For P17**: so the coach's served-with-an-admission paragraph is not
  code-checked. Grading it would need a status override in the runner.
- **One reading of ADR-036 §5, recorded for the user.** The ADR says "a constant metric … fails the
  run loudly". P7's own Done line needs an oracle at ~100% and a null at ~0%, and both are
  constant, so a constant metric is a stderr **warning** in the summary, not a stop. The zeros that
  **stop** the run (exit 3, a `harness_error` line, no row) are: no usage, zero prompt or output
  tokens, a cost not above $0, a 0.0 latency, or no `model` on the response. The worker sets the
  stop flag itself. A test caught that the writing thread falls behind fast attempts, so a stop
  set there let every queued case run.
- **Cost** is priced through `llm_log.cost_usd` on the requested model once the served-model check
  passes (a dated snapshot of it costs the same and is not in the table). Cache writes are priced at
  the 5-minute rate, because no route asks for 1-hour. A `--model`, or a flow's `judge_model`, that
  has no price is refused before any request.
- **The summary** is printed to stdout, and a line per attempt goes to stderr. The headline is the
  first `binary` metric, else the first. It is the mean over cases of each case's mean over its `ok`
  reps, read over every row in the variant's directory, as the report reads it. Its interval is
  Wilson's (n = cases) for a binary metric and the normal one over case means otherwise. Error
  lines superseded by a row are counted as attempts but not as unscored. There is no
  `progress.txt` or 30-second progress line, which the scaffold has and neither P7 nor P8 lists.
- **Also kept from the scaffold**: the run's paths are refused if they are symlinks, a torn last
  line is closed before the next append, control characters are stripped from stderr and error
  text, and case ids are checked as file-safe and as unique ignoring case (refused, not rewritten).
  Metric labels must be 14 characters or fewer, and a grade must carry exactly the declared ids,
  each a finite number (a binary one 0 or 1). A trace is written before its row, so every row has
  its trace.
- **For P8, `_state.json`**: P7 reads nothing from it. A module constant such as
  `STATE_FILE = "_state.json"` will trip `tests/test_conformance.py::test_every_on_disk_artifact_has_a_schema`,
  which scrapes `^_?[A-Z][A-Z_]* = "….json"` from `src/`. It is the skill's format and not this
  repo's, so P8 classifies it in that test (outside P8's Touches line) or asks.
- **For P8, the ceiling**: only the main thread writes, from `as_completed`. A per-case ceiling can
  wait on each future with a deadline and write a `timeout` line for an overdue one. A thread cannot
  be killed, so the late result must be dropped, and the stop flag keeps it from starting more.
- **For P12 and P17**: attempts run on a thread pool, 4 by default. `coach_through_golf_mcp` starts
  a `golf-mcp` per call, so check that it is safe from worker threads before the first real run.
- **For P13**: the credential order is the SDK's first (`ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`,
  an `ant auth` profile), then `GOLF_ANTHROPIC_API_KEY`. The runner prints which one it used.

### P8 (2026-11-09)

- **Built**: `src/golf_coach/evals/runner.py` gains the backoff, the ceiling and the harness gate,
  and `tests/evals/test_runner.py` gains 23 cases (P7's 68 to 91). The command gains
  `--timeout-s S` (1800 unless given, 0 for none) and `--approve-harness`. The module docstring
  has new "Retries", "The ceiling" and "The harness gate" sections, and its "Not here yet" line is
  gone. Full pytest: 2284 passed (P7's 2261 + 23). ruff and mypy are clean. Nothing was written to
  `.claude/hillclimb/` or `data/processed/llm_calls.jsonl`. This session had no Skill tool, so the
  skill's files (build-eval.md Step 3 and its report section, `runner-scaffold.mjs`,
  `build-report-lite.mjs`, eval-hillclimb.md's `_state.json` shape) were read from the 2.1.319
  bundle.
- **One file outside the Touches line, as P7's finding allowed**: `tests/test_conformance.py`
  gains `_FOREIGN_FORMATS = {"_state.json"}`. That is a third named exception, beside package data
  and schema roots, for a file whose format another tool owns (the skill's eval-hillclimb.md). The
  test's message now names that option. `ruff format --check` flags older drift in that file
  (around lines 270 to 490), which was left alone.
- **The backoff keys on the exception the recorder kept** (P7's finding), not on the note's
  wording or the log line. That exception is the note's cause: `rate_limited` (429), `overloaded`
  (529), `connection_failed`, and an `api_error` with a 408, 409 or 5xx. The numbers are the
  scaffold's: five tries, and `min(60, 2**k)` s times a 0.5 to 1.5 jitter before retry k
  (`_backoff_delay`, which tests patch).
- **Strict per attempt, as built.** A try is sent again only when none of its requests was
  answered. So a retry never throws billed work away, and a row's grade, usage, trace and latency
  all come from the one try that answered. A serving error after an answered request is an error
  line carrying that spend, and resume re-runs it. No case is ever re-run because of its grade.
  Rows carry `meta.retries`, plus `meta.judge_retries` when there is a judge. Error lines carry
  both. The judge backs off on its own and never re-sends the app's request. The summary's tally
  line ends "; N backoff retries".
- **The ceiling replaced the thread pool.** Each attempt runs on its own daemon thread, named
  `eval-<case>-rep<k>`, with at most `--concurrency` running at once. The main thread waits until
  the nearest ceiling, at most 1 s at a time, so Ctrl-C still works on Windows. An overdue attempt
  gets a `timeout` line carrying what was billed by then, and its slot goes to the next case. A
  lock decides whether the worker or the ceiling settles an attempt, so there is exactly one line.
  The worker checks the ceiling before each try and before the judge, so an abandoned attempt
  sends nothing more (pinned with a judged flow). A daemon thread means a hung request cannot hold
  the process open. A retry whose backoff would end past the ceiling is not sent, and the line is
  the serving error it met. **One wrinkle**: an abandoned route request that finishes later still
  writes its own `llm_log` line. `run` resets the origin on return, so in a long-lived process
  that line would say `app`. The CLI exits first, so that line never lands there.
- **The harness sha** follows the scaffold's rule: each file adds its name, a NUL, its bytes and a
  NUL, in name order. **It hashes the flow's module automatically**, beside `runner.py`, because
  the scaffold's runner file holds its flow. It also hashes any Python lockfile (`uv.lock`,
  `poetry.lock`, `pdm.lock`, `Pipfile.lock`, `pylock.toml`) beside `runner.py` or at the repo
  root, plus `harness_paths`. `harness_paths` resolve against `REPO_ROOT`, not the cwd. That is
  the scaffold's "cwd, i.e. the repo root", made fixed. Names are repo-relative POSIX paths. This
  repo has no Python lockfile, so every approval prints the "no lockfile" note, and `Cargo.lock`
  is not counted. The gate runs after the refusals that need no state and before cases load, and
  a refusal leaves nothing on disk. An approval writes `_state.json` atomically, keeps every other
  key, and the run goes on. `runner.harness_digest(state, flow)` is public, so the files hashed
  can be shown to the user.
- **Two more, inside the Touches line**: the runner refuses a `_state.json` whose `metrics` are not
  the flow's ids in the flow's order, or give a different kind. Such a list would leave a blank
  column and the wrong headline. Also, an error line's served dated snapshot is now priced as the
  requested model. In P7 it came out unpriced.
- **The lite report on a fake run**: the test module's `Arithmetic` flow and `FakeClient`, run
  through `runner.main` into the session scratchpad, not `.claude/hillclimb/`. The run was 6 cases
  x 2 reps: one case retried, one truncated, one wrong, and one held past a 1 s ceiling. A `v1`
  with its `change.md` answered in words. `_state.json` declared the metrics, as P12 will. `node
  build-report-lite.mjs` rendered "2 variants, 6 cases, primary metric correct (also: chars)":
  baseline 0.750 and v1 0.000, 2 truncated, failed attempts "2 (timeout 2)", and all 22 trace
  links resolve. No column is blank. The only blank cells are the two baseline cases with no `ok`
  rep (all truncated, all timed out), which is the contract.
- **For P12 and P17**: a flow's builder writes `.claude/hillclimb/<flow>/_state.json` with
  `metrics` (ids, kinds and labels, in the flow's order) and `harness_paths` for any grader file
  outside the flow module. The flow module and `runner.py` are hashed already. **Do not list the
  route under test** (`clubs/lookup.py`, `feedback/coach.py`): a hillclimb round edits it, so
  listing it would stop every round for approval. If the coach flow drives `golf-mcp`, the gate
  counts none of the Rust side, so list what matters (P17 decides). The first real run exits 2
  until the user passes `--approve-harness`.
- **For P13 and P20**: the pilot's first run needs `--approve-harness`, and **the user runs it**.
  Give them the exact command, and never pass the flag yourself. The ceiling defaults to 1800 s,
  and the pilot's measured wall-clock is what to tighten it from. Retries are in each row's
  `meta.retries` and the summary line. `llm_log summary --origin eval` shows the 429s and 529s per
  call.
- **For P22**: ADR-036 §5 says the backoff is "keyed on the route's note kind". It keys on the
  recorded exception, which is that note's cause (P7, P8). A doc describing it should say so.

### P9 (2026-11-09)

- **Built**: `evals/club_spec/cases.json` (7 sets, 57 slots: 219 published values and 66
  published blanks), `evals/club_spec/cases.md` (one table per set, **rendered from `cases.json`**),
  and `tests/evals/test_club_spec_cases.py` (20 cases). Full pytest: 2304 passed (P8's 2284 + 20). ruff is clean.
  Nothing in `src/` moved, so mypy was not re-run.
- **Sources, as the user answered**: curl with its plain user agent, plus the Wayback Machine.
  Live maker pages: Takomo, Ping and Mizuno. Wayback copies of maker pages: Titleist (403 to
  curl), TaylorMade (the live URL lands on a DataDome challenge) and Callaway (a Vercel checkpoint,
  429). Each archived source cites both the original URL and the `web.archive.org/web/<14
  digits>/` copy that was read. No retailer page was needed, and no bot check was answered. The
  values were **parsed out of the saved HTML** (a table parser in the session scratchpad), not
  retyped, and the test checks every stored value against its verbatim `published` text.
- **The sets**: Takomo 201 MKII (own-bag, current), Titleist T150 2025 and T250 2025 (own-bag,
  current), Mizuno JPX925 Forged (current), PING i230 (older: the live page says "Fill-In or
  Replacement Only"), TaylorMade P790 2019 (older) and Callaway Apex DCB 2021 (older: "Sold Out").
  Six makes, all iron sets. Only Ping prints bounce, so 8 of the 57 bounce entries hold a value.
- **For P11: the bag file and the user disagree, and P9 did not choose.** `aaron.bag.json` holds
  Titleist T150 in 4i, 5i, 8i, 9i and pw (6i retired, no model year), a 2025 Titleist T250 7i
  (loft 30.5, lie 63.0 and length 37.0, typed in M15 P1, which agree with this key), and no driver.
  The user's 2026-11-09 answer is Takomo 201 Mk II irons and an unnamed Ping driver. So all three
  iron sets are tagged `own-bag`, and each case's notes say why. P11 asks which set is the bag,
  then drops `own-bag` from the others, which stay as `current` real sets. There is **no driver
  case**, because the user named no model. P11 can ask for it, and a driver added then follows P9's
  rules. P11's review is also the user's yes to publishing the bag sets (ADR-036 §7).
- **The case format (for P10, P11 and P12)**: `{about, fields, cases}`. `fields` is the five
  graded `ClubSpec` fields: `loft_deg`, `lie_deg`, `length_in`, `offset_mm`, `bounce_deg`. A case
  is `{id, tags, make, model, model_year, model_year_source, slots, slot_labels, sources, notes,
  answers}`, a closed key set so nothing personal rides in. `answers[slot][field]` is `{value,
  published, unit, source}`: `value` is in the field's unit, `published` is the cell as printed,
  and `unit` is the unit it was printed in. `value: null` is **published-blank**, with
  `published` and `unit` null and `source` naming the page whose table lacks the field. A source is
  `{kind, url, archive_url, read_on, where}`. The kinds are `maker`, `maker-archive`, `retailer`,
  `retailer-archive` and `absence-search`.
- **Calls P9 made that P11 should check**:
  - **Slots**: a maker's AW or GW is `gw`. A wedge labelled only W (Titleist, 48°) or UW (Ping, 50°)
    is left out, because calling it `gw` would be a guess.
  - **Year**: `model_year` is set only where the page prints one. Those are Titleist ("2025
    T-Series Irons"), TaylorMade (the page's Customize link names the head "P790 2019") and
    Callaway ("Mens 2021 Apex DCB Irons Product Specs"). The other cases leave it null, so the
    brief says "not given".
  - **Age tags**: `current` means the maker offered the set when its page was captured. `older`
    means the page is gone, says fill-in only, or shows the set sold out.
  - **Units**: offsets printed in inches (Ping, Mizuno) are converted ×25.4 and rounded to 0.01 mm.
    Fractional lengths (Ping's `38 7/8"`) are written out.
- **For P10**:
  - **Made-up models are already covered.** `problems` checks a `fabricated` case: it carries no
    other tag, every value is null, and every source is an `absence-search` (the search URL that
    found nothing, its `read_on`, and a `where`). A synthetic case in
    `test_each_rule_catches_the_fault_it_names` shows that shape passes.
  - **Once P10 adds them**, add `fabricated` to `test_the_key_has_the_users_bag_and_both_ages_of_real_set`.
    After editing `cases.json`, rewrite `cases.md` with
    `.venv/Scripts/python.exe tests/evals/test_club_spec_cases.py`.
  - **For balance**, add makes not yet here (Srixon, Cobra, Wilson, PXG), and something other than
    irons. Wedges in particular would make bounce more than a Ping-only field.
  - **Finding archived copies**: the Wayback CDX query
    `https://web.archive.org/cdx/search/cdx?url=<host/path>*&output=txt&fl=timestamp,original,statuscode&filter=statuscode:200&collapse=urlkey`
    lists them. It sometimes answers 504, and a retry works. Titleist's old models are archived
    under `titleist.com/golf-clubs/irons/<year>-<model>`, and Callaway's URLs carry the year
    (`irons-2014-apex-pro.html`). Dead ends already tried: the archived Callaway 2019 Apex page
    is a pre-owned listing with no spec table, and Mizuno and Ping product pages print no model
    year.
- **For P12**:
  - **Only the five `fields` are graded.** `_response_schema` asks for every `ClubSpec` field, and
    this key has no answer for the rest (swing weight, head weight and so on). A model filling one
    of those is neither right nor invented by this key.
  - **Tolerance**: compare offsets with at least 0.01 mm of slack, for the inch conversion.
  - **Inputs**: a case's input to `look_up_set` is `make`, `model`, `model_year` (which may be
    null) and `slots`, in bag order. `slot_labels` holds the maker's own label, for the report.
  - **Ids**: case ids are lowercase and hyphenated, and unique ignoring case, as the runner needs.
- **For P23**: `evals/club_spec/cases.md` is a markdown file that `git ls-files '*.md'` counts once
  it is staged. So P1's projected count (87 documents) gains one here, plus whatever P15 and P16
  add.

### P10 (2026-11-09)

- **Resumed once.** A usage limit cut the first session after it had only fetched maker pages into
  its scratchpad. Nothing but the 🔄 marker was in the tree, so nothing had to be kept or redone.
- **Built**: 8 cases appended to `evals/club_spec/cases.json`, P9's 7 left byte-for-byte as they
  were. The key now holds **15 sets and 119 slots: 393 published values, 92 published blanks, and
  110 blanks in the 3 made-up sets**. `cases.md` was re-rendered and shows every case. The `about`
  text says what a `fabricated` case means. `tests/evals/test_club_spec_cases.py` changed in four
  places:
  - the tag test is now `test_the_key_has_the_users_bag_both_ages_of_real_set_and_made_up_ones`,
    and it requires `fabricated`;
  - `_PUBLISHED` reads a number with no leading zero, because PXG prints offsets as `.220"`. A new
    assertion pins it;
  - the `render` intro explains made-up sets and the G, S, L and DW wedge rule;
  - the summary line counts made-up blanks apart from published blanks.

  Results: schema test 20 passed, full pytest 2304 passed (P9's count, as no test was added),
  ruff clean. Nothing in `src/` moved, so mypy was not re-run.
- **The real sets** (5), each parsed from saved HTML by a table parser, with no values retyped:
  - `pxg-0311-xp-gen7` (current): the live page. It has wedges and a bounce for every slot.
  - `cobra-king-tec-hybrid-2025` (current, **hybrids 2h to 5h**): the live page. It is the one set
    that is not irons.
  - `wilson-staff-model-cb` (current): the live page, with a bounce for every slot.
  - `srixon-zx7-mk-ii` (older): the Wayback copy from 2023-01. The live URL now answers 404, and
    the ZXi7 replaced the set.
  - `cleveland-launcher-xl-halo` (older): the Wayback copy from 2023-01. The live URL now
    redirects to the club listing.
- **Balance**: 11 makes now. The 5 new ones are the four P9 named (Srixon, Cobra, Wilson and PXG)
  plus Cleveland. The ages are 7 current, 5 older and 3 fabricated. The plan's even split needed an
  older Wilson page, and the Wayback index returned none for wilson.com's iron paths. Bounce is now
  graded from three makers (Ping, PXG and Wilson), 26 values in all. The `sw` and `lw` slots appear
  for the first time.
- **Wedge-only lines were not used.** Cobra's KING wedge page labels each wedge only by loft and
  grind, so giving one a slot would be a guess. Cleveland's Smart Sole Full-Face does label C, G, S
  and L, but it prints no bounce. No other wedge line was tried (vokey.com answers curl with 403).
- **For P11: calls P10 made, for the user to check**:
  - **PXG's G, S and L are recorded as `gw`, `sw` and `lw`**, because each letter is that slot's
    initial. P9's rule named only AW and GW. PXG's W (41°) is left out, and so is Cleveland's DW
    (49.5°).
  - **Cobra's `model_year` 2025 does not come from visible text.** It comes from the page's
    product data (description "KING TEC Hybrid 2025 | COBRA Golf") and its URL handle. The title
    alone says "KING TEC Hybrid", and an earlier KING TEC hybrid exists.
  - **Two `published` texts are parts of a cell.** PXG's loft is the part after the dash in
    `4 - 18°`. Cobra's lie is the bracketed standard figure in
    `55.5° - 59.5° (57.5° STD LIE)`. The adjustable ranges are not recorded.
  - **Starred rows are kept.** Wilson's `3*` and `GW*` are kept although the page never explains
    the star (the stock SKUs are 4-PW). Srixon's `SW*` ("Available as custom only") is kept too.
    Cleveland's length is the Standard build length, not the custom Accuracy build.
- **The made-up models**: `titleist-t275`, `taylormade-p795` and `mizuno-jpx935-forged`. Each name
  follows its maker's real numbering. Each case's one `absence-search` source is a Wayback CDX
  prefix query over the **maker's own product URLs**. Each query returned 0 rows, and each was run
  beside a control for the real model:
  - `/product/t250*` returned 125 rows;
  - `/P790*` returned 152 rows;
  - `/us/golf-clubs/jpx925*` returned 8 rows.

  Each control's rows include the very page P9 read, and every count was the same on a second run.
  **There is no general web search behind these cases.** DuckDuckGo answers curl with a bot check
  (202, "anomaly"), which was not attempted. Bing answers curl with brand pages that ignore the
  model token, and returned the same pages for the real T250, P790 and JPX925 as for the made-up
  names, so a missing result there proves nothing. **For P11**: a browser search of the three
  names is the human check. If one turns out to exist, replace it with another name, checked the
  same way.
- **For P12**:
  - **The 3 `fabricated` cases have every field null** and carry placeholder slot labels (`4`,
    `PW`), since no maker printed any. Any value the model fills in for them is invented.
  - **One case asks for hybrid slots** (`2h` to `5h`), and its `model_year` is 2025. The inputs
    are otherwise as P9 said.
  - **Offset tolerance**: PXG and Wilson offsets are three-place inches converted and rounded to
    0.01 mm, so P9's 0.01 mm of slack covers them.

### P11 (2026-11-10)

- **Attended**, after `/orchestrate`'s unattended run reported `BLOCKED` at the precondition, as
  the decision under P11's section records. The user went through the key in four batches. Every
  set now carries `verified: "2026-11-10"`.
- **The bag is Takomo 201 MKII irons and a TaylorMade R9 460 driver at 10.5°.** That corrects the
  2026-11-09 answer of "a Ping driver of sorts". The user's words were "takomo only", then "Remove
  all stale data. Keep the taylormade driver and keep takomo irons". When asked which data that
  meant, they rejected the question and said to continue. So it was read as everywhere:
  - **The key**: `titleist-t150-2025` and `titleist-t250-2025` were **deleted**, not kept as
    ordinary current sets. The `own-bag` cases are `takomo-201-mkii` and the new
    `taylormade-r9-460`. The T275 absence search's `where` named "the T250 page a case above is
    read from", so it now names that page's URL instead.
  - **`data/processed/golfers/aaron.bag.json`**, which is outside this plan's Touches and
    git-ignored, was brought up to date through the frozen Python `BagStore.set_entry`, which
    writes the format Rust's `crates/storage` reads. The Titleist T150 4i, 5i, 8i, 9i and pw and
    the T250 7i were **retired onto the bag's shelf**, not deleted. The bag now holds Takomo
    201 MKII 4i, 5i, 7i, 8i, 9i and pw, with the Clubs table's loft, lie, length and offset, plus
    the R9 460 driver (loft 10.5, lie 59, length 45.75). Provenance is `typed`, and the notes cite
    each page, as M15 P1's 7i entry did. **No Takomo 6i was added**, because the bag had already
    retired its 6i, and the user did not say whether they carry one. A pre-change copy was saved
    to that session's scratchpad, which does not outlive the session. **Consequence**:
    `loft_for_club` reads the bag's current lofts, so a swing re-analysed from now on is flown
    with Takomo lofts (7i 31° where it was 30.5°).
- **Takomo was re-read** from the URL the user gave,
  `https://takomogolf.com/products/iron-201-mkii?variant=46180786077893` (Shopify variant: right
  hand, KBS Tour Lite Regular, Lamkin Crossline Black Standard). Its Clubs table matches the
  2026-11-09 read value for value, because the shaft and grip options change no head figure. The
  case's source now cites that URL, read 2026-11-10.
- **The driver case, `taylormade-r9-460`** (`own-bag, older`), follows P9's rules. It is from the
  Wayback copy of TaylorMade's specifications tab,
  `web.archive.org/web/20111109134604/https://www.taylormadegolf.com/products/drivers/r9-460-driver/product-detail-specifications`.
  The https form of the original URL resolves in Wayback (200), so the schema's https rule holds.
  The table has one column per loft (8.5, 9.5, 10.5 and 11.5), with lie 59° and length 45.75" in
  every column, and it prints no offset, no bounce and no year. The lookup is given only make,
  model, year and slots, so **the loft is in the model text, "R9 460 10.5°"**, the way a golfer
  would type it. That was the user's call ("keep the taylormade driver" answered that proposal).
  The slot label is the column header, `10.5`. It is also the key's first `driver` slot. The
  Wayback index only found this page under the 2011 `products/drivers/` scheme: the 2009 `R9/`
  and `R9_driver/` pages are Flash, and no 2009 `product_detail.asp?pID=` page for the R9 was
  captured.
- **Approved as presented**: batches 2 and 3, the bag sets, and every call P9 and P10 left for
  this phase. Those are the `gw` rule (AW and GW are `gw`; W, UW and DW are left out), years only
  where printed, Ping's standard loft only, PXG's G/S/L as `gw`/`sw`/`lw`, Cobra's 2025 from
  product data, the partial-cell `published` texts, and the starred rows. The plan says this
  review is also the user's yes to publishing the bag sets (ADR-036 §7). The user was told so at
  the start, and did not object.
- **The three made-up names**: a web search (the session's WebSearch tool, not the user's
  browser) for each of "Titleist T275 irons", "TaylorMade P795 irons" and "Mizuno JPX935 Forged
  irons" found no such model. Each result set was all neighbours: the 2025 T-Series, the P790, and
  the JPX925, 923 and 921. Each case's notes now say so, and the user accepted that as the human
  check. **None was replaced.**
- **The key now**: 14 sets, 103 slots, 345 published values, 60 published blanks and 110 made-up
  blanks. By tag: 2 `own-bag`, 5 `current`, 6 `older` (the driver among them) and 3
  `fabricated`. There are 11 makes, and Titleist is now only the made-up T275.
- **The schema test**:
  - `verified` joined `CASE_KEYS`, so the closed key set still rejects anything unplanned.
  - `_verified_problems` requires an ISO date that is not in the future and not before the
    case's latest `read_on`.
  - `render` prints "Verified by the user: <date>" under each set's tags.
  - Two new breakages, `_unverified` and `_verified_before_it_was_read`, prove the rule.

  Results: schema test 22 passed (P10's 20 plus 2), full pytest 2306 passed (P10's 2304 plus 2),
  ruff clean, and ruff format leaves the file alone. Nothing in `src/` moved, so mypy was not
  re-run.
- **For P12**:
  - **`verified` is not an input.** A case's input is still `make`, `model`, `model_year` and
    `slots` (P9's finding), and the grader can refuse any case whose `verified` is null as a
    second guard.
  - **The driver case puts the loft in the model text** ("R9 460 10.5°"). That is deliberate,
    not noise for the grader to strip.
  - **The driver's offset and bounce are published-blank**, so a lookup that writes `0` for a
    driver's offset or bounce scores **invented** under P12's rules. Decide whether a zero there
    is an invention or a not-applicable, and say which in P12's findings. The key itself should
    not change for it.
  - **There are now 14 cases, not 15.** Five cases × 2 reps for P13's pilot still fits.
- **For P23**:
  - The bag file change is outside git. The WORKLOG entry should say that the bag was rewritten
    in P11, and why, since no diff will show it.
  - The user's bag (the Takomo set and the R9 460) differs from what M12 and M15 recorded. Any
    doc that names the user's clubs as Titleist is now stale. `grep -rn "T150\|T250" docs
    ROADMAP.md WORKLOG.md` will find them, and only the current-state ones need a note.

### P12 (2026-11-10, finished 2026-11-11): built, BLOCKED on absent metrics, answered `absent`

- **The block.** The plan's four shares cannot all be defined on any one case, and the runner P7
  built refuses a grade that lacks any declared metric (`runner._check_grade`). That rule follows
  the skill: "have the runner populate every id on every case's grade" (build-eval.md:118). The
  three made-up sets have no published value, so they have no accuracy or coverage. Three real
  sets print every field, so they have no invented rate: `ping-i230`, `pxg-0311-xp-gen7` and
  `wilson-staff-model-cb`. The other eight real sets still have no abstention, which applies to
  made-up models only. A refusal or an all-blank answer has no accuracy either. **Fed the answer
  key back through the real runner** (a fake client, a scratchpad runs dir, `--approve-harness` as
  `tests/evals/test_runner.py` passes it on fake flows), **all 14 cases failed as `harness_error`**
  ("the grade's metrics ['accuracy', 'coverage', 'invented'] are not the declared [...]"), and no
  row was written. The question to the user, and the options:
  - **`absent`** (recommended): the runner lets a flow declare the metrics a row may leave out,
    and only where the share has nothing to divide by. A present value is still checked as a
    finite number, and a required metric that is missing is still refused. The runner's
    `summarize` and `build-report-lite.mjs` already average only the values a row holds. The
    costs: it departs from build-eval.md:118 for the declared metrics; it touches `runner.py` and
    `tests/evals/test_runner.py`, outside P12's line; and in the lite report, whose per-case column
    is the headline only, the three every-field sets show a blank `invented` cell, where nothing
    could be invented.
  - **`shares`**: keep the runner and the skill's rule, and grade five shares of each set's graded
    fields (right, wrong, missed, abstained, invented), each defined on every case. The plan's
    four rates are then pooled by hand from the rows in P13 and P14, and neither the runner nor
    the report shows them.
  - **`standins`**: keep the runner, and write a fixed number where a share has no denominator.
    Not recommended, because a stand-in moves every mean it joins: accuracy 1 on an all-blank
    answer scores the null run perfect, and coverage 0 on a made-up set marks the oracle down for
    leaving it blank. It is ADR-010 §2 ("no score beats a wrong one") at the eval's boundary.
- **Built, and passing on its own**: `src/golf_coach/evals/club_spec.py` (`FLOW`, the per-field
  grader, and the shares with `None` where they have no denominator, left out of the `Grade`), and
  `tests/evals/test_club_spec_flow.py` (19 tests, 23 cases). **The plan's file name could not be
  used**: `tests/` is not a package, so test basenames must be unique, and
  `tests/contracts/test_club_spec.py` exists. Pytest refused to collect the second one, so the
  file sits beside P9's `test_club_spec_cases.py`. The tests cover the plan's three checks by name,
  at the grader:
  - the key fed back scores accuracy 1.0, coverage 1.0, invented 0.0 and made-up abstention 1.0,
    wherever each share is defined;
  - an all-blank answer scores coverage 0.0 and invented 0.0, and made-up abstention 1.0. It has
    no accuracy anywhere. Empty `specs`, and specs holding only their identity, grade alike;
  - a hand-built two-slot case gives invented 1/3, accuracy 4/6 and coverage 6/7, worked by hand,
    with both tolerance edges counted as right.

  They also pin the route path on a fake client: one request, the run's model, slots in key
  order, and the brief's year "not given". A refusal is graded as all blank, and an unreadable or
  clubless answer is a `genuine_failure`. Unverified sets and a field with no tolerance are
  refused at load. Results: ruff clean, ruff format clean, mypy clean (119 files). Full pytest
  2329 passed (P11's 2306 + 23). No `_state.json` was written, because its metrics depend on it.
  Nothing was written to `.claude/hillclimb/` or `data/processed/llm_calls.jsonl`. This session
  had no Skill tool, so SCHEMA.md, build-eval.md and eval-hillclimb.md were read from the 2.1.319
  bundle. No request shape was touched: the flow calls `look_up_set` as the bag page does.
- **Calls P12 made, for P13's sign-off**:
  - **A zero is invented** (P11's question). The R9 460's page prints no offset or bounce, so a
    `0` there is graded as any other unpublished figure. The route keeps `0.0` as a value, so the
    bag would store "no offset" as a fact. Pinned in
    `test_a_zero_where_the_maker_printed_nothing_is_invented`. The key does not change.
  - **Tolerances** (`club_spec.TOLERANCE`): loft and lie ±0.5°, length ±0.25" (the plan's
    proposal), and P12's own offset ±0.25 mm and bounce ±0.5°. An answer exactly at the edge is
    right, with 1e-9 of slack for float noise.
  - **Accuracy leaves inventions out** (right over right plus wrong), because `invented` already
    counts them. `abstention` is `1 - invented` on a made-up set, and it is a separate column
    because `invented`'s mean mixes real and made-up sets.
  - **What is graded is the route's outcome**, the specs the golfer is shown, and not the raw JSON.
    The refusal check reads the recorded response's `stop_reason`, not the note's wording.
  - **The report averages per case**, so a one-club driver weighs what a ten-club set does. Each
    row's `meta` carries `slots`, `published` and `unpublished`, and each metric's `explanation`
    names the fields behind it, so a pooled rate can be worked out from the rows.
  - **The imports** are `clubs.lookup`, `contracts.club` and `contracts.club_spec`. These are
    ADR-035 §1's LLM survivors and the shapes it says the LLM side owns, so ADR-036 §1's closure
    is unchanged.
- **The user's answer (2026-11-11): `absent`.** Do the `absent` branch below; it covers
  `runner.py` and `tests/evals/test_runner.py` too, though they are outside P12's file list. Do
  not do `shares` or `standins`.
- **To finish P12 (the session that has the user's answer)**:
  - **`absent`**: add a per-metric flag to `runner.Metric` (something like `always: bool = True`),
    so `_check_grade` accepts a grade that omits only flagged metrics. Flag `invented`, `accuracy`,
    `coverage` and `abstention`: `invented` too, because of the three every-field sets. Then add
    runner tests for an omitted flagged metric (accepted) and an omitted unflagged one (refused).
    The runner docstring's results section needs a line on absent metrics, and the flow docstring
    loses its "does not yet accept" sentence.
  - **`shares`**: replace `rates` with the five per-field shares over `len(results)`, and the
    `METRICS` with five float metrics, `invented` first and lower-is-better (with `wrong` and
    `missed`). Rewrite the three plan checks in terms of those shares.
  - **Either way**, then:
    - write `.claude/hillclimb/club_spec/_state.json` with `metrics` (each `id`, `kind`, `label`,
      `better` and `scale`, in `METRICS` order) and `harness_paths: ["evals/club_spec/cases.json"]`.
      The answer key is grader data, so an edit to it should stop a run for approval. The flow
      module and `runner.py` are hashed already, and the route is not listed (P8). **Omit
      `prices`**: build-eval.md:118 says it is only for models missing from the full viewer's
      built-in table, the full viewer is not on disk, and the runner prices every row from
      `llm_prices.json`, so a copy would only drift;
    - add a through-the-runner test (`runner.main` with `flows={"club_spec": FLOW}` and a fake
      client answering from the key, as the scratch run above did), where the oracle reads
      ~100%/0% and the null reads 0 coverage and 0 invented;
    - tick P12.
- **For P13**: the evidence script (`oracle_run.py`, which feeds the key back on a fake client) was
  in this session's scratchpad and will not outlive it. Its whole method is the bullet above.
  The user's sign-off covers the calls above as well as the tolerances.

**Resumed and finished 2026-11-11, on the user's `absent`.** The cut-off session's
`club_spec.py` and `test_club_spec_flow.py` were kept as they stood. Only `METRICS` changed
(`always=False` on all four) and the docstring sentence that said the runner did not yet accept an
absent metric.

- **The runner** (`runner.py`, outside P12's Touches line, as the user's answer allowed).
  `Metric` gains `always: bool = True`. `_check_grade` now refuses a grade that lacks an
  `always=True` metric or holds an undeclared one. The message names the metrics that may be
  left out. **Two tightenings the plan did not name**, both in the same spirit:
  - a grade that scores nothing at all is refused as a blank row. That case was unreachable
    before, because every metric was required;
  - an `explanation` must be for a metric the grade *scores*. Before, it only had to be a
    declared metric. An explanation beside an absent score would show a reason for a blank cell.

  `always` is the runner's own, so it never reaches `_state.json`, because SCHEMA.md's `Metric`
  has no such field. The module docstring's results section gains the absent-metric paragraph. It
  says this departs from build-eval.md's "every id on every case's grade" for the declared
  metrics only.
- **Runner tests**: `tests/evals/test_runner.py` gains 5 cases on a `Sparse` flow, where `chars`
  is `always=False` and left out on the two products:
  - accepted, with the summary averaging `chars` over its own 4 cases (mean 1.250, CI 0.760 to
    1.740, worked by hand);
  - and refused, as `harness_error`: a required metric left out, a present optional one not
    finite, an explanation of an absent score, and an empty grade.
- **`.claude/hillclimb/club_spec/_state.json`** holds `metrics` (id, kind, label, better and scale,
  in `METRICS` order) and `harness_paths: ["evals/club_spec/cases.json"]`. It has no `prices`,
  and no `harness_sha`, which is the user's to record at P13. A test pins it against `FLOW.metrics`
  and the runner's own state check. It also pins that the digest hashes exactly
  `evals/club_spec/cases.json`, `src/golf_coach/evals/club_spec.py` and
  `src/golf_coach/evals/runner.py`.
- **Through the runner** (`test_club_spec_flow.py`, now 26 tests): `runner.main` runs on the real
  flow, with a fake client answering each request from the key by its brief's make and model, and
  the committed `_state.json` copied into a temporary runs dir.
  - **The oracle** exits 0, with 14 rows, each graded exactly as `grade_specs` grades the key. It
    reads `Invented 0.000` over 11 cases, Accuracy and Coverage `1.000` over the 11 real sets, and
    `Made-up blank 1.000` over 3.
  - **The null** answers every slot with every field blank. It reads Invented `0.000` (11),
    Accuracy "no scored rows", Coverage `0.000` (11) and Made-up blank `1.000` (3).
  - A null of `{"clubs": []}` would be a `genuine_failure`, as the flow's docstring says, not an
    all-blank row.
  - Both runs warn on stderr that every metric is constant, which is P7's rule (a warning, not a
    stop).
- **The lite report** (`build-report-lite.mjs`, 2.1.319 bundle) rendered both runs from a kept
  basetemp: "1 variants, 14 cases", primary metric `invented`. In its per-case column, the three
  every-field sets show a blank cell, as the user accepted with `absent`.
- **Verify**: full pytest 2337 passed (2329 + 8), ruff clean, `ruff format --check` clean on
  `src/golf_coach/evals` and `tests/evals`, mypy clean (119 files). Nothing was written to
  `data/processed/llm_calls.jsonl`. Nothing under `.claude/hillclimb/` but the one
  `_state.json`. No request was made.
- **For P13: `--limit 5` takes the key's first five cases, and none of them is made up.** The key
  orders `takomo-201-mkii`, `taylormade-r9-460`, `ping-i230`, `mizuno-jpx925-forged` and
  `taylormade-p790-2019` first. The three made-up sets are the last three (`titleist-t275`,
  `taylormade-p795`, `mizuno-jpx935-forged`). So a pilot of `--limit 5 --reps 2` would leave
  the `abstention` column with no scored rows, and P13's "no blank metric column" check would fail.
  The runner has no flag to pick cases. P13 must choose and ask, rather than improvise:
  - run all 14 cases at one rep;
  - reorder the key so that the first five include a made-up set and an every-field set (an edit
    to a harness path, so it needs re-approval anyway);
  - or add a case-selection flag to the runner (a runner change).
- **For P13**: eval-audit.md §2's oracle and null now exist as these two tests, so the scratch
  `oracle_run.py` is not needed. The induced error is P7's and P8's runner tests on the fake flow.
  The first real run exits 2 until **the user** passes `--approve-harness`, and that sha covers
  the three files above.
- **For P22**: ADR-036 does not mention absent metrics. A doc describing the club-spec grade
  should say that each of its four metrics is defined only where it has a denominator. It should
  also say that `Metric.always=False` is the runner's departure from build-eval.md:118.

### P13 (2026-11-11): audited offline, BLOCKED twice, piloted, signed off

No request was made. Nothing was written to `data/processed/llm_calls.jsonl` or under
`.claude/hillclimb/`, and no file in the repo changed but this one. The skill's files were read
from the 2.1.319 bundle, because this session had no Skill tool.

- **eval-audit.md, the offline half.** `tests/evals`, `tests/test_llm_log.py` and
  `tests/clubs/test_lookup.py` pass (266). They include P12's oracle and null through the runner.
  A scratchpad script then ran `runner.main` on the **real** `club_spec` flow, on a fake client,
  with the log redirected and `BACKOFF_BASE_S` cut to 0.01 s. Every case landed where it should:
  - a 401 became an `errors.jsonl` `harness_error`, with no row;
  - a 429 and then an answer became an `ok` row with `meta.retries` 1;
  - a 529 on every try became a `serving_error` after 4 retries, at $0 measured;
  - `max_tokens` became a `truncated` row with no grade;
  - a refusal on a real set became a graded row (invented 0, coverage 0) with
    `meta.failure_class: refusal`;
  - a wrong-but-plausible answer (loft and lie +1°, length +0.5") took accuracy from 1.000 to
    0.125 on the first two sets.

  The fake usage (1,500 in, 400 out) was priced at $0.0175, which is Opus 5's $5 and $25 per
  million. The route under test cannot reach the key: `look_up_set` reads no catalogue (the cache
  is its caller's), and the key is not in its request. The gold is the makers' pages, verified by
  the user, and no model wrote it.
- **What the audit turns up, worst first. Each one is for the user's sign-off, and none is a bug.**
  1. **The headline rewards saying nothing.** An all-blank answer and a refusal both score
     invented 0.0, and made-up blank 1.0. Only coverage separates a model that answers less from
     one that answers better. **For P14**: never report `invented` without coverage beside it,
     per model.
  2. **A refusal on a real set is a coverage zero.** It averages in with capability zeros, and
     only `meta.failure_class` tells them apart. eval-audit.md §2 asks for refusals as their own
     metric. P12 chose otherwise, and the user's sign-off covers that choice. **For P14**: count
     refusals per variant from the rows, as P14's Done line already asks.
  3. **No lockfile**, so a dependency change is outside the harness sha. The runner prints this
     itself. It is low severity.
- **The credential (the user's precondition).** `.env` holds a non-empty `GOLF_ANTHROPIC_API_KEY`.
  The agent's shell has no `ANTHROPIC_API_KEY` or `ANTHROPIC_AUTH_TOKEN`, and no `ant` on PATH.
  So unless the user's own shell sets one, the runner falls back to the app's key and prints that.
- **What one request is, measured offline** (captured on a fake client for all 14 cases):
  - about 4,500 characters in (a 2,578-character system prompt, a brief of about 120, and an
    output schema of about 1,790), with `max_tokens` 16,000 and effort `high`;
  - the sets run from 1 slot (the driver) to 10 (`srixon-zx7-mk-ii`).

  **A ceiling, not an estimate**: at most 3,000 input tokens × $5/M, plus 16,000 × $25/M of
  thinking and output, is **$0.415 per call** on `claude-opus-5`. That gives at most $4.15 for
  10 calls and $5.81 for 14. A retry adds nothing billed, because the runner retries only a try
  that no request answered. The real figure comes only from the pilot's `usage`
  (build-eval.md), and P13 records it.
- **The block.** P12 found that `--limit 5` takes the key's first five cases, and none of them is
  made up. The `Made-up blank` column would then have no scored rows, and this phase's Verify
  ("no blank metric column") would fail. P12 left the choice to the user. The options:
  - **(a)** all 14 cases × 1 rep. Nothing changes, and the ceiling is $5.81. But one rep gives no
    rep-to-rep spread, and P14's "2 reps unless the noise floor says otherwise" needs one.
  - **(b), recommended**: move `titleist-t275` to fifth in `evals/club_spec/cases.json` (and
    `cases.md`, if a test pins the two to one order), then 5 × 2, with a ceiling of $4.15. The
    first five are then the user's Takomo set, the R9 driver (which tests the zero-is-invented
    ruling), an every-field set, a real set with blanks, and a made-up set. That fills every
    column. The harness sha has never been recorded, so the user's first `--approve-harness`
    covers the reorder at no extra step.
  - **(c)** add a `--cases id,...` flag to `runner.py` and its tests, then 5 × 2. It leaves the
    key alone, and P20's 16 coach cases will hit the same `--limit` problem. But it is a runner
    change outside P13's line.
- **The command is the user's**, because `--approve-harness` is never the agent's (P8). For
  (b) or (c), after the change:
  `.venv/Scripts/python.exe -m golf_coach.evals.runner --flow club_spec --model claude-opus-5 --reps 2 --limit 5 --approve-harness`
  For (c), use `--cases <ids>` in place of `--limit 5`. For (a), use `--reps 1` and no
  `--limit`. It writes `.claude/hillclimb/club_spec/baseline/`. If the grading holds, P14's
  baseline run resumes after those rows rather than paying for them twice. If the grading
  changes, delete them first.
- **The user's answer (2026-11-11): (b).** Move `titleist-t275` to fifth in
  `evals/club_spec/cases.json`, and the pilot is 5 cases × 2 reps on `claude-opus-5`, at most
  $4.15. Do not do (a) or (c). The user approved this pilot but has **not run it yet**. They run
  it themselves (`--approve-harness` is their flag), after the reorder is in place.
- **The user's first run (2026-11-11) was refused before any call**: `refusing to run: there is no
  flow named 'club_spec' (no FLOW that is a runner.Flow)`. No request was made and nothing was
  billed. `import golf_coach.evals.club_spec` works and `FLOW` *is* a `golf_coach.evals.runner.Flow`.
  The likely cause is the `-m` double import: run as `python -m golf_coach.evals.runner`, the
  runner is `__main__`, so the `isinstance(found, Flow)` at `runner.py:552` checks against
  `__main__.Flow`, a different class from the one `club_spec.py` subclasses. The tests miss it
  because they pass `flows={...}` to `runner.main` directly. **For the session that resumes P13**:
  fix this (for example, have `__main__` call `golf_coach.evals.runner.main` through a fresh
  import), add a test that runs the real `-m` entry point and resolves a flow by name, and then
  hand the pilot back to the user to run again.
- **For the session that resumes P13** (the row stays 🔄):
  - make the user's choice, if it is (b) or (c), and run the pytest, ruff and mypy checks;
  - after the user's run, read one row end to end (served model, usage, cost and latency all
    non-zero, and `meta.retries`);
  - build the report: `node "<bundle>/shared/evals/report/build-report-lite.mjs" .claude/hillclimb/club_spec/`;
  - work out the noise floor on `invented` and `coverage` from the two reps, and put it beside
    the differences P14 hopes to see;
  - show the user one graded row per kind, and record their sign-off with the measured cost and
    wall-clock per case;
  - put the per-model estimate for P14, scaled by `llm_prices.json` and labelled as an
    estimate, in these findings.
- **Resumed 2026-11-11 08:20: the user's (b) is in place, and the pilot is still not run.** No
  request was made, and the row stays 🔄.
  - `titleist-t275` is now the fifth case in `evals/club_spec/cases.json`. It was moved by
    loading the file and dumping it again (`indent=2`, `ensure_ascii=False`), which gives back the
    file byte for byte when the order is unchanged. So the only change is the order. `cases.md` was
    rendered again by `tests/evals/test_club_spec_cases.py`, and its summary table follows the new
    order. No test pins the order of the cases.
  - `FLOW.load_cases()[:5]`, which is what `--limit 5` takes, is now `takomo-201-mkii`,
    `taylormade-r9-460`, `ping-i230`, `mizuno-jpx925-forged` and `titleist-t275` (fabricated). So
    the `Made-up blank` column will have scored rows.
  - pytest (2,337 passed), ruff and mypy are clean.
  - When this note was written, `.claude/hillclimb/club_spec/` held only `_state.json`, and
    `data/processed/llm_calls.jsonl` did not exist. The user's command is unchanged:
    `.venv/Scripts/python.exe -m golf_coach.evals.runner --flow club_spec --model claude-opus-5 --reps 2 --limit 5 --approve-harness`
  - **For the session that resumes P13 next**: the reorder is done, so start at "after the
    user's run" in the list above. If `baseline/` still holds no rows, report `BLOCKED` again
    rather than run the pilot.
- **Resumed 2026-11-11 09:31: the `-m` double import is fixed, and the pilot is still not run.**
  No request was made. `.claude/hillclimb/club_spec/` still holds only `_state.json`,
  `data/processed/llm_calls.jsonl` does not exist, and the row stays 🔄.
  - **The cause was the one guessed above, and it went further than `_flow`.** The user's command
    reproduced from a scratch runs dir, with the same refusal and exit 2. Run as `-m`,
    `runner.py` is `__main__`, so each class check in it is made against the `__main__` copy's
    classes. That covers `_flow`'s `isinstance(found, Flow)`, and past it `_cases`'
    `isinstance(case, Case)`, the grade's `isinstance(grade, Grade)` and the three
    `AttemptFailed` catches. So a run past the flow lookup would have failed every case as a
    `harness_error`.
  - **The fix** (`runner.py`, the `__main__` block only): the command calls
    `importlib.import_module("golf_coach.evals.runner").main()`, the importable module's `main`,
    whose classes the flows share. A comment there says why. Nothing else in the runner changed.
    This changes `runner.py`, which the harness sha covers, but no sha has been recorded yet, so
    the user's first `--approve-harness` covers the fix too.
  - **Two tests, both shown failing on the old entry point and passing on the new one:**
    - `test_runner.py::test_a_real_flow_is_found_by_name_from_the_command_line` runs the real
      `python -m golf_coach.evals.runner --flow club_spec` as a subprocess. It uses the committed
      `_state.json`, with any `harness_sha` dropped, so the gate refuses before a client is built.
      That stays true after the user approves, so this test can never make a request. It asserts
      exit 2 on "no approved harness sha" and not on "no flow named", and that nothing was written.
    - `test_club_spec_flow.py::test_the_command_line_entry_runs_the_flow_by_name_through_every_case`
      runs the file as `__main__` through `runpy`, on a fake SDK (`runner._sdk` patched), with
      `--limit 2 --approve-harness` in a temporary runs dir. Both rows come out `ok` and graded
      as `grade_specs` grades the key, with no `errors.jsonl`. This test exercises the `Case`,
      `Grade` and `AttemptFailed` identities that the subprocess cannot reach without a request.
  - The user's command, `--reps 2 --limit 5` without `--approve-harness`, now stops at the
    harness gate in a scratch runs dir: "no approved harness sha ... (computed 9b23640d87f9 over:
    evals/club_spec/cases.json, src/golf_coach/evals/club_spec.py, src/golf_coach/evals/runner.py)".
    That is the step the user's own run passes with the flag.
  - Full pytest: 2,339 passed (2,337 plus the 2 new tests). Ruff, `ruff format --check` on
    `src/golf_coach/evals` and `tests/evals`, and mypy (119 files) are clean.
  - **For the session that resumes P13 next**: start at "after the user's run" in the list above,
    as before. If `baseline/` still holds no rows, report `BLOCKED` again. The user's command is
    unchanged:
    `.venv/Scripts/python.exe -m golf_coach.evals.runner --flow club_spec --model claude-opus-5 --reps 2 --limit 5 --approve-harness`
  - **For P17**: the coach flow is found by name through the same fixed entry. Its tests should
    include one like the `runpy` test above, because a flow's tests that pass `flows={...}` to
    `runner.main` cannot see an entry-point fault.
- **Resumed 2026-11-11 10:10, after the user's pilot: read, reported, and BLOCKED on the
  sign-off.** No request was made and no code changed. The only new files are the report builder's
  `.claude/hillclimb/club_spec/report.html` and `trajectory/scores.tsv`. The row stays 🔄 until
  the user signs off.
  - **The pilot as it ran.** It made 10 calls, one per (case, rep): 5 cases × 2 reps, harness sha
    `9b23640d87f9` recorded. `data/processed/llm_calls.jsonl` holds exactly those 10 rows, all with
    `origin` `eval:club_spec/baseline`, served `claude-opus-5`, outcome `ok`, a `request_id` each,
    and $0.78862 in total. That matches the runner's sum of `meta.cost_usd`. There were 0 truncated
    rows, 0 failures and 0 retries.
  - **One row end to end** (`taylormade-r9-460` rep 0): `model` `claude-opus-5`; usage 2,101 in
    and 959 out, with no cache tokens; cost $0.03448, which is 2,101 × $5/M + 959 × $25/M;
    latency 15.59 s; stop `end_turn`; `meta.retries` 0. The trace holds the system prompt, the
    brief and the raw JSON answer. Every field is present and none is zero where it should not be.
  - **The headline, recomputed from the raw rows** (eval-audit.md §5), agrees with
    `runner.summarize`: Invented 0.000 (4 cases), Accuracy 0.9375 (2 cases), Coverage 0.1083
    (4 cases) and Made-up blank 1.000 (1 case). Pooled by field, coverage is 10 of 206 published
    values.
  - **The two constant-metric warnings are the model's behaviour, not a wiring fault.** Every
    trace's raw JSON was read field by field. It holds a non-null graded value in only two places:
    the R9's loft (both reps) and `ping-i230` rep 1's eight lengths. Both are published fields.
    So `invented` is 0 because Opus 5 filled no unpublished field, and `abstention` is 1.0 because
    both `titleist-t275` reps left all 35 fields blank. Their notes say no Titleist "T275" exists.
    The metrics do move when the answer moves: P12's hand-built case grades invented 1/3, and
    P13's offline audit took accuracy from 1.000 to 0.125 on a wrong answer.
  - **Coverage of 0.108 is real too.** Under the route's prompt ("do not report any value you
    are not confident of"), Opus 5 answered nothing measurable on `takomo-201-mkii` and
    `mizuno-jpx925-forged` in either rep, or on `ping-i230` rep 0. It answered only non-graded
    text fields there (head type, USGA conformity, and on the i230 the stock shaft and grip). Its
    notes say it would not interpolate. So on this prompt the eval mostly measures how much a model
    is willing to answer without inventing anything. That is what the comparison is for, and no
    prompt changes in M43.
  - **The report** (`node "<2.1.319 bundle>/shared/evals/report/build-report-lite.mjs" .claude/hillclimb/club_spec/`)
    wrote "1 variants, 5 cases", with primary metric `invented` (also: accuracy, coverage,
    abstention). The lite report draws one metric column, the headline, and it reads 0.000 over
    4 cases. Only the `ping-i230` per-case cell is blank, as the user accepted with `absent`. The
    other three metrics are not columns in the lite report at all. The runner's summary above
    prints all four, and none of them is blank. Each per-case row links its two traces.
  - **The noise floor** is measured on Opus 5's two reps.
    - `invented`: 0 on every case and rep, so there is no spread to measure.
    - `coverage`: the two reps match on 3 of the 4 real sets, and on the i230 they read 0.0 and
      0.2. That is a per-row SD of about 0.07. For P14's paired comparison over the 11 real sets at
      2 reps, rep noise alone gives a 95% half-width of about ±0.04.
    - That rests on the one set that moved, so it is a weak estimate. eval-audit.md's worst case,
      1/√(n·R) = 1/√22, is ±0.21.
    - Headroom: `invented` is already 0, so P14 can only show that another model invents more.
      Coverage has 0.89 of headroom. Accuracy rests on 2 cases and 9 answered values, so P14
      should give it pooled per field beside the per-case mean.
    - Against that floor, a change worth acting on would be any invented field on a real set, or a
      coverage gain of about 0.10 with invented still 0. Both clear the measured ±0.04 but not the
      worst-case ±0.21. Two reps stand for P14 (the plan's default), and P14 recomputes the floor
      from its own reps. The user has not yet named the smallest change they would act on.
  - **Measured cost and wall-clock.**
    - Cost per row: mean $0.0789, median $0.0714, ranging from $0.0337 (the R9, 1 slot) to
      $0.1362 (`ping-i230` rep 1, 8 slots).
    - Tokens: 2,101 to 2,168 in (median 2,156), and 926 to 5,015 out (median 2,425). Output
      follows the set's size, about 335 + 385 per slot (a least-squares fit over the 10 rows).
    - Latency per call: 15.6 to 58.9 s (median 28.6).
    - Wall-clock: 94.2 s for the 10 calls at the default `--concurrency 4`, from the first
      request's start to the last response in the call log. That is about 9.4 s per (case, rep),
      or about 19 s per case at 2 reps.
  - **What the user is asked to sign off**, read from the rows:
    1. **The R9's loft echoes the brief.** The key's model name is `R9 460 10.5°`, and the brief
       passes it on as-is. So the one value Opus 5 answered there (loft 10.5, right) was in the
       question, and every model will likely echo it. It lifts every model's coverage equally, by
       1/3 on one of the 11 real sets (about +0.03 on the per-case mean). The proposal is to keep
       it: the golfer is shown that value, and it is right.
    2. **Two "right" lengths sit exactly on the ±0.25" edge.** `ping-i230` rep 1 answered a plain
       half-inch progression (39, 38.5, 38, 37.5, 37, 36.5, 36, 35.75), which the model called
       Ping's standard steel lengths. Against the key (39, 38⅞, 38¼, 37⅝, 37, 36½, 36, 35½), it
       scores 7 of 8 at ±0.25". The 4i is wrong by 0.375, and the 5i and PW are right at exactly
       0.25. At ±0.125" it would score 5 of 8. The proposal is to keep ±0.25", the plan's figure.
    3. Only loft, lie, length, offset and bounce are graded. The text fields a model fills (head
       type, USGA conformity, stock shaft, grip) are not graded at all. That is the plan's scope.
    4. The offline audit's three items stand: the headline rewards saying nothing, so never quote
       `invented` without coverage; a refusal is a coverage zero; and there is no lockfile.
       P12's calls stand too: zero is invented, accuracy leaves inventions out, a refusal is
       graded as all blank, and cases are averaged per case.
  - **For P14, an estimate and not a measurement.**
    - The basis: 2,150 tokens in and 335 + 385 × slots out per row, assumed the same for every
      model, priced from `llm_prices.json`. A model that thinks more costs more than this. The key
      has 14 cases and 103 slots.
    - **baseline**, Opus 5: the 9 cases the pilot did not run (72 slots) × 2 reps is about $1.73.
      That holds if the grading stands and the run resumes after the pilot's 10 rows.
    - **v1**, Opus 5.5: 14 × 2 is about $2.01.
    - **v2**, Sonnet 5.5: about $1.01.
    - **v3**, Haiku 5.5: about $0.05.
    - **Total: about $4.80.**
    - The ceiling, if every call used 3,000 tokens in and all 16,000 `max_tokens`:
      $7.47 + $9.30 + $4.65 + $0.23 = **$21.65**.
    - Wall-clock is about 94.2 s × 28/10, roughly 4.4 minutes per model at Opus 5's speed.
  - Verify: `tests/evals` and `tests/test_llm_log.py` pass (188) against the post-pilot
    `_state.json`, which now carries the harness sha. The one pytest warning is the
    `.pytest_cache` permission, not a test.
  - **For the session that resumes P13 next**: if the user signs off as-is, record the sign-off
    with the cost and wall-clock above and tick P13. If they change a tolerance or the R9's
    grading, that edits `club_spec.py`, which is under the harness sha. The pilot rows then no
    longer match the grader. There are two ways out, and the user picks one:
    - regrade the rows offline from their traces, which hold every raw answer, though the runner
      has no verb for that yet;
    - or delete `baseline/` and have the user run the pilot again with `--approve-harness`,
      for about $0.79.
- **Resumed 2026-11-11 10:24: signed off, and P13 is done.** No request was made and no file
  changed but this one.
  - **The user's sign-off (2026-11-11), recorded as P13's section gives it: yes, as the pilot
    graded.** The R9's loft 10.5 stays graded as answered, though the brief's model name carries
    it. The i230's two lengths on the ±0.25" edge stay right. Length stays ±0.25", and loft and
    lie ±0.5°, as P12 built them. So there was no grader edit, no regrade and no re-run.
  - **The pilot's measured cost and wall-clock**: $0.7886 for 10 rows (5 cases × 2 reps on
    `claude-opus-5`). That is $0.0789 per (case, rep), or **$0.158 per case at 2 reps**.
    Wall-clock was 94.2 s at `--concurrency 4`, which is 9.4 s per (case, rep), or about 19 s per
    case at 2 reps. Every number is from the call log's `usage` and timestamps, not from the
    estimate.
  - The pilot rows still match the harness. None of the three files under the sha
    (`evals/club_spec/cases.json`, `src/golf_coach/evals/club_spec.py`,
    `src/golf_coach/evals/runner.py`) changed after the 10:03 run. `baseline/` holds 10 rows and
    10 traces. `tests/evals` and `tests/test_llm_log.py` pass (188).
  - **For P14**:
    - Keep `baseline/`. Its 10 rows were graded as signed off, so the baseline run resumes at the
      (case, rep) key (`runner.py`'s module doc, "Resume") and pays only for the 9 cases the
      pilot did not run.
    - The estimate above (about $4.80 in all, a ceiling of $21.65) is **not approved**. P14 asks
      the user for it, with its formula, before the first call.
    - The user has not named the smallest coverage change they would act on. Put the ±0.04
      measured and ±0.21 worst-case floors beside P14's results, and recompute them from P14's
      own reps.
    - Report `invented` with coverage beside it, and count refusals per variant, as the audit's
      items 1 and 2 say.

### P14 (2026-11-11): prepared, BLOCKED on the estimate, approved, run

No request was made. Nothing was written to `data/processed/llm_calls.jsonl`, and no code changed.
The row stays 🔄 until the user approves the spend.

- **Prepared, at no cost.**
  - `.claude/hillclimb/club_spec/v1/`, `v2/` and `v3/` each hold a `change.md`. Its first line
    names the model (`claude-opus-5-5`, `claude-sonnet-5-5`, `claude-haiku-5-5`) in place of the
    baseline's `claude-opus-5` and says no prompt changed. The rest gives the request settings
    (`lookup.py`'s `EFFORT` `high`, `MAX_TOKENS` 16,000, adaptive thinking) and the harness sha.
    Each directory holds nothing else yet, so a report built before the runs will show three
    empty variants.
  - The harness still matches. `runner.harness_digest` computes `9b23640d87f9` over the three
    files, and that is the approved sha in `_state.json`. So **no command needs
    `--approve-harness`**. If one is refused at the harness gate, something under the sha changed
    after P13, and that change needs reading first.
  - Every variant passes `runner._prepare` and `_check_harness` offline, and every model has a
    price in `llm_prices.json`. `baseline/` holds the pilot's 10 rows (5 cases × 2 reps), so its
    run resumes at 18 of 28 (case, rep).
- **The estimate the user is asked to approve.** It is P13's, re-derived here. Each row is about
  2,150 tokens in and 335 + 385 × slots tokens out, priced from `llm_prices.json`. That fit comes
  from Opus 5's pilot rows and is assumed for every model, so a model that thinks more costs more.
  The key has 14 cases and 103 slots, and the 9 cases the pilot did not run hold 72 of them.
  - baseline (Opus 5, 9 cases × 2 reps): 38,700 in × $5/M + 61,470 out × $25/M = **$1.73**
  - v1 (Opus 5.5, 14 × 2): 60,200 in × $4/M + 88,690 out × $20/M = **$2.01**
  - v2 (Sonnet 5.5): 60,200 × $2/M + 88,690 × $10/M = **$1.01**
  - v3 (Haiku 5.5): 60,200 × $0.10/M + 88,690 × $0.50/M = **$0.05**
  - **Total about $4.80.** The ceiling, with every row at 3,000 in and all 16,000 `max_tokens`
    out, is 18 × $0.415 + 28 × ($0.332 + $0.166 + $0.0083) = **$21.65**.
  - Wall-clock is about 4.4 minutes per model at Opus 5's pace (`--concurrency 4`).
- **The four commands**, run in this order. Each one resumes after the rows it already has:
  ```
  .venv/Scripts/python.exe -m golf_coach.evals.runner --flow club_spec --model claude-opus-5 --reps 2
  .venv/Scripts/python.exe -m golf_coach.evals.runner --flow club_spec --model claude-opus-5-5 --variant v1 --reps 2
  .venv/Scripts/python.exe -m golf_coach.evals.runner --flow club_spec --model claude-sonnet-5-5 --variant v2 --reps 2
  .venv/Scripts/python.exe -m golf_coach.evals.runner --flow club_spec --model claude-haiku-5-5 --variant v3 --reps 2
  ```
- **For the session that resumes P14** (the row stays 🔄):
  - if `v1/`, `v2/` and `v3/` still hold only `change.md`, and `baseline/` still holds only the
    pilot's 10 rows, the spend is not approved and not run, so report `BLOCKED` again;
  - once the runs exist, read `errors.jsonl` in each variant and explain every line;
  - read one row per new model end to end (served model, usage, cost and latency non-zero);
  - take each model's invented rate (with coverage beside it), accuracy (per case, and pooled per
    field), coverage, made-up blank, refusals counted from `meta.failure_class`, mean latency, and
    cost per case from the rows, absolute numbers first;
  - recompute the noise floor from each model's two reps, and set it beside P13's ±0.04 measured
    and ±0.21 worst-case floors;
  - replace the estimate with the measured spend, summed from the rows' `meta.cost_usd` and the
    errors' `cost_usd`, and cross-checked against `python -m golf_coach.llm_log summary --origin eval`
    (P6's finding for P14);
  - build the report (`build-report-lite.mjs`, from this session's `claude-api` skill base) and
    check that it shows four variants.
- **Resumed 2026-11-11 10:33, after the user's yes (2 reps): run, read and reported. P14 is
  done.** The resume found only the three `change.md` files and this plan newer than the
  marker, as the BLOCKED left them, so nothing was redone. The four commands ran as given, in
  order, without `--approve-harness` (the gate passed on sha `9b23640d87f9`, and `_state.json` is
  unchanged since 10:03). No code changed. The `claude-api` skill's report builder was run from
  the 2.1.319 bundle, because this session had no Skill tool.
  - **The measured spend: $4.2143**, against the $4.80 estimate and well under the $21.65
    ceiling. By variant, for this phase's calls only: baseline $1.7137 (18 rows), v1 $1.6464,
    v2 $0.8100 and v3 $0.0442 (28 rows each). With the pilot's $0.7886, the club-spec eval has
    cost **$5.0029** in all. The rows' `meta.cost_usd` and `llm_log summary --origin eval` agree
    to the cent on every variant. The log holds 112 rows, all `ok`. The estimate ran high because
    the three newer models wrote 70,215, 68,891 and 76,350 output tokens over 28 rows, against
    the 88,690 assumed from Opus 5's fit.
  - **Errors: none.** No variant has an `errors.jsonl`. There were 0 truncated rows, 0 failed
    attempts and 0 backoff retries at `--concurrency 4`, and no 429 or 529 on any model.
  - **One row per new model, end to end** (`taylormade-r9-460`). Each has its served model, a
    `request_id` in the log, stop `end_turn`, `meta.retries` 0, and no cache tokens:
    - v1, rep 1: `claude-opus-5-5`, 2,103 in, 1,107 out, $0.030552 (2,103 × $4/M + 1,107 × $20/M),
      12.5 s.
    - v2, rep 0: `claude-sonnet-5-5`, 2,103 in, 706 out, $0.011266 (× $2/M and $10/M), 6.9 s.
    - v3, rep 0: `claude-haiku-5-5`, 2,103 in, 923 out, $0.0006718 (× $0.10/M and $0.50/M), 5.7 s.
  - **The answers were recounted from the raw traces** (eval-audit.md §5). In every variant, the
    non-null graded fields in the models' JSON (88, 24, 3 and 2) equal the values the grader
    counted as answered. So the grader dropped nothing, and the low coverage below is what the
    models wrote. Every model filled non-graded text fields (head type, conformity, shaft
    material) and left the numbers blank, with notes saying it would not guess.
  - **The results, absolute numbers first.** 14 cases × 2 reps = 28 rows per model. Coverage is
    over the 11 real sets, invented over the 8 real sets with unpublished fields plus the 3
    made-up ones, and accuracy over the cases that answered anything.

    | | Opus 5 (baseline) | Opus 5.5 (v1) | Sonnet 5.5 (v2) | Haiku 5.5 (v3) |
    |---|---|---|---|---|
    | Invented, pooled | 0 of 340 | 0 of 340 | 0 of 340 | 0 of 340 |
    | Coverage, pooled | 88 of 690 | 24 of 690 | 3 of 690 | 2 of 690 |
    | Coverage, per-case mean | 0.139 | 0.071 | 0.032 | 0.030 |
    | Accuracy, pooled | 73 of 88 (0.830) | 21 of 24 (0.875) | 3 of 3 | 2 of 2 |
    | Accuracy, per-case mean | 0.764 (5 cases) | 0.942 (3) | 1.000 (2) | 1.000 (1) |
    | Made-up blank | 220 of 220 | 220 of 220 | 220 of 220 | 220 of 220 |
    | Refusals (`meta.failure_class`) | 0 | 0 | 0 | 0 |
    | Latency, mean (median) | 35.8 s (34.3) | 19.6 s (20.4) | 14.5 s (15.6) | 9.9 s (10.2) |
    | Cost per (case, rep) | $0.0894 | $0.0588 | $0.0289 | $0.0016 |
    | Cost per case, 2 reps | $0.179 | $0.118 | $0.058 | $0.0032 |
    | Wall-clock, this run | 181 s (18 rows) | 145 s | 105 s | 73 s |

  - **What the table says.** No model invented a value, and every model left all three made-up
    sets blank. So, as P13's audit warned, the headline cannot separate them. Only coverage
    does, and accuracy on the few values a model gave.
    - **The R9's loft is most of the smaller models' coverage.** It is echoed from the brief's
      model name (P13's sign-off item 1), and it adds 1/3 ÷ 11 = 0.030 to every model's per-case
      mean. Haiku's coverage is that echo and nothing else. Sonnet's is the echo plus one P790
      loft in one rep. Without it, coverage is 0.109 (Opus 5), 0.040 (Opus 5.5), 0.001 (Sonnet)
      and 0 (Haiku).
    - **Opus 5 answers the most, and it answers wrong the most.** 15 of its 88 values are wrong.
      14 of them are lofts exactly 1° from the published loft, which is twice the ±0.5° tolerance:
      the Apex DCB 2021's 9i, PW and AW in both reps, five ZX7 Mk II lofts (4i to 8i) in one rep,
      and three P790 lofts. The 15th is the i230's 4i length (38.5 against 38⅞). Opus 5.5's 3
      wrong values are among those same ones: the P790's 5i and 6i lofts (24.5 and 27.5 against
      23.5 and 26.5) and the i230's 4i length. A loft 1° off on a whole run of irons looks like a
      neighbouring generation's table. The P790's key notes that TaylorMade sold more than one
      P790 under that name. The key was verified by the user in P11, so these stay graded wrong,
      but **the user may want to re-check the P790's 5i and 6i lofts**, the two cells where two
      models agree on the same 1° difference.
    - Sonnet 5.5 and Haiku 5.5 were right on every value they gave, but they gave almost none.
      They are the fastest and cheapest models here, and on this prompt they leave the bag page
      blank.
  - **The noise floor, recomputed from each model's two reps** (per-row SD from the paired rep
    differences, and a 95% half-width on one model's mean minus another's over 22 rows):
    - `invented`: 0 on every row of every model, so there is no spread.
    - `coverage`: Opus 5 has 2 of 11 sets differing between reps (SD 0.060, ±0.036). Opus 5.5 has
      3 of 11 (SD 0.088, ±0.052). Sonnet has 1 (SD 0.006, ±0.003). Haiku has none. P13's
      figures were ±0.04 measured and ±0.21 worst-case. Opus 5's ±0.036 confirms the first.
    - **Rep noise is not the binding limit. Which sets are in the key is.** Paired by case
      (t, 10 df), Opus 5's coverage lead is +0.069 ± 0.135 over Opus 5.5, +0.108 ± 0.150 over
      Sonnet and +0.109 ± 0.152 over Haiku. None of them clears zero. Each rests mostly on one
      set: the P790, where Opus 5 answered 27 of 36 values in each rep. The ranking (Opus 5, then
      Opus 5.5, then Sonnet and Haiku about even) is suggested by 11 sets, and not established.
      Two reps were enough. More reps would not narrow this, but more real sets would.
  - **The report**: `node "<2.1.319 bundle>/shared/evals/report/build-report-lite.mjs" .claude/hillclimb/club_spec/`
    wrote `report.html` with "4 variants, 14 cases", and `trajectory/scores.tsv` with a
    baseline, v1, v2 and v3 column. The lite report's one metric column is the headline,
    `invented`. It reads 0.000 on every case and variant, with the three every-field sets blank
    as `absent` allows. The other three metrics appear only in the runner's summary and the
    table above. **Verify is met: the report shows four variants.**
  - The runner's constant-metric warnings are explained. `invented` and `abstention` are constant
    for the reasons P13 found: no model filled an unpublished field, and every model blanked the
    made-up sets. v2's and v3's `accuracy` warning is constant because those models answered
    only 3 and 2 values, all right.
  - Nothing in Python changed, so pytest, ruff and mypy were not re-run. The files under the
    harness sha are untouched.
  - **For P20 and P21**: measured on this route at the same effort, against Opus 5's cost per
    row, Opus 5.5 cost 0.66×, Sonnet 5.5 0.32× and Haiku 5.5 0.018×. The newer models wrote 78%
    to 87% of Opus 5's output tokens, so an estimate fitted on Opus 5 ran 12% to 20% high for
    them. Latency was 0.55×, 0.41× and 0.28× of Opus 5's. `--concurrency 4` hit no rate limit
    on any model.
  - **For P22 and P23**: the club lookup runs on `settings.coaching_model` (`api/app.py:332` and
    `:335`), so this result bears on the user's `coaching_model` decision along with P21's. It
    does not settle that decision. On this prompt, the choice is between Opus 5's coverage, which
    comes with wrong lofts, and the newer models' near-total abstention. No prompt changed in M43,
    and Decision 10 leaves a prompt hill-climb out of scope.

### P15 (2026-11-11)

- **Built**: the generator `src/golf_coach/evals/coach_cases.py`
  (`python -m golf_coach.evals.coach_cases [--golf-mcp PATH]`), the 8 cases in
  `evals/coach_feedback/cases.json` (written by hand: id, tags, `session_id`/`swing_id`, `club`,
  `about`, `inputs`), and what the generator writes from it: `evals/coach_feedback/sessions/2026-07-01/1`
  to `/8` (a `manifest.json` and an `analysis.json` each) and `cases.md`. New tests are
  `tests/evals/test_coach_cases.py` (12 tests, 22 cases).
- **The home is `src/golf_coach/evals/`, not the test side**, because the conftests must import
  from it and the generator must run as `python -m` (`tests/` is not a package). Three moves, each
  verbatim or nearly so, so there is still one copy of everything:
  - `make_swing` moved, body verbatim, to `src/golf_coach/evals/synthetic_swing.py`, which
    imports only `contracts/keypoints.py` (a survivor). `tests/analysis/conftest.py` now re-exports
    it and the nine constants `tests/analysis/` imports (`test_measure.py` reaches for five that
    a one-line grep misses), and keeps the `swing` fixture. Not `ruff format`ted, to stay verbatim
    (the repo does not enforce the formatter: `feedback/coach.py` would reformat too).
  - `tests/feedback/conftest.py::_store` moved to `coach_cases.store_swing`, gaining `club=` and
    `when=` keywords (old defaults kept). It now writes `analysis.json` with `indent=2` and a
    trailing newline, so the committed cases diff readably; no test read those bytes.
  - The `golf_mcp` fixture moved to `tests/conftest.py`, so `tests/evals/` reaches it.
- **A recorder (ADR-036 §1)**: the generator imports the frozen engine (`analysis.engine`), the
  verdict (`feedback.rules.build_feedback`), the screen validator (`launch_monitor.screen`) and
  the manifest store. Nothing on the run path imports it, and the flows must not (P17: import
  `GolfMcpServer` and the paths, never `coach_cases`). One case is made in four steps:
  `make_swing` per view, then the shot through the frozen validator, then
  `analyze_swing_bundle` and `build_feedback`, then the placement and rotation rows trimmed to the
  ones the case names.
  - **The shot's review flag is the validator's**: a case gives `printed` values plus
    `read_confidence` and `read_warnings`, never `needs_review`.
  - **Placement and rotation rows are kept only where named** (`population`, `rotation`), and a
    name the engine did not record is refused. On these keypoints both families describe the
    fixture more than a swing: `make_swing`'s shoulders never move, so four of five face-on
    rotation rows read 0, and every trajectory T2 reads about 5.6–5.9, past the 90th percentile,
    with Q about 20–21. Real stored swings carry `tour_joint_distance` and all ten rotation rows,
    and mostly no face-on T2 or Q.
  - **Tags are checked, not trusted.** `derived_tags` reads them off the result and `build_bundle`
    refuses a mismatch. `clean` means something to fix and no other feature. `nothing-to-fix`
    means a full panel, all passed. `partial-alignment` is `quality.is_degraded`, so a
    synchronized pair is not partial. The check caught two of P15's own first drafts: a missing
    spin axis put a flight refusal on the flagged shot, and a short clip recorded no joint
    placement.
- **What made each feature, measured**: `followthrough_frames: 1` gives finish balance
  `too_few_frames` (2 confident hip frames, need 3). `followthrough_frames: 2` still scores.
  `hover_frames: 12` on the down-the-line view gives `top_impact` with "b: motion start was
  estimated". `handedness: null` gives `no_handedness`. A smash factor of 1.87 gives two
  validator warnings and confidence 0.53.
- **A quirk of the frozen engine and golf-mcp, not of the generator** (`no-handedness`): without
  a handedness the flight's `flight_landing_offline_yds` is refused with `no_handedness` too.
  `golf-mcp` files the reason under the checkpoints "excluded from overall_score", not under
  "simulated ball flight" (its `family` is by reason), and `build_feedback` gives it an `[info]`
  tip saying it "is not included in the score". Real swings always have a golfer, so none showed
  it. Left as served. **For P17**: a paragraph that says nothing about a ball-flight row is not
  dropping a caveat. **For P16**: tell the user when asking for approval.
- **`golf-mcp`'s release build could not be rebuilt.** `cargo build --release -p golf-mcp` failed
  with "Access is denied" removing `target/release/golf-mcp.exe`, because a `golf-mcp.exe` was
  running: Claude Code's own registration of the server (PID 15388 at the time). That binary dates
  from 2026-10-29 and predates M42's source. It was neither stopped nor renamed. A debug build
  (`cargo build -p golf-mcp`, current) was used instead. **Over all 8 cases, the stale release
  binary and the current debug one serve byte-identical briefs**, and a rerun of the generator
  writes the committed bytes exactly. **For P17 and P23**: rebuild release while no Claude Code
  session holds the server, or point `GOLF_MCP` at `target/debug/golf-mcp.exe`.
- **For P17**:
  - `golf-mcp` wants a shots directory, and the cases commit none. The generator and the test pass
    an empty temporary one (`GolfMcpServer(..., shots_dir=<empty tmp dir>)`).
  - `CASES_DIR / SESSIONS` is the sessions directory, and `load_cases()` gives the cases.
  - Each spawn prints the server's banner to stderr, and `brief_through_golf_mcp` takes `errlog=`.
  - The stored `analysis_version` is frozen Python's 16 (real Rust swings read 17). No brief line
    shows it.
- **For P16**:
  - P15 used session `2026-07-01`, swings 1 to 8. Use another session (e.g. `2026-07-02`) or
    swings 9 to 16, since `load_cases` refuses a reused swing.
  - `test_every_tag_has_a_case` checks coverage only. P16 tightens it to two per tag.
  - **Left-handed**: `make_swing`'s geometry is a right-hander's. With `handedness: "left"` and the
    clean case's inputs, `head_stays_back` reads +0.25 and misses. A left-handed case needs
    `head_sway` and `hip_sway` negative (the drifts mirrored).
  - Measured ways to vary each tag:
    - unscored: a face-on `hover_frames` gives tempo `boundary_estimated`. A shot with no
      `loft_deg` gives a flight refusal under its own heading, which is P15's draft failure turned
      into a case.
    - partial-alignment: a down-the-line `backswing_frames` of 16 to 70 against 40 face-on gives
      `top_impact` with the tempo-disagreement ("different swings") note.
    - uncalibrated-placement: `tour_trajectory_t2_dtl` with `tour_trajectory_q_dtl`, which needs a
      down-the-line view.
    - interim-pivot: the five `_dtl` rotation rows.
  - **For the user's review**: every real stored brief carries the shell's band-estimator note
    ("this swing was measured by mediapipe:heavy, but the benchmark bands were cut from…",
    `api/pipeline.py::_band_estimator_note`, `crates/lab/src/pose.rs::band_estimator_note`).
    None of P15's eight does, because the generator records the engine and not the shell. Ask
    whether the cases should carry it.
- **For P23**: `evals/coach_feedback/cases.md` adds one to `git ls-files '*.md'` once staged.
- **Verify**: `cargo build -p golf-mcp` (debug, as above). Full pytest: 2361 passed, with the one
  warning being paddle's ccache notice, which predates M43. ruff and mypy are clean. Done when is met:
  the stale release `golf-mcp` serves all 8 cases (the default suite ran on it), and so does the
  debug build (`GOLF_MCP=target/debug/golf-mcp.exe`, 169 passed over `tests/evals/test_coach_cases.py`,
  `tests/feedback` and `tests/analysis/test_conftest.py`). A regeneration leaves no diff.

### P16 (2026-11-11): built, BLOCKED on the user's approval, then approved

No request was made and nothing was written under `.claude/hillclimb/` or to the call log. The row
stays 🔄 until the user approves the inputs (the plan's Done when).

- **Built**: 8 more cases in `evals/coach_feedback/cases.json`, swings `2026-07-01/9` to `/16`. They
  are in P15's session rather than a new one, so `STORED_AT` (2026-07-01) stays true of every case.
  `python -m golf_coach.evals.coach_cases` wrote their sessions and the 16-case `cases.md`. Adding
  them left P15's eight byte for byte as they were: their sessions, and their briefs in `cases.md`.
  Only the "8 cases." line changed. A rerun of the generator leaves no diff.

  | Case | Tag | Club | Hand | Views | What it misses, and what makes the tag |
  |---|---|---|---|---|---|
  | `clean-left-driver` | clean | driver | left | face-on | head_sway 0.5 and head_stays_back **+0.25**: the head drifts toward the target, past the hips |
  | `unscored-no-loft` | unscored | 4h | right | both | tempo 2.62, hip shift 0.23; no spin printed and no loft, so `flight_carry_yds: no_club_loft` |
  | `uncalibrated-placement-dtl` | uncalibrated-placement | 3w | right | both | tempo 2.62, hip shift 0.23; `tour_trajectory_t2_dtl` 5.4 and `_q_dtl` 19.52 kept |
  | `interim-pivot-dtl` | interim-pivot | gw | left | both | hip_sway **0.11**, below its band (two-sided); the five `_dtl` rotation rows kept |
  | `flagged-shot-distance` | flagged-shot | 6i | right | face-on | hip shift 0.26; total 135 against carry 166 + roll 9, confidence 0.91 to 0.66 |
  | `partial-alignment-tempo` | partial-alignment | sw | right | both | tempo 2.67; tempo ratios 2.67 against 4.44, so `top_impact` and "DIFFERENT swings" |
  | `no-handedness-driver` | no-handedness | driver | none | both | tempo **4.94, too slow**; head_stays_back and the flight's offline unscored |
  | `nothing-to-fix-left` | nothing-to-fix | 9i | left | face-on | nothing: all six pass, head_stays_back -0.16 |

- **Tests** (`tests/evals/test_coach_cases.py`, now 14 tests, 32 cases): `test_every_tag_has_a_case`
  became `test_every_tag_has_two_cases` (exactly two per tag, the plan's Decision 9). Two new
  tests: `test_the_two_cases_of_a_tag_differ_in_club_and_inputs`, and
  `test_the_cases_cover_both_hands_and_an_unnamed_golfer`. `test_the_briefs_carry_what_each_tag_is_about`
  now names a line for all 16 cases. Both partial-alignment cases are pinned to the note that
  degraded them, and both flagged shots to the warning that flagged them.
- **Two of P15's measured ways, as they turned out**:
  - **A missing `loft_deg` refuses nothing when spin is printed.** Loft only chooses between two
    spins *inferred* from carry (`analysis/flight_infer.py`, `_two_branches` and
    `_between_plateaus`), so HD Golf's printed spin never reads it. `unscored-no-loft` therefore
    prints no spin, which the reader records as `Spin: no value text under the label`. That
    warning alone does not flag the shot (`needs_review: no`): only a failed cross-check or a
    confidence under 0.6 does. The refusal shows under the brief's "simulated ball flight"
    heading, and the brief's own `unscored:` line still says "none - every checkpoint was
    scored".
  - **A down-the-line backswing of 64 against 40 face-on still aligns `full`.** Measured with the
    face-on view at 40/12: 16, 20 and 24 trip the tempo note, 30, 50 and 64 do not, and 70 and 90
    do. P15's "16 to 70" was the two ends that trip. This case uses 72, which reads 4.44.
- **Measured while varying the faults**:
  - **No case misses `finish_balance` except P15's unscored one.** A `finish_drift` of 0.06 or 0.08
    on a 60-frame follow-through still passes.
  - A slow tempo needs a long backswing: 60/12 reads 3.69 and passes, 75/12 passes, and 80/12 reads
    4.94 and misses.
  - With no drift on the down-the-line view, all five `_dtl` rotation rows read exactly 0. A
    down-the-line `hip_sway` of 0.02 makes `pivot_hip_axis_drift_norm_dtl` read 0.12. Nothing is
    scored from that view, so it moves no checkpoint. The three shoulder rows read 0, and
    `pivot_hip_path_jitter_norm_dtl` (0.0002) prints as 0.
- **For P17**:
  - **A positive `head_stays_back` is the fault in `clean-left-driver`**, not a frame of reference
    to explain away: +0.25 against a band of -0.67 to -0.14, MISS. The left-handed geometry is
    mirrored into the right-handed frame (`nothing-to-fix-left` reads -0.16, PASS). So a paragraph
    that calls that head position a fault agrees with the verdict.
  - **`interim-pivot-dtl` misses `hip_sway` on its low side** (0.11 against 0.14 to 0.5). A
    paragraph that praises the small hip travel contradicts the verdict. This is the set's one case
    of "less is not better".
  - `unscored-no-loft`'s brief says "unscored: none" and refuses `flight_carry_yds` under the
    flight heading. A paragraph that says every *checkpoint* was scored is right. The refusal is
    `no_club_loft`, which is not a capture problem, so advice to re-film or re-photograph is wrong
    there.
  - `no-handedness-driver` repeats P15's quirk: `flight_landing_offline_yds` is refused with
    `no_handedness`, and the brief lists it under the checkpoints excluded from overall_score.
- **For the user's review** (the BLOCKED asks): approve the 16 inputs in
  `evals/coach_feedback/cases.md`; decide P15's question about the band-estimator note; and note
  P15's no-handedness flight quirk, which both no-handedness cases show. **The band-estimator
  note** (`api/pipeline.py::_band_estimator_note`) is the shell's: it names the pose estimator
  that measured the swing, and no estimator measured these swings, so it would be false of every
  case. The generator would also have to reach the frozen server's pipeline for it, or keep a
  second copy of its text. ADR-036 §1's recorder clause lists neither. So the recommendation is to
  leave it out, knowing that the model then never coaches from a brief that carries it.
- **For P23**: no new markdown file. `cases.md` grew but is still the one P15 counted.
- **Verify**: `golf-mcp`'s release binary is still held by Claude Code's server (PID 15388), as P15
  found. So the cases were generated through the debug build, which is current
  (`GOLF_MCP=target/debug/golf-mcp.exe`). On that build `tests/evals/test_coach_cases.py` gives 32
  passed. The full suite ran on the stale release binary and gives 2371 passed. Its one warning is
  the same one P15 saw. So both binaries serve all 16 cases, with briefs byte-identical to
  `cases.md`. ruff and mypy are clean. **The user's approval is the half of Done when that is
  still open.**
- **Resumed and closed (2026-11-11)**: the user answered the BLOCKED **yes** (recorded in P16's
  section): all 16 cases in `evals/coach_feedback/cases.md` are approved as written. The
  band-estimator note stays out, because no pose estimator measured these swings and the note
  would be false of every case. The no-handedness flight quirk is accepted as served, in both
  no-handedness cases. No case changed. The resumed session wrote nothing but this plan: what
  changed after the marker was P16's own work (the 8 new sessions, `cases.json`, `cases.md`,
  the test file), and P15's eight sessions only by mtime. A fresh regeneration through the debug
  `golf-mcp` diffs empty against the tree. `tests/evals/test_coach_cases.py` gives 32 passed on
  the debug build and 32 on the default (stale release) one.
- **For P17**: the inputs are approved, so P17's precondition holds. **No brief carries the
  band-estimator note**, so the "every caveat in the brief is carried" check must take its caveats
  from each case's served brief and never from a fixed list that includes that note. A real stored
  brief carries it, so a check that only passes on these cases has not been tested against one.

### P17 (2026-11-11)

No request was made. Nothing was written under `.claude/hillclimb/coach_feedback/` but
`_state.json`, and nothing to the call log (its last line is P14's, 10:44).

- **Built**: `src/golf_coach/evals/coach_feedback.py` (`FLOW`, `CoachFeedbackFlow`, the four checks
  and `grade_paragraph`), `tests/evals/test_coach_feedback.py` (36 tests, 55 cases), and
  `.claude/hillclimb/coach_feedback/_state.json` (the metrics, and `harness_paths`
  `evals/coach_feedback/cases.json` and `cases.md`; no `harness_sha`, which is the user's). The
  command is `python -m golf_coach.evals.runner --flow coach_feedback --model M`. The runner is
  untouched. The `claude-api` skill was not invoked: this session had no Skill tool, and P17 builds
  no request (it reads the route's answer and the recorded request's `system` and `messages`).
- **Metrics**, all binary: `all_checks` (the headline: a paragraph was served and passes all four),
  then `numbers_in_brief`, `caveats_carried`, `nothing_unmeasured` and `verdict_kept`. The four are
  `always=False` and absent only on a **refusal**, which is a graded row with `all_checks` 0 (there
  is no paragraph to check, and a stand-in would move their means: P12's rule, and the runner's
  mechanism). That is a second departure from build-eval.md's "every id on every grade", taken by
  analogy with the user's P12 call. **For P20**: put it to the user at the sign-off, beside the
  alternative (all five 0 on a refusal). No text is `genuine_failure`; truncation is never graded
  (the runner's, P7).
- **The inputs are the approved ones.** Before the paid request, `run` reads the swing through
  `golf-mcp` (`read_swing`), renders it with `brief_from_view`, and raises `harness_error` with no
  request made unless that is the case's brief in `cases.md` (parsed by `approved_briefs`). After the
  request it checks the brief the route sent the same way. So `cases.md` stands in for the sessions
  and the Rust side in the harness sha. **For P22 and any hillclimb round**: a change to
  `brief_from_view`'s rendering stops every attempt until `cases.md` is regenerated and approved
  again, which is deliberate (the user approved the briefs as written).
- **`golf-mcp` from the runner's threads is safe** (P7's question): each read starts its own server,
  and a test reads eight cases on four threads, all byte-identical to `cases.md`, on the debug build
  and on the stale release one. Each attempt reads twice (the guard, then the route), each server's
  stderr to a scratch file quoted only when a read fails. `GOLF_MCP` is read at each run, not at
  import, so the `runpy` test (P13's finding) sets it.
- **What the checks are, and are not.** They measure faithfulness to the brief, not usefulness: a
  paragraph that says nothing passes all four, and the runner test pins that. Each fails only on a
  violation it can name, and its explanation quotes the number, words or sentence. Patterns come
  from the case where it says the thing: checkpoint names and the lead of each rule-based reading,
  the shot block's labels, the not-measured list and the signed checkpoint read off the **sent**
  system prompt (`('spine angle', 'hip rotation', 'swing plane', 'club path')`, `head_stays_back`;
  a prompt without the list is a `harness_error`). The caveats come from the served view, never a
  fixed list, so a real stored brief's band-estimator note asks nothing (pinned, P16's finding).
  `club path` is exempt from the unmeasured check where the brief prints it, because there it is the
  launch monitor's number.
- **Known limits, for P20's grading review** (measured on hand-written paragraphs; ten in Claude's
  style over ten cases all pass, and one inventing "9-iron carried 132 yards" fails on 132):
  - a whole number below ten nearly always rounds from something in a brief (the session date holds
    a 7), so an invented "7-iron" passes;
  - a worked number fails: a difference (0.07 for 0.28 - 0.21) or a conversion (0.15 s for 150 ms);
  - a paraphrase the patterns do not know escapes (recall is the judge's): "your hips move sideways"
    names no checkpoint, so the verdict check does not read that sentence;
  - the verdict check reads only a sentence naming exactly one checkpoint, with no negation, and with
    praise and no fault word or the reverse.
- **For P18**: the judge's rubric carries what P17 left out on purpose. A code check that the
  paragraph names the top fault was weighed and dropped: paraphrase would false-fail the primary
  metric. So "prioritises the top fault" and "actionable" are the judge's. Append the judge's metrics
  after the five in `METRICS` and in `_state.json`, and add `src/golf_coach/evals/coach_judge.py` to
  `harness_paths`. The judge reads `Coached.brief` and `Coached.text` and never `ctx.model`. Do not
  call it on a refusal (`text is None`). `grade` returns `grade_paragraph`'s `Grade` now, so P18
  merges the two.
- **Found, outside M43's scope**: the brief does not carry an unscored entry's `refilming_helps` or
  its reason, only its `why`. The prompt says "the `refilming_helps` flag says which is which; do not
  infer it from the checkpoint name", so the model has to infer it from the sentence. The check reads
  the flag off the view. Closing the gap is a route change (`brief_from_view`), which M43 rules out.
  Recorded for the user, not addressed to a phase.
- **For P20**: the pilot's first run needs `--approve-harness`, which is the user's (P18 changes the
  sha again first). The stale release `golf-mcp` (2026-10-29) is safe to run on, because the flow
  refuses any brief that differs from the approved one.
- **For P23**: one new test file and no new markdown file.
- **Verify**: `cargo build --release -p golf-mcp` was not attempted: the release binary is still held
  by Claude Code's server (P15). The flow's tests ran on the debug build
  (`GOLF_MCP=target/debug/golf-mcp.exe`, 55 passed) and, in the full suite, on the stale release one.
  Full pytest, first run: 2425 passed and 1 failed. The failure was
  `tests/test_conformance.py::test_every_on_disk_artifact_has_a_schema`, which scrapes bare
  `"….json"` constants from `src/` (P7's finding): the flow's `CASES_FILE = "cases.json"`. It is
  now `Path("cases.json")`, as `coach_cases.CASES` and `club_spec.KEY` already build theirs, because
  an eval's input is read by Python alone. After that change, `tests/test_conformance.py` and both
  coach test files pass, which makes 2426 of 2426 (P16's 2371 plus these 55). The one warning is
  paddle's, as in P15. ruff (`src tests scripts`), `ruff format --check` on the two new Python
  files, and mypy (122 files) are clean. No `feedback/` file changed, so the conformance check was
  not needed.

### P18 (2026-11-11)

No request was made. Nothing was written under `.claude/hillclimb/coach_feedback/` but
`_state.json`, and nothing to the call log.

- **Built**: `src/golf_coach/evals/coach_judge.py` (`JUDGE_MODEL`, `EFFORT`, `MAX_TOKENS`,
  `CRITERIA`, `METRICS`, `SYSTEM_PROMPT`, `verdict_schema`, `judge_request`, `judge`, `read_verdict`,
  `verdict_grade`), and `tests/evals/test_coach_judge.py` (11 tests, 26 cases). Outside the Touches
  line, as P17's finding asked: `coach_feedback.py` now has `CODE_METRICS` (the five), `METRICS =
  CODE_METRICS + coach_judge.METRICS`, `judge_model = coach_judge.JUDGE_MODEL`, and a `grade` that
  merges the checks with the judge. `tests/evals/test_coach_feedback.py` gained 2 tests and had 7
  updated for the judge (its fake `_Coach` answers the judge's model with a verdict, kept in
  `judged`). `_state.json` gained the five judge metrics and the harness path
  `src/golf_coach/evals/coach_judge.py`. The `claude-api` skill was read from the 2.1.319 bundle
  (no Skill tool here): Opus 5.5's migration section, `python/claude-api/tool-use.md`'s structured
  outputs, eval-audit.md §4 and build-eval.md on judges.
- **The request**: `claude-opus-5-5`, `thinking={"type": "adaptive"}`, `output_config={"effort":
  "high", "format": {"type": "json_schema", ...}}`, `max_tokens=16000` (the thinking counts toward
  it), `system` the rubric, and one user turn of two text blocks. The first is the coach's
  instructions in `<coach_instructions>`, with the cache breakpoint. The second is `<brief>`,
  `<paragraph>` and the ask. Exactly those keys: **no `fallbacks`**, no betas, no tools (pinned).
- **The user is told, as Decision 8 requires: the judge sends no `fallbacks`.** The skill's Opus 5.5
  guidance is to "ship the opt-in from day one". ADR-036 §8 declines that, because a fallback would
  swap the judge mid-run. So a judge refusal is an `errors.jsonl` `refusal` line, and a resume
  grades that case again. The cost is that a classifier false positive on a coaching paragraph
  (Opus 5.5 adds the `bio` classifier) costs that row until it is re-run.
- **The rubric**: four binary criteria, each a checkable claim with an explanation that quotes the
  paragraph and names the brief's line: `accurate`, `actionable`, `top_fault` (the brief's
  `headline:`, or "nothing needs fixing" when the headline says every checkpoint is inside the tour
  range) and `no_invented_claims`. `judge_all` is 1 when all four pass, worked out in code. The
  columns are `judge_all`, `judge_accurate`, `judge_actionable`, `judge_top_fault` and
  `judge_no_invented_claims`, after the five code columns, so the headline stays `all_checks`. All
  are `binary`, not SCHEMA.md's `judge` kind, whose default scale is 10.
- **Choices the plan left open, for the user at P20's sign-off:**
  1. **The judge is sent the coach's system prompt too**, beyond the plan's "brief and paragraph".
     The rubric asks for the brief read by the coach's own rules (unscored is not failed, a
     placement is not a fault, less is not better on a two-sided checkpoint). The prompt is read
     off the recorded request, so it is what the coach saw. It names no model (checked), and it is
     the cached part (about 2,000 tokens, with the rubric about 900 more).
  2. **A non-answer fails every criterion**: an empty paragraph, a decline, "I don't know", or one
     plainly about another swing. Otherwise saying nothing would pass `no_invented_claims`, and a
     degenerate policy would score (eval-audit.md §4). P19's six inputs test exactly this.
  3. **The judge's five are `always=False`**: absent on the app's refusal, where the judge is not
     called, by the same rule as P17's four. Put it to the user beside P17's question.
  4. **Only an answer that ended its turn is read.** `refusal` is `AttemptFailed("refusal")`.
     Anything else that is not the schema's shape is `harness_error`: another stop reason
     (`max_tokens` named), no text, not JSON, a missing or extra key, a verdict that is not a bool
     (`"true"` and `1` are refused), or an empty explanation. An SDK error is raised unchanged, so
     the runner backs off on the judge alone (P8).
  5. **Not graded**: the format rules in the coach's prompt (one paragraph, no markdown, at most two
     things to try). Left to P20's review.
- **Blind**: `judge` takes no model under test, and `grade` never reads `ctx.model`. A test grades
  one paragraph under `claude-opus-5` and `claude-haiku-5-5` and gets byte-identical judge requests
  with no model name outside `model`. A tag name inside the data has its `<` escaped (`&lt;`), so a
  paragraph cannot close its own fence (pinned).
- **The judge writes no call-log line**, the "leave the judge's cost to the runner's own rows"
  choice that P2 offered, so ADR-036 §2's three routes and `llm_log.Route` stay as they are. Its
  spend is on the rows (`judge_model`, `judge_usage`, `meta.judge_cost_usd`, inside
  `meta.cost_usd`) and on error lines. **For P21**: `llm_log summary --origin eval` leaves out a coach
  run's judge spend. Cross-check the log against `meta.cost_usd - meta.judge_cost_usd`, not
  `meta.cost_usd`. **For P22**: say so where the docs name the log. The Exit's "every Claude request
  writes a log line" holds for the three routes, and the judge's requests are the runner's to
  measure.
- **For P19**: `coach_judge.judge(client, instructions=, brief=, paragraph=)` makes one request and
  returns the five scores. `judge_request` and `read_verdict` are the two halves, if P19 wants the
  raw response. "All six must fail" means `judge_all` is 0. Each input should also fail every
  criterion under choice 2, so report per criterion. Pass the instructions the route really sends,
  which is golf-mcp's `coaching_system_prompt` and is what `Coached.system` holds. Frozen
  `coach.SYSTEM_PROMPT` is the in-process copy, and P17 checked only that its unmeasured list
  matches. The runner refuses a flow that made no app request (`_measured`). So if P19 writes its
  six rows through the runner, its flow has to send `judge_request(...)` through `ctx.client` with
  `--model claude-opus-5-5`, and it needs its own `_state.json` and the user's
  `--approve-harness`. A small script with `RecordingClient` and `runner._spend` avoids both. Cost
  per judge call, unmeasured until P19: about 2,900 cached input tokens plus about 1,500 uncached
  (brief and paragraph) at $4/MTok, and the output at $20/MTok, mostly thinking at `high`.
- **For P20**: the coach harness changed: `coach_feedback.py`, and the new harness path
  `coach_judge.py`. No `harness_sha` was ever recorded, so the user's first `--approve-harness`
  covers it. The release `golf-mcp` is still the stale 2026-10-29 one. The full suite ran on it,
  and the flow's tests also ran on the debug build (57 passed).
- **For P23**: one more new test file (`tests/evals/test_coach_judge.py`), and no new markdown
  file.
- **Verify**: full pytest 2454 passed (P17's 2426, plus 2 coach-flow tests and the judge's 26). The
  one warning is paddle's. ruff (`src tests scripts`) is clean, `ruff format --check` is clean on
  the four touched Python files, and mypy is clean (123 files). `ruff format --check` on all of
  `src/golf_coach/evals/` flags `synthetic_swing.py` (P15's), which was left alone. No `feedback/`
  file changed, so the conformance check was not needed.

### P19 (2026-11-11): prepared, BLOCKED on the estimate, approved, run

No request was made. Nothing was written to `data/processed/llm_calls.jsonl` or under
`.claude/hillclimb/`. The row stays 🔄 until the user approves the spend.

- **Built, at no cost**: `src/golf_coach/evals/judge_sanity.py` and
  `tests/evals/test_judge_sanity.py` (13 tests, all offline but one that reads through `golf-mcp`
  and skips without it). It is a module in `golf_coach.evals`, not a scratchpad script, so the
  session that resumes P19 runs the same code this one tested. It is not a runner flow (P18's
  finding): it sends `coach_judge.judge` through the runner's `RecordingClient` and measures each
  answer with `runner._measured`, so a substituted model is an error line and a zero usage, cost
  or latency stops the run. The command:
  ```
  .venv/Scripts/python.exe -m golf_coach.evals.judge_sanity
  ```
  `--reps 2` is the default, and `--dry-run` reads the inputs and builds the six requests without
  sending any. The requests go one at a time, so the first writes the cache and the other five
  read it. It exits 0 when all six rows exist and each failed, 1 when the judge passed a probe, and
  2 when a row is missing (an error line, which holds no slot, so a rerun pays only for that probe).
- **The inputs.** Every probe is judged against `clean-swing`'s brief, which is all measured and
  has a clear headline (hip shift at top). The brief is read through `golf-mcp` and refused, with
  no request made, unless it is byte-identical to the approved one in `cases.md`. The coach's
  instructions are the route's: the two caveat sentences read off `golf-mcp`, joined by
  `coaching_system_prompt`, as `coach_through_golf_mcp` joins them. The frozen
  `coach.SYSTEM_PROMPT` is not used (P18's finding). The dry run on the stale release `golf-mcp`
  read both, and the brief matched.
- **The three probes**: `""`, `"I don't know."`, and `OTHER_SWING_PARAGRAPH`. The third is written
  by hand from `clean-left-driver`'s approved brief, in the coach's format (one paragraph, two
  things to try), and is confident throughout: 228 yards of carry, 143 mph, praised tempo, the head
  drifting forward as the top fault. A test pins that `check_numbers` finds all its numbers in that
  brief and not in `clean-swing`'s (228 and 143 are not there). Its suggestions promise nothing,
  so read on its own it could pass `actionable` and `no_invented_claims`. It fails those two only
  if the judge applies the rubric's "plainly about a different swing" rule, which is the point.
- **Each row records both readings**: `failed` (`judge_all` is 0, the plan's Done when) and
  `failed_every_criterion` (P18's second choice). The summary prints all five scores per row. Rows
  go to `.claude/hillclimb/coach_feedback/_judge_sanity/results.jsonl`, error lines to
  `errors.jsonl` beside it. `build-report-lite.mjs` skips a directory whose name starts with `_`,
  so the report never reads it as a variant. Each row carries `judge_model`, `judge_usage`,
  `cost_usd`, `latency_s`, `stop_reason`, `request_id`, the grade and every explanation. The judge
  writes no call-log line (P18), so these rows are the whole record of the spend.
- **Not in `harness_paths`.** The module is the judge's test, not part of what grades a case, so
  it does not move the coach harness sha. **For P20, P21 and any hillclimb round**: a change to
  `coach_judge.py`'s rubric or request makes this run stale. Run it again before the judge grades
  anything after such a change.
- **The estimate the user is asked to approve: about $0.25 for the six calls, with a ceiling of
  $2.06.** No judge request has been measured yet, so it is worked from the request's size:
  - Before the cache breakpoint: the judge's rubric (622 words) and the coach's instructions
    (1,334 words), 11,808 characters. That is about 2,800 tokens at 1.4 a word, plus the schema,
    so call it 3,100. After the breakpoint: the brief (838 words, dense with numbers, about 1,600
    tokens at 1.9 a word) and the paragraph (0, 4 or 126 words), so 1,600 to 1,800.
  - Output: the four explanations are about 300 tokens. Adaptive thinking at `high` is the
    unknown, so take 1,500 tokens a call in all. The club-spec rows at `high` on Opus 5.5 wrote
    774 to 1,107.
  - Priced from `llm_prices.json` (Opus 5.5: $4 in, $5 a 5-minute cache write, $0.20 a cache read,
    $20 out, per million). The first call is 3,100 × $5 + 1,700 × $4 + 1,500 × $20 = $0.052. Each
    of the other five reads the cache: 3,100 × $0.20 + 1,700 × $4 + 1,500 × $20 = $0.037. That is
    **$0.24 in all**, and $0.31 if the cache never hits. Every 1,000 thinking tokens more a call
    adds $0.12 over the six.
  - **The ceiling**, with every call writing the cache and running to all 16,000 `max_tokens`:
    6 × (3,100 × $5 + 1,800 × $4 + 16,000 × $20) per million = **$2.06**.
  - Wall-clock: six requests one at a time, likely two to four minutes.
- **For the session that resumes P19** (the row stays 🔄):
  - if `.claude/hillclimb/coach_feedback/_judge_sanity/` does not exist, the spend is not approved
    and not run, so report `BLOCKED` again;
  - once the user says yes, run the command above. It needs no `--approve-harness` and no
    `golf-mcp` rebuild, because the stale release binary serves the approved brief;
  - read every `errors.jsonl` line and explain it. A `refusal` is a judge that declined a
    coaching paragraph (no `fallbacks`, P18). Rerun for the missing probe only, and say so;
  - read all six rows end to end: served model, `request_id`, usage with cache tokens (the first
    should write and the rest read; if none reads, the prefix did not cache, which is a finding
    for P20's estimate), cost and latency above zero, and each explanation. Check that each one
    names the reason, not just the verdict;
  - report `judge_all` and every criterion per row. **If any row passes `judge_all` (exit 1), stop
    and report `BLOCKED`**: the rubric is P18's code, and changing it moves the coach harness. If
    a row fails `judge_all` but passes a criterion (most likely `actionable` or
    `no_invented_claims` on `other_swing`), the plan's Done when holds, but P18's second choice
    does not. Report it to the user for P20's sign-off and do not change the rubric;
  - replace the estimate with the measured spend, summed from the rows' and errors' `cost_usd`, and
    give **the measured cost per judge call, For P20**, split into cache-writing and cache-reading
    calls, because P20's pilot calls the judge once per paragraph.
- **For P22**: name `python -m golf_coach.evals.judge_sanity` where the docs name the coach eval,
  as the check to rerun after any change to the judge.
- **For P23**: one more new test file (`tests/evals/test_judge_sanity.py`), one new module, no new
  markdown file, and `.claude/hillclimb/coach_feedback/_judge_sanity/` among the run files whose
  commit the user decides (ADR-036 §9).
- **Verify**: full pytest 2467 passed (P18's 2454 plus these 13) in 9 min 21 s. Of the 2 warnings,
  one is paddle's and the other is pytest's own: it could not write `.pytest_cache` (access denied
  in this session's sandbox), which touches no test. ruff (`src tests scripts`) is clean, `ruff
  format --check` is clean on the two new files, and mypy is clean (124 files). The dry run made no
  request and wrote nothing. A fake-client run into a scratch directory printed the summary table
  and exited 0. No `feedback/` file changed, so the conformance check was not needed.
  `.claude/hillclimb/coach_feedback/` still holds only `_state.json`, and the call log's last line
  is still P14's (10:44).
- **Resumed 2026-11-11 14:04, after the user's yes: run once, read, and P19 is done.** Nothing in
  the tree had changed since the marker except the two files above and this plan, so no code was
  touched and nothing needed redoing. `python -m golf_coach.evals.judge_sanity` ran once with no
  flags (credential `GOLF_ANTHROPIC_API_KEY`). It made exactly six requests and **exited 0**.
  - **All six fail `judge_all`, and all six fail every criterion**, so both the plan's Done when
    and P18's second choice hold. The 6 rows are in
    `.claude/hillclimb/coach_feedback/_judge_sanity/results.jsonl`. There is no `errors.jsonl`:
    there was no refusal and no error. The call log is untouched (its last line is still P14's),
    as P18 said it would be.
  - **Every explanation names its reason**, quoting the paragraph and the brief's line. The empty
    and "I don't know." rows cite the rubric's non-answer rule, and name the headline (hip shift at
    top) that the paragraph leaves out. Both `other_swing` rows catch every planted contradiction:
    228 yd and 143 mph against the brief's 151 yd and 112 mph, tempo "2.9 to 1" against 2.67
    MISS, head sway 0.50 against 0.13 PASS, and a fault in `head_stays_back`, which passed. Both
    fail `actionable` and `no_invented_claims` by the rule "plainly about a different swing", as
    intended, and not by a fault found in the suggestion itself.
  - **Each row was read end to end.** The served model was `claude-opus-5-5` on all six. Each has a
    `request_id`, `stop_reason` `end_turn`, and cost and latency above zero. `judged_at` is UTC
    (20:05Z is 14:05 local).

    | probe, rep | input | cache write | cache read | output | cost | latency |
    |---|---|---|---|---|---|---|
    | empty 0 | 2,130 | 4,621 | 0 | 309 | $0.0378 | 4.6 s |
    | empty 1 | 2,130 | 0 | 4,621 | 261 | $0.0147 | 2.6 s |
    | dont_know 0 | 2,136 | 0 | 4,621 | 317 | $0.0158 | 6.3 s |
    | dont_know 1 | 2,136 | 0 | 4,621 | 312 | $0.0157 | 4.7 s |
    | other_swing 0 | 2,347 | 0 | 4,621 | 783 | $0.0260 | 8.0 s |
    | other_swing 1 | 2,347 | 0 | 4,621 | 796 | $0.0262 | 7.3 s |

  - **The measured spend: $0.1362**, $0.0227 a call, against the $0.24 estimate and the $2.06
    ceiling. The run took about 30 s of wall-clock, not the two to four minutes estimated. The
    prefix **does cache**: the first call wrote 4,621 tokens and each of the other five read all
    of them. Two of the estimate's guesses were wrong, in opposite directions:
    - **Input was larger.** The cached prefix (rubric, coach's instructions and schema) is 4,621
      tokens, not 3,100: about 2.4 tokens a word, not 1.4. The uncached part (clean-swing's
      brief plus the ask) is 2,130 tokens with an empty paragraph, and a 126-word paragraph adds
      about 217.
    - **Output was much smaller.** Thinking at `high` stayed short: 261 to 317 output tokens on a
      non-answer and about 790 on a real paragraph, against the 1,500 assumed.
  - **For P20, the measured cost per judge call**, priced from `llm_prices.json`. A coach
    paragraph is about the length of `other_swing`'s, so use its row:
    - **cache-reading call: about $0.026** (as measured on `other_swing`);
    - **cache-writing call: about $0.048** (the same row, with the 4,621-token prefix written at
      $5/MTok instead of read at $0.20/MTok).

    The prefix is the same for every case, so one write per 5-minute window covers every judge
    call in it. An interleaved pilot of 10 paragraphs should pay one write and nine reads, about
    $0.28 of judge spend, if no gap between judge calls passes five minutes. Each case's brief
    is uncached and differs in length from clean-swing's 2,130 tokens. Size it at about 2.5
    tokens a word when estimating, not 1.9. Latency is 3 to 8 s per judge call, on top of the
    coach call.
  - **Verify**: the six rows are on disk, each `failed` and `failed_every_criterion`, and
    `report` exits 0. No Python changed in the resumed session, so pytest, ruff and mypy were not
    rerun. The first session's full run (2467 passed) stands.

### P20 (2026-11-11): audited offline, BLOCKED on the spend and the harness, piloted, BLOCKED on the sign-off, signed off as graded

No request was made. Nothing was written to `data/processed/llm_calls.jsonl` (its last line is
still P14's, 10:44) or under `.claude/hillclimb/coach_feedback/` (still `_state.json` and
`_judge_sanity/`). No file in the repo changed but this one. The row stays 🔄 until the user
approves the spend and the harness. The skill's files were read from the 2.1.319 bundle
(`%LOCALAPPDATA%/Temp/claude/bundled-skills/2.1.319/3ef9aac5f3dee21fb922f9bb2d87f2c0/claude-api/`),
because this session had no Skill tool.

- **eval-audit.md, the offline half.** A scratchpad script ran `runner.main` on the **real**
  `coach_feedback` flow and the stale release `golf-mcp`, `--limit 5`, on a fake client (the SDK's
  own exception classes over `httpx` responses). The call log was redirected and `BACKOFF_BASE_S`
  cut to 0.01 s. Every case landed where it should:
  - **oracle**: a paragraph true to its brief on each of the five pilot cases passed all four
    checks, `all_checks` 1.0 on 5 of 5. Two of the paragraphs are P17's, and three were written
    here for `unscored-finish`, `uncalibrated-placement` and `interim-pivot`;
  - **wrong but plausible**: the same paragraphs with one number moved (0.28 to 0.32, 140 to 150
    yards, 2.4 to 2.2, 128 to 138, 0.24 to 0.26) failed `numbers_in_brief` alone on 5 of 5. Each
    explanation names the number and its sentence;
  - **null, empty**: no text is a `genuine_failure` error line per case. It writes no row and
    carries its billed spend on the line;
  - **null, constant** ("Keep making this swing."): passes all four checks, `all_checks` 1.0 on 5
    of 5. That is P17's known limit, and it is pinned. Only the judge fails a non-answer, and P19
    showed that the real judge does, 6 of 6;
  - a 401 became a `harness_error` line. A 429 and then an answer became an `ok` row with
    `meta.retries` 1. A 529 on every try became a `serving_error` after 4 retries. `max_tokens`
    became a `truncated` row with no grade and no judge call. The coach's refusal became a graded
    row `{"all_checks": 0}` with `meta.failure_class: refusal` and no judge call;
  - on the judge's side, a refusal became an error line of class `refusal`. A 429 and then a
    verdict became an `ok` row with `meta.judge_retries` 1. An answer that is not JSON became a
    `harness_error`. A verdict failing one criterion moved `judge_all` and that criterion only. A
    coach response served by another model became `serving_substitution`.
- **What the audit turns up, worst first. Each one is for the user's sign-off, and none is a bug.**
  1. **The primary metric rewards saying nothing.** A paragraph that says nothing passes every
     code check, by design (P17), so `all_checks` cannot tell a cautious model from a useless one.
     Only `judge_all` can. **For P21**: never report `all_checks` without `judge_all` and
     `judge_actionable` beside it, per model, as P14 never reported `invented` without coverage.
  2. **The judge has no human calibration yet.** eval-audit.md §4 asks for agreement with human
     labels on a few dozen cases, about 90% on clear-cut ones. The pilot's 10 paragraphs, with the
     user's grading review, are the only human labels this eval will have in M43. Ask the user to
     mark each judge verdict they would grade differently, and record the agreement.
  3. **No lockfile**, as for the club-spec flow. The runner prints this itself. It is low severity.
- **What else the audit checked and found sound.** The flow calls the real stdio route
  (`coach_through_golf_mcp`) with the production prompt, effort (`low`), `max_tokens` (4,096) and
  adaptive thinking. The judge is blind (P18), and the code checks are deterministic. There is no
  gold to leak, because the checks read the brief the model is meant to see. **The stale release
  `golf-mcp` (2026-10-29) serves all 16 approved briefs**: a size probe ran every case through
  `flow.run`, whose guard refuses any brief that differs. `cargo build --release -p golf-mcp` was
  tried and failed again with "Access is denied" on `target/release/golf-mcp.exe`, because Claude
  Code's server holds it (P15).
- **The pilot's five cases.** `--limit 5` takes `clean-swing`, `unscored-finish`,
  `uncalibrated-placement`, `interim-pivot` and `flagged-shot`, five different tags. Every metric
  is present on a row unless the coach refuses, so no column will be blank, and P13's `--limit`
  problem does not arise. No reorder is needed.
- **The harness the user approves**: sha `df41c54a485a`, over `evals/coach_feedback/cases.json`,
  `evals/coach_feedback/cases.md`, `src/golf_coach/evals/coach_feedback.py`,
  `src/golf_coach/evals/coach_judge.py` and `src/golf_coach/evals/runner.py`. It was computed with
  `runner.harness_digest` on the committed `_state.json`, which records no sha yet.
- **The estimate the user is asked to approve: about $0.70 for the pilot (likely $0.50 to $1.30),
  with a ceiling of $5.02.** No coach request has been measured yet, so the coach half is worked
  from the request's size, and the judge half from P19's measured calls.
  - **Token sizes**, calibrated on P19's measured judge calls: 2.75 characters a token for prose
    (the judge's 12,697-character prefix was 4,621 tokens), and 2.46 for a brief (clean-swing's
    5,244-character tail was 2,130). Opus 5 and the 5.5 models count the same request within 2
    tokens (the club-spec log), so the calibration carries over. The coach's system prompt is
    8,301 characters, about 3,020 tokens, and the route caches it. The five pilot briefs are 4,751
    to 6,897 characters, about 1,930 to 2,800 tokens: 11,400 in all, or 22,800 over 2 reps.
  - **Coach on Opus 5** ($5 in, $6.25 cache write, $0.50 cache read, $25 out, per million): the
    briefs are 22,800 × $5 = $0.114. The system prompt, at up to 4 writes and 6 reads, is $0.085:
    the first wave of `--concurrency 4` goes out together, so each of its four may write. Output at
    about 600 tokens a call (the paragraph plus thinking at `low`) is $0.15. **About $0.35.**
  - **Judge on Opus 5.5** (P19's prices): each tail is the brief, the ask (about 40 tokens) and the
    paragraph (about 250), so 25,700 × $4 = $0.103. The 4,621-token prefix, at up to 4 writes and 6
    reads, is $0.098. Output at P19's 790 tokens a call is $0.158. **About $0.36.**
  - The range runs from $0.51, if each side writes its cache once and the coach writes 300 tokens a
    call, to $1.31, if every call writes its cache and both sides write 1,500 tokens.
  - **The ceiling** has every call writing its cache and running to `max_tokens`: 4,096 for the
    coach, and 16,000 for the judge, judging a 4,096-token paragraph. The coach is $0.114 + $0.189
    + $1.024 = $1.33, and the judge $0.231 + $0.257 + $3.200 = $3.69, so **$5.02**. A retried try
    is billed only when it was answered, so retries add nothing past this.
  - Wall-clock is unmeasured for the coach. With the judge's 3 to 8 s and a coach call at `low`,
    10 rows at `--concurrency 4` should take one to three minutes.
- **The command is the user's**, because `--approve-harness` is never the agent's (P8):
  ```
  .venv/Scripts/python.exe -m golf_coach.evals.runner --flow coach_feedback --model claude-opus-5 --reps 2 --limit 5 --approve-harness
  ```
  It records sha `df41c54a485a` and writes `.claude/hillclimb/coach_feedback/baseline/`. It needs
  no `GOLF_MCP`, because the default release binary serves the approved briefs. The credential is
  `.env`'s `GOLF_ANTHROPIC_API_KEY`, as in P13 and P19. If the grading holds at the sign-off, P21's
  baseline resumes after these 10 rows rather than paying for them twice.
- **For the session that resumes P20** (the row stays 🔄):
  - if `.claude/hillclimb/coach_feedback/baseline/` holds no rows, the pilot has not run, so report
    `BLOCKED` again;
  - read every `errors.jsonl` line and explain it. Read one row end to end: served `model`, `usage`
    with cache tokens (does the coach's system prompt cache, and does the judge's prefix?),
    `latency_s`, `meta.retries`, `judge_model`, `judge_usage`, `meta.judge_cost_usd`, and
    `meta.cost_usd` above zero. Cross-check the call log's 10 coach lines (origin
    `eval:coach_feedback/baseline`) against `meta.cost_usd - meta.judge_cost_usd`, not
    `meta.cost_usd` (P18);
  - explain any constant-metric warning from the rows, as P13 did, by reading each paragraph;
  - build the report: `node "<bundle>/shared/evals/report/build-report-lite.mjs" .claude/hillclimb/coach_feedback/`.
    Check that `_judge_sanity/` is not read as a variant;
  - work out the noise floor: rep-to-rep agreement per metric from the two reps, beside
    eval-audit.md's worst case for P21's 16 cases × 2 reps, 1/√32 = ±0.18;
  - show the user every row: the paragraph, and each check's and each criterion's verdict with its
    explanation. Ask which they would grade differently. Put the audit's three items above to them,
    with P17's refusal question (the four checks absent on a refusal, against all five at 0), P18's
    five choices (the judge sees the coach's prompt; a non-answer fails every criterion; the
    judge's five are `always=False`; only an `end_turn` answer is read; the format rules are not
    graded), and P17's known limits, all from P17's and P18's findings;
  - record the measured cost per case with the judge's part separate, and the wall-clock. Give
    **for P21** the estimate per model. The coach's half scales with the model; as a first guess,
    use P14's club-route ratios against Opus 5 (Opus 5.5 0.66×, Sonnet 5.5 0.32×, Haiku 5.5
    0.018×). The judge's half does not scale, because the judge is always Opus 5.5.
- **For P21**: run order moves the cache. A variant whose first wave writes the coach's prompt
  pays for that once, and the judge's prefix stays warm across variants only if no five-minute gap
  passes. Compare cache-read shares before reading a cost difference (eval-audit.md §3).
- **Verify**: no Python changed, so pytest, ruff and mypy were not rerun. The audit's six fake runs
  exited as expected: 0 on the oracle, the wrong answers and the constant null, and 1 on the empty
  null and both error sets. The call log was unchanged after them.
- **Resumed 2026-11-11 17:54, after the user's pilot: read, reported, and BLOCKED on the
  sign-off.** No request was made and no code changed. Since the marker, the only changed files
  were the pilot's own (`_state.json` with the sha, `baseline/results.jsonl`, its 10 traces) and
  this plan. This session added the report builder's `.claude/hillclimb/coach_feedback/report.html`
  and `trajectory/scores.tsv`. `runner.harness_digest` still gives `df41c54a485a` over the same
  five files, so the rows match the grader. The skill's files were read from the 2.1.319 bundle
  again, because this session had no Skill tool either. The row stays 🔄 until the user signs off.
  - **The pilot as it ran.** It wrote 10 rows and no `errors.jsonl`. The call log holds exactly 10
    coach lines with origin `eval:coach_feedback/baseline`. They have 10 distinct `request_id`s,
    all served `claude-opus-5` with outcome `ok`. Their $0.278604 equals the rows'
    `meta.cost_usd - meta.judge_cost_usd` to the last digit. Every row's `meta.cost_usd`
    recomputes from `usage` and `judge_usage` through `llm_log.cost_usd`. The run's one backoff
    retry was the judge's, on `flagged-shot` rep 0 (`meta.judge_retries` 1). The row does not
    record its class.
  - **One row end to end** (`unscored-finish` rep 1):
    - the coach: served `claude-opus-5`, 1,909 in, 410 out, a 2,886-token cache write, $0.0378,
      `latency_s` 8.28, `end_turn`, `meta.retries` 0;
    - the judge: served `claude-opus-5-5`, 2,271 in, 1,180 out, a 4,621-token cache write,
      `meta.judge_cost_usd` $0.0558, `meta.judge_retries` 0;
    - `meta.cost_usd` is $0.0936;
    - the trace holds the route's system prompt (8,301 characters), the brief and the paragraph.
      Every grade carries an explanation, and each failing one quotes the paragraph.
  - **Both prefixes cache.** The coach's system prompt is 2,886 tokens. Three of the 10 calls
    wrote it (the first wave: both `unscored-finish` reps and `clean-swing` rep 1), and seven read
    it. The judge's prefix is P19's 4,621 tokens. Two calls wrote it (`unscored-finish`'s pair,
    which reached the judge together), and eight read it.
  - **The headline, recomputed from the raw rows** (per case, then over cases), agrees with the
    runner's summary. The floor is explained in the noise-floor item below.

    | metric | mean | reps agree | headroom | rep-noise floor, 16 × 2 |
    |---|---|---|---|---|
    | `all_checks` | 0.800 | 3 of 5 | 0.20 | ±0.22 |
    | `numbers_in_brief` | 0.900 | 4 of 5 | 0.10 | ±0.16 |
    | `caveats_carried` | 1.000 | 5 of 5 | 0 | none measured |
    | `nothing_unmeasured` | 0.900 | 4 of 5 | 0.10 | ±0.16 |
    | `verdict_kept` | 1.000 | 5 of 5 | 0 | none measured |
    | `judge_all` | 0.500 | 4 of 5 | 0.50 | ±0.16 |
    | `judge_accurate` | 0.600 | 3 of 5 | 0.40 | ±0.22 |
    | `judge_actionable` | 1.000 | 5 of 5 | 0 | none measured |
    | `judge_top_fault` | 1.000 | 5 of 5 | 0 | none measured |
    | `judge_no_invented_claims` | 0.700 | 4 of 5 | 0.30 | ±0.16 |

  - **The four constant-metric warnings come from Opus 5's behaviour, not from the wiring.**
    - `caveats_carried` is 1.0. Four rows carried their caveat: both `flagged-shot` reps call the
      shot uncertain, and both `unscored-finish` reps say finish balance was not scored. On the
      other six, the paragraph touched nothing a caveat qualifies. No paragraph used a Q, a
      rotation row or a raw `head_stays_back` value. So on Opus 5, `uncalibrated-placement` and
      `interim-pivot` never tested their own caveats, because the model left the qualified thing
      alone. The check does fail a paragraph that uses one (P17's tests).
    - `verdict_kept` is 1.0. It read 0 to 3 sentences per row, and none at all on two rows
      (`interim-pivot` rep 1 and `uncalibrated-placement` rep 0). One reversal got past it:
      `uncalibrated-placement` rep 0 says the hip slide at the top "passes nothing you'd call a
      fault", although `hip_shift_at_top` missed its band. That sentence names the checkpoint by
      paraphrase and holds a negation, and both are among P17's known limits. The judge caught it
      under `accurate`.
    - `judge_actionable` and `judge_top_fault` are 1.0. Every paragraph names the headline's
      checkpoint first and gives a drill or a feel for it. P19's probes failed both criteria 6
      of 6, so the judge does fail them. Neither has headroom, so P21 can only show another model
      doing worse on them.
  - **Every failing verdict was read** (eval-audit.md §4, "spot-check the failures"). There are 9
    failing verdicts on 6 rows: 2 from the code checks and 7 from the judge.
    1. **One is a grader error.** `flagged-shot` rep 0 fails `nothing_unmeasured` on "turning so
       the hip rotates away from it rather than bumping into it". That is a drill cue, not a
       claim that hip rotation was measured. P17's `_word` deliberately reads `rotate` as a form
       of `rotation`. So the cue fails when it says "rotates", and the same cue passes when it
       says "turning" (`clean-swing` rep 1, `uncalibrated-placement` rep 0). The judge passed
       the sentence. This one verdict is half of the code checks' failures, and it is what takes
       `all_checks` from 0.9 to 0.8.
    2. **One is correct by the check's rule.** `uncalibrated-placement` rep 0 fails
       `numbers_in_brief` on "360 milliseconds back where the band wants 400 and up". The band
       starts at 408, so the rounding moves the band's edge as well. Rep 1 quoted 408 to 706.
    3. **The four judge `accurate` failures are all defensible:**
       - `interim-pivot` rep 0's "quick, repeatable tempo": one swing cannot show that a tempo
         repeats, and the prompt forbids describing a pattern;
       - `interim-pivot` rep 1 calls a tempo of 2.88 "comfortably inside" a 2.72 to 4.71 band;
       - `uncalibrated-placement` rep 0 says "passes nothing you'd call a fault" about a miss;
       - `unscored-finish` rep 0 ties the toe strike to the hip slide, joining two axes that the
         prompt keeps apart.
    4. **Two of the three judge `no_invented_claims` failures are borderline:**
       - `uncalibrated-placement` rep 0 fails on the link from tempo to the hip slide ("almost
         certainly feeding the other two small misses"). The same link passed three times, in
         both `clean-swing` reps and in `uncalibrated-placement` rep 1, where the judge wrote that
         it "connects the ranked tips as the instructions allow". The coach prompt asks for that
         link ("connect them to each other"). So the judge varies on this one reading, and the
         paragraph did not expose a gap in the rubric;
       - `unscored-finish` rep 1 fails on "there's ball-speed sitting there if you find the
         middle". The rubric rules out any promised gain in distance, speed or accuracy, so the
         verdict follows it, but a golfer would call this common knowledge;
       - `unscored-finish` rep 0's failure is sound: the toe strike link again, and "the quietest
         way ... to start drifting".

    So 1 of the 9 failing verdicts is a grader error and 2 are borderline. That is at or above
    eval-audit.md's "roughly one in ten", beyond which the guide says to fix the grader before
    a full pass. None of the passing verdicts reads as wrong. One paragraph breaks a format rule,
    which P18's choice 5 leaves ungraded: `clean-swing` rep 1 gives three things to try (a slower
    count, a trail-heel feel and a stick check), and the prompt allows two.
  - **The noise floor**, for P21's 16 cases × 2 reps. eval-audit.md's rule gives ±0.18
    (1/√32). The pilot's own rep spread gives the column in the table above:
    1.96 × SD × √(2/32), with the SD taken from the within-case variance, as in P13. Five cases
    make it a weak estimate.
    - **Put plainly: `all_checks` cannot show another model beating Opus 5.** Its headroom of
      0.20 is at the noise floor. It can show a drop of more than about 0.2. `judge_all`, with
      0.50 of headroom, can show a difference of about 0.2 in either direction. The user has not
      named the smallest change they would act on.
    - The levers, cheapest first:
      - 3 reps: ±0.14 by the rule, and about $9.7 for the run instead of $6.4;
      - 4 reps: ±0.125;
      - more cases, which means newly approved ones (P16's work);
      - a count of violations per paragraph in place of pass or fail.
  - **The measured cost and wall-clock.**
    - **Spend**: $0.7198 for the 10 rows, of which the coach was $0.2786 and the judge $0.4412
      (61%). Per (case, rep): $0.0720, the coach $0.0279 and the judge $0.0441. **Per case at 2
      reps: $0.144**, the coach $0.056 and the judge $0.088.
    - **Against the estimate of $0.70**, the two halves missed in opposite directions:
      - the coach came in under, $0.28 against $0.35, because it wrote 408 output tokens a call
        against the 600 assumed;
      - the judge came in over, $0.44 against $0.36, because it wrote 1,407 output tokens a call
        (963 to 1,925) against P19's 790. A real paragraph draws longer explanations than a probe
        does.
    - **Tokens**: the coach read 1,909 to 2,736 and wrote 378 to 444; the judge read 2,271 to
      3,113.
    - **Latency**: the coach's was 6.9 to 12.8 s (median 8.3). The judge's is not on the row,
      because `latency_s` is the coach's alone, as eval-audit.md §3 asks. File times put it at
      about 11 to 18 s a call, and 26 s for the call with the retry.
    - **Wall-clock**: about 85 s from the first coach request to the last row, at
      `--concurrency 4`. That is 8.5 s per (case, rep).
  - **Where the user reads each row**: each row's `grade` and `explanation` are in
    `.claude/hillclimb/coach_feedback/baseline/results.jsonl`. Each paragraph is the last message
    of `baseline/traces/<case>_rep<N>.json`. `report.html` shows `all_checks` per case.
  - **What the user is asked to sign off:**
    1. **The `nothing_unmeasured` false positive** (item 1 above). There are two ways:
       - keep it as graded, and have P21 read every `nothing_unmeasured` failure by hand and
         report the false positives per model beside the metric;
       - or narrow the check. That changes `coach_feedback.py`, which is under the harness sha.
         The pilot's code checks would then need a regrade from the traces, which has no runner
         verb, or a re-run for about $0.72, and the new sha a fresh `--approve-harness`.
    2. **The two borderline judge verdicts** (item 4), and any other judge verdict the user would
       grade differently. Those marks are the only human calibration the judge gets in M43 (the
       audit's item 2). Without them, no agreement is recorded.
    3. **The audit's three items**: `all_checks` rewards saying nothing, so it is never quoted
       without `judge_all` and `judge_actionable` beside it; the judge has no calibration yet; and
       there is no lockfile.
    4. **P17's refusal question**: on a refusal, the four checks are absent (as built), against
       all five scored 0. Opus 5 refused nothing here, so the choice moved no number.
    5. **P18's five choices**: the judge sees the coach's prompt; a non-answer fails every
       criterion (P19 showed that it does); the judge's five metrics are absent on a refusal;
       only an `end_turn` answer is read; and the format rules are not graded.
    6. **P17's known limits**: an invented single digit passes; a worked number fails; a
       paraphrase gets past the checks (seen here, in item 3's hip slide); and the verdict check
       reads only a sentence that names one checkpoint and holds no negation.
  - **For P21, an estimate. It is not approved.**
    - The basis, per row:
      - the coach's input is each case's brief at 2.49 characters a token. That ratio was
        measured on the five pilot briefs, and it puts the other 11 at 1,938 to 2,779 tokens;
      - the coach writes 408 tokens;
      - the coach's 2,886-token system prompt is written by the first wave of 4 and read after;
      - the judge's input is the brief plus 400 tokens, and it writes 1,407;
      - the judge's 4,621-token prefix is written twice per variant and read after;
      - everything is priced from `llm_prices.json`, with the same tokens for every model.
    - The estimate per variant:

      | variant | model | rows | coach | judge | total |
      |---|---|---|---|---|---|
      | `baseline` | Opus 5 | 22 (the 11 cases the pilot did not run) | $0.57 | $0.91 | $1.48 |
      | v1 | Opus 5.5 | 32 | $0.62 | $1.31 | $1.93 |
      | v2 | Sonnet 5.5 | 32 | $0.31 | $1.31 | $1.62 |
      | v3 | Haiku 5.5 | 32 | $0.016 | $1.31 | $1.33 |

      On P14's ratios, the coach column would read $0.53 for Opus 5.5 and $0.25 for Sonnet 5.5.
    - **The total is about $6.4** ($6.2 on P14's ratios). The judge is $4.85 of it, because it
      is always Opus 5.5.
    - **The ceiling is $51.52**, with every call writing its cache and running to `max_tokens`
      (4,096 for the coach, and 16,000 for the judge judging a 4,096-token paragraph). At 3 reps,
      the run is about $9.7 and the ceiling $79.78.
    - Wall-clock, at 8.5 s per row: about 3 minutes for the baseline and 4.5 per variant, so
      about 17 minutes in all.
    - The pilot ran Opus 5 alone, so Decision 8's self-preference reading cannot start until
      P21's variants exist. P21's latency comparison is the coach's, since the judge's is not on
      the row.
  - **For the session that resumes P20 next.** If the user signs off as graded, record the
    sign-off with the cost and wall-clock above, plus any judge verdicts they mark and the
    agreement those give, and tick P20. Keep `baseline/`, because P21's baseline resumes after
    these 10 rows. If they choose to narrow `nothing_unmeasured`, that is an edit to
    `coach_feedback.py`, and they also choose between the regrade and the re-run. P19's sanity
    run stays valid, because the judge does not change.
  - **Verify**: the report rendered as "1 variants, 5 cases", with primary metric `all_checks` at
    0.800 and no blank per-case cell. It did not read `_judge_sanity/` as a variant. The lite
    report draws only the headline column. The other nine metrics are in the runner's summary and
    the raw rows, and none of them is blank. No Python changed, so pytest, ruff and mypy were not
    rerun.
- **Resumed 2026-11-11 18:39, after the user's sign-off: recorded, and ticked.** No request was
  made and no code changed. Since the marker, the only changed files were the pilot's own, the
  report builder's `report.html` and `trajectory/scores.tsv` (17:58) and this plan (the user's
  answer, recorded at 18:31). `runner.harness_digest` still gives
  `df41c54a485ac8c5f842b591475f34f0d89168adcbb64c8b0ae7473153383301`, the `_state.json` sha, over
  the same five files. `baseline/results.jsonl` still holds its 10 rows.
  - **The sign-off, 2026-11-11: as graded.** The user signs off the coach eval's grading as the
    pilot graded it. In detail:
    - `nothing_unmeasured` is **not** narrowed. `coach_feedback.py` and harness sha
      `df41c54a485a` stand, with no regrade and no re-run. `flagged-shot` rep 0's drill-cue
      failure stays in the pilot's numbers, so the baseline's `all_checks` is 0.800 and
      `nothing_unmeasured` 0.900 with it in;
    - the user marked **no** judge verdict. So the two borderline `no_invented_claims` verdicts
      (`uncalibrated-placement` rep 0, `unscored-finish` rep 1) stand as the judge gave them.
      The user accepted all 50 of the pilot's judge verdicts (10 rows by 5 criteria) as graded.
      That is a sign-off, not an independent labelling, so it is **not** recorded as an agreement
      rate. The judge still has no human calibration in M43 (the audit's item 2), and ADR-036's
      results addendum (P22) should say so;
    - the audit's three items stand as findings, not fixes: `all_checks` is never quoted without
      `judge_all` and `judge_actionable` beside it; the judge is uncalibrated; and there is no
      lockfile;
    - P17's refusal choice (the four checks absent on a refusal) and P18's five choices stand as
      built, and so do P17's known limits.
  - **What P20 measured, for the record** (the detail is in the 17:54 item above): $0.7198 for 10
    rows, so **$0.144 per case at 2 reps**, of which the coach is $0.056 and the judge $0.088
    (61%). The coach's latency is 6.9 to 12.8 s (median 8.3), and the wall-clock about 85 s at
    `--concurrency 4`.
  - **For P21**:
    - read every `nothing_unmeasured` failure by hand, and report the false positives per model
      beside the metric. `flagged-shot` rep 0's "rotates" drill cue is the known one, and it is
      in the baseline's numbers;
    - the spend is **not** approved by this sign-off. Stop on the estimate above: about $6.4 at 2
      reps (ceiling $51.52), with the judge $4.85 of it. The user's answer chose no rep count, so
      put 3 reps (about $9.7, ceiling $79.78, ±0.14 by the rule) beside it as the one lever;
    - the baseline resumes after the pilot's 10 rows, because the runner keys resume on
      (case, rep). At 3 reps, it adds rep 2 for the five pilot cases as well. Sha `df41c54a485a`
      is already recorded, so no fresh `--approve-harness` is needed unless a harness file
      changes. The three variants are new directories, and each needs its own run;
    - never report `all_checks` without `judge_all` and `judge_actionable` beside it, per model;
    - Decision 8's self-preference reading starts with P21's v1 (Opus 5.5).
  - **Verify**: the report rendered at 17:58 as "1 variants, 5 cases", with no blank per-case
    cell, and no input to it has changed since. The harness sha recomputes. No Python changed, so
    pytest, ruff and mypy were not rerun.

### P21 (2026-11-11): prepared, BLOCKED on the estimate, approved, BLOCKED on the credit, resumed and run

No request was made. Nothing was written to `data/processed/llm_calls.jsonl`, whose last line is
still the P20 pilot's (origin `eval:coach_feedback/baseline`). No code changed, and no file under
the harness sha did. The row stays 🔄 until the user approves the spend.

- **Prepared, at no cost.**
  - `.claude/hillclimb/coach_feedback/v1/`, `v2/` and `v3/` each hold a `change.md`. Its first
    line names the model (`claude-opus-5-5`, `claude-sonnet-5-5`, `claude-haiku-5-5`) in place of
    the baseline's `claude-opus-5` and says no prompt changed. The rest gives the route
    (`coach_through_golf_mcp`, effort `low`, `max_tokens` 4,096, adaptive thinking), the judge
    (`claude-opus-5-5`, effort `high`, `max_tokens` 16,000, no `fallbacks`, blind to the writer),
    the harness sha and its five files, and P20's sign-off. It says "the baseline's rep count", so
    it holds at 2 reps or 3. Each directory holds nothing else yet, so a report built before the
    runs will show three empty variants.
  - **The harness still matches.** `runner.harness_digest` computes
    `df41c54a485ac8c5f842b591475f34f0d89168adcbb64c8b0ae7473153383301` over the five files, and
    that is the approved sha in `_state.json`. So **no command needs `--approve-harness`**. If one
    is refused at the harness gate, something under the sha changed after P20, and that change
    needs reading first.
  - **Every variant passes the runner's own gates offline.** A scratchpad script called
    `_check_metrics`, `_prepare`, `_check_state_metrics` and `_check_harness` (`approve=False`)
    for `baseline`, `v1`, `v2` and `v3`, and built no client. All four models and the judge have a
    price in `llm_prices.json`. The flow has 16 cases. `baseline/` holds the pilot's 10 rows, so
    at 2 reps its run resumes at 22 of 32 (case, rep), and at 3 reps at 38 of 48. Each variant
    runs 32, or 48 at 3 reps.
  - `.env` holds a non-empty `GOLF_ANTHROPIC_API_KEY`, checked for presence only. The release
    `golf-mcp` is still the 2026-10-29 build that P20's size probe showed serves all 16 approved
    briefs, and no session or harness file has changed since. So no rebuild and no `GOLF_MCP`.
- **The estimate the user is asked to approve** is P20's, unchanged. Its basis is P20's 17:54
  item "For P21, an estimate", which was fitted on the pilot's measured rows.

  | variant | model | rows at 2 reps | coach | judge | total |
  |---|---|---|---|---|---|
  | `baseline` | Opus 5 | 22 | $0.57 | $0.91 | $1.48 |
  | v1 | Opus 5.5 | 32 | $0.62 | $1.31 | $1.93 |
  | v2 | Sonnet 5.5 | 32 | $0.31 | $1.31 | $1.62 |
  | v3 | Haiku 5.5 | 32 | $0.016 | $1.31 | $1.33 |

  - **2 reps: about $6.4, ceiling $51.52.** The judge is $4.85 of it, because it is always Opus
    5.5. Wall-clock is about 17 minutes in all (118 rows at 8.5 s).
  - **3 reps: about $9.7, ceiling $79.78**, and a rep-noise floor of ±0.14 by eval-audit.md's
    rule, against ±0.18 at 2 reps. Wall-clock is about 26 minutes (182 rows). This is the one
    lever P20 left. At 2 reps, `all_checks` cannot show a model beating Opus 5 (its 0.20 of
    headroom is at the floor), and `judge_all` can show a difference of about 0.2.
  - **The runner enforces no spend ceiling.** `--timeout-s` is a wall-clock ceiling per (case,
    rep), so the ceiling is the session's to watch between commands.
- **The four commands**, run in this order and back to back, so that the judge's prefix stays warm
  across variants (P20: a five-minute gap lets it go cold). Each one resumes after the rows it
  already has. At 3 reps, put `--reps 3` in all four, because every variant must use the same
  count:
  ```
  .venv/Scripts/python.exe -m golf_coach.evals.runner --flow coach_feedback --model claude-opus-5 --reps 2
  .venv/Scripts/python.exe -m golf_coach.evals.runner --flow coach_feedback --model claude-opus-5-5 --variant v1 --reps 2
  .venv/Scripts/python.exe -m golf_coach.evals.runner --flow coach_feedback --model claude-sonnet-5-5 --variant v2 --reps 2
  .venv/Scripts/python.exe -m golf_coach.evals.runner --flow coach_feedback --model claude-haiku-5-5 --variant v3 --reps 2
  ```
- **For the session that resumes P21** (the row stays 🔄):
  - if `v1/`, `v2/` and `v3/` still hold only `change.md`, and `baseline/results.jsonl` still
    holds only the pilot's 10 rows, the spend is neither approved nor run, so report `BLOCKED`
    again;
  - after each command, sum the spend so far from every variant's `meta.cost_usd` in
    `results.jsonl` and `cost_usd` in `errors.jsonl`. If that total plus the estimates of the
    variants still to run passes the approved ceiling, stop and report `BLOCKED` before the next
    command;
  - read every `errors.jsonl` line and explain it. Read one row per new model end to end: served
    `model`, `usage` with cache tokens, `latency_s`, `meta.retries`, `judge_model`, `judge_usage`,
    `meta.judge_cost_usd`, and `meta.cost_usd` above zero;
  - cross-check each variant's coach spend against `python -m golf_coach.llm_log summary --origin
    eval`, using `meta.cost_usd - meta.judge_cost_usd`, because the judge writes no log line (P18);
  - take per model, absolute numbers first: each code check's pass rate, and **never `all_checks`
    without `judge_all` and `judge_actionable` beside it** (P20's audit item 1); each judge
    criterion; refusals counted from `meta.failure_class`; truncations; the coach's mean and median
    `latency_s` (the judge's is not on the row); and the cost per case, with the judge's part
    separate;
  - **read every `nothing_unmeasured` failure by hand**, and report the false positives per model
    beside the metric. `flagged-shot` rep 0's "rotates" drill cue is the known one, and it is in
    the baseline's numbers (P20's sign-off);
  - explain every constant-metric warning by reading the paragraphs, as P20 did;
  - compare the cache-read shares before reading any cost difference (P20's finding, eval-audit.md
    §3);
  - **Decision 8's self-preference reading.** Set v1's (Opus 5.5) judge margin over the baseline
    beside its code-check margin over the baseline, per criterion. Where the judge favours Opus
    5.5 by more than the code checks do, say so as possible self-preference and not as quality;
  - recompute the noise floor from each model's reps, beside P20's table and the rule's figure;
  - replace the estimate with the measured spend;
  - build the report (`build-report-lite.mjs`, from that session's `claude-api` skill base, or
    the 2.1.319 bundle that P20 used if the session has no Skill tool). Check that it shows four
    variants and does not read `_judge_sanity/` as one.
- **Verify**: no Python changed, so pytest, ruff and mypy were not rerun. The precheck script
  exited 0 on all four variants, and the call log is unchanged.
- **Resumed 2026-11-11 19:31, after the user's 2-rep approval: the baseline ran, v1 stopped on the
  account's credit balance, BLOCKED.** Since the marker, nothing had run: the only changed files
  were the three `change.md` files and this plan. `runner.harness_digest` recomputed
  `df41c54a485a…3301` over the same five files, so no command needed `--approve-harness`.
  - **The baseline is complete.** The first command exited 0 at 19:34, about 2 minutes. It wrote
    22 rows, so the baseline holds 32 of 32 (case, rep). There were 0 truncated, 0 failed attempts
    and 1 backoff retry. **It cost $1.4625** against the estimate of $1.48, and the variant as a
    whole cost $2.1823, the pilot's $0.7198 included. The coach was $0.8332 of that and the judge
    $1.3491. The runner's summary on 16 cases: All checks 78.1%, Numbers 93.8%, Caveats 100.0%,
    No unmeasured 93.8%, Verdict kept 90.6%, Judge: all 59.4%, J: accurate 75.0%, J: actionable
    100.0%, J: top fault 90.6%, J: no invented 71.9%. It warned that two metrics are constant,
    `caveats_carried` and `judge_actionable`. Neither has been read yet.
  - **v1 stopped partway.** The second command (Opus 5.5) exited 1 at 19:37. **The account ran out
    of credit**, and every request after that point got a 400 `invalid_request_error`: "Your credit
    balance is too low to access the Anthropic API". This is not a harness fault, and the runner
    filed all 11 failures as `harness_error`:
    - 8 failed at the coach and cost nothing: `flagged-shot-distance`, `partial-alignment-tempo`,
      `no-handedness-driver` and `nothing-to-fix-left`, both reps of each. The call log holds 8
      `api_error` / `BadRequestError` lines for them;
    - 3 failed at the judge after the coach had answered (`interim-pivot-dtl` reps 0 and 1,
      `unscored-no-loft` rep 0). Their errors lines carry the coach's spend, $0.0721, and a resume
      will pay for those three coach calls again.

    v1 holds 21 scored rows on 11 cases. Its summary and its six constant-metric warnings come from
    a partial run, so **do not read them**.
  - **The spend so far is $2.7993 of the $51.52 ceiling.** On the estimates, the rest is about
    $0.73 to finish v1 ($0.66 for 11 of its 32 rows at $1.93, plus the $0.07 paid again), $1.62
    for v2 and $1.33 for v3. That makes about $6.5 in all, against the approved $6.4.
  - **The call log cross-checks.** `llm_log summary --origin eval` gives the baseline's coach
    32 calls, all `ok`, $0.833202. That equals the rows' `meta.cost_usd - meta.judge_cost_usd`. It
    gives v1's coach 24 `ok` (the 21 scored plus the 3 whose judge failed) and 8 `api_error`, for
    $0.578211. That equals $0.5061 from the rows plus $0.0721 from the errors lines.
  - **A rerun resumes cleanly.** An `errors.jsonl` line never holds a (case, rep) slot (the
    runner's module doc), so rerunning v1's command runs exactly its 11 unscored pairs. Do not
    delete `v1/errors.jsonl`. It is the record of the stop and of the $0.0721.
  - **For the session that resumes P21 next:**
    - check first that the credit is restored. The user's answer says so, or not. Do not probe
      with a paid call. If the answer is not yes, report `BLOCKED` again;
    - then run the three commands still to run, unchanged from the list above and in that order:
      v1's again, then v2's, then v3's. Sum the spend after each command with every variant's
      rows and errors lines, as above, against the $51.52 ceiling. The baseline is not rerun;
    - the judge's prefix will be cold when v1 resumes, so v1 will write it again. Compare
      cache-read shares before reading any cost difference;
    - after that, do the "For the session that resumes P21" list above in full. None of it has
      been done for the baseline yet, apart from the spend and the cross-check.
- **Resumed 2026-11-11 19:39, after the user's "add credit and resume": v1, v2 and v3 ran, and all
  four variants are complete.** Since the 19:31 resume, the only changed files were the run's own
  (`baseline/` and `v1/` rows and traces, `v1/errors.jsonl`) and this plan. None of the five harness
  files changed, so no command needed `--approve-harness`. The three commands ran unchanged, in
  order, and each exited 0: v1's at 19:40 (its 11 missing pairs, 69 s), v2's at 19:41 (32 rows,
  137 s) and v3's at 19:44 (32 rows, 126 s). The spend was summed after each command, and it never
  came near the ceiling. No call failed on the credit, so there was no third `BLOCKED`.
  - **The measured spend replaces the estimate. P21 cost $6.0924**, against the approved $6.4 and
    the $51.52 ceiling. That is the baseline's 22 rows at $1.4625, v1 at $1.9960, v2 at $1.4407 and
    v3 at $1.1933. v1's figure includes the $0.0721 paid twice for the three coach answers whose
    judge call failed on the credit. The judge was $4.375 of the total, against the $4.85
    estimated. The coach eval's whole spend, the pilot's $0.7198 included, is $6.8122 (P19's judge
    sanity run is separate). The runs took about 10 minutes of wall-clock against 17 estimated:
    Sonnet 5.5 and Haiku 5.5 rows came in at 3.9 to 4.3 s each, against the 8.5 s assumed.
  - **Every `errors.jsonl` line is explained.** `v1/errors.jsonl` holds 11 lines, and all 11 are the
    400 "Your credit balance is too low" from the first v1 attempt. There are 8 at the coach (no
    cost) and 3 at the judge, carrying the coach's $0.0721. All 11 are filed `harness_error`. The
    file is kept as the record of the stop. No other variant has an errors file.
  - **One row per new model, read end to end** (`clean-swing` rep 0 each). Every row in all four
    variants was served by the model asked for, judged by `claude-opus-5-5`, stopped `end_turn`,
    and has `meta.cost_usd` and `latency_s` above zero.
    - v1: served `claude-opus-5-5`; coach 2,089 in, 630 out, 2,886 read from the cache; 13.88 s;
      `meta.retries` 0. Judge 2,523 in, 1,169 out, 4,621 read; $0.0344, `meta.judge_retries` 0.
      `meta.cost_usd` $0.0559, which recomputes from the price table.
    - v2: served `claude-sonnet-5-5`; coach 2,089 in, 405 out, a 2,886-token cache write; 4.62 s.
      Judge 2,534 in, 1,103 out, 4,621 read; $0.0331. `meta.cost_usd` $0.0486.
    - v3: served `claude-haiku-5-5`; coach 2,089 in, 443 out, a 2,886-token cache write; 3.48 s.
      Judge 2,572 in, 1,149 out, 4,621 read; $0.0342. `meta.cost_usd` $0.0350, which also
      recomputes. Haiku 5.5 caches the coach's 2,886-token system prompt, so the prompt is over its
      minimum cacheable length.
    - The three 5.5 models count the coach's 32 requests at exactly 71,472 input tokens. Opus 5
      counts them at 71,408, 2 a call fewer, as P20 found.
  - **The call log cross-checks on every variant.** `llm_log summary --origin eval` gives the
    coach's spend as $0.833202 for the baseline (32 `ok`), $0.834576 for v1 (35 `ok`, 8
    `api_error`), $0.311375 for v2 and $0.016912 for v3 (32 `ok` each). Each equals the rows'
    `meta.cost_usd - meta.judge_cost_usd`, plus v1's $0.0721 on its errors lines.
  - **The results, per model, absolute numbers first.** Every case has both reps, so the mean over
    rows equals the mean over cases. The runner's summary agrees with this table. `all_checks`
    appears only beside `judge_all` and `judge_actionable` (P20's audit item 1).

    | metric | Opus 5 (`baseline`) | Opus 5.5 (v1) | Sonnet 5.5 (v2) | Haiku 5.5 (v3) |
    |---|---|---|---|---|
    | `all_checks`, as graded | 0.781 | 0.969 | 0.781 | 0.875 |
    | `all_checks`, grader errors removed (below) | 0.938 | 1.000 | 0.906 | 0.906 |
    | `judge_all` | 0.594 | 0.594 | 0.594 | **0.062** |
    | `judge_actionable` | 1.000 | 1.000 | 1.000 | 0.844 |
    | `numbers_in_brief` | 0.938 | 1.000 | 0.906 | 0.906 |
    | `caveats_carried` | 1.000 | 0.969 | 1.000 | 1.000 |
    | `nothing_unmeasured` | 0.938 | 1.000 | 0.844 | 0.969 |
    | `verdict_kept` | 0.906 | 1.000 | 0.969 | 1.000 |
    | `judge_accurate` | 0.750 | 0.719 | 0.688 | **0.156** |
    | `judge_top_fault` | 0.906 | 0.938 | 0.969 | 0.906 |
    | `judge_no_invented_claims` | 0.719 | 0.875 | 0.812 | 0.531 |
    | refusals (`meta.failure_class`) | 0 | 0 | 0 | 0 |
    | truncations | 0 | 0 | 0 | 0 |
    | coach `latency_s`, mean / median | 8.12 / 7.61 | 8.83 / 8.46 | 4.29 / 4.24 | 3.19 / 3.16 |
    | coach output tokens a call | 392 | 651 | 411 | 470 |
    | paragraph length, mean characters | 1,133 | 1,217 | 1,262 | 1,395 |
    | cost per case at 2 reps | $0.1364 | $0.1202 | $0.0900 | $0.0746 |
    | of which the coach | $0.0521 | $0.0477 | $0.0195 | $0.0011 |
    | of which the judge | $0.0843 | $0.0726 | $0.0706 | $0.0735 |

  - **13 of the 21 failing code-check verdicts are grader errors, read by hand.** That is far past
    eval-audit.md's "roughly one in ten". The pilot showed one (P20), and the full run shows the
    rest. The errors fall in three checks:
    - **`nothing_unmeasured`: 8 of 8 are false positives**, all of them the drill cue P20 signed off
      as a known one. The baseline has 2 (`flagged-shot` r0, the known one, and `no-handedness` r0),
      v1 has 0, v2 has 5 (`unscored-finish` r1, `interim-pivot` r0, `partial-alignment` r0,
      `uncalibrated-placement-dtl` r0, `flagged-shot-distance` r1) and v3 has 1 (`flagged-shot`
      r1). Each fails on a feel such as "so the hips rotate rather than slide", and none claims
      rotation was measured. Three of v2's paragraphs say outright that it was not ("I can't say
      anything about your turn, since rotation was not measured"). The check passes the same cue
      when it says "turn". **Without the false positives, `nothing_unmeasured` is 1.000 on every
      model.** v2's 0.844 is its fondness for that one verb, not unmeasured claims;
    - **`verdict_kept`: 4 of 4 are false positives**, a kind the pilot did not show. In the
      baseline's `partial-alignment` r0 and r1, a sentence calls the whole swing "strong" (or says
      "fine-tuning") and then names the hip slide as the thing to fix, and the check reads that as
      praise of the miss. In the baseline's `clean-left-driver` r1 and v2's `flagged-shot-distance`
      r1, a sentence calls one checkpoint's miss a miss and lists tempo among the passes. The check
      matched tempo alone, so it read the sentence as calling tempo a fault. Both kinds follow from
      P17's design: the check reads a sentence that names one checkpoint in its own words, and these
      sentences speak of a second one in words it does not match;
    - **`caveats_carried`: 1 of 1 is a false positive.** v1's `flagged-shot-distance` r1 carries
      the flag in its own words ("I'd hold off reading anything ... they don't add up: the total
      doesn't match carry plus roll"), not with the check's markers. This is P17's paraphrase limit
      the other way round: a paraphrase fails a check rather than getting past one;
    - **`numbers_in_brief`: 8 of 8 are correct by the rule.** Every one is a backswing target in
      milliseconds that the brief does not print. Some are rounded band edges ("400 to 700" and
      "410 to 700" for the brief's 408 to 706), and some are a target picked inside the band ("450",
      "500"). This is P17's "a worked number fails". Opus 5.5 is the only model with no such
      failure, and where it gave the band it quoted 408 to 706 as printed (`uncalibrated-placement`
      r1).

    So the raw `all_checks` puts Opus 5.5 0.19 above Opus 5, and the corrected one 0.06, which is
    inside the noise. **The code checks' ranking of the four models is mostly grader error.**
  - **What the judge saw that the code checks did not.** **Haiku 5.5 fails the judge on 30 of 32
    rows** (`judge_all` 0.062, a drop of 0.53, far past every floor). Its 27 `judge_accurate`
    failures were spot-read, and they hold. Nine paragraphs miscount the passes ("five of six pass"
    with two misses), two reverse the club path's sign, several give the hip slide a direction the
    prompt says is never measured, and several call a value at a band's edge "comfortably inside".
    Its 5 `judge_actionable` failures restate the headline's goal without a drill. The code checks
    pass most of these paragraphs, so Haiku's raw `all_checks` (0.875) ranks it above Opus 5. That
    is P20's audit item 1 seen in the data. Opus 5.5's own `judge_accurate` failures include four
    that give the hip slide a direction ("toward the trail side"). Opus 5's and Sonnet 5.5's are
    mostly the pilot's kinds: a miscount of the passes, an overstated margin, a link the brief does
    not draw, and now and then a direction.
  - **Every constant-metric warning is explained by the paragraphs, as P20's were.**
    - `caveats_carried` 1.0 (baseline, v2, v3): 9 or 10 rows per model carried a caveat (finish
      balance unscored, the flagged shot uncertain, `head_stays_back` unscored), and the other 22
      or 23 touched nothing a caveat qualifies. Only one paragraph in the whole run used a qualified
      row: v3's `interim-pivot` r0, which called the rotation numbers interim and passed. So the Q
      and rotation caveats were exercised once in 128 rows.
    - `judge_actionable` 1.0 (baseline, v1, v2): every paragraph gave a drill or a feel for the
      headline. Haiku's 0.844 shows the criterion can fail.
    - v1's `numbers_in_brief` 1.0: every number was printed in its brief, including 408 to 706.
    - v1's `nothing_unmeasured` 1.0: Opus 5.5 never used "rotate" in a cue. Its one mention of
      rotation is a disclaimer ("this view only sees side-to-side movement, not rotation"), which
      passes correctly.
    - `verdict_kept` 1.0 (v1, v3): the check read 0 to 3 sentences per row on v1 and 0 to 5 on v3,
      and none at all on 7 rows of each. Haiku's reversals sit in counts ("five of six pass") and
      sentences naming several checkpoints, which the check does not read (P17's limit). The judge
      caught them.
  - **The cache shares were compared before any cost.** The coach read 44.1% of its prompt tokens
    from the cache on the baseline, 51.1% on v1 and 49.3% on v2 and v3. The baseline is lowest
    because it ran in two sittings, so it wrote the coach's prompt 7 times, against 3 or 4 for the
    others. The judge read 51.8% on the baseline and about 63% on each variant. **The judge's prefix
    never went cold after the baseline.** v1, v2 and v3 wrote it 0 times, because every gap,
    including the credit stop from 19:37 to 19:40, was under five minutes. So the earlier note that
    v1 would write it again did not come true. The baseline's 6 judge writes (pilot 2, resume 4) add
    about $0.008 per case, and its longer explanations (1,327 output tokens a call, against 1,182 to
    1,246) explain the rest of its higher judge cost. On the coach side the shares are close, so the
    cost differences are price and length. Opus 5.5 is 0.8 times Opus 5's price per token but writes
    1.66 times the output, so it costs 0.92 times as much per case. Sonnet 5.5 costs 0.37 times as
    much and Haiku 5.5 0.021 times.
  - **Decision 8's self-preference reading.** The table gives Opus 5.5's (v1) margin over Opus 5
    (the baseline), with each judge criterion set beside the code checks that test the same thing.

    | judge criterion | judge margin | code check | code margin, as graded | code margin, errors removed |
    |---|---|---|---|---|
    | `judge_all` | 0.000 | `all_checks` | +0.188 | +0.062 |
    | `judge_accurate` | -0.031 | `numbers_in_brief`, `verdict_kept` | +0.062, +0.094 | +0.062, 0.000 |
    | `judge_no_invented_claims` | **+0.156** | `nothing_unmeasured` | +0.062 | 0.000 |
    | `judge_actionable` | 0.000 | none | | |
    | `judge_top_fault` | +0.031 | none | | |

    On the whole, the judge favours Opus 5.5 less than the code checks do, not more. **On one
    criterion, `judge_no_invented_claims`, it favours Opus 5.5 by more**: +0.156, where the matching
    code check shows nothing once its errors are removed. By Decision 8, that is **possible
    self-preference, not quality**. It is also at the noise floor (±0.15 to ±0.17). Read by hand,
    the failures are the same kind on both sides: the tempo-to-hip-slide causal link ("a hurried
    takeaway is exactly what lets the hips drift"). Opus 5 failed the criterion on 9 rows, 6 of them
    for a causal link from tempo to the hip slide or the head. Opus 5.5 failed it on 4, 3 of them
    for the same link, so the judge failed Opus 5.5's own links as well. Sonnet 5.5 gains +0.094 on the same criterion
    without being the judge. Both facts lean towards a real difference in how often each model draws
    the link, but neither rules self-preference out.
  - **The noise floor, recomputed from each model's reps**: 1.96 × SD × √(2/32), with the SD taken
    from the within-case variance, as in P13 and P20. Rep agreement out of 16 cases is in brackets.
    A gap between two models is real only past about the larger of their two floors.

    | metric | Opus 5 | Opus 5.5 | Sonnet 5.5 | Haiku 5.5 | P20 (5 cases) | rule, 1/√32 |
    |---|---|---|---|---|---|---|
    | `all_checks` | ±0.19 (11) | ±0.09 (15) | ±0.19 (11) | ±0.17 (12) | ±0.22 | ±0.18 |
    | `numbers_in_brief` | ±0.12 (14) | 0 (16) | ±0.09 (15) | ±0.15 (13) | ±0.16 | ±0.18 |
    | `caveats_carried` | 0 (16) | ±0.09 (15) | 0 (16) | 0 (16) | none | ±0.18 |
    | `nothing_unmeasured` | ±0.12 (14) | 0 (16) | ±0.19 (11) | ±0.09 (15) | ±0.16 | ±0.18 |
    | `verdict_kept` | ±0.09 (15) | 0 (16) | ±0.09 (15) | 0 (16) | none | ±0.18 |
    | `judge_all` | ±0.15 (13) | ±0.23 (9) | ±0.23 (9) | ±0.12 (14) | ±0.16 | ±0.18 |
    | `judge_accurate` | ±0.21 (10) | ±0.19 (11) | ±0.24 (8) | ±0.19 (11) | ±0.22 | ±0.18 |
    | `judge_actionable` | 0 (16) | 0 (16) | 0 (16) | ±0.15 (13) | none | ±0.18 |
    | `judge_top_fault` | ±0.09 (15) | ±0.12 (14) | ±0.09 (15) | ±0.09 (15) | none | ±0.18 |
    | `judge_no_invented_claims` | ±0.15 (13) | ±0.17 (12) | ±0.17 (12) | ±0.19 (11) | ±0.16 | ±0.18 |

    Put plainly, **only Haiku 5.5's drop on the judge is past the noise.** Opus 5, Opus 5.5 and
    Sonnet 5.5 tie on `judge_all` at 0.594. Their corrected `all_checks` (0.938, 1.000, 0.906) and
    every criterion differ by less than the floors. The `judge_all` verdict differs between a
    case's two reps (two different paragraphs) on 7 of 16 cases for both of those 5.5 models, which
    puts its floor at ±0.23 on each, among the widest in the table.
  - **The report** (`node "<2.1.319 bundle>/shared/evals/report/build-report-lite.mjs"
    .claude/hillclimb/coach_feedback/`, because this session had no Skill tool) rendered "4
    variants, 16 cases". `trajectory/scores.tsv` has `baseline`, `v1`, `v2` and `v3` columns with
    no blank cell, and neither file mentions `_judge_sanity`. The lite report draws only
    `all_checks`, the code-check headline as graded. It therefore shows Opus 5.5 ahead, and the
    table above is what corrects it.
  - **For P22**:
    - the ADR-036 results addendum takes its coach numbers from the per-model table above. It
      quotes `all_checks` both as graded and with the grader errors removed, and always beside
      `judge_all` and `judge_actionable`. It also says that the grader errors are 13 of the 21
      failing code-check verdicts, and why (the three kinds above);
    - what the coach eval says about `coaching_model`, as data for the user's decision and not a
      change: Opus 5.5 and Sonnet 5.5 are indistinguishable from Opus 5 on this eval at 2 reps.
      Sonnet 5.5 does it at 0.37 times the coach's cost per case and about half the latency, and
      Opus 5.5 at 0.92 times the cost and 1.1 times the latency. Haiku 5.5 is not a candidate for
      the coach, with a judge pass rate of 0.06. Set this beside P14's club-spec result;
    - say that the judge still has no human calibration (P20's sign-off), and that its one margin
      in Opus 5.5's favour is reported as possible self-preference;
    - if a later milestone fixes the grader, the three false-positive kinds are the list: "rotate"
      in a drill cue, praise of the whole swing in a sentence that names one miss, and a sentence
      listing several checkpoints read as one. A fix would change `coach_feedback.py` and the
      harness sha. It is not in M43 (Decision 10's hill-climb).
  - **For P23**: `.claude/hillclimb/coach_feedback/` now holds `baseline/`, `v1/`, `v2/` and `v3/`,
    each with `results.jsonl` and `traces/`, plus `v1/errors.jsonl`, a regenerated `report.html`
    and `trajectory/scores.tsv`. Keep `v1/errors.jsonl` whatever else is chosen, because it is the
    record of the credit stop and of the $0.0721.
  - **Verify**: the report shows four variants. All four `results.jsonl` files hold 32 rows, one
    per (case, rep), and no (case, rep) is unscored. No Python changed, so pytest, ruff and mypy
    were not rerun. The analysis was done by scratchpad scripts that only read the rows and traces.

### P22 (2026-11-11)

- **Done: the docs name the log and the evals, and ADR-036 has its results addendum.** No Python,
  Rust, test or run file changed, and no paid call was made.
  - **ADR-036**: the addendum "(2026-11-11, M43 P22): what the two evals measured, what that says
    about `coaching_model`, and where the build is more exact than §2 and §5". It holds the four
    refinements the findings asked for (P7's constant-metric warning and the zeros that stop a run,
    P8's backoff on the exception behind the note, P12's absent metrics and `Metric(always=False)`
    as the departure from build-eval.md, and P18's judge writing no log line); the measured spend
    ($11.95 in all: club spec $5.0029, judge sanity $0.1362, coach $6.8122); P14's club-spec table
    and readings, with the P790 5i and 6i lofts flagged for the user's second look; P21's per-model
    coach table, with `all_checks` as graded and corrected always beside `judge_all` and
    `judge_actionable`, the 13 of 21 grader errors and their three kinds, Haiku 5.5's judge drop,
    the self-preference reading and the judge's missing human calibration (P20); the
    `coaching_model` recommendation; and a short list for whoever runs the evals again (the grader
    fix list, the `brief_from_view` gate, more sets rather than more reps, `/claude-api
    hillclimb`). Two italic one-line notes under §2 and §5 point at it, as P7 asked. The Status
    says it is built and points at the addendum, without counting addenda in words (P1's
    finding for P22).
  - **The recommendation, as written there, is the user's to take**: on the coach, Sonnet 5.5 is
    the change the data supports (the same measured quality at 0.37× the coach's cost and about
    half the latency); because `coaching_model` serves all three routes, switching also gives up
    the club lookup's coverage, which Opus 5 has with about one value in six wrong; Opus 5.5 sits
    between; Haiku 5.5 is not a coach candidate; a model per route would need a `config.py` field;
    and any switch makes a stored conversation reseed rather than replay
    (`contracts/conversation.py`'s `model`). `coaching_model` is unchanged.
  - **`docs/README.md`**: "36 decisions, 123 addenda", and row 036's Addenda cell reads **1** with
    a summary of the addendum.
  - **`CLAUDE.md`**: the commands block gains `llm_log summary`, `evals.runner` (naming
    `judge_sanity` and `coach_cases` in its comment) and the lite report builder, with the skill
    base written as a placeholder because it is session-scoped. Each points at its module's
    docstring rather than restating it (P6's finding). The "Python stays only where it is
    required" bullet now says "§1 and its M43 addendum name every file that survives" (P1's
    finding). **One addition outside the Goal's list**: a routing row, "How good are Claude's
    answers here, and what does each call cost?", pointing at ADR-036 and its results addendum.
    It carries no count, as `CLAUDE.md`'s rule asks. Without it, nothing in the file routes a
    session to the evals.
  - **`docs/ARCHITECTURE.md`**: the review line reads 2026-11-11 (§1, §2, §4 and §5). §1's commands
    gain `llm_log summary`, `evals.runner`, `evals.judge_sanity` (named as the check to rerun after
    any change to the judge, P19's finding), `evals.coach_cases` and the report builder, each with
    its flags and exit codes from the module's own parser and docstring. §2 gains a paragraph on
    `llm_log` and `golf_coach.evals` inside the LLM exception, with the import closure and the
    `coach_cases` recorder exception. §4's table gains three rows: the call log (saying that the
    judge writes no line, P18's finding), the eval inputs and the eval runs. §5 gains a bullet on
    the evals as the measurement the canned-response tests cannot make, with the limits named.
- **On this box, Git Bash's `sed -i` rewrites a CRLF file as LF.** It did so to `CLAUDE.md` and
  `docs/ARCHITECTURE.md`, and both were restored to CRLF, which is how they were checked out
  (`core.autocrlf` is `true`, so the commit would have normalised them either way). The plan and
  `docs/README.md` are LF on disk and were kept LF.
- **For P23**:
  - no new markdown file in P22, so the doc-count pin is unchanged by it. The addenda total is
    now 123, which `test_the_map_counts_the_addenda_correctly` pins;
  - `docs/ARCHITECTURE.md` §4's "Eval runs" row ends "What of it is committed is the user's call
    at M43's close". Once the user answers the `.claude/hillclimb/` question, replace that clause
    with what was committed and what `.gitignore` gained;
  - the ROADMAP section and the WORKLOG entry should point at ADR-036's results addendum for the
    numbers and the recommendation, not copy them, by `CLAUDE.md`'s rule about counts;
  - in the WORKLOG's "next", name the user's open decisions: `coaching_model` (the addendum's
    recommendation), and the second look at the P790's 5i and 6i lofts.
- **Verify**: `pytest tests/test_docs_truth.py` gives 91 passed. Its one warning is the
  `.pytest_cache` permission. No Python changed, so the full pytest, ruff and mypy were not rerun.

### P23 (2026-11-11, finished 2026-11-12): verified, the WORKLOG and ROADMAP written, BLOCKED on what of `.claude/hillclimb/` to commit, answered and resumed

Nothing is committed and nothing is staged. The row stays 🔄 until the user answers the question
below, which the plan's Do line and ADR-036 §9 leave to the user.

- **Verify, all green after one fix.**
  - `cargo test`: 1,773 passed, 26 ignored, 0 failed, over 119 suites, as at M42's close.
    `cargo clippy --all-targets` has 0 warnings and `cargo fmt --check` is clean. No Rust file is
    changed in the tree.
  - ruff (`src tests scripts`) is clean, mypy is clean over 124 files, and `conformance.py check`
    exits 0.
  - **The full pytest, first run: 2,465 passed and 2 failed.** One was the doc-count pin, because
    `docs/README.md` was edited to 89 while the run was going (below). **The other was real**:
    `tests/evals/test_coach_feedback.py::test_the_committed_state_declares_the_metrics_and_the_approved_inputs_as_harness`
    ended `assert "harness_sha" not in state`, written in P17 when no sha was recorded. The user's
    `--approve-harness` in P20 recorded one, and P20, P21 and P22 did not run the full suite, so
    the pin was red from P20 on. **The assertion is deleted**, with a comment saying why: the sha
    is the user's record whether present or absent, the runner's gate is what compares it with
    the files, and the club-spec twin (`test_club_spec_flow.py`) never asserted it. A pin on its
    value would make every harness edit a red suite that only the user's flag clears. The harness
    sha does not cover `tests/`, so it is unchanged.
  - **The second full run, after the fix, on the temporary index below: 2,467 passed** in 5 min
    25 s, the same total as P19's. The one warning is paddle's ccache notice, which predates M43.
  - Both runs used `GOLF_MCP` pointed at a scratchpad copy of `target/debug/golf-mcp.exe` (current,
    built 2026-11-11 11:08, after the last Rust change). The release binary is still the 2026-10-29
    build, held by Claude Code's own server (PID 15388), as P15 found. The copy let `cargo test`
    relink the debug binary while pytest ran.
- **The doc count.** `docs/README.md` now reads "89 markdown documents: 74 in `docs/` — … 37 ADRs,
  … 12 in `plans/` — plus 15 outside it (…, two in `evals/`, …)", which is P1's 87 plus the two
  `cases.md` files (P9's and P15's findings). `.claude/` is outside the pin, so the hillclimb
  answer does not move it. It was checked on a **temporary index** (`GIT_INDEX_FILE` set to a
  scratchpad copy of `.git/index`, with the four new markdown files added `-N`): docs-truth 91
  passed. The real index is untouched, so `tests/test_docs_truth.py` fails on the real index
  until the commit's `git add`. That is expected.
- **ROADMAP**: the M43 row and section read ✅ Done (2026-11-11), 24/24 phases (P0–P23), and
  `Last Updated` reads 2026-11-11. Both point at ADR-036's results addendum for the numbers and
  the recommendation rather than copying them (P22's finding). The section names the three things
  that came out differently from the ask: the club-spec headline did not separate the models, the
  judge writes no log line, and the judge has no human calibration.
- **WORKLOG**: a 2026-11-11 entry above M42's, built from the findings and the addendum. It
  says the bag was rewritten in P11 and why (P11's finding), names the user's open decisions
  (`coaching_model`, and the P790's 5i and 6i lofts, P22's finding), and records the stale pin.
  **One bullet is pending**: "What of `.claude/hillclimb/` is committed: P23's question to the
  user, not yet answered." Its "Python" bullet holds the second run's count.
- **The Titleist names** (P11's finding). `grep -rn "T150\|T250" docs ROADMAP.md WORKLOG.md` finds
  ADR-026 and `docs/ARCHITECTURE.md` using "T150" as a key-normalising example, and ADR-027,
  `docs/M12_CLUB_SPECS.md`, ROADMAP's M12 and M15 sections and older WORKLOG entries recording M12
  and M15. None states the user's current bag, so none was changed. The WORKLOG entry says so.
- **The run files, read for the question.** `.claude/hillclimb/` is 3.8 MB, of which `traces/` is
  about 3.2 MB (eight directories, the full messages with thinking). Everything else is 543 KB,
  plus 44 KB of the two `report.html`. No file holds an API key, a local path or the user's name
  (grepped for `sk-ant`, `x-api-key`, `Authorization`, `aaron`, `Users` and `Desktop`). The
  repository is public (ADR-036 Context). **`_state.json` must be committed under any answer**,
  because `tests/evals/test_coach_feedback.py` and `test_club_spec_flow.py` read both flows'
  copies, so a clone without them fails the suite.
- **For P23's resume**, once the user answers:
  1. `.gitignore`: add what the answer ignores. Under the recommended answer that is
     `.claude/hillclimb/**/traces/`, `.claude/hillclimb/**/report.html` and
     `.claude/hillclimb/**/trajectory/`, with a comment naming ADR-036 §9 and that the lite
     builder regenerates the last two from `results.jsonl`. `.gitignore` is CRLF on disk.
  2. `docs/ARCHITECTURE.md` §4's "Eval runs" row (line 1205): replace "What of it is committed is
     the user's call at M43's close" with what was committed and what `.gitignore` gained (P22's
     finding). It is CRLF on disk, so edit with the Edit tool, not `sed -i`.
  3. WORKLOG's pending bullet: the same answer, in one line.
  4. Tick P23 ✅ here, and append "resumed" to this entry.
  5. `git add` the tree's M43 files: everything `git status` lists, plus the chosen
     `.claude/hillclimb/` files. Check `git status --short --ignored .claude/hillclimb` against
     the answer, and that `v1/errors.jsonl` is in (P21's finding). Then run
     `pytest tests/test_docs_truth.py` on the real index, which should give 91 passed.
  6. One commit onto `main`, ending with the attribution lines. A title in the house style:
     "M43: LLM evaluation and observability — a call log, two evals, and four models compared".

**Resumed 2026-11-12, on the user's answer** (option (a), everything but the traces):

- **The partial work was all kept.** `find … -newermt '2026-11-11 20:05'` listed only the stale
  pin's fix, `docs/README.md` and this plan, which the entry above accounts for.
- **`.gitignore`** gained `.claude/hillclimb/**/traces/`, `**/report.html` and `**/trajectory/`,
  under a comment naming ADR-036 §9 and why the last two need no record. `git status --ignored`
  lists the eight `traces/`, the two `report.html` and the two `trajectory/` as ignored, and
  stages 18 files under `.claude/hillclimb/`: both `_state.json`, eight `results.jsonl`, six
  `change.md`, `coach_feedback/v1/errors.jsonl` and `_judge_sanity/results.jsonl`. **Git Bash's
  `sed -i` turned the CRLF `.gitignore` to LF**; it was put back through Python before staging,
  and `git diff` shows the eight added lines only. Use the Edit tool or Python on a CRLF file.
- **`docs/ARCHITECTURE.md` §4's "Eval runs" row** names what is committed and what is ignored.
  **WORKLOG's pending bullet** holds the answer.
- **The close date is 2026-11-12**, the day of the commit, so the WORKLOG heading and duration,
  the ROADMAP row, the M43 section's status and `Last Updated` moved from 2026-11-11 to it. ADR-036's
  results addendum stays dated 2026-11-11, which is when it was written.
- **Verify, on the real index after `git add`**: full `pytest` 2,467 passed in 5 min 44 s (the
  docs-truth pin included, with 89 documents), ruff clean, mypy clean over 124 files,
  `conformance.py check` exit 0, and `cargo test -p golf-mcp --test docs` (the pin that reads
  ARCHITECTURE and ROADMAP) 2 passed. Nothing in Rust changed after the first attempt's full
  `cargo test`, so it was not rerun. `GOLF_MCP` again pointed at a scratchpad copy of the debug
  build, since the release binary is still held.
- **Not changed, and worth a later look**: ADR-036 §9's body still says what is committed "is left
  to M43's close (P23), where the user is asked". The answer is recorded in `docs/ARCHITECTURE.md`
  §4, `.gitignore` and WORKLOG, and no addendum was added, because this phase's Do line did not
  name one.
