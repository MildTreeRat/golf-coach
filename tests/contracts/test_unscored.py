"""The unscored-reason vocabulary: complete, layered, and reachable from a real swing.

Three properties, and each one has a failure mode that reaches a golfer rather than a test:

- a reason with no row in `UNSCORED_REASONS` raises mid-render, on a swing that was otherwise fine;
- a judging reason escaping into `measure.py` would mean the measure/judge split had quietly
  closed again, which is the fusion M6.5 spent a milestone undoing;
- a reason no evaluator can actually produce is dead vocabulary, and the caveat prose derived from
  this table would be telling every coaching model about a case that cannot happen.
"""

from __future__ import annotations

from golf_coach.contracts.unscored import (
    MEASUREMENT_REASONS,
    UNSCORED_REASONS,
    UnscoredCheckpoint,
    UnscoredReason,
)


def test_every_reason_has_a_row() -> None:
    """The exhaustiveness pin. `UnscoredCheckpoint.spec` indexes this table with no `.get`."""
    assert set(UNSCORED_REASONS) == set(UnscoredReason)


def test_every_row_says_something_in_both_registers() -> None:
    """`summary` is embedded in a sentence, `remedy` is a sentence. Both are shipped prose.

    The summary convention — no leading capital, no trailing full stop — is what lets
    `feedback/coach.py` and `mcp/query.py` place it mid-line without either of them editing it.
    """
    for reason, spec in UNSCORED_REASONS.items():
        assert spec.summary, f"{reason} has no summary"
        assert spec.remedy, f"{reason} has no remedy"
        assert not spec.summary.endswith("."), f"{reason}'s summary is punctuated as a sentence"
        assert spec.summary[0].islower(), f"{reason}'s summary starts mid-sentence with a capital"
        assert spec.remedy.endswith("."), f"{reason}'s remedy is not a sentence"


def test_the_reasons_that_are_not_capture_problems_are_the_expected_three() -> None:
    """`refilming_helps` is the one bit every consumer branches on, so it is pinned by name.

    Derived prose in `contracts.caveats` tells every coaching model which reasons must never be
    answered with "film it again". Getting this set wrong sends a golfer back to the bay over a
    form field — the exact failure the old `_UNSCORED_REMEDY` heuristic existed to avoid, now
    stated once instead of inferred.
    """
    not_capture = {reason for reason, spec in UNSCORED_REASONS.items() if not spec.refilming_helps}
    assert not_capture == {
        UnscoredReason.NO_BAND,
        UnscoredReason.NO_HANDEDNESS,
        UnscoredReason.UNRECORDED,
    }


def test_the_judging_reasons_are_exactly_the_ones_measure_may_not_report() -> None:
    """`MEASUREMENT_REASONS` and its complement partition the vocabulary — no member is homeless.

    A reason in neither set would be a cause nothing is allowed to produce; one in both would make
    the layering assertion in `tests/analysis/test_measure.py` vacuous.
    """
    judging = {UnscoredReason.NO_BAND, UnscoredReason.NO_HANDEDNESS, UnscoredReason.UNRECORDED}
    assert MEASUREMENT_REASONS | judging == set(UnscoredReason)
    assert not MEASUREMENT_REASONS & judging


def test_the_engine_never_writes_unrecorded() -> None:
    """`UNRECORDED` is a reader's answer, not a writer's.

    Seeing one means the artifact predates the reason being recorded. If the engine could emit it,
    "this file does not know" and "we did not bother to say" would be the same value, and
    `scripts/reanalyze.py` would stop being the fix for it.

    A test may import `analysis`; the module boundary binds `src`, not the pins on it.
    """
    from golf_coach.analysis.checkpoints import CHECKPOINT_EVALUATORS
    from golf_coach.contracts.checkpoints import CHECKPOINT_REGISTRY
    from golf_coach.contracts.intent import ClubCategory

    for spec in CHECKPOINT_REGISTRY:
        # An empty swing takes every evaluator down its earliest refusal path.
        judged = CHECKPOINT_EVALUATORS[spec.name]([], [], None, ClubCategory.ALL, None)
        assert judged.score is None
        assert judged.reason is not UnscoredReason.UNRECORDED


def test_spec_is_a_lookup_no_consumer_has_to_repeat() -> None:
    entry = UnscoredCheckpoint(name="tempo", reason=UnscoredReason.NO_BAND)
    assert entry.spec is UNSCORED_REASONS[UnscoredReason.NO_BAND]
    assert entry.detail == ""


# --------------------------------------------------------------------------------------
# Reading what is already on disk
# --------------------------------------------------------------------------------------


def test_a_names_only_result_still_loads() -> None:
    """Every `analysis.json` written before 2026-08-19 stores `unscored` as bare names.

    The coercion lives on the field so it happens once, and it must produce an entry rather than
    drop one: a swing judged on five fundamentals that starts reading as one judged on six has had
    `overall_score` silently redefined, which is the whole failure `unscored` exists to prevent.
    """
    from golf_coach.contracts.swing import SwingResult

    result = SwingResult.model_validate(
        {
            "swing_id": "s",
            "session_id": "sess",
            "overall_score": 61.0,
            "unscored": ["tempo", "head_stays_back"],
        }
    )

    assert [entry.name for entry in result.unscored] == ["tempo", "head_stays_back"]
    assert all(entry.reason is UnscoredReason.UNRECORDED for entry in result.unscored)
    assert all(entry.detail == "" for entry in result.unscored)


def test_the_two_forms_can_be_mixed_without_either_being_lost() -> None:
    """Not a shape the engine writes, but the one a hand-edited or partly-migrated file has."""
    from golf_coach.contracts.swing import SwingResult

    result = SwingResult.model_validate(
        {
            "swing_id": "s",
            "session_id": "sess",
            "overall_score": 61.0,
            "unscored": ["tempo", {"name": "hip_sway", "reason": "no_band", "detail": "d"}],
        }
    )

    assert [(e.name, e.reason) for e in result.unscored] == [
        ("tempo", UnscoredReason.UNRECORDED),
        ("hip_sway", UnscoredReason.NO_BAND),
    ]


def test_a_round_trip_through_json_keeps_the_reason() -> None:
    """What the pipeline actually does: build, serialize to `analysis.json`, read back later."""
    from golf_coach.contracts.swing import SwingResult

    original = SwingResult(
        swing_id="s",
        session_id="sess",
        overall_score=61.0,
        unscored=[
            UnscoredCheckpoint(
                name="head_stays_back",
                reason=UnscoredReason.NO_HANDEDNESS,
                detail="no golfer attributed",
            )
        ],
    )

    reloaded = SwingResult.model_validate_json(original.model_dump_json())
    assert reloaded.unscored == original.unscored
    assert reloaded.unscored[0].spec.refilming_helps is False
