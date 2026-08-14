# M1.5 pass/fail thresholds — committed BEFORE any measurement

Written 2026-08-14, before a single frame was extracted. The M7 Phase 0 spike established the
rule this follows (`docs/M7_TWO_PHONE_SPIKE.md`: *"Every threshold in this document was written
before any footage existed, which is the point: the pass/fail bar for the biggest risk is not
allowed to be chosen after seeing the numbers."*). M1.5 has the same shape of risk — a "yes it's
detectable" chosen after looking at an encouraging frame would authorise 200–500 images of
labelling on a hunch.

## The question these decide

Not "can a human see the club" — a human can see almost anything. **Is the club head a bounded,
localizable object that a detector could be trained to put a box around, in the frames that
matter?** M2's whole labelling effort rests on that, and the frames that matter are the impact
window, where the head is fastest.

## Definitions

- **Impact window** — impact frame ±4, at native resolution (2160×3840), no downscaling.
- **Head extent** — the short-axis pixel width of the club head, measured on the head itself and
  not on the shaft.
- **Sharpness ratio** — variance-of-Laplacian over the moving region ÷ the same statistic over a
  static background patch in the same frame. A rigid object frozen by a short exposure scores
  near 1.0; a smear scores near 0. This is the smear-vs-object discriminator, and it is a ratio
  so that it cancels the scene's own texture and exposure.

## The bar

| Outcome | Rule |
|---|---|
| **GO — pure ML (YOLOv8, ADR-005 as written)** | Head localizable in **≥3 consecutive frames** of the impact window, with short-axis extent **≥20 px** and sharpness ratio **≥0.5** |
| **NO-GO pure ML → marker-assisted** | Head localizable at address/top (**≥20 px**) but sharpness ratio **<0.3** through impact, or its extent is an unbounded streak rather than a bounded object |
| **NO-GO both → fusion + interpolation** | Head not separable from the background even at rest, or **<20 px** at rest |

**20 px** is the short-axis floor because a detector trained at a typical 1280 px inference size
sees a 2160-px-wide frame downscaled ~1.7x; a 20 px head arrives as ~12 px, which is at the edge
of what a YOLO-class detector resolves. Below that, native-resolution tiling would be required
and the labelling cost rises sharply.

**0.5 / 0.3** bracket the middle case deliberately. Between them the answer is "marginal, and the
decision needs the lighting test rather than this footage."

## Recorded in advance: what this footage cannot answer

This is 60 fps iPhone footage shot in an indoor bay under ambient light, with auto-exposure. It
can answer *whether the club head survives our current capture*, and it can be used to back out
**what exposure would be required**. It cannot answer whether a bright light plus a forced fast
shutter freezes the head, because no such clip exists — that needs a capture we cannot make
today. Any conclusion about the fast-shutter path is therefore a **specification derived from
measurement**, explicitly not an observation, and the ADR must say so.
