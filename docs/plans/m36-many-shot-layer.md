# Plan: M36 — The many-shot layer in Rust

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-10-07. **Milestone**: [ROADMAP §M36](../../ROADMAP.md#m36-the-many-shot-layer-in-rust).
**The design is the program plan's
[§M36](m31-m40-shot-first-pivot.md#m36--the-many-shot-layer-in-rust)**, re-detailed Rust-only by M31.5
P6b, with the M31.5 plan's inventory rows R3, R7, R9, R13, R16, R25 and R29 and its Q7
([m31-5-rust-first-replan.md](m31-5-rust-first-replan.md)). This file is its phase list. It does not
restate that section: each phase names what it builds and what to read. Where this file and §M36
disagree, this file's interview decisions, findings and calls say why, and this file wins.

**Governing decisions**: [ADR-035](../decisions/035-rust-everywhere-python-where-required.md) clauses 3
(the oracle moves to Rust: record once from frozen Python, then Rust re-records, diff-gated) and 4 (the
frozen lab, and the recorder as its one sanctioned change); [ADR-024](../decisions/024-per-club-shot-history.md)
(per-club history, the corpus) and ADR-028 (mishits); ADR-026 (club specs, the catalogue);
[ADR-032](../decisions/032-the-rust-core.md) §3 (portability edges) and §5 (one copy on disk);
ADR-008 (as cargo edges); ADR-010 §2 (no number beats a wrong one, which is why a refusal is ported to
the letter).

**Level: L3.** A new crate (`crates/storage`), nine contract modules, two new vector families, a new
version key in `golf-core rerecord`, a semantic change to the corpus's `OUTDATED` rule, and five CLI
verbs. Every phase reads its own list below rather than the repo.

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot — the findings are what the next phase
   is planned against;
3. stop, and tell the user to `/clear` and run `/next-phase`.

**Nothing is committed until P18** (the user's call, 2026-10-07). Every phase leaves its work in the
working tree, and P18 makes one commit onto `main`.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document, and the pointers to it | ✅ Done *(2026-10-07)* — approved by the user |
| **P1** | Recorder I: the corpus cases and the real corpus, built but not written | ✅ Done *(2026-10-07)* |
| **P2** | Recorder II: the store operations, and the storage family recorded once | ✅ Done *(2026-10-07)* |
| **P3** | Recorder III: the career family and the report text, recorded once | ✅ Done *(2026-10-07)* |
| **P4** | Foundations: `pyfmt`'s new edges and `contracts::time::Timestamp` | ✅ Done *(2026-10-07)* |
| **P5** | `crates/contracts` I: club, club spec, bag, mishit, golfer, the catalogue | ✅ Done *(2026-10-08)* |
| **P6** | `crates/contracts` II: career and baseline | ✅ Done *(2026-10-08)* |
| **P7** | `crates/contracts` III: dispersion, comparison and club profile | ✅ Done *(2026-10-08)* |
| **P8** | `crates/storage` I: the crate, the manifest, the state readers, the golfer and shot stores | ✅ Done *(2026-10-08)* |
| **P9** | `crates/storage` II: the bag store and the bundle store, reads and writes | ✅ Done *(2026-10-08)* |
| **P10** | `crates/storage` III: `read_corpus`, faithfully | ✅ Done *(2026-10-08)* |
| **P11** | `crates/analysis` I: the CI helpers and the baseline | ✅ Done *(2026-10-08)* |
| **P12** | `crates/analysis` II: dispersion, comparison and the bag profile | ✅ Done *(2026-10-08)* |
| **P13** | `golf-core rerecord` gains the career and storage families, under `CAREER_VERSION` | ✅ Done *(2026-10-08)* |
| **P14** | The change: `COMPARABLE_FROM`, hand-worked vectors, and the first career re-record | ✅ Done *(2026-10-08)* |
| **P15** | Verbs I: `career-corpus`, `career-baseline`, `career-dispersion` | ✅ Done *(2026-10-08)* |
| **P16** | Verbs II: `club-profile`, `flag-mishit`, and the parity run over `data/` | ✅ Done *(2026-10-08)* |
| **P17** | Docs: CONFORMANCE, the ADR addenda, ARCHITECTURE §1, `CLAUDE.md` | ✅ Done *(2026-10-08)* |
| **P18** | Close: verify, WORKLOG, ROADMAP, memory, and the one commit | ✅ Done *(2026-10-08)* |

---

## Rules every phase keeps

- **Every phase ends green on everything it can reach.** If a phase cannot end green, stop and
  record why as a finding; do not carry a red suite into the next session. A vector a phase cannot
  pass yet is skipped *by name* in the gating test, with the phase that will pass it, never by a
  blanket ignore.
- **Frozen Python is untouched outside `scripts/conformance.py`, `scripts/conformance_vectors.py`
  and `tests/test_conformance.py`** (ADR-035 clause 4). The five scripts under R29 are read and
  *imported* by the recorder, never edited. `git diff -- src/` is empty at every phase.
- **Nothing under `data/` is written.** `data/processed/` is gitignored, so `git status` cannot
  prove it. P1 snapshots the sha256 of every file under `data/processed/sessions/`,
  `data/processed/golfers/` and `data/processed/shots/` to the scratchpad before anything runs, and
  copies a count and a combined digest into its findings. P2, P3, P10, P16 and P18 compare against
  it. **P16's `flag-mishit` write test runs on a temporary copy**, never on `data/`.
- **Only these write `spec/vectors/`**: `conformance.py regenerate --storage-once` (P2, once),
  `regenerate --career-once` (P3, once), `regenerate --format-only` (P4, the format family, which
  records CPython and pydantic), and from P13 only `golf-core rerecord`. P14's hand-worked vectors
  are written by hand (or by a throwaway script in the scratchpad that the phase deletes). Any other
  change under `spec/vectors/` is a finding and a stop. **The engine, stage and screen families do
  not change in this milestone at all**: `git diff -- spec/vectors/{synthetic,corpus,stages,screen}`
  is empty at every phase.
- **New code is Rust.** The Python changes are conformance tooling only (P1–P4).
- **ADR-008 as cargo edges.** `crates/storage` depends on `contracts` (and `pyfmt` if a phase needs
  it), never on `analysis`. `analysis` never depends on `storage`. `crates/core` is the one crate
  holding `storage`, `analysis` and `feedback` together, so the verbs and the family gates live
  there. `contracts` gains **no** dependency: the timestamp is hand-rolled (decision 6).
- **Dependencies are named, with why**, in the `Cargo.toml` that takes them, as `rustfft` is: `sha2`
  and `unicode-normalization` in `crates/storage`, `tempfile` as a dev-dependency only. Nothing
  else without a finding first.
- **House style** (`CLAUDE.md` §House style, `docs/CODE_STANDARDS.md`): comments say *why* and carry
  the measurement or the rejected alternative. Each Rust module's doc names the Python module it
  ports, as `crates/analysis/src/stats.rs` does. Python docstrings that carry a reason travel with
  the code, condensed rather than dropped.
- **Do not write counts into prose** that a test or a registry owns. Where this plan quotes a count
  (15 directories, 13 swings, the n-gates), it is a snapshot for the phase to re-measure.
- **The doc-count pin reads `git ls-files`**, so this file is invisible to it until P18 stages it.
  Do not touch `docs/README.md`'s document count before P18. This file avoids the
  `**Status: … N/M phases` spelling on purpose (`tests/test_docs_truth.py::_PHASE_STATUS`).
- **Line endings**: the plain `.json` vectors Python writes are CRLF, and `rerecord`'s writer keeps
  each file's own. Git Bash heredocs mangle `—` and `°`; use the Edit tool or Windows Python for
  in-place edits.

## Verify — the commands, and when each runs

```bash
cargo test                                          # every phase that touches crates/
cargo clippy --all-targets && cargo fmt --check     # every phase that touches crates/
.venv/Scripts/python.exe -m pytest                  # P1–P4, P17, P18 (and any phase touching Python or docs)
.venv/Scripts/python.exe -m ruff check src tests scripts   # P1–P4, P18
.venv/Scripts/python.exe -m mypy src                # P18 (M36 changes nothing under src/)
.venv/Scripts/python.exe scripts/conformance.py check      # P1–P4, P18
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py # every phase that edits a doc or moves a path
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/career-v1.json --dry-run   # P14, P18
```

---

## Decisions taken in the interview (2026-10-07)

1. **`OUTDATED` becomes "not comparable", after a faithful port.** The order is M34's: record frozen
   Python's rule, port it faithfully so the recorded vectors gate it, then change it in Rust (P14)
   against hand-worked vectors and a declared re-record. After the change a swing is `outdated` in
   the corpus — excluded and named, as today — when its `analysis_version` is older than
   **`COMPARABLE_FROM`**, the oldest engine generation whose stored numbers today's engine still
   agrees with. It is no longer "older than the installed `ANALYSIS_VERSION`". The question
   `scripts/reanalyze.py` asks, *is there a newer engine to run*, stays `is_outdated` in the state
   readers, faithful, for M29's re-analysis verb. The corpus asks the other question.
2. **`COMPARABLE_FROM = 14`.** Frozen Python's ledger (`contracts/swing.py`) classes every bump: one
   whose older artifacts **disagree** about a number, or one whose older artifacts are only
   **missing** new measurements. 13 → 14 (the heavy pose model) is the newest that disagrees. 14 → 15
   (flight) and 15 → 16 (pivots) are *missing* bumps, measured byte-identical on every
   `overall_score`, and 16 → 17 is Rust's shape-only bump. A missing measurement already shows as a
   smaller per-metric `n`, which is honest. From 17 every Rust ledger entry classifies itself
   (`shape`, `missing` or `disagrees`), and a test holds `COMPARABLE_FROM` to the newest `disagrees`
   entry, or 14 when there is none. No v14 or v15 artifact exists on disk, so `data/` pools the same
   swings under 14 as it would under 16.
3. **The verbs are `golf-core` subcommands, printing the same text.** One per R29 script, named for
   it: `career-corpus`, `career-baseline`, `career-dispersion`, `club-profile`, `flag-mishit`, with
   the scripts' flags. Each prints the report its script prints, and the text is gated twice: by
   report text recorded into the career family (call 3), and once by a byte-for-byte parity run of
   all five against the Python scripts over `data/` (P16). Each read verb also takes `--json`, which
   prints the aggregate it reported on. M37's `golf-core profile` is the precedent for the home.
4. **The whole bundle store ports**: the reads, `assign_from_path` with the explicit `swing_id`
   target it already has (finding 1), `set_player`, `set_club`, `set_mishit`, `attribute_unlabeled`
   and `delete_swing`. M35 and the phone need the writes.
5. **One commit at the close** (P18), straight onto `main`, as M32 and M34 landed.
6. **Dependencies: a hand-rolled timestamp, crates for the rest.** `contracts::time::Timestamp`
   writes pydantic's exact spelling. `chrono` was declined: its default serializer writes `.300` for
   `.300000`, so it would need a custom serializer to match pydantic anyway, and it would be the first
   dependency `contracts` takes beyond `serde`. `sha2` (the manifest's and the shot store's hashes)
   and `unicode-normalization` (`slugify`'s NFD pass) go in `crates/storage`, and `tempfile` is
   dev-only.
7. **One vector from the real corpus**, beside the synthetic ones. Its tree is `data/`'s manifests,
   states, session files, golfer record and bag, with each `analysis.json` slimmed to the keys
   `read_corpus` reads (P1 proves the slim tree reads identically). Its storage expectation is
   `read_corpus` over it, and its career expectation is every aggregate and every report over that
   corpus and the real bag.
8. **`CAREER_VERSION`, one new version key for both families.** It lives in
   `crates/contracts/src/career.rs` and starts at 0, which the Python-recorded vectors carry. A
   `career-v<N>.json` declaration re-records `spec/vectors/storage/` and `spec/vectors/career/`
   together, as `screen_parser_version` picks the screen family. **`read_corpus` takes the engine
   versions as a parameter** (call 7), recorded in each vector's input, so an `ANALYSIS_VERSION` bump
   never moves either family. M35 is the first consumer: its `read_corpus` change is `career-v2`.

## What the planning read found (2026-10-07)

Measured on this box. Re-measure before leaning on a number.

1. **The bundle store's explicit target already exists.** `SwingBundleStore.assign_from_path`
   takes `swing_id: str | None`, and when it is given the file goes to that swing, created if absent
   (`storage/bundle_store.py:121–148`). `api/app.py:675` passes the upload's `?swing_id=`. It has been
   there since 2026-08-07 (`9508c59`). So the program plan's "the bundle store gains an explicit
   target … new behaviour, with hand-worked vectors" (§M36, from "What the code says" finding 9) is
   faithful behaviour: it is ported and recorded like the rest, and **no hand-worked explicit-target
   vectors are written**. Finding 9's real point stands: the *default* ("the newest swing lacking
   this role") misattaches in mixed sessions, so M35 and the phone should always pass the target.
   P17 corrects §M36's line.
2. **`data/` today**: 15 swing directories across 15 session directories, every `analysis.json` at
   `analysis_version` 16. `scripts/career_corpus.py` reports 13 distinct swings and 13 distinct
   shots for `aaron`, with 2 re-uploads collapsed into `2026-08-07-aaron1/1`. Rust's
   `ANALYSIS_VERSION` is 17, so a faithful Rust `read_corpus` excludes all 15 as `OUTDATED` and pools
   nothing. That is decision 1's reason.
3. **Every timestamp on disk is `YYYY-MM-DDTHH:MM:SS.ffffffZ`**: `created_at`, `updated_at`,
   `received_at`, `recorded_at`, `retired_at` and `retrieved_at`, across the golfer, the bag, the
   manifests, the session files and `club_catalogue.json`. pydantic writes UTC as `Z` and drops the
   fraction when it is zero, so `Timestamp` must write `…:58Z` for a whole second, not `…:58.000000Z`.
   P4 records the spelling from pydantic rather than trusting this note.
4. **The gates the career family must cross** (`contracts/baseline.py:93–138`, `contracts/mishit.py`):

   | floor | CENTER | SPREAD | TREND | sessions (C/S/T) |
   |---|---|---|---|---|
   | default | 5 | 10 | 12 | 1 / 1 / 3 |
   | `tempo_ratio` | 8 | 15 | 18 | 1 / 1 / 3 |
   | `hip_shift_at_top_norm` | 6 | 12 | 14 | 1 / 1 / 3 |
   | mishit floor | `MISHIT_MIN_CLEAN_SHOTS` = 5 distinct carries per club | | | |

   "Crossing" means a case at `n − 1` and at `n` for each, so the program plan's 4/5/9/10/11/12 is
   the default row only. The override rows and the sessions gate (2 and 3) need cases of their own.
5. **CPython edges in this layer** (re-grep before relying on the list):
   - `sum()` over floats is compensated on CPython 3.12+ (the M34 plan's finding 5):
     `analysis/stats.py::mean_and_sd` (both sums) and `analysis/baseline.py::_session_means`.
     `pyfmt::sum` already reproduces it.
   - `x ** 2` in `mean_and_sd`, `sd ** 2` in `sd_ci`, and `z ** 3`, `z ** 5`, `df ** 2` and `(…) ** 3`
     in the critical-value expansions. A float `**` is C `pow`, which `powi` does not promise to
     match. The phase measures `x * x`, `powi` and `powf` against the vectors.
   - `statistics.median` (`contracts/mishit.py:82`): sort, then the middle value or the mean of the
     middle two.
   - `f"{x:.3g}"` (`analysis/dispersion.py:208–209`). `pyfmt::g` is precision 6 only.
   - `round(placement, 1)` (`analysis/comparison.py:124`): `pyfmt::round_to`.
   - `str.lower()` and NFD with `unicodedata.combining` (`contracts/golfer.py::slugify`), and
     `str.lower()` in `contracts/club.py::_normalize` and `club_spec.py::_normalize`. `pyfmt` has
     `upper` and no `lower`.
   - The scripts' text: `:<N`, `:>N`, `:<{width}` padding (code points), `str.split()` in the
     scripts' greedy `_wrap` (Unicode whitespace: `pyfmt::split`), and `len()` in code points.
   - `datetime` comparison across the corpus (`captured_at < recorded_at`) and `f"{dt:%Y-%m-%d}"`
     (`analysis/club_profile.py`), both in the datetime's own offset.
6. **`crates/analysis/src/stats.rs`'s doc is stale**: it says the CI helpers "go with their callers,
   in §M29". They go here (P11 fixes the doc).
7. **`crates/analysis/src/engine.rs` holds private copies of the source strings**
   (`POSE_DTL_SOURCE` at `:93`, `POSE_FACE_ON_SOURCE` at `:83`), where Python's engine imports
   `contracts.career.POSE_DTL_SOURCE`. P6 makes `contracts::career` the one definition.
8. **Already ported, and reused**: `benchmarks::distributions::load_distribution`
   (`crates/analysis/src/benchmarks/distributions.rs`), which `comparison` needs;
   `contracts::intent::ClubCategory`, which `club.py::CLUB_CATEGORY` maps onto;
   `contracts::golfer::Handedness`; `contracts::swing::Measurement` and `ANALYSIS_VERSION`;
   `contracts::shot::ShotData`; `pyfmt::{sum, round_to, fixed, signed_fixed, percent, split, strip}`;
   `crates/core::compare` (§3's comparator).
9. **Not in the workspace yet**: `chrono`, `sha2`, `unicode-normalization`, `tempfile` (none in
   `Cargo.lock`). `ClubId` has no Rust twin.
10. **The scripts' reports are reachable as functions.** `career_corpus._report(corpus, name, *,
    verbose)`, `career_baseline._report(baseline, standing, name, *, verbose)`,
    `career_dispersion._report(dispersion, name, *, verbose)`, `club_profile._report(profile, name,
    *, verbose, club)` and `flag_mishit._list(store, golfers, only)`. The first four print from
    aggregates alone, so the recorder captures them with `redirect_stdout`. `_list` reads stores, so
    it is captured over a materialised tree.
11. **`read_corpus` reads three things from `analysis.json`**: `analysis_version`,
    `swing.measurements` and `swing.shot` (`storage/corpus.py:194–228`, through
    `api/state.py::{load_analysis, stored_analysis_version, is_outdated}`). That is what makes the
    real vector's slimmed analyses faithful (decision 7).
12. **`conformance.py check` defers four families today** (audio, stages, format, screen). Storage
    and career become the fifth and sixth, deferred with no `ANALYSIS_VERSION` test, as screen is.
13. **Out of scope, though it reads the layer**: `analysis/tempo_trainer.py` imports `contracts.
    baseline` and `contracts.career`. It ports with tempo in M40 (R4), and nothing here touches it.

## Calls this plan makes where §M36 left it open

Each is contestable before the phase that builds it; after that, a finding is the way to change it.

1. **The storage vector's shape.**
   - **Corpus cases** (`spec/vectors/storage/corpus/<case>.json`): `input` is `{player_id,
     versions: {installed, comparable_from}, files: {"<session>/<swing>/<name>": "<text>"}}`, where
     `files` is the sessions directory as **raw text**, so a corrupt manifest or a half-written state
     file is a case like any other. `expected` is `{corpus: CareerCorpus, narrowed: {<name>:
     {args, corpus}}}`, `model_dump(mode="json")`. Python records `versions` as `{16, 16}`.
   - **Operation cases** (`spec/vectors/storage/bundle/`, `spec/vectors/storage/stores/`): `input` is
     `{files, ops: [{op, args, now}]}`, `expected` is `{results: [...], files: {...}}` — each op's
     return value, then the whole tree after the last op. A JSON file in `expected.files` is compared
     as a value through `crates/core::compare`, anything else as text. Byte identity of the JSON a
     store writes is **not** pinned (key order and float spelling are not what frozen Python reads
     by); value equality with what pydantic wrote is.
   - **The real corpus** (`spec/vectors/storage/corpus/real.json`, gzipped if P1 finds it over
     ~200 KB, as `spec/vectors/corpus/` is).
   - `provenance`: `{oracle: "python", career_version: 0, analysis_version: 16, python_version,
     pydantic_version, recorded_by}`.
2. **The career vector's shape** (`spec/vectors/career/{synthetic,real}/<case>.json`): `input` is
   `{corpus: CareerCorpus, bag: Bag | null, display_name, club: ClubId | null}`, and `expected` is
   `{baseline, dispersion, standing, bag_profile, reports: {<script>[_verbose][_club]: "<stdout>"}}`.
   The real case's `input.corpus` is the real storage vector's `expected.corpus`, copied, so the two
   families cannot drift apart silently: P3's recorder asserts they are equal.
3. **The report text is recorded, so it stays gated after M29 deletes the scripts.** The recorder
   imports each script by path (`importlib`, as it loads `tests/analysis/conftest.py`) and captures
   its `_report` with `redirect_stdout`. Every synthetic career case records each report plain and
   `--verbose`; `club_profile` also once per club the case has. `flag_mishit --list` is recorded over
   the real tree only. Rust's renderers are gated by `crates/core/tests/reports.rs`.
4. **The clock is an argument.** Every Rust store operation that stamps a time takes `now:
   Timestamp` from its caller, and the verbs pass `Timestamp::now_utc()`. The recorder freezes
   Python's clock by replacing the `datetime` name in each store module with a subclass whose
   `now()` returns the op's `now`, for the recorder's run only. `src/` is not edited.
5. **The family gates live in `crates/core/tests/`** (`storage.rs`, `career.rs`, `reports.rs`), beside
   `engine.rs`, because the comparator is `crates/core::compare` and `core` is the crate that may
   depend on everything. `storage` and `analysis` keep their own unit tests, mirroring the Python
   tests' cases, and do not depend on `core`.
6. **Where each shape lives in Rust**, by its Python module, not by what it is:
   - `crates/contracts`: `club` (`ClubId`, `CLUB_CATEGORY` onto `intent::ClubCategory`,
     `parse_club`), `club_spec`, `bag`, `mishit`, `golfer` (gains `Golfer` and `PLAYER_ID` beside
     `Handedness`), `catalogue` (`clubs/catalogue.py`'s reader; R25 ports with the bag), `career`,
     `baseline`, `dispersion`, `comparison`, `club_profile`, and `time` (new, no Python twin).
   - `crates/storage`: `manifest` (`Role`, `RoleFile`, `SwingManifest`, the hashes, load/save),
     `state` (`api/state.py`'s `AnalysisState`, `input_hashes`, `load_state`, `save_state`,
     `load_analysis`, `stored_analysis_version`, `is_outdated`), `golfer_store` (with `slugify`, which
     needs `unicode-normalization` and so cannot sit in `contracts`), `bag_store`, `shot_store`
     (`launch_monitor/screen/store.py`), `bundle_store`, `corpus`.
   - `crates/analysis`: `stats` (gains `mean_and_sd`, `mean_ci`, `sd_ci` and their tables),
     `baseline`, `dispersion`, `comparison`, `club_profile`.
   - `crates/core`: the five verbs and their renderers (`src/reports/`), and the family runners
     `rerecord` calls.
7. **`read_corpus(sessions_dir, player_id, versions: EngineVersions)`**, where `EngineVersions` is
   `{installed, comparable_from}`. The faithful port (P10) applies `stored < installed`, exactly as
   Python, and ignores `comparable_from`; P14 switches the rule to `stored < comparable_from`. The
   verbs pass `{ANALYSIS_VERSION, COMPARABLE_FROM}`. The faithful vectors carry `{16, 16}`, under
   which both rules agree, so only the `OUTDATED` detail sentence can move at P14, and the
   declaration names each one.
8. **Schemas stay Python-exported.** No shape moves in M36, so `swing_manifest`, `golfer`, `bag` and
   `analysis_state` stay out of `RUST_OWNED_SCHEMAS`, and `regenerate --schemas-only` keeps writing
   them. Rust pins its structs' key sets against those four committed schemas, on
   `crates/contracts/tests/schemas.rs`'s precedent. A root becomes Rust-owned when Rust first moves
   its shape (M35's `face_on_sha256`, or later). §M36's "revisits `schemars`" is answered **no**: no
   hand-maintained schema is added here, so the cost it would save has not grown.
9. **Tolerant readers read what pydantic writes, and pydantic's lax coercions are a named
   divergence.** Every Rust reader that Python's returns `None` from returns `None` on the same file:
   missing, unreadable, invalid JSON, a missing required key, a value out of bounds. Unknown keys are
   ignored, as pydantic's `extra="ignore"` does. What pydantic *also* accepts and nothing here writes
   — a numeric string for an int, a naive or Unix-number datetime, a bare date — Rust refuses, and so
   reads that file as `None`. P8 states it in `storage`'s crate doc, and no vector covers it.
10. **The catalogue crosses by `include_str!`** of `src/golf_coach/clubs/club_catalogue.json`, one
    copy on disk, as the benchmarks do (ADR-032 §5). A row `clubs/lookup.py` remembers reaches Rust
    at the next build, and the module doc says so. Two pins (§M36's P2 finding 4): every committed row
    parses, and every value `contracts/club_spec.py` can write (each `ShaftMaterial`, each
    `ShaftFlex`, and a row with every optional key absent, since the file is written with
    `exclude_defaults=True`) round-trips.
11. **The verbs' data directories**: `--sessions-dir` and `--golfers-dir`, defaulting to the
    `GOLF_SESSIONS_DIR` and `GOLF_GOLFERS_DIR` environment variables (pydantic-settings' names for
    `settings.sessions_dir` and `settings.golfers_dir`), else `data/processed/{sessions,golfers}`
    under the repo root, found from `CARGO_MANIFEST_DIR` as Python finds `REPO_ROOT`. `.env` is not
    read: it holds no directory override today, and P15 says so in the verb's doc.
12. **`ClubId`'s order is Python's declaration order**, because `BagProfile` and the reports iterate
    clubs in it. P5 pins the order against `contracts/club.py`.

---

## Phases

### P0 — This plan document, and the pointers to it

- **Goal**: this file, approved, and the two pointers to it.
- **Files**: `docs/plans/m36-many-shot-layer.md` (this, new); `ROADMAP.md` §M36 (a **Plan** line
  pointing here, and the status row's "Not started" gains the plan link); the program plan's status
  table row for M36 (`docs/plans/m31-m40-shot-first-pivot.md`).
- **Done when**: the user has approved the plan, the pointers resolve, and docs-truth is green.
- **Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P1 — Recorder I: the corpus cases and the real corpus, built but not written

- **Goal**: the half of `build_storage()` that exercises `read_corpus` and `narrow_to`, runnable as a
  dry run. Nothing under `spec/` is written in this phase (`--storage-once` records the whole family
  at P2, and refuses a second run).
- **First**: take the `data/` snapshot (Rules), before any recorder code runs.
- **Read**: `storage/corpus.py`; `api/state.py:119–160`; `storage/manifest.py`;
  `contracts/career.py` (whole); `tests/storage/conftest.py` and `tests/storage/test_corpus.py` (the
  case list); `scripts/conformance_vectors.py:1–60` and its screen section (`build_screen`,
  `_screen_vector`) for the pattern; `scripts/conformance.py`'s `regenerate` and `_kind`.
- **Recorder** (`conformance_vectors.build_storage_corpus`, and `_run_corpus(input) -> expected`,
  which is **the definition a port reproduces**, on `run_vector`'s precedent): materialise `files`
  into a temporary directory and call `read_corpus`, then `narrow_to` for each named narrowing.
  Synthetic cases, at least one per row:
  - an empty or missing sessions directory; an unknown player; `UNATTRIBUTED`; another golfer's
    swing;
  - `NO_FACE_ON`; `DUPLICATE` with three uploads, the survivor the earliest, and two manifests in the
    same second, which the tiebreak decides; conflicting shot photos across duplicates;
  - `NOT_ANALYZED`; `STALE`; `OUTDATED` at stored versions 0, 15, a non-integer, a negative and a
    bool; an analysis whose `swing` is not a dict;
  - a corrupt manifest (skipped), a corrupt analysis, a corrupt state, a measurement list with
    invalid entries;
  - a flagged shot, and a shot that no longer validates (reads as needing review);
  - auto-mishits at 4 and at 5 distinct carries on one club, two clubs with different medians, a
    `CONFIRMED` and a `CLEARED` verdict overriding the rule, a re-used photo counted once in the
    median;
  - an untagged swing; the `pose:down_the_line`, `model:`, an unknown source and `population:golfdb`;
  - the `captured_at`/session/swing sort; `narrow_to` by `since`, by sessions and by club.
- **The real corpus**: read every `data/processed/sessions/*/*/{manifest.json, analysis.state.json}`
  and `*/session.json`, and each `analysis.json` slimmed to `{analysis_version, swing:
  {measurements, shot}}` (finding 11). **Verify before using it**: `read_corpus` over the slim tree
  equals `read_corpus` over `data/` itself, or the builder raises. Player `aaron`.
- **Tooling**: `tests/test_conformance.py` gains a dry-run pin that builds the corpus cases in memory
  and checks each has an `expected` and the case names are unique. The real case is built only when
  `data/` exists, and skipped by name otherwise.
- **Files**: `scripts/conformance_vectors.py`; `tests/test_conformance.py`.
- **Done when**: the dry run builds every case; the slim-tree verify passes; the size of the real
  case is in the findings (and whether it is gzipped); pytest, ruff and `check` are green; `git diff
  -- src/ spec/` is empty; the `data/` snapshot is unchanged.
- **Verify**: the snapshot; `pytest tests/test_conformance.py`; `pytest`; `ruff`; `conformance.py
  check`.

### P2 — Recorder II: the store operations, and the storage family recorded once

- **Goal**: the operation cases, then the whole storage family written by `regenerate
  --storage-once`, which refuses a second run.
- **Read**: `storage/bundle_store.py`, `storage/bag_store.py`, `storage/golfer_store.py`,
  `launch_monitor/screen/store.py`, `contracts/golfer.py::slugify`, `contracts/bag.py`;
  `tests/storage/test_{bundle_store,bag_store,golfer_store,manifest}.py` (the case lists); P1's
  findings.
- **Recorder** (`build_storage_ops`, `_run_ops(input) -> expected`), with the clock frozen per op
  (call 4):
  - **bundle** (`storage/bundle/`): `list_session_ids`, `get_session` (the numeric swing sort),
    `get_swing`, `current_session_id`; `assign_from_path` into an empty session, into the newest
    swing lacking the role, with an explicit `swing_id` (existing and new), and a duplicate digest
    (deduped, tmp file removed); `set_player`, `set_club`, `set_mishit` (each verdict and `None`),
    `attribute_unlabeled`, `delete_swing`, and each on a missing swing.
  - **stores** (`storage/stores/`): the bag store's `get`, `save`, `set_entry`, `remove_entry`,
    `restore_entry` (including the validator refusals, recorded as the exception's class and
    message); the golfer store's `get`, `list_all`, `get_or_create` (an existing name in another
    case is the same golfer); the shot store's `put`, `get`, `has`, `all` (newest first, an
    unreadable file skipped, the `shot_id` key fallback); and a `slugify` table (`"Aaron "`,
    `"María"`/`"Maria"`, punctuation runs, leading and trailing hyphens, nothing left).
- **Tooling**: `conformance.py` gains `storage_vector_paths()`; `check` defers the family with no
  `ANALYSIS_VERSION` test and `list` shows it; `regenerate --storage-once` refuses with exit 2 on any
  file under `spec/vectors/storage/`, before the recorder is imported. `tests/test_conformance.py`
  pins the deferral and the refusal, on a temporary copy of `spec/`.
- **Then record**: `regenerate --storage-once`, once.
- **Files**: `scripts/conformance_vectors.py`; `scripts/conformance.py`; `tests/test_conformance.py`;
  `spec/vectors/storage/{corpus,bundle,stores}/*` (new).
- **Done when**: the family is written; a second `--storage-once` exits 2 and writes nothing; pytest,
  ruff and `check` are green; the `data/` snapshot is unchanged.
- **Verify**: `regenerate --storage-once` twice; `pytest`; `ruff`; `conformance.py check`;
  `conformance.py list`; the snapshot.

### P3 — Recorder III: the career family and the report text, recorded once

- **Goal**: `build_career()`, recorded by `regenerate --career-once`, which refuses a second run.
- **Read**: `analysis/{baseline,dispersion,comparison,club_profile,stats}.py`;
  `contracts/{baseline,dispersion,comparison,club_profile,mishit}.py`; `tests/analysis/test_{baseline,
  dispersion,comparison,club_profile_builder}.py` (their corpus helpers and cases); the five scripts'
  report functions (finding 10); P1–P2's findings.
- **Recorder** (`_run_career(input) -> expected`): `build_baseline`, `build_dispersion`,
  `build_standing`, `build_bag_profile(corpus, bag)`, and the reports (call 3). Synthetic cases:
  - every row of finding 4 at `n − 1` and `n`, and the sessions gate at 2 and 3;
  - a mishit withheld from carry and total but counted for ball speed; a flagged shot; re-uploads
    pooled once;
  - each `DispersionPattern` and the scatter-only reading; session drift on both sides of
    `SESSION_DRIFT_FACTOR`; each `METRIC_TARGETS` kind;
  - each `Standing`, a `TOUR_COMPARISON_BLOCKED` metric, a launch-monitor and a model metric with no
    population, spread not comparable;
  - the bag profile: no bag; a bag entry with no swings; the bag-changed caveat in both forms and at
    the equal instant (strict `<`); a retired entry; the mishit caveat singular, plural and with
    unconfirmed flags; untagged swings; the empty corpus;
  - and the real case (call 2), with the real bag and display name, whose `input.corpus` must equal
    the real storage vector's `expected.corpus` or the recorder raises.
- **The format scan**: list every format spec, padding and width in the five scripts and the four
  `analysis/` modules into the findings, for P4. Finding 5 is the starting list.
- **Tooling**: `career_vector_paths()`, deferred by `check` like storage; `regenerate --career-once`
  refuses on any file under `spec/vectors/career/`; the pins in `tests/test_conformance.py`.
- **Files**: `scripts/conformance_vectors.py`; `scripts/conformance.py`; `tests/test_conformance.py`;
  `spec/vectors/career/{synthetic,real}/*` (new).
- **Done when**: the family is written; a second `--career-once` exits 2; pytest, ruff and `check`
  green; `data/` unchanged.
- **Verify**: as P2.

### P4 — Foundations: `pyfmt`'s new edges and `contracts::time::Timestamp`

- **Goal**: every CPython or pydantic edge P3's scan found, solved before any caller, each gated by a
  format table.
- **Read**: `crates/pyfmt/src/lib.rs` (the existing edges, `g`, `upper`, `split`, `sum`);
  `scripts/conformance_vectors.py::build_format` and its `_format_*` builders; `docs/CONFORMANCE.md`
  §2 "Format"; P3's format-scan finding.
- **`pyfmt`**: `g` at a given precision (`:.3g`), `lower` (Unicode, as `upper` is), and anything else
  P3 found. Each gets rows in the format family, recorded by `regenerate --format-only`.
- **`contracts::time`** (new, no Python twin): `Timestamp` — parse RFC 3339 with a `Z` or `±HH:MM`
  offset and a 0–6 digit fraction, and refuse a naive one (call 9); write pydantic's spelling, keeping
  the offset it was read with; order by instant; `date_ymd()` in its own offset for `%Y-%m-%d`;
  `now_utc()` at microsecond precision. Serde through the string form.
- **A new format table, `timestamp.json`**, recorded by `--format-only` from pydantic: a sweep of
  datetimes (a whole second, a fraction with trailing zeros, six digits, a `+05:30` and a `-08:00`
  offset, the first and last instants of a year), each with pydantic's serialized string and its
  `%Y-%m-%d`, plus strings pydantic parses back to the same instant. Its provenance names the
  pydantic version beside `python_version`.
- **Files**: `crates/pyfmt/src/lib.rs`, `crates/pyfmt/tests/*`; `crates/contracts/src/{time,lib}.rs`,
  `crates/contracts/tests/time.rs`; `scripts/conformance_vectors.py` (the format builders only);
  `spec/vectors/format/*` (rows added, `timestamp.json` new).
- **Done when**: every format row passes in `cargo test`; the existing format rows are byte-identical
  (`git diff` shows only additions, or the regeneration moved nothing else); clippy, fmt, pytest and
  `check` green.
- **Verify**: `regenerate --format-only`; `git diff --stat spec/vectors/format`; `cargo test`;
  clippy; fmt; `pytest`; `conformance.py check`.

### P5 — `crates/contracts` I: club, club spec, bag, mishit, golfer, the catalogue

- **Goal**: the identity and equipment shapes, with pydantic's bounds and validators as `Validate`.
- **Read**: `contracts/{club,club_spec,bag,mishit,golfer}.py`; `clubs/catalogue.py` and
  `club_catalogue.json`; `crates/contracts/src/{lib,golfer,intent,capability}.rs` (the `Validate`
  pattern and the `include_str!` pattern); `crates/contracts/tests/{round_trip,schemas}.rs`.
- **Builds**: `club.rs` (`ClubId` in declaration order, `CLUB_CATEGORY`, `category_of`, `parse_club`
  with its alias table, using `pyfmt::lower`); `club_spec.rs` (`ShaftMaterial`, `ShaftFlex`, their
  parsers, `SpecProvenance`, `ClubSpec` and its validator); `bag.rs` (`BagEntry`, `Bag` and both
  validators); `mishit.rs` (`MishitVerdict`, `MISHIT_EXCLUDED_METRICS`, the floor constants,
  `mishit_carry_floor` with `statistics.median`'s rule); `golfer.rs` gains `Golfer` and `PLAYER_ID`;
  `catalogue.rs` (call 10).
- **Tests**: the Python tests' cases in `tests/contracts/test_{club,club_spec,bag,mishit}.py`,
  ported as unit tests; `parse_club` over every alias; the catalogue's two pins; the `bag` and
  `golfer` schema key-set pins (call 8); the bags inside `spec/vectors/career/` and the bag-store
  vectors round-trip value-equal.
- **Files**: `crates/contracts/src/{club,club_spec,bag,mishit,golfer,catalogue,lib}.rs`;
  `crates/contracts/tests/*`.
- **Done when**: all of the above green; clippy and fmt clean.
- **Verify**: `cargo test -p contracts`; `cargo test`; clippy; fmt.

### P6 — `crates/contracts` II: career and baseline

- **Goal**: the corpus and baseline shapes, with every rule they carry.
- **Read**: `contracts/career.py` and `contracts/baseline.py` (whole); finding 7; P5's findings.
- **Builds**: `career.rs` — the source prefixes and `KNOWN_SOURCE_PREFIXES` (and `population:` left
  out, with Python's warning carried), `ExclusionReason`, `ExcludedSwing`, `CorpusSwing`
  (`counts_toward_metrics`, `is_mishit`, `artifact_key`), `CareerCorpus` (every property, and
  `narrowed_to`), `count_metrics`, and `CAREER_VERSION = 0` with a doc that says what ages on it
  (decision 8). `baseline.rs` — `BaselineClaim`, the three floor tables, `minimum_n`,
  `minimum_sessions`, `Interval`, `WithheldClaim`, `SessionSample`, `MetricSample`, `MetricBaseline`,
  `PersonalBaseline`. `engine.rs` takes its source strings from `contracts::career` (finding 7).
- **Tests**: `tests/contracts/test_career.py`'s cases; every `CareerCorpus` and `PersonalBaseline`
  in both families round-trips value-equal; the engine family is unchanged (`cargo test -p core`).
- **Files**: `crates/contracts/src/{career,baseline,lib}.rs`; `crates/analysis/src/engine.rs`;
  `crates/contracts/tests/*`.
- **Done when / Verify**: as P5.

### P7 — `crates/contracts` III: dispersion, comparison and club profile

- **Goal**: the remaining shapes, and their prose tables, verbatim.
- **Read**: `contracts/{dispersion,comparison,club_profile}.py` (whole; `dispersion.py` is mostly
  `METRIC_TARGETS` and `PATTERN_READING`); P6's findings.
- **Builds**: `dispersion.rs` (the patterns, `PATTERN_READING`, `SCATTER_ONLY_READING`, the tuned
  and judged tuples, `METRIC_TARGETS`, `SESSION_DRIFT_FACTOR`, `MetricTarget`, `Finding`,
  `MetricDispersion` and its validator, `GolferDispersion`); `comparison.rs` (`Standing`,
  `STANDING_READING`, `TOUR_COMPARISON_BLOCKED`, the no-population sentences,
  `no_population_reason`, `MetricComparison`, `GolferStanding`); `club_profile.rs` (`ClubProfile`
  and its validator, `BagProfile`).
- **Tests**: `tests/contracts/test_club_profile.py`'s cases; a pin that `METRIC_TARGETS`' keys and
  every table's sentences equal the Python module's (read from the career vectors where they
  appear, and from a table the phase extracts into the test where they do not); every dispersion,
  standing and bag profile in the career family round-trips.
- **Files**: `crates/contracts/src/{dispersion,comparison,club_profile,lib}.rs`;
  `crates/contracts/tests/*`.
- **Done when / Verify**: as P5.

### P8 — `crates/storage` I: the crate, the manifest, the state readers, the golfer and shot stores

- **Goal**: the new crate and its simplest stores, gated by the `stores/` vectors that cover them.
- **Read**: `storage/{manifest,golfer_store}.py`; `api/state.py:1–160`;
  `launch_monitor/screen/store.py`; `tests/storage/test_{manifest,golfer_store}.py`; call 9;
  `Cargo.toml` (the workspace and how a dependency is introduced); `crates/core/tests/engine.rs` (how a
  family gate reads vectors).
- **Builds**: `crates/storage` in the workspace, its crate doc (what it ports, ADR-008's exception
  not carried over, call 9's divergence, the clock as an argument); `manifest.rs` (`Role`,
  `EXPECTED_ROLES`, `RoleFile`, `SwingManifest`, `missing_roles`, `status`, `hash_bytes`,
  `hash_file`, `content_filename`, tolerant `load_manifest`, atomic `save_manifest`); `state.rs`
  (call 6); `golfer_store.rs` with `slugify`; `shot_store.rs`. `sha2`, `unicode-normalization` and
  `tempfile` (dev) enter here, each named with why.
- **Gate**: `crates/core/tests/storage.rs` is created, runs the golfer-store, shot-store and `slugify`
  vectors, and skips the rest by name until P9 and P10. The `swing_manifest` and `analysis_state`
  schema key-set pins.
- **Files**: `Cargo.toml`; `crates/storage/{Cargo.toml,src/*}`; `crates/core/{Cargo.toml,
  tests/storage.rs}`.
- **Done when / Verify**: as P5, plus `cargo test -p storage`.

### P9 — `crates/storage` II: the bag store and the bundle store, reads and writes

- **Goal**: the two stores that write, gated by the operation vectors.
- **Read**: `storage/{bag_store,bundle_store}.py`; `tests/storage/test_{bag_store,bundle_store}.py`;
  P8's findings.
- **Builds**: `bag_store.rs` (`get`, `save`, `set_entry`, `remove_entry`, `restore_entry`, the
  rebuild-and-validate write); `bundle_store.rs` (decision 4's whole list; the lock as a `Mutex`; the
  tmp-and-rename writes; the numeric swing sort; `AssignmentResult`).
- **Gate**: `storage.rs` runs every `bundle/` and `stores/` vector.
- **Files**: `crates/storage/src/{bag_store,bundle_store,lib}.rs`; `crates/core/tests/storage.rs`.
- **Done when / Verify**: as P8.

### P10 — `crates/storage` III: `read_corpus`, faithfully

- **Goal**: `read_corpus` and `narrow_to` exactly as frozen Python behaves, `NO_FACE_ON` and the
  installed-version `OUTDATED` rule included (call 7).
- **Read**: `storage/corpus.py` (whole); `tests/storage/test_corpus.py`; P1's findings.
- **Builds**: `corpus.rs` with `EngineVersions`, `read_corpus`, `narrow_to`, and the private steps
  (`_corpus_swing`, `_conflicting_shots`, `_measurements`, `_flag_auto_mishits`, `_needs_review`,
  `_arrival`), each carrying its Python reason.
- **Gate**: `storage.rs` runs every `corpus/` vector, the real one included, with no skips left.
- **Also**: re-check the `data/` snapshot. A scratch run of the Rust `read_corpus` over `data/` with
  Rust's versions (`{17, 17}`) excludes every swing as `OUTDATED`; record the count as the "before"
  for P14.
- **Files**: `crates/storage/src/{corpus,lib}.rs`; `crates/core/tests/storage.rs`.
- **Done when / Verify**: as P8; the snapshot is unchanged.

### P11 — `crates/analysis` I: the CI helpers and the baseline

- **Goal**: `stats`' career half and `build_baseline`, gated by the career family's `baseline` key.
- **Read**: `analysis/{stats,baseline}.py`; `tests/analysis/test_baseline.py`;
  `crates/analysis/src/stats.rs`; finding 5.
- **Builds**: `stats.rs` gains `mean_and_sd`, `mean_ci`, `sd_ci`, the three critical-value tables and
  the expansions beyond them, with `pyfmt::sum` and the `**` choice the phase measures; its stale doc
  is corrected (finding 6). `baseline.rs`: `pooled_samples`, `build_baseline`, `refuse` and the claim
  phrasing.
- **Gate**: `crates/core/tests/career.rs` is created, checks `expected.baseline` on every career
  vector, and skips the other keys by name until P12. `analysis` gains `contracts`' career shapes
  only, never `storage`.
- **Files**: `crates/analysis/src/{stats,baseline,lib}.rs`; `crates/core/tests/career.rs`.
- **Done when / Verify**: as P5, plus `cargo test -p analysis`.

### P12 — `crates/analysis` II: dispersion, comparison and the bag profile

- **Goal**: the three remaining aggregates, and the career family green in full, reports aside.
- **Read**: `analysis/{dispersion,comparison,club_profile}.py`; their tests; P11's findings.
- **Builds**: `dispersion.rs` (`build_dispersion`, `dispersion_for`, the deciders, session evidence
  with its `:.3g` sentence through P4's `pyfmt`); `comparison.rs` (`build_standing`,
  `comparison_for`, `_place`, through `load_distribution`); `club_profile.rs` (`build_bag_profile`
  and both caveat builders, through `Timestamp`'s order and `date_ymd`).
- **Gate**: `career.rs` checks every key but `reports`, with no skips left.
- **Files**: `crates/analysis/src/{dispersion,comparison,club_profile,lib}.rs`;
  `crates/core/tests/career.rs`.
- **Done when / Verify**: as P11.

### P13 — `golf-core rerecord` gains the career and storage families, under `CAREER_VERSION`

- **Goal**: decision 8: a `career_version` declaration re-records `storage/` and `career/`, held to
  every rule an engine or screen declaration is.
- **Read**: `crates/core/src/rerecord.rs` (the module doc, the family dispatch, `plan`, `Run::write`,
  the typo guard); `crates/core/tests/rerecord.rs`; `docs/CONFORMANCE.md` §4 "`rerecord`".
- **Builds**: the third version key; family runners `run_storage` and `run_career` in `crates/core`
  (shared with the gates, so the gate and the recorder run one definition); `plan` walks
  `vectors/storage/` and `vectors/career/` for a career declaration and never reads another
  family's files; the career family's `expected.reports` is run too, once P15–P16 exist, and until
  then is held unchanged by the gate (the phase decides whether `reports` is run or carried, and
  says which). A career declaration may not carry `removed`, as an engine one may not.
- **Tests**: a dry run of an empty `career-v0` declaration writes nothing; an undeclared difference
  is refused; the version guard refuses a declaration not at `CAREER_VERSION`; a career run never
  opens an engine or screen file.
- **Files**: `crates/core/src/{rerecord,lib}.rs`, `crates/core/src/bin/golf_core.rs`;
  `crates/core/tests/rerecord.rs`.
- **Done when / Verify**: as P5; the dry run writes nothing (`git status -- spec/` clean).

### P14 — The change: `COMPARABLE_FROM`, hand-worked vectors, and the first career re-record

- **Goal**: decisions 1 and 2, in Rust only.
- **Read**: `contracts/swing.py`'s ledger 13 → 16 and `crates/contracts/src/swing.rs`'s ledger
  doc; `crates/storage/src/{corpus,state}.rs`; the M34 plan's P10 (the re-record's shape); P13's
  findings.
- **Builds**:
  - `contracts::swing::COMPARABLE_FROM = 14`, beside `ANALYSIS_VERSION`, with its doc. The Rust
    ledger entry for 17 gains its class (`shape`), and `every_version_from_seventeen_has_a_ledger_entry`
    gains a sibling: every entry from 17 names one class, and `COMPARABLE_FROM` equals the newest
    `disagrees` entry, else 14.
  - `read_corpus` excludes on `stored < versions.comparable_from`. The `OUTDATED` detail sentence
    is rewritten to say what is now true (older than the oldest comparable engine, not older than the
    installed one). `is_outdated` in `state.rs` is untouched.
  - `CAREER_VERSION` 0 → 1, with its first ledger entry.
- **Hand-worked vectors** (`spec/vectors/storage/hand/`, `provenance.oracle: "hand"`, at
  `career_version` 1, each with its working in a `provenance.note`): versions `{17, 14}` over stored
  13, 14, 16 and 17; the boundary at exactly `comparable_from`; a corpus mixing comparable and
  outdated swings of one club, so the mishit median and the counts see only the comparable ones.
- **The re-record**: `spec/declarations/career-v1.json` names each moved `OUTDATED` detail path and
  `provenance` key, and nothing else. `golf-core rerecord --declare …` writes them; a second run
  writes nothing. No `career/` value moves (its inputs are already corpora); if one does, stop.
- **Measure**: the scratch run from P10 again, now with `{17, 14}`: Rust pools the swings frozen
  Python pools (expected 13 distinct). Both counts go in the findings.
- **Files**: `crates/contracts/src/{swing,career}.rs`; `crates/storage/src/corpus.rs`;
  `spec/vectors/storage/hand/*` (new); `spec/declarations/career-v1.json` (new); the re-recorded
  `spec/vectors/storage/**`.
- **Done when**: green; the dry run reports only the declared paths; the second run writes nothing;
  the measurement shows the swings pooled.
- **Verify**: `rerecord --dry-run`, then for real, then again; `cargo test`; clippy; fmt.

### P15 — Verbs I: `career-corpus`, `career-baseline`, `career-dispersion`

- **Goal**: the first three verbs, their text gated by the recorded reports.
- **Read**: `scripts/{career_corpus,career_baseline,career_dispersion}.py` (whole);
  `crates/core/src/bin/golf_core.rs` (the hand-rolled dispatch and its "Why subcommands" doc); call
  11; P3's format-scan finding.
- **Builds**: `crates/core/src/reports/{mod,corpus,baseline,dispersion}.rs` (the renderers, writing
  to a `String`, with the greedy `_wrap` through `pyfmt::split` and code-point widths); the three
  subcommands with `--name`, `--player-id`, `--verbose` and `--json`, the data-directory resolution,
  and the golfer lookup and `slugify` the scripts do; exit codes as the scripts'.
- **Gate**: `crates/core/tests/reports.rs` runs every recorded `career_corpus`, `career_baseline` and
  `career_dispersion` report, exact string equality.
- **Files**: `crates/core/src/{reports/*,lib.rs,bin/golf_core.rs}`; `crates/core/tests/reports.rs`.
- **Done when / Verify**: as P5; each verb runs over `data/` and exits 0.

### P16 — Verbs II: `club-profile`, `flag-mishit`, and the parity run over `data/`

- **Goal**: the last two verbs, and decision 3's one-time parity check.
- **Read**: `scripts/{club_profile,flag_mishit}.py` (whole); P15's findings.
- **Builds**: `reports/club_profile.rs` and the `club-profile` verb (`--club` through `parse_club`,
  the empty-state report, the bag entry block); the `flag-mishit` verb (`ref --confirm|--clear|--auto`
  through `BundleStore::set_mishit` with `Timestamp::now_utc()`, and `--list [--name]`).
- **Gate**: `reports.rs` runs the `club_profile` and `flag_mishit` reports. `flag-mishit`'s write path
  is tested on a `tempfile` copy of a vector tree: the manifest it writes reads back through the
  storage family's reader, and its `mishit` value is one frozen Python's `MishitVerdict` accepts
  (clause 4).
- **The parity run** (scratch, not committed): run each Python script and its verb over `data/` with
  every flag combination the scripts take (no args, `--name Aaron`, `--player-id aaron`,
  `--verbose`, `--club` for each tagged club, `flag-mishit --list`), and diff stdout byte for byte.
  Expected: identical everywhere, because after P14 both pool the same swings. Any difference is a
  finding, fixed in the renderer or recorded with its reason. The `data/` snapshot is re-checked
  after.
- **Files**: `crates/core/src/{reports/*,bin/golf_core.rs}`; `crates/core/tests/reports.rs`.
- **Done when**: green; the parity run is identical or every difference is accounted for in the
  findings; `data/` unchanged.
- **Verify**: `cargo test`; clippy; fmt; the parity script; the snapshot.

### P17 — Docs: CONFORMANCE, the ADR addenda, ARCHITECTURE §1, `CLAUDE.md`

- **Goal**: every doc that describes what M36 changed says so, and docs-truth is green.
- **Read**: `docs/CONFORMANCE.md` §2, §3, §4; `docs/ARCHITECTURE.md` §1–§2; `CLAUDE.md`; ADR-024,
  ADR-026, ADR-028, ADR-032 (Status block and the last addendum's form), ADR-035 (Consequences);
  the program plan's §M36 and "What the code says" finding 9; `docs/README.md` (the ADR addenda
  count); every phase finding above.
- **Writes**:
  - `docs/CONFORMANCE.md`: the storage and career families (shape, cases, oracle, `CAREER_VERSION`),
    the new edges in §3 (the `pyfmt` additions, `Timestamp`'s spelling, `**`), the `rerecord`
    section's third key, `regenerate`'s two new refusals, and `check`'s two new deferrals.
  - ADR addenda, one each where the decision moved: ADR-024 (the corpus's `OUTDATED` now means "not
    comparable", `COMPARABLE_FROM`, and frozen Python disagreeing on purpose until M40); ADR-032 (the
    tenth crate and what the port measured); ADR-026 (the catalogue is a two-language file read by
    `include_str!`). P17 reads ADR-028 and ADR-035 and adds one only if a clause there is now false.
    `docs/README.md`'s addenda count moves with them.
  - `docs/ARCHITECTURE.md` §1 (the five verbs, `crates/storage`) and §2 (the cargo edges).
  - `CLAUDE.md`: the crate count and the crate paragraph (`crates/storage`, the new `contracts` and
    `analysis` modules), the commands block (one verb as an example), and the "how a port is
    checked" row (the two families, `CAREER_VERSION`).
  - The program plan's §M36: finding 1's correction, and a one-line pointer to this plan.
- **Done when**: docs-truth and pytest green; every new path cited exists.
- **Verify**: `pytest tests/test_docs_truth.py`; `pytest`.

### P18 — Close: verify, WORKLOG, ROADMAP, memory, and the one commit

- **Goal**: the milestone exit (below), checked, recorded, and committed once.
- **Steps**: run every command in "Verify" and the `career-v1` dry run (writes nothing); compare the
  `data/` snapshot a last time; `git diff -- src/` empty; `git diff -- spec/vectors/{synthetic,
  corpus,stages,screen}` empty. Write the WORKLOG entry (what was built, what M35, M29 and M38 should
  know, what is left open). ROADMAP: the status row and §M36 to ✅ with the date, and M35's row to
  unblocked; the program plan's status row the same; `docs/README.md`'s document count (this file
  becomes tracked). Update the `app-transition-decisions` memory (M36 done, next M35). Then one
  commit onto `main`.
- **Done when**: the commit is made and the exit holds.

---

## Exit

M36 is done when all of these hold, each checked at P18:

1. `spec/vectors/storage/` and `spec/vectors/career/` were recorded once from frozen Python, and
   `regenerate` refuses both.
2. `crates/storage` exists, and Rust passes every Python-recorded storage and career vector, report
   text included, through `cargo test`.
3. `golf-core rerecord` re-records both families under `CAREER_VERSION`. P14's change went through it,
   moved only declared paths, and a second run writes nothing.
4. The five verbs exist, and over `data/` they print what the Python scripts print (P16).
5. A Rust `read_corpus` over `data/` pools the swings frozen Python pools, under `COMPARABLE_FROM`.
6. The engine, stage and screen families are untouched; the format family only gained rows.
7. `git diff -- src/` is empty and `data/` is unchanged.
8. `cargo test`, clippy, fmt, pytest, ruff, mypy and `conformance.py check` are green.

---

## Phase findings

*(Appended as each phase closes.)*

### P1 (2026-10-07)

1. **The `data/` snapshot, for P2, P3, P10, P16 and P18.** 192 files under
   `data/processed/{sessions,golfers,shots}/`, combined digest
   `bd384cc40b236b73e3013b817ae79908d6e5ff4dded25b8803fc35c6b7af01cc`. The scratchpad listing does
   not survive the session, so compare by re-running this from the repo root in Git Bash and
   matching the digest:
   `find data/processed/sessions data/processed/golfers data/processed/shots -type f | LC_ALL=C sort | xargs -d '\n' sha256sum | sha256sum`.
   Unchanged at the close of P1.
2. **What was built.** `scripts/conformance_vectors.py` gained a "storage: the corpus reader"
   section at its foot: `build_storage_corpus(*, real=True)` (pairs, writes nothing),
   `_run_corpus(input) -> expected` (the definition a port reproduces) over `_read_and_narrow`,
   `_materialise`, `_read_tree`, `_storage_vector`, the synthetic cases in
   `_storage_corpus_synthetic`, and the real case in `_storage_corpus_real`. The synthetic trees
   are written by `tests/storage/conftest.py::write_swing`, loaded by path through the existing
   `_load_conftest`, and then edited per case. 31 synthetic cases at P1 (a snapshot; the pins hold
   the coverage, not the count), every row of P1's list plus the edges in finding 5. Building them
   takes ~2.5 s.
3. **The vector's shape moved from call 1 in four places**, all inside P1's own design:
   - **`career_version` is top-level**, not in `provenance`: M34's finding for
     `screen_parser_version`, since `golf-core rerecord` and `conformance.py list` look there.
     `provenance` carries `kind: "storage"`, `oracle`, `note`, `source`, `analysis_version`
     (which engine's `is_outdated` answered, informational only), `python_version`,
     `pydantic_version` and `recorded_by`. No vector has a top-level `analysis_version`, so `check`'s
     staleness rule cannot reach it.
   - **The narrowing arguments are input**: `input.narrowings: {<name>: {since, sessions, club}}`,
     each key `null` when absent, with `since` an RFC 3339 string. Call 1 put them under
     `expected`, where `_run_corpus(input)` could not read them.
   - **`expected` is `{corpus, properties, narrowed: {<name>: {corpus, properties}}}`.**
     `properties` holds `CareerCorpus`'s derived counts (`distinct_swings`, `distinct_shots`,
     `distinct_sessions`, `untagged_swings`, `duplicates_collapsed`, `shot_conflicts`,
     `mishit_shots`, `mishit_refs`, `mishit_shots_unconfirmed`), which `model_dump` drops and every
     report prints. P6's `CareerCorpus` properties are gated by them.
   - **`files: null` is a sessions root that does not exist**, `{}` an empty one, and a key ending
     in `/` (with text `""`) an empty directory. Synthetic trees have `\r\n` normalised to `\n`, so
     a vector does not depend on the recording OS; the real tree keeps the bytes on disk, which are
     CRLF.
4. **The real case**: `spec/vectors/storage/corpus/real.json.gz` (P2 writes it), 480 KB plain and
   27 KB gzipped, so gzipped (`_REAL_SUFFIX`). 57 `files` entries: every manifest, state file and
   `session.json`, and 15 slimmed analyses; the empty dotted `.incoming` is left out, as
   `list_session_ids` skips it. It reads 15 sessions, 15 swing directories, 13 distinct swings, 13
   distinct shots, 2 duplicates (`2026-08-09/2` and `2026-08-10/1` into `2026-08-07-aaron1/1`), one
   auto mishit (`2026-08-23/2`) and `population:golfdb` unknown. Narrowed once per club present, in
   `ClubId` order: `club-3w`, `club-7i`. **The slim-tree verify holds**: `read_corpus` and both
   narrowings over the slim tree equal the same calls over `data/processed/sessions/`, and a
   scratch negative control (one analysis at version 15) made them differ, so the verify can see.
5. **Edges a port will hit, for P4 and P10** (each is a recorded case):
   - **`_arrival` sorts on `created_at.isoformat()`, a string in the manifest's own offset**, not
     on the instant. isoformat writes `+00:00` (never `Z`), and a fraction only when the
     microsecond is non-zero, as six digits. So 15:00+05:30 loses to 12:00Z, 08:00-05:00 beats
     12:00Z, and a whole second beats `.500000` because `+` < `.` (`duplicate-across-offsets`).
     **P4: `Timestamp` needs an `isoformat()` with CPython's spelling beside pydantic's, and
     `timestamp.json` should record it.** The tiebreak is then `session_id` and `swing_id` as
     strings, so `10` beats `9` (`duplicate-same-second`).
   - **`swings` sorts by instant**, then `session_id` and `swing_id` as strings (`10` < `2` < `x`).
     **`excluded` keeps the scan order**: sessions lexically, swing directories by
     `_swing_sort_key` (numeric, then non-numeric names after), groups in first-member order
     (`sort-order`).
   - **The analysis and state are read from `sessions_dir / manifest.session_id /
     manifest.swing_id`**, not from the directory listed (`manifest-names-another-dir`).
   - **`load_manifest` drops the whole manifest** for a club, role or verdict no enum names, a
     missing required key, or a non-object; an unknown key is ignored (`corrupt-manifests`).
   - **`stored_analysis_version`** reads `true`, `16.0`, `"16"`, `-1` and a missing key as 0; 17
     is not outdated under `<` at 16 (`outdated-versions`). A readable state with no `inputs`
     defaults to `{}` and is therefore **stale** (`unreadable-state`).
   - **`_needs_review` validates through frozen Python's `ShotData`**; the invalid case is a shot
     with no `timestamp`, chosen because it is required on both sides of M32's widening. P10
     confirms Rust's `ShotData` refuses it too.
   - `statistics.median` averages the middle two of an even count, the floor comparison is a
     strict `<` (a carry exactly at 124.0 stays), and one carry per distinct photo enters it.
   - `_swing_sort_key` uses `str.isdigit()`, which is Unicode-aware; no case uses a non-ASCII digit.
6. **For P2**: `conformance.py list` shows the first present of `analysis_version`,
   `detector_version`, `screen_parser_version`, `python_version` at top level, and a storage vector
   carries none of them, so P2 adds `career_version` to that tuple or `list` prints `vNone`.
   `build_storage()` should be `build_storage_corpus()` plus the operation cases, built in full
   before anything is returned.
7. **Pins** (`tests/test_conformance.py`, "M36 P1"): the dry run builds every synthetic case in
   memory, checks ids, paths, `career_version`, the input and expected keys and that nothing under
   `spec/vectors/storage/` moved; a coverage pin that every `ExclusionReason`, both
   `MishitVerdict`s, an automatic flag, a narrowing and a missing root are reached; the version
   guard (`_run_corpus` refuses any pair but `{16, 16}`); and the real case, skipped by name
   without `data/processed/sessions/`.
8. **Verify**: `pytest` 2030 passed; `ruff` clean; `conformance.py check` exit 0, unchanged (the
   family is not written yet); `git diff -- src/ spec/` empty; the `data/` digest unchanged.

### P2 (2026-10-07)

1. **What was built.** `scripts/conformance_vectors.py` gained "storage: the store operations":
   `build_storage(*, real=True)` (the corpus half plus the op half, built in full before anything
   is returned), `build_storage_ops()`, `_run_ops(input) -> expected` (**the definition a port
   reproduces** for `bundle/` and `stores/`), the op dispatch `_op_table()`, `_frozen_clock`,
   `_raised`, the case builder `_OpsCase`, `_manifest_files`, `_storage_bundle_cases` and
   `_storage_store_cases`. P1's `_storage_vector` takes `run=` (`_run_corpus` by default), and
   `provenance.recorded_by` names the runner, which is how a port (and P13's `run_storage`) tells a
   corpus case from an op case. `scripts/conformance.py` gained `storage_vector_paths()`, `check`'s
   deferral line (no staleness test), `career_version` in `list`'s version-key tuple, and
   `regenerate --storage-once` with its refusal (`_STORAGE_RECORDED`), taken before the recorder is
   imported.
2. **Recorded.** `regenerate --storage-once` wrote 61 files, 1.2 MB: 33 under `corpus/` (P1's 32
   synthetic and `real.json.gz`, 28 KB), 16 under `bundle/`, 12 under `stores/` (counts are a
   snapshot). A second run exits 2 and writes nothing. Family digest, by P1's command pointed at
   `spec/vectors/storage`: `7e45cd4897e676e229f1b9b226db1da4d1b87958a70de6353632304c4472b0d5`.
   **It was recorded twice inside this phase**: the first run's `golfer-reads` note held a U+0130,
   and `conformance.py list` raised `UnicodeEncodeError` printing it to this box's cp1252 console.
   The untracked, unread family was deleted, the note respelled, and the family recorded again;
   `test_the_storage_family_is_committed_in_the_sub_families_python_recorded` now encodes every
   storage note as cp1252. The engine, stage, screen, format and audio families are untouched.
3. **The op vector's shape** (call 1, made concrete):
   - `input: {files, ops: [{op, args, now}]}`, `expected: {results, files}`. Each result is
     `{"returned": value}` or `{"raised": {type, message}}`, and the sequence carries on past a
     raise. `expected.files` is the root after the last op (`null` if it still does not exist),
     `\r\n`-normalised, empty directories as `/` keys, exactly as P1's trees.
   - **Every store opens on the one root**, and an op names its store: `bundle.*`, `bag.*`,
     `golfer.*`, `shot.*`. `slugify` is an op with no store, on `files: null`, so the slugify
     "table" is an op vector and one runner serves both sub-families.
   - An upload's scratch file is in `input.files` from the start, at `.incoming/<n>.part` (where
     `api/app.py` streams one); `args.tmp_path` names it with `/`. `args.digest` is the real
     sha256 of the text, and the store does not hash.
   - **Every `now` is UTC with `Z`**, a minute apart, the microseconds cycling 0, .120000,
     .000001, .999999, .500000, .345678. pydantic writes the fraction as six digits whenever it is
     non-zero (`…:01:00.120000Z`, trailing zeros kept) and drops it at zero (`…:00Z`); the written
     files carry both. An op that cannot stamp has `now: null` and the frozen clock raises if read
     (`test_an_op_recorded_without_a_clock_may_not_read_one`). An op that may stamp but does not
     (a dedupe, a missing swing) still carries one. One op reads one instant, so a new swing's
     `created_at`, `received_at` and `updated_at` are equal here.
   - A raise records `type` and `message`; a `ValidationError` records its validators' own
     messages joined by `; `, or `message: null` when any error is pydantic's (bad JSON, a missing
     key). The bag store's message has the scratch root as `<root>/`.
   - Models are `model_dump(mode="json")` (the same timestamp spelling as `model_dump_json`);
     `AssignmentResult` is a dict of its eight fields; `shot.put` answers the path relative to the
     root.
4. **Edges a port will hit, for P8 and P9** (each is a recorded case):
   - **A manifest is trusted over its directory** (`manifest-names-another-swing`): `_place`
     writes to `<call's session>/<manifest.swing_id>`, the answer's `session_id` is the
     manifest's (the dedupe answer's is the call's), `_next_swing_id` reads manifest ids, and
     `attribute_unlabeled` saves to `manifest.swing_id`'s directory, so it can write one directory
     twice and answer `["5", "5", "6"]`.
   - **A corrupt manifest is invisible**, so its swing number is reused and its directory written
     over, the orphaned file left (`assign-over-corrupt`); an explicit target whose manifest is
     corrupt is opened afresh. An explicit target never dedupes: the same bytes are re-placed, same
     filename, nothing unlinked, `received_at` moved.
   - `content_filename` takes Python's `Path.suffix`: `clip.` and `.hidden` have none (the role's
     default), `photo.tar.gz` keeps `.gz`, case is kept. Rust's `Path::extension` answers `Some("")`
     for `clip.`.
   - The atomic writes go through `with_suffix(".tmp")`: `manifest.tmp`, `<id>.golfer.tmp`,
     `<id>.bag.tmp`. `repairs` pins a stale `manifest.tmp` being consumed by the next save.
   - Bag: the same club returns the *loaded* bag unwritten; `set_entry` clears a caller's
     `retired_at`; a restore whose newest stint is the club already in the slot writes nothing;
     `_load_for_write` refuses on any parse or validation failure, with `bag at <root>/<id>.bag.json
     exists but cannot be read; refusing to overwrite it`; a non-slug id is the validator's message,
     with Python's `repr` of the id.
   - Golfer: the display name is `name.strip()` by `str.isspace` (`pyfmt::strip`: \x1c–\x1f, NBSP,
     EM SPACE); the refusal is `name {name!r} has no usable characters for an id`, through `pyfmt`'s
     `repr` of a `str` (`'​'`, `"'"`); `list_all` sorts on `display_name.lower()` by code
     point, stable, ties in sorted-filename order; `get` answers the record the file holds even
     when its id is not the filename's; `get_or_create` over a corrupt record writes over it.
   - **`slugify`'s "combining" is the canonical combining class, not general category M**:
     U+0903 and U+20DD have class 0, so `unicodedata.combining` keeps them and `_NON_SLUG` makes
     `aःb` into `a-b`. `unicode-normalization`'s `is_combining_mark` would strip them; its
     `canonical_combining_class` agrees with Python. The Kelvin sign lowers to ASCII `k`; `ß`, `ø`,
     a ligature, `ǅ`, `Ⅻ` and full-width forms survive NFD and become `-`.
   - Shot store: `get` raises on an unreadable file, the one reader here that does, while `all`
     skips it; `all` is newest first by instant and stable, ties in sorted-path order; the key
     falls back to `shot_id` for no provenance, a `None` hash and an empty one; each
     `[^A-Za-z0-9._-]+` run, non-ASCII included, becomes one `-`.
   - **This box's filesystem is case-insensitive**: `path_for("Aaron")` found `aaron.bag.json` in
     the first build and fired the write guard where Linux fires the slug validator. That op became
     `Dave Smith`; no case depends on case, and the section header lists it with the other
     machine-dependent edges no case reaches.
5. **For P4 — there is already a `contracts::Timestamp`.** `crates/contracts/src/lib.rs:181` is a
   crate-root newtype over the lexeme (`String`, derived *lexical* `Ord`), used by
   `ShotData.timestamp` and by `feedback`; its doc says it becomes a parsed type "if a phase ever
   needs to order two shots". The shot store's `all()` does (`shot-all-order`: 08:00-05:00 leads
   12:00Z), and P4's `contracts::time::Timestamp` would share the name. P4 decides whether the new
   type replaces the root one (the engine vectors must stay byte-identical, and pydantic-canonical
   lexemes do round-trip) or stands beside it under another name, and records which.
6. **For P8 — Rust's `ShotData` is M32's wider shape**: it serializes `attack_angle` through
   `carry_offline`, and `parser_version`, `fields_present` and `corrections` in the provenance, none
   of which frozen Python's shot files hold. The `shot-*` vectors' `expected.files` and answers are
   frozen Python's key set, so P8's gate compares a written shot on the keys Python wrote (Rust's
   extra keys at their defaults) or the writer leaves defaults out, and P8 records which.
7. **Pins** (`tests/test_conformance.py`, "M36 P2"): the op cases build in memory with one answer
   per call and the call shape above; every op in `_op_table()`, each verdict and `None`, a dedupe,
   an explicit target, a raise of each kind and an absent root are reached; a stamping op with
   `now: null` stops the build; the committed sub-families `corpus`, `bundle` and `stores` are
   Python-recorded at `career_version` 0 with cp1252-safe notes; `check` defers the family and calls
   nothing stale at `career_version` 99; `list` shows `v0`; `--storage-once` refuses on a committed
   vector and on any file, reaching no builder and writing nothing; and an empty family records once
   then refuses. `_refuse_to_build`'s fake gained `build_storage`.
8. **Verify**: `regenerate --storage-once` twice (0, then 2 with the digest unchanged); `pytest`
   2039 passed; `ruff` clean; `conformance.py check` exit 0, the 61 deferred; `conformance.py list`
   exit 0; `cargo test` green (no crate reads `spec/vectors/storage/` yet); `git diff -- src/` and the
   four frozen families empty; the `data/` digest unchanged (`bd384cc4…`).

### P3 (2026-10-07)

1. **What was built.** `scripts/conformance_vectors.py` gained "career: the aggregates and the
   reports" at its foot: `build_career(*, real=True)` (built in full before anything is returned),
   `_run_career(input) -> expected` (**the definition a port reproduces**), `_career_reports`,
   `_career_scripts` (the five scripts loaded by path through `_load_conftest`), `_printed` (the
   `redirect_stdout` capture, which also asserts a report returned `None` or a listing 0),
   `_mishit_listing`, `_patched`, `_career_vector`, `_career_given`, `_digest`, and the three case
   builders `_career_built`, `_career_adopted`, `_career_real`. A `GOLFERS` constant sits beside
   `SESSIONS`. `scripts/conformance.py` gained `career_vector_paths()` (`provenance.kind ==
   "career"`), `check`'s sixth deferral line (no staleness test), `regenerate --career-once` with
   its refusal (`_CAREER_RECORDED`), taken before the recorder is imported, and `_REFUSED` now names
   three record-once flags. `list` needed nothing: P2 put `career_version` in its key tuple.
2. **Recorded.** `regenerate --career-once` wrote 56 files: 55 plain under `career/synthetic/`
   (2.45 MB; the largest, `bag-declared`, 271 KB) and `career/real/aaron.json.gz` (50 KB, 714 KB
   plain, so gzipped by P1's rule). Counts are a snapshot. A second run exits 2 and writes nothing.
   Family digest, by P1's command pointed at `spec/vectors/career`:
   `627af0883e5042434488063b8369cefd14a8d5607eb31d094e5fbff8269757c8`. **It was recorded twice
   inside this phase**, on P2's precedent: the first family reached no sample size past
   `analysis/stats.py`'s critical-value tables (df 30), so nothing gated the Cornish-Fisher and
   Wilson-Hilferty expansions P11 has to port. The untracked, unread family was deleted,
   `past-the-tables` and `session-tie` were added, and the family recorded again. The storage
   family's digest (`7e45cd48…`) and the five frozen families are untouched.
3. **The vector's shape moved from call 2**, inside P3's own design:
   - **`career_version` is top-level** (P1's finding 3). `provenance` is `{kind: "career", oracle,
     note, source, analysis_version, python_version, pydantic_version, recorded_by}`;
     `analysis_version` names the engine the corpus report prints as installed, informational only.
   - **`input` is `{corpus, bag, display_name, versions, clubs}`.** `versions` is the storage pair,
     because `career_corpus._report` prints `ANALYSIS_VERSION` ("version 15; 16 is installed"), so a
     renderer takes the installed engine as a parameter; `_run_career` refuses any pair but frozen
     Python's `{16, 16}`. **`clubs` replaces call 2's single `club`**: every club in the bag profile
     in `ClubId` order, then the first `ClubId` that is not in it, whose report is the "never hit
     and not in the bag" sentence. One `club` could not say which `--club` reports a case holds.
   - **`expected` is `{baseline, dispersion, standing, bag_profile, properties, reports}`.**
     `properties` holds what `model_dump` drops and a report prints: `baseline: {claims_ready,
     nothing_sayable}`, `dispersion: {patterns_established, nothing_established}`, `standing:
     {placements, nothing_placed}`, `bag_profile: {clubs_used, clubs_declared, categories}` (club
     ids, and club id -> `ClubCategory` value). P6 and P7 gate those properties on it.
   - **`reports` keys**: `career_corpus`, `career_baseline`, `career_dispersion`, `club_profile`,
     each plain and `_verbose`; `club_profile_<club>` (plain) per `input.clubs`; and
     **`flag_mishit_list` on every case, not on the real tree only** (call 3). `_list` runs with
     `read_corpus` answering the case's corpus and a registry holding the case's golfer alone, so it
     is the renderer over `(player_id, build_bag_profile(corpus))`. Note it passes **no bag**, so
     only clubs with a shot print. Over the real case the recorder also runs `_list` over the
     materialised real trees (the storage vector's sessions and a copy of `data/processed/golfers/`)
     for `--list` and `--list --name aaron`, and raises unless all three texts are identical.
   - **What is not recorded**: each script's `main` (the golfer loop, the slug refusal, `no golfer
     'x'` with exit 2, the trailing `print()`). A script's stdout over one golfer is its report plus
     `"\n"`; checked by hand over `data/` for the four `_report`s plain and `--verbose`, `--club 7i`
     and `flag_mishit --list`, all byte-identical. P15 and P16's verbs own `main`, and P16's parity
     run checks it.
4. **The cases.** *Built* (23): `floor-{center,spread,trend}-{below,at}` (head_sway_norm,
   tempo_ratio and hip_shift_at_top_norm at n − 1 or n of each claim's floor, over three sessions);
   `trend-sessions-{1,2,3}`; `thin`; `pooling`; `dispersion-patterns`; `dispersion-drift` (pooled
   over within-session spread at 1.268 and 1.234, either side of 1.25); `standings`; `bag-none`;
   `bag-declared`; `bag-mishits`; `empty`; `untagged-only`; `declared-unhit`; `unmeasured-club`;
   `past-the-tables` (n = 31, 32 and 75); `session-tie`. Each asserts at build time that it lands
   where its note says. Values come from a sine walk (`wave`), so sums and powers see full-precision
   floats, or from `tests/analysis/`'s alternating `tight` where a placement is engineered.
   *Adopted* (32): every synthetic storage corpus case's `expected.corpus`, read from the committed
   family, which round-trips through `model_validate` and `model_dump` unchanged (asserted); named
   `storage-<case>`, a stranger's corpus displayed as `<id> (not registered)`, as the scripts do.
   *Real* (1): `career/real/aaron`, whose `input.corpus` is the real storage vector's
   `expected.corpus`, asserted equal to `read_corpus` over `data/`, with the real bag and
   `display_name` "Aaron"; `clubs` is the bag profile's seven (3w and 7i are the ones hit) then
   `driver`.
5. **The format scan, for P4** (and P11, P12, P15, P16). Every spec, width and CPython edge in the
   five scripts and the four `analysis/` modules:
   - **`_fmt`** (career_baseline, career_dispersion, club_profile): `f"{value:.{p}f}"` with `p` from
     `{degrees 1, yards 1, mph 1, ms 1, ratio 3, shoulder_widths 3}`, else 3 (`rpm`, `seconds`,
     `sd_units` and the built `furlongs` reach the default). `pyfmt::fixed`. **`-0.0` prints**
     (`past-the-tables`: a face_to_path_deg center and median of −0.02 print `-0.0`).
   - **`{percentile:.0f}`** (career_baseline): `fixed` at 0 places, half-even on an exact half.
     `standings` engineers 42.5 → `42` and 87.5 → `88`.
   - **`:g`** at the default precision 6: `{declared.tolerance:g}` (career_dispersion `--verbose`),
     `{loft_deg:g}` and `{length_in:g}` (club_profile: 30.5, 15, 24, 38.25, 35.125, 46). `pyfmt::g`.
   - **`:.3g`**: `analysis/dispersion.py:208–209`, the drift caveat's two spreads. **New**:
     `pyfmt::g` is precision 6 only. Recorded in `dispersion-drift`.
   - **`%Y-%m-%d` in the timestamp's own offset**: `club_profile.py::_print_bag_entry` and
     `analysis/club_profile.py::_bag_changed_caveats`. `bag-declared` dates a 23:30−05:00 entry
     `2026-08-20` (UTC says the 21st) and holds a +05:30 entry at the same instant as a `Z` swing.
   - **Timestamps pydantic writes** in this family: `Z` for UTC and `±HH:MM` otherwise
     (`2026-08-20T23:30:00-05:00`, `…+05:30`, `…+02:00`), whole seconds here. For `timestamp.json`.
   - **Padding, by code point**: `{name:<{width}}` (`width` is the longest metric name's `len`),
     `{reason:<14} {len(refs):>3}` (career_corpus); `{claim:<8}`, `{'':<8}` (career_baseline);
     `{label:<10}` behind an indent of 2, 4 or 6 (career_dispersion, club_profile);
     `{club:<6} {n_shots:>3}` (flag_mishit). Every padded string is ASCII today, and Rust's `{:<N}`
     pads by `char`, which is Python's code point, so std formatting matches with no `pyfmt` edge.
   - **The greedy `_wrap`**: `str.split()` (`pyfmt::split`: `bag-declared` carries an NBSP make and
     an EM SPACE shaft, which print as single spaces), `len` in code points (the em dash is one),
     `f"{current} {word}".strip()` (`pyfmt::strip`). Widths: 78 in career_dispersion,
     `94 - indent - 10` in club_profile (78, 80, 82 at indents 6, 4, 2).
   - Slices and joins: `sha256[:12]`, `", ".join`, `" ".join(make, model if present)`,
     `.replace("_", " ")` on a pattern and a category, `"-" * 72`, `str()` of an int version in
     `sorted(set)`, `sorted(dict.items())` on ASCII keys (code point order = UTF-8 byte order).
   - **The arithmetic** (P11, P12): `sum()` is compensated in `mean_and_sd` (both) and
     `_session_means` (`pyfmt::sum`). **`_within_session_sd` accumulates with a plain `+=`**, not
     `sum()`, so Rust must not use `pyfmt::sum` there. Float `**`: `(value - mean) ** 2`,
     `sd ** 2` (`sd_ci`, `_within_session_sd`), `z**3`, `z**5` (`_t_critical`), `(…) ** 3`
     (`_chi2_critical`), and **`(squares / degrees) ** 0.5`**, which is C `pow(x, 0.5)`, not
     `math.sqrt`. `df**2` is an int power and exact; `4 * df`, `9 * df` and `96 * df**2` are ints
     until divided. `round(placement, 1)` (`pyfmt::round_to`); `percentile_of`'s four interior
     segments are each reached (`past-the-tables` [p10, p25], `standings` [p25, p50] and [p75,
     p90], backswing [p50, p75]) and both rails. `min`/`max` over floats with no NaN.
   - **Order**: `_sessions_of` sorts on `(captured_at, session_id)`, the instant then the id;
     `session-tie` makes the instant tie and the lexeme order disagree with the id order.
     `_bag_changed_caveats` compares instants with a strict `<`.
6. **For P14 and P15**: `career_corpus._report`'s outdated paragraph says "analyzed by an older
   engine (version N; M is installed)" and "not comparable with a swing analyzed today". Under P14's
   `COMPARABLE_FROM` that sentence's `M` is still `versions.installed`, but "older engine" would then
   mean "older than comparable". P14 decides whether the corpus report's text changes with the
   detail sentence (a career-family re-record under `career-v1`) or stays faithful. P3 records it
   as frozen Python prints it.
7. **Pins** (`tests/test_conformance.py`, "M36 P3"): the dry run builds every synthetic case in
   memory, writing nothing, with the input, expected and report key sets above; a coverage pin that
   each floored metric crosses each claim's floor at n − 1 (shut) and n (open), the sessions gate
   shuts at 2 and opens at 3, every `Standing` and both sides of the band, every
   `DispersionPattern`, the scatter-only reading, the drift caveat each side of its factor, every
   bag-changed and mishit caveat form, and an interval at df 30 and past it; a branch pin that a
   sentence from each branch of the five reports is in some case's text (`_CAREER_BRANCHES`); the
   version guard; the real case (skipped by name without `data/processed/{sessions,golfers}/`);
   the committed family (`synthetic/` and `real/`, Python-recorded, `career_version` 0,
   cp1252-safe notes); `check` defers it with no staleness at `career_version` 99; `list` shows
   `v0`; `--career-once` refuses on a committed vector and on any file, reaching no builder; an empty
   family records once then refuses. `_refuse_to_build`'s fake gained `build_career`.
8. **Verify**: `regenerate --career-once` twice (0, then 2 with the digest unchanged); `pytest`
   2050 passed; `ruff` clean; `conformance.py check` exit 0, the 56 career vectors deferred beside the
   61 storage ones; `conformance.py list` exit 0; `cargo test` green (no crate reads
   `spec/vectors/career/` yet); `git diff -- src/` and the five frozen families empty; the storage
   digest unchanged (`7e45cd48…`); the `data/` digest unchanged (`bd384cc4…`).

### P4 (2026-10-07)

1. **What was built.** `crates/pyfmt`: `general(x, precision)`, CPython's `:.Ng` (a precision of 0
   is read as 1, as CPython does), with `g(x)` now `general(x, 6)`; and `lower`, `str.lower()`, as
   `str::to_lowercase`. `crates/contracts`: a new module `time` holding `Timestamp` (below), and
   `crates/contracts/tests/time.rs`. `scripts/conformance_vectors.py` gained a "format: the many-shot
   layer" section (`_format_general_precision`, `_format_lower`, `_format_timestamp`, with
   `_G_PRECISIONS`, `_LOWER_TEXTS`, `_TIMESTAMP_*` and `_CAREER_EDGES`), wired into
   `build_format`. `tests/test_conformance.py`: the family's id set and owner pin gained the three
   tables, and `test_the_many_shot_tables_hold_the_cases_that_separate_cpython_from_the_obvious_port`
   is new.
2. **Three new tables, not rows in old ones.** Adding `:.3g` rows to `general.json` or `lower` rows
   to `text_case.json` would have rewritten each file's note, which the done-criterion forbids, so
   each edge has its own: `general_precision` (417 cases, `pyfmt`), `lower` (329 strings plus one
   whole-map case, `pyfmt`), `timestamp` (186 written, 16 respelled, 24 refused; `implemented_by:
   "contracts"`, the first format table not `pyfmt`'s or `screen`'s, with `pydantic_version` in its
   provenance). Counts are a snapshot. `regenerate --format-only` rebuilt the ten existing tables
   byte-identically: `git status -- spec/vectors/format` shows the three new files and nothing else.
   `check` now reports 13 format vectors owned by `contracts`, `pyfmt` and `screen`.
3. **P2 finding 5's decision: the parsed type replaces the root one.** `contracts::Timestamp` is now
   `pub use time::Timestamp`, so `shot.rs`, `feedback.rs` and `crates/screen` name the same type by
   the same path. The lexeme type's own doc said it would become a parsed type the day a phase had
   to order two shots, and its other argument (a reformat may answer `+00:00` for `Z`) is met by
   writing pydantic's spelling back exactly. **The engine and screen families come through byte for
   byte**: every timestamp in them is already pydantic-canonical, `cargo test` passes them, and
   `golf-core rerecord --dry-run` over `v17.json` and `screen-v1.json` both say "nothing differs
   from the committed vectors". Two behaviours moved, both towards frozen Python: a `+00:00` or
   `-00:00` input now writes `Z`, and a short fraction writes six digits. One moved away, named:
   `ShotData`, `CoachingProvenance` and `ScreenInput` now **refuse** a timestamp outside the grammar
   at deserialization, a naive one included, where pydantic takes it. `crates/screen`'s fixture
   builds its timestamp with `.parse()`, and `ScreenInput.timestamp`'s doc says the rule. **For
   M38**: Dart's `toIso8601String()` on a UTC `DateTime` writes `….000Z`, which reads and re-spells;
   on a local `DateTime` it writes no offset, which is refused.
4. **`Timestamp`'s surface, for P8–P16**: `parse` (and `FromStr`, and serde through the string);
   `Display` is pydantic's spelling; `isoformat()` is CPython's (`+00:00` for `Z`), which
   `_arrival` sorts on as a string; `date_ymd()` is `%Y-%m-%d` in its own offset; `utc_micros()`
   and `offset_minutes()`; `now_utc()`. **Eq, Ord and Hash are by instant**, as Python's are, so
   `sort`/`sort_by_key` and a strict `<` port as written and Rust's stable sort keeps equal
   instants in input order; compare `to_string()` where a spelling must match. There is no
   constructor from parts: P9's clock comes from each op's `now` string, through `parse`.
5. **What pydantic 2.13.4 reads, measured** (each pinned in `tests/time.rs` or in the table): it
   reads and re-spells `+00:00` and `-00:00` as `Z`; it **truncates** a seventh fraction digit
   onwards; it reads naive values, a bare date, a Unix number, a space or a lowercase `t`/`z`,
   `+0530`, `12:00Z` with no seconds and a comma for the point, all of which `Timestamp` refuses
   (call 9's divergence); it refuses `+05`, `24:00:00`, `:60`, year 0000 and full-width digits, as
   `Timestamp` does. pydantic writes a `timezone` with a seconds part as `+00:00` (not `Z`) and
   `isoformat` as `+00:00:30`; nothing parsed can produce one. On this box CPython 3.13.3's `%Y`
   zero-pads years below 1000; that is the C library's choice, so the table records `date: null`
   there.
6. **`lower` measured over every code point**: CPython 3.13 lowers 1,433 code points to something
   else and Rust agrees on all of them, `İ`'s two code points and every `Final_Sigma` string case
   included. Rust also lowers 27 that CPython leaves alone, all Unicode 16.0 additions (U+1C89,
   U+A7CB, U+A7CC, U+A7DA, U+A7DC, U+10D50–U+10D65); `tests/format.rs` prints the count rather
   than asserting it, on `upper`'s precedent.
7. **For P5 — `str.isalnum()` is an edge nobody listed.** `club.py::_normalize` and
   `club_spec.py::_normalize` keep `ch for ch in text.lower() if ch.isalnum()`. Rust's
   `char::is_alphanumeric` is a different set: measured over every code point, CPython-only 0,
   **Rust-only 5,877** (`Other_Alphabetic` marks such as U+0345, the combining Latin letters
   U+0363–U+036F, Hebrew points from U+05B0, and 16.0's additions; none below U+0250). So a port
   on `is_alphanumeric` keeps a combining accent Python drops: `7i` plus U+0301 normalizes to `7i`
   in Python and fails to match in Rust. `pyfmt` cannot reproduce `isalnum` without a
   general-category table (it takes no dependency and std exposes none), so it was not added here.
   P5 decides between a recorded exception table, a dependency, or the ASCII-only rule as a named
   divergence (every alias and every catalogue name is ASCII), and records which.
8. **Also measured, for P9 and P10**: `str.isdigit()` is true for 798 non-ASCII code points (P1
   finding 5's `_swing_sort_key`); some of them (the superscripts) make `int()` raise.
9. **For P17**: `docs/CONFORMANCE.md` §2's Format heading still says 10 vectors and its table
   lists ten; §3 needs `general`, `lower` and `Timestamp`'s spelling. Docs-truth pins neither, so it
   stays green until P17 writes them.
10. **Verify**: `regenerate --format-only` (three files added, the ten existing unchanged);
    `cargo test` 735 passed, 0 failed; `cargo clippy --all-targets` clean; `cargo fmt --check`
    clean; `pytest` 2051 passed; `ruff check` clean; `conformance.py check` exit 0 and `list` exit
    0; `rerecord --dry-run` over `v17.json` and `screen-v1.json` nothing differs; `git diff --
    src/` empty; the engine, stage, screen and audio families untouched; the storage digest
    (`7e45cd48…`), the career digest (`627af088…`) and the `data/` digest (`bd384cc4…`) unchanged.

### P5 (2026-10-08)

1. **What was built.** An earlier unattended run had left the six modules in the tree, unpinned;
   this session reviewed each against its Python, re-measured every "frozen Python answers" claim in
   their tests against `.venv` (all held), and added the three integration pins. `crates/contracts`:
   - `club`: `ClubId` with `ALL`, `as_str` and `member_name`; `CLUB_CATEGORY` as a const table that
     a `const` block holds to discriminant order, so `category_of` indexes it bare; `parse_club` over
     an alias table derived as Python's is and held equal to frozen Python's (58 rows, a snapshot).
   - `club_spec`: `ShaftMaterial`, `ShaftFlex` (each with `ALL`), both parsers over derived tables
     held equal to Python's; `SpecProvenance`; `ClubSpec` with `blank(club)`, and
     `_loft_range_is_ordered` as `Validate`, its message through `pyfmt::repr`.
   - `bag`: `BagEntry` is `#[serde(flatten)] spec: ClubSpec` plus the three declaration fields, so
     the file is one flat object; `same_club_as` is spec equality, and a destructuring makes a new
     sibling field a compile error until someone says whether it is identity. `Bag` with `club_ids`,
     `retired_for`, the slug check and both model validators, each with Python's message.
   - `mishit`: `MishitVerdict` (with `ALL`), the constants, `mishit_carry_floor` with
     `statistics.median`'s rule (`(low + high) / 2` on an even count).
   - `golfer` gains `PLAYER_ID` (the pattern's source, for the message), `is_player_id`
     (hand-written; Python's `$` matches before one trailing newline, so `"aaron\n"` is a slug in
     both), `check_player_id` and `Golfer`.
   - `catalogue`: `CatalogueRow`, `SCHEMA_VERSION`, `read_rows(text)` and `load_catalogue()` over
     `include_str!` (call 10).
2. **P4 finding 7's decision: `char::is_alphanumeric` for `str.isalnum()`, as a named divergence.**
   Not ASCII-only, because that would make Rust *accept* `7é iron` as a 7 iron where Python refuses
   it, the wrong direction under ADR-024 §5. CPython's set is inside Rust's (P4: 0 CPython-only) and
   every alias is ASCII, so Rust can only refuse text Python takes (`7i` + U+0345: Python `7i`,
   Rust `None`), never the reverse. `pyfmt::lower`'s 27 extra lowerings (P4 finding 6) each land on
   a non-ASCII letter (checked), so they too can only refuse. `club.rs`'s module doc carries the
   argument and both modules' tests pin each side. No exception table, no dependency.
3. **`contracts` now depends on `pyfmt`** (`lower`, `repr`, `str_repr`). The rule "`contracts`
   gains no dependency" is read as no *third-party* one, which is decision 6's subject, since P5's
   own Builds line names `pyfmt::lower`; `serde` is still the only third-party dependency, and
   `pyfmt` depends on nothing of ours. `contracts`'s crate doc and `crates/analysis/Cargo.toml`'s
   comment are corrected. **For P17**: ADR-032 §1's body says `crates/contracts` depends on nothing
   of ours.
4. **For P8 — the catalogue's key is not in `contracts`.** `catalogue_key`, `key_for` and the keyed
   `lookup` fold through `slugify`, which needs `unicode-normalization` (call 6), so
   `contracts::catalogue` answers rows in file order and P8 puts the keyed reader beside `slugify` in
   `crates/storage`, applying Python's last-row-wins for a repeated key. `remember` stays Python's
   (nothing in Rust confirms a club yet).
5. **Differences from Python, each named in its module doc**:
   - `Bag.entries` is a `BTreeMap` in bag order, not insertion order; visible only in which slot a
     validator names when two slots are wrong, which no vector holds.
   - `Bag::validate` stops at the first failure, where pydantic collects every field error and
     joins them (P2 finding 3's `; `). No vector refuses one bag on two fields; **P9 checks before
     relying on it.**
   - pydantic 2.13.4's lax coercions (call 9) are refused: `"2023"` and `2023.0` for `model_year`,
     `"9.5"` for a loft, `1` and `"true"` for a bool, each measured as accepted by pydantic.
   - The catalogue's version test is Python's `== 1`, so `1.0` and `true` pass, as measured.
6. **For P9 — which text is the message.** A refusal is `ContractError {field, problem}`, and
   `problem` is the validator's message exactly (`player_id '../aaron' is not a slug (expected
   ^[a-z0-9]+(?:-[a-z0-9]+)*$)`, `entry filed under 7i declares itself pw`). Through serde it reads
   `Bag.player_id: <problem> at line … column …`. The op vectors' `raised.message` is `problem`, so
   P9 maps a `ValidationError` to `problem`, never to the serde error's `Display`.
7. **For P6**: `MISHIT_MIN_CLEAN_SHOTS` is a literal 5 in both languages and documented as the
   CENTER floor of `DEFAULT_MINIMUM_N`; P6 lands that table and can hold the two equal.
8. **The pins.**
   - **`tests/stores.rs`** walks every vector under `storage/` and `career/`. Every bag and golfer
     file (both sides), every bag and golfer an op returned, every bag `bag.save` was handed and
     every career `input.bag` reads and writes back as the same value (exact, reported by key
     path). The partial entries `bag.set_entry` was handed write back as pydantic's dump: the keys
     given at their values, the rest blank (measured equal to `model_dump` for all 18). The files
     frozen Python refuses (`REFUSED`, nine, with the sides each is refused on, because
     `golfer.get_or_create` writes a valid record over `corrupt.golfer.json`) are refused, with
     Python's validator message where there is one; an entry the walk never meets fails it. A
     negative control (skipping an empty `retired` on write) failed it with 20 differences.
   - **`tests/python_schemas.rs`** (call 8): for `Bag`, `BagEntry` and `SpecProvenance`
     (`bag.schema.json`) and `Golfer` (`golfer.schema.json`), the key set, the required set (a
     removed key is refused exactly when `required` lists it) and nullability (a `null` is read
     exactly when the schema admits one); and `ClubId` (in `bag`, `swing_manifest` and
     `session_meta`), `ShaftFlex`, `ShaftMaterial`, `MishitVerdict` and `Handedness` member for
     member, in order (call 12). A negative control (dropping `make`'s default, refusing a null
     material) failed it naming both.
   - **`tests/catalogue.rs`** (call 10's two pins): every committed row parses and writes back,
     under `exclude_defaults`, as the row on disk; every club, material and flex, a row with every
     optional key absent, and one with every key present (`false`, `0.0` and a `-05:00` retrieval
     among them, checked equal to Python's `exclude_defaults` dump) read back.
   - `tests/schemas.rs`'s note that M36 "takes the storage roots" now gives call 8's answer.
9. **Verify**: `cargo test` 797 passed, 0 failed; `cargo clippy --all-targets` clean; `cargo fmt
   --check` clean; `pytest tests/test_docs_truth.py` 92 passed; `git diff -- src/ spec/schemas` and
   the engine, stage and screen families empty; the storage (`7e45cd48…`), career (`627af088…`) and
   `data/` (`bd384cc4…`) digests unchanged.

### P6 (2026-10-08)

1. **What was built.** `crates/contracts`:
   - `career`: `CAREER_VERSION = 0`, its doc a ledger like `SCREEN_PARSER_VERSION`'s (what ages on
     it, why one key for both families, why an engine bump never moves it); the four source
     constants and `KNOWN_SOURCE_PREFIXES` with Python's `population:` warning carried, plus
     `is_known_source` (the `startswith(tuple)` both callers ask); `ExclusionReason` (with `ALL`,
     `as_str`), `ExcludedSwing`, `CorpusSwing` (`counts_toward_metrics`, `is_mishit`,
     `artifact_key`), `CareerCorpus` (all nine derived counts and `narrowed_to`), `count_metrics`.
   - `baseline`: `BaselineClaim` (with `ALL`), `DEFAULT_MINIMUM_N` and `MINIMUM_SESSIONS` as tables
     a `const` block holds to discriminant order (`club.rs`'s pattern), `METRIC_MINIMUM_N`,
     `minimum_n`, `minimum_sessions`, `Interval` (with `DEFAULT_CONFIDENCE`), `WithheldClaim`,
     `SessionSample`, `MetricSample`, `MetricBaseline` (`supports`), `PersonalBaseline`
     (`claims_ready`, `nothing_sayable`).
   - `lib.rs` gained `lt`, for `Interval.confidence`'s `lt=1.0`, the first strict upper bound on the
     ported surface; the crate doc names the two modules and their gate.
   - `crates/analysis/src/engine.rs` imports `contracts::career::POSE_DTL_SOURCE` and its private
     copy is gone (finding 7). **`POSE_FACE_ON_SOURCE` and `POPULATION_SOURCE` stay private there**,
     though finding 7 named the face-on one too: `career.py` defines only the `_dtl` string, and
     Python's engine spells the other two inline at their call sites. The engine family is unchanged
     (`cargo test -p core`).
2. **Where the Rust shape departs from Python's, each named in its doc**:
   - Python's `ref` property is a Rust keyword, so it is `swing_ref()` on both `ExcludedSwing` and
     `CorpusSwing`, the spelling `MetricSample.swing_ref` already uses for the same string.
   - **`narrowed_to` takes a `Narrowing { since, sessions, club }`** (`Default` + `Deserialize`) for
     Python's three keyword arguments, so a call names what it narrows on
     (`Narrowing { club: Some(c), ..Narrowing::default() }`). It reads the storage family's
     `input.narrowings` entries as they are. `sessions: Some(vec![])` keeps nothing, as Python's
     empty collection does (`narrowing`'s `no-sessions`). **P10's `narrow_to` and P12's per-club
     builder pass one.**
   - **Every Python `int` field is `i64`**, `contracts`' precedent: pydantic sets no bound on any of
     them, so a negative count reads in both. The derived counts (`distinct_swings` …
     `mishit_shots_unconfirmed`, `claims_ready`) return `usize`, as `len()`s; P11 and P15 cast
     where the two meet.
   - `metric_counts` and `PersonalBaseline.metrics` are `BTreeMap`s, which is the order Python's
     dicts hold, because both builders insert sorted by name (`count_metrics`, `build_baseline`).
   - `METRIC_MINIMUM_N` rows are slices of the claims they move, so Python's per-claim fallback
     (`.get(claim, DEFAULT_MINIMUM_N[claim])`) survives a partial override. Both rows name all three.
   - `CareerCorpus._suppressed_as_mishit` reads nothing of the corpus, so it is a free function.
3. **The pins.**
   - **`tests/career.rs`**: *every corpus* — the 33 storage corpus cases' `expected.corpus` and the
     56 career cases' `input.corpus` (counts a snapshot) — reads and writes back exactly;
     `count_metrics(swings)` answers the corpus's own `metric_counts` and `unknown_sources`; the
     corpus is a fixed point of the empty narrowing; the storage cases' `expected.properties` equal
     the nine derived counts; and each of the 13 recorded narrowings, applied with `narrowed_to` to
     the recorded corpus, equals the recorded narrowed corpus and its properties. *Every baseline* —
     each career case's `expected.baseline` reads and writes back, its `properties.baseline` equal
     `claims_ready` and `nothing_sayable`, and every `MetricBaseline` (161 in `baseline.metrics`,
     161 per club in `bag_profile.clubs[].metrics`) is held to the guard: `ready` and `withheld`
     partition the claims, each in declaration order; each withheld claim names `minimum_n` and
     `minimum_sessions`' floors, carries the metric's own `n`s and falls short (602 withheld, 147 on
     sessions); each ready claim clears both (364); a withheld claim's statistics are absent. *Every
     vector in both families* is at `CAREER_VERSION`, so a bump fails here until `rerecord` has run
     (P13, P14).
   - **Four negative controls, each failing the walk and then reverted**: a mishit no longer
     withholding carry (31 differences), `since` made exclusive (12), the default SPREAD floor at 9
     (219), `missing_roles` not written (629).
   - Unit tests: `tests/contracts/test_career.py`'s six cases; the flight keying on the photo and a
     placement on the swing; `count_metrics` over a re-upload, an outdated swing and a `_dtl` row;
     the empty, absent and club narrowings; `since` inclusive and by instant across offsets; the
     floor tables as Python holds them; **`MISHIT_MIN_CLEAN_SHOTS` equal to the default CENTER floor**
     (P5 finding 7); `confidence` open at both ends, defaulting to 0.95; a nested refusal naming its
     metric.
   - `tests/common/mod.rs` is new: `vectors_dir`, `vector_files`, `read_vector`, `differences` and
     `round_trip`, lifted out of `stores.rs` (which now uses it) on `crates/screen/tests/common`'s
     precedent, since `career.rs` is the second runner over the same two families.
4. **For P10**: a recorded corpus's `metric_counts`, `unknown_sources`, `outdated_swings` and
   `analyzed_without_measurements` are exactly `count_metrics` and `narrowed_to`'s expressions over
   its `swings`, on all 89 including both real ones, so `read_corpus` builds them with those and
   nothing of its own.
5. **For P7**: the per-club baselines inside `ClubProfile` already round-trip as `MetricBaseline`,
   so `ClubProfile.metrics` is `BTreeMap<String, MetricBaseline>` with nothing to add. `Interval` and
   `WithheldClaim` are here for `dispersion` and `comparison` to compose, as Python's do.
6. **For P11**: the guard is held only by its floors here. The refusal's sentence
   (`analysis/baseline.py::refuse` and `_CLAIM_PHRASING`) is P11's, and "a ready claim's statistics
   are present" is the builder's promise, not the shape's, so it is P11's gate too.
7. **Verify**: `cargo test` 818 passed, 0 failed; `cargo clippy --all-targets` clean; `cargo fmt
   --check` clean; `pytest tests/test_docs_truth.py` 92 passed; `git diff -- src/ spec/schemas` and the
   engine, stage and screen families empty; the storage (`7e45cd48…`), career (`627af088…`) and
   `data/` (`bd384cc4…`) digests unchanged.

### P7 (2026-10-08)

1. **What was built.** `crates/contracts`:
   - `dispersion`: `Finding`, `DispersionPattern` (each with `ALL`, `as_str`), `PATTERN_READING` as a
     table a `const` block holds to discriminant order with `DispersionPattern::reading()` over it,
     `SCATTER_ONLY_READING`, the seven provenance strings as private consts, `MetricTarget` and its
     validator, `METRIC_TARGETS` (20 rows, 15 without a target, in Python's declaration order),
     `target_for`, `SESSION_DRIFT_FACTOR`, `MetricDispersion`, `GolferDispersion`
     (`patterns_established`, `nothing_established`).
   - `comparison`: `Standing` (with `reading()`), `STANDING_READING`, `TOUR_COMPARISON_BLOCKED` with
     `tour_comparison_blocked`, the three no-population sentences and `SPREAD_NOT_COMPARABLE`,
     `no_population_reason` (on `career`'s prefixes), `MetricComparison`, `GolferStanding`
     (`placements`, `nothing_placed`).
   - `club_profile`: `ClubProfile` (`category()` through `category_of`, not serialized), `BagProfile`
     and its validator, `clubs_used`, `clubs_declared`, `profile_for`.
   - `lib.rs`: the three modules, and the crate doc's P7 sentence and bounds list.
2. **Where the plan's wording was loose, and what was ported instead.** Python's `MetricDispersion`
   and `ClubProfile` carry no validator. The two validators in these modules are `MetricTarget`'s
   (`gt=0.0` on `tolerance`, and `_reason_required_without_target`, whose message is Python's) and
   `BagProfile._clubs_are_in_bag_order` (`"7i appears twice in clubs"`, `"clubs are not in bag
   order: 3w came too late"`, field `BagProfile`, as `Bag`'s refusals name `Bag`). The "tuned and
   judged tuples" are strings: Python's parenthesised implicit concatenation.
3. **Where the Rust shape departs from Python's, each named in its doc**:
   - **`MetricTarget` is a row of a `const` table, not a serde shape.** Nothing on disk holds one,
     and the row carries its metric, so Python's dict key and `metric` field are one value here (the
     extracted table records both, and the pin holds `key == metric`).
   - `STANDING_READING` has no `Withheld` row, as Python's has none, so `Standing::reading()` answers
     `None` for it where Python would raise `KeyError`. Python's builder never asks.
   - `Finding` and `Standing` derive `Default` as `Withheld`, pydantic's field default.
   - `BagProfile.clubs` is a `Vec`; `clubs_used` and `clubs_declared` return `Vec<&ClubProfile>`.
   - A nested refusal read from JSON is raised by the leaf as it arrives (`validated!`), so the route
     (`BagProfile.clubs[0] -> ClubProfile.dispersion["…"] -> … -> Interval.confidence`) is named only
     by `validate()` on an assembled value. That is `baseline`'s behaviour too; the unit tests pin
     both.
4. **The pins** (`tests/aggregates.rs`, new):
   - *Every aggregate*: each of the 56 career cases' `dispersion`, `standing` and `bag_profile` (35
     club profiles) reads and writes back exactly, and its `properties` (`patterns_established`,
     `nothing_established`, `placements`, `nothing_placed`, `clubs_used`, `clubs_declared`, each
     club's `category`) are what Rust derives. Counts are a snapshot.
   - *Every recorded sentence*: every `MetricDispersion` (the golfer's and each club's) carries its
     row's `target` and `tolerance`, the row's `no_target_reason` exactly once when it has no target,
     and a `points_at` that is its pattern's reading or `SCATTER_ONLY_READING`. An unregistered metric
     carries neither and is refused. Every `MetricComparison`'s `reading` is its standing's, and every
     refusal is `no_population_reason`, the blocked sentence or the spread sentence. **Every entry is
     met by at least one recording**: all four patterns, the scatter-only reading, all 20 targets and
     tolerances, all 15 no-target reasons, the three standings, the three population sentences, the
     blocked metric and the spread sentence. The sentence *frames* are `analysis`'s, so the test
     matches the table entry inside each, not the frame.
   - *Every table, word for word*: `tests/data/python_tables.json` (new, ~19 KB) is every table in
     the two Python modules, extracted at P7 by a scratch script (deleted) under Python 3.13.3. It is
     the only pin on the seven provenance strings and `SESSION_DRIFT_FACTOR`, which no aggregate
     carries (the provenance reaches only the verbose dispersion report, wrapped). Frozen Python does
     not change (ADR-035 clause 4), so the fixture is not regenerated, and it outlives M40 because
     the test never runs Python.
   - **Six negative controls, each failing and then reverted**: one word in a no-target reason (the
     table pin and 6 recorded sentences); a provenance string (the table pin only, as designed);
     `patterns_established` counting `points_at` (6 property differences); an empty `caveats` skipped
     on write (23); the model sentence answering for `launch_monitor:` (63 sentences, 2 table
     entries); one tolerance moved (6 sentences, 1 table entry).
   - Unit tests: `tests/contracts/test_club_profile.py`'s ten cases (with Python's two messages), the
     target-less refusal and the `gt` bound, every row validating and registered once, no reading
     naming a specific check (`test_dispersion.py`'s banned words, on the tables alone), the source
     prefix picking the population sentence, and the derived counts.
5. **For P12**:
   - `test_every_production_metric_has_a_tolerance` (`POSE_MEASUREMENTS ∪ SHOT_MEASUREMENTS ==
     METRIC_TARGETS`) and the flight and pivot disjointness pins need `analysis`'s registries, so they
     land with `analysis::dispersion`, not here.
   - The sentence frames are the builders': `"{name} is not registered in METRIC_TARGETS, so there is
     no measurement error to judge either a bias or a spread against"`, `"no target for {name}:
     {reason}"`, `"{name} has a stored distribution and may not be placed in it: {blocked}"` and
     `"{name}: the spread is not placed against the tour spread — {SPREAD_NOT_COMPARABLE}"`. The
     session-drift caveat (`SESSION_DRIFT_FACTOR`, `:.3g`) is P12's as well.
   - `target_for` answers `Option<&'static MetricTarget>`; `Standing::reading()` is called after
     placement only, as Python's lookup is.
6. **For P15–P16**: the verbose dispersion report prints `f"{declared.tolerance:g} — {provenance}"`,
   so the renderer needs `pyfmt::g` at the default precision on `MetricTarget.tolerance`.
7. **Verify**: `cargo test` 844 passed, 0 failed; `cargo clippy --all-targets` clean; `cargo fmt
   --check` clean; `pytest tests/test_docs_truth.py` 92 passed; `git diff -- src/ spec/schemas` and the
   engine, stage and screen families empty; the storage (`7e45cd48…`), career (`627af088…`) and
   `data/` (`bd384cc4…`) digests unchanged.

### P8 (2026-10-08)

1. **What was built.** `crates/storage`, the tenth workspace member (`Cargo.toml`'s `members`), with
   its crate doc (what it ports, ADR-008's exception not carried over, call 9's divergence, the clock
   as an argument, the machine-dependent edges it takes the POSIX answer to) and `StoreError`:
   - `manifest`: `Role` (with `as_str`, `default_suffix`), `EXPECTED_ROLES`, `MANIFEST_NAME`,
     `RoleFile`, `SwingManifest` (`missing_roles`, `status` as a `BundleStatus` enum), `hash_bytes`,
     `hash_file` (and `hash_file_in_chunks`, Python's `chunk_bytes`), `content_filename`,
     `manifest_path`, tolerant `load_manifest`, atomic `save_manifest`.
   - `state`: `STATE_NAME`, `ANALYSIS_NAME`, `Status` (with `ALL`), `AnalysisState` (`new`,
     `matches`), `input_hashes`, `state_path`, `load_state`, `save_state`, `load_analysis` (a
     `serde_json::Map`), `stored_analysis_version`, `is_outdated`, and **`is_older_than(analysis,
     version)`**, which `is_outdated` is at `ANALYSIS_VERSION` (see 5).
   - `golfer_store`: `slugify`, `SUFFIX`, `GolferStore` (`path_for`, `get`, `list_all`,
     `get_or_create(name, handedness, now)`).
   - `shot_store`: `SUFFIX`, `hash_image`, `ShotStore` (`path_for`, `has`, `get`, `put`, `all`,
     `key_for`).
   - `catalogue`: **P5's finding 4, done here**: `catalogue_key`, `key_for`, `load_catalogue` (keyed
     into a `pyfmt::OrderedMap`, so a repeated key's last row wins in its first row's place, as
     Python's dict assignment does) and `lookup`. `remember` stays Python's.
   - Dependencies, each named with why: `sha2` 0.10 and `unicode-normalization` 0.1 in
     `crates/storage`; `tempfile` 3 as a dev-dependency of `crates/storage` and of `crates/core`
     (whose `storage` edge is a dev-dependency too, until a verb or `run_storage` crosses it). The
     lock resolved `sha2` 0.10.9, `unicode-normalization` 0.1.25 and `tempfile` 3.27.0, and their
     transitive crates, all building on rustc 1.87.
2. **The gate**, `crates/core/tests/storage.rs`: `run_ops` is `_run_ops` for the ported stores
   (materialise `input.files`, open the stores on one root, run each op under its own `now`, read
   the root back), compared through `golf_core::compare`. **All seven runnable vectors pass**
   (`golfer-get-or-create`, `golfer-missing-root`, `golfer-reads`, `shot-all-order`,
   `shot-missing-root`, `shot-store`, `slugify`). Every other storage vector is in `DEFERRED` **by
   id**, in three rows: "P9, the bag store" (5), "P9, the bundle store" (16), "P10, read_corpus"
   (33). `every_deferred_vector_exists_and_waits_for_a_store_not_yet_ported` holds the list honest
   both ways: each id exists and needs an op P8 did not port (or is a corpus case, for the P10 row),
   and no vector outside it does. **P9 and P10 each delete their rows**, and extend `PORTED` and
   `run_op`. A raise maps from `StoreError` by the exception it stands for: `Refused` → `ValueError`
   with the message, `Invalid(ContractError)` → `ValidationError` with `problem`, `Unreadable` →
   `ValidationError` with `null`, and `Io` panics (no recorded op raises an `OSError`). A file whose
   recorded text is JSON is compared as a value, anything else as text, and the gate also checks each
   vector's `career_version` against `CAREER_VERSION`.
3. **P2's finding 6, settled: the shot store writes Rust's whole `ShotData`, and the gate compares a
   shot on frozen Python's key set.** Each of the ten Rust-only keys (M32's seven, and
   `parser_version`, `fields_present`, `corrections` in the provenance) must be at its default and is
   then dropped (`python_shaped_shot`, applied to `shot.get` and `shot.all` answers and to every
   `*.shot.json` left on disk); one at another value stays and is reported as an added key. Leaving
   defaults out in the writer was declined: it would change how every engine vector serializes its
   `shot`, and Python already reads a Rust-shaped shot by ignoring the extras.
4. **Two more pins**:
   - `crates/storage/tests/python_schemas.rs` (call 8): `RoleFile`, `SwingManifest` and
     `AnalysisState` against `swing_manifest.schema.json` and `analysis_state.schema.json` (key set,
     required set, nullability), and `Role` and `Status` member for member, in order. Its `pin` is
     copied from `crates/contracts/tests/python_schemas.rs`, not shared.
   - `every_manifest_and_state_frozen_python_read_round_trips` (in the gate): every manifest a bundle
     op answered (`get_swing`, `get_session`, the three `set_*`) and every `manifest.json` and
     `analysis.state.json` in the real corpus's tree reads and writes back as the same value (63
     manifests and 15 states, a snapshot). **Every real manifest predates `mishit` and the earliest
     predate `club`**, so writing one back adds those keys at `null`, as pydantic's dump would; the
     pin accepts a key the file lacks only when Rust writes a blank there.
5. **For P10 and P14 — `read_corpus` must call `is_older_than(analysis, versions.installed)`, never
   `is_outdated`.** `is_outdated` is the faithful *question* (is there a newer engine to run, for
   M29's re-analysis verb) at **this build's** `ANALYSIS_VERSION`, which is 17 in Rust where frozen
   Python's is 16, so calling it from the corpus would exclude every recorded swing. P14 then
   switches the corpus's call to `versions.comparable_from` and leaves `state.rs` untouched, as the
   plan says.
6. **For P9**:
   - `StoreError` is the error type the bag and bundle stores should return. The bag vectors record
     six raises, all in `bag-write-guard`: four `ValueError`s (`bag at <root>/<id>.bag.json exists
     but cannot be read; refusing to overwrite it`, so `Refused` with the root spelled `<root>/` by the
     gate) and two `ValidationError`s from `bag.set_entry`'s rebuild (`player_id '../aaron' is not a
     slug …`), which `validate()` on the assembled `Bag` returns as a `ContractError`, so `Invalid`
     carries `problem` through. **A bag refused through serde loses its `ContractError`** —
     `validated!` makes it a string, which is `Unreadable` and `message: null` — and no vector reaches
     that: no `bag.save` argument is refused. If P9 adds a path where one could be, the read has to be
     unvalidated and `validate()` called after.
   - `crate::write_atomically` and `crate::tmp_beside` are the shared tmp-and-rename write; `tmp_beside`
     is `with_extension("tmp")`, which agrees with Python's `with_suffix(".tmp")` on every name ending
     in a suffix (all of the stores' names) and not on `clip.` or `.hidden`.
   - `content_filename`'s suffix is Python's `PurePath.suffix` (measured: `clip.`, `.hidden`, `..`,
     `...`, a trailing `/` or `/.` have none or the last real component's), taking the name after the
     last `/` **or `\`**, which is the recording machine's rule and keeps a backslash out of a stored
     filename everywhere.
7. **`slugify` against CPython, measured over every code point** (alone, and between two letters),
   by a scratch run deleted after: **46 differences, all one kind** — combining marks assigned after
   Unicode 15.1 (U+0897, U+1ACF–U+1AEB and others in the supplementary planes) have a non-zero class
   in `unicode-normalization` 0.1.25's tables (Unicode 17.0) and none in CPython 3.13's (15.1), so
   `x` + mark + `y` is `xy` in Rust and `x-y` in Python. No NFD decomposition differs, and P4's 27
   newer lowerings all fold to `-` in both. Named in `slugify`'s doc; no name on disk or in a vector
   holds one. Rust's side of it can only merge two spellings Python keeps apart.
8. **Edges taken, each in its module's doc**: a `glob` is case-sensitive and a listing sorts by code
   point (Windows' `pathlib` folds case for both); line endings are `\n` (Python's `write_text` writes
   `\r\n` on Windows); `shot_store.put` writes in place, not through a `.tmp`, as Python does;
   `load_analysis` refuses what `json.loads` takes beyond JSON (`NaN`, `Infinity`, an unpaired
   surrogate escape), and `stored_analysis_version` saturates an integer past `i64` to `i64::MAX`
   (still "newer than anything"), while one past `u64` is a float to `serde_json` and reads as 0.
9. **Negative controls, each failing the gate or the pin and then reverted**: `is_combining_mark`
   for the combining class (2 `slugify` differences); `list_all` sorted without lowering (12);
   the display name through Rust's `trim` (13); `all()` sorted ascending then reversed (2); an empty
   hash used as a shot key (3); the refusal message through `{:?}` rather than `repr` (4);
   `RoleFile.warnings` made required (the schema pin); a Rust-only shot key held to another default
   (15 added keys).
10. **For P13**: `run_ops` in the gate is the definition `run_storage` must share, written over
    `serde_json::Value` in and out so it lifts as it is. Lifting it makes `storage` a normal
    dependency of `golf-core` and needs a scratch root at run time, so `tempfile` would become a
    normal dependency there (or the recorder writes its own scratch directory); that is a new
    dependency edge and wants naming in that phase.
11. **For P17**: `CLAUDE.md` says nine crates and the workspace now has ten; `docs/ARCHITECTURE.md`
    §2's cargo edges gain `storage → {contracts, pyfmt}` and `core ⇢ storage` (dev); ADR-008's
    addendum exception has no Rust twin (`state` is a module of `storage`), which the ADR-032 addendum
    can say. Docs-truth pins none of these, so it stays green until P17 writes them.
12. **Verify**: `cargo test -p storage` 28 passed (26 unit, 2 schema pins); `cargo test` 875 passed,
    0 failed; `cargo clippy --all-targets` clean; `cargo fmt --check` clean; `pytest
    tests/test_docs_truth.py` 92 passed; `pytest` 2051 passed; `git diff -- src/ spec/schemas` and the
    engine, stage, screen and audio families empty, the format family only P4's three new files; the
    storage (`7e45cd48…`), career (`627af088…`) and `data/` (`bd384cc4…`) digests unchanged.

### P9 (2026-10-08)

1. **What was built.** Two modules of `crates/storage`, each doc naming the Python it ports and
   carrying its docstrings' reasons:
   - `bag_store`: `SUFFIX`, `BagStore` (`new`, `root`, `path_for`, tolerant `get`, `save`,
     `set_entry(player_id, &entry, now)`, `remove_entry(player_id, club, now)`,
     `restore_entry(player_id, club, now)`), and the private `load_for_write` (the write guard) and
     `write` (rebuild, validate, save). Mutators answer `Result<_, StoreError>`.
   - `bundle_store`: `AssignmentResult` (Python's dataclass order, `status` as `BundleStatus`),
     `Upload<'a>` (the ten keyword arguments of `assign_from_path`, `swing_id` the explicit target),
     `SwingBundleStore` (`new`, `root`, `current_session_id(now)`, `list_session_ids`,
     `get_session`, `get_swing`, `assign_from_path(&upload, now)`, `attribute_unlabeled`,
     `set_player`, `set_club`, `set_mishit`, `delete_swing`), with the lock a `Mutex<()>` per store
     (poison recovered, since what it guards is on disk) and the three repairs sharing one private
     `repair`. Writes answer `io::Result`, because frozen Python's bundle store raises only `OSError`.
   - `lib.rs` lists both; `StoreError::Refused`'s doc names the bag guard's message.
2. **The gate**, `crates/core/tests/storage.rs`: **all 28 operation vectors pass** (16 `bundle/`, 12
   `stores/`). `DEFERRED` holds the one `P10, read_corpus` row (33 ids); `PORTED` gained `bag.` and
   `bundle.`. The gate test is renamed `every_store_operation_conforms`, and `run_op` now answers
   every op in the recorder's `_op_table` (an unknown op panics rather than pointing at `DEFERRED`).
   Three gate-side additions, each the recorder's behaviour:
   - `raised(error, root)` rewrites `{root}{MAIN_SEPARATOR}` to `<root>/` in a `ValueError`'s
     message, as `_raised` does, for the bag guard's path.
   - **P8's finding 6, answered without a store change**: a model argument (`bag.save`'s bag,
     `bag.set_entry`'s entry) is read through each contract's *unvalidated* `deserialize` — the
     inherent half `#[serde(remote = "Self")]` generates, `pub` with the struct — and then
     `validate()`d, so a validator's refusal keeps its `ContractError` and its message, as
     `model_validate`'s does. No vector holds such an argument; the path is there for when one does.
   - `bundle.current_session_id` passes `args.now`, or the op's `now` when it is `null` (the
     recorder's frozen clock).
3. **API calls this phase made** (contestable at P10–P16 by a finding):
   - **The clock is never optional.** `current_session_id(now: Timestamp)` has no `None` form;
     Python's `now or datetime.now(tz=UTC)` is the caller passing `Timestamp::now_utc()`.
   - **`BagStore::save` validates**, where Python's does not need to: a pydantic `Bag` cannot be
     built invalid and a Rust one can, so `save` refuses with `StoreError::Invalid` and writes
     nothing. `write` relies on it, so `_write`'s "constructed, not `model_copy`-ed" reason holds.
   - `set_entry` with no bag on disk builds the empty `Bag` and validates it **before** anything
     else, so `../aaron` and `Dave Smith` are refused by the slug rule with Python's message and no
     path is made (the guard's two `ValidationError`s).
   - **A listing that fails is empty.** `list_session_ids` and `get_session` read a missing root, a
     missing session, and a session id naming a file all as `[]`; Python's `iterdir` raises
     `NotADirectoryError` on the last. Named in the module doc.
4. **Swing numbers, measured against Python's `int`** (the module doc's divergence list):
   - ASCII digits only. `str.isdigit` is Unicode-aware (P4 finding 8), so a hand-made `٣` sorts as 3
     in Python and among the names here, and `²` crashes Python's `get_session` and is a name here.
     The store names every swing `str(int)`, so only a hand-made directory reaches either.
   - **No ceiling**: `Number` carries the canonical digits (leading zeros dropped, `007` is 7),
     ordered by length then text, and `_next_swing_id`'s `+ 1` is done on the digits, so
     `99999999999999999999999` sorts and increments exactly. Non-numeric names rank at `10**9`, so
     swing `1000000001` sorts after `x`, as Python's tuple puts it.
   - A key tie (`7` and `007`) is broken by name here, where Python leaves the filesystem's order.
   - `delete_swing` on a symbolic link removes the link; `shutil.rmtree` refuses one.
5. **For P10**: `bundle_store::{swing_sort_key, swing_order, Number}` are `pub(crate)`, for
   `read_corpus`'s scan order (P1 finding 5: `excluded` keeps sessions lexically and swing
   directories by `_swing_sort_key`). `read_corpus` walks `list_session_ids` and `get_session`, as
   `corpus.py:63–73` does, and a `SwingBundleStore` costs nothing to open. **The corpus reads the
   analysis and state from `manifest.session_id / manifest.swing_id`**, not from the listed
   directory, and `get_session` answers manifests without their directory, which is what that rule
   needs.
6. **For P13**: `run_ops` now opens all four stores (`Stores`), still over `serde_json::Value` in and
   out, and `raised` needs the scratch root; P8's finding 10 (lifting it makes `storage` and a
   scratch root normal dependencies of `golf-core`) stands unchanged.
7. **Unit tests**, mirroring `tests/storage/test_{bag,bundle}_store.py` case by case where a vector
   does not already hold it: 11 in `bag_store` (tolerant reads, `save` stamping nothing, the store's
   stamp, the same-club and fresh-provenance short-circuits, retirement, remove and restore as two
   stints, the write guard for all three mutators, the slug refusal, `save`'s refusal) and 11 in
   `bundle_store` (the offset date, role filling, dedupe and its scratch file, newest-wins, the
   explicit target's unlink, a corrupt directory, write-once stamping, the backfill, the three
   repairs, the phantom, and the swing-number order and increment).
8. **Negative controls**, each failing the gate and then reverted: an assignment answering the call's
   session (1 difference, `manifest-names-another-swing`); a superseded file left in place (2,
   `assign-explicit-target`); swing directories sorted by name (17, `reads`); an explicit target that
   dedupes (3, `assign-explicit-target`); the same club re-stamped and written (24, `bag-set-entry`
   and `bag-remove-restore`); a corrupt bag read as no bag by a writer (14, `bag-write-guard`).
9. **Verify**: `cargo test -p storage` 50 passed (48 unit, 2 schema pins); `cargo test` 897 passed,
   0 failed; `cargo clippy --all-targets` clean; `cargo fmt --check` clean (after `cargo fmt` on the
   three files this phase wrote); `pytest tests/test_docs_truth.py` 92 passed; `pytest` 2051 passed;
   `git diff -- src/ spec/schemas` and the engine, stage, screen and audio families empty, `spec/`
   holding only P2–P4's untracked additions; the storage (`7e45cd48…`), career (`627af088…`) and
   `data/` (`bd384cc4…`) digests unchanged.

### P10 (2026-10-08)

1. **Resumed.** The attempt started at 08:18 was cut off after writing `crates/storage/src/corpus.rs`,
   its line in `lib.rs`'s crate doc and the gate's corpus runner, and deleting the `DEFERRED` row
   and its both-ways pin. This session found all of it compiling and the gate green, kept it, and
   finished the rest: the unit tests, the negative controls, the scratch run and `cargo fmt` on
   `crates/core/tests/storage.rs` (left unformatted).
2. **What was built.** `corpus`: `EngineVersions { installed, comparable_from }`,
   `read_corpus(sessions_dir, player_id, versions)`, `narrow_to(corpus, &Narrowing)` (a delegate to
   `CareerCorpus::narrowed_to`, as Python's is), and the private steps `corpus_swing`,
   `conflicting_shots`, `measurements`, `flag_auto_mishits`, `carry_of`, `needs_review`, `exclude`,
   `swing_ref` and `arrival`, each with its Python reason. Calls taken:
   - **The four derived counts are `narrowed_to(&Narrowing::default())`** over the read corpus,
     not a second spelling of Python's expressions (P6's finding 4 measured them equal on every
     recorded corpus).
   - Face-on groups are a `Vec` plus a `HashMap` index (a dict's insertion order, which `excluded`
     inherits); `flag_auto_mishits` groups clubs in a `BTreeMap`, since each club is judged alone.
   - The outdated test is `is_older_than(analysis, versions.installed)` (P8's finding 5), and the
     `OUTDATED` sentence names `versions.installed`.
3. **The gate**: `every_storage_vector_conforms` runs **28 op cases and 33 corpus cases** (the real
   one included) with no skips. A vector recorded by any other runner fails the gate rather than
   being passed over. P1's finding 5 is confirmed: Rust's `ShotData` refuses `shot-review`'s
   timestamp-less shot too, so it reads as needing review in both.
4. **Negative controls**, each reverted after: `arrival` sorting on the instant (10 differences,
   `duplicate-across-offsets`); a duplicate's photo equal to the survivor's counted as a conflict
   (12, across five cases, `real` among them); the outdated test against `comparable_from` (passes
   the family, which is all `{16, 16}`, and fails the new unit test). **Two passed the whole family**:
   - **The swings sort with the session-id tiebreak dropped.** `sort-order` ties three swings
     inside one session only. Frozen Python's `_run_corpus`, by a scratch run, sorts `2026-08-09/2`
     before `2026-08-10/1` at one instant, and a unit test now pins it (it fails the control).
   - **`measurements` and `needs_review` through the inherent, unvalidated `deserialize`.** This is
     an equivalent mutant today: `Measurement`'s validator checks nothing, and `ShotData`'s only
     bound is its provenance's, which the nested field's trait read already enforces. The doc says
     so, and the trait read stays so that a later validator is not skipped. A bound-only refusal
     (a parse confidence of 1.5, flagged by frozen Python in a scratch run) is pinned anyway, since
     no vector reaches one.
5. **Unit tests** in `corpus`, four of them, for what no vector reaches (each expectation read off
   frozen Python's `_run_corpus` over the same tree): the faithful rule reads `installed` and
   ignores `comparable_from`, the reader does not ask this build's `ANALYSIS_VERSION`, the tie
   across sessions, and the out-of-bounds shot. The rest of `tests/storage/test_corpus.py` is a
   recorded case and is not copied.
6. **The "before" for P14**, a scratch run of the Rust `read_corpus` over `data/processed/sessions/`
   (an example in `crates/storage`, deleted after) for `aaron`:

   | versions | sessions | dirs | swings | duplicates | `OUTDATED` | metrics with n > 0 |
   |---|---|---|---|---|---|---|
   | `{16, 16}` | 15 | 15 | 13 | 2 | 0 | 34 |
   | `{17, 17}` | 15 | 15 | 13 | 2 | **13** | **0** |

   Every survivor is excluded at `{17, 17}`; the two duplicates never reach the version test.
7. **For P14**: `corpus::tests::outdated_is_older_than_the_installed_engine_and_comparable_from_is_not_read`
   is the test that changes with the rule. Its second and third reads (`{17, 16}` and `{16, 17}`)
   swap answers, and its sentence assertion moves with the new `OUTDATED` detail. The comparison
   and the sentence are in `corpus_swing`; the module doc's "The engine generations are a parameter"
   section and `EngineVersions`' doc say "until P14" and need rewording. After the switch the
   scratch run above at `{17, 14}` should pool what `{16, 16}` does (0 `OUTDATED`, 34 metrics).
8. **For P13**: `run_corpus` in `crates/core/tests/storage.rs` is the second definition
   `run_storage` must share, beside `run_ops`, over `serde_json::Value` in and out. It reads
   `input.versions` as recorded.
9. **Verify**: `cargo test -p storage` 54 passed (52 unit, 2 schema pins); `cargo test` 900 passed,
   0 failed; `cargo clippy --all-targets` clean; `cargo fmt --check` clean; `pytest
   tests/test_docs_truth.py` 92 passed; `pytest` 2051 passed; `git diff -- src/ spec/schemas` and
   the engine, stage, screen and audio families empty; the storage (`7e45cd48…`), career
   (`627af088…`) and `data/` (`bd384cc4…`) digests unchanged.

### P11 (2026-10-08)

1. **What was built.** `crates/analysis`:
   - `stats` is now `analysis/stats.py` whole: the three 30-row tables, `Z_975`, `t_critical` and
     `chi2_critical` (private, as Python's are), `mean_and_sd`, `mean_ci` and `sd_ci`, with
     `tests/analysis/test_stats.py`'s interval cases ported. Its stale "they go with their callers,
     in §M29" doc is replaced (finding 6).
   - `baseline`: `pooled_samples`, `build_baseline`, `refuse` (all public, as Python's are), and the
     private `claim_phrasing` (a `match`, so a new claim without a phrasing fails the build where
     `_CLAIM_PHRASING` would raise), `baseline_for`, `sessions_of`, `session_means`, `interval`,
     `first_extreme` and `count`. Every case in `tests/analysis/test_baseline.py` is ported, plus
     the refusal sentences in full, the session tie, and the signed-zero range.
   - `lib.rs` declares `baseline` and gains an "M36 adds the career half" section. No dependency is
     added anywhere, and `analysis` reaches `contracts::{baseline, career}` only.
2. **The calls, each named in its module doc.** `mean_and_sd` answers `Option<(f64, Option<f64>)>`,
   `None` on an empty sample where Python raises (`percentile`'s precedent). `t_critical` and
   `chi2_critical` panic at df 0, where Python's `[df - 1]` would read the last row by negative
   index; both callers return first on `n < 2`. `pooled_samples` answers a `BTreeMap`, not the
   first-seen order Python's dict has, since nothing reads that order. `refuse` takes `n` and
   `n_sessions` as `i64` (the `WithheldClaim` fields and `minimum_n`'s type); `build_baseline` casts
   each `len()` once (P6's finding 2).
3. **`**` is C `pow`, measured.** A scratch comparison against CPython's own answers (a table written
   by `analysis/stats.py`, read by a throwaway example, both deleted): the product `x * x` missed 12
   of 20,000 random squares and `b * b * b` missed 1,307 of the 5,940 Wilson-Hilferty values for df
   31–3,000, because the UCRT's `pow` is not the correctly rounded product even at `y = 2`; `powf`
   missed none, and reproduced 2,400 random-sample intervals bit for bit. So `stats::pow` is
   `x.powf(black_box(y))`. **The `black_box` is load-bearing**: with the 2.0 visible, `powf` missed 0
   squares in a test build and 12 in a release build, because LLVM rewrites `pow(x, 2.0)` to `x * x`
   only when optimising, which would leave the gate certifying arithmetic the release verbs do not
   run. `powi` with a runtime exponent also matched, because MSVC lowers it to `pow`. That is a
   property of a target, so it was not used. **The career family cannot see this choice**: with the
   product, all 1,286 baseline floats were still bit-identical. So `stats`'
   `past_the_tables_the_answers_are_cpythons_to_the_bit` holds it, comparing CPython's answers at df
   31, 32, 74, 100 and 1000 and three squares with no tolerance. Three of those rows and all three
   squares are ones the product gets wrong (df 74's lower value is `past-the-tables`' n = 75). It
   is `#[cfg(all(target_os = "windows", target_env = "msvc"))]`, because another libm may answer a
   last bit differently and CPython over it would too.
4. **The compensated sums are held by a bit pin, because §3's `RTOL` cannot see them.** Plain folds
   in `mean_and_sd` moved 203 of the family's 1,286 baseline floats in a bit, and in
   `session_means` 32, and both passed the `RTOL` gate. `crates/core/tests/career.rs` therefore
   carries `every_baseline_float_is_frozen_pythons_to_the_bit` beside the gate (same `cfg`, same
   reason). It compares every float both sides have by bits: 0 of 1,286 differ.
5. **The gate**, `crates/core/tests/career.rs` (new): `every_career_vector_builds_its_baseline`
   reads each vector's `input.corpus` (after asserting `career_version == CAREER_VERSION` and
   `recorded_by == _run_career`), builds the baseline, and compares it through
   `golf_core::compare` with `expected.baseline` and `expected.properties.baseline`: 56 vectors,
   the real one included, and 161 metrics. `the_builder_withholds_exactly_what_it_refuses` holds
   every built metric to P6's finding 6, both ways: `ready` and `withheld` partition the claims, a
   ready claim's statistics are all present (a trend's are the session means), and a withheld
   claim's are all absent. `every_expected_key_is_checked_or_deferred_by_name` fails on any
   `expected` or `expected.properties` key that `CHECKED` (`baseline`, `properties`), `DEFERRED`
   (`dispersion`, `standing`, `bag_profile`, each "P12") or `ELSEWHERE` (`reports`, for
   `tests/reports.rs`) does not name, and on any name no vector carries.
6. **Negative controls**, each reverted after. Three fail the gate: the chi-square expansion's tails
   swapped (4 differences, `past-the-tables`), pooling with the dedupe dropped (107), and the t table
   read one row off (186). **Two pass the whole family**:
   - **The session-id tiebreak dropped from `sessions_of`.** `session-tie`'s first-seen order is
     already its id order, so it catches a sort on the timestamps' *text* (P3's point), not a
     dropped tiebreak. The unit test `sessions_tied_on_the_instant_sort_by_id` fails under it.
   - **`reduce(f64::min)` / `reduce(f64::max)` for the range.** No vector holds a signed-zero tie,
     and on this target the swap also passes the unit test, because x86_64's lowering keeps the
     accumulator on a tie. `f64::min` documents either answer for `±0`, so the explicit first-wins
     fold stays and `the_range_is_the_first_extreme_in_sample_order` holds the behaviour.
7. **For P12**:
   - `dispersion.py::_within_session_sd`'s `sd ** 2` and `(squares / degrees) ** 0.5` are C `pow`
     (finding 5, P3): reuse `stats::pow` (make it `pub(crate)`; it is private today), never `sqrt`
     or a product. Its `+=` accumulation is a plain fold, not `pyfmt::sum`. `dispersion.py:231`'s
     `_, sd = mean_and_sd(values)` meets the `Option` from item 2.
   - `pooled_samples` is a `BTreeMap`; `build_dispersion`'s `samples.get(name, [])` is
     `.get(name).map_or(&[][..], Vec::as_slice)`.
   - Empty `DEFERRED` in `career.rs` (the key pin then requires each of the three to be in
     `CHECKED`), and extend the bit pin to `dispersion`, `standing` and `bag_profile`. The
     helpers `corpus_of`, `bit_differences` and `count_floats` are there to reuse.
8. **For P13**: the career runner a re-record must share with this gate is, so far,
   `build_baseline(&corpus_of(..))` plus `baseline_differences`' `properties.baseline` object
   (`claims_ready`, `nothing_sayable`). P12 grows it to the other three aggregates.
9. **Verify**: `cargo test` 938 passed, 0 failed (`-p analysis` 380); `cargo clippy --all-targets`
   clean; `cargo fmt --check` clean; `pytest tests/test_docs_truth.py` 92 passed; `git diff -- src/
   spec/schemas` and the engine, stage, screen and audio families empty; the storage (`7e45cd48…`),
   career (`627af088…`) and `data/` (`bd384cc4…`) digests unchanged.

### P12 (2026-10-08)

1. **What was built.** `crates/analysis`:
   - `dispersion`: `build_dispersion`, `dispersion_for` (Python's `samples=()` default is `&[]`), and
     the private `decide_bias`, `decide_scatter`, `carry_refusals` (a `match` on the claim, so
     `Trend` is never carried, as `needed.get` answering `None` does), `add_session_evidence` (the
     drift caveat's two spreads through `pyfmt::general(x, 3)`), `within_session_sd` and
     `read_the_pair`. `test_dispersion.py` is ported case for case, plus the sentence frames in full
     (P7 finding 5), a hand-worked pooled spread, and P7's three registry pins
     (`POSE_MEASUREMENTS ∪ SHOT_MEASUREMENTS == METRIC_TARGETS`, flight and pivots disjoint), which
     needed `analysis`'s registries.
   - `comparison`: `build_standing`, `comparison_for` (through `load_distribution_any`, the
     `("all", "all", "all")` stratum, and `pyfmt::round_to(placement, 1)`) and the private `place`.
     `test_comparison.py` is ported, plus the band edges (inclusive for `Inside`, strict for
     `Outside`) and the three refusal sentences in full. Python's two-module import pin is
     `the_guarded_modules_still_reach_no_benchmarks`: it reads `baseline.rs` and `dispersion.rs` by
     `include_str!` and refuses any **code** line naming `benchmarks` (both module docs name it to
     say they do not reach it).
   - `club_profile`: `build_bag_profile(corpus, Option<&Bag>)` and the private `profile_for`,
     `bag_changed_caveats` (`Timestamp`'s instant order with a strict `<`, `date_ymd()` in the entry's
     own offset) and `mishit_caveats`. `test_club_profile_builder.py` is ported, plus the caveats in
     full, the plural forms, and an entry recorded at 23:30−05:00 dated the 3rd where UTC says the
     4th. Python's "imports no storage" pin is not ported as a test: `analysis` has no `storage`
     dependency, so the reach is a build failure, and the module doc says so.
   - `stats::pow` is `pub(crate)` (P11 finding 7) and its doc says why `dispersion` needs it.
     `lib.rs` declares the three modules and its M36 section names them, and that only `comparison`
     reaches `benchmarks`. `baseline.rs`'s doc points at the new pin. No dependency added anywhere.
2. **The gate**, `crates/core/tests/career.rs`, now checks every key but `reports` with no skips:
   `CHECKED` is the four aggregates and `properties`, `DEFERRED` is **empty and kept** (typed
   `[(&str, &str); 0]`) so the next key the recorder grows has a named place to wait, and the key pin
   also fails if an aggregate is built and not checked. Two tests were renamed for what they now
   hold: `every_career_vector_builds_its_baseline` → **`every_career_vector_builds_its_aggregates`**
   and `every_baseline_float_is_frozen_pythons_to_the_bit` →
   **`every_aggregate_float_is_frozen_pythons_to_the_bit`**. `the_builder_withholds_exactly_what_it_refuses`
   now walks each club profile's baselines too. Measured: 56 vectors, 161 golfer metrics, 35 club
   profiles, and **3,280 floats bit-identical** (baseline 1,286, dispersion 733, standing 416,
   bag_profile 845). Green on the first run.
3. **Both `**`s in `within_session_sd` are C `pow`, and the family cannot see either.** Controls,
   each reverted: `sqrt` for `(squares / degrees) ** 0.5`, and the product `sd * sd` for `sd ** 2`,
   each passed the gate, the bit pin and every unit test. CPython measured on this box (a scratch
   script, deleted): the UCRT's `pow(x, 0.5)` differs from `sqrt` on 524 of 1,000,000 random `x`,
   and both alternatives change `_within_session_sd`'s answer on ordinary two-decimal samples. So
   `dispersion`'s `the_pooled_spread_is_cpythons_to_the_bit` (`cfg` windows/msvc, as P11's pins
   are) holds six sample pairs to CPython's answers by bits, three that separate the product and
   three that separate `sqrt`, and recomputes each alternative to prove the row still separates
   it. Both controls now fail it, and it passes in debug and release.
4. **The other controls**, each reverted after: the groups pooled in reverse order fails the bit pin
   only (2 floats, so the first-seen order is load-bearing and the gate's `RTOL` cannot see it);
   `:g` for `:.3g` fails the gate (1, `dispersion-drift`) and a unit test; the drift test `>` as
   `>=` fails the gate (1: a recorded case sits on the factor exactly); `round` dropped from the
   percentile fails the gate and the bit pin (43 each); the caveat fed `corpus.swings` for the
   narrowed ones fails the gate (10) and a unit test; `<=` for the predating `<` fails the gate (1)
   and two unit tests. **One passes the family**: `Inside` with strict edges, since no recorded
   interval ends exactly on a band edge. `the_band_edges_place_as_pythons_comparisons_do` catches
   it.
5. **For P13**: the career runner the re-recorder must share with this gate is `career.rs`'s
   `built(id, vector) -> Value`: `corpus_of` (the version and recorder asserts) and `bag_of`
   (`input.bag` as `Option<Bag>`), then all four builders, giving one document in `expected`'s shape
   minus `reports` (`baseline`, `dispersion`, `standing`, `bag_profile`, and `properties` with each
   aggregate's derived counts, `_run_career`'s keys). `differences(vector, built)` compares it
   aggregate by aggregate under `golf_core::compare`. Lift both into `crates/core/src` and have the
   gate call them. The aggregates read only `input.corpus` and `input.bag`; `input.clubs`,
   `display_name` and `versions` are the reports' and the storage family's.
6. **For P16**: `bag-declared`'s `input.clubs` ends in `5w`, which has no profile in
   `expected.bag_profile` (neither hit nor declared), so the `club_profile --club 5w` report text is
   the no-profile branch. `BagProfile::profile_for` answers `None` for it.
7. **Verify**: `cargo test` 1,007 passed, 0 failed (`-p analysis` 449); `cargo clippy --all-targets`
   clean; `cargo fmt --check` clean; `pytest tests/test_docs_truth.py` 92 passed; `git diff -- src/
   spec/` empty, and the engine, stage, screen and audio families untouched; the storage
   (`7e45cd48…`), career (`627af088…`) and `data/` (`bd384cc4…`) digests unchanged.

### P13 (2026-10-08)

1. **What was built.** `crates/core`:
   - `storage_family` (new): `run_storage(vector) -> Result<Value, String>`, `StorageCase::of`
     (the dispatch on `provenance.recorded_by`, `RUN_OPS` and `RUN_CORPUS`), and `differences`,
     which is `compare`'s verdict with each moved JSON file explained inside. The op and corpus
     runners are `tests/storage.rs`'s, lifted as they were.
   - `career_family` (new): `run_career(vector)` (the gate's `built`, less the version assert, which
     the gate keeps and a re-record must not make), `differences`, `adopted_from`, `AGGREGATES`,
     `CARRIED`, `RUN_CAREER`.
   - `rerecord`: `Version::Career` (`career_version`, held to `CAREER_VERSION`, refusals say "this
     many-shot layer is at N"); the declaration's three version keys, exactly one, and `removed`
     refused on a career declaration as on an engine one (`Version::may_remove`, screen alone);
     `STORAGE`, `CAREER`, `STORAGE_DOCUMENTS` (`corpus`, `bundle`, `stores`), `CAREER_DOCUMENTS`
     (`synthetic`, `real`); `Family::{Storage, Career}`; `walk_career`; the placed-sub-family check
     shared with the screen walk (`every_entry_is_placed`); career ledger entries in the engine's
     shape, with no `removed` list; the report's count line `N vectors run (S storage, C career)`.
   - `lib.rs` declares both modules; the binary's doc and usage name the third key; `Cargo.toml`
     makes `storage` a normal dependency.
   - **The Files line grew** by the two new modules, `tests/{storage,career}.rs` and
     `crates/core/Cargo.toml`, which P8 finding 10, P10 finding 8, P11 finding 8 and P12 finding 5
     asked for: both gates now call the lifted runners, so the gate and the recorder run one
     definition. The career gate's difference paths gained the `.` they lacked
     (`baseline.metrics…`, not `baselinemetrics…`).
2. **`reports` is carried, not run** (the section left it to this phase). `CARRIED = ["reports"]`:
   a career run's `ours.expected` is `run_career`'s document plus each carried key copied from the
   committed vector. Built from `run_career` up rather than from the committed `expected` down, so a
   key the recorder grows that is neither built nor carried is a removed key, which no career
   declaration can pass. `tests/career.rs` pins every `CARRIED` key to be named in its `ELSEWHERE`.
3. **The family's spelling is part of the answer, not the comparison.** Two things the gate did
   before comparing now happen inside `run_storage`: a `shot.get`/`shot.all` answer on frozen
   Python's keys (P8's rule), and **a written JSON file keeps its committed text wherever its value
   agrees** under §3's rules (`in_the_recorded_spelling`, the file-level counterpart of an
   in-tolerance float keeping Python's bits). So the gate's verdict is `compare(expected,
   run_storage(vector))`, exactly what `rerecord` gates on. Measured (control, reverted): without
   the respelling, **11 written files across the 61 vectors** differ from Python's text in spelling
   alone; the gate and four `rerecord` tests fail on them.
4. **An adopted corpus is derived, so the two families cannot drift apart** (call 2's property,
   which no phase built once Rust could write either family). 33 career vectors (`storage-<case>`
   for each of the 32 synthetic corpus cases, and `career/real/aaron`) carry a storage corpus case's
   `expected.corpus` as their `input.corpus`, named only by the prose of `provenance.source`
   (`spec/vectors/storage/corpus/<case>.json[.gz]`, then `,` or a space). `walk_career` runs the
   storage family first, keeps each document *as it will be written*, and runs every adopting career
   vector on its source's corpus: a source whose answer moved, or a copy that drifted from an
   unchanged source, is a difference at `input.corpus…` that the declaration must name. It is the
   one `input` a re-record can change. New pin: `tests/career.rs::every_adopted_corpus_is_its_storage_vectors_answer`
   (the committed families agree, value for value, all 33). Controls: without the derivation, a
   source declared alone passes and leaves its twin's copy stale (the test fails).
5. **A store's written file cannot be declared yet, and is refused by name.** Its key is a path
   (`…/manifest.json`), so a declared `expected.files.<path>` matches the gate as a string and then
   walks to another place: `apply` would have panicked, or copied a decoy. `one` now asks `try_apply`
   (`apply` keeps its panicking contract over it) and checks every declared difference is gone after
   the walk; either failure is a refusal naming the path. Pinned by
   `a_declared_path_through_a_dotted_key_is_refused_by_name`. Nothing in M36 moves a store's write.
6. **No new dependency.** `tempfile` was not promoted (the rules keep it dev-only): the runner's
   scratch root is its own `Scratch` (pid plus a counter under the system temp dir, removed on
   drop), and `tempfile` left `crates/core`'s dev-dependencies, whose one user was the lifted runner.
   It stays a dev-dependency of `crates/storage`. No scratch directory is left behind after the
   suite.
7. **Tests.** Unit: the three-key declaration (two keys or three refused, `careers_version` an
   unknown field), `removed` refused on a career declaration (empty included), the career ledger
   entry's shape and the removed-key refusal's text, the dotted-key refusal, the file respelling,
   the shot file on frozen Python's keys, `Scratch`, `adopted_from` on both spellings and three
   wrong paths, and both runners' unknown-recorder `Err`. Integration, `tests/rerecord.rs`, each on a
   copy of both families whole (a career run over all 117 takes 0.8 s in release): an empty
   `career-v0` finds nothing and writes nothing, dry or not; the version guard, before a vector is
   read; `career_version` aged one back is refused on every document at once by an empty
   declaration, then lands on both families alone, ledgered, CRLF and gzip kept, every other family
   byte for byte, and a second run writes nothing; the adopted-corpus test (finding 4) and its drift
   case; a career run with every engine, stage and screen document unreadable passes, and an engine
   and a screen run with every storage and career document unreadable (and an unplaced
   `storage/unplaced/`) pass and write none of them; an unplaced directory, a loose file and an
   unknown recorder each stop the walk as `Refused::Files` rather than panicking.
8. **Negative controls**, each reverted: the adopted corpus not derived (1 test); `reports` not
   carried (4 tests); the files not respelled (the storage gate, 11 differences, and 4 tests).
9. **For P14**:
   - **The declaration names the copies too.** Each storage corpus case whose `OUTDATED` sentence
     moves has an adopted twin, `career/synthetic/storage-<case>`, whose
     `input.corpus.excluded[i].detail` moves with it in the same run (finding 4); `career-v1.json`
     names both paths (and any `expected.narrowed.<name>.corpus.excluded[i].detail` a case holds),
     plus `career_version` on every vector of both families. The section's "No `career/` value
     moves … if one does, stop" is about the career family's **answers**: a twin's `input` following
     its source is the copy being kept equal, not a career value moving. If any path under a career
     vector's `expected` moves, stop as the section says.
   - **The twins' reports go stale until P15/P16.** `career_corpus_verbose` prints each exclusion's
     detail sentence, and `reports` is carried (finding 2), so a twin whose sentence moved keeps
     frozen Python's sentence in its report text. P15/P16 meet it (below).
   - **`hand/`**: add it to `STORAGE_DOCUMENTS` with the first file in it (a listed directory that
     does not exist is a refusal, by design). `run_storage` dispatches on `provenance.recorded_by`,
     and M34's hand vectors carry no `recorded_by` (`kind`, `note`, `oracle`, `source`), so a hand
     corpus case either names `storage_family::RUN_CORPUS` as the runner it is worked against or the
     dispatch learns another key; `tests/storage.rs` walks the family recursively and dispatches
     through `StorageCase::of`, which refuses an unknown recorder. Decide and say which.
   - The real dry run's line to compare against: `117 vectors run (61 storage, 56 career)`.
10. **For P15 and P16**: when `run_career` renders the reports, empty `CARRIED` and move `reports` in
    `tests/career.rs` from `ELSEWHERE` to where it is checked (the pin from finding 2 then has
    nothing to hold). The first re-record that runs them will find the P14-moved sentences in the
    adopted twins' `career_corpus_verbose` text, and any text P14's rule changes in the plain
    `career_corpus` report; those are declared then, at whatever `career_version` that phase
    decides.
11. **For P17**: `docs/CONFORMANCE.md` §4 "`rerecord`" says two version keys and two families: it
    gains `career_version`, the carried reports, the adopted-corpus rule, the respelled files, and
    the dotted-key refusal (and that M35, the first change to a store's write, builds the escape).
    `docs/ARCHITECTURE.md` §2's edge is now `core → storage` (normal), not P8 finding 11's
    `core ⇢ storage` (dev). `CLAUDE.md`'s `crates/core` sentence ("the engine and stage families, or
    since M34 the screen family") gains the storage and career families.
12. **Verify**: `cargo test` 1,022 passed, 0 failed (`golf-core`: lib 38, `tests/rerecord.rs` 18,
    `tests/storage.rs` 2, `tests/career.rs` 5); `cargo clippy --all-targets` clean; `cargo fmt
    --check` clean; `pytest tests/test_docs_truth.py` 92 passed; `golf-core rerecord --declare
    <scratchpad>/career-v0.json --dry-run`, and again without `--dry-run`: "nothing differs from the
    committed vectors", 117 run, 0 written; `rerecord --dry-run` over `v17.json` and `screen-v1.json`
    nothing differs; `git diff -- src/ spec/` empty, `spec/` holding only P2–P4's untracked
    additions; the storage (`7e45cd48…`), career (`627af088…`) and `data/` (`bd384cc4…`) digests
    unchanged.

### P14 (2026-10-08)

1. **Resumed.** The first attempt (marker 15:54) built the whole section and ran the re-record for
   real (16:05), and was cut off at 16:11 while widening two `tests/test_conformance.py` pins. This
   session read every file it left against the section, found each correct and complete, changed no
   code, and verified. What landed:
   - `crates/contracts/src/swing.rs`: `COMPARABLE_FROM = 14` and its doc (two questions, two
     numbers; why 14; it moves on a `[disagrees]` entry alone), and a compile-time
     `COMPARABLE_FROM <= ANALYSIS_VERSION`. Ledger entries from 17 are spelled `(when) [class]:`, and
     16 → 17 is `[shape]`. The sibling test is `comparable_from_is_the_newest_disagreeing_generation`:
     every entry from 17 names one of `shape`, `missing` or `disagrees`, and the constant is the
     newest `[disagrees]` entry, else 14.
   - `crates/contracts/src/career.rs`: `CAREER_VERSION` 1, with ledger entries 0 and 1, and the
     `ExclusionReason::Outdated` doc naming both rules.
   - `crates/storage/src/corpus.rs`: `is_older_than(analysis, versions.comparable_from)`. The
     sentence is now *"analyzed by engine version N, older than L, the oldest engine whose numbers
     still agree with today's — they are not comparable with a swing analyzed today, so they are
     reported rather than pooled. Re-run scripts/reanalyze.py"*. The module doc and `EngineVersions`'
     doc no longer say "until P14", and the unit test is now
     `outdated_is_older_than_comparable_from_and_installed_is_not_read`, its two middle reads
     swapped. `state.rs` is untouched.
   - **The Files line grew**, and each addition was asked for. `crates/core/src/storage_family.rs`
     answers P13's finding 9: **a hand case names its runner in `provenance.worked_against`**
     (`scripts/conformance_vectors.py::_run_corpus`, i.e. `RUN_CORPUS`), which is read only when
     `provenance.oracle` is `"hand"` (`HAND_ORACLE`). A hand case naming `recorded_by`, or a Python
     case naming `worked_against`, is refused (`a_hand_case_names_the_runner_it_was_worked_against`).
     `crates/core/src/rerecord.rs` adds `hand` to `STORAGE_DOCUMENTS`. `crates/core/tests/rerecord.rs`'s
     aged-version tests now allow for the committed `career-v1` ledger entry. `tests/test_conformance.py`
     (a file the rules allow) changes three pins. The committed-family pins accept a non-zero
     `career_version` on a vector that carries `provenance.rerecords`. Only `corpus`, `bundle` and
     `stores` are held to `oracle: "python"` (`_PYTHON_RECORDED_STORAGE`). `list` shows each
     vector's own version.
2. **The hand-worked vectors, three corpus cases under `spec/vectors/storage/hand/`.** Each has
   `oracle: "hand"`, `worked_against`, `career_version` 1 and its working in `provenance.note`.
   - `versions-either-side`: `{17, 14}` over swings stored at 13, 14, 16 and 17. 13 is excluded; 14,
     on the line, pools; 16 pools where frozen Python's rule would exclude it.
   - `comparable-from-sixteen`: `{17, 16}`, where the line is the caller's number and not the
     constant. 15 is excluded and 16, on the line, pools, plus one narrowing.
   - `mishit-sees-only-comparable`: nine 7i swings, four of them stored at 13 with carries
     120–135. The five comparable carries have median 152 and floor 76, so swing 6's 70 is flagged.
     Pool all nine and the median would be 135, with nothing flagged.

   **Checked independently** by a scratch script, since deleted. Frozen Python's `_read_and_narrow`
   ran over each tree with its `ANALYSIS_VERSION` patched to the case's `comparable_from`, because
   Python's `<` against that line *is* Rust's new rule. It matches every expected value, the
   `detail` sentences aside.
3. **The re-record.** `career-v1.json` adds nothing and moves 23 paths: `career_version`, 7
   `expected.corpus.excluded[i].detail`, 8 narrowings' `excluded[1].detail` and 7
   `input.corpus.excluded[i].detail`. The sentences moved in four storage corpus cases
   (`outdated-versions` 7, `narrowing` 9, `stale-and-outdated` 1, `mishit-ineligible` 1) and in the
   `input.corpus` of those cases' four adopted career twins. Every Python-recorded vector in both families now has
   `career_version` 1 and one `provenance.rerecords` entry. `hand/` carries none, since it was
   written at 1. Every plain file kept its CRLF, and both real files stayed gzipped. The dry run now
   prints *"nothing differs from the committed vectors / 120 vectors run (64 storage, 56 career); 0
   changed; 0 files written"*. That is P13's 117 plus the three `hand/` cases, and running it again
   for real writes nothing.

   **An audit**, by a scratch script since deleted, compared frozen Python's in-memory
   `build_storage` and `build_career` with the committed families. The only differences are
   `career_version`, `provenance.rerecords` and the declared paths, plus four twins'
   `expected.reports.career_corpus_verbose`. Those four are the audit's own artifact:
   `build_career` adopts the **committed** storage corpus and so renders the new sentence, while the
   committed report is carried unchanged at frozen Python's text. No career answer moved.
4. **The measurement**, Rust's `read_corpus` over `data/processed/sessions/` for `aaron` (a
   `crates/storage` example, since deleted). Frozen Python's own `read_corpus` over the same
   directory gives 13 swings, 0 outdated and 34 metrics with n > 0, so the two pool the same swings:

   | versions | sessions | dirs | swings (distinct) | shots | duplicates | `OUTDATED` | metrics n > 0 |
   |---|---|---|---|---|---|---|---|
   | `{16, 16}` | 15 | 15 | 13 | 13 | 2 | 0 | 34 |
   | `{17, 17}` | 15 | 15 | 13 | 13 | 2 | 13 | 0 |
   | `{17, 14}` | 15 | 15 | 13 | 13 | 2 | **0** | **34** |

5. **For P15 and P16 (P3's finding 6, answered: the report text stays as frozen Python printed it
   in P14).** `reports` is carried (P13's finding 2), so no re-record can move report text until a
   renderer runs it. When `career_corpus` is rendered:
   - The four twins' `career_corpus_verbose` carries frozen Python's sentence (*"analyzed by engine
     version 0, and 16 is installed — the numbers are not comparable…"*), but their `input.corpus`
     now holds the new one. A renderer that prints `detail` verbatim differs there. That is P14's
     rule reaching the report, and it gets declared.
   - The plain paragraph *"N distinct swing(s) analyzed by an older engine (version …; M is
     installed)."* comes from Python's own `ANALYSIS_VERSION`, not from `detail`. A career vector's
     input holds no engine versions, while the verbs know `{ANALYSIS_VERSION, COMPARABLE_FROM}`, so
     P15 decides what the renderer prints there and where it gets the number. Over `data/` the
     paragraph never prints (0 `OUTDATED` at `{17, 14}`), so P16's parity run cannot see it and
     only the vectors gate it.
   - **Declaring it.** `rerecord`'s `unmatched` counts a path an earlier ledger entry recorded as
     matched (`ledgered`), so adding the report paths to `career-v1.json` and running it again may
     pass. The other route is a bump to `CAREER_VERSION` 2, whose doc asks for one whenever a family
     would record something different. That route moves decision 8's "M35's `read_corpus` change is
     `career-v2`" one number on. P15 decides and says which.
6. **For P17.** CONFORMANCE §2 should cover the `hand/` storage cases and `worked_against`, and §4
   the career-v1 run, as the first career re-record and an example of the adopted-copy rule.
   ADR-024's addendum takes finding 4's table as its evidence.
7. **Verify**: `cargo test` 1,024 passed, 0 failed; `cargo clippy --all-targets` clean; `cargo fmt
   --check` clean; `pytest` 2051 passed; `ruff check src tests scripts` clean; `conformance.py
   check` green (storage 64, career 56, deferred); `rerecord --declare
   spec/declarations/career-v1.json --dry-run` as finding 3; `git diff -- src/ spec/schemas` and
   the engine, stage, screen and audio families empty; `data/` digest unchanged (`bd384cc4…`). **New
   family digests, which replace P2's and P3's for every later phase** (by P1's command): storage
   `1df5c3bf032e39d25d1b561ccc57dc6d4a739dc53c8b455311771d147d349c6c` (`753d49ac…` without
   `hand/`), career `38a4f8ce8201f50440f44c592160bebeb2e4209b8328b4c7dc78023ce21ff64c`.

### P15 (2026-10-08)

1. **What was built.** `crates/core/src/reports/`: `mod.rs` holds the shared pieces, `Printed`
   (each `print` one line and its `"\n"`), `header`, `plural`, **one** `fmt` precision table where
   the scripts keep three identical copies (`club_profile.py`'s included, so P16 takes it), and
   `wrap(text, width)` (`pyfmt::split`, code-point `len`, `pyfmt::strip`). It also holds the verbs'
   `main` as library code: `Targets`/`targets` (the scripts' golfer lookup and slug), `DataDirs::resolve`
   (call 11), `Verb`, `Request`, `Answer` and `answer(verb, request, versions)`. `corpus.rs`,
   `baseline.rs` and `dispersion.rs` port each `_report` and its private helpers, the scripts'
   reason-carrying comments condensed beside them. `bin/golf_core.rs` gained `career-corpus`,
   `career-baseline` and `career-dispersion`, which parse flags and print. Exit codes are the scripts':
   0 always, an empty registry included, and 2 for a `--name` that slugs to nothing. A flag argparse
   refuses also exits 2, and `-h`/`--help` prints the usage and exits 0. Two argparse conveniences are
   named in the binary's doc as not ported: abbreviated flags, and help text drawn from the docstring.
   `--flag=value` parses, and a repeated value flag keeps the last value.
2. **`--json`** prints one array with an object per golfer the report would cover. Each object holds
   `display_name` and the aggregate the report was rendered from, under the career family's name for
   it: `corpus`; `baseline` and `standing`; `dispersion`. In JSON mode stdout is JSON and nothing else,
   so a refusal's sentence goes to stderr and an empty registry prints `[]`.
3. **`run_career` renders three reports, and the carried set is now per script.** `career_family`:
   `RENDERED = [career_corpus, career_baseline, career_dispersion]` and `CARRIED = [club_profile,
   flag_mishit]`, both naming **scripts** rather than keys of `expected`. `script_of(key)` maps a
   report key to the script whose name it is, alone or followed by `_`. `carry(ours, committed)` copies
   the carried scripts' reports and nothing else, so a report key that no script printed is still a
   removed key the comparator refuses. `rerecord.rs`'s loop became `carry(...)`, and its doc links name
   `crate::career_family::CARRIED`. `tests/career.rs` keeps `reports` in `ELSEWHERE`, with the text
   now naming `reports.rs`. Its old per-key `CARRIED` pin became one assertion: `RENDERED` and
   `CARRIED` cover the five scripts, and `reports` is gated elsewhere. `run_career` now reads
   `input.display_name` and `input.versions.installed`.
4. **P14's finding 5 had one fact wrong: every career vector's input does carry engine versions.**
   `input.versions` is `{installed: 16, comparable_from: 16}` on each Python-recorded case (P3's
   finding 3). **Decided: the plain outdated paragraph stays as frozen Python wrote it, and `M` is
   the caller's `installed`**: `input.versions.installed` in the family, `ANALYSIS_VERSION` in the
   verbs. It is still true under `career_version` 1. An excluded swing is older than
   `COMPARABLE_FROM`, so it is also older than the installed engine, and "not comparable with a swing
   analyzed today" is now exactly the rule. So the plain report moves on no vector. `corpus.rs`'s
   module doc gives the reason.
5. **The re-record: `career-v1.json` amended, not a `career-v2`.** The gate's first run differed in
   exactly the four adopted twins' `career_corpus_verbose`, as P14 predicted, and in nothing else.
   No rule changed between P14 and P15: the renderer started running, and the sentence P14 moved
   reached a report. So `expected.reports.career_corpus_verbose` was added to `career-v1.json`'s
   `moved` list, its note says so, and the declaration ran again. The dry run moved those 4 paths
   only ("120 vectors run (64 storage, 56 career); 4 changed"). The real run wrote 4 files, and a
   second run wrote nothing. Each of the four files now has **two** `career-v1` ledger entries: the
   P14 one, then `moved: ["expected.reports.career_corpus_verbose"]`. The typo guard passes the 23
   older paths through `ledgered`. `CAREER_VERSION` stays 1, so decision 8's "M35 is `career-v2`"
   still holds. Its doc for 1 now records the second run. **New career digest** (by P1's command):
   `9d95149c21cbf5d2f7b531de148644bcbb9d39804b868bd6e373e92919c764bf`. Storage is unchanged
   (`1df5c3bf…`).
6. **The gate**, `crates/core/tests/reports.rs`, has seven tests:
   - **every rendered report is identical to the recorded text, character for character**: 336
     reports, the real one included, which is 56 vectors times six. It also checks that
     `run_career` renders no report the vector lacks.
   - every report key belongs to a rendered script or to a carried one, and `DEFERRED` (each carried
     script, with "P16") equals `CARRIED`.
   - `targets`: slug, id, the id beating the name, empty strings counted as absent, `(not
     registered)`, and both refusals.
   - a verb over `storage/hand/versions-either-side`'s materialised tree prints `career_corpus(expected.corpus)`
     plus one blank line, plain and verbose. That case is read at `{17, 14}`, the verbs' own versions,
     and the test refuses a case at any other versions. The outdated paragraph names version 17.
   - the refusals' text, and their JSON-mode forms.
   - the `--json` shape.
   - the binary end to end through `CARGO_BIN_EXE_golf-core`: both flag spellings, an unknown flag
     and a missing value (exit 2), and `--help` (exit 0).

   Unit tests: the wrap's branches, `fmt`'s default, the data-directory precedence, `prefix`,
   `script_of` and `carry`. The test scratch directories are a local `Scratch`, since `tempfile` stays
   out of `crates/core`.
7. **A quick parity run over `data/`, ahead of P16.** Each of the three verbs was run against its
   script with no args, `--verbose`, `--name Aaron`, `--player-id aaron --verbose`, `--player-id
   nobody`, `--name !!!` and `--name=aaron --verbose`: 21 runs. All 21 matched, exit codes included,
   **once two Windows differences on the Python side were removed**. Python's text-mode stdout writes
   `\r\n` for each `\n`. In a pipe it also encodes as cp1252 unless `PYTHONIOENCODING=utf-8` is set,
   which turns each em dash into `0x97`. Rust writes UTF-8 and `\n`. The run set
   `PYTHONIOENCODING=utf-8` and stripped `\r`. Every verb exits 0 over `data/`.
8. **For P16**:
   - Render `club_profile` (plain, `_verbose`, and `club_profile_<club>` for each `input.clubs`) and
     `flag_mishit_list` inside `run_career`. Then move both scripts from `CARRIED` to `RENDERED`, empty
     `CARRIED` and `reports.rs`'s `DEFERRED` together (the pin holds them equal), and add
     `Verb::ClubProfile`. `flag-mishit` writes, so it is not a `Verb` of `answer`'s kind unless P16
     chooses that.
   - Reuse `reports::{fmt, wrap, header, plural, Printed}`. `club_profile.py` wraps at
     `94 - indent - 10`, so pass `wrap` that width.
   - If a newly rendered report differs from the carried text because of `career_version` 1's
     sentence, declare it the same way: add its path to `career-v1.json` and run it again. A
     difference with any other cause is a renderer bug.
   - **The parity run must normalise the Windows differences in finding 7, or state that they
     differ.** Python's output differs from Rust's byte for byte in `\r\n`, and in cp1252 unless
     `PYTHONIOENCODING=utf-8` is set. Decision 3's "byte-for-byte" holds on the text, not on the
     console encoding. Say which comparison was made.
9. **For P17**:
   - CONFORMANCE §4 should cover `career-v1`'s second run: a declaration amended in the same
     uncommitted change once its effect reached a newly rendered report, and the two-entry ledger it
     leaves.
   - §2's career section gains `reports.rs` as the text gate, and the per-script carry.
   - `CLAUDE.md`'s commands block can take `cargo run --bin golf-core -- career-corpus --name Aaron`
     as the verb example.
   - ARCHITECTURE §1 lists the verbs and their `--json`.
10. **The Files line grew**, each addition asked for by P13 finding 10 or P14 finding 5:
    - `crates/core/src/{career_family,rerecord}.rs` and `crates/core/tests/career.rs`: the carry.
    - `crates/contracts/src/career.rs`: `CAREER_VERSION`'s doc only.
    - `spec/declarations/career-v1.json` and the four re-recorded
      `spec/vectors/career/synthetic/storage-{mishit-ineligible,narrowing,outdated-versions,stale-and-outdated}.json`.
11. **Verify**: `cargo test` 1,037 passed, 0 failed (`golf-core`: lib 45, `tests/reports.rs` 7,
    `tests/career.rs` 5, `tests/rerecord.rs` 18); `cargo clippy --all-targets` clean; `cargo fmt
    --check` clean; `pytest tests/test_docs_truth.py tests/test_conformance.py` 342 passed;
    `conformance.py check` green (storage 64, career 56, deferred); `rerecord --declare
    spec/declarations/career-v1.json` dry, real and again as in finding 5; `git diff -- src/` and
    the engine, stage, screen, audio and schema trees empty; the `data/` digest unchanged
    (`bd384cc4…`).

### P16 (2026-10-08)

1. **What was built.** `crates/core/src/reports/club_profile.rs` ports `club_profile._report` and
   every helper only it calls (`_report_club`, `_report_metric`, `_report_findings`,
   `_print_bag_entry`, `_no_metrics_reason`, `_report_empty`, `_print_untagged`), reusing P15's
   `fmt`, `wrap`, `header`, `plural` and `Printed`. Its label column takes an indent and its
   paragraphs wrap at `94 - indent - 10`. `reports/flag_mishit.rs` holds `flag_mishit_list(player_id,
   &BagProfile)` (one golfer's `_list` block), `MishitAction`, `action(...)` (the script's own check
   after argparse: `--list` wins over a reference, otherwise a reference and exactly one verdict) and
   `flag_mishit(action, dirs, versions, now) -> Answer`, the script's `main` from its store on.
   `reports/mod.rs` gained `Verb::ClubProfile` and `Request.club`. `answer` parses `--club` through
   `parse_club` **before** it looks up a golfer, as the script orders it, so `--club wedge --name !!!`
   prints the club refusal. It reads each golfer's bag through `BagStore` over the golfers
   directory. The binary takes `club-profile` through `parse_career`, which accepts `--club` for that
   verb alone. The other three refuse it as `unrecognized arguments`, as their scripts' argparse does.
   `flag-mishit` has its own `parse_mishit`, with one optional positional, and the binary is the only
   place `Timestamp::now_utc()` is read (call 4).
2. **`run_career` renders all five scripts.** `RENDERED` holds the five, and `CARRIED` is
   `[&str; 0]`. `carry` stays as a no-op, the named place a sixth script's report would wait.
   `club_profile` and `club_profile_verbose` are rendered over the case's bag, `club_profile_<club>`
   (plain) once per `input.clubs`, and **`flag_mishit_list` over `build_bag_profile(&corpus, None)`,
   with no bag**, as `_list` builds it. `reports.rs`'s `DEFERRED` is empty.
3. **No re-record, and no declaration change.** `rerecord --declare
   spec/declarations/career-v1.json --dry-run`: "nothing differs from the committed vectors", 120
   run, 0 changed. Neither newly rendered script prints an exclusion's detail sentence, so
   `career_version` 1's rule reaches neither report. P15 finding 8's "declare it the same way" was not
   needed. Both family digests are unchanged.
4. **The gate.** `tests/reports.rs` holds **595 reports** identical to the character (P15's 336, plus
   each case's `club_profile` pair, its `club_profile_<club>`s and `flag_mishit_list`). It has four
   new tests:
   - `club-profile` over the hand tree with a bag beside the golfer: `--club` absent, empty, `seven
     iron`, `pw` (the no-profile branch), the declared-entry line, and the refusal in both modes.
   - **`flag-mishit`'s write path on a scratch copy of `storage/hand/versions-either-side`'s tree.**
     Each of `--confirm`, `--clear` and `--auto`: `load_manifest` reads it back, `updated_at` is the
     clock passed in, and the raw `mishit` value is `null` or a member of
     `spec/schemas/swing_manifest.schema.json`'s `MishitVerdict` enum, which is the set frozen Python
     reads (clause 4). `read_corpus`'s `manual_mishit` follows it, and every other file stays
     byte-identical. The reference refusals and a missing swing write nothing.
   - `--list`: no name, empty, `  AARON `, `nobody`, and `!!!` (`no golfer ''`, the script's answer).
   - the `flag-mishit` binary end to end, including five argparse refusals (exit 2, stdout empty).

   **No `tempfile`**, though the section's gate names it. It is `storage`'s dev-dependency alone
   (P13 finding 6), and P15's local `Scratch` in `tests/reports.rs` serves the same purpose.
5. **`--json`.** `club-profile --json` prints `{display_name, bag_profile}` per golfer: the whole
   profile, because `--club` narrows the text and not the JSON (one club's report is
   `profile_for` on it). **`flag-mishit` takes no `--json`.** Its listing's rows are the bag profile's
   per-club counters (`n_shots`, `mishits`, `mishits_unconfirmed`, `mishit_refs`), which a bag does not
   change and `club-profile --json` already prints. A flag's answer is one line.
6. **The parity run: 85 runs, all identical.** The scratch script is
   `<scratchpad>/p16_parity.py` and is not committed. It ran each Python script and its verb over
   `data/` and compared **stdout bytes and the exit code**:
   - **The comparison.** Python ran with `PYTHONIOENCODING=utf-8`, and its `\r\n` became `\n` (P15
     finding 7). Rust's stdout was compared raw and checked to hold no `\r`. So the text is
     byte-identical, and the console encoding and line endings are Windows' alone.
   - **The flag sets.** The three P15 verbs over 10 flag sets each (none, `--verbose`, `--name
     Aaron`, `--name=aaron --verbose`, `--player-id aaron` plain and `--verbose`, `--player-id nobody`,
     `--name !!!`, `--name ""`, and both flags). `club-profile` over those 10 plus 27 more: each of the
     bag profile's clubs and `driver`, `5w`, `lw`, `putter`, five spellings (`7 iron`, `seven iron`,
     `Driver`, `3 wood`, `PW`), four `--club … --verbose`, `--club` beside `--name` and an unregistered
     id, `--club ""`, three refusals (`wedge`, `iron`, `xyz`), and `--club wedge --name !!!` for the
     order. `flag-mishit` over 13: the `--list` forms (with `Aaron`, `aaron`, `nobody`, `!!!`, `""`,
     and a reference beside it), four malformed references, and two flags on a swing that does not
     exist. Five argparse refusals were checked by exit code and empty stdout.
   - **What was not compared.** stderr where argparse prints, since its usage text is not ported (the
     binary's doc). **No real `SESSION/SWING` was given a verdict over `data/`.** The `data/` digest
     was `bd384cc4…` before the run and after it.
7. **Named, not measured by the parity run.** Neither path is reachable over `data/`:
   - `flag-mishit`'s argparse refusal prints `golf-core: flag-mishit: error: …` and the `golf-core`
     usage on stderr, where argparse prints its own usage. Both exit 2.
   - A manifest that cannot be written exits 1 with one sentence on stderr, where the script dies on
     the `OSError` with a traceback, which also exits 1.
8. **For P17**:
   - ARCHITECTURE §1 lists all five verbs. `club-profile` takes `--club` and its `--json` prints
     `bag_profile`, and `flag-mishit` writes one manifest and takes no `--json`.
   - CONFORMANCE §2's career section: all five scripts are rendered, `CARRIED` is empty, and
     `tests/reports.rs` gates every report to the character. `flag_mishit_list` is rendered over the
     profile with no bag. §4 can say that P16 needed no declaration change.
   - `crates/contracts/src/club_profile.rs`'s module doc already says the verb renders it (M36 P16),
     and that is now true.
9. **The Files line grew**, each addition a stale "until P16" sentence, or P15 finding 8's move:
   - `crates/core/src/{career_family,lib,rerecord}.rs` and `crates/core/tests/career.rs` (the
     `ELSEWHERE` text).
   - `crates/contracts/src/career.rs`, in `CAREER_VERSION`'s doc only.
10. **Verify**: `cargo test` 1,042 passed, 0 failed (`golf-core`: lib 46, `tests/reports.rs` 11);
    `cargo clippy --all-targets` clean; `cargo fmt --check` clean; `pytest tests/test_docs_truth.py`
    92 passed; the `career-v1` dry run as in finding 3; `git diff -- src/` and the engine, stage,
    screen, audio and schema trees empty; storage (`1df5c3bf…`), career (`9d95149c…`) and `data/`
    (`bd384cc4…`) digests unchanged.

### P17 (2026-10-08)

1. **What was written.** Docs only. No file under `crates/`, `src/`, `tests/`, `scripts/` or `spec/vectors/`
   changed, so `cargo` was not run.
   - `docs/CONFORMANCE.md`:
     - the banner;
     - §1: `schemars` answered, both `python_schemas.rs` pins, and `club_catalogue.json` read by
       `include_str!`;
     - §2: eight families, the oracle and ledger paragraphs, `career-v1.json` and the third key in the
       declarations paragraph, and Format at 13 with three new rows (`general_precision`, `lower`,
       `timestamp`);
     - §2: **new Storage and Career sections**, with each sub-family, both vector shapes,
       `worked_against`, the engine versions as input, the adopted corpus, `CAREER_VERSION`, record →
       port → change → re-record, the runners, the two bit-pin warnings, what reaches no vector, and
       P16's parity run;
     - §3: **a third edge list, "The many-shot layer's edges"**, with `**` as C `pow` and
       `black_box`, the compensated `sum()` against `_within_session_sd`'s `+=`, `:.3g`, `lower`,
       `isalnum`, `slugify`'s combining class, `Timestamp`'s two spellings, `statistics.median`,
       `Path.suffix`, and ASCII swing numbers;
     - §4: the intro, the commands block (the career dry run, `--storage-once` and `--career-once`),
       the third key in `rerecord`, the walk order, the career runners and `CARRIED`, the
       adopted-corpus, respelling and dotted-key rules, `removed` refused on career, the guards, the
       file writing, M36's run as the third worked example (both `career-v1` runs, P16 needing no
       change), `check`'s two deferrals, and `regenerate`'s two refusals;
     - §5: tier 1's career half, the storage row (ADR-008's exception has no Rust twin), tier 2's
       catalogue and shot store (with `mock`/`composite`/`source` left to §M29), and ten crates;
     - "What this does not cover": the Windows-only bit pins and the POSIX case answer.
   - **ADR addenda, three**:
     - ADR-024 (the fifth): `OUTDATED` means "not comparable", `COMPARABLE_FROM` and why 14, P14's
       table as the evidence, and frozen Python disagreeing on purpose until M40. It also says that
       ADR-035 clause 6 ("M35 then changes it") is incomplete rather than false, and that the
       explicit target already existed.
     - ADR-026 (the first): the catalogue is a two-language file, with the reversed pin, `BagEntry`'s
       flatten, and the two Rust-only refusals.
     - ADR-032 (the sixteenth): ten crates and the cargo edges, which correct §1's "contracts
       depends on nothing of ours"; the two families and the third key; and what the port
       measured.
     - ADR-032's Status block now counts **sixteen** and describes the sixteenth.
     - `docs/README.md`: 99 → 102 addenda, the ADR-024 (**5**), ADR-026 (**1**) and ADR-032 (**16**)
       rows, and a "Since M36" sentence in the CONFORMANCE row.
   - `docs/ARCHITECTURE.md`:
     - §1: the five verbs with their flags, the data directories, `--json`, the exit codes and the
       one rule that differs; `--storage-once | --career-once`; the career re-record line; and the
       Rust comment block, now ten crates with `crates/storage`, and `cargo test`'s families;
     - §2: a paragraph of cargo edges (`storage → {contracts, pyfmt}`, `analysis ↛ storage`,
       `core → storage` normal, `contracts → pyfmt`), the dotted edge with no Rust twin, and the
       stale "wired to nothing until M24" fixed to M38 and §M29.
   - `CLAUDE.md`:
     - the opening paragraph names M36;
     - the commands block: `cargo test`'s families, the `career-v<N>.json` declaration, and
       `career-corpus --name Aaron` as the verb example;
     - ten crates, with a `crates/storage` paragraph;
     - the version invariant gains `CAREER_VERSION`;
     - the two routing rows: the families in "How is a port checked", and the M36 modules of
       `contracts`, `pyfmt`, `analysis`, `core` and `storage` in "Where is the Rust".
   - The program plan: a pointer to this plan at the head of §M36. Planning-read finding 1's
     correction is made in the explicit-target bullet and the Vectors line. The `OUTDATED` paragraph
     gains "Decided", the Schemas bullet gains call 8's answer, and "What the code says" finding 9
     gains a dated note. **Its status table row is untouched**, because that is P18's.
2. **The Files line grew**, each addition for the goal's "every doc that describes what M36 changed":
   - `spec/README.md`: its tree, runner list, version list, oracle paragraph, declaration list and
     refusals all named families, and it now includes storage and career;
   - `docs/ARCHITECTURE.md` §4: the `stale`/`outdated` paragraph gains one paragraph on the Rust
     corpus's `COMPARABLE_FROM`, because it states the rule as if there were one language.
3. **ADR-028 and ADR-035 were read, and neither gained an addendum.** No clause in either is now
   false.
   - ADR-028 §2–§5 describe the Python and are true of the port too: the mishit median is taken
     over the pooled swings, which since P14 means the comparable ones, and `mishit-sees-only-comparable`
     pins it. §5's "never been versioned" is about artifacts on disk, and `CAREER_VERSION` versions
     vectors.
   - ADR-035 clause 4's two M36 promises held. No `contracts` or `storage` shape sets
     `deny_unknown_fields` (grepped), and the catalogue pin exists. Clause 6's "M35 then changes it"
     is incomplete now, since M36 changed one rule first. ADR-024's addendum says so, so ADR-035 needs
     no addendum.
4. **Noticed while writing, and not measured: the engine's own `**` sites.** The frozen engine squares
   and roots with `**` in five places:
   - `measure.py:483` and `:776`, `(dx ** 2 + dy ** 2) ** 0.5`;
   - `phases.py:803`;
   - `benchmarks/trajectory.py:123`;
   - `benchmarks/flight_model.py:98`.

   `crates/analysis` writes them as `powi(2)` and `.sqrt()` (`measure.rs:453` and `:731`,
   `phases.rs:302`, `benchmarks/trajectory.rs:141`, `benchmarks/flight_model.rs:354`). P11 and P12
   measured that neither is CPython's `**` on every input. Every engine and stage vector passes, the
   frame indices exactly. Nobody has measured whether an engine float differs in its last bit, or
   whether a `phases` speed sits where a difference would move a frame. ADR-032's sixteenth addendum
   and CONFORMANCE §3's third list both record it as **routed, beside M34's `sum()` question, to
   the user or to §M29**, not counted as §3's seventh edge. **For P18**: carry it into the WORKLOG
   entry's open items.
5. **Left as found, and not M36's**:
   - `docs/README.md`'s CONFORMANCE row says "**five** Rust-specific edges" (CONFORMANCE §3's own
     list says five, and `CLAUDE.md` says six);
   - ADR-026's Status still says "ahead of its code", which M12 overtook;
   - ARCHITECTURE's banner date and §2's interface table ("Career corpus … not yet consumed").
6. **For P18**:
   - `docs/README.md`'s document count is still untouched (the rule), and moves when this plan is
     staged;
   - ROADMAP's status row and §M36, M35's row, and the program plan's status row are P18's. P17
     edited only §M36's body and finding 9 there;
   - no new doc was added, so the doc count moves by this plan alone.
7. **Verify**:
   - `pytest tests/test_docs_truth.py`: 92 passed;
   - `pytest`: 2051 passed;
   - every backticked path and markdown link in the added lines resolves, checked by a scratch
     script whose only misses were paths relative to `spec/vectors/` (checked by hand) and
     `CLAUDE.md` text the edits re-wrapped;
   - `git diff -- src/` is empty, and no file under `crates/`, `tests/`, `scripts/` or `spec/vectors/`
     changed after the marker;
   - the `data/` digest is unchanged (`bd384cc4…`);
   - no edited file has mixed line endings.

### P18 (2026-10-08)

1. **The exit, item by item.** Every item in "Exit" holds:
   1. both families were recorded once (P2, P3), and `regenerate` refuses both (the
      `tests/test_conformance.py` pins, inside `pytest`);
   2. `crates/storage` exists, and `cargo test` passes every storage and career vector, report text
      included: 1,042 passed, 0 failed, 1 ignored (the existing `clip::Cutter` doc-test);
   3. `rerecord --declare spec/declarations/career-v1.json --dry-run` reports "nothing differs from
      the committed vectors", 120 run (64 storage, 56 career), 0 changed, 0 files written;
   4. P16's parity run is the record for the verbs, and no verb or script changed after it;
   5. a Rust `read_corpus` over `data/` pools what frozen Python pools (ADR-024's addendum, P14's
      table; nothing in `crates/storage` changed after P16);
   6. `git diff -- spec/vectors/{synthetic,corpus,stages,screen}` is empty, and the format family's
      only change is three new files (`general_precision`, `lower`, `timestamp`) with no existing
      table modified;
   7. `git diff -- src/` is empty, and the `data/` digest is P1's,
      `bd384cc40b236b73e3013b817ae79908d6e5ff4dded25b8803fc35c6b7af01cc`, 192 files;
   8. `cargo clippy --all-targets` has 0 warnings, `cargo fmt --check` is clean, `pytest` 2,051
      passed, `ruff` is clean, `mypy` is clean over 118 files, and `conformance.py check` exits 0,
      deferring storage (64) and career (56) to `cargo test`.
2. **The doc-count pin failed once, as the rules predicted.** `docs/README.md` moved to 81 documents
   (68 in `docs/`, 7 in `plans/`) before this plan was staged, so the first `pytest` run failed
   `test_the_documentation_map_counts_the_documents_correctly` alone (80 tracked against 81 claimed).
   Staging the change made it green: 2,051 passed.
3. **One edit beyond the Steps line: M29's rows.** M29 depends on M34 and M36 (the program plan's
   order paragraph), so closing M36 unblocks it as well as M35. ROADMAP's status row and §M29's
   Status line, and the program plan's M29 row, now say so. ROADMAP's NEXT ACTION paragraph moved
   from M36 to M35, and §M36's Open paragraph became "Decided", with the plan's exit as its Exit.
4. **What the commit holds, and what it leaves.** The commit holds every M36 file in the working
   tree, this plan included. **It leaves out `.claude/`**: the edited `/orchestrate`, `/next-phase`
   and `/plan-phases` skills and the untracked `phase-runner` agent. Those are the user's harness
   changes, not a phase's work, and skills were last committed on their own (`0a992ee`).
   `CLAUDE.md`'s one line naming `phase-runner` rides in this commit, because P17 edited that file.
5. **The memory note** `app-transition-decisions` (and its index line) says M36 is done, M35 is next
   with `career-v2`, and M29 is unblocked.
