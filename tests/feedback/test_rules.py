"""Rule-based feedback: a swing result becomes a ranked FeedbackPayload with a headline."""

from __future__ import annotations

from golf_coach.contracts.feedback import Severity
from golf_coach.contracts.swing import CheckpointScore, SwingResult
from golf_coach.feedback.rules import build_feedback


def _result_with(*checkpoints: CheckpointScore, overall: float = 100.0, **kwargs) -> SwingResult:
    return SwingResult(
        swing_id="swing-1",
        session_id="session-1",
        checkpoint_scores=list(checkpoints),
        overall_score=overall,
        **kwargs,
    )


def _passing(name: str, percentile: float | None = None, one_sided: bool = True) -> CheckpointScore:
    return CheckpointScore(
        name=name,
        score=1.0,
        passed=True,
        message=f"{name} ok.",
        percentile=percentile,
        population_n=100 if percentile is not None else None,
        one_sided=one_sided,
    )


def _failing(name: str, score: float) -> CheckpointScore:
    return CheckpointScore(
        name=name,
        score=score,
        passed=False,
        message=f"{name} bad.",
        percentile=90.0,
        population_n=100,
        one_sided=True,
    )


def test_passing_tempo_yields_info_tip() -> None:
    cp = CheckpointScore(name="tempo", score=1.0, passed=True, message="Good tempo.")
    payload = build_feedback(_result_with(cp))

    assert payload.swing_id == "swing-1"
    assert payload.overall_score == 100.0
    assert len(payload.tips) == 1
    tip = payload.tips[0]
    assert tip.checkpoint == "tempo"
    assert tip.severity is Severity.INFO
    assert tip.text == "Good tempo."


def test_bad_tempo_is_flagged_by_severity() -> None:
    cp = CheckpointScore(name="tempo", score=0.1, passed=False, message="Tempo too quick.")
    payload = build_feedback(_result_with(cp, overall=10.0))
    assert payload.tips[0].severity is Severity.MAJOR


def test_failures_rank_ahead_of_passes_worst_first() -> None:
    payload = build_feedback(
        _result_with(_passing("head_sway", 80.0), _failing("tempo", 0.7), _failing("balance", 0.2))
    )
    assert [tip.checkpoint for tip in payload.tips] == ["balance", "tempo", "head_sway"]


def test_passes_rank_by_how_close_to_the_edge_they_sit() -> None:
    """Every pass scores 1.0, so only the percentile can order them."""
    payload = build_feedback(
        _result_with(
            _passing("balance", 20.0), _passing("head_sway", 85.0), _passing("tempo", 55.0)
        )
    )
    assert [tip.checkpoint for tip in payload.tips] == ["head_sway", "tempo", "balance"]


def test_two_sided_metric_is_extreme_at_both_rails() -> None:
    """A quick tempo (p10) must outrank a near-median one, which one-sided arithmetic would miss."""
    payload = build_feedback(
        _result_with(_passing("balance", 55.0), _passing("tempo", 12.0, one_sided=False))
    )
    assert [tip.checkpoint for tip in payload.tips] == ["tempo", "balance"]


def test_ranking_survives_missing_percentiles() -> None:
    """No distribution resolved is not an error — fall back rather than crash or invert."""
    payload = build_feedback(_result_with(_passing("a"), _failing("b", 0.3), _passing("c")))
    assert payload.tips[0].checkpoint == "b"
    assert len(payload.tips) == 3


def test_headline_names_the_worst_failure() -> None:
    payload = build_feedback(_result_with(_failing("tempo", 0.7), _failing("balance", 0.2)))
    assert payload.headline is not None
    assert payload.headline.startswith("Work on balance first.")
    assert "balance bad." in payload.headline


def test_clean_swing_headline_names_the_closest_call() -> None:
    """A 100/100 swing still has a nearest fault, and saying so is the point of the percentile."""
    payload = build_feedback(_result_with(_passing("head_sway", 85.0), _passing("balance", 20.0)))
    assert payload.headline == (
        "All 2 checkpoints are inside tour range. Closest to the edge: head sway."
    )


def test_comfortably_clean_swing_gets_no_finger_pointed() -> None:
    payload = build_feedback(_result_with(_passing("head_sway", 30.0), _passing("balance", 20.0)))
    assert payload.headline == "All 2 checkpoints are inside tour range."


def test_no_checkpoints_means_no_verdict_rather_than_a_clean_bill() -> None:
    payload = build_feedback(_result_with(overall=0.0))
    assert payload.headline is None
    assert payload.tips == []


def test_unmeasured_checkpoint_gets_its_own_tip() -> None:
    payload = build_feedback(_result_with(_passing("balance", 20.0), unscored=["tempo"]))
    assert [tip.checkpoint for tip in payload.tips] == ["balance", "tempo"]
    unmeasured = payload.tips[-1]
    assert unmeasured.severity is Severity.INFO
    assert "could not be measured" in unmeasured.text
