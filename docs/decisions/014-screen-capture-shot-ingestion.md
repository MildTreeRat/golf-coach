# ADR-014: Shot Data Ingestion from Launch-Monitor Screen Captures

## Status
Accepted

## Date
2026-08-04

## Context
ADR-004 chose the Garmin R10 on the strength of one criterion: **programmatic data
extraction**. That decision still stands, but the R10 was never bought, and M3 has sat on
mock `ShotData` ever since. Meanwhile a launch monitor became available that the R10
decision never considered — an **HD Golf** simulator, already installed and hittable
today.

HD Golf inverts ADR-004's trade-off. The data quality is good and the hardware costs
nothing extra, but it has no export: no BLE stream, no documented API, no session CSV, no
community reverse-engineering effort of the kind that made the R10 attractive. The
metrics exist only as pixels on a `SHOT DATA` screen.

That screen turns out to be unusually well suited to reading, though — it is a tidy grid
of labelled tiles, and it prints **redundant** metrics (smash factor alongside both
speeds; shot distance alongside carry and bounce & roll) whose arithmetic only works out
if every value was read correctly.

So the question is not "R10 or HD Golf" — it is whether a shot source that *infers*
metrics from an image can be trusted enough to feed the analysis engine, and how to build
it so the R10 (or anything else) still drops in later without a rewrite.

The failure mode to design against is specific. OCR does not fail loudly: a dropped digit
turns a 128.1-yard carry into 28.1, which is a well-formed number that will quietly skew
every session average it lands in. A wrong shot is worse than a missing one.

## Options Considered

### Option A: Wait for the R10 (ADR-004 as written)
- **Pros**: Real-time, structured, no inference. The already-accepted plan.
- **Cons**: Requires the purchase that has not happened in four months, plus shipping and
  BLE reverse-engineering. Meanwhile a working launch monitor sits unused and M3 stays
  blocked. Does not use the hardware actually on hand.

### Option B: Vendor export from HD Golf
- **Pros**: Would be structured data with no inference step.
- **Cons**: No such export exists. Investigated and ruled out, not deferred.

### Option C: Manual entry
- **Pros**: Trivial, exact, no dependencies.
- **Cons**: ~14 numbers per shot, by hand, between shots. Guaranteed to be abandoned
  within one range session, which makes it not a solution.

### Option D: Vision-model parsing (send the photo to a multimodal LLM)
- **Pros**: Robust out of the box to the things that actually break OCR here — off-axis
  photos, ceiling glare, reflections, arbitrary rotation. Little preprocessing to write.
  The project already depends on `anthropic` and holds an API key for M6 coaching.
- **Cons**: Per-shot cost and a network round trip on something that should work in a
  garage. Makes an offline, self-contained pipeline depend on a paid external service for
  basic data ingestion.

### Option E: Local OCR + geometric parsing (chosen)
- **Pros**: Free and offline; no per-shot cost, no network, no API key. PaddleOCR installs
  from pip with no separate system binary. The tile grid is regular enough to parse
  geometrically, and the screen's redundant metrics make the parse self-checkable.
- **Cons**: The photos are the hard case for OCR — taken at an angle, under ceiling glare,
  with the room reflected across the tiles. Most of the work is preprocessing, and it needs
  tuning against real photos rather than being correct by construction.

## Decision
**Option E — local OCR**, behind a swappable recognizer port.

The deciding factor is that this is a home-lab project that should keep working with no
network and no account, and that the redundancy printed on the screen gives a genuine
correctness check that does not depend on trusting the OCR engine.

The cost is preprocessing, which is where the risk sits and where the tuning went:

| Problem in the real photos | Fix |
|---|---|
| Screen is a trapezoid (photographed from beside the monitor) | Detect the monitor's quadrilateral, warp it to a rectangle |
| 5712px phone photos shattered the contour | Run outline detection at a normalized 1000px, so the morphology kernel that closes the bezel edge is meaningfully sized |
| Orientation is not guaranteed (EXIF covers the camera roll, not stripped re-encodes or video frames) | Try all four rotations, keep the one the profile's own labels are found in — the content votes, not the metadata |
| Glare and reflections | CLAHE on the lightness channel: lifts contrast inside the dark tiles without amplifying the bright spots |

**Guessing is worse than declining.** When no convincing screen outline is found, the photo
is parsed uncropped and said so, rather than warped on a bad guess — a wrong quad turns the
tiles into nonsense.

### Trust model

Every parsed shot carries a `ShotProvenance`: the device profile, a 0–1 confidence, the raw
on-screen text per tile, and any warnings. Confidence blends three independent failure
modes — did we find the right screen (label coverage), did we find the values (value
coverage), and was the engine itself sure (mean OCR confidence) — because a clean read of
the wrong screen and an unreadable read of the right one are both untrustworthy for
different reasons.

Two cross-checks then test the parse against the screen's own arithmetic:

```
smash factor  ==  ball speed / club speed        (101.5 / 88.6 = 1.15)
shot distance ==  carry + bounce & roll          (128.1 + 23.4 = 151.5)
```

Both hold exactly on the reference photos, so a mismatch is evidence about the *parse*, not
about the shot. Either a failed check or low confidence sets `needs_review`.

This judges **fidelity, not plausibility**. One reference photo shows a 159.5 mph club
speed and a 0.89 smash factor — odd numbers, but the screen's own arithmetic checks out, so
the parse passes. Flagging it would train the reviewer to ignore flags.

### Structure

`ShotDataSource` (ADR-007) is unchanged; this is a third adapter behind it, joined by
`CompositeShotDataSource` so screen captures, mock shots, and a future R10 feed can be
mixed behind one port. Within the screen package, the OCR engine sits behind a
`TextRecognizer` Protocol, and the device-specific knowledge (tile labels, target fields,
sign rules) lives in `profiles.json` rather than in code.

Parsing, validation, and the shot source are pure and dependency-free; only preprocessing
and the OCR adapter need extras. So the analysis engine and MCP server consume
screen-derived shots on the base install, with no OpenCV, no OCR engine, and no images on
hand (ADR-008).

### Sign conventions

The screen prints magnitudes with a direction word; `ShotData` wants signed degrees. This
mapping is the most dangerous part of the whole pipeline, because a flipped sign is
invisible downstream:

| Printed | Stored | Contract |
|---|---|---|
| `1.6 ° O>I` | `club_path = -1.6` | + = in-to-out |
| `6.8 ° I>O` | `club_path = +6.8` | + = in-to-out |
| `1.1 ° Closed` | `club_face_angle = -1.1` | + = open |
| `2.6 ° L` | `launch_direction = -2.6` | + = right |
| `---` | `None` | never `0` |

A number with no direction word is stored as its magnitude **with a warning**, never with a
guessed sign.

## Consequences
- **M3 is unblocked without a purchase.** Real shot data, from real swings, on hardware
  already owned — which also unblocks M4's `outcome` scoring axis, idle since the PoC.
- **ADR-004 is not superseded.** The R10 remains the right answer for real-time,
  structured, per-shot streaming, and its adapter slots into the same port. This is a
  second source, not a replacement.
- **`ShotData` grew** `bounce_and_roll`, `shot_type`, `impact_position`, and `provenance`.
  Every field stays optional: HD Golf leaves spin blank, and a blank must read as `None`.
- **Consumers must handle low-confidence shots.** `needs_review` exists to be read —
  `ScreenShotDataSource(include_needs_review=False)` is the strict view for anything that
  should only see trusted numbers.
- **A vision-model recognizer stays a cheap option.** If local OCR proves too fragile in
  practice, Option D becomes one new class behind `TextRecognizer`; parsing, validation,
  caching, and the source are untouched. That reversibility is what made choosing the
  cheaper-but-riskier option reasonable.
- **Adding a second launch monitor is a data change.** A new `profiles.json` entry, not new
  code — provided its screen is also a labelled grid.
- **Video is the same pipeline.** Preprocessing operates on a single decoded frame, so a
  future live feed is: sample frames → skip frames whose rectified screen is unchanged →
  same parser, validator, and store. No redesign; only the frame source is new.
- **Preprocessing is empirical and will need retuning** when the room, the mount, or the
  camera changes. The reference photos in `data/raw/shot_screens/` and the integration test
  over them are the regression net.
- **A new `ocr` extra** (`paddleocr`, `paddlepaddle`, `opencv-python`, `numpy`). Needed only
  to *import* screenshots; reading the results needs nothing.

---

## Addendum (2026-08-14): the sign nobody read, and three warnings that cried wolf

Every shot this repo has ever parsed carried the warning `screen title 'SHOT DATA' not
found`. Chasing it turned up a wrong number underneath, which is the more important half
of this entry.

**`spin_axis` was stored with its sign inverted.** HD Golf prints this one tile already
signed — `-9.3 °` — where every other angle on the screen is a magnitude plus a direction
word. The section above only anticipated the second shape, so the parser found no
direction word, warned "sign unknown", and stored the printed number *as printed*. The
contract is `+ = fade`; the device's polarity is the opposite. Both real shots on disk were
fades stored as draws.

Three independent readings of the same screen agree, which is why this is a correction and
not a guess:

- the `Shot Type` tile, one column away, reads `FADE` on both shots;
- the face sits 13.2 ° and 10.9 ° **open to the path** on them, which curves the ball right
  for this right-handed golfer;
- the magnitudes are what that face-to-path would produce.

`ProfileField.printed_sign` now records the polarity as data (`-1` for this tile), so a
device that prints its own signs is a profile change rather than a code change — the same
property this ADR claims for labels and direction words. A direction word still wins if
both appear.

**The correction is now self-checking.** `validate.py` gained a third cross-check beside
the two identities: `sign(spin_axis)` must agree with the curvature word in `shot_type`.
This is the only misread the arithmetic checks cannot see — every magnitude can be perfect
and the shot still reported as bending the wrong way. Below 1 ° the axis is too flat for
the word to be evidence, so the check stands down. If `printed_sign` is ever wrong for some
shot shape, it now says so instead of storing a fade as a draw.

**The sign-conventions table above gains a row**, and it is the row that breaks the
pattern:

| Printed | Stored | Contract |
|---|---|---|
| `-9.3 °` | `spin_axis = +9.3` | + = fade — device polarity **inverted** |

**And the title warning was two bugs wearing one message.** PaddleOCR returns the
wide-tracked banner as the single token `SHOTDATA`, and the check was a substring test
against `SHOT DATA` — so it failed on every real photo. It passed in the tests because the
synthetic fixture's title string was written by hand *with* the space: a fixture kinder
than the OCR engine, testing the parser against a screen that does not exist. Underneath
that, `rectify` crops the reference photos to the tile grid, below where the banner sits at
all; a title missing from a frame that starts at the first tile row is a fact about the
crop, not about the page. The check now compares without spaces and only fires when there
was something above the grid for the title to be *in*.

**Why any of this mattered.** These messages reach a golfer: `provenance.warnings` and
`needs_review` are carried out by the MCP server and named in the standing caveats, and the
coaching brief renders them. Two of the four warnings on a typical shot were unfalsifiable,
and `needs_review` was `False` on shots carrying five of them — so the noise had already
decoupled from the signal it was supposed to raise. A warning that fires on every correct
parse is worse than no warning, because it is what teaches a reader, human or model, to
skip the ones that are real.

**Still open, and deliberately.** Two warnings survive because they are true. `no tile
found for 'Bounce & Roll'` is correct: the bay's screen layout has no such tile — it shows
`Impact Position V` where the reference photos show `Bounce & Roll` — so the profile
describes a layout HD Golf can be configured out of. Enumerating those configurations needs
a bay session, not a decision here. And on the one photo where `rectify` fails, `Impact
Position`'s value is claimed by the `Shot Type` tile next door (`'CENTER SLIGHT FADE'`),
which is a cell-boundary bug that only appears on an uncropped frame.

## Addendum (2026-09-30, M31): the screen is read on the phone, what it prints is decided per golfer, and the first addendum named the wrong cause

[ADR-034](034-shot-first-phone-first.md) makes the launch-monitor photo the product and a
standalone iPhone its host. This ADR is how a photo becomes a `ShotData`, so the pivot moves where
each piece of it runs, and it turns the screen's configurability from a surviving warning into a
rule. Each point routes to an ADR-034 clause rather than restating it. **Nothing here is built**;
M32, M33 and M34 build it.

**Where each piece runs** ([clause 7](034-shot-first-phone-first.md#7-ocr-on-the-phone)).

| Piece of this ADR | On the phone | In the lab |
|---|---|---|
| Recognizer (`TextRecognizer`) | Apple Vision, behind the same boxes seam | PaddleOCR, unchanged |
| Preprocessing (the table above) | VisionKit's document camera rectifies | `preprocess.py` and OpenCV, unchanged |
| Parser, validator and `profiles.json` | the Rust port, `crates/screen` (M34) | the Python the port is recorded from |

- **The seam this ADR drew is what makes the move cheap.** §Structure put the OCR engine behind
  `TextRecognizer` so that a second recognizer would be one new class. Vision is that class, on
  another platform: boxes in, and the same parser after them. Whether Vision's boxes parse as well
  as PaddleOCR's on *this* screen is the product's biggest unknown, so M33 measures it before any
  app work ([ADR-034's sub-decision](034-shot-first-phone-first.md#sub-decision-ocr-on-the-phone)).
- **The parser and validator port because they are pure.** §Structure kept them dependency-free for
  ADR-008's sake, and that is also what lets the port be recorded from Python and gated by vectors.
  They carry six portability edges of their own, which
  [ADR-032's addendum](032-the-rust-core.md#the-screen-parser-has-edges-of-its-own-and-they-are-m34s-list-not-3s)
  lists as M34's. `profiles.json` stays one file, read by both languages.
- **The trust model travels with the validator**, confidence, both cross-checks and `needs_review`
  alike. One term of the confidence blend is the recognizer's own, the mean OCR confidence, and
  Vision's confidences are coarse, so M33 re-checks that term's weight and the review threshold for
  this recognizer. `needs_review` also gains its first reader that can act at the bay: M38's review
  step, where the golfer corrects a flagged shot.
- **Preprocessing's regression net stays the lab's.** The reference photos and the integration test
  over them (Consequences above) guard PaddleOCR and OpenCV. They cannot guard VisionKit, whose
  rectification is Apple's, so M33's measurement is the phone's net for the recognizer, and the M34
  vectors are its net for the parser.

**Option D stays rejected, on its own grounds and now one more.** The Consequence above keeps a
vision-model recognizer as "a cheap option" if local OCR proves too fragile. In the lab it is still
exactly that: one class behind the seam, not chosen, for Option D's reasons (a per-shot cost, a
network round trip and an API key for basic ingestion). On the phone it is closed, because the phone
holds no key ([clause 10](034-shot-first-phone-first.md#10-no-llm-coaching-on-the-phone)).

**What the device prints is decided per golfer**
([clause 2](034-shot-first-phone-first.md#2-printed-and-not-printed-the-device-capability-model)).
The first addendum left one warning standing because it was true: the bay's layout shows
`Impact Position V` where the reference photos show `Bounce & Roll`, and enumerating the
configurations "needs a bay session". M31 P2 read all 15 photos with the recognizer on 2026-09-29,
which made that precise without one:

- the two layouts have 15 tiles each and differ by **exactly one**, `Bounce & Roll` ↔
  `Impact Position V`;
- both carry `Custom`, a settings-gear tile with no value, which `profiles.json` already holds as
  boundary-only. It shows that the screen is configurable, and it does not tell the two apart;
- `Impact Position V` reads `---` on all 13 bay photos, so face impact on HD Golf is horizontal only.

So what one golfer's device prints is not what the device can print, and the Consequence *"adding a
second launch monitor is a data change"* now names two files. `profiles.json` stays the reader's
knowledge: labels, targets and sign rules. **`contracts/devices.json`** (M32) is the analysis side's:
every field the device can print, each `analysed` or `shown_only` with a note, and
**printed = declared ∩ `fields_present`** across the golfer's own shots. M32 adds
`Impact Position V` to `profiles.json` and `fields_present` to `ShotProvenance`, which is what makes
the intersection computable. What `Custom` can be set to show is still unknown, and the next bay trip
enumerates it.

**The surviving warning now meets clause 2.** `no tile found for 'Bounce & Roll'` is true of the
photo. But it reaches a golfer (the first addendum's *Why any of this mattered*), and it is about a
stat their screen does not show, which clause 2 says produces nothing. Whether it stays in
`provenance.warnings` and stops reaching the golfer, or stops firing once the printed set is known,
is M32's to settle. This addendum only records that clause 2 now bears on it.

**The first addendum named the wrong cause.** Its last sentence blames `CENTER` landing in
`Shot Type` on "a cell-boundary bug that only appears on an uncropped frame". The frame is
incidental:

- On `Aaron-shot-1.png` (stored as `2026-08-10-1`, the photo that sentence was about), the OCR reads
  the `Impact Position V` tile's label as plain `Impact Position`, dropping the `V`. Two label boxes
  then score 1.0 for one field, and `_find_labels` keeps the first (`>`). The V tile's box came
  first, so `impact_position` read blank ("no value text under the label"), and the real tile's
  `CENTER` was left for its neighbour: `shot_type` is stored as `'CENTER SLIGHT FADE'`
  ([M31 P2](../plans/m31-shot-first-adr.md#p2--found-2026-09-29), 2026-09-29).
- The same `V` is dropped on the 2026-08-23 session's first photo, which *was* rectified. There the
  real tile's box came first, and `HEEL` read correctly.
- Six of the 13 stored shots carry "screen outline not found - parsing the photo uncropped", and only
  `2026-08-10-1` shows the spill (their stored warnings, read 2026-09-30).

So the cause is the program plan's
[finding 6](../plans/m31-m40-shot-first-pivot.md#what-the-code-says-before-anyone-re-derives-it),
`Impact Position` scoring 0.9375 against `Impact Position V`, meeting ADR-032's edge 5 on a real
photo. Finding 6 expected the hazard to fail as a missing value, never a wrong one. It did fail as
missing, and it also left a wrong string in the next field. That string still normalizes to a fade
only because `shot_measure.normalize_shot_shape` tries curvature words before start-line words
(`_SHAPE_TOKENS`' order). **M32/M34 own the fix and its test**,
so the port does not ship the hazard (ADR-034's Consequences).

**The sign table now decides a grade.**
[5.3](034-shot-first-phone-first.md#53-shot-shapes) classifies a shot's shape from face − path, in
the golfer-relative signs §Sign conventions stores (open, in-to-out). A flipped sign here would now
move a shot between the fade and draw topics, not only mis-describe it.

- The table ports as profile data, `printed_sign` included, with the rest of `profiles.json`.
- Whether HD Golf prints golfer-relative words for a **left-hander** is unverified, because no
  left-handed shot exists. M37's left-handed vector pins the contract's reading. Only a bay photo of
  a left-hander's shot can confirm the device's.

**Not changed**:

- Option E, as the lab's decision, and every row of the preprocessing table for the lab's reader;
- "guessing is worse than declining", the trust model and both cross-checks;
- §Sign conventions, and the first addendum's `spin_axis` row and its self-check;
- `ShotDataSource` and the adapters behind it. M32 widens `ShotData`, and every new field is
  optional for the reason the Consequences give: a blank must read as `None`.

## Addendum (2026-09-30, M31.5): the lab's reader ports too, and the parser is recorded from Python once

[ADR-035](035-rust-everywhere-python-where-required.md) keeps Python only where a library the project
depends on has no alternative the user would take today. PaddleOCR was weighed against that bar and
did not meet it
([clause 2](035-rust-everywhere-python-where-required.md#2-everything-else-ports-including-the-three-things-considered-and-not-kept)).
So the M31 addendum's lab column is wrong in all three rows: two say "unchanged", and the third keeps
Python as the lab's parser. **Nothing here is built**; M34 and M29 build it.

| Piece of this ADR | On the phone | In the lab, from M29 |
|---|---|---|
| Recognizer (`TextRecognizer`) | Apple Vision (unchanged) | Rust, through `ort` (ONNX Runtime), running **the same Paddle models** |
| Preprocessing | VisionKit (unchanged) | ported with the reader, as clause 2 names them together |
| Parser, validator and `profiles.json` | `crates/screen` (M34, unchanged) | the same crate. Frozen Python records it once, before M34 ports it |

**The reader ports behind the same seam.** There are still two recognizers behind one boxes-in seam,
and the parser is still what the vectors gate
([clause 7](035-rust-everywhere-python-where-required.md#7-what-this-supersedes-sentence-by-sentence)).

- **The gate** ([clause 5](035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped)).
  The Rust reader reads the 13 stored bay photos. Each resulting shot is compared with the shot parsed
  from PaddleOCR's boxes on the same photo, and M29's plan sets what may differ.
- **PaddleOCR stops being the lab's reader once the gate passes.** It is deleted in M40, with the
  rest of `launch_monitor/` and the `ocr` extra. `launch_monitor/screen/paddle.py` sits inside the
  frozen FastAPI server's import closure, because the upload path imports the screen importer
  (clause 5's two moments).
- **The preprocessing table above stays the lab's until then.** Its reference photos and the
  integration test over them guard PaddleOCR and OpenCV for as long as the frozen lab runs them. What
  replaces OpenCV's steps in Rust is M29's plan.

**"Recorded from Python" now means once**
([clause 3](035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust), Q7 of
[M31.5 P2](../plans/m31-5-rust-first-replan.md#p2--found-2026-09-30)). The M31 addendum said that the
parser ports because it is pure, and that the port is recorded from Python and gated by vectors. Both
still hold, for the behaviour the parser has today.

- **M34 records today's parser from frozen Python first**, including the `CENTER` spill on
  `2026-08-10-1` that the M31 addendum traced, and then ports it. That recording is the port's one
  independent reference. Adding its family to the frozen recorder is the one change the frozen lab
  allows ([clause 4](035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)).
- **The fix the M31 addendum gave to "M32/M34" is M34's alone, in Rust only.** It is the tie rule
  (withhold on tie) and the `Impact Position V` tile, with hand-worked vectors, and the two label-fix
  shots are the only intended diffs (carried decisions 1, 2 and 4 in
  [the M31.5 plan](../plans/m31-5-rust-first-replan.md#decisions-carried-to-m32-and-m34-from-the-m32-interview-2026-09-30)).
- **The frozen Python parser keeps the hazard**, because frozen means no new behaviour (clause 4).
  So from M34 the phone and the lab read `2026-08-10-1` and `2026-08-23-1` differently. That lasts
  until M29 switches the lab to Rust (ADR-035's Consequences), and on the frozen server's upload path
  for as long as M29 leaves it open (ADR-035's Deferred list). The difference is declared, not a
  regression.

**One sentence of the M31 addendum is now a question for M34.** It says "`profiles.json` stays one
file, read by both languages".

- The frozen parser reads `launch_monitor/screen/profiles.json` as package data (finding 12 of
  [M31.5 P1](../plans/m31-5-rust-first-replan.md#p1--found-2026-09-30)).
- The program plan's M34 reads the same file into `crates/screen` by `include_str!`.
- The `Impact Position V` tile is an entry in that file. Adding it would change what the frozen parser
  reads, which is new behaviour in the frozen lab (clause 4). It would add a warning to every bay shot
  and move each one's `parse_confidence` by about −0.02 (planning findings 2 and 3 in
  [the M31.5 plan](../plans/m31-5-rust-first-replan.md#what-the-planning-read-found-2026-09-30)).
- Whether M34 forks the file or accepts that change is M34's to decide. ADR-035 decides neither.

**Two more of the M31 addendum's routings moved.**

- **"M32 adds `Impact Position V` to `profiles.json` and `fields_present` to `ShotProvenance`."** The
  tile, and filling `fields_present`, are M34's parser work in Rust (carried decision 1). M32 defines
  the field, in `crates/contracts`.
- **The surviving warning, left "M32's to settle".** It was settled before the re-plan: the warning
  stays in `provenance.warnings` and never reaches the golfer (carried decision 3).

**Not changed**:

- Option E as the lab's design, local OCR and geometric parsing, now in Rust;
- Option D, still rejected in the lab and closed on the phone;
- "guessing is worse than declining", the trust model and both cross-checks;
- §Sign conventions, and the first addendum's `spin_axis` row and its self-check;
- the M31 addendum's phone column, and its corrected cause for the `CENTER` spill.

## Addendum (2026-10-02, M34): the parser is Rust's, the profile is forked, and a tie is withheld

**Built**, in [the M34 plan](../plans/m34-screen-reader.md)'s phases P1–P10, whose findings carry the
detail this summarises. The parser, the validator and the profile are `crates/screen`, recorded once
from frozen Python, ported faithfully, then changed, then re-recorded by Rust. The vectors and the
gate are [`docs/CONFORMANCE.md`](../CONFORMANCE.md) §2 and §4, and the port's side of it is
[ADR-032's fifteenth addendum](032-the-rust-core.md#addendum-2026-10-02--the-screen-familys-first-re-record-a-second-version-key-and-nine-crates).
This addendum records the four choices the M31.5 addendum left open or routed here. The frozen
Python parser and its `profiles.json` do not change
([ADR-035 clause 4](035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)).

**`profiles.json` is forked** (the plan's decision 1), which answers the M31.5 addendum's question.
"`profiles.json` stays one file, read by both languages" is superseded.

- `crates/screen/profiles.json` is the Rust copy. It gains `Impact Position V` → `impact_position_v`,
  kind `text`, as the **last** tile, after Spin Axis, which is the bay screen's order. It has no alias.
- `src/golf_coach/launch_monitor/screen/profiles.json` stays frozen without it, because sharing the
  file would have given every bay shot in the frozen lab a new warning and about −0.02 of
  `parse_confidence`, which is new behaviour there.
- ADR-032 §5's one copy on disk is given up for this file until M40 deletes the Python one.
  `crates/screen/tests/profile_fork.rs` holds the two equal except a delta it declares by name, the
  V tile alone. It compares parsed JSON rather than bytes, because `core.autocrlf` decides the bytes.
- `crates/screen/tests/capability.rs` holds `crates/contracts/devices.json`'s `hd_golf` field set
  equal to the fork's targets, as a set. So "adding a second launch monitor is a data change" now
  means two files and a pin between them.

**A tie is withheld, and a tie is a margin** (decisions 3, 5 and 6, as P8 built them in
`crates/screen/src/parser.rs`). This is the fix the M31 addendum traced and the M31.5 addendum gave
to M34.

- **The rule.** Every `(box, field)` pair scoring at least 0.8 is a candidate, with the threshold and
  scoring unchanged. Within each connected group of candidates, the best one-to-one assignment
  of boxes to fields is found. Every assignment within `TIE_MARGIN` (0.02) of the best one is one the
  evidence cannot rule out. A field is read only if all of those give it the same box.
- **What a withheld field is: located, unread.** It is in `provenance.fields_present`, it has **no
  `raw_fields` key**, its value is `None`, and a warning names the boxes:
  `Impact Position: label tie between 'Impact Position' and 'ImpactPosition' - not read`. Every tile
  the parser *read* has a `raw_fields` key, `""` and `---` included. So "in `fields_present`, with no
  `raw_fields` key" is the structural mark that M35 and M37 grade `misread`, with no warning text
  parsed.
- **Its boxes still bound their neighbours.** Every box that any near-best assignment gives a field
  is a label box, so it bounds its row's columns and is never value text. That is what stops the
  spill, and not the withholding.
- **Duplicates are the same ambiguity.** Two boxes that fit one field equally well, like two `Carry`
  boxes, are two equally good reads of one tile, so the field is withheld.
- **A tie of more than eight boxes or labels is refused whole, with a warning**, rather than
  enumerated (`TIE_CAP`). Every tie on a stored photo is 2×2, and no `hd_golf` label is near another
  except the `Impact Position` pair.
- **Why a margin and not equality.** The two ways of assigning the `Impact Position` pair were
  measured on the stored photos before the margin was chosen. OCR damage to a label moves the total
  by 0.0002 to 0.004, and real evidence moves it by 0.066 or more (an `Impact Position Y` V tile is
  0.066, and an exact `Impact Position V` is 0.125). Strict equality would have read `2026-08-23-1`
  on a 0.0002 margin. On that photo's mirror image, where the V tile reads exactly and the real tile
  is damaged, it would have stored `HEEL` as `impact_position_v`, which is a wrong value. "Guessing is
  worse than declining" ranks that below a missing one.

**An erratum to the M31 addendum.** It says the `V` "is dropped on the 2026-08-23 session's first
photo" as on `Aaron-shot-1.png`, so that two boxes score 1.0. On that photo the V tile's label reads
**`ImpactPosition`**, with the `V` and the space both dropped, and it scores 0.9655, not 1.0
([planning finding 4](../plans/m34-screen-reader.md#what-the-planning-read-found-2026-10-01)). So the
real tile won on score there, not on order, and `HEEL` was right by a margin of 0.0002. That is
inside the noise above, and it is why `HEEL` becomes `None`.

**What the re-read changed**, on the boxes the recorder committed (the plan's
[P10 findings](../plans/m34-screen-reader.md#p10--the-13-shot-re-read-2026-10-02) have the per-shot
table):

- **Two shots read differently, as the M31.5 addendum predicted.** `2026-08-10-1`'s `shot_type`
  goes from `CENTER SLIGHT FADE` to `SLIGHT FADE`, and its `impact_position` stays `None`.
  `2026-08-23-1`'s `impact_position` goes from `HEEL` to `None`. No other value on the 13 bay shots
  moved, and no `needs_review` flipped.
- **The V tile is located on all 13 bay photos.** It reads blank on eleven and is withheld on the two
  label-fix shots, so no value has been read from it yet. `fields_present` tells the two layouts
  apart: every bay photo carries `impact_position_v`, and both reference photos carry
  `bounce_and_roll`.
- **`data/processed/shots/` is untouched.** The stored shots stay frozen Python's, unstamped, and the
  lab goes on reading the two label-fix shots the old way until M29 switches it to Rust. The
  difference is declared, not a regression.

**The surviving warning is filtered, by prefix.** `screen::golfer_warnings` drops each
`no tile found for '<label>'` line before a golfer sees it, and keeps the line in
`provenance.warnings`. It reads the same constant as the line that writes it (`NO_TILE_FOUND`), so
the two cannot drift. A tie line is **not** dropped, because it is about a tile on the golfer's own
screen that was there and was not read, and a retake can fix it.

**`ProfileField.scale` is dropped** (decision 4). The M32 interview's carried decision 1 and §M34
listed it, with the scale applied when a cell is read. No device prints a unit that needs
converting, so it would be code with no caller. It lands with the first profile that needs one.
Ranges for M32's six new numeric fields did land in `validate.rs`, because ranges are data and
cheap. Each one is argued as catching a dropped or inserted digit, and no shipped profile reads any
of those fields yet.

**Not changed**:

- the frozen Python parser, its profile and its tie rule, which keep the hazard until M40;
- Option E, the trust model and both cross-checks, which the Rust validator carries over faithfully,
  messages included;
- §Sign conventions and the sign table, which port as profile data;
- the M31.5 addendum's lab column. The Rust recognizer is still M29's, through `ort`.

## Addendum (2026-10-29, M29): the lab reads photos in Rust, through `ort`, and the gate held on all 13 but one declared value

ADR-035 ported the lab's reader, and M29 built it. `crates/ocr` decodes the photo (PNG through
`image`, JPEG through a hand port of libjpeg-turbo, HEIC through ffmpeg), finds and rectifies the
screen with hand ports of `preprocess.py`'s OpenCV steps, and runs three of PaddleOCR's models
through `ort`: PP-OCRv6 medium detection and recognition and PP-LCNet textline orientation, converted
once to ONNX and sha-pinned by `crates/ocr/models.json`. The rotation vote is the Python's, early
stop and tie rule included. Its boxes go to `crates/screen`'s `read`, the parser the phone calls with
Vision's boxes. So **one parser now serves both recognizers**, as the M31 addendum required.
`golf-lab import-photos` is the bulk import and `golf-ocr read` reads one photo and stores nothing.

**The gate was the shot, not the boxes** (the M29 plan's D7). On each of the 13 stored bay photos the
shot parsed from the Rust reader's boxes was held to the one parsed from PaddleOCR's committed boxes:
every value, `fields_present` and `needs_review`. Twelve matched. On `2026-08-10-1` the Rust reader
reads `impact_position` `CENTER`, which the photo shows, and PaddleOCR's UVDoc-warped boxes read
nothing. The user chose to declare that one value rather than port UVDoc (P7). The gate fails if it
goes or another joins it ([CONFORMANCE §2](../CONFORMANCE.md)). Two of the five Paddle models are
skipped on purpose: document orientation, because the rotation vote does that job, and UVDoc, because
the rectifier does.

**The 13 stored shots were re-read and stamped** (M29 P29), so the lab now reads them as the phone
would. Three values moved, all declared: the M34 addendum's two label fixes and the one above.

**Not changed**: the parser, the forked `profiles.json` and the tie rule are M34's, and the frozen
Python reader stays for the frozen server until M40.

## Addendum (2026-11-02, M42): the second profile, four opt-in parser rules, the glyph's colour, and no version bump

**M42**, in Rust only, in [the M42 plan](../plans/m42-full-swing.md). Full Swing's grid screen is
the second layout `crates/screen` reads, and it needed more than a profile. Read as `hd_golf`, a photo
of it came back wrong rather than blank, `launch_angle` holding the ball direction and `smash_factor`
holding the apex. What the device declares, why its club speed counts here, and the impact replay are
[ADR-034's M42 addendum](034-shot-first-phone-first.md#addendum-2026-11-02-m42-full-swing-is-the-second-device-a-value-off-its-impact-video-counts-as-printed-and-the-reader-has-read-none-of-its-clips-yet).
Frozen Python gains none of this (ADR-035 clause 4), and its copy of `profiles.json` keeps `hd_golf`
alone.

### The `full_swing` profile

`crates/screen/profiles.json`'s second entry, whose `_comment` says why it is what it is, and which
**reads the grid layout and no other**: the club-view layout (`IMG_2888`) is kept under `data/raw/` and
read by nothing. Its labels carry their units (`Carry Yd`, `Path Deg`), and aliases without the unit cover the
reads that drop it. `value_span_ratio` is 6.0 where `hd_golf`'s default is 3.5, because three of its
labels are 17 px tall and their values centre 3.62 to 3.85 label heights below them. Path and face
print their own sign, a minus being left, which for a right-handed golfer is `ShotData`'s convention,
so neither tile has a sign rule. `crates/screen/src/parser.rs`'s `full_swing_uses_every_rule` holds
each rule below to the tile it belongs on, because a profile's load is tolerant of an unknown key and
a misspelt one would otherwise read as absent. `validate` gained the ranges the profile first reaches:
`backspin` 0 to 15000, `sidespin` −6000 to 6000, and `apex_height` 0 to 80 yards, which it sees after
the conversion below.

`crates/screen/tests/profile_fork.rs` now declares **a whole device as well as a field**: `full_swing`
is in `DECLARED_DEVICES`, taken out of the comparison with the frozen copy, and must be absent from
that copy. Rust's refusal of an unknown profile now lists `full_swing, hd_golf`. Frozen Python's still
lists `hd_golf`, and no vector records either.

### `TextBox.rgb`, and the lit rule (`lit_sign`)

Each tile prints both of a pair's letters (`L` and `R` over ball direction, `D` and `F` over sidespin)
and lights one yellow, so the sign is a colour. `TextBox` gains an optional `rgb`: **the mean red, green
and blue (0–255) of the brightest quarter of the box's pixels**, in the image the box's coordinates are
in. `crates/ocr` fills it for each box, and Vision's boxes must fill it by the same definition (M38).
`screen::parser::is_lit` decides from it, in `screen` so both hosts share one rule: true when `min(r,
g)` is at least `LIT_MIN_BRIGHTNESS` (120.0) and blue is at most `LIT_BLUE_RATIO` (0.7) of it. On the
six photos lit letters measure 0.32 to 0.48 and unlit ones 0.89 to 0.98, on the clips' frames 0.26 to
0.39 and 0.86 to 0.96, and white digits sit with the unlit. A letter box is a value box of the tile's cell whose text, stripped and upper-cased, is exactly
one of the pair's tokens.

| The pair's letter boxes in the cell | The sign |
|---|---|
| one letter, lit | that letter's |
| one letter, unlit | the other letter's |
| both letters, exactly one lit | the lit one's |
| two boxes of one letter | one letter if they agree; if not, unknown, and the sign is withheld |
| both lit, both unlit, neither detected, or a letter box with no `rgb` | **no value**, and the warning `{label}: neither {A} nor {B} could be read as lit - sign unknown, not stored` |

**A sign that cannot be decided is withheld, never stored unsigned**: a magnitude stored without its
sign on a signed field is a wrong number half the time (ADR-010 §2). So the `sign_tokens` branch's
"storing the magnitude as printed" is not used here. A magnitude of exactly 0 needs no sign and is
stored as `0.0` before any letter is looked at. The magnitude discards a printed `-` or `+`, the tile
stays in `fields_present`, and `raw_fields` keeps the cell's text with its letters. The rule's key is
`lit_sign` (a list of two `LitToken`s), and `DeviceProfile::check` refuses at load a tile that sets it
without being a stored number, beside `sign_tokens` or `printed_sign`, or without two different tokens,
stripped and upper-case, signed 1 and −1.

### The three other rules, each opt-in

- **A box holding two merged labels is split** (`"split_merged_labels": true`, on the profile). The
  OCR sometimes returns two adjacent labels as one box (`CLUBSPEEDMPHBALLSPEEDMPH`). Before labels are
  assigned, a box scoring below `LABEL_MATCH_THRESHOLD` against every field is tried at every cut of
  its despaced text. A cut qualifies when both halves reach the threshold and name different fields,
  and the highest `min(left, right)` wins, ties to the smallest cut. The widths are proportional, which
  is close enough because the cut is the boundary between two columns and a value is placed by its
  centre. A half's best field is `field_for`'s, later-wins tie included, so a half tied between two
  fields names the later one, as a whole box does. Two-way splits only. It costs about 10 ms against a
  2 to 3 minute read. **The orientation vote does not split**: it scores the boxes as returned, so each
  merged box hides two labels from it, and `IMG_2891`'s two merges score it 0.667, above
  `GOOD_ENOUGH_LABEL_RATIO` (0.6). A third merge would make the vote try all four turns.
- **A tile printed twice must agree with itself** (`"printed_twice": true`, on `Total Yd`). The field
  takes up to two label boxes, two assignments differing only in which box is which are one assignment,
  and both cells are read. Two equal numbers store the value; two different numbers store nothing, with
  the warning `{label}: printed twice as {a!r} and {b!r} - not stored`; one number alone is stored. On
  agreement the top cell's warnings are kept, on one number that cell's, and when neither gives a number
  each cell's in turn with a repeated line dropped. `raw_fields` holds the two cells' texts joined with
  `" | "`, top first, even where one is empty. `score`'s value coverage now counts **fields** with text,
  not cells, so a doubled tile counts once and every other tile as it did. `DeviceProfile::check`
  refuses the key at load on anything but a stored number tile, and `profiles()` panics on that refusal.
- **A value printed in feet is stored in yards** (`"printed_unit": "feet"`, on `Apex Ft`).
  `PrintedUnit` has that one variant, and `to_yards` stores `printed / 3.0`, unrounded, with
  `raw_fields` keeping the printed text. **It divides**, and any oracle must too: `85.2 / 3.0` is
  `28.400000000000002`, where `85.2 * (1.0 / 3.0)` is `28.4`. The check allows it only on a number tile
  whose target `ShotData` holds in yards, and `STORED_IN_YARDS` is pinned to the schema's `yards`
  descriptions, so a yard field added later fails a test rather than being refused silently. An unknown
  unit fails the profile's parse, where an unknown key does not, because a unit read as none would
  store feet as yards.

### The quad finder's fix, and where it departs from its twin

`crates/ocr/src/quad.rs::find_screen_quad` refuses a candidate whose ordered corners repeat one. On
`IMG_2893` and `IMG_2894` a convex, plausibly sized four-gon had `order_quad`'s sum and difference rule
assign one corner to two roles, the "rectified" image was a sliver, and no text at all was recognized.
The contour's rotated box stands in, with all the labels read. **The reader now departs from frozen
Python's `find_screen_quad`**, which keeps the defect (ADR-035 clause 4), and the module doc is the
record. A refusal only narrows what was accepted, so a photo whose quad ordered cleanly cannot move,
and the OCR gate on the 13 bay photos says none did.

### No `SCREEN_PARSER_VERSION` bump

The key's rule is "bump whenever a re-read of the same photo could produce a different shot", and none
can (call 3). Each rule is opt-in and only the new profile opts in, `hd_golf`'s entry in `profiles.json`
is byte-unchanged, the quad fix touches only a quad that repeats a corner, and no stored shot names
`full_swing`. So it stays at 1, `crates/contracts/src/shot.rs`'s ledger says why under its first entry,
and **no committed screen vector moved**. Three hand-worked ones were added
([CONFORMANCE §2](../CONFORMANCE.md)).

### What it reads: no wrong value, on the stills and on the clips

The rule is ADR-010 §2's, and [two gates beside the OCR gate](../CONFORMANCE.md#the-full-swing-gates-photo-and-clip--no-family-of-their-own)
hold it.

- **The six stills read 71 of 72 values, with none wrong and no tuning.** The one withheld is the
  printed-twice rule working: on `IMG_2892` the OCR returned a stray `85TAS` box over the header's
  `82`, so its first number was 85, the two copies disagree, and without the rule a wrong 85 would be
  stored.
- **The grid read off a clip's frame is read by the still's reader.** A clip imports a shot of its own
  (ADR-034's addendum, decision 7), so one frame, written as a PNG, goes through the same `PhotoReader`
  and `screen::read` a still does, with the profile, the rules and the validator unchanged. The PNG is
  the shot's photo, with its path and digest, and its mtime is set to the clip's, which makes the clip's
  time the shot's. The decode and the encoding are deterministic, so a clip imported twice lands on one
  entry. Every frame is held in memory while the rule runs.
- **The frame rule kept is the sharpest**: `impact::sharpest_frame`, the largest sum of squared
  luminance differences to the right and lower neighbours, ties to the earliest
  (`golf_lab::shot::GRID_FRAME`). Measured on the five grid clips it reads **60 of 60 with none wrong**,
  where the middle frame reads 59, its one gap being a sidespin sign withheld because that frame returned
  no `D` or `F`. `GridFrame::Middle` stays runnable. No frame either rule chose showed the previous shot's
  numbers, which was the worry, and no profile key was tuned. On those five clips the grid reads one value
  their stills withhold, because `IMG_2892`'s stray box is in neither of its frames.

### Left as later choices, each measured and none reaching a wrong value

- **Where a rule withholds, `needs_review` stays false and the confidence does not notice.**
  `validate.rs` sets the flag only on a failed check or low confidence, as for a tie. `score`
  counts a stored field as read when its cell holds text, so `full-swing-withheld` withholds two values
  and blanks a third at 0.999. The record's only signals are the warnings and `tile_refusals`.
- **`tiles::judged` reads a joined or a signed cell as text.** Two blank cells of a printed-twice tile
  (`" | "`, `"--- | ---"`) are judged `misread` where one cell would be `printed_blank`, and a withheld
  sign's detail says no value could be read when only its sign could not. Full Swing printed a total on
  every photo, so the first gap is reached by no photo, and neither detail is in a vector, so changing
  them re-records nothing.
- **`SHOT DATA` lands in the apex cell on three photos**, because `APEX FT` is the bottom-left tile and
  no next row bounds it. The value is unharmed, since the first number wins, but a blank apex over the
  title would read `--- SHOT DATA`, which is no blank marker: `misread` where `printed_blank` is right. A
  `value_span_ratio` in [4.0, 4.8] would leave the title out on all six photos.
- **A wording edge in the word rule**: its `sign unknown, storing the magnitude as printed` would say "as
  printed" on a tile that also converts its unit. No shipped tile has both rules.
