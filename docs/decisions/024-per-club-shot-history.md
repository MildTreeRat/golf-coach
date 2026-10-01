# ADR-024: Per-Club Shot History — the tag that makes distance a measurable quantity

## Status
**Accepted** 2026-08-22. Design agreed 2026-08-20; all 20 phases of
[docs/M9_PLAYER_TRACKING.md](../M9_PLAYER_TRACKING.md) are built, and this document is the *why*
behind them. Nothing in the four decisions below was corrected by building it, which is why this
flip carries no addendum — the one thing reality did add is on the shelf question, and it has
its own addendum below from 2026-08-21.

## Date
2026-08-20

## Context

Career mode answered "how does this golfer compare to their own history" and it works: the corpus
reader counts an honest `n`, `analysis/baseline.py` refuses a claim the data cannot support, and
`analysis/dispersion.py` separates a repeatable miss from a scattered one. All of it is built and
all of it is silent, waiting on `n`.

None of it can answer the first question a golfer actually asks: **how far do I hit my 7 iron.**

Not for want of statistics. `ShotData` has carried `carry_distance` since ADR-004 and the HD Golf
screen prints it on every shot. The gap is that **no shot on disk records which club hit it**, and
without that a carry distance is not a poolable quantity. A mean over a driver and a sand wedge
describes nobody's swing; it is not a noisy estimate of something real, it is an average of two
different questions. `analysis/shot_measure.py` is why carry was never registered as a measurement
in M6.5 despite being the most obviously useful number on the screen.

So the missing thing is one field, and everything downstream of it already exists:

| Needed | Already built |
|---|---|
| Per-golfer identity | `contracts/golfer.py`, `storage/golfer_store.py` (ADR-008, career step 1) |
| Per-golfer swing corpus with an honest `n` | `storage/corpus.py` -> `CareerCorpus` (career step 2) |
| Filtering that corpus with counts recomputed | `storage.corpus.narrow_to` (career step 6) |
| Mean / sd / CI behind a minimum-`n` guard | `analysis/baseline.py` (career step 4) |
| Repeatable-miss vs scattered-miss | `analysis/dispersion.py` (career step 5) |
| Launch-monitor to measurement registry | `analysis/shot_measure.py` `SHOT_MEASUREMENTS` |
| Capture-time metadata, tolerantly loaded | `SwingManifest.player_id`, `storage/session_meta.py` |

That table is the argument for doing this now. It is mostly wiring, and the statistics it wires
into are the ones already written and already validated.

### Why the club cannot be detected

The obvious alternative to asking is measuring, and it is closed on two independent counts. The HD
Golf screen prints no club tile — `launch_monitor/screen/profiles.json` enumerates every label the
device shows and there is nothing naming the club. And detecting it from video is M2, which
[ADR-017](017-club-head-detection-strategy.md) put behind a ~1/2000 s exposure the bay cannot
currently deliver, and which M1.5 returned a no-go on. A tag typed by a human is not a fallback
here; it is the only source that exists.

## Decision

### 1. The club is a specific club, and the category is derived

The tag is `7i`, not `mid_iron`. `ClubCategory` already exists in `contracts/intent.py` and keys
benchmark rows, and it is the wrong grain for this: "mid iron" pools a 6 and an 8, so "my 7 iron"
stops being expressible — which is the entire question.

A new `contracts/club.py` holds `ClubId` (the taxonomy) and `category_of()` (the derived mapping
onto `ClubCategory`), so band lookup keeps working with no change and there is one table rather
than two vocabularies that can disagree.

Free text was rejected. `slugify` in `contracts/golfer.py` exists because "Aaron" and "aaron"
splitting one golfer's baseline in two is a silent, undetectable failure; "7i" / "7 iron" /
"seven" splitting one club's carry average three ways is the same failure with the same
invisibility. A closed enum plus one tolerant `parse_club` at the boundary is the same posture,
applied to the same problem.

### 2. Loft belongs to the physical club, not to the club id

Two 7 irons have different lofts, and one golfer's changes when it is bent or replaced. So `7i` is
a **slot**, and the thing that carries loft is a **bag entry** — a per-golfer record of the club
that currently occupies that slot, with its loft, make, model, shaft and length.

Naming the slot by its loft (`52`, `56` for wedges) was rejected for the same reason: it puts a
measurement inside an identifier, and then a re-grind renames the club and orphans its history.

The bag is explicit and declared. "Clubs used" stays **derived** from the shot history — always
true, no upkeep — and the two are different questions that both have answers: a club in the bag
you have not hit has no statistics and is not an error, and a club you have hit that has left the
bag still has real history.

`loft_deg` is optional. A golfer who has never measured their lofts still has a bag, and refusing
to record one until they do would mean recording nothing. The work that needs loft refuses per
club when it is absent — the same posture as `SwingResult.unscored` (ADR-010 §2).

### 3. Lateral miss ships as a start-line projection, and is named as one

The ask was "how many yards left or right does it tend to go". The screen prints **no offline
tile** — no side, no deviation, no landing coordinate. What it prints is `Horizontal Angle`, the
initial launch direction in degrees, which `analysis/shot_measure.py` already records as
`start_line_deg`.

So M9 records `start_line_offline_yds = carry * sin(start_line_deg)`: **where the ball would have
landed if it never curved.** That is exact trigonometry over two printed numbers, with no physics
invented and no parameter fitted.

What it is not, stated so it cannot be misread: it is not the landing point. A golfer who starts
it straight and slices 30 yards reads about 0 here, and the whole of that miss lives in
`face_to_path_deg`. The two must be read together, and any prose rendering this must say
*started* and never *finished*.

A real ball-flight model — launch, spin and spin axis integrated to a landing point — is the
honest way to get true offline, and it is deferred rather than rejected. It is blocked on
`spin_axis`, whose sign this repo has already been burned by:
[ADR-014's addendum](014-screen-capture-shot-ingestion.md) records both stored shots being fades
saved as draws, and `shot_measure.py` still refuses to record the field because the screen prints
a magnitude with no direction word. Building a flight model on top of that is a wrong number with
a provenance string attached, which is the exact thing ADR-010 §2 exists to prevent.

If the simulator can be configured to print an offline tile — [ROADMAP](../../ROADMAP.md) already
notes M3's remaining OCR work is enumerating what the screen can be made to show — that is a
one-row addition to `profiles.json`, and the measured value should **supersede** this derived one
rather than sit beside it.

### 4. Per-club statistics reuse the career pipeline unchanged

`storage.corpus.narrow_to` already filters a `CareerCorpus` by time window or session **and
recomputes `metric_counts`**, precisely so a filtered swing list can never sit beside an `n` that
describes a different set. Adding a `club=` clause to it means the whole of career mode's
machinery — the pooling, the artifact-keyed dedupe, the confidence intervals, the minimum-`n`
guard, the bias/scatter discriminator — produces per-club answers with nothing new learning the
rules.

**The guard applying per club is the point, not a side effect.** "Your 7 iron carries 164 yards"
needs five distinct 7-iron shots, not five shots. A golfer with 40 shots across 9 clubs has
almost nothing established, and the bag page saying so is the feature working. This is the same
acceptance criterion career mode shipped under: the correct output today is refusal, and a number
appearing early is the bug.

### 5. A club is required at upload, and read from the session cursor

Unlike the golfer, the club is **required**: `POST /api/uploads` refuses with 409 when no club is
selected. The asymmetry is deliberate and it is about repairability. An untagged golfer is fixable
later — `attribute_unlabeled` and `scripts/backfill_golfer.py` exist for exactly that, and a
session usually has one golfer so reaching backwards is safe. An untagged club is *not* recoverable
after the fact by anything except memory, and a mistagged one silently pools a wedge into a 7
iron's carry average.

But the value is read from a **server-side session cursor**, never from the request body.
`api/app.py` already states the reason for `player_id` and it applies verbatim: *"both phones post
into the same swing, and only one of them is being held by someone who knows whose swing it is."*
Two phones holding two `localStorage` club values would disagree, and the disagreement would land
in the manifest.

**No bulk backfill for club.** `attribute_unlabeled` reaches backwards over a session's unlabeled
swings because a session usually has one golfer. A session has *many* clubs, so the same reach
would confidently mislabel every earlier swing. Club gets a per-swing repair route and nothing
else.

## Consequences

- `SwingManifest` gains an optional `club`, added the way `player_id` was: defaulted, read through
  the same tolerant loader, so manifests written before it existed still load and report `None`.
  **Optional in the shape, required at the boundary** — the requiredness lives in the route
  (CODE_STANDARDS R12), which is what lets the swings already on disk keep loading.
- Every swing currently on disk is untagged, and therefore contributes to no club's statistics.
  They are counted (`CareerCorpus.untagged_swings`) rather than excluded — an untagged swing is
  still a perfectly good contributor to every pose metric and must not shrink the mechanics `n`.
- Carry, total distance and the derived offline enter `SHOT_MEASUREMENTS`, so they reach
  `analysis.json` through the existing engine walk with `source: "launch_monitor:hd_golf"` and
  dedupe on the shot photo's hash like the two metrics already there.
- `contracts/dispersion.py`'s `METRIC_TARGETS` gains rows for them. Without a registered
  tolerance `target_for` returns `None` and both findings are refused, so a measurement with no
  target entry is measured and permanently silent.
- The offline tolerance is a single constant and that is **known to be the wrong shape**: offline
  error scales with carry, so the correct tolerance is per club. It is set at the widest club in
  the bag, which errs wide — the direction `_JUDGED_DEGREES` already argues for, since erring wide
  costs claims where erring narrow buys confident claims about the simulator's own noise.

## Deferred, by choice

- **A ball-flight model** for true landing offset including curve. Blocked on `spin_axis` (§3) and
  wants a bay session's repeats to validate against.
- **Per-club tolerances** for `start_line_offline_yds` (see Consequences).
- **Club fitting** — the reason loft, ball speed and launch angle are recorded now. It needs a
  gapping model and a launch-optimisation model, neither of which has data behind it. Recording
  the inputs is unrecoverable after the fact; the models are not, so the inputs land now and the
  models wait.
- **Per-club benchmark bands.** `ranges.json` carries `club_category: "all"` rows only, and
  [ADR-010](010-benchmark-ranges.md) already gated per-club bands and cut none — the club is not
  an axis this panel varies on. [ADR-023](023-tempo-training-and-absolute-swing-durations.md)'s
  addendum reached the same conclusion from the other direction. This is not obviously worth doing
  even when it becomes possible.
- **Bag entry versioning.** A changed bag entry produces a caveat naming the date rather than a
  modelled history. Same posture as `SESSION_DRIFT_FACTOR`: a loose judgment that only ever adds a
  sentence and never removes a claim.

## Alternatives Considered

**Tag with `ClubCategory` only.** Reuses an existing enum and adds no contract. Rejected: it
cannot answer "my 7 iron", which is the question, and it leaves club fitting with no anchor.

**Free-text club names.** Rejected — see §1; it is `slugify`'s problem again, and silent.

**Detect the club from video or the screen.** Rejected as unavailable, not as undesirable: no
screen tile, and M2 is gated behind an exposure the bay cannot deliver (ADR-017, ADR-018).

**Store loft on the shot.** Would make each shot self-describing. Rejected: loft is a property of
a club that persists across thousands of shots, and copying it onto every one of them is a second
home for a value that can drift from the first (CODE_STANDARDS R4). The bag entry is the single
source; a change to it is caveated rather than duplicated.

**Wire the club into `resolve_range` / `PracticeGoal.club`.** Looks like the obvious payoff of
having a club tag, and it is a trap: `ranges.json` holds `club_category: "all"` rows only, so
passing a real category makes every checkpoint resolve no band and the fundamentals panel goes
dark. `PracticeGoal.club` stays `ALL`. Recorded here because a future session will find that
parameter and think it was an oversight.

**Block on a bay session first, as career mode did.** Rejected: career mode was deferred because
it needed `n` to be *correct*, and no amount of desk work produced data. M9's ingest half needs no
`n` at all — it is what makes the next bay session's data worth more than the last one's, so
building it before the session is the ordering that pays.

## Addendum (2026-08-21): a club that leaves the bag is kept, and that is not versioning

**What changed.** `Bag` gained `retired: tuple[BagEntry, ...]` — an append-only shelf of finished
stints — and `BagEntry` gained `retired_at`. `BagStore.set_entry` moves the outgoing entry there
instead of dropping it, `remove_entry` shelves rather than deletes, and `restore_entry` puts back
the club a slot held previously. Landed with P3.

**Why the original decision was not enough.** §2 says the bag entry is the single home for loft
and that a change to it is caveated rather than duplicated. Both still hold. What §2 did not say
is what happens to the *old* entry at the moment of the change, and the implicit answer — it is
overwritten — loses the one input this milestone exists to capture. A golfer who puts last
season's 7 iron back in the bag would have to re-measure a loft that had already been measured,
and "unrecoverable after the fact" was the entire argument for recording lofts before the models
that use them exist. Deleting them on replacement reintroduces exactly that loss, one club at a
time, through the normal use of the feature.

**The boundary, because these two are easy to confuse.** *Deferred, by choice* defers **bag entry
versioning**: attributing each stored shot to the stint that hit it. That stays deferred, and
nothing in P3 moves toward it. `Bag.retired` has no reader — P16 still builds its caveat from the
**current** entry's `recorded_at`, naming a date rather than modelling a history, exactly as that
bullet says. The shelf is retention: it keeps a declaration from being destroyed and makes
re-declaring a club one call. It makes the deferred modelling *possible* later without promising
it now.

The test for whether this addendum has gone stale is one line: if anything joins a shot to a
member of `Bag.retired`, versioning has happened and it needs a decision here rather than an
addendum.

**Two smaller calls that follow from it.**

- **Re-saving an unchanged entry writes nothing and does not move `recorded_at`.** Identity is
  `BagEntry.same_club_as`, which compares every descriptive field and neither timestamp. Without
  this, P19's save button on an unedited row retires a club and hands P16 a bag-changed caveat
  over shots that were all hit with the same club — the false positive is produced by the UI
  working correctly, which is the worst way to get one.
- **A restore copies off the shelf rather than popping.** Out, back and out again is three stints
  and not one overwritten record. Popping would make a club's second departure look like its
  first, which is the history the shelf exists to hold.

**And one guard the shelf made necessary.** `BagStore.get` is tolerant — a corrupt bag reads as
`None`, as `GolferStore.get` does. The writers deliberately are not: they read through a helper
that distinguishes "no bag yet" from "bag I cannot parse" and raises on the second. A writer that
treated an unreadable file as an empty bag would replace the bag *and its entire shelf* with the
single club it was asked to set. That failure existed before the shelf and got materially worse
with it, since the shelf is the part that was meant to survive replacement.

## Addendum (2026-08-31): loft is the manufacturer's number, and "never a catalogue default" is retired

**What changed.** §2's sentence *"`loft_deg` is optional… the one that does not is named rather
than defaulted to a book value nobody measured"* — and the field description it produced,
*"Measured loft. None means unmeasured, and never a catalogue default"* (`contracts/bag.py:74`) —
is **reversed**. `loft_deg` now means the manufacturer's published loft for that slot of that
model, filled by a lookup and editable in the form.
[ADR-026](026-club-specification-lookup.md) §1 is the decision and carries its reasoning; this note
exists so §2 is not read as current.

**Why the original call was right and stopped being right.** It applied
[ADR-010](010-benchmark-ranges.md) §2 correctly: a catalogue number standing in silently for a
measurement is a wrong number wearing a right one's clothes. What it got wrong was the comparison.
§2 assumed the choice was *book loft versus measured loft*, and for a golfer with no loft machine —
which is this golfer — the choice is **book loft versus nothing**. Nothing has won for a full
milestone: the write route, the store and the row form all shipped with M9 and
`data/processed/golfers/` still contains no `.bag.json`. Epistemic caution about a field in a record
that does not exist protects nobody.

**What stays true.** Loft is still a property of the *physical club* and not of the `ClubId`, the
bag is still the single home for it, and a slot is still never named by its loft. §2's actual
decision is untouched; only its rule about where the number may come from is retired.

**The cost, on the record.** A club bent 2° strong reads its book loft and nothing downstream
knows. That is accepted rather than solved: the field is editable, so the golfer who had it bent can
correct it, and a separate measured-loft field is deferred to the day someone wants the difference
modelled instead (ADR-026, *Deferred*).

**And one thing this addendum obliges the code to do.** ADR-026 widens `BagEntry` with the rest of
the specification, and one of the new fields is a provenance block carrying a retrieval timestamp.
`same_club_as` compares by *exclusion* — a design this ADR's first addendum praised, because a field
added later is compared from the day it is added — so that timestamp would count as club identity,
and re-looking-up an unchanged club would retire it and produce exactly the false bag-changed caveat
the first addendum's *"Two smaller calls"* bullet exists to prevent. The provenance is a **third
timestamp**, and it must be excluded alongside `recorded_at` and `retired_at`.

## Addendum (2026-09-30, M31): the shot is the unit, a drill is not tracked, and the landing offline is a second projection

[ADR-034](034-shot-first-phone-first.md) makes the launch-monitor shot the product, graded per club
and per player. This ADR is what made a shot poolable per club: the club tag, the bag, the
start-line projection and the reuse of career mode's statistics. Its five decisions stand as the
addenda above left them. This addendum records what the pivot adds beside them, each point routed
to an ADR-034 clause. **Nothing here is built**; M35, M36 and M37 build it.

**The shot is the unit, and a photo with no video is admitted**
([clause 1](034-shot-first-phone-first.md#1-the-unit-of-the-product-is-the-shot)). §4's pipeline
reaches a shot only through a swing today. `read_corpus` excludes a manifest with no face-on clip
(`ExclusionReason.NO_FACE_ON`), even though `CorpusSwing.artifact_key` already keys a launch-monitor
sample on its photo, as `shot:{sha}`
([M31 P2](../plans/m31-shot-first-adr.md#p2--found-2026-09-29), 2026-09-29, confirming
[program-plan finding 2](../plans/m31-m40-shot-first-pivot.md#what-the-code-says-before-anyone-re-derives-it)).
[M35](../plans/m31-m40-shot-first-pivot.md#m35--shot-first-sessions-in-rust)
admits photo-only entries, deduped by photo hash, so the per-club statistics §4 built reach every
photographed shot and not only the filmed ones. §4's reuse is otherwise unchanged, and M36 ports the
corpus, baseline and dispersion to Rust with Python as the oracle.

**§5 carries to the phone: the club comes from the session cursor.** The golfer picks the club
before the camera opens, and each photo takes it from the session (M38 P1–P2). §5 gives two
reasons, and only one of them moves with it:

- the cursor is server-side because two phones posting into one swing would disagree. On a phone
  that is the only device, that cannot arise, and the cursor is simply the session's state;
- the club is required because an untagged club is not recoverable after the fact, and a mistagged
  one pools a wedge into a 7 iron. That holds on the phone unchanged, and it is why the club is
  picked before the first photo rather than after.

"No bulk backfill for club" holds as written.

- M35 records a `PracticeGoal` per shot, and its `club` stays `ALL`, for the reason *Alternatives
  Considered* gives. The shot's club is the `ClubId` tag. `PracticeGoal.club` is a band-lookup key
  that `ranges.json` has no rows for, and the pivot gives it no new job.

**A drill is not tracked, and that is neither untagged nor a mishit**
([clause 3](034-shot-first-phone-first.md#3-tracked-shots-are-derived-from-intent-and-a-drill-is-not-tracked)).
A `DRILL` shot never enters a per-club or a per-player aggregate. It is still stored, shown in
history and analysed on its own. Three exclusions now touch §4's statistics, and they differ in
where they come from and what they take:

| Exclusion | Comes from | What it takes |
|---|---|---|
| Untagged (Consequences above) | no club was recorded | the club's stats, and nothing else. It is counted, and still feeds every pose metric |
| Mishit ([ADR-028](028-mishit-exclusion.md)) | the shot's own numbers, automatically | carry and total only |
| Untracked (`DRILL`, clause 3) | the golfer's intent, derived from `PracticeGoal.mode` and never stored | the whole shot, from every aggregate |

ADR-028's rule is unchanged, and so is the untagged count.

**Two levels, and which stats pool across clubs**
([clause 4](034-shot-first-phone-first.md#4-two-levels-club-and-player)). §4's per-club answers are
the **club** level. The **player** level is new, and it has two routes:

- **raw averages**, only for club-independent stats: strike location, face-to-path, start line and
  the consistency rates. A player-level carry average would be the Context's mean over a driver and
  a sand wedge, which this ADR opened by refusing;
- **topic grades**, which pool across clubs by judging each shot against its own club's criterion
  first ([5.4](034-shot-first-phone-first.md#54-two-blends-with-an-honest-pooling-rule)), so
  judgments pool where raw values cannot.

Two things this ADR already said bear on it:

- **§4's per-club guard is the posture of 5.1's minimum n.** "The guard applying per club is the
  point" means a topic grade is withheld per club, not per golfer
  ([5.1](034-shot-first-phone-first.md#51-a-topics-grade-is-a-share-of-good-shots)). The number is
  M37's; the posture is §4's.
- **The offline in yards is club-scoped by this ADR's own reasoning.** Clause 4 lists start line
  (degrees) as club-independent. `start_line_offline_yds` scales with carry, which is why the
  Consequences call its single tolerance "known to be the wrong shape". It reads as club-only, and
  that reason is on the record here for M37's per-stat scope to use.

**The landing offline is a second projection, and §3 is its precedent**
([5.3](034-shot-first-phone-first.md#53-shot-shapes)). The shape topics' "finished on line" share
needs where the ball *landed*. HD Golf prints no offline tile, so ADR-034 projects it, including the
curve, as its one sanctioned inference. §3's rule governs how: **a projection ships named as one**.
§3's "*started* and never *finished*" gains its mirror, "projected" and never "measured".

- It is a **second** projection beside §3's, not a replacement. §3's is exact trigonometry over two
  printed numbers, and it assumes no curve. The new one flies the shot through
  [ADR-027](027-ball-flight-simulation.md)'s model, which lifted this ADR's deferred ball-flight
  model, and it **includes the curve**. That is why §3's number cannot stand in for it: a well-hit
  fade starts away from the target on purpose, so a start-line projection marks it offline.
- ADR-027 records it as `flight_landing_offline_yds`, and only when the curve was actually drawn.
  The curve reaches the flight only through a printed spin axis, so the projection reaches 2 of the
  13 stored shots, and none of the 11 from 2026-08-23 (M31 P2, 2026-09-29). Where it cannot be
  computed, the on-line share is withheld and named.
- **§3's last paragraph is 5.3's "printed replaces projected".** §3 says a printed offline "should
  **supersede** this derived one rather than sit beside it". If the simulator can be configured to
  print an offline tile, the printed value supersedes both projections for this device.
- **The on-line tolerance has a reason on the record here.** 5.3 takes `METRIC_TARGETS`' offline
  row "unless M37 finds a reason otherwise". The Consequences above already give one: that row is a
  single constant set at the widest club, known to be the wrong shape, with per-club tolerances
  deferred. They call erring wide the safe direction, and for a claim it is, because it costs
  claims. For a share it is not: a wider tolerance counts more shots on line, so it flatters the
  grade, most for the shortest clubs. M37 decides; this addendum only records that the argument
  for erring wide does not carry over.

**Not changed**:

- §1–§5, and both addenda above;
- `start_line_offline_yds` and "*started* and never *finished*". The new projection sits beside it
  and changes neither its formula nor its name;
- *Deferred*'s per-club benchmark bands stay uncut. ADR-010's addendum of this date says the shot
  grades need none;
- bag entry versioning stays deferred, and the first addendum's one-line test for it still applies.

## Addendum (2026-09-30, M31.5): M36 ports the corpus first, and M35 admits photo-only shots in Rust

[ADR-035](035-rust-everywhere-python-where-required.md) makes Rust the oracle from M32 and puts M36
before M35 ([clause 6](035-rust-everywhere-python-where-required.md#6-order-the-phone-path-first)).
The previous addendum routes its work to both milestones, and each routing sentence names Python.
This addendum records what each now means. **Nothing here is built.**

**"M36 ports the corpus, baseline and dispersion to Rust with Python as the oracle" holds, once.**
M36 ports §4's pipeline faithfully, including `read_corpus` exactly as it behaves today, with no
photo-only admission. Frozen Python records that behaviour a single time, before the port moves,
because it is the only reference that is independent of the port (Q7 of
[M31.5 P2](../plans/m31-5-rust-first-replan.md#p2--found-2026-09-30), and
[clause 3](035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)). After that,
Rust re-records, and every re-record is diff-gated.

**M35 is Rust, and it comes after M36.** The previous addendum's link names M35 "Python, which is
M36's oracle", and Q4 reversed that order. A Rust M35 needs the corpus and the stores that M36
builds, so M35 depends on M36 (P1 finding 7). What M35 does is unchanged, and it now does it in Rust
with hand-worked vectors:

- It admits photo-only entries to the corpus, deduped by photo hash, so that §4's statistics reach
  every photographed shot and not only the filmed ones.
- It records a `PracticeGoal` per shot, whose `club` stays `ALL` for the reason the previous addendum
  gives.
- It builds `shot_result`, which moved from M36's list to M35's because it is M35's new contract
  ([P2 finding 5](../plans/m31-5-rust-first-replan.md#p2--found-2026-09-30)).

**That link breaks when the heading it names is renamed.** It targets the program plan's §M35 heading
as M31 wrote it. M31.5 re-details §M35 in Rust, renames the heading, and fixes this link in the same
change (P1 finding 5). This addendum routes to ADR-035 instead, so that it adds no second link to fix.

**The frozen lab never admits a photo-only entry.** Frozen means no new behaviour
([clause 4](035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)). Python's
`read_corpus` therefore keeps excluding a manifest that has no face-on clip, as `NO_FACE_ON`
(`storage/corpus.py:97`, read 2026-09-30). The frozen server's career view and Python `mcp/`'s club
and career tools both read through it (`api/app.py:925` and `:947`, `mcp/club.py:438` and
`mcp/career.py`). So wherever frozen Python answers a per-club question, from M35 until M40 deletes
it, it pools fewer shots than Rust does. That disagreement is by design, like the two readings of
the parser's label-fix shots in ADR-035's Consequences, and it is not a defect to fix in Python.

**Not changed**:

- §1–§5, and the three addenda above;
- the previous addendum's drill rule, its two levels, and its landing-offline projection. All three
  are M37's, and M37 was Rust first already;
- "No bulk backfill for club".
