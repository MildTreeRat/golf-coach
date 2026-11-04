# Apple Vision on Full Swing's screen — findings

**Verdict: GO, provisional: pasted copies. Clips: not measured.**

**This is not yet the answer on the photos as captured.** By the table in
[thresholds.md](thresholds.md), stills that pass on a phone path read GO, and they passed on `raw`, at
C0, the first settings, so the verdict carries neither **tuned** nor **tuned, parser**. But what Vision
read were six JPEGs saved from a chat paste, not the files the truth table was read from, and the five
clips, HD Golf's fifteen photos and the lab's reader have not run at all, because none of them is on this
Mac. M33 is **open**. It closes when the stills are measured again on the original files and those three
have run. Whether M38 is unblocked on this evidence is the user's call, and this log does not make it.

**Run 2026-11-04** on the arm64 Mac: macOS 26.2 (25C56), Vision revision 3, `min_confidence` 0.6, the
parser `golf-core parse-screen` (`target/m33/release/`). The thresholds were committed in
[thresholds.md](thresholds.md) before Vision read one real photo, and no phase has edited them.
`probe.py verdict` works the two tables under "What was measured" again from `results/C0.json`, each cell
from its value and the truth table, and exits 1 if this file says otherwise. The plan is
[m33-vision-spike.md](../../docs/plans/m33-vision-spike.md), and its findings hold each phase's working.

## What was measured

Six stills of Full Swing's grid screen, 72 values against the M42 plan's truth table, compared as IEEE
doubles with no tolerance, on three modes:

| mode | the image, in this run | role |
|---|---|---|
| `raw` | the copy, decoded through ImageIO to sRGB | phone path: a photo as imported |
| `vision` | Vision's largest rectangle through `CIPerspectiveCorrection`, or `raw`'s image where none was found | phone path: a stand-in for VisionKit's scanner |
| `lab` | `ocr::rectify::working_image` off the lab's own decode of the copy, 1600 wide | diagnostic: Vision reading the lab's **rectifier**. It is not the lab's reader |

| mode | T1 wrong (at most 0) | T2 read (at least 66) | T3 lowest field (at least 5 of 6) | T4 flagged (at most 1) | arm |
|---|---|---|---|---|---|
| `raw` | 0 | 71 | 5: `sidespin` | 0 | **PASS** |
| `vision` | 0 | **59, miss** | **4: `sidespin`, miss** | 1 | **MISS** |
| `lab` | 0 | 68 | 5: `launch_direction`, `apex_height`, `smash_factor`, `sidespin` | 1 | **PASS** |

Read per photo:

| id | `raw` | `vision` | `lab` |
|---|---|---|---|
| `IMG_2889` | 12 | 12 | 12 |
| `IMG_2891` | 12 | 12 | 12 |
| `IMG_2892` | 12 | 12 | 11 |
| `IMG_2893` | 11 | 11 | 11 |
| `IMG_2894` | 12 | 0 | 12 |
| `2026-10-07-1` | 12 | 12 | 10 |

**The passing arm has no room on one bar.** `raw` is at T3's limit: `sidespin` read on 5 of 6, so one more
lost sign on any photo misses that bar. T2 has 5 to spare.

**`raw` read more than `lab` here, and that is not a like-for-like.** `raw` gives Vision 2576 px on the long
side, where the phone holds 5712, while `lab` shrinks to 1600 either way. These copies are scaled by
2576/5712 per side (five of them, by M42's recorded sizes and the finder's corners; `2026-10-07-1`'s
original size is not recorded anywhere read), so the gap could move either way on the originals.

**The lab's reader, beside each: not run.** Its models are not on this Mac. What it measured in M42 was 71 of
72 on the stills, with `IMG_2892`'s `total_distance` withheld, 60 of 60 on the clip frames and 0 of 6 flagged,
and those numbers are on the **original files**, so nothing in this run stands beside them.

**Every cell not read**, with Vision's text for that tile. Each withheld cell is on an unflagged shot, except
`vision`'s null shot and `lab`'s `2026-10-07-1`.

| mode | id | field | cell | value / truth | Vision's boxes for the tile |
|---|---|---|---|---|---|
| `raw`, `vision` | `IMG_2893` | `sidespin` | withheld | none / -638 | `SIDESPIN RPM`, `638`, and no letter. By eye `D` (lit) and `F` are both printed, and Vision returned neither |
| `vision` | `IMG_2894` | all twelve | withheld, failed read | none | 0 boxes: its image is the replay panel |
| `lab` | `IMG_2892` | `sidespin` | withheld | none / -2032 | `S DESPIN R`, `2032`, and no letter |
| `lab` | `IMG_2893` | `launch_direction` | withheld | none / 2.2 | `BALL DIRECTION DEG`, `2.2`, and no letter |
| `lab` | `2026-10-07-1` | `apex_height` | withheld | none / 47.4 | `APEX RI`, so `no tile found for 'Apex Ft'`, and `142.2` went to the next tile |
| `lab` | `2026-10-07-1` | `smash_factor` | **caught** | 142.2 / 1.43 | tile text `142.2 1.43`; flagged at confidence 0.542 by the speed cross-check and the range check |

No arm has a **wrong** cell. The one **caught** cell's crop shows `142.2` under `APEX FT` and `1.43` under
`SMASH FACTOR`, which agrees with the truth table, so nothing here says the table may be wrong. Stray boxes
reached stored tiles on `raw`, and each still gave the right value: `SHOT DATA` under `Apex Ft` on four photos,
`AII` and `14` under `Ball Speed Mph`.

**Repeatable on this Mac.** Two `measure` runs matched in all 36 boxes and shot files, byte for byte, a third
`read` was byte-identical to `boxes/C0/`, and `parse-screen` on the committed boxes equals each stored shot.
Throwaway recounts, with the plan's table typed afresh and neither `truth()` nor `classify()` in the path, gave
the same totals and bars each time. Each capture was held to its digest before it was read.

## The lit table

`b ÷ min(r, g)`, and in brackets the margin, `LIT_BLUE_RATIO` (0.7) less the ratio, positive on the lit side,
for each letter box returned. Both constants are copied from `crates/screen/src/parser.rs`, parsed at run time.
`is_lit` agrees with the truth's letter on every row, and every sign stored equals the truth's. `vision`'s rows
are `raw`'s, less `IMG_2894`'s. No box here sits near `LIT_MIN_BRIGHTNESS`' 120: the lowest `min(r, g)` is 143.63.

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

`raw`: lit 0.096–0.363, unlit 0.833–0.994, smallest margins +0.337 and −0.133. `lab`: lit 0.336–0.482, unlit
0.893–1.007, smallest margins +0.218 and −0.193. M42's lab numbers were lit 0.32–0.48 and unlit 0.89–0.98.

**The conversion to sRGB is what moves the lit letters down**, and the two columns differ for two reasons, so the
gap between them is not all colour. All six copies are tagged Display P3. `raw` converts them, as the phone's
plugin would, and `lab`'s decode ignores the tag and then goes through CLAHE. One look before CLAHE, on the 15
letter boxes `raw` returned: on the `load_image` decode, lit 0.34–0.45 and unlit 0.85–0.99; on sRGB, lit 0.10–0.36
and unlit 0.83–0.99. Every letter stays on its own side of 0.7 either way. The sRGB side's lowest unlit, 0.833, is
`F` on `IMG_2894`, under M42's 0.89, and it is the margin that came closest.

**Letters returned, of the 24 printed: 15 on `raw`, 13 on `lab`.** Signs, of 12: on `raw`, 7 from a lit letter seen,
4 from an unlit letter alone, 1 withheld; on `lab`, 7, 3 and 2.

Swift's `rgb` was held to Rust's `glyph_rgb` first, by `==` after the JSON trip: 0 boxes differing on all six
decoded stills and a PNG written by Rust's encoder, and 0 of 3,054 boxes given by hand through both. Ties moved
30–38 boxes a still, so the tie rule matters to the bit.

## The rectifier stand-in

`VNDetectRectanglesRequest` with the plan's untuned values found a rectangle on **1 photo of 6, and that one is the
wrong one**: on `IMG_2894`, confidence 1.0, 0.115 of the photo, the replay panel and none of the twelve tiles. So
`vision` fell back to `raw`'s exact bytes on five, and read a panel of no tiles on the sixth, as the plan says it
should. On those five its boxes and shot equal `raw`'s, with only the uncropped note added, so it is not an
independent arm here. **On this input `vision` can read at most 60 of 72, and T2's 66 is out of its reach at any
config.** So on this run "`raw` or `vision`" is `raw` alone. **`vision`'s miss says nothing about VisionKit's
scanner**, which needs a device, and M38 measures it.

The likely reason, by eye and not by test: neither side edge of the monitor is in any of the six frames. The golfer
filled the frame with the tiles, so the screen has no four corners inside the picture. One look with every limit
opened: 30–39 rectangles a photo, the largest a sub-panel or the bezel on five, so call 7's values are not the
obstacle on those five. Rectangle detection at the originals' size is not measured.

## Vision's confidences

At C0, with language correction off. **ADR-014's question, answered on these six: no.** At every `min_confidence`
from 0.5 to 0.9, on every mode, the same shots flag, and neither flag comes from a confidence.

- **Two distinct values on the phone modes, and one on `lab`.** `raw`: 248 boxes at 1.00 and 3 at 0.30. `vision`: 205 and 3. `lab`: 237 at 1.00. The three
  are strays, `DALI`, `AII` and `*ТАМАНA`. A low confidence is not what marks a misread here: the box behind the
  one differing cell, `APEX RI`, is at 1.00.
- **On the phone modes `parse_confidence` is the coverage terms plus a flat +0.30.** The OCR term is 0.300 on 9 of the
  11 parsed shots and 0.294 on the other two, which are one photo read twice.
- **What flagged a shot was a failed read (`vision` `IMG_2894`) or a physics check (`lab` `2026-10-07-1`).** Confidence
  alone flagged none of the 2, and at 0.6 or 0.7 it cannot flag a shot with every label and value (0.70 with
  every box at 0, by the arithmetic of `score`'s weights).
- **The recommended `min_confidence` is 0.6, provisional and vacuous.** It is the rule's floor on an empty set: no
  phone-mode cell differs from the truth at any threshold, so the rule had nothing to test. "0.6 is enough" is not a
  finding of this run.
- **A mutation, not a photo.** With `SMASHFACTOR` garbled on the committed grid vector's boxes, `1.08` falls into
  `Backspin Rpm`'s tile, and `backspin` stores `1.08`, **wrong and unflagged at every threshold up to 0.9**, with
  `no tile found` raising no flag by itself. So on that route `needs_review` rests on a physics check happening to
  fail. Beside it, also on a mutation: a lit `R` box read as `L` is **wrong and unflagged**, because `validate.rs`
  holds `launch_direction` to a range and no identity. No measured arm has either.

## What was tried

**C0 and nothing else.** No rung of the ladder ran and no parser rule was written, because `raw` passes on stills, and
under the decision of 2026-11-04 stills are this run's target. **Both "not needed" rest on that decision, not on
`thresholds.md`**, whose target is stills **and** clips, so its own test of whether the ladder runs has not been
answered, and call 11's condition on clips is unanswered too. C0's standing as final is provisional for the same
reason the verdict is. **C1 to C4 are unmeasured**, so neither "the ladder would not help" nor "C0 is the best setting"
is a finding of this run, and nothing here says whether one would return `IMG_2893`'s letters, lift `raw` off T3's
limit, or add a **wrong** cell.

**None of the 14 cells not read on the phone modes is parser-fixable**: `IMG_2893`'s `sidespin` has `638` and neither
letter on the photo, and call 11's check 3 lets nothing but the lit letters sign it; `IMG_2894`'s twelve have no boxes. On
`lab`, not a target arm, 2 of 4 are (`APEX RI`), and nothing was fitted to them. Nothing under `crates/` or `spec/` moved.

## What Vision did on HD Golf

**Not measured, by the user's omission** ("Omit HD golf for now"), and its photos are not on this Mac. It has no bar and no
verdict, so it moves the verdict neither way. What Vision makes of that screen is unknown in every mode. ADR-034's
`Impact Position V` hazard on `2026-08-10-1` and `2026-08-23-1` is **unanswered for Vision**, and the one declared difference
has not met a Vision read. Two things read like evidence and are not: `control`'s "all 15, 168 read" is the committed
vectors' own boxes, and planning's "42 boxes in, 42 out" was a synthetic render.

## What this does not show

- **The original files.** These are copies: five of the six stills were HEICs, so they are re-encoded, and scaled too.
  They carry no camera metadata. Each id was assigned by matching the picture to the truth table by eye, and all twelve
  values and the lit letters were seen to agree (`check` holds the copies to their own digests, not the originals').
  `lab` is therefore not the pixels M42 measured on any photo.
- **The clips.** Not on this Mac, and that absence is this run's reading of the user's "proceed", not the user's stated
  choice. Nothing here says whether a grid frame reads, and nothing here tells M38 to ship the still import first, which is
  what a **MISS** would have meant.
- **HD Golf**, above.
- **The lab baseline.** The reader's models have not run on this Mac, and their absence is the same reading of "proceed", so
  there is no lab column. The CONDITIONAL arm still means what it says, since `lab` mode is the lab's rectifier in front of
  Vision and needs no model.
- **Six stills, from one setup.** Five taken on 2026-10-10 and one on 2026-10-07, not a new bay session. M38 P5's session
  of at least 30 photos is the real test.
- **macOS's Vision, not iOS's.** M38's plugin is what tests that.
- **A VisionKit stand-in, not VisionKit**, and a stand-in that found the wrong rectangle or none.
- **Anything about tuning.** Nothing was tuned on these photos, which also means nothing was held out, and C0 is the only
  config measured. A pass on a later row that needed a rung or a rule carries that label, because each is fitted on the
  photos it is measured on.
- **A test of `needs_review`.** With no **wrong** cell on any arm, six stills gave it nothing to catch. The two routes above
  are on mutations.

Still needed before M33 closes: P9 again on the original files, and the clips, HD Golf and the lab baseline. `probe.py check`
lists every file by path on each run.

## For M38

Every line carries the label: **provisional: pasted copies**, and at C0.

1. **The settings**: C0, and "won" overstates it, since it is the only config measured. `VNRecognizeTextRequest`, `.accurate`,
   `VNRecognizeTextRequestRevision3`, `usesLanguageCorrection = false`, `recognitionLanguages = ["en-US"]`,
   `automaticallyDetectsLanguage = false`, no custom words, one box per observation from `topCandidates(1)[0]`, flipped to
   top-left pixels and unrounded. `vision_ocr.swift` is spike code, so take what it learned and not necessarily it.
2. **The input path**: `raw`, the photo as imported. `vision` is unanswered rather than failed. A question for it, not a
   result: photos framed like these give a corner-finding rectifier nothing to find, so either the golfer is told to get the
   whole monitor in frame, which costs the tiles pixels, or the app does not depend on a rectifier.
3. **Colour and the lit margin**: decode through ImageIO to 8-bit sRGB, then read `rgb` from those bytes with no colour matching,
   by `TextBox::rgb`'s definition, ranking on `(sum, rowMajorIndex)` as `glyphRGB` does: "the brightest quarter" with any other
   tie rule does not reproduce it to the bit. On that, `raw`'s smallest margins against `LIT_BLUE_RATIO` are +0.337 and −0.133, and
   neither lit constant moved. iPhone HEICs through this path are not measured.
4. **`min_confidence`**: pass 0.6, provisional and vacuous. **`W_OCR` is M38's open question**: leave a flat +0.30 in the score, or
   change the weight and re-record the screen family. Boxes in a hand vector carry Vision's confidence as `0.30000001192092896`.
   All of this is with language correction off, and C3, which turns it on, is unmeasured on photos.
5. **Parser rules**: none landed. `crates/`, `spec/` and `SCREEN_PARSER_VERSION` are untouched. A withheld sign stays an unflagged
   shot at 1.000 with a warning, as ADR-014's M42 addendum says of a rule that withholds, so review does not surface it. Vision losing
   sign letters is what holds `raw` at T3's limit (15 of 24 returned).
6. **Vectors**: the committed `boxes/C0/<mode>/<id>.json` can seed them, with each file's digest in `results/C0.json`. They are the
   copies' boxes, so a vector made from one should say so.
7. **Its P0, the five macOS failures** of `cargo test --workspace --no-fail-fast`, none on this path: `capture --lib`'s
   `a_host_with_no_cameras_is_an_empty_list_and_not_an_error`, `an_unknown_id_is_gone_and_says_which` and
   `listing_twice_gives_the_same_order`; `golf-lab --test flight`'s `an_empty_store_says_how_shots_arrive`; and
   `golf-lab --test import_photos`' `a_path_is_spelled_as_pythons_str_path`. The two HEIC gates in `crates/ocr/tests/` cannot run on
   this Mac either, because this ffmpeg decodes one tile of a HEIC.
8. **clippy**: 28 `warning` lines on Rust 1.99.1, where the Windows box's toolchain reported 0. They are newer lints, not new code.
9. **The missing extras**: `.venv` here is `.venv/bin/python`, Python 3.14.3, with base and dev dependencies only and no `api` or `llm`, so
   the frozen suite shows 4 failures and 13 collection errors. It needs the extras if it wants that suite on this Mac.
10. **Homebrew**: `/opt/homebrew` belongs to another account, so Flutter and CocoaPods need a route that does not use it.
11. **HD Golf, an open item, all of it.** Whether it reads is unknown, so the `hd_golf` path has no Vision evidence behind it.
12. **The clips, an open item.** Neither "ship the still import first" nor "clips read" follows from this run.

## Choices the plan did not spell out

Each taken under the plan's rule 1 as the option changing least, none touching `thresholds.md`. `stored` is the one that reaches
a cell, and it reads as call 8 says. The plan's findings hold each in full.

- **The manifest and `check`.** Every entry has a `copy_of`, so a copy cannot be read as the original and the label rests on data.
  `check` stops on a capture missing or differing, on any file present with the wrong digest, on a manifest `build-manifest` would
  not write, and on no `swiftc` or `cargo`, and only prints a missing ffmpeg. `IMG_2888` is held to its digest though it is not a capture.
- **The self-test's limit.** The plan's 5% is held on a box's **centre**, on five words of the seven drawn, because Vision's boxes are
  not the ink's rectangle, and that split was read off the result. Its worth is the six injected bugs it fails on, not the number: a
  later macOS could shift a box past the limit with nothing wrong in the mapping.
- **Beyond the plan's commands.** `vision-ocr glyph`, because Vision's boxes cannot try the whole of `rgb`'s definition. `working` and
  `rectify` print beyond the keys asked for, and `prepare` adds digests and the macOS build, because `frames/` is gitignored. The
  rectifier's fallback is the input's bytes rather than a second encoding, and the self-test covers the rectifier. `failures` and
  `calibrate` print and add no results file, and `failures` says `not fixable` where the plan says "the recognizer's", because of
  `IMG_2894`'s twelve. `calibrate` prints `lab` as a diagnostic that its rule does not read, reads T4's ceiling per phone mode, and
  adds a 0.95 row and two arithmetic lines, each labelled.
- **The comparator.** Call 14's "non-provenance keys" is read as four fewer (`shot_id`, `session_id`, `timestamp`, `source`), since no
  screen printed them. The bars are parsed from `thresholds.md` and the lit constants from `parser.rs` at run time rather than copied, so
  neither has a second copy. `measure` holds each frame back to its capture through the sidecars. `stored` lets `backspin` and `sidespin`
  be absent, because `ShotData` omits them when `None`, and that was found by the first real read and fixed before any table printed.
  An input's `image_path` and `image_sha256` are the capture's and not the mode's image, `shot_id` the capture's id, and `session_id` `m33`.
- **What is committed.** `boxes/C0/` holds boxes files only, since `parse-screen` regenerates each shot from them, and `results/C0.json`
  is the second run's, since its `repeat: same` is the record that the two matched. One look beyond call 16, at `IMG_2893`'s sidespin tile
  on `raw`, because it is the passing arm's one miss.
- **`verdict` itself.** It classifies each cell again from its value and judges every bar, where the plan asks only that it recompute the
  table. It holds `boxes/C0/` to the digests in `results/`, stops if an arm is absent or a second config is present, since call 10's
  choice among several is not written because the ladder never ran, reads `vision-*.json` hand vectors as "a rule fired", and exits 1 if this
  file's verdict or either table differs. `control` gained a control 8 for all of that, as 1b and 5 to 7 were added beyond the plan's
list.
