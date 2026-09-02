# ADR-023: Tempo Training, and Absolute Swing Durations as Reference Data

## Status
Accepted. Built and surfaced: the durations are derived, the two beat patterns are computed
server-side, and the results page plays them.

**Three addenda at the foot, and two of them correct the text above them.** 2026-08-20: the
Context section below concludes "no mph axis" from a club-stratified test. Club is the wrong axis —
it changes speed by lengthening the lever, not by rotating faster — and on a real speed cohort the
durations *do* move, so the target follows the golfer rather than a speed class. Read that addendum
before acting on the Context. 2026-08-22 extends rather than corrects: the trainer gains a career
scope and a third anchor state, lifting the first item under *Deferred, by choice*. 2026-09-02
reverses the first addendum's *anchor* and only that: the target follows the golfer's own
**downswing** — the half they feel — and the backswing is what it prescribes. §1 survives all three
— no band on either duration, and the panel is still six checkpoints.

## Date
2026-08-20

## Context

Tempo has been in this project since M4-PoC and it is the one checkpoint that ships a verdict a
golfer cannot act on. `evaluate_tempo` says *"Tempo too quick"* and stops. Every other checkpoint
names something a golfer can see themselves doing — a head that slid, a finish that drifted — but
"3.4:1" is a number about a swing already over, and the drill for it is a metronome nobody here
had.

The ask was a practice tool: a timed sequence a golfer swings to.

### The requested design does not survive the data

The original framing was a tempo **indexed by club speed** — a 70 mph tempo, an 80 mph tempo. The
corpus was asked, using club as the only club-speed proxy it has. Derived from the 310
non-slow-motion GolfDB clips that carry a real frame rate (168 players, all 30 fps):

| club | n | backswing | downswing | ratio |
|---|---|---|---|---|
| driver | 225 | 901 ms | **267 ms** | 3.56 |
| iron | 41 | 801 ms | **267 ms** | 3.25 |
| hybrid | 8 | 934 ms | **267 ms** | 3.94 |
| fairway | 33 | 801 ms | 234 ms | 3.33 |

Between-club sd of downswing duration is **6.9 ms** — a fifth of one frame. Between-*golfer* sd is
**47.0 ms**, 6.8x larger. Across the widest club-speed range in golf, tour downswing duration does
not move.

That independently reproduces [ADR-010](010-benchmark-ranges.md)'s 2026-08-18 finding for the
*ratio* (golfer effect 4.7x the club effect) on the *absolute durations*, which is a stronger
result: the ratio could have held constant while both halves scaled, and it does not — the
downswing itself is near-invariant.

Two further facts point the same way. `analysis/shot_measure.py` already refuses to record
`club_head_speed`, because every shot on disk reads a smash factor below 1.0 — ball speed under
club speed, which no strike produces. And there are two shots on disk. **An mph-indexed tempo table
would be a fabricated relationship keyed off a known-bad number.**

So: no mph axis. Instead a *pace* scale that multiplies both durations while holding the ratio —
the practice experience, without a calibration nobody has measured.

## Decision

### 1. Absolute durations ship as reference distributions, never as a band

`golfdb_v1.json` gains `backswing_ms` and `downswing_ms`. It carried `tempo_ratio` and no duration
at all, because the whole GolfDB pipeline was deliberately built **frame-rate invariant** — 641 of
1,399 clips are slow-motion, and a ratio survives that where a duration does not.

These two rows are the exception and they are fenced accordingly:

- **No band, no checkpoint, no placement.** Tempo is already scored once. A band on each half would
  put a second and third verdict on one fundamental and count it repeatedly into `overall_score`.
  `ranges.json` is untouched and the panel stays six. `derive_reference.py` prints no recommended
  band for them, which is otherwise exactly the invitation its report is.
- **A different inclusion rule, recorded per row.** `Distribution` gains an optional `provenance`
  field for this — the file-level `dataset` block stops being true the moment one metric is drawn
  from a different slice than the rest, and a reader has no way to notice. The block now points at
  the field rather than restating the exception.

The restriction is worth stating plainly, because it was not obvious when this was planned: **every
one of the 310 eligible clips is down-the-line.** The face-on half of the pose cache was extracted
before the keypoints file carried a `clip` envelope, so no face-on clip has a recoverable frame
rate, and the source videos are not kept. That does not compromise the numbers — a duration comes
from GolfDB's ground-truth event labels and the clip's frame rate, and reads no landmark at all, so
the camera position is irrelevant to it. It does mean the sample is one view, and the row says so.

At 30 fps the instrument quantizes at 33 ms on a downswing spanning six to nine frames. These are a
distribution, not a stopwatch.

### 2. The two halves become measurements, judged by nothing

`tempo_timings` computed `backswing_ms` and `downswing_ms` and divided them away, so no stored
swing could say *"your downswing took 384 ms"* — and a ratio of 2.35 is the same swing whether both
halves are slow or both are quick, which need opposite advice. Both now join `measurements` under
M6.5's measure-now-judge-later order, inheriting `tempo_timings`' three refusal paths so a clip has
one answer about whether it was timeable.

`ANALYSIS_VERSION` 6 → 7. No score moves; a version-6 artifact is *missing* two quantities rather
than disagreeing about any.

### 3. Two beat patterns, and that is the substance of the design

**No single comfortable pulse marks both the top and impact.** The two intervals differ by ~3.4x: a
pulse slow enough to settle into (901 ms, ~67 BPM) cannot mark impact, and one fast enough to mark
impact (267 ms, ~225 BPM) is not a groove. This is why Tour Tempo ships tones rather than a
metronome. Both answers to it ship, switchable:

- **`GRID`** — the backswing snapped to a whole number of downswing-length ticks. Buys a steady,
  trackable, loopable click; pays with a ratio that is no longer the tour median's. On today's
  distributions it resolves to 3 ticks, and the count is **derived, never written** — it is an
  output of the medians.
- **`CUES`** — three tones at the exact tour medians. Keeps the true ratio; pays with ~900 ms of
  silence through the backswing.

They are **not two renderings of one sequence**. The grid rounds and the cues do not, so the two
carry different durations and different ratios — which is why those live on `BeatPattern` and not
on `TempoPlan`. One `ratio` field beside two patterns would be false for one of them, quietly,
since both are plausible numbers.

Both are computed server-side. A page deriving one from the other would print `GRID`'s rounded
ratio as though it were the tour's, or the tour's as though it were what the golfer is hearing.

### 4. Read-only, and the trainer sets no target it did not measure

Nothing records that a golfer practiced; the mode toggle is a client-side preference that persists
nowhere. [ADR-020](020-conversational-followups.md) says a write path needs its own decision, and
this respects that rather than stretching it.

`build_tempo_plan` returns `None` when the distributions are unavailable. There is no fallback
constant, and the stakes are higher than the usual [ADR-010 §2](010-benchmark-ranges.md) case: a
wrong verdict is read once, and a wrong metronome is rehearsed.

## Consequences

**A `PersonalBaseline` for tempo is now buildable and still gated on `n`.** ADR-010's 2026-08-18
addendum concluded tempo is a personal signature whose home is `PersonalBaseline`. That is
unchanged, and the two duration measurements are the raw material it will want — a golfer whose
ratio is in band while both halves are slow is a finding no ratio can carry.

**`tune_spatial_metric.py` cannot score an absolute duration, and now refuses to.** Its `boundary`
column compares a metric at labelled instants against the same metric at detected ones — but
labelled phases carry *frame indices* in `start_ms` by design (`_phases_from_events`, since only
ratios were ever read from them) while detected phases carry real milliseconds. For a duration that
subtraction is frames minus milliseconds. Left alone it printed `backswing_ms` at ratio 0.1,
"MOSTLY NOISE", which reads exactly like a finding about the metric and is a unit mismatch.

**`derive_pose_metrics.py` would have corrupted the corpus, and now excludes the same set.** It
measures every registered metric at labelled instants and writes the results to `swings.jsonl`. A
duration read off those phases is a frame count wearing a millisecond's name; written to the file it
would sit beside the real milliseconds `derive_reference.py` computes and every duration band would
be cut from the mixture. Nothing in `tests/` covers the research scripts. The excluded set is
`measure.FPS_DEPENDENT_MEASUREMENTS`, derived from the metric's own unit rather than listed, so a
third duration is covered the day it is added.

**The two tolerances in `METRIC_TARGETS` are composed, not tuned.** Since the harness cannot measure
them, they are built from the M4-REF instant errors — a duration is a subtraction of two instants,
so its error is theirs summed: backswing (2 + 7) frames ≈ 300 ms, downswing (1 + 2) ≈ 100 ms.
Address dominates the first exactly as it dominates `tempo_ratio`'s tuned 0.943, and the two agree
in size, which is the only cross-check available. Both are estimates from a broadcast corpus and are
first in line for revision from a bay session.

**`common.keypoints_path` was wrong and is fixed.** It omitted the per-estimator level the cache has
always had, returning a path that has never existed. It had no callers, so nothing failed and the
bug sat there looking like the way to do it.

## Deferred, by choice

- **Diagnosing which half to fix.** *(Partly lifted 2026-08-22 — see the second addendum. The two
  halves are now shown beside the ratio on the career page, which makes the diagnosis visible;
  saying it in the product's own voice is what stays deferred.)* The trainer sets the ratio and
  gives instructions; working out
  whether the backswing or the downswing is the problem is left to the golfer and the metronome.
  Consequence 2 above is the groundwork — once a personal baseline exists over the two durations,
  the diagnosis is a comparison rather than a guess. Worth knowing for whoever picks it up: the
  golfer's two stored swings are 901/384 ms (2.35) and 968/401 ms (2.42). The backswing is *exactly*
  the tour median; the downswing is 28% past the p90 edge. Tempo fails here because the downswing is
  slow, not because the backswing is quick — which is the opposite of what "Tempo too quick" sounds
  like it means.
- **Revisiting club speed.** If the launch monitor's smash factor is ever fixed and `n` grows, the
  mph question can be asked of real data rather than of a club-name proxy. The finding above is
  about tour professionals and says nothing about whether one amateur's tempo varies with club.
- **A face-on duration sample.** Re-extracting the face-on cache with a `clip` envelope would roughly
  double the sample and remove the single-view restriction. It costs a re-download of the corpus and
  changes no number anyone is currently reading.

## Alternatives Considered

**An mph-indexed tempo table, as originally asked for.** Rejected on the measurement above: the
club effect on downswing duration is 6.9 ms against 47.0 ms between golfers, and the only club-speed
number on disk is known bad. It would have been a fabricated relationship presented as a calibration.

**One beat pattern.** Rejected as unbuildable rather than undesirable — see §3. Either choice alone
gives up something a golfer needs, and the two together cost one toggle.

**Storing `TempoPlan` on `SwingResult`.** Rejected: nothing in it is a measurement of the swing, and
deriving it at read time means every already-stored swing gets a trainer with no re-analysis and no
contract change.

**Bands for the two durations.** Rejected as double-counting one fundamental — §1. The distribution
is still readable where it belongs, in the trainer, which turns it into a target to swing at rather
than a verdict.

---

## Addendum — 2026-08-20: swing speed *does* move duration, and the target now follows the golfer

**The Context section above tested the wrong axis and drew too broad a conclusion from it.** "No mph
axis" was inferred from a club-stratified test, and club is not a speed axis in the sense the
question meant. The prompt that caught it: *"the tempo of a 70 mph swing and a 100 mph swing are
going to vary — they will be the same ratio but they are going to be different."* That is correct,
and the corpus says so.

### Why club was the wrong proxy

A longer club raises head speed by **lengthening the lever**, not by rotating faster. The body turns
through roughly the same arc in roughly the same time and the clubhead simply covers more distance,
so duration holds — which is exactly the 6.9 ms the club test found. A golfer who swings harder
raises head speed by **rotating faster**, which compresses the swing. Two different mechanisms; only
the second changes duration, and the club test could not see it.

### The measurement, on the axis that answers it

GolfDB carries `sex`, and LPGA against PGA is a real speed cohort — roughly 94 against 113 mph with
a driver. Driver only, so the club mix cannot confound it, and one vote per golfer so a
much-clipped player cannot carry the result:

| cohort | golfers | backswing | downswing | ratio |
|---|---|---|---|---|
| LPGA | 57 | **1001 ms** | **267 ms** | 3.75 |
| PGA | 84 | **834 ms** | **234 ms** | 3.57 |

A ~20% speed difference moves the backswing **+167 ms (+20%)** and the downswing **+33 ms (+14%)** —
about five times the club effect. Note the *ratio* shifts too (3.75 against 3.57), so this is not a
pure rescale of one shape.

**Two limits, stated because they bound what this licenses.** The 33 ms downswing gap is exactly the
30 fps quantum: it is resolvable as a median over 141 golfers and *not* on a single clip. And it is
about the size of one sd of person-to-person variation (36 ms), so a speed cohort is a real
population effect rather than a precise per-golfer predictor. Both are reasons to fit to the golfer
directly rather than to a speed class.

### What changed

`build_tempo_plan` now **anchors the target to the golfer's own measured backswing**, and derives
the downswing from it at the tour ratio. *(Reversed 2026-09-02 — see the third addendum. The
measurement below stands and so does fitting to the golfer; the half held fixed is now the
downswing, and the guard described further down is deleted rather than flipped.)* A slower golfer's longer backswing *is* the speed signal,
measured rather than inferred — so this captures the effect above without a club-head speed, which
matters because there is still no usable one: every stored shot reads a smash factor below 1.0.

The ratio stays the tour's. It is the anchor's *length* that follows the golfer, never the shape —
the ratio is what the tempo checkpoint judges, and personalising it would coach the golfer toward
their own fault.

**The fit is carried by `pace`, and the patterns stay at the tour reference.** That is what keeps
the golfer's pace control meaningful: the slider is a multiple of the tour median, the page pre-sets
it to the fitted value, and dragging it is a plain override. Baking the anchor into the beats
instead would have left the control reading 100% for every golfer — a slider that cannot show the
decision it exists to override, which is the same class of omission as a placement shipped as a bare
float. It also leaves exactly **one** place where a pace is applied: `build_tempo_plan` used to take
a `pace` and pre-multiply while the page multiplied again by its slider, a double application
waiting for the first caller to pass one. Nothing ever did. The parameter is gone rather than
documented, and the control's bounds are pinned against the guard's own range so a fitted pace is
always reachable.

**Guarded**: the anchor is used only when the observed backswing falls inside the corpus p10–p90.
Outside it the backswing is plausibly the fault itself, and anchoring would build the drill around
the half that needs changing — a 400 ms backswing would be handed back with a downswing scaled under
it, a drill that reads correct and rehearses the error. `TempoPlan.anchored` and
`anchor_backswing_ms` carry which case applied, because the beats alone cannot show it, and the page
says so in words.

Note the guard uses a *distribution's* p10–p90 rather than a band, and this is not §1 being walked
back. Nothing is scored, nothing reaches `SwingResult`, and the range decides whether a number is
usable as an anchor — a different act from judging a golfer against it.

**Anchoring is a generalization, not a change of default.** With no usable backswing the anchor is
the tour median and the arithmetic collapses onto exactly what shipped this morning: `GRID` keeps
the tour downswing (267 ms) exactly and `CUES` both medians. Pinned, so the fallback cannot drift.

One consequence worth recording: `GRID` absorbs its integer rounding in the **backswing** and never
in the downswing. The downswing is the near-invariant half (p10 200, p90 300) and the quantity being
corrected, so moving it to make a whole tick count fit would teach a materially different swing; the
backswing spans 700–1204 and a tick of it is inside the natural spread.

### Still deferred

The mph question proper. This anchors to a *duration*, which is the observable, and never claims a
speed. Whether an amateur at 70 mph follows the same duration relationship as a tour professional at
94 is untested — every golfer in this corpus is a tour player, and the honest reading of the table
above is that it describes the gap between two professional tours, not the whole speed range of
golf. A bay session with a working smash factor is what would extend it.

---

## Addendum — 2026-08-22: the trainer gets a career scope, and a third anchor state

**This lifts the first item under *Deferred, by choice*, and only that item.** That entry said the
groundwork for diagnosing tempo was "once a personal baseline exists over the two durations"; it
does, career mode built it, and nothing was reaching for it. Nothing about the beat patterns, the
bands or the mph question changes here.

### What was actually missing

The trainer existed only on the results page, gated on the tempo checkpoint having *failed* on the
swing being viewed. Three consequences, none of them intended:

1. A golfer whose tempo passed never saw a metronome, and never saw their own ratio anywhere
   outside a table cell on one swing's page.
2. There was no way to ask "what is *my* tempo" — the question every user of a simulator has,
   because every golfer's answer is different, and the one this repo had the measurements for.
3. `backswing_ms` and `downswing_ms` had been measured per swing since this ADR shipped and were
   read by nothing but the anchor of the swing they came from.

### The shape

`contracts/tempo.py` gains `CareerTempo`, built by `analysis/tempo_trainer.build_career_tempo` from
a `CareerCorpus`, and served on `/api/golfers/{player_id}/career` beside the three career contracts
already there — one route over one `read_corpus`, so the tempo a page prints and the `tempo_ratio`
card under it cannot describe different swings.

**Three tiers, gated differently, and the split is the whole design:**

| tier | field | gate |
|---|---|---|
| readings | `swings`, `latest` | none — a measurement asserts nothing about the golfer |
| claims | `typical_ratio`, `typical_backswing_ms`, `typical_downswing_ms` | `PersonalBaseline`'s `CENTER` guard, verbatim |
| target | `plan`, `anchor` | the anchor guard already in this ADR |

That split is what lets this ship useful at n=2. `tempo_ratio` needs 8 samples for a `CENTER` claim
(`contracts/baseline.py`, the noisiest metric in the panel), and the golfer on disk has two — so a
view that could only show a mean would show nothing at all, while the two numbers actually wanted
are sitting measured in the artifacts. Printing the readings is not the guard being bent: the guard
governs what may be *asserted about the golfer*, and "this swing took 901 ms back and 384 ms down"
asserts one swing. `SessionSample` already draws exactly this line, carrying its `n` ungated while
its mean waits.

### The third anchor state

`TempoPlan.anchored` is a bool because one swing has two answers: its own backswing, or the tour
median. A career view has three, and the middle one is the point:

- `CAREER_MEAN` — the mean backswing, once `CENTER` allows it.
- `LATEST_SWING` — the most recent measured backswing, while it does not. **Not a mean smuggled
  past the guard**: it is one swing's measurement used as a target, which is exactly what the
  results page has done per swing since this ADR shipped. One measured swing is a better target
  than a population median for a golfer whose speed the population does not describe.
- `TOUR_MEDIAN` — nothing measurable, *or* the anchor guard above rejected what there was.

`anchor` is read back off the built plan rather than decided alongside it. `_anchor_backswing` can
refuse an observed backswing outside p10–p90, and a view that had branched on its own copy of that
rule would print "matched to your own backswing" over a drill built on the median — the failure
this ADR's guard exists to prevent, reintroduced one layer up.

### Two things this deliberately does not do

**It does not diagnose which half is wrong.** That was the same deferred entry's actual subject and
it stays deferred. The page now shows the backswing and the downswing beside the ratio — which is
what makes the diagnosis *visible* to the golfer, and is the case this ADR already records: 901/384
is a tour-median backswing with a downswing 28% past p90, so "tempo too quick" is true of the ratio
and the opposite of what it sounds like. Saying that sentence in the product's own voice needs a
rule about when it is safe to say, and that rule needs more than two swings.

**It records nothing.** Still read-only, for the reason §Why it is in `contracts/` gives: ADR-020
says a write path gets its own decision, and "the golfer practiced to this" would be one.

### Consequence for the pages

`build_tempo_plan` splits into itself and `build_tempo_plan_for`, which takes the two durations
directly; both scopes go through the second, so there is one definition of when a golfer's own
backswing may be practiced to. On the client, the metronome's mechanism moves to
`static/tempo.js` and is shared by `results.html` and `career.html` — a second Web Audio scheduler
is a second thing that drifts. A plain `<script src>` with no build step, which is what
`REFACTOR_LEDGER.md`'s 2026-08-13 row on the static pages permits: the objection there was a node
toolchain, not a second file. Each page keeps its own framing prose, because the two genuinely say
different things and the anchor sentence has a different number of cases on each.

---

## Addendum — 2026-09-02: the anchor moves to the downswing, the half the golfer feels

**This reverses one choice in the 2026-08-20 addendum and nothing else in it.** That addendum's
measurement stands — a real speed cohort moves the durations, club does not, and the honest response
is to fit the target to the golfer rather than to a speed class. What it got wrong is *which half to
fit to*. It anchored on the golfer's measured **backswing** and derived the downswing from it at the
tour ratio, which prescribes a downswing.

A golfer cannot take that instruction. **The downswing is what they feel — it is how hard they swung
— and the backswing is the half they can deliberately change.** Anchoring on the backswing asks them
to change the thing they experience as effort; anchoring on the downswing asks them to change the
thing they experience as a decision. So the arrow turns round: read the downswing, multiply by the
tour ratio, prescribe the backswing.

### What it does to the golfer on disk

Reference rows are `golfdb_v1.json`, stratum all/all/all, n=310, down-the-line only. Tour ratio is
`900.68 / 266.84` = **3.3755**. The latest stored swing is **901.2 ms back / 383.9 ms down**, 2.35:1.

| | before | after |
|---|---|---|
| anchor | backswing 901.2 ms (inside p10–p90, accepted) | downswing 383.9 ms |
| `pace` | `901.2 / 900.68` = **1.00** | `383.9 / 266.84` = **1.44** |
| `CUES` target | 901 back / 267 down | **1296 back / 384 down** |
| verdict | "aim for the tour range 2.72–4.71:1" | "your downswing was 384 ms; at the tour ratio that wants a 1044–1808 ms backswing and yours was 901" |

The old anchor was a coincidence doing no work: this golfer's backswing *is* the tour median, so a
fit that reads 1.00 personalized nothing while the half that was actually 44% long went unmentioned.

### §1 is not walked back — no band on either duration

The `tempo` checkpoint keeps scoring `tempo_ratio` against the unchanged `[2.72, 4.71]` row in
`ranges.json`. The panel stays at six checkpoints, `overall_score` is byte-identical and
`ANALYSIS_VERSION` does not move — the only stored field that differs is `message`, and stored
swings pick it up with `scripts/reanalyze.py --all`.

The re-worded verdict is a change of *voice*, not of arithmetic: `901 ∈ [2.72 × 384, 4.71 × 384]`
exactly when `ratio ∈ [2.72, 4.71]`. Reading "base tempo on the downswing" as licence to band
`downswing_ms` would grow the panel to seven, count one fundamental twice into `overall_score`, and
make the *score* frame-rate dependent. It is still refused, for §1's original reason.

### The anchor guard is deleted, not flipped

`_anchor_backswing`'s p10–p90 rejection existed because an out-of-range backswing is plausibly the
fault itself, and a drill built on it rehearses the error at a tempo that reads correct. Under this
anchor the downswing is never the fault — **it is the given**, the thing that happened, and the
prescription lands on the other half — so the guard has nothing left to guard. Keeping the rule and
merely swapping the metric would refuse this golfer's 383.9 ms (outside 200.20–300.30) and hand back
the tour median, which is precisely the state this reversal exists to leave.

What replaces its protection, so it is not simply discarded:

- `downswing_in_tour_range: bool` — whether the observed downswing sits inside the reference
  p10–p90, reported on the plan and said in words by the page.
- `in_range_pace: float | None` — the pace that clamps the downswing to the nearest tour edge,
  `None` when it is already inside. For 383.9 ms that is `300.30 / 266.84` = **1.13**, a 1014 ms
  backswing, offered as an opt-in "snap" button beside the pace slider.

*"A wrong verdict is read once, and a wrong metronome is rehearsed"* still holds; the answer is now
the golfer's rather than the server's. A mis-segmented 60 ms downswing is outside p10–p90, so the
page says so and offers the snap, instead of silently substituting a median and calling it a fit.

### Three consequences, recorded rather than argued again later

**`_grid_pattern`'s rounding argument inverts while its code does not.** It absorbs the integer tick
rounding in the backswing and keeps the downswing exact — for the reason the 2026-08-20 addendum
gives, that the downswing is the near-invariant half and the quantity being corrected. That reason
is now the opposite one: the downswing is the *anchor* and must land on the golfer's own, and the
backswing is the prescription. Same behaviour, opposite argument; the docstring is rewritten so a
later reader does not "fix" it back.

**The pace control has to reach further than the old guard allowed.** `static/tempo.js`'s
`min="70" max="140"` were the backswing guard's edges expressed against the median. The fitted pace
here is 144% — off the end, and a silent clamp plays a tempo other than the one the page's own text
claims. The new bounds come from the widest downswing the segmenter admits, `POSSIBLE_DOWNSWING_S =
(0.12, 0.80)` at `analysis/phases.py`, against `downswing.p50`: **44% to 300%**. That constant is
promoted and read, not copied.

**The career anchor unlocks earlier, and that is a consequence rather than a goal.**
`METRIC_MINIMUM_N` overrides `tempo_ratio` to `CENTER: 8` because it is the noisiest metric in the
panel; `downswing_ms` carries no override and gates at the default 5. A career target therefore
becomes claimable at fewer swings than before. Nobody has measured a downswing-specific minimum, so
it is written down here rather than fixed by inventing an override.

### One risk this does not fix

**The re-expressed message is frame-rate dependent where the score is not.** `tempo_ratio` cancels
fps; an absolute duration does not, and `phases` carries milliseconds derived from the clip's own
frame rate, so a slow-motion clip prints an inflated downswing and an inflated backswing target. The
notice above is what catches it — a 3000 ms downswing reads "outside the tour range" — but it is a
notice and not a gate. The fix is a capture-side fps sanity check, which is a different decision from
this one.

### Where it is built

Eight phases in [docs/M13_DOWNSWING_TEMPO.md](../M13_DOWNSWING_TEMPO.md), of which this addendum is
part of P0. The contract fields land in `contracts/tempo.py`, the flip is one divisor and one
deleted guard in `analysis/tempo_trainer.py`, and the verdict's new sentence is three strings in
`analysis/checkpoints/mechanics.py`. Nothing in `ranges.json`, `CHECKPOINT_REGISTRY`,
`contracts/caveats.py` or either benchmark artifact is touched — a phase that finds itself editing
one of those has grown a band and should stop for a decision.
