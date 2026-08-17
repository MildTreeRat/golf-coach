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

---

## Addendum (2026-08-16): the trajectory model, and all three placements surfaced

The Decision above shipped the joint model and deliberately left it unsurfaced. Both halves of that
have now moved.

### A second learned artifact, under the same rule

`trajectory_model_v1.json` (64 KB) fits the swing as a **path through time** rather than six
scalars at four instants: 12 landmarks, x/y, 40 samples on a normalised event axis, PCA to 6
components over 415 face-on clips from 116 golfers. It reports **T²** (distance inside the fitted
subspace) and **Q** (the residual off it — a shape the tour basis cannot represent at all, which
nothing else here can express).

Nothing in §1 changed to accommodate it: fitted offline under the `research` extra, shipped as
numbers, evaluated by stdlib arithmetic. The pattern held for a second, quite different model,
which is the strongest evidence it was the right seam.

**The feature builder lives in `analysis/trajectory.py`, and the fitting script imports it.** This
is the part worth insisting on. A trajectory built one way at fit time and another at scoring time
produces a model evaluated against numbers that were never fitted to it — plausible output, no
error, nothing to catch it. One implementation, in the package, used by both sides; the script's
only additions are corpus facts (GolfDB's absolute event indices, and its pixel-aspect correction).

### Three things measured that would have been asserted

- **`z` lost.** ADR-021's sibling gate (`tune_z_channel.py`) passed MediaPipe's depth channel at a
  ratio of 2.69 while warning the figure was an upper bound, because lite and full are one
  architecture at two sizes and agree with each other while both guessing. Fitted both ways, `z`
  *lowered* variance explained at equal component count and worsened both calibrations. **x/y ships.**
- **The anchor set nearly made the model unusable.** The obvious time axis is GolfDB's eight
  annotated events — but four of them are annotations `segment_phases()` cannot produce, so a model
  keyed to them could never score a real swing. Caught before it shipped. The three instants this
  pipeline does detect validate *better* (73.6% variance against 66.3%, same calibration).
- **A pixel-aspect bug, found by reading `derive_pose_metrics.py`.** `videos_160` squashes a
  non-square crop to a square, so x and y sit on different scales per clip (ADR-012's third
  accepted limit). Metrics built from x-ratios cancel it; this model mixes axes in every component
  and did not. Correcting it moved variance explained 73.6% → 80.3% — about seven points of the
  basis had been describing GolfDB's cropping — and **changed the optimal component count from 10
  to 6**, which is a good reason never to carry a hyperparameter across a change in how features
  are built.

Full sweeps in [M4_POSE_BAKEOFF.md](../M4_POSE_BAKEOFF.md) §Phase E.

### Surfaced, as measurements rather than checkpoints

`ANALYSIS_VERSION` 3 → 4. `engine.py::_placements` records three quantities on every swing —
`tour_joint_distance`, `tour_trajectory_t2`, `tour_trajectory_q` — with `source:
"population:golfdb"`, which is a third source alongside `pose:face_on` and `launch_monitor:*` and
says plainly that these are population-relative rather than measured off the body.

They ride on `measurements`, never `checkpoint_scores`, so **`overall_score` cannot move**. Verified
rather than assumed: re-analysing the four stored swings changed `measurements` 9 → 12 and left
every score identical. `tests/analysis/test_trajectory.py` and `test_joint.py` each pin that
`mechanics.py` and `scoring.py` hold no reference to either model.

This is M6.5's ordering: the quantities are recorded and inspectable now; a band for any of them
has to be earned separately, against a population of these values that does not exist yet.

### The limit that has not moved

`Q` is **not calibrated** — 12% leave-one-player-out exceedance against a 10% target — because a
golfer the basis never saw has idiosyncrasies that land in the residual by construction. So Q
partly measures "a different person" rather than "a worse swing". Both exceedance figures ship
*inside* the artifact under `leave_one_player_out_exceedance` so a consumer sees how far to trust
each without finding this page, and the measurement's own `detail` string says it too.

---

## Addendum (2026-08-17): a second camera, and why its placement is never blended with the first

`trajectory_model_dtl_v1.json` joins the two face-on artifacts. It is fitted the same way — offline
under the `research` extra, shipped as numbers, evaluated by stdlib — so §1 needed no change for a
third model, which is now the second piece of evidence that the seam was drawn in the right place.

### The two views are two models, and the placements are reported side by side

`analyze_swing_bundle` records `tour_trajectory_t2_dtl` and `tour_trajectory_q_dtl` beside the
face-on pair. They are **never combined into a single number**, and that is a decision rather than
an omission: blending them would be exactly the mistake [ADR-009](009-swing-scoring-model.md)
avoided by keeping mechanics and outcome as separate axes. Two cameras answer the same question
about different planes of one swing, and a mean of the two answers neither.

**Disagreement between them is a finding, not a defect.** A swing that places as ordinary face-on
and unusual from behind has departed in the plane the face-on camera cannot see, and saying so is
more useful than averaging it away.

They are separate *names*, not one name with a view field, because `measurements` is keyed by name
everywhere downstream — `analysis/baseline.py::pooled_samples` groups by it, so two entries called
`tour_trajectory_t2` would silently pool a face-on and a down-the-line number into one personal
baseline.

### Anchors are reused, not recomputed

The bundle has already segmented the down-the-line clip to build its alignment anchors, and those
are the same three instants the model resamples onto. `placement_from_anchors` takes them directly.
Cheaper, but the real reason is the one `analyze_swing_bundle` already gives for reusing the
face-on phases: it makes it impossible for the frames the trajectory is read from and the frames
the warp pins to `tau=1` to disagree.

Two independent implementations of "read three instants off a phase chain" now exist —
`alignment.anchors_from_phases` and `trajectory.anchors_from_phases` — and
`test_the_trajectory_anchors_agree_with_the_alignment_anchors` pins them to each other. Nothing
else would catch that drift, and its symptom would be a model scored on different frames than the
swing was aligned on.

### `ANALYSIS_VERSION` 5 → 6

Face-on is untouched; no checkpoint, band or score moves, verified by re-analysing all four stored
swings to identical `overall_score`. `measurements` goes 12 → 14 on a two-view bundle.

### A caveat the stored swings surfaced immediately

Three of the four stored swings report `tour_trajectory_q_dtl` of **11.05** against **5.19** for the
fourth. Those three share the down-the-line clip whose segmentation still reports a one-frame
backswing (§Phase F). So a large Q is doing what it should — flagging a shape the basis cannot
represent — while the *cause* is a broken anchor set rather than an unusual swing. Q cannot tell
those apart, which is one more reason it ships uncalibrated and labelled.
