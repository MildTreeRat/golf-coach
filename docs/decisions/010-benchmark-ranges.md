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
