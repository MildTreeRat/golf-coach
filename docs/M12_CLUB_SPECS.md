# M12 — Club Specs: the golfer names a club, the program determines what it is

> **Tier: REFERENCE.** All eight phases are built, so read the reasoning and not the digits —
> every measurement below is dated and the model that produced it can answer differently tomorrow.
> The *why* is [ADR-026](decisions/026-club-specification-lookup.md), written by this milestone's
> P0; the decision it **reverses** is [ADR-024](decisions/024-per-club-shot-history.md) §2, whose
> 2026-08-31 addendum records the reversal. Read both addenda on ADR-024, not just the ADR.

**Status: 8/8 phases built, closed 2026-09-01** — P0 wrote ADR-026, ADR-024's second addendum and
this document, so a phase number is a complete handoff; P1 landed `contracts/club_spec.py`; P2 made
`BagEntry` inherit it, so **the bag carries the whole specification**; P3 landed
`clubs/catalogue.py`, the cache the lookup misses into; P4 landed `clubs/lookup.py`, the model call;
P5 composed the three into `POST /api/clubs/lookup` and taught the save route to remember what a
golfer confirms; P6 built the surface a person uses; and P7 reconciled the docs with all of it.
**The path has been walked end to end and `data/processed/golfers/aaron.bag.json` exists** — seven
irons, declared through the page on 2026-09-01, which is the first bag this repo has ever held. The
measurements this milestone produced are in P4's, P5's and P6's *As built* — read all three in
order, because each one corrects the reading before it.

**One thing is built and unverified**: P6 was driven headlessly, so nobody has yet looked at a
twenty-seven-field form on a phone. Layout, and only layout, is the open item.

## What this milestone is

**The bag was a blank form and it stayed blank for a whole milestone.** When this document was
written, `data/processed/golfers/` held `aaron.golfer.json` and **no `.bag.json` at all**. M9 P2/P3
built `contracts/bag.py`, P19 built the row form in `api/static/career.html`, and the write route
had worked since. Nothing was broken. The form had simply never been filled in, which is what a
form asking for a lie angle gets. P6 is where that changed, and the fact that it took a lookup to
change it is the whole argument below.

**And the five fields it asks for are not what a club is.** `BagEntry` carried `loft_deg`, `make`,
`model`, `shaft` and `length_in` until P2 replaced them with `ClubSpec` (`contracts/bag.py`). A
Titleist T150 7 iron and a TaylorMade
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

- **"Never a catalogue default" is no longer the rule.** `bag.py`'s `loft_deg` description said it
  and ADR-024 §2 argues for it; both are **superseded** by ADR-026 §1, and P2 rewrote the field
  description accordingly. If the manufacturer says the loft is X, the loft is X. This is the one
  place in the milestone where the obvious move — preserve the existing invariant — is the wrong
  one, so anything touching loft must be read against ADR-024's second addendum rather than against
  §2 itself. ADR-010 §2 is *not* weakened: nothing is guessed, and a spec the model will not commit
  to still comes back `None`.
  **The retired sentence survived in six places P2 deliberately left alone, and P5 rewrote them
  all** — `mcp/club.py`, `contracts/caveats.py`, `contracts/club_profile.py`,
  `contracts/tool_descriptions.py`, `scripts/club_profile.py` and `career.html`. They were true
  until P5, because nothing wrote a looked-up loft before the route landed; the route landing is
  the moment the sentence becomes a lie to a coaching model, which is why this was P5's work and
  not P7's. This list originally named four and got two of them wrong — it said
  `analysis/club_profile.py`, which never carried the sentence, and missed all three `contracts/`
  files, two of which an MCP model reads. See P5's *As built*.
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
disk.** (True the day P2 ran, and it is why the shape could change without a reader for the old
one. The first bag landed in P6, on the new shape.)

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

### [x] P1 — `ClubSpec`, the two shaft enums, and their parsers

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

**As built.** Built 2026-08-31 as listed, 50 pins in `tests/contracts/test_club_spec.py`. Four
things worth recording, three of them decisions this plan did not make.

*The blank is two blanks, and the pin above overstates it.* A `ClubSpec` with only a `club` reads
`None` on the measured fields and `""` on the free-text ones, because that is what `BagEntry` does
today (`bag.py:77`) and P2 inherits rather than replaces it. Changing the string fields to `None`
here would have made P2 a migration instead of an inheritance, for a uniformity nothing reads. The
test pins *falsy* and says why.

*A trailing "flex" is dropped in `_normalize` rather than listed as aliases.* "Stiff Flex", "R
Flex" and "Regular Flex" are one rule, not three rows, and the rule keeps working for a member
added later — which is `_build_aliases`' own argument applied one level up. `"flex"` alone still
folds to nothing and returns `None`.

*`_normalize` is copied from `club.py` rather than imported.* Four lines duplicated on purpose:
these are two vocabularies, and sharing the fold would mean a change made for club spellings
silently changing how a flex parses. Recorded here because a reuse review will find it — see
`docs/CODE_STANDARDS.md` on when a second copy is the cheaper failure.

*Two things §Design left open got closed conservatively.* `"composite"` does **not** parse to
`MULTI_MATERIAL` — makers use it for plain graphite too, so it names two members and picks
neither — and `"hybrid"` does not parse at all, because it already names a club here. Both are
pinned as refusals so removing them is a decision rather than a slip. And `loft_range_deg` gained
a `model_validator` refusing a range that runs high to low: every use of a range is a containment
test, and a reversed pair answers `False` for every loft including the one the club is set to.
That is the one field an LLM can plausibly return in the order it happened to say it (P4).

### [x] P2 — `BagEntry(ClubSpec)`, the shaft split, and the provenance exclusion

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

**As built.** Built 2026-08-31 as listed. `BagEntry` is now three fields and a method; the other
two dozen come from `ClubSpec`, `club`'s field description included — the base already carried the
"an entry travels alone, so the container pins its key against this" argument word for word, so
keeping a second copy would have been the R4 duplication this phase exists to remove. `provenance`
is declared **after** `retired_at` so the three names `same_club_as` excludes are contiguous in the
file; someone adding a fourth exclusion should have to walk past them. Five things worth recording.

*The consumer table found a break no checker would have.* `scripts/club_profile.py:294` printed
`entry.shaft`, and `scripts/` is outside `mypy src` while `ruff` does not resolve attributes — so
that line would have raised `AttributeError` the first time anyone ran the CLI against a declared
bag, and no test or gate covers it. It was caught by reading the table in §Reading order rather
than by running anything, which is the argument for the table having been written in P0.

*Two consumers took the shaft split and stopped there.* `BagEntryRequest` (`api/app.py:119`) and
`ENTRY_FIELDS` (`career.html:607`) lost `shaft` and gained the six fields' worth of it a golfer can
actually name — `shaft_model` on the form, all six on the request — and neither grew the rest of
`ClubSpec`. That is P5 and P6's work and doing it here would have hand-written the spec form twice.
The split itself could **not** wait for them: a body field the contract no longer has is worse than
a missing one, because pydantic accepts it, the route drops it on the way to `BagEntry`, and the
page shows a 200 for a value that went nowhere.

*The R6 pin did exactly what it was built for.* `_ODD_VALUES` in `tests/contracts/test_bag.py`
walks `BagEntry.model_fields`, so the shape change failed with a `KeyError` naming each new field
one at a time rather than quietly comparing six of twenty-seven. It grew from six entries to
twenty-seven, grouped as `ClubSpec` declares them so a field added to a group there lands in the
same group here.

*The store owes two tests, not one.* The plan named the provenance no-op; its opposite is
`test_a_re_shafted_club_is_a_different_club`, which is the only thing proving the exclusion did not
over-reach and quietly stop comparing the twenty-one fields that arrived with it. A pin that only
checks a comparison saying "same" passes just as well if it always says "same".

*The two closed vocabularies forced a decision at the route that §Design did not name.*
`BagEntryRequest` types `shaft_material` and `shaft_flex` as **`str`**, not as their enums, because
typing them as the enum makes pydantic answer 422 to `"S"` before `parse_shaft_flex` ever runs —
and `"S"` is the only spelling most golfers have seen their flex written in. That is `ClubRequest`'s
argument for `"7 iron"` reaching `parse_club`, applied unchanged. New `_resolve_shaft_vocabularies`
(`api/app.py`) parses both beside `_resolve_club` and **400s on text it does not recognise** rather
than blanking it, so a typed hedge is a retype and never a silently dropped field; an *omitted*
field stays undeclared, since the M9 row form does not ask for a material at all. Pinned in
`tests/api/test_bag_route.py::test_the_shaft_vocabularies_are_parsed_at_the_boundary_and_refuse`.

*The provenance defect never had a chance to appear.* Both directions were written before the
suite ran and both were green first time, so this phase's headline risk is recorded as **prevented
by having been written down three times**, not as caught. That is what §Design 2, ADR-026 §4 and
ADR-024's addendum were for, and it is worth saying plainly that the evidence for the practice is
an absence.

### [x] P3 — the committed catalogue and its key

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

**As built.** Built 2026-08-31 as listed, 24 pins in `tests/clubs/test_catalogue.py`, and the
catalogue ships with **zero rows** — there is nothing to measure in this phase, which is why the
milestone's first measurement is still P4's. Seven things worth recording, five of them decisions
this plan did not make.

*The key composes `slugify` and then does one thing `slugify` must not.* It drops the separators
`slugify` leaves in, so "T150", "T-150" and "T 150" are one key. That extra step is the whole
difference between the two jobs: a `player_id` is a filename people read, so "mary jane" and
"maryjane" are allowed to be two golfers, while a model key is never read back out and those three
spellings are one club printed three ways on the same manufacturer's pages. Composing rather than
replacing is what keeps the accent fold and the case fold in one place.

*An unknown year is its own key, not a wildcard.* `model_year=None` becomes an empty segment and
matches only other rows that also failed to establish a year. The tempting fallback — no year, so
serve any year — answers a 2023 lookup with a 2019 set's lofts, which is `parse_club`'s nearest-match
refusal in a new place.

*`lookup` returns the row, not the spec.* The provenance has to travel: a spec handed over with it
stripped is exactly the "a row that came from an LLM mistaken for one typed off the manufacturer's
page" that ADR-026 §7 asks this file to prevent. For the same reason the catalogue does **not**
rewrite `source` to `"catalogue"` on the way out — it is where an answer was kept, not where it came
from, and `"catalogue"` stays the right label only for a row typed straight into the JSON by hand.
`SpecProvenance` is therefore **required** on a row where `BagEntry.provenance` is optional: an
entry can predate anything that recorded a source, a row cannot.

*No `lru_cache`, and that is a deliberate departure from `benchmarks/store.py`.* That module caches
its rows for the life of the process because nothing writes `ranges.json`. `remember` writes this
file, so a cache would serve the state from before the last confirmed club — and it would do it in
tests too, where the path is monkeypatched. Tens of rows read once per lookup is a cost too small
to trade a staleness bug for.

*Every refusal here is silent, which is the opposite of `BagStore`'s posture and for a stated
reason.* This is a cache and the bag is the record: a lookup that cannot read the catalogue must
cost an API call and never an error, and a `remember` that cannot write must cost a *future* API
call and never the save that triggered it. So `remember` returns the key it wrote or `None`, and
the three `None`s — no make or model, an unreadable catalogue, a failed write — are each documented
where they are returned.

*The one refusal that is not about tolerance is `BagStore._load_for_write`'s rule, moved here.*
`remember` will not write over a catalogue it cannot parse. A tolerant writer would replace every
checked row with the single row it was asked to add, nothing would raise, and the loss would be
visible only to someone who remembered what used to be in the file. The damaged file is left
exactly as found. The reader is tolerant at **two** levels beside it: a file that will not parse is
an empty catalogue, and a single row that fails validation is skipped while the rest survive — one
hand-edited typo is not a reason to stop serving the forty rows that were checked.

*Two small things the shape forced.* `_catalogue_path` uses `Path(__file__)` rather than
`importlib.resources`, because this is the one packaged data file that is *written* and a
`Traversable` cannot be — naming it two ways would let the read and the write disagree about which
catalogue this is. And rows are dumped with `exclude_defaults=True`, which is safe only because
every default here is the blank; `tests/clubs/test_catalogue.py` pins a blade's `offset_mm=0.0`
surviving the round trip, since the day a real zero is dropped the club reads as unspecified and
gets looked up forever. One further pin exists because nothing else would ever notice: the tolerant
reader means a stray comma in the committed `club_catalogue.json` turns the cache off silently, so
one test reads the real file and asserts every row in it validates.

### [x] P4 — the LLM lookup, single club and set

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

**As built.** Built 2026-08-31 as listed, in `src/golf_coach/clubs/lookup.py`, with 81 pins in
`tests/clubs/test_lookup.py` and one more in `tests/api/test_pipeline_imports.py` — the import
boundary belongs beside the other extras pins, not in a second place. Four things worth recording,
and the first of them is the phase's measurement, which is not the result this milestone assumed.

**The model refuses `loft_deg` on irons, and that is the one field ADR-026 §1 reversed a rule to
get.** Four real lookups against `claude-opus-5`:

| Asked | Filled | Refused | `loft_deg` |
|---|---|---|---|
| Titleist T150 2023, 7 iron | 9 | 15 | **refused** |
| TaylorMade Stealth 2 2023, driver | 8 | 16 | 10.5°, with `loft_range_deg` [8.5, 12.5] |
| Vokey SM10 2024, sand wedge | 11 | 13 | 56.0° |
| T150 2023, 4i–PW in one call | 10 each | 14 each | **refused, all seven** |

The pattern is clean and it makes sense in hindsight: the model states a loft when the number is
*stamped on the club* — a driver's 10.5, a wedge's 56 — and refuses it when the loft lives in a
manufacturer's spec chart, which is every iron. Its own `notes` say so in as many words: *"I recall
the T150 7-iron being roughly 2 degrees stronger than the T100 but cannot confirm the exact
published figure, so it is null rather than guessed."*

So P0's argument — **book loft versus nothing**, and nothing has won for a milestone — is only half
answered. For the driver and the wedge the lookup delivers what ADR-026 promised. For the irons the
choice turns out to be *nothing versus nothing*, and the reversal buys them a form to type into
rather than a number. That is not a failure of the lookup; it is the refusal working exactly as
ADR-026 §6 asked, and a hallucinated iron loft chart is precisely what §6 exists to prevent. It
does mean the milestone's headline claim needs qualifying, and it is an argument for the deferred
hand-populated catalogue: seven iron lofts are one table typed in once, off a page anyone can read.

*What it does fill is worth having.* Lie, length, head type, set composition, shaft model, shaft
material and grip came back for the irons, `adjustable_hosel` plus a real `loft_range_deg` for the
driver, and `usga_conforming` throughout. The seven-slot call kept its progression internally
consistent — lie 60.5° to 63.5° in half-degree steps, length 38.5" to 35.75" — which is the
one-call-per-set argument (§6) doing its job. And the model **declined to interpolate when told
not to**: every slot's loft is blank, where the tempting answer was seven evenly spaced guesses.

*Two answers for one club were not identical.* The single 7 iron call returned `lie_deg` 61.5; the
set call returned 62.0 for the same slot. Neither is flagged, and nothing downstream could tell.
That is the sharpest argument yet for `catalogue.remember` (P3): the first *confirmed* answer is
the answer, and a second lookup is not a second opinion but a second guess. `head_type` and
`set_composition` also came back as sentences rather than labels — free text is free text, and P6's
form should expect a paragraph in those two boxes.

**The schema cost four rejected requests, and the limits are not in any doc.** In order:
`minItems` other than 0 or 1 is refused; at most **16 union-typed** parameters; at most **24
optional** parameters; and then a bare `Schema is too complex.` The last was measured rather than
reasoned about, and it inverts the obvious guess — the same 24 fields **compile in 20 seconds when
every one is `required`, and are rejected after 180 seconds when fifteen are optional.** It is
optionality the grammar compiler pays for, not field count; a flat object failed identically to the
nested one, so nesting is not the cost either.

The shape that fits all four is the one that was wanted anyway: **every field `required`, numbers
and booleans nullable, text refusing with `""`.** A refusal has to be written down, so a model that
dropped the tail of an object is distinguishable from one that declined four lie angles. It sits at
15 unions of a permitted 16 — one field of margin — which is why `tests/clubs/test_lookup.py` pins
the count rather than trusting it: a sixteenth nullable non-string field on `ClubSpec` has to fail
in CI, because a schema is only ever validated by the API and every test here injects a client.

**The wire shape is walked off `ClubSpec`, not written beside it.** `_KINDS` maps each annotation
to one `_FieldKind` carrying both the JSON type and the coercion — one record, not two parallel
tables (R5) — and `_SPEC_FIELDS` walks `ClubSpec.model_fields` to build it (R6). A field added to
the contract is asked for automatically; a field with a *new annotation type* raises at import
rather than being skipped, which is R8's wiring-bug half. The alternative, a hand-written list, is
`mcp/query.py`'s `_METRIC_FIELDS` failure again, and it fails silently: the new field is simply
never requested and looks exactly like a model that never knew it.

Everything else refuses rather than nudges. `_as_float` rejects `bool` before `int` (Python says
`isinstance(True, int)`, so a `true` would land as a 1.0 lie angle), rejects non-finite (Python's
`json` parses the bare `NaN` literal, and a NaN loft fails every band while printing as a number),
and accepts a clean numeric string but not `"~30"`. The two vocabularies go through
`parse_shaft_material`/`parse_shaft_flex`, so `contracts/club_spec.py` stays the only place text
becomes a flex. **Identity never comes from the answer**: `make`, `model` and `model_year` are
echoed from the request, because they are the catalogue key and a model returning "Titleist Golf"
for "Titleist" would file the row under a key the next identical lookup misses — the catalogue
calling the API forever while looking like it was working.

**Three helpers are knowingly duplicated from `feedback/coach.py`.** `_sdk`, `_text_from` and
`_note_for` are copies, because ADR-008 forbids `clubs/` importing `feedback/` and this phase was
told to mirror that module structurally, not to share it. What is duplicated is the *prose* for an
SDK failure, so the cost of the two drifting is a differently-worded sentence rather than a wrong
number. If a third caller ever needs it, the answer is `contracts/caveats.py`'s — lift the text into
`contracts/`, which is where this repo already puts prose two modules need and ADR-008 forbids them
to share any other way — and not a fourth copy.

**Left for P5.** `EFFORT` is `high`, and a seven-slot call at that rung is slow enough to be worth
timing against the route's own budget before the page depends on it; the SDK's default timeout is
ten minutes and a set call has to finish well inside it. `catalogue.remember` is not called from
here — looking up and saving are two acts (ADR-026 §5), and the composition belongs at the route.

### [x] P5 — `POST /api/clubs/lookup`, and a widened `BagEntryRequest`

**Goal.** The lookup is reachable from the page, and the save route can carry a whole
specification.

**Files.** `src/golf_coach/api/app.py`.
**Tests.** `tests/api/test_bag_route.py`; new `tests/api/test_club_lookup_route.py`.

**Detail.** The new route returns candidates and **writes nothing** (ADR-026 §5): catalogue first,
model on a miss. `_resolve_club` (`app.py:179`) parses every club string, including each member of
`slots`. The key is unwrapped at the route and passed in as a plain `str` — `api/app.py` is already
sanctioned for that (`tests/test_config.py:32`) and **that pin must pass unchanged**.

`BagEntryRequest` (`app.py:119`) grows every `ClubSpec` field, keeping its rule that an omitted
field is *undeclared* rather than zero — P2 gave it the six shaft fields and nothing else, so what
is left here is the head, assembly and performance groups.

**This phase also owns the four retired "no catalogue default" sentences** named in §Three things:
`mcp/club.py:228`, `analysis/club_profile.py`, `scripts/club_profile.py:282` and
`career.html:707`. The moment this route can fill a loft from a lookup, each of them is telling a
reader — one of them an MCP client's coaching model — a rule this milestone reversed. The club stays out of the body. `recorded_at` stays absent —
the store owns the clock. The save route gains a `catalogue.remember(...)`.

Pin in tests: a lookup with no key configured returns 200 with a noted empty candidate rather than
500; a save carrying a full spec round-trips through `GET /api/golfers/{id}/bag`; a saved club is
served from the catalogue on the next lookup with no client call.

**Reuse `_resolve_shaft_vocabularies` (`api/app.py`, M12 P2)** for the two closed vocabularies
rather than widening `BagEntryRequest` with enum-typed fields — P2's *As built* records why the
request carries them as `str`, and a second parse site would be a second place for `"S"` to become
a 422.

**As built.** Built 2026-08-31 as listed, in `src/golf_coach/api/app.py`, with 13 pins in the new
`tests/api/test_club_lookup_route.py` and 5 more in `tests/api/test_bag_route.py`. Six things worth
recording, and the first of them **corrects P4's headline**.

**The model does state an iron loft when it is asked for one club — P4's refusal was not the whole
story.** Two real calls through the route, `claude-opus-5`, same club and same year as P4's:

| Call | `loft_deg` | Wall clock | Filled per slot |
|---|---|---|---|
| Single 7 iron | **32.0°** | 31.4 s | 11 |
| 4i–PW in one call | **refused, all seven** | 46.3 s | 10 each |
| Single 7 iron, second time | 32.0°, from the catalogue | **0.0 s** | 11 |

P4 measured a refusal on both and concluded the model will not recite an iron spec chart. What the
second measurement shows is narrower and more useful: **it refuses to produce a loft *progression*
and will hedge a single one.** Its note on the single call says so — *"Loft is my recollection of
the T150 chart (roughly 2 degrees stronger than the 2023 T100 7-iron); worth confirming against
Titleist's published chart"* — where the set call's note is *"I cannot recall the exact published
figure per slot and will not interpolate."* Seven evenly spaced guesses is the tempting failure and
it declined it twice; one hedged number, marked as a recollection, it will give.

So ADR-026 §1's *book loft versus nothing* holds better than P4 recorded, and §6 is doing exactly
its job in both directions. It also **raises the value of the confirm step rather than lowering
it**: 32.0° arrived with a written request to check it against the manufacturer's chart, and the
form is where a person does that.

**Two lookups of one club still disagree, and now that is measured twice.** P4's single call
returned `lie_deg` 61.5 and its set call 62.0; this phase's single call returned 62.0. Nothing
flags it and nothing downstream could. That is the second independent instance of "a second lookup
is not a second opinion but a second guess", and it is the whole argument for `catalogue.remember`
— which now runs, on confirm, at the save route.

**31–46 seconds is the answer to the question P4 left open, and it lands on P6 rather than here.**
The route is `def` and not `async def`, exactly as `ask_about_swing` is: Starlette runs a sync
handler in a threadpool, so a slow lookup costs one worker and never the event loop, the upload
stream or the analysis worker. That settles the *server* budget with room to spare against the
SDK's ten-minute default. It does not settle the *page*: half a minute with no feedback reads as a
hung form, so **P6 must show a pending state and must not block the rest of the bag section on
it.** The cache hit is 0.0 s, which is the shape the second visit has.

**Six sentences were retired, not the four the phase list named.** `analysis/club_profile.py` never
carried one — the four listed were written from memory of the wrong module — and three
`contracts/` files did: `caveats.py`, `club_profile.py` and `tool_descriptions.py`, the last two of
which are read by an MCP coaching model, which is precisely the reader this phase was worried
about. Rewritten: `mcp/club.py`, `contracts/club_profile.py`, `contracts/tool_descriptions.py`,
`contracts/caveats.py`, `scripts/club_profile.py` and `career.html`. The replacement says the two
things that are now true rather than deleting the refusal: a club with **no** entry still refuses,
and a loft that **is** on record is the manufacturer's published one unless the golfer measured it,
so it describes the model and not a club that has since been bent. `"loft not measured"` became
`"loft not recorded"` in the two places that render it, because under ADR-026 §1 a blank no longer
means unmeasured.

**A test committed a row to the shipped catalogue, and the guard is now repo-wide.** The moment the
save route gained `catalogue.remember`, the first run of `tests/api/test_bag_route.py` wrote a
fixture's Titleist T150 — provenanced `"typed"` — into `src/golf_coach/clubs/club_catalogue.json`
and passed while doing it. `tests/clubs/test_catalogue.py`'s autouse fixture had anticipated exactly
this failure for its own module; what changed is that the set of modules able to reach a write is no
longer enumerable by eye, so the redirect moved to `tests/conftest.py` and covers the suite. P6 will
write through the same route and inherits the guard.

**The save route stamps a provenance when the body carries none.** `BagEntryRequest` grew a
`provenance` block typed as the contract model — echoed back verbatim from a lookup, so the model
id *and its confidence notes* land in the bag — and an absent one becomes `source="typed"` off the
route's own clock. Leaving it `None` was the alternative and it is worse: it would make every
hand-filled entry indistinguishable from the pre-M12 ones that genuinely predate anything recording
a source. `_spec_of` strips `recorded_at`, `retired_at` and `provenance` before the catalogue sees
the entry, because `BagEntry` **is** a `ClubSpec` and pydantic would otherwise keep the subclass and
write one golfer's declaration date into a row about a manufactured object.

**Two costs accepted on the record.** The catalogue remembers *this golfer's* club, so a shaft cut
half an inch short is stored as the model's length and served to the next lookup of that model —
the same trade ADR-026 §1 makes for a bent loft, and the thing that would fix it is the deferred
measured-loft field wearing a different hat. And an echoed provenance covers the whole row, so a
golfer who edits three fields of a looked-up spec saves them attributed to the model; per-field
provenance is the fix and it is not worth a nested block per field today.

**`BagEntryRequest`'s field list is hand-written and pinned.** `test_the_request_carries_every_spec_field_and_the_three_it_must_not`
compares it against `ClubSpec.model_fields` on every run, which is what makes a hand-written copy
safe to be: the failure it catches is a field added to the contract, accepted by pydantic, dropped
by the route and answered with a 200 — `mcp/query.py`'s `_METRIC_FIELDS` failure again. Deriving it
with `create_model` was the alternative and buys less than it looks, since three fields differ from
the contract on purpose (`club` is the path segment, the two vocabularies are `str`, `provenance` is
added).

**One thing P7 does not have to redo.** `docs/ARCHITECTURE.md`'s route table gained the new row
here rather than in P7, because `tests/test_docs_truth.py::test_architecture_lists_every_api_route`
goes red the moment the route exists. §2's import map (`api/ → clubs/`) and §3's walk are still P7's.

### [x] P6 — the bag page: search, confirm, and set fan-out

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

**As built.** Built 2026-09-01 in `src/golf_coach/api/static/career.html` as listed, plus one test
file the phase said it would not have. Six things worth recording, and the first of them is what
the milestone was for.

**A bag exists.** `data/processed/golfers/aaron.bag.json` holds seven irons declared through the
page, and `club_catalogue.json` holds the seven rows they taught it. Both files were empty for the
whole of M9 and the whole of M12 until this phase, which is the entire argument of ADR-026 §1
arriving as a file on disk.

**Driven headlessly, and that is a real limitation to record.** The browser this repo's owner uses
was not available to automate, so the six §Verification steps were walked by loading the page into
jsdom against a live `run_server.py` and dispatching real clicks at the page's own script — the
shipped code path, with only `fetch` and `localStorage` supplied from outside. What that does *not*
cover is layout: the CSS grid, the wrap of the search row and whether twenty-seven fields are
legible on a phone are unverified, and someone should look at it on one. Everything below is
measured through the page's own DOM.

| Through the page | Wall clock | Filled per slot |
|---|---|---|
| Single 7i, Titleist T150 2023 | **20.1 s** | 14 of 27 |
| The same club again | **0.3 s**, from the catalogue | 14 of 27 |
| 4i–PW in one call | **49.3 s** | 10 of 27, and the 7i slot served from the catalogue at 14 |

**The third measurement of the refusal, and it agrees with P5 rather than P4.** The single call
answered `loft_deg` 32.0, `lie_deg` 62.0 and `length_in` 37.0, noting *"T150 runs 2 degrees stronger
than T100 through the mid-irons"*. The seven-slot call refused loft, lie, length and flex for
**every** slot, with a separate written reason each time — *"left null rather than derived from the
set progression"* (9i), *"I could not confirm the published … figures for this slot so they are left
blank rather than interpolated"* (4i). It will hedge one number and will not invent a progression,
now measured four times across three phases.

**And the page put both answers on screen at once.** The set panel rendered the remembered 32.0°
7 iron beside six slots blank in the same three fields, each carrying the sentence saying why. That
is ADR-026 §6 made visible: a golfer looking at that panel can see exactly which numbers anyone is
willing to stand behind. No field anywhere rendered a zero for a blank — 13 empty inputs on the
single-club form and 17 on a set row, which is the check §Design 5 asks for.

**One cost found, and it is not the page's to fix.** `ClubLookupOutcome.provenance` is one block per
call by design (`clubs/lookup.py:110`), so a seven-slot call produces **one 932-character note
covering all seven slots and stores it on each of the seven rows** — the pw entry carries sentences
about the 4i, and `club_catalogue.json` reached 10.5 KB for seven rows, six of them the same
paragraph. Nothing here is wrong, and everything here is duplicated. The fix is a per-slot note,
which needs a shape change `ClubSpec` deliberately does not have, so it belongs with the per-*field*
provenance P5's *As built* deferred: they are now two arguments for the same deferred change and
should be done together.

**The regression §Verification named as most likely to slip does not slip.** Opening a saved 7 iron,
pressing save with nothing changed, and reading `recorded_at` back: unmoved. `BagStore.set_entry`'s
`same_club_as` branch survives a form that now posts twenty-seven fields instead of five, which was
the thing to check, because every one of those fields is a new chance to differ.

**Four structural notes.**

*The lookup renders into its own host.* A set lookup takes most of a minute, and `renderBag`
repainting the whole section when it lands would discard an edit form open in a club row beside it.
Only `#lookup` changes while a call is in flight — that is what P5's "must not block the rest of the
bag section" means once written in DOM. Verified: the pending line appears in the same tick as the
click, every control disables, a second click is inert, and the club rows stay on screen.

*A message is set on an element, never rendered by a branch.* M9's five-field form re-rendered to
show `"…" is not a number` and could afford to; a form carrying a whole `ClubSpec` cannot, because
the re-render discards the twenty-six fields typed beside the bad one. `message()` writes into a
`<p>` that is always in the DOM, and `write()` now **returns** the server's sentence instead of
setting `bagError` itself — a fan-out needs all N results before it can say anything.

*One reversed loft range is caught in the page.* It is the only place this form is stricter than the
route: `ClubSpec._loft_range_is_ordered` raises, so a high-to-low pair would be a 500 rather than a
sentence, and a typo in a form is not a server error.

*The set fan-out is sequential, and that is correctness.* `BagStore` reads, mutates and writes the
golfer's whole bag file per call, so seven concurrent posts would each write a file read before the
other six landed.

**Two deviations from the phase text above.** The search box is its own block *under* the slot
picker rather than beside it, for the isolation reason; and a set is entered as a slot **range** —
two pickers over `clubList` — rather than free text like "4-PW", because the taxonomy already knows
its own order and parsing a dash into it would be a second club vocabulary in the page.

**And one test, where the phase said none.** `tests/api/test_career_page.py` (3 pins) parses
`SPEC_FIELDS` out of the HTML with a regex and compares it against `ClubSpec.model_fields`. The
phase's "no tests" reasoning was about *behaviour*, which still needs driving; this is the risk
§Risks names as the most likely regression in the whole milestone — three surfaces over one field
list, two of them hand-written — and it costs a regex rather than a toolchain, because the constant
is a literal list of literal three-tuples precisely so that reading it needs neither. It also pins
that the page holds no second copy of the two closed vocabularies: `shaft_flex` and `shaft_material`
are text inputs the save route parses and refuses by name, not `<select>`s listing the members.

### [x] P7 — docstrings, the doc map, and the roadmap

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

**As built.** Done 2026-09-01. `tests/test_docs_truth.py` was green **before** this phase started
and green after, which is the finding worth leading with: **every stale sentence P7 repaired was
invisible to it.** The pins count checkpoints, phases, routes, addenda and tiers; they cannot see
a docstring claiming a module has no consumers, and that is the failure mode this phase exists for.
`/doc-check` starts from those failures, so on this phase it had none to start from.

**Three docstrings had gone false, and one of them by the code's own hand.**

*`clubs/catalogue.py` said "the file starts empty, and that is the design".* P6 put seven rows in
it, so the module's own data falsified its docstring inside twenty-four hours. It now says the file
grows by use and is not seeded — which is the claim that was actually load-bearing — and records
what P6 measured: the long tail stays *missing* rather than wrong, and a row's `provenance.notes`
may discuss clubs other than its own, because a set lookup is one call for up to fourteen slots.

*`contracts/club_spec.py` said "nothing consumes this yet".* Five surfaces do. Four derive the
field list off `model_fields` (`BagEntry`, the catalogue, the lookup's wire schema, the route's
`BagEntryRequest`); the fifth, `SPEC_FIELDS` in `career.html`, is hand-written because a form
control is a layout decision and not a type, and is pinned against `model_fields` by
`tests/api/test_career_page.py`. Naming which four derive and which one is pinned is the part a
reader cannot get from the code without opening all five.

*`contracts/bag.py` said the same thing, and had said it since M9.* "P3 puts it on disk, P14
composes it, P16 reads `recorded_at`" was a forward-reference that outlived the phases it pointed
at by a whole milestone. A line naming *future* consumers has no expiry check on it and nothing in
this repo was ever going to fail because of it.

**`docs/ARCHITECTURE.md` gained four things, not one.** `clubs/` in §2's import map with an edge
from `api/` only, plus prose recording that it is the second consumer of the `llm` extra and
mirrors `feedback/coach.py` structurally without importing it (ADR-008 forbids the shared copy, so
what is duplicated is prose and drift costs a sentence rather than a number); a `ClubSpec` row in
the interface-contracts table; `club_catalogue.json` in §4's storage table beside the benchmark
aggregates, which is where it belongs since both are committed package data with per-row
provenance; and the bag row corrected, since a `.bag.json` is a whole specification now. The route
table needed nothing — P5 entered `POST /api/clubs/lookup` when it built it, which is
`test_architecture_lists_every_api_route` working as intended.

**The tier flip is the one part both sides check.** This doc moves TARGET → REFERENCE and
`docs/README.md`'s row moves with it, pinned against each other by
`test_the_map_agrees_with_each_doc_about_its_tier`; the phase count moves 7/8 → 8/8 on both sides,
pinned by `test_the_map_and_each_phase_doc_agree_on_the_phase_count`. REFERENCE is the right tier
rather than AS-BUILT because the numbers in P4, P5 and P6 are one model on three dates: the
structure is built and stable, the refusal measurements are not repeatable claims.

**Two premises in this document were rewritten to the past tense**, because a REFERENCE doc that
still opens "the bag is a blank form" is a doc arguing for work that is finished. §What this
milestone is now says it *stayed* blank until P6, and §Design 2's "migration is free — no
`.bag.json` exists on disk" carries the date that made it true.

**One decision, taken today rather than deferred again: the catalogue rows ship.**
`club_catalogue.json` is committed with all seven T150 rows — a real golfer's real clubs, inside
the package, which is what `catalogue.remember` was built to write. Nothing had ever shipped a row
before. The cost accepted with it is P6's duplicated provenance note: six of the seven rows carry
the same 932-character paragraph, and the pw row's note discusses the 4i. It is a note, nothing
reads it as data, and the fix — per-slot provenance — needs a shape change `ClubSpec`
deliberately does not have. It is now recorded in `catalogue.py` itself, so the next reader of
that file meets it where the rows are.

**What P7 did not do.** `/doc-check` is the user's to invoke and was not run from inside this
phase. The layout check P6 left open is still open: nobody has looked at a twenty-seven-field form
on a phone, and no amount of documentation closes that.

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
