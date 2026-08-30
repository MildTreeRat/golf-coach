"""Audio module — the second I/O edge, and the only shared clock the two phones have. [M11]

Defines the `AudioSource` *port* and its adapters, mirroring `capture/` deliberately: consumers
depend on the port, the decoder dependency stays behind the `audio` extra, and what crosses into
`analysis/` is plain data (a frame index and a confidence), never a decoder (ADR-008).

Why this module exists at all is ADR-015's parked Option C: both phones record the ball strike,
so a physical event neither camera can misinterpret can anchor them to each other.
"""

from golf_coach.audio.source import AudioClip, AudioSource, NoAudioTrackError

__all__ = ["AudioSource", "AudioClip", "NoAudioTrackError"]
