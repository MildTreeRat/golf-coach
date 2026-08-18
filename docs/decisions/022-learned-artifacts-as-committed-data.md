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

---

## Addendum (2026-08-17): the placements are spoken — the policy, since there is no band

Three models were fitted, validated and surfaced onto `SwingResult.measurements`, and **nothing
said any of it to a golfer**. That was M6.5's ordering working as designed — a quantity should be
inspectable across swings before it is spoken — but it left the placements one step short of
useful. Closing that step needed a *band* or a *policy*. It is a policy, and the argument for which
is the whole of this addendum.

### Why not a band

A band would put a placement on the scoring path, and there is nothing to cut one from. Every clip
behind these three models is a tour professional, so they describe the shape of swings that *work*
and hold no information whatever about swings that do not — the same limit
[ADR-012](012-golfdb-reference-data.md) records for `ranges.json`, but sharper here, because a
range check at least asserts a direction. A Mahalanobis distance asserts only *distance*, and
"far from the tour population" includes both the golfer who is doing something wrong and the tour
player with an unusual action. Scoring that would fail every amateur forever while telling them
nothing about what to change.

So the firewall [ADR-010's 2026-08-04 addendum](010-benchmark-ranges.md) put around percentiles
stays exactly where it is: placements ride on `measurements`, never on `checkpoint_scores`, and
`overall_score` does not move. **What changed is not what is computed. It is what may be said.**

### The policy

1. **A placement is context, never a verdict, and never the headline.** `feedback/rules.py` emits
   no tip for one and the coaching prompt says so in as many words. A number with no band cannot
   rank against six numbers that have one.
2. **It earns a clause when it sharpens the thing already being named** — most usefully when a
   metric passes its own band while driving the swing's distance from the population, which is a
   sentence no band can produce and the reason the joint model was fitted at all.
3. **An uncalibrated placement is labelled on the line that carries it**, not only in the caveat
   block above it. The two Q residuals over-flag any golfer the tour basis never saw, which is
   every golfer using this system.
4. **Unusual is not bad** is stated wherever a placement is stated. An unusual swing that works is
   a style.

### Why a registry, and why it is in `contracts/`

`contracts/placements.py` is new, and it is `checkpoints.py`'s argument repeated rather than a new
one: `caveats.py` has to *name* these five in prose that ships into the coaching system prompt and
into every MCP client's `outputSchema`, and ADR-008 forbids `contracts` importing `analysis`. The
alternative was hand-typing the list, which is precisely what went stale during M6.5 — and had
already gone quietly wrong here, since the caveats named **none** of the five while all five
shipped. `analysis/engine.py` now takes each placement's name and unit off the registry, and
`benchmarks/trajectory.py` takes its two view strings from it, so one spelling reaches the artifact
keys, the emitted measurement and the prose.

### The gap this closed in the MCP channel

`mcp/query.py` flattened `measurements` to `name -> float`, dropping unit and detail. For a pose
metric that is right — `address window -> impact window` is provenance for a derivation step. For a
placement the detail *is* the meaning: it carries the percentile, the population size and the
calibration warning, none of which survive being reduced to a float. An MCP client was therefore
receiving `tour_trajectory_q_dtl: 11.06` as a bare number, under a field description promising
there was *no* percentile — the largest figure on the swing, with nothing to say it was a
mis-detected anchor. Placements now travel as their own `population` list, which is the one part of
`measurements` that keeps its `detail`.

### The same gap in the browser channel, closed a day later

The fix above reached the MCP client and the coaching brief and **not the results page**, which is
the one surface a golfer actually looks at. `results.html::measurementsBlock` rendered all fourteen
measurements as `name / value / unit` and dropped `detail` exactly as `mcp/query.py` had, under a
caption reading *"These have no tour benchmark band yet, so nothing here is a pass, a fail, or a
percentile. They are recorded so bands can be derived from real swings later."* Every clause of
that is wrong for a placement: the dropped `detail` states a percentile, and by this addendum a
band is deliberately never coming.

So the policy's line 3 now holds in all three channels. The page groups placements by `view`,
carries each `detail` as the row's primary content, and stamps *not calibrated* on the line
holding the value — grey rather than the panel's amber, because borrowing the checkpoint table's
miss colour would assert the fault this ADR says no band supports.

The split is resolved in Python and never in the page: **`api/state.py::resolve_placements`** is
the one definition of what turns a stored measurement entry into a readable placement, wrapped by
`mcp/query.py`'s `PlacementView` for one channel and served as a derived `population` key on the
swing-detail route for the other. `tests/api/test_results.py` pins that `results.html` names no
placement in code, since a set restated in JavaScript is a second copy that goes stale — which is
how five placements came to ship with no prose naming them in the first place.

One thing found on the way, and fixed with it: that block was captioned *"measured, not yet
judged"* while listing the six metrics behind the checkpoint table directly above it. It now
excludes them via `api/state.py::judged_metrics`, mapping through `CheckpointSpec.metric`, and
holds the three quantities that genuinely have no band — which is what its sentence always claimed.

### `ANALYSIS_VERSION` does not move

Nothing about the engine's output changed meaning: same names, same values, same units, same five
placements. Re-analysing `2026-08-10/2` reproduced its stored `analysis.json` unchanged. The version
counter tracks what the numbers *are*, and this addendum is about what is said about them.

---

## Addendum (2026-08-17): the placements in a personal history — deferred, and what would decide it

The addendum above settled what may be said about a placement on **one** swing. It did not settle
what career mode may say about a golfer's **history** of one, and career mode has been quietly
answering that question since the placements shipped. Found while checking the blast radius of the
policy, not introduced by it. **The decision is deferred rather than taken**, and this records the
argument so the deferral is a choice rather than an oversight.

### What pools today

`analysis/baseline.py::pooled_samples` groups a golfer's corpus by `Measurement.name`. Nothing
filters on `source`, so `build_baseline` for `aaron` returns **fourteen metrics, five of them
placements**, each with the default floors from `contracts/baseline.py`.

Worse than un-filtered: **un-deduplicated**. `CorpusSwing.artifact_key` knows two source prefixes
(`pose:`, `launch_monitor:`) and keys a sample by the clip or the photo it was read off, so two
swings sharing a face-on clip are one pose sample. `population:golfdb` matches neither and falls
through to `swing:{ref}` — one sample per swing *directory*, per the conservative fallback that
branch was written for. `storage/corpus.py::_count_metrics` already reports this, and has been
reporting it since the placements shipped:

```
Unrecognised measurement sources: population:golfdb
  Counted per swing rather than per artifact - check the dedupe key still fits.
```

The reader's own coverage warning fired correctly. Nobody read it.

### What already refuses, and it is most of the surface

Two of the three consumers of a `PersonalBaseline` decline the placements without being told to:

- **`analysis/dispersion.py`** — placements are absent from `METRIC_TARGETS`, so `dispersion_for`
  returns early with an `unavailable` note and **neither a bias nor a scatter finding is ever
  emitted**. `target_for`'s "silent by omission rather than judged against a borrowed error term"
  is doing exactly the job it was written for, on a case it was written before.
- **`analysis/comparison.py`** — refuses all five with *"no reference distribution is stored for
  it"*, since a placement's population is the model, not a `ranges.json` row.

This matters for how alarming the finding is. The worry that reads worst — *dispersion is
documented as a cause discriminator (bias points at something set before the swing, scatter at
timing and release), and that reading applied to a Mahalanobis distance is nonsense* — is real
about the **framing** and false about the **output**. No such finding reaches a reader today.

### The one layer with no refusal, and the date it bites

`build_baseline` itself. Given enough `n` it will publish a mean, a 95% interval, a median, an sd
with its own interval, a min/max and a per-session trend for `tour_trajectory_q_dtl` — a quantity
this repo ships explicitly labelled *NOT calibrated*, whose largest observed value on our own
corpus is a mis-detected down-the-line anchor (§Phase F) rather than anything about the golfer.

Today `n = 2` and every claim is withheld, so nothing false is said. **The default `CENTER` floor
is 5.** One bay session of 20–30 swings crosses it — and a bay session is the single action the
roadmap is pointed at, so the statistic appears on the first day there is enough data to look at.
That is the deadline this addendum exists to record.

### Why it is not obviously wrong to pool them

A golfer's own trend in `tour_joint_distance` is a real question, and arguably the most interesting
number career mode could produce: *is this swing moving toward the shape of swings that work?* That
placement is the **calibrated** one, its exceedance rate was validated on held-out players, and a
trend over a golfer's own history needs no band — it is a comparison to themselves, which is the
whole premise of career mode.

### Why it is not obviously right either

- **The machinery around the number is built for a different kind of number.** Bias/scatter
  discriminates a cause that is fixed before the swing from one that happens during it, and it
  reads a *signed* metric against a target. A Mahalanobis distance is folded and non-negative, has
  no target, and decomposes into neither cause. The refusal above is currently an accident of
  `METRIC_TARGETS` membership rather than a stated position.
- **The two `_dtl` placements cannot be deduplicated at all.** `CorpusSwing` carries
  `face_on_sha256` and `shot_sha256` and no down-the-line hash, so two swings sharing one
  down-the-line clip contribute two samples of one reading. On the corpus as it stands, three of
  four swings share the clip behind `q_dtl = 11.05` — which would read as a *consistent* golfer
  while measuring the same artifact three times. That is the exact failure the dedupe rule exists
  to prevent, arriving through the gap in it.

### What would decide it, and the seam a fix uses

The question is not "filter or don't". It is **whether a distance-from-a-population is a personal
quantity at all**, and the cheapest evidence is the bay session that triggers the problem: with
n≈25, look at whether `tour_joint_distance` moves coherently within one golfer or just tracks
whichever swings the pose estimator handled well.

Whatever is decided, the change goes in one place. `CorpusSwing.artifact_key` is documented as
**the single definition of the dedupe rule**, called by both the counter (`storage/corpus.py::
_count_metrics`) and the pooler (`analysis/baseline.py::pooled_samples`), and returning `None`
already means "contributes no sample" — the branch a flagged shot parse takes. A `population:`
prefix returning `None` there moves counting, pooling, dispersion and the tour join together, and
leaves `tests/storage/test_corpus.py`'s `set(baseline.metrics) == set(corpus.metric_counts)`
true without editing it. Keeping them instead means adding a down-the-line hash to `CorpusSwing`
and a third dedupe branch keyed on `PlacementSpec.view`.

`tests/analysis/test_baseline.py::test_a_placement_pools_as_a_metric_today_and_that_is_deferred`
pins the current behaviour and names this addendum, so the day it changes, it changes on purpose.
