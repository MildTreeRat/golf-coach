# ADR-024: Per-Club Shot History — the tag that makes distance a measurable quantity

## Status
Proposed. Design agreed 2026-08-20; no code written. The phase list is
[docs/M9_PLAYER_TRACKING.md](../M9_PLAYER_TRACKING.md); this document is the *why* behind it.
Flip to Accepted when M9 P20 closes.

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
