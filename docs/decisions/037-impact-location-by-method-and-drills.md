# ADR-037: Where on the face, by method — one question, many methods, and the drill framework they sit in

## Status
**Accepted** 2026-11-12 — M46 P1 ([the plan](../plans/m46-impact-drill.md)). It is written from the plan's
decisions (D1 to D16) and calls (C1 to C24), which the user approved as drafted at the plan's P0 after a
five-round interview. The plan's phase findings are the as-built record, and where the build corrects
a clause below, an addendum here says so.

**Nothing here is built.** M46 P2 to P31 build it. **It amends no ADR.** It applies
[ADR-034](034-shot-first-phone-first.md) §3 (a drill is not tracked) and that ADR's M37 addendum (strike
location is graded only from a measured offset) as they stand. It is the pattern
[ROADMAP §M46](../../ROADMAP.md#m46-impact-drill--impact-tape-first) asked to have written down, so that
M44 and M45 inherit it rather than argue it again.

## Date
2026-11-12

## Context

**Where on the face is the one shot fact nothing here measures.** `ShotData` has carried
`impact_offset_h` and `impact_offset_v` since M32, in millimetres with + toe and + high
(`crates/contracts/src/shot.rs`). M37 grades strike location from them and from nothing else, because HD
Golf's printed `CENTER`/`HEEL`/`TOE` word is a category with no magnitude (ADR-034's M37 addendum). HD
Golf prints neither field. Full Swing's overhead replay declares the first, but M42's reader of it
(`crates/impact`) read none of the clips it was built on, and from overhead it can never see the second
(ADR-034's M42 addendum). So both fields are empty on every stored shot, and strike location is absent on
real data.

**The user's tape can measure it with no launch monitor.** The user always practises with Strike Tape
LLC's STRIKEPRO labels. A pack holds driver, iron and hybrid labels, each good for about 6 to 10 hits, and
a strike leaves a mark on the label where the ball met the face. A photo of the label on the club is
evidence of where the strike landed, if a reader can place the mark in millimetres.

**A drill has nowhere to keep its history.** A drill is untracked by ADR-034 §3. That is derived from the
practice mode (`PracticeMode::is_tracked`, `crates/contracts/src/intent.rs`) and never stored, and the
corpus leaves a drill out as `untracked` (ADR-024's M35 addendum). That is right for the corpus: the
user's words were "some drills we do not want to add to the player stats". It also means an impact
drill's strikes cannot live in the shot corpus, so a drill's progress needs a history of its own.

**More methods and more drills are coming.** The user's call (2026-11-09, ROADMAP §M46) was one interface
for *where on the face*, with one implementation per **method**. Tape is the first, and foot spray, a
dry-erase marker and a launch monitor's printed offsets are named candidates. M44 asks the same of *where
the club bottomed out* (foot powder first), and M45 asks it of *which club this is*. Writing the pattern
once is cheaper than having three plans each work it out.

**The rules this lands under.**
- [ADR-010](010-benchmark-ranges.md) §2: a missing range yields no score rather than a wrong one, which
  this repo reads as *no score beats a wrong one*. A mark that cannot be placed is not read, and it is
  never the centre.
- ADR-034 §3, above. Since M37 it has had a consequence it did not have when written: a drill's strike
  location never reaches the grade.
- [ADR-035](035-rust-everywhere-python-where-required.md) clauses 1 and 4: new work is Rust, and the frozen
  Python gains nothing.
- [ADR-022](022-learned-artifacts-as-committed-data.md): what ships as data carries where it came from.
- [The M38 plan](../plans/m38-iphone-app.md)'s rules 7 to 10: Dart holds layout and no golf, and Rust
  hands it every string a golfer reads.

## Options Considered

Three questions had real alternatives. ROADMAP §M46 raised each of them before the interview. The other
calls are recorded under Decision with their reasons.

### Where the face's centre is

#### Option A: Find it from the grooves
- **Pros**: application error drops out. The grooves are fixed to the face, so a label stuck on off
  centre would not move the origin.
- **Cons**: the label covers the grooves where it sits, which is exactly where the centre is wanted.
  Driver and fairway faces carry few scorelines or none, so the method would differ by club, for a gain
  on irons alone. It would also be a second detector with no truth
  to tune it on. **Declined by the user** (D2).

#### Option B: The label's own centre, in the label's own millimetres *(chosen)*
- **Pros**: one detection gives both the origin and the scale. The label's outline is a known shape of
  known size (C3), so a found label is already a ruler.
- **Cons**: **application error lands in every reading, unmeasured.** A label stuck 4 mm toeward reads
  every strike 4 mm heelward of where it was. The user chose this knowing it: *"let's not worry about that
  for now"*.

### How many photos

#### Option A: One photo per label, after its 6 to 10 hits
- **Pros**: a sixth to a tenth of the photos.
- **Cons**: the order is lost. The ladder judges blocks of strikes in order, and differential practice
  judges each strike against the call made for it, so every strike needs its own position at its own time.
  Marks overlap on a well-used label, and telling overlapping marks apart needs to know which came last. A
  block of 10 would also span labels.

#### Option B: One photo per strike, and the newest mark *(chosen)*
- **Pros**: each strike's position at its own time. Overlap becomes a diff, the photo against the last
  strike's crop, and the diff's leftover is the new mark.
- **Cons**: a photo after every strike, which is the user's call (D4). There is never a photo of a clean
  label, so a label with one mark is taken to be new.

### Placing the mark

#### Option A: The user taps every mark, with no reader
- **Pros**: no reader to build or measure, and a person is a good detector of a mark.
- **Cons**: a chore on every strike, and the tap is a declaration rather than a measurement, so nothing
  would ever be read. **Declined by the user**, who made the tap the last resort: *"a doomsday task where
  we have utterly failed"* (D4).

#### Option B: The photo is read, and the tap is kept for when the reader cannot tell *(chosen)*
- **Pros**: most strikes cost the user nothing past the photo, and the reader's error can be measured
  (clause 3).
- **Cons**: a reader to build, std only, and a truth set to measure it on (M46 P2, P3 and P13).

## Decision

The clauses are numbered so that the plan's phases, M44's plan and later addenda can cite them.

### 1. One question, many methods, and each method declares its axes

`contracts::face_impact` (M46 P5) answers *where on the face*, once, for every method (C2):

| Shape | What it is |
|---|---|
| `ImpactMethod` | `OverheadReplay` (M42's reader) and `StrikeTape` (M46's) |
| `ImpactAxes { heel_toe, high_low }` | what a method can see, from `ImpactMethod::axes()`: the replay `{true, false}`, the tape `{true, true}` |
| `FaceImpact` | `offset_h_mm`, `offset_v_mm` (each an `Option`), `method` and `method_version`, in `ShotData`'s units: millimetres, + toe, + high |

- **A method declares which axes it can see**, and `Validate` refuses a value on an axis it cannot. That is
  why the replay never writes `impact_offset_v`, now stated as a rule rather than left as one reader's
  habit.
- **A method later is a new variant.** `axes()` is a `match`, so the new variant fails to compile until
  its axes are declared.
- **Every reading carries the version of the reader that made it.** The tape's is `TAPE_READER_VERSION`,
  a new key (P5), bumped by any change to what the tape reader produces after P13. The replay reader has
  no version key (ADR-034's M42 addendum). Nothing in M46 makes a `FaceImpact` from a replay, so that gap
  is left for whichever milestone first does, and is named here so that milestone finds it.
- **`ShotData` does not change, and nothing turns a `FaceImpact` into a shot in M46** (clause 6).
- **Where a method lives.** The tape reader is `impact::tape`, a second method in `crates/impact` beside
  the replay (C1). It reuses that crate's `frame`, `mask` and `component`, and it takes no dependency: a
  photo is decoded at the edges, by Swift on the phone and by `golf-lab`'s decoder on the desk. It does
  not reuse `crates/ocr`'s screen finder, which works on `image`'s types inside the one crate allowed to
  reach `ort` (`crates/ocr/tests/ort_reach.rs`). So the tape writes its own 4-point homography and warp,
  and `docs/REFACTOR_LEDGER.md` records the near-duplicate (P7).

### 2. The tape's frame: the label, in its own millimetres

- **STRIKEPRO only, read as that exact product** (D1). Another tape is a new label type or a new method,
  never a change to this one.
- **Face centre is the label's centre, on every club** (D2, Option B above). A reading is the mark's
  position in the label's own frame.
- **The label geometry is data** (C3): `crates/impact/tape_labels.json`, read by `include_str!` as
  `contracts::catalogue` reads its rows. Each label type holds its outline, its centre and its inscribed
  radius, and a `category_labels` map gives each `ClubCategory` its label, or none where no label fits.
  The maker's site gives no dimensions (D1), so the user measures them (P3), and each number names its
  tool and its measurer, as ADR-022 asks of committed data.
- **The axes** (C4). +x is toward the toe and +y toward the top line, whichever hand the club is. **The
  club gives the orientation**: the hosel and shaft leave the face at the heel and run up. The golfer's
  handedness is a cross-check and not a source. A disagreement, or no shaft found, refuses as
  `no_orientation`, because a guessed side is a guessed sign.

### 3. One photo per strike, and what the reader may answer

The reader is `read_tape(photo, previous, label)` (C5). `previous` is the club's last crop, held as raw RGB
rather than JPEG because a JPEG's artefacts would show up in a diff as marks (C16). It answers one of
three ways:

- **`Read`**: the label's corners, the rectified crop, the orientation, whether the label is new, and **one
  or two** new marks in millimetres. Two new marks means a photo was missed, and both count (C10).
- **`NeedsPick`**: the label was found and squared, but the new mark cannot be told apart (an overlap, or
  more than two candidates). The user taps it (clause 4).
- **`NotRead(reason)`**: `no_label`, `no_orientation`, `no_new_mark`, `mark_clipped` (the mark touches the
  label's edge) or `decode_failed`. Each reason has its sentence and a `retake_helps` flag, which works as
  `UnscoredReason`'s `refilming_helps` does: the flag belongs to the reason, so nothing downstream infers
  it.

**The reader's error is measured, not assumed.** A printed mm grid (D7) holds the peeled labels in order,
photographed straight on, and that photo is the truth each on-club reading is compared to. The error is
reported per label type as P50 and P95 in millimetres (P13). **A wrong reading fails the gate** (C24): on
the truth set, a reading more than `WRONG_MM` from the truth on either axis is wrong, and one is enough
(`WRONG_MM` is provisional, 5 mm). A reader that is unsure must say so, by `NeedsPick` or a named
`NotRead`, and coverage is printed beside the error so that a reader that starts withholding is seen.

### 4. A declaration is not a reading

The user can place a strike in two ways (C6). Each is stored as what it is, never as a reading:

- **A pick.** The user taps the new mark on a `NeedsPick` crop, Rust converts the tap to millimetres, and
  the strike is stored with `source: picked`.
- **"Missed the tape".** It is offered on `no_new_mark`, and the strike is stored with `source:
  declared_off_tape` and no offsets.

`source` is `read`, `picked` or `declared_off_tape`. A strike that is not read stores no position and
names its reason in `not_read`. It is never placed at the centre, and it does not count toward a block.
A declared-off-tape strike counts as outside every circle. That holds only because no level's radius
exceeds the label's inscribed radius (clause 7's first cut), so a strike off the tape really is outside
the circle.

### 5. The drill framework: drill, topic, method

A drill is reached as **drill → topic → method**: the app's **Drills** page (D14), then **Impact**, then
**Tape: STRIKEPRO**, then a club. Its history sits under the data root, beside the corpus and never inside
it (C7):

```
drills/<player_id>/<topic>/<method>/
  strikes.jsonl          one strike per line, append-only
  state.json             per club: stage, level, the block in progress, a pending suggestion, level history
  crops/<strike_id>.jpg  the rectified label crop
  last/<club>.rgb        the club's last crop as raw RGB, the diff's reference
```

M46's is `drills/<player_id>/impact/strike_tape/`. Writes go through `storage`'s atomic write and the lock
the shot store holds.

- **A strike record keeps its inputs and derives its verdicts** (C8). `TapeStrike` stores the club, its
  category, the label and its place on it, the stage, the level's radius at the time, the call
  (differential practice only), the offsets, `source`, `not_read`, `reader_version` and the crop. **Inside
  or not is derived, never stored**, for the reason `PracticeMode::is_tracked` is: a stored verdict could
  disagree with its own numbers.
- **Strike ids cannot collide across phones**: `<install>-<N>`, with N from the state's counter. A label
  applied gets `<install>-L<N>`.
- **The image is kept small** (D6). The label is cut out, squared to its own frame and stored as a JPEG of
  tens of KB. The original photo is deleted once its strike, or its pending strike, is stored. A
  `no_label` photo is kept apart, as M38 keeps a photo it cannot read, because those are the photos that
  improve the reader (C16).
- **The lab gets it by export** (C17). The phone's export zips `drills/`, and `golf-lab import-export`
  merges it into `data/processed/drills/` by `strike_id`. A second import of the same zip changes nothing.

### 6. Drills are untracked, and every strike is kept

A drill strike is a drill's alone (D5, ADR-034 §3). **Nothing in M46 writes into `sessions/`, `shots/`, the
corpus, the baselines or the graded profile, and nothing reads a drill strike into them.**
`CENTERED_RADIUS_MM` (`crates/contracts/src/strike.rs`) does not move. Every strike is kept, positioned or
not, with its crop and its reader's version, so a later milestone can decide otherwise without asking the
golfer to hit them again (Consequences, below).

### 7. The ladder: a baseline, then blocks of 10, per club

The ladder is the drill's progress: a target circle around face centre that tightens as the golfer earns
it.

- **Per club, rolled up by category** (D10). Each club has its own stage and level, and the category view
  gathers its clubs' strikes and lists each club's level.
- **A table of radii per category** (D9, C9). The values are the user's and provisional, and are listed
  widest first. **Two cuts apply before a table ships**:
  - **trimmed** (P4): a radius above the category's label inscribed radius is removed, so a strike off the
    tape is outside every circle (clause 4);
  - **floored** (P13): a radius below twice the label type's measured P95 error is removed, so no level is
    tighter than the reader can resolve (ADR-010 §2).

  What ships is the cut tables, in `contracts::drill` (P6). The uncut tables are the plan's C9 and stay in
  that module's doc comment, with both cuts named. They are pointed at here rather than copied, because
  both cuts move them.
- **What counts** (C10). A strike counts when it is `read`, `picked` or `declared_off_tape`. A `NotRead`
  strike is stored and not counted, and so is a pending one. **Inside** means `hypot(h, v) ≤ radius`, as
  `CENTERED_RADIUS_MM`'s rule is.
- **The baseline** (C11): 10 counted strikes aimed at centre. The start level is **the smallest radius in
  the club's table holding at least 5 of the 10**. The baseline's R50 is shown either way.
- **Blocks of 10** (D8, the user's numbers): **7 or more inside moves the circle in a level, 3 or fewer
  moves it out, and anything else stays.** Changing stage mid-block abandons the block: its strikes stay
  stored, and it is never judged.

### 8. The pattern and the suggestions

- **The pattern** (C12) is taken over a club's last 20 positioned strikes in the baseline, ladder and
  recentre stages. Its **centre** is the mean `h` and `v`, each with its 95% interval
  (`analysis::stats::mean_ci`). **R50** is the median distance from face centre (`stats::percentile`),
  and **spread** is the median distance from the pattern's own centre. Differential strikes are aimed off
  centre on purpose and never enter it. **Below `RATE_MINIMUM_N` positioned strikes the pattern is absent,
  never estimated.**
- **Two suggestions, never the default** (D11, C13). The values are the user's and provisional:
  - **recentre**, when the pattern's centre is clearly off: on some axis, its 95% interval excludes 0
    **and** its mean is more than a third of the current radius from 0. A recentre block's goal is its own
    mean within a third of the radius, and meeting it suggests returning to the ladder at the same level;
  - **differential practice**, after two blocks in a row at 3 of 10 or fewer at the widest level, or after
    a baseline whose widest circle holds fewer than 5. A differential block of 7 or more calls made
    suggests a fresh baseline, and the ladder starts again from it.
- **The golfer accepts or declines.** An accepted stage is remembered for the club. A declined suggestion
  is offered again after each block for which it still holds, so "not now" is never remembered as "never".

Separating where the pattern sits from how big it is matters, because they are different faults with
different practice. A pattern can be tight and off centre, which calls for recentring, or centred and
wide, which calls for differential practice. The ladder's circle cannot tell those apart.

### 9. Differential judging

- **The calls are random** (D12): heel, centre or toe, drawn uniformly with an injected `Random`, as M38's
  challenge draws shapes. Dart draws the call and Rust judges and stores it (C19). The call
  changes only after a counted strike, so a `NotRead` keeps it.
- **Each call is judged against the club's current radius `R`** (C14), which is its last ladder level or its
  baseline's start level. A **centre** call is made when `hypot(h, v) ≤ R`, a **heel** call when `h ≤ −R`,
  and a **toe** call when `h ≥ +R`. A declared-off-tape strike makes no call. High and low are not called
  in M46.

Feeling both edges of the face is how differential practice teaches the centre. The call's judgment uses
the same circle as the ladder, so a golfer's "centre" means the same thing in both stages.

### 10. Where the ideas come from

The ladder is an adaptive staircase in the spirit of the challenge-point framework (Guadagnoli and Lee,
2004, *Challenge point: a framework for conceptualizing the effects of various practice conditions in
motor learning*, Journal of Motor Behavior), as Adam Young applies it to practice. Practice is kept near
the edge of what the golfer can do, by making it harder after success and easier after failure.
Separating a pattern's centre from its size, and learning the centre by feeling both edges (differential
practice), are also his emphasis. **No text, drill name or table is copied from his materials** (C22).
The code, the tables and the app's words are this repo's own, and the words are plain: "Differential
practice: hit the spot I call".

### 11. What this does not change

- `ShotData`, `ShotProvenance` and every shot already stored.
- `ANALYSIS_VERSION`, `SCREEN_PARSER_VERSION` and `CAREER_VERSION`, every committed vector under `spec/`,
  `devices.json` and `profiles.json`. The one new key is `TAPE_READER_VERSION`, and a phase that cannot
  pass without moving one of the others stops and asks.
- ADR-034 §3 and its M37 addendum's strike-location rule, which this applies as written, and
  `CENTERED_RADIUS_MM`.
- The frozen Python, which gains nothing.
- M42's replay reader, which M46 does not improve.

## Consequences

- **Easier**: where on the face is measured with no launch monitor, on any club a label fits, in the same
  units as the field the grade reads. A second method is a variant and an implementation, and its axes are
  a compile-time question. Every strike, its crop and its reader's version are kept, so a better reader
  later can re-read old strikes rather than throwing them away.
- **Harder**: a photo after every strike. Application error sits unmeasured inside every reading (D2), so
  a reading is accurate about the label and only as accurate about the face as the label was placed. The
  ladder's floor rests on the reader's measured P95, so a reader change that worsens the error moves what
  ships, and it bumps `TAPE_READER_VERSION`. The drill history is a second store beside the corpus, with
  its own lock discipline and its own export path.
- **The navigation** (D15, C18) is the M46 plan's alone. It replaces the app's tab bar with a golf ball and
  an arc of greens, and it decides nothing about drills.

### For M44: low point, in the same framework

M44 is the same pattern for a different question: *where did the club bottom out*, answered in
`ShotData`'s units, `low_point` in inches with + ahead of the ball.

- **It takes its own question and its own methods.** That is a shape of its own in `contracts` beside
  `face_impact`, with foot powder its first method and a method declaring what it can see. Powder shows
  the low point and never the angle of attack, so `attack_angle` is a quantity powder does not see, as
  `high_low` is an axis the replay does not see (ROADMAP §M44).
- **It inherits the framework**: `drills/<player_id>/low_point/foot_powder/` beside M46's directory, with
  the same files; the Drills page, where Low point joins Impact; and a record that keeps its inputs, its
  `source`, its `not_read` reason, its reader's version and its crop, deriving every verdict. It also
  inherits clauses 4 and 6 unchanged: a declaration is not a reading, and a drill is untracked and kept.
- **It does not inherit the ladder or the pattern by default.** Those are two-dimensional and radial, and
  a low point is one signed distance whose first question may be *before or after* the ball. Whether a
  ladder suits it, and in what units, is M44's plan's call.

### For M45: the method interface, applied to gear

M45 finds the player's clubs by several methods (typed, the catalogue, a model's lookup, a photo). That is
clause 1's pattern applied to a different question, with ADR-026's rule beside it: a lookup is a
proposal, and the golfer's confirmation makes it a declaration. M45 inherits the pattern and none of the
drill framework.

### What a later milestone needs before a drill strike may feed the strike-location grade

Each of these is a decision or a measurement that M46 does not make:

1. **A decision that a drill strike may count.** That is an addendum to ADR-034 §3, or tape worn during
   tracked practice rather than in a drill. The kept strikes allow either.
2. **A join from a strike to a shot.** The grade judges tracked shots in the corpus, and a taped strike has
   no `ShotData`. Pairing it with the same swing's launch-monitor photo, and a conversion from
   `FaceImpact` to `impact_offset_h` and `impact_offset_v` that records the method, are both unbuilt.
3. **The capability model taught a method that is not the device.** A `devices.json` field entry's `from`
   is `screen` or `impact_video`, and both are the device's own output (ADR-034's M42 addendum). A tape
   reading is printed by no device, so ADR-034 clause 2's "analysed on" needs a source that names the
   method.
4. **The error set against the grade's radius.** `CENTERED_RADIUS_MM` stayed provisional at M42 for want of
   ground truth, and P13's truth set is the first. Application error (D2) also needs measuring, because
   the grade's circle is about the face and a tape reading is about the label.
5. **The kept strikes re-read** under whatever `TAPE_READER_VERSION` is then current, which the crops make
   cheap.

## Deferred, by choice

These stay out of M46 (the plan's *Not in M46*): feeding strikes into the corpus, the profile or the
strike-location grade, and revising `CENTERED_RADIUS_MM`; any tape but STRIKEPRO, and the other methods
(foot spray, a dry-erase marker, a launch monitor's printed offsets); measuring application error; a
driver target away from face centre, such as high on the toe; high and low calls in differential
practice; re-reading stored crops when `TAPE_READER_VERSION` moves; M44's low point; a heat map at the
lab; improving M42's replay reader; and Android.
