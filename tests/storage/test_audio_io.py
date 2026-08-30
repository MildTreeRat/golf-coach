"""`{role}.audio.json` I/O. Runs on a base install — the split this file exists to prove.

Decoding audio needs the `audio` extra; reading what it wrote must not. If loading a strike list
ever pulls in ffmpeg or numpy, this suite is what says so first (ADR-008).

The round-trip assertions are written against literal JSON where the on-disk shape is the thing
under test, for `tests/storage/test_keypoints_io.py`'s reason: a fixture built from the models
follows them wherever they drift, which is the regression these tests exist to catch.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from golf_coach.contracts.audio import AudioClipMetadata, AudioFile, AudioStrike
from golf_coach.storage.audio_io import load_audio, save_audio


def _strikes() -> list[AudioStrike]:
    """The §E5 shape: a ball onset, then the louder screen strike ~100 ms later at 48 kHz."""
    return [
        AudioStrike(sample=96_000, confidence=0.62, prominence=2900.0),
        AudioStrike(sample=100_800, confidence=0.91, prominence=8100.0),
    ]


# --- the round trip -------------------------------------------------------------------------


def test_round_trips_with_clip_metadata(tmp_path: Path) -> None:
    path = tmp_path / "face_on.audio.json"
    original = AudioFile(
        clip=AudioClipMetadata(
            sample_rate=48_000,
            duration_s=4.25,
            source_sha256="b" * 64,
            stream_index=0,
            fps=59.94,
        ),
        strikes=_strikes(),
    )

    save_audio(original, path)
    loaded = load_audio(path)

    assert loaded.clip is not None
    assert loaded.clip.sample_rate == 48_000
    assert loaded.clip.duration_s == pytest.approx(4.25)
    assert loaded.clip.source_sha256 == "b" * 64
    assert loaded.clip.stream_index == 0
    assert loaded.clip.fps == pytest.approx(59.94)
    assert [s.sample for s in loaded.strikes] == [96_000, 100_800]
    assert loaded.strikes[1].prominence == pytest.approx(8100.0)


def test_derived_frames_survive_alongside_the_fps_that_derived_them(tmp_path: Path) -> None:
    """A stored frame index that cannot be re-derived is a number nobody can check."""
    path = tmp_path / "face_on.audio.json"
    save_audio(
        AudioFile(
            clip=AudioClipMetadata(sample_rate=48_000, fps=60.0),
            strikes=[AudioStrike(sample=96_000, confidence=0.9, prominence=8100.0, frame=120)],
        ),
        path,
    )

    loaded = load_audio(path)

    assert loaded.clip is not None and loaded.clip.fps == pytest.approx(60.0)
    assert loaded.strikes[0].frame == 120


def test_an_empty_strike_list_round_trips_as_empty(tmp_path: Path) -> None:
    """A rehearsal makes no crack (M11 P5), and `exclude_none` must not turn that into nothing."""
    path = tmp_path / "face_on.audio.json"
    save_audio(AudioFile(clip=AudioClipMetadata(sample_rate=48_000), strikes=[]), path)

    assert json.loads(path.read_text(encoding="utf-8"))["strikes"] == []
    assert load_audio(path).strikes == []


def test_save_creates_missing_parent_directories(tmp_path: Path) -> None:
    path = tmp_path / "nested" / "deeper" / "face_on.audio.json"
    save_audio(AudioFile(strikes=_strikes()), path)

    assert len(load_audio(path).strikes) == 2


# --- unknowns stay out of the file ----------------------------------------------------------


def test_unknowns_are_omitted_rather_than_written_as_null(tmp_path: Path) -> None:
    """Undetermined frames are the common case before an fps is known — a dozen per clip."""
    path = tmp_path / "face_on.audio.json"
    save_audio(AudioFile(strikes=_strikes()), path)

    raw = json.loads(path.read_text(encoding="utf-8"))

    assert "clip" not in raw
    assert all("frame" not in strike for strike in raw["strikes"])
    # Omitted and null mean the same thing on the way back.
    assert all(strike.frame is None for strike in load_audio(path).strikes)


def test_partial_clip_metadata_survives(tmp_path: Path) -> None:
    """A sample rate alone is a legitimate thing to know — synthetic fixtures have no source."""
    path = tmp_path / "face_on.audio.json"
    save_audio(AudioFile(clip=AudioClipMetadata(sample_rate=48_000), strikes=[]), path)

    loaded = load_audio(path)

    assert loaded.clip is not None
    assert loaded.clip.sample_rate == 48_000
    assert loaded.clip.source_sha256 is None


# --- reading files this code did not write ---------------------------------------------------


def test_a_file_written_by_hand_loads(tmp_path: Path) -> None:
    """The enveloped shape as a literal, so the reader is pinned to the layout and not the model."""
    path = tmp_path / "down_the_line.audio.json"
    path.write_text(
        '{"clip":{"sample_rate":48000,"duration_s":4.0,"stream_index":0},'
        '"strikes":[{"sample":96000,"confidence":0.5,"prominence":400.0}]}',
        encoding="utf-8",
    )

    loaded = load_audio(path)

    assert loaded.clip is not None and loaded.clip.sample_rate == 48_000
    assert loaded.strikes[0].sample == 96_000
    assert loaded.strikes[0].frame is None


def test_a_bare_array_is_rejected_and_names_the_file(tmp_path: Path) -> None:
    """Audio was born enveloped. There is no legacy array shape, so accepting one invents it."""
    path = tmp_path / "face_on.audio.json"
    path.write_text("[]", encoding="utf-8")

    with pytest.raises(ValueError, match="face_on.audio.json"):
        load_audio(path)


def test_unusable_top_level_type_names_the_file(tmp_path: Path) -> None:
    path = tmp_path / "face_on.audio.json"
    path.write_text('"not audio"', encoding="utf-8")

    with pytest.raises(ValueError, match="face_on.audio.json"):
        load_audio(path)


def test_malformed_json_raises(tmp_path: Path) -> None:
    path = tmp_path / "face_on.audio.json"
    path.write_text("{not json", encoding="utf-8")

    with pytest.raises(ValueError):
        load_audio(path)


def test_reading_audio_does_not_need_the_audio_extra() -> None:
    """The whole point of the split: `audio` decodes, storage reads, and only one needs ffmpeg."""
    import subprocess
    import sys

    code = (
        "import sys; import golf_coach.storage.audio_io;"
        "assert 'imageio_ffmpeg' not in sys.modules, 'audio_io pulled in the audio extra';"
        "assert 'numpy' not in sys.modules, 'audio_io pulled in numpy'"
    )
    subprocess.run([sys.executable, "-c", code], check=True)
