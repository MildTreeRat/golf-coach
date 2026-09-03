# M14 — Hand Landmarks: six points nobody reads, and the head dots nobody measures

> **Tier: TARGET.** Nothing here is built. P3 is a measurement whose result is not yet known, and
> P4–P5 are explicitly conditional on it — so read this as a plan with a gate in the middle, not a
> description of the program. The governing prior art is
> [M4_POSE_BAKEOFF.md](M4_POSE_BAKEOFF.md) §Phase G (the landmark exclusion floor) and
> [ADR-017](decisions/017-club-head-detection-strategy.md) (the address-versus-motion distinction
> this milestone leans on).

**Status: 0/6 phases built.** P0 wrote this document. P1 fixes the overlay, P2 fixes the synthetic
fixture, P3 is the reliability screen that gates everything after it, P4 and P5 add measurements
with no bands, and P6 reconciles the docs against whatever P3 found.

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
5. **"The synthetic fixture will catch a broken hand metric."** It will not, and this is the trap
   most likely to ship a silent falsehood. `tests/analysis/conftest.py` initialises all 33
   landmarks to `(0.5, 0.5)` at `visibility=1.0` and then overwrites only the ones current metrics
   read. Landmarks 17–22 therefore **clear the visibility gate while parked at frame centre,
   detached from the wrists** — a new measure function would return a plausible non-`None` number
   computed from nothing, and `test_registry_is_complete_and_consistent` would go green while
   measuring garbage. **P2 exists solely to close this, and must land before P4.**

## The phases

P1 and P2 are independent of each other and of P3. P4 requires both P2 and P3.

| Phase | Scope | Principal files | Gate |
|---|---|---|---|
| P1 | The overlay draws what is measured | `pose/overlay.py` | — |
| P2 | The fixture places the hand landmarks | `tests/analysis/conftest.py` | — |
| P3 | Reliability screen at address, face-on | `scripts/hand_landmark_reliability.py`, this doc | **the gate** |
| P4 | Two position metrics | `analysis/measure.py` | P2 + P3 |
| P5 | Signed offset and the rotation proxy | `analysis/measure.py` | P4 |
| P6 | Docs reconciled against P3's numbers | `ROADMAP.md`, this doc, `WORKLOG.md` | — |

### P1 — The overlay draws what is measured

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

### P2 — The fixture places the hand landmarks

Place 17–22 relative to each wrist in the fixture's frame builder, reusing the existing
`_GRIP_OFFSET_X` / `_GRIP_OFFSET_Y` constants — whose comment already describes them as "a grip's
width down the shaft".

This is the **same defect already fixed once** in this file, for the trail wrist. The comment there
records the reasoning verbatim — a fixture that animated only the lead wrist "described a golfer
letting go of it" — along with a note about placing the change so existing lead-wrist expectations
stay untouched. Follow that precedent exactly, including the note.

*Verify:* the full suite, with every existing expectation unchanged. P2 should move no assertion.

### P3 — Reliability screen at address, face-on (the gate)

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

### P4 — Two position metrics (conditional on P3)

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

### P5 — Signed offset and the rotation proxy

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

### P6 — Docs reconciled against what P3 found

- Write an **addendum** to `ROADMAP.md`'s grip claim carrying P3's real numbers, narrowing or
  confirming it. Do not silently edit the original: this repo records corrections rather than
  overwriting them, which is why the addenda are the most-read part of `docs/decisions/`.
- Flip this document to **REFERENCE**, update its status line, and update its row in
  `docs/README.md` so the two counts agree.
- New **top** entry in `WORKLOG.md`.
- Run the doc-truth suite and fix only from its failures.

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
