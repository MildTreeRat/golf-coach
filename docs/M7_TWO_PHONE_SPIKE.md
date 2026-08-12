# M7 Phase 0: Two-Phone Field Spike — findings

> ⬜ **Method fixed; the spike's own footage not yet recorded.** Every threshold in this document was
> written **before** any footage existed, which is the point: the pass/fail bar for the biggest
> risk in M7 is not allowed to be chosen after seeing the numbers. Sections marked *pending* are
> filled in after the bay session; nothing else in them changes.
>
> **Q2 is answered** on borrowed footage (§Q2 Results, 2026-08-07): HEVC decodes, frame-exact.
> **Q1 is not**, and its preliminary observation was **corrected on 2026-08-11** — the pre-fix
> reading blamed down-the-line occlusion for a face-on bug. **Q3 has no footage at all**: every clip
> on disk is normal-rate 60fps, so the slo-mo question is untouched. It needs a 30-second recording
> of anything at all, not a bay.

**Status**: method locked 2026-08-07 · Q2 answered · Q1 preliminary corrected 2026-08-11 · Q3 unstarted
**Gates**: [M7 Phase 1, 2 and 5](M7_TWO_PHONE_CAPTURE.md#the-ladder)
**Decisions**: [ADR-003](decisions/003-camera-hardware.md) addendum 2026-07-02b (face-on vs
down-the-line stream assignment), [ADR-011](decisions/011-camera-synchronization.md) + its
2026-08-05 addendum (two capture tiers), [ADR-013](decisions/013-clip-relative-detection.md)
(clip-relative windows — this spike both *uses* and *tests* it)
**Probe**: `spikes/2026-08-07-two-phone/` — throwaway, per `spikes/README.md`

## Why this spike exists

M7 wants two hand-held iPhones — one face-on, one down-the-line — plus a photo of the HD Golf
`SHOT DATA` screen to describe one swing. The ladder in
[M7_TWO_PHONE_CAPTURE.md](M7_TWO_PHONE_CAPTURE.md) has **Phase 2's entire alignment design resting
on both clips producing usable top and impact anchors**, and that has never been tested on a single
frame of down-the-line footage.

Three assumptions sit under the ladder, all unmeasured:

1. **`segment_phases()` works from down-the-line.** It tracks `LEFT_WRIST` y
   (`analysis/phases.py:71`) and was tuned against **461 face-on** GolfDB clips — top to a median of
   2 frames, impact to 1 ([M4_POSE_BAKEOFF.md](M4_POSE_BAKEOFF.md)). For a right-handed golfer the
   lead wrist is the **far** arm from behind, occluded by the torso through the top. The 461 clips
   say nothing about this; GolfDB's 584 labelled down-the-line clips have never been pose-extracted
   (`M4_ADDRESS_DETECTION.md`, Out of scope).
2. **OpenCV decodes what an iPhone writes.** iPhones default to HEVC/h.265 and the entire ingest
   path is one `cv2.VideoCapture` (`capture/file.py:44`).
3. **`CAP_PROP_FPS` describes real time.** iPhone slo-mo stores 120/240fps behind a stretched
   playback rate. fps is the only thing converting frame indices into the `timestamp_ms`
   (`capture/file.py:74`) that every duration downstream is computed from.

The cost of finding out later is asymmetric, which is why this is a spike and not a phase: a
down-the-line failure discovered during Phase 2 invalidates the alignment design, and one discovered
during Phase 5 drags a frame scrubber into the upload UI.

**Note on ordering.** The ladder is being worked out of order — Phases 3 and 5 have already landed
in trimmed form (`storage/manifest.py`, `storage/bundle_store.py`, `api/app.py`,
`scripts/run_server.py`). So a Q2 failure lands as an edit to an *existing* upload path rather than
a new build, and the footage for this spike travels through that path deliberately: it is the real
production route, so whatever arrives at the desktop is the truth that matters.

## Goal

Three answers, each with a pre-committed threshold and a stated design consequence. **No production
code** — nothing under `src/` is touched by this spike.

**Exit criteria**: Q1, Q2 and Q3 each carry a verdict backed by a number; each verdict names the
change it forces (or does not force) in Phases 1, 2 and 5; the M7 ladder's Phase 0 row is ticked.

---

## Method

### What gets recorded

Right-handed golfer, both phones hand-held. 1080p60, **High Efficiency (HEVC)**, AE/AF locked with
a long-press before rolling, landscape, no pan or zoom, rolling from ~2 s before address to ~2 s
after the finish. Whole body with a head of headroom, feet in frame through the finish, club in
frame at the top, ~3–4 m back, lens at hand height.

Face-on is 3 o'clock with the ball flying to 12 (ADR-003 addendum 2026-07-02b). Down-the-line is
directly behind the golfer **on the target line extended** — standing off-axis turns it into a
hybrid view and makes the whole result unattributable, and it is the one framing detail worth being
fussy about.

**Clips are never trimmed in the Photos app.** Trimming re-encodes, which would silently answer Q2
wrong.

| Set | What | Clips | Answers |
|---|---|---|---|
| **A** Main | 6 swings × 2 angles, 1080p60 HEVC. 3× 7-iron, 3× driver — different hand heights and tempos. | 12 | Q1 |
| **B** Codec control | 1 swing, face-on phone on *Most Compatible* (H.264). | 1 | Q2 |
| **C** Mixed-rate pair | 1 swing: face-on 60fps, down-the-line 240fps slo-mo (~3 s). The Phase 2 nightmare case. | 2 | Q1, Q3 |
| **D** Tempo-invariance pair | 1 swing, **both phones face-on, side by side**: one 60fps, one 240fps slo-mo. | 2 | Q3 |
| **E** fps calibration | Each phone films a millisecond stopwatch ~5 s, once per mode. No golfer needed. | 4 | Q3 |
| **F** Known-hard cases | 1 swing preceded by a full practice swing (the documented ~7% face-on failure mode), 1 where the golfer walks into the shot. | 2–4 | Q1 |
| **G** Shot screens | One `SHOT DATA` photo per set-A swing. Secondary — free evidence for the ADR-014 path. | 6 | — |

Plus **one set-A clip pulled off the phone by cable** in addition to the upload. Comparing the two
copies is the only thing separating "OpenCV cannot read HEVC" from "Safari transcoded on upload" —
two entirely different remedies. Clip inventory and per-clip notes: `spikes/2026-08-07-two-phone/log.md`.

### Ground truth

There are no labels for this footage, so truth is hand-made from an index-burned frame dump
(`probe.py frames`). Per clip: **top** = the last frame before the hands begin to descend,
**impact** = the first frame in which the ball is no longer at rest.

Two disciplines make this evidence rather than confirmation:

- **The two angles of one swing are labelled independently**, without looking at the detector's
  output for either. The matched pair is the control: both clips see the same swing, so a
  difference in error is attributable to the *angle* and to nothing else.
- **Every clip records a label uncertainty in frames.** ±1 for impact at 60fps is normal — the ball
  travels ~74 cm between frames at 100 mph. No verdict below may rest on a margin smaller than the
  uncertainty of the label it is measured against.

### The probe

`spikes/2026-08-07-two-phone/probe.py`, throwaway. It imports `_top_and_impact` and `_MIN_VISIBILITY`
from `analysis/phases.py` **on purpose**: measuring the shipped rule is the entire point, and a
re-implementation inside the probe could pass while the real detector fails. Reaching through the
seam is acceptable precisely because the code is discarded once these findings land
(`spikes/README.md`: fold in the findings, not the code).

`probe.py pose` runs a **streaming** MediaPipe pass rather than `scripts/run_pose.py`, whose
`list(source.frames())` (`run_pose.py:40`) materializes every decoded BGR frame — ~2.9 GB for a
1080p60 8-second clip, ~15 GB at 4K60. Fixing that is Phase 1's commit, so the spike simply does not
depend on it. `run_pose.py` is still run once, deliberately, on one clip, to record what it does
(§Q2).

### Candidate signals

Q1 does not only ask *whether* the lead wrist survives down-the-line — it asks what to use if it
does not. Five named signals, each fed to the shipped `_top_and_impact` with only the signal
swapped, following the `scripts/golfdb/tune_address.py` pattern of keeping the losers runnable so
the evidence outlives the decision:

| Variant | Signal |
|---|---|
| `lead_wrist` | `LEFT_WRIST` — the shipped rule; the baseline every other must beat |
| `trail_wrist` | `RIGHT_WRIST` — the **near**, unoccluded arm from down-the-line |
| `wrist_mean` | mean y of both wrists |
| `hand_mid` | midpoint of `LEFT_INDEX` / `RIGHT_INDEX` — the closest pose proxy to the grip |
| `best_visible` | per frame, whichever wrist MediaPipe is more confident about |

The sweep runs on the **face-on** clips too. If a variant wins down-the-line but loses face-on, the
remedy is a per-view landmark choice keyed on the `camera_id` Phase 1 is adding anyway — not a
change to a rule validated on 461 clips. That distinction is the difference between a small addition
and a regression.

### Pre-flight (done, 2026-08-07)

Before any footage exists, the probe was checked against the one clip whose behaviour is already
known — `data/processed/aaron-swing-2.keypoints.json`, face-on:

```
lead_wrist   top=400  impact=423     scripts/analyze_swing.py: TOP @ 400, IMPACT @ 423
```

`lead_wrist` reproduces the shipped pipeline **exactly**, and `probe.py score` scores it at 0.000
error against labels set to those frames. This is the only thing standing between a subtle probe bug
and a false RED verdict on the biggest risk in the project, so it is asserted in code
(`cmd_variants` exits non-zero if the baseline and the shipped pipeline disagree by more than the
TRANSITION half-window).

**One result already, from the pre-flight, and it matters.** On this face-on clip the *trail* wrist
is below the visibility gate in **47.1%** of frames between top and impact (mean visibility 0.45,
min 0.17), against the lead wrist's **0.0%** (mean 0.75). The `trail_wrist` variant consequently
lands on top=477, impact=571 — it finds the *finish*, the exact failure mode `_top_and_impact` was
written to prevent, off by 3.3 and 6.4 downswings respectively.

So `trail_wrist` is **not** a drop-in replacement. If down-the-line turns out to need it, it must be
per-view. That is now measured rather than assumed, before the first swing was recorded.

---

## Q1 — Does `segment_phases()` work on down-the-line video?

**Verdict: _pending_**

Error is normalized against each clip's **own** downswing, `e = |predicted − truth| / (truth_impact
− truth_top)`. Raw frames are not comparable between a 60fps clip and a 240fps one — this is
ADR-013's clip-relative principle applied to the validation of the thing ADR-013 governs. At 60fps a
downswing is ~15 frames, so `e = 0.20` ≈ 3 frames ≈ 50 ms, roughly twice the labelling noise and
therefore the tightest bar that can be defended.

| Verdict | Condition, over ≥6 down-the-line swings |
|---|---|
| **GREEN** | median `e_top` ≤ 0.20 **and** median `e_impact` ≤ 0.15 **and** no clip above 0.50 **and** ordering `motion_start < top < impact` holds everywhere **and** the DTL median ≤ 2× the face-on median on the same swings |
| **AMBER** | nothing above `e = 1.0`, but a GREEN threshold is missed |
| **RED** | any clip above `e = 1.0` — the instant landed outside the true downswing entirely, i.e. the "found the finish" failure — or `segment_phases()` returns `[]` or non-monotonic boundaries on any clip |

### Results

**Still pending — the verdict table below is unfilled and stays unfilled.** What follows is a
*preliminary observation* from the two bay pairs recorded for M7 Phase 2 (`data/raw/aaron-1`,
`aaron-2`). It is deliberately not scored against the GREEN/AMBER/RED table, because this footage
does not meet the bar this document set before any of it existed:

- **No hand labels.** Truth was read off frame dumps by eye during the Phase 2 work, with no
  recorded uncertainty. Rule 3 of Verification forbids quoting a margin against a label that has
  none.
- **One usable swing per pair, not six.** n=2, against a threshold written for n≥6.
- **The down-the-line framing is off-axis.** The camera sits behind *and to the trail side*, not on
  the target line extended. §Method calls this out as the one framing detail worth being fussy
  about, precisely because it makes a result unattributable to the angle.

So this is evidence about *these clips*, not a measurement of the shipped rule from down-the-line.

**What was observed** (aaron-1: face-on 926 frames, down-the-line 2472 frames, both 4K60).
⚠️ **Measured 2026-08-07, before `phases._DRAWDOWN_FLOOR` landed** — the face-on downswing in this
table is the noise bug, not the swing. Superseded by the re-measurement below; kept because the
inference drawn from it is the thing that needed correcting.

| | face-on | down-the-line |
|---|---|---|
| body found by MediaPipe | 100.0% / 100.0% | 84.6% / 96.2% |
| candidate descents in the clip | 3 | 7 |
| what `segment_phases()` picked unaided | a setup move at 0.9 s | a setup move at 15.2 s |
| top / impact once windowed to the real swing | 704 / 718 | 1551 / 1574 |
| impact by eye (ball present → gone) | ~718 | 1570 → 1580 |
| downswing | 14 frames (0.23 s) | 23 frames (0.38 s) |

Three things follow, and the third is the one that matters:

1. **Neither view finds the real swing unaided on a full-length bay clip**, and that is not a
   down-the-line problem — the *face-on* clip failed the same way. It is the practice-swing failure
   this repo already documents (`phases.py`, the residual 7% of GolfDB), and on untrimmed phone
   footage it is the common case rather than the tail. M7 Phase 2 handles it by selection
   (`candidate_downswings()` + a frame window), not by changing the rule.
2. **Down-the-line generates far more spurious candidates** — 7 against 3 — and inspection shows
   why: a bystander walks through frame at 36 s and the phone is lowered at 40 s, both of which
   MediaPipe's single-person model tracks happily. Down-the-line is filmed from the busy side of a
   bay; face-on is not.
3. ~~**Given the right window, down-the-line top and impact are usable.** Impact landed within the
   1570→1580 bracket the ball's departure defines. The top is the weaker of the two and reads
   **early** — 23 frames of downswing against face-on's 14 for the same swing at the same frame
   rate. That is the shape of an occlusion effect (the lead wrist is the far arm from behind), and
   it is what a real Q1 measurement should be sized to detect.~~
   **Withdrawn 2026-08-11 — the attribution was wrong.** See the re-measurement below. Impact was
   and remains fine; the "top reads early from down-the-line" reading compared DTL against a
   *face-on* number that was itself a bug.

### Re-measurement, 2026-08-11 — post-`_DRAWDOWN_FLOOR`

Both pairs re-run through the shipped path (`smooth_keypoints` → `select_swing` → `segment_phases`),
unchanged, after the drawdown-floor fix in `phases.py` (commit `57d0d33`). **Still not scored against
the verdict table** — the three disqualifications above are untouched: no hand labels, n=2, off-axis
DTL framing. This corrects an inference, it does not close Q1.

| | aaron-1 face-on | aaron-1 DTL | aaron-2 face-on | aaron-2 DTL |
|---|---|---|---|---|
| top → impact | 694 → 718 | 1550 → 1574 | 880 → 903 | 1772 → 1797 |
| **downswing** | **24 fr (401 ms)** | **24 fr (400 ms)** | **23 fr (384 ms)** | **25 fr (417 ms)** |
| backswing | 55 fr | 33 fr | 51 fr | 26 fr |
| lead wrist ≥0.5, whole clip | 74.4% | 28.2% | 74.4% | 22.0% |
| lead wrist ≥0.5, in window | 78.2% | 51.9% | 89.4% | 49.8% |
| trail wrist ≥0.5, whole clip | 100.0% | 65.3% | 100.0% | 84.8% |

**The two views now agree on the downswing to within 1 ms on aaron-1 and 2 frames on aaron-2.** The
pre-fix table's face-on 14 frames was `_rising_runs` splitting a descent on a 0.002 wobble at the top
(ROADMAP: *"the downswing measured 14 frames where the truth was 24"*). DTL's 23 was approximately
**right all along**. So the spike's own preliminary evidence had blamed down-the-line occlusion for a
face-on bug — the exact error §Verification rule 2 exists to catch, arriving from the direction that
rule does not cover: the face-on control misbehaving *quietly* rather than obviously.

**The residual disagreement is entirely in `motion_start`, not the top.** Backswing runs 22 frames
short on aaron-1 and 25 short on aaron-2 from down-the-line, which is what drags DTL `tempo_ratio` to
~1.2–1.6 against face-on's ~2.6 and holds alignment at the `top_impact` tier. That matches the open
M7 item in ROADMAP verbatim and now has a mechanism under it: **the lead wrist is below the 0.5
visibility gate on roughly half the frames of a DTL swing window (51.9% / 49.8% pass) against ~80–90%
face-on**, so `_motion_start`'s quiet-run search is working from a signal that is mostly gated out
exactly where it looks.

Two consequences for the eventual verdict, both pointing the same way:

- **Q1's GREEN/AMBER/RED table asks only about `e_top` and `e_impact`.** On this evidence those are
  the *strong* instants from down-the-line, and `motion_start` — which the table does not score at
  all — is the weak one. Whoever fills in the verdict should add an `e_motion_start` column or the
  spike will return GREEN on a rule that is measurably wrong about the takeaway.
- **The trail wrist is the better-seen landmark from down-the-line** (65.3% / 84.8% against the
  lead's 28.2% / 22.0%), which is the `trail_wrist` variant §Candidate signals already lists. The
  pre-flight finding stands and is now doubly load-bearing: it finds the **finish** on face-on
  footage, so this can only ever be a per-view choice keyed on `camera_id`, never a global swap.

**This does not close Q1.** It says the question is worth answering properly and that the answer is
unlikely to be RED. The verdict still needs the set-A inventory, on-axis framing, and `truth.json`.

*Verdict table — pending. `probe.py score spikes/2026-08-07-two-phone/truth.json`.*

| angle | variant | n | med `e_top` | med `e_impact` | max `e` |
|---|---|---|---|---|---|
| face_on | lead_wrist |  |  |  |  |
| down_the_line | lead_wrist |  |  |  |  |
| down_the_line | trail_wrist |  |  |  |  |
| down_the_line | wrist_mean |  |  |  |  |
| down_the_line | hand_mid |  |  |  |  |
| down_the_line | best_visible |  |  |  |  |

### Mechanism

*Pending. `probe.py wrist` — lead and trail wrist visibility over `[top, impact]`, % below the 0.5
gate, and snap-back "teleport" count (MediaPipe swapping left for right, a real risk from behind
where the model has far less training data than face-on).*

| clip | landmark | vis mean (top→impact) | vis min | % below gate | teleports |
|---|---|---|---|---|---|
|  |  |  |  |  |  |

> Partial data exists — see the 2026-08-11 re-measurement above. It is **not** transcribed into this
> table, because it was taken over `select_swing`'s whole window rather than `[top, impact]` and
> carries no teleport count, and a table that mixes two windows is worse than an empty one. It does
> already settle the prediction below in the affirmative for the *takeaway*: visibility collapses
> (~50% gated out) exactly where `motion_start` fails, which is the occlusion signature rather than
> the projective-geometry one.

This table is what makes the verdict *attributable*. Large errors **with** collapsed visibility mean
occlusion, and a different landmark can fix it. Large errors **with** healthy visibility mean the
signal itself is wrong from that angle, and no landmark swap will help. It is the same species of
evidence as the ADR-003 addendum, which moved the pose camera on a 0.71 → 0.88 knee-confidence
measurement.

**A prediction, recorded before the footage, so it can be refuted:** vertical is vertical in any
camera with a roughly horizontal optical axis, so the *shape* of the wrist-y trace should survive
the angle change — the hands go up and come down in both views. If down-the-line fails, the cause
should therefore be tracking and occlusion, not projective geometry. If it fails with high
visibility and no teleports, this prediction was wrong and the reasoning above needs revisiting
before any remedy is chosen.

### What each outcome implies

- **GREEN** → Phase 2 proceeds exactly as [M7_TWO_PHONE_CAPTURE.md](M7_TWO_PHONE_CAPTURE.md)
  specifies. Down-the-line anchors are primary; no fallback path needed.
- **AMBER** → Phase 2 anchors the warp on the **face-on** clip's instants and treats down-the-line
  as the follower, keeping only its impact anchor. The side-by-side CLI gains the manual
  impact-frame nudge the M7 doc already contemplates, and `alignment_quality` must be able to say
  *"aligned on impact only"* rather than implying precision that does not exist.
- **RED, but a variant is GREEN** → Phase 1 grows a per-view landmark choice keyed on the `camera_id`
  it is already adding, and `phases.py` gains a signal parameter defaulted to `LEFT_WRIST` so the
  461-clip face-on validation is untouched. Record as an ADR-013 addendum, not a rewrite. **The
  pre-flight already shows this cannot be a global swap** — `trail_wrist` finds the finish on
  face-on footage.
- **RED with no variant GREEN** → the lead-wrist signal does not survive down-the-line. Phase 2's
  DTL alignment becomes manual-anchor-only (the user picks the impact frame), which drags a frame
  scrubber into the Phase 5 upload UI. **That is a real scope increase and is recorded here rather
  than discovered halfway through Phase 5.**

---

## Q2 — Can OpenCV decode iPhone `.mov`?

**Verdict: _pending_**

**Precondition.** At least one clip must report a FOURCC of `hvc1` or `hev1`. If everything reads
`avc1`, HEVC was never tested — something transcoded, and the cable-copied control clip is what says
whether it was the phone's Format setting, the Photos export, or the upload path. `probe.py inspect`
prints a warning when this precondition is unmet, because a clean-looking PASS on H.264 footage
would be the most expensive kind of wrong answer here.

| Verdict | Condition |
|---|---|
| **PASS** | opens; decoded count within 1 frame of `CAP_PROP_FRAME_COUNT`; `decoded / fps` within 2% of the duration Photos reports; zero all-black frames; MediaPipe finds a body in ≥95% of frames between address and finish |
| **PARTIAL** | opens, but decodes short or garbled — the dangerous case, because it fails *silently* |
| **FAIL** | `cv2.VideoCapture` will not open the file |

### Results

**Partial, from M7 Phase 2's footage (2026-08-07).** Not the full set-A/B/C inventory this spike
specifies — these are four bay clips recorded for the alignment work, pulled off the phone by cable
rather than through the upload path, and not hand-labelled. So they answer Q2's *decode* question
and nothing about the upload leg. Recorded here because the precondition is met and the answer is
unambiguous.

| file | backend | fourcc | reported fps | reported / decoded frames | resolution | decode+pose fps |
|---|---|---|---|---|---|---|
| `Aaron-front-1.MOV` | FFMPEG | `hevc` | 59.916 | 926 / 926 | 2160×3840 | 9.8 |
| `Aaron-back-1.MOV` | FFMPEG | `hevc` | 59.959 | 2472 / — | 2160×3840 | 9.5 |
| `Aaron-front-2.MOV` | FFMPEG | `hevc` | 59.917 | 1162 / — | 2160×3840 | — |
| `Aaron-back-2.MOV` | FFMPEG | `hevc` | 59.959 | 3211 / — | 2160×3840 | — |

**The precondition is met**: every clip reports `hevc`, so HEVC is genuinely under test rather than
H.264 in disguise. All four open, and the one run to completion so far decoded **exactly** the frame
count the container claimed, with a body found in **100%** of frames. That is the PASS condition for
decode integrity on this evidence.

Two things worth carrying forward:

- **These are 4K portrait (2160×3840), not the 1080p60 the M7 doc asks for.** Nobody told the phones
  otherwise, which is the honest default behaviour to design against.
- **~9.5 fps end-to-end at 4K** (decode + MediaPipe, single pass, no downscale) — so a 40-second
  down-the-line clip is ~4.5 minutes of pose. That is the first real throughput number this project
  has, and it is the input the deferred pre-inference downscale decision (M7 Phase 1) was waiting
  for. It is *tolerable*, which is why nothing was changed on the strength of it.

For calibration, the same probe on the existing committed sample (`data/raw/aaron-swing-2.mov`,
already transcoded to 480×854 H.264 at some point in its history):

| file | backend | fourcc | reported fps | reported / decoded | black | decode fps |
|---|---|---|---|---|---|---|
| `aaron-swing-2.mov` | FFMPEG | `h264` | 58.913 | 656 / 656 | 0 | 675 |

Two things worth noting from that row before any iPhone footage exists: OpenCV's bundled backend is
FFMPEG, and **the reported fps is 58.913, not 60** — which is already a small piece of the Q3 answer.

### `run_pose.py` under a real clip

*Pending — run once, deliberately, on one 8-second 1080p60 clip and record the outcome.* Not as the
working path: as evidence for Phase 1. `run_pose.py:40` holds every decoded frame, so 1080p60 × 8 s
= 480 × 5.93 MB ≈ **2.9 GB** resident; 4K60 is ~15 GB and a 3-second 240fps slo-mo is ~4.3 GB.
Whether it survives, thrashes, or dies is a number Phase 1's commit should carry.

### What each outcome implies

- **PASS** → nothing changes. Phase 5's existing upload path stands as built.
- **PARTIAL or FAIL** → an ffmpeg transcode step on upload in the already-built `api/app.py`, plus a
  decoded-vs-reported frame-count assertion in Phase 1 so a short decode can never pass unnoticed
  again. PARTIAL is the worse of the two to discover late: a file that opens and returns two-thirds
  of its frames produces a plausible-looking analysis of an incomplete swing.
- Either way, **record decode throughput.** It is the input to the Phase 5 worker latency estimate,
  and nobody has measured it.

---

## Q3 — What does `CAP_PROP_FPS` report, normal vs slo-mo?

**Verdict: _pending_**

True capture rate comes from the set-E stopwatch clips: read the millisecond stopwatch on the first
and last decodable frame, count `N` decoded frames between them,
`true_fps = (N − 1) / (t_last − t_first)`.

- **PASS** (fps describes real time): `|reported − true| / true ≤ 2%`, per phone per mode.
- **Independent cross-check, needing no stopwatch** — the set-D pair. The same swing, same angle,
  recorded at 60fps and 240fps must yield the **same `tempo_ratio`** (within 10%). `evaluate_tempo`
  divides `backswing_ms` by `downswing_ms` (`checkpoints/mechanics.py:329`), so fps cancels
  algebraically. Verifying that rather than asserting it makes this the first test of ADR-013's
  clip-relative claim on real phone footage — GolfDB's slow-motion clips are broadcast, not phone.

### Results

*Pending.*

| phone | mode | `CAP_PROP_FPS` | frames | true fps (stopwatch) | error |
|---|---|---|---|---|---|
|  |  |  |  |  |  |

Tempo invariance (set D, same swing, both face-on):

| clip | reported fps | frames | tempo_ratio |
|---|---|---|---|
|  |  |  |  |

### What each outcome implies

- **PASS** → Phase 1 persists `fps` as planned and nothing else moves.
- **FAIL** → absolute time is wrong everywhere `timestamp_ms` flows (`capture/file.py:74`), so Phase
  1's persisted clip metadata must carry `container_fps` **and** a real-time rate or speed factor,
  not the single `fps` field currently planned.
  - **Tempo survives** — it is a ratio, and fps cancels. The set-D pair is what confirms this
    instead of assuming it.
  - Anything absolute does not: backswing duration in milliseconds, and every future club-speed or
    launch-timing number.
  - **Phase 2 is immune by construction**, because the τ warp is normalized to the swing's own
    events rather than to a clock. A Q3 failure is therefore a *confirmation* of the Phase 2 design,
    not a threat to it — worth saying plainly, because the instinct on reading "the fps is a lie" is
    to go looking for a timestamp fix that Phase 2 does not need.

---

## Confirmed decisions

*Pending — written once the verdicts are in.*

## Out of scope

- **Down-the-line metrics** (spine tilt, swing plane). The GolfDB corpus the benchmark bands derive
  from is face-on, so DTL metrics have no reference population to be judged against. M7 captures and
  aligns the DTL view; scoring it is a separate decision (`M7_TWO_PHONE_CAPTURE.md`, Confirmed
  decisions).
- **3D triangulation.** Permanently unreachable for hand-held phones, which cannot be calibrated —
  see ADR-011's 2026-08-05 addendum. This spike does not revisit it.
- **Fixing `run_pose.py`.** The OOM is real and measured here; the fix is Phase 1's commit.
- **Shot-screen OCR accuracy.** Set G is collected because it is free, not because Phase 0 rules on
  it. ADR-014 governs.
- **Promoting the probe.** `spikes/README.md` is explicit: fold in the findings, not the code. The
  probe reaches into `phases.py` privates and is fine doing so *because* it is discarded.

## Verification

1. **Pre-flight** — `probe.py variants data/processed/aaron-swing-2.keypoints.json` reproduces the
   shipped detector's top and impact exactly (400 / 423). ✅ 2026-08-07
2. **The face-on control behaves.** If face-on errors are also poor on this footage, the fault is the
   footage or the phone settings, not the angle — re-shoot rather than conclude anything about
   down-the-line. This is the guard against condemning the detector for a dim bay or an unlocked
   exposure.
3. Every clip in `truth.json` carries a label uncertainty, and no quoted verdict margin is smaller
   than it.
4. `--overlay` renders agree visually with the hand-labelled frames on at least the six set-A
   face-on clips (`scripts/analyze_swing.py … --overlay <clip>`, the
   [M4_FUNDAMENTALS_PANEL.md](M4_FUNDAMENTALS_PANEL.md) precedent).
5. Nothing under `src/` changed: `git status` shows only `docs/`, `spikes/` and gitignored `data/`.
   `.venv\Scripts\python.exe -m pytest -q` still green.

### Repro

```powershell
# per clip — streaming pose, no run_pose.py OOM
.venv\Scripts\python.exe spikes\2026-08-07-two-phone\probe.py pose <clip.mov> --out data\processed\<name>.keypoints.json

# the shipped analysis path, unchanged — this is what Q1 actually asks about
.venv\Scripts\python.exe scripts\analyze_swing.py data\processed\<name>.keypoints.json
.venv\Scripts\python.exe scripts\analyze_swing.py data\processed\<name>.keypoints.json --overlay <clip.mov>

# Q2 + Q3
.venv\Scripts\python.exe spikes\2026-08-07-two-phone\probe.py inspect data\processed\sessions\<sid>\*\*.mov

# Q1 mechanism, candidate signals, and the verdict
.venv\Scripts\python.exe spikes\2026-08-07-two-phone\probe.py wrist data\processed\<name>.keypoints.json
.venv\Scripts\python.exe spikes\2026-08-07-two-phone\probe.py variants data\processed\<name>.keypoints.json
.venv\Scripts\python.exe spikes\2026-08-07-two-phone\probe.py score spikes\2026-08-07-two-phone\truth.json

# labelling
.venv\Scripts\python.exe spikes\2026-08-07-two-phone\probe.py frames <clip.mov> --out spikes\2026-08-07-two-phone\labels\<name>
```
