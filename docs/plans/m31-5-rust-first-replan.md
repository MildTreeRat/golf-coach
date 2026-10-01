# Plan: M31.5 — Rust everywhere, Python only where required (the re-plan, docs only)

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-09-30, in the session that set out to plan M32 and was redirected by the user
mid-interview. **Milestone**: M31.5, a new milestone between M31 and M32, numbered the way `M1.5`
already is. Its `ROADMAP.md` section was written by P7, and it closed at P9 on 2026-09-30.
**Governing decisions**: ADR-035 (written by P3), amending
[ADR-030](../decisions/030-app-platform-rust-core-python-sidecar.md),
[ADR-032](../decisions/032-the-rust-core.md) and [ADR-034](../decisions/034-shot-first-phone-first.md)
by addendum (P4), plus whichever others P1's inventory names (P5).

---

## Status checklist

**Stop after every phase. Every time.** Update this table when a phase is done, in the same change
as the phase, and append what the phase *found* to its section under "Phase findings". The findings
are what the next phase is planned against.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document, and the pointers that stop a session starting M32 | ✅ Done *(2026-09-30)* |
| **P1** | Inventory every Python area and script, and propose a disposition (read-only) | ✅ Done *(2026-09-30)* |
| **P2** | The user signs off the inventory (interactive) | ✅ Done *(2026-09-30)* — 17 answers; LP is M29, deletions wait for M40 |
| **P3** | Write ADR-035 | ✅ Done *(2026-09-30)* — seven clauses in the suggested order; ten ADRs to amend |
| **P4** | Addenda, "who is the oracle and what retires": ADR-030, 032, 034 | ✅ Done *(2026-09-30)* — three addenda, 86 → 89; M32 must name a re-recorded family's oracle |
| **P5a** | Addenda for what M29 ports: ADR-006, 014, 020, 022 | ✅ Done *(2026-09-30)* — four addenda, 89 → 93; M34 inherits a `profiles.json` question |
| **P5b** | Addenda for the order and the M40 line: ADR-016, 024, 033, and ADR-001's Status | ✅ Done *(2026-09-30)* — three addenda and a Status pointer, 93 → 96; M29 is `crates/pose`'s first caller |
| **P6a** | Program plan: the re-plan section, the status table, and M32 re-detailed Rust-only | ✅ Done *(2026-09-30)* — §M32 is Rust-only; schemas split, re-record keeps Python's values, `check` leaves verify at M32 |
| **P6b** | Program plan: M33–M40, the lab-port milestone, invariants and verify | ✅ Done *(2026-09-30)* — §M29 written, §M35 Rust after M36; M34's re-read writes no `data/`, M29's does |
| **P7** | `ROADMAP.md`: the M31.5 row and section, and the milestones the re-plan moves | ✅ Done *(2026-09-30)* — §M29 is the lab port; `docs/FLOW.md` handed to P8 |
| **P8** | `CLAUDE.md`, `docs/CONFORMANCE.md` and `docs/FLOW.md` | ✅ Done *(2026-09-30)* — Rust is the oracle from M32 in all three; FLOW draws ADR-035's order, and its M31.5 node is P9's to flip |
| **P9** | Close: verify, WORKLOG, checklists, memory | ✅ Done *(2026-09-30)* — all six verify commands green at M31's numbers; M31.5 is closed and M32 is next |

**Exit**: `pytest tests/test_docs_truth.py` passes after every phase, and P9 runs the whole verify
suite, which must be as green as M31 left it.

---

## What this milestone is

The user's directive of 2026-09-30, written down as a decision and applied to the program plan
before any of M32–M40 writes code:

> "Why am I still seeing python files? We should be rewriting everything in rust unless it's media
> pose (or another library that we have a dependency on)."
>
> "Let's replan for an entire Rust project EXCEPT for where python is REQUIRED. For instance, if the
> user decides to do the optional cameras to record their pose, we need to use media pose with
> python. There isnt another alternative that we should use for that right at this moment. If we
> ever decide to include AI in the project, python has more maturity in the ai field so we would go
> with it there as well."

Today the program plan (M31's) keeps Python as the recorder of every vector family it ports, plans
M35 as **new Python** on purpose ("M36's oracle"), and retires Python on ADR-030's schedule, which
ends after M40. This milestone reverses that: Rust becomes the oracle now, the Python lab freezes,
and Python survives only where it is required.

**No code changes.** Nothing under `src/`, `crates/`, `tests/`, `spec/` or `scripts/` is touched,
`ANALYSIS_VERSION` stays where it is, and no vector moves. **The archive move is not this
milestone's either**: it happens with the lab port (user, 2026-09-30). The only suite that can fail is
`tests/test_docs_truth.py`, and it fails on counts.

**Every phase that adds a file or an addendum updates those counts in the same change.** Counting
commands:

```bash
git ls-files '*.md' ':!.claude' | wc -l              # docs/README.md line 3; new files must be `git add`ed first
grep -c '^#\+ *Addendum' docs/decisions/*.md        # the per-ADR cells, and their sum is "N addenda between them"
```

Counts at planning time (2026-09-30): **76** markdown documents, **63** in `docs/`, **35** ADRs,
**3** in `plans/`; addenda total **86**, with ADR-030 **3**, ADR-032 **12**, ADR-033 **5**, ADR-034
**none**. Three ADRs state their own count in their Status block, and
`test_an_adr_that_counts_its_own_addenda_counts_them_correctly` reads it: **ADR-030 "Three
addenda"**, **ADR-032 "twelve addenda"** and **ADR-033 "Five addenda"**. Adding an addendum to any
of them means rewording that phrase in the same edit. Re-count before editing; these are a snapshot.

---

## Decisions taken in the interview (2026-09-30)

These are the user's. A later phase that wants to revisit one needs a reason the interview did not
have.

1. **Rust everywhere. Python only where it is required**, and two things are named as required:
   - **MediaPipe pose**, for the optional video. It already runs as a worker behind `crates/pose`
     (ADR-033), and that boundary stays.
   - **AI/LLM work**, "python has more maturity in the ai field". Today that is
     `feedback/coach.py`'s Claude coaching call.
2. **Everything else ports to Rust**, including three things the planning session offered as
   candidates to keep:
   - **PaddleOCR** gets a Rust reader (`ort`/ONNX Runtime running the same Paddle models) in the lab
     port, gated by an accuracy check on the 13 stored bay photos. Python PaddleOCR is retired then.
   - **The MCP server** ports to Rust (`rmcp`, the official SDK). It has no AI inside it; it is
     Claude's data interface.
   - **The FastAPI server (`api/`)** ports to Rust (`axum`) or is dropped, in M40. The phone is the
     host now.
   - The planning session's case for keeping any of them was weak on performance (the heavy work
     sits in C++ inference either way) and the user did not take it.
3. **The research scripts are archived, not ported**: `scripts/golfdb/` and `scripts/caddieset/`
   (numpy, scikit-learn, and pandas for GolfDB's pickled DataFrame) move to an `archive/` folder.
   - The user: "The project is moving in a different direction now than when we made that change.
     That code set is no longer relevant to what we are doing."
   - **The JSON they produced stays where it is**, because the engine reads it (`golfdb_v1.json`,
     `joint_model_v1.json`, `trajectory_model_v1.json`, and `crates/analysis` by `include_str!`).
   - **The move happens with the lab port**, not in M31.5. The `research` extra goes with it.
4. **Rust is the oracle from now on.** `golf-core` records vectors. Today's committed vectors stay
   as the record of what Python said. New behaviour gets hand-worked vectors
   (`provenance.oracle: "hand"`, ADR-034 §9's existing form). Python's `conformance.py check`
   retires together with Python `analysis/`.
5. **The Python lab pipeline is ported to a Rust CLI**, as its own milestone. Rust runs the lab end to
   end (strike detection, OCR, parse, engine, storage) and calls Python only as a worker, for
   MediaPipe (as `crates/pose` already does). Python `analysis/`, `storage/`, `launch_monitor/` and
   the scripts then retire.
6. **Phone path first.** After M31.5, **M32 is next, Rust-only.** The Python lab is **frozen** (no
   new features) and keeps working meanwhile; the lab port is a milestone before M40, not a
   prerequisite of M32.
7. **The inventory is signed off by the user** (P1 → P2) before the ADR is written.

## Decisions carried to M32 and M34, from the M32 interview (2026-09-30)

The session began as M32's planning. These were decided before the redirect and survive it; P6a
writes them into the program plan's re-detailed §M32 and §M34.

1. **All screen-parser work moves to M34, in Rust only.** The Python parser is frozen as it is. That
   is: the `Impact Position V` tile, the tie rule below, `fields_present`, `ProfileField.scale`,
   `parser_version` stamping, the stale-cache re-read, and re-reading the 13 stored shots. M32
   shrinks to the contract, the capability model and the vector re-record.
2. **The label tie: withhold on tie.** Boxes are assigned to labels one-to-one by best similarity.
   Where two boxes could swap fields at equal total score, **neither field is read**, a warning names
   the ambiguity, and **both boxes still bound their neighbours' columns**.
   - On disk (M31 P2 finding 2): `2026-08-10-1` stops spilling `CENTER` into `Shot Type` and
     `impact_position` stays `None`; `2026-08-23-1`'s `HEEL`, right today by luck, becomes `None`.
   - An exact read (`Impact Position` beside `Impact Position V`) resolves uniquely, and so does an
     OCR-damaged `Impact Position Y` (it scores higher against the V label than against the plain
     one). Both need tests.
3. **`no tile found for '<label>'` stays in `provenance.warnings` but never reaches the golfer.**
   Every golfer-facing surface filters it (today the coach brief, `feedback/coach.py::_shot_lines`,
   and the MCP shot view, `mcp/query.py` near line 904; on the phone, whatever M38 shows). This
   answers M31 P7 finding 3.
4. **The 13-shot re-read's exit** (now M34's): every shot *number* is identical except on the two
   label-fix shots; parse bookkeeping (confidence, warnings, `raw_fields`) may change, and each
   change is listed and explained; **no `needs_review` flips**.
5. **Two photo-side unscored reasons, defined in M32, emitted later** (first shot grading, M35/M37).
   This answers M31 P6 finding 4.
   - `printed_blank`: the tile was found and nothing readable sat under it, whether `---` or empty.
     `refilming_helps` is **false**.
   - `misread`: text sat under the tile and no value could be read from it. `refilming_helps` is
     **true**.
   - They cannot be split further, because on the bay photos **the OCR never sees `---` at all**:
     every blank Spin and Spin Axis tile stores "no value text under the label". A third reason
     ("empty, retaking might help") would tell the golfer to retake on every bay shot, wrongly.
6. **The stale comments M31 found are fixed in Rust.** `shot_measure.py`'s smash range, "three
   shots" and "one per session" (M31 P2 finding 9), `engine.py`'s outcome-bands docstring near line
   506 and `intent.py`'s "full M4" comments (M31 P6 finding 5). The user chose to fix them before the
   redirect. Under decision 6 above, the **Rust mirrors** are fixed and the Python copies are left
   frozen, to retire with the lab. P6a says so; if a Rust mirror does not carry the comment, the
   finding says that too.

## What the planning read found (2026-09-30)

For P6a/P6b and the milestones they re-detail. Verify each before leaning on it.

1. **The program plan's §M32 "Sequence" step 4 is wrong.** `scripts/import_shot_screens.py --force`
   on `data/raw/shot_screens/` parses `IMG_2738.jpeg` and `IMG_2739.jpeg`, which are **not** stored
   shots (M31 P2 finding 2), and would *add* them to `data/processed/shots/`. The 13 stored shots are
   re-read through `api/pipeline.py::_shot_for` (keyed on the manifest's photo sha256, filed as
   `{session}-{swing}`), which is what a stale-cache check under `scripts/reanalyze.py` would drive.
   Under the re-plan this moves to M34 and to Rust anyway; the lesson stands: re-read through the
   bundle, never through the bulk importer.
2. **Adding a tile to a profile moves `parse_confidence`.** `parser._score` divides by
   `len(profile.stored_fields)`. Adding `Impact Position V` moves every bay shot by about −0.02
   (the V cell reads empty, so value coverage drops). All 13 stored shots sit at 0.897–0.948 against
   `validate_parse`'s 0.6, so nothing flips; M34's exit allows it by name (carried decision 4).
3. **The V tile will add `Impact Position V: no value text under the label` to every bay shot**, the
   same way Spin and Spin Axis already do, because the OCR sees no `---`.
4. **Any new `ShotData` key changes every corpus vector's output key set.** `SwingResult` echoes the
   shot, `compare_results` requires identical key sets, and `crates/contracts/tests/round_trip.rs`
   tells an absent key from a null one. M32's re-record must therefore be a *diff-gated* re-record:
   outputs may differ from the committed ones **only** by the declared added keys.
5. **`spec/schemas/*.schema.json` are generated from the pydantic models** by
   `scripts/conformance.py`. With Rust as the oracle and Python frozen, something else has to produce
   `shot_data.schema.json` once the Rust contract moves. P1 checks how; P6a decides (candidates:
   `schemars` in `crates/contracts`, or hand-maintained with a Rust round-trip pin).
6. **Frozen Python must still read what Rust writes**, until the lab port. P1 checks each pydantic
   model's `extra` setting: the default (`ignore`) reads a Rust-written shot with new keys and drops
   them; a model with `extra="forbid"` would refuse it.
7. **`ANALYSIS_VERSION` will diverge by design.** Rust's moves in M32 (16 → 17, shape only); frozen
   Python's stays at 16, so `scripts/reanalyze.py` sees nothing to do and the lab does not churn.
   The ADR says so in one line, so nobody "fixes" it.

---

## Rules every phase keeps

- **Point, don't copy.** An addendum routes to ADR-035's numbered clause rather than restating it. A
  ROADMAP section routes to the program plan. No band value, no checkpoint count and no test count
  goes into prose (`CLAUDE.md`, and `test_volatile_counts_stay_out_of_prose`).
- **A number cited in an ADR comes from P1's findings or M31's**, with its date.
- **House style**: read two existing addenda in the ADR you are amending before writing yours, and
  match the heading form it uses (`## Addendum (date, …)` or `## Addendum, date — …`; both match the
  pin's `^#+ *Addendum`). Addenda here explain *why*, and name what they do **not** change. M31's
  form is `(2026-09-30, M31)`; this milestone's is `(<date>, M31.5)`.
- **Dates**: an addendum is dated the day it is written.
- **`docs/README.md`'s ADR rows** have four cells, `| [NNN](decisions/…) | Decision | Status |
  Addenda |`. The last cell is `—` or `**N** — summary`, and the pin reads the first `**N**` in it.
- **Line endings**: some docs are CRLF (`docs/README.md`, ADR-024 at M31's count) and some LF. Check
  with `file <path>` before a `sed -i`, and keep what was there (M31 P7 finding 6).
- **Do not touch Python to apply this milestone.** It is docs only; the rule it writes applies from
  M32 on.

---

## Phases

### P0 — This plan document, and the pointers that stop a session starting M32

**Goal**: this file in the repo, with the counts it moves, and every "next is M32" pointer saying
M31.5 comes first.

**Files**:
- `docs/plans/m31-5-rust-first-replan.md` (new; this file).
- `docs/README.md` lines 3–4: markdown documents **76 → 77**, `docs/` **63 → 64**, `plans/`
  **3 → 4** (re-count first).
- `docs/plans/m31-m40-shot-first-pivot.md`, status checklist: a new **M31.5** row (`🟡 In
  progress`, linking this plan), and M32's state cell → `⏸ Waits on M31.5`.
- `ROADMAP.md`: M32's status-table cell gains "waits on M31.5's re-plan (link to this plan)", and
  the `NEXT ACTION — the pivot, then M32` paragraph gains one sentence routing to this plan. No new
  row or section yet (P7).

**Done when**: the plan exists and is tracked, a fresh session reading ROADMAP's status table or
NEXT ACTION lands on M31.5 rather than M32, and docs-truth passes.

**Verify**:
```bash
git add docs/plans/m31-5-rust-first-replan.md
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py
```

### P1 — Inventory every Python area and script, and propose a disposition (read-only)

**Goal**: one table the user can sign off, covering everything Python in the repo, plus the facts
P3 and P6 need. **Nothing is edited except this plan's findings.**

**Do not read the repo end to end** (~39,000 lines). Per area: `wc -l`, the module docstring's first
lines (`head -25`), CLAUDE.md's crate row, `docs/ARCHITECTURE.md` §1–§2, and callers from
`grep -rn "golf_coach\.<area>" src scripts tests`.

**Areas** (at planning time, lines of Python): `analysis/` 11,168 · `contracts/` 6,878 · `api/`
3,804 · `mcp/` 3,191 · `storage/` 1,904 · `launch_monitor/` 1,726 · `pose/` 1,193 ·
`feedback/` 1,021 · `clubs/` 972 · `audio/` 488 · `capture/` 198 · `detection/` 29; every file in
`scripts/` including `scripts/golfdb/` and `scripts/caddieset/`; `tests/` as a mirror (its
disposition follows its area's); `frontend/` (served by `api/`); `config.py`; `pyproject.toml`'s
extras. Split an area where its halves differ, e.g. `analysis/`'s ported engine vs its Python-only
many-shot layer (`baseline.py`, `dispersion.py`, `club_profile.py`) vs `alignment`'s render half.

**Table columns**: area · lines · what it does (one line) · Rust twin (crate/module, and whether it
conforms on committed vectors) · Python required? (MediaPipe / LLM / neither) · relevant to
shot-first? · callers that keep it alive · **proposed disposition** (port → which milestone / keep
as a Python worker / archive / delete) · one-line reason.

**Also verify, and record as numbered findings:**
1. Every pydantic model's `extra` setting in `contracts/` (planning finding 6).
2. How `spec/schemas/` is produced today, and what could produce it from Rust (planning finding 5).
3. Whether `golf-core` (or a small addition to it) can re-record each engine vector's output from
   its committed input, and what the stage-vector rebuild needs (`conformance.py regenerate
   --stages-only` is the model: it rebuilds from committed vectors, not from `data/`).
4. Every ADR sentence that states Python as the oracle, Python as permanent, or a retirement
   schedule: `grep -n -i "oracle\|stays python\|stay python\|recorded from python\|retire" docs/decisions/*.md`.
   Name the ADRs P5 must amend (likely candidates: 006 MCP, 008 structure, 014 OCR engine, 016
   host, 022 learned artifacts, 033 pose sidecar), and say which need nothing.
5. Every place docs name M35 as Python or as M36's oracle, and M29's current wording.

**Done when**: the table and the five findings are in "Phase findings → P1", with a short list of
the rows most likely to be contested. **Stop.** The user reviews it before P2.

**Verify**: `git status` shows only this plan changed. Docs-truth passes.

### P2 — The user signs off the inventory (interactive)

**Goal**: every disposition in P1's table is the user's, plus the structural calls P1 surfaced.

**How**: `AskUserQuestion` rounds, grouped by disposition, contested rows first. Offer a
recommendation in each. Questions P2 must settle, beyond the table:
- **Archive or delete** a Python module once its Rust twin conforms. The user chose `archive/` for
  the research scripts; ADR-030's rule says *delete*. Ask.
- **The lab-port milestone's number and name** (a new one after M40's number, or a re-used paused
  one such as M24 or M29), and where it sits relative to M35–M38.
- **When the MCP server ports**: with the lab port, or earlier.
- **M35 and M36**: merge into one Rust-first milestone, or keep two with M35 Rust-first.
- **M29** (today: delete Python `analysis/` once `api/` and `mcp/` retire): superseded by the lab
  port, or kept as its final step.

**Files**: this plan only, and its P2 findings record every answer, quoting the user where they
wrote free text.

**Done when**: no row of the table reads "proposed". **Stop.**

### P3 — Write ADR-035

**Goal**: `docs/decisions/035-rust-everywhere-python-where-required.md`, in ADR-034's form (Status,
Date, Context, Options Considered, Decision with numbered clauses, Consequences, Deferred).

**Clauses** (order is a suggestion; P3 may reorder, and the addenda cite the final numbers):
1. The rule, and the two required exceptions (interview decision 1), with the user's words.
2. Everything else ports, and the three things considered and not kept (decision 2).
3. The oracle moves to Rust (decision 4): what `golf-core` records; that committed vectors remain as
   the Python record; that a Rust re-record is **diff-gated** (planning finding 4) so a regression
   pin cannot silently become a self-portrait; that new behaviour needs hand-worked vectors; that
   `ANALYSIS_VERSION` diverges by design (planning finding 7).
4. The frozen Python lab: what frozen means (no new behaviour; a fix only when the lab breaks), and
   that it must keep reading what Rust writes (P1 finding 1).
5. The lab port (decision 5, with P2's number): its scope, the OCR accuracy gate, the archive move
   (decision 3), and what retires, archived or deleted per P2.
6. Order: phone path first (decision 6).
7. What it supersedes, each by name: ADR-030's 2026-09-22 addendum (Python as recorder, and the
   schedule), ADR-032 §7's third clause, ADR-034 §9 in part, and the program plan's M35 "Python
   first". Plus whatever P1 finding 4 adds.

**Files**: the ADR (new); `docs/README.md` (the ADR row, ADRs **35 → 36**, markdown documents and
`docs/` +1).

**Done when**: the ADR exists and is tracked, every decision above is in a clause, and docs-truth
passes. **Stop.**

### P4 — Addenda, "who is the oracle and what retires": ADR-030, 032, 034

**Goal**: one addendum each, routing to ADR-035's clauses and naming the sentences of the original
that became false.
- **ADR-030**: the 2026-09-22 retirement rule is replaced (clause 3, 5); the Python sidecar shrinks
  to MediaPipe; the lab is ported. Status phrase **"Three addenda" → "Four addenda"**.
- **ADR-032**: §7's third clause, and who records vectors from now on (clause 3). Status phrase
  **"twelve addenda" → "thirteen addenda"**.
- **ADR-034**: §9's oracle-per-family table and the Consequences that route M35 to Python (clause
  3, 7). ADR-034 does not count its own addenda.

**Files**: the three ADRs; `docs/README.md` rows 030/032/034 and the total (**86 → 89**, re-count).

**Done when**: docs-truth passes and every link in the three addenda resolves. **Stop.**

### P5 — Addenda P1 named beyond those three (split into P5a and P5b)

**Goal**: an addendum to each further ADR P1 finding 4 named, in the same form as P4. **If the list
is longer than four, split this phase (P5a, P5b) before starting**, and update the checklist.
Likely: ADR-014 (the lab's OCR engine goes Rust via `ort`), ADR-022 (fitting is archived; the JSON
stays), ADR-006 (the MCP server ports to `rmcp`), ADR-008 (the import rule's Python half freezes),
ADR-016 (FastAPI ports or drops in M40), ADR-033 (only if something in it changes; its self-count is
**"Five addenda"**).

**Split before starting (2026-09-30), by the rule above.** P3 finding 3 settled the list at seven
ADRs plus ADR-001's Status, and ADR-008 gets none (ADR-035 clause 7). Each half takes at most four,
grouped by what ADR-035 does to them.

#### P5a — What M29 ports: ADR-006, 014, 020, 022

- **ADR-006**: the MCP server ports to `rmcp` (clause 2). The tool surface survives, and Python
  `mcp/` is deleted in M40 (clause 5).
- **ADR-020**: Option C cannot outlive the MCP port. The LLM reaches the Rust server over stdio, which
  is Option B (Q9, clause 7).
- **ADR-014**: the M31 addendum's lab column. The reader ports via `ort` behind the 13-photo gate, and
  the parser is recorded from frozen Python once (clauses 2, 3 and 5).
- **ADR-022**: §1's fitting stage is archived in M29. The artifacts stay, and Rust evaluates them
  (clauses 5 and 7).

**Files**: the four ADRs; `docs/README.md` rows 006/014/020/022 and the total (**89 → 93**,
re-count).

**Done when**: docs-truth passes, and every link in the four addenda resolves. **Stop.**

#### P5b — The order and the M40 line: ADR-016, 024, 033, and ADR-001's Status

- **ADR-016**: "M29 retires `api/` once M40 has landed" now means M40 decides and deletes, and
  `scripts/import_phone_export.py` becomes a verb of M29's lab CLI (Q8, clause 5).
- **ADR-024**: the M31 addendum's "§M35 … Python, which is M36's oracle" and "M36 ports … with
  Python as the oracle" (clauses 3 and 6). Its `:314` anchor is the one P6b's rename of §M35 breaks,
  so check the anchor's target before citing it.
- **ADR-033**: the `:322` pointer (deleting the Python keypoints writer) moves to M40, and the
  `sidecar/` package pointer (`:348`) is checked. The protocol is unchanged. Its self-count is
  **"Five addenda"**, which moves to six.
- **ADR-001**: a pointer from its Status to ADR-035, or a short addendum. If it is an addendum,
  ADR-035's Status ("It amends ten ADRs") and its README row move to eleven in the same change.

**Files**: those ADRs; `docs/README.md` rows and the total (re-count).

**Done when**: docs-truth passes, and every link in the new addenda resolves. **Stop.**

### P6a — Program plan: the re-plan section, the status table, and M32 re-detailed Rust-only

**Goal**: `docs/plans/m31-m40-shot-first-pivot.md` says what M31.5 changed, and its §M32 is a
Rust-only milestone a fresh session could plan phases from.

**Edits**:
- A **"Re-planned by M31.5 (2026-09-30)"** section near the top: the rule, routing to ADR-035;
  which milestones moved; that the Decisions 1–17 still stand.
- **Status checklist**: M31.5's row; every row whose state, dependency or scope P2 changed; the
  lab-port milestone's row.
- **§M32 rewritten** as Rust-only. What stays, from the old §M32:
  - `crates/contracts/src/shot.rs`: the seven `ShotData` fields (the old table's units stand) and the
    three `ShotProvenance` fields, `#[serde(default)]`; `SCREEN_PARSER_VERSION` and
    `parse_is_current`.
  - `crates/contracts/src/capability.rs`: `FieldUse{Analysed, ShownOnly}` with a note on every
    `ShownOnly`, `DeviceCapability`, `capability_for`, `device_of`, `printed_fields`; `devices.json`
    with `hd_golf` and `mock`, **located in `crates/`** (nothing Python reads it). Settle whether
    `printed_fields(shots)` is the union over shots of (declared ∩ `fields_present`), with a
    per-shot `printed_on(shot)` so a shot from a layout lacking the tile is excluded rather than
    counted blank; the planning session recommends that.
  - `printed_blank` and `misread` added to `crates/contracts`' unscored reasons and their prose
    table (carried decision 5).
  - `ANALYSIS_VERSION` 16 → 17 in Rust only; the diff-gated re-record of every engine and stage
    vector by `golf-core` from committed inputs (P1 finding 3); the schema question settled (P1
    finding 2).
  - What happens to Python's `conformance.py check` and `tests/test_conformance.py` the moment the
    first Rust-only shape change lands. This is the one place M32 touches Python, and why.
  - The stale comments, fixed in the Rust mirrors (carried decision 6).
  - **Moves out to M34**: everything in the old "Screen package" list, `api/pipeline.py::_shot_for`,
    `mcp/query.py::_METRIC_FIELDS`, the sequence, and the old exit's re-read (carried decisions 1–4).
  - New exit, tests and pins, written against the Rust files.
- **Errata** at the foot of "What the code says": planning findings 1–3 above.

**Done when**: §M32 names files, reused functions and tests in the form CLAUDE.md's "Writing a
plan" asks, and docs-truth passes. **Stop.**

### P6b — Program plan: M33–M40, the lab-port milestone, invariants and verify

**Goal**: the rest of the program plan agrees with ADR-035 and P2.

**Edits**:
- **§M33**: unchanged in substance (the Vision spike still feeds today's Python parser, which is
  frozen, not deleted); say so in one line, and note that the frozen parser carries the V-tile tie.
- **§M34**: gains the parser work and the 13-shot re-read with its exit (carried decisions 1–4), the
  `profiles.json` ↔ `devices.json` pin, and the "hide from golfer" filter wherever a Rust surface
  shows warnings. It is new behaviour on a port, so its vectors are Python-recorded for what exists
  and hand-worked for what is new.
- **§M35/§M36**: rewritten per P2 (merged or not), Rust-first.
- **The lab-port milestone**: a new section: scope (Rust CLI, OCR via `ort` with the 13-photo gate,
  storage, reanalyze, the vector regenerate path from `data/`), the archive move and the `research`
  extra, the MCP port if P2 put it here, and what retires.
- **§M37–§M40**: whatever P2 moved; M40 loses anything the lab port took.
- **"Reuse, don't rebuild"**: each Python entry gains its Rust home or "ports in <milestone>".
- **"Invariants every milestone keeps"** and **"Verify, every milestone"**: the Python conformance
  check leaves the verify list at the milestone P6a names; the stdlib-only-scoring invariant is
  restated for Rust alone.

**Done when**: no section of the program plan plans new Python beyond the required exceptions, and
docs-truth passes. **Stop.**

### P7 — `ROADMAP.md`: the M31.5 row and section, and the milestones the re-plan moves

**Goal**: ROADMAP's status table and sections agree with the program plan.

**Edits**: an **M31.5** status row (🟡 until P9) and a short section in the M31–M40 group's form
(status, the ask, depends-on, where it runs, exit, link). Re-scope rows and sections for every
milestone P2/P6 moved, plus the new lab-port milestone and M29. Rewrite the `NEXT ACTION` paragraph
P0 touched. Sections stay short and route to the program plan (M31's Decision 16).

**Done when**: docs-truth passes, every anchor resolves, and line endings are what they were. **Stop.**

### P8 — `CLAUDE.md`, `docs/CONFORMANCE.md` and `docs/FLOW.md`

**Goal**: the two docs a fresh session reads first stop telling it that Python is the oracle, and the
flow map stops drawing M31's order.

**`CLAUDE.md`**: the invariants "Python keeps only what does not translate" (now: only MediaPipe and
AI), "The analysis core is stdlib + contracts only" (the Rust half is the one that matters), and
"An `ANALYSIS_VERSION` bump regenerates `spec/vectors/`" (who regenerates, from M32); the Commands
block's `conformance.py check` line (what it certifies, and until when); the rows "What is the app
written in", "Why did a module leave Python" and "How is a port checked". Keep it a router: point
at ADR-035, copy nothing. **`docs/CONFORMANCE.md`**: §4's commands and §5's oracle paragraph (M31
edited §5; read what it says now). Also `docs/README.md`'s map rows for any doc whose trust tier or
summary the re-plan changed.

**`docs/FLOW.md`** (CRLF). No phase listed it until P7 (P5b finding 3, P6b finding 9), so P7 handed it
here. ROADMAP's shot-first group and §M29 now carry the order, so FLOW mirrors them. Line numbers
were read on 2026-09-30, so re-check them first:
- **The pivot graph** (`:80–105`). The M35 node reads "in Python" (`:85`). The edges draw M31's
  order, `M32 --> M35 --> M36 --> M37` (`:94–95`), where it is now M32 → M36 → M35 → M37. M29 is
  "the last Python", hanging off `M40 --> M29` (`:102–104`), where it is now the lab port, fed by
  M34 and M36 and feeding M40 and M38 P4. The subgraph title names ADR-034 alone.
- **The critical path** (`:130–135`): "M31 → M32 → M35 → M36 → M37 → M38".
- **What the pivot paused** (`:137–141`): "the sidecar's [first caller] is M40" (it is M29's lab
  CLI), and "M29 waits on M40 instead of M25, and its job is unchanged".
- **The lab and the laptop client** (`:391`, `:398–405`): "it stays until M29", the sidecar's first
  caller as M40, and "Once it lands, M29 retires `api/` into it, ports `mcp/` to Rust and deletes
  `analysis/`". M29 ports and M40 decides `api/` and deletes. The lower graph's `DESK -->|"first
  caller"| SIDECAR` edge (`:369`) needs the same check.

**Done when**: docs-truth passes. **Stop.**

### P9 — Close: verify, WORKLOG, checklists, memory

**Goal**: M31.5 closes with the suite as green as M31 left it.

**Steps**:
- Run the whole verify suite below.
- A WORKLOG entry at the top, in M31's form: what was decided, what P1 found, what M32 should read
  first.
- This plan's checklist, and the program plan's M31.5 row → ✅; ROADMAP's M31.5 row → ✅.
  `docs/FLOW.md`'s pivot graph too (P8 finding 4): the `M315` node's 🟡 → ✅, and `M315` moves from
  the `wip` class line to the `done` one.
- Memory: update `app-transition-decisions.md` (next is M32, Rust-only) and
  `rust-everything-python-only-where-required.md` (the ADR number, and the re-plan done).

**Done when**: all six verify commands are green, and `git status` shows docs only. **Stop.**

---

## Verify (docs-truth every phase; the whole suite at P9)

```bash
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py     # every phase
.venv/Scripts/python.exe -m pytest                              # P9
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
.venv/Scripts/python.exe scripts/conformance.py check
cargo test
cargo clippy --all-targets && cargo fmt --check
```

Run `cargo test` unpiped, or check `PIPESTATUS` (WORKLOG, M23 P9), and not at the same time as
pytest (the program plan's "Verify, every milestone").

---

## Phase findings

*(Appended as phases close.)*

### P1 — found (2026-09-30)

Read-only. Only this section and the checklist changed. Line counts are `wc -l` from 2026-09-30,
and every area total matches the planning-time figure. **LP** below means the lab-port milestone
(interview decision 5), whose number P2 chooses. Rows are numbered so that P2 can cite them.
**P2 made LP M29, re-scoped**, and rewrote the table's last-but-one column with the signed-off
dispositions. Everything else in this section is P1's record as it was found.

Totals: `src/golf_coach/` 32,698 · `scripts/` 13,360 (top level 6,971, `golfdb/` 5,477, `caddieset/`
912) · `tests/` 33,109 · `spikes/` 1,621 · one Python file under `crates/` (231).

#### The inventory

| # | Area | Lines | What it does | Rust twin (conforms?) | Python required? | Shot-first? | Kept alive by | **Disposition** (signed off, P2) | Why |
|---|---|---|---|---|---|---|---|---|---|
| R1 | `analysis/`, the ported engine: smoothing, phases, measure, stats, pivot, trajectory, scoring, engine, `checkpoints/`, `benchmarks/*.py`, alignment's analysis half, shot_measure, flight, flight_infer, spin_solve, most of flight_measure | 9,640 | one swing and its shot in, a scored `SwingBundleResult` out | `crates/analysis` + `core`. **Yes**: 21/21 engine vectors, 21 stage vectors | neither | shot half (shot_measure, flight*) yes; pose half only for the optional video | `api/pipeline.py`, `api/flight_view.py`, `mcp/flight.py`, 22 scripts (11 of them in `golfdb/`) | **Frozen**; the Rust lab replaces it at **M29**; **deleted at M40**, with `api/` (Q1, Q17) | the twin conforms, and only lab callers hold it (ADR-032 §7) |
| R2 | benchmark JSON in `analysis/benchmarks/` (six files) | data | bands, reference distributions, and the joint, trajectory and flight models | read by `crates/analysis` through `include_str!` **from this path** | neither | yes | `crates/analysis` | stays put now (decision 3); **moves crates-side at M40**, in the change that deletes `analysis/` (Q17) | finding 12 |
| R3 | `analysis/` many-shot layer: baseline, dispersion, club_profile, comparison | 923 | a golfer's history turned into what it may say: per club, and against the tour | none | neither | **yes**: per-club stats are the product | `api/app.py`, `mcp/{career,club}.py`, the CLIs in R29 | **Port → M36**, which now runs before M35 (Q4). Python deleted at M40 | no twin, and it is the core of shot-first |
| R4 | `analysis/tempo_trainer.py`, `analysis/flight_caveats.py` | 605 | a metronome built from tour durations; the sentences a flight may not be quoted without | none | neither | tempo: low (needs video). Caveats: yes, wherever a flight is shown | `api/{app,state,flight_view}.py`, `mcp/flight.py` | `flight_caveats`: **port with rmcp at M29**. `tempo_trainer`: **decided at M40**, with the web UI (Q12). Python deleted at M40 | Python-only; no vector reaches either |
| R5 | `alignment.py`'s render half (`warp_speeds`, `pair_frames`) and ~130 lines of `flight_measure.py` | inside R1 | pairing frames for the aligned render; flight helpers only a script and a page use | none, by ADR-032 §8 | neither | low | the render in `api/pipeline.py`, `align_swings.py`, `simulate_flight.py`, `flight.html` | **Not ported** (Q10). Deleted at M40, with the frozen server that calls them (Q17) | no committed vector reaches either |
| R6 | `contracts/`, ported: alignment, checkpoints, detections, feedback, golfer, intent, keypoints, pivots, placements, shot, swing, unscored | 2,328 | the payload shapes and the three registries | `crates/contracts`. **Yes**: round-trips every vector | `keypoints.py` for the pose worker; `swing`, `shot`, `unscored`, `alignment`, `pivots`, `placements` and `feedback` for `coach.py` | yes | every area | **Frozen**; the Rust copy is the live contract from M32. Of the ported shapes only `keypoints.py` outlives M40 (Q14); the rest is deleted then | from M32 the Rust copy is the live contract; finding 1 covers what the frozen copy must still read |
| R7 | `contracts/`, unported: audio, bag, baseline, career, caveats, club, club_profile, club_spec, comparison, conversation, dispersion, mishit, reference, tempo, tool_descriptions, `__init__` | 4,550 | the many-shot, bag and club shapes; the MCP prose; the conversation transcript | none | `conversation.py` and `tool_descriptions.py` serve the LLM | career, bag, club and mishit: yes. reference: no | the many-shot layer, `storage/`, `mcp/`, `api/`, `clubs/`, `feedback/` | each **ports with its consumer**: career, baseline, dispersion, club_profile, mishit, bag, club and comparison → M36; caveats and tool_descriptions → rmcp at M29; audio → M29; tempo → M40 with R4; reference → **archived with `golfdb/` at M29** (only `golfdb/` imports it). conversation **stays** (the LLM's own shape); club and club_spec also stay, for R24 (P2 finding 4). Python copies deleted at M40 | each has one consumer to travel with |
| R8 | `api/pipeline.py` | 1,368 | one bundle in, everything out: pose, OCR, strikes, engine, tips, coaching, render | `crates/core::run` covers only the engine and tips calls | neither (it *calls* both required things) | yes, it is the lab | `analyze_bundle.py`, `reanalyze.py`, `api/worker.py` | **Port → M29**: the Rust lab CLI replaces it. The Python stays frozen as the FastAPI worker's pipeline and is deleted at M40 (Q17) | orchestration, which ADR-030 §1 gives to Rust |
| R9 | `api/{app,worker,flight_view,state,__init__}.py` | 2,436 | the FastAPI upload server, the background worker, and the results, career and flight routes | none | neither | low: the phone is the host | `run_server.py`. `api.state` is also imported by `conformance.py` and `reanalyze.py`, and *upward* by `storage/corpus.py` and `mcp/query.py` (ADR-008's exception) | **M40: port to `axum` or drop** (decision 2). `state.py`'s tolerant readers → M36's `crates/storage` | the ADR-008 exception retires with them |
| R10 | `api/static/`: five pages + `tempo.js` | 4,721 (HTML/JS) | the web UI | none | – | low | `api/app.py` | M40, with R9 | not Python; listed so that nothing is orphaned |
| R11 | `mcp/{server,query,career,club,flight,__init__}.py` | 2,926 | the stdio MCP server: Claude's read interface to swings, shots, career, bag and flight | none | neither: there is no AI inside it | yes | `run_mcp_server.py`; R12 | **Port → `rmcp` at M29** (Q5). The Python server is deleted at M40, with the frozen `api/` that reaches its query layer (P2 finding 2) | decision 2 |
| R12 | `mcp/runner_tools.py` | 265 | the same tools, offered to the in-process Anthropic tool runner | none | imports `anthropic` and serves the LLM | yes (on the laptop) | `feedback/conversation.py`, `api/app.py`, `ask_swing.py` | **Replaced at M29**: `conversation.py` drives the rmcp server over stdio (Q6, Q9; ADR-020's Option B, via the SDK's `anthropic.lib.tools.mcp`). Deleted at M40, with the frozen `api/app.py` that imports it | it is the seam between R11 and R23 |
| R13 | `storage/` | 1,904 | flat-file stores (bundle, manifest, session, golfer, bag, transcript, keypoints, audio), the corpus reader, and the join from shot to swing for the flight inputs | only `crates/pose::writer`, which writes keypoints | neither | yes | `api/`, `mcp/`, 23 scripts | **Port → M36** (manifest, bundle store, corpus and shot store) **and M29** (the rest). `transcript_store.py` **stays Python**, with R23. Python copies deleted at M40 | ADR-030 §1 gives storage to Rust |
| R14 | `launch_monitor/screen/{parser,validate,profiles,recognizer}.py` + `profiles.json` | 742 | boxes → a shot; the physics cross-checks; device profiles; the `TextBox` seam | none yet | neither | **yes** | `importer.py`, `api/pipeline.py` | **Port → M34** (`crates/screen`). Python frozen, deleted at M40 | carried decision 1 |
| R15 | `launch_monitor/screen/{paddle,preprocess,importer}.py` | 650 | PaddleOCR, OpenCV rectification, and photo → stored shot | none | neither: PaddleOCR is a dependency, but the user ruled that it ports | yes (it is the lab's reader) | `import_shot_screens.py`, `api/pipeline.py` | **Port → M29** via `ort` with the same Paddle models, gated on the 13 bay photos. Python deleted at M40 | decision 2 |
| R16 | `launch_monitor/{source,mock,composite}.py`, `screen/{store,source}.py`, the two `__init__`s | 334 | the `ShotDataSource` port, the mock source, the composite, and the parsed-shot cache | none | neither | yes | `api/`, `mcp/`, scripts | **Port → M36** (the shot store) and M29. Python deleted at M40 | CONFORMANCE §5's tier 2 already says so |
| R17 | `pose/{estimator,worker}.py` | 688 | MediaPipe Tasks pose, and the sidecar worker that `crates/pose` spawns | `crates/pose` is its caller (ADR-033) | **MediaPipe** | the optional video | `crates/pose`, `api/pipeline.py`, five scripts | **Keep, as a Python worker** | decision 1 |
| R18 | `capture/` | 198 | `FileVideoSource` (an OpenCV decoder) and the `VideoSource` port | `crates/capture` is the camera edge, not a port of this module | **yes, for the pose worker**: `worker.py` decodes through `FileVideoSource` | the optional video | `pose/{worker,estimator,side_by_side}.py`, `api/pipeline.py`, six scripts | **Keep, with R17** (Q11). CONFORMANCE §5 is corrected | CONFORMANCE §5 files it as "ports to Rust", and the worker says otherwise |
| R19 | `pose/{overlay,side_by_side}.py` | 501 | the OpenCV skeleton overlay; the aligned two-view `aligned.mp4` | none | neither: this is drawing, not MediaPipe | low: the video is optional, and this is a lab artifact | `api/pipeline.py`, `analyze_swing.py --overlay`, `align_swings.py`, `run_pose.py` | **Not ported** (Q10). Deleted at M40: the frozen pipeline calls it until then (Q17) | no twin, no vector, and a render only the lab uses |
| R20 | `audio/{ffmpeg,source}.py` | 361 | a clip's audio → PCM through `imageio-ffmpeg`'s binary; the video-start probe | none | neither | the optional video (strikes are its clock) | `api/pipeline.py` | **Port → M29**: Rust shells out to the same ffmpeg. Python deleted at M40 | CONFORMANCE §5 says "ports to Rust, next" |
| R21 | `audio/trigger.py` | 114 | pipes PCM to `golf-trigger` | `crates/trigger`. **Yes**: 30 audio vectors | neither | the optional video | `api/pipeline.py`, `trigger_replay.py` | **Replaced at M29**: the Rust lab calls the crate directly. Python deleted at M40 | it is a Python caller of Rust, and the Rust CLI calls the crate directly |
| R22 | `feedback/rules.py` | 166 | the ranked rule-based tips | `crates/feedback`. **Yes**: all 21 vectors carry the tips | neither | yes | `api/pipeline.py`, `analyze_swing.py`, `conformance.py` | **Retired**: the twin conforms. Python deleted at M40 | the twin conforms |
| R23 | `feedback/{coach,conversation}.py` | 850 | Claude's coaching brief; follow-up questions over the tool runner | none, by design | **LLM** | yes, on the laptop only (ADR-034 §10) | `api/pipeline.py`, `api/app.py`, `ask_swing.py` | **Keep** (Q9: the LLM stays Python). `coach.py` reads its swing and shot as JSON from the rmcp server, not as pydantic models (Q14) | decision 1. How Rust calls these once R8 and R9 are gone is still open (finding 9) |
| R24 | `clubs/lookup.py` | 688 | Claude recalls a club's published specification | none | **LLM** (`anthropic`, imported lazily) | yes: a bag's lofts feed the flight | only `api/app.py`'s `/api/clubs/lookup` | **Keep** (Q13). Its caller is decided at M40, with R9 | it is an LLM call |
| R25 | `clubs/{catalogue,__init__}.py` + `club_catalogue.json` | 284 | the committed catalogue, which makes a second lookup of a club offline | none | neither | yes | `api/app.py`, `clubs/lookup.py` | **Port → M36**, with the bag. The JSON becomes a two-language file (R24 writes it, Rust reads it) | CONFORMANCE §5 tier 2 |
| R26 | `detection/` | 29 | a YOLOv8 stub that raises `NotImplementedError` | none | neither | no (M1.5 closed it) | nothing | **Delete at M29** (Q1). Nothing imports it | there is no behaviour to port |
| R27 | `config.py` | 119 | settings: data paths, the API key, the pose model and variant | none (each crate takes its own arguments) | read by both workers | – | nearly everything | **Keep**; cut at M40 to what R17 and R23 read | the workers need it |
| R28 | `scripts/{analyze_bundle,analyze_swing,reanalyze,import_shot_screens,simulate_flight}.py` | 1,708 | the lab's entry points: a bundle, a keypoints file, stale re-runs, photo import, flight | `golf-core run` covers the engine call only | neither | yes | the user | **Port → M29**, as the Rust lab CLI and `golf-core` verbs. The Python scripts are deleted at M29 (P2 finding 2) | decision 5 |
| R29 | `scripts/{career_baseline,career_corpus,career_dispersion,club_profile,flag_mishit}.py` | 1,143 | CLIs over the many-shot layer | none | neither | yes | the user | **Port with R3 → M36**. The Python CLIs are deleted at M29, with the other lab scripts | they are R3's front door |
| R30 | `scripts/backfill_golfer.py` | 94 | a one-off attribution of old swings to a golfer | none | neither | no | nothing | **Delete at M29** (Q15) | a migration that has already run |
| R31 | `scripts/ask_swing.py` | 206 | the follow-up question CLI | none | **LLM** | yes (on the laptop) | the user | **Keep**, with R23 | decision 1 |
| R32 | `scripts/run_mcp_server.py` | 104 | starts the stdio MCP server | none | neither | yes | MCP clients | **Replaced by the rmcp binary at M29**, and deleted then | decision 2 |
| R33 | `scripts/run_server.py` | 142 | starts FastAPI | none | neither | low | the user | M40, with R9 | decision 2 |
| R34 | `scripts/{run_pose,align_swings}.py` | 601 | pose one clip and overlay it; align two views and render them | `golf-pose run` covers the pose half | neither (both call R17 in-process) | the optional video | the user | **Not ported** (Q10): `golf-pose run` already poses a clip. Deleted at M29; nothing imports them | `golf-pose` already poses a clip |
| R35 | `scripts/{conformance,conformance_vectors}.py` | 1,883 | the Python oracle: schemas out, vectors built from `data/`, and `check`/`run`/`regenerate` | `golf-core run` and `cargo test` | neither | – | `tests/test_conformance.py`, the verify list | **Retires with R1, at M40.** From M32 Rust re-records, diff-gated (decision 4); how `check` and its pins behave from then is P6a's (finding 3). The vector builder that reads `data/` becomes Rust at M29 | decision 4 |
| R36 | `scripts/{trigger_replay,pose_replay}.py` | 774 | Python harnesses that drive `golf-trigger` and `golf-pose` over the stored corpus | the binaries they drive | neither | the optional video | the user (the M20 and M23 measurements) | **Archive at M29** (Q15), as the M20 and M23 measurement records | they measure Rust from Python |
| R37 | `scripts/hand_landmark_reliability.py` | 316 | the M14 gate: can hand landmarks be measured at address? | none | neither | no | nothing | **Archive at M29**, with R38 | a closed research gate, the same kind as `golfdb/`'s `tune_*` |
| R38 | `scripts/golfdb/` (20 files) | 5,477 | GolfDB fetch, pose extraction, band and model derivation, the bake-off and the tuning gates | none | neither (numpy, scikit-learn, pandas; MediaPipe through `estimators.py`) | no | nothing at runtime | **Archive at M29** (decision 3, Q1). Runnable until M40 deletes `analysis/`, a record after (finding 11) | "no longer relevant" (the user) |
| R39 | `scripts/caddieset/` (four files) | 912 | CaddieSet ingestion, and the M8-PAIR study | none | neither | no | nothing | **Archive at M29** (decision 3) | the same |
| R40 | `tests/` | 33,109 | mirrors `src/golf_coach/`, one directory per area | – | – | – | – | each directory follows its area's row. `test_docs_truth.py` stays. `test_conformance.py` changes in M32 (finding 3). `tests/api/test_pipeline_imports.py` retires with R8, at M40 | – |
| R41 | `spikes/*/probe.py` (three) | 1,621 | throwaway probes from field spikes (M7 P0, M1.5, MediaPipe Hands) | none | the Hands probe uses MediaPipe | no | nothing | **Leave in `spikes/`**, as history (Q15) | they are records, not code anyone runs. Two of them import `golf_coach` and will rot with R1 |
| R42 | `crates/pose/tests/stub_worker.py` | 231 | an ADR-033 worker that never poses, for `crates/pose`'s tests | – | it stands in for R17 | – | `cargo test` | **Keep** | it is part of the gate on the MediaPipe boundary |
| R43 | `frontend/` | 7 (a README only) | M5's React placeholder, never built | – | – | no | nothing | **Delete at M29** (finding 6) | `api/` does not serve it; `api/` serves R10 |
| R44 | `pyproject.toml` extras | – | `vision`, `api`, `llm`, `hardware`, `ocr`, `audio`, `research`, `dev` | – | `vision`'s mediapipe and opencv; `llm`'s anthropic | – | – | keep `vision` without `ultralytics`, and `llm` whole: its `mcp` package becomes the client the LLM drives rmcp with (Q9). `api`, `ocr` and `audio` go at M40; `research` at M29, with the archive move (decision 3). Delete `hardware` (finding 13) | – |

**Rows most likely to be contested**, most consequential first:

1. **R12/R23: how the LLM reaches its tools once `mcp/` is Rust** (finding 9). The recommendation is
   that Python's `conversation.py` drives the Rust MCP server over stdio. That is ADR-020's declined
   Option B, served by the SDK's `anthropic.lib.tools.mcp` helpers, and it removes the last Python
   caller of `analysis/`.
2. **R11: when the MCP server ports.** The recommendation is with LP, because it is the last
   in-process consumer of the Python query layer. Earlier is possible.
3. **R19/R34/R5: the aligned render and the skeleton overlay.** The recommendation is to delete them.
4. **R18: `capture/` stays Python** as the pose worker's decoder, against CONFORMANCE §5's "ports to
   Rust".
5. **R24: `clubs/lookup.py` stays as an LLM exception.** Its only caller is `api/app.py`, which M40
   may drop.
6. **R6: how much of `contracts/` survives as the kernel the workers read.**
7. **R4: `tempo_trainer.py`**, ported at M40 or dropped.
8. **R30/R36/R41**: a one-off migration, the replay harnesses and the spikes. Each is kept as
   history, archived or deleted.
9. **R9/R10: `axum` or drop.** Decision 2 leaves this to M40, so P2 may not need to settle it.

**Questions P2 needs beyond its own five:**

- **Is frozen Python still the recorder for ports of behaviour it already has** (M34's parser, M36's
  aggregates)? Carried text for M34 says "Python-recorded for what exists and hand-worked for what is
  new". Decision 4 says "Rust is the oracle from now on". Recommended: yes. Record once from the frozen
  Python before anything moves, since it is the only reference that is independent of the port.
- **M38 P4's `scripts/import_phone_export.py` is planned as Python.** Under the rule it becomes
  Rust, writing the stored layout the frozen lab reads, and finding 1's rule then binds it.
  Otherwise LP has to land before M38 P4.
- **A Rust-first M35 depends on M36's `crates/storage`** (finding 7). That bears on the merge
  question.

#### Findings

1. **Every pydantic model reads with `extra="ignore"`, which makes unknown keys safe. Unknown values
   are not safe.**
   - **No model sets `extra`.** `grep -rn "extra=" src/golf_coach` finds only `config.py:39`, the
     settings object. There is no `model_config`, `class Config` or `strict=` on any model.
   - **So the default applies everywhere.** That covers all 54 models in `contracts/` (53 `BaseModel`
     subclasses plus `BagEntry(ClubSpec)`) and the ones in `storage/`, `api/`, `mcp/`, `clubs/` and
     `analysis/benchmarks/`. They all run pydantic v2's default, `ignore`.
   - **A Rust-written artifact with new keys therefore reads**, and the new keys are dropped.
     `ShotStore.get` (`launch_monitor/screen/store.py:52`) and `load_analysis` (`api/state.py:101`)
     both go through `model_validate_json`.
   - **Enums are closed, so a new value is refused.** `contracts/` declares 23 `StrEnum`s and one
     `IntEnum`. `UnscoredCheckpoint.reason: UnscoredReason` (`contracts/unscored.py:380`) would
     refuse carried decision 5's `printed_blank` and `misread`. `ShotData.source: ShotSource`
     (`contracts/shot.py:64`) would refuse a new source. `Field` bounds still apply as well.
   - **The rule P3's clause 4 needs:** until LP, Rust may add keys to anything the frozen lab
     reads. It must not write a value that the frozen enums or bounds reject.
   - **The two new reasons fit that rule, with one condition.** They are first emitted into
     `shot_analysis.json` (M35/M37), which the frozen lab never reads. They stay safe for as long as
     they stay out of `analysis.json`.
   - **Reading and then writing loses keys.** `ShotStore.put` (`store.py:58`) writes
     `model_dump_json`. A Python re-import of a Rust-written shot (`import_shot_screens.py --force`,
     `analyze_bundle.py --force-ocr`) therefore drops the Rust-only fields without a word. Frozen
     means nobody should run that, but nothing stops it.
   - **The reverse direction behaves the same way.** No contract shape in `crates/contracts` uses
     `deny_unknown_fields`. Only the round-trip test's harness does, and
     `crates/pose/src/protocol.rs`.
2. **`spec/schemas/` is generated by pydantic alone, and nothing in Rust reads it.**
   - **Where it comes from.** There are ten files, one per `SCHEMA_ROOTS` entry
     (`scripts/conformance.py:89`). `regenerate --schemas-only` writes each from
     `model_json_schema()`, with `sort_keys` and an indent of 2.
   - **Three pins in `tests/test_conformance.py`.** The committed bytes must equal
     `export_schemas()` (`:52`), every on-disk artifact must map to a root (`:104`), and no schema
     may lack a root (`:139`).
   - **Rust ignores them.** `crates/` cites them only in doc comments, and `Cargo.lock` has no
     `schemars` or `jsonschema`.
   - **Four of the ten roots have a Rust twin:** `keypoints_file`, `shot_data`, `swing_result` and
     `swing_bundle_result`. The other six do not:
     - `audio_file`: `crates/trigger` emits a `Strike`, and Python wraps it.
     - `golfer`: Rust has only the `Handedness` enum.
     - `swing_manifest`, `session_meta`, `bag` and `analysis_state`: nothing.
   - **A `ShotData` change moves three files, not one.** `swing_result` and `swing_bundle_result`
     embed `ShotData` and `ShotProvenance` in their `$defs`.
   - **Three candidates for P6a:**
     - (a) **`schemars` derives** in `crates/contracts`, with a freshness test like `:52`. Its output
       is not pydantic's (`$defs` naming, how nullable is spelled, titles), so the first run rewrites
       every file it owns.
     - (b) **Hand-maintained schemas**, plus a Rust test that validates the committed vectors against
       them.
     - (c) **Split ownership.** Rust generates the roots whose shape it now owns (the three above,
       in M32). Python's frozen export keeps the six with no Rust twin, and the freshness pin splits
       to match. This is the smallest change for M32, and it leaves two generators running until LP.
3. **`golf-core` can re-record the engine family today with a thin wrapper. The stage family has no
   Rust producer yet.**
   - **Engine (21 vectors).** `golf-core run` (`crates/core/src/bin/golf_core.rs`) reads a whole
     committed vector, or a bare input, and prints the serialized result. `crates/core/tests/engine.rs`
     already runs all 21 end to end. A re-record verb needs three things:
     - replace `expected` and `analysis_version`;
     - write the file the way `conformance.py::_write_json` does: serialize through
       `serde_json::Value` (a `BTreeMap` without `preserve_order`, so keys sort as Python's
       `sort_keys=True` does), indent 2, and gzip `corpus/` with mtime 0;
     - a diff gate. `engine.rs`'s structural `compare` is test-only, and it becomes library code with
       an allow-list of added keys.
   - **The text will churn even when nothing changes.** Rust writes the 77 exponent-form floats as
     plain decimals, and it does not `\u`-escape non-ASCII (every synthetic vector carries three or
     four escapes). So the gate must be structural, never `git diff`.
   - **Stages (21 vectors).** `run_stages` (`scripts/conformance.py:341`, ~185 lines) is the only
     code that assembles the seven stages into the recorded document. The seven are `smoothed`,
     `phases`, `measure`, `measurements`, `checkpoints`, `alignment` and `flight`.
     `_verify_stages_compose` (`conformance_vectors.py:454`) checks that the stages compose onto the
     bundle's answer.
   - **The Rust stage tests do not produce a stage document.** They recompute each stage and compare
     it field by field, and each test file reimplements the orchestration it needs (`windowed` in
     `crates/analysis/tests/geometry.rs`, for example). A Rust re-record needs a port of `run_stages`
     (into `core` or `analysis`) together with the compose check.
   - **`format` (5) and `audio` (30) need nothing.** `format` records CPython's formatter, which
     `pyfmt` mirrors, and it does not age with `ANALYSIS_VERSION`. `audio` is already frozen, and
     `regenerate` refuses to rebuild it.
   - **New vectors from `data/` are LP's job, not M32's.** They need the Rust storage readers (M36)
     and a port of `conformance_vectors.build_all`.
   - **What breaks in Python at the first Rust-only re-record** (for P6a):
     - `cmd_check` tests `analysis_version != ANALYSIS_VERSION` (`conformance.py:689`, `:699`).
       Vectors at v17 against Python's 16 report every engine and stage vector STALE, so `check`
       exits 1.
     - In `tests/test_conformance.py`, four tests fail: `:178` and `:403` (recorded at the current
       version), `:163` (still conforms, because the key sets now differ) and `:420` (the stage is
       what this build produces).
     - `:52` fails for any schema that Rust regenerates.
   - **`golf-core`'s module doc says the opposite of decision 4:** "`regenerate` is deliberately not
     a candidate: the vectors are the oracle and a port that can rewrite them is a port that passes
     by construction".
     - ADR-032 §7 has already answered that objection for the time after M29: once a port conforms,
       its re-record is a changelog, not an oracle.
     - ADR-035 brings that moment forward to M32, and the diff gate is what keeps it honest.
     - M32 rewrites the comment.
4. **ADRs P5 must amend.** The grep matches 13 ADRs, and most of the matches are other senses of
   "retire" (a bag's retired club, a warning that was retired). Beyond P4's 030, 032 and 034:
   - **Must amend** (six, **so P5 has to split into P5a and P5b**, by its own rule):
     - **ADR-014.** Its M31 addendum's table keeps the lab's reader as "PaddleOCR, unchanged" and
       "`preprocess.py` and OpenCV, unchanged", and says the parser port is "recorded from Python"
       (`:248`, `:256`). The reader now ports via `ort`. The parser is recorded from frozen Python
       only for what already exists.
     - **ADR-006.** Its Consequences say "MCP server is a standalone Python service". It ports to
       `rmcp`.
     - **ADR-020.** Option C, the tool runner over `query.py`'s functions in-process, cannot outlive
       the MCP port. Which route replaces it is P2's call (finding 9).
     - **ADR-022.** §1's table puts fitting in `scripts/golfdb/` under `research`, and ADR-032 §7
       reads that as "some of `scripts/` is permanent". Fitting is archived at LP. The artifacts
       stay, and Rust evaluates them.
     - **ADR-016.** It says "M29 retires `api/` once M40 has landed" (`:211`), and it names
       `scripts/import_phone_export.py` (M38 P4), which would be new Python.
     - **ADR-024.** Its M31 addendum routes to "§M35 … Python, which is M36's oracle" (`:314`, an
       anchor that P6b's rename breaks) and says "M36 ports … with Python as the oracle" (`:317`).
   - **Conditional:**
     - **ADR-001** (superseded). Its Status says Python remains "the lab: fitting …, the corpus
       tools, the conformance oracle, LLM coaching and OCR" (`:10–13`), and four of those five become
       false. A short addendum, or a pointer from its Status to ADR-035, is recommended, because this
       ADR is where a reader goes first to ask "why Python".
     - **ADR-033.** It points at §M29 both for deleting the Python keypoints writer and for deciding
       on a standalone `sidecar/` package (`:322`, `:348`). It needs a pointer only if P2 supersedes
       M29. Nothing in its protocol changes, unless P2 makes the LLM a worker on the same framing.
     - **ADR-008.** The Python half of the import rule freezes, and its one exception
       (`storage/corpus.py` and `mcp/query.py` → `api.state`) retires at LP. The recommendation is
       to fold this into ADR-035's supersedes clause rather than write an addendum.
   - **Need nothing:** 002 (MediaPipe stays), 007, 012, 013, 015 and 021 (their matches are other
     senses of "retire", or history); 026 (the lookup stays Python); and 031 (its "no Python
     reference" is about `crates/capture` and stays true, and R18 is now the reason
     `FileVideoSource` stays in the lab).
   - **The archive move will need path edits** in 002, 010, 012, 013, 021 and 022 (finding 14). That
     is LP's change, not an addendum.
5. **Where the docs call M35 Python, or M36's oracle; and what M29 says today.**
   - **M35 as Python, or Python as M36's oracle:**
     - `ROADMAP.md`:
       - `:54` is the status row, "Shot-first sessions, in Python", and `:1814` the heading.
       - `:1821` says "It is Python first because it is M36's oracle".
       - `:1829` is the exit: "`spec/vectors/shot/` recorded from Python".
       - `:1841` is M36's "with Python as the oracle".
     - `docs/FLOW.md:85`: the node reads "in Python".
     - `docs/decisions/024-…md:314` (the anchor `#m35--shot-first-sessions-python-which-is-m36s-oracle`)
       and `:317`.
     - `docs/decisions/032-…md:1212`: "M34's screen parser and M36's many-shot layer are ports".
     - The program plan: `:31` (status row), `:544` (heading), `:582` ("recorded from Python") and
       `:595` (M36: "Ports, with Python as the oracle").
   - **Renaming §M35 breaks three inbound anchors:** ROADMAP `:1833` and `:3411`, and ADR-024 `:314`.
     P6b and P7 fix them in the same change.
   - **Other new Python the program plan still plans.** P6a and P6b remove each of these:
     - M32: `contracts/capability.py` and the Python screen-package edits (`:399–449`).
     - M35 (`:547–590`): `contracts/shot_result.py`, a Python `analyze_shot`, edits to
       `api/pipeline.py`, `api/state.py::load_shot_analysis` and `storage/corpus.py`, new flags on
       `import_shot_screens.py`, `reanalyze.py`, and a new `tests/contracts/test_intent.py`.
     - M38 P4: `scripts/import_phone_export.py` (`:698`).
     - M40: "A laptop OCR recognizer behind the same boxes seam", with its language unstated
       (`:748`).
   - **M29 today** (`ROADMAP.md:3121`) is titled "The last Python — retiring the lab, and deleting
     `analysis/`". It is 🔒 blocked on M40 (status row `:48`).
     - Its ask already matches decision 1: "Reduce Python to the sidecar — MediaPipe pose, and the
       LLM — and delete everything else".
     - Settled there: `api/` retires into the Flutter shell rather than being ported, `mcp/` ports to
       Rust, and `analysis/` plus the non-sidecar half of `contracts/` are deleted.
     - Its "what stays" list is `pose/estimator.py`, `feedback/coach.py` and
       `feedback/conversation.py`. That list misses `pose/worker.py` (M23), the worker's imports
       (R18, R27, `contracts/keypoints.py`) and `clubs/lookup.py`.
     - Its opening question is how ADR-022's fitting scripts reach a measurement once
       `analysis/measure.py` is gone. Decision 3 dissolves it: those scripts are archived, not run.
   - **M29 is referenced from:**
     - ADRs: 016:211, 030:321 and :505, 032 (§7, about ten places), 033:322 and :348.
     - `docs/CONFORMANCE.md`: :8, :305, :527 and :544.
     - `docs/FLOW.md`: :102–104, :140, :391 and :404.
     - The program plan (:359, :749) and `CLAUDE.md:120`.
6. **The plan's area list is wrong about `frontend/`.** It says `frontend/` is "served by `api/`".
   In fact `frontend/` holds one 7-line README, a placeholder for M5's React UI, which was never
   built. What `api/` serves is `src/golf_coach/api/static/`, which is R10.
7. **A Rust-first M35 needs what M36 builds.** M35's work sits in `storage/corpus.py::read_corpus`
   (photo-only admission and the hash dedupe), `api/state.py` and `api/pipeline.py`. In Rust, the
   corpus and its stores are M36's `crates/storage`. So in Rust M35 depends on M36, the reverse of
   today's dependency.
8. **`clubs/lookup.py` is an LLM call.** It imports `anthropic` lazily (`:607`), so decision 1
   requires it to stay Python, although ROADMAP §M29 and CONFORMANCE §5 both still file it as
   "open". Its only caller is `api/app.py`'s `/api/clubs/lookup`, so if M40 drops `api/` it needs a
   new caller.
9. **`conversation.py` reaches its tools through Python `mcp/`, in process.** The chain is:
   - `mcp/runner_tools.py` wraps `mcp/{query,career,club,flight}`,
   - which call `analysis/`, `storage/` and `api.state`.

   Porting `mcp/` removes those in-process tools, which were ADR-020's chosen Option C. From there
   the LLM Python has two routes:
   - drive the Rust MCP server over stdio. That is ADR-020's declined Option B, and its cost was a
     spawn and a round trip per call, which a long-lived client session amortises.
   - keep a Python copy of the query layer. That copy would call `analysis/`, so under ADR-032 §7's
     third clause `analysis/` could never retire.

   The first route is the one decision 1 allows.
10. **The pose worker needs more Python than `pose/`.** `pose/worker.py` imports `config.settings`,
    `contracts/keypoints.py` and `pose/estimator.py`, and it decodes frames through
    `capture/file.py::FileVideoSource` (OpenCV).
    - `crates/pose/tests/stub_worker.py` is Python that stays with that boundary.
    - The alternative is for Rust to decode frames and pipe them across. ADR-033 chose a path in
      and decoding in the worker, and changing that is a protocol change, which is out of scope.
11. **An archived `golfdb/` cannot run once `analysis/` is gone.** Eleven of the 20 `golfdb/`
    scripts import `golf_coach.analysis`, and most of the rest import `contracts`, `storage`, `pose`
    or `capture`. `caddieset/` imports nothing from `golf_coach` and only needs numpy and
    scikit-learn. So P2's "archive or delete" for Python `analysis/` also decides whether the
    `golfdb/` archive can be run, or is only a record.
12. **The committed JSON lives inside the Python package, and Rust reads it from there.**
    - `crates/analysis` does `include_str!` on six files under
      `src/golf_coach/analysis/benchmarks/`.
    - `clubs/club_catalogue.json` and `launch_monitor/screen/profiles.json` are package data in the
      same way.
    - Retiring `analysis/` has to move the JSON first. Decision 3 keeps it where it is until then.
13. **Three extras carry dependencies nothing uses.** Python is frozen, so this is recorded rather
    than fixed.
    - **`vision`'s `ultralytics`** has no user other than the `detection/` stub, which raises.
    - **`hardware`'s `bleak`** is imported nowhere. The R10 adapter is M40's, via `btleplug`.
    - **`audio`'s `numpy`** served only `audio/impact.py`, which M20 deleted. No module in `audio/`
      imports numpy now, and the `pyproject.toml` comment that says it serves `impact.py` is stale.
14. **The archive move has a documentation cost.** 17 tracked markdown files mention
    `scripts/golfdb/` or `scripts/caddieset/`, 71 times in all (this plan not counted). No test imports those scripts, but
    LP has to rewrite those paths in the same change as the move.

### P2 — found (2026-09-30)

Interactive: five `AskUserQuestion` rounds, and only this plan changed. The answers are numbered
Q1–Q17 so that the table above and P3's clauses can cite them. The table's disposition column is
now the signed-off one.

#### The answers

| # | Question | The user's answer |
|---|---|---|
| Q1 | Retired Python whose Rust twin conforms: archive or delete? | **Delete.** Only the research scripts go to `archive/` (decision 3). ADR-030's rule stands, and the `golfdb/` archive becomes a record once `analysis/` is gone (P1 finding 11) |
| Q2 | The lab-port milestone's number and name | **M29, re-scoped** as the lab port. Its deletions were to be its last phase; Q17 moves them |
| Q3 | Where it sits | **After M36.** It depends on M34 and M36, runs on this box beside M37/M38, and is no longer blocked on M40 |
| Q4 | M35 and M36 | **Two milestones, M36 first.** Port faithfully first, with frozen Python as the oracle, then change behaviour on top with hand-worked vectors. The numbers stay, and M35 depends on M36 |
| Q5 | When the MCP server ports | **With M29** |
| Q6 | How the Python LLM reaches its tools once `mcp/` is Rust | The user wrote: "Can we migrate this to rust as well!" Q9 asked how far |
| Q7 | Is frozen Python the recorder for ports of behaviour it already has? | **Yes, once**, before the port moves (M34's parser, M36's aggregates). New behaviour is hand-worked. After that Rust re-records, diff-gated |
| Q8 | M38 P4's phone-export importer | **A verb of M29's Rust lab CLI.** M38 P4 waits on M29, and its exit compares against the Rust reader, not Python `read_corpus` |
| Q9 | How far the LLM moves to Rust (told there is no official Anthropic Rust SDK) | **Keep the LLM in Python.** The tools are Rust (the rmcp server), and `conversation.py` drives them over stdio: ADR-020's Option B. Decision 1 stands |
| Q10 | The aligned render and the skeleton overlay (R19, R34, R5) | **Delete; not ported.** Q17 sets when |
| Q11 | `capture/` as the pose worker's decoder (R18) | **Stays Python, with the pose worker** |
| Q12 | `tempo_trainer.py` (R4) | **Decided at M40**, with the web UI |
| Q13 | `clubs/lookup.py` (R24) | **Keep in Python.** Its caller is decided at M40 |
| Q14 | How much of `contracts/` survives (R6) | **Keypoints only.** `coach.py` reads its swing and shot as JSON from the rmcp server, as `conversation.py` does. See finding 4 |
| Q15 | R30, R36, R41 | R30 **deleted** at M29; R36 **archived** at M29; R41 **left** in `spikes/` |
| Q16 | Every other row | **Accepted as proposed**, with the answers above applied |
| Q17 | The conflict Q2 and Q16 raised: M29 deletes `analysis/`, but `api/` (left to M40) imports it | **M29 ports, M40 deletes.** The frozen Python engine, and everything the FastAPI server imports, keeps working until M40 decides `api/`. Then it is deleted |

P2's own five questions map as follows:
- archive or delete → Q1;
- the milestone's number and name → Q2;
- when MCP ports → Q5;
- M35/M36 → Q4;
- M29 superseded or kept as the final step → neither: M29 *is* the lab port (Q2), and its deletions move to M40 (Q17).

P1's three extra questions are Q7, Q8 and Q4.

#### Findings

1. **M29 is re-scoped, not replaced, so most of the ~20 references to it stay true in meaning.**
   ROADMAP §M29's ask already matches decision 1. What changes:
   - **Status.** "🔒 blocked on M40" becomes "depends on M34 and M36, after M36".
   - **Scope.** It is now the port: the Rust lab CLI, the rmcp server, OCR through `ort`, strike
     audio through ffmpeg, the rest of storage, the vector builder that reads `data/`, and the
     phone-export verb (Q8). Its deletions are limited by finding 2.
   - **Its "what stays" list** gains `pose/worker.py`, `capture/`, `config.py`,
     `contracts/keypoints.py`, the Python-owned shapes of finding 4, `storage/transcript_store.py`,
     `clubs/lookup.py` and `scripts/ask_swing.py`.
   - **Its opening question** (how ADR-022's fitting reaches a measurement) is dissolved by
     decision 3.
   - **References that change meaning.** ADR-016 `:211` ("M29 retires `api/` once M40 has landed")
     and ADR-032 §7 ("`analysis/` leaves in §M29, once `api/` and `mcp/` have") both now mean M40
     for the delete. ADR-033's `:322` pointer (deleting the Python keypoints writer) moves to M40,
     because the frozen pipeline holds the writer, so P1 finding 4's "conditional" ADR-033 addendum
     is now needed. Its `sidecar/` package pointer (`:348`) is P5's to check.
   - **The program plan's §M40** already ends "M29's deletions proceed once `api/` and `mcp/` are
     retired", which is Q17. It stays, and gets reworded to name finding 2's line.
2. **Q17 splits "retire" into two moments, and the line between them is the frozen server's
   import closure.**
   - **M29 replaces.** The lab's entry points switch to Rust: the lab CLI, the rmcp binary for MCP
     clients, and `conversation.py` over stdio. It deletes or archives only what nothing in the
     frozen FastAPI server imports:
     - the lab scripts (R28, R29, R30, R32, R34);
     - `detection/` and `frontend/`;
     - the archive move (R36–R39, `contracts/reference.py`, the `research` extra).
   - **M40 deletes** everything that closure reaches:
     - `analysis/`, `storage/`, `launch_monitor/`, `audio/`, `feedback/rules.py` and
       `pose/{overlay,side_by_side}.py`;
     - Python `mcp/`, because `api/app.py:1384` imports `mcp.runner_tools`, which wraps the query
       layer;
     - the ported half of `contracts/`, and `conformance.py`;
     - `api/` itself, unless M40 ports it.

     The benchmark JSON moves crates-side in the same change.
   - **Q10's "delete" therefore lands twice.** The two modules the frozen pipeline calls (R5, R19)
     go at M40, and the two scripts (R34) go at M29.
   - **M29 adds Rust routes beside the frozen Python ones. It does not rewire the frozen server.**
     `conversation.py` gains the stdio route while `api/app.py` keeps handing it in-process tools
     until M40. `coach.py` gains a JSON entry while `api/pipeline.py` keeps calling the pydantic
     one. M29's own plan names both seams.
   - **The frozen window is M32 → M40, not M32 → M29.** The frozen server reads what the Rust lab
     writes (`api/state.py::load_analysis`), so P1 finding 1's rule binds until M40: Rust may add
     keys, and must not write a value the frozen enums or bounds refuse. P3's clause 4 says so.
   - **The Python lab scripts are deleted at M29 rather than left frozen.** Once Rust writes
     `data/`, a Python re-import (`import_shot_screens.py --force`, `analyze_bundle.py --force-ocr`)
     would drop the Rust-only keys without a word (P1 finding 1).
   - **One hazard stays open for M29's plan.** The frozen server's upload path (`api/worker.py` →
     `api/pipeline.py`) still writes `data/` with Python between M29 and M40. M29 decides whether
     that path stays open.
3. **Q9's route exists in the installed SDK, and there is no Rust SDK.**
   - `anthropic` 0.121.0 in `.venv` has `anthropic.lib.tools.mcp` (checked 2026-09-30).
   - The `mcp>=2.0` package the route needs is already in the `llm` extra, so `llm` stays whole
     (R44).
   - Anthropic's official SDKs are Python, TypeScript, Java, Go, Ruby, C# and PHP. A Rust LLM would
     have meant raw HTTP and a hand-written tool loop, and the user weighed that in Q9.
4. **"Keypoints only" (Q14) covers copies of the Rust result contract. Three Python-owned shapes
   stay**, and P3 says so, so that nobody reads Q14 as "delete `club_spec.py`".
   - **`contracts/conversation.py`** is the transcript the LLM writes and
     `storage/transcript_store.py` stores. It has no Rust twin, and nothing in Rust reads it.
   - **`contracts/club.py` and `contracts/club_spec.py`** are what `clubs/lookup.py` (`:64–65`) and
     `clubs/catalogue.py` (`:60–61`) build and write.
     - They gain Rust twins at M36 (the bag), so the catalogue JSON becomes a file that Python
       writes and Rust reads.
     - That is P1 finding 1 reversed: Rust must accept every value Python writes. M36 pins it the
       way `profiles.json` is pinned.
   - **The caveat prose is still open.** `coach.py` imports from `contracts/caveats.py` (`:48`), and
     `conversation.py` imports `ONLY_CHECKPOINTS_ARE_JUDGED` (`:42`). That prose is derived from
     `CHECKPOINT_REGISTRY`, which is a Rust-contract copy. Under Q14 it reaches Python through the
     rmcp server (tool descriptions and results) rather than by import. M29's plan decides which,
     and a Python copy kept anyway would be a fourth survivor.
5. **Q4 reorders the dependency column, and moves one contract.**
   - **The new order.** M36 depends on M32, not M35. M35 depends on M36. M37 depends on M35, which
     it needs for photo-only shots and `ShotResult`. M29 depends on M34 and M36. M38 P4 depends on
     M29. M40 depends on M38 and M29. P6a/P6b write this, and P7 mirrors it.
   - **What leaves M36.** The program plan's M36 list ports `shot_result` to `crates/contracts`.
     That is M35's *new* contract, so under Q4 it leaves M36 and M35 builds it in Rust directly,
     with hand-worked vectors.
   - **What M36 ports.** It ports `read_corpus` as frozen Python behaves today, without photo-only
     admission. M35 then changes it.
6. **Q7 needs a one-time recorder for each new family, and the recorder is Python.**
   - **The families.** Recording M34's parser and M36's `career/` and `storage/` families from
     frozen Python means adding them to `scripts/conformance_vectors.py`, the frozen recorder.
   - **P3's clause 4 has to allow it.** A recorder adds no behaviour to the lab, so clause 4 names it
     as the sanctioned exception to "frozen".
   - **The sequence is Q4's, again.** For M34, record today's parser, including the `CENTER` spill on
     `2026-08-10-1`, port it, then apply carried decision 1's tie rule and the V tile in Rust with
     hand-worked vectors. The two label-fix shots are the only intended diffs, which is carried
     decision 4's exit.

### P3 — found (2026-09-30)

Docs only. The new ADR is
[`035-rust-everywhere-python-where-required.md`](../decisions/035-rust-everywhere-python-where-required.md),
tracked. `docs/README.md` gained its row and moved its counts: markdown documents 77 → 78, `docs/`
64 → 65, ADRs 35 → 36. Docs-truth passes.

1. **The clauses kept the suggested order, so P4's clause references hold as written.** The anchors
   the addenda cite are:
   - `#1-the-rule-and-the-two-exceptions-it-names`
   - `#2-everything-else-ports-including-the-three-things-considered-and-not-kept`
   - `#3-the-oracle-moves-to-rust`
   - `#4-the-frozen-python-lab`
   - `#5-the-lab-port-is-m29-re-scoped`
   - `#6-order-the-phone-path-first`
   - `#7-what-this-supersedes-sentence-by-sentence`

   Clause 1 carries the survivor list, by name, which is P2 finding 4's "P3 says so". Clause 7
   quotes each superseded sentence, so an addendum can cite the clause rather than re-quote it.
2. **`docs/README.md` has a second ADR count that the plan did not list.** `:107` reads "N decisions,
   M addenda between them", and N excludes the template. It moved 34 → 35. Nothing pins N, only M,
   so a later phase that adds an ADR has to remember it.
3. **The ADR names ten ADRs to amend, and P5 owns seven of them.** P4 has 030, 032 and 034. P5 has
   006, 014, 016, 020, 022, 024 and 033, so **P5 must split into P5a and P5b** before it starts, by
   its own rule.
   - ADR-008 gets no addendum. Clause 7 folds it in, as P1 finding 4 recommended.
   - ADR-001 is left to P5: a pointer or a short addendum. If P5 writes an addendum, ADR-035's
     Status ("It amends ten ADRs") and its README row move to eleven in the same change.
4. **Planning finding 7 holds, checked against code.**
   - `api/state.py::is_outdated` compares `stored_analysis_version(...) < ANALYSIS_VERSION`, so frozen
     Python at 16 reads a Rust-written v17 artifact as current, and `scripts/reanalyze.py` leaves it
     alone.
   - `SwingBundleResult.analysis_version` has no upper bound.
   - Clause 3 cites the first of these.
5. **Three readings the ADR had to make, where the interview left a gap.** Each is flagged so that
   the user or a later phase can contest it.
   - **"Another library that we have a dependency on"** (the user's first message) is read as: a third
     exception needs its own ADR or addendum, and is never the default (clause 1). Decision 2 rejected
     PaddleOCR, although it is a dependency, so a dependency alone cannot be the bar.
   - **"A fix only when the lab breaks"** is read as a crash, or a refusal to read what Rust wrote.
     A stale comment or a known defect stays (clause 4).
   - **The OCR gate** compares the shot parsed from the Rust reader's boxes with the shot parsed from
     PaddleOCR's boxes on each of the 13 bay photos. What may differ is M29's to set (clause 5).
     Interview decision 2 said only "an accuracy check on the 13 stored bay photos".
6. **A consequence none of the earlier phases wrote down:** from M34 until M29, the phone and the lab
   read the two label-fix shots differently. The Rust parser applies the tie rule, and the frozen
   Python parser does not. P6b's §M34 and §M29 should say so.
7. **`ort` is a second numeric library outside the scoring path**, beside `crates/trigger`'s
   `rustfft`. CLAUDE.md calls `rustfft` "the one sanctioned exception", so P8 must reword that
   invariant so that it still reads true once M29 lands.
8. **A test hazard for P4 and P5.** `_SELF_COUNT` (`tests/test_docs_truth.py:608`) matches
   `\*\*(\w+)\s+addenda` anywhere in an ADR. So bold text ending in a word followed by "addenda" is
   read as a self-count, and it fails unless the count is right. ADR-035 has no such phrase.

### P4 — found (2026-09-30)

Docs only. Three addenda, one each in
[ADR-030](../decisions/030-app-platform-rust-core-python-sidecar.md#addendum-2026-09-30--what-is-required-replaces-what-does-not-translate-and-the-lab-is-ported-rather-than-kept),
[ADR-032](../decisions/032-the-rust-core.md#addendum-2026-09-30--rust-records-from-m32-and-7s-schedule-is-m29-to-port-and-m40-to-delete)
and
[ADR-034](../decisions/034-shot-first-phone-first.md#addendum-2026-09-30-m315-the-oracle-moves-to-rust-the-labs-reader-ports-and-m35-follows-m36).
Each routes to ADR-035's clauses by anchor, and each ends with what it does not change. The addenda
total moved 86 → 89. ADR-030's self-count moved to "Four addenda" and ADR-032's to "thirteen
addenda". Docs-truth passes, and all 239 links in the three ADRs, ADR-035 and `docs/README.md`
resolve, anchors included. They were checked with a GitHub-slug resolver kept in the session
scratchpad, not in the repo.

1. **The Status blocks changed as well as the addenda**, following M31's precedent (ADR-030's
   "Partially superseded by ADR-034" paragraph).
   - Each of the three Status blocks gained a "Superseded in part by ADR-035" sentence.
   - ADR-032's "Two implementations stand until §M29, which ADR-034 has blocked on M40" was
     rewritten rather than left for the addendum to contradict.
   - `docs/README.md`'s three rows gained the same note in their Status cells, and a summary in their
     addenda cells.
2. **Heading forms.** ADR-030 and ADR-032 use `## Addendum, 2026-09-30 — …`, which is their own
   form. ADR-030 separates addenda with `---`, and ADR-032 does not, and each file keeps its own
   habit. ADR-034 had no addenda yet, so it took this plan's `## Addendum (2026-09-30, M31.5): …`.
   Line endings are unchanged: ADR-030, ADR-032 and `docs/README.md` are CRLF, and ADR-034 is LF.
3. **ADR-034's Consequences never say "M35 is Python" in words**, whatever P4's goal line implies.
   - Three of them hand M35 work: a policy for a mode other than `FUNDAMENTALS`, photo-only corpus
     admission, and a handedness route for a photo-only shot.
   - M31's program plan then put that work in Python.
   - The addendum routes all three to Rust, after M36 (clause 6), and names `shot_result`'s move
     from M36's list to M35's.
4. **For P6a: a re-recorded family has no oracle label yet.** The twelfth addendum of ADR-032 lists
   the kinds: Python-recorded, hand-worked, Rust-recorded. A family that Python recorded and Rust
   then re-records under the gate is none of them as written.
   - M37's pin will read `provenance.oracle`.
   - None of the five families carries that field today (ADR-032's twelfth addendum, read
     2026-09-29).
   - So §M32 has to say what the engine and stage families' provenance reads after the first
     re-record. ADR-032's thirteenth addendum names the question and leaves it to M32.
5. **For P6a: M32's declared diff includes a moved value, not only added keys.** Planning finding 4
   says the re-record may differ "only by the declared added keys".
   - Every engine vector also carries `expected.analysis_version`, beside the top-level
     `analysis_version`. `spec/vectors/synthetic/baseline-3to1.json` has both at 16 (read
     2026-09-30).
   - So the version bump is a moved value inside the gated output.
   - ADR-035 clause 3 already allows "named moved values". §M32's allow-list has to name this one.
6. **For P5 (ADR-020): no surviving Python file imports `analysis/` directly** (read 2026-09-30).
   - The files checked are clause 1's list: the pose worker and estimator, `capture/`, `config.py`,
     the three LLM modules, `transcript_store.py`, `ask_swing.py` and the four surviving shapes.
   - The LLM's tools are the one indirect route. `scripts/ask_swing.py` (and `api/app.py`) build
     them from `mcp.runner_tools` and hand them to `conversation.py`, and `mcp/` calls `analysis/`.
   - That is the route ADR-020's addendum has to retire. ADR-032's addendum states it this way,
     rather than as "none of the survivors reaches `analysis/`".

### P5a — found (2026-09-30)

Docs only. P5 was split before it started, as its own rule requires. The phase section now carries
P5a and P5b, and the checklist has a row for each. P5a wrote four addenda:
[ADR-006](../decisions/006-mcp-server.md#addendum-2026-09-30-m315-the-server-ports-to-rust-on-rmcp-and-the-tool-surface-outlives-the-language),
[ADR-014](../decisions/014-screen-capture-shot-ingestion.md#addendum-2026-09-30-m315-the-labs-reader-ports-too-and-the-parser-is-recorded-from-python-once),
[ADR-020](../decisions/020-conversational-followups.md#addendum-2026-09-30-m315-option-c-ends-with-the-python-mcp-server-and-the-llm-drives-the-rust-one-over-stdio)
and
[ADR-022](../decisions/022-learned-artifacts-as-committed-data.md#addendum-2026-09-30-m315-fitting-is-archived-the-artifacts-stay-and-rust-evaluates-them).
Each routes to ADR-035's clauses by anchor and ends with what it does not change. The addenda total
moved 89 → 93. The README rows moved: 006 2 → 3, 014 2 → 3, 020 none → 1, 022 4 → 5. Docs-truth passes,
and every link in the four ADRs and `docs/README.md` resolves, anchors included (checked with a
GitHub-slug resolver kept in the session scratchpad). Line endings are unchanged: 006, 020, 022 and
`docs/README.md` are CRLF, and 014 is LF.

1. **The split, and why these four.** P5a took what M29 ports: 006, 014, 020 and 022. P5b takes
   the order and the M40 line: 016, 024, 033 and ADR-001's Status. Each half is at most four.
   ADR-033's self-count is "Five addenda", and it has five (read 2026-09-30), so P5b moves it to six.
2. **Only ADR-020's Status changed.** Its chosen option is the thing that ends, so a reader of its
   Status would otherwise be misled.
   - It reads "Amended by ADR-035", not "Superseded in part". That agrees with ADR-035's own Status,
     which counts 020 among the ten it *amends*.
   - README row 020's Status cell says the same. Its old addenda cell, "(the stdio round trip is for
     *external* clients; in-app calls go direct)", became false at M40 and was replaced.
   - The Status blocks of 006, 014 and 022 were left alone. M31 did the same for ADR-014 when ADR-034
     amended it. P5b should apply that test to 016, 024 and 033.
3. **ADR-006's "port 8081" was already false.** `scripts/run_mcp_server.py` speaks stdio, and its
   docstring says `settings.mcp_port` is not used. The addendum corrects that while it is amending the
   same sentence. The surface the port has to serve is also wider than the 2026-08-10 table, because
   `mcp/club.py` and `mcp/flight.py` added tools since.
4. **For P6b (§M29): Option B has a threading constraint.**
   - `anthropic.lib.tools.mcp.mcp_tool` (0.121.0) is synchronous, and it calls the async
     `ClientSession` through `anyio.from_thread.run`. So the session has to live on an event loop that
     the runner's thread can call into.
   - The same helper takes each tool's `description` and `input_schema` from the server's
     `list_tools`. So ADR-020's "two tool definitions over one implementation" ends with
     `runner_tools.py`.
5. **For P6b (§M34): whether `profiles.json` stays one file is an open question.**
   - The frozen parser reads `launch_monitor/screen/profiles.json`. The program plan's M34
     `include_str!`s the same file into `crates/screen`.
   - So adding the `Impact Position V` tile, which carried decision 1 puts in M34 in Rust only, would
     change what the frozen parser reads (planning findings 2 and 3).
   - M34 either forks the file or accepts the change and says so. ADR-014's addendum hands the question
     to M34, and ADR-035 decides neither.
6. **For P6b (§M29), or for a later decision: the archive can re-derive nothing that Rust has
   changed.**
   - `derive_joint_model.py` fits over `swings.jsonl`, whose metric columns
     `derive_pose_metrics.py` measures through Python's `analysis.measure`.
   - From M32 a metric definition changes in Rust only. So ADR-022's "**must** … re-derive" has no
     runner that agrees with the engine, even before M40 stops the archive running at all.
   - ADR-035's Deferred list does not name this. ADR-022's addendum records it, and says the first
     change that needs a re-derive also needs a decision.
7. **`launch_monitor/screen/paddle.py` is in the frozen server's closure lazily.**
   `api/pipeline.py:589` imports the screen importer, and `importer.py::build_recognizer` imports the
   Paddle recognizer inside the function. That is why ADR-014's addendum dates PaddleOCR's deletion
   to M40, although it stops being the lab's reader at M29's gate.

### P5b — found (2026-09-30)

Docs only. Three addenda, in
[ADR-016](../decisions/016-local-first-host-and-phone-upload-topology.md#addendum-2026-09-30-m315-m40-not-m29-decides-api-and-the-phone-export-is-a-verb-of-the-rust-lab),
[ADR-024](../decisions/024-per-club-shot-history.md#addendum-2026-09-30-m315-m36-ports-the-corpus-first-and-m35-admits-photo-only-shots-in-rust)
and
[ADR-033](../decisions/033-the-pose-sidecar-protocol.md#addendum-2026-09-30--the-python-writer-outlives-m29-the-lab-cli-is-the-first-caller-and-the-protocol-is-unchanged),
and a pointer in [ADR-001](../decisions/001-language-python.md)'s Status. Each addendum routes to
ADR-035's clauses by anchor and ends with what it does not change. The addenda total moved 93 → 96.
The README rows moved: 016 3 → 4, 024 3 → 4, 033 5 → 6, and row 001's Status cell gained the
pointer. ADR-033's self-count is now "Six addenda". Docs-truth passes, and all 197 links in the four
ADRs, ADR-035 and `docs/README.md` resolve, anchors included (checked with a GitHub-slug resolver in
the session scratchpad). Line endings are unchanged: 001 and 016 are LF, and 024, 033 and
`docs/README.md` are CRLF.

1. **ADR-001 got a pointer, not an addendum.** ADR-035 clause 7 offered either. Its body is history,
   and its Status is where a reader asking "why Python" starts. So ADR-035's "It amends ten ADRs"
   and its README row stay as they are. The pointer names the two exceptions, marks four of the five
   lab roles as leaving, and says why Option A's finding no longer stands on its own (the OCR runs
   the same Paddle models from Rust).
2. **P5a finding 2's Status test, applied.** ADR-016's and ADR-024's Status blocks are unchanged,
   because neither chosen option ends. ADR-033's changed for two reasons. Its self-count had to move,
   and "The first *caller* … is now M40's" had become false (finding 3).
3. **New, and no phase named it: M29's lab CLI is `crates/pose`'s first caller.** ADR-035 clause 5
   says the lab CLI "calls Python only as a worker, for MediaPipe, through `crates/pose`", and M40
   depends on M29. The ADR-033 addendum records it. The fifth addendum's live-session measurements
   stay M40's, and whether M29 measures warm-interpreter churn is M29's plan. Places that still say
   M40 is the first caller:
   - `ROADMAP.md:2999` (§M24's paused note) is P7's.
   - `docs/FLOW.md:139` ("the sidecar's is M40", and "M29 waits on M40 … its job is unchanged") and
     `:398–400`. **No phase in this plan lists `docs/FLOW.md`**, although P1 finding 5 found M29 and
     M35 references there too (`:85`, `:102–104`, `:140`, `:391`, `:404`). P7 or P8 should take it,
     or P9 should say it was left.
   - `CLAUDE.md:46` ("It has no caller either") is still true today and needs no edit.
4. **Two anchors are still P6b's and P7's to fix, and P5b added no third.**
   - ADR-024 `:314` still targets `#m35--shot-first-sessions-python-which-is-m36s-oracle`, which
     resolves today. P6b's rename of §M35 breaks it, and P6b fixes it in the same change. The new
     addendum routes to ADR-035 instead of to that heading.
   - ADR-033 `:322` links to ROADMAP `#m29-the-last-python--retiring-the-lab-and-deleting-analysis`.
     If P7 renames §M29's heading, that link breaks, and so do ROADMAP's own `:2426`, `:2727` and
     `:2865`. The new addendum names §M29 in words and links to ADR-035 clause 5.
5. **For P6b (§M35, §M29): a second by-design disagreement, beside P3 finding 6's.** Frozen
   `read_corpus` keeps excluding a photo-only manifest as `NO_FACE_ON` (`storage/corpus.py:97`). The
   frozen server's career view (`api/app.py:925` and `:947`) and Python `mcp/`'s club and career tools
   (`mcp/club.py:438`, `mcp/career.py`) read through it. So from M35 until M40, frozen Python pools
   fewer shots per club than Rust does. The ADR-024 addendum says so. ADR-035's Consequences name only
   the parser's disagreement, and this plan does not reopen them.
6. **For P6b (§M40): ADR-016's rules become the `axum` port's to keep or decline.** The addendum
   hands M40 the question of whether a Rust `api/` keeps the loopback bind, the refused non-loopback
   start and the token as a route dependency, recorded by addendum in ADR-016. It also names the
   upload route as the one path by which frozen Python writes `data/` after M29. That is ADR-035's
   Deferred item, and it is not decided here.
7. **ADR-033's two Consequences pointers, checked.** The `sidecar/` package question stays M29's, as
   ADR-035's Deferred list says. The addendum notes that such a package would now hold the LLM side
   too, and not only the pose worker. "No Python entry point is added" holds, but one of its two
   examples, `scripts/run_pose.py`, is deleted in M29 (Q10).

### P6a — found (2026-09-30)

Docs only. Every change is in the program plan,
[m31-m40-shot-first-pivot.md](m31-m40-shot-first-pivot.md):
- **A new section, "Re-planned by M31.5 (2026-09-30)"**, near the top. It gives the rule, the
  oracle, the freeze and what moved, routes to ADR-035's clauses, and says Decisions 1–17 stand.
- **ADR-035 in the header's governing decisions.**
- **The status table re-cut.** It gains an M29 row and a paragraph on the order.
- **§M32 rewritten** as Rust-only.
- **The M31.5 errata** at the foot of "What the code says". Planning findings 1–3 were re-checked
  against `api/pipeline.py::_shot_for`, `parser._score` and `parser.py:279` first.

No file or addendum was added, so no count moved. Docs-truth passes. All 721 links in the program
plan, `ROADMAP.md`, `docs/FLOW.md` and the ADRs resolve, anchors included. They were checked with a
GitHub-slug resolver kept in the session scratchpad. The program plan is LF, as it was.

1. **§M32 keeps its heading.** `ROADMAP.md:1771` links
   `#m32--wider-shot-contract-and-device-capability-detailed`, and the title is still true. The
   Rust-only statement is the section's first line instead. P7 does not need to touch that link.
2. **Five calls P6a made where the plan left the choice open.** Each is argued in §M32, and a later
   phase or M32's own plan may contest any of them:
   - **Schemas: (c)'s split, using (b)'s mechanism.**
     - `shot_data`, `swing_result` and `swing_bundle_result` become Rust-owned, and are hand-edited
       in pydantic's own spelling.
     - `crates/contracts/tests/schemas.rs` pins their property sets against what the Rust structs
       serialize.
     - The seven other roots stay generated by frozen Python.
     - Why not `schemars`: the three files carry 78 bound keywords, counted on 2026-09-30. Rust holds
       those bounds in `Validate` impls, where `schemars` cannot see them.
     - M36 revisits the choice if hand-maintenance proves the larger cost.
   - **A re-record writes the committed document with only the declared paths replaced.**
     - This is ADR-035 clause 3's "every value Python recorded survives in the file", read literally.
       Undeclared floats keep Python's bits, even where Rust agrees only within §3's tolerance.
     - It changes P1 finding 3's list of Python pins that would fail. `:420` (the stage family) does
       not fail, because the stages are written back unchanged apart from `analysis_version`.
   - **`provenance.oracle` stays `"python"`, and a new `provenance.rerecords` ledger names each
     declared change.** This answers the question ADR-032's thirteenth addendum and P4 finding 4 left
     to M32. It keeps the twelfth addendum's three kinds at three.
   - **Python keeps a "frozen view".**
     - `check` and the conformance pins compare frozen Python only on the paths no ledger declared, so
       they certify the freeze rather than the vectors.
     - `regenerate` refuses the engine and stage families, on `_audio`'s precedent.
     - `conformance.py check` leaves "Verify, every milestone" at **M32**. This is the milestone P6b's
       verify edit needs.
   - **`printed_on` and `printed_fields`.**
     - They follow the planning session's recommendation: declared ∩ present per shot, and the union
       across shots.
     - Two cases the plan did not name are settled. A direct feed prints all that its device
       declares. An unstamped screen shot (`fields_present: None`, which is every stored shot until
       M34) counts only the fields it holds a value for.
     - So a pre-M34 blank is excluded rather than named `printed_blank`. That rule never invents a
       reason, and it keeps M35 and M37 free of a dependency on M34's re-read.
3. **Three routings in P6a's edit list were corrected.** None needed the user, since each follows
   from ADR-035 clause 4 or from code:
   - **`mcp/query.py::_METRIC_FIELDS` goes to M29, not M34.**
     - It is frozen Python's MCP layer, and M34 touches only `crates/screen`.
     - The new fields join a metric list when the `rmcp` server is written. P6b's §M29 should carry
       it.
     - `tests/mcp/test_query.py` stays green meanwhile, because Python's `ShotData` does not change.
   - **`tests/test_conformance.py::_PACKAGE_DATA` does not gain `devices.json`.** The pin scrapes
     filename constants from `src/golf_coach/` only, and `devices.json` lives in `crates/contracts/`.
   - **M33 now depends on M31.5, not M32.** Its old dependency was on M32's edits to the Python
     parser, which moved to M34 in Rust. The frozen parser M33 measures is one M32 no longer
     touches. P6b's one line for §M33 can say so, and P7 mirrors the dependency.
4. **Carried decision 6, checked against the Rust mirrors.** Each mirror carries some of the stale
   comments and not others:
   - **`crates/analysis/src/shot_measure.rs`** does not carry the 0.89–1.00 range, "three shots" or
     "one per session".
     - It carries a pointer to "the Python module docstring" for the smash measurement (`:16–18`,
       `:111–113`), which M40 deletes, and "the two shots on disk".
     - §M32 moves the measurement into the Rust doc with M31's 0.76–1.06.
   - **`crates/analysis/src/engine.rs:478–479`** carries the outcome-bands sentence and "full M4".
   - **`crates/contracts/src/intent.rs`** carries "full M4" five times, on `PracticeMode`,
     `ShotShaping`, `Performance`, `Drill` and `TargetShape`.
5. **For P6b.** Each of these is found here, and none is decided:
   - **§M34's 13-shot re-read has no Rust OCR.** `ort` is M29's, so M34's Rust parser can re-read
     only boxes recorded once from frozen PaddleOCR, which is Q7's one-time recording. Writing the
     result into `data/processed/shots/` is a Rust write that the frozen lab reads, so clause 4's
     rule binds it. §M34 should also set `SCREEN_PARSER_VERSION`'s first ledger value, which §M32
     leaves as 1, and own the `profiles.json` ↔ `devices.json` pin.
   - **The MCP instructions' reason prose, for §M29.** `contracts/caveats.py` builds the "not
     capture problems … the footage is not what went wrong" sentence from Python's table (`:166`).
     The `rmcp` copy derives it from Rust's `UNSCORED_REASONS`, so `printed_blank` joins the
     not-a-capture-problem list by itself. But `misread` is a *photo* worth retaking, which
     "re-film" does not say. The first surface to emit it (M35 or M37) or the port (M29) rewords it.
   - **"Invariants every milestone keeps"** says a version bump regenerates vectors "only on the
     Windows box, where `data/` lives". From M32 the Rust re-record reads only committed vectors, so
     the reason is gone, although the rule stands until M29 builds vectors from `data/` in Rust.
   - **M37's oracle pin can require what §M32 writes:** `oracle` on every family, and a declaration
     on every `rerecords` entry.

### P6b — found (2026-09-30)

Docs only. The edits are in the program plan,
[m31-m40-shot-first-pivot.md](m31-m40-shot-first-pivot.md), plus two inbound links:
- **§M33** gained a paragraph: unchanged in substance, the harness kept outside the package, and the
  tie read into the gate's diffs.
- **§M34, §M35 and §M36 were rewritten**, §M35 as Rust after M36.
- **§M37, §M38, §M39 and §M40 were edited** where they planned Python or pointed at it.
- **A new §M29** sits after §M40, in the order the status table uses.
- **"Reuse, don't rebuild", "Invariants every milestone keeps" and "Verify, every milestone"** were
  rewritten, and `conformance.py check` has left the verify list at M32.
- **Four sentences in §M32 moved**, for finding 2 below.

No file or addendum was added, so no count moved. Docs-truth passes. 856 links in the program plan,
`ROADMAP.md`, `docs/FLOW.md`, this plan, `docs/README.md` and every ADR resolve, anchors included,
checked with a GitHub-slug resolver in the session scratchpad. The one failure is this plan's own
`:209`, a placeholder inside backticks and not a link. Line endings are unchanged: the program plan
and this plan are LF, and `ROADMAP.md` and ADR-024 are CRLF.

1. **Only §M35's heading changed.** It is now `#m35--shot-first-sessions-in-rust`.
   - `ROADMAP.md:1833` and ADR-024 `:314` link to it, and both were fixed in this change, anchor only.
   - ROADMAP's own §M35 heading, and its self-link at `:3411`, are P7's.
   - Every other milestone keeps its heading, so every other link from ROADMAP and the ADRs into
     §M31–§M40 still holds.
   - The new section's anchor is
     `#m29--the-lab-port-a-rust-lab-cli-the-rmcp-server-and-the-archive-move`, for P7 to link.
2. **P6b decided where the 13-shot re-read writes: M34's writes no `data/`, and M29's does.** P6a
   finding 5 left this open.
   - M34 re-reads over the boxes its recorder commits, because `ort` is M29's.
   - Writing `data/processed/shots/` in M34 would change how the lab reads the two label-fix shots
     before M29. ADR-035's Consequences and ADR-014's M31.5 addendum both date that change to M29. It
     would also leave each `analysis.json` echoing the old shot, and it would need the shot store
     before M36 ports it.
   - So the stored shots stay unstamped until M29's lab lookup calls `parse_is_current` and re-reads
     them.
   - §M32 had said "until M34's re-read" in three places, and routed `_shot_for`'s stale-cache check
     to M34. Those four sentences now say M29, or say where the caller lives.
   - This is a call the user or M34's plan may contest. If M34 does write `data/`, clause 4 binds the
     write, and the two ADR sentences above need an addendum.
3. **`SCREEN_PARSER_VERSION`'s entry 1 is the Rust parser as M34 ships it**, tie rule and V tile
   included. The faithful port never stamps a shot, so it gets no version of its own. This is the
   ledger value P6a finding 5 asked §M34 to set.
4. **`profiles.json`, fork or share, is still M34's to decide.** §M34 lays out both sides, and notes
   that clause 4 points at the fork.
5. **New, and no earlier phase found it: `read_corpus`'s `OUTDATED` rule meets the version split.**
   - `storage/corpus.py:252` excludes an analysis older than the installed `ANALYSIS_VERSION`, so
     that its numbers are "reported rather than pooled".
   - Rust's version is 17 from M32, and every `analysis.json` in `data/` is frozen Python's 16. So
     M36's faithful port, run over `data/` by the verbs that replace R29, pools nothing until M29
     re-analyses the lab, although M32's bump moved no number.
   - §M36 names it and leaves the remedy to M36's plan. The committed vectors are unaffected if each
     fixture's version is set relative to the installed one. Neither ADR-035 nor this plan's findings
     mentioned it.
6. **M37's `get_strike_profile` was new Python** (`mcp/club.py`). It is now an `rmcp` tool: M37 adds
   it if M29 has landed, and otherwise M29 carries it.
   - The docs-truth pin on the tool count, `test_the_docs_state_the_real_mcp_tool_count`, reads
     Python's `TOOL_DESCRIPTIONS`.
   - §M29 re-points it at the Rust list in the change that moves the descriptions.
7. **Three readings P6b made where the plan left a gap.** Each can be contested.
   - **M33's harness** lives in `spikes/`, outside `src/golf_coach/`. `TextRecognizer` is a
     `Protocol`, so nothing in the package changes. That is how clause 4's "using the lab is not
     changing it" was applied.
   - **Extras.** R44 signed off deleting `hardware` and dropping `ultralytics` without a date, and
     §M29 now does both. `ultralytics` goes with `detection/`, its only user, and `hardware` goes with
     `research`. `api`, `ocr` and `audio` stay M40's.
   - **M37's corpus vectors** can be recorded from the shots the committed corpus vectors already
     carry as `input.shot`. So M37 need not wait for M29's builder that reads `data/`.
8. **A third by-design disagreement, beside P3 finding 6's parser and P5b finding 5's photo-only
   corpus.** Frozen Python never learns the `DRILL` rule, so it pools a Rust-written `DRILL` swing
   that has a face-on clip.
   - §M35 and the invariants say so.
   - Whether such a swing is ever written depends on M35's policy decision (ADR-034's Consequences).
   - ADR-035's Consequences are not reopened.
9. **For P7**, which mirrors this in `ROADMAP.md`:
   - M33 depends on M31.5.
   - §M34's exit is now carried decision 4's.
   - §M35's row (`:54`), its "Python first" sentence (`:1821`) and its exit (`:1829`), and M36's
     "with Python as the oracle" (`:1841`, P1 finding 5), all change. Re-check the line numbers first.
   - §M29's section moves from "blocked on M40" to the lab port. ADR-033 `:322` links to its current
     heading, so a rename breaks that link (P5b finding 4).
   - M38 P4 waits on M29.
   - **`docs/FLOW.md` is still unlisted by any phase** (P5b finding 3). Its M35 node reads "in
     Python", and it says M29 waits on M40.

### P7 — found (2026-09-30)

Docs only, and a retry. The first P7 attempt was killed by an API rate limit while editing
`ROADMAP.md`, and left no findings. The edits are in `ROADMAP.md`, plus one inbound anchor in
[ADR-033](../decisions/033-the-pose-sidecar-protocol.md) and this plan's P8. No file or addendum was
added, so no count moved. Docs-truth passes. All links in every tracked markdown file resolve,
anchors included, checked with the GitHub-slug resolver in the session scratchpad. Three hits are
not links: this plan's `:209` (as P6b found), and `m31-shot-first-adr.md` `:187` and `:456`, which
are snippets inside backticks that wrap a line. Line endings are unchanged: `ROADMAP.md` and ADR-033
are CRLF, and this plan is LF.

1. **The killed attempt had done most of P7, and the retry checked it rather than redoing it.**
   - Already in place: the M31.5 status row; the table re-cut, with M29's row moved into the group
     and every M32–M40 row and dependency matching the program plan's checklist; the `NEXT ACTION`
     rewrite; the group intro's re-plan paragraph; and §M31.5 to §M40 in the group's form. Each was
     read against the program plan's sections and ADR-035. None was half-written, and none was
     duplicated.
   - Missing: §M29 itself. The table, `NEXT ACTION` and the group intro already linked
     `#m29-the-lab-port--a-rust-lab-cli-the-rmcp-server-and-the-archive-move`, a heading that did
     not exist, so three links dangled. The retry's first link check found that, together with §M4
     full's stale `#m35-shot-first-sessions-in-python`.
2. **§M29 was rewritten, not bannered.** Its heading had to change, because the anchor was already
   linked. Its old body contradicted the new scope in almost every paragraph: `api/` retiring into
   the shell, the fitting question, and an exit of "the sidecar and nothing else". So the section is
   now in the group's form, with a banner naming the old title. It ends with **"What the old section
   said, and where each part went"**, one line per old claim. The old body is in git history at
   `5b58b6a`.
   - That foot list is what lets §M22's three inbound sentences keep their wording (`:2524`,
     `:2825`, `:2963`). They are a closed milestone's record, so only their anchors changed. The
     third says the OCR, `clubs/lookup.py` and the overlay are "still open there", so it gained one
     sentence saying M31.5 settled all three.
   - ADR-033 §8's pointer (`:325`) changed its anchor only. Its M31.5 addendum already says M40, not
     M29, deletes the Python writer.
3. **Four banners outside the group still carried M31's reading, and P7 corrected each.** None of
   them was on the plan's list:
   - **The app group's banner** said "M29 is blocked on M40". It also never superseded the group's
     ADR-030 summary, "Python also keeps OCR, LLM coaching and the whole lab". It gained a paragraph
     routing to ADR-035 and §M29.
   - **§M24's banner** said the pose pool's first caller, and `api/pipeline.py`'s orchestration,
     both move to M40 with M24. The first caller is now M29's lab CLI, and the live-session
     measurements stay with M24 under M40 (ADR-033's M31.5 addendum). The orchestration is replaced
     by M29's lab CLI.
   - **§M3's banner** routed the `Impact Position V` label fix to M32. It is M34's now.
   - **§M4 full's banner** had the dead §M35 link (finding 1).
4. **Left as history, on purpose.** M23's status row and §M23's status both say M24 gives the pool
   its caller. They are a closed milestone's record, and §M24's banner now corrects them. The app
   group's ask, its Order paragraph and its ADR-030 bullets stay too, because M31's banner already
   marks them historical and the new paragraph supersedes the one bullet that mattered.
5. **`docs/FLOW.md` is P8's now** (P5b finding 3, P6b finding 9). P7's scope is `ROADMAP.md` alone,
   and P8 already owns the docs a fresh session reads first. FLOW is CRLF, and its pivot graph draws
   M31's order in its edges as well as its labels, so the fix is more than a word swap. P8's section
   lists the passages, with line numbers.
6. **For P8: `CLAUDE.md`'s "first caller" sentences.** Its Rust paragraph says `crates/pose` "has no
   caller either", which is still true today (P5b finding 3). Its "What is the app written in" row
   says the sidecar is "laptop-only, for M40's desktop client", which reads past M29, the crate's
   first caller under ADR-035. That row is already on P8's list.

### P8 — found (2026-09-30)

Docs only. The edits are in `CLAUDE.md`, `docs/CONFORMANCE.md`, `docs/FLOW.md` and one row of
`docs/README.md`. No file or addendum was added, so no count moved. Docs-truth passes. Every link in
those four files, this plan and `ROADMAP.md` resolves, anchors included, checked with a GitHub-slug
resolver in the session scratchpad. The one failure is this plan's `:209` placeholder again (P6b).
Line endings are unchanged: all four files are CRLF, and this plan is LF. The edits were made by a
script that replaces exact strings and writes CRLF back, so no line was converted.

1. **`CLAUDE.md`: the six listed items, plus three small ones beside them.**
   - **Listed.** The `check` line now says it certifies the vectors until M32's first Rust re-record,
     then `cargo test` does, and `check` retires in M40. The three invariants:
     - "Python keeps only what does not translate" is now "Python stays only where it is required".
       It names the two exceptions, says new work is Rust, and says the lab is frozen M32 → M40. It
       keeps the old rule's surviving sentence: the vectors are the oracle.
     - The stdlib invariant says the Rust half is the one that matters. It now names `ort` from
       §M29 beside `rustfft` as a numeric library *outside* the scoring path, so it stays true once
       M29 lands (P3 finding 7). It no longer calls `rustfft` "the one sanctioned exception".
     - The `ANALYSIS_VERSION` invariant says Rust records from M32, diff-gated, and that frozen
       Python's version stays behind on purpose. It states no version number.
     - The rows: "What is the app written in", "How is a port checked" (now "against the core"),
       and "Why did a module leave Python". That last row is now "Why does a module leave Python,
       and when?". It carries ADR-035's path in backticks, the one place `CLAUDE.md` names the file,
       so `test_claude_md_routes_only_to_files_that_exist` checks it.
   - **Beside the list.** All three come from P7 finding 6's "first caller" sentences:
     - The Rust paragraph's "its first caller is now the phone app (M38)" now names both, the phone
       and §M29's lab CLI. ADR-035 clause 5 has the lab CLI run the engine through `crates/core`.
       M29 and M38 run in parallel, so the sentence does not say which comes first.
     - `crates/pose`'s "MediaPipe is the one thing staying in Python" is now "one of the two".
     - "It has no caller either" is unchanged, because it is still true (P5b finding 3).
   - **Left alone, on purpose.**
     - The opening paragraph ("Python 3.11 … that is what *runs*") is still true today.
     - "Models are fitted offline and ship as data" says fitting lives in `scripts/`. That holds
       until §M29 archives it, so the change that archives it should edit that invariant (ADR-022's
       M31.5 addendum).
     - "A change to the Python engine now silently puts two implementations out of step" still
       describes the mechanism. The new freeze invariant is what says not to make that change.
2. **`docs/CONFORMANCE.md`: §4's commands block is unchanged.**
   - M32's own Docs list rewrites §4 with its `rerecord` verb (program plan §M32, "Docs"). So P8
     added one paragraph above the block, and nothing more. It says the commands hold until M32,
     what moves at the first Rust re-record, and that what `check` and `regenerate` do afterwards is
     §M32's to build. It links §M32's "What changes in Python, and why".
   - **"§5's oracle paragraph" was read as §5's intro plus every row ADR-035 moved.** §5 has no
     paragraph about the oracle as such. M31 changed it by adding an intro sentence and editing the
     rows, so P8 followed that form:
     - A new intro paragraph routes to ADR-035 §1 and §5 and to P1's inventory.
     - **Tier 1, M22**: both implementations stand until M40, not §M29, and ADR-035 §2 deletes the
       two render parts.
     - **The rest of the swing loop**: `capture/` stays Python (R18). The row had said it ports.
       `storage/` goes to M36 and §M29, except `transcript_store.py`.
     - **`audio/ffmpeg.py`** ports in §M29, not "next". **Shots and clubs** port in M36.
     - **The screen reader** is recorded once from frozen Python. The recognizer ports through
       `ort`.
     - **Tier 3** is clause 1's list, with `capture/` and `clubs/lookup.py` added.
     - **Tier 4** was rewritten as §M29's port. Its "Settled" and "Open" lists were each superseded,
       and the row says which ones.
     - **`detection/`**: §M29 deletes it.
   - **Three passages outside §4 and §5 were corrected too.** P1 finding 5 had listed each of them
     as an M29 reference (`:8`, `:305`, `:544`). They are the banner's "until §M29 deletes it", §2's
     stages "regenerable … into §M29", and §5's foot, "§M29 retires [`api/pipeline.py`] into the
     Flutter shell".
   - **The title, "the Python core as a specification", is kept.** It is the document's name, and
     the milestone that makes Rust own the specification can retitle it.
3. **`docs/FLOW.md`: every passage P8 listed, plus four the list did not name.**
   - **Listed.** The pivot graph is redrawn: the M35 node says Rust, and the edges run M32 → M36 →
     M35 → M37. M29 sits inside the group as the lab port. It is fed by M34 and M36, and it feeds
     M40 and, dotted, M38's P4. The subgraph title names ADR-035 too. Also redrawn:
     - the critical path, now M31 → M31.5 → M32 → M36 → M35 → M37 → M38;
     - "What the pivot paused", whose caller sentences moved into a new "What the re-plan moved"
       paragraph;
     - §4's lab and laptop-client paragraphs;
     - the lower graph's first-caller edge, which now runs LABCLI → SIDECAR. DESK → SIDECAR is kept
       and labelled "live sessions" (ADR-033's M31.5 addendum).
   - **Not listed:**
     - **An M31.5 node** (🟡, class `wip`) between M31 and M32, because ROADMAP's table has the row.
       M33 now hangs off M31.5 rather than M32, matching ROADMAP's dependency column.
     - **M3's "open OCR items" edge** now points at M34, because P7 finding 3 moved the label fix
       there.
     - **The lower graph's lab** is split. A frozen Python subgraph sits beside a Rust §M29 one
       (lab CLI, export-import verb, `rmcp`), with `data/` between them.
       `import_phone_export.py — M38` is gone from both the node and the "phone listens on
       nothing" paragraph, because ADR-016's M31.5 addendum says it is never written.
     - **The banner and §1's intro** name ADR-035.
   - **M32's marker stays ⬜**, while ROADMAP says ⏸ waits on M31.5. They agree once P9 closes M31.5.
4. **For P9: `docs/FLOW.md`'s M31.5 node is 🟡 and in the `wip` class.** It flips to ✅ and moves to
   `done` in the same change as ROADMAP's row. P9's steps now say so. The program plan and ROADMAP
   were already on P9's list. FLOW was not, because no phase had listed FLOW before P7.
5. **`docs/README.md`: only the CONFORMANCE row changed.** "Which stay lab" became "which stay
   Python", and since ADR-035 that means only MediaPipe pose and the LLM. The row also gained one
   sentence saying the oracle is Rust from M32. No doc's tier changed. FLOW's row ("Where is this
   going? …") was already generic. `CLAUDE.md` has no map row: it is the paragraph above the
   tiers, which still reads true.
6. **New, and not P8's to fix: ADR-032's M31 addendum still orders the core's callers.** It says
   "The core gains callers on the phone (M38) before it gains one on the laptop" (`:1313`).
   - Under ADR-035, §M29's lab CLI is a laptop caller of `crates/core`. It runs beside M38, so that
     order is no longer guaranteed.
   - ADR-032's M31.5 addendum does not mention it, and neither does any other ADR.
   - `CLAUDE.md` and FLOW now name both callers without an order.
   - The sentence needs an addendum, or a line in the M29 plan, whichever touches ADR-032 first.
     M31.5 writes no more addenda after P5b.

### P9 — found (2026-09-30)

Docs only, and the close. The edits are this plan's checklist and header, the program plan's
status table and header, `ROADMAP.md` (the M31.5 and M32 rows, §M31.5, §M32 and §M33's status
lines, and `NEXT ACTION`), `docs/FLOW.md`'s M31.5 node and class lines, and a new top entry in
`WORKLOG.md`. Outside the repo, the two memory notes the steps named and their lines in the memory
index. No file or addendum was added, so no count moved (78 markdown documents, 96 addenda).
Docs-truth passes. Line endings are what they were: `ROADMAP.md`, `docs/FLOW.md` and `WORKLOG.md`
are CRLF, and both plans are LF.

1. **All six verify commands are green, at the numbers M31 closed on.** `pytest` 1,991 passed;
   `ruff` clean; `mypy` clean on 118 files; `conformance.py check` 21/21 at v16, with the audio,
   stage and format families deferred to `cargo test` as before; `cargo test` 30 result lines,
   582 passed and none failed, run into a log rather than a pipe and not beside pytest; `cargo
   clippy --all-targets` with no warning after every crate's `lib.rs` was touched to force a
   re-lint; `cargo fmt --check` clean. `git status` shows docs only, plus two `.claude/skills/`
   changes (the `plan` → `plan-phases` rename and an untracked `orchestrate/`) that predate this
   milestone. Nothing is committed: M31's work and M31.5's are both still in the working tree.
2. **Closing M31.5 made three status sentences false that P9's steps did not name**, and P9 fixed
   each, because leaving them would have pointed a fresh session at a closed milestone:
   - **M32's "⏸ Waits on M31.5"**, in ROADMAP's row and §M32 and in the program plan's row, is now
     "⬜ Not started, and next". That is what P8 finding 3 meant by "they agree once P9 closes
     M31.5": FLOW's M32 node was already ⬜.
   - **§M33's "🔒 Blocked on M31.5"** is now "Blocked on a Mac". Its table row ("🔒 Blocked",
     depending on "M31.5, and a Mac") still reads true and is unchanged.
   - **`NEXT ACTION`'s heading and its "Once M31.5 closes" sentence** now say M32 is next.
3. **A hazard for any later session: Git Bash's `sed -i` rewrites a CRLF file as LF**, every line of
   it, even for a one-line substitution. It did so to `WORKLOG.md` here. Converting LF back to CRLF
   restored the file, and its diff against `HEAD` is insertions only. M31 P7 finding 6's advice
   ("check with `file` before a `sed -i`") is not enough on this box: edit CRLF files by a byte-level
   replacement (as P8 did), or by the Edit tool, and run `file` afterwards.
4. **Carried, each with an owner, and listed in the WORKLOG entry so that M32 sees them:** ADR-032's
   M31 addendum on the order of the core's callers (P8 finding 6, whichever change touches ADR-032
   first); `read_corpus`'s `OUTDATED` rule across the version split (P6b finding 5, M36); and where
   M34's re-read writes, and `profiles.json`, fork or share (P6b findings 2 and 4, M34).
