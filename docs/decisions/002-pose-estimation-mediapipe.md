# ADR-002: Pose Estimation — MediaPipe

## Status
Accepted

## Date
2026-03-16

## Context
Need a model to extract body keypoints (skeleton) from video frames of a golf swing. Must run locally on consumer hardware.

## Options Considered

### Option A: MediaPipe Pose
- **Pros**: Free, runs locally, fast (real-time on CPU), 33 body landmarks, well-documented, Google-maintained. No training required — works out of the box.
- **Cons**: General-purpose (not golf-specific). May struggle with unusual angles or occlusion. Limited to single person.

### Option B: OpenPose
- **Pros**: Mature, widely used in research. Multi-person support.
- **Cons**: Slower than MediaPipe. More complex setup. License restrictions for commercial use (not relevant here but worth noting). Heavier compute requirements.

### Option C: MMPose
- **Pros**: Very flexible, many model architectures available. Research-grade accuracy.
- **Cons**: Steeper learning curve. More complex configuration. Heavier dependency chain.

### Option D: Train custom pose model
- **Pros**: Could be golf-specific.
- **Cons**: Requires massive labeled dataset. Months of work. No benefit over pretrained models for this use case.

## Decision
**MediaPipe Pose**. It provides 33 landmarks, runs in real-time on CPU, requires no training, and is the fastest path to a working prototype. If accuracy proves insufficient for specific golf positions (e.g., top of backswing with arms overhead), we can revisit.

## Consequences
- No model training needed for pose estimation — major time savings.
- 33 keypoints provide enough detail for swing analysis (hips, shoulders, elbows, wrists, knees, ankles).
- Limited to single-person detection (fine for home lab use).
- If MediaPipe accuracy is poor, switching to MMPose is a contained change (only the Pose module changes, interface contract stays the same).

---

## Addendum (2026-06-28): implementation specifics discovered during M1

The original 2026-03-16 decision settled on *MediaPipe* but not two things that M1
implementation forced us to pin down: **which MediaPipe API** and **which model variant**.

### MediaPipe API: Tasks API (not the legacy Solutions API)
MediaPipe ships two generations of API:
- **Legacy "Solutions" API** (`mp.solutions.pose.Pose`) — what most older tutorials use; it
  bundled its own model and applied built-in landmark smoothing.
- **Tasks API** (`mediapipe.tasks.python.vision.PoseLandmarker`) — the current, supported
  API; needs an explicit model asset and does less automatic smoothing.

The installed mediapipe (**0.10.35**, Python 3.13) has *removed* the legacy Solutions API
entirely — the module only exposes `Image`, `ImageFormat`, and `tasks`. So the Tasks API is
not a preference, it is the only option on current builds. We use `PoseLandmarker` in
`RunningMode.VIDEO` (frame-to-frame tracking). Implemented in `pose/estimator.py`.

### Model: MediaPipe Pose Landmarker — which variant is configuration

Which bundle runs is `settings.pose_model_variant` (`config.py`), not a constant in
`estimator.py`; read it there rather than here. The M1 choice, and what later measurement and
operation did to it, are below.
The model is called the **Pose Landmarker** and comes in three sizes (speed ↔ accuracy):

| Variant | File | Size | Speed | Accuracy | Notes |
|---------|------|------|-------|----------|-------|
| Lite | `pose_landmarker_lite.task` | ~5 MB | Fastest | Good | Fine for the M1 skeleton PoC |
| Full | `pose_landmarker_full.task` | ~9 MB | Medium | Better | Untried middle ground |
| Heavy | `pose_landmarker_heavy.task` | ~30 MB | 3–5× slower | Best on benchmarks | Tested on our first clip in M1 — did **not** improve lower-body/knee tracking; the weak spot there is the *recording* (lighting/contrast/clutter), not model size |

**Decision: default to lite.** The M1 accuracy review showed the lower-body weakness is a
picture problem, not model capacity (heavy didn't help and is much slower). Escalation path
if needed later: lite → full → heavy, then MMPose (per the original decision above).

### Where the model comes from
- **Source**: Google's official MediaPipe model storage —
  `https://storage.googleapis.com/mediapipe-models/pose_landmarker/<variant>/float16/latest/<file>`
- **Download**: `_ensure_model()` in `pose/estimator.py` fetches it on first run.
- **Stored at**: `data/models/` (gitignored; `.gitignore` has `data/models/*.task`).
- Swapping variants is a one-line change (`_MODEL_FILENAME` / `_MODEL_URL`); the
  `FrameKeypoints` contract is unaffected.

Operational reference + the full M1 accuracy-review findings:
[docs/M1_CAPTURE_FLOW.md](../archive/M1_CAPTURE_FLOW.md).

---

## Addendum (2026-08-02): the variant choice, measured against ground truth [M4-REF Phase B0]

The M1 addendum above picked **lite** and rejected **heavy** on the basis of *one clip judged by
eye*. GolfDB supplies hand-annotated event frames for 461 face-on tour swings, so that judgement
could finally be given a number. Full findings and method:
[docs/M4_POSE_BAKEOFF.md](../M4_POSE_BAKEOFF.md); raw results: `docs/pose_bakeoff_v1.json`.

**Method.** Each variant's cached keypoints are pushed through the *real*
`smooth_keypoints` → `segment_phases` path and scored on how well it recovers GolfDB's labelled
address / top / impact, reported as PCE (tolerance scales with swing duration, so slow-motion and
real-time clips are judged alike). This measures the thing we actually consume — does this pose feed
our metrics — rather than COCO AP on generic photographs.

| variant | mean PCE (120 clips) | speed | verdict |
|---|---|---|---|
| **lite** | 53.6% | **~106 fps** | **retained** |
| full | 54.7% | 83 fps | not adopted |
| heavy | 53.9% | **24 fps** | not adopted |
| RTMPose-m (`rtmlib`) | 30.0% | 118 fps | **rejected** |

**Decision: keep lite. No change to `estimator.py`.**

- **No MediaPipe variant differs significantly from another on any event.** Twelve paired exact
  McNemar tests (nine at n=120, three at n=461); none reach p < 0.05.
- **full's one promising result did not survive a larger sample.** At n=120 it led lite by +9.1pp at
  the top (p=0.099); extended to all 461 clips that shrank to +1.9pp (p=0.494) and lite led on mean
  PCE, 52.4% vs 51.5%.
- **heavy costs 4.4x lite and buys nothing** (53.9% vs 53.6%, and the worst address error of the
  three). **The M1 judgement was right** — this is the evidence it was missing.

### Option C ("MMPose / RTMPose") — the dependency objection is stale, the accuracy case is not

The original decision rejected Option C partly for a "heavier dependency chain". That objection is
**factually obsolete**: `rtmlib` is Apache-2.0 and pulls only numpy, opencv and onnxruntime — no
mmcv, no mmpose, no torch. It was correspondingly cheap to actually test.

It lost anyway, by **24.7pp**, far outside the pre-registered "adopt only on >= 3pp PCE" bar set
before any measurement. Verified not to be an adapter bug: its lead-wrist trajectory correlates with
lite's at median 0.948, so it tracks the same motion — 1.5x noisier at the wrist, 2.4x at the
shoulder and **4.3x at the hip**, which would have degraded the pose bands as well as segmentation.
The likely cause is that MediaPipe runs in `RunningMode.VIDEO` with cross-frame tracking while
RTMPose here is per-frame; `rtmlib` ships a `PoseTracker` that would narrow the gap, and a rematch
would need it plus a person detector and would still have to find ~25pp.

One incidental result worth keeping: rtmlib's `Body` wrapper re-runs a YOLOX-m detector at 640x640
on *every frame*, which cost **35x** the pose model itself (3.1 fps vs 118 fps once skipped). On
pre-cropped clips the detector is re-deriving a constant. Full-frame video would still need one.

**Escalation path, updated.** lite → full → heavy is no longer an accuracy ladder for *event
recovery*; the three are indistinguishable. Revisit the variant choice only for a checkpoint that
depends on absolute landmark positions (spine angle, hip rotation) rather than trajectory shape,
where this bake-off is silent — and re-run it against that metric.


## Addendum (2026-08-30): the variant became configuration, and the bay runs heavy

The bake-off above is unchanged and still says what it said: on *event recovery* the three
MediaPipe variants are indistinguishable, and heavy costs 4.4x lite for nothing. This addendum
does not overturn that. It records that the variant stopped being a constant and that the sim bay
is now asked to run **heavy** anyway, for a reason the bake-off did not measure.

**What changed.** `_MODEL_FILENAME`/`_MODEL_URL` became `settings.pose_model_variant` plus a URL
template, so the choice moves with `GOLF_POSE_MODEL_VARIANT` and not with an edit. Each variant
caches under its own filename in `models_dir`, so switching back is free after the first download.
The bake-off's own copy of that downloader — which had none of `ensure_pose_model`'s temp-file
guard — went with it; one file, one downloader.

**Why heavy, against the measurement.** The complaint that prompted it is landmark *steadiness* —
"the dots" — on this bay's own phone clips. Phase B0 scored PCE: whether a variant recovers the
right frame for address, top and impact on GolfDB's 160x160 tour crops. Those are different
questions, and the escalation path at the end of the previous addendum names exactly this gap:
revisit the variant for something depending on absolute landmark positions rather than trajectory
shape, "where this bake-off is silent". It is still silent. Heavy here is an operator preference
with a plausible mechanism, not a measured win — and if it is ever to become a measured one, the
missing instrument is a landmark-jitter comparison on *bay* footage (per-frame displacement of a
landmark against its own smoothed track), not another PCE run.

**What it costs, and what the code does about it.** Three things follow, and none of them are
optional:

- **Speed.** ~24 fps against lite's ~106 on the bake-off hardware. A bay clip is minutes of pose,
  not tens of seconds, and the pipeline caches it precisely so that is paid once per clip.
- **Caches.** `ClipMetadata.source_sha256` keys the stored `*.keypoints.json` on the *footage*, so
  it cannot see a changed instrument. `KeypointsFile.pose_estimator` was added beside it and
  `api.pipeline.keypoints_for` re-runs pose when the two disagree — otherwise every bundle already
  on disk would have gone on serving lite landmarks with nothing saying so. Same shape, same
  reason, as `AUDIO_DETECTOR_VERSION`. `ANALYSIS_VERSION` 13 -> 14 is the matching gate a layer up.
- **Bands.** This is the real cost. `ranges.json` is cut from GolfDB clips extracted with
  `mediapipe:lite`, and ADR-012 §4 is explicit that a band is only comparable to a swing measured
  the same way — the estimator's bias is common-mode across both sides and cancels, right up until
  the two sides diverge. They now diverge. `api.pipeline._band_estimator_note` says so on every
  affected result rather than leaving it to be inferred from two provenance fields in different
  files. **The honest fix is to re-derive the corpus under the running variant**
  (`scripts/golfdb/derive_pose_metrics.py --estimator mediapipe:heavy`, then
  `derive_reference.py`) — roughly 1400 s per 120 clips at heavy's rate, and until it is run the
  scores are approximate in a way the note names.

**Measured after the switch: heavy drops the tour-trajectory placements, and it is not steadier
where it matters.** The corpus re-analysis lost `tour_trajectory_t2` / `_q` on all 15 stored
bundles and the `_dtl` pair on 11 of them. The mechanism is not a bug: `analysis/trajectory.py`
discards a landmark column when more than `MAX_MISSING` of its sampled steps fall under
`MIN_VISIBILITY`, and on `2026-08-23/4` face-on heavy puts right_elbow over at 23/40 and
right_wrist at 21/40, so `build_trajectory` returns None. Re-run on the *same* clip with the same
anchors, lite clears every column with none below the floor.

The comparison worth keeping, because it says heavy is not simply "better but shy" (median
visibility over motion_start..impact; position agreement in normalized frame units; jitter as
median frame-to-frame displacement):

| landmark | vis lite -> heavy | steps under the floor | median \|dx\| | jitter lite -> heavy |
|---|---|---|---|---|
| right_elbow | 0.958 -> 0.987 | 23/40 | 0.005 | 0.00083 -> 0.00073 |
| right_wrist | 0.947 -> 0.987 | 21/40 | 0.013 | 0.00202 -> 0.00188 |
| left_wrist | 0.962 -> 0.681 | 13/40 | 0.006 | 0.00232 -> 0.00374 |
| left_knee | 0.987 -> 0.768 | 15/40 | 0.005 | 0.00035 -> 0.00032 |
| left_shoulder | 1.000 -> 1.000 | 0/40 | 0.004 | 0.00038 -> 0.00058 |

Three things follow. **Heavy is not mislocating joints** — the two variants agree to 0.4-1.3% of
frame width, so this is a confidence disagreement, not a tracking one. **Heavy's confidence is
bimodal**: median *higher* than lite on the trail arm while collapsing under the floor on more
than half the sampled steps, which is it giving up through the fast part of the downswing rather
than being uniformly cautious. And **the jitter result is mixed, against heavy where it counts** —
marginally steadier on the trail arm, 1.6x jitterier on the lead wrist and 1.5x on the lead
shoulder, and the lead wrist is what `phases.segment_phases` and `select_swing` read.

**This is accepted, not fixed** (operator's call, 2026-08-31): the trajectory placements are not
wanted right now, so the corpus runs at 17 measurements rather than 19 and no threshold was moved.
Raising `MIN_VISIBILITY` for heavy was considered and declined — the floor gates every metric in
the analysis core, not just this one, so admitting heavy's given-up frames would trade a visible
failure for a silent one. One clip and five landmarks is also all this measures; it is the clip
that failed, not a sample.

## Addendum (2026-09-21, M18): pose stays here, and "here" becomes a sidecar process

[ADR-030](030-app-platform-rust-core-python-sidecar.md) moves the backend to Rust. **This ADR's
decision is unchanged in every particular** — MediaPipe, the Tasks API, `RunningMode.VIDEO`, the
`.task` bundles, and `settings.pose_model_variant` with the bay running `heavy`. What changes is
only the *process* that calls them: instead of being imported by the pipeline, `pose/estimator.py`
is driven by a long-lived pool of Python workers that a Rust core sends jobs to.

**This addendum exists to record why Rust does not get pose**, because that is the decision the
port could most easily have got wrong. This ADR's 2026-08-02 addendum already measured the cost of
a different pose pipeline: RTMPose through `onnxruntime` lost by **24.7pp** on event recovery and
ran 1.5x noisier at the wrist, 2.4x at the shoulder and 4.3x at the hip. `ranges.json` is cut from
*this* estimator's landmark output, so a Rust reimplementation through ONNX would put every band
back in question to save shipping an interpreter. ADR-030 judged that trade the wrong way round.

Two mechanics from this ADR become constraints on the pool, and both are consequences of
`RunningMode.VIDEO`'s cross-frame tracking:

- **A worker takes one clip start to finish**, and gets a fresh `PoseLandmarker` between jobs.
  Tracking state and the strictly-increasing-timestamp rule in `pose/estimator.py` mean two clips
  cannot be interleaved on one instance. It is the *process* that stays warm — the interpreter and
  the ~30 MB heavy bundle — not the landmarker.
- **The reply is a `KeypointsFile` as `contracts/keypoints.py` already defines it**, `pose_estimator`
  stamp included. That makes the sidecar's output byte-comparable against the 30 keypoint files
  already on disk, which is how the new stack is verified rather than trusted.

The ~24 fps this ADR measured for `heavy` is the throughput the whole live design is built around:
a 10 s 60 fps clip is roughly 25 s of pose, a two-view swing roughly 50 s. The `lite` variant's
~4x speed at no significant cost on event recovery is also ADR-030's answer for a slow laptop, in
place of the cloud worker it declined to build.
