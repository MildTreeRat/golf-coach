# Plan: M29 — The lab port: a Rust lab CLI, the `rmcp` server, and the archive move

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-10-15. **Milestone**:
[ROADMAP §M29](../../ROADMAP.md#m29-the-lab-port--a-rust-lab-cli-the-rmcp-server-and-the-archive-move).
**The design is the program plan's
[§M29](m31-m40-shot-first-pivot.md#m29--the-lab-port-a-rust-lab-cli-the-rmcp-server-and-the-archive-move)**
and [ADR-035](../decisions/035-rust-everywhere-python-where-required.md) clauses 1–5, with the
M31.5 inventory's rows R1–R44 ([m31-5-rust-first-replan.md](m31-5-rust-first-replan.md)) saying
what each Python file becomes. This file is its phase list. Where this file and §M29 disagree, this
file's interview decisions, findings and calls say why, and **this file wins**. ADR-035's Deferred
list leaves four questions to M29; all four are answered below (D10, D11, D12, and D7 for the OCR
gate).

**Governing decisions**: ADR-035 clauses 1 (what survives), 2 (everything else ports), 3 (Rust
records; a port of frozen behaviour is recorded from Python once), 4 (the frozen lab reads what Rust
writes, until M40), 5 (M29's scope and the two retirement moments); [ADR-014](../decisions/014-screen-capture-shot-ingestion.md)
(the screen reader); [ADR-006](../decisions/006-mcp-server.md) (the MCP server);
[ADR-020](../decisions/020-conversational-followups.md) (the follow-up route, Option B);
[ADR-016](../decisions/016-local-first-host-and-phone-upload-topology.md) (the upload path);
[ADR-033](../decisions/033-the-pose-sidecar-protocol.md) (the pose boundary);
[ADR-022](../decisions/022-learned-artifacts-as-committed-data.md) (fitting archived);
[ADR-032](../decisions/032-the-rust-core.md) and `docs/CONFORMANCE.md` (vector families and the
comparator); [ADR-010](../decisions/010-benchmark-ranges.md) §2 (no score beats a wrong one);
ADR-008 (as cargo edges).

**Level: L3.** Three new crates, two new vector families, a new dependency on ONNX Runtime and on
the MCP SDK, the first Python edits since M32 outside conformance tooling (the LLM side, and one
frozen-server closure), a rewrite of `data/`, an archive move and a delete. Every phase reads its
own list below rather than the repo.

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot — the findings are what the next phase
   is planned against — and address anything a later phase must know to that phase by number
   ("**For P25**: …");
3. stop, and tell the user to `/clear` and run `/next-phase`.

**Nothing is committed until P36** (D1). Every phase leaves its work in the working tree, and P36
makes one commit onto `main`. `data/` is gitignored, so git protects none of it: D8's backup does.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document and the pointers to it | ✅ Done *(2026-10-16)* — approved by the user |
| **P1** | Part A — The ONNX models, their manifest, and `crates/ocr`'s skeleton | ✅ Done *(2026-10-16)* |
| **P2** | Part A — Photo decode, and the screen-quad finder | ✅ Done *(2026-10-16)* — resumed after the JPEG question |
| **P3** | Part A — Rectification: warp, CLAHE, resize, rotate | ✅ Done *(2026-10-16)* — resumed after a cut-off |
| **P4** | Part A — Text detection: the det model and DB postprocessing | ✅ Done *(2026-10-17)* — resumed after a cut-off |
| **P5** | Part A — Recognition: crop, the rec model, CTC decoding | ✅ Done *(2026-10-18)* — resumed after a cut-off |
| **P6** | Part A — Textline orientation, and the `recognize` seam | ✅ Done *(2026-10-21)* — resumed after a cut-off |
| **P7** | Part A — The reader end to end, and the OCR gate on the 13 photos | ✅ Done *(2026-10-21)* — resumed after the user's (a): 12 of 13 exact, the thirteenth's one value named |
| **P8** | Part B — Record the `mcp` family from frozen Python | ✅ Done *(2026-10-21)* — 22 cases, 205 calls; 29 of them carry flight numbers (C1) |
| **P9** | Part B — `crates/mcp`'s skeleton: the stdio server, the caveats, the descriptions | ✅ Done *(2026-10-21)* — surface conforms with three differences declared; 205 calls skipped by name |
| **P10** | Part B — The swing and session tools | ✅ Done *(2026-10-21)* — 74 calls conform exactly, text blocks byte for byte; 131 still skipped by name |
| **P11** | Part B — The shot tools | ✅ Done *(2026-10-21)* — 29 calls conform, the filter declared at three paths; M32's fields reached by one hand case |
| **P12** | Part B — The career tools | ✅ Done *(2026-10-21)* — 29 calls conform, one note declared; Rust's corpus reached by one hand case, and two sentences name the drill |
| **P13** | Part B — The club tools | ✅ Done *(2026-10-21)* — 35 calls conform exactly, nothing declared; a drill reads as deleted, as ADR-024 says |
| **P14** | Part B — `simulate_flight` | ✅ Done *(2026-10-21)* — 38 calls conform exactly; the caveats and the printed comparison ported into `crates/analysis`; one reading declared (a judgement) |
| **P15** | Part B — `get_strike_profile`, the stdio handshake, the re-record walk, the tool-count pin | ✅ Done *(2026-10-21)* — 224 calls compared, none skipped; the career walk runs `golf-mcp answer-case` and moved 21 recorded cases 0 → 3 once; the pin reads twelve from Rust |
| **P16** | Part C — `conversation.py` over `golf-mcp`; `ask_swing.py` switches | ✅ Done *(2026-10-22)* — `ask_through_golf_mcp` beside the in-process `ask`; one live question answered through `get_strike_profile` |
| **P17** | Part C — `coach.py`'s JSON entry over `golf-mcp` | ✅ Done *(2026-10-22)* — resumed after the user's (a): the entry prints `{text, provenance}` over `golf-mcp`, `get_swing` gains two declared keys, and `coach.py` is a sanctioned unwrap site with its ADR-019 addendum |
| **P18** | Part D — Record the `selection` family from frozen Python | ✅ Done *(2026-10-22)* — 45 gzipped cases (rules, windows, the 15 stored bundles), 0.78 MB; every stored window reproduced; P19 must ask before the v17 walk moves them 16 → 17 |
| **P19** | Part D — Port the swing selection to the family | ✅ Done *(2026-10-22)* — 95 calls answered exactly, 28 `_auto_windows` skipped by name until P25; the `v17` walk moved the 45 cases 16 → 17 once; an engine run walks `mcp` too (C1) |
| **P20** | Part D — `crates/lab`'s skeleton; the audio contract and its store | ✅ Done *(2026-10-22)* — `golf-lab` exists with no verb; `contracts::audio` and `storage::audio_io` round-trip all 30 stored files, and frozen Python reads what Rust writes |
| **P21** | Part D — Strike audio: ffmpeg, the video-start probe, `audio_for` | ✅ Done *(2026-10-22)* — resumed after the user's (a): 30 of 30 views pass under `PATH`'s 9.0.2, strikes exact and three floats to 2e-3; the decoder's other branches pinned on made clips |
| **P22** | Part D — Pose: `keypoints_for` over `crates/pose` | ✅ Done *(2026-10-22)* — 30 of 30 views hit the cache and nothing is spawned; one forced run on the smallest clip reproduces its 344 frames exactly; `Outcome::Done` carries the acceptance's count |
| **P23** | Part D — The shot lookup and the re-read, dry run | ✅ Done *(2026-10-23)* — resumed after a cut-off: all 13 stored shots found unstamped and re-read in a dry run, identity kept; only the three declared values move, and every provenance change is a listed kind |
| **P24** | Part D — `golf-lab import-photos` | ✅ Done *(2026-10-23)* — the script's import, its table and exit codes, and `--into-session` photo entries analysed through `run_shot_session` that Rust's corpus pools; the release binary reads alone from outside `target/` |
| **P25** | Part D — The bundle pipeline, and the input gate on the 15 bundles (dry run) | ✅ Done *(2026-10-23)* — all 15 inputs equal to the committed corpus but the re-read shot's declared paths; `_auto_windows` ported into `golf_core::auto_windows`, so no `selection` call is skipped and the `v17` dry run still changes 0 |
| **P26** | Part D — The writers, coaching, and `golf-lab analyze` | ✅ Done *(2026-10-25)* — resumed after two cut-offs: three scratch bundles written and read back by frozen Python; the report is the script's except `not measured:`, where the script's `join` crashes; one live paragraph folded in |
| **P27** | Part D — `golf-lab reanalyze` and `golf-lab analyze-keypoints` | ✅ Done *(2026-10-25)* — ten dry runs over `data/` are the script's at engine 17, all 15 listed; photo entries re-run through `run-shot`'s code; the keypoints report is the script's past its crash |
| **P28** | Part D — `golf-lab flight` | ✅ Done *(2026-10-28)* — resumed after a cut-off: 51 of 54 argument sets over `data/` byte-identical to the script, the other 3 where it raises; the corpus join, the gate and the what-if ported; the stored modes pinned on a fixture tree |
| **P29** | Part D — The `data/` write: backup, re-read the 13, re-run the 15 | ✅ Done *(2026-10-28)* — resumed after the user's (b) and (a): backed up, 13 shots stamped, 15 bundles re-run at 17; only the declared values and last-bit scores move, every reader answers; digest `07f37373…` |
| **P30** | Part E — `golf-lab build-vectors`, the builder that reads `data/` | ✅ Done *(2026-10-28)* — 31 vectors built and verified from `data/`; the 15 corpus and stage vectors are the committed ones but the re-read shot's declared paths; the real storage corpus differs in five explained kinds, one of them a `shot_unscored` the committed case cannot reach |
| **P31** | Part E — `golf-lab import-export`, the phone export's import | ✅ Done *(2026-10-28)* — resumed after a cut-off that had written nothing: a zip or directory merged file by file, one conflict refusing all, shots and photo entries deduped by photo; the `zip` crate pinned at 7.3.0; the imported corpus equals the phone's |
| **P32** | Part F — Close the frozen server's write routes | ✅ Done *(2026-10-28)* — resumed after the user's (a): the eight routes answer 410 naming their verbs, `golf-core` gains `set-golfer`, `set-club` and `delete-swing`, and `golf-mcp`'s untagged-bag note names `set-club`, declared; every GET route over `data/` answers as before |
| **P33** | Part F — The archive move, with the doc paths rewritten | ✅ Done *(2026-10-28)* — the research record in `archive/`, `reference.py` beside `golfdb/`, the `research` extra gone; 182 paths rewritten in 68 files, and the residue named (recorded provenance, three schemas, the move's own sentences) |
| **P34** | Part F — Delete the lab scripts, `detection/`, `frontend/`; the extras; re-register MCP | ✅ Done *(2026-10-29)* — resumed after a cut-off that had written nothing: fourteen scripts and `detection/` deleted (staged), `frontend/README.md` unstaged for the count; four career pins went with the scripts and the branch pin reads the committed vectors; four Rust sentences name the verbs, two declared; `golf-coach` ✔ Connected through the release `golf-mcp` |
| **P35** | Part F — Docs: CONFORMANCE, ARCHITECTURE, `CLAUDE.md`, the ADR addenda, the program plan | ✅ Done *(2026-10-29)* — CONFORMANCE gains the `selection` and `mcp` families and the OCR gate; ARCHITECTURE §1, §2, §4 and §5 describe the Rust lab; `CLAUDE.md` thirteen crates; nine ADR addenda (035 answers its four questions), the map at 118 addenda; §M29 and §M38 P4 |
| **P36** | Part F — Close: verify, WORKLOG, ROADMAP, memory, and the one commit | ✅ Done *(2026-10-29)* — every verify command green, the ignored gates included (296 of 296); both dry runs 0 changed; digest still `07f37373…`; the map at 84 documents; one commit onto `main` |

---

## Rules every phase keeps

- **Every phase ends green on everything it can reach.** If a phase cannot end green, stop and
  record why as a finding; do not carry a red suite into the next session. A vector a phase cannot
  pass yet is skipped *by name* in its gate, with the phase that will pass it, never by a blanket
  ignore.
- **Frozen Python changes only where this plan names it** (ADR-035 clause 4):
  - `scripts/conformance_vectors.py`, `scripts/conformance.py` and `tests/test_conformance.py`, for
    the two recorder families (P8 `mcp`, P18 `selection`) and `check`'s deferral of them — clause
    4's one sanctioned change;
  - the LLM side, inside clause 1's exception: `feedback/conversation.py`, `feedback/coach.py`,
    `scripts/ask_swing.py` and `tests/feedback/` (P15–P17);
  - `api/app.py` and `tests/api/` for the write-route closure (P32, D11) — the one "fixed because
    it breaks" edit;
  - the moves and deletes of P33–P34, `pyproject.toml`'s extras with them, and
    `tests/test_docs_truth.py` where P15 moves the tool-count pin.
  Any other `git diff -- src/ scripts/ tests/` is a finding and a stop.
- **`data/processed/` is written by P29 and nothing else.** P23 and P25 are dry runs; P21's forced
  detection, P22's forced pose, P24's import and P26's end-to-end run all work on a **scratch copy**
  under the session scratchpad. P1 writes `data/models/ocr/` (model files, gitignored) and nothing
  else under `data/`. P1 takes the `data/processed` digest (method below; expected still M37's
  `bd384cc4…` over 192 files) and every phase before P29 that runs anything over `data/` compares
  against it; P29 records the new digest and every later phase compares against **that**.
- **Only these write `spec/vectors/`**: P8 (`spec/vectors/mcp/`, by the frozen recorder), P9–P15
  (hand cases and Rust-recorded `get_strike_profile` cases under `spec/vectors/mcp/`, and declared
  paths in the recorded ones), P18 (`spec/vectors/selection/`, by the frozen recorder). P30's
  builder writes to scratch in its gate and commits nothing. **The engine, stage, shot, screen,
  audio, format, storage, career and profile families do not change**:
  `git diff -- spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format,storage,career,profile}`
  is empty at every phase. `ANALYSIS_VERSION`, `SCREEN_PARSER_VERSION` and `CAREER_VERSION` do not
  move, and no `spec/declarations/` file is added.
- **New code is Rust**, except P16–P17's LLM side, which is clause 1's exception.
- **ADR-008 as cargo edges.** `crates/ocr` depends on `screen` and `contracts` and never on
  `analysis`, `feedback` or `core`. `crates/mcp` and `crates/lab` are shells like `core`, and
  nothing depends on either. **`ort` appears only in `crates/ocr`'s closure** and in what depends
  on it (`crates/lab`): never in `contracts`, `pyfmt`, `analysis`, `feedback`, `screen`, `storage`
  or `core`, which keeps it off M38's `crates/app`. P1 adds the pin that says so.
- **No new dependency without a finding first**, except the ones decided here: `ort` (P1, D18),
  `image` and `imageproc` (P2, D4), `rmcp` with `tokio` and whatever schema crate `rmcp` requires
  (P9). Each is pinned to an exact version and named in its phase's finding.
- **External tools are found, never assumed.** `ffmpeg` is `GOLF_FFMPEG` first, then `PATH` (D17).
  A missing tool is a named refusal saying what was tried, never a panic.
- **Model files are verified, never trusted.** `crates/ocr` refuses a missing ONNX file or a sha256
  that disagrees with `crates/ocr/models.json`, by name (D6).
- **Tests that need `data/` or the models** are `#[ignore = "<what they need>"]` with the reason,
  and the phases that own them run `cargo test -p <crate> -- --include-ignored`. A test that needs
  them must never pass vacuously when they are absent.
- **House style** (`CLAUDE.md` §House style, `docs/CODE_STANDARDS.md`): comments say *why* and carry
  the measurement or the rejected alternative. A module ported from Python names its twin in its
  first lines (as `crates/storage`'s do), and a module with none says so.
- **Do not write counts into prose** that a test or a registry owns. Where this plan quotes a count,
  it is a snapshot for the phase to re-measure.
- **The doc-count pin reads `git ls-files`**, so this file is invisible to it until P36 stages it.
  Do not touch `docs/README.md`'s document count before P36. This file avoids the
  `**Status: … N/M phases` spelling on purpose (`tests/test_docs_truth.py::_PHASE_STATUS`).
- **Line endings**: the plain `.json` vectors Python wrote are CRLF, and `rerecord`'s writer keeps
  each file's own. Git Bash heredocs mangle `—`, `±` and `°`; use the Edit tool or Windows Python for
  in-place edits.
- **Scratch work** (reference dumps from frozen Python, copies of bundles) lives in the session
  scratchpad, never in the repo, and a throwaway script is deleted when the phase ends. Running
  frozen Python to *measure* it is using the lab, not changing it (ADR-035 clause 4).
- **Live calls cost money and need the key.** P16, P17 and P26 each make one real Claude call;
  `GOLF_ANTHROPIC_API_KEY` is in `.env` (checked 2026-10-15). If it is missing when a phase runs,
  stop and ask.

## Verify — the commands, and when each runs

```bash
cargo test                                          # every phase that touches crates/ or spec/ — unpiped, or check PIPESTATUS
cargo test -p <crate> -- --include-ignored          # the phase that owns a data/- or model-dependent gate, and P36 for all of them
cargo clippy --all-targets && cargo fmt --check     # every phase that touches crates/
.venv/Scripts/python.exe -m pytest                  # P8, P15–P18, P32–P36 — alone, never beside cargo test (a worker-timeout test flakes under load)
.venv/Scripts/python.exe -m ruff check src tests scripts   # any phase touching Python; P36
.venv/Scripts/python.exe -m mypy src                # P16, P17, P32–P34, P36
.venv/Scripts/python.exe scripts/conformance.py check      # P8, P18, P36
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py # every phase that edits a doc or moves a path
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/v17.json --dry-run        # P19 (it walks selection from then on), P36: 0 changed
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/career-v3.json --dry-run  # P15 (it walks mcp from then on), P36: 0 changed
find data/processed/sessions data/processed/golfers data/processed/shots -type f | LC_ALL=C sort | xargs -d '\n' sha256sum | sha256sum   # P1, every phase reading data/, P29 (new value), P36
```

---

## Decisions taken in the interview (2026-10-15)

- **D1 — One plan, one commit at the end.** Six parts (A the reader, B the `rmcp` server, C the
  LLM's route, D the lab CLI, E the builder and the export, F retire and close), and P36 makes the
  one commit, as M35 and M37 did.
- **D2 — Order: A → B → C → D → E → F.** The reader first, because the lab's shot re-read needs it;
  the MCP server next, because it is independent and carries `get_strike_profile`; the LLM's route
  before the lab CLI, because `golf-lab analyze` spawns `coach.py`'s JSON entry, which reads through
  `golf-mcp`; deletions last, once every Rust route exists.
- **D3 — HEIC is decoded by ffmpeg.** The user first chose ImageMagick; once ffmpeg 9.0.2 was
  installed (D17) and shown to decode the HEIC photo at full size, the user moved HEIC to ffmpeg, so
  there is one external tool, not two. JPEG and PNG decode in-process with the `image` crate.
- **D4 — The rectifier is a hand port on `image` + `imageproc`.** No native OpenCV. CLAHE and
  OpenCV's area resampling are written by hand. The OCR gate decides whether it is close enough.
- **D5 — The `ort` reader runs three of PaddleOCR's five models**: text detection, textline
  orientation and recognition (PP-OCRv6 medium det/rec, PP-LCNet textline orientation). It skips
  document orientation (the rotation vote does that job) and UVDoc unwarping (the rectifier does).
  If the OCR gate fails on geometry, a phase is inserted to add UVDoc, with a finding and the
  user's approval.
- **D6 — The ONNX files are converted once, kept gitignored, and sha-pinned.** PaddleX's
  `paddle2onnx` plugin converts the three models into `data/models/ocr/` (already gitignored by
  `data/models/*.onnx`); `crates/ocr/models.json` is committed and pins each file's sha256, its
  source model and its preprocessing constants. No new Python file enters the repo for it.
- **D7 — The OCR gate.** On each of the 13 stored bay photos, the shot parsed from the Rust
  reader's boxes is compared with the shot parsed from PaddleOCR's committed boxes
  (`spec/vectors/screen/corpus/`): **every value, `fields_present` and `needs_review` identical**.
  Per-box confidences, box geometry and warning text may differ. A photo that fails is a finding and
  a stop, never a loosened gate. This answers ADR-035's first deferred question.
- **D8 — `data/` is backed up first, then rewritten in place.** Before the first write (P29),
  `data/processed/` minus the clips (`.MOV`, `.mp4`) is copied to a dated folder outside the repo
  and its digest recorded.
- **D9 — The `rmcp` server is held to an `mcp` family recorded once from frozen Python** (clause 3's
  one-time recording), over the `tests/mcp` fixture trees and a `data/` snapshot. Every deliberate
  difference is a declared path or a hand case (`oracle: "hand"`); `get_strike_profile` has no
  Python behind it, so its cases are hand-worked or recorded once by Rust (`oracle: "rust"`).
- **D10 — The caveat prose reaches Python through the `rmcp` server.** Its `instructions` carry
  `READING_THIS_DATA_HONESTLY` as today, and a read-only MCP resource serves the other named
  sentences; `coach.py` and `conversation.py`'s new route read them at session start and keep no
  copy. No fourth surviving shape. This answers ADR-035's third deferred question.
- **D11 — The frozen server's write routes close at M29.** Every route writing under `sessions/` or
  `shots/` refuses with a message naming the Rust verb: `POST /api/uploads`,
  `/api/sessions/current/golfer`, `/api/sessions/current/club`,
  `/api/sessions/{s}/swings/{w}/{golfer,club,mishit,analyze}`, and
  `DELETE /api/sessions/{s}/swings/{w}`. Read routes, the bag routes, `/api/clubs/lookup` (the
  LLM's) and `/ask` stay open. This answers ADR-035's second deferred question.
- **D12 — No `sidecar/` package in M29.** M29 deletes only what the frozen server does not import,
  so repackaging now would move files the frozen server imports. Deferred to M40, with that reason.
  This answers ADR-035's fourth deferred question.
- **D13 — Three new crates.** `crates/ocr` (image decode, the rectifier, the `ort` det/cls/rec
  models, the `TextBox` seam; the only crate with `ort`); `crates/lab` (binary `golf-lab`: the
  bundle pipeline, audio, pose, the shot lookup, and every lab verb); `crates/mcp` (binary
  `golf-mcp`: the `rmcp` server). `golf-core` stays the engine and recorder shell that M38's
  `crates/app` can depend on.
- **D14 — The swing selection is gated by a `selection` family recorded once from frozen Python**:
  `phases.py`'s `select_swing` and `select_matching_swing`, and `api/pipeline.py`'s `_auto_windows`,
  over the 15 stored bundles (inputs cut to what selection reads) and the tests' synthetic tracks.
  It ages on `ANALYSIS_VERSION`.
- **D15 — Coaching: `golf-lab` spawns `coach.py`'s JSON entry, which reads `get_swing` over
  `golf-mcp`** (ADR-035's Q14 as written). It prints `{text, provenance}` or `{note}`, and Rust
  folds that into `analysis.json`; Rust stays the only writer of the artifact and Python the only
  caller of the LLM. A brief line `SwingView` cannot feed is a finding: extend the view (an added
  key, which clause 4 allows) or drop the line.
- **D16 — The phone export is the stored layout itself.** An export is a zip or directory of the
  phone's `data/processed` subtree (sessions, shots, golfers) as `crates/storage` writes it.
  `golf-lab import-export` merges it into `data/`. No new contract; M38 writes what storage already
  writes, and M38 P4's exit compares against Rust's `read_corpus`.
- **D17 — ffmpeg is on `PATH`.** The user had the planning session install it: ffmpeg **9.0.2**
  (`winget install Gyan.FFmpeg`, 2026-10-15, alias in
  `%LOCALAPPDATA%\Microsoft\WinGet\Links\ffmpeg.exe`). `golf-lab` looks at `GOLF_FFMPEG`, then
  `PATH`, and does not fall back to `.venv`'s bundled 7.1, which goes with the `audio` extra in M40.
- **D18 — ONNX Runtime comes from `ort`'s `download-binaries`**, with the `ort` version pinned
  exactly. The first build needs the network; Cargo.lock records it, and an ADR-035 addendum names it
  (P35).
- **D19 — The builder builds new vectors only, and reproduces the 15.** `golf-lab build-vectors`
  ports `_corpus` (with its stage vectors) and the storage family's real corpus over
  `crates/storage`'s readers. It writes only vectors that do not exist and refuses to overwrite a
  committed one, naming it (`rerecord` stays the only re-writer).
- **D20 — The lab's gate before it writes `data/`.** Per bundle, the engine input `golf-lab`
  assembles (windows, strikes, shot, handedness, loft) equals the committed corpus vector's input,
  except the two label-fix shots' declared values and the stamp every re-read shot gains. Then it
  writes, and the frozen server's readers and results pages load every artifact. Plus one forced
  pose run, into a scratch copy, compared with the stored keypoints.
- **D21 — The MCP re-registration is the phase's own job.** When `scripts/run_mcp_server.py` is
  deleted (P34), the phase runs `claude mcp remove golf-coach` and `claude mcp add` pointing at the
  release `golf-mcp`, and checks it reports Connected.

## What the planning read found (2026-10-15)

Each is a snapshot; the phase that leans on it re-measures.

1. **`data/` holds 15 swing bundles and 13 distinct shot photos.** Every bundle has two clips, both
   views' `keypoints.json` and `audio.json`, an `analysis.json` and a shot photo. Photo
   `2e81ab05828e…` (shot `2026-08-10-1`) is shared by three bundles: `2026-08-07-aaron1/1`,
   `2026-08-09/2`, `2026-08-10/1`.
2. **The two label-fix shots** (M34): `2026-08-10-1`'s `shot_type` goes `CENTER SLIGHT FADE` →
   `SLIGHT FADE`; `2026-08-23-1`'s `impact_position` goes `HEEL` → `null`. `2026-08-23-1` is also
   the **one HEIC photo**. So four bundles carry a label-fix shot. M34's re-record also listed the
   provenance changes every re-read shot shows: the V tile's `no value text under the label`,
   `parse_confidence` about −0.02, `label_ratio`'s new denominator, the tie warnings on the two, and
   their missing `raw_fields` entries (`docs/plans/m34-screen-reader.md`, P10's findings).
3. **Frozen OCR is PaddleOCR 3.7.0 on paddlepaddle 3.3.1 and paddlex 3.7.2**, oneDNN off
   (`launch_monitor/screen/paddle.py`). PaddleX's `OCR.yaml` runs five models by default, all in
   `~/.paddlex/official_models/`: `PP-LCNet_x1_0_doc_ori` (6.6 MB), `UVDoc` (31 MB),
   `PP-OCRv6_medium_det` (60 MB), `PP-LCNet_x1_0_textline_ori` (6.6 MB), `PP-OCRv6_medium_rec`
   (74 MB), each as PIR `inference.json` + `inference.pdiparams` + `inference.yml`. `paddle2onnx` is
   not installed. `.venv` has the `onnxruntime` 1.28.0 wheel, which the `ort` crate does not use.
4. **Frozen preprocessing is OpenCV** (`launch_monitor/screen/preprocess.py`): downscale to 1000 px
   for detection, bilateral filter, two Canny presets × three close sizes × three polygon epsilons,
   area-plausibility and fill checks, a `minAreaRect` fallback, `warpPerspective`, CLAHE (clip 2.0,
   8×8) on LAB lightness, `INTER_AREA` resize to 1600 px, and a rotation vote over 0/180/90/270 that
   stops at a 0.6 label ratio. The vote's ratio is already ported (`crates/screen/src/orient.rs`).
5. **HEIC.** `.venv`'s bundled ffmpeg 7.1 decodes only an 896×640 tile of the HEIC photo; ffmpeg
   9.0.2 decodes it at 4284×5712, the same size Pillow gives. ImageMagick 7.1.2 also decodes it but
   is not needed (D3). Windows has the HEIF extension but not HEVC, so WIC was not pursued.
6. **The selection half of `phases.py` was never ported**: `select_swing` (line 610),
   `select_matching_swing` (714) and their helpers, and `api/pipeline.py`'s `_auto_windows` (822),
   `_pick_swing` (723) and `_narrate_choice` (774). The corpus vectors take their windows from the
   stored `analysis.json` (`conformance_vectors.py::_corpus_vector`), so **nothing committed gates
   selection** — hence D14. The stored keypoints total 229 MB across 30 whole clips, so the family's
   input must be cut to what selection reads.
7. **The MCP surface is 11 tools in `contracts/tool_descriptions.py::TOOL_DESCRIPTIONS`**, plus
   M37's `get_strike_profile`, so `golf-mcp` serves 12. Tools return pydantic views (`SessionSummary`,
   `SwingView`, `ShotView`, `FlightView`, `NotFound`, …) serialized by the `mcp` 2.0.0 SDK.
   `anthropic` 0.121.0 is installed. `mcp/server.py::instructions(career_tools=…)` is the
   `READING_THIS_DATA_HONESTLY` text. `coach.py` imports `TWO_AXES`, `READING_THIS_DATA_HONESTLY`
   and `ALIGNMENT_CAVEAT`; `conversation.py` imports `ONLY_CHECKPOINTS_ARE_JUDGED`.
8. **`coach.py::build_brief` reads more than `SwingView` may carry**: checkpoint scores, placements,
   pivots, measurements, alignment quality and the shot. P17 measures the gap (D15).
9. **The frozen app has twelve write routes** (`api/app.py`): uploads, clubs lookup, current-session
   golfer and club, bag add and delete, swing delete, swing golfer, club, mishit, analyze, and ask.
   D11 closes seven.
10. **Claude Code's registration** of the `golf-coach` server is in `~/.claude.json`'s project
    entry, pointing at `.venv/Scripts/python.exe` and `scripts/run_mcp_server.py`.
11. **`.env` has `GOLF_ANTHROPIC_API_KEY`; `fastapi` 0.141.1 is installed**, so the frozen server
    can be exercised with `TestClient`.
12. **Doc paths naming the archive's scripts**: 22 Markdown files outside `WORKLOG.md` and
    `docs/plans/` (`grep -rln` for the five paths and `contracts/reference`). P33 re-counts.
13. **Tests that load scripts or the modules M29 moves**, to re-check in P33–P34:
    `tests/analysis/{test_joint,test_measure,test_phases,test_select_swing,test_trajectory}.py`,
    `tests/api/{test_bag_route,test_conversation_routes,test_pipeline_imports,test_uploads}.py`,
    `tests/contracts/test_unscored.py`, `tests/mcp/conftest.py`,
    `tests/storage/{test_flight_inputs,test_transcript_store}.py`, `tests/test_config.py`,
    `tests/test_conformance.py`, `tests/test_docs_truth.py`.

## Calls this plan makes where §M29 and the interview left it open

- **C1 — Families and their version keys.** `selection` ages on `ANALYSIS_VERSION`, and P19 makes
  the engine re-record walk it after the shot family. `mcp` ages on `CAREER_VERSION`, and P15 makes
  the career re-record walk it after the profile family. P8 records which `mcp` cases carry the
  flight model's numbers; if a later `v<N>` declaration moves them, that declaration names them, and
  P8 says whether the engine re-record must walk `mcp` too.
- **C2 — The OCR, input and frozen-server gates run on this box only**, as ignored tests or
  verbs, because the photos and clips are in gitignored `data/`. What they compare against is
  committed (`spec/vectors/screen/corpus/`, `spec/vectors/corpus/`).
- **C3 — No warm-interpreter churn measurement.** The lab uses `crates/pose`'s `PoolConfig`
  defaults; ADR-033's fifth-addendum measurements stay M40's.
- **C4 — No render.** `golf-lab` never writes `aligned.mp4` (ADR-035 clause 2, Q10). An existing
  one is left standing, and `analysis.state.json` records the run as one that did not attempt a
  render, as `record_state` does for `--no-video`.
- **C5 — Coaching defaults mirror Python's**: on for `golf-lab analyze` when a key is configured and
  `GOLF_COACHING_ENABLED` allows it, off for `golf-lab reanalyze` unless `--coaching`. So P29's
  re-run drops the stored coaching paragraphs, as a Python `reanalyze.py` would; the backup keeps
  them.
- **C6 — `analysis.json` carries only reasons frozen Python's `UnscoredReason` knows** (clause 4).
  The photo-side reasons stay in `shot_analysis.json`. P26 pins it.
- **C7 — Photo-only entries re-analyse through `golf-core`'s `run_shot`** and write
  `shot_analysis.json`, as M35's `run-shot --session` does; `golf-lab reanalyze` covers them
  (the program plan's "re-analysis over photo-only directories").
- **C8 — Report text parity, as M36 did.** Where a `golf-lab` verb replaces a script that prints a
  report (`simulate_flight.py`, `analyze_bundle.py`'s summary, `reanalyze.py --dry-run`), it prints
  what its twin prints, measured against the frozen script's output into scratch, with every
  difference declared in the phase's findings.
- **C9 — The tool-count pin moves to Rust** (`crates/mcp/tests/docs.rs`), reading the same documents
  for `the same N tools`, with N from `golf-mcp`'s tool table. The Python pin is removed in the same
  change (P15), and the sentences move to twelve.
- **C10 — `golf-mcp`'s flags mirror `run_mcp_server.py`'s** (sessions, shots and golfers
  directories, with the same defaults `golf-core`'s career verbs use).
- **C11 — An import conflict refuses the whole import.** `golf-lab import-export` writes nothing if
  any path would change content; identical files are skipped, shots dedupe by photo sha.
  *Added 2026-11-05 by M38's D6 ([the M38 plan](m38-iphone-app.md), its P12):* a golfer file differing
  by bytes is read on both sides. The same `player_id` and handedness is kept as the lab's, with a line
  naming the fields that differ, and a different handedness or an unreadable side stays a conflict. A
  bag differing by bytes is kept as the lab's, with a line saying the phone's was not applied. This is
  the value-level row P31's finding said a golfer rule would need. `session.json`, shots and manifests
  keep their rules.
- **C12 — The backup goes to `%USERPROFILE%\golf-coach-backups\m29-pre-write\`**, outside the repo,
  with a `SHA256SUMS` file and the digest in P29's finding.
- **C13 — The `misread` rewording.** The "not capture problems … the footage is not what went wrong"
  sentence gains wording for a photo worth retaking, in the Rust caveats (P9), declared against the
  recorded surface. Python's copy is frozen and unchanged.

---

## Phases

### P0 — This plan document and the pointers to it

- **Goal**: this file at `docs/plans/m29-lab-port.md`, and the pointers that make it findable.
- **Read**: this file; `ROADMAP.md`'s status table row for M29 and §M29 only; the program plan's
  §M29 opening paragraph.
- **Touch**: `docs/plans/m29-lab-port.md` (new); `ROADMAP.md` (the M29 row and §M29's Status and
  Detail lines point here; status reads "planned 2026-10-15, P0 done"); the program plan's §M29
  ("It gets its own `docs/plans/m29-*.md` when it starts" becomes the link).
- **Done**: the file exists with this checklist; P0 ticked.
- **Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P1 — The ONNX models, their manifest, and `crates/ocr`'s skeleton

- **Goal**: the three models as ONNX in `data/models/ocr/`, pinned by a committed manifest, loading
  into `ort` sessions from a new crate that is the only home of `ort`.
- **Read**: `launch_monitor/screen/paddle.py`; each model's `inference.yml` under
  `~/.paddlex/official_models/{PP-OCRv6_medium_det,PP-LCNet_x1_0_textline_ori,PP-OCRv6_medium_rec}/`
  (preprocessing, postprocessing and the rec character dictionary); the root `Cargo.toml`;
  `crates/trigger/Cargo.toml` and its crate doc (how `rustfft` is named as the exception);
  ADR-035's Consequences; D5, D6, D18.
- **Do**:
  1. Take the `data/processed` digest and record it.
  2. Install PaddleX's `paddle2onnx` plugin into `.venv` (needs the network) and convert the three
     models into `data/models/ocr/<model>/model.onnx`; copy the rec dictionary beside its model.
     Record the exact commands, versions and opset in the finding.
  3. `crates/ocr` (add to the workspace): `ort` pinned exactly with `download-binaries`;
     `models.rs` reads `crates/ocr/models.json` (per model: source dir, source `inference.pdiparams`
     sha256, ONNX sha256, opset, and the preprocessing constants from `inference.yml`) and opens a
     session per model, refusing a missing file or a sha mismatch by name.
  4. The dependency pin: a test walking the workspace's path dependencies from each `Cargo.toml`
     that fails if `ort` is reachable from `contracts`, `pyfmt`, `analysis`, `feedback`, `screen`,
     `storage` or `core`.
- **Done**: each session loads and runs once on a zero tensor (ignored test, needs the models); the
  inputs' and outputs' names and shapes are recorded as a finding; the refusal and the pin are
  tested; `data/processed` digest unchanged.
- **Verify**: `cargo test`, `cargo test -p <ocr> -- --include-ignored`, clippy, fmt.

### P2 — Photo decode, and the screen-quad finder

- **Goal**: a photo becomes the same pixels frozen Python decodes, and the same screen quadrilateral
  is found on it.
- **Read**: `preprocess.py`'s `load_image`, `_is_heif`, `_load_via_pillow`, `find_screen_quad`,
  `_as_quad`, `_as_rotated_rect`, `_plausible_area`, `_downscale_for_detection`, `_order_quad`;
  finding 4 and 5 above; P1's findings.
- **Do**: `decode.rs` — JPEG and PNG through `image` with the EXIF orientation applied; HEIF brands
  (the same `ftyp` set as `_HEIF_BRANDS`) through ffmpeg (`GOLF_FFMPEG`, then `PATH`) to PNG on a
  pipe. `quad.rs` — `find_screen_quad`'s twin on `imageproc`: the 1000 px downscale, bilateral filter,
  the Canny presets, a rectangular close, external contours, the five largest, polygon approximation
  at each epsilon, convexity, the area and fill checks, the `minAreaRect` fallback, corner order.
  Reference: a throwaway scratchpad script runs frozen `load_image` and `find_screen_quad` on the 13
  photos and prints the decoded size and the quad's corners.
- **Done**: decoded sizes identical on all 13 (the HEIC one included); quad found or not found as
  Python finds it on every photo; corners within a pixel tolerance this phase sets and records.
- **Verify**: `cargo test -p <ocr> -- --include-ignored`, clippy, fmt.

### P3 — Rectification: warp, CLAHE, resize, rotate

- **Goal**: the image the recognizer sees, close enough to frozen Python's that the models cannot
  tell.
- **Read**: `preprocess.py`'s `warp_to_rect`, `enhance`, `_resize`, `_rotate`, `prepare_screen`;
  P2's findings.
- **Do**: `rectify.rs` — the perspective warp (bilinear, as `warpPerspective`'s default), target
  size as Python computes it; CLAHE on LAB lightness, hand-written to OpenCV's clip-limit and tile
  interpolation; area resampling to 1600 px wide; the four rotations. Reference: the scratch script
  writes frozen Python's working image (after `enhance`, before the vote) per photo as PNG.
- **Done**: sizes identical on all 13; mean and 99th-percentile absolute pixel difference within
  bounds this phase sets and records. A bound it cannot meet is a finding, not a looser bound.
- **Verify**: as P2.

### P4 — Text detection: the det model and DB postprocessing

- **Goal**: the same text boxes PaddleX's detector finds, on the same image.
- **Read**: the det `inference.yml` (resize limit and type, normalization, DB thresholds,
  `unclip_ratio`, `max_candidates`); PaddleX's text-detection processors in `.venv`
  (`paddlex/inference/models/text_detection/`), only the resize, normalize and DB postprocess
  functions; P1 and P3's findings.
- **Do**: `det.rs` — resize to multiples of 32 under the limit, normalize, NCHW; run; DB
  postprocess: threshold the probability map, contours, `minAreaRect` boxes, the fast box score,
  the unclip (a rectangle grown by `area × ratio / perimeter`, which is what a rounded offset's
  `minAreaRect` gives back), the minimum size, rescale and clip; PaddleX's box order. Reference: the
  scratch script runs PaddleX's det model **alone** on P3's frozen-Python images, so detection is
  compared on identical input.
- **Done**: on all 13, box count identical and every box's corners within a tolerance this phase
  sets and records.
- **Verify**: as P2.

### P5 — Recognition: crop, the rec model, CTC decoding

- **Goal**: the same text, read from the same box.
- **Read**: the rec `inference.yml` (image shape, dictionary, `use_space_char`); PaddleX's rec
  processors (resize-normalize-pad, CTC label decode) and its rotate-crop helper; P4's findings.
- **Do**: `rec.rs` — the perspective crop of a box, the rotation when height ≥ 1.5 × width, resize
  to height 48 with the width ratio and padding, normalize, run, greedy CTC (argmax, collapse
  repeats, drop the blank), the score as PaddleX computes it. Reference: the scratch script runs
  PaddleX's rec model on its own crops of P4's boxes; Rust is fed the same boxes.
- **Done**: identical text on every box of all 13; scores within a recorded tolerance.
- **Verify**: as P2.

### P6 — Textline orientation, and the `recognize` seam

- **Goal**: `recognize(image) -> Vec<TextBox>`, the `TextRecognizer` port's Rust twin.
- **Read**: the textline-orientation `inference.yml`; PaddleX's classification processors;
  `paddle.py`'s normalization into `TextBox` (the 3.x branch); `recognizer.py`;
  `crates/screen/src/lib.rs`'s `TextBox`; P4–P5's findings.
- **Do**: `cls.rs` — classify each crop 0°/180° and flip it before recognition, with PaddleX's
  threshold. `recognizer.rs` — det → crop → cls → rec → `TextBox`, mapped and filtered exactly as
  `paddle.py` does. Compare against frozen `PaddleOCRRecognizer.recognize` (the full five-model
  pipeline) on P3's images: this is where D5's skipped models first show.
- **Done**: per photo, the texts the profile's labels and values need are all present and equal;
  every other difference is listed in the finding. If a parser-relevant text differs, the finding
  says so **for P7**, with whether UVDoc is the likely cause.
- **Verify**: as P2.

### P7 — The reader end to end, and the OCR gate on the 13 photos

- **Goal**: a photo in, the parsed shot out, and D7's gate passing on all 13.
- **Read**: `preprocess.py::prepare_screen`; `importer.py::import_screen` (what reaches the parser);
  `crates/screen/src/lib.rs`'s `read` and `ScreenInput`; `crates/screen/src/orient.rs`;
  `spec/vectors/screen/corpus/*.json` (the committed boxes and expected shots); P2–P6's findings.
- **Do**: `prepare.rs` — `prepare_screen`'s twin: quad or the uncropped note, `enhance`, the
  rotation vote with the early stop, the notes worded as Python's. `read_photo(path, profile)` as
  the crate's entry. A small `golf-ocr read <photo> [--json]` bin printing the parsed shot through
  `crates/screen`. The gate: an ignored test over the 13 corpus photos (found through each vector's
  bundle manifest), comparing `screen::read` over the Rust boxes with the vector's expected shot:
  every value, `fields_present` and `needs_review`.
- **Done**: 13 of 13 pass. A failure is a finding and a stop; if it is geometry, the finding
  proposes a UVDoc phase for the user to approve (D5).
- **Verify**: `cargo test`, `cargo test -p <ocr> -- --include-ignored`, clippy, fmt; digest.
- **Decided (user, 2026-10-21), on P7's stop over photo 2026-08-10-1: option (a), no UVDoc.** The
  gate in `crates/ocr/tests/read.rs` names this one difference — the Rust reader's
  `impact_position = "CENTER"` where the committed vector's 5-model reading lost the "V" of
  "Impact Position V" and withheld the tile — and fails if that difference disappears or a second
  one appears. The committed screen vector is not touched. Done is therefore 12 of 13 exact plus
  this one named difference. **For P25**: the input gate must expect two changed values,
  `shot_type` and `impact_position`, on the 3 bundles that share this photo.

### P8 — Record the `mcp` family from frozen Python

- **Goal**: frozen Python's every MCP answer committed as `spec/vectors/mcp/`, before any Rust tool
  exists (clause 3).
- **Read**: `mcp/server.py` (whole); the tool functions' signatures in `mcp/{query,career,club,flight}.py`;
  `tests/mcp/conftest.py` (the fixture trees); `scripts/conformance_vectors.py`'s storage family
  (`build_storage`, `_storage_corpus_real`: how a file tree is encoded) and `_RUST_ONLY_SWING_KEYS`;
  `scripts/conformance.py`'s `check` deferral list; `tests/test_conformance.py`;
  `docs/CONFORMANCE.md` §2's storage and profile sections; `crates/core/tests/rerecord.rs`'s
  `ORACLE_BY_FAMILY`.
- **Do**: `build_mcp()` in `conformance_vectors.py`. A `surface` case: the tool list (name,
  description, input schema) and `instructions` with and without the career tools. Tool cases: a
  given tree (the storage family's encoding), the tool, its arguments, and the expected result as
  the SDK serializes it (structured content and the error flag), produced by calling the built
  server through the SDK's in-memory client. Trees: the `tests/mcp` scenarios, the not-found paths,
  and one slim `data/` snapshot. `oracle: "python"`. `check` defers `mcp` to `cargo test`; the pins
  in `tests/test_conformance.py` and `ORACLE_BY_FAMILY` know the family. Measure its size. C1: say
  which cases carry flight numbers.
- **Done**: the family is committed; every case names its oracle; pytest, `check` and `cargo test`
  green; nothing under `src/` changed.
- **Verify**: pytest (alone), `conformance.py check`, `cargo test`.

### P9 — `crates/mcp`'s skeleton: the stdio server, the caveats, the descriptions

- **Goal**: `golf-mcp` speaks MCP over stdio with Python's surface, and the caveat prose lives in
  Rust.
- **Read**: `mcp/server.py`; `contracts/caveats.py`; `contracts/tool_descriptions.py`;
  `analysis/flight_caveats.py` (where `READING_A_SIMULATED_FLIGHT` is used); the Rust registries
  the prose derives from (`crates/contracts/src/{checkpoints,unscored,placements,pivots,mishit}.rs`);
  `rmcp`'s docs at the pinned version; P8's surface case; C10, C13, D10.
- **Do**: `crates/mcp` with `golf-mcp` (flags per C10). `caveats.rs` (each sentence derived from the
  registries as Python's are, `textwrap.fill`'s width-95 wrapping written to Python's rules, the
  `misread` rewording), `descriptions.rs`, the `instructions`, and the read-only caveats resource
  (D10). The tool table registers all twelve names with their descriptions and input schemas; a
  tool whose body has not landed returns a named "not ported until Pn" error and its cases are
  skipped by name.
- **Done**: the surface case passes, with the rewording declared and input schemas compared on a
  normalization this phase records (property names, types, required, defaults).
- **Verify**: `cargo test`, clippy, fmt.

### P10 — The swing and session tools

- **Goal**: `list_sessions`, `get_swing`, `get_session_summary` and their not-found answers conform.
- **Read**: the parts of `mcp/query.py` they reach (views, `missing_*`); `crates/storage`'s
  `state`, `bundle_store` and `manifest`; `tests/mcp/test_query.py`; P8–P9's findings.
- **Done**: their family cases pass; the rest still skipped by name.
- **Verify**: `cargo test`, clippy, fmt.

### P11 — The shot tools

- **Goal**: `get_recent_shots` and `get_shot_by_id` conform, with what Rust adds on purpose.
- **Read**: `mcp/query.py`'s shot views and `_METRIC_FIELDS`; `launch_monitor/screen/source.py`
  (the shot source's order); `crates/storage/src/shot_store.rs`; `crates/screen`'s `golfer_warnings`;
  the program plan's §M29 "The new `ShotData` fields join a metric list here" and "The shot view
  goes through M34's golfer-facing filter".
- **Do**: the views; M32's fields in the metric list; the golfer-facing filter on warnings. Each
  departure from the recorded answer is a declared path or a hand case, listed in the finding.
- **Done**: their cases pass.
- **Verify**: as P10.

### P12 — The career tools

- **Goal**: `get_golfer_profile`, `get_shot_trends` and `compare_sessions` conform.
- **Read**: `mcp/career.py`; `tests/mcp/test_career_tools.py`; `crates/storage/src/corpus.rs`;
  `crates/analysis`'s `baseline`, `dispersion`, `comparison`; `crates/core/src/reports/`;
  ADR-024's M35 and M36 addenda (where Rust's corpus differs on purpose: `COMPARABLE_FROM`,
  photo-only admission, drills untracked).
- **Done**: their cases pass; every case where Rust's corpus differs from frozen Python's on purpose
  is declared or replaced by a hand case, and the finding lists them.
- **Verify**: as P10.

### P13 — The club tools

- **Goal**: `get_bag_profile` and `get_club_profile` conform.
- **Read**: `mcp/club.py`; `tests/mcp/test_club_tools.py`; `crates/storage`'s `bag_store`,
  `golfer_store` and `catalogue`; `crates/analysis/src/club_profile.rs`.
- **Done**: their cases pass.
- **Verify**: as P10.

### P14 — `simulate_flight`

- **Goal**: `simulate_flight` conforms, with the flight caveats in Rust.
- **Read**: `mcp/flight.py`; `analysis/flight_caveats.py`; `tests/mcp/test_flight_tool.py`;
  `crates/analysis`'s `flight`, `flight_infer`, `flight_measure`; `crates/storage/src/flight_inputs.rs`.
- **Done**: its cases pass.
- **Verify**: as P10.

### P15 — `get_strike_profile`, the stdio handshake, the re-record walk, the tool-count pin

- **Goal**: the twelfth tool, proof the binary works as a server, and the pins moved.
- **Read**: `crates/core/src/profile.rs` (`run_profile`); `golf-core profile --json`'s shape (M37's
  "What M42, M38 and M29 should know"); `crates/core/src/rerecord.rs` (how the career declaration
  walks storage → career → profile); `tests/test_docs_truth.py::test_the_docs_state_the_real_mcp_tool_count`;
  `scripts/ask_swing.py`'s docstring; C1, C9.
- **Do**: `get_strike_profile` (description written new), with hand cases and corpus cases recorded
  once by Rust (`oracle: "rust"`). An integration test that spawns `golf-mcp`, initializes, lists
  twelve tools, calls one, reads the caveats resource and gets a not-found answer. The career
  re-record walks `mcp` after `profile`. The tool-count pin moves to `crates/mcp/tests/docs.rs`, the
  Python pin is removed, and every "the same N tools" sentence reads twelve.
- **Done**: no `mcp` case is skipped any more; the `career-v3` dry run reports 0 changed with `mcp`
  walked; pytest and `cargo test` green.
- **Verify**: `cargo test`, the `career-v3` dry run, pytest (alone), docs-truth, clippy, fmt.

### P16 — `conversation.py` over `golf-mcp`; `ask_swing.py` switches

- **Goal**: ADR-020's Option B: follow-up questions answered through the Rust server over stdio.
- **Read**: `feedback/conversation.py`; `feedback/coach.py`'s `_sdk`, `_note_for`, `_text_from`;
  `mcp/runner_tools.py`; `scripts/ask_swing.py`; `tests/feedback/test_conversation.py`; the SDK's
  `anthropic/lib/tools/mcp*` in `.venv`; ADR-020 (Option B and its addenda); ADR-035's
  sub-decision.
- **Do**: a stdio client session to `golf-mcp` kept on an event loop the tool runner's thread can
  call into (`anyio.from_thread`), tools from the server's `list_tools` through the SDK's MCP
  helpers, and the briefing's caveat from the server (D10). `ask_swing.py` uses it, with the binary
  path defaulting to the release build and a missing binary an exit-2 message naming the build
  command. The in-process route stays for the frozen `api/app.py`. Tests with the real binary,
  skipped by name when it is not built.
- **Done**: tests green; one live question on a stored swing answered through `golf-mcp`
  (`ask_swing.py`, one API call) and its transcript stored.
- **Verify**: pytest (alone), ruff, mypy.

### P17 — `coach.py`'s JSON entry over `golf-mcp`

- **Goal**: `python -m golf_coach.feedback.coach --session S --swing W` prints one JSON object.
- **Read**: `feedback/coach.py` whole; `mcp/query.py`'s `SwingView` (and what P10 built); P16's
  findings; D15, finding 8.
- **Do**: the entry: read the swing through `golf-mcp` (and the shot, if the view needs it), build
  the brief from the view JSON with the caveats from the server, call Claude, print
  `{text, provenance}` or `{note}`, exit 0, or 2 on no answer. The pydantic `build_brief` stays for
  `api/pipeline.py`. Every brief line the view cannot feed is listed: each either gains an added key
  on the Rust `SwingView` (declared in the `mcp` family) or is dropped with a reason.
- **Done**: tests green; one live call on a stored swing prints a paragraph.
- **Verify**: pytest (alone), ruff, mypy; `cargo test` if the view changed.

### P18 — Record the `selection` family from frozen Python

- **Goal**: what frozen Python's swing selection chooses, committed, before the port (clause 3).
- **Read**: `analysis/phases.py`'s selection half (finding 6); `api/pipeline.py`'s `_auto_windows`,
  `_pick_swing`, `_narrate_choice`, `_downswing_seconds`; `tests/analysis/test_select_swing.py`;
  `conformance_vectors.py`'s `_corpus` and `_strikes`.
- **Do**: measure which landmarks and frames selection reads. `build_selection()`: for each stored
  bundle, the input (each view's track cut to that, plus the strikes) and the expected choices,
  windows and the notes `_auto_windows` appends; plus the synthetic tracks from the tests. Gzipped,
  `oracle: "python"`, ageing on `ANALYSIS_VERSION`; `check` defers it. Check that the recorded
  windows equal each stored `analysis.json`'s, so the family says what the stored corpus was built
  from. If the family exceeds about 10 MB, that is a finding and a question before committing.
- **Done**: committed; pytest, `check` and `cargo test` green.
- **Verify**: pytest (alone), `conformance.py check`, `cargo test`.

### P19 — Port the swing selection to the family

- **Goal**: `select_swing` and `select_matching_swing` in `crates/analysis`, conforming.
- **Read**: P18's findings; the Python functions; `crates/analysis/src/phases.rs` (what is already
  there); `crates/core/src/rerecord.rs`.
- **Do**: the port; the family's runner; the engine re-record walks `selection` after `shot`.
  `_auto_windows`' cases stay skipped by name until P25.
- **Done**: the analysis-level cases pass; the `v17` dry run reports 0 changed with `selection`
  walked.
- **Verify**: `cargo test`, the `v17` dry run, clippy, fmt.
- **Decided (user, 2026-10-22, asked at the P18/P19 boundary)**: the first `v17` re-record walk
  **may** write `spec/vectors/selection/`, moving every case's engine version 16 → 17 with a
  `provenance.rerecords` ledger entry. This is the same allowance P15 had for `mcp`, and the
  exception to "only P18 writes `selection/`".

### P20 — `crates/lab`'s skeleton; the audio contract and its store

- **Goal**: `golf-lab` exists, and reads and writes `{role}.audio.json` as frozen Python does.
- **Read**: `contracts/audio.py`; `storage/audio_io.py`; `crates/core/src/bin/golf_core.rs` (the
  argument handling to copy); `crates/contracts/src/lib.rs`.
- **Do**: `crates/contracts`' `audio` module (its Python twin named); `crates/storage`'s audio store;
  `crates/lab` with an empty `golf-lab`.
- **Done**: every stored `audio.json` round-trips structurally (ignored test on `data/`); unit
  tests for the bounds.
- **Verify**: `cargo test` (with `--include-ignored` for the lab), clippy, fmt; digest.

### P21 — Strike audio: ffmpeg, the video-start probe, `audio_for`

- **Goal**: a clip's strikes heard by Rust, cached as frozen Python caches them.
- **Read**: `audio/ffmpeg.py`; `audio/source.py`; `api/pipeline.py`'s `audio_for`,
  `_frames_derived`, `_frame_of`; `crates/trigger/src/offline.rs`; D17.
- **Do**: ffmpeg lookup and PCM extraction with frozen Python's arguments; the video-start probe;
  `audio_for`'s cache rule and its notes; `detect_strikes`.
- **Done**: forced detection over all 30 stored clips, written into a scratch copy, reproduces
  every stored `audio.json`'s strikes. The PCM from ffmpeg 9.0.2 is compared once with `.venv`'s
  7.1 on one clip, and the finding records whether they are bit-identical.
- **Verify**: as P20.
- **Decided (user, 2026-10-22, asked when P21 stopped on its gate)**: option (a). The committed
  gate, `crates/lab/tests/strikes.rs`, runs on PATH's ffmpeg (9.0.2, the one D17 says the lab
  runs). Every strike's sample, frame and rank, and the clip block (fps, video start, sha,
  stream), stay **exact**. `duration_s`, `confidence` and `prominence` are named in the test's
  doc and held to **2e-3 relative**, because 9.0.2 stops each audio track at the edit list's end
  and 7.1 decoded 16–976 samples past it. The exact 30-of-30 pass under `.venv`'s 7.1 is
  recorded once in the findings and is not part of the gate.

### P22 — Pose: `keypoints_for` over `crates/pose`

- **Goal**: `golf-lab` is `crates/pose`'s first caller, and the keypoints cache rule holds.
- **Read**: `api/pipeline.py`'s `keypoints_for`, `_older_pose_note`, `_band_estimator_note`;
  `pose/estimator.py::pose_estimator_name`; `crates/pose`'s `pool`, `worker`, `writer` and crate
  doc; ADR-033; C3.
- **Do**: the cache check (the clip's sha and the estimator), the notes, and a forced run through the
  pool, writing `{role}.keypoints.json`.
- **Done**: all 30 stored views are cache hits and spawn no worker; one forced run on the smallest
  stored clip, in a scratch copy, compared with the stored keypoints (frame count equal, landmark
  differences recorded). Precondition: the pose model file the worker loads is present; if not,
  stop and ask.
- **Verify**: as P20.

### P23 — The shot lookup and the re-read, dry run

- **Goal**: `_shot_for`'s twin, which re-reads every unstamped shot through the Rust reader.
- **Read**: `api/pipeline.py::_shot_for`; `launch_monitor/screen/{importer,store}.py`;
  `crates/storage/src/shot_store.rs`; `crates/contracts/src/shot.rs::parse_is_current`; P7's
  findings; finding 2.
- **Do**: store lookup → `parse_is_current` → re-read the photo through its bundle → stamp, with
  every note Python writes. A dry run writes nothing.
- **Done**: on the 13: all are found unstamped; re-read values are identical except the two
  label-fix shots' declared ones; every provenance change is one finding 2 lists, or a finding and a
  stop.
- **Verify**: as P20.

### P24 — `golf-lab import-photos`

- **Goal**: photo import, store-only and into a session.
- **Read**: `scripts/import_shot_screens.py`; the program plan's §M29 "Its verbs";
  `crates/storage`'s `bundle_store` and `session_meta`; `crates/core/src/session.rs` (M35's
  `run-shot --session`); C7.
- **Do**: `golf-lab import-photos <paths|dir> [--session S] [--force] [--dry-run]` (the store), and
  `--player P --club C --into-session S` (photo-only entries, each analysed through `run_shot` into
  `shot_analysis.json`). Exit codes as the script's.
- **Done**: on scratch directories: storing, dedupe by photo sha, skip unless `--force`, the exit
  codes, and a photo entry that Rust's `read_corpus` admits.
- **Verify**: as P20.

### P25 — The bundle pipeline, and the input gate on the 15 bundles (dry run)

- **Goal**: `analyze_swing_dir`'s assembly in Rust, gated by D20 before anything is written.
- **Read**: `api/pipeline.py::analyze_swing_dir`, `_auto_windows`, `_pick_swing`,
  `_narrate_choice`, `_handedness_for`, `_loft_for`, `loft_remedy`, `_band_estimator_note`;
  `conformance_vectors.py::_corpus_vector` and `_identity` (how a vector's input is cut);
  `crates/core`'s `run`; P19–P23's findings.
- **Do**: the assembly and its notes; `_auto_windows` (the `selection` family's remaining cases).
  The gate: an ignored test over the 15 bundles, comparing each assembled input, cut as
  `_corpus_vector` cuts it, with the committed corpus vector's input.
- **Done**: no `selection` case skipped; all 15 inputs equal except the declared shot paths (values
  on the four bundles carrying a label-fix shot, stamps on all). Nothing written.
- **Verify**: as P20; the `v17` dry run.

### P26 — The writers, coaching, and `golf-lab analyze`

- **Goal**: `golf-lab analyze <SESSION/SWING>` end to end on a scratch copy.
- **Read**: `api/pipeline.py`'s writes, `record_state` and `PipelineOutcome`;
  `scripts/analyze_bundle.py` (flags, summary, exit codes); `crates/storage/src/state.rs`;
  P17's entry; C4, C5, C6, C8.
- **Do**: `analysis.json` (excluding keypoints and detections), `analysis.state.json`, coaching by
  spawning P17's entry, the flags, the summary and the exit codes 0/1/2. The pin for C6.
- **Done**: on scratch copies of two bundles, written artifacts load in frozen Python's readers (a
  throwaway script); one live coaching run folds its paragraph in; the summary matches the script's
  per C8.
- **Verify**: as P20.

### P27 — `golf-lab reanalyze` and `golf-lab analyze-keypoints`

- **Goal**: the staleness re-run, and a keypoints file analysed with no bundle.
- **Read**: `scripts/reanalyze.py`; `scripts/analyze_swing.py`; `crates/storage/src/state.rs`'s
  `matches`, `is_older_than`, `shot_analysis_matches`; C5, C7, C8.
- **Do**: `reanalyze [TARGET] [--player P] [--all] [--dry-run] [--coaching]`, covering photo-only
  directories; `--video` refused with a sentence saying no render exists (C4).
  `analyze-keypoints <file>` with `analyze_swing.py`'s report; `--overlay` refused the same way.
- **Done**: `reanalyze --dry-run` over `data/` lists all 15 as stale, each with Python's reason;
  photo-only scratch directories re-run; the reports match per C8.
- **Verify**: as P20.

### P28 — `golf-lab flight`

- **Goal**: `simulate_flight.py`'s twin.
- **Read**: `scripts/simulate_flight.py`; `analysis/flight.py`'s `VALIDATION_SHOTS`;
  `crates/analysis`'s `flight`, `spin_solve`, `flight_infer`; `crates/storage/src/flight_inputs.rs`;
  C8.
- **Do**: typed conditions, `--gate`, `--shots`, `--shot`, `--points`, `--altitude`, `--verbose`.
- **Done**: each mode's text matches the frozen script's per C8.
- **Verify**: as P20.

### P29 — The `data/` write: backup, re-read the 13, re-run the 15

- **Goal**: the milestone's data exit: shots stamped, bundles re-run by Rust, the frozen server
  still reading.
- **Preconditions**: P23–P28 green; nothing else running against `data/`.
- **Read**: D8, D20, C5, C12; P23, P25 and P26's findings.
- **Do**:
  1. Back up `data/processed/` minus `.MOV`/`.mp4` per C12; record the file count and digest.
  2. `golf-lab reanalyze --all` (no coaching), which re-reads and stamps the 13 shots and rewrites
     the 15 `analysis.json` and `analysis.state.json`.
  3. Verify: all 13 shots stamped; against the backup, only the two label-fix shots change a field
     value; `keypoints.json` and `audio.json` byte-identical; every `analysis.json` at Rust's
     version. A throwaway script drives the frozen app through `TestClient` over every read route
     for every session, swing, shot and golfer, and loads every artifact through frozen Python's
     readers: no exception, no 5xx. `golf-core profile --name Aaron`, the career verbs and a
     `golf-mcp` call answer over the new `data/`.
  4. Record the new digest for every later phase.
- **Done**: all of step 3 holds. Any failure: restore from the backup, record, stop.
- **Verify**: `cargo test` (with `--include-ignored` for lab and ocr), the step 3 script.

### P30 — `golf-lab build-vectors`, the builder that reads `data/`

- **Goal**: new vectors from `data/` are Rust's (clause 3, D19).
- **Read**: `conformance_vectors.py`'s `_corpus`, `_corpus_vector`, `_identity`, `_strikes`,
  `_source_frames`, `_verify_against_stored`, `build_stages` and `_storage_corpus_real`;
  `crates/core`'s `compare` and `stages`; P29's findings.
- **Do**: the builder, writing only absent files and refusing an existing one by name.
- **Done**: built into scratch, the 15 corpus vectors and their stage vectors match the committed
  ones structurally, except the declared shot paths of P25; the real storage corpus's differences
  from the committed one are listed and explained. Nothing committed.
- **Verify**: as P20.

### P31 — `golf-lab import-export`, the phone export's import

- **Goal**: what M38 P4 waits on (D16).
- **Read**: the program plan's §M38 P1 and P4; `crates/storage`'s stores and `corpus`; C11.
- **Do**: `import-export <zip|dir> [--dry-run]`: copy new files, skip identical ones, dedupe shots
  by photo sha, refuse the whole import on any conflict, naming the paths. Zip support is a new
  dependency: a finding first.
- **Done**: hand-built fixtures cover a photo-only session with its `shot_analysis.json`, a shot, a
  golfer and a bag, a re-import, and a conflict; after import, `read_corpus` admits the entries.
- **Verify**: `cargo test`, clippy, fmt.

### P32 — Close the frozen server's write routes

- **Goal**: D11.
- **Read**: `api/app.py`'s write routes (finding 9); `tests/api/test_uploads.py` and the tests of
  the closed routes; ADR-016's addenda.
- **Do**: each closed route answers 410 with a sentence naming the Rust verb that replaces it, and
  a comment at the routes says why the frozen server changed. Tests of the closed routes become
  tests of the refusal; `api/worker.py` and its own tests stay.
- **Done**: pytest, ruff and mypy green; the read routes still serve P29's `data/`.
- **Verify**: pytest (alone), ruff, mypy.

### P33 — The archive move, with the doc paths rewritten

- **Goal**: clause 5's archive move.
- **Read**: ADR-035 clause 5; finding 12 and 13; `pyproject.toml`'s extras.
- **Do**: `git mv` `scripts/golfdb/`, `scripts/caddieset/`, `scripts/hand_landmark_reliability.py`,
  `scripts/{trigger_replay,pose_replay}.py` and `src/golf_coach/contracts/reference.py` into
  `archive/`; the archived scripts still import what they need; drop the `research` extra; re-point
  or move every test that loads them; rewrite every doc path that names them (re-count first).
- **Done**: pytest, ruff, mypy and docs-truth green; `grep` finds no stale path outside history.
- **Verify**: pytest (alone), ruff, mypy, docs-truth.

### P34 — Delete the lab scripts, `detection/`, `frontend/`; the extras; re-register MCP

- **Goal**: clause 5's first retirement moment.
- **Read**: the M31.5 inventory's R26, R28–R30, R32, R34, R43, R44; finding 10 and 13; README's MCP
  section; D21.
- **Do**: delete `scripts/{analyze_bundle,analyze_swing,reanalyze,import_shot_screens,simulate_flight}.py`,
  `scripts/{career_baseline,career_corpus,career_dispersion,club_profile,flag_mishit}.py`,
  `scripts/backfill_golfer.py`, `scripts/run_mcp_server.py`, `scripts/{run_pose,align_swings}.py`,
  `src/golf_coach/detection/` and `frontend/`, each with its tests; `vision` loses `ultralytics` and
  the `hardware` extra goes. Doc paths naming them point at the Rust verbs. Re-register the MCP
  server (D21) and update README.
- **Done**: pytest, ruff, mypy, docs-truth green; `claude mcp list` reports `golf-coach` Connected
  through `golf-mcp`.
- **Verify**: pytest (alone), ruff, mypy, docs-truth.

### P35 — Docs: CONFORMANCE, ARCHITECTURE, `CLAUDE.md`, the ADR addenda, the program plan

- **Goal**: the docs say what was built, routed rather than restated.
- **Read**: every finding above; `docs/README.md` (the map); the sections each edit touches.
- **Do**: CONFORMANCE §2 (the `mcp` and `selection` families, the OCR gate), §5's tiers;
  ARCHITECTURE §1 (`golf-lab`, `golf-mcp`, `golf-ocr`), §2 (the crates and their edges), §4
  (`data/models/ocr/`); `CLAUDE.md` (commands, the crate count, the Rust map, what Python remains);
  addenda to ADR-035 (the four deferred questions answered; `ort` and `rmcp` named), ADR-014,
  ADR-006, ADR-020, ADR-016, ADR-033, ADR-022; the program plan's §M29 pointing here with what
  changed; `data/README.md`.
- **Done**: docs-truth green; every claim routes to code or a finding.
- **Verify**: docs-truth; pytest (alone).

### P36 — Close: verify, WORKLOG, ROADMAP, memory, and the one commit

- **Goal**: M29 closed, in one commit onto `main`.
- **Do**: every verify command, the ignored gates included; the digest against P29's; a WORKLOG
  entry at the top; ROADMAP's status row, §M29 and NEXT ACTION; `docs/README.md`'s count; the
  memory files; stage only the intended paths (never `data/` or a model file) and commit.
- **Done**: all green; one commit.

---

## Exit

The program plan's, refined by the decisions above.
- **The OCR gate passes on the 13 stored bay photos** (D7): values, `fields_present` and
  `needs_review` identical to the parse of PaddleOCR's committed boxes.
- **`golf-lab` re-runs every stored bundle in `data/`** after D20's input gate, and the frozen
  server still starts and reads every artifact it wrote (P29).
- **The 13 stored shots are stamped**, and only the two label-fix shots change a field value.
- **`scripts/ask_swing.py` answers through `golf-mcp` over stdio**, and the tool-count pin reads the
  Rust list.
- **The archive move and the deletions are done**, with the doc paths rewritten in the same change;
  the frozen server's write routes refuse (D11).
- ADR-035's four deferred questions are answered in its addendum (D7, D10, D11, D12).

## Phase findings

### P0 (2026-10-16)

- Planned on 2026-10-15 from the interview recorded above, and approved by the user. The planning
  session installed ffmpeg 9.0.2 on the user's instruction (D17), and that changed D3 from
  ImageMagick to ffmpeg.
- Pointers written: ROADMAP's M29 status row, §M29's Status and Detail lines, and the program
  plan's §M29 opening paragraph. Nothing outside `docs/plans/` and `ROADMAP.md` changed.
- **For P1**: ROADMAP's M29 row and §M29 say "Planned"; the phase that changes M29's state again is
  P36. `docs/README.md` is untouched on purpose (the doc-count pin rule above).

### P1 (2026-10-16)

1. **`data/processed` digest: `bd384cc40b236b73e3013b817ae79908d6e5ff4dded25b8803fc35c6b7af01cc`
   over 192 files**, M37's value, the same at the phase's start and end. Every phase before P29
   compares against it.
2. **The conversion could not run as step 2 wrote it, and ran through the same command instead.**
   `.venv` is **Python 3.13.3** (not the 3.11 `CLAUDE.md` names), and the plugin's pin,
   `paddle2onnx==2.0.2rc3`, has no cp313 wheel, so `paddlex --install paddle2onnx` fails in pip and
   installs nothing (`.venv` is unchanged). The models were converted from a scratch **Python
   3.11.0** venv (`C:\Python311`) running the exact command the plugin builds
   (`paddlex_cli.py::paddle_to_onnx`). That converter needs `paddle` importable, and its extension
   links `libpaddle.pyd` by symbol: 2.0.2rc3 fails to load (`DLL load failed … The specified
   procedure could not be found`) against paddlepaddle 3.3.1, and so does 2.1.0 against 3.3.1 and
   3.2.2. 2.1.0's extension imports 115 symbols from `libpaddle.pyd`, and 3.2.2 and 3.0.0 export only
   one of them, while **3.1.1** exports all of them, and 2.1.0 loads against it. So the
   toolchain is **paddle2onnx 2.1.0, paddlepaddle 3.1.1, onnx 1.17.0, onnxoptimizer 0.3.13,
   polygraphy 0.53.6** (polygraphy installed onnxruntime 1.31.0, onnx_graphsurgeon and sympy into the
   scratch venv itself, for constant folding). The command, per model:
   `paddle2onnx --model_dir ~/.paddlex/official_models/<model> --model_filename inference.json
   --params_filename inference.pdiparams --save_file data/models/ocr/<model>/model.onnx
   --opset_version 7`, with `inference.yml` copied beside it as the plugin does. paddle2onnx raised
   the opset where a graph needed it: **det 11, textline 7, rec 11**. Reconverting det and textline
   reproduced their digests byte for byte. The scratch venv is deleted. `crates/ocr/models.json`'s
   `conversion` block carries all of this.
3. **The conversion is faithful to frozen Paddle.** Against frozen paddlepaddle 3.3.1 (`.venv`,
   oneDNN off, as `paddle.py` runs it) on seeded random inputs, through `.venv`'s onnxruntime 1.28.0:
   the largest output difference is 2.3e-07 for det (two shapes), 3.0e-07 for textline and 1.1e-04
   for rec (mean 2e-09), and every argmax agrees on textline and rec. If P7's gate fails, the
   conversion is not where.
4. **Inputs and outputs** (all `f32`, as ONNX Runtime reports them; also in `models.json`, which
   `Models::open` holds every session to): det `x [-1, 3, -1, -1]` → `fetch_name_0 [-1, 1, -1, -1]`
   (a probability per pixel); textline `x [-1, 3, 80, 160]` → `fetch_name_0 [-1, 2]` (a softmax);
   rec `x [-1, 3, 48, -1]` → `fetch_name_0 [-1, -1, 18710]` (a softmax per step, one step per 8
   pixels of width: 320 → 40).
5. **The dictionary** is `data/models/ocr/PP-OCRv6_medium_rec/dict.txt`, extracted from the rec
   model's `inference.yml` (`PostProcess.character_dict`): 18,708 single-codepoint entries, no
   space, no duplicates, written UTF-8 with LF. The model's 18,710 classes are PaddleX's `blank`
   first, the 18,708, then the space `use_space_char` appends (`text_recognition/processors.py`,
   `BaseRecLabelDecode` and `CTCLabelDecode.add_special_char`); `tests/models.rs` pins the `+ 2`.
6. **The plan's "already gitignored by `data/models/*.onnx`" (D6) was wrong**: that pattern does not
   reach a subdirectory, and `git check-ignore` said `data/models/ocr/<model>/model.onnx` would have
   been committed. `.gitignore` gained `data/models/ocr/`, the one edit outside `crates/`, the root
   `Cargo.toml`/`Cargo.lock` and this file.
7. **`crates/ocr`**: `ort = "=2.0.0-rc.10"` (ONNX Runtime 1.22) with default features off and
   `std`, `download-binaries` and `copy-dylibs` on. rc.10 is the newest `ort` the workspace's
   `rust-version = "1.87"` builds: rc.11 and rc.12 declare 1.88 and rc.14 declares 1.92. `std` pulls
   `ndarray` 0.16 in regardless. The lock gained 52 packages, nearly all `ort-sys`'s download
   machinery (`ureq`, `tar`, the TLS stacks for every platform); `sha2 = "0.10"` and the dev
   dependency `tempfile = "3"` were already in it from `crates/storage`. The first build downloaded
   the runtime and `copy-dylibs` put `DirectML.dll` beside the test binaries in `target/debug/`. No
   `screen` or `contracts` edge yet: each is declared by the phase that first crosses it.
8. **The API later phases use**: `ocr::models::{Manifest, Model, Models, ModelError, Tensor,
   DEFAULT_DIR, MANIFEST_JSON}`. `Models::open(dir)` (or `open_with(&Manifest, dir)`) reads each
   file once, hashes it and loads those same bytes, then checks arity, input and output against the
   manifest; the dictionary's digest and count likewise. `Models::run(Model, Tensor) -> Tensor` is
   one NCHW `f32` tensor in and one out, by the manifest's input name; `Models::characters()` is the
   dictionary in class order from index 1. Refusals, each naming the model: `Missing`, `Unreadable`,
   `Mismatch` (both digests), `Dictionary`, `Arity`, `Graph`, `Runtime`. Sessions use `ort`'s
   defaults (every graph optimisation, the default intra-op threads).
9. **Pins**: `crates/ocr/tests/models.rs` (7 tests, plus `every_model_loads_and_runs_once_on_zeros`,
   ignored for the models, which loads all three, checks the dictionary and runs each on zeros), and
   `crates/ocr/tests/ort_reach.rs` (3: no shipped path from `contracts`, `pyfmt`, `analysis`,
   `feedback`, `screen`, `storage` or `core` reaches `ort` or `ort-sys`; `ocr` alone declares `ort`;
   positive controls so neither passes on a parse that saw nothing). `cargo test`,
   `cargo test -p ocr -- --include-ignored`, `cargo clippy --all-targets` and `cargo fmt --check`
   are green, and `tests/test_docs_truth.py` passes. `git diff -- src/ scripts/ tests/ spec/` is
   empty.
10. **For P4**: the detection constants that apply are **OCR.yaml's** (`thresh 0.3`,
    `box_thresh 0.6`, `unclip_ratio 1.5`, `limit_side_len 64`, `limit_type min`,
    `max_side_limit 4000`), not `inference.yml`'s (0.2, 0.45, 1.4), because paddleocr 3.7.0 passes
    only the arguments it is given and `paddle.py` gives none of these. `models.json` records them so,
    and this was read from the configs and `paddleocr/_pipelines/ocr.py`, not traced through
    `paddlex/inference/pipelines/ocr/`: confirm there before leaning on it. `inference.yml`'s
    `DetResizeForTest` is null, so the predictor's own default (`_get_text_det_resize_defaults`:
    736/min, or 960/max for `_TEXT_DET_MAX_LIMIT_MODELS`) is overridden by the pipeline's. Det's
    `DecodeImage img_mode: BGR` becomes `ReadImage(format="BGR")`
    (`text_detection/predictor.py:164`); what `ReadImage` does with an ndarray rather than a path is
    `common/reader/image_reader.py`, from line 52. The `NormalizeImage` scale is written `1./255.` and
    stored as `0.00392156862745098`.
11. **For P5**: rec has no `NormalizeImage` step; `RecResizeImg [3, 48, 320]` normalises inside the
    resize, and the manifest says so rather than inventing constants. `img_mode` is BGR. OCR.yaml
    batches recognition by 6 (`score_thresh 0.0`), and PaddleX pads a batch to its widest crop
    (`ToBatch`, `text_recognition/processors.py` from line 323), so a crop's input can depend on its
    batch-mates: decide whether to mirror that.
12. **For P6**: textline's `inference.yml` names no image mode. The manifest says RGB because
    PaddleX's `image_classification/predictor.py:76` builds `ReadImage(format="RGB")` and its yml ops
    do not replace it: confirm. Resize to 160×80 (`size [160, 80]`, width first), labels
    `0_degree`, `180_degree`, batches of 6.
13. **For P7**: if a box differs by a hair at a threshold, `ort`'s thread count and graph
    optimisation level are the knobs (`Models` uses the defaults; `load` in `models.rs` is where to
    set them).
14. **For P20**: `crates/lab` depends on `ocr` and never on `ort` directly;
    `ort_reach.rs::ocr_is_the_only_crate_that_declares_ort` fails otherwise. Whether a release
    `golf-lab` needs `DirectML.dll` beside it at runtime is unmeasured; P7 or P20 should run the
    release binary from somewhere other than `target/` once.
15. **For P35**: `CLAUDE.md`'s "Ten crates" is eleven now (and P9 and P20 add two more); `.venv` is
    Python 3.13.3, not the 3.11 `CLAUDE.md`'s first paragraph names; ADR-035's addendum should name
    `ort` rc.10 and ONNX Runtime 1.22, the conversion toolchain in item 2, and `.gitignore`'s new
    line.
16. **For P36**: reconverting needs a Python 3.11 venv with `paddle2onnx==2.1.0`,
    `paddlepaddle==3.1.1` and `packaging` (which paddle2onnx imports and does not declare);
    `models.json`'s digests say whether a reconversion matched.

### P2 (2026-10-16) — blocked on how to decode JPEG, answered, resumed and done

The phase stopped on a question the plan answered wrongly (D3's "JPEG and PNG decode in-process
with the `image` crate"). Items 1–10 are the first attempt's, written when it stopped; items 11–20
are the resumed session's, after the user's answer in item 5.

1. **Built**: `crates/ocr/src/decode.rs` (`load_image`, `is_heif`, `find_ffmpeg`, `FFMPEG_ENV`,
   `DecodeError`), `crates/ocr/src/quad.rs` (`find_screen_quad` → `Option<ScreenQuad { corners,
   source }>`, `order_quad`, `DETECT_WIDTH`), and `crates/ocr/src/cv/`, one hand port per OpenCV
   family: `resize` (`resize_area`, `shrink_to_width`), `filter` (`rgb_to_gray`,
   `bilateral_filter`), `canny`, `morph` (`close_rect`), `contours` (`find_external_contours`,
   `contour_area`, `arc_length`, `approx_poly_dp`, `is_contour_convex`) and `rect`
   (`convex_hull`, `min_area_rect`, `box_points`). RGB throughout. 31 unit tests. The gate is
   `crates/ocr/tests/quad.rs`: an always-run check of the fixture and the ignored
   `every_bay_photo_decodes_and_finds_its_screen_as_frozen_python_does`, held to
   `tests/fixtures/screen_quads.json`, which a scratch script recorded from frozen
   `load_image`/`find_screen_quad`: each photo's file digest, size, decoded-RGB digest, quad
   (float32) and path. It takes about 130 s in a debug build.
2. **Dependency**: `image = "=0.25.9"`, default features off, `jpeg` and `png` only. 0.25.9 is
   the newest that builds on `rust-version = "1.87"` (0.25.10 declares 1.88). It brings `zune-jpeg`
   0.5.15, `zune-core`, `png` 0.18.1, `fdeflate`, `moxcms` 0.7.11 and `pxfm` (its colour
   management, unused here), `bytemuck`, `byteorder-lite`, `num-traits`, `simd-adler32` and
   `miniz_oxide` 0.9 into the lock. **`imageproc` is not used**, and P3 decides whether it ever
   is. Its `canny` blurs with a Gaussian first and quantises angles, its bilateral filter is not
   OpenCV's, and its `find_contours` returns every border pixel from its own start point. The
   start point decides `approxPolyDP` on a closed curve.
3. **Given frozen Python's decoded pixels, the port is exact on all 13**: the 1000 px copy, the
   grey, both edge maps, all six closings, every contour (count, order, points), every polygon
   at all three tolerances, convexity, area, perimeter, and every quad (0.0 px), by the same path:
   corners on 2026-08-23-10 and -11, the rotated box on -08-10-2, -23-1, -23-2, -23-6 and -23-9
   (**the fallback is the common case**), none on the other six. What it took to get there:
   - **OpenCV 4.10's 8-bit grey is 15-bit fixed point** (`B*3735 + G*19235 + R*9798`, `>> 15`).
     The 14-bit weights missed about 300 pixels per photo.
   - **The pip wheel's `bilateralFilter` runs Intel IPP by default.** The port follows OpenCV's own
     C++ loop, with a fused multiply-add in the AVX2 columns. It matches
     `cv2.ipp.setUseIPP(False)` on every pixel, and default `cv2` on all but one pixel of each of
     9 photos. The edges after it are identical either way. Resize, grey, Canny and the closing
     are the same with IPP on or off.
   - `findContours` returns outlines in **reverse discovery order** (lowest first), and Python's
     stable sort by area keeps it.
   - `convexHull`'s final rotation breaks when `lt > 1 && lt + 1 < i`. `lt < i` missed 11 of
     3,000 random sets. Hull, `approxPolyDP`, convexity, area and perimeter are bit-exact against
     `cv2` on 1,756 real contours and 9,000 random point sets.
   - `minAreaRect` is bit-exact on 1,751 of the 1,756 real contours and about 99.9% of random
     ones. The rest are exact ties, where parallel hull edges bound the same box and the last bits
     decide which caliper reports it (angle 90 with the sides swapped, against `-0.0`). The four
     corners agree within 1e-3 px in every case. FMA, strict `<`, `>=` on the cosine, a negated
     start base and a `double` corner were each tried in an emulation, and none explains the
     ties. The gate allows 0.01 px for this.
4. **The decode is where it fails.** The PNGs are identical. The HEIC has the right size
   (5712x4284), with pixels at most 2 levels and on average 0.57 off: ffmpeg's `swscale` against
   libheif's YCbCr-to-RGB. ffmpeg assembles the tile grid in an automatic filter graph, so
   `-sws_flags` changes nothing there and `-vf` is refused. **The ten JPEGs differ by up to 3
   levels on about 0.5% of values, and that changes two quads**: on 2026-08-23-4 Rust finds a
   rotated box where Python finds none, and on 2026-08-23-11 one corner moves 114 px. Measured
   causes:
   - Pillow's RGB equals `cv2`'s (both libjpeg-turbo 3.1).
   - libjpeg-turbo's `jdcolor` tables (16-bit, `ONE_HALF`) on libjpeg-turbo's YCbCr reproduce
     `cv2` exactly. The colour step is solved.
   - `zune-jpeg`'s YCbCr differs from libjpeg-turbo's on about 110k luma values (by 1). Its IDCT
     is not libjpeg-turbo's ISLOW (`jidctint.c`), and its scalar path matches its AVX2 path, so
     neither helps.
   - ffmpeg's `-idct int` and `-idct simple` differ by more.
   - The bay JPEGs are baseline 4:4:4, with no chroma upsampling involved.
5. **The question.** No in-process `image` decode can pass this phase's gate, so the plan has to
   choose how JPEG is decoded:
   - **(a) Hand-port libjpeg-turbo's decode path into `crates/ocr`**: markers, Huffman,
     dequantise, `jpeg_idct_islow` with its range limit, `jdcolor`'s tables (already measured
     exact), and `jdsample`'s fancy upsampling for 4:2:0 and 4:2:2 photos. Exact, with no new
     dependency, in keeping with D4. About 600 lines plus tests. Progressive JPEGs would stay on
     `image` and be named in the doc.
   - **(b) Link libjpeg-turbo through a C binding crate** (`mozjpeg-sys` builds with `cc`, and its
     decoder is libjpeg-turbo's). Exact, but it brings a second native library after `ort`.
   - **(c) Keep `image` and loosen this gate**, naming the two photos and leaving P7's OCR gate
     to say whether the shot values survive.

   **Recommended: (a).** It is D4's own rule applied to the decoder.

   **Answered by the user (2026-10-16): (a), hand-port libjpeg-turbo's decode path** (Huffman,
   ISLOW IDCT, upsampling) into `crates/ocr`, with no new dependency. D3's `image` crate stays for
   PNG only. The resumed P2 builds that.
6. **For P2, resumed**: after the answer, only `decode.rs` (and a module beside it, for (a)) and
   this finding should change. Then `cargo test -p ocr -- --include-ignored` must show the gate
   green, with the HEIC exempt from the pixel digest only. The scratch reference (`ref.py`,
   `compare.py`, `rects.py`, `calipers.py`, `p2_probe.rs` and the dumps) is in this session's
   scratchpad,
   `%LOCALAPPDATA%\Temp\claude\C--Users-aaron-Desktop-repo-golf-coach\2e0f7380-3186-41f1-a0fa-705c935e1d22\scratchpad\p2\`.
   If that is gone, `ref.py` is about 80 lines over `preprocess.py`, and the fixture already holds
   its answers.
7. **For P3**: `cv::resize::{resize_area, shrink_to_width}` are ready for `_resize`'s 1600 px,
   including Python's `int()` truncation. Check every OpenCV call with `cv2.ipp.setUseIPP(False)`
   before porting it, as P2 did. 8-bit `cvtColor` to and from LAB has its own fixed-point tables,
   as grey turned out to have.
8. **For P7**: the HEIC's 2-level colour difference stands under every option above, and it is
   P7's to judge.
9. **For P21**: `ocr::decode::find_ffmpeg()` and `FFMPEG_ENV` already implement D17
   (`GOLF_FFMPEG`, then `PATH`, every miss named). Reuse them rather than writing a second finder.
10. `data/processed` digest unchanged (`bd384cc4…`, 192 files). `git diff -- src/ scripts/ tests/
    spec/` is empty. `cargo test`, `cargo clippy --all-targets` and `cargo fmt --check` pass. The
    ignored gate fails on the ten JPEGs' pixels and the two quads, and only there.
11. **Resumed, not restarted.** `find -newermt '2026-10-16 09:09'` listed exactly item 1's files and
    this plan, nothing half-written, so all of item 1 was kept. The only rework was the JPEG decode.
12. **Built**: `crates/ocr/src/decode/jpeg.rs`, a private submodule of `decode`, porting
    libjpeg-turbo 3.1's decode path by hand. It covers the markers (`jdmarker.c`, with
    `jdapimin.c`'s colour-space rule: JFIF, then Adobe's transform byte, then the component ids
    `R`,`G`,`B`). It covers the Huffman decode (`jdhuff.c`: a 9-bit lookup, then the `maxcode`
    walk), and `jpeg_idct_islow` with `jdmaster.c`'s range-limit table. It covers the default
    fancy upsampling (`h2v1`, `h1v2` and `h2v2`), with box replication for `int_upsample` and for
    chroma too narrow for the filters, and `jdcolor.c`'s YCbCr tables. Last comes OpenCV's EXIF
    orientation, read from the **first** APP1 as `exif.cpp` reads it and applied through
    `image`'s `Orientation`. 8 unit tests, among them the IDCT within one level of the
    real-valued inverse DCT. `decode.rs` routes by the file's bytes: a HEIF brand goes to ffmpeg,
    `FF D8 FF` (cv2's signature) to `jpeg`, and anything else to `image`. A new
    `DecodeError::UnsupportedJpeg { path, kind }` names the kinds it refuses.
13. **The gate is green.** `cargo test -p ocr -- --include-ignored` passes the bay-photo gate: sizes
    identical on all 13, pixels identical on all 12 that are not HEIC (ten JPEGs and two PNGs),
    every quad found or not found by Python's path, corners within 0.01 px (item 3's `minAreaRect`
    ties). It still takes about 131 s in a debug build, so the decode is not the slow part.
14. **A second, always-run gate**: `crates/ocr/tests/jpeg.rs` over `crates/ocr/tests/fixtures/jpeg/`
    (22 JPEGs, 35 KB together, and `expected.json` holding `cv2.imread`'s size and RGB digest for
    each). The ten bay JPEGs are all baseline 4:4:4, orientation 1, with no restart markers, so the
    fixtures cover the rest. A scratch script, since deleted, wrote them. It used `cv2.imencode` at
    quality 90 with each `IMWRITE_JPEG_SAMPLING_FACTOR_*` (4:4:4, 4:2:2, 4:2:0, 4:4:0, 4:1:1) on a
    seeded 45x29 gradient-plus-noise scene. Two narrow photos (3x21 at 4:2:0, 4x9 at 4:2:2) have
    chroma two samples wide, which takes libjpeg's non-fancy path. Others: restart interval 2, a
    97x66 noise photo at q75, a 33x17 one at q100, and grey. `rgb_ids.jpg` is `s444.jpg` with APP0
    stripped and the ids renamed `R`,`G`,`B`. Pillow wrote a 13x7 photo at each EXIF orientation
    1–8 and a CMYK one, and `cv2` a progressive one, the last two for the refusals. **Every photo
    matched `cv2` on the first run**, so the 4:2:x upsampling no bay photo exercises is measured
    exact too. A third test cuts `s420.jpg` short and expects a refusal.
15. **Deliberate differences from `cv2`, each a named refusal.** `UnsupportedJpeg` covers
    progressive, lossless, hierarchical and arithmetic-coded files, 12-bit, any component count
    other than 1 or 3 (CMYK and YCCK are 4), and duplicate component ids. `Undecodable` covers what
    libjpeg-turbo papers over: a Huffman code no table holds, entropy data that runs out, a
    restart marker out of sequence, and a component no scan codes. It also covers a scan naming an
    undefined Huffman table, where libjpeg-turbo substitutes the standard tables for Motion-JPEG.
    A file missing only its EOI after complete scans decodes, as it does in libjpeg. `jpeg.rs`'s
    module doc lists them, with what supporting progressive would take. The message for
    `UnsupportedJpeg` says to re-save as a baseline JPEG or a PNG.
16. **Dependency: one fewer, none new.** `image` is now `features = ["png"]`, per the answer's "the
    `image` crate stays for PNG only", and that drops `zune-jpeg` 0.5.15 and `zune-core` from the
    lock. `moxcms`, `pxfm` and the rest of item 2's list stay, because `png` or `image`'s core
    brings them. `image` still applies the EXIF transform (`Orientation::from_exif`,
    `DynamicImage::apply_orientation`).
17. **Beyond item 6's prediction**, these files changed besides `decode.rs` and `decode/jpeg.rs`:
    `crates/ocr/Cargo.toml` (the feature and its comment), `Cargo.lock`, `crates/ocr/tests/jpeg.rs`
    with its fixtures, and the doc lines in `src/lib.rs` and `src/quad.rs` that still called the
    JPEG decode open.
18. **For P3**: for JPEG and PNG the decoded photo is now `cv2`'s, bit for bit, in RGB order, so
    any difference P3 measures after the warp is P3's own. Only the HEIC differs (item 8).
19. **For P7 and P24**: `load_image` now refuses a progressive or CMYK JPEG by name
    (`DecodeError::UnsupportedJpeg`), where frozen Python would read it. The reader and
    `golf-lab import-photos` should pass that message through rather than wrap it. No stored photo
    is either kind.
20. `data/processed` digest unchanged (`bd384cc4…`, 192 files). `git diff -- src/ scripts/ tests/
    spec/` is empty. `cargo test` (the whole workspace), `cargo test -p ocr -- --include-ignored`,
    `cargo clippy --all-targets`, `cargo fmt --check` and `tests/test_docs_truth.py` pass. Both
    scratch directories are deleted: this session's, and the first attempt's at item 6's path
    (about 580 MB of dumps). If P3 wants `ref.py` back, it is about 80 lines over `preprocess.py`,
    and item 6 describes it.

### P3 (2026-10-16) — cut off once, resumed and done

1. **Resumed, not restarted.** `find -newermt '2026-10-16 11:29'` listed `crates/ocr/src/rectify.rs`,
   `src/lib.rs`, `src/cv/{warp,lab,clahe,mod}.rs`, a scratch probe in `crates/ocr/examples/` and
   this plan. The first attempt's scratch was still in the session scratchpad: its `ref.py`, 593 MB
   of frozen-Python dumps per stage, and the OpenCV 4.10 sources it ported from. All of it was kept.
   The first attempt had the warp and the resize exact and was cut off with L\*a\*b\* still off. It
   had already written `rectify.rs`'s "identical" claim and named test files that did not yet
   exist. Both are true now.
2. **Built**: `crates/ocr/src/rectify.rs` holds `warp_to_rect`, `resize`, `enhance`, `rotate`,
   `Rotation` (with `VOTE_ORDER`, Python's 0/180/90/270, and `degrees`), `working_image` →
   `Working { image, quad }`, and `TARGET_WIDTH`. Under it are three new hand ports in `cv/`, each
   naming the C++ it follows:
   - `cv::warp` has `get_perspective_transform` (OpenCV's 8x8 system, `f32` products entering an
     `f64` LU solve), `invert3` (`invert`'s closed form) and `warp_perspective`. The warp samples in
     fixed point at 1/32 px, with sums grouped by OpenCV's 32-px blocks, 15-bit weights and a black
     border per neighbour.
   - `cv::lab` has `rgb_to_lab` and `lab_to_rgb` (OpenCV's 8-bit tables, `enableBitExactness`).
   - `cv::clahe` has `clahe`, with OpenCV's padding quirk: both axes are padded when either does not
     divide, so 1600x1093 gets tiles 201 wide.

   **`imageproc` never enters** (P2 left it to P3). Its warp is real-valued, and it has neither
   CLAHE nor OpenCV's L\*a\*b\*. No dependency changed.
3. **What the resumed session fixed.** RGB→L\*a\*b\* was off by one L\* on 17,445 of the 2^24
   colours, and on `a` or `b` for a few hundred more. Every miss came from **one cube-root table
   entry** (index 324 of 3,072). OpenCV's `softfloat` `cbrt` (`f32_cbrt`, Turkowski's) truncates
   its mantissa. A correctly rounded root put `32768 * f` on the half, so round-half-even gave
   17746 where OpenCV gives 17745. `f32_cbrt` is ported (`cv::lab`'s `softfloat_cbrt`, with a unit
   test at that entry), and both directions are now exact on every input. `pow` stays `f64::powf`,
   because the exhaustive check passes with it. Clippy flagged `warp.rs`'s back substitution; it
   is rewritten as an iterator in the same order of operations, and the matrices stayed bit-identical.
4. **The bounds this phase sets, and the result.** Sizes are identical at every stage on all 13.
   **Pixels are identical (mean 0, p99 0) at every stage** (warped, 1600 px, enhanced) on the 12
   photos that are not HEIC: 6 warped (2 by corners, 4 by the rotated box) and 6 uncropped. On
   the HEIC, fed frozen Python's own decoded pixels, every stage and `working_image` end to end are
   identical too, so the port adds nothing. From this crate's ffmpeg decode, the HEIC's working
   image differs by **mean 1.10, p99 4, max 17** (3.87 M of 5.25 M values). That is P2's decode
   difference (item 8 above) carried by the warp and spread by CLAHE, and judging it is still P7's job.
5. **New about the HEIC decode: it is a one-sided bias, not noise.** ffmpeg's RGB is never lighter
   than libheif's. On average it is darker by 0.45 (R), 0.84 (G) and 0.42 (B), each value 0 or −1
   below (G 0, −1 or −2), with mean signed difference equal to mean absolute difference. That
   pattern points to a rounding offset in the YCbCr→RGB step, such as a truncation where libheif
   rounds.
6. **Pins**, in `crates/ocr/tests/rectify.rs`:
   - Always run: L\*a\*b\* exhaustively, one test per direction (about 6 s side by side in debug).
     Then a synthetic screen with 9 cases: `enhance` at four sizes covering each CLAHE padding
     branch (neither, both or only the width divisible by 8, and smaller than the grid), the warp
     inside the scene and past its edge, and the three turns. Last, a check of the photo
     fixture's coverage.
   - Ignored: `every_bay_photo_rectifies_as_frozen_python_does`. It cuts each photo by Python's quad
     from `screen_quads.json`, so it measures rectification alone, and it checks every stage's
     digest. The HEIC gets sizes plus each channel's mean within 1.0 level. That is a guard, not
     the bound. A first guess of 0.5, made before measuring, failed on green's 0.57, and 1.0 is the
     decode's own largest bias (0.84) rounded up.

   A mutation check made the padding per-axis, and the "only the width divides" case failed as it
   should. Fixtures: `tests/fixtures/rectify_reference.json` and `rectify/scene.png` (41 KB, written
   by `cv2.imwrite`), and `tests/fixtures/screen_working.json` (per photo: each stage's size and
   digest, plus the enhanced image's channel means). Every OpenCV call was recorded with IPP on
   and off, and none differed.
7. **For P4**: P4 compares on frozen Python's working images, so the HEIC's decode never reaches
   P4. The scratch reference is deleted. Rebuilding it needs a few lines over `preprocess.py`:
   `load_image`, then `find_screen_quad`, then `enhance(_resize(warp_to_rect(image, quad)))`, or
   `enhance(_resize(image))` when the quad is `None`. No turn is needed: no corpus vector's
   `input.notes` has a rotation note, so on all 13 the vote stopped upright, and the committed
   boxes come from the unturned working image. The six "screen outline not found" notes are on
   exactly the six photos P2 found no quad on.
   `cv::rect::{min_area_rect, box_points}` are bit-exact apart from P2 item 3's ties.
   `cv::contours` has only `find_external_contours` (`RETR_EXTERNAL`). DB postprocessing's own
   `findContours` mode and the detector's resize interpolation are P4's to read. `cv::resize`
   holds only `INTER_AREA`.
8. **For P7**: `rectify::working_image` is `prepare_screen` up to its vote. `rotate` and
   `Rotation::VOTE_ORDER` are the vote's turns, and `Working::quad` says whether to add the
   uncropped note. The P3 gate does not run the quad finder; P7's gate is the first committed test
   to run the finder and rectification together on the photos (the scratch probe did, and matched on
   all 13 from Python's pixels). If the OCR gate fails only on the HEIC (`2026-08-23-1`), item 5 is
   the lead. ffmpeg can hand over the raw YCbCr planes for a Rust conversion that matches libheif,
   and that is a change to `decode.rs` (P2's), for the user to approve.
9. **For P36**: `cargo test -p ocr -- --include-ignored` now takes about 9 minutes in a debug build:
   P2's quad gate took 277 s this run (P2 measured 131 s) and P3's 259 s. If later gates make
   it worse, `[profile.dev.package.ocr] opt-level` in the root `Cargo.toml` is the knob. It was not
   taken here, because it is a workspace decision.
10. `data/processed` digest unchanged (`bd384cc4…`, 192 files). `git diff -- src/ scripts/ tests/
    spec/` is empty. `cargo test` (whole workspace), `cargo test -p ocr -- --include-ignored`,
    `cargo clippy --all-targets` and `cargo fmt --check` pass. The four scratch probes in
    `crates/ocr/examples/` (and the directory) and the session's scratch (`ref.py`, the dumps, the
    OpenCV sources) are deleted.

### P4 (2026-10-17) — cut off once, resumed and done

1. **Resumed, not restarted.** `find -newermt '2026-10-16 19:24'` listed `crates/ocr/src/det.rs`,
   `src/clipper.rs`, `src/cv/{contours,draw,resize,mod}.rs`, `src/lib.rs`, `tests/det.rs`, three
   fixtures (`det_reference.json`, `det_boxes.json`, `det/pred_crop.png`), a scratch probe in
   `crates/ocr/examples/` and this plan. The first attempt's scratch was still in the session
   scratchpad: `ref.py` and frozen Paddle's input tensor and probability map for every photo,
   the unit cases, and the scripts that recorded the fixtures. The code built and the always-run
   tests passed. Missing were the module doc's measurements, any run of the photo gate, and this
   finding. All of it was kept. The resumed session re-ran every comparison against the current
   code, ran the photo gate for the first time, and found it failing on the HEIC (item 6).
2. **Built**: `det.rs` holds `det_size` (`resize_image_type0`), `det_input` (the padding, the
   resize and the normalisation, NCHW) → `(Tensor, DetShape)`, `db_postprocess` (`box_type`
   quad, `score_mode` fast, no dilation) → `Vec<DetBox { points: [[i32; 2]; 4], score: f64 }>` in
   the detector's own order, `sort_quad_boxes` (`SortQuadBoxes`, the reading order) and
   `detect(&mut Models, &RgbImage)`. It needs four new ports:
   - `clipper.rs`: `offset_round` and `to_clipper_point`, pyclipper 1.4.0's `JT_ROUND` offset
     (Clipper 6.4.2). The closing union is replaced by `FixupOutPolygon` and a rotation that ends
     the ring at its topmost vertex, which is the union's answer on a convex box (the module doc
     says why the order matters to `minAreaRect`).
   - `cv::resize::resize_linear`: `INTER_LINEAR` on 8 bits, with `VResizeLinearVec_32s8u`'s
     arithmetic on every value. The C++ reads as though the row's tail is rounded another way, and
     ported that way it was a level high on 346 of 400 random sizes.
   - `cv::contours::find_list_contours` (`RETR_LIST`), plus `polygon_area` and
     `polygon_perimeter` on `f32` corners.
   - `cv::draw::{fill_poly, mean_masked}`: `fillPoly` is its outline and its scanline fill
     together, and the mean is OpenCV's own masked loop.

   **No dependency changed**, and `imageproc` is still unused.
3. **The constants, confirmed** (P1 item 10's "confirm there"). The live pipeline's
   `get_text_det_params()` gives `limit_side_len 64`, `min`, `max_side_limit 4000`, `thresh 0.3`,
   `box_thresh 0.6` and `unclip_ratio 1.5`, and `max_candidates 3000` is `inference.yml`'s.
   `ReadImage` passes an array through untouched in BGR (`common/reader/image_reader.py:52`), so
   tensor channel 0 is blue. `models.json`'s two "P4 confirms" now say what was confirmed, and so
   does `models.rs`'s `Normalize` doc. NumPy 2's scalar rules decide three roundings: `pred > thresh`
   compares in `f32`; the normalisation is `f32 × f32`, then `+ f32`, each rounded; and the
   rescale is an `f32` corner times an `f64` scale, rounded half to even.
4. **The reference and the result**, on frozen Python's own working images with PaddleX's det
   model run alone:
   - the input tensor is **bit for bit** on all 13;
   - the postprocess, fed frozen Paddle's own maps, gives **the same boxes, the same scores to
     the bit and the same reading order on all 13** (607 boxes). With IPP on, `cv2.mean` gives
     the same scores on every photo box as OpenCV's own loop does;
   - `ort`'s map differs from Paddle's by 2.2e-4 to 1.1e-3 per photo. One pixel crosses 0.3 on each
     of 2026-08-10-2, 2026-08-23-4, 2026-08-23-5 and 2026-08-23-11. PaddleX's own postprocess fed
     `ort`'s maps gives this crate's boxes exactly, so the port adds nothing. End to end, box
     counts agree on all 13 and **one box of 607 differs**, by 2 px, on 2026-08-23-11;
   - on 2026-08-23-11, Python's onnxruntime 1.28 gives `ort`'s 1.22 map exactly at the default
     optimisation level. Basic or none moves it by 4.3e-6 and leaves it as far from Paddle's
     (7.8e-4). **This corrects the first attempt's claim** ("at every level"), which had reached
     `tests/det.rs`'s doc and has been rewritten there;
   - nothing sits near `box_thresh`: every kept box scores at least 0.646 (2026-08-23-8) and every
     candidate dropped for its score at most 0.527 (2026-08-23-6).
5. **The unit comparisons**, scratch against OpenCV 4.10 and pyclipper 1.4.0: `resize_linear` is
   exact on 400 random sizes and channel counts, `find_list_contours` on 300 random blobs (2,825
   contours), and `fill_poly` on 4,000 random four-gons. The unclip's vertices come back in
   pyclipper's order on 6,000 boxes. `minAreaRect` of the grown polygon differs on 11 of the
   6,000, by up to 0.087 px at a corner. Those are P2 item 3's ties, larger here because the
   polygons are larger, and none moved a box on the 13 photos.
6. **The bound this phase sets, and the HEIC's guard.** The committed gate
   (`every_bay_photo_detects_its_text_as_frozen_python_does`) decodes each photo with this crate
   and cuts it by Python's quad. On the twelve photos whose working image is Python's, it holds
   the input tensor bit for bit, box counts and the detector's order identical, every corner
   within **`CORNER_TOLERANCE_PX = 2`**, and at most **`MOVED_BOXES_ALLOWED = 1`** box across the
   twelve not identical. The first run failed on the HEIC (2026-08-23-1), whose working image
   comes from ffmpeg's decode. It had the same count, but 16 of its 47 boxes moved: 11 by 1 px,
   4 by 2 px and one by 3 px. Two more swapped places in the detector's order, because a pixel's
   change at the top of two blobs on nearly the same row decides which the scan meets first. Fed
   Python's own working image, its boxes are Python's exactly (item 4), so this is P3 item 5's
   decode, not detection. Following P3's precedent, the HEIC is held to a **guard**: the same count
   and, in reading order, every corner within **`HEIC_CORNER_GUARD_PX = 3`** (the worst measured).
   It is the HEIC's only check, so the gate also asserts that the fixture holds exactly one HEIC.
7. **Pins**: `tests/det.rs` has 7 always-run tests. They check the scene's input tensor bit for
   bit, 9 bilinear resizes, 200 fills, 20 `RETR_LIST` scans, 80 unclips, and the postprocess on a
   320x240 crop of 2026-08-23-10's map quantised to 8 bits (boxes, scores to the bit, reading
   order). The last checks `SortQuadBoxes` on all 13 photos' Python boxes, where the detector's
   order is never the reading order. The photo gate is ignored. Fixtures: `det_reference.json`
   (78 KB), `det_boxes.json` (90 KB), `det/pred_crop.png` (5 KB). Unit tests sit in `det.rs`,
   `clipper.rs`, `cv/contours.rs` and `cv/draw.rs`.
8. **For P5**: the boxes to crop are `sort_quad_boxes(&detect(..))`, `i32` corners in the working
   image's pixels (PaddleX holds `int16`). After detection the pipeline (`pipelines/ocr/pipeline.py`,
   lines 404–478) does five things: crops every box (`CropByPolys(det_box_type="quad")`); drops
   any box whose crop is empty; runs textline orientation on all the crops and turns them (P6's);
   sorts the crops by width over height, stably, before recognition, so a crop's batch-mates of 6
   are its neighbours in that order (P1 item 11's padding question); and keeps a text only if
   `rec_score >= text_rec_score_thresh`.
9. **For P6**: frozen `paddle.py` sets only `use_textline_orientation` and `enable_mkldnn`, so
   OCR.yaml's `use_doc_orientation_classify: True` and `use_doc_unwarping: True` stand. In
   the five-model pipeline, det therefore runs on `doc_preprocessor_images`, the UVDoc output, not
   on the working image P4 compared on. P4's boxes are D5's three-model pipeline's. Any geometry
   difference against frozen `recognize` (and the committed corpus boxes) starts there.
10. **For P7 and P36: the det model is slow on this box, and that is ONNX Runtime, not the crate.**
    One det run on a 1600x1216 tensor takes 22–27 s the first time in a process and about 100 s
    the second, in `ort` and in Python's onnxruntime 1.28 alike (20 logical CPUs). The photo gate
    took **1,531 s and 1,537 s** in two debug runs, so `cargo test -p ocr -- --include-ignored`
    now takes about 35 minutes. P3 item 9's `opt-level` knob does not reach the runtime; P1 item 13's
    thread count and optimisation level do, and neither was tried here.
11. `data/processed` digest unchanged (`bd384cc4…`, 192 files). `git diff -- src/ scripts/ tests/
    spec/` is empty. `cargo test` (whole workspace), `cargo test -p ocr -- --include-ignored`,
    `cargo clippy --all-targets` and `cargo fmt --check` pass, and so does
    `tests/test_docs_truth.py`. The probe in `crates/ocr/examples/` (and the directory) and the
    session's scratch (frozen Paddle's maps and tensors, the working images, the unit cases, the
    scripts) are deleted.

### P5 (2026-10-18) — cut off once, resumed and done

1. **Resumed, not restarted.** `find -newermt '2026-10-17 09:38'` listed `crates/ocr/src/rec.rs`,
   `src/cv/warp.rs`, `src/det.rs`, `src/lib.rs`, `tests/rec.rs`, two fixtures
   (`rec_reference.json`, `rec_texts.json`), three scratch probes in `crates/ocr/examples/` and
   this plan. The first attempt's scratch was still in the session scratchpad: `ref.py` (frozen
   Paddle on the 13 photos: every crop, batch, step and reading), `scene_ref.py`,
   `build_fixtures.py`, a NumPy `mean` check and OpenCV's `imgwarp.cpp`. The code built and the
   two scene tests passed. Missing were the decode test, any photo gate, the module doc's
   measurements and this finding, and `tests/rec.rs` had unused imports. All of it was kept, and
   the resumed session ran the photos for the first time.
2. **Built**: `rec.rs` holds `crop` (`CropByPolys(det_box_type="quad")`: `get_mini_boxes`' corners,
   the longer edges truncated, measured as NumPy's `float32` `norm` measures them (no fused
   multiply-add: the same on 200,000 random edges), a bicubic warp with a replicated border, `np.rot90` when the crop
   is at least `TALL = 1.5` times taller than wide), `rec_widths` and `rec_input`
   (`OCRReisizeNormImg` and `ToBatch`, BGR, `MAX_WIDTH = 3200`), `decode_steps` and `ctc_decode`
   (`CTCLabelDecode`, the score NumPy's pairwise `float32` mean), `batches` (added by the resumed
   session: the stable sort by width over height, cut by the manifest's `batch_size`) and
   `recognize`, returning `Recognized { text, score: f64 }` in the crops' order. `cv::warp`
   gained `warp_perspective_cubic` (`BicubicTab_i`, built and repaired as `initInterTab2D` does)
   over a position step it now shares with the bilinear warp, and `det::mini_box` became
   `pub(crate)` for the crop. **No dependency changed.**
3. **The batching question (P1 item 11) is answered by mirroring it.** A crop's input is padded
   to the widest of its batch-mates, and the model reads the padding, so `recognize` takes the
   pipeline's batches, not one crop at a time. `batches` reproduces every photo's batches from
   the crops' sizes alone (an always-run test over `rec_texts.json`).
4. **A fix in P2's `cv::rect`: OpenCV 4.10 does not choose its caliper by cosine.** One crop of
   2026-08-23-10 (box 25, a 90x30 label tilted 2°) came out a rounding off PaddleX's, at the
   right size. Its `minAreaRect` was an ulp off `cv2`'s (a corner's x 669.99994 against 670.0).
   OpenCV 4.10's `rotcalipers.cpp`, fetched from the 4.10.0 tag into scratch, picks the next
   caliper by turning each caliper's next edge into one frame (`rotate90CW`, `rotate180`,
   `rotate90CCW`) and keeping the one furthest clockwise by a cross product's sign
   (`firstVecIsRight`). The port had followed older OpenCV's largest cosine. On 30,000 random
   point sets (integer clouds, rounded rotated boxes, any four points) the old choice missed
   `cv2` on 112, and the new one is `cv2`'s to the bit on all 30,000: centre, size, angle and
   `boxPoints`. So P2 item 3's "exact ties" were this, and P4 item 5's 11 unclip boxes of 6,000
   very likely were too (not re-measured: the scratch that measured them is gone). The base
   vector's seed from the hull's orientation is no longer read, so only the orientation's
   assertion is left of it. `rect.rs` pins four of the missed cases, the label among them.
   **`tests/quad.rs`'s `CORNER_TOLERANCE_PX` is now 0.0**, not 0.01, because this was its stated
   reason, and the quad gate passes at zero on all 13, the HEIC's included.
5. **The constants, confirmed** (the manifest's "P5 confirms both") against the live pipeline:
   `text_rec_model`'s ops are `ReadImage(format="BGR")`, `OCRReisizeNormImg` (`[3, 48, 320]`,
   `max_imgW` 3200) and `ToBatch`; `batch_sampler.batch_size` is 6; `text_rec_score_thresh` is
   0.0; `CTCLabelDecode` holds 18,710 characters, `blank` first and the space last.
   `models.json` now says so.
6. **The reference and the result.** The reference is frozen Python's working image per photo,
   Python's boxes (`det_boxes.json`'s `sorted`), PaddleX's own crops sorted and batched as the
   pipeline does, and its rec model through the live predictor. Each batch was also run through
   the predictor's own preprocess, runner and decode, and the two agreed on every text and score.
   Rust gets the same boxes on its own working image:
   - on the twelve non-HEIC photos, **every crop pixel for pixel (560) and every batch's tensor
     bit for bit**, after item 4;
   - fed Paddle's own steps (argmax and probability per step), `decode_steps` gives **every text
     and score to the bit on all 607 boxes**;
   - through `ort`, **all 607 texts are Paddle's**, the HEIC's included. On the twelve the worst
     score is 2.3e-5 from Paddle's (2026-08-23-4), the worst step probability 3.4e-4, and no step's
     argmax moves;
   - on the HEIC, from ffmpeg's decode, no crop is PaddleX's, though sizes and batches are,
     because a size is its box's. Step probabilities move by up to 0.14, and five steps' argmax
     moves between the blank and the character beside it without changing a text. The worst
     score is 0.026 off ("Ball Speed").
7. **The bounds this phase sets.** The twelve: crops and tensors exact, texts exact, scores
   within **`SCORE_TOLERANCE = 3e-5`** (2.3e-5 rounded up). The HEIC gets a **guard**, as in P3
   and P4: texts exact and scores within **`HEIC_SCORE_GUARD = 0.03`**. The gate asserts that the
   fixture holds exactly one HEIC.
8. **Pins**: `tests/rec.rs` has 4 always-run tests:
   - the scene's 60 crops pixel for pixel (at least 10 of them turned);
   - its batches' tensors bit for bit, including a crop past the 3200 px cap and one at the
     model's own ratio;
   - 13 decode cases of real steps (spaces, doubled letters, `°`, padded batches, low scores),
     against a dictionary built from the fixture's 42 entries so the test needs no gitignored
     file;
   - every photo's batches from its crop sizes.

   The photo gate, `every_bay_photos_boxes_read_as_frozen_paddle_reads_them`, is ignored. It
   first checks the fixture's 42 entries against the real dictionary. Unit tests: `rec.rs` 6
   (widths, the pairwise sum, the decode, the first of equal maxima, the batch order), `cv::warp`
   2 for the bicubic table and border, `cv::rect` 1 for item 4. Fixtures: `rec_reference.json`
   (27 KB), `rec_texts.json` (115 KB).
9. **For P6**: the seam is `rec::crop` on each of `sort_quad_boxes(&detect(..))`, then textline
   orientation on those crops (in box order, batches of 6) and the 180° turn where it says so,
   then `rec::recognize`, which sorts and batches itself. A turn keeps a crop's size, so the
   batches do not move. Two of the pipeline's steps cannot fire here and need only mirroring. No
   crop is empty: a side that truncates to zero gets the source's size, as `warpPerspective`
   gives it. And `rec_score >= 0.0` keeps every reading, since a score is a mean of probabilities
   or 0. The reference was recorded without textline orientation, so no P5 gate would show a box
   P6 turns. The five-model pipeline crops from the UVDoc image (P4 item 9).
10. **For P7 and P36: recognition is slow on this box too.** In a release build one photo's
    batches took 61 s the first time in a process and 128–143 s each after, about 17 s for a
    batch of six at width 320. The 13 took 28 minutes. Detection showed the same climb (P4
    item 10). In the debug suite the rec photo gate took **1,999 s**, so `cargo test -p ocr --
    --include-ignored` now takes **68 minutes** (det 1,518 s, rec 1,999 s, quad 280 s, rectify
    276 s). P1 item 13's knobs (thread count, optimisation level) are still untried.
11. `data/processed` digest unchanged (`bd384cc4…`, 192 files). `git diff -- src/ scripts/ tests/
    spec/` is empty. `cargo test` (whole workspace), `cargo test -p ocr -- --include-ignored` (every
    gate, P2's quad gate at its new zero tolerance and P4's det gate after item 4's fix among them),
    `cargo clippy --all-targets` and `cargo fmt --check` pass, and so does
    `tests/test_docs_truth.py`. The scratch probes in `crates/ocr/examples/` (and the directory)
    and the session's scratch are deleted.

### P6 (2026-10-21) — cut off once, resumed and done

1. **Resumed, but from nothing.** `find -newermt '2026-10-18 14:55'` listed only this plan, whose
   marker the first attempt had set, and no scratch of that attempt survived. The phase was built
   whole in this session.
2. **Built**: `cls.rs` holds `cls_input` (the classifier's `ReadImage(format="RGB")`, so tensor
   channel 0 is **red** where det's and rec's are blue; `Resize` to 160x80 with OpenCV's bilinear,
   skipped for a crop already that size; `NormalizeImage`; CHW), `top_class` (`Topk(topk=1)`),
   `classify` (box order, batches of the manifest's 6), `Orientation` and `turn_half`.
   `recognizer.rs` holds `recognize(&mut Models, &RgbImage) -> Vec<screen::TextBox>`, re-exported
   as `ocr::recognize`: `detect`, `sort_quad_boxes`, `rec::crop`, `classify` and the turn,
   `rec::recognize`, the manifest's `score_thresh`, and `text_box`, `paddle.py`'s `_to_text_box`
   (the polygon's bounding rectangle in `f64`, the score as the confidence, an empty text dropped).
   `models.rs` gained `Normalize::folded`, which `det_input` now uses too (the same arithmetic,
   moved, and the det gate's tensors are still bit for bit). **The crate's first edge**:
   `screen = { path = "../screen" }`, for `TextBox`, with its reason in `Cargo.toml`;
   `ort_reach.rs` passes and the lock gained nothing but that edge. `models.json`'s textline
   "P6 confirms" now says what was confirmed.
3. **The constants, confirmed** against the live pipeline: `textline_orientation_model` is a
   `ClasRunnerPredictor` with `ReadImage(format="RGB")`, `Resize([160, 80])` at
   `cv2.INTER_LINEAR`, `Normalize` (the same fold as det's), `ToCHWImage`, `ToBatch` and
   `Topk(topk=1)`, batches of 6. **There is no threshold**, so the plan's "with PaddleX's threshold"
   names nothing: the likelier class wins. `argsort` keeps a tie in order, so an exact tie reads as
   upside down (NumPy 2.3.5, `[0.5, 0.5]` gives class 1).
4. **The pipeline's half turn is not a flip.** `rotate_image(crop, 180)` is a bicubic
   `warpAffine` about `(w / 2, h / 2)` into a black-bordered canvas of the same size, so output
   `(x, y)` is the source's `(w - x, h - y)`, one pixel past a flip, and row 0 and column 0 are
   black. `turn_half` is that closed form; in scratch it gave `rotate_image`'s bytes on 3,064 random
   images, every size from 1x1 to 8x8 among them, IPP on and off.
5. **No crop of the 13 is upside down**, in either pipeline: frozen `recognize`'s own
   `textline_orientation_angles` were 0 on every box of every photo, and so were the classes
   PaddleX's model gave its crops of `det_boxes.json`'s boxes. So the photo gate also classifies
   every crop turned upside down by a true flip. Paddle reads 208 of those 607 as upside down and the
   rest as upright (short numbers and symmetric text read either way), and the nearest to a tie is
   0.50008 against 0.49992 (2026-08-23-3, crop 31).
6. **The textline result** (`tests/cls.rs`, on frozen Python's boxes cropped from this crate's
   working image, as P5's gate is fed). The twelve: every batch's input tensor **bit for bit**,
   upright and flipped; **every class Paddle's**; probabilities within 3.6e-7 upright and 1.2e-5
   flipped, so **`PROBABILITY_TOLERANCE = 2e-5`**. The HEIC: every upright class Paddle's, with
   probabilities within 0.133, so a guard of **`HEIC_PROBABILITY_GUARD = 0.14`**; flipped, six
   crops take the other class, each where Paddle's own two probabilities were within 0.212 of each
   other, and **`HEIC_FLIPPED_CLASSES_MOVED = 6`** holds the count. The gate asserts one HEIC.
7. **The seam against D5's three models, the bound** (`tests/recognizer.rs`). The reference is
   frozen PaddleOCR built as `paddle.py` builds it plus `use_doc_orientation_classify=False,
   use_doc_unwarping=False`, normalised by `paddle.py`, on frozen Python's working image
   (`tests/fixtures/recognize_boxes.json`). Its detected polygons were `det_boxes.json`'s exactly on
   all 13. Against it, on the photos' own boxes this time (this crate's detection, so the whole
   seam): the twelve have the same boxes in the same order, **every text identical**, one box off,
   by 2 px (P4's 2026-08-23-11 box), and every confidence within 2.3e-5, so P4's and P5's bounds
   hold unchanged. The HEIC has every text identical and its edges within P4's 3 px guard, and its
   confidences move by up to 0.057 ("HD GOLF"), past P5's 0.03 guard, because here its boxes are
   this crate's too (P4: 16 of 47 a pixel or three off), so the crops differ in size as well as
   pixels: **`HEIC_SCORE_GUARD = 0.06`** in this gate. **The two readings parse identically on all
   13**: the same tiles located, the same text under each, the same values.
8. **Against the five-model reading: what UVDoc costs.** Frozen `recognize` as `paddle.py` runs it,
   re-run in scratch on frozen Python's working images, gave `spec/vectors/screen/corpus/`'s
   `input.boxes` **exactly on all 13**, so the gate reads them there. The document-orientation
   model's angle was 0 on every photo, so **every difference is UVDoc's**, and the three-model
   PaddleOCR shows each of them exactly as this crate does. UVDoc returns an image of the working
   image's size, but it moves the text in it and how the text splits into boxes: a box edge of a
   text both readings hold once lands up to 170 px apart.
   Box counts, this crate against five-model, in `det_boxes.json`'s photo order: 40/38, 45/42,
   47/43, 47/46, 50/47, 45/37, 47/45, 47/47, 49/47, 48/44, 46/44, 48/48, 48/48. Most extra boxes
   are the menu bar (`BACK`, `HELP`, `MAINMENU`, `SETTINGS`, `NOTIFICATIONS`, `PRINT`, `TIPS`) and
   pieces of the logo and the title; the gate prints every text on one side only. What the parser
   takes:
   - **the tiles located are identical on all 13** (`fields_present`);
   - **the values are identical on 12. 2026-08-10-1 differs**: the five-model reading has the
     `Impact Position V` label as `Impact Position`, so frozen's parse sees two labels tie and
     withholds both tiles, and the vector's expected shot has no `impact_position` and the tie
     warning. This crate reads the `V`, so the parse reads `impact_position = "CENTER"` and the V
     tile as blank. **The photo shows `CENTER` and `---`**, so this crate's read is the screen's
     and the committed vector's is the misread. The gate names it (`UVDOC_MOVES_VALUES`) and fails
     if it goes or another photo joins it;
   - **the text under a tile differs on seven more photos, and no value moves**: a degree sign read
     or not (2026-08-10-2 `8.6 Open` against `8.6° Open`; 2026-08-23-1 `3.1 Open` against
     `3.1° Open`; 2026-08-23-2 `3.4°` against `3.4` and `0.1 °L` against `0.1°L`; 2026-08-23-4
     `22.4` against `22.4°` and `3.9° Open` against `3.9 Open`; 2026-08-23-8 `1.3 Open` against
     `1.3° Open`; 2026-08-23-9 `17.5` against `17.5 o`), and on 2026-08-23-11 three units left
     outside their tiles (`188.0`, `178.2` and `114.8` without `yds`, `yds` and `mph`).
9. **For P7: D7's gate will fail on 2026-08-10-1, and UVDoc is the cause, but adding it is not a
   geometry fix.** D5 has P7 propose a UVDoc phase "if the OCR gate fails on geometry". Here the
   three-model reader reads a label the five-model one misreads, so adding UVDoc would restore the
   committed vector by restoring its misread, and nothing short of it can, because the value is
   right. Measure the whole read on all 13 first (D7's `needs_review` too, and whether the
   `raw_fields`, warning and confidence differences of item 8 move it), then stop and put it to the
   user: (a) declare 2026-08-10-1's `impact_position` as a difference the gate names, the screen's
   own reading; or (b) add UVDoc to reproduce the vectors, misread included. Item 8's raw-text
   differences reach `provenance.raw_fields` and are outside D7's list. `recognize` takes the image
   the vote hands it, so the vote calls it once per rotation tried.
10. **For P7 and P36: the climb did not show this session.** The recognizer gate (det, textline and
    rec on all 13 photos) took 253 s in a debug build, about 19 s a photo, where P4 item 10 and P5
    item 10 measured det and rec at minutes a photo, and the textline gate took 139 s. The code
    that runs the models has not changed since P5, so the difference is the box's state, and P1
    item 13's knobs are still untried. `cargo test -p ocr -- --include-ignored` took about
    1,040 s, against P5's 68 minutes: textline 136 s, det 184 s, quad 129 s, rec 207 s, recognizer
    254 s and rectify 127 s.
11. **P2's named gap is not reached.** No crop of the 13 is 320x160, an exact halving to the
    classifier's 160x80, and no recognition crop is 96 high with an even width up to 640, an exact
    halving to height 48. Either would send `resize_linear` to `resize_area`'s general path, which
    can differ from OpenCV's fast one at a half (`cls_input`'s doc says so).
12. **One place `recognizer.rs` is not `paddle.py` to the letter, on purpose**: `_from_dict_result`
    pairs the `i`-th kept text with the `i`-th *detected* polygon, which shifts texts onto their
    neighbours' boxes if the threshold ever drops a reading; here each reading keeps its own box.
    At the manifest's `score_thresh` of 0.0 nothing is dropped, so the two are the same list.
13. **Pins**: `tests/cls.rs` has 2 always-run tests, the scene's textline tensors in 11 batches bit
    for bit (its 60 crops and a 61st at exactly 160x80, which `Resize` skips) and `turn_half` on its
    60 crops, and the ignored photo gate. `tests/recognizer.rs` has the ignored seam gate. Unit tests:
    `cls.rs` 2 (the tie, the turn) and `recognizer.rs` 1 (the bounding rectangle, the empty text).
    Fixtures: `cls_reference.json` (119 KB) and `recognize_boxes.json` (69 KB).
14. `data/processed` digest unchanged (`bd384cc4…`, 192 files). `git diff -- src/ scripts/ tests/
    spec/` is empty. `cargo test` (whole workspace), `cargo test -p ocr -- --include-ignored`,
    `cargo clippy --all-targets` and `cargo fmt --check` pass, and so does
    `tests/test_docs_truth.py`. The session's scratch (the reference scripts and their dumps) is
    deleted.

### P7 (2026-10-21) — stopped on the one photo D7 fails, as P6 item 9 said it would; answered (a), resumed and done

1. **Built**: `crates/ocr/src/prepare.rs` holds `prepare_screen` (`working_image`, the uncropped
   note, then the vote) → `PreparedScreen { image, boxes, rotation, quad, label_ratio, notes }`
   (`rectified()` is Python's field), and `vote`, written against a recognizer closure as Python's
   is against `TextRecognizer`, so the early stop at `GOOD_ENOUGH_LABEL_RATIO` (0.6), the strict
   `>` that keeps the earlier turn on a tie, and both notes (`image was rotated N degrees -
   corrected`; `only {ratio:.0%} of the {device} labels ...` through `pyfmt::percent`) are
   unit-tested with no model loaded. `read_photo(&mut Models, path, &DeviceProfile)` is the
   crate's entry (decode, then `prepare_screen`), returning `ReadError::{Decode, Model}`, whose
   message is the cause's own, so P2 item 19's `UnsupportedJpeg` passes through unwrapped.
   `screen_timestamp(path)` is `importer.py::screen_timestamp`. All five are re-exported at the
   crate root. The entry returns the boxes and notes, not a shot: the shot is `screen::read` over
   a `ScreenInput` the caller stamps, so the lab (P23–P24) gives it the store's identity.
2. **`golf-ocr read <photo> [--json] [--device D] [--session S] [--min-confidence X] [--models
   DIR]`** (`src/bin/golf_ocr.rs`) prints the shot `screen::read` makes, or `null`, and writes
   nothing. It stamps as `import_screen` does (`shot_id` is the session and the photo digest's
   first twelve hex digits, the timestamp the mtime, `image_path` as given); the defaults are
   `config.py`'s `launch_monitor_profile` and `ocr_min_confidence` and `import_shot_screens.py`'s
   `screens-<today>` session; the models are found under the repository from the source, as
   `golf-core`'s data directories are. A refusal exits 1 with its cause's message, a bad command
   line 2. `tests/golf_ocr.rs` (2 always-run tests) holds both.
3. **Two edges and one constructor.** `crates/ocr` now depends on `contracts` (`Timestamp`) and
   `pyfmt` (`percent`, `round_half_even`), each already `screen`'s, so the lock gained the two
   names in `ocr`'s list and nothing else; `ort_reach.rs` passes. **Outside `crates/ocr`**:
   `crates/contracts/src/time.rs` gained `Timestamp::from_utc_micros`, because a `Timestamp` could
   be made only by parsing a string or reading the clock. No shape changed.
4. **`screen_timestamp` is a float's, not the file time's.** CPython makes `st_mtime` as `sec +
   nsec * 1e-9` and `fromtimestamp` rounds that double's fraction half to even, so a file time of
   `.828488500` stamps `.828489`, and `.999999900` the next second. The unit test holds seven such
   cases to CPython 3.13's answers. On the HEIC, `golf-ocr --json` stamped
   `2026-08-23T03:38:14.307160Z`, which is frozen `screen_timestamp`'s and the vector's
   `input.timestamp`.
5. **The gate** (`tests/read.rs`, ignored): every corpus vector's photo, found by its
   `input.image_path` and held to its `input.image_sha256` rather than looked up through the
   bundle's manifest (the vector names the file and its digest, and the digest is the proof), read
   by `read_photo` with nothing of frozen Python's, parsed by `screen::read` with the vector's own
   identity and threshold, and compared with `expected.shot`: D7's every value, `fields_present`
   and `needs_review`, plus the notes word for word (they carry the quad and the vote). It prints
   what D7 lets differ. One HEIC asserted. **327 s in a debug build**, each photo recognized once.
6. **The result: 12 of 13 pass. 2026-08-10-1 fails, on `impact_position` alone**: this crate reads
   `"CENTER"`, the vector `null`, exactly P6 item 9's prediction. On all 13:
   - **`fields_present` and `needs_review` are identical** (`needs_review` false everywhere), and
     so are the notes: the six uncropped notes on P2's six photos, no turn and no legibility note.
     **The vote stopped upright at its first reading on all 13.** The label ratio is the
     vector's on 12; on 2026-08-10-1 it is 14/15 against 13/15, the `V` label.
   - **every other value is identical**, the HEIC's included, so the HEIC's decode difference
     (P2 item 8, P3 items 5 and 8) does not reach a shot, and `decode.rs` needs no change;
   - what D7 lets differ: `parse_confidence` on six photos (2026-08-10-1 0.929 against 0.906;
     2026-08-10-2 0.930/0.928, -23-1 0.858/0.859, -23-2 and -23-8 0.882/0.883, -23-9 0.882/0.880);
     `raw_fields`, exactly P6 item 8's degree signs and units, and 2026-08-10-1's two tiles read
     (`Impact Position` `CENTER`, `Impact Position V` blank) where the vector has neither; and the
     warnings, where 2026-08-10-1's two tie lines become `Impact Position V: no value text under
     the label`, and the HEIC's two tie lines name `'Impact Position'` twice rather than
     `'Impact Position'` and `'ImpactPosition'` (the same tie, both tiles withheld either way).
7. **The question for the user**, as P6 item 9 put it, and nothing in the gate or the vector
   changed until it is answered:
   - **(a) Name the difference in the gate**: `tests/read.rs` declares `("2026-08-10-1",
     "impact_position", "CENTER")`, the screen's own reading, and fails if it goes or another
     joins it. The committed screen vector stays as it is (this plan keeps the screen family
     unchanged). **Recommended**: the photo shows `CENTER` and `---` (P6 item 8), so this is the
     reader being right.
   - **(b) Add UVDoc**: convert it (31 MB) with P1 item 16's toolchain and port its pre- and
     post-processing, a phase or more inserted before P8 (D5), to reproduce the vectors, misread
     included, and with it every UVDoc box P6 item 8 lists.
   **Answered by the user (2026-10-21): (a), name the difference in the gate.** `tests/read.rs`
   declares `("2026-08-10-1", "impact_position", "CENTER")`, the committed screen vector stays
   as it is, and no UVDoc phase is inserted. The resumed P7 does item 8's (a) branch, and item 9
   stands for P23 and P25.
8. **For P7, resumed**: on (a), add the declared difference to `tests/read.rs` (a named const
   beside D7's three, as `tests/recognizer.rs`'s `UVDOC_MOVES_VALUES` is), rewrite the gate doc's
   last paragraph (it says the gate fails today and why), and change `recognizer.rs` and
   `tests/recognizer.rs`'s "left to P7's gate" to the answer. Then `cargo test -p ocr --
   --include-ignored` must be green, and P7 ticks. On (b), the user approves the inserted phase
   and its row first.
9. **For P23 and P25, if (a)**: the re-read of photo `2e81ab05828e…` moves **two** values, not
   one. The stored shot (`data/processed/shots/2e81ab05….shot.json`, frozen Python's) has
   `shot_type` `CENTER SLIGHT FADE` and `impact_position` null, and this reader gives `SLIGHT
   FADE` (M34's label fix) and `CENTER`. D20's input gate must declare both on the three bundles
   sharing the photo (`2026-08-07-aaron1/1`, `2026-08-09/2`, `2026-08-10/1`).
10. **For P20 (P1 item 14 answered)**: the release `golf-ocr.exe`, copied alone into a scratch
    directory with no `DirectML.dll` beside it, read 2026-08-10-1's photo in **10.3 s**, opening
    and hashing the models included, and the HEIC as well. So a release binary on the CPU path
    needs nothing beside it, and `golf-lab` will not either.
11. **For P24**: `ocr::read_photo`, `ocr::screen_timestamp` and `Timestamp::from_utc_micros` are
    `import_screen`'s pixel half and stamp; `golf-ocr`'s identity rule is `import_screen`'s, and
    its digest is SHA-256 hex computed in the bin (storage's `hash_image` by another route, to
    spare `ocr` a `storage` edge). `golf-lab import-photos` should reuse the first two.
12. **Pins**: `prepare.rs` 6 unit tests (the vote five ways, the timestamp); `tests/golf_ocr.rs` 2;
    `tests/read.rs` the ignored gate.
13. `data/processed` digest unchanged (`bd384cc4…`, 192 files). `git diff -- src/ scripts/ tests/
    spec/` is empty. `cargo test` (whole workspace), `cargo clippy --all-targets` and `cargo fmt
    --check` pass, and so does `tests/test_docs_truth.py`. `cargo test -p ocr --
    --include-ignored` passes every gate but `tests/read.rs`, which fails on 2026-08-10-1's
    `impact_position` and nothing else (twice, the same answer). **Cargo stops at the first
    failing test binary**, so `rec`, `recognizer` and `rectify`, which sort after `read`, were run
    on their own and passed (207 s, 249 s and 123 s); with the gate red, add `--no-fail-fast`.
    The whole suite now takes about 22 minutes in a debug build, `read` 326 s of it. The
    session's scratch (the copied binary, the logs) is deleted.
14. **Resumed (2026-10-21, after the user's (a)).** What the cut-off session left was correct as
    items 1–13 describe it and was kept: `prepare.rs`, `lib.rs`'s re-exports, `golf_ocr.rs` and its
    test, `Cargo.toml`'s two edges, `Timestamp::from_utc_micros`. The resumed session did item 8's
    (a) branch and touched nothing else:
    - `tests/read.rs` gained `DeclaredDifference` (`shot`, `key`, and both sides as JSON
      literals) and `DECLARED_DIFFERENCES`, holding the one entry `("2026-08-10-1",
      "impact_position", "CENTER", null)` with the reason and the decision in its doc. A value
      that differs is excused only if a declared entry names the shot and key **and** both sides
      are exactly what it says; after the loop, an entry that was not seen fails as "out of date".
      So the gate fails if the difference goes, changes or gains a second. The gate's doc lost
      "it fails today ... P7 stops on it" for that rule.
    - The decision is written where the open question was: `tests/recognizer.rs`'s
      `UVDOC_MOVES_VALUES` doc ("left to P7's gate" becomes what P7 decided), `src/recognizer.rs`'s
      "How close it lands" (one sentence) and `src/prepare.rs`'s (the result in one sentence). The
      committed screen vector is untouched.
15. **Green.** `cargo test -p ocr --no-fail-fast -- --include-ignored` passes everything, `read`
    in 325 s, and the whole run took about 22 minutes again (`cls` 136 s, `det` 184 s, `quad`
    130 s, `rec` 206 s, `recognizer` 254 s, `rectify` 127 s). The gate cannot pass vacuously
    here: the out-of-date check means the declared value was read. `cargo test` (the whole
    workspace), `cargo clippy --all-targets` and `cargo fmt --check` pass. `data/processed`
    digest unchanged (`bd384cc4…`, 192 files). `git status -- src scripts tests spec` is empty.
16. **For P23, P24 and P25**: item 9 stands as written. The difference is now a committed
    statement in `crates/ocr/tests/read.rs`, so the lab's re-read of photo `2e81ab05828e…` should
    point at `DECLARED_DIFFERENCES` for `impact_position` rather than re-arguing it. `shot_type`'s
    `SLIGHT FADE` is M34's label fix, which the vector already holds, so it is not in that list.
    The input gate declares both against the **stored** shot (frozen Python's, which has neither
    change).

### P8 (2026-10-21) — the `mcp` family recorded from frozen Python

1. **What landed.** `build_mcp()` and its runners at the foot of `scripts/conformance_vectors.py`
   (the section's header comment is the encoding, and is what P9–P15 read first);
   `regenerate --mcp-once`, `mcp_vector_paths()` and `check`'s deferral line in
   `scripts/conformance.py`; eight pins at the foot of `tests/test_conformance.py`, plus
   `build_mcp` in `_refuse_to_build`'s list; one doc paragraph on `ORACLE_BY_FAMILY`
   (`crates/core/tests/rerecord.rs`). `_real_sessions_tree` gained an `analysis` transform
   (default `_slim_analysis`, so the storage family's recorder is unchanged). Nothing under
   `src/` changed. The section's runners: `_run_surface`, `_run_mcp`, `_mcp_server` (built as
   `run_mcp_server.py` builds it), `_mcp_answers`, `_mcp_clock`.
2. **The family**: 22 files, 1.8 MB on disk. `mcp/surface/server.json`; 20 plain
   `mcp/tools/*.json` holding 130 calls; `mcp/real/data.json.gz`, 141 KB gzipped (1.9 MB
   plain), 72 files and 75 calls. Every case `kind: "mcp"`, `oracle: "python"`,
   `career_version: 0`, and `provenance.analysis_version` 16 (the engine whose flight model and
   `is_outdated` answered, not a version the case ages on), with `mcp_version` 2.0.0. The record
   took 26 s and the synthetic build alone 13 s.
3. **The two shapes.** The surface case's `input.servers` is `{without_registry, with_registry}`,
   each a `roots` map, and its `expected` per shape is `{protocolVersion, serverInfo,
   instructions, tools: [{name, description, inputSchema}]}`. A tool case's `input` is `{roots:
   {sessions, shots, golfers|null}, files, calls: [{tool, arguments, now}]}`, and its `expected` is
   `{results: [{content, structuredContent, isError}]}`, as the wire keys them, with `null` where
   the wire leaves one out. `files` is `_materialise`'s encoding, the storage family's, with the
   three directories under one root. `golfers: null` builds the server with no registry. Answers
   come through `mcp.client.Client(server, mode="legacy", cache=None)`, so the protocol is the
   handshake one, negotiated as `2025-11-25`. `serverInfo` is `{name: "golf-coach", version:
   ""}`. 2026's `resultType` and `_meta` are dropped. **A tool's `outputSchema` is left out on
   purpose**: it is pydantic's schema for the views, and the tool cases pin what is returned.
4. **For P9–P15, what the answers look like.** Every `structuredContent` is `{"result": …}`,
   because every tool returns a union or a list. `content` has one text block per list item, none
   for an empty list (`content: []`), and one for a model. Every text block equals its item by
   value (240 of 240 checked across the family). Each is pydantic's `indent=2` JSON in model
   field order, with non-ASCII written raw (`—` in the flight caveats). The on-disk
   `structuredContent` has its keys sorted by `_write_json`, so compare as values, never as text
   order. Input schemas are pydantic's: `title` on the object and on every property
   (`list_sessionsArguments`, `Limit`), and `days` as `anyOf [integer, null]` with `default:
   null`. P9's normalization should drop the titles. No call errors (`_mcp_answers` refuses an
   `isError`, because its text is the SDK's), and no case calls a tool its server does not offer.
   Argument-validation errors are not recorded: that text is pydantic's.
5. **The surface**: 11 tools with a registry, 6 without, listed in this order: `list_sessions`,
   `get_swing`, `get_session_summary`, `get_recent_shots`, `get_shot_by_id`, `simulate_flight`,
   then `get_golfer_profile`, `get_shot_trends`, `compare_sessions`, `get_bag_profile`,
   `get_club_profile`. **For P15**: `get_strike_profile` is the twelfth and is not in the
   surface, so adding it is a declared difference to the `with_registry` tool list.
6. **For P9 (C10's flags)**: `run_mcp_server.py` always passes `--golfers-dir`, defaulting to
   `settings.golfers_dir`, so the launched server always offers the five per-golfer tools, even
   over a missing directory. `tools/no-directories` records exactly that, and every per-golfer
   tool there misses. The registry-less shape exists only as `build_server(golfers_dir=None)`,
   and the surface's `without_registry` plus five tool cases are built that way (`sessions`,
   `swing-views`, `session-summaries`, `shots`, `flight`). So `golf-mcp` needs a way to be
   started with no registry, or those cases are declared as a shape it does not serve.
7. **For P11, the shot-source edges in `tools/shots`.** `ShotStore.all()` sorts paths and then
   sorts newest-first stably, so equal timestamps keep file order. The composite's `recent`
   applies each source's `count` before it drops a repeated `shot_id`, so `count: 3` returns two
   blocks. `get_shot_by_id` scans `stream()`, which runs oldest first, so it finds the *older* of
   two shots sharing the id `a`, while `get_recent_shots` shows the newer. An unreadable
   `.shot.json` is skipped, and a file without that suffix is never opened.
8. **For P12, the clock.** Only `get_shot_trends` with `days` reads one (`career.py:345`). Three
   calls carry a `now` (two in `tools/career-window` and one in the real case). Every other call
   carries `null`, and the recorder fails a `null` call that reads the clock. `golf-mcp`'s trend
   must take `now` through a seam its gate can set. The trees are built under `_frozen_clock` at
   `2026-10-01T12:00:00Z`, so the golfer records and bag entries carry that stamp.
9. **C1: which cases carry flight numbers.** `provenance.flight_model_calls` lists, by index,
   every `simulate_flight` call that reached the model. That is 29 calls in three cases:
   `tools/flight` 8, `tools/flight-registry` 6, `real/data` 15 (every real swing has a read
   shot). Those numbers are the frozen flight model's, so a `v<N>` declaration that moves the
   model moves them. **For P15 and P19**: the career re-record P15 adds does not run on an
   `ANALYSIS_VERSION` bump. So the engine re-record must walk the `mcp` cases whose
   `flight_model_calls` is non-empty, or refuse a `v<N>` that moves them. P19 is where the
   engine walk next grows (`selection`), so it is the natural place, unless P15 builds one walk
   for both keys. `get_swing`'s `simulated` block is the stored artifact read back, so it moves
   with nothing. The career and club tools join `ranges.json`'s tour bands, as the career
   family's reports do, and share that family's exposure to a band change.
10. **The trees are the tests'.** `_McpKit` loads `tests/mcp/conftest.py` and the four test
    modules by path, for their writers, constants and shot builders. Each case's note names the
    tests it mirrors. Scenarios the tests build under one id each get their own session, and
    shots get their own `shot_id`, because the composite would drop a repeat.
    `tools/flight-registry` is the one scenario no test builds: `simulate_flight` with a bag
    loft and a handedness. It records a solved spin on the declared 7 iron, and `no_club_loft`
    for a driver with no loft, a club not in the bag, no club, and no golfer. **For P34**: the
    three build pins and the real one import `golf_coach.mcp` and `tests/mcp/`. If P34 moves or
    deletes either, those pins go with it, and the committed family stays.
11. **The freeze pin.** `test_the_committed_mcp_cases_are_what_frozen_python_answers` rebuilds
    the surface and the synthetic cases. It compares `input` exactly and `expected` through
    `frozen_view` and `compare_results`. So a path P9–P15 ledger through `golf-core rerecord`
    drops out of it, and a hand or Rust case, which the builder does not build, is never compared.
12. **For P29**: `test_the_real_mcp_case_answers_exactly_as_data_does` holds the snapshot to
    today's `data/`, as the storage family's real pin does, and never to the committed case. So
    P29's rewrite does not fail it, and the committed `real/data` stays a record of `data/` before
    P29. A Rust gate reads the snapshot inside the vector and needs no `data/`.
13. **`ORACLE_BY_FAMILY` and the family.** `mcp` writes its oracle into every file, so it is not,
    and cannot be, a row: the pin fails a family both claim. The oracle pin's walk now reports
    `"mcp": {"python": 22}` among 327 vectors, and the doc paragraph says why it is not listed.
14. **For P35**: CONFORMANCE §2 has no MCP section yet (item 3 is its content), and §4's
    "`regenerate` refuses the engine, stage, screen, storage and career families" gains
    `--mcp-once`. Docs are P35's, so neither was touched here.
15. **Green.** `cargo test` (the whole workspace, 1,373 passed), `cargo clippy --all-targets`
    and `cargo fmt --check` pass. `conformance.py check` exits 0 and prints the mcp line. pytest,
    run alone, passes 2,064 in 290 s, `tests/test_docs_truth.py` included, and `ruff check src
    tests scripts` is clean. `data/processed` digest unchanged (`bd384cc4…`,
    192 files). `git status -- src` is empty, and the ten other families are untouched. The
    session's scratch (the probes, the dry-build dumps, the logs) is deleted.

### P9 (2026-10-21) — `crates/mcp`'s skeleton: the surface conforms, every tool skipped by name

1. **What landed.** `crates/mcp` (package `golf-mcp`, lib `golf_mcp`, binary `golf-mcp`), the
   workspace's twelfth member: `src/textwrap.rs` (CPython 3.13's `textwrap.fill`), `caveats.rs`,
   `descriptions.rs`, `tools.rs` (the table), `server.rs` (the handler, the briefing, the caveats
   resource), `family.rs` (the tool cases' runner), `bin/golf_mcp.rs`; tests `surface.rs`,
   `tools.rs`, `edges.rs` (nothing depends on `golf-mcp`). Root `Cargo.toml` gained the member.
   Nothing under `src/`, `scripts/`, `tests/` or `spec/` changed in this phase.
2. **The dependencies, pinned and measured.** `rmcp = "=0.11.0"` (default features plus
   `transport-io`; `client` for the tests only), `tokio = "=1.54.0"` (`rt`, `macros`, `io-std`).
   `schemars` 1.2.3 arrives through `rmcp` and is used as `rmcp::schemars`, with no edge of its own.
   **0.11.0 is the newest `rmcp` that `rust-version = "1.87"` builds**, the same call P1 made for
   `ort`: 3.x declares 1.88; 0.13.0 through 2.2.0 declare nothing and use `if let … &&` chains,
   stable only from 1.88 (two, in `model/elicitation_schema.rs`); 0.12.0's `rmcp-macros` needs
   `darling` 0.23, which declares 1.88, and 0.12.0 will not build with `macros` off, because
   `model/capabilities.rs` imports `pastey` unconditionally. Moving the toolchain would have reached
   every crate's clippy, so it was not done. Cargo.lock gained 46 packages and lost none. The lock
   was resolved with `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback`. **For P20 and any phase
   adding a dependency**: resolve the same way, and build before pinning, because an undeclared
   1.88 crate only fails at compile time.
3. **The surface case passes, with three differences declared in `crates/mcp/tests/surface.rs`**,
   whose module doc is the declaration. The gate connects over a real handshake: an in-memory pipe
   with `rmcp`'s client asking for `2025-11-25`, as Python's did. The three differences:
   (1) `protocolVersion` comes back `2025-06-18`, the newest revision 0.11.0 models (set
   explicitly, `server::PROTOCOL`: `rmcp`'s own `LATEST` is 2025-03-26, and 2025-06-18 is the
   revision that brought `structuredContent`). (2) The `unscored` bullet of the briefing is reworded
   (C13). The gate holds the recorded briefing to `instructions_worded(_, Wording::Frozen)` *exactly*,
   which proves every other line of Python's is reproduced from Rust's registries by Rust's `fill`.
   It holds Rust's to the same text with that one bullet replaced. (3) `get_strike_profile` is a
   twelfth tool, last, on the registry shape. `serverInfo` is identical (`{name: "golf-coach",
   version: ""}`), and so are all eleven descriptions. **The vector was not written**: the
   declaration is in test code, because a ledger entry names a declaration file and this plan adds
   none (item 9).
4. **The input-schema normalization** (recorded in `surface.rs`'s module doc). It keeps the
   object's `type`, each property's name, its accepted JSON types as one set (`type` as a name or a
   list, an `anyOf` of types, and `nullable: true` all fold into it), `default` where stated,
   `required` as a set, and any `description`. It drops `title`, `format` and `$schema`.
   pydantic's `days` is `anyOf [integer, null], default: null`, and schemars' is `type: integer,
   nullable: true, default: null, format: int64`. Both normalize to `{types: [integer, null],
   default: null}`. **`description` is kept because the first run caught one**: schemars copies an
   argument struct's `///` doc into the schema a model reads. So `tools::args` uses plain `//`
   comments, says why, and a doc comment there now fails the gate.
5. **The caveats are Python's to the character, except the declared bullet.** All nine named
   sentences were read off the debug binary over stdio (initialize, `resources/read`) and compared
   with `golf_coach.contracts.caveats`. Eight are identical, and `READING_THIS_DATA_HONESTLY`
   differs by the bullet alone. The four short ones (`ONLY_CHECKPOINTS_ARE_JUDGED`,
   `PLACEMENTS_ARE_NOT_SCORES`, `PIVOTS_ARE_INTERIM`, `ALIGNMENT_CAVEAT`) are in no vector, so that
   probe is their only check. The photo-side rows are derived from `UNSCORED_REASONS`' own layout
   rule (the rows after `Unrecorded`), pinned to `[PrintedBlank, Misread]`. The reworded bullet
   puts `printed_blank` among the not-capture reasons ("neither the footage nor the photo of the
   screen is what went wrong … or photograph the screen again"), and says "`misread` is a photo of
   the screen worth retaking, sharper and square-on, and every other one is a clip worth taking
   again". `textwrap.rs` lives in `crates/mcp` rather than `pyfmt`, because every `pyfmt` edge is
   gated by a `format/` table and that family does not change in M29. Its tests are frozen
   Python's `fill` on twelve shapes the regex treats specially.
6. **The caveats resource (D10).** `golf-coach://caveats`, `application/json`: one object of nine
   strings keyed by the Python constant names. Its keys are sorted, because `serde_json` has no
   `preserve_order` here. `ALIGNMENT_CAVEAT` keeps its `{summary}`. **For P16–P17**: read it with
   `resources/read` and index by name. The briefing holds `TWO_AXES`, `READING_THIS_DATA_HONESTLY`,
   `READING_A_SIMULATED_FLIGHT` and, with a registry, the two per-golfer blocks; the short four are
   only in the resource.
7. **For P10–P15, how a body lands.** In `tools::TOOLS`, set the row's `body: Some(f)`, where
   `f: fn(&GolfMcp, &JsonObject, &Clock) -> Result<CallToolResult, ErrorData>`. `GolfMcp::call`
   has already refused an unknown tool or arguments the schema refuses (`invalid_params`), by
   parsing into `tools::args`. Those structs carry `#[allow(dead_code)]` until the bodies read their
   fields: **for P15, remove it**. Build each answer by hand. One text block per list item, one
   for a model, none for an empty list, each pydantic's `indent=2` JSON (P8's item 4).
   `structuredContent` is `{"result": …}`, and `isError` is `false`, which every one of the 130
   synthetic answers records and which `CallToolResult::success` writes. `CallToolResult::structured`
   writes one compact block, which is not that shape. `tests/tools.rs` walks 205 calls in 21 cases
   and now skips every one: P10 74, P11 29, P12 29, P13 35, P14 38. Its comparator (`departures`)
   is unexercised until P10, which may refine it.
8. **For P12, the clock.** `server::Clock` is the seam P8's item 8 asks for: `System` live,
   `At(now)` for a recorded call with a `now`, and `Unset` for one without. A body that reads an
   `Unset` clock must refuse, as the recorder does.
9. **For P15, two things the plan leaves in tension.** (a) `golf-mcp` depends on `golf-core`
   (`reports::data_dir` now, `run_profile` at P15), and `tests/edges.rs` pins that nothing depends
   on `golf-mcp`. So `golf-core rerecord` cannot call `golf_mcp::family`. The `mcp` walk must
   either spawn the `golf-mcp` binary, or live in `golf-mcp` over `golf_core::rerecord`'s public
   `gate`, `apply` and `ledger`. (b) Every `provenance.rerecords` entry names its declaration file,
   the rules add none, and `career-v3.json` names no `mcp` path. Yet P15's done line wants the
   `career-v3` dry run at "0 changed with `mcp` walked", while the surface differs from Rust's in
   item 3's three places. Either the walk applies the same code-side declarations the gates do, or
   those differences are ledgered, which needs a declaration naming them. Settle which before
   building the walk, and ask if neither fits the rules. Also `descriptions::GET_STRIKE_PROFILE` is
   a first draft and its schema is `{player}` alone. Both are P15's to rewrite.
10. **For P34**: `READING_A_BAG`'s mishit bullet names `flag_mishit.py --clear`, in both copies,
    and P34 deletes that script. Rust's sentence should then name `golf-core flag-mishit --clear`,
    a fourth declared difference in `surface.rs`, in the with-registry briefing only.
11. **For P35**: `CLAUDE.md`'s crate count is twelve with `mcp`. ADR-035's addendum, or ADR-006's,
    should name the `rmcp` 0.11.0 pin, why, and the 2025-06-18 revision. CONFORMANCE's MCP section
    gains item 3's declared differences and item 4's normalization. The plan's P9 Read line named
    `analysis/flight_caveats.py` for `READING_A_SIMULATED_FLIGHT`, but that block is in
    `contracts/caveats.py` and only `mcp/server.py` uses it. `flight_caveats.py` is P14's per-flight
    caveats.
12. **Green.** `cargo test` (the whole workspace): 1,390 passed, 0 failed, 9 ignored.
    `cargo clippy --all-targets` is clean, and so is `cargo fmt --check`. `pytest
    tests/test_docs_truth.py`: 92 passed. The debug `golf-mcp` over stdio answered initialize,
    listed twelve tools, answered a stub call `not ported until M29 P10` and exited 0 when stdin
    closed. `data/processed` digest unchanged (`bd384cc4…`, 192 files). No `spec/` file is
    written, no declaration added, and `git status -- src scripts tests` shows only P8's three. The
    session's scratch is deleted.

### P10 (2026-10-21) — the swing and session tools: 74 calls conform, exactly

1. **What landed.** `crates/mcp/src/query.rs` ports the sessions-and-swings half of
   `mcp/query.py`. It holds the twelve views in pydantic's field order, `missing_swing`,
   `missing_session`, `list_sessions`, `get_swing`, `get_session_summary` and `shot_view`, plus
   frozen Python's fifteen metric fields as `NUMERIC_METRICS` (13) and `TEXT_METRICS` (2).
   `crates/mcp/src/answer.rs` adds `model` and `list` (a view on the wire, per P8's item 4) and
   `Ordered`, an insertion-ordered map for the dicts Python builds by insertion. `tools.rs` gains
   the three bodies, `arguments_as`, and a sixth `tool!` arm that takes the body.
   `storage::state` gains `resolve_placements` and `resolve_pivots` (typed `ResolvedPlacement` and
   `ResolvedPivot` rows) beside `load_analysis`, and its module doc no longer says they stayed in
   Python. `golf-mcp` gained path edges on `storage` and `pyfmt`. Both are workspace crates, so no
   external dependency was added and only golf-mcp's entry in Cargo.lock changed. Nothing under
   `src/`, `scripts/`, `tests/` or `spec/` changed.
2. **The gate: 74 of 205 calls compared, none departing.** Still skipped by name: P11 29, P12 29,
   P13 35, P14 38. A stricter throwaway probe (deleted) measured two more things. All 74
   `structuredContent`s are *exactly* equal as `serde_json::Value`, so integers stay integers and
   floats stay floats. All 74 answers' text blocks are **byte-identical** to the recorded ones,
   field order and indentation included. So `serde_json::to_string_pretty` over a struct in field
   order *is* pydantic's `model_dump_json(indent=2)` on everything these views hold. No float in
   the 74 takes the exponent form, the one place the two writers spell a number differently. The
   comparator (`departures`) needed no change.
3. **Why the resolvers went into `storage` and not `golf-mcp`.** Python keeps them beside
   `load_analysis` so that one definition serves every reader of a stored swing (the MCP view, the
   results page, the coaching brief). The Rust port keeps them there for the same reason, so a
   later Rust reader (M38's results view) reuses them rather than copying them. They return typed
   rows, and `query.rs` wraps those in its own view order, as `PlacementView(**row)` does.
4. **Where Rust's readers differ from Python's on purpose. No recorded case reaches any of
   these**, and `query.rs`'s module doc lists them:
   - **Parsing.** Stored files go through `serde_json`, not `json.loads`, so a `NaN` makes an
     artifact unreadable.
   - **Stored shots.** These are checked by Rust's `ShotData`, while Python's goes through
     pydantic. pydantic's lax mode reads `"90.5"` as a float; `ShotData` does not.
   - **Unscored reasons.** The two photo-side reasons (`printed_blank`, `misread`) read as
     themselves, where Python's closed enum reads them as `unrecorded`. C6 keeps both out of every
     `analysis.json`.
   - **Containers.** `str()` of an object puts its keys in sorted order.

   One pydantic behaviour was measured (2.13.4) and copied: a bool `population_n` becomes 1 or 0.
5. **Python quirks kept as they are, recorded or not:**
   - **An empty `analysis.json` object.** `_summarize` reads `{}` as "not analyzed", but
     `_status_of` reads it as "done" (one tests truthiness, the other `is None`).
   - **The swing directory.** The listing joins it from `manifest.swing_id`, not from the directory
     the manifest was found in. `get_session_summary` re-reads every swing by that id.
   - **The listing headline.** It is the state sidecar's, unfiltered.
   - **Session averages.** They use the compensated `pyfmt::sum`, then `round_to(…, 1)`. That
     combination is what decides the session-summaries case's half-even mean.
   - **Best and worst.** `max` and `min` keep the first extreme.
6. **For P11: the shot view is shared with `get_swing`.** `query::shot_view` and the two metric
   lists fill `SwingView.shot` as well as P11's two tools. Its metrics also feed
   `get_session_summary`'s `shot_averages` and `shots_needing_review`.
   - **What moves when P11 changes them.** Adding M32's seven fields, or the golfer-facing filter on
     `warnings`, also moves recorded `get_swing` and `get_session_summary` answers wherever an
     embedded shot carries them. That covers `swing-views`' swing 10 and the
     `session-summaries` shots, plus the real case's 15 `get_swing` calls and 15 summaries. Every
     shot in the real case has a read photo and warnings to filter.
   - **Declare them too.** Those paths need declaring, or hand cases, alongside P11's own. The
     gate in `tests/tools.rs` has no declaration mechanism yet. P11 adds one, by path, in test
     code, as `tests/surface.rs`'s module doc declares the surface's three differences (P9's
     item 3).
   - **Where the fields go.** `impact_position_v` is a text read-out (`TEXT_METRICS`); the other
     six are numeric. The unit test `the_metric_list_is_frozen_pythons_and_names_real_fields`
     names those seven as the fields left over, and P11 updates it. `missing_shot` is not written
     yet; `query::missing` is the helper it goes through.
7. **For P12–P14, how a body lands now.**
   - **Arguments:** `arguments_as::<args::…>(arguments)?`.
   - **Reader:** one `query`-style function in a module named for its Python twin (`career`,
     `club`, `flight`).
   - **Answer:** `answer::model` for a `View | NotFound` union, `answer::list` for a list.
   - **Wiring:** `Some(body)` as the row's sixth `tool!` argument.
   - **Maps:** a Python dict built from `sorted(…)` is a `BTreeMap`; one built by insertion is an
     `answer::Ordered`.

   `tools::args` keeps its `#[allow(dead_code)]` until P15.
8. **The disk filled mid-phase.** The first full `cargo test` failed to link (`LNK1318`, then
   `no space on device`) with C: at 0 bytes free. `target/` holds 29 GB of the 931 GB disk, so
   almost all the rest is outside this repo. The phase removed `target/debug/incremental` (8.9 GB,
   which cargo rebuilds) and ran the suite with `CARGO_INCREMENTAL=0`; 6.4 GB was free afterwards.
   **For P11 onwards**: if a build fails to link or reports no space, check `df -h /c` before
   suspecting the code. Building with `CARGO_INCREMENTAL=0` keeps the cache from growing back.
9. **Green.**
   - **Rust:** `cargo test` (the whole workspace): 1,399 passed, 0 failed, 9 ignored, nine of the
     passes new: two in `answer`, six in `query`, one in `storage::state`. `cargo clippy
     --all-targets` and `cargo fmt --check` are clean.
   - **Docs:** `pytest tests/test_docs_truth.py` passes.
   - **Files and data:** the `data/processed` digest is unchanged (`bd384cc4…`, 192 files). No
     `spec/` file was written; `git status -- src scripts tests` shows only P8's three, and the
     ten other families are untouched.
   - **Scratch:** the probe and the test log are deleted.

### P11 (2026-10-21) — the shot tools: 29 calls conform, every departure declared by path

1. **What landed.** In `crates/mcp`:
   - `src/source.rs` (new): `ShotSource`, the one shot source `run_mcp_server.py` builds
     (`CompositeShotDataSource([ScreenShotDataSource(shots_dir)])`) over `storage::shot_store`.
     It has `recent(count)` and `stream()`, in the composite's two orders (P8's item 7).
   - `query.rs`: `missing_shot`, `recent_shots`, `get_shot`, M32's seven fields in the metric
     lists, and the golfer-facing filter in `shot_view`.
   - `tools.rs`: the two bodies.
   - `family.rs`: `Declared`, `DECLARED` and `as_declared`, the declaration mechanism P10's item 6
     asked for.
   - `tests/tools.rs`: applies the declarations and reads `hand/`.
   - `Cargo.toml`: one path edge, on `screen` (for `golfer_warnings`). It is a workspace crate, so
     no external dependency was added. `screen` has no `ort`, so `golf-mcp` still has none in its
     closure.

   One vector was added: `spec/vectors/mcp/hand/shot-view.json` (`oracle: "hand"`, `career_version`
   3, LF, indent 2). Nothing under `src/`, `scripts/` or `tests/` changed, and nothing in the
   recorded `mcp` cases did either.
2. **The gate: 108 of 210 calls compared, none departing.** That is P10's 74, P11's 29 and the
   hand case's 5. Still skipped by name: P12 29, P13 35, P14 38.

   **Departures from the recorded answers: one rule at three paths.** Each is declared in
   `family::DECLARED`, and the gate prints how many recorded answers each one moved:
   - `get_recent_shots` `warnings`: 1;
   - `get_shot_by_id` `warnings`: 13;
   - `get_swing` `shot.warnings`: 15.

   All 29 are in `real/data`, the only case whose shots carry a `no tile found for` line: all 13
   stored shots have `'Bounce & Roll'`. P10's item 6 predicted that `session-summaries` and
   `swing-views`' swing 10 would move too. They did not, because their shots carry no layout line.
   `get_session_summary` answers no warnings, so the filter cannot reach it.
3. **How a declaration works.** `as_declared(tool, recorded)` rewrites the recorded answer, both
   `structuredContent.result` (each element, for a list) and each `content` block's parsed text,
   by the declared rule at the declared path. The gate then compares as before. **So a declared
   path is held to what the rule makes of the recorded value, not skipped.**
   - **Stale declarations fail.** A declaration that moves no recorded answer fails the gate, and
     a missing or `null` step (a miss, a swing with no shot) leaves that item as recorded.
   - **Checked by mutation.** With the filter taken out of `shot_view`, the gate failed on exactly
     the 29, plus three hand calls. The mutation was reverted.
   - **Why the list is in the library.** It lives in `family.rs`, not in test code as P10's item 6
     suggested, because P15's walk must apply the same list (P9's item 9b).
4. **M32's seven fields moved no recorded answer.** Not one stored shot, synthetic or real,
   carries any of them, because frozen Python wrote them all. The hand case is what reaches them,
   and it was worked before the code ran against it.
   - **The list.** It is now `NUMERIC_METRICS` (19) and `TEXT_METRICS` (3). Python's fifteen keep
     their order, M32's six numbers follow `apex_height`, and `impact_position_v` follows
     `impact_position`. The unit test is now the exhaustive split `test_query.py` makes: the
     leftovers are exactly the five non-metric fields, and Python's fifteen lead the lists, pinned
     by name.
   - **What the hand case covers.** `get_recent_shots` newest first, `get_shot_by_id` found, and a
     miss on a photo hash, which is not a shot id. A shot whose only warning is the layout line
     empties, and the tie and blank-tile lines are kept in order. `get_swing` embeds the view.
     `get_session_summary` averages M32's numbers: `attack_angle` −3.5, `dynamic_loft` 24.5,
     `carry_offline` −6.0, each exact before rounding.
5. **Not applied: `capability::analysed_view`.** It blanks the readings a device prints only to be
   shown (`club_head_speed` and `smash_factor` on every stored HD Golf shot), which is what grading
   may read. The view reports what the screen said, which the phone shows too (ADR-034 §2). Applying
   it would have moved every real shot's metrics, for no reason the plan gives. `query.rs`'s module
   doc says so.
6. **The source's edges, ported as recorded.** All of `tools/shots`' edges reproduce:
   - `recent` applies the count before it drops a repeated id;
   - `stream` is the newest-first list reversed, so two shots at one instant stream in reverse file
     order, and the older of two sharing an id wins;
   - a count of zero or less answers nothing;
   - unreadable and non-`.shot.json` files are skipped.

   One edge no case reaches, inherited from M36's `ShotStore`: Python on Windows globs and sorts
   names case-insensitively, while Rust matches the suffix exactly and sorts by bytes. Every stored
   name is a lowercase hash or id.
7. **For P14**:
   - **The shot lookup.** `flight.py::_shot_for` scans `stream()` for a photo hash, so use
     `source::ShotSource::stream()` and do not write a second scan.
   - **The declarations do not reach it.** `FlightView` carries no shot warnings.
   - **Already passing.** The flight cases' three `get_swing` calls and one `get_recent_shots`
     call pass.
8. **For P15**:
   - **The walk.** The career re-record's walk of `mcp` should apply `family::DECLARED` through
     `as_declared`, the first of P9 item 9b's two options. The second, a declaration file, is not
     needed for P11's three.
   - **The oracle check.** `tests/tools.rs` asserts each case's oracle is `python` or `hand`.
     P15's Rust-recorded cases must extend it, and no declaration may be applied to them.
   - **The oracle pin.** It now counts `mcp: {python: 22, hand: 1}`.
   - **`check`'s wording.** The deferral line says "23 mcp vectors … recorded once from frozen
     Python", and one is now hand-worked. P15 runs pytest and adds Rust cases, so the rewording
     (sanctioned: "check's deferral of them") belongs there, or in P35. P11 left frozen Python
     untouched.
9. **For P16–P17**: `coach.py`'s JSON entry reads its shot through `get_swing`'s view. That view
   now carries M32's fields (once a stored shot has any) and the filtered warnings, so the brief
   never sees the layout line.
10. **For P35**: CONFORMANCE's MCP section gains the `hand/` sub-family and the declared-path
    mechanism (item 3), with its three paths and the rule.
11. **Green.**
    - **Rust:** `cargo test` (the whole workspace, `CARGO_INCREMENTAL=0`): 1,402 passed, 0
      failed, 9 ignored. Three of the passes are new, in `source`, `family` and `query`.
      `cargo clippy --all-targets` and `cargo fmt --check` are clean. C: had 6.3 GB free, and
      nothing was deleted.
    - **Python, run alone:** `pytest tests/test_conformance.py` passes 263 with the hand case
      committed, and `conformance.py check` exits 0. `pytest tests/test_docs_truth.py` passes 92.
    - **Files and data:** the `data/processed` digest is unchanged (`bd384cc4…`, 192 files), and
      so are the ten other families. `git status -- src scripts tests` shows only P8's three.
    - **Scratch:** the hand case's writer script and the test log are deleted.

### P12 (2026-10-21) — the career tools: 29 calls conform, and Rust's corpus reached by hand

1. **What landed.** In `crates/mcp`:
   - `src/career.rs` (new), the port of `mcp/career.py`: the six views in pydantic's field order,
     the three golfer misses, `resolve_golfer`, `golfer_profile`, `shot_trends`,
     `compare_sessions`, and `versions()`, the engine pair every corpus here is read under.
   - `tools.rs`: the three bodies, and `registry()`, which answers the golfer directory.
   - `query.rs`: `missing` is now `pub(crate)`, as `career.py` reaches for `query._missing`.
   - `family.rs`: a fourth `DECLARED` row (item 4).
   - `Cargo.toml`: one path edge, on `analysis`, for the three builders. It is a workspace crate,
     so no external dependency was added, and `ort` is still outside `golf-mcp`'s closure.

   One vector was added: `spec/vectors/mcp/hand/career-corpus.json` (`oracle: "hand"`,
   `career_version` 3, LF, indent 2, keys sorted). Nothing under `src/`, `scripts/` or `tests/`
   changed, and nothing in the recorded `mcp` cases did either.
2. **The gate: 143 of 216 calls compared, none departing.** That is P11's 108, P12's 29 recorded
   calls and the new hand case's 6. Still skipped by name: P13 35, P14 38.
   - **Exact, not just within tolerance.** A throwaway probe (deleted) held the 29 recorded
     answers and the 6 hand ones to exact `serde_json::Value` equality and byte-identical text
     blocks. Rust matches on 34 of the 35. The 35th is the recorded note that item 4 declares.
   - **Versions.** Every corpus is read under `{ANALYSIS_VERSION 17, COMPARABLE_FROM 14}`, as
     `golf-core`'s career verbs read theirs.
3. **Where Rust's corpus differs on purpose (the done line).** No recorded case reaches any of
   ADR-024's three departures. Every recorded tree was written by frozen Python: each `analysis.json`
   is at engine 16, and no tree holds a photo entry or an intent. So no recorded count moved.
   - **The hand case.** `hand/career-corpus.json` reaches all three through the three tools in 6
     calls. It holds an engine-15 swing that pools, an engine-13 swing outdated under both rules,
     a photo entry that is admitted, and a drill that is untracked. Its note works every count.
   - **Mutation check.** With `comparable_from` set to 16, Python's rule, the gate failed on
     exactly four hand calls (the profile, the head-sway trend, both comparisons), and no recorded
     call failed. The mutation was reverted.
4. **Two of Python's sentences would deny the drill, so both now name it.** Each sentence lists
   every reason Python's corpus had for a missing sample. Under Rust's corpus, a drill is a new
   reason the sentence would deny. This is a judgement the plan did not spell out; it is reversible,
   and `career.rs`'s module doc says why.
   - **`get_shot_trends`' "no sample" note.** It ends on `stale, was analyzed by an older engine,
     or was hit as a drill, which counts toward no stat.` (`career::DRILL_IN_THE_NOTE`). This moves
     one recorded answer, `tools/career-refusing[4]` (`invented_metric`). It is declared in
     `family::DECLARED` at `get_shot_trends` `note`, with the rule `drill_in_the_note`.
   - **`compare_sessions`' sampleless sentence.** The case is a session whose analyzed swings
     contribute nothing. When the corpus names any swing there `untracked`, the sentence says how
     many were hit as drills, a count that is always true. Otherwise it is Python's re-upload
     sentence, which `tools/career-reuploads` records. Only the hand case reaches the drill branch.
   - **Not changed: two cases where Python's re-upload sentence was already wrong.** It is also
     false for a session whose analyzed swings are another golfer's, or all outdated. That is
     frozen Python's own flaw, not Rust's corpus departing, so it is kept. The summary scores every
     golfer's swings while the corpus counts only this golfer's.
5. **The clock (P8 item 8, P9 item 8).** `shot_trends` reads the clock only after the golfer
   resolves, and only with `days`, as Python's does.
   - **The call is recorded with no clock (`Clock::Unset`).** Answered with a protocol error
     (`WindowRefused::NoClock`), because the recorder refused such a call.
   - **The window starts outside years 1 to 9999.** CPython raises `OverflowError` there, so this
     is an error result naming the days. Checked arithmetic means `i64::MAX` days cannot wrap.
   - **Unit test.** `the_window_is_read_off_the_clock_and_refused_outside_datetimes_range` pins
     both edges: 739,889 days back from 2026-10-01 reaches `datetime.min`, and one day more is
     refused.
6. **One reading worth knowing, kept as frozen Python's.** `compare_sessions`' `swings_a`,
   `swings_b` and the mean scores are `get_session_summary`'s, which scores only `analysis.json`.
   So a photo-only session answers `swings_b: 0` and `mean_score_b: null` beside a carry `n_b` of
   1. That is not false, since "swings" means analyzed swings, but M38's results view may want to
   say it.
7. **For P13**:
   - **Golfer lookup.** `club.py` resolves its golfer through `career.resolve_golfer`; use
     `career::resolve_golfer` and `tools::registry`, and do not write a second lookup.
   - **Corpus reads.** Read through `career::versions()`, so the club tools read the corpus the
     career tools do.
   - **Where to land it.** The `analysis` edge is already in `golf-mcp`'s manifest, for
     `club_profile`.
8. **For P15**:
   - **The walk.** `family::DECLARED` now has four rows, and the walk applies all four.
   - **The oracle pin.** It counts `mcp: {python: 22, hand: 2}`.
   - **`check`'s wording.** The deferral line says "24 mcp vectors … recorded once from frozen
     Python", and two are hand-worked now. P11's item 8 left the rewording for P15 or P35.
9. **For P35**:
   - **CONFORMANCE's MCP section** gains `hand/career-corpus` and the fourth declared path.
   - **ADR-024's M35 addendum** (drills untracked) could say that the MCP career tools' two
     sentences now name the drill (item 4).
10. **For every later phase: a Git Bash heredoc dropped `\` + newline.** I edited three Rust files
    with Python run from a quoted (`<<'EOF'`) heredoc. Each Rust string continuation, `\` at a
    line's end, came out joined with its indentation kept inside the string. `cargo build` does
    not catch this; only the gate's failure on the hand case did. Write Rust string continuations
    with the Edit or Write tool, never through a heredoc.
11. **Green.**
    - **Rust:** `cargo test` (the whole workspace, `CARGO_INCREMENTAL=0`): 1,404 passed, 0
      failed, 9 ignored. Two of the passes are new, both in `career`. `cargo clippy
      --all-targets` and `cargo fmt --check` are clean. C: had 5.5 GB free afterwards, and
      nothing was deleted.
    - **Python, run alone:** `pytest tests/test_conformance.py tests/test_docs_truth.py` passes
      355, and `conformance.py check` exits 0.
    - **Files and data:** the `data/processed` digest is unchanged (`bd384cc4…`, 192 files), and
      so are the ten other families. `git status -- src scripts tests` shows only P8's three.
    - **Scratch:** the hand case's writer script, the listing script and the test log are
      deleted.

### P13 (2026-10-21) — the club tools: 35 calls conform exactly, nothing declared

1. **What landed.** In `crates/mcp`:
   - `src/club.rs` (new), the port of `mcp/club.py`. It holds the three views in pydantic's field
     order (`ClubMetric`, `ClubView`, `BagView`), the five silences and the untagged sentence, the
     three misses (`missing_golfer_bag`, `missing_golfer_club`, `unknown_club`), `bag_profile` and
     `club_profile`.
     - **One constant is a function.** Python's `NOT_ENOUGH_ON_THIS_CLUB` concatenates
       `career.THE_UNBLOCK`, and a Rust `const` cannot concatenate another `const`. So it is
       `not_enough_on_this_club()`, which writes `NOT_ENOUGH_HEAD`, a space, then
       `career::THE_UNBLOCK`, and there is no second copy of the unblock sentence.
     - **`club_profile` answers `Option<Result<ClubView, NotFound>>`.** `None` is an unknown golfer,
       whose miss the body builds, and `Err` is text that is not a club.
   - `tools.rs`: the two bodies.
   - `career.rs`: `points`, `refusal`, `low` and `high` are now `pub(crate)`, as `club.py` reaches
     for `career._points` and the rest.
   - `lib.rs`: the module, and its line in the crate doc.

   **No new edge.** `analysis`, `storage` and `contracts` were already in the manifest, and
   Cargo.lock did not change. No vector was added, and nothing under `src/`, `scripts/`, `tests/`
   or `spec/` changed.
2. **The gate: 178 of 216 calls compared, none departing.** That is P12's 143 plus P13's 35:
   27 synthetic calls in 13 cases and 8 in `real/data`. Still skipped by name: P14 38
   (`simulate_flight`). The four declared paths moved what they moved before (1, 13, 15, 1).
   - **Exact.** A throwaway probe (deleted) held all 35 answers to exact `serde_json::Value`
     equality and byte-identical text blocks. Both hold, so `BagEntry`'s `#[serde(flatten)]` spec
     writes pydantic's key order, and `34.0` stays `34.0`.
   - **Mutation check.** I put `metrics.is_empty()` ahead of `n_swings == 0` in `club_note`. The
     gate then failed on exactly 8 recorded calls: `club-declared` [0] and [1], and `real/data`
     [67], [69], [70], [72], [73], [74], where a declared, unhit club became `NO_MEASUREMENTS`. The
     mutation was reverted.
3. **No declaration and no hand case, and why.** No recorded club answer reaches any of Rust's
   corpus departures. Every recorded tree is frozen Python's, at engine 16, with no photo entry
   and no intent (P12's item 3).
   - **The done line.** It names only the recorded cases.
   - **Where the departures are gated.** They reach these tools only through `read_corpus` and
     `build_bag_profile`. The career family gates both over Rust's corpus, photo entries and drills
     included. ADR-024's M35 addendum names the two pins: every aggregate and report over a corpus
     with a drill equals the one with the drill deleted.
   - **What P12's case covers.** P12's hand case already reaches the corpus through the MCP.
4. **A judgement: a drill reads as deleted here, and no club sentence was reworded for it.** It is
   reversible, and `club.rs`'s module doc says why.
   - **The sentences that could deny a drill.** Under Rust's corpus, a club hit only in drills has
     no row. So `NEVER_HIT` ("No swing on record was hit with this club"), `IN_BAG_NEVER_HIT` and
     `NOTHING_TAGGED` can each deny a swing that exists as a drill.
   - **Why they were kept.** ADR-024's M35 addendum rules that a drill is as if deleted from disk
     for every report, and `golf-core club-profile` already prints such a club as "never hit and
     not in the bag". P12's two rewordings were a different case: each of those sentences listed
     every reason a sample can be missing, so leaving the drill out made the list false.
   - **What a per-club drill sentence would need.** It would need the club of an excluded swing,
     and `ExcludedSwing` carries `session_id`, `swing_id`, `reason` and `detail` only. Adding the
     club moves the career family under a `career-v4` declaration, which this plan forbids.
   - **For P35 and M38:** if the user wants a club tool to say "hit only as drills", it is that
     contract change, not a sentence edit.
5. **For P15: one tool description is now false under Rust's corpus.** `GET_CLUB_PROFILE` says
   "`n_shots` caps every distance and launch figure … while `n_swings` counts clips". A photo-only
   entry is one of a club's `n_swings` (`CareerCorpus::distinct_swings` is `swings.len()`), with no
   clip.
   - **Why it was not changed here.** The description is the recorded surface, which P9's gate
     holds identical to Python's. P15 owns the surface's declared differences, adding the twelfth
     tool, so it is the place to reword it as a fourth declared difference, or to leave it with a
     reason. One possible wording: "counts swings, clips and photo-only shots alike".
   - **Not affected.** Python's field description ("Clips, deduplicated on their contents") is
     in `outputSchema`, which neither server publishes (P8's item 3).
6. **For P32: `NOTHING_TAGGED` names a route D11 closes, and no Rust verb replaces it.**
   - **What the sentence says.** It tells a model that a stored swing "is retagged one at a time,
     from the change control on its row in the upload page's swing list or with POST
     /api/sessions/<session>/swings/<swing>/club".
   - **What P32 closes.** P32 closes that route. D11 wants each closed route to refuse "with a
     message naming the Rust verb".
   - **What Rust has.** `golf-core`'s verbs are `run`, `parse-screen`, `run-shot`, `set-intent`,
     `rerecord`, `flag-mishit` and the read-only career reports. None sets a stored swing's club
     or golfer. `storage::bundle_store`'s `set_club` exists, and the storage family gates it, but
     no verb calls it.
   - **What P32 must settle.** It must name what the club and golfer refusals point at, or add a
     verb (a finding first). Then `golf-mcp`'s `NOTHING_TAGGED` becomes a declared difference at
     `get_bag_profile` `note`. `READING_A_BAG` and the descriptions name no route. P9's item 10 is
     the precedent, for `flag_mishit.py`.
7. **For P14**: `club::bag` is `read_corpus` plus `BagStore::get` plus `build_bag_profile`.
   `flight-registry`'s per-golfer loft is read off the same `BagStore`, so use
   `storage::bag_store::BagStore` (or `storage::flight_inputs::loft_for_club`) and do not write a
   second bag reader. `flight-registry[7]` (`get_bag_profile`) passes already.
8. **For P15**: `family::DECLARED` still has four rows, and P13 added none. The oracle pin still
   counts `mcp: {python: 22, hand: 2}`.
9. **Green.**
   - **Rust:** `cargo test` (the whole workspace, `CARGO_INCREMENTAL=0`): 1,406 passed, 0 failed,
     9 ignored. Two of the passes are new, both in `club`: the six notes are distinct, and the bag
     note's four arms. `cargo clippy --all-targets` and `cargo fmt --check` are clean. C: had
     5.2 GB free, and nothing was deleted.
   - **Docs:** `pytest tests/test_docs_truth.py` passes 92.
   - **Files and data:** the `data/processed` digest is unchanged (`bd384cc4…`, 192 files), and so
     are the ten other families. `git status -- src scripts tests` shows only P8's three.
   - **Scratch:** the probe and the test log are deleted.

### P14 (2026-10-21) — `simulate_flight`: 38 calls conform exactly, and the flight's prose is Rust's

1. **What landed.**
   - **`crates/analysis`**, because P28's `golf-lab flight` needs the same sentences and nothing may
     depend on `golf-mcp`:
     - `src/flight_caveats.rs` (new), the port of `analysis/flight_caveats.py`: `clamp_caveat`,
       `gate_caveat`, `planar_caveat(count)`, `inferred_caveat`, `altitude_caveat` and
       `caveats_for(result, resolved, model, altitude_m)`. Python's keyword default of sea level is
       an explicit `0.0`, read as Python's truthiness reads it.
     - `src/flight_measure.rs`: `COMPARISON_PAIRS`, `PrintedComparison`, `compare_to_printed`,
       `_reading` and `circular_carry_note(Option<f64>)`, the lines M22 P8b left with their callers.
       The module doc's "What is not ported" section is rewritten to say why they are here now.
       Python's `{:+.2%}` is composed locally (`signed_percent`) from `pyfmt::signed_fixed`, as
       `pyfmt::percent` is from `fixed`, because `pyfmt` gains nothing while `format/` is frozen.
     - `src/flight.rs`: `GATE_AGREEMENT_FRACTION` (0.0259). The validation test now reads it, and
       pins its tightness as Python's test does (the worst case sits within 1e-4 of it).
   - **`crates/mcp`**:
     - `src/flight.rs` (new), the port of `mcp/flight.py`: the seven views in pydantic's field
       order, `missing_swing_flight`, the two silences, `flight_for_swing`, `shot_for` over
       `ShotSource::stream()` (P11's item 7), the recorded comparison through `query::get_swing`,
       and the two readings.
     - `tools.rs`: the body. The unported-tool unit test now names `get_strike_profile`.
     - `family.rs`: a fifth declaration, and a rule now takes `(recorded, item)` (item 4).
     - `query::wire_name` is `pub(crate)`.
   - **No new edge, no dependency, no vector.** No manifest was edited. Nothing under `src/`,
     `scripts/`, `tests/` or `spec/` changed.
2. **The gate: 216 of 216 calls compared, none departing, none skipped.** P14's 38 are
   `tools/flight` 13, `tools/flight-registry` 6, `tools/sessions` 3, `tools/no-directories` 1 and
   `real/data` 15. That covers P8's 29 flight-model calls (C1), so Rust's flight model reproduces
   every recorded flight.
   - **Exact.** A throwaway probe (deleted) held all 38 to exact `serde_json::Value` equality of
     `structuredContent` after the declarations. 37 text blocks are byte-identical. The declared
     one is equal parsed.
   - **Mutation check.** I wrote the offline split's bend with `fixed` instead of `signed_fixed`.
     The gate then failed on exactly 5 recorded calls, `real/data` [31]–[34] and
     `tools/flight-registry[0]`, the flights with an offline row. The mutation was reverted.
3. **The one departure, and it is a judgement: which artifact `recorded_reading` calls current.**
   It is reversible, and `golf_mcp::flight::recorded_reading`'s doc argues it.
   - **What Python asks.** `stored.analysis_version == ANALYSIS_VERSION`, then the stale sentence:
     "the difference is the engine and not the shot".
   - **Why not Rust's 17.** Asked of 17, every artifact on disk (all 16) would blame the engine for
     a declared loft, though 16 -> 17 moved no number. Measured: the gate then departed on exactly
     `tools/flight` [5] and [7], both version-16 artifacts that Python reads as edited.
   - **What Rust asks.** An artifact from `COMPARABLE_FROM` to this engine reads as current.
     That is the question the corpus asks before it pools (ADR-024's M36 addendum), and the one
     this server's career tools already ask. It is Python's own answer on every artifact frozen
     Python writes. It moves one recorded answer, `tools/flight[6]`, a version-15 artifact
     (stale → edited), declared at `simulate_flight` `recorded_reading`.
   - **What stays Python's.** An artifact older than `COMPARABLE_FROM`, newer than this engine, or
     with no readable version still reads as stale. A unit test pins each edge.
   - **The one false case, named in the doc.** A version-14 artifact is inside the range and
     predates the flight family, so a flight it never recorded would read as an edited input.
     None exists in `data/`, and P29 re-runs every stored swing at 17.
   - **To reverse it:** compare with `== ANALYSIS_VERSION`, and declare edited → stale instead. That
     moves `tools/flight` [5] and [7].
4. **The declaration mechanism grew a context argument.** The answer carries no version, so the
   rule reads the one thing in it that dates the artifact. A gap with a non-null `recorded` means
   the artifact recorded a flight, which only engine 15 or later does, and every recorded tree is
   16 or older.
   - **The rule's shape.** It is now `fn(&Value, &Value) -> Value` (the recorded value, then the
     whole recorded item). P11's and P12's rules ignore the second argument.
   - **The self-check works.** The first draft rewrote every stale reading unconditionally. It moved
     6 answers and the gate failed on 5 of them: `tools/flight` [0]–[2] and `tools/flight-registry`
     [0] and [5]. Those are swings with no `analysis_version` at all, which Rust reads as stale too.
5. **What no recorded call reaches is held by unit tests, each to frozen Python's text** (read off
   the frozen modules into scratch).
   - **The clamp caveat's other two branches.** Partly clamped: 150 mph, 12°, 4,000 rpm, "556 of
     1413 points". Never clamped: 2,600 rpm. Every recorded flight is fully clamped.
   - **The rest of the caveats.** `planar_caveat(3)`, and `altitude_caveat(1609.0)` ("1609 m (5279
     ft)").
   - **The comparison.** Both of `compare_to_printed`'s rows on a printed spin and axis, to the
     exact float, and the solved-spin row. `circular_carry_note(None)`, the column form, is
     `golf-lab flight --shots`'.
6. **For P15**:
   - **The walk.** `family::DECLARED` has five rows now. The walk must call `as_declared`, which
     passes each rule its item. Do not call the rules directly.
   - **Nothing is skipped.** `tests/tools.rs`'s skip branch is now reached by no recorded call.
     `get_strike_profile` has none, so the done line's "no `mcp` case is skipped" is about P15's
     own cases.
   - **The oracle pin** still counts `mcp: {python: 22, hand: 2}`.
   - **C1 stands.** The 29 flight-model calls now conform exactly, so a `v<N>` that moves the
     flight moves them, and the engine walk (P19) or a refusal must cover them.
7. **For P28**:
   - **What is ready.** `analysis::flight_caveats` (all five, `planar_caveat` with its count and
     `altitude_caveat` included), `flight_measure::{compare_to_printed, circular_carry_note}` and
     `flight::GATE_AGREEMENT_FRACTION` are ported and tested.
   - **What is left.** `VALIDATION_SHOTS` and `gate_comparisons()` are not ported. The two shots are
     typed inline in `flight.rs`'s
     `both_validation_shots_agree_with_hd_golf_to_within_the_pinned_spread`, which P28 can hoist.
   - **The caveats are unwrapped.** Each is one paragraph, so the CLI's 72-column wrapping is P28's
     to write (Python's `textwrap`, which `golf_mcp::textwrap` already ports at width 95).
8. **For P34**: `golf_mcp::flight::UNREAD_SHOT_SCREEN` is Python's sentence. It names
   `scripts/import_shot_screens.py`, which P34 deletes. Rust's should then name P24's
   `golf-lab import-photos`, declared at `simulate_flight` `detail`, as P9's item 10 is for
   `flag_mishit.py`.
9. **For P35**:
   - **CONFORMANCE's MCP section** gains the fifth declared path, its rule, and the item-aware rule
     shape.
   - **ADR-024's M36 addendum** could say that `simulate_flight` reads an artifact as current from
     `COMPARABLE_FROM` (item 3).
   - **`CLAUDE.md`'s `crates/analysis` paragraph** should name `flight_caveats` and the printed
     comparison.
   - **The M31.5 inventory's R5 row** still lists `flight_measure.py`'s 130 lines as not ported. It
     also missed that `mcp/flight.py` calls them.
10. **Green.**
    - **Rust:** `cargo test` (the whole workspace, `CARGO_INCREMENTAL=0`): 1,417 passed, 0 failed,
      9 ignored. Eleven of the passes are new: five in `flight_caveats`, four in `flight_measure`
      and two in `golf_mcp::flight`. `cargo clippy --all-targets` and `cargo fmt --check` are clean.
      C: had 5.2 GB free, and nothing was deleted.
    - **Docs:** `pytest tests/test_docs_truth.py` passes 92.
    - **Files and data:** the `data/processed` digest is unchanged (`bd384cc4…`, 192 files), and so
      are the ten other families. `git status -- src scripts tests` shows only P8's three.
    - **Scratch:** the two reference probes, the exactness probe, the mutation's backup and the
      test log are deleted.

### P15 (2026-10-21) — `get_strike_profile`, the stdio handshake, the walk and the pin: every `mcp` call compared

1. **What landed.**
   - **`crates/mcp`**:
     - `src/strike.rs` (new, no Python twin): `StrikeProfileView`, `missing_golfer_strike` and
       `strike_profile`, which is `career::resolve_golfer`, `read_corpus` under `career::versions()`,
       the golfer's `BagStore` entry and `golf_core::profile::run_profile`.
     - `src/tools.rs`: the twelfth body. `ToolSpec::body` is no longer an `Option`, and `lands_in`
       and `not_ported` are gone, so a listed tool is an answered tool by construction. `args` lost
       its `#[allow(dead_code)]` (P9's item 7).
     - `src/descriptions.rs`: `GET_STRIKE_PROFILE` written new (item 4), and one sentence of
       `GET_CLUB_PROFILE` reworded (item 6), with `CLUB_PROFILE_N_SWINGS{,_FROZEN}`.
     - `src/family.rs`: `OURS_ALREADY`, `AsOurs`, `recorded_as_ours` (the one reader of `DECLARED`,
       for the gate and the walk alike) and `answer_case`.
     - `src/bin/golf_mcp.rs`: `golf-mcp answer-case < CASE.json`, the runner as a process (item 2).
     - `src/caveats.rs`: `count_word` is `pub`, for the docs pin.
     - Tests: `tools.rs` rewritten (four tests, no skip branch), `stdio.rs` and `docs.rs` new, and
       `surface.rs` gains a fourth declared difference.
   - **`crates/core`**: `rerecord.rs` gains `MCP`, `MCP_DOCUMENTS`, `MCP_UNREAD`, `GOLF_MCP_ENV`,
     `Family::Mcp`, `golf_mcp_binary`, `answer_with`, `blocks_parsed`, `blocks_as_text`,
     `walk_mcp` and `one_against` (`one` is now `one_against` with the committed document as its
     base), plus a fourth count and an "answered by" line in the report, and two unit tests.
     `tests/rerecord.rs` copies the mcp family into every career slice and gains a test for the
     missing binary.
   - **Vectors**: `mcp/hand/strike-profile.json` and the new `mcp/rust/` sub-family (item 5),
     and the 21 walked Python-recorded cases moved `career_version` 0 → 3 (item 3).
   - **Python and prose**: the tool-count pin is gone from `tests/test_docs_truth.py`, whose section
     comment says where it went. `conformance.py check`'s mcp line now also names the hand and Rust
     cases. `scripts/ask_swing.py`'s docstring and `ROADMAP.md`'s M6 line say twelve (item 8).
   - **No new dependency and no manifest edit.** `golf-mcp` already had the `golf-core` edge, and
     the stdio test speaks JSON-RPC by hand rather than through `rmcp`'s `transport-child-process`
     feature, which would have brought a crate of its own.
2. **P9's item 9, settled: (a) the walk spawns `golf-mcp`.**
   - **Why a process.** `golf-core rerecord` cannot link `golf-mcp`, which depends on it, and the
     plan's verify line has `golf-core rerecord` walk `mcp`. So a career run's last family is
     answered by `golf-mcp answer-case`, one process per case, which prints `{answers,
     recorded}`.
   - **Where the binary is found.** `GOLF_MCP` first, then beside `golf-core`, then one up when
     `golf-core` sits in cargo's `deps/` (the in-process `plan` calls in `tests/rerecord.rs`).
     Nothing found is `Refused::Files` before any vector is read, naming every path tried and the
     build command, and a test pins it.
   - **The cost.** The career re-record now needs `golf-mcp` built in the same profile, and
     `cargo run --release --bin golf-core` does not build it. A stale `golf-mcp` would answer with
     stale code, which is why the report prints `the mcp family was answered by <path>`. A workspace
     `cargo test` builds both before any test runs; `cargo test -p golf-core` alone fails the career
     tests on the refusal and never passes them without the family.
   - **(b) The declarations.** The walk applies `family::DECLARED` (P11's choice), through
     `recorded_as_ours` inside `answer-case`, the function the gate calls too, so the two cannot
     hold the family to two lists. It gates Rust's answers against the committed answers as
     declared, then **writes onto the committed document** (`one_against`). A file therefore keeps
     frozen Python's value at every path no career declaration names, `DECLARED`'s paths included.
   - **The surface is not walked.** `surface/` is `MCP_UNREAD`, as the screen family's `units` is:
     nothing `CAREER_VERSION` ages reaches the tool list, the descriptions or the briefing, and its
     four differences are `tests/surface.rs`' to hold. It keeps `career_version` 0 and no ledger,
     which P8's pin allows.
   - **Text blocks are gated parsed**, as `tests/tools.rs` compares them, and the writer puts each
     block back in its committed spelling wherever its value held. A path inside a block is spelled
     `expected.results[i].content[j].text.<key>`. **For the first `career-v<N>` that moves an `mcp`
     answer**: frozen Python's `frozen_view` cannot reach inside a block's string, so the freeze pin
     (`test_the_committed_mcp_cases_are_what_frozen_python_answers`) would then fail on that block.
     That declaration teaches the pin, as the storage family's dotted keys wait for theirs.
     `walk_mcp`'s doc says so.
3. **The done line needed one real write, and P8's pin had already allowed for it.**
   - **What the first dry run showed.** The first `career-v3` dry run with `mcp` walked reported 175
     vectors (76 storage, 56 career, 15 profile, 28 mcp) and **21 changed**, every one a
     Python-recorded case at `career_version: 0` (P8 recorded 0, as the storage family's Python cases
     were before `career-v1`), changing at `career_version` alone. So Rust's answer to every recorded
     call agreed with the committed one, declarations applied, under the gate's comparison.
   - **The write.** `career-v3` declares `career_version` moved, so I ran it for real. The plan lets
     P9–P15 write declared paths in the recorded cases, and
     `test_the_mcp_family_is_committed_in_the_sub_families_python_recorded` already allowed "0
     unless a Rust re-record has ledgered it since". 21 files were written.
   - **Checked against a scratch copy.** Every value is unchanged but `career_version` (now 3) and a
     one-entry ledger (`career_version 3`, `golf-core rerecord`, `career-v3.json`, `added: []`,
     `moved: ["career_version"]`). Each file kept its line endings (the plain ones are CRLF).
   - **What changed in the text.** `serde_json` writes `—` raw where `_write_json` wrote `—`,
     and the gzip's bytes are `flate2`'s. The values are the same, as the module doc says.
   - **After.** The dry run reports 0 changed, debug and release.
4. **`get_strike_profile`: one shape, one corpus, a new description.**
   - **The shape.** The view is `golf-core profile --json`'s element, `{display_name, profile}` with
     the `GradedProfile` whole (M37's handoff), so the phone, the verb and a model read one shape.
     `every_strike_profile_answered_is_the_profile_verbs_json` holds every strike answer in the family
     to `reports::answer(Verb::Profile, json)` over the same tree, value for value.
   - **The other answers.** The miss reuses `missing_golfer_profile`'s sentence and hint. A profile
     that fails its own check is an error result naming the field, as the verb exits 1; no tree
     reaches it.
   - **The schema stays `{player}`.** The player's grades pool every club and the verb's `--club`
     narrows only its text, so a `club` argument would be a second spelling of one answer.
   - **The description is new.** It names the three states a missing grade can be in (absent from
     `topics`, a withheld share with its counts, an absent blend) and says each is not a poor grade.
     It says strike location needs a measured offset and never the printed `CENTER`/`HEEL`/`TOE`
     word, and that drills count toward nothing. P9's draft had conflated absent and withheld.
5. **The strike cases: four worked by hand, four recorded by Rust.**
   - **`hand/strike-profile`.** Four calls, worked from `contracts::strike` and
     `analysis::strike`'s builders before `golf-mcp` answered them: a name nobody answers to; `!!!`,
     which slugs to nothing; `ben`, with nothing on file, whose profile is empty with no `blend` key;
     and `AARON`. Aaron has a photo entry that is admitted, a photo entry practised as a drill and
     left untracked, and a clip tagged 7i with no photo, on a bag declaring a 7i. That gives
     `n_shots` 1, `untagged_shots` 1, no club, no topic and an empty dispersion and list. The first
     run matched it exactly.
   - **`rust/` (`oracle: "rust"`).** Four cases, one call each: `real-data.json.gz`,
     `career-speaking`, `club-mixed-bag` and `mishit`. Each tree is copied whole from the
     Python-recorded case its `provenance.source` names, and a test holds the copy equal to its
     source. The expected answers are `golf-mcp answer-case`'s, laid out by a scratch script, now
     deleted.
   - **Only the real tree grades a topic.** It has consistency, withheld per club and graded for the
     player, and its profile **equals `profile/corpus/real-aaron`'s**, recorded from the storage
     family's slimmer snapshot. A test pins that equality.
   - **The synthetic trees grade none.** Their analyses carry no shot record, so no device says
     what was printed. They pin the club strike profiles: bag order, copied dispersions, and in
     `mishit` a carry `n` of 5 against 6 shots.
   - **The count.** The gate now compares 224 calls: 205 Python-recorded, 15 hand-worked and 4
     Rust-recorded. It asserts that every tool in the table is called somewhere and that all three
     oracles are read. The five declarations moved what they moved before (1, 13, 15, 1, 1).
6. **A fourth declared surface difference, P13's item 5.** `GET_CLUB_PROFILE`'s "while `n_swings`
   counts clips." is false under Rust's corpus, where a photo-only entry is one of a club's swings
   with no clip. It now reads "while `n_swings` counts swings, with or without a clip." The gate
   holds the recorded description to the frozen fragment once and Rust's to the swap, so every
   other description is still Python's exactly.
7. **The stdio test** (`tests/stdio.rs`) spawns the built binary over `tools/career-speaking`'s
   tree, the way Claude Code does, and speaks newline-framed JSON-RPC by hand.
   - **The session.** `initialize` asks for 2025-11-25 and gets 2025-06-18. Then the initialized
     notification, and `tools/list`: twelve, in the table's order, `get_strike_profile` last.
   - **Then three calls.** `get_strike_profile` answers a found golfer, and its block equals its
     `structuredContent`. `resources/read` returns the caveats as JSON. `get_swing` on a missing
     session answers `found: false` as a result, not a protocol error.
   - **The end.** Closing stdin exits 0, and every read gives up after 60 s rather than hanging.
8. **The pin moved (C9).** `crates/mcp/tests/docs.rs` reads Python's list (ARCHITECTURE,
   `ROADMAP.md`, `scripts/ask_swing.py`, every ADR), collapses whitespace, matches Python's `the same
   (\w+) tools` (so "tools." counts), and wants `count_word(TOOLS.len())`. Exactly two sentences
   match, `ROADMAP.md`'s M6 line and `ask_swing.py`'s docstring, and both now read twelve.
   - **How each stays true.** Python's in-process runner still offers eleven, and the frozen
     results page's chat panel keeps it until M40. So ROADMAP says that from M29 P16
     `ask_swing.py` reaches "the same twelve tools `golf-mcp` serves every client" while the panel
     keeps Python's eleven. `ask_swing.py`'s docstring says the same, and that until P16 its tools
     are the eleven, in process.
   - **For P16.** You make both sentences true. Trim `ask_swing.py`'s "until then" clause when it
     switches, and keep the phrase `the same twelve tools` in at least one pinned document, or
     `docs.rs` fails because the claim is gone.
9. **For P19 (C1, P8's item 9).** The career walk now covers `mcp`, but an `ANALYSIS_VERSION`
   declaration still walks only the engine, stage and shot families. So a `v<N>` that moves the
   flight model would leave the 29 flight-model calls unwalked. `walk_mcp` takes any declaration,
   and calling it from `plan`'s `Version::Analysis` arm would walk every mcp case. Its
   `career_version` stamp is already current, so under an engine declaration only answers that
   moved would differ. That, or a refusal of a `v<N>` that moves them, is still P19's.
10. **For P35.**
    - **CONFORMANCE's MCP section** gains `rust/`, the walk (`answer-case`, `GOLF_MCP`,
      `MCP_UNREAD`, the parsed blocks), the 21 cases' `career-v3` ledger, and the fourth surface
      difference.
    - **`CLAUDE.md`'s Commands** should say that a `career-v<N>` re-record needs `golf-mcp` built
      in the same profile, and name `golf-mcp answer-case`.
    - **ADR-032 or ADR-035's addendum** could record that a recorder runs a shell's binary because
      the shell may not be a dependency.
11. **For P36**: build `golf-mcp` in release (`cargo build --release -p golf-mcp`) before the
    `career-v3` dry run, or it refuses by name.
12. **For every later phase: C: is nearly full again.** The release `golf-mcp` build took C: from
    4.7 GB to 2.4 GB free. Nothing was deleted, and P10's item 8 is the remedy if a link fails.
13. **Green.**
    - **Rust:** `cargo test` (the whole workspace, `CARGO_INCREMENTAL=0`): 1,427 passed, 0 failed,
      9 ignored, ten of the passes new. After the last tidy and `cargo fmt`, `cargo test -p golf-mcp
      -p golf-core` passed 185 again. `cargo clippy --all-targets` and `cargo fmt --check` are
      clean.
    - **The dry run:** `cargo run --release --bin golf-core -- rerecord --declare
      spec/declarations/career-v3.json --dry-run` reports 175 vectors run (76 storage, 56 career,
      15 profile, 28 mcp), 0 changed, 0 files written.
    - **Python, run alone:** pytest passes 2,063 in 1,130 s, `tests/test_docs_truth.py` included,
      one fewer than P8's 2,064 because the tool-count pin moved to Rust. Its two warnings are
      paddle's `ccache` notice. `ruff check src tests scripts` is clean, and `conformance.py
      check` exits 0 and prints the reworded mcp line (29 vectors).
    - **The oracle pin** reports 334 vectors, and `mcp: {hand: 3, python: 22, rust: 4}`.
    - **Files and data:** the `data/processed` digest is unchanged (`bd384cc4…`, 192 files), and
      no step read `data/`. `git status -- src` is empty, and the ten other families are
      untouched. Outside `spec/vectors/mcp/`, the frozen Python changed only in the three files the
      plan names for P15.
    - **Scratch:** the hand case's writer, the probe, the Rust recorder, the edit scripts, the
      pre-write copy of the family and the logs are deleted.

### P16 (2026-10-22) — `conversation.py` over `golf-mcp`: the stdio route, and `ask_swing.py` on it

1. **What landed.** Frozen Python changed only in P16's three named files.
   - **`feedback/conversation.py`**:
     - `ask_through_golf_mcp` and `GolfMcpServer` (binary plus the three store flags, with
       `golfers_dir=None` as `--no-registry`).
     - `answering_followups(sentence)`. `ANSWERING_FOLLOWUPS` is now that function over frozen
       `contracts/caveats.py`'s copy, for the in-process route only.
     - `system_prompt` and `ask` gained `answering=`, defaulting to the in-process text.
     - `_sendable` holds the extra, key and model-replay refusals, shared by both routes, so the
       stdio route refuses before it spawns anything.
     - `_caveat`, `_innermost`, and the constants `BUILD_GOLF_MCP`, `CAVEATS_URI` and
       `READ_TIMEOUT_SECONDS`.
     - The module doc has a new *Two routes* section.
   - **`scripts/ask_swing.py`** takes the stdio route.
     - The binary is `--golf-mcp`, else `GOLF_MCP` (the recorder's variable, P15 item 2), else
       `target/release/golf-mcp(.exe)`. A missing binary is exit 2 naming
       `cargo build --release -p golf-mcp`, checked before anything is read.
     - It no longer imports `golf_coach.mcp.*` or `launch_monitor`, and its module-scope imports
       need no SDK.
     - The docstring's "until then" clause is gone, and "the same twelve tools" stays (P15 item 8).
       So both pinned sentences are now true.
   - **`tests/feedback/test_conversation.py`**: seven new tests, three of them against the real
     binary. The real-binary three cover the route end to end, the registry-less tool list, and a
     server that exits before the handshake. The other four: not built, missing key, the
     instructions quoting the route's own caveat, and `ask_swing.py`'s exit 2.
     - The binary is found as `GOLF_MCP`, then release, then debug. The tests skip by name with
       the build command when none is built, or when `mcp`/`anthropic` is missing.
   - **`docs/ARCHITECTURE.md` §1's `ask_swing.py` entry** said "the same eleven tools … called
     in-process, not over stdio", which this phase made false. It now names the stdio route,
     `--golf-mcp` and the exit 2. **For P35**: already done; check it, do not redo it.
   - The in-process route is untouched for `api/app.py`. Nothing under `crates/` or `spec/`
     changed.
2. **The event loop.** ADR-020's addendum left this to M29's plan. The answer:
   - `anyio.run` holds the stdio client session, and `ask` runs on `anyio.to_thread.run_sync`.
   - The SDK's sync runner calls each tool on the thread that iterates it
     (`_beta_runner.py`'s `_generate_tool_call_response`). So `mcp_tool`'s
     `anyio.from_thread.run` finds its token in that worker thread, with no wrapper.
   - **The rejected alternative** was a `BlockingPortal`. It leaves the runner on the main thread,
     which is not an anyio worker, so `mcp_tool` would raise `NoEventLoopError` and every tool
     would need a hand-written portal call instead of the SDK's helper.
   - One session serves one question, so the spawn is paid once per `ask_swing.py` run.
3. **What `mcp` 2.0.0 and the SDK hand back.**
   - The fields are snake_case (`hello.instructions`, `structured_content`). The SDK's
     `_mcp_field_v1_or_v2` reads both spellings.
   - `mcp_tool` returns a tool's answer as a **list of text blocks**, one per `golf-mcp` block,
     not a string. An empty list arrives as `json.dumps(structuredContent)`, i.e.
     `{"result": []}`.
   - So a stdio transcript stores `tool_result.content` as a block list, where the in-process
     route stored a string. `visible_turns` drops both, and `_plain` turns the SDK's `_TaggedDict`
     into a plain dict.
   - The server's input schemas reach the API with `$schema`, `title`, `format: int64` and
     `nullable` in them, and the live call accepted them (the tools are not `strict`).
4. **Measured over the real `data/` with the release build**: spawn plus handshake took 18 ms. The
   slowest tool, `get_golfer_profile`, took 74 ms, so `READ_TIMEOUT_SECONDS` (60, the Rust stdio
   test's bound) is for a dead server, never a slow one. The comment carries the numbers.
5. **The live question**:
   - **The call.** `ask_swing.py 2026-08-10/2 "What does my strike profile say about my ball
     striking overall, and does this swing fit that picture?"` exited 0 in 20.4 s.
   - **Tools.** It called `list_sessions`, then `get_strike_profile`, the twelfth tool, which only
     `golf-mcp` serves.
   - **The answer** separated the outcome and mechanics axes and reported the withheld per-club
     grades as withheld. It said strike location is ungraded and not a bad mark.
   - **The transcript.** `conv-2026-10-22-7478a6fc` is stored under
     `data/processed/conversations/`, which the digest does not cover. It has six messages,
     thinking blocks kept, and `brief_digest` set.
   - **Display.** In this tool's pipe, Python printed `°` and `–` in cp1252, which shows as `�`. A
     real console writes UTF-16 and renders them. This predates P16 and was left alone.
6. **For P17.**
   - **The brief is still frozen Python's.** `ask_swing.py` still seeds the conversation with
     `build_brief(SwingBundleResult)` read from `analysis.json`, which embeds frozen
     `ALIGNMENT_CAVEAT`. Its `_load_brief` docstring wants the conversation and the paragraph to
     open from one rendering. So once the view-JSON brief exists, decide whether `ask_swing.py`
     seeds from it too, and say which.
   - **Reusing the stdio pieces.** P17's entry needs the same session pieces: `GolfMcpServer`,
     `CAVEATS_URI`, `_caveat`, `_innermost`, `READ_TIMEOUT_SECONDS` and `BUILD_GOLF_MCP`. They are
     in `conversation.py`, which imports `coach.py` at module scope. So either `coach.py`'s entry
     imports them inside its function, or P17 moves them into `coach.py` and `conversation.py`
     imports them, the direction `_sdk`/`_note_for`/`_text_from` already go. The second is
     cleaner.
   - **Calling `get_swing` directly.** `await session.call_tool("get_swing", {...})` answers
     `structured_content == {"result": view}` (P9 item 7). `TWO_AXES`, `READING_THIS_DATA_HONESTLY`
     and `ALIGNMENT_CAVEAT` are all keys of the caveats resource.
7. **For P34**: `ask_swing.py` no longer imports `mcp/runner_tools.py`, `mcp/server.py` or
   `launch_monitor`. Only the frozen `api/app.py` still reaches the in-process route.
8. **For P35**: ADR-020's addendum ("M29's plan settles it") gains item 2's answer, and ADR-019's
   unwrap site in `ask_swing.py` is unchanged. CLAUDE.md's Commands could name `ask_swing.py`'s
   `--golf-mcp` and `GOLF_MCP`.
9. **Green.**
   - **Python, run alone:** pytest passed 2,070 in 278 s, seven more than P15's 2,063. Its one
     warning is paddle's `ccache` notice. `tests/test_docs_truth.py` passed 91 after the
     ARCHITECTURE edit. `ruff check src tests scripts` and `mypy src` (118 files) are clean.
   - **Rust:** no crate changed, so `cargo test` was not run.
     `cargo build --release -p golf-mcp` was already up to date.
   - **Files and data:** the `data/processed` digest is unchanged (`bd384cc4…`, 192 files). The
     ten other vector families, `spec/vectors/mcp/` and `spec/declarations/` are untouched.
   - **Scratch:** the probes, the timing script, the test draft and the log are deleted.

### P17 (2026-10-22) — `coach.py`'s JSON entry over `golf-mcp`: stopped on one pin, answered (a), resumed and done

**How this phase ran.** The first session landed everything in items 1–8. It stopped on one test,
`tests/test_config.py::test_only_the_sanctioned_sites_unwrap_a_secret`. The entry's `main()`
unwraps the API key, so `src/golf_coach/feedback/coach.py` is a new `get_secret_value` site.
Adding it to `SANCTIONED_UNWRAP_SITES` touches a test this plan's rules do not name for P17, and
the pin's own message asks for the reason to be recorded in ADR-019. So the phase stopped and
asked (item 9). The live call was made once, in that first session (item 6). The resume made no
call; item 11 is what it did.

**The user's answer (2026-10-22): (a).** Add `src/golf_coach/feedback/coach.py` to
`SANCTIONED_UNWRAP_SITES` in `tests/test_config.py`, and write the ADR-019 addendum recording why
**in this phase**, not in P35. The reason is the one item 9 gives: it is the same boundary as
`ask_swing.py`'s site in ADR-019's 2026-08-15 addendum, and it takes over from `api/pipeline.py`'s
site when M40 deletes that file.

1. **What landed in Rust** (`crates/mcp`, two keys on `get_swing`'s view, both declared):
   - **`CheckpointView.one_sided`**, read off the stored score (`null` when it does not say).
   - **`UnscoredView.family`**, `checkpoint` or `flight`, from
     `contracts::unscored::is_inference_reason` (`query::family_of`, with `CHECKPOINT_FAMILY` and
     `FLIGHT_FAMILY`). A photo-side reason reads as `checkpoint`; C6 keeps those out of
     `analysis.json`.
   - **The module doc of `query.rs`** has a new section saying why each key exists. The briefing
     has always told clients to "read `one_sided` before describing a low number as a strength",
     and no tool answered one until now.
   - **Two declarations in `family::DECLARED`**, now seven, both on `get_swing`:
     - `checkpoints` (`sidedness_from_the_registry`). It reads `CHECKPOINT_REGISTRY` by name,
       because the recorded answer does not carry the artifact. It is self-checking: every
       recorded tree's stored scores carry the registry's sidedness (measured over every `tools/`
       and `real/` case).
     - `unscored` (`family_from_the_reason`).
     - They moved 36 and 14 recorded answers. No hand or Rust case answers `get_swing` with a
       checkpoint or an unscored entry (`hand/shot-view.json`'s has neither), so no committed
       vector changed. There is no `outputSchema` on the surface, so `surface.rs` is unaffected.
   - **One unit test**, `the_brief_s_two_keys_read_the_score_and_the_reason`.
2. **What landed in Python.** Only the files the plan names, plus `tests/feedback/conftest.py`
   (new, under `tests/feedback/`).
   - **`feedback/coach.py`**:
     - `coaching_system_prompt(two_axes, reading)`. `SYSTEM_PROMPT` is now that function over the
       frozen copies, byte-identical (sha256 checked before and after).
     - `_coaching_client` and `_write`, split out of `generate_coaching` so both routes share the
       request, its notes and the extra-before-key order. `generate_coaching`'s behaviour is
       unchanged and its tests pass untouched.
     - The stdio route: `brief_from_view`, `is_analyzed`, `read_swing` (one stdio session:
       `initialize`, the caveats resource, `get_swing`), `brief_through_golf_mcp` (returns a
       `Brief` with a refusal sentence, and `unanalyzed` for a miss or no analysis),
       `coach_through_golf_mcp` (the answer dict), `_spelled`, `main`, and the `__main__` hook.
     - `GolfMcpServer`, `BUILD_GOLF_MCP`, `CAVEATS_URI`, `READ_TIMEOUT_SECONDS`, `_caveat` and
       `_innermost` **moved here from `conversation.py`**, as P16 item 6 recommended.
       `conversation.py` imports them, so every existing name still resolves there.
     - `default_golf_mcp` moved here from `ask_swing.py`'s `_default_golf_mcp`.
   - **`scripts/ask_swing.py`** seeds from `brief_through_golf_mcp` (item 5).
   - **Tests.** `tests/feedback/test_coach.py` has fifteen new tests and
     `tests/feedback/test_conversation.py` one. The `golf_mcp` fixture moved into the new
     `tests/feedback/conftest.py` beside a `store_swing` writer. The real-binary tests skip by name
     when no build exists.
     - The strongest pin is `test_the_view_brief_is_build_brief_line_for_line_but_the_four_named`.
       It stores one result that reaches every line, reads it back through the real server, and
       requires `build_brief`'s text exactly, minus item 3's four differences.
   - **`docs/ARCHITECTURE.md` §1**: the `ask_swing.py` comment says where the seed comes from now,
     and the entry has its own block (flags, the JSON, the exit codes). **For P35**: already done;
     check it, do not redo it.
3. **D15's gap, measured.** Every `build_brief` line was checked against what `SwingView` carries.
   Two needed keys (item 1). Four are rendered differently, each listed in `brief_from_view`'s
   docstring:
   - **The shot's device.** The line says "the launch monitor's screen" where it said "the
     hd_golf screen". `ShotView` has no `device`, and the value is a storage key.
   - **The `no tile found for` warning** is gone, because `golf-mcp`'s filter drops it (P11).
   - **The alignment summary** is not repeated on the `quality:` line. It is inside
     `alignment_caveat` on every tier below `full`.
   - **The alignment notes** are not repeated under TWO-VIEW ALIGNMENT. Both engines copy each
     one into the swing's notes as `alignment: …` (`engine.py:637`, `engine.rs:818`).
   - **Two more that no stored swing reaches:**
     - The verdict heading appears only when there is a headline or a tip. The view cannot tell
       "no feedback" from "empty feedback".
     - A score whose `one_sided` is `null` prints no direction rather than a guessed one.

   **Measured over the 15 stored swings** with the release build: the only differing lines are
   the device (15 swings), the layout warning (15), the quality summary (15), and 8 alignment
   notes over 8 swings. Every other line is identical: checkpoints with their direction, both
   unscored families, placements, pivots, verdict, notes. A view from a server without the two
   keys is refused by name with the rebuild command (`_require_p17_keys`), because a silent fall
   back would drop the direction and mislabel every flight refusal.
4. **The entry's contract. For P26**, which spawns it:
   - **Command**: `python -m golf_coach.feedback.coach --session S --swing W [--golf-mcp PATH]
     [--sessions-dir D] [--shots-dir D] [--model M]`. `golf-mcp` is `--golf-mcp`, else
     `GOLF_MCP`, else `target/release`, and is started with `--no-registry`.
   - **Stdout** is one JSON object, ASCII (`json.dumps` default), so no code page can mangle it:
     `{"text", "provenance": {"model", "generated_at", "input_digest"}}` with exit 0, or
     `{"note"}` with exit 2. A note is already in the results page's vocabulary ("no written
     coaching: …") for `notes`, as `api/pipeline.py` appends one.
     - `generated_at` is pydantic's spelling (`…Z`, no fraction on a whole second). It is pinned
       against `CoachingProvenance.model_dump`, and `contracts::Timestamp` parses it.
     - `input_digest` is the sha256 of the **view** brief, so it differs from a frozen run's.
   - **Stderr** carries `golf-mcp`'s one-line banner.
   - **`get_swing` reads `analysis.json` from disk**, so golf-lab must write it before spawning,
     then fold the answer in and write again. The brief reads `notes`, as Python's does after its
     extend.
   - **The entry does not read `coaching_enabled`.** C5's switch is the caller's, and
     `GOLF_COACHING_ENABLED` may live in `.env`, which only pydantic-settings reads today. P26
     decides how golf-lab learns it.
   - An argparse error is also exit 2, but prints no JSON. Treat unparseable stdout as a failure.
   - `read_swing`'s `errlog` must be a real file (the SDK's stdio client calls `fileno`). A
     `StringIO` fails with `UnsupportedOperation: fileno`.
5. **P16 item 6's question: `ask_swing.py` seeds from the view brief too.** The reasons:
   - Its own docstring wants the conversation and the paragraph to open from one rendering, and
     from P26 the paragraph is written by this entry.
   - The old seed read `SwingBundleResult`, a pydantic shape ADR-035 clause 1 does not keep past
     M40, while `ask_swing.py` survives.
   - It quoted frozen `ALIGNMENT_CAVEAT`, where the view quotes `golf-mcp`'s.

   The cost is one extra spawn (about 18 ms) when a conversation starts, and the docstring says
   so. A stored transcript's `brief_digest` is unaffected, because `--resume` seeds nothing.
   `_load_brief` returns the old "no analyzed swing … Run scripts/analyze_bundle.py" sentence for
   a miss or an unanalyzed swing, and `no brief: <why>` when `golf-mcp` cannot be used.
6. **The live call**:
   - **The call.** `python -m golf_coach.feedback.coach --session 2026-08-10 --swing 2`
     exited 0 and printed `{text, provenance}`, model `claude-opus-5`, `input_digest`
     `0465f6a5…`.
   - **The paragraph** names tempo as the one thing (backswing 868 ms against the 1044 ms the
     ratio wants), ties it to head_stays_back, and brings in the placement as one clause. It
     flags the pose-model mismatch note and says the launch-monitor numbers are not scored.
   - **Nothing was written**: the entry writes nothing. **This was P17's one call.**
7. **For P34 and M40.** `ask_swing.py` no longer imports `contracts/swing.py` or calls
   `build_brief`. On `coach.py`'s stdio route, nothing reaches a frozen contract:
   - `config` is imported inside `main` and `default_golf_mcp` only.
   - The module-scope imports of `contracts/` serve only the in-process half: `build_brief`,
     `generate_coaching`, `SYSTEM_PROMPT` and `CoachingOutcome`.
   - So M40 deletes that half with `api/pipeline.py` and the rest stands.
8. **For P35**:
   - CONFORMANCE's MCP section gains P17's two declared paths (`get_swing` `checkpoints` and
     `unscored`, which add keys rather than rewrite values).
   - ADR-020's addendum gains item 5.
   - **The ADR-019 addendum is written** (item 11), and so is its row in `docs/README.md`'s map.
     Check them; do not redo them.
9. **Why it stopped, and what the resume does.** The pin's message: "If the new site is right, add
   it to SANCTIONED_UNWRAP_SITES and record why in ADR-019". ADR-019's 2026-08-15 addendum is the
   precedent: `ask_swing.py`, the same boundary.
   - **If the answer is yes:**
     - Add `"src/golf_coach/feedback/coach.py"` to the set in `tests/test_config.py`.
     - Make the comment above it say the Anthropic SDK three times: the one-shot paragraph in
       process and over `golf-mcp`, and the conversation. `api/pipeline.py`'s site goes at M40,
       and this one replaces it.
     - Write the ADR-019 addendum, or leave it to P35 with a line here.
     - Run pytest alone (expected: 2,086 passed), then tick.
   - **Rejected alternatives:**
     - Reading the key with `getattr(key, "get_secret_value")()` would dodge the `ast` pin, which
       is the pin's whole point.
     - Having golf-lab pass the key in the environment would put the secret into Rust, which reads
       no `.env`.
10. **Green, except item 9's pin.**
    - **Rust:** `cargo test` (the whole workspace, `CARGO_INCREMENTAL=0`): 1,428 passed, 0 failed,
      9 ignored.
      - The tools gate compared 224 calls (python 205, hand 15, rust 4), and each declaration
        moved at least one.
      - `cargo clippy --all-targets` and `cargo fmt --check` are clean.
      - `rerecord --declare spec/declarations/career-v3.json --dry-run`: 175 vectors (28 mcp,
        answered by the release `golf-mcp`), 0 changed.
      - The release `golf-mcp` is rebuilt with the keys.
    - **Python, run alone:** pytest gave 2,085 passed and 1 failed (the pin), in 260 s. The one
      warning is paddle's `ccache` notice. `ruff check src tests scripts` and `mypy src` (118
      files) are clean, and `tests/test_docs_truth.py` passed 91.
    - **Files and data:**
      - The `data/processed` digest is unchanged (`bd384cc4…`, 192 files).
      - `spec/vectors/`, `spec/declarations/` and the version keys are untouched.
      - `git status -- src scripts tests` shows P8's three, P15's `test_docs_truth.py`, P16's
        files, and P17's `coach.py`, `conversation.py`, `ask_swing.py`, `test_coach.py`,
        `test_conversation.py` and `tests/feedback/conftest.py`.
    - **Scratch:** the comparison script, the live output and the logs are deleted.
11. **The resume (2026-10-22), on the user's (a).** Files changed after the 08:23 marker were
    listed first. All of them were the first session's items 1–8, as item 10 describes. They were
    kept as they were, and nothing in them was redone. The resume changed four files:
    - **`tests/test_config.py`**:
      - `SANCTIONED_UNWRAP_SITES` gained `src/golf_coach/feedback/coach.py`.
      - The comment above the set now says the Anthropic SDK is unwrapped three times: the
        paragraph in process and over `golf-mcp`, and the conversation. It also says
        `api/pipeline.py`'s site goes at M40 and this one replaces it.
      - The pin's docstring said "A fourth `get_secret_value`" over what was already a fifth
        site. It now says "A new … site", so it carries no count.
    - **`docs/decisions/019-secret-handling.md`** has a 2026-10-22 addendum. It gives:
      - why the site has the right shape: the same SDK boundary as `ask_swing.py`, and
        `coach_through_golf_mcp` takes `api_key: str | None`;
      - that it replaces `api/pipeline.py`'s site at M40 rather than widening the surface for
        good;
      - the two alternatives item 9 rejected;
      - how the pin surfaced it.
    - **`docs/README.md`**: the addendum moved two docs-truth pins,
      `test_the_map_counts_the_addenda_correctly` and
      `test_each_adr_row_states_that_adrs_own_addendum_count`. So the map's total went 108 → 109,
      and ADR-019's row went 1 → 2 with the new addendum's gist. **The document count is
      untouched**, as the rules require before P36. **For P36**: the map changed here, so its
      count check starts from this.
    - **Green.**
      - **Python:** pytest, run alone, passed 2,086 with 0 failed, the count item 9 expected. The
        first full run after the pin edit gave 2,084 passed and 2 failed. The two were the map
        pins above, which then passed with `tests/test_config.py` (98 passed together) before the
        final full run.
      - **Warnings:** paddle's `ccache` notice, plus `PytestCacheWarning`s because this session
        could not write `.pytest_cache`. Those are the environment, not a test.
      - **Checks:** `ruff check src tests scripts` and `mypy src` (118 files) are clean.
      - **Rust and data:** nothing under `crates/` or `spec/` changed in the resume, so
        `cargo test` was not re-run. Item 10's run stands. The ten frozen families and
        `spec/declarations/` show no diff. The resume read nothing under `data/`.

### P18 (2026-10-22) — the `selection` family recorded from frozen Python

1. **What landed.**
   - **`scripts/conformance_vectors.py`** has a new section at its foot, with its runners. The
     section's header comment is the encoding, and P19 and P25 should read it first. The runners
     are `build_selection`, `_run_selection`, `_selection_file` (the rebuild),
     `_selection_answers`, `_auto_windows_answer`, `_selection_vector` and the three case
     builders `_selection_rules`, `_selection_windows` and `_selection_corpus`.
   - **`scripts/conformance.py`** gained `selection_vector_paths()`, a `regenerate
     --selection-once` with its refusal text, and `check`'s selection line. `_REFUSED` now names
     five once-flags.
   - **`tests/test_conformance.py`** has nine new pins at its foot, and `_refuse_to_build` knows
     `build_selection`.
   - **`crates/core/tests/rerecord.rs`**: one sentence in `ORACLE_BY_FAMILY`'s doc says
     `selection` writes its oracle into every file, as `mcp` does. Nothing else under `crates/`
     changed, and nothing under `src/` changed.
2. **The family.** 45 cases, all gzipped, 0.78 MB of gzip in all (872 KB on disk, 3.9 MB as plain
   JSON). That is far under the plan's 10 MB question, so nothing stopped here.
   - **`rules/` (17)** comes from `tests/analysis/test_select_swing.py`.
   - **`windows/` (13)** comes from `tests/api/test_pipeline_auto_window.py`'s bundles, plus five
     branches no test reaches: a face-on window given, a down-the-line window given, no second
     view, a clip with no fps, and the down-the-line view readable only on the lead wrist.
   - **`corpus/` (15)** is the stored bundles. Their 30 clips hold 42,648 frames.
   - Every case has `kind: "selection"`, `oracle: "python"`, and top-level `analysis_version` 16,
     which is frozen Python's (call 1: it ages on the engine key). The provenance carries
     `python_version`, `pydantic_version` and `recorded_by`.
   - The record took 26 s. Frozen Python answers all 45 from their committed inputs in 7 s.
3. **What selection reads, measured.** Each call reads one landmark, the wrist it is told to
   read. It reads only that wrist's `y` and `visibility`, and it reads every frame:
   - `candidate_downswings` drops the `x` that `_lead_wrist_xy` carries.
   - `smooth_keypoints` averages each landmark alone, weighted by its own visibility.
   - `_rising_runs`' floor is a fraction of the whole clip's tracked range, so no frame can be
     cut.
   - `frame_index` and `timestamp_ms` are never read.

   The recorder proves this for every case. Each call is answered over the whole clip and again
   over the rebuilt cut, with every other landmark and the wrist's `x` set to zero. The build is
   refused if the two answers differ. The first probe ran `_auto_windows` both ways over all 15
   stored bundles, and windows, notes and log lines were all identical.
4. **The encoding, for P19 and P25** (the section header has it in full).
   - A case's input is `{clips, calls}`. Each clip is `{fps, frames, tracks: {LEFT_WRIST|RIGHT_WRIST:
     {y, visibility}}}`. Each call names its `fn`, and `expected.results` holds one answer per call.
   - The **rules** calls are `select_swing`, `select_matching_swing` and `window_around`. They read
     the clip's frames **as given**, with `fps` passed as an argument. A `rules/` clip is stored
     already smoothed, as the test smoothed it before calling the rule.
   - **`_pick_swing` smooths first** and takes its fps from the clip.
   - `_auto_windows` answers `{windows, notes, log, picks}`. **Each of its picks is also listed as a
     `_pick_swing` call immediately before it**, and `_verify_picks_lead` refuses a case where that
     is not true.
   - A choice is written `{window, downswing: {top, impact, rise}, candidates, reason}`, or `null`
     for a decline.
   - Wrists are written by their `PoseLandmark` names: `LEFT_WRIST` is 15 and `RIGHT_WRIST` is 16,
     the `#[repr(u8)]` index in Rust.
5. **What the corpus says.** On all 15 bundles, `_auto_windows` chose exactly the stored
   `analysis.json` windows. It appended **no** note on any of them. Every face-on pick on the
   corpus took a descent that ended at a ball strike. Every down-the-line pick matched face-on's
   reference, and each of those also had a strike.
   - Only `2026-08-10-2` has a "took the last" case: two struck descents.
   - **No stored bundle reaches a decline, the lone-candidate escape, or the reverse match.**
     Only `windows/` and `rules/` reach those, so they are the only gate on them.
   - Three bundles hold the same clips: `2026-08-07-aaron1-1`, `2026-08-09-2` and `2026-08-10-1`.
     They are kept as three cases, to mirror the corpus.
   - Strike lists are in the stored file's order, unsorted (`_strikes`). For example,
     `2026-08-23-8` face-on is `[437, 430]`.
6. **`check` judges this family's freshness by the stages' rule**, because it is the one later
   family that both ages on `ANALYSIS_VERSION` and was recorded by frozen Python. A case below
   v16, or above it with no ledger, is printed as STALE and makes `check` exit 1. The pin
   `test_check_defers_the_selection_family_and_judges_its_staleness` holds both halves, and
   `list` shows each case at its own engine version. The only Python-side pin on the freeze is
   `test_the_committed_selection_cases_are_what_frozen_python_answers`. It re-runs every committed
   case, corpus included, from the case's own input, and needs no `data/`.
7. **For P19, the version.** The family sits at v16 and Rust is at v17. `walk_shot`'s pattern sets
   `ours["analysis_version"]` to Rust's version, and `v17.json` declares `analysis_version` as
   `moved`. So the first `v17` walk over `selection` **moves each case's version 16 → 17 under a
   ledger entry**, as P15's career walk moved `mcp` 0 → 3. The dry run reports 0 changed only
   after one real run, and that real run writes `spec/vectors/selection/`. The rules list allows
   only P18 to write there.
   - P15's precedent says the move is the declared path, ledgered, and that it is right.
   - Even so, **P19 should stop and ask before that write.** Recommend allowing it, as P15 did.
   - After the move, `ledger_covers` keeps `check` and the committed-family pin green: both read
     the ledger, not equality.
   - `v17.json` also declares `expected.analysis_version`, and a selection case has none. That
     path is matched by the engine vectors, so it should not trip `unmatched`, but P19 should
     check.
8. **For P19, the port.**
   - The `rules/` calls are the rules alone.
   - The `_pick_swing` calls are `smoothing::smooth_keypoints` followed by the rule. Rust's
     `smooth_keypoints` is a straight port in the same order of operations, so the stored float
     `reference_downswing_s` and each `rise` should reproduce bit for bit.
   - Skip the `_auto_windows` calls by `fn` name until P25.
   - Rebuild a clip as `_selection_file` does: 33 landmarks, zeros everywhere except the tracked
     wrist's `y` and `visibility`.
   - The reasons need `pyfmt`'s `.2f` and `%g`. The band prints as `0.12-0.8s`, not `0.80`.
   - `select_matching_swing`'s tie-break is `min` over the reversed list, so the later candidate
     wins a tie.
9. **For P25.** Each `windows/` and `corpus/` case ends in its `_auto_windows` call. That call's
   `picks` give the order the pipeline made its picks in, and `log` holds the lines exactly,
   each with two leading spaces. Two pieces of text are `analyze_swing_dir`'s and are **not**
   recorded here: the `"\nSwing selection:"` heading, and the "no ball strike was heard" note.
   `windows/silent-face-on` records only that `[]` and `null` strikes pick alike. D20's input
   gate can take each bundle's windows from these cases.
10. **For P29.** `test_the_corpus_selection_cases_choose_the_stored_windows` builds the corpus from
    today's `data/`, and `_selection_corpus` raises if a bundle's stored windows or notes differ.
    It is held to `data/` and never to the committed cases, as the `mcp` real pin is. So after
    P29 rewrites `analysis.json`, it holds frozen Python's selection to the windows Rust wrote,
    and a moved window fails it. That failure would be a real finding.
    - `_stored_strikes` also refuses an `audio.json` whose `clip.fps` differs from its keypoints
      file's. If P29 rewrites the audio artifacts, that check runs over the new ones.
    - The committed `corpus/` stays a record of `data/` as it was before P29.
11. **For P33–P34.** The recorder needs four things: `tests/analysis/test_select_swing.py`, loaded
    by path with `tests/analysis/conftest.py` put in `sys.modules["conftest"]` for the one import;
    `tests/api/test_pipeline_auto_window.py`, loaded by path; frozen `analysis/phases.py`; and
    `api/pipeline.py`. The build, reach and freeze pins need the same. If P33 or P34 moves or
    deletes any of them, those pins go with it, and the committed family stays.
12. **For P35.** CONFORMANCE §2 has no selection section. Its content is items 2–6. §4's
    `regenerate` list should gain `--selection-once`. Docs were not touched here.
13. **Green.**
    - **`cargo test`** passed for the whole workspace: 1,428 passed, 0 failed, 9 ignored. The
      oracle pin reports `"selection": {"python": 45}` among 379 vectors.
    - **`cargo clippy --all-targets`** and **`cargo fmt --check`** are clean.
    - **`conformance.py check`** exits 0 and prints `45/45 selection vectors are at frozen v16 or
      ledgered above it`.
    - **pytest**, run alone, passed 2,095 with 0 failed in 313 s. That is P17's 2,086 plus the
      nine new pins, with the corpus pin run and not skipped. The one warning is paddle's
      `ccache` notice.
    - **`ruff check src tests scripts`** is clean.
    - **Data:** the `data/processed` digest is unchanged before and after the record
      (`bd384cc4…`, 192 files).
    - **Diffs:** `git status -- src` shows only P16–P17's `coach.py` and `conversation.py`. The
      ten other families and `spec/declarations/` show no diff.
    - **Scratch:** the probe, the dry-build and peek scripts, and the logs are deleted.

### P19 (2026-10-22) — the swing selection in Rust: every answered call exact, and the walk moved the family 16 → 17 once

1. **What landed.**
   - **`crates/analysis/src/phases.rs`**: `Downswing` (with `seconds`), `candidate_downswings`,
     `SwingChoice`, `window_around`, `struck`, `lone_candidate_choice`, `select_swing` and
     `select_matching_swing`, with `MATCH_TOLERANCE_S`, `WINDOW_LEAD`, `WINDOW_TRAIL`,
     `MIN_ADDRESS_LEAD_S` and `CANDIDATE_MIN_RISE`. `POSSIBLE_DOWNSWING_S` and `STRIKE_TOLERANCE_S`
     went from `pub(crate)` to `pub`, as in Python. Public and private follow Python's underscores.
     The module doc's "not ported yet" section is now the clip-choosing half and its gate. There
     are five new unit tests for what the family cannot be relied on to reach: the tie-break, `%g`'s
     band, a no-fps decline, and the two half-to-even rounding sites (`round(1.5 * fps)` at 7 fps
     and `round(0.20 * fps)` at 12.5 fps).
   - **`crates/core/src/selection_family.rs`** (new; its twin is `_run_selection`). `run_selection`
     takes a case's input and returns one `Option<Value>` per call. `NOT_YET` is
     `[("_auto_windows", "P25")]`, and `not_yet` looks a call up in it. `_pick_swing` is a private
     `pick_swing`: `smooth_keypoints` at `DEFAULT_WINDOW`, then the matching rule when a reference
     is given and the lone rule when not. A clip is rebuilt as `_selection_file` rebuilds it. Every
     call's arguments are read with `deny_unknown_fields`, and only `LEFT_WRIST`/`RIGHT_WRIST`
     are accepted as landmarks.
   - **`crates/core/tests/selection.rs`** (new, five tests):
     - the gate, by value **and** under §3;
     - the version-and-ledger pin;
     - picks are the `_pick_swing` calls ahead of them;
     - a branch census;
     - the runner's refusals.
   - **`crates/core/src/rerecord.rs`**:
     - new: `SELECTION`, `SELECTION_DOCUMENTS`, `Family::Selection` and `walk_selection`;
     - `plan`'s engine arm now walks engine, stage, shot, **selection, then mcp**, and finds
       `golf-mcp` first;
     - the report counts `N selection, N mcp`, and names the `golf-mcp` it ran on an engine run
       too;
     - `golf_mcp_binary`'s refusal now says "an engine or a career declaration";
     - `src/bin/golf_core.rs`'s module doc names the two families an engine run now walks.
   - **`crates/core/tests/rerecord.rs`**:
     - every slice now holds `selection/rules/fast-downswing`, `selection/windows/face-on-only` and
       `mcp/tools/flight-registry`, with the empty sub-families created;
     - two count lines moved from 5 to 8;
     - the career-isolation test leaves `mcp/` readable for its engine run;
     - five new tests: the walk order, a version move, the carry refusal, an unplaced sub-family,
       and an engine run with no `golf-mcp`.
   - **No new dependency, no manifest edit, no Python, no doc.**
2. **The gate: every answered call is frozen Python's answer, by value.** The comparison is
   exact `Value` equality, not only §3's `RTOL`.
   - 45 cases.
   - 95 calls compared: 56 `_pick_swing`, 23 `select_swing`, 11 `select_matching_swing` and 5
     `window_around`.
   - The `_pick_swing` calls ran smoothing and then the rule. Every `rise` reproduced bit for bit
     and every reason string exactly, as P18 item 8 predicted.
   - 28 `_auto_windows` calls are skipped by name until P25. The gate checks that every name in
     `NOT_YET` is still being skipped, so the skip cannot be vacuous.
   - Every branch of both rules is reached. Counts per branch:

     | Branch | Calls |
     |---|---|
     | decline | 15 |
     | a strike narrowed the field | 34 |
     | lone, struck | 5 |
     | lone, nothing to choose between | 2 |
     | one plausible | 32 |
     | took the last | 9 |
     | matched in band | 24 |
     | vouched for outside the band | 3 |
     | window with fps | 2 |
     | window without fps | 3 |

     The census pin fails by name if a case stops reaching one.
3. **The write the user allowed (decided at the P18/P19 boundary).**
   - **The first `v17` dry run** reported 140 vectors: 21 engine, 21 stage, 25 shot, 45 selection
     and 28 mcp. **45 changed**: every selection case, each at `analysis_version 16 -> 17` and
     nothing else.
   - **The real run wrote those 45.** A scratch copy taken beforehand showed every value
     unchanged except the version. Each case gained a one-entry ledger: `analysis_version 17`,
     `golf-core rerecord`, `spec/declarations/v17.json`, `added: []`, `moved: ["analysis_version"]`.
     The gzips are LF, as written.
   - **After the write**, the dry run reports **0 changed**, in release, in 5.6 s.
   - **P18 item 7's question** (does `expected.analysis_version` trip the typo guard?): it does
     not. Every engine vector's ledger already holds that path at v17, so `unmatched` passes.
4. **C1, settled as P15 item 9 suggested: an engine run walks the mcp family too, every case.**
   - **Why every case:** `simulate_flight`'s 29 flight-model calls are the obvious exposure, but
     the career and club tools also join `ranges.json`'s bands. So walking only the cases with a
     non-empty `flight_model_calls` would miss band moves. `walk_mcp`'s doc says this.
   - **The `career_version` stamp** the walk writes is already current, so under an engine
     declaration only an answer that moved would differ. Its ledger entry would carry
     `analysis_version`.
   - **On the real `spec/`:** the 28 walked mcp cases (21 recorded from Python, 3 by hand and 4 by
     Rust; `surface/` stays unread) answered with 0 changed. Nothing under `mcp/` was written.
   - **The cost:** every `golf-core rerecord` with an engine declaration now needs `golf-mcp`
     built in the same profile, as a career run already did.
     - The verify line's `cargo run --release --bin golf-core -- rerecord --declare
       spec/declarations/v17.json --dry-run` needs `cargo build --release -p golf-mcp` first.
       A stale `golf-mcp` would answer with stale code, which is why the report names the binary.
     - `cargo test -p golf-core` alone now fails the engine rerecord tests on the refusal, not
       only the career ones. A workspace `cargo test` builds `golf-mcp` first. Run
       `cargo build -p golf-mcp` before testing `golf-core` alone.
5. **The carry, and its guard.**
   - **The carry:** the walk carries each `_auto_windows` answer as committed.
   - **The guard:** a case that carries an answer is refused if **any** answered call in it moved,
     whether or not the move is declared. The refusal names the moved calls, the carried ones and
     "until M29 P25". The reason is that an `_auto_windows` answer's windows and log are built
     from the `_pick_swing` answers ahead of it, so a declaration could otherwise move a pick and
     leave the windows chosen from it untouched.
   - **The test:** `a_moved_pick_cannot_leave_its_carried_windows_behind` pins both sides. The same
     declared reason lands in a rules case and is refused in a windows case.
6. **For P25.**
   - **The walk cannot link `crates/lab`**, just as it could not link `golf-mcp` (P15 item 2). So
     porting `_auto_windows` into the lab does not, by itself, stop the walk carrying its answers.
     P25 must choose one of three:
     - spawn a `golf-lab` verb, as the mcp walk spawns `golf-mcp answer-case`;
     - put `_auto_windows` somewhere `core` can reach;
     - keep the carry, with the guard.
   - **Whichever P25 picks**, it then empties `NOT_YET`. Removing that entry makes
     `every_selection_call_is_run_selections_answer_exactly` assert that nothing is skipped. That
     is the done line, "no `selection` case skipped".
   - **`_pick_swing`'s composition is `selection_family::pick_swing`, which is private.** The lab's
     pick must be the same composition: smooth at `DEFAULT_WINDOW`, matching **only when a
     reference is given**, and the clip's own fps. Reuse it (make it `pub`, or move it), rather than
     writing a second copy.
   - **The reasons the lab logs** are `SwingChoice::reason` exactly. `_auto_windows`' log adds
     two leading spaces and a `face-on: ` or `down-the-line: ` prefix (P18 item 9).
   - **`not_yet`** is the lookup the gate and the walk share.
7. **For P35.**
   - **CONFORMANCE's selection section** (P18 item 12) should cover: the Rust gate
     (`tests/selection.rs`, by value), the walk after shot, the carry and its guard, and the 45
     cases' `v17` ledger.
   - **The mcp section and §4** should say that an engine declaration walks `mcp` too and needs a
     built `golf-mcp`.
   - **`CLAUDE.md`'s `rerecord` comment** ("the engine and stage vectors, and since M35 the shot
     vectors with them") should gain selection and mcp.
   - **`conformance.py check`'s mcp line** still says the family ages on `CAREER_VERSION` alone.
     That is frozen tooling P35 may reword under the plan's first rule.
   - **`crates/trigger/src/clip.rs` copies `_WINDOW_LEAD`, `_WINDOW_TRAIL` and the 0.80 bound by
     value**, with doc pointers to `phases.py`. Rust twins now exist in `analysis::phases`
     (`WINDOW_LEAD` and `WINDOW_TRAIL` private, as in Python). `trigger` has a Python caller and no
     `analysis` edge, so I did not join them. It is a drift risk to record, or to decide in M40.
8. **For P36.** Build `golf-mcp` in release before the `v17` dry run (item 4). The expected result
   is 140 vectors and 0 changed, unless P25 adds cases.
9. **Green.**
   - **`cargo test`** passed for the whole workspace: 1,443 passed, 0 failed, 9 ignored. That is
     P18's 1,428 plus 15 new tests: 5 in `phases`, 5 in `selection.rs` and 5 in `rerecord.rs`.
   - **The oracle pin** reports `"selection": {"python": 45}` among 379 vectors, with 516 ledger
     entries naming 5 declarations.
   - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**.
   - **`conformance.py check`** exits 0. **`pytest tests/test_conformance.py -k selection`**
     passed 9: P18's pins, which read the new ledger. Neither is on P19's verify line; I ran them
     because this phase wrote the family they read.
   - **`v17` dry run:** 0 changed.
   - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files.
   - **Diffs:**
     - nothing under the ten held families;
     - no new `spec/declarations/` file;
     - `src/` shows only P16–P17's two files.
   - **Scratch:** the selection backup, the dry-run log and the test log are deleted.

### P20 (2026-10-22) — `crates/lab`'s skeleton, and the strike cache in Rust: every stored file round-trips

1. **What landed.**
   - **`crates/contracts/src/audio.rs`**, the twin of `contracts/audio.py`:
     - `AUDIO_DETECTOR_VERSION` (`i64`, 2), `AudioStrike`, `AudioClipMetadata` and `AudioFile`.
     - Every pydantic bound is a `Validate` check that names its field.
     - Integers are `i64` with a bound, as `FrameKeypoints.frame_index` is, so a negative sample is
       refused by name rather than as a serde type error.
     - Every `Option` is `skip_serializing_if`, which is `save_audio`'s `exclude_none`, `clip`
       included.
     - `detector_version` defaults to 0 and has no bound, as Python's has none.
     - It is not `trigger::Strike`, and the module doc says why.
   - **`crates/storage/src/audio_io.rs`**, the twin of `storage/audio_io.py`:
     - `audio_path(swing_dir, Role)` gives `{role}.audio.json`.
     - `load_audio` is **strict**, as Python's raises. A missing file is `Io`. Bad JSON or a broken
       bound is `Unreadable`. A non-object is `Refused`, with Python's own message and
       `type().__name__`.
     - `save_audio` validates first (`Invalid`, nothing written), creates the parent directory,
       and writes atomically.
   - **`crates/lab`** (package `golf-lab`, lib `golf_lab`, bin `golf-lab`):
     - The lib is its crate doc alone.
     - The bin copies `golf-core`'s `usage`: `--help`, `-h` and `help` print to stdout and exit 0.
       A missing or unknown verb goes to stderr and exits 1.
     - **No dependencies.** Its dev-dependencies are `storage`, `serde_json` and `tempfile`, all
       already in the lock, so `Cargo.lock` gained the `golf-lab` package and nothing else. It is a
       workspace member.
2. **The gates.**
   - `crates/contracts/tests/python_schemas.rs` pins the three shapes to frozen Python's
     `audio_file.schema.json` (keys, required, `null`) and pins the `exclude_none` write.
     `the_detector_version_is_frozen_pythons` reads `AUDIO_DETECTOR_VERSION = ` out of
     `contracts/audio.py`. The reason: a Rust bump would make the frozen `audio_for` re-detect and
     overwrite every file Rust wrote. M40 deletes the pin with the file.
   - `crates/lab/tests/audio.rs` is ignored, because it needs `data/`. It reads every stored
     `*.audio.json` and writes each into a temporary directory. It holds the written JSON
     structurally to the stored JSON, and holds the re-read to the first read. It fails on a
     missing directory or on no files found. **30 of 30 round-trip.**
   - `crates/lab/tests/edges.rs`: nothing depends on `golf-lab`, with positive controls. Comments
     are skipped, because `crates/ocr/Cargo.toml` names `golf-lab` in prose.
     `crates/lab/tests/golf_lab.rs` pins the argument handling.
   - Unit tests: 8 in `contracts::audio` (each bound refused by name, the edges admitted, the
     defaults, the omitted `None`s, the built file's path, NaN) and 6 in `storage::audio_io`.
3. **Measured once with a throwaway example, then deleted.**
   - All 30 files were written by `save_audio` into the scratchpad. Frozen Python's `load_audio`
     read each one back equal to the stored model, 30 of 30.
   - The Rust-written bytes are also **byte-identical** to the stored files, 30 of 30. That is a
     coincidence, not a contract: none of these files holds a float that pydantic writes in
     exponent form, and both writers use LF line endings here. The gate stays structural.
4. **Deliberate departures from Python, each in `audio_io`'s module doc.**
   - The write is atomic: a `{role}.audio.tmp` beside the file, then a rename. Python's is a
     `write_bytes`.
   - The writer validates first.
   - A bare `NaN` or `Infinity` is `Unreadable` here. `json.loads` reads it as a float, so Python
     refuses it as "not an object".
   - `indent=None` is not ported. Nothing passes it.
5. **For P21.**
   - **The mapping.** `trigger::detect_strikes` returns `trigger::Strike` (`usize` sample, no
     bounds, no edge to `contracts`). The lab maps each one to `contracts::audio::AudioStrike` and
     stamps `AUDIO_DETECTOR_VERSION`. Write through `save_audio`, which validates. The lab needs
     `trigger` as a library dependency, not `golf-trigger detect`, whose JSON body is not an
     `AudioFile`: `sample_rate` and `sample_count` sit at its top level.
   - **A second copy of the constant.** `crates/trigger/tests/conformance.rs` holds its own
     `AUDIO_DETECTOR_VERSION: u32 = 2`. `trigger` has a Python caller and no `contracts` edge.
     Either give it a `contracts` dev-dependency to read the one constant, or record the drift
     risk.
   - **The strict reader.** `audio_for` calls `load_audio` with no `try`, and so does its caller
     at `api/pipeline.py:1177`, so a broken cache raises out of `analyze_swing_dir`. Whether
     `golf-lab` stops the bundle the same way is P21's call. The store answers with a
     `StoreError` either way.
   - **Dependencies.** Move `storage` from `[dev-dependencies]` to `[dependencies]` when the lib
     first calls it, and add `contracts` and `trigger` there.
   - **The ignored gate.** Run it with `cargo test -p golf-lab -- --include-ignored`.
6. **For P24.** P1 item 14's question, whether a release binary needs `DirectML.dll` beside it,
   was answered for `golf-ocr` in P7 item 10. `golf-lab` does not link `ocr` yet. The first verb
   that does should run the release `golf-lab` from outside `target/` once.
7. **For P35.**
   - The workspace is thirteen crates now. `CLAUDE.md` still says "Ten crates".
   - `contracts`' crate doc names `audio` in two places.
   - ARCHITECTURE §1 should gain `golf-lab`, and §4 `storage::audio_io` beside the other stores.
   - `golf-lab`'s `USAGE` names the verbs still to come. Each verb's phase rewrites it.
8. **Green.**
   - **`cargo test`** passed for the whole workspace: 1,463 passed, 0 failed, 10 ignored. That is
     P19's 1,443 plus 20 new tests: 8 in `contracts::audio`, 2 in `python_schemas.rs`, 6 in
     `storage::audio_io`, and 4 in the lab's `edges.rs` and `golf_lab.rs`. The tenth ignored test
     is the lab's audio gate.
   - **`cargo test -p golf-lab -- --include-ignored`**: 5 passed.
   - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**.
   - **`pytest tests/test_docs_truth.py`** passed, run because this file was edited.
   - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files, and no `.tmp` is
     left under `data/processed`.
   - **Diffs:**
     - nothing under the ten held families;
     - no new `spec/declarations/` file;
     - no `src/`, `scripts/` or `tests/` file touched by this phase.
   - **Scratch:** the example, its output and the test log are deleted.

### P21 (2026-10-22) — strike audio in Rust: stopped on the gate, which fails only on ffmpeg's version; answered (a), resumed and done

**How this phase stopped.** Everything in item 1 landed, and everything but the one ignored gate is
green (item 7). The gate, `crates/lab/tests/strikes.rs`, passes 30 of 30 views under `.venv`'s
ffmpeg 7.1 and fails 26 of 30 under `PATH`'s 9.0.2, on three float fields and nothing else (item 3).
D17 makes 9.0.2 the lab's ffmpeg, and the Done line asks a forced detection to reproduce the stored
strikes, which 7.1 wrote. The plan does not say which wins, and choosing a tolerance would loosen a
gate, so the phase stopped and asked (item 5). The State cell stays 🔄.

1. **What landed.**
   - **`crates/lab/src/ffmpeg.rs`**, the twin of `audio/ffmpeg.py`:
     - `Ffmpeg::find` goes through `ocr::decode::find_ffmpeg`, as P2 item 9 asked, and
       `Ffmpeg::at` takes a path.
     - `decode_audio` and `video_start_seconds` pass frozen Python's arguments verbatim.
     - `AudioClip` has no `camera_id`, because `audio_for` never reads it back.
     - `AudioError` is `Missing`, `NoAudioTrack` or `Failed`, each with Python's message.
       `FfmpegMissing` names every place looked.
     - The `pts_time` regex is parsed by hand. Like `re.search`, it skips an occurrence that does
       not match.
   - **`crates/lab/src/audio.rs`**, the twin of `audio_for`, `_frames_derived` and `_frame_of`:
     - The entry point is `audio_for(swing_dir, manifest, role, AudioOptions { fps, force }, ffmpeg:
       Result<&Ffmpeg, &FfmpegMissing>, log, notes) -> Result<Option<AudioFile>, StoreError>`.
     - `frames_derived` answers `None` where Python returns the same object.
     - Every log line and note is Python's text. Floats print through `pyfmt::repr`, and the
       video-start milliseconds through `pyfmt::fixed(_, 0)`.
   - **Edges.**
     - `golf-lab` gained `contracts`, `storage` (moved up from dev), `trigger`, `ocr` and `pyfmt`.
       All are path crates, so `Cargo.lock` gained no package.
     - `ocr` is the edge that puts `ort` in `golf-lab`'s closure, as P1 item 14 planned.
     - `crates/trigger` gained a `contracts` **dev**-dependency. Its `tests/conformance.rs` now
       reads `AUDIO_DETECTOR_VERSION` from `contracts::audio` (P20 item 5's first option), with
       `detector_version` as `i64`.
   - **Pins.** 18 unit tests: 7 in `ffmpeg`, 11 in `audio`. They cover the frame floor and the
     none-before-the-first-frame case, `frames_derived`'s four no-op guards and its no-re-derive
     case, and the cache hit with and without a write-back. They also cover the older detection
     kept with its note, no ffmpeg, another clip's cache, `force`, a failed decode and a broken
     cache. The ignored gate is `tests/strikes.rs`.
2. **P20 item 5's calls, answered.**
   - **The strict reader.** A broken cache is `Err(StoreError)` and stops the bundle, as Python's
     raise does, and so is a failed write.
   - **The second constant.** There is now one copy (item 1).
   - **The library edge.** The detector is linked in, so Python's mid-run `TriggerUnavailable`
     branch has no twin. "Can this machine listen?" is a question about ffmpeg alone. Its `why` is
     `FfmpegMissing`'s text, for which Python has no wording.
3. **The gate, measured.** Each bundle is copied into a tempdir, and nothing under `data/` is
   written. Each view runs twice. First unforced, over a copy of the stored file. That run must be
   a clean cache hit: the stored value exactly, no write, one log line. That part holds on 30 of 30.
   Then forced.
   - **Under 7.1** (`GOLF_FFMPEG` set to `.venv`'s binary): **30 of 30 pass.** Every exact field
     matches, and the worst float difference is a relative **2.1e-15**. The port is exact.
   - **Under 9.0.2** (`PATH`): on **all 30**, every strike's sample, frame and rank match. So do
     `sample_rate`, `source_sha256`, `stream_index`, `fps`, `video_start_s` (0.107, 0.117, 0.105
     and 0.125 s on clips 1, 7, 9 and 11), `detector_version` and the strike count. **26 of 30 fail** on three
     fields alone:
     - `duration_s` on 26 views, worst relative 1.25e-3;
     - `confidence` on 55 strikes, worst 6.1e-4;
     - `prominence` on 46 strikes, worst 2.5e-5.

     The median float difference is 8.6e-6.
   - **Nothing downstream reads those three fields.** `analyze_swing_dir` passes on only
     `strike.frame`, and no other module reads a strike's confidence or prominence.
4. **The PCM, 9.0.2 against 7.1**, the comparison the Done line asks for. On 2026-08-23/9, the
   down-the-line clip is bit-identical. On the face-on clip, 9.0.2's 581,040 samples are 7.1's first
   581,040, and 7.1 has 528 more. The audio track's edit list reads `media time: 2112, duration:
   581040`. Taken over all 30 clips:
   - **9.0.2 ends each audio track where its edit list ends**, on all 30: its sample count equals
     `ffprobe`'s `duration_ts`. 7.1 decodes every AAC frame after the priming whole (570 × 1024 −
     2112 = 581,568 on the face-on clip). So on 26 of the 30 it runs 16 to 976 samples longer.
   - The four whose lengths agree are 2026-08-23's down-the-line clips 1, 7, 9 and 11. These are
     §E2's four, with the 19200 timescale and the late video.
   - Over the samples both versions decode, 27 of 30 clips are bit-identical. On each of the other
     three (2026-08-23's down-the-line clips 3, 7 and 8), one sample differs by one level. On clip
     7, that sample is the only difference, and its forced detection passed under 9.0.2.
   - 9.0.2 is the more faithful decode. `audio/ffmpeg.py` says the decode applies the edit list,
     and 7.1 applied only its start.
5. **The question, for the user.** What should the committed gate hold, given item 3?
   - **(a) `PATH`'s ffmpeg, as D17 says.** Samples, frames, ranks and the clip block stay exact.
     `duration_s`, `confidence` and `prominence` are declared by name in the test's doc and held to
     a relative 2e-3, which is the measured worst (1.25e-3) with headroom. The 7.1 pass stands
     once, in item 3. *Recommended*: it gates the decoder the lab runs, and the three fields feed
     nothing.
   - **(b) `.venv`'s 7.1 through `GOLF_FFMPEG`**, exact at RTOL 1e-9 on everything, with 9.0.2's
     difference left as this finding. But then the gate needs a binary M40 deletes.
   - **(c) Both in one test**: exact under 7.1 while `.venv` has it, and (a)'s bound under `PATH`'s.
     This doubles the gate's 75 s.
6. **For P21's resume.**
   - Only `tests/strikes.rs` changes: `compare`, and the module doc's "two strengths" paragraph.
     The library code stands under all three answers.
   - `crates/lab/src/ffmpeg.rs`'s module doc already states item 4 neutrally. If the answer is
     (a), add the gate's bound there.
   - Then run `cargo test -p golf-lab -- --include-ignored` (about 75 s per ffmpeg), tick the
     row, and add the answer to this entry.
7. **Green so far.**
   - **`cargo test`**: 1,481 passed, 0 failed, 11 ignored. That is P20's 1,463 plus the 18 unit
     tests, and the eleventh ignored test is `strikes.rs`.
   - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**.
   - **`cargo test -p golf-lab --test audio -- --include-ignored`** passes 30 of 30. The strikes
     gate fails under 9.0.2, as item 3 says, and passes under 7.1.
   - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files, and no `.tmp` is
     left under it.
   - **Diffs:** nothing under the ten held families, no new `spec/declarations/` file, and no
     `src/`, `scripts/` or `tests/` file touched by this phase.
   - **Scratch:** the measurement scripts, the PCM dumps and the logs are deleted.
8. **For P25.**
   - Call `Ffmpeg::find()` once per run and hand `.as_ref()` to each view.
   - The engine's strikes are `audio.strikes`' `frame`s that are `Some`, as at `pipeline.py:1193`.
   - The "no ball strike in this clip" log line and its note belong to the caller, not to
     `audio_for`.
   - `--force-audio` is `AudioOptions.force`.
   - `fps` is the view's `KeypointsFile.clip.fps`. With it, every stored file is a cache hit with
     no write-back (item 3).
9. **For P35.**
   - ARCHITECTURE §1 should name `golf_lab::{ffmpeg, audio}`.
   - The ADR-035 addendum should record that the lab's ffmpeg (9.0.2) ends audio at the edit
     list, and should give the answer to item 5.
10. **Resumed after the user's (a)** (the Decided line in P21's section). The resume found item 1's
    work in the tree as item 6 described it, and kept all of it. Nothing had been written after
    the stop but this file.
    - **`crates/lab/tests/strikes.rs`** now holds (a). It gates the lab's ffmpeg (`Ffmpeg::find`:
      `GOLF_FFMPEG`, then `PATH`), not a named binary. Exact: every strike's sample, frame and
      rank, the clip block (sample rate, sha, stream, fps, video start), the detector version and
      the strike count. `DECODER_RTOL = 2e-3` covers `duration_s`, `confidence` and `prominence`,
      each named in the module doc with its measured worst under 9.0.2. The doc also records the
      7.1 pass at 2.1e-15 as a measurement that is not part of the gate. The audio family's
      1e-9 is gone from this file, and its 1e-12 `ATOL` stays as the floor for a zero.
    - **`crates/lab/src/ffmpeg.rs`**' module doc now states the bound: what the decoder
      difference moves in a stored file, and what the gate holds.
    - **One thing the resume added: `crates/lab/tests/ffmpeg.rs`.** `AudioError::NoAudioTrack`'s
      doc cited a `tests/ffmpeg.rs` that did not exist. The resume made it true rather than
      deleting the claim, because no stored clip reaches those branches (Python's docstring:
      "an unmeasured path is the one that rots"). It is the twin of
      `tests/audio/test_ffmpeg_source.py`, with Python's fixtures built by the lab's ffmpeg. It
      has six ignored tests ("needs an ffmpeg"):
      - the mono 48 kHz decode;
      - the edit list applied (fewer samples than a decode with `-ignore_editlist`, by at least
        10 ms);
      - the stream asked for;
      - no audio track, and an absent stream index, each `NoAudioTrack` with Python's message,
        and garbage `Failed`;
      - a late video's 0.1 s offset, and a measured 0.0;
      - `None` for a clip with no video.

      A fixture that cannot be built **fails**, where Python's skips. An ignored test runs only
      when asked for, and a vacuous pass would say a branch was checked when it was not. All six
      pass under 9.0.2 in about a second. `lib.rs`'s crate doc names the file.
11. **Green, at the close.**
    - **`cargo test -p golf-lab -- --include-ignored`** passes everything. The strikes gate
      passes **30 of 30 views** under `PATH`'s ffmpeg (9.0.2, `GOLF_FFMPEG` unset), with a worst
      relative float difference of **1.250e-3** against the bound of 2e-3, in 71–77 s. The
      round-trip gate passes 30 of 30, and the six ffmpeg tests pass.
    - **`cargo test`**: 1,481 passed, 0 failed, 17 ignored. That is item 7's count, plus the six
      new ignored ffmpeg tests.
    - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**.
    - **`pytest tests/test_docs_truth.py`** passed, run because this file was edited.
    - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files, and no `.tmp` is
      left under it.
    - **Diffs:**
      - nothing under the ten held families;
      - no new `spec/declarations/` file;
      - no `src/`, `scripts/` or `tests/` file touched by this phase.
    - **Scratch:** the test log is deleted.
12. **For P22.** `golf-lab`'s ignored gates now number three files (`audio.rs`, `strikes.rs`,
    `ffmpeg.rs`). `cargo test -p golf-lab -- --include-ignored` runs about 80 s, almost all of it
    `strikes.rs`. Run `--test <name>` alone when only the new gate matters.

### P22 (2026-10-22) — pose in Rust: every stored view a cache hit with nothing spawned, and a forced run exact

The precondition held: `data/models/pose_landmarker_heavy.task` is on disk, and `.venv` imports
`cv2` and `mediapipe`.

1. **What landed.**
   - **`crates/lab/src/pose.rs`** is the twin of `keypoints_for`, `_older_pose_note`,
     `_band_estimator_note` and `pose/estimator.py::pose_estimator_name`:
     - `keypoints_for(swing_dir, manifest, role, force, &mut Sidecar, log, notes) ->
       Result<Option<KeypointsFile>, KeypointsError>`. `KeypointsError` is `Store` (a broken
       cache), `Pose` (a job that did not pose) or `Write`. Each one stops the bundle, as Python's
       raise does.
     - `Sidecar` holds a `pose::Pool` that starts **on the first view that needs posing**. A start
       that fails is remembered, so a second view does not pay a second handshake.
       - `Sidecar::configured()` uses `crates/pose`'s `PoolConfig::default()` (C3). Its variant is
         `GOLF_POSE_MODEL_VARIANT`, else `DEFAULT_POSE_VARIANT`. A test reads `config.py`'s
         default out of the source, so the two cannot drift.
       - `Sidecar::new(variant, PoolConfig)` sets the variant on the workers' process and on every
         job.
       - `estimator()` and `started()` complete the type.
     - `older_pose_note`, and `band_estimator_note(estimator)` over `band_estimator_note_against`.
   - **`crates/storage/src/keypoints_io.rs`**: `keypoints_path` and `load_keypoints`, the twin of
     `storage/keypoints_io.py`'s reader.
     - It reads both shapes, the envelope and the legacy bare array (`clip: None`). It is strict, as
       `audio_io` is, and parses once, telling the shape from the first byte.
     - There is no writer: `pose::writer::write` is the artifact's one spelling (ADR-033 clause 8).
     - `python_type_name` moved from `audio_io` into the crate root, so both strict readers share
       one copy.
   - **`analysis::benchmarks::distributions::dataset_pose_estimators()`** returns `golfdb_v1.json`'s
     `dataset.pose_estimator` (`str | list | None`, as a list). It is the one field of that block
     Rust reads, and `benchmarks/mod.rs`' "what did not come across" now says so.
   - **`crates/pose` changed in one shape: `Outcome::Done(KeypointsFile)` became `Outcome::Done {
     keypoints, claimed_frames }`.** The acceptance's frame count (the container's claim) is the
     half of Python's short-decode check that only the worker can see. `Worker::run` used to drop it
     once the deadline was derived. Ten sites moved, and one new test in
     `tests/worker.rs` checks the count. The stub worker gained three modes for the lab:
     `short_decode`, `empty_clip` and `hashing`. The crate doc now names `golf-lab` as its first
     caller.
   - **Edges.** `golf-lab` gained `pose` and `analysis`, both path crates, so `Cargo.lock` gained no
     package. `file_name` moved into `golf_lab`'s root for `audio` and `pose` to share.
2. **How Python's branches map, and the five departures.** The cache rule, every log line and
   every note Python writes are Python's text. The departures:
   - **"Cannot pose" is the pool failing to start**: no interpreter, the `vision` extra or the model
     absent, a bad variant, or a protocol mismatch. This is Python's `ImportError` branch. Its
     `why` is `PoseError`'s text, which names what was looked for. Python names the extra, so three
     sentences are new: the log lines `{role}: cannot re-run pose ({why}), keeping the older run`
     and `{role}: no pose — {why}`, and the note `no pose for {role}: {why}`. These follow P21's
     pattern for a missing ffmpeg. `_older_pose_note`'s `because` is the same `why`.
   - **A job failure is `Err`** (Python raises), after `crates/pose`'s one retry where clause 5
     allows it. The exception is `clip_empty`, which is Python's `if not frames` note.
   - **What is written is Python's stamp.** `source_sha256` is the manifest's and `pose_estimator`
     is the configured one. Two new checks cover where the worker disagrees:
     - A worker hash that differs from the manifest's is a **note**. Python never hashes the clip,
       so it could not see this.
     - A worker estimator that differs is **refused** (`Err`), because an artifact stamped with an
       instrument that did not measure it is what the cache key exists to prevent.
   - **The variant is not validated in Rust.** `resolve_variant` owns the list (clause 2), so a
     typo reaches the lab as the worker's `bad_variant`. No stored file is ever a hit under a
     misspelt estimator, so every view keeps its older pose, with a note naming the refusal. Python
     raises before posing anything.
   - **`.env` is not read**, as `golf-core`'s `DataDirs` does not read it. An empty variable counts as
     unset.
3. **The gates, measured.**
   - **Every stored view is a cache hit, and nothing is spawned** (`tests/keypoints.rs`, ignored).
     - The 15 bundles' manifests and keypoints were copied into scratch, with an empty stand-in for
       each clip. A miss would therefore have reached the sidecar rather than the missing-clip
       branch.
     - All **30 of 30** views returned the stored file. Each said Python's one log line, added no
       note and rewrote nothing. `Sidecar::started()` was still `false` at the end, under the lab's
       own `Sidecar::configured()` (`mediapipe:heavy`).
     - The same test holds `band_estimator_note("mediapipe:heavy")`'s sentence to every stored
       `analysis.json`: **15 of 15** carry it exactly, which ties the ported text to what Python
       wrote.
   - **One forced run, on the smallest stored clip.** That is `2026-08-23/11` face-on,
     `face_on.21831919bc67.MOV`, 38,911,517 bytes, chosen by size at run time. It ran through
     `Sidecar::configured()` in a scratch copy.
     - **344 frames, equal.** So are the clip block, the estimator, every `frame_index`, every
       `camera_id` and every landmark count.
     - **0 frames and 0 landmark values differ, and 0 timestamps differ.** The written file equals
       the stored one as JSON values. This is M23 P7's result again, now through `golf-lab`.
     - Python's two log lines, and no note.
     - The written file reads back as the value returned, and an unforced second run is a cache
       hit.
     - It took 76.9 s the first time, beside a build, and the whole file took 33 s when it ran
       alone. The differences are recorded and not bounded, as the Done line says.
   - **`tests/sidecar.rs`**, 13 tests against the stub worker, not ignored. They cover:
     - no role;
     - a current cache with no spawn;
     - another clip's cache, and the legacy bare array;
     - another estimator's cache, and an unrecorded one;
     - `force`;
     - the superseded run kept when the clip is gone, and when the machine cannot pose;
     - with nothing to keep, a missing clip and a machine that cannot pose (the second ask is
       answered from the remembered refusal);
     - `clip_empty`;
     - the short decode;
     - the worker-hash note;
     - `clip_unreadable` and `pose_failed` as `Err`;
     - the estimator refusal;
     - a broken cache.
4. **For P25.**
   - Make one `Sidecar::configured()` per run and hand `&mut` to each view's `keypoints_for`, in
     `VIEWS` order. A face-on `None` is the bundle's error, as at `pipeline.py`'s
     `analyze_swing_dir`. `--force-pose` is `force`.
   - Push `band_estimator_note(&sidecar.estimator())` once per bundle, after the missing-roles
     note and before pose, which is Python's order.
   - The default pool's second worker idles, because views are posed one at a time. Submitting both
     at once would use it, but would change Python's log order (C8). P22 left it sequential.
   - `audio_for`'s `fps` is the returned view's `clip.fps` (P21 item 8).
5. **For P26.** In every stored `analysis.json`, the band note sits **after** the engine's
   impact and alignment notes, although `analyze_swing_dir` builds it first. So the writer
   appends the pipeline's notes after the engine's. The loft note (`club 3w is not in this
   golfer's bag…`, on `2026-08-23/7` and `/10`) follows the band note. Check that order against
   the stored files when P26 assembles `notes`.
6. **For P35.**
   - `CLAUDE.md` still says `crates/pose` "has no caller". ARCHITECTURE §1 should name the
     following:
     - `golf_lab::pose` as the sidecar's first caller;
     - `storage::keypoints_io`;
     - `Outcome::Done`'s new field.
   - The ADR-033 addendum should record the lab as the first caller, with item 2's five
     departures.
7. **Green.**
   - **`cargo test`**: 1,504 passed, 0 failed, 19 ignored. That is P21's 1,481 and 17, plus 23
     new tests (4 `keypoints_io`, 1 `distributions`, 4 `pose` unit, 13 `sidecar`, 1 `worker`) and
     the two ignored gates.
   - **`cargo test -p golf-lab -- --include-ignored`** passes everything, in about 75 s (`strikes.rs`
     40 s, `keypoints.rs` 33 s).
   - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**. `cargo doc -p pose`
     warns twice, on two links to private items that were there before this phase.
   - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files, and no `.tmp` is
     left under it.
   - **Diffs:**
     - nothing under the ten held families;
     - no new `spec/declarations/` file;
     - no `src/`, `scripts/` or `tests/` file touched by this phase.
   - **Scratch:** the edit helpers and the logs are deleted.

### P23 (2026-10-23) — the shot lookup in Rust: 13 stored shots re-read in a dry run, and only the declared values move

**Resumed after a cut-off.** The session started 2026-10-22 21:33 left `crates/lab/src/shot.rs`,
`tests/shot_lookup.rs`, `tests/reread.rs`, the `screen` edge in `crates/lab/Cargo.toml` and the
`shot` line in `lib.rs`. All of it compiled and was correct, and it was kept. The resumed session
added one branch that it lacked (item 2), and ran every gate.

1. **What landed.** `crates/lab/src/shot.rs` is the twin of `api/pipeline.py::_shot_for` with
   `importer.py::import_screen` from the bytes on:
   - `shot_for(swing_dir, manifest, &ShotOptions, &mut dyn PhotoReader, log) -> Result<ShotFor,
     StoreError>`. `ShotFor` is Python's `(shot, note)` plus an `Origin`: `Nothing`, `Stored`,
     `KeptOlder`, or `Read { replacing, written }`. `Err` comes only where Python raises, when a
     stored file is not a shot.
   - `ShotOptions` holds `shots_dir`, `device`, `min_confidence`, `force_ocr`, `skip_ocr` and
     `dry_run`. `configured` reads `GOLF_LAUNCH_MONITOR_PROFILE` and `GOLF_OCR_MIN_CONFIDENCE`, and
     treats an empty value as unset. A unit test reads the defaults (`hd_golf`, 0.6) out of
     `config.py`.
   - `PhotoReader` is the pixel half, `build_recognizer` as `ready()` and `prepare_screen` as
     `prepare()`. `OcrReader` is the real one: it opens `data/models/ocr/` on the first photo that
     needs reading and remembers a refusal.
   - The rule: the store is read under the manifest's sha. A shot that `parse_is_current` passes is
     served (`"  shot_screen: {id} from the shot store"`). A stale one is logged (`"… was read by
     an unversioned screen parser, re-reading with screen parser 1"`) and re-read from the
     bundle's photo by `ocr::read_photo` and `screen::read`, **keeping its `shot_id`,
     `session_id`, `timestamp` and `image_path`**. Only the parse is new. A store miss, or
     `force_ocr`, is a new import stamped as `import_screen` stamps one.
   - Every Python note is word for word. When a stale shot cannot be re-read (OCR skipped, photo
     missing, no models, the read raised, or the write failed), the stale shot is kept and
     `older_shot_note` says which parser read it and why. A re-read that finds nothing attaches
     nothing (Python's `failed` note) and leaves the old file in place.
2. **Added on resuming: `import_screen`'s second lookup.** Python looks the store up twice: once in
   `_shot_for` under the manifest's sha, and again in `import_screen` under the digest of the bytes
   on disk. The two differ only when the photo changed after the manifest was written. The cut-off
   code read such a photo again without the second lookup. Now, after a hash note, the shot under
   the manifest's sha is set aside as another photo's. The shot under the bytes' digest then
   follows the same rule: if current, it is Python's `cached` (logged `"  shot_screen: cached"`,
   no read). If stale, it is re-read keeping its identity. If absent, the photo is a new import.
   `load_profile` also now runs before the bytes are read, in Python's order: it is an argument
   to `import_screen`. Two new `shot_lookup.rs` tests cover the branch. No stored bundle reaches
   it.
3. **The gate** (`tests/reread.rs`, ignored) runs `shot_for` with `dry_run` over every stored
   bundle, using the real store and `OcrReader::configured()`. It hashes the store before and after.
   **15 bundles, 13 photos, 15 reads, 356 s in a debug build.** The shared photo is read three
   times, because a dry run writes nothing. The results:
   - **All 13 stored shots are unstamped.** Each is a dry-run re-read of the stored shot with the
     three-line log and no note. `parser_version` goes 0 → 1, and `fields_present` goes from absent
     to the bay layout's tiles (`impact_position_v` in, `bounce_and_roll` out).
   - **Identity, `device`, `needs_review`, `corrections`, `image_sha256` and `image_path` are
     unchanged on all 13.** Every value is identical except `DECLARED_VALUES`: `2026-08-10-1`'s
     `shot_type` (`CENTER SLIGHT FADE` → `SLIGHT FADE`) and `impact_position` (null → `CENTER`,
     P7 item 16, which points at `crates/ocr/tests/read.rs`' `DECLARED_DIFFERENCES`), seen 3 times
     each, and `2026-08-23-1`'s `impact_position` (`HEEL` → null), seen once. If a declared entry
     is never seen, the gate fails.
   - **Every provenance change is a kind finding 2 or P6–P7 lists:**
     - 12 shots gain the V tile's blank line.
     - On `2026-08-10-1` that same line replaces `Impact Position: no value text under the label`,
       because the tile is now read.
     - `2026-08-23-1` gains the two tie lines, which name `'Impact Position'` twice (P7 item 6), and
       drops its `Impact Position` raw text.
     - The other `raw_fields` changes are text that moved while the value did not (P6 item 8): a
       degree sign, a unit (`mph`, `yds`), `17.5 o`, and `1.00 CENTER` → `1.00` in Smash Factor.
   - `parse_confidence` moves −0.015 to −0.018 on 11 shots, −0.041 on `2026-08-23-1` (M34's tie),
     and +0.004 on `2026-08-10-1` (the V label read; P7 item 6). All of these are inside
     `CONFIDENCE_BOUND` (0.05).
   So D7's "or a finding and a stop" did not trigger.
4. **Why the identity is kept** is measured, not argued. The gate prints the stamp a new import
   would have made. For two of the three bundles sharing `2e81ab05828e…`, it is another id and
   another time (`2026-08-07-aaron1-1` at `2026-08-08T00:27:34.996100Z`, `2026-08-09-2` at
   `2026-08-09T21:08:28.933602Z`). The other 11 photos get the same time either way.
5. **Departures from Python, each in `shot.rs`'s module doc:** a stale shot is re-read rather than
   served; an unreadable stale shot is kept with a note; a photo whose bytes changed gets a note;
   a re-read keeps its identity; and there is a dry run. Python has none of these.
6. **For P24**:
   - `read_screen`, the stamp, is private and takes a `SwingManifest` for a new import's identity
     (`{session}-{swing}`). `import-photos`' store-only import stamps `{session}-{digest[:12]}`
     (`golf-ocr`'s rule, P7 item 11), so factor the identity out of `read_screen` rather than
     writing a second stamp.
   - A new import's `image_path` is `image_path.display()` of the path as given. Python's
     `str(Path)` is a normalized Windows path, and every stored shot carries an absolute
     backslashed one (`C:\Users\…\data\processed\sessions\…`). Pass an absolute swing directory if
     the spelling is to match.
7. **For P25**:
   - Log `"\nShot data:"`, call `shot_for`, then log `"  {note}"` and push the note into `notes`,
     as `analyze_swing_dir` does.
   - Hold one `OcrReader` across bundles, so the models open once.
   - The shot D20's gate sees on a dry run is item 3's: the three declared values against the
     **stored** shot, the stamp on all 15, and no identity change.
   - Budget about 24 s a photo in a debug build for the re-read.
8. **For P29**: the real write re-reads 13 photos, not 15. The first of the three bundles sharing
   `2e81ab05828e…` writes the re-read, and the other two find it current (`Origin::Stored`, one log
   line). Each `shots/*.shot.json` changes as item 3 lists, and the stored shots are not renamed.
9. **Pins**:
   - `shot.rs` has 3 unit tests (the defaults from `config.py`, the environment, the parser's
     name).
   - `tests/shot_lookup.rs` has 16 tests, on any machine. A reader replays
     `spec/vectors/screen/corpus/2026-08-23-2.json`'s boxes, so a re-read is held exactly to that
     vector's `expected.shot`.
   - `tests/reread.rs` is the ignored gate.
10. **Green.**
    - **`cargo test`**: 1,523 passed, 0 failed, 20 ignored. That is P22's 1,504 and 19 plus 19
      tests and the one gate.
    - **`cargo test -p golf-lab -- --include-ignored`** passes everything (`reread` 356 s,
      `strikes` 41 s, `keypoints` 37 s).
    - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**.
    - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files, and there is no
      `.tmp` file.
    - **Diffs:** nothing under the ten held families or `spec/declarations/`, and no `src/`,
      `scripts/` or `tests/` file touched by this phase.
    - **Scratch:** the logs are deleted.

### P24 (2026-10-23) — `golf-lab import-photos`: the script's import, and photo entries the corpus pools

1. **What landed.**
   - `crates/lab/src/import.rs` is `scripts/import_shot_screens.py`'s twin, plus `--into-session`,
     which has no Python twin. `import_photos(&Request, &mut dyn PhotoReader, now, out, err) ->
     io::Result<u8>` prints as it goes and answers the exit code.
   - `crates/lab/src/shot.rs` gained `import_photo`, which is `import_screen` alone, its outcome
     `Imported` (`Cached`, `Read { shot, replacing, written }`, `Nothing`), and `NewImport { session_id,
     shot_id: Option }`, which is `import_screen`'s `session_id` and `shot_id` override. P23 item 6
     asked for this: `read_screen` now takes a `NewImport` where it took the manifest, so
     `{session}-{swing}` (from `shot_for`) and `{session}-{digest[:12]}` (from a store-only import)
     are one stamp rule. `shot_for` is otherwise unchanged, and its 16 tests pass as they were.
   - `golf-lab import-photos` is the binary's first verb. `USAGE` names it and the verbs still to
     come.
   - `storage::bundle_store::SwingBundleStore::next_swing_id(session)` is new and public: the id the
     automatic rule would open. It has one unit test. Nothing it returns reaches a vector.
   - `crates/lab/Cargo.toml` gained the `golf-core` edge, for `session::run_shot_session`. It brings
     `feedback` into the lab's closure, and nothing with `ort`.
2. **The store-only import is the script's.** It keeps the same flags (`PATHS`, `--session`,
   `--device`, `--out`, `--min-confidence`, `--force`, `--dry-run`), the same defaults
   (`GOLF_SHOT_SCREENS_DIR`, `GOLF_LAUNCH_MONITOR_PROFILE`, `GOLF_OCR_MIN_CONFIDENCE`,
   `GOLF_SHOTS_DIR`, else `config.py`'s), and the same table, review list, store line and last
   line. The exit codes are the script's too:
   - 0 when clean;
   - 1 when no image is found, the reader cannot run (checked before any photo, as
     `build_recognizer` is), or any photo reads as nothing or is flagged;
   - 2 for an unknown profile, with the `KeyError`'s `repr`, and for whatever argparse refuses.

   It adds `--models DIR`, as `golf-ocr read` has. Five departures, each in the module doc:
   - **A stale stored shot is re-read.** It keeps its identity and prints `re-read`, where Python
     prints `cached`. This is `shot_for`'s rule.
   - **The suffixes are `.jpg .jpeg .png .heic .heif`**, the ones `crates/ocr` decodes. The script's
     are `.jpg .jpeg .png .bmp .webp`. So the warning reads `(not a JPEG, PNG or HEIC photo)` rather
     than `(not an image)`.
   - **A photo that raises fails alone.** It is that photo's `failed` row, with `error: <the
     reader's message, unwrapped>` on stderr (P2 item 19), and the rest still run. Python's loop
     catches nothing, so the first raise ended the script.
   - **Paths are spelled as Python's `str(Path)`** (`spelled`). On Windows a `/` becomes `\`, and on
     every platform a `.` component goes, in the rows, the store line and the stamped `image_path`.
     Git Bash passes `C:/...`, so without this the store line and the stamp differed from the
     script's.
   - **A directory's children are files only**, sorted case-folded on Windows as `WindowsPath`
     sorts them.
3. **Measured against the frozen script** on scratch copies of `2026-08-23/2`'s JPEG and
   `2026-08-23/1`'s HEIC, with a scratch `--out`:
   - The JPEG's row is identical except `conf=0.90` against Rust's `0.88`. That is M34's
     `label_ratio` denominator, the −0.018 P23 item 3 measured on this photo.
   - The script skips the HEIC with `(not an image)`, and Rust reads it.
   - The header, the store line (once spelled) and the last line are identical.

   The scratch is deleted.
4. **`--into-session S --player P --club C`** (all three, or none) opens one **photo entry** per
   photo and analyses it. These are this phase's calls, none of them in the plan:
   - **The checks come first, each exit 2.** `S` must be one plain directory name. `C` is read by
     `parse_club`, refused with `club-profile`'s sentence. `P` is slugged as `--name` is, and
     refused unless the registry holds it; the refusal names who the registry does hold. The
     reason is `parse_club`'s: a refused name costs a retype, and a wrong one files shots under a
     golfer nobody is, with no Rust verb to move them.
   - **Dedupe is by photo, within `S`.** A photo some swing in `S` already holds is `skipped`, with
     the store's shot if there is one.
     - `--force` reads a photo entry's own copy again, stamped `{S}-{N}`, and re-analyses it.
     - A swing's photo (one with a face-on clip) is never touched, forced or not.
     - A forced read that finds nothing leaves the entry and its `shot_analysis.json` as they were.
   - **An entry is opened by name.** It is `S/N` with `N = next_swing_id(S)`, through
     `assign_from_path` with an explicit `swing_id`. The photo is copied into
     `sessions/.incoming/{digest}.{pid}.part`, the copy's mtime is set to the photo's
     (`File::set_modified`, because `screen_timestamp` is the shot's time), and then it is moved in.
     Opening by name is the store doc's advice: the automatic rule would slot the photo into the
     newest swing lacking one, which can be a filmed swing. One test holds this: a filmed swing 1
     with no photo, and the photo lands in 2.
   - **What the entry records.** Golfer and club come from the flags. The intent comes from the
     session's cursor (`load_session_meta`), as an upload stamps it. `original_filename` is the
     photo's name, and `content_type` is chosen by suffix (`image/jpeg`, `image/png`,
     `image/heic`).
   - **The entry's own copy is read**, with `NewImport { S, Some("{S}-{N}") }`, so the stored shot
     names the copy that stays, as a bundle's does.
   - **Analysis goes through `golf_core::session::run_shot_session(S, Some(N), ...)`.** That is
     `golf-core run-shot --session`'s code, so the verb and the corpus agree on what an entry needs,
     and its line is printed indented under the photo's row (C7). A refused analysis is counted
     `not analysed`, and the last line becomes `F unreadable, R flagged for review, A not
     analysed.`
   - **A photo nothing is read from opens no entry.** The entry this run opened is removed
     (`delete_swing`), and a session directory this run created and left empty is removed too.
     The reason: a photo entry with no `shot_analysis.json` is one the corpus lists as
     `not_analyzed`, with the repair `golf-core run-shot`, and that verb would refuse it for having
     no parse.
   - **A dry run writes nothing.** It reads each photo the session does not hold (stamped as a
     store-only import would be, which nothing prints) and says it would open an entry. It opens,
     writes and analyses nothing.
   - **`--session` with `--into-session` is refused**, since the second names the session.
     `--sessions-dir` and `--golfers-dir` are refused without `--into-session`.
5. **Done, as the section words it**, on scratch directories:
   - **storing**: `parsed`, then written under the photo's digest;
   - **dedupe by photo sha**: the same bytes under a second name are `cached` (the store) or
     `skipped` (the session);
   - **skip unless `--force`**: a current shot is not read again, and `--force` reads it as a new
     import;
   - **the exit codes**: 0, 1 and 2 as listed in item 2;
   - **a photo entry Rust's `read_corpus` admits**: one swing pooled, nothing excluded,
     `face_on_sha256` none and `shot_sha256` the photo's. This holds both with the replaying reader
     and, in `tests/import_real.rs`, through the binary with the models.
6. **P20 item 6, answered.** The release `golf-lab.exe` was copied alone into a scratch directory,
   with no `DirectML.dll` beside it. Run from there, `import-photos` read the two photos (the JPEG
   and the HEIC) in **22.8 s**, opening and hashing the models included. Both shots were stored, and
   a second run answered `cached` for both. The binary finds `data/models/ocr/` from the source
   path it was built from, as `golf-ocr` does.
7. **For P25**: hold one `OcrReader` across bundles, as P23 item 7 says. `import-photos` builds one
   per run, and its `ready()` up front is the one model open.
8. **For P27 (C7)**: `golf_core::session::run_shot_session` is the seam re-analysis over photo-only
   entries can reuse, as `import-photos` does. It analyses only from the store, so a photo entry
   whose stored shot is stale (none is yet) wants `shot_for`'s re-read first.
9. **For P31**:
   - `Entries::holding` (the dedupe by photo within a session) and `Entries::open` (copy into
     `.incoming/`, keep the mtime, place by name) are the shapes an export's merge needs.
   - C11 refuses any change of content, which `import-photos` never has to.
10. **For P32: D11 closes the only routes that register a golfer.** In the frozen server,
    `_resolve_golfer` (`get_or_create`) is reached only from `POST /api/sessions/current/golfer` and
    `POST /api/sessions/{s}/swings/{w}/golfer`, and both are on D11's list. The bag routes call
    `_registered`, which refuses a golfer the registry does not hold. `import-photos
    --into-session` refuses an unregistered golfer (item 4). So after P32 nothing in the lab can
    add a new golfer until a Rust verb or M38 does. That is a question for the user at P32, not
    something to settle there.
11. **For P34**:
    - `crates/core/src/session.rs`'s `NoParse` sentence tells the user to run `python
      scripts/import_shot_screens.py {photo} --session {S} --out {shots}`. When P34 deletes the
      script, it should name `golf-lab import-photos`. Check `crates/core/tests/verbs.rs` for the
      text first.
    - The same is true of `UNREAD_SHOT_SCREEN` (P15 item 8).
12. **For P35**:
    - ARCHITECTURE §1 gains `golf-lab import-photos` with both forms and their flags.
    - §4 should say a photo entry can now be written by `golf-lab import-photos --into-session` as
      well as by M35's verbs.
    - The crate graph gains `golf-lab → golf-core`.
    - `SwingBundleStore::next_swing_id` is a public method with no Python twin as a method.
13. **Pins**:
    - `tests/import_photos.rs` has 13 tests, on any machine. They use P23's replaying reader over
      `2026-08-23-2`'s boxes, a scratch golfer `aaron` with the storage family's bag, and the
      binary's refusals. The missing-models case points `--models` at an empty directory.
    - `tests/import_real.rs` is the ignored gate, 63 s in a debug build.
    - `bundle_store.rs` has 1 new unit test.
14. **Green.**
    - **`cargo test`**: 1,537 passed, 0 failed, 21 ignored. That is P23's 1,523 and 20, plus the 13
      tests, the storage unit test and the one gate.
    - **`cargo test -p golf-lab -p storage -- --include-ignored`** passes everything (`import_real`
      63 s, `reread` 355 s, `strikes` 41 s, `keypoints` 37 s). It was re-run after the last edit,
      which added item 4's "left as it was" line and its test.
    - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**.
    - **`pytest tests/test_docs_truth.py`** passes; it was run because this file was edited.
    - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files, and there is no
      `.tmp` or `.part` file. Every run of this phase wrote under the session scratchpad.
    - **Diffs:** nothing under the ten held families or `spec/declarations/`, and no `src/`,
      `scripts/` or `tests/` file touched. The files changed are `crates/lab/{Cargo.toml,
      src/{bin/golf_lab,import,lib,shot}.rs, tests/{import_photos,import_real}.rs}`,
      `crates/storage/src/bundle_store.rs`, `Cargo.lock` (the new edge) and this file.
    - **Scratch:** the photo copies, both stores, the release binary and the logs are deleted.

### P25 (2026-10-23) — the bundle pipeline in Rust: 15 inputs equal to the committed corpus but the re-read shot, and every selection call answered

1. **What landed.**
   - **`crates/core/src/auto_windows.rs`** (new) is the twin of `api/pipeline.py`'s `_pick_swing`,
     `_downswing_seconds`, `_narrate_choice` and `_auto_windows`. Its API is `View { frames, fps,
     strikes }`, `pick_swing`, `downswing_seconds`, `auto_windows(face_on, down_the_line,
     window_face_on, window_dtl, log, notes) -> AutoWindows { face_on, down_the_line, picks }`, and
     `PickCall`. Every sentence is Python's. **P19 item 6's choice: the second option**, "put
     `_auto_windows` somewhere `core` can reach". It is pure, frames in and sentences out, so the
     gate, the re-record and the lab all run one definition, and nothing spawns `golf-lab`.
   - **`crates/core/src/selection_family.rs`** answers `_auto_windows` (`windows_answer`, with
     `wrist_name` the reverse of `wrist_named`). Its private `pick_swing` is gone in favour of
     `auto_windows::pick_swing`. **`NOT_YET` is empty** (`[(&str, &str); 0]`) and stays, as
     `career_family::CARRIED` stayed at M36 P16. The module doc, `lib.rs`'s "swing selection"
     section and `rerecord.rs`' crate doc and `walk_selection` doc say so. The carry and its guard
     in `walk_selection` are unchanged and now unreached.
   - **`crates/lab/src/pipeline.rs`** (new) is `analyze_swing_dir` up to the engine call, plus
     `_handedness_for`, `_loft_for` and `loft_remedy`:
     - `PipelineOptions` (`shot: ShotOptions`, `golfers_dir`, `club`, both windows, `auto_window`,
       `force_pose`, `force_audio`); `Tools { ffmpeg, sidecar, reader }`, borrowed per bundle;
     - `assemble(swing_dir, &options, &mut tools, log) -> Result<Assembly, PipelineError>`, where
       `Assembly` is `Refused(Box<Refusal { manifest, missing, error }>)` or
       `Ready(Box<Assembled { manifest, missing, input: VectorInput, notes, loft_remedy,
       shot_origin }>)`, and `PipelineError` is `Keypoints` or `Store` (Python's raises);
     - `Assembled::loft_note(&unscored)`, the remedy only when the engine reports `no_club_loft`;
     - `handedness_for`, `loft_for`, `loft_remedy`, `VIEWS`;
     - `corpus_input(&VectorInput) -> CorpusInput { input, frame_offset, source_clip_frames }`,
       `_corpus_vector`'s cut with Python's slice semantics (`python_slice`).
   - **The engine input is `golf_core::VectorInput`**, `run`'s, because a corpus vector's `input`
     is exactly the arguments the pipeline hands the engine.
   - **Edges and the lock.** `golf-lab` gained no crate. `serde_json` moved from its dev- to its
     normal dependencies, and `flate2` (the workspace's) became a dev-dependency for the gate. So
     `Cargo.lock` gained one line, `flate2` in `golf-lab`'s list, and no package.
     `import::spelled` is now `pub(crate)`, for the no-manifest refusal.
   - **`crates/lab/tests/common/mod.rs`** (new) holds `DECLARED_VALUES` (moved out of
     `tests/reread.rs`, unchanged) and `swing_dirs`, which both data gates now share.
2. **The selection family: no case skipped.** `tests/selection.rs` answers all 45 cases' calls by
   value, `_auto_windows` 28 of them, with nothing skipped, so the gate's "every call NOT_YET names
   is still skipped" assertion now reads as "nothing is skipped". The first run was exact. The new
   census test, `the_family_reaches_every_sentence_auto_windows_writes`, fails by name if no case
   reaches one of the following: either decline, either reason, the reverse match, the down-the-line
   pick alone for each of its two whys, either window given, or no second view.
3. **The re-record, after the port.** The `v17` dry run reports **140 vectors (21 engine, 21
   stage, 25 shot, 45 selection, 28 mcp), 0 changed**, now with every `_auto_windows` answer Rust's
   rather than carried. `tests/rerecord.rs`' `a_moved_pick_cannot_leave_its_carried_windows_behind`
   became **`a_moved_pick_moves_the_windows_chosen_from_it`**. An older reason in the windows case's
   pick and in its `_auto_windows` log line is now refused by the gate at
   `expected.results[1].log[0]` when only the pick is declared, and lands when both are. That is
   the guard's job, done by the comparison now that Rust answers both calls.
4. **D20's gate, `crates/lab/tests/inputs.rs` (ignored), on all 15 stored bundles.** It runs on a
   scratch copy of `sessions/` (clips as empty stand-ins), `shots/` and `golfers/`, with the shot's
   `dry_run`, `Sidecar::configured()` and `OcrReader::configured()`. It took 371 s in a debug
   build. What it holds:
   - **Every input key but the shot is equal by value**: frames, strikes, intent (`all`),
     handedness and loft. So are `provenance.frame_offset` and `source_clip_frames`, which means the
     lab's selection chose every stored window.
   - **The shot differs only at its declared paths.**
     - `DECLARED_VALUES` are seen 3, 3 and 1 times. Each is seen on exactly the bundles whose shot it
       names: the three sharing `2026-08-10-1` and `2026-08-23-1`.
     - The ten keys `v17.json` added to `expected.swing.shot` are read from the declaration, not
       restated. Each is at the default that re-record wrote into the engine's echo of the same
       shot, except the stamp: `parser_version` is 1 and `fields_present` is a list, on all 15.
     - The re-read's provenance text (`warnings`, `raw_fields`, `parse_confidence`) is excused by
       path prefix, because `tests/reread.rs` holds each change to its kind.
     - Identity, device, `image_*` and `needs_review` are held equal.
   - **The notes are each stored `analysis.json`'s tail.** The committed engine notes, then the
     assembly's (the band note, on all 15), then the loft remedy where the committed result reports
     `no_club_loft` (`2026-08-23-7` and `-10`) equal the stored `notes` exactly. That is P22 item 5's
     order, now held on all 15.
   - **The lines logged under `Swing selection:`** are the committed `selection/corpus/` case's
     `_auto_windows` log.
   - **Nothing ran or was written**: the sidecar never started, the reader read 15 photos (one per
     bundle, since a dry run writes nothing), and the scratch copy hashes the same before and after.

   The first run failed on one thing, the gate's own: it expected `provenance.corrections` to be
   `[]`, and the default is `{}`. The fix was to read every default from the committed echo, as
   described above, rather than restate it. With that fix, nothing in the data moved and no
   difference went undeclared, so the plan's "or a finding and a stop" did not trigger.
5. **`tests/assemble.rs`** (9 tests, on any machine). These are the branches the stored bundles do
   not reach, on scratch bundles served entirely from caches. The sidecar points at a missing
   worker, ffmpeg is missing, and the photo reader panics if asked. The tests cover:
   - the no-manifest refusal (no manifest, nothing logged);
   - the face-on refusal, carrying the manifest;
   - a complete bundle's whole log and note order, and every engine argument;
   - the loft remedy held back until the engine's `no_club_loft`;
   - no photo, so no loft looked for and the shot note last;
   - the shot-without-strike note, and its absence when nobody listened;
   - both handedness notes and their `\nGolfer:` log line;
   - a missing view (`[collecting]`, the missing note first);
   - given windows with `auto_window` off, and `--club` as the goal's category;
   - `corpus_input` worked by hand on a ten-frame clip.
   `pipeline.rs` adds 2 unit tests: Python's slice, measured in CPython, and the four remedies.
6. **Calls this phase made**, each in `pipeline.rs`' module doc:
   - **The intent is Python's, `PracticeGoal(club=options.club)`, never the manifest's M35
     `intent`.** The category picks bands, and every committed corpus vector carries `all`. A
     swing bundle that `golf-core set-intent` marked is therefore analysed as frozen Python would
     analyse it. Whether `golf-lab analyze` should read that intent is a later decision, not this
     port's.
   - **The dry run is the shot's alone.** Pose and audio write only on a miss.
   - **The instruments are the caller's**, one per run.
7. **For P26.**
   - **The engine call.** `golf_core::run` returns a serialized `Value` with feedback already in
     it, and `bundle_request` is private. P26 needs the typed `SwingBundleResult` so it can extend
     the notes, fold in coaching and drop keypoints and detections on write. So add a `pub` seam in
     core (for example `run_bundle(&VectorInput) -> SwingBundleResult`, with `run` over it) rather
     than re-serializing.
   - **The notes order**, which the gate holds on all 15: engine notes, then `assembled.notes`,
     then `assembled.loft_note(&result.swing.unscored)`, then the coaching note. Log
     `"\nBall flight:\n  {remedy}"` when the loft note lands.
   - **`Refusal.manifest`.** With `Some`, record the failed run in `analysis.state.json`, as
     Python does for a missing face-on view. With `None`, write no state.
   - **`flagged`** is the shot's `needs_review` or a non-empty `missing`.
   - **`PipelineOptions` has no `coaching` yet.** Add it with C5's default.
   - **Build `Tools` once per verb run**: `Ffmpeg::find()`, `Sidecar::configured()` and
     `OcrReader::configured()`.
   - **On a scratch copy, run `assemble` with `dry_run: false`.** The first bundle of the three
     sharing `2e81ab05828e…` then writes the re-read, and the other two find it current, as P23
     item 8 says.
8. **For P27.** `reanalyze` walks `swing_dirs`' order (`tests/common/mod.rs` has the walk the gates
   use) with one `Tools` across all bundles. A photo-only entry has no face-on clip, so `assemble`
   refuses it as Python would. C7's `run_shot_session` is its route, and P24 item 8 applies.
9. **For P30.** `pipeline::corpus_input` is `_corpus_vector`'s cut. The builder can build a
   `VectorInput` from the stored files (windows from `analysis.json`, the shot from its
   `swing.shot`, intent from `swing.intent`, strikes from `audio.json`, handedness and loft through
   `handedness_for` and `loft_for`) and cut it. `_verify_against_stored` is not ported.
   `tests/inputs.rs` shows the cut of the assembled input equals every committed corpus input but
   the shot.
10. **For P35.**
    - CONFORMANCE's selection section should say that `_auto_windows` is answered by
      `golf_core::auto_windows`, `NOT_YET` is empty, and the carry guard is unreached.
    - ARCHITECTURE §1 should name `golf_lab::pipeline` and `golf_core::auto_windows`, and say that
      the `golf-lab → golf-core` edge now carries `VectorInput` and `auto_windows`.
    - The lab crate doc names `pipeline` and both new test files.
11. **For P36.** The `v17` dry run's expected result stands: 140 vectors, 0 changed (item 3). Build
    `golf-mcp` in release first.
12. **Green.**
    - **`cargo test`**: 1,549 passed, 0 failed, 22 ignored. That is P24's 1,537 and 21, plus 9
      `assemble`, 2 `pipeline` unit tests and 1 selection census, and the one new ignored gate.
    - **`cargo test -p golf-lab --no-fail-fast -- --include-ignored`** passes everything (`inputs`
      371 s, `reread` 356 s, `import_real` 64 s, `strikes` 41 s, `keypoints` 37 s).
    - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**.
    - **`v17` dry run**: 0 changed.
    - **`pytest tests/test_docs_truth.py`** passes; it was run because this file was edited.
    - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files, and there is no
      `.tmp` or `.part` file. The gate wrote only under a tempdir.
    - **Diffs:** nothing under the ten held families, `spec/vectors/{selection,mcp}` or
      `spec/declarations/`, and no `src/`, `scripts/` or `tests/` file touched. The files changed
      are:
      - `crates/core/src/{auto_windows,lib,rerecord,selection_family}.rs` and
        `crates/core/tests/{rerecord,selection}.rs`;
      - `crates/lab/{Cargo.toml, src/{import,lib,pipeline}.rs,
        tests/{assemble,inputs,reread,common/mod}.rs}`;
      - `Cargo.lock` (item 1) and this file.
    - **Scratch:** the inspection scripts and the logs are deleted.

### P26 (2026-10-25) — `golf-lab analyze`: cut off twice, resumed and done; the report is the script's, and one live paragraph folded in

**How this phase ran.** The row read `🔄 Started 2026-10-23 09:09`.
- **The first session wrote nothing** after its marker.
- **The second session (2026-10-25, from about 11:00) did the whole phase.** It wrote every file in
  item 14, ran the C8 comparison and the frozen readers, made the live call, and ran every gate in
  item 14. It was cut off after drafting these findings in its scratchpad, before it wrote them here
  or ticked the row.
- **This third session closed it.** It listed what changed after the marker: the files in item 14,
  all written between 11:06 and 11:29, and nothing later. It read them against the Files line and
  kept them as they were. Nothing in them was redone. It then re-checked the work:
  - re-ran `cargo test`, clippy, fmt, the ignored gates, the docs-truth pins, the digest and the
    diffs;
  - re-ran the second session's frozen-readers script over its Rust-written scratch artifacts, with
    the same result (item 6);
  - re-compared the logs and reports that session kept (items 5 and 7).

  It made **no coaching call**. One draft line was wrong and is corrected in item 13: `CLAUDE.md`
  does not name `textwrap`.

1. **What landed.**
   - **`golf_core::run_bundle`** (P25 item 7's seam): the engine and its feedback, typed. `run` is
     `serialize(&run_bundle(input))`. **`serialize` is now `pub`**, because `analysis.json` holds the
     same drop as every vector (`keypoints`, `detections`). So the lab writes through it rather than
     through a second copy of the list.
   - **`golf_core::textwrap`**, moved from `crates/mcp` unchanged except its doc. `golf-lab`'s report
     needs `textwrap.wrap(width=88)` for the coaching paragraph, and the two shells may not depend
     on each other. `golf_mcp::textwrap` is now `pub use golf_core::textwrap`, so every path
     `golf-mcp` used still resolves.
   - **`storage::state::save_analysis(&Value, swing_dir)`**, beside `load_analysis`. It writes
     atomically, where Python's `write_text` does not. It does not create the directory, for
     `save_shot_analysis`' reason. It has one unit test.
   - **`crates/lab/src/coach.rs`** (new):
     - `Coach`, the trait a run asks for the paragraph, with `CoachRequest` and `CoachAnswer`.
     - `Entry`, P17's entry spawned in the repository root as `.venv`'s `python -m
       golf_coach.feedback.coach --session S --swing W --sessions-dir D --shots-dir D`, with stdin
       null and both streams captured.
     - `answer_from`, which reads the entry's two shapes, each only under its own exit code.
       Anything else is a note.
     - `coaching_enabled`, C5's switch (item 4).
   - **`crates/lab/src/analyze.rs`** (new):
     - `analyze_swing_dir`, which runs `assemble`, then Python's steps after the engine.
     - `record_state` and `recorded_video`.
     - `write_analysis`, with C6's guard: `UNREADABLE_BY_FROZEN_PYTHON`,
       `unreadable_by_frozen_python` and `AnalyzeError::UnreadableReason`.
     - `resolve_swing_dir`, `Outcome`, `AnalyzeError` and `wire_name`.
   - **`crates/lab/src/report.rs`** (new): `print_report` (`_print_report`, `_print_shot` and
     `_print_coaching`) and `print_swings` (`_print_swings`), with each format spec through `pyfmt`.
   - **`golf-lab analyze`** in `bin/golf_lab.rs`, with `--list-swings` there as `list_swings`.
   - **`PipelineOptions.coaching`**, `true` in `new` (`config.py`'s default). `import::spelled` is
     now `pub`, for the binary.
   - **Tests.**
     - `tests/analyze.rs` has 12 tests, which run on any machine.
     - `tests/golf_lab.rs` gained 2. One covers `analyze`'s five refusals, the missing bundle and
       `--help`. The other runs the verb end to end on a scratch bundle, first with
       `--list-swings` and then with `--no-coaching`.
     - `coach.rs` has 5 unit tests.
     - **`tests/bundle/mod.rs`** (new) is `tests/assemble.rs`' scratch-bundle fixture, moved
       unchanged except for `pub`. `assemble.rs`, `analyze.rs` and `golf_lab.rs` now share it. It
       carries `#![allow(dead_code)]` because each test binary uses only part of it.
   - **Edges and the lock.** No new dependency and no new crate edge. `golf-lab`'s `Cargo.toml`
     changed only in the comment on its `golf-core` edge.
2. **The order after the engine is Python's, and `tests/analyze.rs` holds it.**
   - The steps run in this order:
     - the engine's notes;
     - the assembly's notes;
     - the loft repair, only on the engine's `no_club_loft`, logged under `\nBall flight:`;
     - `\nCoaching:` and the paragraph, or its note as the last note;
     - `\nWrote <path>`;
     - the state.
   - **`analysis.json` is written twice when coaching is on.** The first write comes before the
     coach is asked, with every note in and no paragraph, because `get_swing` reads the file (P17
     item 4). The second write folds the answer in. The test's coach records what the file held
     when it was asked, and the test checks that.
   - **The state:**
     - the worker's `queued_at` is kept;
     - `started_at` and `completed_at` come from the caller's clock;
     - `duration_seconds` is the microseconds divided by 1e6, exactly `timedelta.total_seconds()`;
     - `aligned.mp4` and its codec are carried forward only while the file is still on disk (C4);
     - `score` and `headline` are copied.
   - **The failure cases:**
     - a missing face-on view records a failed run and writes no `analysis.json`;
     - a missing manifest writes nothing;
     - a missing role flags the run and marks the state partial.
3. **C6's pin.** `the_reasons_the_writer_refuses_are_the_ones_frozen_python_lacks` reads
   `spec/schemas/swing_bundle_result.schema.json`'s `UnscoredReason` enum. It requires
   `UNREADABLE_BY_FROZEN_PYTHON` to be exactly Rust's reasons (`UNSCORED_REASONS`) minus that enum,
   which today is `printed_blank` and `misread`. A reason added to Rust but not to the refusal fails
   there. `a_result_naming_a_photo_side_reason_is_not_written` checks that the guard writes nothing.
   The engine never puts either reason into a `SwingResult`, so the guard is a pin, not a path any
   stored bundle reaches.
4. **Calls this phase made.** Each one is in the module doc it belongs to.
   - **C5's switch is `GOLF_COACHING_ENABLED` from the process environment only**, not from
     `.env`, for `DataDirs`' reason. `.env` holds no coaching setting today: it holds the upload
     token and the key, and the key is the entry's to read.
     - The value is read by pydantic's rule, measured on frozen Python's `TypeAdapter(bool)`:
       `1 t true y yes on` are true and `0 f false n no off` are false, in any case, with nothing
       trimmed.
     - **An empty value counts as unset**, the `DataDirs` rule, where pydantic refuses it.
     - Anything else is refused by name, with exit 2. `Settings()` raises on it.
   - **The entry runs in the checkout's `.venv` and nowhere else.** `crates/pose` keeps the first
     `PATH` interpreter that handshakes, but the entry has no handshake, and `PATH`'s 3.13 cannot
     import `golf_coach`. A missing interpreter is a note naming the path.
   - **`--golf-mcp` and `--model` are left to the entry**: `GOLF_MCP`, else the release build, and
     `settings.coaching_model`. That is the same rule `ask_swing.py` uses.
   - **The request uses the directory's names, made absolute.** The entry runs in the repository
     root, and `get_swing` joins `sessions_dir/session/swing`.
   - **An entry that answers neither shape is a note**: `no written coaching: the coaching entry
     could not be used (exit N: <last stderr line>). The scores and ranked tips above are
     unaffected.`, in `brief_through_golf_mcp`'s words.
   - **`analysis.json`'s keys are written in alphabetical order**, because `serialize` goes through
     a `serde_json::Value` whose map is a `BTreeMap`. Pydantic writes declaration order. Every
     reader reads by key, and byte identity is not the contract (`storage::write_atomically`'s
     doc). The frozen readers in item 6 are the evidence.
5. **C8, measured.** Both languages ran on scratch copies of three stored bundles, with empty
   stand-ins for the clips and an empty `aligned.mp4`. Frozen Python ran `analyze_bundle.py
   --no-video --no-coaching`, and Rust ran `golf-lab analyze --no-video --no-coaching` in release.
   The bundles were:
   - `2026-08-10/2`, a clean shot;
   - `2026-08-23/7`, a 3w with the loft note;
   - `2026-08-23/1`, the HEIC label-fix shot.

   Ignoring line endings and the scratch root, the log and report are identical except for three
   declared differences. The resume re-diffed the kept outputs and found the same three:
   - **The shot lookup's lines.** Rust re-reads every unstamped shot (P23). So where Python says
     `from the shot store`, Rust's first run says `… was read by an unversioned screen parser,
     re-reading with screen parser 1`, then `reading <photo> ...` and `parsed`. A second run says
     `from the shot store` in both languages.
   - **The re-read shot's `conf=` and warnings.** These are P23's listed provenance changes:
     `0.95 → 0.93` and the `Impact Position V: no value text under the label` warning on
     `2026-08-10-2`. `2026-08-23-1`'s `strike=HEEL` is gone (finding 2's label fix).
   - **`not measured:`.** The script's `', '.join(swing.unscored)` raises `TypeError` on any swing
     with an unscored entry. Two of the three do, `2026-08-23/7` and `/1`, both on
     `flight_carry_yds`. The script exits 1 with a traceback after both artifacts are written, and
     prints nothing of the report past that line. Rust prints the names, which is what the line was
     written to print, then the rest of the report, and exits 0. This is `report.rs`' module doc.
     Nothing frozen was fixed, and the script goes at P34.

   **The report section is otherwise the script's own.** The throwaway script in item 6 ran frozen
   `_print_report` over each Rust-written `analysis.json`, with `unscored` turned into names. Its
   output was **byte-identical** to `golf-lab`'s report section, ignoring CRLF, on all three
   bundles. That includes the coached one with its `textwrap`'d paragraph (item 7). The resume
   re-ran the script and it wrote the same three files.

   **Exit codes:** Python gave 0, 1 and 1 (the two tracebacks); Rust gave 0, 0 and 0. No shot needs
   review and no role is missing. The other declared CLI differences are in `bin/golf_lab.rs`' doc:
   - `--no-video` is accepted and does nothing.
   - `--tau` is refused (C4).
   - A malformed window, or any error the pipeline raises, is a sentence and exit 2. In Python the
     malformed window's `ArgumentTypeError` escapes `main` and gives exit 1.
   - `--golfers-dir` and `--models` are Rust's own, as `import-photos` has them.
6. **Frozen Python reads what `golf-lab` wrote.** A throwaway script ran under `.venv`'s Python,
   with settings pointed at the scratch copy (the resume ran it with `-B`, so it left no bytecode).
   It checked all three bundles, with the coached artifact for `2026-08-10/2`:
   - `load_analysis` and `load_state` read them: the state is `done`, matches the manifest, and has
     `score` copied;
   - `SwingBundleResult.model_validate`, the `/ask` route's typed read, accepts the artifact. Rust's
     M32 shot keys are extras that pydantic ignores, and `analysis_version` 17 reads as current to
     an engine at 16;
   - `mcp/query.py::get_swing`;
   - `storage/corpus.py::read_corpus`, which admits all three (`CareerCorpus`, 3 swings);
   - the frozen app through `TestClient` (`create_app(..., token=None, worker=None)`):
     `GET /api/sessions`, `/api/sessions/{s}/swings/{w}` and `…/flight` all return 200.

   No check failed in either session's run.
7. **The live coaching run, P26's one call.** It was made by the second session:
   `golf-lab analyze 2026-08-10/2` on the scratch copy, with coaching on by default.
   - The log reads `\nCoaching:` then `claude-opus-5 wrote 1254 characters`, and the run exits 0.
   - `generated_at` is `2026-10-25T16:25:25.124438Z`. `input_digest` is `be815699…`, the view
     brief's, as P17 item 4 says.
   - The run took 11.4 s (the state's `duration_seconds` is 11.427278), almost all of it the call.
   - stderr was empty, because the entry's banner is captured.
   - **The paragraph** names tempo first (868 ms against 384), ties the head-behind-hips flag to
     it, mentions the placement in one clause, says the launch-monitor numbers are not scored, and
     carries the band-estimator caveat.
   - The paragraph reads back through every reader in item 6. The resume checked the written
     artifact: `feedback.coaching_text` has 1254 characters, `feedback.coaching` has the
     provenance, and `keypoints` and `detections` are absent.
8. **Timing, measured under load by the second session.** A game was running on the box through
   the whole of that session (one process far ahead of everything else in CPU time), so every time
   here is an upper bound:
   - On a run that re-reads its shot, the read dominates: 148 s and 167 s in release for
     `2026-08-23/7` and `/1`.
   - The run with the shot already stamped took 11.4 s, coaching call included.
   - The ignored gates ran several times slower than P25's (item 14).

   P23's "about 24 s a photo in a debug build" is the unloaded figure.
9. **For P27.**
   - `reanalyze` is `analyze_swing_dir` with `options.coaching` set by `--coaching`, off by default
     (C5).
   - Build `Tools` once, as P25 item 8 says, and build `Entry::configured()` once as well.
   - **`--video`'s refusal** should read as `--tau`'s does: `golf-lab renders no side-by-side video
     (the M29 plan's C4)…`.
   - **`record_state`** and **`recorded_video`** are public. A run that renders nothing keeps the
     standing `aligned.mp4` in the state, which is what Python's `reanalyze.py` without `--video`
     does.
   - **`resolve_swing_dir`** is in `analyze.rs`.
   - **`report::print_report`** is not the reanalyze report. `reanalyze.py --dry-run`'s text is its
     own (C8).
10. **For P28.** `simulate_flight.py`'s 72-column wrap is `golf_core::textwrap::wrap(text, 72, …)`.
    It is reachable from `golf-lab` now, so P14 item 7's `golf_mcp::textwrap` is
    `golf_core::textwrap`.
11. **For P29.**
    - **Budget the time, and run it on a quiet machine.** Step 2's first run re-reads 13 photos.
      Under load each took up to about 150 s in release (item 8), which is half an hour at worst.
    - **Expect the state to carry `aligned.mp4`.** Each stored state already names it, and it is on
      disk.
    - **Coaching is off for `reanalyze` (C5)**, so the stored paragraphs go, as the plan says.
    - **What step 3's readers script should drive.** Item 6's script is the shape. The route that
      exists is `/flight`, not `/status`: `/status` answered 404, because the frozen app has no
      such route.
12. **For P34.** `analyze_bundle.py`'s `not measured:` crash goes with the script. The script is
    frozen and was not fixed.
13. **For P35.**
    - ARCHITECTURE §1 should name:
      - `golf-lab analyze` and its flags, with `bin/golf_lab.rs`' doc as the source;
      - `golf_lab::{analyze,coach,report}`;
      - `golf_core::{run_bundle, serialize, textwrap}`;
      - `storage::state::save_analysis`;
      - that coaching reaches Python through P17's entry, spawned in `.venv`.
    - **`textwrap`'s home.** Neither `CLAUDE.md` nor ARCHITECTURE names it, so no sentence there
      is stale. P9 item 5's "`textwrap.rs` lives in `crates/mcp`" is history, and `golf_core`'s
      crate doc and `textwrap.rs`' own doc say where it is now. If P35's crate map names it, it is
      `golf-core`'s.
14. **Green.**
    - **`cargo test`** (the resume's run): 1,569 passed, 0 failed, 22 ignored. That is P25's 1,549,
      plus 12 `analyze`, 2 `golf_lab`, 5 `coach` units and 1 `state` unit. The second session's run
      gave the same totals.
    - **`cargo test -p golf-lab --no-fail-fast -- --include-ignored`** passes everything: 114
      passed, 0 failed. The resume's run took 14.6 minutes on a quiet machine, with `inputs` at
      370 s, `reread` 353 s, `import_real` 64 s, `strikes` 41 s and `keypoints` 36 s, which are
      P25's figures again. The second session's run on the same tree also passed, but under item
      8's load (`inputs` 2,866 s, `reread` 2,797 s, `import_real` 410 s).
    - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**.
    - **`pytest tests/test_docs_truth.py`** passes 91. It was run because this file was edited.
    - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files, with no `.tmp` or
      `.part` file. Every write went into a scratchpad.
    - **Diffs:**
      - nothing changed under the ten held families, `spec/vectors/{selection,mcp}` or
        `spec/declarations/`;
      - no `src/`, `scripts/` or `tests/` file is touched: `git status` there shows only
        P8's, P15's, P16's and P17's files;
      - the files changed are:
        - `crates/core/src/{lib,textwrap}.rs` (the latter moved from `crates/mcp/src/`);
        - `crates/mcp/src/lib.rs`;
        - `crates/storage/src/state.rs`;
        - `crates/lab/{Cargo.toml (a comment), src/{analyze,coach,report,import,lib,pipeline}.rs,
          src/bin/golf_lab.rs, tests/{analyze,assemble,golf_lab}.rs, tests/bundle/mod.rs}`;
        - this file.
    - **Scratch:** both sessions' comparison outputs, readers script, scratch copies, edit helpers
      and logs are deleted.

### P27 (2026-10-25) — `golf-lab reanalyze` and `golf-lab analyze-keypoints`: every dry run over `data/` is the script's, and photo entries re-run

1. **What landed.**
   - **`golf_lab::reanalyze`** (new): `reanalyze.py` whole.
     - `collect` is `_collect`: the bundle store walked directly, named refs in the order given,
       and `--player` by stored id.
     - `swing_reason` and `photo_reason` are `_reason_to_rerun`, with the fourth form from item 2.
     - `reanalyze` is `main` after the arguments, with its text and its exit codes 0, 1 and 2.
     - `Py`, `diff`, `swing_rows`, `photo_rows` and `anchors` are `_before_after`, `_window` and
       `_anchors`. `Py` holds a value as the script's diff holds one: a `json.loads` value, a
       formatted string, or a tuple. It prints with `str()` and compares with Python's `==`, so a
       tuple never equals a list, `True == 1`, and `(574, 790) == (574.0, 790.0)`.
     - `DuplicatesByRoot` holds one corpus reader per sessions directory.
   - **`golf_lab::analyze_keypoints`** (new): `analyze_swing.py`. It has `swing_id_for` (Python's
     `Path.stem` rule, then `.removesuffix(".keypoints")`), `analyze_keypoints`, `instants` (over
     `alignment::anchors_from_phases`) and `print_keypoints_report`.
   - **`golf_lab::report`** now has `print_checkpoints` and `print_tips` (`pub(crate)`), factored
     out of `print_report` unchanged, because `analyze_swing.py` prints the same two blocks.
   - **`golf_lab::shot::parser_name`** (`pub(crate)`), split out of `parsed_by`, so the fourth
     reason names a parser in the same words as the shot lookup's log.
   - **`golf_core::pyrepr::py_repr`**, moved unchanged from `golf_mcp::query`, as `textwrap` was
     moved at P26: the shells cannot depend on each other. `golf_mcp::query` imports it, and
     `py_str` stays there. It has its own unit test.
   - **`golf_core::session`**: `Outcome`, `analyse_entry` and `Duplicates` are now `pub`. They are
     `run-shot --session`'s code for one photo entry, which C7 reuses typed rather than parsing the
     verb's line. **`Duplicates` now owns its `PathBuf`**, so a walk can hold one per sessions
     directory. Nothing else in `run_shot_session` changed.
   - **The verbs**: `golf-lab reanalyze` and `golf-lab analyze-keypoints` in `bin/golf_lab.rs`,
     with USAGE and the binary's doc. The USAGE's "arrive as their scripts are" line now names
     only `flight`, `build-vectors` and `import-export`.
   - **One new crate edge: `golf-lab → feedback`**, for `rules::build_feedback` over one view's
     result. `golf-core`'s `run_bundle` builds a bundle's feedback, and nothing there serves a
     single view. Cargo.lock gains only `"feedback"` in `golf-lab`'s dependency list. No external
     dependency was added.
   - **Tests.**
     - `tests/reanalyze.rs` (new) has 6 tests: each form and its order, the walk's order (`10`
       after `5`), `--all`, refs by name (repeats kept), `--player`, a missing ref, a swing run,
       its diff and its idempotence, `--verbose`, the stale-video line, a failed run, a photo
       entry run with each of its reasons, and a duplicate photo entry.
     - `tests/golf_lab.rs` gained 2: the two verbs' refusals, an empty-store dry run, and one
       report over the scratch bundle's keypoints.
     - There are 3 `reanalyze` unit tests, 1 `analyze_keypoints` unit test and 1 `pyrepr` unit
       test.
2. **Calls this phase made.** Each one is in the module doc it belongs to.
   - **A fourth form of out of date, asked last: a shot an older screen parser read.** Frozen
     Python had one parser, so its artifacts could not go stale that way. Rust has the ledger, and
     a run re-reads a stale shot. Without this form, a `SCREEN_PARSER_VERSION` bump would leave
     every stored shot stale while this verb reported nothing to do, which is the gap the script
     was written to close, one ledger over.
     - The parser is read raw off `swing.shot.provenance.parser_version`, with
       `stored_analysis_version`'s tolerance.
     - A shot with no provenance is current, as `parse_is_current` has it.
     - The words are `its shot was read by an unversioned screen parser, screen parser 1 is
       installed`.
     - On `data/` it is never the first reason, since every stored artifact is at engine 16.
   - **Photo entries (C7)** are re-run in two steps. First `shot_for`, which re-reads a stale stored
     shot (P24 item 8). Then `golf_core::session::analyse_entry`, which writes
     `shot_analysis.json`.
     - Their staleness is `shot_analysis.json`'s, asked in the same order and words: none, a photo
       that is not the one it was computed from (the script's re-upload sentence), an older engine,
       an older parser.
     - **A photo entry the corpus counts as a duplicate is never out of date.** `run-shot` skips
       it, so listing it would break the script's idempotence. Named, or under `--all`, its run
       prints `skipped: <the corpus's sentence>` and counts as done.
     - A lookup that attaches no shot is a `FAILED` line. A note it leaves is printed as
       `! <note>`, because no artifact keeps it.
     - Only the shot's `needs_review` asks for a human. A photo entry lacks a face-on clip by
       definition, so a missing role is not a flag.
     - The diff has the rows a `ShotResult` holds: `version` and `measurements`.
   - **A run that raises in Python is a `FAILED` line**, and the walk goes on. Python's traceback
     ends the whole run.
   - **The stale-video line** ends `— and golf-lab renders no side-by-side video to replace it
     (the M29 plan's C4)`, where the script says `— re-run with --video`. The `!` line and its
     exit 1 are the script's.
   - **`--video` and `--overlay` are refused**, exit 2, in `--tau`'s words (P26 item 9). Unknown
     flags are refused too.
   - **`reanalyze` reads `GOLF_COACHING_ENABLED` on every run**, as `analyze` does. An invalid
     value is exit 2 even without `--coaching`. In frozen Python the same value makes `Settings()`
     raise at import.
   - **`analyze-keypoints` parses its arguments.**
     - `--help` prints the usage on stdout with 0, like every `golf-lab` verb, where the script
       printed it on stderr with 2.
     - With no path, argparse's refusal exits 2, which is the script's code.
     - A missing file is the script's `error: <path> not found` with exit 1. So is a file that will
       not read, where the script's traceback also exits 1.
     - A second path is refused, where the script ignored it.
3. **C8 for `reanalyze`, measured.** Frozen `reanalyze.py` was run through a throwaway that set
   `ANALYSIS_VERSION` to 17 in `api.state` and in the script. That is the engine Rust installs, so
   both languages name the same targets. Unpatched, the script says `Nothing to re-analyze …` over
   `data/`, because its engine is 16.
   - **Ten dry runs over `data/` are byte-identical**, ignoring CRLF, with equal exit codes:
     - the walk, which lists all 15 as `analyzed by engine version 16, 17 is installed`;
     - `--all`, `--player aaron` and `--player nobody`;
     - three refs with one repeated;
     - two refs that resolve to nothing (exit 2, both errors);
     - a forward-slash path and a backslash path;
     - `--sessions-dir` with a trailing slash;
     - an empty sessions directory.
   - **Runs on scratch copies of `2026-08-10/2` and `2026-08-23/7`.** The copies held every file
     but the clips, with empty stand-ins for the clips and for `aligned.mp4`.
     - The text is identical except the diff line: Rust says `version 16 -> 17`, where Python's
       engine writes 16 again and says `no change`. No other row moved, as `v17.json` says (shape
       only).
     - Rust's run took 19 s, including both photo re-reads. Python's took 2.7 s.
     - A second Rust run says `Nothing to re-analyze …`.
     - `--all --verbose` in both: 52 lines each, identical but the four reason lines (Python's
       artifacts are at 16, Rust's at 17). The narration, `from the shot store` included, is the
       pipeline's own, as P26 measured.
   - **Photo-only scratch entries.** Two entries were opened by hand in session `2026-10-25` of the
     Rust copy:
     - Entry 1 holds `2026-08-23/2`'s photo, whose stored shot is unstamped. The walk listed it as
       `never analyzed`. The run re-read the photo (`re-reading with screen parser 1`, 9.9 s) and
       printed `version 0 -> 17 | measurements 0 -> 7`. A second run had nothing to do.
     - Entry 2 holds `2026-08-10/2`'s photo, which that filmed swing holds too. It was never
       listed. Run by name, it printed `skipped: shot photo 07ce15cc2b80 is already in
       2026-08-10/2, beside that swing's video — one ball flight, so the copy with the video is the
       one kept`, with exit 0.
     - `golf-core career-corpus --name Aaron` over the copy reported 4 swing directories, 3 distinct
       swings and 3 distinct shots.
4. **C8 for `analyze-keypoints`, measured on five stored views**: `2026-08-10/2` face-on,
   `2026-08-23/7` face-on, `2026-08-23/1` down-the-line, `2026-08-07-aaron1/1` face-on and
   `2026-08-09/2` down-the-line.
   - **The script crashes on every run.** With no golfer, `head_stays_back` is always unscored, so
     its `', '.join(result.unscored)` raises `TypeError`. It exits 1 after the header, the
     instants, the headline and the checkpoints: 13 lines, or 12 where `tempo` was unscored too.
     Those lines are byte-identical to Rust's.
   - **The whole report matches.** A throwaway handed the script's own `_print_report` its own
     result, with `unscored` as strings carrying `.name`, `.reason` and `.detail`, and with
     `build_feedback` answered from the real result. Its output is byte-identical to `golf-lab
     analyze-keypoints` on all five, ignoring CRLF.
   - **Rust exits 0.**
5. **For P29.**
   - **What `reanalyze --all` will print.** Expect each of the 15 to print `version 16 -> 17` and
     nothing else in its diff: `v17.json` moves no number, and item 3's two bundles did exactly
     that.
   - **The stale-video line needs moved anchors**, so none is expected. Then the exit is 0 unless
     a re-read shot needs review.
   - **Add `--verbose` to see the re-reads.** The thirteen `re-reading with screen parser 1`
     lines are narration, printed only under `--verbose`.
   - **No photo entry exists in `data/`.** Item 3's walk listed only the 15.
   - **One `Tools` serves the walk**: one OCR reader, one sidecar and one ffmpeg, built once in
     `run_reanalyze`, as P25 item 8 asked. Coaching is off unless `--coaching` (C5).
6. **For P34.**
   - `analyze_swing.py`'s `not measured:` crash goes with the script, as `analyze_bundle.py`'s
     does (P26 item 12). Neither was fixed.
   - The scratch throwaways imported `scripts/reanalyze.py` and `scripts/analyze_swing.py` as
     modules under `-B`, so they left no bytecode.
7. **For P35.**
   - **ARCHITECTURE §1** should name:
     - `golf-lab reanalyze` and `golf-lab analyze-keypoints` and their flags, with
       `bin/golf_lab.rs`' doc as the source;
     - `golf_lab::{reanalyze, analyze_keypoints}`;
     - `golf_core::pyrepr`;
     - that `golf_core::session`'s `analyse_entry` and `Duplicates` are now public;
     - the `golf-lab → feedback` edge.
   - **The fourth staleness form** is a behaviour no Python doc describes. It belongs wherever
     P35 says what replaced `reanalyze.py`, perhaps ADR-035's M29 addendum.
   - The lab crate doc already names both modules and `tests/reanalyze.rs`.
8. **Green.**
   - **`cargo test`**: 1,582 passed, 0 failed, 22 ignored. That is P26's 1,569, plus 6
     `reanalyze`, 2 `golf_lab`, 3 `reanalyze` units, 1 `analyze_keypoints` unit and 1 `pyrepr`
     unit.
   - **`cargo test -p golf-lab --no-fail-fast -- --include-ignored`**: 126 passed, 0 failed. That is P26's 114 plus
     this phase's 12. Times on a quiet machine: `inputs` 373 s, `reread` 353 s, `import_real` 63 s,
     `strikes` 41 s, `keypoints` 34 s, which are P26's figures again.
   - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**.
   - **`pytest tests/test_docs_truth.py`** passes 91. It was run because this file was edited.
   - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files, with no `.tmp` or
     `.part` file. Every write went into the scratchpad.
   - **Diffs:**
     - nothing changed under the ten held families, `spec/vectors/{selection,mcp}` or
       `spec/declarations/`;
     - no file under `src/`, `scripts/` or `tests/` is newer than this phase's marker;
     - the files changed are:
       - `crates/core/src/{lib,pyrepr,session}.rs` (`pyrepr.rs` is new);
       - `crates/mcp/src/query.rs`;
       - `crates/lab/{Cargo.toml, src/{analyze_keypoints,reanalyze,lib,report,shot}.rs,
         src/bin/golf_lab.rs, tests/{reanalyze,golf_lab}.rs}` (`analyze_keypoints.rs`,
         `reanalyze.rs` and `tests/reanalyze.rs` are new);
       - `Cargo.lock`, for the one edge;
       - this file.
   - **Scratch:** the throwaway scripts, comparison outputs, scratch copies and logs are deleted.

### P28 (2026-10-28) — `golf-lab flight`: cut off once, resumed and done; the script's text on 51 of 54 argument sets, and the other 3 are where it raises

1. **The resume.** The cut-off session (marker 2026-10-25 22:45) had changed four files and
   started no verb. All four were kept as they stood, because their tests pass and each matches its
   Python twin on a re-read:
   - `crates/analysis/src/benchmarks/flight_model.rs`;
   - `crates/analysis/src/flight.rs`;
   - `crates/storage/src/flight_inputs.rs`;
   - `crates/storage/src/lib.rs` (its doc).

   That session's frozen-script dumps were reused for the first comparison. Then they were
   re-recorded in this session, with nine more argument sets, and the old scratch directory was
   deleted.
2. **What landed.**
   - **`analysis::benchmarks::flight_model`** (the cut-off session's work) gained the rest of the
     artifact the script prints:
     - `SourceNote` on every block, and `FlightDatasetInfo` with `flight_dataset_info()`;
     - the atmosphere's name, viscosity, temperature and pressure;
     - `AtmosphereProfile` with `FlightModel::at_altitude`, refusing outside the layer with
       `AltitudeOutsideLayer` in Python's words.

     Unit tests hold the four numbers at four altitudes to frozen Python's `repr` to the bit, and
     zero altitude to the committed air.
   - **`analysis::flight`** (the cut-off session's work) gained:
     - `VALIDATION_SHOTS`, `GateComparison` and `gate_comparisons(model, step)`, hoisted out of the
       validation test as P14 item 7 offered;
     - `FlightResult::sample` and `FlightPoint::{x,y,z}_yds`.

     Tests pin the gate to the bit and `sample`'s indices to Python's.
     `thinner_air_flies_further_and_flatter` now flies the real what-if.
   - **`analysis::flight_measure::signed_percent`** is now `pub`, so the gate table and the error
     column share P14's composition.
   - **`storage::flight_inputs`** (the cut-off session's work) gained `read_flight_inputs`, the
     corpus-wide join, with `FlightInputs`, earliest arrival first. Its 8 new unit tests cover
     `test_flight_inputs.py`'s join cases (its three gaps in one test) and the instant-then-ids
     order.
   - **`golf_lab::flight`** (new) is `simulate_flight.py` after its arguments: all four modes, the
     report, the caveats, the stored findings and `_gate_from_disk`. Its helpers are `wrap`,
     `signed_g` (`:+g`), `local_minute` (`%Y-%m-%d %H:%M` in the stored offset) and the
     `NamedTuple` repr of `LaunchConditions`.
   - **The verb**: `golf-lab flight` in `bin/golf_lab.rs`, with USAGE, the binary's doc and the
     script's three mode checks in its order. The "arrive as their scripts are" line now names only
     `build-vectors` and `import-export`.
   - **No new edge, no dependency.** `golf-lab` already reached `analysis`, `storage`, `contracts`,
     `golf-core` and `pyfmt`, and `Cargo.lock` is unchanged. Only `crates/lab/Cargo.toml`'s comments
     changed.
3. **Calls this phase made.** Each is in `golf_lab::flight`'s module doc.
   - **Where the script raises, this answers.** Neither `gate_comparisons` nor the spin solve is
     guarded in the script, so `--step 0` with `--gate`, `--shots` or `--shot` ends in a traceback
     (exit 1).
     - `--gate` prints `No flight: step must be positive, got 0.0 s` with exit 2, as the typed mode
       already does.
     - Over stored shots, a refused solve becomes that shot's row, `refused by the integrator`, with
       exit 0. That is what the script's own `_Resolved.error` exists for.
   - **`--shot` flies only the shot asked for.** The script resolves every shot first, which is why
     its `--shot 2026-08-10-1 --step 0` raises on another shot's solve. The report is otherwise the
     same, because nothing it prints reads another row.
   - **The empty store names `golf-lab import-photos`**, where the script names
     `scripts/import_shot_screens.py`, which P34 deletes.
   - **Three directory flags of its own**: `--shots-dir`, `--sessions-dir` and `--golfers-dir`,
     defaulting to the variables the script's `settings` reads. The tests need them to point at a
     fixture.
   - **Refusals use `golf-lab`'s usage form** (`golf-lab: flight: …` and the usage), with the
     script's messages and its exit 2, as every other verb does.
4. **C8, measured.** Frozen `simulate_flight.py` and the debug `golf-lab flight` were each run over
   the same argument sets (throwaway driver, deleted), stdout compared byte for byte with CRLF
   normalized, and exit codes compared.
   - **Over `data/`, 51 of 54 argument sets are identical.** The sets cover:
     - typed: lateral; altitude at 1609, −390.5, 0, 20000 and NaN; points 1, 12, −3 and 100000;
       verbose; step; partly clamped, never clamped, hook, rolling and negative spin;
     - `--gate` with altitude and verbose;
     - `--shots` with altitude, verbose and points;
     - all 13 `--shot` ids, plus points, altitude and a full combination;
     - every refusal.

     The other 3 are the `--step 0` sets in item 3.
   - **The empty store**: 5 sets, identical except the import sentence.
   - **No sessions and no golfers**: 5 sets, identical. Every shot is `no_swing`, and the gate's
     "do not match" sentence with both `NamedTuple` reprs comes out exact.
   - Debug Rust ran the first 45 sets in about 3 s. Frozen Python took 2 min 25 s for all 54.
5. **Tests.**
   - **`crates/lab/tests/flight.rs`** (new) has 10 tests and 1 ignored. They need no `data/`.
     - **The typed report** (`--points 12`) and **the gate at 1609 m with `--verbose`** are equal to
       the frozen script's stdout.
     - **Five stored reports** (`--shots` and four `--shot`s) are equal to the script's stdout over
       a committed fixture tree, `crates/lab/tests/flight/`: five shots (four trimmed from
       `data/`, both gate screens among them), four manifests, the golfer and a one-club bag. The
       script read the tree through the environment, and Rust reads it through the flags and once
       through the same variables. `expected/*.txt` is the script's output, recorded once.
     - The refusals, the no-flight sentences, the unknown id and the empty store are each a test.
     - **The ignored test** reads `data/` for what P29 cannot move: the gate screens match the typed
       pair, and the photo three swings share.
   - **Mutation check.** I wrote the error column with `percent` instead of `signed_percent`, and
     the first fixture passed, since its only measured shot erred negative. Adding the unheld
     second gate screen (+2.54%) made it fail on two tests. The mutation was reverted.
   - **Unit tests**: 4 in `golf_lab::flight`, 4 in `flight_model`, 2 in `flight` and 8 in
     `flight_inputs`.
6. **For P29.**
   - The ignored `the_corpus_on_disk_reads_as_the_script_read_it` runs with the lab's
     `--include-ignored` and should hold after the write. The label fix does not touch a flight:
     2026-08-10-1's `shot_type` is a fade under either spelling, and `impact_position` is not read.
   - `golf-lab flight --shots` over the new `data/` is a cheap extra reader check for step 3.
7. **For P34.**
   - `scripts/simulate_flight.py` has its twin. The fixture's `expected/*.txt` stays valid after the
     script goes: it is recorded text, and the test doc says where it came from.
   - The empty-store sentence already names `golf-lab import-photos`.
8. **For P35.**
   - **ARCHITECTURE §1** should name:
     - `golf-lab flight` and its flags, with `bin/golf_lab.rs`' doc as the source;
     - `golf_lab::flight`;
     - `storage::flight_inputs::read_flight_inputs`;
     - `analysis::flight`'s gate and `sample`;
     - `flight_model`'s `AtmosphereProfile`, `at_altitude` and `flight_dataset_info`.
   - **`CLAUDE.md`'s `crates/storage` paragraph** lists `flight_inputs, loft_for_club and LoftGap`
     (M35). The join is ported now.
   - **`crates/analysis`'s crate doc** names P28's additions. `flight.rs`' "What is not here" section
     is rewritten, and `flight_model.rs`' module doc says what P28 brought.
9. **Green.**
   - **`cargo test`** (`CARGO_INCREMENTAL=0`): 1,610 passed, 0 failed, 23 ignored. That is P27's
     1,582 plus 28, and P27's 22 ignored plus 1. The 28 are 10 in `tests/flight.rs`, 4
     `golf_lab::flight` units, 4 `flight_model`, 2 `flight` and 8 `flight_inputs`.
   - **`cargo test -p golf-lab --no-fail-fast -- --include-ignored`**: 141 passed, 0 failed. That is
     P27's 126 plus this phase's 15. Times: `inputs` 363 s, `reread` 347 s, `import_real` 62 s,
     `strikes` 40 s, `keypoints` 37 s.
   - **`cargo clippy --all-targets`** is clean, and so is **`cargo fmt --check`**. The
     `flight_model` test of Python's `repr`s needed `#[allow(clippy::excessive_precision)]`, with
     `pyfmt`'s `hypot` test as the precedent.
   - **`pytest tests/test_docs_truth.py`** passes 91. It was run because this file was edited.
   - **Data:** the `data/processed` digest is still `bd384cc4…` over 192 files, with no `.tmp` or
     `.part` file. Nothing was written under `data/`.
   - **Diffs:**
     - nothing changed under the ten held families, `spec/vectors/{selection,mcp}` or
       `spec/declarations/`;
     - no file under `src/`, `scripts/`, `tests/` or `spec/` is newer than this phase's marker;
     - the files changed are:
       - `crates/analysis/src/{benchmarks/flight_model,flight,flight_measure,lib}.rs`;
       - `crates/storage/src/{flight_inputs,lib}.rs`;
       - `crates/lab/{Cargo.toml (comments), src/{flight,lib}.rs, src/bin/golf_lab.rs,
         tests/flight.rs}` (`flight.rs` in both places is new);
       - `crates/lab/tests/flight/` (new): the fixture tree and `expected/`;
       - this file.
   - **Scratch:** both sessions' dumps, drivers, edit helpers and logs are deleted.

### P29 (2026-10-28) — stopped twice before the write (two lab gates, then two Python pins), answered (b) and (a), resumed and done

**How this phase stopped.** Nothing was written: no backup, no `data/` write, no code. The
`data/processed` digest is still `bd384cc4…` over 192 files, and `%USERPROFILE%\golf-coach-backups\`
does not exist yet. The stop is on the Verify line. It was found before step 2, so nothing has to be
restored. The State cell stays 🔄.

1. **What fails after the write, by construction.** P29's Verify runs `cargo test -p golf-lab --
   --include-ignored`. Two of those gates assert the store as frozen Python left it:
   - **`crates/lab/tests/reread.rs`** fails every bundle with `the stored shot is stamped`, and
     again on `not a dry-run re-read of the stored shot`. After step 2 every shot is current and is
     served from the store (`Origin::Stored`). Its own doc calls it "what P29 will write, measured
     before anything is".
   - **`crates/lab/tests/inputs.rs`** asserts `reader.reads() == 15` (`each bundle's photo is re-read
     once`), and after the write it reads none. Its other checks hold, because the stored shot then
     carries the declared values and the stamp.

   No rule or finding says what these two do after P29. The other ignored gates are unaffected:
   `strikes`, `keypoints`, `audio`, `import_real`, `flight`, `ffmpeg` and all of `crates/ocr`'s read
   files P29 does not change, or scratch copies. The Python pins that read `data/` were built to
   follow it (P8 item 12, P18 item 10).
2. **The question: what do `reread.rs` and `inputs.rs` read once `data/` is Rust's?**
   - **(a) C12's backup**, which is `data/processed/` before P29 by construction. The change is
     the root in `reread.rs`' `processed()` and in `inputs.rs`' call to `scratch_copy`. `inputs.rs`
     already makes each clip's stand-in from its manifest name, so the backup's missing clips do not
     matter. Both gates go on certifying what P23 and P25 built them for, and fail by name if the
     backup is gone. **Recommended.**
   - **(b) The written `data/`.** `reread.rs` becomes "every stored shot is stamped by this parser
     and served from the store with its one log line", and `inputs.rs` expects no photo read. The
     comparison with frozen Python's reads then lives only in P23's and P25's findings and in P29's
     step 3.
   - **(c) Either state**: each gate checks the unstamped store as now, or the stamped one as (b).
3. **For the resumed P29.** Do step 1 (the backup) before editing either gate, since (a) reads it.
   Then run `cargo test -p golf-lab -- --include-ignored` once **before** step 2, as the
   "P23–P28 green" precondition, and once after.
4. **Answered by the user (2026-10-28): (b), the written `data/`.** Rewrite both gates to check
   the store as P29 leaves it. `reread.rs` checks that every stored shot is stamped by this parser
   and served from the store with its one log line. `inputs.rs` expects no photo to be read
   (`reader.reads() == 0`) and keeps its other checks. The comparison with frozen Python's reads
   lives on in P23's and P25's findings and in P29's step 3, not in a gate. Item 3's order still
   holds: back up first, run the lab suite green before step 2, then change the two gates with the
   write and run it once more.

**The second session (2026-10-28, from 09:35)** resumed after the user's (b). Nothing had changed
after the marker but this file. It did step 1, ran the lab suite before step 2, rewrote the two
gates, and rehearsed step 2 on a scratch copy before writing anything. The rehearsal found two more
pins that the write fails, so it stopped again before step 2. **`data/` is still unwritten**: the
digest is `bd384cc4…` over 192 files, and the State cell stays 🔄.

5. **Step 1 is done, and the resumed P29 reuses it.** C12's folder holds `data/processed/` minus
   `.MOV`/`.mp4`: 158 files, 338 MB (the three trees' 147, plus the root's loose keypoints files
   and `conversations/`). `SHA256SUMS` lists all 158 and itself hashes to `a7a9436d533f…`.
   `sha256sum -c` against `data/processed` passed when the backup was made and again at the
   session's end. The backup is valid while the digest stays `bd384cc4…`, and C12's folder name is
   taken, so the resumed P29 checks both and makes no second one.
6. **The lab suite before step 2 (item 3) passed** with the gates as P28 left them: `cargo test -p
   golf-lab --no-fail-fast -- --include-ignored`, 141 passed, 0 failed (`inputs` 368 s, `reread`
   405 s; machine load about 16%). If the digest holds and no `crates/` file has changed since, the
   resumed P29 need not run it again.
7. **The two gates already check the store as P29 will leave it (the user's (b)).**
   - **`crates/lab/tests/reread.rs`** is now
     `every_stored_shot_is_stamped_by_this_parser_and_served_from_the_store`. Per bundle:
     - the stored shot is `parse_is_current`, its `parser_version` is exactly
       `SCREEN_PARSER_VERSION`, and its `fields_present` is the bay layout;
     - `shot_for`, still a dry run, answers `Origin::Stored` with the store's shot, no note, and
       the one log line `  shot_screen: {id} from the shot store`.

     After the loop it requires `reader.reads() == 0` and the store's hashes unchanged.

     **It also checks one thing the answer did not name**: each `DECLARED_VALUES` entry's re-read
     side must be the store's value on the bundles naming its shot, and an entry never seen fails.
     That keeps the list's two gates honest from both sides. It also keeps `DeclaredValue.stored`
     read in that binary, which clippy otherwise flags as dead.

     The ignore reason no longer names the models.
   - **`crates/lab/tests/inputs.rs`** requires `reader.reads() == 0` and keeps every other check.
     Its doc says the committed vector still carries frozen Python's shot, so the declared paths
     are seen as before.
   - **The docs that described the old `reread.rs` are updated**: `crates/lab/tests/common/mod.rs`,
     `crates/lab/src/lib.rs`' `shot` bullet, and `crates/lab/tests/shot_lookup.rs`' last sentence.
   - **Against today's `data/`, both fail until step 2 lands, by construction.** `reread` fails on
     every stamp, and `inputs` on reading 13 photos. They are the only red in the tree:
     - `cargo test` passes 1,610 with 23 ignored (P28's figures);
     - `cargo clippy --all-targets` and `cargo fmt --check` are clean.
   - **Against the rehearsal (item 8), both pass.** They were run once with their root pointed at the
     rehearsal's tree, then restored:
     - `reread`: 15 bundles, 13 photos, 0 read, declared values seen `[3, 3, 1]`;
     - `inputs`: 15 s, 0 photos read, `[3, 3, 1]`.
8. **Step 2, rehearsed on a scratch copy.** The copy held `sessions/`, `shots/` and `golfers/`, with
   the clips as empty stand-ins. The run was `golf-lab reanalyze --all --verbose` in release, with
   the three directories as flags.
   - **The run**: 2 min 41 s, exit 0, `Re-analyzed 15/15`. It logged 13 `re-reading with screen
     parser 1` lines and 2 `from the shot store` lines (P23 item 8's 13, not 15), with no `FAILED`
     line and no `!` note. A `--dry-run` afterwards lists all 15 as up to date.
   - **Files, against the backup.** Exactly 43 changed: 13 `shots/*.shot.json`, 15 `analysis.json`
     and 15 `analysis.state.json`. Nothing is new or missing, and every `keypoints.json` and
     `audio.json` is byte-identical.
   - **The shots.** All 13 are at `parser_version` 1 with 14 tiles present.
     - Values move on the two label-fix shots only, exactly `DECLARED_VALUES`.
     - The M32 keys are added as null.
     - The provenance text moves as P23 item 3 lists.
   - **`analysis.json`.**
     - All 15 go `analysis_version` 16 → 17, and each embedded shot changes as above.
     - Outside the shot, only last-bit floats move. `overall_score`, `mechanics_score` and
       `feedback.overall_score` move on 7 bundles, and one `checkpoint_scores[5].score` on 6, at
       about 1e-15 relative (`91.09575651552704 → 91.09575651552709`).
     - No measurement moves.
     - None of the 15 stored artifacts has a coaching paragraph, so C5 drops nothing.
   - **The frozen readers** found 0 problems before the rehearsal and 0 after. They were driven by a
     throwaway that covered:
     - TestClient over every GET route and the static pages;
     - `load_analysis`, `load_state` and `matches`;
     - `is_outdated` and the four `resolve_*`;
     - `SwingBundleResult.model_validate`;
     - `mcp/query.py`'s functions, `read_corpus` and `ShotStore.all`.

     The before/after diff is exactly the moves above, plus the state's times and `score`. The only
     status change was the video routes, 206 → 416, on the empty stand-ins.
   - **The Rust readers.** `golf-core`'s four career verbs, `profile` (text and `--json`) and
     `golf-lab flight --shots` print byte-identical text before and after.
   - **`golf-mcp`.** Of 55 calls across all twelve tools, only `get_swing` (15), `list_sessions` and
     `get_recent_shots` answer differently. The 13 `get_shot_by_id` calls passed store file names
     rather than shot ids, so they answered not-found both times.
9. **What stopped the write: two Python pins hold `data/` to frozen Python, and P18 item 10 said
   one of them would not.** Each builder was called over the rehearsal's tree, with its root
   constants patched.
   - **`test_the_corpus_selection_cases_choose_the_stored_windows` fails.** `_selection_corpus`
     (`scripts/conformance_vectors.py`) refuses any artifact that is not at frozen Python's
     `ANALYSIS_VERSION`, and it does so before it compares a single window: `2026-08-07-aaron1-1 is
     stored at v17, and frozen Python is v16`. With the version patched past that guard, all 15
     bundles choose the windows Rust wrote. The selection agrees; only the guard trips.
   - **`test_the_real_career_case_starts_from_the_real_storage_corpus` fails.** `_career_real`
     requires frozen `read_corpus` over `data/` to equal the committed `storage/corpus/real` corpus
     (M36's call 2). After the write the two differ at exactly 13 paths, `swings[].analysis_version`
     16 → 17. Today they differ at none.
   - **`_storage_corpus_real` and `_mcp_real` hold** over the rehearsal, as P8 item 12 said.
   - **All four pass today**: `pytest tests/test_conformance.py -k …` gives 4 passed in 41 s.

   Neither failure shows in P29's own Verify, which runs no pytest. Both would fail at P32. Keeping
   them green needs an edit to `scripts/conformance_vectors.py`, which the rules sanction only for
   the `mcp` and `selection` families. The selection guard is inside that; the career builder is
   not. Leaving them red breaks "every phase ends green". **Nothing was written to `data/`**, so
   nothing needs restoring.
10. **The question: what do these two pins do once `data/` is Rust's at engine 17?**
    - **(a) Follow the written `data/`, as (b) did for the Rust gates.**
      - `_selection_corpus` refuses only an artifact *older* than frozen Python's version, and goes
        on comparing windows. A v17 artifact already reads as current to frozen Python (P26
        item 6), and the windows held on 15 of 15 measured.
      - `_career_real` compares with each swing's `analysis_version` excused, which is the one
        difference measured.
      - The edit is those two builders and the two tests' docstrings. The career half is a frozen
        tooling edit the rules do not yet sanction, which this answer would sanction for P29.

      **Recommended.**
    - **(b) Hold both to C12's backup**: once `data/` is Rust's, the builders read
      `%USERPROFILE%\golf-coach-backups\m29-pre-write\`. That puts a machine-local path into frozen
      tooling, and both tests skip on every other machine.
    - **(c) Retire both checks.** The committed `selection/corpus/` and `storage/corpus/real` stay
      records of `data/` before P29, as P8 item 12 and P18 item 10 already call them, and the two
      tests skip with a reason naming P29.
    - **(d) Write anyway and leave both red** until P33–P34 re-check `tests/test_conformance.py`.
11. **For the resumed P29.**
    - **Check before anything else**: the digest is `bd384cc4…`, and `sha256sum -c` passes against
      C12's `SHA256SUMS`. If both hold, step 1 and item 6's run stand.
    - **Make the answer's edit**, then run the four pins green before step 2:
      `pytest tests/test_conformance.py -k 'real_corpus_case_reads or real_career_case_starts or
      real_mcp_case_answers or corpus_selection_cases_choose'`.
    - **Write**: rebuild the release binaries, then run `target/release/golf-lab.exe reanalyze
      --all --verbose`. Then do step 3, run the four pins again, and run the lab suite with
      `--include-ignored`.
    - **Do not rewrite the gates.** Item 7's are already in place.
    - **This session's throwaways and before-snapshots** are in
      `C:\Users\aaron\AppData\Local\Temp\claude\C--Users-aaron-Desktop-repo-golf-coach\365debe2-1b99-4063-b6d6-e6142e343bff\scratchpad\`:
      - `frozen_readers.py OUT`;
      - `against_backup.py DATA BACKUP`;
      - `mcp_calls.py OUT [golf-mcp flags]`;
      - `pins_on.py ROOT`;
      - `readers-before.json`;
      - `before/`: the career verbs, `profile`, `flight --shots`, the dry run and `mcp.json`.

      Diff the new `data/` against them, then delete the folder when P29 ends.
12. **Answered (2026-10-28): (a), follow the written `data/`.** The user chose the recommendation.
    `_selection_corpus` refuses only an artifact older than frozen Python's version and goes on
    comparing windows; `_career_real` compares with each swing's `analysis_version` excused. This
    sanctions, for P29 only, the edit to `scripts/conformance_vectors.py`'s career builder as well as
    its selection guard, plus the two tests' docstrings. Make it as item 11 says, then write.

**The third session (2026-10-28, from 11:21)** resumed after the user's (a), made the edit and
wrote `data/`. Only this file had changed after the second session's last edit (09:57), so nothing
of the second session's work was redone. **P29 is done.**

13. **The checks before anything else held.**
    - The digest was `bd384cc4…` over 192 files.
    - `sha256sum -c` against C12's `SHA256SUMS` passed on all 158 files, and the file still hashes
      to `a7a9436d533f…`.
    - No `crates/` file had changed since 09:57. Item 6's lab run stood and item 7's gates were not
      touched.
14. **The answer's edit**, in `scripts/conformance_vectors.py`, as item 12 sanctions.
    - **`_selection_corpus`** refuses an artifact only when its `analysis_version` is not an int
      or is older than frozen Python's. A newer one is compared, and its comment says why: Rust
      re-ran it, and frozen `is_outdated` is a `<`, so frozen Python reads it as current.
    - **`_career_real`** compares frozen `read_corpus` over `data/` with the recorded corpus through
      a new `_excusing_newer_engine_versions`. That helper puts a swing's recorded
      `analysis_version` back only where three things hold:
      - the read version is a newer int;
      - the two corpora have equally many swings;
      - the swing pairs with one of the same `session_id` and `swing_id`.

      An older version, a mispaired swing or any other moved key still fails.
    - **`tests/test_conformance.py`**: the two tests' docstrings say this. No test body changed.
    - **Before the write**: the four pins passed (4 passed, 34 s), and ruff was clean. Both files
      stay uniformly CRLF.
15. **Step 2, the write.** The release binaries were rebuilt; only `golf-lab` relinked, for item 7's
    doc edits. The `--dry-run` was byte-identical to item 11's `before/`.
    - **The run**: `target/release/golf-lab.exe reanalyze --all --verbose`, 11:24:32 to 11:26:43,
      exit 0, `Re-analyzed 15/15`.
    - **Its log**: 13 `re-reading with screen parser 1` lines and 2 `from the shot store` lines, no
      `FAILED` and no `!` note. This is the rehearsal's log (item 8).
    - **The only other readers running** were two `scripts/run_mcp_server.py` processes. They are
      Claude Code's registered `golf-coach` server, and they only read.
16. **Step 3: every check holds, and each equals the rehearsal (item 8).**
    - **Files, against the backup**: 158 on both sides. Exactly 43 changed: 13 `*.shot.json`, 15
      `analysis.json` and 15 `analysis.state.json`. Nothing is new or gone, and every
      `keypoints.json` and `audio.json` is byte-identical.
    - **Shots**: all 13 are stamped at `parser_version` 1, with 14 fields present.
      - Values move at exactly `DECLARED_VALUES`' three: `2026-08-10-1`'s `shot_type` and
        `impact_position`, and `2026-08-23-1`'s `impact_position`.
      - M32's keys are added as null.
    - **`analysis.json`**: all 15 are at engine 17.
      - Outside the shot, only last-bit floats move: the three overall scores on 7 bundles, and
        `checkpoint_scores[5].score` on 6.
      - The one embedded key removed is `2026-08-23/1`'s `raw_fields["Impact Position"]`. That is
        the tie: located, unread.
      - No stored artifact held a coaching paragraph, so C5 dropped nothing.
    - **Frozen readers** (`frozen_readers.py`) found 0 problems.
      - All 137 GET routes keep their status (77 × 200, 45 × 206, 15 × 404, as before).
      - 23 routes answer differently: the session and swing pages, plus the three shared-shot
        flight routes, where only `shot_type` moved.
      - Frozen `read_corpus` differs only at 13 `swings[].analysis_version`, 16 → 17.
    - **Rust readers**: `golf-core`'s four career verbs, `profile` (text and `--json`) and
      `golf-lab flight --shots` are byte-identical before and after. The dry run now lists all 15
      as up to date.
    - **`golf-mcp`**: 55 calls, 0 errors before and after. Only `get_swing` (15), `list_sessions`
      and `get_recent_shots` answer differently.
    - **Python pins**: the four (item 11's `-k`) pass over the written `data/`, and so does the
      whole of `tests/test_conformance.py`: 272 passed, 100 s.
17. **The new digest is `07f37373abe42fbe333ae263590a626577f3becd2c15b869dd83c47f44ad732d`**, over
    192 files, by the Verify line's method. **For P30–P36**: compare against this, not
    `bd384cc4…`. C12's backup stays where it is as the record of `data/` before P29, and it is valid
    while `SHA256SUMS` hashes to `a7a9436d533f…`. No phase deletes it.
18. **Verify, after the write.**
    - `cargo test -p golf-lab --no-fail-fast -- --include-ignored`: 141 passed, 0 failed.
      - `reread` passes in 0.01 s, since nothing is read now.
      - `inputs` passes in 15 s, `import_real` 61 s, `strikes` 39 s and `keypoints` 36 s.
    - `cargo test -p ocr --no-fail-fast -- --include-ignored`: 127 passed, 0 failed, about 22 min.
      The package is `ocr`, not `golf-ocr`.
    - `cargo test`: 1,610 passed, 23 ignored, as P28.
    - `cargo clippy --all-targets` and `cargo fmt --check`: clean.
    - `git diff -- spec/vectors/{synthetic,…,profile}` is empty, and `spec/declarations/` is
      untouched.
19. **For P30**: `data/` is Rust's now, so the builder reads Rust's artifacts.
    - Every `analysis.json` is at 17.
    - Each embedded shot is the re-read one, with P25's declared paths.
    - The three overall scores moved at the last bit on 7 bundles, and `checkpoint_scores[5].score`
      on 6 (about 1e-15 relative). The committed corpus vectors still carry frozen Python's
      numbers.

    So a builder that verifies against the stored `analysis.json`, as
    `conformance_vectors.py::_verify_against_stored` does, meets those floats and the version.
    Expect them, and list them in P30's findings; do not re-record. The real
    storage corpus now differs from the committed one at the 13 `analysis_version`s (item 16).
20. **For P32–P34**: the four `tests/test_conformance.py` pins that read `data/` now follow the
    written store.
    - `_selection_corpus` and `_career_real` carry P29's only frozen-tooling edit (items 12 and 14).
    - If P33–P34 move `scripts/conformance_vectors.py`, `_excusing_newer_engine_versions` moves
      with `_career_real`.
21. **For P36**: P29's frozen edits are `scripts/conformance_vectors.py`
    (`_excusing_newer_engine_versions`, `_career_real`, `_selection_corpus`) and the two
    docstrings in `tests/test_conformance.py`. They are sanctioned by item 12 and belong in the one
    commit.
22. **Scratch**: item 11's throwaways and before/after snapshots were deleted at the end of the
    phase, as were this session's `jdiff.py`, its logs and its `after/`.

### P30 (2026-10-28) — `golf-lab build-vectors`: the 15 corpus and stage vectors rebuilt from `data/` match the committed ones but the re-read shot, and the real storage corpus differs in five explained kinds

1. **What landed.** No new crate, edge or dependency, and `Cargo.lock` is unchanged.
   - **`crates/lab/src/build_vectors.rs`** (new) ports three of `conformance_vectors.py`'s builders over
     `crates/storage`'s readers: `_corpus` with `_corpus_vector`, `_identity`, `_strikes` and
     `_source_frames`; `build_stages` over what it builds; and `_storage_corpus_real` with
     `_real_sessions_tree` and `_slim_analysis`.
     - `build(sessions, golfers, log)` returns `Vec<Built { id, path, bytes }>`. It builds and
       verifies every vector, and writes nothing.
     - `write(built, out, dry_run)` returns one `Fate` per vector: `Written`, `WouldWrite`,
       `Refused` or `Failed`.
     - `build_vectors(request, out, err)` is the verb, returning its exit code.
     - Also public: `Request`, `BuildError` (`NoSessions`, `Unreadable`, `Unverified`),
       `RECORDED_BY`, `REAL_PLAYER` and `REAL_STORAGE_ID`.
   - **The verb**: `golf-lab build-vectors [--sessions-dir D] [--golfers-dir D] [--out DIR]
     [--dry-run]`, in `bin/golf_lab.rs` with USAGE and the binary's doc.
     - `--out` is the vectors root, `spec/vectors/` by default.
     - The USAGE's "arrives as its script is" line now names only `import-export`.
   - **`golf_core::rerecord`** gained two things:
     - `new_vector_bytes(document, path)`: `file_bytes` at `Layout::RECORDED`, so a built vector and
       a re-recorded one come out of one writer;
     - `RUST_ORACLE`, next to `PYTHON_ORACLE`.
   - **`golf_core::storage_family`** changed in two ways:
     - `run_corpus` is split, and `read_and_narrow(root, input)` is now public. It is
       `_read_and_narrow`, which the builder's verify reads `data/` through.
     - `StorageCase::of` reads `worked_against` for `oracle: "rust"` as it already did for `"hand"`.
       A Rust-recorded case cannot name `_run_corpus` in `recorded_by`, because no Python ran.
       A unit test pins it.
   - **Tests.**
     - `crates/lab/tests/build_vectors.rs` (new) has six cases on `tests/bundle/mod.rs`' scratch
       bundle and one ignored gate on `data/` (item 4). The bundle is analysed by `golf-lab analyze`
       first, with a window on each view, so the cut moves frames, strikes and phases. The six:
       - the three written vectors are each answered as recorded, by `run`, `run_stages` with
         `verify_compose`, and `run_storage`;
       - a second build refuses all three by name and leaves their bytes, and a file standing at
         one path is refused alone;
       - a dry run writes nothing, not even the root;
       - a stale artifact is skipped with the line, and a missing sessions directory is refused;
       - a tampered `unscored` or phase stops the build before any write.
     - `tests/golf_lab.rs` gained the verb's refusals, and a dry run into the committed
       `spec/vectors/` that refuses `storage/corpus/real` by name with exit 1.
     - Unit tests: three in `build_vectors`, one in `storage_family`, and one in `rerecord`
       (`a_new_vector_is_written_as_write_json_wrote_one`).
     - `tests/common/mod.rs` now holds `REREAD_TEXT` and `rust_only_shot_keys`, moved out of
       `tests/inputs.rs` unchanged, so the two gates share one shot rule. `swing_dirs` carries an
       `allow(dead_code)`, because the new binary includes the module and walks the store through
       the builder instead.
   - **Docs**: `crates/lab/src/lib.rs` gained a `build_vectors` bullet, and `crates/lab/Cargo.toml`'s
     `storage` and `golf-core` comments name what P30 crosses.
2. **Calls this phase made** (each in `build_vectors`' module doc).
   - **A refusal is per file.** D19 says the builder "writes only vectors that do not exist and
     refuses to overwrite a committed one". So each absent file is written and each present one is
     refused by name and left alone.
     - The exit code is 1 when anything was refused, 0 when nothing was, and 2 when the build
       stopped or a write failed.
     - Every vector is built and verified before anything is written, so a failed verify writes
       nothing.
     - A file is opened `create_new`, so one that appears between the build and the write is
       refused rather than replaced.
     - The alternative was refusing the whole run when any target exists. It was rejected because
       the 15 committed vectors would then block every new bundle's vector, and the real storage
       case, which is one file over every session, would block every run.
   - **Provenance**: `oracle: "rust"` and `recorded_by: "golf-lab build-vectors"` on all three kinds,
     with no `python_version` or `pydantic_version`. The storage case adds `worked_against:
     scripts/conformance_vectors.py::_run_corpus`.
   - **The real corpus is read under Rust's engine pair**, `{installed: 17, comparable_from: 14}`,
     as every Rust verb reads it. `_REAL_PLAYER` is the constant `REAL_PLAYER = "aaron"` and not a
     flag, as in the twin.
   - **The slim and the tree carry what Rust's reader opens**: `swing.unscored` beside
     `measurements` and `shot`, and a photo entry's `shot_analysis.json` whole. The verify is what
     needed the first: item 5.
   - **`_verify_against_stored` is ported** (P25 item 9 had left it out), under §3's rules via
     `golf_core::compare`. The two other verifies are `verify_compose` and the real tree read
     against `data/` by exact `Value` equality, as Python's `!=`.
   - **The skip line** is `_corpus`' line, with `golf-lab reanalyze` in place of
     `scripts/reanalyze.py`.
   - **The slim is spelled by `serde_json`**, with keys sorted and no spaces, where `json.dumps`
     kept the file's order with spaces. The reader parses it, so only the value counts.
3. **The run.**
   - `target/release/golf-lab.exe build-vectors --dry-run` over `data/`, into the committed
     `spec/vectors/`, took 4.2 s. It built and verified all 31 vectors (15 corpus, 15 stages, 1
     storage), exited 1, refused all 31 by name and wrote nothing.
   - Into scratch, it wrote 31 and exited 0.
   - The debug gate (item 4) takes 20 s.
4. **P30's Done line holds** (`every_stored_bundle_builds_its_committed_vectors_but_the_re_read_shot`).
   - **The 15 corpus vectors.**
     - `input` and `expected` equal the committed ones under §3's rules, except inside the shot:
       `input.shot` and `expected.swing.shot`.
     - There, every difference is one of four declared kinds, `tests/inputs.rs`' rule:
       - a `DECLARED_VALUES` move, seen `[3, 3, 1]` on both sides;
       - the stamp: `parser_version` absent in the input and 0 in the answer, becoming 1, and
         `fields_present` absent or null, becoming the 14 tiles;
       - the other v17 keys, added to the input at their defaults;
       - the re-read's provenance text.
     - `id`, `analysis_version` (17), `note`, `frame_offset` and `source_clip_frames` are equal.
     - Provenance differs only in `oracle` (`python` → `rust`), the added `recorded_by` and the
       absent `rerecords`.
     - **P29's last-bit floats (its item 19) do not show.** They are about 1e-15 relative, inside
       §3's 1e-9.
   - **The 15 stage vectors**: `stages` is equal with no difference at all, and `id`,
     `analysis_version` and `derived_from` are equal.
   - **The real storage corpus** differs from the committed one in exactly these kinds, each counted
     by the gate:
     - **provenance**: `oracle`, `recorded_by`, `worked_against`, `analysis_version` (16 → 17) and
       `note`. Python's two versions and `rerecords` are gone, and `career_version` is 3 on both.
     - **`input.versions`**: `{16, 16}` → `{17, 14}`, Rust's pair (item 2).
     - **`input.files`**: the same 57 keys.
       - The 15 `manifest.json` and 12 `session.json` texts are identical.
       - The 15 `analysis.state.json` differ only at `started_at`, `completed_at` and
         `duration_seconds` (45 values), which is P29's re-run.
       - The 15 slimmed `analysis.json` differ at `analysis_version` 16 → 17 (P29), at the added
         `swing.unscored` (Rust's slim), and at the shot's declared paths, seen `[3, 3, 1]`.
     - **`expected`**:
       - `swings[].analysis_version` 16 → 17 on 24 entries (13 in the whole corpus, 6 under
         `club-3w`, 5 under `club-7i`), which is P29;
       - the shot's declared paths, seen `[1, 1, 2]`, because the shared photo's three bundles
         collapse to one swing;
       - **`swings[].shot_unscored` added on 22 entries** (11, 6 and 5): item 5.
       - Everything else is equal: the exclusions, the counts, all nine properties and both
         narrowings.
5. **Found: the committed real corpus cannot reach `shot_unscored`, and a builder that slims as
   Python does fails its own verify.**
   - `_slim_analysis` keeps `measurements` and `shot` only. Since M37, Rust's `read_corpus` lifts
     every `swing.unscored` entry that is not a checkpoint onto `CorpusSwing::shot_unscored`.
   - 11 of the 13 corpus swings carry a flight refusal: every `2026-08-23` bundle.
     - `flight_carry_yds`: `carry_unreachable` × 7, `no_club_loft` × 2, `spin_not_recoverable` × 1.
     - `flight_landing_offline_yds`: `spin_axis_unresolved` × 1.
   - So, under Rust's reader and since `career-v3`, `storage/corpus/real` does not read as `data/`
     reads, and neither do the cases that adopt its answer: `career/real/*` and
     `profile/corpus/real-aaron`.
   - **It moves no grade.** `golf-core profile --name Aaron --json` over `data/`, which does lift
     them, equals `profile/corpus/real-aaron`'s `expected.profile` with no difference. Its text
     equals `expected.report`.
   - `contracts::career`'s `CAREER_VERSION` doc says "No recorded artifact's `unscored` holds a
     flight row". That is true of the recorded trees only because the slim dropped the key. The
     real store holds 11.
   - Nothing was re-recorded (P30's rule). The builder's slim keeps the key, and its verify holds
     the tree to `data/`.
6. **Nothing committed, nothing under `data/` written.**
   - The digest is still `07f37373…` over 192 files.
   - `git diff -- spec/vectors/{synthetic,…,profile} spec/declarations` is empty.
   - No file under `src/`, `scripts/` or `tests/` changed after the marker.
   - Every dry run into `spec/vectors/` refused, as designed.
7. **For P35.**
   - **ARCHITECTURE §1** names `golf-lab build-vectors` and its flags and exit codes.
   - **CONFORMANCE §4** says that a new vector cut from `data/` is `golf-lab build-vectors`', with
     `oracle: "rust"`, and that `golf-core rerecord` stays the only re-writer.
   - **CONFORMANCE's storage section** says that a Rust-recorded case names its runner in
     `worked_against`.
   - **Item 5's finding** belongs beside the `career-v3` notes, along with the `CAREER_VERSION` doc
     sentence it qualifies, which the user may want reworded.
   - **A built vector committed later fails Python's pins.** `tests/test_conformance.py`'s storage
     pins assert `oracle == "python"` on every file of the Python-recorded sub-families,
     `storage/corpus/` among them. Frozen Python's `check` runs every committed engine vector and
     reads no `oracle`, so it would hold a built one to frozen Python's own answer. That was not
     measured. So the change that first commits a built vector must answer for both (M40 deletes
     them).
8. **For P36**: P30's files for the one commit:
   - new: `crates/lab/src/build_vectors.rs` and `crates/lab/tests/build_vectors.rs`;
   - changed: `crates/lab/src/{lib.rs, bin/golf_lab.rs}`, `crates/lab/Cargo.toml`,
     `crates/lab/tests/{common/mod.rs, inputs.rs, golf_lab.rs}`, and
     `crates/core/src/{rerecord.rs, storage_family.rs}`.
9. **Green.**
   - `cargo test`: 1,622 passed, 24 ignored, exit 0. That is P29's 1,610 and 23, plus this phase's
     12 tests and its one ignored gate.
   - `cargo test -p golf-lab --no-fail-fast -- --include-ignored`: 152 passed, 0 failed.
   - `cargo clippy --all-targets` and `cargo fmt --check` are clean.
   - The scratchpad's throwaways (the probes, the edit scripts, the built tree and the logs) were
     deleted.

### P31 (2026-10-28) — `golf-lab import-export`: the phone's export merged file by file, one conflict refusing all, and the imported corpus equal to the phone's

0. **Resumed.** The 12:36 attempt had written nothing but the 🔄 marker: `find … -newermt '2026-10-28
   12:36'` listed only this file, and its scratchpad was empty. So the phase ran from the start.
1. **The new dependency, as the section asks: `zip` 7.3.0.** It is a finding first, so here it is.
   - **Declared in `crates/lab/Cargo.toml` alone**, as `zip = { version = "=7.3.0",
     default-features = false, features = ["deflate-flate2"] }`. The comment there says why.
   - **Why 7.3.0.** It is the last release whose `rust-version` (1.83) the workspace's 1.87 meets.
     7.4.0 and every 8.x and 9.x want 1.88, measured from the crates.io index.
   - **Why these features.** Default features off drops AES, bzip2, deflate64, LZMA, PPMd, xz,
     zstd, zopfli and `time`. A phone export is stored or deflated, and any other method is refused
     by name. `deflate-flate2` alone inflates through the workspace's own `flate2`, with the backend
     it already has. A feature that named a backend (`deflate-flate2-zlib-rs`, which plain `deflate`
     enables) would have chosen one for every crate sharing the lock, and the `.gz` vectors are
     written through `flate2`.
   - **`Cargo.lock` gains five packages**: `zip` 7.3.0 and its four required dependencies,
     `indexmap` 2.14.2, `hashbrown` 0.17.1, `equivalent` 1.0.3 and `typed-path` 0.12.4. `crc32fast`,
     `memchr` and `flate2` were already there, and no existing package moved.
   - **Rejected alternatives.**
     - A hand-written reader over `flate2` was rejected for ZIP64, data descriptors and name
       encodings: a format parser is the kind of second copy `rustfft`'s comment declines.
     - Shelling out to `tar` was rejected because GNU tar reads no zip, so the verb would work on
       Windows and macOS and not on Linux.
   - **The pin needs nothing new.** `zip` sits in `golf-lab` only, so it stays off `golf-core`,
     `storage` and M38's path. It is not `ort`, so `crates/ocr/tests/ort_reach.rs` does not apply.
2. **What landed.**
   - **`crates/lab/src/import_export.rs`** (new, no Python twin). `import_export(request, out,
     err) -> io::Result<u8>` is the verb. Also public: `Request {export, sessions_dir, shots_dir,
     golfers_dir, dry_run}` and `TREES`.
   - **`crates/lab/src/import.rs`**: `Entries::holding`'s body moved into
     `pub(crate) fn session_holding(store, session_id, digest)`, which both verbs call. That is P24
     item 9's pointer taken: `import-photos` and `import-export` cannot disagree about which photo a
     session holds. `Entries::open` was not reused, because an export's files are placed at their
     own paths, not by the store's next swing id.
   - **The verb**: `golf-lab import-export EXPORT [--sessions-dir D] [--shots-dir D] [--golfers-dir
     D] [--dry-run]`, in `bin/golf_lab.rs`, with USAGE, the binary's doc and `golf-core`'s
     `data_dir` for the three directories.
     - The USAGE's "arrives as its script is" line is gone: every verb has arrived.
     - Flags are argparse's: `EXPORT` is required, a second positional is unrecognized, and
       `--dry-run=x` is an ignored explicit argument, each exit 2.
   - **`crates/lab/src/lib.rs`** gained the module and its bullet.
3. **Calls this phase made** (each is in the module doc).
   - **What an export is (D16).** A zip or a directory holding `sessions/`, `shots/` and `golfers/`.
     One wrapping folder that is none of the three, such as a zipped `processed/`, is read through.
     Anything else at the top is refused.
   - **The merge is per file, and the comparison is by bytes, not values (C11).**
     - A file is new (copied), identical (skipped) or a conflict.
     - **One conflict refuses the whole import.** Every conflicting path is named, as `<export path>
       -> <stored path>`, and nothing is written. That includes files that were new.
     - Bytes and not values, because a golfer or session cursor the phone wrote for itself differs
       in its timestamps. Which `created_at` is the golfer's is not the verb's to guess (item 6).
   - **The two dedupes by photo**, the section's "dedupe shots by photo sha", read at both levels
     where the stores already have a rule:
     - **A shot** is placed at `ShotStore::path_for(key_for(shot))`, not at the export's file name,
       and the report says so when the two differ. A shot keyed by its photo, when the store already
       holds that photo's shot with other bytes, is **`kept`**: the store's copy stays and the line
       says so, because the store answers `cached` for a photo it has parsed. A shot keyed by
       `shot_id` (no photo hash) has no such rule, and a difference is a conflict. Two export shots
       under one key are a conflict as well.
     - **A photo entry** (a shot photo and no face-on clip) whose photo its session holds in another
       swing, or in its own swing with some file different, is a **`duplicate`**. It is skipped
       whole and the stored swing stays. This is `import-photos`' rule through `session_holding`.
       - A swing earlier in the same export holds its photo as a stored one would, filmed or not.
       - **A filmed swing is never deduped**, since the corpus prefers its clips.
       - **The same swing id stored with files missing and none different is filled in**
         (`added`), not deduped: an entry the lab never analysed gains its `shot_analysis.json`.
     - Either dedupe is printed, so a correction made on the phone that the stored copy lacks is
       seen.
   - **Refused before anything is compared (exit 2, nothing written), every reason listed**:
     - a name that is not a plain relative one (`..`, absolute, a `\` or `:`, a control
       character), a link, an encrypted entry, a method other than stored or deflated, a name the
       archive holds twice;
     - a file outside the three trees, or an export with none of them;
     - a `*.shot.json` that does not read as a shot, a `manifest.json` that does not read as one, a
       manifest naming a file its directory lacks;
     - an archive entry whose checksum fails. Every new entry is read through before any write.
     - Only the files the rules read are parsed. The rest go by bytes, and the tolerant readers
       judge them as they judge any file.
   - **Ignored and counted, never copied**: a name starting with `.` anywhere in the path
     (`.DS_Store`, the store's `.incoming/`), `__MACOSX/`, and `*.tmp` and `*.part` leftovers.
   - **The write.**
     - The order is golfers, then shots, then sessions, with a swing's `manifest.json` last, so a
       stopped import leaves no swing a reader half-sees.
     - Each file is written as `<name>.<pid>.part` beside its place, given the export's time, and
       renamed. A file that appeared since the compare is refused.
     - A write that fails names itself and how many were copied before it, and exits 2. A re-run
       finishes, because what was copied is identical by then (pinned).
   - **Times.**
     - A directory's file times are kept.
     - A zip entry's time is kept from its extended-timestamp or NTFS field, both UTC. The DOS time
       is local and zoneless, so it is never read, and an entry with neither field gets the write's
       time.
     - That matters only to a fresh read of the photo, whose shot takes the photo's time (P23 keeps
       a re-read's identity).
   - **A path under a stored file reads as absent** on both platforms (Windows says `NotFound`,
     Unix `NotADirectory`), so the write refuses it the same way everywhere.
   - **Exit codes**: 0 imported (or importable in a dry run), 1 a conflict, 2 not a stored layout or
     a write failed part way.
4. **Done, as the section words it**, in `crates/lab/tests/import_export.rs` (12 tests, on any
   machine).
   - **The fixtures are built by the real writers.** `golf-lab import-photos --into-session` runs
     over a reader replaying `2026-08-23-2`'s committed boxes, and `set_current_club` adds
     `session.json`. That gives a photo-only session with its `shot_analysis.json`, a shot, a golfer
     and a bag (the storage family's), which is what M38's phone will write.
   - **Imported into an empty lab**: 7 files copied, the lab's tree byte-equal to the phone's, and
     the photo's time kept.
     - **`read_corpus` over the lab equals `read_corpus` over the phone**, as a whole
       `CareerCorpus`. That is M38 P4's exit, pinned now.
     - One entry is pooled, with nothing excluded, `face_on_sha256` none, `shot_sha256` the photo's,
       and its shot lifted.
   - **A re-import** copies nothing and leaves every file and time as it was.
   - **A conflict** (a left-handed `aaron` and another `session.json`) exits 1. It names both, in
     the order they would be written, and writes nothing, not the new shot or the swing either.
   - **The dedupes and fills.**
     - A re-read shot is `kept`, and so is an edited `shot_analysis.json`: its entry is a
       `duplicate` of swing 1, and then of swing 7 once the lab's entry is renumbered.
     - An entry missing its `shot_analysis.json` is `added`.
     - An export repeating its own photo has entry 10 a duplicate, with filmed entry 2 not deduped.
     - A shot under a foreign file name lands under its key, and a `shot_id`-keyed shot conflicts.
   - **A zip** under one `processed/` folder, deflated, with extended timestamps and a Finder's
     `__MACOSX/` and `.DS_Store`, imports as the directory does: the same bytes, the photo's time,
     the same corpus, 2 ignored. A second import from the zip is identical throughout.
   - **The refusals**: outside the trees, a torn shot and a manifest naming a missing photo (listed
     together), `..` and `\` in archive names, a missing path, a file that is not a zip, and an
     export with only `.incoming/`. Each exits 2 with nothing written.
   - **A write failing part way**, with `sessions` a file in the lab, names `session.json` and "3 of
     7". A re-run, once the cause is gone, copies 4 and finds 3 identical.
   - **The binary**: five argparse refusals, `--help`, and a dry run through all three directory
     flags.
   - **Unit tests in the module** (5): plain names, the ignore rule, the wrapping folder, the swing
     order the store numbers by, and both UTC time fields read off their bytes.
5. **What this does not do, on purpose.**
   - **No re-read, no re-analysis, no stamp.** An imported file is the phone's bytes, which is what
     makes the two corpora comparable. A stale entry is `golf-lab reanalyze`'s, afterwards.
   - **No manifest hash check.** A manifest's role files must be present, but their sha256 is not
     recomputed, because hashing every clip of a filmed export costs more than a torn copy that the
     checksum and the presence check already catch.
   - **Windows' case-insensitive names are not folded.** Two export files differing only in case
     would map to one stored path. No store writes such a pair.
   - **Not run against frozen Python.** It is not in this phase's Verify. The files are
     `crates/storage`'s bytes, unchanged. P26 showed frozen Python reads the bundles that crate
     writes, and frozen Python's corpus leaves photo entries out by design. No phase has read a
     photo entry with frozen Python's server.
6. **For M38 P1 and P4 (the program plan's §M38)**, what this importer requires of the phone's
   export. **For P35**: carry these into §M38 P4.
   - **Golfers and bags must arrive byte-identical or absent.** A golfer the phone created for
     itself differs in `created_at` from the lab's `aaron.golfer.json`, and a bag edited on the
     phone differs too. Either refuses the whole import.
     - So the phone wants seeding with the lab's `golfers/`, or a decision on which side owns a
       golfer.
     - That decision is the user's, not this verb's. A value-level golfer rule would be a new row
       in C11, not a quiet change.
   - **Sessions the two devices share conflict on `session.json`**, as the same date used on both
     would. A phone session id that cannot collide with the lab's, or sessions recorded on one
     device only, avoids that.
   - **A cumulative export re-imports clean** as long as the lab has not rewritten a phone file.
     - A photo entry the lab re-analyses afterwards reads as a `duplicate` and is kept as the lab's.
     - A rewritten `session.json`, golfer or bag conflicts.
   - **An imported shot's `provenance.image_path` names the phone's path**, as an `import-photos`
     shot names the lab's. Nothing in the lab opens a photo by it: the re-read takes the bundle's
     photo from its manifest. Nothing rewrites it either, since that would change the phone's bytes.
   - **A zip with an extended-timestamp or NTFS field keeps each photo's time.** Without one, the
     copy's time is the import's.
   - If the phone zips in Rust, the same `zip` pin keeps one zip implementation in the lock.
7. **For P32: `import-export` is now a Rust route that registers a golfer.** P24 item 10 said that
   after D11 nothing in the lab could add a new golfer until a Rust verb or M38 did. This is one, but
   only from a phone's export: a `golfers/<id>.golfer.json` the lab lacks is copied in. The
   question P24 item 10 left for the user at P32 is narrower by that much.
8. **For P35.**
   - **ARCHITECTURE §1** gains `golf-lab import-export` with its flags and exit codes.
   - **§4** says `data/processed/` can also be written by an import, of the phone's own bytes.
   - **ADR-035's addendum** names `zip` beside `ort` and `rmcp` as M29's dependencies (item 1).
   - **ADR-016's addendum** can say that the phone's data reaches the lab by export import, once
     P32 closes the upload route.
   - **CLAUDE.md's Commands** gains the verb.
   - The crate graph does not change: no new workspace edge.
9. **For P36: P31's files for the one commit.**
   - new: `crates/lab/src/import_export.rs` and `crates/lab/tests/import_export.rs`;
   - changed: `crates/lab/{Cargo.toml, src/lib.rs, src/import.rs, src/bin/golf_lab.rs}` and
     `Cargo.lock`.
10. **Green.**
    - **`cargo test`** (the whole workspace, `CARGO_INCREMENTAL=0`): 1,639 passed, 0 failed, 24
      ignored, exit 0. That is P30's 1,622 and 24, plus this phase's 12 integration and 5 unit
      tests. No ignored gate was added, since nothing here needs `data/` or the models.
    - **`cargo clippy --all-targets`** and **`cargo fmt --check`** are clean.
    - **Data**: the `data/processed` digest is still `07f37373…` over 192 files, and nothing under
      `data/` was read or written. Every run worked in temporary directories.
    - **Diffs**: nothing under the ten held families or `spec/declarations/`. No `src/`, `scripts/`
      or `tests/` file changed after the marker.
    - **Docs**: `pytest tests/test_docs_truth.py` passes; it was run because this file was edited.
    - **Scratch**: the zip-manifest probes, the index dump, the lock copy, the test-file backup and
      the test log were deleted.

### P32 (2026-10-28) — stopped before any edit: five of D11's routes had no Rust verb to name; answered (a), resumed and done

**How this phase stopped.** Nothing was edited but this file: no `api/app.py` change and no test
change. Nothing under `data/` was read or written. The State cell stays 🔄. P13 item 6 and P24
item 10 both said P32 must put this question to the user and not settle it alone. P31 item 7
narrows it.

1. **D11's closed routes, each with the Rust verb that replaces it.** Measured against
   `golf-core`'s and `golf-lab`'s dispatch (`crates/core/src/bin/golf_core.rs`,
   `crates/lab/src/bin/golf_lab.rs`).
   - **Three have a verb:**
     - `POST /api/uploads` is replaced by `golf-lab import-export` (a phone's stored layout, D16)
       and `golf-lab import-photos --into-session S --player P --club C` (photos).
     - `POST …/swings/{w}/mishit` is replaced by `golf-core flag-mishit`.
     - `POST …/swings/{w}/analyze` is replaced by `golf-lab analyze SESSION/SWING`.
   - **Five have none:**
     - `POST /api/sessions/current/golfer`: registers a golfer (`get_or_create`), sets the cursor
       and backfills unlabeled swings.
     - `POST /api/sessions/current/club`: sets the cursor, which only the upload route reads.
     - `POST …/swings/{w}/golfer`: re-attributes one swing.
     - `POST …/swings/{w}/club`: retags one swing. Its docstring calls it "the club's **only**
       repair path" (ADR-024 §5), and `golf-mcp`'s `NOTHING_TAGGED` (`crates/mcp/src/club.rs`)
       names it.
     - `DELETE …/swings/{w}`: removes a phantom swing.
2. **The storage half already exists in Rust, and no verb calls it.**
   `storage::bundle_store::{set_player, set_club, delete_swing, attribute_unlabeled}`,
   `storage::session_meta::{set_current_player, set_current_club}` and
   `storage::golfer_store::get_or_create` are all ported. The storage family gates them
   (`crates/core/src/storage_family.rs`: `bundle.set_player`, `bundle.set_club`,
   `bundle.delete_swing`, `golfer.get_or_create`). A verb over them is a shell, with no new
   storage code and no new dependency.
3. **Leaving the five open is not safe.** `crates/core/src/session.rs`' module doc ("The
   frozen-Python hazard") measured this. Frozen Python's `SwingManifest` and `SessionMeta` drop
   `intent` on every rewrite: `set_player`, `set_club`, `set_mishit`, `attribute_unlabeled` and a
   cursor update. A drill retagged through the frozen route silently becomes a tracked shot. §M29
   "is what retires the hazard".
4. **What closing them with no verb costs.** Until M38, the lab could not retag a swing's club or
   golfer, delete a phantom, or register a golfer other than by importing a phone's
   `golfers/<id>.golfer.json` (P31 item 7). The only way left would be editing `manifest.json` by
   hand.
5. **The question: what do the five refusals name?**
   - **(a) Add three `golf-core` verbs in this phase**, beside `set-intent` and `flag-mishit` in
     `golf_core::session`, over the item 2 functions, so every refusal names a verb:
     - `set-golfer --session S --swing W --name N [--handedness H]`, which registers through
       `get_or_create`;
     - `set-club --session S --swing W --club C`;
     - `delete-swing --session S --swing W`.

     The two cursor routes then refuse naming `import-photos --into-session`'s `--player`/`--club`
     and `set-golfer`/`set-club`, since nothing reads a cursor once uploads close. `NOTHING_TAGGED`
     becomes a declared difference naming `golf-core set-club`, with P9 item 10 as the precedent.
     This adds Rust work (verbs, tests in `crates/core/tests/verbs.rs`, one `family::DECLARED`
     row) to P32's Python work. **Recommended:** it is D11 as written, and it keeps ADR-024 §5's
     repair path.
   - **(b) Close all eight, and the five name M38.** Each refusal says the repair is the phone's
     until then. `NOTHING_TAGGED` is declared the same way. This is the least work, and it loses
     the club repair path in the lab until M38.
   - **(c) Narrow D11 to the three with a verb**, and leave the five open. This keeps item 3's
     hazard alive, so it is not recommended.
6. **The user's answer (2026-10-28): (a).** Add the three `golf-core` verbs in P32 —
   `set-golfer`, `set-club` and `delete-swing`, beside `set-intent` and `flag-mishit` in
   `golf_core::session` and over item 2's functions — and close all eight routes, each refusal
   naming its verb as item 5 (a) describes. The next P32 session resumes from here and builds that.
7. **Resumed from item 6.** The first session had left nothing in the tree but this file:
   `find … -newermt '2026-10-28 20:05'` listed only it. The second built (a) as answered.
8. **The three verbs are in `golf_core::session`**: `RepairVerb`, `Repair`, `RepairAction`,
   `repair_action` and `repair`. `golf-core set-golfer|set-club|delete-swing` dispatches to them
   through `parse_repair` (`crates/core/src/bin/golf_core.rs`). They call item 2's functions, with
   no new storage code and no new dependency.
   - **Flags.**
     - All three take `--session S --swing W` and `--sessions-dir D`.
     - `set-golfer` adds `--name N [--handedness right|left] [--golfers-dir D]`.
     - `set-club` adds `--club C`, in any spelling `parse_club` takes.
     - A flag that belongs to another of the three is refused as unrecognised.
     - Exit codes are `set-intent`'s: 0 on a write; 2 on a refusal, with nothing written; 1 when a
       file cannot be written.
   - **Each keeps its route's rule, with three differences**, all in the module doc:
     - **The swing is checked before anything is written.** The golfer route registered a new
       golfer before it learned the swing was missing.
     - **The ids must pass the routes' `_SAFE_SEGMENT`** (hand-ported as `is_safe_segment`), and
       the delete also needs a manifest that reads. Without that, `--swing ..` would name the
       session's parent, and `delete_swing` would remove it whole.
     - **A changed golfer or club warns when an analysis is stored.** If the swing has an
       `analysis.json` or `shot_analysis.json`, the line ends "re-run golf-lab reanalyze S/W".
       Neither value is among the inputs `analysis.state.json` hashes, so nothing else marks
       that result as old.
   - **What each prints:**
     - `S/W: golfer = aaron, was brian`, then `(registered now: Dave Smith, left-handed)` for a
       new golfer, or `(aaron is on file as right-handed, so --handedness left changed nothing)`;
     - `S/W: club = pw, was 7i`;
     - `S/W: deleted, with every file in its directory (it held face_on)`.
     - The refusals keep the routes' sentences: `'Rory' is new here — say whether they swing
       right- or left-handed, …`, `'banana' is not a club in the bag — try …`, and `S/W is being
       analyzed (running) — wait for it to finish, then delete it`.
   - **Tests.** Five new ones in `crates/core/tests/verbs.rs`, on scratch copies of
     `mixed-session`. One shows a drill's `intent` surviving `set-golfer`: item 3's hazard,
     retired.
9. **`golf-mcp`'s `NOTHING_TAGGED` now names `golf-core set-club`.**
   - **The sentence.** Its last clause was frozen Python's retag control and route, kept as
     `NOTHING_TAGGED_FROZEN_TAIL`. It now ends `with golf-core set-club --session <session>
     --swing <swing> --club <club>.` (`NOTHING_TAGGED_TAIL`). "New swings get a club at upload"
     stays, since the phone tags at upload and so does `import-photos --club`.
   - **The declaration.** `family::DECLARED` gains an eighth row: `get_bag_profile` at `note`
     (`retag_through_golf_core`).
   - **What it moves.** 4 recorded calls, all Python-recorded `get_bag_profile` answers:
     `career-refusing`, `career-speaking`, and `club-untagged` twice.
   - **Nothing written.** No hand or Rust case carries the sentence, the two gzipped cases
     included, so no vector was written.
10. **The frozen server, `api/app.py`.**
    - **One refusal for all eight routes.** Each answers **410** through `_closed(what, instead)`:
      `{what} moved to Rust at M29, and this server no longer writes swings. {instead}`.
      - The swing routes run `_safe` first, then echo the ids into the command: `Run golf-core
        set-club --session 2026-08-07-aaron1 --swing 1 --club C.`
      - The token guard still resolves first.
      - No handler reads or parses a body, so what used to be a 400, 404, 409, 422 or 503 is now
        the same 410.
    - **One block.** The eight routes sit together where the upload route stood, under a comment
      saying why the frozen server changed. The module docstring says it too.
    - **Dead code removed with the routes**: `GolferRequest`, `MishitRequest`, `_resolve_golfer`,
      `_status_message`, `incoming_dir` and seven imports. `ClubRequest` and `_resolve_club`
      stay, because the bag routes and the lookup use them.
    - **The page shows each sentence as written.** The upload page displays `detail` for every
      one of these controls (`static/index.html`), so the person at the bay reads it verbatim.
11. **The tests under `tests/api/`.**
    - **New: `conftest.py`'s `file_swing` fixture.** It files a swing the way the upload route
      did: the bytes go to `.incoming/` and are hashed, then `assign_from_path` runs with the
      golfer and club the cursors would have given. Eight files had built their disk state
      through the upload route.
    - **Reseeded.** The seed helpers take `file_swing` where they took `client`, and no assertion
      changed, in `test_bag_route.py`, `test_career_route.py`, `test_conversation_routes.py`,
      `test_flight_route.py` and `test_results.py`.
    - **Rewritten:**
      - **`test_uploads.py`.** The eight routes, parametrized: each refuses naming its verb,
        writes nothing, refuses whatever it is sent, and stays behind the token. The file keeps
        the read routes, the stored cursors (written with frozen `set_current_player`), the picker
        and the bind guard. The picker pin now calls `parse_club` on each offered club directly,
        since the cursor route it ran through is closed.
      - **`test_mishit_route.py`.** One refusal test, which also shows a stored verdict left as
        it was.
      - **`test_worker.py`.** No route submits to the worker now, so the tests drive it directly
        on a loop of their own: `start`, `submit`, recovery. Every lifecycle test is kept, and two
        are added: recovery re-runs an orphaned `queued` or `running` state, and the app's
        lifespan starts the worker it is given.
    - **No stale citations.** 52 test names left these three files, and none is cited anywhere
      outside `tests/api/`.
12. **What still names the frozen server's write paths, left alone here.**
    - **For P35 and the user: the worker's startup recovery is the one path left on which the
      frozen server writes a swing.**
      - **Why it is still there.** `create_app` still builds and starts `AnalysisWorker` when
        `settings.analysis_enabled` allows it. The plan keeps `api/worker.py`.
      - **What it does.** Nothing submits to it now. But `_recover`, at startup, re-runs two
        kinds of swing in today's session through frozen Python's pipeline at engine 16: one with
        an orphaned `queued` or `running` state, and a complete bundle with no result.
      - **How a fresh one could arise.** After the closure, only from `golf-lab import-export` of
        a complete filmed bundle dated today with no `analysis.state.json`.
      - **Why it was not fixed.** Turning the worker off is a one-line change to `create_app`'s
        default, but it is outside D11's list, so it is a question for the user, not a fix.
    - **For P34 and P35: `golf-core club-profile`'s text still names the closed route.**
      - **Where.** In `crates/core/src/reports/club_profile.rs`, its `next` paragraph names
        `python scripts/run_server.py`, the upload page's picker and `POST …/club`, and its
        `untagged` paragraph names the same route.
      - **Why it was not edited.** The text is `club_profile.py`'s, recorded in
        `spec/vectors/career/`, which this plan holds unchanged.
      - **What it needs.** When P34 deletes `club_profile.py`, changing the text needs either a
        career declaration, which the plan forbids, or a decision to leave it to M40.
    - **For P34 and P35: the lab's `loft_remedy`** (`crates/lab/src/pipeline.rs`) still tells a
      swing with no club to "tag one on the results page and re-analyze". Both of those controls
      now refuse, naming `golf-core set-club` and `golf-lab analyze`. P26 held the sentence to the
      script's text.
    - **Frozen text not on P32's sanctioned list**, left as it is: `api/worker.py`'s docstring
      ("Analyze anyway"), the controls in `static/index.html`, and `scripts/run_server.py`'s
      docstring. The page's controls now show the 410 sentences.
13. **For P35 and M38: a new golfer still needs a swing, or a phone's export.** `set-golfer`
    registers a golfer only while re-attributing a swing, and `import-photos --into-session`
    refuses an unregistered golfer (P24 item 4). So a golfer with no swing on disk can be added
    only from a phone's export (P31 item 7). That is item 5 (a) as answered. If it becomes a
    problem, the fix is a registration verb, or `import-photos --handedness`.
14. **For P35:**
    - ARCHITECTURE §1 gains the three verbs, their flags and their exit codes (`golf_core.rs`'s
      doc has them).
    - `CLAUDE.md`'s Commands gains the three verbs.
    - **ADR-016's addendum**: the upload path is closed, and the phone's data arrives by export
      import.
    - **ADR-024 §5's addendum**: the club's one repair path is now `golf-core set-club`.
    - **ADR-028's addendum**: the mishit route is closed, and `golf-core flag-mishit` records a
      verdict.
    - **ADR-035's addendum** answers its second deferred question with D11 as built, and with
      items 12 and 13.
    - The `mcp` family now has eight declared rows.
15. **For P36: P32's files for the one commit.**
    - new: `tests/api/conftest.py`;
    - changed: `crates/core/src/{session.rs, bin/golf_core.rs}`, `crates/core/tests/verbs.rs`,
      `crates/mcp/src/{club.rs, family.rs}`, `src/golf_coach/api/app.py`, and
      `tests/api/{test_uploads, test_mishit_route, test_worker, test_bag_route,
      test_career_route, test_conversation_routes, test_flight_route, test_results}.py`.
16. **Green.**
    - **pytest**, alone: 2,088 passed in 20 minutes. **ruff** is clean, and so is **mypy** over
      118 files.
    - **`cargo test`** (the whole workspace, `CARGO_INCREMENTAL=0`): 1,644 passed, 0 failed, 24
      ignored. That is P31's 1,639 plus this phase's 5. **clippy** (`--all-targets`) and **fmt**
      are clean.
    - **The career dry run** (`career-v3`, which walks `mcp` through `DECLARED`) found 0 changed
      over 175 vectors, 28 of them `mcp`, with the release `golf-mcp` answering.
    - **The frozen server over `data/`.** A throwaway answered every GET route twice, once
      through `HEAD`'s `app.py` and once through the closed one: 119 routes, 74 × 200 and
      45 × 206, no 5xx. All 119 answers are byte-identical. All eight closed routes answer 410 on
      a real swing (`2026-08-07-aaron1/1`).
    - **Data.** The `data/processed` digest is unchanged: `07f37373…` over 192 files.
      `sessions/.incoming` predates the phase (2026-08-22) and is empty.
    - **Diffs.** Nothing changed under the ten held families or `spec/declarations/`, and nothing
      under `spec/` changed after the marker. The `src/`, `scripts/` and `tests/` files changed
      after the marker are `api/app.py` and `tests/api/` only, which the rules sanction for this
      phase.
    - **Scratch.** The probe, its copy of `HEAD`'s `app.py`, the splice and reseed scripts and the
      test log are deleted.

### P33 (2026-10-28) — the archive move: the research record in `archive/`, every doc path rewritten, and the residue named

1. **What moved**, by `git mv`, so the renames are already **staged** in the index (the edits made
   after the move are not):
   - `scripts/golfdb/` → `archive/golfdb/`, `scripts/caddieset/` → `archive/caddieset/`, and
     `scripts/{hand_landmark_reliability,trigger_replay,pose_replay}.py` → `archive/`.
   - **`src/golf_coach/contracts/reference.py` → `archive/golfdb/reference.py`**, beside its only
     importer (the M31.5 inventory's R7). Placed there, not at `archive/reference.py`, because running
     a script puts its own directory on `sys.path`: the nine importers now say `from reference import
     ReferenceSwing`, the convention `import common` already uses, and no path hack is needed. Its
     docstring says where it came from. `ruff --select I --fix` re-sorted the eight top-level
     imports. Nothing in `src/` imported it, so `mypy src` lost one file and nothing else.
   - Every move keeps its depth under the repo root, so each script's `parents[2]` or
     `parent.parent` still finds the repo, and `bakeoff.py` still finds `docs/M4_POSE_BAKEOFF.md`.
2. **The archived scripts still import what they need.** A throwaway imported every module under
   `archive/` with its own directory first on `sys.path`, as running it does, plus the one outside
   importer: 29 of 29, no failure. `.venv` already has pandas, numpy, scikit-learn and MediaPipe.
   **`ruff check archive`** is clean, though no verify command reaches `archive/` (its README says
   so).
3. **The `research` extra is gone**, and `all` with it. A comment where it stood says why and points
   at `archive/README.md`, which says how to install the three libraries by hand. `hardware` is
   P34's and is untouched.
4. **No test loaded a moved file.** Finding 13's list was re-checked: the moved paths appear in
   tests only in docstrings and one comment (`tests/analysis/{test_joint,test_phases,test_trajectory}.py`,
   `tests/test_conformance.py`), and those were rewritten. **The one loader was
   `spikes/mediapipe-hands/probe.py`**, which imports `hand_landmark_reliability.py` through
   `sys.path`. Its line now names `archive/`, so the spike still imports.
5. **The doc paths, re-counted.** The five paths and `contracts/reference.py` were named by 121
   tracked files outside `WORKLOG.md` and `docs/plans/`. **The Markdown among them is exactly
   finding 12's 22.** The rest is code, comments, schemas and vectors.
   - A throwaway rewrote 182 occurrences across 68 files: the 22 Markdown files, the moved files'
     own usage lines and messages, and comments and docstrings in `src/` (11 files), `tests/`,
     `scripts/conformance*.py`, three crates (`contracts/src/dispersion.rs`'s doc comment,
     `pose/src/pool.rs`, `trigger/src/stream.rs`) and both spikes. `scripts/` and `archive/` are the
     same length, so nothing re-wrapped except one line in `tune_per_club_bands.py`.
   - **The `src/` edits are comment and docstring text only.** They were read as part of "the move"
     that the rules sanction, because ADR-035 §5 says the move rewrites the doc paths that name the
     scripts, and this phase's Done line says grep finds no stale path.
   - Hand edits covered what a rename could not: the `research` extra in `README.md`,
     `data/README.md`, ARCHITECTURE §1's tooling paragraph and the moved scripts' "Needs" lines;
     `README.md`'s tree, which gains an `archive/` row; and `caddieset/common.py`'s sentence about
     `ReferenceSwing`.
6. **What the grep still finds, kept on purpose.** Each is recorded data or a sentence about the move.
   - **The dispersion tolerances' provenance**, `` `python scripts/golfdb/tune_spatial_metric.py`,
     tolerance = noise + bound ``: in `contracts/dispersion.py`'s two strings, the two Rust constants
     beside them in `crates/contracts/src/dispersion.rs`, `crates/contracts/tests/data/python_tables.json`,
     and 45 `spec/vectors/career/synthetic/` vectors. A provenance string records what was run on its
     date, and rewriting it would move the career family, which the rules hold still.
   - **`contracts/swing.py`'s `Measurement` docstring**, which pydantic writes into three committed
     schemas (`spec/schemas/{swing_result,shot_result,swing_bundle_result}.schema.json`). Rewriting it
     means regenerating them, so it stays until M40 deletes the file.
   - **Text about the move itself:** ADR-035 §5's list of what moves, ROADMAP §M29's description of
     it, ADR-030's M31.5 bullet, and ADR-022's M31.5 addendum: its table row, its dated
     `derive_pose_metrics.py` line, and "The paths above are left as they are … nothing has moved
     yet".
   - `WORKLOG.md` and `docs/plans/`, which are history.
7. **`archive/README.md` is new and left untracked on purpose.** The doc-count pin reads
   `git ls-files "*.md"`, so staging it now would fail docs-truth against `docs/README.md`'s count,
   which the rules leave alone until P36. It declares a tier, but the tier and map pins read only
   `docs/*.md`.
8. **For P34:**
   - Three files outside the deleted set still name scripts P34 deletes:
     `archive/hand_landmark_reliability.py:293`, a refusal message saying "run
     scripts/analyze_swing.py first"; `archive/golfdb/extract_pose.py:16`, naming
     `scripts/run_pose.py`'s envelope; and `spikes/2026-08-07-two-phone/probe.py:137,259`. P34's
     doc-path rewrite should reach them, or say why it leaves them.
   - `README.md`'s tree still has `frontend/`, and `pyproject.toml` still has `hardware`, in `all`
     too.
9. **For P35:**
   - **`CLAUDE.md` was not touched**, because the plan gives it to P35. Two of its sentences are now
     false. Its Invariants say "Fitting lives in `scripts/` under the `research` extra", and its
     extras list still names `research`. The fitting is `archive/golfdb/`, with the libraries
     installed by hand.
   - **ADR-022's addendum** should say the paths were rewritten in P33, which retires its M31.5
     addendum's "left as they are". §1's table still reads "the `research` extra — numpy,
     scikit-learn" beside the new path.
   - ARCHITECTURE §1's tooling paragraph got a one-clause edit, and the full rewrite is P35's.
     `docs/README.md` might route to `archive/README.md`.
10. **For P36:**
    - Stage `archive/README.md`, and `docs/README.md`'s document count rises by one.
    - The renames are staged already.
    - P33's other files are: `pyproject.toml`; the 22 Markdown files in item 5; the comment edits in
      `src/golf_coach/{analysis/benchmarks/joint,analysis/benchmarks/trajectory,analysis/measure,analysis/phases,analysis/stats,analysis/trajectory,api/pipeline,contracts/baseline,contracts/dispersion,pose/estimator,storage/keypoints_io}.py`
      and in `tests/analysis/{test_joint,test_phases,test_trajectory}.py`, `tests/test_conformance.py`
      and `scripts/conformance{,_vectors}.py`; `crates/{contracts/src/dispersion,pose/src/pool,trigger/src/stream}.rs`;
      and `spikes/{2026-08-07-two-phone,mediapipe-hands}/probe.py`.
11. **Green.**
    - **pytest**, alone: 2,088 passed in 18 minutes, as P32. The two warnings are paddle's
      `ccache` notice and pytest failing to write its own `.pytest_cache`. **ruff** over
      `src tests scripts` is clean, and so is **mypy** over 117 files, one fewer than P32's
      because `contracts/reference.py` left `src/`. **docs-truth**: 91 passed.
    - **`cargo test`**, the whole workspace: 1,644 passed, 0 failed, 24 ignored, as P32.
      **clippy** (`--all-targets`) and **fmt** are clean.
    - **Data.** The `data/processed` digest is unchanged: `07f37373…` over 192 files.
    - **Diffs.** Nothing under `spec/` changed after the marker. The `src/`, `scripts/` and
      `tests/` files changed after it are exactly item 10's, and those are comments and
      docstrings, plus the move itself.
    - **Scratch.** The two rewrite scripts, the import check and the logs are deleted.

### P34 (2026-10-29) — cut off once with nothing written, resumed and done: the lab scripts, `detection/` and `frontend/` gone, four career pins with them, and `golf-coach` Connected through `golf-mcp`

1. **Resumed from nothing.** The marker read 2026-10-28 22:02, and `find … -newermt` listed only
   this plan, so the cut-off attempt had written nothing and the phase ran from the start.
2. **What was deleted.**
   - **Staged by `git rm`**: the fourteen scripts the Do line names, and
     `src/golf_coach/detection/` (two files). Their `scripts/__pycache__` bytecode went too.
   - **`frontend/README.md` is deleted from the working tree only, unstaged on purpose.** The
     doc-count pin reads `git ls-files "*.md"`, which still lists a file the index holds, so
     docs-truth stays green against `docs/README.md`'s count, which the rules leave to P36. This is
     P33's `archive/README.md` trick run the other way.
   - Nothing imported `golf_coach.detection`, `bleak` or `ultralytics`.
3. **Five career pins reached the deleted scripts: four went with them, and one was re-pointed.**
   Each reached `conformance_vectors._career_scripts`, which loads the five career scripts by path
   to print what each printed. With the scripts gone, the four with "career" in their names
   failed on `FileNotFoundError` (measured: 4 failed, 34 passed over
   `pytest tests/test_conformance.py -k "career or mcp or selection or storage"`). The fifth fell
   outside that `-k`.
   - **The four, deleted**: `test_the_career_cases_build_in_memory_and_write_nothing`,
     `test_the_career_cases_cross_every_floor_and_reach_every_reading`,
     `test_frozen_python_answers_a_career_case_only_under_its_own_engine` and
     `test_the_real_career_case_starts_from_the_real_storage_corpus`. Their helpers went with
     them: `_career_built`, `_CAREER_REPORTS` and `_GOLFERS`. This is P8 item 10's and P18 item
     11's rule applied to a third family: the pins go with the files they load, and the committed
     family stays. A comment where they stood says so.
   - **The fifth, re-pointed**: `test_every_report_branch_has_recorded_text`, the only pin that
     every renderer branch (`_CAREER_BRANCHES`) is reached by some recorded case. It read the
     in-memory build. It now reads the committed `career/synthetic/` vectors, because its question
     is about the family and not about the recorder, and it passes. This is P33's "re-point or move
     every test that loads them", as against deleting a gate no Rust test replaces. It was found
     late: the first full run counted 2,083, one short of P33's 2,088 less four, and
     `git diff` named it.
   - The pins that read only what is committed (`check` defers it, `list`, `--career-once`
     refuses) are kept and pass.
   - **`conformance_vectors.py` is untouched.** `build_career`, `_run_career` and `_career_real`
     stay as the record of how the family was made, and are no longer runnable. Nothing reaches
     them: `--career-once` refuses a non-empty family before it imports the recorder.
   - **No other test loaded a deleted file.** Before the re-point, collection found the same 2,083
     tests with the deleted files restored from `HEAD` and without them (measured both ways, then
     deleted again). Finding 13's
     list was re-checked, and the deleted names appear in tests only in docstrings, comments and
     frozen Python's own sentences (item 6).
4. **The extras.** `vision` loses `ultralytics`, and `hardware` (`bleak`) is gone, from `all` too.
   A comment where each stood says why. `.venv` was not touched.
5. **Rust sentences that named a deleted script now name the verb that replaced it.** Each is one
   an earlier finding asked for.
   - **`READING_A_BAG`'s mishit bullet** (P9 item 10). `Wording` now governs two bullets, through
     `caveats::{mishit_exclusion_bag_bullet, reading_a_bag_worded}`.
     - **Not the straight swap P9 proposed.** At width 95, "one `golf-core flag-mishit --clear` on
       a borderline shot" puts the line break inside `flag-mishit`, because `fill` breaks a
       hyphenated word as `textwrap` does. A model would then read `flag- mishit`.
     - **So Rust's clause reads** "so clearing a borderline shot with
       `golf-core flag-mishit --clear` puts it back in the average". A unit test holds the
       command whole on one line.
     - **Declared as the surface's fifth difference** in `crates/mcp/tests/surface.rs`. The
       recorded registry-shape briefing must hold the frozen bullet once, and the registry-less
       one not at all. The caveats resource serves Rust's wording, so the LLM side reads it there.
   - **`golf_mcp::flight::UNREAD_SHOT_SCREEN`** (P14 item 8) ends "…imported with golf-lab
     import-photos."
     - It is `DECLARED`'s ninth row: `simulate_flight` at `detail`, rule
       `import_through_golf_lab`. The frozen and Rust tails are constants, P32's shape.
     - It moves three recorded calls: `tools/flight.json` calls 9 and 10, and `tools/sessions.json`
       call 16.
   - **`golf_core::session`'s `NoParse` sentence** (P24 item 11) now reads "golf-lab reads it with
     golf-lab import-photos {photo} --session {S} --out {shots}". `verbs.rs` follows. The docs of
     `golf_core.rs` and `session.rs` no longer say there is no Rust OCR: `golf-core` holds none,
     and `crates/ocr` is reached only by `golf-ocr` and `golf-lab`.
   - **`golf_lab::pipeline::loft_remedy`'s `NoClubTag`** (P32 item 12) now reads "tag one with
     golf-core set-club and re-analyze with golf-lab analyze". P26 had held it to the script,
     which is gone. No vector holds the sentence, and no `analysis.json` in `data/` does. The other
     two remedies name the bag page, whose routes stay open (D11).
6. **What still names a deleted script, on purpose.**
   - **Rust text that the held families record.**
     - `storage::corpus` has three sentences: `backfill_golfer.py`, `analyze_bundle.py` and
       `reanalyze.py`.
     - `core::reports::{baseline, corpus, dispersion, club_profile}` name
       `python scripts/career_corpus.py` and `python scripts/reanalyze.py`, plus the route
       sentences P32 item 12 named.
     - **This answers P32 item 12's question by the plan's own rules.** Changing any of them
       moves the storage or career family, which needs a career declaration, and the rules
       forbid one here. They wait for M40 or the next `CAREER_VERSION` bump.
   - **Rust module docs naming their Python twin.** These are the provenance the rules ask for:
     `golf_lab::*`, `golf_core::reports::*`, `golf_mcp`'s `lib.rs`, `source.rs` and `server.rs`,
     and `analysis::flight*`.
   - **Frozen Python** (clause 4), deleted at M40:
     - its runtime sentences: `mcp/{club,flight}.py`, `api/app.py`, `storage/corpus.py`,
       `contracts/swing.py` and `api/static/career.html`;
     - its comments and docstrings;
     - its tests' docstrings and asserts. `tests/api/test_flight_route.py:470` asserts frozen
       Python's flight detail names `import_shot_screens`, and `test_pipeline_imports.py`'s
       messages name `analyze_bundle.py`.
   - **Recorded data**: the vectors and the schemas.
   - **History**:
     - ROADMAP's milestone sections; §M29's own is P36's;
     - ADR bodies, the REFERENCE-tier milestone docs and `docs/archive/`;
     - `spikes/2026-08-07-two-phone/probe.py` (P33 item 8), a dated spike's record of what it ran
       against;
     - `WORKLOG.md` and the plans.
7. **Doc paths rewritten.**
   - **`README.md`**, as the Do line says:
     - the CLI block is the Rust verbs;
     - the MCP section registers `golf-mcp` and says why a running client blocks a release
       rebuild (item 8);
     - the stack table, the package list, the extras line and the tree lose `detection/`,
       `frontend/`, `hardware` and `research`, and the tree gains `crates/` and `Cargo.toml`;
     - the two `analyze_bundle.py` sentences now name `golf-lab analyze`.
   - **ARCHITECTURE §1's command block** now names `golf-pose run`, `golf-lab analyze-keypoints`,
     `import-photos` (both forms), `analyze`, `reanalyze` and `flight`, with the flags from
     `bin/golf_lab.rs`' doc. The Python career lines are merged into the Rust career block, keeping
     their comments. `run-shot`'s note no longer names the importer, and five instruction sentences
     elsewhere (§1's MCP clause, two in §3 and two in §4) now name verbs.
   - **The rest**:
     - `FLOW.md` (two sentences), `CODE_STANDARDS.md` (the pin's reason, now in the past) and
       `.env.example`;
     - `.gitignore`'s `frontend/` lines;
     - `scripts/ask_swing.py`'s docstring and its "no analyzed swing" sentence, which names
       `golf-lab analyze`, with `tests/feedback/test_conversation.py`'s assert. This is the LLM side
       the rules allow;
     - the two archived files P33 item 8 named. `hand_landmark_reliability.py`'s refusal now names
       `golf-lab analyze`, which is what writes the `analysis.json` it reads.
       `extract_pose.py`'s envelope is `golf-pose run`'s.
8. **The MCP server, re-registered (D21).**
   - **Before.** `claude mcp get golf-coach` showed the local-scope Python registration as "✘
     Failed to connect". That was this phase's own doing: `run_mcp_server.py` was already deleted.
   - **The swap.** `claude mcp remove golf-coach -s local`, then
     `claude mcp add golf-coach -s local -- C:/Users/aaron/Desktop/repo/golf-coach/target/release/golf-mcp.exe`,
     with no arguments. The directories default under the repository.
   - **After.** `claude mcp list` reports `golf-coach: …/golf-mcp.exe - ✔ Connected`. A session
     that was already running keeps the server it connected to until it restarts.
   - **For P36, and anyone rebuilding: Windows will not replace a running executable.** Measured:
     `cp` over a running copy of `golf-mcp.exe` fails with "Device or resource busy". A Claude Code
     session started after this registration holds `target/release/golf-mcp.exe`, so
     `cargo build --release -p golf-mcp` fails until it ends. Building `golf-core` and running the
     dry runs only spawn the binary, and neither rewrites it.
9. **For P35**:
   - **ARCHITECTURE still describes frozen Python's lab** in the places §1–§4's rewrite owns:
     - §1's diagram (the `run_pose.py`, `analyze_swing.py` and `analyze_bundle.py` nodes, the
       `align_swings` render) and its "Reading it" paragraph;
     - the `detection/` sentence at the end of §1, and §2's `DET` node, its `scripts/*.py` CLI node
       and its two `detection/` sentences;
     - §2's worker paragraph (`scripts/run_pose.py`) and its pipeline sentence
       (`analyze_bundle.py` a presentation layer);
     - §3's sequence actor `scripts/analyze_swing.py`;
     - §4's table rows for keypoints, parsed shots and mishits, and its filename-stem paragraph;
     - §4's "three surfaces read that one builder", which still counts
       `scripts/club_profile.py`, and its `reanalyze.py` sentence about `is_outdated`.
   - **CONFORMANCE's MCP section**: the surface now has five declared differences, and the tool
     family has nine `DECLARED` rows. Its "`build_career` stays as the record" now means a record
     that no longer runs (item 3).
   - **`CLAUDE.md`'s invariant** says `api/pipeline.py` and `coach.py` stay import-clean "because
     the offline CLIs run on installs without those extras". Those CLIs are Rust now. The pins
     stand, and the reason wants rewording.
   - **ADR-006's addendum**: the server is `golf-mcp`, registered as a binary, and the Python
     server's script is gone.
   - **`README.md` was not rewritten past the deleted paths.** Its upload paragraphs ("From a
     phone, it needs no command at all", "Uploading swings from a phone") describe the route P32
     closed, and its Decision Log line counts 16 ADRs.
   - **`CODE_STANDARDS.md:73`** still lists `ultralytics` among the extras' packages.
10. **For P36**:
    - **Stage `frontend/README.md`'s deletion** (`git rm --cached`). `git ls-files "*.md"` then
      drops one, against `archive/README.md`'s one (P33 item 10). The map's sentence "one each in
      `data/`, `frontend/` and `spec/`" loses `frontend/` and gains `archive/`.
    - **P34's staged deletions**: the fourteen scripts and `detection/`.
    - **P34's changed files**:
      - `.env.example`, `.gitignore`, `README.md`, `pyproject.toml`;
      - `docs/{ARCHITECTURE,CODE_STANDARDS,FLOW}.md`;
      - `archive/{hand_landmark_reliability.py, golfdb/extract_pose.py}`;
      - `scripts/ask_swing.py`, `tests/feedback/test_conversation.py`, `tests/test_conformance.py`;
      - `crates/core/src/{bin/golf_core.rs, rerecord.rs, session.rs}`, `crates/core/tests/verbs.rs`;
      - `crates/lab/src/{flight.rs, pipeline.rs}`;
      - `crates/mcp/src/{caveats.rs, family.rs, flight.rs}`, `crates/mcp/tests/surface.rs`.
11. **Green.**
    - **pytest**, alone: 2,084 passed in 5 minutes. That is P33's 2,088 less item 3's four, and
      the one warning is paddle's `ccache` notice (`-p no:cacheprovider` kept pytest from writing
      its cache). **docs-truth**: 91 passed, after the last README edit.
    - **ruff** over `src tests scripts` is clean, and so is **`ruff check archive`**. **mypy** is
      clean over 115 files, two fewer than P33's 117: `detection/`.
    - **`cargo test --no-fail-fast`**, the whole workspace (`CARGO_INCREMENTAL=0`): 1,645 passed,
      0 failed, 24 ignored. That is P33's 1,644 plus the bag-bullet unit test. **clippy**
      (`--all-targets`) and **fmt --check** are clean.
    - **The career dry run** (`career-v3`, which walks `mcp` through `DECLARED`) found 0 changed
      over 175 vectors, 28 of them `mcp`. The release `golf-mcp` answered, rebuilt with this
      phase's rows.
    - **Data.** The `data/processed` digest is unchanged: `07f37373…` over 192 files.
    - **Diffs.** Nothing under `spec/` or `spec/declarations/` changed after the marker. The
      `src/`, `scripts/` and `tests/` changes are the deletions, `ask_swing.py` and its test (the
      LLM side), and `tests/test_conformance.py`'s cut and re-point (item 3). Nothing under `src/`
      was edited.
    - **Scratch.** The reference listings, the logs and the lock probe are deleted.

### P35 (2026-10-29) — the docs say what M29 built: two families and the OCR gate, the Rust lab, nine addenda

1. **Started clean.** The marker was set before any other work and nothing was resumed. Every
   finding addressed to P35 was read by its item (the `\bP35\b` grep), and each is either done below
   or named in item 6 as left.
2. **What landed, by document.**
   - **`docs/CONFORMANCE.md`**:
     - The head gains "What M29 added".
     - §2: twelve families. The ledger paragraph covers `selection` (one `v17` entry each) and
       `mcp` (`career-v3` on the recorded cases, `hand` and `rust` with none), and so do the
       declarations and "What reads the ledger" paragraphs.
     - Three new sections before §3. **Selection** (45 cases: the sub-families, the cut and its
       proof, the gate by value, the walk and the old carry). **MCP** (29 cases, 224 calls: five
       sub-families and their oracles, the comparison, the five surface differences, the nine
       `DECLARED` rows by kind, the three oracles, the strike profile held to the verb, both walks).
       **The OCR gate** (`crates/ocr/tests/read.rs`: what is compared, the one declared value, and
       why it is ignored).
     - The storage section names a Rust-recorded case's `worked_against`.
     - §4: the head paragraph; the command block gains the OCR gate, the `golf-mcp` build,
       `answer-case`, `build-vectors`, `--mcp-once` and `--selection-once`; the walk and gate
       rules gain selection and mcp; M29's two writing runs; `build-vectors` in "`rerecord`
       cannot create a vector", with P30's finding 7 caution and finding 5's `shot_unscored`;
       `check`'s two deferrals; the `regenerate` heading, and `build_career` named as a record that
       no longer runs.
     - §5: tier rows for storage, `audio/ffmpeg.py` (ported, M29), the shot source, the reader, tier
       4 and `detection/`. The crate count is thirteen. "What this does not cover" is updated for
       the decoder, the recognizer and the writing verbs.
   - **`docs/ARCHITECTURE.md`**:
     - The review date.
     - §1: a new diagram of the Rust lab. It keeps a `CK["6 checkpoints…` node, because
       `test_architecture_states_the_real_checkpoint_count` reads it; the first draft dropped it and
       the pin failed. "Reading it" is rewritten, with the closed upload route and the worker's
       recovery.
     - §1's command block gains `golf-ocr read`, `import-export`, the three `golf-core` repairs,
       `golf-mcp`, `build-vectors` and the OCR gate. The Rust-half paragraph is thirteen crates,
       with M29's three and their modules. The service paragraph and the tool table's home are
       updated. The routes table marks the eight closed routes, each with the verb its 410 names
       (`api/app.py`).
     - §2: the graph is labelled as the Python one; `DET` is gone; the CLI node is what is left in
       `scripts/`. New paragraphs cover the lab's decode, the M29 crates and their cargo edges, and
       the worker's first caller. The pipeline paragraph is rewritten.
     - §3: the actor.
     - §4: keypoints, overlays, parsed shots, bundles, mishit, `shot_analysis.json`, the
       artifacts, a new **OCR models** row, the filename-stem paragraph, the three surfaces, and
       `reanalyze`'s fourth question.
     - §5: one bullet on how the lab was gated.
   - **`CLAUDE.md`**:
     - The first paragraph: the lab is Rust, and `.venv` is 3.13 (P1 item 15).
     - Commands: `rerecord` walks selection and mcp and needs `golf-mcp` built; `golf-lab`,
       `golf-ocr`, `golf-mcp` and `set-club` are added.
     - Thirteen crates, with each paragraph's "wired to nothing" corrected, and an M29 paragraph.
     - Entry points and extras: `research` is gone.
     - Three invariants: `ort`, the frozen lab, and the import pins' reason (P34 item 9). Fitting
       now lives in `archive/golfdb/`.
     - The routing table: the app row, the conformance row (two families, the OCR gate and
       `build-vectors`), and the Rust map: `ocr`, `mcp` and `lab`, plus M29's additions to `pose`,
       `analysis`, `core` and `storage`. The 130 `flight_measure.py` lines are now named as ported.
   - **ADR addenda, nine.**
     - **035 (its first)**: the four questions answered (D7 as built, D10, D11 with P32's items 12
       and 13, D12); `ort` rc.10/ONNX Runtime 1.22; the conversion toolchain; the `.gitignore` line;
       `image`, the JPEG hand port and `imageproc` never entering; `rmcp` 0.11.0 with `tokio`;
       `zip` 7.3.0; ffmpeg 9.0.2 and the user's (a) at P21; the recorder running a shell's binary;
       the two one-time recordings and `build-vectors`; P29's write; the fourth staleness form;
       clause 4 holding.
     - Its Status gains one sentence. Its Deferred list's three M29 items each gain an italic
       "Answered by the M29 addendum" pointer.
     - **006**: `golf-mcp`, the re-registration, the declared kinds, the protocol revision, and the
       caveats served.
     - **014**: the reader, and the gate on the shot.
     - **016**: closed, with the export import.
     - **020**: P16 item 2's event loop and P17 item 5's seed.
     - **022**: the move done, and "left as they are" and the `research` extra retired.
     - **033**: P22's five departures and D12. Its self-count "Six addenda" is now "Seven", with a
       sentence on the seventh.
     - **024** and **028**, beyond the Do line, because P32 item 14 and P12–P14 asked for them:
       024 covers §5's repair path as `set-club`, the drill sentences, and `COMPARABLE_FROM` in
       `simulate_flight`; 028 covers the override as one verb.
     - ADR-019's P17 addendum and its map row were checked and not redone.
   - **`docs/README.md`**: "109 addenda" is 118. Nine rows carry their new count and a summary,
       035's status cell says M29 built clauses 2 and 5, and 028's "no addendum" clause is
       corrected. The Archive section routes to the repo root's `archive/README.md` (P33 item 9).
       **The document count was not touched** (the plan's rule; P36's).
   - **The program plan**: §M29 gains "Built 2026-10-29, and what changed against this section".
     §M38 P4 carries P31 item 6's requirements of the phone's export (P31 item 6's "For P35").
   - **`data/README.md`**: `models/ocr/` in the tree and a section of its own. A paragraph says
     the Rust lab writes `processed/` and names P29's backup. "Neither exists on disk yet" now
     reads "on the stored bundles".
   - **Also, from P34 item 9's and P14 item 9's lists**:
     - `README.md`: the closed upload route in the intro and its section. The `ADRs 000–016` and
       "16 of them" counts are gone rather than re-counted.
     - `docs/CODE_STANDARDS.md`: `ultralytics` out of the extras example.
     - The M31.5 inventory's R5 row and CONFORMANCE's tier-1 sentence: the `flight_measure.py`
       half was ported at P14.
3. **One frozen-tooling edit, sanctioned by the plan's first rule** ("check's deferral of them").
   P19 item 7 found that `conformance.py check`'s mcp line and `mcp_vector_paths`' docstring said
   the family ages on `CAREER_VERSION` alone. Since P19 an engine declaration walks it too.
   - The line now reads "…and are re-recorded under CAREER_VERSION and ANALYSIS_VERSION alike".
   - The docstring says why: neither key is frozen Python's.
   - The pin (`test_conformance.py:2142`) reads the prefix only, so it passes unchanged.
   - Nothing else under `src/`, `scripts/` or `tests/` changed. Nothing under `crates/` or `spec/`
     changed either.
4. **Line endings, for P36.** `core.autocrlf` is `true`. So the working-tree docs are CRLF while
   `HEAD` holds LF, and some files earlier phases wrote are LF (this plan, ADR-035, the program
   plan, `scripts/conformance.py`). Every edit here kept each file's own endings: Python with
   `newline=''`, and the Edit tool.
   - **`sed -i` from Git Bash rewrote `docs/ARCHITECTURE.md` whole to LF.** It was converted back
     to CRLF.
   - No file is mixed (checked over every doc touched and every ADR).
   - Git normalises either way on commit.
5. **Routed, not restated.** Counts that move are written as snapshots with their phase:
   - selection 45, mcp 29 and 224 calls, measured this phase;
   - the 118 addenda, which the map's pin owns.

   The tool count appears only where the Rust pin reads it ("the same N tools", untouched). Module
   lists point at each crate's `src/lib.rs` as the map.
6. **Left as found, and why.** None is a docs fix this plan allows.
   - **`crates/trigger/src/clip.rs` copies `_WINDOW_LEAD`, `_WINDOW_TRAIL` and the 0.80 bound by
     value** (P19 item 7). Twins exist in `analysis::phases`, and `trigger` has no `analysis` edge.
     It is a drift risk for M40 to decide. **For P36**: worth a line in the WORKLOG entry.
   - **`contracts::career`'s `CAREER_VERSION` doc** still says no recorded artifact's `unscored`
     holds a flight row. P30's finding 5 qualifies it: true of the recorded trees, false of
     `data/`. CONFORMANCE §4 now says so. The code doc is the user's to reword.
   - **`golf-core club-profile`'s text and the lab's `loft_remedy`** still name the closed routes
     (P32 item 12). The first is recorded in `spec/vectors/career/`, which this plan holds
     unchanged, and the second is held to the script's text. Both are M40's, or a career
     declaration's.
   - **`SwingBundleStore::next_swing_id`** (P24 item 12) is named in its crate doc, and not in
     `CLAUDE.md`'s map.
   - The worker's startup recovery (P32 item 12) is recorded in ADR-035's addendum and ARCHITECTURE
     §1 as the user's call, not changed.
7. **For P36**:
   - ROADMAP's §M29 can route to ADR-035's M29 addendum and the program plan's new "Built" block
     rather than restate them.
   - The WORKLOG entry can name the nine addenda and the map's 118.
   - `docs/README.md`'s **document count** is still P36's to move, when this plan is staged.
   - The memory file (`app-transition-decisions.md`) should say M29 is done.
8. **Green.**
   - **docs-truth**: 91 passed.
   - **pytest**, alone (`-p no:cacheprovider`): 2,084 passed in 5 minutes, P34's count.
   - **ruff** over `src tests scripts` is clean.
   - **`cargo test -p golf-mcp --test docs`** (the tool-count pin, which reads every ADR): 2
     passed.
   - `conformance.py check` prints the reworded mcp line and exits as before.
   - **Data.** The `data/processed` digest is `07f37373…` over 192 files, unchanged.
   - **Diffs.** `git diff` over the ten frozen vector families is empty.
   - **Scratch.** The replacement specs and the helper scripts were in the session scratchpad,
     never the repo, and are deleted.

### P36 (2026-10-29) — the close: every gate green, the docs moved, one commit

1. **Started clean.** The marker was set before any other work, and nothing was resumed. Every
   finding addressed to P36 was read by its item (the `\bP36\b` grep): P0, P1 items 16, P3 item 9,
   P4–P6 items 10, P15 item 11, P17's map note, P19 item 8, P25 item 11, P29 items 17 and 21, P30
   item 8, P31 item 9, P32 item 15, P33 item 10, P34 items 8 and 10, and P35 items 4, 6 and 7. Each is
   done below, or carried into the WORKLOG entry where it is a note for a later milestone.
2. **Verify, every command on the plan's list.**
   - **`cargo test`** (the whole workspace, `CARGO_INCREMENTAL=0`): 1,645 passed, 0 failed,
     24 ignored over 112 suites, P34's count.
   - **`cargo test -p ocr -p golf-lab --no-fail-fast -- --include-ignored`**: 296 passed, 0 failed,
     0 ignored. That is `ocr`'s 127, as P29, and `golf-lab`'s 169, P30's 152 plus P31's 17. Only
     these two crates hold an `#[ignore]`. The run took about 20 minutes in a debug build. The
     slowest were the OCR gate (`read.rs`, 318 s), `recognizer.rs` (244 s), `rec.rs` (200 s) and
     `det.rs` (176 s), so the climb P4 and P5 measured did not show.
   - **`cargo clippy --all-targets`**: 0 warnings. **`cargo fmt --check`**: clean.
   - **The `v17` dry run**: 140 vectors (21 engine, 21 stage, 25 shot, 45 selection, 28 mcp),
     0 changed, 0 files written. That is P19's and P25's expected result.
   - **The `career-v3` dry run**: 175 vectors (76 storage, 56 career, 15 profile, 28 mcp), 0 changed,
     0 files written.
   - **The release `golf-mcp` was fresh.** `cargo build --release -p golf-mcp` finished in 1.4 s
     with nothing compiled, so P34 item 8's running-executable lock never came into play. The
     binary is P34's build, and nothing under `crates/` changed after it.
   - **pytest**, alone (`-p no:cacheprovider`): 2,084 passed in 330 s, P34's and P35's count, with
     docs-truth among them. The one warning is paddle's `ccache` notice.
   - **ruff** over `src tests scripts` is clean. **mypy** is clean over 115 files.
     **`conformance.py check`** exits 0. It prints the 21 engine vectors holding the freeze and
     defers every other family to `cargo test`, the mcp line in P35's wording.
   - **Data.** The `data/processed` digest is `07f37373…` over 192 files, P29's, and no `.tmp` or
     `.part` file exists. `git diff` over the ten held families and `spec/declarations/` is empty.
     The only things under `spec/` that are not in `HEAD` are the `mcp` and `selection` families.
3. **The Exit, item by item.**
   - The OCR gate passes (`read.rs` above, with its one declared value).
   - The lab re-ran every stored bundle after D20's gate (P29), and `inputs.rs` and `reread.rs`
     pass over the written `data/`.
   - The 13 shots are stamped, and only the two label-fix shots change a field value (P29).
   - `ask_swing.py` answers through `golf-mcp` (P16), and the tool-count pin reads Rust's table
     (`crates/mcp/tests/docs.rs`, in `cargo test`).
   - The archive move and the deletions are done (P33–P34), and the closed routes answer 410
     (`tests/api/`, in pytest).
   - ADR-035's addendum answers the four questions (P35).
4. **The docs P36 owns.**
   - **`docs/README.md`'s document count is now 84**, up from 83. `docs/` goes 70 → 71 and `plans/`
     9 → 10, for this file. The thirteen outside `docs/` are unchanged in number: `frontend/` went
     and `archive/` came, and the sentence names `archive/` in its place. The three changes were
     staged **before** pytest ran, because the pin reads `git ls-files`: this file and
     `archive/README.md` by `git add`, and `frontend/README.md`'s deletion by `git rm --cached`
     (P33 item 10, P34 item 10).
   - **`ROADMAP.md`**:
     - "Last Updated" is now 2026-10-29.
     - M29's status row is ✅ 37/37. M38's row no longer waits on M29, and M40's waits on M38
       alone.
     - NEXT ACTION's heading and paragraph are now M41 or M42.
     - §M29's Status routes to the program plan's "Built" block and to ADR-035's addendum
       (P35 item 7), and marks its four open questions answered. Its body stays as planned.
     - §M38's and §M40's Status lines are updated: M38 waits on a Mac alone, and M40 is pointed at
       what M29 left it.
     - The closed milestones' sections that say "until M29 calls it" are history, and are left
       as P34 item 6 says.
   - **`WORKLOG.md`** has a new top entry. It names the nine addenda and the map's 118 (P35 item 7)
     and `crates/trigger`'s copied constants (P35 item 6). It also carries P1 item 16's
     reconversion recipe, P1 item 13's untried knobs, and P34 item 8's executable lock, as notes
     for the next re-record.
   - **The memory files.** `app-transition-decisions.md` says M29 is done and names what it
     built, and its description and `MEMORY.md`'s index line follow. Next is M41 or M42, and M38
     waits on a Mac. `rust-everything-python-only-where-required.md` replaces its stale "Next is
     M32" with what still runs in Python after M29.
5. **Line endings.** Every edit kept each file's own: the Edit tool on the CRLF docs (`ROADMAP.md`,
   `WORKLOG.md`, `docs/README.md`), and this file stays LF. `.gitattributes` keeps `spec/**` as
   `-text`, so the CRLF vectors commit byte for byte.
6. **The commit.** One commit onto `main`, as D1 says. Every path in `git status` is M29's, checked
   against each phase's "files for the one commit" list (P29 item 21, P30 item 8, P31 item 9,
   P32 item 15, P33 item 10, P34 item 10) and the rest of the tree's diff. None of it is under
   `data/`, which is gitignored except `data/README.md`. No model file is staged, because
   `data/models/ocr/` is ignored by `.gitignore:22`. The untracked additions come to about 5.8 MB.
   The largest are this file and the `mcp` family.
7. **Scratch.** The test logs and the commit message's draft were in the session scratchpad and are
   deleted.
