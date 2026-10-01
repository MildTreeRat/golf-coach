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

## Addendum (2026-09-30, M31): the outcome axis is graded over many shots, and intent decides which shots count

[ADR-034](034-shot-first-phone-first.md) makes the launch-monitor shot the product and video
optional. It keeps this decision's two axes and its `PracticeGoal`, and changes two things: where
the outcome axis is judged, and what the intent is for. This addendum records what that asks of
*this* decision. ADR-034's numbered clauses are the authority, and each point below routes to one
rather than restating it. **Nothing here is built**; M35 and M37 build it.

**The outcome axis is graded over many shots, never per swing**
([clause 5](034-shot-first-phone-first.md#5-grades-a-grade-and-a-list-per-topic)).

- Outcome becomes **per-topic grades at club and player level**: each a share of good shots with a
  Wilson interval, withheld below a minimum n, and combined into **two equal-weight blends**, one
  per club and one for the player
  ([5.4](034-shot-first-phone-first.md#54-two-blends-with-an-honest-pooling-rule)).
- **`SwingResult.outcome_score` stays `None`.** A share of one shot is 0 or 100 and means nothing.
  §PoC boundary left it `None` as a placeholder; it is now the decision.
- So Option C's per-swing *"policy-weighted blend of the two sub-scores"* is not built. ADR-034's
  blends average topics, not the two axes, and nothing anywhere weights mechanics against outcome.
  That meets Option A's objection (a fixed internal weighting nobody agreed to) by not weighting at
  all, which is a stronger form of this decision's separation rather than a retreat from it.

**The mechanics axis is optional, and a panel of its own**
([clause 1](034-shot-first-phone-first.md#1-the-unit-of-the-product-is-the-shot)). When video
exists, the pose checkpoints are scored exactly as today and sit beside the shot grades. They never
enter a topic grade or a blend, so adding video never moves a club grade, and a golfer with no
video is graded on the same terms as one with it.

**`PracticeGoal` gains a second job: whether a shot counts**
([clause 3](034-shot-first-phone-first.md#3-tracked-shots-are-derived-from-intent-and-a-drill-is-not-tracked)).

- `mode` decides whether a shot is **tracked**. A `DRILL` shot is not; every other mode is,
  `SHOT_SHAPING` and challenge-mode shots included.
- It is **derived from the mode and never stored** as a second flag, so the two cannot disagree.
- An untracked shot is still stored, shown and analysed on its own. It never enters club or player
  stats.
- This is not §Concepts' Drill policy ("spotlight one checkpoint"). That is a way to score one
  swing, and it stays unbuilt. What a drill now does is stay out of the aggregates, which is what
  the user asked for: "Some drills we do not want to add to the player stats as if they were
  actually swinging or actually playing."
- It is distinct from [ADR-028](028-mishit-exclusion.md)'s mishit rule, which is automatic and
  scoped to two metrics. This one is the golfer's choice, and it takes the whole shot.

**Shape intent is judged at last, as topics**
([5.3](034-shot-first-phone-first.md#53-shot-shapes)).

- `SHOT_SHAPING` with a `target_shape` is what the **fade**, **draw** and **straight** topics are
  graded against. The golfer declares it per session or per shot, or challenge mode calls it before
  each shot and records the call.
- **A shape never declared is absent**: not graded, not mentioned, and not in a blend.
- This is the case this ADR's Context opened with. A perfectly struck fade, declared straight, is
  not a "hit the shape" shot, and it lowers the straight topic's share.
- **It is not built the way §Concepts drew it.** Intent does not parameterize a checkpoint's
  expected range. It chooses which topic a shot is graded in, and one rule classifies every shot:
  face-to-path against `METRIC_TARGETS["face_to_path_deg"]`'s tolerance. So there is no
  `analysis/checkpoints/outcome.py` and no intent-keyed row in `ranges.json`; ADR-010's addendum of
  this date takes shot metrics out of that file. "Checkpoints stay generic" remains true of the
  mechanics panel, which is where the checkpoints are.

**The single-swing policies stay unbuilt.** Shot-shaping, performance and drill as
`ScoringPolicy`s are not needed by anything ADR-034 decides. Performance's "grade the result vs.
benchmarks for that club/skill" is superseded outright: shot metrics get no tour bands
([5.6](034-shot-first-phone-first.md#56-no-tour-bands-for-shot-metrics)), and a club's performance
is its topic grades beside its strengths-and-weaknesses list.

**Open, and whose: a mode other than `FUNDAMENTALS` cannot reach the engine today.**
`analysis/scoring.py::policy_for` raises `NotImplementedError` for every other mode,
`crates/analysis`' mirror panics, and `analysis/engine.py` calls it on every `analyze_swing` (M31 P2,
2026-09-29). So a `DRILL` or `SHOT_SHAPING` shot that has video needs either a policy for its mode,
or its mode carried beside the swing rather than into it.
[ADR-034's Consequences](034-shot-first-phone-first.md#consequences) give that choice to **M35**,
which records it by a further addendum here. One constraint binds either answer: clause 1 scores the
pose checkpoints exactly as today, and clause 5 leaves the single-swing policies unbuilt, so neither
answer is a new weighting. The mode decides whether a shot counts and which shape it is graded
against, not how a swing's mechanics are scored.

**Not changed**:

- `PracticeGoal`'s shape: `mode`, `target_shape`, `club` and `focus_checkpoint`, with the enums
  `contracts/intent.py` already has;
- "selected at session level, overridable per shot", which the pivot reuses as written, now for a
  photo-only shot as well as a swing;
- `SwingResult`'s two sub-scores and its `intent`, and the Fundamentals policy that fills them;
- the mechanics checkpoints and `ranges.json`.
