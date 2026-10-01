# Plan: M32 — Wider shot contract and device capability, in Rust; the first Rust re-record

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-10-01. **Milestone**: [ROADMAP §M32](../../ROADMAP.md#m32-wider-shot-contract-and-device-capability).
**The design is the program plan's
[§M32](m31-m40-shot-first-pivot.md#m32--wider-shot-contract-and-device-capability-detailed)**, re-detailed
Rust-only by M31.5 P6a. This file is its phase list. It does not restate that section: each phase
names the subsection it builds, and reads it. Where this file and §M32 disagree, this file's
"Calls this plan makes" section says why, and this file wins.
**Governing decisions**: [ADR-035](../decisions/035-rust-everywhere-python-where-required.md) clauses 3
(the oracle moves to Rust) and 4 (the frozen lab); [ADR-034](../decisions/034-shot-first-phone-first.md)
§2 (declared ∩ printed); [ADR-032](../decisions/032-the-rust-core.md), its twelfth and thirteenth
addenda (the oracle question M32 answers).

**Level: L3.** It changes `crates/contracts`' `ShotData`, bumps Rust's `ANALYSIS_VERSION`, and
re-records the committed vectors. Every phase reads its own list below rather than the repo.

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot — the findings are what the next phase
   is planned against;
3. stop, and tell the user to `/clear` and run `/next-phase`.

**Nothing is committed until P12** (the user's call, 2026-10-01). Every phase leaves its work in the
working tree, and P12 makes one commit onto `main`.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document, and the pointers to it | ✅ Done *(2026-10-01)* — approved by the user |
| **P1** | `compare.rs`: the comparator becomes library code | ✅ Done *(2026-10-01)* |
| **P2** | `stages.rs` I: the stage document, produced in Rust | ✅ Done *(2026-10-01)* |
| **P3** | `stages.rs` II: the compose check | ✅ Done *(2026-10-01)* |
| **P4** | `rerecord` I: the gate, the declaration and the ledger, as a library | ✅ Done *(2026-10-01)* |
| **P5** | `rerecord` II: the `golf-core rerecord` verb, proved on an empty declaration at v16 | ✅ Done *(2026-10-01)* |
| **P6** | Python: the frozen view and the `regenerate` refusal, proved a no-op today | ✅ Done *(2026-10-01)* |
| **P7** | Two photo-side reasons, and the stale comments in the Rust mirrors | ✅ Done *(2026-10-01)* |
| **P8** | The bump: ten new keys, `ANALYSIS_VERSION` 17, and the re-record | ✅ Done *(2026-10-01)* |
| **P9** | The three Rust-owned schemas, and their pin | ✅ Done *(2026-10-01)* |
| **P10** | The capability model: `capability.rs` and `devices.json` | ✅ Done *(2026-10-01)* |
| **P11** | Docs: CONFORMANCE, ARCHITECTURE §1, the ADR-032 addendum, `CLAUDE.md` | ✅ Done *(2026-10-01)* |
| **P12** | Close: verify, WORKLOG, ROADMAP, memory, and the one commit | ✅ Done *(2026-10-01)* |

---

## Rules every phase keeps

- **Every phase ends green on everything it can reach.** That is why this plan's order differs from
  §M32's "Sequence" (call 1 below). If a phase cannot end green, stop and record why as a finding;
  do not carry a red suite into the next session.
- **Frozen Python is untouched outside `scripts/conformance.py` and `tests/test_conformance.py`**
  (ADR-035 clause 4). `src/golf_coach/contracts/swing.py` stays at 16. Nobody "fixes" the version gap.
- **Nothing under `data/` is read or written.** Every step reads committed vectors only.
- **Only the verb writes vectors.** Only `golf-core rerecord` writes `spec/vectors/`, and only in
  P8 (P5's real-spec run must write nothing). If any other phase finds `git status` showing a change
  under `spec/vectors/`, that is a finding and a stop.
- **New code is Rust.** The one Python change is P6's tooling.
- **House style** (`CLAUDE.md` §House style, `docs/CODE_STANDARDS.md`): comments say *why*, and carry
  the measurement or the rejected alternative. Match the module you are in.
- **Do not write counts into prose** that a test or a registry owns. Where this plan quotes a count
  (42 files, 21 vectors), it is a snapshot for the phase to re-measure, not a value to copy.
- **The doc-count pin reads `git ls-files`**, so this file is invisible to it until P12 stages it.
  Do not touch `docs/README.md`'s document count before P12. This file also avoids the
  `**Status: … N/M phases` spelling on purpose: `tests/test_docs_truth.py::_PHASE_STATUS` would then
  demand a `docs/README.md` row, and plans are not listed there.

## Verify — the commands, and when each runs

```bash
cargo test                                          # every phase that touches crates/
cargo clippy --all-targets && cargo fmt --check     # every phase that touches crates/
.venv/Scripts/python.exe -m pytest                  # P6, P8, P11, P12 (and any phase touching Python or docs)
.venv/Scripts/python.exe -m ruff check src tests scripts   # P6, P12
.venv/Scripts/python.exe -m mypy src                # P12 (M32 changes nothing under src/)
.venv/Scripts/python.exe scripts/conformance.py check      # P6, P8, P12 — in the frozen view from P8
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py # every phase that edits a doc
```

---

## Decisions taken in the interview (2026-10-01)

1. **One commit, at the close, straight onto `main`.** Phases leave work uncommitted; P12 commits
   everything once, the way M23 and M31/M31.5 landed.
2. **The declaration is committed, at `spec/declarations/v17.json`.** `golf-core rerecord --declare`
   reads it, and each vector's `provenance.rerecords` copies the entries that matched that vector.
   It must **not** sit under `spec/vectors/`: `conformance.py::vector_paths` globs `*.json*` there and
   would read it as a vector.
3. **`stages.rs` reaches the engine's private helpers by making them `pub`.** The four are
   `windowed`, `shifted`, `camera_id` and `anchored_on_strike` in `crates/analysis/src/engine.rs`
   (near `:857–:930`). Python's `run_stages` imports the same privates (`E._windowed` …) for the same
   reason: a second copy is a thing that drifts. Each gains a doc line naming the stage recorder as
   its outside caller. The stage tests in `crates/analysis/tests/` keep their own duplicates; they are
   test binaries, and their module docs already argue that trade.
4. **`mock` declares every field it fills as `analysed`.** It is a test device whose numbers are
   self-consistent (`launch_monitor/mock.py`: smash ≈ 1.38, ball speed = club speed × smash), so
   nothing on it is untrustworthy the way HD Golf's club speed is. `hd_golf` is §M32's: only
   `club_head_speed` and `smash_factor` are `shown_only`.

## Calls this plan makes where §M32 left it open

Each is contestable before the phase that builds it; after that, a finding is the way to change it.

1. **The order: Python tooling before the bump, so every phase ends green.** §M32's Sequence lands the
   Python changes (step 4) *after* the re-record (step 3), which leaves `pytest` red for a phase. Here
   P6 lands them first. The frozen view is a no-op on a vector with no `rerecords` ledger, which is
   every vector until P8, so P6 can be proved on fixtures while the real suite stays green.
   Likewise the comparator, the stage producer and the verb (P1–P5) land and are proved at v16, which
   is §M32's step 1 exactly.
2. **A gap in §M32's Python list.** `tests/test_conformance.py::test_a_vector_round_trips_through_the_stdin_seam`
   (`:299`) also compares frozen Python's output with a synthetic vector's `expected`, and fails on
   the version move just as `test_each_vector_still_conforms` does. It joins the frozen view in P6.
3. **The ledger's path spelling is document-rooted and has no leading dot.** `analysis_version`,
   `expected.analysis_version`, `expected.swing.shot.attack_angle`; list indices as `[i]`. Python's
   `Difference.path` starts with `.` (`compare_results` builds `f"{path}.{key}"` from `""`), and Rust's
   comparator prints `<root>` for the root. Neither is matched as a string: both languages *parse* a
   ledger path and walk the document with it.
4. **Added-key patterns are exact paths in M32.** §M32 says "pattern"; nothing in M32's declaration
   needs a wildcard, so `[*]` is not built until a declaration needs it. The parser refuses one, so
   a later declaration that writes one gets an error rather than a silent non-match.
5. **A declaration entry that matches nothing in any vector fails the run.** It is a typo guard: a
   misspelled path would otherwise "declare" nothing and the gate would still pass. The check is
   across the whole run, not per vector, because synthetic vectors carry no shot and legitimately
   match none of the ten added keys.
6. **The declaration states its version, and the verb refuses a mismatch.** `v17.json` carries
   `"analysis_version": 17`; `rerecord` refuses it unless that equals `contracts::swing::ANALYSIS_VERSION`.
   That stops a stale declaration being re-run against a later engine.
7. **The run is atomic.** Every vector in both families is run, gated and composed before any file is
   written. One undeclared difference anywhere writes nothing anywhere.
8. **Each file's ledger entry records the entries that matched *that file*,** not the whole
   declaration. So a synthetic engine vector's entry moves `analysis_version` and
   `expected.analysis_version` and adds nothing; a corpus one adds the ten keys as well; a stage
   vector moves `analysis_version` alone. That is what makes "each file says which of its values are
   Rust's" literally true, and it is what the frozen view removes.
9. **The compose check runs against the engine document as it will be written,** not as committed.
   In M32 the two agree on everything the compose check reads, so this costs nothing now; it is the
   right order the first time a declaration moves a composed value.
10. **`--dry-run`.** `rerecord --dry-run` prints the report and writes nothing. §M32 says the report is
    what gets reviewed, never `git diff`; this lets it be reviewed before anything is written.
11. **`--spec <dir>`**, defaulting to the repo's `spec/`, so the verb's own tests run on a copy in a
    temp directory (`std::env::temp_dir()` joined with the process id, the precedent
    `crates/pose/tests/writer.rs:76` set).
12. **Rust's `ANALYSIS_VERSION` gets its own ledger, starting at 17.** Today
    `crates/contracts/src/swing.rs:39`'s doc says the history lives in `contracts/swing.py` and is
    "deliberately not copied here". That stays true for 1–16, but 17 has no Python entry and never
    will (the Python constant stays at 16). So the 16 → 17 entry is written in the Rust doc, in the
    Python ledger's form (`#: 16 -> 17 (2026-10-0x, M32 P8): …`, adapted to `///`), and a unit test in
    `swing.rs` mirrors `tests/test_docs_truth.py::test_the_version_ledger_documents_the_installed_version`
    for `17..=ANALYSIS_VERSION`. The doc paragraph saying "deliberately not copied" is rewritten to
    say where each half of the history lives.
13. **The Rust-owned `swing_result` and `swing_bundle_result` schemas keep `UnscoredReason` at frozen
    Python's set,** without `printed_blank` and `misread`, and P9's pin says so: the schema's enum must
    equal Rust's wire names minus those two. The two photo-side reasons must never reach a
    `SwingResult` (§M32 "Two photo-side reasons"; ADR-035 clause 4), so the schema stating that is
    the contract, and the pin makes it a checked choice rather than an omission.
14. **`SCREEN_PARSER_VERSION` and `parse_is_current` live in `crates/contracts/src/shot.rs`,** beside
    the provenance they read.

---

## Phases

Each phase lists: **goal**, **read first** (the only reading it needs beyond this file and
`CLAUDE.md`), **files**, **done when**, **verify**.

### P0 — This plan document, and the pointers to it

- **Goal**: this file, with the checklist, and the two places a session looks for M32's state
  pointing at it.
- **Files**: `docs/plans/m32-shot-contract.md` (new); `ROADMAP.md` (the M32 row in *Status at a
  glance*, and §M32's **Status** line); `docs/plans/m31-m40-shot-first-pivot.md` (the M32 row of its
  status checklist).
- **Done when**: the three edits point here, and docs-truth is green.
- **Verify**: `pytest tests/test_docs_truth.py`.

### P1 — `compare.rs`: the comparator becomes library code

- **Goal**: §M32 "The re-record", its first paragraph. `crates/core/tests/engine.rs::compare` moves to
  `crates/core/src/compare.rs` and returns typed differences, so P4's gate can tell an added key
  from a moved value.
- **Read first**: `crates/core/tests/engine.rs` (whole; the comparator is `:106–:196`);
  `crates/core/src/lib.rs` (the module doc, for house style); `docs/CONFORMANCE.md` §3 "The rules".
- **Files**: `crates/core/src/compare.rs` (new); `crates/core/src/lib.rs` (`pub mod compare;`);
  `crates/core/tests/engine.rs` (uses `golf_core::compare`; loses its copy and the comparator test).
- **Shape**:
  - `pub struct Difference { pub path: String, pub kind: DifferenceKind }`;
    `pub enum DifferenceKind { AddedKey, RemovedKey, Moved { expected: Value, actual: Value } }`.
    "Added" means the *actual* (Rust) side has a key the expected side lacks. A type difference, a
    refusal against a number, a list-length difference and an out-of-tolerance float are all
    `Moved` at that path: each is "this value is not what was recorded".
  - `pub fn compare(expected: &Value, actual: &Value) -> Vec<Difference>`, paths in call 3's spelling
    (no leading dot, root = `""`).
  - `Display` keeps today's messages ("expected a refusal", "type difference", "key … missing from
    the port", "the vector does not have"), so a red gate reads as it does now.
  - `RTOL`/`ATOL` move with it as `pub const`.
- **Tests**: `the_comparator_sees_the_differences_it_exists_to_see` moves into `compare.rs` and also
  asserts each case's `kind`; add one case per kind if a kind is not already covered, and one for
  the path spelling (`a.b[1].c`).
- **Done when**: `engine.rs`'s whole-bundle test passes through the library comparator, with the same
  vector count as before.
- **Verify**: `cargo test -p golf-core`, then `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`.

### P2 — `stages.rs` I: the stage document, produced in Rust

- **Goal**: §M32 "The stage document gets a Rust producer", first half. Port
  `scripts/conformance.py::run_stages` (`:341–:523`) to `crates/core/src/stages.rs`, producing the
  same `stages` object a committed stage vector holds.
- **Read first**: `scripts/conformance.py:312–:560` (`STAGE_NAMES`, `MEASUREMENT_GROUPS`,
  `run_stages`, `_jsonable`); one committed stage vector (`spec/vectors/stages/synthetic/windowed.json`
  — the one vector where the window offset moves, so the one that catches a forgotten shift);
  `crates/analysis/tests/measurements.rs` (`:324–:390` — how a Rust test already rebuilds the five
  groups), `crates/analysis/tests/flight.rs` (how it reads `flown`), and the `pub` surface they call.
- **Files**: `crates/analysis/src/engine.rs` (the four helpers become `pub`, decision 3);
  `crates/core/src/stages.rs` (new); `crates/core/src/lib.rs`; `crates/core/tests/stages.rs` (new).
- **Shape**: `pub fn run_stages(input: &VectorInput) -> serde_json::Value`. The seven keys are the
  Python's. Notes for the port:
  - the measurement groups are a **positional partition** of `engine::measurements`' one list, cut
    by the sub-calls' lengths — never grouped by `source` (the Python comment at `:422` says why).
    The shot cut can be `engine::shot_measurements(shot).len()` rather than Python's count of rows
    by device string, which avoids restating the device rule here at all (P10 moves that rule into
    `contracts::capability::device_of`);
  - `smoothed` records `[x, y]` per landmark only;
  - `flight.flown` is built by hand into JSON: `FlownShot`, `ShotFlight` and `FlightResult` carry no
    `Serialize` (`crates/analysis/src/flight_measure.rs:95`, `flight_infer.rs:508`, `flight.rs:210`).
    Do **not** add serde derives to `analysis` for this — the shape belongs to the vector, and `core`
    is vector plumbing (§M32's reason for putting `stages.rs` in `core`);
  - every frame index is window-relative, as Python's.
- **Tests** (`crates/core/tests/stages.rs`): all committed stage vectors (21 today — count them, do not
  hard-code from this plan) reproduce under `compare::compare` with **zero** differences; a count
  assertion in the form `engine.rs` uses.
- **Done when**: zero differences on every stage vector.
- **Verify**: `cargo test`, clippy, fmt.

### P3 — `stages.rs` II: the compose check

- **Goal**: §M32's second half of `stages.rs`. Port
  `scripts/conformance_vectors.py::_verify_stages_compose` (`:454–:~560`, read to its end).
- **Read first**: that function, whole; `stages.rs` as P2 left it.
- **Files**: `crates/core/src/stages.rs`; `crates/core/tests/stages.rs`.
- **Shape**: `pub fn verify_compose(stages: &Value, engine_vector: &Value) -> Result<(), String>`,
  checking what Python checks: the `smoothed` pass-through, `phases` shifted back onto
  `expected.swing.phases`, `measure` rounded to four places onto the `pose` rows (Python's `round` —
  use `pyfmt`'s, never `f64::round`; §3's banker's-rounding edge), the groups concatenating to
  `expected.swing.measurements`, `checkpoints` landing in `checkpoint_scores` or `unscored` (with the
  `cross_view_contradicted` allowance), `alignment.clip_alignment` onto `expected.alignment`, and the
  `flight` stage's part, whatever the Python's tail checks.
- **Tests**: every committed stage vector composes onto its engine vector's committed `expected`;
  and the check **can see**: perturb one value in each of three stages (a phase boundary, a pose
  row's value, a checkpoint score) and assert each is refused with a message naming the stage.
- **Done when**: both tests pass.
- **Verify**: `cargo test`, clippy, fmt.

### P4 — `rerecord` I: the gate, the declaration and the ledger, as a library

- **Goal**: §M32 "The verb", steps 2–5, as pure functions over `serde_json::Value`, so the rules are
  unit-tested before any file is touched.
- **Read first**: §M32 "The re-record" through "M32's declaration, in full"; calls 3–9 above;
  `compare.rs`.
- **Files**: `crates/core/src/rerecord.rs` (new); `crates/core/src/lib.rs`.
- **Shape**:
  - `Declaration { analysis_version: i64, note: String, added: Vec<String>, moved: Vec<String> }`,
    deserialized from the file; path parsing per call 3 and call 4 (a `[*]` is refused).
  - `fn gate(committed: &Value, ours: &Value, declaration) -> Result<Applied, Vec<Undeclared>>`, where
    `ours` is the committed document with Rust's answer substituted (`expected` from `run`, or
    `stages` from `run_stages`, and `analysis_version` from the constant). An `AddedKey` passes only at
    a declared `added` path; a `Moved` passes only at a declared `moved` path; a `RemovedKey` never
    passes (M32 declares no removal, and §M32 names no such kind).
  - `fn apply(committed: &Value, ours: &Value, applied: &Applied) -> Value` — **the committed
    document with only the declared paths replaced or inserted**. Every other value is the
    committed one, untouched (§M32 step 4: undeclared floats keep Python's bits).
  - The ledger: `provenance.oracle` is set to `"python"` if absent; `provenance.rerecords` gains
    `{"analysis_version": 17, "by": "golf-core rerecord", "declaration": "spec/declarations/v17.json",
    "added": [...matched...], "moved": [...matched...]}` (call 8).
  - "Nothing differs" returns a value the caller can see, so the verb writes nothing (step 5).
- **Tests** (unit, in `rerecord.rs`): an undeclared added key fails and names its path; an undeclared
  moved value fails and names its path; a removed key fails; a declared added key and a declared
  moved value land; every undeclared value in the applied document is the committed `Value`
  (including an in-tolerance float that Rust computed differently — the committed one survives); an
  identical pair applies nothing; a `[*]` in a declaration is refused; the ledger entry lists only
  matched entries; `oracle` is not overwritten if present.
- **Done when**: the tests pass. No CLI yet.
- **Verify**: `cargo test`, clippy, fmt.

### P5 — `rerecord` II: the verb, proved on an empty declaration at v16

- **Goal**: `golf-core rerecord --declare <file> [--dry-run] [--spec <dir>]`, walking the engine
  family (`synthetic/`, `corpus/`) and the stage family, atomic (call 7), with the typo guard (call
  5) and the version guard (call 6). Then §M32's Sequence step 1: an empty declaration over the
  real `spec/` finds no difference and writes nothing.
- **Read first**: `crates/core/src/bin/golf_core.rs` (whole); `scripts/conformance.py::_write_json`
  (`:155`) and `_read_json` (`:142`); §M32 "How the file is written".
- **Files**: `crates/core/src/bin/golf_core.rs` (the verb, and the module doc's "`regenerate` is
  deliberately not a candidate" paragraph rewritten per §M32's last paragraph of "The re-record" —
  why a gated re-record is not a self-portrait, ADR-035 clause 3); `crates/core/src/rerecord.rs`
  (file walking, reading, writing); `crates/core/src/lib.rs` (its module doc, where it says what the
  crate is); `crates/core/Cargo.toml` (`flate2` moves to `[dependencies]`, with a comment saying
  why); `crates/core/tests/rerecord.rs` (new).
- **Writing** matches `_write_json`: sorted keys (the workspace does not enable `preserve_order`),
  indent 2, a trailing newline on plain `.json`, gzip level 9 with mtime 0 for `.gz`. The text will
  still churn (exponent floats, `\u` escapes); the report is the review, never `git diff`.
- **The report** names, per vector, every applied path, and ends with totals: vectors run, vectors
  changed, files written (0 on `--dry-run`).
- **Stage vectors** are joined to their engine vector by `provenance.derived_from`, run through
  `run_stages`, gated, and composed (call 9) before anything is written.
- **Tests** (`crates/core/tests/rerecord.rs`, against a copy of a small slice of `spec/` in a temp
  dir — one synthetic engine vector, one corpus one, and their stage vectors): an undeclared
  difference fails and writes nothing anywhere; a declared change lands; a file read back gives the
  same `Value` as was applied; a second run writes nothing; `--dry-run` writes nothing; an unmatched
  declaration entry fails; a declaration at the wrong version is refused. To produce a real
  difference without changing the engine, the test edits the *committed copy* (e.g. a different
  `analysis_version`, or a removed key in `expected`) and declares it.
- **Then, on the real spec**: write an empty declaration to the session scratchpad (`{"analysis_version":
  16, "added": [], "moved": [], "note": "P5: empty"}`) and run `cargo run --release --bin golf-core --
  rerecord --declare <scratch>`. It must report zero differences on every vector in both families
  and write nothing. `git status spec/` must be clean. Record the report's totals in the findings.
- **Done when**: the above, and the module doc is rewritten.
- **Verify**: `cargo test`, clippy, fmt; `git status spec/` clean.

### P6 — Python: the frozen view and the `regenerate` refusal, proved a no-op today

- **Goal**: §M32 "What changes in Python, and why", in full, plus call 2's extra test, landed
  **before** any vector carries a ledger (call 1).
- **Read first**: §M32 "What changes in Python, and why"; `scripts/conformance.py` `:100–:240`,
  `cmd_check` (`:661`), `cmd_regenerate` (`:820`); `tests/test_conformance.py` `:40–:200` and
  `:358–:470`.
- **Files**: `scripts/conformance.py`; `tests/test_conformance.py`. Nothing else in Python.
- **`scripts/conformance.py`**:
  - `RUST_OWNED_SCHEMAS = frozenset({"shot_data", "swing_result", "swing_bundle_result"})`;
    `write_schemas` (and so `--schemas-only`) skips them. `SCHEMA_ROOTS` keeps all ten.
  - `frozen_view(vector, actual) -> tuple[expected, actual]`: deep-copies, then removes every path any
    `provenance.rerecords` entry lists (call 3's spelling; `expected.`-prefixed paths are removed from
    `expected` and, with the prefix stripped, from `actual`). A vector with no ledger comes back
    as-is.
  - `ledger_covers(vector, version) -> bool`: true when the vector is at `version`, or when it is
    above it and carries a `rerecords` entry for every version between. This is the re-pointed
    version rule.
  - `cmd_check`: an engine vector is compared through `frozen_view`; the version test uses
    `ledger_covers`; the stage staleness test likewise. The summary line says what is now being
    certified (e.g. "N/N vectors hold the freeze (frozen engine v16)"), and the stage line says the
    same. Keep the audio and format lines as they are.
  - `cmd_regenerate` refuses the full rebuild and `--stages-only`, with a message naming `golf-core
    rerecord` and ADR-035 clause 3, on `conformance_vectors._audio`'s precedent. `--schemas-only`
    writes the Python-owned roots only. `--format-only` is unchanged. `build_all` and
    `build_stages` stay runnable (house style: rejected paths stay runnable), they are just not
    reached from the command.
- **`tests/test_conformance.py`**:
  - `test_every_committed_schema_is_what_contracts_exports_today` (`:52`) skips `RUST_OWNED_SCHEMAS`.
  - `test_each_vector_still_conforms` (`:163`), `test_a_vector_round_trips_through_the_stdin_seam`
    (`:299`, call 2) and `test_each_stage_vector_is_what_this_build_produces` (`:420`) compare
    through `frozen_view`.
  - `test_each_vector_was_recorded_at_the_current_engine_version` (`:178`) and
    `test_each_stage_vector_was_recorded_at_the_current_engine_version` (`:403`) use `ledger_covers`;
    rename them to say what they now pin, and rewrite their docstrings.
  - New unit tests on in-memory fixtures (no file writes): a ledgered vector whose declared value
    moved passes the frozen view; one whose *undeclared* value moved fails it; a v17 vector with no
    ledger fails `ledger_covers`, and one with a v17 entry passes; `regenerate` refuses the engine
    and stage families (call `main(["regenerate", "--stages-only"])` and assert the exit code and
    that nothing was written).
  - `test_each_stage_vector_still_composes_onto_its_bundle_answer` (`:440`) is unchanged.
- **Done when**: `pytest` green with the real vectors untouched — every new branch is reached only by
  fixtures, which is the proof the view is a no-op today; `conformance.py check` still passes, and
  still says v16.
- **Verify**: `pytest`, `ruff check src tests scripts`, `conformance.py check`, `git status spec/` clean.

### P7 — Two photo-side reasons, and the stale comments in the Rust mirrors

- **Goal**: §M32 "Two photo-side reasons" and "The stale comments, fixed in the Rust mirrors". No
  serialized output moves: no vector carries either reason, and comments are not output.
- **Read first**: `crates/contracts/src/unscored.rs` (whole — the table, its tests at `:393–:543`);
  `crates/analysis/src/shot_measure.rs` (module doc and `measure_ball_speed`, near `:16–:18` and
  `:111–:113`); `crates/analysis/src/engine.rs` near `:478` (the doc on `BundleRequest.shot`);
  `crates/contracts/src/intent.rs` (five "full M4"s); ADR-034 §3 and §5 for the replacement wording;
  M31 P2 finding 1 in `docs/plans/m31-shot-first-adr.md` for the smash numbers (0.76–1.06 over the 13
  stored shots).
- **Files**: `crates/contracts/src/unscored.rs`; any `match` on `UnscoredReason` the compiler flags
  (`crates/feedback/src/rules.rs`, `crates/analysis/src/…` — let `cargo build` find them; an arm for a
  photo-side reason in a pose or flight path is `unreachable!` only if the type proves it, otherwise
  handle it as the table says); `crates/analysis/src/shot_measure.rs`; `crates/analysis/src/engine.rs`;
  `crates/contracts/src/intent.rs`.
- **The two rows**. `summary` is a clause, `remedy` the golfer's sentence (`feedback::rules` puts it
  on a `Tip` verbatim). Drafts, to be fitted to the table's voice:
  - `PrintedBlank` / `printed_blank`, `refilming_helps: false` — summary: "the launch monitor printed
    nothing for this number on this shot"; remedy says the tile was there and empty or `---`, so
    another photo of the same screen would read the same, and nothing about the photo went wrong.
  - `Misread` / `misread`, `refilming_helps: true` — summary: "the number on the screen could not be
    read from the photo"; remedy says there was text under the tile and no value could be read from
    it, and a sharper, square-on photo of the screen may fix it.
  - Each variant's doc says it is **never** written into a `SwingResult` (ADR-035 clause 4: frozen
    Python's `UnscoredReason` is closed and would refuse the artifact), and that M35/M37 are the
    first to emit it, into `shot_analysis.json`.
- **Tests**: `every_reason_has_a_row` and the wire-name table gain both; the wire-name test's doc says
  these two have no Python twin, by design. `refilming_helps_exactly_where_the_clip_is_the_problem`
  gains `Misread`, and its doc says "the clip or the photo". `the_inference_family_is_five_rows_of_this_table`
  is unchanged.
- **Done when**: `cargo test` green with every vector still conforming (proof no output moved).
- **Verify**: `cargo test`, clippy, fmt.

### P8 — The bump: ten new keys, `ANALYSIS_VERSION` 17, and the re-record

- **Goal**: §M32 "The shot contract", "`ANALYSIS_VERSION` 16 → 17, in Rust only", and Sequence step 3.
  The one phase that writes `spec/vectors/`.
- **Read first**: §M32 "The shot contract" and "M32's declaration, in full"; `crates/contracts/src/shot.rs`
  (whole); `crates/contracts/src/swing.rs:1–:40`; `crates/contracts/tests/round_trip.rs` (whole);
  call 12; the Python ledger's last entry (`src/golf_coach/contracts/swing.py`, the `15 -> 16` block
  near `:394`) for form only.
- **Files**:
  - `crates/contracts/src/shot.rs` — `ShotData` +7 (`attack_angle`, `dynamic_loft`, `low_point`,
    `impact_offset_h`, `impact_offset_v` as `Option<f64>`; `impact_position_v` as `Option<String>`;
    `carry_offline` as `Option<f64>`; each `#[serde(default)]`, units in the doc as §M32's table
    gives them). `ShotProvenance` +3 (`parser_version: i64` default 0, refused if negative in
    `Validate`; `fields_present: Option<Vec<String>>`; `corrections: BTreeMap<String, String>`, with
    the `raw_fields` reason for the map type). `SCREEN_PARSER_VERSION = 1` with its ledger comment
    (entry 1 is M34's Rust parser; nothing stamps it before M34; whether M34's port and tie rule are
    one version or two is M34's call). `pub fn parse_is_current(shot: &ShotData) -> bool`.
  - `crates/contracts/src/swing.rs` — `ANALYSIS_VERSION = 17`; the ledger (call 12), saying it is
    shape only: `ShotData` and `ShotProvenance` gained keys and no number moved; `contracts/swing.py`
    stays at 16 on purpose, and `api/state.py::is_outdated` compares with `<`, so frozen Python reads a
    v17 artifact as current. The ledger pin.
  - `crates/contracts/tests/round_trip.rs` — the one allowance (§M32 "Tests and new pins"): an `input`
    key may come out of the round trip at its serde default where it went in absent, **only** if the
    same key under `expected.swing.shot` is listed in one of that vector's `rerecords` entries. Spell
    the mapping out in the code (`input.shot.X` ↔ `expected.swing.shot.X`, and the provenance keys
    likewise); anything else absent-then-present is still a difference.
  - `spec/declarations/v17.json` (new) — §M32's declaration in full: `added` = the seven
    `expected.swing.shot.*` and three `expected.swing.shot.provenance.*` paths; `moved` =
    `analysis_version`, `expected.analysis_version`; `analysis_version: 17`; a `note` naming M32 and
    ADR-035 clause 3. **Inputs are not rewritten.**
  - `spec/vectors/**` — written by the verb alone.
- **Order inside the phase**:
  1. the contract and the constant, and `cargo build`;
  2. `golf-core rerecord --declare spec/declarations/v17.json --dry-run` — read the report: only the
     declared paths, on every vector; any other difference stops the milestone here (record it as a
     finding, revert the constant, stop);
  3. the same without `--dry-run`;
  4. once more: it must write nothing;
  5. the full Rust and Python suites.
- **Tests** (`shot.rs` unit tests): a Python-shaped shot with none of the ten keys reads with each at
  its default; a shot with all ten round-trips; `parse_is_current` for a stamped shot, an unstamped
  one, and a direct feed (no provenance → `true`); a negative `parser_version` is refused.
- **Existing pins that must pass unchanged after the re-record**: the version assertions in
  `crates/core/tests/engine.rs`, `crates/analysis/tests/{geometry,measurements,judging,alignment}.rs`
  and `round_trip.rs`; P2/P3's stage tests; P6's Python pins, now exercising real ledgers.
- **Done when**: every committed engine and stage vector carries `provenance.oracle: "python"` and one
  `rerecords` entry; a second run writes nothing; `cargo test` and `pytest` are green;
  `conformance.py check` passes in the frozen view; `contracts/swing.py` still says 16; `git status`
  shows no change under `spec/vectors/format/` or `spec/vectors/audio/`.
- **Verify**: all of the above. Spot-check one corpus file with a short Python one-liner: every value
  outside the declared paths equals the pre-P8 committed value exactly (`git show HEAD:<file>`), which
  is §M32's exit "every undeclared value is the committed Python value", checked by a second language.
  Record the totals and the spot check in the findings.

### P9 — The three Rust-owned schemas, and their pin

- **Goal**: §M32 "Schemas: split ownership, with the Rust half hand-maintained".
- **Read first**: that subsection; `spec/schemas/shot_data.schema.json` (whole) and the `ShotData` /
  `ShotProvenance` / `UnscoredReason` entries under `$defs` in the other two; call 13.
- **Files**: `spec/schemas/{shot_data,swing_result,swing_bundle_result}.schema.json` (hand-edited);
  `crates/contracts/tests/schemas.rs` (new).
- **The edit**: the seven `ShotData` properties and the three `ShotProvenance` ones, spelled as
  pydantic spells their neighbours — `anyOf [{type}, {type: null}]`, `default: null`, `title` in
  Title Case, `description` from the Rust doc; `parser_version` as an integer with `default: 0` and
  `minimum: 0`; `fields_present` as a nullable array of strings; `corrections` as `raw_fields` is.
  Keys sorted, indent 2, as the other files are. Each file's top gains nothing (JSON Schema has no
  comment); the ownership note lives in `docs/CONFORMANCE.md` §1 (P11).
- **The pin** (`schemas.rs`): in each of the three files, the property sets of `ShotData` and
  `ShotProvenance` equal the key sets the Rust structs serialize (serialize a default-ish instance to
  `Value` and take its keys); the `UnscoredReason` enum in the two swing files equals Rust's wire
  names minus `printed_blank` and `misread` (call 13); and the ten new properties each carry a
  `description`.
- **Done when**: the pin passes; Python's schema-freshness test (skipping these three since P6) and
  `test_every_on_disk_artifact_has_a_schema` still pass.
- **Verify**: `cargo test`, `pytest tests/test_conformance.py`.

### P10 — The capability model: `capability.rs` and `devices.json`

- **Goal**: §M32 "The capability model", in full.
- **Read first**: that subsection; `crates/contracts/src/lib.rs` (`ContractError`, how other modules
  load JSON by `include_str!`, if any do — else `crates/analysis/src/benchmarks/store.rs` for the
  pattern); `src/golf_coach/launch_monitor/screen/profiles.json` (the `hd_golf` targets) and
  `launch_monitor/mock.py` (what mock fills); `crates/analysis/src/engine.rs::shot_measurements`
  (`:378`).
- **Files**: `crates/contracts/src/capability.rs` (new); `crates/contracts/devices.json` (new);
  `crates/contracts/src/lib.rs`; `crates/analysis/src/engine.rs` (`shot_measurements` calls
  `device_of`); `crates/contracts/tests/capability.rs` (new).
- **`devices.json`**: a `_comment` and per-entry provenance in the repo's committed-data style
  (ADR-022: a JSON artifact says where it came from). `hd_golf` declares its profile's targets (every
  `target` in `profiles.json`'s fields that is not null) plus `impact_position_v`; `club_head_speed`
  and `smash_factor` are `shown_only`, their note citing the smash measurement now in
  `shot_measure.rs` (P7). `attack_angle`, `dynamic_loft`, `low_point`, both `impact_offset_*` and
  `carry_offline` are not declared. `mock` declares what `mock.py` fills, all `analysed`
  (decision 4).
- **API** (§M32's names): `FieldUse { Analysed, ShownOnly }`; `DeviceCapability` with a note on every
  `ShownOnly`; `capability_for(device) -> Result<&DeviceCapability, ContractError>` refusing an
  unknown device; `device_of(shot) -> &str`; `printed_on(shot) -> Result<BTreeSet<&str>, ContractError>`
  = declared ∩ `fields_present`, with §M32's two special cases (a direct feed prints all it declares;
  `fields_present: None` on a screen shot reads as declared ∩ the fields holding a value);
  `printed_fields(shots) -> Result<BTreeSet<…>, ContractError>` = the union.
- **Tests** (`tests/capability.rs`): §M32's list in full — every declared field is a `ShotData` key
  (serialize a `ShotData` to get the key set); every `shown_only` has a non-empty note; per-shot
  intersection and cross-shot union; a layout lacking a tile excludes the field rather than blanking
  it; the unstamped and direct-feed rules; `hd_golf` never prints `low_point` or `attack_angle`; an
  unknown device is refused; `device_of` returns `hd_golf` on every corpus vector's `input.shot`
  (read the committed files, as `round_trip.rs` does).
- **Done when**: green, and the corpus `measurements` stage still conforms (that is what gates
  `shot_measurements`' source string now that it calls `device_of`).
- **Verify**: `cargo test`, clippy, fmt.

### P11 — Docs

- **Goal**: §M32 "Docs", plus what the build changed that §M32 could not know.
- **Read first**: `docs/CONFORMANCE.md` §1, §2 and §4; `docs/ARCHITECTURE.md` §1 "The commands,
  precisely"; ADR-032's last addendum (2026-09-30) and its thirteenth-addendum question; this file's
  findings P1–P10; `spec/README.md`.
- **Files**:
  - `docs/CONFORMANCE.md` — §1: the three Rust-owned schemas, who edits them, and the pin; §2:
    `provenance.oracle` and the `rerecords` ledger, and `spec/declarations/`; §4: rewritten for the
    `rerecord` verb, what `check` means now (the freeze held, not the vectors are right), and
    `regenerate`'s refusal. Re-measure any count you write (vector counts, family sizes).
  - `docs/ARCHITECTURE.md` §1 — `golf-core rerecord`, and `check`'s new meaning.
  - `docs/decisions/032-the-rust-core.md` — a **new addendum** (2026-10-0x, M32): the first Rust
    re-record; the ledger's form; the answer to the twelfth/thirteenth addenda's question (`oracle`
    stays `"python"`, `rerecords` names what is Rust's); what P1–P10 found. Update ADR-032's Status
    block if it states the version or the oracle. If the ADR counts its own addenda in words, update
    the word (`tests/test_docs_truth.py::test_an_adr_that_counts_its_own_addenda_counts_them_correctly`).
  - `docs/README.md` — ADR-032's row (its addendum count and a summary clause) and the "N addenda
    between them" total. **Not** the document count (rules, above).
  - `CLAUDE.md` — the commands block (`conformance.py check`'s comment: it now certifies the freeze;
    `golf-core rerecord`), the "How is a port checked" row, and the `ANALYSIS_VERSION` invariant if it
    still reads as future tense. Keep it free of counts.
  - `spec/README.md` — `declarations/`, and the ledger keys.
  - `docs/plans/m31-m40-shot-first-pivot.md` — under §M32, a short "As built" note routing to this
    file's findings, and the two corrections this plan made (calls 1 and 2). Do not rewrite §M32.
- **Done when**: docs-truth green, and every link added resolves.
- **Verify**: `pytest tests/test_docs_truth.py`, then `pytest`.

### P12 — Close: verify, WORKLOG, ROADMAP, memory, and the one commit

- **Goal**: close M32.
- **Files**: `WORKLOG.md` (a new top entry: what was built, the findings that matter to M34/M36, the
  verify numbers, what is next); `ROADMAP.md` (M32 row ✅ with the date and phase count, §M32's Status
  line; M34's and M36's rows unblocked); `docs/plans/m31-m40-shot-first-pivot.md` (its status
  checklist: M32 ✅, M34 and M36 ⬜ and unblocked); this file's checklist; `docs/README.md` (the
  document count and the per-folder breakdown, now that this file is tracked — `git add` it first,
  then run the count pin); the auto-memory entry `app-transition-decisions.md` (its "next is M32"
  becomes "next is M34/M36").
- **Verify, in order**:
  1. `cargo test`; `cargo clippy --all-targets`; `cargo fmt --check`;
  2. `pytest`; `ruff check src tests scripts`; `mypy src`;
  3. `conformance.py check` (frozen view);
  4. `golf-core rerecord --declare spec/declarations/v17.json` writes nothing;
  5. `git diff --stat -- src/` is empty (frozen Python untouched), and `contracts/swing.py` says 16;
  6. nothing under `data/` changed.
- **Then commit once, onto `main`**: `git add` the milestone's files by path (not `-A`; check
  `git status` for strays first), one commit with a message in the repo's style ("M32: …"), ending
  with the attribution lines the session's system reminder gives.
- **Done when**: committed, and the working tree is clean.

---

## Exit (§M32's, restated as checks)

- Every suite green; `conformance.py check` passes in the frozen view. *(P12 step 1–3)*
- On every re-recorded file, the report listed only declared differences; a second run writes
  nothing. *(P8, P12 step 4)*
- Every undeclared value is the committed Python value, and each file's `provenance.rerecords` names
  M32's declaration. *(P4's unit test, P8's spot check)*
- Frozen Python is untouched outside `scripts/conformance.py` and `tests/test_conformance.py`, and
  `contracts/swing.py` still says 16. *(P12 step 5)*
- The old exit's three screen-parser items are M34's, not M32's.

---

## Phase findings

*(Appended as phases close. Each phase writes what it found that the next phase should know —
especially anything that contradicts this plan.)*

### P0 — found (2026-10-01)

- **§M32's Python list missed one test**: `test_a_vector_round_trips_through_the_stdin_seam`
  (`tests/test_conformance.py:299`) compares frozen Python's output with a synthetic vector's
  `expected`, so it fails on the version move. Folded into P6 (call 2).
- **No committed vector carries `provenance.oracle` today** (checked 2026-10-01 across
  `spec/vectors/`). §M32's "`oracle` stays `"python"`" therefore means "is written as `"python"`"; P4
  sets it when absent.
- **The stage vectors carry no copy of the shot.** `flight.flown` holds the resolved launch and the
  integrated path, not a `ShotData` — so M32's declaration reaches no stage value, only the stage
  vector's top-level `analysis_version`, as §M32 says.
- **`FlownShot`, `ShotFlight` and `FlightResult` have no `Serialize`.** P2 builds the `flight` stage's
  JSON by hand in `core` rather than adding derives to `analysis`.
- **`serde_json`'s `float_roundtrip` is already on workspace-wide** (`Cargo.toml`, M22 P8), so a
  committed float read and written back by the re-record keeps its bits. The text still changes
  (exponent form, `\u` escapes); the value does not.
- **The four engine helpers `stages.rs` needs are private** (`crates/analysis/src/engine.rs` near
  `:857–:930`); decision 3 makes them `pub`.

### P1 — found (2026-10-01)

- **Landed as planned.** `crates/core/src/compare.rs` holds `Difference { path, kind }`,
  `DifferenceKind { AddedKey, RemovedKey, Moved { expected, actual } }`, `compare(expected, actual)`
  and `RTOL`/`ATOL` as `pub const`. `tests/engine.rs` lost its copy and the comparator test, and its
  whole-bundle gate still reports **21** vectors through the library comparator, zero differences.
- **A key difference's path is the path *of the key*** (`swing.spine_angle`), not of its parent as
  the old sentences printed it (`swing: key "spine_angle" …`). That is what lets P4 match a declared
  `added` path against `Difference.path` as one string. The sentences keep their phrases but not
  their exact form: `swing.spine_angle: key missing from the port` and `…: a key the vector does not
  have`. The root is `""` as a path and `<root>` only in `Display`.
- **The engine gate now prefixes each line with the vector id** (`{id}: {difference}`), because the
  library walk starts at `""` rather than at the id the old helper was seeded with.
- **A list-length difference is one `Moved` at the list's own path, carrying both whole lists**; no
  per-index differences past the shorter end. A *swapped* list is a `Moved` per index (two for
  `["a","b"]` against `["b","a"]`) — the comparator test asserts the count per case.
- **Int against float is not a difference when the values agree** (`3` recorded, `3.0` emitted). Both
  comparators read an int as exact only when *both* sides are ints (`compare_results`'
  `isinstance(expected, int) and isinstance(actual, int)`); the old Rust comment claimed the kind came
  off the recorded value alone, and is corrected. For P4 this means such a pair applies nothing and
  the committed int survives, which is the behaviour step 4 wants.
- **Two docs now describe the old location, left for P11** (not in P1's file list):
  `docs/CONFORMANCE.md` §4 (`:479`, "`crates/core/tests/engine.rs` is §3's rules implemented for the
  Rust side") and `crates/core/src/bin/golf_core.rs`'s module doc (`:18`, which P5 rewrites anyway).
  `crates/core/src/lib.rs`' module doc was updated here.
- **Verify**: `cargo test` green workspace-wide; `cargo clippy --all-targets` clean; `cargo fmt
  --check` clean. `git status spec/` clean.

### P2 — found (2026-10-01)

- **Landed as planned, zero differences on the first run.** `crates/core/src/stages.rs` holds
  `run_stages(input: &VectorInput) -> Value`, plus `STAGE_NAMES` and `MEASUREMENT_GROUPS` as `pub
  const` arrays (P3/P4 can use them rather than retyping seven names). `crates/core/tests/stages.rs`
  joins each stage vector to its engine vector by `provenance.derived_from` (`.json`, then
  `.json.gz`), runs the engine input through `run_stages`, and compares the whole `stages` object
  through `compare::compare`: **21** stage vectors (6 synthetic + 15 corpus, counted), zero
  differences, ~2.7 s in a debug build. It also asserts the document's keys are exactly
  `STAGE_NAMES`, and each vector's `analysis_version` against `ANALYSIS_VERSION` (the form
  `tests/engine.rs` uses — P8's re-record moves both together, so it passes unchanged after P8).
- **A second test pins that the gate can see a forgotten window**: `synthetic/windowed` with its
  `face_on_window` dropped must move a `phases[…]` path. It is the only vector with a window, so
  without this the zero above says nothing about windowing.
- **The four engine helpers are `pub`** (`windowed`, `shifted`, `camera_id`, `anchored_on_strike`),
  each with a doc line naming its `crates/core` caller. `shifted` has none yet: P3's compose check is
  it, as `_verify_stages_compose` is `E._shifted`'s. Clippy does not flag an unused `pub fn`.
- **`partition` measures all five cuts** (Python's `pose` is the remainder) and asserts they sum to
  `measurements`' length, so Rust's assert is a real check where Python's is true by construction.
  The shot cut is `shot_measurements(shot).len()`, as the plan said — no device rule in `stages.rs`.
- **A scored checkpoint row is written with `reason: null, detail: ""`**, because Rust's
  `CheckpointOutcome::Scored` carries no detail and Python's `CheckpointOutcome.scored()` leaves both
  at their defaults. Checked across all 21: no scored row has a detail, and exactly one row in the set
  is unscored (`synthetic/no-handedness`, `head_stays_back`, `no_handedness`).
- **The `flight` stage is nine small `*_json` functions** over `FlownShot` → `ShotFlight` →
  `InferredSpin`/`InferredSpinAxis` → `SpinSolution` → `CarryWindow`/`UnspunLaunch`, and
  `FlightResult` → `LaunchConditions`/`FlightPoint`. Every field is written, so a field left out
  would be a `RemovedKey`, not a silent thinning. Enums go out as `as_str` (the three analysis-side
  ones) or serde (`UnscoredReason`, `TargetShape`). No serde derive was added to `analysis`.
- **Not every number is bit-identical, and that matters for P4 and P8.** Census over the 21 stage
  documents (measured, not committed): 286,343 numbers, **142** within tolerance but not
  bit-identical, on every vector (3–11 each). By path: **133** in `measure[].value` (the unrounded
  pose numbers — reassociation in the last bits), **8** in `checkpoints[].score.score`, **1** in a
  `flight…solution.window.launch.launch_direction_deg`. `smoothed`, every `measurements` row (rounded
  to four places) and every flight point are bit-exact. So P4's `apply` rule — undeclared values keep
  the committed bits — is not academic: a re-record that wrote Rust's own document would churn ~142
  floats in the stage family alone. P5's empty-declaration run still reports nothing, because the
  gate compares under §3's tolerance.
- **Verify**: `cargo test` green workspace-wide; `cargo clippy --all-targets` clean; `cargo fmt
  --check` clean; `cargo doc -p golf-core --no-deps` warning-free. `git status spec/` clean.

### P3 — found (2026-10-01)

- **Landed as planned.** `crates/core/src/stages.rs` holds `verify_compose(stages: &Value,
  engine_vector: &Value) -> Result<(), String>`, `_verify_stages_compose` check for check and in its
  order: `smoothed` pass-through (frame counts, index/timestamp, `camera_id` dropped, `z`/`visibility`
  copied), `phases` through `engine::shifted` (its first caller, as P2 said), `measure` through
  `pyfmt::round_to(v, 4)` onto the `pose` rows, the groups concatenated onto `measurements`,
  `checkpoints` by containment with the `cross_view_contradicted` allowance, `clip_alignment` onto
  `expected.alignment`, and each `flight.unscored` entry onto its `unscored` row. `engine_vector` is
  the whole vector (`input` for the clip and window, `expected` for the answer), so P5 can pass the
  document as it will be written (call 9).
- **Every refusal names its stage, in one form**: ``{id}: the `{stage}` stage does not compose onto
  `expected` — {first difference}``, with the difference's path prefixed the way `compare_results`'
  third argument prefixes it (`measure.tempo_ratio: 2.7273 != 2.7373`) and `(and N more)` when there
  are several. Python's `assert`s carried free-form text that did not always name the stage; the port
  routes them through the same formatter, which is what P3's "naming the stage" test pins.
- **Only the clip and its window are parsed out of `input`**, as the Python parses only `face_on`
  and the window — so the compose check never depends on the shot parsing, which matters in P8
  when `ShotData` gains keys the committed inputs lack.
- **A contract type read from a borrowed `Value` must go through the trait.** `#[serde(remote =
  "Self")]` leaves an *inherent* `T::deserialize` that skips `Validate`, and method resolution prefers
  it at a concrete call site. `stages.rs::parse<T: DeserializeOwned>` reaches the trait method through
  a type parameter. P4/P5 should reuse that shape, or `serde_json::from_value`, if they parse a
  contract type — never `PhaseSegment::deserialize(&value)` directly.
- **Branch coverage on the committed set** (measured): all 21 compose. 11 carry `flight.unscored`
  entries, 15 a `clip_alignment`, 1 a checkpoint refusal (`synthetic/no-handedness`), and **none** uses
  the `cross_view_contradicted` allowance — that branch is reached by no committed vector, in Rust or
  in Python.
- **Tests** (`crates/core/tests/stages.rs`): every committed stage vector composes onto its engine
  vector's committed `expected`; and on `synthetic/windowed`, a phase boundary, an unrounded pose
  value and a checkpoint score each moved by hand are refused under `phases`, `measure` and
  `checkpoints` respectively. The same test asserts the windowed phases do *not* land unshifted, so
  the shift stays under test.
- **Verify**: `cargo test` green workspace-wide; `cargo clippy --all-targets` clean; `cargo fmt
  --check` clean; `cargo doc -p golf-core --no-deps` warning-free. `git status spec/` clean.

### P4 — found (2026-10-01)

- **Landed as planned, with paths typed.** `crates/core/src/rerecord.rs` holds `Declaration`,
  `gate`, `apply`, `ledger`, `Applied` and `Undeclared`, plus `RERECORDED_BY` (`"golf-core
  rerecord"`) and `PYTHON_ORACLE` (`"python"`). One departure from the shape above: `Declaration`'s
  `added` and `moved` are `Vec<LedgerPath>`, not `Vec<String>` — a `LedgerPath` deserializes from a
  string through `LedgerPath::parse`, so a declaration with a bad path does not load at all, and the
  gate and `apply` never meet an unparsed one. `LedgerPath` exposes `parse`, `as_str`, `segments`
  and `get`; P5's report can read a declared path's value out of either document with `get`.
- **A declaration loads with `serde_json::from_str::<Declaration>`**, and refuses: an unknown key
  (a misspelled `"moves"` would otherwise declare nothing), a missing one (all four are required,
  `note` included), a path declared twice in or across the lists, and an `added` path ending in a
  list index (the comparator reports an added key at the key, so it could never match). Path
  refusals: `""`, a leading dot (Python's spelling), `[*]` (call 4), an empty step, a step not
  starting with a key, a non-numeric index — and **anything that does not re-render to itself**,
  which is the check that catches `[01]` and `[+1]` (`usize`'s parser takes both). P6's Python
  parser should accept exactly this grammar: `key(.key|[n])*`.
- **Matching is exact, at the difference's own path.** A key added whole at a parent (a shot whose
  `provenance` object is itself new) would surface at the parent and fail against a child
  declaration. M32 is safe from that: all 15 corpus vectors' `expected.swing.shot.provenance` are
  objects (checked), so the three provenance keys surface at the declared paths.
- **A declared `moved` path whose value moved only inside the tolerance matches nothing**, because
  `compare` reports no difference there; the committed value stays. Under call 5 that entry fails
  the run if it matches nowhere else — the right answer (a declared move that did not happen), but
  P5's message for call 5 should say "moved within tolerance, or not at all", not just "typo".
- **`Applied` can only come from `gate`** (no public constructor), and holds the matched entries in
  the *declaration's* order, not the walk's — so a ledger entry reads in the order the reviewer wrote
  them. `Applied::is_empty` is step 5's signal; `ledger` refuses an empty one, and refuses a document
  with no `provenance` object or a `rerecords` that is not a list, writing nothing when it refuses.
  All 42 engine and stage vectors carry a `provenance` object (checked).
- **For P5**: `ledger` records `declaration_path` verbatim. The verb must spell it repo-relative with
  forward slashes (`spec/declarations/v17.json`) whatever `--declare` was given, or a Windows
  absolute path lands in 42 files. Calls 5 (typo guard, the union of each document's `Applied`) and
  6 (version guard) are the verb's, not here.
- **No committed engine or stage document has a key holding `.`, `[`, `]`, or an empty key**
  (scanned all 42), so the path spelling is unambiguous on every document M32 re-records; the
  module doc says where an escape would go if one ever does.
- **Tests**: 13 unit tests in `rerecord.rs` — each listed in P4's "Tests" above, plus "every
  refusal is named at once", the path grammar both ways (every path `compare` writes parses back to
  itself), the declaration-level refusals, and a refused ledger leaving the document untouched. The
  bit-for-bit test asserts its two floats differ in bits first, so it cannot pass vacuously.
- **Verify**: `cargo test` green workspace-wide; `cargo clippy --all-targets` clean; `cargo fmt
  --check` clean; `cargo doc -p golf-core --no-deps` warning-free. `git status spec/` clean.

### P5 — found (2026-10-01)

- **Landed as planned.** `golf-core rerecord --declare <file> [--dry-run] [--spec <dir>]`. The run is
  library code in `crates/core/src/rerecord.rs`' second half: `plan(spec, declaration) -> Result<Run,
  Refused>` does everything but write (read, run, gate, apply, ledger, compose, guards), `Run::write`
  writes the changed vectors, and `Run::report(dry_run, written)` is the review. `Refused` is
  `Declaration` (load, version guard, placement), `Files` (a broken `spec/`: unreadable, wrong
  `provenance.kind` for its directory, no `derived_from`, a duplicate id — these stop the walk at
  once), `Gate` (every undeclared difference and every compose failure across the run), `Unmatched`
  (call 5), `Write`. The binary only parses flags and prints; `--spec` defaults to the repo's
  `spec/` found from `CARGO_MANIFEST_DIR`, not the working directory, as `conformance.py` finds
  `REPO` from `__file__`.
- **The real-spec run (§M32 Sequence step 1)**: an empty declaration at v16 in the session
  scratchpad, `cargo run --release --bin golf-core -- rerecord --declare <scratch>/p5-empty.json`,
  reported `nothing differs from the committed vectors` and `42 vectors run (21 engine, 21 stage);
  0 changed; 0 files written`, exit 0, ~1.3 s in release. `git status spec/` clean.
- **The typo guard and "a second run writes nothing" contradicted each other as written, and are
  reconciled through the ledger.** On a second run nothing differs, so no declared path matches, and
  call 5 would fail the run — making P8 step 4 and P12 step 4 exit non-zero. So a declared path also
  counts as matched if some committed document's `provenance.rerecords` already lists it, under the
  same kind, in an entry at the declaration's `analysis_version` `by: "golf-core rerecord"`. A
  misspelled path cannot be in a ledger (a path only reaches one by matching), so the guard still
  catches typos. The entry is matched on version and `by`, not on the `declaration` string, which
  depends on where the verb was run from. Pinned by the slice test's second run (exit 0, 0 written).
- **Line endings are the committed file's, and every plain `.json` vector is CRLF.** `git ls-files
  --eol spec/vectors`: all 17 plain files are `i/crlf` (`_write_json`'s `write_text` translates on
  Windows, and `spec/** -text` stores the bytes), every gzipped one is LF inside (a binary stream).
  The writer keeps whichever the committed text has, so P8 will not rewrite every line of the twelve
  synthetic files. The gzip header matches `GzipFile`'s (FNAME = the name less `.gz`, mtime 0, XFL 2,
  OS 255 — pinned by a unit test against a committed header's bytes); the deflate stream does not
  (`flate2`'s backend is miniz_oxide, not zlib), and nothing reads it.
- **Churn, measured on a scratch copy** (aged `synthetic/baseline-3to1` and its stage twin to v15,
  declared the move, wrote, diffed against the committed v16 text): beyond the ledger, the engine
  file differed in **3** lines, each a `—` that `serde_json` writes raw, and the stage file in
  none. Corpus churn (exponent floats) is unmeasured; P8's report, not its diff, is the review.
- **Where a declaration may sit.** The ledger spells it relative to the directory holding `spec/`,
  with `/` (P4's finding). A declaration outside that directory can drive a run that writes nothing
  (P5's own run, from the scratchpad) and is refused if anything would be written; one under
  `spec/vectors/` is refused outright (decision 2). P8 runs from the repo root with
  `--declare spec/declarations/v17.json`.
- **Writes are staged, then renamed.** Every changed file is rendered, written beside its target as
  `<name>.rerecord-staged`, and only then renamed over it; a failure while staging removes the
  staged files and leaves `spec/` untouched. Neither the Rust walk (`.json`/`.json.gz` names) nor
  `conformance.py::vector_paths` (suffix `.json`/`.gz`) would read a left-over staged file.
- **Compose runs against the engine document as it will be written** (call 9), and is skipped for a
  stage vector whose engine vector the gate refused — the run fails on that refusal anyway.
- **Tests**: 7 in `crates/core/tests/rerecord.rs`, each on its own copy of one synthetic and one
  corpus engine vector and their stage twins (the smallest of each half) under
  `%TEMP%/golf-core-rerecord-<pid>-<test>/repo/spec`, removed on drop: an undeclared difference fails
  through the binary and leaves every byte (the declared synthetic pair's included); a declared
  change lands, reads back as planned, carries per-file ledgers, keeps every undeclared value, and a
  second run writes nothing; `--dry-run` reports and writes nothing; an unmatched entry fails; a
  declaration at `ANALYSIS_VERSION + 1` is refused before any vector is read; placement; bad flags.
  Every edit is relative to `ANALYSIS_VERSION`, so they mean the same after P8. Plus two unit tests in
  `rerecord.rs` (file bytes; report values cut on a character boundary).
- **Module docs**: `golf_core.rs`' is rewritten (the "`regenerate` is deliberately not a candidate"
  paragraph is now why a gated re-record is not a self-portrait, and P1's stale `:18` sentence is
  fixed); `lib.rs`' first line and its re-record and `ANALYSIS_VERSION` sections say the crate now
  records. `flate2` is a `[dependencies]` entry with its reason. **Left for P11**:
  `docs/CONFORMANCE.md` §4 still names `tests/engine.rs` as where §3's rules live (P1's finding) and
  does not yet describe the verb.
- **Verify**: `cargo test` green workspace-wide; `cargo clippy --all-targets` clean; `cargo fmt
  --check` clean; `cargo doc -p golf-core --no-deps` warning-free. `git status spec/` clean, and no
  test directory left under `%TEMP%`.

### P6 — found (2026-10-01)

- **Landed as planned, in the two files only.** `scripts/conformance.py` gained
  `RUST_OWNED_SCHEMAS` (`write_schemas` skips it; `SCHEMA_ROOTS` keeps all ten), `parse_ledger_path`,
  `frozen_view`, `ledger_covers` and `_staleness` (the STALE sentence, shared by `check` and both
  version pins). `cmd_check` compares through `frozen_view` and judges versions by `ledger_covers`.
  `cmd_regenerate` refuses the full rebuild and `--stages-only`. `tests/test_conformance.py` re-points
  the four pins the plan names. `src/` and `conformance_vectors.py` are untouched.
- **`frozen_view` finds the answer by the vector's kind**: `stages` on a stage vector, `expected`
  otherwise. The plan's signature had no way to say which, and the stage pin needs it. So a ledger
  path is taken out only when its first step is that key; a top-level path (`analysis_version`) is
  left alone, because the version rule is `ledger_covers`'.
- **With no ledger, the view returns the vector's own objects, uncopied**: identity, not just
  equality (pinned). With a ledger it deep-copies both sides first, so a check run later on the
  same vector still sees the declared values. Absent paths are skipped on either side, including a
  `None` on the way down (a synthetic `swing.shot`). **A declared list element is blanked to `None`
  on both sides, not popped**: popping would shift every later index onto the wrong partner. M32
  declares no index path; the rule is there because Rust's grammar allows a `moved` one.
- **The path grammar is Rust's, by regex**: `[^.\[\]]+(?:\.[^.\[\]]+|\[(?:0|[1-9][0-9]*)\])*`.
  It refuses what `LedgerPath::parse` refuses: `""`, a leading dot, `[*]`, `a..b`, a trailing
  dot, `[01]`, `[+1]`, a leading index, text after an index. A ledger path in any other spelling
  raises in `frozen_view`. It does not quietly remove nothing.
- **`ledger_covers(v, version)`** is true at `version`. Above it, it needs an entry whose
  `analysis_version` is each of `version+1 ..= stated`. It does not check `by`, because the plan's
  rule names only the version. Below `version` it is false.
- **`regenerate`'s refusal** exits **2**, on stderr, before anything is imported or written,
  schemas included. The message names `golf-core rerecord`, ADR-035 clause 3 and the verb's
  command line. `--stages-only` is refused even beside `--format-only` (the stages branch used to
  come first too). `--schemas-only` and `--format-only` behave as before, except that the schemas
  skip the three Rust-owned roots. The refusal tests replace `conformance_vectors` in
  `sys.modules` with recorders, as well as `_write_json` and `write_schemas`. So even a refusal that
  regressed could not read `data/processed/` or write `spec/`. The `--schemas-only` test writes to
  `tmp_path` and never to `spec/`.
- **`check`'s lines now read**: `21/21 vectors hold the freeze (frozen engine v16; 0 re-recorded by
  Rust) — `cargo test` certifies them` and `21/21 stage vectors are at frozen v16 or ledgered above
  it — `cargo test` runs them`. The audio and format lines are unchanged. Exit 0. After P8 the
  first line's count should read 21 re-recorded. That is P8's quick check that every engine vector
  took a ledger.
- **Two stale "run `regenerate`" messages in P6's own files were fixed too**. One is
  `cmd_check`'s empty-`spec/` message. The other is `test_every_engine_vector_has_a_stage_vector`'s,
  which now says a new vector and its stages are §M29's Rust vector builder's. **No command creates
  a stage vector from M32**: `rerecord` re-records only the ones that exist.
- **Left for P11** (outside P6's files, or Rust): `scripts/conformance_vectors.py:3` and `:442`
  still name `regenerate`/`--stages-only` as the entry point. They are frozen Python, so a note in
  `docs/CONFORMANCE.md` is the fix, not an edit. Also `docs/CONFORMANCE.md` `:304`, `:453`,
  `:495`; `spec/README.md:55`; ADR-032 `:168` (history, probably left); and
  `crates/core/tests/engine.rs:122`'s assert message ("regenerate the vectors" — P8 or P11, to
  "re-record"). Three Rust test `expect`s say "run `conformance.py regenerate`" when
  `spec/vectors` is missing: `crates/{trigger/tests/conformance.rs:105,
  contracts/tests/round_trip.rs:144, pose/tests/writer.rs:38}`.
- **Tests**: 24 new in `tests/test_conformance.py` (the module is 214, from 190). The view takes out
  a declared move and a declared added key. It still sees an undeclared one. It edits nothing in
  place, and with no ledger it is identity. It reads a stage vector through `stages`, and it blanks
  a list element without misaligning the rest. The grammar is tested both ways (4 accepted, 9
  refused). `ledger_covers` is tested at, above with and without a ledger, two above with one entry,
  and below. `regenerate` refuses with no flag and with `--stages-only`, and still does
  `--schemas-only` and `--format-only`. The `--schemas-only` writer skips exactly
  `RUST_OWNED_SCHEMAS`, which must be a subset of `SCHEMA_ROOTS`.
- **Verify**: `pytest` 2015 passed; `ruff check src tests scripts` clean; `conformance.py check` exit
  0, v16; `git status spec/ src/` clean. No Rust was touched, so `cargo` was not re-run.

### P7 — found (2026-10-01)

- **Landed as planned; no serialized output moved.** `UnscoredReason` gained `PrintedBlank`
  (`printed_blank`, `refilming_helps: false`) and `Misread` (`misread`, `true`), each with a row in
  `UNSCORED_REASONS` and a doc saying it is photo-side, never written into a `SwingResult` (ADR-035
  clause 4), and first emitted by M35/M37 into `shot_analysis.json`. Neither is in
  `INFERENCE_REASONS`. The module doc gained a section saying both are Rust's alone, and
  `ReasonSpec::refilming_helps`' doc now says "the clip, or for `Misread` the photo", keeping the
  field's name because consumers already read it by that name.
- **The two rows come last, after `Unrecorded`, in the enum and the table.** That keeps Python's rows
  an unbroken prefix in the dict's order, and the table's doc says so. **For P9 (call 13):** the
  schema enum is the Rust wire-name list without its last two entries. P9 must still pin that by
  name, not by position.
- **No `match` on `UnscoredReason` exists outside `unscored.rs`.** `cargo build` flagged nothing.
  `feedback::rules` reaches the table through `spec()` and `is_inference_reason`, and the analysis
  crates only construct reasons. So no arm was added anywhere.
- **Tests**: `every_reason` and the wire-name pairs gained both. The wire-name test's doc says the
  last two have no Python twin, by design, and are pinned because `shot_analysis.json` will carry
  them. `refilming_helps_exactly_where_the_clip_is_the_problem` gained `Misread`, and its doc says
  "the clip or the photo". `every_reason_has_a_row` and `the_inference_family_is_five_rows_of_this_table`
  are unchanged in code and pass.
- **The plan's "`BundleRequest.shot`" is `SwingRequest.shot`.** `engine.rs:478–479` is the doc on
  `SwingRequest::shot`; `BundleRequest::shot` had no doc. The stale text was rewritten where it
  stood, to ADR-034 §5: never banded, graded per club over many shots, `outcome_score` stays `None`.
  `BundleRequest::shot` gained one line pointing at it. **One more stale comment in the same file
  was fixed**: `analyze_swing`'s `// Pose-only: no outcome checkpoints yet (needs M2 detection / M3
  shot data)` now says the empty outcome list is by decision (ADR-034 §5), not by schedule.
- **`shot_measure.rs` now holds the measurement.** The module doc's pointer to the Python docstring
  became a section, "What stays on `ShotData`", with M31 P2's 0.76–1.06 over the 13 stored shots (8
  below 1.00, 2 at, 3 above), the consistency-check argument, and one line each on `spin_axis` and
  dispersion, so nothing points at a docstring M40 deletes. `measure_ball_speed`'s doc carries the
  2026-08-10 pair's numbers (90.7/90.5 mph ball speed, 125.6/121.0 yd carry, 91.0/98.3 mph club
  speed), taken from the Python docstring that M31 P2 finding 9 attributes to that pair. No `data/`
  was read to re-check them.
- **`intent.rs`'s five "full M4"s are gone.** `PracticeMode`'s doc says only `Fundamentals` has a
  single-swing policy and the rest stay unbuilt (ADR-034 §5). `ShotShaping` is judged in M37's shape
  topics (§5.3). `Performance` is graded per club with no tour band (§5.6). `Drill` is not tracked,
  derived from the mode and never stored (§3, M35). `TargetShape` chooses the shape topic.
- **Still saying "full M4", outside P7's files, left for P11 or later**:
  `crates/analysis/src/scoring.rs:94`, `policy_for`'s panic message ("scoring policy for {mode:?}
  lands in full M4"), and its `#[should_panic(expected = "lands in full M4")]` test at `:160`. It is a
  panic string rather than serialized output, so changing it moves no vector, but it needs the test
  edited in the same change. `engine.rs:966`'s "Today the outcome list is empty … so it stays right
  when it is not" is true as a robustness note and was left alone.
- **Line endings**: `sed -i` under Git Bash rewrote `engine.rs`'s working copy to LF, and it was put
  back to CRLF. The index is LF either way, so the commit is unaffected. Later phases should prefer
  the Edit tool or Windows Python for in-place edits.
- **Verify**: `cargo test` green workspace-wide, including every vector gate (`core/tests/{engine,
  stages,rerecord}.rs`, `contracts/tests/round_trip.rs`, the analysis stage tests), which proves no
  output moved. `cargo clippy --all-targets` is clean and `cargo fmt --check` is clean. `cargo doc -p
  contracts -p analysis --no-deps` raises no new warnings; its existing private-link warnings are
  unchanged. `git status spec/` is clean.

### P8 — found (2026-10-01)

- **Landed as planned; the first Rust re-record is in the working tree.** `ShotData` gained the
  seven keys and `ShotProvenance` the three, each `#[serde(default)]`; `parser_version` is refused
  below 0 in `Validate`. `SCREEN_PARSER_VERSION = 1` (entry 1 is M34's, nothing stamps it yet) and
  `parse_is_current` sit in `shot.rs` (call 14). `ANALYSIS_VERSION` is 17, with its `16 -> 17` entry
  in `swing.rs`' doc and the doc's "deliberately not copied" paragraph rewritten to say where each
  half of the history lives (call 12). The pin is `swing::tests::every_version_from_seventeen_has_a_ledger_entry`.
  `spec/declarations/v17.json` is §M32's declaration verbatim, plus a `note`. `contracts/swing.py`
  still says 16.
- **`ShotProvenance` has no `skip_serializing_if`, so all three provenance keys are always written**:
  each corpus answer now carries `parser_version: 0`, `fields_present: null` and `corrections: {}`,
  and the seven shot keys as `null`. Rust writes those values, not Python, and the ledger says so.
- **The run, in totals.** The dry run reported only declared paths, on every vector: **150 added**
  (the ten keys × 15 corpus engine vectors) and **63 moved** (`analysis_version` on all 42,
  `expected.analysis_version` on the 21 engine vectors). Result: `42 vectors run (21 engine, 21
  stage); 42 changed; 0 files written`, exit 0. The real run wrote 42 files and left no
  `.rerecord-staged` file behind. The second run printed `nothing differs from the committed
  vectors … 0 changed; 0 files written`, exit 0. Each file has exactly one ledger entry, per call 8:
  a synthetic engine vector `added: []` with both moves, a corpus one the ten with both moves, and a
  stage vector `moved: ["analysis_version"]` alone. The text churn in the twelve plain synthetic files
  is 210+/60− lines. That is the ledger, the version, and the `—` escapes P5 measured, now
  written raw. The corpus files are binary to `git diff`.
- **The second-language check covered all 42 files, not just one.** Windows Python loaded each file
  and its `git show HEAD:` twin. From the new file it removed the ten declared keys, put both
  versions back to 16 (after asserting 16 → 17), and removed `provenance.oracle` (asserted
  `"python"`) and the single `rerecords` entry. It then compared the two strictly: the same type at
  every node, so an int is not equal to a float, and every float equal by `float.hex()`. The result
  was **0 differences** across the 42, and 150 declared keys removed. So every undeclared value is
  the committed Python value, bit for bit.
- **The `round_trip.rs` allowance is matched by owner and value, not by prefix.** `DECLARED_BY_THE_ANSWER`
  names two exact owners: `input.shot` ↔ `expected.swing.shot`, and `input.shot.provenance` ↔
  `expected.swing.shot.provenance`. An invented input key passes only if two things hold. First,
  its *parent* is one of those owners. Second, its twin is an `added` path in this vector's own
  `rerecords` **and** the value equals the committed twin's value. The value is compared with the
  committed file rather than with today's default, so a default changed later fails here. Two new
  tests pin it. The first shows the allowance holds for a ledgered key at its recorded value, and
  refuses an unledgered key, a wrong value, a foreign owner and a vector with no ledger. The second
  takes a real corpus vector's ledger away and gets back exactly the declared keys as invented ones,
  read off the ledger rather than spelled out in the test.
- **Pins that passed unchanged**: the version asserts in `core/tests/engine.rs`,
  `analysis/tests/{geometry,measurements,judging,alignment}.rs` and `round_trip.rs`; the stage and
  compose tests; the 7 `core/tests/rerecord.rs` tests (relative to `ANALYSIS_VERSION`, as P5 said);
  and P6's Python pins, now on real ledgers. `conformance.py check` reads `21/21 vectors hold the
  freeze (frozen engine v16; 21 re-recorded by Rust)` and `21/21 stage vectors are at frozen v16 or
  ledgered above it`, as P6 predicted.
- **Left for P11, deliberately**: the "regenerate" wording in the Rust test messages. These are the
  version-assert messages at `core/tests/engine.rs:122`,
  `analysis/tests/{alignment:517,geometry:208,judging:265,measurements:268}.rs` and
  `contracts/tests/round_trip.rs:460`, plus the "`spec/vectors` is missing — run `regenerate`"
  expects at `round_trip.rs:149`, `trigger/tests/conformance.rs:105` and `pose/tests/writer.rs:38`.
  This phase's rule was that those pins pass *unchanged*, so their text was not touched. The right
  wording is "re-record with `golf-core rerecord`" for a version mismatch, and "it is committed —
  restore it" for a missing `spec/`, because `rerecord` cannot create a vector.
- **For P9**: the committed `spec/schemas/{shot_data,swing_result,swing_bundle_result}.schema.json`
  still describe frozen Python's `ShotData`, without the ten keys, while every corpus `expected` now
  carries them. Those are P9's hand-maintained Rust-owned schemas. Nothing validates a vector against
  them today, which is why nothing went red.
- **Line endings, again.** After the edits and `cargo fmt`, `shot.rs` and `round_trip.rs` were
  LF in the working tree. Windows Python put them back to CRLF, and `cargo fmt --check` is still
  clean. Git Bash's `grep -c $'\r$'` reported them as CRLF when they were not, so check with a byte
  count (`re.findall(rb'(?<!\r)\n', …)`). `spec/declarations/v17.json` is hand-written and LF, like
  `spec/README.md`. The tool-written vectors keep their CRLF, as P5 built.
- **Verify**: `cargo test --workspace --no-fail-fast` green; `cargo clippy --all-targets` 0
  warnings; `cargo fmt --check` clean; `cargo doc -p contracts --no-deps` shows only the two existing
  warnings. `pytest` 2015 passed; `conformance.py check` exit 0. `git status` shows no change under
  `spec/vectors/{format,audio}/`, `src/` or `data/`.

### P9 — found (2026-10-01)

- **Landed as planned: pure insertions.** Each of `spec/schemas/{shot_data,swing_result,
  swing_bundle_result}.schema.json` gained the seven `ShotData` and three `ShotProvenance`
  properties, 122 added lines each and nothing else changed. The two swing files already carried
  `UnscoredReason` at frozen Python's set, so call 13 needed no edit, only the pin.
- **The spelling is pydantic's own, not imitated.** A throwaway scratch script, not committed,
  declared the ten fields on a pydantic model with the Rust docs as `description`s and took
  `model_json_schema()["properties"]`. It merged those into the three files and wrote them in
  `export_schemas`' format: indent 2, sorted keys, ASCII escapes, a trailing newline, and the CRLF
  the files carry under `spec/** -text`. Before the edit, that format re-serialized all three
  committed files byte-for-byte. `parser_version` came out `{"default": 0, "minimum": 0, "type":
  "integer"}`. `fields_present` is a nullable string array with `default: null`. `corrections` is
  spelled exactly as `raw_fields`, with no `default`, because pydantic emits none for a
  `default_factory`. **For a later hand edit**: generate a property the same way, from a throwaway
  model, rather than typing it.
- **The descriptions** follow each shape's own convention. `ShotData`'s are terse unit strings with
  no backticks ("millimetres, + = toe"). `ShotProvenance`'s are sentences ending in a period.
  `impact_position_v` reads "the vertical partner of impact_position, in the source's own words".
  The Rust doc's "for that field's reason above" pointed into the source file, so it was left out.
- **The pin, `crates/contracts/tests/schemas.rs`, is four tests:**
  1. In each of the three files, the property sets of `ShotData` and `ShotProvenance` equal the
     keys a minimal serialized Rust shot writes. Both directions are reported, every file in one
     report.
  2. **Beyond the plan:** the three copies of each shape are identical, with the root's `$defs`
     set aside. Pydantic wrote one model into all three files; a hand edit is made three times, and
     the key-set pin cannot see a description fixed in one file only. A failure names the part
     that differs, such as `["description", "properties.low_point"]`, rather than printing both
     shapes.
  3. The swing files' `UnscoredReason` equals `UNSCORED_REASONS`' wire names minus `PrintedBlank`
     and `Misread`. They are compared by name as sets, and the two are named as variants, so a
     rename is a compile error.
  4. Every key M32 added carries a non-empty `description`. **The keys are read from
     `spec/declarations/v17.json`'s `added`, not listed again**, by stripping
     `expected.swing.shot.provenance.` or `expected.swing.shot.`. So the declaration is now
     load-bearing for a test as well as for the verb, and P11's `spec/README.md` text should say so.
- **Each test was seen to fail.** With the three files restored from `HEAD`, tests 1 and 4 failed,
  naming all ten keys in all three files. With an invented `ShotProvenance` property, a changed
  `low_point` description in one file and `misread` added to one enum, tests 1, 2 and 3 failed. The
  files were then restored and checked by sha256.
- **For P11 (CONFORMANCE §1)**: the Rust list `RUST_OWNED` in `schemas.rs` repeats
  `conformance.py::RUST_OWNED_SCHEMAS`, and nothing checks one against the other. The Python test
  refuses a name that is not a root, and the Rust doc says that a root added to only one list is
  either unpinned or pinned twice. Say in §1 that both lists must change together when M36 takes
  more roots.
- **Verify**: `cargo test --workspace --no-fail-fast` green, with the new 4 included; `cargo clippy
  --all-targets` 0 warnings; `cargo fmt --check` clean. `pytest tests/test_conformance.py` 214
  passed, including `test_every_committed_schema_is_what_contracts_exports_today` and
  `test_every_on_disk_artifact_has_a_schema`. Nothing under `spec/vectors/`, `src/` or `data/` was
  written by this phase.

### P10 — found (2026-10-01)

- **`devices.json` was already in the working tree when this session opened.** An earlier attempt at
  P10 had written it and stopped before `capability.rs`. It was checked against its sources rather
  than rewritten, and kept as found. `hd_golf` lists every non-null `target` in `profiles.json`, in
  tile order, plus `impact_position_v`. `club_head_speed` and `smash_factor` are `shown_only`, each
  note citing `shot_measure.rs`' module doc (P7) and M31 P2 finding 1. `mock` lists the twelve fields
  `mock.py` fills, in its argument order, all `analysed` (decision 4). It has a `_comment`, and each
  entry has a `source` and an `added`.
- **`capability.rs` landed with §M32's API.** It has `FieldUse { Analysed, ShownOnly }`,
  `DeviceCapability { device, source, fields }` and `capability_for`, `device_of`, `printed_on` and
  `printed_fields`. Beyond the plan's names, and needed by them:
  - `DeclaredField { field, field_use, note }`. `field_use` is `"use"` on disk, which is a keyword.
  - `DeviceCapability::{declared, use_of}`.
  - `devices()`, a `OnceLock` parse, which is `store.rs`' pattern.

  `added` is on disk and not read, like `BenchmarkRange.added`. The returns are `&'static`, as
  `capability_for -> Result<&'static DeviceCapability, _>` and `printed_* -> BTreeSet<&'static str>`,
  because the names come from the parsed file. `printed_fields` takes any
  `IntoIterator<Item = &ShotData>`.
- **Calls this phase made:**
  1. **`note` is `Option<String>` on every field.** It is required on `ShownOnly` by a test, not by
     the type. `devices.json`'s `impact_position_v` is `analysed` *with* a note: it is declared, and
     no profile locates it before M34. So a `ShownOnly { note }` variant would not have fit the file.
  2. **"The fields it holds a value for"**, the unstamped rule, is read off the serialized shot as
     every non-null key. It is not a hand-written match over field names, so the struct stays the
     only list. A NaN metric therefore reads as holding nothing.
  3. **One undeclared device refuses the whole `printed_fields` set.** It does not skip that shot.
  4. **`printed_on` ignores `FieldUse`.** A `shown_only` field is printed and is displayed. Whether
     analysis may use it is `use_of`'s question, which keeps "printed" and "analysed" two separate
     answers.
- **`shot_measurements` now calls `device_of`.** Its inline `match` is gone. Its unit test
  `the_shot_source_names_the_device_and_falls_back_to_the_source_enum` passes unchanged. So do the
  corpus `measurements` stage gates (`analysis/tests/measurements.rs`, `core/tests/stages.rs`) and
  `core/tests/engine.rs`, which shows the source string did not move. The docs on
  `ShotSource::as_str` and `shot_measurements`, and the crate doc in `lib.rs`, now route to
  `capability`. The crate doc names it as the one module with no Python twin.
- **The tests, `crates/contracts/tests/capability.rs`, are thirteen.** Four hold the file:
  - every declared name is a `ShotData` key, and not one of the five identity keys (`shot_id`,
    `session_id`, `timestamp`, `source`, `provenance`);
  - no device and no field is declared twice;
  - every `shown_only` has a note;
  - every entry has a `source`.

  One holds `hd_golf`'s shown-only set to exactly `{club_head_speed, smash_factor}`. Six hold the
  rule:
  - per-shot ∩, and the union across shots;
  - a missing tile is excluded, while a located blank V tile is printed;
  - the unstamped rule;
  - the direct-feed rule, which also checks `device_of`'s fallback;
  - `hd_golf` never prints `low_point` or `attack_angle`;
  - an unknown device is refused, alone and inside a set.

  Two read the corpus:
  - `device_of` is `hd_golf` on every corpus `input.shot`;
  - every corpus shot is unstamped, and the corpus' `printed_fields` sits inside `hd_golf`'s
    declaration and lacks `impact_position_v`. **M29's re-read is what moves that test.**
- **The tests were seen to fail.** `devices.json` was given a missing note, a screen label, a
  duplicate and a blank source, and the four file tests failed. The rule was then mutated:
  `fields_present` was read as held values, and a direct feed was made empty. The layout test and
  the direct-feed test failed. Both files were restored and checked by sha256 and `cmp`.
- **For P11:**
  - `CLAUDE.md`'s crates row says `crates/contracts` mirrors "the nine `contracts/` modules". That
    is still true of the mirrors. It should now also name `capability` and `devices.json` as
    Rust-only.
  - The `_comment` in `devices.json` says what "not printed" means downstream. Nothing consumes
    `printed_fields` yet: M36/M37 are the first callers.
- **Line endings.** The new `capability.rs` and `tests/capability.rs` are LF, like P9's
  `schemas.rs`. The edited `lib.rs`, `shot.rs` and `engine.rs` kept their CRLF after `cargo fmt`.
- **Verify**: `cargo test --workspace --no-fail-fast` green, including the 13 new tests;
  `cargo clippy --all-targets` 0 warnings; `cargo fmt --check` clean. `cargo doc -p contracts -p
  analysis --no-deps` shows no warning in any file this phase touched. `pytest` 2015 passed, which
  is unchanged, so a new JSON file under `crates/` trips no Python pin. Nothing under `spec/`, `src/`
  or `data/` was written.

### P11 — found (2026-10-01)

- **Landed as planned, in every file the phase named.** `docs/CONFORMANCE.md`: §1 gained "Three
  roots are Rust's, and edited by hand (M32)" (who edits them, how to spell a property, why not
  `schemars`, `schemas.rs`' four tests, and the two lists `RUST_OWNED` and `RUST_OWNED_SCHEMAS`
  that must change together, P9's note). §2 gained "Who recorded each value" (`oracle`, the
  `rerecords` entry's keys, the path grammar, the `input.shot` allowance, `spec/declarations/` and
  what reads the ledger on each side). §4 was rewritten into `rerecord`, `check` (the freeze, not
  the vectors, with its exit codes), `regenerate`'s refusal and `run`. `docs/ARCHITECTURE.md` §1
  gained the verb and `check`'s new meaning. ADR-032 has a fourteenth addendum, and its Status
  block counts fourteen. `docs/README.md` has ADR-032's row at **14** with a clause, and the total
  at **97**. `CLAUDE.md`: the commands block, the invariant, the routing row, and the crates row.
  `spec/README.md` was rewritten for `declarations/` and the ledger. §M32 gained an "As built"
  note with calls 1 and 2, plus the typo-guard contradiction P5 found.
- **Re-measured before writing**: 21 engine and 21 stage vectors carry `oracle: "python"` and one
  `rerecords` entry each; `audio` (30) and `format` (5) carry neither. The family sizes on disk
  barely moved (stages 3.28 → 3.30 MB, corpus 8.90 → 8.97 MB), so the "3.2 MB" headings stand.
- **What the phase changed beyond its list, each one left for it by an earlier finding**:
  - **The stale Rust messages** (P6, P8). The six version asserts now say "re-record the vectors
    with `golf-core rerecord` in the change that bumped it". The three missing-`spec/` `expect`s
    in `round_trip.rs`, `trigger/tests/conformance.rs` and `pose/tests/writer.rs` say "it is
    committed: restore it from git", because no command rebuilds a vector now.
  - **`scoring.rs`' "lands in full M4" panic** (P7), together with its `should_panic` test. It now
    reads `{mode:?} has no single-swing scoring policy (ADR-034 §5)`. A doc line says Python's copy
    keeps the old text until M40. No vector reaches the panic.
  - **ARCHITECTURE §1's "first real caller is M24's session engine"** now names the phone app
    (M38) and §M29's lab CLI, as `CLAUDE.md` already did. **§5** gained one bullet for M32, and the
    oracle bullet above it now says "until M32" on `check`.
  - **CONFORMANCE §3's "implemented once"** now names both comparators. The header blurb, the §2
    intro, the stages family's last two paragraphs and the format family's last line no longer
    describe `regenerate --stages-only` as a rebuild that runs.
  - **`docs/README.md`'s CONFORMANCE row** no longer says "which §4 says until M32 rewrites it".
- **Left alone, on purpose**:
  - **`scripts/conformance_vectors.py`'s module doc and `build_stages_from_disk`'s docstring**
    still name `regenerate` and `--stages-only`. They are frozen Python, so CONFORMANCE §4 carries
    the correction instead, as P6 said.
  - **ADR-032's body** (`:126`, `:173–:178`, `:430`) still describes the stages family as
    regenerable. That is history, and the new addendum corrects it.
  - **ARCHITECTURE's "reviewed 2026-09-27" header.** P11 edited two sections and did not review
    the whole file.
- **For P12**: `docs/README.md`'s document count is still untouched, per the rules. This file and
  ADR-032's new addendum are what P12's WORKLOG entry should route to. Nothing under `spec/vectors/`,
  `src/` or `data/` was written. The `ROADMAP.md` modification in `git status` predates this phase.
- **Line endings**: every edited CRLF file is still CRLF (`CONFORMANCE.md`, `ARCHITECTURE.md`,
  ADR-032, `docs/README.md`, `CLAUDE.md`, the Rust test files, `scoring.rs`). `spec/README.md`,
  `m31-m40-shot-first-pivot.md` and `pose/tests/writer.rs` were LF and still are. A Git Bash
  heredoc mangled `—` and `\\\r` in an inline Python script, so the edits went through a script
  file using `chr()`, or through the Edit tool, which keeps CRLF.
- **Verify**: `pytest tests/test_docs_truth.py` 92 passed; `pytest` 2015 passed. Every link the
  phase added resolves, file and anchor: 23 links, checked by a scratch script that slugs the
  target's headings. `cargo test --workspace --no-fail-fast` is green; it ran after the Rust edits,
  and no later edit touched `crates/`. `cargo clippy --all-targets` has 0 warnings, and
  `cargo fmt --check` is clean.

### P12 — found (2026-10-01)

- **Every exit check held, run in the plan's order before any doc was edited.** These are the
  six steps:
  1. `cargo test --workspace --no-fail-fast` passed 634 tests (1 ignored, the existing
     `clip::Cutter` doc-test) across 34 binaries. `cargo clippy --all-targets` had 0 warnings, and
     `cargo fmt --check` was clean.
  2. `pytest` passed 2015, `ruff` was clean, and `mypy` was clean over 118 files.
  3. `conformance.py check`, in the frozen view, read `21/21 vectors hold the freeze (frozen engine
     v16; 21 re-recorded by Rust)` and `21/21 stage vectors are at frozen v16 or ledgered above it`.
  4. `cargo run --release --bin golf-core -- rerecord --declare spec/declarations/v17.json` printed
     `nothing differs from the committed vectors` and `42 vectors run (21 engine, 21 stage); 0
     changed; 0 files written`, exit 0. `git status --porcelain spec/` hashed the same before and
     after, and no `.rerecord-staged` file was left.
  5. `git diff --stat -- src/` was empty, `src/` had no untracked file, and
     `contracts/swing.py:411` still says `ANALYSIS_VERSION = 16`.
  6. `data/` is gitignored apart from its README, so step 6 was checked by mtime. No file under
     `data/` is newer than the last commit (`deb8b40`, 2026-09-30 20:45).
- **The document count moved by one**, now that this file is tracked. It is 79 markdown documents:
  66 in `docs/`, of which 5 are in `plans/`. It was measured with `git ls-files '*.md' ':!.claude'`
  after `git add`.
- **ROADMAP beyond the plan's list**: the `Last Updated` date, the "NEXT ACTION" paragraph (it named
  M32 as next), and §M36's "Open" item ("Rust's moves in M32", now past tense). §M34's Status line
  names what M32 left it in `shot.rs`. The "Needs to start" column of M34 and M36 still reads "M32 —
  this box", as M33's still names M31.5 after it closed.
- **Two things for the next plans, recorded in the WORKLOG entry rather than fixed here.** First,
  no recorder exists for M34's and M36's one-time frozen-Python recordings: `regenerate` refuses the
  engine and stage families, and `conformance.py` has no screen, career or storage family. Second,
  M31.5 P8 finding 6 is still open. ADR-032's M31 addendum orders the core's callers phone-first,
  and P11's addendum did not take the correction, so it moves to §M29's plan.
- **The memory entry** `app-transition-decisions.md` and its `MEMORY.md` line now say M32 is done
  and that next is M34/M36.
