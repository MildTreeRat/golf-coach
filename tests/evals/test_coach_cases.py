"""The coach eval's cases: made as `cases.json` says, committed as made, served by `golf-mcp`.

[M43 P15, P16] `golf_coach.evals.coach_cases` writes `evals/coach_feedback/sessions/` and `cases.md`
from `cases.json`. What these tests hold:

- every tag has two cases, the second varied from the first, and every case shows exactly the tags
  it declares, read off its result, so a change to its inputs cannot quietly lose the feature it
  exists for;
- the committed sessions are byte for byte what the generator writes, and nothing else is there;
- the real `golf-mcp` serves every committed case with an analysis, and `cases.md` shows each brief
  exactly as it is served, which is the text the model coaches from.

The last needs the built server (`tests/conftest.py::golf_mcp` skips by name without it).
"""

from __future__ import annotations

import dataclasses
from pathlib import Path

import pytest

from golf_coach.evals.coach_cases import (
    CASES,
    CASES_DIR,
    CASES_MD,
    SESSIONS,
    TAGS,
    CaseError,
    CoachCase,
    build_bundle,
    derived_tags,
    load_cases,
    render_md,
    serve_briefs,
    store_cases,
    write,
)
from golf_coach.feedback.coach import GolfMcpServer

_REGENERATE = "run `.venv/Scripts/python.exe -m golf_coach.evals.coach_cases`"


def _cases() -> list[CoachCase]:
    return load_cases(CASES)


def _case(case_id: str) -> CoachCase:
    return next(case for case in _cases() if case.id == case_id)


def _files_under(root: Path) -> dict[Path, bytes]:
    return {
        path.relative_to(root): path.read_bytes()
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def test_every_tag_has_two_cases() -> None:
    """The plan's Decision 9: 16 cases, two per tag, so no tag rests on one swing."""
    per_tag = {tag: [case.id for case in _cases() if tag in case.tags] for tag in TAGS}
    assert {tag: len(ids) for tag, ids in per_tag.items()} == dict.fromkeys(TAGS, 2), per_tag


def test_the_two_cases_of_a_tag_differ_in_club_and_inputs() -> None:
    """P16's second case per tag is varied, not a copy: another club, other numbers."""
    for tag in TAGS:
        first, second = (case for case in _cases() if tag in case.tags)
        assert first.club != second.club, tag
        assert first.inputs != second.inputs, tag


def test_the_cases_cover_both_hands_and_an_unnamed_golfer() -> None:
    """`make_swing` draws a right-hander; a left-handed case mirrors the drifts (P15's finding)."""
    hands = {case.inputs["handedness"] for case in _cases()}
    assert hands == {"right", "left", None}


@pytest.mark.parametrize("case", _cases(), ids=lambda case: case.id)
def test_each_case_shows_exactly_the_tags_it_declares(case: CoachCase) -> None:
    """`build_bundle` refuses a mismatch; this asserts the reading too, so a pass is not vacuous."""
    assert derived_tags(build_bundle(case)) == set(case.tags)


def test_a_case_whose_result_does_not_show_its_tags_is_refused() -> None:
    mislabelled = dataclasses.replace(_case("clean-swing"), tags=("flagged-shot",))
    with pytest.raises(CaseError, match="the result shows \\['clean'\\]"):
        build_bundle(mislabelled)


def test_a_row_the_engine_did_not_record_is_refused_rather_than_skipped() -> None:
    """The short clip has five metrics, so no joint placement: naming one is a wrong case."""
    case = _case("unscored-finish")
    asks = dataclasses.replace(case, inputs={**case.inputs, "population": ["tour_joint_distance"]})
    with pytest.raises(CaseError, match="recorded no \\['tour_joint_distance'\\]"):
        build_bundle(asks)


def test_a_case_keeps_only_the_rows_it_names() -> None:
    names = {m.name for m in build_bundle(_case("clean-swing")).swing.measurements}
    assert "tour_joint_distance" in names
    assert not {"tour_trajectory_q", "tour_trajectory_t2_dtl", "pivot_hip_axis_drift_norm"} & names


def test_the_flag_on_a_shot_is_the_validators_and_not_typed_in() -> None:
    """The flagged case's inputs say nothing about review: its smash factor does not divide out."""
    case = _case("flagged-shot")
    assert "needs_review" not in case.inputs["shot"]
    shot = build_bundle(case).swing.shot
    assert shot is not None and shot.provenance is not None
    assert shot.provenance.needs_review
    assert any("smash factor 1.87 does not match" in w for w in shot.provenance.warnings)


@pytest.mark.parametrize("bad", ["Clean-Swing", "clean_swing", "clean--swing", ""])
def test_a_malformed_id_is_refused(tmp_path: Path, bad: str) -> None:
    path = tmp_path / "cases.json"
    text = CASES.read_text(encoding="utf-8").replace('"id": "clean-swing"', f'"id": "{bad}"')
    path.write_text(text, encoding="utf-8")
    with pytest.raises(CaseError, match="lowercase words joined by hyphens"):
        load_cases(path)


def test_a_reused_swing_is_refused(tmp_path: Path) -> None:
    path = tmp_path / "cases.json"
    text = CASES.read_text(encoding="utf-8").replace('"swing_id": "2"', '"swing_id": "1"')
    path.write_text(text, encoding="utf-8")
    with pytest.raises(CaseError, match="more than one case has the swing"):
        load_cases(path)


def test_the_committed_sessions_are_what_the_generator_writes(tmp_path: Path) -> None:
    """Byte for byte, and no file the generator would not write: a rerun leaves no diff."""
    store_cases(tmp_path / SESSIONS, _cases())
    made = _files_under(tmp_path / SESSIONS)
    committed = _files_under(CASES_DIR / SESSIONS)
    assert sorted(committed) == sorted(made), f"the case files differ: {_REGENERATE}"
    changed = sorted(str(path) for path in made if made[path] != committed[path])
    assert not changed, f"{changed} are not what the generator writes: {_REGENERATE}"


def test_writing_removes_a_case_file_no_longer_made(tmp_path: Path) -> None:
    stale = tmp_path / SESSIONS / "2026-01-01" / "9" / "analysis.json"
    stale.parent.mkdir(parents=True)
    stale.write_text("{}", encoding="utf-8")
    kept = Path(SESSIONS) / "2026-07-01" / "1" / "analysis.json"

    write(tmp_path, {kept: b"{}\n", Path(CASES_MD): b"# cases\n"})

    assert not stale.exists()
    assert (tmp_path / kept).read_bytes() == b"{}\n"
    assert (tmp_path / CASES_MD).read_bytes() == b"# cases\n"


def test_golf_mcp_serves_every_case_and_cases_md_shows_each_brief_as_served(
    golf_mcp: Path, tmp_path: Path
) -> None:
    """The committed sessions, through the real server: what the coach eval will send the model."""
    (tmp_path / "shots").mkdir()
    server = GolfMcpServer(
        binary=golf_mcp, sessions_dir=CASES_DIR / SESSIONS, shots_dir=tmp_path / "shots"
    )
    cases = _cases()

    briefs = serve_briefs(server, cases)

    assert sorted(briefs) == sorted(case.id for case in cases)
    for case in cases:
        assert briefs[case.id].startswith(f"SWING {case.swing_id} (session {case.session_id})")
    rendered = render_md(cases, briefs)
    committed = (CASES_DIR / CASES_MD).read_text(encoding="utf-8")
    assert committed == rendered, f"cases.md is not what golf-mcp serves: {_REGENERATE}"


def test_the_briefs_carry_what_each_tag_is_about(golf_mcp: Path, tmp_path: Path) -> None:
    """One line per tag that only that feature puts in a brief, so the tags reach the model."""
    (tmp_path / "shots").mkdir()
    server = GolfMcpServer(
        binary=golf_mcp, sessions_dir=CASES_DIR / SESSIONS, shots_dir=tmp_path / "shots"
    )
    briefs = serve_briefs(server, _cases())
    evidence = {
        "unscored-finish": ["- finish_balance: "],
        "unscored-no-loft": ["- flight_carry_yds: two spins fly that carry and no club loft"],
        "uncalibrated-placement": ["tour_trajectory_q (face-on view, NOT calibrated"],
        "uncalibrated-placement-dtl": ["tour_trajectory_q_dtl (down-the-line view, NOT calibrated"],
        "interim-pivot": ["pivot_hip_axis_drift_norm (face-on view, interim)"],
        "interim-pivot-dtl": ["pivot_hip_axis_drift_norm_dtl (down-the-line view, interim)"],
        "flagged-shot": ["needs_review: yes", "smash factor 1.87 does not match"],
        "flagged-shot-distance": ["needs_review: yes", "shot distance 135 does not match"],
        "partial-alignment": ["quality: top_impact", "motion start was estimated"],
        "partial-alignment-tempo": ["quality: top_impact", "tempo ratios disagree"],
        "no-handedness": ["- head_stays_back: "],
        "no-handedness-driver": ["- head_stays_back: "],
        "nothing-to-fix": ["All 6 checkpoints are inside tour range"],
        "nothing-to-fix-left": ["All 6 checkpoints are inside tour range"],
    }
    for case_id, lines in evidence.items():
        for line in lines:
            assert line in briefs[case_id], (case_id, line)
    for case_id in ("clean-swing", "clean-left-driver"):
        clean = briefs[case_id]
        assert "unscored: none" in clean and "needs_review: no" in clean, case_id
        assert "alignment_caveat" not in clean, case_id
    assert "quality: full" in briefs["clean-swing"]
