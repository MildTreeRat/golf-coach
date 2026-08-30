"""Acoustic contract shapes. Runs on a base install — no ffmpeg, no numpy (ADR-008).

The tests worth having here are the ones that pin a *decision*, not pydantic's own behaviour: that
a strike's frame is allowed to be unknown, that unknown is not zero, and that an empty strike list
survives as a result rather than being smoothed into a gap.
"""

from __future__ import annotations

import pytest
from pydantic import ValidationError

from golf_coach.contracts.audio import (
    AUDIO_DETECTOR_VERSION,
    AudioClipMetadata,
    AudioFile,
    AudioStrike,
)


def test_a_strike_needs_only_a_sample_confidence_and_prominence() -> None:
    """The detector sees a waveform and a rate; nothing else is available to it at that point."""
    strike = AudioStrike(sample=12_000, confidence=0.8, prominence=6100.0)

    assert strike.sample == 12_000
    assert strike.frame is None


def test_an_unknown_frame_is_none_and_not_frame_zero() -> None:
    """ADR-010 §2 at the contract level: an underivable frame index must not read as the start."""
    strike = AudioStrike(sample=1, confidence=0.1, prominence=0.0)

    assert strike.frame is None
    assert strike.frame != 0


def test_prominence_is_unbounded_because_it_is_the_detector_s_own_units() -> None:
    """§E5 measured onset flux in the hundreds to the thousands — a [0, 1] cap would clip it."""
    assert AudioStrike(sample=0, confidence=1.0, prominence=8900.0).prominence == 8900.0


def test_confidence_is_bounded_so_it_can_be_compared_across_clips() -> None:
    with pytest.raises(ValidationError):
        AudioStrike(sample=0, confidence=1.4, prominence=1.0)


def test_a_negative_sample_is_rejected_rather_than_stored() -> None:
    """Samples index into a decoded waveform; there is no before-the-start to point at."""
    with pytest.raises(ValidationError):
        AudioStrike(sample=-1, confidence=0.5, prominence=1.0)


def test_clip_metadata_is_entirely_optional() -> None:
    """Mirrors `keypoints.ClipMetadata`: these are facts about a decode, not requirements of one."""
    clip = AudioClipMetadata()

    assert (clip.sample_rate, clip.duration_s, clip.source_sha256) == (None, None, None)
    assert (clip.stream_index, clip.fps) == (None, None)


def test_stream_index_zero_is_a_real_answer_and_not_a_missing_one() -> None:
    """Face-on clips carry two `soun` tracks (§E2), so "which one" always has to be sayable."""
    assert AudioClipMetadata(stream_index=0).stream_index == 0


def test_an_empty_strike_list_is_a_valid_file() -> None:
    """A rehearsal makes no crack (M11 P5). "Detected nothing" is a result the file must hold."""
    assert AudioFile(strikes=[]).strikes == []


def test_a_file_written_before_the_detector_was_versioned_reads_as_older() -> None:
    """0, not the current version — the same load-bearing default `analysis_version` carries.

    A default of "current" would make every artifact written before the field existed claim to be
    up to date, which is the one wrong answer nothing downstream could recover from: it is
    indistinguishable from a correct one.
    """
    assert AudioFile(strikes=[]).detector_version == 0
    assert AUDIO_DETECTOR_VERSION > 0


def test_a_file_without_strikes_is_not_an_audio_file() -> None:
    """Empty and absent differ, so the absent case is an error rather than a silent empty list."""
    with pytest.raises(ValidationError):
        AudioFile()  # type: ignore[call-arg]
