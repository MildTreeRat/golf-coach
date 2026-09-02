# M13 — Downswing Tempo: the downswing is what you feel, the backswing is what you change

> **Tier: TARGET.** Nothing here is built. Every number below is either read out of a committed
> artifact (`benchmarks/golfdb_v1.json`) or arithmetic on one, and the arithmetic is shown so it
> can be checked rather than trusted. The *why* is
> [ADR-023](decisions/023-tempo-training-and-absolute-swing-durations.md), whose **third addendum
> this milestone's P0 writes** — read its 2026-08-20 addendum first, because that is the decision
> P2 reverses.

**Status: 0/8 phases built.** P0 writes ADR-023's third addendum and this document; P1 lands the
contract fields; P2 flips the anchor in `analysis/tempo_trainer.py`; P3 widens the pace control and
re-sources the pin that guards it; P4 moves the career scope onto the downswing; P5 rewrites what
the two pages say; P6 re-expresses the tempo verdict in backswing milliseconds; P7 reconciles the
docs and re-runs the corpus. **P1–P4 and P6 are testable with no browser. P5 needs the server.**

## What this milestone is

The tempo trainer builds its target from the golfer's **backswing** and derives the downswing from
it at the tour ratio. ADR-023's 2026-08-20 addendum chose that, and the measurement behind it is
sound: a slower golfer's longer backswing is a real, measurable speed signal, and it is the only
speed signal this repo has (every stored shot reads a smash factor below 1.0, so `club_head_speed`
is refused at `analysis/shot_measure.py`).

It is still the wrong half to hold fixed. The downswing is the half a golfer *feels* — it is how
hard they swung — and the backswing is the half they can deliberately change. Anchoring on the
backswing prescribes a downswing, which asks the golfer to change the thing they experience as
effort. Anchoring on the downswing prescribes a backswing, which asks them to change the thing they
experience as a decision.

So: **read the downswing, multiply by the tour ratio, prescribe the backswing.** The verdict
changes voice to match — from an abstract ratio to a backswing duration in milliseconds.

### The arithmetic, on the golfer on disk

Reference rows from `src/golf_coach/analysis/benchmarks/golfdb_v1.json`, stratum all/all/all,
n=310 clips from 168 golfers, 30 fps non-slow-motion, **down-the-line only**:

| | p10 | p50 | p90 |
|---|---|---|---|
| `backswing_ms` | 700.59 | 900.68 | 1204.54 |
| `downswing_ms` | 200.20 | 266.84 | 300.30 |

Tour ratio = 900.68 / 266.84 = **3.3755**. The golfer's latest stored swing is **901.2 ms back /
383.9 ms down, 2.35:1**.

| | today | after M13 |
|---|---|---|
| anchor | backswing 901.2 ms (inside p10–p90, accepted) | downswing 383.9 ms |
| `pace` | 901.2 / 900.68 = **1.00** | 383.9 / 266.84 = **1.44** |
| `CUES` target | 901 back / 267 down | **1296 back / 384 down** |
| `GRID` target | 901 back / 267 down, 3 ticks | **1152 back / 384 down**, 3 ticks |
| verdict | "Tempo too quick — 2.4:1 … aim for the tour range 2.72–4.71:1" | "Tempo too quick — 2.4:1. Your downswing was 384 ms; at the tour ratio that wants a **1044–1808 ms** backswing and yours was 901." |

The two derived numbers in the last row are `2.72 × 383.9` and `4.71 × 383.9`.

## Four things that will look obvious and are wrong

**1. This does not put a band on the downswing, and must not.** ADR-023 §1 — *"No band, no
checkpoint, no placement"* — **stands**. The `tempo` checkpoint keeps scoring `tempo_ratio`
against the unchanged `[2.72, 4.71]` row in `ranges.json`. P6 changes the *sentence* and nothing
else, and the arithmetic is identical: `901 ∈ [2.72 × 384, 4.71 × 384]` if and only if
`ratio ∈ [2.72, 4.71]`. A reader who takes "base tempo on the downswing" as an instruction to band
`downswing_ms` will grow the panel from six checkpoints to seven, count one fundamental twice into
`overall_score`, and make the *score* frame-rate dependent — see §Risks 1.

**2. The anchor guard is deleted, not flipped.** `_anchor_backswing` refuses an observed backswing
outside the tour p10–p90, and its reason is written down: outside that range the backswing is
plausibly the fault itself, so anchoring would build the drill around the half that needs changing.
Under this milestone the downswing is never the fault — it is the given — so the guard has nothing
left to guard. A reader who "keeps the rule and swaps the metric" will get a guard that refuses
383.9 ms (outside 200.20–300.30) and hands the golfer the tour median, which is exactly the state
this milestone exists to leave. **The flip would then change nothing observable on either swing on
disk.**

**3. `_grid_pattern`'s rounding argument inverts while its code does not.** It already absorbs the
integer tick rounding in the backswing and keeps the downswing exact. Today's reason is *"the
downswing is the near-invariant half and the quantity being corrected"*. After M13 the reason is
*"the downswing is the anchor and must land on the golfer's own, and the backswing is the
prescription"*. Same behaviour, opposite argument. P2 rewrites the docstring at
`analysis/tempo_trainer.py:194`; leaving the old argument standing under new behaviour is how a
later reader "fixes" it back.

**4. `ANALYSIS_VERSION` does not move.** No judged number changes. `overall_score` is
byte-identical, `observed` / `expected_low` / `expected_high` on `CheckpointScore` stay in ratio
units, and the only stored field that differs is `message`. Stored swings pick the new sentence up
with `scripts/reanalyze.py --all` (the flag exists, `scripts/reanalyze.py:256`). Say "no
`ANALYSIS_VERSION` bump" explicitly in the WORKLOG entry — the ledger convention in
`contracts/swing.py` expects the clause either way.

## Reading order for a fresh session

1. This file's §Design, then your phase.
2. [ADR-023](decisions/023-tempo-training-and-absolute-swing-durations.md) — **the 2026-08-20
   addendum first** (it is the decision being reversed), then §1 (the rule being kept), then the
   2026-08-22 addendum (the career scope P4 touches).
3. `src/golf_coach/analysis/tempo_trainer.py` — 393 lines, read whole. It is the milestone.
4. `src/golf_coach/contracts/tempo.py` — the vocabulary P1 extends.
5. Only if your phase touches it: `src/golf_coach/api/static/tempo.js` (P3, P5),
   `analysis/checkpoints/mechanics.py::evaluate_tempo` (P6).

## Design

### 1. The anchor moves, and `pace` is still the only place a fit is applied

`build_tempo_plan_for` (`tempo_trainer.py:98`) keeps its shape exactly. The patterns stay at the
tour reference, un-prescaled, and the renderer multiplies by `pace` — the property ADR-023's
addendum argued for and the reason the golfer's pace slider means something. One divisor changes:

```
pace = anchor_downswing_ms / downswing.p50      # was anchor_backswing_ms / backswing.p50
```

That is enough, and the reason it is enough is worth stating. `CUES` carries `backswing.p50` and
`downswing.p50`; at this pace its downswing is `downswing.p50 × anchor_down / downswing.p50` =
**exactly the golfer's own downswing**, and its backswing is `backswing.p50 × anchor_down /
downswing.p50` = **`anchor_down × tour_ratio`**. `GRID`'s tick is `downswing.p50`, so at the same
pace its tick lands exactly on the golfer's downswing too, and its backswing is three of them.
Nothing else in the file needs to know the anchor changed.

### 2. No guard, a notice, and an opt-in snap

`_anchor_backswing` (line 143) becomes `_anchor_downswing` and drops the p10–p90 rejection: the
observed downswing when there is one, `downswing.p50` when there is not. In its place the plan
*reports*:

- `downswing_in_tour_range: bool` — whether the observed downswing sits in the reference p10–p90.
- `in_range_pace: float | None` — the pace that clamps the downswing to the nearest tour edge,
  `None` when it is already inside. For 383.9 ms that is `300.30 / 266.84` = **1.13**, a 1014 ms
  backswing.

`None`-when-nothing-to-offer is deliberate: "is there something to offer" and "should the button
render" become one question, the same shape `_claimed` (line 357) already uses for the withheld
baseline claims.

This is what replaces the guard's protection rather than discarding it. ADR-023's *"a wrong verdict
is read once, and a wrong metronome is rehearsed"* still holds, and the answer is now the golfer's:
a mis-segmented 60 ms downswing is outside p10–p90, so the page says so and offers the snap,
instead of the server silently substituting a median and claiming a fit.

### 3. What the plan carries

`TempoPlan` (`contracts/tempo.py:142`) gains three fields and re-describes two:

| field | |
|---|---|
| `anchor_downswing_ms` | **new**, `gt=0`. The downswing every pattern was built from. |
| `anchor_backswing_ms` | **kept**, now derived: `anchor_downswing_ms × tour_ratio`. It is the prescription the page prints, so it stays first-class rather than being left for a reader to multiply. |
| `downswing_in_tour_range` | **new**, `bool`. |
| `in_range_pace` | **new**, `float \| None`. |
| `pace` | description rewritten — it is a multiple of the tour median **downswing** now. |
| `anchored` | description rewritten — with no guard it means only "a downswing was measured". |

`TempoAnchor`'s three members keep their names and meanings — they say *which* measurement the
target was fitted to, not which half — but every docstring naming a backswing becomes a downswing,
and `TOUR_MEDIAN` loses its "or the guard rejected it" clause.

### 4. The pace control has to reach the new range

`static/tempo.js:80` is `min="70" max="140"`, and those numbers are the old backswing guard
expressed against the median (`700.59/900.68` and `1204.54/900.68`). The fitted pace for the swing
on disk is now **144%** — off the end, which `wire()` would silently clamp (line 103), playing a
tempo other than the one the page's own text claims. That is the exact failure
`test_every_fitted_pace_is_reachable_on_the_pages_slider`
(`tests/analysis/test_tempo_trainer.py:261`) exists to catch, and it will catch it.

New bounds come from the widest downswing the segmenter is designed to admit —
`_POSSIBLE_DOWNSWING_S = (0.12, 0.80)` at `analysis/phases.py:391`, already provenanced there as an
absolute bound because *"a downswing lasts about the same time for every golfer at every frame
rate"*. Against `downswing.p50`: `120/266.84 = 0.45` and `800/266.84 = 3.00`, so `min="44"
max="300"`.

Promote the constant to a public `POSSIBLE_DOWNSWING_S` rather than copying it. A second copy of a
bound is a second thing that drifts, and `tempo_trainer.py` already imports sideways within
`analysis/` (`from golf_coach.analysis.measure import tempo_timings`, line 30), so this crosses no
boundary ADR-008 draws.

One honesty note for the test's docstring: `phases.py:733` admits a descent outside that window
when the *other view* vouches for it, and `tempo_timings` measures top-to-impact where the top is
the centre of the `TRANSITION` window rather than the descent start. So this is the designed range,
not a hard guarantee, and `wire()`'s clamp stays as the backstop.

### 5. The verdict re-expressed, and nothing else in `mechanics.py`

`evaluate_tempo` (`analysis/checkpoints/mechanics.py:227`) keeps `measure_tempo_ratio`, keeps
`resolve_range("tempo_ratio", …)`, keeps `_score_within_range`, keeps `passed`, keeps
`_population_placement`, and keeps `CheckpointScore.observed` / `expected_low` / `expected_high` in
**ratio units**. Stored artifacts stay comparable and no downstream reader re-points.

Only the three messages (lines 253–264) change, gaining the backswing target. The observed
downswing comes from `measure.tempo_timings(phases)` — the same call `measure_tempo_ratio` already
makes, so the two cannot disagree about whether the swing was timeable. When the durations are
absent the message falls back to today's ratio-only wording.

### 6. The career scope follows

`build_career_tempo` (`tempo_trainer.py:265`) prefers `typical_downswing_ms` → `latest.downswing_ms`
→ neither. `_RATIO_METRIC` still defines which swings have a readable tempo, and `_tempo_swings`,
`_claimed` and `_center_refusals` are untouched.

**The career anchor unlocks earlier than it used to**, and that is a consequence rather than a
goal: `contracts/baseline.py`'s `METRIC_MINIMUM_N` overrides `tempo_ratio` to `CENTER: 8` and says
why (it is the noisiest metric in the panel), but `downswing_ms` carries no override and gates at
the default 5. Record it in the addendum; do not "fix" it by adding an override nobody measured.

Keep the read-back at line 314 (`if plan is not None and not plan.anchored`) even though it can no
longer fire. It is the discipline that stops a second copy of the anchor rule appearing one layer
up, which is the failure ADR-023's 2026-08-22 addendum names explicitly.

## Phases

Each phase is independently commit-ready: `pytest`, `ruff` and `mypy` clean on its own. `tests/`
mirrors `src/golf_coach/` package by package.

### [ ] P0 — write the milestone into the repo's own docs

**Goal.** Land the *why* where a reader of ADR-023's 2026-08-20 addendum will see that it was
reversed, and the *how* so a fresh session can execute a phase number without rediscovering the
repo.

**Files.** `docs/decisions/023-tempo-training-and-absolute-swing-durations.md` (a third addendum);
this file (new); `docs/README.md` (an M13 row, ADR-023's per-row addenda count to **3**, the
document counts and the addenda total); `ROADMAP.md` (a row in *Status at a glance* and an `## M13`
section).

**Tests.** None new. `tests/test_docs_truth.py` derives its counts from `docs/README.md`'s own
prose, so the README numbers *are* the pin and it fails until they are bumped. The new file must be
`git add`ed before the count test agrees — it counts through `git ls-files`.

**Detail.** The addendum records: why the backswing anchor read backwards to a golfer; that §1 is
**not** reversed and no band is added; that the guard is deleted and replaced by a notice plus an
opt-in snap; that `_grid_pattern`'s rounding argument inverts while its code does not; the
frame-rate caveat in §Risks 1; and the earlier career unlock from §Design 6.

### [ ] P1 — `contracts/tempo.py`: the vocabulary, no behaviour

**Goal.** The shape the next three phases fill in, landed on its own so a contract review is a
contract review.

**Files.** `src/golf_coach/contracts/tempo.py`.
**Tests.** `tests/contracts/test_tempo.py`.

**Detail.** §Design 3's table. `anchor_downswing_ms` and `in_range_pace` both `gt=0`;
`in_range_pace` optional and defaulting to `None`. Every field description carries the *why* the
existing ones do — in particular, `anchor_backswing_ms`'s must now say it is derived and why it is
still stored rather than left for the page to compute.

**Pin in tests**: a `TempoPlan` with `downswing_in_tour_range=True` and a non-`None`
`in_range_pace` is a contradiction the model should not be able to express — decide in this phase
whether that is a `model_validator` or a documented convention, and pin whichever.

### [ ] P2 — `analysis/tempo_trainer.py`: flip the anchor

**Goal.** The milestone, in one divisor and one deleted guard.

**Files.** `src/golf_coach/analysis/tempo_trainer.py`.
**Tests.** `tests/analysis/test_tempo_trainer.py`.

**Detail.** §Design 1 and 2. `_anchor_backswing` → `_anchor_downswing`, no rejection; `pace =
anchor_downswing / downswing.p50`; populate the three new fields; `anchor_backswing_ms =
anchor_downswing × tour_ratio`. `_source_prose` (line 231) says "set to your own downswing, at the
tour ratio" / "set to the tour median downswing". `build_tempo_plan` (line 57) and its keyword-only
call are unchanged. Rewrite `_grid_pattern`'s docstring per §Obvious-and-wrong 3, and
`_anchor_downswing`'s must carry why the guard went or the next reader re-adds it.

**Tests that change, not just get added.** `test_an_anchor_the_guard_rejects_is_reported_as_the_tour_median`
(line 434) loses its premise — **replace** it with one asserting an out-of-range downswing still
anchors, sets `downswing_in_tour_range is False`, and yields a non-`None` `in_range_pace`. Update
`test_the_unanchored_plan_is_exactly_what_shipped_before_anchoring` (line 288): the unanchored case
is now only "no downswing measured", and the collapse-to-tour-median pin still holds there and
should stay.

### [ ] P3 — the pace control reaches the new range

**Goal.** A fitted 144% opens at 144%.

**Files.** `src/golf_coach/analysis/phases.py` (promote `_POSSIBLE_DOWNSWING_S`);
`src/golf_coach/api/static/tempo.js` (bounds only).
**Tests.** `tests/analysis/test_tempo_trainer.py:261`, rewritten.

**Detail.** §Design 4. Keep the regex-out-of-`tempo.js` mechanism — it is the thing that makes a
widened fit fail here rather than ship — and re-source its reachable range from
`POSSIBLE_DOWNSWING_S` against `downswing.p50`. Carry §Design 4's honesty note into the docstring.

### [ ] P4 — the career scope follows the downswing

**Goal.** "What is my tempo, and what should I practise to" answers with the same half the swing
page does.

**Files.** `src/golf_coach/analysis/tempo_trainer.py` (`build_career_tempo` only).
**Tests.** `tests/analysis/test_tempo_trainer.py`, the career block from ~line 400.

**Detail.** §Design 6.

### [ ] P5 — the two pages say the new thing

**Goal.** The notice, the snap, and three anchor sentences that name the right half.

**Files.** `src/golf_coach/api/static/tempo.js` (the notice and a **"snap to tour range"** button
beside the pace slider, rendered only when `plan.in_range_pace` is present, setting the slider to
it); `src/golf_coach/api/static/results.html:378-385` (`anchorText`);
`src/golf_coach/api/static/career.html:471-478` (`ANCHOR_TEXT`'s three strings, `tour_median`
losing its "outside the tour range" clause — that is the notice's job now) and the `halvesText`
comment at line ~449.
**Tests.** `tests/api/test_results.py` (including
`test_the_results_page_writes_no_tempo_number_of_its_own`, line 397) and
`tests/api/test_career_route.py`.

**Detail.** `tempo.js`'s standing rule holds: no duration, ratio or tick count is written in that
file — everything arrives on the plan. The snap button sets the slider and lets the existing
`input` handler redraw; it must not compute a pace of its own.

**Out of scope, deliberately.** `results.html:338-357` gates the trainer block on the tempo
checkpoint having *failed*, even though the API always sends the plan. That is a real question and
it is not this milestone's.

### [ ] P6 — the verdict re-expressed in backswing milliseconds

**Goal.** The one checkpoint that ships a verdict naming the half a golfer can change.

**Files.** `src/golf_coach/analysis/checkpoints/mechanics.py`, `evaluate_tempo` messages only.
**Tests.** `tests/analysis/test_checkpoints.py:42,51,59` — the pass / too-quick / too-slow trio.

**Detail.** §Design 5. The three sentences read like:

```
"Tempo too quick - 2.4:1. Your downswing was 384 ms; at the tour ratio that wants a
 1044-1808 ms backswing and yours was 901. Take it back longer."
```

Both edges are `band.low × observed_downswing_ms` and `band.high × observed_downswing_ms`.

### [ ] P7 — docs, and the corpus onto the new sentence

**Goal.** Nothing left claiming the target follows the backswing.

**Files.** `docs/ARCHITECTURE.md` (§3's description of the trainer); `WORKLOG.md` (top entry,
stating **"no `ANALYSIS_VERSION` bump"** and why); `ROADMAP.md`; `docs/README.md` (M13 row to
`8/8 phases built` and its tier to REFERENCE); this file's status line and phase checkboxes.

**Detail.** Then `.venv/Scripts/python.exe scripts/reanalyze.py --dry-run`, and
`--all` to put the two stored swings onto P6's sentence. `/doc-check` afterwards — it is manual,
and this milestone touched ADR-023, `docs/README.md` and `ARCHITECTURE.md`.

## Verification

```bash
.venv/Scripts/python.exe -m pytest
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
```

End to end, on the golfer on disk:

1. `.venv/Scripts/python.exe scripts/analyze_bundle.py 2026-08-10/2 --no-video` — the tempo message
   names a backswing target in milliseconds and still reads "too quick", and `overall_score` is
   unchanged from the stored value.
2. `.venv/Scripts/python.exe run_server.py`, then `GET /api/golfers/aaron/career`:
   `tempo.plan.pace ≈ 1.44`, `anchor_downswing_ms ≈ 383.9`, `anchor_backswing_ms ≈ 1296`,
   `downswing_in_tour_range == false`, `in_range_pace ≈ 1.13`.
3. Load `/career.html#tempo` and the results page for that swing. The slider **opens at 144%** and
   does not clamp to 140; the beat strip draws a visibly longer backswing than before; the anchor
   sentence names the downswing; the notice says the downswing is outside the tour range; the snap
   button moves the slider to 113% and redraws. Play it — the impact tick lands on the golfer's own
   384 ms interval.
4. `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py` last, as the doc pin.

## Risks

**1. The re-expressed message is frame-rate dependent where the score is not.** `tempo_ratio`
cancels fps; an absolute duration does not, and `phases` carries milliseconds derived from the
clip's own frame rate. A slow-motion clip would print an inflated downswing and an inflated
backswing target. The notice from §Design 2 is what catches it — a 3000 ms downswing reads "outside
the tour range" — but it is a notice, not a gate, and the score stays correct either way. Stated in
the addendum rather than fixed here; the fix is a capture-side fps sanity check and it is a
different milestone.

**2. The reference downswing sample is one view.** All 310 clips behind `downswing_ms` are
down-the-line, because the face-on half of the pose cache predates the `clip` envelope and has no
recoverable frame rate. ADR-023 §1 argues this does not compromise the numbers — a duration comes
from event labels and the frame rate and reads no landmark — but the anchor now leans on those
p10/p90 edges harder than the old design leaned on the backswing's, since they drive the notice and
the snap. `n=310` on a 33 ms quantum is what it is; re-extracting the face-on cache is ADR-023's
own third *Deferred* item.

**3. Someone bands the downswing anyway.** §Obvious-and-wrong 1. The tell is any diff touching
`ranges.json`, `CHECKPOINT_REGISTRY`, `POSE_MEASUREMENTS`, `contracts/caveats.py`,
`contracts/dispersion.py`'s `METRIC_TARGETS`, `contracts/comparison.py`'s
`TOUR_COMPARISON_BLOCKED`, `golfdb_v1.json`, `joint_model_v1.json`, `ANALYSIS_VERSION` or
`metric_definitions_version`. **This milestone touches none of them.** A phase that finds itself
editing one has grown a band and should stop for a decision.
