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
