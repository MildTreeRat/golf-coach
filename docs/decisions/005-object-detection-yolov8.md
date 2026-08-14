# ADR-005: Object Detection — YOLOv8

## Status
Accepted

## Date
2026-03-16

## Context
Need to detect and track the club head and golf ball in video frames. This is the model we will train ourselves (unlike pose estimation which uses a pretrained model).

## Options Considered

### Option A: YOLOv8 (Ultralytics)
- **Pros**: State-of-the-art speed and accuracy. Easy fine-tuning API (`model.train(data='dataset.yaml')`). Active development. Excellent documentation. Supports tracking out of the box (ByteTrack, BoTSORT).
- **Cons**: May be overkill for 2-class detection. Requires labeled dataset.

### Option B: Faster R-CNN / RetinaNet (via Detectron2)
- **Pros**: Well-understood architectures. Good accuracy.
- **Cons**: Slower inference. More complex setup. Less beginner-friendly than Ultralytics.

### Option C: Custom lightweight CNN
- **Pros**: Minimal, purpose-built.
- **Cons**: Requires more ML expertise to design. Likely worse accuracy than pretrained + fine-tuned YOLO. More work for less result.

### Option D: Classical CV (color thresholding, edge detection)
- **Pros**: No ML training needed. Simple.
- **Cons**: Fragile — breaks with lighting changes, backgrounds, etc. Can't reliably detect club head in motion blur.

## Decision
**YOLOv8** via Ultralytics. The fine-tuning workflow is straightforward, it supports built-in tracking, and this is a great opportunity to learn the object detection training pipeline end-to-end.

## Consequences
- Need to collect and label 200-500 images of club heads and golf balls.
- Labeling tool needed (Label Studio or Roboflow — separate decision, low stakes).
- Training can run on a consumer GPU or even CPU (small dataset, few classes).
- Built-in tracker (ByteTrack) gives us club path for free once detection works.

## Addendum (2026-08-14): the M1.5 spike says not yet — and the blocker is not the detector

The labelling effort above is **deferred on evidence**. M1.5 ran against four real bay clips and
found that the club head, while a perfectly good target at rest (42 px across at 4K, crisp), is
smeared across 600–980 px at impact by the 1/60 s exposure the capture uses — 14x to 23x its own
size, a translucent band with no boundable head. Labelling those frames would teach a model the
smear rather than the club.

Nothing about that is a YOLOv8 problem, and nothing about it is fixed by a different detector or
by a marker on the club head: **the binding constraint is exposure time, measured at ~1/2000 s.**
This ADR's choice of YOLOv8 is not overturned — it is simply not yet startable, and the thing
that would start it is light, not architecture. See
[ADR-017](017-club-head-detection-strategy.md), which also records why the interim path is
fusion + interpolation rather than the marker-assisted option the spike's own threshold table
pointed at.
