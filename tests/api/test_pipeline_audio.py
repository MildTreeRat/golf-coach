"""`audio_for`: the cached per-view acoustic read, and the ways it declines. [M11 P4]

The decode is faked and the detector is real. That split is deliberate: `FfmpegAudioSource` is
already pinned against actual AAC-in-MOV files by `tests/audio/test_ffmpeg_source.py`, and what is
untested until here is everything *around* it — the cache key, the frames derived from a sample
index, and the five separate ways this function can come back with nothing. Handing the real
`detect_strikes` a synthetic waveform keeps the one number these tests assert about time
(`AudioStrike.sample`) produced by the code that will produce it in the field, rather than by a
fixture that agrees with the test by construction.

The waveform is the shape `tests/audio/test_impact.py` states at length: room tone with a
broadband crack in it. Only one transient, because nothing here is about telling the ball from the
screen — that is P3's question and it is answered there.
"""

from __future__ import annotations

import sys
from array import array
from datetime import UTC, datetime
from pathlib import Path

import pytest

np = pytest.importorskip("numpy")
pytest.importorskip("imageio_ffmpeg")

from golf_coach.api.pipeline import audio_for  # noqa: E402  (after importorskip)
from golf_coach.audio import ffmpeg as ffmpeg_module  # noqa: E402
from golf_coach.audio.source import AudioClip, NoAudioTrackError  # noqa: E402
from golf_coach.contracts.audio import (  # noqa: E402
    AUDIO_DETECTOR_VERSION,
    AudioClipMetadata,
    AudioFile,
    AudioStrike,
)
from golf_coach.storage.audio_io import load_audio, save_audio  # noqa: E402
from golf_coach.storage.manifest import (  # noqa: E402
    Role,
    RoleFile,
    SwingManifest,
    manifest_path,
    save_manifest,
)

_WHEN = datetime(2026, 8, 29, 1, 0, tzinfo=UTC)
_RATE = 48_000
_FPS = 60.0
_BALL_AT_S = 1.5
_SHA = {Role.FACE_ON: "sha-face-on", Role.DOWN_THE_LINE: "sha-down-the-line"}


def _waveform(*, with_strike: bool = True, seconds: float = 3.0) -> array[int]:
    """One clip's PCM: room tone, and a decaying broadband crack unless asked for a rehearsal.

    Quantised into an `array("h")` because that is the shape the port hands over
    (`audio/source.py`); passing numpy straight through would test a call this pipeline never
    makes.
    """
    rng = np.random.default_rng(11)
    track = rng.normal(0.0, 200.0, int(seconds * _RATE))
    if with_strike:
        start = int(_BALL_AT_S * _RATE)
        length = int(0.25 * _RATE)
        decay = np.exp(-np.arange(length) / _RATE / 0.025)
        track[start : start + length] += 9_000 * rng.normal(0.0, 1.0, length) * decay
    return array("h", np.clip(track, -32_768, 32_767).astype(np.int16).tolist())


class _FakeSource:
    """Stands in for `FfmpegAudioSource`, and records every path it was asked to open."""

    opened: list[Path] = []
    samples: array[int] = array("h")
    error: Exception | None = None
    stream_index: int = 0

    def __init__(self, path, *, camera_id=None, stream_index: int = 0) -> None:
        self._path = Path(path)
        self._camera_id = camera_id

    def read(self) -> AudioClip:
        _FakeSource.opened.append(self._path)
        if _FakeSource.error is not None:
            raise _FakeSource.error
        return AudioClip(
            samples=_FakeSource.samples,
            sample_rate=_RATE,
            camera_id=self._camera_id,
            stream_index=_FakeSource.stream_index,
        )


@pytest.fixture
def decoder(monkeypatch):
    """Install `_FakeSource` in place of the real one and hand back its record of calls.

    Patched on `golf_coach.audio.ffmpeg` rather than on `pipeline`, because `audio_for` imports the
    adapter lazily *inside* the call — which is the property `test_pipeline_imports.py` pins, so a
    test reaching for `pipeline.FfmpegAudioSource` would assert against a name that must never
    exist there.
    """
    _FakeSource.opened = []
    _FakeSource.samples = _waveform()
    _FakeSource.error = None
    _FakeSource.stream_index = 0
    monkeypatch.setattr(ffmpeg_module, "FfmpegAudioSource", _FakeSource)
    return _FakeSource


@pytest.fixture
def bundle(tmp_path):
    """A swing directory with a manifest and both clips present on disk as empty files.

    Empty is enough: every decode in this file is the fake one, and all the real code does with
    the path is `exists()`.
    """
    swing_dir = tmp_path / "2026-08-29" / "1"
    swing_dir.mkdir(parents=True)
    roles = {}
    for role in (Role.FACE_ON, Role.DOWN_THE_LINE):
        (swing_dir / f"{role.value}.abc.mov").write_bytes(b"")
        roles[role] = RoleFile(
            role=role,
            filename=f"{role.value}.abc.mov",
            content_sha256=_SHA[role],
            original_filename=f"{role.value}.mov",
            content_type="video/quicktime",
            size_bytes=0,
            received_at=_WHEN,
        )
    manifest = SwingManifest(
        swing_id="1",
        session_id="2026-08-29",
        created_at=_WHEN,
        updated_at=_WHEN,
        roles=roles,
    )
    save_manifest(manifest, manifest_path(swing_dir))
    return swing_dir, manifest


def test_a_clip_is_decoded_detected_and_written_to_its_own_artifact(bundle, decoder) -> None:
    swing_dir, manifest = bundle

    audio = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)

    assert audio is not None
    assert audio.strikes, "the synthetic clip has a crack in it and the detector should hear it"
    on_disk = load_audio(swing_dir / "face_on.audio.json")
    assert on_disk.strikes == audio.strikes
    assert on_disk.clip is not None
    assert on_disk.clip.source_sha256 == _SHA[Role.FACE_ON]
    assert on_disk.clip.sample_rate == _RATE
    assert on_disk.clip.stream_index == 0
    # The decoded length, not the container's claim about it (M11 §E2).
    assert on_disk.clip.duration_s == pytest.approx(3.0, abs=0.01)


def test_the_frame_is_derived_from_the_sample_under_the_fps_it_was_given(bundle, decoder) -> None:
    """The whole point of the `fps` parameter, and the only arithmetic this module owns.

    Two assertions, and they are not the same one: the first pins the derivation exactly, the
    second pins that it was applied to a *real* strike rather than to an arbitrary index. The
    tolerance on the second is three frames because `AudioStrike.sample` is the start of the
    analysis window that rose, which M11 P3 measured at +18 ms — about a frame — against the
    waveform. That bias belongs to the detector, not to this conversion.
    """
    swing_dir, manifest = bundle

    audio = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)

    assert audio is not None and audio.clip is not None
    assert audio.clip.fps == _FPS
    for strike in audio.strikes:
        assert strike.frame == int(strike.sample * _FPS / _RATE)
    loudest = audio.strikes[0]
    assert loudest.frame == pytest.approx(_BALL_AT_S * _FPS, abs=3)


def test_a_caller_that_does_not_know_the_fps_leaves_every_frame_unknown(bundle, decoder) -> None:
    """None is the honest answer, and a strike with no frame is not a strike at frame 0."""
    swing_dir, manifest = bundle

    audio = audio_for(swing_dir, manifest, Role.FACE_ON, fps=None)

    assert audio is not None and audio.clip is not None
    assert audio.clip.fps is None
    assert audio.strikes and all(strike.frame is None for strike in audio.strikes)


def test_the_second_read_comes_from_cache_without_decoding_again(bundle, decoder) -> None:
    swing_dir, manifest = bundle

    first = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)
    second = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)

    assert second is not None and first is not None
    assert second.strikes == first.strikes
    assert len(decoder.opened) == 1, "the cached artifact should have answered the second call"


def test_frames_are_derived_from_the_cache_rather_than_by_listening_again(bundle, decoder) -> None:
    """The sample index is the measurement; the frame is arithmetic over it.

    So a first caller with no fps and a second caller with one must cost exactly one decode.
    Re-running an FFT per 5 ms hop to learn a number that follows from what is already on disk
    would be the expensive kind of wrong.
    """
    swing_dir, manifest = bundle

    blind = audio_for(swing_dir, manifest, Role.FACE_ON, fps=None)
    framed = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)

    assert blind is not None and framed is not None
    assert len(decoder.opened) == 1
    assert [strike.sample for strike in framed.strikes] == [s.sample for s in blind.strikes]
    assert all(strike.frame is not None for strike in framed.strikes)
    # Written back, so the next reader gets the frames and the fps they were derived under.
    on_disk = load_audio(swing_dir / "face_on.audio.json")
    assert on_disk.clip is not None and on_disk.clip.fps == _FPS
    assert on_disk.strikes == framed.strikes


def test_a_caller_without_the_fps_does_not_erase_frames_already_derived(bundle, decoder) -> None:
    """ADR-010 §2 in the small: not knowing something is not grounds for deleting it."""
    swing_dir, manifest = bundle

    framed = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)
    blind = audio_for(swing_dir, manifest, Role.FACE_ON, fps=None)

    assert framed is not None and blind is not None
    assert blind.strikes == framed.strikes
    assert blind.clip is not None and blind.clip.fps == _FPS


def test_a_re_uploaded_clip_invalidates_the_cache(bundle, decoder) -> None:
    """Same key as pose, for the same reason: a new clip under an old role invalidates itself."""
    swing_dir, manifest = bundle
    audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)

    manifest.roles[Role.FACE_ON].content_sha256 = "sha-a-different-clip-entirely"
    again = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)

    assert again is not None and again.clip is not None
    assert again.clip.source_sha256 == "sha-a-different-clip-entirely"
    assert len(decoder.opened) == 2


def test_a_detection_records_which_detector_made_it(bundle, decoder) -> None:
    """Stamped on write, so a later reader can tell a current list from a superseded one."""
    swing_dir, manifest = bundle

    audio = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)

    assert audio is not None and audio.detector_version == AUDIO_DETECTOR_VERSION
    assert load_audio(swing_dir / "face_on.audio.json").detector_version == AUDIO_DETECTOR_VERSION


def test_a_cache_from_an_older_detector_is_listened_to_again(bundle, decoder) -> None:
    """The clip's sha256 cannot see this: the clip is unchanged and the *detector* moved.

    Without it a fix to `audio/impact.py` would be invisible on every bundle already on disk —
    stored strikes the current detector would disagree with, read back as if they were its own.
    That is the failure `analysis_version` exists to prevent, one layer down (`contracts/audio.py`).
    """
    swing_dir, manifest = bundle
    audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)
    cache = swing_dir / "face_on.audio.json"
    save_audio(load_audio(cache).model_copy(update={"detector_version": 0}), cache)

    again = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)

    assert len(decoder.opened) == 2
    assert again is not None and again.detector_version == AUDIO_DETECTOR_VERSION
    assert load_audio(cache).detector_version == AUDIO_DETECTOR_VERSION


def test_an_install_that_cannot_listen_again_keeps_the_older_detection(bundle, monkeypatch) -> None:
    """A superseded anchor beats no anchor, and says so.

    A base install has no way to re-detect, and the thing it would fall back to is worse than what
    the cache holds: the pose impact this replaces ran 5-7 frames early on four bundles, where the
    stored list is a measurement where the pose impact it would fall back to is an estimate. Kept,
    and noted — never quietly, because two installs reading the same bundle have to be able to see
    why they disagree.
    """
    swing_dir, manifest = bundle
    stored = AudioFile(
        clip=AudioClipMetadata(
            sample_rate=_RATE, duration_s=3.0, source_sha256=_SHA[Role.FACE_ON], stream_index=0
        ),
        strikes=[AudioStrike(sample=72_000, confidence=0.9, prominence=1_000.0)],
        detector_version=0,
    )
    save_audio(stored, swing_dir / "face_on.audio.json")
    monkeypatch.setitem(sys.modules, "golf_coach.audio.ffmpeg", None)
    notes: list[str] = []

    audio = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS, notes=notes)

    assert audio is not None
    assert [strike.frame for strike in audio.strikes] == [90]
    assert notes == [
        "the face_on strikes were found by an older detector (v0) and the audio extra is not "
        "installed to re-run it"
    ]


def test_force_listens_again_even_with_a_matching_cache(bundle, decoder) -> None:
    swing_dir, manifest = bundle
    audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)

    audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS, force=True)

    assert len(decoder.opened) == 2


def test_a_rehearsal_swing_writes_an_empty_strike_list(bundle, decoder) -> None:
    """An empty list is a *result*, and the one M11 P5 reads. Never listened to is no file."""
    swing_dir, manifest = bundle
    decoder.samples = _waveform(with_strike=False)

    audio = audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)

    assert audio is not None
    assert audio.strikes == []
    assert (swing_dir / "face_on.audio.json").exists()


def test_a_role_that_never_arrived_is_none_and_not_a_note(bundle, decoder) -> None:
    """No shot-screen photo is not a fault in the audio path — there is simply no clip to open."""
    swing_dir, manifest = bundle
    notes: list[str] = []

    assert audio_for(swing_dir, manifest, Role.SHOT_SCREEN, notes=notes) is None
    assert notes == []
    assert decoder.opened == []


def test_a_clip_missing_from_disk_is_a_note(bundle, decoder) -> None:
    swing_dir, manifest = bundle
    (swing_dir / "face_on.abc.mov").unlink()
    notes: list[str] = []

    assert audio_for(swing_dir, manifest, Role.FACE_ON, notes=notes) is None
    assert notes == ["the face_on clip is recorded in the manifest but missing from disk"]


def test_a_clip_with_no_audio_track_is_a_note_and_not_an_exception(bundle, decoder) -> None:
    """Measured 0/30 in this corpus (M11 §E1), which is exactly why the path needs a test.

    It is also the one decline whose note says something no re-run can fix: the bundle cannot be
    anchored on the ball strike at all.
    """
    swing_dir, manifest = bundle
    decoder.error = NoAudioTrackError("face_on.abc.mov has no audio stream at index 0")
    notes: list[str] = []

    assert audio_for(swing_dir, manifest, Role.FACE_ON, notes=notes) is None
    assert notes == [
        "the face_on clip has no audio track, so it cannot be anchored on the ball strike"
    ]
    assert not (swing_dir / "face_on.audio.json").exists()


def test_a_failed_decode_is_a_note_and_not_an_exception(bundle, decoder) -> None:
    """`analyze_swing_dir` never raises for an expected failure, and a broken clip is one."""
    swing_dir, manifest = bundle
    decoder.error = OSError("ffmpeg could not decode audio: moov atom not found")
    notes: list[str] = []

    assert audio_for(swing_dir, manifest, Role.FACE_ON, notes=notes) is None
    assert notes == [
        "the face_on clip's audio could not be decoded, so no strike was detected in it"
    ]


def test_a_missing_audio_extra_degrades_with_a_note(bundle, monkeypatch) -> None:
    """The `vision` extra's rule applied to `audio`: an absent dependency is a note, not a crash.

    None in `sys.modules` is the documented way to make an `import` statement raise ImportError for
    a package that is in fact installed — which it is here, since every other test in this file
    needs it. Faking the absence is the only way to reach this branch in an environment that has
    the extra, and it is the branch a base install takes on every call.
    """
    swing_dir, manifest = bundle
    monkeypatch.setitem(sys.modules, "golf_coach.audio.ffmpeg", None)
    notes: list[str] = []

    assert audio_for(swing_dir, manifest, Role.FACE_ON, notes=notes) is None
    assert notes == ["no audio for face_on: the audio extra is not installed"]


def test_each_view_gets_its_own_artifact(bundle, decoder) -> None:
    """Two clips, two files, keyed on their own hashes — nothing here is bundle-wide."""
    swing_dir, manifest = bundle

    audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS)
    audio_for(swing_dir, manifest, Role.DOWN_THE_LINE, fps=_FPS)

    for role in (Role.FACE_ON, Role.DOWN_THE_LINE):
        stored = load_audio(swing_dir / f"{role.value}.audio.json")
        assert stored.clip is not None and stored.clip.source_sha256 == _SHA[role]


def test_a_clean_read_narrates_to_the_log_and_leaves_the_notes_empty(bundle, decoder) -> None:
    """`log` is for progress and `notes` are for truth — a clean read produces only the first."""
    swing_dir, manifest = bundle
    lines: list[str] = []
    notes: list[str] = []

    audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS, log=lines.append, notes=notes)
    audio_for(swing_dir, manifest, Role.FACE_ON, fps=_FPS, log=lines.append, notes=notes)

    assert notes == []
    assert any("decoding audio" in line for line in lines)
    assert any("from cache" in line for line in lines)
