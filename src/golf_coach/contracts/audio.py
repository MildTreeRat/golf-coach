"""Acoustic contracts (produced by the `audio` module, consumed by `analysis` and `api`). [M11 P2]

The seam audio crosses into the rest of the system, and it is deliberately narrow: an integer
**sample index**, a confidence, and a prominence. No waveform, no envelope, no spectrogram. What
`analysis` gets to know about a ball strike is *when* it happened and *how sure* the detector is —
the same way `contracts/keypoints.py` hands over landmarks without MediaPipe ever appearing
downstream, and the same way ADR-022 lets fitted models cross as JSON.

**Samples, not frames, are the measurement.** The detector (audio/impact.py) is handed a waveform
and a sample rate; it has never seen the video and cannot know its fps. A frame index is therefore
a *derived* convenience filled in later by the caller that does know — see `AudioStrike.frame` and
`AudioClipMetadata.fps`, which is recorded here precisely so a stored frame index stays auditable.

Pydantic + stdlib only, like every other contract (ADR-008). Nothing in here imports `audio`; the
dependency runs the other way, which is what lets the stored `{role}.audio.json` be read on a base
install with no ffmpeg and no numpy.
"""

from __future__ import annotations

from pydantic import BaseModel, Field

#: Which generation of `audio/impact.py` a stored `{role}.audio.json` was detected by. Lives here
#: rather than in `audio/` for `ANALYSIS_VERSION`'s reason (`contracts/swing.py`): the reader that
#: has to decide whether a cached artifact is still current is `api.pipeline.audio_for`, and it must
#: be able to decide that on a base install where `audio/impact.py` — and the numpy it imports —
#: cannot be imported at all.
#:
#: Bump it when the detector would return a *different list* for the same waveform. A cache is not
#: keyed on anything else that could notice: `AudioClipMetadata.source_sha256` catches a re-uploaded
#: clip and nothing catches a changed detector, so without this a fix to detection stays invisible
#: on every bundle already on disk — the same silent-disagreement failure `analysis_version`
#: exists to prevent, one layer down.
#:
#: 0 -> 1 (2026-08-30): the field is introduced, and detection is unchanged by it. Everything
#:                   already on disk reads 0 and is listened to once more, which stamps the store
#:                   without moving a single sample index. It exists now rather than alongside the
#:                   first real detector change because that change is coming: the same day this
#:                   was added, a candidate floor was built, measured against the video frame by
#:                   frame, and reverted (docs/M11_ACOUSTIC_SYNC.md §Addendum) — and it would have
#:                   shipped invisibly on every stored bundle without this.
#: 1 -> 2 (2026-08-30): that floor, landed [M11 P11] — `detect_strikes` now drops every candidate
#:                   under a quarter of the clip's loudest, which is a different list for the same
#:                   waveform and the change this counter was added for. It carries P10 with it:
#:                   the *frames* on a stored file were derived without `video_start_s`, so every
#:                   artifact written before this is re-detected against a clip that is now
#:                   re-probed for its video timebase too.
AUDIO_DETECTOR_VERSION = 2


class AudioStrike(BaseModel):
    """One transient the detector found, ranked among its siblings by `prominence`.

    A strike is a *candidate*, not a verdict. The bay produces four transients per shot — club-ball
    contact, the club hitting the mat, the ball hitting the impact screen 85-145 ms later, and the
    simulator's own ball-flight audio — and the screen strike is the louder of the first two pair on
    every clip measured (docs/M11_ACOUSTIC_SYNC.md §E5). So `detect_strikes` returns all of them and
    lets the caller see how many there were, the same choice `analysis.phases.candidate_downswings`
    made for the same reason: discovering by eye that the wrong peak was taken is the failure this
    shape exists to prevent.
    """

    sample: int = Field(
        ge=0,
        description=(
            "Offset into the decoded mono waveform, in samples, after the container's edit list "
            "has been applied. This is the measurement; everything else here is derived from it."
        ),
    )
    confidence: float = Field(
        ge=0.0,
        le=1.0,
        description="How sure the detector is this is a real onset, normalized to [0, 1].",
    )
    prominence: float = Field(
        ge=0.0,
        description=(
            "How far this onset stands above its local background, in the detector's own units "
            "(unbounded, and not comparable across detectors). Ranks candidates within one clip; "
            "`confidence` is the number to compare across clips."
        ),
    )
    frame: int | None = Field(
        default=None,
        ge=0,
        description=(
            "The video frame this strike lands on, or None when it could not be worked out. "
            "Derived, not measured: the detector sees a waveform and a sample rate and never the "
            "video, so this is filled in by the caller that holds the clip's fps *and* its "
            "`video_start_s`. None means unknown — a strike whose frame could not be worked out "
            "is not frame 0 (ADR-010 §2) — and it has two causes: nobody knew the fps, or the "
            "strike sounded before the first frame the decoder hands back, which is a real "
            "possibility on a clip whose video starts late (see `video_start_s`) and is honestly "
            "no frame at all rather than frame 0."
        ),
    )


class AudioClipMetadata(BaseModel):
    """What audio was decoded, recorded alongside the strikes found in it.

    Every field is optional, mirroring `keypoints.ClipMetadata`: these are facts about a decode,
    and a strike list can legitimately exist without them (a synthetic fixture, a clip long gone).
    None means unknown, never zero.
    """

    sample_rate: int | None = Field(
        default=None, gt=0, description="Samples per second of the waveform the strikes index into."
    )
    duration_s: float | None = Field(
        default=None,
        ge=0.0,
        description=(
            "Length of the waveform actually decoded, which is the honest number: it includes the "
            "edit list's effect, where the container's own duration field does not (§E2)."
        ),
    )
    source_sha256: str | None = Field(
        default=None,
        description=(
            "Content hash of the source video, tying these strikes back to their clip. Same cache "
            "key `keypoints.ClipMetadata` uses, so a re-uploaded clip invalidates both together."
        ),
    )
    stream_index: int | None = Field(
        default=None,
        ge=0,
        description=(
            "Which audio stream was decoded. Not decoration: face-on clips here carry two `soun` "
            "tracks with different edit offsets (§E2), so a sample index only means something "
            "alongside the stream it counts into."
        ),
    )
    fps: float | None = Field(
        default=None,
        gt=0.0,
        description=(
            "The frame rate every `AudioStrike.frame` on this file was derived under, or None when "
            "no frame was derivable. Recorded for `stream_index`'s reason rather than to duplicate "
            "`keypoints.ClipMetadata.fps`: a stored frame index that cannot be re-derived is a "
            "number nobody can check."
        ),
    )
    video_start_s: float | None = Field(
        default=None,
        ge=0.0,
        description=(
            "Presentation time of the first frame the video decoder hands back, subtracted from "
            "every sample time before it becomes a `frame`. Almost always 0.0; it is not on four "
            "down-the-line clips in this corpus, whose containers carry a leading empty edit that "
            "the audio decode honours and the frame counter does not (`audio/ffmpeg.py`'s "
            "`video_start_seconds`, measured at 0.105-0.125 s — 6.3 to 7.5 frames at 60 fps). "
            "Recorded rather than re-measured because it is the difference between two clocks and "
            "a stored frame index is unreadable without it. None means nobody measured it, which "
            "is not the same as a measured zero."
        ),
    )


class AudioFile(BaseModel):
    """The on-disk shape of a `{role}.audio.json`: the strikes plus what decode produced them.

    Read and written through `storage.audio_io`, not by parsing this model directly, so there is
    one place that knows the file layout.

    **An empty `strikes` list is a result, not a gap.** The file exists only once detection has run,
    so empty means the detector ran and found no transient — which is exactly what a rehearsal swing
    sounds like, and is the signal M11 P5 reads. "Never analysed" is the absence of the file.
    """

    clip: AudioClipMetadata | None = None
    strikes: list[AudioStrike]

    detector_version: int = Field(
        default=0,
        description=(
            "Which generation of the detector found these (`AUDIO_DETECTOR_VERSION` above). "
            "**Defaults to 0, not to the current version**, for the reason "
            "`SwingBundleResult.analysis_version` does: a file written before this field existed "
            "must read as older-than-current, because a default of 'current' would make every "
            "legacy artifact claim to be up to date and there is no way back from that. "
            "`api.pipeline.audio_for` re-detects anything that does not match."
        ),
    )
