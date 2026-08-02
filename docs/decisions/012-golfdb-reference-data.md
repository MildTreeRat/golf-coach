# ADR-012: GolfDB as a Reference-Swing Source

## Status
Accepted

## Date
2026-08-01

## Context

[ADR-010](010-benchmark-ranges.md) made benchmark ranges *versioned data with provenance*, and said
plainly where they should end up: "replace published norms with ranges derived from our **own
captured calibration swings**." Until that data exists, the store shipped with one cited row
(`tempo_ratio`, from the *Tour Tempo* book) and two rows marked `PROVISIONAL / UNCALIBRATED` —
thresholds chosen by eye to give sensible pass/fail on a single clip. The ROADMAP's
**Hardware Re-Validation Gate** parked their replacement behind a down-the-line camera and a launch
monitor we do not have.

That gate conflates two different needs. Recalibrating a band needs *a population of good swings*.
Validating our instruments needs *ground truth*. Only the second genuinely requires hardware.

**GolfDB** supplies the first, and unexpectedly much of the second: 1,400 swing clips of 248 PGA,
LPGA and Champions Tour players, each hand-annotated with eight swing events, plus club, sex and
camera view. It is not our own captured data, but it is a real, citable, inspectable population
where we previously had a book quote.

### What GolfDB is and is not

- **It has no pose data.** Video, eight event frame indices, a bounding box, and metadata. The
  paper lists keypoints as future work. Any biomechanics must come from running our own estimator
  over the clips.
- **It has no quality labels.** No handicap, no shot outcome, no good/bad. All 248 players are
  tour professionals, so it describes what tour-level swings *look like* — it cannot teach a model
  to tell a good swing from a bad one, and there is no gradation even within the pro pool.
- **It is roughly half slow-motion.** Absolute durations are therefore meaningless across the
  corpus; ratios of frame counts are not.

## Decision

### 1. Use GolfDB as the reference population, in two independent phases

**Phase A — event labels only.** Every metric derivable from the eight annotated frames is a
*frame ratio*, so it needs no video, no estimator, and no GPU, and it is unaffected by the
slow-motion half of the corpus. This is where `tempo_ratio` comes from. Crucially it consumes
**human labels**, so SwingNet's own weak events (Address and Finish at ~30% PCE) never enter.

**Phase B — pose.** We run *our* estimator over the clips to recalibrate the two `PROVISIONAL`
pose metrics. Kept strictly separate from Phase A so that a dead video download, a bad estimator,
or a later change of estimator cannot invalidate the label-derived bands.

### 2. Clean-room licensing: only aggregate statistics enter git

GolfDB's code is **CC BY-NC 4.0**, the dataset itself states **no license**, and the underlying
clips are **third-party broadcast footage** the authors never licensed. Against a stated
possibility of commercial use, the posture is:

- **Never vendor** the annotation database, the clips, or any per-clip derivative into the
  repository. `data/reference/` is gitignored, and that is a licensing boundary, not a size one.
- **Commit only anonymous aggregates** — percentiles over ~1,400 clips, carrying no player name,
  no clip id, and no frame index. A distribution is not a substantial reproduction of a dataset.
- **Never use their code or weights.** SwingNet is not adopted, not vendored, and not run. We use
  their *labels* to compute statistics, which is a far weaker dependency than shipping their model.
- Every committed artifact states these terms inline (`golfdb_v1.json.dataset.license_note`) so
  they travel with the data rather than living only here.

### 3. Three storage tiers

| tier | what | where | git |
|---|---|---|---|
| 1. Corpus | per-clip keypoints | `data/reference/golfdb/keypoints/<estimator>/` | ignored |
| 2. Per-swing | one `ReferenceSwing` row per clip | `data/reference/golfdb/swings.jsonl` | ignored |
| 3. Aggregates | percentiles per metric per stratum | `analysis/benchmarks/golfdb_v1.json` | **committed** |

Tier 1 is a **cache, not a source of truth** — expensive to produce, fully reproducible, and kept
so that a future checkpoint can be measured across a thousand tour swings without touching a video.
Tier 2 is the layer bands are re-cut from. Tier 3 is the only thing that ships.

`ReferenceSwing` ([contracts/reference.py](../../src/golf_coach/contracts/reference.py)) is
deliberately shaped like the `swings` table ROADMAP M4 will add, so persisting it later is a loader
change rather than a redesign. `events` and `metrics` are open dicts because every corpus names its
own instants and the metric vocabulary grows with every checkpoint.

### 4. Bands are only comparable to swings measured the same way

Two fields in `golfdb_v1.json` are load-bearing: `metric_definitions_version` and `pose_estimator`.
A band cut under one definition of `head_sway_norm` says nothing about a swing measured under
another. Any change to either **must** bump the version and re-derive; the before/after ledger
lives in [docs/M4_POSE_BAKEOFF.md](../M4_POSE_BAKEOFF.md).

This is also why the metric-definition fixes landed *before* band derivation rather than after.

## Consequences

### `tempo_ratio` is now sourced from data, and the old band was wrong

p10–p90 over 1,399 clips is **2.72–4.71** (median 3.39, 246 golfers). Novosel's 2.7–3.3 had
essentially the right floor and much too low a ceiling — it captured roughly the lower third of
real tour swings. Two sanity checks passed: the median lands near the classic 3:1 (confirming the
`events` array is indexed correctly — index 0 is `start`, not `address`), and the ratio is
statistically indistinguishable between slow-motion and real-time clips while raw frame counts
differ four-fold.

A consequence worth knowing: `_score_within_range` decays *in band-widths*, so re-sourcing a band
silently re-scales every partial score computed against it, not just the pass/fail boundary.

### It found a defect that no amount of our own data would have

Validating `segment_phases` against the annotations showed the top of the backswing was being
mislocated by a **median of 26 frames**, always late, on 80% of tour clips: the rule was
"top = highest hands", and a full finish puts the hands higher than they ever were at the top. Our
own clips could not have revealed it — none of them finish that high. Rebuilt around the *earliest
major descent* (see `phases.py`), the median error is now **1 frame** for top and **0** for impact.

On `aaron-swing-2` this moved tempo from 1.53:1 — reported as a MAJOR "too quick" fault — to
**3.39:1**, the tour median. The swing was never the problem.

**Ground truth turned out to be the more valuable half of this dataset.** The reference bands were
the goal; validating our instruments against 461 hand-labelled face-on swings was worth more, and
it retires the Re-Validation Gate item "validate phase instants against ground truth" with no
hardware purchase.

### Limits we accept

- **Tour players only.** These bands describe elite swings. For an amateur user the honest framing
  is "how far from tour", not "how wrong you are" — which is what
  `benchmarks/distributions.py::percentile_of` exists to express.
- **Our estimator, their footage.** Reference metrics and user metrics are produced by the same
  estimator, so much of its bias is common-mode and cancels. That argument fails the moment the two
  sides are measured differently, which is why `pose_estimator` is recorded.
- **160×160 crops distort aspect.** `videos_160` resizes non-square bounding boxes to a square, so
  x and y scale differently. Ratios of x-distances (head sway over shoulder width) cancel; metrics
  mixing axes (finish balance) do not, and need the `bbox` column to correct.
- **This is not "our own captured data."** ADR-010's endgame still stands. GolfDB is a much better
  interim than a book quote, and the store makes the eventual swap a data edit.

## Discovered risk (recorded, not actioned): ultralytics AGPL-3.0

`pyproject.toml` depends on `ultralytics>=8.1` and [ADR-005](005-object-detection-yolov8.md) plans
to fine-tune YOLOv8 for club detection. Ultralytics names *"custom-trained models in proprietary
commercial settings"* as an Enterprise-license trigger, and AGPL-3.0 compliance means publishing
source for the **entire derivative work**, not just the detector. This is dormant only because
[PROJECT_CHARTER](../PROJECT_CHARTER.md) lists commercial deployment as out of scope — which is in
tension with the posture in §2 above.

Options if it activates: buy the Enterprise license, or migrate club detection to an
Apache-2.0/MIT detector (RTMDet via `rtmlib`, or plain YOLOX). The longer custom weights are trained
on the Ultralytics stack, the more expensive migration becomes. **Deserves its own ADR; no work
taken here.**

Related: YOLO is not a candidate for *pose* under any circumstances — YOLO11n-pose scores 50.0 mAP
against RTMPose-t's 68.5, so it would mean worse accuracy *and* AGPL exposure.

## References
- McNally et al., *GolfDB: A Video Database for Golf Swing Sequencing*, CVPR Workshops 2019 —
  <https://arxiv.org/abs/1903.06528>, <https://github.com/wmcnally/golfdb>
- [ADR-010](010-benchmark-ranges.md) — benchmark ranges as versioned data with provenance
- [ADR-002](002-pose-estimation-mediapipe.md) — pose estimator selection
- [ADR-011](011-camera-synchronization.md) — why depth-dependent checkpoints stay deferred
- [docs/M4_POSE_BAKEOFF.md](../M4_POSE_BAKEOFF.md) — the metric/estimator change ledger
