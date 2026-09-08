"""`CorpusSwing`'s mishit flag and the metric-scoped hole it opens in `artifact_key`. [M16 P2]

The disk-level behaviour — auto-flagging in `read_corpus`, the printed `n` staying equal to the
pooled `n` with a mishit present — is pinned in `tests/storage/test_corpus.py`, where the fixtures
that build a corpus live. This file pins the contract in isolation: the truth table that turns two
verdict fields plus an auto flag into one bool, and the fact that the bool withholds `carry` and
`total` and *nothing else* on the same shot.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.contracts.career import CorpusSwing
from golf_coach.contracts.mishit import MishitVerdict
from golf_coach.contracts.swing import Measurement

_NOW = datetime(2026, 9, 1, 12, 0, tzinfo=UTC)


def _swing(**kw: object) -> CorpusSwing:
    base: dict[str, object] = dict(
        player_id="aaron",
        session_id="2026-09-01",
        swing_id="1",
        captured_at=_NOW,
        face_on_sha256="clip-a",
        shot_sha256="photo-1",
    )
    base.update(kw)
    return CorpusSwing(**base)  # type: ignore[arg-type]


def _lm(name: str) -> Measurement:
    return Measurement(name=name, value=150.0, unit="yards", source="launch_monitor:hd_golf")


@pytest.mark.parametrize(
    ("manual", "auto", "expected"),
    [
        (None, False, False),
        (None, True, True),  # auto-flagged, no verdict yet
        (MishitVerdict.CONFIRMED, False, True),  # golfer overrides a rule that stayed quiet
        (MishitVerdict.CONFIRMED, True, True),
        (MishitVerdict.CLEARED, True, False),  # golfer overrides the auto flag
        (MishitVerdict.CLEARED, False, False),
    ],
)
def test_the_verdict_wins_over_the_auto_flag_both_ways(
    manual: MishitVerdict | None, auto: bool, expected: bool
) -> None:
    assert _swing(manual_mishit=manual, auto_mishit=auto).is_mishit is expected


def test_a_mishit_withholds_carry_and_total_and_keeps_every_other_metric() -> None:
    """Metric-scoped: the same shot still counts toward ball speed, launch and offline."""
    swing = _swing(auto_mishit=True)

    assert swing.artifact_key(_lm("carry_distance_yds")) is None
    assert swing.artifact_key(_lm("total_distance_yds")) is None

    for still_counts in ("ball_speed_mph", "launch_angle_deg", "start_line_offline_yds"):
        assert swing.artifact_key(_lm(still_counts)) == "shot:photo-1"

    pose = Measurement(name="head_sway_norm", value=0.2, unit="ratio", source="pose:face_on")
    assert swing.artifact_key(pose) == "pose:clip-a"


def test_a_cleared_shot_counts_its_carry_like_any_other() -> None:
    swing = _swing(auto_mishit=True, manual_mishit=MishitVerdict.CLEARED)
    assert swing.artifact_key(_lm("carry_distance_yds")) == "shot:photo-1"


def test_a_flagged_parse_still_wins_over_a_mishit_that_would_have_kept_carry() -> None:
    """`shot_needs_review` is checked first, so a cleared mishit on a bad parse still contributes
    nothing — the parse, not the strike, is the reason."""
    swing = _swing(
        auto_mishit=True, manual_mishit=MishitVerdict.CLEARED, shot_needs_review=True
    )
    assert swing.artifact_key(_lm("carry_distance_yds")) is None
    assert swing.artifact_key(_lm("ball_speed_mph")) is None
