# M1.5 club-head detectability — findings

**Run 2026-08-14 against the four bay clips already on disk.** No new footage was captured; the
spike needed none. Thresholds were committed in [thresholds.md](thresholds.md) before a single
frame was extracted. Decision recorded as
[ADR-017](../../docs/decisions/017-club-head-detection-strategy.md).

**Verdict: NO-GO on pure ML, and the reason is not the detector.** The club head is a perfectly
good detection target at rest — 42 px across in 4K, crisp, high contrast. It is destroyed by
**exposure time**, and no choice of detector, marker or camera *shutter type* changes that. The
gate on M2 is light, not machine learning.

## What was measured

Four clips, two swings, face-on and down-the-line, 2160×3840 at ~60 fps. Impact frames were read
from the stored `analysis.json` rather than re-detected, so these are the exact frames the
shipped pipeline calls impact.

| | aaron-1 | aaron-2 |
|---|---|---|
| face-on impact frame | 718 of 926 | 903 of 1162 |
| down-the-line impact frame | 1574 of 2472 | 1797 of 3211 |
| ball diameter, face-on | 56 px | 57 px |
| **ball-plane scale** | **1311 px/m** | **1335 px/m** |
| body-plane scale (vertical) | 803 px/m | 805 px/m |
| club head at rest, short axis | — | **42 px** (3.1 cm) |
| shot's club-head speed | 91.0 mph | 98.3 mph |

## Three things found by running it rather than reasoning about it

**1. The scale that matters is the ball's, not the golfer's — and they differ by 1.66x.** The
first cut normalised by the golfer's body, which is what the rest of this repo does. Wrong ruler:
the club head at impact is in the **ball's** plane, and the ball sits closer to a slightly
downward-looking phone than the golfer does. The ball measures 57 px where a body-plane ruler
predicts 34. Every blur figure computed on the body plane is therefore 40% too small. The ball is
also the better ruler on its own terms — 42.7 mm by rule, against an assumed shoulder width.

**2. Shoulder width is not a ruler in the down-the-line view, and the reason is already written
down in this repo.** Measured face-on it gives 887 and 883 px/m across the two swings; measured
down-the-line it gives 198 and 112. From behind the golfer the shoulders are edge-on and
biacromial width projects to almost nothing. This is the same failure M6.5 found in
`head_hip_offset_impact_norm` — *a quantity comparing two body parts at one instant does not
survive a change of camera yaw*. The fix is the same shape: use a **vertical** extent, which yaw
leaves alone. Shoulder-to-ankle gives 803/805 face-on and 1109/1079 down-the-line, agreeing with
itself across swings in both views.

**3. The pre-committed sharpness metric turned out to be uninformative, and is recorded as such
rather than quietly replaced.** `thresholds.md` defined sharpness ratio as variance-of-Laplacian
over the moving region ÷ the same over a static background patch. Measured, it lands between 1.4
and 5.4 — i.e. the *moving* region always scores *higher*. The metric is comparing two different
things: the moving region is full of body and clothing edges, the static region is a smooth bay
mat. It measures scene texture, not blur, and it cannot discriminate what it was written to
discriminate. The verdict below therefore rests on the extent criterion and on direct inspection
of the frames, which is what the M1.5 checklist actually asks for. Recorded here because a metric
that failed is worth more written down than deleted.

## The blur arithmetic

The launch monitor's club-speed reading is **not trustworthy on either shot**: smash factor
(ball speed ÷ club speed) comes out at 1.00 and 0.92, and a real strike cannot exceed ~1.5 or
plausibly fall below ~1.1. So the true club speed is bracketed — from `ball_speed / 1.5` at the
low end to the reading as-printed at the high end — and the verdict is checked across the whole
bracket. It survives either way.

For aaron-2 at the ball plane (1335 px/m), against a head that is 42 px across at rest:

| club speed | between consecutive frames | smear at 1/60 s | at 1/1000 s | at 1/2000 s |
|---|---|---|---|---|
| 60.3 mph (low bracket) | 45 cm = 601 px = 5.6 head-lengths | 600 px | 36 px | 18 px |
| 98.3 mph (as read) | 73 cm = 979 px = 9.2 head-lengths | 978 px | 59 px | 29 px |

**Exposure needed to keep the smear under half a head-width (21 px): 1/1714 s to 1/2793 s.**
Across both swings and the full speed bracket, the requirement is **~1/2000 s or faster**.

At the 1/60 s the phone actually used, the smear is **14x to 23x the head's own size**.

## What the frames show

Extracted with `probe.py frames`; the impact zone is derived from the golfer's wrists and ankles
so the crop follows the golfer rather than being hand-tuned per clip.

- **Address (826):** club head crisp and unambiguous, 42 × 97 px, sitting behind a crisp ball.
  This is a frame anyone could label.
- **Impact −4, −3, −2 (899, 900, 901):** ball crisp on the mat, **club not in the impact zone at
  all** — still above the crop.
- **Impact −1 and impact (902, 903):** the club appears as a **broad translucent band** sweeping
  the whole zone. You can see the mat and the golfer's leg *through* it. There is no bounded
  head, no leading edge, nothing to draw a box around.
- **Impact +2 (905):** club gone, and the ball is gone too.

So the club head is inside the impact zone for **about three frames of the sixty in that second,
and is a labelable object in none of them.**

## Against the committed thresholds

| Threshold | Result |
|---|---|
| GO — pure ML: head localizable ≥3 consecutive frames, ≥20 px, sharpness ≥0.5 | **Fails.** Localizable in **zero** frames of the impact window |
| NO-GO pure ML → marker: localizable at rest (≥20 px) but unbounded through impact | **Matches.** 42 px at rest, unbounded translucent band through impact |
| NO-GO both → fusion: not separable even at rest, or <20 px | Does not apply — at rest it is separable and 42 px |

The committed table lands on **marker-assisted**. The measurement says that is not sufficient on
its own, and ADR-017 records why: a marker raises *contrast*, and the thing destroying the head
is *exposure*. A reflective marker at 1/60 s smears across the same 600–980 px; it would be a
brighter streak, easier to threshold, and still not a per-frame head position. The marker path
becomes viable **only in combination with** the ~1/2000 s exposure, at which point plain pure-ML
would work too. That is the finding the threshold table did not anticipate, and it is why the ADR
does not simply read the verdict off the table.

## What this footage cannot say

Flagged in advance in `thresholds.md` and still true: there is no fast-shutter clip on disk, so
the 1/2000 s figure is a **specification derived from measurement, not an observation**. What
would settle it is one clip of a swing under bright light with the shutter forced short — a
capture, not a purchase. Until then the go/no-go on M2's labelling effort stands as no-go.

## Re-running

```bash
.venv/Scripts/python.exe spikes/club-head-detectability/probe.py measure
.venv/Scripts/python.exe spikes/club-head-detectability/probe.py frames --swing aaron-2 --pad 4
```

`frames/` is git-ignored — the crops are large and regenerate in a minute from footage the
repo already has.
