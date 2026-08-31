"""FfmpegAudioSource tests. Skipped cleanly when the audio extra (imageio-ffmpeg) isn't installed.

Fixtures are synthesised with the same static binary the adapter decodes with, the way
`tests/capture/test_file_source.py` writes its clips with the OpenCV it reads them back with. A
recorded fixture would be truer to the corpus, but the container facts these tests pin — encoder
priming, two audio streams, no audio stream at all — are all reproducible from a synthesised file,
and a 4K MOV in the repo is not.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

imageio_ffmpeg = pytest.importorskip("imageio_ffmpeg")

from golf_coach.audio.ffmpeg import (  # noqa: E402  (after importorskip)
    FfmpegAudioSource,
    video_start_seconds,
)
from golf_coach.audio.source import AudioSource, NoAudioTrackError  # noqa: E402

_TARGET_RATE = 48_000


def _ffmpeg(*args: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [imageio_ffmpeg.get_ffmpeg_exe(), "-y", "-hide_banner", "-loglevel", "error", *args],
        capture_output=True,
    )


def _write_tone(path: Path, *, seconds: float = 1.0, rate: int = 44_100) -> None:
    """An AAC-in-MOV tone. Encoding through AAC is what gives the file a real edit list.

    The rate defaults to something *other* than the adapter's target so every decode in this file
    also exercises the resample, and the channel count is stereo for the same reason.
    """
    done = _ffmpeg(
        "-f",
        "lavfi",
        "-i",
        f"sine=frequency=440:sample_rate={rate}:duration={seconds}",
        "-ac",
        "2",
        "-c:a",
        "aac",
        str(path),
    )
    if done.returncode != 0:
        pytest.skip(f"No AAC encoder available in this environment: {done.stderr!r}")


def _write_silent_video(path: Path) -> None:
    """A clip with a video track and no audio track at all."""
    done = _ffmpeg(
        "-f",
        "lavfi",
        "-i",
        "color=c=black:s=64x48:d=0.5:r=30",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        str(path),
    )
    if done.returncode != 0:
        pytest.skip(f"No H.264 encoder available in this environment: {done.stderr!r}")


def _write_video_starting_late(path: Path, *, delay_s: float = 0.1) -> None:
    """A clip whose video track starts `delay_s` into the presentation timeline.

    `-itsoffset` on the video input makes the MOV muxer write the same container shape the four
    odd clips in this corpus carry: a leading *empty edit*, "show nothing until here", followed by
    the media (docs/M11_ACOUSTIC_SYNC.md §E2, where the real ones measure 105-125 ms). The audio
    comes from a second input with no offset, so the two tracks disagree exactly as they do there.
    """
    _write_silent_video(path.with_suffix(".video.mp4"))
    _write_tone(path.with_suffix(".audio.mov"), seconds=1.0)
    done = _ffmpeg(
        "-itsoffset",
        str(delay_s),
        "-i",
        str(path.with_suffix(".video.mp4")),
        "-i",
        str(path.with_suffix(".audio.mov")),
        "-map",
        "0:v:0",
        "-map",
        "1:a:0",
        "-c",
        "copy",
        str(path),
    )
    if done.returncode != 0:
        pytest.skip(f"Could not build a late-video fixture: {done.stderr!r}")


def _decode_ignoring_the_edit_list(path: Path) -> int:
    """Sample count the naive way — from sample zero, edit list switched off."""
    done = _ffmpeg(
        "-ignore_editlist",
        "1",
        "-i",
        str(path),
        "-map",
        "0:a:0",
        "-vn",
        "-ac",
        "1",
        "-ar",
        str(_TARGET_RATE),
        "-f",
        "s16le",
        "-acodec",
        "pcm_s16le",
        "-",
    )
    assert done.returncode == 0, done.stderr
    return len(done.stdout) // 2


def test_decodes_to_mono_pcm_at_one_known_rate(tmp_path: Path) -> None:
    """Stereo 44.1 kHz in, mono 48 kHz out, so no consumer needs a per-clip case."""
    clip_path = tmp_path / "tone.mov"
    _write_tone(clip_path, seconds=1.0, rate=44_100)

    clip = FfmpegAudioSource(clip_path).read()

    assert clip.sample_rate == _TARGET_RATE
    assert clip.samples.typecode == "h"  # signed 16-bit, the port's contract
    # AAC pads both ends of what it encodes, so the decode is a few tens of ms longer than the
    # tone; the point of the assertion is the rate and the channel count, not the duration.
    assert clip.duration_s == pytest.approx(1.0, abs=0.1)
    assert max(abs(sample) for sample in clip.samples) > 1000  # a tone, not silence


def test_the_container_edit_list_is_applied(tmp_path: Path) -> None:
    """The one behaviour this adapter exists for (docs/M11_ACOUSTIC_SYNC.md §E2).

    AAC encoders emit priming samples and record them as an edit-list skip. Decoding from sample
    zero inherits that skip as a leading silence; letting the demuxer apply the edit list drops
    it. Measured on the real corpus 2026-08-29: 2112 samples, 44 ms, 2.64 frames at 60 fps —
    present on `2026-08-23/9`'s face-on track and *absent* on its down-the-line track, so it does
    not cancel between the two views. The synthetic file here carries a smaller offset of the
    same kind, which is why the assertion is a direction and a floor rather than a number.
    """
    clip_path = tmp_path / "tone.mov"
    _write_tone(clip_path, seconds=1.0)

    decoded = len(FfmpegAudioSource(clip_path).read().samples)
    naive = _decode_ignoring_the_edit_list(clip_path)

    trimmed = naive - decoded
    assert trimmed > 0, "the edit list was not applied — check for -ignore_editlist"
    assert trimmed >= 0.010 * _TARGET_RATE  # at least 10 ms, well under the corpus's 44


def test_the_stream_is_selected_explicitly(tmp_path: Path) -> None:
    """Face-on clips carry two audio streams (§E2), so "the audio track" is not well defined.

    Both of this fixture's streams decode, unlike the corpus's second `soun` track; what is
    pinned here is that the adapter takes the stream it was *asked* for and says which one that
    was, so a decode is reproducible from what it stored.
    """
    clip_path = tmp_path / "two_streams.mov"
    _write_tone(tmp_path / "a.mov", seconds=1.0, rate=44_100)
    _write_tone(tmp_path / "b.mov", seconds=0.4, rate=44_100)
    merged = _ffmpeg(
        "-i",
        str(tmp_path / "a.mov"),
        "-i",
        str(tmp_path / "b.mov"),
        "-map",
        "0:a:0",
        "-map",
        "1:a:0",
        "-c",
        "copy",
        str(clip_path),
    )
    if merged.returncode != 0:
        pytest.skip(f"Could not build a two-stream fixture: {merged.stderr!r}")

    first = FfmpegAudioSource(clip_path, stream_index=0).read()
    second = FfmpegAudioSource(clip_path, stream_index=1).read()

    assert first.stream_index == 0
    assert second.stream_index == 1
    assert second.duration_s < first.duration_s


def test_camera_id_rides_onto_the_clip(tmp_path: Path) -> None:
    """The ADR-011 seam, as `capture.Frame` carries it: set once per source, stamped on the read."""
    clip_path = tmp_path / "tone.mov"
    _write_tone(clip_path, seconds=0.3)

    assert FfmpegAudioSource(clip_path, camera_id="face_on").read().camera_id == "face_on"
    assert FfmpegAudioSource(clip_path).read().camera_id is None


def test_missing_file_raises(tmp_path: Path) -> None:
    with pytest.raises(FileNotFoundError):
        FfmpegAudioSource(tmp_path / "nope.MOV").read()


def test_a_clip_with_no_audio_track_is_told_apart_from_a_broken_decode(tmp_path: Path) -> None:
    """Different answers downstream: unanchorable bundle versus broken tool (ADR-010 §2)."""
    clip_path = tmp_path / "silent.mov"
    _write_silent_video(clip_path)

    with pytest.raises(NoAudioTrackError):
        FfmpegAudioSource(clip_path).read()


def test_an_absent_stream_index_raises_no_audio_track_error(tmp_path: Path) -> None:
    clip_path = tmp_path / "tone.mov"
    _write_tone(clip_path, seconds=0.3)

    with pytest.raises(NoAudioTrackError):
        FfmpegAudioSource(clip_path, stream_index=7).read()


def test_it_implements_the_audio_source_port(tmp_path: Path) -> None:
    assert isinstance(FfmpegAudioSource(tmp_path / "tone.mov"), AudioSource)


def test_a_video_that_starts_late_reports_the_offset(tmp_path: Path) -> None:
    """The whole of M11 P10 in one assertion: audio time zero is not frame zero. [M11 P10]

    On four of the 30 clips on disk the video track carries a leading empty edit and the audio
    track does not, so a sample index and a frame index describe clocks 105-125 ms apart — 6.3 to
    7.5 frames at 60 fps, and the error the eye check on 2026-08-30 found in the stored anchors.
    The fixture is synthetic and its delay is a round number; what is pinned is that the offset is
    *seen*, because the failure mode it guards is a silent zero.
    """
    clip_path = tmp_path / "late_video.mov"
    _write_video_starting_late(clip_path, delay_s=0.1)

    assert video_start_seconds(clip_path) == pytest.approx(0.1, abs=0.005)


def test_a_clip_whose_tracks_agree_reports_zero(tmp_path: Path) -> None:
    """26 of the 30 clips on disk, and the reason this is a measurement and not a correction.

    A measured 0.0 is what lets the caller treat a *missing* value as "nobody looked" rather than
    as "no offset" — the distinction `AudioClipMetadata.video_start_s` is documented around.
    """
    clip_path = tmp_path / "silent.mov"
    _write_silent_video(clip_path)

    assert video_start_seconds(clip_path) == 0.0


def test_a_clip_with_no_video_track_reports_none(tmp_path: Path) -> None:
    """None, not zero: there are no frames for a sample index to land on (ADR-010 §2)."""
    clip_path = tmp_path / "tone.mov"
    _write_tone(clip_path, seconds=0.3)

    assert video_start_seconds(clip_path) is None


def test_probing_a_missing_file_raises(tmp_path: Path) -> None:
    """Same answer `read()` gives, for the same reason: a missing clip is not an absent track."""
    with pytest.raises(FileNotFoundError):
        video_start_seconds(tmp_path / "nope.MOV")
