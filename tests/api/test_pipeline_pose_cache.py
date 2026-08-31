"""The keypoints cache is keyed on the estimator, not on the clip alone.

`settings.pose_model_variant` decides which MediaPipe bundle measures a swing, and the three
bundles do not agree landmark for landmark — they move the phase instants and therefore every
score built on them. `ClipMetadata.source_sha256` cannot notice that: it keys on the *footage*,
and the footage has not changed. So without a second key, moving the setting would leave every
bundle already on disk serving landmarks from the previous variant forever, with nothing in the
result saying so — the silent-disagreement failure `AUDIO_DETECTOR_VERSION` documents one module
over, and `analysis_version` one layer up.

Base install: no pose is ever run here. Every test either reuses a cache or is refused one, and
the clip named in the manifest is deliberately absent from disk — which is also how "this was not
reused" is asserted without a decoder anywhere near it.
"""

from __future__ import annotations

from datetime import UTC, datetime
from pathlib import Path

import pytest

from golf_coach.analysis.benchmarks.distributions import dataset_info
from golf_coach.api.pipeline import _band_estimator_note, keypoints_for
from golf_coach.contracts.keypoints import (
    NUM_POSE_LANDMARKS,
    ClipMetadata,
    FrameKeypoints,
    KeypointsFile,
    Landmark,
)
from golf_coach.pose.estimator import pose_estimator_name
from golf_coach.storage.keypoints_io import save_keypoints
from golf_coach.storage.manifest import Role, RoleFile, SwingManifest

_WHEN = datetime(2026, 8, 30, 1, 0, tzinfo=UTC)
_SHA = "sha-face-on"


def _manifest() -> SwingManifest:
    return SwingManifest(
        swing_id="1",
        session_id="2026-08-30",
        created_at=_WHEN,
        updated_at=_WHEN,
        roles={
            Role.FACE_ON: RoleFile(
                role=Role.FACE_ON,
                filename="face_on.abc.mov",
                content_sha256=_SHA,
                original_filename="face_on.mov",
                content_type="video/quicktime",
                size_bytes=10,
                received_at=_WHEN,
            )
        },
    )


def _cache(swing_dir: Path, estimator: str | None) -> None:
    """One trivial frame, stamped as `estimator` measured it — or as nothing did."""
    save_keypoints(
        KeypointsFile(
            clip=ClipMetadata(fps=100.0, source_sha256=_SHA),
            frames=[
                FrameKeypoints(
                    frame_index=0,
                    timestamp_ms=0.0,
                    landmarks=[
                        Landmark(x=0.5, y=0.5, z=0.0, visibility=1.0)
                        for _ in range(NUM_POSE_LANDMARKS)
                    ],
                    camera_id=Role.FACE_ON.value,
                )
            ],
            pose_estimator=estimator,
        ),
        swing_dir / "face_on.keypoints.json",
    )


def test_a_cache_measured_by_the_running_estimator_is_reused(tmp_path: Path) -> None:
    """The ordinary path: same clip, same instrument, no reason to spend minutes again."""
    _cache(tmp_path, pose_estimator_name())
    notes: list[str] = []

    found = keypoints_for(tmp_path, _manifest(), Role.FACE_ON, notes=notes)

    assert found is not None and len(found.frames) == 1
    assert notes == [], "reusing a matching cache is not worth narrating"


@pytest.mark.parametrize(
    "stamp",
    [
        pytest.param("mediapipe:lite", id="another_variant"),
        # Everything written before the stamp existed. Read as *unknown*, not as lite: guessing
        # would be indistinguishable from knowing, which is the one wrong answer here.
        pytest.param(None, id="unstamped"),
    ],
)
def test_a_cache_from_another_estimator_is_not_reused_silently(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, stamp: str | None
) -> None:
    """It is re-run if it can be, and admitted in a note when it cannot.

    Here it cannot: the clip is not on disk, so this exercises the fallback an archived bundle
    hits after a variant switch. What is pinned is that the landmarks are never handed back as
    though the configured estimator had produced them.
    """
    monkeypatch.setattr("golf_coach.config.settings.pose_model_variant", "heavy")
    _cache(tmp_path, stamp)
    notes: list[str] = []

    found = keypoints_for(tmp_path, _manifest(), Role.FACE_ON, notes=notes)

    assert found is not None, (
        "an archived bundle whose clip is gone lost its only pose run — face-on is the view "
        "every checkpoint is measured from, so that fails the whole bundle"
    )
    assert len(notes) == 1
    assert "mediapipe:heavy" in notes[0]
    assert (stamp or "an unrecorded estimator") in notes[0]


def test_a_cache_for_a_different_clip_still_loses_to_the_sha256(tmp_path: Path) -> None:
    """The original key, unchanged by the new one: a re-uploaded clip invalidates itself.

    Stamped with the running estimator, so only the sha256 can reject it — and it must, or the
    estimator key would have quietly replaced the check it was added beside.
    """
    _cache(tmp_path, pose_estimator_name())
    manifest = _manifest()
    manifest.roles[Role.FACE_ON].content_sha256 = "sha-a-different-take"
    notes: list[str] = []

    found = keypoints_for(tmp_path, manifest, Role.FACE_ON, notes=notes)

    assert found is None, "keypoints for one clip were served for another"
    assert notes == ["the face_on clip is recorded in the manifest but missing from disk"]


def test_a_swing_measured_off_the_corpus_variant_says_so(monkeypatch: pytest.MonkeyPatch) -> None:
    """ADR-012 §4: a band is only comparable to a swing measured the same way.

    The bands in `ranges.json` were cut from GolfDB clips extracted with `mediapipe:lite`, so most
    of that estimator's bias is common-mode with the golfer's and cancels. Running a different
    variant on the golfer's side breaks that, and the result has to admit it — the note is what
    stops "scored 71" being read as though the two sides were still measured alike.
    """
    monkeypatch.setattr("golf_coach.config.settings.pose_model_variant", "heavy")

    note = _band_estimator_note(pose_estimator_name())

    assert note is not None
    assert "mediapipe:heavy" in note and "mediapipe:lite" in note


def test_no_such_note_when_the_two_sides_agree(monkeypatch: pytest.MonkeyPatch) -> None:
    """It has to disappear on its own, or re-deriving the corpus would leave a permanent lie."""
    corpus = dataset_info().pose_estimator
    assert isinstance(corpus, str), "the corpus block names more than one estimator; widen this"
    monkeypatch.setattr(
        "golf_coach.config.settings.pose_model_variant", corpus.removeprefix("mediapipe:")
    )

    assert _band_estimator_note(pose_estimator_name()) is None
