"""`_ensure_model` caches the pose bundle, so a bad cache entry is permanent.

The download is guarded by `if not model_path.exists()`. Fetching straight to that path meant an
interrupted download — Ctrl-C, a dropped connection — left a truncated `.task` that the guard
accepts forever after, and every later `estimate_pose` failed inside MediaPipe with a model-parse
error nothing connects back to the interrupted fetch. Recovery required knowing to delete a file
by hand.

No MediaPipe here: `_ensure_model` is pure path handling plus one `urlretrieve`, so the failure
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
        estimator._ensure_model(tmp_path)

    assert not (tmp_path / estimator._MODEL_FILENAME).exists(), (
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

    first = estimator._ensure_model(tmp_path)
    second = estimator._ensure_model(tmp_path)

    assert first == second == tmp_path / estimator._MODEL_FILENAME
    assert first.read_bytes() == b"a whole model bundle"
    assert len(calls) == 1, "the cached bundle was re-downloaded"
