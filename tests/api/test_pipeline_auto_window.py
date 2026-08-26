"""Which landmark each view's swing is *selected* on. [M10 P5]

`engine.analyze_swing_bundle` has segmented the down-the-line view on the trail wrist since
M4 §Phase F — from behind, the lead wrist is the far arm and is tracked in 39% of frames. The
pipeline picked that view's *window* on the lead wrist anyway, and the window is what the
segmentation then runs over, so the frames that got scored were chosen on an arm the anchors never
looked at. On the worst bundle on disk the lead wrist reads a 9.673 s "downswing" and windows the
whole 1996-frame clip; the trail wrist finds the swing at 0.400 s (docs/M10_ALIGNMENT_ACCURACY.md
§A1).

This pins the call site rather than the threading: `analyze_swing_dir` is run for real, with only
`analyze_swing_bundle` replaced by a spy, so what is asserted is the window the analysis would have
been handed.

Base install. Pose is never invoked: `keypoints_for` reuses a cached `*.keypoints.json` whose
`source_sha256` matches the manifest, which is the whole point of that cache and is what keeps this
test off cv2 and MediaPipe.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.api import pipeline
from golf_coach.api.pipeline import PipelineOptions, analyze_swing_dir
from golf_coach.contracts.keypoints import (
    NUM_POSE_LANDMARKS,
    ClipMetadata,
    FrameKeypoints,
    KeypointsFile,
    Landmark,
    PoseLandmark,
)
from golf_coach.contracts.swing import ANALYSIS_VERSION, SwingBundleResult, SwingResult
from golf_coach.storage.keypoints_io import save_keypoints
from golf_coach.storage.manifest import (
    Role,
    RoleFile,
    SwingManifest,
    manifest_path,
    save_manifest,
)

_WHEN = datetime(2026, 8, 25, 1, 0, tzinfo=UTC)

# One frame per 10 ms, so these clips are 100 fps and a frame count reads directly as a duration.
_FPS = 100.0
_ADDRESS_FRAMES = 140
_BACKSWING_FRAMES = 70
_DOWNSWING_FRAMES = 25  # 0.25 s — inside `phases._PLAUSIBLE_DOWNSWING_S`
_FOLLOWTHROUGH_FRAMES = 40
# The clip keeps rolling long after the finish, which is what a bay clip does — and it is what
# makes a window a real choice here rather than "the whole clip, clamped at both ends".
_TRAILING_FRAMES = 200

_ADDRESS_Y, _TOP_Y, _FINISH_Y = 0.60, 0.30, 0.25


def _wrist_track() -> list[float]:
    """One swing as the tracked wrist's `y`: address dwell, backswing, downswing, finish.

    `y` grows downward, so the downswing is the only *rising* stretch — the single candidate
    descent `candidate_downswings` will find.
    """
    def ramp(start: float, end: float, count: int) -> list[float]:
        return [start + (end - start) * (k + 1) / count for k in range(count)]

    return (
        [_ADDRESS_Y] * _ADDRESS_FRAMES
        + ramp(_ADDRESS_Y, _TOP_Y, _BACKSWING_FRAMES)
        + ramp(_TOP_Y, _ADDRESS_Y, _DOWNSWING_FRAMES)
        + ramp(_ADDRESS_Y, _FINISH_Y, _FOLLOWTHROUGH_FRAMES)
        + [_FINISH_Y] * _TRAILING_FRAMES
    )


def _clip(*, visible: PoseLandmark, camera_id: str) -> list[FrameKeypoints]:
    """A clip readable on `visible` and on nothing else.

    Both wrists follow the same track; the other one is dropped below `phases._MIN_VISIBILITY`, so
    selection reading it sees a series held flat at the first confident value and finds no descent
    at all. Blanking a wrist is the only part of a camera angle a synthetic clip can model, and it
    is the part selection depends on — `tests/analysis/test_select_swing.py::_dim_lead_wrist` does
    the same for the same reason.

    Making *each* view readable only on the landmark its call site is supposed to use is not a
    claim about real footage (a face-on camera sees both wrists). It is how one fixture can tell
    two call sites apart: pass the wrong wrist for either view and that view declines.
    """
    frames: list[FrameKeypoints] = []
    for index, y in enumerate(_wrist_track()):
        landmarks = [Landmark(x=0.5, y=0.5, visibility=1.0) for _ in range(NUM_POSE_LANDMARKS)]
        for wrist in (PoseLandmark.LEFT_WRIST, PoseLandmark.RIGHT_WRIST):
            landmarks[wrist] = Landmark(
                x=0.45, y=y, visibility=1.0 if wrist is visible else 0.0
            )
        frames.append(
            FrameKeypoints(
                frame_index=index,
                timestamp_ms=index * (1000.0 / _FPS),
                landmarks=landmarks,
                camera_id=camera_id,
            )
        )
    return frames


@pytest.fixture
def bundle(tmp_path):
    """A swing directory with a manifest and both views' keypoints already cached."""
    swing_dir = tmp_path / "2026-08-25" / "1"
    swing_dir.mkdir(parents=True)

    manifest = SwingManifest(
        swing_id="1",
        session_id="2026-08-25",
        created_at=_WHEN,
        updated_at=_WHEN,
        roles={
            role: RoleFile(
                role=role,
                filename=f"{role.value}.abc.mov",
                content_sha256=f"sha-{role.value}",
                original_filename=f"{role.value}.mov",
                content_type="video/quicktime",
                size_bytes=10,
                received_at=_WHEN,
            )
            for role in (Role.FACE_ON, Role.DOWN_THE_LINE)
        },
    )
    save_manifest(manifest, manifest_path(swing_dir))

    for role, visible in (
        (Role.FACE_ON, PoseLandmark.LEFT_WRIST),
        (Role.DOWN_THE_LINE, PoseLandmark.RIGHT_WRIST),
    ):
        save_keypoints(
            KeypointsFile(
                clip=ClipMetadata(fps=_FPS, source_sha256=f"sha-{role.value}"),
                frames=_clip(visible=visible, camera_id=role.value),
            ),
            swing_dir / f"{role.value}.keypoints.json",
        )
    return swing_dir


@pytest.fixture
def windows(monkeypatch):
    """Capture the windows the pipeline hands the analysis, and score nothing."""
    captured: dict[str, object] = {}

    def spy(**kwargs) -> SwingBundleResult:
        captured.update(kwargs)
        return SwingBundleResult(
            swing_id=kwargs["swing_id"],
            session_id=kwargs["session_id"],
            swing=SwingResult(
                swing_id=kwargs["swing_id"],
                session_id=kwargs["session_id"],
                overall_score=0.0,
            ),
            analysis_version=ANALYSIS_VERSION,
        )

    monkeypatch.setattr(pipeline, "analyze_swing_bundle", spy)
    return captured


def test_the_down_the_line_view_is_windowed_on_the_trail_wrist(bundle, windows) -> None:
    """The window and the anchors read one landmark, or they describe two different swings."""
    outcome = analyze_swing_dir(
        bundle, options=PipelineOptions(render_video=False, coaching=False)
    )

    assert outcome.result is not None
    window = windows["down_the_line_window"]
    assert window is not None, "the trail wrist can see this descent; the lead wrist cannot"

    top = _ADDRESS_FRAMES + _BACKSWING_FRAMES
    start, end = window
    assert start < top and end > top + _DOWNSWING_FRAMES, "the window must hold the swing"
    # A strict interior slice at both ends: the failure this pins is a "window" that is really the
    # whole clip, which scores every motion in it and calls the result one swing.
    assert start > 0 and end < len(_wrist_track())
    assert not any(
        "down-the-line" in note and "could not pick" in note for note in outcome.result.notes
    )


def test_the_face_on_view_keeps_the_lead_wrist(bundle, windows) -> None:
    """Face-on is the view every band was measured on and every stored window was picked with.

    Pinned in the same run as the trail-wrist case rather than by asserting on a default, because
    the two views are threaded separately and the mistake worth catching is one call site being
    changed to match the other.
    """
    analyze_swing_dir(bundle, options=PipelineOptions(render_video=False, coaching=False))

    assert windows["face_on_window"] is not None
