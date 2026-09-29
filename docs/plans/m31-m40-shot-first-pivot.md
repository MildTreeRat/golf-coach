# Plan: M31–M40 — shot-first, phone-first

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Findings are appended as milestones close, and a finding contradicting
the plan wins.

**Planned**: 2026-09-29. **Milestones**: M31–M40, which do not exist in `ROADMAP.md` yet — writing
them there is M31's job. **Governing decisions**: ADR-034 (to be written by M31), amending
[ADR-009](../decisions/009-swing-scoring-model.md), [ADR-014](../decisions/014-screen-capture-shot-ingestion.md),
[ADR-016](../decisions/016-local-first-host-and-phone-upload-topology.md), [ADR-024](../decisions/024-per-club-shot-history.md),
[ADR-030](../decisions/030-app-platform-rust-core-python-sidecar.md) and
[ADR-032](../decisions/032-the-rust-core.md).

This is a program plan, not one milestone's phase list: it covers ten milestones, and details only
the first one that writes code (M32). Each later milestone gets its own `docs/plans/mNN-*.md` when it
starts, as M23 did.

---

## Status checklist

| Milestone | What | Where it runs | Depends on | State |
|---|---|---|---|---|
| **M31** | The pivot decided (ADR-034, docs only) | desk | – | ⬜ Not started |
| **M32** | Wider shot contract and device capability | this box | M31 | ⬜ Not started |
| **M33** | Apple Vision spike: can the phone read the screen? | Mac | M32 | ⬜ Not started |
| **M34** | The screen reader in Rust (`crates/screen`) | this box | M32 | ⬜ Not started |
| **M35** | Shot-first sessions in Python, and `analyze_shot` | this box | M32 | ⬜ Not started |
| **M36** | The many-shot layer ported to Rust | this box | M35 | ⬜ Not started |
| **M37** | Strike profile and strengths/weaknesses (Rust first) | desk, then bay | M36 | ⬜ Not started |
| **M38** | The iPhone app | Mac, then bay | M34 (skeleton), M37 (profile screens) | ⬜ Not started |
| **M39** | Optional video on the phone | Mac, then bay | M38 | ⬜ Not started |
| **M40** | The laptop client resumes (M21/M24/M25 re-scoped, R10/API adapters) | later | M38 | ⬜ Not started |

**M33 runs early on purpose.** Whether Apple Vision reads the HD Golf screen well enough is the
product's biggest unknown. It can be tested before any Rust port, by feeding Vision's boxes to
today's Python parser. If it fails, this plan changes before any app work starts.

The iPhone app skeleton (M38 P2: photo → read → store → history) can start once M34 lands, in
parallel with M35–M37.

---

## Why

Today the product is pose analysis of face-on video. The shot photo is "attached and displayed
but never scored" (`docs/ARCHITECTURE.md` §1): `analysis/engine.py` sets `outcome = []`, and
`feedback/rules.py` reads no shot field.

The pivot turns that around:
- The golfer photographs the launch-monitor screen, many times.
- Strengths and weaknesses per club come from those numbers: strike quality (face impact, and low
  point where the device prints it) and distance/direction consistency.
- Video is optional. It is a visual aid ("on this shot your low point was here (red); it should be
  here (green)"), and the pose checkpoints are computed only when video exists.
- The first host is a standalone iPhone app. The laptop stays as a later client.

## Decisions (the user's, 2026-09-29)

1. **HD Golf now, others later.** A second device should be a `profiles.json` entry plus a
   `devices.json` entry, with no code change.
2. **A stat the launch monitor doesn't print is not tracked.** Nothing is inferred, whether from
   other numbers or from video. This gives a *device capability model*:
   - A field that is **not tracked** produces no measurement, no `unscored` entry, no caveat and
     no tip.
   - A field that is **tracked but blank on one shot** goes through the existing `unscored`
     refusal (ADR-010 §2).
3. **The pose checkpoints stay, but are optional.** They are computed only when video was
   supplied, and a session with no video is first-class.
4. **Keep the laptop, phone first.** The laptop comes back in M40 for:
   - a launch monitor wired in directly (R10 BLE, ADR-004);
   - a launch-monitor API;
   - camera orchestration (M21/M24).
5. **Standalone on the iPhone.** The Rust core runs on-device through Flutter and
   `flutter_rust_bridge`. The user has a Mac, so iOS builds happen there and everything else on
   the Windows box.

## What the HD Golf data can and can't support

- **It prints no attack angle, dynamic loft or low point** (ADR-027 says so too), so low point is
  never tracked for HD Golf.
- **Face impact is text only.**
  - Horizontal: `impact_position` is one of {CENTER, HEEL, TOE} on every stored shot.
  - Vertical: the bay's screen shows an `Impact Position V` tile the profile doesn't read (ROADMAP
    §M3, ADR-014 addendum). It read `---` on the bay photos checked.
  - The screen layout is configurable (there is a `Custom` gear tile), so a bay trip should list
    which tiles can be enabled.
- **Spin is blank on 11 of the 13 stored shots.** Smash reads 0.89–1.00, which is physically
  implausible, so club speed and smash become `shown_only`. `shot_measure.py`'s docstring already
  excludes them for this reason.
- **So for HD Golf, "strike quality" is:**
  - the face-impact grid per club, and the centered-strike rate;
  - ball-speed consistency;
  - carry, start-line and face-to-path bias against scatter.

## What the code says, before anyone re-derives it

These are findings from the planning session's read of the code (2026-09-29). Verify each one
before leaning on it.

1. **The capability model needs a per-golfer intersection, because layouts vary per user.** Bay
   photos have `Custom` and `Impact Position V` where the two reference photos in
   `data/raw/shot_screens/` have `Bounce & Roll`. So:
   - each device declares the fields it can print;
   - tracked = declared ∩ the tiles actually seen on that user's photos.
2. **The corpus already keys launch-monitor samples on the photo:** `CorpusSwing.artifact_key`
   returns `shot:{sha}`. What is swing-bound:
   - Admission: `read_corpus` excludes a manifest with no face-on clip
     (`ExclusionReason.NO_FACE_ON`, `storage/corpus.py:97`).
   - Measurements: they come only from `analysis.json`.
3. **Adding fields to `ShotData` forces a vector re-record even though no number moves:**
   - Corpus vectors take `input.shot` from the stored `analysis.json`.
   - `compare_results` requires identical key sets.
   - `crates/contracts`' `round_trip.rs` distinguishes an absent key from a null one.
4. **`mcp/query.py::_METRIC_FIELDS` / `_NON_METRIC_FIELDS`** are checked against `ShotData`
   exhaustively by `tests/mcp/test_query.py`.
5. **The screen parser has Python-specific behaviour that a Rust port must reproduce exactly.**
   None of it is on ADR-032 §3's list of edges:
   - `difflib.SequenceMatcher.ratio` (`profiles.py:76`);
   - the look-behind regex `_THOUSANDS` (`parser.py`), where the Rust `regex` crate has no
     look-around;
   - float floor division `center_y // bucket`;
   - `{text!r}` inside warnings, which are compared exactly;
   - tie rules: `field_for` uses `>=` (the last field wins), `_find_labels` uses `>` (the first
     box wins);
   - Unicode behaviour of `str.upper()` and `split()`.
6. **`Impact Position` and `Impact Position V` score 0.9375 against each other**, against a
   threshold of 0.8.
   - Exact text resolves correctly.
   - An OCR misread such as `Impact Position Y` could land on the wrong field. It fails as
     "missing", never as a wrong value, but it needs a test.
7. **What pose runs today:**
   - MediaPipe Tasks `PoseLandmarker`, `RunningMode.VIDEO` (`pose/estimator.py`).
   - The model comes from a `float16/latest` URL (`estimator.py:45`), so **the model bytes are
     unpinned**.
   - iOS has the same Tasks API and `.task` format, so conforming is plausible, not guaranteed.
     Decoder, delegate, version skew and the unpinned model all differ.
8. **Two places say "OCR stays Python":** `docs/CONFORMANCE.md` §5, and the `profiles.json`
   comment in `tests/test_conformance.py::_PACKAGE_DATA`. Both change once the parser ports.
9. **`bundle_store` attaches an upload to "the newest swing lacking this role."**
   - Photo-only sessions are fine: each photo opens a new entry.
   - Mixed sessions misattach. The phone's storage API should take explicit targets.

## Reuse, don't rebuild

- **Photo → shot:** `launch_monitor/screen/`
  - The chain: `import_screen` → `prepare_screen` → `TextRecognizer` (PaddleOCR) →
    `parse_screen` → `validate_parse` → `to_shot_data` → `ShotStore.put`.
  - Bulk CLI: `scripts/import_shot_screens.py`.
- **Per-shot numbers:** `analysis/shot_measure.py`, `flight*.py` and `spin_solve.py`, already
  ported in `crates/analysis`.
- **The many-shot layer (Python only):**
  - `analysis/baseline.py::build_baseline`
  - `analysis/dispersion.py::build_dispersion` (with `METRIC_TARGETS` and `PATTERN_READING`)
  - `analysis/club_profile.py::build_bag_profile`
  - `contracts/mishit.py::mishit_carry_floor`
  - `storage/corpus.py::read_corpus`
- **Club specs:** `clubs/` (ADR-026). **Per-club tagging:** ADR-024.
- **Video:**
  - impact frame: `phases.py` + `crates/trigger` + `alignment.with_measured_impact`;
  - drawing: `pose/overlay.py::annotate_frame`;
  - pose: `crates/pose` + `pose/worker.py` on the laptop.

---

## M31 — ADR-034, "Shot-first: the launch-monitor screen is the product, the phone is the host"

**Files:**
- `docs/decisions/034-shot-first-phone-first.md`
- Addenda to ADR-009, 014, 016, 024, 030 and 032.
- `docs/README.md`: the ADR row, plus the addendum counts. Both are pinned by
  `tests/test_docs_truth.py`.
- `ROADMAP.md`: status-table rows M31–M40 and the section for each.
- `CLAUDE.md`: the product sentence at the top, and the "What is the app written in" row.
- `docs/CONFORMANCE.md` §5.

**Roadmap changes:**
- M21, M24 and M25 are paused and re-scoped under M40.
- M28 is superseded by M39.
- M29 is re-scoped.
- M3's open OCR items fold into M32 and M33.
- "M4 full" is superseded by M37.

**The ADR records:**
1. **The unit of the product is the shot.** It also records the capability model:
   - each device declares every field it prints as `analysed` or `shown_only`;
   - tracked = declared ∩ `fields_present`, across the golfer's shots.
2. **ADR-030:**
   - §5 and the "the machine is a laptop" premise are superseded;
   - §1 is amended: storage is on-device too;
   - pose on the phone is reopened, behind a conformance gate (M39 P0).
3. **OCR on the phone:**
   - Apple Vision sits behind the `TextRecognizer` boxes seam, with VisionKit doing rectification;
   - the parser and validator port to Rust;
   - PaddleOCR and OpenCV stay as the lab's reader;
   - a vision LLM stays rejected, on ADR-014's grounds.
4. **The oracle for each vector family:**
   - ports of existing Python are recorded from Python;
   - *new* analysis is Rust first, against hand-worked vectors (`provenance.oracle: "hand"`);
   - the lab reaches Rust-only analysis through `golf-core` subcommands.
5. **Bands:**
   - Shot metrics get no tour bands; they are judged against the personal baseline plus the
     targets already in `METRIC_TARGETS`.
   - A **strength** needs an established equivalence claim: the CI of the mean lies inside
     target ± tolerance, *and* the upper bound of the SD is below tolerance.
   - "Nothing established" is never a strength.
6. **No LLM coaching on the phone,** since it would need the network and an API key.

**Exit:** `pytest tests/test_docs_truth.py` passes.

## M32 — Wider shot contract and device capability (detailed)

This is L3: `contracts/` and `ANALYSIS_VERSION`.

### Python contracts

**`src/golf_coach/contracts/shot.py`**

`ShotData` gains seven fields:

| Field | Units / meaning |
|---|---|
| `attack_angle` | deg, + = up |
| `dynamic_loft` | deg |
| `low_point` | in, + = ahead of the ball |
| `impact_offset_h` | mm, + = toe |
| `impact_offset_v` | mm, + = high |
| `impact_position_v` | text |
| `carry_offline` | yd, + = right |

`ShotProvenance` gains:
- `parser_version: int = 0`;
- `fields_present: list[str] | None`, the tiles located, blank or not;
- `corrections: dict[str, str]`, for M38's review/edit, added now so M38 does not force a second
  re-record.

Also add:
- `SCREEN_PARSER_VERSION = 1`, with a ledger comment like `ANALYSIS_VERSION`'s;
- `parse_is_current(shot)`.

**New `src/golf_coach/contracts/capability.py` + `contracts/devices.json`** (entries `hd_golf`
and `mock`):
- `FieldUse{analysed, shown_only}`; every `shown_only` field carries a note.
- `DeviceCapability`, `capability_for`, `device_of(shot)` and `tracked_fields(shots)`.
- `hd_golf`'s `smash_factor` and `club_head_speed` are `shown_only`, citing `shot_measure.py`'s
  docstring.
- Re-export from `contracts/__init__.py`.

### Screen package

- **`launch_monitor/screen/profiles.json`:** add `Impact Position V` → `impact_position_v`, kind
  text.
- **`launch_monitor/screen/profiles.py`:** `ProfileField.scale: float = 1.0`.
- **`launch_monitor/screen/parser.py`:**
  - `ParsedShot.fields_present`;
  - `_read_cell` applies `scale` after the sign;
  - `to_shot_data` stamps `parser_version` and `fields_present`.
- **`launch_monitor/screen/validate.py`:** `_PLAUSIBLE_RANGES` entries for the new numeric fields.
- **`launch_monitor/screen/importer.py::import_screen`:** a cached shot with a stale
  `parser_version` counts as a cache miss.
- **`api/pipeline.py::_shot_for`:**
  - the same stale-cache check;
  - if the `ocr` extra is missing, serve the stale shot with a note rather than dropping it.
- **`mcp/query.py::_METRIC_FIELDS`:** add the new fields.

### Rust mirror

- **`crates/contracts/src/shot.rs`:** the new fields with `#[serde(default)]`, and the
  `SCREEN_PARSER_VERSION` constant.
- **New `crates/contracts/src/capability.rs`:** `include_str!` of `devices.json`, so there is one
  copy on disk (ADR-032 §5).
- **`ANALYSIS_VERSION` 16 → 17,** in both `crates/contracts/src/swing.rs` and
  `contracts/swing.py`.
  - The ledger entry says shape only: no number moved.
  - The bump is forced by the "What the code says" finding 3.

### Sequence

Run this on the Windows box only; it holds `data/` and the `ocr` extra.

1. Snapshot `data/processed/shots/*.shot.json` to the scratchpad.
2. Make the code changes.
3. Run `scripts/reanalyze.py`.
4. Run `scripts/import_shot_screens.py --force` on `data/raw/shot_screens/`.
5. Run `scripts/conformance.py regenerate`.
6. Run the full verify suite (below).

### Tests and new pins

- **`tests/contracts/test_capability.py`:**
  - every declared field exists on `ShotData`;
  - every `shown_only` field has a note;
  - tracked = declared ∩ present;
  - `low_point` and `attack_angle` are never tracked for `hd_golf`;
  - an unknown device is refused.
- **`tests/launch_monitor/test_profiles_capability.py`:** each profile's targets equal its
  `devices.json` field set.
- **`tests/launch_monitor/test_screen_parser.py`**, with a new `BAY_ROWS` layout in `conftest.py`
  that has the `Custom` and `Impact Position V` tiles:
  - V is read;
  - `---` becomes `None`;
  - `Impact Position` and `Impact Position V` never claim each other, including OCR-damaged labels;
  - `fields_present` is filled;
  - `scale` is applied;
  - the parser version is stamped.
- **Stale-cache tests** in `tests/launch_monitor/test_screen_store.py` and
  `tests/api/test_pipeline_shot_screen.py`.
- **Existing pins to update:**
  - `tests/mcp/test_query.py`'s exhaustive list;
  - `tests/test_conformance.py::_PACKAGE_DATA` gains `devices.json`.
- **Rust:** unit tests in `crates/contracts`, and `round_trip.rs` passes on the re-recorded
  vectors.

### Exit

- Every suite is green.
- The re-parsed shots, diffed against the snapshot, show **added keys only; no value moved**.
- `fields_present` separates the bay layout from the reference layout.
- The ADR-014 addendum records how many photos have V, and how many of those are non-blank.

## M33 — Apple Vision spike (Mac)

**The recognizer:** a Swift CLI, `tools/vision-ocr/`.
- It runs `VNRecognizeTextRequest` in `.accurate` mode, with `usesLanguageCorrection=false` and
  `customWords` = the profile labels.
- It flips Vision's bottom-left origin to top-left and emits boxes as JSON.

**The harness:** a Python `JsonBoxesRecognizer` implementing `TextRecognizer`.
- Vision's boxes go through today's `parse_screen` / `validate_parse`.
- The resulting shots are diffed per field against the stored ones.

**Two modes:**
1. the OpenCV-rectified images;
2. raw photos, rectified with `VNDetectRectanglesRequest` + `CIPerspectiveCorrection`.

**Gate:**
- no field value differs unless it is flagged `needs_review`;
- `min_confidence` is calibrated for this recognizer. Vision's confidences are coarse, so the 30%
  OCR weight in `parse_confidence` and the 0.6 review threshold need re-checking.

## M34 — The screen reader in Rust

**New `crates/screen`,** depending on `contracts` only, with no `regex` crate:
- `profile.rs`: `include_str!` of `profiles.json`;
- `difflib.rs`: CPython's `SequenceMatcher.ratio`, reproduced exactly;
- `parser.rs`, `validate.rs`, and `orient.rs` (`label_ratio`).

**Crate split:** move `crates/analysis/src/pyfmt.rs` into its own `crates/pyfmt`, so that `screen`
does not depend on `analysis` (ADR-008 as a cargo edge). Check `docs/REFACTOR_LEDGER.md` first.

**Entry point:** `golf-core parse-screen`, boxes in, `ShotData` out.

**Vectors:**
- `spec/vectors/screen/{synthetic,corpus}`: PaddleOCR boxes from the stored photos → the expected
  `ShotData`. The family ages on `SCREEN_PARSER_VERSION`.
- Format-family tables for every edge in "What the code says" finding 5, generated from CPython.

**Exit:** `cargo test` passes on every screen vector.

## M35 — Shot-first sessions (Python, which is M36's oracle)

**Engine:**
- New `contracts/shot_result.py::ShotResult`, written as `shot_analysis.json`.
- In `analysis/engine.py`, extract `_shot_measurements(shot, flown)` out of `_measurements`. The
  swing output stays byte-identical.
- Add `analyze_shot(...)`, filtered by `tracked_fields`.

**Storage and API:**
- `api/pipeline.py`: a directory holding only a shot photo writes `shot_analysis.json`.
- `api/state.py::load_shot_analysis`.
- `contracts/career.py`: `face_on_sha256` becomes optional.
- `storage/corpus.py::read_corpus`:
  - admits photo-only entries;
  - dedupes them by photo hash;
  - no longer lets `NO_FACE_ON` exclude them.

**Scripts:**
- `scripts/import_shot_screens.py` gains `--player --club --into-session`.
- `scripts/reanalyze.py` covers photo-only directories.

**Vectors and Rust:**
- `spec/vectors/shot/…`, recorded from Python.
- Rust: `analysis::engine::analyze_shot` and `golf-core run-shot`.

**Tests:**
- `tests/storage/test_corpus.py`: photo-only, dedupe, mixed sessions.
- `tests/analysis/test_engine.py`: a pin that the swing output is unchanged.
- `tests/api/test_pipeline_shot_screen.py`.

## M36 — The many-shot layer in Rust

**Ports, with Python as the oracle:**
- To `crates/contracts`: career, baseline, dispersion, club_profile, mishit, bag, club and
  shot_result.
- To `crates/analysis`: `baseline`, `dispersion` and `club_profile`. `stats.rs` gains the CI
  helpers.

**New `crates/storage`:** manifest, bundle store, `corpus` and the shot store. The bundle store
takes an explicit target ("What the code says" finding 9).

**Vectors:**
- `spec/vectors/career/`: synthetic corpora crossing every n-gate (4/5/9/10/11/12) and the mishit
  floor.
- `spec/vectors/storage/`.

## M37 — Strike profile and strengths/weaknesses (Rust first)

**Contract:** `crates/contracts/src/strike.rs`, holding `ImpactCell`, `StrikeProfile` and
`RateEstimate`. `RateEstimate` is a Wilson interval using stdlib `sqrt` only.

**`crates/analysis/src/strike.rs`, per club:**
- the impact grid and the centered-strike rate;
- heel/toe asymmetry;
- ball-speed SD;
- carry, start line and face-to-path, each through the existing `dispersion`;
- low point, only when it is tracked;
- everything gated by `minimum_n`.

**`crates/feedback/src/profile_rules.rs::rank_profile`:**
- Weaknesses are established bias or scatter, ranked by how far they exceed tolerance.
- Strengths are equivalence claims only.

**Vectors:**
- hand-worked synthetic vectors, plus Rust-recorded corpus vectors;
- a new pin: every vector family names its oracle.

**Reaching it from the lab:**
- `golf-core profile`;
- `mcp/club.py` gains `get_strike_profile`. The MCP tool count is pinned in the docs-truth suite.

## M38 — The iPhone app (Mac; P5 at the bay)

- **P0 — toolchain.** Install the `aarch64-apple-ios` and `-sim` targets, Flutter and FRB. Get
  `cargo test` green on macOS; it has never run on anything but Windows.
- **P1 — facade crate `crates/app`.** It exposes `parse_screen`, the session cursor,
  `record_shot`, `correct_shot`, `bag_profile`, `strike_profiles`, `profile_feedback` and
  `export_sessions`. It does not depend on `capture` or `pose`.
- **P2 — the Flutter `app/` skeleton.** The flow:
  1. pick golfer and bag, then club;
  2. VisionKit document camera;
  3. Vision OCR (a Swift plugin);
  4. Rust parse;
  5. review/edit when the shot `needs_review`;
  6. history.
- **P3 — profile screens:** the per-club profile, strengths and weaknesses.
- **P4 — export:** `scripts/import_phone_export.py`.
- **P5 — bay session:** ≥30 photos across ≥2 clubs. Record the success rate, the review rate and
  latency.

**Exit:**
- At the bay, with no laptop, photos become stored, validated shots and the per-club profile
  renders.
- Python `read_corpus` agrees with the Rust reader on the exported data.

### Testing on the iPhone without the App Store

- **Day to day:**
  - Plug the phone into the Mac and run `flutter run`. Wi-Fi works after the first pairing.
  - Turn on Developer Mode on the phone (Settings → Privacy & Security).
  - Sign with a free Apple ID "Personal Team" in Xcode.
  - The install expires after 7 days (re-run to re-sign), and a free account allows at most 3
    sideloaded apps.
  - Camera, Vision and VisionKit need no paid entitlement, only `NSCameraUsageDescription`.
- **The iOS Simulator** covers the photo → OCR → parse loop, using the stored bay photos from the
  photo library. It has no camera, so the document scanner is device-only.
- **Most of the product is tested without the Mac.** The Rust parser, strike analysis and feedback
  run under `cargo test` and the vectors on the Windows box. The Mac is only needed for the
  Flutter shell and Apple Vision.
- **The paid Apple Developer Program ($99/yr) is optional, later.** It gives 1-year signing, and
  TestFlight internal testing needs no App Review.

## M39 — Optional video on the phone (Mac, then bay)

- **P0 — pose conformance gate:**
  - iOS `PoseLandmarker` with the same `.task` file, pinned by sha256. Pin the laptop's `latest`
    URL first.
  - VIDEO mode, CPU delegate.
  - Re-pose the stored face-on clips, then run `golf-core run`.
  - **Gate:** every `passed` verdict is identical, and every delta is within its tolerance.
  - **If it fails:** the phone records, and mechanics are computed on the laptop (M40).
- **P1 — capture:**
  - record face-on;
  - detect the strike with `crates/trigger` on-device;
  - attach the clip to the current shot explicitly.
- **P2 — overlay:**
  - the impact frame, carrying that shot's launch-monitor facts;
  - the red/green low-point marker only when the device tracks low point, so never for HD Golf.

## M40 — The laptop client resumes

- M21, M24 and M25 are re-scoped: the same Flutter app as a desktop target.
- An R10 BLE adapter (`btleplug`), plus its `devices.json` entry.
- Launch-monitor API adapters.
- A laptop OCR recognizer behind the same boxes seam.
- M29's deletions proceed once `api/` and `mcp/` are retired.

---

## Invariants every milestone keeps

- **ADR-008:** `analysis` imports only `contracts`, and capability data lives in `contracts/`.
  `screen` must not depend on `analysis`, hence the `pyfmt` split.
- **Stdlib-only scoring, in both languages:** no `regex` crate, and Wilson intervals written by
  hand.
- **"No score beats a wrong one," extended:** not tracked is not unscored. Pin that an untracked
  field appears nowhere.
- **An `ANALYSIS_VERSION` bump regenerates `spec/vectors/` in the same change,** and only on the
  Windows box, where `data/` lives.
- **These two tests must stay green:** `tests/api/test_pipeline_imports.py` and
  `tests/test_docs_truth.py`.

## Verify, every milestone

```bash
.venv/Scripts/python.exe -m pytest
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
.venv/Scripts/python.exe scripts/conformance.py check
cargo test
cargo clippy --all-targets && cargo fmt --check
```

Run `cargo test` unpiped, or check `PIPESTATUS`: `cargo test | tail` reports `tail`'s exit status
(WORKLOG, M23 P9). Don't run it at the same time as pytest either. The timing test
`tests/api/test_worker.py::test_failure_is_recorded_and_the_consumer_survives` failed once under
that load on 2026-09-29, and passed alone.
