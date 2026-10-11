# Plan: M35 — Shot-first sessions in Rust

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-10-09. **Milestone**: [ROADMAP §M35](../../ROADMAP.md#m35-shot-first-sessions-in-rust).
**The design is the program plan's
[§M35](m31-m40-shot-first-pivot.md#m35--shot-first-sessions-in-rust)**, Rust-only since M31.5, with
M36's handoff ("What M35, M29 and M38 should know" in the top WORKLOG entry of 2026-10-08) and M34's
`misread` rule ([m34-screen-reader.md](m34-screen-reader.md), P8 finding 3). This file is its phase
list. It does not restate that section: each phase names what it builds and what to read. Where this
file and §M35 disagree, this file's interview decisions, findings and calls say why, and this file
wins.

**Governing decisions**: [ADR-034](../decisions/034-shot-first-phone-first.md) §1 (the shot is the
unit; mechanics is a separate panel), §2 (printed / not printed / printed blank), §3 (a drill is not
tracked) and its Consequences (the two open questions this milestone answers);
[ADR-009](../decisions/009-swing-scoring-model.md) (`PracticeGoal`, the per-mode seam);
[ADR-024](../decisions/024-per-club-shot-history.md) (the corpus) and ADR-028 (mishits);
[ADR-010](../decisions/010-benchmark-ranges.md) §2 (excluded and named, never guessed);
[ADR-035](../decisions/035-rust-everywhere-python-where-required.md) clauses 3 (Rust records) and 4
(the frozen lab must keep reading what Rust writes); ADR-008 (as cargo edges).

**Level: L3.** A new contract (`ShotResult`) and a new on-disk artifact (`shot_analysis.json`), a
shape change to the manifest, the session file and `CorpusSwing`, a semantic change to `read_corpus`
under a new `CAREER_VERSION`, a new vector family, and the first Rust verbs that write into `data/`.
Every phase reads its own list below rather than the repo.

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot — the findings are what the next phase
   is planned against;
3. stop, and tell the user to `/clear` and run `/next-phase`.

**Nothing is committed until P13** (the user's call, 2026-10-09). Every phase leaves its work in the
working tree, and P13 makes one commit onto `main`.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document, and the pointers to it | ✅ Done *(2026-10-09)* — approved by the user |
| **P1** | `crates/contracts` I: the tracked rule, `ShotResult`, its schema; the `data/` snapshot | ✅ Done *(2026-10-09)* |
| **P2** | `crates/analysis`: every mode reaches the engine, and `analyze_shot` | ✅ Done *(2026-10-09)* |
| **P3** | `crates/screen`: a printed tile with no value, as `printed_blank` or `misread` | ✅ Done *(2026-10-09)* |
| **P4** | `crates/core` I: the `run_shot` seam, `golf-core run-shot` on stdin, the shot family's gate | ✅ Done *(2026-10-09)* |
| **P5** | The shot family's cases from the corpus and screen families | ✅ Done *(2026-10-09)* |
| **P6** | `golf-core rerecord` gains the shot family; the unchanged-swing pin | ✅ Done *(2026-10-09)* |
| **P7** | `crates/storage` I: intent on the manifest, the session cursor, `set_intent` | ✅ Done *(2026-10-09)* |
| **P8** | `crates/storage` II: `CorpusSwing`'s new shape, the `shot_analysis.json` reader and writer, `loft_for_club` | ✅ Done *(2026-10-09)* |
| **P9** | The hand-worked storage cases for the new corpus rules, skipped by name until P10 | ✅ Done *(2026-10-10)* — resumed |
| **P10** | The change: `read_corpus` admits photo entries and leaves drills out; `career-v2` | ✅ Done *(2026-10-10)* |
| **P11** | `crates/core` II: `golf-core run-shot --session` and `golf-core set-intent` | ✅ Done *(2026-10-10)* |
| **P12** | Docs: CONFORMANCE, ARCHITECTURE, `data/README.md`, the ADR addenda, `CLAUDE.md` | ✅ Done *(2026-10-10)* |
| **P13** | Close: verify, WORKLOG, ROADMAP, memory, and the one commit | ✅ Done *(2026-10-10)* — resumed |

---

## Rules every phase keeps

- **Every phase ends green on everything it can reach.** If a phase cannot end green, stop and
  record why as a finding; do not carry a red suite into the next session. A vector a phase cannot
  pass yet is skipped *by name* in the gating test, with the phase that will pass it, never by a
  blanket ignore.
- **Frozen Python is untouched outside `scripts/conformance.py`, `scripts/conformance_vectors.py`
  and `tests/test_conformance.py`** (ADR-035 clause 4), and those change only for what this plan
  names: the shot family's deferral in `check`, and `RUST_OWNED_SCHEMAS`. `git diff -- src/` is empty
  at every phase.
- **Nothing under `data/` is written by any phase**, though P11 builds verbs that can write there.
  Every write test runs on a `tempfile` copy. P1 re-takes M36's snapshot (method in finding 11) and
  P10, P11 and P13 compare against it. `data/processed/` is gitignored, so `git status` cannot
  prove this; the digest does.
- **Only these write `spec/vectors/`**: P4 and P5 write the new `spec/vectors/shot/` cases (by hand,
  or by a throwaway script in the scratchpad that the phase deletes); P9 writes new
  `spec/vectors/storage/hand/` cases the same way; P10's `golf-core rerecord` under
  `spec/declarations/career-v2.json`. Any other change under `spec/vectors/` is a finding and a stop.
  **The engine, stage, screen, audio and format families do not change in this milestone at all**:
  `git diff -- spec/vectors/{synthetic,corpus,stages,screen,audio,format}` is empty at every phase.
- **New code is Rust.** The Python changes are conformance tooling only.
- **ADR-008 as cargo edges.** `analysis` depends on `contracts` and `pyfmt` alone and never on
  `screen` or `storage`; `storage` never on `analysis`; `screen` never on `analysis`. `crates/core` is
  the one crate that holds them together, so `run_shot`, the verbs and the family gates live there.
- **No new dependency** without a finding first.
- **House style** (`CLAUDE.md` §House style, `docs/CODE_STANDARDS.md`): comments say *why* and carry
  the measurement or the rejected alternative. A ported module's doc names its Python twin, as
  `crates/storage/src/corpus.rs` does. A module with no twin says so.
- **Do not write counts into prose** that a test or a registry owns. Where this plan quotes a count,
  it is a snapshot for the phase to re-measure.
- **The doc-count pin reads `git ls-files`**, so this file is invisible to it until P13 stages it. Do
  not touch `docs/README.md`'s document count before P13. This file avoids the
  `**Status: … N/M phases` spelling on purpose (`tests/test_docs_truth.py::_PHASE_STATUS`).
- **Line endings**: the plain `.json` vectors Python wrote are CRLF, and `rerecord`'s writer keeps
  each file's own. Git Bash heredocs mangle `—` and `°`; use the Edit tool or Windows Python for
  in-place edits.

## Verify — the commands, and when each runs

```bash
cargo test                                          # every phase that touches crates/ or spec/
cargo clippy --all-targets && cargo fmt --check     # every phase that touches crates/
.venv/Scripts/python.exe -m pytest                  # P1, P6, P7, P12, P13 (and any phase touching Python, schemas or docs)
.venv/Scripts/python.exe -m ruff check src tests scripts   # P1, P6, P7, P13
.venv/Scripts/python.exe -m mypy src                # P13 (M35 changes nothing under src/)
.venv/Scripts/python.exe scripts/conformance.py check      # P1, P6, P7, P13
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py # every phase that edits a doc or moves a path
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/v17.json --dry-run        # P2, P6, P13: the swing output is unchanged
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/career-v2.json --dry-run  # P10, P13
find data/processed/sessions data/processed/golfers data/processed/shots -type f | LC_ALL=C sort | xargs -d '\n' sha256sum | sha256sum   # P1, P10, P11, P13
```

---

## Decisions taken in the interview (2026-10-09)

1. **Every mode gets the mechanics analysis, and the mode decides which stats a shot feeds.**
   - *Why it panicked* (asked by the user): ADR-009 named four single-swing scoring policies and only
     `FUNDAMENTALS` (mechanics only) was built. Python raises `NotImplementedError` for the other
     three, and the Rust port copied that as a panic. ADR-034 §5 then said shots are graded over many
     shots per club, so those three were never going to be built.
   - **Per swing**: every mode scores mechanics exactly as `FUNDAMENTALS` does. `policy_for` stops
     refusing, and `SwingResult.intent` records the real mode.
   - **Per mode, at the aggregate level**:

     | mode | analysed on its own | club and player stats, the mechanics baseline | shape grade (M37) |
     |---|---|---|---|
     | `DRILL` | yes | **no** | no |
     | `SHOT_SHAPING` | yes | yes | yes, for its declared `target_shape` |
     | `FUNDAMENTALS`, `PERFORMANCE` | yes | yes | no |

   - The user: "We want to analyze fundamentals as well, we just don't wish to add to the 'did the
     shot shape correctly when they tried it?'" Confirmed in the next round as the table above.
2. **A shot's intent lives on its manifest, stamped from a session cursor.** `session.json` gains an
   `intent` cursor beside `player_id` and `club`. A new swing takes it at creation, write-once as
   `club` is, and `set_intent` overrides one shot. `CorpusSwing` carries the intent so M37 can see
   each tracked shot's `target_shape`. The named hazard: frozen Python reads a manifest or session
   file holding `intent` and **drops the key on any rewrite** (finding 3).
3. **A located tile with nothing under its label is `printed_blank`.** On every stored bay photo the
   empty cell is a `---` the OCR does not see (Spin, Spin Axis, Impact Position V), so telling the
   golfer to retake would be wrong on most shots.
4. **The same screen photo saved twice, once attached to a swing with video and once on its own:
   the copy with video counts.** It is one ball flight. The photo-only copy is excluded as a
   `duplicate` and named.
5. **One commit at the close** (P13), straight onto `main`, as M32, M34 and M36 landed.
6. **`golf-core run-shot` also writes into a session.** Beside the stdin-to-stdout seam, a mode reads
   a photo-only swing directory and writes its `shot_analysis.json`. M35 is the first Rust writer
   into `data/`, while the frozen server still writes there. This milestone never runs it on `data/`.
7. **`golf-core set-intent`** sets one shot's intent, or a session's default for shots created after
   it, so a `DRILL` shot can exist on disk before M29 and M38.

## What the planning read found (2026-10-09)

Measured on this box. Re-measure before leaning on a number.

1. **Where the panic is.** `crates/analysis/src/scoring.rs::policy_for` panics for every mode but
   `Fundamentals` (`:98–104`), and `engine::analyze_swing` calls it unconditionally with the intent's
   mode (`engine.rs:589`). Frozen Python's pipeline always builds `PracticeGoal(club=options.club)`
   (`api/pipeline.py:1251`), so every `analysis.json` on disk says `fundamentals`.
2. **Nothing records an intent today.** `SwingManifest` (`crates/storage/src/manifest.rs`) has
   `player_id`, `club` and `mishit`, no intent. The session cursors are
   `src/golf_coach/storage/session_meta.py` (`SessionMeta`: `player_id`, `club`, `updated_at`), which
   **has no Rust twin**. Its schema is `spec/schemas/session_meta.schema.json`, Python-owned, and
   `crates/contracts/tests/python_schemas.rs:207` pins its `ClubId` enum.
3. **Frozen Python drops an unknown key on rewrite.** `SwingManifest` and `SessionMeta` are pydantic
   models with the default `extra="ignore"`. They read a Rust-written `intent` without complaint,
   and `save_manifest` (`storage/manifest.py:166`, `model_dump_json`) and `session_meta`'s cursor
   update (`:99`, `model_dump`) write it away. Any frozen-server write to a swing (another role
   uploaded into it, `set_player`, `set_club`, `set_mishit`, `attribute_unlabeled`) erases a
   recorded drill, and the shot silently becomes tracked. Named, not fixed: M29 replaces the server.
4. **`read_corpus` is face-on-keyed** (`crates/storage/src/corpus.rs`): a manifest with no face-on
   role is `NoFaceOn` (`:141–144`) before anything else is read, survivors group on the clip hash,
   and measurements come only from `analysis.json` (`:287–329`). `CorpusSwing.face_on_sha256` is a
   required `String` (`crates/contracts/src/career.rs`). `artifact_key` already keys launch-monitor
   and model rows on `shot:{sha}`.
5. **The recorded corpus case the admission moves.** `spec/vectors/storage/corpus/no-face-on.json`'s
   swing `2026-08-10/1` holds a down-the-line clip **and a shot photo**, so under the new rule it is a
   photo entry, not `NoFaceOn`. Its swing `2` holds no photo and stays `NoFaceOn`. Its adopted copy,
   `spec/vectors/career/synthetic/storage-no-face-on.json`, moves with it. P10's dry run is what
   lists every move; this is the one the planning read found.
6. **Blank against misread needs the profile.** `crates/screen/src/parser.rs::read_cell` (`:686–`)
   writes `provenance.raw_fields` keyed by the tile's **label**, `""` for an empty cell, and adds no
   value for a `profile.blank_markers` match. A tile the tie rule withheld is in `fields_present`
   with no `raw_fields` key (M34 P8 finding 3). Telling them apart maps a `ShotData` field to its
   label and reads the blank markers, which only `crates/screen`'s profile knows, so the rule lives
   in `screen` and reaches `analysis` through the shell.
7. **The capability cut moves no HD Golf number.** `contracts::capability::printed_on`
   (`crates/contracts/src/capability.rs:163`) gives declared ∩ `fields_present`, or declared ∩ the
   fields holding a value on an unstamped shot. HD Golf's `shown_only` fields are `club_head_speed`
   and `smash_factor` (`crates/contracts/devices.json`). No `SHOT_MEASUREMENTS` row reads either,
   and `flight_infer` reads `shot_type` (declared `analysed`) for the spin-axis cross-check. So a
   view of the shot with every non-analysed field blanked measures and flies exactly as the shot
   does today.
8. **The photo-to-shot join is the shot store.** `api/pipeline.py::_shot_for` (`:557`) looks the
   shot up in `ShotStore` by the manifest's photo sha256 before any OCR. There is no Rust OCR until
   M29's `ort`, so a Rust verb can analyse only a photo frozen Python already parsed. Every stored
   shot is unstamped (`parser_version` 0, `fields_present` `None`), so `printed_on` reads it
   conservatively and **no blank is ever named on a stored shot** until M29 re-reads them.
9. **The handedness route already exists for a photo entry.** `api/pipeline.py::_handedness_for`
   (`:634`) goes manifest `player_id` → `GolferStore` → `Golfer.handedness`. A photo-only entry is a
   swing directory with a manifest, so it has the same route. ADR-034's "a photo with no swing" is a
   shot in `data/processed/shots/` that no session holds, which the corpus never reads and M29's
   import verb files into a session (`--player --club --into-session`).
10. **The loft join has no Rust twin.** `src/golf_coach/storage/flight_inputs.py::loft_for_club`
    (`:159`) with `LoftGap` (`:58`) and `_resolve_loft` (`:195`), called by
    `api/pipeline.py::_loft_for` (`:667`). The bag and the catalogue it reads are ported
    (`crates/storage/src/{bag_store,catalogue}.rs`).
11. **`data/` today**: 192 files under `data/processed/{sessions,golfers,shots}/`, digest
    `bd384cc4…`, the same as M36 P1's, by the command in "Verify". **No photo-only directory exists
    on disk**, so over `data/` frozen Python and Rust still pool the same swings after M35.
12. **`rerecord`'s families** are constants in `crates/core/src/rerecord.rs` (`:767–820`): the
    engine halves and the stage family under `ANALYSIS_VERSION`, the screen family under
    `SCREEN_PARSER_VERSION`, storage and career under `CAREER_VERSION`. A declared path cannot cross
    a store's file key (`…/manifest.json`), refused by name (CONFORMANCE §4).
13. **`flight_unscored`'s names** are `flight_carry_yds` and `flight_landing_offline_yds`
    (`crates/analysis/src/flight_measure.rs:308`, `:321`), appended after the checkpoints in
    `analyze_swing` (`engine.rs:582–584`).
14. **Docs-truth pins each ADR's addendum count** in `docs/README.md`
    (`test_the_map_counts_the_addenda_correctly`, `test_each_adr_row_states_that_adrs_own_addendum_count`).

## Calls this plan makes where §M35 left it open

Each is contestable before the phase that builds it; after that, a finding is the way to change it.

1. **`ShotResult`** (`crates/contracts/src/shot_result.rs`), written as `shot_analysis.json`. Fields,
   in this order:
   - `swing_id`, `session_id`: the entry it describes.
   - `analysis_version`: `ANALYSIS_VERSION`, set by `analyze_shot`. The corpus applies
     `COMPARABLE_FROM` to it exactly as to a swing.
   - `photo_sha256: Option<String>`: the photo this was computed from. It is the photo entry's
     staleness check, so Rust never writes Python's `analysis.state.json`.
   - `intent: PracticeGoal`: the one it was analysed under.
   - `shot: ShotData`, as stored, `shown_only` fields included, because the phone displays them.
   - `measurements: Vec<Measurement>`: `shot_measurements` then `flight_measurements`, over the
     analysed view (call 2).
   - `unscored: Vec<UnscoredCheckpoint>`: the photo-side refusals (call 3) in the device's declared
     tile order, then `flight_unscored`'s. That is the order `analyze_swing` uses, checkpoints then
     flight.
   - `notes: Vec<String>`: ADR-014's needs-review note in `engine.rs:832`'s sentence, and whatever
     the shell appends (the handedness and loft gaps, P11).

   **Never carried in a `SwingResult`**, so `printed_blank` and `misread` never reach
   `analysis.json`, which frozen Python reads with a closed enum. A Rust-owned schema,
   `spec/schemas/shot_result.schema.json`, hand-maintained as `shot_data`'s is, for M38's Dart side.
2. **`analyze_shot(&ShotRequest) -> Result<ShotResult, ContractError>`** in
   `crates/analysis/src/engine.rs`, beside `analyze_swing`. Fields are `swing_id`, `session_id`,
   `shot`, `intent: Option<&PracticeGoal>`, `handedness`, `loft_deg`, `tiles: &[UnscoredCheckpoint]`
   (the screen's refusals, passed in by the shell) and `photo_sha256`.
   - **The analysed view**: a copy of the shot with every field outside `printed_on(shot)` ∩
     `FieldUse::Analysed` set to `None`, built through `serde_json::Value` as
     `capability::fields_holding_a_value` reads one. `shot_measurements`, `fly_shot` and
     `flight_measurements` all run on the view. That is "not printed produces nothing" by
     construction, with no per-row field list to drift. A helper `capability::analysed_on(shot)`
     returns that field set.
   - **It refuses an undeclared device**, as `printed_on` does.
   - **Decision 12 as a pin**: on every committed corpus vector carrying a shot, `analyze_shot`'s
     measurements equal that vector's recorded shot and flight rows, and its flight `unscored`
     entries equal the vector's last ones. Adding or removing video can never move a shot number.
3. **The photo-side refusals** come from `crates/screen`, as `screen::tile_refusals(&ShotData) ->
   Result<Vec<UnscoredCheckpoint>, …>`.
   - **Which fields**: a field in `printed_on`, declared `analysed`, holding no value. A `shown_only`
     tile that is blank is shown as blank and refused nothing.
   - **The rule** (decision 3, M34 P8 finding 3):

     | what the parse recorded for the field's label | reason |
     |---|---|
     | no `raw_fields` key (located, withheld by the tie rule) | `misread` |
     | a `blank_markers` match | `printed_blank` |
     | `""` | `printed_blank` (decision 3) |
     | other text that gave no value | `misread` |

   - **None on an unstamped shot or a direct feed.** An unstamped shot's printed set is only the
     fields holding a value, so it names no blank (finding 8). A shot with no provenance was never
     a photo, and both reasons are photo-side; a direct device's blanks wait for one (M40).
   - **`name` is the `ShotData` field** (`spin_rate`), the vocabulary `devices.json` and
     `fields_present` speak. **`detail`** names the tile's label and what was seen, worked in P3 and
     pinned by the shot vectors.
4. **The engine policy.** `policy_for` maps every `PracticeMode` to the one mechanics-only blend,
   named for what it does, so the per-mode seam stays visible as a `match`. **No `ANALYSIS_VERSION`
   bump**: every recorded input is `FUNDAMENTALS` or none, so no recorded answer moves, and a
   non-`FUNDAMENTALS` result could not be produced before. The v17 dry run proves it (P2, P6, P13).
   ADR-009 gets the addendum.
5. **The intent record** (decision 2).
   - `SwingManifest.intent: Option<PracticeGoal>` and `SessionMeta.intent: Option<PracticeGoal>`,
     both **written only when set** (`skip_serializing_if`). So every manifest frozen Python wrote,
     and every recorded storage op, reads and writes byte-for-value as before, and the file-key
     escape CONFORMANCE §4 waits for is not needed. P10 builds it only if its dry run finds a
     recorded store write that moves.
   - **Stamping**: `Upload` gains the cursor's `intent`, stamped onto a swing that has none, as
     `club` is. `SwingBundleStore::set_intent(session, swing, Option<PracticeGoal>, now)` is the
     repair, and it overwrites, as `set_mishit` does.
   - **Reading**: `None` is the default goal, `FUNDAMENTALS`, so it is tracked, which is every swing
     on disk. `PracticeMode::is_tracked()` is the one rule (only `Drill` is false), derived and
     never stored (ADR-034 §3).
   - **What a recorded goal holds**: the mode and, for `SHOT_SHAPING`, the shape. `club` stays the
     default, and the shell fills the category from the manifest's club when it analyses, as
     `api/pipeline.py:1251` does. `set-intent` refuses `--shape` without `shot_shaping`, and
     `shot_shaping` without `--shape`, because a shape attempt with no shape declared grades nothing
     (Decision 17).
6. **The corpus** (`career-v2`).
   - **Two kinds of entry.** A *swing* has a face-on clip, and is read exactly as today. A *photo
     entry* has no face-on clip and has a shot photo. A down-the-line clip does not make it a swing,
     because nothing is scored from that clip alone. A manifest with neither is `NoFaceOn`, and its
     sentence is rewritten to say there is neither a face-on clip nor a shot photo.
   - **Dedupe.** Photo entries group on the photo hash, and the earliest arrival survives (the
     `arrival` key, as swings do). Then a surviving photo entry whose photo is a surviving swing's
     `shot_sha256` is a `Duplicate` of that swing (decision 4), and of the earliest such swing when
     one photo sits on two (the `one-photo-two-clips` shape).
   - **Untracked.** After the dedupe, a survivor whose intent is not tracked is excluded with a new
     `ExclusionReason::Untracked` (`untracked`), and is **not** in `swings`, as a duplicate is not.
     So no count, aggregate, mishit median, `distinct_*` or report sees it. Its duplicates are
     duplicates of it, and stay excluded.
   - **A photo entry reads `shot_analysis.json`** through the new tolerant reader: `analyzed`,
     `analysis_version` and `outdated` (against `comparable_from`), `stale` (when `photo_sha256` is
     absent or differs from the manifest's photo hash), `shot_needs_review` (through the existing
     `needs_review` on `ShotResult.shot`), and `measurements`. A swing's `analysis.json` beside it
     is never read for a photo entry, and a swing's `shot_analysis.json`, if one ever exists, is not
     read either: one artifact per kind.
   - **Shape.** `CorpusSwing.face_on_sha256: Option<String>`. `artifact_key` gives a `pose:` row
     `None` when there is no clip, though a photo entry carries none. `CorpusSwing.intent:
     Option<PracticeGoal>`, written only when set. `CAREER_VERSION` 1 → 2.
7. **The shot family** (`spec/vectors/shot/<case>.json`). `input` is the `run-shot` request
   (`swing_id, session_id, shot, intent?, handedness?, loft_deg?, photo_sha256?`), and `expected` is
   the `ShotResult`.
   - **`provenance.oracle: "hand"`**, and its `note` says where each number came from. The
     composition (which rows, which refusals, in what order, the intent and the notes) is worked by
     hand. Where the shot's analysed fields equal a committed corpus vector's `input.shot`, the
     measurement values are that vector's Python-recorded rows. Where no Python vector holds them
     (a re-read shot whose fields differ), they are Rust's, and the note says so per value.
   - **Re-recorded under `ANALYSIS_VERSION`** with the engine and stage families, because its
     numbers are the engine's.
   - **Gated** by `crates/core/tests/shot.rs` through the `run_shot` seam, so the screen's refusals
     are exercised end to end.
8. **The new storage cases** (`spec/vectors/storage/hand/`, `oracle: "hand"`, `career_version` 2,
   each with its working in `provenance.note`):
   - corpus cases: a photo entry analysed and pooled; two photo entries with one photo; a photo-only
     copy of a swing's photo (decision 4); one photo on two swings and a photo-only copy; a mixed
     session; a `DRILL` swing and a `DRILL` photo entry, each `untracked`; a drill among five carries
     of one club, so the mishit median sees only the tracked four; a photo entry not analysed, stale
     (wrong `photo_sha256`) and outdated; a down-the-line-only manifest (`NoFaceOn`);
   - op cases: an upload stamping the session's intent, a second upload not overwriting it,
     `set_intent` overriding and clearing.
9. **Schemas.** `shot_result` is new and Rust-owned. `swing_manifest` and `session_meta` become
   Rust-owned when they gain `intent`: hand-edited, added to `RUST_OWNED_SCHEMAS` in
   `scripts/conformance.py`, so `regenerate --schemas-only` stops writing them. `tests/test_conformance.py`
   pins the set, and the Rust key-set pins move to `crates/contracts/tests/schemas.rs`'s pattern.
10. **The verbs** (P4, P11), each with its doc naming what it does not do.
    - `golf-core run-shot`, no arguments: a request (or a shot vector) on stdin, the `ShotResult`
      on stdout, as `parse-screen` reads.
    - `golf-core run-shot --session S [--swing N]`: every photo entry in the session, or the one
      named. It reads the shot from the shot store by the photo hash, handedness through the golfer
      store, the loft through `loft_for_club`, and the intent off the manifest. It writes
      `shot_analysis.json` atomically, and prints one line per entry. It refuses a swing with a
      face-on clip by name, because that shot is analysed with its swing. A photo the store has no
      parse for is named with the frozen import that would parse it, and skipped. A handedness or
      loft gap is appended to `notes` in the shot's own words, not `head_stays_back`'s.
    - `golf-core set-intent --session S (--swing N | --default) --mode M [--shape SHAPE]`, and
      `--clear` for one swing, through `set_intent` or the session cursor.
    - **Directories**: `--sessions-dir`, `--golfers-dir`, `--shots-dir`, defaulting to the
      `GOLF_SESSIONS_DIR`, `GOLF_GOLFERS_DIR` and `GOLF_SHOTS_DIR` environment variables, else
      `data/processed/{sessions,golfers,shots}`, as M36's call 11 resolves them.
    - **The clock** is `Timestamp::now_utc()` in the binary alone, as M36 left it.
11. **"A drill reaches no aggregate", pinned twice.** In `crates/storage`: a corpus with a drill is
    the corpus without it, plus one `untracked` exclusion. In `crates/core/tests/career.rs`: every
    aggregate and report over that corpus equals the one over the corpus with the drill deleted from
    disk, except the excluded list the verbose report prints.
12. **The frozen-Python hazard** (finding 3) is named in ADR-024's addendum, in `set-intent`'s doc,
    and in the WORKLOG for M29: the verb is safe on a session the frozen server no longer writes to.

---

## Phases

### P0 — This plan document, and the pointers to it

- **Goal**: this file, approved, and the two pointers to it.
- **Files**: `docs/plans/m35-shot-first-sessions.md` (this file); `ROADMAP.md` §M35 (a **Plan** line
  pointing here) and its status-table row (the plan link); the program plan's status-table row for
  M35 (`docs/plans/m31-m40-shot-first-pivot.md`).
- **Done when**: the user has approved the plan, the pointers resolve, and docs-truth is green.
- **Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P1 — `crates/contracts` I: the tracked rule, `ShotResult`, its schema; the `data/` snapshot

- **Goal**: decision 1's rule and call 1's shape, before anything produces one.
- **Read**: `crates/contracts/src/{intent,swing,unscored,lib}.rs`; `crates/contracts/tests/{schemas,round_trip}.rs`;
  `spec/schemas/shot_data.schema.json` (the hand-maintained precedent); `scripts/conformance.py`
  around `RUST_OWNED_SCHEMAS` (`:138`).
- **Builds**:
  - `PracticeMode::is_tracked()`. The variant docs say what each mode feeds (decision 1's table).
  - `crates/contracts/src/shot_result.rs::ShotResult` with `Validate` (nested, as `SwingResult`
    validates), registered in `lib.rs`. The module doc says it has no Python twin and never reaches
    `analysis.json`.
  - `spec/schemas/shot_result.schema.json`, hand-written. `RUST_OWNED_SCHEMAS` gains `shot_result`,
    and `tests/test_conformance.py` pins it.
- **Tests**: only `Drill` is untracked, over `PracticeMode`'s every variant; a `ShotResult` round
  trips, absent keys apart from null ones; the schema's key set equals the struct's (the
  `schemas.rs` pattern); a `ShotResult` holding `printed_blank` validates.
- **Also**: take the `data/` digest and record it in the findings, expected `bd384cc4…` (finding 11).
- **Files**: `crates/contracts/src/{intent,shot_result,lib}.rs`; `crates/contracts/tests/*`;
  `spec/schemas/shot_result.schema.json` (new); `scripts/conformance.py`; `tests/test_conformance.py`.
- **Done when**: green; the digest matches.
- **Verify**: `cargo test -p contracts`; `cargo test`; clippy; fmt; pytest; ruff; `conformance.py check`.

### P2 — `crates/analysis`: every mode reaches the engine, and `analyze_shot`

- **Goal**: decision 1 per swing (call 4), and call 2.
- **Read**: `crates/analysis/src/{scoring,engine,flight_measure,shot_measure}.rs` (the engine:
  `analyze_swing`, `shot_measurements`, `flight_measurements`, `measurements`, `:381–626`);
  `crates/contracts/src/capability.rs`; `crates/analysis/tests/flight.rs` and
  `crates/core/tests/engine.rs` for how corpus vectors are loaded.
- **Builds**:
  - `policy_for` returns the mechanics-only blend for every mode, renamed for what it does. The
    module doc and the `#[should_panic]` test change to say why.
  - `capability::analysed_on(shot)`: `printed_on` ∩ `FieldUse::Analysed`.
  - `ShotRequest` and `analyze_shot` (call 2), validating the result as `analyze_swing` does.
- **Tests**:
  - `analyze_swing` under `DRILL`, `SHOT_SHAPING` and `PERFORMANCE` scores what `FUNDAMENTALS`
    does, with `intent` recording the real mode.
  - **The Decision-12 pin**: over every committed corpus vector with a shot, `analyze_shot`'s rows
    equal the recorded shot and flight rows, positionally, and its flight `unscored` entries equal
    the vector's last ones.
  - A `shown_only` field holding a value stays on `ShotResult.shot` and reaches no measurement.
  - An undeclared device is refused.
  - The refusals passed in (`tiles`) come first in `unscored`, in the order given.
- **Files**: `crates/analysis/src/{scoring,engine}.rs`; `crates/contracts/src/capability.rs`;
  `crates/analysis/tests/shot.rs` (new).
- **Done when**: green; the v17 dry run reports 0 changed.
- **Verify**: `cargo test`; clippy; fmt; `rerecord --declare spec/declarations/v17.json --dry-run`.

### P3 — `crates/screen`: a printed tile with no value, as `printed_blank` or `misread`

- **Goal**: call 3.
- **Read**: `crates/screen/src/{lib,parser,profile,read}.rs` (`read_cell`, `is_blank`,
  `golfer_warnings`, the profile's field→label map); `crates/screen/tests/hand.rs`
  (`a_withheld_tile_is_located_and_unread_and_nothing_else_is`); the M34 plan's P8 finding 3.
- **Builds**: `screen::tile_refusals(&ShotData)`, the table in call 3, reading the profile named by
  `provenance.device`. Each `detail` names the label and what was read.
- **Tests**: over the screen family's Rust-read shots, a tie-withheld field (`impact-pair-*`) is
  `misread`, an empty Spin cell is `printed_blank`, and a read value is never refused. An unstamped
  shot and a shot with no provenance give none. A `shown_only` blank gives none. An unknown device
  is refused. The order is the device's declared order.
- **Files**: `crates/screen/src/{lib,tiles}.rs` (`tiles` new, or inside `read.rs` if the phase finds
  it reads better there); `crates/screen/tests/tiles.rs` (new).
- **Done when / Verify**: green; `cargo test -p screen`; `cargo test`; clippy; fmt.

### P4 — `crates/core` I: the `run_shot` seam, `golf-core run-shot` on stdin, the shot family's gate

- **Goal**: the shell that joins P2 and P3, its verb, and the first shot vectors.
- **Read**: `crates/core/src/{lib,bin/golf_core}.rs` (the `run` seam, `parse-screen`, the
  vector-or-bare input reader `:240–250`); `crates/core/tests/{engine,parse_screen}.rs`.
- **Builds**: `core::run_shot(request) -> Result<ShotResult, …>`, which calls `screen::tile_refusals`
  and then `analyze_shot`, and fills the intent's club category as call 5 says. The `run-shot` verb
  with no arguments (call 10). The `spec/vectors/shot/` directory.
- **Vectors** (call 7), the synthetic ones: a `DRILL` intent (the result differs from `FUNDAMENTALS`
  in `intent` alone); a `mock` direct feed (no refusals); `club_head_speed` present on an HD Golf shot
  (no measurement); no handedness; no loft; a stamped shot with a `---` tile, an empty tile, a tied
  tile and an unreadable one.
- **Gate**: `crates/core/tests/shot.rs` runs every case through the seam, and once through the verb
  as `parse_screen.rs` does.
- **Files**: `crates/core/src/{lib,bin/golf_core}.rs`; `crates/core/tests/shot.rs` (new);
  `spec/vectors/shot/*.json` (new).
- **Done when / Verify**: green; `cargo test`; clippy; fmt.

### P5 — The shot family's cases from the corpus and screen families

- **Goal**: call 7's real cases.
- **Read**: P2's and P4's findings; `spec/vectors/corpus/` (which carry `input.shot`);
  `spec/vectors/screen/{corpus,hand}/` and their expected reads.
- **Builds**: one shot case per corpus vector carrying a shot, its numbers from that vector's
  recorded rows; and one per stamped screen read that crosses a row of call 3's table or a flight
  outcome no corpus case reaches. Each `provenance.note` names its numbers' source.
- **Files**: `spec/vectors/shot/*.json`.
- **Done when**: green; every case's note names its source; no other family changed.
- **Verify**: `cargo test`; `git diff -- spec/vectors/{synthetic,corpus,stages,screen}` empty.

### P6 — `golf-core rerecord` gains the shot family; the unchanged-swing pin

- **Goal**: an `ANALYSIS_VERSION` bump re-records the shot family with the engine's (call 7), and
  the swing output is pinned unchanged.
- **Read**: `crates/core/src/rerecord.rs` (the module doc, `Family`, `plan`, the engine walk
  `:1050–1210`); `crates/core/tests/rerecord.rs`; `docs/CONFORMANCE.md` §4; `scripts/conformance.py`'s
  `check` deferrals.
- **Builds**: `Family::Shot` under `Version::Analysis`, run through `run_shot`. `conformance.py
  check` defers the shot family to `cargo test`, as it does screen, and `tests/test_conformance.py`
  says so.
- **Tests**: an engine declaration's dry run counts the shot family; a declaration naming a shot
  path re-records only that path; a screen or career declaration never opens `shot/`.
- **Done when**: green; the v17 dry run reports 0 changed across engine, stage and shot.
- **Verify**: `cargo test`; clippy; fmt; pytest; ruff; `conformance.py check`; the v17 dry run.

### P7 — `crates/storage` I: intent on the manifest, the session cursor, `set_intent`

- **Goal**: decision 2 and call 5, as library code.
- **Read**: `crates/storage/src/{manifest,bundle_store,lib}.rs`; `src/golf_coach/storage/session_meta.py`
  (whole) and `tests/storage/test_session_meta.py`; `crates/storage/tests/python_schemas.rs`;
  `crates/contracts/tests/python_schemas.rs`; `spec/schemas/{swing_manifest,session_meta}.schema.json`.
- **Builds**:
  - `SwingManifest.intent`, written only when set.
  - `crates/storage/src/session_meta.rs`, a port of `session_meta.py` (the tolerant load, the save,
    the cursor update keeping the two cursors independent) plus the `intent` cursor, written only
    when set, with `now` as an argument.
  - `Upload.intent` stamping, and `SwingBundleStore::set_intent`.
  - Both schemas hand-edited to carry `intent` and moved into `RUST_OWNED_SCHEMAS`, with their pins.
- **Tests**: the Python suite's session-meta cases, mirrored; a manifest without `intent` reads and
  re-saves to the same value; one with it round-trips; stamping onto a new swing, never over a set
  intent; `set_intent` overrides and clears. **Every recorded storage vector still passes
  unchanged**, which is the proof that skip-if-`None` moved no store write.
- **Files**: `crates/storage/src/{manifest,session_meta,bundle_store,lib}.rs`;
  `crates/storage/tests/*`; `crates/contracts/tests/python_schemas.rs`;
  `spec/schemas/{swing_manifest,session_meta}.schema.json`; `scripts/conformance.py`;
  `tests/test_conformance.py`.
- **Done when / Verify**: green; `cargo test`; clippy; fmt; pytest; ruff; `conformance.py check`.

### P8 — `crates/storage` II: `CorpusSwing`'s new shape, the `shot_analysis.json` reader and writer, `loft_for_club`

- **Goal**: everything P10's change reads, landed with no recorded answer moving.
- **Read**: `crates/contracts/src/career.rs` (`CorpusSwing`, `ExclusionReason`, `artifact_key`);
  `crates/storage/src/state.rs`; `src/golf_coach/storage/flight_inputs.py` (`LoftGap`,
  `loft_for_club`, `_declared_lofts`, `_resolve_loft`) and its tests.
- **Builds**:
  - `CorpusSwing.face_on_sha256: Option<String>`, `CorpusSwing.intent` (written only when set),
    `ExclusionReason::Untracked` (appended to `ALL`), and `artifact_key`'s `None` for a `pose:` row
    with no clip.
  - `state::load_shot_analysis(swing_dir) -> Option<ShotResult>` and `save_shot_analysis`
    (atomic, as the stores write), with `SHOT_ANALYSIS_NAME`.
  - `crates/storage/src/flight_inputs.rs`: `LoftGap` and `loft_for_club`, faithful. The
    corpus-wide `read_flight_inputs` is M29's and is named as not ported.
- **Tests**: every recorded storage and career vector passes unchanged (`face_on_sha256` still
  serializes as the string, `intent` is absent); a missing, unreadable or invalid
  `shot_analysis.json` is `None`; `loft_for_club`'s four gaps and its answer, mirroring the Python
  tests.
- **Files**: `crates/contracts/src/career.rs`; `crates/storage/src/{state,flight_inputs,lib}.rs`;
  tests beside each.
- **Done when / Verify**: green; `cargo test`; clippy; fmt.

### P9 — The hand-worked storage cases for the new corpus rules, skipped by name until P10

- **Goal**: call 8, worked from the rules before the code that follows them.
- **Read**: call 6 and call 8; `spec/vectors/storage/hand/` (M36 P14's cases, their shape and
  notes); `crates/core/src/storage_family.rs` (how a corpus case and an op case run);
  `crates/core/tests/storage.rs` (how cases are listed and skipped).
- **Builds**: the cases at `career_version` 2, each `expected` worked by hand with its working in
  `provenance.note`. The op cases need `storage_family.rs` to read `intent` on an upload and a
  `set_intent` op, built here with the cases that use them. The gate skips every new case by name,
  "P10 passes these".
- **Files**: `spec/vectors/storage/hand/*.json` (new); `crates/core/src/storage_family.rs`;
  `crates/core/tests/storage.rs`.
- **Done when**: green with the new cases skipped by name; each note shows its working.
- **Verify**: `cargo test`; clippy; fmt.

### P10 — The change: `read_corpus` admits photo entries and leaves drills out; `career-v2`

- **Goal**: call 6, in Rust only, and its re-record.
- **Read**: `crates/storage/src/corpus.rs` (whole); P8's and P9's findings; the M36 plan's P14
  (the re-record's shape) and `spec/declarations/career-v1.json`.
- **Builds**:
  - `read_corpus`: photo entries, the two dedupes, the `untracked` exclusion, the rewritten
    `NoFaceOn` sentence. The module doc's "What is read where, and in which order" says where each
    new exclusion falls in `excluded`.
  - `CAREER_VERSION` 1 → 2, with its ledger entry.
  - Call 11's two pins.
  - **The re-record**: a dry run lists every moved path; each is checked against call 6 and named
    in `spec/declarations/career-v2.json` with the reason in its `note` (finding 5 is the expected
    one). Then the real run, then a second run that writes nothing. The P9 cases are un-skipped and
    pass. If the dry run shows a store write moving, stop and build CONFORMANCE §4's file-key escape
    as its own finding first.
- **Measure**: `read_corpus` over `data/` pools what it pooled before M35, since no photo entry
  exists there (finding 11). The digest is re-checked.
- **Files**: `crates/{contracts,storage}/src/…` as above; `crates/core/tests/career.rs`;
  `spec/declarations/career-v2.json` (new); the re-recorded `spec/vectors/{storage,career}/**`.
- **Done when**: green; the dry run reports only declared paths; the second run writes nothing;
  `data/` is unchanged.
- **Verify**: the `career-v2` dry run, then for real, then again; `cargo test`; clippy; fmt.

### P11 — `crates/core` II: `golf-core run-shot --session` and `golf-core set-intent`

- **Goal**: decisions 6 and 7, call 10.
- **Read**: `crates/core/src/bin/golf_core.rs` (the M36 verbs' dispatch and directory resolution);
  `api/pipeline.py::{_shot_for,_handedness_for,_loft_for,loft_remedy}`; P7's and P8's findings.
- **Builds**: the two verbs; the handedness and loft gap notes, worded for a shot; each verb's doc
  naming the frozen-Python hazard (call 12).
- **Tests**, all on `tempfile` copies of a P9 tree plus a golfer, a bag and a shot store:
  - `run-shot --session` writes `shot_analysis.json` for each photo entry and none for a swing;
    a re-run writes the same bytes; the written file reads back through `load_shot_analysis`; and
    `read_corpus` then pools the entry;
  - a photo with no stored parse is named and skipped; a missing golfer or loft gives its note;
  - `set-intent` stamps one swing, the session default, and clears; the refusals in call 5 hold;
    and a manifest it wrote reads in frozen Python's `SwingManifest` (one scratch check, recorded in
    the findings, as M36 P16 checked `flag-mishit`'s);
  - a drill set by `set-intent`, then `run-shot`, is stored, analysed and absent from the corpus.
- **Files**: `crates/core/src/{lib,bin/golf_core}.rs`; `crates/core/tests/verbs.rs` (new, or beside
  `reports.rs` if the phase finds the harness there).
- **Done when**: green; `data/` unchanged.
- **Verify**: `cargo test`; clippy; fmt; the digest.

### P12 — Docs: CONFORMANCE, ARCHITECTURE, `data/README.md`, the ADR addenda, `CLAUDE.md`

- **Goal**: every doc that describes what M35 changed says so, and docs-truth is green.
- **Read**: `docs/CONFORMANCE.md` §2 and §4; `docs/ARCHITECTURE.md` §1, §3 and §4; `data/README.md`;
  `spec/README.md`; `CLAUDE.md`; ADR-009, ADR-024, ADR-034 (Consequences and its last addendum's
  form), ADR-014 and ADR-035; `docs/README.md` (the addenda counts); every phase finding above.
- **Writes**:
  - `docs/CONFORMANCE.md`: the shot family (shape, oracle, version key, gate); the storage family's
    new hand cases and `CAREER_VERSION` 2; `check`'s new deferral; the file-key escape only if P10
    built it.
  - ADR addenda, one each: **ADR-009** (every mode scores mechanics; the mode routes the stats;
    the three single-swing policies retired); **ADR-024** (photo entries, the two dedupes,
    `untracked`, `career-v2`, the frozen-Python hazard); **ADR-034** (the Consequences' two open
    questions answered, and the two photo-side reasons first emitted, with decision 3's rule). P12
    reads ADR-014 and ADR-035 and adds one only if a clause there is now false. `docs/README.md`'s
    addenda counts move with them.
  - `docs/ARCHITECTURE.md` §1 (the two verbs, the new modules), §3 (`analyze_shot` beside
    `analyze_swing`), §4 (`shot_analysis.json`, `intent` in the manifest and in `session.json`).
  - `data/README.md` and `spec/README.md` for the same files.
  - `CLAUDE.md`: the crate paragraph (`shot_result`, `session_meta`, `flight_inputs`,
    `screen::tile_refusals`, `run_shot`), the commands block (`run-shot`), and the "how a port is
    checked" row (the shot family).
  - The program plan's §M35: a one-line pointer here, and any line a finding made false.
- **Done when**: docs-truth and pytest green; every new path cited exists.
- **Verify**: `pytest tests/test_docs_truth.py`; `pytest`.

### P13 — Close: verify, WORKLOG, ROADMAP, memory, and the one commit

- **Goal**: the exit below, checked, recorded, and committed once.
- **Steps**: run every command in "Verify", the v17 dry run (0 changed) and the `career-v2` dry run
  (0 changed); compare the `data/` digest a last time; `git diff -- src/` empty;
  `git diff -- spec/vectors/{synthetic,corpus,stages,screen,audio,format}` empty. Write the WORKLOG
  entry: what was built, what M37, M29 and M38 should know (the frozen-Python hazard, the one
  artifact per kind, `run-shot` needing a frozen parse until `ort`), and what is left open. ROADMAP:
  the status row, §M35 to ✅ with the date, M37 unblocked, and the NEXT ACTION paragraph; the program
  plan's status row the same; `docs/README.md`'s document count (this file becomes tracked). Update
  the `app-transition-decisions` memory (M35 done, next M37, M41 after M35). Then one commit onto
  `main`.
- **Done when**: the commit is made and the exit holds.

---

## Exit

M35 is done when all of these hold, each checked at P13:

1. `ShotResult` exists, and `analyze_shot` writes the shot and flight rows over the analysed view.
   On every committed corpus vector with a shot, those rows equal the swing's recorded ones
   (Decision 12).
2. Every `PracticeMode` reaches the engine and scores mechanics, and the swing output is unchanged:
   the v17 dry run reports 0 changed across the engine, stage and shot families.
3. `printed_blank` and `misread` are emitted into `ShotResult.unscored` by call 3's rule, and never
   into a `SwingResult`.
4. `spec/vectors/shot/` exists, hand-worked with each number's source named, gated by `cargo test`
   and re-recordable under `ANALYSIS_VERSION`.
5. `read_corpus` admits photo entries, dedupes them by photo with the copy that has video winning,
   and leaves an untracked shot out, named. The hand-worked storage cases pass, `career-v2` moved
   only declared paths, and a second run writes nothing.
6. A `DRILL` shot is stored, analysed into its own `ShotResult`, and reaches no aggregate (call 11's
   pins).
7. A shot's intent is recorded on its manifest, stamped from `session.json`'s cursor and overridable
   per shot. `golf-core set-intent` and `golf-core run-shot --session` work on temporary copies.
   Handedness reaches a photo entry through its manifest's golfer, and loft through
   `loft_for_club`.
8. `git diff -- src/` is empty, `data/` is unchanged, and the engine, stage, screen, audio and format
   families are untouched.
9. `cargo test`, clippy, fmt, pytest, ruff, mypy and `conformance.py check` are green.

---

## Phase findings

*(Appended as each phase closes.)*

### P1 (2026-10-09)

1. **The `data/` snapshot**: 192 files under `data/processed/{sessions,golfers,shots}/`, digest
   `bd384cc40b236b73e3013b817ae79908d6e5ff4dded25b8803fc35c6b7af01cc`, by the command in "Verify".
   It is finding 11's and M36 P1's. **For P10, P11 and P13**: compare against this full digest.
2. **`shot_result` has no Python model, so it cannot be in `SCHEMA_ROOTS`**, and two Python pins
   assumed every committed schema and every `RUST_OWNED_SCHEMAS` name is a root. The plan did not
   say so; the change stayed inside the files it names. In `tests/test_conformance.py`:
   `test_no_schema_is_committed_without_a_root_that_produces_it` now admits
   `SCHEMA_ROOTS | RUST_OWNED_SCHEMAS`, the typo guard in
   `test_schemas_only_writes_the_python_owned_roots_and_none_of_rusts` checks the names against the
   committed files instead of the roots, and the new pin
   `test_shot_result_is_the_one_rust_owned_schema_with_no_python_model` holds
   `RUST_OWNED_SCHEMAS - SCHEMA_ROOTS == {"shot_result"}`. `conformance.py`'s comment on the set says
   why, and `regenerate`'s help no longer says "the three".
3. **For P7: `RUST_OWNED` in `crates/contracts/tests/schemas.rs` cannot simply gain `swing_manifest`
   and `session_meta`.** Every test that walks it reads `ShotData` and `ShotProvenance` out of each
   file, and those two schemas hold neither. Give the storage roots a list of their own (or split
   the walk), keep `conformance.py::RUST_OWNED_SCHEMAS` the union, and mind CONFORMANCE §1's "two
   lists name the Rust-owned roots". Both storage roots keep their Python models, so they stay in
   `SCHEMA_ROOTS` and P1's new pin stays true.
4. **`ShotResult`'s serde choices** (`crates/contracts/src/shot_result.rs`), so the schema and the
   readers agree: `required` is `swing_id`, `session_id`, `shot`. `analysis_version` defaults to 0,
   for `SwingBundleResult`'s reason: a file without it reads as older than every engine. `photo_sha256`
   defaults to `None` and is **written as `null`**, never skipped. `intent` defaults to
   `PracticeGoal::default()`, so it is tracked, consistent with call 5's "`None` is the default goal".
   The three lists default to `[]`. `Validate` recurses into `intent`, `shot`, `measurements` and
   `unscored`. **For P2**: `analyze_shot` sets `analysis_version` and `intent` explicitly. **For P8**:
   the tolerant reader gets "absent version reads as outdated" and "absent photo hash reads as stale"
   from these defaults, with no special case.
5. **The schema** (`spec/schemas/shot_result.schema.json`) was written in `export_schemas`' format
   (indent 2, sorted keys, ASCII escapes, trailing newline, CRLF like every file under
   `spec/schemas/`). `ShotData` and `ShotProvenance` are copied from `swing_result`'s `$defs`. Its
   `UnscoredReason` is Rust's whole set, and its `PracticeMode`, `TargetShape` and
   `UnscoredCheckpoint` descriptions are written for Rust. `crates/contracts/tests/schemas.rs` gained
   `shot_result` in `RUST_OWNED`, which puts it under the shot-key, identical-copies and M32-description
   pins. `the_three_copies_of_each_shot_shape_are_identical` was renamed
   `every_copy_of_each_shot_shape_is_identical`. Three new tests: the root's key set, its `required`
   set and each spelled `default` against serde; the schemas' `PracticeMode`, which is frozen Python's
   four, with only `drill` untracked; and `shot_result`'s `UnscoredReason` covering every reason.
6. **For P2**, three sentences go false when `policy_for` stops refusing. The `PracticeMode` enum doc
   in `crates/contracts/src/intent.rs` still says "`analysis::scoring::policy_for` keeps refusing
   them", and P1 left it for P2 on purpose; the variant docs P1 wrote speak only of what each mode
   feeds. `scoring.rs`'s module doc (`:9`) and `policy_for`'s doc (`:86`) say the same. The two swing
   schemas (`swing_result`, `swing_bundle_result`, Rust-owned and hand-edited) describe `PracticeMode`
   in frozen Python's words, "rejects with `NotImplementedError` until full M4". Edit both alike.
   `shot_result.rs`'s module doc says "nothing builds one before M35 P2", so update it when
   `analyze_shot` lands.
7. **For P3**: the `detail` strings in `shot_result.rs`'s unit-test fixture (`"Spin: empty"`,
   `"Spin Axis: '8S'"`) are illustrative and pinned to nothing. P3 owns the real wording.
8. **For P12**: CONFORMANCE §1 now undercounts. It says "four tests over the three files" (`:105`,
   now seven tests over four files) and "the three in `RUST_OWNED_SCHEMAS`" (`:1228`). Its "writes
   the other seven" is still true, because `shot_result` is not a root. §1 should also say that
   `shot_result` is the first schema with no Python model, and name the new Python pin.
9. **Verified**: `cargo test` (every suite ok), `cargo clippy --all-targets` (no warnings),
   `cargo fmt --check`, pytest (2052 passed), `ruff check`, `conformance.py check` (exit 0).
   `git diff -- src/ spec/vectors/` is empty, and no dependency was added.

### P2 (2026-10-09)

1. **`policy_for` answers every mode with `mechanics_only`** (`crates/analysis/src/scoring.rs`,
   renamed from `fundamentals`). The `match` keeps an or-pattern arm naming all four modes, so a
   fifth mode still stops the build there. The `#[should_panic]` test became
   `every_mode_blends_mechanics_only`, and `engine.rs` gained
   `every_mode_scores_what_fundamentals_scores_and_records_its_own_mode`. That test compares the
   whole `SwingResult` with only `intent` put back. The v17 dry run reports 0 changed over 42
   vectors, so call 4's "no `ANALYSIS_VERSION` bump" holds.
2. **The analysed view lives in `contracts::capability`, beside `analysed_on`**. The plan put only
   `analysed_on` there. The view needs to know which `ShotData` keys are not readings, and that is a
   fact about `contracts`, so it is `capability::NOT_READINGS` (`shot_id`, `session_id`,
   `timestamp`, `source`, `provenance`). `analysed_view(shot) -> Result<ShotData, ContractError>`
   blanks every other key outside `analysed_on` through `serde_json::Value`. Two things guard it.
   `crates/contracts/tests/capability.rs` used to keep a private copy of the same five keys
   (`NOT_PRINTED_VALUES`); it now imports `NOT_READINGS`. And
   `the_keys_that_are_not_readings_are_the_ones_a_bare_shot_fills` holds the list to the struct.
   **On every corpus shot the view blanks `club_head_speed` and `smash_factor` and nothing else**
   (`on_every_corpus_shot_the_view_blanks_club_speed_and_smash_alone`), which measures finding 7.
3. **The signature, for P4 and P11**: `analysis::engine::{ShotRequest, analyze_shot}`, with
   `ShotRequest<'a> { swing_id: &str, session_id: &str, shot: &ShotData, intent:
   Option<&PracticeGoal>, handedness: Option<Handedness>, loft_deg: Option<f64>, tiles:
   &[UnscoredCheckpoint], photo_sha256: Option<&str> }`, returning `Result<ShotResult,
   ContractError>`. It returns `Err` in two cases: an undeclared device (`field` is
   `DeviceCapability.device`), and a result that fails validation. The second is not a panic as it
   is in `analyze_swing`, because `tiles` and the shot come from the shell. `analysis_version` and
   `intent` are set explicitly (P1 finding 4). `intent` is `None` → `PracticeGoal::default()`.
   `notes` holds only ADR-014's needs-review note. It comes from a new private `needs_review_note`
   that `analyze_swing_bundle` now calls too, so both say it in one sentence. **For P11**: append
   the shell's handedness and loft notes after it. `tiles` are copied first into `unscored` in the
   order given and never sorted. **For P3**: ordering them in the device's declared tile order is
   `tile_refusals`' job, and `the_screens_refusals_come_first_in_the_order_given` passes them out of
   that order on purpose.
4. **The Decision-12 pin holds on every corpus vector** (`crates/analysis/tests/shot.rs`,
   `the_shot_alone_produces_every_committed_shot_and_flight_row`). The rows are the vector's
   `launch_monitor:` and `model:flight_v1` rows, positionally, strings exactly and values within
   `RTOL`. The refusals are the vector's `unscored` entries after its last `CHECKPOINT_REGISTRY` name.
   That tail is empty on the four vectors whose flight refused nothing, and is still compared there.
   The notes are the vector's needs-review notes. No corpus vector has one, so only the unit-level
   test reaches that sentence. Values were compared within `RTOL` and not counted to the bit.
   **For P5**: if a shot case copies a vector's rows as its `expected`, its gate needs the same
   tolerance, or else a census of its own.
5. **For P5: a stamped shot is the case that moves rows**. `a_reading_the_screen_did_not_print_reaches_no_number`
   stamps `2026-08-23-4`'s shot (`parser_version` 1, a `fields_present` without `carry_distance`).
   The result loses `carry_distance_yds` and `start_line_offline_yds`, and every flight row, though
   the shot still holds a carry. Any shot case with a `fields_present` therefore measures differently
   from the corpus vector it came from, so its values are Rust's and its note has to say so (call 7).
6. **Docs P2 corrected**: `intent.rs`'s `PracticeMode` doc and `Performance` variant,
   `scoring.rs`'s module and `policy_for` docs, `engine.rs`'s module doc and `analyze_swing`'s
   outcome paragraph, and `shot_result.rs`'s module doc (P1 finding 6). The `PracticeMode`
   description in `spec/schemas/swing_result.schema.json` and `swing_bundle_result.schema.json` was
   replaced by a raw-text edit, which left the format, CRLF and ASCII escapes as they were. The two
   files carry the same sentence, and `shot_result`'s is left as P1 wrote it. **For P12**: two
   sentences outside `crates/` are now false. ADR-009 `:173` says `policy_for` raises for every
   other mode, and ADR-034 `:385` says `crates/analysis`' mirror panics. They go in ADR-009's
   addendum (call 4) and ADR-034's addendum.
7. **Verified**: `cargo test` (60 suites, all ok, including `analysis`' new `shot` suite of 7),
   `cargo clippy --all-targets` (no warnings), `cargo fmt --check`, the v17 dry run (0 changed),
   pytest (2052 passed, `test_docs_truth.py` among them), and `conformance.py check` (exit 0).
   `git diff -- src/ spec/vectors/` is empty, and no dependency was added.

### P3 (2026-10-09)

1. **What landed**: `crates/screen/src/tiles.rs` (new, no Python twin), re-exported as
   `screen::tile_refusals`. Call 3's table is one `match` (`judged`), and the module doc carries
   the table and why each row is the reason it is. The crate doc lists the module.
2. **The signature, for P4 and P11**: `screen::tile_refusals(shot: &ShotData) ->
   Result<Vec<UnscoredCheckpoint>, ContractError>`. The plan left the error type open. It is
   `ContractError` so that `run_shot` composes it with `analyze_shot` by `?`.
   - The checks run in this order. No provenance gives `[]` before any device is looked up, because
     a direct feed was never a photo. Then `capability_for` refuses an undeclared device
     (`field: "DeviceCapability.device"`, as `printed_on` would), stamped or not. Then
     `load_profile` refuses a device no profile reads: its `UnknownProfile` text becomes the
     `problem`, under `field: "ShotProvenance.device"`. A stamped `mock` record is refused here.
     Only after those does an unstamped shot (`fields_present: None`) give `[]`.
   - A declared field in `fields_present` that the profile has no tile for is a `ContractError`
     (`ShotProvenance.fields_present`), not a panic. No read writes one.
   - **For P4**: pass the shot **as stored**, not `analysed_view`'s copy. The answer is the same
     either way, because only analysed fields are judged, but the stored record is what the rule
     reads.
3. **The `detail` wording, for P5's vectors to pin** (the label bare, as the parser's warnings put
   it, and the text through `pyfmt::str_repr`):

   | row | `detail` |
   |---|---|
   | no `raw_fields` key → `misread` | `"{label}: located, but its label tied with another tile's, so it was not read"` |
   | `""` → `printed_blank` | `"{label}: nothing under the label"` |
   | a blank marker → `printed_blank` | `"{label}: '---', the screen's blank"` |
   | other text → `misread` | `"{label}: 'Open' under the label, and no value could be read from it"` |

4. **What the screen family's Rust-read shots refuse** (a census by scratch script, deleted). Each
   row of the table is reached by a real record, except a blank shown-only tile, which no vector
   has.
   - On every `2026-08-23-*` corpus photo, Spin and Spin Axis hold `""` and are `printed_blank`,
     and so is Impact Position V on every one but `-1`, where it is withheld. On `2026-08-10-2`,
     Impact Position V is the only blank, and `2026-08-10-1` has none.
   - On `2026-08-10-1` and `2026-08-23-1`, and on the hand vectors `impact-pair-v-dropped`,
     `-v-joined` and `-real-joined`, the tie rule withholds the `Impact Position` pair, and both
     fields are `misread`. On `carry-twice` it withholds Carry. `impact-pair-exact` and
     `-v-read-as-y` withhold nothing: their V tile reads `""`.
   - **`2026-08-23-3` is the only `misread` from text**: `'Open'` under Club Face Angle, a
     direction word with no digits.
   - The synthetic screens hold a `---` the OCR saw, and it is `printed_blank`.
   - **For P5**: `2026-08-23-1` covers both reasons on one shot, `2026-08-23-3` covers text, and
     any synthetic screen covers the marker. `one_bay_photo_refuses_exactly_its_two_blanks_and_its_tied_pair`
     pins `2026-08-23-1`'s four in full. A stamped shot moves rows (P2 finding 5), so these cases'
     measurement values are Rust's.
5. **`contracts::capability::fields_holding_a_value` is now `pub`**. That file is outside P3's
   Files line. It was taken rather than copied because it is the one definition of "holds a
   value", and `printed_on`'s unstamped rule already reads it. Its doc says why it is public.
6. **Two sentences P3 made false, both fixed**:
   - `crates/screen/tests/capability.rs` said the declared order is "a convenience … nothing
     downstream reads". `tile_refusals` now reads it. The sentence is replaced, and a new pin,
     `the_declaration_lists_its_fields_in_the_profiles_tile_order`, holds the declared order to the
     profile's tile order field by field.
   - The `ShotResult` fixture in `crates/contracts/src/shot_result.rs` (P1 finding 7) now uses
     P3's wording. Its misread raw text changed from `"8S"` to `"R"`, because `8S` reads as the
     number 8 and so could not have been a misread.
7. **An observation, changed nothing**. Frozen Python's unstamped records still carry `raw_fields`,
   `""` for a located empty tile included, so on a stored shot a `printed_blank` could be told
   from a raw key alone. That was not done. Without `fields_present`, a missing key cannot say
   withheld from not on this layout. Frozen Python's first-box-wins locating (M34 P8) also means
   its `""` may sit under the wrong tile. Call 3's "none on an unstamped shot" stands, and M29's
   re-read is still the way blanks get named on stored shots.
8. **Verified**: `cargo test` (61 suites, all ok; `screen` gained the `tiles` suite of 9, three unit
   tests in `tiles`, and the order pin), `cargo clippy --all-targets` (no warnings),
   `cargo fmt --check`, and `test_docs_truth.py`. `git diff -- src/ spec/vectors/` is empty, no
   dependency was added, and no Python was touched.

### P4 (2026-10-09)

1. **What landed**: `golf_core::{ShotInput, ShotVector, run_shot}` in `crates/core/src/lib.rs`, with
   two private helpers, `analysed_intent` and `shot_request` (the second split out for the reason
   `bundle_request` is, so `every_shot_input_field_reaches_the_engine` checks it field by field).
   The lib doc gained "The shot seam". `golf-core run-shot`, with no arguments, is in
   `bin/golf_core.rs` with its doc section and usage line. `crates/core/tests/shot.rs` is the gate
   (8 tests), and `spec/vectors/shot/` holds six cases.
2. **`ShotInput` has a key call 7 does not list: `club: Option<ClubId>`, the swing manifest's.**
   P4's Builds line has `run_shot` fill the intent's club category "as call 5 says", and call 5
   fills it from the manifest's club, but a request with no manifest behind it has to carry that
   club. The rule (`analysed_intent`): when a club is given, `intent.club = category_of(club)`, and
   it wins over a category the goal already carries. A recorded goal never holds a club (call 5),
   so the two disagree only in a hand-built request. With no club, the goal's category stands.
   **Call 5's citation is wrong, and the call stands**: `api/pipeline.py:1251` builds
   `PracticeGoal(club=options.club)` from `PipelineOptions.club`, a CLI flag defaulting to `ALL`,
   not from the manifest. So every `analysis.json` on disk says `all`, and a Rust `ShotResult` for
   a tagged swing will say `mid_iron` where its swing twin says `all`. No number depends on it.
   **For P11**: pass the manifest's `club` as `ShotInput.club`. **For P12**: name this where
   ADR-009's addendum and ARCHITECTURE §3 describe the shot's intent.
3. **`ShotInput` refuses unknown keys** (`deny_unknown_fields`), where `VectorInput` ignores them,
   because every shot case is worked by hand and a misspelt `handedness` would otherwise pass as a
   different case. **For P5**: an input carries only `swing_id, session_id, shot, intent, club,
   handedness, loft_deg, photo_sha256`.
4. **The file shape, for P5 and P6**: top-level `id` (`shot/<case>`), `analysis_version`,
   `provenance` (`kind: "shot"`, `oracle: "hand"`, `source`, `note`), `input`, `expected` (a whole
   `ShotResult`). Written as the screen family's hand cases are: indent 4, sorted keys, ASCII
   escapes, **LF**, trailing newline. The gate asserts the kind, the oracle, a non-empty note, the
   id against the file name, and the top-level `analysis_version` against `ANALYSIS_VERSION`.
   **For P6**: that top-level key is what the engine re-record substitutes for the family, and no
   case has a `provenance.rerecords` yet. Inputs are never rewritten, as `v17.json` says of the
   engine's. `cases()` reads every entry of the flat directory as a file, so a sub-family directory
   needs `cases()` extended first.
5. **The six cases, and whose numbers they hold.** The cases are variations on stored shots,
   never a plain one, so P5's one-per-corpus-vector cases duplicate none of them.
   - `drill-intent`: `2026-08-23-4` under `DRILL`, club `7i`. Its rows and refusal are that
     vector's, recorded under `FUNDAMENTALS`, and
     `the_drill_differs_from_fundamentals_in_its_mode_alone` re-runs it to hold the difference to
     `intent.mode`.
   - `mock-direct-feed`: `2026-08-23-4`'s numbers on mock's fields, with no provenance, so no tile
     is refused. Its rows are that vector's values under `source` `launch_monitor:mock`. Leaving
     out `shot_type` moved nothing.
   - `club-speed-shown-only`: `2026-08-23-4` stamped as a bay read, with Club Speed holding a value
     and Smash Factor printed blank. Neither is refused and neither reaches a row. Spin, Spin Axis
     and Impact Position V are `printed_blank`. **This is the first committed blank shown-only tile**
     (P3 finding 4).
   - `no-handedness`: `2026-08-10-1`, which printed a spin axis. **Handedness moves nothing on a
     shot with no printed axis**: `infer_spin_axis` returns `spin_axis_unresolved` before it asks
     for one, so this case could not be on `2026-08-23-4`. With no handedness, the four flight
     values are Rust's (the ball is flown with no axis). `flight_landing_offline_yds` is refused as
     `no_handedness`, a sentence no other committed vector reaches.
   - `no-loft`: `2026-08-23-4` with no loft gives `no_club_loft` in `2026-08-23-10`'s sentence
     form. Its numbers are Rust's, and nothing flies.
   - `stamped-tiles`: `2026-08-23-3` stamped with every row of call 3's table, in declared order:
     Club Face Angle `'Open'`, Spin `'---'`, Spin Axis `''`, and the Impact Position pair withheld,
     with `impact_position` nulled.
6. **Measured: a stamped shot measures exactly as unstamped when its `fields_present` lists every
   field it holds a value for.** P2 finding 5's rows moved because that `fields_present` lacked
   `carry_distance`. `impact_position` is read by nothing in `crates/analysis`, so nulling it moved
   nothing. That is why the three stamped and direct-feed cases keep Python's rows. **For P5**: a
   Rust-read screen shot keeps a corpus vector's rows only where its analysed values equal that
   vector's `input.shot`. Where M34's reader reads a tile differently from the stored parse, the
   values are Rust's, and the note has to say so.
7. **The gate's census** (`the_cases_reach_every_row_of_the_tile_table_and_each_named_gap`) counts
   with `>=` and matches `detail` fragments in P3's wording. So P5's cases cannot break it, and a
   change to P3's wording does. The values were checked by a scratch script (deleted): each
   hand-composed `expected` against Rust within `RTOL`. A perturbed value fails the gate by path
   (tried, then reverted).
8. **For P11**: `run-shot` is dispatched as `Some("run-shot") if args.next().is_none()`. Its doc
   paragraph says "It writes nothing, and it reads no session", and `tests/shot.rs`'s
   `the_verb_takes_no_arguments` pins `run-shot --spec x` as a usage refusal. `--session` changes
   all three. The refusal exit is 1 for an undeclared device or one no profile reads, with nothing
   on stdout.
9. **For P6**: `conformance.py check` exits 0 with `spec/vectors/shot/` present and prints no line
   for it, since `_kind` "shot" is in no family set. P6's deferral is what adds the line.
   `crates/core/Cargo.toml`'s description still names only the `run` and `parse-screen` seams. That
   is not false, and was left alone.
10. **Verified**: `cargo test` (62 suites, all ok; `golf-core` gained the `shot` suite of 8 and three
    unit tests), `cargo clippy --all-targets` (no warnings), `cargo fmt --check`, the v17 dry run
    (0 changed over 42), `pytest tests/test_conformance.py tests/test_docs_truth.py` (343 passed) and
    `conformance.py check` (exit 0). `git diff -- src/ spec/vectors/` is empty: the shot family is
    untracked and new. No dependency was added, nothing under `data/` was written (P4 has no write
    path), and no Python was touched.

### P5 (2026-10-09)

1. **What landed**: nineteen cases in `spec/vectors/shot/`, still flat (P4 finding 4), written by a
   scratch script (deleted) in P4's format. Fifteen are `corpus-<vector>.json`, one per corpus
   vector carrying a shot, which is every corpus vector. Four are `screen-<sub-family>-<name>.json`,
   one per screen read chosen below. Each `provenance.note` names its numbers' source, and
   `provenance.source` says the Rust check was made. Each hand-composed `expected` was checked
   against `golf-core run-shot` within `RTOL` before the gate ran, and the gate then passed.
2. **The corpus cases' shape.** `input` is the vector's request less its clips: `swing_id`,
   `session_id`, `shot`, `intent`, `handedness` and `loft_deg` (null where the vector's is), all
   verbatim, plus `photo_sha256` set to the shot's `image_sha256`. There is **no `club` key**,
   because the vector records none, so `intent` keeps the category `all`, as every `analysis.json`
   does (P4 finding 2). `expected.intent` and `expected.shot` are the vector's
   `expected.swing.intent` and `.shot`. `measurements` are its `launch_monitor:` and
   `model:flight_v1` rows, and `unscored` is its `flight_*` tail (empty on `2026-08-10-1` and `-2`).
   `notes` is `[]`. All values are Python's.
3. **One photo on three swings, in real data**: `2026-08-07-aaron1-1`, `2026-08-09-2` and
   `2026-08-10-1` hold the same stored shot (`2026-08-10-1`, photo `2e81ab05…`), and their cases
   differ in `swing_id` and `session_id` alone. **For P9 and P10**: this is the real form of call 6's
   one-photo-two-clips shape. No photo-only copy of the photo exists, so nothing moves today. If
   one did, call 6 makes it a `duplicate` of the earliest of the three.
4. **The screen reads chosen** come from a census of every screen read through `run-shot`, each
   right-handed at its corpus twin's loft, or 30.5 where it has no twin:
   - `screen-corpus-2026-08-23-1` covers both reasons on one real read: `""` gives `printed_blank`,
     and the tied Impact Position pair gives `misread`.
   - `screen-corpus-2026-08-23-3` is the one `misread` from text, `'Open'`.
   - Both bay reads hold the stored parse's analysed values, except `impact_position`, which no row
     reads. So their rows and flight refusal are their corpus twin's, Python's values.
   - `screen-hand-carry-twice` is **the one flight outcome no corpus case reaches**. The tie rule
     withholds Carry, so `flight_carry_yds` is `no_launch_conditions` ("no spin printed, and no
     carry to solve one from either"). Its tiles cross three table rows: a tie, `'---'` and `""`.
     `synthetic/duplicate-label` reaches the same outcome and was not taken.
   - `screen-synthetic-spin-axis-disagrees` is the one case with **ADR-014's needs-review note**,
     the sentence no corpus vector reaches (P2 finding 4). Eight synthetic reads reach it. This one
     also has a printed axis beside a `---` spin.
   - The two cases with no corpus twin hold no Python value. Their rows are the printed numbers,
     and their arithmetic is in the note: face − path, and carry × sin(start line) to four places.
5. **Unreached, and not invented**: a solved spin flown on a printed axis, which would give
   `flight_spin_rpm` and `flight_landing_offline_yds` together. Every read with a printed axis and a
   blank spin is a synthetic screen whose 128.1 yd carry is past the 125.92 yd peak of its launch
   conditions. Lofts 9 to 25 were tried, and each gave `carry_unreachable`. Every other screen read's
   flight outcome is one a corpus case already reaches.
6. **Measured for M29**: on `2026-08-10-1`, M34's read of `shot_type` differs from the stored parse
   (`SLIGHT FADE` against `CENTER SLIGHT FADE`). Running the read through `run-shot` in its twin's
   request **moves no number**: every shot and flight row is exactly the corpus vector's. Its
   outcome is reached elsewhere, so it is not a case.
7. **The gate gained two pins**, in `crates/core/tests/shot.rs`, which is outside P5's Files line.
   Without them, the four purposes above would hold only until someone edited a case:
   - the census (`the_cases_reach_every_row_of_the_tile_table_and_each_named_gap`) now requires
     `no_launch_conditions` and a needs-review note;
   - the new `every_corpus_vector_carrying_a_shot_has_its_case` requires a `corpus-<stem>.json` for
     every corpus vector with a shot, with that vector's `input.shot`, `swing_id` and `session_id`.
   - Both were checked to fail by name with a case moved away, and the cases were then restored. The
     new pin reads the gzipped corpus through `flate2`, already a `golf-core` dependency.
8. **For P6**: the family is 25 files. The `corpus-*` and `screen-corpus-*` rows are Python's bits,
   and the engine re-record writes the same Rust values into a corpus vector's rows and its shot
   case's alike. The new pin compares only inputs and ids, which a re-record never rewrites. The
   v17 dry run with `Family::Shot` should therefore report 0 changed over all 25.
9. **Verified**: `cargo test` (62 suites, all ok; `golf-core`'s `shot` suite is now 9), `cargo
   clippy --all-targets` (no warnings), `cargo fmt --check` (after one `cargo fmt` of the gate), and
   `test_docs_truth.py`. `git diff -- src/ spec/vectors/{synthetic,corpus,stages,screen,audio,format,storage,career}`
   is empty. No dependency was added, nothing under `data/` was written, and no Python was touched.

### P6 (2026-10-09)

1. **What landed**: `rerecord::{SHOT, Family::Shot}` and `walk_shot` in `crates/core/src/rerecord.rs`.
   `plan` runs it after `walk_engine` under `Version::Analysis` and joins the two walks with
   `Walked::extend`, so the typo guard reads both. Each case goes through `ShotInput`'s trait
   `Deserialize` and `run_shot`, then the shared gate, `apply`, `ledger` and writer. Inputs are never
   rewritten, and `provenance.oracle: "hand"` is kept, because `ledger` never overwrites an oracle.
   The report's count line is now `N engine, N stage, N shot`. The module docs (`rerecord`, `lib`,
   `bin/golf_core`) say so. `conformance.py::shot_vector_paths` and `check`'s new line (the shot
   count, "worked by hand at Rust's ANALYSIS_VERSION", ending in "run `cargo test`") are the
   deferral, pinned by
   `tests/test_conformance.py::test_check_defers_the_shot_family_and_judges_no_staleness`.
2. **The unchanged-swing pin**: the v17 dry run reports `67 vectors run (21 engine, 21 stage, 25
   shot); 0 changed; 0 files written`. P5 finding 8's prediction held.
3. **Why `check` judges no staleness on a family that ages on `ANALYSIS_VERSION`**: every case is at
   Rust's 17, above frozen Python's 16, with no ledger, so `ledger_covers` would call all 25 stale.
   The pin asserts that every committed case sits above frozen Python's version, so it keeps
   biting, and it adds a case at 99. The shot gate is what holds the version.
4. **Not in the plan: the writer now keeps each file's indent**, as it already kept line endings.
   Measured over every vector: 32 files are at an indent of 4 (all 25 shot cases and the 7 in
   `screen/hand/`). Every other file is at 2, `storage/hand/` included. Without the change, the first
   engine bump that moved a shot row would rewrite each touched case at 2 and leave the family in
   two layouts. `Layout { crlf, indent }` replaces `Rerecorded.crlf`. The indent is read from the
   second line, and a one-line file gets 2. Unit test: `a_file_keeps_the_indent_it_was_committed_with`.
   **One churn is left, and the module doc already names it**: 24 of the 25 cases hold `\u` escapes
   (the scratch scripts wrote ASCII), and `serde_json` writes the character itself. So a rewritten
   case changes those lines in text but not in value.
5. **No compose check for the shot family, on purpose.** A `corpus-*` case's rows are its corpus
   vector's rows at another spelling: `expected.measurements[i].value` against
   `expected.swing.measurements[j].value`. A declaration that names one and not the other is
   refused at the path it left out, so the gate keeps the two copies in step. `walk_shot`'s doc says
   so. **For a later engine bump** (and P13's WORKLOG): moving a shot row means declaring both
   spellings.
6. **The walk refuses a non-flat family** before reading any case: any entry under `vectors/shot/`
   that is not a plain `.json` file, matching what `tests/shot.rs`' `cases()` would choke on. It also
   refuses a duplicate id, an input `ShotInput` rejects, and a case `run_shot` refuses. Each is
   `Refused::Files`, a broken `spec/`. A `spec/` with no `vectors/shot/` now fails an engine run.
7. **Tests** (`crates/core/tests/rerecord.rs`): `Slice::new` now copies
   `shot/corpus-2026-08-23-11.json` (the slice's corpus vector's case), edited in its own layout by
   `edit_the_shot_case`. `a_dry_run_reports_and_writes_nothing` ages that case beside the synthetic
   pair, so one declaration of the two version paths lands on three files and the count line reads
   `(2 engine, 2 stage, 1 shot)`. The new
   `a_shot_path_declared_lands_there_alone_and_a_second_run_writes_nothing` checks four things: the
   undeclared run is refused at `shot/…: expected.measurements[0].value`; the declared run changes
   only that file, with its ledger, oracle and layout kept; the value is within `RTOL`; and a second
   run writes nothing. The new `a_screen_or_career_run_never_opens_the_shot_family` covers screen and
   career runs that pass with `shot/` unreadable plus a stray directory, while an engine run stops on
   each. The suite is 20 tests.
8. **For P12**: CONFORMANCE §4 needs four additions. The shot family is re-recorded under
   `analysis_version` with the engine's. The run reads three walks. The indent is now a third thing
   matched to the committed file, beside line endings and the deflate stream. And there is `check`'s
   seventh deferral line. The quoted `42 vectors run (21 engine, 21 stage)` lines in older plans
   and in CONFORMANCE (`:166`) are history and stay as written.
9. **For P13**: exit criterion 2 is read off the count line above. Expect `(21 engine, 21 stage, 25
   shot); 0 changed`.
10. **Verified**: `cargo test` (62 suites, all ok), `cargo clippy --all-targets` (no warnings),
    `cargo fmt --check`, pytest (2053 passed), `ruff check src tests scripts`, `conformance.py check`
    (exit 0, with the shot line), and the v17 dry run (0 changed over 67). `git diff -- src/
    spec/vectors/` is empty, and no file under `spec/vectors/shot/` changed. No dependency was added,
    and nothing under `data/` was written. Python changed only in `scripts/conformance.py` and
    `tests/test_conformance.py`. Both are CRLF, and both kept it.

### P7 (2026-10-09)

1. **What landed**: `SwingManifest.intent: Option<PracticeGoal>` (`crates/storage/src/manifest.rs`),
   last, after pydantic's fields, `#[serde(default, skip_serializing_if = "Option::is_none")]`.
   `crates/storage/src/session_meta.rs` is the port of `session_meta.py`, as free functions over a
   session directory like Python's: `SESSION_META_NAME`, `SessionMeta` (with `intent` last, written
   only when set), `session_meta_path`, `load_session_meta` (tolerant, answering
   `SessionMeta::default()`), `save_session_meta` (atomic, creates the directory, writes
   `session.tmp` as Python does), and `set_current_player`, `set_current_club` and
   `set_current_intent(session_dir, value, now)`. All three go through a private `update_cursor`
   closure over the record as loaded, so an unnamed field is carried by construction. In
   `bundle_store.rs`: `Upload.intent: Option<&'a PracticeGoal>` (a borrow, because `Upload` is
   `Copy` and `PracticeGoal` holds a `String`), stamp-if-empty in `place` and `new_manifest` exactly
   as `club` is, and `SwingBundleStore::set_intent(session, swing, Option<PracticeGoal>, now)`
   through the shared `repair`. Every module doc says what it gained and names the frozen-Python
   hazard (finding 3).
2. **Not in the Files line: `crates/core/src/storage_family.rs`'s `Upload` literal gained
   `intent: None`**, the one compile fix the new field forced. **For P9**: read `args.intent`
   there, as `club` is read (`arg::<Option<PracticeGoal>>`), and add the `bundle.set_intent` op
   beside `bundle.set_mishit`. No storage op reaches `session_meta` today, so the cursor ops are
   new names if P9's cases want them (call 8 lists only upload stamping and `set_intent`).
3. **`AssignmentResult` does not report the intent**, because frozen Python's does not and the
   storage family records that shape. **For P11 and M38**: an upload's answer will not say a shot
   was stamped as a drill. Read it back with `get_swing` if a surface needs to show it.
4. **An intent no enum names drops the whole manifest in Rust**, where frozen Python ignores the key
   and reads the swing as tracked. It is consistent with an unknown `club` or `mishit`, and it is
   the conservative answer, since a shot whose intent cannot be read is not counted on a guess.
   Pinned by `an_intent_round_trips_and_one_that_cannot_be_read_drops_the_manifest`. The same holds
   for `session.json`: a bad intent empties every cursor, as pydantic's `ValidationError` does for a
   bad club. **For P10**: `get_session` skips such a manifest silently, as it skips a corrupt one,
   so it is invisible to `read_corpus` and is not a named exclusion. No writer here produces one.
5. **The skip-if-`None` proof**: every recorded storage and career vector passes unchanged, and
   two unit tests re-save a frozen-Python manifest and session file to the JSON value they were
   read from, with no `intent` key. **For P10**: no recorded store write moves on `intent`, so the
   file-key escape is not needed on its account.
6. **The schemas** were edited from a throwaway pydantic subclass of each model with
   `intent: PracticeGoal | None = Field(default=None, description=...)`, so the spelling is
   pydantic's (`anyOf [{$ref PracticeGoal}, {type: null}]`, `default: null`, no `title`). Before the
   edit, both files re-rendered byte for byte in `export_schemas`' format. Both kept CRLF. Each
   gained `$defs` `PracticeGoal`, `PracticeMode`, `TargetShape` and `ClubCategory`, copied from
   `shot_result`'s, and an `intent` description naming who writes it and what frozen Python does
   with it. `session_meta`'s root description now says three cursors. The scratch script is
   deleted.
7. **The pins.** New `crates/storage/tests/schemas.rs` has `RUST_OWNED = ["swing_manifest",
   "session_meta"]`, P1 finding 3's "list of their own". Its `pin` is `python_schemas.rs`' with
   `schemas.rs`' default check added: key set, required, null and each spelled default, over
   `RoleFile`, `SwingManifest`, `SessionMeta` and `PracticeGoal`. It holds every intent shape equal
   to `shot_result`'s copy and checks that `intent`'s description names the hazard.
   `crates/storage/tests/python_schemas.rs` keeps `analysis_state` alone. `conformance.py::RUST_OWNED_SCHEMAS`
   is now the six, with a comment saying why. The new
   `tests/test_conformance.py::test_the_rust_owned_roots_are_the_union_of_the_two_rust_pins` reads
   both Rust `const RUST_OWNED` lists out of the source, spells each set, and holds the constant to
   their union with no overlap. The docs of `crates/contracts/tests/{schemas,python_schemas}.rs`
   were updated to match. P1's `RUST_OWNED_SCHEMAS - SCHEMA_ROOTS == {"shot_result"}` still holds,
   because both storage roots keep their models.
8. **For P11**: `set-intent --swing N` is `SwingBundleStore::set_intent`, and `--default` is
   `session_meta::set_current_intent(&sessions_dir.join(S), …)`. Neither refuses anything: call 5's
   shape rules are the verb's, and the store records the goal as given, `club` included, so the
   verb should build `PracticeGoal { mode, target_shape, ..PracticeGoal::default() }`.
   `save_session_meta` creates the session directory, as Python's does, so a mistyped `--session`
   with `--default` would create one. The verb should refuse a session directory that does not
   exist before writing. `set_intent` already answers `None` for a missing swing.
9. **For P12**: CONFORMANCE §1 is now false in four places. The heading "Three roots are Rust's"
   and "writes the other seven" (`:82`, `:88`) should say six Rust-owned, of which five are in
   `SCHEMA_ROOTS`, and five Python-written: `analysis_state`, `audio_file`, `bag`, `golfer` and
   `keypoints_file`. Next, "`swing_manifest`, `golfer`, `bag` and `analysis_state` stay
   Python-exported" (`:104`). Last, "Two lists … nothing checks one against the other" (`:121`):
   there are three lists now, and the Python pin checks them. It should name
   `crates/storage/tests/schemas.rs`. ARCHITECTURE §4 and `data/README.md` need `intent` in
   `manifest.json` and `session.json` (already on P12's list), and `CLAUDE.md`'s crate paragraph
   needs `session_meta`.
10. **Verified**: `cargo test` (63 suites, all ok: P6's 62 plus `storage/tests/schemas.rs`),
    `cargo clippy --all-targets` (no warnings), `cargo fmt --check`, pytest (2054 passed: P6's
    2053 plus the new pin; the 2 warnings are the sandbox denying `.pytest_cache`), `ruff check src
    tests scripts`, and `conformance.py check` (exit 0). `git diff -- src/ spec/vectors/` is empty.
    The `data/` digest is still `bd384cc4…` over 192 files. No dependency was added. Python changed
    only in `scripts/conformance.py` and `tests/test_conformance.py`, and both kept CRLF.

### P8 (2026-10-09)

1. **What landed in `crates/contracts/src/career.rs`**: `CorpusSwing.face_on_sha256: Option<String>`
   (`#[serde(default)]`, written as `null` when `None`, never skipped, as `shot_sha256` is);
   `CorpusSwing.intent: Option<PracticeGoal>`, last, `skip_serializing_if = "Option::is_none"`, and
   `Validate` now recurses into it; `ExclusionReason::Untracked` (`"untracked"`), appended to `ALL`
   (now seven), with a doc saying it is not in `swings`, as a duplicate is not; and `artifact_key`'s
   fourth `None`, a `pose:` row on an entry with no clip (`self.face_on_sha256.as_deref()?`). Three new
   unit tests: the pose-row `None` beside the photo-keyed shot and model rows, the shape a swing with a
   clip and no intent writes (a bare string, no `intent` key) against a photo entry's `null`, and an
   intent round trip.
2. **`read_corpus` already carries the intent.** `corpus_swing` sets `intent: manifest.intent.clone()`
   off the survivor's manifest, as `club` is, and wraps the group's clip hash in `Some`. No recorded
   manifest holds an intent, so no answer moved. **For P9**: a tracked hand case whose manifest
   records a goal expects that goal on its corpus swing. **For P10**: you do not add it. What is left
   is the photo-entry path, where `corpus_swing` still takes `face_on_sha256: &str`; make it
   `Option<&str>`, or give the photo entry its own builder.
3. **Outside the Files line, forced by the type change**: the `CorpusSwing` literals in the tests of
   `crates/analysis/src/{baseline,club_profile,comparison,dispersion}.rs` (wrapped in `Some`, plus
   `intent: None`); `crates/storage/src/corpus.rs` (finding 2); and `crates/core/src/reports/corpus.rs`,
   whose "Collapsed N re-upload(s)" line now prints `photo <prefix>` for an entry with no clip and
   `face_on <prefix>` as before for every swing. No vector reaches the new arm. **For P10 and P12**: it
   is the only report text a photo entry can change, so a hand case or a career pin with two photo
   entries on one photo will record it.
4. **`state.rs` gained the `shot_analysis.json` half**: `SHOT_ANALYSIS_NAME`, `shot_analysis_path`,
   `load_shot_analysis(swing_dir) -> Option<ShotResult>` (typed and tolerant: missing, not JSON, not
   an object, a missing `shot`, a broken bound, an unknown reason, `NaN`, a `true` or `17.0` version
   all read as `None`), `save_shot_analysis(result, swing_dir) -> Result<(), StoreError>`, and one
   not in the plan, `shot_analysis_matches(result, manifest) -> bool`, the photo entry's
   `AnalysisState::matches`. It sits beside that function so the stale rule has one home. **For P10**:
   - stale is `!shot_analysis_matches(..)`, false when the result names no photo **or** the manifest
     has none;
   - outdated is `result.analysis_version < versions.comparable_from`, read off the struct. Do not use
     `is_older_than` or `stored_analysis_version`, which take a raw `analysis.json` map;
   - a version that is not an integer makes the whole file unreadable, so the entry is `not_analyzed`
     where the same value in an `analysis.json` is `outdated`. The module doc says why, and a hand case
     could pin it;
   - `shot_needs_review` is `result.shot.provenance.is_some_and(|p| p.needs_review)`. The swing
     path's `needs_review` (`corpus.rs:440`) takes a raw `Value` and reads a shot that fails to
     validate as flagged. A typed `ShotResult` has already validated its shot, so that branch cannot
     arise, and the struct read is the same rule.
5. **`save_shot_analysis` validates first, as `BagStore::save` does**, so an invalid result is
   `StoreError::Invalid` and nothing is written. **It does not create the swing directory**, where
   `save_state` does as Python does: a result with no manifest beside it is a directory every reader
   skips. A missing directory is `StoreError::Io`. Writing the same result twice writes the same bytes,
   pinned, and the `.tmp` is renamed away. **For P11**: `run-shot --session`'s "a re-run writes the same
   bytes" rests on this, and the verb should name a missing swing before it reaches the writer.
6. **`crates/storage/src/flight_inputs.rs`** ports `LoftGap` (all four variants with Python's wire
   names, `ALL`, `as_str`), `loft_for_club`, and the private `declared_lofts` and `resolve_loft`, branch
   for branch. **The return is `Result<f64, LoftGap>`, not Python's `(loft, gap)` pair**: Python's
   docstring promises the gap is `None` exactly when the loft is not, and in Rust the type enforces
   that. The signature is `loft_for_club(player_id: Option<&str>, club: Option<ClubId>, golfers_dir:
   Option<&Path>)`. An empty `player_id` is no golfer, Python's truthiness, and an unreadable bag is
   no bag. `LoftGap::NoSwing` is kept and produced nowhere, documented as the corpus-wide join's. That
   join (`read_flight_inputs`, `FlightInputs`, `_swings_by_shot_photo`) is named as not ported, with
   its three Python callers. Six tests: Python's three `loft_for_club` cases, plus the club tag asked
   first, no registry or an empty id or a corrupt bag each being `NoBagEntry`, and the wire names.
   **For P11**: `.ok()` is the request's `loft_deg`, and `.err()` picks the note. `loft_remedy`'s three
   sentences (`api/pipeline.py:692`) name pages and say "this swing", so P11 writes the shot's own.
7. **Verified**: `cargo test` (63 suites, all ok; 14 new unit tests and no new suite), `cargo clippy
   --all-targets` (no warnings), `cargo fmt --check`, and `pytest tests/test_docs_truth.py` (92
   passed). Every recorded storage and career vector passes unchanged. The storage family compares
   each corpus as it serializes, so that is the proof that the clip still writes as a string and
   `intent` writes no key. `git diff -- src/ spec/vectors/` is empty. No dependency was added, and no
   Python file changed.

### P9 (2026-10-10)

1. **Resumed.** The first attempt (marker 2026-10-09 16:48) was cut off after writing eleven cases
   and the runner's two additions, before any gate change, and it left `cargo test` red (finding 3).
   Kept after review: all eleven cases, each re-worked against call 6 and then held by the two new
   tests (finding 9), and `storage_family.rs`' `args.intent` and `bundle.set_intent`. Done here: the
   skip in both places, the tests, one case added (finding 4), two cases' photo hashes lengthened
   (finding 5), and the runner's doc.
2. **What landed**: twelve cases under `spec/vectors/storage/hand/` at `career_version` 2,
   `oracle: "hand"`, `worked_against` naming the runner, each `provenance.note` showing its working,
   in M36 P14's layout (CRLF, indent 2, sorted keys, `\u` escapes):
   - corpus: `photo-entry-pooled`, `photo-entries-one-photo`, `photo-only-copy-of-a-swing`,
     `one-photo-two-swings-and-a-copy`, `mixed-session`, `drills-untracked`,
     `drill-among-five-carries`, `photo-entry-repairs` (not analysed, stale by a wrong and by a
     missing `photo_sha256`, outdated, on the line, a `16.0` version, a flagged read, stale and
     outdated), `no-face-on-and-no-photo`, and `excluded-order-across-kinds` (finding 4);
   - op: `upload-stamps-intent` (four uploads: a stamp, a second upload not restamping, a `null`
     cursor stamping nothing, an empty intent taking the cursor's) and `set-intent-repairs` (sets,
     overrides, clears, a missing swing answers `null`).
3. **The skip is by name in three places, not one** (outside the Files line). Two more tests walk
   the whole storage family. `crates/core/tests/rerecord.rs`' career tests copy it and run a career
   declaration at `CAREER_VERSION` over the copy, so the first attempt's cases failed five of them
   (`an_empty_career_declaration_finds_nothing_and_writes_nothing` and four more), measured on the
   resumed tree. `crates/contracts/tests/career.rs::every_vector_in_both_families_is_at_this_career_version`
   failed on all twelve. The one list is `crates/core/tests/common/mod.rs::P10_PASSES_THESE`, read
   by `crates/core/tests/storage.rs` (the gate skips and prints the names), by
   `crates/core/tests/rerecord.rs` (`Slice::copy_the_career_families` copies both families and
   removes the named cases; it replaced two inline copies), and by the contracts pin, which includes
   the file by `#[path]` as `mod waiting_on_p10`. **For P10**: in the change that bumps
   `CAREER_VERSION`, delete `crates/core/tests/common/mod.rs`, both `mod common;` lines, the
   contracts pin's `waiting_on_p10` module and its skip, the skip in `every_storage_vector_conforms`,
   the removal loop in `copy_the_career_families` (keep the method), the paragraph each module doc
   gained, and `the_cases_p10_passes_are_worked_and_wait_on_nothing_else`. Until then `golf-core rerecord
   --declare spec/declarations/career-v1.json` would refuse on the committed tree, since each new case
   is a difference and a `career_version` move; nothing runs it, and P10 runs `career-v2`.
4. **Two orders call 6 left open, decided and pinned** by `excluded-order-across-kinds`, added beyond
   call 8's list because no listed case could tell either apart:
   - **untracked is read off the survivor's manifest before any artifact**, so an untracked survivor
     is named once, `untracked`, and never also `not_analyzed`, `stale` or `outdated`. A drill is not
     work waiting on a pipeline run.
   - **`excluded` holds every clip group's exclusions before every photo group's**, each kind in
     first-scan order. A photo entry's cross-kind dedupe needs every surviving swing first, so the
     photo groups are walked after. `drills-untracked` gives the same list under one walk over both
     kinds in first-scan order; the new case does not.

   **For P10**, the order the module doc's "What is read where" should state: the unattributed
   (scan order), then `no_face_on` (scan order), then each clip group (its duplicates by arrival,
   then its survivor's `untracked`, or its `not_analyzed`, `stale`, `outdated`), then each photo group
   (its duplicates by arrival, then its survivor's cross-kind `duplicate`, or its `untracked`, or its
   three). A photo survivor that duplicates a swing is named once, as a duplicate, and never reaches
   the untracked test, because the dedupes come first (call 6). No case gives such a copy an intent
   of its own: `drills-untracked`'s 5 has none.
5. **A photo hash in a sentence is cut to 12 characters**, as the clip's is (`chars().take(12)`, and
   the report's `prefix`). Pinned by giving `photo-entries-one-photo` and `photo-only-copy-of-a-swing`
   real SHA-256 photo hashes (of the texts `photo-15-1` and `photo-17-1`). Every other hand hash is
   shorter than 12 and pins nothing.
6. **The sentences, for P10 to copy.** Each is verbatim in at least one case. `{sha}` is the 12-cut,
   `{ref}` is `session/swing`, and the dash is U+2014:
   - `no_face_on`: "neither a face-on clip nor a shot photo, and every checkpoint is measured from the
     one and every shot number read from the other";
   - a photo entry's `duplicate` of a photo entry: "shot photo {sha} is already in {ref} — the same
     shot uploaded again, not a second shot";
   - a photo entry's `duplicate` of a swing: "shot photo {sha} is already in {ref}, beside that
     swing's video — one ball flight, so the copy with the video is the one kept";
   - `untracked`: "hit as a drill, which feeds no club or player stat and no mechanics baseline —
     clear its intent with golf-core set-intent if it should count";
   - a photo entry's `not_analyzed`: "no shot_analysis.json — run golf-core run-shot over its session";
   - a photo entry's `stale`: "shot_analysis.json names another shot photo than this one, or none, so
     the stored numbers may describe a photo that is no longer here — re-run golf-core run-shot over
     its session";
   - a photo entry's `outdated`: the swing's sentence with "a shot analyzed today" for "a swing
     analyzed today" and "Re-run golf-core run-shot over its session" for "Re-run
     scripts/reanalyze.py".

   A swing's four sentences are unchanged. **For P11 and P12**: recorded text now names
   `golf-core run-shot` "over its session" and `golf-core set-intent`.
7. **What the cases decide beyond call 6's words**, each said in its note:
   - a photo survivor lists its photo group's duplicates in `duplicates`, so `duplicates_collapsed`
     counts them and the report prints them as `photo` lines (P8 finding 3). A photo entry made a
     duplicate of a swing is **not** added to that swing's `duplicates`, which lists re-uploads of
     its clip (the field's doc), so it is in `excluded` alone;
   - a re-upload collapsed into an untracked survivor is in no `duplicates` list, since that survivor
     is in no `swings` (`drills-untracked`: `duplicates_collapsed` 0);
   - the cross-kind dedupe looks at every surviving swing, an untracked one included, so a drill's
     photo cannot re-enter the stats through a photo-only copy (`drills-untracked`'s 5);
   - a photo entry's `club` and `intent` are its survivor manifest's, never `ShotResult.intent`'s
     (`photo-entry-pooled`: the manifest's `club: all`, not the result's `mid_iron`);
   - a photo entry's `conflicting_shots` is `[]`, since a photo group shares its photo by definition;
   - `drill-among-five-carries` shows "the mishit median sees only the tracked four" as **no floor**,
     because four is below `MISHIT_MIN_CLEAN_SHOTS`, with the floor the drill would have made in its
     note. `mixed-session` is where a floor is taken over both kinds at once: photo entry 4, carry 70,
     is the automatic mishit.
8. **The runner** (`crates/core/src/storage_family.rs`) keeps the first attempt's `args.intent` and
   `bundle.set_intent`, and its module doc gained a section saying both are Rust's alone, with no
   `_op_table` twin, called only by hand cases. The unknown-op panic says so too. `corpus_answer` is
   now `pub`, for finding 9's second test. One module-doc sentence had been false since P7 ("the first
   change that does (M35's manifest key) is the one that builds the escape"). It now says the manifest
   key moves no recorded write.
9. **Two new tests in `crates/core/tests/storage.rs`**:
   - `the_cases_p10_passes_are_worked_and_wait_on_nothing_else` (P10 deletes it). Every named case
     exists and is hand-worked at `CAREER_VERSION + 1` with a note. **Both op cases conform today**,
     through P7's store and this phase's runner. Every corpus case's manifests read as
     `SwingManifest`, and every `shot_analysis.json` reads as a `ShotResult` (by
     `load_shot_analysis`' own `serde_json` read) except `photo-entry-repairs`' `2026-08-24/6`, which
     must not. **Today's reader gives none of the ten corpus answers**, so each one pins the change.
   - `every_hand_corpus_case_counts_what_its_own_swings_hold` (permanent). A hand corpus case's
     `expected.corpus` round-trips through `CareerCorpus`, and its derived counts and nine properties
     equal `corpus_answer` over `narrowed_to` of its own swings, for each narrowing too. It holds on all
     13 hand corpus cases, M36's three included. It cannot check `auto_mishit` or which entries are
     listed. Those are the gate's, against P10's reader.

   Negative controls, each reverted byte for byte: a `metric_counts` slip and a `distinct_shots` slip
   in `mixed-session` fail the second test by name, and a wrong `target_shape` in
   `set-intent-repairs` fails the first.
10. **For P10**: `no-face-on-and-no-photo`'s swing 2 is finding 5's recorded `storage/corpus/no-face-on`
    swing 1 under the new rule, so the dry run should move that case as worked here. The op cases need
    no reader change and pass once the version is 2.
11. **Verified**: `cargo test` (63 suites, all ok, 1130 tests; the gate prints 28 op and 36 corpus
    cases conforming and 12 skipped by name), `cargo clippy --all-targets` (no warnings),
    `cargo fmt --check`. Python was not touched, but the family it counts grew, so
    `pytest tests/test_conformance.py tests/test_docs_truth.py` (345 passed) and `conformance.py
    check` (exit 0, 76 storage vectors) were run too. `git diff -- src/ spec/vectors/` is empty: under
    `spec/vectors/` only the twelve new `storage/hand/` files exist, untracked. No dependency was
    added. `data/` was not read or written, and P9 is not a digest phase. The scratch scripts were in
    the session scratchpad and are deleted.

### P10 (2026-10-10)

1. **What landed in `crates/storage/src/corpus.rs`**: `read_corpus` sorts each attributed manifest
   into a clip group (a face-on clip), a photo group (no clip, a shot photo) or `no_face_on`
   (neither), each group in first-scan order (a small `Groups` type replaces the inline pair). Clip
   groups are walked first: duplicates, then the survivor's photo noted for the cross-kind dedupe
   (every survivor's, a drill's included, the earliest by `arrival` kept), then `untracked` or the
   swing's artifacts. Photo groups follow: duplicates (the photo sentence), then a `duplicate` of
   the swing holding the photo, else `untracked`, else the photo entry's artifacts. `corpus_swing`
   is now `entry` (the shared skeleton, `face_on_sha256: Option<&str>`) plus `read_swing_artifacts`
   (unchanged rule) and `read_photo_artifacts` (`load_shot_analysis`, `shot_analysis_matches` for
   stale, `analysis_version < comparable_from` for outdated, the stored shot's own `needs_review`,
   P8 finding 4 to the letter). The two `outdated` sentences share one `outdated(…)` function, and
   the 12-character cut is `cut`. Every sentence is P9 finding 6's, verbatim. The module doc gained
   "Two kinds of entry, and the drill that counts nothing", and its "What is read where" states P9
   finding 4's order.
2. **`CAREER_VERSION` 1 → 2** with its ledger entry (`crates/contracts/src/career.rs`). The docs of
   `NoFaceOn`, `Duplicate`, `NotAnalyzed`, `Stale` and `Untracked`, and of `CorpusSwing.duplicates`
   and `.analyzed`, now say what each means for a photo entry. `crates/storage/src/lib.rs`'s
   `corpus` line says the reader admits photo entries and leaves drills out.
3. **P9's skip is gone, as P9 finding 3 listed**: `crates/core/tests/common/` deleted; `mod common;`
   removed from `tests/storage.rs` and `tests/rerecord.rs`; the contracts pin's `waiting_on_p10`
   module and its skip; the gate's skip and its "skipped by name" line; the removal loop in
   `copy_the_career_families` (kept as a method); each module doc's paragraph; and
   `the_cases_p10_passes_are_worked_and_wait_on_nothing_else`, with the four imports only it used.
   `every_hand_corpus_case_counts_what_its_own_swings_hold` stays.
4. **Finding 5's move is a `duplicate`, not a pooled photo entry.** In the recorded
   `storage/corpus/no-face-on`, swing 1 (down-the-line clip plus photo `shot-hash`) is a photo entry,
   but swing 3 holds the same `shot-hash` beside its face-on clip, so call 6 makes 1 a duplicate of 3
   (decision 4). Swing 2 (down-the-line alone) stays `no_face_on` under the new sentence and now
   comes first, since photo groups are walked after clip groups. So `excluded[0]` and `[1]` trade
   `swing_id`, and `[1]`'s reason and sentence move. Nothing pooled moves. The adopted copy
   `career/synthetic/storage-no-face-on` moves the same five `input.corpus` paths. **Not foreseen
   by the plan**: both its `career_corpus` reports move too. The plain one prints the exclusions by
   reason (`duplicate 1` and `no_face_on 1` where it printed `no_face_on 2`), and the verbose one
   prints each sentence. That is this rule reaching the report, as at M36 P15, and it is declared.
5. **The re-record.** `spec/declarations/career-v2.json` adds nothing and moves 13 paths:
   `career_version`, five `expected.corpus.excluded[…]` paths, the same five under `input.corpus`,
   and `expected.reports.career_corpus` with its `_verbose` form. The dry run listed only those,
   plus `career_version` on every vector at 1. The real run wrote 120 files. A second run, and a
   dry run after every later change, print *"nothing differs from the committed vectors / 132
   vectors run (76 storage, 56 career); 0 changed; 0 files written"*. An audit by a scratch script,
   since deleted, compared every changed vector with `HEAD` minus `career_version` and
   `provenance.rerecords`: only the two files above differ in value. Plain files kept their line
   endings and both real files stayed gzipped. **One byte-level change is the writer's, not the
   rule's**: M36's three `hand/` cases (`versions-either-side`, `comparable-from-sixteen`,
   `mishit-sees-only-comparable`), now re-recorded once with a `rerecords` entry, had their `—`
   escapes written as raw UTF-8 `—`. That is the same value, and the spelling every Python-recorded
   storage vector already uses. P9's twelve cases were at 2 already and were not rewritten.
6. **Call 11's two pins, worded exactly.** A drill on disk is also **one more directory seen**, so
   neither pin can be plain equality:
   - `crates/storage/src/corpus.rs::a_corpus_with_a_drill_is_the_corpus_without_it_plus_one_untracked_exclusion`
     covers one drill of each kind, beside a tracked swing and a tracked photo entry. The corpus with
     the drill, less its one `untracked` entry and with `swing_dirs_seen` minus one, equals the corpus
     with the drill's directory deleted.
   - `crates/core/tests/career.rs::a_drill_moves_no_aggregate_and_no_report_but_the_corpus_counter`
     uses `drill-among-five-carries`' swing 5 and `drills-untracked`' photo entry 2, read through
     `run_storage` with the drill's files dropped from `input.files`. Every aggregate and
     `properties` are equal, and so is every report but `career_corpus`. That one report (plain and
     verbose) prints the directory count and the exclusions, so it must differ, and rendered from
     the accounted corpus it is the deleted tree's to the character. Neither drill's session
     empties, so `sessions_scanned` is unmoved.

   Negative control: `is_tracked` forced true fails both by name. Reverted.
7. **The measurement**: `golf-core career-corpus --name Aaron --json` over `data/` gives 15
   sessions, 15 directories, 13 distinct swings, 13 shots, 2 duplicates, 0 outdated and 34 metrics
   with n > 0. That is M36 P14 finding 4's `{17, 14}` row exactly. There are no photo entries, no
   intent and no exclusion but the 2 duplicates. The `data/` digest is `bd384cc4…` over 192 files,
   before and after.
8. **For P11**: a photo entry pools only when its `shot_analysis.json` reads as a `ShotResult`,
   names the manifest's photo in `photo_sha256`, and is at `analysis_version >= COMPARABLE_FROM`.
   `run-shot --session` must write all three, or the "then `read_corpus` pools the entry" test will
   see `stale` or `not_analyzed`. A photo entry whose photo a surviving swing also holds is a
   `duplicate` whatever its `shot_analysis.json` says. A verb that writes one for it gets a result
   the corpus never reads, so the verb should skip it rather than write one.
9. **For P12**: `docs/ARCHITECTURE.md:240` and `docs/CONFORMANCE.md:990` print `rerecord --declare
   spec/declarations/career-v1.json`, which `golf-core` now refuses ("at career_version 1, and this
   many-shot layer is at 2"). Make them `career-v2.json`. CONFORMANCE §2's storage/career section
   (`:681–700`) and §4's runs list (`:1133`) need the second run (finding 5) and the report move
   (finding 4). The `NoFaceOn` sentence changed for every manifest with neither role. ADR-024's
   addendum should state the two dedupe keys' cross-kind rule and that a photo-entry duplicate of a
   swing is in no `duplicates` list.
10. **Verified**: `cargo test` (63 suites, 1131 tests, all ok: P9's 1130, less the deleted
    `the_cases_p10_passes_…`, plus the two pins; the gate prints 30 op and 46 corpus cases
    conforming, none skipped), `cargo clippy --all-targets` (no warnings), `cargo fmt --check`. The
    family changed, so `pytest tests/test_conformance.py tests/test_docs_truth.py` (345 passed) and
    `conformance.py check` (exit 0, 76 storage and 56 career vectors) ran too. `git diff -- src/`
    and `git diff -- spec/vectors/{synthetic,corpus,stages,screen,audio,format}` are empty. No
    dependency was added and no Python file changed. The scratch files are deleted.

### P11 (2026-10-10)

1. **What landed, and where the Files line moved.** The verbs' logic is a new module,
   `crates/core/src/session.rs` (no Python twin), rather than more of `lib.rs`. That follows
   `flag-mishit`'s precedent, where `reports::flag_mishit` holds the `main` and the binary parses
   flags. It holds `ShotDirs` (sessions, golfers, shots), `run_shot_session`, `IntentTarget`,
   `IntentAction`, `intent_action` (the flags' decision and call 5's shape rules) and `set_intent`.
   Both verbs answer `reports::Answer`. `lib.rs` gained the module, a "verbs that write into a
   session" doc section, and two doc pointers that said "(P11)". `bin/golf_core.rs` gained the
   dispatch (`Some("run-shot")` with no arguments is still the stdin seam, and any argument selects
   `--session`), `parse_shot_session`, `parse_set_intent`, two usage lines, the `run-shot`
   paragraph's second mode, and a `# set-intent` section naming the hazard. **Also outside the
   Files line**: `DataDirs::resolve`'s closure became `reports::data_dir(flag, var, leaf, env)`, so
   the "flag, else `GOLF_*_DIR`, else `data/processed/<leaf>`" rule has one definition across the
   three directories (`GOLF_SHOTS_DIR` is `settings.shots_dir`'s name). In `tests/shot.rs`,
   `the_verb_takes_no_arguments` became `a_flag_the_session_mode_does_not_take_is_a_usage_refusal`
   (exit 2 now, as argparse's), which is P4 finding 8's third change.
2. **The duplicate skip (P10 finding 8) asks `read_corpus` itself**, once per golfer, and skips a
   photo entry that its `excluded` names as `duplicate`, printing the corpus's own sentence. A second
   copy of the two dedupes would be a second thing to drift. It covers both kinds: the photo-only
   copy of a swing's photo, and the later of two photo entries holding one photo, whose result no
   reader reads either. Duplicates are decided off manifests alone, so the verb's own writes cannot
   move the answer mid-walk. **An entry naming no golfer is in no corpus, so it is analysed.**
   `--swing N` naming a duplicate prints the skip line and exits 0.
3. **Both gap notes are gated on the engine's reason**, `no_handedness` and `no_club_loft`. The loft
   gate is `api/pipeline.py`'s (its ⚠️ comment). **The handedness gate is new**: a swing always needs
   a handedness (`head_stays_back`), so Python appends that note unconditionally, but a shot needs
   one only when its screen printed a spin axis (P4 finding 5). An ungated note would tell a golfer
   to repair what nothing used. The wording, `session.rs`' `handedness_for` and `loft_note`: "no
   golfer is attributed to this shot" / "golfer 'x' is not in the registry" (`pyfmt::str_repr`, as
   Python's `!r`), each followed by "so this shot's landing offline could not be computed: the
   screen's spin axis reads '+ = fade', and which way a fade curves depends on which side the golfer
   swings from". For the loft, three sentences, each "this shot's ball flight could not be
   simulated" plus the repair, ending "then re-run golf-core run-shot over its session". They name
   no page. `LoftGap::NoSwing` has no note, as `remedy.get` has none. A unit test holds every
   sentence to "this shot" and to no page, "this swing" or `head_stays_back`.
4. **The lines and the exits.** `run-shot --session` prints one line per manifest on stdout:
   `S/N: wrote shot_analysis.json (<mode>[, a drill that no club or player stat counts]): n
   measurement(s), m unscored, k note(s)`, or `S/N: skipped: <why>` for a swing, a manifest with no
   photo, a duplicate, or a photo with no stored parse. The no-parse line names the store and the
   frozen import that would parse it: `python scripts/import_shot_screens.py <photo path> --session S
   --out <shots dir>`. An empty session prints `S: no swings`. A failed entry (an undeclared device, an
   unreadable stored parse, a failed write) goes to stderr as `S/N: not analysed: …`, the walk goes
   on, and the exit is 1. Exit 2, with nothing written, for a session that does not exist, and for a
   `--swing` naming no manifest, a swing with a face-on clip, or a manifest with no photo. Skips exit
   0. The result is written to `manifest.session_id / manifest.swing_id`, where the corpus reads it.
5. **`set-intent` takes `--default --clear`**, which call 10 did not list ("`--clear` for one
   swing"). Without it, a session cursor once set could only be pointed at another mode, never
   cleared. `--mode` and `--shape` are read through serde, so only the wire names parse (`Drill` is
   refused), and no list of modes is written into a sentence. Refused with exit 2 and the usage:
   `--shape` without `shot_shaping` or `shot_shaping` without `--shape` (call 5), `--clear` with
   `--mode` or `--shape`, neither or both of `--swing` and `--default`, no `--session`, a swing or
   session that does not exist. `--default` refuses a missing session directory before
   `save_session_meta` could create one (P7 finding 8). The goal written is `PracticeGoal { mode,
   target_shape, ..default }`, with the club left at `all`. The clock is `Timestamp::now_utc()` in
   the binary alone.
6. **The frozen-Python check (a scratch tree, deleted).** `golf-core set-intent` wrote a manifest
   (`shot_shaping`, `draw`) and a `session.json` (`drill`). Frozen Python's `load_manifest` and
   `load_session_meta` read both without error, ignoring `intent`, and its `save_manifest` of the
   loaded manifest wrote the file back **with no `intent` key**. That is finding 3's hazard, now
   measured. The Rust tests also hold the written mode to `swing_manifest.schema.json`'s
   `PracticeMode` members.
7. **The tests**: `crates/core/tests/verbs.rs` (new, 6 tests), each on a scratch copy of a P9 hand
   case with its `shot_analysis.json` files removed, plus golfer `aaron`, his bag from
   `stores/bag-reads` (7i at 34°) and a shot store filled from two shot cases. `no-loft`'s shot
   (`2026-08-23-4`) needs a loft and never a handedness, and `no-handedness`'s (`2026-08-10-1`) the
   reverse. So one walk over `mixed-session` shows each note appended (7: no golfer, axis printed;
   8: `brian` unregistered and bagless, no axis) and each left off. Every written result equals
   `run_shot` over what the stores resolve, with those notes appended. A re-run writes the same bytes
   and prints the same lines. `read_corpus` then pools 2 and 4 with the written measurements, and
   names 6 (no parse) `not_analyzed`. The duplicate test reads the sentence out of
   `photo-only-copy-of-a-swing` and `photo-entries-one-photo`'s recorded `excluded`. The drill test
   runs both verbs through the binary, with the shot store found through `GOLF_SHOTS_DIR`. Three unit
   tests in `session.rs` cover the note wording, the flag decision and the goal's description.
   Negative control: the handedness gate forced open fails the walk test by name. Reverted.
8. **For P12**:
   - `CLAUDE.md`'s commands block and ARCHITECTURE §1 should list `run-shot --session S [--swing
     N]`, with the three directory flags, and `set-intent`. §1's verb count moves: `golf-core` now
     has `run`, `rerecord`, `parse-screen`, `run-shot`, `set-intent`, the four career readers and
     `flag-mishit`. Re-count it from `bin/golf_core.rs`' usage rather than from here.
   - `data/README.md` and ARCHITECTURE §4 should say `shot_analysis.json` is written by `golf-core
     run-shot --session`, beside the manifest, for a photo entry only, and that `GOLF_SHOTS_DIR` is
     read.
   - The handedness gate (finding 3) belongs in ADR-034's addendum, beside the photo-side reasons,
     and the measured hazard (finding 6) in ADR-024's.
   - CLAUDE.md's crate paragraph should name `golf_core::session`.
9. **For P13** (the WORKLOG's M29 paragraph): `run-shot --session` can analyse only a photo frozen
   Python's import already parsed, and its no-parse line names `scripts/import_shot_screens.py`,
   which §M29 deletes. When §M29's `ort` reader lands, that branch becomes a Rust read and the
   sentence changes. Neither verb was run on `data/`.
10. **Verified**: `cargo test` (64 suites, 1140 tests, all ok: P10's 1131 plus `verbs`' 6 and
    `session`'s 3), `cargo clippy --all-targets` (no warnings), `cargo fmt --check`. The `data/`
    digest is `bd384cc4…` over 192 files. `git diff -- src/` and `git diff --
    spec/vectors/{synthetic,corpus,stages,screen,audio,format}` are empty, and nothing under
    `spec/vectors/` changed. No dependency was added and no Python file changed. The scratch tree is
    deleted.

### P12 (2026-10-10)

1. **What landed**, docs only (no file under `crates/`, `src/`, `scripts/`, `tests/` or `spec/vectors/`
   changed):
   - `docs/CONFORMANCE.md`: the banner; §1 rewritten for six Rust-owned schemas, `shot_result` the one
     with no model, the storage pin, and the three lists the Python pin holds together (P1 finding 8,
     P7 finding 9); §2's ninth family, a new **Shot** section, the ledger and declaration paragraphs,
     the storage table at version 2 with M35's twelve hand cases and the runner's two Rust-only ops,
     the career section's "How they came to be at version 2" with call 11's two pins; §4's intro and
     command block (`career-v2.json`, the `run-shot` line), the shot family under the engine
     declaration and its two-spellings rule, the flat-family refusal, the indent rule (P6 finding 8),
     M35's run as a fourth worked example, the shot deferral in `check`, `regenerate` having no shot
     family, a `run-shot` seam section, §5's storage row (`session_meta`, `flight_inputs`), and one
     "does not cover" bullet for the writing verbs. The file-key escape sentence now says neither M36
     nor M35 moved a recorded write, so it is still unbuilt (P10 built none).
   - `docs/ARCHITECTURE.md`: §1's career comment (the second rule that differs), a block for
     `run-shot --session`, `set-intent` and `run-shot` on stdin, the Rust paragraph (M35's modules,
     the shot family under `cargo test`), and `career-v2.json` (P10 finding 9); §2's `ShotResult` row;
     §3's new last subsection, "One `analyze_shot` call", with the filled club category (P4 finding 2)
     and every mode reaching the engine; §4's rows for the manifest's `intent`, `shot_analysis.json`,
     the third cursor and `GOLF_SHOTS_DIR`, and a corpus paragraph for the two kinds of entry.
   - `data/README.md` and `spec/README.md` for the same files, families, schemas and declarations.
   - ADR addenda: **ADR-009** (every mode scores mechanics, the mode routes the stats, the
     single-swing policies retired, the filled club category, frozen Python still raising),
     **ADR-024** (photo entries, the two dedupes and their two consequences from P9 finding 7,
     `untracked`, intent on the manifest and the cursor, the hazard measured at P11, the re-record,
     the handedness and loft routes), **ADR-034** (the three Consequences M35 owned, with P11
     finding 3's handedness gate; clause 2's refusals with decision 3's table). `docs/README.md`:
     the total 102 → 105, the three rows' counts (2, 6, 2) and summaries, and the CONFORMANCE,
     ARCHITECTURE and data README rows.
   - `CLAUDE.md`: the opening's built pieces, the commands block (`run-shot`, and the engine
     declaration writing the shot vectors), the `crates/storage` paragraph, the "how a port is
     checked" row, and the crate row (`shot_result`, `is_tracked`, `analysed_on`/`analysed_view`,
     `analyze_shot`, `policy_for`, `run_shot`, `golf_core::session`, `screen::tile_refusals`,
     `session_meta`, `flight_inputs`, `state`'s shot-analysis half).
   - The program plan's §M35: a pointer line to this plan, and *As built* / *Decided* / *Found*
     notes on the six lines the findings made false (the filter is a view, both verbs, the policy
     decision, the handedness route, the drop-on-rewrite hazard, the pin and test locations).
2. **ADR-014 and ADR-035 needed no addendum.** Each M35-facing clause still holds as built: ADR-014's
   M34 addendum says the withheld tile is "the structural mark that M35 and M37 grade `misread`",
   which M35 now does; ADR-035 clause 4 says the photo-side reasons are safe while they stay in
   `shot_analysis.json`, which is where M35 writes them, and its "a Python re-write loses keys without
   a word" is the general form of the intent hazard.
3. **ADR-034's Status says "Nothing here is built", false since M32.** The Status body was left as
   written (the ADRs amend by addendum); the M35 addendum names it and lists what M32, M34, M36 and
   M35 built, and `docs/README.md`'s row now says "Built in part since M32, and wired to nothing yet".
4. **A finding corrected while writing**: P4 finding 4 says the shot cases were written "as the
   screen family's hand cases are: … LF". Measured: `screen/hand/` is CRLF at indent 4, the shot cases
   LF at indent 4, every other family CRLF at indent 2. CONFORMANCE §2 and §4 state the measured form.
5. **Counts written into prose are snapshots**, as the plan's rules allow: CONFORMANCE §2's headings
   now say Storage 76 and Shot 25 (from `conformance.py check`'s own lines), and the M35 run's 13
   moved paths and 120 files written are quoted from P10 finding 5. The schema pin's test count was
   taken out of §1's prose rather than updated.
6. **For P13**:
   - `docs/README.md`'s document count was not touched; P13 moves it when this plan is staged.
   - `crates/core/src/bin/golf_core.rs`' career-verbs doc says a corpus is read "(`career_version`
     1)". It names the version that introduced the comparable-from rule, so it is not false, but a
     reader may take it for the current version. Left alone, as crates are not P12's.
   - `crates/core/Cargo.toml`'s description still names only the `run` and `parse-screen` seams (P4
     finding 9). Also left.
   - `docs/ARCHITECTURE.md`'s banner still says "reviewed 2026-09-27". Nothing pins it.
7. **Verified**: `pytest tests/test_docs_truth.py` (92 passed); the full `pytest` (2054 passed, the
   one warning the sandbox denying `.pytest_cache`; about 16 minutes, so run it in the background);
   every backticked path and every markdown link in the added lines resolves, except
   shorthands for sub-families and vector ids, and `crates/storage/tests/corpus.rs`, which the
   program plan's note names on purpose as not existing. Line endings kept per file (CRLF where the
   file was CRLF). `git diff -- src/ spec/vectors/` is unchanged by this phase, and `data/` was not
   read or written.

### P13 (2026-10-10)

1. **Resumed.** An earlier attempt, started 13:16, was cut off after it had moved the ROADMAP status
   table's M35, M37 and M41 rows, the NEXT ACTION paragraph and "Last Updated", and
   `docs/README.md`'s document count (81 → 82, `plans/` 7 → 8, now that this file is staged). Each
   was read against this section and kept as written. Nothing under `crates/`, `src/`, `scripts/`,
   `tests/` or `spec/` had changed after the marker, and WORKLOG had not been touched.
2. **What landed**: ROADMAP §M35's Status (✅ with the date, pointing at the three addenda and
   CONFORMANCE §2) and §M37's (⬜ and next, no longer blocked). One more ROADMAP line was false: the
   M31 note on Milestone 3's checklist called the policy question "§M35's open question", and now
   says it was answered there. The program plan's M35 row is ✅. WORKLOG's top entry, with what M37,
   M29 and M38 should know and what is left open. The `app-transition-decisions` memory (M35 done,
   next M37, M41 free to start) and its `MEMORY.md` line.
3. **The exit, item by item**:
   1. `crates/analysis/tests/shot.rs::the_shot_alone_produces_every_committed_shot_and_flight_row`
      is decision 12's pin, green.
   2. The v17 dry run: *"67 vectors run (21 engine, 21 stage, 25 shot); 0 changed; 0 files
      written"*, the count P6 finding 9 expected. `engine.rs`'
      `every_mode_scores_what_fundamentals_scores_and_records_its_own_mode` is green.
   3. `screen::tile_refusals` has one caller, `golf_core::run_shot`, which builds a `ShotResult`;
      `analyze_swing` never sees it. `crates/contracts/tests/schemas.rs::the_swing_schemas_admit_every_reason_but_the_photo_side_two`
      holds the swing schemas to the closed set.
   4. 25 cases under `spec/vectors/shot/`, gated by `crates/core/tests/shot.rs` and re-recorded under
      the v17 declaration (item 2).
   5. The `career-v2` dry run: *"132 vectors run (76 storage, 56 career); 0 changed; 0 files
      written"*. The hand cases pass in `crates/core/tests/storage.rs`.
   6. Call 11's two pins (P10 finding 6) and
      `crates/core/tests/verbs.rs::a_drill_set_by_set_intent_is_analysed_by_run_shot_and_absent_from_the_corpus`
      are green.
   7. `crates/core/tests/verbs.rs` (P11 finding 7), on scratch copies.
   8. `git diff -- src/` is 0 lines, `git diff -- spec/vectors/{synthetic,corpus,stages,screen,audio,format}`
      is 0 lines with no untracked file there, and the `data/` digest is
      `bd384cc40b236b73e3013b817ae79908d6e5ff4dded25b8803fc35c6b7af01cc` over 192 files, P1's.
   9. Finding 4, below.
4. **Verified**: `cargo test` (64 suites, 1,140 passed, 1 ignored, the existing `clip::Cutter`
   doc-test), `cargo clippy --all-targets` (no warnings), `cargo fmt --check`, `pytest` (2,054
   passed; see below), `ruff check src tests scripts`, `mypy src` (118 files), `conformance.py check` (exit 0:
   21/21 engine vectors hold the freeze, and the storage (76), career (56) and shot (25) families
   deferred to `cargo test`), and `pytest tests/test_docs_truth.py` after the last doc edit. Line
   endings kept per file: WORKLOG, ROADMAP and `docs/README.md` CRLF, the two plans LF.
   **pytest needed a second run.** The first, run beside `cargo test`, failed one test,
   `tests/api/test_worker.py::test_failure_is_recorded_and_the_consumer_survives`, on its wait
   timeout (the swing still `running`). M35 touches neither that test nor anything it imports. The
   file then passed 3 runs of 3 alone, and the full suite passed (2,054) with nothing beside it.
   That test's wait is load-sensitive, so run the full suite on its own.
5. **Left for later, not done here** (P12 finding 6, carried into WORKLOG's "Left open"):
   `golf_core.rs`' "(`career_version` 1)", `crates/core/Cargo.toml`'s description, and
   ARCHITECTURE's banner date. P13's steps do not touch crates, so neither line was edited.
