# Plan: M46 — The impact drill: STRIKEPRO tape, the ladder, and the golf-ball navigation

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against code
before acting on one. Phase findings are appended as phases close, and a finding that contradicts the plan
wins, except that a decision (D-number) moves only with the user's word.

**Planned**: 2026-11-12, on the Windows desktop, by interview (five rounds). **Milestone**:
[ROADMAP §M46](../../ROADMAP.md#m46-impact-drill--impact-tape-first). This file is the phase list, and
where it and the ROADMAP disagree, **this file wins**.

**Governing decisions**: [ADR-010](../decisions/010-benchmark-ranges.md) §2 (no score beats a wrong one),
[ADR-034](../decisions/034-shot-first-phone-first.md) §3 (a drill is untracked) with its M37 and M42 addenda,
[ADR-035](../decisions/035-rust-everywhere-python-where-required.md) clauses 1 and 4 (new work is Rust),
[the M38 plan](m38-iphone-app.md)'s rules 7 to 10 and calls C7, C10, C13, C15, C16 and C22, and
**ADR-037**, which P1 writes from this plan.

**Level: L3.** A new subsystem (the drill history), two new `contracts` modules (`face_impact`, `drill`),
a new version key (`TAPE_READER_VERSION`), a second method in `crates/impact`, and the app's navigation
replaced. No existing shape changes, and `ANALYSIS_VERSION`, `SCREEN_PARSER_VERSION` and `CAREER_VERSION`
do not move (rule 4).

**How this runs.** Each phase is tagged with where it runs:
- 🪟 **any desk**: Rust and docs. The Windows box is fine, and so is the Mac.
- 🍎 **the Mac**: Flutter, Swift and the Simulator.
- 🧍 **attended**: you do something only you can do. A `phase-runner` reports `BLOCKED` at each of these by
  design, and you run it with `/next-phase`.

M38 is still open (its bay phases are set aside, D16), and M46 interleaves with it freely. **Moving between
machines is the user's to do**: a phase whose tag differs from the last phase's lists "push on the old
machine, pull on the new" as its precondition.

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot, and address anything a later phase must
   know to that phase by number ("**For P17**: …");
3. commit (rule 2), stop, and tell the user to `/clear` and run `/next-phase`.

| Phase | What | State |
|---|---|---|
| **P0** | 🪟 This plan, the ROADMAP pointer, and the plan's commit | ✅ Done *(2026-11-12)* — approved by the user |
| **P1** | 🪟 ADR-037: where on the face, by method, and the drill framework | ✅ Done *(2026-11-12)* |
| **P2** | 🪟 The truth sheet and the tape session's run sheet | ⬜ |
| **P3** | 🧍 The tape session: labels measured, strikes photographed, truth sheets photographed | ⬜ |
| **P4** | 🪟 The session's inventory and the label geometry as data | ⬜ |
| **P5** | 🪟 `contracts::face_impact`: the method interface and `TAPE_READER_VERSION` | ⬜ |
| **P6** | 🪟 `contracts::drill`: the strike record, the stages and the ladder tables | ⬜ |
| **P7** | 🪟 `impact::tape` geometry: the homography and the rectified crop | ⬜ |
| **P8** | 🪟 `golf-lab read-tape`: the desk tool and its review images | ⬜ |
| **P9** | 🪟 The label finder, on P3's photos | ⬜ |
| **P10** | 🪟 Orientation: heel and up, from the hosel | ⬜ |
| **P11** | 🪟 The mark on a fresh label | ⬜ |
| **P12** | 🪟 The newest mark: the diff against the last strike | ⬜ |
| **P13** | 🪟 The truth reader, the measured error, and the real-photo gate | ⬜ |
| **P14** | 🪟 `analysis::drill`: the baseline and the ladder | ⬜ |
| **P15** | 🪟 `analysis::drill`: the pattern, the triggers and differential judging | ⬜ |
| **P16** | 🪟 `feedback::drill_rules`: the sentences | ⬜ |
| **P17** | 🪟 `storage::drill_store`: the drill history on disk | ⬜ |
| **P18** | 🪟 Facade: `record_tape_photo`, the pick and "missed the tape" | ⬜ |
| **P19** | 🪟 Facade: the drill's actions (start, stage, suggestions, calls) | ⬜ |
| **P20** | 🪟 Facade: the progress views (dots, level, pattern, heat map, roll-up) | ⬜ |
| **P21** | 🪟 Export carries `drills/`, and `golf-lab import-export` keeps it | ⬜ |
| **P22** | 🍎 Swift: the photo decoded for Rust, and the tape camera's overlay | ⬜ |
| **P23** | 🍎 The Drills page: picker, strike loop, reading card, pick and missed tape | ⬜ |
| **P24** | 🍎 The Drills page: baseline, block end, suggestions, differential calls | ⬜ |
| **P25** | 🍎 The progress view: dots and level, the heat map, the roll-up | ⬜ |
| **P26** | 🍎 The golf-ball navigation, as a widget on its own | ⬜ |
| **P27** | 🍎 The ball replaces the tab bar, and every test is moved onto it | ⬜ |
| **P28** | 🍎 The whole drill on the Simulator, over P3's photos: desk-complete | ⬜ |
| **P29** | 🧍 A real drill session on the iPhone, exported and imported at the lab | ⬜ |
| **P30** | 🪟 Docs | ⬜ |
| **P31** | 🪟 Close: verify, WORKLOG, ROADMAP, memory, and the close commit | ⬜ |

---

## The ask, in one paragraph

The golfer opens **Drills**, picks **Impact**, then the method (**Tape: STRIKEPRO**), then a club. They put a
STRIKEPRO label on the face, hit, and photograph the label on the club after **every** strike. The app finds
the newest mark and records where it landed, in millimetres from the label's centre (+ toe, + high). A
**baseline** of 10 strikes sets the club's **level**, a target circle from a table per club category. Then
come **blocks of 10**: 7 or more inside moves the circle in a step, 3 or fewer moves it out, and anything
else stays. When the pattern calls for it, the app suggests **recentring** (the pattern sits off centre) or
**differential practice** (the pattern is too big: random heel, centre and toe calls). The golfer accepts or
declines, and the app remembers the club's stage. Progress shows per club and per category as dots on the
label, the level's circle, the pattern's centre and spread, and a heat map. Strikes are a drill's alone and
never feed the shot corpus. The app's tab bar is replaced by a golf ball that opens an arc of greens.

**Where the ideas come from** (C22): the ladder is an adaptive staircase in the spirit of the challenge-point
framework (Guadagnoli and Lee, 2004) as Adam Young applies it to practice. Separating a pattern's centre from
its size, and learning the centre by feeling both edges (differential practice), are also his emphasis. The
code and the app's words are this repo's own.

## Exit

All of these hold:
1. **The reader's error is measured** on P3's truth set, per label type, as P50 and P95 in millimetres, and
   the real-photo gate (P13) passes with **no wrong reading**: every strike is read, sent to a pick, or
   refused by name.
2. **On the iPhone (P29)**: one club has a baseline and at least two judged ladder blocks, and a club of
   another category has a baseline. Every strike is read, picked, declared off the tape, or named not read,
   and the progress view shows the dots, the level, the pattern and the heat map.
3. **The export round-trips**: the lab's `import-export` keeps every strike, the state and the crops under
   `data/processed/drills/`, and a second import of the same zip changes nothing.
4. **The navigation is the ball**: no `NavigationBar` remains, every page is reachable through a green, and
   `flutter test` plus the integration tests pass on it.
5. `cargo test --workspace`, clippy, `fmt`, `flutter analyze` and the doc-truth test are clean (P31).

---

## Rules every phase keeps

1. **Stop after every phase.** One phase, one session, one gate.
2. **Each phase commits its own work and never pushes.** One commit on `main` at the phase's end, with
   `git add` of the phase's own paths only, never `-A` or `.`, because M38 may hold uncommitted work on the
   Mac. The message names the phase (`M46 P7: …`) and ends with the attribution lines. **No phase runs
   `git checkout`, `restore`, `clean`, `stash` or `reset`**, and none deletes a file it did not create.
3. **Python gains nothing, and frozen Python is not touched** (ADR-035 clause 4). New work is Rust, plus
   Dart and Swift under `app/`.
4. **Version keys and `spec/`.** `ANALYSIS_VERSION`, `SCREEN_PARSER_VERSION` and `CAREER_VERSION` stay, no
   committed vector moves, and `devices.json` and `profiles.json` stay. **The one new key is
   `TAPE_READER_VERSION`** (P5). Every stored strike carries it, and a phase that changes what the tape reader
   produces after P13 bumps it in the same change. If a phase cannot pass without moving one of the others, it
   reports `BLOCKED`.
5. **Drills are untracked** (D6, ADR-034 §3). Nothing in M46 writes into `sessions/`, `shots/`, the corpus,
   the baselines or the graded profile, and nothing reads a drill strike into them. `CENTERED_RADIUS_MM`
   (`crates/contracts/src/strike.rs`) does not move. Keeping every strike is what lets a later milestone
   decide otherwise.
6. **No score beats a wrong one** (ADR-010 §2). A mark the reader cannot place is *not read*, never the
   centre. A guessed side is a guessed sign, so orientation refuses rather than guesses. A strike the user
   places by hand is a **declaration**, and is stored as one (`source: picked` or `declared_off_tape`). On the
   truth set, **a wrong reading fails the gate** (C24).
7. **The reader is std only.** `impact::tape` lives in `crates/impact`, which takes no dependency (no
   `image`, no `ort`), so `crates/ocr/tests/ort_reach.rs` keeps holding. Photos are decoded at the edges:
   Swift on the phone (C16), `golf-lab`'s decoder on the desk (P8).
8. **M38's boundary rules hold for every app phase**: its rule 7 (`api/` types, no enum with fields, no
   `pub` helper in `api`), rule 8 (**Dart holds layout and no golf**: Rust hands every display string, and
   `None` shows as not read, never 0), rule 9 (Dart passes text through unparsed), and rule 10 (no network,
   no runtime plugin, exact versions, never `flutter pub upgrade`).
9. **`data/` is written only in two places**: P3's photos under gitignored `data/raw/strike_tape/`, and
   P29's lab import under `data/processed/drills/`. Tests run on scratch directories.
10. **Real photos are gitignored, and the gates on them are ignored by default.** That is the Full Swing
    gates' precedent (`docs/CONFORMANCE.md`, "The Full Swing gates"): each photo is held to its sha256 in the
    test, the gate runs with `--include-ignored`, and without the photos it fails rather than passing
    vacuously.
11. **What stops a phase.** The decisions are final, so a runner does not re-open them. It reports
    `BLOCKED` if something in the plan is wrong and a decision or rule 4 stands in the way.
12. **House style.** Comments and docs say why, a test's name says what it pins, and a new doc declares its
    tier on its first lines. Provisional constants say whose they are and what will revise them. Pipe build
    and test output through `tail` or `grep`.
13. **The cheap gate is the default**: `cargo test -p <crate>` for what a phase touched. The whole workspace
    runs only in P21, P28 and P31, and the Simulator only in phases that name it.

## Verify — the commands, and when each runs

On Windows the Python is `.venv/Scripts/python.exe`. On the Mac it is `.venv/bin/python`, and every command
follows `. app/tool/env.sh` in the same Bash call (the M38 plan's Verify section).

| Command | When |
|---|---|
| `cargo test -p <crate> 2>&1 \| tail -15` | every phase touching that crate |
| `cargo clippy -p <crate> --all-targets -- -D warnings 2>&1 \| tail -15` | same, for `impact`, `app` and any crate this plan creates a module in |
| `cargo fmt --check` | every phase touching Rust |
| `cargo test -p ocr --test ort_reach 2>&1 \| tail -3` | P7, P18, P31 |
| `cargo test --release -p golf-lab --test strike_tape -- --include-ignored 2>&1 \| tail -20` | P13 onward, every phase touching `impact::tape` |
| `cargo build -p app --target aarch64-apple-ios 2>&1 \| tail -3` | 🍎, the first Mac phase after P18 adds `jpeg-encoder` |
| `<python> -m pytest tests/test_docs_truth.py 2>&1 \| tail -3` | every phase touching a doc or the workspace's members |
| `cargo test --workspace --no-fail-fast 2>&1 \| grep -E '^test result\|FAILED\|failed' \| tail -40` | P21, P28, P31 |
| `(cd app && flutter analyze 2>&1 \| tail -5 && dart format --output=none --set-exit-if-changed lib test integration_test \| tail -3)` | every phase touching Dart |
| `(cd app && flutter test 2>&1 \| tail -5)` | every phase touching Dart |
| `app/tool/sim_test.sh integration_test/<file>.dart 2>&1 \| tail -20` | the phases naming it |
| `app/tool/codegen_check.sh` | the first 🍎 phase after any change to `crates/app/src/api/`, run before its own work |

**The codegen runs only on the Mac.** P18 to P20 change `crates/app/src/api/` on the desk and are tested
there by `cargo test -p app`. The bindings are regenerated at P22, which runs `codegen_check.sh` first.

---

## Decisions taken in the interview (2026-11-12)

- **D1 — STRIKEPRO labels only, read as that exact product.** The user always uses Strike Tape LLC's
  STRIKEPRO labels ([strike-tape.com](https://strike-tape.com/)). Packs hold driver, iron and hybrid labels, at
  about 6 to 10 hits per label. The site gives no dimensions and says nothing of how a strike shows, so P3
  measures them and P4 commits them as data. Another tape later is a new label type or a new method, never a
  change to this one.
- **D2 — Face centre is the label's centre, on every club.** The user's call, made knowing that application
  error then lands in every reading unmeasured: *"let's not worry about that for now"*. A reading is the mark's
  position in the label's own frame.
- **D3 — Every club.** Each category takes one of the three label types. Which label goes on fairway woods
  and on the putter (if any) is the user's answer at P3 (the run sheet asks), and P4 records it.
- **D4 — One photo per strike, newest mark.** There is no clean photo of a fresh label. One mark on the label
  means a new label. More than one means diff against the last strike's crop. If the reader cannot tell, the
  user taps the new mark, which is the last resort (*"a doomsday task where we have utterly failed"*).
- **D5 — Drills only, and everything kept.** Strikes never feed the corpus or M37's strike-location grade in
  M46 (rule 5), and every strike's data is kept so a later milestone can decide.
- **D6 — Images are kept as a small rectified crop.** The label is cut out, squared to its own frame and
  stored as a compressed JPEG of tens of KB, and the original photo is deleted once the strike is stored.
- **D7 — The truth is a printed mm grid.** P2 prints a grid sheet. In P3, peeled labels are stuck onto it in
  order and photographed straight on, and that photo is the truth the on-club readings are compared to.
- **D8 — Baseline, then the ladder.** A 10-strike baseline sets the start level. Blocks of 10 follow: **7 or
  more inside moves in a level, 3 or fewer moves out, and anything else stays** (the user's numbers).
- **D9 — The levels are a table per category** (C9 has the provisional radii, for the user to confirm at
  approval).
- **D10 — Level and stage are per club, and the category is a roll-up.** The category view gathers its clubs'
  strikes and lists each club's level. Per club is what matters most.
- **D11 — Recentring and differential practice are suggestions, never the default.** Triggers fire from the
  ladder's own results (C13). The golfer accepts or declines, and an accepted stage is remembered for the club.
- **D12 — Differential calls are random**: heel, centre or toe, uniform per strike, as M38's challenge draws
  shapes. They're judged against the club's current circle (C14).
- **D13 — Progress shows dots on the label and the level, and a heat map beside them.** The ROADMAP had the
  heat map as "later", and the user moved it into M46.
- **D14 — A Drills page**, where M44's low point will join Impact later.
- **D15 — The golf-ball navigation replaces the tabs.** A ball sits at the bottom centre of every page.
  Tapping it fans an arc of five greens, each with a flag naming its page. Tapping a green rolls the ball into
  its cup and opens the page. It ships after the Drills page, inside M46.
- **D16 — Interleave freely with M38.** The bay is set aside for a while.

## Calls this plan makes

Each was taken as the option with the least to go wrong, and is approved with the plan.

**C1 — The reader is a second method in `crates/impact`**, module `tape`, beside M42's overhead replay, and
reuses that crate's `frame` (`Frame`, luminance, clipped rectangles), `mask` and `component` (4-connected
components with centroid, box and principal axis). It does **not** reuse `crates/ocr`'s `quad` or `rectify`:
those are OpenCV-faithful ports on `image::RgbImage` inside the one crate that may reach `ort`, so depending
on them is forbidden (`ort_reach.rs`). Moving them out would be a structural refactor nothing here needs.
`impact::tape` writes its own 4-point homography (a direct 8×8 solve) and a bilinear warp, about 150 lines
with no oracle to match. P7 adds a `docs/REFACTOR_LEDGER.md` row recording the near-duplicate and why it
stays.

**C2 — The interface: `contracts::face_impact`.** One answer to *where on the face*, one implementation per
method:
- `ImpactMethod`: `OverheadReplay` (M42's `crates/impact`) and `StrikeTape`. A method later is a new variant,
  and a `match` makes it a compile error until its axes are declared.
- `ImpactAxes { heel_toe: bool, high_low: bool }`, from `ImpactMethod::axes()`: replay `{true, false}`, tape
  `{true, true}`.
- `FaceImpact { offset_h_mm: Option<f64>, offset_v_mm: Option<f64>, method, method_version: i64 }`, in
  `ShotData`'s units (+ toe, + high). `Validate` refuses a value on an axis the method cannot see.
- `TAPE_READER_VERSION: i64 = 1`.

`ShotData` does not change, and nothing converts a `FaceImpact` into a shot in M46 (rule 5).

**C3 — The label geometry is data**: `crates/impact/tape_labels.json`, read by `include_str!` the way
`contracts::catalogue` reads its rows. Per label type it holds `id` (`strikepro_driver`, `strikepro_iron`,
`strikepro_hybrid`), the outline (`width_mm`, `height_mm`, corner radius, or a polygon if the shape is not a
rounded rectangle), the centre in label millimetres, and `inscribed_radius_mm`. Beside it, a
`category_labels` map (`ClubCategory` → label id, or absent for a category no label fits). Every number has its
provenance: "measured by the user at P3 with <tool>".

**C4 — Coordinates.** The label frame is millimetres with the origin at the label's centre, **+x toward the
toe and +y toward the top line**, whichever hand the club is. The rectified crop is `CROP_PX_PER_MM`
pixels per millimetre (provisional 10, set at P7), with the label filling it. **Orientation comes from the
club**: the hosel and shaft leave the face at the heel and run up (P10). The golfer's `Handedness` is a
cross-check, not a source. For a right-handed club seen face-on with the shaft up, the toe is on the image's
left. A disagreement, or no shaft found, refuses as `no_orientation`.

**C5 — The reader's outcomes** (`impact::tape::read_tape(photo: &Frame, previous: Option<&Frame>, label:
&LabelType)`):
- **`Read`**: the label's corners in the photo, the crop, the orientation, `new_label: bool`, and **one or
  two** new marks, each with its centroid in crop pixels and its offset in label millimetres. Two new marks
  means a photo was missed (C10).
- **`NeedsPick`**: the label was found and squared, but the new mark cannot be told apart (overlap, or more
  than two candidates). It carries the crop, and the user taps (C6).
- **`NotRead(reason)`**: `no_label`, `no_orientation`, `no_new_mark`, `mark_clipped` (it touches the label's
  edge), or `decode_failed`. Each reason has its sentence and its `retake_helps` flag, as `UnscoredReason`'s
  `refilming_helps` works.

**C6 — The user's two declarations.** **A pick**: the user taps the new mark on the crop, Rust converts the
tap to millimetres, and the strike is stored with `source: picked`. **"Missed the tape"**: offered on
`no_new_mark`, stored with `source: declared_off_tape` and no offsets. It counts as outside every circle,
which holds only because no level's radius exceeds the label's inscribed radius (C9).

**C7 — The drill history on disk**, beside the corpus and never inside it, under the data root:
```
drills/<player_id>/impact/strike_tape/
  strikes.jsonl          one TapeStrike per line, append-only
  state.json             per club: stage, level, the block in progress, a pending suggestion, level history
  crops/<strike_id>.jpg  the rectified label crop (D6)
  last/<club>.rgb        the club's last crop as raw RGB, the diff's reference (C16)
```
Writes go through `storage`'s `write_atomically` and the same lock the shot store holds. The path's shape,
`drills/<player>/<topic>/<method>/`, is what M44's `low_point/foot_powder/` will sit beside.

**C8 — `TapeStrike`** (`contracts::drill`): `strike_id` (`<install>-<N>`, N from the state's counter, so two
phones cannot collide), `player_id`, `club`, `category`, `label_type`, `label_id` (one per label applied,
`<install>-L<N>`), `n_on_label`, `captured_at`, `stage`, `level_radius_mm` at the time, `call` (differential
only), `offset_h_mm` and `offset_v_mm` (`Option`), `source` (`read` / `picked` / `declared_off_tape`),
`not_read` (a reason, when nothing was stored as a position), `reader_version`, and `crop`. **Inside or not is
derived, never stored**, for `PracticeMode::is_tracked`'s reason: a stored verdict could disagree with its
own numbers.

**C9 — The ladder tables, per category: the user's, provisional.** Target radii in mm, widest first:

| Category | Radii (mm) |
|---|---|
| `driver` | 30, 26, 22, 19, 16, 14, 12, 10 |
| `wood` | 28, 24, 21, 18, 15, 13, 11, 9 |
| `hybrid` | 26, 22, 19, 16, 14, 12, 10, 8 |
| `long_iron` | 24, 21, 18, 15, 13, 11, 9, 8, 7 |
| `mid_iron` | 22, 19, 16, 14, 12, 10, 9, 8, 7, 6 |
| `short_iron` | 20, 17, 15, 13, 11, 10, 9, 8, 7, 6, 5 |
| `wedge` | 18, 16, 14, 12, 10, 9, 8, 7, 6, 5 |
| `putter` | 15, 12, 10, 8, 7, 6, 5, 4 (only if P3 maps a label to it) |

Each table has two cuts applied:
- **trimmed at P4**: a radius above the category's label `inscribed_radius_mm` is removed, so a strike off the
  tape is outside every circle (C6);
- **floored at P13**: a radius below `2 × P95` of the label type's measured error is removed, so a level is
  never tighter than the reader can resolve (ADR-010 §2).

The cut tables are what ship. The uncut ones stay in the doc comment, with both cuts named.

**C10 — Blocks.** A block is **10 counted strikes**. A strike counts when it is `read`, `picked` or
`declared_off_tape`, and a `NotRead` strike is stored but not counted. A photo with two new marks counts both,
each judged in its own right, and in differential practice both answered the same call (the next call is drawn
only after a photo). Changing stage mid-block abandons the block: its strikes stay stored, and the block is
never judged. Inside means `hypot(h, v) ≤ radius`, as `CENTERED_RADIUS_MM`'s rule is.

**C11 — The baseline.** 10 counted strikes, aiming at centre. The start level is the **smallest radius in the
club's table holding at least 5 of the 10**. If even the widest holds fewer than 5, the baseline's own trigger
fires (C13). The baseline's R50 is shown either way.

**C12 — The pattern** (`analysis::drill`), over a club's **last 20 positioned strikes** in the baseline,
ladder and recentre stages. Differential strikes are aimed off centre on purpose and never enter it:
- **centre**: the mean `h` and `v`, each with its 95% interval through `analysis::stats::mean_ci`;
- **R50**: the median distance from face centre, through `stats::percentile`;
- **spread**: the median distance from the pattern's own centre.

Below `RATE_MINIMUM_N` (10) positioned strikes, the pattern is absent, never estimated (rule 6).

**C13 — The triggers: the user's, provisional.**
- **Recentre** is suggested when the pattern's centre is clearly off: on some axis the 95% interval excludes
  0 **and** `|mean|` exceeds a third of the current radius. In the recentre stage a block's goal is its own
  mean within a third of the radius. Meeting it suggests returning to the ladder at the same level.
- **Differential** is suggested after **two blocks in a row at 3 of 10 or fewer at the widest level**, or a
  baseline whose widest circle holds fewer than 5. A differential block of **7 or more calls made** suggests a
  fresh baseline, and the ladder then starts from it.
- **A declined suggestion** is offered again after each block for which it still holds. "Not now" is never
  remembered as "never".

**C14 — Differential judging**, against the club's current radius `R` (its last ladder level, or its
baseline's start level): a **centre** call is made when `hypot(h, v) ≤ R`, a **heel** call when `h ≤ −R`, a
**toe** call when `h ≥ +R`. A `declared_off_tape` strike makes no call. High and low are not called in M46.

**C15 — The facade: `crates/app/src/api/drill.rs`**, with its logic in a non-`api` `crates/app/src/drill.rs`
(rule 8):

| Function | Does |
|---|---|
| `drill_clubs(root, player_id) -> Vec<DrillClubView>` | each bag club with its category, label type, stage and level line |
| `start_drill(root, player_id, club, now) -> DrillView` | opens or resumes the club's drill. A club with no baseline starts one |
| `record_tape_photo(TapeIn) -> TapeOutcome` | `root, player_id, club, raw_path, width, height, captured_at, now, call`. Reads, stores and judges |
| `pick_mark(root, pending_id, x, y, now) -> TapeOutcome` | the user's tap on a `NeedsPick` crop (C6) |
| `declare_off_tape(root, pending_id, now) -> TapeOutcome` | the "missed the tape" declaration (C6) |
| `answer_suggestion(root, player_id, club, accept, now) -> DrillView` | accept or decline the pending suggestion |
| `set_stage(root, player_id, club, stage, now) -> DrillView` | the golfer's own stage change, which abandons a block |
| `drill_progress(root, player_id, club) -> ProgressView` | P20 |
| `category_progress(root, player_id, category) -> ProgressView` | P20 |

A `NeedsPick` or `no_new_mark` outcome is held as a **pending** strike (`pending_id`), with its crop, until it
is picked, declared or discarded. A pending strike is not counted.

**C16 — Pixels at the boundary.** Swift decodes the photo the way `ScreenReader.swift` does (`CIImage` with
the orientation applied, sRGB, RGBA8) and writes the bytes to a temporary file under `.app/tmp/`, passing its
path, width and height. Rust reads and deletes it. **Dart never holds pixels.** The crop is JPEG-encoded in
Rust by `jpeg-encoder` (exact pin, pure Rust, no dependencies), the one dependency M46 adds, in `crates/app`
only. The diff's reference is the club's last crop as raw RGB (`last/<club>.rgb`), because a JPEG's
artefacts would show up as marks in a diff, and because nothing in Rust here decodes a JPEG. The original
photo is deleted once its strike, or its pending strike, is stored. A photo with `no_label` goes to
`.app/failed/` as M38's C13 does, since those are the photos that improve the reader.

**C17 — Export and the lab.** M38's C16 list gains `drills/`, so `export_sessions` zips it. `golf-lab
import-export` copies `drills/` into `data/processed/drills/`: strikes merge by `strike_id` (a held id is
kept, as the M29 plan's dedupe rule keeps a held shot), `state.json` takes the newer `updated_at`, and a crop is
kept if absent. A second import of the same zip changes nothing.

**C18 — The navigation** (`app/lib/src/ui/golf_nav.dart`). `Shell` keeps its `IndexedStack`, so each page
survives, and loses its `NavigationBar`. Its parts:
- **the pages**, in order: Record, Drills, History, Profile, Settings. Record opens first (M38's D13);
- **the ball**: a `Semantics(button, label: "Navigate")` at the bottom centre, above the safe area;
- **the arc**: tapping the ball fans five greens in an arc above it. Each is a `Semantics(button, label:
  <page>)` with a flag naming it, and the current page's flag is marked. Tapping outside closes the arc;
- **the roll**: tapping a green rolls the ball into its cup (about 450 ms), then switches the page;
- **reduced motion**: with `MediaQuery.disableAnimations` (iOS Reduce Motion) there is no roll, and the page
  switches at once.

Tests reach a page by its green's semantics label, through one helper in `app/test/fakes.dart`.

**C19 — Differential calls are drawn in Dart**, uniformly over heel, centre and toe with an injected `Random`,
exactly as M38's challenge draws shapes (its C15 and P23 finding 3). The call passes to Rust with the photo,
and Rust judges and stores it. The call changes only after a counted strike, so a `NotRead` keeps it.

**C20 — Rust hands every view its strings and geometry** (rule 8): an offset line ("8.4 mm toe, 2.1 mm
high"), a verdict ("Inside 12 mm"), the block line ("Block: 6 of 10, 4 inside"), and the label outline,
circle, dots and pattern as **normalised label coordinates** (0 to 1 across the label), which Dart only
scales and paints.

**C21 — The heat map** is a density over the label: 2 mm cells, a Gaussian kernel of σ = 3 mm (provisional),
normalised so the densest cell is 1. Rust computes it and Dart maps each value to a colour. It is shown at
`RATE_MINIMUM_N` positioned strikes or more, and below that the view says how many it needs.

**C22 — Credit.** ADR-037 and ARCHITECTURE credit the ladder's idea to the challenge-point framework and
Adam Young's practice writing, and the pattern and differential ideas to Adam Young. No text, drill name or
table is copied from his materials. The app's words are plain ("Differential practice: hit the spot I call").

**C23 — The desk tool, `golf-lab read-tape <photos…> [--sequence] [--label T] [--review DIR]`.** It decodes
through `ocr::decode::load_image` (HEIC via ffmpeg, as the OCR gate does) into a `Frame`, runs `read_tape`,
and prints one JSON line per photo. With `--sequence`, each photo's previous frame is the one before it.
`--review` writes PNGs of the found label, the crop with each mark outlined, and the orientation arrow, in
`read_impact.rs`'s pattern. It stores nothing.

**C24 — The gate's "wrong".** On the truth set, a reading is **wrong** when it is more than `WRONG_MM`
(provisional 5) from the truth on either axis. **One wrong reading fails the gate.** Coverage (read, picked
and refused) is printed, and a `READ_AT_LEAST` floor is set at what P13 measured, so a reader that starts
withholding is seen (the clip gate's precedent).

### Not in M46

Feeding strikes into the corpus, the profile or M37's strike-location grade, and revising
`CENTERED_RADIUS_MM` (rule 5): a later milestone, since every strike is kept. Any tape but STRIKEPRO, and the
other methods (foot spray, dry-erase marker, a launch monitor's printed offsets). Application error (D2). A
driver target off face centre (high toe), since every target is the label's centre. High and low calls in
differential practice. Re-reading stored crops when `TAPE_READER_VERSION` moves, which the crops make cheap
later. M44's low point. The heat map at the lab. Improving M42's replay reader. Android.

---

## Phases

### P0 — 🪟 This plan, the ROADMAP pointer, and the plan's commit

**Goal**: this file in `docs/plans/m46-impact-drill.md`, with the ROADMAP pointing at it.
**Read**: this plan; `ROADMAP.md`'s status table row for M46 and its §M46 (lines near "## M46").
**Touch**: `docs/plans/m46-impact-drill.md`, `ROADMAP.md`.
**Do**: write the plan. Set M46's status row to 🔄 *Planned 2026-11-12, 0/32 phases*, linking the plan. In
§M46, add a **Planned** line naming the plan and saying the plan's decisions answer the four questions the
section raised: tracked (no, D5), millimetres (label centre and label scale, D2 and C3), one mark or several
(newest mark, D4), and not read (rule 6). Leave the rest of §M46 as the record of the ask. Commit `M46 P0`.
**Done when**: the doc-truth test passes and the commit holds only those two files.
**Verify**: `<python> -m pytest tests/test_docs_truth.py 2>&1 | tail -3`.

### P1 — 🪟 ADR-037: where on the face, by method, and the drill framework

**Goal**: the pattern ROADMAP §M46 asked P0 to write down as an ADR, so M44 and M45 inherit it.
**Read**: this plan's D and C sections; ADR-034 §3 and its M37 and M42 addenda (grep `## Addendum`); ADR-010
§2; `docs/decisions/036-llm-evaluation-and-observability.md`'s first 30 lines for the house ADR shape.
**Touch**: `docs/decisions/037-impact-location-by-method-and-drills.md`, `docs/README.md` (the map's row).
**Do**: write the ADR. Status Accepted, 2026-11-12. It covers:
- **Context**: `impact_offset_*` empty on real data, and drills untracked.
- **Decision**: one question, many methods (C2); the method declares its axes; the drill framework (drill →
  topic → method, C7's layout); drills untracked and kept (D5); a declaration is not a reading (C6); the
  ladder (D8, C9 to C11), the triggers (C13), and differential judging (C14), with the credit (C22).
- **Consequences**: M44's place in the framework, and what a later milestone needs to feed the grade.
- **Rejected alternatives**: groove-based centre, one photo per strip, tip only. Each stays named with why.
**Done when**: the doc-truth test passes and `docs/README.md` lists the ADR with its tier.
**Verify**: doc-truth.

### P2 — 🪟 The truth sheet and the tape session's run sheet

**Goal**: everything the user needs printed and read before P3, so the session happens once.
**Read**: this plan's D1, D3, D4, D7 and C3; `docs/plans/m38-iphone-app.md`'s P31 run sheet, for the house
style of one.
**Touch**: `spikes/strike-tape/truth-sheet.svg`, `spikes/strike-tape/README.md`,
`docs/plans/m46-tape-run-sheet.md`.
**Do**: the sheet and the run sheet.
- **The truth sheet**: an A4/Letter-safe SVG in millimetre units with a 1 mm grid (a 5 mm and a 10 mm grid
  heavier), built from SVG `<pattern>`. It has four solid black corner squares 10 mm on a side at known
  positions (the fiducials P13 reads), six numbered 70 × 60 mm slots, and a 100 mm scale bar to check that the
  print came out at 100%.
- **The run sheet**: a numbered checklist.
  1. Print at 100% and check the scale bar with a ruler (±0.5 mm, or reprint).
  2. Measure each label type with calipers or a ruler (width, height, corner radius, and whether it has any
     printing), and photograph one of each flat on the grid.
  3. Answer which label goes on fairway woods and on the putter, or "none".
  4. Per club, for driver, a wood or hybrid, one long or mid iron and a wedge at least: apply a label and hit
     6 to 10 strikes, photographing the label on the club after **every** strike with the iPhone camera.
     Fill the frame with the head, keep the hosel in view, and hold the shaft up.
  5. Twice, deliberately skip a photo, so two marks are new, and note which.
  6. Shoot a few photos at 20 to 30° off square.
  7. Peel each finished label and stick it in the next slot, in order, writing the slot's club and label number
     beside it. Photograph each full sheet straight on.
  8. Copy every photo, untouched (HEIC is fine), to `data/raw/strike_tape/<date>/` on the desk, keeping the
     file names.
  9. Write down the club, handedness and photo range of each label in `labels.txt` beside them.
**Done when**: the SVG renders at the right size (the scale bar is 100 mm in its own units, checked by
reading the file) and the run sheet names every item P4 and P13 need.
**Verify**: doc-truth.

### P3 — 🧍 The tape session: labels measured, strikes photographed, truth sheets photographed

**Precondition (yours)**: STRIKEPRO labels of all three types, a ruler or calipers, a printer, the clubs
named in the run sheet, somewhere to hit (no launch monitor needed), and the iPhone.
**What you do**: the run sheet, `docs/plans/m46-tape-run-sheet.md`, top to bottom.
**What the session does**: lists `data/raw/strike_tape/<date>/`, checks every item the run sheet asked for is
there (photo counts per label, the skipped-photo notes, the truth sheet photos, the measurements, the two
answers), and asks for anything missing. It writes nothing but this phase's finding.
**Done when**: the finding lists what arrived, per club and label, and nothing the later phases need is
missing.

### P4 — 🪟 The session's inventory and the label geometry as data

**Goal**: P3's session turned into the two things every later desk phase reads.
**Read**: P3's finding; C3, C4 and C9; `crates/contracts/src/catalogue.rs` for the `include_str!` pattern.
**Touch**: `crates/impact/tape_labels.json`, `crates/impact/src/tape/mod.rs` (new, the loader only),
`crates/impact/src/lib.rs` (the module line and the crate doc's new method paragraph),
`crates/impact/tests/tape_labels.rs`, `data/raw/strike_tape/<date>/inventory.json` (gitignored).
**Do**:
- **The inventory**: one entry per photo, holding its sha256, club, category, handedness, label number, its
  index on the label, and whether a photo before it was skipped.
- **`tape_labels.json`**: C3's fields, from the user's measurements, cross-checked against the flat photos on
  the grid (each dimension within 1 mm, or the finding says which number won and why), plus the
  `category_labels` map from P3's two answers.
- **The loader**: `impact::tape::LabelType` with a `std`-only parse of exactly this file's shape. `impact`
  takes no `serde`, so this is a hand parser of a fixed, small format, or the file is a Rust table
  (`tape_labels.rs`) instead of JSON if the hand parser would pass 80 lines, with the finding saying which.
- **The tables**: compute each category's C9 table **trimmed** at its label's inscribed radius, and record the
  trimmed tables in the finding (P6 commits them).
**Done when**: a test loads every label type and holds each to the measured numbers. Every `ClubCategory`
maps to a label or is named as having none.
**Verify**: `cargo test -p impact`, clippy, fmt.

### P5 — 🪟 `contracts::face_impact`: the method interface and `TAPE_READER_VERSION`

**Goal**: C2.
**Read**: C2; `crates/contracts/src/shot.rs` around `impact_offset_h` (for units and doc style);
`crates/contracts/src/capability.rs`'s line about `impact_offset_h`; `crates/contracts/src/lib.rs`'s
`validated!` and `Validate`.
**Touch**: `crates/contracts/src/face_impact.rs`, `crates/contracts/src/lib.rs`,
`crates/contracts/tests/face_impact.rs`, `crates/impact/src/lib.rs` (a doc line naming
`ImpactMethod::OverheadReplay` as this reader, and no code).
**Do**: C2's types with serde (snake_case), `Validate` refusing an axis the method cannot see, and a
non-finite value. `ImpactMethod::axes()` is a `match` with an arm per variant. Add `TAPE_READER_VERSION = 1`
with a doc saying what moves it (rule 4).
**Done when**: these tests pass:
- a replay `FaceImpact` holding a `v` is refused;
- a tape one with both axes passes;
- a NaN is refused;
- the wire names are pinned.
**Verify**: `cargo test -p contracts`, clippy, fmt.

### P6 — 🪟 `contracts::drill`: the strike record, the stages and the ladder tables

**Goal**: the drill's shapes, so the store, the analysis and the facade share one spelling.
**Read**: C7 to C14; P4's finding (the trimmed tables); `crates/contracts/src/intent.rs` (the `is_tracked`
doc, for "derived, never stored"); `crates/contracts/src/strike.rs`'s constants block for the provenance style.
**Touch**: `crates/contracts/src/drill.rs`, `crates/contracts/src/lib.rs`, `crates/contracts/tests/drill.rs`.
**Do**: the shapes, the constants and the tables.
- **Shapes**: `DrillTopic { Impact }`, a one-variant enum with a doc naming M44's `LowPoint` as the next
  variant. `TapeMethod { StrikeTape }`. `DrillStage { Baseline, Ladder, Recentre, Differential }`.
  `StrikeCall { Heel, Centre, Toe }`. `StrikeSource { Read, Picked, DeclaredOffTape }`. `TapeNotRead`
  (C5's reasons, each with `sentence()` and `retake_helps()`). `TapeStrike` (C8). `Suggestion { Recentre,
  Differential, ReturnToLadder, FreshBaseline }`. `ClubDrillState` and `DrillState` (C7's `state.json`:
  stage, level index, block strike ids, pending suggestion, level history entries `{at, from, to, block}`, the
  strike and label counters, `updated_at`).
- **Constants**: `BLOCK_SIZE = 10`, `MOVE_IN_AT = 7`, `MOVE_OUT_AT = 3`, `BASELINE_HOLDS = 5`,
  `PATTERN_WINDOW = 20`, `RECENTRE_FRACTION = 1/3`, and `ladder_radii(category) -> &'static [f64]`, the
  trimmed tables. Each says "the user's, provisional, M46 interview" and what revises it. Leave a
  `// floored at P13` note where P13 will apply the floor.
- **`Validate`**: `TapeStrike` refuses offsets with `declared_off_tape`, no offsets with `read` or `picked`,
  a `call` outside the differential stage, and `n_on_label < 1`.
**Done when**: a round-trip test pins every wire name, and each `Validate` refusal has a test. Every table is
strictly decreasing and at or below its label's inscribed radius (the test reads `impact`'s data through a
dev-dependency).
**Verify**: `cargo test -p contracts`, clippy, fmt.

### P7 — 🪟 `impact::tape` geometry: the homography and the rectified crop

**Goal**: four corners in a photo become a square crop in label millimetres, and back.
**Read**: C1, C4; `crates/impact/src/frame.rs` (`Frame`, the coordinate convention in `lib.rs`'s doc).
**Touch**: `crates/impact/src/tape/geometry.rs`, `crates/impact/src/tape/mod.rs`,
`crates/impact/tests/tape_geometry.rs`, `docs/REFACTOR_LEDGER.md` (C1's row).
**Do**: build the geometry with no oracle to match.
- `Homography::from_corners(photo: [[f64;2];4], label: [[f64;2];4])` by a direct 8×8 solve (Gaussian
  elimination with partial pivoting), refusing a degenerate quad.
- `apply` and `inverse`.
- `rectify(frame, &h, label, px_per_mm) -> Frame`: bilinear sampling, with outside pixels marked.
- `CROP_PX_PER_MM`, set here with a reason: the mark's size in pixels against the crop's bytes.
**Done when**: a synthetic label drawn under a known perspective (up to 30°, rotated, scaled) has a dot at a
known label position recovered within 0.1 mm, `inverse ∘ apply` holds within 1e-9, and a collinear quad is
refused.
**Verify**: `cargo test -p impact`, clippy, fmt, `ort_reach`.

### P8 — 🪟 `golf-lab read-tape`: the desk tool and its review images

**Goal**: C23, so every reader phase after it is tuned and checked on real photos.
**Read**: `crates/lab/src/read_impact.rs` (the verb's shape, its review images, its JSON lines);
`crates/lab/src/bin/golf_lab.rs` around `"read-impact"`; `crates/ocr/src/decode.rs`'s `load_image`.
**Touch**: `crates/lab/src/read_tape.rs`, `crates/lab/src/lib.rs`, `crates/lab/src/bin/golf_lab.rs`,
`crates/lab/tests/read_tape.rs`.
**Do**: the verb over whatever `impact::tape` exposes. Until P9, that is `read_tape` returning
`NotRead(no_label)`, a stub P9 replaces. Flags are as C23. The output is one JSON line per photo with the
outcome's every field, and the review images go under `DIR/<stem>/`.
**Done when**: a test runs the verb on two synthetic PNGs (drawn by the test) and gets two JSON lines and
the review files, and an unknown flag and a missing file are each refused with their sentences.
**Verify**: `cargo test -p golf-lab --test read_tape`, clippy, fmt.

### P9 — 🪟 The label finder, on P3's photos

**Goal**: a STRIKEPRO label's four corners found in a photo of it on the club, or `no_label`.
**Read**: P3's and P4's findings (what the label looks like against each face); C3; `crates/impact/src/mask.rs`
and `component.rs` (what to reuse); `crates/ocr/src/quad.rs`'s doc (the screen-finding idea, not its code).
**Touch**: `crates/impact/src/tape/label.rs`, `crates/impact/src/tape/mod.rs`,
`crates/impact/tests/tape_label.rs`.
**Do**: segment the label by its colour and luminance against the head. Take the largest component whose
shape fits the label type's outline. Fit the four corners (a rounded rectangle's corners come from its four
edge lines intersected, not from the rounded pixels). Accept only when the squared crop's aspect ratio is
within `ASPECT_TOLERANCE` of the label's. Tunables carry their brackets, as `crates/impact/src/lib.rs`'s block
does. Run `golf-lab read-tape --review` over every P3 photo.
**Done when**: synthetic tests pin a found label and a refused non-label. Over P3's photos the finding
tables found against `no_label`, per label type, with every refusal's review image looked at and its cause
named.
**Verify**: `cargo test -p impact`, the verb over P3's photos, clippy, fmt.

### P10 — 🪟 Orientation: heel and up, from the hosel

**Goal**: C4's orientation, so a reading's signs are the club's and not the photo's.
**Read**: C4; P9's finding; `crates/impact/src/shaft.rs` (M42's dark-bar-from-one-end stage, to reuse or
generalise); `crates/contracts/src/golfer.rs` (`Handedness`).
**Touch**: `crates/impact/src/tape/orient.rs`, `crates/impact/src/tape/mod.rs`,
`crates/impact/tests/tape_orient.rs`.
**Do**: around the found label, find the hosel and shaft. The shaft is a dark bar leaving the head beyond the
label at one end, and its side is the heel and its direction up. Map the crop's axes to C4's frame.
Cross-check against the inventory's handedness, and refuse `no_orientation` on no shaft or a disagreement.
**Done when**: synthetic tests pin both hands and a missing shaft. Over P3's photos the finding tables the
outcomes, and **no photo is oriented wrongly** (each checked against its review image).
**Verify**: `cargo test -p impact`, the verb, clippy, fmt.

### P11 — 🪟 The mark on a fresh label

**Goal**: the first strike on a label: one mark, its centroid, and its offset in millimetres.
**Read**: P3's finding (how a STRIKEPRO strike looks: darker, lighter, or coating gone); P9 and P10's
findings; C5.
**Touch**: `crates/impact/src/tape/mark.rs`, `crates/impact/src/tape/mod.rs`,
`crates/impact/tests/tape_mark.rs`.
**Do**: in the crop, segment the mark against the label's own clean colour, estimated from the crop's
border band. The mark is the component of at least `MARK_MIN_AREA_MM2` and its centroid weighted by
contrast. A mark touching the label's edge is `mark_clipped`, and a crop with none is `no_new_mark`.
`read_tape` with `previous: None` returns `Read { new_label: true, marks: [one] }` when exactly one mark
stands, and `NeedsPick` when two or more do.
**Done when**: synthetic tests pin one mark (within 0.2 mm), a clipped one, none, and two (needs a pick).
Over P3's first-strike photos the finding tables the outcomes, with each review image looked at.
**Verify**: `cargo test -p impact`, the verb, clippy, fmt.

### P12 — 🪟 The newest mark: the diff against the last strike

**Goal**: D4's rule on a used label: what is new since the last photo.
**Read**: P11's finding; D4, C5, C10.
**Touch**: `crates/impact/src/tape/diff.rs`, `crates/impact/src/tape/mod.rs`,
`crates/impact/tests/tape_diff.rs`.
**Do**: build `read_tape(photo, Some(previous))` from these steps:
- **The diff**: both crops are in label millimetres, so align them with a small translation search (±2 mm,
  for the label's re-find error), then take the per-pixel contrast change.
- **The new marks** are the components of the change above a threshold.
- **A new label**: the previous crop's marks are absent from this one. Then the label is new, and P11's
  single-mark rule applies.
- **Outcomes**: one new mark is `Read`, two separated new marks are `Read` with two (a skipped photo, C10), an
  overlap or more than two is `NeedsPick`, and none is `no_new_mark`.
**Done when**: synthetic sequences pin each outcome. Over P3's sequences, run with `--sequence`, the
finding tables outcomes per label, and **both deliberately skipped photos come out as two new marks**.
**Verify**: `cargo test -p impact`, the verb, clippy, fmt.

### P13 — 🪟 The truth reader, the measured error, and the real-photo gate

**Goal**: exit item 1. It also floors C9's tables.
**Read**: D7, C9, C24; P2's sheet geometry (`spikes/strike-tape/README.md`); P4's inventory;
`crates/ocr/tests/full_swing.rs` and `crates/lab/tests/full_swing_clips.rs` (the ignored-gate precedent).
**Touch**: `crates/impact/src/tape/truth.rs`, `crates/lab/tests/strike_tape.rs`,
`crates/contracts/src/drill.rs` (the floored tables and `TAPE_ERROR_P95_MM` per label type),
`data/raw/strike_tape/<date>/truth.json` (gitignored).
**Do**: read the truth, write the gate, then floor the tables.
- **The truth**: in each truth-sheet photo, find the four fiducial squares, rectify the sheet to millimetres,
  find each slot's label edges (the label's own centre, per D2) and its marks. Every mark on a peeled label is
  every strike that label took, in sheet order. The last photo of each label in the sequence matches the
  peeled label, so each truth mark is matched to the strike that made it by the diff order. Write
  `truth.json`. What the truth cannot see is stated in the doc: the mark's segmentation is shared with the
  reader, so this measures the geometry chain (finding, squaring, orientation, curvature) and not
  segmentation.
- **The gate test** (ignored by default, rule 10): every photo is held to its sha256. Each strike is read
  (error measured), `NeedsPick`, or a named refusal, and a wrong reading fails (C24). It prints coverage and
  P50 and P95 per label type, and holds `READ_AT_LEAST` at what it measured.
- **The floor**: set `TAPE_ERROR_P95_MM` per label type, apply C9's floor to the tables (any cut radius named
  in the finding), and set `TAPE_READER_VERSION` to 1 as the first gated reader.
**Done when**: the gate passes with `--include-ignored`, the finding holds the error table and the floored
tables, and the gate fails without the photos rather than passing.
**Verify**: the gate's command, `cargo test -p contracts -p impact`, clippy, fmt.

### P14 — 🪟 `analysis::drill`: the baseline and the ladder

**Goal**: C10 and C11 as pure functions.
**Read**: C9 to C11; `crates/contracts/src/drill.rs`; `crates/analysis/src/strike.rs`'s top doc (house style
for a judgment module).
**Touch**: `crates/analysis/src/drill.rs`, `crates/analysis/src/lib.rs`, `crates/analysis/tests/drill.rs`.
**Do**: `counted(strikes)`, `inside(strike, radius)`, `start_level(baseline, category)`, and
`judge_block(block, level, category) -> BlockResult { inside, counted, move: In | Out | Stay, new_level,
at_widest_out: bool }`. Moving in at the tightest level stays and says so, and so does moving out at the
widest.
**Done when**: tests pin 7 in (move in), 3 in (move out), 4 to 6 (stay), both table ends, a `NotRead` not
counted, two marks counted, `declared_off_tape` outside, and a start level by C11 including the "fewer than 5
at the widest" case.
**Verify**: `cargo test -p analysis`, clippy, fmt.

### P15 — 🪟 `analysis::drill`: the pattern, the triggers and differential judging

**Goal**: C12 to C14.
**Read**: C12 to C14; P14's functions; `crates/analysis/src/stats.rs` (`mean_ci`, `percentile`).
**Touch**: `crates/analysis/src/drill.rs`, `crates/analysis/tests/drill.rs`.
**Do**: `pattern(strikes) -> Option<Pattern { centre, centre_ci, r50, spread, n }>` over C12's window,
`suggestion(state, last_block, pattern) -> Option<Suggestion>` by C13, and `call_made(strike, call, radius)` by
C14, with a recentre block's goal judged by its own mean.
**Done when**: tests pin each trigger firing and not firing at its edge, a declined suggestion offered
again, differential strikes kept out of the pattern, each call's edge, and no pattern below 10.
**Verify**: `cargo test -p analysis`, clippy, fmt.

### P16 — 🪟 `feedback::drill_rules`: the sentences

**Goal**: every sentence the drill shows, in one place, with no golf in Dart.
**Read**: C13, C20, C22; `crates/feedback/src/profile_rules.rs` (house style for ranked sentences);
`crates/pyfmt/src/lib.rs` (the `.Nf` formatter, so numbers print as everywhere else).
**Touch**: `crates/feedback/src/drill_rules.rs`, `crates/feedback/src/lib.rs`,
`crates/feedback/tests/drill_rules.rs`.
**Do**: write the sentences as functions over P14 and P15's results. `feedback` does not depend on
`analysis` (ADR-008's cargo edge), so the inputs are `contracts::drill` shapes and plain numbers.
- **The offset line**: "8.4 mm toe, 2.1 mm high", or "dead centre" within 0.5 mm on both axes.
- **The baseline tip**: "Half your strikes landed within 14 mm. Your target: 7 of 10 inside 12 mm."
- **The block result**: in, out or stay, with the next target.
- **Each suggestion**, with why it fired: "Your strikes sit 6 mm toward the toe. Move the pattern before
  shrinking it." For differential: "Your strikes are spread wider than the widest circle. Try differential
  practice: hit the spot I call."
- **Each not-read reason's sentence**, as P6 wrote them.
**Done when**: a test pins every sentence for a named input, singular and plural included, and no sentence
carries a raw unrounded float.
**Verify**: `cargo test -p feedback`, clippy, fmt.

### P17 — 🪟 `storage::drill_store`: the drill history on disk

**Goal**: C7.
**Read**: C7, C8; `crates/storage/src/lib.rs` (`write_atomically`, and the lock the shot store uses);
`crates/storage/src/shot_store.rs` (an append-and-read store's shape).
**Touch**: `crates/storage/src/drill_store.rs`, `crates/storage/src/lib.rs`, `crates/storage/tests/drill_store.rs`.
**Do**: `DrillStore::open(root, player_id)`, `append_strike`, `strikes(club)`, `state()`, `put_state`, the
crop path for a strike, and the club's `last/` reference read and write. Writes go under the lock, and a
torn last line of `strikes.jsonl` is skipped and counted on read, as the shot store's tolerant readers do.
A strike id already held is refused.
**Done when**: tests on a scratch root pin a round trip, a duplicate refused, a torn line tolerated, two
clubs kept apart, and the layout's paths exactly.
**Verify**: `cargo test -p storage`, clippy, fmt.

### P18 — 🪟 Facade: `record_tape_photo`, the pick and "missed the tape"

**Goal**: C15's first three rows and C16.
**Read**: C5, C6, C10, C15, C16; `crates/app/src/api/record.rs` and `crates/app/src/record.rs` (the
photo-in, outcome-out pattern, and `PhotoIn`); `crates/app/src/api/mod.rs`'s doc (rule 7).
**Touch**: `crates/app/Cargo.toml` (`impact`, `analysis`, `feedback`, `jpeg-encoder = "=<exact>"`),
`crates/app/src/api/drill.rs`, `crates/app/src/api/mod.rs`, `crates/app/src/drill.rs`, `crates/app/src/lib.rs`,
`crates/app/tests/drill_record.rs`.
**Do**: read the raw file, delete it, and run `read_tape` against the club's `last/` reference, holding the
`label_id` and `n_on_label` counters in state. Then act on the outcome:
- **`Read`**: store one or two strikes, a crop JPEG each, and the new `last/`; judge the block when it
  reaches 10.
- **`NeedsPick` or `no_new_mark`**: hold a pending strike with its crop.
- **`pick_mark` and `declare_off_tape`**: resolve a pending strike.

`TapeOutcome` carries the card's strings (C20) and the block line.
**Done when**: tests drive raw files written from synthetic frames through every outcome, and the stored
strikes, crops, state and deletions are checked on a scratch root. `ort_reach` still passes.
**Verify**: `cargo test -p app`, clippy, fmt, `ort_reach`.

### P19 — 🪟 Facade: the drill's actions (start, stage, suggestions, calls)

**Goal**: C15's `drill_clubs`, `start_drill`, `answer_suggestion` and `set_stage`.
**Read**: C11, C13, C15; P14 to P16's functions; `crates/app/src/api/setup.rs` (`bag_slots`, for the clubs).
**Touch**: `crates/app/src/api/drill.rs`, `crates/app/src/drill.rs`, `crates/app/tests/drill_actions.rs`.
**Do**: `DrillView` holds the stage, the level line, the block line, the pending suggestion's sentence, and
whether a call is wanted (differential). A club whose category has no label is listed with "No STRIKEPRO
label fits this club" and cannot start.
**Done when**: tests walk a club through the whole cycle on a scratch root: baseline → ladder → a declined
suggestion offered again → recentre → ladder → differential → fresh baseline. They also pin a mid-block stage
change abandoning the block.
**Verify**: `cargo test -p app`, clippy, fmt.

### P20 — 🪟 Facade: the progress views (dots, level, pattern, heat map, roll-up)

**Goal**: C15's two progress rows, C20 and C21.
**Read**: C10, C12, C20, C21; `crates/app/src/api/profile.rs` (a view built for Dart).
**Touch**: `crates/app/src/api/drill.rs`, `crates/app/src/drill.rs`, `crates/app/tests/drill_progress.rs`.
**Do**: `ProgressView` holds:
- the label outline and current circle, in normalised coordinates;
- each positioned strike as a dot (with its stage, so differential dots can be drawn apart);
- the pattern's centre and spread, or its absence sentence;
- R50 and the level line;
- level history rows `{date, from, to}` as strings;
- the heat map as rows of 0 to 1 values, or its "needs N more" sentence;
- for a category, each club's level line.

**Done when**: tests pin a club's view at 0, 9 and 10+ strikes, a heat map's peak equal to 1, a category over
two clubs, and that differential strikes are dots but are not in the pattern.
**Verify**: `cargo test -p app`, clippy, fmt.

### P21 — 🪟 Export carries `drills/`, and `golf-lab import-export` keeps it

**Goal**: exit item 3 and C17.
**Read**: C17; `crates/app/src/export.rs`'s doc (C16's list walked by name); `crates/lab/src/import_export.rs`.
**Touch**: `crates/app/src/export.rs`, `crates/lab/src/import_export.rs`, `crates/app/tests/export.rs` (or the
existing round-trip test's file), `crates/lab/tests/import_export.rs`.
**Do**: add `drills/` to the walked list. Lab side: merge `strikes.jsonl` by `strike_id`, `state.json` by
`updated_at`, and crops if absent. `last/` is a phone-side reference and is not imported. Print counts in the
verb's existing summary style.
**Done when**: a round trip from a scratch phone root to a scratch lab root keeps every strike, the state and
every crop. A second import changes nothing (byte-compared). A newer phone state replaces an older lab one.
The whole workspace is green.
**Verify**: `cargo test -p app -p golf-lab`, the workspace command, clippy, fmt.

### P22 — 🍎 Swift: the photo decoded for Rust, and the tape camera's overlay

**Precondition (yours)**: push on the desk and pull on the Mac.
**Goal**: C16's Swift half, and a camera for tape.
**Read**: C16; `app/ios/Runner/PhoneChannel.swift`, `PhotoSource.swift`, `ScreenReader.swift` (its decode);
`app/lib/src/phone/phone.dart`; the M38 plan's C4 and C10.
**Touch**: `app/ios/Runner/PhoneChannel.swift`, `app/ios/Runner/PhotoSource.swift`,
`app/ios/Runner/PhotoDecode.swift` (new, added to `project.pbxproj` by C4's recipe),
`app/lib/src/phone/phone.dart`, `app/test/fakes.dart`, `app/test/phone_test.dart`.
**Do**: run `app/tool/codegen_check.sh` first, and commit the regenerated bindings with this phase. Then:
- **`decodePhoto(path)`** returns `{rawPath, width, height}`: the decode exactly as `ScreenReader.swift`'s,
  written as raw RGBA under `<Documents>/processed/.app/tmp/`.
- **`takePhoto(overlay:)`** takes an overlay argument, screen (today's words) or tape ("Fill the frame with the
  club head. Keep the hosel in view, shaft up."), with the screen overlay as the default so Record is unchanged.
**Done when**: `flutter test` and `phone_test.dart` pass. An integration test decodes one P3 HEIC through the
channel on the Simulator and Rust reads its size back.
**Verify**: the Dart commands, `sim_test.sh integration_test/phone_test.dart`,
`cargo build -p app --target aarch64-apple-ios`.

### P23 — 🍎 The Drills page: picker, strike loop, reading card, pick and missed tape

**Goal**: one strike, end to end, on the phone's UI.
**Read**: C15, C19, C20; `app/lib/src/screens/record_screen.dart` and `app/lib/src/state/record_flow.dart` (the
loop's pattern); `app/lib/src/core/core_api.dart` (`GolfCore`); `app/lib/src/widgets/shot_card.dart`.
**Touch**: `app/lib/src/screens/drills_screen.dart`, `app/lib/src/state/drill_flow.dart`,
`app/lib/src/widgets/tape_card.dart`, `app/lib/src/widgets/label_painter.dart`, `app/lib/src/core/core_api.dart`,
`app/lib/src/ui/shell.dart` (a fifth tab for now; P27 replaces the bar), `app/test/drills_test.dart`.
**Do**: build the page and the loop.
- **The picker**: Impact, then "Tape: STRIKEPRO", then the bag's clubs from `drill_clubs`.
- **The loop**: the tape camera, then `decodePhoto`, then `record_tape_photo`, then the card. The card is the
  label drawn by `label_painter` with the dot and the circle, plus Rust's strings.
- **`NeedsPick`**: shows the crop for a tap.
- **`no_new_mark`**: offers "Missed the tape" and "Retake".
**Done when**: widget tests over a fake `GolfCore` pin each outcome's screen. One test through `RustCore` with
a P3 photo stores a strike.
**Verify**: the Dart commands.

### P24 — 🍎 The Drills page: baseline, block end, suggestions, differential calls

**Goal**: the ladder and the stages in the UI.
**Read**: C13, C14, C19; `app/lib/src/state/challenge.dart` (the injected-`Random` draw);
`app/lib/src/state/drill_flow.dart`.
**Touch**: `app/lib/src/state/drill_flow.dart`, `app/lib/src/screens/drills_screen.dart`,
`app/lib/src/widgets/suggestion_sheet.dart`, `app/test/drills_test.dart`.
**Do**: the stages on screen.
- **The baseline**: a progress line ("Baseline: 6 of 10").
- **The block end**: a sheet with Rust's result and the next target.
- **A pending suggestion**: Accept or Not now, then `answer_suggestion`.
- **A stage menu**: `set_stage`, with a confirmation when it abandons a block.
- **In differential**: the call shown large, drawn by C19 and passed with each photo.
**Done when**: widget tests with named draws pin a baseline completing, each block move, a suggestion
accepted and declined, a call kept over a `NotRead`, and an abandon confirmation.
**Verify**: the Dart commands.

### P25 — 🍎 The progress view: dots and level, the heat map, the roll-up

**Goal**: D13 on screen.
**Read**: C20, C21; P20's `ProgressView`; `app/lib/src/screens/profile_screen.dart` (layout precedent).
**Touch**: `app/lib/src/screens/drill_progress_screen.dart`, `app/lib/src/widgets/label_painter.dart`,
`app/lib/src/widgets/heat_painter.dart`, `app/test/drill_progress_test.dart`.
**Do**: per club, the label with its dots (differential dots hollow), the circle, the pattern's centre and
spread, and the level line and history. A segmented control switches between dots and heat map, which paints
Rust's grid through a fixed sequential colour ramp. A category view lists its clubs' level lines over the
pooled label view. Absence sentences show as Rust gives them.
**Done when**: widget tests pin each absence, the toggle, and a category with two clubs. A golden-free test
checks the painter's call counts, so nothing pixel-exact is pinned.
**Verify**: the Dart commands.

### P26 — 🍎 The golf-ball navigation, as a widget on its own

**Goal**: C18's widget, before it touches the shell.
**Read**: C18; `app/lib/src/ui/theme.dart` (colours, dark first).
**Touch**: `app/lib/src/ui/golf_nav.dart`, `app/test/golf_nav_test.dart`.
**Do**: build `GolfNav(pages: [...], current, onSelected)`, with these parts:
- **the ball**: a `CustomPaint` with dimples;
- **the arc**: tapping the ball opens five greens in an arc, each a pin with a flag label, the current one
  marked, and the arc laid out to fit a 375 pt wide screen;
- **the roll**: an `AnimationController` of about 450 ms rolls the ball into the tapped green's cup, then calls
  `onSelected`;
- **closing and reduced motion**: a tap outside closes the arc, and `MediaQuery.disableAnimations` skips the
  roll.

Semantics are as C18.
**Done when**: widget tests pin opening, closing, selecting by semantics label (after the roll, and instantly
under reduced motion), the current page marked, and no overflow at 320 and 430 pt wide.
**Verify**: the Dart commands.

### P27 — 🍎 The ball replaces the tab bar, and every test is moved onto it

**Goal**: exit item 4.
**Read**: C18; `app/lib/src/ui/shell.dart`; `grep -rn "NavigationBar\|NavigationDestination\|byIcon\|tab"`
across `app/test` and `app/integration_test`, for every test that reaches a page.
**Touch**: `app/lib/src/ui/shell.dart`, `app/test/fakes.dart` (the one helper, `goTo(tester, 'History')`),
every test file that navigates, and `app/integration_test/*` likewise.
**Do**: remove the `NavigationBar`, mount `GolfNav` over the `IndexedStack`, keep Record first, and move every
test onto `goTo`.
**Done when**: `flutter test` passes in full, `grep NavigationBar app/lib` finds nothing, and
`sim_test.sh integration_test/smoke_test.dart` passes.
**Verify**: the Dart commands, the smoke integration test.

### P28 — 🍎 The whole drill on the Simulator, over P3's photos: desk-complete

**Goal**: everything before the iPhone, run together.
**Read**: `app/integration_test/whole_flow_test.dart` and `test_host.dart` (the `TestPhone` pattern, the M38
plan's C22); P4's inventory.
**Touch**: `app/integration_test/drill_flow_test.dart`, `app/integration_test/test_host.dart` (a
`takePhoto(overlay: tape)` override returning P3's photos in order).
**Do**: run one club's P3 sequence through the app: it fills a baseline (padded with the same photos if fewer
than 10, said in the test) and reaches a judged block. Reach a pick and a missed-tape declaration, open the
progress view, toggle the heat map, export, and import the zip with `golf-lab import-export` into a scratch lab
root. Navigation goes only through the ball.
**Done when**: the integration test passes, and so do the whole workspace and the whole `flutter test`.
**Verify**: `sim_test.sh integration_test/drill_flow_test.dart`, the workspace command, the Dart commands.

### P29 — 🧍 A real drill session on the iPhone, exported and imported at the lab

**Precondition (yours)**: the iPhone at the Mac for a `--release` install (the M38 plan's C20: free signing
lasts 7 days, and deleting the app deletes its data, so export first). Labels, two clubs of different
categories, and somewhere to hit.
**What you do**: these steps, with the app alone:
1. Install the build.
2. On club A, take a baseline and hit at least two ladder blocks.
3. On club B, take a baseline.
4. Answer any suggestion as you like.
5. Open both progress views and the heat map.
6. Export, and AirDrop the zip to the Mac.
**What the session does**: imports the zip with `golf-lab import-export` into `data/processed/drills/`,
checks exit item 2 against what was imported (strike counts, every strike accounted for, levels and history),
and imports it a second time to check exit item 3. It records anything that misread, with the crop.
**Done when**: exit items 2 and 3 hold, or the finding names what failed and the user decides.

### P30 — 🪟 Docs

**Goal**: the docs say what was built.
**Read**: every finding above; `docs/ARCHITECTURE.md` §1 and §4 headings; `data/README.md`; `CLAUDE.md`'s
crate paragraphs about `crates/impact` and `crates/app`.
**Touch**: `docs/ARCHITECTURE.md` (§1: `golf-lab read-tape`, the Drills page; §4: `drills/`),
`data/README.md`, `CLAUDE.md` (the `crates/impact` and `contracts` sentences, the new key in the invariants'
version paragraph), ADR-034 (an M46 addendum: the drill history, the ball navigation), ADR-037 (an as-built
addendum if a finding moved anything), and `docs/CONFORMANCE.md` (the strike-tape gate, beside the Full Swing
gates).
**Do**: write each change in the house's voice, pointing at code rather than copying numbers into prose (the
measured error lives in `contracts::drill`, not in a sentence).
**Done when**: the doc-truth test passes.
**Verify**: doc-truth.

### P31 — 🪟 Close: verify, WORKLOG, ROADMAP, memory, and the close commit

**Goal**: M46 closed.
**Read**: this plan's checklist and findings; `WORKLOG.md`'s top entry (for its shape only).
**Touch**: `WORKLOG.md` (a new top entry), `ROADMAP.md` (M46 ✅ with its date and phase count; M44's row
saying the drill framework now exists, at `drills/<player>/<topic>/<method>/`),
`C:\Users\aaron\.claude\projects\C--Users-aaron-Desktop-repo-golf-coach\memory\` (the product-direction
memory updated with M46's state), and this plan's checklist.
**Do**: run every Verify command once (the gate with `--include-ignored` where the photos are present, and
named as skipped where they are not), then write the three records, and commit `M46: …`.
**Done when**: everything is green or named, and the commit holds the close.
**Verify**: the workspace command, the gate, the Dart commands on the Mac if this runs there, and doc-truth.

---

## Phase findings

### P0 (2026-11-12)

1. **The plan was approved as drafted**, after a five-round interview on the Windows desktop. The
   ROADMAP's status row and §M46 now point here, and §M46's exit points at this plan's Exit.
2. **What the web says about STRIKEPRO** ([strike-tape.com](https://strike-tape.com/), fetched during
   planning): one universal label, packed as driver, iron and hybrid (30 each per 90), about 6 to 10
   hits per label, right and left handed alike. A trademark record calls them pressure-sensitive reusable
   impact labels. The product page rate-limited the fetch, so no dimensions and nothing about how a
   strike shows were found. **For P2 and P3**: the run sheet must therefore ask for every dimension,
   and P3's finding must say what a strike looks like on the label.
3. **For P8**: `golf-lab` already reaches `ocr::decode::load_image` (HEIC through ffmpeg), so the desk
   decode needs no new path.

### P1 (2026-11-12)

1. **ADR-037 is written**, `docs/decisions/037-impact-location-by-method-and-drills.md`, Accepted
   2026-11-12, and `docs/README.md` lists it with no addenda. **It amends no ADR**: it applies ADR-034 §3
   and the M37 addendum's strike-location rule as written. Its clauses are numbered 1 to 11 for later
   phases to cite: the interface (1), the tape's frame (2), the reader's answers and the gate (3),
   declarations (4), the framework and its layout (5), untracked and kept (6), the ladder (7), the pattern
   and suggestions (8), differential judging (9), the credit (10) and what does not change (11). C9's radii
   are pointed at rather than copied, because P4 and P13 cut them.
2. **P0 left the map's document count stale.** `docs/README.md` said 94 markdown documents while git
   tracked 95, the plan itself, so the doc-truth test failed at P1's start. P1's counts take both files:
   96 documents, 79 in `docs/`, 38 ADR files, 16 plans, and 37 decisions. **For every later phase that
   adds a committed `.md`** (P2's truth sheet and run sheet, if they are markdown): bump the map's counts
   in the same change, and `git add` the file before running doc-truth, because the test counts
   `git ls-files`.
3. **The plan's rejected alternative "tip only" was read as "tap only"**: the user tapping every mark,
   with no reader. That is the alternative ROADMAP §M46 raised ("the user tapping it, or the photo being
   read") and D4 declines as the last resort. If the user meant something else, the ADR's third option
   under Options Considered is the one to change.
4. **For P5**: `FaceImpact.method_version` has nothing to hold for `OverheadReplay`, because M42's replay
   reader has no version key (ADR-034's M42 addendum). ADR-037 clause 1 names the gap and leaves it to the
   first milestone that makes a `FaceImpact` from a replay, since nothing in M46 does. P5 should say so in
   the doc comment rather than invent a replay version key, which would be improving M42's reader (Not in
   M46).
5. **For P14 and P15**: C11 gives no start level to a baseline whose widest circle holds fewer than 5, and
   C14's `R` is "the last ladder level, or the baseline's start level". So after such a baseline, which
   suggests differential practice (C13), `R` is unstated, and ADR-037 clause 9 repeats the plan without
   deciding. The widest radius is the obvious reading. If the plan still says nothing more by then, that
   is the user's call, and the phase reports `BLOCKED` rather than choosing.
6. **For P15**: ADR-037 clause 8 reads C13's recentre trigger as both conditions on the **same** axis: on
   some axis, the 95% interval excludes 0 and that axis's mean is more than a third of the radius from 0.
7. **For P30**: nothing routes to ADR-037 yet outside the map. CLAUDE.md's *Where the answer is* table and
   `docs/ARCHITECTURE.md` (with C22's credit) are P30's to add.
