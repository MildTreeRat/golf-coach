"""The read layer behind the MCP tools.

Weighted toward the cases where a wrong answer would be *plausible* — a stale score, a flagged
OCR parse quoted as fact, an alignment tier presented as full sync. Those are the ones an LLM
consumer cannot catch for itself.
"""

from __future__ import annotations

from datetime import UTC, datetime
from pathlib import Path

from golf_coach.contracts.shot import ShotData, ShotProvenance, ShotSource
from golf_coach.mcp import query
from golf_coach.storage.manifest import Role


class _StubSource:
    """A `ShotDataSource` over a fixed list — the port is a Protocol, so this is enough."""

    def __init__(self, shots: list[ShotData]) -> None:
        self._shots = shots

    def recent(self, count: int) -> list[ShotData]:
        return self._shots[:count]

    def stream(self):
        yield from self._shots


def _shot(shot_id: str, *, needs_review: bool = False, carry: float = 121.0) -> ShotData:
    return ShotData(
        shot_id=shot_id,
        session_id="2026-08-10",
        timestamp=datetime(2026, 8, 10, 1, 39, tzinfo=UTC),
        source=ShotSource.SCREEN,
        club_head_speed=98.3,
        ball_speed=90.5,
        carry_distance=carry,
        shot_type="DRAW",
        provenance=ShotProvenance(
            device="hd_golf",
            parse_confidence=0.5 if needs_review else 0.95,
            needs_review=needs_review,
            warnings=["smash factor failed the physics cross-check"] if needs_review else [],
        ),
    )


# --------------------------------------------------------------------------------------
# Sessions and swings
# --------------------------------------------------------------------------------------


def test_sessions_are_listed_newest_first(sessions_dir: Path) -> None:
    sessions = query.list_sessions(sessions_dir)

    assert [s.session_id for s in sessions] == ["2026-08-10", "2026-08-09", "2026-08-08"]
    assert sessions[0].swing_count == 2
    assert sessions[0].analyzed_count == 2


def test_listing_falls_back_to_analysis_json_when_there_is_no_state_sidecar(
    sessions_dir: Path,
) -> None:
    """Swings analyzed by the CLI before M7 Phase 5 have no `analysis.state.json`.

    Reporting them as unanalyzed would contradict `get_swing`, which returns their full result
    a moment later — the inconsistency this fallback exists to prevent.
    """
    session = next(s for s in query.list_sessions(sessions_dir) if s.session_id == "2026-08-09")

    swing = session.swings[0]
    assert swing.status == "done"
    assert swing.score == 86.1
    assert swing.headline is not None
    assert session.analyzed_count == 1


def test_an_uploaded_but_unanalyzed_swing_reports_no_score(sessions_dir: Path) -> None:
    session = next(s for s in query.list_sessions(sessions_dir) if s.session_id == "2026-08-08")

    swing = session.swings[0]
    assert swing.status == "not analyzed"
    assert swing.score is None
    assert swing.roles == ["face_on"]
    assert session.analyzed_count == 0


def test_a_reuploaded_clip_makes_its_own_result_stale(
    tmp_path: Path, swing_writer, analysis_factory
) -> None:
    """The state file records the input hashes; changed inputs must not report `done`.

    Handing out a score for footage that is no longer on disk is the failure mode — it reads
    as current and is not.
    """
    root = tmp_path / "sessions"
    swing_writer(root, "2026-08-10", "1", analysis=analysis_factory("2026-08-10", "1"), stale=True)

    swing = query.list_sessions(root)[0].swings[0]

    assert swing.status.startswith("stale")


def test_get_swing_returns_checkpoints_with_bands_and_percentiles(sessions_dir: Path) -> None:
    view = query.get_swing(sessions_dir, "2026-08-10", "1")

    assert view is not None
    assert view.overall_score == 94.9
    tempo = next(c for c in view.checkpoints if c.name == "tempo")
    assert (tempo.expected_low, tempo.expected_high) == (2.72, 4.71)
    assert tempo.observed == 2.42
    assert tempo.passed is False
    assert (tempo.percentile, tempo.population_n) == (10.0, 1399)
    assert view.tips and view.tips[0].checkpoint == "tempo"


def test_get_swing_is_none_for_a_swing_that_does_not_exist(sessions_dir: Path) -> None:
    assert query.get_swing(sessions_dir, "2026-08-10", "99") is None
    assert query.get_swing(sessions_dir, "1999-01-01", "1") is None


def test_a_swing_without_analysis_still_reports_its_status(sessions_dir: Path) -> None:
    view = query.get_swing(sessions_dir, "2026-08-08", "1")

    assert view is not None
    assert view.status == "not analyzed"
    assert view.overall_score is None
    assert view.checkpoints == []


def test_unscored_checkpoints_are_named_and_not_confused_with_failures(
    tmp_path: Path, swing_writer, analysis_factory
) -> None:
    root = tmp_path / "sessions"
    swing_writer(
        root,
        "2026-08-10",
        "1",
        analysis=analysis_factory("2026-08-10", "1", unscored=["tempo"]),
    )

    view = query.get_swing(root, "2026-08-10", "1")

    assert view is not None
    assert view.unscored == ["tempo"]


# --------------------------------------------------------------------------------------
# The caveats — the reason the views are not passthroughs
# --------------------------------------------------------------------------------------


def test_a_degraded_alignment_carries_a_caveat(sessions_dir: Path) -> None:
    view = query.get_swing(sessions_dir, "2026-08-10", "1")

    assert view is not None
    assert view.alignment_quality == "top_impact"
    assert view.alignment_caveat is not None
    assert "not on every instant" in view.alignment_caveat


def test_a_full_alignment_carries_no_caveat(
    tmp_path: Path, swing_writer, analysis_factory
) -> None:
    """FULL is the only tier that anchored on all three instants, so it is the only silent one."""
    root = tmp_path / "sessions"
    analysis = analysis_factory("2026-08-10", "1", quality="full")
    swing_writer(root, "2026-08-10", "1", analysis=analysis)

    view = query.get_swing(root, "2026-08-10", "1")

    assert view is not None
    assert view.alignment_quality == "full"
    assert view.alignment_caveat is None


def test_a_flagged_shot_arrives_labelled(
    tmp_path: Path, swing_writer, analysis_factory
) -> None:
    """ADR-014: a photographed number is a measurement of a measurement.

    OCR can drop a digit and produce a figure that is wrong and perfectly plausible, so
    `needs_review` has to reach the consumer or it becomes a confident wrong answer.
    """
    root = tmp_path / "sessions"
    shot = _shot("2026-08-10-1", needs_review=True).model_dump(mode="json")
    swing_writer(root, "2026-08-10", "1", analysis=analysis_factory("2026-08-10", "1", shot=shot))

    view = query.get_swing(root, "2026-08-10", "1")

    assert view is not None and view.shot is not None
    assert view.shot.needs_review is True
    assert view.shot.warnings
    assert view.shot.parse_confidence == 0.5


# --------------------------------------------------------------------------------------
# Session aggregates
# --------------------------------------------------------------------------------------


def test_session_summary_aggregates_both_axes(
    tmp_path: Path, swing_writer, analysis_factory
) -> None:
    root = tmp_path / "sessions"
    for swing_id, score in (("1", 94.9), ("2", 93.8)):
        shot = _shot(f"2026-08-10-{swing_id}", carry=120.0 if swing_id == "1" else 130.0)
        swing_writer(
            root,
            "2026-08-10",
            swing_id,
            analysis=analysis_factory(
                "2026-08-10", swing_id, overall=score, shot=shot.model_dump(mode="json")
            ),
        )

    detail = query.get_session_summary(root, "2026-08-10")

    assert detail is not None
    assert detail.analyzed_count == 2
    assert detail.mean_score == 94.3  # (94.9 + 93.8) / 2 = 94.35, rounded half-to-even
    assert (detail.best_score, detail.worst_score) == (94.9, 93.8)
    assert detail.checkpoint_pass_rates == {"head_sway": "2/2", "tempo": "0/2"}
    assert detail.shots_attached == 2
    assert detail.shot_averages["carry_distance"] == 125.0


def test_a_flagged_shot_is_counted_but_kept_out_of_the_averages(
    tmp_path: Path, swing_writer, analysis_factory
) -> None:
    """A parse we do not trust must not silently move a number presented as a session average."""
    root = tmp_path / "sessions"
    good = _shot("good", carry=120.0)
    bad = _shot("bad", needs_review=True, carry=400.0)
    for swing_id, shot in (("1", good), ("2", bad)):
        swing_writer(
            root,
            "2026-08-10",
            swing_id,
            analysis=analysis_factory("2026-08-10", swing_id, shot=shot.model_dump(mode="json")),
        )

    detail = query.get_session_summary(root, "2026-08-10")

    assert detail is not None
    assert detail.shots_attached == 2
    assert detail.shots_needing_review == 1
    assert detail.shot_averages["carry_distance"] == 120.0


def test_session_summary_is_none_only_when_the_session_is_absent(sessions_dir: Path) -> None:
    """"You uploaded a swing and nothing finished" is an answer, not a missing session."""
    assert query.get_session_summary(sessions_dir, "1999-01-01") is None

    empty = query.get_session_summary(sessions_dir, "2026-08-08")
    assert empty is not None
    assert empty.swing_count == 1
    assert empty.analyzed_count == 0
    assert empty.mean_score is None


def test_listing_a_missing_sessions_dir_is_empty_not_an_error(tmp_path: Path) -> None:
    assert query.list_sessions(tmp_path / "nope") == []


# --------------------------------------------------------------------------------------
# Shots
# --------------------------------------------------------------------------------------


def test_recent_shots_surface_review_state(tmp_path: Path) -> None:
    source = _StubSource([_shot("a"), _shot("b", needs_review=True)])

    views = query.recent_shots(source, 10)

    assert [v.shot_id for v in views] == ["a", "b"]
    assert views[0].needs_review is False
    assert views[1].needs_review is True
    assert views[0].metrics["club_head_speed"] == 98.3
    assert views[0].metrics["shot_type"] == "DRAW"


def test_recent_shots_of_zero_or_fewer_is_empty(tmp_path: Path) -> None:
    source = _StubSource([_shot("a")])

    assert query.recent_shots(source, 0) == []
    assert query.recent_shots(source, -5) == []


def test_get_shot_by_id_finds_and_misses(tmp_path: Path) -> None:
    source = _StubSource([_shot("a"), _shot("b")])

    assert (found := query.get_shot(source, "b")) is not None and found.shot_id == "b"
    assert query.get_shot(source, "missing") is None


def test_every_shot_field_is_accounted_for_by_the_metric_split() -> None:
    """`_METRIC_FIELDS` is a hand-typed copy of `ShotData`, read by `getattr` with a default.

    That default is why nothing else can catch this: add a metric to `ShotData` — ADR-004 names
    the R10 adapter as the next thing to do exactly that — and it is silently missing from every
    MCP payload and from `get_session_summary`'s averages, with mypy at "no issues found" and the
    suite green. Asserting the split is *exhaustive*, rather than that the listed names still
    exist, is what makes the new field fail here instead of vanishing.
    """
    split = set(query._METRIC_FIELDS) | set(query._NON_METRIC_FIELDS)

    assert split == set(ShotData.model_fields), (
        "ShotData and mcp/query.py's metric split have diverged: "
        f"unclassified {sorted(set(ShotData.model_fields) - split)}, "
        f"no longer on the model {sorted(split - set(ShotData.model_fields))}"
    )


def test_roles_that_never_arrived_are_visible_in_the_listing(sessions_dir: Path) -> None:
    """A partial bundle is a real state — the swing exists with one of three files."""
    session = next(s for s in query.list_sessions(sessions_dir) if s.session_id == "2026-08-08")

    assert session.swings[0].roles == [Role.FACE_ON.value]
