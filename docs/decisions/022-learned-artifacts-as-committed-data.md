# ADR-022: Learned Artifacts as Committed Data — and the Tour Joint-Distribution Model

## Status
Accepted. The model is fitted, committed and tested; **it is not yet surfaced on a `SwingResult`**,
which is a deliberate separate step.

## Date
2026-08-16

## Context

Nothing in this repo has ever been trained. The vision stack is pretrained inference (MediaPipe,
PaddleOCR), the statistics core is stdlib `statistics`, and the one place a model was ever planned —
[ADR-005](005-object-detection-yolov8.md)'s YOLOv8 — is deferred on evidence. M4-REF's last open
bullet flagged that closing the address-detection gap "looks like a **learned-model problem, not a
heuristic one**", and noted it "would need its own ADR — ADR-008 keeps the analysis core
stdlib-only." This is that ADR, arrived at from a different direction.

The direction is the shape of the judgment. A swing is scored as **six scalars against six
independent bands** (`CHECKPOINT_REGISTRY` → `mechanics.py::_score_within_range`). That shape can
say each number is normal. It cannot say the *combination* is one no tour player produces, because
six range checks have no way to express "this is fine only if that is also true" — and that
conditional is most of what separates a coach's eye from an average golfer's.

[ADR-021](021-caddieset-paired-reference-data.md) established that the outcome-prediction route to
"good" is closed: face-on body pose does not predict ball flight, because the club sets the ball and
the club is not in the picture. What face-on pose *can* speak to is whether a swing resembles the
swings of people who demonstrably play well — and every one of GolfDB's 458 face-on clips is a tour
professional, so the population is entirely positive examples and needs no labels at all.

### The gate that had to pass first

If the six metrics were near-independent, a joint model would collapse to a sum of squared z-scores
and say nothing the bands already say. `scripts/golfdb/tune_joint_structure.py` measured it over the
458 clips, after applying `derive_reference`'s own screening and handedness folding so the
population matches the one the bands were cut from:

| pairing | r |
|---|---|
| `head_sway_norm` × `hip_shift_at_top_norm` | **+0.441** |
| `head_sway_norm` × `head_hip_gain_norm` | **-0.385** |
| `hip_sway_norm` × `head_hip_gain_norm` | **-0.246** |
| `tempo_ratio` × `hip_sway_norm` | **-0.236** |

Four pairings past the 0.2 gate, and a correlation-matrix condition number of 4.8 — comfortably
invertible. The structure is there.

## Decision

### 1. Fit offline, ship the artifact, evaluate in stdlib

This is the whole architectural claim, and it is not a new pattern — it is what `ranges.json` and
`golfdb_v1.json` already are. A band is a fitted quantity; the fitting happened in a script; what
ships is numbers plus stdlib arithmetic to read them.

| stage | where | may import |
|---|---|---|
| fit | `scripts/golfdb/derive_joint_model.py` | the `research` extra — numpy, scikit-learn |
| ship | `analysis/benchmarks/joint_model_v1.json` | — it is data |
| evaluate | `analysis/benchmarks/joint.py` | stdlib + pydantic |

`research` gains numpy and scikit-learn and keeps its declaration that it is script-only and never
imported by the package. **`tests/api/test_pipeline_imports.py` is what enforces that**, and it
fails in seconds if a `golf_coach.*` module ever reaches for either. [ADR-008](008-project-structure.md)
holds unchanged: no numpy in `analysis/`.

A Mahalanobis distance, a logistic score and a PCA projection are all dot products. Nothing about
"a model" requires a heavyweight runtime; it requires the coefficients and the discipline to write
the loop.

### 2. Interpretable estimators, because n is small and the repo's product is honesty

458 clips, six features. In that regime a robust center, a robust scale and an inverse correlation
matrix are not a compromise — they are the better estimator, and they ship as about fifty numbers a
human can read. A deep model over 458 samples would overfit and be unauditable, which is the
opposite of what this repo is for. The artifact is median/MAD standardisation, clipped at 4 robust
z, then the inverse correlation of the clipped values.

`MinCovDet` was run as an audit of that choice and **did not converge** — the population has no
tight elliptical core. The consequence is recorded rather than ignored: a distance here may not be
read against a chi-square distribution, so the artifact calibrates on **empirical quantiles of the
tour population's own distances** instead of assuming a parametric tail.

### 3. It ships because a shape is an aggregate

ADR-012 §2 permits only anonymous aggregates into git. A center vector, a scale vector and a 6×6
matrix carry no player name, no clip id and no frame index — the same test the percentiles already
pass. The corpus stays gitignored; its shape ships.

This is worth noticing beyond this ADR: it is also the route around the redistribution problem that
blocks "compare my swing against a reference pro". A named pro's clip cannot be redistributed. **A
basis fitted over 122 of them is not a pro.**

### 4. Unusual, never bad — and off the scoring path

Every clip behind the model is a tour player's swing, so it knows what working swings look like and
nothing whatsoever about broken ones. Output is a percentile *within the tour population*, never a
score. `mechanics.py` and `scoring.py` do not import `joint`, and
`tests/analysis/test_joint.py::test_placement_is_not_reachable_from_the_scoring_path` pins that
while it is still trivially true — the same firewall `test_population.py` keeps around percentiles.

### 5. All six metrics or nothing

A distance over five of six is the marginal distribution of those five wearing the same name. A
missing metric returns `None`, which is ADR-010 §2's rule applied to a new quantity. In practice
this abstains when tempo is absent, on the ~14% of clips where address detection declines to guess.

## Consequences

### It generalises to golfers it has never seen

Leave-one-player-out: refit without each of the 122 golfers, score their swings against *that*
model's own p90. **11.1% exceed it, against 10.0% in-sample and a 10% target.** The shape transfers;
it has not memorised these players. The derivation script warns and refuses to recommend shipping if
that figure ever drifts past 5 points.

### It immediately says something the six bands cannot

Placed against the swings on disk, `2026-08-10/2` scores **96.9** on the existing panel — near
perfect, every band it can be judged on passed except tempo — and sits at the **73rd percentile** of
unusualness as a combination. The decomposition names why: `head_hip_gain_norm` contributes 38.6% of
the departure **while passing its band**, because its value is unusual *given* the tempo. Six
independent range checks cannot produce that sentence.

### What is deliberately not done here

The model is not registered as a `Measurement` and does not appear on a `SwingResult`, so
`ANALYSIS_VERSION` does not move and no stored analysis changes. Surfacing it is a separate step
with its own blast radius — the measurements list, the MCP payloads, the caveats prose, a
re-analysis of every stored swing — and M6.5's "measure now, judge later" ordering says the quantity
should be inspectable before it is spoken. It is inspectable now:

```
python scripts/golfdb/tune_joint_structure.py     # the gate
python scripts/golfdb/derive_joint_model.py       # dry run; --write to update the artifact
```

### Limits

- **Tour only, and therefore "how far from tour", not "how wrong".** Same limit as the bands.
- **Six metrics is the whole model.** It sees what the panel sees. A swing can be strange in a way
  none of the six measures and be placed near the center.
- **Correlations are population-level.** That two metrics travel together across 122 tour players is
  not a claim that moving one moves the other in any individual golfer.
- **It inherits every measurement limit upstream of it** — same estimator, same
  `metric_definitions_version`, same 160×160 aspect distortion. Any change to either **must** bump
  the artifact and re-derive, exactly as ADR-012 §4 requires of the bands.

## References
- [ADR-008](008-project-structure.md) — the stdlib-only analysis core this preserves
- [ADR-010](010-benchmark-ranges.md) — benchmarks as versioned data with provenance; §2's "no score
  beats a wrong one"
- [ADR-012](012-golfdb-reference-data.md) — the corpus, and §2's aggregates-only licensing rule
- [ADR-021](021-caddieset-paired-reference-data.md) — why the outcome-prediction route was closed
- `scripts/golfdb/tune_joint_structure.py` — the gate, including player-clustered band intervals
