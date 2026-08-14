# ADR-017: Club-Head Detection Strategy — and the constraint that actually binds

## Status
Accepted

## Date
2026-08-14

## Context

[ADR-005](005-object-detection-yolov8.md) chose YOLOv8 to detect the club head and ball, and it
is the one model this project trains itself. It has never been built: `detection/detector.py`
raises `NotImplementedError("M2: implement YOLOv8 detection (after M1.5 spike).")`. The gate is
M1.5, a time-boxed spike whose entire purpose is to avoid labelling 200–500 images before anyone
has checked that the club head is visible in the frames that matter.

The question was always framed as a detection question — pure ML, or a marker on the club head,
or fusion from wrist position plus shaft angle. **The measurement says the choice between those
three is downstream of a constraint none of them addresses.**

The spike ran on 2026-08-14 against the four bay clips already on disk (two swings, face-on and
down-the-line, 2160×3840 at 60 fps). No new footage was captured. Pass/fail thresholds were
committed before any frame was extracted. Full method and numbers:
[spikes/club-head-detectability/](../../spikes/club-head-detectability/log.md).

### What was measured

- **The club head is a good detection target at rest**: 42 px across the short axis at 4K,
  crisp, high contrast against the mat, sitting behind a crisp ball. Comfortably above the 20 px
  floor below which a detector at typical inference resolution would struggle.
- **It is destroyed by motion blur at impact.** At the ball plane (1335 px/m, calibrated on the
  ball's known 42.7 mm rather than on the golfer), and across the full plausible club-speed
  bracket, the head smears **600–980 px at the 1/60 s the phone used** — 14x to 23x its own size.
  In the frames either side of impact it is a broad translucent band with no leading edge and
  nothing to bound.
- **The head is in the impact zone for about three frames**, travelling 5.6–9.2 head-lengths
  between consecutive frames. No frame catches it at the ball.
- **Exposure required to hold the smear under half a head-width: ~1/2000 s**, consistently across
  both swings and the whole speed bracket.

## Options Considered

### Option A: Pure ML — train YOLOv8 on the unmarked club head (ADR-005 as written)
- **Pros**: most general; no physical modification to the club; the model also gets the ball.
- **Cons**: **there is nothing to label in the impact window.** Labelling would produce boxes
  around a translucent band whose extent is set by exposure, not by the object — a model trained
  on that learns the smear, not the head, and cannot localise the head within it. The frames it
  *could* be trained on (address, top, follow-through) are the frames the club path least needs.

### Option B: Marker-assisted — reflective or bright tape on the club head
- **Pros**: the classic fix for a low-contrast target; colour thresholding alone can work; cheap.
- **Cons**: **it solves the wrong problem.** A marker raises *contrast*; the head is being
  destroyed by *exposure*. At 1/60 s a marker smears across the same 600–980 px — a brighter
  streak, easier to threshold, and still not a per-frame head position. It becomes viable only
  once the exposure is short, and at that point Option A works too.

### Option C: Fusion + interpolation — wrist position + shaft angle + a tracker, anchored by the
launch monitor's `club_path`
- **Pros**: **does not require the head to be visible at impact at all.** Uses signals this repo
  already has: MediaPipe wrist tracking on every frame, the shaft (which is visible and crisp
  well outside the impact window), and the simulator's own `club_path` as an anchor. Degrades
  honestly — it interpolates through the gap rather than claiming a detection.
- **Cons**: gives a *modelled* club path, not a measured one; inherits the wrist jitter this repo
  has already quantified; the launch-monitor anchor is itself OCR'd and sometimes wrong (both
  shots in this spike carry a physically impossible smash factor).

### Option D: Fix the capture — bright light plus a forced short shutter, then revisit
- **Pros**: the only option that makes the head actually visible at impact, which unblocks A and
  B together and improves C's anchors.
- **Cons**: needs ~1/2000 s, which is roughly five stops below the 1/60 s the bay currently
  supports — on the order of **30x more light**. Not a camera purchase: a lighting problem.

## Decision

**No-go on pure ML (Option A) at current capture. No-go on marker-assisted (Option B) as a
standalone fix.** Neither is a detector problem and neither is solved by a detector.

**The binding constraint is exposure time, and it is recorded here as a specification:
~1/2000 s at the moment of impact, needing on the order of 30x the light the bay currently
provides.** Everything else about club tracking is gated on that number.

**A global-shutter camera does not fix this.** ADR-003's 2026-07-02 addendum already said
"global shutter ≠ no motion blur"; this ADR supplies the number behind it. Global shutter removes
rolling-shutter *distortion*; blur is governed by exposure duration alone. **The M2 purchase
should therefore be evaluated on light and minimum exposure, not on shutter type** — and the
lighting may cost more than the camera.

**Until the capture changes, Option C (fusion + interpolation) is the only path that produces a
club path from what we can actually record**, and it is the one M2 should pursue if M2 is
pursued at all. It is explicitly a modelled path and must be surfaced as one — this repo does not
present a model as a measurement.

**The labelling effort in ADR-005 stays unstarted.** That was the decision M1.5 existed to
inform, and the answer is "not yet, and not for the reason we expected."

## Consequences

**Easier.** The M2 hardware question is now a number rather than an argument, and it is a
different number than the one being shopped for: minimum exposure and lux, not shutter type. A
purchase can be evaluated against it. The 200–500 image labelling effort is deferred on evidence
rather than on a hunch, and the evidence is re-runnable from footage already in the repo.

**Harder.** M2 has no cheap path. Fusion gives a modelled path, and every metric derived from it
inherits that status — which under [ADR-010](010-benchmark-ranges.md) §2 means it must be
distinguishable from a measured one everywhere it surfaces. The club-derived metrics that M4's
outcome axis wants stay out of reach.

**Unchanged.** Nothing in the shipped pipeline moves. This ADR records a decision about work not
yet started; `detection/detector.py` keeps raising, and its message stays accurate.

**Re-open when** a single clip exists of a swing under bright light with the shutter forced to
1/2000 s or shorter. That is a capture, not a purchase, and it flips Options A and B together.
The spike's harness re-runs against new footage unchanged — add the clip to `SWINGS` in
`probe.py` and the same table prints.

---

## Addendum — 2026-08-14: why the threshold table did not decide this

The thresholds committed before the measurement landed on **marker-assisted**, and this ADR
chose otherwise. Recorded because the discipline only means something if departures from it are
visible.

The table asked the right question — *bounded object or unbounded streak?* — and got a clean
answer: unbounded. What it did not anticipate was that the *cause* of unboundedness makes the
marker remedy inert. The table implicitly assumed the failure was contrast (which a marker
fixes); the measurement showed it is exposure (which a marker does not). The threshold was not
wrong, it was under-specified: it discriminated the symptom without discriminating the cause.

A future spike of this shape should commit a threshold on **the cause as well as the symptom** —
here, that would have been a smear-length-to-object-size ratio, which is computable from speed
and scale alone and would have pointed at exposure directly.
