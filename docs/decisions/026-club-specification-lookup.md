# ADR-026: Club Specification Lookup — the golfer names a club, the program determines what it is

## Status
**Accepted** 2026-08-31. The phases are [docs/M12_CLUB_SPECS.md](../M12_CLUB_SPECS.md), whose P0
wrote this document. **This ADR is ahead of its code** — P1–P7 are unbuilt, so every claim below is
a design intention and not a measurement. Read it as a plan, and expect the corrections to arrive
here as addenda the way ADR-024's did.

## Date
2026-08-31

## Context

[ADR-024](024-per-club-shot-history.md) §2 settled where a club's properties live: `7i` names a
**slot**, and the physical club occupying it is a `BagEntry` carrying loft, make, model, shaft and
length. That decision is right and nothing here disturbs it. What it did not settle is *how the
fields get filled in*, and the answer it left implied — the golfer types them — has now been tested
by a milestone's worth of elapsed time.

**The bag is a blank form, and it has stayed blank.** `data/processed/golfers/` holds
`aaron.golfer.json` and no `.bag.json` at all. M9 built the whole path — `contracts/bag.py`,
`storage/bag_store.py`, the write route at `api/app.py:693`, the row form at
`api/static/career.html:607` — and it works; it has simply never been used. That is not a defect in
any of those parts. It is what a five-field form asking for numbers nobody has to hand produces.

**And five fields are not what a club is.** A Titleist T150 7 iron and a TaylorMade Stealth 7 iron
differ in loft, lie, length, shaft weight, shaft profile, offset, bounce and head construction, and
every one of those moves launch, spin and start line. `BagEntry.shaft` is a single free-text string
covering what is really six independent facts. A bag filled in perfectly, exactly as the shape
allows, still could not say whether a 7 iron is a 30.5° players iron on a 120 g steel shaft or a
27° game-improvement iron on 60 g graphite — two clubs that produce different ball flights from the
same swing.

The downstream goal is stated in ADR-024's *Deferred, by choice*: a ball-flight model, and club
fitting on top of it. Neither can be built on a slot name. **The inputs those models need are
unrecoverable after the fact and the models are not** — which is the ordering argument ADR-024 used
to record loft before anything read it, applied now to the rest of the specification.

### Why this is lookup and not measurement, and why that differs from the club tag

ADR-024 has a section titled *"Why the club cannot be detected"*, and it is worth being precise
about how little of it applies here. That section is about **which club hit a given shot** — a fact
about an event in this bay, which the HD Golf screen does not print and which M2's club-head
detection is gated behind an exposure [ADR-017](017-club-head-detection-strategy.md) and
[ADR-018](018-bay-lighting.md) say the bay cannot currently deliver. It is unmeasurable here, and a
human tag is the only source that exists.

A club's **specification** is a different kind of fact. It is not an event; it is a published
property of a manufactured object, printed by the manufacturer, reprinted by every retailer, and
stable for the life of the model. Nothing in this bay can measure it either — but it does not need
to be measured. It needs to be *retrieved*, and this repo already has a component that retrieves
published facts on request: the Anthropic client behind `feedback/coach.py`.

So the shape of this milestone is: **the golfer enters what they know — a make, a model and a slot
— and the program determines the rest.** Typing "Titleist T150, 7 iron" is a thing a golfer can
actually do standing in their garage. Typing a lie angle is not.

## Decision

### 1. The manufacturer's number is the loft, and that reverses ADR-024 §2

`BagEntry.loft_deg` is documented today as *"Measured loft. None means unmeasured, and never a
catalogue default"* (`contracts/bag.py:74`). **Reversed. If the manufacturer says the loft is X, the
loft is X.** One `loft_deg` field with one meaning — the published specification — filled by the
lookup and editable in the form.

ADR-024's original reasoning was ADR-010 §2's, correctly applied: a number nobody measured is not a
measurement, and a catalogue default silently standing in for one is a wrong number wearing a right
number's clothes. What changed is the alternative it was compared against. §2 assumed the choice was
*book loft* versus *measured loft*. The choice a golfer with no loft machine actually faces is
**book loft versus nothing**, and nothing has won every time for a milestone. A blank `loft_deg` is
not epistemic caution when the bag it sits in does not exist.

The cost, stated once so it is on the record: **a club bent 2° strong reads its book loft, and
nothing downstream will know.** That is real and it is accepted. Bending is uncommon on a home
golfer's irons, it is knowable to the golfer who had it done, and the field is editable — so the
person who bent it can correct it, which is the same posture the club tag itself takes.

If that ever needs modelling rather than correcting by hand, it is a **second field added then** — a
measured loft beside the published one, with provenance to tell them apart — and not a shape carried
speculatively now. ADR-024 gets an addendum rather than a silent edit, so both the original
reasoning and its reversal stay readable.

### 2. The unit is a slot of a model, not a model

A set has a different loft, lie, length and swing weight in every slot. "Titleist T150" is not a
specification; "Titleist T150 7 iron" is. So the record this milestone introduces, `ClubSpec`, is
keyed by `(make, model, model_year, club)` and describes **one slot of one model**.

This falls out of ADR-024 §2 rather than adding to it. `7i` is a slot in *this golfer's bag*; a
T150 7 iron is a slot in *the manufacturer's set*. The bag entry is the join between them.

Every field on `ClubSpec` is `None` or `""` when unknown. Nothing is defaulted, nothing is
interpolated between slots, and nothing is guessed — which is ADR-010 §2 unchanged. §1 does not
weaken that rule; it changes which value counts as *known*.

### 3. One field list, two shapes — `BagEntry` inherits `ClubSpec`

`class BagEntry(ClubSpec)`, adding only `recorded_at`, `retired_at` and `provenance`. A hand-copied
second list of two dozen fields is a failure this repo has already fixed once — `mcp/query.py`'s
`_METRIC_FIELDS` — and [docs/REFACTOR_LEDGER.md](../REFACTOR_LEDGER.md) records it. Two lists drift
in the direction nobody is looking, and here the drift is a spec field that saves into the bag and
never comes back out.

Free-text `shaft: str` splits into `shaft_model`, `shaft_material`, `shaft_flex`, `shaft_weight_g`,
`shaft_torque_deg` and `shaft_kick_point`. **The migration is free, because no `.bag.json` exists on
disk.** This is the last moment that is true, and it is the whole argument for splitting the field
now rather than adding six more beside it later.

`shaft_material` and `shaft_flex` become `StrEnum`s beside `ClubId`, each with a tolerant `parse_*`
at the boundary in the shape of `parse_club` (`contracts/club.py:185`) — "S", "stiff" and "Stiff
Flex" are one flex, and a miss **refuses rather than nudges**. That is ADR-024 §1's argument
unchanged: a closed vocabulary plus one tolerant parser at the edge, because free text splits one
population into three silently.

### 4. `same_club_as` must not see the provenance

`BagEntry.same_club_as` (`contracts/bag.py:103`) compares **by exclusion** — every field that is not
a timestamp counts toward club identity — and its docstring says why: a hand-listed tuple's failure
mode is a new field that silently never counts as a change. That is right for every spec field this
ADR adds and **wrong for `provenance`**, which carries a retrieval timestamp.

Left alone, looking the same club up a second time and saving it mints a new timestamp, reads as a
*different physical club*, retires the good entry onto `Bag.retired` and hands M9 P16 a false
bag-changed caveat over shots all hit with the same club. ADR-024's addendum already names that
exact false positive — "produced by the UI working correctly, which is the worst way to get one" —
and this is a second route to it. So `provenance` joins the excluded set: it is a **third
timestamp**, not a descriptive field.

### 5. Looking up and saving are two acts

`POST /api/clubs/lookup` returns candidate specifications and **writes nothing**. Saving is the
existing per-club bag route. The separation is the point: what comes back is a proposal from a
model, and the golfer confirming it is what turns it into a declaration. A lookup that wrote
straight into the bag would make a hallucinated lie angle indistinguishable from a typed one.

A set is **N calls to the existing per-club write route**, not a bulk route. One writer;
`Bag._keys_match_entries` still checks per club; and a partial failure leaves a partially declared
bag, which is the honest outcome. It is also ADR-024 §5's stance on backfill applied in the other
direction — this repo does not have a bulk club writer and is not gaining one.

### 6. A lookup that refuses is worth more than one that guesses

The prompt instructs the model to **return null for any specification it is not confident of**, and
the parse keeps the null. A hallucinated lie angle is worse than a blank, because a blank is visible
in the form and a wrong number is not — it looks exactly like a right one, and it is the input to
every model this milestone exists to enable. This is ADR-010 §2 again, at a new boundary: no spec
beats a wrong one.

A whole set is looked up in **one call** rather than one call per slot, because the per-slot loft
progression has to be internally consistent — 4 through PW is a sequence, and seven independent
answers are seven chances to break it.

`clubs/lookup.py` follows `feedback/coach.py` exactly, because that pattern is proven and already
pinned: `anthropic` is lazy-imported inside a function (`coach.py:311`) so the module imports clean
on a base install; a `client=` seam (`coach.py:341`) lets tests assert request and parse shape with
no network and no `llm` extra; and an expected failure — missing extra, missing key, API error —
**returns a result carrying a note rather than raising**, the way `CoachingOutcome` does.

No new extra and no new configuration. It reuses the `llm` extra, `settings.anthropic_api_key` and
`settings.coaching_model`. `api/app.py` is already in `tests/test_config.py`'s
`SANCTIONED_UNWRAP_SITES`, so the key is unwrapped at the route and passed in as a plain `str`, and
[ADR-019](019-secret-handling.md)'s surface does not widen.

### 7. The catalogue is committed data, and the second lookup is offline

An accepted specification is written into `clubs/club_catalogue.json`, which ships inside the
package. Lookup reads the catalogue first and calls the model only on a miss.

This is [ADR-022](022-learned-artifacts-as-committed-data.md)'s pattern rather than a new one:
`analysis/benchmarks/ranges.json` and `golfdb_v1.json` are both artifacts produced by something
heavier than the package and evaluated by stdlib arithmetic inside it, and every row carries its own
provenance. A catalogue row is the same thing — produced by a model call, consumed by a dictionary
lookup, provenanced per row. The provenance says which, so a row that came from an LLM is never
mistaken for a row someone typed off the manufacturer's own page.

The key is built with **`slugify` from `contracts/golfer.py:54`**, not a second normaliser. That
function exists because "Aaron" and "aaron" splitting one golfer's baseline in two is silent and
undetectable; "T150" and "T-150" splitting one club into two catalogue rows is the same failure with
the same invisibility, and ADR-024 §1 already made this argument once.

## Consequences

- **`BagEntry` grows from five descriptive fields to roughly two dozen**, and `same_club_as`
  compares all of them from the day they are added — which is exactly what its exclusion-based
  design was for. A re-shafted club now reads as a different club, correctly, and retires the old
  entry onto the shelf ADR-024's addendum built.
- **`BagEntryRequest` (`api/app.py:119`) widens to match.** The route shape does not change: the
  club stays on the path and out of the body, the store keeps the clock, and the response is still
  the whole bag.
- **A new subsystem, `src/golf_coach/clubs/`**, importing `contracts` only. It does not import
  `analysis`, `storage` or `api` ([ADR-008](008-project-structure.md)). `api/app.py` is what
  composes it with `BagStore`.
- **`tests/api/test_pipeline_imports.py` gains a third module to care about.** `clubs/lookup.py`
  must not pull `anthropic` at module scope, for the same reason `feedback/coach.py` must not: the
  offline CLIs run on installs without the `llm` extra.
- **The bag page grows a search step before its row form**, and the row form becomes a grouped spec
  form. `ENTRY_FIELDS` (`api/static/career.html:607`) becomes one grouped list — one place per
  shape, not a second hand-maintained list beside `ClubSpec`.
- **A blank field means the model refused**, and the form must render it visibly blank rather than
  zero-filled. A zero lie angle is a number; an empty box is a question.
- **Nothing existing goes dark.** No swing on disk is tagged with a club yet, so
  `analysis/club_profile.py`, `mcp/club.py` and `scripts/club_profile.py` keep refusing at the same
  floors for the same reason, before this milestone and after it.

## Deferred, by choice

- **Ball trajectory, swing efficiency, gapping and club fitting.** All four are what the
  specification is *for*, and all four stay deferred exactly where ADR-024 left them. The flight
  model additionally has a named blocker: `spin_axis` has no direction word on the HD Golf screen,
  and [ADR-014's addendum](014-screen-capture-shot-ingestion.md) records two fades stored as draws.
- **Measured loft as a field of its own** (§1). It arrives the day someone is bending clubs and
  wants the difference modelled rather than corrected.
- **Bag entry versioning**, still — attributing a stored shot to the stint that hit it. ADR-024
  defers it, its addendum re-defers it, and nothing here moves toward it. Provenance describes where
  a *specification* came from, not which stint hit a shot.
- **Scraping manufacturer sites, and any scheduled catalogue refresh.** The catalogue grows by use.
- **Shipping a pre-populated catalogue.** `club_catalogue.json` starts empty or nearly so. A
  catalogue is worth committing once it has been checked, and nothing has been checked yet.

## Alternatives Considered

**Keep typing the fields.** The status quo, and it has the best possible epistemic story: every
number came from the person who owns the club. Rejected on evidence — the bag has been buildable for
a milestone and no `.bag.json` exists. A form that is never filled in records nothing, and recording
nothing is not more honest than recording a published specification labelled as one.

**Buy or vendor a club specification database.** The most reliable source, and a real option if one
existed at the grain this needs. Rejected for now: specifications are per model *per year* and per
slot, no free dataset covers the long tail of what is actually in a garage, and this repo's posture
on a dependency it cannot verify is to defer it. The catalogue is the seam — if a dataset arrives,
it loads into `club_catalogue.json` and `clubs/lookup.py` becomes the fallback rather than the
source.

**Scrape the manufacturer's page.** More authoritative than a model, and it fails on exactly the
clubs that matter — discontinued models, whose pages are gone. It is also a per-manufacturer parser
to maintain forever, which is a worse maintenance shape than a prompt.

**Ask the model per slot rather than per set.** Simpler prompt, simpler parse. Rejected: the loft
progression across a set is the thing most likely to come back wrong, and it can only be checked
when the slots are answered together (§6).

**Add lie, bounce and the rest beside the existing `shaft: str`.** No migration, no inheritance, no
new module. Rejected: it keeps six facts inside one string permanently, and the moment a `.bag.json`
exists the split stops being free. Doing it now costs nothing and doing it later costs a migration
(§3).

**Write the looked-up specification straight into the bag.** One step instead of two, and the bag
would fill itself. Rejected — §5. The confirm step is what distinguishes a model's proposal from a
golfer's declaration, and without it the provenance field would be the only thing that knew.

## Addendum (2026-10-08, M36): the catalogue is a two-language file, read by `include_str!`

M36 ported the bag, the club and specification shapes and §7's catalogue reader to Rust
([the M36 plan](../plans/m36-many-shot-layer.md), phases P5 and P8). §7 assumed one language, and
there are now two. Python still writes the catalogue and Rust now reads it. This addendum records what
that changes. §1–§7 stand as decided.

**Who writes and who reads.** `clubs/lookup.py` is an LLM call, so it stays Python
([ADR-035 §1](035-rust-everywhere-python-where-required.md#1-the-rule-and-the-two-exceptions-it-names)),
and frozen Python's `remember` goes on writing `src/golf_coach/clubs/club_catalogue.json` when a
golfer confirms a club. Rust reads that file in two places:

- **`crates/contracts::catalogue`** reads the committed rows by `include_str!`, in file order. There
  is one copy on disk, as the benchmark data crosses ([ADR-032](032-the-rust-core.md) §5).
- **`crates/storage::catalogue`** keys them through `slugify` and answers `lookup`. A repeated key's
  last row wins, in its first row's place, as Python's dict assignment does. The key sits in a second
  crate for a dependency reason, not a design one: `slugify` needs `unicode-normalization`, and
  `contracts` takes no third-party dependency beyond `serde` and `serde_json`.

**A row Python remembers reaches Rust at the next build**, not at the next lookup. Nothing in Rust
confirms a club yet, so no Rust caller is waiting on a fresher row.

**The direction of the pin is reversed.** ADR-035 clause 4 holds frozen Python to reading what Rust
writes. Here Rust reads what Python writes, so `crates/contracts/tests/catalogue.rs` pins that
direction:

- every committed row parses, and writes back under `exclude_defaults` as the row on disk;
- every value `contracts/club_spec.py` can write reads back: each `ShaftMaterial`, each `ShaftFlex`,
  a row with every optional key absent, and one with every key present.

**§3 and §4 survive the crossing.** `BagEntry` is a `ClubSpec` flattened beside its three declaration
fields, so the file stays one flat object and there is still one field list. `same_club_as` is
equality on the spec. Its implementation destructures the entry, so a new field beside the spec does
not compile until someone says whether it is part of a club's identity.

**Two places where Rust refuses what Python takes, both named:**

- **`str.isalnum()`.** §3's tolerant parsers keep a name's `isalnum` characters. Rust's
  `char::is_alphanumeric` is a strict superset of CPython's set, so a name carrying a combining mark,
  such as `7i` with U+0345, normalises to `7i` in Python and to nothing Rust recognises. Every alias
  is ASCII, so Rust can only refuse a name Python takes, never accept one Python refuses. That is
  §3's "refuses rather than nudges", applied to the port.
- **pydantic's lax coercions.** pydantic reads `"2023"` or `2023.0` as a `model_year`, `"9.5"` as a
  loft and `1` as a bool. Rust refuses each. Nothing writes those values, so a row holding one can
  only have been hand-edited.

**Not changed**: §1–§7, and the catalogue's place in `src/golf_coach/clubs/`. The file stays where
`clubs/lookup.py` writes it, and moves only with that writer, whose caller ADR-035 leaves to M40.
