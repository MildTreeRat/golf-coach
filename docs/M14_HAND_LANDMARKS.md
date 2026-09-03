# M14 — Hand Landmarks: six points nobody reads, and the head dots nobody measures

> **Tier: REFERENCE.** All six phases are built, so read the reasoning and not the digits — every
> fraction, tolerance and visibility number below is a measurement dated 2026-09-03, and the two
> scripts that produced them can answer differently on a bigger corpus. What is *not* a snapshot is
> the shape of the finding: the camera was the variable. The governing prior art is
> [M4_POSE_BAKEOFF.md](M4_POSE_BAKEOFF.md) §Phase G (the landmark exclusion floor) and
> [ADR-017](decisions/017-club-head-detection-strategy.md) (the address-versus-motion distinction
> this milestone leans on).

**Status: 6/6 phases built, closed 2026-09-03.** P0 wrote this document; P1 fixed the overlay, P2
fixed the synthetic fixture, P3 ran the gate, P4 and P5 added four metrics, and P6 reconciled the
docs — all on one day. **P3 passed**: all six hand landmarks track in 100% of address frames
face-on, and the down-the-line control reproduces §Phase G's failure on the same corpus, which
isolates the camera rather than the landmark. **All four metrics scored `signal`** against the
spread/error harness, and none of them has a band, a checkpoint or an `ANALYSIS_VERSION` bump — so
no stored score moved and nothing needed re-analysing. P5 turned up the one finding neither phase
went looking for: `hand_offset_from_hips_norm` is **bimodal** over GolfDB, which is the handedness
warning in `measure.py` firing for the first time. **What is unverified is the overlay in a
browser** — P1's pins are unit tests, stored `aligned.mp4` clips carry the old dots until
re-rendered, and nobody has looked at a freshly rendered one.

## What this milestone is

It began as "can we move the plot points from MediaPipe to better spots — fewer in the head, more
in the hands," with grip rating as the goal.

**The landmarks cannot be moved.** BlazePose's 33-point topology is baked into the trained model
weights; it is what the network learned to predict, not a configuration. `pose_model_variant`
(`config.py`) selects lite/full/heavy — the same 33 points at different capacities — and that is
the only dial there is.

But the question decomposes into two real findings, and both are actionable.

### Finding 1 — the drawn dots and the drawn bones have drifted apart

`pose/overlay.py` draws a green dot by looping over every member of `PoseLandmark` — **all 33**,
including four eye points, two mouth corners, two ears, a nose, and four foot points. Its `_BONES`
tuple connects **twelve**. Nothing reconciles the two lists, so the overlay renders eleven dots on
the head that no bone touches and no checkpoint measures, and it renders the six hand landmarks as
loose specks with no structure.

The head clutter the original question noticed is real, and it is a *rendering* defect. Fixing it
moves no numbers.

### Finding 2 — six hand landmarks are already on disk and nothing reads them

`PoseLandmark` indices 17–22 — `LEFT_PINKY`, `RIGHT_PINKY`, `LEFT_INDEX`, `RIGHT_INDEX`,
`LEFT_THUMB`, `RIGHT_THUMB` (`contracts/keypoints.py`) — are extracted by the estimator, carried
through `smoothing.smooth_keypoints`, and written into **every** `.keypoints.json` this repo has
ever stored. They appear in no bone, no `POSE_MEASUREMENTS` row, neither trajectory landmark list,
and no `_COCO17_TO_POSE` slot.

Every stored swing already contains hand data nobody has looked at. That is what makes P3 cheap:
the screen runs over artifacts that already exist, with no re-capture and no new dependency.

## The claim this milestone reopens

`ROADMAP.md` states the position twice — that causal coaching "is unreachable with this instrument
— grip, wrists and clubface are all invisible to it", and that grip and lead-wrist angle are
unmeasurable, so dispersion variance is used as a cause discriminator instead.

The evidence behind that is §Phase G of the pose bake-off, and **§Phase G does not say what the
roadmap says it says.** Its table is tracked-frame fraction over 584 **down-the-line** clips at
eight labelled instants:

- the **lead** thumb / index / pinky come in at **0.37 / 0.39 / 0.40** — below the 0.60 exclusion
  floor, which is the number the non-goal rests on;
- the **trail** thumb / index / pinky come in at **0.84 / 0.85 / 0.85** — comfortably *above* it,
  in the very view that is worst for hands;
- and §Phase G notes in passing that "face-on the same lead elbow tracks at 0.93" against 0.46
  down-the-line.

So the measured finding is narrower than the conclusion drawn from it: *the lead arm crosses the
body and the torso hides it, from behind*. It is an anatomy result about one camera. The trail hand
was never the problem, and the face-on camera — the one this program actually scores mechanics from
— has never been screened for hands at all.

ADR-017 reached a structurally identical verdict for club-head detection and was careful about the
same distinction: the club head is "a fine target *at address*", and what killed it was motion blur
at swing speed. Address is a different regime — static golfer, no blur, hands at their most
stationary.

**Grip is therefore untested here, not rejected.** P3 is the test. If it fails, this document says
so and the milestone stops at P2, which is a complete result.

**P3 ran on 2026-09-03 and the narrower reading was right.** Face-on at address, all six hand
landmarks track in 100% of frames on 15 of 15 swings; down-the-line, on the same swings in the same
window, the lead hand collapses to 0.25 and clears the floor on 4 of 15. §P3 has the tables. The
roadmap's claim was true of the camera §Phase G measured and false of the camera this program
scores from.

## Five things that will look obvious and are wrong

1. **"Move the landmarks to better spots."** Not possible. The topology is model weights. What is
   movable is which landmarks get *drawn* (P1) and which get *read* (P4–P5).
2. **"Fewer head points means delete the head."** No — `evaluate_head_sway` measures the **ear
   midpoint**, and `tests/analysis/conftest.py` explains at length why it is the ears and not the
   nose. P1 keeps exactly two head landmarks and draws the bone *between* them, so the overlay
   shows the quantity being scored. Eleven dots become two, and the two that survive are the
   measured ones.
3. **"The bake-off already settled hands."** It settled the **lead** hand **down-the-line**. See
   above. Re-read §Phase G before arguing from it — it is also explicit that it is "an exclusion
   floor, not a relevance ranking", which means passing P3 says a landmark is *visible*, not that
   it is *useful*.
4. **"Add a grip checkpoint."** A `CheckpointSpec` with no `ranges.json` row fails
   `tests/analysis/test_checkpoints.py::test_every_registered_checkpoint_has_an_evaluator_and_a_band`
   immediately. Bands are cut from a corpus *after* a metric has been measured across it — the
   M6.5 measure-before-judge order, of which `backswing_ms`, `downswing_ms` and
   `head_hip_offset_impact_norm` are the standing examples. This milestone adds **no checkpoint,
   no band, and no `ANALYSIS_VERSION` bump.**
5. **"The synthetic fixture will catch a broken hand metric."** It would not have, and this was
   the trap most likely to ship a silent falsehood. `tests/analysis/conftest.py` initialises all 33
   landmarks to `(0.5, 0.5)` at `visibility=1.0` and then overwrites only the ones current metrics
   read. Landmarks 17–22 therefore **cleared the visibility gate while parked at frame centre,
   detached from the wrists** — a new measure function would have returned a plausible non-`None`
   number computed from nothing, and `test_registry_is_complete_and_consistent` would have gone
   green while measuring garbage. **P2 closed this on 2026-09-03**; it is recorded here rather than
   deleted, because the same trap is one line away for any landmark the fixture does not place.

## The phases

P1 and P2 are independent of each other and of P3. P4 requires both P2 and P3.

| Phase | Scope | Principal files | Gate |
|---|---|---|---|
| P1 | The overlay draws what is measured ✅ | `pose/overlay.py` | — |
| P2 | The fixture places the hand landmarks ✅ | `tests/analysis/conftest.py` | — |
| P3 | Reliability screen at address, face-on ✅ **passed** | `scripts/hand_landmark_reliability.py`, this doc | **the gate** |
| P4 | Two position metrics ✅ | `analysis/measure.py`, `contracts/dispersion.py` | P2 + P3 |
| P5 | Signed offset and the rotation proxy ✅ | `analysis/measure.py`, `contracts/dispersion.py` | P4 |
| P6 | Docs reconciled against P3's numbers ✅ | `ROADMAP.md`, `docs/ARCHITECTURE.md`, this doc, `WORKLOG.md` | — |

### [x] P1 — The overlay draws what is measured

One idea: **derive the dot set from the bone list** rather than maintain a second list that can
drift from it. This is the rule `CLAUDE.md` already applies to the checkpoint panel — derive
membership, never restate it.

- Add **one head bone**, `LEFT_EAR — RIGHT_EAR`. Deliberate, not decorative: it renders the exact
  pair whose midpoint `head_sway` scores.
- Add **six hand bones** — `WRIST → INDEX`, `WRIST → PINKY`, `WRIST → THUMB` per side — giving a
  three-spoke fan at each hand. This is the minimum structure that makes hand tracking reviewable
  by eye, which is the prerequisite for trusting P3 at all.
- Replace the all-33 dot loop with a set derived from `_BONES`, so the two can no longer disagree.
- Leave the visibility gate, the colours and `annotate_frame` alone.

Comment *why* the ear bone is there, or the next reader will tidy it away as redundant.

Every caller (`pose/side_by_side.py`, `api/pipeline.py`, `scripts/run_pose.py`,
`scripts/analyze_swing.py`, `scripts/golfdb/spot_check.py`) goes through this renderer and needs no
edit — but **previously rendered `aligned` clips become stale.**

*Verify by looking at it:* run `scripts/run_pose.py` on one face-on clip. Two ear dots joined by a
bone; a three-spoke fan at each hand; no eye, mouth or foot dots.

**Built 2026-09-03.** `_JOINTS` is derived from `_BONES` (thirteen dots gone: nine head points, four
foot points), the ear bone and the two hand fans are in, and `tests/pose/test_overlay.py` pins both
halves — the topology in a tuple, and the ink on a canvas, because a drifted dot set still renders a
video that plays. The look was checked rather than assumed: address and impact frames of
`2026-08-23/9` were re-rendered from the stored `face_on.keypoints.json` and read at 3x, showing two
ear dots on one bone with no eye, nose or mouth dot, and a three-spoke fan on each hand — the gloved
lead hand included. The visibility gate, the colours and `annotate_frame` are untouched, and **every
stored `aligned.mp4` now shows the old dot set** until it is re-rendered.

### [x] P2 — The fixture places the hand landmarks

Place 17–22 relative to each wrist in the fixture's frame builder, reusing the existing
`_GRIP_OFFSET_X` / `_GRIP_OFFSET_Y` constants — whose comment already describes them as "a grip's
width down the shaft".

This is the **same defect already fixed once** in this file, for the trail wrist. The comment there
records the reasoning verbatim — a fixture that animated only the lead wrist "described a golfer
letting go of it" — along with a note about placing the change so existing lead-wrist expectations
stay untouched. Follow that precedent exactly, including the note.

*Verify:* the full suite, with every existing expectation unchanged. P2 should move no assertion.

**Built 2026-09-03.** All six ride with their own wrist, placed as `(along, across)` multiples of
the grip offset — `along` toward the club head, `across` perpendicular — so the fan is expressed in
the shaft direction the two wrists already define rather than as raw `dx`/`dy` that a retuned grip
offset would leave pointing off the club. The note the precedent asks for is in the code: the
placement sits beside the trail wrist's, not in the change that first reads it, so every existing
wrist, head, shoulder and hip expectation is untouched. It moved no assertion — 1417 green against
1414, all three of them new.

**P2 added a test, where the trail-wrist precedent did not.** That precedent could rely on the
down-the-line path to notice a regression; here nothing in `analysis/` reads 17-22 and nothing will
until P4, so dropping the placement would stay green for a whole phase and then surface as a hand
metric measuring frame centre. `tests/analysis/test_conftest.py` pins the three properties that
matter: the landmarks are a hand's reach from their wrist (checked at the top of the backswing,
where a landmark still at frame centre is 0.35 away — an assertion about *distance from the wrist*,
not about the literal value `(0.5, 0.5)`, which stops catching anything the moment a wrist passes
through the middle of the frame); the wrist-to-hand offset is identical in every frame, because
both hands stay on the club; and the six are six distinct points, since a fan collapsed onto one
point passes the other two and would let a metric reading the wrong landmark agree with the right
one.

### [x] P3 — Reliability screen at address, face-on (the gate)

A new script, modelled on `scripts/golfdb/tune_landmarks.py` — the same question that produced
§Phase G's table, asked of the other camera and a different window. Report in §Phase G's shape so
the two are directly comparable.

For each stored face-on swing under `data/processed/sessions/`:

- load with `storage.keypoints_io.load_keypoints`, which already tolerates both legacy artifact
  shapes;
- `smoothing.smooth_keypoints` → `phases.segment_phases` → **reuse
  `measure.address_sample_bounds`**. Do not hand-roll an address window: that helper deliberately
  *ends at* the address boundary rather than starting at frame 0, and its docstring explains why
  frame 0 is not address;
- per landmark 15–22, report mean visibility, p10/p50/p90 and **tracked-frame fraction** at the
  measure module's `MIN_VISIBILITY` — over the address window, and over the whole clip for
  contrast.

**The gate is §Phase G's 0.60 tracked-frame floor.** Apply the same number, for comparability.

If the hand landmarks fail it at address, **stop, and write the table into this document** — the
address regime was screened and also came back no-go. That is a complete and publishable result,
and shipping a metric anyway is exactly what ADR-010 §2 forbids. Do not proceed to P4.

Note what a pass does *and does not* license. §Phase G is explicit that this screen is an exclusion
floor, not a relevance ranking: it answers "is this landmark measurable from this camera", never
"does this landmark help". The second question is settled by fitting, and it is out of scope here.

**Ran 2026-09-03. It passed, and it was not close.** `scripts/hand_landmark_reliability.py` over
all **15 stored face-on swings** — every analyzed swing in the corpus, one right-handed golfer.
Tracked-frame fraction at `measure.MIN_VISIBILITY` (0.50), pooled over `address_sample_bounds`
(median 11 frames per clip), in §Phase G's shape. `clips ≥ floor` counts swings individually.

| landmark | lead (L) | trail (R) | clips ≥ floor (L \| R) |
|---|---|---|---|
| shoulder † | 1.00 | 1.00 | 15/15 \| 15/15 |
| elbow † | 1.00 | 1.00 | 15/15 \| 15/15 |
| wrist | 1.00 | 1.00 | 15/15 \| 15/15 |
| **thumb** | **1.00** | **1.00** | 15/15 \| 15/15 |
| **index** | **1.00** | **1.00** | 15/15 \| 15/15 |
| **pinky** | **1.00** | **1.00** | 15/15 \| 15/15 |

† Shoulder and elbow are controls, not candidates. §Phase G puts shoulders at 1.00 in the *harder*
view; anything else here would mean the script is reading the windows or the artifacts wrong, and
its hand numbers would be worth nothing. They came in at 1.00, and the elbow control is the sharper
of the two — the same lead elbow §Phase G measured at 0.46 down-the-line.

Fractions of 1.00 say the landmarks clear the gate but not by how much, so the **visibility
distribution** over the same window is what shows the margin:

| landmark | mean | p10 | p50 | p90 |
|---|---|---|---|---|
| lead thumb | 0.80 | 0.67 | 0.80 | 0.89 |
| lead index | 0.85 | 0.72 | 0.86 | 0.93 |
| lead pinky | 0.83 | 0.68 | 0.85 | 0.92 |
| trail thumb | 0.92 | 0.89 | 0.93 | 0.97 |
| trail index | 0.95 | 0.93 | 0.95 | 0.98 |
| trail pinky | 0.94 | 0.92 | 0.95 | 0.97 |

The lead hand runs about 0.10 below the trail hand — the anatomy §Phase G found, still faintly
present face-on — but its **p10 is 0.67**, above the 0.50 gate. The weakest hand landmark in the
corpus is not marginal at address; it is the thumb, at a tenth of a swing's worst frames.

#### The down-the-line control, which is the strongest part of this result

The same script, same corpus, same window, same code path, one flag apart. `--view down-the-line`
reads the down-the-line artifact and passes `TRAIL_WRIST` to `segment_phases`, because from behind
the lead wrist is the far arm (§Phase F).

| window | lead thumb / index / pinky | trail thumb / index / pinky |
|---|---|---|
| **face-on, address** | **1.00 / 1.00 / 1.00** | 1.00 / 1.00 / 1.00 |
| **down-the-line, address** | **0.25 / 0.25 / 0.25** | 1.00 / 1.00 / 1.00 |
| down-the-line, whole clip | 0.44 / 0.48 / 0.47 | 0.90 / 0.91 / 0.91 |
| *§Phase G, down-the-line, 8 instants, 584 clips* | *0.37 / 0.39 / 0.40* | *0.84 / 0.85 / 0.85* |

The last two rows are the harness checking itself, and they agree: our own 15 clips reproduce
§Phase G's lead/trail split at close to its magnitudes on a completely different corpus. They are
not identical and should not be — §Phase G samples eight labelled instants including the fastest
frames of the swing, this pools every frame of the clip — but the sides, the ordering and the
verdict all match.

That is what makes the top two rows a finding rather than a hope. **The same six landmarks, on the
same swings, in the same window, go from 0.25 to 1.00 when the camera moves.** The variable is the
camera, not the landmark and not the golfer. Down-the-line the lead hand clears the floor on
**4 of 15** swings; face-on, on 15 of 15.

#### What the whole-clip contrast says, and why P4 is scoped to address

Face-on over the entire clip the hands fall to **0.63–0.68** — still above the floor, but that is
where the fast frames are, and it is a different regime for the reason ADR-017 gave about the club
head. The margin at address (1.00) is not the margin during the swing (~0.65), so a hand metric
measured over a whole swing is a much weaker instrument than one measured at setup. P4 and P5 read
`address_sample_bounds` and nothing wider; a swing-wide hand metric is a separate question this
screen does not answer in the affirmative.

#### Three limits on this result, stated because the numbers look emphatic

1. **It is an exclusion floor, not a relevance ranking**, exactly as §Phase G says. A 1.00 means
   the landmark is *visible*, never that it is *useful*. §Phase E is the precedent: the `z` channel
   passed its screen and then lost the fit.
2. **15 clips, one golfer, one bay, one camera position.** §Phase G had 584 clips and 166 golfers.
   The per-clip column exists because of this — at 15 clips, a landmark tracked perfectly on nine
   swings and absent on six would pool to 0.60 and clear the floor while being unusable on 40% of
   swings. Here every landmark is 15/15, so the two readings agree, but the day this corpus grows
   past one golfer the pooled number alone stops being enough.
3. **Visibility is MediaPipe's own confidence, not ground truth.** A landmark can be confidently
   placed and wrong. The bake-off's standing caveat carries over: two estimators tend to fail the
   same way, so any confidence-based screen is generous.

### [x] P4 — Two position metrics

Rows in `POSE_MEASUREMENTS` with **no checkpoint and no band**, measured over
`address_sample_bounds`, reusing `shoulder_width` for the ruler and `midpoint_series` for
confident-frames-only midpoints.

| metric | unit | what it is for |
|---|---|---|
| `hand_separation_norm` | `shoulder_widths` | lead-to-trail wrist distance — also the reliability canary, since it should be near-constant while the club is held |
| `hand_height_norm` | `shoulder_widths` | hands midpoint against shoulder midpoint in `y` — hand position at address |

Refusals must draw only from `MEASUREMENT_REASONS`; a measure function that reports a judging
reason is caught by its own pin in `tests/analysis/test_measure.py`. Naming is likewise pinned: a
metric ends in `_norm` **iff** its unit is `shoulder_widths`.

Tests: one per metric against the ideal synthetic swing, plus a refusal test driving the hand
landmarks below the visibility gate to confirm `LANDMARKS_UNCONFIDENT` rather than a fabricated
number.

**Built 2026-09-03.** Both rows are in `POSE_MEASUREMENTS`, both read `address_sample_bounds` and
nothing wider, and neither has a checkpoint, a band or an `ANALYSIS_VERSION` bump. `separation_series`
is new beside `midpoint_series` and gated the same way; `_address_ruler` is the window-then-scale
guard pair the setup metrics share, deliberately *not* retrofitted onto the three existing functions
that open with it, because each interleaves a second `phase_bounds` call and folding it in would
reorder their guards and change which reason a broken clip reports.

**The plan named one file and the change needed three.** `contracts/dispersion.py` is the one it
missed: `tests/analysis/test_dispersion.py::test_every_production_metric_has_a_tolerance` asserts
that `POSE_MEASUREMENTS | SHOT_MEASUREMENTS` equals `METRIC_TARGETS` exactly, so a metric added to
`measure.py` alone fails the suite. That pin is doing its job — a metric with no row there is silent
in career mode, and silent for a reason nobody chose. The third is `tests/analysis/conftest.py`,
which gained a `_SHOULDER_Y` constant so the new expectations read off the fixture's own numbers
instead of a literal `0.4`; no value moved.

**The tolerances are measured, not judged.** `tune_spatial_metric.py` was re-run over all 461
face-on GolfDB clips on 2026-09-03, and the same run **reproduced all seven existing tolerances to
the digit** — which is the only available check that a number derived three weeks later came off the
same instrument.

| metric | n | p10 | p90 | spread | noise | bound | ratio | verdict |
|---|---|---|---|---|---|---|---|---|
| `hand_separation_norm` | 455 | 0.211 | 0.520 | 0.309 | 0.057 | 0.017 | 4.1 | signal |
| `hand_height_norm` | 455 | 0.951 | 1.739 | 0.787 | 0.049 | 0.020 | 11.5 | signal |

Both clear the harness's ratio-of-2 screen, so a band would be worth deriving — which is a
different sentence from "these are the right things to coach on", exactly as the harness says.
Note the `noise` column: `hand_separation_norm`'s **0.057 is the largest of any pose metric**, which
is the bake-off's "wrists jitter ~6x more" showing up where it was predicted. It clears anyway
because the population varies more than the jitter does.

**Two limits worth stating, because neither is visible in the table.** First, both metrics mix `x`
and `y` — the wrists are separated mostly *along the shaft*, which face-on is mostly vertical, so an
`|dx|` version would measure the residual and be dominated by jitter — so unlike the lateral family
they do **not** cancel the frame's pixel aspect. The offline path corrects for it from
`ReferenceSwing.pixel_aspect`; the live path has no per-clip aspect to correct with. Each says so in
its own `detail` string, which is the difference between declaring an assumption and inheriting one,
and it is the reason the pair ships unjudged. Second, P4 reads the **wrists** (15/16) and not
landmarks 17-22: `hand_separation_norm` is a wrist-to-wrist distance and `hand_height_norm` a wrist
midpoint. P5's `trail_hand_roll_deg` is the first thing in `analysis/` to read the fan P2 placed.

One doc consequence for P6: `docs/ARCHITECTURE.md` says "three of the eight metrics are refused the
tour join", and these two are a ninth and a tenth with no tour distribution behind either.

### [x] P5 — Signed offset and the rotation proxy

| metric | unit | what it is for |
|---|---|---|
| `hand_offset_from_hips_norm` | `shoulder_widths` | **signed** `x` of hands midpoint against hip midpoint — shaft lean, hands ahead or behind |
| `trail_hand_roll_deg` | `degrees` | angle of the trail wrist-to-index-knuckle vector off vertical — the grip-rotation proxy |

`hand_offset_from_hips_norm` is signed and therefore **camera-relative**, exactly like
`head_hip_offset_impact_norm`. Store the signed number and say so in its `detail`: handedness is a
*judging* problem, `mechanics.evaluate_head_stays_back` shows how this repo handles it, and it does
not belong in `measure.py`.

`trail_hand_roll_deg` is the speculative one. Face-on is not the ideal angle for grip rotation, and
this is a two-point proxy for a three-dimensional quantity. Label it a proxy in its `detail`. **If
P3's numbers for the index knuckle are marginal, drop it** and record that here rather than
shipping a metric the corpus cannot support.

**Built 2026-09-03.** Both rows are in `POSE_MEASUREMENTS`, both read `address_sample_bounds`, and
neither has a checkpoint, a band or an `ANALYSIS_VERSION` bump. `trail_hand_roll_deg` shipped —
§P3's index-knuckle numbers were not marginal (1.00 tracked at address, both hands), so the
conditional above did not fire. It is the **first thing in `analysis/` to read landmarks 17–22**,
which is what P2 placed the fixture's fan for.

`direction_series` is new beside `midpoint_series` and `separation_series` — the third member of
that family, gated the same way plus one gate of its own — and returns **unit** vectors, so the
caller averaging them gets a circular mean rather than a length-weighted one.

**Three design calls, each of which had a plausible wrong version.**

- **`trail_hand_roll_deg` takes no shoulder ruler.** An angle is already scale-free, so reusing
  `_address_ruler` (P4's shared window-then-scale guard) would have refused a side-on clip for
  `SCALE_UNAVAILABLE` over a ruler this measurement never divides by — a refusal naming a reason
  that was not the problem. It calls `address_sample_bounds` directly instead.
- **The average is circular, not arithmetic.** Angles wrap at ±180°, which for this vector is the
  hand pointing at the *top* of the frame. Two mis-detected frames either side of the wrap average
  to a confident straight-down under a mean of angles, and to the correct 180° under a mean of unit
  vectors. `test_trail_hand_roll_averages_around_the_wrap_not_through_it` is the pin.
- **A collapsed hand refuses rather than reporting 0.** `math.atan2(0.0, 0.0)` is `0.0` in Python,
  and 0° here reads as "the hand points straight down" — an entirely ordinary address value, so a
  fabricated one is indistinguishable downstream from a measured one. `MIN_DIRECTION_LENGTH` drops
  those frames and `MIN_DIRECTION_CONSENSUS` catches a whole window of them (ADR-010 §2).

**Both score `signal`, and one carries a finding the P4 pair did not.** `tune_spatial_metric.py`
re-run over the same 461 face-on GolfDB clips, which for the third time reproduced every prior
tolerance in `METRIC_TARGETS` unchanged — now all nine of them.

| metric | n | p10 | p90 | spread | noise | bound | ratio | verdict |
|---|---|---|---|---|---|---|---|---|
| `hand_offset_from_hips_norm` | 455 | -0.069 | 0.234 | 0.302 | 0.035 | 0.015 | 6.1 | signal, **bimodal** |
| `trail_hand_roll_deg` | 458 | 20.537 | 51.478 | 30.941 | 5.029 | 2.370 | 4.2 | signal |

**`hand_offset_from_hips_norm` came back bimodal — 26% of 455 clips negative — and that is the
handedness prediction landing.** `measure_head_hip_offset_impact`'s docstring has warned since
metric definitions v2 that a camera-relative sign over a mixed-handedness corpus splits into two
opposite populations, and that metric was then measured and found *clear* of it (458 clips,
consistently head-behind-hips). This is the first metric in the repo where the harness's bimodality
flag has actually fired. The consequence is specific: a band cut across both modes would sit in the
empty middle, and a left-handed golfer would read as a gross fault. That is why the row's
`no_target_reason` says handedness must be resolved before this has a target — and why the
*tolerance* is unaffected, since our error in measuring the number does not care which way the
golfer stands.

**The pair diverges in every way P4's matched.** `hand_offset_from_hips_norm` is `x`-over-`x` and so
**cancels** the pixel aspect that P4's two carry; `trail_hand_roll_deg` carries it a third way again
— an angle *rotates* with the aspect rather than scaling. One is signed and camera-relative;
the other is signed about image-vertical, which no handedness moves, but names the *right* side,
which handedness does. `phases.TRAIL_WRIST` is a right-handed convention, so both read mirrored for
a left-handed golfer, and that is the second reason the pair ships unjudged.

**One doc consequence for P6, on top of P4's.** `POSE_MEASUREMENTS` is now twelve rows, not eight,
and four of them have no tour distribution behind them.

### [x] P6 — Docs reconciled against what P3 found

**Ran 2026-09-03.** Documentation only — no source file changed, and the suite that was green
before this phase is the same suite that is green after it.

**`ROADMAP.md`'s grip claim is stated twice, so the correction is written twice and only once in
full.** The full addendum sits under *Biggest constraint on coaching*, which is where the sentence
"grip, wrists and clubface are all invisible to it" lives; the career-mode bullet gets a short one
that points at it. Neither original was edited — this repo records corrections rather than
overwriting them, which is why the addenda are the most-read part of `docs/decisions/`, and a
reader who arrives at the original claim from a link still meets it followed by its correction.

**Both addenda spend more words on what did not change than on what did**, deliberately. The
measured finding is narrow: the *hands are visible face-on at address*. Four things a reader could
reasonably take from that are false, and each is named — a visibility screen is an exclusion floor
and not a relevance ranking; it is an address result and not a swing one (0.63–0.68 over the whole
clip); grip *strength* still needs MediaPipe Hands; and wrists and clubface are untouched, because
a visible wrist landmark is not a lead-wrist *angle*. The career-mode bullet's addendum makes the
fifth point in the place it matters most: the dispersion-as-discriminator argument is **unaffected**,
since what became measurable is where the hands *are*, not whether the grip is *good*.

**`ROADMAP.md` had no M14 at all** — the milestone was designed in `docs/` and never entered on the
board, unlike M13 whose section its P0 wrote. So P6 added both the *Status at a glance* row and the
detail section, rather than only the addendum the plan named. A done milestone missing from the
board is the drift the board exists to prevent.

**`docs/ARCHITECTURE.md`'s count was wrong before this milestone and P4 predicted it.** §P4 flagged
"three of the eight metrics are refused the tour join"; the honest fix was not to write "twelve of
twenty" but to **stop stating the count** and state the *reasons*, which is what `CLAUDE.md`
requires of a set whose membership is derived. There are now **three** reasons where the sentence
said two, and the third is the new one: a pose metric that is measured but that nobody has cut a
distribution for. That is a different absence from a launch-monitor metric's — `NO_POPULATION` is a
row that could be derived tomorrow from the same GolfDB corpus the harness already ran over, while
`NO_LAUNCH_MONITOR_POPULATION` is a corpus that does not exist and would have to be acquired.

**One thing found by reading that code, and it is a trap for whoever cuts those distributions.**
`hand_offset_from_hips_norm` is refused the tour join today only because `load_distribution` returns
`None` for it. It is **not** in `contracts/comparison.py`'s `TOUR_COMPARISON_BLOCKED`, whose single
entry is `head_hip_offset_impact_norm` — and the reason that entry exists is *exactly* the property
P5 measured on this metric and that one does not have. So the day someone cuts hand distributions
from GolfDB, this metric silently becomes placeable in a bimodal population, which is the fault the
blocked dict was written to prevent. **It needs a `TOUR_COMPARISON_BLOCKED` row in the same commit
that gives it a distribution, not afterwards.** Recorded here rather than fixed, because a row
blocking a join against a distribution that does not exist is untestable and would read as dead
code; the reason it is safe to defer is that no distribution can appear without a deliberate
fitting run, and this paragraph is what that run should find.

**Doc-truth ran and passed.** The two pins this phase had to satisfy are
`test_the_map_and_each_phase_doc_agree_on_the_phase_count` (this document's status line and its
`docs/README.md` row are two hand-maintained counts of one thing) and
`test_the_map_agrees_with_each_doc_about_its_tier` (the REFERENCE flip has to happen on both sides
or neither). Both are pinned to each other rather than to a literal, so neither needed an edit.

**One piece of drift was found and deliberately left**, because P6's brief is to fix from doc-truth
failures and this fails nothing: `ROADMAP.md`'s career-mode paragraph still says the counter prints
**n = 2 for every metric** and that the tour join refuses **all nine** placements. The corpus reader
run against `data/processed/sessions` today returns 13 swings over 19 metrics. That prose went stale
somewhere between M9 and M13 and has nothing to do with hands; correcting it means re-deriving four
sentences of career-mode numbers, which is its own piece of work and is noted here so it is not
found a third time by accident.

## What this milestone is not

- **It is not MediaPipe Hands.** The 21-landmark-per-hand model is the only thing that could
  deliver grip *strength* — the V's and the knuckle count — and it has never been evaluated in this
  repo. It needs a `spikes/` probe first, on the `spikes/club-head-detectability/` + ADR-017
  pattern, because a golf grip is two interlocked hands wrapped around a shaft and that is out of
  distribution for a model trained on open, gesturing hands. It would also force an **L3** change:
  a parallel optional field on `FrameKeypoints`, never an extension of the flat 33-list, which both
  `smoothing.smooth_keypoints` and the estimator's fabricate-33-on-miss path assume — plus a
  `pose_estimator` stamp change so cached poses re-run. That is the natural next milestone if P3
  encourages it, and a firm no-go if P3 does not.
- **It is not a grip verdict.** No `CheckpointSpec`, no band, no score movement.
- **It is not grip consistency.** Repeatability across swings falls out of P4 and P5 for free once
  the numbers are stored, but computing it belongs with the dispersion work, not here.
