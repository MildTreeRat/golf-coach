# M9 — Player Tracking: per-club shot history

> **Tier: TARGET.** This is the agreed plan for M9. Everything past the phase marked done below
> is still a plan, so verify every claim about the codebase against the code. The *why* behind it
> is [ADR-024](decisions/024-per-club-shot-history.md); this document is the *how*, as a phase
> list.

**Status: in progress, 19/20 phases.** P19 landed 2026-08-22 and gave the bag a browser: a section
inside `career.html` over three new routes, and **the first thing in this repo that ever wrote a
bag** — `BagStore.set_entry` and `remove_entry` had no caller outside their own tests until now. It
renders the same five silences P18 enumerated, in the page's own voice, and it found two prose
errors by driving rather than by reasoning: `fmt` printed `154.400 yards` on two pages, and the CLI
pointed at a retag button that did not exist on any page. That button now does, and the paragraph
below is its record. **What is left is the docs.** Start at P20.

**The one thing standing between this milestone and a real answer was not a phase on this list, and
it is now fixed.** `POST /api/sessions/{session}/swings/{swing}/club` had no UI, so the swings
already on disk — every one of which predates the tag — could not be retagged from a phone. As of
2026-08-22 `index.html`'s swing list carries a per-swing **change** control beside the golfer one:
each row says which club it was hit with, `no club` in the warning colour when it says nothing, and
*change* opens the same chip grid the upload picker uses, scoped to that swing.

Three shapes there are worth knowing before P20 reads the page:

- **`clubChips` gained a `selected` argument**, defaulting to the cursor's club. The retag picker
  passes the *swing's* instead — highlighting `currentClub` inside a swing row would assert the
  swing was hit with whatever the bay is on now, which is the one fact a repair control must never
  invent. Breaking that argument in place fails four checks of the drive below.
- **The picker is collapsed behind *change*, where the bar at the top is never collapsed.** Same
  argument, run the other way: the cursor changes every few shots and a retag happens once per
  swing, so an always-open grid would put 22 chips on every row for a tap almost none of them get.
- **`retagSwing` and `retagPending` live in the page's state, not in the DOM.** `renderStatus`
  replaces that subtree every five seconds; held in the markup the picker would shut itself under
  the golfer's thumb, and a poll landing mid-write would flash the old club back.

Nothing needs re-analyzing after a retag: `analysis.json` never carried the club and
`storage/corpus.py` reads `manifest.club` off the manifest on every scan.

**No new tests, which is the precedent rather than an omission** — nothing here tests a static
file, and the route itself is pinned in `tests/api/test_uploads.py`. The suite stayed at 993. The
page's own script was driven against a live server with a stubbed DOM instead, over an untagged
swing beside a tagged one, and **three behaviours were watched fail rather than assumed**, the
P3–P19 habit: the `selected` argument (4 checks), the `retagPending` guard (1), and holding the
open picker in state (5+). Then driven on a **copy** of the real corpus, which is the payoff — with
the two stored swings retagged, `scripts/club_profile.py` leaves the empty state for a `7i` row at
`n = 2` with every claim withheld, and `mcp/club.py` moves from `NEVER_HIT` to
`NOT_ENOUGH_ON_THIS_CLUB` with `untagged_swings` at 0. The real `data/` was left alone: only the
golfer knows what those two swings were hit with (ADR-024 §5).

**The four prose blocks that said the button did not exist are corrected** — `mcp/club.py`'s
`NOTHING_TAGGED`, `career.html`'s `nothingTagged()`, and both blocks in `scripts/club_profile.py`.
Each keeps its own audience's wording: the model and the CLI still name the route, the page names
the control.

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

### [x] P8 — carry and total distance become measurements *(done 2026-08-21)*

**Goal.** "How far did it go" enters the corpus.

**Files.** `analysis/shot_measure.py`, `tests/analysis/test_shot_measure.py` — plus three the box
did not name: `contracts/dispersion.py`, `contracts/swing.py` and `tests/analysis/test_engine.py`.

**Reuse.** The `SHOT_MEASUREMENTS` registry. The engine already walks it, so nothing else needs
touching for these to reach `analysis.json`.

**Detail.** `carry_distance_yds` and `total_distance_yds`, `None`-safe reads, unit `yards`.

**Comment to write — this is the phase's real content.** The module docstring explains what is
*excluded* and why; add carry to the *included* side with its own reason. Carry was not measurable
before because a carry pooled across clubs is meaningless — a mean over a driver and a sand wedge
describes nobody's shot. **The club tag is what makes distance poolable at all**, which is why
this metric arrives with M9 and not with M6.5. Do not touch the existing exclusions for
`smash_factor`, `club_head_speed` or `spin_axis`. ✅ — written as its own section, *"What arrived
late, and why it could not arrive earlier"*, above the exclusions rather than inside them; the three
exclusions are untouched.

**Two things this box did not name, and the phase needed both.** They are why P8 was an L2 rather
than the L1 its file list implied:

- **`contracts/dispersion.py`, because P8 alone leaves the suite red.**
  `test_every_production_metric_has_a_tolerance` asserts
  `set(POSE_MEASUREMENTS) | set(SHOT_MEASUREMENTS) == set(METRIC_TARGETS)` — **strict equality**, not
  a subset. So a new measurement with no target row is not "measured and permanently silent" as P11
  below claimed; it is a failing test. P11's two distance rows were pulled forward, along with the
  `_JUDGED_YARDS` constant they hang off and P11's "no `METRIC_MINIMUM_N` overrides" note. **P11 is
  amended accordingly** and now holds `start_line_offline_yds` only.
- **`ANALYSIS_VERSION` 7 → 8.** Precedent is exact — `6 -> 7` (ADR-023) was two new `measurements`
  entries with no checkpoint, band or score moved, and a version-7 artifact is *missing* two
  quantities rather than disagreeing about any. Every test derives the number from the constant, so
  nothing pinned the literal and nothing went red to say the bump was owed.

**The tolerance is judgment and says so.** `_JUDGED_YARDS` is 5 yards for both distances, modelled
on `_JUDGED_DEGREES`: no instrument-error evidence exists for the OCR path, 5 yards is below the
level a coaching action follows from, and erring wide costs claims where erring narrow buys
confident claims about the simulator's own noise. **Neither distance gets a target** — how far a
golfer *should* hit a club is not a number this repo has, every distribution here is cut from GolfDB
(which contains no ball flight), and a tour carry band would judge an amateur against a population
they are not in. The provenance string records the same deferral P11 records for offline: distance
error scales with the club, so one constant is the wrong shape and the correct tolerance is per
club.

**Tests.** Present → measured; missing → `None`; unit is `yards`. An engine-level test that an
analyzed swing with a shot carries both. ✅ — **5 added, suite 905 → 910.** Three in
`test_shot_measure.py` (both distances carried as printed, a missing distance is `None` and never
0.0, both registered in `yards`) and two in `test_engine.py` — the registry-to-artifact path, plus
the direction that matters: a swing with no shot records no distance at all. The engine test asserts
`source == "launch_monitor:hd_golf"`, because `storage.corpus` keys its honest sample counts off
that string and a distance recorded under `pose:face_on` would be counted per face-on clip rather
than per shot photo.

**Two pins were watched fail first**, the P3–P7 habit — and one of them is why this box grew.
`test_every_production_metric_has_a_tolerance` fails on the bare registry addition, which is what
found the missing P11 dependency. And `test_registry_entries_are_well_formed` fails on its own
fixture: it asserts every registered measurement reads non-`None` off one shot, so the fixture had
to gain a carry and a total. That is the pin working — it is the cheapest place to notice that a new
metric has no test of its own — so it was fed, not weakened.

**Done when.** Tests pass. ✅ — and driven end to end, since the point of the phase is that the
number reaches a stored artifact. `scripts/reanalyze.py` re-ran all four stored swings (still on
engine 6): `version 6 -> 8 | measurements 14 -> 18` on each, `carry_distance_yds` 125.6 and
`total_distance_yds` 131.0 under `launch_monitor:hd_golf` in yards, and **every `overall_score` is
byte-identical** — 97.45951982132875 on three, 96.88296555239968 on the fourth — which is the
expected non-change for a bump of this shape. `scripts/career_dispersion.py` then refused both
metrics correctly: `n = 2 over 2 sessions` (the four directories dedupe to two shot photos, exactly
as `contracts/career.py` describes), both claims waiting on their sample floors, and the no-target
reason printed rather than the metric going quietly absent.

**One thing to know before P13.** Until `narrow_to(club=)` lands, everything that reads these two
pools them **whole-bag**. Nothing false ships today — `DEFAULT_MINIMUM_N[CENTER]` is 5 and there are
2 samples — but the guard that saves it is a sample count, not an argument about clubs, so it would
go on being satisfied by a mixed bag. That is commented at the registry rows rather than left to be
rediscovered.

---

### [x] P9 — `start_line_offline_yds` *(done 2026-08-21)*

**Goal.** "How many yards right or left", as exact geometry, honestly named.

**Files.** `analysis/shot_measure.py`, `tests/analysis/test_shot_measure.py` — plus two the box did
not name, both of them P8's documented consequences rather than surprises: `contracts/dispersion.py`
(the target row, see below) and `contracts/swing.py` (`ANALYSIS_VERSION` 8 → 9).

**Detail.** `carry_distance * sin(radians(launch_direction))`, `None` if either is missing.
Positive is right of target, matching `launch_direction` and `start_line_deg`. Unit `yards`. ✅ —
and it reads its two inputs **through `measure_carry_distance` and `measure_start_line`** rather
than off `ShotData` directly, so there is one definition of which field is the carry and which is
the start line.

**Docstring to write — the honesty is the deliverable.** State that this is where the ball *would*
have landed if it never curved, not where it landed; that the screen prints no offline tile; that
the curve is reported separately in degrees by `measure_face_to_path`. Name the consequence: a
golfer who starts it straight and slices reads about 0 here and the whole miss lives in
`face_to_path_deg`, so the two must be read together and any prose must say *started*, never
*finished*. Note that a configurable offline tile would be a one-row `profiles.json` addition and
should **supersede** this rather than sit beside it.

**Tests.** 150 yd at +2° → +5.23 yd (within 0.01). Negative angle → negative. Zero → 0.0. Either
input missing → `None`. A pin that the sign agrees with `start_line_deg`. ✅ — **5 added, suite
910 → 915**, all five in `test_shot_measure.py`. The sign pin is written as the box asked and the
shape matters: it asserts the two metrics agree **with each other** over four angles rather than
each against a constant, because two readings of one quantity in two units is exactly where a
dropped minus sign produces a coherent-looking artifact that says the golfer misses the other way.
The zero case is pinned beside the `None` cases on purpose — `0.0` and `None` are the same falsy
value to a careless reader, and here they mean *dead straight* and *unknown*.

**The pin that had to fail first did, and the fixture that fed P8 did not need feeding.**
`test_every_production_metric_has_a_tolerance` went red on the bare registry row, exactly as P8's
box predicted it would for any new metric. But `test_registry_entries_are_well_formed` stayed green
without being touched: its one shot already carries `launch_direction=4.0` and `carry_distance=125.6`
because `start_line_deg` and `carry_distance_yds` are already registered, so a metric derived from
two existing ones inherits its fixture. That is worth knowing for P10, which reads two fields
**nothing** currently measures and will therefore have to feed it.

**`ANALYSIS_VERSION` 8 → 9, and it is the cheapest bump on that list.** Same shape as `3 -> 4`,
`6 -> 7` and `7 -> 8` — one new `measurements` entry, no checkpoint, band or score moved. What makes
it cheaper than P8's is that the quantity is *derived from two fields a version-8 artifact already
carries*: a version-8 file was never missing the information, only the arithmetic. Nothing went red
to say the bump was owed, for the same reason as last time — every test derives the number from the
constant.

**Done when.** Tests pass. ✅ — and driven end to end, since the point is that the number reaches a
stored artifact. `scripts/reanalyze.py` moved all four stored swings `version 8 -> 9 |
measurements 18 -> 19`, and **every `overall_score` is byte-identical** to P8's recorded values
(97.45951982132875 on three, 96.88296555239968 on the fourth). The two real shots on disk miss in
**opposite directions** — `start_line_deg` −5.3° at 125.6 yd carry gives −11.6017 yd, and +4.0° at
121.0 yd gives +8.4405 yd — which is the sign working on real data in both directions rather than in
a test. `scripts/career_dispersion.py` then printed the row beside `start_line_deg` with
`target 0.000 +/- 9.000` and withheld both claims at `n = 2`, waiting on the 5- and 10-sample floors.

---

### [x] P10 — ball speed and launch angle as fitting inputs *(done 2026-08-21, written up 08-22)*

**Goal.** Record the two fields club fitting will need, judged by nothing. Pure measure-now-judge-
later (M6.5).

**Files.** `analysis/shot_measure.py`, `tests/analysis/test_shot_measure.py`.

**Detail.** `ball_speed_mph` and `launch_angle_deg`, straight reads.

**Comment to write.** Why these two and not `club_head_speed` or `smash_factor`: the existing
exclusion stands — every shot on disk reads a smash factor below 1.0, which no strike produces, so
club speed is a known-bad number on this device. These two are not implicated by that check. They
are recorded because launch conditions are the input to any future fitting model and are
unrecoverable after the fact; they get no target and no band.

**As built, and the box is later than the code.** P10 shipped in `f930973` alongside P12's work
and this box was left unchecked — it is written up here after the fact, which is why the two dates
differ. Both metrics are in `SHOT_MEASUREMENTS` as straight reads with the comment above; the
optional phase was taken rather than skipped.

**It carried the two `METRIC_TARGETS` rows with it, which was not optional.** P11's parity pin
(`test_every_production_metric_has_a_tolerance`) means an unregistered production metric is a red
suite, so `_JUDGED_LAUNCH` shipped in the same change: both rows take `target=None` — a "right"
ball speed is a fitting output and optimal launch needs the model that does not exist — leaving
the tolerance to do the one job it still can, which is the *scatter* finding. Neither number is a
fresh judgment: 2 degrees for launch angle is what this repo already claims about the OCR path's
other angle fields, and 4 mph for ball speed is `_JUDGED_YARDS`'s 5 yards carried through the
~1.4 yd/mph the two stored shots show. `_JUDGED_DEGREES` is deliberately *not* reused for launch
angle, because that constant argues from the level a coaching action follows from and nobody
coaches a launch angle off this instrument.

**`ANALYSIS_VERSION` went 9 → 10, and the disk did not follow until 2026-08-22.** For a day every
stored swing read `OUTDATED`, `Honest n per metric` printed *(none)*, and the corpus counted
nothing — the cost of a version bump without its `scripts/reanalyze.py` run. Repaired with P13:
all four swings moved to version 10, every `overall_score` byte-identical, `measurements` 19 → 21
on each.

**Done when.** Tests pass. Skip this phase entirely for the smallest useful M9. ✅

---

### [x] P11 — targets and tolerances for the new metrics *(done 2026-08-21, across P8 and P9)*

**Goal.** Let `build_dispersion` speak about the new metrics instead of refusing them.

**This phase never ran as a phase, and could not have.** The strict-equality pin means a target row
cannot lag its metric by even one commit, so P8 took two thirds of it and P9 took the last row. What
is recorded below is the argument; the code shipped with the metrics it judges.

**If P10 is ever done it ships its own two rows in the same change** — that is not a new decision,
it is the same pin. Both get no target: a "right" ball speed is a fitting output, and optimal launch
needs the model that does not exist.

**Files.** `contracts/dispersion.py`, `tests/contracts/`.

**Amended by P8 — read this first.** Two thirds of this phase is already done, and the sentence
that deferred it was wrong. `target_for` returning `None` does *not* leave a metric "measured and
permanently silent": `test_every_production_metric_has_a_tolerance` asserts **strict equality**
between the two measurement registries and `METRIC_TARGETS`, so an unregistered production metric is
a red suite, not a quiet one. P8 therefore shipped `_JUDGED_YARDS`, both distance rows and the
`METRIC_MINIMUM_N` note below. **What is left of this phase is one row**, plus P10's two if P10 is
ever done.

**Reuse.** `METRIC_TARGETS`, and now `_JUDGED_YARDS` beside `_JUDGED_DEGREES`.

**Detail.** `start_line_offline_yds` gets target `0.0` by geometry, the same argument
`start_line_deg` already makes. ✅ — shipped in P9, `tolerance=9.0` under a new `_JUDGED_OFFLINE`
provenance constant. The tolerance is **not a fresh judgment**: it is `_JUDGED_DEGREES`'s 2° carried
through the same multiplication the metric is, leaving one free parameter — which carry to evaluate
it at. The widest club in the bag decides that, because the constant has to hold for every club that
shares it: `250 * sin(2 deg)` = 8.7, rounded up to 9. The 250 is stated in the provenance rather than
left implicit, since it is the one number a bay session can replace with a measured driver carry. (Shipped in P8: `carry_distance_yds` and `total_distance_yds`, both
with **no** target — how far a golfer *should* hit a club is not a number this repo has, and a tour
carry band would judge an amateur against a population they are not in. If P10 is done, its two get
no target either: a "right" ball speed is a fitting output, and optimal launch needs the model that
does not exist.)

**The design note to write into the code.** The offline tolerance is the `_JUDGED_DEGREES` figure
evaluated at the *widest club in the bag*. Offline error scales with carry, so a single constant is
the wrong shape and the correct tolerance is per club. Erring wide is the documented safe direction
here — it costs claims, where erring narrow buys confident claims about the simulator's own noise.
Record the deferral in the provenance string.

**Do not add `METRIC_MINIMUM_N` overrides.** The defaults are right for distance and there is no
evidence to justify moving them. Say so in a comment so nobody adds one speculatively. ✅ — the
comment shipped with P8's rows.

**Tests.** Every metric in `SHOT_MEASUREMENTS` and `POSE_MEASUREMENTS` has a `METRIC_TARGETS`
entry — derive both sides from the registries (R6), so a metric added later fails loudly instead
of going silently unjudgeable. ✅ — this test already existed and is what collapsed the phase; it
was watched fail on both P8's rows and P9's.

**Done when.** Tests pass; `tests/analysis/test_dispersion.py` still green. ✅

---

### [x] P12 — the corpus carries the club *(done 2026-08-21)*

**Goal.** `CorpusSwing` knows which club hit it.

**Files.** `contracts/career.py`, `storage/corpus.py`, `tests/storage/test_corpus.py` — plus
`tests/storage/conftest.py` (the builder had to be able to say the word) and **`scripts/career_corpus.py`**,
which the box did not name; see below.

**Detail, as built.** `CorpusSwing.club: ClubId | None` declared beside `shot_sha256`;
`_corpus_swing` reads `manifest.club` inside the `CorpusSwing(...)` constructor — **not** below it,
because the `NOT_ANALYZED` branch returns early and a field assigned after that point would be
silently absent on every unanalyzed swing.

**`untagged_swings` shipped as a derived property, not the `int` field this box specified.** The box
says it is counted "for the same reason `unattributed_swings` is", and the reason carries over while
the shape does not. An unattributed manifest never becomes a `CorpusSwing` at all — it is excluded
before the swings are grouped — so that counter *must* be tallied during the scan. An untagged
manifest is a real swing sitting in `swings`, so this one is read back off them, and there is
therefore no field for P13's `narrow_to` to forget to recompute: a stored version would have been a
fifth entry in a hand-listed `model_copy(update=...)` that nothing goes red about. It also shares
`distinct_swings`' denominator, which is what lets "2 of 2 swings name no club" be printed as one
honest sentence.

**The phase's one unnamed decision: the club comes off the survivor, and there is no
`conflicting_clubs`.** A duplicate group is one clip uploaded more than once, and every upload stamps
the cursor as it stood when *that* file arrived — so a clip re-sent after the golfer moved on to a
wedge carries a wedge. The survivor is the earliest arrival, closest to the capture, and its tag
wins; this is also P5's instruction that the club appears at every site `player_id` does and at no
others, and `player_id` is read off the survivor with no fallback. Deliberately **not** mirroring
`conflicting_shots` immediately above it: a second shot photo names a repair, because whichever of
the two is misattached is attached to a swing being scored on it, whereas a duplicate's club is
attached to a directory that contributes nothing — so a `conflicting_clubs` field would have no
reader (R11). By the same argument an untagged survivor does **not** borrow a tagged duplicate's
club: that infers what hit the swing from what the cursor said when someone re-sent the clip, which
is the guess `parse_club` refuses at the boundary (R7). Repair stays per swing, via
`POST /api/sessions/{session_id}/swings/{swing_id}/club`.

**Design note, as shipped.** No `ExclusionReason` for an untagged swing, and the property's docstring
now says why rather than leaving the absence to be read as an oversight: the club was never an input
to measuring head sway, so excluding it would shrink the mechanics `n` to punish a missing tag
mechanics never needed.

**`scripts/career_corpus.py` gained a reporting block, which is why the file list grew.** A counter
nothing prints is a counter nobody notices is wrong, and that script is the corpus's own inspector —
it is also where this phase's design note reaches a human, since the block says the untagged swings
still count toward every metric above it.

**Tests: 6 added, suite 919 → 925.** Both directions of the survivor rule are written out separately,
and the P4/P5 habit is why: breaking the read in-process (`swing.club = duplicates[-1].club`) failed
**each direction on a different assertion** — `SAND_WEDGE is not SEVEN_IRON` on one, `SAND_WEDGE is
not None` on the other — so a single test would have missed one. The design-note pin was watched fail
the same way: excluding untagged swings in-process turns `{"head_sway_norm": 2}` into
`{"head_sway_norm": 1}`, which is the silently-shrunk mechanics `n` the note describes, arriving as a
visible failure.

**No `ANALYSIS_VERSION` bump**, and worth saying because P8, P9 and P10 all carried one. P12 changes
no measurement and writes no `analysis.json`, so nothing on disk became outdated and
`scripts/reanalyze.py` was not run.

**Done when.** Tests pass; the existing corpus suite is green. ✅ — 925 passed, `ruff check src tests
scripts` clean, `mypy src` clean across 92 files. Driven on the real data too, since the point is
that the field crosses into a corpus that already exists: `scripts/career_corpus.py --player-id
aaron` prints **2 of 2 distinct swings naming no club**, and every other line of its output is
byte-identical to the run before the change — `Honest n per metric` unmoved, `Contributing no sample`
unmoved, no new exclusion reason. A metric count that had moved was the failure this phase was most
likely to produce.

**Still stale for P20**, unchanged from P4–P7: `docs/ARCHITECTURE.md` §4 calls `session.json` the
"golfer cursor", its manifest row names only `player_id`, and no route table knows the four routes M9
has added. `tests/test_docs_truth.py` pins none of it, so nothing goes red.

---

### [x] P13 — `narrow_to(club=)` *(done 2026-08-22)*

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

**As built: three lines, exactly as specified.** An import of `ClubId`, `club: ClubId | None =
None` on the keyword-only signature, and `and (club is None or swing.club == club)` in the
comprehension. The `model_copy(update=...)` block was not touched — it already recomputes
`metric_counts`, `unknown_sources`, `outdated_swings` and `analyzed_without_measurements`, which
is the per-club `n` arriving honest for free.

**The docstring gained the untagged asymmetry as well as the unlock.** `read_corpus` keeps an
untagged swing on purpose, so a club narrowing is *the* place it drops out — the one view where
the tag is load-bearing. And `untagged_swings` needs no recomputation here, because P12 shipped it
as a derived property; the docstring says so at the call site the shape was chosen for, since a
stored version would have been a fifth entry in a hand-listed update that nothing goes red about.

**`contracts/dispersion.py` was deliberately not touched.** `_JUDGED_YARDS`, `_JUDGED_OFFLINE` and
`_JUDGED_LAUNCH` each name `narrow_to(club=)` as what makes their per-club tolerance *measurable*,
and that stays true after this phase: revising them needs a per-club **sample**, which needs a bay
session. All three say they should be revised in one pass, and that instruction is left standing.

**No `ANALYSIS_VERSION` bump** — no measurement changed and no artifact was written, same as P12.

**Tests.** Narrowing keeps only that club's swings **and** recomputes `metric_counts` to match —
pin the count, not just the list; that is the failure the docstring warns about. An unhit club
gives an empty corpus, not an error. Club and `since` compose. ✅ — four added, suite 925 → 929,
and a fourth the box did not ask for: a club narrowing drops the untagged swing *and*
`untagged_swings` follows it to 0, which is the assertion that goes red if that counter is ever
converted to a field.

**One test was wrong before the code was.** The first draft built three swings with distinct clubs
and asserted `carry_distance_yds` counted 3; it counted 1, because launch-monitor metrics are
keyed on the **shot photo's** hash and the builder defaults every swing to one `shot-hash`. The
reader was right — the fixture had built one shot photographed once and attributed to three
swings. Fixed by giving each swing its own shot, which is also the shape the dedupe rules describe.

**Both pins were watched fail.** Removing the clause fails all four; carrying the whole read's
`metric_counts` through instead of recomputing fails five (the four plus the pre-existing
narrowing test) on counts rather than lists — `{'carry_distance_yds': 3} == 2` is the count pin
doing the job the list pin cannot.

**Done when.** Tests pass. ✅ — 929 passed, `ruff check src tests scripts` clean, `mypy src` clean
across 92 files. Driven on the real corpus too: the whole read is 2 swings with
`carry_distance_yds` n=2, and every club narrowing comes back honestly empty with
`untagged_swings` 0, because both swings on disk predate the tag. `scripts/career_corpus.py
--player-id aaron` is byte-identical to the run before the change, which is the acceptance
criterion — nothing in that script passes `club=`, so any movement would have been a bug.

---

### [x] P14 — `contracts/club_profile.py`: the shape of a club's history *(done 2026-08-22)*

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
real history. Collapsing them into one list loses which is which. ✅ — written into the module
docstring as its own section, since `BagProfile` derives a list from each and the pair only reads
as deliberate together.

**The box named one evidence counter and the phase shipped two, which was its one design decision.**
`ClubProfile` carries `n_swings` *and* `n_shots`, mirroring the split `CareerCorpus` already makes
between `distinct_swings` and `distinct_shots`. Per club they genuinely diverge: a 7 iron filmed six
times with two shot-screen photos is six swings of history and a **carry ceiling of two**, because
every launch-monitor claim dedupes on the photo's hash. A single counter named `n_shots` would
either undercount the history or overstate what a distance statistic can be built from, and a carry
average is the number nobody audits. Both are always populated and never gated — they are facts
about how much data exists rather than claims about the golfer, which is why `MetricBaseline.n` sits
outside the guard too. `clubs_used` therefore derives off `n_swings`, so a club hit on video with no
screen photo keeps its history.

**`category` is a derived property, not the field the box listed.** `contracts/club.py` is explicit
that `CLUB_CATEGORY` is *the* one table so a club's category cannot be two things in two places; a
copy stored on every profile is a second home free to disagree with it (R4). It is a plain
`@property` and so not serialized, matching `Bag.club_ids` and `CareerCorpus.untagged_swings` — no
contract here uses `computed_field`, and P18/P19 project their own view models the way
`mcp/career.py` already does.

**`BagProfile.untagged_shots` shipped as `untagged_swings`**, because its only source is
`CareerCorpus.untagged_swings` and a number renamed on the way through is two spellings free to be
reported differently.

**Two shapes the box did not specify, both taken from precedent in this file's own contracts.**
`clubs` is a flat `tuple[ClubProfile, ...]` rather than a dict keyed by club — `ClubProfile.club`
already names the slot, and a second key is a second thing that can disagree with the entry it
holds, which is the bug `Bag._keys_match_entries` exists to catch. And a `model_validator` **pins
canonical bag order** and rejects a repeated club: `club.py` states that declaration order is read
and not decorative and that sorting downstream is the bug it exists to prevent, so a validator is
what makes that enforceable rather than a convention P15 can forget. `profile_for` is there for the
`Bag.retired_for` reason — P17's `--club 7i` and P18's `get_club_profile` are both scans over
`clubs`, and two hand-rolled ones are two places to get it wrong.

**No `ANALYSIS_VERSION` bump**, same as P12 and P13: nothing here is measured and nothing is written
to an artifact. Worth saying because P8, P9 and P10 each carried one.

**Tests.** JSON round-trip. The two derived lists are correct for all four combinations of (in
bag, has shots). ✅ — **9 added, suite 929 → 938.** The round-trip carries a real `MetricBaseline`
with both a ready claim and a `WithheldClaim` inside it, plus a `MetricDispersion` with an
`unavailable` entry, because the nesting is the phase and a refusal that fails to survive storage
renders as a blank cell — which reads as zero, the one thing `contracts/baseline.py` exists to
prevent. `category` is asserted against `category_of` across every `ClubId` rather than a written-out
list (R6), and the order pin uses `test_bag.py`'s `pw, 3w, driver, 7i` set, where insertion,
alphabetical and bag order all differ.

**Both pins were watched fail**, the P3–P13 habit. Collapsing `clubs_declared` onto `n_swings > 0`
fails the four-combination test on `[driver, 7i] != [driver, 3w]` — the two lists answering one
question. Dropping the order clause from the validator fails the ordering test with `DID NOT RAISE`,
in both the insertion-order and alphabetical directions.

**Done when.** Tests pass; `mypy src` clean. ✅

---

### [x] P15 — `analysis/club_profile.py`: the builder *(done 2026-08-22)*

**Goal.** Turn a corpus plus a bag into a `BagProfile`. **It should be short** — if it is long,
something is being reimplemented.

**Files.** `analysis/club_profile.py`, `tests/analysis/test_club_profile_builder.py` — plus the two
the ADR-008 answer required, `contracts/career.py` and `storage/corpus.py`. The box called this an
L2; the contract change makes it an **L3**, and that is the honest classification for any phase
whose design decision is "which module may import which".

**Reuse — all of it, and the box's list is one item stale.** `narrow_to` (P13), `build_baseline`,
`build_dispersion`. **Not `category_of`**: P14 made `ClubProfile.category` a derived property, so
the builder cannot set it and must not try.

**The one design decision was taken as the box prefers: the filter moved onto the contract.**
`CareerCorpus.narrowed_to(*, since, sessions, club)` now holds the body and the whole docstring;
`storage.corpus.narrow_to` keeps its name and delegates in one line, because `mcp/career.py` calls
it at three sites and it is where a caller holding a corpus off disk looks. `CorpusSwing
.artifact_key` is the precedent the box names, and it is exact — a rule both sides need lives on
the shape both sides hold.

**The move dragged a second function with it, which the box did not anticipate.** `narrowed_to`
recomputes `metric_counts`, and that recomputation *is* `storage.corpus._count_metrics`. So it moved
too, as the public `contracts.career.count_metrics`. A module-level function rather than a
`CareerCorpus` method, because `read_corpus` needs it on a bare list before there is a corpus to
call it on — it now has two callers, one on each side of the seam.

**And it gained a third derived property.** `CareerCorpus.distinct_sessions`, beside
`distinct_swings` and `distinct_shots`, so the builder's three evidence counters all come off the
narrowing rather than one being assembled by hand. Its docstring carries the warning that goes with
it: **this is not the number a TREND claim gates on.** That gate reads `MetricBaseline.n_sessions`,
which counts only sessions that contributed a sample *to that metric*, so a session whose swings are
unanalyzed raises one and not the other. Both are right, and the gap is named where it can be found.

**Two things the builder deliberately does not do**, both commented at the line rather than left to
be rediscovered:

- **`bag_entry` never reads `Bag.retired`.** The shelf is retention, not the bag-entry versioning
  ADR-024 defers, and `contracts/bag.py` states that nothing reads it. So a club that has left the
  bag keeps every statistic and loses only its loft — which is exactly the split `clubs_used` /
  `clubs_declared` exists to express. Borrowing the retired entry's loft would attach a measurement
  from one physical club to shots hit with another.
- **It does not optimise away the second `build_baseline`.** `build_dispersion` takes a corpus and
  builds its own baseline on purpose, so its guarded statistics and its raw per-session samples
  provably describe the same read; handing it one built in the loop would be the seam through which
  one club's spread could pair with another's sessions. The cost is a second pass over a swing list
  this repo counts in tens.

**The phase's one unbudgeted find: two test files cannot share a basename here, and the failure is
total.** `tests/` holds no `__init__.py`, so pytest's default `prepend` import mode imports every
test module under its bare name — and `tests/analysis/test_club_profile.py` beside P14's
`tests/contracts/test_club_profile.py` does not skip one file, it **interrupts the entire run at
collection**. The real fix, `--import-mode=importlib`, was tried and reverted: ten modules across
`tests/analysis/` and `tests/launch_monitor/` do `from conftest import ...`, which is the same
`prepend` idiom and stops resolving under it. That is its own commit and is written up in
`WORKLOG.md`. Here the file is `test_club_profile_builder.py` and its docstring says why, so nobody
restores the mirror and discovers this the hard way.

**Tests: 10 added, suite 938 → 948.** The three the box named (6 `7i` and 2 `driver` → a CENTER
claim and a refusal with the right `n`; an empty bag still profiles what has shots; a
declared-but-unhit club with every claim withheld) plus seven the phase needed — canonical bag order
from a bag declared `pw 3w driver 7i`, the two counters diverging on one club filmed three times
with two photos, a retired club keeping its history, untagged swings reaching no profile while still
counting whole-bag, an empty corpus being an empty profile, the baseline and the dispersion agreeing
on `n`, and **a static pin that this module imports no `storage`** — read with `ast` for the reason
`test_comparison.py` gives, since `analysis/__init__.py` imports `engine` and a `sys.modules` check
would answer a question about the package rather than about this file.

**Six behaviours were watched fail rather than assumed**, the P3–P14 habit, each broken in-process:
club order taken from `bag.entries` instead of walked off `ClubId` (fails the ordering test on the
`BagProfile` validator), the whole corpus handed to `build_baseline` (fails on `n`, not on the club
list — the count pin doing what a list pin cannot), the same for `build_dispersion`,
`untagged_swings` read off a narrowed corpus, `bag_entry` falling back to the shelf, and the
convenient one-line `from golf_coach.storage.corpus import narrow_to`, which fails only the new
import pin and nothing else — which is the point of having it.

**No `ANALYSIS_VERSION` bump**, same as P12–P14: nothing here is measured and no artifact is
written.

**Done when.** Tests pass; `tests/api/test_pipeline_imports.py` green — this is the phase most
likely to trip it. ✅ — and driven on the real corpus, since the point of a builder is that it
produces something: `build_bag_profile(read_corpus(...), None)` for `aaron` returns **no club
profiles and `untagged_swings` 2**, because both swings on disk predate the tag. That is the correct
output and a club appearing here would have been the bug.

---

### [x] P16 — the bag-changed caveat *(done 2026-08-22)*

**Goal.** Never silently pool two physical clubs under one name.

**Files.** `analysis/club_profile.py`, **`tests/analysis/test_club_profile_builder.py`** — not the
`test_club_profile.py` this box named, and the difference is not cosmetic. That basename collides
with P14's `tests/contracts/test_club_profile.py` under pytest's default `prepend` import mode and
**interrupts the whole run at collection** rather than skipping a file; P15 found it and wrote it
up above. The box's list was written before that was known.

**Detail, as built.** `_bag_changed_caveats(entry, swings)` — a private helper taking the entry and
the **narrowed** swing list, returning a list of zero or one sentences. Wired into `_profile_for`'s
`ClubProfile(...)` constructor beside `dispersion`. No statistic is touched, which is the phase.

**The sentence has two forms, and that was the phase's one real decision.** A single form covering
both sides reads as false on the common one. When *some* swings predate the entry there genuinely
are two populations in one average and the sentence says so. When **every** swing predates it there
is one population of unknown provenance and nothing is pooled at all — and that is not the exotic
case, it is what every club looks like the day a golfer declares a bag after months of range
sessions (which is every golfer, since nothing writes a bag until P19). Telling them their carry
average "mixes two clubs" would be alarming and wrong; what is actually in doubt is whether the
make, model and loft on the entry describe the club that hit any of it, so that form says that
instead.

**The count ships alongside the date, which is one more than this box asked for.** The sentence is
permanently true once it fires — the earliest swing never moves — so a bare date reads identically
on the day a club is declared and a year later. A proportion deflates on its own as
post-declaration history accumulates, and "2 of 40" is a different situation from "2 of 4" in the
only way a reader can act on. The all-predate form drops the count for the reason above: there is
no proportion to report when every swing is on one side.

**Three smaller calls, each commented at the line rather than left to be rediscovered:**

- **`narrowed.swings`, never `corpus.swings`.** The count has to describe *this club's* history.
  The unnarrowed list produces a caveat that is wrong about its own subject while looking exactly
  like a working one — the module docstring's opening argument, one layer down.
- **Strictly `<`.** An entry recorded in the same instant as a swing *was* the club that hit it.
  Off by one here and every bag declared during a session caveats the session it was declared in.
- **"swings", not "shots".** `n_shots` in this repo counts distinct shot *photos*, so prose saying
  "shots" beside it would name a smaller number than the profile just printed.

**Two things it deliberately does not have.** No threshold constant — `SESSION_DRIFT_FACTOR` is the
precedent for the *posture*, not the shape, and there is no "how much later is suspicious" judgment
to make here, only "later at all"; a grace window would be a free parameter with no evidence behind
it (R11). And no naive/aware guard on the datetime comparison: `CareerCorpus.narrowed_to(since=)`
makes the same bare `captured_at` comparison and does not guard either, and every writer on both
sides stamps `datetime.now(tz=UTC)`.

**Tests: 8 added, suite 948 → 956.** The three this box named, plus five the phase needed — the
`<`/`<=` boundary, the two prose forms (including the one that renders "1 of these 1 swings" if the
count is naive), a retired club staying silent rather than borrowing the shelf's `recorded_at`, and
**the load-bearing one: the caveat qualifies the statistics and never withholds them.** Six
7-iron shots with an entry recorded midway still state their CENTER mean at `n = 6`. That is the
one way this phase could have deleted real history, so it is the one test written to catch it.

**Four breaks were watched fail rather than assumed**, the P3–P15 habit. Handing the helper
`corpus.swings` fails the mid-history test on the *count* (`"2 of the 8 swings"` for a club with
four) and not on the presence of a caveat — a test asserting only "a caveat exists" would have
passed. `<` → `<=` fails the same-instant test. And withholding was broken in **two** ways, because
the obvious one is not the dangerous one: zeroing the caveat fails on `len(caveats) == 1`, while
the realistic bad fix — keep the caveat, cut the statistics back to post-entry swings — fails on
`BaselineClaim.CENTER in ready` with the mean gone to `None` at `n = 3`. Only the second exercises
the assertions that matter.

**No `ANALYSIS_VERSION` bump**, same as P12–P15: nothing here is measured and no artifact is
written. Worth saying because P8, P9 and P10 each carried one.

**Done when.** Tests pass. ✅ — 956 passed, `ruff check src tests scripts` clean, `mypy src` clean.
**Driven on the real corpus, with the honest limit stated:** `read_corpus` + `build_bag_profile`
for `aaron` is byte-identical to P15's verified result (2 distinct swings, **0 club profiles**,
`untagged_swings` 2), which is the useful check — nothing moved. It exercises no caveat, and
nothing on disk can: every swing predates the club tag and no bag exists. The caveat's own drive
was one real stored swing retagged `7i` in a REPL with a bag entry recorded a day later, which
produced the sentence and left `carry_distance_yds` refusing at `n = 1` exactly as it did without
the bag. P17's CLI is the first thing that will render one for real.

---

### [x] P17 — `scripts/club_profile.py` CLI *(done 2026-08-22)*

**Goal.** Read the numbers without a browser or an MCP client. **The phase that proves the spine
works.**

**Files.** `scripts/club_profile.py` — plus a two-line carry-over into `scripts/career_baseline.py`
and `scripts/career_dispersion.py`, argued under *formatting* below.

**Reuse — all of it.** `career_baseline.py` and `career_dispersion.py` are the templates and were
copied structurally: the target-resolution block, `_FINDING_LABEL` with its reasoning, and
`_label` / `_print_block` / `_wrap`. Off the contracts: `read_corpus`, `BagStore.get`,
`build_bag_profile`, `BagProfile.profile_for` (P14 put the lookup there so this and P18's
`get_club_profile` cannot disagree about "no such club"), and `parse_club` for `--club`.

**The argument shape follows the templates, not this box's own sketch.** The box wrote
`club_profile.py <player>`; the three career CLIs all take `--name` / `--player-id`, and its own
Reuse line said "same argument shape". Four CLIs over one corpus that disagree about how to name a
golfer is the worse outcome, so the flags won. `--club` accepts anything `parse_club` does and
**refuses rather than nudges** — "wedge" and "iron" name a category, so both exit 2 with the
sentence saying so.

**No tests, and that is the precedent rather than an omission.** `tests/` has no `scripts/` mirror
and none of the three career CLIs carries one. Acceptance is the real-data drive.

**What this phase actually found: the box's Done-when could not happen.** It predicted "a table of
refusals". `build_bag_profile` for `aaron` returns **zero club profiles** — a club with no swings
and no bag entry gets no row, so there is nothing to refuse *about*. So the deliverable grew a
branch the box did not have, `_report_empty`, which says how many untagged swings exist and where a
tag comes from. The two states need opposite responses — a refusal table means go and hit balls, an
empty profile means nothing on disk names a club at all — and a golfer's name followed by a blank
would read as a bug in the CLI when the finding is about the disk.

**One decision inside the output, and it was checked rather than assumed.**
`MetricDispersion.withheld` is not printed. `analysis/dispersion.py`'s `_carry_refusals` builds it
by **filtering `MetricBaseline.withheld`**, so every sentence in it is already printed verbatim
beside it — verified in a REPL across two metrics (`d <= b` for both), not read off the source and
hoped. On output that is nothing but refusals, rendering both would double every metric block to
say each thing twice. The finding labels still read "withheld", and `unavailable` — a different
list saying a different thing, which no bay session fixes — is printed, last.

**The formatting rule is the one thing the templates could not supply.** Both spell it
`1dp if unit == "degrees" else 3dp`, written when every other metric here was
shoulder-width-normalized. P8–P10 added `yards` and `mph`, and `ms` was already there, so that rule
prints `154.400 yards`. Replaced by a unit → precision table, and **carried back into both career
CLIs** rather than left to disagree: `career_dispersion.py` has printed both distances since P8, so
the wart is already theirs and only latent because no center has cleared its floor yet. Three
copies of a six-entry dict, because `scripts/` is not a package and a per-script `_fmt` is the
standing shape here.

**Done when.** ✅ 956 passed (unchanged — no test moved and nothing importable did), `ruff check src
tests scripts` clean, `mypy src` clean. **Driven on the real corpus:** `aaron`, zero club profiles,
`untagged_swings` 2 — byte-identical to what P15 and P16 both verified, now rendered as the empty
state. `--club 7i` → "never hit and not in the bag", exit 0; `--club wedge` → exit 2;
`--player-id nobody` → the "(not registered)" path, exit 0.

**And driven populated, which is what a renderer has to be.** Nothing on disk can produce a club
profile, so two scratch drives: the real stored swings retagged `7i` with a bag entry recorded a day
later (P16's all-predate caveat, its first render outside a test, plus a declared-but-unhit driver),
and a synthetic 12-swing 7-iron history across 4 sessions with the entry recorded midway. The second
is what proves the formatting change: it prints `center 154.4` in yards beside `center 3.014` for
`tempo_ratio`, where the template's rule would have printed `154.400`. It also rendered P16's
*mixed* form — "9 of the 12 swings…" — which no other drive has produced.

---

### [x] P18 — MCP tools *(done 2026-08-22)*

**Goal.** Claude can answer "how far do I hit my 7 iron".

**Files.** `mcp/club.py` (new), `mcp/server.py`, `mcp/runner_tools.py`, `mcp/query.py`,
`contracts/tool_descriptions.py`, `contracts/caveats.py`, `scripts/run_mcp_server.py`,
`tests/mcp/test_club_tools.py`, plus pins in `tests/contracts/test_tool_descriptions.py`,
`tests/mcp/test_server.py`, `tests/mcp/test_query.py`, `tests/mcp/conftest.py` and
`tests/test_docs_truth.py`.

**A new module, not an extension of `career.py`.** The box allowed either. `career.py` is ~590
lines and cohesive, and the club shapes, the notes and the misses are their own thing;
`mcp/club.py` reaches into it for `Refusal`, `resolve_golfer`, `THE_UNBLOCK`, `_refusal`, `_points`
and `_low`/`_high`, exactly as `career.py` reaches into `query._missing`.

**`ClubMetric` is not `career.MetricProfile`, and that is the shape decision.** `MetricProfile`
carries the tour join, and there is no per-club tour join — ADR-024's own trap says `ranges.json`
holds `club_category: "all"` rows only. Reusing the shape would have shipped
`tour_standing: "withheld"` on every metric of every club forever: a refusal of a claim nothing
ever intended to make, which reads to a model as "ask again with more data". So the baseline and
dispersion halves are mirrored field for field and the tour block is absent. The duplication is the
price of not fabricating a refusal, and `test_no_metric_carries_a_tour_placement` pins it.

**The phase turned out to be about counting silences, and driving it populated found two more.**
The box named the refusal case; P17 had already found the untagged case. Rendering a populated bag
found the other two, both of which the first cut got wrong by giving them the refusal sentence:

  1. `NOTHING_TAGGED` — no club profiles at all. **Tagging** fixes it, not a bay session.
  2. `NOT_ENOUGH_ON_THIS_CLUB` — a club whose every figure is withheld. Shots on it fix it.
  3. `NEVER_HIT` — never hit and not in the bag. An answer, not a `NotFound`.
  4. `IN_BAG_NEVER_HIT` — declared today, never hit. Nothing was claimed, so nothing was refused,
     and saying "every claim is withheld" would be a refusal this module invented — the same
     fabricated refusal the tour block is left out to avoid, one field over.
  5. `NO_MEASUREMENTS` — swings that contributed no value. Not about the club at all.

Plus `SOME_SWINGS_UNTAGGED`, appended when untagged swings sit beside a bag that does have clubs:
both things are true at once, and a note carrying only one describes half the situation as the
whole of it. `scripts/club_profile.py` draws the same distinctions on screen, which is what made
the first three obvious and the last two findable — a payload less honest than the dev CLI is the
wrong way round.

**The descriptions and the briefing.** `GET_BAG_PROFILE` and `GET_CLUB_PROFILE` in
`contracts/tool_descriptions.py`, plus a new `caveats.READING_A_BAG` gated on the same registry
flag as `READING_A_PERSONAL_HISTORY` and shipped beside it, never instead — it adds only what a
club changes and leans on that block for what a withheld claim is. `CAREER_TOOL_NAMES` kept its
meaning; `CLUB_TOOL_NAMES` and `REGISTRY_TOOL_NAMES` joined it, because the gate is "a registry
exists" and it now covers five tools rather than three.

**One carry-over outside the box: `SwingView.club`.** `get_swing` already loads the manifest that
carries it, and a model able to ask about a 7 iron but unable to see which club hit a given swing
is a gap with a four-line fix. `None` is explained in the field description rather than left to
read as a data error.

**Done when.** ✅ 979 passed (956 → 979, +23), `ruff check src tests scripts` clean, `mypy src`
clean. **Driven on the real corpus:** the server advertises **10** tools with a registry
configured, `get_bag_profile(player="aaron")` returns the empty state with `untagged_swings` of 2 —
byte-identical to what `scripts/club_profile.py` prints — `get_club_profile(player="aaron",
club="7 iron")` resolves the spelling and reports `NEVER_HIT`, `club="wedge"` is refused as a
category, and an unknown golfer is the `NotFound` shape.

**And driven populated**, P17's precedent: the real swings retagged `7i` with a bag entry recorded
a day later (P16's all-predate caveat, a declared-but-unhit driver on `IN_BAG_NEVER_HIT`), and a
synthetic 12-swing 7-iron history across 4 sessions with the entry recorded midway — `center 154
yards`, `sd 3.133`, and P16's *mixed* caveat, "3 of the 12 swings…".

---

### [x] P19 — the bag page *(done 2026-08-22)*

**Goal.** A browser view of the bag — and the first thing in this repo that ever wrote one.

**Files.** `api/app.py`, `api/static/career.html`, `tests/api/test_bag_route.py` — plus two
carry-overs argued below, `api/static/results.html` and `scripts/club_profile.py`.

**Reuse.** `GET /api/golfers/{player_id}/career` was the route template and `career.html` the page
template, both as the box says. Off the contracts: `read_corpus`, `BagStore.get`/`set_entry`/
`remove_entry`, `build_bag_profile`, `_resolve_club` (so `parse_club` gets its fourth caller), and
`GET /api/clubs` from P7 for the "add a club" picker.

**The box's first open question, settled with the user: a section inside `career.html`, not a new
`bag.html`.** One page per golfer. The bag sits **above** the whole-bag cards, and that order is
M9's own argument rendered as layout — a carry pooled across a driver and a wedge describes
nobody's shot, so a page leading with whole-bag numbers re-teaches the reading this milestone was
built to correct. The old `Metrics` heading became `Whole bag`, because there are now per-club
metrics on the same page and the bare word stopped distinguishing them.

**The box's second, also settled with the user: set and remove, not restore.** `POST` and `DELETE`
on `/api/golfers/{player_id}/bag/{club}`. The shelf is still written by the store and still not
served — `BagProfile` carries none by P15's decision, and `Bag.retired_for` therefore still waits
for its second caller.

**The one route-shaped decision: this route cannot serve its contract raw, and the career route
can.** `ClubProfile.category`, `clubs_used` and `clubs_declared` are plain properties, which P14
chose deliberately while naming this surface as the projector. `category` is the half that matters:
a page deriving it in JavaScript would hold a second copy of `CLUB_CATEGORY` in a static file
nothing tests — the exact failure `GET /api/clubs` was added in P7 to prevent, and quiet in the same
way, since a club added to `ClubId` would keep working at every route while rendering under the
wrong heading at the bay. So `_bag_summary` projects it beside `_corpus_summary`. The two lists
travel as **counts**: the page has every use for "10 clubs, 8 of them hit" and none for a second
copy of each profile, so the membership rule stays on the contract.

**Both writers return the whole bag, not the row they saved.** That gives the page one render path,
and it means a save shows exactly what a reload would — including P16's caveat, which *declaring an
entry is what creates*. A response assembled from the request is the one place that caveat could go
missing, and it would then appear from nowhere on the next visit. The cost is one `read_corpus` per
write, which is what the career route already pays per read.

**The phase found a helper it did not expect to need.** Three new routes plus the career one all
open with the same two steps — validate the id, then know the golfer — and `player_id` reaches a
filesystem path in every one of them. That is now `_registered`, and the career route was moved onto
it: four hand-rolled copies is four places for one to be forgotten, and the one that forgets looks
exactly like the three that do not.

**`BagEntryRequest` carries no `recorded_at` and no `club`, and both absences are deliberate.** The
store owns the clock (`bag_store.py`), so a field for it would be a value the API accepts and
silently ignores. The club is the path segment, so `Bag._keys_match_entries` can never be handed a
7 iron's row claiming to be a wedge — not putting it in two places is what stops it being asked to.

**Two carry-overs, both honesty fixes rather than features.**

- **`fmt` printed `154.400 yards`, on two pages.** `career.html` and `results.html` both spelled the
  rule `1dp for degrees, 3dp otherwise`, written when every metric here was shoulder-width
  normalized; P8–P10 put `yards` and `mph` into `measurements`. P17 hit this in the CLIs and carried
  a unit → precision table into both rather than let them disagree, and this is the same carry to the
  two pages. `results.html` is not this phase's subject and its `num()` helper, which strips trailing
  zeros for the measurements table, was left alone.
- **`scripts/club_profile.py` pointed at a button that does not exist.** Its empty-state message said
  a stored swing is retagged "which the same page does with one tap". Driving this phase found that
  `POST /api/sessions/{s}/swings/{sw}/club` has **no UI on any page** — `index.html` has a per-swing
  *golfer* repair control (`button.golfer-fix`) and no club counterpart. Both the CLI and the new
  page now say the honest thing: there is no button yet. **That gap is the real blocker on M9
  speaking**, and it is named in the roadmap rather than fixed here.

**The five silences are rendered in the page's own voice**, with a comment pointing at `mcp/club.py`
as where the taxonomy of five is stated. The wording differs per surface on purpose — the CLI points
at a shell command, the MCP note at a tool, the page at a control someone can tap — but the
distinctions must not, so the enumeration lives in one place and this is a rendering of it.
`NEVER_HIT` has no branch: a club nobody has hit and nobody declared has no row, and its absence is
the answer.

**Tests: 14 added, suite 979 → 993**, in `tests/api/test_bag_route.py`. The envelope (bag order from
a bag where insertion, alphabetical and canonical all differ; `category` asserted against
`category_of` and never a written-out list; a withheld claim absent with its reason; untagged swings
reaching no row; the two counters diverging), the gates (404 on every verb, the slug guard, the
token), and the writes — which is the half nothing in this repo had ever exercised.

**Two of the new tests were wrong before the code was, and both taught something.**

- The counter-divergence test first built two swings *sharing* one shot photograph. They cannot:
  `assign_from_path` dedupes an identical upload back onto the swing that already holds it, so the
  second swing silently got no shot file, no `analysis.json` and no metrics at all. The real shape of
  the `n_swings > n_shots` gap is a clip filmed without the screen being photographed, and the
  fixture now builds that.
- The traversal test copied `test_career_route.py`'s `..%2F..%2Fetc` form — **and that form never
  reaches the guard.** httpx normalises it away before it is sent, so the assertion passes on a
  routing 404 and `_safe` is never called. This file pins the character class instead, which is the
  guard actually speaking. `tests/api/test_career_route.py` still carries the weaker form; it is not
  wrong, it is just not testing what it reads as.

**Three behaviours were watched fail rather than assumed**, the P3–P18 habit. Forcing a write past
`same_club_as` fails the unedited-save test on a moved `recorded_at` — the failure that would
otherwise reach a golfer as a caveat produced by the act of looking at the page. Assembling the POST
response from the request instead of re-reading fails the save-equals-reload assertion. And
`del bag.entries[club]` in place of `remove_entry` fails the removal test on an empty shelf.

**No `ANALYSIS_VERSION` bump**, same as P12–P18: nothing here is measured and no artifact is written.

**Done when.** `pytest tests/api/` green; the page renders against the data on disk showing
refusals. ✅ — 993 passed, `ruff check src tests scripts` clean, `mypy src` clean across 95 files.
The page's own script was run against a live server with a stubbed DOM, since nothing tests a static
file, in both states:

- **Against the real corpus**, the empty state — 0 clubs, `untagged_swings` 2 — byte-identical in
  substance to what `scripts/club_profile.py` prints and what `get_bag_profile` returns. Three
  surfaces over one builder, agreeing, which is what P18 was accepted on.
- **Against a seeded corpus**, populated: a 7 iron over 13 swings, 12 shot photographs and 4
  sessions rendering `carries 154.3 yards over 12 shots`, its `sd 3.1`, P16's all-predate caveat, and
  the two counters visibly diverging with the ceiling line beside them; a driver on one shot with
  every claim withheld; a declared-but-unhit 3 wood on the never-hit note. The precision carry-over
  is visible in that same output — `154.3` and `0.2` in yards beside `3.024` for `tempo_ratio`, where
  the old rule printed `154.258`.

**Still stale for P20**, unchanged from P4–P18: `docs/ARCHITECTURE.md` §4 calls `session.json` the
"golfer cursor", its manifest row names only `player_id`, and no route table knows the **seven**
routes M9 has now added. `tests/test_docs_truth.py` pins none of it, so nothing goes red.

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
5. `python scripts/club_profile.py --player-id aaron` — expect refusals with correct `n`s, not
   blanks, and before any swing is tagged the empty state rather than a table (see P17).
6. Via MCP: `get_club_profile(player="aaron", club="7i")`, and confirm the refusal reads honestly.
   As of P18 the honest answer on today's disk is not a refusal at all — it is "never hit and not
   in the bag", and `get_bag_profile` says nothing on disk names a club. Both are answers; a
   sample-size refusal appears with the first tagged swing.

**The correct result at every stage is refusal**, because there is not enough data on disk yet. A
number appearing early is the bug — the same acceptance criterion career mode used. The feature
turns on when a bay session puts five or more shots on a club.
