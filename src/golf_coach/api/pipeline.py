"""One assembled swing bundle in, the complete result out. [M7 Phase 5]

This is the orchestration ADR-008 always meant to live in `api/` — "the imperative shell", the
one place allowed to do I/O around the pure analysis core. It ran inside
`scripts/analyze_bundle.py` until the background worker needed it too, and two copies of a
pipeline is how they drift.

Pose per view, listen for the ball strike, OCR the shot screen, score, rank tips, align the two
views; four artifacts land in the swing directory:

    analysis.json           the complete result, for a results page to render
    analysis.state.json     the sidecar summary of this run — written *here* so it can never
                            quote a score the analysis beside it has stopped agreeing with
    aligned.mp4             the two views side by side, banners landing together
    <role>.keypoints.json   pose output per view, cached so a re-run is free
    <role>.audio.json       the transients heard in that view's clip, cached the same way (M11)

**Everything expensive is cached and content-addressed.** Pose on a 4K60 clip is minutes, so
keypoints are reused whenever their recorded `source_sha256` still matches the manifest's, and
the shot is looked up in the shot store by the photo's hash before any OCR is considered — which
means an already-imported shot attaches with no `ocr` extra installed at all.

Degradation is graceful and always narrated: a bundle with no down-the-line clip still scores and
still writes JSON, it just has no alignment and no video. Only a missing face-on clip is fatal —
that is the view every checkpoint is measured from.

Two rules this module exists to keep:

**It must not import fastapi — nor the decoders behind the other extras.**
`scripts/analyze_bundle.py` imports it and runs on a `vision`-only install; dragging the web
framework in here would break the extras boundary ADR-008 draws, and so would a module-scope
reach for `imageio_ffmpeg` on `audio_for`'s behalf. `tests/api/test_pipeline_imports.py` pins
both.

**Nothing is narrated only to a terminal.** The CLI passes `log=print`; the worker passes a
logger, and nobody is watching. So anything a reader of the *result* would need to know — a clip
that decoded short, a shot that could not be read, a swing the selector declined to pick — is
appended to `result.notes`, which is part of `analysis.json`. `log` is for progress; `notes` are
for truth.

Needs the `vision` extra to run pose or render, `ocr` only to read a shot photo that is not
already in the store, and `audio` only to hear a clip that has not been listened to yet.
"""

from __future__ import annotations

import math
from collections.abc import Callable
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

from golf_coach.analysis.alignment import (
    DEFAULT_TAU_RANGE,
    pair_frames,
    warp_speeds,
)
from golf_coach.analysis.benchmarks.distributions import dataset_info
from golf_coach.analysis.engine import analyze_swing_bundle
from golf_coach.analysis.phases import (
    LEAD_WRIST,
    TRAIL_WRIST,
    SwingChoice,
    select_matching_swing,
    select_swing,
)
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.api.state import AnalysisState, input_hashes, load_state, now, save_state
from golf_coach.config import settings
from golf_coach.contracts.audio import (
    AUDIO_DETECTOR_VERSION,
    AudioClipMetadata,
    AudioFile,
    AudioStrike,
)
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.intent import ClubCategory, PracticeGoal
from golf_coach.contracts.keypoints import ClipMetadata, KeypointsFile, PoseLandmark
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import SwingBundleResult
from golf_coach.contracts.unscored import UnscoredReason
from golf_coach.feedback.coach import generate_coaching
from golf_coach.feedback.rules import build_feedback
from golf_coach.launch_monitor.screen.store import ShotStore
from golf_coach.pose.estimator import pose_estimator_name
from golf_coach.storage.audio_io import load_audio, save_audio
from golf_coach.storage.flight_inputs import LoftGap, loft_for_club
from golf_coach.storage.golfer_store import GolferStore
from golf_coach.storage.keypoints_io import load_keypoints, save_keypoints
from golf_coach.storage.manifest import Role, SwingManifest, load_manifest, manifest_path

ANALYSIS_NAME = "analysis.json"
ALIGNED_NAME = "aligned.mp4"

VIEWS: tuple[tuple[Role, str], ...] = (
    (Role.FACE_ON, "face-on"),
    (Role.DOWN_THE_LINE, "down-the-line"),
)

Log = Callable[[str], None]


def _noop(_: str) -> None:
    """Default `log`: run silently. The worker's default state, not the CLI's."""


@dataclass(frozen=True)
class PipelineOptions:
    """Everything about a run that a caller can vary — the CLI's flags, essentially."""

    shots_dir: Path = field(default_factory=lambda: settings.shots_dir)
    club: ClubCategory = ClubCategory.ALL
    min_confidence: float = field(default_factory=lambda: settings.ocr_min_confidence)
    window_face_on: tuple[int, int] | None = None
    window_down_the_line: tuple[int, int] | None = None
    auto_window: bool = True
    render_video: bool = True
    force_pose: bool = False
    #: Re-detect the ball strikes even when `{role}.audio.json` matches the clip on disk. There is
    #: no `skip_audio` beside it: audio is only ever *evidence* for a rule that already worked
    #: without it, so a run with no audio available is the ordinary path and not a degraded one.
    force_audio: bool = False
    force_ocr: bool = False
    skip_ocr: bool = False
    tau_range: tuple[float, float] = DEFAULT_TAU_RANGE
    #: Ask Claude for the written coaching paragraph (M6). Defaults to the setting rather than to
    #: a literal, so `--no-coaching` and `GOLF_COACHING_ENABLED=0` are the same switch. With no
    #: API key configured the call is skipped either way, and the skip is recorded in `notes`.
    coaching: bool = field(default_factory=lambda: settings.coaching_enabled)


@dataclass
class PipelineOutcome:
    """What a run produced. `result is None` means nothing could be scored at all."""

    result: SwingBundleResult | None = None
    analysis_path: Path | None = None
    video_path: Path | None = None
    video_codec: str | None = None
    #: Whether this run *tried* to render. `video_path is None` alone cannot say why there is no
    #: video, and the two reasons want opposite handling in `record_state`: a run that skipped the
    #: render (`--no-video`, or `reanalyze.py` without `--video`) leaves any existing `aligned.mp4`
    #: standing, while a run that tried and failed has just contradicted the one on disk.
    render_attempted: bool = False
    missing_roles: list[Role] = field(default_factory=list)
    #: Something is worth a human's attention — a role never arrived, or the shot needs review.
    #: The CLI turns this into exit code 1.
    flagged: bool = False
    #: Set only when `result is None`; the CLI turns it into exit code 2.
    error: str | None = None


def resolve_swing_dir(target: str, sessions_dir: Path) -> Path | None:
    """Accept either `SESSION/SWING` inside the store or a path to the swing directory."""
    direct = Path(target)
    if direct.is_dir() and manifest_path(direct).exists():
        return direct
    parts = target.replace("\\", "/").split("/")
    if len(parts) == 2:
        candidate = sessions_dir / parts[0] / parts[1]
        if manifest_path(candidate).exists():
            return candidate
    return None


def _older_pose_note(
    role: Role, superseded: KeypointsFile, estimator: str, because: str
) -> str:
    """Why a result carries landmarks the configured estimator did not produce.

    Both callers reach it the same way — the cache was measured by another variant and this run
    could not redo it — and both have to say so rather than substitute quietly, because a reader
    comparing this result against one measured by the configured estimator has to be able to see
    where the difference came from. `analysis_version` makes the same admission a layer up.
    """
    return (
        f"the {role.value} landmarks were measured by "
        f"{superseded.pose_estimator or 'an unrecorded estimator'} rather than by the configured "
        f"{estimator}, and pose could not be re-run because {because}"
    )


def keypoints_for(
    swing_dir: Path,
    manifest: SwingManifest,
    role: Role,
    *,
    force: bool = False,
    log: Log = _noop,
    notes: list[str] | None = None,
) -> KeypointsFile | None:
    """Pose output for one view, from cache when the clip behind it has not changed.

    The cache key is the clip's own sha256, recorded in the keypoints envelope by M7 Phase 1 and
    in the manifest by the upload route. Comparing them is what makes reuse safe rather than
    merely fast: a re-uploaded clip under the same role invalidates itself.

    **The cache is keyed on the estimator as well as on the clip**, exactly as `audio_for`'s is
    keyed on its detector. `settings.pose_model_variant` decides which MediaPipe bundle runs, and
    two bundles do not agree landmark for landmark — they move the phase instants and therefore
    every score. The sha256 cannot see that: it keys on the *footage*, and the footage has not
    changed. A file stamped with another estimator (or, for anything written before the stamp
    existed, with none) is re-run rather than read. An install without the `vision` extra cannot
    re-run and keeps the older landmarks, saying so in a note.
    """
    role_file = manifest.roles.get(role)
    if role_file is None:
        return None

    estimator = pose_estimator_name()
    cache_path = swing_dir / f"{role.value}.keypoints.json"
    # Same clip, another estimator: worth re-running, and worth keeping if this install cannot.
    superseded: KeypointsFile | None = None
    if cache_path.exists() and not force:
        cached = load_keypoints(cache_path)
        if cached.clip is None or cached.clip.source_sha256 != role_file.content_sha256:
            log(f"  {role.value}: cached keypoints are for a different clip, re-running pose")
        elif cached.pose_estimator != estimator:
            superseded = cached
            measured_by = cached.pose_estimator or "an unrecorded estimator"
            log(f"  {role.value}: cached pose is from {measured_by}, re-running with {estimator}")
        else:
            log(f"  {role.value}: {len(cached.frames)} frames from cache")
            return cached

    video_path = swing_dir / role_file.filename
    if not video_path.exists():
        # A cached run whose clip has since been deleted is still a measurement, and losing it
        # would cost the whole bundle: face-on is the view every checkpoint is measured from, so
        # `analyze_swing_dir` gives up entirely when this returns None for it. Before the
        # estimator became part of the cache key this branch was unreachable for a bundle with
        # cached pose — the sha256 check above returned early. Now a variant switch sends every
        # archived bundle down it.
        if superseded is not None:
            log(f"  {role.value}: {video_path.name} is gone, keeping the older pose run")
            _note(notes, _older_pose_note(role, superseded, estimator, "the clip is not on disk"))
            return superseded
        log(f"  {role.value}: {video_path.name} is missing from the swing directory")
        _note(notes, f"the {role.value} clip is recorded in the manifest but missing from disk")
        return None

    try:
        from golf_coach.capture.file import FileVideoSource
        from golf_coach.pose.estimator import estimate_pose
    except ImportError:
        # Older landmarks beat none, the same trade `audio_for` makes below: the swing is still
        # scoreable, just by an instrument this install cannot reproduce. Said in a note rather
        # than used quietly, because a reader comparing this result against one measured by the
        # configured estimator has to be able to see why the two differ.
        if superseded is not None:
            log(f"  {role.value}: no vision extra to re-run pose with, keeping the older run")
            _note(
                notes,
                _older_pose_note(
                    role, superseded, estimator, "the vision extra is not installed"
                ),
            )
            return superseded
        log(f"  {role.value}: pose needs the vision extra — pip install -e '.[vision]'")
        _note(notes, f"no pose for {role.value}: the vision extra is not installed")
        return None

    log(f"  {role.value}: running pose over {role_file.original_filename} ...")
    # The generator goes straight into estimate_pose, so no decoded frame outlives its
    # iteration and the clip's resolution stops mattering to memory (M7 Phase 1).
    with FileVideoSource(video_path, camera_id=role.value) as source:
        fps, width, height = source.fps, source.width, source.height
        reported = source.frame_count
        frames = estimate_pose(source.frames())

    if not frames:
        log(f"  {role.value}: no frames decoded")
        _note(notes, f"the {role.value} clip decoded no frames at all")
        return None
    if reported and reported != len(frames):
        # A file that opens cleanly and then decodes short produces a plausible-looking analysis
        # of an incomplete swing (docs/M7_TWO_PHONE_SPIKE.md, Q2). This used to go to stderr,
        # where the worker would lose it — it is a note precisely so it cannot pass unnoticed.
        message = (
            f"the {role.value} clip claims {reported} frames but only {len(frames)} decoded — "
            "this analysis may be of an incomplete swing"
        )
        log(f"  {role.value}: warning - {message}")
        _note(notes, message)

    keypoints = KeypointsFile(
        clip=ClipMetadata(
            fps=fps,
            width=width or None,
            height=height or None,
            frame_count=len(frames),
            source_sha256=role_file.content_sha256,
        ),
        frames=frames,
        pose_estimator=estimator,
    )
    save_keypoints(keypoints, cache_path)
    log(f"  {role.value}: {len(frames)} frames -> {cache_path.name}")
    return keypoints


def audio_for(
    swing_dir: Path,
    manifest: SwingManifest,
    role: Role,
    *,
    fps: float | None = None,
    force: bool = False,
    log: Log = _noop,
    notes: list[str] | None = None,
) -> AudioFile | None:
    """The transients in one view's clip, from cache when the clip behind it has not changed.

    A direct sibling of `keypoints_for`: same cache key (the clip's own sha256, recorded in the
    envelope and in the manifest), same lazy import of the extra it needs, same rule that a
    missing dependency is a note and not an exception. Writes `{role}.audio.json`.

    **The cache is keyed on the detector as well as on the clip.** `AUDIO_DETECTOR_VERSION` moves
    when detection would return a different list for the same waveform, and an artifact behind it
    is re-detected rather than read — otherwise a fix to `audio/impact.py` would be invisible on
    every bundle already on disk, which is the failure `analysis_version` exists to prevent one
    layer up. An install without the `audio` extra cannot re-detect and keeps the older list,
    saying so in a note.

    Cached for the reason pose is, one order of magnitude down: the decode is a subprocess and
    detection is an FFT every 5 ms, so a whole down-the-line clip is seconds rather than minutes
    — but the artifact is also the *auditable* half of an anchor. A sample index nobody can look
    at afterwards is a number nobody can check (`contracts/audio.py`).

    **`fps` is a parameter and not something this reads off the video**, which is the one place it
    departs from the file list M11 P4 was written against. The detector never sees the video, so a
    frame index is derived and not measured; the caller doing the deriving has to be the one that
    knows the frame rate, and that is `_auto_windows`, holding `KeypointsFile.clip.fps` already.
    Opening the clip a second time here to rediscover a number the pipeline is already holding
    would be a second answer to the same question, and the two could disagree. With `fps=None`
    every strike keeps `frame=None` — honest, and exactly what a caller that only needs sample
    offsets (P6's cross-correlation) should get.

    Returns `None` when there is nothing to detect in: no such role, the clip missing from disk,
    the `audio` extra absent, no audio stream in the container, or a decode that failed. An
    **empty `strikes` list is not one of those** — it is a result, and the one P5 reads, because
    a rehearsal swing makes no crack.
    """
    role_file = manifest.roles.get(role)
    if role_file is None:
        return None

    cache_path = swing_dir / f"{role.value}.audio.json"
    # Same clip, older detector: worth re-detecting, and worth keeping if this install cannot.
    superseded: AudioFile | None = None
    if cache_path.exists() and not force:
        cached = load_audio(cache_path)
        if cached.clip is None or cached.clip.source_sha256 != role_file.content_sha256:
            log(f"  {role.value}: cached audio is for a different clip, re-detecting")
        elif cached.detector_version != AUDIO_DETECTOR_VERSION:
            # A cache the current detector would disagree with, which the sha256 cannot see: it
            # keys on the *clip*, and the clip has not changed (`contracts/audio.py`).
            superseded = cached
            log(
                f"  {role.value}: cached audio is from detector v{cached.detector_version}, "
                f"re-detecting at v{AUDIO_DETECTOR_VERSION}"
            )
        else:
            log(f"  {role.value}: {len(cached.strikes)} strikes from cache")
            framed = _frames_derived(cached, fps)
            if framed is not cached:
                # The strikes were detected by a caller that did not know the fps, and this one
                # does. Re-deriving beats re-detecting — the sample index is the measurement and
                # it has not changed — and writing it back means the next reader gets the frames
                # too, with the fps they were derived under recorded beside them.
                save_audio(framed, cache_path)
                log(f"  {role.value}: frames derived at {fps} fps -> {cache_path.name}")
            return framed

    video_path = swing_dir / role_file.filename
    if not video_path.exists():
        log(f"  {role.value}: {video_path.name} is missing from the swing directory")
        _note(notes, f"the {role.value} clip is recorded in the manifest but missing from disk")
        return None

    # The detector is a Rust binary now (M20), so "can this machine listen?" is two questions
    # rather than one: the `audio` extra has to be installed *and* `golf-trigger` has to have been
    # built. They are asked separately because they cannot be asked together — naming
    # `TriggerUnavailable` in the same `except` as `ImportError` reads a name the failed import
    # never bound, which is an `UnboundLocalError` on exactly the path meant to degrade politely.
    # Both are asked before the clip is decoded, so a machine that cannot detect does not spend
    # thirty seconds of ffmpeg finding that out.
    why: str | None = None
    try:
        from golf_coach.audio.ffmpeg import FfmpegAudioSource, video_start_seconds
        from golf_coach.audio.source import NoAudioTrackError
        from golf_coach.audio.trigger import TriggerUnavailable, binary, detect_strikes
    except ImportError:
        why = "the audio extra is not installed"
    else:
        try:
            binary()
        except TriggerUnavailable as missing:
            why = str(missing)

    if why is not None:
        # An older detection beats none — a stored strike list is a measurement, where the pose
        # impact it would otherwise fall back to is an estimate. Said out loud rather than used
        # quietly: a reader comparing this run against one made on an install that *could* listen
        # has to be able to see why the two differ. And said with the reason attached rather than
        # as one word, because the two causes want different things from the reader: an extra is
        # `pip install`, an unbuilt binary is `cargo build --release`, and a note saying only
        # "no detector" sends them to the wrong one.
        if superseded is not None:
            log(f"  {role.value}: cannot re-detect ({why}), keeping the older detection")
            _note(
                notes,
                f"the {role.value} strikes were found by an older detector (v"
                f"{superseded.detector_version}) and this machine cannot re-run it: {why}",
            )
            return _frames_derived(superseded, fps)
        log(f"  {role.value}: no strike detection — {why}")
        _note(notes, f"no audio for {role.value}: {why}")
        return None

    log(f"  {role.value}: decoding audio from {role_file.original_filename} ...")
    try:
        clip = FfmpegAudioSource(video_path, camera_id=role.value).read()
    except NoAudioTrackError:
        # Told apart from a broken decode on purpose (`audio/source.py`): this bundle simply
        # cannot be acoustically anchored, which is a fact about the footage and worth recording,
        # where a decode failure is a broken tool. Measured 0/30 in this corpus (§E1) — an
        # unmeasured path is the one that rots, so it says so rather than falling through.
        log(f"  {role.value}: the clip carries no audio track")
        _note(
            notes,
            f"the {role.value} clip has no audio track, so it cannot be anchored on the "
            "ball strike",
        )
        return None
    except OSError as error:
        log(f"  {role.value}: audio could not be decoded — {error}")
        _note(
            notes,
            f"the {role.value} clip's audio could not be decoded, so no strike was detected in it",
        )
        return None

    # The second clock, measured while this file is open and the extra that can see it is
    # imported (M11 P10). It belongs to the *video* track, and it is recorded here rather than on
    # `keypoints.ClipMetadata` for a practical reason: the pose path decodes with OpenCV, which
    # never reports it, and the only question it answers is the one this artifact asks — which
    # frame a sample index lands on. A probe that fails is a note, not a failed decode: the
    # strikes are still the measurement and they are still worth storing.
    try:
        video_start_s = video_start_seconds(video_path)
    except OSError as error:
        log(f"  {role.value}: could not read the video timebase — {error}")
        _note(
            notes,
            f"the {role.value} clip's video timebase could not be read, so its strike frames "
            "assume the video starts where the audio does",
        )
        video_start_s = None
    else:
        if video_start_s:
            # Rare and load-bearing, so it is said out loud rather than left in the artifact:
            # four clips in this corpus start their video 105-125 ms after their audio (§E2).
            log(f"  {role.value}: video starts {video_start_s * 1000:.0f} ms after the audio")

    try:
        strikes = detect_strikes(clip.samples, clip.sample_rate)
    except TriggerUnavailable as error:
        # Checked before the decode, so reaching here means the binary went away underneath us or
        # failed on this particular waveform. Either way a stored measurement beats none and a
        # guess beats neither (ADR-010 §2).
        log(f"  {role.value}: strike detection failed — {error}")
        _note(notes, f"the {role.value} clip's audio could not be searched for a strike: {error}")
        return _frames_derived(superseded, fps) if superseded is not None else None
    audio = _frames_derived(
        AudioFile(
            clip=AudioClipMetadata(
                sample_rate=clip.sample_rate,
                # The decoded length, which is the honest one: it carries the edit list's effect
                # where the container's own duration field does not (§E2).
                duration_s=clip.duration_s,
                source_sha256=role_file.content_sha256,
                stream_index=clip.stream_index,
                video_start_s=video_start_s,
            ),
            strikes=strikes,
            detector_version=AUDIO_DETECTOR_VERSION,
        ),
        fps,
    )
    save_audio(audio, cache_path)
    log(f"  {role.value}: {len(strikes)} strikes -> {cache_path.name}")
    return audio


def _frames_derived(audio: AudioFile, fps: float | None) -> AudioFile:
    """`audio` with every strike's video frame worked out under `fps`; `audio` itself if it can't.

    Pure, and returns the *same object* when there is nothing to add, which is what lets the
    caller tell "already derived" from "just derived" without comparing two models field by field.

    Derived and never guessed: with no fps, no sample rate, or a strike list already carrying
    frames from this same fps, nothing is invented and nothing already recorded is thrown away —
    a caller that does not know the frame rate must not erase the frames a caller that did know
    wrote (ADR-010 §2). Rounding is a floor, because a frame index answers "which frame was being
    exposed when this happened", and a strike 1.5 frames in happened during frame 1.

    **`video_start_s` is subtracted first, and that subtraction is the whole of M11 P10.** A sample
    index is a time on the *presentation* clock, because `FfmpegAudioSource` applies the
    container's edit list; a frame index counts from whatever frame the video decoder hands back
    first. Four down-the-line clips here start their video 105-125 ms into that clock, so on them
    the two differ by 6.3 to 7.5 frames and every earlier reading of this artifact was that much
    late. Absent, it is treated as zero — the assumption every reader made before the field
    existed, and true of 26 of the 30 clips on disk — which is why `AUDIO_DETECTOR_VERSION` moved
    with it rather than leaving stored frames to be re-derived silently under a changed rule.

    A strike **before** the first decoded frame gets `frame=None` rather than a clamp to 0: the
    sound is real, it is the measurement, and there is genuinely no frame it happened during.
    """
    clip = audio.clip
    if fps is None or fps <= 0.0 or clip is None or not clip.sample_rate:
        return audio

    rate = clip.sample_rate
    start = clip.video_start_s or 0.0
    framed: list[AudioStrike] = [
        strike.model_copy(update={"frame": _frame_of(strike.sample, rate, fps, start)})
        for strike in audio.strikes
    ]
    if clip.fps == fps and framed == audio.strikes:
        # Nothing to add: derived under this same fps already, and every strike that could have a
        # frame has one. Compared rather than assumed, because `frame=None` now has a second
        # meaning — a strike ahead of the first decoded frame keeps None forever, and a guard of
        # "are they all filled in?" would re-derive and re-save such a clip on every read.
        return audio
    return AudioFile(
        clip=clip.model_copy(update={"fps": fps}),
        strikes=framed,
        # Carried, never re-stamped: deriving a frame index from a sample index is arithmetic, and
        # arithmetic does not turn an older detector's list into this one's.
        detector_version=audio.detector_version,
    )


def _frame_of(sample: int, rate: int, fps: float, video_start_s: float) -> int | None:
    """Which decoded frame a sample index lands on, or None when it precedes the first. [M11 P10]

    Named rather than left inline because the arithmetic has a sign now and `int()` no longer
    does what the docstring above claims: it truncates *toward zero*, so it would put a strike
    0.4 frames before the video started on frame 0 and hide the case this returns None for.
    """
    frame = math.floor((sample / rate - video_start_s) * fps)
    return frame if frame >= 0 else None


def _shot_for(
    swing_dir: Path,
    manifest: SwingManifest,
    *,
    options: PipelineOptions,
    log: Log,
) -> tuple[ShotData | None, str | None]:
    """The launch-monitor shot for this swing. Returns `(shot, note_if_absent)`.

    Cache first, and that is not merely an optimisation: the manifest already records the
    photo's sha256 and `ShotStore` is keyed on exactly that, so a shot imported earlier — by
    this or by `import_shot_screens.py` — attaches on the base install without OCR, without
    OpenCV, and without opening the image.
    """
    role_file = manifest.roles.get(Role.SHOT_SCREEN)
    if role_file is None:
        return None, "no shot-screen photo in this bundle — no launch-monitor data to attach"

    store = ShotStore(options.shots_dir)
    if not options.force_ocr:
        cached = store.get(role_file.content_sha256)
        if cached is not None:
            log(f"  shot_screen: {cached.shot_id} from the shot store")
            return cached, None

    if options.skip_ocr:
        return None, "shot-screen OCR skipped by request — no launch-monitor data attached"

    image_path = swing_dir / role_file.filename
    if not image_path.exists():
        return None, f"shot-screen photo {role_file.filename} is missing from the swing directory"

    from golf_coach.launch_monitor.screen.importer import (
        MissingOCRExtra,
        build_recognizer,
        import_screen,
    )
    from golf_coach.launch_monitor.screen.profiles import load_profile

    try:
        recognizer = build_recognizer(settings.ocr_engine)
    except (MissingOCRExtra, ValueError) as exc:
        return None, f"no launch-monitor data attached: {exc}"

    log(f"  shot_screen: reading {role_file.original_filename} ...")
    try:
        status, shot = import_screen(
            image_path,
            recognizer=recognizer,
            profile=load_profile(settings.launch_monitor_profile),
            store=store,
            session_id=manifest.session_id,
            min_confidence=options.min_confidence,
            force=options.force_ocr,
            shot_id=f"{manifest.session_id}-{manifest.swing_id}",
        )
    except Exception as exc:
        # The shot screen is the *optional* third of a bundle and this is the one place that
        # forgot it. `import_screen` decodes an image, warps it, and runs a third-party OCR
        # engine over it, so its failure surface is wide and none of it is under this repo's
        # control — a HEIC photo raised OSError out of `preprocess.load_image` and took a whole
        # analysis down with it, discarding pose that had already run and every mechanics
        # checkpoint that had already scored. Nothing about a photo justifies losing the swing.
        #
        # Deliberately broad. The narrow `except (MissingOCRExtra, ValueError)` above guards
        # `build_recognizer`, where the failures are this repo's own and enumerable; here they
        # are OpenCV's, Pillow's and PaddleOCR's, and an exception type this list had not
        # anticipated is exactly the bug being fixed. `notes` is what makes that safe — the
        # result says out loud that no numbers attached and why (see this module's docstring:
        # "log is for progress; notes are for truth").
        return None, f"the shot screen could not be read — no numbers attached: {exc}"
    if shot is None:
        return None, f"the shot screen could not be read ({status}) — no numbers attached"
    log(f"  shot_screen: {status}")
    return shot, None


def _handedness_for(manifest: SwingManifest) -> tuple[Handedness | None, str | None]:
    """The swinging golfer's handedness, or `(None, why_not)`.

    The registry lookup lives here rather than in `analysis` on purpose: the analysis core is pure
    and depends only on `contracts` (ADR-008), so *reading a JSON file to find out who swung* is the
    shell's job. This is the whole seam M6.5 named — `Golfer.handedness` has been recorded since
    career step 1, and nothing could carry it to the one checkpoint whose sign depends on it.

    Both failure modes are narrated rather than defaulted. Guessing right-handed would score a
    left-handed golfer's ordinary impact position as a gross fault, so an unattributed swing loses
    the checkpoint and says so — `unscored` reports it with reason `NO_HANDEDNESS`.

    **The note here is not a duplicate of that reason, and the difference is the layer.** The
    evaluator knows only that no handedness reached it; it cannot know *why* none did, because
    `analysis` does not import the golfer registry and must not. Whether nobody picked a golfer or
    a `player_id` points at a golfer who is not in the registry is knowledge that exists only in
    this shell — and the two want different fixes. So the reason says what was missing and the note
    says how it went missing.
    """
    if manifest.player_id is None:
        return None, (
            "no golfer is attributed to this swing, so the head_stays_back checkpoint could not be "
            "scored — its sign depends on which side the golfer swings from"
        )
    golfer = GolferStore(settings.golfers_dir).get(manifest.player_id)
    if golfer is None:
        return None, (
            f"golfer {manifest.player_id!r} is not in the registry, so the head_stays_back "
            "checkpoint could not be scored — its sign depends on which side the golfer swings from"
        )
    return golfer.handedness, None


def _loft_for(manifest: SwingManifest) -> tuple[float | None, str | None]:
    """The declared loft of the club this swing was hit with, and the repair when there is none.

    [M15 P11] The third artifact read this shell performs on `analysis`'s behalf, beside the shot
    and the handedness, and for the same ADR-008 reason: opening a bag file to find out what a `7i`
    is bent to is not something a pure analysis core may do.

    **The second element is a candidate remedy, not a note to append.** Loft picks a branch of the
    *spin solve*, so a shot whose screen printed a spin never reaches for it and flies perfectly
    well without one — two swings on disk are exactly that. The caller appends this only once the
    engine has reported `NO_CLUB_LOFT`, which is the only evidence that the loft was wanted.

    **It is not a duplicate of that reason either, and the difference is the layer** — the same
    split `_handedness_for` above makes. `analysis` learns only that no loft arrived; which of the
    four repairs is wanted — tag the swing with a club, add the club to the bag, put a number on
    the entry — exists only here, and they are different minutes of work on different pages.
    `storage.flight_inputs.LoftGap` is that distinction, and the reason it is enumerated rather
    than being one bare `None`.
    """
    loft, gap = loft_for_club(manifest.player_id, manifest.club, golfers_dir=settings.golfers_dir)
    if loft is not None:
        return loft, None
    return None, loft_remedy(gap, manifest.club)


def loft_remedy(gap: LoftGap | None, club: ClubId | None) -> str | None:
    """The minutes of work that would give this swing a loft, named per gap.

    Public, and the only thing in this module that is: [M15 P14] gave the flight an HTTP route,
    and `api/app.py` resolves the same loft through the same `loft_for_club` in order to draw the
    flight on demand. Both shells owe the golfer the same sentence, and a second copy of four
    repair strings is four strings that drift — `contracts/caveats.py`'s argument, one layer up.

    It stays in `api/` rather than moving down beside `LoftGap` in `storage/flight_inputs.py`
    because every sentence here names a *page*: which of the repairs is wanted is storage's
    knowledge, but "the bag page" is this shell's.
    """
    remedy = {
        LoftGap.NO_CLUB_TAG: (
            "no club is tagged on this swing, so its ball flight could not be simulated — tag one "
            "on the results page and re-analyze"
        ),
        LoftGap.NO_BAG_ENTRY: (
            f"club {club} is not in this golfer's bag, so its loft is unknown and the "
            "ball flight could not be simulated — add the club on the bag page"
        ),
        LoftGap.NO_DECLARED_LOFT: (
            f"the bag entry for {club} carries no loft, so the ball flight could not be "
            "simulated — the club's book loft on the bag page is enough"
        ),
    }
    # `NO_SWING` cannot arrive from either caller: both are holding the swing. Defaulted rather
    # than asserted, because a note is not worth taking an analysis down over.
    return remedy.get(gap) if gap is not None else None


def _pick_swing(
    keypoints: KeypointsFile,
    *,
    wrist: PoseLandmark,
    reference_downswing_s: float | None = None,
    strike_frames: list[int] | None = None,
) -> SwingChoice | None:
    """One view's swing, chosen alone or against a duration the other view already measured.

    Selection only — no `log`, no `notes`. That split is what lets `_auto_windows` try a view
    *twice*: a face-on decline the down-the-line view then rescues must not have already told
    `analysis.json` that the whole clip was scored.

    With a reference this is `select_matching_swing`, without one it is `select_swing`. There is
    deliberately no "matched, else plain" fallback: the matching rule keeps every candidate the
    duration band keeps **plus** the ones the reference vouches for, and both rules end at the
    same `_lone_candidate_choice`, so `select_matching_swing` returns `None` exactly where
    `select_swing` would on the same clip. A fallback there would be unreachable.

    `strike_frames` are the transients heard in **this view's own clip**, in its own frame
    numbering, and both rules apply them ahead of everything else (M11 P5). Passing another view's
    strikes here would be a real error rather than a slack one: the two phones start at different
    instants, and putting them on one clock is P6's job, not a detail this call site may assume.
    """
    fps = keypoints.clip.fps if keypoints.clip else None
    frames = smooth_keypoints(keypoints.frames)
    if reference_downswing_s is not None:
        return select_matching_swing(
            frames,
            fps=fps,
            reference_downswing_s=reference_downswing_s,
            wrist=wrist,
            strike_frames=strike_frames,
        )
    return select_swing(frames, fps=fps, wrist=wrist, strike_frames=strike_frames)


def _downswing_seconds(choice: SwingChoice | None, keypoints: KeypointsFile) -> float | None:
    """The chosen descent's duration — the whole of what one view offers the other.

    `None` when there is nothing to offer: no choice, no frame rate, or a descent measuring no
    time at all. `select_matching_swing` declines a non-positive reference, so passing one on
    would quietly turn "no reference" into "no window".
    """
    fps = keypoints.clip.fps if keypoints.clip else None
    if choice is None or fps is None or fps <= 0.0:
        return None
    seconds = (choice.downswing.impact - choice.downswing.top) / fps
    return seconds if seconds > 0.0 else None


def _narrate_choice(
    label: str,
    choice: SwingChoice | None,
    keypoints: KeypointsFile,
    *,
    wrist: PoseLandmark,
    log: Log,
    notes: list[str],
) -> tuple[int, int] | None:
    """One view's final pick said out loud: its window, or why it has none.

    A decline is a note, not just a log line: scoring the whole clip when it holds practice
    swings produces numbers that look fine and describe the wrong motion.

    The narration says which landmark was read because on a down-the-line clip that is the
    *answer* to a decline, not a detail of it: the lead wrist is the far arm there and is tracked
    in 39% of frames, so "no plausible downswing" usually means "not on that arm".

    Called once per view and only after the last attempt at it — `_auto_windows` may pick face-on
    twice, and a note describing the attempt that was superseded would be false in the file.

    **When a ball strike decided the pick, this says so** — through `choice.reason`, which names
    the strike itself (`phases.select_swing` rule 0), rather than through a second sentence added
    here. That is deliberate and it is the reason `SwingChoice.reason` is prose in the first
    place: one selection rule, one place that explains it. There is no matching *note* for the
    positive case, because a note is for a reader who has to be told something went differently
    from usual, and a strike is the evidence this rule most wants to have.
    """
    if choice is None:
        # Named from the constant rather than from `wrist.name`, which would say "left wrist" —
        # true of the landmark index and wrong for a left-handed golfer, whose lead wrist is the
        # right one. `phases.LEAD_WRIST` carries that assumption; this only has to report it.
        landmark = "trail wrist" if wrist is TRAIL_WRIST else "lead wrist"
        fps = keypoints.clip.fps if keypoints.clip else None
        # "no plausible downswing" covers a matched pick too — it declines only where the plain
        # rule also would (see `_pick_swing`), so there is no third reason to name here.
        why = "the keypoints file records no fps" if fps is None else "no plausible downswing"
        log(f"  {label}: could not pick a swing ({why}, on the {landmark}) — using the whole "
            "clip. Run --list-swings and pass a window if this clip holds more than one swing")
        notes.append(
            f"could not pick a swing in the {label} view ({why}, reading the {landmark}) — the "
            "whole clip was scored, so these numbers describe every motion in it, not one swing"
        )
        return None
    log(f"  {label}: {choice.reason}")
    return choice.window


def _auto_windows(
    views: dict[Role, KeypointsFile],
    *,
    window_face_on: tuple[int, int] | None,
    window_dtl: tuple[int, int] | None,
    strikes: dict[Role, list[int]] | None = None,
    log: Log,
    notes: list[str],
) -> tuple[tuple[int, int] | None, tuple[int, int] | None]:
    """Both views' windows: the confident view picked first, the other matched against it.

    Face-on leads because it is the view that works — it picked a sane swing on 11 of 11 stored
    clips, while "the last plausible descent" on a down-the-line clip is routinely a move made
    after the ball was gone, the phone on the busy side of the bay having kept rolling 15-24 s
    past impact. Six of the fifteen stored bundles were windowed on such a descent until this
    ordering existed (M10 §A2).

    **The pick is mutual, not face-on-first**, and the extra rule pays for itself on the view that
    matters most. When face-on declines, the down-the-line view is picked alone and *its* duration
    becomes the reference for a second attempt at face-on. On `2026-08-23/8` face-on's real
    descent measured 0.467 s and missed the duration bound of the day by 0.017 s, so the view every
    checkpoint is measured from was scored over its whole clip; the down-the-line view was
    confident at 0.400 s and recovered it. That bound is now `phases.POSSIBLE_DOWNSWING_S` and
    admits 0.467 s outright, so the rescue no longer fires on *this* bundle — it is kept because
    the reverse path is the one with no alternative, not because of the clip it was found on.

    The risk that buys is stated rather than guarded: a down-the-line reference that is itself a
    post-impact descent would hand face-on a confidently wrong window instead of a decline, and
    that is the worse failure of the two (`phases._MATCH_TOLERANCE_S` says so). It is accepted
    only because the reverse runs nowhere else — on that path the alternative is already "every
    motion in the clip scored as one swing".

    An explicit window is not a reference. A hand-picked window is not a downswing duration, and
    it may deliberately point at a different swing than the selector would have chosen, so the
    other view is picked alone and told so.

    **`strikes` does not change any of that ordering** (M11 P5). Each view is handed the transients
    heard in its own clip and applies them before its own duration rules, so the mutual pick above
    runs exactly as described — face-on first, the other matched to it, and back again on a
    decline. What audio changes is what each of those three calls is choosing *between*, which is
    why a bundle with no audio at all still walks the same path. It is optional here for that
    reason and not merely for the tests' convenience: `analyze_swing_dir` passes `{}` on a base
    install, and `scripts/align_swings.py` has its own copy of this ordering with no audio in it.
    """
    strikes = strikes or {}
    face_on = views[Role.FACE_ON]
    face_on_given = window_face_on is not None
    # `None` means there is nothing to pick for the down-the-line view — it is absent, or its
    # window was given by hand, and an explicit window always wins.
    dtl = views.get(Role.DOWN_THE_LINE) if window_dtl is None else None

    face_on_choice = (
        None
        if face_on_given
        else _pick_swing(
            face_on, wrist=LEAD_WRIST, strike_frames=strikes.get(Role.FACE_ON)
        )
    )
    reference = _downswing_seconds(face_on_choice, face_on)

    dtl_choice: SwingChoice | None = None
    if dtl is not None:
        # The trail wrist, matching `engine.analyze_swing_bundle`'s own down-the-line call — and
        # it has to match, because this window is what that segmentation then runs over. A window
        # chosen on one wrist and anchors measured on the other are two different swings: on the
        # worst bundle the lead wrist reads a 9.7 s "downswing" here and windows the entire clip,
        # while the trail wrist finds the swing at 0.40 s (M10 §A1). Face-on keeps the default;
        # it is the view the rule was tuned on.
        dtl_choice = _pick_swing(
            dtl,
            wrist=TRAIL_WRIST,
            reference_downswing_s=reference,
            strike_frames=strikes.get(Role.DOWN_THE_LINE),
        )

    matched_in_reverse = False
    if not face_on_given and face_on_choice is None and dtl is not None:
        back_reference = _downswing_seconds(dtl_choice, dtl)
        if back_reference is not None:
            face_on_choice = _pick_swing(
                face_on,
                wrist=LEAD_WRIST,
                reference_downswing_s=back_reference,
                strike_frames=strikes.get(Role.FACE_ON),
            )
            matched_in_reverse = face_on_choice is not None

    if not face_on_given:
        window_face_on = _narrate_choice(
            "face-on", face_on_choice, face_on, wrist=LEAD_WRIST, log=log, notes=notes
        )
        if matched_in_reverse:
            notes.append(
                "the face-on swing could not be picked from that clip alone and was chosen by "
                "matching the down-the-line view instead — a weaker basis than usual, so check "
                "the aligned video shows the swing and not a practice move"
            )
    if dtl is not None:
        window_dtl = _narrate_choice(
            "down-the-line", dtl_choice, dtl, wrist=TRAIL_WRIST, log=log, notes=notes
        )
        if dtl_choice is not None and reference is None:
            why = (
                "the face-on window was given rather than measured"
                if face_on_given
                else "the face-on view has no measured downswing to match against"
            )
            notes.append(
                f"the down-the-line swing was picked from that clip alone ({why}), with nothing "
                "to cross-check it — on a clip that keeps rolling past impact the last plausible "
                "descent is often a move made after the ball was gone"
            )
    return window_face_on, window_dtl


def _render(
    result: SwingBundleResult,
    swing_dir: Path,
    manifest: SwingManifest,
    views: dict[Role, KeypointsFile],
    *,
    options: PipelineOptions,
    log: Log,
    notes: list[str],
) -> tuple[Path | None, str | None]:
    """Write the side-by-side MP4, streaming both clips. Returns `(path, codec)`."""
    from contextlib import ExitStack

    alignment = result.alignment
    if alignment is None or alignment.a is None or alignment.b is None:
        log("\nNo aligned video: the two views could not be aligned.")
        return None, None

    face_on, dtl = views[Role.FACE_ON], views[Role.DOWN_THE_LINE]
    schedule = pair_frames(
        alignment, len(face_on.frames), len(dtl.frames), reference="a",
        tau_range=options.tau_range,
    )
    if not schedule:
        log("\nNo aligned video: the two clips share no overlapping swing time.")
        notes.append("no aligned video: the two clips share no overlapping swing time")
        return None, None

    # The one property of this render a viewer can check by eye, so it is worth saying out
    # loud: `2026-08-23/9` shipped a down-the-line panel at 2.08x for a milestone with no line
    # of log anywhere admitting it. `pair_frames` has already held the schedule to native rate
    # by the time this prints, so these numbers report what the *warp alone* would have done —
    # a speed far from 1.00 means the two views' anchors still disagree and the guard carried
    # the render.
    speeds = warp_speeds(alignment, reference="a")
    if speeds:
        log(
            "\nWarp speed (down-the-line, before the render guard): "
            + ", ".join(f"{name} {speed:.2f}x" for name, speed in speeds.items())
        )

    try:
        from golf_coach.capture.file import FileVideoSource
        from golf_coach.pose.side_by_side import (
            BROWSER_HOSTILE_CODECS,
            Panel,
            render_side_by_side,
        )
    except ImportError:
        log("\nNo aligned video: rendering needs the vision extra — pip install -e '.[vision]'")
        return None, None

    out_path = swing_dir / ALIGNED_NAME
    with ExitStack() as stack:
        sources = {
            role: stack.enter_context(FileVideoSource(swing_dir / manifest.roles[role].filename))
            for role, _ in VIEWS
            if role in manifest.roles and (swing_dir / manifest.roles[role].filename).exists()
        }
        render = render_side_by_side(
            out_path,
            schedule,
            Panel(face_on.frames, "face-on", face_on.clip,
                  sources[Role.FACE_ON].frames() if Role.FACE_ON in sources else ()),
            Panel(dtl.frames, "down-the-line", dtl.clip,
                  sources[Role.DOWN_THE_LINE].frames()
                  if Role.DOWN_THE_LINE in sources else ()),
            fps=alignment.a.anchors.fps or 60.0,
            quality=alignment.quality,
        )
    log(
        f"\nWrote {render.frames} aligned frames ({render.codec}) -> {out_path}"
        f"  [reads back {render.frames_read}]"
    )
    if render.frames_read is not None and render.frames_read != render.frames:
        # Logged rather than raised: a short render is not a wrong score, and the analysis stands
        # without the video. It is worth saying loudly because the render's own stderr looks like
        # this whether it succeeded or not — see `side_by_side._CODECS`.
        log(
            f"  note: the file decodes {render.frames_read} of {render.frames} frames written - "
            "re-render before trusting it"
        )
    if render.codec in BROWSER_HOSTILE_CODECS:
        # Deliberately not a `note`: notes describe the *swing*, and this describes the file we
        # wrapped it in. It travels as `PipelineOutcome.video_codec` instead, which the results
        # page reads as structured data rather than parsing prose.
        log(f"  note: {render.codec} plays in VLC but not in most browsers - see README")
    return out_path, render.codec


def _recorded_video(
    swing_dir: Path,
    outcome: PipelineOutcome,
    previous: AnalysisState | None,
) -> tuple[str | None, str | None]:
    """Which `aligned.mp4`, if any, this run should claim — `(filename, codec)`.

    **A run that did not render must not erase the record of one that did.** `reanalyze.py` keeps
    the render off by default and says so in its own docstring — "anything analyzed without a
    render keeps whatever `aligned.mp4` it already had" — but the sidecar did not keep it, and the
    sidecar is what `GET /api/sessions/.../swings/...` serves as `has_video`. So a corpus-wide
    re-analysis without `--video` left thirteen of fifteen swings advertising no video while a
    perfectly playable H.264 file sat in each directory, and `results.html` fell back to the raw
    iPhone clip — HEVC in a QuickTime container, which most browsers refuse.

    The carry-forward is deliberately narrow. It applies only when no render was *attempted*
    (`render_attempted`); a run that tried and produced nothing has contradicted whatever is on
    disk, and the stale file must not be re-advertised. And the file is stat'd rather than
    trusted, because the sidecar outlives anything anyone deletes by hand.

    Staleness in the other direction — the anchors moved under a file this run did not re-render —
    is `reanalyze._video_went_stale`'s job and stays there: it can see the before and after, and
    it reports rather than deletes, because a slightly-misaligned render is still worth watching.
    """
    if outcome.video_path is not None:
        return outcome.video_path.name, outcome.video_codec
    if outcome.render_attempted or previous is None or previous.video is None:
        return None, None
    if not (swing_dir / previous.video).is_file():
        return None, None
    return previous.video, previous.video_codec


def record_state(
    swing_dir: Path,
    manifest: SwingManifest,
    outcome: PipelineOutcome,
    *,
    started_at: datetime,
) -> AnalysisState:
    """Write the `analysis.state.json` summary of a run that just finished.

    **The sidecar is a denormalised copy of `analysis.json`, so whoever writes one must write the
    other.** It was not always so: the worker wrote it and `analyze_swing_dir` did not, which made
    every CLI run a way to leave the two disagreeing. `2026-08-09/2` sat for three days with a
    sidecar reading 66.67 beside an analysis reading 94.92 — the upload page serves the sidecar
    and the results page serves the analysis, so the same swing showed two scores. Nothing could
    catch it, either: `AnalysisState.matches` compares the *inputs*, and re-analysis does not
    change the inputs.

    Any existing state is read first so the worker's `queued_at` survives a run it did not start,
    and — see `_recorded_video` — so does the render a run that skipped rendering did not replace.
    Everything else describes this run.
    """
    previous = load_state(swing_dir)
    missing = [role.value for role in manifest.missing_roles()]
    completed = now()
    result = outcome.result
    video, video_codec = _recorded_video(swing_dir, outcome, previous)

    state = AnalysisState(
        status="done" if result is not None else "failed",
        inputs=input_hashes(manifest),
        queued_at=previous.queued_at if previous else None,
        started_at=started_at,
        completed_at=completed,
        duration_seconds=(completed - started_at).total_seconds(),
        error=None if result is not None else (outcome.error or "the pipeline produced no result"),
        partial=bool(missing),
        missing_roles=missing,
        video=video,
        video_codec=video_codec,
        score=result.swing.overall_score if result is not None else None,
        headline=(
            result.feedback.headline if result is not None and result.feedback else None
        ),
    )
    save_state(state, swing_dir)
    return state


def analyze_swing_dir(
    swing_dir: Path,
    *,
    options: PipelineOptions | None = None,
    log: Log = _noop,
) -> PipelineOutcome:
    """Run the whole pipeline over one assembled swing bundle.

    Never raises for an *expected* failure — a missing face-on view, an unreadable manifest — it
    returns an outcome with `error` set. The caller decides whether that is an exit code; the
    state sidecar is written here either way, so it can never disagree with the `analysis.json`
    beside it. Unexpected failures still propagate; the worker catches those separately and
    records them, because it is the only layer that can see a crash.
    """
    options = options or PipelineOptions()
    started_at = now()
    manifest = load_manifest(manifest_path(swing_dir))
    if manifest is None:
        # No state written: without a manifest there are no `inputs` to key one on, and a
        # directory that cannot say which bytes it holds is not a swing yet.
        return PipelineOutcome(error=f"{swing_dir} has no readable manifest")

    missing = manifest.missing_roles()
    log(f"\nBundle {manifest.session_id}/{manifest.swing_id}  [{manifest.status()}]")
    if missing:
        log(f"  missing: {', '.join(role.value for role in missing)}")

    notes: list[str] = []
    if missing:
        notes.append(
            "analyzed without " + ", ".join(role.value for role in missing)
            + " — the result is thinner than a complete bundle's"
        )
    band_mismatch = _band_estimator_note(pose_estimator_name())
    if band_mismatch is not None:
        notes.append(band_mismatch)

    log("\nPose:")
    views: dict[Role, KeypointsFile] = {}
    for role, label in VIEWS:
        keypoints = keypoints_for(
            swing_dir, manifest, role, force=options.force_pose, log=log, notes=notes
        )
        if keypoints is not None:
            views[role] = keypoints
        elif role is Role.FACE_ON:
            outcome = PipelineOutcome(
                missing_roles=missing,
                error=(
                    f"no usable {label} view — that is the view every checkpoint is measured "
                    "from, so there is nothing to score"
                ),
            )
            record_state(swing_dir, manifest, outcome, started_at=started_at)
            return outcome

    window_face_on = options.window_face_on
    window_dtl = options.window_down_the_line
    # Unconditional, where M11 P5 ran this only under `auto_window` (M11 P6 hoisted it).
    # Selection is no longer the only reader: `analyze_swing_bundle` pins each view's tau=2
    # anchor to the strike that view heard, and a bundle whose windows were given by hand wants
    # that as much as one that found them — arguably more, since a hand-picked window means
    # someone was already unhappy with what the detector did.
    strikes: dict[Role, list[int]] = {}
    log("\nAudio:")
    for role, label in VIEWS:
        if role not in views:
            continue
        clip = views[role].clip
        audio = audio_for(
            swing_dir,
            manifest,
            role,
            # The fps this view's pose was decoded at, which is what makes a sample index a
            # frame index in *this clip's* numbering — the numbering selection then compares
            # against `Downswing.impact`, and the numbering `alignment.with_measured_impact`
            # compares against `SwingAnchors.impact` (`contracts/audio.py`, `_frames_derived`).
            fps=clip.fps if clip else None,
            force=options.force_audio,
            log=log,
            notes=notes,
        )
        if audio is None:
            continue
        strikes[role] = [
            strike.frame for strike in audio.strikes if strike.frame is not None
        ]
        if not audio.strikes:
            # The one result that is a *finding* rather than a gap: the detector ran over this
            # clip and nothing in it stood above the clip's own floor. It does not decline the
            # window — `phases._struck` records why silence must not throw a window away — so
            # the note is what carries it to a reader of `analysis.json`.
            log(f"  {role.value}: no ball strike in this clip")
            notes.append(
                f"no ball strike was heard in the {label} clip, so its swing was picked on "
                "movement alone — if this was a rehearsal, the numbers describe a swing that "
                "never hit a ball"
            )

    if options.auto_window:
        log("\nSwing selection:")
        window_face_on, window_dtl = _auto_windows(
            views,
            window_face_on=window_face_on,
            window_dtl=window_dtl,
            strikes=strikes,
            log=log,
            notes=notes,
        )

    log("\nShot data:")
    shot, shot_note = _shot_for(swing_dir, manifest, options=options, log=log)
    if shot_note:
        log(f"  {shot_note}")
        notes.append(shot_note)

    if shot is not None and strikes and not any(strikes.values()):
        # Two independent witnesses to one event disagreeing, which is the only kind of check the
        # simulator's own data can give the footage: ADR-014's shot was read off the screen and
        # the strikes off the microphone, and neither knows about the other. Silence is the
        # surprising half — §E1 measured every real strike at z = 113-599 above its clip's own
        # floor — so a recorded ball with no crack in either clip most likely means this bundle's
        # video and its shot are not the same swing. A note and not an error, deliberately: the
        # shot still scores, and mis-pairing is a thing only a human can confirm.
        notes.append(
            "the simulator recorded a shot for this swing but no clip in the bundle carries a "
            "ball strike — the footage and the shot data may not describe the same swing"
        )

    handedness, handedness_note = _handedness_for(manifest)
    if handedness_note:
        log(f"\nGolfer:\n  {handedness_note}")
        notes.append(handedness_note)

    # Resolved before the analysis and **narrated after it**, which is not fussiness — see below.
    loft_deg, loft_remedy = _loft_for(manifest) if shot is not None else (None, None)

    result = analyze_swing_bundle(
        swing_id=manifest.swing_id,
        session_id=manifest.session_id,
        face_on=views[Role.FACE_ON],
        down_the_line=views.get(Role.DOWN_THE_LINE),
        shot=shot,
        intent=PracticeGoal(club=options.club),
        face_on_window=window_face_on,
        down_the_line_window=window_dtl,
        face_on_strikes=strikes.get(Role.FACE_ON),
        down_the_line_strikes=strikes.get(Role.DOWN_THE_LINE),
        handedness=handedness,
        loft_deg=loft_deg,
    )

    # ⚠️ **The loft note is gated on the engine having actually missed it**, and the corpus is why.
    # Loft picks a branch of the *spin solve*, so a shot whose screen printed a spin needs none at
    # all — and the two 2026-08-10 swings are exactly that: no club tagged, and they fly anyway.
    # Narrating the gap up front told those two swings to go and tag a club so their ball flight
    # could be simulated, beside the simulated ball flight. `NO_CLUB_LOFT` in `unscored` is the
    # engine saying the loft was reached for and missing, which is the only condition under which
    # the repair below is a repair.
    if loft_remedy and any(
        entry.reason is UnscoredReason.NO_CLUB_LOFT for entry in result.swing.unscored
    ):
        log(f"\nBall flight:\n  {loft_remedy}")
        notes.append(loft_remedy)

    video_path: Path | None = None
    video_codec: str | None = None
    render_attempted = options.render_video and Role.DOWN_THE_LINE in views
    if render_attempted:
        video_path, video_codec = _render(
            result, swing_dir, manifest, views, options=options, log=log, notes=notes
        )

    # Extended after the render so a codec warning lands in the stored JSON too. Order is
    # encounter order, which is what makes the list readable top to bottom.
    result.notes.extend(notes)
    # `analysis` must not import `feedback` (ADR-008), so the shell joins the two — which is
    # what makes analysis.json the complete result instead of something to recompute.
    result.feedback = build_feedback(result.swing)

    # Coaching runs last, on the finished result, and is the one step allowed to fail without
    # costing anything: `generate_coaching` returns its reason rather than raising, the reason
    # becomes a note, and the swing keeps its score. It reads `result.notes`, so it has to come
    # after the extend above — the brief should carry everything that degraded, not most of it.
    if options.coaching:
        log("\nCoaching:")
        # One of the three sanctioned `get_secret_value` sites (tests/test_config.py pins the
        # set). `generate_coaching` takes a plain `str` so it stays injectable in tests without
        # importing pydantic; the unwrap belongs here, at the boundary, not in `feedback/`.
        key = settings.anthropic_api_key
        coaching = generate_coaching(
            result,
            model=settings.coaching_model,
            api_key=key.get_secret_value() if key else None,
        )
        if coaching.text and coaching.provenance is not None:
            result.feedback.coaching_text = coaching.text
            result.feedback.coaching = coaching.provenance
            log(f"  {coaching.provenance.model} wrote {len(coaching.text)} characters")
        elif coaching.note:
            log(f"  {coaching.note}")
            result.notes.append(coaching.note)

    # The heavy streams are excluded deliberately: the keypoints already sit beside this file in
    # their own per-view JSON, and inlining several hundred frames of 33 landmarks would make
    # the artifact a results page has to fetch tens of megabytes for nothing.
    analysis_path = swing_dir / ANALYSIS_NAME
    analysis_path.write_text(
        result.model_dump_json(indent=2, exclude={"swing": {"keypoints", "detections"}}),
        encoding="utf-8",
    )
    log(f"\nWrote {analysis_path}")

    needs_review = (
        shot is not None and shot.provenance is not None and shot.provenance.needs_review
    )
    outcome = PipelineOutcome(
        result=result,
        analysis_path=analysis_path,
        video_path=video_path,
        video_codec=video_codec,
        render_attempted=render_attempted,
        missing_roles=missing,
        flagged=bool(needs_review or missing),
    )
    record_state(swing_dir, manifest, outcome, started_at=started_at)
    return outcome


def _band_estimator_note(estimator: str) -> str | None:
    """Say so when the swing and the bands it is judged against were measured differently.

    ADR-012 §4 is explicit that a band is only comparable to a swing measured the same way: the
    reference metrics and the golfer's metrics share an estimator, so most of its bias is
    common-mode and cancels — and that argument fails the moment the two sides diverge. Which is
    exactly what moving `settings.pose_model_variant` off the variant the corpus was extracted
    with does.

    It is a note and not a refusal, because the bands are still the best reference available and
    a swing scored against them is worth more than no swing scored at all — but it is the kind of
    thing a reader has to be told rather than left to infer from two provenance fields in
    different files. It disappears on its own when the two agree again, either by moving the
    setting back or by re-deriving the corpus (`scripts/golfdb/derive_pose_metrics.py
    --estimator`, then `derive_reference.py`).
    """
    corpus = dataset_info().pose_estimator
    if corpus is None:
        return None
    measured_by = [corpus] if isinstance(corpus, str) else list(corpus)
    if estimator in measured_by:
        return None
    return (
        f"this swing was measured by {estimator}, but the benchmark bands were cut from swings "
        f"measured by {' and '.join(measured_by)} — the two instruments do not cancel, so read "
        "the scores as approximate until the corpus is re-derived (ADR-012 §4)"
    )


def _note(notes: list[str] | None, message: str) -> None:
    if notes is not None:
        notes.append(message)
