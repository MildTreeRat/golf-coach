# Plan: M42 — Full Swing: the second device, its screen, and its impact video (M41 folded in)

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-10-30. **Milestone**:
[ROADMAP §M42](../../ROADMAP.md#m42-full-swing-the-second-device-and-its-impact-video), with
[§M41](../../ROADMAP.md#m41-the-course-play-screen-layout) folded into it (decision 2): M41's
"course-play layout" photo (`data/raw/shot_screens/2026-10-07-1.jpg`) is the same Full Swing grid
screen as five of the six Live Photos this milestone was planned from. This file is the phase list,
and where it and either ROADMAP section disagree, **this file wins**.

**Governing decisions**: [ADR-034](../decisions/034-shot-first-phone-first.md) §2 (declared ∩
printed; what counts as printed), [ADR-014](../decisions/014-screen-capture-shot-ingestion.md) and
its M34 addendum (the screen reader), [ADR-010](../decisions/010-benchmark-ranges.md) §2 (no score
beats a wrong one), [ADR-035](../decisions/035-rust-everywhere-python-where-required.md) clauses 3
(Rust records) and 4 (frozen Python), [ADR-008](../decisions/008-project-structure.md) as cargo
edges, [ADR-022](../decisions/022-learned-artifacts-as-committed-data.md) (committed data carries
its source), and the M37 plan's decision 3 (strike location only from a measured face-impact offset).

**Level: L3.** `ShotData` gains two keys, a second real device enters `devices.json` and
`profiles.json`, the screen parser gains four opt-in rules, `TextBox` gains a field the phone must
fill (M38), and a fourteenth crate, `crates/impact`, is added. Every phase reads its own list below
rather than the repo.

**Priority, in the user's words**: the screen reading comes first ("it's more important to get the
other stuff"). The impact video is attempted in good faith and **abandoned without blocking anything
if it does not work** (P13's rule). Accuracy of the impact reading is not judged in this milestone;
the user reviews it afterwards.

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot — the findings are what the next phase
   is planned against — and address anything a later phase must know to that phase by number
   ("**For P9**: …");
3. stop, and tell the user to `/clear` and run `/next-phase`.

**Nothing is committed until P16** (call 18). Every phase leaves its work in the working tree, and
P16 makes one commit onto `main`.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document; the six Live Photos copied with their sidecars; the pointers | ✅ Done *(2026-10-30)* — approved by the user |
| **P1** | `ShotData` gains `backspin` and `sidespin`, written only when set | ✅ Done *(2026-10-30)* |
| **P2** | `TextBox.rgb`: the glyph colour, measured by `crates/ocr`; `golf-ocr read --boxes` | ✅ Done *(2026-10-30)* |
| **P3** | The screen-quad finder refuses a quad whose ordering repeats a corner | ✅ Done *(2026-10-30)* |
| **P4** | Parser rule 1: a box holding two merged labels is split (opt-in) | ✅ Done *(2026-10-30)* |
| **P5** | Parser rule 2: a tile printed twice must agree with itself (opt-in) | ✅ Done *(2026-10-30)* |
| **P6** | Parser rule 3: the sign read from a lit letter (opt-in) | ✅ Done *(2026-10-30)* |
| **P7** | Parser rule 4: a value printed in feet is stored in yards (opt-in) | ✅ Done *(2026-10-30)* |
| **P8** | The `full_swing` profile and its `devices.json` declaration | ✅ Done *(2026-10-30)* |
| **P9** | The Full Swing photo gate on the six grid photos, and the profile's tuning | ✅ Done *(2026-10-30)* |
| **P10** | Hand-worked Full Swing screen vectors | ✅ Done *(2026-10-30)* |
| **P11** | `crates/impact` I: frames, and the ball | ✅ Done *(2026-10-30)* |
| **P12** | `crates/impact` II: the club head, the shaft, the contact frame, the offset | ✅ Done *(2026-10-30)* |
| **P13** | `golf-lab read-impact` on the five Live Photo clips; tuning (no go/no-go since decision 9) | ✅ Done *(2026-10-30)* |
| **P14** | The grid read off a clip's frame, and the Full Swing clip gate (decision 7) | ✅ Done *(2026-10-30)* |
| **P14b** | `golf-lab import-video`: the second import method, the impact offset wired in regardless | ✅ Done *(2026-11-02)* |
| **P15** | Docs: ADR addenda, CONFORMANCE, ARCHITECTURE, data/README, CLAUDE.md, the program plan | ✅ Done *(2026-11-02)* |
| **P16** | Close: verify, WORKLOG, ROADMAP, memory, and the one commit | ✅ Done *(2026-11-03)* |

---

## Rules every phase keeps

- **Every phase ends green on everything it can reach.** If a phase cannot end green, stop and
  record why as a finding; do not carry a red suite into the next session. A vector a phase cannot
  pass yet is skipped *by name* in its gate, with the phase that will pass it, never by a blanket
  ignore.
- **No phase asks the user anything.** Every decision a phase needs is in this file (the decisions,
  the calls, the phase's own "If" lines). Where a phase hits something this file does not cover, it
  takes the option that changes the least committed behaviour, records the choice as a finding
  addressed to P15 (so the docs say it) and continues. It stops only where a rule below says *stop*.
- **Preconditions are checked first, and a failed one stops the phase** with a finding naming it:
  the ONNX models in `data/models/ocr/` (P2, P3, P9, P10, P16), ffmpeg on `PATH` or in `GOLF_FFMPEG`
  (P2, P3, P9, P10, P13, P14, P16), and the photos under `data/raw/shot_screens/` with the digests in
  "The photos" below (P9, P10, P13, P14, P16).
- **Frozen Python is not edited.** `git diff -- src/ scripts/ tests/` is empty at every phase. The one
  exception: if a hand edit to a Rust-owned schema (P1) trips `tests/test_conformance.py`, the fix
  goes in that file's existing `RUST_OWNED_SCHEMAS` allowance and nowhere else, and is a finding.
  The second exception, the user's answer at P16: one clause in that file's
  `test_a_screen_document_carries_what_a_port_reads`, so that a box may carry `rgb`.
- **`data/`**: P0 writes `data/raw/shot_screens/` (the six Live Photos, their sidecars, and one key
  in `2026-10-07-1.json`); P13 writes review images under `data/raw/impact-review/`. **Nothing
  writes `data/processed/`**: its digest is `07f37373abe42fbe333ae263590a626577f3becd2c15b869dd83c47f44ad732d`
  over 192 files, checked at P0, P9, P14 and P16 with the command under "Verify".
- **No committed vector changes, and no version key moves.** `ANALYSIS_VERSION` stays 17,
  `SCREEN_PARSER_VERSION` 1 and `CAREER_VERSION` 3 (call 3). No declaration is added under
  `spec/declarations/`. `git diff -- spec/vectors` shows **additions only**, and only the new files
  P10 writes under `spec/vectors/screen/hand/`. Any change to an existing vector file is a finding
  and a **stop**.
- **`hd_golf` reads exactly as before.** Every parser rule this milestone adds is opt-in per profile
  or per field, and `hd_golf`'s profile entry in `crates/screen/profiles.json` is byte-unchanged.
  The 13-photo OCR gate (`crates/ocr/tests/read.rs`) passes unchanged wherever it runs.
- **New code is Rust.** `crates/impact` depends on nothing (std only). `crates/lab` may depend on
  it. No other crate depends on it in M42 (M38's `crates/app` will). `ort` is reached only through
  `crates/ocr`, as `crates/ocr/tests/ort_reach.rs` pins, and `impact` joins that test's `GUARDED`
  list (P11).
- **No new dependency**, except `image = "=0.25.9"` with `default-features = false, features =
  ["png"]` in `crates/lab` for P13's review images (already in the lock through `crates/ocr`).
- **Every release build and release test uses `--target-dir target/m42`.** A Claude Code session
  holds `target/release/golf-mcp.exe` open as the `golf-coach` MCP server, and Windows will not
  replace a running executable (the M29 close). `target/m42` is never in use by a session. Debug
  builds (`cargo test`) are unaffected and use `target/`.
- **House style** (`CLAUDE.md` §House style, `docs/CODE_STANDARDS.md`): comments say *why* and carry
  the measurement or the rejected alternative. Every module this milestone adds **has no Python twin
  and says so** in its first lines, as `crates/contracts/src/capability.rs` does. Every constant
  carries the measurement it was set from (this file's findings are where those measurements are).
- **Do not write counts into prose** that a test or a registry owns. Where this plan quotes a count,
  it is a snapshot for the phase to re-measure.
- **The doc-count pin reads `git ls-files`**, so this file is invisible to it until P16 stages it.
  Do not touch `docs/README.md`'s document count before P16. This file avoids the
  `**Status: … N/M phases` spelling on purpose (`tests/test_docs_truth.py::_PHASE_STATUS`).
- **Line endings**: the plain `.json` vectors Python wrote are CRLF, hand-written ones LF; keep each
  file's own. Git Bash heredocs mangle `—`, `±` and `°`; use the Edit/Write tools or Windows Python
  for files that hold them.
- **Scratch work goes in the session's scratchpad**, never in the repo, and a throwaway script a
  phase writes is deleted before the phase ends.

## Verify — the commands, and when each runs

```bash
cargo test                                          # every phase that touches crates/ or spec/
cargo clippy --all-targets && cargo fmt --check     # every phase that touches crates/
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py   # every phase that edits a doc (P0, P15, P16)
.venv/Scripts/python.exe -m pytest                  # P1 (schemas), P16 — run it alone, not beside cargo test
.venv/Scripts/python.exe -m ruff check src tests scripts      # P16
.venv/Scripts/python.exe -m mypy src                # P16
.venv/Scripts/python.exe scripts/conformance.py check         # P1, P16
# The 13-photo OCR gate (ignored by default: needs the models, ffmpeg and data/processed's photos):
cargo test --release --target-dir target/m42 -p ocr --test read -- --include-ignored          # P2, P3, P9, P16
# The Full Swing photo gate (P9 creates it):
cargo test --release --target-dir target/m42 -p ocr --test full_swing -- --include-ignored    # P9, P10, P14, P14b, P16
# The Full Swing clip gate (P14 creates it):
cargo test --release --target-dir target/m42 -p golf-lab --test full_swing_clips -- --include-ignored  # P14, P14b, P16
# The three re-record dry runs, each must report 0 changed and 0 files written. golf-mcp is found
# beside golf-core, so build both into target/m42 first:
cargo build --release --target-dir target/m42 -p golf-core -p golf-mcp                        # before the dry runs
target/m42/release/golf-core rerecord --declare spec/declarations/v17.json --dry-run          # P1, P16
target/m42/release/golf-core rerecord --declare spec/declarations/screen-v1.json --dry-run    # P1, P8, P10, P16
target/m42/release/golf-core rerecord --declare spec/declarations/career-v3.json --dry-run    # P1, P16
# The data/processed digest (expected 07f37373…, 192 files):
find data/processed/sessions data/processed/golfers data/processed/shots -type f | LC_ALL=C sort | xargs -d '\n' sha256sum | sha256sum   # P0, P9, P14b, P16
git diff --stat -- spec/vectors src scripts tests    # every phase: additions under spec/vectors/screen/hand/ only (P10 on)
```

---

## Decisions taken in the interview (2026-10-30)

The user's, in one round. A later phase that wants to revisit one needs a reason the interview did
not have, recorded as a finding, and does not act on it without the user.

1. **The target is the Full Swing grid layout** (photos `IMG_2889`, `IMG_2891`–`IMG_2894` and
   `2026-10-07-1`): sixteen tiles plus the impact-video panel, chosen because the panel is what
   impact can be read from. The club-view layout (`IMG_2888`: eight tiles, a face/path diagram, signs
   as `-` and `LEFT`/`RIGHT`, no impact video) is copied with the rest (decision 6) and **not read**
   by anything this milestone builds.
2. **M41 is folded into M42.** M41's photo, and its open questions (the lit L/R and D/F signs, the
   spin tiles with no `ShotData` key, the duplicate `TOTAL YD`), are this milestone's. M41 closes as
   merged at P16.
3. **The impact reader is built and tried, and its accuracy is not judged.** No face-tape or
   foot-spray test, in M41 or M42. "Try to get it to work, if it doesn't then move on, it's more
   important to get the other stuff. We can always take another look at impact again later." "Don't
   worry about whether it was accurate or not. Just implement the stuff. I can take a look at it
   after." So P13's gate is mechanical (does it produce a reading, and are its refusals named), and
   its review images exist for the user's later look.
4. **`backspin` and `sidespin` are added to the shot record as printed.** Whether a spin rate and
   axis computed from them is useful is decided later, so **nothing in M42 computes one**:
   `spin_rate` and `spin_axis` stay unset on every Full Swing shot.
5. **Full Swing's club speed and smash factor are `analysed`**, not `shown_only` as HD Golf's are.
   Full Swing measures the club itself; the low smashes on these photos (0.85, 1.07, 1.08) look like
   real off-centre strikes, not a broken club speed.
6. **The six Live Photos are copied** from `C:\Users\aaron\Desktop\pictures` into
   `data/raw/shot_screens/` (P0).

### Decisions taken after P9 (2026-10-30)

The user's, asked by the orchestrating session once P9 was done. They **replace calls 14 and 15** and
reshape P13–P16; read them before any of those.

7. **Two import methods, neither needing the other.** "Don't make the video a requirement, just
   treat the two different pictures we have as two different methods to import data." A Live
   Photo's **still** imports a shot as today (`golf-lab import-photos`: the numbers, no impact
   offset). Its **motion clip** imports a shot on its own (`golf-lab import-video`, new): "The video
   has numbers of the shot stats next to it", because the grid is on screen beside the replay panel
   (finding 9). So a clip's shot holds the grid's numbers, read off one of its frames, **and** the
   impact offset read off the replay. Nothing pairs a still with its clip; importing both halves of
   one Live Photo makes two shots of one swing, which P15 documents and the user avoids by choosing
   one method per Live Photo. Why the clip matters: "The graphic doesn't accurately show where the
   ball was hit. The video is the best indicator we can get."
8. **Live Photo clips only.** The 1–2 s motion part (`.mov`, any case). An ordinary phone video of
   the replay is not accepted in M42: none exists to tune on.
9. **The impact reader is wired in regardless — no go/no-go.** Every clip import runs it. A clip
   whose replay it cannot read still stores its shot, with `impact_offset_h` `None`, the warning and
   `misread` of call 13. P13 still runs, tunes within call 12's brackets and writes the review
   images; it decides nothing.
10. **A clip's numbers keep the photo rule: no wrong value** (ADR-010 §2, the rule P9's gate holds
    the stills to). This one is the repo's invariant rather than the user's answer, recorded so P14
    does not have to rediscover it: a field the clip gate reads wrong after tuning is **BLOCKED** to
    the user, never shipped.

## What the planning read found (2026-10-30)

Measured in the planning session, with the shipping reader and a throwaway box dump (deleted). Each
is the evidence for a call below; a phase re-measures what it depends on.

1. **Today's reader gets this screen wrong, not just blind.** `golf-ocr read IMG_2889.HEIC` (default
   device `hd_golf`) read four fields, two under the wrong label: `launch_angle 10.1` (the ball
   direction), `smash_factor 85.2` (the apex). The read was flagged `NEEDS REVIEW`, so it would not
   have been stored silently, but it is unusable.
2. **The OCR returns labels without spaces** (`CARRYYD`, `PATHDEG`, `BALLDIRECTIONDEG`), which the
   label matcher already scores above `LABEL_MATCH_THRESHOLD` against `Carry Yd` etc.
   (`normalize_label` and `difflib` ratio: `BALLDIRECTIONDEG` against `BALL DIRECTION DEG` ≈ 0.94).
   A unit-less read happens too: `BALLSPEED` on `IMG_2892` (≈ 0.78 against `Ball Speed Mph`, below
   the threshold, which is why the profile carries unit-less aliases, P8).
3. **Two adjacent labels sometimes come back as one box**: `CLUBSPEEDMPHBALLSPEEDMPH` (`IMG_2891`)
   and `BALLDIRECTIONDEGLAUNCHANGLE` (`IMG_2891`, `2026-10-07-1`). Neither half is located today
   (call 6, P4).
4. **The sign letters are detected only sometimes, and their colour says which is lit.** Each tile
   prints both letters (`L` `R` above ball direction, `D` `F` above sidespin) and lights one yellow.
   PP-OCR returned 0, 1 or 2 of each pair as single-letter boxes. Mean RGB of the brightest quarter
   of each letter box's pixels, in the rectified image:

   | photo | letter | rgb | lit (by eye) |
   |---|---|---|---|
   | IMG_2889 | R | (239, 222, 90) | yes |
   | IMG_2889 | D | (227, 207, 72) | yes |
   | IMG_2889 | F | (187, 183, 176) | no |
   | IMG_2891 | R | (200, 206, 197) | no (L lit, not detected) |
   | IMG_2891 | F | (215, 214, 206) | no (D lit, not detected) |
   | IMG_2892 | R | (243, 225, 73) | yes |
   | IMG_2892 | D | (241, 223, 91) | yes |
   | 2026-10-07-1 | R | (249, 220, 96) | yes |
   | 2026-10-07-1 | F | (254, 238, 113) | yes |

   Lit: blue ÷ min(red, green) = 0.32–0.47. Unlit: 0.94–0.98. White value digits on the same photos
   measure ≈ (233–251, 233–250, 221–249), ratio ≈ 0.95–1.0. That gap is call 4's threshold.
5. **The screen-quad finder accepts a degenerate quad on two photos**: on `IMG_2893` and `IMG_2894`
   `find_screen_quad` returns a `Corners` quad whose ordered corners repeat one point
   (`[[17.1, 1639.3], [5706.3, 2638.9], [5706.3, 2638.9], [542.6, 4032.7]]` on `IMG_2893`), so the
   "rectified" image is a 1600×678 (1600×540) sliver and **no text at all** is recognized. The
   four-gon is convex and plausibly sized, and `order_quad`'s sum/difference rule assigns one corner
   to two roles. It is the frozen Python rule ported faithfully, so Python has the same defect (P3).
6. **`TOTAL YD` is printed twice**, in the header row and the data row, with the same value on every
   photo (call 7). The header's third tile is `TO HOLE YD`, or **`TO HOLE FT`** on the green
   (`IMG_2893`).
7. **`APEX FT` is in feet**; `ShotData.apex_height` is yards (`crates/contracts/src/shot.rs`)
   (call 8).
8. **Path and face print their own sign** (`-2.0°`, `13.2°`); a positive value has no `+`. The
   club-view photo spells the convention out: `CLUB PATH 1.0° RIGHT`, `FACE ANGLE -7.8° LEFT`, so a
   minus is left. For a right-handed golfer that is `ShotData`'s convention (+ = in-to-out, + =
   open) with no conversion, which is why those two tiles carry no sign rule at all (P8).
9. **The impact video is an overhead replay.** The Live Photo's motion part (`.MOV`, HEVC,
   1920×1440 stored, variable rate, 31–61 frames over 1.1–2.1 s) catches the screen's slow-motion
   replay in the panel right of the grid: the ball a white disc ≈ 40 px across at 1920 wide, an iron
   head a light bar ≈ 2–3 ball-diameters long with a dark shaft from one end, motion-blurred and often
   doubled (two club images in one frame). The club reaches the ball over ≈ 4–6 frames; after
   impact the ball is gone and a small **blue dot** marks where it sat (`IMG_2893`). **Heel–toe is
   visible from above; high–low is not**, so only `impact_offset_h` can come from this video.
   `IMG_2892`'s still caught the panel empty, so the shutter's timing decides whether the replay is
   in the motion part at all. `IMG_2891` and `IMG_2892` are portrait stills (4284×5712); their
   motion parts carry rotation metadata that ffmpeg applies by default.
10. **Three tests derive `ShotData`'s key set by serializing a default shot**
    (`crates/contracts/tests/capability.rs::shot_data_keys`, `crates/contracts/tests/schemas.rs::rust_keys`)
    and say outright that a `skip_serializing_if` field is "the right question to be asked then"
    (call 1 answers it).
11. **`screen::tile_refusals` requires a profile tile for every declared, analysed field** it may
    refuse, and errors otherwise (`crates/screen/src/tiles.rs:81`), so a field read from the impact
    video needs its own rule there (call 13, P14b).
12. **A photo entry's photo is stored as `shot_screen.<digest12>.<ext>`** inside its swing
    directory; nothing enumerates the files of a swing directory (only session and swing
    directories), and `golf-lab import-photos` scans a directory one level deep for
    `jpg jpeg png heic heif` (`crates/lab/src/import.rs::images_in`), so `.MOV` files and
    subdirectories beside the photos are never imported as photos.
13. **The device is the caller's choice today**: `golf-ocr read --device D`, `golf-lab import-photos
    --device D`, or `GOLF_LAUNCH_MONITOR_PROFILE`; the default is `hd_golf` (call 10).

## The photos

Copied by P0 into `data/raw/shot_screens/` under their own names, with the modification time
preserved (the reader stamps a shot's timestamp from the photo's mtime,
`ocr::screen_timestamp`). Digests, SHA-256:

| file | sha256 |
|---|---|
| IMG_2888.HEIC | be930f8575d6aed8b8716ed380d4af5b34d21be3b62d19212331f0d771ec2db2 |
| IMG_2888.MOV | 83a83326dca4c2eef48d3e6292a6b48f8d709a253b044cd41e9e2f9f2e67023e |
| IMG_2889.HEIC | d47e3e9607211a1350c09416d96f2f7e64a3d237a29d6f5a9ba57890f5257d8e |
| IMG_2889.MOV | ed2112dbf7f85320d04a1b3d2681853ba5329c989669b8dc1e6095a705c6a2b7 |
| IMG_2891.HEIC | 73be98e34cee2eafa22ad8d693c9c1c4c4efac585616f452d6cd5ceda58a15c0 |
| IMG_2891.MOV | 0c86193a1af6cf592ced2aa23cca34fe67f19cd1b8eada2601c7019758790815 |
| IMG_2892.HEIC | 1c4b731de2871303b6387a843e65d893a0c37d059b7ff0cca321eea2f1fc6d04 |
| IMG_2892.MOV | 0fa5aa0d2d799d87800693a52d5454ed66ccdf08c293967edc59be9425b13513 |
| IMG_2893.HEIC | 204872ef4b6ccd2c7c2730ca14283b43e4a3c5f4e57164373d52c19fb72d9ca8 |
| IMG_2893.MOV | bf97341db7521dd3b8b50836cc4e90120c192d437b03f18dc6b5903c17b8520a |
| IMG_2894.HEIC | 227d9985b337a70248f0e268eea4e73fc3e5860bc0b2418afe6082a4b19c399b |
| IMG_2894.MOV | 6ed8385c1891a3ba1c5c1db0fd129a77eaf283be88db3395b03fca8fc3943d31 |
| 2026-10-07-1.jpg (already there) | 96241978f183d70cfbd633802b395362a68ea0b3905324fbbc9a40e18c51b0e1 |

## The truth table — what each grid photo prints, read by eye

The oracle for P9's photo gate. Read from the photos in the planning session; the lit letter of each
pair confirmed on a crop. `ShotData` values after the contract's conventions: `launch_direction` + =
right (R lit), `sidespin` + = fade (F lit) and − = draw (D lit), `apex_height` = printed feet ÷ 3.0
(call 8). Boundary tiles (`SHOT RESULT`, `TO HOLE`) are not stored and not listed.

| photo | carry | total (×2) | club | ball | path | face | ball dir | launch | apex ft | smash | backspin | sidespin |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| IMG_2889 | 131 | 134 | 95 | 102.5 | -2.0 | 13.2 | 10.1 R | 24.5 | 85.2 | 1.08 | 9899 | 4199 D |
| IMG_2891 | 162 | 203 | 86.9 | 112.8 | -1.7 | -0.6 | 0.8 L | 12.1 | 39.9 | 1.30 | 4236 | 464 D |
| IMG_2892 | 80 | 82 | 91.6 | 77.4 | -4.5 | 6.9 | 4.6 R | 34.6 | 71.4 | 0.85 | 8855 | 2032 D |
| IMG_2893 | 184 | 224 | 107.6 | 134 | -1.7 | 3.2 | 2.2 R | 5.5 | 15.5 | 1.25 | 2284 | 638 D |
| IMG_2894 | 105 | 107 | 81.3 | 87.2 | -1.7 | 7.5 | 5.7 R | 31.4 | 80.6 | 1.07 | 9985 | 2209 D |
| 2026-10-07-1 | 279 | 289 | 111.3 | 158.6 | 1.6 | 4.0 | 3.5 R | 19 | 142.2 | 1.43 | 2521 | 336 F |

As `ShotData`: `carry_distance`, `total_distance`, `club_head_speed`, `ball_speed`, `club_path`,
`club_face_angle`, `launch_direction` (`+` for R, `-` for L), `launch_angle`, `apex_height` (feet ÷
3.0, computed in the test as `85.2 / 3.0` etc., never typed as a decimal), `smash_factor`,
`backspin`, `sidespin` (`-` for D, `+` for F). Twelve stored fields per photo, seventy-two in all.

The header rows, for the hand vectors' notes: `IMG_2889` NATURAL AREA / 134 / TO HOLE YD 127.1;
`IMG_2891` FAIRWAY / 203 / YD 142.0; `IMG_2892` FAIRWAY / 82 / YD 19.2; `IMG_2893` GREEN / 224 /
**FT** 45.4; `IMG_2894` FIRST CUT / 107 / YD 30.2; `2026-10-07-1` NATURAL AREA / 289 / YD 36.7.

## Calls this plan makes

Where the interview left a question open, this plan answers it here, so a phase never has to.

1. **`backspin` and `sidespin` are written only when set** (`#[serde(default, skip_serializing_if =
   "Option::is_none")]`), the precedent M37 P3 set for `CorpusSwing`'s two keys and for the same
   reason: a key at its default on every committed shot would otherwise move every vector family
   that serializes a shot (engine, shot, screen, storage, mcp) and force three version bumps and
   three re-records for a shape change with no number in it. Written only when set, **no committed
   vector moves** and every reader of an older shot reads `None`. `shot.rs` exports
   `pub const WRITTEN_WHEN_SET: [&str; 2] = ["backspin", "sidespin"]`, and the two test helpers that
   derive the key set from a default serialization (finding 10) take the union with it.
2. **Names, units and signs.** `backspin`: rpm, as printed, never negative. `sidespin`: rpm, **+ =
   fade, − = draw**, read from which of `D`/`F` is lit; the same polarity as `spin_axis` (+ = fade)
   and the device's own letters, never converted by handedness. Placed after `carry_offline` in
   `ShotData`'s field order. Plausible ranges in `crates/screen/src/validate.rs`: `backspin` 0 to
   15000 (as `spin_rate`), `sidespin` −6000 to 6000, and `apex_height` 0 to 80 yards (no profile
   read apex before; 80 yd is 240 ft, past any real apex).
   **No cross-check of the sidespin's sign against face-to-path**: five of the six photos show `D`
   with the face open to the path, which a toe strike's gear effect produces, so such a check would
   flag the screen as wrong when it is right.
3. **No version key moves.** `SCREEN_PARSER_VERSION`'s rule is "bump whenever a re-read of the same
   photo could produce a different shot". Every rule M42 adds is opt-in and only the new profile opts
   in, `hd_golf`'s profile is unchanged, and P3's quad fix touches only a quad that repeats a corner,
   which none of the 13 bay photos has (the OCR gate proves it), so a re-read of any photo a stored
   shot came from is unchanged; and no stored shot names `full_swing`. `ANALYSIS_VERSION` and
   `CAREER_VERSION` do not move because no committed artifact can change (call 1). P8 adds one
   sentence under ledger entry 1 in `shot.rs` saying so. **If any committed vector changes in any
   phase, stop**: this call was wrong and the user decides.
4. **`TextBox.rgb`: the glyph colour, and the lit rule.** `TextBox` (`crates/screen/src/lib.rs`)
   gains `#[serde(default)] pub rgb: Option<[f64; 3]>`: **the mean red, green and blue (0–255) of
   the brightest quarter of the box's pixels**, in the image the box's coordinates are in. Pixels
   are those of the box clipped to the image (`x0 = floor(max(x, 0))`, `x1 = min(ceil(x + width),
   W)`, likewise y); ranked by `r + g + b` ascending with a stable sort; the brightest quarter is
   the pixels from index `n * 3 / 4` (integer division) to the end; `None` when the box holds no
   pixel. `crates/ocr` fills it (P2); the phone's Vision boxes must fill it by the same definition
   (M38, written into the program plan by P15). The parser decides **lit** from it, in
   `crates/screen` so both hosts share one rule (P6): `pub fn is_lit(rgb: [f64; 3]) -> bool`, true
   when `min(r, g) >= LIT_MIN_BRIGHTNESS` (120.0) and `b <= LIT_BLUE_RATIO * min(r, g)`
   (`LIT_BLUE_RATIO` = 0.7, between finding 4's 0.47 and 0.94).
5. **A sign that cannot be decided is withheld, never stored unsigned.** For a `lit_sign` tile:
   exactly one of the pair's letters detected and lit → that letter's sign; detected and unlit → the
   other letter's sign; both detected and exactly one lit → its sign; both lit, both unlit, neither
   detected, or a letter box with no `rgb` → **no value**, and the warning
   `"{label}: neither {A} nor {B} could be read as lit - sign unknown, not stored"`. A magnitude of
   exactly 0 needs no sign and is stored as 0.0. (The existing `sign_tokens` branch's "storing the
   magnitude as printed" is not used for these tiles: a magnitude stored unsigned on a signed field
   is a wrong number half the time, ADR-010 §2.) A letter box is a value box of the tile's cell whose
   text, stripped, uppercased, is exactly one of the pair's tokens.
6. **Merged labels are split, opt-in per profile** (`"split_merged_labels": true` on a profile,
   default false). Before labels are assigned, every box whose text scores below
   `LABEL_MATCH_THRESHOLD` against every field is tried as two labels: for each cut `k` in
   `1..len` of its normalized text with spaces removed (`despace`), the left part and the right part
   are each scored against every field (label and aliases, `ProfileField::matches`); a cut qualifies
   when both best scores reach the threshold and name **different** fields; the qualifying cut with
   the highest `min(left, right)` wins, ties to the smallest `k`. The box is replaced, in place in
   the list, by two boxes: the left with `width * k / len`, the right with the rest, both keeping
   `y`, `height`, `confidence` and `rgb`, and texts the two halves. Two-way splits only.
7. **A tile printed twice must agree with itself**, opt-in per field (`"printed_twice": true`,
   default false, allowed only on a stored number field). The field takes **up to two** label boxes,
   and two assignments that differ only in which of its boxes is which are one assignment, so they
   never tie with each other. Both cells are read. Both give a number and the numbers are equal → the
   value is stored. They differ → no value, and the warning
   `"{label}: printed twice as {a!r} and {b!r} - not stored"`. Only one gives a number (the other
   blank, unreadable or absent) → that number is stored. `raw_fields[label]` holds the two cells'
   texts joined with `" | "` in reading order (top first), or the one text when one box was located.
8. **A value printed in feet is stored in yards**, opt-in per field (`"printed_unit": "feet"`; the
   only variant). The parser stores `magnitude / 3.0`, unrounded; `raw_fields` keeps the printed
   text. Allowed only on a target whose `ShotData` unit is yards (`apex_height`, `carry_distance`,
   `total_distance`, `bounce_and_roll`, `carry_offline`), pinned by a profile test.
9. **Profile load stays tolerant of unknown keys** (the profiles carry `_comment`), so each new key
   is pinned by a test that a profile using it parses to the intended value, and P8's
   `full_swing_uses_every_rule` test checks the shipped profile sets each one where this plan says.
10. **The device stays the caller's choice**: `--device full_swing` or
    `GOLF_LAUNCH_MONITOR_PROFILE=full_swing`. No auto-detection in M42; the phone's per-session
    device choice is M38's.
11. **`full_swing`'s declaration** in `crates/contracts/devices.json`, in the profile's tile order:
    `total_distance`, `carry_distance`, `club_head_speed`, `ball_speed`, `club_path`,
    `club_face_angle`, `launch_direction`, `launch_angle`, `apex_height`, `smash_factor` — all
    `analysed` (decision 5) — then `backspin` and `sidespin`, **`shown_only`**, each with the note
    "Printed by Full Swing's grid screen and stored as printed. No analysis reads it: whether a spin
    rate and axis computed from backspin and sidespin are worth having is the user's to decide
    later (the M42 plan's decision 4)." P14b appends `impact_offset_h` (call 13), whatever P13 read (decision 9).
    `source`: "The non-null targets of the full_swing profile in crates/screen/profiles.json, in its
    tile order, written at M42 P8 from six photos of Full Swing's grid screen (the M42 plan's truth
    table). crates/screen/tests/capability.rs holds the screen-read fields equal to the profile's
    targets." `added`: "2026-10-30, M42 P8".
12. **The impact reader** (`crates/impact`, std only, no Python twin):
    - **API.** `pub struct Frame { pub width: usize, pub height: usize, pub rgb: Vec<u8> }` (row-major,
      3 bytes a pixel). `pub fn read_impact(frames: &[Frame]) -> Result<ImpactReading, ImpactRefusal>`.
      `pub struct ImpactReading { pub offset_h_mm: f64, pub contact_frame: usize, pub frames:
      usize, pub ball_center: [f64; 2], pub ball_diameter_px: f64, pub heel: [f64; 2], pub toe:
      [f64; 2] }` (pixels in the frames). `pub enum ImpactRefusal { NoFrames, NoBall, NoClubHead,
      NoContact, NoShaft, OutOfRange(f64) }`, each with a `pub fn sentence(&self) -> String` a golfer
      can read. `pub const BALL_DIAMETER_MM: f64 = 42.67` (the Rules of Golf minimum).
    - **Colour classes**, per pixel: *white* when `min(r,g,b) >= WHITE_MIN` (200) and
      `max − min <= WHITE_SPREAD` (45); *light* when `min >= LIGHT_MIN` (140) and `max − min <=
      LIGHT_SPREAD` (60); *dark* when its luminance `0.299r + 0.587g + 0.114b` is at most
      `DARK_RATIO` (0.6) × the median luminance of the search region (below).
    - **Components** are 4-connected. Each carries area, centroid, bounding box, and its principal
      axis from second moments: `length` and `width` are the extents of its pixels projected on the
      axis and across it.
    - **Ball.** Per frame, white components whose equivalent diameter `2·sqrt(area/π)` lies in
      `[BALL_MIN_FRAC, BALL_MAX_FRAC]` (0.01, 0.06) × `min(width, height)`, bounding-box aspect in
      `[0.75, 1.33]`, and fill (area ÷ bounding-box area) in `[0.65, 0.92]`. Candidates are chained
      across consecutive frames when their centroids are within 0.5 × the candidate's diameter. **The
      ball is the longest chain, at least `BALL_MIN_FRAMES` (3) frames long**; ties go to the chain
      that starts first. Its centre and diameter `D` are the medians over the chain. The chain's last
      frame is `last_ball`. No chain → `NoBall`.
    - **Search region**: the square of side `12·D` centred on the ball, clipped to the frame.
    - **Club head**, per frame `0..=last_ball + 1`: light components in the search region, with the
      ball's disc (radius `0.55·D` round its centre) removed from the light mask in frames where the
      ball chain is present, whose `length` is in `[1.5·D, 5·D]` and `length / width >= 2.5`; per
      frame the candidate whose axis segment passes closest to the ball's centre. No candidate in
      any frame → `NoClubHead`.
    - **Contact frame**: among frames `0..=last_ball + 1` with a head, the one whose head segment
      passes closest to the ball's centre; ties to the earliest. That distance above `1.0·D` →
      `NoContact`.
    - **Heel and toe**: in the contact frame (or, when none is found there, the nearest earlier
      frame with a head), a dark component in the search region with `length >= 1.5·D` and `length /
      width >= 3`, one end of whose axis segment lies within `0.5·D` of one end of the head's; that
      head end is the **heel** (the hosel takes the shaft), the other the **toe**. None → `NoShaft`
      (never a default orientation: a guessed toe is a sign guessed, call 5's reason).
    - **Offset**: project the ball's centre on the heel→toe axis; `offset_px` = projection − the
      segment's midpoint, positive toward the toe; `offset_h_mm = round1(offset_px × BALL_DIAMETER_MM
      / D)`, `round1(x) = (x * 10.0).round() / 10.0`. `|offset_h_mm| > 60.0` (the validator's range
      for `impact_offset_h`) → `OutOfRange`. The segment's midpoint stands for the face centre; that
      is a stated approximation (the hosel lengthens the bar), not a measured one.
    - **Constants** are `pub const`s with these starting values, and P13 may tune **only these**,
      each within the bracket given here, recording before and after: `WHITE_MIN` [170, 230],
      `WHITE_SPREAD` [25, 70], `LIGHT_MIN` [110, 190], `LIGHT_SPREAD` [40, 90], `DARK_RATIO`
      [0.4, 0.8], `BALL_MIN_FRAC` [0.005, 0.02], `BALL_MAX_FRAC` [0.04, 0.10], `BALL_MIN_FRAMES`
      [2, 5], the head length bracket's ends [1.0, 2.0]·D and [4, 7]·D, the head aspect [2.0, 3.5],
      the contact distance [0.5, 1.5]·D, the shaft join [0.3, 1.0]·D. No new stage is designed in
      P13.
13. **A value read from the impact video counts as printed by the device** (the question ADR-034 §2
    left to M42), because the replay is the device's own output, photographed. `devices.json`'s
    field entry gains an optional `"from"`: `"screen"` (the default) or `"impact_video"`
    (`capability::FieldSource`). **Since decision 9 there is no go: P14b declares it whatever P13
    found.** Read "a photo" below as "a capture": a clip's shot is the one with a motion part the
    reader ran on, and a still's shot never lists the field. `full_swing` declares `{ "field": "impact_offset_h",
    "use": "analysed", "from": "impact_video", "note": … }`, last. **`impact_offset_v` is never
    declared** (high–low is invisible from above, finding 9). A shot's `fields_present` gains
    `impact_offset_h` when its photo has a motion part the reader ran on; the value is set when the
    reader answered, and left `None` with a warning (`"impact video: {refusal sentence}"`) when it
    refused, which `tile_refusals` names `misread` (retaking the Live Photo during the replay can
    help) with the detail `"Impact video: the Live Photo's motion was read, and no impact offset
    came of it"`. A photo with no motion part leaves the field out of `fields_present`: not printed,
    and nothing downstream (ADR-034 §2). A motion part ffmpeg cannot decode is a machine problem:
    warning `"impact video: could not be decoded: {error}"`, field not listed.
14. **A clip is imported on its own** *(rewritten after P9 by decision 7: the original found a
    still's motion part beside it, `motion_beside`, and nothing builds that now)*. `golf-lab
    import-video --device full_swing <clip-or-dir>... [--into-session S] [--dry-run]` takes `.mov`
    files (a directory is scanned one level deep for `mov` in any case, as `images_in` scans for
    images) and keeps `import-photos`' flags with their meaning. Per clip:
    - **Frames**: P13's `decode_frames(clip, None)`, the clip's own width, rotation applied.
    - **The grid frame**: one frame, chosen by the rule P14 measures (by default the sharpest: the
      largest sum of squared luminance differences to the right and lower neighbours, ties to the
      earliest, `impact::sharpest_frame`). It is written as a PNG and read through the **same**
      `PhotoReader` and `screen::read` a still goes through, so the profile, the four parser rules
      and the validator are the still's, unchanged. The frames before the still's moment may show
      the previous shot's numbers; P14's gate would see that as a wrong value, and the frame rule is
      where it is fixed.
    - **The impact offset**: `impact::read_impact` on `decode_frames(clip, Some(960))`, with call
      13's three outcomes (read, refused, undecodable).
    - **Storage**: the grid frame's PNG is the entry's photo (`shot_screen.<digest12>.png`, its
      digest taken as for any photo, so importing one clip twice lands on one entry. The decode and
      the PNG encoding must be deterministic, and a test pins it). Its mtime is the clip's, which is
      the shot's timestamp (`ocr::screen_timestamp`). With `--into-session` the clip is copied beside
      it as `shot_screen.<digest12>.mov`, mtime preserved, **not** listed in the manifest (no new
      `Role`: frozen Python reads manifests and would refuse an unknown role, ADR-035 clause 4); a
      deleted entry takes it with it.
    - A device that declares no `impact_video` field refuses `import-video` (exit 2, naming the
      devices that do). A clip ffmpeg cannot decode fails that clip alone, as an unreadable photo
      does.
    `import-photos` is unchanged: it never looks beside a still for its clip (finding 12 already
    keeps `.MOV` files out of its scan).
15. **No go/no-go** *(rewritten after P9 by decision 9: the original ran P14 only if P13 read at
    least one clip)*. P13 runs, tunes and records; P14 and P14b run whatever it found, and P14b
    declares `impact_offset_h` from `impact_video` on `full_swing`. If P13 reads no clip, every clip
    import stores the offset as `misread` until the reader is tuned further, and P15 says so in the
    ADR-034 addendum.
16. **`CENTERED_RADIUS_MM` stays 10 and provisional.** M42 was to revise it from the reader's
    measured error; with no ground truth (decision 3) there is no error to measure. P15 says so in its
    doc comment (`crates/contracts/src/strike.rs`) and in the ADR-034 addendum.
17. **Nothing is imported into `data/processed/`.** P16's report prints the commands that import the
    grid captures store-only, one method per Live Photo (decision 7): the six stills
    (`golf-lab import-photos --device full_swing <the six>`) or the five clips (`golf-lab
    import-video --device full_swing <the five .MOV>`), for the user to run after reviewing; the
    clubs are unknown (the sidecars say `null`), so not into a session.
18. **One commit, at P16**, onto `main`, as M35 and M37 did.
19. **`golf-mcp` shows the two new fields**: `crates/mcp/src/query.rs::NUMERIC_METRICS` appends
    `backspin` and `sidespin` after `carry_offline` (its unit test fails on a `ShotData` reading the
    list lacks). No stored shot holds either, so no recorded answer moves.

---

## Phases

### P0 — This plan document; the six Live Photos copied with their sidecars; the pointers

- **Goal**: the plan in the repo; the photos where every later phase expects them; ROADMAP pointing
  here.
- **Read first**: this file; `data/raw/shot_screens/2026-10-07-1.json`; ROADMAP's status table rows
  for M41 and M42 and the two sections' `**Status**` lines (grep `^## M41`, `^## M42`).
- **Do**:
  1. This file at `docs/plans/m42-full-swing.md` (written in the planning session; P0 ticks its row).
  2. Copy the twelve files from `C:\Users\aaron\Desktop\pictures` into `data/raw/shot_screens/` with
     `cp -p` (mtime preserved), and check each digest against "The photos".
  3. Beside each pair, `IMG_NNNN.json` (LF, two-space indent), the shape of `2026-10-07-1.json`:
     `{"image": "IMG_NNNN.HEIC", "motion": "IMG_NNNN.MOV", "player_id": "aaron", "club": null,
     "device": "full_swing", "layout": "grid"` (`"club_view"` for `IMG_2888`) `, "received":
     "2026-10-30", "taken": "2026-10-10", "note": "…"}`. The note says which layout it is, that the
     club is unknown, and for `IMG_2888` that M42 does not read the club view (decision 1).
  4. In `2026-10-07-1.json`, add `"device": "full_swing"` and append to its note: "M42 (2026-10-30):
     this is Full Swing's grid screen; M41 is folded into M42."
  5. ROADMAP: the status-table rows for M41 (`➡️ Folded into M42 (2026-10-30)`, linking this file)
     and M42 (`🔄 Planned 2026-10-30, 17 phases — docs/plans/m42-full-swing.md`), and each section's
     `**Status**` line to the same effect. Nothing else in ROADMAP (P16 does the rest).
  6. The `data/processed/` digest, compared with the rules' value.
- **Done when**: the twelve files are there with matching digests and mtimes; seven sidecars
  (six new, one edited); ROADMAP points here; docs-truth is green; the digest matches.
- **Verify**: `sha256sum data/raw/shot_screens/IMG_*`; `ls -l --time-style=full-iso` against the
  source; `pytest tests/test_docs_truth.py`; the digest command.

### P1 — `ShotData` gains `backspin` and `sidespin`, written only when set

- **Goal**: decision 4, calls 1, 2 and 19. The shot record can hold Full Swing's two spin tiles, and
  nothing committed moves.
- **Read first**: `crates/contracts/src/shot.rs` (whole); `crates/contracts/tests/capability.rs:60–100`
  and `:425–470` (`shot_data_keys`, `NOT_READINGS`, the synthetic test);
  `crates/contracts/tests/schemas.rs:1–60` and `:100–200`; the `synthetic` entry in
  `crates/contracts/devices.json`; `crates/mcp/src/query.rs:620–680`;
  `crates/screen/src/validate.rs:60–100`.
- **Files**:
  - `crates/contracts/src/shot.rs` — after `carry_offline`, a comment block ("Added in M42, in Rust
    only, and written only when set: the M42 plan's call 1 …") and the two fields:
    `#[serde(default, skip_serializing_if = "Option::is_none")] pub backspin: Option<f64>` (doc:
    "rpm, as printed (Full Swing's `BACKSPIN RPM`)") and the same for `sidespin` (doc: "rpm, + =
    fade, − = draw: Full Swing's `SIDESPIN RPM`, signed by which of its `D` and `F` is lit").
    `pub const WRITTEN_WHEN_SET: [&str; 2] = ["backspin", "sidespin"];` with a doc saying why
    (call 1) and that a default serialization omits them. Unit tests: a Python-shaped shot reads both
    as `None`; a shot with both round-trips with both written; a shot with neither serializes with
    neither key; `WRITTEN_WHEN_SET` names exactly the struct's skip-when-`None` fields (assert by
    serializing a shot with both set and both unset and diffing the key sets).
  - Every `ShotData { … }` struct literal in the workspace (find with
    `grep -rn "ShotData {" crates --include=*.rs`) gains `backspin: None, sidespin: None`.
  - `crates/contracts/tests/capability.rs::shot_data_keys` and
    `crates/contracts/tests/schemas.rs::rust_keys` — the default serialization's keys ∪
    `WRITTEN_WHEN_SET`; each doc says why (rewrite `rust_keys`' paragraph that anticipates this).
  - `crates/contracts/devices.json` — `synthetic` gains `backspin` and `sidespin`, `analysed`, after
    `carry_offline` (struct order); its `source` sentence gains "and M42's two spin readings".
  - `spec/schemas/{shot_data,swing_result,swing_bundle_result,shot_result}.schema.json` — every copy
    of the `ShotData` shape gains the two properties, in the style of that file's `carry_offline`
    entry (title `Backspin` / `Sidespin`, the same `anyOf` number-or-null and default null, a
    description carrying the unit and sign). `every_copy_of_each_shot_shape_is_identical` holds the
    copies equal.
  - `crates/mcp/src/query.rs` — `NUMERIC_METRICS` gains `("backspin", |s| s.backspin)` and
    `("sidespin", |s| s.sidespin)` after `carry_offline`; its doc gains "M42's two after M32's, for
    the same reason: no stored shot holds either, so no recorded answer moves."
  - `crates/screen/src/validate.rs` — `PLAUSIBLE_RANGES` gains `("backspin", 0.0, 15000.0)`,
    `("sidespin", -6000.0, 6000.0)`, `("apex_height", 0.0, 80.0)`, each with a one-line comment
    (call 2).
- **If** a test outside these files fails because it lists `ShotData`'s keys or readings: give it the
  union with `WRITTEN_WHEN_SET` (if it derives keys by serializing) or the two names in struct order
  (if it lists them), and record which as a finding for P15.
- **If** `tests/test_conformance.py` objects to the schema edits: the rules' one exception.
- **Done when**: `cargo test` green; clippy and fmt clean; `pytest` green; `conformance.py check`
  passes; the three dry runs report 0 changed; `git diff --stat -- spec/vectors` is empty.
- **Verify**: as "Done when", plus `git diff -- src/ scripts/ tests/` empty.

### P2 — `TextBox.rgb`: the glyph colour, measured by `crates/ocr`; `golf-ocr read --boxes`

- **Goal**: call 4's first half. Every box the lab's reader returns carries its glyph colour, and a
  person (and P9, P10) can see the boxes a photo reads as.
- **Read first**: `crates/screen/src/lib.rs:73–112` (`TextBox`); `crates/ocr/src/recognizer.rs`
  (whole); `crates/ocr/src/prepare.rs:55–130` (where `recognize` runs per rotation);
  `crates/ocr/src/bin/golf_ocr.rs` (whole).
- **Files**:
  - `crates/screen/src/lib.rs` — `TextBox` gains `#[serde(default)] pub rgb: Option<[f64; 3]>`
    with call 4's definition as its doc, and derives `Serialize` (for `--boxes`). Every `TextBox {`
    literal in the workspace (`grep -rn "TextBox {" crates --include=*.rs`) gains `rgb: None` except
    the recognizer's.
  - `crates/ocr/src/recognizer.rs` — `pub fn glyph_rgb(image: &RgbImage, x: f64, y: f64, width:
    f64, height: f64) -> Option<[f64; 3]>`, exactly call 4's definition; `recognize` sets each box's
    `rgb` from the image it was recognized in. Unit tests: a dark box holding yellow strokes reads
    yellow within 1.0 per channel; a box entirely outside the image is `None`; a box straddling the
    edge is clipped; ties in brightness keep the stable order (a fixed image's answer pinned).
  - `crates/ocr/src/bin/golf_ocr.rs` — `--boxes`: print `{"device": …, "boxes": […], "notes": […]}`
    (pretty JSON, the prepared screen's boxes and notes, each box with its `rgb`) instead of the shot,
    so the output can be pasted into a screen vector's `input`. Usage text and module doc updated.
- **Done when**: `cargo test` green; clippy and fmt clean; the 13-photo OCR gate passes unchanged;
  `golf-ocr read --device hd_golf --boxes` on `data/raw/shot_screens/IMG_2889.HEIC` prints an `R`
  box whose `rgb` matches finding 4 within 3.0 per channel (record what it printed).
- **Verify**: as above; screen vectors untouched.

### P3 — The screen-quad finder refuses a quad whose ordering repeats a corner

- **Goal**: finding 5. A four-gon that `order_quad` cannot order is not a screen.
- **Read first**: `crates/ocr/src/quad.rs` (whole); `crates/ocr/tests/quad.rs` (its pins on the bay
  photos).
- **Files**: `crates/ocr/src/quad.rs` — in `find_screen_quad`, where `as_quad` accepts a candidate:
  order it (`order_quad(divide(quad, scale))`) and, if any two of the four ordered corners are equal,
  treat the candidate as if `as_quad` had refused it (the loop continues, and the contour's rotated
  box may still become the fallback). Likewise, a rotated box whose ordering repeats a corner is not
  kept as the fallback. A doc paragraph: the defect, the two photos, that frozen Python has it too
  and keeps it (ADR-035 clause 4), and that the change cannot reach a photo whose quad orders cleanly.
  Unit tests: `order_quad` on a 45° diamond repeats a corner (the defect, pinned); a synthetic image
  of a dark 45° diamond on a light wall gives `None` or a quad with four distinct corners.
- **If** any of the 13 bay photos' quads changes (`tests/quad.rs` or the OCR gate fails): **stop** —
  call 3 rests on it not happening.
- **Done when**: `cargo test` green; the OCR gate passes unchanged; on `IMG_2893.HEIC` and
  `IMG_2894.HEIC`, `golf-ocr read --device hd_golf --boxes` reports boxes containing tile labels
  (record the quad source, or "uncropped", and the box count for each).
- **Verify**: as above.

### P4 — Parser rule 1: a box holding two merged labels is split (opt-in)

- **Goal**: call 6.
- **Read first**: `crates/screen/src/profile.rs` (whole); `crates/screen/src/parser.rs:1–210` and
  `:306–440` (`parse_screen`, `despace`, `assign_labels`).
- **Files**:
  - `crates/screen/src/profile.rs` — `DeviceProfile` gains `#[serde(default)] pub
    split_merged_labels: bool`, documented.
  - `crates/screen/src/parser.rs` — `fn split_merged(boxes: &[TextBox], profile: &DeviceProfile) ->
    Vec<TextBox>` (call 6, a module-doc paragraph on why and on the proportional split being an
    approximation), run first in `parse_screen` when the profile opts in; every later step sees the
    returned list.
  - Unit tests with made-up profiles (`parse_profiles`): `CLUBSPEEDMPHBALLSPEEDMPH` splits into two
    located labels with the cut's widths; a box that already matches a label is never split; a cut
    whose halves name the same field does not qualify; the tie on `min` goes to the smallest `k`; a
    profile without the flag leaves the box whole (and, as today, unlocated).
- **Done when**: `cargo test` green (every screen vector unchanged); clippy and fmt clean.

### P5 — Parser rule 2: a tile printed twice must agree with itself (opt-in)

- **Goal**: call 7.
- **Read first**: `crates/screen/src/parser.rs:340–600` (`assign_labels`, `ties`, `decide`,
  `assignments`) and `:626–760` (`build_cells`, `read_cell`); P4's finding.
- **Files**: `crates/screen/src/profile.rs` — `ProfileField` gains `#[serde(default)] pub
  printed_twice: bool`. **The load-time check** this and P6–P7 extend: `impl DeviceProfile { pub fn
  check(&self) -> Result<(), String> }`, which `profiles()` runs on every shipped profile inside its
  `OnceLock` (panicking with the message, as its `expect` already does on a parse failure) and which
  unit tests call on made-up profiles. P5's clause: `printed_twice` only on a stored number field. `crates/screen/src/parser.rs` — the field takes up to two boxes in the
  assignment, the two slots interchangeable (call 7); `read_cell`'s result for the field combines the
  two cells by call 7. Unit tests: two equal cells store the value and join `raw_fields`; two unequal
  cells store nothing and warn with call 7's sentence; one cell stores its value; the field's two
  boxes never make a tie with each other; a third box matching the label makes a real tie, withheld as
  today; a field without the flag and two boxes behaves exactly as today (`carry-twice`'s case).
- **Done when**: `cargo test` green (every screen vector unchanged); clippy and fmt clean.

### P6 — Parser rule 3: the sign read from a lit letter (opt-in)

- **Goal**: call 4's second half and call 5.
- **Read first**: `crates/screen/src/parser.rs:688–760` (`read_cell`), `:867–912` (`sign_from`);
  `crates/screen/src/profile.rs:40–80`; P2's finding (the `rgb` P2 printed).
- **Files**: `crates/screen/src/profile.rs` — `pub struct LitToken { pub token: String, pub sign:
  i64 }`; `ProfileField` gains `#[serde(default)] pub lit_sign: Vec<LitToken>`; load-time checks:
  empty or exactly two tokens with signs +1 and −1, and never beside `sign_tokens` or
  `printed_sign`. `crates/screen/src/parser.rs` — `pub const LIT_MIN_BRIGHTNESS: f64 = 120.0`,
  `pub const LIT_BLUE_RATIO: f64 = 0.7` (each doc carrying finding 4's table), `pub fn is_lit(rgb:
  [f64; 3]) -> bool`, and `read_cell`'s path for a `lit_sign` field per call 5: the magnitude from
  `first_number` (its printed sign, if any, discarded), the sign from the cell's letter boxes. Unit
  tests: every row of finding 4's table classifies as its "lit" column; each branch of call 5 (one
  lit, one unlit, both one-lit, both lit, both unlit, neither, a letter with `rgb: None`, a zero
  magnitude); a field without `lit_sign` is untouched.
- **Done when**: `cargo test` green (every screen vector unchanged); clippy and fmt clean.

### P7 — Parser rule 4: a value printed in feet is stored in yards (opt-in)

- **Goal**: call 8.
- **Read first**: `crates/screen/src/profile.rs:40–80`; `crates/screen/src/parser.rs:688–760`.
- **Files**: `crates/screen/src/profile.rs` — `#[serde(rename_all = "lowercase")] pub enum
  PrintedUnit { Feet }`; `ProfileField` gains `#[serde(default)] pub printed_unit:
  Option<PrintedUnit>`; a load-time check that it sits only on call 8's yard targets.
  `crates/screen/src/parser.rs` — `read_cell` divides by 3.0 after the sign is applied. Unit tests:
  `85.2` reads as `85.2 / 3.0` bit for bit and `raw_fields` keeps `"85.2"`; the check refuses a
  degrees target; a field without it is untouched.
- **Done when**: `cargo test` green (every screen vector unchanged); clippy and fmt clean.

### P8 — The `full_swing` profile and its `devices.json` declaration

- **Goal**: the device exists on both sides of the capability pin, using every rule P4–P7 built.
- **Read first**: `crates/screen/profiles.json`; `crates/screen/tests/capability.rs` (whole);
  `crates/screen/tests/profile_fork.rs` (whole); `crates/screen/src/parser.rs`'s test
  `only_the_impact_position_pair_overlaps`; `crates/contracts/devices.json`;
  `crates/contracts/src/capability.rs:40–130`; calls 9–11; P4–P7's findings.
- **Files**:
  - `crates/screen/profiles.json` — a second profile, after `hd_golf`, exactly:

    ```json
    {
      "_comment": ["Full Swing's grid screen (M42). Labels carry their units; aliases without the unit cover the reads that drop it (the M42 plan's finding 2). Path and face print their own sign, a minus being left, which for a right-handed golfer is ShotData's convention, so neither has a sign rule (finding 8). Total Yd is printed twice (call 7), Apex in feet (call 8), and ball direction and sidespin are signed by a lit letter (calls 4, 5)."],
      "device": "full_swing",
      "title": "SHOT DATA",
      "value_span_ratio": 6.0,
      "split_merged_labels": true,
      "blank_markers": ["---", "--", "—", "- - -"],
      "fields": [
        { "label": "Shot Result", "target": null, "kind": "text" },
        { "label": "Total Yd", "aliases": ["Total"], "target": "total_distance", "kind": "number", "unit": "yards", "printed_twice": true },
        { "label": "To Hole Yd", "aliases": ["To Hole Ft", "To Hole"], "target": null, "kind": "text" },
        { "label": "Carry Yd", "aliases": ["Carry"], "target": "carry_distance", "kind": "number", "unit": "yards" },
        { "label": "Club Speed Mph", "aliases": ["Club Speed"], "target": "club_head_speed", "kind": "number", "unit": "mph" },
        { "label": "Ball Speed Mph", "aliases": ["Ball Speed"], "target": "ball_speed", "kind": "number", "unit": "mph" },
        { "label": "Path Deg", "aliases": ["Path"], "target": "club_path", "kind": "number", "unit": "degrees" },
        { "label": "Face Deg", "aliases": ["Face"], "target": "club_face_angle", "kind": "number", "unit": "degrees" },
        { "label": "Ball Direction Deg", "aliases": ["Ball Direction"], "target": "launch_direction", "kind": "number", "unit": "degrees",
          "lit_sign": [ { "token": "L", "sign": -1 }, { "token": "R", "sign": 1 } ] },
        { "label": "Launch Angle", "target": "launch_angle", "kind": "number", "unit": "degrees" },
        { "label": "Apex Ft", "aliases": ["Apex"], "target": "apex_height", "kind": "number", "unit": "yards", "printed_unit": "feet" },
        { "label": "Smash Factor", "target": "smash_factor", "kind": "number", "unit": "ratio" },
        { "label": "Backspin Rpm", "aliases": ["Backspin"], "target": "backspin", "kind": "number", "unit": "rpm" },
        { "label": "Sidespin Rpm", "aliases": ["Sidespin"], "target": "sidespin", "kind": "number", "unit": "rpm",
          "lit_sign": [ { "token": "D", "sign": -1 }, { "token": "F", "sign": 1 } ] }
      ]
    }
    ```
  - `crates/contracts/devices.json` — the `full_swing` entry of call 11; the top `_comment` gains one
    line naming it. `crates/contracts/src/capability.rs` — `FieldSource { Screen, ImpactVideo }`
    (`#[serde(rename_all = "snake_case")]`) and `DeclaredField.from: FieldSource` with
    `#[serde(default)]` (`Screen`), documented by call 13, though nothing declares `impact_video`
    until P14. `crates/contracts/tests/capability.rs` gains a test that `full_swing` declares call
    11's fields in that order with those uses.
  - `crates/screen/tests/capability.rs` — generalized from `const DEVICE` to every device that has
    a profile: the fields it declares with `from: screen` equal the profile's targets as a set and in
    tile order. Its module doc says so.
  - `crates/screen/tests/profile_fork.rs` — `DECLARED_DEVICES: &[&str] = &["full_swing"]`, devices
    the fork has and the frozen copy does not, compared nowhere else; the doc paragraph on "a second
    entry wants a decision of its own" cites the M42 plan's decision 1 as that decision.
  - `crates/screen/src/parser.rs` tests — `full_swing_labels_do_not_overlap`: no label or alias of
    one `full_swing` field scores ≥ `LABEL_MATCH_THRESHOLD` against another field's label or alias.
    `full_swing_uses_every_rule` (call 9).
  - `crates/screen/src/tiles.rs` test `every_declared_field_has_its_tile` — for every device with a
    profile, over its `from: screen` fields.
  - `crates/contracts/src/shot.rs` — one sentence under ledger entry 1 (call 3): "M42 added the
    `full_swing` profile and four opt-in parser rules, none of which changes a read with `hd_golf`'s
    profile, so no entry: every stored shot's parse is as current as it was."
- **If** the overlap test fails: drop the alias that overlaps (never a label) and record it.
- **Done when**: `cargo test` green (every screen vector unchanged); clippy and fmt clean; the
  screen dry run reports 0 changed; `golf-ocr read --device full_swing data/raw/shot_screens/IMG_2889.HEIC`
  prints a shot (record it, against the truth table).

### P9 — The Full Swing photo gate on the six grid photos, and the profile's tuning

- **Goal**: "the pictures good", measured: the shipping reader on the six real grid photos against
  the truth table.
- **Read first**: `crates/ocr/tests/read.rs` (the 13-photo gate, the pattern to follow); the truth
  table; P2, P3 and P8's findings.
- **Files**: `crates/ocr/tests/full_swing.rs` (new) — one `#[ignore = "needs the six Full Swing
  photos in data/raw/shot_screens/, ffmpeg and the ONNX models in data/models/ocr/"]` test,
  `every_grid_photo_reads_as_it_prints_or_withholds`. For each truth-table photo: check its digest,
  `ocr::read_photo` with `load_profile("full_swing")`, `screen::read` with `min_confidence` 0.6, then
  per stored field: **equal to the truth** (exact `f64` equality; apex as `printed / 3.0`) → read;
  **`None`** → withheld; **anything else → wrong**. The test fails on any wrong value, listing all of
  them, and prints (`eprintln!`) a table of read/withheld per photo and field with each photo's
  `needs_review` and warnings. Read/withheld does not fail it.
- **Tuning** — allowed, in this order, until no value is wrong and at least 66 of the 72 field reads
  are read, or the list is exhausted: (a) aliases and `blank_markers` of `full_swing`; (b) its
  `value_span_ratio` within [4.0, 8.0]; (c) a new optional per-profile `row_tolerance` in
  `profiles.json` (`DeviceProfile.row_tolerance: Option<f64>`, `None` = today's `ROW_TOLERANCE` 0.7,
  so `hd_golf` is unchanged), within [0.5, 1.2]; (d) `LIT_BLUE_RATIO` within [0.6, 0.8] and
  `LIT_MIN_BRIGHTNESS` within [100, 160], only if a letter's colour on these photos sits outside
  finding 4's range. Nothing else changes. Each tuning step is recorded with the coverage before and
  after.
- **If**, with the list exhausted, a value is still wrong: make the field withhold in that case (the
  narrowest rule that does, opt-in to `full_swing`) rather than store it, and record it. **If**
  coverage stays below 66: record which tiles on which photos, and why, and continue — it does not
  block (the target is a target).
- **Done when**: the photo gate passes (no wrong value); coverage and `needs_review` per photo are in
  the findings; `cargo test` green (every screen vector unchanged); the OCR gate passes unchanged;
  the `data/processed/` digest matches.

### P10 — Hand-worked Full Swing screen vectors

- **Goal**: the new rules pinned on real boxes, where `cargo test` runs without the models.
- **Read first**: `crates/screen/tests/hand.rs` (whole); two of `spec/vectors/screen/hand/`
  (`carry-twice.json`, `impact-pair-exact.json`) for shape, `provenance` and `note` style;
  `docs/CONFORMANCE.md` §2's screen section; P9's findings.
- **Files**: three new vectors in `spec/vectors/screen/hand/`, each `screen_parser_version: 1`,
  `provenance.oracle: "hand"`, `kind: "screen"`, a `source` ("worked by hand in M42 P10 from `golf-ocr
  read --device full_swing --boxes` of <photo> (sha256 …), edited only as the note says; Rust's
  answer (`screen::read`) was checked equal to this expected before the file was committed") and a
  `note` that works every value, the confidence arithmetic, `label_ratio` and every warning, as the
  M34 cases do:
  1. `full-swing-grid.json` — `IMG_2889`'s boxes as P9's tuned reader prints them, unedited: the
     header `TOTAL YD` read twice in agreement, `R` lit, `D` lit, apex in yards.
  2. `full-swing-merged-unlit.json` — `IMG_2891`'s boxes, unedited: the two merged label boxes
     split, ball direction `L` from an unlit `R`, sidespin `D` from an unlit `F`.
  3. `full-swing-withheld.json` — `IMG_2889`'s boxes edited three ways, each in the note: the header
     `134` changed to `138` (total withheld, call 7's warning), the `R` box removed (ball direction
     withheld, call 5's warning), and the `85.2` box's text changed to `---` (apex a printed blank).
  `crates/screen/tests/hand.rs` — `CASES` gains the three; `the_seven_cases_are_the_plans` becomes
  `the_cases_are_the_plans` with its doc naming both plans.
- **How to work them**: by hand, with a throwaway scratchpad script allowed for the arithmetic
  (deleted after); then run Rust. **If** Rust disagrees with the hand-worked value: decide by calls
  4–8 which is wrong; fix the code if the code breaks a call, the vector if the working erred; record
  which.
- **Done when**: `cargo test` green; the screen dry run reports 0 changed (the new files are gated,
  not rewritten); `git diff --stat -- spec/vectors` shows exactly the three additions.

### P11 — `crates/impact` I: frames, and the ball

- **Goal**: call 12's crate, up to the ball.
- **Read first**: call 12; `Cargo.toml` (workspace members); `crates/ocr/tests/ort_reach.rs:15–35`.
- **Files**: `crates/impact/Cargo.toml` (package `impact`, workspace edition, rust-version and
  license, no dependencies, a description); `crates/impact/src/lib.rs` (module doc: what it reads,
  the Live Photo, no Python twin, the stated approximations, the phone as the later caller);
  `crates/impact/src/{frame,mask,component,ball}.rs` as the code needs; the workspace `members`
  gains `crates/impact`; `ort_reach.rs`'s `GUARDED` gains `"impact"`.
  `Frame`, the colour classes, 4-connected components with moments, the ball candidates and chain,
  and a crate-private `fn find_ball(frames) -> Result<Ball, ImpactRefusal>`; `read_impact` exists
  and returns `NoBall` / `NoFrames` / (for now) `NoClubHead` past the ball.
- **Tests** (`crates/impact/tests/ball.rs`), on synthetic frames drawn in the test (a dark green
  field (60, 80, 50), a white disc, noise-free): a disc stationary in frames 0–5 then gone is found
  with centre within 0.5 px and diameter within 1 px, and `last_ball` = 5; a moving disc (more than
  0.5·D a frame) makes no chain of 3; a white square is not a ball; two discs, the longer chain wins;
  no frames → `NoFrames`; no disc → `NoBall`.
- **Done when**: `cargo test` green; clippy and fmt clean.

### P12 — `crates/impact` II: the club head, the shaft, the contact frame, the offset

- **Goal**: call 12, to the end.
- **Read first**: call 12; P11's code and finding.
- **Files**: `crates/impact/src/{head,shaft,contact}.rs` as needed; `read_impact` complete;
  `ImpactRefusal::sentence`.
- **Tests** (`crates/impact/tests/impact.rs`), synthetic: a ball of D = 24 px; a light bar 60×10 px
  (the head) approaching over frames, its axis at 30°, a dark bar 120×6 px joined at one end (the
  shaft); at contact the ball's centre projects 6.0 mm toward the toe of the bar's midpoint → reading
  `offset_h_mm` within 0.5 of 6.0 and `contact_frame` the drawn one; the same toward the heel → −6.0;
  the shaft moved to the other end flips the sign; no bar → `NoClubHead`; the bar stopping 2·D short
  → `NoContact`; no shaft → `NoShaft`; a projection past 60 mm → `OutOfRange`. Every refusal's
  sentence is pinned.
- **Done when**: `cargo test` green; clippy and fmt clean.

### P13 — `golf-lab read-impact` on the five Live Photo clips; tuning

*Amended after P9 by decisions 7–9: arguments are clips, `decode_frames` takes an `Option` width, and
nothing is decided here.*

- **Goal**: call 12 on real frames, and review images for the user.
- **Read first**: `crates/lab/src/ffmpeg.rs` (whole); `crates/lab/src/bin/golf_lab.rs:160–260` and
  one existing verb's parse/run pair (`parse_flight`/`run_flight`) for the pattern;
  `crates/lab/Cargo.toml`; finding 9; P11–P12's findings.
- **Files**:
  - `crates/lab/Cargo.toml` — `impact = { path = "../impact" }` and the `image` dependency the rules
    allow, each with the comment the manifest's style asks for.
  - `crates/lab/src/ffmpeg.rs` — `pub fn decode_frames(&self, clip: &Path, width: Option<u32>) ->
    Result<Vec<impact::Frame>, String>`: `ffmpeg -hide_banner -nostdin -loglevel error -i <clip>
    -map 0:v:0 -vf scale=<width>:-2 -f image2pipe -vcodec ppm -` (no `-vf` when `width` is `None`:
    the clip's own size, for P14's grid frame), parsing the stream of binary PPM (`P6`) images, each
    header's width and height read from it. The impact reader always asks for `Some(960)`.
  - `crates/lab/src/read_impact.rs` (new) — the verb: `golf-lab read-impact <clip>... [--json]
    [--review DIR]`, each argument a Live Photo clip (`.mov`, any case); a still is a usage error
    naming `import-photos`. Per argument, one line: `<name>: impact offset +6.3 mm (toe), contact
    frame 21 of 36, ball 20.4 px` or `<name>: not read - <refusal sentence>`; `--json` prints the
    reading's fields or `{"refusal": …}` per argument. `--review DIR` writes, per argument, the
    contact frame and the two frames either side (or, on a refusal, every fifth frame) as PNGs under
    `DIR/<stem>/`, with the ball's disc outlined, the head segment drawn and the toe end marked.
    Exit 0 when every argument was read, 1 when any refused or could not be decoded, 2 on usage.
    `crates/lab/src/lib.rs` registers the module; `golf_lab.rs` dispatches `read-impact` and its
    usage line.
  - A unit test of the PPM parser on two hand-built frames, and of the verb's line format on a fake
    reading.
- **Run**: `golf-lab read-impact --review data/raw/impact-review data/raw/shot_screens/IMG_2889.MOV
  …IMG_2894.MOV` (the five grid Live Photos' clips). Record each line. Then tune per call 12, only its
  constants and only within its brackets, re-running after each change, until all five read or the
  brackets are exhausted; record each change and its effect. Update the constants' docs with what
  they were tuned on.
- **Decide nothing** (decision 9, call 15): the first line of this phase's findings is how many of
  the five clips read, addressed to P14b and P15, who say it; P14b wires the reader in either way.
- **Done when**: `cargo test` green; clippy and fmt clean; the five results, the tuning record and
  the review images' path are in the findings.

### P14 — The grid read off a clip's frame, and the Full Swing clip gate

*Rewritten after P9 by decisions 7–10. The impact wiring that was P14 is P14b.*

- **Goal**: decision 7's numbers half: a clip's grid read the way a still's is, with no wrong value.
- **Read first**: decisions 7–10; call 14; `crates/lab/src/shot.rs:440–640` (`read_screen`,
  `PhotoReader`); `crates/ocr/tests/full_swing.rs` (P9's gate, whole: the clip gate copies its form
  and its truth table); `crates/lab/src/ffmpeg.rs`'s `decode_frames`; the truth table; P9's and
  P13's findings.
- **Files**:
  - Beside `impact::Frame` (where P11 put it) — `pub fn sharpest_frame(frames: &[Frame]) ->
    Option<usize>` by call 14's definition, with tests on synthetic frames (a blurred copy loses to
    the sharp one; ties go to the earliest; no frames → `None`). It lives in `impact` because the
    phone (M38) needs it beside `read_impact`.
  - `crates/lab/src/shot.rs` — `pub fn read_clip_grid(…)`: decode at `None`, choose the frame by the
    rule kept below, write it as a PNG at a path the caller gives, and read that PNG through
    `read_screen`'s own path; returns the shot read and the chosen frame's index. Pass the reader as
    `PhotoReader` is passed.
  - `crates/lab/tests/full_swing_clips.rs` — the clip gate, `#[ignore]`d as P9's is (it needs the
    models, ffmpeg and the clips): the five grid clips (`IMG_2889`, `IMG_2891`–`IMG_2894`, `.MOV`)
    through `read_clip_grid` with `full_swing`, every value against the truth table, **no wrong
    value**, the coverage asserted at what this phase measures.
- **Measure** the frame rule: the sharpest frame and the middle one (`len / 2`), each on the five
  clips. Keep the rule with no wrong value and more values read (the sharpest when equal), and record
  both counts.
- **Tune** only on a wrong value: the profile keys P9 could tune, within P9's bounds, with P9's photo
  gate re-run green after every change. A value still wrong after that is **`BLOCKED`** (decision
  10): name the clip, the field, the value read and the truth.
- **Done when**: `cargo test` green; clippy and fmt clean; the clip gate passes with no wrong value;
  P9's photo gate still passes; both frame rules' counts and the gate's coverage are in the findings.

### P14b — `golf-lab import-video`: the second import method

*New after P9 (decisions 7 and 9). It carries the old P14's impact wiring, moved from the still to
the clip.*

- **Goal**: a clip imports a shot holding the grid's numbers and the impact offset, or the offset's
  named refusal, and nothing needs the still.
- **Read first**: decisions 7–9; calls 13–14; `crates/lab/src/import.rs:1–60` and `:600–700`
  (opening an entry) and `images_in`; `crates/lab/src/bin/golf_lab.rs`'s `import-photos` parse/run
  pair; `crates/screen/src/tiles.rs` (whole); `crates/contracts/src/capability.rs`; P13's and P14's
  findings.
- **Files**:
  - `crates/contracts/devices.json` — `full_swing` gains `{ "field": "impact_offset_h", "use":
    "analysed", "from": "impact_video", "note": "Read from Full Swing's overhead impact replay in a
    Live Photo's clip, imported by golf-lab import-video, by crates/impact (the M42 plan's call 12): the ball's centre
    against the club head's heel-to-toe axis, scaled by the ball's diameter. Never validated against
    a mark on the face (decision 3); heel-toe only, high-low being invisible from above." }`, last.
  - `crates/screen/src/tiles.rs` — a declared field with `from: impact_video`, printed and analysed
    and holding no value, is `misread` with call 13's detail, without looking for a tile; the module
    doc's table gains the row.
  - `crates/contracts/src/capability.rs` — `FieldSource` (`screen`, the default, and
    `impact_video`) by call 13, the field's absence read as `screen`, unless an earlier phase added
    it.
  - `crates/lab/src/shot.rs` — a `pub trait MotionFrames { fn frames(&mut self, clip: &Path, width:
    Option<u32>) -> Result<Vec<impact::Frame>, String>; }` with the ffmpeg implementation
    (`decode_frames`) and a fake for tests, which `read_clip_grid` takes too. After the grid read,
    when the device declares an `impact_video` field, call 13's three outcomes (read, refused,
    undecodable) on `read_impact` over the frames at `Some(960)`.
  - `crates/lab/src/import.rs` — `import_video` beside `import_photo`, sharing how an entry is
    opened, and a `clips_in` beside `images_in`; the clip copied beside the entry's photo (call 14);
    the module doc's step list says so.
  - `crates/lab/src/bin/golf_lab.rs` — `import-video` dispatched, with its usage line.
  - Tests: a fake reading sets `impact_offset_h` and lists it; a fake refusal lists it, leaves it
    `None`, warns, and `tile_refusals` names it `misread`; an undecodable clip fails that clip alone;
    `import-video --device hd_golf` exits 2; a still never asks for frames, and `import-photos` on a
    directory holding both halves of a Live Photo imports the still alone; `import-video
    --into-session` on a scratch tree stores the frame's PNG as the entry's photo with the clip
    beside it, and the shot holds the fake offset; one clip imported twice lands on one entry.
- **Done when**: `cargo test` green; clippy and fmt clean; P9's photo gate and P14's clip gate still
  pass; `golf-lab import-video --device full_swing --dry-run data/raw/shot_screens/IMG_2889.MOV` runs
  (record its output, the offset or its refusal with it); the `data/processed/` digest matches (no
  real import is run).

### P15 — Docs

- **Goal**: every doc that describes what changed says so, and docs-truth is green.
- **Read first**: every phase's findings (each "For P15"); `docs/README.md`'s map rows for the ADRs
  touched; the addendum style of ADR-034's and ADR-014's last addenda.
- **Files**:
  - `docs/decisions/034-shot-first-phone-first.md` — an addendum (M42): Full Swing is a second
    device; club speed and smash `analysed` for it (decision 5) and why HD Golf's are not; a value
    read from the device's impact video counts as printed (call 13), and is wired in whatever P13
    read (decision 9, saying how many of the five clips read); a Live Photo is two import methods,
    the still and the clip, neither needing the other, and importing both halves makes two shots of
    one swing (decision 7); `impact_offset_v` never from an overhead replay; `CENTERED_RADIUS_MM`
    unchanged (call 16); M41 folded in.
  - `docs/decisions/014-screen-capture-shot-ingestion.md` — an addendum (M42): the `full_swing`
    profile; `TextBox.rgb` and the lit rule (calls 4–5); the three other parser rules (6–8); the quad
    fix (P3); no `SCREEN_PARSER_VERSION` bump and why (call 3); P9's coverage; the grid read off a
    clip's frame by the still's reader, the frame rule P14 kept, and the clip gate's coverage.
  - `docs/README.md` — each touched ADR's addendum count in its map row (the pin), and nothing else
    before P16.
  - `docs/CONFORMANCE.md` §2's screen section — the three hand vectors, and the Full Swing photo
    gate and clip gate beside the OCR gate, and `TextBox.rgb` in the input shape.
  - `docs/ARCHITECTURE.md` — §1 (`golf-lab read-impact` and `import-video`, `golf-ocr read
    --boxes`, `--device full_swing`), §2 (`crates/impact` and its one edge), §4 (an entry imported
    from a clip: its grid frame as the photo, the clip beside it, call 14).
  - `data/README.md` — `raw/shot_screens/` holds Live Photo pairs with sidecars; `raw/impact-review/`;
    an entry imported from a clip holds its grid frame as `shot_screen.<digest>.png` and, in a
    session, the clip beside it as `shot_screen.<digest>.mov`.
  - `CLAUDE.md` — the crate count and the crate table's `crates/impact` entry, `golf-lab`'s verb list
    (`read-impact`, `import-video`), the M42 sentence in the opening paragraph's list of what Rust
    holds; nothing else.
  - `docs/plans/m31-m40-shot-first-pivot.md` §M38 — two bullets for the phone: Vision's boxes must
    carry `rgb` by call 4's definition, or Full Swing's ball direction and sidespin are withheld on
    every shot; a Live Photo is two import methods (decision 7), the still, or the clip's frames
    reaching `crates/impact` (`sharpest_frame`, `read_impact`) through `crates/app`.
  - `crates/contracts/src/strike.rs` — `CENTERED_RADIUS_MM`'s doc: M42 did not revise it (call 16).
- **Done when**: `pytest tests/test_docs_truth.py` green; `cargo test` green (doc comments only in
  Rust).

### P16 — Close: verify, WORKLOG, ROADMAP, memory, and the one commit

- **Goal**: the milestone verified and committed.
- **Do**:
  1. Every command under "Verify" that lists P16, in that order; record each result.
  2. `WORKLOG.md` — a top entry in the house form (duration, what was built, what changed against
     the plan, what M38 and the impact review should know, what is left open, the verify results).
  3. `ROADMAP.md` — the M41 and M42 status rows and sections (M42 done, M41 folded), and the
     **NEXT ACTION** paragraph: M38 (a Mac) and the user's review of the impact readings
     (`data/raw/impact-review/`).
  4. `docs/README.md` — this plan's map row and the document count (now that `git add` makes it
     visible to the pin).
  5. Memory: update `app-transition-decisions.md` (M42 done, what it built, the impact outcome) and
     its `MEMORY.md` line.
  6. `git add` the milestone's files (never `data/`, never `target/`), and one commit onto `main`:
     `M42: Full Swing in Rust — its screen read, two spin fields, and the impact video`, the body
     listing the parts, ending with the attribution lines the committing session's system reminder
     gives.
  7. The close report to the user: what reads now (P9's and P14's coverage), the impact outcome
     (how many clips P13 read), where the review images are, and call 17's two import commands.
- **Done when**: every verify command green, the commit made, `git status` clean except `data/` and
  `target/`.

---

## Exit

- The six grid photos read through the shipping lab reader with **no wrong value**, and P9's
  coverage recorded (target ≥ 66 of 72).
- `ShotData` holds `backspin` and `sidespin`; `full_swing` is declared and read; `hd_golf` reads
  exactly as before; no committed vector moved and no version key moved.
- `crates/impact` is built and tested on synthetic frames, and `golf-lab read-impact` runs on the five
  Live Photo clips with review images written.
- A Live Photo imports by either half (decision 7): the still through `import-photos`, the clip
  through `import-video`, whose shot holds the grid read off its frame (the clip gate: **no wrong
  value**) and the impact offset or its named `misread`, wired in whatever P13 read (decision 9).
- Docs, WORKLOG, ROADMAP and memory say all of it, in one commit.

---

## Phase findings

### P0 (2026-10-30)

- **Landed as planned.** The plan was approved by the user as written. The twelve Live Photo files
  are in `data/raw/shot_screens/`, copied with `cp -p`: every digest equals "The photos" table, and
  the modification times equal the source's (`IMG_2889.HEIC` and `.MOV` both
  `2026-10-10 14:36:14 -0500`). Six sidecars `IMG_NNNN.json` (LF, two-space indent) carry `image`,
  `motion`, `player_id` `aaron`, `club` `null`, `device` `full_swing`, `layout` (`grid`, or
  `club_view` for `IMG_2888`), `received`, `taken` and a `note`. `2026-10-07-1.json` gained
  `"device": "full_swing"` after `club`, and its note gained the M42 sentence; it keeps its own
  line endings.
- **ROADMAP**: the M41 and M42 status-table rows and the two sections' `**Status**` lines point at
  this file (M41 folded, M42 planned). Nothing else in ROADMAP moved.
- **Verify**: `pytest tests/test_docs_truth.py` 91 passed. The `data/processed/` digest is
  `07f37373…` (unchanged). `git status` shows `ROADMAP.md` modified and this file new; `data/` is
  gitignored.
- **For P9 and P13**: `IMG_2891` and `IMG_2892` are portrait stills (4284×5712); the other four
  HEICs are 5712×4284. `2026-10-07-1` is a JPEG with no motion part, so the impact phases have five
  Live Photos, not six.

### P1 (2026-10-30)

- **Landed as planned.** `ShotData` has `backspin` and `sidespin` after `carry_offline`, each
  `#[serde(default, skip_serializing_if = "Option::is_none")]`, under a comment block giving call 1's
  reason; `shot::WRITTEN_WHEN_SET` names them. Four unit tests in `shot.rs` (a Python-shaped shot
  reads neither; both set round-trip written; neither set writes neither key; the constant equals
  the keys a set shot writes and an unset one omits, read off two serializations). `synthetic` in
  `devices.json` declares both, `analysed`, after `carry_offline`. `NUMERIC_METRICS` is 21 long and
  `PLAUSIBLE_RANGES` 21 (backspin, sidespin, apex_height appended after `carry_offline`, so
  `hd_golf`'s warning order is untouched).
- **No `ShotData { … }` struct literal exists in the workspace**: every shot is built by serde,
  `screen::to_shot_data` included (it builds a JSON map of the profile's targets and deserializes
  it). So the plan's "every struct literal gains `backspin: None`" had nothing to edit, and **for
  P8**: a `full_swing` profile targeting `backspin` or `sidespin` reaches the struct with no code
  change in `crates/screen/src/lib.rs`.
- **The schemas** were edited by Windows Python: load, insert the two properties into each copy of
  `ShotData` (titles `Backspin`/`Sidespin`, `anyOf` number-or-null, `default` null, descriptions
  with unit and sign, ending "Written only when set."), dump with `sort_keys`, indent 2,
  `ensure_ascii`, CRLF and a trailing newline. That dump reproduces each of the four files byte for
  byte before the edit, so the diff is 26 added lines per file and nothing else.
  `tests/test_conformance.py` did not object (the four are already in `RUST_OWNED_SCHEMAS`), so the
  rules' one exception was not used.
- **The "If" line fired once, for P15**: `crates/mcp/src/query.rs`'s
  `the_metric_list_and_the_view_s_own_fields_split_every_shot_field` reads the key set off a
  default shot's serialization, so it now takes the union with `WRITTEN_WHEN_SET`. With
  `capability.rs::shot_data_keys` and `schemas.rs::rust_keys`, that is three tests that union with
  the constant; `rust_keys`' paragraph that anticipated a `skip_serializing_if` is rewritten.
- **Beyond the plan's list, for P15**: `screen::validate`'s
  `the_new_fields_are_checked_once_a_profile_reads_them` gained the three new ranges (backspin
  9899 → 98990, sidespin -4199 → -41990, apex 47.4 → 474), each caught. Nine range failures
  (1.35) pass the clean read's confidence, so the test's confidence assertion now expects the clamp
  at zero, and the warnings list is what pins the count.
- **For P8**: `screen::validate::tests::no_shipped_profile_reads_the_new_fields` checks
  `PLAUSIBLE_RANGES[12..]`, now M32's six **and M42's three**, against every shipped profile, so it
  fails the moment `full_swing` targets `apex_height`, `backspin` or `sidespin`. P8 narrows it to
  M32's six (`[12..18]`, or by name) and rewrites `PLAUSIBLE_RANGES`' doc, which says M42's three
  "reach no photo until a profile targets them (M42 P8)".
- **Not edited, and why**: `crates/core/src/storage_family.rs::RUST_ONLY_SHOT_KEYS` (the keys
  dropped to view a Rust-written shot on frozen Python's key set) drops a key only when it is
  `null`, and the two are never written as `null`; a set one is kept and reported as added, which
  is right.
- **Verify**: `cargo test` 1649 passed, 0 failed, 24 ignored; `cargo clippy --all-targets` and
  `cargo fmt --check` clean; `pytest` 2084 passed; `conformance.py check` exit 0; the three dry runs
  (`v17`, `screen-v1`, `career-v3`, built into `target/m42`) each "nothing differs … 0 changed; 0
  files written"; `git diff --stat -- spec/vectors src scripts tests` empty.

### P2 (2026-10-30)

- **Landed as planned, with one placement.** `TextBox` has `#[serde(default)] pub rgb: Option<[f64;
  3]>` with call 4's definition as its doc, and derives `Serialize`. The recognizer's literal is in
  `text_box`, `_to_text_box`'s twin, which is given no image. So it writes `rgb: None`, and
  `recognize` fills `rgb` from the image the boxes were found in straight after.
  `text_box`'s signature is unchanged. `ocr::recognizer::glyph_rgb` is call 4 exactly, with a
  private `clipped` for each axis. The five other literals (`prepare.rs`, `orient.rs`, `parser.rs`,
  and `validate.rs` twice, all tests) gained `rgb: None`. Tests: four in `recognizer.rs` (yellow
  strokes on a dark box read yellow exactly, with white text just past the box's right edge left
  out; six boxes with no pixel are `None`, and a one-pixel box is that pixel; a box run off the
  top-left corner clips to 16 pixels and reads 135; eight equally bright pixels read `[90.0, 105.0,
  105.0]`, pinning the stable row-major order). Two more in `screen`: a pre-M42 box reads `rgb`
  `None`, and a box with its colour round-trips.
- **`rgb` is always written**, as `null` when unset (the plan gave `#[serde(default)]` only, no
  `skip_serializing_if`). Nothing serialized a `TextBox` before this phase, so no output moved.
  Frozen Python's `conformance.py check` skips the screen family, so a hand vector whose boxes carry
  `rgb` (P10) is never read by the frozen `TextBox`.
- **`--boxes`** prints `{"device", "boxes", "notes"}` in that order, through a struct because this
  workspace's `serde_json` has no `preserve_order` and `json!` would sort the keys. It computes
  neither the digest nor the timestamp, so the models are now opened before the photo's mtime is
  read. That changes nothing a test pins: the order only shows when both would fail. **`--boxes`
  with `--json` exits 2**, a case the plan did not cover; `tests/golf_ocr.rs`'s malformed list
  pins it. No ignored test reads a photo with `--boxes`, because no verify command would ever run
  it. The measurement below is the record.
- **The measurement** (done criterion): `target/m42/release/golf-ocr read
  data/raw/shot_screens/IMG_2889.HEIC --device hd_golf --boxes` printed 41 boxes and the one note
  `only 33% of the hd_golf labels were legible at the best orientation - the parse below is
  unreliable`, so the vote tried all four turns (47 s). The sign letters, against finding 4:

  | box | x, y, w, h | `rgb` printed | finding 4 | b ÷ min(r, g) |
  |---|---|---|---|---|
  | `R` | 754, 467, 21, 19 | (238.93, 222.16, 90.45) | (239, 222, 90) | 0.41 |
  | `D` | 796, 616, 21, 19 | (226.9, 207.04, 71.97) | (227, 207, 72) | 0.35 |
  | `F` | 968, 617, 17, 18 | (186.55, 183.21, 175.62) | (187, 183, 176) | 0.96 |

  Each is within 0.5 per channel. No `L` box was detected. Every tile label and value box in the
  same read measured b ÷ min(r, g) 0.99–1.02. The screen's chrome measured 1.04–1.21 (`TEAM1`,
  `CONTINUE`, `SHOT DATA`, a `1` badge), and the lowest min(r, g) of any box was 161.
- **For P6**: through the shipped reader, lit letters measure 0.35 and 0.41, the unlit one 0.96,
  and white text 0.99 and up. Call 4's `LIT_BLUE_RATIO` 0.7 sits in the gap. The unlit `F`'s
  min(r, g) is 183, above `LIT_MIN_BRIGHTNESS` 120, so on this photo the blue ratio alone says it
  is unlit; brightness separates nothing here.
- **For P4**: on `IMG_2889` today's reader returned `CLUBSPEEDMPH` and `BALLSPEEDMPH`, and
  `BALLDIRECTIONDEG` and `LAUNCHANGLE`, as separate boxes. The merges in finding 3 are on
  `IMG_2891` and `2026-10-07-1`, not this photo. `TOTALYD` came back twice (header and data row),
  as finding 6 says.
- **For P9 and P10**: `--boxes` output pastes as a screen vector's `input` up to its identity keys
  (`shot_id`, `session_id`, `timestamp`, `min_confidence` and the image keys are the vector's to
  add). Each box's `rgb` is in the pixels of the working image at the vote's turn, the same image
  its coordinates are in. Under `hd_golf` the vote runs all four turns on a Full Swing photo. Under
  `full_swing` (P8) it should stop at the first, so a read should take about a quarter of the time.
- **For P15**: `docs/CONFORMANCE.md` mentions `TextBox` and should name `rgb` (optional; frozen
  Python's boxes and every committed vector have none). `docs/ARCHITECTURE.md` §1's `golf-ocr read`
  flags should gain `--boxes`, which cannot be combined with `--json`.
- **Verify**: `cargo test` 1654 passed, 0 failed, 24 ignored. `cargo clippy --all-targets` and
  `cargo fmt --check` are clean. The 13-photo OCR gate (`--release --target-dir target/m42 -p ocr
  --test read -- --include-ignored`) passed unchanged in 161 s. `git diff --stat -- spec/vectors src
  scripts tests` is empty. Preconditions: the three model directories are in `data/models/ocr/`,
  ffmpeg is on `PATH` (WinGet), and `IMG_2889.HEIC`'s digest is the table's.

### P3 (2026-10-30)

- **Landed as planned.** `find_screen_quad` orders an accepted `as_quad` candidate on the spot and
  refuses it when a private `repeats_a_corner` (any two ordered corners equal) says so; the contour
  loop goes on and the contour's rotated box may still stand in. The fallback is now kept as a
  `ScreenQuad`, ordered when kept rather than at the end (the same corners: `order_quad(divide(..))`
  either way), and a rotated box whose ordering repeats a corner is not kept. The module doc has a
  section, "Where it departs from its twin", with the defect, both photos' corners, frozen Python
  keeping it (ADR-035 clause 4), and why no photo whose quad ordered cleanly can move: a refusal
  only narrows what was accepted.
- **One test departs from the plan's wording.** `order_quad` on the exact 45° diamond repeats a
  corner, as planned (`a_diamond_orders_with_a_corner_repeated`). But a *photo* of a 45° square does
  not reach the defect: measured on the unfixed finder, its corners reduce a pixel off their ties,
  (199, 20) (330, 149) (201, 280) (70, 151), and order cleanly, so "`None` or four distinct corners"
  held before the fix and pinned nothing. The image test
  (`a_diamond_screen_is_never_found_with_a_corner_repeated`) uses a diamond twice as wide as tall
  instead (diagonals 360 and 180 px, 27% of a 400×300 frame), which ties nothing. Unfixed it came
  back `Corners` `[[20, 149], [380, 151], [380, 151], [20, 149]]`; fixed, `RotatedBox` (Canny 30,
  closing 5) with four distinct corners. A test choice, not behaviour, so nothing for the docs.
- **The measurement** (done criterion). A throwaway scratch crate replayed the unfixed search
  through `ocr::cv`'s public functions beside the fixed `find_screen_quad`, and was deleted:

  | photo | before P3 | after P3 | `--boxes` (hd_golf) |
  |---|---|---|---|
  | `IMG_2893` | `Corners` (30, 5, ε 0.04), TR = BR = (5706.3, 2638.9) | `RotatedBox` (30, 5), (−234.9, 1092.1) (5541.4, 426.3) (5889.0, 3441.6) (112.6, 4107.4) | 38 boxes, all 15 tile labels |
  | `IMG_2894` | `Corners` (30, 5, ε 0.04), TR = BR = (5706.3, 2707.5) | `RotatedBox` (30, 5), (15.0, 967.3) (5738.8, 1081.8) (5687.0, 3671.1) (−36.7, 3556.6) | 39 boxes, all 15 tile labels |

  The "before" corners are finding 5's to the tenth. Neither read was uncropped (no
  `UNCROPPED_NOTE`). Both votes ran all four turns under `hd_golf` (33% and 27% of its labels
  legible), 40 s each. The labels came back as `SHOTRESULT`, `TOTAL YD` (twice), `TOHOLEFT` /
  `TOHOLEYD`, `CARRYYD`, `CLUB SPEEDMPH`, `BALL SPEED MPH`, `PATH DEG`, `FACEDEG`,
  `BALLDIRECTIONDEG`, `LAUNCHANGLE`, `APEXFT`, `SMASHFACTOR`, `BACKSPINRPM` and `SIDESPIN RPM`, in
  a mix of spaced and unspaced spellings. Every one of the truth table's twelve values for each
  photo is among the boxes.
- **For P9**: `IMG_2894`'s read carried the note `image was rotated 90 degrees - corrected` though
  its rotated box is near upright, and its boxes came back out of reading order; its club and ball
  speed labels were unit-less (`CLUB SPEED`, `BALLSPEED`, the case P8's aliases are for). It also
  carried four stray boxes (`22`, `会`, `二`, `マ`). Check under `full_swing` which turn the vote
  takes on it. `IMG_2893`'s rotated box runs off the photo's left edge (x −234.9); the warp read it
  fine.
- **For P15**: the reader now departs from frozen Python's `find_screen_quad` on a quad whose
  ordering repeats a corner; `crates/ocr/src/quad.rs`'s module doc is the record, and ADR-014's M42
  addendum should name it beside the parser rules.
- **For P6**: on `IMG_2893` the sign letters came back as `R`, `D` and a two-letter box `DD`. Call
  5's letter box must be exactly one of the pair's tokens, so `DD` is not one; on `IMG_2894`, `R`
  and `D`.
- **Verify**: `cargo test` 1656 passed (P2's 1654 and the two new), 0 failed, 24 ignored.
  `cargo clippy --all-targets` and `cargo fmt --check` are clean. The quad gate and the 13-photo
  OCR gate (`cargo test --release --target-dir target/m42 -p ocr --test quad --test read --
  --include-ignored`) passed unchanged, the OCR gate in 193 s. `git diff --stat -- spec/vectors src
  scripts tests` is empty. Preconditions: the three model directories are in `data/models/ocr/`,
  ffmpeg is on `PATH` (WinGet), and `IMG_2893.HEIC`'s and `IMG_2894.HEIC`'s digests are the
  table's.

### P4 (2026-10-30)

- **Landed as planned.** `DeviceProfile` has `#[serde(default)] pub split_merged_labels: bool`
  (`crates/screen/src/profile.rs`, with a module-doc section on keys the frozen shapes lack), and
  `parser::split_merged` is call 6 exactly, with its cut search in a private `merged_cut`.
  `parse_screen` runs it first when the profile opts in, through a `Cow`, so every later step (the
  title check included) sees the split list. The parser's module doc has a section, "Rules a profile
  opts into [M42]", which P5 to P7 should extend rather than repeat: why the rules are opt-in, why
  merged labels are split, and why the proportional width is close enough (the cut *is* the
  boundary between the two columns, and a value is placed by its centre).
- **Two readings the plan left open, both the least-inventive one.** A half's best field is
  `DeviceProfile::field_for`'s, so a half scoring the same against two fields names the later one,
  as a whole box does. "Different fields" is pointer identity in the profile, not label text. A split
  box's halves are the despaced texts (`CLUBSPEEDMPH`), so a tie warning on a half would quote that.
- **Tests** (six new, on made-up profiles): finding 3's two merges split at 12 of 24 and 16 of
  27 letters (widths 120/120 of 240 and 160/110 of 270), keeping `y`, `height`, `confidence` and
  `rgb`; the speed pair located and both values read through `parse_screen` with the flag, and the
  same boxes locating nothing without it; `Ball Speed` and `BALLSPEED` never cut where `Ball` and
  `Speed` are labels too; `CARRYCARRY` left whole; `CARRY|XTOTAL` against `CARRYX|TOTAL`, a tie at
  10/11, goes to the earlier cut; and the profile key parses where named and is off on `hd_golf`.
  Four mutations (the different-fields check, the tie direction, the matching-box guard, the flag)
  each failed exactly the test meant for it, and were reverted.
- **For P8**: the profile-level key is `"split_merged_labels": true`, beside `device` and `title`,
  not on a field. `full_swing_uses_every_rule` should assert it on the shipped profile.
- **For P9**: the orientation vote does not split. `screen::orient::label_ratio`, which
  `crates/ocr/src/prepare.rs` stops on at `GOOD_ENOUGH_LABEL_RATIO` (0.6), scores the boxes as the
  OCR returned them, so a merged pair counts as neither label there while the parse reads both. On a
  photo with a merge (`IMG_2891`, `2026-10-07-1`) the vote's ratio is two labels lower than what the
  parser locates. If that keeps a `full_swing` read from stopping at its first turn, or puts its
  legibility note on a photo the parse reads whole, record it and take the least-change option
  (the plan does not cover splitting in the vote, and the vote's ratio is recorded in every screen
  vector, so changing it is not P9's to do). Also measure `split_merged`'s cost on a real photo's
  boxes: it scores every unmatched box at every cut against every field.
- **For P15**: ADR-014's M42 addendum should name the rule, the key, and that a half's best field
  follows `field_for`'s later-wins tie.
- **Verify**: `cargo test` 1662 passed (P3's 1656 and the six new tests; one existing profile test
  gained an assertion), 0 failed, 24 ignored. `cargo clippy --all-targets` and `cargo fmt --check`
  are clean. Every screen vector passed unchanged, `crates/screen/profiles.json` is untouched, and
  `git diff --stat -- spec/vectors src scripts tests` is empty.

### P5 (2026-10-30)

- **Landed as planned.** `ProfileField` has `#[serde(default)] pub printed_twice: bool`, and
  `DeviceProfile::check(&self) -> Result<(), String>` exists with call 7's clause as its first.
  `profiles()` runs it on every shipped profile inside its `OnceLock` and panics with the refusal.
  In `crates/screen/src/parser.rs`: `boxes_taken(field)` (2 or 1) is each field's capacity;
  `assignments` takes a `capacity: &[u8]` and counts what each field has taken in a `[u8; TIE_CAP]`
  where it had a bit mask; `Decided::outcomes` holds sets of boxes (`BTreeSet<Vec<usize>>`, the empty
  one standing for "none") where it held `Option<usize>`; and `tied_boxes` names the union of those
  sets. That union is the old output exactly for a field taking one box. `read_cell` is split into
  `read_text` (the cell's text to a `Reading`, its value and warnings) and `Reading::write`, so
  `read_pair` can read both cells before deciding. A tile read once is written as before. The
  module doc's "Rules a profile opts into [M42]" has a paragraph on the rule.
- **Readings the plan left open, each the one that invents least.**
  - `{a!r}` and `{b!r}` in call 7's warning are the two **cells' texts**, top first, through
    `pyfmt::str_repr`, as every other parser warning quotes text: `Total Yd: printed twice as '134'
    and '137' - not stored`.
  - **Warnings.** If the cells agree, the field gets the top cell's own warnings. If only one gives
    a number, it gets that cell's warnings, and the other cell's are dropped, because "no value text
    under the label" over a stored total is false. If neither gives a number, it gets each cell's
    warnings in turn, and a line the top cell already raised is not repeated, so two empty cells
    give one `no value text`.
  - `raw_fields` is joined even where a cell is empty, as call 7 says literally: `"134 | "`,
    `" | 134"`. The pair is read where its **top** cell stands in reading order, so the field's
    `raw_fields` and `values` entries sit there. A printed-twice field with only one box located is
    read by `read_cell`, exactly as any tile.
  - **Confidence**: `score`'s value coverage now counts **fields** with text, not cells, so a total
    whose two cells both hold text counts once. Every field without the flag has one cell at most,
    so its count is unchanged, and the screen vectors confirm it.
- **Tests** (eight new: two in `profile.rs`, six in `parser.rs`). In `profile.rs`, the key parses
  where it is named, and is off on every `hd_golf` field. The check refuses a layout-only tile and a
  text tile, each by name, and passes every shipped profile. In `parser.rs`:
  - Two equal cells (`134`/`134`, and `134`/`134.0`) store 134 and join `raw_fields`, at confidence
    1.0, and at 0.825 with `Carry` empty. Counting cells would have given 1.0 there.
  - Two unequal cells store nothing and give call 7's warning, with the tile still in
    `fields_present` and `raw_fields`.
  - One number is stored whichever cell holds it, the other being absent, `---` or `yds`, with no
    warning. One box located reads as any tile. With neither cell giving a number the warnings are
    deduplicated.
  - The field's two boxes resolve together, a `TOTALYD` copy included. Capacity two gives 4
    assignments where capacity one gives 3.
  - A third box is withheld, naming all three boxes.
  - Without the flag, the same boxes withhold the total (`carry-twice`'s case).

  Five mutations each failed the test meant for it and were reverted: capacity forced to 1,
  coverage counted by cell, the dedupe removed, a disagreement storing the top value, and the second
  cell read again.
- **For P6 and P7**: extend `DeviceProfile::check` with your clauses as more `if … return Err(…)`
  inside its one walk over the fields. Its refusals are spelt `"{device}: {label!r} …"`. Add each
  clause to its doc list. Both rules live in `read_text` now, not in `read_cell`: that is where the
  sign is applied and the number made, and `read_pair` gets both rules through it. So P7's "divide
  by 3.0 after the sign" goes at the end of `read_text`, and a printed-twice field compares values
  already in yards.
- **For P8**: the key is per field, `"printed_twice": true` on `TOTAL YD`. `full_swing_uses_every_rule`
  should assert it is on that field and on no other. Putting it on the header's `SHOT RESULT` or
  `TO HOLE` tiles would panic every profile load (they are layout-only). Also, `tiles::judged`
  (`crates/screen/src/tiles.rs:137`) reads the joined `raw_fields`. Two cells that disagree are
  `misread` (`'134 | 137' under the label`), which is right, because a retake can fix it. But two
  empty cells (`" | "`) or two blanks (`"--- | ---"`) are also judged `misread`, where one cell would
  be `printed_blank`. Full Swing prints a total on every photo the plan read, so nothing reaches
  that today. The least-change option is to leave `judged` alone and have P15 document it. If P8
  touches `tiles.rs` anyway, splitting on `" | "` before judging is the fix.
- **For P9**: the gate's `total (×2)` column is where this rule meets real photos. If the OCR
  misreads one copy, the total is withheld with call 7's warning rather than stored, and the gate
  should record which photo did that. CPython's `difflib` scores `TO HOLE YD` 0.667 and `TOHOLEYD`
  0.625 against `TOTAL YD`, below the 0.8 threshold, so neither should become a third box for the
  label. Measure it on the real boxes all the same.
- **For P15**: ADR-014's M42 addendum should cover:
  - the rule and its key;
  - the warning's text;
  - which cell's warnings are kept;
  - the joined `raw_fields`;
  - value coverage counting fields, not cells;
  - that `DeviceProfile::check` refuses a misplaced key at load;
  - the `tiles::judged` gap above, if P8 left it.
- **Verify**: `cargo test` 1670 passed (P4's 1662 and the eight new tests), 0 failed, 24 ignored.
  `cargo clippy --all-targets` and `cargo fmt --check` are clean. Every screen vector passed
  unchanged, `crates/screen/profiles.json` is untouched, and
  `git diff --stat -- spec/vectors src scripts tests` is empty.

### P6 (2026-10-30)

- **Landed as planned.** `crates/screen/src/profile.rs` has `LitToken { token, sign }` and
  `ProfileField`'s `#[serde(default)] pub lit_sign: Vec<LitToken>`. `crates/screen/src/parser.rs`
  has `LIT_MIN_BRIGHTNESS` (120.0) and `LIT_BLUE_RATIO` (0.7), the second carrying finding 4's
  table, and `pub fn is_lit`, reached as `screen::parser::is_lit`. A private `lit_sign(pair,
  value_boxes)` implements call 5, and `read_text` calls it for a `lit_sign` tile. The magnitude is
  `first_number`'s `abs()`. A magnitude of exactly 0 is stored as `0.0` before any letter is
  looked at, so a zero with both letters lit stores `0.0` with no warning. A sign the letters
  cannot decide gives no value and call 5's warning, with the tokens in profile order: `Ball
  Direction Deg: neither L nor R could be read as lit - sign unknown, not stored`. The tile stays
  in `fields_present`, and `raw_fields` keeps the cell's text with its letters (`L R 10.1`). The
  `sign_tokens`/`printed_sign` branch is now an `else if` and is otherwise untouched. The parser's
  module doc has a paragraph on the rule under "Rules a profile opts into [M42]".
- **`read_text` takes the cell's value boxes** (`read_text(text, value_boxes, field, profile)`).
  The letters' colours are on the boxes, not in the text. `read_cell` passes its cell's boxes, and
  `read_pair` passes each cell its own.
- **Readings the plan left open, each the one that invents least.**
  - **Two boxes of one letter** count as one letter when they agree (lit or unlit). When they
    disagree, the letter is unknown and the sign is withheld, as for a letter box with no `rgb`.
  - **The load-time check has three clauses the plan did not name.** `lit_sign` is allowed only on
    a stored number tile, for `printed_twice`'s reason: elsewhere it would be ignored in silence.
    Its two tokens must differ. Each token must be non-empty and already stripped and upper-cased,
    because a box is compared that way and a token no box can equal would withhold on every photo.
    The plan's own clauses are there too: exactly two tokens, signed `1` and `-1`, and never
    beside `sign_tokens` or `printed_sign`. Refusals: `"{device}: {label!r} sets lit_sign, which
    only a stored number tile may"`, `"… sets lit_sign beside sign_tokens or printed_sign, and a
    sign has one source"`, `"… sets lit_sign, which takes two different tokens, stripped and
    upper-case, one signed 1 and one -1"`. `check` now builds its refusals through one closure,
    and `printed_twice`'s text is unchanged.
  - **A cell with letters and no number** gets the usual `could not read a number from 'R --'`.
- **Call 5's geometry holds on `IMG_2889`.** This was read from the earlier `--boxes` dump of
  that photo: P2's 41 boxes, all in the rectified image's pixels. The lit `R` (754, 467, 21×19)
  centres inside `BALLDIRECTIONDEG`'s cell (label 572–788 wide, y 438, height 17; the column ends
  at 794, halfway to `LAUNCHANGLE`). `D` (796, 616) and `F` (968, 617) centre inside
  `SIDESPINRPM`'s cell (label 814–967, y 588, height 17). Each letter sits on a line above its
  value, so the cell texts read `R 10.1` and `D F 4199`.
- **Tests** (seven new: two in `profile.rs` and five in `parser.rs`, plus one assertion added to
  `an_absent_key_takes_pydantics_default`).
  - In `profile.rs`: the key parses where named and is off on every `hd_golf` field. The check
    refuses twelve made-up tiles, each with its own sentence, and passes every shipped profile.
  - In `parser.rs`:
    - Finding 4's nine rows classify as their "lit" column, as do P2's three readings through the
      shipping reader and three white boxes. Each edge is tested on its own: the 120 floor is
      inclusive in red and in green, and the blue ratio is tested on both sides.
    - Each of call 5's branches is tested through `parse_screen`, thirteen cases in all.
    - A printed `-`/`+` is discarded, a `°` is ignored, and `0`, `0.0` and `-0.0` are stored as
      `+0.0`.
    - Letter boxes are matched exactly: ` r ` is `R`, while `RR` and `R.` are no letter. Two
      boxes of one letter count once when they agree and withhold when they disagree.
    - A tile without the rule reads its letters as text. With no sign rule, `L R -10.1` is stored
      as -10.1. Under HD Golf's word rule the residual `LR` is no direction word, so the word
      rule's own warning shows why it cannot read this screen.
  - Six mutations each failed the tests meant for them and were reverted: the ratio raised to
    0.99, `abs` removed, the zero skip removed, a colourless letter read as absent, two
    disagreeing boxes resolved last-wins, and the two unlit-alone branches swapped.
- **For P7**: as P5's finding said, the rule goes at the end of `read_text`, which now also takes
  `value_boxes`. Dividing by 3.0 after `magnitude *= sign` keeps a zero at `+0.0`.
- **For P8**:
  - `lit_sign` goes on `BALL DIRECTION DEG` as `[{"token": "L", "sign": -1}, {"token": "R",
    "sign": 1}]` and on `SIDESPIN RPM` as `[{"token": "D", "sign": -1}, {"token": "F", "sign":
    1}]` (call 2: + = fade). Neither tile may carry `sign_tokens` or `printed_sign`, because the
    check refuses that at load. `full_swing_uses_every_rule` should assert the key is on exactly
    those two tiles.
  - **`value_span_ratio` needs to be above `hd_golf`'s default of 3.5 on this screen.** The
    measurements are from the same `IMG_2889` dump. Three labels are 17 px tall, and their values
    centre farther below the label's bottom than 3.5 heights:

    | label | value | distance below | heights |
    |---|---|---|---|
    | `BALLDIRECTIONDEG` | `10.1` | 62.5 px | 3.68 |
    | `SIDESPINRPM` | `4199` | 61.5 px | 3.62 |
    | `CLUBSPEEDMPH` | `95` | 65.5 px | 3.85 |

    At 3.5 all three values fall outside their cells and are lost. Every 20–24 px label is under
    3.5: `PATHDEG` 3.1, `BACKSPINRPM` 3.0, `APEXFT` 2.9, and the header's `TOTALYD` 3.46. The
    next row of labels bounds a cell in any case, and the header row's next row starts at y 280,
    so a ratio near 5 would be safe there. `value_span_ratio` is a profile-level key that already
    exists, so this needs no code. P9 holds whatever P8 sets.
  - `tiles::judged` reads a withheld sign as `misread`. That is right, because a retake can read
    the letter. Its detail, though, is `'R 10.1' under the label, and no value could be read from
    it`, and that is inexact: the magnitude was read and only its sign was not. The least change
    is to leave it, as for P5's `" | "` gap. P15 should document it.
- **For P9**: the gate's ball direction and sidespin columns are where this rule meets the
  photos. The OCR returns 0, 1 or 2 of each pair (finding 4), so record per photo which letters
  came back, their `rgb`, and whether the sign was decided or withheld. On `IMG_2893` the `D` came
  back as `DD`, which is no letter (P3's finding), so its sidespin is decided only if `F` is
  detected, unlit.
- **For P15**: ADR-014's M42 addendum should cover:
  - the rule, its key `lit_sign` and `LitToken`;
  - `is_lit` and its two constants, kept in `screen` so the phone shares them;
  - call 5's table, with the two-boxes-of-one-letter reading;
  - the zero rule;
  - the warning's text;
  - that `raw_fields` keeps the letters;
  - the check's clauses, including the three this phase added;
  - the `judged` detail above.
- **Verify**: `cargo test` 1677 passed (P5's 1670 and the seven new tests), 0 failed, 24 ignored.
  `cargo clippy --all-targets` and `cargo fmt --check` are clean. Every screen vector passed
  unchanged, `crates/screen/profiles.json` is untouched, and
  `git diff --stat -- spec/vectors src scripts tests` is empty.

### P7 (2026-10-30)

- **Landed as planned.** `crates/screen/src/profile.rs` has `pub enum PrintedUnit { Feet }`
  (`rename_all = "lowercase"`) and `ProfileField`'s `#[serde(default)] pub printed_unit:
  Option<PrintedUnit>`. The conversion is a method, `PrintedUnit::to_yards(self, printed) ->
  f64`, which returns `printed / 3.0` unrounded. `read_text` calls it as its last step before the
  number is stored, so it applies on every sign path, and `raw_fields` keeps the printed text. The
  parser's module doc has a paragraph on the rule under "Rules a profile opts into [M42]".
- **It divides, and the oracle must too.** `85.2 / 3.0` is `28.400000000000002`, and `85.2 * (1.0
  / 3.0)` is `28.4`. Of the truth table's six apexes, `15.5` and `142.2` also differ between the
  two forms. `to_yards`' doc says why it divides, and a test holds the bits.
- **Readings the plan left open, each the one that invents least.**
  - **The yard targets** are a private `const STORED_IN_YARDS` in `profile.rs` (call 8's five).
    The test `the_yard_targets_are_the_fields_shot_data_holds_in_yards` holds them equal to the
    properties of `spec/schemas/shot_data.schema.json` whose description starts `yards`. So a yard
    field added to `ShotData` later fails a test rather than being refused in silence.
  - **The check's clause also requires a stored number tile**, as P5's and P6's clauses do. A text
    tile targeting `apex_height` is refused, and so is a layout-only tile. The refusal is `"{device}:
    {label!r} sets printed_unit, which only a number tile stored in yards may"`.
  - **A unit with no variant fails the parse**, unlike an unknown key. `"metres"` or `"Feet"` makes
    `parse_profiles` return an error, because serde refuses an unknown enum variant. Given call 9's
    worry, this is the safer of the two behaviours: a unit read as none would store feet as yards.
  - **The profile's `unit` key is not checked against `printed_unit`.** Nothing in the crate reads
    `unit`. P8's profile writes `"unit": "yards"` on `APEX FT`, which names the stored unit, and
    that is consistent.
  - **"After the sign".** A sign is a multiplication by ±1, which is exact, so the division gives
    the same float before or after it. It comes last so it is written once, and a zero that the lit
    rule made `+0.0` stays `+0.0`. The test says this rather than claiming the order is pinned.
- **Tests** (six new: four in `profile.rs` and two in `parser.rs`, plus one assertion added to
  `an_absent_key_takes_pydantics_default`).
  - In `profile.rs`:
    - The key parses where named, is off on every `hd_golf` field, and an unknown unit fails the
      parse.
    - The check refuses `launch_angle`, `club_head_speed`, a layout-only tile and a text tile with
      a yard target. It passes each of the five yard targets and every shipped profile.
    - The schema pin above.
    - `to_yards(85.2)` has the bits of `85.2 / 3.0` and not those of `85.2 * (1.0 / 3.0)`.
  - In `parser.rs`:
    - Each of the truth table's six apexes, through `parse_screen`, is stored bit for bit as
      `printed / 3.0`, with `raw_fields` holding the printed text and no warning. Without the key,
      the same tile stores the number as printed.
    - A printed `-30` ft (`printed_sign`) stores `-10` yd. A lit `L` over `30` stores `-10`, and
      `0` stores `+0.0`. A printed-twice total of `402`/`402.0` ft stores `134` yd. `402`/`405`
      is withheld with call 7's warning, which quotes the printed text.
  - Six mutations each failed the test meant for it and were reverted:
    - multiplying by a third;
    - removing the conversion;
    - the check reduced to yards-only;
    - the check reduced to stored-number-only;
    - a non-yard target swapped into the list;
    - a yard target dropped from the list, which does not compile because the array has a fixed
      length.
- **For P8**:
  - The key is per field, `"printed_unit": "feet"` on `APEX FT`, as P8's profile already
    writes it. `full_swing_uses_every_rule` should assert that it is on that tile and on no other.
  - Do not put it on `TO HOLE YD`/`TO HOLE FT`. That tile is layout-only, and the check would panic
    every profile load.
- **For P9**:
  - The truth table's oracle `85.2 / 3.0` etc. is already the parser's float, so compare exactly.
  - The validator's `apex_height` range of 0–80 yd sees the converted value: the table's apexes
    run from 5.2 to 47.4 yd.
  - Stored as printed, `85.2`, `80.6` and `142.2` would have been flagged by that range, and
    `39.9`, `71.4` and `15.5` would not. That is the case the module doc makes for the rule.
- **For P10**:
  - A hand vector's expected `apex_height` is the division's float written in its shortest form,
    for example `28.400000000000002` for `85.2`, not `28.4`.
  - Its `raw_fields` entry is the printed text, `"85.2"`.
- **For P15**: ADR-014's M42 addendum should cover:
  - the rule, its key `printed_unit` and `PrintedUnit` (one variant);
  - `to_yards` dividing by 3.0 unrounded, and why it divides;
  - the check's clause, and the schema pin behind `STORED_IN_YARDS`;
  - that an unknown unit fails the parse where an unknown key does not;
  - a wording edge. The word rule's warning `… sign unknown, storing the magnitude as printed`
    would still say "as printed" on a tile that also converts, where "as printed" means unsigned.
    No shipped tile has both rules (P8's apex has no sign rule), so nothing reaches it today.
- **Verify**: `cargo test` 1683 passed (P6's 1677 and the six new tests), 0 failed, 24 ignored.
  `cargo clippy --all-targets` and `cargo fmt --check` are clean. Every screen vector passed
  unchanged, `crates/screen/profiles.json` is untouched, and
  `git diff --stat -- spec/vectors src scripts tests` is empty.

### P8 (2026-10-30)

- **Landed as planned.** `crates/screen/profiles.json` has the `full_swing` profile after
  `hd_golf`, as the plan writes it (indented to the file's depth). `git diff` of that file removes no
  line, so `hd_golf`'s entry is byte-unchanged. `crates/contracts/devices.json` has call 11's
  `full_swing` entry between `hd_golf` and `mock`, and the top `_comment` gained one sentence
  naming it. `capability.rs` has `FieldSource { Screen, ImpactVideo }` (`#[default] Screen`) and
  `DeclaredField.from`, documented by call 13. Its module doc and `DeviceCapability.fields` doc now
  say every device with a profile, not `hd_golf` alone. `shot.rs` ledger entry 1 has call 3's
  sentence.
- **Tests** (seven new, three generalized):
  - `crates/contracts/tests/capability.rs` gained two. One holds `full_swing` to call 11's twelve
    `(field, use)` pairs in order. The other covers `from`: absent and `"screen"` parse to `Screen`,
    `"impact_video"` to `ImpactVideo`, an unknown source fails the parse, and **no shipped field is
    `impact_video`**.
  - `crates/screen/tests/capability.rs` walks every profile. It gets the list from
    `load_profile`'s refusal (`UnknownProfile.known`), because `profiles()` is `pub(crate)` and a new
    public API for a test was more change than this. A profile with no declaration fails there. It
    compares only `from: screen` fields. `the_walk_covers_every_profile` pins `["full_swing",
    "hd_golf"]`. `a_tile_with_no_target_is_not_a_field` gives each device its own layout-only list
    (`Custom`; `Shot Result`, `To Hole Yd`).
  - `crates/screen/tests/profile_fork.rs` has `DECLARED_DEVICES` and `without_devices`, and two
    tests: the whole profile is taken out and nothing else, and a stale entry fails. A declared
    device must also be absent from the frozen copy. The doc cites decision 1, and says why a
    device the fork alone has differs from a field delta.
  - `parser.rs` gained `full_swing_labels_do_not_overlap` and `full_swing_uses_every_rule`. The
    second holds each of the four rules to its tile(s): `split_merged_labels`; `printed_twice` on
    `Total Yd` only; `lit_sign` on `Ball Direction Deg` (L −1, R +1) and `Sidespin Rpm` (D −1, F +1)
    only; and `printed_unit` feet on `Apex Ft` only. It also checks that no tile has `sign_tokens`
    or `printed_sign`, and that `value_span_ratio` is in P9's bracket [4.0, 8.0] rather than equal
    to 6.0, so P9's tuning does not have to edit it.
  - `tiles.rs`'s `every_declared_field_has_its_tile` walks `profiles()` over the `from: screen`
    fields.
  - `validate.rs`'s test is renamed `no_shipped_profile_reads_m32s_fields`, checks
    `PLAUSIBLE_RANGES[12..18]` by name, and asserts that `full_swing` reads M42's three ranges. The
    ranges' doc ("unlike M32's, they gate reads") and the module doc's "What gates it" are
    rewritten.
- **Beyond the plan, for P15**: two tests quoted the unknown-profile refusal, `... Known profiles:
  hd_golf`. They are `screen::profile::tests::an_unknown_device_is_refused_in_pythons_words` and
  `crates/lab/tests/shot_lookup.rs`'s `nope` case, and both now read `full_swing, hd_golf`. No
  vector records that text (grep of `spec/`). Frozen Python's own refusal still lists `hd_golf`
  alone. The tie margin's doc and the `tile_refusals` doc also named `hd_golf` alone and now name
  both.
- **The overlap margin, for P9.** The closest pair of names on two different tiles is `Club Speed
  Mph` and `Ball Speed Mph`, at 0.786 (11/14, CPython's difflib, pinned by the test). That is 0.014
  under the threshold. Next come `Club Speed` against `Ball Speed Mph` at 0.70, and `Total Yd`/`To
  Hole Yd` and `Backspin Rpm`/`Sidespin Rpm` at 0.667. So an OCR-damaged speed label can reach 0.8
  against both speeds, and the tie rule then decides it. An alias added in tuning step (a) must
  not narrow that gap. The overlap test names any alias that does.
- **The measurement** (done criterion). `target/m42/release/golf-ocr read --device full_swing
  data/raw/shot_screens/IMG_2889.HEIC` printed `turned 0 degrees, 41 boxes, 100% of the labels
  legible`, all twelve truth-table fields, `confidence 1.000`, and no warning and no `NEEDS
  REVIEW`. Every value equals the truth table: `apex_height 28.400000000000002` (`85.2 / 3.0`),
  `backspin 9899.0`, `ball_speed 102.5`, `carry_distance 131.0`, `club_face_angle 13.2`,
  `club_head_speed 95.0`, `club_path -2.0`, `launch_angle 24.5`, `launch_direction 10.1` (R lit),
  `sidespin -4199.0` (D lit), `smash_factor 1.08`, `total_distance 134.0` (both copies). The vote
  stopped at its first turn, as P2 predicted. The `--boxes` dump at that turn is P2's 41 boxes:
  `R` (238.9, 222.2, 90.5), `D` (226.9, 207.0, 72.0), `F` (186.5, 183.2, 175.6), the speeds as
  separate boxes, and `TOTALYD` twice. Notes were empty.
- **For P9: budget the time.** That read took 150 s, twice. `--boxes` alone took the same, so the
  OCR is the cost, not the parse or P4's split. P2 measured 47 s for four turns of the same photo
  under `hd_golf`, so this session's machine was slower, with no other cargo or OCR process
  running. Run the six-photo gate in the background and time it yourself.
- **For P14** *(now P14b: the declaration moved there with decisions 7–9)*:
  - `a_declared_field_is_read_from_the_screen_unless_it_says_otherwise` (in
    `crates/contracts/tests/capability.rs`) ends by asserting that no shipped field is
    `impact_video`. When `full_swing` declares `impact_offset_h` with `"from": "impact_video"`,
    narrow that loop to every field but that one.
  - Two pins already compare only `from: screen` fields and need no edit:
    `crates/screen/tests/capability.rs` and `tiles.rs`'s `every_declared_field_has_its_tile`.
  - `devices.json`'s top `_comment` says nothing about `from`. Add a line when the first
    `impact_video` entry lands.
  - `full_swing_declares_its_grid_in_tile_order_with_the_spins_shown_only` lists the twelve fields
    exactly, so it gains call 13's thirteenth.
- **For P15**:
  - `tiles::judged` was not touched, so P5's `" | "` gap and P6's sign-withheld detail are both
    left for the docs.
  - ADR-034's addendum should name `FieldSource` and `from` as call 13's answer, even though
    nothing declares `impact_video` before P14.
  - CONFORMANCE or ADR-014 should say that `profile_fork.rs` now declares a whole device as well
    as a field.
- **Verify**: `cargo test` 1690 passed (P7's 1683 and the seven new tests), 0 failed, 24 ignored.
  `cargo clippy --all-targets` and `cargo fmt --check` are clean. Built into `target/m42`, the
  `screen-v1` dry run reported "nothing differs … 42 vectors run (42 screen); 0 changed; 0 files
  written". `git diff --stat -- spec/vectors src scripts tests` is empty. Preconditions for the
  read: the three model directories are in `data/models/ocr/`, ffmpeg is on `PATH` (WinGet), and
  `IMG_2889.HEIC`'s digest is the table's.

### P9 (2026-10-30)

- **Landed as planned, and no tuning was needed.** `crates/ocr/tests/full_swing.rs` holds the one
  ignored test, `every_grid_photo_reads_as_it_prints_or_withholds`, with the plan's ignore text. The
  truth table is written in its own spelling (a magnitude and its lit letter, the apex in feet), and
  `Photo::truth` turns it into `ShotData`'s conventions in one place. Values are compared by
  `to_bits`. The test prints the read/withheld table. For each photo it also prints the turn, the
  box count, the label ratio, `needs_review`, the confidence, the sign letters (with `rgb` and
  `is_lit`) and the warnings, and then the coverage. **Before any tuning step, 71 of the 72 values
  were read and none was wrong**, so steps (a)–(d) were not taken. `full_swing`'s profile is
  unchanged, `DeviceProfile` gained no `row_tolerance`, and the lit constants did not move.
- **The gate** (875 s for six reads, 121–174 s each):

  | photo | read | withheld | turned | label ratio | needs_review | confidence |
  |---|---|---|---|---|---|---|
  | `IMG_2889` | 12 | — | 0 | 1.0 | false | 1.000 |
  | `IMG_2891` | 12 | — | 0 | 0.667 | false | 0.999 |
  | `IMG_2892` | 11 | `total_distance` | 0 | 1.0 | false | 0.993 |
  | `IMG_2893` | 12 | — | 0 | 1.0 | false | 0.998 |
  | `IMG_2894` | 12 | — | 0 | 0.833 | false | 0.997 |
  | `2026-10-07-1` | 12 | — | 0 | 0.833 | false | 0.999 |

  No photo carried a note: no legibility note, and no rotation note. `IMG_2894`'s `rotated 90
  degrees` note under `hd_golf` (P3) is gone under `full_swing`.
- **The one withheld value is P5's rule working on a real photo.** On `IMG_2892` the OCR returned a
  stray box, `85TAS` (confidence 0.24, at (541, 85), 144×32, rgb (194, 196, 183)). It sits under the
  header `TOTALYD` label and above its `82`, so that cell reads `85TAS 82` and its first number is
  85. The data row reads `82`, so the field gets call 7's warning, `Total Yd: printed twice as
  '85TAS 82' and '82' - not stored`. Without the rule the header copy alone would have stored a
  wrong 85. None of steps (a)–(d) reaches a stray box, and a blank marker for one photo's noise
  would be invention. `TOHOLEYD`/`TOHOLEFT` never became a third box for the total (P5's question).
- **The sign letters, per photo** (b ÷ min(r, g) in brackets). Every sign was decided, and no `L`
  box came back on any photo:

  | photo | letters returned | decided |
  |---|---|---|
  | `IMG_2889` | `R` lit (0.41); `D` lit (0.35), `F` unlit (0.96) | R; D (both, one lit) |
  | `IMG_2891` | `R` unlit (0.98); `F` unlit (0.97) | L (unlit `R` alone); D (unlit `F` alone) |
  | `IMG_2892` | `R` lit (0.32); `D` lit (0.41) | R; D |
  | `IMG_2893` | `R` lit (0.38); `D` lit (0.34) | R; D |
  | `IMG_2894` | `R` lit (0.36); `D` lit (0.34), `F` unlit (0.89) | R; D (both, one lit) |
  | `2026-10-07-1` | `R` lit (0.44); `F` lit (0.48) | R; F |

  Through the shipping reader, lit letters measure 0.32–0.48 and unlit ones 0.89–0.98.
  `IMG_2894`'s unlit `F` (185.3, 182.1, 162.0) is below finding 4's unlit range of 0.94–0.98, but
  still 0.19 above `LIT_BLUE_RATIO`. Step (d) was not needed: no sign was wrong. `IMG_2893`'s `D`
  came back as a single `D` under `full_swing`, not as P3's `DD`, which was read under `hd_golf`.
  The lowest min(r, g) of any letter was 182.1, so `LIT_MIN_BRIGHTNESS` separated nothing here
  either.
- **Merged labels on real photos, and the vote (P4's question).** The merges were:
  - `IMG_2891`: `CLUBSPEEDMPHBALLSPEEDMPH` and `BALLDIRECTIONDEGLAUNCHANGLE`;
  - `IMG_2894`: `BALLDIRECTION DEGLAUNCH ANGLE`, with spaces, which the split's `despace` handles;
  - `2026-10-07-1`: `BALLDIRECTIONDEGLAUNCHANGLE`.

  Each was split, and both halves were read. The vote stopped at its first turn on all six. But a
  merged box hides **two** labels from the vote, not one, so `IMG_2891`'s two merges score it 8 of
  12 (0.667). That is 0.067 above `GOOD_ENOUGH_LABEL_RATIO` (0.6). A third merge would put it at
  0.5, and the vote would try all four turns. That is the least-change option P4 named, so it was
  left as it is.
- **`split_merged`'s cost** (P4's question). Measured with `golf-core parse-screen`, release, best
  of 15: `IMG_2891`'s 35 boxes, with both merges, parsed in 15.1 ms per process. An empty box list
  took 4.6 ms, so the whole parse is about 10 ms, the split included, against a 2–3 minute read.
- **The screen's title lands in the apex cell on three photos.** `Apex Ft`'s `raw_fields` is `39.9
  SHOT DATA` on `IMG_2891`, `71.4 SHOT DATA` on `IMG_2892` and `15.5 SHOT DATA` on `IMG_2893`.
  `APEX FT` is the bottom-left tile, so no next row bounds its cell. The `SHOT DATA` title's centre
  sits these distances below the apex label's bottom, in apex-label heights, and `value_span_ratio`
  6.0 reaches it wherever it is 6.0 or less:

  | `IMG_2889` | `IMG_2891` | `IMG_2892` | `IMG_2893` | `IMG_2894` | `2026-10-07-1` |
  |---|---|---|---|---|---|
  | 6.45 | 5.11 | 6.00 | 4.90 | 6.10 | 7.67 |

  The value is unharmed, because the first number wins.
- **For P15**:
  - **The title in the apex cell.** It is latent, not harmless. A blank apex over the title would
    read `--- SHOT DATA`. That is no blank marker, because `is_blank` compares whole texts, so the
    parser would warn `could not read a number` and `tiles::judged` would name a `misread` where
    `printed_blank` is right. No photo has a blank apex. A ratio in [4.0, 4.8] would leave the
    title out on all six photos and still keep the farthest value P6 measured (3.85 heights, on
    `IMG_2889` only). It was not changed, because the tuning list is allowed only while the target
    is unmet, and it was met before step (a). Name it as a later choice, beside P5's `" | "` gap
    and P6's sign-withheld detail.
  - **`needs_review` stays false where the parser withholds.** It is false on all six photos,
    `IMG_2892` included. `validate.rs:152` sets it only on a failed check or low confidence, so a
    printed-twice disagreement withholds without flagging review, as a tie does. The tile is still
    in `fields_present`, and `tile_refusals` names it `misread`.
  - **Two labels per merged box.** The vote scores the boxes unsplit (P4's finding), and each
    merged box costs it two labels.
- **For P10**:
  - **The dumps were not kept.** The scratchpad belongs to one session, and `data/` is not P9's to
    write. Re-dump `IMG_2889` and `IMG_2891` with `golf-ocr read --device full_swing --boxes`; each
    took 165–175 s here. P9 tuned nothing, so P8's reader is the "tuned reader" P10's first vector
    names.
  - **`IMG_2889`** printed 41 boxes, with the letters' `rgb` equal to P2's and P8's to the tenth,
    so the reader is repeatable. Its apex cell holds only `85.2`, because the title is 6.45 heights
    down. So vector 3's `---` apex is a clean `printed_blank` with no title in the cell.
  - **`IMG_2891`** printed 35 boxes, with label ratio `0.6666666666666666` and confidence 0.999:
    - the header `TOTALYD` at (645, 97), 138×24, and the data row's `TOTAL YD` at (404, 310),
      135×24, confidence 0.968, so `raw_fields` is `203 | 203`;
    - the merged boxes `CLUBSPEEDMPHBALLSPEEDMPH` at (607, 310), 472×24, and
      `BALLDIRECTIONDEGLAUNCHANGLE` at (598, 479), 465×27;
    - `R` at (804, 514), 25×23, rgb (199.9, 206.2, 196.9), giving `raw_fields` `R 0.8`;
    - `F` at (1047, 683), 19×20, rgb (214.8, 213.7, 206.5), giving `F 464`;
    - `Apex Ft` `raw_fields` is **`39.9 SHOT DATA`**, which vector 2's note has to work.
- **For P16: budget the gates.** This session the 13-photo OCR gate took **2066 s**, against P2's
  161 s and P3's 193 s, on the same unchanged reader. The Full Swing gate took 875 s. Run each in
  the background, alone.
- **Verify**:
  - `cargo test`: 1690 passed, 0 failed, 25 ignored. The 25th ignored test is the new gate.
  - `cargo clippy --all-targets` and `cargo fmt --check` are clean.
  - The Full Swing gate passed: 0 wrong, 71 of 72 read.
  - The 13-photo OCR gate passed unchanged.
  - The `data/processed/` digest is `07f37373…` over 192 files.
  - `git diff --stat -- spec/vectors src scripts tests` is empty.
  - Preconditions: the three model directories are in `data/models/ocr/`, ffmpeg is on `PATH`
    (WinGet), and all six photos' digests are the table's.

### P10 (2026-10-30)

- **Landed as planned. Rust agreed with every hand-worked value on the first run**, so neither the
  code nor a vector was wrong, and nothing was fixed. There are three new files in
  `spec/vectors/screen/hand/`: `full-swing-grid`, `full-swing-merged-unlit` and
  `full-swing-withheld`. Each note works the label scores, the splits, the cells, the colours,
  the confidence, `label_ratio` and every warning. `crates/screen/tests/hand.rs`'s `CASES` lists
  ten, and `the_cases_are_the_plans` names both plans.
- **Agreement was checked to the bit, not only to `RTOL`.** `hand.rs` compares through
  `golf_core::compare`, which allows `RTOL`. The bit tests in `parse.rs` and `read.rs` walk the
  three recorded sub-families and not `hand/`. So a throwaway test printed the shipping answer for
  each new vector, and Python compared it with the file exactly. All three were equal, the
  unrounded confidences included: `0.9995100840049631`, `0.9993849196217277` and
  `0.999495317177339`. The test was deleted.
- **The re-dumps match P9's.** `golf-ocr read --device full_swing --boxes` printed 41 boxes for
  `IMG_2889` in 150 s and 35 for `IMG_2891` in 143 s. Every number P9 quoted for them came back:
  positions, `rgb`, the label ratio, `203 | 203`, `R 0.8`, `F 464` and `39.9 SHOT DATA`. The
  reader is repeatable across sessions.
- **The plan missed four tests that assumed every hand vector is `hd_golf`.** All four failed once
  the Full Swing vectors landed:
  - `hand.rs`: `a_withheld_tile_is_located_and_unread_and_nothing_else_is` loaded `hd_golf`'s
    profile, and `what_was_located_tells_the_layouts_apart` required `impact_position_v` on every
    vector but `reference-layout`;
  - `tiles.rs`: `a_tile_the_tie_rule_withheld_is_misread` used `hd_golf`'s labels, and
    `exactly_the_printed_analysed_tiles_with_no_value_are_refused_in_declared_order` used
    `hd_golf`'s declaration.

  The least change was taken. Each test now reads the vector's own device (`input.device`), or the
  shot's (`provenance.device`, through a new `device` helper in `tiles.rs`). The layout test now
  asserts that a vector from another device locates neither HD Golf layout tile. `DEVICE` in
  `tiles.rs` stays for the tests that name an `hd_golf` shot by `id`, and its doc says so.
  `golfer_warnings.rs`, `crates/core/tests/parse_screen.rs` and `crates/core/tests/rerecord.rs`
  walk the same files and passed unchanged.
- **The files' layout follows their directory, not the plan's line-endings rule.** The rule says
  hand-written vectors are LF. That describes `shot/` and `profile/hand/`. All seven siblings in
  `screen/hand/` are CRLF (`git ls-files --eol`), indented by 4, with sorted keys and `°`
  escapes, and the three new files are written the same way. `spec/** -text` keeps the bytes.
- **The `input`'s identity keys**: `shot_id` is `hand-<stem>`, and `session_id` is `hand`.
  `min_confidence` is 0.6, as on the siblings. `timestamp` is the photo's mtime in UTC
  (`2026-10-10T19:36:14Z` and `20:03:46Z`). `image_sha256` and `image_path` are `null`, as on the
  siblings, and the photo's digest is in `source`. The edited vector's boxes are no longer the
  photo's, so a digest in `input` would claim a match that is not there.
- **What the vectors reach.** In `full-swing-grid` the sign comes from a lit `R` with `L` absent,
  and from a lit `D` beside an unlit `F`. In `full-swing-merged-unlit` it comes from an unlit
  letter alone, `R` and `F`. `full-swing-withheld` adds neither letter detected. Between them the
  three cover four of call 5's outcomes on real geometry. The split cuts' runners-up lose by 0.04 (`CLUBSPEEDMPH|BALLSPEEDMPH`: 0.9231 against 0.88 at
  `k` 11 and 13) and 0.03 (`BALLDIRECTIONDEG|LAUNCHANGLE`: 0.9412 against 0.9091 at `k` 15 and 17).
- **For P15**:
  - **The confidence does not see a value a rule withheld.** `score` counts a stored field as read
    when its cell holds text, so a printed-twice disagreement or a sign no letter decides counts
    as a read value. `full-swing-withheld` withholds two values and blanks a third, and still
    scores 0.999. Only the removed `R` box's share of the OCR mean is lost. With `needs_review`
    false as well (P9's finding), the record's only signals are the warnings and `tile_refusals`.
    Name it beside the `needs_review` finding as a later choice.
  - `full-swing-withheld` is the first committed record that reaches P5's `" | "` detail and
    P6's sign-withheld detail. `tile_refusals` names its total `misread` with `'138 | 134' under
    the label, and no value could be read from it`, its ball direction `misread` with `'10.1' …`,
    and its apex `printed_blank`. The details are not in the vector, so changing them later
    re-records nothing.
  - CONFORMANCE §2's screen section should list three more vectors under its heading (the gate
    walks 45 documents), and its `hand/` row should name the Full Swing cases: the four opt-in
    rules on boxes a recognizer printed, with `device: "full_swing"`. `crates/screen/tests/hand.rs`'s
    module doc now says this.
  - Name the four device-aware tests above in ADR-014's M42 addendum, or wherever the hand
    vectors are described.
- **Verify**:
  - `cargo test`: 1690 passed, 0 failed, 25 ignored. That is P9's count, because the new cases are
    files inside existing tests.
  - `cargo clippy --all-targets` and `cargo fmt --check` are clean.
  - The screen dry run (`target/m42/release/golf-core rerecord --declare
    spec/declarations/screen-v1.json --dry-run`) reported `nothing differs from the committed
    vectors` and `45 vectors run (45 screen); 0 changed; 0 files written`. That is 13 corpus, 2
    reference, 20 synthetic and 10 hand.
  - The Full Swing photo gate passed in 890 s. It asserts no wrong value, and P10 changed no code it
    reaches.
  - `git diff --stat -- spec/vectors src scripts tests` is empty. `git status` shows exactly the
    three new files under `spec/vectors/screen/hand/`.
  - Preconditions: the three model directories are in `data/models/ocr/`, ffmpeg is on `PATH`
    (WinGet), and `IMG_2889.HEIC` and `IMG_2891.HEIC` match the table's digests.

### P11 (2026-10-30)

- **Landed as planned, with one deviation in visibility.** `crates/impact` (package `impact`, std
  only, no dependencies): `lib.rs` (the module doc, `BALL_DIAMETER_MM`, P11's eight tunables each
  with its bracket and what it was set from, `ImpactReading`, `ImpactRefusal`, `read_impact`, and a
  crate-private `median`); `frame.rs` (`Frame`, `Frame::is_well_formed`, `luminance`, `Rect` with
  `whole` and `square`); `mask.rs` (`is_white`, `is_light`, `is_dark`, `median_luminance`,
  `Mask::of`); `component.rs` (`components`, 4-connected, in row-major scan order; `Component` with
  `area`, `centroid`, `bbox`, `axis`, `length`, `width`, `ends`, and `equivalent_diameter`, `fill`,
  `aspect`); `ball.rs` (`find_ball`, `Ball`, `Ball::search_region`). The workspace `members` gained
  `crates/impact`, `Cargo.lock` its four-line package entry, and `ort_reach.rs`'s `GUARDED` is eight
  with `"impact"`, its doc saying why a crate with no dependency is guarded.
- **The deviation, for P15**: `find_ball` and `Ball` are **public** (re-exported at the crate root)
  where call 12 and the P11 section said crate-private, because `tests/ball.rs` is an integration
  test and sees only public items. `find_ball`'s doc says so. No other crate uses either; P13 may,
  to mark the ball on a refused clip's review image.
- **What the plan left open, decided here** (P15 documents; P12 builds on it):
  - Coordinates are continuous, pixel `(i, j)`'s centre at `(i + 0.5, j + 0.5)`; centroids, ends and
    the ball's centre are all in them, and `Rect::square` clips by `TextBox.rgb`'s rule.
  - `length` and `width` are the spread of the pixel centres' projections **plus one pixel**, so a
    straight run of n pixels measures n. `ends` is the centroid moved along the axis to each extreme
    projection widened by half a pixel, so the two are `length` apart, the lower projection first.
    `axis` is `(cos θ, sin θ)`, `θ = ½·atan2(2·μ11, μ20 − μ02)` in `(−π/2, π/2]`. A unit test draws
    P12's 60 × 10 bar at 30° and measures θ within 0.01 rad and length and width within 1.5 px.
  - The chain step is measured against the diameter of the candidate **joining** the chain (the
    later frame's). A candidate with two predecessors on equally long chains continues the nearer.
  - A median of an even count is the mean of the middle two (`crate::median`, the ball's centre and
    diameter, and `median_luminance`).
  - `find_ball`, so `read_impact`, **panics** on a frame whose buffer is not `width × height × 3`
    bytes or on frames of two sizes: the caller's bug, documented under `# Panics`.
  - Call 12's fixed numbers are private constants in `ball.rs`: `ASPECT` [0.75, 1.33], `FILL`
    [0.65, 0.92], `CHAIN_STEP` 0.5, `SEARCH_SIDE` 12.0. The tunables are `pub const`s in `lib.rs`.
- **Measured**: a disc of D = 16 px at rest at (250.3, 180.6) on 640 × 480 was found at
  (−0.065, +0.110) px off with D 15.958; one of D = 20 on 60 frames of 960 × 720, at (+0.022,
  −0.089) off with D 20.06, in **92 ms** in release (one whole-frame white mask and labelling per
  frame, noise-free).
- **For P12**:
  - Every item P12's stages read carries `#[allow(dead_code)] // … the allow leaves with it`:
    `frame::luminance`, `Rect::square`, `mask::{is_light, is_dark, median_luminance}`,
    `Component::{axis, length, width, ends}` and `Ball::search_region`. Remove each allow as a stage
    reads the item; one left over means a stage did not use what call 12 says it uses.
  - `read_impact` binds `_ball` and returns `NoClubHead`. `ImpactRefusal::sentence` is not written
    (it is in P12's Files). `Mask`'s `rect` and `bits` are crate-visible for the ball-disc removal,
    which is not written. The frames "where the ball chain is present" are
    `ball.first_frame..=ball.last_frame`; `last_frame` is call 12's `last_ball`.
  - P12's test ball (D = 24) stays inside the size bracket at every tuning P13 may make only when
    the frames' shorter side is in `[24 / 0.04, 24 / 0.02]` = `[600, 1200]`: draw on 960 × 720.
    P11's tests use D = 16 on 640 × 480 for the same reason, and `tests/ball.rs` derives its
    out-of-bracket sizes from the constants, so a P13 tuning does not break it.
- **For P13**: the longest-chain rule prefers **any** white blob that passes the shape tests and is
  on screen in every frame over the ball, which vanishes at impact. The grid's digits mostly fail
  the aspect (taller than wide) and fill tests, but a round mark of the right size (a degree sign
  drawn solid, a dot, a UI disc) would win the chain. Look at what `find_ball` picks on each clip
  first; call 12 allows no new stage, so `BALL_MIN_FRAC` (its floor above the screen's marks) and
  `BALL_MIN_FRAMES` are the only levers, and if neither separates them, record it.
- **Verify**: `cargo test` 1715 passed (P10's 1690 and this crate's 25: 14 unit tests and 11 in
  `tests/ball.rs`), 0 failed, 25 ignored. `cargo clippy --all-targets` reports nothing and `cargo
  fmt --check` is clean. `git diff --stat -- spec/vectors src scripts tests` is empty. Nothing
  under `data/` was touched and no release build was needed.

### P12 (2026-10-30)

- **Landed as planned.** `head.rs` (`Head`, `heads`, `head_in`), `contact.rs` (`contact_frame`) and
  `shaft.rs` (`heel_and_toe`); `read_impact` complete in `lib.rs`, with the offset in a private
  `offset_mm`; `ImpactRefusal::sentence`. Call 12's five P13-tunable numbers are `pub const`s in
  `lib.rs` beside P11's, each doc carrying its bracket and what it was set from: `HEAD_MIN_LENGTH`
  1.5, `HEAD_MAX_LENGTH` 5.0, `HEAD_MIN_ASPECT` 2.5, `CONTACT_MAX_DISTANCE` 1.0 and `SHAFT_MAX_JOIN`
  0.5 (all in ball diameters, the aspect a ratio). The fixed ones are private: `DISC_RADIUS` 0.55
  (`head.rs`), `SHAFT_MIN_LENGTH` 1.5 and `SHAFT_MIN_ASPECT` 3.0 (`shaft.rs`), and `OFFSET_LIMIT_MM`
  60.0 (`lib.rs`, the validator's ±60 for `impact_offset_h`, copied because the crate depends on
  nothing). Two helpers joined P11's modules: `Mask::clear_disc` and
  `component::segment_distance`. **Every `#[allow(dead_code)]` P11 left is gone**, so each item call
  12 names is read by a stage.
- **What the plan left open, decided here** (P15 documents):
  - **Shafts at both ends are no shaft.** When qualifying dark components join *both* head ends (a
    shadow lying along the head would), the frame has found no heel and the earlier frame is asked as
    for none; both failing is `NoShaft`. Taking the nearer join would be a guessed sign.
  - **The fallback frame lends its heel, not its geometry.** Call 12's "or, when none is found there,
    the nearest earlier frame with a head" is read literally (one frame, the nearest earlier with a
    head, and nothing earlier, pinned by `only_the_nearest_earlier_frame_is_asked_for_the_shaft`).
    The heel found there is carried to the contact head by the sign of the two heads' axis dot
    product (ends run along the axis, lower projection first), and the reading's `heel`, `toe` and
    offset are the contact head's, so `contact_frame` and the geometry agree. Axes at exactly right
    angles carry nothing: `NoShaft`.
  - Of two head candidates at one distance in a frame, the first a row-major scan meets. The disc
    taken out of the light mask is the pixels whose **centres** lie within 0.55·D. The frames
    searched are `0..=min(last_ball + 1, frames − 1)`. A distance exactly at `CONTACT_MAX_DISTANCE`·D
    is contact, and an offset of exactly ±60.0 is a reading (`> 60.0` refuses, as the validator's
    range does). `OutOfRange` carries the **rounded** offset. A rounded −0.0 is written 0.0.
  - **The sentences are lower case and unpunctuated**, for call 13's `"impact video: {sentence}"`
    and P13's `<name>: not read - <sentence>`: `the clip held no frames to read`, `no ball could be
    found sitting still in the replay`, `the replay showed the ball but no club head near it`, `the
    club head never reached the ball in the replay`, `no shaft could be seen leaving the club head,
    so its heel could not be told from its toe`, `the strike read {:+.1} mm from the middle of the
    face, outside the 60 mm either way a reading may be`. `every_refusal_says_why_in_words` pins them.
- **The tests' strike** (`tests/impact.rs`, 12 tests): ball D = 24 at (480, 360) on 960 × 720,
  frames 0–5; the head 60 × 10 at 30° coming one D a frame along the face normal, so the face is
  ≈ 7 px clear of the ball in frame 5; in frame 6 (`last_ball + 1`) it lies over the spot the ball
  left, which is the contact frame. The contact head is centred on the half-pixel grid, so the bar
  is point-symmetric and its measured midpoint is its centre exactly: without that the ends'
  rasterization alone moves the midpoint up to ≈ 0.35 px, ≈ 0.6 mm, past the 0.5 mm tolerance.
  **Measured**: the toe-ward strike reads **+6.0** exactly, contact frame 6 of 9, ball D 23.883, the
  heel-ward and swapped-shaft strikes **−6.0**, the 62 mm one `OutOfRange` within 0.5 of 62.
  Beyond the plan's list: shafts at both ends, the shaft hidden at contact (read from frame 5, both
  signs), and only one fallback frame. Unit tests: the disc removal keeps a head touching the ball
  its own bar, the nearer of two bars is the head, a bar outside the shape is none, the head range
  stops one past the ball, contact ties go to the earlier and the reach is inclusive.
- **For P13**:
  - **Every P12 test passes at both ends of all thirteen tunables' brackets** (measured by setting
    each in turn and running `cargo test -p impact --no-fail-fast`). Three **P11** tests pin starting
    values and fail at a bracket end: `mask::tests::the_stills_colours_land_in_the_classes_the_plan_expects`
    at `WHITE_MIN` 170 (the unlit letter turns white) and `LIGHT_MIN` 190 (it stops being light),
    `mask::tests::dark_is_relative_to_the_median` at `DARK_RATIO` 0.4 and 0.8, and
    `tests/ball.rs::of_two_equally_long_chains_the_earlier_wins` at `BALL_MIN_FRAMES` 5 (its chains
    are 4 long). A tuning to one of those must update that test in the same change.
  - `tests/impact.rs` checks its drawing against the tunables in a `const _: () = { assert!(…) }`
    block, so a tuning past the drawing fails to **compile**, naming the clause.
  - A face touching the ball holds its centre ≈ 0.7·D from the head's axis (half a D plus half the
    head's depth), so `CONTACT_MAX_DISTANCE` below ≈ 0.7 leaves contact possible only in a frame
    where the head lies over the ball's spot, typically `last_ball + 1`.
  - Only `find_ball`, `Ball`, `read_impact` and the reading are public. The per-frame heads are
    crate-private, so a refused clip's review image can mark the ball (`find_ball`) but not the
    heads; exposing them would be a new public item, not a new stage, and is P13's call to record.
- **For P14b**: `sentence()` is written to follow `"impact video: "`. The ±60 limit is the
  validator's, copied, so the reader never answers an offset the validator would then refuse.
- **Verify**: `cargo test` 1735 passed (P11's 1715 and this phase's 20: 8 unit tests and 12 in
  `tests/impact.rs`), 0 failed, 25 ignored. `cargo clippy --all-targets` reports nothing and `cargo
  fmt --check` is clean. `git diff --stat -- spec/vectors src scripts tests` is empty, and
  `spec/vectors/` holds only P10's three untracked files. Nothing under `data/` was touched and no
  release build was needed.

### P13 (2026-10-30)

- **For P14b and P15: the reader read 0 of the 5 grid clips, at its starting values and at every
  tuning call 12's brackets allow.** No constant moved. Every clip import will therefore store
  `impact_offset_h` as `misread` (call 15), and the ADR-034 addendum says so. The lines, unchanged
  from the first run to the last (exit 1):

  ```
  IMG_2889.MOV: not read - no shaft could be seen leaving the club head, so its heel could not be told from its toe
  IMG_2891.MOV: not read - no shaft could be seen leaving the club head, so its heel could not be told from its toe
  IMG_2892.MOV: not read - no shaft could be seen leaving the club head, so its heel could not be told from its toe
  IMG_2893.MOV: not read - the club head never reached the ball in the replay
  IMG_2894.MOV: not read - the club head never reached the ball in the replay
  ```

  Each refusal comes after a **wrong ball**. The chain `find_ball` chose sits on the grid or the
  name banner, never in the replay panel: (128.6, 111.8) D 17.2 over frames 4–35 on `IMG_2889`, the
  `O` of `AARON` in the banner; (84.5, 440.2) D 20.6 over 0–51 on `IMG_2891`; (132.0, 277.4) D 20.5
  over 0–50 on `IMG_2892`; (96.6, 453.8) D 19.8 over 0–30 on `IMG_2893`; and (106.7, 386.9) D 19.6
  over 1–58 on `IMG_2894`, all at the 960-wide decode. The replay panel starts at x ≈ 640 on the
  landscape clips and x ≈ 690 on the portrait ones.
- **Why, measured** (with a scratch copy of the crate, now deleted):
  - **The replay's ball is never a candidate at the starting values: it fails call 12's fixed
    `ASPECT` [0.75, 1.33].** Its white component is an ellipse wider than tall, with a bounding box
    of 1.44 (`IMG_2889`, 23 × 16), 1.47 (`IMG_2891`, 28 × 19), 1.38 (`IMG_2892`, 29 × 21), 1.40
    (`IMG_2893`, 21 × 15) and 1.35 (`IMG_2894`, 23 × 17), one frame each with the ball at rest.
    Everything else passes: fill 0.77–0.82, and equivalent diameter 18.2–24.4 px, or 0.024–0.027
    of the shorter side. At the white class's bracket ends the box runs from 1.27 to 1.53, and only
    `IMG_2892` at `WHITE_MIN` 170 with `WHITE_SPREAD` 70 (1.27) gets inside. `ASPECT` is not among
    P13's tunables (P11 made it a private constant by call 12), so it was left alone and its doc
    records this.
  - **The longest chain is always a mark of the screen's own**, as P11 warned: letters of the name
    banner and digits of the grid, 17–26 px across and on screen for 32–58 frames, the whole clip
    or nearly. The ball leaves at impact. `BALL_MIN_FRAC` cannot keep the marks out and the ball in,
    because their sizes overlap it, and `BALL_MIN_FRAMES` is a floor, so it cannot either.
  - **The phone moves.** On `IMG_2889` the whole panel pans over frames 0–3, so even a ball chain
    at rest starts at frame 4.
- **The tuning record.** Each run was the real rule with the named constants changed, over all
  five clips:
  - **Starting values**: 0 read (3 `NoShaft`, 2 `NoContact`).
  - **Each tunable alone, at each end of its bracket** (26 runs: `WHITE_MIN` 170 and 230,
    `WHITE_SPREAD` 25 and 70, `BALL_MIN_FRAC` 0.005 and 0.02, `BALL_MAX_FRAC` 0.04 and 0.10,
    `BALL_MIN_FRAMES` 2 and 5, `LIGHT_MIN` 110 and 190, `LIGHT_SPREAD` 40 and 90, `DARK_RATIO` 0.4
    and 0.8, `HEAD_MIN_LENGTH` 1.0 and 2.0, `HEAD_MAX_LENGTH` 4 and 7, `HEAD_MIN_ASPECT` 2.0 and
    3.5, `CONTACT_MAX_DISTANCE` 0.5 and 1.5, `SHAFT_MAX_JOIN` 0.3 and 1.0): **0 read in every
    run**. Only three runs moved the ball, and each moved it to another mark: `WHITE_MIN` 170
    (`IMG_2889` to (98.6, 180.6) D 29.0, `IMG_2894` to (391.0, 280.9), still on the grid);
    `WHITE_MIN` 230 (three clips `NoBall`, the other two on grid marks lasting 3–4 frames); and
    `BALL_MAX_FRAC` 0.10 (`IMG_2889` to (517.7, 124.8) D 65.8, the white score box beside the
    name). The rest changed only which refusal a clip got.
  - **Two combinations**: first `WHITE_MIN` 170, `WHITE_SPREAD` 70, `BALL_MIN_FRAC` 0.02 and
    `BALL_MAX_FRAC` 0.04, the white class at its widest and the size window at its narrowest; then
    the same with `BALL_MIN_FRAMES` 5, `HEAD_MIN_LENGTH` 1.0, `HEAD_MIN_ASPECT` 2.0,
    `CONTACT_MAX_DISTANCE` 1.5, `SHAFT_MAX_JOIN` 1.0 and `DARK_RATIO` 0.8 added. **0 read** in
    both, and every ball still on a grid or banner mark.
- **Past the ball, for the later look (decision 3), a diagnostic and not a tuning.** In the scratch
  copy only, `ASPECT` was widened to [0.6, 1.6] and the ball chain forced onto the replay's ball.
  At the starting values that still read **0** (3 `NoContact`, 2 `NoShaft`, and `IMG_2894`'s
  "contact" fell at frame 2, before the club arrives). Each of the eight head, contact, shaft and
  class tunables at either bracket end also read 0. One five-constant combination
  (`HEAD_MIN_LENGTH` 1.0, `HEAD_MIN_ASPECT` 2.0, `CONTACT_MAX_DISTANCE` 1.5, `SHAFT_MAX_JOIN` 1.0,
  `DARK_RATIO` 0.8) read `IMG_2893` at **−47.5 mm**, with its "heel" left of the ball's centre,
  which nothing should trust. What the frames show (a tiled panel crop of `IMG_2889`):
  - the club arrives from the lower right with a dark shaft;
  - its head from above is a short light bar, measured 1.50 D long and 0.55 D wide at frame 19,
    right at `HEAD_MIN_LENGTH`;
  - the club is doubled by a ghost image in most frames;
  - head and ball merge into one light blob over the contact frames (20–22);
  - the replay cuts away from frame 24.

  So a reader that works needs a ball stage call 12 did not design: one that accepts the replay's
  ellipse and either confines the search to the replay panel or prefers a chain that ends. It also
  needs a head stage that survives the merge. Recorded, not built: call 12 allows no new stage in
  P13.
- **Landed**:
  - `crates/lab/Cargo.toml` has `impact` and `image` (`=0.25.9`, PNG only), each with its comment.
    The lock gained only those two names in `golf-lab`'s list, with `image` already there through
    `ocr`.
  - `ffmpeg.rs` has `Ffmpeg::decode_frames` and `parse_ppm_stream` (crate-private) with its header
    reader, and the module doc gained a paragraph.
  - `read_impact.rs` is new: `DECODE_WIDTH` (960), `Request`, `Outcome`, `Decode<'a>` (the frames
    source as a `dyn FnMut`), `is_clip`, `line`, `json_line`, `read_impact_clips`, `review_frames`,
    `write_review` and `review_name`.
  - `lib.rs` registers it, and `golf_lab.rs` has `parse_read_impact`/`run_read_impact`, the usage
    line and a `# read-impact` section.
  - In `crates/impact`, docs only: the crate doc, the tunables' block, `BALL_MIN_FRAC`,
    `BALL_MIN_FRAMES` and `ball.rs`'s `ASPECT` carry these measurements. No `impact` test changed.
- **What the plan left open, decided here** (P15 documents):
  - **`decode_frames` adds two flags past the plan's list**: `-fps_mode passthrough`, because
    without it ffmpeg duplicated two frames on the two clips whose stream claims 600 fps (38 for
    `IMG_2889`'s 36 and 33 for `IMG_2893`'s 31, measured by output bytes), and `-enc_time_base
    demux`, because passthrough alone prints `non monotonically increasing dts` at error level on
    those two. With both, all five decode to the container's `nb_frames` (36, 61, 55, 31, 59) with
    an empty stderr and exit 0. A header with a largest sample other than 255 is refused.
  - **The line's contact frame is the reading's 0-based index**, the review image's number. The
    side is `toe` (+), `heel` (−) or `middle` (0.0, printed `+0.0`). A clip ffmpeg cannot decode
    reads `<name>: not read - could not be decoded: <error>`. `--json` prints one object a line:
    `{"clip", "offset_h_mm", "contact_frame", "frames", "ball_center", "ball_diameter_px", "heel",
    "toe"}`, `{"clip", "refusal"}` or `{"clip", "error"}`.
  - **Usage**: a still among the arguments exits 2, naming `import-photos`, before anything is
    decoded, and so does no clip at all. No ffmpeg exits 1, with `FfmpegMissing`'s sentence on
    stderr.
  - **Review**:
    - a reading writes the contact frame ± 2 and a refusal every fifth frame;
    - every image outlines the ball in magenta: a reading's ball, or on a refusal whatever
      `find_ball` chose, so on these five clips **the ring marks the wrong ball**, which is what
      the user should see;
    - the heel-to-toe segment (green) and a toe disc (red) go on the contact frame alone;
    - the `frame-NNN.png` files a previous run left in `DIR/<stem>/` are removed first, and nothing
      else there is touched.

    P12 left the per-frame heads crate-private and P13 kept them so, so a refusal's images show no
    head. Exposing them is a new public item, and nothing needs it until a later look at the reader.
- **The review images** are under `data/raw/impact-review/<stem>/` (IMG_2889, IMG_2891 to IMG_2894):
  60 PNGs, 57 MB, every fifth frame of each clip at 960 wide, from the final run.
- **For P14**:
  - `decode_frames(clip, None)` decodes these clips at **1744 × 1308** (1308 × 1744 for the
    portrait two), not the 1920 × 1440 `ffprobe` reports, because ffmpeg 9.0.2 applies the
    container's crop. `DECODE_WIDTH`'s 960 is scaled from that.
  - A full-size decode holds all of a clip's frames at once: 6.8 MB a frame, so 417 MB for
    `IMG_2891`'s 61, 246 MB for `IMG_2889`'s 36. Whether `read_clip_grid` keeps them all is P14's
    call.
- **For P14b**: reuse `golf_lab::read_impact::DECODE_WIDTH` for call 14's `Some(960)`. Its `Decode`
  alias is the closure form of the plan's `MotionFrames`. P14b may make `read_impact_clips` take the
  trait instead, as long as `tests/golf_lab.rs`'s `read_impact` case and the module's tests still
  pass.
- **Verify**:
  - `cargo test`: 1748 passed (P12's 1735 and this phase's 13: four `ffmpeg` unit tests for the PPM
    parser and the missing clip, eight `read_impact` unit tests for the line, the JSON, the clip
    test, the review's frames, the run with a fake decoder, the drawing and the review names, and
    one in `tests/golf_lab.rs` for the refusals and the missing clip, with `GOLF_FFMPEG` pinned so
    it holds on a box without ffmpeg), 0 failed, 25 ignored.
  - `cargo clippy --all-targets` reports nothing, after a `type_complexity` warning was answered
    with the `Decode` alias. `cargo fmt --check` is clean.
  - The release build went into `target/m42`.
  - `git diff --stat -- spec/vectors src scripts tests` is empty, and `spec/vectors/` holds only
    P10's three untracked files. The `data/processed/` digest is `07f37373…` over 192 files.
  - Nothing under `data/` was written but `data/raw/impact-review/`.

### P14 (2026-10-30)

- **For P14b, P15 and P16: the clip gate reads all 60 values with none wrong, by the sharpest
  frame, and no profile key was tuned.** The rule kept is call 14's default
  (`golf_lab::shot::GRID_FRAME = GridFrame::Sharpest`). Both rules, measured through the gate on
  the five grid clips:

  | clip | sharpest frame | middle frame (`len / 2`) |
  |---|---|---|
  | `IMG_2889` | 31 of 36: 12 read | 18 of 36: 12 read |
  | `IMG_2891` | 17 of 61: 12 read | 30 of 61: 12 read |
  | `IMG_2892` | 21 of 55: 12 read | 27 of 55: 12 read |
  | `IMG_2893` | 29 of 31: 12 read | 15 of 31: 11 read, `sidespin` withheld |
  | `IMG_2894` | 41 of 59: 12 read | 29 of 59: 12 read |
  | **total** | **60 of 60, 0 wrong** | **59 of 60, 0 wrong** |

  The middle frame's one gap is call 5 working: `IMG_2893`'s frame 15 returned only an `R` box, no
  `D` or `F`, so the warning `Sidespin Rpm: neither D nor F could be read as lit - sign unknown,
  not stored` withheld the value. **No frame either rule chose showed the previous shot's
  numbers**, call 14's worry. No clip carried a note, a warning or `needs_review` under the
  sharpest rule.
- **The clip reads one value its still withholds**: `IMG_2892`'s `total_distance`. The stray
  `85TAS` box P9 found over the still's header copy is not in either frame, so both copies read
  `82` and agree. P15 can say so: on these five, the clip's grid reads 60 of 60 where the stills
  read 59 of 60 of the same values (P9's 71 of 72 includes `2026-10-07-1`, which has no clip).
- **The sign letters off a clip's frame**, b ÷ min(r, g). Under the sharpest rule only `R` and `F`
  came back, never `L` or `D`, so every sidespin sign was decided from an unlit `F` alone (call 5's
  "detected and unlit → the other letter's sign"):

  | clip | `R` | `F` |
  |---|---|---|
  | `IMG_2889` | lit 0.29 | unlit 0.94 |
  | `IMG_2891` | unlit 0.96 | unlit 0.95 |
  | `IMG_2892` | lit 0.26 | unlit 0.86 |
  | `IMG_2893` | lit 0.28 | unlit 0.90 |
  | `IMG_2894` | lit 0.39 | unlit 0.88 |

  The lit letters measure 0.26 to 0.39 and the unlit ones 0.86 to 0.96, so `LIT_BLUE_RATIO` (0.7)
  still separates them with room on both sides. The lowest unlit value, `IMG_2892`'s 0.86, is
  below P9's stills (0.89) and 0.16 above the threshold. The lowest min(r, g) of any letter was
  172.2, so `LIT_MIN_BRIGHTNESS` (120) separated nothing here either. The middle rule's frames did
  return `D`, lit at 0.34 and 0.26 on `IMG_2889` and `IMG_2892`.
- **Landed**:
  - `crates/impact/src/frame.rs` has `pub fn sharpest_frame(frames: &[Frame]) -> Option<usize>`,
    with call 14's definition and the private helper `gradient_energy`. `lib.rs` re-exports it,
    and the crate doc names it as the one public item that is not a stage of the replay. A frame
    that is not well formed is never chosen. Three unit tests: a smeared copy loses wherever it
    sits, ties go to the earliest, and no frames (or none well formed) give `None`.
  - `crates/lab/src/shot.rs`:
    - the module doc has a section for the clip's half;
    - `GridFrame { Sharpest, Middle }` with `choose`, and `GRID_FRAME`, whose doc carries the
      table above;
    - `GridWritten { frame, frames, digest }` and `write_grid_frame(clip, png, rule, decode)`;
    - `encode_png(frame)`, with `image`'s `PngEncoder` defaults;
    - `ClipGrid { shot, frame, frames }` and `read_clip_grid(clip, png, rule, new, profile,
      options, decode, reader)`.

    Four unit tests cover the rule, the encoding, the read and the refusals.
    `crates/lab/src/lib.rs`'s `shot` bullet names the clip's half and the gate.
  - `crates/lab/tests/full_swing_clips.rs` holds the one ignored test,
    `every_grid_clip_reads_as_its_screen_prints_or_withholds`. Its form, truth table and digests
    are P9's for the five clips. It asserts no wrong value and `READ_AT_LEAST = 60`, a floor rather
    than an equality so a later improvement cannot fail it. A `Keeping` reader wraps `OcrReader` so
    the gate can print each frame's sign letters. The gate runs the kept rule only: measuring the
    middle rule took a second, temporary test, which was removed once it had run. `Middle` stays
    runnable through `GridFrame`, by the house convention for a rejected alternative.
- **What the plan left open, decided here** (P15 documents):
  - **`read_clip_grid` takes the rule as an argument**, eight arguments under
    `#[allow(clippy::too_many_arguments)]`. The precedent is `import_export`'s `swing`, and the
    doc says why. The importer passes `GRID_FRAME`.
  - **It reads and stores nothing else.** It is a new import every time: no store lookup, no write
    but the PNG, `stale` `None`, and the identity is `new`'s. The shot's photo is the PNG: its
    path, its digest, and its mtime, which is set to the clip's. So the shot's timestamp is the
    clip's (`ocr::screen_timestamp`), held by a unit test. The mtime is set through a second
    handle after the bytes are written and closed, because Windows may move a file's write time
    when the handle that wrote it closes.
  - **Every frame is held** while the rule runs. That was kept rather than streamed, and the doc
    says why.
  - **The frames source is P13's `Decode` alias** (`&mut Decode<'_>`), not the plan's
    `MotionFrames` trait, which P14b's Files line owns.
- **For P14b**:
  - **Reuse the split rather than `read_clip_grid` whole** if `import_video` wants the store's
    cache. `write_grid_frame` returns the PNG's digest. Then `import_photo` on the written PNG
    gives the still's cache rule (`cached` for a current shot under that digest), the
    `{session}-{digest[:12]}` stamp, the store write and `--dry-run`, with no second copy of any of
    them. For a dry run the PNG still has to be written somewhere for the reader, so a scratch
    path is needed (the PNG is the reader's input).
  - **Determinism**: `encode_png` is pinned by
    `a_frame_encodes_to_the_same_png_twice_and_decodes_to_its_pixels`. ffmpeg's decode with
    `decode_frames`' arguments gave the same bytes twice on `IMG_2893.MOV` (sha256 of the whole
    PPM stream `e62ab86b…`, ffmpeg 9.0.2). No test pins the real decode; call 14's "one clip
    imported twice lands on one entry" test pins it with the fake decoder, as the plan lists.
  - **Switching to `MotionFrames`** touches `read_clip_grid`, `write_grid_frame`, two unit tests
    in `shot.rs` (`a_clips_grid_frame_is_read_as_a_photo_stamped_with_the_clips_time` and
    `a_clip_with_no_frame_or_no_decode_is_refused_naming_it`) and the gate's `run`, which builds
    its decoder from `Ffmpeg::find()`.
  - **Two decodes per clip**, if the impact read stays at `Some(DECODE_WIDTH)` while the grid
    reads at `None`. Each full-size decode buffers ffmpeg's whole stdout and then copies each
    raster out of it, so `IMG_2891` peaks near twice its 417 MB of frames. That held on this
    desktop; it is a note, not a problem.
- **For P16: budget the clip gate at about 11 minutes** (643 s in its final form, 114–137 s a
  clip, run alone), and P9's photo gate at about 15 (882 s this session, unchanged from P9's 875
  s). The two-rule measuring run took 1294 s.
- **Verify**:
  - `cargo test`: 1755 passed (P13's 1748, three `impact` tests and four `shot` tests), 0 failed,
    26 ignored. The 26th ignored test is the clip gate.
  - `cargo clippy --all-targets` reports nothing, and `cargo fmt --check` is clean.
  - The clip gate passed in its final form: 60 of 60 read, 0 wrong, with each clip's frame and
    read identical to the measuring run.
  - P9's photo gate passed unchanged: 71 of 72 read, 0 wrong, the same table as P9's.
  - Release builds and tests went into `target/m42`.
  - The `data/processed/` digest is `07f37373…` over 192 files.
  - `git diff --stat -- spec/vectors src scripts tests` is empty, and `spec/vectors/` holds only
    P10's three untracked files.
  - Preconditions: the three model directories are in `data/models/ocr/`, ffmpeg is on `PATH`
    (WinGet), and the five clips' digests are the table's.
  - Nothing under `data/` was written: the gate's PNGs go to a temporary directory.

### P14b (2026-11-02)

- **For P15 and P16: `import-video` works, and every real clip stores its numbers with the offset
  `misread`, as P13's 0 of 5 said it would (call 15).** The recorded run, exit 0, writing nothing:

  ```
  $ golf-lab import-video --device full_swing --dry-run data/raw/shot_screens/IMG_2889.MOV

  Importing 1 clip(s) as session 'screens-2026-11-03' [full_swing]
    IMG_2889.MOV                 dry-run conf=1.00
        carry=131 total=134 ball=102.5 club=95 smash=1.08 path=-2 face=13.2
        impact video: no shaft could be seen leaving the club head, so its heel could not be told from its toe

  1 shot(s) imported cleanly, 1 of them with no impact offset.
  ```

  The clip gate now runs the replay too, and the five refusals are P13's five: `NoShaft` on
  `IMG_2889`, `IMG_2891` and `IMG_2892`, `NoContact` on `IMG_2893` and `IMG_2894`.
- **Landed**:
  - `devices.json`: `full_swing` declares `impact_offset_h` last, `from: impact_video`, with the
    plan's note, and the top `_comment` explains `from`.
  - `capability.rs`: `DeviceCapability::reads_impact_video`. `FieldSource` was already there (P8).
  - `tiles.rs`: a field `from: impact_video`, printed, analysed and empty, is `misread` with
    `IMPACT_VIDEO_MISREAD` (public, re-exported by `screen`) and no tile is looked for. The module
    doc's table has the row.
  - `shot.rs`:
    - the `MotionFrames` trait, implemented for `Ffmpeg` and for any closure of its shape;
    - `read_impact_video`, `record_impact_outcome` (call 13's three outcomes, in one place) and
      `IMPACT_VIDEO_WARNING`;
    - `import_photo_with`, which is `import_photo` with a hook between the read and the store;
    - `read_clip_grid` runs the replay after the grid, so the gate exercises it.
  - `read_impact.rs`: P13's `Decode` alias is gone. `read_impact_clips` takes `MotionFrames`, and its
    tests' closures needed no edit. P14's note expected the switch to touch two unit tests and the
    gate; the unit tests only moved one assertion (two decodes now), and the gate passes `Ffmpeg`.
  - `import.rs`: `import_video` and `clips_in` beside `import_photos` and `images_in`, both over one
    `run_import`; `Capture` (what the row names, the photo's bytes, the clip) threads through the
    photo steps, which are otherwise unchanged. `tests/import_photos.rs` passes unedited.
  - `golf_lab.rs`: `import-video` dispatched through `parse_import`, which now takes the verb, with
    its usage lines and a `# import-video` section.
  - Tests: `tests/import_video.rs` (11, a fake decoder and a strike the real reader reads), and one
    each in `tests/golf_lab.rs`, `screen/tests/tiles.rs` and `import.rs`, four in `shot.rs`.
    `contracts/tests/capability.rs`'s two pins were moved as P8's finding said.
- **What the plan left open, decided here** (**For P15**, who documents them):
  - **A refused replay moves neither `needs_review` nor the exit code.** A flag sets a shot's numbers
    aside (the corpus's `shot_needs_review`), and the grid's numbers are as good as they were. The
    line under the row says what the replay gave, and the last line counts the shots with no offset.
  - **A store-only shot's `image_path` names the clip**, because its PNG is scratch and goes with
    the run. An entry's shot names the entry's copy, as a still's does. The scratch directory is
    under the system's temporary one, per run, removed when the run ends; `tempfile` is only a
    dev-dependency here and no new dependency is allowed.
  - **An entry's manifest says `original_filename: IMG_2889.png`, `content_type: image/png`**, since
    the photo is the frame and `content_filename` takes the suffix from that name.
  - **`CLIP...` is required**: no default directory, by call 14's usage, and the default one holds
    `IMG_2888.MOV`, which no profile reads. The default device is `hd_golf`, so `import-video` with no
    `--device` exits 2 naming `full_swing`, before any model or ffmpeg is looked for.
  - **No ffmpeg fails each clip, not the verb** (exit 1, each clip's row `failed`), so the device
    refusal above holds on a machine with none. `read-impact` still exits 1 at the start.
  - **A still named to `import-video` is warned about and skipped**, as `images_in` skips a non-photo;
    `read-impact` refuses one with exit 2. The verbs differ because `import-video` keeps
    `import-photos`' handling.
  - **The clip is copied after a shot was read**, so a read of nothing has no clip to clean up. A
    failed copy is said on stderr and counted as unreadable, and the shot stands; `--force` copies it
    again. A clip named as its own copy is left alone (`copy_with_time`), because `fs::copy` onto
    itself truncates on some platforms.
- **Past the plan, and small: a stale shot's re-read carries what its replay gave**
  (`carry_impact_video`, in `read_screen`). The re-read reads the PNG, which holds no replay, so
  without this a `SCREEN_PARSER_VERSION` bump would silently turn every clip's shot into a still's on
  its next `shot_for`, dropping both the offset and the `misread`. `hd_golf`'s shots never list the
  field, so nothing committed moves. `import-video` on the clip reads the replay again and replaces
  what was carried.
- **For P15: the reader has no version key.** A current stored shot is `cached` on its parser version
  alone, so when `crates/impact` is tuned later, a shot already stored keeps its old answer until
  `import-video --force`. Say so where the docs describe re-reading the impact.
- **For P15: two sentences in `crates/contracts/src/strike.rs` age with this phase.** `CENTERED_RADIUS_MM`'s
  doc says "no device supplies an offset yet" (line 81, already in your Files line through call 16),
  and `StrikeProfile.impacts`'s says "every real device until M42" (line 946). Both are still true of
  stored shots: a device now declares the offset and the reader has yet to deliver one.
- **For P16**:
  - Call 17's commands are right as written: `golf-lab import-video --device full_swing <the five
    .MOV>` reports `5 of them with no impact offset`, and exits 0.
  - **The gates ran far faster here than P14 recorded**: the clip gate in 50 s (9–11 s a clip) against
    643 s, and the photo gate in 61 s against 882 s. This phase did not find out why, and nothing in
    it touches the OCR, so keep P14's budget as the ceiling.
- **Verify**:
  - `cargo test`: 1773 passed (P14's 1755 and this phase's 18), 0 failed, 26 ignored.
  - `cargo clippy --all-targets` reports nothing, and `cargo fmt --check` is clean.
  - The clip gate passed: 60 of 60 read, 0 wrong, the same frames as P14's (31, 17, 21, 29, 41), no
    `needs_review`.
  - P9's photo gate passed unchanged.
  - Release builds and tests went into `target/m42`. After those runs only `import.rs` changed (the
    copy guard and a lookup's spelling) and `golf_lab.rs`'s doc, none of it on either gate's path, and
    `cargo test`, clippy and fmt were run after them.
  - The `data/processed/` digest is `07f37373…` over 192 files, and no real import was run.
  - `git diff --stat -- spec/vectors src scripts tests` is empty, and `spec/vectors/` holds only P10's
    three untracked files. `ANALYSIS_VERSION` 17, `SCREEN_PARSER_VERSION` 1, `CAREER_VERSION` 3.
  - Preconditions: the three model directories are in `data/models/ocr/`, ffmpeg is on `PATH`
    (WinGet), and `IMG_2889.MOV`'s digest is the table's.

### P15 (2026-11-02)

- **Landed as the section lists.** `docs/decisions/034-shot-first-phone-first.md` and
  `docs/decisions/014-screen-capture-shot-ingestion.md` each have an M42 addendum, dated 2026-11-02,
  and each "For P15" above went into one of them, into `docs/ARCHITECTURE.md`, or into
  `docs/CONFORMANCE.md`. `docs/README.md`'s two rows, `data/README.md`, `CLAUDE.md` and §M38 of
  `docs/plans/m31-m40-shot-first-pivot.md` say what the section asks. Only doc comments changed in
  `crates/contracts/src/strike.rs`, both sentences P14b named.
- **Beyond the section's list, each because a pin or a true sentence needed it**:
  - `docs/README.md`'s total went from 118 to 120 addenda. The section says "nothing else before
    P16", but `test_the_map_counts_the_addenda_correctly` pins that number beside the per-row counts.
    The document count was not touched.
  - `spec/README.md`'s one line on `screen/hand/` names Full Swing's rules.
  - `docs/CONFORMANCE.md` also moved outside §2's screen section: §1 has the two keys and
    `WRITTEN_WHEN_SET`, §4 the clip gate's command, and §5 "thirteen in the workspace" became fourteen.
    The screen heading says 51 vectors (48 and P10's 3). The new section is "The Full Swing gates,
    photo and clip", after the OCR gate.
  - `CLAUDE.md` and `docs/ARCHITECTURE.md` said "M29 added the last three crates", now false, so both
    say "three". `docs/ARCHITECTURE.md`'s reviewed line is 2026-11-02.
  - ADR-034's addendum corrects its M37 addendum's two sentences on M42 (a feasibility spike, and
    revising `CENTERED_RADIUS_MM`), since neither happened.
- **`backspin` and `sidespin` are "no analysis reads them", not "nothing reads them"**: `golf-mcp`'s
  `NUMERIC_METRICS` lists both (call 19). The addendum, the README row and ARCHITECTURE say it that way.
- **A consequence the addendum states, read from the code and not run**: strike location exists from
  `capability::analysed_on`, and a refused clip's shot lists `impact_offset_h`. So a club whose entries
  came from clips has the topic named, with those shots unscored under `misread` and the grade withheld
  rather than low. No stored shot reaches this and no vector does, since nothing was imported (call 17).
- **For P16**:
  - **`data/README.md` is tracked**, so the "never `data/`" in your step 6 does not cover it. `git add`
    it by name, with `spec/README.md`. `git status --short data/` should show that one file.
  - `docs/README.md`'s document count and this plan's map row are still yours, as the rules say.
  - `CLAUDE.md`'s M37 paragraph still says a measured offset is what "no device in use prints until
    M42". It was left, because the section says "nothing else" for that file. After M42 a device
    declares the offset and the reader has yet to deliver one, so it reads a little ahead of the truth.
    The fix is one clause, and the choice is yours or a later session's.
  - ROADMAP's §M42 may still promise the spike and the `CENTERED_RADIUS_MM` revision. This phase did not
    read it. ADR-034's M42 addendum is the wording to match.
- **For a later session on this box**: `python3` is the Windows Store stub and hangs waiting on stdin,
  which cost a 120 s timeout here. Use `.venv/Scripts/python.exe -I`. `CLAUDE.md`, `data/README.md` and
  `docs/{README,ARCHITECTURE,CONFORMANCE}.md` are CRLF in the working tree, and their edits kept that.
- **Verify**:
  - `pytest tests/test_docs_truth.py`: 91 passed.
  - `cargo test`: 1773 passed, 0 failed, 26 ignored, the same as P14b.
  - `cargo clippy --all-targets`: 0 warnings. `cargo fmt --check`: clean.
  - `git diff --stat -- src scripts tests spec/vectors` is empty, and `spec/vectors/` holds only P10's
    three untracked files.
  - The `data/processed/` digest is `07f37373…` over 192 files. This phase ran it though the verify table
    does not ask, and it wrote nothing there.
  - The throwaway patch scripts lived in the scratchpad and are deleted.

### P16 (2026-11-02), stopped before step 2

- **Stopped: the full `pytest` is red on one frozen pin, and the rules forbid its fix.** The rule
  "Every phase ends green" says to stop and record why, and "Frozen Python is not edited" allows an
  edit under `tests/` only in `RUST_OWNED_SCHEMAS`. So the row stays 🔄, and nothing under `tests/`
  was touched. Steps 2–7 (WORKLOG, ROADMAP, the map's count, memory, the commit, the close report) are
  **not done**, because each would say M42 closed while the suite is red. The user's answer is needed.
- **The failure**: `tests/test_conformance.py::test_a_screen_document_carries_what_a_port_reads`,
  line 1145, `assert all(set(box) == box_keys for box in vector["input"]["boxes"]), name`, with
  `AssertionError: hand/full-swing-grid.json`. `box_keys` is exactly six keys, and P10's three
  `spec/vectors/screen/hand/full-swing-*.json` carry `rgb` on their boxes, which is what they exist to
  test (call 4, P6's lit rule). The whole suite: 1 failed, 2083 passed.
- **Why no phase saw it**: the verify table lists the full `pytest` at P1 and P16 only. P1 came before
  `rgb` and P10 ran `cargo test`, so this is the first run since P10's vectors landed. P10's three files
  are the plan's own, so the plan's two rules collide here and no phase went wrong.
- **Measured, read-only**: the pin's loop over the 45 documents it walks, with `rgb` optional on a box,
  finds nothing else wrong, and only those three fail the strict form. The probe was a scratch script,
  now deleted. Every other pytest test passed, so this is the only frozen pin in the way.
- **The options, for the user**:
  - **(a), recommended**: one clause in that pin, `set(box) - {"rgb"} == box_keys`, as a recorded second
    exception to the "not edited" rule. ADR-035 §4 fixes the frozen lab when it breaks, and names "a
    refusal to read something Rust wrote". No vector or behaviour moves.
  - **(b)**: move the three vectors out of what `_screen_by_family` walks, which needs a new family and
    a change to the Rust runner, and exists only to dodge the pin.
  - **(c)**: drop `rgb` from the vectors. Not workable: their sign and sidespin expectations rest on it.
- **The user's answer (2026-11-02): (a).** The resumed P16 changes that one clause in
  `tests/test_conformance.py:1145` to `set(box) - {"rgb"} == box_keys`, and nothing else under `tests/`,
  and records it as the second exception to "Frozen Python is not edited". Only this pin is approved.
- **For the resume**: the results below stay good if only that test file and docs change. Then run the
  full `pytest` and `tests/test_docs_truth.py` again and carry on from step 2; no gate needs repeating.
  P14b's and P15's "For P16" items all still stand. And **the map has no row for a plan**:
  `docs/README.md`'s intro says a plan is "listed in the count and not in a table below", so step 4 is
  the count sentence alone: 85 documents, 72 in `docs/`, 11 in `plans/`. Check them after `git add`.
- **Verify, this run**, each command under "Verify" that lists P16:
  - `cargo test`: 1773 passed, 0 failed, 26 ignored. `cargo clippy --all-targets`: nothing.
    `cargo fmt --check`: clean.
  - `pytest tests/test_docs_truth.py`: 91 passed. **`pytest`, run alone: 1 failed, 2083 passed** (344 s).
  - `ruff`: clean. `mypy`: clean over 115 files. `conformance.py check`: exit 0.
  - The 13-photo OCR gate passed in 139 s. The Full Swing photo gate passed in 64 s, 71 of 72 read and
    0 wrong. The clip gate passed in 65 s, 60 of 60 read and 0 wrong. P14b's fast gates hold, so P9's and
    P14's budgets stay ceilings and the cause is still unknown.
  - Dry runs, each 0 changed and 0 files written: `v17` 140 vectors, `screen-v1` 45, `career-v3` 175.
  - The `data/processed/` digest is `07f37373…` over 192 files.
  - `git diff --stat -- spec/vectors src scripts tests` is empty, with P10's three files the only
    untracked ones there. `ANALYSIS_VERSION` 17, `SCREEN_PARSER_VERSION` 1, `CAREER_VERSION` 3.
  - Preconditions: the three model directories are present, ffmpeg is on `PATH`, and all thirteen
    photo and clip digests are the table's.

### P16, resumed (2026-11-03)

- **Resumed from the marker, on the user's (a).** `find` over `crates src tests scripts spec docs` newer
  than the marker listed this file alone, so no code had moved since the run above and, as that entry
  says, no gate was repeated. The one change under `tests/` is the approved clause,
  `tests/test_conformance.py`'s `set(box) - {"rgb"} == box_keys`. It is recorded in "Rules every phase
  keeps" as the second exception.
- **Steps 2–5 landed as the section lists.** `docs/README.md` reads 85 documents, 72 in `docs/` and 11 in
  `plans/`, and `git ls-files` agrees once this file is staged.
- **Beyond the section's list, each because a sentence would have been false**:
  - `CLAUDE.md`'s M37 clause is the one P15 offered: a measured offset is what "no stored shot holds yet".
  - ROADMAP's §M42 keeps its ask as the record, as the file's header says detail sections do, and its
    Status says the spike and the `CENTERED_RADIUS_MM` revision did not happen, in the addendum's terms.
  - §M41's Status says what became of each open question, and that no version key moved against its Exit.
  - §M38's Status has one sentence on what M42 asks of the phone, pointing at the program plan.
- **For a later session on this box**: this plan's warning about Git Bash heredocs is real. A patch script
  written by one came through with its `—` mangled and its `\\` halved, and failed before it wrote
  anything. Write such a script with the Write tool.
- **Verify, this run**:
  - `pytest`, run alone: **2084 passed**, 0 failed (328 s), with the clause in.
  - `pytest tests/test_docs_truth.py` after every doc edit, against the count of 85: 91 passed.
  - `ruff check src tests scripts`: clean, since the test file changed. `cargo test -p golf-mcp --test docs`,
    which reads ROADMAP: 2 passed.
  - `git diff -- src scripts spec/vectors` is empty, and `spec/vectors/` holds only P10's three new files.
  - Every other result is the run above, which still stands: nothing but this test file and docs changed.
