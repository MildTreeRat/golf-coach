"""Rule-based feedback. [M4-PoC, ranking M5]

Maps each `CheckpointScore` in a `SwingResult` to a plain-English `Tip`, **ordered so the thing
worth working on comes first**, plus a one-line headline naming it. Pure function, no I/O. The tip
text itself is authored in `analysis/checkpoints/mechanics.py` and passed through; the fuller rules
catalogue (phrasing tuned per phase) is M5 and LLM coaching is M6.

## Why ordering needs two different signals

A swing produces two kinds of checkpoint, and no single number ranks both:

- **Failures** are ranked by `score`. `_score_within_range` decays in band-widths, so a lower score
  is a bigger overshoot. The percentile *cannot* do this job: `ranges.json`'s bands were cut at the
  reference p10/p90 (ADR-012), and `Distribution.percentile_of` clamps to `[10, 90]` because the
  tails were never stored — so **every** failing checkpoint reports percentile 90 whether it missed
  by a hair or by triple. The percentile saturates exactly where the band ends.
- **Passes** are ranked by percentile, because `score` is *exactly 1.0* for every one of them and
  carries no information at all. This is the case that motivated the whole thing:
  `golf_swing-aaron-1` scores 100/100 with all three checkpoints in band, and only the percentile
  can say that its head sway is nonetheless higher than 83% of tour swings — the one thing on that
  swing worth watching.

So: failures first, worst overshoot leading; then passes, closest-to-the-edge leading. Severity
stays on `score` for the same saturation reason.
"""

from __future__ import annotations

from golf_coach.contracts.feedback import FeedbackPayload, Severity, Tip
from golf_coach.contracts.swing import CheckpointScore, SwingResult

# A checkpoint that failed but still scored at/above this is a minor miss; below it, major.
_MINOR_SCORE_FLOOR = 0.5

# Distance from the population median to the stored tail, in percentile points. `percentile_of`
# clamps at p10/p90, so 40 points is the full usable half-width of the distribution and a tail
# distance of 1.0 means "at or past the rail".
_TAIL_HALF_SPAN = 40.0

# A *passing* checkpoint this far into the tail is close enough to the edge to name in the headline
# when nothing has actually failed. 0.5 is the midpoint between the median and the rail — roughly
# the 70th percentile for a one-sided metric.
_WATCH_TAIL = 0.5


def _tail_distance(checkpoint: CheckpointScore) -> float | None:
    """How far into the population tail this observation sits: 0.0 at the median, 1.0 at the rail.

    Returns None when no percentile was resolved, so callers fall back rather than treating an
    unknown placement as a typical one.

    The two metric shapes need different arithmetic, which is what `one_sided` is for. For a
    one-sided metric (lower is strictly better) only the *upper* half is a fault, so sitting at the
    10th percentile is excellent and scores 0.0. For a two-sided metric (tempo) both rails are
    equally wrong, so the distance is symmetric about the median.
    """
    if checkpoint.percentile is None:
        return None
    offset = checkpoint.percentile - 50.0
    if checkpoint.one_sided:
        return max(0.0, offset / _TAIL_HALF_SPAN)
    return abs(offset) / _TAIL_HALF_SPAN


def _rank_key(checkpoint: CheckpointScore) -> tuple[int, float]:
    """Sort key putting the most actionable checkpoint first (ascending sort).

    Failures group ahead of passes, then each group is ordered by the signal that actually
    discriminates within it — see the module docstring for why those are different signals.
    """
    if not checkpoint.passed:
        return (0, checkpoint.score)
    return (1, -(_tail_distance(checkpoint) or 0.0))


def _severity_for(checkpoint: CheckpointScore) -> Severity:
    """INFO if it passed, else MINOR/MAJOR by how far outside the band it landed.

    Deliberately still on `score`, not on the percentile: past the band edge the percentile is
    pinned at the rail and cannot tell a near-miss from a gross one.
    """
    if checkpoint.passed:
        return Severity.INFO
    return Severity.MINOR if checkpoint.score >= _MINOR_SCORE_FLOOR else Severity.MAJOR


def _tip_for(checkpoint: CheckpointScore) -> Tip:
    text = checkpoint.message or f"{checkpoint.name}: score {checkpoint.score:.0%}."
    return Tip(checkpoint=checkpoint.name, text=text, severity=_severity_for(checkpoint))


#: Checkpoints that can go unscored for a reason **re-shooting the clip would not fix**, mapped to
#: the measurement that distinguishes the two causes and the remedy that actually applies.
#:
#: If the measurement is present the footage was readable, so the missing score is not a capture
#: problem and the standard "film it again" advice is wrong — actively so, since it sends the golfer
#: back to the bay over a form field. `head_stays_back` is the only such checkpoint: its sign is
#: camera-relative, so it needs `Golfer.handedness` and refuses without it.
#:
#: The two names are string literals rather than imports on purpose. `feedback` must not import
#: `analysis` (ADR-008) and this is the same problem `contracts.caveats` solves for the standing
#: warnings — shared vocabulary, no shared module. `tests/feedback/test_rules.py` pins both against
#: the real checkpoint so a rename cannot leave this silently unmatched.
_UNSCORED_REMEDY: dict[str, tuple[str, str]] = {
    "head_stays_back": (
        "head_hip_gain_norm",
        "The swing itself was measured fine - this checkpoint also needs to know which side you "
        "swing from, and no golfer is attributed to this swing. Pick a golfer for the session and "
        "it will score without re-filming.",
    ),
}


def _unmeasured_tip(name: str, measured: frozenset[str]) -> Tip:
    """Say plainly that a checkpoint was attempted and could not be scored.

    `overall_score` is a mean over the checkpoints that *did* produce a score, so without this the
    golfer cannot tell a swing graded on three things from one graded on two (ADR-013).

    The remedy is chosen from what is actually wrong. A checkpoint listed in `_UNSCORED_REMEDY`
    whose measurement came through was blocked by something the camera cannot fix, and telling that
    golfer to steady their camera would be a confident answer to a question they did not ask.
    """
    entry = _UNSCORED_REMEDY.get(name)
    if entry is not None and entry[0] in measured:
        advice = entry[1]
    else:
        advice = (
            "This usually means an instant of the swing wasn't clearly detected - try a clip with "
            "the whole swing in frame and the camera steady."
        )
    return Tip(
        checkpoint=name,
        text=(
            f"{name.replace('_', ' ').capitalize()} could not be measured on this swing, so it is "
            f"not included in the score. {advice}"
        ),
        severity=Severity.INFO,
    )


def _headline(checkpoints: list[CheckpointScore]) -> str | None:
    """The single thing to work on, or a clean bill with the closest call named.

    None when there is nothing to say (no checkpoints scored at all) — an empty headline is a
    truthful "we have no verdict", which the caller renders differently from "everything is fine".
    """
    if not checkpoints:
        return None

    failures = [checkpoint for checkpoint in checkpoints if not checkpoint.passed]
    if failures:
        worst = min(failures, key=lambda checkpoint: checkpoint.score)
        label = worst.name.replace("_", " ")
        return f"Work on {label} first. {worst.message}"

    closest = max(checkpoints, key=lambda checkpoint: _tail_distance(checkpoint) or 0.0)
    clean = f"All {len(checkpoints)} checkpoints are inside tour range."
    if (_tail_distance(closest) or 0.0) >= _WATCH_TAIL:
        return f"{clean} Closest to the edge: {closest.name.replace('_', ' ')}."
    return clean


def build_feedback(result: SwingResult) -> FeedbackPayload:
    """Produce ranked rule-based tips from a swing result (LLM coaching added in M6)."""
    ranked = sorted(result.checkpoint_scores, key=_rank_key)
    measured = frozenset(measurement.name for measurement in result.measurements)
    return FeedbackPayload(
        swing_id=result.swing_id,
        overall_score=result.overall_score,
        headline=_headline(result.checkpoint_scores),
        tips=[_tip_for(checkpoint) for checkpoint in ranked]
        + [_unmeasured_tip(name, measured) for name in result.unscored],
    )
