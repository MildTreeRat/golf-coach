"""The three career tools, on a corpus that refuses and on one that speaks. [Career mode, step 6]

`test_baseline.py`, `test_dispersion.py` and `test_comparison.py` pin the guard, the findings and
the join over contracts built in memory. What is tested here is the trip **out**: that a claim the
guard refused arrives at a model as absent rather than as a number, that the evidence which stays
populated is labelled as evidence, and that the two tools ADR-006 deferred for want of sample size
are safe to offer now because the refusal travels with them.

**Both paths are tested and that is deliberate.** Everything on disk today refuses, so a suite that
only fed these tools the real shape would assert that eight metrics print nothing — and would pass
just as happily if the speaking branch were broken, unreachable, or wrong. Career mode step 5
shipped a coaching sentence that was nonsense under seven of its eight metrics with every test
green; it was caught by *rendering* the speaking path. `speaking_dir` is that path, in a test.

The MCP SDK is not imported here. These call `mcp.career` directly, which is the same boundary
`test_query.py` holds and what keeps them running on the base install.
"""

from __future__ import annotations

from datetime import UTC, datetime, timedelta
from pathlib import Path

import pytest

from golf_coach.mcp import career
from golf_coach.storage.manifest import load_manifest, manifest_path, save_manifest

# Values chosen so the panel shows every outcome at once rather than eight of the same:
#   head_sway_norm      tight at 0.20   -> inside tour range, and a bias (0.20 is not 0)
#   finish_balance_norm tight and tiny  -> below tour range, no bias (inside the tolerance)
#   hip_sway_norm       wide            -> scatter, and no target so no bias
#   tempo_ratio         tight at 5.4    -> above tour range (band 2.72-4.71)
#   face_to_path_deg    open and steady -> biased, and no tour population at all
_CENTERS = {
    "head_sway_norm": (0.200, 0.015),
    "hip_sway_norm": (0.300, 0.140),
    "hip_shift_at_top_norm": (0.110, 0.020),
    "head_hip_offset_impact_norm": (0.140, 0.030),
    "finish_balance_norm": (0.020, 0.004),
    "tempo_ratio": (5.400, 0.300),
    "face_to_path_deg": (8.600, 1.000),
    "start_line_deg": (0.400, 7.000),
}

#: 18 swings over 3 sessions clears every floor in the panel: CENTER tops out at 8
#: (`tempo_ratio`), SPREAD at 15, and TREND needs 18 samples across at least 3 sessions.
_SWINGS = 18
_SESSIONS = 3


def _values(i: int) -> dict[str, float]:
    """Alternating about each center, so a spread is exact and no test flakes on a seed."""
    return {
        name: center + (half if i % 2 == 0 else -half)
        for name, (center, half) in _CENTERS.items()
    }


@pytest.fixture
def refusing_dir(tmp_path: Path, swing_writer, career_writer) -> Path:
    """Two analyzed swings across two sessions — the shape actually on disk today."""
    root = tmp_path / "sessions"
    for i in range(2):
        swing_writer(
            root,
            f"2026-08-{i + 1:02d}",
            "1",
            player_id="aaron",
            created_at=datetime(2026, 8, i + 1, 12, tzinfo=UTC),
            analysis=career_writer(_values(i)),
        )
    return root


@pytest.fixture
def speaking_dir(tmp_path: Path, swing_writer, career_writer) -> Path:
    """18 swings across 3 sessions — a corpus every guard in the milestone lets through."""
    root = tmp_path / "sessions"
    for i in range(_SWINGS):
        swing_writer(
            root,
            f"2026-09-{(i % _SESSIONS) + 1:02d}",
            str(i),
            player_id="aaron",
            created_at=datetime(2026, 9, (i % _SESSIONS) + 1, 12, i, tzinfo=UTC),
            analysis=career_writer(_values(i)),
        )
    return root


def _metric(profile, name: str):
    return next(m for m in profile.metrics if m.name == name)


# ------------------------------------------------------------------ resolving the golfer


def test_an_unregistered_name_is_a_miss_on_every_tool(refusing_dir, golfers_dir) -> None:
    """None, so `server.py` can turn it into a `NotFound` that says what to do.

    A tool that returned an empty profile for an unknown name would have the model narrate "you
    have no swings on file" about a golfer whose only problem is a typo.
    """
    assert career.golfer_profile(refusing_dir, golfers_dir, "Nobody") is None
    assert career.shot_trends(refusing_dir, golfers_dir, "head_sway_norm", "Nobody") is None
    assert career.compare_sessions(refusing_dir, golfers_dir, "a", "b", "Nobody") is None


def test_a_typed_name_and_a_stored_id_reach_the_same_golfer(refusing_dir, golfers_dir) -> None:
    """`slugify` folding both forms is what stops one golfer becoming two half-histories."""
    typed = career.golfer_profile(refusing_dir, golfers_dir, "Aaron")
    stored = career.golfer_profile(refusing_dir, golfers_dir, "aaron")

    assert typed is not None and stored is not None
    assert typed.player_id == stored.player_id == "aaron"


# ------------------------------------------------------------------ the refusing path


def test_at_the_n_on_disk_the_profile_says_nothing_and_explains_itself(
    refusing_dir, golfers_dir, career_metrics
) -> None:
    profile = career.golfer_profile(refusing_dir, golfers_dir, "Aaron")
    assert profile is not None

    assert profile.nothing_sayable
    assert profile.note, "a refusal with no note is a silence the model has to interpret"
    assert len(profile.metrics) == len(career_metrics)

    for metric in profile.metrics:
        assert metric.center is None and metric.sd is None
        assert metric.bias == "withheld" and metric.scatter == "withheld"
        assert metric.tour_standing == "withheld"
        assert metric.withheld, f"{metric.name} refuses without saying what it waits for"


def test_a_refused_trend_ships_the_counts_labelled_as_evidence(refusing_dir, golfers_dir) -> None:
    """The exact combination this milestone has to survive.

    The per-session counts stay populated — they are what makes a refusal actionable — while every
    per-session mean is absent. A model handed both can average the counts itself, so `ready` is
    false, the note says not to, and `contracts.caveats.READING_A_PERSONAL_HISTORY` says it again.
    """
    trend = career.shot_trends(refusing_dir, golfers_dir, "head_sway_norm", "Aaron")
    assert trend is not None

    assert not trend.ready
    assert trend.sessions and all(point.mean is None for point in trend.sessions)
    assert all(point.n > 0 for point in trend.sessions)
    assert trend.withheld is not None and trend.withheld.claim == "trend"
    assert "not a series" in trend.note


def test_a_metric_the_golfer_has_never_recorded_is_empty_not_missing(
    refusing_dir, golfers_dir
) -> None:
    trend = career.shot_trends(refusing_dir, golfers_dir, "invented_metric", "Aaron")

    assert trend is not None, "'no samples of that' is an answer, not a miss"
    assert trend.n == 0 and not trend.sessions
    assert "No sample" in trend.note


# ------------------------------------------------------------------ the speaking path


def test_with_enough_swings_the_profile_speaks(speaking_dir, golfers_dir) -> None:
    profile = career.golfer_profile(speaking_dir, golfers_dir, "Aaron")
    assert profile is not None

    assert not profile.nothing_sayable
    assert profile.distinct_swings == _SWINGS
    assert profile.sessions == _SESSIONS

    head_sway = _metric(profile, "head_sway_norm")
    assert head_sway.n == _SWINGS
    assert head_sway.center == pytest.approx(0.200)
    assert head_sway.center_ci_low is not None and head_sway.center_ci_high is not None
    assert head_sway.sd is not None


def test_the_two_findings_come_out_as_different_things(speaking_dir, golfers_dir) -> None:
    """The argument the whole milestone rests on, end to end and through the wire shape.

    A repeatable miss and a scattered one must not read the same. `head_sway_norm` is held tight
    off zero; `hip_sway_norm` is deliberately wide.
    """
    profile = career.golfer_profile(speaking_dir, golfers_dir, "Aaron")
    assert profile is not None

    repeatable = _metric(profile, "head_sway_norm")
    scattered = _metric(profile, "hip_sway_norm")

    assert repeatable.bias == "established"
    assert repeatable.pattern == "biased"
    assert scattered.scatter == "established"
    assert repeatable.points_at != scattered.points_at


def test_the_tour_join_speaks_on_both_sides_of_the_band(speaking_dir, golfers_dir) -> None:
    """`tempo_ratio` at 5.4 is above the band; `finish_balance_norm` at 0.02 is below it.

    Both are `outside`, and `outside_by`'s sign is the only thing separating "more head movement
    than the tour" from "better balanced than the tour". A consumer reading the enum alone would
    render them identically, which is the defect this pair pins.
    """
    profile = career.golfer_profile(speaking_dir, golfers_dir, "Aaron")
    assert profile is not None

    above = _metric(profile, "tempo_ratio")
    below = _metric(profile, "finish_balance_norm")

    assert above.tour_standing == "outside" and above.outside_by is not None
    assert above.outside_by > 0
    assert below.tour_standing == "outside" and below.outside_by is not None
    assert below.outside_by < 0

    inside = _metric(profile, "head_sway_norm")
    assert inside.tour_standing == "inside"
    assert inside.tour_population_players is not None


def test_the_metrics_with_no_usable_population_stay_refused_even_when_speaking(
    speaking_dir, golfers_dir
) -> None:
    """More swings fix a `withheld`. They do not fix an `unavailable`, and the two must not merge.

    This is the assertion that the tour join did not quietly acquire the three metrics it is not
    allowed to place once the `n` arrived.
    """
    profile = career.golfer_profile(speaking_dir, golfers_dir, "Aaron")
    assert profile is not None

    for name in ("face_to_path_deg", "start_line_deg", "head_hip_offset_impact_norm"):
        metric = _metric(profile, name)
        assert metric.tour_standing == "withheld", f"{name} was placed in a forbidden population"
        assert metric.unavailable, f"{name} refuses the tour join without saying why"
        assert metric.center is not None, "the personal side of this metric is perfectly readable"


def test_a_trend_opens_only_once_the_sessions_are_there(speaking_dir, golfers_dir) -> None:
    trend = career.shot_trends(speaking_dir, golfers_dir, "head_sway_norm", "Aaron")
    assert trend is not None

    assert trend.ready
    assert trend.n_sessions == _SESSIONS
    assert all(point.mean is not None for point in trend.sessions)
    assert trend.withheld is None and not trend.note


def test_the_window_narrows_the_n_it_reports_on(
    tmp_path, golfers_dir, swing_writer, career_writer
) -> None:
    """A narrowed window has to narrow the `n` printed beside the values, not just the values.

    `storage.corpus.narrow_to` recomputes `metric_counts` rather than filtering the swing list
    beside a stale count. Without that, a 30-day window would report a sample size describing a
    different set of swings than the ones behind it — the same class of invisible disagreement
    step 4 found when the dedupe rule was private.

    Dated relative to the clock rather than to a fixed month, because a window is measured from
    `now` and a hardcoded date silently stops filtering the moment it falls inside one.
    """
    root = tmp_path / "sessions"
    now = datetime.now(UTC)
    for i in range(12):
        long_ago = i < 6
        when = now - timedelta(days=400 if long_ago else 3, minutes=i)
        swing_writer(
            root, f"{'old' if long_ago else 'new'}-{i}", "1", player_id="aaron",
            created_at=when, analysis=career_writer(_values(i)),
        )

    everything = career.shot_trends(root, golfers_dir, "head_sway_norm", "Aaron")
    windowed = career.shot_trends(root, golfers_dir, "head_sway_norm", "Aaron", days=30)

    assert everything is not None and windowed is not None
    assert everything.n == 12 and everything.window_days is None
    assert windowed.n == 6, "only the six recent swings are inside a 30-day window"
    assert windowed.window_days == 30
    assert len(windowed.sessions) == 6
    # The narrowed corpus is 6 swings over 6 sessions, which clears TREND's session floor but not
    # its sample floor — so the window refuses where the whole history speaks.
    assert not windowed.ready and everything.ready


# ------------------------------------------------------------------ comparing sessions


def test_two_sessions_are_gated_the_same_way_the_pooled_corpus_is(
    speaking_dir, golfers_dir
) -> None:
    """Six swings per session clears CENTER, so the deltas are real — and each side is guarded.

    The point is that no second threshold exists: a per-session mean faces the identical CENTER
    floor a pooled mean faces, asked of less data.
    """
    compared = career.compare_sessions(
        speaking_dir, golfers_dir, "2026-09-01", "2026-09-02", "Aaron"
    )
    assert compared is not None

    head_sway = next(m for m in compared.metrics if m.metric == "head_sway_norm")
    assert head_sway.n_a == 6 and head_sway.n_b == 6
    assert head_sway.mean_a is not None and head_sway.mean_b is not None
    assert head_sway.delta == pytest.approx(head_sway.mean_b - head_sway.mean_a)
    assert compared.note, "a comparison with no standing caveat invites a significance claim"


def test_at_the_n_on_disk_a_comparison_yields_counts_and_no_delta(
    refusing_dir, golfers_dir
) -> None:
    compared = career.compare_sessions(
        refusing_dir, golfers_dir, "2026-08-01", "2026-08-02", "Aaron"
    )
    assert compared is not None

    for metric in compared.metrics:
        assert metric.mean_a is None and metric.mean_b is None
        assert metric.delta is None
        assert metric.withheld, "a refused delta has to say what it is waiting for"


def test_a_session_that_scores_and_contributes_nothing_explains_itself(
    tmp_path, golfers_dir, swing_writer, career_writer
) -> None:
    """It reads as a contradiction and is not.

    Swings dedupe on the face-on clip's hash, so a session made of re-uploads scores normally while
    contributing to no `n` — the same physical swing already counted. Unexplained, "1 analyzed
    swing, 0 samples" reads as a broken reader, which is what `CareerCorpus.excluded` exists to
    prevent one layer down. It is also the shape actually on disk: three of the four stored swings
    are byte-identical re-uploads of one clip.
    """
    root = tmp_path / "sessions"
    for day in (1, 2):
        swing_writer(
            root, f"2026-08-{day:02d}", "1", player_id="aaron",
            created_at=datetime(2026, 8, day, 12, tzinfo=UTC),
            analysis=career_writer(_values(0)),
        )

    # Make the second session's clip the *same bytes* as the first's. `make_manifest` derives its
    # hashes from the session and swing id, so by default these are two distinct swings.
    original = load_manifest(manifest_path(root / "2026-08-01" / "1"))
    later_path = manifest_path(root / "2026-08-02" / "1")
    later = load_manifest(later_path)
    assert original is not None and later is not None
    later.roles = original.roles
    save_manifest(later, later_path)

    compared = career.compare_sessions(root, golfers_dir, "2026-08-01", "2026-08-02", "Aaron")

    assert compared is not None
    assert compared.swings_b == 1, "the re-upload was analyzed and does have a score"
    assert all(metric.n_b == 0 for metric in compared.metrics), "and contributes no sample"
    assert "contributes no samples" in compared.note
