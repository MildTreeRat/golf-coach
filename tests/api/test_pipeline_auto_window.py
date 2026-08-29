"""How the pipeline picks each view's swing: which landmark, and in which order. [M10 P5, P8]

Two failures live here, and they are the same failure at different distances from the camera.

**Which landmark.** `engine.analyze_swing_bundle` has segmented the down-the-line view on the trail
wrist since M4 §Phase F — from behind, the lead wrist is the far arm and is tracked in 39% of
frames. The pipeline picked that view's *window* on the lead wrist anyway, and the window is what
the segmentation then runs over, so the frames that got scored were chosen on an arm the anchors
never looked at. On the worst bundle on disk the lead wrist reads a 9.673 s "downswing" and windows
the whole 1996-frame clip; the trail wrist finds the swing at 0.400 s (§A1).

**In which order.** Judged alone, a down-the-line clip's swing is "the last plausible descent" —
and the phone on the busy side of the bay keeps rolling 15-24 s past impact, so that is routinely a
move made after the ball was gone. Six of the fifteen stored bundles were windowed on one (§A2).
P8 picks face-on first and matches the other view to its downswing duration, and matches *back*
when face-on is the view that declined.

These pin the call sites rather than the threading: `analyze_swing_dir` is run for real, with only
`analyze_swing_bundle` replaced by a spy, so what is asserted is the window the analysis would have
been handed.

Base install. Pose is never invoked: `keypoints_for` reuses a cached `*.keypoints.json` whose
`source_sha256` matches the manifest, which is the whole point of that cache and is what keeps this
test off cv2 and MediaPipe.
"""

from __future__ import annotations

from collections.abc import Sequence
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

# One frame per 10 ms, so these clips are 100 fps and a frame count reads directly as a duration:
# `phases._PLAUSIBLE_DOWNSWING_S` is 15-45 frames here and `_MATCH_TOLERANCE_S` is 12.
_FPS = 100.0
_ADDRESS_FRAMES = 260
_RESET_FRAMES = 60  # lowering the hands from the finish back to address, between swings
_DWELL_FRAMES = 200  # between one descent and the next, so their windows cannot overlap
_FOLLOWTHROUGH_FRAMES = 40
# The clip keeps rolling long after the finish, which is what a bay clip does — and it is what
# makes a window a real choice here rather than "the whole clip, clamped at both ends".
_TRAILING_FRAMES = 200

_ADDRESS_Y, _TOP_Y, _FINISH_Y = 0.60, 0.30, 0.25

#: `(backswing, downswing)` in frames. The default swing: a 0.25 s descent, mid-band.
_SWING = (70, 25)

# A `Swings` is one clip's swings in order. Every downswing falls the same distance (_TOP_Y to
# _ADDRESS_Y), so `phases.CANDIDATE_MIN_RISE` keeps all of them and duration is the only thing
# selection has to judge on — which is the rule under test, isolated.
Swings = Sequence[tuple[int, int]]


def _wrist_track(swings: Swings) -> list[float]:
    """One clip as the tracked wrist's `y`: address, then each swing, then a long dead finish.

    `y` grows downward, so a downswing is a *rising* stretch and the follow-through that ends it is
    what closes the run — without one the descent would merge into the flat dwell that follows and
    measure whole seconds. Lowering the hands back to address between swings is a rising stretch
    too, which is honest: it is the "setup move" `select_swing`'s docstring names as the thing
    duration exists to reject, and here it is slow enough to be rejected the same way.
    """
    def ramp(start: float, end: float, count: int) -> list[float]:
        return [start + (end - start) * (k + 1) / count for k in range(count)]

    track: list[float] = []
    for index, (backswing, downswing) in enumerate(swings):
        if index == 0:
            track += [_ADDRESS_Y] * _ADDRESS_FRAMES
        else:
            track += ramp(_FINISH_Y, _ADDRESS_Y, _RESET_FRAMES) + [_ADDRESS_Y] * _DWELL_FRAMES
        track += ramp(_ADDRESS_Y, _TOP_Y, backswing)
        track += ramp(_TOP_Y, _ADDRESS_Y, downswing)
        track += ramp(_ADDRESS_Y, _FINISH_Y, _FOLLOWTHROUGH_FRAMES)
    return track + [_FINISH_Y] * _TRAILING_FRAMES


def _descents(swings: Swings) -> list[tuple[int, int]]:
    """Each swing's `(top, impact)` frame, from the same arithmetic `_wrist_track` builds with.

    Derived rather than written down: a test that hard-codes 330 and 355 stops meaning anything the
    first time a constant above changes.
    """
    at = 0
    found: list[tuple[int, int]] = []
    for index, (backswing, downswing) in enumerate(swings):
        at += _ADDRESS_FRAMES if index == 0 else _RESET_FRAMES + _DWELL_FRAMES
        top = at + backswing
        impact = top + downswing
        found.append((top, impact))
        at = impact + _FOLLOWTHROUGH_FRAMES
    return found


def _clip(swings: Swings, *, visible: PoseLandmark, camera_id: str) -> list[FrameKeypoints]:
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
    for index, y in enumerate(_wrist_track(swings)):
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
    """Build a swing directory with a manifest and both views' keypoints already cached.

    A factory rather than a fixed directory because what each test varies is the *descents in each
    clip* — one swing, a swing plus a post-impact decoy, a clip with nothing plausible in it.
    """
    def build(
        *, face_on: Swings = (_SWING,), down_the_line: Swings = (_SWING,)
    ) -> object:
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

        for role, visible, swings in (
            (Role.FACE_ON, PoseLandmark.LEFT_WRIST, face_on),
            (Role.DOWN_THE_LINE, PoseLandmark.RIGHT_WRIST, down_the_line),
        ):
            save_keypoints(
                KeypointsFile(
                    clip=ClipMetadata(fps=_FPS, source_sha256=f"sha-{role.value}"),
                    frames=_clip(swings, visible=visible, camera_id=role.value),
                ),
                swing_dir / f"{role.value}.keypoints.json",
            )
        return swing_dir

    return build


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


def _run(swing_dir) -> SwingBundleResult:
    outcome = analyze_swing_dir(
        swing_dir, options=PipelineOptions(render_video=False, coaching=False)
    )
    assert outcome.result is not None
    return outcome.result


def test_the_down_the_line_view_is_windowed_on_the_trail_wrist(bundle, windows) -> None:
    """The window and the anchors read one landmark, or they describe two different swings."""
    result = _run(bundle())

    window = windows["down_the_line_window"]
    assert window is not None, "the trail wrist can see this descent; the lead wrist cannot"

    top, impact = _descents((_SWING,))[0]
    start, end = window
    assert start < top and end > impact, "the window must hold the swing"
    # A strict interior slice at both ends: the failure this pins is a "window" that is really the
    # whole clip, which scores every motion in it and calls the result one swing.
    assert start > 0 and end < len(_wrist_track((_SWING,)))
    assert not any(
        "down-the-line" in note and "could not pick" in note for note in result.notes
    )


def test_the_face_on_view_keeps_the_lead_wrist(bundle, windows) -> None:
    """Face-on is the view every band was measured on and every stored window was picked with.

    Pinned in the same run as the trail-wrist case rather than by asserting on a default, because
    the two views are threaded separately and the mistake worth catching is one call site being
    changed to match the other.
    """
    _run(bundle())

    assert windows["face_on_window"] is not None


def test_the_down_the_line_view_matches_face_on_rather_than_taking_the_last(
    bundle, windows
) -> None:
    """The six-bundle bug, in a fixture: a decoy descent *after* impact, and plausible.

    Judged alone this clip's swing is the decoy — `select_swing` takes the last plausible descent,
    which is right on a clip that stops when the swing does and wrong on a down-the-line clip that
    keeps rolling. Matched against face-on's 0.25 s it is the real one, by 0.07 s.
    """
    decoy = (30, 18)  # 0.18 s: comfortably inside the band, so only the reference rejects it
    result = _run(bundle(down_the_line=(_SWING, decoy)))

    (top, impact), (decoy_top, _) = _descents((_SWING, decoy))
    start, end = windows["down_the_line_window"]
    assert start < top and impact < end, "the window must hold the real swing"
    assert end < decoy_top, "and stop before the descent that happened after the ball was gone"
    assert not any("could not pick" in note for note in result.notes)


def test_face_on_is_matched_back_when_it_is_the_view_that_declined(bundle, windows) -> None:
    """The mutual half of P8, and the one that moves scores rather than only the render.

    This face-on clip is `2026-08-23/8`'s shape: a real descent just outside
    `_PLAUSIBLE_DOWNSWING_S` and a second, implausible candidate — so the band declines it and the
    lone-candidate escape cannot fire either. Face-on is the view every checkpoint is measured
    from, so declining costs the whole clip's frames. The down-the-line view is confident within
    `_MATCH_TOLERANCE_S` of it, and matching in reverse recovers it.
    """
    real, rehearsal = (70, 47), (60, 120)  # 0.47 s, 0.05 s over the band; and 1.20 s
    result = _run(bundle(face_on=(real, rehearsal), down_the_line=((70, 40),)))

    top, impact = _descents((real, rehearsal))[0]
    start, end = windows["face_on_window"]
    assert start < top and impact < end, "the descent the other camera vouched for"
    assert start > 0 and end < len(_wrist_track((real, rehearsal)))
    assert not any("could not pick a swing in the face-on view" in n for n in result.notes)
    # The weaker basis is on the record: this window was not chosen from its own clip.
    assert any("matching the down-the-line view" in note for note in result.notes)


def test_with_no_face_on_reference_the_other_view_is_picked_alone_and_says_so(
    bundle, windows
) -> None:
    """Nothing to match against is not a licence to guess, but it is worth writing down.

    Face-on holds two implausible descents, so it declines and stays declined — the down-the-line
    pick is 0.95 s away and cannot rescue it. That leaves the down-the-line window chosen by the
    rule P8 exists to stop relying on, which the notes have to say, because a reader of
    `analysis.json` cannot otherwise tell a cross-checked window from an uncross-checked one.
    """
    result = _run(bundle(face_on=((60, 120), (60, 130)), down_the_line=(_SWING,)))

    assert windows["face_on_window"] is None
    assert windows["down_the_line_window"] is not None
    assert any("could not pick a swing in the face-on view" in n for n in result.notes)
    assert any("picked from that clip alone" in note for note in result.notes)
