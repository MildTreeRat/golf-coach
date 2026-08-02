# ADR-013: Clip-Relative Detection Windows and Explicit Detection Confidence

## Status
Accepted

## Date
2026-08-02

## Context

[ADR-012](012-golfdb-reference-data.md) validated our phase instants against GolfDB's 461
hand-annotated face-on clips and found the top of the backswing was being mislocated by a median of
26 frames. Rebuilt around the earliest major descent, top and impact now land within **2 and 1
frames**. Address did not follow: it sat at **9 frames, with 46% of clips more than 10 out**, and
[Phase B5](../M4_POSE_BAKEOFF.md) established that the two constants controlling it were already at
the centre of their plateau. The ROADMAP recorded the conclusion: further gains need a *different
rule*, not a better constant.

Three facts framed what that rule had to be.

**The error was concentrated in slow-motion.** Split by capture speed, the same rule scored a median
of **5 frames on real-time clips and 17 on broadcast slow-motion** ones, and the corpus is ~47%
slow-motion. A pooled frame median over that mix substantially measures the mix.

**The rule contained exactly one fps-dependent absolute.** `_MOTION_STALL_FRAMES = 4` required four
consecutive still frames. A downswing is ~8 frames in a real-time clip and ~30 in a slow-motion one,
so four frames meant something four times different across the two halves of the corpus — and a
gradual takeaway spends long stretches below any fraction of its own peak speed, so the backward
walk stopped *mid-takeaway*. The signature was unmistakable: signed error median **+4 frames, with
66% of clips detected late**.

**The failure path was worse than the failures.** When the wrist never settled, the rule returned
frame 0. That is not a neutral answer — GolfDB clips carry a median of **59 frames** of pre-roll
(p90 254) before the golfer is set. It fired on 11% of clips and cost them a median of 31 frames.

A fourth fact set the ceiling. `address = top - 3.5 x (impact - top)`, using **no pose signal at
all**, scores a median of 11 frames. Whatever the lead wrist contributes here is real but small.

## Options Considered

### Option A: Keep calibrating the existing constants
- **Pros**: Zero risk, no new concepts, the sweep harness already existed in spirit.
- **Cons**: Phase B5 already showed the constants sit on a plateau — the 2D grid is flat at 9 frames
  across frac 0.05–0.06 and stall 3–6. There is nothing left to win, and the grid argmin is a
  known edge-fit trap. This is the option the ROADMAP explicitly ruled out.

### Option B: A different pose signal
Six families were implemented and scored against all 461 clips:

| family | median | why it fails |
|---|---|---|
| setup-ball displacement from a density-clustered anchor | 13–17 | displacement is fps-invariant where speed is not, but a gradual takeaway leaves a fixed radius *late*; shrinking the radius hits the pose noise floor |
| noise-floor threshold (median + k·MAD of setup speed) | 41–47 | right intuition — setup jitter and swing peak are unrelated quantities — but without a persistence test any slow mid-takeaway frame trips it |
| takeaway-ramp back-extrapolation | 14–15 | the textbook onset estimator; removes the late bias but the wrist's early takeaway is not linear in speed, so the fit follows the chosen band |
| torso / hip motion energy | 19 | at 160x160 the torso barely moves during a takeaway |
| shoulder-line rotation | 24 | conceptually the best candidate — a waggle moves hands, not shoulders — but the shoulder line is too short a baseline for a stable angle at this resolution |
| directional persistence (net displacement / path length) | 32 | amplitude-free, so it should have been immune to slow motion; defeated because `smooth_keypoints` is a centered moving average that makes setup jitter look directionally persistent |

- **Pros**: Would have addressed the root cause if any had worked.
- **Cons**: All worse than lead-wrist speed. Upper-body motion energy merely *ties* it (8.0) at the
  cost of eight extra landmarks. The wrist is simply where the takeaway shows first.

### Option C: Scale the detection window to the clip's own time base, and bound the fallback
- **Pros**: Removes the only fps-dependent absolute in the path. Correct by construction across
  frame rates, which the product needs regardless of GolfDB — users shoot 30/60/120/240fps.
  Costs no new landmarks, no new dependency, and stays inside the stdlib core (ADR-008).
- **Cons**: A modest gain. Does not touch the fundamental difficulty of the instant.

### Option D: A learned model
- **Pros**: The real headroom. SwingNet reaches 31.7% PCE against our 15.8%.
- **Cons**: Conflicts with [ADR-008](008-project-structure.md)'s stdlib-only analysis core, and
  ADR-012 §2 commits us to not running GolfDB's weights. Deserves its own ADR.

## Decision

**Option C**, expressed as three principles rather than as one fix, because each generalizes past
this instant.

### 1. Detection windows are expressed in the clip's own time base, never in absolute frames

The quiet run is now `_MOTION_STALL_FRACTION` (0.25) of the detected downswing duration
`(impact - top)`, floored at 2 frames — the clip's own time base, taken from the two instants we
locate to within 2 and 1 frames. `_MOTION_QUIET_FRAC` was already a fraction of the clip's own peak
speed and is unchanged.

Any future detector reading a temporal window inherits this. The one remaining absolute in the
address path is `smoothing.py`'s 5-frame window, which the ROADMAP already flags for re-tuning.

### 2. A detector that fails says so

`PhaseSegment` gains `detected: bool = True`. When the wrist never settles, `_motion_start` returns
the bounded estimate `top - 3.5 x (impact - top)` and marks the ADDRESS and BACKSWING segments
`detected=False`. The ratio is GolfDB's own median over 1,399 clips (ADR-012), not a book value.

The flag exists because that estimate is **fit for one purpose and not the other**. It is derived
from an assumed tempo ratio, so:

- **Posture checkpoints use it.** They only need the boundary to place a sampling window.
- **`evaluate_tempo` drops its score.** Scoring it would report the assumption straight back as an
  observation — a wrong score wearing the clothes of a measurement. This is
  [ADR-010](010-benchmark-ranges.md) §2, "no score beats a wrong one", applied to a boundary rather
  than to a missing band.

### 3. Measurement windows are anchored to boundaries, not bounded by them

`head_sway` and `finish_balance` read a short window **ending at** the address boundary
(`_address_sample_bounds`) rather than the whole ADDRESS phase. The phase runs `[0, motion_start]`,
and frame 0 is not address — it is wherever the clip begins.

This is the more valuable half of the decision, and it inverts the problem. A window a few frames
long that moves with the boundary lands on genuine setup even when the boundary is 7 frames off; a
window that *starts at frame 0* inherits everything the clip happens to contain. Posture stops
needing address to be accurate.

## Consequences

### The instant improved, modestly and measurably

| | median | mean | >10 frames | med_norm | PCE |
|---|---|---|---|---|---|
| before (fixed 4-frame stall, frame-0 fallback) | 9.0 | 27.2 | 46% | 0.133 | 14.3% |
| + clip-relative stall | 8.0 | 24.1 | 41% | 0.129 | 15.0% |
| **+ bounded fallback (shipped)** | **7.0** | **22.9** | **40%** | **0.122** | **15.8%** |

Top and impact are unchanged at 2 and 1 frames, as they must be — this touches neither.

### Posture got a real correction, and it showed up on our own footage

On `golf_swing-aaron-1`, `head_sway` moves from **1.21 to 0.36** shoulder-widths — a hard fail
against the 0.43 band becomes a comfortable pass. The old window averaged the head across 459
frames, and the golfer's head sits **+0.41 shoulder-widths** off its setup position early in that
clip, settling only by frame ~400. The metric was reading the walk up to the ball as swing sway.
This is the same class of defect ADR-012 found in top detection: a plausible number that was
measuring the wrong thing, invisible until something independent was pointed at it.

Across the corpus the head baseline error against a tight true-address window falls at the tail —
p90 **0.135 to 0.090** shoulder-widths, clips over 0.10 from 16% to 9%, over 0.21 from 5% to 2%.

**One claim did not survive.** The shoulder-width ruler was expected to improve and did not: 10% of
clips off by more than 10% before, 11% after. That error is pose noise, not window content, so it is
still there and still divides both checkpoints.

### 14% of clips lose their tempo score

This is the most contestable consequence and it is a deliberate trade: a dropped checkpoint is
visible, a circular one is not. It is also a *reporting* change, not only a quality one — the
fallback used to fire silently.

### The metric definitions version had to move, 2 → 3

Changing where a metric samples changes the metric. Per ADR-012 §4 the bands were re-derived rather
than assumed still valid: `head_sway_norm` p90 **0.42 → 0.43**, `finish_balance_norm` p90
**0.28 → 0.29**. Small, but the point of §4 is that this drift gets recorded instead of quietly
invalidating every comparison.

### Limits we accept

- **The pose signal contributes ~2–4 frames over knowing nothing.** A constant tempo prior scores
  11; we score 7. That is the honest size of what the lead wrist tells us about takeaway onset.
- **Slow-motion is still 4x harder** (median 17 against 4). The clip-relative stall narrowed the
  gap but did not close it.
- **We are at 15.8% PCE against SwingNet's 31.7%.** The remaining headroom is a learned-model
  problem, i.e. Option D and its own ADR.
- **The synthetic test fixtures cannot prove the fps-invariance claim** — their setup is perfectly
  still, so both the old and new rules find it. The corpus harness is the evidence; the unit tests
  guard the contract.

## References
- [ADR-008](008-project-structure.md) — stdlib-only analysis core
- [ADR-010](010-benchmark-ranges.md) — benchmark ranges; §2 "no score beats a wrong one"
- [ADR-012](012-golfdb-reference-data.md) — GolfDB tiers, ground truth, §4 metric-definition versioning
- [docs/M4_ADDRESS_DETECTION.md](../M4_ADDRESS_DETECTION.md) — the feature flow
- [docs/M4_POSE_BAKEOFF.md](../M4_POSE_BAKEOFF.md) — Phase B6, the measurement ledger
- `scripts/golfdb/tune_address.py` — every rule above, still runnable
