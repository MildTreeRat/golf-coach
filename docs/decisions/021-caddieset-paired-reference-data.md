# ADR-021: CaddieSet as a Paired Mechanics/Outcome Source

## Status
Accepted as a corpus — **and the study it was acquired for returned a negative result.** Kept, and
the negative result is the reason to keep it.

## Date
2026-08-16

## Context

[ADR-012](012-golfdb-reference-data.md) named the limit of the reference data plainly: GolfDB has
"no handicap, no shot outcome, no good/bad… it cannot teach a model to tell a good swing from a bad
one." [`contracts/comparison.py`](../../src/golf_coach/contracts/comparison.py)'s
`NO_LAUNCH_MONITOR_POPULATION` says the same in code, refusing to place `face_to_path_deg` or
`start_line_deg` in any population because every distribution here comes from pose over broadcast
video and none of it carries ball flight.

That is the gap between the two things this project measures. Mechanics are scored against a tour
population; outcome is recorded and never compared to anything. And the question underneath — *do
the six things we score actually matter?* — had no data that could answer it, because answering it
needs mechanics and outcome **on the same row**.

**CaddieSet** supplies exactly that: 1,757 rows, 924 of them face-on, from eight golfers of mixed
skill hitting into a camera-based launch monitor. Each row carries per-phase joint metrics across
the same eight swing events GolfDB annotates, plus the ball's Carry, BallSpeed, DirectionAngle,
SpinAxis and spin components.

### What it is and is not

- **It is one CSV of derived metrics.** No video, no raw keypoints, no frame indices. Its joint
  metrics are CaddieSet's own definitions from CaddieSet's own pipeline.
- **It cannot speak to tempo.** Without frame indices no ratio of durations survives — and tempo is
  the one checkpoint currently failing on this golfer's swings.
- **Eight golfers is few, and they differ enormously.** The "desirable spin axis" base rate runs
  from 18% for one golfer to 79% for another. Any model given the pooled rows will learn which
  golfer it is looking at.
- **It is dirty in ways that are silent.** 11 cells hold Excel's `#NAME?`, 11 hold an infinity from
  a division by a zero-length body segment, and the normalised columns run to 33 shoulder-widths
  where the median is -0.15.
- **It is MIT-licensed**, which makes it the first third-party corpus here that *could* be vendored.

## Decision

### 1. Take it, and keep it under the same gitignored root as GolfDB

`data/reference/caddieset/`, fetched by `scripts/caddieset/fetch.py`, ignored by the existing
`data/reference/` rule. The MIT licence would permit vendoring the CSV into git; we decline, so that
the licensing boundary stays a **directory** rather than a per-file judgement someone has to
remember for each corpus. The LICENSE is fetched alongside the data so its terms travel with it.

### 2. Its numbers may never become bands here

ADR-012 §4 governs: a band is only comparable to a swing measured the same way, and these are not
our `metric_definitions_version: 3` measurements. Because the CSV ships no keypoints, they cannot be
re-derived into ours either. **What transfers is which features carry signal and in which
direction — never a threshold.** No script in `scripts/caddieset/` writes to `ranges.json`.

### 3. Keep its row shape inside `scripts/`

`contracts/reference.py::ReferenceSwing` has no slot for a shot outcome. Widening a shape in
`contracts/` for a corpus whose value was unproven would have been a speculative L3 change; nothing
in `src/` reads per-swing reference rows, only aggregates ship, so the shape stays local to the
ingest script. Revisit only if a second paired corpus arrives.

### 4. Validate by golfer, and let folds refuse

`LeaveOneGroupOut` on `GolferId`, with a fold **skipped rather than scored** below 25 shots or 5 in
the minority class. Per-fold AUC, not pooled: AUC inside one golfer's shots asks "do these features
rank this golfer's good shots above their bad ones", which is the coaching question, while pooling
lets a between-golfer difference in base rate masquerade as skill at ranking.

## Consequences

### The panel does not predict ball flight, and that is the finding

`scripts/caddieset/study_panel.py`, over the 924 face-on shots:

| target | mechanics (transfer) | club choice alone | verdict |
|---|---|---|---|
| straight start, \|direction\| ≤ 6° | 0.594 | 0.535 | marginal pass |
| desirable spin axis, \|axis\| ≤ 10° | 0.532 | 0.572 | **fail** |
| carry, standardised within (golfer, club) | R² = **-0.205** | — | **fail** |

Stable across a regularisation sweep (straight start 0.566–0.610, spin axis 0.515–0.520), so the
verdict is not an artefact of one hyperparameter. A negative R² means the model does worse than
predicting the golfer's own average shot.

**Centering each feature on the golfer's own mean makes transfer worse, not better** (0.443 and
0.502, at or below chance). That is the decisive detail: whatever little signal existed was
*between* golfers — traits of these eight people — not a within-golfer mechanism anyone could act
on.

### Which is what ball-flight physics predicts, and the ROADMAP already suspected

Start line and curvature are set by the club face and the club path. A face-on camera pointed at a
body does not see the club. The ROADMAP says this in its career-mode section — "we can measure that
a club face is open; we cannot see *why*, because grip, lead-wrist angle and release timing are all
invisible to this instrument" — and this is that sentence measured on 924 shots.

So the honest reading is **not** "the checkpoints are worthless". It is that the mechanics axis and
the outcome axis are genuinely separate, which is what [ADR-009](009-swing-scoring-model.md) built
them as. This is the first empirical support that decision has had.

### It redirected the modelling work rather than blocking it

The plan this came from had a mechanics→outcome model as its centrepiece. The gate killed it before
it was built — the job `tune_arm_parallel.py` did for two candidate checkpoints. What survived is
the question face-on pose *can* answer: not "will this swing hit it straight" but "is this a
combination tour players produce", which is [ADR-022](022-learned-artifacts-as-committed-data.md).

### Limits worth restating

- **Eight golfers.** Nothing here generalises to a population; it generalises to these eight.
- **Their instrument, not ours.** A null result can mean the biomechanics do not predict the ball,
  or that CaddieSet's own pose pipeline is too noisy to show it. This study cannot separate those,
  and 22 structurally unreadable cells suggest the pipeline does fail sometimes.
- **The two views are the same shots.** 803 face-on rows have a down-the-line row with a
  byte-identical launch-monitor reading, so 1,757 rows are roughly 950 physical shots.
  `common.shot_key` exists to make that visible rather than let a future study double-count it.

## References
- Jung et al., *CaddieSet: A Golf Swing Dataset with Human Joint Features and Ball Information* —
  <https://arxiv.org/abs/2508.20491>, <https://github.com/damilab/CaddieSet>
- [ADR-012](012-golfdb-reference-data.md) — the corpus this one complements, and the licensing
  posture it reuses
- [ADR-009](009-swing-scoring-model.md) — the dual-axis split this result supports
- [ADR-022](022-learned-artifacts-as-committed-data.md) — what was built instead
