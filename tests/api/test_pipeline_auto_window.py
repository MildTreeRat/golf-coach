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

from golf_coach.analysis.phases import _STRIKE_TOLERANCE_S
from golf_coach.api import pipeline
from golf_coach.api.pipeline import PipelineOptions, analyze_swing_dir
from golf_coach.contracts.audio import (
    AUDIO_DETECTOR_VERSION,
    AudioClipMetadata,
    AudioFile,
    AudioStrike,
)
from golf_coach.contracts.keypoints import (
    NUM_POSE_LANDMARKS,
    ClipMetadata,
    FrameKeypoints,
    KeypointsFile,
    Landmark,
    PoseLandmark,
)
from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.contracts.swing import ANALYSIS_VERSION, SwingBundleResult, SwingResult
from golf_coach.launch_monitor.screen.store import ShotStore
from golf_coach.storage.audio_io import save_audio
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
# `phases._POSSIBLE_DOWNSWING_S` is 12-80 frames here and `_MATCH_TOLERANCE_S` is 12.
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


# The corpus's rate (M11 §E1), so the sample indices these fixtures write are the size the real
# ones are — 480 samples to a frame at 100 fps.
_AUDIO_RATE = 48_000


def _shots_dir(swing_dir):
    """Where a bundle's shot store lives: beside the session directory, not inside it."""
    return swing_dir.parents[1] / "shots"


def _heard(frames: Sequence[int], *, role: Role) -> AudioFile:
    """A cached `{role}.audio.json` with one strike on each of `frames`.

    Written with **sample indices and no fps**, which is the artifact `audio_for` produces when
    detection ran without knowing the video's frame rate (`contracts/audio.py`). So the frame each
    test below reasons about is one the pipeline derived on the way through, not one the fixture
    asserted into place — and the derivation is part of what these tests cover.

    An empty `frames` is not an empty fixture: it is the file a rehearsal swing produces, and the
    one M11 P5 exists to read.

    Stamped with the **current** detector version, which is what keeps these tests off ffmpeg: an
    artifact from an older one is re-detected rather than read (`contracts/audio.py`), and there is
    nothing here to decode — the clips on disk are empty stubs.
    """
    return AudioFile(
        detector_version=AUDIO_DETECTOR_VERSION,
        clip=AudioClipMetadata(
            sample_rate=_AUDIO_RATE,
            duration_s=30.0,
            source_sha256=f"sha-{role.value}",
            stream_index=0,
        ),
        strikes=[
            AudioStrike(
                sample=round(frame * _AUDIO_RATE / _FPS),
                confidence=0.95,
                prominence=5000.0,
            )
            for frame in frames
        ],
    )


@pytest.fixture
def bundle(tmp_path):
    """Build a swing directory with a manifest and both views' keypoints already cached.

    A factory rather than a fixed directory because what each test varies is the *descents in each
    clip* — one swing, a swing plus a post-impact decoy, a clip with nothing plausible in it.
    """
    def build(
        *,
        face_on: Swings = (_SWING,),
        down_the_line: Swings = (_SWING,),
        strikes: dict[Role, Sequence[int]] | None = None,
        shot: bool = False,
    ) -> object:
        swing_dir = tmp_path / "2026-08-25" / "1"
        swing_dir.mkdir(parents=True)

        roles = [Role.FACE_ON, Role.DOWN_THE_LINE] + ([Role.SHOT_SCREEN] if shot else [])
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
                for role in roles
            },
        )
        save_manifest(manifest, manifest_path(swing_dir))

        if shot:
            # Put straight into the store rather than run OCR: `_shot_for` reads the cache first,
            # keyed on the photo's own sha256, which is the base-install path and needs no `ocr`
            # extra and no image on disk.
            ShotStore(_shots_dir(swing_dir)).put(
                ShotData(
                    shot_id="2026-08-25-1",
                    session_id="2026-08-25",
                    timestamp=_WHEN,
                    source=ShotSource.SCREEN,
                    carry_distance=140.0,
                    provenance=ShotProvenance(
                        device="hd_golf",
                        parse_confidence=0.95,
                        needs_review=False,
                        image_sha256=f"sha-{Role.SHOT_SCREEN.value}",
                    ),
                )
            )

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
            if strikes is not None and role in strikes:
                save_audio(
                    _heard(strikes[role], role=role),
                    swing_dir / f"{role.value}.audio.json",
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


def _run(swing_dir, *, log=None, **overrides) -> SwingBundleResult:
    outcome = analyze_swing_dir(
        swing_dir,
        options=PipelineOptions(render_video=False, coaching=False, **overrides),
        **({"log": log} if log is not None else {}),
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

    This face-on clip is `2026-08-23/8`'s shape: a descent the band will not admit and a second,
    longer candidate — so face-on declines and the lone-candidate escape cannot fire either, there
    being two of them. Face-on is the view every checkpoint is measured from, so declining costs
    the whole clip's frames. The down-the-line view is confident within `_MATCH_TOLERANCE_S` of it,
    and matching in reverse recovers it.

    **The real descent misses the band underneath rather than above it**, which is the shape M10's
    defect actually takes: `_DRAWDOWN_FLOOR`'s fragmenting run reads a descent *short*, and face-on
    is the view it happens to (0.183-0.267 s on the stored corpus). Overshooting the ceiling would
    do here too, but only at durations long enough that `_WINDOW_LEAD` reaches past the start of a
    synthetic clip, which would test the fixture's length rather than the rule.
    """
    real, rehearsal = (70, 7), (60, 130)  # 0.11 s, a fragment under the band; and 1.28 s
    result = _run(bundle(face_on=(real, rehearsal), down_the_line=((70, 18),)))

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


# The acoustic evidence (M11 P5). These pin the *threading* — that each view is handed the strikes
# heard in its own clip and that the artifact reaches selection at all — rather than the rule,
# which `tests/analysis/test_select_swing.py` covers directly.
#
# No decoding happens here and none may: every `{role}.audio.json` these tests write already
# matches the manifest's sha256, which is the same cache contract that keeps this file off cv2 and
# MediaPipe. A test that started an ffmpeg subprocess would have gone wrong somewhere.
_OUT_OF_BAND = (70, 90)  # 0.90 s — a descent `phases._POSSIBLE_DOWNSWING_S` will not admit
_IN_BAND_DECOY = (60, 18)  # 0.18 s — comfortably in band, and after the ball was gone


def test_a_ball_strike_wins_the_window_the_duration_band_would_have_lost(
    bundle, windows
) -> None:
    """The failure this phase exists to fix, run through the whole pipeline.

    Face-on holds a real swing just past the band and a shorter descent inside it, which is
    `2026-08-23/8`'s shape. Judged on duration alone the decoy wins and every checkpoint is then
    measured on a move made after the ball was gone. One strike, at the real swing's impact,
    reverses it — and it does so on the view that matters most, since face-on is where every
    checkpoint is measured.
    """
    swings = (_OUT_OF_BAND, _IN_BAND_DECOY)
    (top, impact), (decoy_top, _) = _descents(swings)

    lines: list[str] = []
    result = _run(
        bundle(face_on=swings, strikes={Role.FACE_ON: [impact]}), log=lines.append
    )

    start, end = windows["face_on_window"]
    assert start < top and impact < end, "the window must hold the descent the ball was struck on"
    assert end < decoy_top, "and stop before the descent the band preferred"
    assert not any("could not pick a swing in the face-on view" in n for n in result.notes)
    # `_narrate_choice` says when a strike decided it, through the reason the rule wrote.
    assert any("ends at a ball strike" in line for line in lines)


def test_without_the_strike_that_same_clip_is_windowed_on_the_decoy(bundle, windows) -> None:
    """The other half of the pin above: without audio the fixture really does go wrong.

    Worth its own test rather than an assertion inside the one above, because a fixture that
    happened to pick the right descent either way would make that test pass while proving nothing.
    """
    swings = (_OUT_OF_BAND, _IN_BAND_DECOY)
    (_top, impact), (decoy_top, decoy_impact) = _descents(swings)

    _run(bundle(face_on=swings))

    start, end = windows["face_on_window"]
    assert start > impact, "without a strike the band takes the later, shorter descent"
    assert start < decoy_top and decoy_impact < end


def test_each_view_is_judged_on_the_strikes_heard_in_its_own_clip(bundle, windows) -> None:
    """Two phones, two clocks, and no offset between them until M11 P6.

    The down-the-line clip here holds the same trap in the same shape, and its own strike lands at
    its own impact frame. Handing it the face-on view's strike frames instead would window it on
    nothing — which is the mistake this asserts against, by giving the two clips descents at
    visibly different frames and a correct answer in both.
    """
    face_on = (_OUT_OF_BAND, _IN_BAND_DECOY)
    dtl = ((120, 90), _IN_BAND_DECOY)  # a longer backswing, so its impact sits elsewhere
    (face_top, face_impact), _ = _descents(face_on)
    (dtl_top, dtl_impact), _ = _descents(dtl)
    # Farther apart than the rule's own tolerance, or one view's strikes would vouch for the
    # other's descent and the test would pass however the two were threaded.
    assert abs(face_impact - dtl_impact) > _STRIKE_TOLERANCE_S * _FPS

    _run(
        bundle(
            face_on=face_on,
            down_the_line=dtl,
            strikes={Role.FACE_ON: [face_impact], Role.DOWN_THE_LINE: [dtl_impact]},
        )
    )

    face_start, face_end = windows["face_on_window"]
    dtl_start, dtl_end = windows["down_the_line_window"]
    assert face_start < face_top and face_impact < face_end
    assert dtl_start < dtl_top and dtl_impact < dtl_end


def test_a_clip_that_made_no_crack_keeps_its_window_and_says_so(bundle, windows) -> None:
    """Silence is a finding, not a decline.

    A rehearsal makes no crack — that is the whole premise — but an empty strike list can also mean
    the phone was across the bay. Throwing the window away over it would score every motion in the
    clip as one swing, which is strictly worse than scoring the right descent with a caveat. So the
    window stands and the note carries the doubt (ADR-010 §2).
    """
    result = _run(bundle(strikes={Role.FACE_ON: []}))

    assert windows["face_on_window"] is not None
    assert any(
        "no ball strike was heard in the face-on clip" in note for note in result.notes
    )
    assert not any("down-the-line clip" in note for note in result.notes), (
        "a view with no audio artifact at all has nothing to report"
    )


def test_a_recorded_shot_with_no_crack_in_either_clip_is_a_note(bundle, windows) -> None:
    """Two independent witnesses disagreeing: the simulator saw a ball, the microphones did not.

    §E1 measured every real strike at z = 113-599 above its own clip's floor, so silence in both
    views while the launch monitor recorded a carry most likely means this bundle's footage and its
    shot are not the same swing. A note and not an error — the shot still scores, and only a human
    can confirm a mis-pairing.
    """
    swing_dir = bundle(strikes={Role.FACE_ON: [], Role.DOWN_THE_LINE: []}, shot=True)
    result = _run(swing_dir, shots_dir=_shots_dir(swing_dir))

    assert any("may not describe the same swing" in note for note in result.notes)


def test_a_crack_in_one_clip_is_enough_to_believe_the_shot(bundle, windows) -> None:
    """The other side of that check: one view hearing the ball settles it for the bundle.

    The down-the-line phone sits on the busy side of the bay and is the one that can miss the
    strike. Requiring both would fire this note on ordinary swings, which is how a note stops being
    read.
    """
    impact = _descents((_SWING,))[0][1]
    swing_dir = bundle(
        strikes={Role.FACE_ON: [impact], Role.DOWN_THE_LINE: []}, shot=True
    )
    result = _run(swing_dir, shots_dir=_shots_dir(swing_dir))

    assert not any("may not describe the same swing" in note for note in result.notes)


# --- the strikes reach the alignment too, not only the selection ------------------------ [M11 P6]


def test_the_strikes_are_handed_to_the_analysis_per_view(bundle, windows) -> None:
    """Each view's own frame numbering, kept apart all the way to `analyze_swing_bundle`.

    Two phones, two clocks: the down-the-line strike below is 9 frames from the face-on one for the
    same event, and `with_measured_impact` reads each against the anchors of the clip it came from.
    Crossing them here is the one mistake that would be invisible downstream, because both numbers
    are plausible impacts.
    """
    _top, impact = _descents((_SWING,))[0]

    _run(
        bundle(strikes={Role.FACE_ON: [impact], Role.DOWN_THE_LINE: [impact + 9]})
    )

    assert windows["face_on_strikes"] == [impact]
    assert windows["down_the_line_strikes"] == [impact + 9]


def test_a_hand_picked_window_still_gets_listened_to(bundle, windows) -> None:
    """The hoist M11 P5 asked for: `auto_window=False` used to skip the microphone entirely.

    Selection is no longer the only reader. A window given by hand has answered *which* descent to
    score and says nothing about *when* the ball was struck, so the alignment still wants the
    strike — and a run that had to be windowed by hand is if anything the one most likely to need
    a measured anchor.
    """
    _top, impact = _descents((_SWING,))[0]

    _run(
        bundle(strikes={Role.FACE_ON: [impact]}),
        auto_window=False,
        window_face_on=(0, impact + 30),
    )

    assert windows["face_on_window"] == (0, impact + 30)
    assert windows["face_on_strikes"] == [impact]


def test_a_view_that_was_never_listened_to_hands_over_nothing(bundle, windows) -> None:
    """No `{role}.audio.json` and no decoder is `None`, never an empty list.

    The distinction is the one `contracts/audio.py` insists on: `[]` means the detector ran and
    heard nothing, which is a finding, and `None` means nobody listened. Both leave the anchor
    inferred, but only one of them is worth a note.
    """
    _run(bundle())

    assert windows["face_on_strikes"] is None
    assert windows["down_the_line_strikes"] is None
