"""AudioSource port tests. Deliberately runs on a base install — no extras, no skip.

That is the point of the file: the port is the half of `audio/` that `analysis` and the storage
readers are allowed to see, so if importing it ever needs ffmpeg or numpy, this suite is what
says so first (ADR-008).
"""

from __future__ import annotations

from array import array

from golf_coach.audio.source import AudioClip


def test_duration_comes_from_the_sample_count() -> None:
    """Not from the container, which reports the track *before* its edit list is applied."""
    clip = AudioClip(samples=array("h", [0] * 24_000), sample_rate=48_000)

    assert clip.duration_s == 0.5


def test_an_unusable_sample_rate_reports_zero_rather_than_dividing_by_it() -> None:
    """A rate of 0 means nothing was decoded; 0.0 s is the honest answer, not a crash."""
    assert AudioClip(samples=array("h", [1, 2, 3]), sample_rate=0).duration_s == 0.0


def test_camera_id_defaults_to_unlabelled() -> None:
    """Unlabelled is the honest default, as it is for `capture.Frame` (ADR-011)."""
    clip = AudioClip(samples=array("h"), sample_rate=48_000)

    assert clip.camera_id is None
    assert clip.stream_index == 0
