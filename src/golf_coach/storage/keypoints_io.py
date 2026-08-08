"""Reading and writing `*.keypoints.json`. [M7 Phase 1]

One place that knows the on-disk shape of pose output, because there are now two of them.

**The old shape** is a bare JSON array of frames, written by `scripts/run_pose.py` and by
`scripts/golfdb/extract_pose.py`. Hundreds of those files exist: `data/processed/*.keypoints.json`
and the 461-clip GolfDB reference cache under `data/reference/golfdb/keypoints/`, which cost hours
of estimator time to build and is not going to be regenerated.

**The new shape** wraps the same frames in an envelope carrying `ClipMetadata` — fps above all,
which until now lived only inside `FileVideoSource` and was thrown away when the CLI exited, even
though the alignment work downstream needs it.

`load_keypoints` accepts either and reports `clip=None` for the old one, so nothing has to be
migrated and no reader has to care which it got. Every consumer goes through here rather than
parsing the JSON itself — that is the whole point, and it is why `scripts/golfdb/bakeoff.py` and
friends dropped their local `TypeAdapter`s.

Pydantic and stdlib only, so the base install can read pose output (ADR-008).
"""

from __future__ import annotations

import json
from pathlib import Path

from pydantic import TypeAdapter

from golf_coach.contracts.keypoints import FrameKeypoints, KeypointsFile

_FRAMES_ADAPTER = TypeAdapter(list[FrameKeypoints])


def load_keypoints(path: str | Path) -> KeypointsFile:
    """Load a keypoints file in either the enveloped or the legacy bare-array shape.

    A legacy file comes back as a `KeypointsFile` with `clip=None`, which is accurate: those files
    genuinely do not record what clip they came from. Callers that only want the frames read
    `.frames`; callers that need fps check `.clip is not None` first.
    """
    source = Path(path)
    payload = json.loads(source.read_bytes())

    if isinstance(payload, list):
        return KeypointsFile(clip=None, frames=_FRAMES_ADAPTER.validate_python(payload))
    if isinstance(payload, dict):
        return KeypointsFile.model_validate(payload)
    raise ValueError(
        f"{source}: expected a keypoints object or a bare frame array, got {type(payload).__name__}"
    )


def save_keypoints(keypoints: KeypointsFile, path: str | Path, *, indent: int | None = 2) -> None:
    """Write a keypoints file in the enveloped shape.

    `exclude_none` keeps unknowns out of the file rather than writing a null for every one of them:
    a clip with no `camera_id` would otherwise carry several hundred `"camera_id": null` lines
    saying nothing. The only nullable fields in the tree are the ones the envelope introduced, so
    nothing meaningful can be dropped this way.
    """
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(keypoints.model_dump_json(indent=indent, exclude_none=True).encode("utf-8"))
