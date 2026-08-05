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
