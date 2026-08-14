# ADR-018: Bay Lighting — buying the exposure ADR-017 asked for

## Status
Accepted

## Date
2026-08-14

## Context

[ADR-017](017-club-head-detection-strategy.md) closed M1.5 as a no-go and left exactly one
question open: **does bright light plus a forced fast shutter actually freeze the club head?** It
also supplied the number — **~1/2000 s** — and said the thing to buy is light, not a
global-shutter camera, because shutter *type* does not appear in a blur calculation at all.

This ADR is the purchase that follows. It is deliberately small: the point is to answer a
question for ~$200, not to build a lighting rig.

### What the exposure actually costs in light

At the iPhone main camera's ~f/1.8, holding 1/2000 s needs roughly:

| ISO | Lux on the impact zone |
|---|---|
| 400 | ~4,000 |
| 800 | ~2,000 |
| 1600 | ~1,000 |

A sim bay runs perhaps 200–500 lux, so the gap is **4–10x — and only on the ball**, not the whole
room. That is a correction to ADR-017's "~30x more light", which held ISO constant. Going
1/60 s → 1/2000 s is 5 stops; light and ISO can each pay some of them. The verdict does not move,
but the purchase gets cheaper.

### Constraints that are not about light

- **The bay is a facility, not our space.** Anything bought has to arrive, set up in a couple of
  minutes, and leave. A permanent mount or a mains run across a floor is not available to us.
- **Flicker is the failure that wastes the trip.** At 1/2000 s a PWM-dimmed LED can band across
  frames, and the result looks like a camera fault rather than a lighting one. This is the spec
  most products do not publish.
- **Light alone is inert.** Stock iOS cannot set a shutter speed, so without a manual-shutter app
  a brighter bay yields a brighter video with an identical smear. **Blackmagic Camera (free)** is
  a prerequisite of this purchase, not an accessory to it.

## Options Considered

### Option A: One Neewer MS60B, 65 W COB, ~$170 (battery kit ~$200)
- **Pros**: 40,000 lux @ 1 m with the reflector, and **the manufacturer publishes a flicker-free
  rating at 1/2000 s** — our exact shutter, which almost nobody states. Bowens mount, runs on
  2× NP-F970 so no cable crosses the bay floor. At 2 m on battery it still clears ISO 100–200.
- **Cons**: on battery it is power-limited to 48 W (33 W on one cell), so ~29,500 lux rather than
  40,000 — which sets the placement at 2 m rather than 3 m. Fan noise; a stand is extra.

### Option B: Godox SL60IID daylight, ~$139
- **Pros**: cheapest credible option; daylight-only is fine here, since nothing about this needs
  colour matching.
- **Cons**: **no published flicker-free figure at 1/2000 s.** That is the one spec the trip turns
  on, and finding out at the bay costs a session. Saving $30 to gamble the thing we came for is a
  bad trade.

### Option C: Two lights, ~$340
- **Pros**: fills the shadow the golfer casts over the ball, and buys about a stop.
- **Cons**: double the cost and **double the setup** in a space where setup friction is the real
  constraint. The shadow problem is hypothetical until one light has been tried.

### Option D: Bigger single — Neewer CB120B / FS150B, ~$250–300
- **Pros**: 72,000–78,000 lux, generous headroom, still one stand.
- **Cons**: the headroom is largely wasted, because **the light must run near 100% anyway** —
  dimming is exactly what produces PWM banding at a fast shutter. Heavier to carry to a facility.

### Option E: Buy nothing; test with the free app first
- **Pros**: costs $0 and directly measures the gap — force 1/2000 s at the bay and read the ISO
  the app needs.
- **Cons**: not actually an alternative. It is the **first step of Option A**, and it cannot be
  run until the next bay visit, which is not scheduled.

## Decision

**One Neewer MS60B with the battery kit, plus a light stand.** Chosen on a single spec that the
alternatives do not publish: flicker-free at 1/2000 s. Placement, output and camera settings are
written into [BAY_SESSION_RUNBOOK.md §8](../BAY_SESSION_RUNBOOK.md) rather than here, because
that is the document that gets read standing in a bay.

**Blackmagic Camera is part of the decision, not an accessory.** Without a forced shutter the
light changes nothing measurable, and it is free.

Run the light at or near 100% and control exposure with ISO and distance. Reflector, not softbox
— diffusion costs 2–3 stops and hard light *helps* here, sharpening the club head's edges rather
than wrapping them.

## Consequences

**Easier.** The one open M1.5 question becomes a ten-minute errand on a trip that was going to
happen anyway, instead of a blocked milestone. If it passes, ADR-017 reopens and M2's labelling
effort becomes startable; if it fails, the fusion + interpolation path stands and the answer cost
one session and one light rather than 500 labelled images.

**A payoff that does not depend on M2 happening at all.** A shorter exposure sharpens the
*hands*, not just the club. This repo already records that wrists jitter 6x more than hips and
that 14.5% of frames fail the visibility gate; motion blur is a plausible contributor. Any
improvement there lands on the swings already being scored, through the existing pipeline.

**What this cannot buy, at any price.** Lighting fixes blur *within* a frame. It does not change
how often the camera samples: at 60 fps the head still crosses the impact zone in about three
frames, travelling 5.6–9.2 head-lengths between them. Sharp heads in three frames beats smears in
zero, but a dense club path is not available from a 60 fps phone. Shooting 240 fps slo-mo trades
resolution for frames — the head drops to ~21 px at 1080p, right on the 20 px floor — and the
slo-mo software path is still untested (M7 Phase 0 Q3).

**Risks, recorded so they are not surprises.** The facility may not permit a light stand — worth
asking before ordering. Banding may appear anyway and need the shutter nudged off the light's PWM
beat. And at ISO 400+ iPhone noise may hurt small-object detection even once the blur is gone;
that would show up as a marginal rather than clean result against the threshold.

**Not re-litigated per swing.** This buys a *test*, not a capture standard. The routine per-swing
loop stays at 1080p60 for the upload-latency reasons in the runbook; the fast-shutter clips are a
separate deliberate capture.
