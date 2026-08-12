"""The personal baseline and its minimum-N guard. [Career mode, step 4]

The statistics are the easy half and are pinned in `test_stats.py`. What is tested here is the
**refusal** — because a guard that is slightly too permissive fails in the one direction nobody
notices: it produces a confident-looking sentence about a golfer's tendency, built out of two
swings, that reads exactly like a sentence built out of two hundred.

Two properties carry most of the weight:

- **Withheld means absent.** Every gated statistic must be `None` when its claim is refused, not
  populated-with-a-flag. A consumer that has to remember to check a flag is one that will forget.
- **Pooling agrees with counting.** The values averaged under an `n` must be the values that `n`
  counted — see `test_corpus.py` for the end-to-end version of this over a corpus read off disk.

Corpora are built by constructing the contracts directly. There is no disk here and no fixtures:
the reader has its own tests, and mixing the two would make a guard failure look like a reader
failure.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.analysis.baseline import build_baseline, pooled_samples
from golf_coach.contracts.baseline import BaselineClaim, minimum_n
from golf_coach.contracts.career import CareerCorpus, CorpusSwing
from golf_coach.contracts.swing import Measurement

POSE = "pose:face_on"
LM = "launch_monitor:hd_golf"


def _at(day: int) -> datetime:
    return datetime(2026, 8, day, 12, tzinfo=UTC)


def _measurement(name: str, value: float, *, source: str = POSE) -> Measurement:
    unit = "degrees" if source == LM else "shoulder_widths"
    return Measurement(name=name, value=value, unit=unit, source=source, detail="test")


def _swing(
    index: int,
    *,
    values: dict[str, float] | None = None,
    source: str = POSE,
    session_id: str | None = None,
    face_on: str | None = None,
    shot: str | None = None,
    analyzed: bool = True,
    stale: bool = False,
    outdated: bool = False,
    shot_needs_review: bool = False,
) -> CorpusSwing:
    """One distinct swing. Defaults give every swing its own clip, photo and session."""
    return CorpusSwing(
        player_id="aaron",
        session_id=session_id or f"2026-08-{index:02d}",
        # Unique per swing even when several share a session — `ref` is `session/swing`, and two
        # swings colliding on it would silently read as one.
        swing_id=str(index),
        captured_at=_at(index),
        face_on_sha256=face_on or f"clip-{index}",
        shot_sha256=shot or f"photo-{index}",
        measurements=[
            _measurement(name, value, source=source) for name, value in (values or {}).items()
        ],
        analyzed=analyzed,
        stale=stale,
        outdated=outdated,
        shot_needs_review=shot_needs_review,
    )


def _corpus(*swings: CorpusSwing) -> CareerCorpus:
    return CareerCorpus(player_id="aaron", swings=list(swings))


def _n_swings(n: int, metric: str = "head_sway_norm", **kwargs) -> CareerCorpus:
    """`n` distinct swings, each in its own session, each carrying one measurement."""
    return _corpus(*(_swing(i, values={metric: 0.2 + i * 0.01}, **kwargs) for i in range(1, n + 1)))


# --------------------------------------------------------------------------- the refusal


def test_the_guard_refuses_below_the_floor_and_says_what_it_needs() -> None:
    """The state on disk today: two swings, and nothing sayable about any of it."""
    baseline = build_baseline(_n_swings(2))

    metric = baseline.metrics["head_sway_norm"]
    assert baseline.nothing_sayable
    assert metric.n == 2
    assert metric.ready == []

    refusals = {refusal.claim: refusal for refusal in metric.withheld}
    assert set(refusals) == set(BaselineClaim)
    assert refusals[BaselineClaim.CENTER].have_n == 2
    assert refusals[BaselineClaim.CENTER].need_n == 5
    assert "5 samples" in refusals[BaselineClaim.CENTER].reason
    assert "there are 2" in refusals[BaselineClaim.CENTER].reason


def test_a_withheld_claim_leaves_no_number_to_render() -> None:
    """Withheld means absent, not flagged — `Measurement`'s principle one level up.

    This is the test that would catch someone "helpfully" populating the mean anyway and gating
    only the display, which puts the guard one forgotten conditional away from being decorative.
    """
    metric = build_baseline(_n_swings(2)).metrics["head_sway_norm"]

    assert metric.mean is None
    assert metric.mean_ci is None
    assert metric.median is None
    assert metric.sd is None
    assert metric.sd_ci is None
    assert metric.minimum is None
    assert metric.maximum is None
    assert all(session.mean is None for session in metric.sessions)


def test_the_evidence_behind_a_refusal_is_always_present() -> None:
    """A refusal has to be actionable: how much data there is, and from how many occasions."""
    metric = build_baseline(_n_swings(3)).metrics["head_sway_norm"]

    assert metric.n == 3
    assert metric.n_sessions == 3
    assert [session.n for session in metric.sessions] == [1, 1, 1]
    assert [session.session_id for session in metric.sessions] == [
        "2026-08-01",
        "2026-08-02",
        "2026-08-03",
    ]


# --------------------------------------------------------------------------- per-claim grain


def test_center_can_be_ready_while_spread_is_still_withheld() -> None:
    """The reason the gate is per (metric, claim) and not per metric.

    At n=6 the mean is resolved to about half a personal spread, but the sample sd still carries
    ~32% relative error — enough to report a typical value, not enough to call it repeatable.
    """
    metric = build_baseline(_n_swings(6)).metrics["head_sway_norm"]

    assert metric.supports(BaselineClaim.CENTER)
    assert not metric.supports(BaselineClaim.SPREAD)

    assert metric.mean is not None and metric.mean_ci is not None and metric.median is not None
    assert metric.sd is None and metric.sd_ci is None
    assert metric.minimum is None and metric.maximum is None


def test_spread_opens_at_its_own_floor_and_brings_the_range_with_it() -> None:
    metric = build_baseline(_n_swings(10)).metrics["head_sway_norm"]

    assert metric.supports(BaselineClaim.SPREAD)
    assert metric.sd is not None and metric.sd_ci is not None
    assert metric.minimum == pytest.approx(0.21)
    assert metric.maximum == pytest.approx(0.30)
    assert metric.sd_ci.low < metric.sd < metric.sd_ci.high


def test_a_trend_needs_occasions_not_swings() -> None:
    """Twelve swings hit in one bay hour are one occasion, however many of them there are."""
    swings = [
        _swing(i, values={"head_sway_norm": 0.2 + i * 0.01}, session_id="2026-08-01")
        for i in range(1, 13)
    ]
    metric = build_baseline(_corpus(*swings)).metrics["head_sway_norm"]

    assert metric.n == 12
    assert metric.n_sessions == 1
    assert metric.supports(BaselineClaim.CENTER)
    assert metric.supports(BaselineClaim.SPREAD)
    assert not metric.supports(BaselineClaim.TREND)

    refusal = next(r for r in metric.withheld if r.claim is BaselineClaim.TREND)
    assert refusal.have_sessions == 1
    assert refusal.need_sessions == 3
    assert "3 sessions" in refusal.reason
    assert "12 samples" not in refusal.reason, "the sample floor was met; only sessions are short"


def test_a_ready_trend_fills_in_the_per_session_means() -> None:
    swings = [
        _swing(i, values={"head_sway_norm": 0.2 + i * 0.01}, session_id=f"2026-08-{i % 3:02d}")
        for i in range(1, 13)
    ]
    metric = build_baseline(_corpus(*swings)).metrics["head_sway_norm"]

    assert metric.n == 12
    assert metric.n_sessions == 3
    assert metric.supports(BaselineClaim.TREND)
    assert all(session.mean is not None for session in metric.sessions)
    assert sum(session.n for session in metric.sessions) == 12


# --------------------------------------------------------------------------- per-metric floors


def test_a_noisier_metric_stays_silent_where_a_cleaner_one_speaks() -> None:
    """`tempo_ratio` needs 8 where `head_sway_norm` needs 5, and the gap is recorded, not vibes.

    Tempo is the noisiest instrument in the panel (spread/error 2.4 against 8.2) and the only
    metric depending on the address instant, which carries a median error of 7 frames.
    """
    swings = [
        _swing(i, values={"head_sway_norm": 0.2 + i * 0.01, "tempo_ratio": 2.4 + i * 0.05})
        for i in range(1, 7)
    ]
    baseline = build_baseline(_corpus(*swings))

    assert baseline.metrics["head_sway_norm"].supports(BaselineClaim.CENTER)
    assert not baseline.metrics["tempo_ratio"].supports(BaselineClaim.CENTER)
    assert minimum_n("tempo_ratio", BaselineClaim.CENTER) == 8
    assert minimum_n("head_sway_norm", BaselineClaim.CENTER) == 5


def test_an_unknown_metric_falls_back_to_the_default_floor() -> None:
    """A metric added tomorrow is gated by default rather than ungated by omission."""
    assert minimum_n("some_future_metric", BaselineClaim.CENTER) == 5
    assert minimum_n("some_future_metric", BaselineClaim.SPREAD) == 10


# --------------------------------------------------------------------------- what may be pooled


def test_a_swing_that_counts_toward_no_metric_moves_no_statistic() -> None:
    """Unanalyzed, stale and outdated each contribute nothing — and nothing is not zero.

    A swing pooled as 0.0 would drag the mean and inflate the spread. The point of excluding an
    outdated artifact is that pooling two engine generations manufactures variance out of a code
    change, which is the mirror of counting a re-upload twice.
    """
    good = [_swing(i, values={"head_sway_norm": 0.25}) for i in range(1, 6)]
    bad = [
        _swing(10, values={"head_sway_norm": 9.9}, analyzed=False),
        _swing(11, values={"head_sway_norm": 9.9}, stale=True),
        _swing(12, values={"head_sway_norm": 9.9}, outdated=True),
    ]
    metric = build_baseline(_corpus(*good, *bad)).metrics["head_sway_norm"]

    assert metric.n == 5
    assert metric.mean == 0.25


def test_re_uploads_of_one_clip_contribute_one_pose_sample() -> None:
    """The variance-collapsing failure, at the pooling layer rather than the counting layer."""
    swings = [
        _swing(i, values={"head_sway_norm": 0.25}, face_on="same-clip", shot=f"photo-{i}")
        for i in range(1, 6)
    ]
    baseline = build_baseline(_corpus(*swings))

    assert baseline.metrics["head_sway_norm"].n == 1
    assert not baseline.metrics["head_sway_norm"].supports(BaselineClaim.CENTER)


def test_two_swings_sharing_a_shot_photo_are_two_pose_readings_and_one_shot_reading() -> None:
    """One physical swing produced one ball flight; the clip is what makes it two swings."""
    swings = [
        _swing(1, values={"head_sway_norm": 0.21}, face_on="clip-a", shot="one-photo"),
        _swing(2, values={"head_sway_norm": 0.29}, face_on="clip-b", shot="one-photo"),
    ]
    for swing in swings:
        swing.measurements.append(_measurement("face_to_path_deg", 10.9, source=LM))

    baseline = build_baseline(_corpus(*swings))

    assert baseline.metrics["head_sway_norm"].n == 2
    assert baseline.metrics["face_to_path_deg"].n == 1


def test_a_flagged_shot_contributes_no_launch_monitor_value() -> None:
    """The rule `mcp.query.get_session_summary` already applies before averaging a shot metric."""
    swings = [
        _swing(
            i,
            values={"face_to_path_deg": 10.0 + i},
            source=LM,
            shot_needs_review=(i == 1),
        )
        for i in range(1, 4)
    ]
    metric = build_baseline(_corpus(*swings)).metrics["face_to_path_deg"]

    assert metric.n == 2
    assert 11.0 not in [sample.value for sample in pooled_samples(_corpus(*swings))[metric.name]]


def test_a_launch_monitor_metric_needs_a_shot_photo_to_dedupe_on() -> None:
    """No photo hash means no artifact to key the reading on, so it is not a reading."""
    swing = _swing(1, values={"face_to_path_deg": 10.9}, source=LM)
    swing.shot_sha256 = None

    assert "face_to_path_deg" not in build_baseline(_corpus(swing)).metrics


def test_an_unrecognised_source_falls_back_to_swing_identity() -> None:
    """Conservative: it can over-count against a real artifact key, never under-count."""
    swings = [_swing(i, values={"future_metric": float(i)}, source="wearable:imu") for i in (1, 2)]

    assert build_baseline(_corpus(*swings)).metrics["future_metric"].n == 2


# --------------------------------------------------------------------------- assembly


def test_the_baseline_reports_what_it_was_built_from() -> None:
    """Not `distinct_swings`: a swing that contributed nothing does not explain any `n`."""
    swings = [
        _swing(1, values={"head_sway_norm": 0.21}, session_id="2026-08-01"),
        _swing(2, values={"head_sway_norm": 0.25}, session_id="2026-08-01"),
        _swing(3, values={"head_sway_norm": 0.29}, session_id="2026-08-02"),
        _swing(4, values={"head_sway_norm": 9.9}, outdated=True),
    ]
    baseline = build_baseline(_corpus(*swings))

    assert baseline.built_from_swings == 3
    assert baseline.built_from_sessions == 2


def test_provenance_survives_onto_the_baseline() -> None:
    baseline = build_baseline(_n_swings(2))
    metric = baseline.metrics["head_sway_norm"]

    assert metric.unit == "shoulder_widths"
    assert metric.source == POSE


def test_metrics_are_sorted_so_two_runs_agree() -> None:
    swings = [
        _swing(i, values={"tempo_ratio": 2.4, "head_sway_norm": 0.2, "hip_sway_norm": 0.3})
        for i in range(1, 3)
    ]
    names = list(build_baseline(_corpus(*swings)).metrics)

    assert names == sorted(names)


def test_an_empty_corpus_is_an_empty_baseline_not_an_error() -> None:
    """The first true answer about every golfer."""
    baseline = build_baseline(CareerCorpus(player_id="nobody"))

    assert baseline.metrics == {}
    assert baseline.nothing_sayable
    assert baseline.built_from_swings == 0


def test_a_swing_carrying_no_measurements_produces_no_metric() -> None:
    assert build_baseline(_corpus(_swing(1))).metrics == {}
