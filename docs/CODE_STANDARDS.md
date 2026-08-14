# Code standards

> **Tier: AS-BUILT.** Every rule below cites something this repo already did, or states plainly
> that it has no precedent here yet. Nothing is aspirational; nothing is copied from a style guide
> without being calibrated against this codebase.

This is the rubric `architect-refactor` judges by. It lives here rather than inside the agent for
one reason: **work done in the main thread should follow the same rules it will later be judged
by.** A rubric hidden in an agent file is a rubric you only meet at review time.

## How a rule is written, and why

Every rule carries four parts:

| Part | Why it is required |
|---|---|
| **Violated when** | A finding is a classification, not an opinion. If the condition cannot be stated as a check, the rule is not ready. |
| **Precedent** | The general principle supplies the reasoning; the precedent supplies the *calibration*. SOLID does not know that a stdlib-only analysis core is deliberate here. |
| **NOT a finding** | Load-bearing. Generic principles flag this repo's deliberate choices — the single-adapter ports, the hand-written HTML, the duplicated-looking phase constants — unless the exception is written down. |
| **Evidence required** | What a finding must show. No evidence, no finding. |

Rules marked **[survey]** are cheap and structural: answerable by grep, `ruff`, a file listing or
an existing test. Those are the only rules `/refactor-review --all` applies, because a survey that
exercises judgment on twelve modules sprawls instead of converging.

**Zero findings is a correct and common result.** A module that violates nothing returns
`verdict: clean`. Anything that has been considered and declined lives in
[REFACTOR_LEDGER.md](REFACTOR_LEDGER.md) and may not be re-raised.

---

## Part 1 — Rules this repo proved

These are not imports from a book. Each one is a decision this project made, usually after being
bitten.

### R1 — Modules import `contracts`, never each other **[survey]**

*(Dependency Inversion; Acyclic Dependencies)*

- **Violated when:** a module under `src/golf_coach/` imports a sibling module — anything other
  than `contracts`, the standard library, or its own subpackages.
- **Precedent:** ADR-008 principle 1. Data flows producer → contract → consumer, which is why
  career mode's shapes went into `contracts/career.py` rather than `storage` and `analysis`
  learning about each other.
- **NOT a finding:** `storage/corpus.py` and `mcp/query.py` importing `load_analysis` / `load_state`
  from `api.state`. This inverts the rule *knowingly* and is recorded in ADR-008's addendum of
  2026-08-11, with the reason (one tolerant reader beats three copies), the proof it is not a cycle,
  and the fix for when someone wants it. The fix is already ticketed; re-raising it as a discovery
  is noise.
- **Evidence required:** the import line with `file:line`, and the module it reaches into.

### R2 — The analysis core is stdlib + `contracts` only **[survey]**

*(ADR-008 principle 4, dependency-level decoupling)*

- **Violated when:** anything under `analysis/` imports numpy, OpenCV, MediaPipe, pydantic-beyond-
  `contracts`, or any package outside the base install.
- **Precedent:** ADR-008's consequence "tests for the seam run with only `pip install -e .`". The
  whole scoring path is runnable on a machine with no ML stack, and that is load-bearing rather
  than tidy — it is what lets the corpus be re-analyzed anywhere.
- **NOT a finding:** `pose/` and `capture/` reaching for numpy and OpenCV. Pixels are an I/O-edge
  concern; `capture/source.py` says so in its docstring and keeps numpy out of `contracts` on
  purpose. The rule is about `analysis/`, not about the whole repo.
- **Evidence required:** the import, and which extra in `pyproject.toml` would be needed to satisfy
  it.

### R3 — Optional extras are imported lazily, never at module scope **[survey]**

*(Interface Segregation, applied to dependencies)*

- **Violated when:** a module reachable from an offline CLI imports a package from an extra
  (`fastapi`, `anthropic`, `paddle`, `ultralytics`) at module scope.
- **Precedent:** `tests/api/test_pipeline_imports.py` pins exactly two of these —
  `api/pipeline.py` must not pull in fastapi, `feedback/coach.py` must not import anthropic at
  module scope — because `scripts/analyze_bundle.py` runs on a `vision`-only install. Both are
  "easy to break by adding one convenient import", in the test's own words.
- **NOT a finding:** `api/app.py` importing fastapi at module scope. It *is* the web app; the
  boundary is `pipeline.py`, not the whole package.
- **Evidence required:** the import line, plus which entry point in `scripts/` stops working.

### R4 — One source of truth for anything that is also stated in prose

*(DRY, in the specific form that has bitten this repo three times)*

- **Violated when:** a fact that ships to a user or a model is hand-typed in a place that a
  registry, a constant or a data file already knows. Counts, member lists, and "which things
  exist" sentences are the recurring shape.
- **Precedent:** the reason `contracts/checkpoints.py` exists. `caveats.py` claimed five
  checkpoints while six shipped, for a whole milestone, and that text goes verbatim into the
  coaching system prompt and to every MCP client. `caveats.py`'s own docstring had already made the
  argument — *"prose that exists twice is prose that goes stale in one place"* — and was stale
  anyway. Deriving beats remembering.
- **NOT a finding:** a band value quoted in a doc that is labelled a snapshot with a date. Bands
  live in `analysis/benchmarks/ranges.json` and every doc here already treats a quoted one as
  historical.
- **Evidence required:** both locations, and which one a reader or a model actually consumes.

### R5 — Parallel collections keyed by the same names become one collection of records

*(High cohesion; the concrete form R4 takes in code)*

- **Violated when:** two or more dicts, lists or tuples are keyed or ordered by the same identifiers
  and must be edited together.
- **Precedent:** `analysis/measure.py` carried three dicts keyed by the same seven metric names —
  function, unit, detail — with only a test asserting their key sets matched. As its `PoseMeasurement`
  docstring records, that "caught a missing entry but never a misaligned one". One `NamedTuple` per
  metric made the misalignment unrepresentable.
- **NOT a finding:** `CHECKPOINT_REGISTRY` in `contracts/` and `CHECKPOINT_EVALUATORS` in
  `analysis/checkpoints/`. They look parallel and are separated deliberately: ADR-008 forbids
  `contracts` importing `analysis`, so identity lives in one and behaviour in the other, with a test
  pinning them to each other. Merging them would break the dependency rule.
- **Evidence required:** the collections, their shared keys, and the edit that would have to touch
  all of them.

### R6 — A registry, once it exists, is walked and not re-listed **[survey]**

*(Open/Closed)*

- **Violated when:** code enumerates checkpoint names, metric names or phase names literally when a
  registry already holds them.
- **Precedent:** `POSE_MEASUREMENTS`' docstring names the pain directly — a metric name "hardcoded
  two, in three places" — and `derive_pose_metrics.py` now iterates the registry instead. Adding a
  candidate metric became one line.
- **NOT a finding:** a test that names checkpoints literally in order to assert the registry
  contains them. A test that derives its expectations from the thing under test asserts nothing.
- **Evidence required:** the literal list, the registry that already holds it, and what a partial
  edit would produce.

### R7 — Refuse rather than guess

*(Fail-fast, in this repo's strongest form)*

- **Violated when:** a missing input is filled with a default, a zero or a plausible value, instead
  of producing absence that the caller can see.
- **Precedent:** ADR-010 §2 — no score beats a wrong one. A checkpoint that cannot be measured
  returns `None` and is named in `SwingResult.unscored`; it is never counted as zero. `head_stays_back`
  with unknown handedness is the sharpest case: guessing right-handed would read a left-handed
  golfer's ordinary impact position as a gross fault, silently, in the direction nothing downstream
  can detect.
- **NOT a finding:** the tolerant artifact readers in `api/state.py`, where a half-written or
  older-schema file reads as *absent* rather than raising. That is the same principle — absence is
  the honest answer — not its opposite.
- **Evidence required:** the substituted value, and what a consumer would conclude from it.

### R8 — Raise on a wiring bug; return absence for a data condition

*(Fail-fast, and its boundary)*

- **Violated when:** an impossible state is swallowed and returned as `None`, or an ordinary
  data condition raises.
- **Precedent:** `contracts/checkpoints.py`'s `spec_for` raises `KeyError` with the reason spelled
  out — every caller holds a name that came out of the registry, so a miss is a wiring bug, not a
  data condition. The tolerant readers do the opposite, for the opposite reason.
- **NOT a finding:** a measurement function returning `None` on short or noisy input. That is a
  data condition and R7 requires it.
- **Evidence required:** the call sites, and which of the two categories the failure falls in.

### R9 — Measuring stays separate from judging

*(Single Responsibility, in the shape this domain needs)*

- **Violated when:** a band, threshold or verdict appears in `analysis/measure.py`, or a raw
  measurement is recomputed inside `analysis/checkpoints/`.
- **Precedent:** the split is what lets a metric be measured across the whole corpus *before* a band
  for it exists — the exact sequence M6.5 needed, where `head_hip_offset_impact_norm` was measured,
  studied, and then rejected without ever gaining a band.
- **NOT a finding:** normalization inside a measurement (dividing by shoulder width). That is part
  of producing the number, not judging it.
- **Evidence required:** the threshold literal or the recomputation, with `file:line`.

### R10 — Rejected alternatives stay runnable

*(This repo's answer to re-litigation)*

- **Violated when:** a losing candidate is deleted rather than kept as a named row, or a decision
  is reversed without an addendum recording why.
- **Precedent:** `docs/README.md`'s Conventions section — `scripts/golfdb/tune_*.py` keep every
  losing candidate, including deliberate no-pose baselines any future candidate must beat. The ADR
  addenda do the same for decisions. It is why the same idea does not get re-tried.
- **NOT a finding:** deleting code that never represented an alternative — a dead helper, an unused
  import. `ruff` owns those.
- **Evidence required:** what was removed, and where its result was recorded instead.

---

## Part 2 — General rules, calibrated here

Standard principles, each with a repo example or an honest note that this repo has not yet had the
problem.

### R11 — Speculative abstraction *(KISS / YAGNI)*

- **Violated when:** an abstraction has exactly one implementation and no second one is named in
  `ROADMAP.md`, an ADR, or the abstraction's own docstring.
- **Precedent:** `docs/archive/M4_POC_PLAN.md`'s anti-over-engineering guardrails declined
  `merge.py`, `checkpoints/outcome.py`, extra scoring policies and SQLite on exactly this ground —
  "pose-only = one stream, nothing to align (YAGNI)". Named seams, not built ones.
- **NOT a finding:** `capture/source.py`'s `VideoSource` and
  `launch_monitor/screen/recognizer.py`'s `TextRecognizer`. Both are Protocols with one live
  adapter, and both name the second in their own docstring — `LiveCameraSource` waiting on
  hardware, and the OCR engine `build_recognizer` is written to swap. Hardware and vendors that do
  not exist yet are the reason ADR-007 asked for the port before the second adapter.
  `launch_monitor/source.py`'s `ShotDataSource` is the same bet already paid off — it now carries
  four adapters, three of them live.
- **Evidence required:** the abstraction, its single implementation, and a grep showing no named
  second.

### R12 — Validation belongs at boundaries, not in the middle

- **Violated when:** a value parsed and validated once at an edge is re-validated on every internal
  hop, or an internal constant is modelled as if it arrived from outside.
- **Precedent:** `CheckpointSpec` is a `NamedTuple` rather than a `BaseModel`, and says why in its
  docstring — a compile-time constant, never parsed from JSON, never crossing a boundary. The same
  reasoning as `benchmarks.store.ResolvedRange`. Pydantic is for what arrives from disk or HTTP.
- **NOT a finding:** the pydantic models in `contracts/`. Those *are* the boundary shapes.
- **Evidence required:** where the value enters the system, and the redundant check.

### R13 — A comment records why, not what

- **Violated when:** a comment or docstring restates the code beneath it, or a non-obvious decision
  has no comment at all.
- **Precedent:** this repo's house style, and the reason it is navigable. The comments that carry
  the measurement or the rejected alternative behind a decision — why `lite` was kept over `heavy`,
  why a band edge is asserted only where it clears the instrument — are the ones worth having.
- **NOT a finding:** a short docstring on an obvious helper. Brevity is not absence.
- **Evidence required:** the comment and the line it restates — or the decision that has none, with
  a note on what a reader would have to reconstruct.

### R14 — Long parameter lists and boolean traps

- **Violated when:** a public function takes more than about five positional parameters, or a
  boolean flag changes what the function fundamentally does.
- **Precedent:** **none in this repo.** Stated so the absence is deliberate rather than an
  oversight — the functional-core style has kept signatures short so far. `analyze_swing` is the
  closest call, at seven parameters, and stays readable only because every optional one defaults to
  `None` and is passed by keyword at every call site. Treat it as the ceiling, not the target.
  Flag a new offender if one appears; do not go hunting.
- **Evidence required:** the signature, and a call site where the argument order or the flag's
  meaning is unclear at the call.

---

## What is deliberately not a rule here

- **Test coverage percentages.** This repo tests behaviour that would be expensive to get wrong,
  not lines. A coverage number is not evidence of anything.
- **Formatting, import order, unused names.** `ruff` owns these and runs in seconds. A finding a
  linter could have made is a wasted finding.
- **Hand-written HTML under `api/static/`.** A deliberate choice, not an absent framework.
- **File length on its own.** Long is not a defect; long *and* doing two unrelated jobs is R9.
