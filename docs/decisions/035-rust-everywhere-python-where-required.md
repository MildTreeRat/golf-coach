# ADR-035: Rust everywhere — Python only where it is required

## Status
**Accepted** 2026-09-30 — M31.5 of the program plan
[m31-m40-shot-first-pivot.md](../plans/m31-m40-shot-first-pivot.md), whose phase list, inventory and
interview record are [m31-5-rust-first-replan.md](../plans/m31-5-rust-first-replan.md).
**Supersedes, in part**: [ADR-030](030-app-platform-rust-core-python-sidecar.md)'s 2026-09-22
addendum (Python as the recorder of every port, and the retirement schedule),
[ADR-032](032-the-rust-core.md) §7's schedule, and [ADR-034](034-shot-first-phone-first.md) §7's lab
reader and §9 in part. Clause 7 names each sentence. The rest of those three ADRs stands.

It amends ten ADRs, each by an addendum in that ADR rather than by editing it here: 030, 032 and 034
(M31.5 P4), and [006](006-mcp-server.md), [014](014-screen-capture-shot-ingestion.md),
[016](016-local-first-host-and-phone-upload-topology.md), [020](020-conversational-followups.md),
[022](022-learned-artifacts-as-committed-data.md), [024](024-per-club-shot-history.md) and
[033](033-the-pose-sidecar-protocol.md) (M31.5 P5). Each routes to a numbered clause below instead of
restating it. [ADR-001](001-language-python.md)'s Status and [ADR-008](008-project-structure.md)'s
one exception are handled in clause 7.

**Nothing here is built, and nothing moves in M31.5.** It is docs only: no file under `src/`,
`crates/`, `tests/`, `spec/` or `scripts/` changes, `ANALYSIS_VERSION` stays at 16, and no vector is
re-recorded. The rule applies from M32 on. Every count below comes from M31.5 P1 and P2, both taken
on 2026-09-30, and their commands are in
[P1's findings](../plans/m31-5-rust-first-replan.md#p1--found-2026-09-30) and
[P2's](../plans/m31-5-rust-first-replan.md#p2--found-2026-09-30). "Rn" below is a row of P1's
inventory, and "Qn" is one of P2's answers.

**M29 built clauses 2 and 5** (2026-10-29). The first addendum at the foot answers the four
questions this ADR left to M29 and names the dependencies the build brought in. The second
(2026-11-09, M43, with [ADR-036](036-llm-evaluation-and-observability.md)) adds the
call log and the evals to clause 1's LLM list.

## Date
2026-09-30

## Context

**What the program plan said, as M31 left it.** Python was the recorder of every port: ADR-034 §9
records a port of existing Python from Python, as M22 was. M35 was planned as **new Python on
purpose**, because it was to be M36's oracle. The PaddleOCR lab reader stayed Python beside the
phone's Apple Vision (ADR-034 §7). Retirement ran on ADR-030's rule as ADR-032 §7 scheduled it: a
module leaves Python once its Rust twin conforms *and nothing that stays Python calls it*, which put
the lab's exit at §M29, and ADR-034 blocked §M29 on M40.

**What already exists in Rust.** `crates/analysis` and `crates/core` conform on all 21 engine
vectors and all 21 stage vectors, `crates/trigger` on the 30 audio vectors, and `crates/feedback`
carries the ranked tips every engine vector records. `crates/pose` spawns the MediaPipe worker and
speaks ADR-033's protocol to it. None of it has a caller yet, and the Python lab it mirrors is
32,698 lines under `src/golf_coach/` and 13,360 in `scripts/` (P1).

**The directive (the user's, 2026-09-30).** It arrived mid-way through planning M32:

> "Why am I still seeing python files? We should be rewriting everything in rust unless it's media
> pose (or another library that we have a dependency on)."
>
> "Let's replan for an entire Rust project EXCEPT for where python is REQUIRED. For instance, if the
> user decides to do the optional cameras to record their pose, we need to use media pose with
> python. There isnt another alternative that we should use for that right at this moment. If we
> ever decide to include AI in the project, python has more maturity in the ai field so we would go
> with it there as well."

**What the inventory found that shapes the answer** (P1, 44 rows, signed off in P2 as Q1–Q17):

- **Frozen Python can read what Rust writes, within limits.** No pydantic model sets `extra`, so every
  one of them ignores an unknown key, but the contract's enums are closed and would refuse a new
  value (P1 finding 1).
- **`golf-core` can re-record the engine family with a thin wrapper.** The stage family has no Rust
  producer, because `conformance.py::run_stages` is the only code that assembles a stage document
  (P1 finding 3).
- **Q4 reverses a dependency.** A Rust-first M35 needs the corpus and stores that M36's
  `crates/storage` builds, so M35 now depends on M36 (P1 finding 7).
- **The LLM reaches its tools through Python `mcp/`, in process.** Porting `mcp/` therefore decides
  how the LLM gets them (P1 finding 9).
- **The pose worker needs more Python than `pose/`.** It decodes frames through `capture/`'s
  OpenCV source (P1 finding 10).
- **Ten ADRs state Python as the oracle, as permanent, or on a retirement schedule** that this
  decision changes. P1 finding 4 names nine, and P2 finding 1 adds ADR-033.

## Options Considered

### Option A: Keep M31's plan
Python records every port, M35 is written in Python first, and the lab retires on ADR-032 §7's
schedule after M40.
- **Pros**: every port keeps an oracle independent of the implementation under test, the way M22's
  did. Nothing is re-planned.
- **Cons**: it writes Python that is born to be retired, which ADR-034 §9 already calls the thing to
  avoid, and M35 is exactly that. Two implementations of the engine stand until after M40, and a
  second copy is a second thing that drifts (ADR-030's 2026-09-22 addendum). And the user rejected
  it.

### Option B: Rust everywhere, Python only where required, with the lab frozen and then ported *(chosen)*
Rust becomes the oracle now, the Python lab freezes, and a milestone ports the lab. Python survives
only for MediaPipe and the LLM.
- **Pros**: no new Python is written. The phone path (M32 onward) never waits on a Python reference.
  One language owns the analysis from M32, and the frozen copy cannot drift by edit, because nobody
  edits it.
- **Cons**: a re-record from the implementation under test is the self-portrait that
  `conformance_vectors._audio` refuses to make, so it needs a gate (clause 3). The lab has to be
  ported, which is a milestone of work (clause 5). And the frozen copy disagrees with Rust *by
  design* from M32 on (clause 4).

### Option C: Rust for the required exceptions too
Pose through ONNX Runtime, and the LLM over raw HTTP from Rust.
- **Pros**: one language, with no worker boundary at all.
- **Cons**: pose is [ADR-030](030-app-platform-rust-core-python-sidecar.md) Option C, declined
  because `ranges.json` was cut from MediaPipe-Python output and pose is the calibration of
  everything downstream. ADR-034 §8 reopens pose only on the phone, and behind a gate. For the LLM,
  Anthropic has no official Rust SDK (P2 finding 3, 2026-09-30), so Rust would mean a hand-written
  tool loop over raw HTTP. The user weighed that and kept the LLM in Python (Q9).

### Option D: Keep more Python, as "required by a library"
PaddleOCR, the MCP server and the FastAPI server stay Python.
- **Pros**: PaddleOCR is tuned against these photos, the MCP server uses the Python `mcp` SDK, and
  none of the three needs porting.
- **Cons**: the performance case is weak, because the heavy work sits in C++ inference whichever
  language calls it. The MCP server has no AI inside it: it is Claude's data interface, not an LLM
  call. The planning session offered all three as candidates to keep, and the user did not take any
  of them (interview decision 2).

### Sub-decision: how the LLM reaches its tools once `mcp/` is Rust

- **Drive the Rust MCP server over stdio *(chosen, Q9)***. This is
  [ADR-020](020-conversational-followups.md)'s Option B, served by the SDK's
  `anthropic.lib.tools.mcp` helpers, which `anthropic` 0.121.0 in `.venv` has (checked 2026-09-30).
  Option B's cost was a spawn and a wire round trip per call. A long-lived client session amortises
  the spawn, and the round trip is what any client of the server already pays.
- **Keep a Python copy of the query layer.** That copy calls `analysis/`, so under ADR-032 §7's own
  clause `analysis/` could never retire.
- **Move the LLM to Rust.** This is Option C above, and it was declined there.

## Decision

**Option B.** The numbered clauses are what the addenda cite.

### 1. The rule, and the two exceptions it names

> **Everything is Rust. Python stays only where it is required**, and "required" means a library
> the project depends on that has no alternative the user would take today.

Two things meet that bar, both named by the user:

- **MediaPipe pose**, for the optional video. It runs as a worker behind `crates/pose`, on
  [ADR-033](033-the-pose-sidecar-protocol.md)'s protocol, and that boundary stays as built.
- **The LLM**, because "python has more maturity in the ai field". Today that means Claude's coaching
  brief, the follow-up conversation and the club-specification lookup.

**A third exception needs its own decision**, an ADR or an addendum that names it. It is never the
default, and "uses numpy" or "a Python SDK exists" is not the bar. PaddleOCR, the MCP SDK and FastAPI
were each weighed against this clause and did not meet it (clause 2).

**What survives, by name.** Each exception pulls in its import closure, and nothing else survives:

- **The pose worker**: `pose/estimator.py`, `pose/worker.py`, and `capture/`, which the worker decodes
  frames through (R18, Q11; P1 finding 10). It also needs `contracts/keypoints.py` and `config.py`,
  which M40 cuts down to what the two workers read. `crates/pose/tests/stub_worker.py` stays as part
  of the gate on the boundary.
- **The LLM**: `feedback/coach.py`, `feedback/conversation.py`, `storage/transcript_store.py`,
  `clubs/lookup.py` (Q13) and `scripts/ask_swing.py`.
- **The shapes the LLM side owns**: `contracts/conversation.py` (the transcript), and
  `contracts/club.py` and `contracts/club_spec.py`, which `clubs/lookup.py` builds and writes (P2
  finding 4).
- **Two extras**: `vision` without `ultralytics`, and `llm` whole, because its `mcp` package is the
  client the LLM drives the Rust server with (R44).

Of the contract Rust already mirrors, **only `contracts/keypoints.py` survives** (Q14). `coach.py`
reads its swing and shot as JSON from the Rust MCP server, as `conversation.py` does, and not as
pydantic models. Whether the caveat prose reaches Python through that server or through a kept copy
is M29's to settle (P2 finding 4). A kept copy would be a fourth surviving shape, and M29 would have
to name it.

### 2. Everything else ports, including the three things considered and not kept

- **The lab's OCR reader** ports to Rust through `ort` (ONNX Runtime), running **the same Paddle
  models**, with the preprocessing that goes with it. It is gated on the 13 stored bay photos
  (clause 5).
- **The MCP server** ports to Rust on `rmcp`, the official SDK, in M29 (Q5). The tool surface
  survives and the language does not.
- **The FastAPI server** (`api/`) is ported to `axum` or dropped, in M40. The phone is the host now
  (ADR-034 §6).
- **Everything without a required library ports** with its consumer. That covers the many-shot
  layer, storage, the screen parser, strike audio, the flight caveats and the catalogue. P1's
  inventory gives each row its milestone.
- **Some things are not ported at all.** The aligned render and the skeleton overlay are deleted
  (Q10): `pose/{overlay,side_by_side}.py`, `alignment.py`'s render half, the render-only helpers in
  `flight_measure.py`, and `scripts/{run_pose,align_swings}.py`. No committed vector reaches any of
  them, and `golf-pose run` already poses a clip.
- **The research scripts are archived, not ported** (clause 5).

**Delete, not archive** (Q1), for any retired Python whose Rust twin conforms. ADR-030's reason holds:
a second copy that nothing runs is a second copy that drifts. Only the research record goes to
`archive/`.

### 3. The oracle moves to Rust

- **From M32, `golf-core` records.** A re-record verb reads each committed vector's input and writes
  its output. M32 adds it. The engine family (21) needs only a thin wrapper around `run`. The stage
  family (21) needs a Rust port of `conformance.py::run_stages` and its compose check first (P1
  finding 3). `format` and `audio` need nothing: `format` does not age on `ANALYSIS_VERSION`, and
  `audio` stays frozen as ADR-030's addendum froze it.
- **A Rust re-record is diff-gated.** Every output is compared *structurally* with the committed one,
  and it may differ **only** by what the change declares: named added keys, or named moved values.
  Anything else fails the re-record. That gate is what makes a re-record ADR-032 §7's changelog
  rather than a self-portrait. The gate is never `git diff`, because Rust spells 77 exponent-form
  floats as plain decimals and does not `\u`-escape non-ASCII, so the text churns when nothing has
  changed (P1 finding 3).
- **The committed vectors stay as the record of what Python said.** The gate lets a re-record change
  only what the change names, so every value Python recorded survives in the file, and git keeps the
  bytes.
- **A port of behaviour frozen Python already has is recorded from Python once** (Q7), before the port
  moves. That covers M34's parser and M36's aggregates and stores. It is the only independent
  reference those ports will ever have. After that, Rust re-records, diff-gated.
- **New behaviour gets hand-worked vectors**, with `provenance.oracle: "hand"` (ADR-034 §9's form),
  and M37's pin that every family names its oracle stands.
- **`ANALYSIS_VERSION` diverges by design.** Rust's moves in M32, from 16 to 17 for a shape change
  alone. Frozen Python's stays at 16. `api/state.py::is_outdated` compares with `<` (read
  2026-09-30), so frozen Python reads a v17 artifact as current and `scripts/reanalyze.py` finds
  nothing to do. **Nobody should "fix" the gap by bumping Python.**
- **Python's `conformance.py check` stops certifying at M32's first Rust-only re-record.** It would
  report every re-recorded engine and stage vector as stale (P1 finding 3). What it still checks
  after that, and how `tests/test_conformance.py`'s pins change, is M32's plan. It retires with
  `analysis/` in M40.
- **Who generates `spec/schemas/` once Rust owns a shape** is M32's to settle, from P1 finding 2's
  three candidates.
- **New vectors from `data/` are M29's.** The builder that reads `data/` becomes Rust there, because
  it needs M36's storage readers.

### 4. The frozen Python lab

**Frozen means no new behaviour.** The lab is fixed only when it breaks: a crash, or a refusal to read
something Rust wrote. A comment that has gone stale, or a known defect, stays as it is. The stale
comments M31 found are fixed in their Rust mirrors only, and the `Impact Position V` tie stays in the
Python parser (the plan's carried decisions 1 and 6). Using the lab is not changing it: M33's spike
feeds Apple Vision's boxes to the frozen parser as a measurement.

**The one sanctioned change is a recorder.** Adding a family to `scripts/conformance_vectors.py` so
that clause 3's one-time recording can capture behaviour that already exists adds nothing to the lab
(P2 finding 6).

**The frozen window is M32 → M40, not M32 → M29.** M29 switches the lab to Rust, but the frozen
FastAPI server keeps reading what the Rust lab writes (`api/state.py::load_analysis`) until M40
decides `api/`. Throughout that window:

- **Rust may add keys** to anything frozen Python reads. Every pydantic model ignores an unknown key,
  because none sets `extra` (P1 finding 1).
- **Rust must not write a value that a frozen enum or bound refuses.** `UnscoredReason` and
  `ShotSource` are closed enums, for example. ADR-034's two photo-side reasons are safe for as long as
  they stay out of `analysis.json`: they are first written to `shot_analysis.json`, which the frozen
  lab never reads.
- **A Python re-write loses keys without a word.** `ShotStore.put` writes `model_dump_json`, so a
  Python re-import of a Rust-written shot drops the fields only Rust has. That is why M29 deletes the
  lab scripts rather than leaving them frozen (P2 finding 2).
- **The reverse direction holds too.** No `crates/contracts` shape sets `deny_unknown_fields`, and
  M36 pins that Rust accepts every value `clubs/lookup.py` writes into the catalogue.

### 5. The lab port is M29, re-scoped

**M29 keeps its number and its ask, and changes its job** (Q2). Its ask was already clause 1's:
reduce Python to the sidecar and delete everything else. It now **ports** the lab. It comes after
M36, depends on M34 and M36, runs on this box beside M37 and M38, and is **no longer blocked on
M40** (Q3).

**Scope.** A Rust lab CLI runs the lab end to end: strike detection through `crates/trigger`
directly, strike audio through ffmpeg as a subprocess, OCR through `ort`, the parse through
`crates/screen` (M34), the engine through `crates/core`, and the storage that M36 did not port. It
calls Python only as a worker, for MediaPipe, through `crates/pose`. M29 also carries:

- **the `rmcp` server** (Q5), taking the flight caveats, `contracts/caveats.py` and
  `contracts/tool_descriptions.py` with it;
- **`conversation.py`'s stdio route** to that server (Q9), and **`coach.py`'s JSON entry** (Q14);
- **the vector builder that reads `data/`** (clause 3);
- **the phone-export import, as a verb of the lab CLI** (Q8). M38 P4 waits on M29, and its exit
  compares against the Rust reader rather than Python's `read_corpus`.

**The OCR gate.** The Rust reader reads the 13 stored bay photos, and each resulting shot is compared
with the one parsed from PaddleOCR's boxes on the same photo. M29's plan sets what may differ, and
Python PaddleOCR retires once the gate passes.

**The archive move** (interview decision 3) happens in M29, and not in M31.5:

- **What moves to `archive/`**: `scripts/golfdb/`, `scripts/caddieset/`,
  `scripts/hand_landmark_reliability.py`, `scripts/{trigger_replay,pose_replay}.py` (Q15, as the M20
  and M23 measurement records) and `contracts/reference.py`. The `research` extra goes with them.
- **The JSON they produced stays where it is.** `crates/analysis` reads six files by `include_str!`
  from `src/golf_coach/analysis/benchmarks/`, until M40 moves them crates-side.
- **Archived `golfdb/` runs until M40 deletes `analysis/`**, and is a record after that (P1 finding 11).
- **The move rewrites the doc paths that name those scripts, in the same change** (P1 finding 14).
- **`spikes/` stays where it is**, as history rather than code anyone runs (Q15).

**Retirement happens at two moments** (Q17, P2 finding 2). The line between them is the frozen
FastAPI server's import closure.

- **M29 replaces.** The lab's entry points switch to Rust. M29 adds its Rust routes beside the frozen
  Python ones and does not rewire the frozen server. It deletes only what that server does not import:
  the lab scripts (R28–R30, R32, R34), `detection/` and `frontend/`.
- **M40 deletes** everything in the closure: `analysis/`, `storage/` except `transcript_store.py`,
  `launch_monitor/`, `audio/`, `feedback/rules.py`, `pose/{overlay,side_by_side}.py`, Python `mcp/`,
  the ported half of `contracts/`, the conformance scripts, and `api/` unless M40 ports it. The
  benchmark JSON moves crates-side in the same change, and the `api`, `ocr` and `audio` extras go
  with the code they served.

### 6. Order: the phone path first

- **After M31.5, M32 is next, and it is Rust-only.** The lab port is a milestone before M40, not a
  prerequisite of M32 (interview decision 6).
- **M36 comes before M35** (Q4). M36 ports the many-shot layer faithfully, with frozen Python as its
  one-time oracle, and that includes `read_corpus` exactly as it behaves today. M35 then changes it in
  Rust with hand-worked vectors: photo-only admission, and `shot_result`, which moves from M36's list
  to M35's (P2 finding 5).
- **The dependencies** (P2 finding 5): M36 depends on M32, M35 on M36, and M37 on M35. M29 depends on
  M34 and M36. M38 P4 depends on M29, and M40 on M38 and M29. The program plan and `ROADMAP.md` carry
  the table.

### 7. What this supersedes, sentence by sentence

- **[ADR-030's 2026-09-22 addendum](030-app-platform-rust-core-python-sidecar.md#addendum-2026-09-22--the-retirement-rule-and-the-first-module-to-meet-it).**
  "Python keeps only what does not translate" is replaced by clause 1. The two rules agree about
  MediaPipe, but this one drops "The lab stays Python too". Its survivor list, "`pose/estimator.py`,
  `feedback/coach.py` and `feedback/conversation.py`, and nothing else", is replaced by clause 1's.
  "The vectors are the oracle, not the code that recorded them" stands, and so does M20's delete.
  What changes is who records (clause 3) and when things retire (clause 5).
- **[ADR-032 §7](032-the-rust-core.md#7-nothing-is-deleted-in-m22-and-analysis-is-retired-after-it).**
  The third clause, "…and nothing that stays Python calls it", stands as a rule. Its *stays* now
  means clause 1's list, and its schedule is clause 5's: M29 ports and M40 deletes. "After M29, an
  `ANALYSIS_VERSION` bump re-records the engine family from the Rust core" is brought forward to M32,
  diff-gated (clause 3). "Some of `scripts/` is permanent by an existing decision" is dissolved: the
  fitting scripts are archived rather than kept running.
- **[ADR-034 §9](034-shot-first-phone-first.md#9-the-oracle-per-vector-family)**, in part. "A port of
  existing Python is recorded from Python" now means once, and then Rust (clause 3). "New analysis is
  Rust first" stands, and now covers all new behaviour. "The lab reaches Rust-only analysis through
  `golf-core` subcommands" is superseded, because the frozen lab reaches no new analysis at all and
  the Rust lab CLI is the lab from M29.
- **[ADR-034 §7](034-shot-first-phone-first.md#7-ocr-on-the-phone)**: "PaddleOCR and OpenCV stay as
  the lab's reader". The reader ports through `ort` in M29 (clause 5). There are still two
  recognizers behind one seam, and the parser is still what the vectors gate. In ADR-034's
  Consequences, "M29 is blocked on M40 … and its job is unchanged" is superseded by clause 5.
- **The program plan.** §M35's "Python, which is M36's oracle" becomes Rust, after M36. §M32's Python
  edits move to M34 or are dropped. M38 P4's `scripts/import_phone_export.py` becomes an M29 verb.
- **The ADRs M31.5 P5 amends**, each for the sentence P1 finding 4 names:
  - [ADR-006](006-mcp-server.md): the MCP server as "a standalone Python service" (clause 2).
  - [ADR-014](014-screen-capture-shot-ingestion.md): its M31 addendum's lab reader, "PaddleOCR,
    unchanged", and its parser port "recorded from Python" (clauses 2 and 3).
  - [ADR-016](016-local-first-host-and-phone-upload-topology.md): "M29 retires `api/` once M40 has
    landed" (M40 decides and deletes), and `scripts/import_phone_export.py` (clause 5).
  - [ADR-020](020-conversational-followups.md): Option C cannot outlive the MCP port. The stdio route
    is added in M29, and the in-process one ends with the frozen server in M40.
  - [ADR-022](022-learned-artifacts-as-committed-data.md): fitting is archived, while the artifacts
    stay and Rust evaluates them.
  - [ADR-024](024-per-club-shot-history.md): its M31 addendum's "with Python as the oracle" for M35
    and M36 (clauses 3 and 6).
  - [ADR-033](033-the-pose-sidecar-protocol.md): its pointer to §M29 for deleting the Python keypoints
    writer moves to M40, because the frozen pipeline holds that writer. The protocol is unchanged.
- **[ADR-001](001-language-python.md)'s Status** says Python remains "the lab: fitting …, the corpus
  tools, the conformance oracle, LLM coaching and OCR". Of those five, only LLM coaching stays true.
  P5 marks it, as a pointer or a short addendum.
- **[ADR-008](008-project-structure.md)** needs no addendum. The import rule's Python half freezes with
  the lab, and its one exception (`storage/corpus.py` and `mcp/query.py` importing `api.state`)
  retires in M40, when both sides of it are deleted. The Rust half is cargo's to enforce, and it is
  unchanged.

## Consequences

- **Two implementations of everything ported stand until M40**, not M29, and they now disagree on
  purpose. From M34, the phone's Rust parser applies the tie rule and the frozen Python parser does
  not, so the two read the plan's two label-fix shots differently until the lab switches to Rust in
  M29. Freezing is what keeps that disagreement to the declared diffs.
- **The verify list changes in M32.** Python's `conformance.py check` stops certifying at the first
  Rust-only re-record, and `cargo test` is the gate from then on (clause 3). M32's plan names the step.
- **The Rust build gains two dependencies.** `ort` brings ONNX Runtime, a C++ inference runtime, into
  the lab reader (M29), and `rmcp` brings the MCP SDK. `ort` is a second numeric library outside the
  scoring path, beside `crates/trigger`'s `rustfft`, and like it is named rather than assumed. The
  scoring path stays free of both.
- **Every follow-up answer depends on a subprocess launching**, which is ADR-020's named cost for
  Option B, accepted here because the alternative keeps `analysis/` alive (the sub-decision above).
- **One contract moves between milestones.** `shot_result` leaves M36 for M35, and M35 now depends on
  M36.
- **Docs move with it.** M31.5 P4–P8 write the addenda, the program plan, `ROADMAP.md`, `CLAUDE.md` and
  `docs/CONFORMANCE.md`, and each one routes here rather than restating a clause.
- **Unchanged:** `ranges.json`; the MediaPipe boundary and ADR-033's protocol; ADR-034's clauses other
  than §7's lab reader and §9; ADR-008's import rule in Rust; the stdlib-only, no-numeric-library
  scoring core; and "no score beats a wrong one".

## Deferred, by choice

- **`api/`: port to `axum`, or drop.** M40 decides (interview decision 2), and with it the caller of
  `clubs/lookup.py` (Q13) and the fate of `tempo_trainer.py` (Q12).
- **Who generates `spec/schemas/`.** M32 decides, from P1 finding 2's candidates.
- **How the caveat prose reaches Python.** M29 decides (clause 1). *Answered by the M29
  addendum: through the server, with no kept copy.*
- **Whether the frozen server's upload path stays open between M29 and M40.** `api/worker.py` →
  `api/pipeline.py` still writes `data/` with Python, and M29 decides (P2 finding 2).
  *Answered by the M29 addendum: closed.*
- **A standalone `sidecar/` package for the surviving Python.** ADR-033 left it to §M29, and this ADR
  does not move it. *Answered by the M29 addendum: not in M29, and M40's.*
- **An LLM in Rust.** It is revisited only by a decision, if an official Anthropic Rust SDK changes
  the cost the user weighed in Q9.

## Addendum (2026-10-29, M29): the lab is ported, the four questions left to M29 are answered, and four dependencies are named

[M29](../plans/m29-lab-port.md) built clauses 2 and 5. The lab's verbs are `golf-lab`'s, the MCP
server is `golf-mcp`, the screen's recognizer is `crates/ocr`, and the lab scripts, `detection/` and
`frontend/` are deleted, with the research record in `archive/`. This addendum answers what the
Deferred list left to M29 and names what the build brought in. The plan's findings hold the
measurements, and each item points at the one that carries its numbers.

### The four questions, answered

- **The OCR gate (clause 5: "M29's plan sets what may differ").** The plan's D7: on each of the 13
  stored bay photos, the shot parsed from the Rust reader's boxes must equal the one parsed from
  PaddleOCR's committed boxes in every value, `fields_present` and `needs_review`. Confidences, box
  geometry and warning text may differ. **12 of 13 passed outright. The thirteenth, `2026-08-10-1`,
  differs at one value**: the Rust reader reads `impact_position` `CENTER`, which the photo shows,
  where PaddleOCR's UVDoc-warped boxes read nothing. The user chose to declare it rather than port
  UVDoc (P7, answer (a)), and `crates/ocr/tests/read.rs`' `DECLARED_DIFFERENCES` fails if it goes,
  changes or gains a second ([CONFORMANCE §2's OCR gate](../CONFORMANCE.md)). PaddleOCR is no longer
  the lab's reader. The `ocr` extra stays for the frozen server's pipeline until M40, and with its
  upload route closed, only that server's startup recovery (below) could still reach it.
- **How the caveat prose reaches Python (clause 1; the plan's D10).** **Through the server, with no
  kept copy.** `golf-mcp`'s `instructions` carry the standing caveats, and a read-only MCP resource
  serves the other named sentences (`golf_mcp::caveats`). `feedback/coach.py`'s JSON entry and
  `feedback/conversation.py`'s stdio route read both at session start. So no fourth surviving shape
  joins clause 1's list. The frozen copies in `contracts/caveats.py` still feed the in-process route
  the frozen server uses, and go with it in M40.
- **Whether the frozen server's upload path stays open (the plan's D11).** **Closed.** Every route
  that wrote under `sessions/` or `shots/`, eight of them, answers 410 naming the Rust verb that does
  its job now (`api/app.py`'s `_closed`; P32). `golf-core` gained `set-golfer`, `set-club` and
  `delete-swing` for the three repairs that had no verb. A phone's data reaches the lab as an export
  (`golf-lab import-export`). Two consequences are left as found (P32's findings 12 and 13):
  - **One write path is left.** The server's worker still starts, and at startup it re-runs, through
    frozen Python at engine 16, an orphaned run or a complete bundle with no result in today's
    session. After the closure only an imported filmed bundle dated today could be one. Turning the
    worker off is a one-line default outside D11's list, so it is the user's call, not a fix.
  - **A new golfer needs a swing or a phone's export.** `set-golfer` registers a golfer only while
    re-attributing a swing, and `import-photos --into-session` refuses an unregistered one. If that
    bites, the fix is a registration verb.
- **A standalone `sidecar/` package (the plan's D12).** **Not in M29.** M29 deletes only what the
  frozen server does not import, and repackaging would move files that server imports. It is M40's,
  for that reason, and [ADR-033](033-the-pose-sidecar-protocol.md)'s addendum says so beside the
  protocol.

### The dependencies, named as the Consequences said they would be

Each is pinned to an exact version, and each crate's `Cargo.toml` says why that version.

- **`ort` `=2.0.0-rc.10`, ONNX Runtime 1.22**, in `crates/ocr` alone, with `download-binaries` and
  `copy-dylibs`: the first build fetches the runtime, and `Cargo.lock` records it. rc.10 is the
  newest `ort` the workspace's `rust-version = "1.87"` builds. `crates/ocr/tests/ort_reach.rs` fails
  if `contracts`, `pyfmt`, `analysis`, `feedback`, `screen`, `storage` or `core` can reach it, which
  keeps it off M38's `crates/app`. It is a second numeric library outside the scoring path, beside
  `rustfft`.
- **The models are converted once, gitignored and sha-pinned** (the plan's D6). Three of PaddleOCR's
  five run: PP-OCRv6 medium detection and recognition and PP-LCNet textline orientation; document
  orientation and UVDoc are skipped (D5). `crates/ocr/models.json` pins each file's sha256 and its
  preprocessing constants, and its `conversion` block records the toolchain. `paddle2onnx` 2.1.0
  against `paddlepaddle` 3.1.1, the one pairing whose extension loads, in a Python 3.11 scratch venv,
  because the plugin's pin has no wheel for `.venv`'s 3.13 (P1's finding 2). `.gitignore` gained
  `data/models/ocr/`, because the plan's `data/models/*.onnx` does not reach a subdirectory.
- **`image` `=0.25.9`, PNG only.** JPEG is decoded by a hand port of libjpeg-turbo, because `image`'s
  JPEG codec does not decode to OpenCV's pixels and that moved the screen quad on two bay photos (the
  user's answer at P2). HEIC is decoded by ffmpeg (D3). `imageproc`, which the plan allowed, never
  entered: the rectifier is a hand port of OpenCV's steps.
- **`rmcp` `=0.11.0` with `tokio` `=1.54.0`**, in `crates/mcp` alone. It negotiates protocol revision
  2025-06-18, where Python's SDK negotiated 2025-11-25. That revision brought `structuredContent`, so
  nothing a tool returns needs the later one, and the surface declares the difference (ADR-006's
  addendum).
- **`zip` `=7.3.0`**, in `crates/lab` alone, for the phone's export (P31).
- **ffmpeg is found, not bundled.** `golf-lab` reads `GOLF_FFMPEG`, else `PATH` (ffmpeg 9.0.2 on
  this box, the plan's D17), and never falls back to `.venv`'s `imageio-ffmpeg` 7.1. **9.0.2 ends each
  audio track where its edit list ends**, and 7.1 decoded the trailing priming frame whole, so 26 of
  the 30 stored clips decode 16 to 976 samples shorter (P21's finding 4). The user chose to gate the
  decoder the lab runs (P21, answer (a)): strikes, frames and ranks exact, and the three fields that
  feed nothing (`duration_s`, `confidence`, `prominence`) to a relative 2e-3.

### What else the build settled, for a reader of clause 3 or clause 4

- **A recorder runs a shell's binary.** Nothing may depend on `crates/mcp` or `crates/lab`, which are
  shells like `core`, so `golf-core rerecord` walks the mcp family by running `golf-mcp answer-case`
  rather than linking the runner (P15). Both version keys walk that family now: a career declaration
  because its tools read the corpus, and an engine one because its flight and band answers are the
  engine's (P19). Every re-record therefore needs `golf-mcp` built in the same profile.
- **Clause 3's one-time recording ran twice more**, for `mcp` (P8) and `selection` (P18), each through
  the sanctioned recorder change, each refusing a second run
  ([CONFORMANCE §2](../CONFORMANCE.md)). And **"new vectors from `data/` are M29's"** is
  `golf-lab build-vectors` (P30), which writes only where no file is and stamps `oracle: "rust"`, so
  `golf-core rerecord` stays the only re-writer.
- **`data/` is Rust's.** P29 backed up `data/processed/` without the clips (the plan's D8), re-read and
  stamped the 13 stored shots and re-ran the 15 bundles at engine 17. Only the declared shot values
  and last-bit scores moved, and every frozen reader still answers (P29's finding 16).
- **`golf-lab reanalyze` asks a fourth question** `reanalyze.py` never could: whether an older screen
  parser read the shot. Frozen Python had one parser, so its artifacts could not go stale that way.
  Without the question, a `SCREEN_PARSER_VERSION` bump would leave every stored shot stale while the
  verb reported nothing to do (P27).
- **Clause 4 held in both directions.** The frozen server reads every artifact the Rust lab wrote
  (P29), and the one value it cannot, a photo-side `unscored` reason, stays in `shot_analysis.json`,
  which it never opens (the plan's C6).

### What this does not change

Clauses 1–7 stand. The survivors are clause 1's list, the scoring path has no numeric library, the
oracle is Rust's, and M40 still deletes the frozen server's closure: `analysis/`, `storage/` but
`transcript_store.py`, `launch_monitor/`, `audio/`, Python `mcp/` and `api/` unless M40 ports it.

## Addendum (2026-11-09, M43): the LLM exception gains the call log and the evals

[M43](../plans/m43-llm-evals.md) measures the three Claude routes, logging what each request costs
and grading the answers against an answer key. That is new Python, and clause 1 says nothing
survives that it does not name. So clause 1's **LLM** list gains, by name:

- **`src/golf_coach/llm_log.py`**, the call log, which the three routes write to;
- **`src/golf_coach/llm_prices.json`**, the price table it reads;
- **the `golf_coach.evals` package**, the eval runner, its flows and the judge;
- **the `evals/` data directory** at the repo root, the answer key and the coach cases.

A file inside the package or the directory needs no further decision (ADR-036 clause 1). The runs
under `.claude/hillclimb/` are data, not Python, and fall under no clause here.

**Why this is the LLM exception and not a third one.** Every one of these exists only to observe
or measure the LLM routes. `llm_log` is imported by `clubs/lookup.py`, `feedback/coach.py` and
`feedback/conversation.py`, and the runner calls those same functions. None of them is required by
some other library, so this is not the bar clause 1 sets for a third exception, and it does not
lower that bar.

**The closure stays where it was.** `llm_log` imports no `anthropic`. The runner, the flows and the
judge import nothing from `golf_coach` beyond `config.py`, `llm_log` and this clause's survivors, so
they add nothing to what M40 cuts. The one possible exception is a coach-case generator that reads
the frozen lab's shapes as a recorder, under clause 4 (ADR-036 clause 1 sets its terms). **For
M40**: `llm_log` writes beneath `settings.processed_dir`, so M40's cut of `config.py` keeps that
field, or gives `llm_log` its path another way.

**What this does not change.** Clauses 1–7 stand, with clause 1's list extended as above. The
bar for a third exception is where it was, the frozen lab gains nothing, and M43 changes no Rust
and adds no `config.py` field.
