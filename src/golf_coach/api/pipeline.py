"""One assembled swing bundle in, the complete result out. [M7 Phase 5]

This is the orchestration ADR-008 always meant to live in `api/` — "the imperative shell", the
one place allowed to do I/O around the pure analysis core. It ran inside
`scripts/analyze_bundle.py` until the background worker needed it too, and two copies of a
pipeline is how they drift.

Pose per view, OCR the shot screen, score, rank tips, align the two views; four artifacts land
in the swing directory:

    analysis.json           the complete result, for a results page to render
    analysis.state.json     the sidecar summary of this run — written *here* so it can never
                            quote a score the analysis beside it has stopped agreeing with
    aligned.mp4             the two views side by side, banners landing together
    <role>.keypoints.json   pose output per view, cached so a re-run is free

**Everything expensive is cached and content-addressed.** Pose on a 4K60 clip is minutes, so
keypoints are reused whenever their recorded `source_sha256` still matches the manifest's, and
the shot is looked up in the shot store by the photo's hash before any OCR is considered — which
means an already-imported shot attaches with no `ocr` extra installed at all.

Degradation is graceful and always narrated: a bundle with no down-the-line clip still scores and
still writes JSON, it just has no alignment and no video. Only a missing face-on clip is fatal —
that is the view every checkpoint is measured from.

Two rules this module exists to keep:

**It must not import fastapi.** `scripts/analyze_bundle.py` imports it and runs on a `vision`-only
install; dragging the web framework in here would break the extras boundary ADR-008 draws.
`tests/api/test_pipeline_imports.py` pins that.

**Nothing is narrated only to a terminal.** The CLI passes `log=print`; the worker passes a
logger, and nobody is watching. So anything a reader of the *result* would need to know — a clip
that decoded short, a shot that could not be read, a swing the selector declined to pick — is
appended to `result.notes`, which is part of `analysis.json`. `log` is for progress; `notes` are
for truth.

Needs the `vision` extra to run pose or render, and `ocr` only to read a shot photo that is not
already in the store.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

from golf_coach.analysis.alignment import DEFAULT_TAU_RANGE, pair_frames
from golf_coach.analysis.engine import analyze_swing_bundle
from golf_coach.analysis.phases import select_swing
from golf_coach.analysis.smoothing import smooth_keypoints
from golf_coach.api.state import AnalysisState, input_hashes, load_state, now, save_state
from golf_coach.config import settings
from golf_coach.contracts.golfer import Handedness
from golf_coach.contracts.intent import ClubCategory, PracticeGoal
from golf_coach.contracts.keypoints import ClipMetadata, KeypointsFile
from golf_coach.contracts.shot import ShotData
from golf_coach.contracts.swing import SwingBundleResult
from golf_coach.feedback.coach import generate_coaching
from golf_coach.feedback.rules import build_feedback
from golf_coach.launch_monitor.screen.store import ShotStore
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
    """
    role_file = manifest.roles.get(role)
    if role_file is None:
        return None

    cache_path = swing_dir / f"{role.value}.keypoints.json"
    if cache_path.exists() and not force:
        cached = load_keypoints(cache_path)
        if cached.clip is not None and cached.clip.source_sha256 == role_file.content_sha256:
            log(f"  {role.value}: {len(cached.frames)} frames from cache")
            return cached
        log(f"  {role.value}: cached keypoints are for a different clip, re-running pose")

    video_path = swing_dir / role_file.filename
    if not video_path.exists():
        log(f"  {role.value}: {video_path.name} is missing from the swing directory")
        _note(notes, f"the {role.value} clip is recorded in the manifest but missing from disk")
        return None

    try:
        from golf_coach.capture.file import FileVideoSource
        from golf_coach.pose.estimator import estimate_pose
    except ImportError:
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
    )
    save_keypoints(keypoints, cache_path)
    log(f"  {role.value}: {len(frames)} frames -> {cache_path.name}")
    return keypoints


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


def _auto_window(
    label: str, keypoints: KeypointsFile, *, log: Log, notes: list[str]
) -> tuple[int, int] | None:
    """`select_swing`'s pick for one view, narrating what it chose or why it declined.

    A decline is a note, not just a log line: scoring the whole clip when it holds practice
    swings produces numbers that look fine and describe the wrong motion.
    """
    fps = keypoints.clip.fps if keypoints.clip else None
    choice = select_swing(smooth_keypoints(keypoints.frames), fps=fps)
    if choice is None:
        why = "the keypoints file records no fps" if fps is None else "no plausible downswing"
        log(f"  {label}: could not pick a swing ({why}) — using the whole clip. Run "
            "--list-swings and pass a window if this clip holds more than one swing")
        notes.append(
            f"could not pick a swing in the {label} view ({why}) — the whole clip was scored, "
            "so these numbers describe every motion in it, not one swing"
        )
        return None
    log(f"  {label}: {choice.reason}")
    return choice.window


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

    Any existing state is read first so the worker's `queued_at` survives a run it did not start.
    Everything else describes this run.
    """
    previous = load_state(swing_dir)
    missing = [role.value for role in manifest.missing_roles()]
    completed = now()
    result = outcome.result

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
        video=outcome.video_path.name if outcome.video_path else None,
        video_codec=outcome.video_codec,
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
    if options.auto_window:
        log("\nSwing selection:")
        if window_face_on is None:
            window_face_on = _auto_window("face-on", views[Role.FACE_ON], log=log, notes=notes)
        if window_dtl is None and Role.DOWN_THE_LINE in views:
            window_dtl = _auto_window(
                "down-the-line", views[Role.DOWN_THE_LINE], log=log, notes=notes
            )

    log("\nShot data:")
    shot, shot_note = _shot_for(swing_dir, manifest, options=options, log=log)
    if shot_note:
        log(f"  {shot_note}")
        notes.append(shot_note)

    handedness, handedness_note = _handedness_for(manifest)
    if handedness_note:
        log(f"\nGolfer:\n  {handedness_note}")
        notes.append(handedness_note)

    result = analyze_swing_bundle(
        swing_id=manifest.swing_id,
        session_id=manifest.session_id,
        face_on=views[Role.FACE_ON],
        down_the_line=views.get(Role.DOWN_THE_LINE),
        shot=shot,
        intent=PracticeGoal(club=options.club),
        face_on_window=window_face_on,
        down_the_line_window=window_dtl,
        handedness=handedness,
    )

    video_path: Path | None = None
    video_codec: str | None = None
    if options.render_video and Role.DOWN_THE_LINE in views:
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
        missing_roles=missing,
        flagged=bool(needs_review or missing),
    )
    record_state(swing_dir, manifest, outcome, started_at=started_at)
    return outcome


def _note(notes: list[str] | None, message: str) -> None:
    if notes is not None:
        notes.append(message)
