# M9 — Player Tracking: per-club shot history

> **Tier: TARGET.** This is the agreed plan for M9, written before any code. Nothing in it is
> built yet, so verify every claim about the codebase against the code. The *why* behind it is
> [ADR-024](decisions/024-per-club-shot-history.md); this document is the *how*, as a phase list.

**Status: not started, 0/20 phases.** Start at P1.

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

### [ ] P1 — `contracts/club.py`: the club vocabulary

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

### [ ] P2 — `contracts/bag.py`: what a club in the bag is

**Goal.** The shape of a bag entry, carrying the loft fitting will need. Pure contract.

**Files.** `src/golf_coach/contracts/bag.py`, `tests/contracts/test_bag.py`.

**Detail.**
- `BagEntry`: `club: ClubId`, `loft_deg: float | None`, `make: str`, `model: str`, `shaft: str`,
  `length_in: float | None`, `recorded_at: datetime`.
- `Bag`: `player_id: str`, `entries: dict[ClubId, BagEntry]`, `updated_at: datetime`, plus a
  `club_ids` property returning canonical bag order (derived from `ClubId` declaration order, not
  sorted alphabetically).

**Comment to write.** Why `loft_deg` is optional — a golfer who has not measured their lofts still
has a bag; the work that needs loft refuses per club, the same posture as `SwingResult.unscored`.
Why `recorded_at` exists — a bag entry that changes makes shots before and after it two
populations, and P16 turns that into a caveat rather than pooling them silently.

**Tests.** JSON round-trip. `club_ids` returns canonical order, not insertion or alphabetical.

**Done when.** Tests pass; `mypy src` clean.

---

### [ ] P3 — `storage/bag_store.py`: the bag on disk

**Goal.** Read/write one `bag.json` per golfer.

**Files.** `src/golf_coach/storage/bag_store.py`, `tests/storage/test_bag_store.py`.

**Reuse.** Mirror `storage/golfer_store.py` exactly — same class shape, same tolerant reads. Same
atomic write (tmp + `os.replace`) as `storage/manifest.py:save_manifest`.

**Detail.** `BagStore`: `get(player_id) -> Bag | None`, `save(bag)`, `set_entry(player_id, entry)`
(upsert one club, stamping `recorded_at` and `Bag.updated_at`), `remove_entry(player_id, club)`.
Path `<root>/<player_id>.json`; `player_id` is already slug-validated by
`contracts/golfer.py:PLAYER_ID`, so no second sanitising step. A missing or corrupt bag is `None`,
never an exception (R8).

**Tests.** Round-trip; unknown player → `None`; corrupt file → `None`; `set_entry` twice on one
club replaces rather than duplicating.

**Done when.** Tests pass.

---

### [ ] P4 — `club` reaches the manifest and the session cursor

**Goal.** The storage layer can record which club hit a swing. No API, no UI yet.

**Files.** `storage/manifest.py`, `storage/session_meta.py`, and their mirrored tests.

**Reuse.** `SwingManifest.player_id` is the precedent — read its docstring and copy the posture.

**Detail.**
- `SwingManifest.club: ClubId | None = None`, with a description saying it is stamped from the
  session cursor at swing creation, that `None` means it predates the field, and that it is
  **write-once in practice** so switching the cursor mid-session cannot rewrite earlier swings.
- **Optional in the shape, required at the boundary** (R12). The manifests already on disk have no
  club and must still load; requiredness is enforced in P6, in the route.
- `SessionMeta.club: ClubId | None = None`, plus `set_current_club(session_dir, club)` mirroring
  `set_current_player`.
- **Careful:** `set_current_player` currently *replaces* the whole `SessionMeta`. With two cursors
  both setters must load-modify-save, or setting the club clears the golfer. Fix
  `set_current_player` in this phase.

**Tests.** A legacy manifest JSON (pin it as a literal string, not a constructed model) loads with
`club is None`. Setting the club preserves the player and vice versa — the regression the
load-modify-save change exists to prevent.

**Done when.** Tests pass; the existing `tests/storage/` suite is green.

---

### [ ] P5 — `bundle_store` stamps and repairs the club

**Goal.** Thread the club through swing creation, plus the per-swing repair path.

**Files.** `storage/bundle_store.py`, `tests/storage/test_bundle_store.py`.

**Reuse.** `assign_from_path`'s `player_id` parameter and `set_player` are the two shapes to copy.

**Detail.** `assign_from_path(..., club=None)` threaded into `_new_manifest` and `_place` exactly
as `player_id` is; write-once. `set_club(session_id, swing_id, club)` as the explicit human-driven
override. **No `attribute_unlabeled` analogue** — write the comment saying why, pointing at that
method's docstring.

**Tests.** Two uploads into one swing with different clubs → first wins. `set_club` overwrites;
on a missing swing returns `None`.

**Done when.** Tests pass.

---

### [ ] P6 — the upload endpoint requires a club

**Goal.** No shot can be ingested without a club.

**Files.** `api/app.py`, `tests/api/test_uploads.py`.

**Reuse.** The golfer cursor routes are the template; `_safe()` for path segments.

**Detail.**
- `GET` / `POST /api/sessions/current/club` — the cursor. `POST` parses via `parse_club`, 400 on
  unparseable. **No backfill call.**
- `POST /api/uploads` — read the club cursor; if `None`, **409 before streaming the body to
  disk**, with a message naming the fix. Pass the club to `assign_from_path`; add `club` to the
  response.
- `POST /api/sessions/{session_id}/swings/{swing_id}/club` — the repair route.

**Comment to write.** Extend the existing note above the cursor read: the club is read from the
cursor for the same two-phone reason as the golfer, and it is *required* where the golfer is not,
because a mistagged club silently pools a wedge into a 7 iron's carry average whereas an untagged
golfer is repairable later.

**Tests.** No cursor → 409 **and nothing written to `incoming_dir`**. With a cursor → 200 and the
manifest carries the club. Setting the club does not clear the golfer (route-level pin of P4).

**Done when.** `pytest tests/api/` green.

---

### [ ] P7 — the club picker on the upload page

**Goal.** A phone in a bay can pick a club in one tap.

**Files.** `api/static/index.html`.

**Reuse.** The existing golfer-picker block — same fetch pattern, same styling.

**Detail.** A club selector posting to the cursor route, showing the current club prominently (it
changes every few shots, unlike the golfer). Order by canonical bag order; if the current golfer
has a bag, list theirs first and the full taxonomy under it. Disable the upload control while no
club is selected, and surface the 409 if one slips through.

**Done when.** Manual check via `python scripts/run_server.py`: pick a club, upload, and the
manifest names it.

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
