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
