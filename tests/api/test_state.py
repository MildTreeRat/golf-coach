"""`analysis.state.json` and the version stamp — the two ways a stored result goes out of date.

They are genuinely different axes and the distinction is the point of this file. `matches()` asks
whether the *bytes* under a result have changed; `is_outdated` asks whether the *code* has. A swing
can fail either while passing the other, and before the stamp existed the second one was
unobservable — which is how three swing directories sat on disk looking current while carrying no
measurements at all.

The `record_state` tests pin the invariant that came out of that: the sidecar is a denormalised
copy of `analysis.json`, so whatever writes one writes the other. It used to be written only by
the worker, so every CLI run was a way to leave the two disagreeing — and `2026-08-09/2` did, for
three days, with the upload page showing 67/100 for a swing whose results page showed 95/100.

Base install: nothing here imports cv2, mediapipe or paddleocr.
"""

from __future__ import annotations

from datetime import UTC, datetime, timedelta

import pytest

from golf_coach.api.pipeline import PipelineOutcome, analyze_swing_dir, record_state
from golf_coach.api.state import (
    AnalysisState,
    is_outdated,
    load_state,
    now,
    save_state,
    stored_analysis_version,
)
from golf_coach.contracts.feedback import FeedbackPayload
from golf_coach.contracts.swing import ANALYSIS_VERSION, SwingBundleResult, SwingResult
from golf_coach.storage.manifest import (
    Role,
    RoleFile,
    SwingManifest,
    manifest_path,
    save_manifest,
)

_WHEN = datetime(2026, 8, 12, 1, 0, tzinfo=UTC)


def _manifest(swing_dir, *, roles=("face_on", "down_the_line", "shot_screen")) -> SwingManifest:
    manifest = SwingManifest(
        swing_id="1",
        session_id="2026-08-12",
        created_at=_WHEN,
        updated_at=_WHEN,
        player_id="aaron",
        roles={
            Role(role): RoleFile(
                role=Role(role),
                filename=f"{role}.abc123.mov",
                content_sha256=f"sha-{role}",
                original_filename=f"{role}.mov",
                content_type="video/quicktime",
                size_bytes=10,
                received_at=_WHEN,
            )
            for role in roles
        },
    )
    swing_dir.mkdir(parents=True, exist_ok=True)
    save_manifest(manifest, manifest_path(swing_dir))
    return manifest


def _outcome(score: float = 88.5, headline: str = "Work on tempo first.") -> PipelineOutcome:
    return PipelineOutcome(
        result=SwingBundleResult(
            swing_id="1",
            session_id="2026-08-12",
            swing=SwingResult(swing_id="1", session_id="2026-08-12", overall_score=score),
            analysis_version=ANALYSIS_VERSION,
            feedback=FeedbackPayload(swing_id="1", overall_score=score, headline=headline),
        )
    )


# ------------------------------------------------------------------ reading the version stamp


@pytest.mark.parametrize(
    ("analysis", "expected"),
    [
        ({}, 0),
        ({"analysis_version": ANALYSIS_VERSION}, ANALYSIS_VERSION),
        ({"analysis_version": "1"}, 0),
        ({"analysis_version": -3}, 0),
        ({"analysis_version": True}, 0),
        (None, 0),
    ],
    ids=["absent", "current", "string", "negative", "bool", "no-analysis"],
)
def test_a_version_that_cannot_be_trusted_reads_as_zero(analysis, expected) -> None:
    """"Old enough that I cannot tell" and "definitely old" want the same repair, so they get the
    same answer. `True` is in here because `isinstance(True, int)` is the trap."""
    assert stored_analysis_version(analysis) == expected


def test_a_missing_analysis_is_absent_rather_than_outdated() -> None:
    """A different repair with a different report — `NOT_ANALYZED`, not `OUTDATED`."""
    assert is_outdated(None) is False
    assert is_outdated({}) is True
    assert is_outdated({"analysis_version": ANALYSIS_VERSION}) is False


# --------------------------------------------------------------- the sidecar and its original


def test_record_state_denormalises_the_result_it_was_handed(tmp_path) -> None:
    swing_dir = tmp_path / "swing"
    manifest = _manifest(swing_dir)

    state = record_state(swing_dir, manifest, _outcome(), started_at=now())

    assert state.status == "done"
    assert state.score == 88.5
    assert state.headline == "Work on tempo first."
    assert state.partial is False
    assert load_state(swing_dir) == state


def test_record_state_reports_a_resultless_run_as_failed(tmp_path) -> None:
    swing_dir = tmp_path / "swing"
    manifest = _manifest(swing_dir, roles=("down_the_line",))

    state = record_state(
        swing_dir,
        manifest,
        PipelineOutcome(error="no usable face-on view"),
        started_at=now(),
    )

    assert state.status == "failed"
    assert state.error == "no usable face-on view"
    assert state.score is None
    # A bundle that never had all three roles is partial whichever way the run went.
    assert state.partial is True
    assert sorted(state.missing_roles) == ["face_on", "shot_screen"]


def test_record_state_keeps_the_queued_at_of_a_run_it_did_not_start(tmp_path) -> None:
    """The worker owns `queued`; the pipeline owns the summary. Neither may erase the other."""
    swing_dir = tmp_path / "swing"
    manifest = _manifest(swing_dir)
    queued = now() - timedelta(seconds=30)
    save_state(AnalysisState(status="queued", queued_at=queued), swing_dir)

    state = record_state(swing_dir, manifest, _outcome(), started_at=now())

    assert state.queued_at == queued
    assert state.duration_seconds is not None


def test_a_rerun_leaves_the_sidecar_agreeing_with_the_analysis(tmp_path) -> None:
    """The regression this whole seam exists for.

    An earlier run's sidecar quoting an earlier score must not survive a later run — the upload
    page reads this file and the results page reads `analysis.json`, so a stale copy shows one
    swing at two scores with nothing anywhere to flag it. `matches()` cannot catch it: the inputs
    are identical, which is exactly why they are identical on a re-analysis.
    """
    swing_dir = tmp_path / "swing"
    manifest = _manifest(swing_dir)
    record_state(swing_dir, manifest, _outcome(score=66.7, headline="old"), started_at=now())

    record_state(swing_dir, manifest, _outcome(score=94.9, headline="new"), started_at=now())

    state = load_state(swing_dir)
    assert state is not None
    assert state.matches(manifest)  # unchanged inputs, so staleness could never have caught it
    assert (state.score, state.headline) == (94.9, "new")


def test_the_pipeline_records_its_own_failure(tmp_path) -> None:
    """`analyze_swing_dir` writes the sidecar on the way out, including when it produced nothing.

    Reachable on the base install because the manifest names clips that are not on disk, so the
    run dies at "no usable face-on view" long before anything imports cv2.
    """
    swing_dir = tmp_path / "swing"
    _manifest(swing_dir)

    outcome = analyze_swing_dir(swing_dir)

    assert outcome.result is None
    state = load_state(swing_dir)
    assert state is not None
    assert state.status == "failed"
    assert "face-on" in (state.error or "")


def test_a_directory_with_no_manifest_gets_no_state(tmp_path) -> None:
    """Without a manifest there are no `inputs` to key a state on, and a directory that cannot say
    which bytes it holds is not a swing yet."""
    swing_dir = tmp_path / "swing"
    swing_dir.mkdir()

    outcome = analyze_swing_dir(swing_dir)

    assert outcome.error is not None
    assert load_state(swing_dir) is None
