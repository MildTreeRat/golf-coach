# M17 — Pivot points: the shoulder line, the hip line, and the three points they turn about

> **Tier: TARGET.** This is the plan, not a record — every phase below is unbuilt as of
> 2026-09-09, and the numbers and file names are what P0 intends, not what shipped. Verify
> against code once a phase lands and flip its checkbox. The governing decision is
> [ADR-029](decisions/029-pivot-points.md); the constraint it is designed around is
> [ADR-011](decisions/011-camera-synchronization.md) and its 2026-08-05 addendum.

**Status: 0/9 phases built.** P0 writes this document, [ADR-029](decisions/029-pivot-points.md)
and the [ADR-011](decisions/011-camera-synchronization.md) addendum, and puts M17 on the board.
P1 draws the pivot markers and the two rotation lines into the overlay. P2 lands
`contracts/pivots.py` — the shape a fiducial-calibrated source will later produce instead of
pose. P3 and P4 build `analysis/pivot.py`: the pivot paths, then the stdlib rule checks that
call a turn wonky. P5 wires the numbers into the engine and moves `ANALYSIS_VERSION` 15 → 16.
P6 gives the numbers a voice — a derived caveat and an MCP field, both saying they are interim.
P7 re-renders every stored swing. P8 reconciles the docs and merges `GOLF-5`.

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
6. **"`analysis/pivot.py` can read `FrameKeypoints` in the rule checks."** The rule checks read
   `PivotObservation`, which is the point of the seam — the fiducial source produces the same
   shape from calibrated 3-D and the checks are reused unchanged. Only the one producer M17
   ships (`pivot_observations`) touches `FrameKeypoints`.

## The phases

P1 and P2 are independent of each other; P1 lands first by the overlay-before-contracts
ordering rule. P3 needs P2; P4 needs P3; P5 needs P4; then the chain is linear to the merge.

| Phase | Scope | Principal files | Gate |
|---|---|---|---|
| P0 | The milestone enters the repo | ADR-029, this doc, ADR-011 addendum, `docs/README.md`, `ROADMAP.md`, `WORKLOG.md` | doc-truth |
| P1 | The overlay draws the pivot points and the rotation lines | `pose/overlay.py` | — |
| P2 | `contracts/pivots.py` — the fiducial-ready seam + the registry | `contracts/pivots.py` | — |
| P3 | `analysis/pivot.py` — the pivot paths | `analysis/pivot.py` | — |
| P4 | The stdlib rule checks that call a turn wonky | `analysis/pivot.py` | — |
| P5 | The numbers reach the engine; `ANALYSIS_VERSION` 15 → 16 | `analysis/engine.py`, `contracts/swing.py` | version ledger |
| P6 | The numbers leave the system saying they are interim | `contracts/caveats.py`, `mcp/query.py` | doc-truth |
| P7 | Re-render every stored swing | `scripts/reanalyze.py --all --video` (operational) | — |
| P8 | Docs reconciled, `GOLF-5` merged | ADR-029, this doc, `docs/README.md`, `ROADMAP.md`, `WORKLOG.md` | doc-truth |

Every phase's gate also includes the full `.venv/Scripts/python.exe -m pytest`,
`-m ruff check src tests scripts` and `-m mypy src`. One commit per phase, message
`M17 P<n>: <lowercase evocative sentence>`, both trailers
(`Co-Authored-By: Claude Sonnet 5 …` and `Claude-Session: …`).

### [ ] P0 — the milestone enters the repo

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

### [ ] P1 — the overlay draws the pivot points and the rotation lines

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
`_GUIDE_COLOR` line between the shoulder landmarks; no marker when one endpoint is dim). Then
render one clip through `scripts/run_pose.py` and look at it.

Commit: `M17 P1: the shoulder line, the hip line, and the three points they turn about`

### [ ] P2 — `contracts/pivots.py`: the fiducial-ready seam + the registry

Create `src/golf_coach/contracts/pivots.py`, modelled field-for-field on
`contracts/placements.py`:

- `FrameOfReference(StrEnum)` — `IMAGE_PLANE_FACE_ON`, `IMAGE_PLANE_DTL`, `CALIBRATED_3D`.
- `PivotObservation` (NamedTuple) — the three pivot points as `(x, y)` in the declared frame,
  `shoulder_line_deg`, `hip_line_deg`, `frame_of_reference`, `club_head: tuple[float, float] |
  None = None`.
- `PivotMeasurementSpec` (NamedTuple) — `name`, `label`, `view`, `unit`, `detail`,
  `interim_reason`.
- `PIVOT_MEASUREMENT_REGISTRY` (tuple, order load-bearing, append-only), `PIVOTS_BY_NAME`,
  `pivot_measurement_names()`, `spec_for()`.

Import `FACE_ON` / `DOWN_THE_LINE` from `contracts/placements.py` — one spelling. Starter
metric set, each a face-on + `_dtl` pair (the final list is [ADR-029](decisions/029-pivot-points.md)'s
to pin — keep it small): `pivot_hip_axis_drift_norm`, `pivot_shoulder_axis_drift_norm`,
`pivot_path_jitter_norm`, `shoulder_turn_reversal_deg`.

*Verify by:* `tests/contracts/test_pivots.py` (new), mirroring `tests/contracts/test_placements.py`
— no duplicate names; every `view` is one of `placements.py`'s two exact strings; `spec_for`
raises on a miss; every spec has a non-empty `interim_reason`; face-on and `_dtl` names come in
matched pairs; `_norm` iff `shoulder_widths`, `_deg` iff `degrees`.

Commit: `M17 P2: contracts/pivots.py — the shape a fiducial source will produce instead`

### [ ] P3 — `analysis/pivot.py`: the pivot paths

Create `src/golf_coach/analysis/pivot.py` — stdlib + `contracts/` only; the docstring states
it is the single implementation, the way `analysis/trajectory.py` does. Provide
`pivot_observations(keypoints, phases, *, wrist, frame_of_reference) -> list[PivotObservation]
| None` and a `from_anchors` variant (the down-the-line path already holds a `SwingAnchors`,
mirror `trajectory.placement_from_anchors`). Resample onto event time
(`trajectory.anchors_from_phases` + `trajectory.sample_positions`) so a slow-mo clip and a
real-time one — and face-on vs down-the-line — are comparable; confident frames only;
hip-relative; scaled by one shoulder width for the whole swing.

Reuse: `analysis/trajectory.py` (`anchors_from_phases`, `sample_positions`, `_interpolate_gaps`);
`analysis/measure.py` (`midpoint_series`, `mean_of`, `shoulder_width`, `direction_series`,
`MIN_VISIBILITY`, `MIN_SHOULDER_WIDTH`); `analysis/phases.py` (`LEAD_WRIST` / `TRAIL_WRIST`).
The engine smooths the keypoints before calling, as it does for `measure.py`.

*Verify by:* `tests/analysis/test_pivot.py` (new) — the ideal synthetic swing from
`tests/analysis/conftest.py` produces a clean observation list; `frame_of_reference` on the
output matches the argument; collapsed anchors or missing landmarks return `None`.

Commit: `M17 P3: analysis/pivot.py — the swing as three points moving`

### [ ] P4 — the stdlib rule checks that call a turn wonky

Extend `analysis/pivot.py` with `PIVOT_MEASUREMENTS: dict[str, MeasureFn]`, keyed 1:1 to
`PIVOT_MEASUREMENT_REGISTRY`, each `(frames, phases) -> MeasureOutcome` computed from
`pivot_observations`. Angles use a circular mean (the `direction_series` precedent); refusals
draw only from `MEASUREMENT_REASONS`. The checks: lateral excursion of the hip / shoulder
centre from its address `x` (aspect-immune, `x`-over-`x`); frame-to-frame roughness of a pivot
path; the largest against-the-turn move of the shoulder-line angle within the backswing and
within the downswing (0 = monotone).

Reuse: `analysis/measure.py` (`MeasureOutcome`, `MeasureFn`, `MEASUREMENT_REASONS`,
`phase_bounds`, `address_sample_bounds`), `contracts/unscored.UnscoredReason`.

*Verify by:* extend `tests/analysis/test_pivot.py` — the ideal swing scores every check
small/clean; a wonky synthetic (inject a mid-backswing shoulder-line reversal, a lateral hip
slide, per-frame jitter) fires the matching check large while the others stay clean; a refusal
path returns a reason from `MEASUREMENT_REASONS`.

Commit: `M17 P4: the rule checks that call a turn wonky`

### [ ] P5 — the numbers reach the engine; `ANALYSIS_VERSION` 15 → 16

`analysis/engine.py`: a `_pivot_measurements(smoothed, phases)` helper called from
`_measurements` after `_placements` (face-on, `source="pose:face_on"`, name/unit/detail off the
registry via `spec_for` — the `placement_spec` precedent); a
`_dtl_pivot_measurements(down_the_line.frames, dtl_anchors, handedness)` call folded into the
same `swing.model_copy(update={"measurements": …})` block that already appends
`_dtl_placements`. `contracts/swing.py`: `ANALYSIS_VERSION` 15 → 16 plus the `#: 15 -> 16
(2026-…, M17): …` ledger line — a stored artifact is *missing* the pivot rows, not
*disagreeing* about anything.

*Verify by:* `tests/analysis/test_engine.py` (face-on pivot rows on `SwingResult.measurements`,
names/units off the registry); `tests/analysis/test_engine_bundle.py` (`_dtl` rows only for a
two-view bundle; face-on / `_dtl` names never collide; `overall_score` unchanged);
`tests/analysis/test_dispersion.py` — a new pin that `pivot_measurement_names()` is disjoint
from `METRIC_TARGETS` and from `POSE_MEASUREMENTS | SHOT_MEASUREMENTS`, and a dispersion build
over a pivot metric reports no tolerance;
`test_the_version_ledger_documents_the_installed_version` covers the ledger line. Then
`scripts/reanalyze.py --all` and record in `WORKLOG.md` that every `overall_score` /
`checkpoint_scores` is byte-identical and how many pivot rows each artifact gained.

Commit: `M17 P5: the pivot numbers reach the engine; ANALYSIS_VERSION 15 -> 16`

### [ ] P6 — the numbers leave the system saying they are interim

`contracts/caveats.py`: a derived block (mirror `_PLACEMENTS_ARE_NOT_SCORES` /
`_PLACEMENTS_PER_VIEW`) naming every `PIVOT_MEASUREMENT_REGISTRY` entry and stating they are an
interim 2-D-per-view rotation reading — unjudged, foreshortened, per-view and never blended,
not in `overall_score`, fiducial calibration the exit — wired into `READING_THIS_DATA_HONESTLY`
so it reaches the coaching call. Add a `PIVOTS_ARE_INTERIM` constant. `mcp/query.py`: a
`SwingView.rotation` field (a small `PivotView` model), filled by splitting `measurements` on
`PIVOTS_BY_NAME` (the `population` / `simulated` split-out pattern, not a name prefix), its
description interpolating `PIVOTS_ARE_INTERIM`.

Reuse: `contracts/caveats.py` (`_and_list`, `_count_word`, `fill`, the
`_PLACEMENTS_ARE_NOT_SCORES` shape); `mcp/query.py` (`PlacementView`, the `_measurements`
split).

*Verify by:* a new `tests/test_docs_truth.py` pin (every registry name appears in
`READING_THIS_DATA_HONESTLY`; `SwingView.rotation`'s description interpolates the constant);
`tests/mcp/test_query.py` (a swing with pivot rows returns them under `rotation`, not
`measurements`); `test_no_mcp_field_description_miscounts_the_panel` stays green — derive any
count, never state it.

Commit: `M17 P6: the rotation numbers leave the system saying they are interim`

### [ ] P7 — re-render every stored swing

Operational; no source change (an optional one-line `scripts/reanalyze.py` docstring touch).
Run `.venv/Scripts/python.exe scripts/reanalyze.py --all --video`. Open one freshly rendered
`aligned.mp4` — both panels show the three pivot markers and the two rotation lines, and the
down-the-line panel's lines track the turn. Record in `WORKLOG.md` and in this doc's P7
section: score byte-identity, `frames_read == frames` on every bundle, and the one visual check
(the M14 P1 "unverified in a browser" honesty applies until a browser has shown it).

Commit: `M17 P7: re-render every stored swing onto the new overlay and engine`

### [ ] P8 — docs reconciled, `GOLF-5` merged

Flip [ADR-029](decisions/029-pivot-points.md) Status to built (an addendum only if building
corrected the design — the ADR-028 precedent). This doc → `**Status: 9/9 phases built.**`,
every `### [x]`, a `**Built <date>.**` note per phase, tier TARGET → REFERENCE, a "what is
unverified" paragraph, an exit-criteria paragraph. `docs/README.md` — M17 row → REFERENCE,
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
