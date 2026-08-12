"""The personal-vs-tour join, and the three metrics it refuses. [Career mode, step 6]

The statistics are pinned in `test_stats.py`, the guard in `test_baseline.py` and the findings in
`test_dispersion.py`. What is tested here is the **placement**: that a center is put in the tour
population only when the data supports putting it anywhere, and that the two questions this join
must not answer stay unanswered.

The failure this file exists to catch is a comparison that means nothing while looking exactly like
one that does. Three of the eight metrics have no tour population they may be placed in, and one of
those three *has a stored distribution* — so "did we get a row back" is not the test, and a naive
join would produce a confident percentile for `head_hip_offset_impact_norm` against a population
that mixes both handednesses.

Corpora are built from contracts directly, no disk, same as `test_baseline.py` — so a join failure
can never be mistaken for a reader failure. The tour side is the real packaged `golfdb_v1.json`,
because pinning against a fixture would let the two drift and this module's entire job is agreeing
with that file.
"""

from __future__ import annotations

import ast
from datetime import UTC, datetime
from pathlib import Path

import pytest

import golf_coach.analysis
from golf_coach.analysis.baseline import build_baseline
from golf_coach.analysis.benchmarks import load_distribution
from golf_coach.analysis.comparison import build_standing, comparison_for
from golf_coach.contracts.baseline import BaselineClaim
from golf_coach.contracts.career import CareerCorpus, CorpusSwing
from golf_coach.contracts.comparison import Standing
from golf_coach.contracts.swing import Measurement

POSE = "pose:face_on"
LM = "launch_monitor:hd_golf"

#: Enough swings over enough sessions to clear the CENTER floor for every metric in the panel.
#: `tempo_ratio` has the highest at 8, so 12 clears them all with room to spare.
ENOUGH = 12


def _measurement(name: str, value: float) -> Measurement:
    unit = "degrees" if name.endswith("_deg") else "shoulder_widths"
    source = LM if name.endswith("_deg") else POSE
    return Measurement(name=name, value=value, unit=unit, source=source, detail="test")


def _corpus(metric: str, values: list[float]) -> CareerCorpus:
    """One swing per value, spread over three sessions, each with its own clip and photo."""
    swings = [
        CorpusSwing(
            player_id="aaron",
            session_id=f"2026-09-{(i % 3) + 1:02d}",
            swing_id=str(i),
            captured_at=datetime(2026, 9, (i % 3) + 1, 12, i, tzinfo=UTC),
            face_on_sha256=f"clip-{i}",
            shot_sha256=f"photo-{i}",
            measurements=[_measurement(metric, value)],
            analyzed=True,
        )
        for i, value in enumerate(values)
    ]
    return CareerCorpus(player_id="aaron", swings=swings)


def _tight(center: float, n: int = ENOUGH, half: float = 0.002) -> list[float]:
    """`n` values with mean exactly `center` and an interval narrow enough to decide a side.

    Alternating rather than random: a test that can flake on a seed is a test that gets rerun
    until it passes.
    """
    return [center + (half if i % 2 == 0 else -half) for i in range(n)]


def _one(metric: str, values: list[float]):
    return build_standing(_corpus(metric, values)).metrics[metric]


# ------------------------------------------------------------------- placing a center


def test_a_center_inside_the_band_is_placed_inside() -> None:
    band = load_distribution("head_sway_norm")
    assert band is not None

    metric = _one("head_sway_norm", _tight(band.p50))

    assert metric.standing is Standing.INSIDE
    assert metric.band_low == band.p10 and metric.band_high == band.p90
    assert metric.population_players == band.n_players
    assert metric.outside_by is None, "nothing is past an edge it did not cross"


def test_a_center_past_p90_is_placed_above_with_the_distance() -> None:
    band = load_distribution("head_sway_norm")
    assert band is not None

    metric = _one("head_sway_norm", _tight(band.p90 + 0.2))

    assert metric.standing is Standing.OUTSIDE
    assert metric.outside_by is not None and metric.outside_by > 0
    assert round(metric.outside_by, 6) == round(band.p90 + 0.2 - band.p90, 6)


def test_a_center_below_p10_is_outside_on_the_good_side() -> None:
    """The sign of `outside_by` is the whole difference between "better than tour" and "worse".

    `head_sway_norm` is a one-sided magnitude on a `[0, high]` band, so a center under p10 is less
    head movement than the tour population, not more. `Standing.OUTSIDE` cannot distinguish the two
    and is not supposed to — every consumer reads the sign, and this pins that the sign is there to
    read. Rendering the page without it was the defect this test was written from.
    """
    band = load_distribution("head_sway_norm")
    assert band is not None

    metric = _one("head_sway_norm", _tight(band.p10 / 2, half=0.001))

    assert metric.standing is Standing.OUTSIDE
    assert metric.outside_by is not None and metric.outside_by < 0


def test_an_interval_crossing_the_edge_settles_nothing() -> None:
    """The reason the standing is read off the interval and never off the mean.

    A center sitting exactly on p90 is one swing away from either side, and reporting `OUTSIDE`
    there is a placement that flips on the next shot. `STRADDLES` is the honest answer and it is
    not "borderline" — it is unresolved.
    """
    band = load_distribution("head_sway_norm")
    assert band is not None

    metric = _one("head_sway_norm", _tight(band.p90, half=0.05))

    assert metric.standing is Standing.STRADDLES
    assert metric.outside_by is None, "a distance past an edge asserts a side was established"


def test_the_percentile_reports_its_own_clamp() -> None:
    """`percentile_of` stops at the stored quantiles, so a rail value is a floor and not a rank.

    Reported rather than left for a reader to infer from the value 90.0, which is exactly the
    inference `contracts.swing` records as having been got wrong before.
    """
    band = load_distribution("head_sway_norm")
    assert band is not None

    extreme = _one("head_sway_norm", _tight(band.p90 + 1.0))
    middling = _one("head_sway_norm", _tight(band.p50))

    assert extreme.percentile == 90.0 and extreme.percentile_clamped
    assert middling.percentile is not None and not middling.percentile_clamped


# ------------------------------------------------------------------- the two refusals


def test_below_the_center_floor_nothing_is_placed_and_the_refusal_carries() -> None:
    """The step-4 guard reaching one layer further out, unchanged.

    There is no mean *in the input* to place, so this is absence rather than a comparison this
    module declined to make — and the refusal that arrives is the same object step 4 built, not a
    second copy phrased differently.
    """
    metric = _one("head_sway_norm", [0.20, 0.21])

    assert metric.standing is Standing.WITHHELD
    assert metric.center is None and metric.percentile is None
    assert [r.claim for r in metric.withheld] == [BaselineClaim.CENTER]
    assert metric.band_low is not None, "the band exists; it is the golfer's n that does not"


def test_a_launch_monitor_metric_has_no_population_and_says_why() -> None:
    """GolfDB is pose estimated from broadcast video, so there is no ball flight in it at all.

    Refused on `Measurement.source` rather than on a list of metric names, so a launch-monitor
    metric added tomorrow inherits the reason instead of the generic one.
    """
    metric = _one("face_to_path_deg", _tight(8.6, half=0.5))

    assert metric.standing is Standing.WITHHELD
    assert metric.band_low is None
    assert len(metric.unavailable) == 1
    assert "no tour population exists" in metric.unavailable[0]
    assert not metric.withheld, "an n refusal here would send someone to the bay for nothing"


def test_head_hip_offset_is_refused_although_a_distribution_exists() -> None:
    """The one metric a personal baseline can read and a tour band cannot.

    Its sign is camera-relative; a personal corpus is single-handed by construction and GolfDB is
    not, which is why M6.5 blocked it from becoming a checkpoint. A stored distribution existing is
    not the same as a stored distribution meaning something, and this is the case that separates
    "did we get a row back" from "is the row worth anything".
    """
    assert load_distribution("head_hip_offset_impact_norm") is not None, (
        "the premise of this test is that the row exists and must not be used"
    )

    metric = _one("head_hip_offset_impact_norm", _tight(0.14, half=0.01))

    assert metric.standing is Standing.WITHHELD
    assert metric.percentile is None
    assert any("mixed-handedness" in reason for reason in metric.unavailable)


def test_the_spread_is_never_placed_against_the_tour_spread() -> None:
    """A category error, recorded rather than computed.

    The tour `sd` is between-player variation over a whole field; a personal `sd` is one golfer's
    shot-to-shot repeatability. Comparing them would flatter every golfer alive, because one person
    always varies less than a field does. Said only once the golfer *has* a spread, since that is
    when its absence becomes a question worth answering.
    """
    with_spread = _one("head_sway_norm", _tight(0.20, n=ENOUGH))
    without = _one("head_sway_norm", _tight(0.20, n=6))

    assert any("shot to shot" in reason for reason in with_spread.unavailable)
    assert not any("shot to shot" in reason for reason in without.unavailable)


def test_an_unregistered_metric_gets_the_generic_reason() -> None:
    metric = _one("invented_metric_norm", _tight(1.0))

    assert metric.standing is Standing.WITHHELD
    assert metric.unavailable == [
        "invented_metric_norm: no reference distribution is stored for it"
    ]


# ------------------------------------------------------------------- the whole golfer


def test_the_standing_agrees_with_the_baseline_on_n() -> None:
    """Four layers, one `n`. The equality `test_corpus.py` pins for the reader and the baseline,
    extended to the join — a placement counted over a different number of swings than the mean it
    places would be wrong only where the dedupe rule matters."""
    corpus = _corpus("head_sway_norm", _tight(0.20))
    baseline = build_baseline(corpus)
    standing = build_standing(corpus)

    for name, metric in baseline.metrics.items():
        assert standing.metrics[name].n == metric.n
        assert standing.metrics[name].n_sessions == metric.n_sessions


def test_nothing_placed_is_the_state_on_disk() -> None:
    standing = build_standing(_corpus("head_sway_norm", [0.20, 0.21]))

    assert standing.nothing_placed
    assert standing.placements == 0


def test_comparison_for_reads_the_guarded_baseline_not_the_samples() -> None:
    """`comparison_for` takes a `MetricBaseline` and nothing else.

    That signature is the design: when CENTER was refused the mean is absent from the input, so
    there is no path by which a placement can be produced from raw values by a caller who forgot to
    check. Passing a hand-built baseline with a mean but no interval must still refuse.
    """
    baseline = build_baseline(_corpus("head_sway_norm", _tight(0.20))).metrics["head_sway_norm"]
    baseline.mean_ci = None

    assert comparison_for(baseline).standing is Standing.WITHHELD


# ------------------------------------------------------------------- the import boundary


@pytest.mark.parametrize("module", ["baseline", "dispersion"])
def test_the_guarded_modules_still_import_no_benchmarks(module: str) -> None:
    """The boundary step 6 moved out by one layer rather than dissolving.

    `analysis.baseline` and `analysis.dispersion` both state in their docstrings that they import
    no `benchmarks`, and that is what stops a personal statistic from quietly becoming a change to
    how a swing is scored (ADR-010 §2). The join has to import it, so it lives in its own module —
    and this is the assertion that the convenient one-line import never gets added to the other two.

    **Read statically, and it has to be.** The obvious version of this test — import the module in
    a subprocess and check `sys.modules` — cannot work and passes for the wrong reason if it ever
    appears to: `analysis/__init__.py` imports `engine`, which reads the bands, so importing
    *anything* under `golf_coach.analysis` pulls `benchmarks` in before the module body runs. The
    property being defended is what these two files themselves import, which is a fact about their
    source, so the source is what gets asserted on.
    """
    path = Path(golf_coach.analysis.__file__).parent / f"{module}.py"
    tree = ast.parse(path.read_text(encoding="utf-8"))

    imported = {
        name
        for node in ast.walk(tree)
        for name in (
            [node.module or ""]
            if isinstance(node, ast.ImportFrom)
            else [alias.name for alias in node.names]
            if isinstance(node, ast.Import)
            else []
        )
    }

    assert not any("benchmarks" in name for name in imported), (
        f"analysis.{module} imports analysis.benchmarks — the personal baseline path must not read "
        "tour data (ADR-010 §2). The join belongs in analysis.comparison, which is the one module "
        f"allowed to import both. Imports found: {sorted(imported)}"
    )
