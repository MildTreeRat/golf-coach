# Plan: M31.5 — Rust everywhere, Python only where required (the re-plan, docs only)

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-09-30, in the session that set out to plan M32 and was redirected by the user
mid-interview. **Milestone**: M31.5, a new milestone between M31 and M32, numbered the way `M1.5`
already is. It has no `ROADMAP.md` section yet, because writing one is P7's job.
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
| **P0** | This plan document, and the pointers that stop a session starting M32 | ⬜ |
| **P1** | Inventory every Python area and script, and propose a disposition (read-only) | ⬜ |
| **P2** | The user signs off the inventory (interactive) | ⬜ |
| **P3** | Write ADR-035 | ⬜ |
| **P4** | Addenda, "who is the oracle and what retires": ADR-030, 032, 034 | ⬜ |
| **P5** | Addenda P1 named beyond those three | ⬜ |
| **P6a** | Program plan: the re-plan section, the status table, and M32 re-detailed Rust-only | ⬜ |
| **P6b** | Program plan: M33–M40, the lab-port milestone, invariants and verify | ⬜ |
| **P7** | `ROADMAP.md`: the M31.5 row and section, and the milestones the re-plan moves | ⬜ |
| **P8** | `CLAUDE.md` and `docs/CONFORMANCE.md` | ⬜ |
| **P9** | Close: verify, WORKLOG, checklists, memory | ⬜ |

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

### P5 — Addenda P1 named beyond those three

**Goal**: an addendum to each further ADR P1 finding 4 named, in the same form as P4. **If the list
is longer than four, split this phase (P5a, P5b) before starting**, and update the checklist.
Likely: ADR-014 (the lab's OCR engine goes Rust via `ort`), ADR-022 (fitting is archived; the JSON
stays), ADR-006 (the MCP server ports to `rmcp`), ADR-008 (the import rule's Python half freezes),
ADR-016 (FastAPI ports or drops in M40), ADR-033 (only if something in it changes; its self-count is
**"Five addenda"**).

**Files**: those ADRs; `docs/README.md` rows and total.

**Done when**: docs-truth passes; every link resolves. **Stop.**

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

### P8 — `CLAUDE.md` and `docs/CONFORMANCE.md`

**Goal**: the two docs a fresh session reads first stop telling it that Python is the oracle.

**`CLAUDE.md`**: the invariants "Python keeps only what does not translate" (now: only MediaPipe and
AI), "The analysis core is stdlib + contracts only" (the Rust half is the one that matters), and
"An `ANALYSIS_VERSION` bump regenerates `spec/vectors/`" (who regenerates, from M32); the Commands
block's `conformance.py check` line (what it certifies, and until when); the rows "What is the app
written in", "Why did a module leave Python" and "How is a port checked". Keep it a router: point
at ADR-035, copy nothing. **`docs/CONFORMANCE.md`**: §4's commands and §5's oracle paragraph (M31
edited §5; read what it says now). Also `docs/README.md`'s map rows for any doc whose trust tier or
summary the re-plan changed.

**Done when**: docs-truth passes. **Stop.**

### P9 — Close: verify, WORKLOG, checklists, memory

**Goal**: M31.5 closes with the suite as green as M31 left it.

**Steps**:
- Run the whole verify suite below.
- A WORKLOG entry at the top, in M31's form: what was decided, what P1 found, what M32 should read
  first.
- This plan's checklist, and the program plan's M31.5 row → ✅; ROADMAP's M31.5 row → ✅.
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
