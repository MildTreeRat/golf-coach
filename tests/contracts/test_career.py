"""`CorpusSwing`'s mishit flag and the metric-scoped hole it opens in `artifact_key`. [M16 P2]

The disk-level behaviour — auto-flagging in `read_corpus`, the printed `n` staying equal to the
pooled `n` with a mishit present — is pinned in `tests/storage/test_corpus.py`, where the fixtures
that build a corpus live. This file pins the contract in isolation: the truth table that turns two
verdict fields plus an auto flag into one bool, and the fact that the bool withholds `carry` and
`total` and *nothing else* on the same shot.

M17 P5 added the second hole in the same dispatch and it is a different kind: the mishit withholds a
sample that exists, while `POSE_DTL_SOURCE` withholds one whose artifact this shape cannot name at
all. They are pinned in one file because they are one function's two `None`s, and a reader meeting
either needs to know the other is not it.
"""

from __future__ import annotations

from datetime import UTC, datetime

import pytest

from golf_coach.contracts.career import (
    KNOWN_SOURCE_PREFIXES,
    POSE_DTL_SOURCE,
    CorpusSwing,
)
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


def test_the_second_camera_is_a_reading_of_a_clip_this_corpus_cannot_name() -> None:
    """The `pose:` source that keys on nothing, and the one beside it that keys normally. [M17 P5]

    `CorpusSwing` holds `face_on_sha256` and no hash for the rear clip. Under the prefix alone a
    `_dtl` row would take the face-on clip's key, so two different rear clips paired with one
    face-on clip would collapse to a single sample and the pooled value would be whichever was read
    first. Both halves are asserted together because the defect is the pair agreeing, not either
    being wrong alone — and the face-on half pooling normally is a decision (ADR-029's 2026-09-09b
    addendum §5), not an accident of the dispatch order.
    """
    swing = _swing()
    dtl = Measurement(
        name="pivot_hip_axis_drift_norm_dtl",
        value=0.31,
        unit="shoulder_widths",
        source=POSE_DTL_SOURCE,
    )
    face_on = Measurement(
        name="pivot_hip_axis_drift_norm",
        value=0.28,
        unit="shoulder_widths",
        source="pose:face_on",
    )

    assert swing.artifact_key(dtl) is None
    assert swing.artifact_key(face_on) == "pose:clip-a"


def test_the_second_camera_s_source_is_still_a_recognised_provenance() -> None:
    """It contributes no sample, and that is not the same as being unrecognised.

    `count_metrics` reports anything outside `KNOWN_SOURCE_PREFIXES` in `CareerCorpus
    .unknown_sources` so a new provenance cannot silently acquire the `swing:{ref}` fallback. A
    `pose:` source that keys on nothing must not turn up in that report: nothing about it is
    unaccounted for, and a reader chasing the entry would find a decision rather than a gap.
    """
    assert POSE_DTL_SOURCE.startswith(KNOWN_SOURCE_PREFIXES)


def test_a_flagged_parse_still_wins_over_a_mishit_that_would_have_kept_carry() -> None:
    """`shot_needs_review` is checked first, so a cleared mishit on a bad parse still contributes
    nothing — the parse, not the strike, is the reason."""
    swing = _swing(
        auto_mishit=True, manual_mishit=MishitVerdict.CLEARED, shot_needs_review=True
    )
    assert swing.artifact_key(_lm("carry_distance_yds")) is None
    assert swing.artifact_key(_lm("ball_speed_mph")) is None
