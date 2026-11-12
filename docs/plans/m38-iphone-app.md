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
| **P1** | The toolchain: Flutter, the bridge's codegen, the iOS targets | ✅ Done *(2026-11-04)* |
| **P2** | `cargo test --workspace` green on macOS | ✅ Done *(2026-11-04)* |
| **P3** | The photo entry's opener moves from `golf-lab` down to `golf-core` | ✅ Done *(2026-11-04)* |
| **P4** | `crates/app` exists, builds for iOS, and is held off `ort` | ✅ Done *(2026-11-04)* |
| **P5** | Facade: golfers, bags and the app's own state | ✅ Done *(2026-11-04)* |
| **P6** | Facade: `record_photo`, boxes in and a stored, analysed shot out | ✅ Done *(2026-11-04)* |
| **P7** | `correct_shot`: what a person's edit stores | ✅ Done *(2026-11-04)* |
| **P8** | The lab's re-read keeps a person's correction | ✅ Done *(2026-11-04)* |
| **P9** | Facade: history and one shot's detail, as views | ✅ Done *(2026-11-04)* |
| **P10** | Facade: a shot's actions (club, intent, mishit, delete) | ✅ Done *(2026-11-05)* |
| **P11** | Facade: the graded profile, as a view | ✅ Done *(2026-11-05)* |
| **P12** | `import-export`'s value rule for a golfer and a bag | ✅ Done *(2026-11-05)* |
| **P13** | Facade: `export_sessions`, round-tripped through `import-export` | ✅ Done *(2026-11-05)* |
| **P14** | `golf-bay-report bars`: the session's verdict from an export | ✅ Done *(2026-11-05)* |
| **P15** | `golf-bay-report truth` and `compare`, controlled on M33's numbers | ✅ Done *(2026-11-05)* |
| **P16** | The Flutter `app/` and its bridge, running on the Simulator | ✅ Done *(2026-11-05)* |
| **P17** | `ScreenReader.swift`, held to M33's committed boxes on macOS | ✅ Done *(2026-11-05)* |
| **P18** | The six captures through iOS Vision and Rust, on the Simulator | ✅ Done *(2026-11-05)* |
| **P19** | The app's shell: model, theme, tabs, first-run setup | ✅ Done *(2026-11-05)* |
| **P20** | `PhotoSource.swift`: the camera, the library, a photo's time | ✅ Done *(2026-11-05)* |
| **P21** | The Record tab | ✅ Done *(2026-11-05)* |
| **P22** | The review screen | ✅ Done *(2026-11-05)* |
| **P23** | Challenge mode | ✅ Done *(2026-11-05)* |
| **P24** | The History tab | ✅ Done *(2026-11-05)* |
| **P25** | The shot detail screen and its actions | ✅ Done *(2026-11-06)* |
| **P26** | The Profile tab | ✅ Done *(2026-11-06)* |
| **P27** | The Settings tab: export and share | ✅ Done *(2026-11-06)* |
| **P28** | The whole flow on the Simulator: desk-complete | ✅ Done *(2026-11-06)* |
| **P29** | 🧍 The iPhone: paired, signed, launched with no cable | ✅ Done *(2026-11-08)* |
| **P30** | 🧍 Dry run on the iPhone: the six captures, the camera, real HEICs | ✅ Done *(2026-11-08)* — camera, landscape and share sheet waived by the user (D16), carried to P31 |
| **P30b** | The app's event and error log, for the bay (D16) | ✅ Done *(2026-11-08)* |
| **P30c** | `golf-bay-report events`: what the log says about a session (D16) | ✅ Done *(2026-11-08)* |
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
2. **Only P0 and P36 commit** (D15), **plus the user's checkpoint of 2026-11-10, which committed P1–P30c**
   (D15's amendment). **All other work lives uncommitted in the tree, for weeks across the
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
  **Amended 2026-11-10 by the user: a checkpoint commit, pushed.** P1–P30c went onto `main` as one
  checkpoint commit before the bay, and `main` was pushed with it. The reason is that M46 (raised as M43, the impact drill) can then start
  on top of a known build without mixing into M38's diff, and the app taken to the bay is that build. So
  three commits now, not two: P0, the checkpoint, and P36's close holding P31 onward. Rule 2's ban on
  `checkout`, `restore`, `clean`, `stash` and `reset` stands.
- **D16 — The bay session records everything, added 2026-11-08 at the user's direction**: *"Lets be sure to collect
  lots of data so when I actually take it to the sim we can see what went well and what did not."* So P30b logs what
  the person does in the app and every error inside it, into the export, and P30c reads that log. Neither judges: no
  bar moves (rule 5), and the bars stay the verdict. The same day, the user waived P30's hands-on steps: *"just go
  ahead and do what you can with what you got."*

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
**Answered by the user (2026-11-04), after this phase's `BLOCKED`**: `finite`. `set_bag_slot` refuses a non-finite `loft_deg` (`NaN`, `inf`, `-inf`) in the facade's own words, and any real number stands, `340`, `-5` and `3.4` included. No bound goes into `contracts` and no range goes into `crates/app`. This replaces the Done line's "a loft refused by `Validate` surfacing its message": the pin is now a non-finite loft refused with the facade's message. The rest of the Done line stands. Why, and what a loft does today, are in this phase's entry under "Phase findings".
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

**Decision (the user, 2026-11-05, on P10's question): fix the reused shot number, first, in this phase.** Deleting a session's newest shot gives its number to the next photo, and if the lab had imported the deleted shot, `golf-lab import-export` refuses that export and every later one. Before the export work, record deleted numbers in `.app/` and give `NewPhotoEntry` a floor, so a number is never reused (P10's estimate: about 40 lines across `crates/core`, `crates/lab` and `record.rs`; those are in this phase's Touch). Pin the fix: export, import, delete, another photo under the same date, and the next export imports. This replaces the recorded limit in P10's findings, so P13 does not pin the refusal and P31's run sheet does not need that warning.

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

**Decision (the user, 2026-11-05, on P11's finding 5): the Profile view gains a mean.** Add a `center` field, held to `golf-core club-profile`'s text, so the tab can show a club's carry once it clears its floor (P11's estimate: about 20 lines; `crates/app/src/api/profile.rs`, `crates/app/src/profile.rs` and `crates/app/tests/profile.rs` are in this phase's Touch). Not built yet: build it here, before the tab that shows it, unless an earlier phase already has.

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

### P30b — The app's event and error log, for the bay

**Added 2026-11-08 (D16).**
**Goal**: what the person did at the bay, and everything that went wrong inside the app, travels in the export beside
`.app/reads.jsonl`, so P32 can say what went well and what did not, and not only what was read.
**Read**: C7, C16 and rules 7 to 9; P30 findings 12 and 13; `crates/app/src/api/record.rs` (`log_read` and `ReadLog`,
the writer to reuse, not copy), `crates/app/src/export.rs` (the `.app/` file set), `app/lib/main.dart`,
`app/lib/src/core/core_api.dart`, and the model's action points (`grep -n 'Future<' app/lib/src/model/*.dart`).
**Touch**: `log_event(root, AppEvent)` beside `log_read` in `crates/app/src/api/record.rs`, writing
`.app/events.jsonl` through the same appender; `crates/app/src/export.rs`, which adds that file; the codegen;
`core_api.dart` (`logEvent` on `GolfCore`, `RustCore` and the fakes); `main.dart`, where `FlutterError.onError` and
`PlatformDispatcher.instance.onError` log an `error`; and the model, at each action below.
**The event**: `AppEvent { at, kind, detail }`. `at` is RFC 3339 with an offset. `kind` is a plain `EventKind`:
`app_start`, `club_changed`, `mode_changed`, `camera_opened`, `camera_cancelled`, `library_picked`,
`review_opened`, `review_saved`, `shot_deleted`, `club_set`, `intent_set`, `mishit_set`, `export_shared` and
`error`. `detail` is short text for a person, naming the subject (`7i`, `3 photos`, `2 edits`, `completed false`, or an
error's message and its stack's first five lines), and Rust stores it as given. Each is one JSON line, appended as
`reads.jsonl` is. **A failed write never stops what the person was doing**, and the `error` handlers never throw.
Rule 8 holds: an event is not golf, and Dart computes nothing in one.
**Done when**: a widget test per kind shows the action writing it, through the fake. A thrown exception in a widget test
writes `error`. `cargo test -p app` holds a line's shape, and holds the export carrying `.app/events.jsonl`.
`golf-lab import-export` still ignores it, as a dotted name (C7). `codegen_check.sh` passes, and nothing in `spec/`
moves.
**Verify**: `cargo test -p app`, clippy, fmt, `codegen_check.sh`, `flutter analyze`, `dart format`, `flutter test`, and
`app/tool/whole_flow.sh`, whose zip must now hold `.app/events.jsonl`. **Not on the phone**: `device_test.sh` costs a
re-trust (P30 finding 10).

### P30c — `golf-bay-report events`: what the log says about a session

**Added 2026-11-08 (D16).**
**Goal**: P32 reads the event log in one command, beside `bars`.
**Read**: P30b's findings; `crates/app/src/bin/golf_bay_report.rs` and the `bay` module it runs, for `--since` and
the way a directory or a zip is opened.
**Touch**: the `bay` module, and the bin's verb table.
**It prints**: counts by kind; every `error`, verbatim, with its time; the time between consecutive recorded shots
(p50, p95, max) from `reads.jsonl`; review time, `review_opened` to the next `review_saved` (p50, p95, max); camera
cancels; and the photos that failed beside the retake that followed, if any. It takes `--since` as `bars` does.
**It judges nothing**: no bar is added and `m38-bay-thresholds.md` is not touched (rule 5).
**Done when**: a fixture export with a known log prints known numbers (`cargo test -p app`), and `whole_flow.sh`'s zip
runs through it with exit 0.
**Verify**: `cargo test -p app`, clippy, fmt.

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

### P1 (2026-11-04)

1. **The toolchain is in place, and `. app/tool/env.sh` is the one line that reaches it.** Checked with `env -i` under `/bin/sh`, from a bare system `PATH`: Flutter 3.47.9 (Dart 3.13.7), codegen 2.13.0, both iOS targets on 1.99.1, one `iPhone 17`. `flutter doctor` complains only of Android and CocoaPods. The zip downloaded whole on the first try, 2,268,740,583 bytes, and its digest matched C3's.
2. **The zip's one top-level directory is `flutter/`, and it was moved so that `flutter-3.47.9/` is itself the SDK's root**: the binary is `flutter-3.47.9/bin/flutter`, and `env.sh` relies on that.
3. **`stable` is also rustc 1.99.1 here and already holds both iOS targets**, and the repo root has no `rust-toolchain.toml`. So the iOS lines run from the root use `stable`, not the pin. **For P4**: `cargo build -p app --target aarch64-apple-ios` should find its target without another `rustup target add`. `app/rust` alone is pinned, once P16 writes its `rust-toolchain.toml`.
4. **P0's commit left the docs-truth test red, and this phase fixed it outside its Touch line.** The test counts `git ls-files`, so `docs/README.md`'s 88 stayed true until P0's two new plan files were committed, and HEAD then tracked 90 (HEAD~1 tracks 88). So the Verify table's "91 pass today" did not hold at HEAD: it was 90 and 1 failed. Three numbers in the map's first sentence moved (88 to 90, 73 to 75, 12 to 14), and 91 pass again. No test changed.
5. **For P35 and P36**: the same trap waits at the close commit. A new `.md` file does not count until it is staged, so docs truth stays green all through the uncommitted phases and goes red the moment P36 commits. `app/README.md` is one, `m38-bay-results.md` another, and **for P16**: `flutter create` emits a `README.md` of its own, and usually one under `ios/Runner/Assets.xcassets/LaunchImage.imageset/`, so keep or delete them on purpose. P36: `git add` first, then run docs truth, set the map's numbers (its sentence also needs a clause for `app/`), and only then commit.

### P2 (2026-11-04)

1. **`cargo test --workspace` is green on this Mac: 1,765 passed, 0 failed, 26 ignored, over 119 result lines**, from 1,761, 5 and 26 at the phase's start. The difference is the phase: three `capture` tests off this OS, one `Unsupported` test and one store test on. `cargo fmt --check` is clean, clippy still prints 28 `warning` lines with none in a file this phase touched, and docs truth passes 91. **For P28 and P36**: that is the baseline, so a failure is news, and an ignored count other than 26 means a gate moved. No `crates/ocr` file changed, so the HEIC gates skip as they did.
2. **`import_photos`' failure was a real bug, and it was not about the photo's `./`.** Its stderr: `could not store the shot read from …: No such file or directory`. The test's `--shots-dir {root}/shots/.` names a directory that does not exist yet. `mkdir("shots/.")` answers `ENOENT` (checked at the shell), and `create_dir_all`'s retry goes to `Path::parent`, which treats the `.` as absent, so `shots` is never made. Windows drops the `.` before the filesystem sees it and Python's `Path` drops it on construction, so neither met this. It reached any `golf-lab` run given such a directory: `import-photos`, `import-video`, `analyze`, `reanalyze`. Reasoned, and not run off this Mac: Linux should fail the same way, since this is `mkdir`'s rule and not Apple's.
3. **The fix is `ShotOptions::store` in `crates/lab/src/shot.rs`**, which both shot writers (`shot_for`, `import_photo_with`) now open the store through, spelled as `Entries::check` already spells `sessions_dir`. So the test keeps every assertion. It is not behind `cfg`, because nothing prints that path: the table's store line was spelled already, so no spelling on Windows moves, only what is handed to the OS. `shot::tests::a_shots_dir_ending_in_a_dot_is_made_though_it_does_not_exist_yet` is the new pin, and the import test's failure message now carries its stderr, whose absence hid this.
4. **`ShotStore::put` itself is unchanged, and `GolferStore` has the same hazard** (`crates/storage/src/golfer_store.rs:166`, `create_dir_all(&self.root)`). Neither was fixed: no test reaches them with such a root, and this phase's Touch is the lab. Only a root whose *last* component is `.` trips it, since a `.` mid-path is harmless. **For P5 and P6**: a root built by joining onto `<Documents>/processed` never ends that way, so `crates/app` needs nothing, as long as Dart does not pass a path ending in `/.`.
5. **`flight` keeps more than the plan's line asked.** Flattening alone would have stopped checking the header's layout and the two-space indent on Windows too. So the text stays exact through `No parsed shots in`, which no path length can move, and only the following word is asked of the flattened text. What is given up is where the line wraps, which is the path's length and no behaviour.
6. **The Windows side was read and not run.** The three `capture` tests are unchanged under `#[cfg(windows)]`, and were neither compiled nor run here. The spike's reading of the `capture` and `flight` failures held.

### P3 (2026-11-04)

1. **The three moved, and `crates/lab` keeps only their callers.** `golf_core::session` now holds `open_photo_entry`, `session_holding` and `content_type_of` (the lab's `content_type`, renamed as this plan names it), and step 2 of `crates/lab/src/import.rs`' module doc moved with them, leaving a pointer. `Entries::holding` and `Entries::open` stay as one-call wrappers. `golf-lab` and `golf-core`: 356 passed, 0 failed, 16 ignored over 37 result lines, with no existing test edited. `Cargo.lock` did not move, `cargo fmt --check` is clean, and clippy names neither touched source file.
2. **The arguments are a struct, `NewPhotoEntry`**, because eight values plus the store and `now` would trip clippy's `too_many_arguments`: `open_photo_entry(&store, &NewPhotoEntry { session_id, digest, bytes, original_filename, modified, player_id, club, intent }, now) -> io::Result<SwingManifest>`. **For P6**: `modified` is a `SystemTime` and `captured_at` is RFC 3339 text, and no converter exists: `Timestamp::parse`, then `SystemTime::UNIX_EPOCH` plus `utc_micros()`. `original_filename` must keep its extension, since its suffix picks both the content type and the stored file's name (`storage::manifest::content_filename`). The digest is the caller's, as `session_holding` needs it first.
3. **Two differences from the lab's code, neither of which anything prints or tests.** The lab's stat now happens before `.incoming/` is made rather than after, so a stat failing straight after a successful `fs::read` leaves no empty `.incoming/` behind, with the same error line. And `.incoming/` is under `store.root()` rather than the `spelled` sessions directory: the same directory, since a `.` mid-path is harmless (P2 finding 4) and the temporary path is never printed or stored.
4. **For P6 and P21: `record_photo` must run one at a time.** `next_swing_id` is taken outside the store's lock (its own doc says so), and that lock is per store instance. So two concurrent opens in one session can take one swing id, and the second `place` then replaces the first's photo. Reasoned from `crates/storage/src/bundle_store.rs` and not run. The lab is serial, so it never met this, and nothing in Rust serializes bridge calls. So `RecordFlow` must `await` each photo before the next, or the facade must hold a lock. P21's "for each photo" is safe only if it awaits.
5. **Four new pins in `session.rs`' tests, beyond the Done line**, because the lab's tests only reach the opener through a real file on disk: an entry holds the time and name it was handed, with no source file at all; opening does not dedupe and `session_holding` does; a photo that cannot be placed leaves nothing in `.incoming/`; the content-type table.
6. **For P13**: a successful open leaves an empty `sessions/.incoming/` behind, as the lab's always did. It holds no file unless a crash left a `.part`, and it is a dotted name, so C7's rule should cover both. Check that an empty directory neither lands in the zip nor counts as a file.
7. **For P35**: `CLAUDE.md`'s `crates/core` row says what `session` holds and does not name the opener. `crates/lab/Cargo.toml`'s comment on the `golf-core` edge lists what crossed it phase by phase, and it does not name this phase either, since that file is outside this phase's Touch.

### P4 (2026-11-04)

1. **The crate is in, with C2's first row and nothing else.** Package and lib `app`, one normal edge (`contracts`), `api::about::about() -> AboutView { analysis_version, screen_parser_version, career_version, device, crate_version }`, and at the root `DEVICE`, `MIN_CONFIDENCE`, `parse_now` and `Dirs::of` (`sessions`, `shots`, `golfers`, `app`). `cargo test -p app`: 8 passed. The clippy line and `cargo fmt --check` are clean, and both iOS targets built with no `rustup target add`, as P1 finding 3 expected. `Cargo.lock` gained 7 lines, all of them `app`. `ort_reach`: 4 passed.
2. **For P5, P6, P7, P9, P10, P11, P13, P14, P15 and P28: the Verify table's clippy line goes red the moment `crates/app` takes a `storage` edge, which P5 does, and through no fault of that phase.** `cargo clippy -p X -- -D warnings` denies warnings in path dependencies too, and `crates/storage/src/shot_store.rs:128` holds an `unnecessary_sort_by`, one of the existing warnings "Not in M38" leaves alone. Measured through `golf-core`, which holds `storage`: `cargo clippy -p golf-core -- -D warnings` stops at `could not compile storage`, and with `--no-deps` it passes. So run **`cargo clippy -p app --all-targets --no-deps -- -D warnings 2>&1 | tail -15`** in place of the table's line. It is what the plan means by "`crates/app` is held to 0", and it passes today. Do not fix `shot_store.rs`. The table was left as approved, since a finding wins. `screen`, `analysis` and `feedback` each pass `-D warnings` today, so `storage` is the only one of those edges that trips it. P8's and P12's clippy lines are `-p golf-lab` with no `-D`, so they do not meet it.
3. **For P5, P6 and P13, or anyone editing `crates/app/Cargo.toml`: do not write the MCP server's package name in that file, even in a comment.** `crates/mcp/tests/edges.rs` is a bare substring search over every manifest, comments included, and answers "depends on golf-mcp". This phase's first draft of the manifest's comment tripped it. P4's Verify lines do not run that test, so it would have stayed red, unnoticed, until P28's workspace run. `crates/lab/tests/edges.rs` skips comments, and `crates/feedback/tests/profile_rules.rs` only asks for package names, and both pass. After any manifest edit, `cargo test -p golf-mcp --test edges` is cheap.
4. **`tests/test_docs_truth.py` holds no crate count**, so this phase's Touch line on `CLAUDE.md` and `docs/ARCHITECTURE.md` reached nothing, and neither was edited: 91 pass with fifteen crates on disk. Both keep saying fourteen until P35. **For P35**: its grep will find `CLAUDE.md:100` and `:237`, and `docs/ARCHITECTURE.md:337`, `:398` and `:657`. It does not look at `docs/CONFORMANCE.md:1992` ("There are fourteen in the workspace"). And four places say M38's `crates/app` calls `crates/impact`, which D4 made false: `CLAUDE.md:237`, `docs/ARCHITECTURE.md:659`, `crates/impact/Cargo.toml:11` and `crates/impact/src/lib.rs:17`. No phase's Touch line names the two `crates/impact` files, so P35 should take them. `ort_reach`'s module doc already says what D4 means.
5. **The guard was shown to fire on `app`, beyond the existing controls.** `violations()` was pulled out of the test so a control can run it: `the_facade_taking_an_edge_to_the_lab_is_a_violation` adds `app -> lab` to the real graph and asks for the chain `app -> lab -> ocr -> ort`, and `the_walk_sees_the_edges_it_is_meant_to` now asks that `app` reaches `contracts`. Once, by hand, `ocr = { path = "../ocr" }` was appended to `crates/app/Cargo.toml`: `no_crate_on_the_phone_path_reaches_ort` failed naming `app -> ocr -> ort`. The manifest was restored and `Cargo.lock` went back to item 1's 7 lines.
6. **`parse_now` and `Dirs` are `pub` at the crate root rather than `pub(crate)`**, because nothing in the crate calls them until P5, and a `pub(crate)` item nothing calls is dead code under `-D warnings`. Staying outside `api` is what keeps the codegen off them. **For P16**: if the generated Dart holds either, `rust_input` is reading wider than C1 says, and that is a finding. `crates/app/src/api/mod.rs` holds `pub mod about` and no item of its own.
7. **For P19: Dart's obvious `now` is refused.** `DateTime.now().toIso8601String()` is a local time with no offset, and `parse_now` refuses it, naming the offset. `DateTime.now().toUtc().toIso8601String()` is read, with or without microseconds, and so is `ISO8601DateFormatter`'s spelling. The tests hold these spellings as Dart's and Swift's documentation give them, and neither language was run here. A UTC `now` is fine for a `now` argument, and it would be wrong for `captured_at`, since C8 names the session by the photo's date in its own offset, which is why C9 has Swift answer that one.

### P5 (2026-11-04)

*Resumed once. The first attempt stopped on item 2's question with the rest in the tree and green, and the resume added the guard, its two pins and the tick.*

1. **C2's `setup.rs` row is in, all nine functions, with `AppState` holding C7's ten fields.** `cargo test -p app`: 42 passed, 10 in the lib and 32 in `tests/setup.rs`. P4's `--no-deps` clippy line and `cargo fmt --check` are clean, both iOS targets build, and `golf-mcp`'s `edges`, `ort_reach` and `golf-lab`'s `edges` pass. `Cargo.lock` gained `app`'s four new names and no package. Outside the Touch line, because the phase needed them: `club_id` in `crates/app/src/lib.rs`, and `storage`, `serde`, `serde_json` and dev `tempfile` in `crates/app/Cargo.toml`.
2. **The Done line's "a loft refused by `Validate`" could not be met, because no `Validate` refuses a loft, so the phase reported `BLOCKED` under rule 11 and the user answered `finite`.** `ClubSpec::validate` (`crates/contracts/src/club_spec.rs:416`) asks only that `loft_range_deg` runs low to high, and `set_bag_slot`'s `loft_deg` never sets a range (D7). `BagEntry` and `Bag` add nothing on the loft. Frozen Python's `loft_deg` has no bound either (`src/golf_coach/contracts/club_spec.py:279`), and nor does `spec/schemas/bag.schema.json`. `set_bag_slot` now refuses `NaN`, `inf` and `-inf` before the store is called, saying `a loft has to be a real number of degrees, and NaN is not`. Nothing went into `contracts`, and `crates/app` holds no range. The pins are `a_loft_that_is_not_finite_is_refused_in_the_facades_words_and_changes_no_bag`, which compares the whole bag file after a refusal, and `any_real_loft_stands_as_given_however_unlikely` (`340`, `-5`, `3.4`, `0`, `0.001`). Both were written before the guard: the first failed on `Ok([BagSlot { club: "7i", loft_deg: Some(NaN), loft_display: Some("NaN°") }])`, and the second passed, as it should with no range to get past. What `Validate` *can* refuse through a slot, the slug, is pinned as surfacing its message (`a_refusal_from_the_contract_or_the_store_reaches_dart_in_its_own_words`).
3. **Why finite and no range: what a loft did through `set_bag_slot` with no guard, measured once with a probe test since removed.** `NaN`, `inf` and `-inf` were all accepted: the answer said `NaN°`, the file said `"loft_deg": null`, `bag_slots` read back no loft, and declaring it again shelved a phantom entry, because the file reread holds no loft and the new entry holds one. By its documentation (not run here), Dart's `double.tryParse` answers `NaN` and `Infinity` for those words, so that group is reachable from a text field. `-5`, `0`, `340` and `1e308` are stored as given, which still stands, and `1e308` displays as 309 digits. In the flight, `crates/analysis/src/flight_infer.rs:385` only asks if a loft is below 13°, and below it refuses the spin. So `340` takes the branch any iron takes, and `3.4` (a typo for 34) leaves the spin unscored as a driver's is: neither flies a wrong number on an iron, and no plausible range would catch the second.
4. **Added beyond the Do line, each for a reason that may be challenged.** `init_state` refuses an id that is not C8's 4 lowercase hex digits. `set_state` cannot move, clear or invent the id, so `init_state` is its one writer. `set_state` refuses a `club` or `last_export_at` under C2's rule, and does not ask C15's, since a change sheet passes through `shot_shaping` with no shape yet. A process-wide lock in `crate::state` covers each read and write, because of P3 finding 4: `overlapping_saves_each_succeed_and_the_install_id_survives_them` was shown to fire, failing 5 runs of 5 with the lock taken out, on `No such file or directory` from the shared `.tmp`.
5. **The atomic write is a local three lines**, not `storage::write_atomically`, which is `pub(crate)` and outside this phase's Touch. A crash can leave `.app/state.tmp`. **For P13**: C16's list names `.app/`'s three diagnostics, so neither `state.json` nor that file may travel. Walk the named paths, and do not walk `.app/`.
6. **For P6**: `get_state(root).install_id` is `None` until `init_state` has run, so `record_photo` must refuse then and never name a session without it. `club_id` at the crate root is the strict reader of a club crossing from Dart, and `PracticeMode` and `TargetShape` in `api::setup`, with `From` both ways in `crate::state`, are the ones to reuse. The golfer's `player_id` in `AppState` is not checked for a slug by `set_state`.
7. **For P16**: `AppState` and the four enums derive `Serialize` and `Deserialize` inside `api`, with `#[serde(...)]` on them, which the codegen should ignore and was not run here. `impl Default for AppState` and the `From` impls are in `crate::state` so the codegen cannot see them. `Vec<TargetShape>` is a list of a plain enum, a shape the spike did not run. Rule 7 is the fix if any of these is refused. `refuse_a_loft_that_is_not_finite` and the other helpers in `api::setup` are private, so nothing there should appear in the generated Dart.
8. **For P19**: every setter answers the stored result, so a model replaces what it holds: `init_state` and `set_state` an `AppState`, `set_bag_slot` and `remove_bag_slot` the whole `Vec<BagSlot>`, and `create_golfer` a `GolferView` whose handedness may not be the one ticked. Show that one. `BagSlot.loft_display` is the text, with `°` (rule 8), and `list_golfers`, `bag_slots` and `get_state` never refuse, so they throw nothing in Dart. A loft field passes what it parsed and shows the refusal under the field, and asks no range of its own (rule 8 and item 2). **For P27**: `last_export_at` and `last_export_shots` go through `set_state`, and `last_export_shots` is `Option<i64>`, `None` before any export.

### P6 (2026-11-04)

*Everything read off the six captures below is `provisional: pasted copies`, and every number is this Mac's and not a phone's.*

1. **C2's `record.rs` row is in, and M33's numbers come out of it.** Through `record_photo` over the committed `raw` boxes: 71 cells read, 1 withheld, 0 wrong and 0 flagged, each stored field equal to `results/C0.json`'s value with no tolerance, and the confidence, warnings and `raw_fields` equal too. `read_corpus` lists all six with no exclusion. `cargo test -p app`: 75 passed and 1 ignored, of which `tests/record.rs` is 33 and the ignored one. P4's `--no-deps` clippy line and `cargo fmt --check` are clean, both iOS targets build, and `ort_reach`, `golf-mcp`'s `edges`, `golf-lab`'s `edges` and docs truth pass. `Cargo.lock` gained `app`'s three new names (`golf-core`, `screen`, `pyfmt`) and no package. `crates/screen`, `crates/contracts`, `spec/` and `spikes/` are untouched.
2. **The timing line: 56 ms with 500 entries in the store, so `Duplicates` stays as it is and is not scoped.** Release, 500 real feeds into one session: 40 ms with none stored, 37 ms with 249, 56 ms with 499, and 55 to 58 ms across the ten feeds at 500 to 509. A duplicate takes 10 ms and `read_corpus` alone 17 ms at 510. The limit was 1 s. `cargo test --release -p app --test record -- --ignored --nocapture timing` reprints it. **For P28 and P36**: that `#[ignore]` moves P2's baseline of 26 ignored to 27, and no gate moved.
3. **What was decided here that the Do line left open, each open to challenge.**
   - **The golfer, club and intent come from `state.json`, not `PhotoIn`** (C8), so a stale model cannot stamp a wrong club. `PhotoIn` is `root, path, boxes_json, captured_at, now, challenge_call`. The call is an argument because P23's Touch is Dart alone and the draw is not in the state, and it leaves the state's mode alone for "Off restores the prior mode". A golfer the registry does not hold is refused, since `set_state` asks nothing of the id (P5 finding 6). The intent is always stamped, `{fundamentals}` included.
   - **A second kind of duplicate, which the Do line does not name: the same photo in another session.** It is reachable because C9's fallbacks make a photo with no EXIF date take its file's date or the clock, so a pasted copy imported on another day names another session. `analyse_entry` would answer `Outcome::Duplicate` only after `ShotStore::put` had rewritten the held shot's id to name an entry about to be dropped. So `Duplicates::of` is asked between the read and the `put`, the entry is discarded, and the answer is the held shot. Shown to fire: without it `the_same_photo_under_another_date_…` fails with an `Err`.
   - **A photo the store already holds a shot for keeps its boxes file**, so a repeat cannot replace what the held shot was read from. A failed read's boxes are replaced by the retake's.
   - **`record_photo` takes a process-wide lock**, for P3 finding 4. `overlapping_feeds_each_get_an_entry_of_their_own` was run 6 times with the lock taken out and failed 4, on `["1", "2", "2", "3", "4", "5"]` and on a refused feed. **So P21's `await` is no longer what keeps a store whole**, though it still keeps a batch in order.
   - **A failed `put` takes the entry back, and a failed analysis does not**, since the read was good and P9 lists such an entry with its reason. Text that is not boxes is an `Err` and not `Failed`, because a retake cannot fix a reader gone wrong, and the text is kept in `.app/boxes/`.
   - **`crates/app/src/record.rs` is outside this phase's Touch line, for rule 7**: `api/record.rs` holds only the shapes and two one-line calls, and it has no private helper for the codegen to meet. `engine_versions()` at the crate root is C23's pair, for P9 and P11 to reuse.
4. **What a card's `display` does.** `pyfmt::g` with no table of decimals, because the screen is not uniform (`95` beside `86.9`, `19°` beside `24.5°`), so `-2.0°` shows as `-2`. A lit tile shows its magnitude and letter, and `sign_letters` is the negative's first: `("L", "R")` and `("D", "F")`. The plan's `"2.2 L"` was an example: `IMG_2893`'s is `2.2 R`, which is pinned.
5. **For P7 and P22: `value` and `display` differ on three rows, and C11 does not say which a person's number is. Settle it in P7.** `apex_height`'s `value` is yards and its label and `display` are feet (`28.4`, `Apex Ft`, `85.2`), and `launch_direction` and `sidespin` hold a signed `value` and print a magnitude and a letter. A person reads `85.2` off the photo, so passing what they type through as `ShotData`'s number stores an apex three times too high. Rule 8 leaves Dart no arithmetic and no sign rule. **Recommended**: `correct_shot`'s edit takes the printed number and, on a lit tile, the letter, and Rust converts, through `PrintedUnit::to_yards` and the tile's `lit_sign`, with `corrections` still holding the stored value's `pyfmt::repr` (C11). `crate::card::card_of(&shot, session_id, swing_id)` is P7's card, and a corrected row reads `Corrected` already, a cleared one included.
6. **For P9**: `card_of` refuses a shot with no provenance, since a 0 confidence would be false, and it is a `Result`. An entry whose shot the store does not hold has no card, and `crate::record::held_card` refuses that case, so the detail screen has to say something of its own there.
7. **For P10**: the shot store is keyed by the photo's hash and `Duplicates` is per golfer, so one photo under two golfers in two sessions gets a second entry whose `put` rewrites the first's record. `delete_shot` removing "its shot store record" would then take the other golfer's. Reasoned and not run. Also, `intent_action`'s refusals name its flags (`--mode shot_shaping needs --shape`). `crate::record::goal_for` prefixes them with a sentence for a person, and `set_shot_intent` will meet the same words.
8. **For P13 and P14**: a crash can leave `.app/boxes/<digest>.tmp`, so read only `*.json` there. A file under `.app/failed/` may have no extension. A box file is a list or an object with a `boxes` key, so P14 should read them with `crate::record::parse_boxes`, already `pub(crate)`. `ReadLog` derives `Deserialize`, and its statuses are spelled `read`, `flagged`, `duplicate` and `failed`, as the bars file asks. **A refusal writes no line**, since it has no status, so an `Err` at the bay is in no count. P14's validity check should know that, and it is why P21 has to show one.
9. **For P16**: new shapes for the codegen: `Option<(String, String)>` inside `Vec<CardRow>` (the spike ran a bare tuple), `Option<TargetShape>` on a struct, and `RecordStatus`, `ReadSource` and `ReadLog` deriving serde with `#[serde(...)]`. Rule 7 is the fix if any is refused. `crate::record` and `crate::card` are private modules, so neither should appear in the generated Dart.
10. **For P17**: `boxes_json` reads as a list of boxes or as an object with a `boxes` key beside anything else, so `ScreenReader`'s `json` may be either, and M33's file reads as it stands. Keys beyond `TextBox`'s, inside a box, are not refused. The text is stored as given, which is what P18 compares.
11. **For P18 and P21's integration tests: copy a capture out of the repo before feeding it.** A `Failed` read moves the file it was given, as the plan says, and everything else leaves it. So a capture Vision returned nothing for would leave `spikes/vision-ocr/captures/`, against rule 5. `tests/record.rs`' `Phone::photo` is the pattern. C22's `TestPhone` should hand back copies.
12. **For P20**: the path's file name is what the manifest records as `original_filename`, and P15's fallback matches on its stem, so `pickPhotos` must keep the whole name and not only the extension. Nothing removes a photo from `tmp/` on `Read`, `Flagged` or `Duplicate`, so Swift or `RecordFlow` should, or a HEIC batch sits there until iOS purges it.
13. **For P21**: `RecordOutcome` is `status, card, message, photo_sha256, boxes, ms_rust`, and `log_read` needs the last three because Dart neither hashes nor parses. `ms_vision` is Swift's and `ms_total` Dart's. `card` is `None` only on `Failed`, and `message` holds the sentence for `Failed` and `Duplicate`. `Err` is a third thing, with no card and no status, and it says what to change: no golfer, no club, or a mode and shape that do not go together. The last is reachable, because `set_state` lets a change sheet save `shot_shaping` with no shape (P5), so **the sheet must not close in that state**, or the golfer reads a sentence naming `--mode`. **For P23**: pass the current call as `challenge_call`, with `None` otherwise, and the state's mode need not change.

### P7 (2026-11-04)

*The captures' results below are `provisional: pasted copies`.*

1. **C11 is in `golf_core::session::correct_shot`, with `api::review::correct_shot` as C2 names it over it, and every clause of the Done line has its pin.** `cargo test -p golf-core -p app`: all green, with 25 new pins in `session.rs` and 10 in `crates/app/tests/review.rs`. P4's `--no-deps` clippy line and `cargo fmt --check` are clean, the iOS target builds, and `ort_reach`, `golf-mcp`'s `edges`, `golf-lab`'s `edges` and docs truth (91) pass. `Cargo.lock` did not move. `spec/`, `spikes/`, `crates/screen` and `devices.json` are untouched, and the one edit in `crates/contracts` is two doc comments.
2. **Frozen Python reads a corrected shot**, by hand and on a real one: `IMG_2893` through the facade with four edits (a typed withheld sidespin, the apex in feet, a lit letter, a clear), then `.venv/bin/python -c` through `golf_coach.contracts.shot.ShotData.model_validate`. It answered `None 30.0 -1.5`, `needs_review False`, over a file holding `{'apex_height': '5.166666666666667', 'carry_distance': '184.0', 'launch_direction': '2.2', 'sidespin': ''}`. So ADR-035 clause 4 holds, by frozen Python ignoring the key.
3. **P6 finding 5 is settled as it recommended: a person types what the screen prints.** `ShotEdit` is `field, printed, letter`, and `stored_number` runs the parser's order, the sign and then the unit, so `90` under `Apex Ft` stores `30.0` and `2.2` with `L` stores `-2.2`. `corrections` stays in `ShotData`'s units and sign (C11), as item 2's file shows. The strongest pin is `typing_back_what_every_row_shows_moves_no_byte`: on all 71 of M33's read cells, typing back `printed` and `letter` lands on the read's own bits, leaves no entry and moves no file.
4. **C11's "fails `ShotData`'s `Validate`" and the Done line's "an invalid value" cannot both mean what they say, for P5's reason, and this was not stopped on because the user has already answered it.** `ShotData` bounds no reading, on purpose (its doc), so no number can fail `Validate`. P5 reported that `BLOCKED` for a loft and the answer was `finite`, with no range anywhere. So the invalid value here is what is not a finite number, refused whole with nothing written (`8S.2`, `""`, `NaN`, `inf`, `1e999`), and any real number stands (`any_real_number_stands_however_unlikely`). Every edit is still read back through `Validate`, so a bound added later bites with its own message: `what_validate_refuses_is_refused_with_its_message` reaches that clause only with an in-memory shot, since a stored one that fails cannot be read. **If the user reads this differently, it is one guard**, the `is_finite` filter in `stored_number`.
5. **What was decided here that the plan left open, each open to challenge.**
   - **The number crosses as text, not `f64`.** C2 says only `edits`. A Dart `double.tryParse` answers `null` for garbage, by its documentation, and an `Option<f64>` would carry that as "clear this", so a typo would delete a value. As text it comes back as a sentence, and Rust is the one place saying what a number is.
   - **`CardRow` gained `printed` and `letter`**, outside the Touch line and for rule 8: `display` in two pieces, so a review can fill its field and its control with no string splitting in Dart. `pyfmt::g` is six significant digits, so **a row a person did not touch must not be sent**, and `FieldEdit`'s doc says so.
   - **A lit tile showing zero now shows no letter**, a change to P6's card. It said `0 R` for a letter the screen never lit, and the parser's rule is that zero needs no sign. No test pinned the old answer.
   - **A value typed for a tile the parse did not locate joins `fields_present`.** C11 does not say this, and without it the number is stored and ignored: `analysed_view` blanks a reading outside that list. Shown to fire: the analysis then answers `None` where `Some(130.5)` is asked. A later clear does not take it off again, because the list cannot say who put it there.
   - **A shot that does not change moves no byte**, stamp included, so opening and saving an unflagged card leaves a re-import at the lab reading `identical`. `now` stamps `updated_at` on a change, through `set_intent` with the intent the manifest already holds, because `SwingBundleStore::repair` is private and storage is outside the Touch. Nothing reads `updated_at` today, so that is `now`'s whole effect. The analysis runs either way.
   - **A failed analysis puts the old shot back**, so the store and `shot_analysis.json` cannot disagree (shown to fire). The manifest's stamp stays, harmlessly. P6 keeps an entry whose analysis failed because the read was good, and this differs because a person's edit can simply be saved again.
   - **Three refusals beyond C11**: a swing with a face-on clip, which the lab analyses and this cannot; an entry the corpus counts as a duplicate, since it shares a record with the one that counts; and a tile signed by a word or a polarity (`hd_golf`'s), since a guess at its sign is a wrong number half the time and D3 offers none. `is_safe_segment` guards both ids, and its refusal moved into `check_segments` with its words unchanged.
   - **`correct_shot` holds `record_photo`'s lock**, now reached through `crate::record::one_writer()`. `overlapping_corrections_each_land` failed 6 runs of 6 with it taken out.
6. **A limit, recorded and not fixed: a cleared tile's `unscored` sentence is not true.** Clearing `carry_distance` on `IMG_2893` writes `misread`, with `Carry Yd: '184' under the label, and no value could be read from it`. A person cleared it, and the read was fine. `crates/screen/src/tiles.rs`' doc says `corrections` need not be read because "a field a person corrected holds the corrected value", which a clear falsifies. Fixing it would need a choice of reason from `UnscoredReason`'s closed list, and it is outside this phase's Touch. **For P25**: show the row's `Corrected` state rather than this sentence for such a field. **For P35**: amend that doc line, or name this limit in ADR-014's addendum.
7. **For P8**: a re-read has three things to carry and not two. The field's value and its `corrections` entry (C12), and also its place in `fields_present`, or a value typed for a tile the new read did not locate is stored and blanked by `analysed_view` (item 5). The entry's value stays what the first read said, as C11 means "as read before a person changed it", so do not rewrite it from the new read. A cleared field (`value None`, an entry holding the old number) is a correction too and has to stay cleared. `written[field]` through `serde_json::Value`, as `with_edits` does it, keeps the struct the only list of fields. **A field's value is `ShotData`'s**, so no unit or sign work is needed on that side.
8. **For P9**: `crate::card::card_of(&shot, session, swing)` is the detail screen's card too, and a corrected row reads `Corrected` already. **For P10**: every writer of a shot or an entry should hold `crate::record::one_writer()` for its whole read, change and write, and `set_shot_club`, `set_shot_intent` and `delete_shot` are such writers. `set_shot_mishit` writes only the manifest, through the store's own lock, but a re-analysis beside it would be one too. `analyse_entry` answers `Outcome::Duplicate` only after a write has happened, which is why `correct_shot` asks `Duplicates::of` before writing, and the same order applies there.
9. **For P13**: reasoned from `crates/lab/src/import_export.rs:654-790` and not run. After a correction, a re-import prints the shot as `kept` (C16's limit) and the swing as `duplicate … and that swing stays`, since its `shot_analysis.json` and manifest differ and `session_holding` finds the photo. So neither is a conflict, and the cumulative export still imports. A save that changed nothing leaves `identical`. Pin both in the round trip.
10. **For P14 and P15**: a `corrections` value is a stored number as `pyfmt::repr` spells it, in yards for the apex and signed for a lit tile, so it compares with `results`' cells by value and needs no conversion. `""` means the reader gave nothing there. A field with an entry and no value is a person's clear. **An entry can be a cell a person disagreed with while the reader was right**, so it counts what was corrected and not what was wrong.
11. **For P16**: new shapes for the codegen: `Vec<FieldEdit>` as an argument (a list of a struct, inbound), and two more `Option<String>` on `CardRow`. `crate::record::one_writer` and `crate::card` are outside `api`, so neither should appear in the generated Dart. `api::review` holds one struct and one function and no helper.
12. **For P22**: the review's whole contract with Rust. Fill the field from `CardRow::printed` and the control from `CardRow::letter`, offering `sign_letters`. Send a `FieldEdit` only for a row the person changed, or the six-digit `printed` could move a longer stored number. `printed: None` clears. **An emptied field is `Some("")` if sent as it stands, and that is refused**, so decide on purpose whether emptying means clear, given that the plan has a clear control. A lit tile with a non-zero number needs `letter`: the refusal says which two, and the field's `NotRead` row starts with none selected. Save's `Err` is one sentence naming the tile's label, for the whole call, with nothing written. Show it inline. Replace the card you hold with the one `correct_shot` answers. An empty list releases a flagged shot.

### P8 (2026-11-04)

1. **C12 is `carry_corrections` in `crates/lab/src/shot.rs`, called from `read_screen`, so `shot_for` and `import_photo_with` both have it**, and every clause of the Done line has its pin. `cargo test -p golf-lab`: all green, with 16 new pins in `crates/lab/tests/reread.rs` and no existing test edited. `cargo fmt --check` is clean, clippy names neither touched file (the one `golf-lab` warning left is `tests/import_video.rs`', not touched here), and docs truth is 91. `spec/`, `spikes/`, `crates/screen` and `devices.json` are untouched, and no dependency was added. Each shot starts from P7's real `correct_shot` on an opened and analysed photo entry, so what is carried is what a correction writes.
2. **Shown to fire.** With the tests written and no `carry_corrections`, 12 of the 16 failed and the 4 about today's output passed: both `moves_no_byte` pins, `--force` over a file that is not a shot, and a read of nothing. So "moves no byte" held on the code as it was. One of the 12 also held a wrong word of mine (`Smash Factor` for the validator's `smash factor`), fixed once the rest passed. Then seven mutations of the finished function, each caught: a clear's place dropped, a value's place only if listed before, no place at all, the `--force` lookup raising, a clear not carried, `--force` not carried, and the entry rewritten. The file was put back and all 16 pass.
3. **P7 finding 7 was right about all three things, and one of them bites in a way it did not name.** `backspin` and `sidespin` are `skip_serializing_if` at `None`, so a cleared `sidespin` is an absent key in the stored file rather than a null. `carried[field]` reads an absent key as null and `written[field] = Null` deserializes to `None`, so it works by `Value`'s indexing and not by a case here. `a_cleared_tile_stays_cleared_with_the_number_it_held_in_the_map` runs one key of each kind.
4. **What was decided here that the plan left open, each open to challenge.**
   - **`--force` now looks in the store, in `read_screen`, for the corrections alone**, under the bytes' digest. So a correction follows the bytes and not the manifest (`a_correction_follows_the_photos_bytes_and_not_its_manifest`). Its identity stays a new import's and its status stays `parsed` with `replacing: None`, as C12 asks only for the value and the entry.
   - **A stored file that is not a shot is not an error under `--force`**, though it is on the lookup. Today's `--force` never reads it and replaces it, so raising there would have moved today's output. It holds no correction anyone could keep.
   - **A value always has its place, and a clear has the place it had.** A clear says nothing about whether a tile is there, so the shot's answer does not depend on the re-read. `a_value_on_a_shot_with_no_list_is_listed_on_the_new_read` covers an older shot with `fields_present: None`, where a value still gets a place, which is `correct_shot`'s rule. No run reaches that shot today, since a stored shot that has met `correct_shot` has been stamped, so it is a guard.
5. **Limits, recorded and not fixed.**
   - **`--force` can no longer undo a correction.** Typing the first read back on the phone does (`a_carried_entry_is_still_removed_by_typing_the_first_read_back`), and at the lab deleting the store's file does. Nothing prints that a value was kept: the log lines and the `import-photos` table read as they did, and `parsed` can now sit over a person's number. A line saying so would be one `log` call in `shot_for`, and C12 did not ask for one.
   - **The new read's flag and warnings judge its own numbers, not the person's**, and `raw_fields` is its text. So a warning can name a number the shot no longer holds, and `Carry Yd: 132` can sit beside a stored `130.5`, with the map saying `131.0`. C12's "a new flag stands" means this.
   - **A corrected shot a re-read flags has nowhere to be reviewed at the lab**, which has no review verb, and the phone does not re-read ("Not in M38"). The corpus holds it out of auto-mishit until someone reviews it. It can only arise on a parser bump or `--force`.
   - `tests/shot_lookup.rs`' comment "the store is not consulted" on the `--force` case and `crates/lab/src/import.rs`' module doc on `--force` were left, because the Done line says otherwise unedited and neither is false about what it describes. `crates/lab/src/shot.rs`' module doc has the whole rule.
6. **Not run, and why**: `every_stored_shot_is_stamped_by_this_parser_and_served_from_the_store` stays ignored, since this clone has no `data/`, and the real reader did not run, since this Mac has no OCR models. Neither is reached by a re-read of a corrected shot in a test here: a stand-in reader answers with `full-swing-grid`'s committed boxes, so the parse and the stamp are `crates/screen`'s real ones.
7. **For P13**: reasoned and not run. A shot corrected on the phone and imported keeps its correction through a later `golf-lab reanalyze`, since the store is keyed by the photo's digest and `import-export` moves the file as it is. One re-read in the round trip would pin it.
8. **For P35**: name C12's rule in ADR-014's addendum with the corrections, including that `--force` no longer undoes one, and in `docs/ARCHITECTURE.md` §1 beside `--force` and `--force-ocr`, whose readers would otherwise expect a fresh read to replace every value.

### P9 (2026-11-04)

*The captures' results below are `provisional: pasted copies`.*

1. **C2's two read functions are in, `list_sessions` and `shot_detail`, and every clause of the Done line has its pin.** `cargo test -p app`: 122 passed and 1 ignored, of which `tests/history.rs` is 34 and 3 more are unit tests in `crate::history`. `cargo test -p golf-core` is green, P4's `--no-deps` clippy line and `cargo fmt --check` are clean, the iOS target builds, and `ort_reach`, `golf-mcp`'s `edges`, `golf-lab`'s `edges` and docs truth (91) pass. No manifest was edited, so this phase did not move `Cargo.lock`, and nothing under `spec/`, `spikes/`, `crates/screen`, `crates/storage` or `crates/contracts` was edited. **Outside the Touch line**: `crates/app/src/history.rs`, for rule 7 and P6's reason; `tile_for` and `display_of` in `crates/app/src/card.rs`, so a row's text is a card's; the two `mod` lines; and `crates/core/src/session.rs`, where `loft_note`'s private `RERUN` became `pub const RERUN_SHOT`, its words unchanged, for item 5.
2. **Shown to fire.** Sixteen mutations of `crates/app/src/history.rs`, each failing the test meant for it and the file put back: no lock on the list, the corpus's words for a drill, no time sort, rows walked forwards, sessions oldest first, a cleared tile keeping its sentence, the lab's verb in a note, a drill's detail not told of the file, every golfer listed, an empty session shown, a drill's verdict ignored, the corpus's mishit ignored, a missing carry as `0`, the reason as `summary`, the reason as `detail`, and arrival for the photo's time. **The race took two tries, and that is on the record so nobody re-merges it.** One loop calling both reads passed 8 runs of 8 with either lock out, because the locked read lines the loop up behind each save. Split into one test per read, at 60 saves and with its lock out, a list failed 5 of 5 and a detail 2 of 5. A list's failure was the real thing: `carry_display: None`, with `time` fallen back to the entry's arrival, on a shot that held a value throughout. Neither test can fail with the lock in, so a detail's 2 of 5 is a weak alarm and not a flaky one.
3. **The timing line: 32 to 35 ms for a list and 17 ms for a detail, with 500 entries in the store**, release, this Mac and not a phone, three reads in one run. So P24's refresh on every tab entry is fine as planned. A temporary test, removed, so P6's count of 27 ignored stands.
4. **Two lines of the Done list could not be met as worded, and neither was a reason to stop, since each has only one meaning that holds.**
   - **M33's named case never reaches `unscored`.** `IMG_2893`'s sidespin is `shown_only` on Full Swing, and `tile_refusals` asks only of analysed tiles, so its `NotRead` card row is all that says so, which is pinned. "An unscored row carries the contract's sentence" is pinned on two edited-box tiles, a `---` carry (`printed_blank`) and an `XX` ball speed (`misread`), and on what the six really hold: **each has exactly one row, `flight_carry_yds`, 4 `carry_unreachable` and 2 `no_club_loft`.** So on this evidence a flight refusal may be the usual row in this list and not the exception, though a declared loft removes the second kind, and P25 should lay it out as ordinary and not as an alarm.
   - **"A `None` measurement" has no such thing to be.** `Measurement.value` is an `f64`, so a number with no value has no row. The pin is that a withheld tile and a cleared one read as absent on the row, on the card and in the measurements, and a typed `0` still reads `0`.
5. **What was decided here that the plan left open, each open to challenge.**
   - **`left_out`'s sentences are the phone's, by `ExclusionReason`, and not the corpus's `detail`.** The corpus's name verbs a phone does not have: every drill would read `clear its intent with golf-core set-intent`. `spec/` pins those words (rule 4), so they stay for the lab, and `crate::history` has an exhaustive match, with a unit test walking `ExclusionReason::ALL` for lab words. A duplicate keeps the corpus's, prefixed, since only it says which entry was kept. The Do line says "why the corpus left it out", which this keeps. **If the user wants the corpus's words verbatim, it is one function**, `left_out_sentence`.
   - **A loft note's last clause is swapped for `then save a review of this shot`, as it is shown**, and the stored file keeps the lab's. I first took this note for rare, and the captures say otherwise (item 4). It goes through `RERUN_SHOT`, so there is no second copy of the words to drift. Every repair the sentences offer is pinned as working: a save restores a missing analysis to the whole detail, clears `stale` with `outdated`, and with a loft in the bag clears the note and the refusal.
   - **Rows are newest first as well as sessions**, by the photo's time and not its arrival, with ties to the later entry. The Do line does not say which level "newest first" means, and P24's Done line pins "order".
   - **Fields beyond the Do line**: `session_id` on a row; `time_display` (`20:42`), since C2's RFC 3339 `time` parsed by a Dart `DateTime` loses the offset; `carry_label` and `ball_speed_label`, the tile's own words, so a `None` still says what is missing; `mishit_verdict` beside `mishit`, because P25's control needs "automatic" told apart from a verdict; `left_out` on the row, for the Done line's "with its reason"; `detail` on a measurement, which `shot_measure`'s comment calls "the one line about this metric a reader is guaranteed to see"; `label` on an unscored row; and `row` and `no_card` on the detail.
   - **`left_out` is a list**, since one entry can be `stale` and `outdated` at once. A detail adds the not-analysed sentence where the corpus stopped before asking, on a drill, or its empty lists would be unexplained, so its `row.left_out` can hold one more than the list's. Those are the only rows that differ.
   - **Both reads hold `one_writer()`**, item 2 being why. A wait costs at most one photo's record.
   - **`list_sessions` is a `Result`**, though only a build that lost a tile can refuse. `shot_detail` asks nothing of `is_safe_segment`, which is private to `golf-core`, since it writes nothing and its ids came from the list.
6. **Limits, recorded and not fixed.**
   - **A measurement's `name` and a flight row's `label` are the contract's raw keys** (`start_line_offline_yds`, `flight_carry_yds`), because the Do line says "existing labels" and none exists for them. Rule 8 keeps the fix out of Dart, so **if P25's screen reads badly, the fix is a table in `crate::history`**, and that is the user's call.
   - **`pyfmt::g` shows derived numbers to six digits**: `22.973` and `-2.2619` yards, beside a printed `131`. It is the card's rule and the Do line's, and it hides nothing. A table of decimals would be the same kind of fix.
   - **Dropping a cleared tile's row is in the view only.** The stored `misread` stays in `shot_analysis.json`, and the corpus's `shot_unscored` still lifts it, so P7 finding 6's limit stands for a topic's count.
   - Only a loft note's clause is translated. `handedness_for`'s and ADR-014's notes name no verb, and I read no other note.
7. **For P10**: `api::history::MishitVerdict` is declared, and `crate::history` holds only the `From` out of `contracts`, so add the reverse there as `set_shot_mishit` needs it. **A writer must not call `crate::history::list` or `detail` while holding `one_writer()`: `std`'s `Mutex` does not re-enter, and it hangs.** Drop the guard first if a writer answers a `ShotDetail`, as `correct_shot`'s shape would suggest. The drill's sentence says `Change what it was practised as if it should count`, so `set_shot_intent` is what makes that true, and a test there should see `left_out` empty afterwards. The tests' `set_verdict` goes straight to the store, and leaves `tests/history.rs` true when P10 lands.
8. **For P11**: `CorpusAnswers` is `crate::history`'s and reads the corpus for its own two questions, so it is no shortcut for a profile, and C23's `run_profile` stands. `display_of` and `tile_for` in `crate::card` are `pub(crate)`.
9. **For P16**: new shapes for the codegen: `Option<MishitVerdict>` on a struct, `Vec<SessionView>` holding `Vec<ShotRow>`, a struct field holding a struct (`ShotDetail.row`), and `Option<ShotCard>` on one. `crate::history` and its `From` impl are outside `api`, so neither should appear in the generated Dart. `api::history` holds one enum, five structs and two functions, and no helper.
10. **For P24**: everything is on `SessionView` and `ShotRow`. Marks are `flagged`, `corrected`, `mishit` and `untracked`, and `mishit` already means "held out", whoever said so. A not-read carry is `carry_display: None` with `carry_label` still set. `photo_path` is `Option` and can be a HEIC. `time_display` is ready to show, so do not parse `time`. `left_out` is sentences and not a mark, and a row with any but no mark is a shot missing its analysis, so decide how it shows. Rows arrive in order, so sort nothing.
11. **For P25**: `ShotDetail.row` carries the photo, club, intent and mishit state, so the screen needs no second call. **The mishit control starts on `row.mishit_verdict`, `None` being automatic**, and `row.mishit` says what is in force, which can differ. **P7 finding 6 is already handled**: a cleared tile has no `unscored` row, and its card row reads `Corrected`, so Dart does nothing. `card` can be `None`, with `no_card` the sentence to show, and then edit has nothing to open. `MeasurementRow::detail` is what stops `flight_carry_yds` reading as measured, so show it. `row.left_out` here can be longer than the list's (item 5).
12. **For P35**: name in ADR-014's addendum that the phone words `left_out` itself, and why, and that the stored notes and corpus sentences keep the lab's. Item 6's first two limits are what a person will see first, so they belong beside it.

### P10 (2026-11-05)

*Resumed: the attempt marked 2026-11-04 22:34 left nothing but its marker, so this is the whole phase. Everything read off the six captures below is `provisional: pasted copies`, and every number is this Mac's and not a phone's.*

1. **C2's last four `history.rs` functions are in, and every clause of the Done line has its pin.** `set_shot_club`, `set_shot_intent` and `set_shot_mishit` answer `ShotDetail`, and `delete_shot` answers `()`. `tests/shot_actions.rs` is 33. `cargo test -p app`: 155 passed and 1 ignored, P6's timing line. `cargo test -p golf-core`, P4's `--no-deps` clippy line for `app`, `cargo fmt --check`, and `tests/test_docs_truth.py` all pass, and `golf-lab`, `golf-mcp` and the iOS target still build. No dependency, no vector and no version key moved.
2. **One thing beyond the Touch line, and why.** `crates/core/src/session.rs` gained `entry_to_change` and `EntryToChange`, the opening of `correct_shot` moved out unchanged, and `check_segments` went `pub`. A retag has to refuse exactly what a correction refuses, before it writes (P7 finding 8), so a copy in the app would be the second definition C23 forbids. `crates/app/src/record.rs` split `remove_entry` out of `discard` and made `goal_for` `pub(crate)`. `correct_shot`'s own tests are unedited and green.
3. **Shown to fire.** Fifteen mutations of `crates/app/src/history.rs`, the file put back after each. Thirteen failed the tests meant for them on the first run. The other two:
   - **A verdict with no lock passed the first test written for it**, `a_verdict_and_a_retag_overlapping_both_land`, because each answer's read queues behind the retag, as P9's race note warns. Eight threads flipping one verdict fail 4 runs of 4 with it out, with `could not write …: No such file or directory`: every manifest write goes through one `manifest.tmp`, and two writers rename each other's away. So that is now `overlapping_verdicts_never_refuse_each_other`, and the first test's comment says what it does not show.
   - **A writer reading its answer while holding the lock hangs**, measured and no longer only reasoned (P9 finding 7): that mutation did not finish in 200 s, and its process had to be killed by hand. A hang is how such a mutation fails here, so `every_writers_answer_is_the_detail_as_it_now_stands` reports it only by never returning.
4. **What was decided here that the plan left open, each open to challenge.**
   - **Analysed first, stored second.** `analyse_entry` takes the manifest it is handed, so it is handed the changed one, and a refused analysis leaves nothing moved, `updated_at` included. The other order needs a way back, and the store has none to a manifest naming no club. If the manifest then cannot be written, the analysis is run again under the stored one, pinned by a directory at `manifest.tmp`.
   - **Saying what is already said moves no byte**, stamp included, for all three, for `correct_shot`'s reason. The analysis still runs, so picking the same club is also a repair of a lost analysis. A manifest with no goal reads as `fundamentals`, so saying that moves nothing either.
   - **A retag refuses a duplicate and an entry whose shot is gone. A verdict and a delete take both.** A verdict reads no shot, and a delete is what is left for such an entry. A club outside the bag is not refused: D7's picker is Dart's, and the loft note already says what it costs.
   - **`delete_shot` takes no `now`**, having nothing to stamp, and removes an emptied session's directory, as `discard` does. A manifest that does not read cannot be deleted, since no screen could have offered it.
   - **The segment check is what guards a delete.** `delete_swing` removes what it is pointed at, and with a manifest at `sessions/manifest.json`, `S/..` would pass the manifest check and remove every session. The test plants that decoy.
5. **P6 finding 7 is now run.** One photo under two golfers in two sessions is two entries over one record, so `delete_shot` walks every session after the entry has gone, and takes the record only if nobody holds the photo. Pinned in both orders. The same rule keeps a duplicate's kept entry whole, and when the kept one goes, the copy reads `Saving a review…` and that repair works.
6. **Limits, recorded and not fixed. The second needs the user's call.**
   - **A retag, an intent or a verdict after the lab imported the shot does not reach the lab, nor does a delete.** Run through the real `golf-lab import-export`: a retag after an import prints `duplicate sessions/S/1: session S holds its photo already, in swing 1, and that swing stays`, exit 0, and its shot file reads `identical`. So it is C16's limit, under the word `duplicate` and not `kept`, since a shot's record does not hold its club.
   - **Deleting a session's newest entry gives its number to the next photo, and if the lab imported the deleted entry, every later import from this phone is refused whole.** Run through the real verb: two shots imported, the second deleted, a third recorded as `S/2`. Exit 1, `nothing was imported`, naming `S/2/manifest.json` and `S/2/shot_analysis.json`. The lab does not hold the new photo, so it is no duplicate, and it is other bytes at one path. Export is cumulative, so the conflict comes back on every export until someone removes the lab's `S/2` by hand. It needs one session exported, imported, then a stray deleted and another photo taken under the same date, so a bay session with one export at its end does not meet it. **The smallest fix seen, not built because it is a new decision**: `.app/` records each deleted entry's number and `NewPhotoEntry` takes a floor for `next_swing_id`, about 40 lines across `crates/core`, `crates/lab`'s two callers and `record.rs`. `deleting_the_newest_entry_gives_its_number_to_the_next_photo_and_an_older_one_does_not` pins the phone's half.
   - **A surviving entry's shot can go on naming the entry deleted**, as its record's ids are whichever entry wrote last (P6 finding 7). Nothing opens a shot by them.
7. **For P13**: pin both of item 6's first two in `export_roundtrip.rs`, as the Done line pins C16's: a retag after an import prints `duplicate`, refuses nothing and moves nothing at the lab; and a deleted number reused after an import is exit 1 with nothing written, **unless the user has decided on the fix by then**. The hand-run's shape: copy `sessions`, `shots` and `golfers` to a directory, since the verb takes one.
8. **For P14**: a deleted shot's `reads.jsonl` line and `.app/boxes/<digest>.json` stay, as what happened at the bay (C7), and a photo recorded afresh appends a second line for one digest and replaces the boxes. So a read line can name a digest no entry holds, and reads need not equal entries. A delete writes no line, so nothing in an export says one happened.
9. **For P16**: new shapes for the codegen: `Option<MishitVerdict>`, `Option<TargetShape>` and a plain `PracticeMode` as **arguments**, where they have only been struct fields, and `Result<(), String>` as a return. `golf_core::session::EntryToChange` and `crate::history`'s writers are outside `app::api`, so neither should appear in the generated Dart.
10. **For P25**: all three writers answer the `ShotDetail` the screen is showing, so it needs no second call, and `row.left_out`, `notes` and `measurements` have already moved. `delete_shot` answers nothing, so go back to History. Show a refusal as its text: none changes anything. **Do not offer a shape without `ShotShaping`, nor `ShotShaping` without a shape**, since `goal_for`'s refusal carries the lab's flags (`--mode shot_shaping needs --shape`) behind its sentence for a person. **Delete's confirmation should say it cannot be undone**: the photo in the entry goes too. An entry with no shot refuses a club or an intent, so delete is the action left on it.
11. **For P31**: if the user has not chosen item 6's fix, the run sheet says not to delete a shot and then record more under the same date once that date has been exported and imported. **For P35**: item 6's first two belong beside C16's limit and P9 finding 12's, in ADR-014's addendum.

### P11 (2026-11-05)

*Everything read off the six captures below is `provisional: pasted copies`, and every number is this Mac's and not a phone's.*

1. **C2's `graded_profile` is in, and every clause of the Done line has its pin.** `api/profile.rs` holds the shapes and one call, and `crates/app/src/profile.rs` fills them. `tests/profile.rs` is 24. `cargo test -p golf-core -p app` is green with `golf-core profile`'s vectors unedited, P4's `--no-deps` clippy line passes for `app` and for `golf-core`, `cargo fmt --check` and `tests/test_docs_truth.py` pass, and the iOS target, `golf-lab` and `golf-mcp` still build. No dependency, no vector and no version key moved: `git status` on `spec/` is empty.
2. **The gate, and what it does beyond the Done line.** Each view is destructured whole with no `..`, so a new field stops the test file compiling until someone says whether it is a display string. It runs over each hand case against the live report, as the Done line asks, and also over each `corpus/` case against its **committed** `report`, the real golfer's included. `the_gate_reads_every_field_at_least_once` fails if no case fills one of the 28 fields. A substring gate passes a true sentence on the wrong club, so `every_field_sits_where_the_vector_says` holds the structure to each hand case's `expected`.
3. **The gate changed one shape, and that is the one departure from the Do line.** `player` is `Option<PlayerView>`. With no tracked shot, `golf-core profile` prints its note and stops, so the first draft's `0 tracked shot photos across the bag` and `none: these shots print…` were things the report does not say, and the gate said so. `None` is the plan's "empty profile". Nothing else was asked of that state.
4. **`numbers` is what the report's `spread` rows print, read through `run_profile`'s `StrikeProfile`.** That is the Do line's "carry, ball speed and the other per-club stats, with n and spread", and `StrikeProfile` copies them from `build_bag_profile`'s `ClubProfile`. So there is no second build and no `analysis` edge.
5. **A club's mean carry is not on the view, and the user may want to look at this.** `golf-core profile` prints no mean, so the gate refuses one, and the Do line's "with n and spread" does not ask for it. `golf-core club-profile` does print it. The cost shows on the six captures: every number reads `withheld: a claim about how repeatable this is needs 10 samples (there are 6)`, while `minimum_n` lets a mean through at fewer, so the Profile tab says nothing about how far the 7 iron goes. **The smallest fix seen, not built because the gate forbids it**: a `center` on `NumberRow` held to `reports::club_profile`'s text for the same case, about 20 lines, with no re-record.
6. **Beyond the Touch line, for rule 7**: `crates/app/src/profile.rs`, a line each in `lib.rs` and `api/mod.rs`. `app::profile_view` is `pub` at the crate root because the gate's inputs are corpora and a test is outside the crate, which is `parse_now`'s footing. In `reports/profile.rs`, sentences written inline became named helpers the report now prints through (`landings_text`, `blend_text`, `spread_rows`, `not_judged`, `no_landing`, `no_clubs_note`, `preamble`, `club_shots`, `player_shots`, `NO_TOPICS`, `NOTHING_LISTED`, `PLAYER_BASIS`). No text moved.
7. **Fields beyond the Do line, each because Dart may not spell it (rule 8)**: `preamble`, `no_clubs`, `shots`, `basis`, `no_topics`, `nothing_listed`; `ShareView.percent` for a screen to set large, whose `None` is what says withheld; `NumberRow.withheld`; `BlendView.text`.
8. **Left out on purpose, in `crate::profile`'s doc**: the impact map and its lean, since only a clip fills one and D4 keeps clips off the phone; the player's raw numbers, by the Do line; the sentence for a missing blend, since P26 shows no card.
9. **Shown to fire.** Twenty mutations, each file put back after. Seventeen failed the tests meant for them. The other three:
   - **Dropping the bag fails nothing, and that is true of the code**: nothing in `analysis::strike` or `feedback::profile_rules` reads a declared entry. It is still passed, as `golf-core profile` passes it, so a grade that comes to read it cannot split the two.
   - **The lock is not shown to matter here.** With it out, `a_profile_taken_during_a_save_still_counts_every_shot` passed 6 runs of 6. P9's torn read is in `ShotStore::put`, and the corpus does not read the shot store: `save_shot_analysis` and the manifest are each replaced whole. All that is left is a read between a retag's two writes, invisible in this view. It stays, for one rule across the phone's reads, and the test's comment and the module doc say what it does not show.
   - **Rewording `landings_text` fails nothing in `crates/app`**, since the hand cases' report is computed live and moves with it, and no `corpus/` case reaches a projected landing. `crates/core/tests/reports.rs` fails on it instead. So the gate catches a sentence moved on one side, and only the committed text catches one moved on both.
10. **For P16**: new shapes for the codegen: `Option<PlayerView>` and `Option<ShapeView>`, a struct inside an `Option` inside a struct. `app::profile_view` takes a `CareerCorpus` and is outside `api`, so if it appears in the generated Dart, `rust_input` is reading wider than C1 says.
11. **For P26**:
    - `player` is `null` with no shots at all: show `no_clubs` and no Player in the selector. `no_clubs` can also sit above a real `player`, when no shot names a club.
    - **`percent == null` is what says withheld, not the absence of `%`.** Consistency's criterion holds a `10%`, so its withheld text has one, and a widget test grepping for `%` would fail on Rust's own text.
    - A shape topic can have its `grade` withheld and `hit_shape` graded, or the reverse, and `draw-on-line-withheld` is the fixture. When `on_line_projected` is true, show `landings`, which holds the word.
    - A club in the bag with no shots is `bag_slots` minus `clubs`, a set difference and no golf. No string for it crosses, since the whole-profile report prints none.
    - `NumberRow.name` and `FindingView.metric` are contract names, `carry_distance_yds`, because the gate refuses a friendlier word, as `MeasurementRow.name` is since P9. If that reads badly on the screen, it is a decision to raise at P28 and not a label table to add in Dart.
    - The six captures: one club, 6 shots, one withheld topic (consistency), every number withheld, no blend and nothing listed.
12. **For P35**: item 5, if the user leaves it, belongs beside the plan's other recorded limits in ADR-034's addendum.

### P12 (2026-11-05)

1. **D6 is in `crates/lab/src/import_export.rs`, and each clause of the Done line has its pin.** `Plan::golfers_file` judges what `Plan::differs` (the plain rule's first half, split out of `file`) leaves over. `tests/import_export.rs` gains 6 cases and is now 19. Its verify line, `cargo fmt --check` and `tests/test_docs_truth.py` (91) pass, clippy names neither touched file, and exit codes are unchanged. **No existing test moved.** The one that names a golfer's conflict, `one_file_that_differs_refuses_the_whole_import_and_names_each`, stores a left-handed golfer, so it is still a conflict under D6 and passes unedited, with no "D6 moved it" comment to write. Only that file's header line changed. The dated line is under the M29 plan's C11.
2. **Calls the Do line left open**, each in the module doc:
   - **Another `player_id` in the same file is a conflict too.** D6 names only handedness. The Do line's "the same `player_id`" decides it.
   - **The line prints both values as well as the field's name**, because D6 says "the difference is printed" and the Do line says "naming the fields". So the line is a superset of both: `the export differs in display_name ("Aaron", stored "Aaron B"), created_at (…, stored …)`.
   - **A golfer can differ by bytes with no field differing**: spacing, a key the contract does not read, or one instant in two spellings, which `Timestamp` calls equal. It is `kept`, saying `no field it reads differs, only the file's bytes`. Pinned for all three.
   - **The rule reaches only a name directly under `golfers/` with a store's suffix.** A nested copy and `notes.txt` go by bytes, as `ShotStore`'s rule already does for a name it does not list.
   - **A bag is parsed on neither side**, so a torn bag on either side is still `kept`. A directory stored at a bag's name holds no bag to keep, and is a conflict.
   - **A `kept` file counts under `left as stored`**, the tally's existing word.
3. **One hole the new rule opened, closed.** `stored` learns a difference from the size alone and stops reading, and a `kept` file is never written, so a torn archive entry for a golfer or bag would have passed silently, where before P12 it at least refused as a conflict. The entry is now read through first, and a failed checksum is exit 2, as the module doc promises. Pinned on a real archive with one byte flipped, with the same archive whole as the control.
4. **Shown to fire.** Eleven mutations, each failing the test meant for it and the file put back (`cmp` against a copy taken first): handedness not compared, id not compared, each field not named, a directory counted as a bag, nested names reaching the rule, a torn entry not noticed, a kept file not tallied, an unreadable golfer on either side read as the other's, and the bag left a conflict.
5. **Run by hand through the real binary**, on a scratch export and lab: another `created_at` and another bag gave exit 0, two `kept` lines and `0 file(s) copied; 0 identical, 2 left as stored, 0 ignored.`, with the lab's bag unchanged. The export's golfer then turned left-handed gave exit 1, nothing written, naming the file.
6. **Outside the Touch line, one clause.** `crates/lab/src/bin/golf_lab.rs`'s doc said one file that differs refuses the whole import, which P12 made false, and P35's Touch does not name that file. It is a comment and no code moved. `crates/lab/src/lib.rs`'s bullet is still true, so it stayed.
7. **For P13**: the exact words, so the round trip can pin them. `kept      golfers/<id>.golfer.json: the store holds this golfer already, and it stays; the export differs in created_at (<export's>, stored <lab's>)` and `kept      golfers/<id>.bag.json: the store holds this golfer's bag already, which differs, and it stays; the phone's bag was not applied`. Each adds 1 to `left as stored`, so a lab holding the same golfer with another `created_at` reads `<n> file(s) copied; 0 identical, 1 left as stored`, and goes on doing so on every re-import, rather than settling to `identical`.
8. **For P31 and P32**: three things the rule does not do, reasoned from the code and not run on a phone.
   - **It is keyed by the file's name, which is the slug.** A golfer typed `Aaron B` on the phone is `aaron-b`, new at the lab, with no line saying it looks like `aaron`, and its shots pool under another id. So P31's run sheet should say to type the name as the lab has it.
   - **A kept bag means the lab's lofts are the ones a re-analysis uses.** `read_corpus` right after an import still equals the phone's, since `shot_analysis.json` moves as bytes, and P13's exit is about that. But `golf-lab reanalyze` reads its loft through `loft_for_club`, from the lab's `BagStore`, so a shot the phone analysed with its own loft can move at the lab's next re-analysis, and the printed line is the only sign. If P32's corpus check runs after a re-analysis, this is the first place to look.
   - **A golfer conflict's message names the path and not the reason.** `golfers/aaron.golfer.json -> …` covers another handedness, another id and an unreadable side alike, because the existing test pins that format exactly. If P32 meets one, compare the two files by eye.
9. **For P35**: two sentences P12 made false and its Touch did not name. `docs/ARCHITECTURE.md:166` ("one file that differs refuses the whole import"), and the program plan's §M38 at `docs/plans/m31-m40-shot-first-pivot.md:1433-1442` ("Golfers and bags arrive byte-identical or absent", and "a rewritten `session.json`, golfer or bag conflicts"). Line numbers are as of this phase.

### P13 (2026-11-05)

*Everything read off the six captures below is `provisional: pasted copies`, and every number is this Mac's and not a phone's.*

1. **The exit's desk half holds, and each clause of the Done line has its pin.** Six captures through `record_photo`, exported, imported by `golf_lab::import_export` into empty scratch dirs: `read_corpus` equal to the phone's, six swings, none excluded, and the three trees the phone's bytes. `tests/export_roundtrip.rs` is 31. `cargo test -p app`: 218 passed and 1 ignored, P6's timing line. P4's `--no-deps` clippy line, `cargo fmt --check`, the iOS build, `ort_reach`, `tests/test_docs_truth.py` (91), `cargo test -p golf-core`, and `golf-lab`'s `import_export`, `import_photos`, `reread` and `edges` all pass. `Cargo.lock` differs by two lines, `golf-lab` and `zip` under `app`. No vector, version key or bar moved.
2. **The user's decision is built: a number is never given out twice.** `NewPhotoEntry` gained `floor` (`0` is none, and the lab passes it), and `crate::history::delete` writes the entry's number to **`.app/deleted.json`**, the highest for each session, before it removes anything. The pin is the Decision's own: export, import, delete, another photo under the same date, and the next export imports, with the deleted shot still at the lab. It has a twin for a session's only shot, whose directory goes with it. **P10 finding 6's second limit and P10 finding 11's run sheet warning no longer apply**, and neither does the refusal P10 finding 7 asked to pin. `tests/shot_actions.rs`' test of the old rule is replaced by five: the number stays spent, outlives its session and binds no other, a read of nothing spends none, a delete that cannot write it removes nothing, and a torn file costs no shot.
3. **The plan missed one pin, and I changed it. It needs the user's eye.** `crates/lab/tests/edges.rs` read every manifest line and failed on any declaration of the lab, so the `golf-lab` dev-dependency this phase's Touch line asks for would have stayed red until P28's workspace run, since no Verify line here runs it. **It now skips `[dev-dependencies]` tables**, which is `ort_reach`'s own line and the reason the file gives (a dev-dependency never carries `ort` into what ships), and it gains `the_one_dev_edge_is_the_facades_round_trip`, so a second crate has to come and say why. Control, run: the same edge moved to `[dependencies]` fails both `no_crate_in_the_workspace_depends_on_golf_lab` and `no_crate_on_the_phone_path_reaches_ort` (`app -> lab -> ocr -> ort`). If the user would rather keep that pin as it was, the round trip has to move to `crates/lab/tests/`, taking `app` as the lab's dev-dependency: that reverses the edge and costs the plan's file name.
4. **Outside the Touch line, each for a reason.** `crates/app/src/export.rs`, since rule 7 and every phase before this keep a body out of `api`, with `api/mod.rs` and `lib.rs` naming it. `crates/app/src/history.rs` and `tests/shot_actions.rs`, for item 2 (a limit in the module doc became a rule, and `clock_of` went `pub(crate)`). `crates/lab/tests/edges.rs`, `crates/lab/src/lib.rs` and `crates/lab/Cargo.toml`, for item 3: one clause each in the last two, "nothing that ships depends on it".
5. **What was decided here that the plan left open, each open to challenge.**
   - **A phone with no golfer refuses to export.** The first draft of the test assumed the lab reads an empty zip as nothing to do, and it refuses one: exit 2, `it holds no file under sessions/, shots/ or golfers/`. So the phone says so first and hands the share sheet nothing. **A golfer with only failed reads does export**, because that session's diagnostics are the ones most wanted. Pinned both ways.
   - **`now` reaches only `ExportSummary::exported_at`, and an export writes nothing to the phone.** So two exports of one store are the same bytes, pinned with two different `now`s, and P27 saves `last_export_at` only once the share has gone (P5 finding 8).
   - **`shots_since` counts entries by `created_at` and does not subtract.** Two exported, one deleted, one more, reads 1 and not 0. A retag or a correction is not a new shot here, and C16's limit is what says those do not reach the lab anyway.
   - **The export holds `one_writer()` for the whole walk, and `export_status` holds nothing**, since it reads only manifests, each renamed into place. A torn shot refuses the whole import at the lab, and a wait costs a photo a few seconds at worst.
   - **A path inside the data root is refused**, lexically, or the next export would carry the last.
   - **A `.tmp`, a `.part` and a dotted name do not go**, by the lab's rule for a name, so a crash adds nothing to its tally. An empty `.incoming/` lands nowhere and counts as nothing (P3 finding 6), and `.app/state.tmp` is not walked (P6 finding 5).
   - **A torn `.app/deleted.json` reads as no floors**, `crate::state`'s rule, since it should not cost a shot at the bay. What that risks is one named refusal at the lab, and only a failing disk tears a file renamed into place.
6. **Shown to fire: 25 mutations of `export.rs`, `record.rs`, `history.rs`, `session.rs` and `edges.rs`**, each failing the test meant for it, the file put back and compared, plus item 3's control on the manifest. Two survived the first pass and changed a test:
   - **The lock taken out of the export passed the overlap test**, because a zip of a few files finishes between one writer's steps. `.app/failed/` sorts before `sessions/`, so the test now plants 40 MB there and each export spends its time between listing an entry and opening it. With the lock out it fails 3 runs of 3, `round N: could not write …: No such file or directory`. It also gained a direct pin, `an_export_waits_for_the_writer_holding_the_lock`, which depends on no writer landing anywhere.
   - **`<=` to `<` in the floor passed the test written for it**, whose last line was above the floor either way. That line now names the floor's own number.
7. **A trap for any later phase's mutations.** Restoring a mutated file with a tool that keeps its old modification time (`cp -p`, `shutil.copy2`) leaves cargo sure nothing changed, so the next honest run tests the mutated binary and fails for no reason. It cost one confusing run here, on both lock pins. Touch what you restore.
8. **Reasoned findings now run, and each held.** P7 finding 9: a correction after an import prints `kept` for the shot and `duplicate` for the entry, and an empty save leaves `identical`. P8 finding 7: a corrected shot keeps its value and its `corrections` entry through a `--force` re-read at the lab, replaying the capture's own boxes. P10 finding 6's first limit: a club, an intent and a verdict each print `duplicate`, move nothing at the lab, and leave the shot `identical`. P12 finding 7: both `kept` lines word for word, and the tally not settling. `tests/export_roundtrip.rs` pins all four.
9. **Limits, recorded and not fixed.**
   - **The zip's time is whole seconds.** A fraction is lost on the file at the lab and nowhere else, and only a `--force` re-read there, which stamps the shot from the photo, could see it. Pinned.
   - **The lab can still spend a phone's next number**, reasoned and not run: `golf-lab import-photos --into-session` into an imported phone session opens the next number at the lab, and that phone's next photo in the same session is other bytes at that path. The floor cannot know. Nothing in M38 does that, and the session names say whose they are.
   - **The lab keeps a shot the phone deleted**, so after a delete following an import the two corpora differ for good (P10 finding 6).
10. **For P14**: the zip is sorted by name with no directory entries, and it never holds a `.tmp` or `.part`, but `bars` also takes a directory, where P6 finding 8's rule on `*.json` still applies. `.app/deleted.json` stays home, so P10 finding 8 stands: nothing in an export says a delete happened. A golfer with only failed reads is a valid export with no `sessions/`, so `bars` should answer `INVALID` on it rather than fail.
11. **For P16**: new shapes, `ExportSummary` and `ExportStatus`, only `String`, `i64` and `Option`, and `export_status` returns its struct with no `Result`. `crate::export` is outside `app::api`, so it should not appear in the generated Dart. **`app/rust/Cargo.lock` gains `zip =7.3.0` and should gain no `ort`**, since cargo does not resolve a path dependency's dev-dependencies from outside its workspace. That is reasoned and not run, and `grep -c 'name = "ort"' app/rust/Cargo.lock` is the cheap check. C1's `lock_agreement.rs` already names `zip`.
12. **For P20**: Swift is free to send a fraction in a photo's time, and item 9 is what it costs.
13. **For P27**: call `export_sessions(root, <tmp>/….zip, now)`, then `share`, then `set_state` with **`summary.exported_at`** as `last_export_at` and **`summary.shots`** as `last_export_shots`, and not the clock when the sheet closed, since shots since are counted from when the zip was cut. `export_status(root)` holds the row: `last_export_display` (`2026-11-05 11:00`), `last_export_shots`, `shots`, `shots_since`. Dart subtracts nothing (rule 8). Show a refusal as its text, the likely one being `There is nothing on this phone to export yet.` `tmp/` is outside the root, so the path check passes, and it compares paths as written, so take both from the one `documentsDir` answer. A `.part` beside the zip exists only during the call.
14. **For P28**: `cargo test -p app` now builds `golf-lab` and everything under it, so its first build on a clean tree is the lab's, `ort`'s download included. The round trip adds 13 s, 2 s of it the overlap test.
15. **For P31**: drop P10 finding 11's warning, since item 2 removes its cause. P12 finding 8's line about typing the name as the lab has it stands. **For P32**: its corpus check imports into empty scratch dirs, so no earlier import can leave a deleted shot at the lab to make the two differ. An import into anything else can (item 9).
16. **For P35**: C7's list of `.app/` files gains `deleted.json`, which stays home. ADR-014's addendum takes C16's limit and P10 finding 6's first limit, and not its second, which is now a rule. **"Nothing depends on" `golf-lab` is now true only of what ships**: `CLAUDE.md`'s two sentences on `mcp` and `lab` as shells, `docs/ARCHITECTURE.md:392` and `docs/CONFORMANCE.md:1999` (line numbers as of this phase; `mcp` still has no dev edge). `crates/lab/src/import_export.rs`'s module doc still calls this `M38 P1's export_sessions` and `M38 P4's exit`, the program plan's numbers, where this plan's are P13 and P32.

### P14 (2026-11-05)

*Everything read off the six captures below is `provisional: pasted copies`, and every number is this Mac's and not a phone's.*

1. **`golf-bay-report bars` exists, and each clause of the Done line has its pin.** `tests/bay_bars.rs` is 64: the ten photos worked by hand in `ten_photos`' comment (105 read, 1 wrong, 13 withheld, 1 blank; T1 0, T2 105 of 119, T3 8 of 10 on `carry_distance`, T4 2 of 10, T5 4200 ms at rank 10 of 10), both sides of each limit, a wrong cell on a flagged photo left out of T1, a blank in no denominator, a failed read as twelve withheld and flagged, the p95 as nearest rank, and a scratch copy of the bars moving the verdict. `cargo test -p app`: 282 passed and 1 ignored, P6's timing line. P4's `--no-deps` clippy line, `cargo fmt --check`, both iOS targets, `ort_reach`, `golf-lab`'s `edges` and docs truth (91) pass. No dependency, version key, vector or bar moved. `spec/`, `spikes/`, `crates/screen` and `m38-bay-thresholds.md` are untouched.
2. **The plan has a hole here, and this phase put in the flag that closes it. It needs the user's eye. A bare `bars` on P32's zip gives the wrong verdict.** The file's session is "one sitting at the bay", the export is cumulative (C16) and a reinstall keeps the data (C20), so P30's rehearsal is still in the zip, and its step 2 photos are in-app camera photos of a monitor, which the log calls `camera`. They count towards the thirty, the clubs and all five bars. So `--since <RFC 3339>` leaves out every read before it, compared by instant and with the offset required, and the report always prints `read from … to …`, so a rehearsal left in shows. Pinned: `rehearsal_then_bay` reads `MISS` without it and `PASS` with it. Nothing changes without the flag, which is why this was not a stop, but P32's Do line has to be read with item 11's line.
3. **M33's box numbers come out of the phone's own store and log**, through `record_photo`, `log_read` and `export_sessions`: 248 boxes at 1.00 and 3 at 0.30, smallest margins **+0.337** and **-0.133**, no value moved and no flag a confidence's. A zip and its directory give one report. Stored as read, M33's `sidespin` not read is a *blank*. Typing a value in, as P31's run sheet asks, makes it the withheld cell, and then T2 is 71 of 72 and T3 is 5 of 6. **That is the file's definition doing its job, and it is why `bars` alone cannot be P15's control**: `bars` trusts the review, and `truth` is what holds the reader to M33's truth without one.
4. **D8's first datum, and it is against the confidence.** One of M33's three 0.30 boxes is behind a tile's cell, and that cell reads right: asked of the parser, 71 read cells and 1 of them with a box below 1.00 behind it, which the control now pins. M33's log calls all three strays, and its 0.294 OCR term on one photo is consistent with this box. So on these six a low confidence has sat behind a right cell and never behind a wrong one, of which there are none. The evidence is too thin to judge `W_OCR` by, which is P32's job.
5. **What was decided here that the plan left open, each open to challenge.**
   - **A photo is a digest with a line that is not `duplicate`, and its last such line speaks for it** (P10 finding 8). A failed read retaken with the same bytes is the retake's read.
   - **A photo with a `read` or `flagged` line and no stored shot was deleted at the phone, and it still counts for the session's size, T4 and T5, with no cells.** The file says a later review cannot change a flag, and a delete should not either, or tidying up at the phone improves T4. So a delete can leave thirty photos and twenty-nine shots, and the report names each. Leaving them out instead is one `continue`.
   - **A stored shot with no line** is a crash between `record_photo` and `log_read`. It is in no count, because nothing says it was a camera photo, and the report names it.
   - **The session's 30 and 2 are read from the file's prose, not only the table's limits**, so they have no second copy either, and a file that stops saying them is refused. A limit's unit is required (`at least 92` is refused), and a bar the report does not measure (`T6`) is refused rather than left unjudged.
   - **A ten-photo fixture is `INVALID`, so each bar still prints its number and PASS or MISS on an `INVALID` session**, and the verdict line is what says they judged nothing. P30 reads those numbers. "Judged" is `valid` and nothing else.
   - **A bar with nothing to measure is not met**, so a valid session cannot pass on an absence. It reaches T2 and T3 only if a person cleared everything.
   - **`--source` is added too** (camera by default; `library`, `scanner`, `all`), because P30 and P34 ask for it and neither may touch this code. Anything but `camera` is `INVALID` however many photos it holds, since a session is camera photos.
   - **"Whether any cell's read depended on one" is asked of the parser**, as M33 did, with no second copy of `build_cells`. The same boxes are read at every confidence 1.00 and 0.00 (a value moved; a flag only the confidences put there), and a box is behind a cell if taking it away moves that cell's text or value. It can overcount, since a label taken away can shift a neighbour, which errs towards a low confidence reaching a cell. Only boxes below 1.00 are asked about, which is what keeps a 36-photo zip of 60 MB at 0.96 s in release and 5.8 s in debug. **The line P32 quotes is `did a confidence separate a wrong cell from a right one?`**, answered `no wrong cell`, `yes`, `partly`, `no` or `unknown`, where `yes` means every wrong cell's lowest is strictly below every read cell's.
   - **A verdict exits 0**, `MISS` and `INVALID` too, since an answer is not a failure. A refusal exits 2 and a bad verb 1. A torn shot, a correction that is no number or a shot with no provenance refuses the whole report, because a number cannot be given for them. A torn log line, or a box file missing or unreadable, is a note, because the bars do not need boxes and one torn line should not cost a session's verdict.
   - **Only JSON leaves a zip**, into a scratch directory removed on drop, by hand since `tempfile` as a normal dependency would ship in the bridge's lock.
6. **Outside the Touch line, each for a reason.** `crates/app/Cargo.toml` for the `[[bin]]`, since the auto-discovered name would be `golf_bay_report`. `crates/app/src/lib.rs` for `pub mod bay`. `crates/app/src/record.rs`, where `spelled` goes `pub(crate)`, so a source has one spelling.
7. **Shown to fire: 48 mutations of `bay.rs`**, each failing a test, the file restored and compared. One survived the first pass and changed a test: a deleted photo's latency taken out of T5, so the delete pin now gives that photo the slowest time. The one branch no test can reach is `values_moved` holding a name, since `parse_screen` reads no confidence for a value. That line is M33's `confidence_terms` stop as a report, and it is there for the parser that changes.
8. **One slip of mine, in a test and not the code**: the fixture's printed median was written as 2200, where nearest rank 5 of ten is 2000. The median is printed for P32's eye, and no bar reads it.
9. **For P15**: `app::bay::Export::open(path)?.root()` is a data root for a zip or a directory, so `Dirs::of` and `read_corpus` work on it as they stand, and `read_corpus` reads no photo (checked by reading it, not by running it). `classify`, `Bars` and `nearest_rank` are `pub`, `cells_of` and `screen_fields` are private and can be widened in the same file, and `tests/bay_bars.rs`' `Phone` is the pattern for a store with real reads. A photo's `original_filename` is on the entry `entries_by_digest` holds.
10. **For P21**: `bars` reads only the log for the session's size, T4 and T5, so **call `log_read` on every status, `failed` included**. A failed read `log_read` misses is invisible, where a stored shot it misses is at least named. `at` is what `--since` compares, so write it with its offset (`log_read` refuses otherwise). **For P28**: this adds 8 s to `cargo test -p app`.
11. **For P30**: the line is `golf-bay-report bars <zip> --source all`. It says `INVALID`, and the numbers and letters are what is read. Every box that is a letter is listed with its `x,y`, so a stray can be told from the tile's, and the smallest margins are on the table's first line. **For P31**: record the install's **time with its offset**, not only the date, since item 2 needs it. Also tell the user the rehearsal's shots stay in History and Profile, and reach the lab at import, as a monitor's numbers: deleting them first is theirs to choose, and `--since` covers the report either way. **For P32**: the line is `golf-bay-report bars <zip> --since <P31's install time>`, then check that `read from … to …` is one sitting. `compare` takes no `--since`, since it compares whole corpora. The results file's "success and review rates" are the line under `Photos:`. **For P34**: the two columns are `--source camera --since T` and `--source scanner --since T`, with `T` that install's time. Both say `INVALID` by construction, ten shots being no session, so read each bar's PASS or MISS.
12. **For P35**: ARCHITECTURE §1's `golf-bay-report` entry takes `--source` and `--since`, and `crates/app` holds a bin.

### P15 (2026-11-05)

*Everything read off the six captures below is `provisional: pasted copies`, and every number is this Mac's and not a phone's.*

1. **The control holds, and each clause of the Done line has its pin.** P6's store over M33's committed `raw` boxes: **71 of 72, 0 wrong, 0 caught, 1 withheld, `sidespin` lowest at 5 of 6, 0 flagged**, labelled `provisional: pasted copies`, with the log written and with none. The totals could agree by luck, so `every_cell_is_the_one_m33_recorded_to_the_bit` holds all 72 cells to `results/C0.json`'s `raw` arm: its word, the value read and the truth, each by `==`, so the conversion here is `truth()`'s to the bit. `compare` says equal on P13's round trip, from the zip and from the phone's directory, and a club changed in a scratch copy of the lab names `swings[2].club`, `"7i"` against `"8i"`. `tests/bay_truth.rs` is 41 and `bay.rs` gains 6 unit tests. `cargo test -p app`: 329 passed and 1 ignored, P6's timing line. P4's `--no-deps` clippy line, `cargo fmt --check`, the iOS build, `ort_reach` and docs truth (91) pass. No dependency, version key, vector or bar moved. `spec/`, `spikes/`, `crates/screen` and `m38-bay-thresholds.md` are untouched.
2. **What was decided here that the plan left open, each open to challenge.**
   - **`truth` and `compare` exit 3 on a miss, where `bars` exits 0.** `bars` judges, and a `MISS` is an answer. These two hold a thing to a known answer, and P28's `whole_flow.sh` has to stop on them, so a compare exiting 0 on a difference would be a trap. For `truth` a miss is a wrong cell on an unflagged photo or a capture with no cells. 2 is a refusal and 1 a bad verb, as P14 left them. The words print either way.
   - **`truth` holds what the reader gave, so a review cannot move it.** A wrong number corrected to the truth is still wrong, a value typed in is still withheld, and a right number typed over is still read. That is P14 finding 3's reason for this verb.
   - **A flag is the log's, by the bars file's rule, and the stored one only where there is no line.** A save clears the stored one, which would turn a caught cell into a wrong one. A note names each photo read that way, and that note is expected on P18's road.
   - **`caught` is kept from M33**: a wrong number on a flagged photo. It costs the read share and does not stop the report holding.
   - **It judges no limit.** T1 to T4 are named and no bar is passed or missed, since six photos are no session and no limit should get a second reader.
   - **Two entries under one stem are named and neither is chosen**, and a capture deleted at the phone (a `read` or `flagged` line and no shot) holds no cells. Each is in no denominator, and like a capture not found, stops the report holding: `0 wrong` over five of six is not the control's answer.
   - **A failed read with no line is known by its boxes**, since P18's road logs nothing. The only other way to that state is a delete whose line a crash ate, and reading that as a retake costs that capture's cells and hides nothing.
   - **A failed read keeps no name**, so a re-encoded capture that failed to read cannot be found. The report says how many of the log's failures could be it, only when a capture is missing.
   - **The truth's conversion is typed in `TruthRow` and read from nothing the reader uses.** Taking the sign rule from `profiles.json` would have the oracle agree with whatever it checks. A device declaring a thirteenth field is refused, and a bad letter, a missing column or row, another device and an empty photos file are each refused by name.
   - **`compare` is exact, and does not call `golf_core::compare`**, whose float tolerance is for two implementations adding one measurement up differently. Here one reader reads one set of bytes twice. It keeps that comparator's path spelling, and a list of two lengths parts at the list.
   - **Two empty corpora are refused and not called equal**, because an unknown golfer reads as an empty corpus, so a mistyped `--player` would answer the milestone's exit with a comparison of nothing. A `--sessions-dir` that is no directory is refused for the same reason. One empty side is a real difference.
3. **`first difference` is first in sorted key order, so it is often an aggregate and not the edit.** A missing shot first shows as `metric_counts.ball_speed_mph`, 6 against 5, and a club changed on `swings[0]` as `metric_counts.carry_distance_yds`, 6 against 5. The second is most likely `flag_auto_mishits`, whose floor is cut per club from the carries, so moving a shot out of the 7-iron moves it: that is reasoned from the function and the count, and not traced to a photo. Both are true differences, and the same edit on `swings[2]` shows itself. So a difference's line also prints each side's swings and excluded, which is what says a whole shot is missing.
4. **`truth_report` returns `Result<TruthReport, String>`**, where C2's table shows a bare `TruthReport`. Rule 7 asks for that wherever a call can refuse, and this one reads two files. **`TruthReport` also carries `text`**, the whole table, because Dart formats nothing (rule 8), and `held`, which is no wrong cell and every capture found.
5. **Outside the Touch line.** `crates/app/src/api/mod.rs`, for `pub mod bay`. Inside `bay.rs`, two refactors of P14's code, both held by its 64 tests: `read_log` under `read_lines`, because "so no photo can be counted" is false of `truth`, which still has the stores, and `lowest_field` lifted out so T3 has one tie rule.
6. **Shown to fire: 36 mutations of `bay.rs` and the bin**, each failing a test, each file restored and its digest compared. None survived. They included each sign, the apex's division, the log's flag and the stored flag, the digest, name, deleted and absent routes, the exit code and both clauses of `held`.
7. **A trap for any later phase: this shell is zsh, and `echo ======` fails** with `(eval):1: ===== not found`, since a leading `=` is expansion there. It takes down a compound command at that point, so use a different separator.
8. **For P16**: new shapes in `app::api::bay`, `TruthRoute` and `TruthCellKind` (plain enums), `TruthCell`, `TruthPhoto`, `TruthField` and `TruthReport`, of `String`, `i64`, `f64`, `bool`, `Option` and `Vec`, and `truth_report` is the one function. They derive `Serialize`, as `ReadLog` does, and `TruthReport.text` carries `#[serde(skip)]`, the first field-level serde attribute in `api`. I expect the codegen to ignore it, which is reasoned and not run, so `app/tool/codegen_check.sh` is the check. `crate::bay` stays outside `app::api`.
9. **For P18**: call `truth_report(root, photos, truth)` with `spikes/vision-ocr/photos.json` and `truth_full_swing.json` under `GOLF_REPO`, and print `report.text`, which already holds the label. **Rule 11's number is `report.wrong`**, and `caught` is not it. T2, T3 and T4 are `read`/`judged`, `lowest_field` with `lowest_read`/`lowest_judged`, and `flagged`/`matched`, to go beside `results/C0.json`'s. No `log_read` is needed, and the one note about read lines is then expected. An `Err` is a file, not a read.
10. **For P20**: P6 finding 12 now has a test behind it. The fallback compares the stem of `original_filename` to the stem of the capture's `path`, exactly and case-sensitively, so if `loadFileRepresentation`'s temporary name is a UUID and not the asset's, the fallback finds nothing and P30 reads `not in this export`. Keep the asset's own name.
11. **For P28**: the lines are `golf-bay-report truth <zip> --photos spikes/vision-ocr/photos.json --truth spikes/vision-ocr/truth_full_swing.json` and `golf-bay-report compare <zip> --sessions-dir <scratch>/sessions --player <player_id>`. Both exit 0 when held, so `set -e` carries the script. The test's one correction moves neither: `truth` reads as read, and the zip is cut after it. This adds 24 s to `cargo test -p app`.
12. **For P30**: step 2's and 3's photos show as one note, `N other photo(s) in the export match no capture`, and move no number. **`truth` exiting 3 is not by itself rule 11's stop**: a capture Photos renamed also exits 3, reading `not in this export`, and the stop is `T1 wrong cells on photos not flagged` above 0, so read the words. A capture found by name reads `name: a re-encoded copy`. That label is only an inference from one name holding other bytes: a stock-camera `IMG_2889.HEIC` from step 3 would be taken for one if the capture's digest were lost, and it would show as a row of wrong cells under that label.
13. **For P32**: `compare` says `Corpus of <player>: equal, …` on its last line, and takes the `player_id` and not the name. Item 3 is how to read a difference.
14. **For P35**: ARCHITECTURE §1's `golf-bay-report` entry names three verbs and four exit codes, 0, 1, 2 and 3. `crates/app/src/api/` gains `bay.rs`, which C2's table already lists.

### P16 (2026-11-05)

1. **The app runs on the Simulator with the real facade behind it, and every clause of the Done line was run.** `app/tool/sim_test.sh integration_test/smoke_test.dart` passes, with `about()` answering the three version keys and the device through the bridge. `flutter analyze` is clean, `dart format` changes nothing across 14 files, `flutter test` passes, `codegen_check.sh` passes, and `lock_agreement` is 7 passed. **The root `Cargo.lock`'s digest is `28c62c5f…` at this phase's start and end.** `git status` shows `.gitignore` and 70 untracked files under `app/`, none of them in a generated build directory. `cargo test -p app` passes 336, the `--no-deps` clippy line, `cargo fmt --check`, `golf-mcp`'s and `golf-lab`'s `edges`, `ort_reach` and docs truth (91) pass. The built `Runner.app`'s `Info.plist` reads `Golf Coach`, `dev.golfcoach.lab`, minimum OS 16.0, and the camera sentence.
2. **The codegen refused nothing, so `crates/app/src/` has no edit from this phase.** Every shape the "For P16" items named bound as it stands: `Option<(String, String)>` in a struct is `(String, String)?`, and `Vec<TargetShape>`, `Vec<FieldEdit>`, `Option<PlayerView>`, the plain-enum arguments and `Result<(), String>` all passed. **`#[serde(skip)]` on `TruthReport.text` does not reach the bridge**: Dart gets `final String text`. It binds 24 functions, C2's 23 and the init. None of `parse_now`, `Dirs`, `crate::record`, `crate::card`, `crate::history`, `crate::export`, `crate::bay` or `app::profile_view` is in the generated code, so P4 finding 6's question is answered: `rust_input` reads what C1 says.
3. **The first resolve left the locks apart, as this plan's spike warned.** `serde` 1.0.193, `serde_json` 1.0.109 and `unicode-normalization` 0.1.26 against the root's 1.0.229, 1.0.151 and 0.1.25. `cargo update -p <crate> --precise` in `app/rust` fixed all three, and `serde_derive` with them. `sha2`, `flate2` and `zip` agreed already. **36 other packages still differ, and I read the list for what could decide bytes.** `miniz_oxide` is 0.9.1 behind `flate2` in both locks (the bridge's 0.7.1 is `backtrace`'s), `crc32fast` differs at 1.5.2 and 1.5.4 and computes a fixed function, and `zmij` and `itoa` agree. `crypto-common` and `typenum` differ under one `sha2`, and they only carry its types. The rest are FRB's own stack (`tokio` 1.34.0 among them), proc macros, or plumbing with no output of its own, such as `libc` and `cfg-if`. `serde_json`'s `float_roundtrip` is on in the bridge by `cargo tree`, since `serde_json.workspace = true` resolves against the root's table even from outside. `grep -c 'name = "ort"' app/rust/Cargo.lock` is 0, confirming P13 finding 11, and **`the_bridge_lock_holds_no_ort` now holds that**, beyond C1's list, because `ort_reach` cannot see this lock.
4. **Each guard was shown to fire, and each change put back.**
   - **The smoke test, on `analysis_version: CAREER_VERSION`**: `Expected: <17>, Actual: <3>`, exit 1, and the Simulator down afterwards. **This also answers a question the plan did not ask: the build hook recompiles a change in `crates/app`, a path dependency outside `rust/`**, with a 10.5 s Xcode build, and again on the way back. So a Simulator run cannot silently test stale Rust.
   - **`lock_agreement`, on the real bridge lock at `unicode-normalization` 0.1.26**: failed, naming the crate and the `cargo update` that fixes it. Four controls on written locks also pin a second version in one lock, a crate missing from a lock, and a name that only starts the same.
   - **`codegen_check.sh`, four ways.** A shim generator writing a new line each run: "differs between two runs". `rust_input: crate::api,app`: names 72 wire functions from outside `api`. A compile error added to `app/rust/src/lib.rs`: the generator exits 0 saying "Done!" and prints no error, and the script fails. A module added and removed: 25 functions and then 24. **The removed module's Dart stays on disk**, and `flutter analyze` fails on it.
5. **`codegen_check.sh` does two things beyond the Verify table's sentence, both because of item 4.** It compiles the bridge, since the generator's exit code does not say, and it fails on a wire function outside the two `api` modules. The width check runs before the compile, because a widened line also stops compiling, with 133 errors that do not say why. A run costs about 14 s.
6. **Beyond the Do list, each for a reason.**
   - **`integration_test/simple_test.dart` and the generated `lib/src/rust/api/simple.dart` went with `simple.rs`**, since both call `greet`.
   - **`test/app_test.dart` is new**, since `flutter test` needs a test and the template's `test/` is empty.
   - **`cupertino_icons` is gone** by rule 10, and `flutter_lints` is exact. `flutter pub get` moved nothing else in `pubspec.lock`. `flutter_rust_bridge_hooks` stays in `dependencies` where the template and the spike have it. It is the build hook's, and no runtime dependency in rule 10's sense.
   - **`RunnerTests`' id is `dev.golfcoach.lab.RunnerTests`**, and the iPad key is portrait-only too.
   - **The smoke test reads each version key from `crates/contracts/src/` through `GOLF_REPO`**, so a bump does not have to find a copy here. Today's three values differ, which is why a crossed field fails.
7. **The template's gitignores miss nothing that a build writes.** So the root `.gitignore` gained one thing, `app/ios/Flutter/Signing.xcconfig`, and `git check-ignore` confirms it. **For P29**: the include was run with a made-up team in that file: `xcodebuild -showBuildSettings` reads `DEVELOPMENT_TEAM` in Debug and Release, and the file was then removed. The Simulator builds with no such file. `cargo check --target aarch64-apple-ios --lib` in `app/rust` passes on the bridge's own lock, which is not a device link, and that has not been tried.
8. **For P17, P18 and P20**: `AppDelegate.swift` is already C4's shape, `FlutterImplicitEngineDelegate` with `didInitializeImplicitFlutterEngine(_ engineBridge:)`, calling `GeneratedPluginRegistrant.register`, and keep that call. `SceneDelegate.swift` and the `UIApplicationSceneManifest` exist, so an active window scene is there for P20's presenting. `Runner.xcodeproj` holds the three `Runner` Swift and Objective-C sources and no others, so C4's four hand edits are the first. A first `sim_test.sh` took 66 s with a 35 s Xcode build, and a change in `crates/app` about 10 s of build.
9. **For P19**: `GolfApp` in `lib/main.dart` and `test/app_test.dart` are placeholders to replace. Imports are `package:golf_app/src/rust/third_party/app/api/<file>.dart`, with `frb_generated.dart` beside `third_party/`. Every bridged call is a `Future`, `about()` and `all_clubs()` included, and arguments are named (`listSessions(root: …, playerId: …)`). An `i64` is `PlatformInt64`, which is `int` on iOS.
10. **For any phase removing a module from `crates/app/src/api/`**: delete its Dart under `lib/src/rust/third_party/app/api/` by hand, by item 4. **For anyone editing `Info.plist`**: `PlistBuddy` drops apostrophes from a value it is given, which is how the camera sentence first read "monitors", so use `plutil -replace`.
11. **For P35 and P36**: P1 finding 5's two READMEs are deleted on purpose, since neither declares a tier (rule 13) and neither says anything about this app. So `app/` holds no `.md`, `app/README.md` is P35's to write, and P36's count gains only what P35 writes. Generated code stays committed: `lib/src/rust/` and `rust/src/frb_generated.rs`. So do `pubspec.lock` and `app/rust/Cargo.lock`, and the build needs all of them, since the hook does not regenerate.

### P17 (2026-11-05)

Resumed: the first attempt was cut off by a usage limit after writing only the 🔄 marker, and `find -newermt` listed nothing else, so the phase ran from the start.

1. **The reader that ships gives M33's boxes to the bit, provisional: pasted copies.** `app/tool/reader_check.sh` reads all six captures through `app/ios/Runner/ScreenReader.swift` itself: **0 of 251 boxes differ**, by `==` on every number after the JSON trip, and the box rows are byte-identical to `spikes/vision-ocr/boxes/C0/raw/<id>.json` as well, beyond the gate. Width, height, `config` and `revision` match on each. Both systems read `26.2 (25C56)`. Each capture's sha256 is held to `photos.json` before it is read, and the six are found from `boxes/C0/raw/`, not listed.
2. **The in-memory decode equals the spike's PNG round trip, so C5's fallback is not built.** The app renders through `CIContext.createCGImage(… .RGBA8, sRGB)` and hands that image straight to Vision and to `rgb`, and nothing differs. The file's header says so.
3. **The orientation check passes, and its control says it can fail.** A drawn 1600x1000 frame, stored turned as 1000x1600 with Orientation 6 in a JPEG, reads as 1600x1000. `CARRY` lands (1.70, 0.08) px from where it was drawn upright and `TOTAL` (−0.45, 1.44), against the spike's 5% limits of 2.41 and 2.44. The same bytes with no tag read as 1000x1600.
4. **A Simulator build compiles the file, but not through the Verify line's command.** `flutter build ios --simulator --debug` stops in the native-assets hook before any Swift is compiled: `The rust-toolchain.toml does not include the target x86_64-apple-ios`. It builds both Simulator architectures, and C3 names three targets with no `x86_64-apple-ios`. **It fails the same way with the project file as it was before this phase**, so the reader is not the cause. P16 never ran it, since its builds were `sim_test.sh`'s. So I met the Done clause through `sim_test.sh integration_test/smoke_test.dart`, which builds arm64 for the "iPhone 17" Simulator and passed. The built `Runner.debug.dylib` holds 154 `ScreenReader` symbols. C3 stands, because adding a fourth target is a call for the user and not a runner. **For P33**: its Verify names the same command, so either use `sim_test.sh`'s build, or the user adds `x86_64-apple-ios` to `app/rust/rust-toolchain.toml` (and `rustup target add`).
5. **Each guard was shown to fire, and the file was restored and compared by `cmp` after each.**
   - `rgb`'s rank from `n * 3 / 4 + 1` failed 251 of 251 boxes.
   - y taken from `minY` failed 251 of 251.
   - Language correction on failed 53 boxes over the six. `IMG_2891` read 39 boxes for 41, and on the others `→YAMAHA` read as `→ YAMAHA` and `+9` as `+ 9`, with confidences of 0.5.
   - Ties ranked in reverse row-major order failed 214 of 251 on `rgb` alone, which is the log's "For M38" item 3, measured: the tie rule decides most of the colours.
   - Dropping `.applyOrientationProperty` failed only the orientation check, with three failures, and the six captures did not move. **So the pasted copies carry no Orientation tag, and the drawn fixture is the only proof of the turn until P30's HEICs.**
6. **Beyond the Do list, each for a reason.**
   - **Everything is inside `enum ScreenReader`**: `ScreenReader.Failure`, `.Pixels`, `.Box` and the functions. Swift has no file scope for types, and the spike's `Box`, `Pixels` and `sRGB` would otherwise meet names in `PhotoTime.swift` and `PhoneChannel.swift`.
   - **The answer has one key the spike's does not, `platform`**: `macOS`, `iOS Simulator` or `iOS`, fixed when the file is compiled. On the Simulator, `os` alone may not say which system read the photo, since `kern.osversion` is the kernel's build. Rust ignores the key (P13 finding 10). The spike's other keys and their order are unchanged.
   - **`jsonNumber` throws where the spike's precondition stopped**, so a number that is not finite is a failed read with a sentence and not a crash. No finite box gives one.
   - **`osString` decodes the build up to its zero byte, rather than with `String(cString:)` on an array**, which Swift 6 deprecates. The text is the same. The file typechecks with no warning for iOS 16.0, device and Simulator, in Swift 5, which the Runner uses, and in Swift 6 too.
   - `reader_main.swift` is `@main` under `-parse-as-library`, because swiftc allows top-level code only in a file named `main.swift`. `reader_check.sh` builds with `-O` and Swift 5, as the spike was built, into `target/m38/reader-check`, and runs in about 4 s with the compile.
7. **For P18 and P21: `msVision` is the whole call**: the decode, Vision, `rgb` and the JSON, timed from entry to return. So `ms_vision` is the Swift side's share of `ms_total`, beside `ms_rust`, and a slow decode cannot hide between the three. On this Mac it was 118–274 ms a capture, the first the slowest. `ScreenReader.read` is synchronous, so **P18's channel must run it off the main thread** (for example `DispatchQueue.global(qos: .userInitiated)`), and reply on the main one. The spike measured 1.26 s on the Simulator, and the `CIContext` is one static, shared and made on the first read. **For P18**: record what `os` and `platform` read on the Simulator, which this phase could not see.
8. **For P20**: `reader_check.sh` compiles a fixed list of two files. Add `app/ios/Runner/PhotoTime.swift` to its `swiftc` line, and its check to `reader_main.swift` (one `@main` only). **For anyone adding a Swift file to the Runner**: this phase used the ids `37A2CB05103A499F1824FE8F` (the file reference) and `2FA4FC37AEA99F1E4F60B8F0` (the build file), drawn by `openssl rand -hex 12 | tr a-f A-F` and checked absent first. `plutil -lint` on `project.pbxproj` is a quick check that the four edits left it well formed.

### P18 (2026-11-05)

*Everything read off the six captures below is `provisional: pasted copies`, and read on the Simulator, not a device.*

1. **The Simulator's iOS Vision leaves no wrong cell, so rule 11's stop does not fire.** `app/tool/sim_test.sh integration_test/vision_gate_test.dart` passes: each capture goes through `readScreen` on the `golf/phone` channel and then `record_photo`, as a library photo will, and then `truth_report`. The table, with macOS beside it. The macOS column is P15's control run in the same test into a second root, M33's committed `raw` boxes through the same code, so no number of M33's is copied:

   | | Simulator, iOS Vision | M33, macOS (`results/C0.json`) |
   |---|---|---|
   | T1 wrong on unflagged photos | **0** (caught 0) | 0 (caught 0) |
   | T2 read cells | 71 of 72 (98.6%) | 71 of 72 (98.6%) |
   | T3 the lowest field | `launch_direction`, 5 of 6 (83.3%) | `sidespin`, 5 of 6 (83.3%) |
   | T4 flagged photos | 0 of 6 (0.0%) | 0 of 6 (0.0%) |

   All six matched by digest, `held` true on both sides. **The one withheld cell moved.** The Simulator missed `2026-10-07-1`'s lit `R` beside launch direction `3.5`, and the cell is withheld, not guessed. It also read `IMG_2893`'s lit `D`, which macOS missed, so that sidespin reads. The whole table, every read and the box comparison are in `target/m38/sim-c0.txt`, and each Simulator box file is in `target/m38/sim-c0/<id>.json`. Both are gitignored, and the test rewrites them on every run. The report's one note, that no photo has a read line, is the one P15 finding 9 expected. `flutter analyze` and `dart format` are clean, and `flutter test` and docs truth (91) pass. No Rust, no `crates/app/src/api/`, no vector and no bar moved.
2. **The Simulator's Vision is not macOS's to the box, although both report `os` `26.2 (25C56)`.** That holds on all six captures. **So the Do line's comparison is measured and not asserted.** My first run asserted it and failed on it, with T1 at 0. The Done line asks only for T1, and rule 11's stop is a cell, not a box. Over the six:
   - **Boxes**: 247 against M33's 251, with the count differing on four captures (42/42, 41/41, 42/43, 40/39, 41/43, 41/43).
   - **Order**: different on every capture, so **boxes are paired by text** (each of M33's with the nearest unpaired Simulator box reading the same) and not by index. Pairing by index compares unrelated boxes as soon as one is missing.
   - **Position**: 236 pairs. 165 are within 1 px, 224 within 5 px, and the furthest is 13.8 px (`IMG_2891`).
   - **Colour and confidence**: on the pairs, `rgb` is within 14.4 of macOS's, and the confidence is the same on all 236.
   - **Texts read on one side only**: 15 by macOS alone and 11 by the Simulator alone, the boxes left unpaired. Spacing (`TOTALYD`/`TOTAL YD`), a last letter (`BALL SPEED MP.`/`BALL SPEED MPH`, `APEX FT`/`APEX FU`, `TO HOLE AU`/`TO HOLE FU`), the brand line, unlit letters, the lit letters of item 1, and a lit `R` on `IMG_2889` that only the Simulator read and that moved no cell.

   **The Simulator's read is deterministic.** Three runs gave byte-identical box files on all six, and the same table apart from the mutation in item 4. **If the user wants the plan's 1 px comparison to hold, it cannot on the Simulator.** That is the Goal's own warning ("the Simulator's framework, and not a device's"), now measured.
3. **P17 finding 7's question: on the Simulator `platform` reads `iOS Simulator` and `os` reads `26.2 (25C56)`, the same string as macOS's.** `operatingSystemVersion` gives the iOS runtime's 26.2, which happens to equal this Mac's macOS, and `kern.osversion` is the host kernel's build. So `os` alone cannot tell the two readers apart here, and the `platform` key is what does.
4. **Each guard was shown to fire, and the file was restored and compared by `cmp`.** A quiet wrong number: `bay_truth.rs`' `9899` to `9898`, in `IMG_2889`'s Simulator boxes before `record_photo`. It read `T1 wrong cells on photos not flagged: 1` and `IMG_2889 backspin: wrong, as read 9898.0, truth 9899.0`, and the test failed with `Expected: <0> Actual: <1>`, "rule 11: a wrong cell on a photo nothing flagged". The first run's failing box assertion (item 2) showed the comparison has teeth.
5. **Timing, Simulator only.** Swift's `msVision`, the whole call, was 1422 to 1872 ms a capture, the first the slowest. That is against P17's 118 to 274 ms on macOS and the spike's 1.26 s. Rust's `ms_rust` was 18 to 26 ms. It says nothing about T5's latency on a phone. A cached `sim_test.sh` run of this test took about 47 s.
6. **What was decided here that the Do line left open, each open to challenge.**
   - **The channel's contract.** `readScreen` takes `{path}` and answers `{json, msVision}`. The read runs on `DispatchQueue.global(qos: .userInitiated)` and replies on the main queue, per P17 finding 7. A photo that opens and holds no text answers empty `boxes` and is no error. One that cannot be opened is a `FlutterError` with code `unreadable` and `ScreenReader`'s sentence, which reaches Dart as a `PlatformException`. A missing path is `bad_arguments`. `documentsDir` answers the container's `Documents` path only, since C7's `processed/` under it is Dart's to name. The test checks that the directory exists.
   - **`lib/src/phone/phone.dart`**: an abstract `Phone` with `documentsDir()` and `readScreen(path)`, a `ScreenRead {json, msVision}` that Dart never parses (rule 9), and `ChannelPhone`, `const` and not `final`, so that C22's `TestPhone` can extend it, with `static const channel`.
   - **`PhoneChannel.swift` is an `enum` namespace**, as `ScreenReader` is, registered beside `GeneratedPluginRegistrant` in `didInitializeImplicitFlutterEngine`. That registrant is what reaches `integration_test`'s plugin, the only plugin here, so it stays. The Xcode ids are `632BA469ABCB79499E51FB21` (the file reference) and `4659D67532D5A854D4C2BC05` (the build file), checked absent first, and `plutil -lint` passes. `PhoneChannel.swift` imports Flutter, so it is not in `reader_check.sh`'s `swiftc` list, by C6.
   - **The test copies each capture into its scratch root first** (P6 finding 11), deletes the scratch on the way out, and states `positionLimit` as a constant.
7. **For P20 and P27**: add each new method as a `case` in `PhoneChannel.register`'s switch and as a method on `Phone` and `ChannelPhone`. A long call follows `readScreen`'s pattern: off the main queue, replying on it. A Swift file of their own needs C4's four hand edits (P17 finding 8 for the ids).
8. **For P21**: a `PlatformException` from `readScreen` is neither `Failed` nor a refusal. `record_photo` is never called, so there is no card, no status and no log line. RecordFlow has to say the sentence itself, and decide whether it counts toward the bay's denominators, as P6 finding 8 decided for an `Err`. `ScreenRead.msVision` is `ReadLog.ms_vision`.
9. **For P21's and P28's integration tests: on the Simulator, expect the Simulator's numbers in item 1 and not M33's.** In particular, `2026-10-07-1`'s `launch_direction` is withheld and `IMG_2893`'s `sidespin` reads, and a box count or position from `spikes/vision-ocr/boxes/` will not match. `golf-bay-report truth` over a Simulator export should still exit 0 (`held`), with T3 on `launch_direction`.
10. **For P30**: the phone is a third Vision. Against macOS the Simulator moved boxes by up to 13.8 px, read 15 of macOS's texts differently or not at all, three lit letters among them, and colours on the pairs by up to 14.4, all with no wrong cell. So read a device's differences from macOS as expected, and a wrong cell as rule 11's stop. Item 1's lit `R` and `D`, and item 2's on `IMG_2889`, about (249, 228, 64), are where to look first for the `LIT_BLUE_RATIO` check. They are yellow, `rgb` about (249, 217, 79) and (249, 210, 21), against an unlit (165, 161, 152).

### P19 (2026-11-05)

1. **The app opens on a real first-run setup and then four tabs, and every clause of the Done line was run.** `flutter analyze` is clean, `dart format` changes nothing over 25 files, `flutter test` passes 27 (the shell and theme's 6 in `test/app_test.dart`, setup's 21 in `test/setup_test.dart`), and `app/tool/sim_test.sh integration_test/setup_test.dart` passes, three runs of three after item 5's fix, in about 7 s of test after a 9 s Xcode build. Docs truth passes 91. No Rust and nothing in `crates/app/src/api/` moved, so `codegen_check.sh` did not run, and neither did a `cargo` line. The new files are `lib/src/core/core_api.dart`, `lib/src/state/app_model.dart`, `lib/src/ui/{theme,shell}.dart`, `lib/src/screens/{setup,bag}_screen.dart`, `test/{fakes,setup_test}.dart` and `integration_test/setup_test.dart`. `lib/main.dart` and `test/app_test.dart` were P16's placeholders, rewritten in place rather than deleted (rule 2).
2. **What the model is, for every later phase.** `AppModel` (`ChangeNotifier`) takes `core`, `phone`, an injected `Random` (`Random.secure` by default) and a `clock`, and `GolfApp(model:)` calls its `start()` once. It reads `<documentsDir>/processed` as the root, calls `getState`, and calls `initState` with `mintInstallId(random)` **only when no id is stored**. It holds what Rust answered and replaces it with each answer (P5 finding 8): `state`, `clubs`, `golfers`, `golfer`, `bag`. **`Stage` is read from the files at `start()`**: no golfer that `state.player_id` names and a file holds is `golfer`, a golfer with an empty bag is `bag` (D7: the picker would list nothing), and anything else is `ready`. After that only setup's own steps move it, so a relaunch part way through setup resumes there, and emptying the bag later does not throw the app back into setup. A failed start is `Stage.failed` with the error's text on a plain screen. Setters: `createGolfer`, `setSlot`, `removeSlot`, `finishSetup`, `editState(edit)` and `setAppearance`. **For P21, P23 and P27**: `editState` with `AppStateEdits.copyWith` is the one way to change a setting. `copyWith` takes a getter for each nullable field, so `club: () => null` clears it and leaving the argument out keeps it, and it has no `installId` (P5 finding 4).
3. **`GolfCore` holds setup's nine functions and nothing else**, each one line in `RustCore` with the facade's name and named arguments. `core_api.dart` re-exports the generated setup types, so a screen imports one file. **A facade `Err` reaches Dart as a bare `String`**, as the spike said: with `createGolfer`'s wrap taken out, the integration test died on "The following message was thrown". So `RustCore` turns it into `Refusal(message)` once, and **a screen catches `on Refusal` and shows `.message` as it stands**. A panic or a bridge failure is not one, and surfaces as itself. **For P21, P22, P25, P26 and P27**: add each new facade call to `GolfCore`, `RustCore` (through `_refusing` if it returns a `Result`) and `FakeCore` in `test/fakes.dart`. `FakeCore` holds no store rule. It logs `calls` and `nows`, and refuses only what a test names (`refusedNames`, `refusedLofts`), so a test asking for Rust's sentence on screen is asking for the one it gave the fake.
4. **`now()` and `rfc3339(DateTime)` are in `core_api.dart`**: microseconds and the device's `±HH:MM`, never Dart's offsetless `toIso8601String`. `test/setup_test.dart` pins the shape, a `DateTime.parse` round trip to the same instant (which proves the sign on this Mac's negative offset), and UTC as `+00:00`. The integration test proves `parse_now` reads it, since `create_golfer` and `set_bag_slot` take it. **For P20 and P21**: this is for a `now` argument. A photo's `captured_at` is Swift's to answer (C9).
5. **For P21, P22, P25 and P28's integration tests: the clock is real there, so wait, and settle before every tap.** `integration_test/setup_test.dart` has `pumpUntil(tester, finder)`, which pumps 50 ms frames for up to 20 s, because `pumpAndSettle` does not wait for a bridged call. The first passing run was luck: the second found **two `Save` buttons**, a closing loft dialog and the next one, and failed on an ambiguous tap. So `setLoft` calls `pumpAndSettle` before opening a dialog and after it opens, and the test settles before tapping `Done`, since a tap during either animation can land on the barrier. `pumpAndSettle` returns at once while Rust works, because nothing schedules a frame, so it does not hang. The integration test's `ScratchPhone extends ChannelPhone` moves only `documentsDir` to a scratch directory, so a run never meets the last run's files in the Simulator's container. P21's `TestPhone` can extend it or replace it.
6. **The theme (D13)**: Material 3, seed `0xFF2E7D32`, `Dark` → `ThemeMode.dark`, `System` and `Light` likewise, and dark while loading, since that is the default. Every text style gains `FontFeature.tabularFigures()` merged over the typography's own. **A tap target is at least 56 (`touchTarget`)**, set on the filled, outlined, text and icon buttons' `minimumSize` and `ListTileThemeData.minTileHeight`. **`SegmentedButton` ignores `minimumSize`**: Flutter 3.47 drops it from the style it hands each segment and sizes the row from a 40 px floor plus the density, so its pin first measured 48. It takes `VisualDensity(vertical: 4)`, which adds 16, and `test/app_test.dart` measures all six kinds on both sides. **For anyone finding a `TextButton` in a test**: a segmented button is built of text buttons, so `find.byType(TextButton)` finds those too.
7. **Setup, as built.** A name field, a `SegmentedButton` for the hand (right by default), and `Continue`. **The name is not checked in Dart**: an empty name goes to `create_golfer`, and its refusal shows under the field. A name that folds to a known golfer is that golfer, and the bag screen says `Already known as left-handed, and that stands.` when the stored hand is not the one ticked (P5 finding 8). The bag screen lists `all_clubs()` in order, keyed `club-<id>`. A tick (the row or its checkbox) calls `set_bag_slot` with no loft or `remove_bag_slot`, and an in-bag row's trailing button reads `loft_display` or `Add loft`. The loft is typed in `LoftDialog`. Empty sends no loft, which clears one. `NaN` and `Infinity` parse in Dart and go to Rust, whose guard refuses them under the field (the integration test reads `a loft has to be a real number of degrees, and inf is not` through the real bridge), and `340` is sent as typed. `Done` is enabled once the bag holds a club.
8. **Two sentences are Dart's, each open to challenge under rule 8.** `Type the loft in degrees, as a number.` is shown for text `double.tryParse` cannot read, and nothing is sent, since there is no number to pass. It is a parse, not a range. And item 7's handedness note. **For P30**: an iPhone set to a comma locale may offer `34,5` on the decimal pad, which `double.tryParse` refuses with that sentence. Localisation is "Not in M38", so this is recorded and not fixed.
9. **Each guard was shown to fire, and each file restored and compared by `cmp`.** `rfc3339` returning `toIso8601String()` failed the shape test, the UTC test (`…Z` for `+00:00`) and setup's `now` check. `Appearance.dark` mapped to light failed both theme tests. The bag row writing `'${slot.loftDeg}°'` itself failed on `34.0°` for `34°`. The integration test with `createGolfer`'s `_refusing` removed died on the bare `String` (item 3).
10. **For P20**: `test/fakes.dart`'s `FakePhone implements Phone`, so a method added to `Phone` stops `flutter test` compiling until `FakePhone` has it. `ScratchPhone` extends `ChannelPhone` and inherits anything new. **For P21**: setup sets no club, so `state.club` is `null` when the tabs first open, and the change sheet and `record_photo`'s refusal decide what that means. `Shell` is a placeholder per tab (`TabPlaceholder`) in an `IndexedStack`, with the tab in `_ShellState` (`ShellTab`, Record first). **For P24**: "refreshes on tab entry" hooks `onDestinationSelected`, which is the only place a tab changes. **For P27**: `BagScreen(model:)` with no `onDone` is the bag row's screen, a `StatelessWidget` listening to the model, so push it as a route. `handednessWord` is in `setup_screen.dart`.

### P20 (2026-11-05)

1. **`PhotoSource.swift` and `PhotoTime.swift` are in the Runner, and every Done clause that can hold was run.** `app/tool/reader_check.sh` now compiles `PhotoTime.swift` beside the reader and pins it on seven JPEGs it writes, a missing file and the camera's clock, all exact strings with the device fixed in `America/New_York` (item 4), and still finds 0 of 251 boxes differing on the six captures (provisional: pasted copies). `app/tool/sim_test.sh integration_test/phone_test.dart` passes 4, `flutter analyze` and `dart format` are clean over 27 files, `flutter test` passes 37 (10 new in `test/phone_test.dart`, beyond the Touch line, holding `ChannelPhone`'s parsing against a mocked channel and `Photo.discard`), and docs truth passes 91. No Rust, nothing in `crates/app/src/api/` and no generated file moved, so no `cargo` line and no `codegen_check.sh` ran. Both new files typecheck with no warning for iOS 16.0, device and Simulator, in Swift 5.
2. **The plan was wrong about the Simulator: the iOS 26.2 Simulator has a camera.** `UIImagePickerController.isSourceTypeAvailable(.camera)` answers `true`, and `takePhoto` raises the system's camera prompt, with the Info.plist sentence intact, and waits for a finger. So the Done line's "`cameraAvailable` is false" cannot hold, and `phone_test.dart` measures it and does not assert it. `no_camera`, the refusal `takePhoto` gives before asking, is reached by no host this repo has. A `simctl privacy grant camera dev.golfcoach.lab` before the run did not survive `flutter test`'s install, so the prompt cannot be answered from outside either. I reset that grant afterwards. No decision or rule stood in the way, so I recorded it and did not stop (rule 11). Dart's `Platform.environment` holds no `SIMULATOR_*` key in a Simulator app, so Dart cannot tell the Simulator apart that way. **For P21 and P28: on the Simulator the Record tab will show the shutter, and a tap on it hangs the test on that prompt until the timeout.** So an integration test's `TestPhone` overrides `cameraAvailable` to `false`, or never taps the shutter. A widget test's `FakePhone(camera: false)` is the hidden-shutter case.
3. **The channel, as built** (P18 finding 7's pattern). `cameraAvailable` answers a bool. `takePhoto` answers `{path, capturedAt}`, or `null` for a cancel, which is not an error. `pickPhotos` answers `{photos: [{path, capturedAt}], skipped: [String]}`, in the order picked, and empty lists for a cancel. `share {path}` answers whether the golfer finished an action. Refusals are `FlutterError`s with Swift's sentence: `no_camera`, `camera_denied` (asked once through `AVCaptureDevice.requestAccess`, then Settings), `busy` (one presentation at a time), `no_window`, `no_image`, `unwritable`, `no_file`, `share_failed`, `bad_arguments`. Dart adds `source`, since it chose the call: `Photo {path, capturedAt, source}` with `source` the generated `ReadSource`, which `phone.dart` re-exports, so `log_read` takes it with no mapping. **One addition beyond the plan, open to challenge: `PhotoPick.skipped`.** A photo iOS will not hand over (an iCloud original that will not download, say) becomes a sentence naming it rather than an error, so one bad photo does not cost the batch and none leaves it without the golfer being told. **For P21**: show `skipped` with the batch.
4. **`PhotoTime`, as built.** It reads the primary image's EXIF, since that is the image `ScreenReader` reads from a HEIC. **Whole seconds**: `SubSecTimeOriginal` is not read, and the format has no fraction field, so a fraction is cut off and never rounded up (the `.789` file date pins that). This answers P13 finding 12 the other way: a fraction would be lost at the lab, and a `--force` re-read there would then stamp another instant. UTC is `+00:00`, never `Z`. An EXIF date is strict (`2026:02:30` is refused, not rolled into March) and a blank one is no date. An offset is `±HH:MM` in ASCII digits up to 18 hours, or it is none, since EXIF's blanks `   :  ` are written by real cameras. **A date with no offset takes the device's offset at that date**, not today's: a July photo reads `-04:00` and a November one `-05:00`. **ImageIO will not write a blank `DateTimeOriginal`**, so that case is held on `PhotoTime.exifDate` itself and not on a JPEG. The check also confirms ImageIO wrote each case's EXIF as given, since a dropped offset would turn the first case into the second and both would pass.
5. **The library, as built.** Each photo is copied to `tmp/library/<8 hex>-<N>/<name>`, a directory per photo, so two assets sharing a name cannot meet. **The name is the provider's `suggestedName`** plus the picker's file's suffix, because the picker's file may be named for its copy (P15 finding 10). A suggested name that already ends in an image suffix keeps it, and `Screenshot 10.03.22` does not lose `.22`. The suffix is the picker's spelling, so `IMG_2889.heic` may stand where Photos shows `IMG_2889.HEIC`, and P15's fallback compares stems only. The type asked for is the first registered one conforming to `public.image` and not to `com.apple.live-photo` (D4). The time is read from the picker's file before it goes. **Unproven until P30: what that file's modification date is.** It is C9's third route, and may be the copy's own time and not the photo's.
6. **The camera, as built.** The flash is off, since a flash on a screen is a white patch where the tiles were. The time is the shutter's tap, not the moment the picker finished the image. The JPEG keeps the camera's metadata, with the Orientation in both the top level and the TIFF dictionary, so the two cannot disagree. It is named `tmp/camera/camera-YYYYMMDD-HHMMSS.jpg` from the photo's own wall clock, with `-2` on a clash. **Each presentation answers before it has finished closing**, so the read starts while the camera slides away and `ms_total` does not hold the animation. `active` is cleared only once it has gone, so a call during the slide is `busy` rather than ignored by UIKit, which would leave `active` held for good. `topController` skips a controller being dismissed for the same reason. The overlay is plain UIKit with a 76 pt shutter and a 56 pt cancel.
7. **`Photo.discard()` answers P6 finding 12.** It deletes the copy and then its directory, not recursively, so a directory holding another photo stays. It never throws, since a `Failed` read has already moved the file. **For P21**: `RecordFlow` calls it once `record_photo` has answered, whatever the status. Only call it on a photo a `Phone` answered: C22's `TestPhone` hands back copies (P6 finding 11), and discarding one also removes its scratch directory once that is empty.
8. **`FakePhone` is no longer `const`** (P19 finding 10). It is `FakePhone({camera = true, taken, picks})`, answering each `takePhoto` with the next queued photo (`null` is a cancel) and each `pickPhotos` with the next queued pick. It fails a test that queued nothing, and it has `shareCompletes` and `calls`. The four `const FakePhone()` in `test/app_test.dart` and `test/setup_test.dart` became `FakePhone()`. **For P21**: `readScreen` still throws `UnimplementedError`, so the Record tab's fakes add the reads.
9. **For P27**: `share` answers `false` when the sheet is closed with nothing chosen. P13 finding 13's order stands, `export_sessions`, `share`, `set_state`. Whether a `false` still records `last_export_at` is P27's to decide, since the zip was cut either way. The answer comes once, since `completionWithItemsHandler` can be called again when an activity's own screen closes. On an iPad the sheet is a popover anchored at the view's centre, which UIKit requires.
10. **For P29: an unsigned device build already links.** `flutter build ios --no-codesign --debug` built `build/ios/iphoneos/Runner.app` in 31 s, with 251 `PhotoSource` and `PhotoTime` symbols in `Runner.debug.dylib`, and the project file's digest unchanged. So the bridge links for `aarch64-apple-ios`, which P16 finding 7 left untried, and what remains for the device is signing. The Xcode ids are `AE137B857EDDD81DB3E67F15` and `216CE70857D938733DEF5392` (`PhotoTime.swift`'s file reference and build file) and `E4B30CE910CD58EFE611A064` and `03975EFCC5A4A874ECAEA70F` (`PhotoSource.swift`'s), checked absent first, and `plutil -lint` passes.
11. **Each guard was shown to fire, and each file restored and compared by `cmp`.** In `PhotoTime`: ignoring the EXIF offset failed 4 cases, and today's offset for a dated photo failed the summer one. A lenient formatter read `2026:02:30` as 2 March and accepted a dashed date. `Z` for UTC, no file-date route and no digit check on an offset each failed. In Dart: a camera photo tagged `library_`, a recursive `discard` and a pick dropping `skipped` each failed `test/phone_test.dart`. In Swift, on the Simulator: `no_file` recoded `missing` failed `phone_test.dart`, and it passed again restored.
12. **For P30: unproven here, and what to look at.** The camera's overlay and shutter, the permission prompt on a device, `takePicture` straight to a read, and a landscape photo reading upright from its tag (C10). The picker handing over a HEIC as a HEIC, and `original_filename` in a phone manifest reading the asset's own name (item 5, and P15 finding 10's `not in this export`). What C9's file-date route answers for a photo with no EXIF date, a screenshot say. The share sheet reaching AirDrop. `phone_test.dart` can run on the iPhone as it stands, since no test in it calls `takePhoto`, and there `cameraAvailable` reads `true` because the phone has a camera.

### P21 (2026-11-05)

*Everything read off the six captures below is `provisional: pasted copies`, and read on the Simulator, not a device.*

1. **The Record tab is built, and every clause of the Done line was run.** `flutter analyze` is clean, `dart format` changes nothing over 33 files, and `flutter test` passes 54, 17 of them new in `test/record_test.dart`. `app/tool/sim_test.sh integration_test/record_test.dart` passes, twice, in about 14 s of test after a 10 s Xcode build, and P19's `integration_test/setup_test.dart` still passes now that its last step lands on the real Record tab. No Rust, nothing in `crates/app/src/api/` and no generated file moved, so no `cargo` line and no `codegen_check.sh` ran. Nothing under `crates/`, `spec/` or `spikes/` is newer than the phase's marker. The new files are `lib/src/screens/record_screen.dart`, `lib/src/state/record_flow.dart`, `lib/src/widgets/{shot_card,change_sheet}.dart`, `test/record_test.dart` and `integration_test/record_test.dart`. **Four edits beyond the Touch line, each one a P19 finding asked for**: `core_api.dart` gained `recordPhoto` and `logRead` through `_refusing`, and re-exports the record types. `test/fakes.dart` gained `FakeCore.recordAnswers` (a `RecordOutcome`, or a `String` to refuse with), `recorded`, `logged` and `refusedLog`, and `FakePhone.reads` (a `ScreenRead`, or a `PlatformException` to throw). `app_model.dart` gained `chooseGolfer` and a public `now()` on the model's clock. `ui/shell.dart` puts `RecordScreen` in the Record tab.
2. **The six captures through the Record tab, on the Simulator.** One library batch of six, each card read off the widgets while it was up and held to `shot_detail`'s card from disk and to the stored `.shot.json`: every row present in the screen's order, each `value` equal to the stored field, and each `display` on screen. All six were `read` and none was flagged. One row was not read, `2026-10-07-1`'s ball direction, and it was highlighted, with the stored field `null`. `IMG_2893`'s sidespin read `638 D`. Both are what P18 finding 9 said to expect. Then a grey PNG drawn by `dart:ui` gave `Failed`, with Rust's sentence and no card, and one file under `.app/failed/`. The first capture again gave `Duplicate`, holding shot 1 under Rust's sentence. `reads.jsonl` held 8 lines in order, six `read`, then `failed` and `duplicate`, all `library`. Every copy under the scratch `tmp/` was gone afterwards, and the captures were untouched. The cards and the log are in `target/m38/record-cards.txt`, which is gitignored and labelled on its first lines.

   | status | boxes | ms_vision | ms_rust | ms_total |
   |---|---|---|---|---|
   | read ×6 | 40–42 | 1454–1718 | 18–26 | 1529–1811 |
   | failed | 0 | 215 | 1 | 267 |
   | duplicate | 42 | 1633 | 3 | 1666 |

   That is the second run, the one `target/m38/record-cards.txt` holds. The first was within 80 ms of it on every line. `ms_total` less `ms_vision` and `ms_rust` was 14 to 69 ms over both runs: the channel, the bridge and the card's frame. **A duplicate pays Vision's whole read** before Rust says it is held, since only Rust hashes.
3. **What was decided here that the Do line left open, each open to challenge.**
   - **`ms_total` starts when the photo reaches Dart**, either `takePhoto` answering or a batch's turn coming, and it stops in `addPostFrameCallback` on the frame that draws the card. So a library photo's total holds no picker copy, and the shutter's slide-away is not in it (P20 finding 6). The flow asks `ensureVisualUpdate` as well, so it cannot wait on a screen that draws nothing.
   - **What a refusal does.** Rust's sentence shows as the problem, **the last card is taken away**, since it could be taken for the refused photo, no log line is written, and it **ends a batch**, discarding the copies after it, because every later photo would be refused for the same reason. A refused camera photo is lost, since it exists only in `tmp/`. **The shutter is not disabled with no club**, because that rule is `record_photo`'s and a second copy in Dart would drift. Instead the header says `No club` in the error colour, which is what a golfer sees on the first tabs (P19 finding 10).
   - **P18 finding 8's question: a photo Swift cannot open is a note naming the file with Swift's sentence, the batch carries on, and it is in no count.** It has no status and no hash, so `log_read` cannot take it. A camera photo always opens, so only a library photo can reach this. **For P28 and P32**: the bay report cannot see such a photo, as it cannot see a refusal (P6 finding 8).
   - **The change sheet saves each pick as it is made, one save at a time.** `editState` works from the state the model holds, so a second tap before the first answer could undo it. **Leaving `Shot shaping` clears the shape**, since the shape control lives under that mode alone and a hidden shape would make the next photo a refusal. That much of C15 is in Dart, in `_pickMode`. The sheet cannot close on `shot_shaping` with no shape (P6 finding 13): `Done` is disabled and `PopScope(canPop: false)` stops the barrier. **The sheet is not draggable**, because in Flutter 3.47 a drag closes a modal sheet through `Navigator.pop` (`bottom_sheet.dart`, `onClosing`), which goes past `PopScope`. The barrier goes through `maybePop`, which does not.
   - **Choosing another golfer keeps the club only if their bag holds it** (`AppModel.chooseGolfer`), since the picker lists the bag alone (D7). The mode and shape stand.
   - **Choices are buttons, not chips**: `FilledButton.icon` with a check when selected and `OutlinedButton` otherwise, because the theme holds buttons to 56 and chips to nothing. They are keyed `pick-golfer-<id>`, `pick-club-<club>`, `pick-mode-<name>` and `pick-shape-<name>`.
   - **While anything runs, the shutter, the library and `Change` are all disabled**, so a batch runs under the club and intent it started with (C14). The shutter is 88 across, and hidden until `cameraAvailable` answers.
   - **The card does not show the confidence**, since Dart would have to format a number (rule 8). A `Corrected` row is labelled `corrected`. A corrected row with no value reads `cleared`, not `not read`, and is not highlighted, since a person chose it. The flag is a `Needs review` chip keyed `flagged`, and the warnings are shown in the error colour.
   - **`Photo.discard` is not awaited.** It never throws, and the next photo need not wait on a delete. An awaited delete inside `testWidgets`' fake zone would also never complete.
   - **A log line that cannot be written** becomes the problem sentence and ends a batch. The shot and its card stand.
4. **Each guard was shown to fire, and each file restored and compared by `cmp`.** In `test/record_test.dart`, each of these failed: no highlight on a row not read; the log written before the frame (it failed the first test and the batch test); a sheet that always closes; a shape kept on leaving the mode; a club kept across a golfer change; a batch going on past a refusal; a card kept on a refusal; a shutter shown with no camera; and `Change` left enabled during a batch. On the Simulator, with `discard` taken out, the integration test failed on `still waiting after 20 s` for `tmp/` to empty.
5. **`app/ios/Runner.xcodeproj/project.pbxproj` was rewritten once**, at 20:07:01 during this phase's first Simulator build, with no message from `flutter`, and left alone by the later runs. `app/` is untracked, so there is no digest to compare against. What was checked: all six hand-added ids from P18 and P20, three `IPHONEOS_DEPLOYMENT_TARGET = 16.0;`, three `PRODUCT_BUNDLE_IDENTIFIER = dev.golfcoach.lab;`, and `plutil -lint` passing. **For P28 and P36**: its digest is now `7130d00c246d959e93a23f53ba53f4c7430f913fb247f8620c559fec44409a08`, so a later change can be noticed.
6. **For P22: where the forced review goes.** `RecordFlow._recordOne` sets `last`, notifies, waits for the card's frame and then logs. A flagged answer's review belongs after the log, inside `_run`, so `busy` holds the controls while it is up. The flow has no `BuildContext`, so the screen has to give it one way to present the review, a callback or a queue it drains. A batch's flagged answers queue and are reviewed before `_run` returns (C14). `ShotCardView(card:, message:)` has no `onTap` yet, which is how an unflagged card should open the review. `test/record_test.dart`'s flagged test pins "opens nothing yet", so P22 replaces that clause.
7. **For P23**: the call goes in `_recordOne`'s `challengeCall: null`, which is the only place it is passed. `_recordOne` knows the status, so "a failed read or duplicate keeps the call" can be decided there. `sheetCanClose` and `_pickMode` are in `change_sheet.dart`. The header shows the stored state's `intentWord`, so under a challenge it shows the mode to restore, and the call needs its own large line. `modeWord`, `shapeWord` and `intentWord` are there for P24 and P25 too.
8. **For P24 and P25**: `ShotCardView` is keyed `card-<session>-<swing>` and each `CardRowView` is keyed `row-<field>`. `CardRowView.notRead` and `CardRowView.cleared` are the two words a row with no display shows.
9. **For P28's integration test**: `integration_test/record_test.dart`'s `TestPhone extends ScratchPhone` queues `picks` of repo paths and copies each into `tmp/library/<n>/`, with `cameraAvailable` false (P20 finding 2). `pumpWhile(tester, condition)` and `libraryReady(tester)` wait for the flow to finish a tap, including its log line. `blankPhoto` makes a `Failed` read through real Vision. The test imports `ScratchPhone` and `pumpUntil` from `integration_test/setup_test.dart`, and `test/record_test.dart` imports `rfc3339Shape` from `test/setup_test.dart`, rather than adding a helpers file the Touch lines do not name.
10. **For P30: unproven here.** The shutter's path, from `takePhoto` to a card on a device, and `ms_total` for a camera photo, which is T5's measure. The Simulator's library numbers in item 2 are not T5's.

### P22 (2026-11-05)

*The one edit below is read off a capture, so it is `provisional: pasted copies`, and read by the Simulator's Vision, not a device's.*

1. **The review screen is built, and every clause of the Done line was run.** `flutter analyze` is clean, `dart format` changes nothing over 36 files, and `flutter test` passes 68, 14 of them new in `test/review_test.dart`. `app/tool/sim_test.sh integration_test/review_test.dart` passes, twice, in about 8 s of test after a 10 s Xcode build, and P21's `integration_test/record_test.dart` still passes over the changed flow. No Rust, nothing in `crates/app/src/api/` and no generated file moved, so no `cargo` line and no `codegen_check.sh` ran, and nothing under `crates/`, `spec/`, `spikes/`, `src/`, `tests/` or `scripts/` is newer than the marker. `project.pbxproj`'s digest is still P21's `7130d00c…`. The new files are `lib/src/screens/review_screen.dart`, `test/review_test.dart` and `integration_test/review_test.dart`. **Five edits beyond the Touch line, each one a finding asked for**: `core_api.dart` gained `correctShot`, `shotDetail`, `setShotIntent` and `deleteShot` through `_refusing`, and re-exports `FieldEdit` and history's five types (P19 finding 3). `test/fakes.dart` gained `FakeCore.details` (a `ShotDetail`, or a `String` to refuse with), `correctAnswers`, `corrections`, `refusedIntent` and `refusedDelete`, and the helpers `detailOf(card, {mode, shape, photoPath})` and `withIntent`. `record_screen.dart` hands the flow its presenter and gives the card a tap. `shot_card.dart`'s `ShotCardView` gained `onTap`, through an `InkWell` (P21 finding 6). `test/record_test.dart`'s flagged test now ends on the pushed review, replacing "opens nothing yet" as P21 finding 6 said.
2. **A real edit, down to the file.** `IMG_2889` through the Record tab, its unflagged card tapped, the stored photo drawn by the Simulator's decoder, the carry typed over and the intent changed: the review showed `131`, `123.4` was typed, and the shot then held `carry_distance` `123.4`, `corrections` `{"carry_distance":"131.0"}` (the as-read value, compared by value), `needs_review` false, and every other reading as read. `shot_detail` answered `performance` with no shape and `corrected` true, and the card back on the tab showed `123.4` marked `corrected`. `target/m38/review-edit.txt` holds it, gitignored and labelled.
3. **What was decided here that the Do line left open, each open to challenge.**
   - **P7 finding 12's question: an emptied field is a clear.** The clear control empties the field and its letter, so on screen the two cannot be told apart, and they mean one thing. A field that is empty after Dart's `trim` sends `FieldEdit(field)` if the row had a number, and nothing if it had none. Anything else goes as typed, spaces included (` 90 ` is sent as ` 90 `, and Rust strips it). A row is sent only if its text or its letter differs from `printed` and `letter`, so a field typed over and put back is not sent.
   - **Everything is a draft until Save, the intent included.** Save calls `set_shot_intent` first, only if the pick differs from what `shot_detail` said is stored, and then `correct_shot` always, with whatever changed, since an empty list is what releases a flag and an unchanged shot moves no byte (P7 finding 5). `correct_shot` goes last because it is what releases the shot. A refused intent writes nothing. An intent stored before a refused value stays stored, and is not sent again on the next Save. A refusal shows in the error colour above Save and Delete, and the draft stands.
   - **The intent is one picker of six valid pairs** (`shotIntents`): each mode, with shot shaping as one choice per shape, labelled by `intentWord`. So no pick is one Rust would refuse as a pair, and the review has no half-chosen state to hold the golfer in. That puts C15's pair rule in Dart a second time, after P21's `_pickMode`, as a list rather than a check.
   - **What forces a review is the card's `flagged`, the stored `needs_review`, and not the status `Flagged`.** So a duplicate whose held shot is still flagged is forced too, which is what `ShotCard::flagged`'s doc says. A forced review can be cut off by the app being closed, and nothing else would ask again.
   - **The queue** (C14, P21 finding 6): a flagged card is queued as `record_photo` answers, and `_run` drains the queue after the tap's work, so after that photo's log line, after a batch's last photo, and also after a batch a refusal ended. `busy` stays set throughout. The title reads `Review i of n` when a tap queued more than one. A saved card replaces the card held when it is the same shot. A deleted one takes it away, with the note `Shot N was deleted.`.
   - **A forced review** is a `fullscreenDialog` route with no close button and `PopScope(canPop: false)`. Save and Delete leave through `Navigator.pop`, which `PopScope` does not stop. While a Save or a Delete runs, no review can be left. An unforced one has the close button and the system's back, and leaving it writes nothing.
   - **Delete asks first**, with `Keep` and `Delete`. **A review whose entry is not on this phone lets go**, even forced, with the refusal shown and one `Close` button: `shot_detail`'s only `Err` is that, so there is nothing left to review or release. A refused delete loads the detail again to learn whether the entry is gone, since `delete_shot` can refuse after removing it.
   - **The photo is decoded through `ResizeImage(policy: fit)`**, bounded to twice the screen's width in pixels on both sides (2340 on an iPhone 17), rather than through `cacheWidth` alone, which bounds the width only and would let a portrait photo decode at nearly full height. It sits in `InteractiveViewer` (to 8×), in a 260-point box. **Nothing on the review spins**: a widget test's image never finishes loading, and an endless animation would stop `pumpAndSettle` from settling. The drawn frame is keyed `review-photo-shown`, which is how the integration test knows the Simulator decoded it.
   - **The keyboard offers a minus** (`numberWithOptions(decimal: true, signed: true)`), since some tiles print one. On a tile signed by a lit letter, Rust refuses a minus and says to choose the letter.
   - **Dart's own sentences**, under rule 8: `Check each value against the photo, then save.`, the delete dialog, `Shot N was deleted.`, and the photo box's five notes. None is about a value.
   - `ReviewEnd` (`ReviewSaved(card)` or `ReviewDeleted`) is a sealed class in `review_screen.dart`, and `record_flow.dart` imports it, so a state file imports a screen file for a type.
4. **Each guard was shown to fire, and each file restored and compared by `cmp`.** In `test/review_test.dart`: a forced review that can pop (3 tests failed), every row sent whether changed or not (4), an emptied field sent as typed (4), the intent sent after the values (7), the queue drained after each photo instead of after the tap (2: the batch, and the review before the log line), and a saved card not replacing the one held (2). On the Simulator, Save sending no edits failed the integration test on the card's `123.4`.
5. **For P23**: the queue drains after `_recordOne` has returned, so a challenge that draws its next call in `_recordOne` will already have drawn it when a flagged shot's review is up. The review's intent picker then shows the shot's own stored `{shot_shaping, call}`, which is right. P23 decides whether a review that deletes a challenge shot gives its call back.
6. **For P24 and P25**: `showReview(context, model, card, forced:, place:)` is the review from anywhere, answering `ReviewEnd?` (`null` is a back-out), and `shotIntents` is the intent picker's list of pairs. A flagged shot found in History, one whose review the app was closed during, is P25's to decide: open it forced, or not. `GolfCore` now has `shotDetail`, `setShotIntent` and `deleteShot`. P24 adds `listSessions`, P25 `setShotClub` and `setShotMishit`. `FakeCore.details` and `detailOf` are there for both. **For P25**: the review's Delete and intent repeat two of the detail screen's actions. Reuse them rather than add a second spelling.
7. **For P28's integration test**: `integration_test/review_test.dart` imports `TestPhone`, `cardOf`, `installId`, `libraryReady`, `pumpWhile`, `session` and `storedShots` from `integration_test/record_test.dart`, as that test imports from setup's. It has no flagged shot, since none of the six captures reads flagged (P21 finding 2), so the forced path is held by the widget tests alone, over fakes, and has not run through the real bridge.
8. **For P30: unproven here.** Flutter drawing a HEIC from the library (the captures are JPEGs). The soft keyboard `signed: true` raises on iOS, and whether it covers the Save bar. A comma-locale `131,5`, which goes to Rust as typed and is refused as not a number (P19 finding 8's limit, met here by Rust's sentence instead of Dart's). Pinch-zoom on a 12-megapixel photo at the 2340 bound.

### P23 (2026-11-05)

*The one test through real Rust reads two captures through M33's committed boxes, so it is `provisional: pasted copies`, and it is this Mac's Rust under `flutter test`, with no Vision.*

1. **Challenge mode is built, and every clause of the Done line was run.** `flutter analyze` is clean, `dart format` changes nothing over 38 files, and `flutter test` passes 82, 14 of them new in `test/challenge_test.dart`. No Rust, nothing in `crates/app/src/api/` and no generated file moved, so no `cargo` line and no `codegen_check.sh` ran, and nothing under `crates/`, `spec/`, `spikes/`, `src/`, `tests/` or `scripts/` is newer than the marker. `project.pbxproj`'s digest is still P21's `7130d00c…`. The new files are `lib/src/state/challenge.dart` and `test/challenge_test.dart`. **Two edits beyond the Touch line, each one a finding asked for**: `record_flow.dart` owns the `Challenge`, passes its call in `_recordOne` and draws the next there (P21 finding 7), and `app_model.dart` gained `Random get random`, since C15 puts the `Random` in the model. The install id's mint uses the same one, on a first run only.
2. **The test through real Rust, and how `flutter test` reaches the bridge.** The Record tab over `RustCore`, with a `FakePhone` whose documents are a scratch directory, camera copies of `IMG_2889` and `IMG_2891`, and M33's committed `raw` boxes for each. One shot under the stored `performance` left `graded_profile` with no shape topic. Then the challenge went on over all three, with the draws named Draw and then Straight. The second shot is stored `{shot_shaping, draw}` and the first is still `{performance}`, the 7i's one shape topic is `draw` (withheld at 1 of the 10 shots a share needs), the call on screen is then Straight, and `state.json`'s mode is still `performance`. **`RustLib.init()` alone fails under `flutter test`**, because it looks for the iOS framework. But the native-assets hook writes the host's build to `app/build/native_assets/macos/libgolf_bridge.dylib` before any test runs, and `RustLib.init(externalLibrary: ExternalLibrary.open(...))` loads it. Bridged answers arrive on the real event loop, which a widget test's fake clock does not run, so the test pumps between `tester.runAsync` slices (`pumpReal`). It takes about a second.
3. **What was decided here that the Do line left open, each open to challenge.**
   - **"Hidden under Drill and turning Drill off" is built as two guards.** The switch is hidden while the mode is Drill, and while a challenge is on, the Drill button is disabled. So the two never stand together, and only the switch ends a challenge. The other reading, where picking Drill ends the challenge, was not built. A state holding both, which only a file written elsewhere could, still shows the switch, so the challenge can be turned off.
   - **The call lives in `Challenge`, which `RecordFlow` owns, and it follows the model** rather than being told what the sheet did, so a relaunch and the sheet agree with it alike. It changes on exactly three things: a photo recorded under it (`Read` or `Flagged`), the switch, and its own tick coming off. Ticking another shape keeps it. A relaunch draws a fresh one, since the call is not stored (P6 finding 3). The call is taken when the photo reaches the flow, before Vision reads it.
   - **The draw is `nextInt(n)` over the ticks in `TargetShape`'s order** (`challengeTicks`), so a test names a draw by its index, and the stored order cannot move it. Uniformity is pinned on a seeded `Random(23)`: 3000 draws over each of three tick sets, each count within 4 SD of a fair share, and no unticked shape ever came up.
   - **Turning it on with no shapes stored ticks all three. Turning it off keeps the ticks, so it comes back with the last ones used.** "All three by default" is read as "when none is stored". The last tick's button is disabled. The mode and shape above stay pickable under a challenge, since they are what turning it off goes back to, and the switch's subtitle says so: `Off goes back to <intent>, and Drill waits until then.` `sheetCanClose` also refuses a challenge with nothing ticked, and still asks for the mode and shape pair under a challenge.
   - **A library batch of more than one is refused under a challenge, with none read and none recorded** (`A challenge calls one shot at a time, so import one photo for each call. None of the N photos was recorded.`). Under the Do line's "a recorded shot takes `{shot_shaping, call}` and draws the next", each later photo of a batch would take a call drawn after it was taken, and be graded in a shape topic the golfer never aimed at. A single library photo takes the call.
   - **P22 finding 5's question: a review that deletes a challenge shot does not give its call back.** The shot was recorded under the call, and the next call was drawn with its card. The delete is the same action History's will be (P25), which cannot reach the challenge. And a fresh draw is as uniform as the old call would have been.
   - **Under a challenge the header reads `Challenge` in place of the mode**, since the next photo is stamped with the call and not the stored mode. This replaces P21 finding 7's "it shows the mode to restore". The call shows large beneath it (`Challenge: hit a` over the shape in `displayMedium`, keyed `challenge-call` and `challenge-shape`).
   - **Dart's own sentences**, under rule 8: the switch's title and two subtitles, the call's label, and the batch refusal. None is about a value.
4. **Each guard was shown to fire, and each file restored and compared byte for byte.** Thirteen mutations each failed the tests meant for them: the call not passed (5 tests, the real-Rust one among them), no next draw (4), a draw after any status (1), a batch let through (1), the switch offered under Drill (1), Drill pickable under a challenge (1), the last tick enabled (1), turning on with nothing ticked (1), a redraw on every model change (3), the header keeping the mode (2), a biased draw, `nextInt(n) ~/ 2` (4), a sheet that closes with nothing ticked (1), and a call kept after the switch is off (2). **One more passed**: a "never the last tick" check inside `_tick`, which the disabled button never lets a tap reach. It was taken out rather than left as an untested second copy, and the button's guard is what is pinned.
5. **For P24 and P25**: a challenge shot is stored as `{shot_shaping, call}`, and nothing on disk says it was a challenge's, so History and the detail screen show it as `Shot shaping: Draw` like any other. If a mark is wanted, it needs a manifest field, which is a shape change and outside M38. **For P24, P25 and P26**: real Rust runs under `flutter test` (item 2), and `test/challenge_test.dart` holds `hostBridge`, `ScratchFakePhone` and `pumpReal` for it. `GolfCore` has no `gradedProfile` yet, so this test calls the generated function directly. **For P26**: add `gradedProfile` to `GolfCore` and `FakeCore`.
6. **For P28**: the whole-flow test imports the six captures as one batch, so it must run with no challenge on. P21's and P22's integration tests were not re-run on the Simulator over the changed flow (rule 15: P23's Verify names none), so P28's run is the first to.
7. **For P30 and P31: unproven here.** Whether the call can be read at arm's length from the hitting position in the bay's light, and a challenge run on the device's camera. **For P31**: the run sheet should say that a challenge's shots are photographed one at a time, because a library batch is refused under one.

### P24 (2026-11-05)

*The one test through real Rust reads two captures through M33's committed boxes, so it is `provisional: pasted copies`, and it is this Mac's Rust under `flutter test`, with no Vision.*

1. **The History tab is built, and every clause of the Done line was run.** `flutter analyze` is clean, `dart format` changes nothing over 40 files, and `flutter test` passes 94, 12 of them new in `test/history_test.dart`. No Rust, nothing in `crates/app/src/api/` and no generated file moved, so no `cargo` line and no `codegen_check.sh` ran, and nothing under `crates/`, `spec/`, `spikes/`, `src/`, `tests/` or `scripts/` is newer than the marker. `project.pbxproj`'s digest is still P21's `7130d00c…`, since no Xcode build ran. The new files are `lib/src/screens/history_screen.dart` and `test/history_test.dart`. **Five edits beyond the Touch line, each one the Do line or a finding asking for it**: `core_api.dart` gained `listSessions` through `_refusing` and re-exports `SessionView` (P22 finding 6). `test/fakes.dart` gained `FakeCore.sessions` (by player id, a list or a `String` to refuse with, and a golfer with nothing there has none, as in Rust). `app_model.dart` gained `shotsVersion` and `shotsChanged()`. `record_flow.dart` calls `shotsChanged()` after a `Read` or `Flagged` answer and at the end of every review, back-outs included, since a review can store an intent and then be left (P22 finding 3). `ui/shell.dart` puts `HistoryScreen(model:, active:)` in the History tab.
2. **Through real Rust.** Two captures recorded through `RustCore` out of the order they were taken (`IMG_2891` at 10:05 first, then `IMG_2889` at 10:00), and shot 2 then set to Drill by `set_shot_intent`. The tab showed shot 1 above shot 2, which is `list_sessions`' own order and the reverse of arrival's. Only shot 2 was marked, `Drill · not counted`, its intent line read `Drill`, and each row's carry was the stored card's `carry_distance` display.
3. **What was decided here that the Do line left open, each open to challenge.**
   - **"After a record" is the model's `shotsChanged()`.** The flow belongs to the Record tab's state, and a library batch runs on while the golfer looks at another tab, so History, when it is the tab shown, has to hear of each shot as it lands. It reads the stores on four things: entering the tab, a shot written while it is shown, another golfer chosen while it is shown, and coming back from a row's tap. A setting changing reads nothing. Nothing is cached between entries (P9 finding 3's 35 ms at 500 entries), the last list stays up while a read is out, and of two reads out at once the later one asked is the one shown.
   - **The newest session is open and the rest are closed**, until the golfer taps a header, and a tap is remembered while the tab lives. So today's session is open on entry and yesterday's does not take the screen. **It is one flat list built as it scrolls**, not an `ExpansionTile` per session, so a session of hundreds builds only the rows on screen, and only their thumbnails decode.
   - **P9 finding 10's question: a row left out with no mark of its own is marked `Not counted`, with Rust's sentences beneath it.** That is a stale, outdated, unanalysed or duplicate entry, which the golfer can repair and nothing else on the list would show. **A drill's sentence is not repeated on every drill row**, since its mark says it, and the detail screen holds the sentence. The other marks read `Needs review`, `Corrected` and `Mishit`. **A limit**: a drill that is also a duplicate does not show the duplicate's sentence on its row, and only such an entry can hold both, since the corpus decides duplicates before drills.
   - **A row**: the thumbnail, `Shot N · 20:42 · 7i` (`No club` where none), the intent by `intentWord`, so a challenge's shot reads `Shot shaping: Draw` like any other (P23 finding 5), then carry and ball speed under their tiles' own labels, then the marks. A session's header is Rust's `date` as it stands, `2026-11-05`, with `N shots`. The title is `<name>'s shots`.
   - **A value with no display reads `not read`**, italic in the error colour, and never 0. **A limit**: that is said of a value a person cleared too, since `ShotRow` carries one `None` for both and no state per value. Only the card's `RowState` tells them apart. The `Corrected` mark is on such a row, and if the words matter, the fix is a field in Rust.
   - **The thumbnail is decoded through `ResizeImage(policy: fit)` to twice its 56-point side in pixels** (336 at 3x), then filled by `cover`. Twice, because `cover` crops the long side, and a 4:3 photo fitted into twice the box keeps its short side at 1.5 times the box. A missing photo, one still loading, and one that cannot be drawn each show an icon, and nothing spins.
   - **A row's tap is `HistoryScreen.openShot`, which nothing passes yet**, so until P25 a row does nothing when tapped. The list is read again once it completes.
   - **Dart's own sentences**, under rule 8: the title, the empty state (`No shots for Aaron yet. Photograph the screen on the Record tab after each shot, and every shot is listed here, whether or not it counts.`), the no-golfer and reading lines, and the five marks' words. None is about a value.
4. **Each guard was shown to fire, and each file restored and compared by `cmp`.** Twelve mutations each failed the tests meant for them: rows sorted by swing number, no read on entering the tab, the flow never saying a shot landed, a missing value as `0`, a drill also marked `Not counted` (2 tests), no drill mark (2), a read on every change of the model, a read only on a golfer change, every session open, no read after a row's tap, a thumbnail decoded whole, and the last golfer's list left standing under the new one's name. The read ticket's mutation passed at first, since no test held two reads out at once, so one went in (`HeldCore`), and it then failed under that mutation. **One more passed and was taken out**: a redraw in the model's listener. `_Home` in `main.dart` rebuilds the Shell on every change of the model, so the title already follows the golfer, and the redraw was an untested second copy (P23 finding 4's precedent).
5. **For P25**: pass `openShot` (`OpenShot = Future<void> Function(BuildContext, ShotRow)`) to `HistoryScreen` from `ui/shell.dart`, completing once the golfer is back. History reads the stores again then, so the detail screen does not have to tell it. A detail writer should still call `model.shotsChanged()`, which is free. `HistoryRow` and `Thumbnail` are public in `history_screen.dart`. `test/history_test.dart` has `shotRow`, `sessionOf`, `rowFor`, `inShot`, `marksOn`, `pumpApp`, `enterHistory` and `enterRecord`. Whether a flagged shot found here opens forced is still yours (P22 finding 6).
6. **For P26**: `ui/shell.dart`'s `active` flag and `didUpdateWidget` are how a tab reads again on entry, and `model.shotsChanged()` says a shot changed, if the Profile tab should follow either.
7. **For P28's integration test**: rows are keyed `shot-<session>-<swing>`, the marks `mark-flagged`, `mark-corrected`, `mark-mishit`, `mark-drill` and `mark-left-out`, and the values `carry` and `ball-speed` inside a row. The list is built as it scrolls, so scroll to a low row first: `test/history_test.dart`'s `scrollTo` uses `scrollUntilVisible` on the Scrollable under `history-list`. While the Record tab is busy, its progress bar keeps `pumpAndSettle` from returning, even with History shown.
8. **For P30: unproven here.** A HEIC thumbnail drawn by Flutter on the device (P22 finding 8), and scrolling a long session on the phone.

### P25 (2026-11-06)

*Resumed: the attempt marked 2026-11-05 20:57 had left the screen, both tests and the shared widgets in the tree, and its integration test had last run before the screen's final edit. This session kept all of it, since each piece held up when read against the Do and Done lines, then re-ran every Verify line over it and showed the guards fire. The three captures below are recorded through M33's committed C0 boxes, so they are `provisional: pasted copies`, on the Simulator and not a device.*

1. **The shot detail screen is built, and every clause of the Done line was run.** `flutter analyze` is clean, `dart format` changes nothing over 43 files, and `flutter test` passes 108, 14 of them new in `test/shot_detail_test.dart`. `app/tool/sim_test.sh integration_test/shot_actions_test.dart` passes in about 8 s of test after an 11.5 s Xcode build. No Rust, nothing in `crates/app/src/api/` and no generated file moved, so no `cargo` line ran and neither did `codegen_check.sh`. Nothing under `crates/`, `spec/`, `spikes/`, `src/`, `tests/` or `scripts/` is newer than the marker, and `project.pbxproj`'s digest is still P21's `7130d00c…`. The new files are `lib/src/screens/shot_detail_screen.dart`, `test/shot_detail_test.dart` and `integration_test/shot_actions_test.dart`. **Five edits beyond the Touch line, each one a finding asked for**:
   - `core_api.dart` gained `setShotClub` and `setShotMishit` through `_refusing` (P22 finding 6).
   - `test/fakes.dart` gained the two writers, each answering the held detail with its row changed, plus `refusedClub` and `refusedMishit` (P19 finding 3).
   - `ui/shell.dart` passes `openShot: showShotDetail` to History (P24 finding 5).
   - `review_screen.dart` makes `ShotPhoto` (with a `keyPrefix`), `ProblemNote`, `IntentPicker`, `Pick` and `confirmDelete` public, so the detail screen uses the review's own Delete dialog and intent picker and does not spell them a second time (P22 finding 6).
   - `history_screen.dart`'s `openShot` doc now names the screen.
2. **Through real Rust, on the Simulator** (`target/m38/shot-actions.txt`, gitignored and labelled). Three captures were recorded under the 7i, with lofts of 34 and 46 declared so no `no_club_loft` row arises. Each was then changed from its History row's detail screen: shot 1 retagged `pw`, shot 2's mishit confirmed, and shot 3 deleted, which went back to History by itself. Afterwards everything agreed:
   - History on screen showed 2 rows, `Shot 1 · 10:00 · pw` and shot 2 alone marked `Mishit`.
   - `list_sessions` gave rows `2, 1`, with shot 1 `pw`, no verdict and not a mishit, and shot 2 a mishit, `confirmed`.
   - On disk, the manifests read `club: pw` and `mishit: confirmed`, `sessions/S/3` was gone, and the shot store held the two survivors.
   - `graded_profile` counted `7i 3` before and `7i 1, pw 1` after. **For P26: a confirmed mishit still counts in its club's `n_shots`.** It is held out of the distance averages and not out of the count, so a club's number of shots is not the number its carry is averaged over.
3. **What was decided here that the Do line left open, each open to challenge.**
   - **Each action writes as it is tapped, with no draft and no Save**, unlike the review. Each writer answers the `ShotDetail` (P10 finding 10), and the answer replaces the screen. A tap on the club already chosen moves no byte but re-runs the analysis, so it is also the repair of a lost analysis. Only one action runs at a time: every control is disabled while one is out. A refusal shows in Rust's words in the bottom bar above Delete and Edit, so it stays on screen wherever the control was, and the next action clears it.
   - **P22 finding 6's question: a flagged shot found here is not forced.** It is a shot whose forced review was cut off by the app closing. Forcing it again here would trap someone who came to delete a stray photo on a screen that can only save. So the screen says `This read has not been reviewed. Edit its values, check each against the photo, then save.`, the shot keeps its `Needs review` mark on History, and Edit (or a tap on the card) opens the review with its close button.
   - **However the review ends, the detail is read again**, a back-out included, since a review can store an intent and then be left (P22 finding 3). A review that deletes the shot also ends the detail screen.
   - **The mishit control is three segments, `Automatic`, `Mishit` and `Not a mishit`**, starting on `row.mishit_verdict` (none is `Automatic`). Beneath it is one of four lines read off `(mishit_verdict, mishit)`, saying what is in force and whose call it is, so a shot the rule holds out reads differently from one a person did. Those four sentences are Dart's and judge nothing: they read two values Rust gave. If they should be Rust's, the fix is a field on `ShotRow`.
   - **The club picker is the bag alone (D7).** A shot tagged with a club the bag no longer holds has nothing chosen and says `This shot is tagged driver, which is not in the bag.` Rust does not refuse such a club (P10 finding 4), so this is layout and not a rule.
   - **An entry with no card offers only a verdict and a delete**, with `no_card` shown, since the other two writers refuse it (P10 finding 10). An entry that `shot_detail` refuses shows the refusal and offers nothing, and so does one found gone after a refused delete, which shows both sentences.
   - **The rows that were not measured are laid out as plainly as the measurements**, not in the error colour (P9 finding 4). Each measurement shows its `detail` beneath it. **P7 finding 6 needed nothing here**: Rust leaves a cleared tile out of `unscored`, and the card's row reads `cleared` (P9 finding 11).
   - **Dart's own sentences**, under rule 8: the reading line, the flagged line, `No photo is stored for this shot.`, the not-in-bag line, the four mishit lines and the section titles. None is about a value.
4. **Each guard was shown to fire, and each file was restored and checked by `shasum`.** Eighteen mutations each failed `test/shot_detail_test.dart`. The answer not replacing the screen failed 4 tests. A delete without asking and a forced Edit failed 2 each. Each of these failed 1:
   - the controls never disabled;
   - the mishit control always on `Automatic`;
   - a refused delete not read again;
   - a card-less entry offered a club and an intent;
   - a review's delete leaving the detail up;
   - the mishit line ignoring whose call it is;
   - a refusal kept past the next action;
   - a writer not calling `shotsChanged`;
   - the not-in-bag line unsaid;
   - the flag unsaid;
   - a measurement's `detail` hidden;
   - `Not a mishit` sending `confirmed`;
   - `left_out` unshown;
   - History not read again after the detail.

   `ui/shell.dart` passing no `openShot` failed 13. One mutation, the not-in-bag line as `if (false)`, first failed to compile, because it broke the null promotion. It was re-run as a condition that is always false, and then failed its test. On the Simulator, `Mishit` mapped to `cleared` failed the integration test at 25 s, waiting for `Your call: a mishit…`.
5. **For P35, and the user's call: P9 finding 6's raw keys are now on screen.** A measurement reads `start_line_offline_yds` and a flight row `flight_carry_yds`, beside `pyfmt::g`'s six digits (`-2.2619`). Rule 8 keeps the fix out of Dart, so it is a label table in `crate::history` if the user wants one.
6. **For P26**: every detail writer calls `model.shotsChanged()`, so a Profile tab that follows it (P24 finding 6) hears of a retag, a verdict or a delete. `ShotPhoto`, `ProblemNote`, `Pick` and `IntentPicker` are public in `review_screen.dart`. The detail screen's `_Section` and `_Line` are private.
7. **For P28's integration test**: the detail is reached by tapping `shot-<session>-<swing>` on History, and its card is keyed `card-<session>-<swing>`. These are the keys:
   - the photo `detail-photo`, and the heading `detail-heading`;
   - the clubs `detail-club-<club>`, and the intents `intent-<mode>`, or `intent-shotShaping-<shape>`;
   - the mishit control `mishit`, with segments `mishit-automatic`, `mishit-confirmed` and `mishit-cleared`, and its line `mishit-in-force`;
   - the buttons `detail-delete` and `detail-edit`, then the dialog's `delete-confirm` and `delete-keep`;
   - the notices `detail-problem`, `detail-gone`, `detail-no-card`, `detail-flagged` and `detail-left-out`;
   - the rows `measure-<name>` and `unscored-<name>`, and `note`.

   `integration_test/shot_actions_test.dart` has `tapShown`, `openShot` and `backToHistory`. **This test records through the facade with committed boxes, not through the Record tab's Vision**, so P28's run is the first to reach the detail screen from a Vision read.
8. **For P30: unproven here.** The detail screen on a device: a HEIC drawn in its photo box (P22 finding 8), the three mishit segments' labels on a phone narrower than the iPhone 17, and the bottom bar beside the home indicator.

### P26 (2026-11-06)

*The six captures below are recorded through M33's committed C0 boxes, so they are `provisional: pasted copies`, on the Simulator and this Mac and not a device.*

1. **The Profile tab is built, the user's `center` with it, and every clause of the Done line was run.** `flutter analyze` is clean, `dart format` changes nothing over 49 files, and `flutter test` passes 123, 15 of them new in `test/profile_test.dart`. `app/tool/sim_test.sh integration_test/profile_test.dart` passes in about 3 s of test after a 14 s Xcode build. `cargo test -p app -p golf-core` passes 533, with `tests/profile.rs` at 27 (3 new), `cargo clippy -p app --all-targets --no-deps -- -D warnings` and `cargo clippy -p golf-core --no-deps -- -D warnings` are clean, `cargo fmt --check` passes, the iOS target, `golf-lab` and `golf-mcp` build, `codegen_check.sh` settles at 24 functions bound, and docs truth passes 91. No vector, version key or `spec/` file moved, and `project.pbxproj`'s digest is still P21's `7130d00c…`. The new files are `lib/src/screens/profile_screen.dart`, `lib/src/widgets/{topic_tile,blend_card,finding_list}.dart`, `test/profile_test.dart` and `integration_test/profile_test.dart`. **Edits beyond the Touch line, each asked for by the decision or a finding**:
   - `crates/core/src/profile.rs` gained `run_profile_and_bag_profile`, and `run_profile` is now that call with the `BagProfile` dropped (item 2);
   - `crates/core/src/reports/club_profile.rs`'s `center` row is now printed through a `pub fn center_text`, with no change to its text (C23's footing for a helper made `pub`);
   - `core_api.dart` gained `gradedProfile` through `_refusing` and re-exports the nine profile types, and `test/fakes.dart` gained `FakeCore.profiles` (P23 finding 5, P19 finding 3). A golfer with nothing in it fails the test, since even an empty profile carries Rust's sentences;
   - `ui/shell.dart` puts `ProfileScreen(model:, active:)` in the Profile tab;
   - the codegen rewrote `lib/src/rust/third_party/app/api/profile.dart` and `app/rust/src/frb_generated.rs`.
2. **The decision's `center`, and why it is not the graded profile's own `center`.** `NumberRow.center: Option<String>` is `golf-core club-profile`'s `center` row after its label, `156.8   95% CI 82.7 .. 231.0   (median 146.5)`, with no unit, as the report prints it. **The graded profile could not supply it for carry**: `StrikeProfile.dispersion`'s `center` is set only by `decide_bias`, and only where there is a target to be biased from, and carry has none (`contracts::dispersion::METRIC_TARGETS`). So the mean comes from the club's baseline in the `BagProfile` that `run_profile` already builds. `run_profile_and_bag_profile` hands that build out, so there is no second build and `crates/app` gains no `analysis` edge. The floor is `Center` 5 against `Spread` 10 (`contracts::baseline::DEFAULT_MINIMUM_N`), so on the six captures every mean is through and every spread is withheld. **What holds it**:
   - `every_center_is_club_profiles_own_line_for_each_case`: over every hand and corpus case, each club's mean is the line `club_profile(.., Some(club))` prints for that metric, word for word, and is absent exactly where that report prints none. Both states are reached.
   - `a_mean_shows_before_its_spread_does` and `a_mean_below_its_floor_is_absent`.
   - The six-captures test, which holds each mean to the stdout of the verb itself (`reports::answer(Verb::ClubProfile, ..)` with `--club 7i`) over the same stores.
   - The profile report's gate, which names `center: _` and says why. Its 28 fields are unchanged.
   This came to about 30 lines of code, and not P11's 20.
3. **Through real Rust, on the Simulator** (`target/m38/profile.txt`, gitignored and labelled). Six captures were recorded under the 7i, with `pw` in the bag and never hit. The tab showed the 7i and then Player, and `In the bag with no tracked shot, so not in the profile: pw.` beneath. It showed `6 tracked shot photos`, no blend card, `consistency` withheld at `6 of the 10 shots a share needs` with no number, and `nothing is established either way yet`. These were the means and spreads:

   | Number | Mean (`center`) | Spread |
   |---|---|---|
   | `ball_speed_mph` | 112.1, 95% CI 80.4 .. 143.8, median 107.7 | withheld, 6 of 10 |
   | `carry_distance_yds` | 156.8, 95% CI 82.7 .. 231.0, median 146.5 | withheld, 6 of 10 |
   | `face_to_path_deg` | 7.4, 95% CI 1.6 .. 13.1, median 7.0 | withheld, 6 of 10 |
   | `start_line_deg` | 4.2, 95% CI 0.4 .. 8.1, median 4.0 | withheld, 6 of 10 |

   Player showed its basis and its grades, with no numbers and no blend. The carry's interval is 148 yards wide, which is the honest answer for these six copies at n=6, and why the floor is 5 and not fewer.
4. **What was decided here that the Do line left open, each open to challenge.**
   - **The selector is `review_screen.dart`'s `Pick` buttons in a `Wrap`**, not chips, for the 56 a tap needs, as the change sheet and the detail screen draw theirs. Three entries already wrap to a second row on the iPhone 17. The first club shows until a pick. A pick is kept across reads while its club is still in the profile, and otherwise the first club stands in. Player shows at once when no club is listed. Choosing another golfer resets the pick.
   - **A club's page reads**: heading and `shots`, the blend card, `Numbers`, `Grades`, then `Established` (the list). "The grade beside the list" is built as the list directly after the grades and not inside a topic, since a finding names a metric and not a topic, and ADR-034 §5.7 says neither feeds the other. The player's page is the same with `basis` and no numbers.
   - **A topic tile** shows the name, then the on-line share (its percent set large beside Rust's text, or the text alone when withheld), then on a shape topic the hit-the-shape share beneath it. Then `landings`, which is boxed in the tertiary colour with an icon and keyed `projected` when `on_line_projected`, then each `no_landing` and `unscored` sentence. `criterion` is not drawn on its own, since both share sentences hold it, and no Dart label tells the two shares apart, since each sentence says what it is a share of.
   - **The blend card is `mean` and Rust's `text`**, which names what it averaged and what it left out, so `averaged` and `excluded` are not drawn a second time. A missing blend builds no card.
   - **A number** shows its contract name, its mean in `titleLarge` when Rust gives one, and its spread or refusal, in the muted colour when withheld.
   - **Reads follow History's rule** (P24 finding 3): on entering the tab, and while it is shown on `shotsChanged()` or another golfer chosen, with a ticket so the later of two reads wins. **This is a second copy of History's 40 or so lines of follow logic.** A shared mixin would have meant editing History, which P26's Touch line does not reach. It is a candidate for `/refactor-review`, not a decision taken.
   - **Dart's own sentences**, under rule 8: the title, the no-golfer and reading lines, `Player`, the section titles (`Numbers`, `Grades`, `Established`, `Blend`, `Strengths`, `Weaknesses`), and the not-in-the-profile line. That line says "tracked" because a club hit only in drills is absent too. None is about a value.
   - `test/challenge_test.dart` still calls the generated `gradedProfile` directly. It tests the facade, and P23 finding 5's ask was `GolfCore`'s, which is done.
5. **Each guard was shown to fire, and each file was restored and checked by `shasum`.** Seventeen Dart mutations each failed `test/profile_test.dart`. A withheld share drawn as `0%` failed 3 tests, the Player pick removed 4, and the bag's missing clubs unsaid 3. No read on entering the tab, and the shell without the screen, each failed 15. A `0%` blend card for no blend failed 2. Each of these failed 1:
   - no hit share;
   - projected never marked;
   - landings unshown;
   - a strengths heading always drawn;
   - `nothing_listed` unshown;
   - a read on every change of the model;
   - the pick reset on every read;
   - a mean unshown;
   - no Player when no club is listed;
   - a refusal unshown;
   - a golfer change not read.

   "Landings unshown" first failed to compile as `if (false)`, which broke the null promotion (P25 finding 4's case). It was re-run as a condition that is always false at run time. There were five Rust mutations:
   - In `crate::profile`, a mean with text appended and no mean at all each failed 3 tests in `tests/profile.rs`, and another metric's mean and a mean below its floor each failed 2.
   - `center_text` reworded on both sides failed `crates/core/tests/reports.rs`, which holds the committed career text. `tests/profile.rs` alone cannot catch that, which is P11 finding 9's point, made again for the mean. `crates/core/tests/career.rs` passed under it.
6. **For any later phase that mutates Rust: restore with a fresh mtime.** The first restore here used Python's `shutil.copy2`, which keeps the backup's mtime. So cargo saw nothing newer than its last build, and kept the mutated one. The next `cargo test -p app --test profile` failed 3 on files `shasum` called restored. A `touch` and a rebuild made it green. Use `cp` without `-p`, or `touch` after.
7. **For P28's integration test**: the tab is `Icons.insights_outlined`. These are the keys:
   - the list `profile-list`, and the lines `profile-title`, `profile-preamble`, `profile-no-clubs`, `profile-unhit` and `profile-problem`;
   - the heading `profile-heading`, with `profile-shots` and, for the player, `profile-basis`;
   - the picks `pick-club-<club>` and `pick-player`;
   - `blend-card`, with `blend-mean` and `blend-text`;
   - each `number-<name>`, with `center` and `spread`;
   - each `topic-<name>`, with `grade-percent`, `grade-text`, `hit-percent`, `hit-text`, `landings`, `projected`, `no-landing` and `unscored`, and `no-topics` in place of the topics;
   - `finding-list`, with `nothing-listed`, `strengths` and `weaknesses`, and each entry `strength-<metric>` or `weakness-<metric>`.

   **The list is a `ListView` of children, which builds lazily**, so scroll to a lower section before finding it. `integration_test/profile_test.dart`'s `scrollTo` does, and `up: true` gets back to the picks. **This test records through the facade with committed boxes**, so P28's run is the first to reach the Profile tab from Vision reads.
8. **For P35**: the Profile tab is now held to two reports, `golf-core profile` for every sentence and `golf-core club-profile` for each mean. ADR-034's addendum should say so. P11 finding 12's limit is closed by the user's decision and not left open. P11 finding 11's contract names on screen now also cover the four `NumberRow.name`s and each finding's `metric`, which is the same question as P25 finding 5.
9. **For P30: unproven here.** The tab on a device: the picks' `Wrap` on a phone narrower than the iPhone 17, and a mean's text in `titleLarge`, which already wraps to two lines on the iPhone 17 (`156.8   95% CI 82.7 .. 231.0   (median 146.5)`). Rule 8 makes any shorter spelling of it Rust's.

### P27 (2026-11-06)

*The six captures below are recorded through M33's committed C0 boxes, so they are `provisional: pasted copies`, on the Simulator and this Mac and not a device. The share sheet was answered by the test, not by a finger.*

1. **The Settings tab is built, and every clause of the Done line was run.** `flutter analyze` is clean, `dart format` changes nothing over 52 files, and `flutter test` passes 145, 22 of them new in `test/settings_test.dart`. `app/tool/sim_test.sh integration_test/export_test.dart` passes, twice, in about 3 s of test after a 9 s Xcode build. No Rust, nothing in `crates/app/src/api/` and no generated file moved, so no `cargo` line ran and neither did `codegen_check.sh`. Nothing under `crates/`, `spec/`, `spikes/`, `src/`, `tests/` or `scripts/` is newer than the marker, `project.pbxproj`'s digest is still P21's `7130d00c…`, and docs truth passes 91. The new files are `lib/src/screens/settings_screen.dart`, `test/settings_test.dart` and `integration_test/export_test.dart`. **Edits beyond the Touch line, each one a finding or the Do line asking for it**:
   - `core_api.dart` gained `exportSessions` through `_refusing`, plus `exportStatus` and `about`, which never refuse, and re-exports `AboutView`, `ExportStatus` and `ExportSummary` (P19 finding 3).
   - `test/fakes.dart` gained `FakeCore.exportAnswers`, `status` and `aboutView`, and `FakePhone.shareFails`.
   - `app_model.dart` gained `addGolfer` (item 3), and `chooseGolfer` now clears `keptHandedness`, since the bag screen's note is about a name just typed and not a golfer picked.
   - `setup_screen.dart` takes an optional `create`, so Add a golfer is setup's own name screen (item 3).
   - `ui/shell.dart` puts `SettingsScreen(model:, active:)` in the Settings tab. **`TabPlaceholder` is gone**: no tab used it any more, and no test named it.
2. **Through real Rust, on the Simulator** (`target/m38/export.zip` and `export.txt`, gitignored and labelled). Six captures were recorded under the 7i. Settings showed `Nothing has been exported from this phone yet.` and `6 shots on this phone.`, then Export cut `<scratch>/tmp/export/golf-coach-a1b2-2026-11-06-080517.zip`, 10.1 MB and 32 entries, and handed that path to `share`. The test checked these:
   - The entries are sorted by name. The six photos are stored, each one a capture's own bytes, and every JSON is deflated.
   - There are six `shots/<digest>.shot.json`, with the same digests as the six `.app/boxes/`, and no `.app/state.json`.
   - `state.json` then held `last_export_shots: 6` and the summary's `last_export_at`, and the tab read `Last export 2026-11-06 08:05, holding 6 shots.` and `6 shots on this phone, 0 new since then.`
   - A second export left one zip in `tmp/export/`, the second one.

   **On the host, outside the test**: `golf-lab import-export` into empty scratch dirs copied 26 files and ignored 6 (the `.app/` ones), and `golf-bay-report compare` printed `Corpus of aaron: equal, 6 swing(s) and 0 excluded on each side`, exit 0. **The shot store keys a shot by its photo's digest**, not by `{session}-{N}`, which is what my first draft of the test assumed. `bars` counted nothing (`There is no .app/reads.jsonl`), since a facade-recorded shot logs no read. That is P28's Record-tab run to fill.
3. **What was decided here that the Do line left open, each open to challenge.**
   - **P20 finding 9's question: a share answering `false` records nothing**, and the tab says `The share sheet closed with nothing sent, so this does not count as the last export.` No file left the phone, so its shots are still unsent, which is what the count is for. A `PlatformException` from the sheet shows Swift's sentence and records nothing too. A Rust refusal shows as it stands, with no share. However an export ends, `export_status` is read again, so the call order on success is `export_sessions`, `share`, `set_state`, `export_status`, and the stored time is the summary's `exported_at` and not the clock (P13 finding 13).
   - **The zip's path is `<container>/tmp/export/golf-coach-<install>-<YYYY-MM-DD-HHMMSS>.zip`**, built by `exportPath` from the data root, which is the one `documentsDir` answer. On iOS the `tmp` beside `Documents/` is the app's own temporary directory, the one `PhotoSource` copies into. The time is the `now` the zip is cut under, with its colons taken out, since macOS shows a colon as a slash. **Only the newest zip is kept**: `tmp/export/` is removed before each export, and never after a share answers. AirDrop's completion handler has not been seen on a device, and deleting on its answer could cut a transfer short.
   - **Add a golfer pushes setup's own name screen with `create: model.addGolfer`, then replaces it with the bag screen.** `AppModel.createGolfer` is setup's step and moves the stage to `bag`, which would swap the Shell for setup's bag screen and lose the Record tab's state, a library batch's included. So `addGolfer` creates through `create_golfer`, makes the golfer current by `chooseGolfer`'s rule (a club their bag does not hold is cleared), and leaves the stage at `ready`. A known name keeps its hand, and the bag screen says so. A refusal stays under the field. A tap on another golfer is `chooseGolfer`, and a tap on the current one writes nothing.
   - **The rows, in order**: the golfers (each with its hand, the current one ticked) and `Add a golfer`. Then the bag (its clubs in bag order, a tap pushing `BagScreen` with no `onDone`) and the appearance (`Dark`, `System`, `Light`). Then the export: the two status lines, the cumulative note, `Export and share`, a bar while it is out, the outcome, and the delete warning. Then About: the three version keys, `Launch monitor` as Rust spells it (`full_swing`, with no Dart name table) and `Core build`. A golfer's or an appearance's refusal shows in a snack bar, as the bag screen's does.
   - **Reads**: `export_status` on entering the tab, after every export, and while it is shown on `shotsChanged()`, with a ticket. `about` is read once, on the first entry.
   - **Dart's own sentences**, under rule 8: the title, `Add a golfer`, the bag's hint and empty lines, the never-exported, cumulative, not-shared and delete-warning sentences, the two status templates and the row labels. None is about a value. The templates put Rust's display and counts in words: `N new since then` appears only once there has been an export, and nothing is subtracted.
4. **Each guard was shown to fire, and each file was restored and checked by `shasum`, with a fresh mtime.** Twenty-three mutations of `settings_screen.dart`, `app_model.dart` and `shell.dart` each failed `test/settings_test.dart`. The shell without the screen failed 21. The bag row writing lofts failed 5. The clock as `last_export_at`, no read after an export and no tick failed 3 each. A status read on every change, Add a golfer through setup's step, and a refusal unshown failed 2 each. Each of these failed 1:
   - a closed sheet recorded;
   - the asked path shared and not the summary's;
   - a status read only on entry;
   - old zips kept;
   - the shots since subtracted;
   - the handedness note surviving a pick;
   - `about` on every entry;
   - the button never held;
   - no bar;
   - the sheet's failure unshown;
   - the current golfer chosen again;
   - the appearance not stored;
   - the not-shared line unsaid;
   - `1 shots`.

   One survived the first pass, **the last outcome not cleared as the next export starts**, because the `finally` sets it anyway. So the screen was right, but no test held the sentence while an export was out. The held-export test now starts from a refusal and checks that the sentence is gone while the next export is out, and the mutation then failed it.
5. **For P28**: the tab is `Icons.settings_outlined`. These are the keys:
   - the list `settings-list` and the title `settings-title`;
   - `golfer-<id>`, `golfer-add` and `bag-row`, and `appearance` (the `SegmentedButton`);
   - `export-last`, `export-shots`, `export-button`, `export-busy` and `export-outcome`;
   - `delete-warning`;
   - `about-analysis`, `about-screen`, `about-career`, `about-device` and `about-build`, each with its `value`.

   **The list builds lazily, so scroll to the export button first.** **While an export is out, its bar keeps `pumpAndSettle` from returning**, so poll on the phone's answer, as `export_test.dart` does for its second export. `export_test.dart`'s `ExportPhone` is the pattern for `whole_flow_test.dart`: `documentsDir` answers `$scratch/Documents`, so the zip lands in the scratch's own `tmp/export/` and goes with it, and `share` copies the zip out and answers `true`. **A `TestPhone` whose `documentsDir` is the scratch itself puts the zip in the scratch's parent's `tmp/export/`**, outside the scratch and never cleaned up. `zipEntries` in the same file reads a zip's names, methods and stored bytes with no package.
6. **For P30: unproven here.** These all need the device:
   - the share sheet itself, and AirDrop to the Mac;
   - whether `completed` is `true` only once AirDrop has finished, which is what keeping the zip until the next export guards against;
   - Save to Files;
   - how long an export of a 30-photo HEIC session takes, with photos stored and not deflated;
   - the zip's name as AirDrop shows it;
   - the three appearance segments, and the About rows' values, on a phone narrower than the iPhone 17.
7. **For P31**: the button reads `Export and share`, and the zip arrives as `golf-coach-<install>-<date>-<time>.zip`, which the run sheet renames to `<session-date>.zip`. The tab's own warning says what C20's third warning says.
8. **For P33**: the screen is one `ListView` of sections, each under a `_Heading`, so a "Use scanner" toggle is a section of its own. `AppState` has no field for it, so that is P33's to add.
9. **For P35**: ADR-034's addendum should say that a share closed with nothing sent records no export, and that only the newest zip is kept in `tmp/export/`. `app/README.md` can say where the zip is cut.

### P28 (2026-11-06)

*Everything read off the six captures below is `provisional: pasted copies`, read by the Simulator's iOS Vision and not a device's. The share sheet was answered by the test, not by a finger.*

1. **M38 is desk-complete, and every clause of the Done line was run.** `app/tool/whole_flow.sh` exits 0, twice, in 65 s each (23 s of test after the Xcode build, then the host's three checks). In it the test passes, `truth` holds, `golf-lab import-export` copies 26 files into empty scratch directories and ignores 7, and `compare` prints `Corpus of aaron: equal, 6 swing(s) and 0 excluded on each side`. The 7 are the six `.app/boxes/` files and `.app/reads.jsonl`, one more than P27's six, since this export carries a log. **The workspace: 2,161 passed, 0 failed, 27 ignored, over 132 result lines.** The 27 is P2's 26 plus P6's timing line, so no gate moved. The other checks:
   - `cargo clippy -p app --all-targets --no-deps -- -D warnings` is clean, and the workspace's clippy still prints 28 `warning` lines. `cargo fmt --check` is clean.
   - Docs truth passes 91. `flutter analyze` is clean, `dart format` changes nothing over 53 files, and `flutter test` passes 145.

   No Rust and nothing in `crates/app/src/api/` moved, so `codegen_check.sh` did not run. Nothing under `crates/`, `src/`, `tests/`, `scripts/`, `spec/` or `spikes/` is newer than the marker, and `project.pbxproj`'s digest is still P21's `7130d00c…`. The new files are `app/integration_test/whole_flow_test.dart` and `app/tool/whole_flow.sh`, and `ROADMAP.md`'s NEXT ACTION now says desk-complete, waiting on the iPhone for P29.
2. **The table.** It is `golf-bay-report truth` on the whole flow's export, beside P18's Simulator column and M33's:

   | | P28, the whole flow's export | P18, Simulator | M33, macOS |
   |---|---|---|---|
   | T1 wrong on unflagged photos | **0** (caught 0) | 0 (caught 0) | 0 (caught 0) |
   | T2 read cells | 71 of 72 (98.6%) | 71 of 72 (98.6%) | 71 of 72 (98.6%) |
   | T3 the lowest field | `launch_direction`, 5 of 6 (83.3%) | `launch_direction`, 5 of 6 | `sidespin`, 5 of 6 |
   | T4 flagged photos | 0 of 6 | 0 of 6 | 0 of 6 |

   All six captures matched by digest. The one withheld cell is `2026-10-07-1`'s `launch_direction` (truth 3.5), as P18 finding 9 said. Both runs gave the same table, under install ids `7ecf` and `2399`. Per photo, Simulator library reads: `ms_vision` was 1419 to 1783 ms, `ms_rust` 18 to 26 ms and `ms_total` 1491 to 1881 ms. None of these is T5's. `target/m38/whole-flow.txt` holds the run and `target/m38/whole-flow.zip` the export, both gitignored, and the next run rewrites both.
3. **What the flow did, through the widgets alone, from a root holding nothing.**
   - **Setup**: the install id minted, the golfer `Aaron`, then a bag of the 7i with a loft of 34. The Change sheet then picked the 7i.
   - **The read**: the six captures as one library batch through the Simulator's Vision, with no challenge on (P23 finding 6). All six were `read`, none flagged, and `reads.jsonl` held six `library` lines.
   - **The correction**: from History, shot 1's (`IMG_2889`) detail screen, then Edit. The carry, read as `131`, was typed over with `123.4` and saved. The detail screen read the shot again and showed `123.4` marked `corrected`. Back on History, shot 1 alone carried `mark-corrected`, and `list_sessions` agreed. The stored shot holds `123.4` with `corrections` `{"carry_distance":"131.0"}`, and so does the shot inside the zip, inflated by hand.
   - **Profile**: the 7i read `6 tracked shot photos`, with the carry's mean `155.6   95% CI 80.8 .. 230.4   (median 142.7)`, equal to Rust's view. P26's mean was 156.8 before any correction. Then Player.
   - **Export**: Settings, a zip of 33 entries, and the tab then read `6 shots on this phone, 0 new since then.`

   **The correction goes through History's detail screen and not the Record card**, because that is run sheet step 8's path. It makes this the first run to reach the detail screen from a Vision read (P25 finding 7). The card's path is P22's test. It is also the first run to reach the Profile tab from Vision reads (P26 finding 7), and the first export with a log that `bars` can count (P27 finding 2).
4. **What was decided here that the Do line left open, each open to challenge.**
   - **A forced review is saved as it stands, and the count is held to the log.** Anything flagged is reviewed once the batch's last photo is in (C14). The set the test reviewed must equal the swings `reads.jsonl` calls `flagged`. Both are empty on the Simulator, so this branch has not run through the real bridge (P22 finding 7 still stands). The loop asks for a review before it asks for the library button, since a review route leaves that button offstage.
   - **The install id is the app's own mint each run, not an injected one**, since "a fresh root" includes the first launch. The session id is derived from it. `player_id` `aaron` is a constant in both files, and the test asserts that setup made it.
   - **The script deletes any earlier `whole-flow.zip` before the test runs**, so a stale zip cannot pass for this run's. Its checks run debug builds through `cargo run -q`, built once first, since a release `golf-lab` would build `ort` again for nothing.
   - **`bars --source all` runs last, for its numbers alone.** It reads `INVALID` by construction, six library photos being no session, and a verdict exits 0.
5. **For P31 and P32: `bars` counts a person's correction against the reader, so a wrong correction is a T1 miss.** The test typed `123.4` over a carry the reader got right (`131` is M33's truth). On this export `truth` says 0 wrong, and `bars` reads `T1 wrong cells on photos not flagged 1 … MISS` and T3 `carry_distance` 5 of 6. That is the bars file's definition, since `bars` trusts the review (P14 finding 3). It also means one mistyped correction at the bay is a T1 miss, and a T1 miss alone triggers P33 (D12, D14). **For P31**: step 8 should say to change a value only where the photo shows the card is wrong. **For P32**: check each `wrong` cell `bars` names against its photo before reading T1 as the reader's.
6. **Each guard was shown to fire.**
   - **Dart**: `ui/shell.dart` passing a no-op `openShot` made the test fail with `nothing found by … ShotDetailScreen in 20 s`. `whole_flow.sh` then exited 1 before the host checks, without printing `held`. The file was restored by `cp` with a fresh mtime, and its shasum (`388f92d4…`) checked.
   - **The host half**: run from a scratchpad copy over the passing run's zip.
     - A truth file with `IMG_2889`'s backspin changed from 9899 to 9898: `truth` exited 3 on `IMG_2889 backspin: wrong, as read 9899.0, truth 9898.0`, and the script stopped there.
     - One imported manifest's club changed from 7i to 8i: `compare` exited 3 on `DIFFERENT, first at swings[1].club`.
     - Unmutated, it exited 0.

     So `cargo run` passes a verb's exit code through, and `set -e` stops on a 3 from inside the `bay` function.
7. **`bars` on this export, for P30 to read beside a device's.** These are the Simulator's numbers, read in an `INVALID` session.
   - The smallest sign-letter margins were **+0.420 lit and −0.133 unlit** against `LIT_BLUE_RATIO` 0.7, every lit letter yellow.
   - Of 247 boxes, 246 were at 1.00 and 1 was a stray at 0.30, behind no cell.
   - `ms_total`'s p95 was 1818 ms, for Simulator library reads.
8. **P11 finding 11's question, raised here as it asked, and not a stop.** The Profile tab shows contract names (`carry_distance_yds`, and each finding's `metric`), and so does the detail screen (`start_line_offline_yds`, `flight_carry_yds`, P25 finding 5). On the iPhone 17 they read as code, not words. Rule 8 keeps any fix out of Dart. **Options**:
   - leave them, and P35 records them as a follow-up in ADR-034's addendum;
   - or add a Rust `label` beside each `name` in `crate::history` and `crate::profile_view` as a phase of its own before P29, which would take a codegen run and a Dart edit.

   I recommend leaving them. The bay session measures the read, and run sheet step 7 asks only that the profile renders.
9. **For P35**: `docs/ARCHITECTURE.md` §1 and `app/README.md` should name `app/tool/whole_flow.sh` as the desk gate, with its three host checks, and say that it boots and shuts down the Simulator. **For P36**: it is one of the Verify lines, about 65 s once the bridge is built, and it needs nothing exported first. It rewrites `target/m38/whole-flow.zip` and `whole-flow.txt`.
10. **P23 finding 6's gap is closed.** The whole flow runs the Record tab's batch over the flow as P23 left it, and then `integration_test/record_test.dart` and `review_test.dart` were re-run on the Simulator too, outside the Verify line. They passed in 14 s and 8 s of test, the first Simulator runs of either since P22.

### P29 (2026-11-08)

Attended, with the user at the Mac. No Rust, Dart, Swift, `spec/` or project-file change: the one file written is the
gitignored `app/ios/Flutter/Signing.xcconfig`. No team id, Apple ID or device UDID is written here, because this file is
committed at P36 (C19's reason for the xcconfig).

1. **The device: an iPhone 15 Pro Max (`iPhone16,2`), iOS 26.6.2 (23G90)**, above D2's floor of 16. It was already paired and
   trusted when the phase began (`devicectl`: `pairingState: paired`, `tunnelState: connected`), with
   `developerModeStatus: disabled`.
2. **Developer Mode needs pairing, not a build.** The user had read that the switch stays hidden until a developer build is
   on the phone. Apple's page says otherwise: *"Developer Mode only appears in Settings if you initiate pairing or if you
   previously paired the device to a Mac"*, and the switch was there (Settings → Privacy & Security, at the foot, under
   Security). After the restart, `developerModeStatus: enabled` and `ddiServicesAvailable: true`.
3. **Xcode had no Apple ID**, and 0 signing identities. The user added one, and Xcode's `IDEProvisioningTeamByIdentifier`
   then held one free Personal Team (`isFreeProvisioningTeam = 1`). Its id is in `Signing.xcconfig` with
   `CODE_SIGN_STYLE=Automatic`, and `git check-ignore -v` answers `.gitignore:75`. The first signed build created the
   one "Apple Development" identity in the login keychain. **Apple accepted `dev.golfcoach.lab`**, so C19's fallback id
   was not needed.
4. **`flutter build ios --release` alone signs for the wrong device.** It built and signed in 40 s, but its profile listed
   only a different device already registered to the team, and `devicectl device install app` refused it:
   `0xe8008012 (This provisioning profile cannot be installed on this device.)`. A build with no destination never
   registers the connected phone. **`flutter run --release -d <udid>`** passes the destination, so Xcode registered the
   phone and the next profile listed it alone. Its build took 14 s.
5. **The launch was refused until the profile was trusted, as planned.** `flutter run` installed the app and then exited 2
   with "Could not run … Try launching Xcode", and `devicectl device process launch` gave the reason: *"its profile has
   not been explicitly trusted by the user"*. The user trusted it under Settings → General → VPN & Device Management.
   Xcode's Product → Run fallback was not needed.
6. **Done, unplugged.** The app opened from the home screen with no cable. The user confirmed the three checks ("It looks
   great!"), and the container on the phone shows the golfer that setup made: `devicectl device info files --domain-type
   appDataContainer --domain-identifier dev.golfcoach.lab` lists `Documents/processed/golfers/aaron.golfer.json` and
   `aaron.bag.json` (12:04) and `.app/state.json` (12:05).
7. **The project file is untouched.** `project.pbxproj`'s digest is still P21's `7130d00c…`, and it holds no
   `DEVELOPMENT_TEAM`.
8. **Install: 2026-11-08 12:02 −06:00. The profile expires 2026-11-15 12:02:25 CST** (C20's 7 days).

**For P30**: the phone is set up, and `flutter devices` names its UDID. To see what the app stored without an export,
`xcrun devicectl device info files --device <coredevice-id> --domain-type appDataContainer --domain-identifier
dev.golfcoach.lab` lists the container, and `devicectl device copy from` can fetch a file. The share sheet's AirDrop is
still what the Done line tests.

**For P31**: re-sign with the phone connected, using `flutter run --release -d <udid>` and not `flutter build ios`
alone (finding 4). Then read `ProvisionedDevices` in the built `Runner.app/embedded.mobileprovision`
(`security cms -D -i …`) and check it lists this phone before calling the install fresh. Expect exit 2 and a refused launch
if the profile has to be trusted again. The current profile lapses on 2026-11-15 at 12:02, so the bay session must happen
before then, or after P31 re-signs.

### P30 (2026-11-08)

Attended, with the user's direction to skip every step needing their hands and run the rest, so this records what ran
and what did not. It was cut short once, at item 9, and resumed the same day. **Anything run on the phone reads items 6
and 10 first.**

1. **The user's own library photo was the original HEIC of `IMG_2894`**, whose pasted copy is one of M33's six. So it is
   not a pasted copy, though the report's header still says `provisional: pasted copies`, and the truth it was held to is
   M33's. The app stored it as `shot_screen.227d9985b337.heic` (`image/heic`, 4,483,358 bytes, 5712×4284) with
   `original_filename: IMG_2894.heic`, so the asset's own name survives the picker (P20 finding 12, P15 finding 10). Its
   EXIF date named the session `2026-10-10-phone-ba90`, and the photo's modification time is that capture time (C9).
   It was a `library` read, under `pw`.
2. **`truth` on it: 12 of 12 cells read, 0 wrong, 0 withheld, not flagged.** T1 0, T2 100%, T3 100%, T4 0 of 1. The verb
   labels it `name: a re-encoded copy`, the inference P15 finding 12 warned of: it is the original and the copy is M33's.
   43 boxes, every one at confidence 1.00.
3. **The first sign-letter margins on a HEIC: +0.464 lit and −0.138 unlit** against `LIT_BLUE_RATIO` 0.7. The lit `R` and
   `D` were yellow (`rgb` 250,223,53 and 250,218,29), and there was one stray unlit `D` at 4658,2565, behind no cell. No
   letter was on the wrong side, so rule 11's second stop did not trip. P28's Simulator figures, on JPEG copies, were
   +0.420 and −0.133.
4. **Latency on the device, one library HEIC: `ms_total` 831** (`ms_vision` 782, `ms_rust` 44). That is not T5's measure,
   which is camera photos, and P28's Simulator p95 was 1818 ms on JPEGs.
5. **`integration_test/phone_test.dart` passes on the iPhone, 4 of 4**, with `cameraAvailable` `true`.
6. **`flutter test` on a physical iPhone uninstalls the app when the run ends, and that deletes its data container.**
   It is `flutter_tools`' integration-test device, which stops and then uninstalls the app. It deleted the user's shot,
   golfer and bag at 12:17. They were restored from a copy pulled at 12:16, before any test ran
   (`target/m38/device-backup-20261108-1216/`, `devicectl device copy from --source Documents`): the release build was
   reinstalled, `devicectl device copy to --source <backup> --destination Documents` put it back, and `diff -r` against
   a second pull matched all 9 files, with modification times kept. **Removing the team's last app also drops the
   developer trust**, so the reinstalled app was refused launch until the user trusted it again.
   **For P30's resume, P31, P33 and P34**: never run `flutter test … -d <phone>` on a phone holding real shots. Pull
   `Documents` first, whatever runs. `devicectl device copy to` an app's container fails with `ContainerLookupErrorDomain
   error -1` while the app is not installed.
7. **The six captures were not read by the device's Vision.** `vision_gate_test.dart` and `whole_flow_test.dart` now take
   `repo` from `integration_test/test_host.dart`: `GOLF_REPO=@tmp` resolves to the container's `tmp/m38-repo/`, and any
   other value is the host repo as before, with the same `sim-c0` names and headings on the Simulator. `flutter analyze` is
   clean. The device runner written for it was deleted unrun, because `flutter test`'s uninstall would take the staged
   inputs, the outputs and the user's data with it. The untried route is `flutter drive` with a
   `test_driver/integration_test.dart`. Its help says it *stops* the app when done and says nothing of uninstalling, so
   that is to be proved with a backup taken and a pull compared after. Or the user imports the six from the library, as
   step 1 asks.
8. **Since P29 registered the phone, `flutter build ios --release` signs a profile that lists it**, so a build then
   `devicectl device install app` reinstalls with no `flutter run`. P29 finding 4 held only before registration. The
   reinstall at 2026-11-08 12:21:26 −06:00 reused the same profile, which still lapses on 2026-11-15 at 12:02:25 CST.
9. **Unproven, for the resume:** the in-app camera going straight to a read, a landscape photo reading upright from its tag
   (this HEIC is 5712×4284 with no rotation to undo, so it proves nothing about the tag), a stock-Camera HEIC, the share
   sheet and AirDrop, and the six captures' table on the device.
10. **`flutter drive` uninstalls the app too**, though its help speaks only of stopping it. `vision_gate_test.dart` ran
    through it at 12:38 and passed, and the app was gone afterwards, with the trust dropped again. The data came back from
    the pull `device_test.sh` took before the run (`target/m38/device-backup-20261108-123825/`). `device_restore.sh`
    reinstalled the release build and copied the backup back, and a second pull matched it byte for byte.
    **For everyone**: `app/tool/device_test.sh <test>` now pulls `Documents/`, runs the test through `flutter drive`, keeps
    the console in `target/m38/device-<test>.log`, and hands the pull to `app/tool/device_restore.sh [<backup>]`. That
    script reinstalls the release build, restores the data, holds the phone to it, and says whether the app launches.
    **Each run costs the user a re-trust**, so never run it in the week of a bay session, or on a phone whose shots are
    not exported. `--keep-app-running` is untried.
11. **The six captures through the iPhone's own Vision: 6 of 6 matched by digest, 0 wrong, 0 caught, 69 of 72 cells read
    and 3 withheld** (`provisional: pasted copies`, device, iOS 26.6.2). `IMG_2893` withheld 2 and `2026-10-07-1` withheld
    1, and the other four read 12 of 12. The test's three assertions held: no wrong cell, `held`, and M33's control.
    Beside P18: the Simulator and macOS each read 71 of 72. So the phone withholds more and still guesses nothing, and T2's
    95.8% stands against its 92% bar. iOS's log cut the printed report off after this table, and the file the test wrote
    went with the container. So the per-read lines, the boxes against macOS's, and the device's margins and latency on
    the JPEGs are not recorded. `printReport` in `integration_test/test_host.dart` now prints a line per call.
12. **Against the Done line.** Met: HEIC bytes stored as `.heic`, read and shown in History (items 1 and 2, with the
    user's "looks good on my end"); the table (item 11); no hard stop, with 0 wrong cells over seven device reads, where a
    flipped sign would have been a wrong or caught cell; the smallest lit margins on a HEIC (item 3); and a latency
    (item 4). **Waived by the user (D16)**: the in-app camera giving a read with no confirm step, a landscape photo
    reading upright from its tag, a stock-Camera HEIC beyond the user's own, and the share sheet delivering the zip.
    **For P31**: the bay is where these are first met, so the run sheet should open with a one-photo smoke test: one
    in-app camera photo, landscape, and a card with values. If the camera fails, keep the session with the stock Camera
    and library import, which items 1 and 2 prove. `bars` calls a library session `INVALID`, but the data survives. Then
    export and AirDrop one zip before the thirty, so the share sheet is proved while there is time to fall back to
    `devicectl device copy from` at the Mac. P31 also re-trusts if any device run happened after this phase.
13. **For P30b: the app records reads and nothing else.** There is no handler for an uncaught Dart error
    (`FlutterError.onError`, `PlatformDispatcher.onError` and zones appear nowhere in `app/lib`). Nothing records a review
    opened or saved, a club change, a delete, a camera cancel or an export, beyond what each leaves in the stores.
14. **For P32: crash reports.** `xcrun devicectl device copy from --device <id> --domain-type systemCrashLogs --source /
    --destination <dir>` should fetch the phone's crash reports, and the ones naming `Runner` are the app's. Untried.
15. **For P35**: `app/README.md` and `docs/ARCHITECTURE.md` §1 should name `device_test.sh` and `device_restore.sh`
    beside `sim_test.sh`, with item 10's cost. This phase's files: `app/integration_test/test_host.dart` (new),
    `vision_gate_test.dart` and `whole_flow_test.dart` (`repo` from it, and `printReport`, with the Simulator's names
    and headings unchanged), `app/test_driver/integration_test.dart` (new, the package's own driver; `flutter_driver`
    was already in `pubspec.lock` through `integration_test`, so the lock did not move), and the two scripts.
16. **The Simulator after the `repo` change**: `app/tool/whole_flow.sh` exits 0 (12:43), with the six at 71 of 72, 0
    wrong, as P18 and P28 read them, and the log in `target/m38/p30-whole-flow.log`. `flutter test` passes 145, and
    `flutter analyze`, `dart format` and docs truth (91) are clean.

### P30b (2026-11-08)

1. **The event log is in, and every clause of the Done line was run.** `crates/app/src/api/record.rs` gained
   `EventKind` (P30b's fourteen kinds, a plain enum spelled snake_case), `AppEvent { at, kind, detail }` and
   `log_event`, beside `log_read`. Both now go through one `append_line` in `crates/app/src/record.rs`
   (`READS_NAME`, `EVENTS_NAME`), which refuses a time with no offset and writes one `write_all` per line.
   `crates/app/src/export.rs` adds `.app/events.jsonl` beside the reads. `cargo test -p app`: 344 passed, 1
   ignored. `tests/record.rs` gains 5 (the line's keys in order, every kind's spelling both ways, a detail with
   quotes, newlines and non-ASCII stored as given and still one line, the directory made, a naive time refused
   with nothing written). `tests/export_roundtrip.rs`: the zip's exact list now holds `.app/events.jsonl`, an
   unnamed `.app/stray.jsonl` stays home, and the real `import-export` ignores 6 `.app/` files where it was 5,
   copying none. `cargo clippy -p app --all-targets --no-deps -- -D warnings` and `cargo fmt --check` are clean,
   and `codegen_check.sh` passes with 25 functions bound (24 before). **Without `--no-deps`, clippy stops in
   `impact` and `storage`** on two newer lints (`manual_is_multiple_of`, `sort_by_key`), which are among the 28
   the plan leaves alone. Nothing in `spec/` moved, and `project.pbxproj`'s digest is still `7130d00c…`.
2. **Dart.** `GolfCore.logEvent`, one `_refusing` line in `RustCore`, and in `test/fakes.dart`
   `FakeCore.events`, `refusedEvent` and `eventsOf(kind)`. **An event is not in `FakeCore.calls`**, whose order
   the tests before this pin. `AppModel.logEvent(kind, detail)` stamps `at` by the model's clock and **chains each
   write after the last without awaiting it**, so the file holds events in the order they happened and nothing a
   person does waits on one. A failed write goes to `debugPrint` alone. `eventsWritten` is the chain, for tests.
   `logProblem(where, sentence)` is an `error` reading `where: sentence`. `logError(error, stack, context:)` is
   an `error` of the message, Flutter's context if given, and the stack's first `AppModel.stackLines` (5)
   non-empty lines, joined by newlines, and it catches everything. `main.dart`'s `logUncaughtErrors(model)` sets
   `FlutterError.onError` (logs, then calls the handler before it) and `PlatformDispatcher.instance.onError`
   (logs, then answers what the one before would have, `false` with none, so an unhandled error is still
   reported as unhandled). Before `start` has the data root there is nowhere to write, so nothing is: `_load` now
   sets the root before reading the state, so a state that cannot be read is logged.
3. **Where each kind is written, and its detail.**
   - `app_start`: `AppModel._load`, the stage it opens on (`ready`, `golfer`, `bag`, `failed`), followed by
     `error` `start: …` on `failed`.
   - `club_changed` and `mode_changed`: `AppModel._logChanges`, after `editState` and `chooseGolfer`, the state's
     two writers between swings, so the change sheet and Settings need no line of their own. `club_changed` is the
     club or `none`, plus `golfer <id>` when the golfer changed (`pw, golfer sam`, `none, golfer sam`), since a
     golfer has no kind of its own. `mode_changed` is `intentDetail(state)`: `fundamentals`, `shotShaping draw`,
     `shotShaping draw, challenge straight draw fade`. The appearance and the last export log nothing. **Setup's
     own steps (`createGolfer`, a bag slot) log no event**, only a refusal.
   - `camera_opened` and `camera_cancelled`: `RecordFlow.takePhoto`, both `<club>` or `<club>, call <shape>`.
   - `library_picked`: `RecordFlow.importPhotos`, `2 photos, 1 skipped, 7i`, a cancel as `0 photos, 7i`.
   - `review_opened`: `showReview`, both ways in, `<session>/<swing>`, then `forced`, then `2 of 3` in a batch.
   - `review_saved`: `<session>/<swing>, 1 edit`. `intent_set` too, when a review stored one.
   - `shot_deleted`: `<session>/<swing>`, from a review or the detail screen.
   - `club_set`, `intent_set`, `mishit_set`: the detail screen's `_act`, `<session>/<swing> pw`,
     `<session>/<swing> shotShaping draw`, `<session>/<swing> confirmed` (`cleared`, `automatic`).
   - `export_shared`: Settings, `completed true, 6 shots`, once the share sheet answers.
   - `error`: the two handlers, **and every refusal or failure a person is shown, as `where: sentence`**, since a
     sentence on screen is gone at the next tap and nobody else is at the bay. The `where`s: `start`, `camera`,
     `library` (the picker's refusal, each skipped photo, a batch refused under a challenge), `read` (a photo
     Swift could not open, in none of the bay's counts), `record`, `reads.jsonl`, `review`, `delete`,
     `shot detail`, `export`, `share`, `settings`, `change sheet`, `history`, `profile`, `bag`, `setup`.
4. **`app/test/events_test.dart` is 26**, a test per kind through the screens over the fakes, plus a refused save,
   a refused action, a failed share, a Record tab refusal, an unopenable photo, a state that cannot be read, a
   thrown exception (through a button: logged with `while handling a gesture` and 5 stack lines, and still
   reaching the harness, `takeException`), an unawaited error (answered as the handler before answered, and
   `false` with none), and a log that refuses every write while a photo is read, recorded and logged as a read
   with no problem shown. `flutter test` passes 171 (145 before). `flutter analyze` and `dart format` are clean.
   **Five mutations each failed the tests meant for them**, each file put back and `cmp`-ed: no event from the
   detail's `_act` (3 tests), a write failure not swallowed, the harness's handler not called, an unhandled error
   answered as handled, no `camera_cancelled`.
5. **`app/tool/whole_flow.sh` exits 0** (13:02, log `target/m38/p30b-whole-flow.log`). Its zip holds
   `.app/events.jsonl` (a new host check, `unzip -l`, before `truth`), the test holds the file's kinds in order
   (`app_start golfer; club_changed 7i; library_picked 6 photos, 7i; review_opened …/1; review_saved …/1, 1 edit;
   export_shared completed true, 6 shots`, now in the report as `- events:`) with no `error`, and the zip's copy
   one line short of the phone's. `import-export` copies 26 and ignores 8 (7 at P28), `compare` is equal, and
   `truth` 71 of 72 with 0 wrong, as at P28 (`provisional: pasted copies`, the Simulator's Vision).
6. **Not run on the phone**, as the section says (P30 finding 10). **For P31: the phone holds P30's build, which
   has no event log.** P31's `flutter run --release` reinstalls from this tree, which is what puts it there, so
   that reinstall must come after P30c, and the run sheet's smoke test can check Settings → Export holds
   `.app/events.jsonl` once the zip is on the Mac.
7. **For P30c, what the log is and is not.**
   - One JSON object a line, `{at, kind, detail}`, kinds as in item 3. `detail` is free text for a person:
     **print it, do not parse it**, except that each shot's kind leads with `<session>/<swing>` up to the first
     `,` or space. An `error`'s detail can span lines (message, context, up to 5 stack lines), so print it
     indented under its time.
   - **A back-out of a review that is not forced writes nothing.** So pair a `review_opened` with the next
     `review_saved` or `shot_deleted` naming the same shot before another `review_opened`, and count an open with
     neither as left. A forced review cannot be backed out of.
   - **`export_shared` is written after its zip was cut**, so a zip never holds its own share's line: the last
     share in a zip is the one before.
   - `at` is Dart's clock when the event happened, and the chain keeps lines in that order, but `reads.jsonl`'s
     `at` is written after the card's first frame, so a read's line can trail the next event by up to `ms_total`.
     Sort by `at` before interleaving the two logs.
   - A second `app_start` in a session is a relaunch (a kill or a crash). Camera cancels are `camera_cancelled`.
     A failed photo is `reads.jsonl`'s `failed` line, and the retake that followed is the next camera read.
   - An export from before P30b has no `.app/events.jsonl`: say so, as `bars` does for `reads.jsonl`, and do not
     fail. A line that does not parse should be a note naming its number, as `bars` does.
8. **Limits, recorded.** An error before `documentsDir` answers is not logged. A native crash in Swift or Rust
   never reaches Dart (P30 finding 14 is the route). A crash just after an action can lose that action's line,
   since the write is not awaited. `debugPrint` is the only trace of a failed write.
9. **For P35**: C7's `.app/` list and C16's export list gain `events.jsonl`, in ADR-034's addendum,
   `docs/ARCHITECTURE.md` §4 and `app/README.md`, and §1 can name `logUncaughtErrors`. This phase's files:
   `crates/app/src/api/record.rs`, `crates/app/src/record.rs`, `crates/app/src/export.rs`,
   `crates/app/tests/record.rs`, `crates/app/tests/export_roundtrip.rs`, the generated bridge,
   `app/lib/main.dart`, `app/lib/src/core/core_api.dart`, `app/lib/src/state/app_model.dart`,
   `app/lib/src/state/record_flow.dart`, the screens `review`, `shot_detail`, `settings`, `history`, `profile`,
   `bag` and `setup`, `app/lib/src/widgets/change_sheet.dart`, `app/test/fakes.dart`, `app/test/events_test.dart`
   (new), `app/integration_test/whole_flow_test.dart` and `app/tool/whole_flow.sh`.

### P30c (2026-11-08)

1. **`golf-bay-report events <export.zip|dir> [--since RFC3339] [--json]` is in, and every clause of the Done
   line was run.** It lives in `crates/app/src/bay/events.rs`, a submodule of `bay` so the 2,479-line file did
   not grow by another verb, and the bin's verb table gains `events`. It prints the counts by kind (all fourteen,
   a zero included, in `EventKind`'s order, held by a `place` match with no wildcard so a new kind does not
   compile until it is placed), each `app_start` as a launch, each `error` verbatim and indented under its time,
   the camera's opens and each cancel, the reviews (opened, saved, ended by a delete, left) with opened-to-saved
   p50, p95 and max, the read lines with the gaps between consecutive recorded shots (p50, p95, max), and each
   failed photo beside its retake. **It judges nothing**: exit 0 whenever the export reads, 2 on a flag or a file
   it cannot read, as `bars`. `m38-bay-thresholds.md` is not opened.
2. **`bay.rs`'s reader is now shared, not copied.** `read_log` and `lines_since` became a `LogLine` trait
   (`FILE`, `WHAT`, `NOUN`, `at`) with `read_jsonl<T>` and `lines_since<T>`, implemented for `ReadLog` and
   `AppEvent`, and `bars`' and `truth`'s notes spell exactly as before. **Checked, not assumed**: `bars --source
   all` on P30b's `target/m38/whole-flow.zip` is byte-identical to the `bars` section of
   `target/m38/p30b-whole-flow.log` (65 lines, `diff` clean).
3. **The rules it reads by**, each in the module doc, from P30b finding 7: a detail is printed and never parsed,
   except a shot's subject up to the first `,` or space; each log is stably sorted by instant and **the two are
   never interleaved**, so every number is one log's; a review pairs with the next save or delete naming its shot
   before another open, and only a save has a time; the gaps are between **every** `read` or `flagged` line, not
   one per photo as `bars` counts, since this is pace; **a failed photo's retake is the next line from the same
   source**, and a library photo has none; the `export_shared` caveat is printed beside the counts. A torn line is
   a note naming its number, a line whose `at` is no time is a note, and an export with no `events.jsonl` says so
   and still prints the reads' numbers. Times between lines are whole milliseconds, printed as seconds to a tenth.
4. **Tests.** `crates/app/tests/bay_events.rs` is 16, over a 21-event, 8-read sitting worked by hand in
   `sitting`'s comment and written by the real `log_event` and `log_read`: every kind's count, the error and
   launches verbatim, the camera and reviews (4 opened: 2 saved in 20 s and 5 s, 1 deleted, 1 left), the gaps (50,
   100, 150 s: p50 100 s, p95 150 s), the three failures (a camera retake 40 s on, a library photo with none
   sought, a camera failure with nothing after), the rendered text line by line, `--since` in another offset over
   both logs (a line at exactly the instant kept), no events log, no reads log, a torn line, file order against
   time order, a save naming another shot, **a zip cut by the real `export_sessions` giving the same report as its
   directory**, and the command (text, `--json` with `--since`, and three refusals exiting 2). Two unit tests hold
   `place` to `KINDS` and the subject rule. `cargo test -p app`: **362 passed, 1 ignored** (344 at P30b).
   `cargo clippy -p app --all-targets --no-deps -- -D warnings`, `cargo fmt --check`, the iOS lib build and docs
   truth (91) pass. No dependency, `api/` function, vector, version key or bar moved, so no codegen ran.
5. **On the real zips.** P30b's `whole-flow.zip`: exit 0, 5 events (`app_start`, `club_changed`,
   `library_picked`, `review_opened`, `review_saved`), 0 errors, one review saved in 1.9 s, 6 recorded shots with
   5 gaps at p50 1.6 s (a library batch on the Simulator, `provisional: pasted copies`). P28's `export.zip`, cut
   before P30b: exit 0, and it says the export holds no `.app/events.jsonl`.
6. **For P31**: P30c changed nothing that ships. `golf-bay-report` is a host bin and the iOS build compiles the
   lib alone, so P30b finding 6 stands as it was: the reinstall from this tree is what puts the event log on the
   phone.
7. **For P32**: the line is `golf-bay-report events <zip> --since <P31's install time>`, beside `bars` with the
   same `--since`. `--since` is printed as it was given (`Z` stays `Z`), as `bars` prints it. **It takes no
   `--source`**, so the gaps and failed photos are over every source: if a library batch was imported at the bay,
   it shows as `library` failures and as short gaps, so read the `Shots:` line against `bars`' by-source counts.
   A shot deleted from History ends no review and appears only in the counts. A review left open is a back-out of
   a review that was not forced, which is not a problem by itself.
8. **For P35**: ARCHITECTURE §1's `golf-bay-report` entry gains a fourth verb, `events`, with `--since` and
   `--json`, exiting 0 or 2. This phase's files: `crates/app/src/bay.rs` (the shared reader and the module doc's
   pointer), `crates/app/src/bay/events.rs` (new), `crates/app/src/bin/golf_bay_report.rs`,
   `crates/app/tests/bay_events.rs` (new).
9. **For P36**: `app/tool/whole_flow.sh` does not run `events`, since it was outside the Touch line and a
   changed script would need a Simulator run to be verified. It was run by hand on that script's zip (item 5).
   Adding `bay events "$zip"` after `bars` is one line, if the close wants the flow to exercise it.
