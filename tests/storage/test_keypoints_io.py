"""Keypoints file I/O — above all, that the legacy bare-array shape still loads.

The backward-compatibility tests here stand in for files this suite cannot ship: hundreds of
`data/processed/*.keypoints.json` and the 461-clip GolfDB reference cache under
`data/reference/golfdb/keypoints/`, all gitignored, all written before the clip-metadata envelope
existed, and the cache in particular representing hours of estimator time nobody is going to spend
again. Their exact on-disk shapes are therefore reproduced here as **literal JSON strings** rather
than by dumping the current models — a fixture built from the models would happily follow them
wherever they drift, which is precisely the regression these tests exist to catch.

Pydantic only; no OpenCV, no MediaPipe. Runs on the base install (ADR-008).
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from golf_coach.contracts.keypoints import (
    NUM_POSE_LANDMARKS,
    ClipMetadata,
    FrameKeypoints,
    KeypointsFile,
    Landmark,
    PoseLandmark,
)
from golf_coach.storage.keypoints_io import load_keypoints, save_keypoints


def _landmarks_json(*, compact: bool) -> str:
    """33 landmarks as either extract_pose.py writes them (rounded) or run_pose.py does."""
    lm = (
        '{"x":0.51234,"y":0.62345,"z":-0.1,"visibility":0.9876}'
        if compact
        else '{"x": 0.51234, "y": 0.62345, "z": -0.1, "visibility": 0.9876}'
    )
    return ",".join([lm] * NUM_POSE_LANDMARKS)


def _legacy_compact_json(frames: int = 3) -> str:
    """The GolfDB reference-cache shape: a bare array, compact separators, rounded coordinates."""
    return "[" + ",".join(
        f'{{"frame_index":{i},"timestamp_ms":{round(i / 30 * 1000, 3)},'
        f'"landmarks":[{_landmarks_json(compact=True)}]}}'
        for i in range(frames)
    ) + "]"


def _legacy_indented_json(frames: int = 3) -> str:
    """The `data/processed/*.keypoints.json` shape: a bare array, indented, full precision."""
    return json.dumps(
        json.loads(_legacy_compact_json(frames)),
        indent=2,
    )


def _keypoints(frames: int = 3, camera_id: str | None = None) -> list[FrameKeypoints]:
    return [
        FrameKeypoints(
            frame_index=i,
            timestamp_ms=i / 30 * 1000,
            landmarks=[
                Landmark(x=0.5, y=0.6, z=-0.1, visibility=0.9) for _ in range(NUM_POSE_LANDMARKS)
            ],
            camera_id=camera_id,
        )
        for i in range(frames)
    ]


# --- backward compatibility: the whole reason this module exists ---------------------------


def test_legacy_reference_cache_shape_still_loads(tmp_path: Path) -> None:
    path = tmp_path / "1003.keypoints.json"
    path.write_text(_legacy_compact_json(frames=4), encoding="utf-8")

    loaded = load_keypoints(path)

    # `clip is None` is the honest answer, not a gap: these files never recorded a source clip.
    assert loaded.clip is None
    assert len(loaded.frames) == 4
    assert [f.frame_index for f in loaded.frames] == [0, 1, 2, 3]
    assert all(f.camera_id is None for f in loaded.frames)
    assert loaded.frames[0].landmark(PoseLandmark.LEFT_WRIST).x == pytest.approx(0.51234)


def test_legacy_processed_shape_still_loads(tmp_path: Path) -> None:
    path = tmp_path / "aaron-swing-2.keypoints.json"
    path.write_text(_legacy_indented_json(frames=3), encoding="utf-8")

    loaded = load_keypoints(path)

    assert loaded.clip is None
    assert len(loaded.frames) == 3
    assert len(loaded.frames[0].landmarks) == NUM_POSE_LANDMARKS


def test_reference_cache_on_disk_loads_if_present() -> None:
    """Load the real GolfDB cache when this checkout has one — the actual regression risk.

    Skipped in a clean checkout, since `data/` is gitignored. When the cache *is* there, this is
    the only test that exercises files nothing in this repo produced under the current code.
    """
    from golf_coach.config import REPO_ROOT

    cache = REPO_ROOT / "data" / "reference" / "golfdb" / "keypoints"
    if not cache.is_dir():
        pytest.skip("no local GolfDB reference cache")
    clips = sorted(cache.glob("*/*.keypoints.json"))[:25]
    if not clips:
        pytest.skip("GolfDB reference cache is empty")

    for clip in clips:
        loaded = load_keypoints(clip)
        assert loaded.frames, f"{clip} loaded no frames"
        assert len(loaded.frames[0].landmarks) == NUM_POSE_LANDMARKS


# --- the enveloped shape --------------------------------------------------------------------


def test_envelope_round_trips_with_clip_metadata(tmp_path: Path) -> None:
    path = tmp_path / "swing.keypoints.json"
    original = KeypointsFile(
        clip=ClipMetadata(
            fps=58.913,
            width=480,
            height=854,
            frame_count=3,
            source_sha256="a" * 64,
        ),
        frames=_keypoints(camera_id="face_on"),
    )

    save_keypoints(original, path)
    loaded = load_keypoints(path)

    assert loaded.clip is not None
    assert loaded.clip.fps == pytest.approx(58.913)
    assert (loaded.clip.width, loaded.clip.height) == (480, 854)
    assert loaded.clip.frame_count == 3
    assert loaded.clip.source_sha256 == "a" * 64
    assert all(f.camera_id == "face_on" for f in loaded.frames)


def test_unknowns_are_omitted_rather_than_written_as_null(tmp_path: Path) -> None:
    """A file that knows nothing extra must not carry a null per frame saying so."""
    path = tmp_path / "swing.keypoints.json"
    save_keypoints(KeypointsFile(frames=_keypoints()), path)

    raw = json.loads(path.read_text(encoding="utf-8"))

    assert "clip" not in raw
    assert all("camera_id" not in frame for frame in raw["frames"])
    # Still readable, and still says None — omitted and null mean the same thing on the way back.
    assert load_keypoints(path).frames[0].camera_id is None


def test_partial_clip_metadata_survives(tmp_path: Path) -> None:
    """fps alone is a legitimate thing to know — a source needn't report its dimensions."""
    path = tmp_path / "swing.keypoints.json"
    save_keypoints(KeypointsFile(clip=ClipMetadata(fps=240.0), frames=_keypoints()), path)

    loaded = load_keypoints(path)

    assert loaded.clip is not None
    assert loaded.clip.fps == pytest.approx(240.0)
    assert loaded.clip.width is None


def test_save_creates_missing_parent_directories(tmp_path: Path) -> None:
    path = tmp_path / "nested" / "deeper" / "swing.keypoints.json"
    save_keypoints(KeypointsFile(frames=_keypoints()), path)

    assert load_keypoints(path).frames


# --- malformed input ------------------------------------------------------------------------


def test_unusable_top_level_type_names_the_file(tmp_path: Path) -> None:
    path = tmp_path / "swing.keypoints.json"
    path.write_text('"not keypoints"', encoding="utf-8")

    with pytest.raises(ValueError, match="swing.keypoints.json"):
        load_keypoints(path)


def test_malformed_json_raises(tmp_path: Path) -> None:
    path = tmp_path / "swing.keypoints.json"
    path.write_text("{not json", encoding="utf-8")

    with pytest.raises(ValueError):
        load_keypoints(path)
