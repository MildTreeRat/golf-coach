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
| **P1** | The photo manifest and the truth table as data; every digest checked | 🔄 Started 2026-11-04 10:08 |
| **P2** | `vision-ocr`: `decode`, `read` at C0, and a geometry self-test | ⬜ |
| **P3** | `TextBox.rgb` in Swift, held exactly to Rust's `glyph_rgb` | ⬜ |
| **P4** | Grid frames off the five clips, with the lab's rule | ⬜ |
| **P5** | The lab's working images | ⬜ |
| **P6** | `vision-ocr rectify`: the phone's rectifier stand-in | ⬜ |
| **P7** | `probe.py measure`: the comparator, proven on controls | ⬜ |
| **P8** | The lab's reader on the same inputs: the baseline rules must not move | ⬜ |
| **P9** | The first measurement: C0 on Full Swing's stills | ⬜ |
| **P10** | C0 on the clip frames | ⬜ |
| **P11** | C0 on HD Golf, diagnostic | ⬜ |
| **P12** | The Swift ladder, if C0 missed | ⬜ |
| **P13** | Parser rule 1, if still missing | ⬜ |
| **P14** | Parser rule 2, if still missing | ⬜ |
| **P15** | Parser rule 3, if still missing | ⬜ |
| **P16** | The confidence calibration | ⬜ |
| **P17** | The verdict: `log.md`, and what M38 is told | ⬜ |
| **P18** | Docs | ⬜ |
| **P19** | Close: verify, WORKLOG, ROADMAP, memory, and the commit | ⬜ |

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
