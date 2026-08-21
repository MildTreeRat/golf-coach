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

## Addendum (2026-08-18): the clustered interval on `hip_sway_norm`'s lower edge, and why it stays

M8's `tune_joint_structure.py` measured something the addendum above could not have known when it
asserted this band's lower edge: **458 clips from 122 golfers are not 458 independent samples.**
Several clips are often cut from one broadcast of one swing motion, so a bootstrap resampling
*clips* is confident in proportion to a count it has not earned. Resampling **players** instead
leaves the point estimate where it was — p10 `0.1414` — and widens its 95% interval down to
**`0.0801`**.

ROADMAP and WORKLOG both recorded that as "0.0801 sits 1.6x the error floor rather than the 2.8x
ADR-010 justified it on", which reads as *the edge may be unmeasurable*. That reading is what this
addendum exists to correct: **the rule above measures one thing and the bootstrap measures
another.**

- **"Clears the instrument" is a claim about resolution.** Can the pipeline tell 0.14 from zero?
  Yes — 0.050 noise+boundary, so 2.8x. Clustering does not enter that calculation at all. It is a
  property of the measurement, not of the sample.
- **The bootstrap is a claim about placement.** Is the tour p10 *at* 0.14? Best estimate yes;
  honestly, somewhere at or above 0.08.

Only the second degraded. `1.6x` is a coherent quantity — it is where an edge *placed at the
interval's floor* would sit against the noise — but it does not describe the edge that ships, which
is at 0.14 and clears at 2.8x. Reading it as though it had replaced the 2.8x is what turns a
placement result into an apparent resolution failure. **The finding is that we are less certain
where the edge belongs, not that the edge cannot be measured**, and the response to reduced
confidence is to record it rather than to delete the edge.

**So `low` stays at `0.14`**, and the `ranges.json` row now carries the interval beside the 2.8x —
the half it previously stated alone.

Two further reasons, beyond the reframing:

- **`hip_shift_at_top_norm` is not the precedent it resembles.** Its edge was dropped because its
  p10 of 0.015 sits *below* its 0.053 error floor, separating golfers the pipeline cannot tell
  apart. Nothing about `hip_sway_norm` is in that condition. Two rows looking alike while resting on
  different arguments is the exact trap the 2026-08-12 addendum was written about, arriving from the
  other direction.
- **Dropping the edge would make derived prose wrong.** `one_sided` feeds `contracts/caveats.py`'s
  "on the one-sided checkpoints … below the band is the good side" — the bullet that stops a model
  congratulating a golfer for a low number. Filing `hip_sway` under it would assert the one thing
  this ADR has now twice refused to assert: that less lateral hip travel is better.

**What would reopen it**, so the next reader inherits a trigger rather than a judgement call: bay
footage measuring near this edge, or a re-derivation over a corpus with more *distinct golfers* —
not more clips, which is the quantity that flattered the original interval.

**Consequences.**

- **Nothing executable changes and `ANALYSIS_VERSION` does not move.** No band, evaluator or score
  is touched, so no stored `analysis.json` becomes outdated and `scripts/reanalyze.py --dry-run`
  reports every result current. All four stored swings pass this checkpoint at 1.0 (0.27, 0.27,
  0.27, 0.41), which is precisely why this was the cheap moment to settle it: the same call taken
  after a bay session would be moving scores while deciding.
- **The decision is pinned where the reasoning lives, not where the number does.** Behaviour is
  unchanged, so the only thing that can silently regress is the argument going missing.
  `tests/analysis/test_benchmarks.py` asserts this row's provenance still names the clustered bound,
  so a later tightening cannot be justified on the confident half alone.
- **The 2026-08-12 rule stands, with a second axis.** Assert a band edge only where it clears the
  instrument — *and* record how well the sample locates it. The first governs whether an edge may
  exist at all; the second governs how much confidence its provenance is allowed to project.

## Addendum (2026-08-18): per-club bands were costed, then gated, and none is cut

ROADMAP carried "per-club bands are now costed" as M8's last open box: `tune_joint_structure.py`
had counted the strata that *could* be cut — face-on driver 341 clips, iron 69, fairway 32 clearing
`derive_reference.MIN_SAMPLES`. **Costed is not gated.** That a stratum is big enough to cut a band
from says nothing about whether the band that comes out differs from the one already shipped.
`scripts/golfdb/tune_per_club_bands.py` asks the second question and the answer is no.

**The test is in two parts, and the first alone would have been wrong.** Part 1 screens each
per-club p90 against the all-club p90 in units of that metric's own noise+boundary error, read from
`contracts.dispersion.METRIC_TARGETS` rather than restated, at `tune_spatial_metric.py`'s 2.0x bar.
Part 2 puts a player-clustered bootstrap on any stratum that clears it — the addendum above is why:
32 clips from 14 golfers is not 32 observations, and `MIN_SAMPLES` is a *clip* gate that does not
notice.

**Nineteen strata were cut and screened. Two survive both parts**, and they do not make a panel:

| stratum | golfers | naive shift | after clustering |
|---|---|---|---|
| `head_sway_norm` x iron | 27 | 3.35x | **2.85x — survives** |
| `finish_balance_norm` x fairway | 14 | 3.89x | **2.56x — survives** |
| `finish_balance_norm` x iron | 27 | 2.46x | 1.28x — fails the bound |
| `head_sway_norm` x fairway | 14 | 2.62x | interval contains the all-club edge |

Everything else screened out before the bootstrap: all four `tempo_ratio` strata (0.17–0.55x), and
`hip_sway_norm`, `hip_shift_at_top_norm` and `head_hip_gain_norm` at every club (0.08–1.62x).

**Driver never differs from the all-club band** — 0.17x to 0.56x on all six metrics — which is not a
finding about drivers but about the corpus: driver is 341 of the 458 face-on clips, so the all-club
band very largely *is* the driver band. A per-club driver row would restate a shipped one.

**The two survivors say one physical thing** — a shorter club is swung with less movement — so the
same test was run with every non-driver club pooled, on 42 golfers rather than 14 or 27. Exactly one
effect survives there: `head_sway_norm`, p90 **0.2614** against the all-club **0.4311**, worst-case
**2.20x** the floor. `finish_balance_norm` falls to 1.49x. So the real, measured result is narrower
than the per-club framing: **head sway is genuinely lower with non-driver clubs, and nothing else
about the panel depends on club at all.**

**Tempo deserved its own question, and the club is the wrong axis for it.** Tempo screens out at
every club while being the metric a coach would most expect to track club length, so the variance
was decomposed both ways over the 1,399 labelled swings:

- between **golfers**, sd of group means **0.7618**
- between **clubs**, sd of group means **0.1626**

The golfer effect is **4.7x** the club effect, and the club effect is **5.8x smaller than our own
tempo measurement error** (0.943, `METRIC_TARGETS`). A per-club tempo band would key on a quantity
five times below the noise on the measurement it would be applied to. Tempo is a personal signature,
and the axis it belongs on is the golfer — which is `PersonalBaseline`, already built and waiting
on `n`, not a band in this file.

**Decision: no per-club row is added to `ranges.json`.** Beyond the thinness above, shipping even
the one real effect needs three things this repo does not have:

1. **A corpus-to-`ClubCategory` mapping that is not a fiction.** `ClubCategory` distinguishes
   `long_iron` / `mid_iron` / `short_iron`; the corpus says only `iron`. Seeding three members from
   one undifferentiated stratum would be three rows claiming to be three strata while being one, and
   `resolve_range` matches `club_category` exactly. `contracts/reference.py` has always said the
   mapping is "the consumer's"; nothing has had to write it, and it is not writable from this corpus.
2. **Something that records which club was swung.** The plumbing is complete —
   `analyze_bundle.py --club` reaches `resolve_range` through `PracticeGoal` — but it is a manual
   flag. `Shot` carries club head speed, face angle and path, and no club identity, so the launch
   monitor does not supply it either. Every stored swing reads `club: all`.
3. **`_population_placement` following the band's stratum.** Its docstring already flags this: the
   band and the percentile must move together, or a swing reads "inside the band" and "past the 90th
   percentile" at once. `ResolvedRange` carries `(low, high, source)` and not which stratum it
   resolved to, so that coupling is a contract change rather than a data edit.

**What would reopen it**: more *golfers* per club stratum, or the club becoming a recorded fact
rather than a flag — at which point the pooled non-driver head-sway result is the one candidate with
evidence behind it, and item 3 is the work.

**Consequences.**

- **Nothing executable changes and `ANALYSIS_VERSION` does not move.** The gate is a script that
  prints; no band, evaluator or score is touched. ADR-010 §2's rule holds unchanged: bands resolve
  most-specific-first, and with no per-club row every swing continues to land on `(all, all)`.
- **The negative result is committed, not just concluded.** `tune_per_club_bands.py` re-runs in
  seconds on a base install, so the next person to ask this question gets the numbers instead of the
  argument — the same reason `tune_arm_parallel.py` stayed in the tree after killing two candidate
  checkpoints.
- **`MIN_SAMPLES` is now known to be the wrong gate on its own.** It counts clips. Fairway passes it
  at 32 clips and 14 golfers, and 14 golfers is what actually killed `head_sway_norm x fairway`.
  Left as-is rather than changed, because it guards `derive_reference.py`'s published distributions
  where a thin stratum is reported with its `n_players` beside it, and no band is cut without this
  gate having been run.

## Addendum (2026-08-19): `unscored` carries the reason, not just the name

§2 says no score beats a wrong one, and ADR-013 added the disclosure half: a checkpoint that cannot
be scored is *named* in `SwingResult.unscored`, because `overall_score` is a mean over survivors and
a swing judged on five fundamentals must not print the same shape of number as one judged on six.

**Names alone turned out to be half the disclosure.** They say a fundamental went missing and
nothing about what to do next — and the two most common causes want opposite advice. An unreadable
clip wants re-filming. A swing with no golfer attributed was measured perfectly and wants a golfer
picked, which takes one click on the results page. Telling that second golfer to go back to the bay
is a confident answer to a question they did not ask.

**So two consumers reconstructed the cause, and neither was the source.** `feedback/rules.py` held a
`_UNSCORED_REMEDY` table keyed on checkpoint name and decided between the two by checking whether
`head_hip_gain_norm` had survived into `measurements` — if the number was there the frames must have
been readable, so the missing score had to be the handedness case. That is inference from a side
effect rather than from the failure. It gave the right answer for the one checkpoint someone wrote a
row for and could never have covered a second; a `NO_BAND` would have been told to steady the
camera. `api/pipeline.py` narrated the same case again as free text.

**The causes were always enumerable.** Six `return None` sites in `analysis/measure.py` and two more
in `checkpoints/mechanics.py`, each of which knows exactly which condition it just failed and has
the window and landmark group still in hand. `contracts/unscored.py` gives them names —
`PHASE_NOT_SEGMENTED`, `BOUNDARY_ESTIMATED`, `TIMING_DEGENERATE`, `LANDMARKS_UNCONFIDENT`,
`TOO_FEW_FRAMES`, `SCALE_UNAVAILABLE`, `NO_BAND`, `NO_HANDEDNESS` — and one `ReasonSpec` row each.

**The load-bearing field is `refilming_helps`,** not the reason itself. It is the bit every consumer
needs and none could derive, and `contracts/caveats.py` derives its warning from it rather than
listing the causes in prose, so a reason added later cannot leave the coaching models reading a
stale list.

**Why `contracts/` rather than `analysis/`.** `feedback` may not import `analysis` (ADR-008), which
is exactly what forced `rules.py` to retype two checkpoint names as string literals with a comment
apologising for it. The vocabulary living in `contracts/` removes the need instead of working around
it — the same argument `contracts/caveats.py` makes for the standing warnings and
`contracts/dispersion.py::METRIC_TARGETS` for the error floors.

**A split worth keeping: `MEASUREMENT_REASONS`.** `analysis/measure.py` may report only the six
measurement causes; `NO_BAND` and `NO_HANDEDNESS` are judging failures and cannot come out of it.
That is M6.5's measure/judge separation made checkable rather than aspirational — a `NO_BAND`
escaping into `measure.py` would mean the measuring layer had started consulting bands again, which
is the fusion that kept the panel at three checkpoints.

**Consequences.**

- **`ANALYSIS_VERSION` does not move.** Every number and every verdict is where it was; what changed
  is what a refusal *says about itself*. Its own rule is "not for anything that leaves every number
  where it was", and this leaves all of them there.
- **Stored artifacts keep reading.** `SwingResult.unscored` coerces a bare name to `UNRECORDED` in
  one `field_validator`, and `mcp/query.py` does the same for the raw JSON it reads without building
  the model. Dropping those entries was the one unacceptable option: a swing judged on five
  fundamentals would start reading as one judged on six. `UNRECORDED` is a reader's answer and the
  engine never writes it — a pin asserts that, so "this file does not know" cannot decay into "we
  did not bother to say".
- **`api/pipeline.py`'s note stays, and is not a duplicate.** The evaluator knows only that no
  handedness reached it; whether nobody picked a golfer or a `player_id` names one missing from the
  registry is knowledge that exists only in the shell, and the two want different fixes. The reason
  says what was missing, the note says how.
