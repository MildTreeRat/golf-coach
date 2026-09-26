"""Strike detection, in Rust, reached over a pipe. [M20 P5]

This module replaces `audio/impact.py`, which was deleted in M20 P6 after
`crates/trigger` reproduced it on all thirty stored clips. What used to be numpy in this process
is now a subprocess: `golf-trigger detect` reads raw s16le PCM on stdin and writes strikes on
stdout.

**Why the detector moved and this did not.** ADR-030 §1 gives Rust "audio strike detection"; the
rule M20 lands under is narrower than "rewrite everything" — Python keeps only what does not
translate, which is MediaPipe pose and the bands cut from its landmarks. A spectral flux detector
is portable arithmetic plus one FFT, so it went. `audio/ffmpeg.py` has not gone, and this file is
the seam between the two: **Python still decodes, Rust detects.** The decoder holds the container
subtleties M11 paid for — edit lists, the two `soun` tracks the face-on clips carry, the
`video_start_seconds` probe — and reaches ffmpeg through the binary the `imageio-ffmpeg` wheel
ships rather than a system install. It is the next thing to move, not this milestone's.

**A subprocess rather than a native extension.** `audio/ffmpeg.py` already shells out in this
exact module, so the pattern is not new; and a PyO3 extension would make `pip install -e '.[audio]'`
require a Rust toolchain, which is a real regression for a lab environment that mostly wants to
read artifacts. The cost is one process spawn and one copy of the waveform per clip, against a
pipeline that already spends ~25 s per clip in pose.

**A missing binary is not an error here.** It is the same situation as the `audio` extra not being
installed, and it lands on the same path in `api/pipeline.py`: keep an older stored detection if
there is one, say so in a note, and never fall back to guessing. Hence `TriggerUnavailable`.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from array import array
from collections.abc import Sequence
from pathlib import Path

from golf_coach.config import REPO_ROOT
from golf_coach.contracts.audio import AudioStrike

#: Where a source checkout leaves the binary after `cargo build --release`.
#:
#: `REPO_ROOT` assumes a source checkout and so does this, which is the assumption ADR-030 records
#: as dying at packaging time (M26) — the shipped app has one process and no pipe. `PATH` is
#: consulted first so that a packaged or otherwise-installed binary wins without this constant
#: having to know where it went.
_BINARY_NAME = "golf-trigger.exe" if sys.platform == "win32" else "golf-trigger"
_BUILT = REPO_ROOT / "target" / "release" / _BINARY_NAME


class TriggerUnavailable(RuntimeError):
    """`golf-trigger` could not be found or could not be run.

    Deliberately *not* an `ImportError` subclass, though it lands on the same branch in
    `api/pipeline.py`. The two are different facts about the machine — one is a missing Python
    extra, the other a binary that has not been built — and a reader looking at the note the
    pipeline writes should be able to tell them apart. Making it an `ImportError` to save one
    clause in an `except` would have thrown that away.
    """


def binary() -> Path:
    """Where `golf-trigger` is, or raise `TriggerUnavailable` saying how to get one."""
    found = shutil.which(_BINARY_NAME) or shutil.which("golf-trigger")
    if found:
        return Path(found)
    if _BUILT.exists():
        return _BUILT
    raise TriggerUnavailable(
        f"{_BINARY_NAME} is not on PATH and {_BUILT} does not exist — run `cargo build --release`"
    )


def detect_strikes(samples: Sequence[int] | array, rate: int) -> list[AudioStrike]:
    """Every transient in one decoded clip, loudest first.

    The same contract `audio/impact.py::detect_strikes` had, because thirty golden vectors say the
    implementation behind it is the same function: `spec/vectors/audio/` was recorded from the
    Python reference and verified against the `{role}.audio.json` already on disk, and
    `crates/trigger` reproduces every sample index exactly and every float to a worst relative
    difference of 6.9e-15.

    **The first entry is not the ball.** A simulator bay produces four transients per shot and the
    screen strike outranks the ball on roughly half the clips measured, so this returns the list
    and lets the caller decide — `analysis/alignment.py::with_measured_impact` is what decides.

    `frame` is None on every strike: the detector sees a waveform and a sample rate and has never
    seen the video, so it cannot know the fps (`contracts/audio.py`). The caller holding the
    manifest fills it in.

    Raises `TriggerUnavailable` when the binary is missing or fails.
    """
    if rate <= 0:
        raise ValueError(f"rate must be positive, got {rate}")

    payload = samples.tobytes() if hasattr(samples, "tobytes") else array("h", samples).tobytes()
    try:
        done = subprocess.run(
            [str(binary()), "detect", "--rate", str(rate)],
            input=payload,
            capture_output=True,
            check=True,
        )
    except OSError as error:
        raise TriggerUnavailable(f"could not run {_BINARY_NAME}: {error}") from error
    except subprocess.CalledProcessError as error:
        detail = error.stderr.decode("utf-8", "replace").strip() or f"exit {error.returncode}"
        raise TriggerUnavailable(f"{_BINARY_NAME} failed: {detail}") from error

    try:
        reply = json.loads(done.stdout)
    except json.JSONDecodeError as error:
        raise TriggerUnavailable(f"{_BINARY_NAME} did not return JSON: {error}") from error
    return [AudioStrike.model_validate(strike) for strike in reply["strikes"]]
