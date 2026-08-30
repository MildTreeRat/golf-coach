"""AudioSource port: the interface every audio source implements. [M11]

Adapters:
  - FfmpegAudioSource (audio/ffmpeg.py) — decode one clip's audio track  [M11 P1, today]

Samples cross this seam as a stdlib `array.array("h")` of mono 16-bit PCM, not a numpy array.
That is the same rule `capture/source.py` follows for pixels, one step stricter: `capture` may
annotate `np.ndarray` because the `vision` extra is already installed wherever frames exist, but
audio detection (audio/impact.py) wants numpy while the *port* must stay importable on a base
install. An `array("h")` costs nothing to hand over — `np.frombuffer(clip.samples, "<i2")` is a
zero-copy view of it — and keeps the numpy dependency on one side of the seam.

Unlike `VideoSource`, this port is **not** a context manager and does not stream. Two reasons,
both measured rather than stylistic: a decoded swing clip is ~1 MB of PCM where a decoded one is
hundreds of MB of pixels, and both consumers downstream (strike detection and cross-correlation)
need the whole waveform at once anyway. Symmetry for its own sake would buy a `with` block that
holds nothing open.
"""

from __future__ import annotations

from array import array
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol, runtime_checkable


class NoAudioTrackError(OSError):
    """The clip opened fine and has no audio stream to decode.

    Separate from a plain decode failure because the two want different answers: a clip with no
    audio track is a bundle that simply cannot be acoustically anchored (say so in a note and
    fall back), while a decode failure is a broken tool or a corrupt file (worth surfacing).
    Measured 0/30 in this corpus as of 2026-08-29 (docs/M11_ACOUSTIC_SYNC.md §E1), which is
    exactly why it needs naming — an unmeasured path is the one that rots.
    """


@dataclass(frozen=True)
class AudioClip:
    """One clip's audio as mono PCM, after the container's edit list has been applied.

    `samples` is signed 16-bit, one channel, at `sample_rate` Hz.
    """

    samples: array[int]
    sample_rate: int
    camera_id: str | None = None
    """Which camera this clip came from, or None when the caller didn't say (ADR-011).

    Free-form for `capture.Frame.camera_id`'s reason: `audio` must not import `storage` to borrow
    its `Role` values (ADR-008).
    """
    stream_index: int = 0
    """Which audio stream was decoded.

    Not decoration — face-on clips in this corpus carry two `soun` tracks with *different* edit
    offsets (docs/M11_ACOUSTIC_SYNC.md §E2), so a clip's samples are only reproducible alongside
    the index they came from.
    """

    @property
    def duration_s(self) -> float:
        """Length of the decoded waveform in seconds; 0.0 for an empty or rate-less decode.

        Derived from the sample count rather than read from the container, so it describes the
        waveform actually handed over — including the edit list's effect on it, which the
        container's own `duration` field does not.
        """
        if self.sample_rate <= 0:
            return 0.0
        return len(self.samples) / self.sample_rate


@runtime_checkable
class AudioSource(Protocol):
    """A source of one clip's audio."""

    @property
    def path(self) -> Path: ...

    @property
    def camera_id(self) -> str | None: ...

    def read(self) -> AudioClip:
        """Decode the whole track to mono PCM.

        Raises `NoAudioTrackError` when the clip carries no audio stream, and `OSError` when the
        decode itself fails. Never returns a silent buffer to stand in for either — a zero-filled
        waveform would cross-correlate to a confident, meaningless offset (ADR-010 §2).
        """
        ...
