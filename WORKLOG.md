# Work Log

Write a short entry every time you sit down to work. Reverse chronological (newest first).
This is your "pick up where I left off" document.

---

## 2026-08-13 — The band that did not survive our camera (M6.5, closed)

**Duration**: ~1 session, a new harness + implementation + tests + a re-analysis of everything on disk
**What prompted it**: M6.5's last open item, `head_hip_offset_impact_norm`. The roadmap framed it as
an architecture problem — `analysis` is pure and cannot read `Golfer.handedness` — and that framing
was right about the blocker and wrong about the risk.

**What landed**: the handedness seam, a new transfer-check harness, a rejected metric, a promoted
one, and the panel at **six checkpoints**. `ANALYSIS_VERSION` 2 → 3. **552 tests** (up from 547),
ruff and mypy clean.

**The metric M6.5 named was the wrong one, and the check that says so did not exist.** The repo had
`tune_spatial_metric.py`, which asks *is this metric signal or jitter* — entirely inside the
reference corpus. Nothing asked the question that comes after it: **does the population the band was
cut from project the way ours does?** So `scripts/golfdb/check_metric_transfer.py` measures the same
quantity **at address**, where the body is square, the hips have not rotated, and there is no swing
yet to disagree about. The result:

| | at address | at impact | impact − address |
|---|---|---|---|
| GolfDB (broadcast) | −0.198 | −0.617 | −0.404 |
| our bay clips | **+0.124** | −0.032 | −0.156 |
| gap | **+0.322** | +0.585 | +0.249 |

**55% of the disagreement is present before the golfer moves**, at ~4x the metric's own 0.082 error.
Promoting the absolute offset would have scored 0.39–0.50 on all four stored swings, made "you are
not staying behind the ball" the top-ranked tip on every one of them, and dropped every score from
~96 to ~87 — roughly half of it camera.

**Why this metric and not the other five.** Every shipped checkpoint differences *one landmark
across time* — sway and shift are travel, finish balance is drift from its own mean, tempo is a
ratio of durations — so a fixed camera bias is common-mode and cancels in the subtraction. The
absolute head-hip offset compares *two different body parts at one instant*. Shoulder-width
normalization removes the `1/Z` scale, so distance from the camera is handled; it does not remove
**yaw**, and at impact the hips have rotated open while the head has not, so the two sit at
genuinely different depths exactly where the metric reads. That distinction is now written into
`mechanics.py`'s module docstring, because it generalises past this metric: *a band cut from
broadcast footage transfers to a phone only for quantities where the camera cancels.*

**So the delta was promoted instead, and it costs almost nothing.** `head_hip_gain_norm` is impact
minus address under one shared address-window ruler — the same coaching concept, in the regime where
the bias cancels. Ratio **7.1** against the absolute's 7.6. Band two-sided `[-0.67, -0.14]`, both
edges 3.4x the 0.080 error. On the real swings it reads −0.16 to −0.17: **inside** the tour range,
near the shallow edge at the 87th percentile — the opposite verdict from the one the absolute would
have delivered, and the impact frame agrees with it.

**Three things found by running it rather than reasoning about it:**

1. **One golfer, two handednesses.** Classifying handedness independently per signed metric gave
   `TOBY KEITH` opposite labels, from medians of −0.110 and +0.015 that are both inside measurement
   error. A person does not swing right-handed under one measurement and left-handed under another,
   so handedness is resolved **once** per subject, from the metric whose population sits furthest
   from zero. The classifier validates itself by name: four of 122 subjects fold, and two are Phil
   Mickelson and Bubba Watson.
2. **A stored `sd` that was half artifact.** `head_hip_offset_impact_norm` carried sd 0.504; two
   clips of one Stacy Lewis driver swing read **5.44** and **6.13** shoulder-widths — a collapsed
   `shoulder_width` denominator, not a body. Dropping them gives 0.249. Quantiles were robust so no
   shipped band ever moved, which is precisely why nothing caught it.
3. **The unscored tip told the golfer to re-film a clip that was fine.** With no golfer attributed,
   `head_stays_back` drops out and `feedback` emitted its standard "try a clip with the whole swing
   in frame and the camera steady" — confident, and wrong: the swing measured perfectly, the *form
   field* was empty. The remedy is now chosen from what is actually missing, keyed on whether the
   measurement came through.

**The seam itself is small, which is the point.** `analyze_swing(..., handedness=)`, resolved from
the manifest's `player_id` by `api/pipeline.py`. `analysis` imports `Handedness` from `contracts`
and no registry; `None` costs that one checkpoint and names it in `unscored`, because guessing
right-handed would read a left-handed golfer's ordinary impact position as a gross fault — silently,
and in the direction nothing downstream can detect.

**Verified**: 552 tests, ruff and mypy clean. `derive_reference.py` re-run touched **only** the two
signed metrics' rows — 7 added, 7 changed, every other row byte-identical, which is the regression
property that made the change auditable. All four swings re-analyzed `version 2 -> 3`; sidecars
agree with `analysis.json`; the MCP `get_swing` view carries the sixth checkpoint; career mode picked
up the ninth metric with no career-mode change at all and still refuses everything at n = 2.

**Where I left off**: M6.5 is closed. The rejected metric is still measured on every swing, so a
later camera-geometry fix can revisit it without re-capturing anything.
**Blockers**: none. The roadmap's standing NEXT ACTION is still an Anthropic API key — M6's live
path has never made a real request.

---

## 2026-08-12 — The panel widens to five, and the default band shape was wrong for both (M6.5)

**Duration**: ~1 session, implementation + tests + a re-analysis of everything on disk
**What prompted it**: M6.5's last open item — "decide what to promote". The measuring/judging split
shipped five metrics recorded and none scored, and the panel had sat at three checkpoints for four
milestones. With the bay session a while off, this is the largest user-visible change available
that needs no new data at all.

**What landed**: `evaluate_hip_sway` and `evaluate_hip_shift_at_top` in
`analysis/checkpoints/mechanics.py`, two rows in `ranges.json`, engine wiring, an ADR-010 addendum,
and `ANALYSIS_VERSION` 1 → 2. **547 tests** (up from 540), ruff and mypy clean. The mechanics panel
is now **five checkpoints**.

**The roadmap's own note about what blocked this was aimed at the wrong thing.** It said promotion
"wants more than one golfer's swings behind it". It does not: the bands are cut from 458 face-on
GolfDB swings by 122 tour golfers, not from ours, and they were already derived and committed by
M6.5. What actually had to be decided was **band shape**, and that is where the work was.

**`derive_reference.py`'s default would have shipped a wrong band for both metrics.** It recommends
a one-sided `[0, p90]` band for anything named `_norm`, which encodes *less is better*. That is
established for head sway and finish drift. It is **not** established for hip travel — some of it is
the weight shift a swing needs — and this repo had already written that down, in career mode step 5,
as the reason both metrics were denied a bias target. The tour distribution says the same thing out
loud: `hip_sway_norm`'s p10 is **0.14**, so 90% of tour swings move the hips *further* than that. A
`[0, p90]` band would have scored a golfer who barely moves their lower body as perfect.

**Then the two metrics needed opposite treatment, which is the part I did not expect.** Having
rejected the default for both, the obvious move is to make both two-sided. The measurement error
says no:

| metric | p10 | p90 | noise + boundary | p10 vs error |
|---|---|---|---|---|
| `hip_sway_norm` | 0.138 | 0.499 | 0.050 | **2.8x above** |
| `hip_shift_at_top_norm` | 0.015 | 0.207 | 0.053 | **0.3x — below it** |

So `hip_sway_norm` is two-sided `[0.14, 0.50]`, and `hip_shift_at_top_norm` is one-sided
`[0, 0.21]` — the same shape as head sway, reached by the opposite argument. Not because less is
better, but because a lower edge at 0.015 would separate golfers this pipeline cannot tell apart.
The rule that falls out is worth more than either row: **assert a band edge only where it clears the
instrument.** `tune_spatial_metric.py` already computed the numbers for step 5's tolerances; this is
the second thing they have been used for, and neither use needed new extraction.

**Two things found by running it rather than reasoning about it:**

1. **The default synthetic swing now fails a checkpoint, and it should.** `make_swing`'s body holds
   its hips perfectly still, which no golfer does, so it fails the two-sided `hip_sway` while
   passing all four others. `test_a_fully_measured_swing_leaves_nothing_unscored` asserted
   `== 3` and was the only test in the suite that broke. Rather than just bumping the number, it now
   pins the pass/fail split too — a later fixture default that adds a hip shift has to come past
   that line instead of quietly flipping a checkpoint back to green.
2. **Adding checkpoints raised every stored score, which is why the version had to move.**
   `overall_score` is a mean over survivors, so two new passing checkpoints dilute the one failing
   tempo: 94.92 → **96.95** on three swings and 93.77 → **96.26** on the fourth. Nothing about those
   swings changed. This is the coupling the 2026-08-01 ADR-010 addendum flagged for band *width*,
   showing up for panel *membership* — and it is the case `ANALYSIS_VERSION` exists for.
   `reanalyze.py` found all four unprompted and `record_state` kept the sidecars in step, so career
   step 3's two mechanisms both paid off without being touched.

**`one_sided` stopped being an internal ranking detail.** With `tempo_ratio` and `hip_sway_norm` both
two-sided, "a low percentile is good news" is now wrong on two of five checkpoints.
`contracts/caveats.py` says so explicitly, because a model handed a low number will otherwise
congratulate the golfer for it — the same class of defect as career step 6's amber "outside tour
range" pill, caught this time before it shipped rather than by a render.

**Verified**: 547 tests, ruff and mypy clean. Against the real disk, all four swings re-analyzed
`version 1 -> 2`, and on `2026-08-07-aaron1/1` the new checkpoints read `hip_sway` 0.27 (percentile
41.5) and `hip_shift_at_top` 0.08 (percentile 48.6) — both near the tour median, which is the
sanity check that they are not manufacturing faults on a real amateur swing. Feedback still leads
with tempo, the only failure. `analysis.state.json` agrees with `analysis.json` on all four, and the
three career CLIs still report `n = 2` for all eight metrics, unchanged: promotion judges
measurements, it does not alter them.

**Where I left off**: `head_hip_offset_impact_norm` is the one candidate left, and it is an
architecture change rather than a data edit — the band exists and the ratio is 7.6, but its sign is
camera-relative, and `analysis` is pure and cannot read `Golfer.handedness` without a seam that does
not exist. That is the next decision, not the next commit.
**Blockers**: none. Nothing here needed the bay session.

---

## 2026-08-12 — A band is not a target, and "outside" is not a verdict (career mode, step 6)

**Duration**: ~1 session, implementation + tests
**What prompted it**: the last step of the milestone. Steps 1–5 built the whole mechanism and left
it unreachable from anything but three dev CLIs. Step 6 is the surfacing — the two MCP tools ADR-006
deferred, a page, and the personal-vs-tour join that turns "a consistent 0.31 of head sway" into
"and that sits inside tour range".

**What landed**: `contracts/comparison.py` + `analysis/comparison.py` (the join),
`storage.corpus.narrow_to`, `mcp/career.py` with `get_golfer_profile` / `get_shot_trends` /
`compare_sessions`, `GET /api/golfers/{id}/career`, `static/career.html`, an "Against your own
history" block on the swing page, `READING_A_PERSONAL_HISTORY` in `contracts/caveats.py`, and a
`vs tour` row in `career_baseline.py`. 540 tests (up from 501), ruff and mypy clean.

**The join is decided by the interval, like everything else in this milestone.** `Standing` is read
off the mean's 95% CI, never the mean: whole CI inside p10–p90 is `inside`, whole CI past an edge is
`outside`, anything crossing is `straddles` — which is "unresolved", not "borderline". A center a
hair past p90 with an interval over it has not been shown to be outside anything, and reporting so
would be a placement that flips on the next swing.

**The four target-less metrics never needed targets.** Going in, the plan was that `tempo_ratio`
could acquire a bias finding via "a one-line edit to `METRIC_TARGETS`". It could, and it would have
been wrong: the recorded reason for its missing target is that *its target is a band*, and asking
whether the center's CI sits inside p10–p90 is that band's own question asked directly. A point
target would have meant declaring the midpoint of 2.72–4.71 to be what good is — a claim with
nothing behind it. `METRIC_TARGETS` is untouched and the join answers what the targets were wanted
for.

**Five things found by building it:**

1. **The one metric a personal baseline can read is the one the tour band cannot.** Step 4's finding
   was that `head_hip_offset_impact_norm`'s sign is readable personally, because a personal corpus
   is single-handed by construction. The stored distribution is cut from GolfDB's *mixed*-handedness
   population — the exact reason M6.5 blocked it as a checkpoint. So the join must refuse the only
   metric that has both a center and a distribution. A naive implementation returns a confident
   percentile here and every "did we get a row back" test passes.
2. **`sd` against `sd` looks like a free second finding and is a category error.** `Distribution`
   carries an `sd` one field from the p10 already being read. But it is *between-player* variation
   (458 clips, 122 players, under four each) against a personal *within-player* one. "Your spread is
   tighter than the tour's" compares one golfer's repeatability to how much a field differs from
   itself, and is true for everybody. Recorded in `unavailable` rather than computed — and only once
   the golfer actually has a spread, since that is when the absence becomes a question.
3. **"Outside" is a verdict for half the panel and its opposite for the other half.** Caught by
   rendering the speaking path over a synthetic 18-swing corpus: `finish_balance_norm` sits *below*
   p10 on a one-sided `[0, high]` band — better balance than tour — and the page printed "outside
   tour range" in amber, identically to a center above p90. Fixed structurally rather than in
   wording: every standing pill is neutral (the contract carries no `score` and no `passed` so it
   cannot read as a verdict, and colour was re-adding one), and `outside` always names its side.
   This is step 5's defect one milestone later, and again only a render caught it — every assertion
   was green.
4. **A refusal keeps its evidence, and a model can do arithmetic.** `n`, `n_sessions` and the
   per-session counts stay populated because that is what makes a refusal actionable. Handed
   "session A: 4 swings, session B: 5" and a withheld mean, a model can average them and narrate the
   trend the guard just declined. Nothing in a payload stops it, so `READING_A_PERSONAL_HISTORY`
   names the move and forbids it — kept *separate* from the block `feedback/coach.py` gets, which
   writes about one swing and would be reading rules for tools it does not have.
5. **The import-boundary test everyone would write cannot work.** `analysis.baseline` and
   `analysis.dispersion` promise to import no `benchmarks`, and the obvious check — import them in a
   subprocess, inspect `sys.modules` — fails immediately: `analysis/__init__.py` imports `engine`,
   which reads the bands, so importing *anything* under `golf_coach.analysis` pulls them in before
   the module body runs. Had it happened to pass it would have been passing for the wrong reason
   forever. The property is about what those two files import, so the test parses their source.

**The window and the two sessions turned out to be one operation.** `get_shot_trends` narrows by
date, `compare_sessions` narrows by session id, and both hand the result to the same
`build_baseline` — so a per-session mean faces the identical CENTER floor a pooled mean faces, asked
of less data, and no second threshold exists to drift. `narrow_to` recomputes `metric_counts` rather
than filtering the swing list beside a stale count, which is step 4's pooling-vs-counting lesson
arriving by a different route.

**One output looked like a bug and was the dedupe rule working.** `compare_sessions` on the real
disk reports session `2026-08-09` with a mean score of 94.9 and **zero samples on every metric** —
its clips are re-uploads of a swing already counted in an earlier session, so it scores normally and
contributes no `n`. Left unexplained a reader concludes the corpus reader is broken, so the payload
says it in a sentence.

**Verified**: 540 tests, ruff and mypy clean. Against the real disk all three career CLIs still
agree on `n = 2` for every metric, the tour join refuses all eight placements (five waiting on `n`,
three blocked for want of a population), and the MCP server registers eight tools with a registry
and five without. Both pages were rendered head-less against a real payload and against a synthetic
18-swing corpus, and that second render is what caught defect 3.

**Where I left off**: career mode is complete. The milestone is `6/6` and moved to ROADMAP's Done
section, with the caveat stated there: it is finished and currently silent.
**Blockers**: none to build, for the first time in this milestone. Everything now waits on one bay
session — 20–30 swings with shots attached — after which every surface built here starts speaking
and the tolerances in `METRIC_TARGETS` are the first thing to revise against real repeats.
**Notes**: the em-dash/cp1252 note still applies to all three career CLIs (`PYTHONIOENCODING=utf-8`).
Two test modules briefly collided on the basename `test_career.py` — `tests/` has no `__init__.py`,
so basenames must be unique across test packages; they are `test_career_route.py` and
`test_career_tools.py`.

---

## 2026-08-12 — The shape of a miss, and a reading meant for one metric (career mode, step 5)

**Duration**: ~1 session, implementation + tests
**What prompted it**: step 4 built the baseline and made `pooled_samples()` public specifically as
this step's door. Step 5 is the argument the whole milestone rests on: this instrument cannot see
*why* a face is open — grip, lead wrist and release are all invisible to it — but the **shape** of a
miss is itself evidence. A miss that repeats at the same size is produced by something that is the
same every swing; a miss that moves cannot be. Those have different fixes, and separating them needs
no view of the body.

**What landed**: `contracts/dispersion.py` (`Finding`, `DispersionPattern`, `MetricTarget`,
`METRIC_TARGETS`, `PATTERN_READING`, `MetricDispersion`, `GolferDispersion`),
`analysis/dispersion.py` (`build_dispersion`, `dispersion_for`), `scripts/career_dispersion.py`, and
`analysis/baseline._refuse` promoted to `refuse`. 501 tests (up from 481), ruff and mypy clean.

**Two findings, never one verdict.** `bias` (the center is further from the target than measurement
error explains) and `scatter` (the spread is larger than measurement error explains) are answered
independently, because the *contrast* is the whole signal — the same 6° average miss means opposite
things at sd 1 and at sd 9, and collapsing them into one number throws away exactly the thing this
step exists to read. Both are decided by an interval and never a point estimate: bias needs the
mean's 95% CI to clear the tolerance band entirely, scatter needs the sd's CI **lower** bound to
clear it. So a tolerance set wrong surfaces as `NOT_ESTABLISHED` — an honest "cannot tell" — rather
than as a confident wrong pattern. No new statistics: `mean_ci` and `sd_ci` already existed.

**Five things found by building it:**

1. **The reading was written for one metric and printed for all eight.** The `BIASED` text named
   the checks outright — "grip, alignment, ball position, face at address" — which reads correctly
   under `face_to_path_deg` and is nonsense under `head_sway_norm`, where it printed **unchanged**.
   Found by rendering the speaking path on a synthetic session rather than by reasoning about it:
   the tests all passed while the output would have sent a golfer to check their grip about a head
   that moves. One reading serves every metric, so it may name only the class of cause; naming the
   specific check needs per-metric vocabulary and belongs in `feedback`. There is now a test that
   asserts the pose and shot readings are the *same string* and that it names no specific check.
2. **Six of the eight tolerances were already measured — they just had never been written down.**
   `tune_spatial_metric.py` computes `noise` + `bound`, which is precisely "the smallest difference
   distinguishable from this pipeline's own error", and it prints them and stores nothing. Re-ran it
   over the 461-clip face-on corpus (both estimator caches were already on disk, so no extraction):
   0.024 for `finish_balance_norm` up to 0.943 for `tempo_ratio`. Tempo's `noise` column is
   **0.000**, which is structural rather than lucky — it reads phase instants only and both
   estimators were handed GolfDB's labelled ones, so its entire error term is our own address
   detection, the weakest instant we have. The two launch-monitor metrics have no analogue for a
   photographed screen, so 2.0° is judgment, recorded as judgment.
3. **The tolerance does double duty, and both uses are the same quantity used correctly.** It bounds
   how far a center must sit from the target before a bias is real, and — because measurement error
   inflates observed spread (`sd_obs² ≈ sd_true² + sd_err²`) — it is also the level a spread must
   exceed before the scatter is the golfer's rather than the instrument's. A sample sd sitting at
   the tolerance is exactly what a perfectly repeatable golfer measured by this pipeline produces.
4. **Four of the eight may carry a scatter finding and must not carry a bias one.** Declaring a
   target is declaring what *good* is, which this repo does in exactly one place: a band with a
   derivation behind it. `hip_sway_norm` / `hip_shift_at_top_norm` have no band and "less is better"
   is not established for either; `head_hip_offset_impact_norm` has a readable sign (step 4's
   finding) but no known right amount; `tempo_ratio`'s target *is* a band, and reading it here would
   import `benchmarks` into the personal-baseline path — the boundary step 4 held deliberately. Each
   is refused with its reason attached, in `unavailable` rather than `withheld`, because the two
   need opposite responses: one says book another bay hour, the other says this needs a band first.
5. **A bias on a one-sided magnitude asserts less than it reads.** Zero head sway is unattainable,
   so "distinguishable from 0" is established for every golfer alive. What it actually says is *a
   consistent amount, above measurement error* — the half of the contrast this step needs — and not
   that the amount is too much. That is the tour band's question. Documented on the two entries it
   applies to, because the finding is true and the obvious reading of it is not.

**The guard is inherited, not re-invented.** `MetricDispersion` is built from the `MetricBaseline`
step 4 already sealed, so when `CENTER` was refused there is no mean *in the input* to test a bias
against — absent rather than ignored, which is "withheld means absent" holding by construction one
level further out. The one thing raw samples are read for is the within-session spread, and that is
itself gated on `SPREAD` so it cannot become a route around the seal. Only `_refuse` had to become
public; a second copy of the guard is how two floors drift apart with the looser one deciding what
gets said, which is step 4's own `artifact_key` lesson repeating.

**Pooled spread mixes two different quantities**, and the cause reading is only about one of them:
"your release is inconsistent" is a claim about one bay hour, while a pooled sd across sessions also
contains whatever the golfer changed in between. When two sessions each carry ≥2 samples the pooled
within-session sd is computed alongside, and a caveat fires when the two diverge. Classification
stays on the pooled figure — a proper variance decomposition has its own `n` requirements and there
is no corpus to test one against, so naming the possibility is the honest amount to say today.

**Verified**: 501 tests, ruff and mypy clean. `tests/analysis/test_baseline.py` passes with **no
assertion moved**, which is the check that making `refuse` public changed nothing. Against the real
disk, `career_dispersion.py --name Aaron` lists all eight metrics at n=2 with both findings withheld
and each naming its shortfall, and `career_corpus.py` / `career_baseline.py` / `career_dispersion.py`
agree on `n` for every metric. The speaking path was rendered separately over a synthetic 12-swing
session — a consistently open face reads `biased`, a wandering start line reads `scattered`, and
that render is what caught defect 1.

**Where I left off**: step 6, the last one — `get_shot_trends` / `compare_sessions`, the results
page, and the personal-vs-tour join. That join is what turns "a consistent 0.31 of head sway" into
"and that sits inside tour range", and it is also what would let the four target-less metrics
acquire a bias finding: one line each in `METRIC_TARGETS`.
**Blockers**: none to build. Everything after this wants the bay session — 20–30 swings with shots
attached.
**Notes**: the em-dash/cp1252 note from step 4 applies to this CLI too (`PYTHONIOENCODING=utf-8`),
unchanged for the same reason — it is a terminal setting, and it now affects all three career CLIs
equally.

---

## 2026-08-12 — A baseline that refuses to speak (career mode, step 4)

**Duration**: ~1 session, implementation + tests
**What prompted it**: steps 1–3 assembled the inputs and step 3's re-run brought `n` to 2 for all
eight metrics. Step 4 is the consumer: turn a `CareerCorpus` into per-metric statistics, and refuse
to report them below a threshold.

**What landed**: `contracts/baseline.py` (`PersonalBaseline`, `MetricBaseline`, `BaselineClaim`,
`WithheldClaim`, `SessionSample`, `MetricSample`, and the threshold table), `analysis/baseline.py`
(`build_baseline`, `pooled_samples`), `mean_ci` / `sd_ci` / `mean_and_sd` in `analysis/stats.py`,
and `scripts/career_baseline.py`. Against the disk it refuses all 24 claims (8 metrics × 3), each
naming what it waits for. 481 tests, ruff and mypy clean.

**The gate is per (metric, claim).** `CENTER`, `SPREAD` and `TREND` have different appetites for
`n` — the sample sd is a noisier estimator than the sample mean (relative error `1/sqrt(2(n-1))`,
still 24% at n=10), and a trend needs repeated *occasions* rather than repeated swings, so it gates
on `n_sessions` separately. One threshold per metric would either block a defensible mean or ship a
spread the data cannot support.

**Every statistic carries a 95% CI**, Student-t for the mean and chi-square for the sd. No scipy on
the base install (ADR-008), so both are small hardcoded critical-value tables for df 1..30 with
Cornish-Fisher / Wilson-Hilferty fallbacks beyond — pinned in `test_stats.py` against published
values and hand-computed intervals.

**Four things found by building it:**

1. **Pooling could have disagreed with counting, invisibly.** Step 2's dedupe rule lived inside
   `storage/corpus.py::_count_metrics`, which returned *counts and not values*. A baseline
   iterating `swing.measurements` naively would have averaged a different number of values than the
   `n` printed beside it — and only where the rule matters (a re-uploaded clip; one shot photo
   across two real swings), which is to say only where nobody would see it. Lifted the rule to
   `CorpusSwing.artifact_key`, called by both sides; the existing 21 corpus tests passing unchanged
   is the check that nothing moved, and a new end-to-end test pins
   `metric_counts[name] == baseline.metrics[name].n`.
2. **M6.5's spread/error ratios cannot set the thresholds.** The obvious move, and wrong: that
   ratio is *population* spread over *instrument* error, while what binds a personal baseline is
   the golfer's own shot-to-shot variability — unmeasured and much larger. Deriving from tempo's
   r = 2.4 gives a usefully-resolved personal mean at n ≈ 3. So the floors are judgment, documented
   as such, and the CI is what makes that safe: too low a floor reads as a visibly wide interval,
   not as a confident wrong number.
3. **Withheld had to mean absent, not flagged.** A statistic shipped beside `ready: false` is one
   forgotten conditional away from being rendered. Gated fields are `None` — `Measurement`'s
   "structurally incapable of reading as a verdict", one level up. What stays populated is the
   evidence (`n`, `n_sessions`, per-session counts), because that is what makes a refusal
   actionable rather than merely silent. The CLI demonstrates it: it never calls `supports()`, it
   just checks whether there is a number.
4. **`head_hip_offset_impact_norm` is readable here and nowhere else.** M6.5 blocked it as a
   checkpoint because its sign is camera-relative and a GolfDB band over mixed handedness would be
   meaningless. A personal corpus is single-handed by construction, so a personal baseline reads
   the sign without consulting `Golfer.handedness` at all.

**Where I left off**: step 5 (dispersion as cause discriminator) reads `pooled_samples()`. Step 6
surfaces it, and is where the personal-vs-tour join lands — `MetricBaseline` mirrors
`Distribution`'s shape so that is a lookup, not a redesign. `TREND` is gated but exposes only the
per-session breakdown; the slope and its significance wait for a corpus to test against.
**Blockers**: none to build. The `n` still needs a bay session — 20–30 swings with shots attached.
**Notes**: pre-existing and cosmetic — both career CLIs print em-dashes, which the Windows console
garbles at cp1252. `PYTHONIOENCODING=utf-8` fixes it; not changed here since it affects
`career_corpus.py` equally and is a terminal setting rather than a code defect.

---

## 2026-08-12 — The backfill, and the staleness nothing could see (career mode, step 3)

**Duration**: ~1 session, implementation + the real backfill over the four swings on disk
**What prompted it**: step 2's counter printed `n = 1` for all eight metrics, and named its own
worklist: three of the four `analysis.json` predate M6.5 and carry no `measurements`. Re-running
them was supposed to be the whole job.

**It was, but "which ones need re-running?" turned out to be unanswerable.** The only signal was
`measurements: []`, and that works *once*, by accident — M6.5 happened to add a field. The
counterexample is three entries below this one: the 2026-08-09 `_DRAWDOWN_FLOOR` fix moved a
stored tempo from 0.43 to 2.42 and changed the shape of nothing. An artifact from before it is
indistinguishable from one after it, and `AnalysisState.matches` cannot help — it compares the
*inputs*, which a re-analysis does not change. That is a live hazard for step 4 rather than a
tidiness complaint: a `PersonalBaseline` reads **spread**, so pooling two engine generations
manufactures variance out of a code change. Same failure as counting a re-uploaded clip, inverted.

1. **`contracts/swing.ANALYSIS_VERSION`** + `SwingBundleResult.analysis_version`. The default is
   **0, not the current version**, and that is the load-bearing part: the field is read back by
   parsing artifacts written before it existed, and a default of "current" would make every legacy
   file claim to be up to date — the one wrong answer indistinguishable from a right one. The
   engine sets it explicitly; `api/state.py` gains `stored_analysis_version` / `is_outdated`.
2. **`ExclusionReason.OUTDATED`** and `CareerCorpus.outdated_swings`. `counts_toward_metrics()`
   now asks three questions of three different things — the artifact (`analyzed`), the bytes under
   it (`stale`), the code that joined them (`outdated`).
3. **`scripts/reanalyze.py`** — targets whatever needs it, `--dry-run`, `--all`, `--video` and
   `--coaching` off by default. Walks the bundle store rather than `read_corpus`, deliberately:
   the corpus collapses re-uploads, but each of those directories still holds an `analysis.json`
   the results page and the MCP server will serve.
4. **`analyzed_without_measurements` survived, narrowed.** It gates on `counts_toward_metrics()`,
   so a pre-M6.5 artifact is now `outdated` and no longer lands there. What is left is
   "current-engine swing where every metric returned None" — measurement failed rather than the
   engine being old, a different problem with a different fix. Both read 0 now.

**The bug I found while making sure the backfill would not create one.** `analysis.state.json` is a
denormalised copy of `analysis.json` — the upload page's 5-second poll reads it so it does not
parse a 7 KB file per swing. Only `api/worker.py` wrote it, and `analyze_swing_dir` did not. So
every CLI re-analysis desynced them, and `2026-08-09/2` had been sitting for three days with a
sidecar reading **66.67** and the pre-fix "Tempo too quick - 0.4:1" headline beside an analysis
reading **94.92** / "2.4:1" — the session list showing 67/100 for a swing whose results page showed
95/100, with nothing anywhere able to flag it. Step 3 is a bulk CLI re-run, so shipping it as
planned would have produced two more copies of this while fixing something else. `record_state`
now lives in `api/pipeline.py` and runs as part of writing the analysis; the worker keeps
`queued` / `running` / crash-`failed`, which are the three states no analysis on disk corresponds
to. `tests/api/test_state.py` is new and pins the invariant, including that `matches()` returns
True across the desync — the reason nothing caught it.

**Key decisions**:
- **A re-run rewrites the whole artifact, and that is the honest choice.** A measurement-only patch
  was the cautious-looking option and is worse: today's code computing measurements while
  yesterday's checkpoint scores stay put yields one file whose `checkpoint_scores[tempo].observed`
  and `measurements[tempo_ratio].value` both claim to be this swing's tempo and can disagree.
- **`--video` off by default, with a check instead of an assumption.** Pose keypoints and shots are
  both content-addressed caches, so a re-run is seconds; the render is minutes. But a moved
  alignment anchor would leave `aligned.mp4` disagreeing with the JSON beside it, so the script
  compares anchors across the run and says so. They did not move here — which is exactly why it
  should be checked rather than assumed.
- **All four swings were re-run, including the one that already had measurements.** It was written
  before the stamp existed, so it cannot prove it is current, and "cannot prove it" is the whole
  rule. The corpus is now a single generation.
- **No ADR.** Nothing cross-cutting was decided; the rule lives in the `ANALYSIS_VERSION` and
  `ExclusionReason.OUTDATED` docstrings, which is where anyone meets it. Next free number stays 017.

**Two mistakes in my own reporting, both caught by running it.** The first run warned that the
alignment anchors had moved on all four swings and printed `window [574, 790] -> (574, 790)` as a
change. Neither was real: `_anchors` was reading the analysis dict one level too high so the
"before" side was always unknown, and the window row was comparing a JSON list against a tuple by
their string forms. Worth recording because the failure mode is the one this repo keeps meeting —
a diff tool that reports drift it invented is indistinguishable from a real regression, and I very
nearly re-rendered four videos to fix nothing.

**Verified**: 442 tests (up from 426), ruff and mypy clean. Against the real four swings:
`reanalyze.py --dry-run` found all four outdated, the run produced `version 0 -> 1` and
`measurements 0 -> 8` with **no other field moving** — scores, checkpoints, phases, windows and
anchors all identical to the pre-run backup — and a second plain run reports nothing to do.
`career_corpus.py --name Aaron` now prints **`n = 2` for all eight metrics**, no outdated swings,
and nothing excluded but the two known duplicates. Through the real API: session-list score and
results-page score agree on every swing, `2026-08-09/2` included. `aligned.mp4` mtimes untouched.
`mcp.query.get_swing("2026-08-07-aaron1", "1")` returns all eight measurements.

**Where I left off**: step 4 — `PersonalBaseline` and the per-metric minimum-N guard. Pure
functions over `CareerCorpus.metric_counts`, testable on synthetic input, no bay session needed to
*build*. The two `face_to_path_deg` samples now on disk are 10.9 and 13.2, which is what step 5
will consume and nowhere near enough to conclude anything from.
**Blockers**: none for step 4. Steps 5–6 want the bay session.

---

## 2026-08-11 — The corpus reader, and a dedupe key that only works one way (career mode, step 2)

**Duration**: ~1 session, implementation + verification against the real four swings
**What prompted it**: step 1 made "who swung this" addressable; nothing could *assemble* it. Every
reader in the repo is per-session — `get_session`, `get_session_summary` — so "every swing Aaron
has ever hit" was not a question the code could answer, and steps 4-5 are pure math over exactly
that list.

**The counting is the feature.** Four swing directories hold two swings: the same three files were
re-uploaded three times while the upload path was being tested. Counting directories would hand a
personal baseline one swing's numbers three times, which does not merely inflate `n` — it drives
the variance toward zero, and variance is the single quantity career mode exists to read (a tight
spread points at a static cause, a wide one at timing). The most confident possible wrong answer,
manufactured out of a testing artifact.

1. **`contracts/career.py`** — `CareerCorpus`, `CorpusSwing`, `ExcludedSwing`/`ExclusionReason`.
   `CorpusSwing.measurements` carries `Measurement` whole rather than flattening to name -> value
   the way `query._measurements` does, because `source` is what decides the sample count.
2. **`storage/corpus.py`** — `read_corpus(sessions_dir, player_id)`. Pure reads over artifacts that
   already exist; no file is re-hashed, since `RoleFile.content_sha256` was recorded as the upload
   streamed in.
3. **`scripts/career_corpus.py`** — the honest-`n` counter, printing per-metric `n`, the collapsed
   re-uploads, and everything contributing nothing, with reasons.
4. **`SwingBundleStore.list_session_ids()`** — the same listing existed inline in
   `mcp/query.list_sessions` and `scripts/backfill_golfer._sessions`; this would have been a third.

**The design error the test caught, and it was mine.** I argued for two dedupe keys — pose metrics
counting distinct face-on clips, launch-monitor metrics distinct shot photos — and the case I used
to justify it was one clip re-uploaded with a different shot photo attached, which I claimed was
one pose sample and two shot samples. The test asserting `n=2` failed, and it should have: the
duplicates share the face-on bytes, so they are one physical swing, and one swing produced one ball
flight. The second photo is misattached. Counting it would have put a `face_to_path_deg` into a
dispersion that no swing ever produced — the exact class of error the milestone is being built to
avoid, arrived at while arguing for the mechanism meant to prevent it.

The two keys are still right, but they diverge in only **one** direction: `bundle_store`'s "newest
swing missing this role" rule can attach one photo to two genuinely different swings, which is two
pose samples and one shot sample. The other direction is a data conflict, reported as
`conflicting_shots` for repair — the same posture `bundle_store` already takes toward a
misattributed upload, which it documents and hands to a human rather than engineering around.

**Key decisions**:
- **No fallback to `checkpoint.observed`.** Three of the four `analysis.json` predate M6.5 and
  carry `measurements: []` while their checkpoints still hold `observed` (tempo 2.35, head sway
  0.25). Reading those as measurements would have lifted `n` to 2 for three metrics immediately and
  mixed two derivation paths under one name. Reported as `analyzed_without_measurements` instead,
  which is precisely step 3's worklist.
- **`excluded` means "contributes no sample", not "absent from the corpus".** An unanalyzed or
  stale swing is a real distinct swing carrying no usable numbers yet; both are fixed by re-running
  the pipeline, so they are reported as work rather than as absence.
- **Survivor of a duplicate group is the earliest arrival.** A re-upload's timestamp dates the
  upload, so taking the latest would file a swing under the day someone retested the upload path.
- **`storage` imports `api.state`**, which inverts ADR-008's direction. Deliberate, and the
  precedent `mcp/query.py` set: a second copy of a tolerant reader is a second copy that drifts.
  The honest fix — moving `load_analysis`/`load_state` to `storage/analysis_io.py` — is contained
  and is not this commit.

**Verified**: `python scripts/career_corpus.py --name Aaron` reports 4 directories -> 2 distinct
swings, 2 distinct shots, 2 collapsed re-uploads of `face_on 91b9d32c`, and `n = 1` for all eight
metrics. That last number is the point: it is what the ROADMAP already asserted in prose, now
produced by code, and it makes step 3's scope self-evident. Full suite green (426 tests), ruff and
mypy clean.

---

## 2026-08-11 — Who swung this, recorded before the bay session (career mode, step 1)

> **Written 2026-08-12, after the fact.** This session shipped without an entry; the record below is
> reconstructed from the ROADMAP §Career step-1 paragraph, which was written at the time. It carries
> only what that section already states — no findings have been added from memory.

**Duration**: ~1 session, implementation + a backfill over the four swings on disk
**What prompted it**: career mode was about to be declared "nothing to build, only `n` to collect",
and that was wrong about exactly one thing — **capture-time metadata**. Everything else career mode
needs is derivable from artifacts after the fact, so it can be built whenever. Who swung a clip and
which way they face are *recorded or lost*. Those had to land before the bay session, not after it.

**What landed**: `contracts/golfer.py` (`Golfer`, `Handedness`, `slugify`), a flat-file registry in
`storage/golfer_store.py`, a per-session cursor in `storage/session_meta.py`, `player_id` stamped
**write-once** onto `SwingManifest`, a golfer bar on the upload page, and `scripts/backfill_golfer.py`.
The backfill has been run: all four existing swings are `aaron`, right-handed.

**Uploads are deliberately never blocked on it.** A phone at a bay uploading three files is the one
moment in this system where friction costs data that cannot be recovered — the swing is over. So an
unlabeled upload is accepted, setting a golfer *adopts* the swings that arrived without one, and each
swing row carries a repair link. Identity is a thing you attach, not a gate you pass.

**Handedness is on the record for a reason that only pays off later.** It is what will eventually let
`head_hip_offset_impact_norm`'s camera-relative sign be interpreted, since the GolfDB band behind it
is cut from a mostly right-handed population.

**Where I left off**: step 2, the cross-session corpus reader — every reader in the repo is
per-session, so "every swing Aaron has ever hit" was not yet a question the code could answer.
**Blockers**: none.

---

## 2026-08-11 — Claude writes the verdict, and a percentile of 90 printed as 9 (M6)

> **Written 2026-08-12, after the fact.** This session shipped without an entry; the record below is
> reconstructed from the ROADMAP §M6 section, which was written at the time. It carries only what
> that section already states — no findings have been added from memory.

**Duration**: ~1 session, implementation + tests against a fake client
**What prompted it**: the analysis produces numbers, bands, percentiles and ranked tips, and a golfer
still has to assemble them into a sentence. M6 is the sentence.

**What landed**: `feedback/coach.py` (prompt template + the API call), `contracts/caveats.py`,
`CoachingProvenance` on `contracts/feedback.py`, the coaching card in `api/static/results.html`, and
`coaching_model` / `coaching_enabled` in `config.py`. The model moved to **`claude-opus-5`** — the
2026-08-10 entry below flagged `claude-opus-4-8` sitting behind a comment claiming "latest, most
capable" and deferred it to here, which is where it belongs.

**The brief is rendered, not dumped.** Every value is labelled with the vocabulary the caveats warn
about — `unscored`, `percentile`, `needs_review`, `alignment_caveat` — so a warning about `unscored`
lands beside a line that actually says `unscored`. Keypoints and phases are excluded: several hundred
frames of landmarks that no coach reasons from and that would dominate the prompt.

**It never raises for an expected failure.** No key, no `llm` extra, a rate limit, a refusal, a
truncation — each returns a *reason* that becomes a note on the result. Coaching is the last thing
that happens to a swing and the least important thing about it; it must never be able to cost a
golfer their score.

**One source of truth for the caveats.** The standing warnings moved to `contracts/caveats.py` and
are composed by **both** `mcp/server.py` and `feedback/coach.py`. ADR-008 forbids either importing
the other, and the alternative was two copies of load-bearing prose drifting apart — which this repo
has been bitten by three times.

**The bug, found by running it rather than reasoning about it.** The first cut trimmed trailing zeros
unconditionally, so a percentile of **90 rendered as 9** and 10 as **1**. Plausible, wrong, and aimed
straight at a model that would have repeated it as fact. Chasing it exposed a second, real inaccuracy
in the caveat *text*: it claimed every failing checkpoint reports "about 90", but a two-sided metric
that misses **low** clamps to 10 — as tempo does on `2026-08-10/2`. Both fixed, the second for the
MCP server too.

**Display is gated on provenance, not just on text.** `results.html` renders the paragraph as a
tinted card, never as a fourth `.tip`, with the model named *above* the prose. Unattributed prose on
a page of measurements is indistinguishable from a measurement.

**Verified**: full suite green, ruff and mypy clean — against a **fake client**. A swing analyzed
with a key configured carries `feedback.coaching_text` and `feedback.coaching` (model, timestamp, and
a sha256 of the brief, so a stored result can tell when the numbers moved underneath it).

**Where I left off**: the live path is unproven. **No real request has ever been sent** — no
`GOLF_ANTHROPIC_API_KEY` is configured. Put a real key in `.env`, run
`python scripts/analyze_bundle.py 2026-08-10/2 --no-video`, and read the paragraph against the
numbers above it: it must assert nothing the brief did not contain, and must not mention spine angle,
hip rotation, swing plane or club path — none of which this system measures.
**Blockers**: an API key, and nothing else.

---

## 2026-08-11 — Measure now, judge later, and the circle that kept the panel at three (M6.5)

> **Written 2026-08-12, after the fact.** This session shipped without an entry; the record below is
> reconstructed from the ROADMAP §M6.5 section, which was written at the time. It carries only what
> that section already states — no findings have been added from memory.

**Duration**: ~1 session, implementation + a full re-derivation over the 461-clip face-on corpus
**What prompted it**: the panel had sat at three checkpoints for two milestones, and the reason was
not that any metric was hard. **Measurement and judgment were fused.** `evaluate_head_sway` measured,
resolved a band, scored, and returned `None` if *any* step failed — so a metric with no band could
not be measured at all, while bands are derived from populations of measurements. That circle is the
blocker, and it is a code-shape problem rather than a data problem.

**What landed**: `analysis/measure.py` and `analysis/shot_measure.py` (the measuring half — pure, no
`resolve_range`, where `None` means *could not measure*), the `Measurement` contract,
`scripts/golfdb/tune_spatial_metric.py`, and a registry-driven `derive_pose_metrics.py`.
`checkpoints/mechanics.py` keeps the judging half and the three evaluators are unchanged in
behaviour — pinned by the existing `tests/analysis` suite passing with **no assertion moved**.

**`Measurement` is ADR-010 §2 expressed as a type rather than a convention.** No band, no `passed`,
no `score`. A metric earns a verdict only once a population exists to judge it against, and until
then it is data — so a measurement is made *structurally incapable* of rendering as a fault, however
it travels.

**Five new metrics, and the ones deliberately left out matter as much.** Three face-on pose:
`hip_sway_norm`, `hip_shift_at_top_norm`, `head_hip_offset_impact_norm` — all hips/shoulders/ears,
all windowed, all `x`-over-`x` and so immune to the 16:9 pixel-aspect assumption. Vertical and
shoulder-tilt metrics were excluded for being aspect-*sensitive*; ankles and knees for having zero
recorded reliability evidence. Two launch-monitor: `face_to_path_deg` and `start_line_deg`.
`smash_factor` and `club_head_speed` are excluded because every shot on disk reads smash 0.89–1.00 —
ball speed *below* club speed — and the OCR is faithful, so the **simulator itself** is printing a
physically impossible number. `spin_axis` too: its sign contradicts the contract and the parser warns
that it stored an uninterpreted magnitude.

**The gate the repo did not have.** There were three harnesses for *temporal* rules and none for
spatial quantities, so "should we add this checkpoint?" was answerable only by argument.
`tune_spatial_metric.py` scores population spread against measurement error, where error is estimator
disagreement (`lite` vs `full`, both already cached for all 461 face-on clips — no new extraction)
plus segmentation error (labelled vs detected instants). All six pose metrics clear it
(spread / error): `finish_balance` **8.2**, `head_hip_offset_impact` **7.6**, `hip_sway` **7.1**,
`head_sway` **6.7**, `hip_shift_at_top` **3.6**, `tempo` **2.4**.

**The harness validates itself three ways**, which is the only reason to believe the numbers above:
it reproduces `head_sway_norm`'s shipped band (p10–p90 **0.029–0.430** against `ranges.json`'s
0.0–0.43), `finish_balance_norm`'s p90 (**0.287** against 0.29), and the face-on `tempo_ratio` p90 of
**5.000** that `mechanics.py` documents against the all-view 4.71.

**Two things found by running it:**

1. **Normalizing the simulator's shape text matched `CENTER` before `FADE`**, classifying a real
   recorded `"CENTER SLIGHT FADE"` as **straight**. Curvature words now beat centering words.
2. **The corpus had been storing `CheckpointScore.observed`, which is rounded to 2dp.** Measurements
   now carry full precision. Re-deriving moves 42 distribution rows by under 0.005 and leaves every
   shipped band unchanged at the precision it quotes — which is the check that says this was a
   latent-precision bug rather than a change of answer.

**`head_hip_offset_impact_norm` is signed and camera-relative.** It is empirically *not* bimodal on
this corpus (p10 −0.88 to p90 −0.33, consistently head-behind-hips), so a band is derivable — but
that is a fact about GolfDB's handedness mix, not a guarantee, and handedness has to be resolved
before it can become a checkpoint. `derive_reference.py`'s recommended band for it is also garbage
(`low=0.00 high=-0.33`): its one-sided heuristic assumes non-negative values.

**Where I left off**: nothing is scored and `ranges.json` is untouched — **promotion is deliberately
a separate decision**, and it wants more than one golfer's swings behind it. Exit criteria for the
milestone are otherwise met: every measurable quantity is recorded on every analyzed swing, with a
harness that says which of them a band is worth deriving from.
**Blockers**: none to build.

---

## 2026-08-10 — The MCP server, and a tool list three milestones out of date (M3)

**Duration**: ~1 session, implementation + verification against real bay data
**What prompted it**: Looking for the next task. The MCP server was the last M3 item, fully
unblocked — `mcp>=1.0` declared in the `llm` extra since March, `scripts/run_mcp_server.py` a
`NotImplementedError` stub, ADR-006 accepted. Straightforward. It was not.

**ADR-006's tool list predated everything that makes this project worth querying.** All five tools
it specifies — `get_recent_shots`, `get_shot_by_id`, `get_session_summary`, `compare_sessions`,
`get_shot_trends` — are shot-only, written 2026-03-16 when M3 *was* the system. Since then the
pose-only track delivered scored swings with tour percentiles and ranked tips, and M7 Phase 4
started writing `analysis.json` per swing.

So the shot metrics are now the **less** differentiated half of what this repo holds. They are HD
Golf's own readout, photographed and OCR'd — the simulator already shows them on a screen in front
of the golfer. What only this system can say is where a swing sits against 458 tour swings. Built
to the table as written, Claude could report "club speed 98.3, carried 121 yards" and would have
been unable to say "your head sway sits higher than 83% of tour swings." That inverts the point of
a coaching interface, so the tool set covers both axes of ADR-009's model. ADR-006 has a second
addendum recording it; no new ADR, because nothing was re-decided, only re-scoped.

The join turned out to be free: `data/processed/shots/*.shot.json` carries `session_id`, and the
bundles live at `data/processed/sessions/<session_id>/<swing>/`.

1. **`mcp/query.py`** — reads sessions, swings and shots. Imports no MCP SDK, so it installs and
   tests on the base install; a subprocess test pins that the way `test_pipeline_imports.py` pins
   fastapi out of `api/pipeline.py`. Reuses `SwingBundleStore`, `api/state.py`'s `load_state` /
   `load_analysis`, and `ScreenShotDataSource` behind `CompositeShotDataSource` — whose docstring
   has named the MCP server as an intended consumer since M3 shipped.
2. **`mcp/server.py`** — five tools, stdio transport, delegating every call. `settings.mcp_port`
   (8081) stays unwired: under stdio the client launches the process and talks over the pipe.
3. **`scripts/run_mcp_server.py`** — the stub replaced, `mcp` imported lazily so a base install
   gets an instruction instead of a traceback out of a process holding a client's pipe.

**The views are not passthroughs, and that is the substance of the phase.** An LLM presents what it
is handed as fact, so everything this repo knows to be provisional had to survive the trip out:
`needs_review` hoisted from inside `ShotProvenance` to the top level, the alignment tier turned
into an actual sentence about what not to conclude from the side-by-side, and `unscored` labelled
as "dropped, not failed" — with the same three restated in the server's `instructions`, which the
model reads once on connect, alongside a note that spine angle and swing plane are *not* measured
here and must not be inferred.

**Key decisions / surprises**:
- **Two tools from the original table are deliberately not built.** `get_shot_trends` and
  `compare_sessions` both invite Claude to narrate a trend, and the store holds **three** shots. A
  trend tool over n=3 reports noise in a confident voice — the same failure ADR-012 found when a
  120-clip result vanished at 461. Recorded as a decision in the addendum so it does not read as an
  oversight.
- **ADR-008 puts the MCP server in `launch_monitor/`**, which was right for a shot-only server and
  wrong now: it would make the launch-monitor module import `analysis` and `storage` to serve tools
  that have nothing to do with a launch monitor. New top-level `src/golf_coach/mcp/`. (`from mcp.server
  import ...` inside `golf_coach/mcp/` resolves to the SDK — Python 3 has no implicit relative
  imports. It reads like a shadowing bug and is not one.)
- **A tool returning `None` serializes to zero content blocks**, which reads identically to a call
  that silently did nothing — and my tool descriptions were claiming "returns null". Found by
  running it, not by reasoning about it. The three lookup tools now return an explicit `NotFound`
  that names the miss and points at the tool listing valid ids.
- **The listing said "not analyzed" for swings that were analyzed.** The 2026-08-07 sessions have an
  `analysis.json` but no `analysis.state.json` — analyzed by the CLI before Phase 5 introduced the
  sidecar. Reading only the sidecar contradicted `get_swing`, which returns their full result a
  moment later. Falls back to `analysis.json` when the sidecar is absent, which costs one parse on
  legacy swings only.
- **`mcp` 2.0 is not the API in most examples.** `FastMCP` is `MCPServer`, and the wire fields are
  snake_case (`input_schema`, `is_error`) where older versions used camelCase. Checked against the
  installed package rather than recalled.

**Verification**: 294 tests pass (27 new), ruff and mypy clean. Against the real bay data rather
than fixtures: `list_sessions` returns four sessions newest-first, `get_swing("2026-08-10", "1")`
returns **94.9** with all three checkpoints, their bands and percentiles, the `top_impact` caveat
attached, and the joined shot at 125.6 yards carry — matching `analysis.json` exactly.

**Where I left off**: M3's only remaining no-hardware item is tuning OCR preprocessing on a real
range session's photos. The server is ready for M6 to build on.
**Blockers**: None. Not yet registered with a real Claude Desktop / Claude Code client — the tools
were exercised through `call_tool` in-process, which validates the schemas and the data but not the
client handshake.
**Notes**: `config.py:coaching_model` is `claude-opus-4-8` and its comment says "latest, most
capable"; the current model is `claude-opus-5`. Unread until M6, so left alone — fold it in there.

---

## 2026-08-09 — One noise bug behind three separate symptoms (tempo, playback speed, sync)

**Duration**: ~1 session
**What prompted it**: Looking at `results.html?session=2026-08-09&swing=2` and finding three things
wrong at once — the down-the-line panel of the aligned video plays visibly fast, tempo reads 0.4:1
against an eyeballed ~2:1, and the face-on swing visibly starts before the down-the-line swing.

**They were all one bug, and it was not in the video path.** There is no client-side sync to get
wrong: `results.html` plays a single pre-rendered `aligned.mp4` and there is no `playbackRate`
anywhere in the repo. The distortion was baked in at render time by a wrong phase anchor.

`_rising_runs` closed a descent run when the drawdown exceeded `_DRAWDOWN_TOLERANCE` of the run's
**own accumulated rise** — which is near zero in a run's first frames, so any noise clears it. The
golfer hovered at the top; the smoothed lead wrist drifted 0.3205 → 0.3244 over nine frames and
wobbled back 0.0020, which was more than a quarter of the 0.0039 accumulated. That split the
descent, and `_top_and_impact` took the second fragment: **top at 704 instead of 694, a 14-frame
downswing where the truth was 24.**

Everything downstream inherited it:

| | before | after |
|---|---|---|
| face-on downswing | 14 fr (0.234s) | 24 fr (0.400s) — matches DTL exactly |
| scored window (`select_swing` leads by 5 downswings) | (634, 760) | (574, 790) |
| face-on `motion_start` | 698 — 6 frames before the top | 636, still `detected` |
| tempo | **0.43:1** | **2.42:1** |
| DTL panel speed | **1.69×** | 0.99× |
| panel offset at first frame | **+983 ms** | −2 ms |

The tempo number was never a `_motion_start` bug, which is where ROADMAP and the runbook both
predicted it. `_motion_start` was doing its job on a top that was 10 frames late; widen the window
by fixing the top and it finds the takeaway unaided.

**The occlusion hypothesis was wrong, and I nearly built on it.** The 2026-08-07 entry above (and
`phases.py`'s own `_PLAUSIBLE_DOWNSWING_S` comment) blamed the disagreement on the DTL lead wrist
being the far, occluded arm. Measured before planning around it: on DTL the two wrists **agree**
(LEFT → 24 frames, RIGHT → 25). Face-on was the outlier at 14. Three of the four measurements
cluster at a top of ~694 in face-on coordinates; only face-on's LEFT_WRIST said 704. There is no
DTL landmark problem to fix, and no view-aware wrist selection was written.

**Validation.** The floor was swept against the 461-clip GolfDB corpus, paired per-clip against the
floor-0 baseline rather than compared on pooled columns — the pooled table is too coarse to choose
with (median and impact do not move at all). Impact is untouched at every floor tried: **0 clips
move.** Top at the chosen 0.012: median 2.0 unchanged, mean 10.56 → 10.58, the >10 tail count
unchanged at 46, and 12 clips better against 12 worse. Past 0.014 the tail count and the win/loss
balance both turn. Address re-measured after (it scales its quiet run off top/impact): median 7.0
unchanged, mean 22.9 → 22.8, PCE 15.8% → 16.1%.

**Also landed, as a guard rather than a fix**: `align_swings` now checks the two views' downswing
*durations in seconds* and, when they disagree beyond 30%, emits `AlignmentQuality.IMPACT_ONLY` —
a tier that was defined, documented and never reachable. There both clips take one shared duration
measured back from impact and each panel advances at its own native rate, so a disagreement about
instants can no longer be paid for in playback speed. Run against the *pre-fix* anchors it turns
the 1.69× panel into 0.99×, which is what it is for. It does not fire on this swing any more.

One guard I wrote and then deleted: refusing the fallback when the two clips' reported frame rates
differ. A 30 fps phone beside a 60 fps one is an ordinary pairing, not a broken clock, and slo-mo's
stretched rate is not detectable from the number anyway. Replaced with the check that can actually
be made — that the duration about to be imposed on both panels is a physically possible downswing.
There is a test pinning the 30/60 case specifically.

**Next**: the two views still disagree on *tempo* (2.42 vs 1.50) because DTL's `motion_start` reads
~22 frames late, so quality stays `top_impact`. Harmless now — the fallback anchor is symmetric and
derived from downswings that agree — but it is the remaining soft-anchor weakness on real footage.

---

## 2026-08-09 — The doc that planned work already done (M7 Phase 6, closed out)

**Duration**: ~1 session, docs and validation only — no `src/` or `scripts/` change
**What I did**: Started this session by pasting the Phase 6 planning prompt out of
`docs/M7_TWO_PHONE_CAPTURE.md` into a fresh context, as the doc instructs. **Phase 6 shipped
three days ago, and the prompt contradicts what shipped point by point** — bind the tailnet IP
(the bind never widens), add a host to `config.py` (deliberately not a field), `api_port: int =
8080` (it is 3000), tailnet membership is the access control (it stopped being sufficient the
moment Funnel existed). Building it as written would have reversed ADR-016 and deleted a guard
that has a regression test.

So this commit is the close-out Phase 6 never got, plus disarming the trap:

1. **`docs/BAY_SESSION_RUNBOOK.md`** (new, AS-BUILT) — the two bullets of the Phase 6 prompt that
   were genuinely never delivered: what to check before driving out, and the failure modes to
   expect on-site. At-home preflight, the guest/Funnel path, 1080p60-not-4K with the reasoning,
   a timings table built only from measured numbers, the per-swing loop, a symptom→cause→check
   table, and what footage to bring back for Phase 0.
2. **Bannered the planning prompts.** Phases 1–6 are built, so their prompts are history, not
   instructions; Phase 6's carries a `🛑 SUPERSEDED` banner with the point-by-point diff, because
   it is the one that is affirmatively wrong rather than merely dated. Kept, not deleted — the
   divergence is the interesting part, and this repo banners rather than rewrites.
3. **Fixed the stale counts** that let this happen quietly: the M7 ladder still showed four built
   phases as `[ ]`, the doc header said "no phase implemented yet", `README` said ADRs 000–014,
   `docs/README` said 14 decisions and 31 documents, `ROADMAP` was dated 2026-08-05 and described
   `run_server.py` as hard-coding `127.0.0.1`.

**Key decisions / surprises**:
- **A planning doc that outlives its phase becomes an instruction to redo it.** The prompts were
  written to be self-contained so a cold session wouldn't need the surrounding doc — which is
  exactly what made this one dangerous, because the context that would have corrected it was the
  part deliberately left out. Self-contained prompts need an expiry marker; that is the general
  lesson, and the banner is the cheap version of it.
- **The doc count is now spelled out by directory rather than given as a bare number** (38 = 31 in
  `docs/` + 7 elsewhere, checkable with `git ls-files '*.md'`). A bare count had already gone
  stale twice; a number nobody can verify is one nobody updates.
- **The tempo defect is now in the runbook as a "do not trust this" item.** It was tracked in
  ROADMAP, which is not what anyone reads at a driving range. Whoever takes this to a bay would
  otherwise have been handed a confident, wrong "work on tempo first" with no caveat anywhere in
  reach.
- **No ADR written.** Nothing new was decided here — ADR-016 already covers all of it. The next
  free number stays 017.

**Where I left off**: M7 has two things left and one trip closes both — the Phase 0 field spike
(method locked 2026-08-07, thresholds pre-committed, waiting only on footage) and the on-site
validation of everything Phases 3–6 built. The runbook is written to be the thing you actually
carry.

**Blockers**: **The Funnel guest path is still unvalidated** and I could not validate it from
here — it needs a physical phone that is not on the tailnet. The procedure is written up in §2 of
the runbook and takes about five minutes at home: `tailscale funnel --bg 3000`, confirm 401
without a token and 200 with `?t=`, upload one clip, time it, `funnel --bg off`. The measured
time fills the one blank row in the runbook's timings table. Serve was verified on 2026-08-06,
but tailnet membership was also protecting the endpoint then, so the token has never actually
been the only thing holding the door.

**Notes**: Next commit is the tempo defect — `phases._motion_start` or `evaluate_tempo` via the
`MIN_PLAUSIBLE_TEMPO` floor `analysis/alignment.py` already has, measured against GolfDB first.

---

## 2026-08-09 — No command at all (M7 Phase 5, completed)

**Duration**: ~1 session, implementation + end-to-end verification through the real server
**What I did**: Closed the loop. An upload used to land a file and stop; now the third file
starts the analysis and a results page renders it.

1. **`api/pipeline.py`** — the orchestration lifted out of `scripts/analyze_bundle.py`
   near-verbatim. `print()` became an injected `log` callback so a headless worker doesn't lose
   the narration, and anything a *reader of the result* needs — a short decode, a shot that
   couldn't be read, a swing the selector declined to pick — became a durable note in
   `analysis.json` instead of a line on stderr nobody is watching. The CLI kept every flag and
   every exit code and is now presentation only. It imports no fastapi, so it still runs on a
   `vision`-only install; `tests/api/test_pipeline_imports.py` pins that in a subprocess.
2. **`api/worker.py`** — asyncio queue, one consumer, `asyncio.to_thread` for the heavy part.
   Triggers **only on a complete bundle**; a partial one waits for an explicit "Analyze anyway"
   behind a confirm dialog. No timeout heuristic, deliberately — it would be wrong in both
   directions. `api/state.py` persists an `analysis.state.json` sidecar keyed on the
   role→sha256 map, so a re-uploaded clip invalidates its own result, and the 5 s status poll
   reads a denormalised score rather than parsing a 7 KB JSON per swing.
3. **Routes + `results.html`** — swing detail, the analyze override, and a video route (byte
   ranges confirmed; iOS Safari won't play a `<video>` without them). Path segments are now
   validated — `..` was a live traversal into `sessions_dir`'s parent.

**Three things I was wrong about, all caught by running it rather than reasoning about it:**

- **The `avc1` codec fallback.** OpenCV's bundled FFmpeg carries no libx264, only libopenh264,
  and dlopen's a DLL that isn't shipped — so it prints `Failed to load OpenH264 library` and
  fails. I expected the fallback to `mp4v` to fire. It doesn't: OpenCV falls back to Media
  Foundation, which encodes real H.264. The output is `avc1`, 85/85 frames, honouring the
  requested fps exactly. `isOpened()` is the only thing worth believing; the stderr lies.
- **`isOpened()` reporting success mid-failure** is why `RenderResult` now carries the codec
  that actually won rather than the one that was asked for.
- **`asyncio.Queue.put_nowait` from a non-loop thread** enqueues without waking a consumer
  parked on `get()`. Only bit a test — every production caller is a route handler — but the
  constraint is now documented on `submit()`.

**Verified end to end** through `run_server.py` against the real bay footage: three uploads
(35 MB in 0.1 s), `queued=False` on the first two, `complete` + `queued=True` on the third,
`running → done` in **31.8 s**, score 86.1 matching the CLI exactly, and the video served as
H.264 with `accept-ranges: bytes`. The partial path was checked separately: face-on alone
auto-started nothing, the override produced `partial=true` with the three degradations spelled
out in the notes, no aligned video, and the raw face-on clip served as the fallback.
Also found that uvicorn leaves the root logger at WARNING, so the worker was running silently —
`run_server.py` now configures logging and the whole pipeline narrates to the terminal.

258 tests pass (31 new, all on the base install thanks to the injectable `runner` seam), ruff
and mypy clean.

**Where I left off**: M7 has only the Phase 0 field spike outstanding. The upload leg of Q2 is
still unmeasured — nobody has confirmed what iOS Safari does to a `.mov` on submit, and one real
phone upload plus `spikes/2026-08-07-two-phone/probe.py inspect` answers it. The louder problem
is the tempo checkpoint: it is untrustworthy on real footage and the tips now lead with a
confident, wrong "work on tempo first" to a reader who never sees a caveat on a terminal.

---

## 2026-08-08 — One command, whole bundle (M7 Phase 4)

**Duration**: ~1 session, implementation + verification against real footage
**What I did**: Joined everything that already existed into one command, and found out along the
way that the hard part was not the joining.

1. **`analyze_swing_bundle()`** (`analysis/engine.py`) — pure. Scores the face-on view through
   `analyze_swing()` **unchanged**, uses down-the-line for alignment anchors only, attaches the
   shot. Reuses the phases `analyze_swing` already computed for the anchors rather than
   segmenting twice, so the frame a checkpoint was measured on and the frame the warp pins to
   τ=1 *cannot* drift. New `SwingBundleResult` contract; `SwingResult.shot` is finally populated.
2. **`scripts/analyze_bundle.py`** — the one command. Writes `analysis.json` (7 KB — heavy
   streams excluded, the keypoints already sit beside it), `aligned.mp4`, and per-view keypoints
   cached against the clip's sha256. Cold run 50 s, warm 9 s.
3. **`phases.select_swing()`**, which was not in the plan and turned out to be the substance of
   the phase — see below.
4. **Two moves, no duplication**: the side-by-side renderer out of `align_swings.py` into
   `pose/side_by_side.py`, and `_import_one` out of `import_shot_screens.py` into
   `launch_monitor/screen/importer.py`. The renderer move needed a seam — it would otherwise
   have made `pose` import `analysis` — so the frame correspondence became
   `alignment.pair_frames()` returning `FramePairing`s, and the renderer now only draws.

**Verification**: 227 passed (up from 201 passed / 4 skipped — the OCR integration tests run for
the first time), ruff clean, mypy clean. `aaron-swing-2` still reports **TOP @ 400 / IMPACT @
423**. The renderer move is provably faithful: re-rendering `aaron-1-aligned.mp4` gives 98 frames
at **0.000 mean absolute pixel difference** against the committed reference — every pixel.
End-to-end on the genuine `aaron-1` triple: auto-selection reproduced the Phase 2 verified pair
unaided (face-on 704/718, down-the-line 1550/1574), a *new* shot photo OCR'd at conf 0.93, and
all 24 banner frames render on **both** panels with zero strays — at IMPACT the club is at the
ball in both views.

**Key decisions / surprises**:
- **The window is not framing — it decides what gets scored.** This is the whole reason
  `select_swing` exists. Unaided, `segment_phases` picks a *setup move* on all four real bay
  clips. Whole-clip `aaron-1-front` scores 58/100 with tempo unscored and finish balance a 0.53
  MISS; the actual swing scores 67/100 with both in band. On the 2026-08-07 bundle the same
  effect took finish balance from 0.46 (a MAJOR fault) to 0.07 — the unwindowed figure was
  measuring the golfer *walking away after the shot*.
- **"Take the last descent" is half right, and the half that fails does so silently.** Aaron's
  idea — nobody rehearses after hitting — is correct on both face-on clips and wrong on both
  down-the-line ones, because the DTL phone keeps rolling 15–24 s past impact on the busy side
  of the bay. On `aaron-2-back` it picks a 5-frame artifact at the very end of the clip. And
  *nothing catches it*: `align_swings`' tempo cross-check is skipped once the soft anchor has
  already been refused for another reason, so the wrong pick renders a confident, plausible,
  completely wrong video. What rescues the idea is **downswing duration** — real swings measured
  0.23/0.38/0.40/0.42 s against a setup-move cluster at 0.48–0.53 s — so: filter by duration,
  then take the last. Correct on all four.
- **`candidate_downswings`' default 0.80 rise threshold hides the swing.** On `aaron-1-back` the
  largest descent is a *bystander* (0.417), which puts the cut at 0.334 and excludes the real
  swing (0.257) entirely. Selection uses the same looser threshold `--list-swings` already did,
  now shared as `CANDIDATE_MIN_RISE`, so the set a human chooses from and the set the rule
  chooses from are identical.
- **The window's lead-in was sized for viewing, not measuring.** Phase 2's 2 downswings before
  the top is *inside* a tour-tempo backswing (~3.5), so motion start went undetected on two of
  six clips, which drops the tempo checkpoint and degrades the alignment. Swept 2–8 across all
  six: **5** is the smallest that detects motion start everywhere while leaving top and impact
  exactly where they were. On the 2026-08-07 bundle that alone took the alignment from
  `top_impact` to `full` and made tempo measurable.
- **The band is calibrated on 60 fps and a 30 fps clip reads longer** — the bundle's face-on view
  measures 0.60 s for its only swing. Widening the band would swallow the whole setup-move
  cluster at 60 fps, so instead: **one candidate wins on its own**, whatever it measures. There
  is nothing to choose between, and `segment_phases` would pick that same descent anyway — the
  only question is whether it gets measured in isolation or with the clip's dead air mixed in.
- **`paddle.py` had a latent bug that only a real install could find.** It was written to absorb
  the PaddleOCR 2.x/3.x split in the *result* shape but still called the 2.x *constructor*, and
  3.x validates argument names strictly. Nobody had noticed because `paddleocr` had never been
  installed here — the integration tests had always skipped. Also had to disable oneDNN:
  paddlepaddle 3.3.1's kernel aborts at *predict* time on this CPU, well after a clean
  construction, so no constructor fallback could catch it.
- **The `2026-08-07/1` bundle is not a real paired capture.** Its two clips show different
  swings in different rooms — it came from the Phase 6 *networking* test and holds whatever
  three files were on the phone. Worth knowing before trusting anything measured on it. It also
  demonstrates the limitation exactly: the tempo cross-check passed (1.89 vs 1.54, gap 0.19
  against a 0.35 threshold), and the alignment reported `full` on two different swings. A τ warp
  makes any two swings look aligned; the video is not evidence they are the same swing. The real
  end-to-end check used the `aaron-1` triple, assembled through `SwingBundleStore` as
  `2026-08-07-aaron1` (≈380 MB of copied video — delete it whenever).

**Noticed, not fixed** — and now a named ROADMAP item: **the tempo checkpoint is untrustworthy
on real footage.** `_motion_start` walks back from the top for a quiet stretch, and a golfer who
pauses at the top hands it one immediately, so the boundary collapses onto the top: `aaron-1`
reads a backswing of **0.43 downswings**, which is not physically possible. `analyze_swing`
scores it anyway and the ranked tips lead with "work on tempo first, the downswing is rushing the
backswing" — a confidently wrong instruction. Phase 4 deliberately changes no scoring and instead
makes it impossible to render without seeing it (a note, printed *above* the tips). The fix
belongs in `_motion_start`, or in `evaluate_tempo` applying the floor `alignment.py` already has
as `MIN_PLAUSIBLE_TEMPO` — measured against GolfDB first, since that is where the band came from.

**Where I left off**: the full use case works end to end offline. Phase 5's remaining piece is
the background worker that triggers this on upload instead of a human running it.
**Blockers**: None.

---

## 2026-08-07 — Two views of one swing, aligned (M7 Phase 2)

**Duration**: ~1 session, implementation + first real bay footage
**What I did**: Built the alignment engine, then met real footage and had to build one more thing.

1. **`contracts/alignment.py` + `analysis/alignment.py`** — event-anchored piecewise-linear warp on
   a normalized swing-time axis τ (0 = motion start, 1 = top, 2 = impact), ADR-011's Option C
   standalone. Pure, stdlib + pydantic. `AlignmentQuality` (`full` / `top_impact` / `impact_only` /
   `unaligned`) carries outward how much of the swing was actually anchored.
2. **`scripts/align_swings.py`** — text report with no video needed; `--out` renders the
   side-by-side MP4. Both clips **stream** (the warp is monotone, so the follower only moves
   forward), reusing `draw_skeleton`/`annotate_frame` unchanged.
3. **Multi-swing selection**, which was not in the plan — `phases.candidate_downswings()` (a pure
   addition; `segment_phases` untouched), `--list-swings`, `--window START:END`.
4. **`analyze_swing.py` now derives its instants from `anchors_from_phases`** instead of its own
   copy, so the frame the TOP banner is stamped on and the frame the warp pins to τ=1 cannot drift.
5. **ADR-015**, which also settles the `FrameBundle` question ADR-011's addendum left open: they
   stay two types. `FrameBundle` pairs frames within a millisecond tolerance and so presumes a
   clock; this tier has none.

**Verification**: 201 passed / 4 skipped, ruff clean, mypy clean (52 files, `--python-version 3.12`
per the standing numpy-stub issue). `aaron-swing-2` still reports **TOP @ 400 / IMPACT @ 423** after
the `_instants` refactor — the pinned baseline did not move. The real proof is visual, on the first
bay pair: at τ=1.00 both panels are stamped TOP OF BACKSWING (face-on 704, down-the-line 1551) and
at τ=2.00 both are stamped IMPACT (718 / 1574), with the club at the ball in both. Independently, by
eye, the ball leaves the mat between down-the-line frames 1570 and 1580.

**Key decisions / surprises**:
- **The practice-swing problem is the common case, not the tail**, and Aaron flagged it before the
  footage confirmed it. `_top_and_impact` takes the *earliest* major descent — right for the
  single-swing GolfDB corpus, wrong for a 41-second bay clip. Unaided, both views picked a **setup
  move**: face-on a 0.50 s "downswing" at 0.9 s, down-the-line one at 15.2 s. Downswing *duration*
  is what makes the real swing obvious in the listing (~0.23 s against 0.4-9.1 s for the decoys),
  so `--list-swings` prints it. Worth remembering: **N swings yield about N+1 descents**, because
  the hands coming back down to address is a descent like any other.
- **The tempo cross-check earned its place immediately, and not for the reason I wrote it.** It was
  meant to catch two clips locking onto different swings. What it actually caught on the first real
  pair was `motion_start` **collapsing onto the top** in *both* views — the golfer pauses at the
  top, and `_motion_start` walks back from the top looking for exactly such a quiet stretch. The
  backswing measured 0.43 and 0.04 downswings. `phases.py` reports `detected=True` and is not
  wrong to: from inside one clip nothing looks off. So alignment now refuses any anchor implying a
  backswing shorter than its downswing — no reference distribution needed to know that is not a
  golf swing.
- **A τ warp makes *any* two swings look aligned.** That is the feature, and it means a
  convincing-looking side-by-side is *not* evidence the two clips show the same physical swing.
  Here they do — both are the last swing in their clip and the ball departs in both — but the video
  cannot establish that by itself, and a results page must not imply otherwise.
- **Down-the-line generates far more spurious candidates than face-on** (7 vs 3): a bystander walks
  through frame, the phone gets lowered, and MediaPipe's single-person model tracks both. Down-the-
  line is filmed from the busy side of the bay.

**Noticed, not fixed**:
- ~~**The down-the-line top reads early**~~ — 23 frames of downswing against face-on's 14 for the
  same swing at the same frame rate. Attributed here to the lead wrist being the far, occluded arm
  from behind. **This was the wrong way round — see 2026-08-09 below.** Face-on was the outlier;
  down-the-line had it right all along.
- **The clips are 4K60 portrait, not the 1080p60 the M7 doc asks for**, because nobody told the
  phones otherwise. Pose runs at ~9-14 fps end-to-end at that resolution — tolerable, so the
  deferred pre-inference downscale stays deferred, but that is now a measured number rather than a
  guess.
- I wasted a chunk of this session on a self-inflicted harness bug: my throwaway extraction script
  walked parent directories looking for `pyproject.toml` from a path outside the repo, which never
  terminates, and I read the resulting 100%-CPU-zero-IO process as "4K pose is very slow" and
  started optimising a problem that did not exist. The tell was there and I misread it: zero bytes
  read over four seconds is not a slow decode, it is no decode.

---

## 2026-08-07 — Capture layer survives phone footage (M7 Phase 1)

**Duration**: ~1 session, implementation
**What I did**: The three things standing between the pipeline and a real iPhone clip.

1. **Killed the OOM.** `run_pose.py` held every decoded BGR frame. Both it and
   `analyze_swing.py --overlay` — same bug, and the one you actually point at phone footage —
   now stream. Pose and overlay are two passes over the file rather than one pass and a list.
2. **`camera_id`** on `Frame` and `FrameKeypoints`, ADR-011's Phase 1 seam, prescribed in July
   and never built. `run_pose.py --camera-id face_on`.
3. **Clip metadata persisted.** fps existed only inside `FileVideoSource` and died with the
   process. The keypoints JSON grew an envelope — `{"clip": {...}, "frames": [...]}` — carrying
   fps, width, height, decoded frame count and the source clip's sha256.

Touched `capture/source.py`, `capture/file.py`, `contracts/keypoints.py`, `pose/estimator.py`,
new `storage/keypoints_io.py`, `storage/manifest.py` (+`hash_file`), `run_pose.py`,
`analyze_swing.py`, the three `scripts/golfdb/` readers/writers, the spike probe's loader, and
+19 tests.

**Verification**: 184 passed / 4 skipped, ruff clean. The real proof is an identity, not a
judgement call: re-running pose on the committed sample produced all **656 × 33 × 4 landmark
values bit-identical** to the previous file, `TOP @ 400 / IMPACT @ 423` unmoved, and the spike's
pre-flight still exits 0. Peak RSS **971 MB → 233 MB** on that 480×854 clip (measured, not
estimated), and the new number no longer scales with clip length. Clip metadata came out exactly
as predicted: `fps=58.913` — not 60 — `480×854`, 656 frames, sha matching `Get-FileHash`. The
461-clip GolfDB cache loads untouched, pinned by a test that reads the real files when present.

**Key decisions / surprises**:
- **Dropped the pre-inference downscale**, one of the four items in the phase plan. The
  justification for it — "MediaPipe resizes internally, so 4K buys zero accuracy" — is right
  about the ceiling but skips that downscaling first makes it a *two*-step resample, and that
  difference has never been measured here. It is a throughput optimisation, not a correctness
  fix, and streaming is what actually makes 4K survivable. Revisit with Phase 0 footage and a
  real throughput number. Dropping it is also what let the sample come out bit-identical, which
  is a far stronger check on a refactor this wide than "the instants still look right".
- **The envelope had to be a format change, not a sidecar**, and that put the 461-clip cache in
  the blast radius. One sniffing loader (`load_keypoints`) absorbs it: a top-level array is
  legacy, an object is enveloped, and legacy files report `clip=None` — which is *true*, not a
  missing value. Nothing was migrated; nothing needs to be.
- **`analyze_swing.py` had the same OOM and nobody had noticed**, because it is behind
  `--overlay` and the only clip anyone ran it on is 2.7 MB. Worth remembering the pattern is
  copy-pasted, not unique. The one remaining `list(source.frames())` is in
  `scripts/golfdb/extract_pose.py` and is left alone deliberately: 160px clips, ~15 MB, and the
  estimator interface takes a Sequence.

**Noticed, not fixed**: `mypy src/golf_coach` now dies inside numpy's own `__init__.pyi` —
"Type statement is only supported in Python 3.12 and greater". Nothing to do with this work
(`--python-version 3.12` is clean across all 50 source files); the installed numpy's stubs have
outrun `python_version = "3.11"` in `pyproject.toml`. Bumping the mypy target is its own call.

---

## 2026-08-06 — Phone reaches the server from anywhere (M7 Phase 6)

**Duration**: ~1 hour, implementation
**What I did**: Made the upload server reachable from a phone on any network, which was the whole
point of Phases 3 & 5 and the one thing they stopped short of. **The bind did not change** —
uvicorn still listens on `127.0.0.1` and Tailscale proxies to it over real TLS. Wrote
**ADR-016**; touched `config.py`, `api/app.py`, `api/static/index.html`, `scripts/run_server.py`,
`tests/api/test_uploads.py` (+7 tests), and the M7 doc / ROADMAP / FLOW / READMEs.
**Verification**: full suite green (169 passed, 4 skipped), ruff and mypy clean. Live server on
`127.0.0.1:3000`: 401 with no token and with a wrong token, 200 with the header, 200 with `?t=`,
static page still reachable unauthenticated, and a real 2.6 MB `.mov` uploaded end-to-end through
the query-param token. `--host 0.0.0.0` with no token exits 2 instead of starting.

**Then verified for real, same day, from an actual iPhone.** Installed Tailscale 1.102.2, enabled
MagicDNS + HTTPS certificates, `tailscale serve --bg 3000` →
`https://desktop-snhi10c.taila73d7e.ts.net`. Over that URL: static page 200 with a trusted cert,
`/api` 401 without a token and 200 with either the header or `?t=`. A complete swing bundle landed
from the phone in 30 seconds — `IMG_2712.mov` (33.6 MB, face-on), `IMG_2746.mov` (11.1 MB,
down-the-line), `IMG_2739.jpeg` (4.5 MB, shot screen) — all three grouped into swing 1, status
`complete`, no warnings. The 33.6 MB face-on clip sits right in the 30–50 MB range estimated for
1080p60, which is the number the Cloudflare-vs-Tailscale decision turned on.

`netstat` during all of it showed exactly one listener, `127.0.0.1:3000` — and Tailscale connected
to it as a *loopback client* (`127.0.0.1:7629 → 127.0.0.1:3000`). That is the whole design in one
line of output: nothing was exposed, Tailscale reaches in from inside.

**Still outstanding: Funnel.** `tailscale funnel --bg 3000` (the guest-phone path) is untested, and
it's the one that actually needs the token — Serve was verified with the token on, but tailnet
membership was also protecting it. Deliberately not changed at the same time as Serve.
**Key decisions / surprises**:
- **The plan of record was wrong, and the reason was a person, not a protocol.** Phase 6 said bind
  to the tailnet IP because tailnet membership is the access control. That holds right up until
  the down-the-line phone belongs to whoever is at the bay — a helper can't join my tailnet. So:
  `tailscale serve` for my devices, `tailscale funnel` for guests, and because Funnel is public,
  a `GOLF_UPLOAD_TOKEN` gate on `/api/`. The token is what buys the guest case.
- **Serving via Tailscale is strictly better than binding the tailnet IP**, which is a bonus I
  wasn't looking for. Loopback bind stays loopback (the dangerous config becomes unreachable, not
  merely warned against), and TLS termination gives a secure context — so `getUserMedia` is
  available whenever the page wants to capture directly instead of via the camera roll.
- **Cloudflare Tunnel is out on a hard number**: free and Pro cap request bodies at 100 MB with an
  edge-side 413. A 1080p60 clip (~30–50 MB, measured against the 2.6 MB sample) usually fits; 4K
  doesn't, and the workaround is client-side chunking. Tailscale Funnel documents no body cap.
- **`create_app(token=...)` needed a sentinel.** `None` has to mean "no auth", so it can't also
  mean "look it up in settings" — without the sentinel, tests asserting the unauthenticated path
  would flip to 401 the moment someone put `GOLF_UPLOAD_TOKEN` in their `.env`. Defaulting to the
  sentinel also keeps the lookup fail-closed.
- **The guard is a route dependency, not middleware**, specifically so it resolves before the
  handler reaches `request.stream()`. A rejected upload writes zero bytes to `.incoming/`; there's
  a test for exactly that.
- **`fetch` cannot report upload progress** — only `XMLHttpRequest.upload.onprogress` can. On
  cellular a 50 MB clip left the page showing a motionless "Uploading…", which reads as hung.
  Swapped to XHR with a progress bar. Nothing to do with networking, everything to do with the
  page being usable on a phone.
- **The 8080 → 3000 port note below is now fixed in code, not just remembered.** `api_port`
  defaults to 3000 with the excluded-range explanation inline.
- **`httpx2` in `pyproject.toml` is not a typo** — I assumed it was and was wrong. starlette 1.4.1
  depends on `httpx2` 2.9.1, which is installed and is what `TestClient` uses. Left alone.

---

## 2026-08-06 — Swing bundle store + phone upload server (M7 Phases 3 & 5, trimmed)

**Duration**: ~2 hours, implementation
**What I did**: Built the storage and API layers that let a phone browser upload a swing bundle
(face-on video, down-the-line video, shot-tracker photo) and have it land on the desktop,
correctly grouped — trimmed from the full M7 Phase 3 + Phase 5 design in
[docs/M7_TWO_PHONE_CAPTURE.md](docs/M7_TWO_PHONE_CAPTURE.md) at the user's explicit direction:
no auto-triggered analysis (Phase 4 + the Phase 5 worker), no Tailscale (Phase 6) — both are
separate future work. New: `storage/manifest.py`, `storage/bundle_store.py`, `api/app.py`,
`api/static/index.html`, `scripts/run_server.py`, plus `tests/storage/` and `tests/api/` (21
new tests, all passing; suite stays green with only the base + `api` + `dev` extras).
**Verification**: `pytest -q` green (21 new + all existing). Manually exercised the running
server on localhost: all three roles landing in one swing in every arrival order, dedupe on a
repeated upload, the documented "different bytes reopens a new swing" behavior and its
`swing_id` repair path, and confirmed via `netstat` that the bind stays on `127.0.0.1` (never
`0.0.0.0`).
**Key decisions / surprises**:
- **Swing identity assigned by the store, exactly as ADR-011's addendum and the M7 doc call
  for** — a role-only upload (`face_on` / `down_the_line` / `shot_screen`) slots into the
  newest swing missing that role. Pinned down, not silently accepted: if two swings are
  simultaneously missing the same role, "newest wins" can misattribute an out-of-order upload.
  A test (`test_newest_wins_when_two_swings_are_missing_the_same_role`) locks the behavior down
  so a future change can't alter it unnoticed; the escape hatch is an explicit `swing_id` query
  param that overwrites a role slot by hand.
- **`status()` is derived, not persisted, and deliberately not named `pending`/`ready`/
  `analyzed`.** Those words imply something consumes `ready` to fire analysis, which nothing
  does yet. `collecting`/`complete` says only what's actually true; `analyzed_at` can be added
  later as a pure addition with no manifest migration.
- **`StaticFiles(..., html=True)` only serves a directory-relative `index.html`** — the upload
  page had to be named `index.html`, not `upload.html`, or `GET /` 404s. Caught during manual
  verification, not by the test suite (the API tests never hit `/`).
- **Port 8080 — this repo's own configured `api_port` — is inside a Windows TCP port exclusion
  range (`netsh interface ipv4 show excludedportrange`, 8069–8168 here) on this machine.** Binding
  failed with `WinError 10013` until moved to port 3000 for the manual check. Environment-specific
  and not a code issue, but worth remembering if `scripts/run_server.py` ever refuses to bind on
  the default port on this box.
- Also fixed doc drift while in the area: `README.md` still described `storage/` as SQLite and
  `api/` as unused/declared-only. Left `docs/decisions/008-project-structure.md` alone —
  ADRs in this repo get addenda, not rewrites (see ADR-011's pattern), and 008 is a decision
  record, not living documentation.

---

## 2026-08-05 — Investigation: can two phones at a sim feed this thing? (M7 planned)

**Duration**: ~1 hour, investigation and planning only
**What I did**: Answered a use-case question — *record one swing at an indoor sim on two iPhones
(face-on + down-the-line), photograph the HD Golf screen, send all three in, get analysis back* —
and turned the answer into a seven-phase plan at
[docs/M7_TWO_PHONE_CAPTURE.md](docs/M7_TWO_PHONE_CAPTURE.md). No code written. ADR-011 has a
2026-08-05 addendum; ROADMAP has an M7 section and two corrected `Future` entries.
**Verification**: docs only — nothing under `src/`, `scripts/` or `tests/` touched, so the suite is
unchanged.
**Key decisions / surprises**:
- **The answer was "no", and the missing parts are the hard parts.** Everything downstream of *"a
  video file is already on the desktop"* works. Everything upstream is absent: no upload path, no
  `camera_id`, no structure holding two views of one swing, no session registry (`storage/` is a
  4-line docstring), no host (`api/` is a 6-line docstring with fastapi already declared and unused).
  Call it 60% built — but the remaining 40% is ingestion, identity, and alignment.
- **`run_pose.py:36` will OOM on the first real phone clip.** `frames = list(source.frames())`
  materializes every decoded BGR frame; a 10-second 4K60 iPhone clip is 600 × ~24.9 MB ≈ **15 GB**.
  It has survived purely because the one committed sample is small. Found while tracing whether the
  desktop could do the compute — the answer is yes, comfortably, but not through that line.
- **3D fusion is off the table here, and that is a construction fact, not a scheduling one.**
  ADR-011 reads as though multi-camera work leads to triangulation. It does — for the fixed ELP rig.
  Two phones held by two people have no stable extrinsics to solve for, so spine angle, hip rotation
  and X-factor stay fixed-rig-only. Wrote the addendum specifically so the next person reading
  ADR-011 doesn't spend a day trying to calibrate hand-held phones.
- **But ADR-011's Option C survives on its own, and is the better tool here anyway.** Aligning the
  two clips on the phase instants each already produces (top, impact — median 2 and 1 frames against
  461 GolfDB clips) needs no shared clock and no calibration. It also absorbs the thing host
  timestamps would choke on: two consumer phones aren't configured alike, and **iPhone slo-mo**
  stores 120/240fps with a stretched playback rate, so `CAP_PROP_FPS` may not describe real time.
- **Swing identity has to be assigned server-side.** The tempting design has each phone label its
  upload with a swing number. Two people, two phones, mid-session — they will drift within about
  three swings. Each phone declares only its *role*; the store does the matching.
- **Split into seven one-commit phases with a planning prompt each**, at Aaron's request, after the
  first draft came back as one large plan. Alignment alone is its own problem with its own failure
  modes and deserves its own planning pass. The prompts exist so each phase is planned in a fresh
  session rather than at the tail of a stretched context.

---

## 2026-08-04 — M3: shot data arrives, read off a photo of the simulator screen

**Duration**: ~3 hours
**What I did**: Unblocked M3 without buying anything. The HD Golf simulator has no data export
of any kind, so shot metrics are now read off photographs of its `SHOT DATA` screen: rectify the
screen out of the photo, recover its orientation, OCR it, parse the tile grid geometrically,
cross-check the numbers against the screen's own arithmetic, and cache the result. Plus
`CompositeShotDataSource`, so screen captures, mock shots, and a future R10 mix behind one port.
Decision: [ADR-014](docs/decisions/014-screen-capture-shot-ingestion.md); ADR-004 has an addendum.
**Verification**: `pytest` → **141 passed, 4 skipped** (was 96); `ruff` clean; `mypy` clean on
`contracts` + `launch_monitor`. Rectification verified by eye on both reference photos. The 4 skips
are the OCR integration tests — `paddleocr` isn't installed in this venv yet.
**Key decisions / surprises**:
- **Chose local OCR over vision-model parsing, knowing it is the riskier build.** The photos are
  the hard case: shot at an angle, ceiling glare, the room reflected across the tiles. What made
  it worth it is that the choice is *cheap to reverse* — the engine sits behind a `TextRecognizer`
  Protocol, so if it proves too fragile in practice, a vision adapter is one new class and parsing,
  validation, caching and the source are untouched.
- **Quad detection failed on both real photos, and the cause was scale, not thresholds.** The
  morphology kernel that bridges gaps in the bezel edge is a fixed pixel size; on a 5712px phone
  photo it is far too small to close anything, so the screen contour came back shattered every
  time. Running detection on a normalized 1000px copy and scaling the corners back fixed both
  photos immediately. I had been about to start tuning Canny thresholds — that would have been
  hours down the wrong hole.
- **The "upside-down" photo was never upside-down to the code.** IMG_2739 displays inverted in
  viewers that ignore EXIF, and I wrote a test asserting the rotation search would correct it.
  `cv2.imread` applies the EXIF orientation tag, so it loads upright and the assertion was wrong.
  The rotation search still earns its place — EXIF does not survive re-encodes, screenshots, or
  video frames — but it is now tested on synthetic images where I control the orientation, and the
  integration test asserts what actually matters (the screen got cropped, the labels are legible).
- **The screen validates its own parse.** It prints redundant metrics: `smash == ball / club` and
  `total == carry + bounce & roll`. Both hold *exactly* on both reference photos, which makes a
  mismatch evidence about the parse rather than about the shot. That is the whole answer to "how
  do I know OCR didn't drop a digit" — a 128.1 read as 28.1 is a well-formed number that breaks
  the arithmetic.
- **Fidelity is not plausibility, and conflating them would have been a bug.** IMG_2739 shows a
  159.5 mph club speed and a 0.89 smash factor — nonsense as a golf shot, but the screen's own
  arithmetic checks out, so the parse is correct and passes clean. Flagging it would train me to
  ignore flags. Range checks are deliberately wide enough to admit it; they exist to catch a
  dropped digit, not an unusual swing.
- **No fixed ROIs anywhere.** Tiles are located from the labels' own geometry — find the labels,
  group into rows, derive each column from its neighbours, read what falls in the cell below. A
  pixel-coordinate template would break the first time I stood somewhere else to take the photo.
  Device knowledge (labels, target fields, sign rules) is `profiles.json`, so a second launch
  monitor is a data change.
- **Signs are the most dangerous part of the pipeline.** `1.6 ° O>I` must become `-1.6`, `Closed`
  negative, `L` negative, and `---` must become `None` and never `0`. A flipped sign is invisible
  downstream. A number with no direction word is stored as a magnitude *with a warning*, never
  with a guess.
**Where I left off**: Ingestion works end-to-end on the two reference photos. Next: install the
`ocr` extra, run `import_shot_screens.py` over a full range session, and tune preprocessing on
photos that were not the two I developed against. Then the MCP server itself, which is now the
only thing left in M3.
**Blockers**: None. `paddleocr` is not installed yet, so the integration tests skip.

---

## 2026-08-04 — M5: the feedback learns to prioritise, and the panel fails to widen

**Duration**: ~3 hours
**What I did**: Turned three equal-weight readouts into ranked coaching. Wired the reference
distributions — built, tested and *never called* since M4-REF — onto every `CheckpointScore`, ranked
tips by what actually discriminates within their group, added a headline, and stopped unmeasurable
checkpoints vanishing without a word. Then tried to widen the panel from 3 to 5 using the
arm-parallel metrics already sitting in `golfdb_v1.json`, and the gate said no. Design doc:
[docs/M5_COACHING_FEEDBACK.md](docs/M5_COACHING_FEEDBACK.md); ADR-010 has a new addendum.
**Verification**: `pytest` → **96 passed** (was 75); `ruff` clean; `mypy` clean on `analysis` +
`feedback`. `analyze_swing.py` re-run on both clips. `tune_arm_parallel.py` scored 461 face-on clips.
**Key decisions / surprises**:
- **The motivating bug was a 100/100 swing.** `golf_swing-aaron-1` passes all three checkpoints and
  scores perfect — while its head sway sits higher than **83% of 458 tour swings**. Nothing we stored
  could say so, because `_score_within_range` returns exactly 1.0 for *every* pass. Three passes,
  three identical scores, no ordering derivable. That is the whole argument for percentiles, and it
  is a better one than "show the golfer a stat".
- **The percentile saturates exactly where the band ends.** The bands *are* the reference p10/p90
  (ADR-012) and `percentile_of` clamps there, so every failing checkpoint reports 90 whether it
  missed by a hair or by triple. I had planned to rank failures by percentile; that would have
  flattened them all into one bucket. Ranking needs **two** signals: `score` for failures (it decays
  in band-widths and does separate them), percentile for passes (score cannot). Severity stays on
  `score` for the same reason.
- **Percentiles query the same stratum the band was cut from**, not the most specific match.
  `tempo_ratio` face-on p90 is 5.00 against the all-view 4.71 — mixing them would let a swing read
  "inside the band" and "past the 90th percentile" at once. There is now a test that blinds the
  evaluators to the distributions and asserts `score`/`passed` do not move, so ADR-010 §2's firewall
  is executable rather than a comment.
- **The panel did not widen, and that is the result.** GolfDB's mid-backswing/mid-downswing are
  *lead arm parallel to the ground* — a genuine body pose, unlike `toe_up`, which is a club event
  gated on M2. Built `tune_arm_parallel.py` and measured before writing either checkpoint.
  **`prior_frac` — which reads no pose signal at all, just "assume the tour-median fraction" — beats
  every pose rule on every column, for both events.** frac_err 0.043 against argmin's 0.059
  (mid_backswing) and 0.038 against cross's 0.100 (mid_downswing). A checkpoint built on our
  detection would be worse than a constant. First time `tune_address.py`'s `prior_*` bar has actually
  rejected something.
- **Reported `frac_err`, not frames.** The error in the metric the checkpoint would *report*, against
  the p10–p90 width of the band it would be compared against. A 4-frame error sounds excellent until
  you notice `mid_backswing_frac`'s entire tour spread is ~4 frames on a real-time clip.
- **A hypothesis I checked and had wrong.** I assumed the tight spread was frame quantization — an
  8-frame downswing can only produce a handful of fractions. It is not: spread is **0.148 real-time
  vs 0.149 slow-motion** for mid_backswing, on 29- vs 108-frame backswings. Four times the temporal
  resolution does not narrow it. The variation is real; our detector cannot resolve it.
- **Unscored checkpoints are named now.** Tempo drops on ~14% of clips (ADR-013) and `overall_score`
  is a mean over survivors, so a 2-checkpoint and a 3-checkpoint swing printed the same number.
  `SwingResult.unscored` carries the names. The score is deliberately *not* penalised — dropping the
  checkpoint is right, dropping it silently was not.
**Where I left off**: The panel stays at three checkpoints, now ranked and quantified against the
tour population. `tune_arm_parallel.py` is kept runnable alongside the rejected address signals. The
two things that would change its verdict are both hardware/milestone gated: M2 club detection gives
`toe_up` directly, and the down-the-line view (ADR-011) sees arm-parallel far more cleanly than a
160×160 face-on crop.
**Blockers**: None.
**Notes**: `unscored` carries names but not *reasons* — "tempo could not be measured" does not say
"because the address boundary was estimated". A reason means the evaluators returning one instead of
`None`, which touches every return site; deferred deliberately. Also: the coaching prose still lives
in `mechanics.py`. Fine at three checkpoints, extract a `feedback/catalogue.py` the moment that
changes.

---

## 2026-08-02 — M4-REF Phase B6: address detection, and the posture bug hiding behind it

**Duration**: ~3 hours
**What I did**: Closed the last open M4-REF item. Replaced the address rule's one fps-dependent
constant with a clip-relative one, bounded its failure path, made it report when it fails, and —
the part that turned out to matter more — stopped the posture checkpoints depending on the address
boundary being right. New `scripts/golfdb/tune_address.py` is the runnable address report that
never existed; ADR-013 records the decision; M4_POSE_BAKEOFF Phase B6 and
`docs/M4_ADDRESS_DETECTION.md` carry the numbers and the flow.
**Verification**: `pytest` → **75 passed** (was 67); `ruff` clean. `tune_address.py` reproduces
median **7.0** / mean 22.9 / 40% over 10 / med_norm 0.122 / PCE 15.8% against the old rule's
9.0 / 27.2 / 46% / 0.133 / 14.3%. `bakeoff.py --merge` confirms top and impact **unchanged** at 2
and 1. Full band re-derivation chain re-run; `analyze_swing.py` re-run on both own clips.
**Key decisions / surprises**:
- **Splitting the corpus by capture speed is what cracked it.** The famous "median 9 frames" is
  really 5 real-time and **17 slow-motion**, and the corpus is 47% slow-motion — so the headline was
  substantially reporting the corpus mix. That immediately implicated `_MOTION_STALL_FRAMES = 4`, a
  frame count sitting in a corpus whose downswing runs 8 frames real-time and 30 slow-motion. Now
  **0.25 of the clip's own downswing duration**. Median 9 → 8 on its own.
- **The frame-0 fallback was a second, separate bug.** It fired on 11% of clips at a median of 31
  frames, because frame 0 is not a neutral answer — GolfDB clips carry a median 59 frames of
  pre-roll. Bounded it to `top - 3.5 x downswing` and marked the segment `detected=False`. Median
  → **7**, mean 27.2 → 22.9.
- **Six alternative signals, all worse.** Setup-ball displacement, noise-floor/MAD, ramp
  back-extrapolation, torso energy, shoulder rotation, directional persistence. Upper-body motion
  energy *ties* the lead wrist for eight extra landmarks. Kept every one of them runnable in
  `tune_address.py` rather than deleting the evidence — at 160x160 the torso simply does not move
  enough during a takeaway.
- **The sobering number**: a rule using **no pose signal at all** (`top - 3.5 x downswing`) scores
  median 11. We score 7. That is the honest size of what the lead wrist contributes, and it is now
  a permanent row in the harness so no future candidate gets graded against a soft baseline.
- **The real find was in posture, not in the boundary.** `head_sway` averaged the head across
  `[0, motion_start]` — which starts at frame 0, i.e. the golfer walking in. On
  `golf_swing-aaron-1` that read **1.21 shoulder-widths** of sway, nearly 3x the tour p90, when the
  head sits +0.41 off setup early in the clip and only settles by ~frame 400. Sampling a short
  window *ending at* the boundary gives **0.36** — a hard fail becomes a comfortable pass, and the
  clip goes to 100/100. Same shape as the top-detection defect ADR-012 found: a plausible number
  measuring the wrong thing.
- **One predicted win did not happen.** I expected the shoulder-width ruler (it divides *both*
  posture checkpoints) to improve with the window. It did not — 10% of clips off by >10% before,
  11% after. That error is pose noise, not window content. Recorded as a miss rather than quietly
  dropped.
- **Tempo now drops rather than guesses on 14% of clips.** The fallback boundary is derived from an
  assumed tempo ratio, so scoring it would hand the assumption back as an observation. ADR-010 §2.
  The most contestable call here, and the one to revisit if it annoys in practice.
- **Metric definitions v2 → v3.** Changing *where* a metric samples changes the metric, so the bands
  were re-derived rather than assumed: `head_sway_norm` 0.42 → **0.43**, `finish_balance_norm`
  0.28 → **0.29**. Small, which is exactly the drift ADR-012 §4 exists to catch.
**Where I left off**: M4-REF exit criteria met. Address is still the weakest instant (7 frames, 40%
over 10) but it is measured, improved by a different rule rather than a better constant, and can no
longer fail silently. Two follow-ups on the ROADMAP: switch the tracked headline from pooled frame
median to `med_norm` + the slow-mo split, and decide whether the remaining headroom (SwingNet 31.7%
PCE vs our 15.8%) justifies a learned model — which needs its own ADR, since ADR-008 keeps the
analysis core stdlib-only.
**Blockers**: None.
**Notes**: `smoothing.py`'s 5-frame window is now the *only* absolute left in the address path, and
it is still the value tuned by eye on ~60fps phone clips. Changing it globally moves top and impact
too, so it stays its own item — but it is the obvious next thing to question. Also worth knowing:
the synthetic test fixtures cannot prove the fps-invariance claim, because their setup is perfectly
still and both the old and new rules find it. The corpus harness is the evidence there; the unit
tests only guard the contract.

---

## 2026-08-02 — M4-REF Phase B: estimator bake-off settled, both provisional bands recalibrated

**Duration**: ~4 hours (mostly background extraction)
**What I did**: Finished the GolfDB reference work. Locked `_MAJOR_RISE_FRACTION` against the full
corpus, ran the four-variant pose bake-off to completion, and replaced the last two
`PROVISIONAL / UNCALIBRATED` bands in `ranges.json` with tour-derived ones. Every number is in
`docs/M4_POSE_BAKEOFF.md` + `docs/pose_bakeoff_v1.json`; ADR-002 has an addendum recording the
variant decision.
**Verification**: `pytest` → **67 passed** on the base install (was 66; one rewritten, one new);
`ruff` clean; `mypy` clean on `analysis` + `feedback`. End-to-end re-run of `scripts/analyze_swing.py`
on both clips against the new bands.
**Key decisions / surprises**:
- **`_MAJOR_RISE_FRACTION` 0.50 → 0.80**, picked from the **plateau centre** (0.75–0.85 are flat at
  ~10.5 mean top error) rather than the argmin. The old 0.50 was fit on 97 clips; on all 461 it costs
  8 frames of mean error. Median top error is now 2 frames, impact 1 — against 21 and 35 for the
  `argmin` rule this replaced. No effect at all on my own clips: both have a single dominant rising
  run, so every fraction from 0.40–0.95 gives identical instants.
- **Keep MediaPipe lite.** Twelve paired McNemar tests across lite/full/heavy — **not one reaches
  p < 0.05**. full's +9.1pp at the top looked real at n=120 (p=0.099) and **evaporated to +1.9pp
  (p=0.494) at n=461**, with lite ahead on mean PCE. Same overfitting-to-sample-size trap as the 0.50
  constant, caught this time *before* it got baked into a band. heavy costs 4.4x and buys nothing,
  which retires ADR-002's one-clip judgement with an actual number.
- **RTMPose rejected by 24.7pp**, and the "3 fps, run it overnight or drop it" dilemma was false:
  rtmlib's `Body` re-runs a YOLOX detector every frame, which is **35x** the pose model. Skipping it
  (the clips are already bbox crops) gave **118 fps** — faster than lite — so the whole question cost
  5 minutes instead of 3 hours. It still lost badly: same trajectory (r=0.948 with lite) but 1.5–4.3x
  noisier per landmark, worst at the hip.
- **Both eyeballed bands were loose; `finish_balance` by more than 2x.** head_sway 0.5 → **0.42**,
  finish_balance 0.6 → **0.28** (p90 of 458 face-on swings from 122 tour golfers, measured at
  GolfDB's *annotated* instants, never at our own segmentation). `aaron-swing-2` drops 100 → 78: its
  0.47 finish drift was always there and sits above the 90th percentile of tour finishes.
- **The tightened band exposed a vacuous test.** `test_finish_balance_survives_a_single_bad_frame`
  asserted `passed is True` while the metric read 0.48 — it had only ever passed because 0.48 < 0.6,
  and never demonstrated the p90 property at all. Root cause: `smooth_keypoints` is a 5-frame moving
  average, so **one bad frame becomes five**, and p90 can only reject it beyond ~50 follow-through
  frames. The fixture hardcoded **8**; the real corpus is p10 50 / p50 89. The metric is fine (only
  8% of clips fall in the bad regime) — the fixture was testing a regime real footage never reaches.
  A median center was tried and is *worse*; the center was never the problem.
- **Calibrated the address constants too** — `_MOTION_QUIET_FRAC` / `_MOTION_STALL_FRAMES` were the
  last things in `phases.py` still set from one clip. Swept over all 461: **0.08/3 → 0.05/4**, median
  address error **13 → 9 frames**. Took the plateau centre again, *not* the grid minimum (8.0 at
  0.03/2), which sits on the grid edge and falls apart one step in any direction. My own clips barely
  move (address 322 → 319, tempo 3.39 → 3.52, no verdict change).
- **Verified the corpus by eye, finally.** Added `scripts/golfdb/spot_check.py` — 5 clips x 4
  ground-truth instants with production skeletons. Every other check this milestone was a statistic,
  and I'd cut two committed bands from 458 clips without once looking at a pose. Torso/hips/
  shoulders/legs correct in all 20 tiles, which is what the bands actually rest on.
**Where I left off**: M4-REF is complete. `ranges.json` has no uncalibrated rows left, all three
bands cite an inspectable distribution, and nothing in `phases.py` is tuned on a single clip any
more. The one open item is **improving address detection** (median 9 frames, 46% of clips over 10) —
now calibrated as far as the current rule goes, so further gains need a *different* rule, not a
better constant. Intrinsically hard: SwingNet manages only 31.7% PCE there.
**Blockers**: None.
**Notes**: ~10% of corpus clips contain a *practice swing* before the annotated one, which no
estimator fixes — those failures have **higher** tracking confidence than the successes (0.85 vs
0.82), so they are structural, not pose quality. Nothing under `data/reference/` is committed; the
only new committed data files are `golfdb_v1.json` and `pose_bakeoff_v1.json`.

---

## 2026-08-01 — Hardened motion-start detection (velocity-anchored) → tempo is now honest

**Duration**: ~1.5 hours
**What I did**: Fixed the last-flagged segmentation error from the 07-16 session — motion-start
landing mid-takeaway and collapsing tempo. Replaced the height-crossing rule in
`analysis/phases.py` with a **velocity-anchored** one: new `_motion_start` measures **2D lead-wrist
speed** (new `_wrist_speed`; refactored `_lead_wrist_y` → `_lead_wrist_xy` since we now need `x`
too) and, walking back from the top, takes the takeaway to begin just after the last sustained
*quiet* stretch (`_MOTION_QUIET_FRAC` of peak speed for `_MOTION_STALL_FRAMES` frames). Extended the
synthetic fixture (`tests/analysis/conftest.py` `make_swing`) with a near-horizontal `takeaway_x`
preamble — the sideways move a wrist-*height* rule can't see — and added two tests (a motion-start
robustness test in `test_phases.py`, a tempo-not-collapsed test in `test_checkpoints.py`).
**Verification**: `pytest` → **41 passed** on the base install (was 39); `ruff` clean; `mypy
src/golf_coach/analysis src/golf_coach/feedback` clean. Then the decisive real-clip check on face-on
`aaron-swing-2`: dumped the smoothed lead-wrist speed profile and re-ran `scripts/analyze_swing.py
--overlay`. **Extracted and eyeballed the annotated frames** — ADDRESS marker @322 lands with the
hands *still at the ball* (frame 290 setup and 322 look identical; motion only appears by 345),
where it used to land mid-backswing.
**Key decisions / surprises**:
- **The clip's tempo is genuinely ~1.5:1, not a bug.** The speed profile settles this: the lead
  wrist is provably *still* (< ~3% of peak, flat `y`) from frame ~250 to ~318, then onsets sharply
  (12% → 22% → 54% …) at ~320. So motion-start @322 is *correct*; this golfer just swings quick.
  The move 1.05 → **1.53:1** is the backswing finally being counted from the true onset, not the
  number magically becoming 3:1. Honest > flattering.
- **Height can't see the takeaway; speed can.** The early takeaway is near-horizontal — the lead
  wrist moves back at roughly constant height — so the old "wrist rose past address height" rule
  skipped it. Using 2D speed is what fixed it; anchoring on wrist-`y` alone never could.
- **Tuned `_MOTION_QUIET_FRAC` to 0.08** against the real speed profile: setup waggle jitter tops
  out ~3% of peak, the takeaway jumps past ~12%, so 8% sits cleanly between. TOP @383 / IMPACT @423
  unchanged throughout (only motion-start moved).
**Where I left off**: Tempo is now trustworthy end-to-end (correct instant, honest value, visual
proof). The three pose-only checkpoints are solid. Next is either the **Hardware Re-Validation
Gate** (recalibrate the provisional sway/balance bands, validate instants against club/R10 timing)
when hardware lands, or non-pose work (M1.5 club spike / full-M4 outcome axis).
**Blockers**: None — pose-only, no hardware.
**Notes**: `_MOTION_QUIET_FRAC` is tuned on one clip; re-check when higher-fps / global-shutter
footage arrives (folded into the existing `HARDWARE-REVALIDATE:` note on smoothing/instants). The
annotated overlay + extracted frames live under gitignored `data/processed/`.

---

## 2026-07-16 — M4-PoC+: hardened Fundamentals panel (smoothing, 2 checkpoints, verification overlay)

**Duration**: ~2 hours
**What I did**: Made the pose analysis *trustworthy without hardware*, per the approved plan
([docs/M4_FUNDAMENTALS_PANEL.md](docs/M4_FUNDAMENTALS_PANEL.md)). New `analysis/smoothing.py`
(visibility-weighted centered moving average, stdlib only), applied once in `engine.analyze_swing`
so phase segmentation + checkpoints read a denoised signal. Added two pose-only mechanics
checkpoints in `checkpoints/mechanics.py` — `evaluate_head_sway` (lateral nose travel to impact)
and `evaluate_finish_balance` (post-impact hip-center settle), both shoulder-width normalized;
extracted the shared `_score_within_range`/geometry helpers. Seeded two **PROVISIONAL /
UNCALIBRATED** benchmark rows (`head_sway_norm`, `finish_balance_norm`) — data-only, ADR-010
addendum. New `scripts/analyze_swing.py`: prints a scores+tips report AND (with `--overlay`)
renders an annotated clip stamping ADDRESS/TOP/IMPACT + a score HUD, via a new
`pose/overlay.py:annotate_frame`. Wrote the feature doc (two mermaid diagrams, reliable-vs-deferred
table, SOLID/GRASP notes), a ROADMAP **M4-PoC+** section + a new **Hardware Re-Validation Gate**,
and the ADR-010 addendum.
**Verification**: `pytest` → **39 passed** on the base install (was 27); `ruff` clean; `mypy
src/golf_coach/analysis src/golf_coach/feedback` clean. Real-clip run on face-on `aaron-swing-2`:
produced a 3-checkpoint SwingResult (tempo MISS 1.05:1, head_sway PASS, finish_balance PASS) and a
656-frame annotated overlay; extracted and eyeballed the marked frames.
**Key decisions / surprises**:
- **Dropped the plan's velocity-based impact detector.** Implementing it revealed it *miscalibrates*
  tempo: peak hand speed precedes ball contact, so it shortens the downswing and inflates the ratio,
  breaking the Tour-Tempo calibration (and the synthetic tests). Reverted impact to the correct
  return-to-address-height rule; **smoothing is the real robustness win**, not a new impact rule.
- **The overlay earned its keep immediately** — it localized the remaining tempo error. TOP @ 383 and
  IMPACT @ 423 are visually correct, but **motion-start @ 341 lands mid-takeaway**, truncating the
  backswing to ~42 frames → the ~1:1 reading. So tempo is low because of *motion-start*, not top/impact.
- Provisional bands are labelled as such (greppable) and gated in the Hardware Re-Validation Gate;
  `HARDWARE-REVALIDATE:` comments mark every spot to revisit when cameras/R10 arrive.
**Where I left off**: M4-PoC+ done & verified. Next segmentation task is **hardening motion-start**
(anchor the takeaway on the last stable-setup frame, not the last frame at/above address height) —
that's what will finally make tempo believable. Sway/balance await calibration data.
**Blockers**: None — pose-only, no hardware.
**Notes**: Tip/HUD/banner text stays ASCII (plain hyphen) for the Windows console. The `pytest`
run needs `--basetemp`/`-p no:cacheprovider` redirected to the scratchpad under the sandbox.

---

## 2026-07-03 — M4-PoC implemented: pose-only Fundamentals analysis spine

**Duration**: ~2 hours
**What I did**: Implemented the whole [M4-PoC plan](docs/archive/M4_POC_PLAN.md) — the pose-only
Fundamentals analysis spine, end-to-end. New: `contracts/intent.py` (`PracticeGoal` + enums),
the benchmark store (`analysis/benchmarks/` — `ranges.json` seeded with Tour Tempo ~3:1 +
`resolve_range` with most-specific→`all` fallback), `analysis/phases.py` (lead-wrist
segmentation → 6 phases), `analysis/checkpoints/mechanics.py` (`evaluate_tempo`),
`analysis/scoring.py` (`FundamentalsPolicy` + `policy_for`). Extended `SwingResult` with
`intent`/`mechanics_score`/`outcome_score`, wired `analyze_swing`, and implemented
`feedback/rules.py` (`build_feedback`). Added the synthetic-swing fixture + 7 test modules (23
new tests). Also **added the missing runtime sequence diagram to `docs/archive/M4_POC_PLAN.md`** (the
plan only had a data-flow ASCII block), wrote the milestone flow doc
**[docs/archive/M4_ANALYSIS_POC.md](docs/archive/M4_ANALYSIS_POC.md)** (mermaid data + sequence, GRASP
callouts, files, findings), an **ADR-010 addendum** (JSON-not-YAML), and checked off the
ROADMAP M4-PoC boxes.
**Verification**: `pytest` → **27 passed** on the **base install** (no vision/ML extras);
`ruff check` clean; `mypy src/golf_coach/analysis src/golf_coach/feedback` clean. Real-clip
eyeball: fed the face-on `data/processed/aaron-swing-2.keypoints.json` through
`analyze_swing → build_feedback` → a full `SwingResult` + tempo tip. Exit criterion met.
**Key decisions / surprises**:
- **Phase segmentation had to anchor on the top of the backswing, not "first motion."** First
  pass forward-scanned for the first wrist movement; on the real clip the golfer waggles/sets
  up for ~5.8 s, so that swallowed the setup into the backswing → a nonsense **9.6:1** tempo.
  Re-anchored on the global wrist-`y` minimum (the top) and walked backward to the start of the
  final rise → address phase collapses correctly (0–342) and tempo reads a believable **~1.1:1**
  (amateur-quick; the "too quick" tip is the right cue). Synthetic tests stayed green.
- **Benchmark store ships JSON, not YAML** to keep the analysis core stdlib-only (ADR-010
  addendum). One seeded row (Tour Tempo).
- Kept the guardrails: no `merge.py`, no outcome checkpoints, one policy — all named seams
  (`outcome=[]`, `outcome_score=None`, absent `checkpoints/outcome.py`).
**Where I left off**: M4-PoC is **done and verified**. The spine is proven; the thing to harden
next (full M4) is segmentation *accuracy* — landmark smoothing, validating top/impact against
the overlay video, then more mechanics checkpoints (posture/hip rotation, several needing
down-the-line / synced 3D per ADR-011) and the outcome axis + other scoring policies.
**Blockers**: None — pose-only, no hardware.
**Notes**: Runtime tip text uses a plain hyphen (not em-dash) so the Windows-console eyeball
doesn't mojibake. Real-clip number is one heuristic on one clip — good enough to prove the
spine, not yet a trustworthy tempo measurement.

---

## 2026-07-02 (cont. 2) — M4-PoC implementation plan written up (no code yet)

**Duration**: ~0.5 hour
**What I did**: Reviewed the M4-PoC scope (ROADMAP + ADR-009/010), re-verified the current
contracts/stubs against the plan, and documented the agreed implementation plan into the repo
as **[docs/archive/M4_POC_PLAN.md](docs/archive/M4_POC_PLAN.md)** so it survives outside the local scratch
plan. Confirmed nothing in the plan is stale: `analysis/engine.py` and `feedback/rules.py` are
still `NotImplementedError` stubs; `contracts/{intent}.py` and
`analysis/{benchmarks,phases,checkpoints,scoring}` don't exist yet (all new); hatchling packages
all of `src/golf_coach`, so a `benchmarks/ranges.json` ships with no pyproject change.
**Plan in one line**: build the pose-only Fundamentals spine
(`FrameKeypoints + PracticeGoal → analyze_swing → SwingResult → build_feedback → FeedbackPayload`)
with a single **tempo** checkpoint, the dual-axis/intent seam in place, benchmark ranges as
JSON-with-provenance (Tour Tempo ~3:1 seeded), all on the stdlib base install. Full breakdown of
the 10 change-sets (new modules, contract extensions, tests, verification) is in the plan doc.
**Key decisions (all captured in the plan doc)**:
- **Scope = M4-PoC only, one tempo checkpoint.** No `merge.py`, no outcome checkpoints, no extra
  scoring policies, no SQLite — those are full-M4 and left as named seams.
- **Analysis core stays pure-Python/stdlib** (no numpy/MediaPipe) so the spine + tests run on
  `pip install -e .`; benchmark store ships as **JSON not YAML** to keep base deps tiny.
**Where I left off**: Plan is documented and approved; **no analysis code written yet**. Next
session: implement change-sets 1→9 in `docs/archive/M4_POC_PLAN.md` (contracts → benchmarks → phases →
checkpoint → scoring → engine → feedback → tests), then write the `docs/archive/M4_ANALYSIS_POC.md`
milestone flow doc and check off the ROADMAP boxes.
**Blockers**: None — pose-only, no hardware.
**Notes**: Real-clip eyeball can reuse the existing face-on keypoints JSON in `data/processed/`
(from `aaron-swing-2.mov`) for the exit-criterion sanity check.

---

## 2026-07-02 — M1 angle re-shoot: face-on confirmed as canonical pose placement

**Duration**: ~0.5 hour
**What I did**: Re-shot the swing from a **face-on** angle (`data/raw/aaron-swing-2.mov`,
480×854, 58.9 fps, 656 frames) to test the self-occlusion hypothesis from the 06-28 review,
ran it through `run_pose.py`, and computed the same metrics on both clips for a
side-by-side (detection, per-group visibility, knee-by-decile, jitter).
**Findings** (full write-up:
[M1_CAPTURE_FLOW.md → angle comparison](docs/archive/M1_CAPTURE_FLOW.md#m1-findings-angle-comparison-2026-07-02)):
- Face-on wins on **every** body metric. **Knees 0.71 → 0.88 (+24%)**, lower body
  0.70 → 0.83, overall visibility 0.78 → 0.89. Leg jitter also dropped.
- The big one: knee confidence now stays 0.85–0.95 *through the bent swing posture* where it
  used to collapse to ~0.60. Confirms the occlusion diagnosis — at ~5 o'clock the legs
  stacked front-to-back; face-on separates them.
- Honest caveat: all-landmark jitter rose slightly, but that's genuine (larger arm arc in
  face-on), not noise. Temporal smoothing still worth doing.
**Key decision**:
- **Canonical pose-camera placement = face-on / 3 o'clock** (ball flies to 12; RH golfer,
  mirror to 9 for LH). Recorded in **ADR-003 addendum (2026-07-02)**, which also reconciles
  this with the ADR's original "down-the-line first" note: down-the-line is for the *club*
  stream (M2/YOLOv8), face-on is for the *pose* stream.
**Where I left off**: M1 pose setup is now solid. The "re-record & re-review lower body" open
refinement is **done**; only the optional temporal-smoothing pass remains. Ready to start
**M4-PoC** (tempo) on the new keypoints JSON whenever we choose.
**Blockers**: None.
**Notes**: old raw clip (`golf_swing-aaron-1.mov`) no longer in `data/raw`, but its
keypoints JSON remains in `data/processed` and was used for the comparison.

---

## 2026-07-02 (cont.) — Spine-angle investigation → camera topology & sync plan

**Duration**: ~0.5 hour
**What I did**: Investigated a hunch that shot 2's spine looked "very vertical." Estimated
spine tilt (shoulder-mid → hip-mid vs vertical) on both clips, in 2D and depth-aware 3D.
Used the result to settle the camera topology and to plan synchronization.
**Findings**:
- Shot 1 (~5 o'clock): ~37° spine tilt at address (believable). Shot 2 (face-on): ~2° in the
  image — looks dead vertical, but that's a **projection artifact**: forward tilt lives in the
  camera's depth axis and foreshortens to ≈0 face-on. `z`-based 3D put shot 2 at ~66–71° (if
  anything more bent), but MediaPipe `z` is unreliable. **Can't** conclude a more upright
  stance — the "vertical" look is the camera angle. Rule: forward spine tilt = down-the-line
  measurement, not face-on.
**Key decisions (documented)**:
- **Two cameras, not three.** The spine/hip stream and the club stream share the
  down-the-line viewpoint; one global-shutter camera there runs both MediaPipe + YOLOv8. →
  **ADR-003 addendum (2026-07-02b)** with the stream→camera assignment table + spine caveat.
- **ADR-011 (Proposed): Camera Synchronization & Multi-View 3D Fusion** — phased plan
  (Phase 1 single-cam now, build `camera_id`+timestamp+`FrameBundle` seam; Phase 2 two-cam
  software/event sync + calibration; Phase 3 hardware trigger for frame-accurate dynamic 3D).
  Unlocks true spine angle / hip rotation / X-factor / kinematic sequence for M4 mechanics.
- Also noted the spine caveat in M1_CAPTURE_FLOW.md and added both items to ROADMAP Future.
**Where I left off**: Camera plan is now on record end-to-end (angle, count, stream
assignment, sync roadmap). No code change — ADR-011's capture-seam work (`camera_id` on
`Frame`, `FrameBundle`) is a small future task, not needed for M1/M4-PoC.
**Blockers**: None.
**Notes**: ADR-011 is Proposed/forward-looking — no multi-cam hardware yet; seam designed now.

---

## 2026-06-28 (cont. 2) — M1 accuracy review & documentation

**Duration**: ~1 hour
**What I did**: Ran the M1 pipeline on the first real swing clip
(`data/raw/golf_swing-aaron-1.mov`, 480×854, 58.9 fps, 674 frames) and did the accuracy
review. Then documented the model + this step thoroughly so it's easy to pick back up.
**Findings** (full write-up in
[docs/archive/M1_CAPTURE_FLOW.md → M1 findings](docs/archive/M1_CAPTURE_FLOW.md#m1-findings-accuracy-review-2026-06-28)):
- 100% detection, avg visibility 0.78; **upper body tracks well**; **59 fps is plenty**.
- **Knees/lower body weak during the bent swing posture**, fine once standing; plus some
  **jitter**.
- Ran a **lite-vs-heavy model experiment on the same clip** — heavy did NOT fix the knees
  (and is 3–5× slower). Inspecting matched frames, the cause is the **picture** (dim light,
  dark shorts on dark floor = no leg contrast, cluttered background), not model size.
**Key decisions**:
- **Keep the lite model** (`pose_landmarker_lite.task`). Documented the whole model story —
  Tasks API vs legacy Solutions API, the lite/full/heavy variants, the download URL/location
  (`data/models/`, via `_ensure_model`) — in **ADR-002 addendum** and the M1 flow doc's new
  **"Pose model (reference)"** section.
- The lower-body fix is a **better recording**, not code: more/even lower-body lighting,
  declutter, contrast the legs, tripod framing. Captured in the findings.
**Where I left off**: M1 is effectively done and fully documented. Pose-setup refinement
continues **later**: (1) re-record with the lighting/background fixes, (2) optionally add a
temporal-smoothing pass for jitter. Otherwise ready to start **M4-PoC** (tempo) on the
keypoints JSON whenever we choose.
**Blockers**: None.
**Notes**: heavy model + heavy overlay were generated as a one-off experiment (gitignored
under data/); committed default remains lite.

---

## 2026-06-28 (cont.) — M1 implementation

**Duration**: ~1.5 hours
**What I did**: Implemented Milestone 1 (Capture & Skeleton). Wrote `FileVideoSource`
(`capture/file.py`, a `VideoSource` adapter over `cv2.VideoCapture`), implemented
`estimate_pose` (`pose/estimator.py`) with the raw→`FrameKeypoints` mapping isolated in a
pure `_to_frame_keypoints` helper, added `draw_skeleton` (`pose/overlay.py`) working purely
off our contract, and wired the lot in `scripts/run_pose.py` (capture → pose → keypoints
JSON + skeleton overlay mp4). Added tests for the pure mapping (no ML deps) and for
`FileVideoSource` (guarded by `importorskip` so the base suite still runs). Set up a `.venv`,
installed `.[vision,dev]`, and verified end-to-end on a synthetic clip. Wrote the M1 design
doc `docs/archive/M1_CAPTURE_FLOW.md` (with mermaid data-flow + sequence diagrams).
**Key decisions / surprises**:
- **MediaPipe Tasks API, not the legacy Solutions API.** The installed mediapipe (0.10.35
  on Python 3.13) has *removed* `mp.solutions` — only `Image`/`ImageFormat`/`tasks` remain.
  Rewrote the estimator to use `PoseLandmarker` (VIDEO mode); it needs a `.task` model
  bundle, so `_ensure_model()` downloads `pose_landmarker_lite.task` (~5 MB) into
  `data/models/` on first run. Corrected `docs/archive/M1_CAPTURE_FLOW.md` accordingly.
- Undetected frames emit 33 placeholder landmarks at visibility 0 (one record per frame) so
  the timeline stays aligned for M4-PoC.
**Verification**: `ruff check` clean; `pytest -q` → 9 passed (4 contract + 3 mapping + 2
capture); `python scripts/run_pose.py <synthetic.mp4>` produced a 15-frame keypoints JSON
(33 landmarks each) and an overlay mp4. Synthetic artifacts cleaned up afterward.
**Where I left off**: M1 code is complete and verified mechanically. **Next: the human
accuracy review** — drop a real swing clip at `data/raw/swing.mp4`, run `run_pose.py`, and
confirm the skeleton tracks address→follow-through; then document findings (30fps enough?
keypoints stable?) to close M1. After that, M4-PoC (tempo) can consume the keypoints JSON.
**Blockers**: None — needs a real swing clip for the accuracy review (no hardware required).
**Notes**: `.venv` created locally; `data/models/` and `data/processed/` are gitignored.

---

## 2026-06-28

**Duration**: ~1 hour
**What I did**: Design session on the central question — *how does the system decide what a
good swing is, distinguish good from bad, and pick the correction?* Surveyed where real
benchmark data comes from (outcome: TrackMan tour/amateur averages, Arccos/Shot Scope,
FlightScope, ShotLink; mechanics: TPI kinematic sequence + X-factor, GEARS/AMM3D/K-Vest,
academic biomechanics, Tour Tempo). Concluded the industry moat is *owning the data*, so v1
leans on published norms stored as cited data and migrates to our own captured data later.
Made four decisions and wrote them up as ADR-009 and ADR-010, then updated ROADMAP.
**Key decisions**:
- ADR-009 — **dual-axis scoring**: separate `mechanics_score` and `outcome_score`, combined
  by a **scoring policy chosen by the user's practice mode** (fundamentals / shot-shaping /
  performance / drill). Intent (`PracticeGoal`) parameterizes outcome ranges, so "good fade
  when I wanted straight = bad" and "don't care where it went, grade my fundamentals" both
  fall out naturally. `SwingResult` + `analyze_swing` carry intent and two sub-scores from
  day one. Supersedes the original single-score M4 framing.
- ADR-010 — **benchmark ranges as versioned data with provenance** (not hardcoded), resolved
  via `resolve_range(checkpoint, club, profile)` with most-specific→`all` fallback;
  parameterized by `ClubCategory` and `PlayerProfile`. v1 seeds **Tour Tempo (~3:1)** only;
  expand to TPI / TrackMan / Arccos as M2/M3 land, then to our own calibration data.
- PoC scope — **M4-PoC: pose-only Fundamentals analysis**. Prove phases→checkpoint→score→tip
  end-to-end on the M1 skeleton with the tempo checkpoint, one scoring policy, the intent +
  dual-axis seam in place but only Fundamentals implemented. No club detection, no hardware.
**Where I left off**: Decisions captured in ADR-009/010; ROADMAP now has M4-PoC before the
full M4. No analysis code written yet — `analysis/engine.py` and `feedback/rules.py` are
still stubs. M1 (capture + pose) remains the prerequisite for M4-PoC.
**Blockers**: None. M4-PoC depends on M1 producing keypoints first.
**Notes**: M1 is still the next *code* step; M4-PoC is the first analysis iteration that
follows it.

---

## 2026-06-20

**Duration**: ~1.5 hours
**What I did**: Picked the project back up after the planning session. Reviewed all docs/ADRs to re-orient. Decided to decouple software development from hardware acquisition and wrote ADR-007 to capture it; annotated ROADMAP milestones with explicit hardware dependencies. Discussed the role of YOLOv8 and how swing path is captured. Assessed how hard reliable club-head tracking really is and decided to de-risk it before investing — added **Milestone 1.5: Club-Head Detectability Spike** with a go/no-go gate. Then designed and scaffolded the whole project per ADR-008: `src/golf_coach/` package, the shared `contracts/` seam (fully implemented Pydantic models), ports + a working mock shot source, module stubs, pyproject with optional extras, tests, scripts, spikes/, frontend/, data/. Verified: `pip install -e '.[dev]'` works, `pytest` green (4 contract tests), `ruff` clean. Added `docs/FLOW.md` with PROPOSED-flow mermaid diagrams (runtime data flow, decoupling seam, build order + hardware gates, two-source swing path) and flagged ARCHITECTURE.md's diagrams as proposed too.
**Key decisions**:
- ADR-007 — software and hardware tracks run in parallel. M1 (capture + MediaPipe pose) starts now with phone/sample video, no purchase needed. MCP server (M3) gets a mock `ShotData` mode from the start. Hardware purchase is a parallel task, not a blocker.
- Added M1.5 detectability spike: prove the club head is detectable (especially through impact) BEFORE labeling 200–500 images. Strategy options to choose from once we see real frames: pure-ML / marker-assisted / fusion+interpolation. M2 labeling is now gated on this spike. Charter risk register and ADR-003 updated accordingly.
- ADR-008 — project structure & decoupling: a shared `contracts/` package is the seam (modules never import each other), ports+adapters at I/O boundaries (real vs mock), analysis is a pure functional core, heavy deps are optional extras so any module runs without the ML stack.
**Clarified**: YOLOv8 detects the club head + ball (MediaPipe only tracks the body); its detections through a tracker produce the visual club-path arc. The Garmin R10's `club_path` is the numeric counterpart. YOLOv8 is also the train-it-yourself ML exercise (ADR-005) and the most cuttable piece if scope tightens. Important correction: **global shutter removes distortion, not motion blur** — sharp impact frames need fast shutter + bright light (ADR-003 addendum).
**Where I left off**: Project is scaffolded and the contracts seam is real (tests green). Next concrete step is implementing **M1**: install the `vision` extra, write `FileVideoSource` (capture/file.py), implement `estimate_pose` (pose/estimator.py) over a sample/phone clip, and render a skeleton overlay via `scripts/run_pose.py`. In parallel, the M1.5 spike — grab a few swing clips with the impact zone and eyeball club-head detectability before any labeling.
**Blockers**: None for M0/M1/M1.5. Hardware purchase (cameras + R10) pending but no longer blocking.
**Notes**: —

---

## 2026-03-16

**Duration**: ~1 hour
**What I did**: Project planning session. Created project charter, architecture doc, roadmap, ADR templates, and decision log. Defined tech stack and milestone sequence.
**Key decisions**: Python stack, MediaPipe for pose, YOLOv8 for detection, MCP server for launch monitor, Claude API for coaching. See `docs/decisions/` for full ADRs.
**Where I left off**: No code yet. Next step is Milestone 1 — acquire/set up camera, record a test swing, run MediaPipe.
**Blockers**: Need to decide on and purchase camera hardware (ADR-003) and launch monitor (ADR-004).
**Notes**: —

---

<!--
TEMPLATE — copy this for each session:

## YYYY-MM-DD

**Duration**: 
**What I did**: 
**Key decisions**: 
**Where I left off**: 
**Blockers**: 
**Notes**: 

-->
