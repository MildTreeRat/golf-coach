"""Detection module — YOLOv8 club/ball detection + tracking. [M2]

Turns frames into `list[FrameDetections]`, then a tracker links club-head detections
across frames into the club-path arc. The only place Ultralytics is imported.
Gated on the M1.5 detectability spike (ADR-007).
"""
