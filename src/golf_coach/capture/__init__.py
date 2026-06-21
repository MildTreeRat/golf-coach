"""Capture module — video input (the imperative shell's camera edge).

Defines the VideoSource *port* and its adapters. Consumers depend on the port, so we
can feed the pipeline from a sample file today and a live ELP camera later without
changing anything downstream (ADR-007).
"""

from golf_coach.capture.source import Frame, VideoSource

__all__ = ["VideoSource", "Frame"]
