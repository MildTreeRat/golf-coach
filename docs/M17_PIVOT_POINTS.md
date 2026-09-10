# M17 — Pivot points: the shoulder line, the hip line, and the three points they turn about

> **Tier: TARGET.** This is the plan, not a record — P5 onward is unbuilt as of 2026-09-09, and
> the numbers and file names below are what the plan intends, not what shipped. Verify against
> code once a phase lands and flip its checkbox. The governing decision is
> [ADR-029](decisions/029-pivot-points.md); the constraint it is designed around is
> [ADR-011](decisions/011-camera-synchronization.md) and its 2026-08-05 addendum.
>
> ⚠️ **P2–P6 were rewritten on 2026-09-09 after the first phase list was read back against the
> code**, which found six places it could not be built as written — the wrong function type for
> the rule checks, phases the second camera does not have, two index spaces, a raw line angle
> through a projection singularity, three unwalked consumers, and a registry whose names would
> not have pooled. [ADR-029](decisions/029-pivot-points.md)'s 2026-09-09b addendum is the
> record; the phases below already carry the corrections.

**Status: 8/9 phases built.** P0 wrote this document, [ADR-029](decisions/029-pivot-points.md)
and the [ADR-011](decisions/011-camera-synchronization.md) addendum, and put M17 on the board.
P1 drew the pivot markers and the two rotation lines into the overlay. P2 landed
`contracts/pivots.py` — the shape a fiducial-calibrated source will later produce instead of
pose. P3 built the producer that fills that shape: `analysis/pivot.py::pivot_observations`, one
signature over anchors serving both cameras. P4 added the five stdlib rule checks that call a turn
wonky, keyed by `PivotMeasurementSpec.check` so one implementation serves both views. P5 wired the
numbers into the engine and moved `ANALYSIS_VERSION` 15 → 16 — including the `pose:down_the_line`
source that keys on nothing, so the second camera's rows are reported and pool into no baseline.
P6 gives the numbers a voice — a derived caveat, and one resolver in `api/state.py` feeding the
three surfaces that read `measurements` (the results page, the coaching prompt, `get_swing`), all
saying the numbers are interim. P7 re-renders every stored swing; **built 2026-09-10** — all
fifteen re-rendered, every score byte-identical, every file's frame count reads back exactly what
was written. P8 reconciles the docs and merges `GOLF-5`.

**Built 2026-09-10 (P6).** `contracts/caveats.py` gained `PIVOTS_ARE_INTERIM` and a derived
block naming every `PIVOT_MEASUREMENT_REGISTRY` row, wired into `READING_THIS_DATA_HONESTLY`
beside the two placement bullets. `api/state.py::resolve_pivots` mirrors `resolve_placements`
field-for-field, with `interim_reason` off the spec standing in for `calibrated` — there is no
calibrated half to contrast pivots against, so one bullet and one field cover all ten rather than
two of each. The three channels: `api/app.py` hands the results page a `"rotation"` key beside
`"population"` and `results.html` renders a "Your rotation (interim)" block reusing `.plc`'s
styling (the `measurements` fallback table excludes both now, not just placements);
`feedback/coach.py` gained `_pivot_lines` and a ROTATION section in the brief, labelling each row
`interim` and its `spec.interim_reason` on the next line, the same two-line shape
`_placement_lines` uses for `calibrated`; `mcp/query.py` gained `PivotView` and
`SwingView.rotation`, and `_measurements` now skips `PIVOTS_BY_NAME` names the way it already
skipped `PLACEMENTS_BY_NAME` ones. Full suite green, `ruff` and `mypy` clean.

## What this milestone is

A golfer watching `aligned.mp4` should be able to *see* their rotation: the **shoulder-line
midpoint**, the **hip-line midpoint** and the **hands** drawn as marked points, with the
shoulder line and the hip line drawn through them so the turn is visible as those lines rotate.
Alongside the picture, a set of numbers that say whether the turn is clean or "wonky" — a pivot
point that reverses mid-swing, a hip centre that slides off its axis, a jittery path.

**The turn cannot be measured properly here, and that is the whole design.**
[ADR-011](decisions/011-camera-synchronization.md) says true rotation — hip turn, shoulder
turn, X-factor — is 3-D and the two hand-held phones cannot be fused to recover it. So M17's
rotation numbers are an **explicit interim 2-D-per-view instrument**: unjudged, per-camera,
never blended, each carrying the sentence that says why it is provisional. The golfer is
printing QR-code fiducial markers to put around the hitting area; when those land they give
real camera calibration and a calibrated 3-D source, and the seam M17 builds
(`contracts/pivots.py`) lets that source slot in as a second producer of one shape — the rule
checks, the registry and the overlay do not change, only the "interim" prose flips.

Both cameras get pivot tracking. Down-the-line is where the turn is least foreshortened, so its
numbers matter most, but the two views are two scales and are reported side by side and never
combined — the `_dtl` naming already used for the trajectory models.

## Things that will look obvious and are wrong

1. **"Measure the shoulder turn in degrees and score it."** A 2-D image-plane line angle
   carries the frame's pixel aspect and an unknown amount of foreshortening that changes with
   how the phone was held. `trail_hand_roll_deg` is the precedent for such a proxy — and it
   ships **unjudged**, over the static address window only. A rotation checkpoint needs a
   calibrated instrument and a band, and M17 has neither. [ADR-010](decisions/010-benchmark-ranges.md)
   §2.
2. **"Add the club head — it's the fourth pivot point."** No club detector exists; the one
   spike ([ADR-017](decisions/017-club-head-detection-strategy.md), M1.5) was a no-go on
   exposure time. `PivotObservation` carries a `club_head` slot so a future detector has
   somewhere to write; M17 populates nothing and draws no club.
3. **"Put the pivot metrics in `POSE_MEASUREMENTS` like every other pose metric."** That
   registry is pinned equal to `METRIC_TARGETS` (`tests/analysis/test_dispersion.py`), and it
   is only ever run over the face-on frames. The pivot metrics are per-view and deliberately
   have no dispersion target — they live in their own registry, `contracts/pivots.py`, the
   same separation the `flight_*` family has.
4. **"Draw a trail showing where each point has been."** That needs the whole keypoint history
   threaded through `pose/side_by_side.py`, which is stateless per frame today. Deferred in
   [ADR-029](decisions/029-pivot-points.md); the per-frame line and marker are this milestone.
5. **"Move the shoulder/hip segments out of `_BONES` so they aren't drawn twice."** The
   double-draw is invisible under the thicker guide line, and touching `_BONES` re-derives
   `_JOINTS`. Leave `_BONES` alone; add `_GUIDE_LINES` and `_PIVOT_POINTS` beside it.
6. **"The rule checks are `MeasureFn`s like every other measurement."** They are not, and this
   is the one the first phase list got wrong. `measure.MeasureFn` is
   `Callable[[list[FrameKeypoints], list[PhaseSegment]], MeasureOutcome]` — it takes
   `FrameKeypoints`, so typing the checks that way means a `CALIBRATED_3D` producer can never
   feed them and the whole seam buys nothing. The checks read `PivotObservation` and nothing
   else; `analysis/pivot.py` declares `PivotCheckFn` for them. Only the single producer M17
   ships (`pivot_observations`) touches `FrameKeypoints`.
7. **"Pass the down-the-line view its phases, like the face-on one."** It has none.
   `analysis/engine.py` segments only the face-on clip; the second camera gets a `SwingAnchors`
   from `anchors_from_keypoints(..., wrist=TRAIL_WRIST)` and nothing more, which is why
   `_dtl_placements` takes `(frames, anchors, handedness)`. `pivot_observations` takes the three
   anchor positions, so one producer serves both views — and `phase_bounds` /
   `address_sample_bounds` are out of this milestone entirely, being `PhaseSegment`-only.
8. **"`phase_bounds` gives the backswing span for the reversal checks."** It gives *frame*
   indices, and the observations have been resampled onto event time — two index spaces that
   look alike and silently disagree. Event time answers it with no lookup: sample 0 is address,
   the middle sample is the top, the last is impact. `PIVOT_SAMPLES` is odd so the top lands on
   a sample at all.
9. **"Store the line orientations as degrees."** A 2-D line angle here passes through a
   projection singularity — face-on the shoulder line collapses toward zero width at the top,
   down-the-line it is collapsed at address — and `atan2` is worst-conditioned exactly where
   MediaPipe is guessing an occluded shoulder. A reversal check on raw degrees fires hardest on
   the *cleanest* turns. `PivotObservation` carries unit vectors that are `None` below
   `MIN_DIRECTION_LENGTH`, the rule `measure.direction_series` already states and the reason it
   returns unit vectors; degrees are derived once at the measurement boundary.
10. **"Split the pivot rows out inside `mcp/query.py`, next to the placements."** That is the
    wrapper, not the implementation — `PlacementView`'s docstring says so in as many words. The
    split lives in `api/state.py::resolve_pivots`, beside `resolve_placements`, because
    `api/app.py` hands the same rows to the results page and `feedback/coach.py` partitions the
    same list for the coaching prompt. Three surfaces, one rule.

## The phases

P1 and P2 are independent of each other and either may land first; P1 is written first because it
is the one phase a golfer can see the result of without any of the rest. P3 needs P2; P4 needs
P3; P5 needs P4; then the chain is linear to the merge.

| Phase | Scope | Principal files | Gate |
|---|---|---|---|
| P0 | The milestone enters the repo | ADR-029, this doc, ADR-011 addendum, `docs/README.md`, `ROADMAP.md`, `WORKLOG.md` | doc-truth |
| P1 | The overlay draws the pivot points and the rotation lines | `pose/overlay.py` | — |
| P2 | `contracts/pivots.py` — the fiducial-ready seam + the registry | `contracts/pivots.py` | — |
| P3 | `analysis/pivot.py` — the pivot paths | `analysis/pivot.py` | — |
| P4 | The stdlib rule checks that call a turn wonky | `analysis/pivot.py` | — |
| P5 | The numbers reach the engine; `ANALYSIS_VERSION` 15 → 16 | `analysis/engine.py`, `contracts/swing.py`, `contracts/career.py` | version ledger |
| P6 | The numbers leave the system saying they are interim | `contracts/caveats.py`, `api/state.py`, `api/app.py`, `api/static/results.html`, `feedback/coach.py`, `mcp/query.py` | doc-truth |
| P7 | Re-render every stored swing | `scripts/reanalyze.py --all --video` (operational) | — |
| P8 | Docs reconciled, `GOLF-5` merged | ADR-029, this doc, `docs/README.md`, `ROADMAP.md`, `WORKLOG.md` | doc-truth |

Every phase's gate also includes the full `.venv/Scripts/python.exe -m pytest`,
`-m ruff check src tests scripts` and `-m mypy src`. One commit per phase, message
`M17 P<n>: <lowercase evocative sentence>`, both trailers
(`Co-Authored-By: Claude Sonnet 5 …` and `Claude-Session: …`).

### [x] P0 — the milestone enters the repo

**Built 2026-09-09** (`c18284c`). Full suite 1713 passed, `ruff` and `mypy` clean, doc-truth
green (87). A follow-up commit the same day rewrote P2–P6 against the code and added
[ADR-029](decisions/029-pivot-points.md)'s 2026-09-09b addendum; the design P0 decided is
unchanged.

Docs only. Write [ADR-029](decisions/029-pivot-points.md) (Status *Accepted*, the four options,
the seven decisions), this document, and the [ADR-011](decisions/011-camera-synchronization.md)
addendum (a third handling for the phone tier — measured per view, explicitly interim,
pointing here). Update `docs/README.md` (new ADR-029 row, ADR-011 addenda `1` → `2`, a new
Living-docs row for this file at tier TARGET / `0/9 phases built`, and recompute the opening
`git ls-files '*.md'` counts and the "N decisions / N addenda" line against the command, not by
hand). Add the `ROADMAP.md` status row and a `## M17` detail section. Add a `WORKLOG.md` top
entry.

*Verify by:* `-m pytest tests/test_docs_truth.py` — the document count needs both new files
committed; `test_the_map_counts_the_addenda_correctly`, `test_each_adr_row_states_that_adrs_own_addendum_count`,
`test_the_map_and_each_phase_doc_agree_on_the_phase_count`, `test_every_living_doc_declares_a_tier`
/ `_is_routed_to_from_the_map` / `_agrees_with_each_doc_about_its_tier` all green.

Commit: `M17 P0 (docs only): joint centres, drawn and tracked, with the fiducial seam left open`

### [x] P1 — the overlay draws the pivot points and the rotation lines

**Built 2026-09-09.** Full suite 1719 passed (1713 + 6), `ruff` and `mypy` clean. Built as
written, with two notes. The guide lines run 25% of their own length past each endpoint
(`_GUIDE_EXTENSION`), applied as a proportion of the segment vector rather than a normalised
direction — a collapsed line then extends to itself instead of dividing by zero, which is the
same degeneracy P2's `None` orientation answers. And `_PIVOT_POINTS` *derives* its first two
pairs from `_GUIDE_LINES` (`(*_GUIDE_LINES, (LEFT_WRIST, RIGHT_WRIST))`) rather than restating
them, the `_JOINTS`-from-`_BONES` rule; the topology test names the contents explicitly on the
other side so the pin is not a tautology. **Verified visually**: `scripts/run_pose.py` over
`data/raw/aaron-1/Aaron-front-1.MOV`, four frames through the top of the backswing — the cyan
shoulder and hip lines are legible over the white bones, visibly rotate, and read as axes rather
than bones; the three magenta centres land where the swing turns. At full 4K the marks are small,
which is the pre-existing fixed-pixel-size property of this module — `side_by_side._draw` scales
*before* annotating for exactly that reason, so the `aligned.mp4` a golfer watches is the
in-proportion surface (unconfirmed until P7 re-renders one).

Modify `src/golf_coach/pose/overlay.py` only. Add `_GUIDE_LINES = ((LEFT_SHOULDER,
RIGHT_SHOULDER), (LEFT_HIP, RIGHT_HIP))` and `_PIVOT_POINTS = (… those two …, (LEFT_WRIST,
RIGHT_WRIST))`, plus `_GUIDE_COLOR` and `_PIVOT_COLOR`. Draw the guide lines after `_BONES`
(thicker, optionally extended past the joints so they read as axes), then a marker at each
`_PIVOT_POINTS` midpoint after `_JOINTS`. Reuse the local `pixel()` helper and
`_MIN_VISIBILITY` (both endpoints of a pair must clear it). Comment *why* the two lines are
emphasised — the M14 ear-bone precedent ("not decorative; do not tidy it away").

Both `aligned.mp4` panels and `scripts/{run_pose,analyze_swing}.py` pick this up unchanged;
`pose/side_by_side.py` and `api/pipeline.py` are not touched. Stored `aligned.mp4` show the old
overlay until P7.

*Verify by:* extend `tests/pose/test_overlay.py` on its two halves — a topology pin
(`_PIVOT_POINTS` / `_GUIDE_LINES` contents; every guide pair is also a pivot pair) and an ink
pin (`pytest.importorskip` numpy/cv2: a `_PIVOT_COLOR` marker at each midpoint pixel; a
`_GUIDE_COLOR` line between the shoulder landmarks; the hands marker lands *between* the two
wrist joints and not on either, since both wrists are already in `_JOINTS`; no marker when one
endpoint is dim). Then render one clip through `scripts/run_pose.py` and look at it.

Commit: `M17 P1: the shoulder line, the hip line, and the three points they turn about`

### [x] P2 — `contracts/pivots.py`: the fiducial-ready seam + the registry

**Built 2026-09-09.** Built as written, with three notes. `PIVOT_SAMPLES` is **41** — the odd
neighbour of the fitted trajectory models' `steps = 40`, so the two resamplings read the swing at
the same temporal resolution and the top of the backswing lands on sample 20. The two spans are
`range` objects (`range(21)` and `range(20, 41)`), which reads as half-open in Python while their
*contents* are inclusive of the top on both sides — the reconciliation the ADR addendum's
"half-open spans" and this section's "both inclusive" each described from one side. And the three
`interim_reason` sentences are module-level constants (`_POSITIONAL_INTERIM`,
`_ANGLE_INTERIM_FACE_ON`, `_ANGLE_INTERIM_DTL`) quoted by the specs rather than retyped ten times:
the four angle specs need the *view's own* singularity sentence, which is the addendum §4 narrowing
made structural.

The `detail` prose commits P4 to what it computes — "peak lateral excursion … from its address x",
"sample-to-sample roughness", "largest against-the-turn move … between consecutive samples". A
check that ends up measuring something else has to change the spec in the same commit; the prose
ships on every `Measurement` and cannot be corrected retroactively on stored `analysis.json`.

Create `src/golf_coach/contracts/pivots.py`, modelled on `contracts/placements.py` — same
registry-plus-lookup shape, same `NamedTuple`-as-compile-time-constant posture, and read that
file's `PlacementSpec` docstrings first (its registry tuple is `POPULATION_PLACEMENT_REGISTRY`).
Not the same fields: a placement's `detail` is written at the engine call site because it
interpolates a percentile, and a pivot's is fixed prose, so it lives on the spec here.

- `FrameOfReference(StrEnum)` — `IMAGE_PLANE_FACE_ON`, `IMAGE_PLANE_DTL`, `CALIBRATED_3D`.
- `PivotObservation` (NamedTuple) — one resampled instant: `shoulder`, `hip`, `hands` as
  `(x, y)` in the declared frame; `shoulder_line` and `hip_line` as
  **`tuple[float, float] | None`** — *unit* vectors, `None` where the two landmarks collapsed
  within `MIN_DIRECTION_LENGTH`; `frame_of_reference`; `club_head: tuple[float, float] | None =
  None`. Unit vectors and not degrees for the reason in
  [ADR-029](decisions/029-pivot-points.md)'s addendum §4 and in `measure.direction_series`'
  docstring: the projected line collapses at one end of every swing in both views, `atan2` is
  worst-conditioned exactly there, and a `None` is the honest reading. Say that in the field
  comment — a later reader will want to "simplify" it to a float.
- `PIVOT_SAMPLES: int` — the resampled sample count, **odd**, because
  `trajectory.sample_positions` lands an anchor on an integer sample only when it is
  (`t = span·i/(steps−1)` reaches 1 at an integer `i` iff `steps` is odd). With `BACKSWING_SPAN`
  (sample 0 through the middle) and `DOWNSWING_SPAN` (the middle through the last) — both
  inclusive, so the top sample belongs to each — this is the whole index-space contract, and it
  is here rather than in `analysis/` because both the producer and the checks are held to it.
- `PivotMeasurementSpec` (NamedTuple) — `name`, `label`, `view`, `unit`, `detail`, `check`,
  `interim_reason`. `check` is the unsuffixed key of the rule that computes it: the face-on and
  `_dtl` members of a pair name the *same* check, which is what makes one implementation serve
  two views without string surgery on the `_dtl` suffix.
- `PIVOT_MEASUREMENT_REGISTRY` (tuple, order load-bearing, append-only), `PIVOTS_BY_NAME`,
  `pivot_measurement_names()`, `spec_for()`.

Import `FACE_ON` / `DOWN_THE_LINE` from `contracts/placements.py` — one spelling. The metric set,
five face-on + `_dtl` pairs, every name `pivot_`-prefixed (the prefix is load-bearing: P6 splits
on `PIVOTS_BY_NAME`, but a registry that is append-only should not need a lookup to be readable):

| `check` | Names | Unit |
|---|---|---|
| `hip_axis_drift` | `pivot_hip_axis_drift_norm` (+`_dtl`) | `shoulder_widths` |
| `shoulder_axis_drift` | `pivot_shoulder_axis_drift_norm` (+`_dtl`) | `shoulder_widths` |
| `hip_path_jitter` | `pivot_hip_path_jitter_norm` (+`_dtl`) | `shoulder_widths` |
| `shoulder_reversal_backswing` | `pivot_shoulder_reversal_backswing_deg` (+`_dtl`) | `degrees` |
| `shoulder_reversal_downswing` | `pivot_shoulder_reversal_downswing_deg` (+`_dtl`) | `degrees` |

Backswing and downswing reversal are two names and not one, because `baseline.pooled_samples`
groups by name — one name for two windows pools two distributions, and a firing value could not
say which half of the swing produced it. **Jitter is on the hip centre, and the hands get no
metric at all**: the hip midpoint is the best-tracked of the three, and M14 P3 measured face-on
hand tracking at 0.63–0.68 *over the whole clip*, which is why M14 scoped its own hand metrics to
the address window. A through-swing hand-path number would report the tracker's noise as the
golfer's. The hands are drawn (P1) and unmeasured, on purpose.

*Verify by:* `tests/contracts/test_pivots.py` (new), mirroring `tests/contracts/test_placements.py`
— no duplicate names; every `view` is one of `placements.py`'s two exact strings; `spec_for`
raises on a miss; every spec has a non-empty `interim_reason`; face-on and `_dtl` names come in
matched pairs *and each pair shares one `check`*; every name starts `pivot_`; `_norm` iff
`shoulder_widths`, `_deg` iff `degrees`; `PIVOT_SAMPLES` is odd, and the two spans cover
`range(PIVOT_SAMPLES)` overlapping only on the middle sample.

Commit: `M17 P2: contracts/pivots.py — the shape a fiducial source will produce instead`

### [x] P3 — `analysis/pivot.py`: the pivot paths

**Built 2026-09-09.** Built to the signature below, with three notes and one fixture repair.

**The ruler is the median per-sample shoulder width, not `measure.shoulder_width`'s mean.** The
Reuse list below named that function, and reading it against this window is what changed the call:
it is written for the *address* window, where the shoulder line is square to a face-on camera,
while a pivot path runs address → impact and its mean is dragged down by the collapse at the top
that `PivotObservation.shoulder_line` reports `None` for. A ruler that shrinks with the turn
inflates every excursion normalised against it — so the biggest turns would score the wonkiest.
`build_trajectory` already answers this for a resampled path with a median over the samples, and
taking the same one keeps the two resamplings on one ruler.

**The origin is the hip centre at address — one point, fixed for the whole swing.** "Hip-relative"
in `build_trajectory` means a *per-sample* hip origin, and that reading is unbuildable here: it
puts the hip at `(0, 0)` in every observation and zeroes the travel `hip_axis_drift` and
`hip_path_jitter` are the measurement of. A fixed origin still buys the invariance the per-sample
one buys — a golfer standing anywhere in frame produces identical numbers — while leaving the
motion in.

**`midpoint_series` and `direction_series` are not reused, and their gates are.** Both *drop* an
unconfident frame rather than reporting it, which throws away the frame index — and an index is
what a resampled path is made of. `_read` is `build_trajectory`'s inner `read` named and returning
both axes, and it imports `MIN_VISIBILITY` / `MIN_DIRECTION_LENGTH` rather than restating them, so
a retune still moves everything at once. `_interpolate_gaps` *is* imported from `trajectory.py`
across the underscore: it is the rule for how much of a timeline may be bridged, and a copy would
be a second `MAX_MISSING`.

One thing outside the plan: **the synthetic fixture's two hips sat on one point**, which was
invisible while everything read their midpoint and made the hip line unreadable the moment
something read the line itself. `tests/analysis/conftest.py` now straddles `hip_x` symmetrically
(`_HIP_HALF_SPAN`, the `_EAR_HALF_SPAN` precedent), so every midpoint it ever produced is
unchanged — the full suite confirms it — and `test_conftest.py` pins both halves of that.

P4 inherits one thing from `_unit`'s docstring: **the shoulder line is undirected.** The stored
vector points left landmark → right landmark and flips sign as the shoulders cross the camera
axis, so a check reading turn between samples must fold the angle onto ±90° and not ±180°; a flip
is the same line, not half a revolution.

Create `src/golf_coach/analysis/pivot.py` — stdlib + `contracts/` only; the docstring states
it is the single implementation, the way `analysis/trajectory.py` does. **One producer, one
signature, both views:**

```python
def pivot_observations(
    frames: list[FrameKeypoints],
    anchors: tuple[float, float, float],
    *,
    frame_of_reference: FrameOfReference,
) -> list[PivotObservation] | None: ...
```

Anchors and not phases, because the down-the-line view never holds a `list[PhaseSegment]` —
`analysis/engine.py` segments the face-on clip only and gives the second camera a `SwingAnchors`.
The face-on caller passes `trajectory.anchors_from_phases(phases)`; the down-the-line caller
passes `(float(a.motion_start), float(a.top), float(a.impact))`, the conversion `_dtl_placements`
already does at its call site. There is **no `from_anchors` variant** — this is it. (Note the two
modules named `trajectory`: the resampling helpers are `analysis/trajectory.py`;
`placement_from_anchors`, worth reading as the shape precedent, is
`analysis/benchmarks/trajectory.py`.)

Resample onto event time with `trajectory.sample_positions(anchors, PIVOT_SAMPLES)` so a slow-mo
clip, a real-time clip and the two cameras all land on one index space; confident frames only;
hip-relative; scaled by one shoulder width for the whole swing. The two orientations come from
the `direction_series` rule — unit vector, or `None` below `MIN_DIRECTION_LENGTH` — so the
conditioning gate is applied once here rather than remembered by five checks. No `wrist`
argument: the hands pivot is the midpoint of *both* wrists.

Reuse: `analysis/trajectory.py` (`anchors_from_phases`, `sample_positions`, `_interpolate_gaps`);
`analysis/measure.py` (`midpoint_series`, `mean_of`, `shoulder_width`, `direction_series`,
`MIN_VISIBILITY`, `MIN_SHOULDER_WIDTH`, `MIN_DIRECTION_LENGTH`); `analysis/phases.py`
(`LEAD_WRIST` / `TRAIL_WRIST`). The engine smooths the keypoints before calling, as it does for
`measure.py` — and as `_dtl_placements` does inline for the second view.

*Verify by:* `tests/analysis/test_pivot.py` (new) — the ideal synthetic swing from
`tests/analysis/conftest.py` produces `PIVOT_SAMPLES` observations with no `None` orientation;
`frame_of_reference` on the output matches the argument; the observation at `BACKSWING_SPAN`'s
last index sits at the top anchor (the odd-sample-count pin, from the other side); a synthetic
whose shoulders collapse to a point yields `None` orientations rather than an angle; collapsed
anchors or missing landmarks return `None`.

Commit: `M17 P3: analysis/pivot.py — the swing as three points moving`

### [x] P4 — the stdlib rule checks that call a turn wonky

Extend `analysis/pivot.py` with its own function type and a check table keyed by `check`, not by
metric name:

```python
PivotCheckFn = Callable[[list[PivotObservation]], MeasureOutcome]
PIVOT_CHECKS: dict[str, PivotCheckFn]          # five entries, keyed by PivotMeasurementSpec.check
```

**Five checks, ten metrics.** A face-on and a `_dtl` spec name the same `check`; the view decides
the name, the check does not know which camera it is reading. That is the seam working — and it
is the reason the type is `PivotCheckFn` and not `measure.MeasureFn`, which takes
`list[FrameKeypoints]` and would put the producer back inside every check
([ADR-029](decisions/029-pivot-points.md) Decision 3 and its addendum §1).

The engine calls `pivot_observations` **once per view** and hands the same list to all five —
which is also why the checks take the list rather than the frames: five checks each resampling
the swing from scratch would be five times the work for one answer.

The checks:

- `hip_axis_drift` / `shoulder_axis_drift` — lateral excursion of that centre from its address
  `x`, in shoulder widths. Aspect-immune (`x` over `x`) and the two least calibration-sensitive
  numbers in the milestone, which is why they carry it.
- `hip_path_jitter` — sample-to-sample roughness of the hip centre's path.
- `shoulder_reversal_backswing` / `shoulder_reversal_downswing` — the largest against-the-turn
  move of the shoulder line within `BACKSWING_SPAN` / `DOWNSWING_SPAN`; 0 is monotone. Read off
  the unit vectors (signed angle between consecutive samples, circular by construction), skipping
  `None` samples rather than interpolating across a collapsed line.

Refusals draw only from `MEASUREMENT_REASONS` — `LANDMARKS_UNCONFIDENT` when too many samples
carry a `None` orientation, `TOO_FEW_FRAMES` and `SCALE_UNAVAILABLE` on the paths.

**Handedness is not an argument.** Every check above is unsigned — an excursion magnitude, a
roughness, a largest-reversal magnitude — so the mirroring that
`analysis/engine.py::analyze_swing`'s docstring warns about ("a face-on camera sees a
left-handed swing mirrored") cancels. That is a property to *preserve*, not an accident: a signed
pivot metric added later needs `handedness` threaded to both call sites in P5, and the face-on
one is the one that will be forgotten. Say so in the module docstring.

Reuse: `analysis/measure.py` (`MeasureOutcome`); `contracts/unscored.py` (`MEASUREMENT_REASONS`,
`UnscoredReason` — both live there, not in `measure.py`); `contracts/pivots.py`
(`BACKSWING_SPAN`, `DOWNSWING_SPAN`). **Not** `phase_bounds` or `address_sample_bounds`: both are
`PhaseSegment`-only, the second camera has no phases, and the observations are in event time
rather than frame indices.

*Verify by:* extend `tests/analysis/test_pivot.py` — the ideal swing scores every check
small/clean; a wonky synthetic (inject a mid-backswing shoulder-line reversal, a lateral hip
slide, per-sample jitter) fires the matching check large while the others stay clean; the same
synthetic mirrored left-handed scores identically, pinning the unsigned property; a run whose
orientations are mostly `None` refuses with a reason in `MEASUREMENT_REASONS`; `PIVOT_CHECKS`'
keys equal `{spec.check for spec in PIVOT_MEASUREMENT_REGISTRY}` exactly.

Commit: `M17 P4: the rule checks that call a turn wonky`

### [x] P5 — the numbers reach the engine; `ANALYSIS_VERSION` 15 → 16

**Built 2026-09-09.** Built as written, plus one defect the corpus run found that no test could
have, one thing done that the plan did not name, and one consequence of P4 that only shows up here.

**⚠️ The first `reanalyze.py --all` recorded the `_dtl` five on every swing and the face-on five on
none of them** — the milestone's primary view producing nothing, on a green suite. P3 put the
shoulder, hip and hand columns through **one** `_interpolate_gaps` call, which refuses when *any*
column exceeds `MAX_MISSING`; face-on, 44-61% of the resampled samples on these fifteen clips have
no readable wrist pair (M14 P3's 0.63-0.68 whole-clip hand tracking, the very reason M14 scoped its
hand metrics to the address window) while the shoulder and hip midpoints read on every sample. The
one point this milestone deliberately does **not** measure was vetoing the five that never read it;
down-the-line the trail wrist tracks well, which is why only half the failure was visible.

The fix is two gates — shoulder and hip refuse the swing, the hands refuse only themselves — and
`PivotObservation.hands` is now `tuple[float, float] | None`. **P8 owes ADR-029 a third addendum
for it** (`docs/README.md`'s ADR-029 row → **2**): the ADR's Decision 6 says the hands are drawn and
unmeasured, and the shape did not carry that far enough to stop them refusing on everyone else's
behalf. The synthetic fixture places all 33 landmarks at `visibility=1.0`, so it can never reproduce
this — `test_unreadable_wrists_cost_the_hands_and_nothing_else` blinds both wrists and asserts the
*other two points are unchanged*, because a test that only checked `hands is None` would pass on a
producer that had stopped placing them too.

**`_dtl_placements` now takes the smoothed frames and the anchor tuple, not the raw clip and a
`SwingAnchors`.** The rear clip acquired a second reader in this phase, and the plan's shape would
have had each of them smooth the same frames and build `(motion_start, top, impact)` from the same
`SwingAnchors` independently — two copies of one conversion, in a repo whose reason for reusing
`dtl_anchors` at all was that the frames the trajectory reads and the frames the warp pins must not
be able to disagree. Both now resample onto literally the same three numbers, converted once at the
call site.

**The version bump's claim is not "ten rows appear".** Ten is a ceiling: a view whose swing cannot
be resampled records none of its five, a single check that refuses drops its own row, and a bundle
with no rear clip records only the face-on half — and none of it reaches `unscored`, because that
list is `UnscoredCheckpoint`s and nothing here is a checkpoint. The ledger entry says so, and
`test_an_unsegmentable_swing_records_no_pivot_rather_than_a_flat_one` is the pin that matters most:
every check is a magnitude, so a swing recorded as all-zeroes would read as the cleanest turn in the
corpus rather than as the absence it is.

**Measured over the corpus on 2026-09-09**, after the fix: fifteen artifacts re-analysed, **ten
pivot rows each** with no check refused, every `overall_score`, `checkpoint_scores` entry and
`unscored` list byte-identical, nothing lost. Read the reasoning and not the digits, but two
readings are worth carrying into P6 and any future band. The two views are visibly two scales —
median hip drift 0.58 face-on against 0.26 down-the-line, shoulder drift 0.35 against 0.14. And the
reversal pair is the strong case for the caveat rather than a fault in these swings: face-on the
backswing figure runs 5.0–**71.0**° and the downswing 0.3–**37.3**°, while their `_dtl` partners sit
at 0.0–2.9° and 0.0–0.8°. That is §9's projection singularity in real data on the first run — the
face-on shoulder line collapses toward zero width at the top, which is exactly where
`BACKSWING_SPAN` ends — and it is why `_ANGLE_INTERIM_FACE_ON` says *read the pair, never one
alone*. A band cut off the face-on reversal alone would be cut off the noise.

`analysis/engine.py`: a `_pivot_measurements(frames, anchors, view)` helper — one helper, both
views, since P3 gave the producer one signature. It calls `pivot_observations` once, runs the
five `PIVOT_CHECKS` over the result, and emits a `Measurement` per registry row *for that view*,
with name/unit/detail off `pivots.spec_for` (the `placement_spec` precedent). Called from
`_measurements` after `_placements` for the face-on view, and folded into the same
`swing.model_copy(update={"measurements": …})` block that already appends `_dtl_placements` for
the second.

**Sources: `pose:face_on` and `pose:down_the_line`** — and the second one needs a contract change
in the same commit. `contracts/career.py::CorpusSwing.artifact_key` maps *any* `pose:`-prefixed
measurement to `pose:{face_on_sha256}`, and the corpus holds no hash for the down-the-line clip,
so `_dtl` rows would dedupe on the wrong artifact: two different second-camera clips over one
face-on clip would collapse to a single sample. Add a `POSE_DTL_SOURCE = "pose:down_the_line"`
constant and an `artifact_key` branch returning `None` for it — the module's own vocabulary,
"returning None is the assertion that there is no reading here at all", which is exactly true
while that clip has no identity. Face-on pivot rows key normally and **do** enter the personal
baseline, as `flight_*` already does; that is a decision, recorded in
[ADR-029](decisions/029-pivot-points.md)'s addendum, not an oversight.

`contracts/swing.py`: `ANALYSIS_VERSION` 15 → 16 plus the `#: 15 -> 16 (2026-…, M17): …` ledger
line — a stored artifact is *missing* the pivot rows, not *disagreeing* about anything.

*Verify by:* `tests/analysis/test_engine.py` (face-on pivot rows on `SwingResult.measurements`,
names/units off the registry, five of them); `tests/analysis/test_engine_bundle.py` (`_dtl` rows
only for a two-view bundle; face-on / `_dtl` names never collide; `overall_score` unchanged);
`tests/contracts/test_career.py` (a `pose:down_the_line` measurement gets no artifact key, so it
contributes no baseline sample, while a `pose:face_on` one keys on the face-on hash);
`tests/analysis/test_dispersion.py` — a new pin that `pivot_measurement_names()` is disjoint
from `METRIC_TARGETS` and from `POSE_MEASUREMENTS | SHOT_MEASUREMENTS`, and a dispersion build
over a pivot metric reports no tolerance;
`test_the_version_ledger_documents_the_installed_version` covers the ledger line. Then
`scripts/reanalyze.py --all` and record in `WORKLOG.md` that every `overall_score` /
`checkpoint_scores` is byte-identical and how many pivot rows each artifact gained.

Commit: `M17 P5: the pivot numbers reach the engine; ANALYSIS_VERSION 15 -> 16`

### [x] P6 — the numbers leave the system saying they are interim

**Built 2026-09-10.** Full suite green, `ruff` and `mypy` clean. Built as written; one call worth
recording. `_ONLY_THESE_FUNDAMENTALS` (the `READING_THIS_DATA_HONESTLY` bullet that says "Spine
angle, hip **rotation**, swing plane and club path are NOT measured here") is left untouched
rather than cross-referenced against the new `rotation` block: that bullet is scoped to the six
*judged checkpoints*, which still measure none of those, and `tour_trajectory_q`'s placement
already introduced a motion-shaped number without needing an edit there. The new
`_PIVOTS_ARE_INTERIM_PROSE` bullet sits two bullets below it and says plainly that `rotation` is
unjudged and has no band, which resolves the apparent tension without touching a sentence that
is still true in its own scope.

Four surfaces read `measurements` and all four must learn about pivots, or the caveat describes
rows the other three still present as ordinary numbers.

**`contracts/caveats.py`** — a derived block (mirror `_PLACEMENTS_ARE_NOT_SCORES` /
`_PLACEMENTS_PER_VIEW`) naming every `PIVOT_MEASUREMENT_REGISTRY` entry and stating they are an
interim 2-D-per-view rotation reading — unjudged, foreshortened, per-view and never blended, not
in `overall_score`, fiducial calibration the exit — wired into `READING_THIS_DATA_HONESTLY`. Add
a `PIVOTS_ARE_INTERIM` constant.

**`api/state.py`** — `resolve_pivots(analysis)` beside `resolve_placements`, same shape and same
tolerant-reader posture (unit and detail off the stored entry, view and `interim_reason` off the
spec; membership by `PIVOTS_BY_NAME`, never a `pivot_` prefix test, for the reason stated there).
This is the implementation; the next two are its channels. `resolve_placements`' docstring says
"one resolver, two channels" — extend that sentence rather than writing a second splitter.

**`api/app.py` + `api/static/results.html`** — the route gains a `"rotation"` key beside
`"population"`, and the page renders it as its own block with the interim sentence attached.
Without this, P5's new rows appear on the results page as bare unexplained floats, which is the
failure `contracts/placements.py` exists to prevent.

**`feedback/coach.py`** — the `PLACEMENTS_BY_NAME` partition gains its `PIVOTS_BY_NAME` sibling,
so a pivot row reaches the model labelled rather than in the plain-pose-metric bucket.

**`mcp/query.py`** — a `SwingView.rotation` field (a small `PivotView` model) filled from
`resolve_pivots`, exactly as `placements` is filled from `resolve_placements`; its description
interpolates `PIVOTS_ARE_INTERIM`. `_measurements` skips `PIVOTS_BY_NAME` names the way it
already skips `PLACEMENTS_BY_NAME` ones.

Reuse: `contracts/caveats.py` (`_and_list`, `_count_word`, `fill`, the
`_PLACEMENTS_ARE_NOT_SCORES` shape); `api/state.py` (`_measurement_entries`,
`resolve_placements`); `mcp/query.py` (`PlacementView`, the `_measurements` split).

*Verify by:* a new `tests/test_docs_truth.py` pin (every registry name appears in
`READING_THIS_DATA_HONESTLY`; `SwingView.rotation`'s description interpolates the constant);
`tests/api/test_state.py` (`resolve_pivots` over a stored artifact — every registry row
resolved, an old artifact without them resolving to an empty list); `tests/mcp/test_query.py` (a
swing with pivot rows returns them under `rotation`, not `measurements`);
`tests/feedback/test_coach.py` (a pivot row reaches the prompt labelled interim);
`test_no_mcp_field_description_miscounts_the_panel` stays green — derive any count, never state
it.

Commit: `M17 P6: the rotation numbers leave the system saying they are interim`

### [x] P7 — re-render every stored swing

**Built 2026-09-10.** Operational, no source change. Ran
`.venv/Scripts/python.exe scripts/reanalyze.py --all --video --verbose` over all fifteen stored
swings. **15/15 re-analyzed, 0 failures, 0 flagged for attention** (exit code 0). Every one of the
five compared fields — `version`, `score`, `measurements`, `window`, `anchors` — reported `no
change`: the new overlay redraws pixels, nothing it draws moves a number. Every render's `[reads
back N]` count matched the `Wrote N aligned frames` count exactly, on all fifteen files (frame
counts ranged 111–202 across the corpus), so no file decodes short of what was written. Every
render opened codec `avc1` — the OpenH264 stderr wall (`Failed to load OpenH264 library` /
`VIDEOIO/FFMPEG: Failed to initialize VideoWriter`, once per render) is the documented noise from
`pose/side_by_side.py`'s comment above `_CODECS`, not a failure: OpenCV's bundled FFmpeg gives up
on its GPL-free `libopenh264` and falls back to Media Foundation, which is exactly what `avc1`
succeeding fifteen times confirms.

**Visual check** (not yet a browser — that honesty still applies below): read three frames back
out of `2026-08-23/2`'s fresh `aligned.mp4` with `cv2.VideoCapture` — address, mid-downswing and
follow-through. Both panels carry all three magenta pivot markers (shoulder-line midpoint,
hip-line midpoint, hand midpoint) and both cyan guide lines at every sampled frame. The
down-the-line panel is the one worth reading closely: at address its shoulder line is nearly a
point (§9's singularity — the line is nearly perpendicular to that camera at address), and by
follow-through it has opened into a long, clearly diagonal line — the guide line visibly tracking
the turn, exactly as the exit criterion asks. The face-on panel shows the complementary
foreshortening, shrinking through the same swing. Neither collapse is a defect; it is the
projection singularity P3/§9 already named, now visible in a real render rather than only in the
angle statistics.

**Still unverified**: actual playback in a browser. The three frames above were decoded directly
from the file, which confirms the pixels on disk are correct; it does not confirm a browser plays
this `avc1` file inline the way `aligned.mp4` is meant to be watched. The M14 P1 / M17 P1 "unverified
in a browser" note carries forward unchanged.

Commit: `M17 P7: re-render every stored swing onto the new overlay and engine`

### [ ] P8 — docs reconciled, `GOLF-5` merged

Flip [ADR-029](decisions/029-pivot-points.md) Status to built (a *second* addendum only if
building corrected the design again — the ADR-028 precedent; the 2026-09-09b one is already
there, so `docs/README.md`'s ADR-029 row goes to **2** if you write one). This doc →
`**Status: 9/9 phases built.**`, every `### [x]`, a `**Built <date>.**` note per phase, tier
TARGET → REFERENCE, a "what is unverified" paragraph, an exit-criteria paragraph, and the ⚠️
banner about the P2–P6 rewrite folded into that history. `docs/README.md` — M17 row → REFERENCE,
`9/9 phases built`; ADR-029 → built; re-check the `git ls-files '*.md'` count on the branch.
`ROADMAP.md` — status row → ✅ Done, `## M17` detail → done, phases past-tense, exit-criteria
paragraph, `Last Updated`. `WORKLOG.md` — closing entry, "Next: merge `GOLF-5` on the user's
go-ahead".

*Verify by:* full `-m pytest` and `-m pytest tests/test_docs_truth.py` (phase count 9/9 both
sides, tier REFERENCE both sides, ADR row counts, markdown doc count); `ruff`; `mypy`. Then
`git merge --no-ff GOLF-5 -m "Merge GOLF-5: M17 pivot points"` and `git branch -d GOLF-5`.

Commit: `M17 P8 (docs): the joint centres are drawn, tracked and flagged`

## What this milestone is not

- **It is not a rotation checkpoint.** No `CheckpointSpec`, no `ranges.json` row, no
  `overall_score` movement. [ADR-010](decisions/010-benchmark-ranges.md) §2 and
  [ADR-029](decisions/029-pivot-points.md) Option C.
- **It is not X-factor, spine angle, hip rotation or the kinematic sequence.** Those are the
  fixed-rig 3-D measurements [ADR-011](decisions/011-camera-synchronization.md) reserves; the
  interim instrument here does not approximate them and does not claim to.
- **It is not the fiducial-marker work.** M17 builds the seam
  (`contracts/pivots.py::FrameOfReference`) that the calibrated source will slot into; it does
  not read a marker, solve a pose, or add a `CALIBRATED_3D` producer.
- **It is not club-path or club-head tracking.** The slot exists; nothing fills it.
- **It is not a motion trail.** Per-frame markers and lines only; the trail is deferred in
  [ADR-029](decisions/029-pivot-points.md).
- **It is not a hand-path measurement.** The hands are drawn as a pivot point and carry no
  metric: face-on hand tracking over the whole clip is 0.63–0.68 (M14 P3), so a through-swing
  hand number would mostly report the tracker. Jitter is measured on the hip centre instead.
