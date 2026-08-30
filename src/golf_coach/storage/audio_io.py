"""Reading and writing `{role}.audio.json`. [M11 P2]

One place that knows the on-disk shape of acoustic output, written against
`storage/keypoints_io.py` — the same envelope, the same `exclude_none` dump, the same rule that
every consumer comes through here rather than parsing the JSON itself.

**Where it deliberately differs, and why.** `load_keypoints` accepts a bare JSON array because
hundreds of pre-envelope files exist that nobody is going to regenerate. Audio has no such history:
it was born enveloped in this phase, so a bare array here would not be tolerance of a real file, it
would be a shape invented out of symmetry that then has to be supported forever. A future legacy
shape gets absorbed at the same seam this one is — the `isinstance` branch below — and that is the
line to add it on.

Pydantic and stdlib only, so a base install with no ffmpeg and no numpy can read what the `audio`
extra wrote (ADR-008). That split is the point: decoding needs the extra, reading never does.
"""

from __future__ import annotations

import json
from pathlib import Path

from golf_coach.contracts.audio import AudioFile


def load_audio(path: str | Path) -> AudioFile:
    """Load an audio file, reporting `clip=None` when the decode was not recorded.

    An empty `strikes` list comes back as an empty list, which is the honest answer and not a
    missing one: the file only exists once detection has run, so it means no transient was found
    (`contracts.audio.AudioFile`). Callers that need the sample rate check `.clip is not None`
    first, exactly as they do for keypoints.
    """
    source = Path(path)
    payload = json.loads(source.read_bytes())

    if isinstance(payload, dict):
        return AudioFile.model_validate(payload)
    raise ValueError(f"{source}: expected an audio object, got {type(payload).__name__}")


def save_audio(audio: AudioFile, path: str | Path, *, indent: int | None = 2) -> None:
    """Write an audio file.

    `exclude_none` for `save_keypoints`'s reason — unknowns stay out of the file instead of writing
    a null that says nothing — and it matters more here: `AudioStrike.frame` is None on every strike
    until an fps is known, and a clip with a dozen candidates would otherwise carry a dozen
    `"frame": null` lines. Omitted and null both read back as None, so nothing is lost.
    """
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(audio.model_dump_json(indent=indent, exclude_none=True).encode("utf-8"))
