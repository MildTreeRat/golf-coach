# M38's bay session: pass/fail bars, committed BEFORE the session

**Tier: TARGET until the session runs, then a record of what was asked.** Written 2026-11-04, before
any phone had photographed a launch-monitor screen. The rule is the one
[`spikes/vision-ocr/thresholds.md`](../../spikes/vision-ocr/thresholds.md) reused: the bar for the
biggest risk is not chosen after seeing the numbers.

**No phase edits this file** ([the plan's rule 5](m38-iphone-app.md#rules-every-phase-keeps)). A bar
that looks wrong once the numbers are in is a finding, and the measured verdict stands beside it.

**Which numbers are whose.** M33's four bars, restated as shares, and the latency bar are the user's,
from M38's planning interview on 2026-11-04. Six stills cannot support a tighter number, so these are
M33's own limits scaled: 66 of 72 is 92%, 5 of 6 is 83%, 1 of 6 is 17%.

## The question these decide

**At the bay, with no laptop, does the app's path (an in-app camera photo as taken, read by iOS Vision
at C0, parsed by `crates/screen`) turn Full Swing's screen into correct shots, fast enough to use
between swings?**

"Correct" is what ADR-010 §2 asks: a wrong number is worse than a missing one, so a wrong value the
golfer is not warned about is the failure this looks for, and a value left out is only a cost.

## Definitions

- **The session**: at least 30 photos taken with the in-app camera, across at least 2 clubs, on Full
  Swing's grid screen, in one sitting at the bay. Fewer photos or one club is **not a session**, and the
  report says `INVALID` rather than judging it. Library imports are not counted.
- **The fields**: the twelve Full Swing tiles `crates/contracts/devices.json` declares from the screen.
  `impact_offset_h` comes from the replay and is not among them (the plan's D4).
- **The truth**: each shot's values after the person's review, with the photo on the screen beside them.
  Every shot is reviewed before the export, every not-read value is typed in, and a tile the screen itself
  left blank stays blank (the plan's P31 run sheet). The report trusts this review; nothing else can
  stand behind 30 new photos.
- **As read**: a field's value in `provenance.corrections` if it has an entry there, otherwise the value
  stored. An empty entry means nothing was read.
- **A cell**: one field of one photo. Each is one of:
  - **read**: as read and truth both hold the same value, compared as IEEE doubles with no tolerance;
  - **wrong**: as read holds a value the truth differs from;
  - **withheld**: nothing was read and the truth holds a value;
  - **blank**: neither holds a value. It is left out of every denominator.
- **A photo the session already held** (status `duplicate`) is not a photo for any count below.
- **A failed read** (no shot stored) is twelve withheld cells and counts as flagged. Its truth is not
  typed in, so its cells are counted without a value.
- **Flagged**: `.app/reads.jsonl` says the photo's status at read time was `flagged` or `failed`. A later
  review cannot change it.
- **Latency**: `ms_total` in `.app/reads.jsonl`, from the shutter's image reaching the app to the result
  card's first frame, over camera photos, at the nearest-rank 95th percentile.

## The bars

| bar | what | limit |
|---|---|---|
| T1 | wrong cells on photos not flagged | at most 0 |
| T2 | read cells, of read + wrong + withheld | at least 92% |
| T3 | the lowest field's read cells, of its read + wrong + withheld | at least 83% |
| T4 | flagged photos, of all photos | at most 17% |
| T5 | latency at p95 | at most 5000 ms |

`golf-bay-report bars` parses this table at run time, so it has no second copy of these limits.

## What a result means

- **All five met**: **PASS**. The app's path stands, and ADR-034 clause 7 is amended to say the scanner
  stays unmeasured by choice (the plan's D12).
- **Any of T1 to T4 missed**: **MISS**. It does not fail the milestone by itself. It triggers the plan's
  P33 and P34, which build and measure VisionKit's scanner, and the user decides what ships with both
  numbers in front of them.
- **Only T5 missed**: **MISS (latency)**. The scanner would not help, so P33 is not triggered. The user
  decides at P32 whether M38 closes on it.

## What this does not show

- **Vision's confidence as a catch.** T1 can be met with no confidence ever mattering. The report prints
  the confidences beside the bars, and that is the evidence for M33's open `W_OCR` question (the plan's D8).
- **HD Golf, the clips and the impact offset**, none of which this session reads (the plan's D3 and D4).
- **A second bay, or other light.** One sitting from one spot.
