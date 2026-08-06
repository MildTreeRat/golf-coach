# M4-REF: metric-definition and pose-estimator change record

> **Tier: REFERENCE — a change ledger, so it records superseded numbers by design.** This is the
> provenance target cited by `ranges.json`'s committed `source` strings and by nine code comments
> across `analysis/phases.py`, `analysis/checkpoints/mechanics.py` and `scripts/golfdb/*` — it is
> a live reference, not an archived doc.
>
> **Read it as history, not as current values.** Its whole purpose is recording what each number
> was *before* a change, so almost every band quoted below is intentionally out of date. The
> `Baseline` section is explicitly pre-change state. For current values, read
> `src/golf_coach/analysis/benchmarks/ranges.json`.
>
> **Use it when** a band looks wrong and you need to find which change moved it, or when you want
> to know whether an idea has already been tried and rejected — the estimator bake-off (§B0), the
> six rejected address signals (§B6) and the arm-parallel no-go all live here with numbers.

**Purpose.** Two things in the M4-REF work change what our numbers *mean* — the metric-definition
fixes (Phase 0b) and the pose estimator (Phase B0). Neither is allowed to happen without the prior
state recorded here first. If a benchmark band later looks wrong, this file is how you find out
which change moved it.

This is a **findings log**, not a design doc. The decision lives in
[ADR-012](decisions/012-golfdb-reference-data.md); the plan lives in the M4-REF plan.

Machine-readable companion: [`pose_bakeoff_v1.json`](pose_bakeoff_v1.json), written by
`scripts/golfdb/bakeoff.py` (which hard-codes that path — do not move it).

---

## Baseline — before any M4-REF change

- **Date**: 2026-08-01
- **Commit**: `d426a5f`
- **Pose estimator**: MediaPipe Pose Landmarker **lite** (`pose_landmarker_lite.task`), Tasks API,
  `RunningMode.VIDEO`
- **Metric definitions version**: **1**
- **Bands**: `tempo_ratio` 2.7–3.3 (Tour Tempo / Novosel), `head_sway_norm` 0.0–0.5 and
  `finish_balance_norm` 0.0–0.6 (both `PROVISIONAL / UNCALIBRATED`)

Metric definitions as of v1:

| metric | definition |
|---|---|
| `tempo_ratio` | `(top_ms − backswing.start_ms) / (impact.start_ms − top_ms)`, `top_ms` = midpoint of TRANSITION |
| `head_sway_norm` | lateral (`x`) travel of **`NOSE`** from mean-over-ADDRESS to mean-over-IMPACT, ÷ mean shoulder width |
| `finish_balance_norm` | **`max`** hip-center displacement from its own mean over FOLLOW_THROUGH, ÷ mean shoulder width |
| top-of-backswing instant | integer `argmin` of lead-wrist `y` |
| landmark visibility gate | `>= 0.5` everywhere |

### Results (v1 / lite)

| clip | frames | overall | tempo | head_sway | finish_balance |
|---|---|---|---|---|---|
| `aaron-swing-2` | 656 | **67**/100 | 1.53 (MISS) | 0.04 (PASS) | 0.53 (PASS) |
| `golf_swing-aaron-1` | 674 | **33**/100 | 1.69 (MISS) | 1.18 (MISS) | 0.38 (PASS) |

Detected instants:

| clip | address | top | impact |
|---|---|---|---|
| `aaron-swing-2` | 322 | 383 | 423 |
| `golf_swing-aaron-1` | 458 | 517 | 552 |

**Notes on the baseline, for later comparison.**
- Both clips miss tempo badly against the Novosel band. Whether that is a real fault in the swings
  or a bad band is precisely what Phase A is meant to answer — do not "fix" it before then.
- `golf_swing-aaron-1` head_sway of **1.18 shoulder-widths** is implausibly large for a real swing.
  This is a prime candidate for the `NOSE` → ear-midpoint definitional bias (Phase 0b fix 2), and
  is the most useful single number in this table for judging whether that fix works.
- `finish_balance` passes on both clips, but under a `max()` definition that a single bad frame can
  set. Expect `aaron-swing-2`'s 0.53 to **drop** under p90 — if it doesn't move at all, the fix
  isn't wired in.

---

## Phase 0b — metric definitions v1 → v2

- **Date**: 2026-08-01
- **Pose estimator**: unchanged (MediaPipe **lite**) — this section isolates the *definition*
  changes, so nothing else moved.

### What changed

| # | metric | v1 | v2 | why |
|---|---|---|---|---|
| 1 | `finish_balance_norm` | `max` of hip-center drift | **p90** of hip-center drift | `max` is an extreme-value statistic: one mis-detected frame *was* the metric |
| 1b | hip visibility gate | `>= 0.5` (global) | **`>= 0.7`** (hips only) | `visibility` is a learned logit, not a calibrated probability; 0.5 passes confidently-wrong occluded hips |
| 2 | `head_sway_norm` | `NOSE` | **ear midpoint** | the nose rides on a rotating head; the ears straddle its rotation axis |
| 3 | top-of-backswing | integer `argmin` | integer `argmin` — **unchanged, see below** | proposed parabola refinement was measured and rejected |

### Results (v2 / lite)

| clip | overall | tempo | head_sway | finish_balance |
|---|---|---|---|---|
| `aaron-swing-2` | 67/100 *(=)* | 1.53 *(=)* | **0.02** *(was 0.04)* | **0.47** *(was 0.53)* |
| `golf_swing-aaron-1` | 33/100 *(=)* | 1.69 *(=)* | **1.21** *(was 1.18)* | **0.20** *(was 0.38)* |

Detected instants unchanged on both clips (322/383/423 and 458/517/552), as expected once fix 3
was dropped.

### Predictions vs. outcomes

**Confirmed — `max` → p90 is wired in and doing what it was meant to.** Finish drift fell on both
clips (0.53 → 0.47, 0.38 → **0.20**). The much larger drop on `golf_swing-aaron-1` says its `max`
was being set by a small number of outlier frames, which is exactly the failure mode the change
targets. Neither clip changed verdict — both still pass — so the checkpoint didn't go slack.

**Wrong — head sway on `golf_swing-aaron-1` was not a nose-rotation artifact.** The baseline noted
1.18 shoulder-widths as "implausibly large" and predicted the ear-midpoint fix would explain it.
It moved to **1.21** — i.e. essentially unchanged. So that clip's head really is travelling
laterally by more than a shoulder width relative to the ear midpoint, and the fault is either a
genuine lunge or a pose failure on the M1 clip (ADR-002's addendum already records that clip's
lighting/contrast/clutter problems). **Open question, deliberately not chased here** — it does not
affect the GolfDB corpus, and B0/B5 will provide far better tools for diagnosing it than guessing
would. The fix is still correct on its merits (`aaron-swing-2` halved, 0.04 → 0.02, and the
rotation-invariance property is now pinned by a test); it simply was not the explanation for this
clip.

### Rejected: sub-frame parabola refinement of the top

The plan called for locating the top by fitting a parabola near the `argmin` and taking its vertex
— the standard remedy for a flat, noise-dominated extremum. **Implemented, measured, reverted.**

The premise doesn't hold for golf. The top is not a *symmetric* flat extremum but an asymmetric
reversal, and a symmetric-window fit is pulled toward whichever side has the shallower slope:

| clip | approach slope | departure slope | raw argmin | fitted | shift |
|---|---|---|---|---|---|
| `aaron-swing-2` | −0.00596 /frame | +0.00189 /frame | 383 | 384 | **+1** |
| `golf_swing-aaron-1` | −0.00452 /frame | +0.00022 /frame | 517 | 518 | **+1** |

On a synthetic swing whose asymmetry runs the other way, the same code moved the top *earlier*. A
correction whose **sign depends on the local shape of the trajectory** is a data-dependent bias,
not noise reduction — and it would land differently on GolfDB's broadcast footage than on phone
clips, breaking the common-mode cancellation that justifies comparing the two at all. One frame of
movement is not worth that.

The underlying concern (how accurately does `argmin` find the top?) is real and stays open. It gets
**measured** in B5 against GolfDB's ground-truth event labels instead of guessed at. Rationale is
recorded in `phases.py`'s module docstring so this isn't re-attempted.

### Gates

`54 passed` (up from 41 — 9 new, incl. rotation-invariance and single-bad-frame properties),
`ruff` clean, `mypy` clean on `analysis` + `feedback`, all on the base install.

## Phase A — `tempo_ratio` re-sourced from GolfDB event labels

- **Date**: 2026-08-01
- **Input**: 1,399 of 1,400 GolfDB clips (one rejected for out-of-order event labels), 246
  distinct golfers, 580 source videos. Event labels only — **no video, no pose estimator.**

| | low | high | source |
|---|---|---|---|
| was | 2.7 | 3.3 | Tour Tempo (Novosel, 2004) — a book |
| now | **2.72** | **4.71** | p10–p90 of the GolfDB tour population |

**Both plan sanity checks passed**, which is what licenses trusting the rest:

- **p50 = 3.39.** Near the classic 3:1, so `events` is being indexed correctly. The trap here is
  that index 0 is `start`, not `address` (upstream's dataloader rebases with `events -= events[0]`);
  getting it wrong would have produced ~1 or ~9, not 3.4.
- **Slow-motion invariance holds.** p50 is 3.43 for real-time clips and 3.38 for slow-motion, while
  the raw downswing frame counts differ four-fold (median 8 vs 30 frames). The ratio is genuinely
  fps-invariant, so GolfDB's ~46% slow-motion half is usable rather than discardable.

**The finding: Novosel's floor was right, his ceiling was not.** p10 = 2.72 lands almost exactly on
the book's 2.7. But p90 = 4.71 is well above 3.3 — the 2.7–3.3 band captured only about the lower
third of real tour swings, and the population is right-skewed (mean 3.61 > median 3.39, max 10.0).

Four more label-derived metrics were extracted at the same time and are published in
`golfdb_v1.json` but **not** wired into `ranges.json`, since no checkpoint reads them yet:
`toe_up_frac` (p50 0.50), `mid_backswing_frac` (0.69), `mid_downswing_frac` (0.57),
`follow_through_ratio` (0.73).

### Score impact, and an emergent effect worth knowing

| clip | overall before → after | tempo score |
|---|---|---|
| `aaron-swing-2` | 67 → **80** | 0% → **40%** |
| `golf_swing-aaron-1` | 33 → **49** | 0% → **48%** |

Neither clip's *observed* tempo moved (1.53, 1.69) and neither now passes — but both scored
markedly better, because `_score_within_range` decays **in band-widths** and the band is now three
times wider. This was not designed; it falls out of sourcing the band from a real distribution, and
it is defensible: sitting 1.19 outside a population whose own spread is ~2.0 is genuinely less
extreme than sitting 1.19 outside an arbitrary 0.6-wide band. Worth knowing because it means
**re-sourcing a band silently re-scales every partial score computed against it**, not just the
pass/fail boundary.

Follow-on: `evaluate_tempo`'s coaching text quoted a hardcoded "~3:1", which contradicted the new
band the moment it moved. It now quotes the resolved band, like the sway message already did.

## Phase B0 — estimator bake-off

**Blocked on a defect the bake-off itself uncovered.** The harness runs (100+ fps on the 160×160
clips), but it cannot compare estimators yet, because the stage it scores them through —
`segment_phases` — is mislocating the top of the backswing on tour swings by so much that the
estimator differences are lost in it.

### The defect: `argmin` of lead-wrist `y` finds the finish, not the top

`phases.py` anchors on "highest hands = global minimum `y`". On a full tour swing the hands finish
**higher than they were at the top of the backswing**, so the global minimum lands in the
follow-through. Measured on the first face-on clips, against GolfDB's hand-annotated `top`:

| clip | gt top | `argmin` | error |
|---|---|---|---|
| 8 | 59 | 102 | +43 |
| 9 | 148 | 220 | +72 |
| 10 | 104 | 122 | +18 |
| 11 | 155 | 215 | +60 |
| 14 | 66 | 83 | +17 |

Median absolute error over 20 clips: **53.5 frames.** The error is one-signed — always late — which
is the signature of a systematic geometry error, not noise.

Reading clip 8's trajectory frame by frame confirms it: address `y≈0.489`, top `y≈0.279` at frame
56 (gt 59 ✓), impact `y≈0.438` at 67 (gt 67 ✓) — the swing itself is tracked well — then the
follow-through carries the hands to `y=0.255` at frame 74, *below* the top, and `argmin` takes it.

**Why our own clips never showed this.** `aaron-swing-2`'s follow-through stays lower than his top,
so the global minimum happened to be correct. The instants were visually verified on the overlay
and were genuinely right — for that swing. One clip could not have revealed a failure that needs a
tour-length finish to appear. This is the single strongest argument for having ground truth at all.

### Two secondary findings from the same trace

- **Held values can win `argmin`.** `_lead_wrist_xy` holds the last confident position through dim
  frames. On clip 8 the wrist drops below `_MIN_VISIBILITY` after frame ~76 and the series
  flatlines at the held value for 38 frames. A flatline is a legitimate `argmin` candidate, so lost
  tracking can *become* the detected top. Frames without a confident wrist should be excluded from
  the search rather than filled in.
- **Impact does not return to address height.** On clip 8 impact reaches `y=0.438` against an
  address of `0.489` — only ~76% of the way back. `_impact_frame`'s primary rule (`y >= address_y`)
  therefore never fires and it silently falls through to its `max`-of-descent fallback. A
  recovery-fraction threshold would fire correctly; the current absolute one does not.

### Fix directions tested so far

| rule | median err | clips > 10 frames off |
|---|---|---|
| `argmin` over all frames (current) | 53.5 | 16/20 |
| first excursion below address, at 50/70/90% margins | 19.0–40.5 | 16–18/20 |
| **`argmin` restricted to before peak wrist speed** | **3.5** | 8/20 |

Restricting the search to before the fastest wrist motion — the downswing is unambiguously the
quickest part of a swing, and the finish comes after it — recovers most of the error at a stroke.
Its residual failures are clips where peak speed itself lands in the follow-through.

Some of the remaining error is not the algorithm's: clip 20's pose track is visibly poor
throughout (jitter of the same magnitude as the signal, `y` jumping to 0.596 near the end), which
is exactly the estimator-quality difference the bake-off exists to measure. Those two effects can't
be separated until top detection is sound.

### The rule adopted: earliest major rising run

`phases.py` now finds the top and impact together, as the two ends of the downswing: take the
maximal near-monotone **rising runs** of lead-wrist `y` (`y` grows downward, so a rising run is the
hands coming *down*), keep those within `_MAJOR_RISE_FRACTION` of the largest, and return the
**earliest** one.

"Earliest" rather than "largest" is the whole fix. A full finish puts the hands higher than the top,
and post-finish tracking often degrades into large spurious excursions — either can produce a rise
that rivals the true downswing. What neither can do is happen *before* it. Ordering is structure
every golf swing has, and unlike a threshold it needs no calibration per camera, player, or fps.
Held (untracked) frames are excluded from run detection, so a stretch of lost tracking can no longer
bound a descent — worth ~1.5 frames of mean top error on its own.

### Locking `_MAJOR_RISE_FRACTION` — full 461-clip sweep

The constant was first set to **0.50** on a 97-clip sample. At 250 clips the optimum had already
moved to 0.80, which is the signature of fitting to sample size. Re-run over the **complete 461-clip
face-on corpus** (`scripts/golfdb/tune_phases.py`, MediaPipe lite, ground truth = GolfDB event
labels):

| rule | top med | top mean | top >10fr | imp med | imp mean | imp >10fr |
|---|---|---|---|---|---|---|
| `argmin_all` (the original) | 21.0 | 39.4 | 84% | 35.0 | 42.6 | 80% |
| `before_peak_speed` | 4.0 | 28.8 | 37% | 1.0 | 26.3 | 30% |
| `max_rise` | 2.0 | 14.3 | 17% | 1.0 | 12.0 | 12% |
| `max_rise_confident` | 2.0 | 11.5 | 15% | 1.0 | 9.9 | 10% |
| `best_rising_run` (largest, not earliest) | 2.0 | 11.8 | 13% | 1.0 | 10.7 | 10% |
| `fastest_rising_run` | 2.0 | 19.0 | 29% | 1.0 | 17.0 | 27% |
| `earliest_major_rise` @ 0.40 | 2.0 | 25.0 | 17% | 1.0 | 22.6 | 17% |
| @ 0.50 *(was in force)* | 2.0 | 18.8 | 13% | 1.0 | 16.6 | 12% |
| @ 0.60 | 2.0 | 15.5 | 11% | 1.0 | 13.0 | 10% |
| @ 0.70 | 2.0 | 12.6 | 9% | 1.0 | 10.6 | 8% |
| @ 0.75 | 2.0 | **10.5** | 9% | 1.0 | **8.9** | 7% |
| **@ 0.80 — adopted** | 2.0 | **10.6** | **9%** | 1.0 | **8.9** | **7%** |
| @ 0.85 | 2.0 | 10.6 | 10% | 1.0 | 8.9 | 7% |
| @ 0.90 | 2.0 | 11.1 | 11% | 1.0 | 9.7 | 8% |
| @ 0.95 | 2.0 | 11.6 | 12% | 1.0 | 10.4 | 9% |

**0.80 is the centre of the 0.75–0.85 plateau, deliberately not the argmin** (0.75, by 0.1 frames).
The three are indistinguishable and the curve rises on both sides, so the choice is insensitive to
which corpus it was cut on — the mistake that produced 0.50 in the first place.

Median top error is **2 frames** and median impact error **1 frame**, against 21 and 35 for the rule
this replaces.

**Effect on the user's own clips: none.** Both have a single dominant rising run (`aaron-swing-2`
0.340 vs 0.022 next; `golf_swing-aaron-1` 0.283 vs 0.006), so every fraction in 0.40–0.95 returns
the same instants. Scores are byte-identical at 0.50 and 0.80. The constant only matters on clips
with a rival late excursion — which is exactly why one clip could never have tuned it.

### Residual failures are structural, not pose quality

46 of 461 clips (10%) are still more than 10 frames off at the top, and the mean error (10.6) sits
far above the median (2.0) because those failures are enormous — 578, 522, 405 frames. Those are
clips containing a **practice swing before the real one**, where "earliest major rise" correctly
finds the earliest major descent and it is simply not the swing GolfDB annotated.

The decisive evidence that this is not an estimator problem: the failing clips have **higher** mean
wrist-tracking confidence than the successes (0.85 vs 0.82). A better pose model will not fix them;
only swing *selection* would. Recorded as an open M4-REF item, not a blocker — the corpus is a
distribution and 10% of clips landing on a practice swing widens it slightly rather than biasing it.

---

## Phase B0 — estimator comparison (partial: lite vs full)

- **Date**: 2026-08-01
- **Sample**: 120 face-on clips, 120 source videos, 87 golfers, scored from the Tier 1 cache
- **Held fixed**: `smooth_keypoints` → `segment_phases` (with `_MAJOR_RISE_FRACTION = 0.80`) →
  boundary extraction. Only the estimator varies.

| estimator | address PCE | top PCE | impact PCE | **mean PCE** | mean med_norm | extraction speed |
|---|---|---|---|---|---|---|
| MediaPipe **lite** (baseline) | **10.8%** | 61.7% | **88.3%** | 53.6% | 0.060 | **~106 fps** |
| MediaPipe **full** | 6.7% | **70.8%** | 86.7% | **54.7%** | **0.058** | — |
| MediaPipe **heavy** | 8.3% | 66.7% | 86.7% | 53.9% | 0.065 | **24 fps** (1400 s / 120 clips) |
| **RTMPose-m** | 2.5% | 30.8% | 56.7% | 30.0% | 0.233 | 118 fps (275 s / 120 clips) |

Median frame errors are identical across all three MediaPipe variants (address 6, top 1, impact 0),
so PCE is where any difference has to show up.

**The three MediaPipe variants span 1.1pp of mean PCE and no variant wins any event significantly.**
Exact McNemar over the 120 paired clips, all nine variant x event comparisons:

| pair | address | top | impact |
|---|---|---|---|
| lite vs full | 0.267 | **0.099** | 0.774 |
| lite vs heavy | 0.581 | 0.362 | 0.791 |
| full vs heavy | 0.791 | 0.487 | 1.000 |

Not one reaches p < 0.05. The single suggestive result is full's +9.1pp at the top — the instant
`tempo_ratio` is computed from — where full wins 24 of the 37 clips the two disagree on. Address is
near-meaningless for every variant (6.7–10.8% correct within tolerance), which is consistent with
GolfDB's own SwingNet managing only 31.7% there; it is intrinsically the hard instant, not a model
failure.

**Heavy is the clearest result in the table: it costs 4.4x lite and buys nothing** (53.9 vs 53.6
mean PCE, and the *worst* address `med_norm` of the three). ADR-002 rejected heavy in M1 on the
basis of one clip judged by eye; that judgement was right, and this is the number that was missing
from it.

### RTMPose was 35x too slow for the wrong reason

RTMPose-m initially measured **3.1 fps** against MediaPipe lite's ~106 — which would have made a
120-clip run ~3 hours and turned the fourth bake-off variant into an overnight-or-drop decision.

The published figure for RTMPose-m is ~11 ms/frame, so 3.1 fps was not the pose model. rtmlib's
`Body` wrapper pairs it with a **YOLOX-m person detector re-run on every frame** at 640x640 — an
input 16x the area of the 160x160 clip it is searching. Calling `RTMPose` directly with no `bboxes`
makes it treat the whole frame as the person box:

| path | speed | agreement with the detector path |
|---|---|---|
| `Body` (YOLOX-m + RTMPose-m) | 3.07 fps | — |
| **RTMPose-m alone, full-frame box** | **66.3 fps** *(21.6x)* | median 0.84 px, max 8.7 px (of 160) |

Measured on 40 frames of clip 8, under CPU contention from the heavy extraction; a subsequent
3-clip extraction ran at **117 fps**, i.e. faster than MediaPipe lite. The detector was ~97% of the
cost and moved the keypoints by under a pixel.

**Why skipping it is correct here and not a shortcut.** `videos_160` clips *are* GolfDB's own
bounding-box crops around the golfer, resized to square — the full frame already is the person box,
so YOLOX was re-deriving a constant 120 times a clip. It also removes a confound: detector jitter
would have been scored as pose jitter.

**The condition this attaches to the result.** It does not generalize to full-frame video. If
RTMPose ever cleared the adoption bar, the user's own phone clips are not pre-cropped and would
need a detector, or a bbox tracked once and reused — a cost this bake-off therefore does *not*
measure. That belongs in the adoption decision alongside the 33 → 17 landmark contract change.

### RTMPose-m: rejected, by 24.7pp

Once it was cheap to run, it lost decisively. 120 clips extracted in 275 s (**118 fps**, faster than
MediaPipe lite) and scored on the identical sample:

| estimator | address PCE | top PCE | impact PCE | **mean PCE** | mean med_norm |
|---|---|---|---|---|---|
| MediaPipe lite | 10.8% | 61.7% | 88.3% | 53.6% | 0.060 |
| MediaPipe full | 6.7% | 70.8% | 86.7% | **54.7%** | 0.058 |
| **RTMPose-m** | 2.5% | 30.8% | 56.7% | **30.0%** | 0.233 |

The adoption bar was "beats the best MediaPipe variant by >= 3pp PCE". It comes in **24.7pp below**
it, with median address error 20.5 frames against 6 and median top error 4 against 1. The 33 → 17
landmark contract change is not on the table.

**This is a real result, not a broken adapter.** Two checks were run before accepting it, because a
75.8-COCO-AP model scoring this badly is suspicious:

- **The COCO-17 mapping is correct.** Lead-wrist `y` from RTMPose correlates with MediaPipe lite's
  at **median 0.948** across 60 clips, with 1 clip below 0.5. It is tracking the same motion.
- **It is tracking it far more noisily.** Mean absolute second difference of `y` (frame-to-frame
  jitter, x1000), same clips, after `smooth_keypoints`:

  | landmark | lite | RTMPose-m | ratio |
  |---|---|---|---|
  | LEFT_WRIST | 2.85 | 4.19 | 1.5x |
  | LEFT_SHOULDER | 0.54 | 1.30 | 2.4x |
  | LEFT_HIP | 0.45 | 1.94 | **4.3x** |

  Its lead-wrist confidence is also lower — p50 0.60 vs 0.90 — so only **72.9%** of frames clear
  `phases.py`'s `_MIN_VISIBILITY = 0.5` gate, against 85.5% for lite.

Locating an *instant* means reading the shape of a trajectory, so jitter hurts far more than a
static accuracy score suggests. The hip and shoulder figures matter independently of segmentation:
both pose checkpoints normalize by shoulder width, and `finish_balance_norm` reads hip drift
directly, so RTMPose would have degraded those bands too.

**The most likely cause, and the caveat it carries.** MediaPipe runs through
`RunningMode.VIDEO`, which tracks landmarks across frames; RTMPose here is independent per frame,
with no temporal filter. That is not a like-for-like architecture comparison — rtmlib ships a
`PoseTracker` that would narrow the gap. It *is* the right comparison for **this** decision, because
production would use MediaPipe's video mode too, and the question asked is "is switching worth a
contract change", not "which backbone is stronger". A rematch would need the tracker plus a
detector, and would have to find 25pp.

### lite vs full at n=461: the one suggestive result was sample noise

Full's +9.1pp at the top was the only signal in the 120-clip table worth chasing, and at p = 0.099
it was suggestive without being established. Rather than leave the estimator choice resting on it,
full was extracted across the **whole 461-clip face-on corpus** (341 new clips, 1220 s, 83 fps) and
the comparison re-run at 3.8x the sample:

| | address PCE | top PCE | impact PCE | **mean PCE** | mean med_norm |
|---|---|---|---|---|---|
| **MediaPipe lite** | **8.7%** | 61.4% | **87.0%** | **52.4%** | 0.073 |
| MediaPipe full | 6.5% | **63.3%** | 84.6% | 51.5% | **0.069** |

**Full's top advantage collapsed from +9.1pp to +1.9pp, and lite now leads on the mean.** Exact
McNemar over all 461 paired clips:

| event | lite correct only | full correct only | exact p |
|---|---|---|---|
| address | 32 | 22 | 0.220 |
| **top** | **64** | **73** | **0.494** |
| impact | 36 | 25 | 0.200 |

At n=120 full won 24 of 37 discordant clips at the top; at n=461 it wins 73 of 137 — a coin flip.
**This is the same failure mode that put `_MAJOR_RISE_FRACTION` at 0.50**: an effect estimated on a
sample too small to support it, which regresses once the corpus is large enough. It cost 20 minutes
of extraction to find out, instead of being silently baked into every band.

### Decision: keep MediaPipe lite

| criterion | verdict |
|---|---|
| accuracy | no variant differs significantly on any event (9 tests at n=120, 3 at n=461, none p < 0.05) |
| mean PCE at n=461 | lite **52.4%** vs full 51.5% |
| speed | lite **~106 fps** vs full 83, heavy 24 |
| corpus already cached | lite **461/461** |
| change risk | none — lite is the incumbent |

Nothing in the evidence justifies a switch, and lite wins the tie-breaks outright. `estimator.py`
is unchanged; `golfdb_v1.json.dataset.pose_estimator` records `mediapipe:lite` as what produced the
bands.

**What would reopen this.** The bake-off scores *event recovery*, which is what our metrics consume,
but it is insensitive to steady-state landmark accuracy — a variant could track hips better while
recovering the same instants. If a future checkpoint depends on absolute landmark positions rather
than trajectory shape (spine angle, hip rotation), this comparison does not settle the choice for it
and should be re-run against that metric.

---

## Phase B3 — the 160x160 aspect-ratio correction, measured

`videos_160` clips are GolfDB bounding-box crops resized to a **square**, so `x` and `y` are
squashed by different factors. `ReferenceSwing.pixel_aspect` carries the correction, derived from
the `bbox` column assuming a 16:9 source (the one unavoidable assumption).

**The corpus-wide aspect distribution is not the one that applies to us.** Over all 1,399 clips it
is p10 0.683 / p50 1.031 / p90 1.366 — near-symmetric about 1.0. But we extract **face-on only**,
and that stratum is different in kind:

| view | n | p10 | p50 | p90 |
|---|---|---|---|---|
| all | 1399 | 0.683 | 1.031 | 1.366 |
| **face-on (the corpus we use)** | **461** | **0.647** | **0.726** | **1.070** |
| down-the-line | 584 | 0.936 | 1.123 | 1.449 |
| other | 354 | 0.839 | 1.131 | 1.442 |

A golfer viewed face-on is a tall, narrow subject, so those crops are systematically **portrait**:
`y` is compressed by ~27% at the median, in one direction, rather than spread symmetrically about
1.0. Quoting the corpus-wide figure would understate a systematic bias as a wash.

Effect on the two metrics (MediaPipe lite, 458 clips measured at ground-truth instants):

| metric | corrected p50 | p90 | p95 | uncorrected p50 | p90 | p95 |
|---|---|---|---|---|---|---|
| `head_sway_norm` | 0.170 | 0.420 | 0.530 | 0.170 | 0.420 | 0.530 |
| `finish_balance_norm` | 0.150 | **0.280** | 0.370 | 0.170 | **0.300** | 0.370 |

**`head_sway_norm` is identical to three decimals, corrected or not** — exactly as predicted, since
it is an x-distance over an x-scale and the factor cancels. That is the useful check here: it
confirms the correction is wired to affect only what it should, rather than quietly rescaling
everything.

`finish_balance_norm` does move, but by ~7% at p90, far less than the 27% median compression
implies — hip drift through the follow-through is mostly lateral, so the `y` rescale is diluted.
Real, worth applying, not the dominant term.

---

## Phase B4 — the two PROVISIONAL bands, recalibrated

- **Date**: 2026-08-02
- **Estimator**: MediaPipe lite · **metric definitions v2** · aspect-corrected
- **Population**: 458 face-on GolfDB clips from **122 tour golfers**, measured at GolfDB's
  hand-annotated instants (never at `segment_phases` output)

| checkpoint | was | now | basis |
|---|---|---|---|
| `head_sway_norm` | 0.0 – **0.5** *(uncalibrated)* | 0.0 – **0.42** | p90, n=458 |
| `finish_balance_norm` | 0.0 – **0.6** *(uncalibrated)* | 0.0 – **0.28** | p90, n=458 |

Both are one-sided ("lower is better"), so they are cut at `low: 0.0, high: p90` rather than
p10–p90. **Both eyeballed bands were loose, and `finish_balance` was loose by more than 2x.** The
full percentile curves are in `benchmarks/golfdb_v1.json`; `PROVISIONAL / UNCALIBRATED` is gone from
both `source` strings.

### Effect on the user's clips

| clip | overall | tempo | head_sway | finish_balance |
|---|---|---|---|---|
| `aaron-swing-2` | 100 → **78** | 3.39 PASS | 0.02 PASS | **0.47 MISS** *(band 0.6 → 0.28)* |
| `golf_swing-aaron-1` | 67 → **67** | 2.92 PASS | 1.21 MISS | 0.20 PASS |

`aaron-swing-2` loses its perfect score to a genuinely tightened band, not to a changed measurement
— its finish drift of 0.47 was always there, and 0.47 is above the 90th percentile of tour finishes.
That is the recalibration doing its job: the old 0.6 passed a swing the reference population says is
unusual.

### A defect the tightened band exposed: the p90 robustness test was vacuous

`test_finish_balance_survives_a_single_bad_frame` failed on the new band, and the reason was not the
band. The test asserted `passed is True` after blowing out one follow-through hip frame — but the
metric read **0.48**, so it had only ever been passing because 0.48 < 0.6. **It never demonstrated
the property it was named for.** Tightening to 0.28 is what revealed that.

Investigating it produced the real finding:

- `smooth_keypoints` is a 5-frame moving average, so **one blown-out frame becomes five**
  contaminated frames before the statistic ever sees it.
- p90 can therefore only discard the outlier once `5/n < 0.10` — **beyond roughly 50 follow-through
  frames**. Measured: the contamination survives at 8, 12, 20, 30 and 40 frames and disappears at 60.
- The fixture's follow-through was a hardcoded **8 frames**. The corpus the band is cut from is
  **p10 50 / p50 89** frames. The test was running the metric in a regime real footage never reaches.

**The metric is fine; the fixture was not.** Only 8% of corpus clips fall below 50 follow-through
frames, so the v2 p90 change works as claimed on real data — confirmed independently by the original
v2 measurement (`aaron-swing-2` 0.53 → 0.47, `golf_swing-aaron-1` 0.38 → 0.20).

A median center was tried as an alternative and is **worse** (0.56 vs 0.48 on the short window): with
a robust center the outlier sits at its full distance and a near-max quantile then picks it up. The
problem was never the center.

Fixed by giving `make_swing` a `followthrough_frames` parameter (default 8, so no other test moves),
testing the property at a realistic 90, and asserting on **`observed` rather than `passed`** so the
claim is about the statistic instead of about whatever `ranges.json` currently holds. A companion
test now pins the short-window limit so it is recorded rather than rediscovered. `67 passed`.

### Visual spot-check — the one non-numerical verification

Every other check in this milestone is a statistic. All of them can look healthy while the pose
underneath is wrong in a way that happens not to move an event index, and `videos_160` is the
lowest-quality input anywhere in this project. `scripts/golfdb/spot_check.py` renders a contact
sheet — 5 clips x 4 ground-truth instants, skeletons drawn by the production `draw_skeleton` — to
`data/reference/golfdb/spot_check_<estimator>.png` (gitignored).

Reviewed for MediaPipe lite over clips 8, 268, 656, 114, 925 (3 drivers, 2 irons, 114–322 frames):

- **Torso, hips, shoulders and legs are structurally correct in all 20 tiles.** That is precisely
  what the two recalibrated bands rest on — `finish_balance_norm` reads hip drift through the
  follow-through, and both metrics divide by shoulder width. Hip and leg structure is intact in
  every finish tile, which is the most occluded moment and the one the metric cares about.
- **The annotated instants match the imagery**: address over the ball, hands high at the top, club
  at the ball at impact, body wrapped at the finish. An independent confirmation that `events` is
  being indexed and rebased correctly, arrived at by eye rather than by the p50-near-3.0 argument.
- **Arm and hand joints cannot be separated by eye at this resolution.** The dense marker cluster
  around the head at the top and finish is mostly expected — 11 of MediaPipe's 33 landmarks sit on
  the face, and at the top the hands arrive in the same region — but individual wrist/elbow accuracy
  is not visually assessable on a 160x160 crop. This is a limit of the footage. It is also the part
  already covered numerically: lead-wrist tracking is what segmentation uses, and that is validated
  against ground truth at a median of 2 frames.

**Verdict: the corpus is trustworthy for the two bands derived from it.** No clip showed the failure
mode this check exists to catch — a confident, well-formed skeleton attached to the wrong thing.

---

## Phase B5 — the address constants, off one clip no longer

`_MOTION_QUIET_FRAC = 0.08` and `_MOTION_STALL_FRAMES = 3` were read off `aaron-swing-2`'s speed
profile alone. The plan called for putting a real number on them; the top/impact tuning did not
touch them, so they were swept separately against the 461-clip corpus.

Median absolute address error, 2D grid:

| frac \ stall | 2 | 3 | 4 | 5 | 6 | 8 | 10 |
|---|---|---|---|---|---|---|---|
| 0.03 | **8.0** | 10.0 | 13.0 | 14.0 | 17.0 | 19.0 | 23.0 |
| 0.04 | 9.0 | 9.0 | 10.0 | 12.0 | 12.0 | 15.0 | 18.0 |
| **0.05** | 10.0 | 9.0 | **9.0** | 9.0 | 9.0 | 12.0 | 13.0 |
| 0.06 | 10.0 | 10.0 | 9.0 | 9.0 | 9.0 | 10.0 | 11.0 |
| 0.07 | 12.0 | 11.0 | 10.0 | 10.0 | 9.0 | 9.0 | 11.0 |
| 0.08 *(was)* | 13.0 | **13.0** | 11.0 | 11.0 | 10.0 | 10.0 | 10.0 |

**Adopted (0.05, 4).** The grid minimum is 8.0 at (0.03, 2), but it sits **on the grid edge** and
degrades in every direction — 10.0 one step in stall, 9.0 one step in frac. Taking it would repeat
the edge-fit that put `_MAJOR_RISE_FRACTION` at 0.50 and full ahead of lite at n=120. (0.05, 4) sits
at the centre of a flat 9.0 region spanning frac 0.05–0.06 and stall 3–6.

| | median | mean | > 10 frames |
|---|---|---|---|
| (0.08, 3) — from one clip | 13.0 | 35.8 | 52% |
| **(0.05, 4) — from 461 clips** | **9.0** | **27.2** | **46%** |

Effect on the user's clips is small and changes no verdict: `aaron-swing-2` address 322 → **319**
(tempo 3.39 → 3.52, still mid-band), `golf_swing-aaron-1` unchanged at 458. The 3-frame shift is
consistent with the original visual verification, which observed motion only appearing by frame 345
and so cannot distinguish 319 from 322.

**This is a calibration, not a fix.** Address is still the weakest instant by a wide margin — 46% of
clips more than 10 frames out. It is intrinsically hard: the takeaway onset is a gradual departure
from stillness, not a direction change (top) or a contact event (impact), which is why GolfDB's own
SwingNet reaches only 31.7% PCE there. Improving it beyond calibration remains an open ROADMAP item.

### `_TRANSITION_HALF_FRAMES` cannot be tuned this way

The plan named it alongside `_MOTION_QUIET_FRAC`, but **GolfDB labels the top as a single instant,
not a window**, so there is no ground truth for how wide the transition should be. It is also
currently inert for scoring: `top_ms` is the window's *midpoint*, which equals the detected top for
any symmetric half-width, and no checkpoint measures over the TRANSITION segment. Left at 3 and
recorded here so the gap is deliberate rather than overlooked.

---

## Phase B6 — address, with a different rule rather than a better constant

B5 ended by parking this: *"Improving it beyond calibration remains an open ROADMAP item."* This is
that item. The rule changes; the constants B5 swept are, with one exception, exactly where B5 left
them.

### First: the pooled median was hiding the problem

Split the same 461 clips by capture speed and the single number in B5's headline comes apart:

| population | share | median | mean | > 10 frames |
|---|---|---|---|---|
| real-time | 53% | **5.0** | 23.6 | 29% |
| broadcast slow-motion | 47% | **17.0** | 31.3 | 65% |

A frame is worth four times less time in a slow-motion clip, so a pooled frame median over a corpus
that is ~47% slow-motion measures the corpus mix as much as the rule. This is the same argument
`bakeoff.py` already makes for preferring `med_norm`.

**All three instants scale this way, not just address** — measured after this change, top runs 1
frame real-time against 4 slow-motion, and impact 0 against 3. That is expected and is precisely
why a raw frame count is the wrong unit. What singles address out is not that it scales differently
but that it is far larger in *both* units: `med_norm` 0.122 against 0.029 for top and 0.015 for
impact. The split is a lens, not a diagnosis; it mattered here because it pointed at the one
constant in the rule that was denominated in frames.

`bakeoff.py` now emits `med_err_frames_by_speed` so this cannot again be read as one number.

### The diagnosis: one absolute, and a bad failure path

**The stall was a frame count.** `_MOTION_STALL_FRAMES = 4` required four consecutive still frames.
The downswing is 8 frames real-time and 30 in slow motion, so "four frames" meant something four
times different across the corpus. Worse, a gradual takeaway spends long stretches below *any*
fraction of its own peak speed, so the backward walk stopped mid-takeaway. The fingerprint:

```
signed error   median +4.0   mean -0.4   p10 -32  p25 -4  p75 +13  p90 +31
               early 32%     late 66%
```

Two-thirds of clips detected **late**, which is exactly what a threshold-crossing rule does to a
signal that departs from rest gradually.

**The fallback was frame 0.** When the wrist never settled the rule answered 0 — not a neutral
answer, because GolfDB clips carry a median of **59 frames** of pre-roll (p90 254). It fired on 11%
of clips and cost them a median of 31 / mean 58 frames, which is most of the gap between the corpus
median (9) and its mean (27.2).

### Six alternative signals, all worse

Each is a named rule in `scripts/golfdb/tune_address.py` and re-runs in about a second. Median
absolute error over all 461 clips:

| rule | median | mean | > 10 | med_norm | why it fails |
|---|---|---|---|---|---|
| `setup_ball 0.10` | 13.0 | 23.7 | 57% | 0.180 | displacement *is* fps-invariant where speed is not — but a gradual takeaway leaves a fixed-radius ball late, and shrinking the radius hits the pose noise floor |
| `setup_ball 0.20` | 17.0 | 27.8 | 64% | 0.234 | worse the looser the ball, which is the tell |
| `noise_floor k=3` | 41.0 | 66.2 | 73% | 0.748 | correct intuition (setup jitter and swing peak are unrelated quantities) but no persistence test, so any slow mid-takeaway frame trips it |
| `noise_floor k=10` | 47.0 | 70.8 | 86% | 0.774 | |
| `ramp_extrap 0.05-0.30` | 15.0 | 35.4 | 57% | 0.219 | the textbook onset estimator; it does kill the late bias (signed median -5) but the wrist's early takeaway is not linear in speed, so the fit chases the chosen band |
| `ramp_extrap 0.10-0.50` | 14.0 | 38.1 | 57% | 0.246 | |
| `torso_energy 0.10` | 19.0 | 40.1 | 62% | 0.256 | at 160x160 the torso barely moves during a takeaway |
| `upper_energy 0.10` | 8.0 | 23.8 | 43% | 0.132 | **ties** the lead wrist alone, for eight extra landmarks — no information, just cost |
| `shoulder_turn 0.05` | 24.0 | 45.0 | 71% | 0.370 | the most attractive candidate on paper — a waggle moves the hands, not the shoulders — but the shoulder line is too short a baseline for a stable angle here |
| `persistence 0.70` | 32.0 | 58.2 | 74% | 0.628 | amplitude-free, so it *should* be immune to slow motion; defeated because `smooth_keypoints` is a centered moving average and makes setup jitter look directionally persistent |

Averaging many joints should suppress independent landmark noise by the square root of N and let the
threshold drop far enough to catch a gradual onset. It does not pay at this resolution. The lead
wrist is where the takeaway shows first, and it stayed.

### The ceiling, stated plainly

`prior_tempo` — `top - 3.5 x (impact - top)`, using **no pose signal whatsoever** — scores:

| | median | mean | > 10 | med_norm | PCE |
|---|---|---|---|---|---|
| `prior_tempo (no pose)` | 11.0 | 28.0 | 50% | 0.181 | 13.7% |

The shipped rule beats it by 4 frames of median. That is the honest size of what the lead wrist tells
us about takeaway onset, and it is the bar any future candidate must clear — not the `current` row.
It is a permanent row in `tune_address.py` for exactly that reason.

### What shipped

| | median | mean | > 10 | med_norm | PCE | slow-mo | real-time |
|---|---|---|---|---|---|---|---|
| before (fixed stall, frame-0 fallback) | 9.0 | 27.2 | 46% | 0.133 | 14.3% | 17.0 | 5.0 |
| + clip-relative stall | 8.0 | 24.1 | 41% | 0.129 | 15.0% | 19.0 | 4.0 |
| **+ bounded fallback (shipped)** | **7.0** | **22.9** | **40%** | **0.122** | **15.8%** | **17.0** | **4.0** |

`_MOTION_STALL_FRAMES = 4` becomes `_MOTION_STALL_FRACTION = 0.25` of the detected downswing
duration, floored at 2 frames. `_MOTION_QUIET_FRAC` stays at **0.05** — B5's value, still on its
plateau under the new stall. The frame-0 fallback becomes `top - 3.5 x (impact - top)` and marks the
segment `detected=False`; it fires on **14%** of clips, and on the 86% where it does not fire the
rule scores median 6.0 / mean 18.7 / 35% over 10 frames.

Top and impact are unchanged at 2 and 1 frames. See
[ADR-013](decisions/013-clip-relative-detection.md) for why this is stated as three principles
rather than one fix.

### The posture half, which turned out to matter more

`head_sway` and `finish_balance` averaged over the whole ADDRESS phase — `[0, motion_start]` — so
they inherited both the boundary error and all of the pre-roll. Measured against a tight window
ending at the *labelled* address, over 453 clips:

| head-baseline error (shoulder-widths; the `head_sway` band is 0.42 in total) | median | p90 | > 0.10 | > 0.21 |
|---|---|---|---|---|
| old — the whole `[0, motion_start]` window | 0.023 | 0.135 | 16% | 5% |
| **new — a short window ending at the boundary** | **0.014** | **0.090** | **9%** | **2%** |

**One expected win did not materialize.** The shoulder-width ruler — which divides *both*
checkpoints — was off by more than 10% on 10% of clips before and 11% after. That error is pose
noise, not window content. It is unchanged and still there.

**The effect on our own footage is the clearest evidence in this section.** On
`golf_swing-aaron-1`, `head_sway` moves from **1.21 to 0.36** shoulder-widths: a hard fail against
the 0.43 band becomes a comfortable pass, and the clip's overall score goes to 100/100. Tracing the
head across the old 459-frame window shows why —

```
frames   0- 49: +0.14      frames 250-299: +0.16
frames  50- 99: +0.27      frames 300-349: +0.08
frames 100-149: +0.41      frames 350-399: -0.02
frames 150-199: +0.31      frames 400-449: -0.03
frames 200-249: +0.20      frames 450-458: -0.00
```

(shoulder-widths from the frame-458 setup position). The golfer walks in, settles by ~frame 400, and
the old baseline averaged all of it. The metric was reading the approach as swing sway. This is the
same shape of defect ADR-012 found in top detection: a plausible number measuring the wrong thing,
invisible until something independent was pointed at it. `aaron-swing-2` barely moves
(`head_sway` 0.015 → 0.030, `finish_balance` 0.468 → 0.480, both verdicts unchanged) because its
pre-roll happens to be steady.

### Metric definitions v2 → v3

Changing *where* a metric samples changes the metric, so per ADR-012 §4 the bands were re-derived
rather than assumed still valid:

| band | v2 | v3 |
|---|---|---|
| `head_sway_norm` p90 | 0.42 | **0.43** |
| `finish_balance_norm` p90 | 0.28 | **0.29** |

Small — which is the point. §4 exists so that drift this size gets recorded instead of quietly
invalidating every comparison made against the old band.
