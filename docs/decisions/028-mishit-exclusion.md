# ADR-028: Mishit exclusion — the topped shot that is not your seven iron

## Status
**Accepted** 2026-09-07, **built** 2026-09-08 — [ROADMAP §M16](../../ROADMAP.md), nine phases
(P0–P8) on branch `GOLF-6`, one commit each, the full suite / `ruff` / `mypy` green at every one.
**Building corrected nothing here**: `MISHIT_EXCLUDED_METRICS`, the `0.50 × median` floor gated at
five clean carries, the `CorpusSwing.artifact_key` chokepoint, `read_corpus`'s `_flag_auto_mishits`
before `count_metrics`, the `SwingManifest.mishit` override winning both ways, and the metric scope
all shipped as §Decision describes them — so there is no addendum below.

What the corpus added is one real subject the §Context example only imagined. P5's
`scripts/flag_mishit.py --list` found aaron's 7 iron carrying five tagged shots — 121.8, **33.6**,
124.7, 126.1, 101.9 yd — with `2026-08-23/2` a genuine 33.6-yd top, auto-flagged against a floor of
60.9. It is the only mishit on disk. The case is *tighter* than the exit fixture (five total, not
five clean plus a top): the exclusion leaves four clean carries, which is **below** the CENTER-claim
floor the same constant sets, so `get_club_profile` stops reporting that club's `carry_distance_yds`
and `total_distance_yds` altogether rather than meaning over four. Where it used to print ≈101.6 yd —
a top-dragged number — it now says five samples are needed and there are four. That is §Decision 1
resolving into ADR-010 §2 ("no score beats a wrong one") on real data, which is the outcome this
milestone wanted. `ball_speed_mph` and `backswing_ms` for that club still count all five.

## Date
2026-09-07

## Context

Career mode and the per-club profile (ADR-024) answer "how far do I hit my 7 iron" by pooling every
tagged shot's `carry_distance` and taking a guarded mean. The pooling is honest about sample size
and blind to sample quality: a topped 7 iron that carries 20 yards is one of the five shots the
mean is built from, counted exactly like the four real ones, and it drags "your 7 iron carries 150"
down by six yards on its own.

Nothing in the repo filters a shot on quality today. `CorpusSwing.artifact_key` withholds a
launch-monitor sample when the OCR parse was flagged (ADR-014), `get_session_summary` skips the
same, and `CareerCorpus.excluded` itemises swings that are stale, unattributed or unanalysed —
every one of those is a *data-integrity* judgment, never a judgment about the swing.

**The obvious automatic check does not exist, and cannot be built from the flight model.** A genuine
top prints a low ball speed *and* a short carry, and the two agree: `analysis/spin_solve.py`'s
`BELOW_FLOOR` case and the `carry_unreachable` refusal fire when a printed carry is impossible *for
its launch conditions*, and a topped shot's launch conditions are entirely consistent with 20
yards. The flight model catches OCR misreads, not mishits. The only signals that a 20-yard 7 iron
was a mistake rather than the shot the golfer meant are: it is far shorter than that golfer's *own*
7 irons; the simulator's `Impact Position` tile said so; or the golfer says so.

**"Miss" is already taken.** `analysis/dispersion.py` and `contracts/dispersion.py` use *miss* and
*the shape of a miss* to mean the golfer's error distribution — a repeatable (biased) miss versus a
scattered one. A second "miss = mishit" meaning would collide with that vocabulary in field names
and in prose. This milestone uses **mishit** throughout.

## Options Considered

### Option A: A caveat, and remove nothing
`analysis/club_profile.py::_bag_changed_caveats` is the precedent — when some of a club's shots
predate its bag entry, it *adds a sentence and removes no samples*. A mishit caveat would say "one
of these shots looks like a top" and leave the mean alone.
- **Pros**: no sample is ever wrongly deleted; matches the repo's lightest-touch posture; no new
  store, no write surface.
- **Cons**: the headline number stays wrong. A bag-changed caveat names a *date* a reader can
  reason about ("shots before 2026-08-01 were a different shaft"); "one of these five shots is a
  top" gives a reader nothing to act on, and they cannot mentally subtract an unknown quantity from
  a mean. The wrong mean is what every downstream surface reads and quotes.

### Option B: Auto-detect, exclude, report, and let the golfer override *(chosen)*
Flag a shot whose carry is below half that club's median; keep it out of the carry and
total-distance pool only; count it, name it, and caveat it; let the golfer confirm or clear any
shot, with the manual verdict winning.
- **Pros**: the mean reflects real strikes; the exclusion is fully visible (`mishit_refs` names
  every held-out shot) and reversible either way; detection needs no tour data, only the golfer's
  own history.
- **Cons**: it *removes samples*, which is a stronger move than anything career mode currently
  makes — so it needs this ADR to authorise it, and it needs the reporting to be load-bearing
  rather than decorative. A borderline real shot near the floor could be flagged; the `cleared`
  verdict is the answer.

### Option C: Manual tag only
No automatic detection; the golfer marks each mishit by hand.
- **Pros**: nothing is ever auto-deleted; simplest to reason about.
- **Cons**: a bay session is 20–30 shots per bucket, and tagging each top by hand is exactly the
  bookkeeping the golfer asked the program to do for them. A topped shot that no one remembers to
  tag stays in the average.

### Option D: A per-club "should carry" band
Give each club a target carry and flag anything far below it.
- **Pros**: a single absolute rule.
- **Cons**: rejected for the reason `contracts/dispersion.py::METRIC_TARGETS` already gives
  `carry_distance_yds` no target — "how far a golfer should hit a given club is not a number this
  repo has" (ADR-010 §2, ADR-024 *Deferred*). GolfDB carries no ball flight, and a tour carry band
  would judge an amateur against a population they are not in. Detection is relative to the
  golfer's own club median, or it does not happen.

## Decision

### 1. A mishit is excluded from carry and total distance only

`carry_distance_yds` and `total_distance_yds` — the frozenset `MISHIT_EXCLUDED_METRICS` in
`contracts/mishit.py`. Ball speed, launch angle, start-line offline, face-to-path, and every pose
and mechanics checkpoint still count the shot. A topped swing is a real swing to score on tempo and
sway; only its distance is meaningless. This is the `untagged_swings` lesson (ADR-024 Consequences)
applied one level in: an exclusion that is right for one metric is not licence to drop the shot
from the others.

### 2. Detection is auto plus a manual override, and the override wins

**Auto.** In `storage/corpus.py::read_corpus`, after the swing list is built and before
`count_metrics`, group the golfer's shots by club and compute `floor = 0.50 × median(that club's
pooled carries)`. Any shot below the floor is flagged. Gated on `MISHIT_MIN_CLEAN_SHOTS = 5`
distinct carry samples for the club — the same floor `contracts/baseline.py` sets for a CENTER
claim, and for the same reason: below it there is no established distance to be an outlier of. The
`0.50` is a documented judgment with a provenance string, deliberately loose — half a club's own
median is a topped or bladed shot, not the low edge of dispersion — and it is the one number in
this milestone expected to be revised from a bay session with labelled tops.

**Manual.** `SwingManifest.mishit: MishitVerdict | None` — `CONFIRMED` or `CLEARED`, set through
`POST /api/sessions/{session}/swings/{swing}/mishit` or `scripts/flag_mishit.py`, `None` by
default. It is the `club` field's pattern exactly (ADR-024 §5): optional in the shape, tolerantly
loaded, written only by an explicit per-swing repair route, and with **no bulk backfill** — only
the golfer who hit swing 3 knows whether they topped it.

**Combined.** `CorpusSwing.is_mishit`: `CLEARED → False`, `CONFIRMED → True`, otherwise the auto
flag. Three visible states — auto-flagged-and-unconfirmed, confirmed, cleared.

### 3. The exclusion happens at one chokepoint, and is metric-scoped

`CorpusSwing.artifact_key(measurement)` already returns `None` to keep a flagged-OCR shot out of
launch-monitor pooling. It gains one clause: return `None` when `measurement.name in
MISHIT_EXCLUDED_METRICS and self.is_mishit`. Because `count_metrics` and
`analysis/baseline.py::pooled_samples` both route through `artifact_key`, the printed `n` and the
pooled `n` cannot disagree — the invariant `tests/storage/test_corpus.py` pins.
`analysis/club_profile.py` and `mcp/career.py` inherit the exclusion with no change, because they
all build on `read_corpus`.

`get_session_summary` is the one surface that acts on the **manual** verdict only, not the auto
flag: a single session rarely holds five shots of one club, so it has no distribution to detect an
outlier against, and "how did I hit it today" is a lighter read than the considered career
statistic. Documented asymmetry, not an oversight.

### 4. Every excluded shot is counted and named

`ClubProfile` gains `mishits`, `mishit_refs` and `mishits_unconfirmed`; `BagProfile` and
`GolferProfile` gain `mishits_excluded`; `SessionDetail` gains `mishits_excluded`;
`analysis/club_profile.py` grows a `_mishit_caveats` helper shaped like `_bag_changed_caveats`;
`contracts/caveats.py` gains a derived bullet whose metric names come from `MISHIT_EXCLUDED_METRICS`
so the prose cannot drift from the code. "The exclusions are output, not control flow"
(`storage/corpus.py`) — a mean built from fewer samples than the shot count says so, in the voice
the golfer reads.

### 5. `ANALYSIS_VERSION` does not move

Every `measurements` entry in every `analysis.json` is byte-identical after this change;
`overall_score`, `checkpoint_scores` and `unscored` are untouched. Club, bag and career aggregates
are computed live from the corpus at read time and have never been versioned. This is ADR-010's
2026-08-19 posture (an `unscored` reason changed what a refusal *says*, no bump), not M15 P13's
(four new measurements the artifact did not carry, so `is_outdated` could not see they were missing
— a bump was owed). Here a version-15 `analysis.json` is complete and correct; nothing is added to
it. `scripts/reanalyze.py` has nothing to redo.

## Consequences

- **This is an L3 change.** `contracts/mishit.py` is new; `CorpusSwing`, `CareerCorpus`,
  `ClubProfile`, `BagProfile` and `SwingManifest` all change shape. Every consumer of those shapes
  was walked during planning.
- **A new write route and a new CLI.** `POST …/mishit` joins the per-swing club-retag route as the
  second explicit-repair endpoint; `scripts/flag_mishit.py` joins `scripts/backfill_golfer.py` as a
  corpus-repair CLI. Both land in `docs/ARCHITECTURE.md` §1.
- **Bulk-imported bare shots are out of scope, by construction.** A shot in `data/processed/shots/`
  with no swing bundle has no manifest and no club, so it is in no `CareerCorpus` and no per-club
  average regardless of any verdict — the same situation ADR-024 established for untagged swings.
  If it is later joined into a tagged bundle, `set_mishit` applies then.
- **The auto rule is inert on a club whose majority of shots are tops.** A low median makes `0.50
  ×` it flag nothing. Accepted: the rule removes only shots no average should contain, and never a
  real one; a golfer hitting mostly tops has a coaching problem the mean should reflect.
- **A borderline real shot near the floor can be auto-flagged.** `mishits_unconfirmed` surfaces
  exactly these, and one `scripts/flag_mishit.py --clear` returns the shot to the average.

## Deferred, by choice

- **A content-addressed override store keyed on the shot photo's `image_sha256`.** Better grain — a
  mishit is a property of the ball flight, which is what carry dedupes on — and it would let a bare
  imported shot be annotated and reported uniformly in `get_shot_by_id`. Rejected for v1: a new
  store plus a read-path signature change through four `mcp/query.py` functions, for shots that
  influence no average.
- **A per-club floor fraction.** `0.50` is one constant for every club. A driver's mishit and a lob
  wedge's may not sit at the same fraction of median. One number until a bay session says otherwise
  — the same posture ADR-024 took on the offline tolerance.
- **`impact_position` as a detection input.** The HD Golf `Impact Position` tile is parsed and
  stored and currently read by nothing. Using "THIN" / "FAT" / "TOE" as a mishit signal is real and
  deferred: it wants a bay session to see what the tile actually prints and how often.
