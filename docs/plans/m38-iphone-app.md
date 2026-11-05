# Plan: M38 — The iPhone app

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against code
before acting on one. Phase findings are appended as phases close, and a finding contradicting the plan
wins, except that **nothing may move a bar in `docs/plans/m38-bay-thresholds.md`** (rule 5).

**Planned**: 2026-11-04, on the arm64 Mac, with the whole stack spiked on the iOS Simulator during
planning (the section "What the planning spike measured"). **Milestone**:
[ROADMAP §M38](../../ROADMAP.md#m38-the-iphone-app) and the program plan's
[§M38](m31-m40-shot-first-pivot.md#m38--the-iphone-app-mac-p5-at-the-bay). This file is the phase list,
and where it and either of those disagree, **this file wins**. The program plan's P0 to P5 are all here,
renumbered. What is deliberately overturned: VisionKit moves from "doing rectification" to conditional
(D12), and both a device choice and clip import leave M38 (D3, D4).

**Governing decisions**: [ADR-034](../decisions/034-shot-first-phone-first.md) clauses 6 and 7 with its
M33 and M42 addenda, [ADR-030](../decisions/030-app-platform-rust-core-python-sidecar.md) §4 as amended,
[ADR-035](../decisions/035-rust-everywhere-python-where-required.md) clauses 1 and 4,
[ADR-014](../decisions/014-screen-capture-shot-ingestion.md),
[ADR-010](../decisions/010-benchmark-ranges.md) §2, and
[the M29 plan's C11](m29-lab-port.md) with its P31 finding 6.

**Level: L3.** `crates/app` is a new subsystem and the phone is a new host. No shape in `contracts/`
changes and no version key moves (rule 4), so each phase reads its own list below and not the repo.

**How this runs.** P1 to P28 need no person and no iPhone: `/orchestrate m38 --max 40`, in as many runs
as it takes. **P29 to P32 and P34 are attended** (🧍). A `phase-runner` reports `BLOCKED` at each of them
by design, and you then run it with `/next-phase`, with the iPhone at the Mac. That stop is not a
question: each attended phase lists what you do and what the session does. **The bay session itself
happens between P31 and P32.** The only stops that ask you to choose are P32's calls on `W_OCR` and on a
latency-only miss, P34's last step, and rule 11's two hard stops. P33, P35 and P36 need no person.

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot, and address anything a later phase must
   know to that phase by number ("**For P17**: …");
3. stop, and tell the user to `/clear` and run `/next-phase`.

| Phase | What | State |
|---|---|---|
| **P0** | This plan, the bay bars, the pointers, and the pre-registration commit | ✅ Done *(2026-11-04)* — approved by the user |
| **P1** | The toolchain: Flutter, the bridge's codegen, the iOS targets | ⬜ |
| **P2** | `cargo test --workspace` green on macOS | ⬜ |
| **P3** | The photo entry's opener moves from `golf-lab` down to `golf-core` | ⬜ |
| **P4** | `crates/app` exists, builds for iOS, and is held off `ort` | ⬜ |
| **P5** | Facade: golfers, bags and the app's own state | ⬜ |
| **P6** | Facade: `record_photo`, boxes in and a stored, analysed shot out | ⬜ |
| **P7** | `correct_shot`: what a person's edit stores | ⬜ |
| **P8** | The lab's re-read keeps a person's correction | ⬜ |
| **P9** | Facade: history and one shot's detail, as views | ⬜ |
| **P10** | Facade: a shot's actions (club, intent, mishit, delete) | ⬜ |
| **P11** | Facade: the graded profile, as a view | ⬜ |
| **P12** | `import-export`'s value rule for a golfer and a bag | ⬜ |
| **P13** | Facade: `export_sessions`, round-tripped through `import-export` | ⬜ |
| **P14** | `golf-bay-report bars`: the session's verdict from an export | ⬜ |
| **P15** | `golf-bay-report truth` and `compare`, controlled on M33's numbers | ⬜ |
| **P16** | The Flutter `app/` and its bridge, running on the Simulator | ⬜ |
| **P17** | `ScreenReader.swift`, held to M33's committed boxes on macOS | ⬜ |
| **P18** | The six captures through iOS Vision and Rust, on the Simulator | ⬜ |
| **P19** | The app's shell: model, theme, tabs, first-run setup | ⬜ |
| **P20** | `PhotoSource.swift`: the camera, the library, a photo's time | ⬜ |
| **P21** | The Record tab | ⬜ |
| **P22** | The review screen | ⬜ |
| **P23** | Challenge mode | ⬜ |
| **P24** | The History tab | ⬜ |
| **P25** | The shot detail screen and its actions | ⬜ |
| **P26** | The Profile tab | ⬜ |
| **P27** | The Settings tab: export and share | ⬜ |
| **P28** | The whole flow on the Simulator: desk-complete | ⬜ |
| **P29** | 🧍 The iPhone: paired, signed, launched with no cable | ⬜ |
| **P30** | 🧍 Dry run on the iPhone: the six captures, the camera, real HEICs | ⬜ |
| **P31** | 🧍 Before the bay: re-sign and the run sheet. Then you go to the bay | ⬜ |
| **P32** | 🧍 The session's export: the verdict, the exit's corpus check, and your calls on it | ⬜ |
| **P33** | VisionKit's scanner route, if P32 missed T1 to T4 | ⬜ |
| **P34** | 🧍 The scanner measured at the bay, if P33 ran, and your decision | ⬜ |
| **P35** | Docs | ⬜ |
| **P36** | Close: verify, WORKLOG, ROADMAP, memory, and the close commit | ⬜ |

P33 and P34 are conditional. A phase whose condition is not met still runs: it reads the result, proves
the condition is not met, ticks its row with "not needed", and records that as its finding.

---

## Rules every phase keeps

1. **Stop after every phase.** One phase, one session, one gate.
2. **Only P0 and P36 commit** (D15). **All other work lives uncommitted in the tree, for weeks across the
   device and bay gaps, by the user's choice.** So no phase runs `git checkout`, `git restore`, `git
   clean`, `git stash` or `git reset` on anything, nor deletes a file it did not create. A runner that
   finds the tree holding work it does not recognise reads it as an earlier phase's and carries on.
3. **Python gains nothing, and frozen Python is not touched** (ADR-035 clause 4). New work is Rust, plus
   Dart and Swift under `app/`. `app/tool/` is shell.
4. **No version key moves, and `spec/` is read-only.** `ANALYSIS_VERSION`, `SCREEN_PARSER_VERSION` and
   `CAREER_VERSION` stay, no committed vector moves, and `crates/screen/profiles.json` and
   `crates/contracts/devices.json` stay. If a phase cannot pass without one of these, it reports `BLOCKED`.
5. **`docs/plans/m38-bay-thresholds.md` and `spikes/vision-ocr/` are read-only after P0.** M33 stays open
   and its files are its own.
6. **`data/` is not written.** Tests run on scratch directories. P32 is the one phase that places a file
   there, a zip under the gitignored `data/raw/phone-exports/`.
7. **The boundary.** `crates/app` holds no `flutter_rust_bridge`, and `crates/app/src/api/` holds only what
   crosses to Dart: free functions, and structs and plain enums of `String`, `i64`, `f64`, `bool`, `Option`,
   `Vec` and tuples, returning `Result<T, String>` where they can refuse. **No enum carries fields**: that
   needs `freezed` and `build_runner`, and an `Option` field says the same thing. No `pub` helper goes in
   `api`, because the codegen binds every one. Everything else lives outside `api` in the crate.
8. **Dart holds layout, and no golf.** No rounding, threshold, sign rule, sentence about a grade or
   arithmetic on a value in Dart: Rust hands each view its display strings. `None` shows as not read and
   never as 0, a blend with no graded topic is absent, and what is not printed is absent (ADR-010 §2,
   ADR-034 §2 and §5.5).
9. **One path for boxes.** Swift returns boxes as JSON text, Dart passes the text through unparsed, and Rust
   parses it, because a Dart double would be one more place `rgb`'s bits could move.
10. **No network and no runtime plugin.** The app's runtime dependencies are the Flutter SDK and
    `flutter_rust_bridge` alone, native work goes through this repo's Swift channel, and nothing opens a
    socket. Versions are exact and `pubspec.lock` is committed. **Never `flutter pub upgrade`.**
11. **What stops a phase.** The plan's decisions are final, so a runner does not re-open them. Report
    `BLOCKED` if something in the plan is wrong and rule 4 or a decision stands in the way. Two hard stops
    are named: P18 finding a **wrong** cell on an unflagged shot, and P30 finding a lit letter on the wrong
    side of `LIT_BLUE_RATIO`.
12. **Everything read off the six captures is labelled `provisional: pasted copies`**, wherever it is
    written: they are chat-pasted JPEGs and not the originals (M33's decision of 2026-11-04).
13. **House style.** Comments and docs say why, and a test's name says what it pins. A new doc declares its
    tier on its first lines. Pipe build and test output through `tail` or `grep`.
14. **`/opt/homebrew` belongs to another account and CocoaPods is absent.** Neither is needed (C3).
    Do not install either.
15. **The cheap gate is the default.** `cargo test -p <crate>` for what a phase touched; the whole workspace
    only in P2, P28 and P36; the Simulator only in phases that name it.

## Verify — the commands, and when each runs

`cargo` is not on `PATH` in this environment's shell. P1 writes `app/tool/env.sh`, and every command below
assumes `. app/tool/env.sh` has run in that same Bash call. Until P1: `export PATH=$HOME/.cargo/bin:$PATH`.

| Command | When |
|---|---|
| `cargo test -p app 2>&1 \| tail -15` | every phase touching `crates/app` |
| `cargo clippy -p app --all-targets -- -D warnings 2>&1 \| tail -15` | same |
| `cargo fmt --check` | every phase touching Rust (it is clean today) |
| `cargo build -p app --target aarch64-apple-ios 2>&1 \| tail -3` | P4 onward, every phase adding a dependency to `crates/app` |
| `cargo test -p ocr --test ort_reach 2>&1 \| tail -3` | P4, P13, P36 |
| `.venv/bin/python -m pytest tests/test_docs_truth.py 2>&1 \| tail -3` | every phase touching a doc or the workspace's members (91 pass today) |
| `cargo test --workspace --no-fail-fast 2>&1 \| grep -E '^test result\|FAILED\|failed' \| tail -40` | P2, P28, P36 |
| `(cd app && flutter analyze 2>&1 \| tail -5 && dart format --output=none --set-exit-if-changed lib test integration_test \| tail -3)` | every phase touching Dart |
| `(cd app && flutter test 2>&1 \| tail -5)` | every phase touching Dart |
| `app/tool/sim_test.sh integration_test/<file>.dart 2>&1 \| tail -20` | the phases naming it. It boots "iPhone 17", runs with `--dart-define=GOLF_REPO=<repo>`, and shuts the Simulator down |
| `app/tool/codegen_check.sh` | every phase changing `crates/app/src/api/` after P16. It regenerates, and fails if a generated file differs before and after the second run |
| `app/tool/reader_check.sh` | P17 onward, every phase touching `ScreenReader.swift` or `PhotoTime.swift` |

## Decisions taken in the interview (2026-11-04)

Each is the user's, from M38's planning interview. **None is re-opened by a phase.**

- **D1 — Start on M33's provisional evidence.** The ≥30-photo bay session is the real test of its verdict.
  M33 stays open until its originals, clips, HD Golf and lab baseline have run, and M38 does not wait for it.
- **D2 — The iPhone is not at this Mac yet, and runs iOS 16 or later.** Everything possible runs on the
  Simulator first. The device phases wait. **The deployment target is iOS 16.0**, because
  `VNRecognizeTextRequestRevision3` and `automaticallyDetectsLanguage` need it.
- **D3 — Full Swing only.** No device picker: the app passes one constant, `app::DEVICE`. The `hd_golf`
  profile stays in `crates/screen`, unoffered, with no Vision evidence behind it.
- **D4 — Stills only.** A Live Photo imports as its still, one shot with no impact offset. `crates/app`
  takes no edge to `crates/impact`, and clip import is a named follow-up for when the reader works. M42's
  "ask for one per Live Photo" does not arise.
- **D5 — In-app camera and library import.** One tap to photograph, and the read follows at once. A
  multi-select library import covers photos taken elsewhere.
- **D6 — The phone creates its golfer, and the lab gains a value rule.** In `golf-lab import-export`: the
  same `player_id` with the same handedness is the same golfer, the lab's copy is kept, and the difference
  is printed. A different handedness stays a refusal. A bag that differs is kept as the lab's and printed.
  It is a dated addition to the M29 plan's C11 (P12).
- **D7 — A bag is slots and an optional loft.** Which clubs are in it, and a loft on each if known. The club
  picker lists only those. No shaft, no catalogue.
- **D8 — `W_OCR` stays, decided after the bay.** `min_confidence` is 0.6 (M33's provisional, vacuous value)
  and the parser is untouched. The bay report prints the confidences, and P32 puts the question to the user with
  that evidence.
- **D9 — The card always shows, and a flagged shot must be reviewed.** Every read shows a result card with
  its values. A printed tile with no value is highlighted and can be typed in. A flagged shot must be reviewed
  before the next photo, and an unflagged one needs no tap.
- **D10 — Delete and mishit on a shot's detail screen.** Delete removes a stray photo's entry. Mishit
  confirms or clears the verdict that holds a shot out of the distance averages.
- **D11 — Challenge mode ships.** Each call is uniformly random over the shapes ticked at the start, and shows
  large until the photo is recorded. Its shots count fully (Decision 17 §1).
- **D12 — VisionKit only if raw misses.** Ship the raw path. P33 and P34 build and measure the scanner only if
  P32's verdict misses T1 to T4. If not, ADR-034 clause 7 is amended to say the scanner is unmeasured by choice.
- **D13 — Four tabs, dark first.** Record (opens first), History, Profile, Settings. Material 3. Appearance is
  a setting with `Dark` the default, then `System` and `Light`.
- **D14 — M33's bars as shares, plus latency**, in `m38-bay-thresholds.md`. A miss does not fail the milestone
  by itself: T1 to T4 trigger P33, and the user decides.
- **D15 — Two commits on `main`.** P0's docs-only pre-registration (the plan, the bars and the pointers), then
  P36's close commit holding everything else. Nothing is pushed. The user chose this knowing the tree stays
  uncommitted across the device and bay gaps.

## What the planning spike measured (2026-11-04)

All of it in the scratchpad, and nothing in the repo. It left traces only in `~/.rustup`, `~/.cargo`,
`~/.pub-cache` and Flutter's user settings (analytics off). Numbers are from one run.

- **The whole path runs on the iOS Simulator with no CocoaPods.** `flutter test integration_test/… -d <iPhone
  17, iOS 26.2>`: Dart called a Swift `MethodChannel`, Swift ran Vision at C0 on
  `spikes/vision-ocr/captures/IMG_2889.jpg`, and Dart passed the boxes to `screen::read` through the bridge.
  **42 boxes, the committed `raw` file's count, and its first twelve texts equal.** Vision took 1.26–1.28 s in
  the app. Only that one photo was read, and with `device: hd_golf`, so it shows the stack runs and not that the
  read is right. P18 is the read.
- **The phone's closure compiles for iOS.** `golf-core`, `screen`, `storage`, `contracts` and `zip =7.3.0` as
  path dependencies of the bridge crate built for `aarch64-apple-ios-sim` and linked as
  `golf_bridge.framework`. The first app build took 67 s, and later ones 8–16 s.
- **Flutter 3.47.9 has native assets and Swift Package Manager on by default**, and FRB 2.13.0's
  `--integration-backend native-assets` builds through `hook/build.dart`, with no podspec. `flutter doctor`
  reports CocoaPods and Android missing, and neither stopped anything.
- **`rust-toolchain.toml` must name an exact channel** (`native_toolchain_rust`'s rule). `1.99.1` worked.
- **The bridge binds a plain external crate.** `rust_input: crate::api,fac::api` generated
  `lib/src/rust/third_party/fac/api/*.dart`, submodules included, with `fac` holding no FRB dependency at all.
  A free function, a struct's method, `Option`, `Vec`, a tuple (`.$1` in Dart), a plain enum and `Result<_,
  String>` all ran (an `Err` throws the `String` in Dart). The submodule layout passed `cargo check` and codegen,
  and the others ran on the Simulator.
- **An enum carrying fields needs `freezed` and `build_runner`** (`MissingDep`), hence rule 7. Bridged functions
  are async in Dart by default, so nothing here blocks the UI.
- **A Simulator app reads and writes host paths**, and `--dart-define` reaches the test. So integration tests take
  the repo's path and leave their results under `target/m38/`.
- **`flutter test` on a plain widget test passes** in 21 s with the hook present.
- **The Flutter zip's first download was silently truncated**, 506 MB of 2.27 GB, with `curl -sL`. Use `curl -fL`
  and check the digest. Unpacked, it is 4.1 GB.
- **FRB's template lock resolves `tokio 1.34.0`, where `crates/mcp` pins `=1.54.0`**, with 109 packages against the
  root lock's 182. One reason the bridge has its own workspace (C1).
- **The five macOS failures, read during planning.** `capture`'s three: `list()` answers `Unsupported` off Windows,
  and each `unwrap`s it. `flight`'s `an_empty_store_says_how_shots_arrive`: macOS's long temp path wraps the sentence
  at 72 columns before the path, so `No parsed shots in ` is not followed by a space. `import_photos`'
  `a_path_is_spelled_as_pythons_str_path`: the row reads `failed`, exit 1, on `{root}/photos/./shot-1.jpg`, and its
  stderr was not read.
- **Baselines on this Mac**: Xcode 26.2, the iOS 26.2 runtime with `iPhone 17` available, `cargo fmt --check` clean,
  `tests/test_docs_truth.py` 91 passed, clippy 28 `warning` lines (M33's count). No iPhone is paired and there are 0
  signing identities.

## Calls this plan makes

Each was taken as the option with the least to go wrong, and approved with the plan.

**C1 — Two Rust crates.** `crates/app` (package `app`) is a workspace member, plain Rust, tested by `cargo test` on
any OS. `app/rust` (package `golf_bridge`) is the FRB shell, with `crate-type = ["cdylib", "staticlib"]`, an empty
`[workspace]` table detaching it from the root workspace, its own committed `Cargo.lock`, and its own
`rust-toolchain.toml`. So the root lock, the root's Rust version and the Windows box's `cargo test` never meet FRB. Its
`api` module holds only the `#[frb(init)]` function, and `app/flutter_rust_bridge.yaml` is:
```yaml
rust_input: crate::api,app::api
rust_root: rust/
dart_output: lib/src/rust
```
A pin (`crates/app/tests/lock_agreement.rs`) holds the two locks to one version each of `serde`, `serde_json`, `sha2`,
`flate2`, `zip` and `unicode-normalization`, because those decide bytes.

**C2 — The facade's whole surface**, one file per row under `crates/app/src/api/`, each `root` being the data root:

| File | Functions |
|---|---|
| `about.rs` | `about() -> AboutView` (the three version keys, `DEVICE`, the crate's version) |
| `setup.rs` | `init_state(root, install_id)`, `get_state(root)`, `set_state(root, state)`, `list_golfers(root)`, `create_golfer(root, name, handedness, now)`, `all_clubs()`, `bag_slots(root, player_id)`, `set_bag_slot(root, player_id, club, loft_deg, now)`, `remove_bag_slot(root, player_id, club, now)` |
| `record.rs` | `record_photo(PhotoIn) -> RecordOutcome`, `log_read(root, ReadLog)` |
| `review.rs` | `correct_shot(root, session_id, swing_id, edits, now) -> ShotCard` (empty `edits` is "reviewed, nothing to change") |
| `history.rs` | `list_sessions(root, player_id)`, `shot_detail(root, session_id, swing_id)`, `set_shot_club`, `set_shot_intent`, `set_shot_mishit`, `delete_shot` |
| `profile.rs` | `graded_profile(root, player_id) -> ProfileView` |
| `export.rs` | `export_sessions(root, out_path, now) -> ExportSummary`, `export_status(root)` |
| `bay.rs` | `truth_report(root, photos_path, truth_path) -> TruthReport` |

Clubs cross as `ClubId`'s own spelling (`7i`), times as RFC 3339 strings with an offset, refused otherwise.

**C3 — The native-assets backend, no CocoaPods, no plugin.** The spike's route, unchanged. Pins: Flutter **3.47.9**
stable (Dart 3.13.7), `flutter_macos_arm64_3.47.9-stable.zip`, sha256
`6545b85938745dc5070e9faf2adf70233c1811009d87fe6d27bfcda5a04b7698`; `flutter_rust_bridge` and
`flutter_rust_bridge_hooks` and the codegen **2.13.0** exactly; Rust channel **1.99.1** in `app/rust/rust-toolchain.toml`,
with targets `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `aarch64-apple-darwin`. Flutter lives at
`~/.local/share/golf-coach/flutter-3.47.9`, the codegen at `~/.local/share/golf-coach/frb`, beside M33's ffmpeg.

**C4 — The native side is this repo's Swift, on one channel** `golf/phone`: `documentsDir`, `readScreen(path)`,
`cameraAvailable`, `takePhoto`, `pickPhotos`, `share(path)`. On Flutter 3.47's implicit engine it is registered in
`didInitializeImplicitFlutterEngine` through `engineBridge.applicationRegistrar.messenger()`, as the spike ran it. A new
Swift file is added to `Runner.xcodeproj/project.pbxproj` by hand: a `PBXBuildFile`, a `PBXFileReference`, its entry in
the Runner group and in the Sources phase, each with a fresh 24-hex id.

**C5 — The reader is M33's C0, exactly**, per [the log's "For M38"](../../spikes/vision-ocr/log.md#for-m38):
`VNRecognizeTextRequest`, `.accurate`, `VNRecognizeTextRequestRevision3`, `usesLanguageCorrection = false`,
`recognitionLanguages = ["en-US"]`, `automaticallyDetectsLanguage = false`, no custom words, one box per observation from
`topCandidates(1)[0]`, flipped to top-left pixels and unrounded. Decode: `CIImage(contentsOf:, options:
[.applyOrientationProperty: true])` rendered through `createCGImage(format: .RGBA8, colorSpace: sRGB)`, `rgb` read from
those bytes with no colour matching, ranked on `(sum, rowMajorIndex)` as `glyphRGB` does. From `vision_ocr.swift`, copied
as code that now ships: `Pixels`, `clipped`, `glyphRGB`, `recognize`, the decode, `jsonNumber`, `jsonString`, `jsonRGB`.
`rectify`, `selftest` and the command line stay in the spike. The spike reloads the decode from a PNG, and the app does
not: if P17's gate finds the in-memory bytes differ, the app writes the same temporary PNG the spike does.

**C6 — `ScreenReader.swift` and `PhotoTime.swift` import no UIKit or Flutter**, only Foundation, CoreImage, ImageIO, Vision
and CoreGraphics. So `app/tool/reader_check.sh` compiles the app's own files with `swiftc` for macOS against
`app/tool/reader_main.swift`, and holds what ships to M33's committed boxes.

**C7 — The data root** is `<Documents>/processed/`, with `sessions/`, `shots/` and `golfers/` as `crates/storage` writes
them, and one dotted directory of the app's own: `.app/state.json`, `.app/reads.jsonl`, `.app/boxes/<digest>.json` (every
read's boxes as Swift returned them, failed reads too) and `.app/failed/<digest>.<ext>`. `import-export` ignores and counts
a dotted name, so the diagnostics travel in the export and never reach the lab's stores. `state.json` stays home.
`AppState` holds: `install_id`, `player_id`, `club`, `mode`, `target_shape`, `challenge`, `challenge_shapes`, `appearance`,
`last_export_at`, `last_export_shots`.

**C8 — Session ids are `<date>-phone-<install>`**: the date is the photo's capture date in its own offset, and `install` is
4 lowercase hex digits Dart generates once, so two phones or a reinstall cannot collide with each other or with the lab.
They sort by date and `is_safe_segment` admits them. A shot's id is `{session}-{N}`, as the lab stamps it. **The phone
writes no `session.json`**: the current golfer, club and intent live in `state.json` and are stamped straight onto each
manifest, so changing clubs can never make a cumulative export conflict at the lab.

**C9 — A photo's time.** Camera: now, with the device's offset. Library: EXIF `DateTimeOriginal` with `OffsetTimeOriginal`,
else that date in the device's offset, else the file's date, else now. Swift answers it, as RFC 3339. It stamps the shot,
names the session, and is set as the stored photo's modification time.

**C10 — The camera is `UIImagePickerController`** with `showsCameraControls = false`, an overlay holding a shutter, a cancel
and the words "Fill the frame with the tiles", and `takePicture()`, so a tap goes straight to a read with no "Use Photo" step.
The image is stored as JPEG at quality 0.95 with its orientation tag kept. **The library is `PHPickerViewController`**, any
number of images, `preferredAssetRepresentationMode = .current` and `loadFileRepresentation`, so an original HEIC stays a
HEIC and its bytes are what is stored and hashed. App UI is portrait-only. The photo's orientation comes from its metadata,
so a landscape photo of a wide monitor still reads upright (held to it in P30).

**C11 — What `corrections` records**, which M32 left to M38. Field name to **the value as read before a person changed it**:
`pyfmt::repr` of the number, or `""` if nothing was read. Recorded on the first edit only, and removed if a later edit
restores that value. So the typed field holds the truth and the map is the audit, which is what the bay report reads. Saving a
review sets `needs_review = false`, with or without edits, because a person has now looked. Warnings stay. Editable fields are
the device's `from: screen` declared fields, numbers only. An edit that fails `ShotData`'s `Validate` is refused with its
message. Clearing a value is allowed. The physics cross-checks do not re-run, because they judge a read and a person has
overruled it. The shot is re-analysed by `analyse_entry`.

**C12 — A re-read keeps a person's correction.** `golf-lab`'s stale re-read and `--force` re-read carry each corrected field's
value and its `corrections` entry onto the new read, beside `carry_impact_video` in `crates/lab/src/shot.rs`. A new flag from the
new read stands, since that is a new question to ask.

**C13 — A read of nothing keeps nothing but the diagnostics.** No entry opens (the lab's rule, reached through P3's code), the
photo goes to `.app/failed/`, and the card says it could not be read and to retake. A photo the session already holds opens no
entry, and the card shows the held shot, marked as already recorded.

**C14 — A library batch** runs one photo at a time with progress, under the current club and intent. Flagged shots queue, and
the queue is reviewed before the batch ends.

**C15 — The intent's rules are `golf_core::session::intent_action`'s**: a shape only with `shot_shaping`, and `shot_shaping`
only with a shape. A per-shot override goes through `SwingBundleStore::set_intent` and re-analyses. Drill and challenge exclude
each other. A challenge's call changes only after a recorded shot, so a failed read keeps it. It is stored as `{mode:
shot_shaping, target_shape: call}`, and `Random` is injected into the model so a test can name the calls.

**C16 — Export is cumulative.** `sessions/`, `shots/`, `golfers/`, `.app/reads.jsonl`, `.app/boxes/` and `.app/failed/`, sorted,
in a zip through `zip =7.3.0` with the lab's exact features: JSON deflated, photos stored, and each entry an extended-timestamp
extra field (0x5455) so a photo keeps its time at the lab. Written to `tmp/`, then handed to `UIActivityViewController`.
Settings shows the last export and the shots since. **A limit, recorded and not fixed**: a correction made after the lab
imported that shot is printed as `kept` by the lab and not applied, which is the M29 plan's dedupe rule.

**C17 — A shot's `image_path` is the root-relative path**, `sessions/S/N/<file>`, since an absolute one holds a container UUID
that changes on reinstall and nothing opens a photo by it. The app finds a photo through the manifest's role.

**C18 — `min_confidence` is `app::MIN_CONFIDENCE = 0.6`**, one constant with a comment naming M33's "provisional and vacuous".

**C19 — Bundle id `dev.golfcoach.lab`, display name `Golf Coach`.** The Apple team is in the gitignored
`app/ios/Flutter/Signing.xcconfig` (`DEVELOPMENT_TEAM=…`, `CODE_SIGN_STYLE=Automatic`), which `Debug.xcconfig` and
`Release.xcconfig` include with `#include?`, so a personal team id is never committed. If Apple refuses the id as taken at P29,
it becomes `dev.golfcoach.lab.<install_id>`, and that is a finding.

**C20 — The bay build is `--release`.** A debug Flutter app will not launch from the home screen without the tool attached, and the
exit says no laptop. **Free signing expires after 7 days**, a reinstall over the app keeps its data, and **deleting the app deletes
every shot**, so export first. P31's run sheet says all three.

**C21 — `golf-bay-report` is a bin of `crates/app`** (`src/bin/golf_bay_report.rs` over a non-`api` `bay` module). An iOS build
compiles the lib alone. It takes a directory or a zip, and parses its limits from `m38-bay-thresholds.md`'s table at run time, so
no bar has a second copy. It takes no `golf-lab` edge, so it compares what `import-export` has already written.

**C22 — The state pattern is `ChangeNotifier` and `ListenableBuilder`**, from the SDK. `lib/src/core/core_api.dart` is an abstract
`GolfCore` with `RustCore` behind it, and `lib/src/phone/phone.dart` an abstract `Phone` with `ChannelPhone` behind it. Widget tests
use fakes of both, with no Rust and no Swift. Integration tests use `RustCore` and a `TestPhone extends ChannelPhone` that overrides
`pickPhotos` with the captures' paths, because a native picker cannot be driven from `integration_test`.

**C23 — Reuse, and no second path.** Every shot-writing call ends in `golf_core::session::analyse_entry`. Warnings go through
`screen::golfer_warnings`, refusals through `screen::tile_refusals`, slugs through `storage`'s `slugify`, the corpus through
`storage::corpus::read_corpus` with `EngineVersions` of `ANALYSIS_VERSION` and `COMPARABLE_FROM`, and the profile through
`golf_core::profile::run_profile`. Grade sentences are `golf_core::reports::profile`'s helpers, made `pub`, not re-spelled.

**C24 — Decisions are recorded as addenda**, not a new ADR (P35): ADR-034 for the product's shape, ADR-030 for the bridge, ADR-014 for
corrections. That is M33's and M42's precedent.

### Not in M38

Clip import and `crates/impact` on the phone (D4). HD Golf on the phone (D3). VisionKit, unless P32 triggers it (D12). Re-reading a
stale shot on the phone: `.app/boxes/` makes it cheap when a parser version first moves, and nothing needs it yet. Android, pose and video
(M39), paid signing and CI (M40), per-shot coaching tips, iCloud, a second device, localisation, and fixing clippy's 28 existing warnings
(newer lints on code not touched here; `crates/app` is held to 0).

---

## Phases

Every phase: mark it 🔄, read what its **Read** names plus findings addressed to it, do the work, run its **Verify**, tick it, record
findings, stop. Rules 2 and 4 bind each.

### P0 — This plan, the bay bars, the pointers, and the pre-registration commit

**Goal**: the plan and the bars are in git before anything is measured.
**Read**: nothing beyond this file.
**Touch**: `docs/plans/m38-iphone-app.md`, `docs/plans/m38-bay-thresholds.md` (both drafted in planning); `ROADMAP.md`'s M38 row and §M38
Status: in progress, on M33's provisional evidence by the user's decision (D1), naming this plan; its NEXT ACTION's first line;
`docs/plans/m31-m40-shot-first-pivot.md`: §M38's table row, and a pointer under "What M33 tells this section" that this plan decides what
that section left open.
**Done when**: the docs-truth test passes, and one commit holds only those files.
**Verify**: `.venv/bin/python -m pytest tests/test_docs_truth.py 2>&1 | tail -3`; `git show --stat HEAD | tail -8`.

### P1 — The toolchain: Flutter, the bridge's codegen, the iOS targets

**Goal**: this Mac can build the app, and one script says how.
**Read**: C3 and the spike's section above.
**Touch**: `app/tool/env.sh` (new; prepends the Flutter, codegen and `~/.cargo/bin` paths); outside the repo, C3's two directories.
**Do**:
1. `curl -fL -m 1500 --retry 3 https://storage.googleapis.com/flutter_infra_release/releases/stable/macos/flutter_macos_arm64_3.47.9-stable.zip -o <scratchpad>/flutter.zip`, checking the digest and retrying once on a mismatch. Unzip to `~/.local/share/golf-coach/flutter-3.47.9`, and remove the zip.
2. `flutter config --no-analytics`; `flutter precache --ios`.
3. `cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked --root ~/.local/share/golf-coach/frb`.
4. `rustup toolchain install 1.99.1 --profile minimal`, `rustup target add --toolchain 1.99.1 aarch64-apple-ios aarch64-apple-ios-sim` (the spike may have left them).
**Done when**: `flutter --version` says 3.47.9, the codegen says 2.13.0, `rustup target list --installed --toolchain 1.99.1` lists both iOS targets, and `xcrun simctl list devices available | grep -c 'iPhone 17 ('` is at least 1. `flutter doctor`'s only complaints are Android and CocoaPods, which C3 does not need.
**Verify**: those commands, from `. app/tool/env.sh` in a fresh shell.

### P2 — `cargo test --workspace` green on macOS

**Goal**: the program plan's "Get `cargo test` green on macOS", so a later failure means something.
**Read**: the spike section's failure list; `crates/capture/src/lib.rs:100-170`; `crates/lab/tests/flight.rs:262-285`; `crates/lab/tests/import_photos.rs:455-490`.
**Touch**: those test modules only, unless item 3 finds a real bug.
**Do**:
1. **`capture`**: `#[cfg(windows)]` on the three tests, unchanged, and a `#[cfg(not(windows))]` test that `list()` answers `Unsupported` naming the OS. That is the crate's stated answer until M40.
2. **`flight`**: compare the prefix on the flattened whitespace the test already computes for its next assertion, since the sentence wraps by path length.
3. **`import_photos`**: read the third value of `run(...)`, the stderr, and fix the cause. A test-side assumption is fixed in the test. A real macOS bug in `golf-lab` is fixed in the code and named in a finding. A fix that would change Windows's spelling stays behind `cfg`, because that box cannot be run here.
**Done when**: the workspace has 0 failures, with Windows's assertions no weaker. The two HEIC gates in `crates/ocr/tests/` skip as they do today. Findings record the count of tests and clippy's `warning` lines.
**Verify**: the workspace `cargo test` line, `cargo fmt --check`.

### P3 — The photo entry's opener moves from `golf-lab` down to `golf-core`

**Goal**: the phone opens an entry with the lab's code, since it cannot link `golf-lab` (which reaches `ort`).
**Read**: `crates/lab/src/import.rs:696-800` and `:925-1003` and `:1021-1050`; `crates/lab/src/import_export.rs`' calls to `session_holding`; `crates/core/src/session.rs`' module doc.
**Touch**: `crates/core/src/session.rs` (gains `session_holding`, `open_photo_entry` and `content_type_of`); `crates/lab/src/import.rs` and `import_export.rs` (call them).
**Do**: move the three, unchanged in behaviour. `open_photo_entry` takes the photo's modification time as an argument, and the lab's caller reads it with `fs::metadata` as today, so the function does not stat. It takes the original file name as an argument too. The module doc moves with it.
**Done when**: no copy is left in `crates/lab`, and every `golf-lab` and `golf-core` test passes unedited.
**Verify**: `cargo test -p golf-lab -p golf-core 2>&1 | grep -E '^test result|FAILED' | tail -30`; `cargo clippy -p golf-core --all-targets 2>&1 | tail -3`.

### P4 — `crates/app` exists, builds for iOS, and is held off `ort`

**Goal**: the facade crate, with `about()` and the rules that bind everything after it.
**Read**: C1, C2, C18; `crates/ocr/tests/ort_reach.rs`; the workspace `Cargo.toml`; `crates/core/Cargo.toml` as the comment style for edges.
**Touch**: `crates/app/{Cargo.toml,src/lib.rs,src/api/mod.rs,src/api/about.rs}`; root `Cargo.toml`'s members; `crates/ocr/tests/ort_reach.rs` (`GUARDED` gains `app`, and the module doc says so); `CLAUDE.md` and `docs/ARCHITECTURE.md` only as far as the docs-truth test's crate count requires.
**Do**: package `app`, lib `app`. Normal edges are declared in the phase that first crosses them, as this repo does. The crate doc states rule 7, why the crate holds no FRB, and D3's and C18's constants. `api::about::about()` answers the three version keys. Times parse in one internal helper (`parse_now`), refusing a naive time. `Dirs::of(root)` names the three stores and `.app/`.
**Done when**: `cargo test -p app` passes `about` and `parse_now`, and the crate builds for both iOS targets. `ort_reach` passes with `app` guarded and its positive controls still fire. `Cargo.lock` gains only `app`.
**Verify**: the `-p app`, clippy, fmt, iOS and `ort_reach` lines; also `cargo build -p app --target aarch64-apple-ios-sim 2>&1 | tail -1`; docs truth; `git diff --stat Cargo.lock`.

### P5 — Facade: golfers, bags and the app's own state

**Goal**: everything first-run setup needs.
**Read**: C2's `setup.rs` row, C7, D7; `crates/storage/src/golfer_store.rs:60-175`; `crates/storage/src/bag_store.rs:40-200`; `crates/contracts/src/bag.rs:50-110`; `crates/contracts/src/club.rs` for `ClubId`'s spelling and order.
**Touch**: `crates/app/src/api/setup.rs`, `crates/app/src/state.rs`, `crates/app/tests/setup.rs`.
**Do**: thin calls to `GolferStore` and `BagStore`, with a bag entry declared as a slot and an optional loft. Plain enums for handedness, mode, shape and appearance, with one test holding each to `contracts`' spelling. `all_clubs()` is in bag order. `state.json` is written atomically. `init_state` writes the `install_id` it is given only if none exists, and `get_state` on nothing answers defaults.
**Done when**: tests pin a golfer's slug and handedness rule (a known golfer's handedness wins), a slot with and without a loft, a removed slot being retired and not lost, a loft refused by `Validate` surfacing its message, and state surviving a reload.
**Verify**: the `-p app`, clippy and fmt lines.

### P6 — Facade: `record_photo`, boxes in and a stored, analysed shot out

**Goal**: the heart of the app, in plain Rust.
**Read**: C2's `record.rs`, C7, C8, C9, C13, C17, C18, C23; P3's findings; `crates/screen/src/lib.rs:130-235`; `crates/screen/src/parser.rs:482-490`; `crates/screen/src/tiles.rs` (doc); `crates/screen/src/profile.rs:51-75,138-160` (how a sign is declared); `crates/core/src/session.rs:147-175,296-400`; `crates/contracts/src/capability.rs:119,199-240`; `spikes/vision-ocr/boxes/C0/raw/*.json`.
**Touch**: `crates/app/src/api/record.rs`, `crates/app/src/card.rs`, `crates/app/tests/record.rs`.
**Do**, per photo: read the bytes and digest; store the boxes under `.app/boxes/`; if the session holds the photo, answer the held card as `Duplicate`; open the entry through P3's code, with `captured_at` as the modification time; `screen::read` with `app::DEVICE`, `MIN_CONFIDENCE` and the shot id and `image_path` of C8 and C17; on `None`, delete the entry, move the photo to `.app/failed/`, and answer `Failed`; `ShotStore::put`; `analyse_entry` to write `shot_analysis.json`; answer `Read` or `Flagged` with a `ShotCard`. `log_read` appends one JSON line to `reads.jsonl`: `at, photo_sha256, source (camera|library|scanner), status, boxes, ms_vision, ms_rust, ms_total`.
**`ShotCard`**: `session_id, swing_id, flagged, confidence, warnings (golfer_warnings), rows`, one row per `from: screen` declared field in the device's order: `field, label, value (Option<f64>), display (Option<String>, with the lit letter where the profile declares one, as "2.2 L"), state (Read|NotRead|Corrected), sign_letters (Option<(String, String)>)`.
**Done when**: `tests/record.rs`, labelled `provisional: pasted copies`, feeds each capture and its committed `raw` boxes through `record_photo` under a scratch root and holds each stored field to `results/C0.json`'s `sets.fs-still.arms.raw.shots.<id>.cells`, by value. `read_corpus` lists all six, with no exclusion. A second feed of one photo answers `Duplicate` and opens nothing. An empty box list answers `Failed`, with the photo in `.app/failed/`, no entry and no shot. `IMG_2893`'s withheld `sidespin` shows `NotRead` on an unflagged card (M33's named case). **A timing line**: a release-profile feed with 500 entries in the store is recorded in findings, and if above 1 s on this host, `Duplicates` is scoped to the session, the lab's dedupe rule, and that is a finding.
**Verify**: the `-p app`, clippy, fmt and iOS lines.

### P7 — `correct_shot`: what a person's edit stores

**Goal**: C11, in one function both surfaces could share.
**Read**: C11; `crates/contracts/src/shot.rs:95-110` and the `Validate` impl; `crates/storage/src/corpus.rs:640-652,700-716` (a flagged shot is held out of auto-mishit, so review must clear it); `crates/storage/src/shot_store.rs:98-112`; P6's `card.rs`.
**Touch**: `crates/core/src/session.rs` (`correct_shot`), its tests; `crates/app/src/api/review.rs`, `crates/app/tests/review.rs`; the comment on `ShotProvenance::corrections` (it now says what an entry's value records).
**Done when**: tests pin each clause of C11: first-edit-only, restore removes the entry, `""` for a value that was not read, clearing allowed, `needs_review` cleared with empty edits, warnings kept, an invalid value refused with nothing written, a non-screen field refused by name, and `shot_analysis.json` rewritten to agree with the shot store. The corpus's `shot_needs_review` then reads false. A shot with a non-empty map still reads through frozen Python: one line of `.venv/bin/python -c` validating it through `golf_coach.contracts.shot.ShotData` (ADR-035 clause 4), run by hand and recorded in findings.
**Verify**: `cargo test -p golf-core -p app 2>&1 | grep -E '^test result|FAILED'`; the clippy and fmt lines.

### P8 — The lab's re-read keeps a person's correction

**Goal**: C12. Today a re-read would silently replace a person's value with the misread they corrected.
**Read**: `crates/lab/src/shot.rs:575-680,755-785`; `crates/lab/tests/reread.rs` for its fixtures; P7's findings.
**Touch**: `crates/lab/src/shot.rs` (`carry_corrections` beside `carry_impact_video`, its module doc); `crates/lab/tests/reread.rs`.
**Done when**: tests pin that a stale re-read and a `--force` re-read of a corrected shot keep the person's value and the map's entry, a field the person did not touch takes the new read, and a shot with an empty map moves no byte against today's output. `cargo test -p golf-lab` is otherwise unedited.
**Verify**: `cargo test -p golf-lab 2>&1 | grep -E '^test result|FAILED' | tail -30`; `cargo clippy -p golf-lab --all-targets 2>&1 | tail -2`; fmt.

### P9 — Facade: history and one shot's detail, as views

**Goal**: everything the History tab and detail screen show, as plain data.
**Read**: C2's `history.rs`, C23, rule 8; `crates/storage/src/bundle_store.rs:153-225`; `crates/storage/src/state.rs` (`shot_analysis.json` reader); `crates/contracts/src/shot_result.rs`; `crates/storage/src/corpus.rs` (doc) for `ExcludedSwing` and its reasons; `crates/contracts/src/mishit.rs`.
**Touch**: `crates/app/src/api/history.rs` (the two read functions), `crates/app/tests/history.rs`.
**Do**: `list_sessions` answers newest first, each session's date, count and rows: `swing_id, time, club, mode, target_shape, carry display, ball speed display, flagged, corrected, mishit, untracked (drill), photo_path (absolute)`. `shot_detail` answers the `ShotCard`, the measurements as `name, display, unit` in stored order, the unscored as `name, reason sentence`, the notes, and why the corpus left it out if it did. Display strings come from `pyfmt` and existing labels.
**Done when**: tests over P6's six captures pin order and counts, and a drill shot is listed and marked untracked. An unscored row carries the contract's sentence, a `None` measurement never reads as `0`, and an entry whose analysis file is missing still lists, with its reason.
**Verify**: the `-p app`, clippy and fmt lines.

### P10 — Facade: a shot's actions (club, intent, mishit, delete)

**Goal**: D10, C15, and the club repair every session needs when someone forgets to change clubs.
**Read**: `crates/storage/src/bundle_store.rs:300-400`; `crates/core/src/session.rs:472-560` (`intent_action`, `set_intent`); `crates/core/src/reports/flag_mishit.rs`; C15.
**Touch**: `crates/app/src/api/history.rs` (four writers), `crates/app/tests/shot_actions.rs`.
**Done when**: tests pin that `set_shot_club` and `set_shot_intent` re-analyse the entry and the corpus sees the change; a shape with no `shot_shaping`, and `shot_shaping` with no shape, are refused by `intent_action`'s rule; a drill leaves the aggregates; `set_shot_mishit(None|Confirmed|Cleared)` reaches the corpus's verdict; `delete_shot` removes the entry and its shot store record, and the photo can then be recorded afresh.
**Verify**: the `-p app`, clippy and fmt lines.

### P11 — Facade: the graded profile, as a view

**Goal**: the Profile tab's data, with every sentence Rust's.
**Read**: C23, rule 8; `crates/core/src/profile.rs`; `crates/core/src/reports/profile.rs` (whole, it is the display logic to reuse); `crates/contracts/src/strike.rs:591-700,947-1070,1243-1296`; `crates/analysis/src/club_profile.rs` (doc); `spec/vectors/profile/` listing; the program plan's §M38 P3 bullets.
**Touch**: `crates/app/src/api/profile.rs`; `crates/core/src/reports/profile.rs` (helpers `pub`, no text change); `crates/app/tests/profile.rs`.
**Do**: `ProfileView { player_id, clubs: Vec<ClubView>, player: PlayerView }`. A `ClubView` holds `club, n_shots`, `numbers` (carry, ball speed and the other per-club stats, with n and spread, from `build_bag_profile`), `topics` (`name, criterion, grade text or withheld text, unscored rows, and for a shape topic both shares and whether the on-line share is **projected**), `blend` (`Option`, its mean's text, what it averaged, what it **excluded** by name), and `strengths`, `weaknesses`. `PlayerView` is the same less `numbers` per club. No golfer data means an empty profile, not a refusal.
**Done when**: **the gate**: for each `spec/vectors/profile/hand/` case, every display string in the view appears, whitespace flattened, in `reports::profile::profile()`'s text for that case. So the view cannot say what the CLI does not. `golf-core profile`'s vectors are unedited and green. Tests pin a withheld grade below `RATE_MINIMUM_N`, a blend naming its exclusions, an absent blend, and the word "projected".
**Verify**: `cargo test -p golf-core -p app 2>&1 | grep -E '^test result|FAILED' | tail`; clippy; fmt.

### P12 — `import-export`'s value rule for a golfer and a bag

**Goal**: D6, so a phone's first export can reach a lab that knows the golfer.
**Read**: `crates/lab/src/import_export.rs:1-75` and its judging code; `crates/lab/tests/import_export.rs`; the M29 plan's C11 (`grep -n 'C11' docs/plans/m29-lab-port.md`) and its P31 finding 6; `crates/contracts/src/golfer.rs`; the `SUFFIX` of each store.
**Touch**: `crates/lab/src/import_export.rs` (rule and module doc), `crates/lab/tests/import_export.rs`, `docs/plans/m29-lab-port.md` (a dated line under C11, naming this plan's D6).
**Do**: a golfer file conflicting by bytes is read on both sides. The same `player_id` and handedness is **`kept`** as the lab's, with a line naming the fields that differ. A different handedness, or either side unreadable, stays a conflict. A bag conflicting by bytes is **`kept`**, with a line saying the phone's bag was not applied. Nothing else changes: `session.json`, a shot and a manifest keep their rules.
**Done when**: tests pin all four outcomes, and every existing test passes unedited except any that named a golfer's byte conflict, which moves to the new rule with a comment saying D6 moved it. Exit codes are unchanged.
**Verify**: `cargo test -p golf-lab --test import_export 2>&1 | tail -3`; `cargo clippy -p golf-lab --all-targets 2>&1 | tail -2`; fmt; docs truth.

### P13 — Facade: `export_sessions`, round-tripped through `import-export`

**Goal**: half of the milestone's exit, proven on the desk.
**Read**: C7, C16; `crates/lab/Cargo.toml`'s `zip` comment; `crates/lab/src/import_export.rs:364-380,826` (the time fields it reads), `Request`; P12's findings.
**Touch**: `crates/app/src/api/export.rs`, `crates/app/Cargo.toml` (`zip`, with the lab's exact pin and features, and `golf-lab` as a **dev-dependency**), `crates/app/tests/export_roundtrip.rs`.
**Done when**: a store built by `record_photo` over the six captures (`provisional: pasted copies`), exported and imported by `golf_lab::import_export` into empty scratch dirs, gives **`read_corpus` equal to the phone's**. Also: a photo keeps its modification time; `.app/` files are counted as ignored and copied nowhere; `state.json` is not in the zip; a re-import copies nothing; a later export with one more shot imports clean; a lab holding the same golfer with another `created_at` imports clean (P12); a correction after the first import prints `kept` and refuses nothing (C16's limit, pinned so it is seen). `Cargo.lock` gains nothing but `app`'s edges. `ort_reach` still passes, since a dev-dependency does not count.
**Verify**: the `-p app`, clippy, fmt, iOS and `ort_reach` lines.

### P14 — `golf-bay-report bars`: the session's verdict from an export

**Goal**: the bay session's arithmetic exists and is proven before there is a session.
**Read**: `docs/plans/m38-bay-thresholds.md` whole; C21; `crates/lab/src/import_export.rs`' `ZipArchive` use; P6's `reads.jsonl` line; `crates/screen/src/parser.rs`' `LIT_BLUE_RATIO` and `is_lit`.
**Touch**: `crates/app/src/bay.rs`, `crates/app/src/bin/golf_bay_report.rs`, `crates/app/tests/bay_bars.rs`.
**Do**: `golf-bay-report bars <export.zip|dir> [--bars F] [--json]`, with `F` defaulting to the repo's `docs/plans/m38-bay-thresholds.md`. Classify each cell by the file's definitions from the stored shots and `reads.jsonl`, judge T1 to T5 against limits parsed at run time, and print: the session's validity (`INVALID` under 30 camera photos or 2 clubs), each bar with its number and PASS or MISS, the verdict line by the file's three outcomes, every wrong and withheld cell by photo and field, per-club counts, **the distribution of box confidences and whether any cell's read depended on one (D8's evidence)**, and each sign letter's ratio against `LIT_BLUE_RATIO` with its margin.
**Done when**: a hand-worked fixture of ten photos, with its arithmetic in the test's comments, pins each bar's number and both sides of each limit. A wrong cell on a flagged photo does not count against T1, a blank is in no denominator, a failed read counts as twelve withheld and flagged, and the p95 is nearest-rank. Editing a limit in a scratch copy of the bars file changes the verdict, which proves there is no second copy.
**Verify**: the `-p app`, clippy, fmt and iOS lines.

### P15 — `golf-bay-report truth` and `compare`, controlled on M33's numbers

**Goal**: the two remaining verbs, each with a known answer.
**Read**: `spikes/vision-ocr/probe.py`'s `truth()` and `classify()` (`grep -n 'def truth\|def classify'`); `spikes/vision-ocr/truth_full_swing.json`, `photos.json`; `results/C0.json`'s `raw` arm; P13's findings.
**Touch**: `crates/app/src/bay.rs`, `crates/app/src/api/bay.rs` (`truth_report`), the bin, `crates/app/tests/bay_truth.rs`.
**Do**: `truth <export> --photos F --truth F`: photos matched by digest, else by the manifest's `original_filename` stem, because Photos may re-encode an AirDropped file (P30), with each match's route printed and a re-encoded copy labelled as such; truth turned into `ShotData`'s conventions as `truth()` does, comparing IEEE doubles. `compare <export> --sessions-dir D --player P`: `read_corpus` over the export and over `D`, equal or the first difference by path.
**Done when**: **the control**: P6's store over M33's committed `raw` boxes reads **71 of 72, 0 wrong, `sidespin` lowest at 5 of 6, 0 flagged**, M33's own numbers, labelled `provisional: pasted copies`. `compare` says equal on P13's round trip, and names the path when one value is changed in a scratch copy.
**Verify**: the `-p app`, clippy and fmt lines.

### P16 — The Flutter `app/` and its bridge, running on the Simulator

**Goal**: an empty app with the real facade behind it, and the commands every later phase uses.
**Read**: C1, C3, C4, C19, the spike section; P4's findings.
**Touch**: `app/` (new); root `.gitignore` as the template's own misses require; `app/tool/{sim_test.sh,codegen_check.sh}`; `crates/app/tests/lock_agreement.rs`.
**Do**:
1. From the repo root: `flutter_rust_bridge_codegen create golf_app --integration-backend native-assets --platforms ios --rust-crate-name golf_bridge --org dev.golfcoach --skip-fvm-install`, then `mv` its contents into `app/` beside `app/tool/`. Remove `.idea/`, `*.iml`, `test_driver/`, the template's `simple.rs` and its call in `lib/main.dart`.
2. `app/rust/Cargo.toml`: add the empty `[workspace]`, `app = { path = "../../crates/app" }`, keep `flutter_rust_bridge = "=2.13.0"`. `app/rust/rust-toolchain.toml`: C3's channel and three targets. `src/api/mod.rs` holds only the init function.
3. `app/flutter_rust_bridge.yaml` as C1. `flutter_rust_bridge_codegen generate`. If the codegen refuses a shape from P5 to P15, fix it in `crates/app` under rule 7.
4. `ios/Runner.xcodeproj/project.pbxproj`: all three `IPHONEOS_DEPLOYMENT_TARGET = 16.0;`, and `PRODUCT_BUNDLE_IDENTIFIER = dev.golfcoach.lab;`. `Info.plist`: `CFBundleDisplayName` `Golf Coach`, `NSCameraUsageDescription` "Photographs the launch monitor's screen to read each shot's numbers.", portrait only. `#include? "Signing.xcconfig"` in `Flutter/Debug.xcconfig` and `Release.xcconfig`, and `Signing.xcconfig` gitignored.
5. `sim_test.sh` and `codegen_check.sh` per the Verify table. **No script reads stdin.**
6. `integration_test/smoke_test.dart`: `about()` answers the three version keys through the bridge.
**Done when**: `sim_test.sh integration_test/smoke_test.dart` passes, `flutter analyze` is clean, `flutter test` passes, `codegen_check.sh` passes, and `lock_agreement` passes (if the locks disagree on a named crate, `cargo update -p <crate> --precise` in `app/rust`, as a finding). The root `Cargo.lock` is unchanged by this phase, and `git status` shows no generated build directory.
**Verify**: those; `cargo test -p app --test lock_agreement 2>&1 | tail -2`.

### P17 — `ScreenReader.swift`, held to M33's committed boxes on macOS

**Goal**: the reader that ships reproduces M33's measurement to the bit, before any Simulator differs from it.
**Read**: C5, C6; `spikes/vision-ocr/vision_ocr.swift:50-66,114-270,652-666`; `spikes/vision-ocr/log.md`'s "For M38" items 1 and 3.
**Touch**: `app/ios/Runner/ScreenReader.swift` (new, in the Xcode project by C4), `app/tool/reader_main.swift`, `app/tool/reader_check.sh`.
**Do**: `ScreenReader.read(path:) throws -> (json: String, msVision: Int)`. `reader_main.swift` runs it on each of the six captures and compares against `spikes/vision-ocr/boxes/C0/raw/<id>.json`'s `boxes`, each number by `==` after the JSON trip. It also checks a generated JPEG with orientation tag 6 (the spike's `storedForOrientation6`): the width and height swap and a known word lands upright. `reader_check.sh` compiles with `xcrun --sdk macosx swiftc` into `target/m38/`, and runs it.
**Done when**: 0 boxes differ on all six (labelled `provisional: pasted copies`), the orientation check passes, and a Simulator build still compiles the file. If the in-memory decode differs, C5's fallback and a finding. This Mac's macOS is the one M33 measured on, so a difference is the port's.
**Verify**: `app/tool/reader_check.sh`; `(cd app && flutter build ios --simulator --debug 2>&1 | tail -2)`.

### P18 — The six captures through iOS Vision and Rust, on the Simulator

**Goal**: the first read by iOS's Vision, with M33's bars beside it. It is the Simulator's framework, and not a device's.
**Read**: C4, rule 9, rule 11; P15's and P17's findings; the spike section's `AppDelegate` route.
**Touch**: `app/ios/Runner/AppDelegate.swift`, `app/ios/Runner/PhoneChannel.swift` (`documentsDir`, `readScreen`), `app/lib/src/phone/phone.dart`, `app/integration_test/vision_gate_test.dart`.
**Do**: the test reads `GOLF_REPO`, then per capture: `readScreen`, `record_photo`, under a temp root. Then it calls `truth_report`, prints the table, compares each box file with M33's committed one (count, text, and position within 1 px), and writes both to `target/m38/sim-c0.txt`.
**Done when**: the test passes with **T1 wrong = 0**, and the table is in findings with T2, T3 and T4 beside M33's macOS numbers, labelled `provisional: pasted copies` and "Simulator, not a device". **Wrong > 0 is rule 11's hard stop.** T2 to T4 are recorded and do not stop it (D1: the bay is the test).
**Verify**: `app/tool/sim_test.sh integration_test/vision_gate_test.dart`; `flutter analyze`.

### P19 — The app's shell: model, theme, tabs, first-run setup

**Goal**: the app opens to a real setup and four tabs.
**Read**: C7, C22, D7, D13, rule 8; P5's findings.
**Touch**: `app/lib/main.dart`, `lib/src/core/core_api.dart`, `lib/src/state/app_model.dart`, `lib/src/ui/theme.dart`, `lib/src/ui/shell.dart`, `lib/src/screens/setup_screen.dart`, `lib/src/screens/bag_screen.dart`, `app/test/{fakes.dart,setup_test.dart}`, `app/integration_test/setup_test.dart`.
**Do**: `RustLib.init`, root from `documentsDir`, `install_id` by Dart's `Random.secure` on first run. No golfer: setup (name, handedness, then bag slots with an optional loft). Then `NavigationBar` with Record, History, Profile, Settings, with placeholders until their phases. Theme: Material 3, dark by default from `state.appearance`, one seed colour, touch targets at least 56, tabular figures for numbers. A `now()` helper writes RFC 3339 with the offset, since Dart's own ISO string has none.
**Done when**: widget tests with fakes pin the first-run route, a refused name showing Rust's message, bag editing, and `now()` carrying an offset. The integration test creates a golfer and a bag through real Rust, and a relaunch of the model finds them.
**Verify**: analyze, format, `flutter test`, `app/tool/sim_test.sh integration_test/setup_test.dart`, `codegen_check.sh` if `api` moved.

### P20 — `PhotoSource.swift`: the camera, the library, a photo's time

**Goal**: D5 and C9, C10, as far as a Simulator can prove them.
**Read**: C4, C9, C10; P18's `PhoneChannel.swift`.
**Touch**: `app/ios/Runner/PhotoSource.swift`, `app/ios/Runner/PhotoTime.swift` (both in the Xcode project), `PhoneChannel.swift`, `app/lib/src/phone/phone.dart` (`Photo {path, capturedAt, source}`), `app/tool/reader_main.swift`, `app/integration_test/phone_test.dart`.
**Do**: `cameraAvailable`, `takePhoto` (C10's overlay, presented from the active window scene's root controller), `pickPhotos` (C10's picker, copied to `tmp/` keeping the extension), `share(path)`. `PhotoTime.capturedAt(url:)` per C9.
**Done when**: `reader_check.sh` also pins `PhotoTime` on generated JPEGs: with EXIF date and offset, with date only, and with neither. On the Simulator `cameraAvailable` is false and `documentsDir` answers. The whole target compiles. **The camera, picker and share sheet cannot be driven here, and that is stated as unproven until P30.**
**Verify**: `app/tool/reader_check.sh`, `app/tool/sim_test.sh integration_test/phone_test.dart`, analyze, `flutter test`.

### P21 — The Record tab

**Goal**: the screen used between swings.
**Read**: D5, D9, C13, C14, C15; P6's and P20's findings; the layout D13 approved: a header (golfer · club · mode, `Full Swing`, change), the last result's card, a large shutter, and "from library" beneath.
**Touch**: `lib/src/screens/record_screen.dart`, `lib/src/state/record_flow.dart`, `lib/src/widgets/shot_card.dart`, `lib/src/widgets/change_sheet.dart`, `app/test/record_test.dart`, `app/integration_test/record_test.dart`, with `TestPhone` per C22.
**Do**: the change sheet picks golfer, club (the bag's slots), mode, and shape under `shot_shaping`, each saved to `state`. `RecordFlow`: for each photo, `readScreen`, `record_photo`, `log_read` with `ms_total` measured to the card's first frame through `addPostFrameCallback`. The card shows rows, with `NotRead` highlighted. `Failed` says retake, `Duplicate` shows the held shot. No camera means the shutter is hidden, with the library alone. A batch shows progress. Until P22 a flagged result only marks the card.
**Done when**: widget tests pin each outcome, `NotRead` highlighted, the header following the state, and club and mode kept between shots. The integration test imports the captures by `TestPhone`, and each card shows the stored values, labelled `provisional: pasted copies`.
**Verify**: analyze, format, `flutter test`, `app/tool/sim_test.sh integration_test/record_test.dart`.

### P22 — The review screen

**Goal**: D9's forced review, and C11 from a person's side.
**Read**: D9, C11, C14, C15, rule 8; P7's findings; P21's flow.
**Touch**: `lib/src/screens/review_screen.dart`, `lib/src/state/record_flow.dart`, `app/test/review_test.dart`, `app/integration_test/review_test.dart`.
**Do**: the photo (`InteractiveViewer`, decoded with a sensible `cacheWidth`), then each row editable: a number field, a sign control showing `sign_letters` where the profile declares them, a clear control, `NotRead` highlighted, Rust's warnings above, and the per-shot intent override. Save calls `correct_shot`, and a refusal shows inline. **A flagged result pushes this with `PopScope(canPop: false)`**, leaving only by Save or Delete. A batch's flagged shots queue (C14). An unflagged card opens it on tap.
**Done when**: widget tests pin forced entry and no back-out, Save with no edits releasing it, a typed value reaching `correct_shot`, a refusal shown, a batch's queue, and an unflagged shot not forced. The integration test edits a real value, and the corrections map holds the as-read value on disk.
**Verify**: analyze, format, `flutter test`, `app/tool/sim_test.sh integration_test/review_test.dart`.

### P23 — Challenge mode

**Goal**: D11.
**Read**: D11, C15; `crates/contracts/src/intent.rs`; P21's flow and sheet.
**Touch**: `lib/src/state/challenge.dart`, `lib/src/screens/record_screen.dart`, `lib/src/widgets/change_sheet.dart`, `app/test/challenge_test.dart`.
**Do**: a challenge toggle in the sheet with shape ticks (at least one, all three by default), hidden under Drill and turning Drill off. The current call shows large on Record. An injected `Random` draws uniformly over the ticks. A recorded shot takes `{shot_shaping, call}` and draws the next. A failed read or duplicate keeps the call. Off restores the prior mode.
**Done when**: widget tests with named draws pin each clause, and one test through `RustCore` shows a challenge shot feeding its shape topic in `graded_profile`.
**Verify**: analyze, format, `flutter test`.

### P24 — The History tab

**Goal**: every shot, including what never counts.
**Read**: P9's findings, rule 8; `lib/src/widgets/shot_card.dart`.
**Touch**: `lib/src/screens/history_screen.dart`, `app/test/history_test.dart`.
**Do**: the current golfer's sessions, newest first, by date, each expanding to rows with a thumbnail, club, carry, ball speed, and marks for flagged, corrected, mishit and drill. Empty state. Tap opens detail (P25). Refreshes on tab entry and after a record.
**Done when**: widget tests pin order, each mark, the empty state, and a not-read carry showing as not read rather than 0.
**Verify**: analyze, format, `flutter test`.

### P25 — The shot detail screen and its actions

**Goal**: D10 and the shot's repairs.
**Read**: P9's and P10's findings, C15; P22's screen, reused for edit.
**Touch**: `lib/src/screens/shot_detail_screen.dart`, `app/test/shot_detail_test.dart`, `app/integration_test/shot_actions_test.dart`.
**Do**: photo, card, measurements, unscored with their sentences, notes, why it is outside the corpus if so. Actions: edit (review screen, not forced), club, intent, mishit (automatic, confirmed, cleared), delete with an in-app confirmation.
**Done when**: widget tests pin each action's call and its refusal shown. The integration test changes a club, confirms a mishit and deletes a shot through real Rust, then finds History and the corpus agreeing.
**Verify**: analyze, format, `flutter test`, `app/tool/sim_test.sh integration_test/shot_actions_test.dart`.

### P26 — The Profile tab

**Goal**: the program plan's P3: the point of the product.
**Read**: P11's findings, rule 8; the program plan's §M38 P3 bullets; ADR-034 §5.5 and §5.7.
**Touch**: `lib/src/screens/profile_screen.dart`, `lib/src/widgets/{topic_tile,blend_card,finding_list}.dart`, `app/test/profile_test.dart`, `app/integration_test/profile_test.dart`.
**Do**: a selector of each graded club, in bag order, then Player. For a club: blend, numbers, topics with the grade beside the list, both shares on a shape topic with the projected label, withheld text where Rust gives it, strengths and weaknesses, and a note where the list is empty rather than a fake strength. A club in the bag with no shots is absent, and the selector says so.
**Done when**: widget tests with `ProfileView` fixtures pin: both shares shown, "projected" shown, a withheld grade showing Rust's text and no number, excluded topics named, no blend meaning no blend card and never 0, and an empty list saying so. The integration test renders the six captures' profile through real Rust. With n=6 most grades are withheld, and that is the right answer on them.
**Verify**: analyze, format, `flutter test`, `app/tool/sim_test.sh integration_test/profile_test.dart`.

### P27 — The Settings tab: export and share

**Goal**: how data leaves the phone.
**Read**: C7, C16, C20, D13; P13's findings.
**Touch**: `lib/src/screens/settings_screen.dart`, `app/test/settings_test.dart`, `app/integration_test/export_test.dart`.
**Do**: golfers (switch, add), bag (P19's screen), appearance, **Export** (runs `export_sessions`, then `share`, recording `last_export_at`), the last export and shots since, a plain warning that deleting the app deletes the shots, and About (`about()`).
**Done when**: widget tests pin each row and the export's call order. The integration test makes a real zip under `target/m38/` with the shots in it. The share sheet stays unproven until P30.
**Verify**: analyze, format, `flutter test`, `app/tool/sim_test.sh integration_test/export_test.dart`.

### P28 — The whole flow on the Simulator: desk-complete

**Goal**: everything but a camera and a bay, end to end, and the workspace still green.
**Read**: every finding addressed to P28; the Exit below; P13's and P15's findings.
**Touch**: `app/integration_test/whole_flow_test.dart`, `app/tool/whole_flow.sh`, `ROADMAP.md` (NEXT ACTION: desk-complete, waiting on the iPhone, P29).
**Do**: the test drives the widgets with `TestPhone` from a fresh root: setup, club, import the six captures, review anything flagged, correct one value, History, Profile, Export, and copies the zip to `target/m38/whole-flow.zip`. The script then runs, on the host: `golf-bay-report truth` on it, `golf-lab import-export` into empty scratch dirs, and `golf-bay-report compare`.
**Done when**: `whole_flow.sh` exits 0: corpus equal, wrong 0 (`provisional: pasted copies`, Simulator), with the table in findings. The workspace has 0 failures, `crates/app` has 0 clippy warnings, and docs truth passes.
**Verify**: `app/tool/whole_flow.sh`; the workspace `cargo test` line; clippy; fmt; docs truth; analyze.

### P29 — 🧍 The iPhone: paired, signed, launched with no cable

**A `phase-runner` reports `BLOCKED: P29 is attended: /clear, then /next-phase with the iPhone at the Mac` and stops.**
**Goal**: the app runs on the real device, on its own.
**You do**: plug the iPhone in and tap Trust. Settings → Privacy & Security → Developer Mode, on (it restarts). Xcode → Settings → Accounts → add your Apple ID.
**The session does**: reads the device with `xcrun devicectl list devices` and records its model and iOS (D2: if below 16, rule 11's `BLOCKED`). Finds the Personal Team id and writes `app/ios/Flutter/Signing.xcconfig` per C19. Runs `flutter build ios --release` and then `flutter run --release -d <id>`. **You** then, on the phone: Settings → General → VPN & Device Management → trust your developer profile, and launch again.
**If Flutter reports a signing error**: open `app/ios/Runner.xcworkspace` in Xcode, select the Runner target → Signing & Capabilities → Automatically manage signing, pick the Personal Team, and Run once. Then move any `DEVELOPMENT_TEAM` that wrote into `project.pbxproj` back out to `Signing.xcconfig`, so no team id is committed.
**Done when**: unplugged, the app opens from the home screen, a golfer can be created, and all four tabs render. `cat app/ios/Flutter/Signing.xcconfig` is ignored by git (`git check-ignore`). Findings record the device, the iOS version and the install time, because free signing lasts 7 days.
**Verify**: `xcrun devicectl list devices | tail -3`; `git check-ignore app/ios/Flutter/Signing.xcconfig`; your confirmation.

### P30 — 🧍 Dry run on the iPhone: the six captures, the camera, real HEICs

**A `phase-runner` reports `BLOCKED: P30 is attended: /clear, then /next-phase with the iPhone at the Mac` and stops.**
**Goal**: everything P20 and P27 could not prove, and the first reads by iOS's Vision on a real device, without a trip to the bay.
**You do**, with the session reading each result:
1. AirDrop `spikes/vision-ocr/captures/{IMG_2889,IMG_2891,IMG_2892,IMG_2893,IMG_2894,2026-10-07-1}.jpg` to the phone, and import them through "from library".
2. Show each capture full-screen on the Mac, and photograph it with the in-app camera, holding the phone in **landscape** as at the bay. A photo of a monitor is a rehearsal and is not a bay photo.
3. Take two of those with the stock Camera in its default HEIC format, and import them through the library.
4. Settings → Export, and share the zip to the Mac by AirDrop, to `target/m38/device-dry-run.zip`.
**The session does**: `golf-bay-report truth` on the six (step 2's and 3's match no capture and are ignored, and a capture Photos re-encoded is labelled so by the verb), and `golf-bay-report bars` for the sign-letter table and latency over everything.
**Done when**: the camera returned a read without a confirm step, a landscape photo read upright, HEIC bytes are stored as `.heic`, read, and show in History, the share sheet delivered the zip, and the table is in findings (`provisional: pasted copies`, device) beside P18's and M33's. **A wrong cell on an unflagged shot, or a lit letter on the wrong side of `LIT_BLUE_RATIO`, is rule 11's hard stop.** Findings record the smallest lit margins on HEIC, which M33 could not measure, and the latency.
**Verify**: the two report commands above.

### P31 — 🧍 Before the bay: re-sign and the run sheet. Then you go to the bay

**A `phase-runner` reports `BLOCKED: P31 is attended: /clear, then /next-phase with the iPhone at the Mac` and stops.**
**Goal**: a fresh install, and a sheet you can follow alone.
**The session does**: `flutter run --release -d <id>` (a reinstall keeps the data, C20), records the install date, and prints the run sheet below in its reply.
**The run sheet** (it is yours at the bay, and no laptop goes with you):
1. Install date within 7 days of the session. Phone charged.
2. In the app: your golfer, your bag holds at least 2 clubs you will hit. Settings → Appearance as you like it.
3. Pick the club, mode Fundamentals, and leave challenge off for now.
4. Full Swing on its grid screen. After each shot, tap the shutter, **filling the frame with the tiles**, in landscape. Read the card. If it forces review, compare it with the screen and fix it there.
5. **At least 30 photos across at least 2 clubs, all with the in-app camera.** Change club in the header.
6. If a photo fails, retake it. The failure is logged and counts, and that is the point.
7. Afterwards, on the app's Profile tab: confirm the per-club profile renders. That is half the exit, so note it.
8. **Before exporting, review every shot** in History, photo beside values: correct every wrong value, and **type in every value marked not read**, leaving blank only what the screen itself left blank. This is the truth the report trusts.
9. Settings → Export. AirDrop the zip to the Mac as `data/raw/phone-exports/<session-date>.zip`.
**Done when**: the install is fresh and the run sheet is in the reply. **The session at the bay happens after this phase.**
**Verify**: the install date in findings.

### P32 — 🧍 The session's export: the verdict, the exit's corpus check, and your calls on it

**A `phase-runner` reports `BLOCKED: P32 is attended: /clear, then /next-phase once the bay session's zip is in data/raw/phone-exports/` and stops.** So does a session that finds no zip there newer than P31's install.
**Goal**: the measured verdict, and the milestone's second exit clause.
**Read**: `docs/plans/m38-bay-thresholds.md`; C21; P30's findings.
**Touch**: `docs/plans/m38-bay-results.md` (new; tier AS-BUILT), and this plan's findings.
**Do**: `golf-bay-report bars` on the zip. `golf-lab import-export` into empty scratch dirs, **not** `data/processed` (rule 6). `golf-bay-report compare` for the golfer. The results file holds: the bars table, the verdict line, each wrong and withheld cell, per-club counts, latency, the success and review rates, lit margins, confidences, and the user's report of step 7.
**Then answer D8 by evidence**: did a box's confidence ever separate a wrong cell from a right one? Say so in the results, with a recommendation.
**Your calls, with the results in front of you**, each recorded under the decisions as D16 and on: **`W_OCR`**: leave it, or change it as its own later change under `SCREEN_PARSER_VERSION` (not part of M38, by rule 4). **On a latency-only miss**: does M38 close on it, or does a named follow-up come first?
**Done when**: the results file exists, `compare` says equal, your calls are recorded, and findings state, for P33: `CONDITION MET` if any of T1 to T4 missed and `CONDITION NOT MET` otherwise. A session report saying `INVALID` stops as `BLOCKED: too few photos or clubs: another session is needed`.
**Verify**: the three commands; `.venv/bin/python -m pytest tests/test_docs_truth.py 2>&1 | tail -3`.

### P33 — VisionKit's scanner route, if P32 missed T1 to T4

**Condition**: P32's finding. **Not met: read it, tick "not needed", and stop.**
**Goal if met**: D12's second arm, as a route and nothing else.
**Read**: C10; P20's `PhotoSource.swift`; `spikes/vision-ocr/log.md`'s "The rectifier stand-in".
**Touch**: `app/ios/Runner/PhotoSource.swift` (`VNDocumentCameraViewController`, device only), `PhoneChannel.swift`, `lib/src/phone/phone.dart`, `lib/src/screens/settings_screen.dart` (a "Use scanner" diagnostic toggle, off by default), `record_flow.dart` (`source: scanner`), `app/test/`.
**Done when**: with the toggle on, the shutter calls the scanner, the rectified page's JPEG goes through the same `readScreen`, and `reads.jsonl` says `scanner`. It compiles, and widget tests pass. It is unproven until P34.
**Verify**: analyze, format, `flutter test`, `(cd app && flutter build ios --simulator --debug 2>&1 | tail -2)`.

### P34 — 🧍 The scanner measured at the bay, if P33 ran, and your decision

**Condition**: P33 built it. **Not met: tick "not needed" and stop, with no person needed.** Met, a `phase-runner` reports `BLOCKED: P34 is attended` and stops.
**You do**: install as in P31. At the bay, photograph at least 10 shots **twice each**, the raw route then the scanner, reviewing as in P31's step 8. Export, and place the zip as in P31.
**The session does**: splits `golf-bay-report bars` by `source` and writes both columns into `m38-bay-results.md`, noting that the scanner must get the whole monitor in frame, which costs the tiles pixels, as M33 predicted.
**Done when**: both columns are in the results. **Then this is the plan's one question for you, as D14 promises**: keep raw only, ship the scanner as the default, or keep both with a toggle. Bring it with the numbers and a recommendation, and record the answer under the decisions.
**Verify**: the report commands.

### P35 — Docs

**Goal**: the docs tell the truth about what runs.
**Read**: `docs/README.md` (map), `docs/ARCHITECTURE.md` §1, §2 and §4, the M33 and M42 addenda's form in ADR-034, `m38-bay-results.md`, and this plan's findings by grep.
**Touch**:
- **ADR-034**: an M38 addendum: the verdict, what was built, D3, D4 and D6, **clause 7 amended per D12's outcome**, session ids (C8), what `W_OCR` showed, what stayed unmeasured. Update any self-count of addenda (the docs-truth test holds it).
- **ADR-030**: an addendum on the bridge (C1, C3), and that CocoaPods was not needed. **ADR-014**: an addendum on C11 and C12. **ADR-024**: an addendum if `C8`'s no-`session.json` reads as a change to its session model.
- **`docs/ARCHITECTURE.md`** §1 (`app/`, `golf-bay-report`, `app/tool/`), §2 (`crates/app` and its edges), §4 (the phone's root, `.app/`).
- **`CLAUDE.md`**: the commands block, the Rust row (`crates/app`, and `app/rust` as not a workspace member), the intro's list of what is built, with no count that is not pinned.
- **`app/README.md`** (new; tier AS-BUILT): how to build, test, sign and run, and C20's three warnings. **`docs/README.md`**: the map's new rows. **`data/README.md`**: `data/raw/phone-exports/`.
- **The program plan's §M38**: what happened to each P and each *(M33)* item. **ROADMAP §M38, §M39, §M40**: M42's clip route and HD Golf on the phone named as follow-ups.
**Done when**: docs truth passes, and `grep -n 'Fourteen\|fourteen' CLAUDE.md docs/ARCHITECTURE.md` finds no stale crate count.
**Verify**: `.venv/bin/python -m pytest tests/test_docs_truth.py 2>&1 | tail -3`; `cargo test -p ocr --test ort_reach 2>&1 | tail -2`.

### P36 — Close: verify, WORKLOG, ROADMAP, memory, and the close commit

**Goal**: M38 closed, on the decisions P32 and P34 recorded. If they left it open, the commit and the docs say so and why.
**Do**:
1. Every Verify line once: the workspace, clippy on `crates/app`, fmt, docs truth, `ort_reach`, `reader_check`, analyze, `flutter test`, `whole_flow.sh`.
2. `WORKLOG.md`'s entry on top; `ROADMAP.md`'s status table and NEXT ACTION; M33 **stays open** and says so. M40 now waits on nothing from M38 if closed.
3. Memory: the toolchain's paths, that the iPhone is paired, and that 7-day signing means a re-run before a session.
4. `git status`, read whole: no `Signing.xcconfig`, no `target/`, no `data/`. Then **one commit on `main`**, not pushed, ending with the attribution lines the conversation's system reminder gives.
**Done when**: the commit exists, the tree is clean, and every row above is ticked.
**Verify**: `git log --oneline -3`; `git status --short | wc -l`.

---

## Exit

The program plan's two clauses, and how each is shown:

- **At the bay, with no laptop, photos become stored, validated shots and the per-club profile renders.** P31's run sheet, with `m38-bay-results.md` recording the session and step 7's confirmation.
- **The lab's Rust reader agrees with the phone on the exported data.** P32's `golf-bay-report compare`, over `read_corpus` on the export and on what `golf-lab import-export` wrote. Frozen Python's reader is not the reference (it excludes photo entries by design).

**The bars do not define the exit.** A miss triggers P33 and P34 or the user's call (D14), and the verdict is written down either way.

**What M38 leaves open**: M33 itself; clip import; HD Golf on the phone; `W_OCR`, per the user's call at P32; C16's limit on a correction after import.

---

## Phase findings

*(Appended as phases close, newest last, as `### Pn (date)`. Address anything a later phase must know to it by number.)*

### P0 (2026-11-04)

1. **The plan and the bars went in as approved**, with the bars in `m38-bay-thresholds.md` before any phone has photographed a screen. One docs-only commit on `main`, not pushed (D15).
2. **The scratchpad's spike is deleted**, so it is not there to copy from: the Flutter unpack and the `frbapp` project. **For P1**: install from C3's URL and digest. `~/.rustup` already holds 1.99.1 and the iOS targets, from the spike, so step 4 should find them present. **For P16**: the spike's working pieces are all stated in C1, C3, C4 and D2, and "What the planning spike measured".
