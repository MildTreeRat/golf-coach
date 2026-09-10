# ADR-029: Pivot points — an interim 2-D-per-view rotation instrument behind a fiducial-ready seam

## Status
**Accepted** 2026-09-09, **built** 2026-09-10 — [ROADMAP §M17](../../ROADMAP.md), nine phases
(P0–P8) on branch `GOLF-5`, one commit each, the full suite / `ruff` / `mypy` green at every one.
P0 is this document, the [ADR-011](011-camera-synchronization.md) addendum, and the milestone's
entry on the board; the phase list lives in [M17_PIVOT_POINTS.md](../M17_PIVOT_POINTS.md).

## Date
2026-09-09

## Context

"Pivot points" are the joint centres a golfer watches to judge rotation: the **shoulder-line
midpoint**, the **hip-line midpoint**, and the **hands** (the lead/trail wrist midpoint). The
ask is to track them through the swing, draw them and the two **rotation lines** — the shoulder
line and the hip line — on the swing video, and raise a signal when the turn is "wonky" rather
than clean. Club-head tracking is explicitly later: no detector exists and the one spike
([ADR-017](017-club-head-detection-strategy.md), M1.5) returned a no-go on exposure time.

**True rotation is 3-D, and this capture tier cannot read it.** [ADR-011](011-camera-synchronization.md)
established that spine angle, hip rotation, X-factor and the kinematic sequence are inherently
three-dimensional; its 2026-08-05 addendum then established that the hand-held two-phone tier
"can be aligned but never fused" — two phones held by two people have no stable extrinsics, so
triangulation is "unreachable by construction" and those four measurements are fixed-rig only.
A single 2-D camera foreshortens the turn badly: the ADR measured ~37° of spine tilt
down-the-line against ~2° face-on for the same posture, and MediaPipe's monocular `z` put it
anywhere from 54° to 71°.

**Fiducial markers are coming, and they change the premise.** The golfer will print QR-code
fiducial squares and place them around the hitting area. That gives real per-camera intrinsics
and a stable ground plane / extrinsics — the calibration [ADR-011](011-camera-synchronization.md)
§Architecture calls the prerequisite for true 3-D — and unblocks a calibrated pose / 3-D
landmark source. It is not built yet, so M17 must ship a rotation reading that is honest about
being interim **and** structured so the calibrated source drops in behind it without a rewrite.

**A checkpoint is not available.** [ADR-010](010-benchmark-ranges.md) §2 forbids a score that
might be wrong, and a checkpoint needs a band cut from a corpus of the metric. A 2-D
image-plane line angle carries the frame's pixel aspect (`analysis/measure.py`'s module
docstring; `trail_hand_roll_deg` ships **unjudged** for exactly this reason), and measured
*through the swing* — where M14 P3 found face-on hand tracking falling to 0.63–0.68 — it also
carries foreshortening that changes with how the phone was held. There is no band to earn yet
and no calibrated instrument to earn it with.

## Options Considered

### Option A: Wait for the fiducial markers
Build nothing until calibration exists, then measure rotation properly in 3-D.
- **Pros**: every number that ships is a real one; no interim instrument to explain, deprecate
  or migrate off; matches the repo's lightest-touch posture.
- **Cons**: the overlay — the shoulder line, the hip line, the three points — is useful *now*
  and needs no calibration to draw; the rule checks on the pivot **paths** (does a point
  reverse mid-swing, does the hip centre slide off its axis) are largely aspect-immune and
  useful now too; and the seam that lets the 3-D source slot in later is cheap to build
  alongside the thing that motivates it and expensive to retrofit.

### Option B: An interim 2-D-per-view instrument behind a `contracts/` seam *(chosen)*
Draw the overlay into `aligned.mp4`; measure the pivot paths per view with stdlib rule checks;
surface the numbers as unjudged `Measurement`s named per view (`_dtl` suffix), each carrying
the sentence that says *why* it is interim; put the swappable shape in `contracts/`.
- **Pros**: the overlay and the wonky signal land now; the numbers leave the system saying
  what they are; when the calibrated source arrives it is a second producer of one
  `contracts/` shape and the rule checks, the registry and the overlay are untouched — only
  the "interim" prose flips.
- **Cons**: it ships an instrument that a later milestone will partly supersede, which is a
  stronger move than the repo usually makes — so it needs this ADR to authorise it and the
  "interim" label has to be load-bearing rather than decorative.

### Option C: The interim instrument, but as a checkpoint with a provisional band
Give shoulder-line and hip-line rotation a band cut from GolfDB and score them.
- **Pros**: a single mechanism; the golfer gets a pass/fail.
- **Cons**: rejected under [ADR-010](010-benchmark-ranges.md) §2. A through-swing 2-D angle is
  pixel-aspect-distorted and foreshortened by an unknown amount per clip; a band cut from
  GolfDB's corrected corpus would judge a live bay clip against a scale it is not on.
  `trail_hand_roll_deg` is the precedent for shipping such a proxy — and it ships *unjudged*,
  and only over the static address window where tracking is 1.00.

### Option D: Fit an offline "wonkiness" model from GolfDB
An [ADR-022](022-learned-artifacts-as-committed-data.md)-pattern learned artifact over the
pivot-point paths.
- **Pros**: one calibrated number for "how far from a tour turn".
- **Cons**: the trajectory model ([ADR-022](022-learned-artifacts-as-committed-data.md), 2nd
  addendum) already does "distance from the tour swing shape" and "a shape the basis cannot
  represent" (Q) with `residual_by_interval` for *when*. A second learned artifact for
  rotation specifically has no ground truth for "wonky", would bake the interim 2-D instrument
  into a committed basis that the fiducial source then invalidates, and is a milestone of its
  own. The user asked for stdlib rule checks, not a model.

## Decision

### 1. Full stack in M17

The overlay, the per-view measurements and the wonky signal all land in this milestone —
against the M14 pattern of stopping at the screen. The screen here is not a gate: the pivot
paths are visible from face-on (M14 P3), and the rule checks that need the least calibration
(axis drift, path reversal) are the ones that carry the milestone.

### 2. Both cameras, per view, never blended

Face-on and down-the-line each get their own pivot tracking and their own rule-check
measurements. Down-the-line is where the turn is least foreshortened, so its numbers matter
most — but the two views are two scales and are never combined, the same call
[ADR-022](022-learned-artifacts-as-committed-data.md)'s third addendum made for the trajectory
models (`trajectory_model_v1.json` / `trajectory_model_dtl_v1.json`, reported side by side).
Every down-the-line metric is named with a `_dtl` suffix and paired with its face-on
counterpart, exactly as `contracts/placements.py` names `tour_trajectory_t2_dtl` — the suffix
is part of the name, not a field, because `analysis/baseline.py::pooled_samples` groups a
golfer's history by name and one name for two cameras would pool two scales into one baseline.

### 3. The fiducial-ready seam lives in `contracts/pivots.py`

A new module, modelled field-for-field on `contracts/placements.py`:

- **`FrameOfReference`** — `IMAGE_PLANE_FACE_ON` / `IMAGE_PLANE_DTL` now, `CALIBRATED_3D`
  later. Every `PivotObservation` declares which one it is in.
- **`PivotObservation`** — one sample of the swing: the three pivot points as `(x, y)` in the
  declared frame, the shoulder-line and hip-line orientation, the `frame_of_reference`, and a
  `club_head` slot left `None` that nothing populates in M17.
- **`PivotMeasurementSpec`** and **`PIVOT_MEASUREMENT_REGISTRY`** — per-view specs, `_dtl`
  names in matched pairs, order load-bearing and append-only, each carrying an
  `interim_reason`: the sentence a `CALIBRATED_3D` producer would clear.

**The rule checks consume `PivotObservation`, never `FrameKeypoints`.** M17 ships one producer
(`analysis/pivot.py::pivot_observations`, reading pose). A fiducial-calibrated source becomes a
*second producer of the same shape* in `CALIBRATED_3D`; the registry and the checks do not
change, and the `interim_reason` prose is what flips. A `RotationSource` Protocol port is the
natural next step and is **deferred until that second producer exists** — the repo does not add
an abstraction before its second case (`docs/REFACTOR_LEDGER.md` culture).

### 4. Wonky detection is stdlib rule checks on the pivot paths

In a new `analysis/pivot.py` — stdlib + `contracts/` only, like the rest of `analysis/`
([ADR-008](008-project-structure.md)). Not `measure.py` (scalars at instants over a window)
and not `trajectory.py` (the PCA feature vector one fitting script imports); a third sibling,
"the pivot points as paths, and the checks on them", stated as the single implementation the
way `trajectory.py` is. The checks — path smoothness / frame-to-frame roughness, no
against-the-turn reversal within the backswing or the downswing, the hip and shoulder centres
staying near a stable vertical axis — each return a `MeasureOutcome` and become an unjudged
`Measurement`, **not** a `CheckpointScore` (Decision, Option C).

### 5. `ANALYSIS_VERSION` moves 15 → 16

New `measurements` entries are what a version bump is for — the `3 -> 4` / `14 -> 15`
precedents, and M14's mistake ([ADR-027](027-ball-flight-simulation.md) 2026-09-06b addendum)
of reading the band question and the version question as one. A version-15 `analysis.json` is
*missing* the pivot rows, not *disagreeing* about anything: `overall_score`,
`checkpoint_scores` and `unscored` are byte-identical after the change. The ledger line in
`contracts/swing.py` says so, in the same commit as the bump (P5). The corpus is re-analysed
for data that session; every stored `aligned.mp4` is re-rendered in P7 (`reanalyze.py` checks
anchors, which do not move, so the overlay re-render is a deliberate `--video` pass).

### 6. The pivot measurements are registered nowhere near `METRIC_TARGETS`

They live only in `contracts/pivots.PIVOT_MEASUREMENT_REGISTRY`, never in `POSE_MEASUREMENTS`,
so `tests/analysis/test_dispersion.py::test_every_production_metric_has_a_tolerance` stays
green untouched — the same handling the `flight_*` family gets, and a deliberate disjointness
assertion pins it (mirroring `test_the_simulated_flight_is_registered_nowhere_and_that_is_the_choice`).
Career mode reports them `unavailable` rather than as a dispersion finding: a scatter statistic
on a foreshortened, pixel-aspect-distorted through-swing angle would report the camera's noise
as the golfer's repeatability. This is "measure now, judge later" ([ADR-010](010-benchmark-ranges.md),
M6.5) where "later" is "after fiducial calibration".

### 7. Club head stays out

`PivotObservation` carries the slot so a future detector has somewhere to write; nothing
populates it, no measurement reads it, and the overlay draws no club.

## Consequences

- **This is an L3 change.** `contracts/pivots.py` is new; `analysis/pivot.py` is a new
  subsystem; `SwingResult.measurements` grows per-view pivot rows; `mcp/query.py::SwingView`
  gains a `rotation` field; `contracts/caveats.py` gains a derived bullet that reaches every
  coaching call and MCP client. Every consumer of `SwingResult.measurements` was walked during
  planning.
- **Every stored swing needs re-analysis and re-rendering.** `is_outdated` catches the
  measurement gap; the overlay re-render is a manual `reanalyze.py --all --video`, and stored
  `aligned.mp4` show the old overlay until it is run — the M14 P1 situation, recorded in the
  milestone doc.
- **The interim instrument is honest but weak, and the ADR says how.** Through-swing 2-D,
  foreshortened, pixel-aspect on the angles, and the down-the-line clip carries no hash of its
  own in `CorpusSwing`. Each spec's `interim_reason` states its own limit, and the fiducial
  seam is the documented exit.
- **[ADR-011](011-camera-synchronization.md) gains an addendum.** Its 2026-08-05 addendum
  called the phone tier's rotation quantities "unreachable by construction"; M17 adds a third
  handling — not fused, not refused, but measured per view as an explicitly interim
  instrument.
- **The overlay change is one file.** `pose/overlay.py` gains the guide lines and the pivot
  markers; `pose/side_by_side.py` and `api/pipeline.py` are untouched, so both `aligned.mp4`
  panels and every script-driven render pick it up unchanged — the M14 P1 pattern.

## Deferred, by choice

- **A `RotationSource` Protocol port.** The right abstraction once the fiducial-calibrated
  producer exists; premature before it (Decision 3).
- **A motion trail** — where each pivot point has travelled, drawn as a fading path. Needs the
  whole keypoint history threaded through `pose/side_by_side.py`, which is stateless per frame
  today. A later overlay phase, not this milestone.
- **A learned rotation model** (Option D). Reconsider once a calibrated 3-D source makes a
  turn angle a real quantity to fit a basis on.
- **The `Impact Position` / club path** as a rotation input. Waits on club detection (M2) and
  a bay session, the same posture [ADR-028](028-mishit-exclusion.md) took on the tile.

## Addendum (2026-09-09b): the phase plan read back against the code — six corrections before P1

P0 shipped the decisions above and a phase list in [M17_PIVOT_POINTS.md](../M17_PIVOT_POINTS.md).
Reading that phase list against the modules it names found six places where it could not be built
as written. **The decisions are unchanged — five of the six are P4–P6 contradicting them**, which
is the useful shape of the finding: the ADR was right and the plan drifted off it. This addendum
records the corrections so the building session inherits them rather than rediscovering them.

**1. The rule checks get their own function type; `MeasureFn` was the wrong one.** P4 typed
`PIVOT_MEASUREMENTS: dict[str, MeasureFn]`, and `measure.MeasureFn` is
`Callable[[list[FrameKeypoints], list[PhaseSegment]], MeasureOutcome]` — it takes
`FrameKeypoints`, which is exactly what Decision 3 forbids. Under that type a `CALIBRATED_3D`
producer cannot feed the checks and the seam buys nothing, so the type deletes the milestone's
stated payoff. `analysis/pivot.py` declares its own instead:

```python
PivotCheckFn = Callable[[list[PivotObservation]], MeasureOutcome]
```

It lives in `analysis/` rather than `contracts/` because `MeasureOutcome` does
([ADR-008](008-project-structure.md) — `contracts` imports nothing of ours).

**2. One producer signature for both views, and it takes anchors, not phases.** The
down-the-line view never holds a `list[PhaseSegment]`: `analysis/engine.py` segments the face-on
clip and, for the second camera, builds only a `SwingAnchors` from
`anchors_from_keypoints(..., wrist=TRAIL_WRIST)`. That is why `_dtl_placements` takes
`(frames, anchors, handedness)` and converts to `(float(motion_start), float(top),
float(impact))` at the call. So `pivot_observations` takes those three anchor positions too —
`trajectory.anchors_from_phases(phases)` supplies them on the face-on side — and there is one
producer serving both views rather than a `from_anchors` variant beside it. This also removes
`phase_bounds` and `address_sample_bounds` from the milestone: both are `PhaseSegment`-only and
neither can run on the second camera.

**3. Event time is the index space, and the sample count must be odd.** P3 resamples onto event
time via `trajectory.sample_positions`, whose output is *fractional frame positions* — so the
frame indices `phase_bounds` returns do not index the observation list P4 reads. Event time
already carries the answer and needs no phase lookup: with the three anchors, sample 0 is
address, the middle sample is the top and the last is impact. `contracts/pivots.py` therefore
pins `PIVOT_SAMPLES` **odd**, because `sample_positions` puts an anchor on an integer sample only
when it does (`t = span·i/(steps-1)` reaches 1 at an integer `i` iff `steps` is odd), and exports
the two half-open spans the checks read. A slow-motion clip, a real-time clip and the two
cameras all land on the same index space, which is the reason the resampling is there at all.

**4. The line orientation is a unit vector, not an angle in degrees.** `PivotObservation` was
specified with `shoulder_line_deg` / `hip_line_deg` floats. A 2-D line angle is not merely
imprecise here — it passes through a **projection singularity**. Face-on, the shoulder line is
full-width at address and collapses toward zero width at the top, where the shoulders point at
the camera; down-the-line it is the reverse. `atan2(dy, dx)` is worst-conditioned exactly where
the projected segment is shortest, and it is the same place MediaPipe is estimating an occluded
shoulder. A reversal check reading raw degrees would fire hardest on the cleanest turns.

`measure.direction_series` already solved this and states the rule in its docstring: a frame
whose two landmarks have collapsed within `MIN_DIRECTION_LENGTH` is **dropped rather than
normalized**, "because its direction is jitter divided by roughly nothing", and unit vectors are
returned so an averaging caller gets a circular mean. `PivotObservation` follows it — the two
orientations are `tuple[float, float] | None`, `None` where the segment collapsed — which makes
the gate structural instead of something each check has to remember. Degrees are derived once, at
the measurement boundary. A check whose samples are mostly `None` refuses with
`LANDMARKS_UNCONFIDENT` (in `MEASUREMENT_REASONS`) rather than averaging what is left.

This narrows Decision 2. "Down-the-line is where the turn is least foreshortened, so its numbers
matter most" holds for the hip and shoulder **centre paths** and is wrong for the **line
angles**, which are worst-conditioned from behind at address and best face-on. Neither view is
the good one throughout; each is good over the half of the swing the other is blind to, and the
`interim_reason` on the four angle specs says so.

**5. Two consumers and one contract were missed.** Consequences claimed "every consumer of
`SwingResult.measurements` was walked during planning"; three had not been.

- **`api/state.py::resolve_placements`** is the real implementation P6 meant to mirror.
  `mcp/query.py`'s `PlacementView` docstring says it is "the MCP-shaped wrapper around it and
  **not a second implementation**", and `api/app.py` hands the same resolver's output to the
  results page. Splitting `measurements` inside `mcp/query.py` alone would build the second
  implementation that docstring exists to prevent, and leave
  `api/static/results.html` rendering pivot rows as bare unexplained floats — the failure
  `contracts/placements.py` was written to stop. P6 adds `resolve_pivots` beside
  `resolve_placements`, and both channels read through it.
- **`feedback/coach.py`** partitions measurements through `PLACEMENTS_BY_NAME`. Pivot rows fall
  through into the plain-pose-metric bucket and reach the coaching model as naked numbers. The
  derived caveat block helps every call, but the per-measurement path needs the matching
  `PIVOTS_BY_NAME` branch or the block is describing rows the prompt presents as ordinary.
- **`contracts/career.py::CorpusSwing.artifact_key`** maps *any* `pose:`-prefixed measurement to
  `pose:{face_on_sha256}`. This ADR already noted that the down-the-line clip carries no hash of
  its own in `CorpusSwing`; the consequence was not followed through. A `_dtl` row sourced
  `pose:down_the_line` would dedupe on the **face-on** hash, so two different down-the-line clips
  paired with one face-on clip collapse into a single sample and the pooled value is whichever
  was read first. `artifact_key` returns `None` for that source instead — the module's own
  vocabulary for it: "returning None is the assertion that there is no reading here at all",
  which is precisely true while no hash for that clip exists. The fiducial work, or any change
  that gives the second clip an identity, is what reverses it.

**6. The registry: five pairs, one prefix, and the hands are drawn but not measured.** P2's
starter set was four names of which one (`shoulder_turn_reversal_deg`) broke the `pivot_` prefix
the others shared, in a registry that is append-only. It also folded the backswing and the
downswing into one reversal number, so a firing value could not say which half of the swing
produced it — and `analysis/baseline.py::pooled_samples` groups by name, so the two halves would
pool. The set is now five face-on + `_dtl` pairs, every name `pivot_`-prefixed:
`pivot_hip_axis_drift_norm`, `pivot_shoulder_axis_drift_norm`, `pivot_hip_path_jitter_norm`,
`pivot_shoulder_reversal_backswing_deg`, `pivot_shoulder_reversal_downswing_deg`.

Jitter is measured on the **hip centre** and not the hands, and the hands are drawn without being
measured at all. The hip midpoint is the best-tracked of the three pivot points; M14 P3 measured
face-on hand tracking at 0.63–0.68 **over the whole clip**, which is why M14 scoped its own hand
metrics to the address window where tracking is 1.00. A jitter number over a through-swing hand
path would report the tracker's noise as the golfer's, and no number beats a wrong one
([ADR-010](010-benchmark-ranges.md) §2). The hands earn a metric when a calibrated source or a
better tracker makes their path a real one; until then the overlay shows them and the registry
does not.

**And one decision that was absent rather than wrong: face-on pivot rows do enter the personal
baseline.** `pooled_samples` and `build_baseline` filter by no registry — every measurement name
gets an entry — so the face-on pivot rows acquire a personal mean and sd exactly as `flight_*`
already does. That is left in place deliberately: `baseline.refuse` already governs what a
baseline may *assert*, and absence from `METRIC_TARGETS` already makes career mode report them
`unavailable` (Decision 6). The `_dtl` rows contribute nothing, by the `artifact_key` change
above. Recorded here because a silent inheritance is not a decision, and the next reader would
otherwise have to work out whether it was one.

## Addendum (2026-09-09c, M17 P5): the hands' own gate was vetoing the swing they are not measured on

Decision 6 says the hands are drawn and unmeasured — that the three points do not share a fate.
P3's implementation did not carry that far enough: `shoulder`, `hip` and `hands` were bridged
through **one** `trajectory._interpolate_gaps` call, and that function refuses the whole call the
moment *any* column it was given exceeds `MAX_MISSING`. Face-on, 44–61% of the resampled samples
across the fifteen stored clips have no readable wrist pair (M14 P3's 0.63–0.68 whole-clip hand
tracking is exactly why M14 scoped its own hand metrics to the address window) while the shoulder
and hip midpoints read on every sample. P5's first `reanalyze.py --all` recorded the `_dtl` five
on every swing and the **face-on** five on **none** of them — the milestone's primary view,
[Decision 2](#decision) says down-the-line is the more foreshortened camera and face-on's centre
paths matter most, producing nothing, on a green suite. Only down-the-line was visible in the
result because the trail wrist tracks well from behind.

**The fix is two gates, not one.** `pivot_observations` now bridges `shoulder` and `hip` through
one `_interpolate_gaps` call — if either exceeds `MAX_MISSING` the whole observation list refuses,
which is correct, since Decision 6 needs both centres on every sample — and bridges `hands`
through a second, independent call, so a wrist pair that cannot be bridged costs only the hands.
`PivotObservation.hands` is now `tuple[float, float] | None` rather than a bare `(x, y)`: a
consumer that wants a hand path must handle its absence, while `shoulder` and `hip` stay
non-optional because a swing that cannot produce them is not an observation at all
([contracts/pivots.py](../../src/golf_coach/contracts/pivots.py)'s field docstring carries this).
`test_unreadable_wrists_cost_the_hands_and_nothing_else` blinds both wrists on the synthetic swing
and asserts the *other two points are unchanged* — a test that only checked `hands is None` would
still pass on a producer that had silently stopped placing them too.

**No decision above moved.** The registry, the checks, the seam and "the hands are drawn and
unmeasured" are exactly as Decision 6 states; what was wrong was a shared gate the shape did not
prevent. Re-run after the fix: fifteen artifacts, ten pivot rows each, no check refused, every
`overall_score`, `checkpoint_scores` entry and `unscored` list byte-identical to before P5.
