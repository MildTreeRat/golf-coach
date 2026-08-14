# ADR-010: Benchmark Ranges — Sourcing, Provenance & Club/Player Parameterization

## Status
Accepted

## Date
2026-06-28

## Context
[ADR-009](009-swing-scoring-model.md) scores a swing by comparing each checkpoint's
`observed` value against an `expected_low`/`expected_high` range. The entire credibility
of the system rests on **where those ranges come from** and **whether they fit the golfer
and the club**. Two sub-problems:

1. **Sourcing** — what defines a "correct" range, and how do we defend it? We are not
   (yet) sitting on a proprietary dataset the way the industry leaders are.
2. **Parameterization** — a "good" range is not a constant. Spine angle, attack angle,
   launch, and spin differ by **club**; acceptable dispersion and distance differ by
   **skill level**. A single fixed band per checkpoint will misfire.

### The industry data landscape (for reference)
- **Outcome / ball-flight**: TrackMan "Tour Averages" (gold standard, per-club: speed,
  launch, spin, smash, attack angle), Arccos / Shot Scope (large *amateur*-by-handicap
  datasets — more realistic targets than tour data), FlightScope, PGA Tour ShotLink /
  Strokes Gained.
- **Mechanics / biomechanics**: TPI (kinematic sequence, X-factor — the body-swing
  standard), 3D rigs GEARS / AMM3D / K-Vest (kinematic-sequence norms), academic
  biomechanics literature (MacKenzie, Cheetham; *Sports Biomechanics*), and **Tour Tempo**
  (the ~3:1 backswing:downswing ratio — camera-measurable).
- **The real moat is owning the data.** TrackMan/Arccos/3D vendors built authority by
  capturing millions of swings/shots with ground-truth hardware. We can't out-data them on
  day one, so v1 leans on published benchmarks and migrates toward our own captured data.

## Options Considered

### Sourcing

#### Option A: Hardcode "best guess" thresholds in the checkpoint code
- **Pros**: Fastest to write.
- **Cons**: No provenance — can't defend or audit a number. Changing a range is a code
  change. No path to per-club/per-player data without a rewrite.

#### Option B: Versioned benchmark data file with provenance (chosen)
- **Pros**: Ranges live as data, each entry carrying its `source` + `date`. Auditable,
  swappable, and the natural place to later replace published norms with our own
  calibration data. Keying supports club/skill parameterization with graceful fallback.
- **Cons**: A small resolver + schema to build up front.

### v1 source coverage

#### Adopt everything at once (TrackMan + TPI + Tour Tempo + Arccos)
- **Cons**: Most of it (per-club launch/spin, kinematic sequence) needs the launch monitor
  and club detection we don't have yet. Premature.

#### Start with Tour Tempo, expand later (chosen)
- **Pros**: Tempo (~3:1) is camera-measurable from pose alone — exactly the pose-only PoC
  (ADR-009, ROADMAP M4-PoC). One well-cited source, immediately usable, no hardware.
- **Cons**: Single checkpoint at first. Accepted — the store is built to grow.

## Decision

### 1. Ranges are versioned data with provenance
Benchmarks live in a data file (YAML/JSON) under the package (e.g.
`analysis/benchmarks/`), **not** as constants in code. Every entry records its source so
each threshold is auditable and replaceable:

```yaml
- checkpoint: tempo_ratio
  club_category: all          # tempo is club-independent
  skill_level: all
  low: 2.7
  high: 3.3
  source: "Tour Tempo (Novosel) — ~3:1 backswing:downswing across tour players"
  source_date: "2004"
  added: "2026-06-28"
```

### 2. Ranges resolve as a function, with fallback
The evaluator does not read a constant; it asks a resolver:

```python
resolve_range(checkpoint, club_category, player_profile) -> (low, high, source)
```

Resolution falls back most-specific → least-specific so the store can be sparse:
`(7-iron, 10-hcp)` → `(mid-iron, mid-skill)` → `(all, all)`. A missing range yields no
score for that checkpoint rather than a wrong one.

### 3. Parameterization contracts
- **`ClubCategory`** enum (e.g. driver / wood / hybrid / long-iron / mid-iron /
  short-iron / wedge / putter, plus `all`). Checkpoints declare whether they vary by club.
- **`PlayerProfile`** contract (skill level / handicap band; optionally height & physical
  limits for future mechanics ranges; `all` default).
- Pose-mechanics checkpoints that are genuinely club-independent (tempo, posture) key on
  `all`; club/skill keying matters mostly on the **outcome** axis (launch/spin/distance),
  which arrives with the launch monitor anyway.

### 4. v1 sourcing: Tour Tempo now, expand later
Ship with **Tour Tempo (~3:1)** as the only seeded benchmark (club- and skill-agnostic),
powering the pose-only Fundamentals PoC. Add TPI kinematic-sequence / X-factor (mechanics),
and TrackMan + Arccos/Shot Scope per-club outcome norms, as M2/M3 bring detection and shot
data online. Longer term, replace published norms with ranges derived from our **own
captured calibration swings** (and eventually a personal baseline: "vs. your last month").

## Consequences
- Every threshold is auditable to a named source and dated — defensible, not a magic number.
- New benchmarks, clubs, and skill bands are **data edits**, not code changes; the resolver
  and schema don't move.
- The PoC needs exactly one seeded row and no hardware; the same store scales to per-club,
  per-skill, and eventually personalized ranges.
- A genuine gap remains: published amateur biomechanics norms are sparser than outcome
  norms. Mitigated by the fallback-to-`all` resolution and the long-term plan to capture
  our own data. The store makes that migration a data swap.
- Pairs with [ADR-009](009-swing-scoring-model.md): intent selects *which* range applies
  (e.g. fade vs. straight face-to-path); this ADR governs *where the numbers come from*.

## Addendum (2026-07-03): PoC ships JSON, not YAML

This decision says ranges live in a data file, "YAML/JSON" — either is acceptable. The
**M4-PoC** implementation ships **JSON** (`src/golf_coach/analysis/benchmarks/ranges.json`),
loaded via `importlib.resources`. Rationale: it keeps the analysis core on the **standard
library** (`json` is built in; YAML would add PyYAML to the *base* install), consistent with
ADR-008's "analysis is a pure functional core" that runs on `pip install -e .` with no extras.
The schema is unchanged from §1 (each row carries `source` / `source_date` / provenance), and
the resolver contract (`resolve_range(...)` with most-specific → `all` fallback) is exactly as
specified. If human-authored comments or multi-document ergonomics later justify it, switching
to YAML is a loader change behind `resolve_range`, not a schema or contract change.

## Addendum (2026-07-16): two PROVISIONAL / UNCALIBRATED rows added (head sway, finish balance)

The M4-PoC+ Fundamentals-panel work ([docs/M4_FUNDAMENTALS_PANEL.md](../M4_FUNDAMENTALS_PANEL.md))
adds two pose-only mechanics checkpoints and seeds a benchmark row for each:
`head_sway_norm` (`0.0–0.5`) and `finish_balance_norm` (`0.0–0.6`), both `(all, all)`, in
shoulder-widths.

**These are explicitly not published norms.** Unlike Tour Tempo (a cited, defensible source),
these bands are *internal, uncalibrated heuristics* chosen by eye to give sensible pass/fail on
the current face-on clips. Their `source` field says so verbatim and is greppable
(`PROVISIONAL / UNCALIBRATED`). This is a deliberate, honest exception to §1's provenance rule
for the PoC — the store's data-driven design means recalibrating them later is a **data edit**,
not a code change.

**Gate:** they are listed in the ROADMAP **Hardware Re-Validation Gate** for replacement with
values derived from captured ground-truth data once the down-the-line camera / launch monitor
land (ADR-011). Until then, treat sway/balance scores as directional, not authoritative.

## Addendum (2026-08-01): `tempo_ratio` re-sourced from GolfDB — see ADR-012

§4 above said "longer term, replace published norms with ranges derived from our own captured
calibration swings." The first half of that migration has happened, via an interim source this ADR
did not anticipate: [ADR-012](012-golfdb-reference-data.md) adopts **GolfDB** — 1,400 hand-annotated
tour swings — as a reference population.

`tempo_ratio` moves from Tour Tempo's **2.7–3.3** to the **2.72–4.71** p10–p90 of 1,399 clips from
246 tour golfers. Novosel's floor was essentially right and his ceiling was not: the book band
captured roughly the lower third of the real tour distribution (median 3.39).

Three things this validates about the design here:

- **It was a data edit.** `store.py`, `resolve_range`, and the schema were untouched; the fallback
  semantics did not move. Exactly what §1 and the Consequences promised.
- **Provenance carried the reasoning.** The new row's `source` records n, the golfer count, the
  derivation, and what it replaced — so the *next* person to touch it can tell whether it is still
  the best available number.
- **One gap the schema does not cover.** `_score_within_range` decays in *band-widths*, so widening
  a band also softens every partial score computed against it. Re-sourcing a band is therefore not
  a purely local change to its pass/fail boundary, and nothing in the row records that coupling.
  Worth remembering when the remaining rows are re-cut.

The two `PROVISIONAL / UNCALIBRATED` rows are **not** yet replaced; they need pose over the clip
corpus rather than labels alone (ADR-012 Phase B). The Hardware Re-Validation Gate item for them
stands, though ADR-012 shows it no longer strictly requires hardware.

## Addendum (2026-08-04): percentiles ride on `CheckpointScore`, but never on the scoring path

§2 said a missing range yields no score, and ADR-012 kept `golfdb_v1.json` deliberately *off* the
`resolve_range` hot path — "scoring reads `ranges.json` and nothing here". `CheckpointScore` now
carries `percentile` / `population_n` / `one_sided`, filled from that same distribution file. This
records why that is not a violation, and the rule that keeps it from becoming one.

**The firewall.** `score` and `passed` are computed from `ranges.json` alone, exactly as before.
The percentile is attached afterwards and read only by `feedback`. `tests/analysis/test_population.py
::test_percentile_never_moves_the_score_or_the_verdict` blinds the evaluators to the distributions
and asserts both fields are unchanged, so the boundary is executable rather than a comment.

**Why it was needed at all.** `_score_within_range` returns exactly `1.0` for *every* passing
checkpoint, so a swing that passes everything has three identical scores and no way to rank them.
`golf_swing-aaron-1` scores 100/100 while its head sway sits higher than 83% of tour swings — the
one thing on that swing worth telling the golfer, and invisible to every number we stored before.
Ranking in-band checkpoints is the percentile's job; nothing else can do it.

**Same stratum as the band.** `_population_placement` queries `(club, sex, view)` all `"all"` —
the stratum the bands were cut from — rather than the most specific match. `tempo_ratio`'s face-on
p90 is 5.00 against the all-view 4.71, so mixing strata would let one swing read "inside the band"
and "past the 90th percentile" simultaneously. Moving to per-club bands means moving *both* the band
and the percentile together; the resolver's fallback semantics make that a data edit, but it is not
a data edit that can be done on one side only.

**A limit worth recording, because it constrains consumers.** The bands *are* the reference p10/p90
(ADR-012), and `Distribution.percentile_of` clamps to `[10, 90]` because the tails were never
stored. So **every failing checkpoint reports percentile 90**, whether it missed by a hair or by
triple — the percentile saturates precisely where the band ends. It is therefore useless for grading
severity, and `feedback.rules` deliberately keeps severity and failure-ranking on `score` (which
decays in band-widths and does separate them), using the percentile only to rank passes. Storing the
p1/p99 or the raw sd would lift this, and is the change to make if failure severity ever needs
population units.

## Addendum (2026-08-12): two hip checkpoints promoted, and a rule for which band edges may be asserted

M6.5 recorded five metrics without judging any of them, leaving "decide what to promote" as an
explicit later call. Two are now promoted to scored checkpoints, taking the mechanics panel from
three to five: **`hip_sway_norm`** (`0.14–0.50`) and **`hip_shift_at_top_norm`** (`0.0–0.21`), both
`(all, all)`, both cut from the same 458 face-on GolfDB swings by 122 golfers that the other two
spatial bands came from. Both cleared `tune_spatial_metric.py` — spread/error 7.1 and 3.6.

Promotion needed no new data and no schema change; it is two rows and two evaluators, which is what
§1 promised. What it *did* need was a decision this ADR had not previously had to make.

**The rule: assert a band edge only where it clears the instrument.** `derive_reference.py`
recommends a one-sided `[0, p90]` band for any metric named `_norm`, which encodes *less is better*.
That is established for head sway and finish drift and **not** established for either hip metric —
some lateral hip travel is the weight shift a swing needs, the same finding that denied both metrics
a bias target in career mode step 5. Applying the default heuristic would have shipped a band
asserting that motionless hips are ideal. The two metrics then diverge, for different reasons:

- **`hip_sway_norm` is two-sided.** The tour p10 is 0.14, so 90% of tour swings show *more* hip
  travel than that — not the shape of a quantity to minimise. The lower edge is assertable because
  it clears measurement error: `tune_spatial_metric.py` puts this metric's noise + boundary at
  0.050, and 0.14 sits 2.8x that above zero.
- **`hip_shift_at_top_norm` is one-sided, and not because less is better.** Its p10 is 0.015
  against an error floor of 0.053. A lower edge there would separate golfers this pipeline cannot
  tell apart, so only overshoot is judged — the half the instrument resolves.

The distinction is recorded because the two rows *look* like the existing one-sided/two-sided pair
and were reached by a different argument. A later reader tidying the panel into one shape would be
undoing a measurement, not a preference.

**Consequences.**

- **`ANALYSIS_VERSION` goes 1 → 2.** `overall_score` is a mean over surviving checkpoints, so a mean
  over five is not comparable with a mean over three. No measurement moved; what the score *means*
  did. The four stored swings were re-analyzed through `scripts/reanalyze.py`, which found them
  unprompted, and their scores rose (94.92 → 96.95, 93.77 → 96.26) purely because two passing
  checkpoints now dilute one failing one. That is the coupling the 2026-08-01 addendum flagged in a
  different form: panel membership, like band width, changes every score computed against it.
- **`one_sided` is now load-bearing for readers, not just for `feedback`.** With `tempo_ratio` and
  `hip_sway_norm` both two-sided, "a low percentile is good" is wrong on every two-sided
  checkpoint — which ones those are is `CHECKPOINT_REGISTRY`, not a count stated here.
  `contracts/caveats.py` says so explicitly, because an LLM handed a low number will otherwise
  congratulate the golfer for it.
- **`head_hip_offset_impact_norm` stays unpromoted.** Its band exists and its ratio is 7.6, but the
  sign is camera-relative and the stored distribution is cut from a mixed-handedness population.
  `Golfer.handedness` now records the fact (career mode step 1), but `analysis` is pure and does not
  read the golfer registry, so promoting it is an architecture change rather than a data edit. It is
  the next one to consider, and it is not this addendum.
