"""`ensure_pose_model` caches the pose bundle, so a bad cache entry is permanent.

The download is guarded by `if not model_path.exists()`. Fetching straight to that path meant an
interrupted download — Ctrl-C, a dropped connection — left a truncated `.task` that the guard
accepts forever after, and every later `estimate_pose` failed inside MediaPipe with a model-parse
error nothing connects back to the interrupted fetch. Recovery required knowing to delete a file
by hand.

No MediaPipe here: `ensure_pose_model` is pure path handling plus one `urlretrieve`, so the failure
is reproducible with a stub.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from golf_coach.pose import estimator


def test_an_interrupted_download_leaves_no_cached_model(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """The regression: a half-written bundle must not satisfy the next call's `exists()` check."""

    def die_midway(url: str, filename: Path) -> None:
        Path(filename).write_bytes(b"half a model bundle")
        raise KeyboardInterrupt

    monkeypatch.setattr(estimator.urllib.request, "urlretrieve", die_midway)

    with pytest.raises(KeyboardInterrupt):
        estimator.ensure_pose_model(tmp_path)

    assert not (tmp_path / estimator.model_filename()).exists(), (
        "an interrupted download left a truncated bundle at the cache path — the next call's "
        "exists() check will accept it and MediaPipe will fail on it forever"
    )
    assert list(tmp_path.iterdir()) == [], "the partial file was left behind"


def test_a_completed_download_lands_and_is_not_refetched(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    calls: list[str] = []

    def fake_fetch(url: str, filename: Path) -> None:
        calls.append(url)
        Path(filename).write_bytes(b"a whole model bundle")

    monkeypatch.setattr(estimator.urllib.request, "urlretrieve", fake_fetch)

    first = estimator.ensure_pose_model(tmp_path)
    second = estimator.ensure_pose_model(tmp_path)

    assert first == second == tmp_path / estimator.model_filename()
    assert first.read_bytes() == b"a whole model bundle"
    assert len(calls) == 1, "the cached bundle was re-downloaded"


def test_each_variant_caches_under_its_own_name(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Switching `pose_model_variant` must not overwrite the bundle for the old one.

    One filename for all three would make every switch a 30 MB re-download, and — worse — would
    leave a `pose_landmarker.task` whose contents nobody could identify from the path.
    """
    urls: list[str] = []

    def fake_fetch(url: str, filename: Path) -> None:
        urls.append(url)
        Path(filename).write_bytes(b"a whole model bundle")

    monkeypatch.setattr(estimator.urllib.request, "urlretrieve", fake_fetch)

    heavy = estimator.ensure_pose_model(tmp_path, "heavy")
    lite = estimator.ensure_pose_model(tmp_path, "lite")

    assert heavy.name == "pose_landmarker_heavy.task"
    assert lite.name == "pose_landmarker_lite.task"
    assert [u.count("heavy") for u in urls] == [2, 0], "the URL did not follow the variant"
    assert len(urls) == 2


def test_an_unknown_variant_is_refused_before_the_network(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """A typo in `GOLF_POSE_MODEL_VARIANT` must not surface as a 404 mid-session."""
    monkeypatch.setattr(
        estimator.urllib.request,
        "urlretrieve",
        lambda url, filename: pytest.fail("a bad variant reached the network"),
    )

    with pytest.raises(ValueError, match="unknown MediaPipe pose variant"):
        estimator.ensure_pose_model(tmp_path, "hevy")


def test_the_estimator_name_follows_the_configured_variant(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """The stamp `api.pipeline.keypoints_for` keys its cache on, and what it has to say."""
    assert estimator.pose_estimator_name("lite") == "mediapipe:lite"

    monkeypatch.setattr(estimator.settings, "pose_model_variant", "heavy")
    assert estimator.pose_estimator_name() == "mediapipe:heavy"
