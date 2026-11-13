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
| **P2** | 🪟 The truth sheet and the tape session's run sheet | ✅ Done *(2026-11-12)* |
| **P3** | 🧍 The labels: one flat photo, their sizes measured off it, and the user's answers | ✅ Done *(2026-11-12)* |
| **P4** | 🪟 The session's inventory and the label geometry as data | ✅ Done *(2026-11-12)* |
| **P5** | 🪟 `contracts::face_impact`: the method interface and `TAPE_READER_VERSION` | ✅ Done *(2026-11-12)* |
| **P6** | 🪟 `contracts::drill`: the strike record, the stages and the ladder tables | ✅ Done *(2026-11-12)* |
| **P7** | 🪟 `impact::tape` geometry: the homography and the rectified crop | ✅ Done *(2026-11-12)* |
| **P8** | 🪟 `golf-lab read-tape`: the desk tool and its review images | ✅ Done *(2026-11-12)* |
| **P9** | 🪟 The label finder, on synthetic photos | ✅ Done *(2026-11-12)* |
| **P10** | 🪟 Orientation: heel and up, from the hosel | ✅ Done *(2026-11-12)* |
| **P11** | 🪟 The mark on a fresh label | ✅ Done *(2026-11-12)* |
| **P12** | 🪟 The newest mark: the diff against the last strike | ✅ Done *(2026-11-13)* |
| **P13** | 🪟 The reader end to end on synthetic photos, and its gate | ⬜ |
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
| **P28** | 🍎 The whole drill on the Simulator, over synthetic photos: desk-complete | ⬜ |
| **P29** | 🧍 The first real test: a drill session on the iPhone, exported, imported and checked at the lab | ⬜ |
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
1. **The reader works on real strikes, tried for the first time at P29**, with **no wrong reading**. Every
   strike of P29's session is read, sent to a pick, or refused by name. Before P29 the reader is built and
   gated on synthetic photos only (P13). *Changed at P3 by the user's word (P3 finding 10).* No label is hit
   before the end and there is no truth sheet, so P3's truth set is gone. What a reading is checked against,
   and whether P50 and P95 can still be measured per label type, is P29's to settle with the user.
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
| `cargo test --release -p golf-lab --test strike_tape -- --include-ignored 2>&1 \| tail -20` | P29, if it writes the real-photo gate, and every phase after it touching `impact::tape` (moved from P13 at P3, finding 10) |
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
  change to this one. *Changed at P3 by the user's word (P3 findings 10 and 11)*: no ruler was used, and no
  size is published, so the dimensions are measured off P3's flat photo. The user reports a strike as a
  darker mark (finding 8).
- **D2 — Face centre is the label's centre, on every club.** The user's call, made knowing that application
  error then lands in every reading unmeasured: *"let's not worry about that for now"*. A reading is the mark's
  position in the label's own frame.
- **D3 — Every club.** Each category takes one of the three label types. Which label goes on fairway woods
  and on the putter (if any) is the user's answer at P3 (the run sheet asks), and P4 records it. *Answered at
  P3 (finding 8): a fairway wood takes the hybrid label, and the putter takes none and is out of the drill.*
- **D4 — One photo per strike, newest mark.** There is no clean photo of a fresh label. One mark on the label
  means a new label. More than one means diff against the last strike's crop. If the reader cannot tell, the
  user taps the new mark, which is the last resort (*"a doomsday task where we have utterly failed"*).
- **D5 — Drills only, and everything kept.** Strikes never feed the corpus or M37's strike-location grade in
  M46 (rule 5), and every strike's data is kept so a later milestone can decide.
- **D6 — Images are kept as a small rectified crop.** The label is cut out, squared to its own frame and
  stored as a compressed JPEG of tens of KB, and the original photo is deleted once the strike is stored.
- **D7 — The truth is a printed mm grid.** P2 prints a grid sheet. In P3, peeled labels are stuck onto it in
  order and photographed straight on, and that photo is the truth the on-club readings are compared to.
  *Changed at P3 by the user's word (P3 finding 10)*: there are no truth sheets beyond the one flat photo of
  unused labels, and no label is hit before P29, so no peeled-label truth exists. P2's sheet stays in
  `spikes/strike-tape/`, unused.
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
provenance: "measured by the user at P3 with <tool>". *Changed at P3 (finding 11)*: the provenance is P3's
flat photo, as finding 11 words it.

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
  never tighter than the reader can resolve (ADR-010 §2). *Moved to P29 at P3 (finding 10)*: the real error
  is first measured there, if it can be, so the tables ship trimmed and unfloored until then.

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
withholding is seen (the clip gate's precedent). *Changed at P3 (finding 10)*: P13 applies this rule against
its synthetic truth and sets no floor from it. The real gate, with `READ_AT_LEAST` set from real photos,
moves to P29, where what counts as the truth is settled with the user.

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

### P3 — 🧍 The labels: one flat photo, their sizes measured off it, and the user's answers

**Rewritten at P3 by the user's word** (2026-11-12, relayed by the orchestrator in four rounds). The later
round wins wherever two conflict:
1. *"Ignore strike photos"*.
2. A fairway wood takes the hybrid label. The putter takes none and is out of the drill. The three labels
   in the first photo are the driver (top), hybrid (middle) and iron (bottom) labels. A strike shows as a
   darker mark. The photo's sizes were not accepted as the record, which round 4 reversed, and the print
   check and truth sheets were kept, which round 3 replaced.
3. *"There will be no more truth sheets. Just the one. There will be no measuring labels with a ruler. Use
   the online information. There will be no hitting labels until we are done, the finished product should
   be our first "test" of the product and we are hoping it works. (In otherwords if testing is going to be a
   milestone it should be at the VERY end)."*
4. Nothing was found published (finding 11), so **the sizes measured off the photo are the record**:
   driver 90.5 × 49.6, hybrid 73.5 × 34.4, iron 73.0 × 39.9 mm, ±0.5 mm, on the assumption that the
   graph paper's squares are 1 mm.

So the run sheet, `docs/plans/m46-tape-run-sheet.md`, is now only a record of what was first asked. **Nothing
in M46 hits a label before P29**, which becomes the plan's first real test. Every real-photo check that was
placed earlier moves there, and P3's finding 10 says what each phase builds and tests against now.
**What P3 holds**:
- the one flat photo, filed (finding 2), which is the "one" truth sheet (finding 10);
- the user's answers (finding 8);
- each label type's size, outline, printed centre and printing, measured off that photo (finding 4) and
  sourced as finding 11 says.

**Done when**: met on 2026-11-12. Every label type has its size from a named source, every category has a
label or is named as having none, and nothing a later phase needs is left with the user.

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

### P9 — 🪟 The label finder, on synthetic photos

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

### P13 — 🪟 The reader end to end on synthetic photos, and its gate

**Rewritten at P3 by the user's word (P3 finding 10).** No label is hit before P29 and there is no truth
sheet, so this phase's old work moves to P29: the truth reader (`truth.rs`), the real-photo gate
(`crates/lab/tests/strike_tape.rs`), `TAPE_ERROR_P95_MM`, `READ_AT_LEAST` and C9's floor. Here the reader is
gated on synthetic photos with a known truth instead.
**Goal**: the whole reader, P9 to P12, checked end to end against known marks before the app is built on it.
**Read**: C4, C5, C10, C24; P9 to P12's findings (each one's synthetic renderer, to reuse rather than copy);
`crates/impact/tape_labels.json`.
**Touch**: `crates/impact/tests/tape_synthetic.rs`, and the shared renderer wherever P9 to P12 left it.
**Do**: render a set per label type, then run the reader over it.
- **The set**: each outline in `tape_labels.json`, with the dark parts of its printing (P3 finding 4), on a
  rendered head with a hosel and shaft, in both hands, square and 20 to 30° off square. Each label carries a
  run of 6 to 10 darker marks (P3 finding 8) at known label millimetres, with one photo holding two new marks and one holding
  an overlap.
- **The run**: run `read_tape` over each sequence. Every strike is read, `NeedsPick` or refused by name, and
  none is wrong by C24's `WRONG_MM` against the synthetic truth.
- **The record**: print coverage, P50 and P95 per label type, for the finding only. A synthetic error is
  not the reader's real error, so it sets neither `TAPE_ERROR_P95_MM` nor C9's floor, and
  `TAPE_READER_VERSION` stays at P5's 1.

**Done when**: the synthetic gate passes in the default `cargo test`, and the finding holds its table.
**Verify**: `cargo test -p impact`, clippy, fmt.

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

### P28 — 🍎 The whole drill on the Simulator, over synthetic photos: desk-complete

**Changed at P3 (P3 finding 10)**: P3 has no strike photos, so `TestPhone` returns P13's synthetic sequence,
written out as images, wherever this section says P3's photos. P4's inventory is not read.
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

### P29 — 🧍 The first real test: a drill session on the iPhone, exported, imported and checked at the lab

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
**Changed at P3 by the user's word (P3 finding 10): this is the plan's first real test, and nothing before it
hits a label.** It takes over every real-photo check the plan had placed earlier:
- P9's table of found against `no_label`;
- P10's "no photo oriented wrongly";
- P11's first-strike outcomes;
- P12's skipped photos read as two new marks;
- P13's real-photo gate, with C24's "wrong" and `READ_AT_LEAST`;
- exit item 1;
- the measured error, with `TAPE_ERROR_P95_MM` and C9's floor;
- P22's real HEIC decode, P23's strike stored from a real photo, and P28's real sequence.

**Settled here with the user, because the plan cannot settle them for this phase**:
- with no truth sheet, what a reading is checked against. What exists is each strike's stored crop with its
  reading drawn, looked at strike by strike;
- whether P50 and P95 can be measured at all;
- whether the original photos are kept for a desk re-run through `golf-lab read-tape`, since C16 deletes
  them;
- whether the run sheet's steps 5 and 6 (a deliberately skipped photo, and photos off square) are repeated
  in the app.

The floor and any `READ_AT_LEAST` this phase sets change shipped constants, so it commits them as P13 would
have done, and rule 4's `TAPE_READER_VERSION` rule holds.
**Done when**: exit items 1, 2 and 3 hold, or the finding names what failed and the user decides.

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

### P2 (2026-11-12)

1. **The sheet, its geometry and the run sheet are written**: `spikes/strike-tape/truth-sheet.svg`,
   `spikes/strike-tape/README.md` (the geometry, by the SVG's `id`s), and `docs/plans/m46-tape-run-sheet.md`
   (steps 1 to 9 as this section lists them, a `labels.txt` template, and a table of which phase reads each
   item). **The size was checked twice**: from the file (`width="180mm" height="250mm"` over a
   `viewBox` of 180 × 250, so one unit is one millimetre, and the scale bar's end ticks at x 40 and 140),
   and from an `rsvg` render at 10 px/mm (ImageMagick, 1800 × 2500 px), where the end ticks sit 1000 px
   apart and each fiducial is 100 px square. Nothing has been printed here, so a browser's print at 100%
   is unverified until the user's step 1, which is the check. `rsvg` fell back to its own sans font, and
   no position depends on the font.
2. **What the sheet adds to the plan's letter**, each for a reason the README states:
   - **An orientation key**: a fifth black square, 5 mm, at (15, 2.5) beside the top-left fiducial, because
     four identical squares cannot tell a sheet read upside down, which would move slot 1's label to slot
     6 and turn every mark through 180°. **For P13**: the key is not a fiducial (a quarter of one's area),
     and a photo with the key nearest the bottom-right square is upside down.
   - **The print is checked in both directions**: the 100 mm bar across, and the left squares' outer
     edges, 250 mm, down. Both measured numbers go into `labels.txt`'s `print:` line.
   - **The grid is blue**, every colour with a full blue channel, so it should nearly vanish in a photo's
     blue channel while the black stays. That is unmeasured. **For P13**: try the blue channel first if a
     label is see-through and the grid shows through it as structure.
   - **Slots are corner brackets and midpoint ticks, not outlines**, and the columns are 30 mm apart, so a
     label up to 90 mm wide still fits its slot, centred, overhanging. Wider than 90 mm, labels go in slots
     1, 3 and 5 only. Nobody knows the labels' size yet (P0 finding 2).
3. **The run sheet's choices**, made where the plan's steps left it open:
   - **Every irregular photo is noted by name**: a **miss** (a strike with no new mark, photographed
     anyway), a **retake** (a second photo of the same strike), **two new** (the skipped photo), and an
     **extra** (step 6's off-square photo, taken *after* the strike's square photo, so each label's run
     stays one photo per strike). **For P4**: these are what the inventory's "index on the label" and
     "skipped before" come from. A retake and an extra share their strike's index, a miss takes an index
     and has no mark, and a two-new photo spans two indices. An inventory field for the strikes since the
     last photo (0 for a retake or extra, 2 for two new) would make P13's matching mechanical.
   - **The skipped photo is never a label's first.** **For P12**: by P11's rule a fresh label with two marks
     is `NeedsPick`, so a skip there could never come out as two new marks, which is P12's done-when.
   - **No photo of the clean label on the club** (D4: the drill never has one), so a label's first photo
     follows its first strike.
   - **The flat photos are a `FRESH` sheet**: one unused label of each type stuck in slots 1 (driver),
     2 (iron) and 3 (hybrid). **For P4**: those slots are the cross-check, and P13's truth reader does not
     exist yet, so P4 reads them against the grid by eye or by rectifying through the four fiducials
     (README's table).
   - **Only the plan's two questions are asked** (fairway wood, putter), and the rest is stated as assumed
     (driver on the driver, hybrid on hybrids, iron on every iron and wedge), with a line to correct it.
   - **All three label types must be hit**, as an invariant rather than through the club list alone, since
     the error is per type. A second driver and hybrid label is suggested, not required, because the iron
     label gets two clubs and the others one.
4. **For P13**: how a peeled label sits is the README's last section. It goes in its slot unturned, top line
   toward the number (−y), and the toe left (−x) for a right-handed club and right (+x) for a left-handed
   one. Its table converts sheet millimetres to C4's frame, and the centre is the label's own (D2), not the
   slot's. The slot's band holds club, hand, label number and strike count.
5. **Two files outside the Touch line**: `docs/README.md`'s counts (98 documents, 80 in `docs/`, 17 plans,
   18 outside, nine in `spikes/`), as P1 finding 2 asked, and an entry in `spikes/README.md`, whose list
   names every spike folder.
6. **For P3**: the session checks the folder against the run sheet's last table, *What each item is for*,
   and `labels.txt` against its template.

### P3 (2026-11-12)

1. **The user's decision: no strike photos.** Their words, relayed by the orchestrator while P3 was open,
   were *"Ignore strike photos"*. P3's section records it. The run sheet's steps 4 to 6 are no longer asked
   for. What each phase loses, so that none finds out at its own start:
   - **Exit item 1 and P13.** The truth was each on-club reading compared with the same mark on the peeled
     label, and no on-club photo now exists. So there is no measured error, no `TAPE_ERROR_P95_MM`, no
     `READ_AT_LEAST`, and no real-photo gate for rule 10 to ignore by default. C9's floor (`2 × P95`) and C24's
     `WRONG_MM` have nothing to apply to, and P13 set `TAPE_READER_VERSION` "as the first gated reader".
   - **P4**: the inventory was one entry per strike photo, and only the flat photo below is left. The label
     geometry and the `category_labels` map are unaffected.
   - **P9, P10, P11 and P12**: each one's done-when ends in a run over P3's photos (found against `no_label`,
     no photo oriented wrongly, the first-strike outcomes, and both skipped photos read as two new marks).
     Their synthetic tests are unaffected. P10's handedness cross-check read the inventory, which came from
     the strike photos.
   - **P22** decodes "one P3 HEIC", and the one P3 photo is a JPEG (finding 2). **P23**'s test through
     `RustCore` stores a strike from a P3 photo, which needs a marked label on a club. **P28** runs "one club's
     P3 sequence" through the app.
   - **D7, P2's truth sheet, and the run sheet's steps 1 and 7** (the print check, peeled labels in their
     slots, and the sheet photos). The peeled labels' only reader was the comparison with the strike photos.
     The user's words do not say whether these go too, so finding 5 asks.
   - P29 is unaffected, because its photos are taken in the app.
2. **What arrived**: one photo, filed untouched as `data/raw/strike_tape/2026-11-12/da3ef6f9-image.jpg`
   (sha256 `5741307b…4c8f15`), under the day it arrived because it carries no date. It shows three unused
   labels on the user's own graph paper, not on the printed truth sheet, so there are no fiducials, no scale
   bar and no slot bands. It is not an original. It is 1512 × 2016, exactly half an iPhone's 3024 × 4032, a
   JPEG at quality 95 in Display P3. Its EXIF is stripped to the dimensions, with no date, make or model, and
   the file name is the upload's. There is no `labels.txt`, no print measurement and no truth-sheet photo.
3. **Which label is which, read by shape and unconfirmed** (finding 5). The **top** label is the driver label:
   the largest, an oval with distance-loss rings. The **middle** one is the hybrid label. The **bottom** one is
   the iron label, with straight sides and bottom and a dashed grid.
4. **What the photo measures, which is the record.** The user first declined these numbers as the record
   (finding 8), then took them as the record when nothing was found published (finding 11). The paper has
   no fiducials, so it was rectified by its own grid. The grid is
   1 mm, with every 5th and every 10th line heavier, so it reads as metric paper, and **every number below
   assumes its pitch is exactly 1.000 mm**. Grid lines were indexed in label-free bands and fitted as straight
   lines, and an image-to-grid homography over 37,359 intersections fits with an RMS residual of 0.13 mm (max
   0.71). Each label edge was taken where the grid lines stop, sampled every 1 mm. It agrees with a brightness
   edge to 0.2–0.4 mm on every side but the top (finding 7). Each label was then turned into its own frame by
   its printed cross's arms, since it sat 0.2° to 0.8° off square on the paper. The numbers are good to
   about ±0.5 mm, and the dashed printing to about ±1 mm:

   | Label (by shape) | Width × height, mm | Printed cross from left, top edge | Box centre − cross (+x right, +y up) | Inscribed radius about the cross | Red ring Ø |
   |---|---|---|---|---|---|
   | driver (top) | 90.5 × 49.6 | 45.2, 25.1 | +0.1, +0.3 | 24.4 (bottom edge) | 23.1 |
   | hybrid (middle) | 73.5 × 34.4 | 36.2, 17.6 | +0.6, +0.4 | 16.8 (bottom edge) | 18.3 |
   | iron (bottom) | 73.0 × 39.9 | 36.6, 24.8 | −0.1, +4.8 | 14.6 (bottom edge) | 18.3 |

   **None is a rounded rectangle.** Each shape is given below with the edge's distance from the cross every
   30° (0° is right in the photo and 90° up), where an angle is missing because the logo or glare hid the edge:
   - **driver**: an asymmetric oval, widest about 9 mm above the cross, with a flatter top and a rounder
     bottom. 0:43.1 30:39.6 90:25.1 120:28.2 150:40.5 180:43.3 210:34.1 240:26.5 270:24.4 300:26.2 330:32.8.
   - **hybrid**: a flat top edge, straight to within 0.3 mm over 55 mm, with rounded ends and a bottom that
     curves up into them. 0:35.9 30:34.2 60:21.0 90:17.6 120:20.3 150:33.8 180:34.9 210:27.7 270:16.8
     300:19.1 330:27.7.
   - **iron**: a straight bottom and sides, the sides vertical from about 13 mm below the top. From each side
     the top rises at about 22° to a flat run of about 20 mm under the logo. 0:36.3 30:33.9 60:27.3
     120:27.6 150:34.4 180:36.4 210:29.3 240:16.9 270:14.6 300:17.0 330:30.5.

   **Printing, colour, see-through.** All three are white and opaque, so no grid shows through. Across each
   label's interior, printing included, the grid's signal is 5 to 7% of the paper's around it, and on a
   print-free patch of the driver label it is under 1%. Each carries the navy `STRIKE⊕TAPE.com` logo, centred along the top edge, with a
   red target as the ⊕, and a red circle with a red cross (arms 9.5 to 10.2 mm) at the printed centre. On the
   driver label, two black dashed rings surround the circle, with black text: `-7%` at the top, `-15%` left
   and right, and `7% Loss of Distance` at the bottom. On the hybrid label, two black dashed elliptical rings
   cross the horizontal axis at about ±19.5 and ±28.5 mm, with no text. On the iron label, a black dashed grid
   has horizontal lines through the cross and about 10.5 mm above and below it, and vertical lines about
   18.5 mm either side, with a `+` where they cross.
5. **Still missing at the first report, asked through `BLOCKED`, and answered in finding 8**:
   - (a) the fairway wood and putter answers (step 3), without which P4's `category_labels` cannot be built;
   - (b) confirmation of finding 3's identities;
   - (c) whether finding 4's numbers stand in for step 2's ruler measurements, given the 1 mm assumption;
   - (d) whether steps 1 and 7 go with the strike photos;
   - (e) what a strike looks like on the tape (step 4's note, which P11 reads);
   - (f) `labels.txt`, or the same facts in the user's reply, which a resumed P3 writes into this finding.
6. **For P4**:
   - **C3's centre is not decided between the printed cross and the outline's box centre.** They agree
     within 0.6 mm on the driver and hybrid labels, and differ by 4.8 mm vertically on the iron label, whose
     top is shouldered. The cross is what a golfer aims at, and it is what finding 4's inscribed radii are
     measured about. If P4 finds no rule for this in the plan, it is the user's call.
     - **Decided by the user (2026-11-12): the centre is the printed red cross.** The user chose it over
       the outline's box centre after being given three reasons. First, the maker offset the cross on the
       iron label on purpose, so it is the maker's sweet spot and the outline's middle is only the shape
       of the sticker. Second, the printed −7% and −15% rings are centred on the cross, so a centre at
       the outline's middle would score a strike dead on the cross as 4.8 mm off. Third, the cross is a
       fixed point in each label's geometry, so choosing it costs nothing. Neither point is the club's
       true sweet spot, which depends on where the label is stuck; P29 is where that would show.
   - About the cross, finding 4's inscribed radii **trim C9** as follows. P4 computes the trim from the
     outline it commits, which is finding 4's, so this is the result it should get:
     - the driver table loses 30 and 26;
     - hybrid loses 26, 22 and 19;
     - `long_iron` loses 24, 21, 18 and 15;
     - `mid_iron` loses 22, 19 and 16;
     - `short_iron` loses 20, 17 and 15;
     - `wedge` loses 18 and 16;
     - `wood`, which takes the hybrid label (finding 8), loses 28, 24, 21 and 18;
     - `putter` has no label and no table.
   - The photo is the only source, so P4 has nothing to cross-check it against. If P4 re-derives a number,
     it repeats finding 4's method, rectifying by the grid, rather than the README's fiducials.
7. **For P9**: the logo sits within about 1 mm of every label's top edge. On the driver label the logo's
   target comes within 0.7 mm of it. A blurred-brightness outline therefore ran 0.7 to 1 mm inside the true
   top edge, and notched into the logo where the target and the swoosh near the edge. Expect the same notch
   when the label is segmented against a club face, and fit the top edge from the rest of the outline.
8. **The user's answers** (2026-11-12, relayed by the orchestrator), each addressed to the phases it affects:
   - **A fairway wood takes the hybrid label.**
     - **For P4**: `category_labels` maps `wood` to `strikepro_hybrid`. The rest is as the run sheet assumed:
       `driver` to `strikepro_driver`, `hybrid` to `strikepro_hybrid`, and every iron category and `wedge` to
       `strikepro_iron`.
     - **For P6**: C9's `wood` table ships, trimmed at the hybrid label's inscribed radius.
   - **The putter takes no label, and is out of the drill.**
     - **For P4**: `putter` is named as having no label.
     - **For P6**: C9's putter table does not ship (C9: "only if P3 maps a label to it"). It stays only in the
       doc comment, with the uncut tables.
     - **For P19 and P23**: the putter has no label type, so `drill_clubs` and the picker cannot offer it a
       tape drill. Whether it is left out or shown as unavailable is for that phase to settle with the user.
   - **The label identities are confirmed**: top is the driver label, middle the hybrid and bottom the iron
     (finding 3). **For P4**: finding 4's table is keyed by type.
   - *Superseded by finding 10, which drops the ruler.* **Step 2 still needs a ruler.** The user did not accept finding 4's numbers as the measurements. **For
     P4**: `tape_labels.json` takes the ruler's numbers, with C3's provenance ("measured by the user at P3
     with <tool>"). Finding 4's numbers are what P4's "within 1 mm" cross-check compares them with. Where
     the two disagree by more than 1 mm, P4's finding says which number won and why, as its section asks.
   - *Superseded by finding 10, which drops every truth sheet but the one photo.* **Steps 1 and 7 stay**: the print check, the peeled labels in their slots, and the truth-sheet photos
     are all still asked for, though the strike photos they were to be compared with are gone (finding 1).
     **For P13**: the truth sheets will be its only real photos of marks, with no on-club reading to match
     them against. What it measures from them is for P13 to settle with the user.
   - **A strike shows as a darker mark**, size not given. **For P11 and P12**: segment a mark as darker than
     the label's clean white, and a new mark in the diff as a darkening. On these labels the printing is dark
     too (finding 4: the navy logo, black dashes and text, and the red circle and cross), so a mark has to be
     told from the printing. A label type's printing is the same on every label, and the flat photo shows it
     clean (finding 2). No mark's size is known before P29 (finding 10).
9. *Superseded by finding 10: none of this is asked for now, and it stays as the record of what was
   asked.* **What is left at the bay to finish P3**, by run sheet step. With no strike photos, step 4 keeps only the
   hitting and its notes, and the per-label face finish is dropped, because it served the on-club photos.
   1. **Print the truth sheet** at 100%: one copy for the FRESH sheet, plus one for every six used labels. On
      each, measure the scale bar (99.5 to 100.5 mm) and the left squares (249 to 251 mm), and note both
      (step 1).
   2. **Measure one unused label of each type with a ruler or calipers**, width and height to 0.5 mm. None is
      a rounded rectangle (finding 4). So sketch each shape with its straight edges' lengths and its corners'
      positions, in mm from the label's left and top edges, and photograph the sketches (step 2). Finding 4
      already covers the printing, the colour and see-through, unless the user corrects it.
   3. **Stick those three unused labels on the FRESH sheet**: the driver label in slot 1, the iron label in
      slot 2 and the hybrid label in slot 3 (step 2).
   4. **Hit labels, with no photos**: the driver label on the driver, the hybrid label on a hybrid or fairway
      wood, and the iron label on a long or mid iron and on a wedge. Give each label 6 to 10 marks. Number the
      labels L1, L2 and on, and note each one's club, hand and strike count, misses included (step 4).
   5. **Peel each label into the next truth-sheet slot, unturned**: top line toward the slot's number, and the
      toe to the left for a right-handed club. Fill in the slot's band with the club, hand, label number and
      strikes (step 7).
   6. **Photograph every sheet straight on, twice each**, the FRESH one included, with all four corner
      squares in frame and no shadow (step 7).
   7. **Copy the originals, untouched**, into `data/raw/strike_tape/<date>/`, by AirDrop or with *Keep
      Originals* over USB (step 8). The first photo arrived re-encoded at half size (finding 2).
   8. **Write `labels.txt`, or give the same facts in a reply** (step 9). It needs:
      - the print numbers;
      - the ruler measurements;
      - each label's club, hand, type, sheet and slot, and strikes;
      - the photo names of the FRESH sheet and of every truth sheet.

      The two answers and "darker" are already recorded here.
10. **The user's third round replaces the second wherever the two conflict** (2026-11-12, relayed by the
    orchestrator, and quoted verbatim in P3's section). It brings three changes:
    - no truth sheet beyond "the one";
    - no ruler, so the published information is the record, and since none was found, round 4 made the
      photo's sizes the record (finding 11);
    - no label hit until the finished product, whose use at the very end is the first test.

    What follows from it:
    - **"Just the one" is read as the flat photo already filed** (finding 2). The other reading is one printed
      truth sheet still to come. With no label hit before P29, that sheet could hold only unused labels,
      which is what the filed photo already shows. It would add P2's fiducials and nothing a later phase
      reads, so the two readings change no phase, and the question was not put to the user.
    - **P29 is the plan's first real test.** It keeps its number, so no phase is renumbered. Its section
      lists what moved into it and what it settles with the user. Exit item 1, D1, D7, C3, C9's floor, C24
      and the verify table each say so where they stand.
    - **What each phase builds and tests against now**:
      - **For P4**: `tape_labels.json` takes finding 4's numbers (size, outline, printed centre and
        inscribed radius), with finding 11's provenance. None of the labels is a rounded rectangle, so each
        outline is a polygon. The inventory holds the one photo, and the category map is finding 8's.
      - **For P6**: the `// floored at P13` note becomes `// floored at P29`.
      - **For P8**: unchanged. Its done-when is already synthetic, and its verb is what P29 may run over any
        originals it keeps.
      - **For P9, P10, P11 and P12**: each is built and tested on synthetic photos only, rendered from
        `tape_labels.json`'s outlines with finding 4's printing and finding 8's darker marks. Each section's
        run "over P3's photos" moves to P29. P9 may use the filed flat photo as a smoke test of finding a label
        against a grid, but the photo shows no head.
      - **For P10**: the printed logo makes a wrong orientation visible on a stored crop, as mirrored or
        upside-down text. That is something P29 can check by eye.
      - **For P13**: rewritten in place, as a synthetic end-to-end gate that sets no floor and no
        `TAPE_ERROR_P95_MM`.
      - **For P18**: C16 deletes each original photo once its strike is stored, and P29 may want originals
        kept for a desk re-run. P18 keeps C16's delete as written, and its finding says where a switch to keep
        them would go, so that P29 can add one if the user wants it.
      - **For P22**: P3's one photo is a JPEG, so there is no P3 HEIC to decode. A HEIC from anywhere else
        will do, or the test decodes the JPEG and says so.
      - **For P23**: the `RustCore` test stores a strike from a P13 synthetic photo, written as C16's raw RGBA.
      - **For P28**: `TestPhone` returns P13's synthetic sequence, as its section now says.
      - **For P29**: everything above that moved, plus the four questions in its section.
    - **What this costs.** The reader, the ladder's levels and the app are all built without a real strike
      ever being seen. Until P29 the tables ship trimmed and unfloored, and nothing measures how far a real
      reading lands from its mark. That is the user's stated plan: *"we are hoping it works"*.
11. **No size is published, so the label sizes are the photo's.** The user's fourth round (2026-11-12,
    relayed by the orchestrator) made finding 4's sizes the record: driver 90.5 × 49.6, hybrid 73.5 × 34.4
    and iron 73.0 × 39.9 mm, ±0.5 mm. This reverses round 2's "not the record". **How the sizes were
    sourced**:
    - This phase-runner had no web tool: WebSearch and WebFetch were not available (checked through
      ToolSearch), and its rules forbid working around a missing tool.
    - The orchestrator searched the web from its own session and found nothing published:
      - strike-tape.com's home page lists no sizes;
      - the product page, `strike-tape.com/products/strike-tape-labels`, returned HTTP 429 twice;
      - the Amazon listing, ASIN `B0DWLVGJC1`, returned HTTP 503.
    - P0's fetch during planning had already found no dimensions on strike-tape.com (P0 finding 2).

    So every number came from one photo of three unused labels on the user's graph paper, read by finding
    4's method.
    - **For P4**: write the provenance into `tape_labels.json` once, in these words: "measured at M46 P3 off one
      photo of unused labels on graph paper (`data/raw/strike_tape/2026-11-12/da3ef6f9-image.jpg`, sha256
      `5741307ba794deb3f90dfdec01c6faa2c746f6bc973edde4061c23039c4c8f15`), rectified by its grid; assumes
      the paper's squares are exactly 1 mm; ±0.5 mm, the dashed printing ±1 mm; no size is published (P3
      finding 11)". If the 1 mm assumption is wrong, every size scales with it, so a ruler laid on any one
      label settles all three. That stays the user's to offer.
    - If Strike Tape later publishes sizes, they are a check on these, and a difference of more than 1 mm is
      the user's to decide.

### P4 (2026-11-12)

1. **The geometry is a Rust table, not `tape_labels.json`.** It lives in `crates/impact/src/tape/tape_labels.rs`, behind
   `crates/impact/src/tape/mod.rs`, and `lib.rs` declares `pub mod tape`. The section's own rule chose it: this crate
   takes no `serde`, and a hand parser of nested arrays, numbers, strings and `null`, plus the code that lifts the
   values into fields, passes 80 lines. Nothing outside Rust reads the geometry (C20). **For every later phase that
   names `tape_labels.json`** (C3, P6, P9 to P12), read it as `impact::tape`:
   - `LabelType` and `LABEL_TYPES`, with `STRIKEPRO_DRIVER`, `STRIKEPRO_HYBRID` and `STRIKEPRO_IRON`;
   - `label_type(id)`;
   - `category_label(wire_name)`, which returns `Some(Label(&LabelType))`, `Some(NoLabel { why })`, or `None` for a
     category the table has no row for;
   - `CATEGORY_LABELS` and `PROVENANCE`, the latter in finding 11's words.
2. **What a `LabelType` holds.** `id`, `width_mm`, `height_mm`, `centre_from_left_mm`, `centre_from_top_mm` and
   `inscribed_radius_mm` are finding 4's numbers exactly, the record. `outline_mm` is a polygon about the cross. None
   of the labels is a rounded rectangle, so there is no corner radius. `crates/impact/tests/tape_labels.rs` holds the
   record exactly, and holds the outline to the record within finding 4's ±0.5 mm. The cross is the centre, as the
   user decided in finding 6. That decision was in the tree uncommitted when P4 started, and P4's commit carries it.
3. **The outline is denser than finding 4's 30° profile, and comes from the same edge points.** Those are P3's
   grid-end points in the printed frame, still in the session scratchpad as `work/frame{1,2,3}.npy`, after
   finding 4's spike filter. The method:
   - take the median radius about the cross in each 2° sector;
   - simplify by Douglas–Peucker at 0.2 mm and round to 0.1 mm, which gives 42, 32 and 25 vertices (driver, hybrid,
     iron);
   - check that every kept edge point lies within 0.33 mm of the result.

   The edge was hidden in four places: the driver's 54° to 92°, the hybrid's 58° to 86° and 216° to 264°, and the
   iron's 72° to 118°. Those gaps are filled by the label's measured opposite side, mirrored about the outline's
   vertical middle, with a chord where both sides were hidden under the logo. A plain chord would have cut the iron's
   hidden top-left corner by about 0.8 mm, outside the ±0.5. The mirrored corner meets the measured left shoulder
   within 0.1 mm. The hybrid's mirrored bottom-left lies within 0.3 mm of P3's brightness outline. On the driver, a
   quadratic through the flanks of the whole gap departs by 0.34 mm at most from a chord across all of it, so the
   shorter chord left is closer still.

   The result agrees with the record:
   - **extents**: within 0.2 mm of the record's on every side;
   - **inscribed radii**: 24.33, 16.74 and 14.66 mm, against the record's 24.4, 16.8 and 14.6.

   The section's cross-check, each dimension within 1 mm, has no second source (finding 6). So it is this
   comparison of the outline with the record, and no number had to win.
4. **Finding 4's 30° profile is up to 0.8 mm off at two tight curves**: the driver's 150° and the hybrid's 60°. Each
   profile value was a median over ±4°, and the edge points within 1.5° of those two angles measure 39.1–40.5 and
   20.0–20.5 mm. The outline follows those points, so the test holds the profile at 1.0 mm and gives the reason.
5. **The table is in the label's printed frame, not C4's.** The origin is the cross, `+y` points toward the logo's
   edge, and `+x` toward the printed right, the end the logo's `.com` reads toward. Take a label stuck logo-up on a
   right-handed club: its printed left is at the toe, so C4's `x` is the printed `x` negated. On a left-handed club
   it is the printed `x`.
   - **For P7, P9 and P10**: draw, crop and find the label in the printed frame, and convert at the reading. The
     side comes from the hosel (P10), never from this table.
   - Do not assume the outline is mirror-symmetric. The cross sits up to 0.6 mm off the outline's middle on the
     hybrid label, so on a mirrored photo, mirror the stored outline.
   - The table assumes the label is stuck logo-up, toward the top line. Whether a label stuck logo-down is
     refused or turned is P9's and P10's to settle.
6. **The category map is finding 8's, plus `all`.** `ClubCategory::All` is named as taking no label, because no club
   maps to it (`contracts::club::CLUB_CATEGORY`). `putter` is named the same way, with the user's reason. **For
   P6**: `impact` cannot see the enum, so its test lists the nine wire names by hand. `crates/contracts/tests/drill.rs`
   should walk every `ClubCategory` through `impact::tape::category_label(c.as_str())` and fail on `None`.
7. **The trimmed tables, for P6 to commit.** Each is cut at the record's inscribed radius. The outline's own radii
   give the same cuts, and they match finding 6's prediction.

   | Category | Label (inscribed, mm) | Radii (mm) |
   |---|---|---|
   | `driver` | driver (24.4) | 22, 19, 16, 14, 12, 10 |
   | `wood` | hybrid (16.8) | 15, 13, 11, 9 |
   | `hybrid` | hybrid (16.8) | 16, 14, 12, 10, 8 |
   | `long_iron` | iron (14.6) | 13, 11, 9, 8, 7 |
   | `mid_iron` | iron (14.6) | 14, 12, 10, 9, 8, 7, 6 |
   | `short_iron` | iron (14.6) | 13, 11, 10, 9, 8, 7, 6, 5 |
   | `wedge` | iron (14.6) | 14, 12, 10, 9, 8, 7, 6, 5 |
   | `putter` | none | no table |

   The tightest margin is 0.6 mm, between `mid_iron`'s and `wedge`'s 14 and the iron label's 14.6. That is just over
   the measurement's ±0.5. C6's "outside every circle" holds on the measured label, and P29 is the first place
   that could show otherwise.
8. **The inventory** is `data/raw/strike_tape/2026-11-12/inventory.json` (gitignored). It has one entry, the flat
   photo, with its sha256, its strike fields `null`, and its three labels by position and type.
9. **For P9 and P11**: finding 4's printing is not in the table, because C3 does not ask for it. That means the ring's
   diameter, the cross's arms, the dashed rings and the logo band. A phase that renders the printing or masks it out
   adds those fields to `LabelType` then, from finding 4.
10. **For P30**: `crates/impact/Cargo.toml`'s `description` still names only the replay, and the manifest was not on
    P4's Touch line. Separately, `cargo doc -p impact` under `-D warnings` already failed before P4, on M42's private
    link at `frame.rs:51` (`sharpest_frame` to `luminance`). P4's docs add no warning.

### P5 (2026-11-12)

1. **`contracts::face_impact` is built**, `crates/contracts/src/face_impact.rs`, with C2's shapes as written:
   `ImpactMethod` (`overhead_replay`, `strike_tape`), `ImpactAxes { heel_toe, high_low }` from `axes()`, a
   `match` with no wildcard, and `FaceImpact`, `validated!` so that reading one from JSON applies the checks.
   `TAPE_READER_VERSION = 1` sits beside them, its doc a ledger in `SCREEN_PARSER_VERSION`'s style: what bumps
   it (any change to what a re-read of the same photo produces, after P13, per rule 4), why it is its own
   ledger, and that 0 is no entry. `crates/contracts/tests/face_impact.rs` holds the four done-when cases plus
   the axes, an unknown method, absent keys and the version's value. A unit test holds the refusal message's
   method name to its serde spelling. `crates/impact/src/lib.rs` gained one doc paragraph naming its reader
   `ImpactMethod::OverheadReplay`, and no code.
2. **On the wire an unread axis is an absent key, never `null`**, `crate::strike`'s convention (skipped when
   `None`, and a `null` that arrives reads as absent). C2 did not say which. **For P6**: `TapeStrike`'s
   `offset_h_mm` and `offset_v_mm` would match it by the same convention, and that is P6's to settle.
3. **The replay's `method_version` is left as P1 finding 4 asked**: the field's doc says it has nothing to
   hold for `OverheadReplay`, and that the first milestone making a `FaceImpact` from a replay settles it.
   No replay key was invented, and `Validate` does not check the field against its method. The tests write 0
   for a replay as a placeholder only.
4. **`Validate` checks only what C2 names**: a value on an axis the method cannot see (including 0, which is
   face centre and so a claim), and a non-finite value on either axis. It does **not** require any axis to
   hold a value, so a `FaceImpact` with both `None` is valid. **For P13 and P18**: if `Read` must carry both
   tape axes, the reader or the facade has to hold that rule, since this shape does not.
5. `cargo doc -p contracts` under `-D warnings` already fails, as P4 finding 10 found for `impact`, on three
   links that predate M46 (`unscored.rs:351`, `feedback.rs:44`, `swing.rs:48`). This phase's docs add no
   warning. **For P30**, beside finding 10's.

### P6 (2026-11-12)

1. **`contracts::drill` is built**, `crates/contracts/src/drill.rs`, with every shape and constant P6's section
   names, and `ladder_radii` returning P4 finding 7's tables exactly (an empty slice for `putter` and `all`).
   C9's uncut tables and both cuts are in the module doc, and `// floored at P29` sits above the tables (P3
   finding 10). It also has three id helpers, `strike_id(install, n)`, `label_id(install, n)` and
   `crop_file(strike_id)` (`crops/<id>.jpg`), so that the store and the facade spell them once. Beside them
   are `TapeMethod::impact_method()` and `TapeStrike::counts()` and `position()`. `crates/contracts/tests/drill.rs`
   has 38 tests, and `contracts` takes `impact` as a dev-dependency only, so the tables are checked against
   `impact::tape`'s inscribed radii, and every `ClubCategory` is walked through `category_label` (P4 finding 6).
   `Cargo.lock` gained that one edge.
2. **What C8 left open about a strike, and how P6 settled it.** Each answer is a `Validate` refusal with its
   own test:
   - **`source` is an `Option`, absent exactly when `not_read` is present.** C8's three sources cannot
     describe a strike that was not read, and the plan's own "no offsets with `read`" refusal would
     otherwise refuse one.
   - **A position is both offsets or neither**, written as absent keys. That settles P5 finding 2 by the same
     convention.
   - `call` is present exactly in the differential stage.
   - `level_radius_mm` is absent in the baseline and present in every other stage.
   - `crop` is `crop_file(strike_id)` or absent, and required on `read` and `picked`.
   - `reader_version` is at least 1, and `player_id` is a slug (`check_player_id`), because it names the
     directory.

   **`category` is not checked against `category_of(club)`**, on purpose. A strike records the table it was
   judged by, and checking it against today's map would refuse the whole history the day `CLUB_CATEGORY`
   moves a club. `label_type` is text, `impact::tape`'s `LabelType::id`.
   - **For P18**: a fresh-baseline strike stores no radius, even though the club still holds a level. A
     differential strike that is not read still carries its call (C19).
   - **For P14**: `counted(strikes)` filters by `TapeStrike::counts()`. `inside(strike, radius)` is yours and
     reads `position()`. P6 left inside out of `contracts`, because it is a judgment.
   - **For P17**: "the crop path for a strike" is `crop_file`, joined to the method's directory.
3. **What P6's section left open about the state, and how P6 settled it.**
   - **`DrillState`** has `player_id`, `strikes_issued` and `labels_issued` (the next id is `strike_id(install,
     issued + 1)`), `clubs: BTreeMap<ClubId, ClubDrillState>` in bag order (as `Bag` keys its entries), and
     `updated_at`.
   - **`ClubDrillState`** has `stage`, `level`, `block`, `suggestion`, `history` and `label`. `level` is an
     index into `ladder_radii(category_of(club))`, absent only in a club's first baseline. `block` also holds
     the baseline in progress. `label: Option<AppliedLabel { label_id, n_on_label }>` holds the counters P18
     names.
   - **`Validate` refuses**: a stage past the baseline with no level, a stored block of `BLOCK_SIZE` or more
     (a block is judged at its tenth), a level past its table, any state for a club whose category has no
     ladder (the putter), a label with `n_on_label < 1`, and negative counters.
   - **For P14 and P15**: `history` is `Vec<LevelEntry { at, from, to, block }>`, and it holds **one entry per
     judged block of any stage**, with `from == to` where the level stayed. `block` holds the block's strike
     ids. That is the only way C13's "two blocks in a row at 3 or fewer at the widest" is visible, because a
     failed block at the widest cannot move out. A block's stage and its inside count are read from its
     strikes, never stored. An abandoned block leaves no entry.
   - **For P18**: pending strikes (C15's `pending_id`) are not in `DrillState`, and P6 did not shape them.
     A discarded `NeedsPick` has no C5 reason. If it must be stored as not read, P18 adds a `TapeNotRead`
     variant here, with its sentence and its flag. If not, it stores nothing.
4. **For P15**: there are two constants beyond the section's six, so that C13's other numbers are not
   literals: `DIFFERENTIAL_AFTER_BLOCKS = 2` and `FRESH_BASELINE_AT = 7`. The second is a different success
   from `MOVE_IN_AT`, so it is its own constant. The seven counts are `usize`, because they compare with
   lengths, and `RECENTRE_FRACTION` is `f64`.
5. **`TapeNotRead`**: `retake_helps` is true for every reason but `mark_clipped`, where the strike itself ran
   off the label's edge. `no_new_mark` is true, which matches P23's "Missed the tape" and "Retake". The
   test holds each sentence to saying "take it again" exactly when a retake helps, and to holding no digit.
   - **For P16**: the not-read sentences are `TapeNotRead::sentence()`.
   - **For P13 and P18**: `impact::tape`'s reasons will be its own enum, since `impact` cannot see this
     crate. The facade maps them across by wire name. P13 should add a walk of them to
     `crates/contracts/tests/drill.rs`, as `category_label` is walked there.
6. **For P29**: the floor changes `every_table_is_c9s_trimmed_at_its_labels_inscribed_radius`, which derives
   each shipped table from C9's uncut one. A stored `level` is an index, so a revised table re-means it. The
   floor removes only the tightest radii, so it leaves every level above them alone, and a club whose level
   it removes is refused by `DrillState`'s check until it is moved. Every strike keeps the radius it was
   judged against.
7. `cargo doc -p contracts` shows only the three warnings from before (P5 finding 5). `ort_reach` and the
   doc-truth test pass.

### P7 (2026-11-12)

1. **The geometry is built**, in `crates/impact/src/tape/geometry.rs`, and `impact::tape` re-exports it:
   - `Homography::from_corners(photo, label) -> Result<Homography, Degenerate>`, which maps photo pixels to
     printed-frame millimetres. Each side is normalised (Hartley), then the 8 × 8 system is solved by Gaussian
     elimination with partial pivoting. `apply` returns `None` on or beyond the plane's horizon, `inverse()`
     cannot fail (both matrices are held), and `matrix()` is for review images.
   - `Degenerate`: `NotFinite`, `Collinear` (three corners on a line, two that coincide, or all four at one
     point), `Folded` (a bow-tie, or a corner inside the other three's triangle), and `Mirrored`.
   - `LabelType::box_corners_mm()`: the record's box about the cross, listed top-left, top-right,
     bottom-right, bottom-left.
   - `CropGrid { width, height, px_per_mm, left_mm, top_mm }`, with `new(label, px_per_mm)`, `to_label_mm`
     and `to_crop_px`, in continuous pixel coordinates (pixel `(i, j)`'s centre is `(i + 0.5, j + 0.5)`).
   - `rectify(frame, &h, label, px_per_mm) -> Crop { grid, frame, outside }`.
   - `CROP_PX_PER_MM = 10.0`.

   `crates/impact/tests/tape_geometry.rs` has 17 tests and `geometry.rs` 3 unit tests. `docs/REFACTOR_LEDGER.md`
   has C1's row (R4, Declined).
2. **Where the section's signatures moved, and why.**
   - `rectify` returns a `Crop`, not a bare `Frame`. "Outside pixels marked" needs a mask, because a `Frame`
     has no alpha, and any colour chosen to mean nothing is one a photo can hold. An outside pixel is
     black and flagged, and a frame that is not well formed comes back all outside at the grid's size.
   - `CropGrid` is a value apart from the pixels, because a stored crop is read back by its grid alone.
   - `CropGrid::new` panics on a `px_per_mm` that is not positive and finite. That is R8's wiring bug.
3. **`Mirrored` is a refusal the section did not name.** A front view of an opaque label keeps its handedness.
   The photo's `y` runs down and the label's up, so the forward matrix's determinant is negative, given that
   `w` is held positive. A positive one means the two corner lists pair as a mirror image, and reading through
   it would put every mark on the wrong side of the cross. That is a guessed sign (rule 6). A label turned
   to any angle in the photo is accepted, and a test pins all four quarter-turns of one quad.
   - **For P9**: list the photo's corners as seen, top-left, top-right, bottom-right, bottom-left, and pair
     them with `box_corners_mm()`. A wrong order is refused, never read. Whether a label stuck logo-down is
     refused or turned is still P9's and P10's (P4 finding 5). Here it is just another rotation.
   - **For P9**: `box_corners_mm()` is the record's box, and no label is a rectangle (P4), so its corners
     are not points on the label. They are where lines along the label's four extremes meet. The record box
     and the outline's extents agree within 0.2 mm (P4 finding 3).
4. **`CROP_PX_PER_MM` stays at the plan's 10**, and its doc carries the reason:
   - **The photo's sampling.** On P3's flat photo (half size, P3 finding 2) the label boxes span about 7.4
     (driver), 8.3 (hybrid) and 8.8 to 9.2 (iron) pixels a millimetre, read by eye off the photo. A full-size
     photo framed the same way would sample about 15 to 18.
   - **The crop sizes.** 905 × 496, 735 × 344 and 730 × 399, which as raw RGB (C16's `last/<club>.rgb`) is
     0.74 to 1.3 MB a club.
   - **The JPEG sizes.** Each box was cut from P3's photo by eye, resized (bilinear) and encoded with Pillow.
     That was a desk measurement in the session scratchpad, and nothing of it is in the repo. At 10 px/mm the
     three came to 36 to 58 KB at quality 75 and 47 to 76 KB at quality 85. That fits D6's "tens of KB". At
     8 px/mm they were 27 to 56 KB, at 5 12 to 26 KB, and at 12 47 to 99 KB.
   - **No mark size is known** (P3 finding 10). At 10, a pixel is 0.1 mm, and the tightest radius and
     `WRONG_MM` are each 50 pixels.
   - **For P18**: the quality those numbers suggest is 75 to 85, and the grid paper in the boxes' corners
     inflated them.
   - **For P29**: the first real marks are what revise the constant, and after P13 it moves with
     `TAPE_READER_VERSION`, because it sets every crop's size.
5. **What the tests measured.** The dot test has seven poses over all three label types: two face-on, and five
   tilted 28.7° to 30° about different axes. They are turned −90° to 180° in the image, at 110 to 200 mm, which
   is 5.5 to 10 photo pixels a millimetre. **The worst dot was recovered 0.0028 mm off**, inside the 0.1 mm by
   35 times, and the dot's photo position maps through the homography within 1e-9 mm. `inverse ∘ apply` holds
   within 1e-9 over a 50-pixel grid of each whole photo and a 2 mm grid of each label box. The 0.1 mm bound
   would pass a crop shifted by half a pixel (0.05 mm), so a second test pins the pixel-centre convention. At a
   pure one-to-one shift, the crop must equal the photo's pixels exactly.
6. **For P11 and P12**: `rectify` does not smooth before it samples. A full-size photo, at 15 to 18 px/mm,
   is decimated to 10, so the label's finest printing (the 1 mm dashes) may alias a little. A mark is much
   larger than that. Two crops of one label type at `CROP_PX_PER_MM` share one `CropGrid`, so P12's diff
   is pixel to pixel. A pixel flagged `outside` in either crop holds nothing and must be left out of the
   diff.
7. **For P10**: the crop is the printed frame. Row 0 is the box's top edge (the logo's), the cross is at
   `grid.to_crop_px([0.0, 0.0])`, and C4's `x` is the printed `x` negated or not, decided at the reading.
8. **For P18**: a tap on a stored crop becomes millimetres as
   `CropGrid::new(label, CROP_PX_PER_MM).to_label_mm([x, y])`, in the crop's pixel coordinates.
9. `cargo doc -p impact` shows only the warning from before (P4 finding 10). `ort_reach`, fmt, clippy and the
   doc-truth test pass.

### P8 (2026-11-12)

1. **The verb is built**: `golf-lab read-tape PHOTO... --label T [--sequence] [--review DIR]`, in
   `crates/lab/src/read_tape.rs` and wired in `crates/lab/src/bin/golf_lab.rs`. It decodes each photo through
   `ocr::decode::load_image`, runs `impact::tape::read_tape`, and prints one JSON line a photo, with every field
   of the outcome. It stores nothing. The exit codes are 0 when every photo was read, 1 when any needed a pick
   or was not read, and 2 for a usage error or a photo that is not there. `crates/lab/tests/read_tape.rs` runs
   the binary (4 tests): two drawn PNGs give two lines and their review images, a file that will not decode gives
   its own line, and every refusal is checked, including an unknown flag and a missing photo. The module has 7
   unit tests over outcomes built by hand. A smoke run on P3's flat photo (a 1512 × 2016 JPEG) gave `no_label`.
   Its review went to the session scratchpad, and nothing was written under `data/`.
2. **The stub, and C5's shapes, are in `crates/impact/src/tape/mod.rs`.** That file is not on P8's Touch line,
   but the section's Do needs the stub there, and P9's Touch line names `mod.rs` for replacing it. The stub
   answers `no_label` for any well-formed frame and `decode_failed` for a frame that is not well formed, and
   has 3 unit tests. The shapes are:
   - `TapeOutcome::Read(TapeReading)`, `NeedsPick(TapePick)` and `NotRead { reason, found }`, with `found()`;
   - `FoundLabel { corners, crop }`, whose corners pair with `box_corners_mm()` by index, so the homography is
     derived from them and not stored;
   - `TapeReading { found, orientation, new_label, marks }`, `TapePick { found, orientation }`, and
     `TapeMark { centroid_px, offset_h_mm, offset_v_mm }`;
   - `Orientation { toe_printed_right, top_line_printed_top }`, with `to_reading_mm` and `to_printed_mm`;
   - `TapeRefusal`, the five reasons, with `as_str()` (the wire name) and `ALL`.

   **For P9 to P12**: replace or extend these as each stage needs. The verb prints and draws whatever they hold,
   so a new field means a line in `json_line`, and in `write_review` if it should be drawn.
3. **What P8 settled that C5 left open.** A later phase may revise any of these:
   - **`NotRead` carries `found: Option<FoundLabel>`.** C5 names a bare reason, but C15 holds a `no_new_mark`
     strike pending "with its crop", and a refusal's review image is how the refusal gets tuned. **For P10 to
     P12**: set `found` for every reason that comes after the finder (`no_orientation`, `no_new_mark`,
     `mark_clipped`). **For P18**: a pending `no_new_mark` strike's crop is `found.crop`.
   - **`NeedsPick` carries the orientation**, because C6 turns the tap into millimetres in the reading frame.
     So orientation comes before the marks, and `no_orientation` refuses before any mark is looked for. **For
     P10 and P11.**
   - **An orientation is two flips and no turn**, because a label lies along the face. The four combinations are
     every real case: right- or left-handed, each with the logo up or down. A label turned 180° flips both axes.
     **For P10**: if logo-down is read rather than refused, it is `top_line_printed_top: false`, with the toe
     flag flipped as well.
   - **No sentences in `impact`.** `golf_lab::read_tape::not_read` maps a `TapeRefusal` to
     `contracts::drill::TapeNotRead` by wire name, and its test walks `TapeRefusal::ALL`. **For P13**: P6
     finding 5's walk in `crates/contracts/tests/drill.rs` is still wanted, because P18's facade maps the
     reasons again, in `crates/app`.
4. **`--label` is required, where C23 bracketed it.** No default is safe. The driver label's box is
   90.5 × 49.6 mm and the iron's is 73 × 39.9, aspects of 1.825 and 1.830. Read as the wrong type, a photo
   could pass an aspect check and come out at a scale 1.24 off, a wrong reading where a refusal was owed (rule
   6). The value is a label id, checked at parse time against `LABEL_TYPES` (argparse's `invalid choice`).
   **For P9**: `ASPECT_TOLERANCE` alone cannot tell the driver label from the iron, so if a default or an
   auto-detected type is ever wanted, the outline's shape has to make the call.
5. **`--sequence` reads each photo against a crop, not against the previous photo.** ADR-037 clause 3 says
   `previous` is the club's last crop. Here that is the crop of the latest photo that came out `Read` or
   `NeedsPick`, which is what the phone's `last/<club>.rgb` will hold: the last strike stored with a crop.
   - A refusal leaves the reference where it was. A photo that did not decode is read against nothing, and its
     `previous` is `null`. The line names, in `previous`, the photo whose crop was the reference.
   - **For P12 and P18**: since a refusal leaves the reference alone, the mark of a `mark_clipped` or
     `no_orientation` photo shows as new again on the next photo. The phone and the desk tool should follow
     one rule. If P18 picks another, the desk tool follows it in one `if let` in `read_tape_photos_with`.
   - **For P12**: the verb passes `crop.frame` alone, which is C5's `Option<&Frame>`, without the `outside`
     mask. That matches C16, whose `last/<club>.rgb` is raw RGB. An outside pixel is therefore black in the
     reference. To leave it out of the diff (P7 finding 6), either treat a black reference pixel as outside, or
     widen `previous` to a `Crop` and add the mask to C16's file.
6. **The review**, under `DIR/<stem>/`:
   - **`photo.png`**: the photo, shrunk to fit `REVIEW_MAX_SIDE` (1600). A full-size photo would be a PNG of
     about 25 MB, and P3's flat photo came to 4 MB at 1200 × 1600. It shows the found box in yellow, the outline
     through the same map in cyan, the cross in white, and once the label is oriented, `ARROW_MM` arrows toward
     the toe in red and toward the top line in blue. It is written for every photo that decoded, so a
     `no_label` photo gets one too.
   - **`crop.png`**: the crop at its own size, with the cross, the two arrows, and each mark ringed in magenta.
   - Before writing, both files a previous run left are removed. A photo that will not decode only has its
     stale files removed.
   - **For P11**: the ring is a fixed `MARK_RING_MM` (3 mm), because a `TapeMark` carries no extent. If a mark
     gains one, the ring in `write_review` can follow it.
   - The drawing reuses `read_impact`'s `ring`, `disc` and `segment`, which are now `pub(crate)`. That is a
     visibility change in `crates/lab/src/read_impact.rs` and nothing more.
7. Small things:
   - The line's keys come out sorted, because `serde_json` is built without `preserve_order`.
   - `crates/lab/Cargo.toml`'s `impact` and `image` comments name the new verb. No dependency was added, and the
     lock did not change.
   - `cargo doc` gives no new warning.
   - Clippy and fmt are clean. `cargo test -p impact`, `-p golf-lab --lib`, `--test golf_lab` and
     `--test read_tape`, `-p contracts --test drill`, `ort_reach` and the doc-truth test all pass.
8. **For P30**: `CLAUDE.md`'s list of golf-lab's other verbs, in the Commands block's `golf-lab analyze` comment,
   does not name `read-tape`, and neither does `docs/ARCHITECTURE.md` §1. P30's Touch line covers §1, but its
   `CLAUDE.md` entry names only the `crates/impact` and `contracts` sentences.

### P9 (2026-11-12)

1. **The finder is built**: `impact::tape::find_label(photo, label) -> Result<(FoundLabel, OutlineFit),
   LabelMiss>`, in `crates/impact/src/tape/label.rs`, and `read_tape` uses it in place of P8's stub. The
   module doc gives the steps. A pixel is a candidate when it is bright (above one of two Otsu levels) and
   neutral. The candidate regions the size of a label are hole-filled and checked for the label's aspect.
   Then the type's measured outline is fitted to each region's sub-pixel edge through a homography, from two
   starts (each way along the long axis). A fit is accepted on its measures and the logo, and exactly one
   distinct label must remain. **`read_tape` answers `no_orientation`, carrying the found label, for every
   label it finds until P10**, because nothing has looked for the hosel and a side would be a guessed sign.
   It answers `no_label` for any `LabelMiss` and `decode_failed` as before.
2. **Where the section's Do was not followed, and why.** None of these touches a decision.
   - **Not four edge lines.** The section asked for a rounded rectangle's corners from its four edge lines.
     No label is a rectangle (P4), and the driver label has no straight edge at all, so the whole measured
     outline is fitted instead (P7 finding 3 foresaw this). The corners are `box_corners_mm()` mapped
     through the fitted map, so they pair with the printed frame by index and the crop is cut logo-up.
   - **The aspect check is on the region.** A crop cut through a fitted outline has the label's aspect by
     construction. `ASPECT_TOLERANCE` therefore bounds the region's moment aspect against the outline's
     before the fit, and the fitted map's foreshortening at the cross after it.
   - **Files beyond the Touch line**:
     - `crates/impact/src/component.rs` gained `components_with_pixels`. The flood fill is shared, and
       `components` is unchanged, so M42's tests pass as they were.
     - `crates/impact/src/tape/geometry.rs`'s `gauss` is now `pub(super)`, so there is one 8 × 8 solver.
     - `crates/lab/tests/read_tape.rs` changed in its doc only. Its drawn photos are still `no_label`, now
       for the finder's reasons.
     - The shared renderer is `crates/impact/tests/tape_scene/mod.rs` (finding 11).
3. **The fit is measured in the photo.** The first version mapped edge points into the label and measured
   them there. On a notched label it walked off to a strongly projective map (`h₆ = −0.44`) that squeezed
   the points onto part of the outline. Measured in the photo, to the outline as the map projects it, the
   points cannot be shrunk. The method is Levenberg–Marquardt on point-to-tangent residuals, through three
   robust stages that narrow: Huber 1 mm, then Tukey 2 mm, then Tukey 0.6 mm. The last stage exists because
   notch points within 2 mm left a marked driver label 0.43 mm off. It is wider than P3's ±0.5 mm on the
   outline. Edge points are subsampled to 500 for the iterations, and all of them are judged.
4. **The logo decides which edge is the top, because the outline cannot.**
   - **The outline alone.** The hybrid label fitted the wrong way up scored 0.35 mm RMS, inlier share 0.90
     and support 0.84, and passed loose thresholds. So the photo was refused as ambiguous.
   - **The rule.** A fit is accepted only when the band across the logo's letters is darker than the band as
     deep in from the bottom edge, by `LOGO_CONTRAST`. Each band's darkness is `1 − mean / p90` of its own
     samples, so a light gradient across the label does not count. On P3's photo the iron label's bottom
     band was 13 levels dimmer at its white than its top band.
   - **The band, measured on P3's flat photo.** The real letters lie 3.2 to 6.2 mm below the top edge on all
     three labels, under a thin swoosh within about 1 mm of it. They span about ±16 mm (hybrid, iron) and
     ±19 mm (driver). The band is 2.5 to 6.5 mm in and ±12 mm across. A first guess, 1.2 to 4.0 mm, mostly
     caught white, and refused the real iron label.
   - **The scores.** Right way up, the real labels score 0.33 (driver), 0.20 (hybrid) and 0.18 (iron), the
     iron lowest because its grid's bottom line lies in its bottom band. Turned, they score about the
     negative.
5. **A wrong reading was found and closed.** Under the first thresholds an iron label read as the hybrid type
   was *accepted*. Those thresholds were inlier 0.8, support 0.8, no outside share, and the logo only vetoing
   a decisive upside-down. The accepted fit had inlier 0.80, support 0.87, RMS 0.47 mm and foreshortening
   1.27. Three measures now refuse it:
   - `MAX_OUTSIDE_SHARE`. A notch only ever falls inside the outline, so edge beyond it is white the label
     does not have.
   - Support of 0.9.
   - The logo, now required.

   `MIN_INLIER_SHARE` went down to 0.7. Notches add their inner sides to the edge, and a label whose swoosh
   crosses its top edge, with two strikes notching its bottom, scores 0.80 to 0.82. Across the final tests
   there were 72 accepted fits and 61 refused. **Every refused fit fails at least two measures.** Every
   accepted one had:
   - outside share 0;
   - support at least 0.955;
   - logo contrast at least 0.172;
   - foreshortening at most 1.181;
   - inlier share at least 0.797.
6. **The tunables**, each with its doc and the bracket P29 may move it within:

   | Constant | Value | Bracket |
   |---|---|---|
   | `LABEL_SATURATION_MAX` | 0.25 | 0.15 to 0.4 |
   | `MIN_PX_PER_MM` | 4 | 2.5 to 6 |
   | `ASPECT_TOLERANCE` | 1.3 | 1.15 to 1.6 |
   | `OUTLINE_INLIER_MM` | 1.0 | 0.5 to 2.0 |
   | `MIN_INLIER_SHARE` | 0.7 | 0.6 to 0.95 |
   | `MAX_OUTSIDE_SHARE` | 0.05 | 0.02 to 0.15 |
   | `MIN_OUTLINE_SUPPORT` | 0.9 | 0.7 to 0.97 |
   | `LOGO_CONTRAST` | 0.08 | 0.04 to 0.15 |

   The fit's own settings (the kernel widths, `FIT_POINTS`, the logo band's geometry) are private constants
   with their reasons.
7. **The tests.**
   - **Where they are**: `crates/impact/tests/tape_label.rs` holds 11 tests, and `label.rs` holds 5 unit
     tests (Otsu, the crossing, neutrality, the outline's moments and distance, and hole filling).
   - **What a found label is held to.** Every found label is within `FOUND_WITHIN_MM` (0.25) of the camera's
     truth, at every outline vertex and the cross. The worst was 0.19 mm, the marked driver label with its
     swoosh crossing the edge. Every other was 0.09 mm or less.
   - **What is found**:
     - each type face-on;
     - six poses tilted 28.7° to 30° and turned −90° to 180°;
     - left-handed clubs, and labels stuck logo-down, with the crop asserted logo-up;
     - every type on a steel face, a dark face and no face;
     - eight marks, two of them clipped by the bottom edge, plus the swoosh crossing the top edge.
   - **What is refused**:
     - no label: `NoCandidate` on a dark face, and on a steel face `OutlineMisfit` (the face is a candidate);
     - a white card the iron box's size: `OutlineMisfit`;
     - a disc: `WrongProportions`;
     - four wrong-type pairs;
     - a label cut by the photo's edge: `NoCandidate`;
     - one at 400 mm, about 2.5 px/mm;
     - two hybrid labels in one photo: `Ambiguous`.
   - **`read_tape`'s outcomes** each have a test too.
   - **Speed.** The file runs in about 9 s in a debug build. The renderer samples each pixel's corners once
     and supersamples only the pixels whose corners disagree, which took it from 40 s.
8. **P3's flat photo, the one real photo, as a smoke test** (P3 finding 10 allows it). It was read through
   `golf-lab read-tape --review`, with the reviews in the session scratchpad and nothing written under
   `data/`.
   - **All three labels are found** against the graph paper with no head behind them. Every one has inlier
     share and support of 1.0 and outside share 0, with RMS 0.22 (driver), 0.13 (hybrid) and 0.11 mm (iron).
   - **The fitted sampling matches.** It is 7.95, 8.35 and 9.0 px/mm, where P7 read 7.4, 8.3 and 8.8 to 9.2
     by eye.
   - **The cross is off by up to 0.9 mm.** The printed red cross's arms, found in each crop, lie 0.9 mm
     (driver), 0.3 mm (hybrid) and 0.2 mm (iron) from where the fit puts the label's centre. The paper is
     nearly as white as the label, so the edge there is faint, and P3 measured the outlines and the cross
     to ±0.5 mm.
   - **Speed.** A read takes 0.28 s in release, decode included. A 3024 × 4032 copy upscaled from it takes
     0.57 s.
9. **For P10**:
   - **The renderer draws what P10 needs.** It draws the hosel and shaft. A right-handed club seen face-on
     has its heel on the photo's right (C4): a hosel band 7 mm either side of a line from the heel, at a
     62° lie, and a graphite shaft 4.5 mm either side beyond 30 mm. `Hand::Left` mirrors the head, and
     `logo_up: false` turns the label half a turn on the face. `Scene::true_corners` and `photo_of` are the
     truth.
   - **The finder's corners are always the printed frame's**, whatever the label's turn on the club, so
     logo-down is P10's alone (`Orientation`'s doc now says so). P10 replaces the `Ok` arm in `read_tape`,
     which refuses as `no_orientation`.
   - **A steel face is bright and neutral**, so it is often a candidate region of its own. The renderer's
     shaft is dark graphite. A steel-shafted iron's real shaft is chrome and may not be the dark bar the
     section expects. That is for P29 to see.
10. **For P11**:
    - **The cross may anchor the reading better than the fit.** On the real photo the fitted centre lay up to
      0.9 mm from the printed cross (finding 8). Every crop holds that cross: red arms 9.5 to 10.2 mm long
      (P3 finding 4), and since D2 and P3 finding 6 it *is* the label's centre. Locating its arms in the
      crop and measuring each mark from them would take the fit's translation error out of every reading.
      The section does not ask for this. It is P11's to decide, and P29's to measure.
    - **A mark across the bottom band costs logo contrast.** Two strikes there took the synthetic iron label
      from 0.22 to 0.17, so a label marked hard along its bottom edge is refused as `no_label`.
    - `Scene.marks` draws discs at printed-frame millimetres with a 0.4 mm ramp, in the guessed `MARK`
      colour (120, 116, 122). P3 finding 8 says only "darker".
11. **For P13**:
    - **The shared renderer** is `crates/impact/tests/tape_scene/mod.rs`, included by `mod tape_scene;`.
    - **Its printing matches P3's photo where the finder looks.** That covers the logo's letters and swoosh,
      the driver's and hybrid's dashed ellipses (the outer ones 2 mm above the bottom edge), the driver's
      text, and the iron's grid. Its logo contrasts, 0.21 to 0.35, match the real 0.18 to 0.33.
    - **Keeping photos.** `TAPE_SCENE_DIR=<dir>` keeps every drawn photo as a PNG (`tape_scene::keep`, a
      std-only writer), to look at or to run `golf-lab read-tape --review` over.
    - **A second camera.** `tests/tape_geometry.rs` keeps P7's own copy of the camera, and P13 may fold it
      onto `tape_scene::Camera`.
12. **For P29**:
    - every tunable in finding 6, with its bracket;
    - a label on a curved driver face, which a homography takes as flat (the module doc);
    - the cross offset of finding 8;
    - chrome shafts (finding 9);
    - the logo band, which was measured on unused labels only.
13. **For P18**: on the desk a full-size photo is found in about 0.6 s in release, decode included (finding
    8). The finder holds the photo's luminance as `f32`, a neutral flag and a mask per level, about 6 bytes
    a pixel, which is about 75 MB on a 12 MP photo before the frame itself.
14. **For P30**: `impact::tape` now finds the label. `docs/ARCHITECTURE.md` and `CLAUDE.md`'s `crates/impact`
    sentence still describe the replay alone (P4 finding 10 already names the manifest's `description`).

### P10 (2026-11-12)

1. **The orientation is built**: `impact::tape::orient(photo, found, label, hand) -> Result<(Orientation,
   Shaft), OrientMiss>`, in `crates/impact/src/tape/orient.rs`, and `read_tape` calls it after the finder.
   The module doc gives the steps:
   - **a window**: the label's box grown by `SEARCH_REACH_MM` (150) on every side, cut through the found
     label's map at 1 sample a millimetre;
   - **dark**: at most `SHAFT_DARK_RATIO` of the label's white, the window's 90th percentile inside the box;
   - **thin**: what a disc `SHAFT_MAX_WIDTH_MM` across cannot reach inside the dark, an opening by an exact
     Euclidean distance transform. For the opening the label's box and the part of the window outside the
     photo count as dark, so a dark driver face round the label is not a thin rim and a dark strip along
     the photo's edge is not a bar;
   - **bars**: each 4-connected run of thin, dark pixels outside the box (the crate's `component`), its
     end nearer the cross the hosel's;
   - **a shaft**: long enough, slim enough, near the box, at a lie's angle, starting toward one end of the
     label and leaning further that way. Those two cues to the heel must agree;
   - **one answer**: shafts that disagree are `Ambiguous`, never chosen between, then the hand is checked.

   The shaft's side is the heel and its direction up, so both of `Orientation`'s flips come from the club.
   `Handedness` (`Right`, `Left`, with `as_str`) and `Orientation::handedness()` are in `mod.rs`: the two
   flags agree on a left-handed club and differ on a right-handed one, whichever way up the label was stuck.
2. **What P10 settled that the plan left open.** None of these touches a decision.
   - **`read_tape` takes a fourth argument, `hand: Option<Handedness>`.** C5 names three, and so does
     ADR-037 at line 157. The section's cross-check read the inventory's handedness, and the inventory has
     none since P3 (P3 finding 1), so the hand is now the caller's. `None` skips only the cross-check, and
     the shaft still decides. `golf-lab read-tape` passes `None`, because the desk has no golfer. Its
     `read_tape_photos` wraps the reader in a closure, so its `Reader` type and its tests did not change.
   - **A label stuck logo-down is read, not refused** (P4 finding 5, P8 finding 3). The shaft says which
     way is up, so `top_line_printed_top` is `false` there and the toe flag flips with it.
   - **An oriented label is `NeedsPick` until P11.** Nothing looks for a mark yet, and a pick is the one way
     to place one without a guess. This is the same kind of placeholder as P9's `no_orientation`.
   - **M42's `shaft.rs` is not reused.** It measures in ball diameters and joins a head bar's ends, and
     here there is neither. A driver's face is as dark as its shaft, so the opening is new. Its rule that
     bars at both ends decide nothing is kept, as `Ambiguous`.
   - **Files beyond the Touch line**:
     - `crates/impact/src/tape/geometry.rs`: `rectify` now calls `rectify_onto(frame, map, grid)`, a
       `pub(super)` that samples onto any `CropGrid`. The window uses it.
     - `crates/impact/tests/tape_scene/mod.rs`: finding 9 below.
     - `crates/impact/tests/tape_label.rs`: its last test was P9's placeholder ("until P10"). It is now
       `read_tape_answers_no_label_for_a_photo_the_finder_finds_none_in`, and the found-label case moved to
       `tape_orient.rs`.
     - `crates/lab/src/read_tape.rs`: the closure, and its module doc, which said the reader was a stub
       until P9.
3. **The tunables**, each with its doc and the bracket P29 may move it within:

   | Constant | Value | Bracket |
   |---|---|---|
   | `SHAFT_DARK_RATIO` | 0.35 | 0.25 to 0.45 |
   | `SHAFT_MAX_WIDTH_MM` | 20 | 15 to 30 |
   | `SHAFT_MIN_LENGTH_MM` | 25 | 15 to 50 |
   | `SHAFT_MIN_ASPECT` | 2.5 | 2 to 4 |
   | `SHAFT_MAX_GAP_MM` | 80 | 50 to 120 |
   | `SHAFT_MIN_ANGLE_DEG` | 35 | 25 to 45 |
   | `SHAFT_MAX_ANGLE_DEG` | 85 | 80 to 89 |
   | `HEEL_MIN_SHARE` | 0.33 | 0.2 to 0.8 |

   `ORIENT_PX_PER_MM` (1) and `SEARCH_REACH_MM` (150) are public with their reasons. The box's margin
   (1.5 mm) and the white's percentile (0.9) are private.
   - **The angle range is wide on purpose.** A real shaft rises behind a lofted face, so seen square to the
     face it draws at `tan(drawn) = tan(lie) · cos(loft)`. That is about 57° for a 62° lie behind a 34°
     face, and about 46° for a 64° lie behind a 60° lob wedge. Only the side and the sense of up are used,
     never the angle.
4. **The tests.**
   - **Where they are**: `crates/impact/tests/tape_orient.rs` holds 8 tests. `orient.rs` holds 3 unit tests:
     the transform against brute force, an opening that keeps a disc and drops its bar, and the shaft's
     rules one at a time. `mod.rs` holds 1, the hand from the two flags.
   - **The truth.** The test works each scene's orientation out from its hand and how the label was stuck
     (`truth` in `tape_orient.rs`), never from the reader.
   - **What is oriented, every one right**, with no hand and with the scene's own:
     - 12 face-on photos: 3 label types × 2 hands × logo up and down;
     - 6 photos tilted 28.5° to 30° and turned −90° to 180°, across both hands and both ways up.
   - **What the shaft measured.** Face-on, its angle was 61.1° to 62.3° against the drawn 62°, and tilted
     61.8° to 66.1°. Both ends lie within 2 mm of the line the scene drew. The visible length was 27.8 to
     104 mm. The width was 10.0 to 10.9 mm on the iron head, where the shaft alone is dark, and 15.0 to
     15.1 mm on the driver, where the dark hosel joins it.
   - **What is refused**:
     - a right-handed club given a left-handed golfer, both ways up: `HandDisagrees`;
     - `Shaft::Missing` on an iron and on a driver, the label on the mat with no club, and the label framed
       too tight at 160 mm face-on: `NoShaft`;
     - a hosel and shaft at both ends, on an iron and on a driver: `Ambiguous`;
     - a near-black background, behind an iron and a driver of each hand: `NoShaft` in all four. The test
       allows a right reading too, and fails only a flipped one.
   - **What `read_tape` answers**: each refusal is `no_orientation` carrying the found label, and an
     oriented label is `NeedsPick` with its orientation, whether or not a previous crop is passed.
   - The file takes about 14 s in a debug build, almost all of it drawing.
5. **The review images were checked by eye, on synthetic photos.** The verb, run over the 33 photos the
   tests drew (`TAPE_SCENE_DIR`), gave 23 `needs_pick` and 10 `no_orientation`, as the tests expect. The two photos of a right-handed
   club given a left-handed golfer are among the 23, because the desk passes no hand. Four
   `photo.png` reviews were looked at: a right-handed iron logo-down, a left-handed iron tilted and turned
   65°, a left-handed driver logo-down turned 180°, and a left-handed hybrid tilted 30°. On each, the toe
   arrow points to the end away from the hosel, and the top-line arrow toward the edge the hosel rises
   from. The section's run over P3's photos moved to P29 (P3 finding 10).
6. **P3's flat photo, as a smoke test.** All three labels are found and refused as `no_orientation`, which
   is the right answer for a photo with no club in it. A release read takes 0.30 s, decode included, where
   P9 measured 0.28 s, so the orientation costs about 20 ms. The reviews went to the session scratchpad,
   and nothing was written under `data/`.
7. **For P11**: replace the `Ok((orientation, _))` arm in `read_tape`, which answers `NeedsPick`, with the
   marks. Orientation runs before the marks, so a `no_orientation` refuses before any mark is looked for (P8
   finding 3). `read_tape` drops the `Shaft` as it drops the `OutlineFit`.
8. **For P13**: the set must frame the hosel and some shaft, or every photo is `no_orientation`.
   - **Aiming.** `Camera::aimed(pitch, yaw, roll, distance, aim_mm)` puts a plane point at the photo's
     centre. `tape_orient.rs`'s `framed` aims at `[±12, 15]` mm toward the heel at 200 mm. That keeps the
     whole label and at least 32 mm of shaft in the 800 × 600 photo, except where yaw and roll swing the
     shaft toward the photo's edge. Pose 2 (yaw −30°, roll −40°) needed `[−22, 26]`, and showed 27.8 mm.
   - **Reuse `truth` and `framed`** from `tape_orient.rs` rather than writing them again.
9. **The renderer** (`crates/impact/tests/tape_scene/mod.rs`) gained:
   - `Hand` is now `impact::tape::Handedness`, re-exported, so a scene's hand is what a test passes to the
     reader;
   - `Scene::shaft`: `AtHeel` (the default), `Missing` (no hosel or shaft), or `AtBothEnds`;
   - `Scene::background`: `MAT` by default;
   - `Camera::aimed`.
10. **For P18**:
    - `contracts::golfer::Golfer::handedness` is required, so the facade always has a hand and passes
      `Some`. Map it with a two-arm `match`: `impact` cannot see `contracts`, and `Handedness::as_str` spells
      the same wire names.
    - The window is about 140k samples, a few MB with its masks and two distance grids. That is small beside
      the finder's 75 MB (P9 finding 13).
11. **For P22**: the tape camera's frame must ask for the whole label **and** the hosel with some shaft. At
    least 25 mm of dark shaft must show beyond it. A photo framed on the label alone is refused (finding 4's
    "framed too tight").
12. **For P29**:
    - every tunable in finding 3, with its bracket;
    - chrome shafts, which may not be dark (P9 finding 9). A refusal is the answer there, until a ferrule or
      hosel is shown to stand in;
    - backgrounds dark enough to hide a shaft;
    - labels stuck crooked;
    - the real loft and camera geometry against the angle range.
    - **A wrong orientation shows on a stored crop** as the logo on the wrong side of the arrows (P3 finding
      10's note).
    - **A `--hand` flag for the desk tool.** If P29 wants the cross-check there, it goes in
      `parse_read_tape` (`crates/lab/src/bin/golf_lab.rs`) and the closure in `read_tape_photos`.
13. **For P30**:
    - ADR-037 line 157 and C5 name the three-argument `read_tape`. The reader now takes the golfer's hand as
      well (finding 2).
    - `impact::tape` now orients the label. P9 finding 14 already names `docs/ARCHITECTURE.md` and
      `CLAUDE.md`'s `crates/impact` sentence.

### P11 (2026-11-12)

1. **The mark finder is built**: `impact::tape::find_marks(crop, label) -> Result<Vec<Spot>, MarkMiss>`, in
   `crates/impact/src/tape/mark.rs`, and `read_tape` calls it after the orientation. The module doc gives the
   steps:
   - **the interior**: the outline less a `MARK_EDGE_MARGIN_MM` rim, where the face may show;
   - **the white**: a plane over the crop, fitted robustly to a `WHITE_BAND_MM` band just inside the rim;
   - **dark**: a darkening `(white − luminance) / white` of at least `MARK_MIN_CONTRAST`;
   - **wide**: opened by a disc `MARK_OPEN_RADIUS_MM` in radius, which strips the printing;
   - **spots**: runs of at least `MARK_MIN_AREA_MM2`, each centroid weighted by darkening;
   - **clipped**: a spot reaching the rim.

   With no previous crop, `read_tape` reads one whole spot as `Read { new_label: true }`. One clipped spot is
   `mark_clipped`, and none is `no_new_mark`. Two or more spots need a pick, and so does one spot longer than
   `MARK_MAX_ELONGATION`. `MarkMiss::NoWhite` (a band too dark or too mixed to give a white) is also
   `NeedsPick`, because that is a label found and squared whose mark cannot be placed (C5).
2. **What P11 settled that the plan left open.** None of these touches a decision.
   - **The printing is told apart by shape.** Every printed stroke is thin, and a 4 mm disc fits in none of
     them. Colour was not used, because the mark's colour is unknown. A mask of the printing was not used
     either: its layout is not in `LabelType` (P4 finding 9), and the mask would cut a hole where a mark
     covers printing. The module doc gives the reasons.
   - **The cross is not the anchor** (P9 finding 10). A centred strike covers the cross, so the anchor would
     fail on the strikes the drill is for, and one pattern would mix two frames. The 0.9 mm was measured on
     white paper, where the edge is faint. The module doc says so, and P29 measures it.
   - **An elongation gate the section did not name.** A lone spot longer than 1.3 (`√(λ₁/λ₂)`) is taken for
     two marks run together and needs a pick, rather than being read halfway between them (rule 6). Its known
     cost is that a pair closer than about 0.8 radii reads as one mark at their middle, at most 0.42 radii
     from each. A test pins that.
   - **With a previous crop, a marked label needs a pick until P12.** A spot there may be the last strike's.
     A label with no spot is `no_new_mark` either way. This placeholder is the same kind as P10's.
   - **`TapeMark` gained `area_mm2`** (P8 finding 6's invitation). No strike's size is known, so the reader
     reports each. `golf-lab read-tape` prints it as `area_mm2`, and the review's ring is now the disc of that
     area. `MARK_RING_MM` gave way to `mark_ring_px`.
   - **A mark mostly off the label is a speck.** What lies inside the rim is under `MARK_MIN_AREA_MM2`, so the
     reader answers `no_new_mark`, not `mark_clipped`. "Missed the tape" then gives the same verdict, outside
     every circle, because that mark's centre is at or beyond the edge.
   - **Files beyond the Touch line**:
     - `crates/impact/src/tape/orient.rs`: `opening` is split out of `thin_parts` and is `pub(super)`, as is
       `squared_distance`. The marks and the shaft now share one distance transform.
     - `crates/impact/tests/tape_scene/mod.rs`: `truth` and `framed` moved here from `tape_orient.rs`, and the
       six tilted poses became `tilted_poses()`, with `Pose::scene`.
     - `crates/impact/tests/tape_orient.rs` now uses those. Its last test, P10's placeholder ("`NeedsPick`
       until P11"), is gone, and `tape_mark.rs` covers what `read_tape` answers for an oriented label.
     - `crates/lab/src/read_tape.rs`: `area_mm2`, as above.
3. **The tunables**, each with its doc and the bracket P29 may move it within:

   | Constant | Value | Bracket |
   |---|---|---|
   | `MARK_MIN_CONTRAST` | 0.2 | 0.1 to 0.35 |
   | `MARK_OPEN_RADIUS_MM` | 2.0 | 1.0 to 3.0 |
   | `MARK_MIN_AREA_MM2` | 30 | 15 to 100 |
   | `MARK_EDGE_MARGIN_MM` | 1.0 | 0.5 to 2.0 |
   | `MARK_MAX_ELONGATION` | 1.3 | 1.15 to 1.8 |

   `WHITE_BAND_MM` (3) is public. These are private, each with its reason:
   - `CLIP_REACH_MM` (0.5);
   - the white fit's settings: 5 rounds, keep within 3 spreads, a least spread of 1 level, at most 0.25 of the
     band brighter than the white, and a least white of 32 levels.
4. **The tests.**
   - **Where they are**: `crates/impact/tests/tape_mark.rs` holds 9 tests, and `mark.rs` 5 unit tests (the
     interior, the white through stripes and a gradient, a mostly dark band refused, elongation, and the
     fresh-label rule). Every mark is a 6 mm-radius disc unless named.
   - **Read, all within 0.2 mm**:
     - 12 face-on scenes: 3 types × 2 hands × both ways up, with marks over the cross, the dashes, the iron's
       grid, the driver's text and the edge of the logo's letters. The worst was **0.089 mm**.
     - The 6 tilted poses. The worst was **0.059 mm**.
     - Each area came to 114.2 to 117.2 mm² for a 113.1 mm² disc. The largest is the hybrid mark touching the
       logo's letters.
   - **No mark found on 12 unmarked labels**, and none of the printing taken for one. They are face-on at 200
     mm, at 240 mm (about 4.2 px/mm, near the finder's least), and the 6 tilted poses. Each is `no_new_mark`.
   - **Refused or handed to a pick**:
     - `mark_clipped`: a mark 1 mm in from each type's bottom edge, and one at the iron label's end;
     - `NeedsPick`, two marks 24 mm apart: each spot lies within 0.2 mm of its mark;
     - `NeedsPick`, two 5 mm marks 8 mm apart: elongation 1.88 to 1.97;
     - read as one, two 5 mm marks 3 mm apart: the reading is their middle, within 0.2 mm;
     - `no_new_mark`, a 2.5 mm speck;
     - with a previous crop: `NeedsPick` when marked, and `no_new_mark` when not;
     - `no_orientation`, a marked label with no shaft, before any mark is looked for.
   - **Speed.** The file takes about 21 s in a debug build, almost all of it drawing.
5. **The verb over the drawn photos.** All 45 photos the tests drew (`TAPE_SCENE_DIR`) went through `golf-lab
   read-tape --review`, into the session scratchpad. Each outcome is as its test expects. Four reviews were
   looked at: a left-handed driver logo-down, a hybrid mark over the logo's letters, a tilted iron, and the
   clipped iron end. On each, the ring sits on the mark at its size, and the arrows match the scene's hand
   and turn. P3's flat photo is still `no_orientation` on all three types, since no mark is looked for on
   a label with no club. A release read of an 800 × 600 photo takes 0.12 to 0.14 s, marks included. Nothing
   was written under `data/`. The run over P3's first-strike photos is P29's (P3 finding 10).
6. **For P12**:
   - Replace the `if previous.is_some()` block in `read_tape`. `on_a_fresh_label` is the single-mark rule for a
     label the diff finds new.
   - `mark.rs`'s `Interior`, `White`, `darkening` and `spots` are `pub(super)`. A change field can be handed to
     `spots` in place of the darkening, which brings the opening, the area floor, the centroid and the clip
     test along with it.
   - Both crops share one `CropGrid` (P7 finding 6), so the current crop's `Interior` serves both. The previous
     crop arrives as a bare `Frame` (P8 finding 5), and its outside pixels are black, which reads as dark.
     Leave them out.
   - P9's synthetic marks, 2.5 to 3.5 mm in radius, are specks to this reader. A whole mark needs about 3.1
     mm of radius for `MARK_MIN_AREA_MM2`. Draw them 4.5 mm or more.
7. **For P13**:
   - `truth`, `framed`, `tilted_poses` and `Pose` are in `tape_scene`.
   - Keep whole marks at least 1.5 mm plus their radius inside the outline, or they come out `mark_clipped`.
   - With no previous crop, an overlap closer than about 0.8 radii reads as one mark (finding 2). The set's
     overlap photo is P12's diff to judge, not this rule.
8. **For P18**:
   - C8's `TapeStrike` has no area, so `TapeMark::area_mm2` is the reader's alone unless P18 stores it.
   - A pending pick can come from `MarkMiss::NoWhite` as well as from two marks.
9. **For P29**:
   - every tunable in finding 3, with its bracket;
   - the real mark's colour, size (each JSON line's `area_mm2`) and shape (the elongation gate);
   - marks speckled by the ball's dimples, which may break into pieces narrower than the disc;
   - shadows across the label, which a plane cannot follow;
   - whether the darkening weights pull a translucent mark's centroid toward the printing under it;
   - the cross's offset from the fit (finding 2).
10. **For P30**:
    - `impact::tape` now finds marks, and P9 finding 14 names the docs to update.
    - The desk tool's JSON line has a new key, `area_mm2`, in each mark.

### P12 (2026-11-13)

1. **The diff is built**: `impact::tape::diff_marks(crop, previous, label) -> Result<Change, DiffMiss>`, in
   `crates/impact/src/tape/diff.rs`. `read_tape` calls it whenever a previous crop is given and the label
   holds a spot, and it measures the photo's crop once for both. The module doc gives the steps:
   - **each crop measured by `mark.rs` alone**: its interior, its own white, its darkening and its spots, so
     two photos lit differently compare;
   - **the alignment**: every whole-pixel translation within `DIFF_SEARCH_MM` (the plan's ±2 mm), coarse to
     fine over a three-level pyramid. Each is scored by the mean darkening difference over the pixels where
     neither crop is dark and wide, so it aligns on the printing and never on the marks;
   - **the change**: each pixel's darkening less the previous crop's at the aligned pixel. A pixel is newly
     dark at `MARK_MIN_CONTRAST`;
   - **what each spot holds** (`Holds`): `New` with at least `PART_MIN_AREA_MM2` newly dark, `Old` with as
     much on a previous spot, `Both` for an overlap, and `Unclear` for neither;
   - **which label**: a previous mark is still here when half of it lies on what is dark and wide here. None
     still here means a new label, read by P11's single-mark rule with `new_label`.

   The outcomes (`Change::newest`):
   - **a new label**: P11's rule, so two marks on it need a pick (P2 finding 3);
   - **a pick**: some old marks gone and some not, an overlap, or an unclear spot;
   - **on the same label**: none new is `no_new_mark`, and a new spot cut by the edge is `mark_clipped`, even
     beside a whole one. A long new spot, or more than two, needs a pick. One or two are read with
     `new_label: false`.

   A previous crop that cannot be compared leaves a marked label to a pick. `DiffMiss` says why: `OtherGrid`,
   `NoWhite` or `Unaligned`.
2. **What P12 settled that the plan left open.** None of these touches a decision.
   - **The change picks the new spots, and the spot gives the reading.** The section names the change's
     components as the new marks. Here a new mark is read where its own spot lies in this crop, exactly as
     on a fresh label, for two reasons:
     - the alignment is to a whole pixel, and would carry its error into every reading;
     - a mark is lighter than the printing it covers, so the change has a hole along each stroke under a new
       mark.

     P11's 0.2 mm bound therefore holds unchanged.
   - **Two guards on the alignment, beyond the section's.**
     - A best shift on the search's edge is refused, since the cost may still be falling past it.
     - A best shift the printing did not pick out is refused. That is a top-level cost more than
       `ALIGN_MAX_SHARE` (0.7) of the median over the reach.
   - **A new label is not called where a spot stands on an old mark's place** (`PLACE_TAKEN_MM`, 4 mm, in
     each crop's own printed frame). The alignment's unit test found the hole this closes:
     - **The case**: rows 28 px apart, with the true shift 24 px beyond the reach, lined up one row at a
       false shift of 4 px. Its share, 0.48, passes the share guard.
     - **Why it matters**: an alignment wrong by more than about 2.6 mm leaves even the smallest old mark
       (30 mm²) under half on itself. Every old mark then reads as gone, and the label is called new. A lone
       old mark beside a missed strike would then be read as a fresh label's first strike.
     - **What the veto leaves**: an alignment wrong by less keeps each old mark `Old` or `Both`, never
       `New`.
   - **A clipped new mark refuses the photo, even beside a whole one.** Reading the whole one alone would leave
     the clipped strike uncounted and unrecorded. Refusing loses both, but says why.
   - **The previous crop's outside is its pure black pixels**, P8 finding 5's first option. C5's
     `Option<&Frame>` and C16's raw RGB stay as they are. A real pure-black pixel on the label is only left
     out of the diff.
   - **`TapeOutcome` gained no field.** The `Change` is public, for the tests and for P29's look at real
     pairs.
   - **Files beyond the Touch line**:
     - `crates/impact/src/tape/mark.rs`: `spots` is split into two parts, and keeps its signature:
       - `wide_dark`, the opened mask, which the alignment leaves out;
       - `spot_pixels`, each spot with its pixels.
     - `crates/impact/tests/tape_scene/mod.rs`: `bottom_at` moved here from `tape_mark.rs`.
     - `crates/impact/tests/tape_mark.rs`: P11's placeholder test ("with a previous crop, a marked label
       needs a pick until P12") is gone, and `tape_diff.rs` covers what replaced it.
     - `crates/lab/src/read_tape.rs`: the reference rule (finding 3).
3. **The desk tool's reference now advances on `mark_clipped`.** P8 finding 5 left this rule to P12 and P18.
   - **Why**: under P8's rule every refusal left the reference where it was. A clipped mark would then show
     as new on every later photo, and each of those photos would be refused `mark_clipped` in turn.
   - **The new rule**: the verb advances the reference on `Read`, `NeedsPick` and `mark_clipped`, every
     outcome whose new marks are accounted for.
   - **Where it stays**: on `no_label`, `no_orientation`, `no_new_mark` and `decode_failed`, because a
     retake helps each of those. The retake is read against the same reference, so it still shows the
     strike. If the golfer hits again instead, the unplaced mark shows beside the next one, and both are
     read (C10).
   - **Where it is written down**: the module doc and the sequence test
     (`a_sequence_reads_each_photo_against_the_last_accounted_strikes_crop`).
4. **The tunables**, each with its doc and the bracket P29 may move it within:

   | Constant | Value | Bracket |
   |---|---|---|
   | `DIFF_SEARCH_MM` | 2.0 | 1 to 4 |
   | `PART_MIN_AREA_MM2` | 12 | 5 to 30 |
   | `ALIGN_MAX_SHARE` | 0.7 | 0.3 to 0.9 |

   These are private, each with its reason:
   - `PLACE_TAKEN_MM`: twice the search;
   - `OLD_PRESENT_SHARE`: 0.5;
   - `ALIGN_MIN_OVERLAP`: 0.5;
   - `PYRAMID_STEPS`: 2, so the top level is at 2.5 px/mm;
   - `REFINE_RADIUS`: 2.
5. **The tests.**
   - **Where they are**: `crates/impact/tests/tape_diff.rs` holds 10 tests, and `diff.rs` 7 unit tests. The
     unit tests cover the outcome rules, the new-label veto, the alignment on drawn strokes, and another
     grid.
   - **The marks**: every mark is 5 mm in radius, and each label's six places are 12.6 mm or more apart. The
     photos come from six poses: up to 12° off square, turned up to 10°, at 190 to 210 mm.
   - **A run per label type**: six strikes with a photo after each, every photo read against the last one's
     crop.
     - All 18 readings are within 0.2 mm, the worst **0.088 mm**.
     - The first photo reads `new_label: true` and every later one `false`. Each later photo holds one `New`
       spot, and every old mark is still there.
   - **The alignment on the runs**:
     - the two fits disagreed at the cross by at most 0.42 crop px (0.04 mm), so every shift found was
       `[0, 0]`;
     - the residual was 0.014 to 0.021;
     - the share was 0.07 to 0.15.
   - **Two new marks (a photo missed)** are read on all three types, the worst 0.178 mm on the iron. Three
     new marks need a pick.
   - **An overlap**: a new mark half over an old one, or just touching it, needs a pick on every type, with a
     `Both` spot. Its new part was 42 or 72 mm², and its old part 80.
   - **Nothing new**:
     - a retake from another pose is `no_new_mark`, and so is a photo against its own crop (shift `[0, 0]`);
     - every spot is `Old`;
     - an old mark's newly dark rim was at most 0.06 mm².
   - **A fresh label after a used one**, stuck the other way up:
     - one mark reads `new_label: true`, with every old mark gone;
     - two marks need a pick;
     - one mark 1 mm from an old mark's place also needs a pick. That old mark counts as still there and the
       others as gone, and its crescent was 9.0 to 10.6 mm² newly dark, under the floor.
   - **A new mark cut by the bottom edge** is `mark_clipped` on every type.
   - **A moved reference**:
     - moved by `[13, −7]` px, it is aligned at exactly that much more, and the photo reads the same;
     - moved 2.5 mm, beyond the reach, its best shift lies on the edge (share 0.49). That is `Unaligned`,
       so the photo needs a pick.
   - **A reference whose top fifth is black** (outside) reads the same. A reference of another label type is
     `OtherGrid`, so the photo needs a pick.
   - **Speed**:
     - **Release**: the file takes about 7 s, almost all of it drawing.
     - **Debug**: it took 27 s on a quiet machine and 68 s while another build ran, against P11's file at
       21 s.
     - **One diff**: about 2 s in debug, about half of it the two crops' measurements and half the
       alignment.
     - **What was done about it**: `read_tape` measures the photo's crop once. The run test is split into
       one test per label type, so the three run side by side.
6. **A mark tangent to the printed red ring reads up to 0.18 mm off.** That is P11's reader, met here, not
   the diff.
   - **The case**: the iron's run first had a mark at `[1, 4]`. Its far edge lay 9.12 mm from the cross, on
     the ring (9.15 ± 0.3 mm). It read 0.178 mm off, against at most 0.088 mm for every other mark.
   - **Why**: the ring's stroke along the mark's edge survives the opening and is darker than the mark, so it
     pulls the centroid outward.
   - **What was done**: the run's place moved to `[1, 3]`. The skipped-photo test still holds that mark near
     the ring, at 0.178 mm, inside the bound.
7. **The verb over the drawn sequences.** All 65 photos the tests drew went through `golf-lab read-tape
   --sequence --review`, with no hand, into the session scratchpad. Each label type had ten sequences: the
   run, two missed-photo pairs, two overlaps, nothing new, three fresh labels and a clipped mark. Every line
   came out as its test expects:
   - each run is six `read`s, the first with `new_label: true`, each within 0.07 mm;
   - the two-new photo is `read` with two marks;
   - three new, each overlap, two marks on a fresh label, and a fresh mark on an old place are each
     `needs_pick`;
   - nothing new is `no_new_mark`, and the cut mark is `mark_clipped`.

   Each sequence's first photo, holding two or three marks with no reference, is `needs_pick` by P11's rule,
   and becomes the next photo's reference as finding 3 says. Two reviews were looked at:
   - the iron's two-new crop rings the two new marks and neither old one;
   - the fresh hybrid's photo has its arrows toward a left-handed club's toe and top line.

   Nothing was written under `data/`.
8. **For P13**:
   - `tape_diff.rs`'s `run`, `POSES`, `scene`, `reads` and `moved` are shapes a synthetic set can reuse, and
     `bottom_at` is in `tape_scene`.
   - The drawn photos of one label are fitted within 0.05 mm of each other, so nothing drawn exercises the
     alignment but `moved`. A set that wants the search exercised has to move the reference itself.
   - Keep marks off tangency with the red ring (finding 6), and 12.6 mm or more apart for 5 mm marks.
   - The renderer lays a mark over a mark in the same colour, so a re-strike over an old mark darkens
     nothing. A strike within about 1.2 mm of an old mark's centre is `no_new_mark`. That is the diff's
     known limit, not a wrong reading.
   - A debug build draws a photo in about 0.45 s and diffs one in about 2 s (finding 5). A set of many
     sequences should split its tests so that they run side by side.
   - **For the user, not a phase**: a `[profile.dev.package.impact] opt-level` line in the workspace
     `Cargo.toml` would cut every tape test's debug time several-fold. P12 left that shared file alone,
     since M38's Mac work edits it too.
9. **For P18**:
   - **Advance `last/<club>.rgb` as the desk tool now does** (finding 3):
     - on `Read`;
     - on `NeedsPick`, whatever becomes of the pending strike, since its mark is on the label either way;
     - on `mark_clipped`.

     Leave it on `no_label`, `no_orientation`, `no_new_mark` and `decode_failed`, so that a retake still
     sees the strike.
   - **Write the crop's frame as `rectify` left it.** Its outside pixels must stay pure black, because that
     is how the diff knows them.
   - **A `Read` may hold two marks**, and each is a `TapeStrike` (C10).
   - **A `no_new_mark` pending may be a re-strike within about 1.2 mm of an old mark.** C15's pick of it
     covers that case, though its sentence advises a retake, which will not help there.
   - **A pick can now come from `DiffMiss` too**: a reference of another grid, one with no white, or one that
     could not be aligned. The `NeedsPick` that follows advances the reference, so the next photo diffs
     against a good crop.
10. **For P29**:
    - the three tunables in finding 4, with their brackets;
    - the real fits' disagreement (each pair's `shift_px`), and each pair's `residual` and `match_share`;
    - whether a re-strike darkens an old mark;
    - how much a mark's look changes between two photos: its newly dark rim, which was 0.06 mm² here;
    - a turn or a scale between two fits, which a translation does not follow;
    - marks tangent to the printed ring (finding 6).
11. **For P30**:
    - `impact::tape` now diffs a used label (`diff_marks`), and P9 finding 14 names the docs to update.
    - `golf-lab read-tape --sequence` advances its reference on `mark_clipped` too (finding 3).
12. **For P21: one `golf-lab` test fails on Windows, before P12 and apart from it.**
    - **The test**: `crates/lab/tests/import_export.rs`'s `a_file_no_golfers_store_lists_still_goes_by_bytes`,
      last changed in the M38 checkpoint commit (`0b88173`).
    - **Why it fails**: its expected conflict lines join `old/aaron.bag.json` onto a Windows path, which
      keeps the forward slash. The verb prints `golfers\old\aaron.bag.json`.
    - **What it is not**: P12 touched nothing on that path. `cargo test -p golf-lab --no-fail-fast` passes
      every other test, `read_tape`'s included.
    - **Who fixes it**: P21 edits `import-export` and runs the workspace, so the test's expected string is
      P21's to fix, or the user's call if the Mac's output is the one meant.
