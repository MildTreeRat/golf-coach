"""The cause discriminator and the two findings behind it. [Career mode, step 5]

The statistics are pinned in `test_stats.py` and the guard in `test_baseline.py`. What is tested
here is the **reading**: that a repeatable miss and a scattered one come out as different things,
and — the case that matters more — that neither comes out at all when the data cannot support it.

The failure this file exists to catch is a confident pattern. "Your face is consistently open, check
your grip" is an instruction someone will act on, and it is produced by exactly the same code path
whether it rests on 40 shots or on noise that happened to line up. So the assertions cluster on the
boundaries: a bias inside the tolerance, an interval straddling it, a metric with no target at all.

Corpora are constructed from contracts directly, no disk — same posture as `test_baseline.py`, so a
discriminator failure can never be mistaken for a reader failure.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.analysis.baseline import build_baseline
from golf_coach.analysis.dispersion import build_dispersion, dispersion_for
from golf_coach.analysis.measure import POSE_MEASUREMENTS
from golf_coach.analysis.shot_measure import SHOT_MEASUREMENTS
from golf_coach.contracts.baseline import BaselineClaim
from golf_coach.contracts.career import CareerCorpus, CorpusSwing
from golf_coach.contracts.dispersion import (
    METRIC_TARGETS,
    DispersionPattern,
    Finding,
    MetricTarget,
)
from golf_coach.contracts.swing import Measurement

POSE = "pose:face_on"
LM = "launch_monitor:hd_golf"


def _measurement(name: str, value: float) -> Measurement:
    unit = "degrees" if name.endswith("_deg") else "shoulder_widths"
    source = LM if name.endswith("_deg") else POSE
    return Measurement(name=name, value=value, unit=unit, source=source, detail="test")


def _corpus(metric: str, values: list[float], *, sessions: list[str] | None = None) -> CareerCorpus:
    """One swing per value, each with its own clip and shot photo so nothing dedupes away."""
    swings = [
        CorpusSwing(
            player_id="aaron",
            session_id=(sessions[i] if sessions else f"2026-08-{i + 1:02d}"),
            swing_id=str(i),
            captured_at=datetime(2026, 8, 1 + (i % 28), 12, tzinfo=UTC),
            face_on_sha256=f"clip-{i}",
            shot_sha256=f"photo-{i}",
            measurements=[_measurement(metric, value)],
            analyzed=True,
        )
        for i, value in enumerate(values)
    ]
    return CareerCorpus(player_id="aaron", swings=swings)


def _spread(center: float, half_width: float, n: int) -> list[float]:
    """`n` values with mean exactly `center` and a spread set by `half_width`.

    Alternating `center ± half_width` rather than anything random: a test that can flake on a seed
    is a test that gets rerun until it passes.
    """
    return [center + (half_width if i % 2 == 0 else -half_width) for i in range(n)]


def _one(metric: str, values: list[float], **kwargs):
    return build_dispersion(_corpus(metric, values, **kwargs)).metrics[metric]


# --------------------------------------------------------------------------- the refusal


def test_at_the_n_on_disk_nothing_can_even_be_asked() -> None:
    """Two swings: both findings withheld, no pattern, and no number left to render."""
    metric = _one("face_to_path_deg", [10.9, 13.2])

    assert metric.n == 2
    assert metric.bias is Finding.WITHHELD
    assert metric.scatter is Finding.WITHHELD
    assert metric.pattern is None
    assert metric.points_at is None
    assert metric.center is None and metric.center_ci is None and metric.offset is None
    assert metric.sd is None and metric.sd_ci is None


def test_a_refusal_names_the_shortfall_it_is_waiting_on() -> None:
    metric = _one("face_to_path_deg", [10.9, 13.2])

    claims = {refusal.claim for refusal in metric.withheld}
    assert claims == {BaselineClaim.CENTER, BaselineClaim.SPREAD}
    center = next(r for r in metric.withheld if r.claim is BaselineClaim.CENTER)
    assert center.have_n == 2 and center.need_n == 5


def test_a_trend_refusal_is_not_carried_here() -> None:
    """It is real, and it is not why either finding is missing — sending someone to book a third
    session would be answering a question this step never asked."""
    metric = _one("face_to_path_deg", [10.9, 13.2])

    assert BaselineClaim.TREND not in {refusal.claim for refusal in metric.withheld}


# --------------------------------------------------------------------------- the four patterns


def test_a_repeatable_miss_reads_biased() -> None:
    """The static-cause half: same size every swing, so something fixed is producing it."""
    metric = _one("face_to_path_deg", _spread(9.0, 0.5, 12))

    assert metric.bias is Finding.ESTABLISHED
    assert metric.scatter is Finding.NOT_ESTABLISHED
    assert metric.pattern is DispersionPattern.BIASED
    assert metric.offset == pytest.approx(9.0)
    assert metric.points_at is not None and "before the swing starts" in metric.points_at


def test_a_miss_that_moves_reads_scattered() -> None:
    """The timing half: centered on the target, but nowhere near it twice running."""
    metric = _one("face_to_path_deg", _spread(0.0, 10.0, 12))

    assert metric.bias is Finding.NOT_ESTABLISHED
    assert metric.scatter is Finding.ESTABLISHED
    assert metric.pattern is DispersionPattern.SCATTERED
    assert metric.points_at is not None and "timing" in metric.points_at


def test_both_at_once_is_a_pattern_of_its_own() -> None:
    """They are separable problems, which is the whole reason the findings stay independent."""
    metric = _one("face_to_path_deg", _spread(15.0, 5.0, 12))

    assert metric.bias is Finding.ESTABLISHED
    assert metric.scatter is Finding.ESTABLISHED
    assert metric.pattern is DispersionPattern.BIASED_AND_SCATTERED


def test_the_reading_names_a_class_of_cause_and_never_a_specific_check() -> None:
    """One reading serves every metric, so it may not name checks that only fit one of them.

    Found by running it: the first cut said "grip, alignment, ball position, face at address",
    which is right under `face_to_path_deg` and printed unchanged under `head_sway_norm`, where a
    golfer would have been sent to check their grip about a head that moves.
    """
    face = _one("face_to_path_deg", _spread(9.0, 0.5, 12))
    sway = _one("head_sway_norm", _spread(0.30, 0.01, 12))

    assert face.pattern is DispersionPattern.BIASED
    assert sway.pattern is DispersionPattern.BIASED
    assert face.points_at == sway.points_at
    for banned in ("grip", "ball position", "face at address", "head"):
        assert banned not in face.points_at.lower()


def test_nothing_established_is_not_a_clean_bill() -> None:
    metric = _one("face_to_path_deg", _spread(0.3, 0.4, 12))

    assert metric.pattern is DispersionPattern.NOTHING_ESTABLISHED
    assert metric.points_at is not None
    assert "not a clean bill" in metric.points_at


# --------------------------------------------------------------------------- the boundaries


def test_a_miss_inside_the_tolerance_is_not_a_miss() -> None:
    """The case that stops the mechanism reporting the simulator's own noise as a tendency.

    A mean of 1.5 degrees with a tight interval is a real, reproducible number — and it sits inside
    the 2-degree tolerance, which is to say inside what this instrument can distinguish from zero.
    """
    metric = _one("face_to_path_deg", _spread(1.5, 0.2, 12))

    assert metric.center == pytest.approx(1.5)
    assert metric.bias is Finding.NOT_ESTABLISHED
    assert metric.pattern is DispersionPattern.NOTHING_ESTABLISHED


def test_an_interval_straddling_the_tolerance_settles_nothing() -> None:
    """The underpowered case. The mean is past the tolerance and the interval is not, so the data
    is consistent with a real bias *and* with none — and `NOT_ESTABLISHED` says exactly that."""
    metric = _one("face_to_path_deg", _spread(2.5, 2.0, 12))

    assert metric.center == pytest.approx(2.5)
    assert metric.center_ci is not None and metric.center_ci.low < 2.0 < metric.center_ci.high
    assert metric.bias is Finding.NOT_ESTABLISHED


def test_a_negative_bias_is_a_bias() -> None:
    """The tolerance is a band around the target, not a floor — a closed face is a miss too."""
    metric = _one("face_to_path_deg", _spread(-9.0, 0.5, 12))

    assert metric.bias is Finding.ESTABLISHED
    assert metric.offset == pytest.approx(-9.0)


def test_scatter_is_decided_on_the_lower_bound_not_the_estimate() -> None:
    """Measurement error inflates observed spread, so a sample sd at the tolerance is what a
    perfectly repeatable golfer measured by this pipeline would produce."""
    metric = _one("face_to_path_deg", _spread(0.0, 2.0, 12))

    assert metric.sd is not None and metric.sd > 2.0
    assert metric.sd_ci is not None and metric.sd_ci.low < 2.0
    assert metric.scatter is Finding.NOT_ESTABLISHED


# --------------------------------------------------------------------------- targets


def test_a_metric_with_no_target_gets_scatter_and_never_a_bias() -> None:
    """Four of the eight are here, each because declaring what *good* is would be inventing a band.

    They are not silently skipped: the scatter finding is real and `unavailable` carries the reason
    the other half is missing.
    """
    metric = _one("tempo_ratio", _spread(2.4, 2.0, 15))

    assert metric.target is None
    assert metric.bias is Finding.WITHHELD
    assert metric.scatter is Finding.ESTABLISHED
    assert metric.pattern is None, "half a pair is not a pattern"
    assert metric.points_at is not None and "no target" in metric.points_at
    assert any("step 6" in reason for reason in metric.unavailable)


def test_an_unregistered_metric_is_silent_by_omission() -> None:
    """One notch more conservative than the minimum-N fallback: with no tolerance, neither question
    can be asked, and borrowing another metric's error term is how a verdict nobody derived
    appears."""
    metric = _one("future_metric", _spread(1.0, 0.1, 12))

    assert metric.tolerance is None
    assert metric.bias is Finding.WITHHELD and metric.scatter is Finding.WITHHELD
    assert any("METRIC_TARGETS" in reason for reason in metric.unavailable)


def test_every_production_metric_has_a_tolerance() -> None:
    """The pin that catches a metric added to `measure.py` and nowhere else. A new metric with no
    row here is silent, which is safe — but silent for a reason nobody chose."""
    produced = set(POSE_MEASUREMENTS) | set(SHOT_MEASUREMENTS)

    assert produced == set(METRIC_TARGETS)


def test_a_target_less_metric_must_say_why() -> None:
    """A bias claim that is simply absent is indistinguishable from one nobody thought to make."""
    with pytest.raises(ValueError, match="no reason was given"):
        MetricTarget(metric="x", target=None, tolerance=1.0, provenance="test")


# --------------------------------------------------------------------------- the inherited guard


def test_bias_opens_at_centers_floor_and_scatter_at_spreads() -> None:
    """No new thresholds: the two findings gate on the two step-4 claims, unchanged."""
    metric = _one("head_sway_norm", _spread(0.30, 0.02, 6))

    assert metric.n == 6
    assert metric.bias is Finding.ESTABLISHED
    assert metric.scatter is Finding.WITHHELD
    assert metric.pattern is None
    assert metric.sd is None
    assert [r.claim for r in metric.withheld] == [BaselineClaim.SPREAD]


def test_a_noisier_metric_stays_silent_where_a_cleaner_one_speaks() -> None:
    """`tempo_ratio`'s per-metric floors still apply, because it is genuinely the same guard."""
    tempo = _one("tempo_ratio", _spread(2.4, 0.2, 12))
    sway = _one("head_sway_norm", _spread(0.30, 0.02, 12))

    assert tempo.scatter is Finding.WITHHELD, "tempo's SPREAD floor is 15, not 10"
    assert sway.scatter is not Finding.WITHHELD


def test_a_withheld_center_leaves_no_number_for_a_bias_to_be_computed_from() -> None:
    """The structural property: this module consumes guarded statistics rather than raw values, so
    a sealed mean is absent from its input, not merely ignored by it."""
    baseline = build_baseline(_corpus("head_sway_norm", _spread(0.30, 0.02, 3)))
    metric = dispersion_for(baseline.metrics["head_sway_norm"])

    assert baseline.metrics["head_sway_norm"].mean is None
    assert metric.bias is Finding.WITHHELD
    assert metric.center is None


# --------------------------------------------------------------------------- sessions


def test_spread_inflated_by_drift_between_sessions_says_so() -> None:
    """Two tight sessions in different places pool into one wide spread. Read naively that is a
    timing problem; it is a golfer who changed something in between."""
    sessions = ["2026-08-01"] * 6 + ["2026-08-08"] * 6
    metric = _one(
        "head_sway_norm",
        _spread(0.20, 0.01, 6) + _spread(0.50, 0.01, 6),
        sessions=sessions,
    )

    assert metric.n_sessions == 2
    assert metric.within_session_sd is not None
    assert metric.sd is not None and metric.sd > metric.within_session_sd
    assert any("between sessions" in caveat for caveat in metric.caveats)


def test_sessions_that_agree_raise_no_caveat() -> None:
    sessions = ["2026-08-01"] * 6 + ["2026-08-08"] * 6
    metric = _one("head_sway_norm", _spread(0.30, 0.10, 12), sessions=sessions)

    assert metric.within_session_sd is not None
    assert metric.caveats == []


def test_one_session_cannot_show_drift_between_sessions() -> None:
    metric = _one("head_sway_norm", _spread(0.30, 0.10, 12), sessions=["2026-08-01"] * 12)

    assert metric.n_sessions == 1
    assert metric.within_session_sd is None
    assert metric.caveats == []


def test_the_within_session_spread_is_gated_with_the_pooled_one() -> None:
    """It is a spread statistic, so it must not reach the output while SPREAD is refused — that
    would be a number arriving around the guard rather than through it."""
    sessions = ["2026-08-01"] * 3 + ["2026-08-08"] * 3
    values = _spread(0.20, 0.01, 3) + _spread(0.50, 0.01, 3)
    metric = _one("head_sway_norm", values, sessions=sessions)

    assert metric.scatter is Finding.WITHHELD
    assert metric.within_session_sd is None


# --------------------------------------------------------------------------- assembly


def test_n_agrees_with_the_baseline_for_every_metric() -> None:
    """Three readers of one corpus, and the `n` beside a pattern has to be the `n` behind it."""
    corpus = _corpus("head_sway_norm", _spread(0.30, 0.02, 7))
    baseline = build_baseline(corpus)
    dispersion = build_dispersion(corpus)

    assert set(dispersion.metrics) == set(baseline.metrics)
    for name, metric in dispersion.metrics.items():
        assert metric.n == baseline.metrics[name].n
        assert metric.n_sessions == baseline.metrics[name].n_sessions
        assert corpus.metric_counts.get(name, metric.n) == metric.n


def test_re_uploads_of_one_clip_cannot_manufacture_a_pattern() -> None:
    """The variance-collapsing failure, reaching the thing it would have corrupted. Twelve copies
    of one swing would read as a golfer of impossible consistency."""
    corpus = _corpus("head_sway_norm", _spread(0.30, 0.02, 12))
    for swing in corpus.swings:
        swing.face_on_sha256 = "one-clip"

    metric = build_dispersion(corpus).metrics["head_sway_norm"]

    assert metric.n == 1
    assert metric.pattern is None


def test_an_empty_corpus_is_an_empty_reading_not_an_error() -> None:
    dispersion = build_dispersion(CareerCorpus(player_id="nobody"))

    assert dispersion.metrics == {}
    assert dispersion.nothing_established
    assert dispersion.patterns_established == 0


def test_metrics_are_sorted_so_two_runs_agree() -> None:
    corpus = _corpus("head_sway_norm", _spread(0.30, 0.02, 4))
    for swing in corpus.swings:
        swing.measurements.append(_measurement("face_to_path_deg", 9.0))

    names = list(build_dispersion(corpus).metrics)

    assert names == sorted(names)


def test_provenance_survives_onto_the_reading() -> None:
    metric = _one("face_to_path_deg", _spread(9.0, 0.5, 12))

    assert metric.unit == "degrees"
    assert metric.source == LM
