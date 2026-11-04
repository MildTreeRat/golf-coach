# Plan: M33 — Apple Vision spike: can the phone read Full Swing's screen?

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins, except that **nothing may move a bar in `spikes/vision-ocr/thresholds.md`** (rule 2).

**Planned**: 2026-11-04, on the arm64 Mac, with Rust and ffmpeg installed and the macOS baselines
measured during planning. **Milestone**: [ROADMAP §M33](../../ROADMAP.md#m33-apple-vision-spike--can-the-phone-read-the-screen)
and the program plan's [§M33](m31-m40-shot-first-pivot.md#m33--apple-vision-spike-mac). This file is
the phase list, and where it and either of those disagree, **this file wins**. Three things in them
are deliberately overturned here: the parser is Rust's rather than frozen Python's (decision 1),
Full Swing rather than HD Golf decides the verdict (decision 2), and the gate gains a coverage floor
and a review ceiling (decisions 4 and 7).

**Governing decisions**: [ADR-034](../decisions/034-shot-first-phone-first.md)'s sub-decision "OCR on
the phone" and clause 7, [ADR-014](../decisions/014-screen-capture-shot-ingestion.md) with its M34 and
M42 addenda, [ADR-010](../decisions/010-benchmark-ranges.md) §2 (no score beats a wrong one),
[ADR-035](../decisions/035-rust-everywhere-python-where-required.md) clauses 3 and 4, and
[the M42 plan](m42-full-swing.md) (the truth table, and calls 4 and 5 on `TextBox.rgb` and the lit
sign).

**Level: L2**, and **L3 only if a parser rule fires** (P13–P15). Until then the spike touches nothing
under `crates/`, `src/`, `scripts/`, `tests/` or `spec/`, and every phase reads its own list below
rather than the repo.

**The question, in one line**: on the photos of Full Swing's grid screen we already hold, does Apple
Vision produce boxes that `crates/screen` turns into correct shots, without a laptop? If it cannot,
the app plan changes before any app work starts (ADR-034).

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot — the findings are what the next phase
   is planned against — and address anything a later phase must know to that phase by number
   ("**For P17**: …");
3. stop, and tell the user to `/clear` and run `/next-phase`.

| Phase | What | State |
|---|---|---|
| **P0** | This plan, `thresholds.md`, the pointers, and the pre-registration commit | ✅ Done *(2026-11-04)* — approved by the user |
| **P1** | The photo manifest and the truth table as data; every digest checked | ✅ Done *(2026-11-04)* — on the six pasted copies, by the decision of 2026-11-04 |
| **P2** | `vision-ocr`: `decode`, `read` at C0, and a geometry self-test | ✅ Done *(2026-11-04)* — on `captures/IMG_2889.jpg`, by the decision of 2026-11-04 |
| **P3** | `TextBox.rgb` in Swift, held exactly to Rust's `glyph_rgb` | ✅ Done *(2026-11-04)* — on the six pasted copies, by the decision of 2026-11-04 |
| **P4** | Grid frames off the five clips, with the lab's rule | ✅ Not run: the five MOV clips are not on this Mac (decision of 2026-11-04) |
| **P5** | The lab's working images | ✅ Done *(2026-11-04)* — on the six pasted copies, by the decision of 2026-11-04 |
| **P6** | `vision-ocr rectify`: the phone's rectifier stand-in | ✅ Done *(2026-11-04)* — on the six pasted copies, by the decision of 2026-11-04 |
| **P7** | `probe.py measure`: the comparator, proven on controls | ✅ Done *(2026-11-04)* — on committed vector boxes, with no capture measured, by the decision of 2026-11-04 |
| **P8** | The lab's reader on the same inputs: the baseline rules must not move | ✅ Not run: the OCR models are not on this Mac (decision of 2026-11-04) |
| **P9** | The first measurement: C0 on Full Swing's stills | ✅ Done *(2026-11-04)* — on the six pasted copies, by the decision of 2026-11-04: `raw` and `lab` pass, `vision` misses |
| **P10** | C0 on the clip frames | ✅ Not run: the five MOV clips are not on this Mac (decision of 2026-11-04) |
| **P11** | C0 on HD Golf, diagnostic | ✅ Not run: omitted by the user, and HD Golf's photos are not on this Mac (decision of 2026-11-04) |
| **P12** | The Swift ladder, if C0 missed | ✅ Not needed; the final config is C0 *(2026-11-04)* — stills pass on `raw`, this run's target by the decision of 2026-11-04, and clips are not measured |
| **P13** | Parser rule 1, if still missing | ✅ Not needed *(2026-11-04)*: what is missing is the recognizer's (`IMG_2893` `sidespin` on `raw` and `vision`, and `vision`'s twelve on `IMG_2894`) |
| **P14** | Parser rule 2, if still missing | ✅ Not needed *(2026-11-04)*: what is missing is the recognizer's (P13's same cells: `IMG_2893` `sidespin` on `raw` and `vision`, and `vision`'s twelve on `IMG_2894`) |
| **P15** | Parser rule 3, if still missing | ✅ Not needed *(2026-11-04)*: what is missing is the recognizer's (P13's same cells: `IMG_2893` `sidespin` on `raw` and `vision`, and `vision`'s twelve on `IMG_2894`), and at the cap that is the measured result |
| **P16** | The confidence calibration | ✅ Done *(2026-11-04)* — `fs-still` only, by the decision of 2026-11-04: no threshold from 0.5 to 0.9 changes a flag, and the recommended 0.6 is provisional, with no **wrong** cell to test it on |
| **P17** | The verdict: `log.md`, and what M38 is told | ✅ Done *(2026-11-04)* — **GO, provisional: pasted copies**, clips **not measured**, by the decision of 2026-11-04. `probe.py verdict` reproduces it from `results/C0.json` and holds `log.md` to it |
| **P18** | Docs | ✅ Done *(2026-11-04)* — every touched doc carries **GO, provisional: pasted copies**, clips **not measured**, and the list of what is unmeasured. No rule fired, so CONFORMANCE §2's count, its `hand/` row and `crates/screen/`'s docs did not move |
| **P19** | Close: verify, WORKLOG, ROADMAP, memory, and the commit | ✅ Done *(2026-11-04)* — one commit onto `main`, not pushed. **GO, provisional: pasted copies. Clips: not measured**, and M33 stays open: this run does not meet "Exit", by the decision of 2026-11-04 |

P12–P15 are conditional. A phase whose condition is not met still runs: it reads the results, proves
the condition is not met, ticks its row with "not needed", and records that as its finding.

---

## The user's decision of 2026-11-04: pasted copies, stills only

**Where this section and anything below it disagree, this section wins. It does not touch
`thresholds.md`, whose bars stand (rule 2).**

P1 stopped because none of the captures or models had reached this Mac, and the Windows box is out of
the user's reach. Asked how to proceed, and told that pasted images are not the original files, the
user answered: "I pasted them into this chat. Please save them on the Mac, commit them to the repo, and
then proceed", and "Here are the full swing pictures. Omit HD golf for now."

**The captures are now seven JPEGs in `spikes/vision-ocr/captures/`**, tracked in git and outside
`data/`, so rule 4 holds. Each is the file Claude Code saved from the chat paste: 2576×1932 or 1932×2576,
tagged Display P3, with no camera metadata. They are **copies, not the files the truth table's digests
name**: five of the six stills were HEICs, so they are re-encoded, and the originals' size is not known
here, so they may be scaled down too. Rule 9 still applies, against these digests:

| id | file | sha256 |
|---|---|---|
| `IMG_2889` | `IMG_2889.jpg` | `6f22661840a5e698d0b1c72a10fd7214b0b37fa0301f4af4129223f87cff951b` |
| `IMG_2891` | `IMG_2891.jpg` | `c13f58c6bb5297daf61a1a489f1e73398f2700b97115c847fad66b1631e35ead` |
| `IMG_2892` | `IMG_2892.jpg` | `798395eddca63606a882bb9861b872e81a231a341b4918b5e6a7605d279bce3a` |
| `IMG_2893` | `IMG_2893.jpg` | `240a6cd2caf98e42e90626070f925e6b8bc6ac5d49d1093807d0fb6a22c87d17` |
| `IMG_2894` | `IMG_2894.jpg` | `4f1e91006ad356fb5d7eb1fd7ef68731216ea9e905756cac0f7ba441453d0a5e` |
| `2026-10-07-1` | `2026-10-07-1.jpg` | `f1547903c770c29d9e00548c61b86a5b5fdea2e9ad3b04501b2afbaac072b25d` |
| `IMG_2888` | `IMG_2888.jpg` | `fc583423dbf80bb17ef594909138c337aea2a784ae521f7fcd900f8debe21691` |

Each id was assigned by matching the picture to the truth table by eye, and all twelve values and the
lit letters agree on all six stills. That was checked once, at display resolution, by the orchestrating
session. `IMG_2888` is named by its layout alone (the club view). It has no row in the truth table, and
M33 still does not read it, so it is not a capture in `photos.json`.

**Still absent**: the five MOV clips, HD Golf's fifteen photos and `data/models/ocr/`. The clips and models
were not part of the user's answer, so their absence is this section's reading of "proceed", not the
user's stated choice. The orchestrator tells the user so.

**What each phase does.** A phase that cannot run proves its input is absent, records that as its finding,
and ticks its row `✅ Not run: <reason> (decision of 2026-11-04)`.

| phase | under this decision |
|---|---|
| P1 | `build-manifest` writes the six `fs-still` entries only, with the paths and digests above. `probe.py check` holds them to those digests, and a mismatch is still a stop. It prints the clips, HD Golf and the models as absent, which is **not** a stop. Done when six match and the throwaway comparison against `PHOTOS` finds no difference in **values**; `PHOTOS`' digests are the originals' and are expected to differ |
| P2 | `captures/IMG_2889.jpg` in place of `IMG_2889.HEIC` |
| P3, P6, P7 | As written, on the six stills. P7's controls use committed vector boxes and need no photo |
| P4, P10 | Not run: no clips. The clips' verdict is **not measured** |
| P5 | The six stills only, each through `working` directly, since they are JPEGs. The HD Golf clause of "Done when" falls away |
| P8 | Not run: no models. This overrides its "stop (`BLOCKED`)". There is no lab baseline, so finding 10's 71 of 72 may be quoted only as measured on the originals |
| P9 | As written on the six copies, with no lab column |
| P11 | Not run: omitted by the user |
| P12 | The target for this run is stills alone: stills pass on `raw` or `vision`. The ladder runs on `fs-still` only |
| P13–P15 | Steps 1 and 2 run, and "not needed" ticks as written. If a cell is parser-fixable, list the cells as the finding and tick `✅ Not run`: check 6 of a rule's acceptance needs P8's baseline, so no rule can be accepted in this run, and none is fitted to these copies. `crates/`, `spec/` and `SCREEN_PARSER_VERSION` stay untouched |
| P16 | `fs-still` only, with no lab comparison. The recommended value is provisional |
| P17 | The verdict by call 9's table, labelled **provisional: pasted copies**, with the clips **not measured**. `probe.py verdict` prints the same label. "What this does not show" adds the original files, the clips, HD Golf and the lab baseline. Whichever way it reads, it says this is not yet the answer on the photos as captured |
| P18 | Every doc carries the label and the list of what is unmeasured |
| P19 | M33's ROADMAP row is **open, not ✅**, naming the provisional verdict and what is outstanding. M38's row names the verdict and leaves unblocking to the user. Memory says the Mac holds pasted copies and no originals or models. The commit title carries "provisional". Step 7's report lists what is outstanding |

**This run does not meet "Exit".** M33 closes when P9 is re-measured on the original files and the clips,
HD Golf and the lab baseline have run. Those get new rows here when the files reach this Mac. The captures
commit is a third commit, beside the two "Exit" names.

---

## Rules every phase keeps

1. **No phase asks the user anything.** Every decision a phase needs is in this file. Where a phase
   hits something this file does not cover, it takes the option that changes the least committed
   behaviour, records the choice as a finding addressed to P17 (so the verdict and docs say it) and
   continues. It stops, reporting `BLOCKED` or `FAILED`, only where a rule below says *stop*.
2. **The bars do not move.** `spikes/vision-ocr/thresholds.md` is written and committed at P0, before
   Vision has read one real photo. No later phase edits it, however a bar looks once the numbers are
   in. A bar that looks wrong is a finding for P17, and the measured verdict stands beside it.
3. **Preconditions are checked first, and a failed one stops the phase** with a finding naming it.
   The photos and clips with the digests below (P1 on). The ONNX models in `data/models/ocr/` (P8 and
   P13–P15). `cargo` on `PATH` (if it is not, `. "$HOME/.cargo/env"` first). ffmpeg at the path in
   rule 6 (P4, P8, P13–P15).
4. **`data/` is read-only.** Nothing writes under it. Everything a phase generates goes under
   `spikes/vision-ocr/frames/`, which `spikes/*/frames/` in `.gitignore` already covers.
5. **`crates/`, `src/`, `scripts/`, `tests/` and `spec/` do not change**, so
   `git diff --stat -- crates src scripts tests spec` is empty at every phase up to P12. P13–P15 may
   touch only what call 11 names. `src/`, `scripts/` and `tests/` never change in this milestone. A
   frozen Python test that a new vector breaks is a **stop**.
6. **ffmpeg is `~/.local/share/golf-coach/ffmpeg-7.1`, set by `GOLF_FFMPEG` on the commands that
   decode a clip, and never on `PATH`.** It is the `imageio-ffmpeg` 0.6.0 wheel's binary, and
   `crates/ocr/src/decode.rs` records why it must not be found by accident: it decodes one 896×640
   tile of a HEIC rather than the photo. So **no HEIC goes through it**. HEICs decode through ImageIO
   (call 2). Homebrew cannot install a newer one because `/opt/homebrew` belongs to another account on
   this Mac, and the fallback was refused in planning, so do not try to obtain another ffmpeg.
7. **The macOS test baseline is exact.** `cargo test --workspace --no-fail-fast` passes 1,761 with 26
   ignored and fails exactly the five named under "What the planning read found" (finding 1). They are
   not M33's to fix. A sixth failure, or one of the five coming back different, is a **stop**, unless
   the phase's own rule change caused it, in which case the rule is withdrawn (call 11).
8. **Every run uses the release builds in `target/m33/`**, and the spike's outputs say which macOS
   build and Vision revision produced them.
9. **Hold every capture to its digest before reading it**, so the truth table only meets what it was
   read from. `probe.py check` is that step, and a mismatch is a **stop**.
10. **Comparisons are exact.** A value is read only if it equals the truth as an IEEE double, compared
    with `==` after the JSON round trip. There is no tolerance on a value a screen printed.
11. **Spike code is spike code**, in `spikes/vision-ocr/`, and not held to `docs/CODE_STANDARDS.md`.
    Its comments still say why. Rust written under call 11 is shipping code and is held to it: why in
    the comments, the measurement beside each constant, and "no Python twin" where that is true.
12. **Do not write counts into prose** that a test or registry owns. Where this plan quotes a count it
    is a snapshot. This file avoids the `**Status: … N/M phases` spelling on purpose
    (`tests/test_docs_truth.py::_PHASE_STATUS`).
13. **Commits are exactly two, both onto `main`, and neither is pushed**: P0's pre-registration and
    P19's close (call 13). Every other phase leaves its work in the tree.
14. **Scratch work goes in the session's scratchpad**, never in the repo, and a throwaway script is
    deleted before the phase ends. `spikes/vision-ocr/` is not scratch: what it holds is the record.

## Verify — the commands, and when each runs

```bash
# Builds (P2 on, as needed):
swiftc -swift-version 5 -O spikes/vision-ocr/vision_ocr.swift -o target/m33/vision-ocr
cargo build --release --target-dir target/m33 --bin golf-core --bin golf-mcp --bin golf-ocr
cargo build --release --target-dir target/m33 --manifest-path spikes/vision-ocr/lab-images/Cargo.toml

# The spike (each from P7 on; the first from P1):
.venv/bin/python spikes/vision-ocr/probe.py check            # digests and tools; a mismatch is a stop
.venv/bin/python spikes/vision-ocr/probe.py control          # the comparator sees what it must
.venv/bin/python spikes/vision-ocr/probe.py measure --config C0 --set fs-still
.venv/bin/python spikes/vision-ocr/probe.py lab-regress      # P13-P15: the lab's parses did not move

# The whole workspace, P13-P15 and P19: exactly the five known failures (rule 7). Pipe it through
# grep or tail; it is 2,900 lines.
cargo test --workspace --no-fail-fast 2>&1 | grep -E '^test result: FAILED|^test .* FAILED$|^error: '
cargo fmt --check                                            # P13-P15, P19
cargo clippy --all-targets 2>&1 | grep -c '^warning'         # P13-P15, P19: not above the baseline
                                                             # (finding 2)
# The screen re-record dry run must report 0 changed and 0 files written. golf-core finds golf-mcp
# beside itself, so build both first.
target/m33/release/golf-core rerecord --declare spec/declarations/screen-v1.json --dry-run  # P13-P15, P19
# The other two, at P19 only if a rule fired:
target/m33/release/golf-core rerecord --declare spec/declarations/v17.json --dry-run
target/m33/release/golf-core rerecord --declare spec/declarations/career-v3.json --dry-run
# The clip gate through the lab's reader (needs the models; decodes MOVs, so 7.1 serves): P8, P13-P15
GOLF_FFMPEG="$HOME/.local/share/golf-coach/ffmpeg-7.1" \
  cargo test --release --target-dir target/m33 -p golf-lab --test full_swing_clips -- --include-ignored

# Python: only these two on this Mac (finding 3). Docs phases: P0, P18, P19.
.venv/bin/python -m pytest tests/test_docs_truth.py -p no:cacheprovider
.venv/bin/python -m pytest tests/test_conformance.py -p no:cacheprovider   # P13-P15, P19 if a rule fired
.venv/bin/python scripts/conformance.py check                              # P13-P15, P19 if a rule fired

git diff --stat -- crates src scripts tests spec             # every phase: empty up to P12
```

**Not runnable on this Mac, on purpose**: the 13-photo OCR gate (`crates/ocr/tests/read.rs`) and the
Full Swing photo gate (`crates/ocr/tests/full_swing.rs`). Both decode HEICs through ffmpeg, and rule 6
says no. If a parser rule fires, P19's close report tells the user to run both on the Windows box
after pulling (call 15).

---

## Decisions taken in the interview (2026-11-04)

1. **The parser is Rust's**, `golf-core parse-screen`, the code the phone will call, and not frozen
   Python's as §M33 wrote it. That section was written before M34 existed. Rust also has the
   `Impact Position V` fix and reads Full Swing.
2. **Full Swing decides the verdict.** The user's words: "Full swing matters most."
3. **HD Golf is diagnostic only**: measured and logged, with no bar and no verdict of its own. (The
   user chose this over a verdict that could not block.)
4. **Full Swing's coverage bar**: at least 66 of the 72 values on the stills, and each of the twelve
   fields read on at least 5 of the 6 photos. The per-field floor exists because 66 alone would pass a
   recognizer that always loses one field, ball direction say. On the clip frames the same rate is at
   least 55 of 60, each field on at least 4 of 5.
5. **The clip frames get a verdict of their own, which cannot block M38.** If stills pass and clips
   miss, M38 ships the still import first.
6. **The toolchain was installed during planning**: Rust through rustup, ffmpeg as a user-local
   binary. So the baselines below are measured, not assumed.
7. **The review ceiling**: at most 1 of 6 stills and at most 1 of 5 clip frames flagged
   `needs_review`. Without it a recognizer passes "no unflagged wrong value" by flagging everything,
   and the golfer checks every shot at the bay. The lab's reader flags 0 of 6.
8. **Either phone path can earn GO.** GO if Vision-rectified or raw passes on the stills. CONDITIONAL
   if only the lab's rectifier passes: Vision can read the screen, but M38 must also ship a rectifier.
   NO-GO if none passes, and the app waits on the user's decision.
9. **Tuning may reach the parser.** A fixed Swift ladder first (P12), then opt-in Full Swing rules with
   hand-worked vectors, as M42 did (P13–P15). The user chose this over Swift-only, accepting that it
   tunes the parser on the photos it is measured on, so a pass that needed it says so.
10. **Six stills are enough for M33.** M38 P5's bay session (at least 30 photos) is the real test, and
    the verdict states the sample's size. Five of the six were taken on 2026-10-10 and one on
    2026-10-07.

## What the planning read found (2026-11-04)

All measured on this Mac: arm64, macOS 26.2 (25C56), Xcode 26.2, Swift 6.2.3, Rust 1.99.1 from rustup.

1. **The first macOS `cargo test`**: **1,761 passed, 5 failed, 26 ignored**, with the five in three
   targets, none on M33's path. It takes `--no-fail-fast`, since plain `cargo test` stops at the first
   failing target and `capture` is early.
   - `capture --lib`: `tests::a_host_with_no_cameras_is_an_empty_list_and_not_an_error`,
     `tests::an_unknown_id_is_gone_and_says_which`, `tests::listing_twice_gives_the_same_order`. Each
     unwraps `Unsupported("macos")`, as the crate has only a Media Foundation backend.
   - `golf-lab --test flight`: `an_empty_store_says_how_shots_arrive` (`crates/lab/tests/flight.rs:273`).
   - `golf-lab --test import_photos`: `a_path_is_spelled_as_pythons_str_path`
     (`crates/lab/tests/import_photos.rs:470`), where a `./` in a path fails the read.

   **No float gate failed.** Every engine, stage, flight and format vector, including the bit-for-bit
   census, passes on Apple's math library, which was the prediction before running it. **For M38 P0**:
   these five are the rest of "get `cargo test` green on macOS".
2. **clippy**: `cargo clippy --all-targets 2>&1 | grep -c '^warning'` prints **28** on Rust 1.99.1,
   where the Windows box's toolchain reported 0 (the M42 close). They are newer lints, not new code.
   `cargo fmt --check` is clean. A rule phase may not raise the count.
3. **Python on this Mac.** `.venv` is Python 3.14.3 with the base and dev dependencies only, and no
   `api` or `llm` extras. `pytest --continue-on-collection-errors`: **1,719 passed, 62 skipped, 4 failed,
   13 collection errors**, in 68 s, every error a missing extra. `ruff` and `mypy` are clean, and
   `conformance.py check` exits 0. M33 touches no Python, so it runs only `tests/test_docs_truth.py` (91
   passed), `tests/test_conformance.py` and `conformance.py check`. **`tests/test_conformance.py`'s
   baseline is those four failures, all of this environment**: three need `anyio`
   (`test_the_mcp_cases_build_in_memory_and_write_nothing`,
   `test_the_mcp_cases_reach_every_tool_on_both_servers_and_every_miss`,
   `test_the_committed_mcp_cases_are_what_frozen_python_answers`), and
   `test_the_format_table_regenerates_byte_identically` differs only in `python_version` (3.14.3 against
   the recorded 3.13.3). CLAUDE.md's `.venv/Scripts/python.exe` is a Windows path, and here it is
   `.venv/bin/python`. **For M38 P0**: the extras, if it wants the frozen suite here.
4. **The re-record dry runs pass on macOS**, each 0 changed and 0 files written: `v17` 140 vectors,
   `screen-v1` 45, `career-v3` 175. `golf-core`, `golf-mcp` and `golf-ocr` are already built in
   `target/m33/release/`.
5. **Homebrew is not available to this user.** `/opt/homebrew` is owned by another account, so
   `brew install` fails on permissions, and changing its ownership would take it from that account.
   **For M38 P0**: Flutter and CocoaPods need a route that does not use it.
6. **ffmpeg 7.1 is on disk off `PATH`**, per rule 6. `golf_lab::ffmpeg::Ffmpeg` needs no `ffprobe`.
7. **Vision runs headless here.** A smoke test in the planning session's scratchpad rendered the
   committed box geometry of `screen/corpus/2026-08-10-2` and `screen/hand/full-swing-grid` to a
   synthetic image, and fed a skewed synthetic panel through the rectifier. **No real photo was read,
   which is why the bars could still be set.** Synthetic results are not evidence about photos:
   - `VNRecognizeTextRequest` at revision 3 took about 250 ms at 1659×1034.
   - **With `usesLanguageCorrection=false`, every confidence came back 1.00.** With it on, they were
     0.3, 0.5 or 1.0. So with correction off the parser's 0.30 OCR term may be a constant: Vision adds
     no doubt signal, and `needs_review` would rest on the coverage terms and the physics checks alone.
     P16 measures this on photos. On `parse_confidence` it means a shot gets a flat +0.30.
   - Neighbouring tiles did not merge: 42 boxes in, 42 out on the HD Golf layout, 41 of 41 on Full
     Swing's. Full Swing's single-letter boxes `R`, `D` and `F` came back alone, and each is what
     `lit_sign` needs: a value box whose text is one sign letter exactly.
   - `VNDetectRectanglesRequest` and `CIPerspectiveCorrection` work headless, and
     `VNRecognizedText.boundingBox(for:)` returns word boxes.
8. **§M33's recognizer settings pair `usesLanguageCorrection=false` with `customWords`.** Apple's web
   reference (from memory, not checked in this session) says custom words are ignored without
   correction, and the SDK header does not say either way. So C0 sets no custom words, and C3 turns
   both on together.
9. **None of the 21 stills needs the rotation vote.** No HD Golf vector's `input.notes` says
   `image was rotated`, and M42 P9 found the vote stopped at its first turn on all six Full Swing
   photos. So the spike reads each image upright as ImageIO decodes it, with no vote. If a photo turns
   out sideways, it shows up as a miss.
10. **What the lab's reader measured, for comparison and not as a bar**: 71 of 72 on the stills, with
    `IMG_2892`'s `total_distance` withheld (a stray box over the header copy), 60 of 60 on the clip
    frames, and 0 of 6 flagged (the M42 plan's P9 and P14). Its lit letters measure blue ÷ min(red,
    green) 0.32–0.48 and its unlit ones 0.89–0.98, with `LIT_BLUE_RATIO` 0.7 between them
    (`crates/screen/src/parser.rs`). `IMG_2891` returned neither `L` nor `D`, and its signs were decided
    from unlit letters alone.
11. **The profile's Full Swing fields**: `crates/screen/profiles.json`, `device: "full_swing"`,
    `split_merged_labels: true`, fourteen fields, of which twelve are stored and two (`Shot Result`,
    `To Hole Yd`) are boundary tiles. `lit_sign` is on `Ball Direction Deg` and `Sidespin Rpm`,
    `printed_twice` on `Total Yd`, `printed_unit: feet` on `Apex Ft`.

---

## The photos

Paths are relative to the repo root. The user copies them from the Windows box before P1, preserving
paths and modification times, along with `data/models/ocr/`.

**Full Swing stills and clips** (`data/raw/shot_screens/`), digests SHA-256, from the M42 plan:

| id | still | still sha256 | clip | clip sha256 |
|---|---|---|---|---|
| `IMG_2889` | `IMG_2889.HEIC` | `d47e3e9607211a1350c09416d96f2f7e64a3d237a29d6f5a9ba57890f5257d8e` | `IMG_2889.MOV` | `ed2112dbf7f85320d04a1b3d2681853ba5329c989669b8dc1e6095a705c6a2b7` |
| `IMG_2891` | `IMG_2891.HEIC` | `73be98e34cee2eafa22ad8d693c9c1c4c4efac585616f452d6cd5ceda58a15c0` | `IMG_2891.MOV` | `0c86193a1af6cf592ced2aa23cca34fe67f19cd1b8eada2601c7019758790815` |
| `IMG_2892` | `IMG_2892.HEIC` | `1c4b731de2871303b6387a843e65d893a0c37d059b7ff0cca321eea2f1fc6d04` | `IMG_2892.MOV` | `0fa5aa0d2d799d87800693a52d5454ed66ccdf08c293967edc59be9425b13513` |
| `IMG_2893` | `IMG_2893.HEIC` | `204872ef4b6ccd2c7c2730ca14283b43e4a3c5f4e57164373d52c19fb72d9ca8` | `IMG_2893.MOV` | `bf97341db7521dd3b8b50836cc4e90120c192d437b03f18dc6b5903c17b8520a` |
| `IMG_2894` | `IMG_2894.HEIC` | `227d9985b337a70248f0e268eea4e73fc3e5860bc0b2418afe6082a4b19c399b` | `IMG_2894.MOV` | `6ed8385c1891a3ba1c5c1db0fd129a77eaf283be88db3395b03fca8fc3943d31` |
| `2026-10-07-1` | `2026-10-07-1.jpg` | `96241978f183d70cfbd633802b395362a68ea0b3905324fbbc9a40e18c51b0e1` | none | — |

`IMG_2888` is the club-view layout no profile reads, and M33 does not read it.

**HD Golf, the diagnostic set**: each of `spec/vectors/screen/corpus/*.json` (13) and
`spec/vectors/screen/reference/*.json` (2) names its photo in `input.image_path` and its digest in
`input.image_sha256`. `probe.py` reads both from the vector, so there is one copy of them. One of the
13 is a HEIC, two are PNGs, the rest JPEGs, and the two references are `.jpeg`.

## The truth table — what each Full Swing photo prints

**The M42 plan's table, read by eye there with each lit letter confirmed on a crop**, and the oracle
for both Full Swing arms. The same table is `PHOTOS` in `crates/ocr/tests/full_swing.rs`, and P1 checks
its data file against that. Clip frames answer to the same row as the still of the same id.

| id | carry | total | club | ball | path | face | ball dir | launch | apex ft | smash | backspin | sidespin |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `IMG_2889` | 131 | 134 | 95 | 102.5 | -2.0 | 13.2 | 10.1 R | 24.5 | 85.2 | 1.08 | 9899 | 4199 D |
| `IMG_2891` | 162 | 203 | 86.9 | 112.8 | -1.7 | -0.6 | 0.8 L | 12.1 | 39.9 | 1.30 | 4236 | 464 D |
| `IMG_2892` | 80 | 82 | 91.6 | 77.4 | -4.5 | 6.9 | 4.6 R | 34.6 | 71.4 | 0.85 | 8855 | 2032 D |
| `IMG_2893` | 184 | 224 | 107.6 | 134 | -1.7 | 3.2 | 2.2 R | 5.5 | 15.5 | 1.25 | 2284 | 638 D |
| `IMG_2894` | 105 | 107 | 81.3 | 87.2 | -1.7 | 7.5 | 5.7 R | 31.4 | 80.6 | 1.07 | 9985 | 2209 D |
| `2026-10-07-1` | 279 | 289 | 111.3 | 158.6 | 1.6 | 4.0 | 3.5 R | 19 | 142.2 | 1.43 | 2521 | 336 F |

As `ShotData`, converted in one place, `probe.py`'s `truth()`: `carry_distance`, `total_distance`,
`club_head_speed`, `ball_speed`, `club_path`, `club_face_angle`, `launch_direction` (`+` for R, `-` for
L), `launch_angle`, `apex_height` (feet ÷ 3.0, computed as `85.2 / 3.0`, never typed as a decimal),
`smash_factor`, `backspin`, `sidespin` (`-` for D, `+` for F). Twelve fields a photo, seventy-two in
all, and sixty on the clips, which exclude `2026-10-07-1`.

---

## Calls this plan makes

These were not put to the user. Each is the plan's, and the approval covers them.

**Call 1 — The layout.** Everything is in `spikes/vision-ocr/`:

```
thresholds.md                   # P0. The bars, the verdict table and the ladder. Frozen after P0
log.md                          # P17. The findings, in the house form of mediapipe-hands/log.md
photos.json                     # P1. Every capture: id, set, path, sha256, device, truth or vector
truth_full_swing.json           # P1. The table above in the table's own spelling
vision_ocr.swift                # P2, P3, P6, P12. One file, built with swiftc
lab-images/                     # P3, P4, P5. Rust helper, not a workspace member (call 6)
probe.py                        # P1 on. Stdlib-only Python: runs arms, classifies, prints tables
results/<config>.json           # Each measurement's classification, committed
boxes/<config>/<mode>/<id>.json # C0's and the final config's boxes, committed, so M38 can make vectors
lab-boxes/<id>.json             # P8. The lab's reader on the same inputs, committed
lab-boxes/parses.sha256         # P8. Each lab parse's digest, which `lab-regress` holds P13-P15 to
frames/                         # Gitignored: decoded/, vision/, lab/, grid/, crops/, and each run's
                                #   boxes and shots under runs/<config>/<mode>/<id>.{boxes,shot}.json
```

The written §M33 put the Swift CLI in `tools/vision-ocr/`. It is spike code, so it stays with the spike.
M38's plugin takes what it learned, not necessarily its code. `probe.py` is Python because earlier
spikes' are, and because it is glue over two binaries and a JSON diff, which ADR-035's §1 does not
reach: nothing ships from `spikes/`.

**Call 2 — One decode path, ImageIO, as on the phone.** `vision-ocr decode IN OUT` opens a HEIC, JPEG or
PNG with `CIImage(contentsOf:options:[.applyOrientationProperty: true])`, renders it to 8-bit RGBA in
sRGB, and writes a PNG tagged sRGB. Every Swift step then reads PNGs. Consequences:
- iPhone HEICs are Display P3, so this **converts colour**, where M42's numbers came from ffmpeg's
  `rgb24`, which did not. P9's lit table shows what that does to `LIT_BLUE_RATIO`'s margin. The phone's
  plugin would convert the same way, so this is the measurement M38 needs, and M42's plan asked for.
- The lab's images (P5, P8) cannot take the lab's own HEIC decode here (rule 6), so a HEIC's lab image
  starts from this PNG, while JPEGs and PNGs go through `ocr::decode::load_image`, the lab's own decode.
  Five of the six stills are HEICs, so `lab` mode is **not** the lab's exact pixels on them, and the
  log says so. That is acceptable: it is a diagnostic, not a gate.

**Call 3 — The three modes**, each an input image and its `notes`:

| mode | image | `notes` | role |
|---|---|---|---|
| `raw` | the decoded PNG, or a grid frame | `[]` | **phone path**: a photo as imported |
| `vision` | `frames/vision/<id>.png`, rectified by Vision; where no rectangle is found, the raw image | `[]`, or the uncropped note | **phone path**: the stand-in for VisionKit's scanner |
| `lab` | `frames/lab/<id>.png`, `ocr::rectify::working_image` | `[]`, or the uncropped note | diagnostic: tells the recognizer's faults from the rectifier's |

The uncropped note is `ocr::prepare::UNCROPPED_NOTE`'s text, verbatim:
`screen outline not found - parsing the photo uncropped`.

**Call 4 — C0, the first settings.** `VNRecognizeTextRequest` with `recognitionLevel = .accurate`,
`revision = VNRecognizeTextRequestRevision3`, `usesLanguageCorrection = false`,
`recognitionLanguages = ["en-US"]`, `automaticallyDetectsLanguage = false`, `customWords = []`. One box
per observation, from `topCandidates(1)[0]`: `text`, `confidence`, and the bounding box flipped to
top-left pixels (`x = minX·W`, `y = (1 − maxY)·H`, `width·W`, `height·H`, unrounded). `vision-ocr read`
prints `{"image", "width", "height", "config", "revision", "os", "boxes": [...]}`, with each box in
`screen::TextBox`'s keys.

**Call 5 — `rgb` by `TextBox::rgb`'s definition, exactly**, in the pixels of the image the box's
coordinates are in (`crates/screen/src/lib.rs`, `crates/ocr/src/recognizer.rs::glyph_rgb`): clip to the
image (`x0 = floor(max(x, 0))`, `x1 = min(ceil(x + width), W)`, likewise y), rank the pixels by
`r + g + b` ascending with ties in row-major order, and take the mean of the ranks from `n * 3 / 4`
(integer division) to the end. A box with no pixel is `null`. Swift's `sort` is not documented as stable,
so the rank key is `(sum, rowMajorIndex)`. Read the pixels from the PNG's own bytes without colour
matching, so an untagged PNG (Rust writes those) is not converted.

**Call 6 — `lab-images`**, three subcommands over the crates that already do the work:

```toml
[package]
name = "vision-spike-lab-images"
version = "0.0.0"
edition = "2021"
publish = false

[workspace] # Empty on purpose: not a member of the root workspace, which the crate count pins.

[dependencies]
ocr = { path = "../../../crates/ocr" }
golf-lab = { path = "../../../crates/lab" }
image = { version = "=0.25.9", default-features = false, features = ["png"] }
serde_json = { version = "1.0", features = ["float_roundtrip"] }
```

Copy the root `Cargo.lock` to `lab-images/Cargo.lock` before the first build and commit it, so it
resolves what the workspace resolves.
- `glyph IMAGE BOXES.json`: `ocr::decode::load_image`, then `ocr::recognizer::glyph_rgb` for each box;
  prints the `rgb` list.
- `grid-frame CLIP OUT.png`: `golf_lab::ffmpeg::Ffmpeg::find()`, then
  `golf_lab::shot::write_grid_frame(clip, png, GRID_FRAME, &mut ffmpeg)`; prints frame, frames and digest.
- `working IMAGE OUT.png`: `ocr::decode::load_image`, then `ocr::rectify::working_image`; writes the
  image and prints whether a quad was found.

**Call 7 — The phone's rectifier stand-in.** `VNDetectRectanglesRequest` with `minimumAspectRatio = 0.4`,
`maximumAspectRatio = 0.9`, `minimumSize = 0.3`, `quadratureTolerance = 30`, `minimumConfidence = 0.5`,
`maximumObservations = 0`, keeping the largest by area, since a screen holds panels of its own. Then
`CIPerspectiveCorrection` on its corners, written as an sRGB PNG with a sidecar JSON
`{"found", "confidence", "corners"}`. These values are not tuned at all: rectangles found on few photos
is a finding, and `vision` then falls back to `raw` on those. It is a stand-in. VisionKit's real scanner
needs a device, and M38 measures it.

**Call 8 — The comparator.** For each of the twelve fields of a shot:

| cell | means |
|---|---|
| **read** | the value equals the truth (rule 10) |
| **withheld** | the value is `None` |
| **wrong** | a value that differs, on a shot **not** flagged `needs_review` |
| **caught** | a value that differs, on a flagged shot |

A `null` shot (a failed read) is twelve **withheld** and counts as **flagged**: to a golfer it is a
re-take. A read needs `golf-core parse-screen`, with an input of `device`, `boxes`, `notes`, `shot_id`,
`session_id`, `timestamp`, `image_sha256`, `image_path` and `min_confidence: 0.6`. `ScreenInput` refuses
an unknown key and a naive timestamp, so Full Swing's is the fixed `2026-11-04T00:00:00Z`, and HD Golf's
is the vector's.

**Call 9 — The bars, as `thresholds.md` writes them.** An arm is a config, a mode and a set.

| bar | stills (6 photos, 72 values) | clip frames (5, 60 values) |
|---|---|---|
| **T1** wrong | 0 | 0 |
| **T2** read | at least 66 | at least 55 |
| **T3** each field read on | at least 5 of 6 | at least 4 of 5 |
| **T4** flagged | at most 1 | at most 1 |

**caught** cells do not fail T1, because review exists for them, but they are not read, so they count
against T2 and T3. All measured at `min_confidence` 0.6, whatever P16 goes on to recommend.

| | stills pass on `raw` or `vision` | stills pass on `lab` only | stills pass on none |
|---|---|---|---|
| **Verdict** | **GO** | **CONDITIONAL**: Vision reads the screen; M38 must also ship a rectifier | **NO-GO**: the app waits on the user |

The clips' verdict, beside it and never blocking: **PASS** on `raw` or `vision`, **LAB-ONLY** on `lab`,
**MISS** otherwise, which means M38 ships the still import first. HD Golf gets no verdict. A verdict
reached with any config but C0 is labelled **tuned**, and one reached with a parser rule **tuned, parser**,
because the ladder and the rules were fitted on the photos they were measured on.

**Call 10 — The ladder (P12).** Each config adds to C0, and combines by `+`:

| config | what it changes |
|---|---|
| **C1** `words` | each observation split on whitespace into word boxes through `VNRecognizedText.boundingBox(for:)`; a word whose box comes back nil leaves the observation whole |
| **C2** `x2` | the image upscaled ×2 by `CILanczosScaleTransform` first; coordinates and `rgb` in the upscaled image |
| **C3** `lexicon` | `usesLanguageCorrection = true` and `customWords` = `CARRY TOTAL CLUB SPEED BALL PATH FACE DIRECTION LAUNCH ANGLE APEX SMASH FACTOR BACKSPIN SIDESPIN MPH DEG YD FT RPM L R D F` |
| **C4** `second-pass` | after the first pass, four overlapping corner crops (each 60% of each side) upscaled ×2 and read; boxes mapped back and added where their centre lies in no existing box; `rgb` in the full image |

The target is: stills pass on `raw` or `vision`, **and** clips pass on `raw` or `vision`. Round 1 runs
C1, C2, C3 and C4 alone, and stops at the first in that order that meets the target. Round 2, if none
did, takes the best single config and adds each of the other three in that order, stopping the same way.
So at most eight configs are ever measured, C0 included, and that bound holds however it goes.

"Best" compares each config on its better phone mode, in this order, first difference deciding: stills
pass; fewest **wrong** on stills; most **read** on stills; fewest flagged on stills; most **read** on
clips; earliest in the order measured. **The final config** is the first that meets the target; else
the first on which stills pass; else the best.

**Call 11 — The parser rules (P13–P15).** Condition: the final config misses on stills on both phone
modes, or misses on clips on both phone modes, with the stills' misses taken first. What may change:
`crates/screen/src/parser.rs`, `crates/screen/src/profile.rs` (a `#[serde(default)]` opt-in), the
`full_swing` entry of `crates/screen/profiles.json` (including its aliases and blank markers),
`crates/screen/tests/hand.rs`'s `CASES`, and new files `spec/vectors/screen/hand/vision-<slug>.json`.
A rule is accepted only if **all** of these hold:

1. **Only parser-fixable misses are in scope**: the printed value is present in Vision's boxes, and the
   parser reads it wrongly or not at all. A value Vision never returned is the recognizer's, no parser
   rule can honestly recover it, and it is out of scope.
2. **It is opt-in and states a general rule**, and names no photo, no value and no position unique to a
   photo. `hd_golf`'s entry in `profiles.json` stays byte-identical, and `tests/profile_fork.rs` passes.
3. **It does not weaken ADR-010 §2.** A signed value is still stored only when the lit letters decide it
   (`lit_sign`'s table). `LIT_BLUE_RATIO`, `LIT_MIN_BRIGHTNESS`, the `W_*` weights and `min_confidence` do
   not move, and neither does `SCREEN_PARSER_VERSION`, because M42 established that opt-in rules which
   move no committed vector need no bump.
4. **It has its own hand-worked vector**, from Vision's unedited boxes, with `provenance.oracle: "hand"`
   and a `note` working every value, the confidence arithmetic, `label_ratio` and every warning, as M42's
   P10 vectors do, and `CASES` gains it.
5. **No committed vector moves**: `git diff --stat -- spec/vectors` shows additions only, and the
   screen dry run reports 0 changed.
6. **The lab does not move**: `probe.py lab-regress` shows every parse of `lab-boxes/` byte-identical to
   P8's, and the clip gate gives P8's result.
7. **Nothing else breaks**: exactly rule 7's five failures, `cargo fmt --check` clean, clippy not above
   finding 2's count, and `tests/test_conformance.py` and `conformance.py check` as at the baseline.
8. **It helps**: re-measured at the final config, the arm it targets has more **read** cells and no more
   **wrong** ones.

**A rule failing any of these is withdrawn**, by undoing that phase's edits by hand (never `git reset`),
and recorded as a finding with the reason. That is the phase's result, not a stop. **The cap is three
rules.** The ladder is not re-run after a rule: the final config stays final, which keeps the number of
things fitted to six photos small.

**Call 12 — The lit table.** `probe.py` prints, for every still, mode and config, each letter box
returned (`L`, `R`, `D`, `F`) with its `rgb`, `b ÷ min(r, g)` and what `is_lit` says, beside the sign
decided, with `LIT_MIN_BRIGHTNESS` and `LIT_BLUE_RATIO` copied from `crates/screen/src/parser.rs`, naming
the source. That is the answer to M42's ask that the plugin's colour be held to the lab's before the
threshold is trusted.

**Call 13 — The two commits.** P0: this plan, `thresholds.md`, `docs/README.md`'s count, and the
pointers, so that the bars are provably in history before any measurement, as `spikes/README.md`
describes earlier spikes doing. P19: everything else. Both onto `main`, as the milestones before this
have, and neither pushed. Each ends with the attribution lines the committing session's system reminder
gives.

**Call 14 — HD Golf's report.** `probe.py` applies call 8 against each vector's `expected.shot`, over its
non-`provenance` keys, plus an **extra** cell where the vector holds null and Vision a value. It carries
`crates/ocr/tests/read.rs`'s one declared difference: on `2026-08-10-1`, `impact_position` `CENTER` is
what the photo shows, and counts as read. No bar applies and nothing is adjudicated: differences are
listed for P17, which reports them as an open item for M38.

**Call 15 — What cannot be checked here.** If a parser rule fires, the lab's HEIC gates need the Windows
box (rule 6). P19's report gives the command, `cargo test --release -p ocr -- --include-ignored`, and
the WORKLOG lists it as left open. `lab-regress` and the clip gate are what stand in for it until then.

**Call 16 — Reading a wrong value by eye.** The truth table wins. On any **wrong** or **caught** cell,
the phase writes a crop to `frames/crops/<config>-<mode>-<id>-<field>.png` and looks at it with the Read
tool. If it plainly disagrees with the table, the cell still counts as measured, and the phase records a
finding to P17 saying the table may be wrong, so that the verdict states it. Neither the table nor the
bars are edited (rule 2).

---

## Phases

### P0 — This plan, `thresholds.md`, the pointers, and the pre-registration commit

- **Goal**: the bars in git history before Vision reads a real photo.
- **Read first**: this file; ROADMAP's M33 row and §M33's `**Status**` line; `docs/README.md`'s first
  paragraph; `spikes/mediapipe-hands/thresholds.md` for form.
- **Do**:
  1. This file at `docs/plans/m33-vision-spike.md`.
  2. `spikes/vision-ocr/thresholds.md`: the question, the definitions (calls 3 and 8), the bars and
     verdict table (call 9), the ladder and its order (call 10), the parser rules' acceptance (call 11),
     and a first line saying it was written before any real photo was read, and that planning's smoke
     test used synthetic renders only. Say which numbers came from the interview.
  3. ROADMAP's M33 row: `🔄 Planned 2026-11-04, 20 phases — docs/plans/m33-vision-spike.md. Full Swing
     decides, through the Rust parser`. §M33's `**Status**` line to the same effect. The program plan's
     M33 checklist row to point here too. Nothing else.
  4. `docs/README.md`: the count, now 87: 73 in `docs/` with 12 in `plans/`, and 14 outside it with seven
     in `spikes/`.
  5. `git add` those files and commit onto `main`: `M33 P0: the Vision spike's plan and its bars,
     before any measurement`, with the attribution lines. Not pushed. Tick this row in the commit.
- **Done when**: docs-truth is green with the files staged, the commit exists, and `git status` is clean.
- **Verify**: `.venv/bin/python -m pytest tests/test_docs_truth.py -p no:cacheprovider`; `git log -1`.

### P1 — The photo manifest and the truth table as data; every digest checked

- **Goal**: every capture named, held to its digest, and its truth checked against the Rust table.
- **Read first**: "The photos"; "The truth table"; `crates/ocr/tests/full_swing.rs:30–200` (`Photo`,
  `PHOTOS`); `data/README.md` is not needed.
- **Files**: `spikes/vision-ocr/probe.py` (`check`, `truth()`, `build-manifest`), `photos.json`,
  `truth_full_swing.json`.
- **Do**: `probe.py build-manifest` writes `photos.json`: six `fs-still` entries, five `fs-clip` entries
  and fifteen `hd` entries, the last from the vectors' `input`. `truth_full_swing.json` holds the table in
  its own spelling (a magnitude and its letter, an apex in feet), and `truth()` converts it in one place.
  `probe.py check` hashes every capture, checks `swiftc`, `cargo`, the ffmpeg of rule 6 and
  `data/models/ocr/`, and prints what is missing.
- **Done when**: `probe.py check` reports 26 captures matching and the models present; a throwaway
  comparison against `PHOTOS`'s digests and values finds no difference.
- **If**: any capture or the models are missing, or a digest differs, **stop** (`BLOCKED`), naming which.
  The user copies them; nothing here can substitute. Finding this now is why P1 checks the models, though
  only P8 needs them.

### P2 — `vision-ocr`: `decode`, `read` at C0, and a geometry self-test

- **Goal**: boxes out of Vision, in the coordinates `screen::TextBox` means, proven without a photo.
- **Read first**: calls 2, 4 and finding 7; `crates/screen/src/lib.rs:75–110` (`TextBox`).
- **Files**: `spikes/vision-ocr/vision_ocr.swift`.
- **Do**: subcommands `decode`, `read` (C0, no `rgb` yet, printing `"rgb": null`), `selftest`. The
  self-test draws known words at known rectangles on a dark image and fails unless each comes back with
  the right text and its box within 5% of the rectangle's height of where it was drawn, top-left origin.
  That is what catches a flipped y, which would otherwise cost a measurement to notice.
- **Done when**: `vision-ocr selftest` exits 0, and `decode` then `read` on `IMG_2889.HEIC` gives a PNG of
  the photo's dimensions, upright by eye (Read tool), and boxes including `CARRY`-like label text.

### P3 — `TextBox.rgb` in Swift, held exactly to Rust's `glyph_rgb`

- **Goal**: Swift's `rgb` is the definition, not an approximation of it.
- **Read first**: call 5, call 6; `crates/ocr/src/recognizer.rs:125–175` (`glyph_rgb`, `clipped`).
- **Files**: `vision_ocr.swift`; `spikes/vision-ocr/lab-images/` (`Cargo.toml` per call 6, `Cargo.lock`,
  `src/main.rs` with `glyph`).
- **Do**: fill `rgb` in `read`. `lab-images glyph` computes Rust's answer for the same boxes on the same
  PNG. Compare both on all six stills' decoded PNGs and on one PNG Rust wrote (`frames/`, any image
  through `image`'s encoder), since those are untagged.
- **Done when**: every box's three channels are **equal as doubles** across the two, on all of them.
- **If**: they differ, the fault is Swift's pixel access or its sort, so fix Swift until they agree. Do not
  loosen the comparison. Colour conversion in `decode` is not a difference here, because both read the
  same PNG.

### P4 — Grid frames off the five clips, with the lab's rule

- **Goal**: the frames the lab's clip route reads, on disk as PNGs.
- **Read first**: call 6; rule 6; `crates/lab/src/shot.rs:780–870` (`GridFrame`, `write_grid_frame`).
- **Files**: `lab-images/src/main.rs` (`grid-frame`).
- **Do**: `GOLF_FFMPEG=… lab-images grid-frame data/raw/shot_screens/IMG_n.MOV
  spikes/vision-ocr/frames/grid/IMG_n.png` for the five.
- **Done when**: five PNGs, each with its frame numbers recorded. Look at one by eye. M42 P14 chose frames
  31 of 36, 17 of 61, 21 of 55, 29 of 31 and 41 of 59 under ffmpeg 9.0.2, and these are compared.
- **If**: a frame index differs under 7.1, record it and keep the frame the rule chose here; that is not a
  stop. If 7.1 cannot decode a clip at all, the fallback is `AVAssetImageGenerator` in `vision_ocr.swift`
  over every frame into `lab-images`, with `impact::sharpest_frame`'s rule, recorded as a finding.

### P5 — The lab's working images

- **Goal**: `lab` mode's inputs.
- **Read first**: call 2's second bullet; call 3; call 6; `crates/ocr/src/rectify.rs:160–180`.
- **Files**: `lab-images/src/main.rs` (`working`); `probe.py` (`prepare`, recording each image's quad into
  `frames/lab/<id>.json`).
- **Do**: `probe.py prepare` decodes all 21 stills to `frames/decoded/<id>.png` (`raw` mode's inputs) and
  runs `working` over all 26 captures: JPEG and PNG directly, HEIC from `frames/decoded/`, clips from
  `frames/grid/`.
- **Done when**: 26 images and sidecars. On the 15 HD Golf photos, whether a quad was found agrees with
  each vector's `input.notes` (the uncropped note present or not), except that the one HEIC may differ
  (call 2), which is a finding if it does. Look at two Full Swing images by eye.
- **If**: a JPEG or PNG disagrees, record it, since `crates/ocr/tests/read.rs` was never run here; not a
  stop.

### P6 — `vision-ocr rectify`: the phone's rectifier stand-in

- **Goal**: `vision` mode's inputs.
- **Read first**: call 7; call 3.
- **Files**: `vision_ocr.swift` (`rectify`); `probe.py` (`prepare` gains it).
- **Do**: run over all 26, writing `frames/vision/<id>.png` and `.json`.
- **Done when**: every capture has a sidecar. Every Full Swing rectification is looked at by eye, and the
  finding records for each: found or not, whether it is the whole screen, and whether it is upright.
- **If**: few rectangles are found, or the wrong one is, that is the finding. These values are not tuned
  (call 7), and `vision` mode falls back or reads what it found. **For P17**: say so.

### P7 — `probe.py measure`: the comparator, proven on controls

- **Goal**: a comparator that has been shown to see each thing it must see, before it measures anything.
- **Read first**: calls 3, 8, 9, 12 and 14; `crates/core/src/bin/golf_core.rs:81–92`, `:350–368`
  (`parse-screen`); `crates/screen/src/lib.rs:140–165` (`ScreenInput`); `spec/vectors/screen/hand/full-swing-grid.json`
  (its shape; not its boxes).
- **Files**: `probe.py` (`measure`, `control`, `results/`).
- **Do**: `measure --config C --set S [--mode M]` runs Vision for each capture, builds the input, runs
  `golf-core parse-screen`, classifies, prints the tables (cells per photo, totals, per-field, flagged, bars
  T1–T4 and pass or miss, the lit table), and writes `results/<config>.json`. `control` needs no Vision:
  1. every HD Golf vector's committed boxes through `parse-screen`, classified against its own `expected`,
     give no **wrong** and no **withheld**;
  2. `full-swing-grid.json`'s boxes, classified against `IMG_2889`'s truth, give 12 **read**;
  3. mutations on those boxes, each of which must be seen, and each measured in planning on this binary:
     `9899` changed to `9099` gives `backspin` **wrong** on an unflagged shot; `1.08` changed to `1.88` fails
     the smash identity, flags the shot, and gives `smash_factor` **caught**; the `R` box removed gives
     `launch_direction` **withheld**; `rgb` dropped from the `R` box gives the same; an empty box list gives a
     `null` shot, twelve **withheld** and one flagged.
  4. the bars: synthetic totals either side of each of T1–T4 flip that bar and only that one.
- **Done when**: `probe.py control` exits 0 and prints each control's result.
- **If**: control 1 or 2 fails on the parser's side, that contradicts `cargo test`, so **stop**.

### P8 — The lab's reader on the same inputs: the baseline rules must not move

- **Goal**: the lab's answers frozen before any rule, and a like-for-like comparison for P17.
- **Read first**: rule 6; finding 10; `crates/ocr/src/bin/golf_ocr.rs:1–50` (`--boxes`); `crates/lab/tests/full_swing_clips.rs:1–30`.
- **Files**: `spikes/vision-ocr/lab-boxes/<id>.json`; `probe.py` (`lab`, `lab-regress`).
- **Do**:
  1. `target/m33/release/golf-ocr read <image> --device full_swing --boxes` on the six stills'
     `raw` PNGs and the five grid frames. Eleven reads at roughly 10 to 170 s each, so run it as a
     background command and wait with Monitor.
  2. `probe.py lab` classifies them against the truth and prints the tables.
  3. `probe.py lab-regress --write` stores each parse's digest; without `--write` it compares.
  4. The clip gate, under `GOLF_FFMPEG`.
- **Done when**: both results are in the finding. Note this is the lab's reader on ImageIO's pixels with
  ort on arm64, so it may not be finding 10's 71 and 60. Any difference from those is a finding and not a
  failure, because what later phases hold to is **this** baseline.
- **If**: the models are absent, **stop** (`BLOCKED`). If the clip gate fails, record exactly how, and
  P13–P15 hold to that outcome.

### P9 — The first measurement: C0 on Full Swing's stills

- **Goal**: the pre-registered number.
- **Read first**: `thresholds.md`; calls 8, 9, 12 and 16; finding 10; P6's and P8's findings.
- **Do**: `measure --config C0 --set fs-still` on all three modes, then a second time, comparing the two
  runs' boxes byte for byte. Copy `frames/runs/` boxes to `boxes/C0/`. Crop and look at every **wrong** and
  **caught** cell (call 16).
- **Done when**: the finding holds, per mode: the T1–T4 table, read beside the lab's, every non-read cell
  with Vision's text for that tile, the lit table with its margins against `LIT_BLUE_RATIO`, and whether two
  runs matched. **State which modes pass.** Do not interpret past that.
- **If**: two runs differ, record it and use the first; not a stop. **For P12**: whether stills pass.

### P10 — C0 on the clip frames

- **Goal**: the clips' number at the same settings.
- **Read first**: call 9; P4's and P9's findings.
- **Do**: `measure --config C0 --set fs-clip` on all three modes. Crops as in P9.
- **Done when**: the finding holds the same table as P9's, beside the lab's 60. **For P12**: whether clips
  pass.

### P11 — C0 on HD Golf, diagnostic

- **Goal**: what Vision makes of the other screen, with no bar.
- **Read first**: call 14; `crates/ocr/tests/read.rs:60–100` (`DECLARED_DIFFERENCES`).
- **Do**: `measure --config C0 --set hd` on all three modes.
- **Done when**: the finding holds, per mode: read, withheld, wrong, caught and extra totals, flagged count,
  and the differences by photo and field. Note where the two photos ADR-034 named for the V tile
  (`2026-08-10-1`, `2026-08-23-1`) differ. **For P17**: this is an open item for M38, not a verdict.

### P12 — The Swift ladder, if C0 missed

- **Goal**: call 10, unattended.
- **Read first**: call 10; `results/C0.json`; P9's and P10's findings.
- **Condition**: C0 misses the target (stills pass on `raw` or `vision`, and clips pass on `raw` or
  `vision`). If C0 meets it, tick with "not needed; the final config is C0".
- **Files**: `vision_ocr.swift` (`--config`, with `+`); `probe.py` (`ladder`).
- **Do**: `probe.py ladder` runs the order in call 10 on `fs-still` and `fs-clip`, on all three modes, and
  writes each config's results. Copy the final config's boxes to `boxes/<final>/`.
- **Done when**: the finding holds one line per config measured (each mode's read, wrong, flagged, pass or
  miss, stills and clips) and **names the final config** by call 10's rule. **For P13**: the final config, and
  whether its condition holds.

### P13 — Parser rule 1, if still missing

- **Goal**: call 11, the commonest parser-fixable miss.
- **Read first**: call 11; P12's and P8's findings; `crates/screen/src/parser.rs:1–210` and `:306–440`;
  `crates/screen/src/profile.rs:140–220`; `crates/screen/tests/hand.rs:1–100`; one of
  `spec/vectors/screen/hand/full-swing-*.json`; the M42 plan's §P10 (`grep -n '^### P10' docs/plans/m42-full-swing.md`).
- **Condition**: call 11's. Otherwise tick "not needed".
- **Do**:
  1. `probe.py failures --config <final>` lists every non-read cell on the target arms, and whether the
     printed value is in Vision's boxes for that tile (call 11's item 1).
  2. If no cell is parser-fixable, tick with "not needed: what is missing is the recognizer's", naming the
     cells. P14 and P15 then tick the same way.
  3. Otherwise group the cells by cause, and take the group with most cells, stills before clips, then
     profile field order. Write a failing hand vector first, then the rule.
  4. All eight of call 11's checks. If any fails, withdraw the rule (call 11) and record why.
- **Done when**: the rule is accepted or withdrawn, and the finding holds the new read counts on both
  arms. **For P14**: whether the condition still holds. **For P18**: the rule, for ADR-014 and CONFORMANCE.
  **For P19**: that a rule fired.

### P14 — Parser rule 2, if still missing

Exactly P13, with **Read first** adding P13's finding, and the next group by the same order. A group P13
withdrew is not retried.

### P15 — Parser rule 3, if still missing

Exactly P14. This is the cap: whatever is still missing after it is the measured result.

### P16 — The confidence calibration

- **Goal**: ADR-014's question, whether Vision's coarse confidences change `needs_review`, and what the
  phone should pass as `min_confidence`.
- **Read first**: finding 7; `crates/screen/src/parser.rs:195–205`, `:1325–1375` (`score`);
  `crates/screen/src/validate.rs:130–160`; the final config's results.
- **Files**: `probe.py` (`calibrate`).
- **Do**: at the final config, over all three sets and the phone modes, report:
  1. the histogram of Vision's box confidences, and how many distinct values appear;
  2. each shot's `parse_confidence` and which boxes it averaged, against the lab's for the same capture;
  3. which flags came from a physics check and which from confidence alone;
  4. the what-if: at `min_confidence` 0.5, 0.6, 0.7, 0.8 and 0.9, which shots flag, and whether each
     **wrong** cell would have been caught.
- **The recommendation, by rule**: the lowest of 0.6, 0.7, 0.8 and 0.9 that flags every shot holding a
  **wrong** cell, without taking stills past T4's ceiling; 0.6 if none does. `min_confidence` is the
  caller's (`ScreenInput`), so this changes no code and no vector. If the OCR term proves a constant, say so.
  Changing `W_OCR` would move every screen vector, so it is **named for M38 and not done here**.
- **Done when**: the finding holds all four and the recommended value. **For P17**: both. The verdict stays
  as measured at 0.6 (call 9).

### P17 — The verdict: `log.md`, and what M38 is told

- **Goal**: the answer, in the house form.
- **Read first**: `spikes/mediapipe-hands/log.md:1–30` for form; `thresholds.md`; every finding addressed to
  P17, and the final config's results.
- **Files**: `spikes/vision-ocr/log.md`; `probe.py` (`verdict`, which recomputes call 9's table from
  `results/`).
- **Do**: open with the verdict line by call 9's table, with its label (**tuned**, **tuned, parser**, or
  neither), and the clips' verdict beside it. Then what was measured (OS build, revision, the mode table, the
  lab beside each), the lit table, what was tried, and what Vision did on HD Golf. Then:
  - **What this does not show**, plainly: six stills; macOS's Vision and not iOS's; a VisionKit stand-in, not
    VisionKit; photos from one setup, not a new bay session; any tuning done on the photos measured. M38's
    plugin and P5 are what test these.
  - **For M38**, a list it can plan from: the settings that won; which input path (`raw`, `vision` or both);
    the colour handling and the lit margin; the recommended `min_confidence`; any parser rule that landed;
    that committed boxes in `boxes/` can seed vectors; rule 7's five failures for its P0; clippy's 28 lines;
    the missing extras; Homebrew's ownership; whether HD Golf reads, as an open item.
  - On **NO-GO**, what each diagnostic said (did `lab` pass? was the failure recognition, or the parse?), and
    the alternatives ADR-034 already named, adding one fact: `crates/ocr` now exists in Rust, which is not what
    ADR-034 priced when it set on-device PaddleOCR aside. **Recommend nothing further**: the choice is the
    user's.
- **Done when**: every bar is reproduced from `results/` by `probe.py verdict`, which prints the same verdict the
  log states.

### P18 — Docs

- **Goal**: every doc that describes this says so, and docs-truth is green.
- **Read first**: every finding addressed to P18; the addendum style of ADR-034's and ADR-014's last addenda;
  `docs/README.md`'s rows for those ADRs; `docs/CONFORMANCE.md:612–620` and `:2064–2072`.
- **Files**:
  - `docs/decisions/034-shot-first-phone-first.md`: an addendum (M33): the verdict; Full Swing decides, HD Golf
    is diagnostic, and the parser is Rust's; what the sub-decision's "biggest unknown" now stands at; what is
    still unmeasured.
  - `docs/decisions/014-screen-capture-shot-ingestion.md`: an addendum (M33): Vision as the phone's recognizer,
    its confidences' measured behaviour against the OCR term, and the recommended `min_confidence`; any rule
    from P13–P15, with why no `SCREEN_PARSER_VERSION` bump.
  - `docs/README.md`: each touched ADR's addendum count in its map row.
  - `docs/CONFORMANCE.md`: the two sentences saying M33 "measures", now what it measured. If a rule fired: §2's
    screen count and the `hand/` row.
  - `docs/plans/m31-m40-shot-first-pivot.md`: the checklist row, a note at the head of §M33 that this plan
    replaced it, and §M38's bullets as P17's list changes them.
  - `spikes/README.md`: an entry for `vision-ocr/`, in the form of the others.
  - `CLAUDE.md`: on a Mac the Python commands are `.venv/bin/python`, in the Commands block's comment; a clause
    on M33 in the sentence where ADR-034's route is described, if the verdict changes it. If a rule fired: the
    `crates/screen` row. Nothing else: it holds no counts.
  - `crates/screen/` module docs, if a rule fired (done by that phase; check).
- **Done when**: `tests/test_docs_truth.py` is green.
- **Note**: do not touch `docs/README.md`'s document count; P19 does, with `log.md` staged.

### P19 — Close: verify, WORKLOG, ROADMAP, memory, and the commit

- **Goal**: the milestone verified and committed.
- **Do**:
  1. Every command under "Verify" that lists P19, recording each result. `git diff --stat -- src scripts tests`
     empty. `data/` untouched.
  2. `WORKLOG.md`: a top entry in the house form: what was built, the verdict, what changed against the plan,
     what M38 should know, what is left open (call 15 if a rule fired), and the verify results.
  3. `ROADMAP.md`: the M33 row and §M33 (✅ with the verdict and its labels); the M38 row's "Needs": on **GO** and
     **CONDITIONAL**, unblocked, with what P17 said; on **NO-GO**, blocked on the user's decision. The `Last
     Updated` date. Nothing else.
  4. `docs/README.md`: the count, with `log.md` tracked (88: seven becomes eight in `spikes/`, 14 becomes 15
     outside `docs/`).
  5. Memory: update `golf-coach-status` (the Mac now holds the photos and models; what the verdict was, as a
     pointer to `log.md`) and its `MEMORY.md` line.
  6. `git add` the milestone's files (never `data/`, never `target/`, never `frames/`), and one commit onto
     `main`: `M33: Apple Vision on Full Swing's screen — <verdict and labels>`, the body listing the parts,
     ending with the attribution lines. Not pushed.
  7. The close report to the user: the verdict and its labels; stills and clips, each against the lab; what
     tuning it took; what M38 now knows; and on a fired rule, call 15's command for the Windows box.
- **Done when**: every verify command is as the rules say, the commit is made, and `git status` is clean except
  ignored files.

---

## Exit

- `spikes/vision-ocr/thresholds.md` is in git history ahead of every measurement.
- Vision's boxes on Full Swing's six stills and five clip frames, through `golf-core parse-screen`, are
  classified against the truth table on three modes, with a verdict by call 9's table that `probe.py verdict`
  reproduces. GO, CONDITIONAL and NO-GO all close the milestone: the spike's job is to answer.
- HD Golf is measured and logged.
- The phone's `rgb` is held exactly to `glyph_rgb`, and the lit margin measured on the phone's colour handling.
- A recommended `min_confidence`, with the confidences' behaviour measured.
- Any parser rule that landed is opt-in, hand-vectored and moved nothing committed.
- Docs, WORKLOG, ROADMAP and memory say all of it, in two commits, neither pushed.

---

## Phase findings

### P0 (2026-11-04)

- The plan and `spikes/vision-ocr/thresholds.md` are committed, ahead of any real photo. Only synthetic
  renders went through Vision in planning (finding 7).
- `docs/README.md` counts 87 documents. The pointers are ROADMAP's M33 row and `**Status**` line, and the
  program plan's checklist row.
- **For P1**: when this was written, `data/` held only its README. The user is copying the photos and
  `data/models/ocr/` from the Windows box, and P1 stops as `BLOCKED` until both are here.

### P1 (2026-11-04) — stopped at its precondition, nothing built

- **Stopped by rule 3 and P1's own "If".** None of the 26 captures is on this Mac, and neither are the
  models. Checked path by path: 0 of the 11 Full Swing captures, 0 of the 15 HD Golf photos, and no
  `data/models/`. `data/` still holds only its README, last changed at 08:32, so the copy P0's finding
  names has not reached it. No digest could be checked, and the truth table was not compared with `PHOTOS`.
- **The copy has to reach further than `data/raw/shot_screens/`.** By each vector's `input.image_path`,
  13 of HD Golf's 15 are under `data/processed/sessions/` (`2026-08-10/{1,2}` and `2026-08-23/{1..11}`,
  each a `shot_screen.<digest>.*`). The other two are `data/raw/shot_screens/IMG_2738.jpeg` and
  `IMG_2739.jpeg`. The models are `data/models/ocr/`, holding `PP-OCRv6_medium_det/`,
  `PP-LCNet_x1_0_textline_ori/` and `PP-OCRv6_medium_rec/`, each with a `model.onnx`
  (`crates/ocr/models.json`).
- **No copy turned up elsewhere, though that is not proof.** A search of `$HOME` for `IMG_2889.*`,
  `2026-10-07-1.jpg`, `IMG_2738.jpeg` and `shot_screen.*` found nothing. It skipped `Library`, `.cargo`,
  `.rustup` and `target/`, and was refused the Photos library and the Trash. `/Volumes` lists only
  `Macintosh HD`.
- **The tools are present**, by path only, with no version checked: `swiftc`, `cargo` and rule 6's ffmpeg.
- **For P1, on resuming**: the row stays 🔄, and there is no partial work to keep. `probe.py`, `photos.json`
  and `truth_full_swing.json` were not written, because rule 3 stops the phase before them, and this finding
  is the only change in the tree. Start from "Do", once the copy has landed.

### P1, resumed (2026-11-04) — the six pasted copies held to their digests

- **Resumed from the row's 🔄, with nothing to keep.** The marker-time `find` listed only this plan, and
  `spikes/vision-ocr/` held only `thresholds.md` and `captures/`. The entry above stands as that attempt's
  record. This run is on the decision of 2026-11-04, so `photos.json` holds six entries, not 26.
- **`probe.py check`: 6 of 6 captures match their digests**, and `IMG_2888` matches its own as "not a
  capture". It prints as outstanding, all absent: the 6 original stills, the 5 clips, HD Golf's 15 and the 3
  models. Tools by version: Swift 6.2.3, cargo 1.99.1, ffmpeg 7.1, macOS 26.2 (25C56).
- **The throwaway comparison found no difference in values.** `truth_full_swing.json`'s 72 cells equal
  `PHOTOS`' and the plan's table's, and `truth()` equals `Photo::truth` reworked from `PHOTOS`' numbers, all
  by `==`. `PHOTOS`' six digests equal each entry's `copy_of` and differ from every copy, as the decision
  expected. `probe.py`'s typed digests equal the plan's two tables. The scripts are deleted (rule 14).
- **`check`'s stops were proven on scratch manifests**: a wrong digest, a missing capture, a hand-edited
  manifest, no manifest, and a present file with a wrong digest in an outstanding group each exit 1. The
  last one's first try proved nothing, because its path ran through a directory that does not exist and read
  as absent, so it was redone on a file that is there.
- **For P2**: the copies carry no orientation tag, so `applyOrientationProperty` does nothing on them, and
  all are Display P3, so call 2's decode still converts colour. `IMG_2889`, `IMG_2893` and `IMG_2894` are
  2576×1932, and `IMG_2891`, `IMG_2892` and `2026-10-07-1` are 1932×2576. Nobody looked at whether each is
  upright in this phase, and finding 9's "no rotation vote needed" was measured on the originals, so a
  sideways copy would show up only as a miss.
- **For P3**: `cargo` is not on `PATH` in this shell. Run `. "$HOME/.cargo/env"` first (rule 3). `check` finds it
  at `~/.cargo/bin/cargo` and says so.
- **For P7**: `photos.json` is `{"_note", "captures": [{id, set, path, sha256, device, truth, copy_of: {path,
  sha256}}]}`. `probe.truth(id)` gives the twelve fields as floats in the table's order. `main()` adds a
  subcommand in one line, and each returns its exit code.
- **For P17**: choices this plan did not spell out, each taken under rule 1. Every manifest entry has a
  `copy_of`, beyond call 1's keys, so that a copy cannot be read as the original and the "provisional: pasted
  copies" label rests on data. `check` stops on a capture missing or differing, on any file present with the
  wrong digest (an outstanding group's too), on a manifest that is not what `build-manifest` writes, and on no
  `swiftc` or `cargo`. A missing ffmpeg is printed and is not a stop, because only P4 and P8 would call it and
  neither runs. `IMG_2888` is held to its digest though it is not a capture. `check` prints what is outstanding
  on every run, so the list "What this does not show" needs is one command away.

### P2 (2026-11-04) — Vision's boxes are in the right place, and they are not the ink's

- **What landed**: `spikes/vision-ocr/vision_ocr.swift`, with `decode`, `read` at C0 (`"rgb": null`) and
  `selftest [DIR]`, built by the plan's `swiftc` line to `target/m33/vision-ocr`. `selftest` exits 0, and
  three runs print byte-identical tables. `probe.py check` ran first (rule 9).
- **The photo**: `captures/IMG_2889.jpg` decodes to 2576×1932, profile `sRGB IEC61966-2.1` by `sips`, upright
  by eye. `read` returns 42 boxes in 0.25 s, among them `CARRY YD`, and a second read is byte-identical. Crops
  cut at the printed coordinates of `CARRY YD` and `SHOT DATA` landed on those words, so the top-left origin
  is checked on a real photo as well as on the drawn image.
- **The plan's 5% does not hold as written on every word, because `boundingBox` is not the ink's rectangle.**
  Nearly every edge sits outside the ink, typically by 2–7 px and by up to 45 on a 1600×1000 frame, in steps of
  about 2.33 px. The box also runs long below the ink: its centre sits low by 8–16% of the ink's height on a word
  with a point (`182.4`, `102.5`, at 23–117 px), and usually under 7% on a four-digit integer, though not
  reliably: `9899` at 96 pt gave 0.80 px at (300, 200), 4.75 at (1040, 110) and 8.08 at (1040, 200). A box also
  depends on the rest of the image, since moving `9899` changed `182.4`'s box, about 500 px away, from a bottom
  overhang of 33.26 px to 26.59.
- **The self-test tries each injected bug and fails on all six**, each in a throwaway build: y flipped (28
  failures, `CARRY` 812 px off), `minY` for `maxY` (28, 57 px), W and H swapped (28), orientation not applied
  (16), the drawing's own flip wrong (35, caught by scanning the file's rows before Vision is asked anything,
  so a drawing bug cannot cancel a mapping bug), and output in P3 rather than sRGB (3).
- **Decode beyond the plan's ask**, because the originals will need it and no HEIC is here to try it on. An
  sRGB PNG comes through with no channel of any pixel changed. A Display P3 yellow (235, 200, 70) comes out (242,
  198, 17) against (242.0, 198.3, 16.8) worked by hand from the primaries. Orientation 6 is applied exactly
  once for a JPEG and a HEIC, with no tag left in the PNG. That HEIC was written by ImageIO, so a phone's own
  HEIC is still untried.
- **For P3**: `Pixels` (the file's bytes, no colour matching, as call 5 asks) and `loadCGImage` are already there,
  `Box` needs the field, and `readJSON`'s row prints the `null`. The PNGs `decode` writes are RGBA with alpha 255,
  and all six are in `frames/decoded/`. **Padding reaches `rgb`**: a padded box holds more ground than the lab's
  boxes did, and the brightest quarter of a thin letter's box may no longer be all strokes. So Rust's `glyph_rgb`
  over Vision's box is the definition, but M42's measured margins were on the lab's boxes. P9's lit table is where
  this shows. Do not hand `CGContext(data: &bytes, …)` a Swift array: the pointer is good only for that call, and
  this file's first colour patches read back right on one build and as zeros on the next.
- **For P6**: `decode` renders through `ciContext.createCGImage` and `writeImage`, and `writeImage` makes the
  directory. `CIContext.writePNGRepresentation` would not have carried a stale Orientation either: tried on the
  tagged file, Core Image had already dropped the tag.
- **For P9**: all six decoded stills are upright by eye, at 700 px (P1 asked), so a miss will not be rotation. On
  `IMG_2889`, which is a look at one photo and not a measurement, `CLUBSPEED MPHBALLSPEED MPH` comes back as one
  box, `L`, `D` and `F` come back alone with no `R`, and the playback controls and the monitor's logo add `11`,
  `DD` and `DALI`. Values in one row have unequal boxes (`134` is 82 px tall and `127.1` is 101), so if a miss
  looks geometric, the padding above is the first suspect.
- **For P12**: `read --config` refuses all but `C0`, so a rung cannot print C0's boxes under its own label.
- **For P16**: on that one photo, 41 of 42 confidences are 1.00 and `DALI` is 0.30, so finding 7's constant is
  nearly but not quite so with correction off.
- **For P17**: choices under rule 1, none touching `thresholds.md`. The 5% stands as the plan's number but is
  held on the box's **centre**, and on five words of the seven drawn (`CARRY`, `BACKSPIN`, `4199`, `R`, `TOTAL`).
  `9899` and `182.4` missed it on the fixture's first run, so they stay as printed rows under a weaker check (right
  text, the ink's centre inside the box), which a flip still fails. That split was read off the result, and moving
  `9899` once made it worse, so the five pass on Vision's behaviour here rather than on a margin that is promised:
  a later macOS could shift a box past the limit with nothing wrong in the mapping. Its worth is the six injected
  bugs above, not the number. `selftest` writes to a temporary directory and removes it, unless given `DIR`.

### P3 (2026-11-04) — Swift's `rgb` equals Rust's `glyph_rgb` as doubles on every box

- **What landed**: `vision_ocr.swift` has `clipped` and `glyphRGB`, and `read` fills `rgb`. `lab-images/` holds
  call 6's `Cargo.toml`, the copied `Cargo.lock` and `src/main.rs` with `glyph`. `probe.py check` ran first (rule
  9), and re-running `decode` on all six captures gave PNGs byte-identical to `frames/decoded/`, so the chain from
  digest to pixels held.
- **The comparison, by `==` after the JSON trip, 0 boxes differing anywhere**: all six decoded stills (42, 41, 43,
  39, 43 and 43 boxes, 753 channels) and one PNG written by `image`'s encoder (`frames/rust-written/IMG_2889.png`,
  42 boxes). `read` is still byte-identical on a second run, and `selftest` still passes.
- **Vision's boxes cannot try the whole definition, so this phase added a fourth subcommand.** No box on these
  stills reaches an image edge, and none is empty. `vision-ocr glyph IMAGE BOXES.json` mirrors `lab-images glyph`,
  and both took 3,054 boxes given by hand, in quarter-pixel steps: off each edge, wholly outside, no width, negative
  width, every small `n`, and 3,000 seeded at random. On `decoded/IMG_2889`, `rust-written/IMG_2889` and a built
  `rust-written/ties.png` (64×48, every pixel from a palette of equal sums and different colours), 0 differ, with
  15, 15 and 123 of them `null`. A box with no width at a fractional x holds one column in both, which `glyph_rgb`'s
  doc comment does not say and its code does.
- **The comparison sees what it must.** Each fault was put in a throwaway build and run on all of the above, with
  boxes differing per still in the range shown: ties reversed 30–38 of each still's boxes; the cut rounded up 15–28;
  the whole box's mean, the far edge floored, BGR and a truncated mean all of them; the near edge rounded 26–33. A
  clip one pixel short moved no box on the stills and 180 to 1,984 through `glyph`, which is why that seam exists.
  Two faults moved nothing, and both are explained. Sorting on the sum alone: Swift's `sort` is stable here, though
  not documented to be, so the key's index is a guarantee that cannot be seen failing. Colour matching through a
  `CGContext` into sRGB: every input is already sRGB to ImageIO, so it does nothing, and on a copy of `IMG_2889`
  tagged Display P3 with no pixel changed it moved 489 of 500 boxes while the real build moved 0.
- **Call 5's worry about untagged PNGs does not bite on this Mac**: ImageIO reports Rust's RGB8 PNG and `decode`'s
  PNG alike as 32 bits a pixel, `noneSkipLast`, default byte order, colour space sRGB. `Pixels` now refuses a
  little-endian image too, since that would put blue first with nothing else looking wrong.
- **For P5**: the binary is `target/m33/release/vision-spike-lab-images`, from call 6's package name, not
  `lab-images`. The first build took 21 s with `ort`'s runtime already cached. The lock holds 113 registry packages,
  all of them in the root's lock at the same version, and `cargo metadata` at the root does not list the helper.
  `glyph`'s pattern is the one to extend, and `frames/rust-written/` is left there but nothing depends on it.
- **For P9**: a look at one photo, not a measurement. On `IMG_2889`, `D` is (248.0, 217.5, 41.1), a ratio of about
  0.19, and `L` and `F` are about 0.98 and 0.94, each deciding the table's sign. So the padding P2 warned of did not
  reach the sign letters here, and the lit letter sits below M42's 0.32–0.48, which is what converting P3 to sRGB
  should do to a yellow's blue. Whether that holds is P9's lit table.
- **For P12**: `glyphRGB` takes a `Pixels` and a rectangle, and `readJSON` is where it is called, outside `recognize`
  on purpose. C2 and C4 pass the image call 10 names for each. Nothing in the ladder needs to touch `glyphRGB`.
- **For P17**: a choice under rule 1. `vision-ocr glyph` is beyond call 5, for the reason above. The comparison and
  fault scripts were throwaway and are deleted (rule 14), so this entry is their record. **For M38's plugin**: ties
  moved 30–38 boxes a still, so "the brightest quarter" with any other tie rule does not reproduce `TextBox.rgb` to
  the bit on a real photo. It needs `glyphRGB`'s key, or a sort it can prove stable.

### P4 (2026-11-04) — not run: the five clips are not on this Mac

Run under the decision of 2026-11-04, whose table gives P4 "Not run: no clips", so nothing was built or decoded.

- **The proof.** `probe.py check` exits 0 with 6 of 6 captures matching, prints all five
  `data/raw/shot_screens/IMG_n.MOV` as absent, and ends "the clips 0 of 5". `data/` holds only its `README.md`, so
  `data/raw/` does not exist, and `git ls-files` tracks no `.MOV`. A wider search of the home directory did not
  finish, sat for three minutes and was killed, so it is not evidence either way. The clips may sit elsewhere on this
  Mac, and the manifest's paths are the only place rule 9 would accept them from anyway.
- **What is not the obstacle.** ffmpeg 7.1 is at rule 6's path and prints its version, which is all that was asked
  of it. Whether it decodes these clips is unmeasured, and so is whether it picks M42 P14's frames (31 of 36, 17 of
  61, 21 of 55, 29 of 31, 41 of 59, under ffmpeg 9.0.2).
- **`grid-frame` is not written.** `lab-images/src/main.rs` still dispatches only `glyph`. A subcommand with nothing
  to run it on would be untested code left in the record. `golf-lab` is already in `lab-images/Cargo.toml`, so the
  row that runs this later adds only the subcommand. `frames/grid/` does not exist.
- **For P5**: its `frames/grid/` clause does not apply. The decision's table has it read the six stills, and no grid
  frame exists to feed `working`.
- **For P10**: the same absence stands unless the clips have arrived, so re-run `probe.py check` for its proof. Do
  not look for grid frames first: with no `grid-frame` subcommand, none can exist.
- **For P17**: the clips' verdict is **not measured**, and P4 contributes no number to it. When the clips reach this
  Mac, the new row needs this phase as written, including its comparison against M42 P14's frames and its
  `AVAssetImageGenerator` fallback.

### P5 (2026-11-04) — six lab images, a quad on all six, and the copies are now known to be scaled

- **What landed**: `lab-images working IMAGE OUT.png`, and `probe.py prepare`, which holds all six to their digests
  (rule 9), then writes `frames/decoded/<id>.png` and `frames/lab/<id>.{png,json}`. `probe.py check` ran first. The
  decoded PNGs came out byte-identical to P2's, and a second `prepare` rebuilt all 18 files byte-identical. The
  rule 5 diff is empty.
- **The quad, on the copies**: found on 6 of 6, so every sidecar's `notes` is `[]` and none is uncropped. Five are
  `rotated_box` (Canny 30, closing 5) and `2026-10-07-1` is `corners` (Canny 10, closing 5, ε 0.04). The images are
  1600 wide and 922, 830, 1116, 834, 724 and 1346 tall, in the manifest's order. `load_image`'s hand-ported JPEG
  decoder took all six, the first time it has run on this Mac.
- **By eye**: all six looked at, where the plan asks two. Every one of the twelve tiles is inside its image, with all
  twelve values agreeing with the truth table and the lit letters still yellow, at 900 px wide. The finder cuts the
  monitor, not the panel, so the video replay, the course art and `CONTINUE` are all in the image. A `rotated_box`
  corrects a turn and no perspective: `IMG_2893` is still visibly skewed, with black wedges at its edges where the
  corners overrun the photo. `IMG_2894`'s lower edge cuts through `CONTINUE` and takes no tile.
- **"Possibly scaled" is settled for the five HEICs: scaled, by 2576/5712 per side, about a fifth of the pixels.** The
  M42 plan records the originals as 5712×4284 and 4284×5712. M42 P3 also recorded the fixed finder's corners on
  `IMG_2893` and `IMG_2894`, and these copies' corners, multiplied by 5712/2576, land within 1.3 px of them at full size
  (0.2 px on `IMG_2894`), through the same path, `RotatedBox` (30, 5). So the finder is indifferent to the re-encoding
  on those two. `2026-10-07-1`'s original size is not recorded anywhere this phase read.
- **`lab` keeps the Display P3 numbers and `raw` does not.** All six copies are tagged Display P3 (`sips`). `load_image`
  ignores the tag and Rust's PNG is untagged, which ImageIO then calls sRGB (P3), while `vision-ocr decode` converts. One
  look, not a measurement: Vision's raw boxes over each decode, as `b ÷ min(r, g)`, before CLAHE, on the 15 letter boxes
  returned. On the `load_image` decode, lit 0.34–0.45 and unlit 0.85–0.99. On `decode`'s sRGB, lit 0.10–0.36 and unlit
  0.83–0.99. So the lab decode of a copy lands on M42's lit 0.32–0.48, and the conversion is what moves it. Every letter
  stays on its own side of `LIT_BLUE_RATIO`'s 0.7 either way, and the lowest unlit (`F` on `IMG_2894`) is under M42's
  0.89.
- **For P6**: each `frames/lab/<id>.json` has `quad.corners` in photo pixels, the lab's answer to set beside Vision's
  rectangle. Note what they are, though: on five of six a rotated box around the whole monitor, some corners off the
  photo, so disagreeing with them is not by itself a fault in Vision's. Black wedges in a `lab` image are the warp's
  border, not the scene.
- **For P9**: `lab` mode's image is `frames/lab/<id>.png` and its `notes` the sidecar's, so neither needs working out.
  Box coordinates in that mode are in the 1600-wide image, and `rgb` in it has been through CLAHE on L\*. That and the
  P3 numbers above mean the lit table's `lab` and `raw` columns differ for two reasons, so a gap between them is not all
  colour conversion. A look at `raw` only: letter boxes are sparse, and `IMG_2893` returned `R` and nothing else, so no
  letter is there to decide its sidespin.
- **For later rows, when the originals and clips reach this Mac**: `prepare` refuses a capture that is not a JPEG or
  PNG, and `working` refuses a HEIC outright (tried on a HEIC written by `sips`: exit 1, no image), because `load_image`
  would pass it to rule 6's ffmpeg, which decodes one tile and reports success. The HEIC-from-`frames/decoded/` and
  clip-from-`frames/grid/` routes are not written, as P4 left `grid-frame`. Those rows add them to `lab_input`, and
  they are where the plan's HD Golf clause (quad against each vector's `input.notes`) finally runs.
- **For P17**: call 2's "`lab` is not the lab's exact pixels on a HEIC" reads differently in this run. All six go through
  the lab's own decode, but of a re-encoded copy, so `lab` is not the pixels M42 measured on any photo. Say so beside the
  scale above, which also weighs unevenly: `raw` gives Vision 2576 where the phone holds 5712, while `lab` shrinks to
  1600 either way. Choices under rule 1: `working` prints `quad`, `photo`, `image` and `notes` beyond call 6's "whether a
  quad was found", and `prepare` adds the digests and the macOS build, because `frames/` is gitignored and would
  otherwise carry no link to its capture (rule 8).

### P6 (2026-11-04) — a rectangle on one photo of six, and that one is the replay panel

- **What landed**: `vision-ocr rectify IN.png OUT.png`, with call 7's values untouched and `VNDetectRectanglesRequestRevision1`
  pinned (the SDK header's only one), and `probe.py prepare`, which now also writes `frames/vision/<id>.{png,json}` from
  `frames/decoded/`. `probe.py check` ran first. All six sidecars are there. A second `prepare`, and a third after a rebuild,
  left all 30 files under `frames/` byte-identical, so Core Image's render is repeatable here. The rule 5 diff is empty.
- **The result, each looked at by eye at 900 px on the long side, with both quads drawn over the decoded photo.** "Turned"
  is the lab's top edge from `frames/lab/<id>.json`, for scale.

  | id | found | whole screen? | upright? |
  |---|---|---|---|
  | `IMG_2889` | no, 0 rectangles | fell back: the whole photo, all twelve tiles in it | yes, turned about 1.8° |
  | `IMG_2891` | no, 0 rectangles | fell back, the same | yes, about 1.4° |
  | `IMG_2892` | no, 0 rectangles | fell back, the same | yes, about 0.3° |
  | `IMG_2893` | no, 0 rectangles | fell back, the same | yes, about 6.6°, and visibly skewed |
  | `IMG_2894` | **yes, the wrong one**: confidence 1.0, 0.115 of the photo | **no**: 767×902, the replay panel and the fairway showing through below it, and **none of the twelve tiles** | yes |
  | `2026-10-07-1` | no, 0 rectangles | fell back, the same | yes, about 2.6° |

  Each of the five fallbacks is the decoded PNG's own bytes (`cmp`), with the uncropped note in its sidecar.
- **Why, by eye and not by test**: neither side edge of the monitor is in any of the six frames. The golfer filled the frame
  with the tiles, so the screen runs off the photo left and right, and it has no four corners inside the picture to find.
  The lab's finder copes because it fits a box to the edges it can see, and on five photos its corners go off the picture for
  the same reason.
- **A look, not a measurement, and nothing was moved** (a throwaway request, deleted). With every limit opened (aspect 0–1,
  size 0.05, tolerance 45, confidence 0), Vision returns 30–39 rectangles a photo. On five photos the largest is 0.046–0.097 of
  the image and, by its coordinates, a sub-panel, or on `2026-10-07-1` the bezel below the screen. `IMG_2894`'s largest is 0.342, x 198–2462, y 419–1615, about the tile block
  and the replay panel together, and call 7's values did not return it. Nothing tried why. So looser values would not have
  found the monitor on five of six, and call 7's are not the obstacle there.
- **The self-test gained the rectifier**, where call 7 did not ask, for what the photos cannot show. It draws a skewed panel with
  a colour in three corners and a dark panel inside it, and a blank frame. It keeps the larger of the 2 offered, with its worst
  corner 2.62 px from the drawing. Six faults were each put in a throwaway build and each failed it: Core Image's corners
  flipped, the sidecar's y unflipped (722 px), top right swapped with bottom left in the filter (came out 730×1006), the smaller
  panel kept (203 px), the fallback re-encoded, and the note on the wrong branch. Two runs print byte-identical tables.
  The sidecar's corners are also right on a real photo: drawn on `IMG_2894`, they sit on the replay panel's edge.
- **A correction to P5's finding**: its "a rotated box around the whole monitor" overstates it. Drawn on the photos, the lab's
  quad is the monitor's outline only on `2026-10-07-1`. On the other five its top edge runs through the displayed sky, and on
  `IMG_2891` its lower edge meets `CONTINUE`, as P5 saw on `IMG_2894`. All twelve tiles are inside it on all six, which is what P5
  checked and what matters.
- **For P7**: `vision` mode's image is `frames/vision/<id>.png` and its `notes` the sidecar's. The sidecar holds call 7's `found`,
  `confidence` and `corners` (TL, TR, BR, BL, photo pixels from the top left, as the lab's `quad.corners`), plus `area_fraction`,
  `photo`, `image`, `notes`, `revision`, `os`, `candidates` (every rectangle returned, largest first), `id`, `input`,
  `input_sha256` (the decoded PNG's), `capture_sha256` and `image_sha256`.
- **For P9**: `vision` is not an independent arm here, and neither result below is a fault.
  - On five photos its input is `raw`'s exact bytes plus the note. By reading `crates/screen/src/lib.rs::record` and
    `validate.rs`'s `needs_review`, the note lands in `warnings` only, so those five should differ from `raw` by that line and no
    cell. A cell that differs is the harness's fault, since P2 found a second read byte-identical.
  - On `IMG_2894` it reads the replay panel, so no value of that shot can be read. **`vision` can therefore read at most 60 of 72,
    and T2's 66 is out of its reach**, on this input at any config. Stills can pass only on `raw`.
  - The rectified image holds 1,668 edge pixels of partial alpha (of 691,834, none at alpha 0), because the render is not
    flattened. `glyphRGB` reads the stored bytes and no box here is likely to reach that edge, but a later rectified screen could.
- **For P12**: no rung touches the input image, so the rectifier is not re-run per config and `vision`'s ceiling of 60 stands on
  every rung. "Better phone mode" in call 10's "best" will be `raw` throughout.
- **For P17**: say all of this. The stand-in found a rectangle on 1 of 6 and the wrong one, these values were not tuned (call 7),
  and `vision` falls back on five and reads a panel of no tiles on the sixth, as call 3's table says it should. So on this run
  the verdict's "`raw` or `vision`" is `raw` alone, and a miss on `vision` **says nothing about VisionKit's scanner**, which needs a
  device, and M38 measures it. What M38 may take from it, as a question and not a result: photos framed like these give a corner-finding
  rectifier nothing to find, so either the golfer is told to get the whole monitor in frame, which costs the tiles pixels, or the
  app does not depend on a rectifier. These are copies at 2576 on the long side. The same framing holds on the originals, but
  rectangle detection at full size is not measured. Choices under rule 1: `rectify` prints beyond call 7's three keys so that "the wrong one" can be checked
  against what was passed over; the fallback is the input's bytes rather than a second encoding; the self-test section above; and
  `prepare` runs the rectifier on every run, with the digests added as for `lab`.
- **For later rows, when the originals reach this Mac**: `prepare` rectifies from `frames/decoded/`, so a HEIC's route is already
  the one that runs, and only `lab_input` refuses it (P5). A clip's grid frame needs `frames/grid/`, which P4 did not write.

### P7 (2026-11-04) — the comparator sees what it must, and it has measured nothing

- **What landed**: `probe.py control` and `probe.py measure`, with `classify`, `classify_vector`, `summarize`, `judge`, `read_bars`,
  `lit_rows` and `parse_screen`. `probe.py check` ran first. **No capture went through `measure`**, so `results/` and `frames/runs/` do
  not exist yet, on purpose: P9 is the first measurement. The rule 5 diff is empty, `thresholds.md` is unchanged, and a `prepare` after
  this phase's change to `run_tool` left all 30 files under `frames/` byte-identical (it now runs with the repo root as its directory).
- **`probe.py control` exits 0**, needing no Vision and no photo, on `target/m33/release/golf-core`:

  | control | result |
  |---|---|
  | 1, the HD Golf vectors' committed boxes against their own `expected` | all 15, 168 read, and no withheld, wrong, caught or extra |
  | 2, `full-swing-grid.json` against `IMG_2889`'s truth | 12 read, unflagged |
  | 3, `9899` to `9099` | `backspin` wrong, unflagged |
  | 3, `1.08` to `1.88` | `smash_factor` caught, flagged |
  | 3, the `R` box removed | `launch_direction` withheld, unflagged |
  | 3, `rgb` dropped from the `R` box | `launch_direction` withheld, unflagged |
  | 3, an empty box list | a `null` shot, 12 withheld, 1 flagged |
  | 4, T1–T4 either side, on both sets | at the limit every bar passes, and one past it that bar alone misses |

  Each mutation's expected map names every non-read cell, so one that disturbed a second cell would fail.
- **Beyond the plan's list, each for something the list could not catch**: **1b** tells call 14's cells apart, because control 1 alone
  passes a classifier that says "read" for everything. A cell **one unit in the last place** off is wrong (rule 10), because every
  listed mutation moves a printed digit. A **caught** cell misses neither T1 nor any bar but is not read (call 9). **Control 5** holds
  the lit table's `is_lit` to the binary's sign at each limit, exactly and one step past, so that the table cannot explain a rule the
  binary does not apply.
- **One measured on this binary that P9 should have in mind**: the `R` box reading `L`, still lit, gives `launch_direction` **wrong on
  an unflagged shot**. `validate.rs` holds `launch_direction` to a range and no identity, so a sign read wrong by colour is a T1 failure
  and not a review.
- **The controls were themselves tested**: 29 faults, each put into the comparator in a throwaway build, each made `control` exit 1. A
  tolerance on `==`, caught and wrong swapped (both classifiers), a `null` shot unflagged, `needs_review` ignored, apex rounded, either
  sign flipped, `<` for `<=` and a limit forgiven by one on each of T1–T4, T1 counting caught, T3 on the highest field, a still's limit
  read as a clip's, `is_lit` on the greater channel, with `<`, with `>` and with no floor, HD's cells collapsed, the declared difference
  on any photo and gone, and a mutation that changes nothing.
- **`measure` was driven only with a stand-in for `vision-ocr read`**, from a throwaway script with outputs in the scratchpad, since
  the real thing is P9's. Committed grid boxes on all six gave `IMG_2889` 12 read, a flagged shot's cells caught, the others wrong, all
  three modes with their own images and notes, `--mode` merging into the file, and `repeat` printing `first`, `same` and `DIFFERS`. No
  boxes gave `FAILED`, flagged. The real binary's output on the self-test's **drawing** goes through `parse-screen` as a `null` shot (7
  boxes), so its keys are accepted. **A non-null parse of real Vision boxes has not happened here, and P9's is the first.**
- **Its stops, each proven on scratch inputs, each exit 1**: a set with no captures, a wrong capture digest, five captures against bars
  for six, a missing sidecar, a sidecar from another capture, a decoded or lab PNG not matching its sidecar, and a `thresholds.md` row
  that no longer parses. `read --config C1` still refuses in the binary. A miss exits 0.
- **For P9**: `measure --config C0 --set fs-still` runs all three modes, and prints every table this phase's Do names. Things it leaves to you:
  - The second run **overwrites** `frames/runs/C0/` and `results/C0.json`. So copy `frames/runs/C0/` to `boxes/C0/` and set `results/C0.json`
    aside **after the first run**, or "use the first" has nothing to use. `repeat` and each shot's `boxes_sha256` say whether they matched.
  - The `tile` column of "not read" is `provenance.raw_fields`, the text the parser kept. It is `None` where the parser kept none, as on a
    withheld tie, and then "Vision's text for that tile" comes from the boxes file. **Crops are yours** (call 16); `measure` writes none.
  - The lit table lists every box that is one letter, the tile's or a stray's, so `x,y` tell them apart, and `margin` is
    `LIT_BLUE_RATIO − ratio`, positive on the lit side. `sign decided` is the stored value's, beside the truth's.
  - `results/<config>.json` is `{config, min_confidence, thresholds_sha256, sets: {<set>: {label, limits, arms: {<mode>: {os, revision,
    summary, bars, pass, shots: {<id>: {flagged, cells: {<field>: {cell, value, truth}}, image, image_sha256, notes, boxes, boxes_sha256,
    repeat, failed_read, parse_confidence, parser_version, warnings, raw_fields, lit}}}}}}}`. Each boxes file's `image` is repo-relative.
- **For P12**: `measure` already passes `--config`, and stops if the binary prints a different config. So a rung needs only the Swift and
  `ladder`. `judge` and `summarize` are what call 10's "best" should read.
- **For P13**: `failures` is not written. `frames/runs/<config>/<mode>/<id>.boxes.json` and the same `.shot.json` are its inputs.
- **For P16**: each shot's `parse_confidence` and `warnings` are in `results/`, and box confidences in the boxes files. A flag from confidence
  alone adds no warning (`validate_parse`), and notes and the parser's own warnings sit on unflagged shots too, so `warnings` alone does
  not say why a shot flagged.
- **For P17**: `verdict` should read `sets.<set>.label`, which `measure` sets from data: a capture whose digest is not its `copy_of`'s.
  Choices under rule 1, none touching `thresholds.md`:
  - **Call 14's "non-provenance keys" is read as four fewer.** `shot_id`, `session_id`, `timestamp` and `source` come from the input or
    are stamped, so counting them would add four **read** to every shot that no screen printed. A difference in one is refused as the
    harness's fault. A key null on both sides is no cell.
  - The bars are **parsed from `thresholds.md`** and the lit constants **from `parser.rs`** at run time, where call 9 and call 12 say
    copied, so neither has a second copy, and a row that stops parsing is a stop. Control 4's change counts are typed by hand, so a
    misparsed limit fails there.
  - `measure` holds each frame back to its capture through the sidecars, beyond rule 9's capture digest, since `frames/` is gitignored.
  - An input's `image_path` and `image_sha256` are the capture's and not the mode's image, `shot_id` the capture's id, and `session_id`
    `m33`. Neither reaches a cell.
- **For later rows, when the files reach this Mac**: `measure --set fs-clip` and `--set hd` stop on "no capture", because the manifest holds
  neither and `mode_image` knows only the stills' paths, as with P5's `lab_input`. `fs-clip`'s limits are parsed and control 4 proves them.
  `classify_vector` and `DECLARED` are proven by controls 1 and 1b, and **no `measure` path calls them yet**: P11's row wires them.

### P8 (2026-11-04) — not run: the OCR models are not on this Mac, so there is no lab baseline

Run under the decision of 2026-11-04, whose table gives P8 "Not run: no models" and overrides the section's
`BLOCKED`. Nothing was read, built or written under `spikes/vision-ocr/`.

- **The proof.** `probe.py check` exits 0 with 6 of 6 captures matching and ends "the OCR models 0 of 3". The reader
  itself refuses: `target/m33/release/golf-ocr read spikes/vision-ocr/frames/decoded/IMG_2889.png --device full_swing
  --boxes` exits 1 with `PP-OCRv6_medium_det: …/data/models/ocr/PP-OCRv6_medium_det/model.onnx is missing`, before it
  reads a box. `git ls-files` tracks no `.onnx`, and neither `data/models/` nor `~/.paddlex/official_models/` exists.
  Spotlight (`mdfind`, indexing on) finds no `model.onnx` and nothing named `PP-OCRv6`, which is weak evidence since it
  skips hidden directories. A `find /` did not finish in two minutes and was killed, so it is not evidence either way.
- **Unlike the clips, the models need not be copied from the Windows box.** `crates/ocr/models.json`'s `conversion`
  records the recipe, and says reconverting reproduces every digest, which was not measured on this Mac. It needs
  Python 3.11, `paddlepaddle` 3.1.1, `paddle2onnx` 2.1.0 and PaddleX's downloaded sources. None of those is here (no
  `python3.11` on `PATH`, none of the three in `.venv`), and whether their wheels exist for arm64 macOS, or reproduce
  the digests here, is unmeasured. Nothing was tried: the decision's table says not run, and installing a toolchain is
  the user's choice.
- **The clip gate did not run.** It needs the clips as well as the models, and P4 proved those absent. Its test target
  was not built, so the gate's outcome on macOS is unmeasured, including whether ffmpeg 7.1 serves.
- **Not written**: `lab-boxes/` and `lab-boxes/parses.sha256` do not exist, and `probe.py` has no `lab` or `lab-regress`
  (it answers `invalid choice`, among `build-manifest`, `check`, `prepare`, `control` and `measure`). As with P4's
  `grid-frame`, a subcommand with no input to run on would be untested code left in the record, and a digest file
  with no parse behind it would be a baseline in name only.
- **So this changes nothing already measured, and it removes a comparison.** P3 built `ort`, and none of the reader's
  models has run through it on this Mac. Finding 10's numbers (71 of 72, 60 of 60, 0 of 6 flagged, the lit ranges)
  remain the lab's on the **originals**, measured off this Mac, and whether this Mac would reproduce any of them is
  unknown.
- **For P9**: no lab column, as the decision's table says. Do not fill it from finding 10, whose 71 of 72 is on the
  originals and these are copies, so it cannot sit beside a C0 number as like for like. It may be quoted only with that
  stated. No bar `thresholds.md` holds P9 to is measured against a lab read, so none is affected.
- **For P13, P14 and P15**: there is no baseline for call 11's check 6, and the Verify block's `probe.py lab-regress`
  and clip gate cannot run. The decision's table already covers this: no rule can be accepted in this run, so list any
  parser-fixable cells and tick `✅ Not run`. Do not write `lab-regress` for them.
- **For P17**: "What this does not show" lists the lab baseline, as the decision's table says, and adds that the
  reader's models have not run on this Mac. The verdict's CONDITIONAL arm still means what decision 8 says: `lab` mode
  in P9 is the lab's **rectifier** in front of Vision, which needs no model, so it is measured even with the lab's
  reader unmeasured.
- **For P19**: its report's list of what is outstanding can name reconverting as a second way to P8's row besides
  copying the models from the Windows box, unproven on this Mac as above.
- **For later rows, when the models reach this Mac**: this section as written, on whichever captures the row then holds.
  A baseline taken on the pasted copies is the copies' and would have to be re-taken on the originals, because
  `lab-regress` holds parses byte for byte and P5 found the copies scaled. The models belong at `data/models/ocr/`,
  which is where rule 3, `probe.py check` and the clip gate's `OcrReader::new(root.join(ocr::DEFAULT_DIR))` look, so
  `golf-ocr --models DIR` alone would not serve. Someone puts them there by hand, as rule 4 keeps a phase from writing
  under `data/`.

### P9 (2026-11-04) — C0 on the six copies: `raw` and `lab` pass, `vision` misses

Measured under the decision of 2026-11-04, so **provisional: pasted copies**, and with no lab column (P8). `probe.py check`
and `control` ran first. `thresholds.md` is unchanged, and `results/C0.json`'s `thresholds_sha256` is the committed file's.
The rule 5 diff is empty. macOS 26.2 (25C56), Vision revision 3, `min_confidence` 0.6.

- **The bars**, `measure --config C0 --set fs-still`, each limit parsed from `thresholds.md`:

  | mode | T1 wrong (0) | T2 read (at least 66) | T3 lowest field (at least 5 of 6) | T4 flagged (at most 1) | arm |
  |---|---|---|---|---|---|
  | `raw` | 0 | 71 | 5: `sidespin` | 0 | **PASS** |
  | `vision` | 0 | **59, miss** | **4: `sidespin`, miss** | 1 | **MISS** |
  | `lab` | 0 | 68 | 5: `launch_direction`, `apex_height`, `smash_factor`, `sidespin` | 1 | **PASS** |

  Read per photo, `raw` / `vision` / `lab`: `IMG_2889` 12/12/12, `IMG_2891` 12/12/12, `IMG_2892` 12/12/11, `IMG_2893` 11/11/11,
  `IMG_2894` 12/0/12, `2026-10-07-1` 12/12/10. A throwaway recount, with the plan's table typed afresh and neither `truth()` nor
  `classify()` in the path, gave the same 71, 59 and 68. **Two runs matched**: all 36 boxes and shot files byte for byte, and
  `results/C0.json` is the same apart from `repeat`.

- **Every cell not read, with Vision's text for that tile.** Each withheld cell is on an unflagged shot, except `vision`'s null shot and `lab`'s `2026-10-07-1`.

  | mode | id | field | cell | value / truth | Vision's boxes for the tile |
  |---|---|---|---|---|---|
  | `raw`, `vision` | `IMG_2893` | `sidespin` | withheld | none / -638 | `SIDESPIN RPM`, `638`, and no letter. By eye `D` (lit) and `F` are both printed, and Vision returned neither |
  | `vision` | `IMG_2894` | all twelve | withheld, FAILED | none | 0 boxes: its image is the replay panel (P6), so the shot is null and the one flag |
  | `lab` | `IMG_2892` | `sidespin` | withheld | none / -2032 | `S DESPIN R`, `2032`, and no letter |
  | `lab` | `IMG_2893` | `launch_direction` | withheld | none / 2.2 | `BALL DIRECTION DEG`, `2.2`, and no letter |
  | `lab` | `2026-10-07-1` | `apex_height` | withheld | none / 47.4 | `APEX RI`, so `no tile found for 'Apex Ft'`, and `142.2` went to the next tile |
  | `lab` | `2026-10-07-1` | `smash_factor` | **caught** | 142.2 / 1.43 | tile text `142.2 1.43`; flagged at confidence 0.542 by the speed cross-check and the range check |

  **Call 16's crop** (`frames/crops/C0-lab-2026-10-07-1-smash_factor.png`, the run's only **wrong** or **caught** cell) shows `142.2`
  under `APEX FT` and `1.43` under `SMASH FACTOR`. It agrees with the truth table. No arm has a **wrong** cell.

- **The lit table**: `b ÷ min(r, g)`, and in brackets the margin, `LIT_BLUE_RATIO` (0.7) less the ratio, positive on the lit side, for
  each letter box returned. `is_lit` agrees with the truth's letter on every row, and every sign stored equals the truth's. `vision`'s
  rows are `raw`'s, less `IMG_2894`'s. No box here sits near `LIT_MIN_BRIGHTNESS`' 120: the lowest `min(r, g)` is 143.63.

  | id | letter (per the truth) | `raw` | `lab` |
  |---|---|---|---|
  | `IMG_2889` | `L` unlit | 0.976 (−0.276) | 0.976 (−0.276) |
  | | `R` lit | not returned | 0.397 (+0.303) |
  | | `D` lit | 0.189 (+0.511) | 0.353 (+0.347) |
  | | `F` unlit | 0.943 (−0.243) | 0.950 (−0.250) |
  | `IMG_2891` | `R` unlit | 0.968 (−0.268) | 0.960 (−0.260) |
  | | `D` lit | not returned | 0.336 (+0.364) |
  | | `F` unlit | 0.951 (−0.251) | not returned |
  | `IMG_2892` | `R` lit | 0.096 (+0.604) | 0.365 (+0.335) |
  | | `D` lit | 0.172 (+0.528) | not returned |
  | | `F` unlit | 0.904 (−0.204) | not returned |
  | `IMG_2893` | `R` lit | 0.181 (+0.519) | not returned |
  | | `F` unlit | not returned | 0.893 (−0.193) |
  | `IMG_2894` | `L` unlit | 0.914 (−0.214) | not returned |
  | | `R` lit | 0.169 (+0.531) | 0.370 (+0.330) |
  | | `D` lit | 0.127 (+0.573) | not returned |
  | | `F` unlit | 0.833 (−0.133) | 0.897 (−0.197) |
  | `2026-10-07-1` | `R` lit | 0.363 (+0.337) | 0.426 (+0.274) |
  | | `D` unlit | 0.994 (−0.294) | 1.007 (−0.307) |
  | | `F` lit | not returned | 0.482 (+0.218) |

  The ranges: `raw` lit 0.096–0.363 and unlit 0.833–0.994, `lab` lit 0.336–0.482 and unlit 0.893–1.007. So the smallest margins are
  `raw`'s +0.337 and −0.133, and `lab`'s +0.218 and −0.193. **Letters returned**, of the 24 printed: 15 on `raw`, 13 on `lab`. **Signs**,
  of 12: on `raw`, 7 from a lit letter seen, 4 from an unlit letter alone, 1 withheld; on `lab`, 7, 3 and 2.

- **What earlier phases asked this one to look at.** P6's prediction held: on the five fallbacks `vision`'s boxes and shot equal `raw`'s
  and the only difference is the uncropped note in `warnings`. Its ceiling of 60 is 59 here, since `IMG_2893`'s `sidespin` is lost on
  both. P3's look held on `raw`: six of its seven lit letters are under M42's 0.32, and the seventh, on the JPEG original
  `2026-10-07-1`, is 0.363. P7's lit `R` read as `L` did not occur. P2's padding: no miss on `raw` is geometric.
- **A fault in the harness, found by the first read, and fixed before any table printed.** The first `measure` stopped with `KeyError:
  'sidespin'` on `raw`'s `IMG_2893`. `ShotData` omits `backspin` and `sidespin` when `None` (`skip_serializing_if`,
  `crates/contracts/src/shot.rs`), and `classify` and `lit_rows` indexed them. P7's controls never withheld either. An absent key is
  call 8's **withheld**, so `probe.py` gained `stored`, which allows absence on those two keys alone, and `control` gained `D` and `F`
  removed, the `9899` box removed, and the lit table on a shot with no `sidespin` key. Removing either key from the allowance, or the
  allowance itself, makes `control` raise, and any other absent key still raises. No bar, value or classification changed. The stopped
  attempt's 8 files were byte-identical to the first full run's, and `frames/runs/C0/` was cleared so that `repeat` could say `first`
  honestly.
- **For P12**: **stills pass at C0, on `raw`.** They do not pass on `vision`. The target for this run is stills alone (the decision's
  table), so `results/C0.json` is what its "not needed" reads.
- **For P13, P14 and P15**: the phone path passing is `raw`, with one cell not read, and that value's letters are not in Vision's boxes,
  so call 11's check 1 would rule it the recognizer's. P12 names the final config and these read their condition from it.
- **For P16**: on `raw`, 248 of 251 boxes are at 1.00, and the other three are 0.30, all strays: `DALI` (`IMG_2889`), and `AII` and
  `*ТАМАНA` (`IMG_2891`). `AII` landed in `Ball Speed Mph`'s tile, and `IMG_2891`'s 0.994 is the only `parse_confidence` under 1.0 on `raw`.
  On `lab`, all 237 are at 1.00, and the one `parse_confidence` under 1.0 is the caught shot's 0.542. Finding 7's near-constant holds here.
- **For P17**: facts the verdict's text needs, with no reading put on them here.
  - `raw` is **at** T3's limit: `sidespin` read on 5 of 6, so one more lost sign on any photo misses that bar. T2 has 5 to spare.
  - `raw` reads more than `lab` on these copies (71 to 68), and P5's caveat applies: `raw` gives Vision 2576 where the phone holds 5712,
    while `lab` shrinks to 1600 either way.
  - `vision`'s miss is P6's: five fallbacks and one replay panel. It says nothing about VisionKit's scanner.
  - Stray boxes reached stored tiles on `raw` and each still gave the right value: `SHOT DATA` under `Apex Ft` on four photos, `AII`
    and `14` under `Ball Speed Mph`. Boxes are in `boxes/C0/`.
  - Choices under rule 1, none touching `thresholds.md`: `stored` and its three controls, above. `boxes/C0/<mode>/<id>.json` holds the boxes
    files only, by call 1's layout, where P7's finding says to copy the directory, since `parse-screen` regenerates each shot from them.
    `results/C0.json` is the second run's, since its `repeat: same` is the record that they matched. One look beyond call 16, `IMG_2893`'s
    sidespin tile on `raw`, because it is the passing arm's one miss, and its crop was scratch and is deleted.
- **For P19**: `boxes/C0/` (18 files) and `results/C0.json` are in the tree and not ignored, for its commit (rule 13).
- **For later rows, when the originals reach this Mac**: another `measure --config C0 --set fs-still` overwrites `results/C0.json`'s arms
  and `frames/runs/C0/`, and these copies' `boxes/C0/` only if copied over. P19's commit is what keeps this measurement.

### P10 (2026-11-04) — not run: the five clips are still not on this Mac

Run under the decision of 2026-11-04, whose table gives P10 "Not run: no clips", so nothing was decoded, read or measured, and
there is no table to set beside the lab's 60.

- **The proof.** `probe.py check` exits 0 with 6 of 6 captures matching, prints all five `data/raw/shot_screens/IMG_n.MOV` as
  absent, and ends "the clips 0 of 5". `data/` holds only its `README.md`, `data/raw/` does not exist, and `git ls-files` tracks no
  `.MOV`. `measure --config C0 --set fs-clip` exits 1 before reading anything: `STOP: photos.json holds no capture of set 'fs-clip'
  (the decision of 2026-11-04)`.
- **A search of `/` for `IMG_2889.MOV` is not evidence either way.** It ran for about three and a half minutes, printed nothing and
  was killed unfinished, as P4's wider search was. The manifest's paths are the only place rule 9 would accept the clips from, so
  a later phase has no reason to run it again.
- **Nothing moved.** `results/C0.json` is as P9 left it: its `sets` holds `fs-still` alone, and it was last written before this
  phase started. `thresholds.md`'s digest still equals the one it records, `frames/grid/` does not exist, and the rule 5 diff is
  empty.
- **For P12**: whether clips pass is **not measured**. `results/C0.json` has no `fs-clip` key, and that is not a miss: the decision's
  table makes this run's target stills alone, so P9's "stills pass at C0, on `raw`" is all its condition reads.
- **For P17**: the clips' verdict is **not measured**, and P10 contributes no number to it. Finding 10's 60 of 60 stays the lab's, on
  the originals, with nothing here beside it. `probe.py` has no `verdict` yet, and call 9's clip column ends "**MISS** otherwise",
  which tells M38 to ship the still import first. So `verdict` must print **not measured** on an absent `fs-clip` key, and not fall
  through to **MISS**, which would state a result nobody measured.
- **For later rows, when the clips reach this Mac**: `measure` stops until all of these exist, so this phase as written follows them.
  P4's row (`grid-frame`, `frames/grid/`, and its comparison against M42 P14's frames), `fs-clip` entries in `photos.json`, and the
  clip's modes in `probe.py` (P5's `lab_input`, and `measure`'s comment on the same gap). The lab's column also needs P8's models.

### P11 (2026-11-04) — not run: the user omitted HD Golf, and its photos are not on this Mac

Run under the decision of 2026-11-04, whose table gives P11 "Not run: omitted by the user", so nothing was decoded, read or
measured. There are no totals, no flagged count and no differences by photo or field to report, in any mode.

- **Whose call.** P4, P8 and P10 rest on that section's reading of "proceed". This one rests on the user's words, "Omit HD golf
  for now". The absence below is a second reason beside it, and the section's closing paragraph still owes HD Golf a new row
  when the files arrive.
- **The proof.** `probe.py check` exits 0 with 6 of 6 captures matching, and prints "HD Golf: 0 of 15 present" with each path
  absent. A separate walk of `input.image_path` over `spec/vectors/screen/corpus/` and `reference/` gives the same 0 of 15. `data/`
  holds only its `README.md`, neither `data/raw/` nor `data/processed/` exists, and `git ls-files data` tracks that README alone.
  `photos.json` holds `fs-still` entries alone. `measure --config C0 --set hd` exits 1 before reading anything: `STOP: photos.json
  holds no capture of set 'hd' (the decision of 2026-11-04)`.
- **Nothing moved.** The 55 files under `results/`, `boxes/` and `frames/runs/` hash the same before and after this phase.
  `results/C0.json`'s `sets` holds `fs-still` alone, `thresholds.md`'s digest equals the one it records, the rule 5 diff is empty,
  and `probe.py control` exits 0.
- **For P17**: HD Golf is **not measured**, so this phase's "open item for M38" has no differences to list, and the item is all of
  it: what Vision makes of that screen, in every mode, is unknown. It has no bar and no verdict (decision 3, call 9), so it moves
  the verdict neither way. `verdict` must not look for a `sets.hd` key, and "What this does not show" names HD Golf, as the
  decision's table says. Say it as the user's omission, since the clips and models are absences the user did not choose. Two
  things read like evidence here and are not, so quote neither as Vision on HD Golf:
  - P7's control 1, "all 15, 168 read", is the committed vectors' own boxes, and it proves the comparator. Vision is not in it.
  - Finding 7's "42 boxes in, 42 out" on the HD Golf layout was a synthetic render, as that finding says.

  ADR-034's `Impact Position V` hazard on `2026-08-10-1` and `2026-08-23-1` is therefore **unanswered for Vision**, and `DECLARED`'s
  one entry has not met a Vision read.
- **For later rows, when the photos reach this Mac**: this section as written. Read against `probe.py`, `measure --set hd`
  needs all of these, beyond P7's note that no `measure` path calls `classify_vector`:
  - **`hd` entries in `photos.json`.** `manifest` writes `fs-still` alone. Path and digest come from each vector, and each
    `input.image_sha256` is the whole 64 hex, so rule 9 holds as it stands (the 12 in the file names are only a prefix).
  - **A way past the second stop.** `read_bars` knows only `fs-still` and `fs-clip`, and a set not in it stops `measure` right after the
    manifest. `hd` has no bars, so the fix is a path that skips `judge`, **never** a row in `thresholds.md` (rule 2).
  - **The vector's echoes in the input.** `screen_input` stamps `session_id` `m33` and the fixed `TIMESTAMP`, and `classify_vector`
    refuses a difference in `ECHOED` as the harness's fault, so the first read would stop there. Call 8 already says HD Golf's
    timestamp is the vector's, and `shot_id` and `session_id` need the same.
  - **The images.** `mode_image` and `prepare` know the stills only. `2026-08-23-1`, one of ADR-034's two, is the set's one HEIC, so
    it decodes through ImageIO (rule 6) and needs `lab_input`'s unwritten route from `frames/decoded/`, which makes its `lab` image
    not the lab's exact pixels (call 2). P5's HD Golf clause, the quad against each vector's `input.notes`, runs there too.

### P12 (2026-11-04) — not needed: C0 meets this run's target, so the final config is C0

Read under the decision of 2026-11-04, so **provisional: pasted copies**. Its table makes this run's target stills alone, stills passing
on `raw` or `vision`, and `results/C0.json` has them passing on `raw`. So the condition is not met, no rung ran, and no code was written.

- **One line per config measured**, of call 10's bound of eight, each cell read / wrong / flagged and the arm:

  | config | set | `raw` | `vision` | `lab` |
  |---|---|---|---|---|
  | **C0** | `fs-still` | 71 / 0 / 0 **PASS** | 59 / 0 / 1 **MISS** | 68 / 0 / 1 **PASS** |
  | **C0** | `fs-clip` | not measured | not measured | not measured |

- **The final config is C0**, by call 10's "the first that meets the target". It was measured first, so there was no best to compare.
- **The proof, beyond reading P9's table.** `probe.py check` exits 0 with 6 of 6 matching, and `control` holds. A throwaway run over all 18
  arm-and-photo pairs, with the plan's table typed afresh, and neither `truth()` nor `classify()` in the path, recounted what `results/C0.json`
  records in every total, the lowest field and all four bars. Limits typed from call 9 agree with the ones parsed into the file. On the same 18:
  a fresh `vision-ocr read` is byte-identical to `boxes/C0/`, so this is a third matching read; `boxes/C0/` equals `frames/runs/C0/`
  and each digest in the results; and `parse-screen` on the committed boxes equals each stored shot. **Nothing moved**: the 56 files
  of `results/`, `boxes/`, `frames/runs/` and `thresholds.md` hash the same before and after, `thresholds.md`'s digest equals the
  one recorded and `HEAD`'s, and the rule 5 diff is empty. `measure` was not re-run, since it would rewrite `repeat`, P9's record that
  two runs matched. The script is gone, and so is the `__pycache__` its import left (rule 14).
- **`boxes/C0/` is already the final config's boxes**, so "copy" had nothing to do.
- **For P13**: the final config is **C0**. On stills, call 11's condition does not hold, since `raw` passes. On clips it needs a miss on
  both phone modes, and they are not measured, which P10 says is not a miss. `raw`'s one unread cell is P9's `IMG_2893` `sidespin`, and
  in `boxes/C0/raw/IMG_2893.json` the tile has `SIDESPIN RPM` and `638`, and no box on the photo is `D` or `F`.
- **For P16**: "the final config's results" are `results/C0.json`, `fs-still` only.
- **For P17**: three things the verdict's text needs.
  - The verdict is reached at C0, so it carries neither **tuned** label, and **provisional: pasted copies** is what it carries.
  - **"Not needed" rests on the decision, not on `thresholds.md`.** That file's target is stills **and** clips, so with clips unmeasured,
    its own test of whether the ladder runs has not been answered. It is C0's standing as final on this run that is provisional, for the
    same reason the verdict is. Say so rather than letting "the ladder was not needed" read as a result on clips.
  - **C1 to C4 are unmeasured**, so nothing here says whether any would return `IMG_2893`'s letters, or lift `raw` off T3's limit, or
    whether any would add a **wrong** cell. Neither "the ladder would not help" nor "C0 is the best setting" is a finding of this run.
- **For later rows, when the clips reach this Mac**: the target goes back to stills and clips, and if C0 misses on clips this phase runs as
  written, on what P10 lists first. **None of the ladder exists**: `vision-ocr read --config C1` still exits with `config C1 is not built
  yet`, `+` is not parsed, and `probe.py` has no `ladder`. P3's and P7's notes for this phase still hold for whoever builds it: `glyphRGB`
  needs no change, and `measure` already passes `--config`. On the originals, P9's own re-measure may change the stills' answer, so read it first.

### P13 (2026-11-04) — not needed: what is missing is the recognizer's

Read under the decision of 2026-11-04, so **provisional: pasted copies**, and its table's steps 1 and 2 are what ran. No rule was written, so
the read counts stand at P9's (`raw` 71, `vision` 59). Nothing under `crates/` or `spec/` moved, the rule 5 diff is empty, and `SCREEN_PARSER_VERSION`
and `thresholds.md` are untouched. `probe.py` gained `failures` and a control 6, and that is all this phase changed.

- **The condition does not hold.** `probe.py failures --config C0` judges each arm again from its cells against the bars parsed out of
  `thresholds.md`, rather than taking `pass` from the file: on `fs-still`, `raw` PASS, `vision` MISS, `lab` PASS. So there is no miss on both phone
  modes. `fs-clip` prints "not measured, which is not a miss".
- **Steps 1 and 2: parser-fixable on `raw` and `vision`, 0 of the 14 cells not read.**

  | mode | id | field | cell | printed | in Vision's boxes | what that makes it |
  |---|---|---|---|---|---|---|
  | `raw`, `vision` | `IMG_2893` | `sidespin` | withheld | `638 D` | `638` in its tile, and neither `D` nor `F` on the photo | recognizer's: call 11's check 3 lets nothing but the lit letters sign it |
  | `vision` | `IMG_2894` | all twelve | withheld, FAILED | the row | 0 boxes | recognizer's by call 11's wording, "a value Vision never returned". The cause is the image: it is the replay panel (P6), and `raw` reads all twelve on this photo |

  `IMG_2893` is counted twice because `vision` falls back to `raw` there (P6). The one box near a letter is `DD` at 2048,1168, far outside the
  tile and no sign letter, and `whose` does not count it.
- **How item 1 was read, in `whose`.** A number anywhere on the photo counts, not only under its label, because a garbled label leaves no tile to look in.
  So it errs towards "fixable", and each row prints `in_tile` and where the box sits, to be read rather than trusted. A signed cell also needs a
  letter of its pair, as a token of its own (`D`, `638 D`, `638D`, not `DD`), with an `rgb`, since a letter decides by its colour and the host measures that.
- **`lab`, not a target arm, has two that are fixable** (`failures --config C0 --mode lab`, 2 of 4): `2026-10-07-1`'s `apex_height` and `smash_factor`, one cause.
  Vision's `APEX RI` matches no label, so `142.2` falls into the next tile as P9 recorded, with `1.43` beside it. Call 10's target and call 11's condition name
  only the phone modes, `lab` passes anyway, and nothing was fitted to it. `lab`'s other two, `IMG_2892` `sidespin` and `IMG_2893` `launch_direction`,
  have their number and no letter of the pair, as `IMG_2893` does on `raw`.
- **The proof.** `check` exits 0 with 6 of 6 matching. `control` holds, with control 6 new: seven mutations of known cause on the grid's boxes, each landing
  on the side named, and four on the helpers. A throwaway harness put 12 faults into `whose` and its helpers, and each made `control` fail. It proved six
  stops on scratch copies, each exiting non-zero: no results file, a different `thresholds.md`, a boxes file or a shot file not matching `results/`, a `pass`
  flipped, a run file gone. It drove three paths no real cell reaches: a `D` handed to `IMG_2893` turning that cell PARSER-FIXABLE through the whole listing, the
  condition reading "holds" with both phone modes missing, and a synthetic `fs-clip` set reading through. Harness and scratch are gone (rule 14).
- **`failures` writes nothing and runs no Vision**, since a re-read would rewrite `repeat`. A sorted digest of the 56 files of `results/`, `boxes/`, `frames/runs/` and
  `thresholds.md` is the same before and after a last `check`, `control` and both `failures`. All 36 boxes files equal `results/C0.json`'s digests, and none of the
  56 has an mtime or ctime after this phase's marker. (An unsorted digest of the same 56 differed between runs with no file touched, so only the sorted one is evidence.)
- **The rule-side steps did not run.** No failing vector, `lab-regress`, or the Verify block's whole-workspace lines: they guard a rule, and with the rule 5 diff empty
  they have nothing to say. P19 runs its own. This phase's other **Read first** files are a rule's reading, and were not read.
- **For P14 and P15**: tick the same way, by P13's step 2. No rule fired, so `results/C0.json` has not moved, and `failures --config C0` is the one command each needs. There is no
  group to take next.
- **For P16**: nothing here changes its inputs.
- **For P17**: facts the verdict's text needs, with no reading put on them here.
  - No parser rule fired, so the verdict does not carry **tuned, parser**.
  - **"Not needed" rests on the decision, as P12's did.** It answers on stills. Call 11's condition on clips is unanswered, so no rule's being needed there is not a result of this run.
  - `raw`'s T3 limit (`sidespin` on 5 of 6, P9) rests on Vision returning a letter. The one cell it loses is not one a parser may recover.
  - **Measured on a mutation, not a photo**: with `SMASHFACTOR` garbled on the grid's boxes, `1.08` falls into `Backspin Rpm`'s tile, `backspin` stores `1.08`, and that is **wrong on an unflagged
    shot**, with the warning `no tile found for 'Smash Factor'` and no flag. It is P9's `APEX RI` spill on another tile, which was caught there only because `142.2` fails the speed cross-check and the range check (P9). It sits beside
    P7's lit `R` read as `L`, as a route to a **wrong** cell that no arm here measured. No arm measured has a **wrong** cell.
  - Choices under rule 1, none touching `thresholds.md`: `failures` reads `frames/runs/` as P7 said, holding each file to `results/`. Its default arms are the phone modes, with `--mode lab` the diagnostic.
    It prints, and adds no results file, so call 1's layout is unchanged. Its output says `not fixable` where the plan says "the recognizer's", because of `IMG_2894`'s twelve. Control 6 is beyond the plan's list.
- **For P18**: no rule, so nothing from this phase goes to ADR-014 or CONFORMANCE. It may name `probe.py failures` with the spike's other subcommands.
- **For P19**: no rule fired in this phase. Unless P14 or P15 says otherwise, the Verify block's "if a rule fired" lines (`v17` and `career-v3`, `tests/test_conformance.py`, `conformance.py check`) and
  call 15's Windows gate do not apply. `probe.py`'s change is in the tree for its commit (rule 13).
- **For later rows, when the files reach this Mac**: P9's re-measure may move this answer, so run `failures` again, and if the condition then holds this phase runs as written. Three things will be in its way. `lab-regress`
  is still not written (P8). `failures` reads `frames/runs/<config>/<mode>/<id>`, which `run_arm` keys by id alone, so a clip frame and the still of the same id would overwrite each other until that row separates them. And the
  plan's "failures" lists on `fs-clip` only through the synthetic set above, so no real clip has been through it.

### P14 (2026-11-04) — not needed: there is no second group, because nothing on the phone modes is parser-fixable

Read under the decision of 2026-11-04, so **provisional: pasted copies**, and its table's steps 1 and 2 are what ran. No rule was written, so the read counts stand at P9's
(`raw` 71, `vision` 59). P13 wrote and withdrew nothing, so there was no group to pass over. Nothing under `crates/` or `spec/` moved, and `SCREEN_PARSER_VERSION`, `thresholds.md` and `probe.py`
are untouched. This row and this entry are all the phase changed.

- **The condition does not hold.** `probe.py failures --config C0`, on `fs-still`: `raw` PASS, `vision` MISS, `lab` PASS. So there is no miss on both phone modes. `fs-clip` prints "not measured, which is not a miss".
- **Steps 1 and 2: parser-fixable on `raw` and `vision`, 0 of the 14 cells not read**, and they are P13's cells: `IMG_2893` `sidespin` on both modes, and `vision`'s twelve on `IMG_2894`.
- **Those two causes, checked on the boxes files rather than through `whose`**, with a one-off pattern for a lone `D` or `F`. `IMG_2893` on `raw` and on `vision` holds 39 boxes each, with `638` present and `DD` at 2048,1168
  the only match, which `whose` rightly refuses as a letter. `IMG_2894` on `vision` holds 0 boxes. The same pattern finds `D` at 1243,1423 and `F` at 1505,1423 on `IMG_2894`'s `raw`, so a pair that is there does show up.
- **Nothing moved.** A sorted digest of the 56 files of `results/`, `boxes/`, `frames/runs/` and `thresholds.md` is the same before and after `check`, `control` and both `failures`. None of them, nor `probe.py`, has an mtime after this
  phase's marker. The rule 5 diff is empty, and `thresholds.md` has no diff against `HEAD`. `check` matches 6 of 6, and `control` prints all held.
- **The rule-side steps did not run**, as at P13: they guard a rule, and there is none. `failures --config C0 --mode lab` still names `2026-10-07-1`'s `apex_height` and `smash_factor` as fixable, and `lab` is still not a target arm, so that is P13's diagnostic and not a group.
- **For P15**: tick the same way, by P13's step 2. `results/C0.json` has not moved, and `failures --config C0` is the one command it needs. There is no group to take next.
- **For P16**: nothing here changes its inputs.
- **For P17**: no new fact. P13's list stands as written, and no parser rule fired here, so the verdict still does not carry **tuned, parser**.
- **For P18**: no rule, so nothing from this phase goes to ADR-014 or CONFORMANCE.
- **For P19**: no rule fired in this phase, so P13's line on the "if a rule fired" commands and call 15's Windows gate still stands, unless P15 says otherwise.

### P15 (2026-11-04) — not needed: the cap, reached with no rule written, so what is missing is the measured result

Read under the decision of 2026-11-04, so **provisional: pasted copies**, and its table's steps 1 and 2 are what ran. No rule was written in P13, P14 or here, so the read counts stand at P9's (`raw` 71, `vision` 59). Nothing under
`crates/` or `spec/` moved, and `SCREEN_PARSER_VERSION` is still 1. `thresholds.md` and `probe.py` are untouched. This row and this entry are all the phase changed. The models are absent (`check`: 0 of 3), which rule 3 would stop on, and the decision's table is why it does not: steps 1 and 2 need neither models nor ffmpeg.

- **The condition does not hold.** `probe.py failures --config C0`, exit 0, on `fs-still`: `raw` PASS, `vision` MISS, `lab` PASS. So there is no miss on both phone modes. `fs-clip` prints "not measured, which is not a miss".
- **Steps 1 and 2: parser-fixable on `raw` and `vision`, 0 of the 14 cells not read**, the same cells as P13 and P14.
- **This is the cap, so these are the measured result on this run's stills:**

  | mode | read | not read | why, by call 11's check 1 |
  |---|---|---|---|
  | `raw` | 71 | `IMG_2893` `sidespin` | `638` is in its tile, and no box holds `D` or `F` |
  | `vision` | 59 | the same cell, and all twelve on `IMG_2894` | the same, and that image gave 0 boxes |

- **Those two causes, read straight off the boxes files rather than through `whose`.** `IMG_2893` holds 39 boxes on each of `raw` and `vision`, with `638` present and `DD` the only all-`D`-or-`F` box, which is no sign letter. `IMG_2894` on `vision` holds 0 boxes. The same read finds `D` and `F` on `IMG_2894`'s `raw`, so a pair that is there does show up.
- **Nothing moved.** The sorted digest of the 56 files of `results/`, `boxes/`, `frames/runs/` and `thresholds.md` is `2c18796f…82c5` before and after `check`, `control` and both `failures`, each of which exits 0, and none of the 56 has an mtime after this phase's marker. The only file under `crates`, `src`, `tests`, `scripts`, `spec`, `docs` or `spikes` newer than the marker is this plan. The rule 5 diff is empty, and `thresholds.md` has no diff against `HEAD`. `check` matches 6 of 6, and `control` prints all held.
- **The rule-side steps did not run**, as at P13 and P14: they guard a rule, and there is none. `failures --config C0 --mode lab` still names `2026-10-07-1`'s `apex_height` and `smash_factor` as fixable, 2 of `lab`'s 4 cells not read. `lab` is not a target arm, so that remains P13's diagnostic and not a group.
- **For P16**: nothing here changes its inputs. `results/C0.json` and the boxes are as P9 left them, and the digest above is there to hold them to.
- **For P17**: no new fact on top of P13's list, and two that close it. None of the three phases fired a rule, so the verdict does not carry **tuned, parser**. The table above is what §P15's "the measured result" means here, and it answers on stills only: it says nothing about the original files, the clips or HD Golf.
- **For P18**: no rule across P13-P15, so nothing from them goes to ADR-014 or CONFORMANCE, and §P18's "why no `SCREEN_PARSER_VERSION` bump" has no rule to explain.
- **For P19**: no rule fired in P13, P14 or P15, and this is the "unless" P13 and P14 left open, closed. The Verify block's "if a rule fired" lines (`v17` and `career-v3`, `tests/test_conformance.py`, `conformance.py check`) and call 15's Windows gate do not apply. `probe.py`'s change from P13 is still in the tree for its commit (rule 13).
- **For later rows, when the files reach this Mac**: P13's paragraph stands. No rule was written, so none of call 11's three is spent, and a re-measure that makes the condition hold would have all three to use.

### P16 (2026-11-04) — Vision's confidences change no flag, and the recommended `min_confidence` is 0.6 with nothing to catch

Measured under the decision of 2026-11-04, so **provisional: pasted copies**, and on `fs-still` only: `fs-clip` and `hd` are not measured, and there is no lab reader to compare with (P8), so item 2's "against the lab's" did not run. `lab` below is Vision on the lab's image, a diagnostic. The final config is C0 (P12). `probe.py calibrate --config C0` prints everything here and writes nothing. It reads the committed `boxes/C0/`, each held to `boxes_sha256`, and each parse at 0.6 is held to `results/C0.json`, so its 0.6 row is the bars' own.

**ADR-014's question, answered on these six: no.** At every threshold from 0.5 to 0.9, on every mode, the same shots flag, and neither flag comes from a confidence.

- **1. The histogram.** Two distinct values on the phone modes, and one on `lab`:

  | mode | boxes | distinct | at 0.30 | at 1.00 |
  |---|---|---|---|---|
  | `raw` | 251 | 2 | 3 | 248 |
  | `vision` | 208 | 2 | 3 | 205 |
  | `lab` | 237 | 1 | 0 | 237 |

  `vision`'s are `raw`'s less `IMG_2894`'s, because five of its images are fallbacks (P6). The 0.30 is Vision's `Float` as JSON, `0.30000001192092896`. The three boxes at 0.30 are `DALI` (`IMG_2889`), and `AII` and `*ТАМАНA` (`IMG_2891`), all strays and none a tile's label or value. **A low confidence is not what marks a misread, on this evidence**: the box behind the run's one differing cell, `APEX RI`, is at 1.00, as are `S DESPIN R`, `PYAMAHA` and `→YAMAHA`.

- **2. Each `parse_confidence` and what it averaged.** "coverage" is the same parse with every box at 0, so the two coverage terms less any penalty, and "OCR" is what that takes off the shot's own. "Averaged" is every box in a tile's cell, the tile's label and values and any stray that falls in the cell, so the table lists what was left out. `used` is printed nowhere, so `averaged_boxes` moves one box's confidence and watches `parse_confidence`, and `confidence_terms` stops unless `W_OCR` times the mean equals the term to the record's three decimals.

  | mode | id | boxes | averaged | conf | coverage | OCR | mean | not averaged |
  |---|---|---|---|---|---|---|---|---|
  | `raw` | `IMG_2889` | 42 | 33 | 1.000 | 0.700 | 0.300 | 1.0000 | `TEAM 1`, `AARON`, `SHOT DATA`, `Ev`, `1`, `11`, `DD`, `CONTINUE`, `DALI` |
  | `raw` | `IMG_2891` | 41 | 34 | **0.994** | 0.700 | **0.294** | 0.9794 | `TEAM 1`, `AARON`, `+2`, `1`, `CONTINUE`, `DELL`, `*ТАМАНA` |
  | `raw` | `IMG_2892` | 43 | 33 | 1.000 | 0.700 | 0.300 | 1.0000 | `TEAM 1`, `AARON`, `SHOT DATA`, `1`, `+2`, `44`, `DI`, `CONTINUE`, `DELL`, `PYAMAHA` |
  | `raw` | `IMG_2893` | 39 | 33 | 1.000 | 0.700 | 0.300 | 1.0000 | `TEAM 1`, `AARON`, `1`, `+3`, `DD`, `CONTINUE` |
  | `raw` | `IMG_2894` | 43 | 35 | 1.000 | 0.700 | 0.300 | 1.0000 | `TEAM 1`, `AARON`, `1`, `+2`, `DELL`, `14`, `CONTINUE`, `DD` |
  | `raw` | `2026-10-07-1` | 43 | 34 | 1.000 | 0.700 | 0.300 | 1.0000 | `TEAM 1`, `AARON`, `DELL`, `14`, `DD`, `CONTINUE`, `→YAMAHA`, `1`, `+9` |
  | `vision` | the five fallbacks | | | as `raw` | | | | as `raw` |
  | `vision` | `IMG_2894` | 0 | none | failed read | | | | |
  | `lab` | the other five | 37–42 | 31–34 | 1.000 | 0.700 | 0.300 | 1.0000 | `calibrate` prints each |
  | `lab` | `2026-10-07-1` | 41 | 32 | **0.542** | **0.242** | 0.300 | 1.0000 | `TEAM 1`, `AARON`, `APEX RI`, `SHOT DATA`, `1`, `+9`, `14`, `→`, `CONTINUE` |

  `AII` is the only box under 1.00 that was averaged: 33 at 1.00 and one at 0.30, of 34, is the 0.9794. **The OCR term is not strictly a constant, and is in effect one**: 0.300 on 9 of the 11 parsed phone-mode shots and 0.294 on the other two, which are one photo read twice. That 0.006 is a tenth of what one lost label costs (0.058). So on the phone `parse_confidence` is the coverage terms plus a flat +0.30, as finding 7 predicted on synthetic images. Setting every box to 0 and to 1 moved no value and no physics flag on any of the 17 parsed shots, so a confidence reaches the score and nothing else.

- **3. Why each flagged shot is flagged**, at 0.6. **From confidence alone: 0 of 2.** Physics is told apart by re-parsing at a threshold of 0, where only a failed check can flag, since a confidence is clamped at 0.

  | mode | shot | cause |
  |---|---|---|
  | `raw` | none | |
  | `vision` | `IMG_2894` | failed read: 0 boxes, the replay panel (P6), so no check ran |
  | `lab` | `2026-10-07-1` | physics: the speed cross-check (−0.25) and the range check (−0.15) |

  That shot is 0.35·11/12 + 0.35·11/12 + 0.30 − 0.40 = 0.542, so it is also under 0.6, and only because of the penalties. Before them it is 0.942, over every threshold here (arithmetic). `IMG_2893`'s withheld `sidespin` stays 1.000 and unflagged on `raw`, as ADR-014's M42 addendum says of a rule that withholds.

- **4. The what-if**, each threshold re-parsed through `golf-core` and every cell classified again. **All five rows are the same**, so they are one row here:

  | `min_confidence` | `raw`: read, flagged | `vision`: read, flagged | `lab`: read, flagged | **wrong**, any mode | **caught** |
  |---|---|---|---|---|---|
  | 0.5, 0.6, 0.7, 0.8, 0.9 | 71, none | 59, `IMG_2894` (failed read) | 68, `2026-10-07-1` (physics) | **0** | 1: `lab`'s `2026-10-07-1` `smash_factor` |

  **There is no wrong cell, so none could be caught or missed.** The one cell that differs is on `lab`, and physics catches it at every threshold, 0.5 included. A throwaway recount, with none of `probe.py` in its path, gave the same histogram and the same flagged shots at all five and at 0, and is deleted (rule 14).

- **Beside it, a mutation and not a photo: P13's route to a wrong cell**, on the committed grid vector's boxes. With `SMASHFACTOR` garbled, `backspin` stores `1.08` at `parse_confidence` 0.941 (0.942 with every box at 1.00). It is **wrong and unflagged at every threshold up to 0.9**, and caught at 0.95, which is not among §P16's four. **By arithmetic, not measured on photos** (`score`'s weights, every box at 1.00): the labels of twelve a shot must lose to flag are 9 at 0.5, 7 at 0.6, 6 at 0.7, 4 at 0.8 and 2 at 0.9. A shot with every label and value is 0.70 with every box at 0, so at 0.6 or 0.7 no confidence Vision can report flags it.

- **The recommendation: `min_confidence` 0.6, provisional.** By §P16's rule, all four of 0.6, 0.7, 0.8 and 0.9 hold on both phone modes, and 0.6 is the lowest. **It holds vacuously.** No phone-mode cell differs from the truth at any threshold, so this does not show that 0.6 catches anything, and it does not rank the four. On these six a higher value would cost nothing and catch nothing. `ScreenInput.min_confidence` is the caller's, so this changes no code and no vector. **`W_OCR` is named for M38 and not changed here**: moving it would move every screen vector, and on this evidence its 0.30 is a flat credit rather than a signal.
- **A control's own fault, found on its first run and fixed.** Control 7's arithmetic clause asked at the rounded 0.942, which is already over the unrounded 0.94167, so it failed. It now asks just either side of the sum, through the binary too. No result depended on it. `calibrate`'s tables after the fix agree with its first run, compared by eye because that run's output was not kept, and the fix added two printed lines and a stop. Two later runs were `diff`ed and are identical.
- **Nothing moved.** The sorted digest of the 56 files of `results/`, `boxes/`, `frames/runs/` and `thresholds.md` (`shasum -a 256` of each, sorted, then digested: `4396a7eb…ee32`) is the same before and after every run here, and none of the 56 has an mtime after this phase's marker: the newest is 11:54:08, and the marker is 12:16. **This does not reproduce P15's `2c18796f…82c5`.** P15 did not record its spelling and five tried here gave other digests, so that digest holds nothing here, and the mtimes and each boxes file's `boxes_sha256` are what tie these inputs to P9's. The rule 5 diff is empty, and `thresholds.md` has no diff against `HEAD`. `check` matches 6 of 6, and `control` prints all held.
- **Resumed once.** The session was cut off after this entry and the tick were written, before it reported. The resumption checked the tree against this entry, corrected the two statements above, and measured nothing new.
- **For P17**: the four items and the value above are its input, and **the verdict stays as measured at 0.6** (call 9). Facts its text needs, with no reading put on them here:
  - Say 0.6 is **provisional and vacuous**. It is the rule's floor on an empty set, and six stills gave no **wrong** cell to test it on. "0.6 is enough" is not a finding of this run.
  - On these six, what flagged a shot was a failed read or a physics check. Confidence alone flagged nothing, and at 0.6 or 0.7 it cannot flag a shot with every label and value.
  - No threshold offered catches P13's mutation, and `no tile found` raises no flag by itself, so for that route `needs_review` rests on a physics check happening to fail. It is a mutation, so it stays labelled as one.
  - For M38's list: `W_OCR` is the open question, with the two options being to leave a flat +0.30 in the score or to change the weight and re-record the screen family, and the choice is M38's. Boxes in a hand vector carry Vision's confidence as `0.30000001192092896`. **All of this is C0 only**: `usesLanguageCorrection` is off, and C3, which turns it on, is unmeasured on photos (finding 7's 0.3, 0.5 and 1.0 were synthetic).
  - Choices under rule 1, none touching `thresholds.md`. `calibrate` prints and adds no results file, as `failures` does, so call 1's layout is unchanged. It prints `lab` as a diagnostic, and the rule reads only the phone modes. "Without taking stills past T4's ceiling" is read per phone mode, each on its own, and it does not bind here. The 0.95 row and the two arithmetic lines are beyond §P16's list, each labelled, as is control 7.
- **For P18**: ADR-014's clause "M33 re-checks that term's weight and the review threshold" now has its answer here: the term is in effect a constant, the threshold changes nothing on these six, the recommended value is 0.6 and provisional, and the weight is left for M38. All of it on pasted copies at C0.
- **For P19**: `probe.py`'s `calibrate` and control 7 are in the tree for its commit (rule 13). P16 changed nothing else outside this plan.
- **For later rows, when the files reach this Mac**: run `calibrate` again after P9's re-measure, since the originals are the images that matter and a new read may hold other confidences. It stops on purpose if `results/` holds `fs-clip` or `hd`: a clip's id is its still's, so `boxes/<config>/<mode>/<id>.json` must tell them apart first, as P13 said of `frames/runs/`. Item 2's lab column needs P8's `lab-boxes/`, and a **wrong** cell on the originals would be the first real test of §P16's rule.

### P17 (2026-11-04) — GO, provisional: pasted copies; clips not measured, and `verdict` holds the log to it

- **The verdict line, as `probe.py verdict` prints it and `log.md` states it**: `**Verdict: GO, provisional: pasted copies. Clips: not measured.**`. Reached at C0 with no rule, so neither **tuned** label. It rests on `raw` alone (P6), which is at T3's limit (P9), and `log.md` says at once that this is not yet the answer on the photos as captured, and that M33 stays open.
- **What landed**: `spikes/vision-ocr/log.md`, and in `probe.py` the subcommand `verdict` and a control 8. Nothing else in the tree moved: the rule 5 diff is empty, `thresholds.md` has no diff against `HEAD`, and the sorted digest of the 56 files of `results/`, `boxes/`, `frames/runs/` and `thresholds.md` is P16's `4396a7eb…ee32`, by P16's spelling, with none of them newer than this phase's marker. No Vision ran, because a re-read would rewrite `repeat`.
- **`verdict` works the answer again rather than reading it.** For each arm it rebuilds each cell through `classify` from the cell's value, the flag and `truth()`, then `summarize` and `judge` against the bars parsed from `thresholds.md`, and it stops if a `cell`, `truth`, `flagged`, `summary`, `bars`, `pass` or limit in the file disagrees. An absent `fs-clip` prints **not measured** (P10), `sets.hd` is never opened (P11), and the label is the file's own, as P7 asked. It then checks the log: the one `**Verdict:` line and all 13 table lines must be whole lines of `log.md`, or it exits 1. A NO-GO would exit 0.
- **The proof.** `check` matches 6 of 6, and `control` holds, with control 8 new: all eight rows of call 9's table on both sets, the two labels, a wrong and a caught cell, 14 edits that must stop it, and six logs. A throwaway harness put 32 faults into the verdict's code and `judge`, and all 32 made control 8 fail. Its first run had one survivor, the shot-count stop, because deleting a shot is already caught by the summary, and that is now tried on a record true to itself with six shots against five. 15 cases on a scratch tree each gave the exit and the message named: no results, a second config, a file not named for its config, a different `thresholds.md`, another `min_confidence`, a boxes file changed or gone, a `pass` flipped, no `fs-still`, the label removed, a `vision-*.json` on disk reading **tuned, parser**, the log gone, and the log saying otherwise. A separate recount, with the plan's table typed afresh and none of `probe.py` in its path, gave the same bars and GO. The same throwaway checked all 28 lit values in `log.md`'s table, and each "not returned", against `results/C0.json`, with 0 differing. Two `verdict` runs print the same bytes. `tests/test_docs_truth.py`: 91 passed, with `log.md` untracked. The scripts are deleted (rule 14).
- **A control's own fault, found on its first run and fixed.** `edit_failed` turned a failed read into a parse, which is the same cells by call 8's own words and moves no bar, so nothing stopped and nothing should. The control now asks the other direction, a parsed shot called a failed read, which does change the cells.
- **Nothing addressed to this phase contradicted the plan, and no bar looked wrong** (rule 2). `vision`'s T2 being out of reach is its input (P6), not the bar. Call 16's one crop agrees with the truth table, so the log says nothing against the table.
- **Choices under rule 1**, none touching `thresholds.md`, and all in `log.md`'s last section beside the earlier phases': `verdict` goes back to values where §P17 asks only for a recomputed table; it holds `boxes/C0/` to `results/`; it stops on a missing arm, because "passes on none" with one unmeasured would be a NO-GO nobody measured; it stops on a second config, because call 10's choice among configs is not written (the ladder never ran); and it reads `spec/vectors/screen/hand/vision-*.json` as "a rule fired", since call 11's item 4 is what brings one. Control 8 is beyond the plan's list. `log.md` has two sections §P17 does not list, on the rectifier and on the confidences, because P6 and P16 each told it to say those things.
- **For P18**: every doc carries the verdict with its label and the list of what is unmeasured: the original files, the clips, HD Golf (the user's omission) and the lab baseline (the clips and models are absences the user did not choose). `log.md`'s "What this does not show" is that list, and its "For M38" (twelve numbered items) is what §M38's bullets are changed from. Do not edit `log.md`'s verdict line or its two tables by hand, because `verdict` exits 1 if they differ. Do not write that M38 is unblocked: that is the user's.
- **For P19**: `log.md` and `probe.py` are in the tree for its commit (rule 13). Add `probe.py verdict` to its checks, exit 0 expected. Its report and the commit title use this spelling of the verdict, and the title carries "provisional".
- **For later rows, when the files reach this Mac**: re-measuring on the originals makes `measure` set `label` to `None`, so `verdict`'s line changes and it exits 1 until `log.md` is rewritten, which is on purpose. A second config in `results/` stops it until call 10's choice is written. `fs-clip`'s arms already reckon, and their boxes are not held, since `boxes/` cannot yet tell a clip from its still (P16).

### P18 (2026-11-04) — the docs say the verdict with its label, and none says M38 is unblocked

- **What landed.** An M33 addendum each in `docs/decisions/034-shot-first-phone-first.md` (the verdict and its table; Full Swing decides, HD Golf is diagnostic, the parser is Rust's; the "biggest unknown"; what is unmeasured) and `docs/decisions/014-screen-capture-shot-ingestion.md` (Vision as the recognizer; the answer P16 addressed here, that the term is in effect a constant, the threshold changes nothing, the recommended 0.6 is provisional and vacuous, and the weight is M38's; no rule). `docs/README.md`: those two rows, and the addenda total, which docs-truth owns, from 120 to 122. `docs/CONFORMANCE.md`: the two sentences. `docs/plans/m31-m40-shot-first-pivot.md`: the checklist row, a note heading §M33, and §M38 from `log.md`'s twelve items, each marked *(M33)* under a note saying that label and C0 apply and that unblocking is the user's. `spikes/README.md`: the entry. `CLAUDE.md`: one comment in the Commands block.
- **Verified.** `tests/test_docs_truth.py`: 91 passed. `probe.py verdict` exits 0, and `log.md` was not edited. `git diff --stat -- crates src scripts tests spec` is empty, and `thresholds.md` has no diff. Each new cross-link's target exists, with the anchors worked by GitHub's slug rule by a throwaway, deleted (rule 14).
- **What §P18 made conditional, and did not fire.** No rule (P13-P15), so CONFORMANCE §2's screen count and `hand/` row, CLAUDE.md's `crates/screen` row and `crates/screen/`'s module docs are untouched, and ADR-014's section on `SCREEN_PARSER_VERSION` says there is no bump to explain. **CLAUDE.md gets no clause on M33**: §P18 asks for one only "if the verdict changes" ADR-034's route, and a GO does not, and the file holds no status.
- **Choices under rule 1**, none touching `thresholds.md`, each the smaller edit:
  - **§P18's Files line is the list, so the other docs naming M33 had those lines read and were left.** `docs/FLOW.md` is TARGET and last reviewed 2026-09-30, and its map still shows M32 as ⬜ and M33 as 🔒, so it is stale on more than this. `docs/PROJECT_CHARTER.md:40`, ADR-032 at its "M33's question" sentence and ADR-035's "M33's spike" sentence look forward, and an ADR's body is corrected by addendum. The program plan's line "**M33 is unchanged**" and its "M33 runs early on purpose" paragraph still say HD Golf and the frozen parser: the row and the new note both say the M33 plan wins.
  - **§M38's flow step 2 now leads with the photo as imported** and leaves VisionKit as what the milestone measures, rather than deleting it or choosing for M38. It states item 2's question as a question.
  - **CLAUDE.md's line 7 still says `.venv` is 3.13**, where this Mac's is 3.14.3, because §P18 says "Nothing else".
- **For P19**: `git add` these beside `log.md` and the rest of the spike: `CLAUDE.md`, `docs/CONFORMANCE.md`, `docs/README.md`, both ADRs, `docs/plans/m31-m40-shot-first-pivot.md`, `docs/plans/m33-vision-spike.md` and `spikes/README.md`. `docs/README.md`'s document count is untouched at 87, and P19 moves it to 88 with `log.md` staged. The program plan's M33 row reads open and its M38 row was not touched, which is what P19's rows should agree with in `ROADMAP.md`. The WORKLOG entry can name FLOW.md's staleness as left open.

### P19 (2026-11-04) — closed as far as this run can: one commit, verified, and M33 still open

- **What landed.** The top `WORKLOG.md` entry. `ROADMAP.md`: the `Last Updated` date, the M33 row and §M33's status (🔄 open, not ✅, with **GO, provisional: pasted copies. Clips: not measured**, and what is outstanding), and the M38 row's "Needs" naming the verdict and leaving unblocking to the user, under the decision's table. `docs/README.md`: 88 documents, 15 outside `docs/`, eight in `spikes/`. Memory: `golf-coach-status` and its `MEMORY.md` line say the Mac holds pasted copies and no originals, clips, HD Golf photos or models, with the verdict as a pointer to `log.md`. One commit onto `main`, not pushed.
- **Verified**, each as the rules say:
  - `cargo test --workspace --no-fail-fast`: **1,761 passed, 5 failed, 26 ignored**, the five exactly finding 1's (rule 7). `cargo fmt --check` exits 0. `cargo clippy --all-targets 2>&1 | grep -c '^warning'` prints 28, the baseline (finding 2). `golf-core rerecord --declare spec/declarations/screen-v1.json --dry-run`: 45 vectors, 0 changed, 0 files written.
  - `probe.py check`: 6 of 6 match, and it prints the original stills 0 of 6, the clips 0 of 5, HD Golf 0 of 15 and the OCR models 0 of 3 as outstanding. `control`: all held. `verdict`: exit 0, as P17 asked. `tests/test_docs_truth.py`: 91 passed, with `log.md` staged.
  - `git diff --stat -- crates src scripts tests spec` is empty, and `thresholds.md` has no diff. `data/` holds only its README, with nothing ignored or changed under it. The sorted digest of the 56 files of `results/`, `boxes/`, `frames/runs/` and `thresholds.md`, by P16's spelling, is still `4396a7eb…ee32`, and none is newer than this phase's marker.
- **The "if a rule fired" lines did not run**: `v17`, `career-v3`, `tests/test_conformance.py` and `conformance.py check`, as P15 closed. So call 15's Windows gates are not in the report.
- **Choices under rule 1**, each the smaller edit:
  - **`probe.py measure` did not run.** No Verify line lists it for this phase, the block's "each from P7 on" notwithstanding, and a re-read would rewrite `results/C0.json`'s `repeat`, which is the record P9 kept. `check`, `control` and `verdict` ran, since the last two hold what is committed.
  - **§P19's "Nothing else" is read as written for `ROADMAP.md`.** The NEXT ACTION paragraph still says whether Vision reads the HD Golf screen is the biggest unknown, §M38's status is as it was, and the M38 row's "A Mac for the skeleton" stands with the verdict added after it. The WORKLOG entry lists these as left stale, with FLOW.md's map, the program plan's "M33 is unchanged" paragraph and `CLAUDE.md`'s `.venv` 3.13, which P18 left.
  - **The commit title uses `log.md`'s spelling** with its colon and full stop, as P17 asked, which makes it longer than the repo's other titles.
- **For later rows, when the files reach this Mac**: this commit is what keeps the copies' measurement, as P9 said. A new row reopens the status table here, and `ROADMAP.md`'s M33 row and §M33, the program plan's row, `log.md` and the two addenda each carry the label and move together.
