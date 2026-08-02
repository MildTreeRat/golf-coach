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

### Model: MediaPipe Pose Landmarker — "lite" variant (default)
The model is called the **Pose Landmarker** and comes in three sizes (speed ↔ accuracy):

| Variant | File | Size | Speed | Accuracy | Notes |
|---------|------|------|-------|----------|-------|
| **Lite (our default)** | `pose_landmarker_lite.task` | ~5 MB | Fastest | Good | Fine for the M1 skeleton PoC |
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
[docs/M1_CAPTURE_FLOW.md](../M1_CAPTURE_FLOW.md).

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
