# M5: Prioritised coaching feedback (pose-only)

> **Tier: AS-BUILT.** The ranking below is what `feedback/rules.py` does today.

**Status**: implemented and verified, 2026-08-04
**Decisions**: [ADR-009](decisions/009-swing-scoring-model.md) (scoring model),
[ADR-010](decisions/010-benchmark-ranges.md) + its 2026-08-04 addendum (percentiles),
[ADR-012](decisions/012-golfdb-reference-data.md) (reference population),
[ADR-013](decisions/013-clip-relative-detection.md) (detection confidence)

## The problem

After M4-REF, a swing produced three independent pass/fail readouts and three tips of equal weight,
in whatever order the engine happened to call the evaluators. Two things were wrong with that as
*coaching*:

1. **Nothing said what to work on first.** Three tips, no ranking, passing checkpoints mixed in with
   failing ones.
2. **A perfect score could hide a real weakness.** `golf_swing-aaron-1` scored **100/100** — all
   three checkpoints in band — while its head sway sat at 0.36 shoulder-widths, higher than **83% of
   458 tour swings**. Nothing in the output could say so.

The second is the interesting one, and it points at the cause. `_score_within_range` returns exactly
`1.0` for every passing checkpoint. Three passes means three identical scores, so **no ordering of
in-band checkpoints is derivable from anything we stored**.

## What was built

### 1. Population placement on every checkpoint

`golfdb_v1.json` — 64 distributions over 7 metrics, with a built-and-tested `percentile_of()` — had
no production consumer at all; it was imported only by its own test. Each evaluator in `mechanics.py`
now calls `_population_placement()` and attaches `percentile`, `population_n` and `one_sided` to its
`CheckpointScore`, and appends a plain-English clause to the tip:

```
Unbalanced finish - 0.48 shoulder-widths of drift after impact. Swing to a held, balanced
finish (aim under 0.29). That is a looser finish than at least 90% of 458 tour swings.
```

Constraints, all covered by the ADR-010 addendum and by `tests/analysis/test_population.py`:

- **Never touches `score` or `passed`.** A test blinds the evaluators to the distributions and
  asserts both are unchanged.
- **Same stratum as the band** — `(all, all, all)`. Drawing the percentile from a narrower stratum
  than the band produces "inside the band, past the 90th percentile" contradictions.
- **Phrasing respects the `[10, 90]` clamp** — "at least 90%", never a precise-looking 97th.

### 2. Ranked tips and a headline

`feedback/rules.py` sorts tips most-actionable-first and adds `FeedbackPayload.headline` naming the
single thing to work on.

**Ordering needs two different signals, and this is the part that is easy to get wrong.** The bands
*are* the reference p10/p90, and `percentile_of` clamps there, so every failing checkpoint reports
percentile 90 — a hair over the line and triple the line are indistinguishable. The percentile
saturates exactly where the band ends. So:

| group | ranked by | why the other signal fails |
|---|---|---|
| failures | `score` (ascending) | percentile is pinned at 90 for all of them |
| passes | percentile tail distance (descending) | `score` is exactly 1.0 for all of them |

Severity stays on `score` for the same reason. `_tail_distance` maps the median to 0.0 and the rail
to 1.0, using `one_sided` to decide whether the lower tail is excellent (sway, balance) or equally
wrong (tempo).

### 3. Unscored checkpoints are named, not hidden

`evaluate_tempo` returns `None` when the address boundary was estimated rather than detected
(ADR-013) — about 14% of corpus clips. `overall_score` is a mean over whatever survived, so a
two-checkpoint swing and a three-checkpoint swing previously printed the same number with nothing to
tell them apart. `SwingResult.unscored` now carries the names, the CLI prints a `not measured:` line,
and `build_feedback` emits an INFO tip. The score itself is **not** penalised — dropping the
checkpoint is correct (ADR-010 §2); dropping it silently was not.

### Result on the two local clips

```
aaron-swing-2        78/100   >> Work on finish balance first. [...] a looser finish than at
                                least 90% of 458 tour swings.
                              tempo pct=54.7  head_sway pct=10.1  finish_balance pct=90

golf_swing-aaron-1  100/100   >> All 3 checkpoints are inside tour range.
                                 Closest to the edge: head sway.
                              tempo pct=20.6  head_sway pct=83.4  finish_balance pct=13.8
```

---

## The panel stayed at three checkpoints — a no-go, with numbers

`golfdb_v1.json` carries four metrics with no user-side counterpart. `toe_up_frac` is a club-shaft
event and gated on M2. But **mid-backswing** and **mid-downswing** are defined as *the lead arm
parallel to the ground*, which is a body pose a face-on camera can see: the lead wrist passes through
the height of the lead shoulder. Two extra checkpoints, from data already committed, looked free.

They are not. `scripts/golfdb/tune_arm_parallel.py` measured three candidate rules against the 461
hand-annotated face-on clips before either checkpoint was written:

| event | rule | med err (frames) | PCE | **frac_err** | vs tour spread |
|---|---|---|---|---|---|
| mid_backswing | `cross` (labelled span) | 4.0 | 25.6% | 0.071 | 45% |
| mid_backswing | `argmin` (labelled span) | 3.0 | 39.7% | 0.059 | 37% |
| mid_backswing | **`prior_frac`** (labelled span) | **2.0** | **60.1%** | **0.043** | **27%** |
| mid_downswing | `cross` (labelled span) | 1.0 | 76.8% | 0.100 | 43% |
| mid_downswing | `argmin` (labelled span) | 2.0 | 71.6% | 0.125 | 54% |
| mid_downswing | **`prior_frac`** (labelled span) | **1.0** | **98.3%** | **0.038** | **17%** |

`prior_frac` reads **no pose signal at all** — it places the instant at the tour-median fraction of
the span. It beats every pose rule on every column, for both events. A checkpoint built on our
detection would therefore be worse than a constant: it would report noise where "assume the tour
median" does better, and score every golfer as average by construction with extra steps. This is the
same bar `tune_address.py`'s permanent `prior_tempo` row exists to enforce, and the first time it has
actually rejected something.

`frac_err` is the headline column deliberately — the error in the metric the checkpoint would
*report*, in the units of the band it would be compared against. Frames flatter a slow-motion corpus;
the fraction does not.

**One hypothesis checked and rejected.** The obvious explanation for a tight spread is frame
quantization — a real-time downswing is ~8 frames, so `mid_downswing_frac` can only take a handful of
values. It does not hold: the p10–p90 spread is **0.148 real-time vs 0.149 slow-motion** for
mid_backswing (29- vs 108-frame backswings) and 0.196 vs 0.186 for mid_downswing. Four times the
temporal resolution does not narrow it, so the variation is real and our detector simply cannot
resolve it.

`tune_arm_parallel.py` stays runnable, like the six rejected address signals before it, so the next
person to have this idea re-runs it in a second rather than rebuilding the harness. What would change
the answer is a signal, not a constant: club detection (M2) gives `toe_up` directly, and the
down-the-line view (ADR-011) sees arm-parallel far more cleanly than a 160×160 face-on crop does.

## Verification

- `pytest` → **96 passed** (was 75); `ruff` clean; `mypy` clean on `analysis` + `feedback`
- `scripts/analyze_swing.py` re-run on both local clips (output above)
- `scripts/golfdb/tune_arm_parallel.py` → 461 face-on clips, table above

## Known gaps

- **`unscored` carries names, not reasons.** "Tempo could not be measured" does not say *why*
  (an estimated address boundary). A reason needs the evaluators to return one instead of `None`,
  which changes every return site.
- **Failure severity has no population units**, because the percentile saturates at the band edge.
  Storing p1/p99 or the raw sd in `golfdb_v1.json` would lift this.
- **The coaching prose still lives in `mechanics.py`.** Fine for three checkpoints; the moment the
  panel widens, extract a `feedback/catalogue.py` of per-checkpoint templates.
- **Percentiles are club-agnostic.** Per-club strata exist in the corpus but need the band moved in
  step (ADR-010 addendum).
