# M10 — Alignment Accuracy: the two panels leave address together

> **Tier: REFERENCE.** Read the reasoning, not the digits. Every table here is a measurement of
> the 15 bundles that were on disk on the day it is dated — 2026-08-25 for the bug, 2026-08-26 for
> the repair in P10 — and a bundle re-analysed after either date will not match. The *why* behind
> the alignment itself is [ADR-015](decisions/015-handheld-two-phone-capture-and-event-anchored-alignment.md);
> this document is the repair, as a phase list.

**Status: 10/10 phases built** — the alignment-math track (P1-P3), the selection track (P4-P8),
P9's paperwork, and P10's re-analysis of the corpus on 2026-08-26. Every stored bundle is on
`ANALYSIS_VERSION` 11; what that moved, and the one defect it exposed underneath, are in P10's
*As built* below.

## What this milestone is

The side-by-side replay puts the down-the-line panel into its takeaway while the face-on panel is
still standing at address. Up to **0.42 s** — 25 frames at 60 fps — at the moment the render opens.

**It is not one bug.** It is a bad-anchor problem upstream wearing an alignment problem's clothes,
plus one real flaw in the alignment's degraded tier. `analysis/alignment.py` is largely *correct*:
it detects the trouble, refuses the soft anchor, and writes an accurate note. What it cannot do is
render a good video from anchors that describe the wrong swing — and what it does wrong is degrade
in **ratio** where it should degrade in **time**.

**Outcome:** every bundle either aligns to within a few frames through the takeaway, or says
honestly that it could not. No bundle claims `full` while drifting 0.2 s.

## Three things that will look obvious and are wrong

- **Do not "fix the down-the-line phase detector."** DTL is not intrinsically harder here. Given a
  correct window its takeaway signal margin is 4.4–7.4× the quiet threshold, against face-on's
  4.8–12.5. The six failures are all windowing, not viewing angle. M7_TWO_PHONE_SPIKE Q1 is still
  open but is **not** what this milestone is about.
- **Do not widen `_PLAUSIBLE_DOWNSWING_S`.** Its comment (`phases.py:355`) already records why the
  band cannot move: widening it to admit a 30 fps clip's 0.60 s swallows the whole setup-move
  cluster at 60 fps. P4 admits the marginal case by *matching a reference*, not by loosening.
- **Do not delete the tau axis in favour of seconds.** ADR-015's normalized axis is the right
  frame. The failure is that normalizing by a *wrongly measured* downswing amplifies error instead
  of absorbing it, so the **anchors** need a real-time sanity check even though the **warp** does
  not. P9 records exactly that distinction.

## Reading order for a fresh session

`CLAUDE.md`, then ADR-015, then this file, then the two-to-four files your phase names. Every phase
states its own files and what to reuse, so a phase number is a complete handoff.

---

## The evidence

All figures measured 2026-08-25 over `data/processed/sessions/`, 15 bundles, 11 of them from
`2026-08-23`. Baseline tiers: **4 `full`, 3 `top_impact`, 4 `impact_only`.**

### Why a bad window is fatal

`_motion_start` (`phases.py:517`) sets its quiet threshold to `_MOTION_QUIET_FRAC` (5%) of the peak
wrist speed over frames `[1, top]`. On an unwindowed 20-second bay clip that peak is a walk-in or a
practice swing, so the threshold lands 20–30× too high, the entire takeaway reads as "quiet", and
the walk-back sails through it and stops under the top.

`ta/thr` = mean takeaway wrist speed ÷ quiet threshold; it must exceed 1.

| | face-on | down-the-line |
|---|---|---|
| windowed properly | 4.8 – 12.5 (11/11) | 4.4 – 7.4 (7/11) |
| window missing or whole-clip | — | **0.00, 0.04, 0.24** (sessions 4, 5, 10) → backswing collapses to 1 frame |

### A1 — `select_swing` uses the LEAD wrist; the anchors use the TRAIL wrist

`candidate_downswings` (`phases.py:290`) calls `_lead_wrist_xy(keypoints)` with the default
`_LEAD_WRIST`, and `select_swing` (`phases.py:402`) takes no `wrist` argument. `pipeline._auto_window`
(line 321) calls it identically for both views. But `engine.analyze_swing_bundle` (line 413) then
segments DTL with `wrist=TRAIL_WRIST` — its own comment says the lead wrist is tracked in 39% of
DTL frames. The window and the anchors come from two different landmarks.

| Session | lead-wrist pick | trail-wrist pick |
|---|---|---|
| 10 | downswing **9.673 s** → window `(0, 3619)` on a 1996-frame clip = *no window* | 0.400 s → `(1099, 1315)` ✔ |
| 4 | **declined** (4 candidates, 0 plausible) | 0.484 s → `(1130, 1391)` ✔ |
| 3 | 0.417 s → `(958, 1183)` ✔ | 0.233 s → `(1637, 1763)` ✘ |
| 6 | 0.284 s → `(1056, 1209)` ✔ | 0.183 s → `(1693, 1792)` ✘ |

### A2 — selection is per-view, so "last plausible" picks different swings

Sessions 3 and 6 above: on the trail wrist the rule finds descents at frames 1707 and 1748, long
after impact, because the DTL phone keeps rolling on the busy side of the bay. **Switching wrist
alone regresses these**, which is why P1/P2 and P4/P5 are both required. Face-on is the reliable
view — 11/11 sane windows — so once its swing is chosen, DTL's is not a free choice.

### A3 — the plausibility band rejects the real swing when it is marginally outside

Session 5 DTL: the true descent is `(1540, 1568)` = 0.467 s, just past the 0.45 s bound. Filtered
out; with 3 candidates the single-candidate escape at `phases.py:452` cannot fire; declines.

### A4 — `_WINDOW_LEAD = 5` leaves too little quiet address

`address_room = 5·downswing − takeaway`, in frames:

| Session | downswing | 5·dn | takeaway | address room | motion start detected |
|---|---|---|---|---|---|
| 2 | 13 | 65 | 46 | **19** | **False** |
| 4 | 12 | 60 | 42 | **18** | **False** |
| 5 | 11 | 55 | 38 | **17** | **False** |
| 8 | 28 | 140 | 53 | 87 | True |

A feedback loop: short measured downswing → short window → no address → undetected motion start →
the `top_impact` fallback → which multiplies that same short downswing by 3.5.

### B1 — the `TOP_IMPACT` fallback is not symmetric *(this is what the user sees)*

`_estimated_motion_start` (`alignment.py:503`) returns `top − 3.5 × downswing_frames` from **each
clip's own** downswing. The docstring at `alignment.py:255` claims both clips "fall back to the same
tour-median estimate ... so the pre-top region degrades symmetrically". It is the same **ratio**,
not the same **duration**, and the two views routinely disagree on the downswing by 10–40%, which
×3.5 amplifies:

| Session | face-on tau=0 lands | DTL tau=0 lands | disagreement at tau=0 |
|---|---|---|---|
| 10 | 84 frames (1.401 s) before top | 66 frames (1.101 s) before top | **0.300 s** |
| 3 | 80 frames (1.334 s) | 98 frames (1.634 s) | **0.300 s** |
| 1 | 80 frames (1.334 s) | 91 frames (1.518 s) | 0.184 s |

`DEFAULT_TAU_RANGE` opens at tau = −0.4, so the render *starts* with that gap grown by 40% — 0.42 s,
25 frames at 60 fps. `_shared_tops` (`alignment.py:347`) already solves this exact problem for the
`IMPACT_ONLY` tier by converting a shared duration through each clip's own fps. `TOP_IMPACT` has no
equivalent.

### B2 — the tempo cross-check is blind to the error it guards

`_TEMPO_AGREEMENT` (`alignment.py:65`) compares backswing:downswing **ratios**. A ratio divides out
the downswing, so when both views' errors scale together the ratio agrees while the durations do not:

| Session | quality | tempo ratios | gap vs 0.35 | backswing durations | real disagreement |
|---|---|---|---|---|---|
| 9 | `full` | 4.06 vs 4.25 | 0.045 → passes | 1.084 s vs 0.851 s | **0.233 s** |
| 6 | `full` | 2.36 vs 2.14 | 0.093 → passes | 0.984 s vs 0.784 s | **0.200 s** |
| 8 | `full` | 1.89 vs 2.60 | 0.273 → passes | 0.884 s vs 0.867 s | 0.017 s ✔ genuinely fine |

Bundles reported `full` still ship 12–14 frames of visible misalignment. **The complaint is not
confined to the degraded tiers**, which is why Group B must be fixed even though Group A is the
larger defect.

---

## Phases

Each phase is independently commit-ready and has at most one design decision. `tests/` mirrors
`src/golf_coach/` package by package.

| Track | Phases | Notes |
|---|---|---|
| Alignment math | P1 → P3 | **Independent of P4–P8** — can run in parallel. P1 is a zero-behaviour cleanup and a safe first commit. |
| Selection | P4 → P8 | Strictly in order. P4+P5 alone regress sessions 3 and 6; P7 is what rescues them. |
| Close | P9, P10 | Last, in order. |

Run after every phase:

```bash
.venv/Scripts/python.exe -m pytest
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
```

---

### [x] P1 — remove the dead comment and the false symmetry claim

**Goal.** Zero behaviour change. Make `alignment.py` stop asserting something the evidence
disproves, before P2 changes what it does.

**Files.** `src/golf_coach/analysis/alignment.py`.

**Detail.**
- Delete the orphaned comment block at lines **101–109**. It documents a constant that no longer
  exists — left over from an earlier edit — and reads as a docstring for `DEFAULT_TAU_RANGE`, which
  it is not about.
- Correct `align_swings`' docstring (line ~255): "otherwise both clips take the same tour-median
  estimate, so the pre-top region degrades symmetrically" is false. Say what it actually does today
  (the same *ratio*, applied to each clip's own downswing) and mark it as the thing P2 fixes.

**Tests.** None. No behaviour changed.

**Done when.** Suite green at its current count; `ruff` and `mypy` clean.

---

### [x] P2 — the `TOP_IMPACT` fallback shares a duration, not a ratio *(fixes B1)*

**Goal.** When the soft anchor is refused, both panels place tau=0 at the same *real-time* distance
before their own top.

**Files.** `src/golf_coach/analysis/alignment.py`, `tests/analysis/test_alignment.py`.

**Reuse.** `_shared_tops` (line 347) already does this arithmetic for the `IMPACT_ONLY` tier —
including the "reference is the face-on clip" rule at lines 367–369 and the convert-through-own-fps
step at lines 385–386. Follow it; do not invent a second convention. `_FALLBACK_TEMPO_RATIO` and
`_PLAUSIBLE_DOWNSWING_S` are already imported from `phases`.

**Detail.**
- Add a pair-aware helper beside `_estimated_motion_start`: derive **one** backswing duration in
  seconds (`_FALLBACK_TEMPO_RATIO × reference downswing`, reference chosen as `_shared_tops` chooses
  it), then convert it through each clip's own fps.
- **Keep `_estimated_motion_start` as the no-fps path.** One clip without fps must still degrade the
  way it does today rather than raise — ADR-013, reported not raised.
- Clamp to `>= 0` as the current code does.

**Design decision (the only one here).** The shared duration comes from the **face-on** downswing,
matching `_shared_tops`, because that is the view the detector was tuned on and the only one scored.

**Tests.** `test_both_clips_take_the_same_fallback_when_the_soft_anchor_is_dropped` (line 181)
**encodes this bug** — it asserts the same ratio. Rewrite it to assert the same *duration*, and
keep a sibling test pinning the no-fps path to today's ratio behaviour.

**Done when.** Two clips with the same swing but downswings of 24 and 19 frames place tau=0 within
a frame of each other in seconds.

---

### [x] P3 — cross-check the backswing in seconds *(fixes B2)*

**Goal.** Stop `full` being reported when the two views disagree about the takeaway by 0.2 s.

**Files.** `src/golf_coach/analysis/alignment.py`, `tests/analysis/test_alignment.py`.

**Reuse.** `_relative_gap` (line 512) and `SwingAnchors.backswing_frames` (`contracts/alignment.py`,
~line 138). No new contract field.

**Detail.**
- Add `_BACKSWING_AGREEMENT` beside `_TEMPO_AGREEMENT` (line 65), compared on backswing **durations
  in seconds**, applied inside `align_swings`' `use_soft` chain only when both clips report fps.
- **Size it from the table above, and write the derivation into the comment** — this repo's rule:
  sessions 9 (0.233 s) and 6 (0.200 s) must fail; sessions 8 (0.017 s) and 11 (0.133 s) must pass.
- Extend `_tempo_disagreement_note` (line 315) so a backswing-duration failure names the takeaway
  boundary rather than implying two different swings. It already draws this distinction for the
  downswing at lines 331–340; follow that shape.

**Comment to write.** Why a ratio cannot do this job: a ratio divides out the downswing, so two
views whose errors scale together agree on tempo while disagreeing on time. That is not a loose
threshold, it is a blind one — the reason this constant exists at all.

**Tests.** Two clips agreeing on tempo ratio but disagreeing on backswing seconds refuse the soft
anchor (the session-9 case). An honest pair keeps `full`.

**Done when.** Sessions 6 and 9 would no longer report `full`.

---

### [x] P4 — `select_swing` learns which wrist to read *(fixes A1, part 1)*

**Goal.** Make swing selection able to use the trail wrist. No caller changes yet, so this phase
cannot regress anything.

**Files.** `src/golf_coach/analysis/phases.py`, `tests/analysis/test_select_swing.py`.

**Reuse.** `TRAIL_WRIST` is already exported at `phases.py:87`. `segment_phases` already takes a
`wrist` argument — mirror that signature exactly so there is one convention.

**Detail.**
- Add `wrist: PoseLandmark | None = None` to `candidate_downswings` (line 290) and `select_swing`
  (line 402); thread into the existing `_lead_wrist_xy(keypoints, wrist)`.
- **`_wrist_confident` (line 205) takes a `wrist` parameter and then hardcodes `_LEAD_WRIST` in its
  body.** Same bug one level down; fix it here.

**Tests.** Selection on the trail wrist finds a swing the lead wrist misses. Default behaviour is
unchanged — pin that, since every existing caller relies on it.

**Done when.** All existing `test_select_swing.py` tests pass untouched.

**As built.** Two deviations, both deliberate. The signature is `wrist: PoseLandmark =
_LEAD_WRIST`, keyword-only — `segment_phases`' own signature rather than this plan's `| None =
None`, since the Reuse note's "one convention" is the stronger instruction and a `None` default
buys only a branch. And the `_wrist_confident` fix is **not** behaviour-neutral: `segment_phases`
has passed `TRAIL_WRIST` from `engine.analyze_swing_bundle` since M4 §Phase F, so down-the-line
segmentation was already masking trail-wrist positions with lead-wrist visibility. That path
changes here, ahead of P5. (The file held 7 tests, not 8.)

---

### [x] P5 — the down-the-line view is selected on the trail wrist *(fixes A1, part 2)*

**Goal.** The window and the anchors read the same landmark.

**Files.** `src/golf_coach/api/pipeline.py`, `scripts/align_swings.py`.

**Detail.**
- `pipeline._auto_window` (line 321) gains a `wrist` argument; the DTL call site (line 527) passes
  `TRAIL_WRIST`, matching `engine.analyze_swing_bundle` line 413.
- `scripts/align_swings.py`: the `--list-swings` path (line 91) and its `_auto_window` (line 215)
  take the same argument. The existing comments at lines 90 and 104 promise the listing a human
  chooses from is identical to the set `select_swing` chooses from — this keeps that true.

**Expected at this point.** Sessions 4 and 10 recover; **sessions 3 and 6 regress** (see A2). That
is expected and P7 fixes it — do not tune anything to paper over it.

**Tests.** `tests/api/test_pipeline_shot_screen.py` and the bundle tests should stay green. Add a
pin that the DTL view is windowed on the trail wrist.

**Done when.** Session 10's DTL window is ~216 frames, not 3619.

**As built.** Session 10's DTL window is `(1099, 1315)` — 216 frames, from 3619 — and that bundle
now reports `full` with both views measuring a 24-frame downswing. Three deviations:

- **`phases._LEAD_WRIST` is now public as `LEAD_WRIST`**, which this plan did not list and which
  touches `phases.py`. A caller that chooses per view has to be able to name *both* halves of the
  pair; the alternatives were the `| None = None` default P4's As-built note declined, or a second
  literal `PoseLandmark.LEFT_WRIST` written in `api/`. Zero behaviour change.
- **`scripts/align_swings.py`'s anchors were fixed too** — its two `anchors_from_keypoints` calls
  passed no `wrist` at all, so it segmented *both* clips on the lead wrist while `engine` has used
  the trail wrist for down-the-line since M4 §Phase F. This is an oversight in the phase list, not
  a consequence of P5: the script appears in this document only here and in P6's Reuse note, so no
  later phase would have caught it. Left alone, P5 would have given the dev CLI the exact
  window/anchor split §A1 is about. It picks its wrist from `camera_id`, the field `main()` already
  reads to choose the reference clip; a file recording none keeps the lead wrist.
- **A shared `view → wrist` helper in `analysis/` was considered and declined.** `engine` and
  `pipeline` hold a `storage.manifest.Role`, which `analysis` must not import (ADR-008);
  `align_swings.py` holds only a free-form `camera_id`. One helper cannot key on both, and two is
  the drift a single helper would have existed to prevent.

**The regression is wider than this plan predicted, and it is the same defect.** Measured over all
15 stored down-the-line clips, lead-wrist selection against trail-wrist:

| | before (lead) | after (trail) |
|---|---|---|
| session 10 | `(0, 3619)`, 9.673 s — the whole clip | `(1099, 1315)`, 0.400 s ✔ |
| session 4 | declined | `(1130, 1391)`, 0.484 s ✔ |
| session 3 | `(958, 1183)`, 0.417 s | `(1637, 1763)`, 0.233 s ✘ post-impact |
| session 6 | `(1056, 1209)`, 0.284 s | `(1693, 1792)`, 0.183 s ✘ post-impact |
| `2026-08-07-aaron1/1`, `2026-08-09/2`, `2026-08-10/1` — one clip, three bundles | `(1430, 1646)`, 0.400 s | `(2298, 2442)`, 0.267 s ✘ post-impact |
| `2026-08-10/2` | `(1647, 1872)`, 0.417 s | declined ✘ |
| the other six | — | windows shift a few frames and durations move *toward* the band (session 9: 0.183 → 0.417) |

So **five clips regress, not two**, all of them A2 ("last plausible" picks a descent after impact,
because the DTL phone keeps rolling) — and `2026-08-10/2` is a clean second instance of **A3**: its
true descent measures 27 frames at 59.959 fps = **0.4503 s**, 0.0003 s outside
`_PLAUSIBLE_DOWNSWING_S`, so all seven of its candidates are filtered and the single-candidate
escape cannot fire. P7's two rules cover both shapes; nothing here was tuned to hide either.

---

### [x] P6 — the window always holds a quiet address *(fixes A4)*

**Goal.** A short measured downswing must not squeeze the address out of the window and take
motion-start detection with it.

**Files.** `src/golf_coach/analysis/phases.py`, `tests/analysis/test_select_swing.py`.

**Detail.**
- `window_around` (line 396) currently returns `top − _WINDOW_LEAD·dn`. Extend the lead until either
  `_WINDOW_LEAD·dn` **or** a minimum address allowance is satisfied, whichever is larger. Keep the
  allowance in the clip's own time base (ADR-013) — do not introduce a frame count.
- Keep the `_WINDOW_LEAD` comment block (lines 366–374) intact and **add** why the floor was needed;
  that block records a real measurement and must not be overwritten.

**Comment to write.** The feedback loop, in one sentence: a short measured downswing shrinks the
window, which removes the address, which loses the motion start, which sends the alignment to the
fallback that multiplies that same short downswing by 3.5.

**Reuse.** `window_around` is shared with `scripts/align_swings.py:106`, so both paths get it free.

**Tests.** A clip with an 11-frame downswing and a 38-frame takeaway keeps a detectable quiet
address. Sessions 2, 4, 5 are the shape to build the fixture from.

**Done when.** Face-on motion start is detected on the sessions 2/4/5 geometry.

**As built.** Sessions 2, 4 and 5 detect motion start, their BACKSWING segments measure 1.017 /
1.167 / 1.067 s instead of the 3.5-ratio estimate, and **no clip's top or impact moved by a
frame**. The floor binds on 5 of the 14 face-on clips (2, 4, 5, 7, 9); the other 9 keep `5·dn`
untouched. One deviation and two findings:

- **The allowance is in seconds, not in the clip's own time base — this phase's instruction was
  wrong and the measurement is why.** Across the 14 stored face-on clips the lead a window needs
  before `_motion_start` can find a quiet run is **0.92–1.27 s**: near-constant in real time, but
  **3× to 7×** in downswing-lengths. It tracks the *backswing* (0.83–1.22 s here), and an
  amateur's backswing runs ~0.8–1.2 s whatever their downswing does — so `_WINDOW_LEAD · dn`
  under-reaches precisely on the fast downswings (sessions 4, 5, 2 get 1.00, 0.92, 1.08 s and
  fail; session 11 gets 1.67 s for a requirement of 0.92). The quantity is not expressible in
  downswing-lengths, so `_MIN_ADDRESS_LEAD_S = 1.5` is seconds and `window_around` gained
  `fps: float | None = None` — keyword-only, and `None` keeps today's arithmetic exactly, the
  no-fps path P2 set the precedent for. This is the module's *second* seconds-based rule and it
  carries the same kind of note `_PLAUSIBLE_DOWNSWING_S` already does. `scripts/align_swings.py`
  and `scripts/analyze_bundle.py` both already held an `fps` local and pass it, so the `--window`
  a human copies out of `--list-swings` stays the window `select_swing` picks.

- **Rejected: `_WINDOW_LEAD = 7`.** Measured, and it also fixes 2/4/5 with zero top movement on
  this corpus. It gives the *failing* clips the thinnest margins (1.28–1.52 s against a 1.17–1.27 s
  requirement — the multiplier shrinks the lead where it is needed most) while widening every
  already-working clip to 2.7–2.9 s of lead for nothing, which is that much more room for a
  rehearsal to enter the window on footage not yet on disk. Kept in the constant's comment.

- **§A4's table is circular on its three failing rows, and §A4's session 8 row is really an A3.**
  The "address room" column is `5·dn − takeaway`, but on an undetected clip the takeaway *is* the
  `3.5 × dn` estimate, so the column reduces to `1.5 · dn` — 19, 18, 17 exactly. It measures the
  fallback, not the address. The real quantity is the table in this note. And **`2026-08-23/8`
  face-on declines through `select_swing`**: 2 candidates, 0.467 s and 2.018 s, so the real swing
  is 0.017 s outside `_PLAUSIBLE_DOWNSWING_S` and the single-candidate escape cannot fire. §A4
  lists it as `downswing 28`, motion start `True` — the same 0.467 s descent, measured past
  `select_swing` rather than through it. That makes it a **third instance of A3**, alongside
  `2026-08-10/2` DTL (0.4503 s); P7 rule 1 covers all three. Nothing was tuned to it here.

---

### [x] P7 — `select_matching_swing`: choose DTL against face-on *(fixes A2, A3)*

**Goal.** The DTL clip's swing is chosen to *match* the face-on one, not independently.

**Files.** `src/golf_coach/analysis/phases.py`, `tests/analysis/test_select_swing.py`.

**Reuse.** `candidate_downswings`, `window_around`, and the existing `SwingChoice` NamedTuple
(line 387). Add no parallel machinery — this is a second rule over the same candidates.

**Detail.** New function beside `select_swing`:

```
select_matching_swing(keypoints, *, fps, reference_downswing_s, wrist) -> SwingChoice | None
```

Three rules, in order:
1. Candidates within `_PLAUSIBLE_DOWNSWING_S` **or** within tolerance of `reference_downswing_s`.
   This admits session 5's 0.467 s descent without widening the band globally.
2. Among survivors take the one **nearest the reference duration**, not the last. This is what
   fixes sessions 3 and 6.
3. Return `None` when nothing is within tolerance — never guess (ADR-013).

`SwingChoice.reason` must name the reference duration it matched against, the way `select_swing`'s
reasons name what they judged on.

**Design decision.** Rule 2 replaces "last, not first" *for the matched view only*. `select_swing`'s
own rule is unchanged, and its docstring at lines 419–422 already explains why "last" is wrong on
DTL clips — cite that rather than re-arguing it.

**Docstring note to write.** `select_swing`'s docstring (lines 438–441) currently says a plausible
duration is *not* evidence two clips show the same swing, and that only `align_swings`' tempo
cross-check speaks to that. This phase adds a second thing that does; update that paragraph.

**Tests.** A reference duration picks the nearest rather than the last candidate. A descent just
outside `_PLAUSIBLE_DOWNSWING_S` is admitted when it matches the reference. Nothing within
tolerance declines.

**As built.** `select_matching_swing` fixes **six of the fifteen stored bundles** and changes no
other, simulated over the cached keypoints before the code was written and re-run against the
shipped function afterwards. Nothing calls it yet — that is P8 — so this is the rule's behaviour,
not the pipeline's:

| Bundle | face-on ref | today | with P7 |
|---|---|---|---|
| `2026-08-07-aaron1/1`, `2026-08-09/2`, `2026-08-10/1` — one clip, three bundles | 0.401 s | `2378-2394`, 0.267 s ✘ post-impact | **`1549-1574`**, 0.417 s ✔ |
| `2026-08-10/2` | 0.384 s | declined (7 candidates, 0 plausible) | **`1770-1797`**, 0.450 s ✔ |
| `2026-08-23/3` | 0.384 s | `1707-1721`, 0.233 s ✘ post-impact | **`1080-1108`**, 0.467 s ✔ |
| `2026-08-23/6` | 0.417 s | `1748-1759`, 0.183 s ✘ post-impact | **`1136-1165`**, 0.484 s ✔ |
| `2026-08-23/10` | 0.400 s | `1219-1243` ✔ | unchanged |
| sessions 1, 2, 4, 7, 9, 11 | — | one candidate each | unchanged |

Three of those six are admitted by the *reference* rather than by the band (0.450, 0.467, 0.484 s),
which is §A3 twice over and §A2 three times.

**`_MATCH_TOLERANCE_S = 0.12` was sized by sweep, and the corpus is flat across a wide plateau.**
Below 0.07 s sessions 6 and `2026-08-10/2` fall back to a post-impact descent or a decline; below
0.085 s session 3 does too (it needs 0.084). From 0.085 s to 0.25 s every bundle picks the same
swing. Above 0.25 s session 5 comes in, but only because a tolerance that wide has stopped
asserting anything. 0.12 sits inside the plateau, ~2 frames at 60 fps above the binding case, and
the lower edge is the hard one — miss it and the rule picks the wrong swing confidently rather than
declining. A *relative* tolerance was measured and rejected: the three requirements are 16-22% of
their references so a 25% rule also fits, but what is being absorbed is `_top_and_impact` bracketing
one descent 4-5 frames early or late, which is a frame count and not a proportion of the swing.

Three deviations from this phase's text, and two findings:

- **The single-candidate escape is kept, and this phase's three rules do not mention it.**
  `2026-08-23/4`'s only down-the-line descent measures 0.484 s against a **0.200 s** face-on
  reference: neither branch of rule 1 admits it, so a literal reading of rule 3 declines and that
  bundle loses the window P5 just won it. It is now `_lone_candidate_choice`, extracted from
  `select_swing` and called from both — the escape's argument ("a filter with nothing to choose
  between has no job") does not weaken when a reference is present, it strengthens, because a
  reference can reject a lone descent the band would have kept.

- **Rule 3 is "nothing survived rule 1", not a second gate after rule 2.** Gating the *chosen*
  candidate on the tolerance would make rule 1's band branch dead code — anything the band admitted
  and the tolerance did not would be dropped at the end anyway, which collapses the whole rule to
  "within tolerance of the reference". The union is the point: the band and the other camera are
  two different kinds of evidence and either one is enough to keep a candidate.

- **`reference_downswing_s` is a plain `float`, not `float | None`.** A caller with no reference has
  no question for this rule and should call `select_swing`; a non-positive reference declines the
  same way a missing fps does. `wrist` follows P4's convention (`PoseLandmark = LEAD_WRIST`,
  keyword-only), so the function is view-agnostic — it takes a duration, not a view, and can
  therefore be run in either direction, which P8 needs (see below).

- **This phase's rule 1 says it admits session 5 and it does not.** `2026-08-23/5`'s true
  down-the-line descent is 0.467 s, but its *face-on* view measures that swing's downswing as 11
  frames — 0.183 s — so the two are **0.284 s** apart and only a tolerance wide enough to admit
  almost anything would let it through. It holds three candidates, so the escape cannot fire
  either. It stays declined, exactly as today. The defect there is not selection: sessions 2, 4 and
  5 measure 0.217 / 0.200 / 0.183 s face-on against 0.400 / 0.484 / 0.467 s down-the-line **for the
  same swings**, which is a `_top_and_impact` disagreement of the kind §B1 describes, one view
  bracketing the descent much tighter than the other. P10 should record session 5 as still-open
  rather than fixed, and it is the strongest remaining lead in this milestone.

- **P6's As-built says "P7 rule 1 covers all three" A3 instances; it covers two.** The third is
  `2026-08-23/8`'s **face-on** view (0.467 s, 0.017 s outside the band, 2 candidates), and face-on
  is the reference — P8 as written keeps it on plain `select_swing` and falls back for
  down-the-line when it declines, so nothing matches it against anything. Its down-the-line view
  declines nothing and picks a 0.400 s descent, which is 0.067 s from the face-on descent the band
  threw away: matching *in reverse* would recover it. `select_matching_swing` already accepts a
  duration rather than a view, so this is a decision for P8 — should the pick be mutual (whichever
  view is confident becomes the reference) rather than face-on-first — and not work for P7.

---

### [x] P8 — the pipeline picks face-on first, then matches *(wires P7 in)*

**Goal.** Production uses the cross-view rule.

**Files.** `src/golf_coach/api/pipeline.py`, `tests/api/` bundle tests.

**Detail.**
- `_auto_window` orchestration (lines 522–530): pick face-on with `select_swing` as today, read its
  downswing duration, pass it into `select_matching_swing` for DTL with `TRAIL_WRIST`.
- Face-on keeps the independent rule — it is the view that works.
- Fall back to plain `select_swing` for DTL when face-on itself declined, and put that in `notes`.
  The `notes` list already carries `_auto_window`'s narration.

**Done when.** Sessions 3 and 6 are back to their correct windows and 4, 5, 10 are fixed.

**As built.** Production picks the second view with `select_matching_swing`, and the before/after
was measured by running `api.pipeline._auto_windows` itself over the cached keypoints of all 15
stored bundles. Exactly the six down-the-line windows P7 predicted move, and no seventh:

| bundle | down-the-line before | after |
|---|---|---|
| `2026-08-07-aaron1/1`, `2026-08-09/2`, `2026-08-10/1` — one clip, three bundles | `(2288, 2442)`, 0.267 s ✘ | **`(1424, 1649)`**, 0.417 s ✔ |
| `2026-08-10/2` | declined — whole clip | **`(1635, 1878)`**, 0.450 s ✔ |
| `2026-08-23/3` | `(1617, 1763)`, 0.233 s ✘ | **`(940, 1192)`**, 0.467 s ✔ |
| `2026-08-23/6` | `(1658, 1792)`, 0.183 s ✘ | **`(991, 1252)`**, 0.484 s ✔ |
| the other nine | — | unchanged |

Two decisions and three deviations:

- **The pick is mutual, which was P7's open question, and it is the face-on view it rescues.**
  When face-on declines, the down-the-line view is picked alone and *its* duration becomes the
  reference for a second attempt at face-on. On `2026-08-23/8` that recovers **`(262, 514)`**,
  0.467 s — the descent §A4 lists and the band misses by 0.017 s. The measured consequence is a
  **scoring** one, not an alignment one, and that distinction is the finding: session 8's anchors
  are top 402 / impact 430 either way, because `segment_phases` finds that swing over the whole
  clip too, and its alignment was already `full`. What moves is which frames get *scored* —
  `finish_balance` 0.70 → 1.00 and the swing 86.5 → 91.7, the whole 575-frame clip having carried
  motion long after the finish. The risk this buys (a down-the-line reference that is itself a
  post-impact descent would hand face-on a confidently wrong window rather than a decline) is
  written into `_auto_windows`' docstring rather than guarded, because the reverse only ever runs
  where the alternative is already "every motion in the clip scored as one swing".

- **`scripts/align_swings.py --auto-window` was wired to the same order, which this phase's Files
  list does not name.** P6's As-built protected the property that the window a human copies out of
  the diagnostic CLI is the window the pipeline picks; leaving the script on the independent rule
  would have broken it on those same six bundles. The script already knew which clip is face-on
  (`_selection_wrist`, and `main`'s `b_leads`), so the ordering is that test plus a swap. The rule
  is shared — `phases.select_matching_swing` — and only the narration is written twice, because
  the script prints where the pipeline writes notes.

- **There is no "matched, else plain" fallback for the down-the-line view, which this phase's
  third bullet asks for.** `select_matching_swing` keeps every candidate the band keeps *plus* the
  ones the reference vouches for, and both rules end at `_lone_candidate_choice`, so it declines
  exactly where `select_swing` would on the same clip: the fallback would be unreachable code. The
  `notes` half of that bullet is kept, on the path that does exist — no reference *at all*,
  because face-on declined or was given by hand. `2026-08-23/5` is the only bundle on that path
  and it declines either way.

- **`_auto_window` became four functions, and the reason is `notes` rather than tidiness.**
  `_pick_swing` selects, `_downswing_seconds` converts a choice into the one number the other view
  wants, `_narrate_choice` says what happened, and `_auto_windows` orders the three. Narration had
  to move *after* the last attempt at a view: face-on is picked twice on the reverse path, and the
  decline note from the first attempt would otherwise sit in `analysis.json` saying the whole clip
  was scored, which is exactly the kind of note this module exists to keep true.

- **This phase's "Done when" line is stale and P10 should not inherit it.** Sessions 4 and 10 were
  already correct after P5 and are unchanged here, and **session 5 is not fixed and cannot be by
  selection**: its two views measure the same swing 0.183 s and 0.467 s apart, so no reference and
  no band admits it — the `_top_and_impact` disagreement P7's As-built flags as this milestone's
  strongest remaining lead. The real done-when is the six windows above plus session 8's face-on.

**Tests.** `tests/api/test_pipeline_auto_window.py` gains three pins beside P5's two: a
down-the-line clip whose *last* plausible descent is a post-impact decoy (the six-bundle bug in a
fixture — without matching, selection takes the decoy), face-on rescued in reverse from a descent
just outside the band (without the reverse, it declines), and the no-reference path saying so in
`notes`. Both new behaviours were confirmed to fail on the plain rule before being asserted.

---

### [x] P9 — ADR-015 addendum, `ANALYSIS_VERSION`, docs

**Goal.** Record why fps entered a design that says it does not use fps.

**Files.** `docs/decisions/015-handheld-two-phone-capture-and-event-anchored-alignment.md`,
`src/golf_coach/contracts/swing.py`, `docs/README.md`, `ROADMAP.md`, this file.

**Detail.**
- **Addendum to ADR-015.** Its Consequences say "fps is almost entirely unused, which is the point."
  That is now false in two more places. Write what the evidence showed: the tau axis is the right
  frame, but normalizing by a *wrongly measured* downswing amplifies error rather than absorbing it,
  so the **anchors** need a real-time sanity check even though the **warp** does not. Note that
  `_shared_tops` had already set this precedent unrecorded, and that all footage on disk is
  ~59.96 fps — so this is not a claim that fps is trustworthy in general, and the no-fps paths in
  P2 and P3 stay.
- Bump `ANALYSIS_VERSION` in `contracts/swing.py` — this is what makes stored bundles read as stale
  to `api/state.is_outdated` and `storage/corpus.py`.
- `docs/README.md`: add the M10 row. `ROADMAP.md`: status row and section.

**Tests.** `tests/test_docs_truth.py` is the gate; it will have been failing since P2.

**As built.** ADR-015 gained its first addendum, `ANALYSIS_VERSION` went 10 -> 11, and the map, the
roadmap and this file were brought back into agreement. Three findings, one of them a test change
this phase's Files list does not name:

- **`tests/test_docs_truth.py` had to change, because it was shaping this document rather than
  checking it.** `test_the_map_and_the_m9_doc_agree_on_the_phase_count` gathered *every*
  `N/M phases built` string in `docs/README.md` into one set and asserted it equalled M9's status
  line — so writing M10's real count into its map row failed a green suite. The M10 row had been
  passing only because "**0/10 built**" omits the word "phases". It is now
  `test_the_map_and_each_phase_doc_agree_on_the_phase_count`, parametrized over the docs that
  declare a `**Status: N/M phases**` line and matched against *that doc's own* row, so M11 needs no
  edit here. `M4_FUNDAMENTALS_PANEL.md` has a `**Status:` banner with no phase count and is
  correctly not collected.

- **This phase's Tests line was wrong: the suite was green, not failing since P2.** Nothing
  `test_docs_truth.py` asserts touches the alignment math — it pins checkpoint membership, route
  tables, ADR addendum counts and tier banners. The stale claims P1-P8 left behind (the map's
  "0/10 built — nothing here is as-built yet", ROADMAP's "expect the corpus to look worse until
  P7") were all in prose no test could see, which is the same shape of gap that motivated the file
  in the first place. The new pin closes the phase-count half of it.

- **The bump is the first non-additive one since version 3, and the changelog comment says so.**
  `3 -> 4`, `6 -> 7`, `7 -> 8`, `8 -> 9` and `9 -> 10` each added a measurement and left
  `overall_score` byte-identical, so a stale artifact was *missing* a quantity. This one adds no
  field at all — the same six checkpoints over different frames — so a version-10 artifact
  **disagrees**, which is the class `2` and `3` are in. That distinction is what a future reader of
  `storage/corpus.py`'s mixed-version refusal needs, and it is written into the comment rather than
  left to be inferred from the diff.

Two things named in the Detail bullets were deliberately widened. The addendum names **five**
fps-dependent sites rather than "two more places" — `_shared_tops` and `_PLAUSIBLE_DOWNSWING_S`
both predate M10, the latter unmentioned by anyone until now — and it also amends two *Decision*
sections rather than only the Consequences bullet, because P7/P8 gave "are these the same swing?" a
second cross-check and made the automatic window no longer per-clip independent. Both are stated in
the addendum rather than edited in place, which is this repo's convention.

---

### [x] P10 — re-run the corpus and record before/after

**Goal.** Prove the fix on the footage that exposed the bug.

**Files.** `WORKLOG.md` (new top entry), `docs/M10_ALIGNMENT_ACCURACY.md` (status line, tier
REFERENCE, phase boxes ticked).

**Detail.**

```bash
.venv/Scripts/python.exe scripts/reanalyze.py --all --dry-run   # confirm all 15 targeted
.venv/Scripts/python.exe scripts/reanalyze.py --all --video     # re-analyse and re-render
```

`reanalyze.py` has `_video_went_stale` (line 205) to flag a render disagreeing with its JSON, so a
partial run self-reports. Keypoints are cached against each clip's sha256, so this touches neither
MediaPipe nor OCR.

Fill in the *after* column against this baseline (both columns are the eleven bundles from
2026-08-23, the set the evidence tables were measured on):

| metric | before (2026-08-25) | after (2026-08-26) |
|---|---|---|
| quality tiers | 4 `full`, 3 `top_impact`, 4 `impact_only` | 3 `full`, 3 `top_impact`, 5 `impact_only` |
| DTL windows missing or whole-clip | 3 (sessions 4, 5, 10) | **1** (session 5) |
| face-on windows missing | 1 (session 8) | **0** |
| face-on motion start undetected | 3 (sessions 2, 4, 5) | **0** |
| DTL backswing collapsed to ≤1 frame | 3 (sessions 2, 5, 10) | **2** (sessions 2, 5) |
| worst tau=0 disagreement | 0.300 s (sessions 10, 3) | **0.167 s** (session 10) |
| worst disagreement on a `full` bundle | 0.233 s (session 9) | **0.167 s** (session 10) |

Over all fifteen the tiers read 4/7/4 before and 3/7/5 after; the four older bundles are re-uploads
of two clips and none of them changed tier. The two worst-case rows are the same bundle because
session 10 is now the only `full` one carrying any gap at all, and its 0.1665 s sits just inside
`_BACKSWING_AGREEMENT_S` — no `full` bundle exceeds the bound, where two did.

Then **watch two renders by eye** — session 10 (worst `top_impact`) and session 9 (a `full` bundle
that is nonetheless off) — and confirm both panels leave address together.

**Also record the checkpoint scores.** The upstream changes move which frames get *scored*, not
just rendered — `analyze_swing_bundle`'s docstring is explicit that the window decides this. Scores
on the 15 stored bundles will move, and that is intended (a bundle scored on a practice swing was
scored on the wrong thing), but the before/after score table belongs beside the alignment table so
the change is legible later.

**As built.** Ran 2026-08-26. `reanalyze.py --all --video` re-analysed and re-rendered 15/15 in
~26 minutes and exited 0; a second `--dry-run` reports every stored result current, and
`scripts/career_corpus.py` is back to 13 distinct swings and 21 metrics with no `OUTDATED`
exclusion. Four bundles moved a score, and only four:

| bundle | before | after | what moved |
|---|---|---|---|
| `2026-08-23/2` | 98.70 | 95.68 | `tempo` left `unscored` and fails at 4.92:1; `head_stays_back` 0.94 → 0.85 |
| `2026-08-23/4` | 95.78 | 88.50 | `tempo` left `unscored` and fails at 6.08:1; `hip_shift_at_top` and `hip_sway` now pass |
| `2026-08-23/5` | 87.26 | 83.47 | `tempo` left `unscored` and fails at 6.09:1; `hip_sway` now passes, `head_stays_back` now fails |
| `2026-08-23/8` | 88.78 | **93.07** | it got its first face-on window; `finish_balance` reads 0.13 rather than 0.36 and passes |

The other eleven kept their `overall_score` to the digit. Six findings:

- **The windows landed exactly where §A1 predicted.** Session 4's down-the-line window went from
  *none* to `(1130, 1391)` and session 10's from the whole clip `(0, 3619)` to `(1099, 1315)` — the
  two trail-wrist picks that table printed, frame for frame. Session 8, the only bundle with no
  *face-on* window, got `(262, 514)`, and its `finish_balance` stopped being measured over a clip
  that included the walk-off.

- **The tier count got worse on paper, and that is the cross-check working.** Sessions 9 and 6 are
  the two bundles §B2 caught claiming `full` while 0.233 s and 0.200 s apart. Both now degrade and
  say why in their notes: session 9 to `impact_only` (*"downswing durations disagree — face_on
  0.267s vs down_the_line 0.417s"*), session 6 to `top_impact` (*"down_the_line is finding its
  motion start late … the swing itself is fine"*). Session 10 moved the other way, `top_impact` →
  `full`. Nothing reads `unaligned`.

- **The two nominated renders were watched, and both pass.** Session 10 at tau = +0.40: before, the
  face-on club head had barely left the ball while the down-the-line panel was a third of the way
  into its takeaway; after, both panels have the hands at hip height and the shaft back. Session 9
  at the same point in the swing: before, face-on was waist-high while down-the-line was still at
  address; after, both are chest-high with the shaft up — and the banner now says *aligned on
  impact only* instead of claiming the alignment it did not have.

- **The residual defect is the face-on top, and it is the only one left.** Four bundles (2, 4, 5,
  9) carry a note that the two views disagree about the downswing, and always the same way round:
  face-on measures 0.183–0.267 s where down-the-line measures 0.384–0.484 s of the same swing.
  So the three `tempo` readings that appeared above (4.92, 6.08 and 6.09:1, all past the band's
  4.71 ceiling) rest on a denominator the second view contradicts — **do not read them as coaching
  truth**, and do not read the three score drops as those swings having got worse. This is the
  class `_DRAWDOWN_FLOOR` (`phases.py:132`) was fitted against — its own comment records the
  face-on top landing late — so the floor helped and did not finish the job. Which view is wrong is
  *not* settled here: the notes say "one view's top is wrong" and stop, which is the honest
  reading.

- **Session 5 is the one bundle still windowless down-the-line**, and it is §A3 unrescued rather
  than a new failure. Face-on offers a 0.183 s reference, the clip's only descent measures 0.450 s,
  and no rule will match those two, so the whole clip is scored and the swing notes say exactly
  that. The cause is the short face-on downswing above, not anything down-the-line.

- **One thing that looks like a P10 change and is not.** The renderer's first-choice H.264 encoder
  fails to load on this machine (`openh264-1.8.0-win64.dll`, wrong version) and OpenCV prints a
  `VideoWriter` failure per bundle before falling back. All 15 files are h264 and read back at
  their full frame count; the noise predates this phase.

`2026-08-23/1` was re-run afterwards with `--coaching` — it is the only bundle that had a written
paragraph, and `build_feedback` rebuilds the rules half on every run but leaves `coaching_text` to
the flag, so without that second run it would have kept a paragraph describing the pre-M10 window.
`claude-opus-5` wrote 1266 characters against the corrected one.

## Verification, end to end

Beyond the per-phase suite: after P10, no bundle has a collapsed backswing or a whole-clip window;
no `full` bundle disagrees by more than `_BACKSWING_AGREEMENT`; anything still degraded says why in
its notes. `tests/api/test_pipeline_imports.py` must stay green throughout — `alignment.py` and
`phases.py` remain stdlib + `contracts`.

**Three of those four hold, and the fourth does not.** Measured 2026-08-26 after P10: no `full`
bundle exceeds `_BACKSWING_AGREEMENT_S` (two did), every degraded bundle names its reason in
`notes`, and the import pins are green. But **sessions 2 and 5 still collapse their down-the-line
backswing to a single frame**, and session 5 still has no down-the-line window at all. Both are the
same defect and it is not in the alignment: the face-on top lands late, the face-on downswing
therefore measures 0.183–0.217 s, and no reference that short will match a real down-the-line
descent. This criterion was written believing the windowing was the whole problem; it was most of
it. Closing the last of it is a job for the milestone after this one, and it needs the face-on top,
not another windowing rule.
