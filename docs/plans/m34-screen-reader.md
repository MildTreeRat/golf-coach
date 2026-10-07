# Plan: M34 — The screen reader in Rust

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-10-01. **Milestone**: [ROADMAP §M34](../../ROADMAP.md#m34-the-screen-reader-in-rust).
**The design is the program plan's
[§M34](m31-m40-shot-first-pivot.md#m34--the-screen-reader-in-rust)**, re-detailed Rust-only by M31.5
P6b, with the M31.5 plan's
[carried decisions 1–4](m31-5-rust-first-replan.md#decisions-carried-to-m32-and-m34-from-the-m32-interview-2026-09-30).
This file is its phase list. It does not restate that section: each phase names the subsection it
builds, and reads it. Where this file and §M34 disagree, this file's interview decisions and "Calls"
say why, and this file wins.
**Governing decisions**: [ADR-035](../decisions/035-rust-everywhere-python-where-required.md) clauses 3
(the oracle moves to Rust: record once from frozen Python, then hand-worked) and 4 (the frozen lab);
[ADR-014](../decisions/014-screen-capture-shot-ingestion.md) and its M31 and M31.5 addenda (the parser,
and the `profiles.json` question this milestone answers); [ADR-032](../decisions/032-the-rust-core.md)
§3 and §5 (portability edges; one copy on disk); ADR-008 (as cargo edges); ADR-010 §2 (no value beats
a wrong one, which is the tie rule's whole argument).

**Level: L3.** A new subsystem (`crates/screen`), a crate split (`crates/pyfmt`), a new vector family,
and an extension of `golf-core rerecord`. Every phase reads its own list below rather than the repo.

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot — the findings are what the next phase
   is planned against;
3. stop, and tell the user to `/clear` and run `/next-phase`.

**Nothing is committed until P12** (the user's call, 2026-10-01). Every phase leaves its work in the
working tree, and P12 makes one commit onto `main`. **All thirteen are done** *(2026-10-02)*, and
M34 is closed.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document, and the pointers to it | ✅ Done *(2026-10-01)* — approved by the user |
| **P1** | `crates/pyfmt`: the split out of `analysis` | ✅ Done *(2026-10-01)* |
| **P2** | The parser's CPython edges: format tables and `pyfmt` functions | ✅ Done *(2026-10-01)* |
| **P3** | `crates/screen` I: the crate, the forked profile, `difflib` and the matcher | ✅ Done *(2026-10-01)* |
| **P4** | The recorder: the screen family, recorded once from frozen Python | ✅ Done *(2026-10-01)* |
| **P5** | `crates/screen` II: the faithful parser | ✅ Done *(2026-10-01)* |
| **P6** | `crates/screen` III: validate, the shot, and `golf-core parse-screen` | ✅ Done *(2026-10-01)* |
| **P7** | `golf-core rerecord` gains the screen family | ✅ Done *(2026-10-01)* — one question for the user before P10 (P7 finding 3) |
| **P8** | The change I: the tie rule, the V tile, `fields_present` and the stamp | ✅ Done *(2026-10-02)* — P10 still waits on P7 finding 3 |
| **P9** | The change II: validate ranges, the golfer-facing filter, the `hd_golf` pin | ✅ Done *(2026-10-02)* — P10 still waits on P7 finding 3 |
| **P10** | The 13-shot re-read: declaration, re-record, and the faithful port deleted | ✅ Done *(2026-10-02)* — with the user's option (a), `removed` for screen declarations |
| **P11** | Docs: CONFORMANCE, ADR-014 and ADR-032 addenda, ARCHITECTURE §1, `CLAUDE.md` | ✅ Done *(2026-10-02)* |
| **P12** | Close: verify, WORKLOG, ROADMAP, memory, and the one commit | ✅ Done *(2026-10-02)* — M34 closed |

---

## Rules every phase keeps

- **Every phase ends green on everything it can reach.** If a phase cannot end green, stop and
  record why as a finding; do not carry a red suite into the next session. This is why the faithful
  rules survive behind `screen::frozen` from P8 to P10 (decision 2): the Python-recorded vectors keep
  gating the faithful port while the hand-worked ones gate the change.
- **Frozen Python is untouched outside `scripts/conformance.py`, `scripts/conformance_vectors.py`
  and `tests/test_conformance.py`** (ADR-035 clause 4). `src/golf_coach/launch_monitor/screen/` —
  `profiles.json` included — does not change. `git diff -- src/` is empty at every phase.
- **Nothing under `data/` is written.** `data/processed/` is gitignored, so `git status` cannot prove
  it: P4 (the only phase that *reads* `data/`) snapshots the sha256 of every file under
  `data/processed/shots/` and every `data/processed/sessions/*/*/manifest.json` to the scratchpad
  before it runs and compares after. P10 and P12 repeat the comparison against P4's snapshot, which
  P4 also copies into its findings as a count and a combined digest.
- **Only two things write `spec/vectors/`**: `conformance.py regenerate --format-only` (P2, the format
  family, which records CPython) and `regenerate --screen-once` (P4, once), and from P10 only
  `golf-core rerecord`. Any other change under `spec/vectors/` is a finding and a stop. The engine
  and stage families do not change in this milestone at all.
- **New code is Rust.** The Python changes are conformance tooling only (P2, P4, and P11's comment).
- **House style** (`CLAUDE.md` §House style, `docs/CODE_STANDARDS.md`): comments say *why* and carry
  the measurement or the rejected alternative. Match the module you are in; `pyfmt`'s and
  `rerecord.rs`'s module docs are the register to match.
- **Do not write counts into prose** that a test or a registry owns. Where this plan quotes a count
  (13 photos, 15 bundles, 0.0002), it is a snapshot for the phase to re-measure, not a value to copy.
- **The doc-count pin reads `git ls-files`**, so this file is invisible to it until P12 stages it.
  Do not touch `docs/README.md`'s document count before P12. This file also avoids the
  `**Status: … N/M phases` spelling on purpose: `tests/test_docs_truth.py::_PHASE_STATUS` would then
  demand a `docs/README.md` row, and plans are not listed there.
- **Line endings**: the plain `.json` vectors are CRLF (Python's `write_text` on Windows) and
  `rerecord`'s writer keeps each file's own. Git Bash heredocs mangle `—` and `°`; use the Edit
  tool or Windows Python for in-place edits.

## Verify — the commands, and when each runs

```bash
cargo test                                          # every phase that touches crates/
cargo clippy --all-targets && cargo fmt --check     # every phase that touches crates/
.venv/Scripts/python.exe -m pytest                  # P2, P4, P11, P12 (and any phase touching Python or docs)
.venv/Scripts/python.exe -m ruff check src tests scripts   # P2, P4, P12
.venv/Scripts/python.exe -m mypy src                # P12 (M34 changes nothing under src/)
.venv/Scripts/python.exe scripts/conformance.py check      # P2, P4, P12
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py # every phase that edits a doc or moves a path
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/screen-v1.json --dry-run   # P10, P12
```

---

## Decisions taken in the interview (2026-10-01)

1. **`profiles.json` is forked.** `crates/screen/profiles.json` is the Rust copy and gains the
   `Impact Position V` tile; `src/golf_coach/launch_monitor/screen/profiles.json` stays frozen without
   it. ADR-032 §5's one copy on disk is given up for this file until M40 deletes the Python one, and
   a pin (`crates/screen/tests/profile_fork.rs`) holds the two equal **except a declared delta**,
   which is the V tile alone. ADR-014 gets the addendum that records the choice (P11).
2. **The faithful port is deleted at the re-record.** It is a gate step and never ships (§M34). From
   P8 it lives behind `screen::frozen`, reading the frozen Python profile, and gates only the
   Python-recorded vectors; P10 re-records them with the shipping parser and deletes it. Git history
   and each vector's `rerecords` ledger carry the evidence, as M32's did. The frozen Python parser
   itself is the alternative that stays runnable (`docs/README.md` §Conventions) until M40.
3. **A field the tie rule withholds is "located, unread".** It is in `fields_present`, has **no
   `raw_fields` entry**, and its value is `None`; a warning names the ambiguity. Every *read* tile has
   a `raw_fields` key (possibly `""`), so "in `fields_present` with no `raw_fields` key" is a
   structural mark that M35/M37 read as `misread` (retaking may help), with no parsing of warning
   text. P8's findings state the rule for M35 in one line.
4. **Validate ranges for the new numeric fields: yes. `ProfileField.scale`: no.** No device prints
   the new fields, but ranges are data and cheap. `scale` is code with no caller; it lands with the
   first profile that needs a unit conversion. The program plan's §M34 and the M31.5 carried decision
   1 list `scale`; P11 records that it was dropped and why.
5. **A tie is a margin, not an equality: `TIE_MARGIN = 0.02` on the assignment total.** Measured in
   the planning read (below): label-damage noise between the two `Impact Position` tiles is
   0.0002–0.004, and real evidence (an exact `Impact Position V`, or `Impact Position Y`) is ≥ 0.066.
   Strict equality would read `2026-08-23-1` on a 0.0002 margin, and on its mirror image would store
   `HEEL` as `impact_position_v` — a wrong value, which ADR-010 §2 ranks below none.
6. **Duplicates are withheld too.** Two boxes that equally well match the one tile a label has (two
   `Carry` boxes) are the same ambiguity — two equally good reads of one field — so the field is not
   read, and both boxes still bound their neighbours' columns. No stored photo has one.
7. **One commit, at the close, straight onto `main`** (P12), as M32 landed.

## What the planning read found (2026-10-01)

Measured on this box. Re-measure before leaning on a number.

1. **Today's OCR reproduces every stored shot exactly.** A scratch run of `prepare_screen` →
   `parse_screen` (+ notes) → `validate_parse(min_confidence=0.6)` over every bundle's shot photo,
   with `paddleocr` 3.7.0 / `paddle` 3.3.1 (PP-OCRv6 medium det/rec), matched each stored shot's
   values, `parse_confidence`, `warnings` and `raw_fields` on all of them, at ~20 s a photo. So P4's
   recorder can **verify against the store** before writing, on `_verify_against_stored`'s
   precedent, and refuse on any difference.
2. **15 bundles carry a shot photo, and 13 distinct photos.** `Aaron-shot-1.png`
   (`2e81ab05828e…`) is the shot photo of `2026-08-07-aaron1/1`, `2026-08-09/2` and `2026-08-10/1`;
   its stored shot is `2026-08-10-1`. The recorder keys on `content_sha256` and names each corpus
   vector by the stored shot's `shot_id`.
3. **The `Impact Position` pair, scored** (`SequenceMatcher` on `normalize_label`; the margin is
   |(A→IP + B→IPV) − (A→IPV + B→IP)|):

   | photo | box A | box B | margin | today | under decision 5 |
   |---|---|---|---|---|---|
   | `2026-08-10-1` | `Impact Position` (V tile, x≈1437) | `Impact Position` (real, x≈488) | 0 | V box wins (first, `>`), `CENTER` spills into Shot Type | both withheld; Shot Type reads `SLIGHT FADE` |
   | `2026-08-23-1` | `Impact Position` (real, `HEEL`) | `ImpactPosition` (V tile) | 0.0002 | real wins on score (1.0 > 0.9655) | both withheld; `HEEL` → `None` |
   | the other 11 bay | `Impact Position` | `Impact Position V` | 0.125 | real wins | resolved; V reads blank |
   | hand: `Impact Position Y` as the V tile | | | 0.066 | | resolved |
   | hand: mirror of `2026-08-23-1` | `Impact Position` (V tile) | `ImpactPosition` (real) | 0.0002 | V tile's blank read as IP | both withheld (strict equality would store `HEEL` as `impact_position_v`) |
   | hand: `lmpact Position` as the V tile | | | 0.004 | | withheld |

4. **Erratum to M31 P2 finding 2.** On shot 1's HEIC (the stored `2026-08-23-1`) the V tile's label
   reads **`ImpactPosition`** (V and space dropped), not plain `Impact Position`; the two boxes do not
   tie at 1.0. They differ by 0.0002 on the assignment total, which is why decision 5 exists.
   Carried decision 2's prediction for this shot (`HEEL` becomes `None`) holds only under a margin.
5. **The recording interpreter is CPython 3.13.3**, and since 3.12 `sum()` over floats is
   **compensated** (Neumaier): `sum([0.1]*10) == 1.0` where a left-to-right fold gives
   `0.9999999999999999`. `parser._score` takes `sum(ocr) / len(ocr)`, which reaches
   `round(conf, 3)` and the `< min_confidence` bool. So `pyfmt` needs CPython's `sum` (P2).
   `crates/analysis/src/scoring.rs::mean_percent`'s doc says Python's `sum()` is left-to-right; that
   is false on the interpreter that recorded the vectors. Its numbers sit inside `RTOL`, so P2 fixes
   the comment only and routes the code question as a finding.
6. **Non-ASCII reaches the parser**: `°` on every photo and `ē` on six. `str.upper()`, `str.split()`
   and `re`'s `\s` are Unicode-aware in Python, and Python's whitespace set includes `\x1c`–`\x1f`,
   which Rust's `char::is_whitespace` does not; `\d` matches every Unicode decimal digit and
   `float()` accepts them. P2 and P4 table each.
7. **`_label_ratio` lives in `preprocess.py`**, not the parser, and divides by
   `len(profile.stored_fields)`, so the V tile moves it on every photo (14 → 15). It is `orient.rs`
   (§M34) and each corpus vector records it.
8. **`import_screen` prepends the preprocessing notes to `warnings` before validating**
   (`screen outline not found - parsing the photo uncropped`, the rotation note, the `{ratio:.0%}`
   legibility note). A screen vector's input therefore carries `notes` beside its boxes, and the
   phone passes its own (VisionKit's) or none.
9. **`feedback::rules` re-implements `pyfmt::percent`** because it could not import `analysis`
   (`crates/feedback/src/rules.rs:33–38`, `:548`). With `pyfmt` its own crate, that reason is gone.
10. **`data/processed/` is gitignored** (`.gitignore:15`), so "nothing under `data/` changed" needs a
    hash snapshot (Rules).
11. **`Impact Position V` is on both 2026-08-10 photos**, not only the 2026-08-23 session: all 13
    stored shots are the bay layout, and only the two reference photos carry `Bounce & Roll`.

## Calls this plan makes where §M34 left it open

Each is contestable before the phase that builds it; after that, a finding is the way to change it.

1. **The screen vector's shape.**
   - `input`: `{device, boxes: [{text, x, y, width, height, confidence}], notes: [str],
     shot_id, session_id, timestamp, image_sha256, image_path, min_confidence}`. `image_path` is
     repo-relative with `/` (the store holds absolute Windows paths; a vector must not).
   - `expected`: `{label_ratio, parsed: {values, raw_fields, confidence, warnings}, shot}` — `parsed`
     is `ParsedShot` *before* validation (confidence unrounded, warnings without the notes), and
     `shot` is the `ShotData` `import_screen` would write, or `null` where it returns `failed`.
     Splitting them is what lets P5 gate the parser before P6 has a validator.
   - `provenance`: `{kind: "screen", oracle: "python" | "hand", screen_parser_version, note, source}`,
     plus `python_version` and, on photo-derived vectors, `paddleocr_version`.
   - Plain `.json`, not gzipped: ~45 boxes a photo.
2. **Where each table lives.** CPython's own behaviour (string `repr`, float `//`, `upper`/`split`/
   `\s`, compensated `sum`, `difflib.SequenceMatcher.ratio`) joins the **format family**, recorded by
   `regenerate --format-only` and ageing on `python_version`, because it is the language's and
   rebuilds anywhere. Each format vector names the crate that implements it: `pyfmt` for all but
   `difflib_ratio`, which is `crates/screen`'s (§M34 puts `difflib.rs` there). The frozen
   *parser's* private functions (`normalize_label`, `_first_number` with its `_THOUSANDS`
   look-behind, `_sign_from`, `_is_blank`, `field_for`'s `>=`, `_Cell.text`'s ordering) are a
   **`units`** sub-family of the screen family, recorded once with it. Each units case carries the
   profile it ran against as input, so the V tile cannot move it.
3. **Recording once: `conformance.py regenerate --screen-once`.** It needs the `ocr` extra and
   `data/`, writes `spec/vectors/screen/{corpus,reference,synthetic,units}/`, and **refuses with exit 2
   when any screen vector is already committed**, naming `golf-core rerecord` — so the command that
   recorded the family is in its provenance and cannot run twice. `build_screen` stays in
   `conformance_vectors.py` and stays runnable (`docs/README.md` §Conventions), on `_audio`'s
   precedent. `check` defers the family (its implementation is `crates/screen`) and applies no
   `ANALYSIS_VERSION` staleness test: it ages on `SCREEN_PARSER_VERSION`.
4. **Synthetic screens come from `tests/launch_monitor/conftest.py::build_screen`**, loaded the way
   `conformance_vectors._make_swing_module` loads the swing conftest, so the fixtures the Python tests
   trust are the ones the port is gated on. One vector per path the parser tests cover (both
   reference screens, blanks, split value boxes, the missing direction word, a printed sign, a
   cropped and a missing title, an unreadable screen, a partial one), each with its `note`.
5. **The seam: `golf-core parse-screen`.** stdin is a screen vector's `input` (or a whole vector);
   stdout is the `ShotData` JSON, or `null` for a failed read. The library entry point is
   `screen::read(&ScreenInput) -> Option<ShotData>`, which M38 calls directly. `crates/core` depends
   on `screen`; `screen` depends on `contracts` and `pyfmt` only.
6. **`golf-core rerecord` picks its family from the declaration's version key.** A declaration
   carries either `analysis_version` (engine and stage families, unchanged from M32) or
   `screen_parser_version` (the screen family only); the guard compares it with `ANALYSIS_VERSION` or
   `SCREEN_PARSER_VERSION` respectively, and a run never touches the other family. The ledger entry on
   a screen vector records `screen_parser_version`. The declaration is
   `spec/declarations/screen-v1.json`.
7. **The faithful gate's one allowance.** Rust's `ShotData` serialises M32's ten keys
   (`spec/declarations/v17.json`'s `added`, re-rooted from `expected.swing.shot` to `expected.shot`)
   and frozen Python's does not. Until P10 re-records them as `added` paths, P6's end-to-end test
   allows exactly those keys, absent from `expected` and at their default in Rust's output — the same
   allowance `crates/contracts/tests/round_trip.rs` carries for `input.shot`. P10 deletes it.
8. **The tie rule, specified.**
   - Candidates are `(box, field)` pairs with `matches ≥ 0.8` (threshold, normalisation and scoring
     unchanged).
   - In each connected component of the candidate graph, enumerate the one-to-one partial
     assignments (a box takes at most one field, a field at most one box) and take the maximum
     total. N is every assignment within `TIE_MARGIN` of it.
   - A field is read only if every member of N gives it the same box. Otherwise it is withheld
     (decision 3).
   - Every box assigned in any member of N is a label box. It bounds its row's columns as a
     boundary-only tile, and it is never value text.
   - A component above a small cap (8 boxes or 8 fields) is refused with a warning rather than
     enumerated; real components are at most 2×2. The phase sets the cap and argues it.
9. **`fields_present` is the located stored fields, in profile tile order**, read ones and withheld
   ones alike. `parser_version` is stamped `SCREEN_PARSER_VERSION` by the shipping parser and never
   by `screen::frozen`.
10. **The golfer-facing filter** is `screen::golfer_warnings(&[String])`, beside the one function
    that writes `no tile found for {label!r}`, both reading one prefix constant (carried decision 3).
11. **`pyfmt` takes `feedback` with it** (finding 9). `feedback` depends on `pyfmt`, calls
    `pyfmt::percent`, and drops its copy and its agreement test. `analysis` gets no re-export: one
    path, not two that drift.
12. **Ranges for the new fields** (decision 4), as P9's starting values, each argued in its comment
    as "catches a dropped or inserted digit": `attack_angle` (−20, 20) °, `dynamic_loft` (−10, 80) °,
    `low_point` (−10, 10) in, `impact_offset_h` and `impact_offset_v` (−60, 60) mm, `carry_offline`
    (−150, 150) yd. `impact_position_v` is text and has none.

---

## Phases

### P0 — This plan document, and the pointers to it

- **Goal**: this file, with the checklist, and the places a session looks for M34's state pointing
  at it.
- **Files**: `docs/plans/m34-screen-reader.md` (new); `ROADMAP.md` (the M34 row in *Status at a
  glance*, and §M34's **Status** line); `docs/plans/m31-m40-shot-first-pivot.md` (the M34 row of its
  status checklist, and a one-line pointer under §M34's heading).
- **Done when**: the edits point here, and docs-truth is green.
- **Verify**: `pytest tests/test_docs_truth.py`.

### P1 — `crates/pyfmt`: the split out of `analysis`

- **Goal**: §M34 "Crate split". `crates/analysis/src/pyfmt.rs` becomes the crate `pyfmt`, so
  `screen` can format like CPython without depending on `analysis` (ADR-008 as a cargo edge). No
  behaviour changes; every number and string is where it was.
- **Read first**: `docs/REFACTOR_LEDGER.md` (no crate-split row as of 2026-10-01 — re-check);
  `crates/analysis/src/pyfmt.rs` (the module doc, `:1–:45`); `crates/analysis/src/lib.rs` (its
  `pyfmt` section); `crates/analysis/tests/format.rs`; `crates/feedback/src/rules.rs:30–40` and
  `:540–560`; the workspace `Cargo.toml`.
- **Files**: `Cargo.toml` (members); `crates/pyfmt/{Cargo.toml, src/lib.rs}` (moved, with its
  history kept by `git mv`); `crates/pyfmt/tests/format.rs` (moved from `analysis`);
  `crates/analysis/{Cargo.toml, src/lib.rs}` and every `crate::pyfmt` call site
  (`grep -rn "pyfmt" crates/analysis`); `crates/core/{Cargo.toml, src/stages.rs}`;
  `crates/feedback/{Cargo.toml, src/rules.rs}` (call 11). Path references outside `crates/`:
  `grep -rn "analysis/src/pyfmt\|analysis::pyfmt" --include=*.md --include=*.py .` — update each to
  the new path in this phase, so docs-truth stays green. The narrative rewrite waits for P11.
- **Done when**: `cargo tree -p feedback` and `cargo tree -p pyfmt` show no `analysis`; the format
  family runs from `crates/pyfmt/tests/`; no file names the old path; everything green.
- **Verify**: `cargo test`; `cargo clippy --all-targets`; `cargo fmt --check`;
  `pytest tests/test_docs_truth.py`.

### P2 — The parser's CPython edges: format tables and `pyfmt` functions

- **Goal**: §M34 step 1's "Format-family tables for every edge in finding 5", for the edges that are
  the *language's* (call 2), plus finding 5 above. Each is recorded from CPython and implemented in
  `pyfmt`, gated **exactly**, before any parser code calls it. This is what P3 and P5 then stand on.
- **Tables** (new vectors in `spec/vectors/format/`, built by `conformance_vectors.build_format`):
  - `str_repr`: `repr(s)` for `str` — quote choice, `\\`, `\n`, `\t`, `\x..`/`\u....` for
    non-printables, printable non-ASCII kept (`°`, `ē`). Inputs: every box text from the planning
    run's 13 photos plus crafted cases.
  - `floor_div`: `int(a // b)` for floats, including quotients that round up to an integer (where
    `(a / b).floor()` differs from CPython's `fmod`-based `//`), negatives, and `b = 1e-6` (the
    `max(…, 1e-6)` in `_Cell.text`).
  - `text_case`: `str.upper()`, `str.split()` and `re.sub(r"\s+", "", s)` over Unicode, including
    `\x1c`–`\x1f`, NBSP, `　`, `ß`, `ē`.
  - `sum`: CPython's `sum()` over float lists (finding 5), at the recording interpreter's version.
  - `difflib_ratio`: `SequenceMatcher(None, a, b).ratio()` over `normalize_label(box text)` ×
    `normalize_label(label or alias)` for every box in the planning run and every label in the
    frozen profile **plus** `Impact Position V`, and crafted cases (repeated characters, `find_longest_match`
    ties, an empty side). Implemented in P3, not here.
- **`pyfmt` gains**: `str_repr`, `floor_div`, `upper`, `split` / `is_space` (Python's whitespace
  set), `strip_space` (the `\s+` removal), `sum`. Each doc carries the measured difference from the
  Rust std call it replaces.
- **Also**: `crates/analysis/src/scoring.rs::mean_percent`'s doc comment is corrected (finding 5).
  The code is unchanged; record the routing question as a finding.
- **Files**: `scripts/conformance_vectors.py` (`build_format` and new table builders);
  `spec/vectors/format/*.json` (new files; **the five existing ones must come back byte-identical**);
  `crates/pyfmt/{src/lib.rs, tests/format.rs}`; `tests/test_conformance.py` (if it pins the format
  family's membership); `crates/analysis/src/scoring.rs` (comment).
- **Done when**: `regenerate --format-only` adds the new vectors and `git diff spec/vectors/format/`
  shows no change to the old five; `cargo test` passes every new table with no tolerance; pytest
  green; `check` green.
- **Verify**: `regenerate --format-only`; `git diff --stat spec/vectors/format/`; `cargo test -p pyfmt`;
  full `cargo test`, clippy, fmt; `pytest`; `ruff`; `conformance.py check`.

### P3 — `crates/screen` I: the crate, the forked profile, `difflib` and the matcher

- **Goal**: §M34 "New `crates/screen`", the half that has no parser in it yet.
  - `TextBox` with `right`/`bottom`/`center_x`/`center_y`, mirroring `recognizer.py`.
  - `profile.rs`: `include_str!("../profiles.json")`. The fork (decision 1) is a byte-identical copy
    of the frozen file in this phase; the V tile lands in P8. It holds serde types for
    `DeviceProfile`/`ProfileField`/`SignRule` with pydantic's defaults (`value_span_ratio` 3.5,
    `blank_markers` `["---"]`, `kind` `"number"`, empty `aliases`/`sign_tokens`). Its functions are
    `stored_fields`, `normalize_label`, `ProfileField::matches` and `DeviceProfile::field_for` (`>=`,
    so the last field wins at equal), plus `load_profile(device)` with frozen Python's error text.
  - `difflib.rs`: `SequenceMatcher.ratio` exactly. That means `find_longest_match`'s tie-breaking,
    `get_matching_blocks`, and `autojunk`, which fires only when `len(b) >= 200`: port it, or refuse
    such input with the argument written down (every `b` here is a short label).
  - `orient.rs`: `label_ratio`, from `preprocess._label_ratio` (finding 7).
  - Pin `profile_fork.rs`: the fork equals the frozen file (read at test time from
    `src/golf_coach/launch_monitor/screen/profiles.json`) except a declared delta, empty for now.
- **Read first**: `src/golf_coach/launch_monitor/screen/{profiles.py, profiles.json, recognizer.py}`;
  `preprocess.py:320–330`; CPython's `difflib.py` (`SequenceMatcher.__chain_b`,
  `find_longest_match`, `get_matching_blocks`, `ratio`); `crates/contracts/src/capability.rs` (its
  `include_str!`, for the pattern).
- **Files**: `Cargo.toml` (members); `crates/screen/{Cargo.toml, profiles.json, src/lib.rs,
  src/profile.rs, src/difflib.rs, src/orient.rs, tests/difflib.rs, tests/profile_fork.rs}`.
- **Done when**: the `difflib_ratio` table passes exactly; `field_for`'s tie and the threshold have
  unit tests; `cargo tree -p screen` shows `contracts` and `pyfmt` only.
- **Verify**: `cargo test`; clippy; fmt.

### P4 — The recorder: the screen family, recorded once from frozen Python

- **Goal**: §M34 "The order" step 1, as calls 1–4 shape it. This is the milestone's one run of
  PaddleOCR, about six minutes on this box.
- **Recorder** (`conformance_vectors.build_screen`):
  - **corpus**: walk `data/processed/sessions/*/*/manifest.json` and take `roles.shot_screen`. Dedupe
    by `content_sha256` (finding 2), and find the stored shot by that sha as
    `api/pipeline.py::_shot_for` does, never through the bulk importer (the M31.5 errata). Then run
    `prepare_screen(load_image(photo), PaddleOCRRecognizer(), load_profile("hd_golf"))`,
    `parse_screen`, the notes prepended, `validate_parse(min_confidence=settings.ocr_min_confidence)`,
    and `to_shot_data` with the stored shot's `shot_id`/`session_id`/`timestamp`, its sha, and a
    repo-relative `image_path`.
  - **The corpus is verified against the store**: the result must equal the stored shot on every key
    but `provenance.image_path`, or nothing is written (finding 1).
  - **reference**: the same chain on `data/raw/shot_screens/IMG_2738.jpeg` and `IMG_2739.jpeg`, read
    directly and stored nowhere, with fixed synthetic identity fields.
  - **synthetic**: call 4.
  - **units**: call 2's parser-private functions over crafted tables, each case carrying its
    profile. The tables include a Unicode digit for `_first_number`, `,`-grouped thousands at and off
    the `\d{3}\b` boundary, single-letter sign tokens against `CLOSED`, and every blank marker spaced
    and unspaced.
- **Tooling**:
  - `scripts/conformance.py` gains `screen_vector_paths()`. `check` defers the family with no
    `ANALYSIS_VERSION` test, `list` shows it, and `regenerate --screen-once` refuses once any screen
    vector exists (call 3).
  - `tests/test_conformance.py` pins the deferral and the refusal, on a temporary copy of `spec/`.
- **Files**: `scripts/conformance_vectors.py`; `scripts/conformance.py`; `tests/test_conformance.py`;
  `spec/vectors/screen/{corpus,reference,synthetic,units}/*.json` (new).
- **Done when**: the run wrote the family and the corpus verify passed on every photo. A second
  `--screen-once` exits 2 and writes nothing. pytest, ruff and `check` are green, and the `data/`
  snapshot is unchanged.
- **Verify**: the snapshot before and after; `regenerate --screen-once` twice; `pytest`; `ruff`;
  `conformance.py check`; `conformance.py list`.

### P5 — `crates/screen` II: the faithful parser

- **Goal**: §M34 step 2, the parser half. `parser.rs` reproduces `parse_screen` on every screen
  vector's `expected.parsed`, and `label_ratio` on `expected.label_ratio`; every `units` table passes.
  This is the faithful port, under `docs/CONFORMANCE.md` §3's rules: strings exact, floats within
  `RTOL`.
- **What must be reproduced, not improved**:
  - `_find_labels`: `>`, so the first box wins, with the result in first-seen label order.
  - `_group_rows` and its sort, which is stable in both languages; use a stable sort on the same key.
  - `_build_cells` excludes label boxes by `id()`; in Rust that becomes their index.
  - `_Cell.text`: `pyfmt::floor_div` and the `(int, center_x)` key.
  - `_read_cell`, and the `_is_blank` `\s` strip.
  - `_first_number`: a hand scanner for `[-+]?\d[\d,]*(?:\.\d+)?` plus the `_THOUSANDS` rule, with
    no `regex` crate.
  - `_sign_from`.
  - `_score`: `pyfmt::sum`.
  - Every warning's text: `pyfmt::str_repr`.
- **Read first**: `src/golf_coach/launch_monitor/screen/parser.py` (whole); P2–P4's findings;
  `crates/core/src/compare.rs` (to reuse it in tests rather than write a comparator).
- **Files**: `crates/screen/src/{lib.rs, parser.rs}`; `crates/screen/tests/{common/mod.rs, parse.rs,
  units.rs}`. `screen` gains `golf-core` as a dev-dependency only if `compare` is needed; otherwise
  re-check whether `compare` belongs lower.
- **Done when**: every `expected.parsed` and `expected.label_ratio` passes, along with every units
  table.
- **Verify**: `cargo test`; clippy; fmt.

### P6 — `crates/screen` III: validate, the shot, and `golf-core parse-screen`

- **Goal**: §M34 step 2, completed, and §M34 "Entry point".
  - `validate.rs` is `validate_parse`, faithfully: the identities, the spin-axis/shape check, the
    ranges and the penalties, with messages through `pyfmt::g` and `pyfmt::fixed(_, 3)`.
  - `to_shot_data` uses `pyfmt::round_to(conf, 3)`.
  - `screen::read` composes parse, notes, validate and shot, and returns `None` where `import_screen`
    returns `failed`.
  - `golf-core parse-screen` (call 5).
  - Every screen vector passes end to end on `expected.shot` under call 7's allowance.
  - The faithful port stamps nothing: `parser_version` 0, `fields_present` `None`.
- **Read first**: `validate.py` (whole); `parser.py::to_shot_data`; `importer.py::import_screen`;
  `crates/core/src/bin/golf_core.rs` (the `run` verb, for the pattern);
  `crates/contracts/tests/round_trip.rs` (its allowance, which call 7 mirrors).
- **Files**: `crates/screen/src/{validate.rs, lib.rs}`; `crates/screen/tests/read.rs`;
  `crates/core/{Cargo.toml, src/bin/golf_core.rs}`; `crates/core/tests/parse_screen.rs` (the verb, on
  one vector and on a failed read).
- **Done when**: every Python-recorded screen vector passes end to end, both through the library and
  through the verb.
- **Verify**: `cargo test`; clippy; fmt; `cargo run --release --bin golf-core -- parse-screen <
  spec/vectors/screen/corpus/2026-08-23-2.json`.

### P7 — `golf-core rerecord` gains the screen family

- **Goal**: §M34 step 4's tooling. The verb re-records `spec/vectors/screen/` behind M32's gate,
  family chosen by call 6, running the shipping `screen::read` (still the faithful rules here).
- **Read first**: `crates/core/src/rerecord.rs`: the module doc, `Declaration`, `plan`, `Family`, and
  the ledger. `docs/CONFORMANCE.md` §2's ledger and §4's `rerecord`. `spec/declarations/v17.json`.
- **Find out first, and record it**: how `compare` reports a list whose **length** changed (the
  warnings lists will), and whether a declaration can name it. If the answer is "only at an index",
  P10's declaration cannot be written, and the fix lands here.
- **Files**: `crates/core/src/rerecord.rs`; `crates/core/src/bin/golf_core.rs` (usage text);
  `crates/core/tests/` (the screen run on a temporary copy of `spec/`).
- **Done when**:
  - The unit tests show that a screen declaration touches no engine or stage file, and the reverse.
  - The version guard reads `SCREEN_PARSER_VERSION`.
  - On a temporary copy, a shape-only declaration (call 7's ten keys as `added`) re-records exactly
    those keys, and a second run writes nothing.
  - On the real `spec/`, `--dry-run` with that declaration reports exactly those keys and writes
    nothing.
  - The v17 run still reports `0 changed`.
- **Verify**: `cargo test`; clippy; fmt; `golf-core rerecord --declare spec/declarations/v17.json
  --dry-run`.

### P8 — The change I: the tie rule, the V tile, `fields_present` and the stamp

- **Goal**: §M34 step 3 — everything that changes what a parse *produces*.
  - The tie rule (decisions 3, 5 and 6; call 8), with `TIE_MARGIN` beside it carrying the measurement
    from finding 3.
  - The fork gains `Impact Position V` → `impact_position_v`, kind `text`, and `profile_fork.rs`
    declares it as the one delta.
  - `fields_present` and the `parser_version` stamp (call 9).
  - `screen::read` becomes the shipping parser. The faithful rules move behind `screen::frozen`,
    which `include_str!`s the frozen Python profile and is used **only** by the Python-recorded gate
    until P10.
- **Hand-worked vectors** (`spec/vectors/screen/hand/`, `oracle: "hand"`, `screen_parser_version: 1`).
  Each `note` works its scores and sums by hand:
  1. exact `Impact Position` beside exact `Impact Position V`: resolved, V blank;
  2. `Impact Position Y` as the V tile: resolved;
  3. the dropped V, two exact `Impact Position` boxes (`2026-08-10-1`'s shape): both withheld, and
     Shot Type no longer spills;
  4. `2026-08-23-1`'s shape: both withheld;
  5. its mirror: both withheld (the case strict equality gets wrong);
  6. two `Carry` boxes: Carry withheld, its neighbours still bounded;
  7. the reference layout: IP resolves, and `no tile found for 'Impact Position V'`.
- **Read first**: §M34 step 3; the M31.5 carried decision 2; finding 3 above;
  `crates/contracts/src/shot.rs` (`fields_present`, `SCREEN_PARSER_VERSION`).
- **Files**: `crates/screen/{profiles.json, src/parser.rs, src/frozen.rs (new), src/lib.rs,
  tests/profile_fork.rs, tests/hand.rs}`; `crates/screen/tests/{parse,read}.rs` (now through
  `frozen`); `spec/vectors/screen/hand/*.json` (new, hand-written).
- **Done when**:
  - Every hand vector passes against `screen::read`, and every Python-recorded vector still passes
    against `screen::frozen`.
  - The findings state the `misread` rule for M35 in one line (decision 3).
- **Verify**: `cargo test`; clippy; fmt.

### P9 — The change II: validate ranges, the golfer-facing filter, the `hd_golf` pin

- **Goal**: the rest of §M34 step 3, and §M34 "The capability pin and the golfer-facing filter".
  - Ranges for the new fields go in `validate.rs` (call 12), unit-tested on a synthetic profile.
  - `screen::golfer_warnings` (call 10), tested on every warning the corpus vectors carry.
  - `crates/screen/tests/capability.rs` pins that `devices.json`'s `hd_golf` field set equals the
    fork's non-null targets (a set, not an order).
  - `devices.json`'s `hd_golf` `source` and `impact_position_v` note are rewritten now that the
    profile locates the tile.
  - `SCREEN_PARSER_VERSION`'s ledger entry 1 is rewritten in `shot.rs`: the Rust parser as M34 ships
    it, tie rule and V tile included, with the faithful port unversioned.
- **Read first**: §M34 "The capability pin and the golfer-facing filter";
  `crates/contracts/{devices.json, src/capability.rs, tests/capability.rs}`;
  `crates/contracts/src/shot.rs:132–163`.
- **Files**: `crates/screen/src/{validate.rs, parser.rs, lib.rs}`; `crates/screen/tests/capability.rs`
  (new); `crates/contracts/{devices.json, src/shot.rs}`.
- **Done when**: everything is green, including `crates/contracts/tests/capability.rs` against the
  edited `devices.json`.
- **Verify**: `cargo test`; clippy; fmt.

### P10 — The 13-shot re-read: declaration, re-record, and the faithful port deleted

- **Goal**: §M34 step 4 and §M34 "Exit".
  > **Precondition, from P7 finding 3**: as the gate stands, this declaration cannot be written. The
  > two label-fix shots lose `raw_fields` keys under decision 3, and `golf-core rerecord` never
  > passes a removed key. The user picks the fix before step 1. P7 finding 4 corrects step 1's
  > `raw_fields` wording.
  >
  > **Answered by the user (2026-10-02): option (a).** P10 adds a `removed` list for **screen
  > declarations only**. It works like `added`: each path is named exactly, recorded in the
  > `provenance.rerecords` ledger, and refused if it matches nothing. Engine declarations keep M32's
  > rule that any removed key fails, and frozen Python (`frozen_view`, `ledger_covers`) does not
  > change. Build it in `crates/core/src/rerecord.rs`, with tests, before step 1. The declaration
  > then names the 7 paths in P8's findings under "Removals". Rejected: (b), a `removed` list for
  > both families, which would teach frozen Python a case no engine change has needed; and (c),
  > keeping the rule by giving withheld fields a `raw_fields` entry, which loses the "in
  > `fields_present` but no `raw_fields` key" signal that M35 and M37 read as a misread.
  1. Write `spec/declarations/screen-v1.json`, with `added` for call 7's keys, `fields_present` and
     whatever P8 added to `parsed`, and `moved` for the version, the two label-fix shots' values, and
     the bookkeeping (`label_ratio`, `confidence`, `parse_confidence`, `warnings`, `raw_fields`).
  2. `--dry-run`, and check the report against the exit item by item, writing the per-shot table
     into the findings.
  3. Run it, and run it again (nothing written).
  4. Delete `screen::frozen`, its `include_str!` of the Python profile, and call 7's allowance.
     Every screen test then runs `screen::read`.
- **Exit, as this phase checks it** (carried decision 4, as decisions 3 and 5 make it):
  - Every shot value is identical except on the two label-fix shots. `2026-08-10-1`'s `shot_type`
    goes `CENTER SLIGHT FADE` → `SLIGHT FADE` and `impact_position` stays `None`; `2026-08-23-1`'s
    `impact_position` goes `HEEL` → `None`.
  - Every bookkeeping change is listed and explained. The expected ones: the V tile's
    `no value text under the label` on every bay shot, `parse_confidence` about −0.02, `label_ratio`'s
    new denominator, the tie warnings on the two label-fix shots and their missing `raw_fields`
    entries, and `no tile found for 'Impact Position V'` on the two reference photos. Anything else
    is a finding and a stop.
  - No `needs_review` flips.
  - `fields_present` tells the two layouts apart: the report counts the photos with a V tile
    (expected: all 13 bay) and those with `Bounce & Roll` (the two reference).
  - A second run writes nothing, and the `data/` snapshot is unchanged.
- **Files**: `spec/declarations/screen-v1.json` (new); `spec/vectors/screen/**` (by the verb only);
  `crates/screen/src/{frozen.rs (deleted), lib.rs}`; `crates/screen/tests/*`.
- **Verify**: `golf-core rerecord --declare spec/declarations/screen-v1.json --dry-run`, then without
  `--dry-run`, then again; `cargo test`; clippy; fmt; the `data/` snapshot.

### P11 — Docs

- **Goal**: every doc that the code now contradicts, and §M34's own open questions closed in the
  places that asked them.
- **Files**:
  - `docs/CONFORMANCE.md`:
    - §2: the screen family, its sub-families, its oracle split, and its version key. The format
      family's new tables, each naming its implementing crate.
    - §3: the parser's edges, as their own list beside the engine's five.
    - §4: `--screen-once`, and `rerecord`'s family selection.
    - §5: the tier-2 screen row is done, and "OCR stays Python" goes.
  - `tests/test_conformance.py::_PACKAGE_DATA`: the `profiles.json` comment. It is now the frozen
    parser's copy, forked by `crates/screen` and deleted at M40.
  - ADR-014: an addendum recording the fork (decision 1), the tie rule as decided (decisions 3, 5
    and 6), the erratum in finding 4, and `scale` dropped (decision 4).
  - ADR-032: an addendum for the screen family's first re-record and the crate count.
  - `docs/ARCHITECTURE.md` §1.
  - `CLAUDE.md`: the crate list, the `pyfmt` path, and `parse-screen` in Commands.
  - `spec/README.md`.
  - The program plan's §M34: a pointer to this file's decisions where they changed it (the margin,
    `scale`, located-unread).
- **Done when**: docs-truth is green and `grep` finds no stale `analysis::pyfmt`, "OCR stays Python",
  or "seven crates".
- **Verify**: `pytest`; `pytest tests/test_docs_truth.py`.

### P12 — Close: verify, WORKLOG, ROADMAP, memory, and the one commit

- **Goal**: M34 closed.
- **Steps**:
  1. Run every verify command.
  2. `git diff -- src/` is empty, and the `data/` snapshot is unchanged.
  3. A WORKLOG entry at the top, with a "What M29, M35 and M38 should know" list.
  4. `ROADMAP.md`: the status row, §M34, and the NEXT ACTION block.
  5. The program plan's status checklist.
  6. This file's checklist.
  7. `docs/README.md`'s document count, now that this file is staged.
  8. The memory file `app-transition-decisions.md` (M34 done; next is M36/M35).
  9. Stage, and make one commit with the attribution lines.
- **Verify**: every command in "Verify" above; `git status` clean after the commit.

---

## Phase findings

*(Appended as phases close. A finding contradicting the plan wins.)*

### P1 — `crates/pyfmt` (2026-10-01)

1. **The move is a rename, and nothing else in the module changed.** `git mv` took
   `crates/analysis/src/pyfmt.rs` to `crates/pyfmt/src/lib.rs` and `crates/analysis/tests/format.rs`
   to `crates/pyfmt/tests/format.rs`, both staged as renames (staged, not committed: P12 commits).
   The only edits inside the module are its crate doc. It gained a "# A crate of its own since M34
   P1" section, and `[`crate::flight`]` became plain `` `analysis::flight` ``, because `pyfmt` cannot
   link upward. `format.rs` changed its import and two comments, and its `spec_dir()` still works
   unchanged, since `CARGO_MANIFEST_DIR`'s grandparent is the repo root from either crate.
2. **`cargo tree`**: `pyfmt` has no normal dependencies at all. `serde` and `serde_json` are
   dev-dependencies only, for `tests/format.rs`. `feedback` shows `contracts` and `pyfmt` and no
   `analysis`. `analysis`, `feedback` and `golf-core` each name `pyfmt` directly, and nothing
   re-exports it (call 11). `golf-core` needed its own edge for `stages.rs`'s `round_to`.
3. **`feedback`'s copy is gone, and so is its agreement test, with no coverage lost.**
   `the_percent_fallback_agrees_with_pyfmt` asserted the same nine `(value, want)` pairs as
   `pyfmt`'s own `a_percentage_scales_before_it_rounds`, so the deleted test was a duplicate of a
   table that stays. `tip_for` calls `pyfmt::percent(score, 0)`. The two spellings also agreed off
   the table (`inf` → `inf%` in both).
4. **Inside `analysis`**, every `use crate::pyfmt::…` and `crate::pyfmt::…` is now `pyfmt::…`.
   `mechanics.rs`'s bare `use crate::pyfmt;` was dropped, because an extern crate is already in
   scope by name. `cargo doc` raises no new warning. Its two unresolved links are older than this
   phase and live in `pivot.rs`.
5. **`docs/REFACTOR_LEDGER.md` had no row for the split (re-checked), and it has a `Done` row
   now** (R1, 2026-10-01). P1's file list did not name the ledger. ADR-032's twelfth addendum did:
   "M34 either lands it with a `Done` row or records why it did not." The row belongs with the
   change it records.
6. **Path references updated in this phase**: `docs/CONFORMANCE.md` (the format family's
   implementation, and §3's "one module"), ADR-032 §3's body (the new path, with "inside
   `crates/analysis` until M34 P1" so the history still reads), the `format_vector_paths`
   docstring in `scripts/conformance.py`, and the format block comment in
   `scripts/conformance_vectors.py`. **Left for P11 on purpose**, because each one names the *from*
   path of the move inside a dated record or an instruction:
   - ADR-032's twelfth addendum, its "`pyfmt` moves out of `analysis`" section (`:1296–:1310`):
     "M34 plans to move…", "`crates/feedback` cannot import `analysis::pyfmt`", and "It is planned,
     not decided." P11's addendum should say that it landed and point at the ledger row. P11's "no
     stale `analysis::pyfmt`" grep will hit `:1303` until then.
   - The program plan's §M34 "Crate split" line (`m31-m40-shot-first-pivot.md:925`).
   - `CLAUDE.md` still says "Seven crates" and lists `pyfmt` under `crates/analysis`. The workspace
     has eight now. docs-truth cannot see this, because
     `test_claude_md_routes_only_to_files_that_exist` checks only `.md/.py/.json/.toml` paths.
7. **For P2**: the format gate now lives at `crates/pyfmt/tests/format.rs`. Its
   `every_committed_format_vector_is_run_by_a_test_in_this_file` pins the exact five names, so
   every new table has to be added to that list as well as given a test. `cargo test -p pyfmt`
   runs the unit tests and the format family together.
8. **`Cargo.lock` changed** (the new member) and is tracked, so P12's commit carries it. Each
   edited file kept its own line endings. Git warns that LF will become CRLF on several of them,
   which is `core.autocrlf=true` normalising, not a content change.
9. **Verified green**: `cargo test` (whole workspace), `cargo clippy --all-targets` with no
   warnings, `cargo fmt --check`, `pytest tests/test_docs_truth.py tests/test_conformance.py`, and
   `ruff check src tests scripts`. `git diff -- src/ spec/` is empty.

### P2 — the parser's CPython edges (2026-10-01)

1. **Five tables landed, and the old five came back byte-identical** (`cmp` against a copy taken
   before the first run, not only `git diff`). The new ones are `str_repr`, `floor_div`,
   `text_case` and `sum`, which are `pyfmt`'s, and `difflib_ratio`, which is `crates/screen`'s. As
   recorded they hold 274, 561, 1,098, 149 and 3,364 cases, which is a snapshot. `difflib_ratio`
   is 335 KB, the family's largest file.
2. **`provenance.implemented_by` is on the new five only.** Call 2 says every format vector names
   its crate, and P2's done-when says the old five must not change, so the two cannot both hold.
   Done-when won: the five engine tables lack the key, and its absence means `pyfmt`. Three places
   read it: `tests/format.rs`'s discovery pin, `check`'s summary line ("are `pyfmt`'s and
   `screen`'s"), and a new pytest pin (`test_every_format_vector_names_the_crate_that_runs_it`),
   which also holds every *new* table to naming its crate. P11's CONFORMANCE §2 should say this.
3. **For P3, `difflib_ratio` is committed and nothing runs it yet.** `tests/format.rs`'s discovery
   pin lists it as `("difflib_ratio", "screen")`, so the pin documents the gap rather than hiding
   it. P3's `crates/screen/tests/difflib.rs` runs it on bits, since `ratio` is exact. Its crafted
   cases include three with `len(b) >= 200`. In `("QQQ", "XYZ" + "Q"*197)` autojunk changes the
   answer: 0.0 against 0.0296 without it. In `("AB", "AB"*100)` it does not, because
   `find_longest_match`'s extension step re-grows a match across *popular* characters (it refuses
   only `isjunk` junk). If P3 refuses `len(b) >= 200` instead of porting autojunk, it skips those
   three cases by name, with the argument. A box that normalizes to `""` is not a case, because
   `matches` returns 0.0 before it reaches `difflib`.
4. **The box texts are transcribed, not read.** The format family must rebuild without `data/`,
   so the planning run's 198 distinct texts (its scratch dump of the 15 bundles' boxes) are
   `conformance_vectors._OCR_TEXTS`, with non-ASCII as `\x`/`\u` escapes. P4's corpus vectors
   carry the same boxes with their geometry. The labels, title and blank markers are read from the
   frozen profile through `load_profile`, plus `Impact Position V`
   (`_FORK_ONLY_LABELS`). `conformance_vectors` now imports `launch_monitor.screen.{parser,
   profiles}`, which is base-install safe.
5. **`pyfmt` gained one more function than the plan listed: `strip`.** `_Cell.text` calls
   `b.text.strip()`, which uses the same whitespace set, and `str::trim` differs from it. The full
   list is `str_repr`, `floor_div`, `is_space`, `upper`, `split` (an iterator), `strip`,
   `strip_space` and `sum`, plus a private `is_printable`. `split` returns an iterator, so the text
   tile is `split(&upper(t)).collect::<Vec<_>>().join(" ")`.
6. **The measured differences from the Rust std calls**, each in its function's doc:
   - **Whitespace.** `str.isspace`, `re`'s `\s`, `split()` and `strip()` share one 29-code-point
     set. Rust's `char::is_whitespace` is that set minus U+001C–U+001F, and nothing else differs.
     This was checked over every code point, and `text_case` records both whole sets so the Rust
     test walks all 1,114,112.
   - **`upper`** differs on 27 code points, measured exhaustively. Rust 1.87 is on Unicode 16.0
     and CPython 3.13 on 15.1. Of the 27, 25 were newly assigned in 16.0 and two are older letters
     that gained a capital (`ƛ` U+019B, `ɤ` U+0264). Not corrected, since no launch monitor prints
     them.
   - **`repr`'s printable test** reads `core`'s own table through a `str::escape_debug` probe
     (`"a" + c`, so a combining mark is not escaped as a leading grapheme extender). It agrees with
     CPython on every code point 15.1 had assigned. It differs on exactly the 5,185 that 16.0
     added. `{:?}` itself differs from `repr` on 272 of the 274 table cases.
   - **`//`.** On PaddleOCR's integer-pixel grid (centers on the half-pixel to 2,000, labels 1–80
     px), 300 of 320,080 pairs differ from `floor(a / b)`. `21.0 // 4.2` is `4.0` while
     `21.0 / 4.2` rounds to exactly `5.0`. All 300 are in the table. On 200,000 uniform fractional
     pairs, none differ. **P5 should expect a corpus photo to be able to hit this.**
   - **`sum`.** 82 of the table's 149 lists differ from `iter().sum()`. 80 are compensation. The
     other two are the all-`-0.0` lists, because Rust's `f64` `Sum` starts from `-0.0` and CPython
     from the `int` 0.
7. **Compensation never moves a photo's mean, and it moves the synthetic screens'.** OCR
   confidences are float32 widened to float64. That holds for PaddleOCR's, and for VisionKit's
   `VNConfidence` too. Fifty 24-bit values sum *exactly* in 53 bits, so on the planning run's 15
   photos and on 10,000 seeded photo-sized lists, compensated and left-fold sums are identical.
   `build_screen`'s constant `0.95` is the opposite case: `sum([0.95]*k)` leaves a left fold for 50
   of k in 1–60. So **P4's synthetic vectors, not its corpus ones, are what will gate
   `pyfmt::sum` in `_score`.** The first seeded draw was float32 only and gated nothing (0 of 60
   lists differed). It was replaced before recording by 40 float64 lists, 10 float32 lists and the
   `[0.95]*k` series.
8. **The `scoring.rs` routing question** (finding 5's code half), for the user or M29, not M34:
   - **Scale.** On 4 of the 21 engine vectors (`2026-08-23-1`, `-3`, `-5`, `tempo-too-quick`),
     Python's compensated mean and Rust's left fold part by 1–2 ulp, inside `RTOL`.
   - **Other sites.** The engine has at least eight more `sum()` sites ported as left folds:
     - `measure.rs:173,174,194,769,895,896`;
     - `pivot.rs:398,469`.
   - **The one to look at first.** `pivot.rs:469` (`sum(deltas) >= 0` picks the turn direction)
     is the one that reaches a **branch** rather than a float inside `RTOL`. Nobody has measured
     whether any vector's net turn sits near zero.
   - **What P2 changed.** Comments only, on `mean_percent` and on its test. The test's name,
     `the_mean_sums_in_the_pythons_order`, is now known to be false: it asserts
     `20.000000000000004`, where CPython gives `20.0`. It was kept because the plan said the code
     is unchanged.
9. **NaN.** `repr` writes `nan` for every NaN, so the `sum` table cannot carry a NaN's sign, and
   `inf + -inf` is the sign-bit-set default NaN on x86. `compensated_sum` matches NaN to NaN, and
   says so. `floor_div`'s `as_int` is recorded beside its float, because the parser's key is
   `int(...)`, which forgets `-0.0`.
10. **Small things the next phases inherit.**
    - **Moved loading.** `build_format` now reads the stages family once, for both
      `_engine_floats` and `_format_sum`; `_engine_floats` no longer reads disk itself.
    - **Prose count.** `pyfmt`'s module doc lost its stale "2,681 cases". It had been stale since
      M22 P5 and now just says "every case". `crates/analysis/src/lib.rs:25` still quotes 2,681 as
      M22 P3's dated history, which is for P11 to judge.
    - **Docs that now lag the code.** CONFORMANCE §2 and §3 do not mention the new tables. That
      is P11's; docs-truth does not see it.
11. **Verified green**:
    - `regenerate --format-only` was run three times. The pytest byte-identity pin compares a
      fresh build with disk.
    - `cargo test -p pyfmt` covers 10 format tests. Full `cargo test` passed.
    - `cargo clippy --all-targets` gave 0 warnings, and `cargo fmt --check` passed.
    - Full `pytest` gave 2,017 passed. `ruff check src tests scripts` passed, and `conformance.py
      check` exited 0.
    - `git diff -- src/` is empty. `spec/` changed only by the five new untracked files.

### P3 — `crates/screen` I (2026-10-01)

1. **What landed.** `crates/screen` is a workspace member (`Cargo.toml`, `Cargo.lock`), untracked
   until P12. Its public surface for P5 is:
   - `screen::TextBox`, with `right`, `bottom`, `center_x` and `center_y`. It derives `Deserialize`,
     with `confidence` defaulting to 1.0, so P4's `input.boxes` deserializes straight into it.
   - `screen::profile::{DeviceProfile, ProfileField, SignRule, FieldKind, LABEL_MATCH_THRESHOLD,
     normalize_label, load_profile, UnknownProfile}`.
   - `screen::difflib::ratio(&str, &str) -> f64`.
   - `screen::orient::label_ratio(&[TextBox], &DeviceProfile) -> f64`.

   `profile::parse_profiles(&str)` is `pub(crate)`, so that P8's `frozen` module can parse the
   frozen Python copy through the same shapes. `stored_fields()` returns `Vec<&ProfileField>`.
2. **`cargo tree -p screen` shows `pyfmt`, `serde` and `serde_json`, and no `contracts` yet.** The
   done-when listed `contracts`, but nothing in P3 uses it. An edge that nothing crosses is a claim
   cargo cannot check, so it was not declared. **P6 must add it** (P6's file list does not name
   `crates/screen/Cargo.toml`) when `to_shot_data` first builds a `ShotData`. The manifest's
   comment says so. Nothing of ours other than `pyfmt` is reachable, so ADR-008's edge holds.
3. **`autojunk` is ported, not refused.** It is six lines, and it makes the three long cases pass
   with no skip list. All 3,364 `difflib_ratio` cases pass on bits, in 0.03 s.
   - **Mutation checks**, each run against the table and then reverted:
     - `autojunk` off: 1 case fails.
     - `>=` in the scan's tie-break: 156 fail.
     - The forward extension removed: 1 fails.
     - The **backward** extension removed: **0 fail**. The table does not reach it.
   - **Covering the backward extension.** A unit test (`difflib.rs`) holds CPython 3.13.3's answer
     for `("QAB", "XQAB" + "Q"*196)`: 6/203, with blocks `[(0,1,3), (3,200,0)]`. Without the
     backward extension the answer would be 4/203. That case could join the table the next time the
     format family is legitimately regenerated. P3 does not regenerate it (Rules: P2 is that
     command's phase).
   - **The tie-break example.** It was also measured: `("aaa", "aaba")` is 6/7 in CPython and 4/7
     with `>=`. It is in the module doc and its unit test.
4. **Two places the Rust profile is stricter than pydantic**, both at load time on a compiled-in
   file, and both stated in `profile.rs`'s module doc:
   - `kind` is an enum (`FieldKind`), so a typo fails to parse instead of reading as a number tile.
   - serde does no lax coercion (`"sign": "1"`).

   Every default is pydantic's, and `an_absent_key_takes_pydantics_default` pins them.
5. **The fork pin compares parsed JSON, not bytes.** It compares every key, `_comment`s included.
   With `core.autocrlf=true`, both files check out CRLF on Windows and LF elsewhere, so a byte
   pin would be about the checkout rather than the content. The fork is byte-identical today
   (`cmp`).
   - **The delta.** `DECLARED_DELTA` is a list of `(device, label)` pairs, empty for now. **P8 adds
     `("hd_golf", "Impact Position V")`.**
   - **What `without()` refuses.** It panics if a declared field is missing from the fork or
     appears more than once. The pin separately fails if a declared field is already in the frozen
     copy.
   - **Its own tests.** Both paths are tested now on hand-made JSON, so the removal logic is not
     first exercised in P8.
   - **Where the frozen copy is read from.** The test reads it at test time from `src/`, not by
     `include_str!`, so this crate's build does not depend on a file it does not own.
6. **`load_profile` has no default device.** Python's signature has `"hd_golf"`. The error's
   `Display` is the f-string Python passes to `KeyError` (`e.args[0]`), not `str(KeyError)`, which
   would add a second pair of quotes.
7. **For P5: the box-level tie is not here.** `field_for`'s `>=` (the later field wins) picks a
   field for one box. Which *box* a field gets, when two boxes pick the same field, is
   `_find_labels`'s `>` (the first box wins). The `Impact Position` tie lives there, and P8's tie
   rule lands there. `field_for`'s doc says so, so that nobody "fixes" the tie in the matcher.
8. **`crates/pyfmt/tests/format.rs`**: the discovery pin is unchanged. Its doc comment now says
   `difflib_ratio`'s runner exists, instead of "committed and run by nothing".
9. **For P11**: `CLAUDE.md` still says "Seven crates". The workspace has nine since this phase:
   P1's `pyfmt` and P3's `screen`.
10. **Verified green**:
    - `cargo test` passed on the whole workspace. `screen` has 16 unit tests, 2 in `tests/difflib.rs`
      and 3 in `tests/profile_fork.rs`.
    - `cargo clippy --all-targets` gave 0 warnings, and `cargo fmt --check` passed.
    - `cargo doc -p screen --no-deps` gave 0 warnings. A bare `[M34]` tag had resolved as an
      intra-doc link, so the crate doc's tag is `[M34 P3]`.
    - `git diff -- src/ spec/` is empty. `spec/`'s only changes are still P2's five untracked
      files.

### P4 — the recorder (2026-10-01)

1. **What landed.**
   - **`scripts/conformance_vectors.py`** has a screen section at its foot:
     - `build_screen` builds everything, and returns nothing until the corpus verify has passed
       on every photo.
     - Its sub-builders are `_screen_corpus`, `_screen_reference`, `_screen_synthetic` and
       `_screen_units`.
     - `_run_screen(input) -> expected` is **the definition a port reproduces**, on
       `run_vector`'s precedent.
     - `_verify_screen_against_stored` checks the corpus against the store.
     - Two refactors with no byte change: `_make_swing_module` now loads through a shared
       `_load_conftest`, and `_format_vector` reads `_python_version()`. The format
       byte-identity pin is green.
   - **`scripts/conformance.py`**:
     - `screen_vector_paths()` is new.
     - `check` prints `N screen vectors are crates/screen's …`. It runs no staleness test and
       never calls `run_vector` on them.
     - `list` takes its version column from the first key *present*, so frozen Python's `v0`
       shows.
     - `regenerate --screen-once` refuses with exit 2 on **any file** under
       `spec/vectors/screen/`, before the recorder is imported. A missing `ocr` extra
       (`MissingOCRExtra`) is also refused, with exit 2.
   - **`tests/test_conformance.py`** has 8 new pins (9 cases), and `_refuse_to_build` now stands
     in for `build_screen` too.
2. **The run.**
   - **41 files**: 13 corpus, 2 reference, 20 synthetic and 6 units, all plain CRLF `.json`,
     about 490 KB together.
   - **The corpus verify passed on all 13**, by exact `==` on every key but
     `provenance.image_path`. Each vector's `label_ratio` also equals the rotation vote's.
   - **It took ~38 minutes, not ~6.** That is about 2.5 min a photo against the planning run's
     ~20 s. The full pytest took 16 min on the same afternoon, so the box was slow. The cause was
     not investigated.
   - **A second `--screen-once` exited 2 and wrote nothing**, checked by sha256 of every screen
     file before and after.
   - **The `data/` snapshot is unchanged.** It covers 28 files: 13 stored shots and 15
     manifests. The combined digest is `eea2ef734aac1a96377c96938f0483e339563019dcf56c993fdb44c061793217`,
     the sha256 of `{ find data/processed/shots -type f; ls data/processed/sessions/*/*/manifest.json; } | sort | xargs sha256sum`,
     run from the repo root in Git Bash. **P10 and P12 compare against that.**
3. **One departure from call 1: `screen_parser_version` is top-level, not under `provenance`.**
   - **Why.** Every family keeps the version it ages on at the top level (`analysis_version`,
     `detector_version`, `python_version`). That is where `list` reads it, and where `golf-core
     rerecord` moves `analysis_version`.
   - **What sits under `provenance`.** `python_version`, and `paddleocr_version` on the photo
     vectors (3.7.0).
   - **For P7.** The version move is `screen_parser_version` at the root, as M32 moved
     `analysis_version`. Frozen Python's vectors say `0`.
   - **The refusal text** names `spec/declarations/screen-v<N>.json`.
   - **Pins.** No vector carries the key twice, and no vector carries an `analysis_version`.
4. **Two shapes, for P5.**
   - **Documents** (`corpus`, `reference`, `synthetic`) carry `input` and `expected`, with floats
     as JSON numbers. The workspace's `serde_json` has `float_roundtrip`, so they land on
     Python's bits.
     - `input.boxes` deserialize straight into `screen::TextBox`.
     - `expected.parsed.warnings` exclude the notes.
     - `expected.shot.provenance.warnings` are the notes, then the parser's warnings, then the
       validator's.
     - Corpus timestamps are the stored strings. The others are `2026-08-04T12:00:00Z`.
     - Synthetic vectors have a `null` `image_sha256` and `image_path`.
   - **`units`** carry `cases`, with floats as `repr` strings, the format family's convention.
     Case counts are a snapshot.

     | table | cases | each case |
     |---|---|---|
     | `normalize_label` | 289 | `{text, expected}` |
     | `field_for` | 298 | `{profile, text, label, score}`, with `score` a `repr` and both `null` on no match |
     | `first_number` | 185 | `{text, expected: null \| {value, matched}}` |
     | `sign_from` | 41 | `{residual, field, expected: 1 \| -1 \| null}`, with `field` a full `ProfileField` dump |
     | `is_blank` | 92 | `{text, blank_markers, expected}` |
     | `cell_text` | 10 | `{case, label_box, value_boxes, expected}` |

   - **`field_for`'s profiles.** Each `field_for` case names an entry in the table-level
     `profiles`, which is `{hd_golf, tie}` as pydantic dumps with every default explicit. That is
     "each case carries its profile" without 298 copies.
   - **The `tie` profile** is crafted. It makes `>=` observable: `Spin Z` scores 5/6 against both
     fields, and a bare `Spin` scores exactly the 0.8 threshold against both, and in each case the
     later field wins.
5. **The corpus, measured.**
   - **The `Impact Position` pair is on all 13 photos.**
     - On 11, it is `Impact Position` (1.0) beside `Impact Position V` (0.9375).
     - On `2026-08-23-1`, the V tile reads `ImpactPosition` (0.9655).
     - On `2026-08-10-1`, both read `Impact Position` at 1.0, and the V tile (x≈1437) is seen
       first, so it wins.

     Each vector's note lists its pair, generated from the boxes.
   - **The `CENTER` spill is recorded**, and it lands in **two** neighbours' raw text, not one.
     `2026-08-10-1` has `Shot Type: "CENTER SLIGHT FADE"` and `Smash Factor: "1.00 CENTER"`. Only
     Shot Type changes a value, since `_first_number` reads `1.00`. **P10's bookkeeping list
     should expect the Smash Factor raw field to change too.**
   - **The box texts are exactly P2's `_OCR_TEXTS`**: 198 distinct, with nothing in either set
     that is not in the other. The transcription holds.
   - **Preprocessing notes.** 6 of the 13 carry `screen outline not found - parsing the photo
     uncropped`. None has a rotation or legibility note, and the reference photos have none.
   - **The reference photos** read Spin and Spin Axis as `no value text under the label`. The OCR
     did not see the `---`, where the synthetic transcription has it. Neither has a duplicate
     label, and both have `label_ratio` 1.0.
6. **What gates which edge.** These counts are a snapshot.
   - **`pyfmt::floor_div` in `_Cell.text`** is gated only by `units/cell_text`'s "floor division"
     case. No value box in any photo vector sits where `//` and `floor(a / b)` part (0 of every
     cell). P2 finding 6 said a photo *can* hit this; none of these does.
   - **`pyfmt::sum` in `_score`** is gated by 5 of the 20 synthetic vectors, as P2 finding 7
     predicted, and by no photo vector:
     - `split-value-boxes`, `title-cropped`, `partial`, `partial-below-min-confidence` and
       `duplicate-label` change their `parsed.confidence` under a left fold.
     - `test_the_synthetic_screens_gate_cpythons_compensated_sum` pins this on the *inputs*, so
       it survives P10.
7. **Synthetic covers more than call 4 listed**, because P6 gates `validate.rs` on these vectors.
   - **Added beyond the parser paths**:
     - the eight validator paths of `test_screen_validate.py`, with messages through `:g` and
       `.3f`;
     - `partial-below-min-confidence`, the confidence branch of `needs_review`;
     - `duplicate-label`, which is in no test. A stray `Carry` is seen first and wins on `>`.
   - **`unreadable` is the only `shot: null`**, and a pin requires at least one.
   - **`title-cropped` mirrors its test's filter**, which drops every box containing `SHOT`. That
     takes the Shot Distance and Shot Type labels too, so Horizontal Angle reads
     `SLIGHT 2.6 ° L DRAW` and loses its sign.
8. **`_THOUSANDS` runs over the matched text, not the cell.** So its `\b` only ever sees the end
   of the match, a `,`, a `.` or a fourth digit. `\d{3}\b` reduces to "three digits not followed by
   a digit", and `5,991rpm` reads 5991.0.
   - **Unicode digits** count for `\d`, and `float()` reads them (`١٢٣` → 123.0).
   - **`float()` refuses `1,2,3` and `1,2.5`**, so `_first_number` returns `None` on both.
   - **For P5's hand scanner**, that means no general Unicode `\b` is needed, only "is the next
     char of the match a digit".
9. **For P5: nothing in Rust runs this family yet.** `check`'s line already says "run `cargo
   test`". P5 should add the runner, plus a discovery pin over `spec/vectors/screen/` like
   `tests/format.rs`'s, so that a sub-family nothing runs fails rather than hides. **The
   definition to read is `_run_screen`**, beside `parser.py`.
10. **Verified green**:
    - Full `pytest`: 2,026 passed, which is P2's 2,017 plus 9.
    - `ruff check src tests scripts` passed.
    - `conformance.py check` exited 0 and printed the screen line. `list` shows 41 screen vectors
      at `v0`.
    - `cargo test` passed. P4 touched no crate; it ran because `spec/vectors/` grew.
    - `git diff -- src/` is empty. `spec/`'s changes are untracked files only: P2's five and
      `spec/vectors/screen/`.

### P5 — the faithful parser (2026-10-01)

1. **What landed.**
   - `crates/screen/src/parser.rs`: `parse_screen(&[TextBox], &DeviceProfile) -> ParsedShot`,
     plus `ParsedShot` and `FieldValue` (`Number(f64)` or `Text(String)`). `ParsedShot.values` and
     `.raw_fields` are `pyfmt::OrderedMap`s, because they are dicts with insertion order.
     `confidence` is unrounded. `needs_review` is always `false` until P6's validator sets it, and
     `is_empty()` is `import_screen`'s `failed` test.
   - `first_number`, `sign_from`, `is_blank` and `cell_text` are `parser.py`'s private functions,
     made `pub` only because `tests/units.rs` gates each one. `is_blank` takes the markers rather
     than the profile.
   - `lib.rs` gains `pub mod parser`, and the crate doc lists it.
   - Tests: `tests/common/mod.rs`, `tests/parse.rs` (3 tests), `tests/units.rs` (7 tests), and 6
     new unit tests in `parser.rs`.
   - `Cargo.toml` gains `golf-core` as a dev-dependency (finding 3), and `Cargo.lock` changes with
     it.
2. **Every gate passed on its first run.**
   - All 35 documents (13 corpus, 2 reference, 20 synthetic) reproduce `expected.parsed` and
     `expected.label_ratio` under `golf_core::compare`.
   - `confidence` and `label_ratio` also match on bits (finding 4).
   - All six `units` tables pass exactly.
   - Because a first-run pass proves little, finding 6 is the evidence that the gate can fail.
3. **`compare` is reached through a dev-dependency on `golf-core`, and it stays there.**
   `compare.rs` needs only `serde_json`, so it *could* live lower. Moving it, though, is a
   structural change that P5 does not need.
   - **The P6 cycle.** When P6 gives `golf-core` its normal edge to `screen`, this becomes a
     dev-dependency cycle. It was **tried**: the edge was added temporarily, `golf-core` built, and
     `cargo test -p screen --test parse` was green. Then it was reverted. Cargo accepts the cycle,
     because the integration tests link one `screen`, the same one `golf-core` links. The manifest's
     comment says so.
   - **If someone wants `compare` lower later**, that is a `REFACTOR_LEDGER` question, not P6's.
4. **`confidence` is held to the bit, because `RTOL` cannot see the edge it exists for.**
   - **The mutation.** With `pyfmt::sum` replaced by a left fold, `compare` passed all 35
     documents. The bits test failed on exactly P4's five synthetic vectors, each by 1 or 2 ulp, for
     example `partial`: `0.6349999999999998` against `0.635`.
   - **For P7 and P10.** `golf-core rerecord` compares under `RTOL` too, so a move of an ulp or
     two in `confidence` is invisible to a declaration. Only `tests/parse.rs` would see it.
5. **`\d` is a table in `parser.rs` (`DECIMAL_ZEROS`), not a `pyfmt` function.**
   - **What it is.** 68 runs of ten, 680 characters, generated from the recording interpreter:
     CPython 3.13.3, Unicode 15.1. There, `chr(c).isdecimal()` and `re`'s `\d` agree on every code
     point. `float()` reads each digit at its offset from its run's start.
   - **Why not `pyfmt`.** That crate's rule is one format table per function, and P5 may not
     regenerate the format family. `units/first_number` gates it here: 4 cases fail with ASCII
     digits only.
   - **When it could move.** If the format family is ever legitimately re-recorded, a `decimal`
     table would let it join `pyfmt`.
   - **Rust 1.87's extra runs are left out on purpose.** Unicode 16.0 has Nd runs that 15.1 lacks,
     as with `upper` and `is_printable` in P2.
6. **The mutation run.** Eighteen mutations were applied one at a time, each run against the whole
   crate's tests and then reverted (the file was restored and `cmp`'d).

   | mutation | caught by |
   |---|---|
   | `_find_labels`' `>` made `>=` | `parse.rs`: 14 differences, the 2026-08-10-1 tie among them |
   | `pyfmt::sum` made a left fold | the bits test only (finding 4) |
   | `str_repr` made `{:?}` in `no tile found for` | `parse.rs`: 29 differences |
   | `floor_div` made `(a / b).floor()` | `units/cell_text`: 1 case, the only one (P4 finding 6) |
   | the `_THOUSANDS` rule dropped | `units/first_number`: 13 |
   | ASCII digits only | `units/first_number`: 4, plus the table's unit tests |
   | `strip` made `trim` | `units/cell_text`: 1 |
   | `strip_space` made `trim` in `is_blank` | `units/is_blank`: 11 |
   | the one-letter sign rule dropped | `parse.rs`: 1, and `units/sign_from`: 4 |
   | the title check, either half removed | `parse.rs`: 5, and 3 |
   | the printed sign ignored | `parse.rs`: 5 |
   | `by_label` keeping the first field | **no vector**, now a unit test |
   | label boxes not excluded from cells | **no vector**, now a unit test |
   | a printed sign beating a direction word | **no vector**, now a unit test (finding 7) |
   | a text tile split with `split_whitespace` | **no vector**, now a unit test |
   | `first_min` made `f64::min` | nothing. It differs only on a NaN, and geometry has none |
   | `value_coverage`'s `min(…, 1.0)` dropped | nothing. It is unreachable, since `read_values` ≤ distinct stored labels ≤ `expected` |

   - **How the four "now a unit test" rows were pinned.** Each was measured on frozen Python first,
     and its test holds that answer, as `profile.rs`'s tests do. After that, all four were caught.
7. **A gap in the synthetic family, and in the Python test it came from.**
   - **The case.** `word-beats-printed-sign` reads `-9.3 ° R` on Spin Axis. That tile's
     `printed_sign` is `-1`, so the word (`R`, +1) and the inverted print agree on `+9.3`, and the
     vector cannot tell which rule answered.
   - **The Python test has the same gap.**
     `test_screen_parser.py::test_a_direction_word_still_beats_a_printed_sign` cannot tell the two
     rules apart either. It is frozen and was left alone.
   - **The Rust pin.** `a_direction_word_beats_a_printed_sign_where_they_disagree` reads
     `-9.3 ° L`, where frozen Python answers `-9.3` and the printed rule would give `+9.3`.
   - **No stop**, because the rule is now gated. P8's hand vectors could carry a disagreeing case if
     the shipping parser should be gated on it by a vector too.
8. **For P6.**
   - **The notes are the caller's.** `import_screen` prepends them to `warnings`; the parser never
     sees them.
   - **The `contracts` edge.** P6 adds it (P3 finding 2), and `to_shot_data` maps
     `ParsedShot.values` onto `ShotData` by target key.
   - **Order is dropped at the record.** `ShotProvenance.raw_fields` is a `BTreeMap`, so the
     parser's insertion order goes no further than the record.
   - **Which vector gates validation.** `unreadable` is the only document with no label, so it is
     the only `shot: null`, and the early return there is gated.
9. **For P8.**
   - **Where the tie lives.** The `>` is in `find_labels`, an `OrderedMap<(score, box index)>`.
     `build_cells` excludes the label boxes by an index set. Call 8's "every box assigned in any
     member of N is a label box" goes into that set.
   - **Pins P8 must update.** `parse.rs`'s `every_screen_sub_family_is_run_by_a_test` must gain
     `hand`. `common::assert_recorded_by_frozen_python` is where the faithful gate states that it
     answers to Python-recorded, unstamped vectors only.
10. **A tooling trap.** In Git Bash, `grep $'\r'` reported LF for a file that `od -c` showed was
    CRLF, so line endings were checked by reading bytes in Python. A Python `write_text` on this
    box also writes CRLF. `parser.rs` briefly went CRLF that way and is LF again, like the rest of
    the crate.
11. **Verified green**:
    - Full `cargo test` passed.
    - `cargo clippy --all-targets` gave 0 warnings, and `cargo fmt --check` passed.
    - `cargo doc -p screen --no-deps` gave 0 warnings.
    - `git diff -- src/` is empty. `spec/` was only read.

### P6 — validate, the shot, and `golf-core parse-screen` (2026-10-01)

1. **What landed.**
   - **`crates/screen/src/validate.rs`**: `validate_parse(ParsedShot, min_confidence) -> ParsedShot`,
     faithful to `validate.py`. It holds the three identities, the twelve ranges in the dict's order
     (`PLAUSIBLE_RANGES`, an array P9 lengthens), and the penalties added left to right with `+=`.
     Messages go through `pyfmt::{g, fixed, str_repr, upper, split}`.
   - **`crates/screen/src/lib.rs`**:
     - `ScreenInput` is call 1's `input` and derives `Deserialize` with `deny_unknown_fields`.
     - `read(&ScreenInput)` composes parse, notes, validate and record.
     - `to_shot_data` is private.
   - **Small widenings in P5's files**:
     - `FieldValue` derives `Serialize`, untagged.
     - `parser::{first_min, first_max}` are `pub(crate)`, so `validate` clamps with the same two.
     - `profile::profiles()` is `pub(crate)`, for the target pin (finding 4).
     - `tests/common` gains `DOCUMENTS` and `every_document`, moved from `parse.rs`, which now
       uses them.
   - **Cargo edges.** `screen` → `contracts` is the edge P3 finding 2 deferred. `golf-core` →
     `screen` makes the dev-dependency cycle P5 finding 3 tried, and both manifests' comments now
     say so in the present tense. `cargo tree -p screen -e normal` shows `contracts`, `pyfmt`,
     `serde` and `serde_json`, and nothing else of ours.
   - **The verb.** `golf-core parse-screen` exists, with a module-doc section and a usage line.
     `run` and `parse-screen` now share `read_stdin` (vector or bare `input`) and `write_stdout`.
     `run`'s behaviour is unchanged, and `tests/engine.rs` still passes.
   - **Tests.**
     - `crates/screen/tests/read.rs` has 4 tests.
     - `crates/core/tests/parse_screen.rs` has 6.
     - The screen unit tests went from 22 to 34 (finding 6).
2. **Two departures from the plan's wording, neither a change of design.**
   - **`read` returns `Result<Option<ShotData>, UnknownProfile>`, not `Option<ShotData>`.**
     - **Why.** Call 5's signature has no answer for a device no profile names. Python's
       `load_profile` raises before `import_screen` is reached.
     - **The two cases differ.** A failed read is the photo's fault, and a missing profile is the
       caller's. The verb mirrors this: `null` with exit 0 for a failed read, and exit 1 with
       nothing on stdout for an unknown device.
     - **M38 calls `read` directly**, so it inherits the `Result`.
   - **`ScreenInput.min_confidence` is required, with no default.** `import_screen` requires it
     too, and the lab's value is `settings.ocr_min_confidence`. A default here would be a second
     copy of that setting. **M38 must pass its own threshold.** `notes`, `image_sha256` and
     `image_path` do default (to empty, `None` and `None`).
3. **Every Python-recorded document passes end to end, through the library and through the verb.**
   - **Through the library.** `tests/read.rs` runs all 35 documents (34 shots and `unreadable`'s
     `null`) under `golf_core::compare`. It also holds `parse_confidence` to the bit, for
     `parse.rs`'s reason one step later.
   - **Through the verb.** `tests/parse_screen.rs` pipes all 35 through the binary. Each stdout
     equals `screen::read`'s value with no tolerance, so the verb passes because the library does.
   - **The plan's verify command.** `parse-screen < corpus/2026-08-23-2.json` prints a shot equal
     to `expected.shot` on every key Python wrote. The ten M32 keys are at their defaults:
     `parser_version` 0, `fields_present` null, and `corrections` `{}`.
   - **The faithful port stamps nothing.** `the_faithful_port_stamps_nothing` holds that.
4. **`to_shot_data` hands the values to serde by key, which is the port of `ShotData(**values)`.**
   - **One list.** `ShotData`'s own field list is the only list of targets. **P8's
     `impact_position_v` target needs no edit to `to_shot_data`.**
   - **Where serde and pydantic part, measured on frozen Python.** The two part only on a wrong
     profile:
     - An unknown target is dropped by both.
     - A number for a text field is refused by both.
     - A numeric-looking word for a float field (`'1.5'`) is coerced by pydantic and refused by
       serde.
     - A target that collides with an identity key raises `TypeError` in Python and hits an
       `assert!` here.
   - **The pin.** `every_shipped_target_is_a_shot_data_field_of_its_kind` fills every stored field
     of every shipped profile with a value of its kind and checks that it lands on its key
     unchanged. That pin makes all four cases unreachable, and it will check P8's new target
     without being edited.
5. **Call 7's allowance, as built. P10 deletes it.**
   - **Where the keys come from.** They are read from `spec/declarations/v17.json`'s `added`,
     re-rooted from `expected.swing.shot` to `expected.shot`, and not spelt in the test. The
     default each must hold comes from serializing an unfilled `ShotData`.
   - **The path spelling.** Each comparison is rooted at the document, so `compare` reports the
     keys as `expected.shot.attack_angle` … `expected.shot.provenance.corrections` (kind
     `AddedKey`). **Those ten strings are exactly what P7's shape-only declaration and P10's
     declaration name.**
   - **`the_allowance_covers_exactly_m32s_keys`.** All 34 read shots use all ten keys and nothing
     else, and `unreadable` uses none.
   - **`the_allowance_opens_only_at_the_default`.** Each of these is still a difference:
     - a stamped `parser_version`;
     - a recorded `fields_present`;
     - a key outside the ten;
     - an M32 key that the record *has*, at another value.
   - **For P10.** The parts to delete are `m32_keys`, `defaults`, `allowed`, and those two tests.
     `differences` then collapses to a plain `compare`.
6. **The mutation run.** Twenty-one mutations were applied one at a time against `cargo test -p
   screen`, and each file was restored and checked byte-equal.
   - **Caught from the start: 14.** Eleven were caught by the vectors: the notes appended instead
     of prepended, the confidence unrounded, the empty read stored, a stamped `parser_version`,
     `.3f` made `g`, `needs_review`'s `||` made `&&`, either penalty changed, `str_repr` made
     `{:?}`, the sign test inverted, and the deadband removed. Three were caught only by the unit
     tests written with `validate.rs`: the contradictory-tile guard removed, the zero clamp
     removed, and the range order swapped.
   - **Not caught at first: 7.** No vector reaches any of them. Six survived the first run. The
     seventh, the half-even rounding, was added after it, and the `0.0625` case alone would not
     catch it. Each was measured on frozen Python and pinned as a unit test, and all 21 are caught
     now.

     | mutation | why no vector sees it | pinned by |
     |---|---|---|
     | `round_to` made `(x*1000).round()/1000` | needs an exact binary tie | `the_confidence_rounds_as_python_rounds`: `0.0625` → `0.062` |
     | the same, made half-even | needs a value just above the half | the same test: `0.0005` → `0.001` |
     | no `upper` on the shape words | the parser upper-cases every text tile first | `a_shape_word_counts_in_any_case` (a direct caller) |
     | the `club > 0` guard removed | at 0 Rust's `inf` fails no comparison, and Python would raise | `a_club_speed_not_above_zero_skips_the_smash_identity`, now with `-88.6` |
     | distance slack in yards only | no synthetic total is above ~67 yd *and* off by 1–3% | `a_long_shot_gets_proportional_slack` |
     | smash scale without the printed smash | needs a printed smash above the computed one, within 3% of it | `the_smash_slack_scales_with_the_larger_of_the_two` |
     | ranges made open | no value sits on a bound | `a_value_on_a_bound_is_inside` |
7. **For P7.**
   - **Not diffed against Python.** `parse-screen` has no `conformance.py` counterpart, and none is
     owed, because the screen family is recorded once (call 3).
   - **`rerecord` will need its own call into `screen::read`.** The verb is a seam for a human; it
     is not the recorder's entry point.
   - **`parse_confidence` is held to the bit in `read.rs`, as P5's two floats are.** A re-record
     compares it under `RTOL`, and that is safe for this one. A value rounded to three places moves
     by at least 0.001 or not at all, so no move of it can hide inside an ulp.
8. **For P8.**
   - **Where the stamp lands.** It lands in `read`. Today `read` *is* the faithful port.
   - **What moves to `screen::frozen`.** Once the shipping parser stamps, three things go there
     with the faithful rules:
     - `tests/read.rs`, which calls `common::assert_recorded_by_frozen_python`;
     - `the_faithful_port_stamps_nothing`;
     - `the_confidence_rounds_as_python_rounds`, if `round_to` changes. It should not.
   - **One `ScreenInput`.** The verb and M38 call the shipping `read`.
9. **A tooling trap.** On Windows, Python's `open()` without `encoding='utf-8'` decodes as cp1252.
   The verify output's `°` then compared unequal to a byte-identical vector. Every check on this box
   that reads JSON needs the explicit encoding.
10. **Verified green**:
    - Full `cargo test` passed.
    - `cargo clippy --all-targets` gave 0 warnings, and `cargo fmt --check` passed.
    - `cargo doc -p screen -p golf-core --no-deps` gave 0 warnings.
    - `pytest tests/test_docs_truth.py` passed (92).
    - The plan's `cargo run --release --bin golf-core -- parse-screen < …/2026-08-23-2.json` exited
      0 (finding 3).
    - `git diff -- src/` is empty. `spec/` was only read.

### P7 — `golf-core rerecord` gains the screen family (2026-10-01)

1. **What landed, all in `crates/core`.**
   - **The declaration.** `Declaration.version: Version` replaces `analysis_version: i64`. `Version`
     is `Analysis(n)` or `ScreenParser(n)`, with `key()`, `number()` and `current()`. A file
     carries exactly one of the two keys. Both, or neither, is refused at load, naming call 6.
     `v17.json` loads unchanged.
   - **The walk.** `plan` checks the version against its family's constant, then dispatches:
     - `walk_engine` is M32's walk, moved verbatim;
     - `walk_screen` walks `SCREEN_DOCUMENTS` (`corpus`, `reference`, `synthetic`).

     The tail is shared: the gate, the outside-the-repo refusal and the typo guard. A run never
     opens the other family's files.
   - **`SCREEN_UNREAD = ["units"]`.** `units/` holds case tables, not documents, and each
     `field_for` case carries its own profile. Any other entry under `vectors/screen/` is refused
     before a vector is read, a loose file included.
   - **`rerecord::run_screen(&ScreenInput) -> Result<Value, UnknownProfile>`** is the counterpart
     of `_run_screen`. It returns `label_ratio`, `parsed` (the four keys, built by hand) and `shot`
     (through `screen::read`). It is `pub`, beside the walk, for `run_stages`' reason: a vector's
     layout is vector plumbing.
   - **The ledger and the report.**
     - The ledger entry is keyed by the declaration's version key, so a screen vector's says
       `screen_parser_version`.
     - The report's header names the key.
     - Its footer reads `(N screen)`.
     - Every engine-side string is unchanged, and so are the existing tests.
   - **The verb.** `golf_core.rs` gains a module-doc paragraph and a `USAGE` line, both saying the
     version key picks the family. There is no flag for it. `lib.rs` gains one sentence.
2. **The "find out first" answer: a list is declarable, and no fix lands here.**
   - **A list of another length** is one `Moved` at the list's own path, carrying both whole lists.
     So `moved: ["expected.parsed.warnings"]` matches it, and `apply` copies the whole list.
   - **A list of the same length** whose entries differ moves once per index (`…warnings[2]`), and
     the list's path does not cover that. Where both kinds occur across the vectors, a declaration
     names both spellings, and the typo guard holds each to having matched somewhere.
   - **The pin.** `a_list_is_declared_whole_when_its_length_changes` covers it, and the module doc
     says it.
3. **⚠ The finding that stops P10: under decision 3 the re-record *removes* keys, and the gate
   never lets a removed key through.**
   - **The rule.** It is M32's gate rule, recorded in four places as "a removed key never passes":
     `rerecord.rs`'s module doc, `golf_core.rs`'s, CONFORMANCE §4, and ADR-032's addendum. M32's plan gave it a
     situational reason: "M32 declares no removal, and §M32 names no such kind".
   - **What decision 3 removes.** A withheld field "has no `raw_fields` entry". Today both label-fix
     shots *have* one:
     - `2026-08-10-1` has `"Impact Position": ""`, because the V box won and read blank;
     - `2026-08-23-1` has `"HEEL"`.
   - **The paths P10 would have to declare as removed**:
     - `expected.parsed.raw_fields.Impact Position` (both shots);
     - `expected.shot.provenance.raw_fields.Impact Position` (both shots);
     - `expected.parsed.values.impact_position` (`2026-08-23-1`, unless P8 stores a withheld
       field's `None` in `values`).
   - **What is not a removal.** `expected.shot.impact_position` going `HEEL` → `null` is a *move*,
     since `ShotData` always serializes the key, and it is declarable.
   - **The contradiction.** P10's exit lists "their missing `raw_fields` entries" as expected
     bookkeeping. The plan saw the removal coming and did not see that the gate refuses it.
   - **Why P7 did not fix it.** By P7's own rule ("the fix lands here") this would be P7's to
     solve. The fix, though, reverses a rule the ADR records, so it waits for the user. P8 and P9 do
     not depend on it.

   **Options**:
   - **(a) A `removed` list, for screen declarations only.** *(Recommended.)*
     - **How it works.** It mirrors `added`. Paths are matched exactly, ledgered as `removed`, and
       held to the typo guard, and `apply` deletes the key.
     - **Why it is safe.** A removal is named path by path and reviewed in the report as an added
       key is, so the rule's purpose still holds: a port cannot drop a key *silently*. Engine
       declarations keep M32's rule. So `conformance.py::frozen_view`, which knows only `added`
       and `moved`, needs no change, and frozen tooling is not touched.
     - **Cost.** About 60 lines and tests in `rerecord.rs`, at the start of P10 or as a P7b. P11's
       ADR-032 addendum records the change of rule.
   - **(b) `removed` for both families.** It also has to teach Python's `frozen_view` and
     `ledger_covers`, for a case no engine change has needed.
   - **(c) Keep the rule and amend decision 3**, so a withheld field keeps a `raw_fields` entry. That
     loses the structural "in `fields_present`, no `raw_fields` key" mark that M35 and M37 read as
     `misread`.
4. **P10 step 1's wording is wrong in a second, smaller way: `raw_fields` is declared per key, not
   as a whole.**
   - **Why.** `compare` walks an object key by key. So `moved: ["expected.parsed.raw_fields"]`
     matches nothing, and the typo guard would refuse it.
   - **The added entries.** The V tile's new entry is `added` at
     `expected.parsed.raw_fields.Impact Position V` and at
     `expected.shot.provenance.raw_fields.Impact Position V`.
   - **The moved entries.** Changed entries are each `moved` at their own key, for example
     `2026-08-10-1`'s `Shot Type` and `Smash Factor`. A key with spaces or `&` parses as a ledger
     path. Only `.`, `[` and `]` cannot be named.
   - **The version.** `screen_parser_version` 0 → 1 is a `moved` path on every document.
5. **The shape-only declaration is the ten keys *and* the version move.** Rust's side carries this
   build's `SCREEN_PARSER_VERSION` at the top level, as M32's carried `ANALYSIS_VERSION`, so "exactly
   those keys" means the ten plus `screen_parser_version`.
   - **On the real `spec/`**, a scratch declaration in `target/` (inside the repo, gitignored,
     deleted after) gave this `--dry-run` report:
     - 35 vectors run, all `screen`, and 35 changed;
     - each of the ten keys added on 34 documents;
     - `screen_parser_version` moved on 35;
     - `unreadable` moving the version alone;
     - 0 files written.
   - **Nothing was written.** The sha256 of all 135 files under `spec/` was identical before and
     after.
   - **The v17 run.** `rerecord --declare spec/declarations/v17.json --dry-run` reports
     `42 vectors run (21 engine, 21 stage); 0 changed; 0 files written`.
6. **The tests.**
   - **Unit tests in `rerecord.rs`**, three new:
     - exactly one version key, which picks the family;
     - a screen ledger entry keyed by `screen_parser_version`;
     - the list rule.
   - **Integration tests in `tests/rerecord.rs`**, four new, each on a copy of the *whole* screen
     family:
     - The shape-only run first leaves out one key, and the gate refuses on each of the 34 shots
       and writes nothing. With the full declaration it then lands exactly its paths, keeps CRLF
       and ledgers each file. Undoing the declared paths gives back every document. Every
       non-document file, `units/` included, is byte-identical, and a second run writes nothing.
     - A screen run passes with every engine and stage vector unreadable. An engine run passes with
       a screen document unreadable and an unplaced sub-family beside it, and writes no screen
       byte.
     - The screen version guard is checked before anything is read.
     - An unplaced directory or loose file stops the walk.
   - **They survive P10.** `Slice::age_the_screen_documents` puts each copy back to frozen
     Python's shape (one version back, no M32 keys, no ledger), so the shape-only change stays a
     real difference after P10 re-records the committed files.
   - **Mutations**, each caught, each file restored and `cmp`'d:
     - placement check off;
     - ledger always keyed `analysis_version`;
     - screen version not substituted.
7. **For P8.**
   - **`hand/` must be placed.** Until it is in `SCREEN_DOCUMENTS` or `SCREEN_UNREAD`, the two
     tests that copy the committed family fail, by design. The recommendation is
     `SCREEN_DOCUMENTS`: a hand vector is a document, and a later parser change re-records it
     under a declaration, keeping `oracle: "hand"` with a ledger.
   - **`run_screen` lists `parsed`'s keys by hand.** A key P8 adds to `ParsedShot` reaches a
     vector only if it is added there. That covers P10's "whatever P8 added to `parsed`".
   - **The recorder calls the shipping path.** That is `screen::parser::parse_screen`,
     `screen::read` and `load_profile`, and never `screen::frozen`. P8's shipping parser must
     answer at those names.
8. **For P11.**
   - **CONFORMANCE §2** says "four required keys — `analysis_version`, …". It is now one of two
     version keys.
   - **CONFORMANCE §4's `rerecord`** walks one family per run.
   - **`tests/test_conformance.py:948–950`** says "P10 re-records these four". `units/` is
     `SCREEN_UNREAD`, so it is three.
   - **Python's pins already allow a screen ledger.** The "version twice" pin reads only
     `provenance`'s own keys, and the `== 0` pin is skipped where `rerecords` exists. `check`
     never reads the screen family's ledger, so no Python change is owed for P10's write.
9. **Verified green**:
   - Full `cargo test` passed.
   - `cargo clippy --all-targets` gave 0 warnings, and `cargo fmt --check` passed.
   - `cargo doc -p golf-core --no-deps` gave 0 warnings.
   - Both dry runs are in finding 5.
   - `git diff -- src/` is empty. `spec/` was only read, and its hashes are unchanged.

### P8 — the tie rule, the V tile, `fields_present` and the stamp (2026-10-02)

1. **What landed.**
   - **`crates/screen/src/parser.rs`**:
     - `parse_screen` is now the shipping parser: `assign_labels` (the tie rule), then
       `read_labels`, the shared body.
     - `read_labels` is everything after locating: rows, cells, reads, the missing/withheld walk
       and the score. `screen::frozen` hands it the faithful labels, so it is one copy gated by
       both families.
     - `Labels { tiles, withheld }` carries a `Located` that is `(Option<&ProfileField>, box)`,
       where `None` is a box the rule left unresolved.
     - `pub const TIE_MARGIN = 0.02` and `pub const TIE_CAP = 8`, each doc carrying its measurement.
     - `ParsedShot.fields_present: Vec<String>`.
   - **`crates/screen/src/frozen.rs`** (new, deleted in P10):
     - `include_str!`s `src/golf_coach/launch_monitor/screen/profiles.json`.
     - Exports `load_profile`, `parse_screen` (the faithful `_find_labels`, moved verbatim from
       `parser.rs`, plus `read_labels`) and `read`, which unstamps the record.
     - Holds three moved tests (`by_label`, the unstamped record, and one new: the first of two
       equal boxes wins) and one new pin: the frozen profile has no V tile.
   - **`crates/screen/src/lib.rs`**:
     - `read` stamps `parser_version: SCREEN_PARSER_VERSION` and `fields_present:
       Some(parsed.fields_present)`.
     - `record(parsed, input)` is the shared notes → validate → `None`-on-empty → `to_shot_data`
       tail, `pub(crate)` for `frozen::read`.
   - **`crates/screen/profiles.json`** gains `Impact Position V` → `impact_position_v`, kind
     `text`, **last** (after Spin Axis, the bay screen's tile order). It has a `_comment` of its
     own and no alias. The file stays CRLF, and `profile_fork.rs` declares the field as the one
     delta.
   - **Tests.**
     - `tests/parse.rs` and `tests/read.rs` now run `screen::frozen`.
     - `tests/hand.rs` (new) has 4 tests.
     - `parsed_json` moved into `tests/common` (with `HAND`), so both runners render
       `expected.parsed` one way.
     - The discovery pin gained `hand`.
     - The screen unit tests went from 34 to 46.
   - **`crates/core`**:
     - `SCREEN_DOCUMENTS` gains `hand` (P7 finding 7).
     - `run_screen`'s doc says why `fields_present` is not in `parsed`.
     - `tests/parse_screen.rs` runs `hand/` through the verb too.
     - `tests/rerecord.rs`: finding 5.
   - **`spec/vectors/screen/hand/`**: the seven vectors (finding 4).
2. **The tie rule as built** (call 8). Calls the plan left open, decided here:
   - **What a candidate is, and what a tie is.**
     - Candidates are every `(box, field)` scoring ≥ 0.8 against the field's label or aliases
       (`ProfileField::matches`, unchanged). `field_for` is no longer on the shipping path, and
       `label_ratio` still uses it.
     - A tie is a connected component of the candidate graph, found by BFS from each box in
       index order.
   - **The enumeration.** Every one-to-one partial assignment is visited **twice**: once for the
     best total, once to collect what each near-best assignment gives each field. That avoids
     holding up to 1.4M assignments.
     - Totals are summed in box order.
     - "Within the margin" is `best - total <= TIE_MARGIN`.
   - **The cap.** It is `> 8` boxes or `> 8` labels.
     - Unit tests pin the counts the doc quotes: 7 at 2×2 and 1,441,729 at 8×8.
     - An 8×8 tie of all-0.889 labels takes ~0.1 s in a debug test build, and resolves.
     - A ninth box refuses the whole tie: every field withheld, every box a boundary tile.
   - **`only_the_impact_position_pair_overlaps`** pins what the cap's argument and
     `raw_fields`' label keying rest on: in every shipped profile the labels are distinct, and no
     label or alias reaches 0.8 against another field except within the IP pair.
   - **Warnings.**
     - A withheld field gets `"{label}: label tie between 'A' and 'B' - not read"`. Box texts
       come through `str_repr` in box order, with `no box` last where a near-best assignment gives
       it none. 3+ items read `A, B and C`.
     - A refused tie gets `"{label}: label tie among {b} boxes and {f} labels, more than the tie
       rule weighs - not read"`.
     - Order: the cell warnings first (unchanged), then **one walk over the stored fields in tile
       order**, where each field gets its tie warning if withheld or `no tile found` if not
       located. With nothing withheld this is the faithful loop exactly.
   - **The score: a withheld field counts as a found label and as no read value**, and its boxes
     (and anything under them) are in `used`, as a boundary tile's are. This is a call the plan did
     not make; `score`'s doc argues it. The faithful score is unchanged (the frozen gate is
     bit-exact on `confidence`).
   - **Which boxes are labels.** Every box any near-best assignment gives a field becomes a label
     box. A box in a tie that no near-best assignment uses (`Carny` beside an exact `Carry`) stays
     ordinary text, as it did in frozen Python.
3. **For M35 and M37, the rule in one line** (decision 3): a stored field whose `ShotData` key is in
   `provenance.fields_present` but whose profile label has no key in `provenance.raw_fields` was
   located and withheld by the tie rule, so grade it `misread`.
   - **The complement.** Every label the parser *read* has a `raw_fields` key, `""` and `---`
     included. A key absent from `fields_present` was not on the screen.
   - **Pinned.** `hand.rs::a_withheld_tile_is_located_and_unread_and_nothing_else_is` checks it on
     the vectors.
4. **The hand vectors.**
   - **The seven, by stem, in the plan's order:**
     1. `impact-pair-exact`
     2. `impact-pair-v-read-as-y`
     3. `impact-pair-v-dropped`
     4. `impact-pair-v-joined` (`2026-08-23-1`)
     5. `impact-pair-real-joined` (the mirror)
     6. `carry-twice`
     7. `reference-layout`
   - **Their shape.** Each is call 1's shape: all nine `input` keys, M32's ten shot keys, top-level
     `screen_parser_version: 1`, `provenance {kind: "screen", oracle: "hand", note, source}`. They
     are CRLF, sorted, indented 4 and ASCII, as P4 writes.
   - **The inputs** are the bay layout, laid out with `build_screen`'s geometry (every box 0.95)
     by a scratch script outside the repo, which also wrote the JSON.
     - Cases 1–6 are the bay layout with IMG_2738's numbers.
     - Case 7 is `synthetic/reference-2738`'s boxes.
   - **The answers were typed from the working in each `note`**, never from a parser's output:
     values, raw fields, warnings, label and value counts, `label_ratio`, `fields_present`. The
     script did only the confidence arithmetic.
     - Frozen Python was run on the inputs only to check the notes' "frozen Python does X" claims:
       `1.15 CENTER`, `HEEL` read, and `151.5 128.1 yds yds`.
     - All seven passed against `screen::read` on the first run.
   - **The mutation run.** Six mutations were applied one at a time, each caught by `hand.rs`, and
     each file restored and `cmp`'d:

     | mutation | caught on |
     |---|---|
     | `TIE_MARGIN = 0` (strict equality) | `v-joined` and `real-joined`; the mirror stores `HEEL` as `impact_position_v` |
     | unresolved boxes not made labels | `v-dropped` (`1.15 CENTER`), `real-joined` (`1.15 HEEL`), `carry-twice` |
     | withheld fields left out of `fields_present` | the four withholding vectors |
     | withheld fields not counted as found labels | the same four, on `confidence` and `parse_confidence` |
     | `parser_version` 0 | all seven |
     | a field resolved when one box is among several outcomes | `carry-twice` and the pairs |

   - **Left out on purpose.** P5 finding 7's disagreeing word-vs-printed-sign case was not added as
     a hand vector. Its unit test still runs the shipping parser.
5. **⚠ A gap in P7, closed here: its screen re-record tests assumed the shipping parser reproduces
   frozen Python until P10.**
   - **What broke.** `a_screen_shape_only_declaration_…` and `a_screen_run_reads_no_engine_vector…`
     age the *committed* documents and expect the shape-only declaration to be the whole
     difference. From P8, `run_screen` (the shipping parser, as P7 finding 7 requires) also moves
     the tie, the V tile and the stamp. Without a fix the first test refused on **253**
     undeclared differences (measured, then restored).
   - **The fix.**
     - `Slice::age_the_screen_documents` first sets each copy's `expected` to `run_screen`'s
       answer, which is what P10 will commit, then takes the ten keys out and the version back.
       The tests then mean the same thing on both sides of P10.
     - The oracle check now reads "kept as found", since `hand/` is `"hand"`.
     - The unplaced-directory test uses `unplaced/`, because `hand/` is placed now.
6. **A P10 preview.** Measured on the real `spec/` with a scratch shape-only declaration in
   `target/` and `--dry-run` (deleted after). The sha256 of every file under `spec/` was identical
   before and after.
   - **The run refused on 212 undeclared differences across 34 documents.** All 7 hand vectors and
     `synthetic/unreadable` differ in nothing.
   - **Corpus values move only on the two label-fix shots**, as the exit says:
     - `2026-08-10-1`: `shot_type` goes `CENTER SLIGHT FADE` → `SLIGHT FADE`;
     - `2026-08-23-1`: `impact_position` goes `HEEL` → `null`.
   - **`needs_review` flips nowhere**, the synthetic family included.
   - **One synthetic value moves, and it was wrong before.** `synthetic/duplicate-label`'s
     `bounce_and_roll` goes **128.1 → 23.4**. Frozen Python's spill read the real Carry's `128.1`
     into Bounce & Roll, and the tie rule fixes it. Its `Shot Distance` and `Bounce & Roll` raw
     text change too, and its `raw_fields.Carry` is removed. P10 must list this beside the
     corpus.
   - **Removals** (P7 finding 3's question), exactly 7 paths:
     - `expected.parsed.raw_fields.Impact Position` and `expected.shot.provenance.raw_fields.Impact
       Position` on both label-fix shots;
     - `expected.parsed.values.impact_position` on `2026-08-23-1` (P8 keeps withheld fields out
       of `values`);
     - both `raw_fields.Carry` paths on `duplicate-label`.
   - **`raw_fields.Impact Position V` is added on 11 bay shots, not 13.** On the two label-fix shots
     the V tile is withheld, so it has no entry, and those two carry the tie warnings instead of
     the V tile's `no value text under the label`. P10 step 1 should declare it on 11.
   - **`parse_confidence` moves:**

     | documents | move |
     |---|---|
     | 11 bay shots | −0.016 to −0.020 |
     | `2026-08-10-1` | −0.019 |
     | `2026-08-23-1` | −0.040 |
     | the two reference photos | −0.043 and −0.044 (a stored label more, and not found) |
     | synthetic | −0.023 to −0.047 |

   - **`label_ratio` moves on all 34**, through the new denominator.
   - **`fields_present`, from `golf-core parse-screen` on the 15 photo vectors.** All 13 bay photos
     carry `impact_position_v` (on the label-fix two it is withheld), and the 2 reference photos
     carry `bounce_and_roll`. Every photo has 14 entries.
7. **For P9.**
   - **The filter.** `no tile found for` is written in `read_labels`' walk, which is where call
     10's prefix constant and `golfer_warnings` go.
   - **The new tie warning is golfer-facing as built.** It is a parse problem that retaking can
     fix. Whether the filter drops it is P9's call to make explicitly.
8. **For P10, what deleting `frozen` touches.**
   - **Delete:**
     - `frozen.rs`, with its `include_str!`;
     - `pub mod frozen` and the crate-doc bullet in `lib.rs`;
     - `common::assert_recorded_by_frozen_python`;
     - `read.rs`' allowance.
   - **Mentions to update:**
     - `record`'s and `read`'s docs;
     - `parser.rs`'s module doc ("The faithful rules stay runnable behind `crate::frozen`");
     - the docs of `Labels.withheld` and `read_labels`;
     - `profile_fork.rs`' parenthetical about `frozen`'s `include_str!`.
   - **Repoint** `tests/parse.rs` and `tests/read.rs` at `parser::parse_screen`,
     `profile::load_profile` and `screen::read`.
   - **Unchanged.** `read_labels` can stay as the seam it is. The aging step in
     `crates/core/tests/rerecord.rs` is right on both sides.
9. **Verified green**:
   - **Rust.**
     - `cargo test` (whole workspace): 719 passed, 0 failed, across 45 binaries.
     - `cargo clippy --all-targets` gave 0 warnings, and `cargo fmt --check` passed.
     - `cargo doc -p screen -p golf-core --no-deps` gave 0 warnings, once one link to the private
       `read_labels` was made plain text.
   - **Python, because `spec/` grew.**
     - `pytest tests/test_conformance.py`: 225 passed. The pins already exclude `hand/` from the
       Python-recorded checks.
     - `conformance.py check` exited 0 and counts 48 screen vectors, and `list` shows `hand/` at
       `v1`.
   - **Untouched.**
     - `git diff -- src/` is empty.
     - `spec/` changed only by the seven new `hand/` files. No Python-recorded vector was written.
       `data/` was not read.

### P9 — validate ranges, the golfer-facing filter, the `hd_golf` pin (2026-10-02)

1. **What landed.**
   - **`crates/screen/src/validate.rs`**:
     - `PLAUSIBLE_RANGES` goes from 12 rows to 18. Call 12's six ranges are appended in
       `ShotData` field order, after frozen Python's twelve, each with a comment arguing it.
     - The table's doc says the six are Rust's alone, unreachable from every shipped profile, and
       argued rather than measured.
     - Two new unit tests:
       - `the_new_fields_are_checked_once_a_profile_reads_them` reads a made-up profile's six tiles
         through `parse_screen` and then `validate_parse`. The clean read passes, and the slipped
         read (each comment's example ×10) gives the six range lines in table order, −0.15 each.
       - `no_shipped_profile_reads_the_new_fields` fails the day a profile targets one.
   - **`crates/screen/src/parser.rs`**:
     - `pub const NO_TILE_FOUND = "no tile found for "`, which `read_labels`' walk now formats with.
     - `pub fn golfer_warnings(&[String]) -> Vec<String>` sits beside it, and `lib.rs` re-exports
       it as `screen::golfer_warnings` (call 10). `lib.rs`' crate doc gained a bullet for it.
   - **`crates/screen/tests/golfer_warnings.rs`** (new, 4 tests):
     - **The sources.** Every document's warnings are taken twice: as recorded in
       `expected.shot`, and as `screen::read` writes them. That covers corpus, reference, synthetic
       and `hand/`.
     - **What it asserts.** The filter drops exactly the lines that start with a *spelt-out*
       `no tile found for '`, and keeps the rest in order. Both sources must drop something, and
       tie lines must come through.
     - **The layout test.** On every photo, the one dropped line names the tile its layout lacks:
       `Bounce & Roll` on the 13 bay photos and `Impact Position V` on the 2 reference photos. It
       is also exactly the stored field `fields_present` leaves out.
   - **`crates/screen/tests/capability.rs`** (new, 3 tests):
     - **The pin.** `devices.json`'s `hd_golf` equals the fork's non-null targets, compared as a
       set. A failure names both differences.
     - **Not vacuous.** `impact_position_v` is on both sides.
     - **Targets, not tiles.** The fork's one target-less tile is `Custom`, which is why the pin
       compares targets.
   - **`crates/contracts/devices.json`**:
     - The `hd_golf` `source` now names the fork and both pins.
     - The `impact_position_v` note now says the Rust parser locates the tile and the frozen lab's
       does not, and that no value has been read from it yet.
     - The `_comment` line about where labels live now names the fork.
   - **`crates/contracts/src/shot.rs`**: `SCREEN_PARSER_VERSION`'s entry 1 is rewritten.
     - **What 1 is.** The Rust parser as M34 ships it: the tie rule, the forked profile with the V
       tile, `fields_present`, and the new ranges.
     - **The faithful port has no entry.** It is one version, not two, because nothing shipped
       between them.
     - A stored shot is therefore either 0 (frozen Python) or on the ledger.
2. **Calls the plan left open, decided here.**
   - **The tie line stays golfer-facing (P8 finding 7).** The filter drops `no tile found for` and
     nothing else.
     - **Why the no-tile line goes.** It is about the layout: every bay photo lacks `Bounce & Roll`,
       and every reference photo lacks `Impact Position V`. ADR-034 §2 says a golfer is never told
       about a stat their screen does not show.
     - **Why the tie line stays.** It is about a tile the golfer can see, which was there and was
       not read. M35/M37 grade that `misread`, and a retake can fix it. A visible stat gone with no
       reason given reads as a bug. `golfer_warnings`' doc carries both arguments.
     - **The other lines stay too.** The refused-tie line, blanks, unreadable numbers, missing
       direction words, failed cross-checks and the preprocessing notes are all kept.
   - **The filter matches a prefix, not a substring.** A line that merely *mentions* the phrase is
     kept, and the made-up-list test pins it.
   - **A label the OCR damaged past 0.8 also reads `no tile found`, and is dropped too.** The line
     cannot tell that case from a missing tile. On every stored photo the line is the layout, as
     measured by the layout test.
   - **The pin covers `hd_golf` only.** `profile::profiles()` is `pub(crate)`, so an integration
     test can name a device but cannot walk every profile. One profile ships, and a second one
     should get its own `devices.json` entry and its own line here.
   - **Call 12's six bounds are kept as planned.**
     - **One blind spot, and its price.** `carry_offline`'s ±150 does not catch a small offline
       that loses its decimal point (12.5 as 125), the same blind spot `carry_distance`'s range
       has. Its comment names it, and why closing it would flag a real slice.
     - **The frozen path is unaffected.** `frozen::read` shares `validate_parse`, so it carries the
       six ranges as well. The frozen profile targets none of their fields, so every
       Python-recorded vector is unchanged (`read.rs` green).
3. **Measured.**
   - **The V tile on the 13 bay vectors** (`golf-core parse-screen`): on 11 it is read blank
     (`raw_fields` `""`, `no value text under the label`), and on the two label-fix shots it is
     withheld. It is in `fields_present` on all 13, and no value is read from it anywhere. That
     is what the rewritten `devices.json` note claims.
   - **The mutation run.** With `NO_TILE_FOUND` set to `"no tile for "`, all four
     `golfer_warnings.rs` tests failed. The file was restored from a copy.
4. **Outside the phase's file list, on purpose.** Two doc comments in
   `crates/contracts/src/capability.rs` named the pin and the V tile as M34's future work: the
   module doc's last paragraph, and `DeclaredField::note`'s. They now point at
   `crates/screen/tests/capability.rs` and at the two parsers' difference. No code changed there,
   and `crates/contracts/tests/capability.rs` is untouched and green. Its corpus test reads the
   engine vectors' `input.shot`, which stays unstamped.
5. **For P10.**
   - **`golfer_warnings.rs` needs no edit.** It does not call `common::assert_recorded_by_frozen_python`,
     and its two sources become one set after the re-record, as its doc says.
   - **One tense to reword after the run.** `shot.rs`' ledger says the faithful port "goes when they
     are re-recorded (M34 P10)". That stays true after P10, but P10 may want to make it past tense.
   - **The devices.json note holds after the re-record.** It says "on every bay photo the screen
     vectors hold", which is still true then.
6. **For P11.**
   - **Where the filter and the pin are named.** The program plan's §M34 (lines ~570, ~1002, ~1263
     and ~1418) and `m32-shot-contract.md:1073` name them as M34's future work. They are plans, so
     at most a pointer is needed.
   - **New things for `docs/CONFORMANCE.md` or `ARCHITECTURE.md` §1.** `NO_TILE_FOUND`,
     `golfer_warnings`, and the two pins (`profile_fork.rs`, `capability.rs`).
   - **Not P9's: two old `cargo doc` warnings in `contracts`.** They are unresolved links at
     `src/feedback.rs:44` (`M6`) and `src/unscored.rs:351` (`every_reason_has_a_row`), and they
     were there before this phase. `cargo doc -p screen` gives 0 warnings.
7. **Verified green.**
   - **Rust.**
     - `cargo test` (whole workspace): 728 passed, 0 failed, across 47 binaries. That is +9 tests
       and +2 binaries on P8.
     - `cargo clippy --all-targets` gave 0 warnings, and `cargo fmt --check` passed.
   - **Untouched.**
     - `git diff -- src/` is empty.
     - No file under `spec/` or `data/processed/` is newer than P8's close.
     - `data/` was not read.

### P10 — the 13-shot re-read (2026-10-02)

1. **The precondition, built first: `removed`, for screen declarations only** (the user's option
   (a) to P7 finding 3), all in `crates/core/src/rerecord.rs`.
   - **`Declaration.removed`**. The file key is optional, so `v17.json` loads unchanged. A
     declaration at `analysis_version` that carries the key at all, even empty, is refused at load.
     A `removed` path that ends in a list index is refused, as an `added` one is. The duplicate check
     now covers all three lists.
   - **The walk.** `gate` lets a `RemovedKey` through only at a declared `removed` path. `apply`
     deletes the key (new `LedgerPath::remove`, beside `put`, sharing `parent_mut`). The typo guard
     and `Ledgered` read `removed` as they read `added`. The report lists `removed <path> (was …)`.
   - **The ledger.** Every *screen* entry carries `removed`, empty or not, so the family's entries
     share one shape. No engine entry does, so frozen Python's `frozen_view` and `ledger_covers` and
     every engine-side string and test are unchanged.
   - **The report header.** A screen declaration's reads `N added, N moved and N removed`. The engine
     header is unchanged.
   - **Docs.** The module doc's "No removal" bullet is now two: the engine rule, and the screen
     exception with the two rejected options. `golf_core.rs`'s module doc says the same. The other
     two places P7 finding 3 named, CONFORMANCE §4 and ADR-032's addendum, are P11's.
   - **Tests.** Two new unit tests (`a_screen_declaration_removes_exactly_the_keys_it_names`,
     `removed_is_a_screen_declarations_alone`). One new integration test
     (`a_screen_removal_lands_where_it_is_named_and_nowhere_else`), on an aged copy with one planted
     `raw_fields` key. `a_removed_key_never_passes` is renamed `…_an_engine_declaration`. The two
     tests that assert a screen ledger entry exactly gained `"removed": []`.
2. **`spec/declarations/screen-v1.json`**: 12 added, 19 moved, 5 removed. LF, indent 2, as `v17.json`.
   - **Found by probing, then checked item by item.** An empty screen declaration's `--dry-run`
     refused 587 differences across 35 documents. Every distinct (kind, path) became one entry.
   - **`added`**: call 7's ten keys, and `raw_fields.Impact Position V` at both roots (11 bay
     shots, P8 finding 6). `fields_present` is one of the ten. P8 added nothing to `parsed`.
   - **`moved`**: `screen_parser_version`; the bookkeeping on 34 documents (`label_ratio`,
     `parsed.confidence`, `parsed.warnings`, `parse_confidence`, `provenance.warnings`); and per
     shot, at both roots where both exist:
     - `2026-08-10-1`: `values.shot_type`, `shot.shot_type`, `raw_fields.Shot Type` and
       `raw_fields.Smash Factor`;
     - `2026-08-23-1`: `shot.impact_position`, `HEEL` → `null`. It is a move, not a removal, since
       `ShotData` always serializes the key;
     - `synthetic/duplicate-label`: `values.bounce_and_roll`, `shot.bounce_and_roll`,
       `raw_fields.Shot Distance` and `raw_fields.Bounce & Roll`.
   - **`removed`**: exactly P8 finding 6's 7 paths, as 5 entries. Those are `raw_fields.Impact
     Position` at both roots (both label-fix shots), `parsed.values.impact_position`
     (`2026-08-23-1`), and `raw_fields.Carry` at both roots (`duplicate-label`).
   - **Every warnings list changed length**, so each is declared whole, and no `warnings[i]` path is
     needed (P7 finding 2).
3. **The exit, item by item.** It was measured on a scratch copy of `spec/` run without `--dry-run`,
   because the report cuts values at 72 characters. The copy was then diffed whole against the
   committed files, and the real run wrote byte-identical files.

   | documents | shot values | `parse_confidence` | `label_ratio` | warnings | `raw_fields` | `fields_present` |
   |---|---|---|---|---|---|---|
   | `2026-08-10-1` | `shot_type` `CENTER SLIGHT FADE` → `SLIGHT FADE`; `impact_position` stays `null` | −0.019 | 13/14 → 13/15 | − `Impact Position: no value text under the label`; + the two tie lines | Shot Type and Smash Factor lose `CENTER`; Impact Position removed | V located, withheld |
   | `2026-08-23-1` | `impact_position` `HEEL` → `null` | −0.040 | 13/14 → 13/15 | + the two tie lines | Impact Position removed | V located, withheld |
   | the other 11 bay | none | −0.016 to −0.020 | 13/14 → 14/15 | + `Impact Position V: no value text under the label` | + `Impact Position V: ""` | V |
   | `IMG_2738`, `IMG_2739` | none | −0.044, −0.043 | 1 → 14/15 | + `no tile found for 'Impact Position V'` | none | Bounce & Roll |
   | 18 synthetic shots | none | −0.023 to −0.047 | × 14/15 | + `no tile found for 'Impact Position V'` | none | Bounce & Roll |
   | `synthetic/duplicate-label` | `bounce_and_roll` 128.1 → 23.4 | −0.045 | × 14/15 | − `Carry: no value text under the label`; + `Carry: label tie between 'Carry' and 'Carry' - not read`; + the no-tile line | Carry removed; Shot Distance and Bounce & Roll lose the spilled `128.1` | Bounce & Roll |
   | `synthetic/unreadable` | still `null` | — | unchanged | — | — | — |
   | the 7 `hand/` | nothing differs | | | | | |

   - **Corpus values** move on the two label-fix shots alone, as the exit says. The one synthetic
     value is P8 finding 6's, and it was wrong before.
   - **No `needs_review` flips**, in any family.
   - **`fields_present` tells the layouts apart.** All 13 bay photos carry `impact_position_v` and not
     `bounce_and_roll`. The 2 reference photos are the reverse. Every photo has 14 entries.
   - **Warnings.** Every line the old and new lists share is word for word the same and in the same
     order. The only lines added or dropped are the ones the table names. The two tie lines on a
     label-fix shot read `Impact Position V: label tie between 'Impact Position' and 'Impact
     Position' - not read` and the same for `Impact Position` (`'ImpactPosition'` on `2026-08-23-1`).
   - **Two things the exit list did not spell out, both explained by the V tile, so not a stop.**
     - **`label_ratio` on the label-fix shots keeps its numerator (13/14 → 13/15).** `label_ratio`
       still counts through `field_for` (P8 finding 2), and both boxes' best field there is
       `Impact Position`, so the V tile adds to the denominator only. On the other bay shots the V
       box's best field is the V tile, so the numerator gains it too. Nothing reads `label_ratio`
       but the rotation vote.
     - **Every synthetic screen gains the no-tile line.** They are the reference layout, as
       `fields_present` shows, so they are the reference photos' row.
   - **A second run wrote nothing**: `0 changed; 0 files written`. Afterwards `--dry-run` reports the
     same, and the v17 `--dry-run` still reports `0 changed`.
   - **The `data/` snapshot is unchanged**: `eea2ef73…3217` over 28 files, P4's digest.
4. **What the re-record wrote.**
   - **The files.** 35 files: 13 corpus, 2 reference, 20 synthetic. `units/` (`SCREEN_UNREAD`) and
     `hand/` (nothing differs) are byte-identical, and so is every other file under `spec/`, by
     sha256. The only other change is the new declaration.
   - **What stays the same.** CRLF is kept, and so is `provenance.oracle: "python"`. Each file has
     one `rerecords` entry.
   - **Text churn, not value churn.** Non-ASCII is now raw UTF-8 (`7.6° Open`) where Python wrote
     `°`, because `serde_json` does not escape. The module doc's "How a file is written"
     already names this. Every reader here decodes UTF-8 (`_read_json`, `fs::read_to_string`).
5. **Step 4: the faithful port is deleted. Every screen test now runs the shipping reader.**
   - **Deleted.** `crates/screen/src/frozen.rs` goes, and with it the `include_str!` of the frozen
     profile and its four tests. Two of those were faithful-only rules: `by_label`'s later field,
     and the first of two equal boxes. The unstamped record and the frozen profile's missing V tile
     went too. `pub mod frozen` is gone, and so is the crate-doc bullet, which is now a paragraph
     saying where the rules went.
   - **Repointed.** `tests/parse.rs` runs `parser::parse_screen` and `profile::load_profile`, and
     `tests/read.rs` runs `screen::read`.
   - **Call 7's allowance is gone.** That is `m32_keys`, `defaults`, `allowed`, `differences`,
     `V17_SHOT` and their two tests. `every_recorded_shot_is_read` is now a plain `compare`.
   - **The bit tests are renamed `…_held_to_the_bit`.** Their expected values are the re-record's
     now.
     - **Still a gate, measured.** With `pyfmt::sum` swapped for a left fold, the confidence test
       fails on 3 synthetic documents (`duplicate-label`, `split-value-boxes`, `title-cropped`).
       P4's count of 5 was against frozen Python's denominators. The file was restored and `cmp`'d.
   - **One departure from P8 finding 8: `common::assert_recorded_by_frozen_python` was moved, not
     deleted.** `tests/units.rs` used it as well. `units/` stays frozen Python's at version 0, so
     the check now sits inline in `units.rs`, saying why. `common` gains
     `assert_recorded_for_this_parser` (`screen_parser_version == SCREEN_PARSER_VERSION`), and
     `parse.rs` and `read.rs` call it.
   - **Narrowed, since nothing else calls them:** `Labels`, `Located`, `read_labels` (`parser.rs`)
     and `record` (`lib.rs`) lose `pub(crate)`. `read_labels` stays a function of its own, as the
     seam between locating and reading, and its doc says why.
   - **Docs moved to the past tense**:
     - `parser.rs`'s module doc, and the docs of `read_labels`, `Labels.withheld` and `score`;
     - `lib.rs`'s `read`;
     - `profile.rs`'s `parse_profiles`;
     - `profile_fork.rs`, `golfer_warnings.rs`, `hand.rs` and `crates/core/tests/{parse_screen,
       rerecord}.rs`;
     - `shot.rs`' ledger entry 1 (P9 finding 5).
6. **For P11.**
   - **CONFORMANCE §4 and the ADR-032 addendum** should record that `removed` exists for screen
     declarations only, why, and the two rejected options. Copy them from `rerecord.rs`'s module
     doc.
   - **CONFORMANCE §2's ledger.** A screen entry has three lists, and an engine entry two.
   - **`tests/test_conformance.py:948–950`** still says "P10 re-records these four". It re-recorded
     three, since `units/` is unread (P7 finding 8).
   - **`spec/README.md` and CONFORMANCE §2** should say the screen family is at
     `screen_parser_version` 1 with a Rust ledger, except `units/` at 0 and `hand/` at 1 with
     `oracle: "hand"`. `conformance.py list` shows this.
   - **No doc outside this file names `screen::frozen`**: `grep` finds it in no `.md` under `docs/`.
7. **For P12.** `spec/`'s changes are the 35 re-recorded screen documents and
   `spec/declarations/screen-v1.json`, all untracked until P12 stages them. The `data/` digest to
   compare is still P4's.
8. **Verified green.**
   - **Rust.**
     - `cargo test` (whole workspace): 725 passed, 0 failed, across 47 binaries. That is P9's 728,
       less the 4 `frozen` and 2 allowance tests, plus the 3 new `removed` tests.
     - `cargo clippy --all-targets` gave 0 warnings, and `cargo fmt --check` passed after a
       `cargo fmt`.
     - `cargo doc -p screen -p golf-core --no-deps` gave 0 warnings.
     - `golf-core rerecord --declare spec/declarations/screen-v1.json` ran in all three forms:
       `--dry-run` (35 changed, 0 written), the real run (35 written), and again (0 changed, 0
       written).
   - **Python, because `spec/` changed.**
     - `pytest tests/test_conformance.py tests/test_docs_truth.py`: 317 passed.
     - `conformance.py check` exited 0 and counts 48 screen vectors.
   - **Untouched.** `git diff -- src/` is empty, and the `data/` digest is P4's.

### P11 — docs (2026-10-02)

1. **What landed, file by file.**
   - **`docs/CONFORMANCE.md`**:
     - **The header** names M34's family.
     - **§1.** `profiles.json` is "the frozen screen parser's", forked by `crates/screen`. A paragraph
       now names `crates/screen/profiles.json` beside `devices.json` as Rust-read data with Rust pins.
     - **§2's intro** has six families. The ledger paragraph says an entry is keyed by the
       declaration's version key and a screen entry carries `removed`. The declarations paragraph
       says exactly one version key picks the family, and lists every load-time refusal.
     - **§2's format section** drops its case count (`check` prints it: 8,505) and gains the five M34
       tables. Its table has an "implemented by" column, and a paragraph says that `implemented_by`
       absent means `pyfmt` (P2 finding 2) and names the three readers.
     - **A new §2 "Screen" section** gives the sub-family table, the two shapes, the version key, how
       the family reached v1, the runners, the two pins to know (bits, `units/`) and what reaches no
       vector.
     - **§3** has a second list, "The screen parser's edges", with each edge, where it is solved and
       what gates it. The engine's own `sum()` sites are recorded there as routed, not counted.
     - **§4** covers the commands (both `rerecord` forms, `parse-screen`, `--screen-once`). The
       `rerecord` section covers family selection, `removed` with the two rejected options, the list
       rule, the guards, and M34's run as a second worked example. `check` gets the screen deferral,
       `regenerate --screen-once` gets its own paragraph, and there is a new `parse-screen`
       subsection.
     - **§5's tier-2 row** reads **Done, M34**. The crate count is nine, and "What this does not
       cover" gains the recognizer.
   - **`docs/decisions/014-…md`**: a fourth addendum covering the fork, the tie rule as built,
     planning finding 4's erratum, what the re-read changed, the filter, and `scale` dropped.
   - **`docs/decisions/032-…md`**:
     - a fifteenth addendum, covering the `pyfmt` move and nine crates, the parser's edges as built
       (three found beyond the twelfth's six), the engine's `sum()` question, the first family
       recorded once and the first hand vectors, the second version key and `removed`, and what the
       run measured;
     - the Status block, which now has "fifteen addenda", a summary of the new one, and "nine since
       M34";
     - one italic forward pointer under the twelfth addendum's "`pyfmt` moves out of `analysis`,
       pending the ledger", so that its "planned, not decided" is not read as current.
   - **`docs/README.md`** (not in P11's file list, and forced by docs-truth):
     - the addenda total, 97 → 99;
     - the ADR-014 row, 3 → 4, and the ADR-032 row, 14 → 15, each with a summary of its new addendum;
     - one sentence in the CONFORMANCE row.

     **The document count is untouched**, because it is P12's.
   - **`docs/ARCHITECTURE.md` §1**: the commands block. It adds `--screen-once`, the screen
     `rerecord`, and `parse-screen`. The "Rust half" comment says nine crates and describes `screen`
     (wired to nothing, `golfer_warnings`) and `pyfmt`, and `cargo test`'s family list is corrected,
     because it had the format table under `analysis`.
   - **`CLAUDE.md`**:
     - the intro's "none of that is built yet" is now "only its first pieces are built … wired to
       nothing";
     - `parse-screen` and the screen re-record are in Commands;
     - "Nine crates", with a paragraph for `screen` and `pyfmt`;
     - the `ANALYSIS_VERSION` invariant gains its `SCREEN_PARSER_VERSION` twin;
     - in the routing table, the conformance row names the screen family, the port row says nine
       crates and points at the second edge list, and the Rust row gives `crates/pyfmt` and
       `crates/screen` entries of their own and says `rerecord` and `parse-screen` are in
       `crates/core`.
   - **`spec/README.md`**:
     - the tree gains `screen/` with its sub-families, `declarations/` names `screen-v1.json`, and
       the format comment names the parser edges;
     - the crates that run each family are listed, which corrects `format/` from `analysis` to
       `pyfmt`/`screen`;
     - the version each family ages on, the screen ledger, and the declaration's two version keys
       and `removed`;
     - "do not edit a vector by hand", with the hand-worked exception, and `--screen-once` among the
       refusals.
   - **`tests/test_conformance.py`**: the `_PACKAGE_DATA` comment, and the `_PYTHON_RECORDED_SCREEN`
     comment, which now says P10 re-recorded three of the four.
   - **`docs/plans/m31-m40-shot-first-pivot.md` §M34**: four italic "as built" or "decided" notes,
     under the crate split, after the order's step 4, under the fork question, and under the
     capability pin and filter. P0's head pointer already named the margin, `scale`, located-unread
     and the fork.
   - **`crates/analysis/src/lib.rs`**: the stale "2,681 cases" (P2 finding 10). It now reads "2,681
     cases at P3 and grown since". The P3 history is kept, and it is no longer a false present-tense
     count.
2. **A correction to P10 finding 6, found by reading the files.** It says the screen family is at
   v1 "except `units/` at 0 and `hand/` at 1 with `oracle: "hand"`". That is right, but
   **`oracle` was not written by the re-record.** P4 wrote `oracle: "python"` on every
   Python-recorded vector when it recorded them, `units/` included (call 1). So all 48 carry
   `oracle`. `units/` has it with no ledger, and the 35 documents have it with one ledger entry each.
   The docs say this, and they do not say "`units/` carries neither key".
3. **The done-when grep is clean of stale claims.** What `grep` still finds for `analysis::pyfmt`,
   "OCR stays Python" and "seven crates" is dated record, each read and left:
   - ADR-032's twelfth addendum, which now carries the forward pointer;
   - ADR-034's Consequences;
   - the M23, M31 and program plans, where each instance quotes a question that is now answered
     beside it;
   - ROADMAP's M22 and M23 history, including `ROADMAP.md:2987`'s "Seven crates now" in §M23's dated
     Status line, which is for P12 to judge with the rest of ROADMAP;
   - WORKLOG.

   `crates/pyfmt`'s own docs name the old path as where it came from.
4. **Left alone on purpose.**
   - **ARCHITECTURE §2's "Six of these modules now have a Rust counterpart" (M22).** The plan names
     §1 only, and §2's sentence is dated and still true of M22's crates.
   - **ARCHITECTURE's "reviewed 2026-09-27" banner.**
   - **The program plan's other mentions of the filter and pin.** These are the M32, M38 and M29
     sections that P9 finding 6 listed. Each says what a later milestone calls, and that is still
     true.
   - **`m32-shot-contract.md:1073`.** It is a plan and a dated record.
5. **For P12.**
   - `ROADMAP.md` still says M34 is in progress, and the program plan's status checklist row says
     🔄. Both are P12's, as are the WORKLOG and the memory file.
   - **The WORKLOG's "What M29, M35 and M38 should know"** should carry four things:
     - the misread rule (P8 finding 3);
     - `screen::read`'s `Result` and its required `min_confidence` (P6 finding 2);
     - `golfer_warnings`;
     - the engine `sum()` question (P2 finding 8, now in ADR-032's fifteenth addendum and
       CONFORMANCE §3).
   - The `docs/README.md` document count still waits for this file to be staged.
6. **Verified green.**
   - **Python.**
     - Full `pytest`: 2,026 passed, as at P4.
     - `pytest tests/test_docs_truth.py` passed (92), re-run after the last doc edit.
     - `ruff check src tests scripts` passed. `ruff format --check` flags `tests/test_conformance.py`,
       but only on lines that are already in `HEAD`, not on this phase's comments.
   - **Rust**, because a doc comment in `crates/analysis` changed.
     - `cargo test`: 725 passed, 0 failed, across 47 binaries, as at P10.
     - `cargo clippy --all-targets` gave 0 warnings, and `cargo fmt --check` passed.
     - `cargo doc -p analysis --no-deps` raises its 28 older warnings and none in `lib.rs`.
   - **Untouched.** `git diff -- src/` is empty. No file under `spec/` changed except `spec/README.md`,
     and `data/` was not read. Every edited file kept its own line endings, which were checked by
     reading bytes.
