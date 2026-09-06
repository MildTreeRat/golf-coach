# MediaPipe Hands pass/fail thresholds — committed BEFORE any measurement

Written 2026-09-03, before the model was run over a single frame. The rule comes from the M7
Phase 0 spike and was reused by M1.5: *"Every threshold in this document was written before any
footage existed, which is the point: the pass/fail bar for the biggest risk is not allowed to be
chosen after seeing the numbers."* The risk here has the same shape. A "yes, the hands come back"
chosen after looking at one encouraging frame would authorise an **L3** change — a parallel
optional field on `FrameKeypoints`, a `pose_estimator` stamp bump, and every cached pose in
`data/processed/` re-run — on a hunch.

## The question these decide

**Does MediaPipe's `HandLandmarker` return two correctly-placed, stable 21-point hands on a golf
grip, at address, from the face-on camera?**

Not "can it find a hand". It is a hand-tracking model and it will find hands. A golf grip is the
adversarial case for it: two hands interlocked around a shaft, fingers of one inside the other,
seen from a view where they overlap almost completely. The model was trained on open, gesturing,
separated hands. The failure this is looking for is not absence — it is **two detections collapsed
onto one physical hand**, or landmarks placed confidently in the wrong place.

M14 settled the neighbouring question and does not answer this one. MediaPipe Pose tracks its own
four coarse hand points (wrist, thumb, index, pinky) at address in this footage; that is why
`hand_separation_norm`, `hand_height_norm`, `hand_offset_from_hips_norm` and `trail_hand_roll_deg`
exist. Those say where the hands *are*. Grip **strength** — the V's, the visible-knuckle count —
needs the 21-point model, and nothing in this repo has ever run it.

## Definitions

- **Address window** — `analysis.measure.address_sample_bounds` over the trimmed clip, re-applying
  the stored `face_on_window` first. The same window M14 P3 screened in, reused rather than
  re-derived, so the two sets of numbers are about the same frames.
- **Shoulder-width (`sw`)** — the pose model's shoulder separation in pixels on the same frame; the
  repo's usual face-on normalizer. Every distance below is in `sw`, not pixels, so the numbers
  survive a change of camera distance.
- **Detected frame** — one where `HandLandmarker` returns ≥1 hand. **Two-hand frame** — exactly 2.
- **Placement** — a returned hand is *placed* when its landmark 0 (wrist) is nearer to one pose
  wrist than to the other **by a margin of half the pose wrist separation on that frame**. This is
  self-calibrating on purpose: it needs no hand-labelled truth, and it asks the only question that
  matters downstream — is this detection on the hand it thinks it is on, or on its neighbour?
- **Distinct** — on a two-hand frame, the two detections place to *different* pose wrists. This is
  the collapse failure, and it is the one a naive detection count would hide completely.
- **Jitter ratio** — median frame-to-frame displacement of `HandLandmarker`'s index MCP (landmark
  5) across the address window, divided by the same statistic for the pose model's INDEX landmark
  on the same hand over the same frames. A ratio because the golfer is not perfectly still at
  address and a raw number would be measuring their waggle; dividing by the instrument we already
  ship metrics from cancels it. Same reasoning as M1.5's sharpness ratio.

## Two arms, and the crop is not cheating

**Arm A — full frame.** The 2160×3840 frame straight into the model. This is the control and it is
expected to fail: the palm detector works on a heavily downscaled input, and a hand that is ~100 px
in a 4K frame arrives at a few pixels.

**Arm B — pose-guided crop.** A square centred on the midpoint of the two pose wrists, side
`k × sw`, resized to 512×512. This is the arm the gates apply to, because it is the arm a real
implementation would use: pose already runs first in this pipeline and already knows where the
wrists are, so cropping to them costs nothing and is the standard way this model is deployed.

`k` is swept over **{1.0, 1.5, 2.0, 3.0}** and the gates are applied to the **best** `k`, with the
`k` reported. Pre-registered here so it is not a post-hoc choice: crop scale is a free
implementation parameter and not a finding, in the way that "does the model see a grip at all" is a
finding. What is *not* allowed is sweeping anything else after seeing the table.

## The bar

Gates are applied per hand (lead and trail separately), on Arm B, over the address window, pooled
across every face-on swing on disk.

| Gate | Rule | Why this number |
|---|---|---|
| **Detection** | two-hand frames ≥ **0.60** of address frames | §Phase G's exclusion floor, reused unchanged by M14 P3's screen — so this row reads directly against that table |
| **Placement** | placed ≥ **0.90** of detected frames, **and** distinct ≥ **0.90** of two-hand frames | A grip metric reads one named hand. At 0.90 the failures are droppable frames; below it, the assignment itself is the thing being measured |
| **Stability** | jitter ratio ≤ **2.0** | "No worse than twice the frame-to-frame noise of the instrument this repo already cuts bands from." A grip angle is a small quantity and jitter is what would eat it |

| Outcome | Rule | What it authorises |
|---|---|---|
| **GO** | all three gates, **both** hands | An L3 milestone: parallel hand field on `FrameKeypoints`, estimator stamp bump, grip metrics as `signal` |
| **PARTIAL — lead only** | gates pass on the lead hand, fail on the trail | Still a milestone. Grip strength is read on the **lead** hand — the knuckle count and the V — so a lead-only pass is most of the prize |
| **PARTIAL — trail only** | the reverse | **Not** a grip-strength milestone. Written down in advance because the temptation on seeing one green column will be to call it a pass; the trail hand alone does not carry the coaching claim this is for |
| **NO-GO** | detection under the floor, or placement scrambled | The question closes. `ROADMAP.md`'s "grip is invisible to this instrument" stands, now measured rather than inherited, and M14's open successor is struck |

## Recorded, never gated

- **Handedness labels.** The model's own Left/Right call and its confidence. Not a gate, because
  this probe assigns by proximity to the pose wrists and never needs the label. If it is wrong on a
  grip that is a fact about MediaPipe worth writing down, not a blocker.
- **Whole-clip and impact-window detection.** Expected to collapse — M14 already saw the coarse
  pose hand points fall from 1.00 at address to 0.63–0.68 over the whole clip, and that is the
  easier model. Recorded so nobody re-asks.
- **Down-the-line, as the control.** §Phase G killed the lead hand in that view at 0.37–0.40 with
  the pose model. If the hand model does *better* there, something in this probe is wrong.

## Recorded in advance: what this footage cannot answer

- **Whether a grip is *good*.** This measures whether the landmarks exist and hold still. Grip
  strength is a claim about the V's and the knuckle count, and validating it needs a coach's read
  on the same swings, which no data here has. Exactly M14 P3's distinction between an exclusion
  floor and a relevance ranking, and it carries over unchanged.
- **A tour band. Ever, from GolfDB.** Those clips are 160×160 crops (ADR-012), where a hand spans a
  few pixels, and the videos are not even on disk here — only annotations and extracted pose. So a
  GO authorises `signal` metrics with no distribution behind them, like M14's four, and any band
  would need a corpus that does not exist. This is not a reason to skip the spike; it is a reason
  the spike cannot be followed by a checkpoint.
- **Anyone else's hands.** One golfer, one grip style, one glove, one bay light, fifteen swings. A
  pass is a pass *for this capture*, and the generalisation question is a bay session, not a rerun.
