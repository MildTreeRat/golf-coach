# Apple Vision pass/fail thresholds — committed BEFORE any real photo was read

Written 2026-11-04, before Vision had read a single real photo of a launch-monitor screen. The rule
comes from the M7 Phase 0 spike and was reused by M1.5 and MediaPipe Hands: the pass/fail bar for the
biggest risk is not allowed to be chosen after seeing the numbers. Here the risk is the product's: if
Vision cannot read the screen, ADR-034's app plan changes before any app work starts.

Planning did run Vision, on **synthetic renders only**: the committed box geometry of two screen
vectors drawn onto a plain background, and a skewed synthetic panel. That proved the framework runs
headless and showed the shape of its boxes. It says nothing about photos, and no number below came
from it.

**No phase edits this file** ([the plan's rule 2](../../docs/plans/m33-vision-spike.md)). A bar that
looks wrong once the numbers are in is a finding, and the measured verdict stands beside it.

**Which numbers are whose.** T1–T4 and the verdict table are the user's, from the planning interview
on 2026-11-04: the coverage floor is M42's own target of 66 of 72, with a per-field floor added, and
the flagged ceiling was chosen against the lab reader's 0 of 6. The ladder's order and limit, and the
acceptance rules for a parser rule, are the plan's calls, approved with it.

## The question these decide

**On the photos of Full Swing's grid screen we already hold, does Apple Vision produce boxes that
`crates/screen` turns into correct shots, with no laptop in the path?**

"Correct" is what ADR-010 §2 asks: a wrong number is worse than a missing one, so a wrong value the
golfer is not warned about is the failure this is looking for, and a value left out is only a cost.
The parser is Rust's, `golf-core parse-screen`, because that is what the phone will call.

Full Swing decides. HD Golf is measured and logged, with no bar and no verdict. In the user's words:
"Full swing matters most."

## Definitions

- **The stills**: `IMG_2889`, `IMG_2891`, `IMG_2892`, `IMG_2893`, `IMG_2894` and `2026-10-07-1`. Twelve
  stored fields each, seventy-two in all.
- **The clip frames**: the five clips' grid frames, each chosen by the lab's rule
  (`golf_lab::shot::GRID_FRAME`). Sixty values, against the same truth.
- **The truth**: the M42 plan's table, read by eye with each lit letter confirmed on a crop, the same
  table as `PHOTOS` in `crates/ocr/tests/full_swing.rs`. Every capture is held to its digest first.
- **A phone path**: `raw` (the photo as imported, upright) or `vision` (rectified by
  `VNDetectRectanglesRequest` and `CIPerspectiveCorrection`, the stand-in for VisionKit's scanner, and
  the raw image where no rectangle is found). The third mode, `lab` (`ocr::rectify::working_image`), is
  a diagnostic: it tells the recognizer's faults from the rectifier's.
- **An arm**: one config, one mode, one set of captures.
- **A cell**, one field of one shot, compared as an IEEE double with no tolerance:

| cell | means |
|---|---|
| **read** | the value equals the truth |
| **withheld** | the value is `None` |
| **wrong** | a value that differs, on a shot **not** flagged `needs_review` |
| **caught** | a value that differs, on a flagged shot |

A `null` shot, a failed read, is twelve **withheld** and counts as **flagged**: to a golfer it is a
re-take. Everything is measured at `min_confidence` 0.6, whatever the calibration goes on to recommend.

## The bars

| bar | stills (6 photos, 72 values) | clip frames (5, 60 values) |
|---|---|---|
| **T1** wrong | 0 | 0 |
| **T2** read | at least 66 | at least 55 |
| **T3** each field read on | at least 5 of 6 | at least 4 of 5 |
| **T4** flagged | at most 1 | at most 1 |

- **T1** is the point of the spike. **caught** cells do not fail it, because review exists for them.
- **T2** and **T3** are why a recognizer cannot pass by leaving things out. **caught** cells are not read,
  so they count against both. T3 exists because 66 alone would pass one that always loses a field.
- **T4** is why a recognizer cannot pass by flagging everything, which would otherwise satisfy T1 and
  leave the golfer checking every shot at the bay.

An arm passes when it meets all four.

## The verdict

| | stills pass on `raw` or `vision` | stills pass on `lab` only | stills pass on none |
|---|---|---|---|
| **Verdict** | **GO** | **CONDITIONAL**: Vision reads the screen; M38 must also ship a rectifier | **NO-GO**: the app waits on the user |

The clips get a verdict beside it that never blocks: **PASS** on `raw` or `vision`, **LAB-ONLY** on `lab`,
**MISS** otherwise, which means M38 ships the still import first.

A verdict reached with any config but C0 is labelled **tuned**, and one reached with a parser rule
**tuned, parser**, because neither has a held-out sample: each was fitted on the photos it was measured
on.

Every outcome closes the milestone. The spike's job is to answer.

## What may be tried, and in what order

**C0**, the first settings: `VNRecognizeTextRequest`, `.accurate`, revision 3,
`usesLanguageCorrection = false`, `recognitionLanguages = ["en-US"]`, no custom words, one box per
observation.

**The ladder**, only if C0 misses the target, which is: stills pass on a phone path, **and** clips pass on a
phone path.

| config | what it changes |
|---|---|
| **C1** `words` | each observation split into word boxes |
| **C2** `x2` | the image upscaled ×2 first |
| **C3** `lexicon` | language correction on, with the labels and sign letters as custom words |
| **C4** `second-pass` | a second read of four overlapping corner crops at ×2, adding what the first missed |

Round 1 runs C1, C2, C3 and C4 alone, stopping at the first that meets the target. Round 2, if none did,
takes the best single config and adds each of the other three in turn, stopping the same way. **At most
eight configs are ever measured, C0 included.** "Best" is the plan's call 10, and so is which config ends up
final.

**Parser rules**, only if the final config still misses on a phone path. **At most three**, each opt-in for
`full_swing`, each with its own hand-worked vector. A rule is accepted only if all of these hold:

1. The printed value is in Vision's boxes and the parser mishandles it. What Vision never returned is the
   recognizer's, and out of scope.
2. It states a general rule, naming no photo, value or position unique to one. `hd_golf`'s profile entry
   stays byte-identical.
3. It does not weaken ADR-010 §2. A signed value is stored only when the lit letters decide it. The lit
   constants, the confidence weights, `min_confidence` and `SCREEN_PARSER_VERSION` do not move.
4. It has a hand-worked vector from Vision's unedited boxes.
5. No committed vector moves.
6. The lab's parses of the same captures, and the clip gate, stay as measured before any rule.
7. Nothing else breaks: the macOS test baseline, fmt, clippy's count and the conformance tests all hold.
8. It helps: more **read** cells on the arm it targets, and no more **wrong** ones.

A rule failing any of these is withdrawn, and that is a finding, not a stop.
