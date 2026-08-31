# M12 — Club Specs: the golfer names a club, the program determines what it is

> **Tier: TARGET.** This is a plan and almost none of it is built — verify against code before
> trusting a detail. The *why* is [ADR-026](decisions/026-club-specification-lookup.md), written by
> this milestone's P0; the decision it **reverses** is
> [ADR-024](decisions/024-per-club-shot-history.md) §2, whose 2026-08-31 addendum records the
> reversal. Read both addenda on ADR-024, not just the ADR.

**Status: 1/8 phases built** — P0 wrote ADR-026, ADR-024's second addendum and this document, so a
phase number is now a complete handoff. P1–P7 are the build and none of them has started. Every
number in §Design is a field list or a path, not a measurement; the first measurements this
milestone produces arrive in P4's *As built*.

## What this milestone is

**The bag is a blank form and it has stayed blank.** `data/processed/golfers/` holds
`aaron.golfer.json` and no `.bag.json` at all. M9 P2/P3 built `contracts/bag.py`, P19 built the row
form in `api/static/career.html`, and the write route at `api/app.py:693` has worked since. Nothing
is broken. The form has simply never been filled in, which is what a form asking for a lie angle
gets.

**And the five fields it asks for are not what a club is.** `BagEntry` carries `loft_deg`, `make`,
`model`, `shaft` and `length_in` (`contracts/bag.py:52`). A Titleist T150 7 iron and a TaylorMade
Stealth 7 iron differ in loft, lie, length, shaft weight, shaft profile, offset, bounce and head
construction, and every one of those moves launch, spin and start line. `shaft` is one free-text
string standing in for six independent facts. A bag filled in *perfectly*, exactly as the shape
allows, still cannot tell a downstream model whether a 7 iron is a 30.5° players iron on a 120 g
steel shaft or a 27° game-improvement iron on 60 g graphite.

**Scope: the golfer enters a club, and the program determines and saves its full specification.**
Typing "Titleist T150, 7 iron" is a thing a golfer can do standing in their garage. The lookup fills
the rest, the golfer confirms it, and it lands in the bag with a provenance block saying where each
answer came from.

**Outcome:** a `.bag.json` that exists, holding real specifications with their provenance; one field
list rather than two; and the inputs the deferred ball-flight and fitting models need, recorded
before the bay session that would otherwise make them unrecoverable.

**Not in scope, deliberately:** ball trajectory, swing efficiency, gapping and club fitting. ADR-024
defers all four and ADR-026 keeps them deferred. The flight model has a named blocker of its own —
`spin_axis` has no direction word on the HD Golf screen, and ADR-014's addendum records two fades
stored as draws. M12 lands the *inputs*, which is the ordering ADR-024 used for loft: the inputs are
unrecoverable after the fact and the models are not.

## Three things that will look obvious and are wrong

- **"Never a catalogue default" is no longer the rule.** `bag.py:74` says it today and ADR-024 §2
  argues for it, and both are **superseded** by ADR-026 §1. If the manufacturer says the loft is X,
  the loft is X. This is the one place in the milestone where the obvious move — preserve the
  existing invariant — is the wrong one, so P1 and P2 must be read against ADR-024's second
  addendum rather than against §2 itself. ADR-010 §2 is *not* weakened: nothing is guessed, and a
  spec the model will not commit to still comes back `None`.
- **`same_club_as` compares by exclusion, and that is a trap for exactly one new field.** Its
  design — everything that is not a timestamp counts toward club identity — is right for all two
  dozen spec fields and wrong for `provenance`, which carries a retrieval time. Miss it and a second
  lookup of an unchanged club retires the good entry and hands M9 P16 a false bag-changed caveat.
  ADR-024's *first* addendum already names that false positive from the other direction. **P2 is
  where this is fixed and it is the regression most likely to slip.**
- **The migration is free exactly once.** No `.bag.json` exists anywhere, so splitting `shaft: str`
  into six fields costs nothing today and costs a migration the day after the first bag is saved.
  That is the argument for doing the split in P2 rather than adding fields beside the free-text one
  later — and it is why P6 (the page that writes the first bag) comes after P2 and not before it.

## Reading order for a fresh session

`CLAUDE.md`, then **ADR-024 §2 and both of its addenda**, then
[ADR-026](decisions/026-club-specification-lookup.md), then this file, then the two-to-four files
your phase names. Every phase states its own Goal, Files, Tests and Detail, so a phase number is a
complete handoff and nobody re-reads the repo to start.

**This is an L3 — STRUCTURAL change** by CLAUDE.md's ladder: it changes a `contracts/` shape and
adds a subsystem. Read [ADR-008](decisions/008-project-structure.md) (import rules),
[ADR-010](decisions/010-benchmark-ranges.md) §2 (no score beats a wrong one),
[ADR-022](decisions/022-learned-artifacts-as-committed-data.md) (artifacts ship as committed data)
and `docs/ARCHITECTURE.md` §2–§3 before touching `contracts/bag.py`.

**Before P2, read every consumer of `BagEntry`** — the shape is changing underneath all of them:

| Consumer | What it does with a `BagEntry` |
|---|---|
| `src/golf_coach/storage/bag_store.py` | Writes it, and owns the clock and the retirement shelf |
| `src/golf_coach/contracts/club_profile.py` | Holds one as `ClubProfile.bag_entry` |
| `src/golf_coach/analysis/club_profile.py` | Composes it into the per-club profile |
| `src/golf_coach/mcp/club.py` | Renders it to an MCP client |
| `src/golf_coach/api/app.py` | `BagEntryRequest` (`:119`) builds one; the route at `:693` saves it |
| `scripts/club_profile.py` | Prints it |
| `src/golf_coach/api/static/career.html` | `ENTRY_FIELDS` (`:607`) is a hand-written copy of the field list |

Their tests, which are what will fail first: `tests/contracts/test_bag.py`,
`tests/contracts/test_club_profile.py`, `tests/storage/test_bag_store.py`,
`tests/analysis/test_club_profile_builder.py`, `tests/mcp/test_club_tools.py`,
`tests/api/test_bag_route.py`, `tests/api/test_uploads.py`.

---

## Design

### 1. `ClubSpec` — one home for the field list

New `src/golf_coach/contracts/club_spec.py`. The unit is a **slot of a model**, not a model: a set
has a different loft, lie and length in every slot, so "Titleist T150" is not a specification and
"Titleist T150 7 iron" is (ADR-026 §2).

Every field is `None` or `""` when unknown. Nothing is defaulted, nothing is interpolated between
slots, and nothing is guessed.

| Group | Fields |
|---|---|
| Identity | `club: ClubId` · `make` · `model` · `model_year: int \| None` · `head_type` · `set_composition` |
| Head | `loft_deg` · `lie_deg` · `bounce_deg` · `grind` · `offset_mm` · `face_angle_deg` · `head_weight_g` · `adjustable_hosel: bool \| None` · `loft_range_deg: tuple \| None` |
| Shaft | `shaft_model` · `shaft_material: ShaftMaterial \| None` · `shaft_flex: ShaftFlex \| None` · `shaft_weight_g` · `shaft_torque_deg` · `shaft_kick_point` |
| Assembly | `length_in` · `swing_weight` · `total_weight_g` · `grip` |
| Head performance | `cor` · `moi_g_cm2` · `usga_conforming: bool \| None` |

Two `StrEnum`s beside it, following `ClubId`'s pattern in `contracts/club.py`: `ShaftMaterial`
(`STEEL`, `GRAPHITE`, `MULTI_MATERIAL`) and `ShaftFlex` (`LADIES` … `XX_STIFF`). Each gets a
tolerant `parse_*` at the boundary the way `parse_club` (`club.py:185`) does — "S", "stiff" and
"Stiff Flex" are one flex — and **refuses rather than nudges** on a miss, returning `None` for text
it does not recognise.

`SpecProvenance` lives here too: `source: str` (`"llm:claude-opus-5"`, `"catalogue"`, `"typed"`),
`retrieved_at: datetime`, `notes: str`.

### 2. `BagEntry` inherits `ClubSpec`, and `shaft` splits

`class BagEntry(ClubSpec)`, adding only `recorded_at`, `retired_at` and
`provenance: SpecProvenance | None`. **One field list, two shapes.** A hand-copied second list is
the R4/R6 failure this repo already fixed once — `mcp/query.py`'s `_METRIC_FIELDS`, recorded in
`docs/REFACTOR_LEDGER.md`.

`shaft: str` splits into the six shaft fields in §1. **Migration is free — no `.bag.json` exists on
disk.**

**`same_club_as` (`bag.py:103`) must exclude `provenance`, or this breaks:**

```python
timestamps = {"recorded_at", "retired_at", "provenance"}
```

The method compares by exclusion, which is right for every spec field and wrong for a block
carrying `retrieved_at`. Without the exclusion, re-looking-up the same club mints a new timestamp,
reads as a different physical club, retires the good entry onto `Bag.retired`, and hands M9 P16 a
false bag-changed caveat over shots all hit with the same club.

### 3. `src/golf_coach/clubs/` — a new subsystem, two modules

Imports `contracts` only (ADR-008). Does not import `analysis`, `storage` or `api`.

**`clubs/catalogue.py`** — the committed catalogue. `club_catalogue.json` ships inside the package,
following ADR-022's "fitted offline, ships as data" pattern that `analysis/benchmarks/ranges.json`
and `golfdb_v1.json` already use; every row carries its own provenance the way `ranges.json` rows
do.

- `catalogue_key(make, model, model_year, club) -> str`. **Reuse `slugify` from
  `contracts/golfer.py:54`** rather than writing a second normaliser — it exists because
  "Aaron"/"aaron" splitting a golfer in two is silent, and "T150"/"T-150" splitting a club is the
  same failure.
- `load_catalogue()`, `lookup(make, model, year, club)`, `remember(spec)` — so the next lookup of an
  accepted club is offline.

**`clubs/lookup.py`** — the LLM half, mirroring `feedback/coach.py` because that pattern is proven
and already pinned:

- `_sdk()` lazy-imports `anthropic` inside the function (`coach.py:311`) — the module must import
  clean on a base install or `tests/api/test_pipeline_imports.py` fails.
- `look_up_club(...)` takes the same `client=` injection seam (`coach.py:341`), so tests assert
  request and parse shape with no network and no `llm` extra installed.
- **Never raises for an expected failure.** Missing extra, missing key and API error each return a
  result carrying a `note`, the way `CoachingOutcome` does.
- The prompt must instruct: **return null for any spec you are not confident of.** A hallucinated
  lie angle is worse than a blank, because a blank is visible and a wrong number is not.
- `look_up_set(make, model, year, slots)` returns one spec per slot **in one call** — that is what
  keeps the per-slot loft progression internally consistent.

No new extra and no new config: reuse the `llm` extra, `settings.anthropic_api_key` and
`settings.coaching_model`. **`api/app.py` is already in `SANCTIONED_UNWRAP_SITES`**
(`tests/test_config.py:32`), so the key is unwrapped at the route and passed in as a plain `str` —
no change to that pin.

### 4. API — one new route, one extended

- **`POST /api/clubs/lookup`** (new). Body: `make`, `model`, `model_year`, and either `club` or
  `slots`. Returns candidates **without writing anything** — catalogue first, LLM on a miss. The
  write being a separate act is what makes the confirm step real (ADR-026 §5).
- **`POST /api/golfers/{player_id}/bag/{club}`** (`app.py:693`, extended). `BagEntryRequest`
  (`app.py:119`) grows to carry every `ClubSpec` field. Route shape unchanged — club off the path,
  store owns the clock, response is `_bag_for()` — plus a `catalogue.remember(...)` on save.
- Set fan-out is **N calls to the existing per-club route** from the page, not a bulk route. One
  writer; `Bag._keys_match_entries` still checks per club; a partial failure leaves a partially
  declared bag, which is the honest outcome.

`_resolve_club` (`app.py:179`) is reused for every club string, so `contracts/club.py` stays the
only place free text becomes a `ClubId`.

### 5. `career.html` — the bag section grows a search step

The bag section already exists: render at `:615`, field list `ENTRY_FIELDS` at `:607`, delegated
click handler `onBagClick` at `:794`, add control `addBlock` at `:784`. Hand-written HTML, no build
step — `docs/REFACTOR_LEDGER.md` declined a toolchain here.

`addBlock` gains a make/model/year search box beside the slot picker; a results panel renders the
returned spec as a **filled-in, editable form** grouped as in §1, with unknown fields visibly blank
rather than zero-filled; save posts to the existing per-club route; set entry renders one row per
slot with a single confirm. `ENTRY_FIELDS` becomes a grouped `SPEC_FIELDS`, one place per shape.

---

## Phases

Each phase is independently commit-ready. `tests/` mirrors `src/golf_coach/` package by package, so
a module's test file is its path with `tests/` on the front. **P1–P4 are testable with no browser
and no network. P5 onward needs the server.**

### [x] P0 — write the milestone into the repo's own docs

**Goal.** Land the *why* and the *how* so a fresh session can execute a phase number without
rediscovering the repo, and record the ADR-024 reversal where a reader of §2 will see it.

**Files.** `docs/decisions/026-club-specification-lookup.md` (new);
`docs/decisions/024-per-club-shot-history.md` (addendum); this file (new); `ROADMAP.md` (a row in
*Status at a glance* and an `## M12` section); `docs/README.md` (M12 and ADR-026 rows, the document
and addenda counts, and ADR-024's per-row count to **2**).

**Tests.** None new. `tests/test_docs_truth.py` needs no edit — it derives its counts from
`docs/README.md`'s own prose, so the README numbers *are* the pin, and it fails until they are
bumped.

**Detail.** No `src/` changes. The counts that move in `docs/README.md`: total markdown documents,
the `docs/` breakdown (top-level and ADRs), the decision count, the addenda total, and ADR-024's
addenda cell. The new files must be `git add`ed before the count test agrees, because it counts
through `git ls-files`.

**As built.** Written 2026-08-31 exactly as listed. Two things worth recording.

*The reversal went in as an addendum and not an edit.* ADR-024 §2's "never a catalogue default"
sentence and `bag.py:74`'s field description both still say what they said; the addendum says they
are retired and points at ADR-026 §1. That is this repo's convention for a decision reality
corrected, and it is the reason a reader of §2 alone would otherwise implement the opposite of this
milestone.

*The `same_club_as` provenance exclusion is written down in three places on purpose* — ADR-026 §4,
ADR-024's addendum, and §Design 2 here. It is one line of code and the only defect in this plan that
produces a *wrong statistic* rather than a failure, so it is stated where each of the three likely
readers will be.

### [ ] P1 — `ClubSpec`, the two shaft enums, and their parsers

**Goal.** One field list, with a tolerant boundary for the two vocabularies.

**Files.** New `src/golf_coach/contracts/club_spec.py`.
**Tests.** New `tests/contracts/test_club_spec.py`.

**Detail.** The field groups in §Design 1, `ShaftMaterial` and `ShaftFlex` as `StrEnum`s beside
`ClubId`, `parse_shaft_material` / `parse_shaft_flex` built the way `parse_club` (`club.py:185`) is
— an alias table, one normalisation pass, and `None` on a miss rather than a nearest match.
`SpecProvenance` lands here.

Docstrings carry the *why*, as `bag.py`'s do: why the unit is a slot of a model and not a model, why
every field is optional, and why a parser that refuses beats one that nudges.

Pin in tests: `"S"`, `"stiff"`, `"Stiff Flex"` and `"  STIFF  "` are one flex; `"regular-ish"` is
`None` and not `REGULAR`; a `ClubSpec` with nothing but a `club` is valid and reads `None`
everywhere else.

### [ ] P2 — `BagEntry(ClubSpec)`, the shaft split, and the provenance exclusion

**Goal.** The bag entry stops carrying its own copy of the field list, and `same_club_as` stays
correct across the new fields.

**Files.** `src/golf_coach/contracts/bag.py`.
**Tests.** `tests/contracts/test_bag.py`, `tests/storage/test_bag_store.py`.

**Detail.** `class BagEntry(ClubSpec)` keeping only `recorded_at`, `retired_at` and `provenance`.
Delete the five descriptive fields; they come from the base. `shaft: str` goes, replaced by the six
shaft fields.

**Add `"provenance"` to the `timestamps` set in `same_club_as` (`bag.py:120`)**, and update the
docstring's "excluding the two timestamps" to say three and to say why the third is one.

Update `bag.py:74`'s `loft_deg` description — it currently states the rule ADR-026 §1 reverses — and
point it at ADR-026 §1 and ADR-024's 2026-08-31 addendum. The module docstring's ADR-024 §2
reference needs the same treatment.

**The new test this phase owes, and it is the point of the phase:** a second lookup of the same
club, saved again, does **not** move `recorded_at`. Build two entries identical but for
`provenance.retrieved_at`, assert `same_club_as` is `True`, then drive it through
`BagStore.set_entry` and assert `Bag.retired` is still empty.

Then walk the consumer table in §Reading order and fix what the shape change broke.

### [ ] P3 — the committed catalogue and its key

**Goal.** An accepted specification is remembered, so the second lookup of a club is offline.

**Files.** New `src/golf_coach/clubs/__init__.py`, `clubs/catalogue.py`,
`clubs/club_catalogue.json`.
**Tests.** New `tests/clubs/test_catalogue.py`.

**Detail.** `catalogue_key` normalises through **`slugify` (`contracts/golfer.py:54`)** — not a
second normaliser. `load_catalogue` reads the packaged JSON tolerantly: an unreadable or absent
catalogue is an empty one, never an error, because a corrupt catalogue must cost a cache hit and
never a lookup.

Each row carries its own provenance, the way `ranges.json` rows do, so a row that came from a model
is distinguishable from a row someone typed off the manufacturer's page (ADR-022, ADR-026 §7).

The file ships **empty or nearly so**. A catalogue is worth committing once it has been checked.

Pin in tests: `"T150"`, `"T-150"` and `"t 150"` are one key; a row round-trips through
`remember` → `load_catalogue` → `lookup`; a truncated JSON file reads as an empty catalogue with no
exception.

### [ ] P4 — the LLM lookup, single club and set

**Goal.** A make, a model and a slot become a specification — or a blank with a note, which is the
honest failure.

**Files.** New `src/golf_coach/clubs/lookup.py`.
**Tests.** New `tests/clubs/test_lookup.py`.

**Detail.** Mirror `feedback/coach.py` structurally: `_sdk()` lazy-imports `anthropic` inside the
function (`coach.py:311`); `look_up_club(...)` and `look_up_set(...)` take a `client=` seam
(`coach.py:341`) so every test runs with no network and no `llm` extra; an expected failure returns
a result carrying a `note` rather than raising, the way `CoachingOutcome` does.

The prompt instructs the model to **return null for any spec it is not confident of**, and the parse
keeps the null — a `None` is a refusal and must never become a `0.0`. `look_up_set` asks for every
slot in one call so the loft progression is internally consistent.

Pin in tests: a fake client returning a spec with nulls parses to a `ClubSpec` with `None` in those
fields and not zeros; a missing key returns a noted result and does not raise; a set call issues
**one** request and returns one spec per requested slot; the module imports with `anthropic`
absent.

**This is where the first real measurement of this milestone arrives** — whether the model's
specifications are actually right. Record what a real lookup of two or three known clubs returns,
and what it refused, in this phase's *As built*.

### [ ] P5 — `POST /api/clubs/lookup`, and a widened `BagEntryRequest`

**Goal.** The lookup is reachable from the page, and the save route can carry a whole
specification.

**Files.** `src/golf_coach/api/app.py`.
**Tests.** `tests/api/test_bag_route.py`; new `tests/api/test_club_lookup_route.py`.

**Detail.** The new route returns candidates and **writes nothing** (ADR-026 §5): catalogue first,
model on a miss. `_resolve_club` (`app.py:179`) parses every club string, including each member of
`slots`. The key is unwrapped at the route and passed in as a plain `str` — `api/app.py` is already
sanctioned for that (`tests/test_config.py:32`) and **that pin must pass unchanged**.

`BagEntryRequest` (`app.py:119`) grows every `ClubSpec` field, keeping its rule that an omitted
field is *undeclared* rather than zero. The club stays out of the body. `recorded_at` stays absent —
the store owns the clock. The save route gains a `catalogue.remember(...)`.

Pin in tests: a lookup with no key configured returns 200 with a noted empty candidate rather than
500; a save carrying a full spec round-trips through `GET /api/golfers/{id}/bag`; a saved club is
served from the catalogue on the next lookup with no client call.

### [ ] P6 — the bag page: search, confirm, and set fan-out

**Goal.** The golfer types "Titleist T150, 7 iron" and a filled-in editable form appears.

**Files.** `src/golf_coach/api/static/career.html`.
**Tests.** None — hand-written HTML with no build step, verified by driving it (§Verification).

**Detail.** `addBlock` (`:784`) gains a make/model/year search box beside the slot picker.
`onBagClick` (`:794`) gains the search and confirm branches. `ENTRY_FIELDS` (`:607`) becomes a
grouped `SPEC_FIELDS` matching §Design 1's groups — **one place per shape**, so a field added to
`ClubSpec` is added here and nowhere else.

**A field the model refused renders visibly blank, never zero-filled.** A zero lie angle is a
number; an empty box is a question, and the difference is the whole of ADR-026 §6 at the surface a
person actually looks at.

Set entry renders one row per slot with a single confirm, then posts **N times to the existing
per-club route**. A partial failure leaves a partially declared bag and says which slots landed.

### [ ] P7 — docstrings, the doc map, and the roadmap

**Goal.** Close the milestone with the docs agreeing with the code.

**Files.** `src/golf_coach/contracts/bag.py` and `club_spec.py` docstrings, `docs/ARCHITECTURE.md`
(§3's walk and the route table), `docs/README.md`, `ROADMAP.md`, this file's status line and *As
built* notes.
**Tests.** `tests/test_docs_truth.py`.

**Detail.** `docs/ARCHITECTURE.md` is tier AS-BUILT, so the new route enters its route table and
`clubs/` enters §2's import map. This doc's tier moves from TARGET to REFERENCE once every phase is
built, and both sides of that — the banner here and the row in `docs/README.md` — are pinned
against each other by `test_the_map_agrees_with_each_doc_about_its_tier`.

Then run `/doc-check`, which starts from the doc-truth failures rather than from a sweep.

---

## Verification

Per phase:

```bash
.venv/Scripts/python.exe -m pytest
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
```

Three that carry specific weight:

- **`tests/api/test_pipeline_imports.py`** — must stay green, proving `clubs/lookup.py` does not
  pull `anthropic` at module scope. It is the fastest signal that the extras boundary survived.
- **`tests/test_config.py::test_only_the_sanctioned_sites_unwrap_a_secret`** — must pass
  **unchanged**. A new unwrap site means the key escaped the route.
- **A second lookup of the same club, saved again, does not move `recorded_at`** (P2). The
  provenance-exclusion defect in §Design 2, and the regression most likely to slip.

End to end, after P6:

1. `.venv/Scripts/python.exe scripts/run_server.py`, open the career page for `aaron`.
2. Search "Titleist T150", 7 iron. Confirm the specs render editable, and **blank where the model
   refused rather than zeroed**.
3. Save. Verify `data/processed/golfers/aaron.bag.json` now exists with the full spec and its
   provenance block.
4. Search the same club again — served from `club_catalogue.json`, no API call.
5. Enter a set ("Titleist T150, 4–PW") and confirm seven entries land with a per-slot loft
   progression that reads like a real set.
6. `.venv/Scripts/python.exe scripts/club_profile.py --player-id aaron --club 7i` and MCP
   `get_club_profile` both still answer — the per-club statistical refusals are unchanged, since no
   swing on disk is tagged yet.

## Risks

- **The model may be confidently wrong about a spec.** This is the milestone's central risk and
  ADR-026 §6 is the whole mitigation: refuse rather than guess, and confirm rather than write. P4's
  *As built* must record what a real lookup returned for clubs whose specs can be checked by hand —
  if the refusal rate is near zero on obscure clubs, the prompt is not working and the confirm step
  is carrying more weight than intended.
- **Discontinued models are the ones that matter and the ones with the least published data.** A
  garage holds clubs from a decade of releases. Expect the long tail to come back mostly blank; that
  is the system working, and it is also the argument against ever trusting a filled field's mere
  presence.
- **`same_club_as` is a single line with a statistical consequence.** See §Design 2 and P2. It fails
  silently — the bag looks right and a caveat appears later over shots that never changed club.
- **Two field lists is the shape this milestone is most likely to regress into.** `ClubSpec`,
  `BagEntryRequest` and `SPEC_FIELDS` are three surfaces over one list, and two of them are
  hand-written. P5 and P6 should each be checked against `ClubSpec` field by field before they are
  called done.
- **`ANALYSIS_VERSION` does not move and must not.** Nothing here changes how a swing is scored, so
  no stored result becomes stale and no corpus re-run is owed. A phase that finds itself wanting a
  bump has strayed out of scope.
