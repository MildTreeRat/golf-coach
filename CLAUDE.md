# Working in this repo

A home-lab golf swing trainer: face-on phone video → pose → phase segmentation → checkpoints
scored against bands cut from tour footage → ranked coaching tips. Shot data arrives by OCR of a
simulator screen. Python 3.11, `src/` layout, stdlib-only analysis core.

**This file exists to stop you reading the whole repo before you start.** It is ~39,000 lines
across source, tests, docs and HTML; a full orientation sweep costs more than most tasks are
worth, and — because it forces compaction mid-task — it makes you *less* accurate, not more. Read
this file, then the three-to-six things your change actually touches.

Nothing here is a count, a status or a band value. Those live in code and are pointed at, never
copied — that is the rule this file has to keep, because the last document that copied them told
every coaching call a false fact for a milestone.

## Commands

```bash
.venv/Scripts/python.exe -m pytest          # `-q` is already in pyproject addopts; a second -q
                                            # silences the summary line
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
.venv/Scripts/python.exe scripts/conformance.py check   # the committed golden vectors
cargo test                                  # the Rust crates, and every vector family `check`
                                            # defers: audio, format, all seven stages, and all 21
                                            # engine vectors end to end against the port
cargo clippy --all-targets && cargo fmt     # the Rust half of ruff
```

**There is Rust here now** (M20), a workspace at the repo root beside `pyproject.toml`. Six
crates, and exactly one of them has a Python caller: `crates/trigger` — ball-strike detection and
clip cutting, and `cargo build --release` is what `api/pipeline.py` needs before it can detect a
strike at all. `crates/capture` (M21) is the camera edge, and
`crates/{contracts,analysis,feedback,core}` (M22) are a **complete second implementation of the
analysis engine**, conforming on all 21 committed vectors and wired to nothing until M24. Nothing
in Python calls any of those five, and `api/pipeline.py` still calls `analysis/engine.py` — so a
change to the Python engine now silently puts two implementations out of step, and the committed
vectors are the only thing that says so.

`.venv/` is the real environment. A `venv/` directory also exists and is an empty stub — ignore
it. Entry points are the thin CLIs in `scripts/`; `docs/ARCHITECTURE.md` §1 "The commands,
precisely" lists them with their flags. Extras (`vision`, `api`, `llm`, `ocr`, `audio`, `research`) are
declared in `pyproject.toml`; the analysis core runs on a base install and that is load-bearing.

## Invariants

Break one of these and something breaks a long way from your edit.

- **Modules never import each other; everything imports `contracts/`** (ADR-008). `analysis`
  depends on `contracts` alone. No cycles. The one knowing exception is recorded in ADR-008's
  addendum: `src/golf_coach/storage/corpus.py` and `src/golf_coach/mcp/query.py` import *upward*
  into `api.state` for the tolerant artifact readers.
- **The analysis core is stdlib + `contracts` only.** No numpy in `analysis/`. This is a
  *two-language* invariant since ADR-030: the Rust core inherits it, so no numeric library in the
  scoring path there either. `crates/trigger` carries the one sanctioned exception, `rustfft`,
  for the same FFT the Python detector needed numpy for.
- **Python keeps only what does not translate** (ADR-030's 2026-09-22 addendum). That means
  MediaPipe, and the lab. It does *not* mean "uses numpy" — the strike detector `audio/` used to
  hold was numpy, and M20 ported it and **deleted** it. A module is retired from Python once a
  conforming Rust implementation exists *and* the vectors that prove it are committed; the vectors
  are the oracle, not the code that recorded them, which is why they must be right *before* the
  delete, and why `conformance.py regenerate` now refuses to rebuild that family.
- **`api/pipeline.py` must not import `fastapi`, and `feedback/coach.py` must not import
  `anthropic` at module scope.** Both are pinned by `tests/api/test_pipeline_imports.py`, because
  the offline CLIs run on installs without those extras.
- **Benchmark bands live in `src/golf_coach/analysis/benchmarks/ranges.json`, never in prose.**
  Each row carries its own provenance. Any band quoted in a doc is a snapshot and is probably old.
- **No score beats a wrong one** (ADR-010 §2). A checkpoint that cannot be measured returns `None`
  and is named in `SwingResult.unscored`; it is never guessed and never counted as zero.
- **Which checkpoints exist is `src/golf_coach/contracts/checkpoints.py`** (`CHECKPOINT_REGISTRY`).
  The engine walks it and `src/golf_coach/contracts/caveats.py` builds its prose from it. Do not
  write the panel's size or membership into a sentence — derive it.
- **Measuring is separate from judging.** `analysis/measure.py` produces numbers with no band in
  sight; `analysis/checkpoints/mechanics.py` turns them into verdicts. That split is what lets a
  new metric be measured across the corpus *before* a band for it exists.
- **An `ANALYSIS_VERSION` bump regenerates `spec/vectors/`, in the same change.** They are this
  repo's answer to a Rust port that disagrees silently (ADR-030 §8), and a vector recorded by an
  older engine certifies a port against answers that have been retracted.
  `tests/test_conformance.py` fails until they are re-recorded; `docs/CONFORMANCE.md` is the how.
- **Models are fitted offline and ship as data** (ADR-022). Fitting lives in `scripts/` under the
  `research` extra and may use numpy/scikit-learn; what enters the package is a provenanced JSON
  artifact plus stdlib arithmetic to evaluate it. `ranges.json` and `golfdb_v1.json` already work
  this way, `joint_model_v1.json` is the first *learned* one, and
  `tests/api/test_pipeline_imports.py` is what fails if a `golf_coach.*` module reaches for either
  library.

## Where the answer is

Route to a **section**, not a file. `docs/ARCHITECTURE.md` answers most of these and reading one
of its five sections costs a fraction of reading the file.

| Question | Read |
|---|---|
| What actually runs today? | `docs/ARCHITECTURE.md` §1 |
| What may import what? | `docs/ARCHITECTURE.md` §2, then ADR-008 |
| What happens in one `analyze_swing` call? | `docs/ARCHITECTURE.md` §3 |
| What is stored on disk, and where? | `docs/ARCHITECTURE.md` §4, `data/README.md` |
| How does this project know it works? | `docs/ARCHITECTURE.md` §5 |
| Why is it built this way? | `docs/decisions/` — and read the **addenda**, that is where reality corrected the call |
| Which doc answers X? | `docs/README.md` — the map, with a trust tier per doc |
| Where is this going next? | `ROADMAP.md` — *Status at a glance*, then your section only |
| What happened last session? | `WORKLOG.md` — **the top entry, and only the top entry** |
| Has this been tried and rejected? | `docs/M4_POSE_BAKEOFF.md` — grep it |
| Why is there a trained model, and where? | ADR-022, then `analysis/benchmarks/joint.py` |
| What is the app written in, and why? | ADR-030 — Rust core, MediaPipe pose in a Python sidecar, Flutter shell. **The core is built and wired to nothing** (M22); the sidecar, the session engine and the shell are not, so this repo still *runs* as the Python pipeline it describes |
| How is a port checked against this core? | `docs/CONFORMANCE.md` — schemas, golden vectors and the tolerance rules. Artifacts in `spec/`, runners `scripts/conformance.py` *and* `cargo test`. **Regenerate the vectors in the same change that bumps `ANALYSIS_VERSION`** |
| How is the *port itself* gated, stage by stage? | ADR-032 — the crates, the committed stage vectors, and §3's edges. **M22 is done** and the body is a phase behind the addenda on both counts: §1's two crates are four (six in the workspace), and §3's three edges are **six** — banker's rounding, `%g`/`.Nf` reaching the sentences, dict insertion order deciding which name lands in one, `str()` on a bare float, `max` returning the *first* maximum, and three-argument `math.hypot`, whose last bit decides a **bool**. Read the eleventh addendum for what the whole port measured, and the Status block before touching the port |
| Where is the Rust, and what is in it? | Six crates. `crates/trigger` — ball-strike detection offline and live, plus the ring buffer and cutting rules that turn a trigger into a clip. `crates/capture` — which cameras a host can see and what they can be asked for. `crates/contracts` — the payload shapes and their runtime bounds, mirroring the nine `contracts/` modules the engine's result is built from (M22 P2), plus all three registries and the `unscored` prose table, each landed beside its walker — `CHECKPOINT_REGISTRY` (P5), `placements` and `pivots` (P5b), `UNSCORED_REASONS` (P6). `crates/analysis` — the engine port (M22, ADR-032). `pyfmt` is §3's portability edges solved before any caller (P3); `smoothing`, `phases`, `measure` and `stats` are the **geometry**, the measuring half (P4); `benchmarks/store` and `checkpoints/` are the **judging**, the half that turns a number into a verdict and a sentence (P5); `trajectory`, `pivot`, `benchmarks/{joint,trajectory}` and `engine`'s three face-on measurement groups are the **population and rotation** numbers, recorded and judged by nothing (P5b); `engine`'s `analyze_swing_bundle`, `scoring` and the face-on slice of `alignment` are the **assembly** (P6); the rest of `alignment`, the second trajectory basis, the two `_dtl` measurement groups and `_without_contradicted_scores` are the **second view** (P7); `benchmarks/flight_model`, `flight` and `spin_solve` are the **ball flight**, the forward model and the inverse over it (P8) — `pyfmt` gained CPython's `math.hypot` there, the first §3 edge that reaches a bool rather than a sentence — and `shot_measure`, `flight_infer` and `flight_measure` are the **join** from a stored shot to those numbers (P8b), which is where the milestone's exit criterion was met. `crates/feedback` — `rules.py`'s ranked tips, depending on `contracts` and **not** on `analysis`, which is ADR-008's rule as a cargo edge. `crates/core` — the shell: the `run` seam and the `golf-core` binary, the only crate allowed to hold both halves, and where the first whole-bundle gate lives. What stayed in Python is named rather than left to be noticed: `alignment`'s render half (`pair_frames`, `warp_speeds`) and 130 lines of `flight_measure.py`, both because §M29 retires their callers rather than porting them, and neither reached by any committed vector. Start at any `src/lib.rs`; the constants carry their measurements, and `capture`'s crate doc carries the choice it has **not** yet made |
| Why did a module leave Python? | ADR-030's **2026-09-22 addendum** — the retirement rule, and M20 as the first module to meet it. ADR-032 §7 adds its third clause — *and nothing that stays Python calls it* — which makes the rule a schedule: `analysis/` leaves in §M29, once `api/` and `mcp/` have |
| What rules is code held to? | `docs/CODE_STANDARDS.md` — each rule with its precedent *and* a known non-violation |
| Has this refactor already been declined? | `docs/REFACTOR_LEDGER.md` — read it before proposing a structural change |

**Trust tiers** are declared at the top of every doc and tell you how hard to read: **AS-BUILT**
means trust it, **TARGET** means it is a plan so verify against code, **REFERENCE** means read the
design and not the numbers, **FOUNDING**/**SUPERSEDED** mean history.

## Do not read these end to end

Three files are ~3,900 lines between them and will eat a context window for almost no return:

- `WORKLOG.md` — append-only across 30-odd sessions. The top entry is the "pick up where I left
  off" doc; everything below it is history.
- `ROADMAP.md` — read the status table, then the one section you are working in.
- `docs/M4_POSE_BAKEOFF.md` — a change ledger of superseded numbers. Grep it for the thing you
  are about to re-try; do not read it.

## How much to read

Classify by **what the change touches**, not by what it is called — every change has these
properties, including ones nobody anticipated, so there is no "not in the list" case.

- **L1 — LOCAL.** A bug fix, test or docstring inside one module. Read this file, that module, and
  its mirrored tests under `tests/`. *Escalate if you find yourself crossing an import boundary.*
- **L2 — FEATURE.** A new metric, checkpoint, endpoint or adapter. Add: `contracts/checkpoints.py`,
  `analysis/measure.py`'s registry, `ranges.json`, `contracts/caveats.py`, the governing ADR, and
  `docs/ARCHITECTURE.md` §3. *Escalate if a shape in `contracts/` has to change.*
- **L3 — STRUCTURAL.** Anything in `contracts/`, `config.py`, `ANALYSIS_VERSION`, or a new
  subsystem. **Read widely here** — ADR-008/009/010, every consumer of the shape you are changing,
  `docs/ARCHITECTURE.md` in full, `docs/FLOW.md`. This level is not "read less"; a contract change
  with an unread consumer is the expensive kind of mistake.

**If you realise mid-task that you are under-informed, go up one level and say so in one line** —
what you escalated and why. That note is the only signal available for tuning these boundaries.

Reading less is safe here in proportion to how fast being wrong gets caught, and this repo catches
it fast: `tests/api/test_pipeline_imports.py` fails in seconds on a wrong architectural assumption,
and `tests/test_docs_truth.py` fails when code and documentation disagree.

## Writing a plan

A plan is the handoff between the session that designs and the session that builds, and the
second session pays full price for anything the plan left vague. "Add a checkpoint" makes it
rediscover the repo. Name, every time:

- **the files it will touch**, with paths;
- **the existing functions and registries it should reuse**, with paths — most things here already
  have a home, and a second copy of a tolerant reader is a second thing that drifts;
- **the tests that cover them**, and the new pin the change needs.

`tests/` mirrors `src/golf_coach/` package by package, so the test file for a module is its path
with `tests/` on the front.

## House style

Comments and docstrings here explain **why**, and often carry the measurement or the rejected
alternative behind a decision. Match that: a comment restating the code is noise, a comment
recording why the obvious thing was not done is the reason this repo is navigable. Rejected
alternatives stay runnable rather than being deleted — see `docs/README.md` §Conventions.

The rules behind that, written down and numbered, are `docs/CODE_STANDARDS.md`. It is worth
skimming before a structural change, because it is also the rubric `/refactor-review` judges by —
so code written against it is code that reviews clean. Its companion `docs/REFACTOR_LEDGER.md`
records what has already been declined and why; a change it argues against needs a new row rather
than a fresh argument.

`.claude/agents/` holds three review agents, all **manual only** — `/doc-check`, `/refactor-review`
and `/design`. Do not reach for them mid-task; the user invokes them.
