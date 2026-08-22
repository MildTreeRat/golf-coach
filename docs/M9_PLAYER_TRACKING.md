# M9 — Player Tracking: per-club shot history

> **Tier: TARGET.** This is the agreed plan for M9. Everything past the phase marked done below
> is still a plan, so verify every claim about the codebase against the code. The *why* behind it
> is [ADR-024](decisions/024-per-club-shot-history.md); this document is the *how*, as a phase
> list.

**Status: in progress, 7/20 phases.** P7 landed 2026-08-21 and closes the ingest spine.
Start at P8 — the measurements track, which is independent of P1–P7.

---

## What this milestone is

This repo can say how a swing compares to a tour population, and how it compares to the golfer's
own history. It cannot say **how far you hit your 7 iron**, because no shot on disk records which
club hit it.

That one missing field is the whole gap. Career mode already built the corpus reader, the honest
sample counts, the confidence intervals, the minimum-`n` guard and the bias/scatter discriminator.
Adding a club tag at capture and a `club=` filter to `storage.corpus.narrow_to` makes all of that
produce per-club answers with nothing new learning the rules.

**Outcome:** a bag page listing every club with its average carry, its spread, its start-line
bias, and its loft — each with an honest `n` or an explicit refusal. Plus a declared bag carrying
per-club loft, which is the anchor future club fitting needs and which cannot be recovered after
the fact.

Read ADR-024 §1–§5 before starting. The four design decisions are settled and should not be
relitigated: specific club id with derived category; loft on the bag entry; lateral miss as a
start-line projection; club required at upload but read from the session cursor.

## Two things that will look obvious and are wrong

- **Do not wire the club into `resolve_range` / `PracticeGoal.club`.** `ranges.json` holds
  `club_category: "all"` rows only. Passing a real category makes every checkpoint resolve no band
  and the fundamentals panel goes dark. ADR-010 already gated per-club bands and cut none.
  `PracticeGoal.club` stays `ALL`.
- **Do not add an `attribute_unlabeled` equivalent for club.** That bulk backfill is safe for
  golfer (a session usually has one) and destructive for club (a session has many). Club gets a
  per-swing repair route only.

## Reading order for a fresh session

`CLAUDE.md`, then ADR-024, then this file, then the three-to-six files your phase names. Every
phase below states its own files and what to reuse, so a phase number is a complete handoff — you
should not need to re-explore the repo.

---

## Phases

Each phase is independently commit-ready and has at most one design decision. `tests/` mirrors
`src/golf_coach/` package by package.

| Track | Phases | Notes |
|---|---|---|
| Ingest spine | P1 → P7 | Strictly in order. |
| Measurements | P8 → P11 | **Independent of P1–P7** — can run in parallel. P10 is skippable. |
| Corpus + profile | P12 → P17 | Needs both tracks above. |
| Surfaces | P18, P19 | Independent of each other; both need P15. |
| Docs | P20 | Last. |

Run after every phase:

```bash
.venv/Scripts/python.exe -m pytest
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
```

---

### [x] P1 — `contracts/club.py`: the club vocabulary *(done 2026-08-21)*

**Goal.** A fixed club taxonomy and a derived mapping to the existing `ClubCategory`. Nothing
consumes it yet.

**Files.** `src/golf_coach/contracts/club.py`, `tests/contracts/test_club.py`.

**Reuse.** `ClubCategory` from `contracts/intent.py` — import it, do not redefine.

**Detail.**
- `ClubId(StrEnum)`: `driver`, `3w`, `5w`, `7w`, `2h`–`5h`, `1i`–`9i`, `pw`, `gw`, `sw`, `lw`,
  `putter`. Declaration order is canonical bag order; P2 reads it.
- `CLUB_CATEGORY: dict[ClubId, ClubCategory]` — the single mapping table. driver→DRIVER,
  woods→WOOD, hybrids→HYBRID, 1i–4i→LONG_IRON, 5i–7i→MID_IRON, 8i–9i→SHORT_IRON, wedges→WEDGE,
  putter→PUTTER.
- `category_of(club) -> ClubCategory`.
- `parse_club(text) -> ClubId | None` — the tolerant boundary parser: lowercase, strip spaces and
  hyphens, accept "7 iron" / "7i" / "7-iron". Returns `None` rather than guessing (R7). **The only
  place free text becomes a `ClubId`.**

**Comment to write.** Why wedges are named `pw/gw/sw/lw` and not by loft: loft belongs to the
physical club and lives on the bag entry, so naming the slot by it would put a measurement inside
an identifier and a re-grind would orphan the club's history (ADR-024 §2).

**Tests.** Every `ClubId` has a category — walk the enum, do not re-list it (R6). `parse_club`
round-trips every enum value's own string, and returns `None` on junk.

**Done when.** Tests pass; `mypy src` clean.

---

### [x] P2 — `contracts/bag.py`: what a club in the bag is *(done 2026-08-21)*

**Goal.** The shape of a bag entry, carrying the loft fitting will need. Pure contract.

**Files.** `src/golf_coach/contracts/bag.py`, `tests/contracts/test_bag.py`.

**Detail.**
- `BagEntry`: `club: ClubId`, `loft_deg: float | None`, `make: str`, `model: str`, `shaft: str`,
  `length_in: float | None`, `recorded_at: datetime`.
- `Bag`: `player_id: str`, `entries: dict[ClubId, BagEntry]`, `updated_at: datetime`, plus a
  `club_ids` property returning canonical bag order (derived from `ClubId` declaration order, not
  sorted alphabetically).
- **As built, P2 also added** a `model_validator` pinning each `entries` key against its own
  `BagEntry.club`, and the `PLAYER_ID` slug check copied from `Golfer` — the store reads the id
  straight into a path, so the contract is where that has to hold. P3 then added `retired`,
  `retired_at`, `same_club_as` and `retired_for`; see its entry.

**Comment to write.** Why `loft_deg` is optional — a golfer who has not measured their lofts still
has a bag; the work that needs loft refuses per club, the same posture as `SwingResult.unscored`.
Why `recorded_at` exists — a bag entry that changes makes shots before and after it two
populations, and P16 turns that into a caveat rather than pooling them silently.

**Tests.** JSON round-trip. `club_ids` returns canonical order, not insertion or alphabetical.

**Done when.** Tests pass; `mypy src` clean.

---

### [x] P3 — `storage/bag_store.py`: the bag on disk *(done 2026-08-21)*

**Goal.** Read/write one bag per golfer.

**Files.** `src/golf_coach/storage/bag_store.py`, `tests/storage/test_bag_store.py`; amended
`contracts/bag.py` and `tests/contracts/test_bag.py`.

**Reuse.** Mirrors `storage/golfer_store.py` — same class shape, same tolerant read. Same atomic
write (tmp + `os.replace`) as `storage/manifest.py:save_manifest`.

**Path, as built:** `data/processed/golfers/<player_id>.bag.json` — **beside the golfer**, reusing
`settings.golfers_dir` rather than adding one. `GolferStore.list_all` globs `*.golfer.json` and
this store globs nothing, so neither sees the other's files, and the `.golfer.json` suffix already
existed so that directory could hold more than one record kind per player. No new config field.
`player_id` is slug-validated by `contracts/golfer.py:PLAYER_ID` inside `Bag`, so the store does no
second sanitising step — the ordering is what makes that safe: a bad id fails constructing the
model, before `save` can turn it into a filename.

**Detail.** `BagStore`: `get`, `save`, `set_entry`, `remove_entry`, `restore_entry`. A missing or
corrupt bag reads as `None` for a *reader* (R8) — writers deliberately refuse instead, see below.

**The phase gained a decision the list did not anticipate: nothing deletes a club.** Replacing or
removing one moves the outgoing entry to `Bag.retired`, an append-only shelf, and `restore_entry`
puts back the club a slot held before. Overwriting it would destroy a measured loft, which is the
exact loss "unrecoverable after the fact" was the argument against. This is **retention, not the
bag entry versioning ADR-024 defers** — nothing reads the shelf, and P16 still caveats from the
current entry's `recorded_at`. See the [ADR-024 addendum](decisions/024-per-club-shot-history.md).

Two consequences worth knowing before P19:

- **Re-saving an unchanged entry writes nothing and does not move `recorded_at`** (identity is
  `BagEntry.same_club_as`, every descriptive field and neither timestamp). P19's save button on an
  unedited row would otherwise hand P16 a bag-changed caveat over a club that never changed.
- **The store owns the clock.** `set_entry` discards the caller's `recorded_at`, as
  `get_or_create` discards `handedness` for a known golfer.

**And one guard the shelf made necessary.** `get` collapses "no bag" and "unreadable bag" into
`None`; the three mutators read through `_load_for_write`, which distinguishes them and **raises**
on the second. A writer treating a corrupt file as an empty bag replaces the bag *and its whole
shelf* with the one club it was asked to set.

**Tests.** 21 in `tests/storage/test_bag_store.py`, 7 added to `tests/contracts/test_bag.py`. The
three load-bearing pins — the clobber guard, the unchanged-row short-circuit, and the shelf being
copied rather than popped — were each checked by removing the behaviour in-process and watching the
test fail, rather than assumed.

---

### [x] P4 — `club` reaches the manifest and the session cursor *(done 2026-08-21)*

**Goal.** The storage layer can record which club hit a swing. No API, no UI, and no writer yet —
P5 stamps the field, P6 makes it required at the boundary.

**Files.** `src/golf_coach/storage/manifest.py`, `src/golf_coach/storage/session_meta.py`,
`tests/storage/test_manifest.py`, and a new `tests/storage/test_session_meta.py`.

**Reuse.** `SwingManifest.player_id` is the precedent and was copied literally — same
`Field(default=None, description=...)` posture, same no-migration argument.

**Detail, as built.**

- `SwingManifest.club: ClubId | None = None`. The description records the three facts that go
  load-bearing later: stamped from the session cursor at swing creation, **write-once in
  practice**, and `None` meaning *predates the field* — which is where it parts company with
  `player_id`, since P6 refuses an untagged upload. Optional in the shape, required at the
  boundary (R12), which is what lets every manifest already on disk keep loading.
- `SessionMeta.club: ClubId | None = None` and `set_current_club`.

**The load-modify-save fix, which was the phase's real content.** `set_current_player` constructed
a whole fresh `SessionMeta` — correct while there was one cursor, and silently destructive the
moment there were two: picking a golfer would have cleared the club, on disk, mid-session, visible
only later as a mistagged shot. Both setters now go through one private `_update_cursor`, which
loads, applies only the named change, and persists.

**Why one helper rather than load-modify-save written out twice.** Carrying the sibling field by
hand in each setter fixes it today and re-introduces it the day a third cursor arrives and one
setter forgets — so the preservation lives in one place a new cursor inherits without asking. It
merges onto a `model_dump` and re-runs `model_validate` rather than `model_copy(update=...)`, for
the reason `bag_store._write` already gives: `model_copy` skips validators.

**One `updated_at` for the whole record**, not one per cursor. Nothing reads it today, and "when
the session's choices last changed" is a truthful reading of one field — but it cannot answer
"when was the club chosen", so P19 needs its own field if it wants that. Said in the docstring so
P19 does not discover it by shipping a wrong timestamp.

**No backfill counterpart for the club**, and `set_current_club`'s docstring says why:
`attribute_unlabeled` reaches backwards over a session because a session usually has one golfer; a
session has many clubs, so the same reach would confidently mislabel every earlier swing
(ADR-024 §5).

**Tests.** The session-cursor tests **moved** out of `tests/storage/test_golfer_store.py` into a
new `tests/storage/test_session_meta.py` — `session.json` stopped being about golfers alone, and
`tests/` mirrors `src/` package by package. 15 there (5 moved, 10 new) and 3 added to
`test_manifest.py`, taking the suite 858 → 871. Both directions of the cross-cursor pin are written out separately, and both
were checked by **reverting each setter to its replacing form in-process and watching the suite go
red** — which is how it is known they are pins. Each break was caught by a *different* test, so a
single direction would have missed one.

**Deliberately left stale for P20:** `docs/ARCHITECTURE.md` §4 still calls `session.json` the
"golfer cursor" (line ~400) and its manifest row (~397) names only `player_id`. `tests/test_docs_truth.py`
does not cover those tables, so nothing goes red in the meantime.

---

### [x] P5 — `bundle_store` stamps and repairs the club *(done 2026-08-21)*

**Goal.** Thread the club through swing creation, plus the per-swing repair path.

**Files.** `storage/bundle_store.py`, `tests/storage/test_bundle_store.py`.

**Reuse.** `assign_from_path`'s `player_id` parameter and `set_player` were the two shapes copied,
literally — the club now appears at every site `player_id` does and at no others.

**Detail, as built.** `assign_from_path(..., club=None)` threaded into `_new_manifest` and `_place`;
write-once. `set_club(session_id, swing_id, club)` as the explicit human-driven override, with no
`attribute_unlabeled` analogue and a docstring saying why.

**The phase list did not name `AssignmentResult`, and it had to change.** It gained `club`, for two
reasons that only appear once you write the dedupe path out: P6 puts the club in the upload response
and builds that response field-by-field off the result, and the deduped early return needs somewhere
to report `manifest.club` — the club the swing *says*, not the one the retry asked for. A phone
retrying an upload after the cursor has moved on would otherwise be told its stale club won.

**Where the club is not the golfer, in the one place it shows.** `_place`'s stamp-if-empty branch is
the whole two-phone fix for `player_id` — one phone uploads before a golfer is picked, the other
after, and the swing gets attributed on the second file. For the club that branch is **defensive
rather than load-bearing**: P6 refuses an upload with no club, so a swing created since M9 cannot
reach `_place` untagged. What it still covers is a swing written before the field existed receiving
a later role, and tagging that from the cursor current *now* is right, because now is when the swing
is being completed. The comment beside it says so, so nobody later reads the two branches as equally
important and deletes the wrong one.

**Tests.** 10 added to `tests/storage/test_bundle_store.py`, 19 → 29, mirroring the golfer block
below its own divider. Three are load-bearing and each was checked by **removing the behaviour
in-process and watching that one test go red**: the write-once guard, the dedupe echo reading
`manifest.club` rather than the argument, and `set_club` mutating the loaded manifest rather than
rebuilding one (which is P4's `set_current_player` bug, one layer down — the pin exists because the
mistake has already been made once in this repo). The cross-field pin is written out in **both**
directions, for the reason P4 found: each direction is caught by a different test.

**Still stale for P20**, unchanged from P4: `docs/ARCHITECTURE.md` §4 calls `session.json` the
"golfer cursor" and its manifest row names only `player_id`.

---

### [x] P6 — the upload endpoint requires a club *(done 2026-08-21)*

**Goal.** No shot can be ingested without a club.

**Files.** `api/app.py`, `tests/api/test_uploads.py`; plus the club cursor added to the four other
API test files that upload (see below).

**Reuse.** The golfer cursor routes were the template and were followed literally: `GolferRequest`
→ `ClubRequest`, `_resolve_golfer` → `_resolve_club`, `set_swing_golfer` → `set_swing_club`,
`_safe()` on both path segments. `parse_club` got its first caller, which was the point of P1.

**Detail, as built.**

- `GET` / `POST /api/sessions/current/club`, declared immediately after the golfer cursor routes —
  that position is load-bearing for the reason the comment above `/api/sessions/{session_id}`
  already gives, since `{session_id}` would otherwise swallow the literal `current`. **No backfill
  call**, and the docstring says why rather than leaving the absence to be read as an oversight.
- `POST /api/uploads` reads **both** cursors in one `load_session_meta`, before the body streams,
  and 409s when the club is `None` with a detail naming the route that fixes it.
- `POST /api/sessions/{session_id}/swings/{swing_id}/club` — the repair route.

**`ClubRequest.club` is a `str`, not a `ClubId`, and that is a real decision.** Typed as the enum,
pydantic rejects "7 iron" with a 422 before `parse_club` ever runs — and those tolerant spellings
are the entire reason that parser exists. Parsing therefore happens in `_resolve_club`, which keeps
`contracts/club.py`'s claim to be *the* place free text becomes a `ClubId`, and keeps the boundary's
own 400 (R12).

**`session_id` is computed once and threaded down**, where the handler previously read it after the
stream. A large upload spanning midnight would otherwise check one session's cursor and write the
swing into the next day's — a mistag with no downstream symptom. The comment says so; it is the
kind of thing that reads like a pointless local variable a year later.

**Two read routes gained `club` beyond what this list named.** `session_detail`'s per-swing rows and
`swing_detail` now report it wherever they already report `player_id`, which is P5's design
instruction ("the club appears at every site `player_id` does") applied one layer up. Without it P7
has a repair route it cannot show the current value for. `_club_value` is the one place
`ClubId | None` becomes JSON.

**Tests: 16 added, 23 → 39 in the file, suite 881 → 897.** They sit under their own divider
mirroring the golfer block, because the two asymmetries — required where the golfer is not, no bulk
backfill where the golfer has one — only read as deliberate side by side. **Fifteen existing tests
across five files had to select a club first**: explicitly at each call site in
`tests/api/test_uploads.py` (so the tests that *do not* call `_pick_club` are visibly the refusal
ones), and once at the construction point in `test_results.py`, `test_worker.py`,
`test_career_route.py` and `test_conversation_routes.py`, which are not about the club.

**Three pins were watched fail rather than assumed**, the P3/P4/P5 habit. Moving the 409 to after
the streaming block fails "a clubless upload writes nothing to disk" — and leaves an orphaned
`.part` behind, which is the failure in full. Echoing the cursor instead of `manifest.club` in the
response fails the dedupe test. Rebuilding `SessionMeta` instead of going through `_update_cursor`
fails "picking a club leaves the golfer alone", which is P4's bug at the route.

**Known and deliberate: the upload page cannot upload after this phase.** It has no club picker, so
every file it sends gets a 409 until P7. That is the cost of landing P6 alone and it is not a
regression to hunt.

**Still stale for P20**, unchanged from P4 and P5: `docs/ARCHITECTURE.md` §4 calls `session.json`
the "golfer cursor", its manifest row names only `player_id`, and no route table knows about the
three routes added here. `tests/test_docs_truth.py` pins no route list, so nothing goes red.

---

### [x] P7 — the club picker on the upload page *(done 2026-08-21)*

**Goal.** A phone in a bay can pick a club in one tap.

**Files.** `api/static/index.html`, **`api/app.py`**, `tests/api/test_uploads.py`.

**The file list grew a route, and that was the phase's one decision.** P6's worklog deferred it
here explicitly: *does the picker derive its list from a route, or inline it?* Inlined, the 22 ids
in `contracts/club.py` would have a second copy in a static file nothing tests — and the failure is
quiet, because a club added to `ClubId` would parse at every route in `api/app.py` while being
unpickable at the bay. So `GET /api/clubs` was added and the phase is an L2 rather than the L1 its
box implied.

**Reuse.** The golfer bar for the fetch/poll/render shape and its warning styling; `/api/golfers`
as the route's sibling; `Bag.club_ids` for ordering; `statusEl`'s delegated listeners for a
subtree the poll replaces.

**Detail, as built.**

- **`GET /api/clubs`** returns `{"clubs": [...], "bag": [...]}` — the full taxonomy straight off
  `ClubId`, plus the session golfer's declared clubs through `Bag.club_ids`. Both walk the enum, so
  **canonical bag order comes from the declaration** and nothing sorts. Serving `bag.entries`
  instead returns insertion order and is what `test_a_declared_bag_comes_back_in_bag_order_...`
  catches.
- **Two things on one route** because a phone on cellular pays for round trips and neither half is
  useful alone: the taxonomy cannot put the golfer's own clubs first, and the bag cannot offer the
  club they have just borrowed.
- **No labels are invented.** The ids go over the wire as they are and the page uppercases them in
  CSS. A server-side `"7 iron"` would be a second spelling table beside `_build_aliases`, free to
  disagree with the one that does the parsing.
- **`BagStore` is derived, not injected**: `BagStore(golfer_store.root)` in `create_app`. The bag
  file lives in the golfer directory by design (`<player_id>.bag.json` beside
  `<player_id>.golfer.json`), so deriving the root means the pair cannot be pointed at different
  directories — and no test fixture had to learn about a store it does not exercise.
- **The picker is an always-open chip grid**, not a collapsed bar with a *change* link. That is what
  this box's *"unlike the golfer"* is contrasting against rather than asking us to copy: the club
  changes every few shots, so a collapse step would be a tap on almost every shot. Two sections when
  the golfer has a bag (**In the bag**, then **Every club**), one when they do not — which is every
  golfer today, since nothing writes a bag until P19.
- **The file input is disabled while no club is selected**, and starts disabled in the markup so
  there is no window at load in which it looks usable. This is the golfer bar's stated asymmetry
  made visible: that bar *never* blocks the input, and the comment beside this CSS says why this
  one does.
- **The 409 is surfaced, not dumped.** The failure branch used to print the raw response body;
  it now shows `detail` and, on 409 specifically, re-reads the cursor — reaching that status means
  the page's idea of the cursor is stale (the other phone moved it, or the session rolled over),
  not that the golfer did something wrong.

**Two things the box did not name and the page needed.** `clubPending` holds the poll off across a
POST, which is `editingGolfer` applied to a subtree that would otherwise flash the previous chip
back. And the 5s poll of the cursor is what **survives midnight**: the cursor is per-session and a
session is a day, so the new directory answers `null`, the picker goes unset and the input disables
rather than the page showing a club it is no longer tagging anything with.

**Tests: 8 added, 39 → 47 in the file, suite 897 → 905.** They assert against `ClubId` itself
rather than a written-out list — a literal here would be the very duplicate the route exists to
prevent, and it would pass on the day a club is added to the enum and forgotten everywhere else.
One test walks every club the route serves through the cursor route, which is the pin that the
picker's list and the parser are one vocabulary.

**Two pins were watched fail rather than assumed**, the P3–P6 habit. Serving `bag.entries` fails the
bag-ordering test with `['pw', 'driver', '7i']`; sorting `clubs` fails the canonical-order test at
index 0. **And the DOM half was driven, since nothing tests a static file**: the page's own script
was run against a live server with a stubbed DOM, in all three states — club set (input enabled,
chip highlighted), bag declared out of order (rendered `driver 7i pw sw`), and cursor cleared
(`unset`, header `none selected`, input disabled).

**Done when.** Manual check via `python scripts/run_server.py`: pick a club, upload, and the
manifest names it. ✅ — verified live against a scratch data directory: `7i` then `pw`, two swings,
`["7i", "pw"]` on the read route and `"club": "7i"` in swing 1's manifest.

**Still stale for P20**, unchanged from P4–P6: `docs/ARCHITECTURE.md` §4 calls `session.json` the
"golfer cursor", its manifest row names only `player_id`, and no route table knows about the four
routes M9 has added. `tests/test_docs_truth.py` pins no route list, so nothing goes red.

---

### [ ] P8 — carry and total distance become measurements

**Goal.** "How far did it go" enters the corpus.

**Files.** `analysis/shot_measure.py`, `tests/analysis/test_shot_measure.py`.

**Reuse.** The `SHOT_MEASUREMENTS` registry. The engine already walks it, so nothing else needs
touching for these to reach `analysis.json`.

**Detail.** `carry_distance_yds` and `total_distance_yds`, `None`-safe reads, unit `yards`.

**Comment to write — this is the phase's real content.** The module docstring explains what is
*excluded* and why; add carry to the *included* side with its own reason. Carry was not measurable
before because a carry pooled across clubs is meaningless — a mean over a driver and a sand wedge
describes nobody's shot. **The club tag is what makes distance poolable at all**, which is why
this metric arrives with M9 and not M6.5. Do not touch the existing exclusions for
`smash_factor`, `club_head_speed` or `spin_axis`.

**Tests.** Present → measured; missing → `None`; unit is `yards`. An engine-level test that an
analyzed swing with a shot carries both.

**Done when.** Tests pass.

---

### [ ] P9 — `start_line_offline_yds`

**Goal.** "How many yards right or left", as exact geometry, honestly named.

**Files.** `analysis/shot_measure.py`, `tests/analysis/test_shot_measure.py`.

**Detail.** `carry_distance * sin(radians(launch_direction))`, `None` if either is missing.
Positive is right of target, matching `launch_direction` and `start_line_deg`. Unit `yards`.

**Docstring to write — the honesty is the deliverable.** State that this is where the ball *would*
have landed if it never curved, not where it landed; that the screen prints no offline tile; that
the curve is reported separately in degrees by `measure_face_to_path`. Name the consequence: a
golfer who starts it straight and slices reads about 0 here and the whole miss lives in
`face_to_path_deg`, so the two must be read together and any prose must say *started*, never
*finished*. Note that a configurable offline tile would be a one-row `profiles.json` addition and
should **supersede** this rather than sit beside it.

**Tests.** 150 yd at +2° → +5.23 yd (within 0.01). Negative angle → negative. Zero → 0.0. Either
input missing → `None`. A pin that the sign agrees with `start_line_deg`.

**Done when.** Tests pass.

---

### [ ] P10 — ball speed and launch angle as fitting inputs *(optional; skippable)*

**Goal.** Record the two fields club fitting will need, judged by nothing. Pure measure-now-judge-
later (M6.5).

**Files.** `analysis/shot_measure.py`, `tests/analysis/test_shot_measure.py`.

**Detail.** `ball_speed_mph` and `launch_angle_deg`, straight reads.

**Comment to write.** Why these two and not `club_head_speed` or `smash_factor`: the existing
exclusion stands — every shot on disk reads a smash factor below 1.0, which no strike produces, so
club speed is a known-bad number on this device. These two are not implicated by that check. They
are recorded because launch conditions are the input to any future fitting model and are
unrecoverable after the fact; they get no target and no band.

**Done when.** Tests pass. Skip this phase entirely for the smallest useful M9.

---

### [ ] P11 — targets and tolerances for the new metrics

**Goal.** Let `build_dispersion` speak about the new metrics instead of refusing them.

**Files.** `contracts/dispersion.py`, `tests/contracts/`.

**Reuse.** `METRIC_TARGETS` and the `_JUDGED_DEGREES` provenance-constant pattern beside it.
`target_for` returns `None` for unregistered metrics, which refuses **both** findings — so without
this phase P8/P9's metrics are measured and permanently silent.

**Detail.** Add a `_JUDGED_YARDS` provenance constant, then one `MetricTarget` each. Targets:
`carry_distance_yds` and `total_distance_yds` get **no** target (how far a golfer *should* hit a
club is not a number this repo has, and a tour carry band would judge an amateur against a
population they are not in); `start_line_offline_yds` gets target `0.0` by geometry, the same
argument `start_line_deg` already makes; P10's two get no target (a "right" ball speed is a
fitting output, and optimal launch needs the model that does not exist).

**The design note to write into the code.** The offline tolerance is the `_JUDGED_DEGREES` figure
evaluated at the *widest club in the bag*. Offline error scales with carry, so a single constant is
the wrong shape and the correct tolerance is per club. Erring wide is the documented safe direction
here — it costs claims, where erring narrow buys confident claims about the simulator's own noise.
Record the deferral in the provenance string.

**Do not add `METRIC_MINIMUM_N` overrides.** The defaults are right for distance and there is no
evidence to justify moving them. Say so in a comment so nobody adds one speculatively.

**Tests.** Every metric in `SHOT_MEASUREMENTS` and `POSE_MEASUREMENTS` has a `METRIC_TARGETS`
entry — derive both sides from the registries (R6), so a metric added later fails loudly instead
of going silently unjudgeable.

**Done when.** Tests pass; `tests/analysis/test_dispersion.py` still green.

---

### [ ] P12 — the corpus carries the club

**Goal.** `CorpusSwing` knows which club hit it.

**Files.** `contracts/career.py`, `storage/corpus.py`, `tests/storage/test_corpus.py`.

**Detail.** `CorpusSwing.club: ClubId | None`; `_corpus_swing` reads `manifest.club` alongside
`manifest.player_id`; `CareerCorpus.untagged_swings: int`, counted for the same reason
`unattributed_swings` is.

**Design note.** Do **not** add an `ExclusionReason` for an untagged swing. It is a perfectly good
contributor to every pose metric and to whole-bag numbers; it is excluded only from per-club
views. Excluding it from the corpus would silently shrink the mechanics `n`.

**Tests.** A tagged manifest produces a `CorpusSwing` carrying the club. A legacy manifest produces
`None`, increments `untagged_swings`, and does **not** appear in `excluded`.

**Done when.** Tests pass; the existing corpus suite is green.

---

### [ ] P13 — `narrow_to(club=)`

**Goal.** The one-line change that makes every per-club statistic possible.

**Files.** `storage/corpus.py`, `tests/storage/test_corpus.py`.

**Reuse.** `narrow_to` — read its docstring first; it already explains why counts are recomputed
inside the filter rather than by the caller.

**Detail.** Add `club: ClubId | None = None` to the keyword-only signature and one clause to the
comprehension. The `metric_counts` recomputation is already there and is what makes the per-club
`n` honest for free.

**Docstring addition.** Say what this unlocks: narrowing to a club and handing the result to
`build_baseline` gives a per-club mean carry that refuses at the same thresholds a whole-corpus
metric refuses at — so "your 7 iron carries 164 yards" needs 5 distinct 7-iron shots, not 5 shots.
Nothing new had to learn the guard.

**Tests.** Narrowing keeps only that club's swings **and** recomputes `metric_counts` to match —
pin the count, not just the list; that is the failure the docstring warns about. An unhit club
gives an empty corpus, not an error. Club and `since` compose.

**Done when.** Tests pass.

---

### [ ] P14 — `contracts/club_profile.py`: the shape of a club's history

**Goal.** The output shape. Pure contract, nothing builds it yet.

**Files.** `contracts/club_profile.py`, `tests/contracts/test_club_profile.py`.

**Reuse.** `MetricBaseline` from `contracts/baseline.py`, `MetricDispersion` from
`contracts/dispersion.py`, `BagEntry` from P2. **Compose these, do not restate their fields** —
`contracts/dispersion.py` already reuses `Interval` and `WithheldClaim` for exactly this reason
(R5).

**Detail.** `ClubProfile`: club, category, `bag_entry`, `n_shots`, `n_sessions`, `metrics`,
`dispersion`, `caveats`, `in_bag`. `BagProfile`: `player_id`, `clubs` in canonical order,
`untagged_shots`, plus `clubs_used` (n_shots > 0) and `clubs_declared` (`in_bag`) as **derived
properties**, so the two categories cannot drift out of agreement with the data.

**Comment to write.** Why `in_bag` and `n_shots > 0` are separate: a club in the bag you have not
hit has no statistics but is not an error, and a club you have hit that has left the bag still has
real history. Collapsing them into one list loses which is which.

**Tests.** JSON round-trip. The two derived lists are correct for all four combinations of (in
bag, has shots).

**Done when.** Tests pass; `mypy src` clean.

---

### [ ] P15 — `analysis/club_profile.py`: the builder

**Goal.** Turn a corpus plus a bag into a `BagProfile`. **It should be short** — if it is long,
something is being reimplemented.

**Files.** `analysis/club_profile.py`, `tests/analysis/test_club_profile.py`.

**Reuse — all of it.** `narrow_to` (P13), `build_baseline`, `build_dispersion`, `category_of`.
For each club present in the corpus or declared in the bag: narrow, build both, assemble.

**The one design decision.** `analysis` must not import `storage` (ADR-008), and `narrow_to` lives
in `storage/corpus.py`. **Check the import direction before writing this.** If it cannot be
imported, move the pure filter half of `narrow_to` onto `CareerCorpus` in `contracts/career.py` and
have `storage.corpus.narrow_to` delegate. **Prefer the move** — it is the ADR-008-clean answer, and
`CorpusSwing.artifact_key` is already precedent for a shared rule living on the contract because
both sides need it.

**Tests.** A corpus with 6 `7i` shots and 2 `driver` gives a 7i with a CENTER claim and a driver
refusing one, both with the right `n`. An empty bag still profiles clubs that have shots. A
declared-but-unhit club appears with `n_shots=0` and every claim withheld.

**Done when.** Tests pass; `tests/api/test_pipeline_imports.py` green — this is the phase most
likely to trip it.

---

### [ ] P16 — the bag-changed caveat

**Goal.** Never silently pool two physical clubs under one name.

**Files.** `analysis/club_profile.py`, `tests/analysis/test_club_profile.py`.

**Detail.** When `bag_entry.recorded_at` is later than the earliest pooled shot's `captured_at`,
append a caveat naming the date and saying shots before and after it may have been hit with a
different club. Do **not** withhold the statistic — the entry may simply have been recorded late
for a club that never changed.

**Comment to write.** The same posture `SESSION_DRIFT_FACTOR` takes: a loose judgment that only
ever adds a caveat and never removes a claim, so being wrong about it costs a sentence rather than
a verdict.

**Tests.** Recorded before all shots → no caveat. Recorded mid-history → caveat naming the date.
No bag entry → no caveat.

**Done when.** Tests pass.

---

### [ ] P17 — `scripts/club_profile.py` CLI

**Goal.** Read the numbers without a browser or an MCP client. **The phase that proves the spine
works.**

**Files.** `scripts/club_profile.py`.

**Reuse.** `scripts/career_baseline.py` and `scripts/career_dispersion.py` are the templates —
same argument shape, same output style, same tolerant reads.

**Detail.** `python scripts/club_profile.py <player> [--club 7i]`. Per club: `n`, mean carry with
its CI or the refusal, sd, the start-line offline finding, and the bag entry's loft. Refusals print
`WithheldClaim.reason` verbatim — that sentence already names what is missing.

**Done when.** Running it against the data on disk prints a table of refusals — expected, since
every swing on disk is untagged — without crashing. "Refuses cleanly on real data" is the
acceptance criterion, exactly as it was for career mode.

---

### [ ] P18 — MCP tools

**Goal.** Claude can answer "how far do I hit my 7 iron".

**Files.** `mcp/club.py` (or extend `mcp/career.py`), `mcp/server.py`,
`contracts/tool_descriptions.py`, `tests/mcp/test_club_tools.py`.

**Reuse.** `mcp/career.py`'s view-model pattern — `MetricProfile`, `Refusal`, `resolve_golfer`,
`_refusal`. Register in `server.py` the way the career tools are.

**Detail.** `get_bag_profile(player)` and `get_club_profile(player, club)`.

**The tool descriptions are load-bearing** — `tests/test_docs_truth.py` reads them, and the MCP
instructions block is how Claude learns to read this data honestly. Say explicitly: a withheld
claim is not a zero; `start_line_offline_yds` is where the ball *started*, projected to carry
distance, and never where it landed; curve is `face_to_path_deg`, in degrees; a club with no bag
entry has no loft, so any fitting question about it must be refused.

**Tests.** Tools appear in the advertised list. Unknown player → the `NotFound` shape. A club at
`n=2` returns refusals with `have_n` / `need_n` populated. Mirror `tests/mcp/test_career_tools.py`.

**Done when.** `pytest tests/mcp/` green; `python scripts/run_mcp_server.py` advertises them.

---

### [ ] P19 — the bag page

**Goal.** A browser view of the bag.

**Files.** `api/app.py`, `api/static/career.html` (extend) or a new `bag.html`,
`tests/api/test_bag_route.py`.

**Reuse.** `GET /api/golfers/{player_id}/career` is the route template, including its
`_safe(player_id, "player id")` guard. `career.html` is the page template.

**Detail.** `GET /api/golfers/{player_id}/bag` returning the `BagProfile`, plus
`POST /api/golfers/{player_id}/bag/{club}` to set an entry's loft/make/model. One row per club:
carry mean with CI or "needs N more shots", spread, start-line bias, loft, and an edit control.

**Render refusals as first-class text, not blanks.** A blank cell reads as zero; the
`WithheldClaim.reason` string is already written for a human and should be shown verbatim.

**Done when.** `pytest tests/api/` green; the page renders against the data on disk showing
refusals.

---

### [ ] P20 — docs reconciliation

**Goal.** The docs and the code agree; ADR-024 flips to Accepted.

**Files.** `docs/ARCHITECTURE.md` (§1 commands, §3 the `analyze_swing` walk, §4 what is on disk),
`docs/README.md`, `ROADMAP.md`, `WORKLOG.md`, `data/README.md`, and this file's status line.

**Detail.** Run `pytest tests/test_docs_truth.py` **first** and work only from its failures — that
is the `/doc-check` protocol and this repo's stated method. Do not copy any band, count, threshold
or tolerance into prose; point at the registry instead.

**Done when.** `pytest` fully green, `ruff check src tests scripts` clean, `mypy src` clean.

---

## Verification, end to end

Once P1–P13 are in:

1. `python scripts/run_server.py`; register a golfer, pick a club.
2. Confirm an upload with no club selected is refused with a 409.
3. Upload a face-on clip plus a shot screen photo tagged `7i`.
4. `python scripts/analyze_bundle.py <session>/<swing>` and confirm `analysis.json` carries
   `carry_distance_yds` and `start_line_offline_yds` with `source: "launch_monitor:hd_golf"`.
5. `python scripts/club_profile.py aaron` — expect refusals with correct `n`s, not blanks.
6. Via MCP: `get_club_profile(player="aaron", club="7i")`, and confirm the refusal reads honestly.

**The correct result at every stage is refusal**, because there is not enough data on disk yet. A
number appearing early is the bug — the same acceptance criterion career mode used. The feature
turns on when a bay session puts five or more shots on a club.
