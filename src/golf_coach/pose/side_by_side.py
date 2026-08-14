"""Two aligned views of one swing, drawn into a single side-by-side clip. [M7 Phase 2/4]

The visible correctness claim of the whole alignment engine: the ADDRESS / TOP / IMPACT banners
appear on both panels **on the same output frame**. This module does not work that out — it is
handed a schedule of `FramePairing`s by `analysis.alignment.pair_frames`, each carrying one
`tau` shared by both panels, so if the alignment is right the banners cannot disagree. Drawing
and warp arithmetic stay in different modules (ADR-008), which also means the correspondence is
testable without decoding a frame.

Frames are taken as *iterables*, not paths, mirroring `estimator.estimate_pose` — opening files
belongs to the caller (a script, or the API's worker). Pixels are optional: with no frames the
skeletons render on a black canvas, which still proves the alignment and works on keypoints
whose clips are long gone.

**Both clips stream.** A render schedule is monotone in each clip, so the decoders are only ever
asked to move forward and neither clip is ever buffered. That matters — two 1080p60 clips held
in memory is ~5.8 GB and 4K is far worse. It is the same bug M7 Phase 1 removed from
`run_pose.py`; re-introducing it here would undo that.

Needs the `vision` extra (OpenCV + numpy), imported lazily so importing this module stays cheap.
"""

from __future__ import annotations

from collections.abc import Iterable, Iterator, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING, Any, NamedTuple

from golf_coach.contracts.alignment import (
    TAU_IMPACT,
    TAU_MOTION_START,
    TAU_TOP,
    AlignmentQuality,
    FramePairing,
)
from golf_coach.contracts.keypoints import ClipMetadata, FrameKeypoints
from golf_coach.pose.overlay import annotate_frame

if TYPE_CHECKING:
    import numpy as np

    # Type-only for the same reason numpy is: `Frame` holds a numpy image, and this module
    # advertises that importing it stays cheap. See `estimator.py`'s note.
    from golf_coach.capture.source import Frame

# Height each panel is scaled to before the two are joined. 4K portrait phone footage is 3840
# tall; writing that verbatim produces a gigantic file nobody can scrub through, and the
# alignment is just as visible at 720.
_PANEL_HEIGHT = 720

# Fallback canvas when there is no video and the clip metadata never recorded a size.
_DEFAULT_CANVAS = (720, 1280)  # (height, width)

# How wide a tau window each banner stays up for, as a fraction of one downswing. ~0.13 is two
# frames at a typical 15-frame 60fps downswing — long enough to see when scrubbing, short enough
# that TOP and IMPACT never overlap.
_BANNER_HALF_TAU = 0.13

_BANNERS: tuple[tuple[float, str], ...] = (
    (TAU_MOTION_START, "ADDRESS"),
    (TAU_TOP, "TOP OF BACKSWING"),
    (TAU_IMPACT, "IMPACT"),
)

# Tried in order. `mp4v` (MPEG-4 Part 2) is what this module used to write unconditionally, and
# it plays in VLC but *not* in Chrome or Firefox — which stops mattering the moment a results
# page puts the clip in a <video> tag. `avc1` is H.264, which every browser plays.
#
# Whether avc1 opens depends on the machine, not the code, so the fallback is real rather than
# decorative. On Windows expect a wall of stderr on every render:
#
#     Failed to load OpenH264 library: openh264-1.8.0-win64.dll
#     VIDEOIO/FFMPEG: Failed to initialize VideoWriter
#
# That is OpenCV's *bundled FFmpeg* giving up — it carries no libx264 (GPL), only libopenh264,
# which dlopen's a DLL that is not shipped. It is noise, not failure: OpenCV then falls back to
# Media Foundation, whose native H.264 encoder opens fine and honours the requested frame rate
# exactly. Verified on this repo's own footage — the output really is `avc1`, 85/85 frames.
# Dropping the Cisco DLL on the path silences the noise; nothing else changes.
#
# `RenderResult.codec` records which one actually won, because `isOpened()` is the only thing
# worth believing here and the stderr above will happily contradict a perfectly good file.
_CODECS: tuple[str, ...] = ("avc1", "mp4v")

# The codec a browser will not play inline. Callers warn on this rather than test `!= "avc1"`,
# so adding a third acceptable codec here doesn't silently start warning about it.
BROWSER_HOSTILE_CODECS = frozenset({"mp4v"})


class RenderResult(NamedTuple):
    """What was written, and in which codec — `codec` is None when nothing was written."""

    frames: int
    codec: str | None


@dataclass
class Panel:
    """One view's contribution to the output: its skeletons, its pixels, and how to label it."""

    keypoints: list[FrameKeypoints]
    label: str
    clip: ClipMetadata | None = None
    frames: Iterable[Frame] = field(default_factory=tuple)


def render_side_by_side(
    out_path: Path,
    schedule: Sequence[FramePairing],
    a: Panel,
    b: Panel,
    *,
    fps: float = 60.0,
    quality: AlignmentQuality = AlignmentQuality.UNALIGNED,
) -> RenderResult:
    """Write the aligned side-by-side MP4. Returns the frames written and the codec used.

    An empty `schedule` writes nothing and returns `RenderResult(0, None)` — the two clips shared
    no overlapping swing time, which the caller reports rather than this raising (ADR-013).
    """
    import cv2
    import numpy as np

    if not schedule:
        return RenderResult(0, None)

    pull_a = _FramePuller(iter(a.frames), np.zeros((*_canvas_size(a.clip), 3), dtype=np.uint8))
    pull_b = _FramePuller(iter(b.frames), np.zeros((*_canvas_size(b.clip), 3), dtype=np.uint8))
    writer: cv2.VideoWriter | None = None
    codec: str | None = None

    try:
        for tau, frame_a, frame_b in schedule:
            banner = _banner_for(tau)
            panel_a = _draw(pull_a.at(frame_a), a, frame_a, banner, ())
            panel_b = _draw(
                pull_b.at(frame_b), b, frame_b, banner, (f"tau {tau:+.2f}  {quality.summary}",)
            )

            joined = np.hstack([panel_a, panel_b])
            if writer is None:
                height, width = joined.shape[:2]
                writer, codec = _open_writer(out_path, fps, (width, height))
            writer.write(joined)
    finally:
        if writer is not None:
            writer.release()

    return RenderResult(len(schedule), codec)


def _open_writer(
    out_path: Path, fps: float, size: tuple[int, int]
) -> tuple[Any, str]:
    """First codec in `_CODECS` that actually opens, and its name.

    A refused `VideoWriter` still leaves a zero-byte stub behind, so each failed attempt is
    released and unlinked before the next is tried — otherwise a successful `mp4v` would be
    written on top of avc1's debris and `out_path`'s mtime would lie about which one won.
    """
    import cv2

    for tag in _CODECS:
        # `VideoWriter.fourcc`, not the older module-level `VideoWriter_fourcc`: same value, but
        # the free function is missing from opencv's type stubs and this is library code rather
        # than a script, so it is type-checked.
        writer = cv2.VideoWriter(str(out_path), cv2.VideoWriter.fourcc(*tag), fps, size)
        if writer.isOpened():
            return writer, tag
        writer.release()
        out_path.unlink(missing_ok=True)

    raise RuntimeError(
        f"OpenCV would not open a video writer for {out_path.name} in any of {_CODECS}"
    )


def _draw(
    image: np.ndarray[Any, Any],
    panel: Panel,
    frame: int,
    banner: str | None,
    extra_hud: tuple[str, ...],
) -> np.ndarray[Any, Any]:
    """Scale FIRST, then annotate.

    Landmarks are normalized to [0, 1] so the skeleton lands identically either way — but
    `annotate_frame` draws text at a fixed pixel size, and text drawn on a 3840-tall 4K frame is
    unreadable once the panel is shrunk to 720. Annotating the scaled frame keeps the HUD legible
    at output size.
    """
    return annotate_frame(
        _scaled(image),
        panel.keypoints[min(frame, len(panel.keypoints) - 1)],
        banner=banner,
        hud_lines=(f"{panel.label}  frame {frame}", *extra_hud),
    )


class _FramePuller:
    """Pull frames from a one-shot decoder at non-decreasing indices, holding the last.

    A repeated index re-serves the frame already in hand, which is what happens whenever the
    reference clip is the faster of the two.

    **It always decodes from frame 0**, so a swing 3,000 frames into a 4K clip costs a minute of
    decoding before the first output frame appears. `VideoSource` is a forward-only port with no
    seek, deliberately (a live camera cannot seek), and adding one is a capture-layer change
    rather than a rendering one. Streaming is what keeps memory flat; the wait is the price.
    """

    def __init__(self, frames: Iterator[Frame], blank: np.ndarray[Any, Any]) -> None:
        self._frames = frames
        self._current: np.ndarray[Any, Any] = blank
        self._index = -1

    def at(self, index: int) -> np.ndarray[Any, Any]:
        while self._index < index:
            frame = next(self._frames, None)
            if frame is None:
                break  # clip ended early; hold the last frame we did get
            self._current = frame.image
            self._index += 1
        return self._current


def _banner_for(tau: float) -> str | None:
    for anchor_tau, label in _BANNERS:
        if abs(tau - anchor_tau) <= _BANNER_HALF_TAU:
            return label
    return None


def _canvas_size(clip: ClipMetadata | None) -> tuple[int, int]:
    """(height, width) for the black-canvas fallback, from the clip metadata when it exists."""
    if clip is not None and clip.width and clip.height:
        return clip.height, clip.width
    return _DEFAULT_CANVAS


def _scaled(panel: np.ndarray[Any, Any]) -> np.ndarray[Any, Any]:
    import cv2

    height, width = panel.shape[:2]
    scale = _PANEL_HEIGHT / height
    return cv2.resize(panel, (max(1, int(width * scale)), _PANEL_HEIGHT))
