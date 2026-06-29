# ADR-009: Swing Scoring Model — Dual-Axis (Mechanics + Outcome) with Intent-Driven Policies

## Status
Accepted

## Date
2026-06-28

## Context
The core question the product must answer is: *what makes a swing "good," and good
compared to what?* ADR/ROADMAP M4 originally framed analysis as a single rule-based
score (0–100) from pose/detection/shot checkpoints. That framing has a hole: **"good"
is not one axis, and it is meaningless without intent.**

A perfectly-struck fade is a *bad* shot if the golfer was trying to hit it straight.
Conversely, a golfer drilling fundamentals for a repeatable swing may not care where
the ball finished at all. So judging a swing requires separating three things:

- **Mechanics** — are the fundamentals sound (pose-based checkpoints)? Largely
  intent-independent.
- **Outcome** — what did the ball actually do (shape, start line, distance, dispersion)?
  Launch-monitor side.
- **Intent** — what was the golfer *trying* to do?

"Good" = how well **outcome matches intent**, and/or how sound the **mechanics** are —
*weighted by what the golfer is practicing.* A single fixed score cannot express that.

## Options Considered

### Option A: Single blended 0–100 score
- **Pros**: Simplest to design and explain. One number.
- **Cons**: Fixed internal weighting of mechanics vs. outcome can't represent intent.
  Cannot express "good fade when I wanted straight = bad," or "ignore the result, grade
  my fundamentals." Forces a definition of "good" the user never agreed to.

### Option B: Mechanics-only for now, defer outcome + intent
- **Pros**: Smallest surface to design. Needs no launch monitor.
- **Cons**: Paints us into a corner — bolting on outcome/intent later means reworking the
  `SwingResult` contract and the scoring entry point. Throws away the project's most
  interesting capability (goal-aware practice).

### Option C: Dual-axis sub-scores combined by an intent-driven scoring policy (chosen)
- **Pros**: `mechanics_score` and `outcome_score` are computed independently; the
  **practice mode** the user selects picks a **scoring policy** (Strategy pattern) that
  weights them and frames the feedback. Intent parameterizes the *expected ranges* of
  outcome checkpoints (a fade has a different ideal face-to-path than a straight shot).
  Expresses every case the user raised. The pose-only PoC implements one policy
  (Fundamentals) while leaving the seam for the rest.
- **Cons**: More contract surface (a `PracticeGoal`, two sub-scores, a policy concept).
  Slight indirection in scoring.

## Decision
**Option C.** Scoring is dual-axis with an intent-driven policy.

### Concepts
- **`PracticeGoal` (intent)** — a new contract describing what the golfer wanted:
  `mode`, optional target shape (straight / draw / fade), optional `club`, optional
  `focus_checkpoint` (for drill mode). Selected at session level, overridable per shot.
- **Practice modes** (each maps to a scoring policy):
  - **Fundamentals** — grade mechanics only; outcome is informational. ("repeatable swing")
  - **Shot-shaping** — declare intended shape + club; grade = outcome-vs-intent (+ mechanics
    consistency). This is where a good fade you didn't intend scores low.
  - **Performance** — grade the result vs. benchmarks for that club/skill (distance,
    dispersion); mechanics informational.
  - **Drill** — spotlight one checkpoint; everything else informational.
- **Checkpoints stay generic.** The existing `CheckpointScore` (`observed` vs.
  `expected_low/high`) expresses *both* mechanics and outcome checkpoints. An outcome
  checkpoint like "shot shape" has `observed = face-to-path` and an expected range that
  **depends on the intent** — so intent flows in as range parameterization, not as a
  special case in the scoring code.

### Contract / code shape
```
contracts/intent.py     # PracticeGoal: mode + target shape + club + focus_checkpoint
analysis/
  merge.py              # align keypoints + detections + shot by timestamp
  phases.py             # segment the swing
  checkpoints/
    mechanics.py        # pose-based: tempo, posture, hip rotation, ...
    outcome.py          # shot-vs-intent: shape, start line, distance, dispersion
  scoring.py            # ScoringPolicy chosen by practice mode -> overall_score
```
- `analyze_swing(..., intent: PracticeGoal)` — intent is a parameter of analysis.
- `SwingResult` gains `mechanics_score`, `outcome_score`, and the `intent` it was judged
  against. `overall_score` becomes the **policy-weighted blend** of the two sub-scores.
- The M6 Claude coaching layer reads the `PracticeGoal` so advice is goal-aware.

### PoC boundary
The first iteration (see ROADMAP M4-PoC) implements **Fundamentals mode only**: pose-only
mechanics checkpoints, `outcome_score` left `None`, the Fundamentals policy = mechanics
100%. The `PracticeGoal` parameter and the two-sub-score `SwingResult` exist from day one
so the other modes/policies are additive, not a rewrite. The outcome axis can be built
against the existing `MockShotDataSource` before the Garmin R10 arrives (per ADR-007).

## Consequences
- The "good fade when I wanted straight = bad" case, and the "I don't care where it went"
  case, both fall out of the model naturally instead of being special-cased.
- `SwingResult` and `analyze_swing` carry intent and two sub-scores from the start, so no
  contract rework when outcome/shot-shaping land. (This supersedes the single-score framing
  implied in M4 of the original ROADMAP.)
- Adding a practice mode = adding a scoring policy, not touching the checkpoint evaluators.
- Outcome checkpoints depend on the launch monitor (M3) and club detection (M2); the
  Fundamentals PoC deliberately needs neither.
- Expected ranges become intent- and club-dependent — handled by the benchmark store in
  [ADR-010](010-benchmark-ranges.md).
