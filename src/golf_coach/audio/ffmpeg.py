"""FfmpegAudioSource: decode one clip's audio track to mono PCM. [M11 P1]

An `AudioSource` adapter over the static ffmpeg binary that `imageio-ffmpeg` ships as a wheel,
so there is no system install to keep in step with the venv.

Requires the `audio` extra (`pip install -e '.[audio]'`), which is why this adapter is imported
directly (`from golf_coach.audio.ffmpeg import FfmpegAudioSource`) rather than re-exported from
the package `__init__` — importing the port stays dependency-free, exactly as `capture/file.py`
does for OpenCV. `tests/api/test_pipeline_imports.py` is what fails if `api/pipeline.py` ever
imports this at module scope.

**Why a demuxer and not a hand-rolled MOV reader.** The clips in this corpus carry non-uniform
audio edit lists (docs/M11_ACOUSTIC_SYNC.md §E2): most `soun` tracks start with 2112 samples of
AAC encoder priming — 44 ms, 2.64 frames at 60 fps — and at least one does not. A constant bias
would cancel between the two views and cost nothing; one that is 44 ms on one clip and 0 on the
other does not cancel, and it is the same order as the error this milestone exists to remove.
Applying an edit list correctly is a demuxer's job, and that is the argument for the dependency
rather than merely a convenience of it.

Verified through this decode path on 2026-08-29, decoding each track twice and differencing the
sample counts: `2026-08-23/2` skips 2112 samples on **both** views, so it cancels there;
`2026-08-23/9` skips 2112 on face-on and **0** on down-the-line, so it does not. That bundle is
one of the four carrying M10's late-top defect.

**And the video track has an edit list too, which is why `video_start_seconds` is here** [M11 P10].
Applying the audio one correctly put the samples on the *presentation* timeline; the frame indices
they are compared against come off a decoder that starts counting at its first decoded frame. On
four clips in this corpus those are not the same clock — see that function.
"""

from __future__ import annotations

import re
import subprocess
import sys
from array import array
from pathlib import Path

import imageio_ffmpeg

from golf_coach.audio.source import AudioClip, NoAudioTrackError

# One rate for everything downstream, so no consumer needs a per-clip case. 48 kHz is not a
# compromise here: every clip in the corpus is already 48 kHz stereo `mp4a` (§E1), so for today's
# footage this resamples nothing and only pins the contract for footage that arrives otherwise.
_TARGET_RATE = 48_000

# ffmpeg says this, on stderr, when `-map` selects a stream the file does not have. Matching on
# it is how a clip with no audio track is told apart from a broken decode — the two want
# different answers from the caller (see `NoAudioTrackError`), and ffmpeg exits non-zero for
# both. Lowercased before comparison because the wording's capitalisation has moved between
# ffmpeg releases and the static wheel's version is not ours to pin.
_NO_STREAM_MARKERS = ("matches no streams", "does not contain any stream")


class FfmpegAudioSource:
    """Decodes one clip's audio to mono 16-bit PCM. Implements the `AudioSource` port.

    Not a context manager, and not lazy: `read()` runs the decode and hands back the whole
    waveform. See `audio/source.py`'s module docstring for why this port is shaped differently
    from `VideoSource`.

        clip = FfmpegAudioSource("face_on.MOV", camera_id="face_on").read()
    """

    def __init__(
        self,
        path: str | Path,
        *,
        camera_id: str | None = None,
        stream_index: int = 0,
    ) -> None:
        self._path = Path(path)
        self._camera_id = camera_id
        self._stream_index = stream_index

    @property
    def path(self) -> Path:
        return self._path

    @property
    def camera_id(self) -> str | None:
        """Which camera this clip came from, or None if the caller didn't say (ADR-011)."""
        return self._camera_id

    @property
    def stream_index(self) -> int:
        """Which audio stream this source decodes, `0` being the first.

        Explicit, and defaulted rather than guessed: face-on clips here carry two `soun` tracks
        (iPhone spatial audio) whose edit offsets differ, so "the audio track" is not a
        well-defined phrase for them and picking by accident is a silent 44 ms (§E2). This
        adapter always decodes the stream it was asked for and records which one that was on the
        `AudioClip`; deciding *which* one is right is a question for the detection phases, which
        can read both and compare.
        """
        return self._stream_index

    def read(self) -> AudioClip:
        """Decode the whole audio track to mono PCM at `_TARGET_RATE`.

        Raises `FileNotFoundError` when the clip is missing, `NoAudioTrackError` when it carries
        no audio stream, and `OSError` when ffmpeg fails for any other reason.
        """
        if not self._path.exists():
            raise FileNotFoundError(f"Video file not found: {self._path}")

        completed = subprocess.run(
            self._command(),
            capture_output=True,
            check=False,
        )
        if completed.returncode != 0:
            raise self._decode_error(completed.stderr)

        samples = array("h")
        # `frombytes` needs a whole number of items; a truncated tail means ffmpeg was cut off
        # mid-sample, which is a decode failure wearing a success exit code.
        payload = completed.stdout
        if len(payload) % samples.itemsize:
            raise OSError(
                f"ffmpeg returned {len(payload)} bytes of 16-bit PCM for {self._path.name}, "
                "which is not a whole number of samples"
            )
        samples.frombytes(payload)
        if sys.byteorder != "little":
            # `-f s16le` is little-endian by name; `array("h")` is native. They agree everywhere
            # this project runs, so this branch is untested and is here to fail loudly rather
            # than quietly halve every amplitude if it ever does not.
            samples.byteswap()

        return AudioClip(
            samples=samples,
            sample_rate=_TARGET_RATE,
            camera_id=self._camera_id,
            stream_index=self._stream_index,
        )

    def _command(self) -> list[str]:
        """The ffmpeg argv. Three flags carry the design; the rest is quiet and non-interactive.

        Note what is *absent*: `-ignore_editlist`. ffmpeg applies the container's edit list by
        default, which is the whole point of decoding through it (see the module docstring), so
        the correctness here is in not switching it off.
        """
        return [
            imageio_ffmpeg.get_ffmpeg_exe(),
            "-hide_banner",
            "-nostdin",
            "-loglevel",
            "error",
            "-i",
            str(self._path),
            # Explicitly the Nth *audio* stream. Face-on clips carry two — the AAC stereo track
            # and an `apac` (Apple spatial audio) one this ffmpeg build has no decoder for — and
            # without `-map`, which one comes out is ffmpeg's stream-selection heuristic rather
            # than this project's decision. It happens to pick the AAC track today; that is not
            # a thing to depend on across ffmpeg builds when the wrong answer is silent.
            "-map",
            f"0:a:{self._stream_index}",
            "-vn",
            # Mono at one known rate, so nothing downstream needs a per-clip case. Onset
            # detection and cross-correlation both want a single envelope; a stereo pair would
            # only invite each consumer to invent its own downmix.
            "-ac",
            "1",
            "-ar",
            str(_TARGET_RATE),
            "-f",
            "s16le",
            "-acodec",
            "pcm_s16le",
            "-",
        ]

    def _decode_error(self, stderr: bytes) -> OSError:
        """The right exception for a non-zero ffmpeg, with its own words attached."""
        message = stderr.decode("utf-8", errors="replace").strip()
        haystack = message.lower()
        if any(marker in haystack for marker in _NO_STREAM_MARKERS):
            return NoAudioTrackError(
                f"{self._path.name} has no audio stream at index {self._stream_index}"
            )
        return OSError(f"ffmpeg could not decode audio from {self._path}: {message}")


# ffmpeg's `showinfo` filter prints one line per frame; this is the presentation timestamp on it,
# in seconds. Parsed rather than computed because the whole point of the call is to ask the
# demuxer what it did with the edit list, not to re-derive it (see `video_start_seconds`).
_PTS_TIME = re.compile(rb"pts_time:\s*([0-9]+\.?[0-9]*)")


def video_start_seconds(path: str | Path, *, stream_index: int = 0) -> float | None:
    """Presentation time of the **first frame a decoder hands back** from this clip. [M11 P10]

    Almost always 0.0, and the exceptions are why this function exists. A container can carry a
    leading *empty edit* — "show nothing for the first N ms" — and a decoder honours it by
    presenting its first frame at N rather than at zero. Nothing downstream counts frames that way:
    `capture/file.py` hands out frame 0, 1, 2 and `keypoints.ClipMetadata.fps` turns those into
    seconds from zero. So on such a clip a *sample* index (presentation time, because
    `FfmpegAudioSource` applies the edit list) and a *frame* index (decoder output order) describe
    two clocks N ms apart, and comparing them — which is exactly what M11 does when it pins tau=2
    to a heard strike — is wrong by N.

    **Measured over all 30 clips on disk, 2026-08-30.** 26 return 0.0. The four that do not are the
    2026-08-23 down-the-line clips of bundles 1, 7, 9 and 11 — the same four §E2 flagged for their
    odd structure, video timescale 19200 against everything else's 600 — at 0.107, 0.117, 0.105 and
    0.125 s. That is 6.3 to 7.5 frames at 60 fps, it is *not* a constant, and it is the error the
    eye check found on 2026-08-30 (docs/M11_ACOUSTIC_SYNC.md §Addendum): contact by eye at
    down-the-line frame 321-322 on bundle 9 against an audio anchor of 327.7.

    **Why ffmpeg and not the container.** The atom tree gives the same answer — the empty edit's
    duration, to the millisecond — but the question is what a *decoder* does with it, and that is
    not a fact about the file. Both decoders in this project agree here, verified frame-for-frame
    on 2026-08-30: `cv2.VideoCapture` and this binary return pixel-identical frames in the same
    order on bundle 9's down-the-line clip (cv2[i] matches ffmpeg's passthrough output i for
    i = 0, 1, 2 and nowhere else), and both drop the three samples the edit list trims from the
    head. cv2 simply never says so — `CAP_PROP_POS_MSEC` reads 0.0 on that clip's first frame,
    which is the whole reason the offset has to be measured through the other one.

    `-fps_mode passthrough` matters: ffmpeg's default output mode *duplicates* the first frame to
    fill the empty edit — seven copies of it on bundle 9 — and a timestamp read off that padding
    would be 0.0 and useless.

    Returns None when the clip has no such video stream, and when ffmpeg produces no timestamp at
    all. None is "nobody measured it", never zero: a caller that treats it as no offset is making
    the same assumption every reader made before this existed, and `AUDIO_DETECTOR_VERSION` is what
    makes sure the artifacts written under that assumption get re-measured. Raises `OSError` when
    ffmpeg fails for any reason other than a missing stream.
    """
    target = Path(path)
    if not target.exists():
        raise FileNotFoundError(f"Video file not found: {target}")

    completed = subprocess.run(
        [
            imageio_ffmpeg.get_ffmpeg_exe(),
            "-hide_banner",
            "-nostdin",
            # `showinfo` logs at info level, so this is the one call here that cannot be quiet.
            "-loglevel",
            "info",
            "-i",
            str(target),
            "-map",
            f"0:v:{stream_index}",
            "-fps_mode",
            "passthrough",
            "-vf",
            "showinfo",
            # One frame is the whole measurement, and it keeps this cheap enough to run beside
            # every decode: the first frame of a 4K HEVC clip is a keyframe.
            "-frames:v",
            "1",
            "-f",
            "null",
            "-",
        ],
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        message = completed.stderr.decode("utf-8", errors="replace").strip()
        if any(marker in message.lower() for marker in _NO_STREAM_MARKERS):
            return None
        raise OSError(f"ffmpeg could not read the video timebase of {target}: {message}")

    found = _PTS_TIME.search(completed.stderr)
    return float(found.group(1)) if found else None
