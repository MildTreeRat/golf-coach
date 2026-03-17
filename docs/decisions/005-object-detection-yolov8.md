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
