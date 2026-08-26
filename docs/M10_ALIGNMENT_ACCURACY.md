# M10 — Alignment Accuracy: the two panels leave address together

> **Tier: TARGET.** This is a plan, so verify every number against code before trusting it. The
> evidence tables below are a snapshot of the 15 bundles on disk as of 2026-08-25 and are the
> baseline the fix must beat — they are measurements of a bug, not facts about the design. The
> *why* behind the alignment itself is [ADR-015](decisions/015-handheld-two-phone-capture-and-event-anchored-alignment.md);
> this document is the repair, as a phase list.

**Status: 5/10 phases built** — the alignment-math track (P1-P3) is complete; the selection
track has P4 and P5 in and resumes at P6.

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

### [ ] P6 — the window always holds a quiet address *(fixes A4)*

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

---

### [ ] P7 — `select_matching_swing`: choose DTL against face-on *(fixes A2, A3)*

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

---

### [ ] P8 — the pipeline picks face-on first, then matches *(wires P7 in)*

**Goal.** Production uses the cross-view rule.

**Files.** `src/golf_coach/api/pipeline.py`, `tests/api/` bundle tests.

**Detail.**
- `_auto_window` orchestration (lines 522–530): pick face-on with `select_swing` as today, read its
  downswing duration, pass it into `select_matching_swing` for DTL with `TRAIL_WRIST`.
- Face-on keeps the independent rule — it is the view that works.
- Fall back to plain `select_swing` for DTL when face-on itself declined, and put that in `notes`.
  The `notes` list already carries `_auto_window`'s narration.

**Done when.** Sessions 3 and 6 are back to their correct windows and 4, 5, 10 are fixed.

---

### [ ] P9 — ADR-015 addendum, `ANALYSIS_VERSION`, docs

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

---

### [ ] P10 — re-run the corpus and record before/after

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

Fill in the *after* column against this baseline:

| metric | before | after |
|---|---|---|
| quality tiers | 4 `full`, 3 `top_impact`, 4 `impact_only` | |
| DTL windows missing or whole-clip | 3 (sessions 4, 5, 10) | |
| face-on motion start undetected | 3 (sessions 2, 4, 5) | |
| DTL backswing collapsed to ≤1 frame | 3 (sessions 2, 5, 10) | |
| worst tau=0 disagreement | 0.300 s (sessions 10, 3) | |
| worst disagreement on a `full` bundle | 0.233 s (session 9) | |

Then **watch two renders by eye** — session 10 (worst `top_impact`) and session 9 (a `full` bundle
that is nonetheless off) — and confirm both panels leave address together.

**Also record the checkpoint scores.** The upstream changes move which frames get *scored*, not
just rendered — `analyze_swing_bundle`'s docstring is explicit that the window decides this. Scores
on the 15 stored bundles will move, and that is intended (a bundle scored on a practice swing was
scored on the wrong thing), but the before/after score table belongs beside the alignment table so
the change is legible later.

## Verification, end to end

Beyond the per-phase suite: after P10, no bundle has a collapsed backswing or a whole-clip window;
no `full` bundle disagrees by more than `_BACKSWING_AGREEMENT`; anything still degraded says why in
its notes. `tests/api/test_pipeline_imports.py` must stay green throughout — `alignment.py` and
`phases.py` remain stdlib + `contracts`.
