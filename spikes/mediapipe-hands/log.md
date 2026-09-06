# MediaPipe Hands on a golf grip — findings

**Run 2026-09-03 against the fifteen face-on swings already on disk.** No new footage; the spike
needed none. Thresholds were committed in [thresholds.md](thresholds.md) before the model was run
over a single frame.

**Verdict: NO-GO, and the failure is not the one that was expected.** The model does not fail to
*find* a hand — at the best crop it returns one on **179 of 179** address frames, correctly placed,
with the right handedness label. It fails to find **two**. A golf grip reads to `HandLandmarker` as
one hand, and the hand it settles on is the **trail** hand: two-hand detection **0.15** against a
0.60 floor, lead-hand detection **0.14**. Grip strength is read on the lead hand, so the
pre-registered PARTIAL rows do not apply either — the column that came back green is the one
written down in advance as not carrying the coaching claim.

## What was measured

Fifteen face-on swings, 2160×3840, one right-handed golfer, one bay. 179 frames inside
`measure.address_sample_bounds` — the same windows M14 P3 screened, resolved by importing that
script's `load_clip` rather than re-deriving them. `RunningMode.IMAGE`, MediaPipe defaults,
`num_hands=2`.

| arm | ≥1 hand | **two hands** | placed | distinct | lead det | trail det | lead jit | trail jit |
|---|---|---|---|---|---|---|---|---|
| full frame (control) | 0.16 | **0.00** | 1.00 | — | 0.00 | 0.16 | — | 4.66 |
| crop k=1.0 | 0.99 | 0.02 | 0.85 | 1.00 | 0.07 | 0.79 | 7.52 | 2.87 |
| crop k=1.5 | 0.94 | 0.04 | 0.97 | 0.38 | 0.20 | 0.76 | 2.54 | 4.95 |
| **crop k=2.0** | **1.00** | **0.15** | **1.00** | **0.96** | **0.14** | **1.00** | **7.08** | **3.14** |
| crop k=3.0 | 0.98 | 0.15 | 0.95 | 0.58 | 0.23 | 0.83 | 11.41 | 2.56 |

k=2.0 is the best crop by every column that matters and is the one the gates are read against, as
`thresholds.md` pre-registered. The full-frame control failed exactly as predicted, which is what
says the crop arm is doing real work rather than papering over a broken probe.

Recorded, not gated, at k=2.0: handedness label agreement **1.00** on the trail hand and 0.80 on
the lead; hand-model wrist **0.03 / 0.07 sw** from the pose wrists; index MCP **0.04 / 0.09 sw**
from the pose model's own index landmark.

## Four things found by running it rather than reasoning about it

**1. The model sees a grip as one hand, and it is not a confidence problem alone.** Dropping the
palm-detection threshold from 0.5 to 0.2 — a post-hoc diagnostic, labelled as one, and it cannot
promote a FAIL — takes two-hand detection from 0.15 to **0.35** and lead-hand detection from 0.14
to **0.35**. So the second hand is *partly* found and suppressed as an overlapping duplicate, and
partly never proposed. Neither half gets within sight of 0.60, and buying detections by lowering a
threshold buys false ones too.

**2. The one hand it returns is anatomically placed, which is what makes this a clean no-go.**
Placement 1.00 and distinctness 0.96 at k=2.0, and the index MCP lands 1.6–3.6 cm from the pose
model's own index knuckle. There is no story here where the probe mis-assigned detections or the
crop was wrong. The model is doing something reasonable and it is not the thing a grip metric
needs.

**3. The glove is not the explanation.** The obvious next suggestion — the lead hand is gloved and
white gloves are out of distribution — is dead on arrival: the address frames include gloved and
bare-handed swings, and the count comes back **one hand** on all fifteen. Written down because it
is the cheapest-sounding follow-up and it would have cost a bay session to learn.

**4. The hand-span column settled less than it promised.** Wrist-to-middle-fingertip came back at
0.28 sw against the ~0.47 sw an **open** adult hand measures, which looks like a collapsed blob
until you notice that a hand curled around a grip is genuinely shorter than an open one. The
column stays in the probe as context and is not evidence in either direction. The two knuckle
columns are the ones that carry the finding.

## Against the committed thresholds

| Gate (crop k=2.0, address, face-on) | Bar | Result |
|---|---|---|
| Detection — two-hand frames | ≥ 0.60 | **0.15 — fails**, by 4x |
| Placement — placed / distinct | ≥ 0.90 | 1.00 / 0.96 — **passes** |
| Stability — jitter vs raw pose | ≤ 2.0 | 3.14 trail, 7.08 lead — **fails** |

| Outcome | Applies? |
|---|---|
| GO — both hands | No. Two hands are resolved on 15% of frames |
| PARTIAL — lead only | No. The lead hand is the one that is missing (0.14) |
| PARTIAL — trail only | Reading detection per-side rather than as the two-hand rate, the trail hand clears detection (1.00) and placement (1.00) and still **fails stability** at 3.14. And `thresholds.md` wrote down in advance that a trail-only pass is *not* a grip-strength milestone |
| **NO-GO** | **This one.** The question closes |

The stability gate is the one worth arguing about, and arguing it down changes nothing. Its
denominator is the pose model's index landmark over a static golfer, which barely moves — the
absolute figures are 0.0148 sw/frame for the hand model against 0.0021 sw/frame for pose, so the
ratio is large partly because the divisor is small. A reader who wants to call 3.14 acceptable for
the trail hand can; the verdict does not move, because the lead hand is not there to measure.

## What this footage cannot say

Flagged in advance in `thresholds.md` and still true. One golfer, one grip, one bay, fifteen
swings — a no-go here is a no-go *for this capture*, and the honest generalisation is that a model
trained on open gesturing hands does not resolve two interlocked ones, which is a property of the
model rather than of the light. Nothing here says whether a grip is *good*: that needs a coach's
read on the same swings, and no data in this repo has one.

And a GO would not have produced a band. GolfDB's clips are 160×160 crops (ADR-012) where a hand
spans a few pixels, and the videos are not even on disk — only annotations and extracted pose. Any
metric out of this path would have been `signal` with no distribution behind it, like M14's four.

## What would change the answer

Not a re-run and not a bay session. A hand model would have to be trained or fine-tuned on
interlocked hands on a club, which is a labelling effort of the same shape M1.5 declined for the
club head — and it would still be answering "where are the knuckles", with the coaching claim
("is this grip strong or weak?") unvalidated behind it. The cheap thing that *is* still open is
the down-the-line view at a different camera height, but the control run below argues against it.

## The control runs

**Down-the-line, address (15 clips, 178 frames)** — worse in every column, best crop k=2.0:
≥1 hand 0.26, two hands 0.17, placement 0.62. §Phase G killed the lead hand in that view with the
pose model and the hand model does not rescue it. The control behaving worse is what says the
face-on numbers are the instrument and not the probe.

**Whole clip, face-on (15 clips, 3357 frames)** — ≥1 hand 0.66, two hands 0.08, lead 0.12, trail
0.53. As expected: M14 already watched the coarse pose hand points fall from 1.00 at address to
0.63–0.68 over a clip, and this is the harder model. The jitter ratios print below 1.0 here and
mean nothing — the denominator is the pose model over a *moving* golfer, so both terms are large.

## Re-running

```bash
.venv/Scripts/python.exe spikes/mediapipe-hands/probe.py measure                     # the table
.venv/Scripts/python.exe spikes/mediapipe-hands/probe.py measure --view down-the-line
.venv/Scripts/python.exe spikes/mediapipe-hands/probe.py measure --window clip --k 2.0
.venv/Scripts/python.exe spikes/mediapipe-hands/probe.py measure --k 2.0 --min-confidence 0.2
.venv/Scripts/python.exe spikes/mediapipe-hands/probe.py frames --k 2.0
```

Needs the `vision` extra and the package installed; it reads the stored artifacts through the
shipped code. The model bundle downloads once into `data/models/`. `frames/` is git-ignored —
white crosses are the pose wrists, dots are the hand model, and the picture worth looking at is
that there is only ever one colour of dot.
