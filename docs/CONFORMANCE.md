# Conformance: the Python core as a specification

> **Tier: AS-BUILT.** The schemas, vectors and runner described here are in the repo and run.
> **So does the Rust core they exist to check.** As of M22 P8b all 21 engine vectors conform
> through `cargo test`, and `golf-core run` diffs against `conformance.py run` at zero differences
> on every one of them. This document stays written for a port that has to be *judged* rather than
> in the past tense, because the Python it judges against is still here until M40 deletes it —
> frozen since M32, when the oracle moved to Rust
> ([ADR-035 §3](decisions/035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust))
> — and because the next port to be judged is the one that reads this file after that. **What
> moved at M32** is §4: `golf-core rerecord` records the engine and stage families under a
> structural gate, `cargo test` certifies them, and `conformance.py check` certifies the freeze.
> **What M34 added** is a second family recorded that way, the screen reader's (§2). Frozen Python
> recorded it once, `crates/screen` ported it, and Rust re-records it under a version key of its
> own. It brought the parser's CPython edges (§3's second list). **What M36 added** is two more,
> the many-shot layer's `storage` and `career` families (§2), recorded and ported the same way and
> re-recorded under a third version key, `CAREER_VERSION`, after the one change M36 made in Rust
> alone: the corpus's `OUTDATED` now means *not comparable*. Its edges are §3's third list.

[ADR-030](decisions/030-app-platform-rust-core-python-sidecar.md) commits this project to a second
implementation of the swing loop. **Two cores that disagree silently is the failure mode that
decision has to survive**, and review does not catch a drift of a fraction of a unit — ADR-027's
own corrections included a 0.29-yard identity that held in one case and broke in another, which is
exactly the size of thing a reviewer signs off on.

Before this milestone the only oracle was the Python test suite, and a Rust port cannot run it. So
the suite is not the specification. **This is:**

| Artifact | Where | What it fixes |
|---|---|---|
| JSON Schemas | `spec/schemas/*.schema.json` | The shapes that cross the seam |
| Golden vectors | `spec/vectors/**` | Inputs paired with the output this engine produces |
| The rules below | §3 of this document | Which fields must match exactly, and which to an epsilon |

A port conforms when it reads a vector's `input`, produces a result, and the comparison in §3 finds
no differences against that vector's `expected`. It does not have to be written in Python and it
does not have to call anything in this repo.

---

## 1. The schemas

Exported by `pydantic`'s `model_json_schema`, one file per root, `$defs` inlined — seven of the ten
still are, and three have been **Rust's, edited by hand, since M32** (below). The roots are
`scripts/conformance.py::SCHEMA_ROOTS`, and the rule is:

> **A schema exists for every JSON artifact a non-Python implementation opens off disk.**

Not "every model in `contracts/`" — a schema is a promise to keep a shape stable, and promising
that for shapes only this package reads would freeze parts of the contract that still move. But
the looser form of the rule ("something other than Python parses it") is what the first pass at
this list applied, and it got four wrong: `SwingManifest` was dropped as internal, along with
`SessionMeta`, `Golfer` and `Bag`. ADR-030 §1 gives Rust **storage**, so a Rust core opens
`manifest.json` on the way to every swing, and the golfer registry and the bag are the two files
behind the `handedness` and `loft_deg` arguments `analysis` is forbidden to fetch for itself. A
port would have had to reverse-engineer all four from example files.

The ten roots cover a swing directory end to end — `manifest.json`, `{role}.keypoints.json`,
`{role}.audio.json`, `analysis.json`, `analysis.state.json` — plus `session.json` a level up and
the `.golfer.json`, `.bag.json` and `.shot.json` a swing resolves through.

`tests/test_conformance.py` holds two pins. One regenerates the schemas in memory and compares, so
a field added to a Python-owned shape without a re-export fails at the commit rather than at the
port; it skips the three Rust-owned roots, which have a pin of their own. The
other **scrapes every `*.json` filename constant out of `src/golf_coach/`** and requires each to be
either mapped to a schema root or named as package data — the committed, provenanced JSON that
ships inside the wheel and ports as bytes (ADR-022: `ranges.json`, `golfdb_v1.json`,
`joint_model_v1.json`, `flight_model_v1.json`, `club_catalogue.json`, and the frozen screen
parser's `profiles.json`, which `crates/screen` forks rather than reads until M40 deletes this copy,
as §2's screen family says). Discovery rather than a listing, so a *new* artifact fails here instead of
being forgotten the way these four were.

**The serialization is the shell's, not the contract's**, and this is the detail most likely to be
implemented faithfully and wrongly. `api/pipeline.py` writes `analysis.json` as
`model_dump_json(exclude={"swing": {"keypoints", "detections"}})` — the exclusion lives at the call
site, so it is in no schema. A port that serialized `SwingBundleResult` as the schema describes it
would emit the whole keypoint list and differ on a field nobody meant to compare.
`conformance.EXCLUDED_FROM_RESULT` is the one copy a port is asked to match, and
`test_the_spec_serializes_a_result_exactly_as_the_pipeline_stores_one` reads the literal back out
of `pipeline.py` so the two cannot drift.

### Three roots are Rust's, and edited by hand (M32)

**`shot_data`, `swing_result` and `swing_bundle_result`** are the roots whose shape M32 moved:
`ShotData` gained seven keys and `ShotProvenance` three, in `crates/contracts` alone. Frozen Python
never gains them ([ADR-035 §4](decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)),
so its exporter would write them back out of all three files. So those three are listed in
`conformance.py::RUST_OWNED_SCHEMAS`: `regenerate --schemas-only` writes the other seven and never
these, the freshness pin skips them, and `SCHEMA_ROOTS` keeps all ten, so
`test_every_on_disk_artifact_has_a_schema` still maps every artifact to a root.

**Whoever changes one of those shapes in `crates/contracts` edits the three files, in the same
change.** Spell a new property the way pydantic spells its neighbours (`anyOf [{type}, {type:
null}]`, `default`, a Title Case `title`, the units in `description`), and the reliable way to do
that is to generate it: declare the field on a throwaway pydantic model, take
`model_json_schema()["properties"]`, and write the file in `export_schemas`' format (indent 2,
sorted keys, ASCII escapes, a trailing newline, and the CRLF the files carry under `spec/** -text`).
That is how M32 P9 made its edit, after checking that format re-serialized all three committed
files byte for byte. Generating them from the structs with `schemars` was declined in
[§M32](plans/m31-m40-shot-first-pivot.md#schemas-split-ownership-with-the-rust-half-hand-maintained):
the files carry pydantic's bound keywords, Rust holds those bounds in its `Validate` impls where a
derive cannot see them, and restating each as an attribute is a second copy that drifts. M36
weighed it again and kept the answer: it moved no shape, so `swing_manifest`, `golfer`, `bag` and
`analysis_state` stay Python-exported, and Rust pins its structs' key sets, required sets and
nullability against those committed files instead (`crates/contracts/tests/python_schemas.rs`,
`crates/storage/tests/python_schemas.rs`). A root becomes Rust's when Rust first moves its shape.

**The pin is `crates/contracts/tests/schemas.rs`**, four tests over the three files:

- the property sets of `ShotData` and `ShotProvenance` equal the keys the Rust structs serialize,
  both ways, so a key added to either struct without a schema edit fails `cargo test`;
- the three copies of each shape are identical (the root's `$defs` set aside), because a hand edit
  is made three times and the key-set test cannot see a description fixed in one file only;
- the swing files' `UnscoredReason` enum is Rust's wire names **minus `printed_blank` and
  `misread`**, compared by name. Those two are photo-side and never reach a `SwingResult`, so the
  schema leaving them out is the contract, and the pin makes that a checked choice;
- every key M32 added carries a `description`, with the keys read from
  [`spec/declarations/v17.json`](../spec/declarations/v17.json)'s `added` list rather than listed
  again.

**Two lists name the Rust-owned roots and nothing checks one against the other**: `RUST_OWNED` in
`schemas.rs` and `RUST_OWNED_SCHEMAS` in `conformance.py`. A root added to one only is unpinned or
pinned twice, so when a later milestone takes more roots over, both change together.

`crates/contracts/devices.json` (M32, the device capability model) and `crates/screen/profiles.json`
(M34, the forked device profiles) are committed data that a Rust crate reads by `include_str!`. They
are not artifacts opened off disk, so neither is a schema root. Neither is in the package-data pin
either, because that pin scrapes `src/golf_coach/` and is Python's. Each has a Rust pin instead:
`crates/screen/tests/profile_fork.rs` holds the fork equal to the frozen copy except one declared
tile, and `crates/screen/tests/capability.rs` holds `devices.json`'s `hd_golf` fields equal to the
fork's targets.

`club_catalogue.json` is the other way round (M36). It stays package data, in the package-data pin,
because `clubs/lookup.py` writes it and stays Python. `crates/contracts::catalogue` reads that one
copy by `include_str!`, so a row Python remembers reaches Rust at the next build.
`crates/contracts/tests/catalogue.rs` pins the direction that matters: every committed row parses,
and every value `contracts/club_spec.py` can write reads back
([ADR-026](decisions/026-club-specification-lookup.md)'s M36 addendum).

## 2. The vectors

Eight families, covering different things. The first two are the **engine** families: `cargo test`
certifies them end to end against `crates/core`, `golf-core rerecord` records them from M32, and
`conformance.py check` runs frozen Python against them to certify the freeze (§4). The other six
name a Rust crate as their implementation and are deferred to `cargo test`. `audio`, `stages` and
`format` serve the engine port and its edges. **`screen`** (M34) is the screen reader's, a port of
its own that `golf-core rerecord` also records, under its own version key. **`storage`** and
**`career`** (M36) are the many-shot layer's: the stores and the corpus reader, then the aggregates
and the five career reports over a corpus. The verb records the two together, under a third key.

**Synthetic** (`spec/vectors/synthetic/`) comes from `tests/analysis/conftest.py::make_swing`,
which is deterministic, RNG-free and pure stdlib — so it re-implements in another language exactly
and a port can generate its own inputs rather than trusting ours. Small, uncompressed, readable.
These cover the **code paths**: a window that has to be un-applied, a checkpoint that fails, a
checkpoint that cannot be scored at all, a bundle with no second view. Each vector's
`provenance.note` says which path it is there for; a case that only re-runs a path another case
covers is a vector that costs a re-record and buys nothing.

**Corpus** (`spec/vectors/corpus/`) is the fifteen real swings on disk, gzipped. These cover what
synthetic input never can: real MediaPipe landmark noise, dropped-visibility frames, two genuinely
unsynchronised cameras, and a launch monitor.

### Who recorded each value: `provenance.oracle` and the `rerecords` ledger (M32)

**Every engine and stage vector says who recorded it**, since M32's first Rust re-record wrote the
two keys onto all forty-two (21 engine, 21 stage; counted 2026-10-01). **So does every screen
vector, and from the day it was recorded**, because M34 wrote `oracle` when it recorded the family.
It is `"python"` on everything frozen Python recorded and `"hand"` on `hand/`. The three document
sub-families have carried a Rust ledger since M34's re-record. `units/` has none, because the
re-record never reads it. **So does every storage and career vector** (M36): `"python"` on what
frozen Python recorded, each with a `career-v1` ledger since M36 P14, and `"hand"` on
`storage/hand/`, which was written at version 1 and has none. `audio` and `format` carry neither key yet. M37's pin, that every family
names its oracle, extends `oracle` to them
([ADR-032](decisions/032-the-rust-core.md)'s 2026-09-29 addendum).

- **`provenance.oracle` is `"python"`**, and stays so after a Rust re-record, because every value
  the re-record did not declare is still the one Python recorded, bit for bit. M32 P8 checked that
  in a second language across all forty-two files, comparing every float by `float.hex()` once the
  declared paths were taken out: zero differences.
- **`provenance.rerecords` is a list with one entry appended per re-record**, and it is what says
  which values are Rust's. An entry holds the declaration's version key and value (the version the
  re-record moved the file to: `analysis_version` on an engine or stage vector,
  `screen_parser_version` on a screen one, `career_version` on a storage or career one), `by`
  (`"golf-core rerecord"`), `declaration` (the
  declaration's repo-relative path, with `/`), and `added` and `moved`: the declared paths **that
  matched in that file**, in the declaration's order. A screen entry carries a third list,
  `removed`, empty or not, so the family's entries share one shape; an engine entry never does (§4
  says why). So M32's entry on a synthetic engine vector moves `analysis_version` and
  `expected.analysis_version` and adds nothing (synthetic vectors carry no shot); a corpus one adds
  the ten new shot keys as well; a stage vector moves `analysis_version` alone, because no stage
  holds a copy of the shot.
- **A ledger path is rooted at the vector document**, with no leading dot: `analysis_version`,
  `expected.swing.shot.attack_angle`, and `[i]` for a list index. Both languages *parse* it
  (`golf_core::rerecord::LedgerPath::parse`, `conformance.parse_ledger_path`) with one grammar,
  `key(.key|[n])*`, and refuse anything else, a wildcard included, rather than match it as a
  string. That is not the spelling either comparator prints — Python's `Difference.path` starts with
  `.` — which is why neither is matched as text.
- **Inputs are not rewritten.** `input.shot` keeps the shape frozen Python writes, because the lab
  goes on writing that shape until M40 and Rust has to read it. So the round trip
  (`crates/contracts/tests/round_trip.rs`) carries one allowance: an `input.shot` key that went in
  absent may come out at its default only where its twin under `expected.swing.shot` is an `added`
  path in that vector's own ledger, holding that value.

**The declarations are committed, in `spec/declarations/`**, one file per re-record, named for the
version it records (`v17.json` is M32's, `screen-v1.json` M34's, `career-v1.json` M36's). Each
carries **exactly one version key**, and that key picks the family it re-records: `analysis_version`
for the engine and stage families, `screen_parser_version` for the screen family, or
`career_version` for the storage and career families together. Each also carries a `note`, `added`
and `moved`, and a screen declaration may carry `removed` as well (§4). A declaration loads or is
refused whole, before any vector is read. Refused are: an unknown key; more than one version key, or
none; a path declared twice across the lists; an `added` or `removed` path ending in an index; and
`removed` in an engine or career declaration. They sit outside
`spec/vectors/` on purpose, because `conformance.py::vector_paths` would read a file there as a
vector. The declaration is what a reviewer reads, beside the verb's report (§4), and it is
load-bearing twice: the verb reads it, and `crates/contracts/tests/schemas.rs` reads `v17.json`'s
`added` to know which schema properties must carry a description (§1).

**What reads the ledger.** On the Rust side, nothing but the round-trip allowance above and the
re-record's own second-run rule (§4): the end-to-end gate, the stage tests, the screen tests and the
storage, career and report gates compare the whole file, Rust's values and Python's alike, with the
version equal to `ANALYSIS_VERSION`, `SCREEN_PARSER_VERSION` or `CAREER_VERSION`. On the Python side,
`conformance.frozen_view` takes every ledgered path out of both `expected` and frozen Python's answer
before comparing, and `ledger_covers` accepts a vector above frozen Python's version only when it
carries an entry for every version in between (§4). Both read engine and stage ledgers only. `check`
never reads a screen, storage or career ledger, because it runs none of those families.

### Why the corpus vectors are a slice, and what that costs

The stored clips are 239 MB of keypoints. Sliced to the frames the pipeline actually scored they
are 25.6 MB, and gzipped 8.3 MB. That is not a lossy reduction: `api/pipeline.py` picks the window
with `phases.select_swing` *before* the engine sees it, and frames outside it are read by nothing
that produces a number.

The cost is recorded rather than hidden. Because the slice is handed to the engine with
`face_on_window=None`, **every frame index in a corpus vector's `expected` is window-relative**,
where the `analysis.json` beside the clip holds whole-clip indices. `provenance.frame_offset` is
the difference. `conformance_vectors._verify_against_stored` adds it back at build time and
requires the archive to be reproduced exactly — which is what makes these the corpus rather than
fifteen plausible-looking files, and what caught the first attempt at recovering a swing's declared
loft from its own result (a loft the flight solve *refuses* on leaves no `club_loft_deg`
measurement behind, so the recovery silently produced `None` and a refusal sentence that blamed a
missing loft the run had).

The landmarks are **not** reduced, though they could be — `analysis/` names 15 of the 33 and reads
`z` on none of them, which would take the set to 3.0 MB. Declined: a reduced file is no longer a
`KeypointsFile` a port can parse with the shipped schema, and 5 MB does not buy a second shape plus
the test that would have to prove the reduction sound.

**Strikes come from the stored `{role}.audio.json`, not from re-detecting them**, and
deliberately: the stored artifacts are what the pipeline actually read. They are taken in file
order and not sorted or deduplicated, because `with_measured_impact` takes the strike nearest the
pose impact and a reordering here is a different tie-break there.

### Audio (`spec/vectors/audio/`) — 30 vectors, 4.3 MB, run by `cargo test`

Added by M20, and the family that closed the one tier-1 gap this document used to name. The
implementation under test is `crates/trigger`, not this repo's Python: ADR-030 §1 gives Rust
strike detection and M20 deleted `audio/impact.py` once these existed.

One vector per stored clip, recorded by that Python detector before it was deleted and verified
against the `{role}.audio.json` already beside each clip — the same `_verify_against_stored` guard
the corpus family carries, and what makes these the corpus rather than thirty plausible files.

**Windowing audio is lossy where windowing keypoints was not**, and that is why each vector has
two layers instead of one waveform. The corpus family ships a keypoint file's scored *slice*
because `phases.select_swing` picks that window before the engine sees it. Detection has no such
property: the median, the MAD and `MIN_RELATIVE_PROMINENCE` are all taken over the **whole** clip,
so an excerpt has different statistics and a different answer. Shipping every waveform instead
would have cost 68 MB. So:

| layer | input | expected | what it judges |
|---|---|---|---|
| statistics | the whole clip's flux envelope | the strikes found in it | the median/MAD floor, both prominence floors, peak-picking and suppression — at real clip length |
| the transform | two 0.5 s full-rate excerpts, one on the loudest transient and one on the quietest stretch | each one's flux envelope | `np.hanning`, the FFT, the `(peak + 1) * hop` sample convention |
| the second clock | *(the sibling view's envelope)* | `offset_between`'s `ClipOffset`, or a refusal | cross-correlation and the margin rule, on the 15 two-view swings |

Matching all three is matching `detect_strikes` on the whole clip. The one thing no layer reaches
is a signal long enough to cross the Python reference's 4096-frame block boundary (20.5 s), and
that is covered by a **property** rather than by bytes: the Rust implementation has no block
structure at all, so there is no stitch to get wrong, and
`flux::tests::envelope_is_independent_of_chunking` asserts a 25-second signal agrees with its own
parts. Buying the same assurance in bytes would have cost ~1.7 MB for one case.

Measured agreement: every sample index **exact** on all 30 clips, worst relative float difference
**7.7e-15** against `RTOL = 1e-9` — `rustfft` against numpy's pocketfft, and a sequential sum
against numpy's pairwise one, staying exactly where a reassociation lives.

### Stages (`spec/vectors/stages/`) — 21 vectors, 3.2 MB, **all seven stages run by `cargo test`**

Added by M22 P1, and the family that makes the *port* gateable rather than only the finished
engine. They were built before there was a line of Rust to run them — the same order M20 P0 used —
so that the first implementation of a stage has an answer to fail against. **M22 P4 is where that
started paying**: `crates/analysis/tests/geometry.rs` runs `smoothed`, `phases` and `measure` on
all 21, P5's `tests/judging.rs` adds `checkpoints`, P5b's `tests/measurements.rs` adds the
face-on three-sevenths of `measurements`, P7's `tests/alignment.rs` adds `alignment` and the two
`_dtl` groups, and P8's `tests/flight.rs` closes the set. **All seven run on all 21 as of M22
P8b**, which widened three of them rather than adding an eighth;
`conformance.py check` reports the whole family and defers it. The two families above gate the
**whole bundle** and nothing smaller — there is no
committed answer for "phases alone". That is fine for judging a finished port and useless for
building one, because it means every stage between the first line of Rust and a complete engine is
gated by review, which is what
[ADR-030](decisions/030-app-platform-rust-core-python-sidecar.md) §8 exists to forbid.

One file per engine vector, mirroring that family's layout and its compression decision —
`stages/synthetic/*.json` readable, `stages/corpus/*.json.gz` not. The input is held **by
reference**: `provenance.derived_from` names the engine vector, and none of the 8.6 MB above is
duplicated.

Seven stages, each a call `analyze_swing_bundle` already makes, in dependency order:

| stage | what it records | the M22 phase it gates |
|---|---|---|
| `smoothed` | every landmark's `x`/`y` after `smooth_keypoints` | P4 — **running** |
| `phases` | `segment_phases`, **window-relative** | P4 — **running** |
| `measure` | each `POSE_MEASUREMENTS` entry's *unrounded* value, or its refusal reason | P4 — **running** |
| `measurements` | `swing.measurements` partitioned into the seven groups that build it | P5b/P7/P8b — **all seven groups running** |
| `checkpoints` | each evaluator's raw `CheckpointOutcome`, registry order | P5 — **running** |
| `alignment` | both views' anchors before and after the strike pins impact, plus the warp | P7 — **running** |
| `flight` | the `FlownShot` — launch conditions, the whole integrated path, the refusals | P8/P8b — **running** |

**A green stage is not a covered stage, which P4 measured rather than assumed.** `measure` records
a value *or a refusal reason*, and on all 21 committed vectors every one of the thirteen metrics is
measurable — so the stage gates thirteen happy paths and **none** of `measure.py`'s twenty-odd
refusal branches, each of which carries a `detail` sentence §3 compares exactly and a golfer
reads. A port that got every refusal string wrong passes this family cleanly. The same holds
wherever a guard is inert on this corpus: P4 confirmed by mutation that five deliberate
divergences survive all 21 vectors, and they are listed in
[ADR-032](decisions/032-the-rust-core.md)'s P4 addendum with what each one needs in order to be
reachable. **Where a stage cannot see a branch, the port's own unit tests are the gate** — so a
phase that lands with nothing but a green vector run has not finished.

`checkpoints` is the same story a layer up and thinner: 21 vectors times six checkpoints is **126
evaluations and exactly one refusal**, so the stage gates 125 happy paths, `no_handedness`, and
none of the rest — not `NO_BAND`, which needs a per-club query no committed vector makes. The
sharpest instance is in ADR-032's P5 addendum: the corpus's one left-handed vector has a
`head_hip_gain_norm` of exactly `0.0`, so deleting `evaluate_head_stays_back`'s camera-frame mirror
— the entire reason that checkpoint takes a `Handedness` — passes all 21.

`measurements` turns the same question inside out, and P5b's answer is the sharpest one yet:
**every one of the fifteen corpus vectors *refuses* the face-on trajectory placement**, always on
the same two landmarks. The trail elbow and trail wrist are missing 45-70% of their resampled
timeline against `trajectory.MAX_MISSING`'s 40%, because face-on the trail arm crosses the torso
for most of the swing. So `tour_trajectory_t2` and `tour_trajectory_q` — and the `%g` percentile
and interval name their `detail` sentences interpolate — are gated by the **six synthetic vectors
only**, which have perfect visibility by construction; the fifteen real swings gate the refusal.
That is the opposite of the coverage "21 vectors, all green" suggests, and
`tests/measurements.rs::the_corpus_never_reaches_the_trajectory_basis` pins it so it fails the day
a corpus vector reaches the basis.

**`alignment` is where the gap between "green" and "covered" is widest, and M22 P7 measured all of
it.** The stage runs on all 21 and every answer matches, and four separate things it was supposed to
gate turn out to be unreachable from the committed vectors:

- **Every one of the fifteen two-camera pairs reports `synchronized`**, because every corpus clip pair
  heard the ball strike. `full`, `top_impact` and `impact_only` never reach a committed answer, so
  returning the constant `SYNCHRONIZED` from `align_swings` passes the whole family. The soft-anchor
  decision underneath *is* gated — it reaches the payload through `warp_motion_start` and the notes,
  seven pairs accepting and eight refusing — but the tier it sets is not.
- **`warp_top` and `top_late_by` are null on all thirty clips**, because the widest disagreement the
  two views have about the downswing is 27.6% against `_DOWNSWING_AGREEMENT`'s 30%. So
  `_shared_tops` and `_arbitrate_tops` return nothing on every vector, and that takes `_top_at`,
  `_tempo_restated`, the `IMPACT_ONLY` tier and `engine._without_contradicted_scores` with them —
  the last being the only place in the engine where a cross-view finding re-opens a face-on score.
  **The defect M11 P7 built the arbiter for has been repaired upstream**: on the *unpinned* anchors
  the widest gap is 21.7%, and pinning tau=2 to the heard strike moves it to 27.6%, still short of
  the threshold. `no_committed_pair_disagrees_enough_about_the_downswing_to_be_arbitrated` records
  the margin so it fails with a number the day a vector gets close.
- **Four of the fifteen reach the down-the-line trajectory basis and eleven refuse it**, which is the
  mirror image of the `measurements` finding above rather than a repeat: face-on the *trail* arm
  crosses the torso, and from behind it is the *lead* arm that hides — which the rear fit drops.
- **No two-camera vector is left-handed and none carries a `down_the_line_window`**, so the rear
  basis's mirror and `anchors_from_keypoints`' window branch have no committed answer either.

Each of the four is pinned by a test in `crates/analysis/tests/alignment.rs`, and the port's own unit
tests are what stand in for them — which is this section's rule doing more work in P7 than in any
phase before it.

**`flight` is the narrowest of the seven and the only one that adds a census beside the
tolerance** (M22 P8). It is green on all 21, and on most of them it is green because nothing flew:
**ten of the fifteen corpus vectors refuse before the integrator is reached** — `above_peak`,
`between_plateaus` or `no_club_loft` — so they gate the *solve* and never the flight. Five fly,
four of them fully clamped for their whole path, and exactly one (`2026-08-23-4`) reads an
interpolated coefficient row anywhere; nothing designed that, and it is pinned so a re-recorded
corpus cannot lose it quietly. **Three of the seven `SpinSolveCase`s reach a committed answer**;
the other four ship against the port's unit tests, `below_floor` being the one to know about
because it is the case ADR-027 §Decision 4 did not know existed. The census is the other half:
once `serde_json`'s `float_roundtrip` was on and `pyfmt` had CPython's three-argument `hypot`, all
**41,287 floats** in the five committed flights came back *bit-identical* rather than merely inside
`RTOL`, so the gate asserts that too with a 99% floor — a tolerance six orders above a ulp is
structurally unable to tell a converged integration from a systematically wrong primitive, and the
census is what caught the one below. The floor is 99% and not 100% because `exp`, `cos`, `sin` and
`atan2` are the platform's libm on both sides and another target may legitimately move a last bit.

**The `flight` stage is also the only one whose *caller* had to be gated from outside it** (M22
P8b). P8's three gates each hand a committed intermediate to the function under test — a recorded
`LaunchConditions` to the integrator, a recorded `UnspunLaunch` to the window and the solve — which
is what let the integrator be checked without the inference chain above it. `fly_shot` *is* that
chain, so its input is not in the stage at all and `tests/flight.rs` reads the **engine** vector for
it. Worth stating because of what it bought: until the two ends met, `resolved.launch` was an
assumption the port and the gate shared, and a port that built the wrong `UnspunLaunch` would have
passed all three of P8's cleanly.

**`feedback` was listed as an eighth stage and is not recorded.** The rule for what earns a key is
that a port must be able to run the stage *in isolation from the vector's input*, and
`build_feedback` takes the assembled `SwingResult`, which no stage produces and this family does
not hold. A port cannot reach it before the engine is finished, and by then `expected.feedback` on
the engine vector gates it — which is what M22 P6's end-to-end gate is. Recording it would have
been a second copy of an answer committed a few hundred bytes away.
[ADR-032](decisions/032-the-rust-core.md) §2 says eight; this is the correction, pinned by
`test_feedback_is_not_a_stage_and_that_is_deliberate` so the absence does not read as an oversight.

**Two stages are 97% of the bytes, and only one of them was reducible.** `smoothed` records `x`
and `y` alone: `smooth_keypoints` copies `z`, `visibility`, `frame_index` and `timestamp_ms`
through untouched, so recording them would record the *input* a second time and cost 2.3 MB
gzipped to test a copy. The pass-through is asserted at build time instead — the same trade the
audio family makes when it buys chunk-independence as a property rather than 1.7 MB of waveform.
`flight` is the other, and it is **not** reduced: most of it is the integrated path at
`DEFAULT_STEP_S`, ~919 points on a flown shot, and that path is a product surface M15 P15 and P19
draw rather than an intermediate — the Flutter shell will need it after `api/` retires.

**Grouping `measurements` by `Measurement.source` is the obvious way and is wrong**, which P1
found by producing eighteen `pose` rows against thirteen pose measurements: the face-on pivot rows
carry `pose:face_on` too, and `placements` and `placements_dtl` both carry `population:golfdb`.
`source` answers *which instrument read this*, which is not *which function appended it*, and only
the second question maps onto a port's phases. The groups are therefore a positional partition of
the one real list, asserted to cover it exactly.

**The guard that makes these evidence rather than twenty-one self-consistent files** is
`conformance_vectors._verify_stages_compose`, the stages family's answer to what
`_verify_against_stored` does for the corpus. `run_stages` calls the engine's own functions but
*re-orchestrates* them, and that orchestration is a second copy of `analyze_swing_bundle`'s. So
every stage that reaches the artifact is composed forward and compared against the committed
bundle under the same `compare_results` a port is judged by: `phases` shifted back by the window
offset, `measure` rounded the way `_measurements` rounds it, the groups concatenated,
`clip_alignment` against `expected.alignment`, the flight's refusals against `unscored`. It runs at
build time *and* in `tests/test_conformance.py`, because a build-time-only guard stops running the
moment nobody regenerates — which on this family is most of the time.

**From M32 both halves of that have a Rust copy, and the Rust one is what records.**
`crates/core/src/stages.rs` ports `run_stages` (`run_stages`) and the guard (`verify_compose`),
reaching the engine's own helpers rather than copying them, as the Python imports `E._windowed` and
its siblings. `crates/core/tests/stages.rs` holds both to the committed family — every stage
document reproduced under §3's rules, every one composing onto its engine vector's answer — and
`golf-core rerecord` runs the compose check on every re-record, against the engine document as it
will be written (§4). The port reproduced all twenty-one documents at zero differences on its first
run, though not bit for bit: **142** of their 286,343 numbers land inside `RTOL` without being
bit-identical (133 unrounded `measure` values, 8 checkpoint scores, 1 launch direction; M32 P2).
That is why a re-record writes Python's bits back for every undeclared value rather than Rust's
document. The stage tests in `crates/analysis/tests/` keep their own orchestration, which gates
the port one stage at a time; the compose check is what catches this second Rust copy drifting.

The family derives from the **committed** engine vectors rather than from `data/processed/`, so it
never needed the capture machine. `regenerate --stages-only` used to rebuild it on any box; it
refuses since M32
([ADR-035 §3](decisions/035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)),
and `golf-core rerecord` re-records the stage vectors that exist. **No command creates one**: a new
engine vector and its stages are §M29's Rust vector builder's. The Python engine stays, frozen,
until M40 deletes it — see [ADR-032](decisions/032-the-rust-core.md) §7 and its 2026-09-30
addendum for that schedule.

### Format (`spec/vectors/format/`) — 13 vectors, run by `cargo test`

Added by M22 P3, extended by P5, and extended again by M34 P2 and M36 P4. It is the only family whose subject
is **not this repo's code at all**. It records what CPython itself does, because §3's edges below
are where a Rust port diverges from Python on the *same* arithmetic, and nearly every one of them
lands on the strings §3 compares exactly. The tables are ported before anything calls them, so each
edge is solved once rather than at every call site downstream. The case count is `check`'s to print.

**Each table names the crate that implements it.** The five M34 added and the three M36 added carry
`provenance.implemented_by`. The five engine tables predate the key and were not regenerated to gain
it, because M34 P2's rule was that they come back byte-identical, and M36 P4 held the ten older
tables to the same rule. So its absence means `pyfmt`. Three
things read it: `crates/pyfmt/tests/format.rs`'s discovery pin, which fails on a table no test runs;
`check`'s summary line; and `tests/test_conformance.py::test_every_format_vector_names_the_crate_that_runs_it`,
which holds every *new* table to naming its crate. M36's edges each got a table of their own rather
than rows in an old one, because adding rows would have rewritten that table's note.

| vector | what it records | the edge | implemented by |
|---|---|---|---|
| `rounding` | `round(x)` and `round(x, n)` | engine 1 — half-to-even, and decimal-aware at `n` places | `pyfmt` |
| `fixed` | `f"{x:.Nf}"` at the four precisions the engine interpolates | engine 2 | `pyfmt` |
| `general` | `f"{x:g}"` at CPython's default precision of 6 | engine 2 | `pyfmt` |
| `repr` | `str(x)` — an f-string with no format spec at all | engine 4 | `pyfmt` |
| `ordering` | a stable sort over an insertion-ordered dict, and `engine.py`'s registry rank | engine 3 | `pyfmt` |
| `difflib_ratio` | `SequenceMatcher(None, a, b).ratio()` over every normalized box text against every label | parser 1 | `screen` |
| `str_repr` | `repr(s)` on a `str` — the quote, the escapes, printable non-ASCII kept | parser 2 | `pyfmt` |
| `floor_div` | float `a // b`, and `int(a // b)` | parser 3 | `pyfmt` |
| `text_case` | `str.upper()`, `split()`, `strip()` and `re`'s `\s`, plus both whole whitespace sets | parser 4 | `pyfmt` |
| `sum` | `sum()` over float lists, compensated since CPython 3.12 | parser 5 | `pyfmt` |
| `general_precision` | `f"{x:.Ng}"` at precisions other than 6, the session-drift caveat's `:.3g` among them | many-shot 1 | `pyfmt` |
| `lower` | `str.lower()` over every code point it moves, and the string cases (`İ`, `Final_Sigma`) | many-shot 2 | `pyfmt` |
| `timestamp` | a pydantic `datetime` as `model_dump_json` writes it, with its `isoformat()` and its `%Y-%m-%d` in its own offset; the other spellings pydantic reads back; and strings `Timestamp` refuses | many-shot 3 | `contracts` |

**It does not age on a version of this repo's.** `ANALYSIS_VERSION`, `SCREEN_PARSER_VERSION` and
`CAREER_VERSION` all leave every answer in it true, because a rounding rule is the language's, not
the engine's, the parser's or the corpus reader's. What it carries instead is `python_version`, and
`timestamp`'s provenance carries `pydantic_version` beside it, because its subject is pydantic's. `check` therefore reports these without a
staleness test, where it runs one over the stages. That is pinned in `tests/test_conformance.py`,
because the *absence* of the field is the decision, and it reads as an oversight otherwise.

**Every float travels as its Python `repr`, not as a JSON number.** `repr` is shortest-round-trip
and Rust's `str::parse` is correctly rounded, so both sides land on identical bits and the table is
compared with **no tolerance at all** — nothing in it computes a measurement, so §3's `RTOL` would
be measuring the wrong thing. It also sidesteps `json.dumps` writing a bare `NaN` that `serde_json`
rejects, which matters because non-finite input is one of only two places the two languages print
different text (`NaN` against `nan`; the other is that Rust has no `%g` at all).

**The tie cases are enumerable rather than sampled, and that is what keeps the table small.** A tie
at `n` decimal places needs `x * 10**n == j + 0.5` exactly, which forces `x` to be an odd multiple
of `2**-(n+1)` — so the values where half-to-even and half-away-from-zero differ can be listed
completely, and a uniform sweep hits **none** of them. The table carries each tie with its two
nearest neighbours, which must round the other way, then the decimal traps, the engine's own
unrounded values read off the stage family's `measure` rows, and a seeded sweep for breadth.

`regenerate --format-only` rebuilds it on any machine, and since M32 it is the one vector rebuild
`regenerate` still does (§4): this family records CPython rather than the engine, so moving the
engine's oracle to Rust left it where it was. The screen tables need no `data/` for that: the box
texts they run over are transcribed into `conformance_vectors._OCR_TEXTS` rather than read from the
photos.

### Screen (`spec/vectors/screen/`) — 48 vectors, run by `cargo test`

Added by M34, and the screen reader's oracle: OCR boxes in, `ShotData` out. The implementation under
test is `crates/screen`. **It gates the parser, never the recognizer.** The phone reads the screen
with Apple Vision and the lab with PaddleOCR, and nothing makes their boxes identical. Whether
Vision's boxes parse as well is M33's question, and whether §M29's Rust reader matches PaddleOCR's
is §M29's.

| sub-family | what it is | oracle | version |
|---|---|---|---|
| `corpus/` | PaddleOCR's boxes for each stored bay photo, deduplicated by content sha256 and named by the stored shot's `shot_id`. Each was verified at record time against the stored shot, on every key but `provenance.image_path` | `python`, with a Rust ledger | 1 |
| `reference/` | `IMG_2738` and `IMG_2739`, the other layout, read directly and written to no store | `python`, with a Rust ledger | 1 |
| `synthetic/` | `tests/launch_monitor/conftest.py::build_screen`'s screens, one per parser or validator path, each `note` naming its path | `python`, with a Rust ledger | 1 |
| `hand/` | the tie rule and the `Impact Position V` tile. Each `note` works the scores, the sums and the confidence by hand from the stated rule | `hand` | 1 |
| `units/` | case tables over the frozen parser's private functions (`normalize_label`, `field_for`, `_first_number`, `_sign_from`, `_is_blank`, `_Cell.text`), each case carrying the profile it ran against | `python`, with no ledger | 0 |

**Two shapes.** A *document* (every sub-family but `units/`) carries the following:

- **`input`**: the device, the boxes with their geometry, the preprocessing `notes`, the identity
  fields, and `min_confidence`. That is `screen::ScreenInput`, and it refuses an unknown key.
- **`expected`**: `label_ratio`, `parsed`, and `shot`.
  - `parsed` is the parse *before* validation, with its confidence unrounded and its warnings
    without the notes.
  - `shot` is the `ShotData` the reader makes, or `null` where frozen Python's `import_screen`
    returned `failed`.

  Splitting `parsed` from `shot` is what let the parser be gated before there was a validator.

A `units` table carries `cases`, with floats as `repr` strings, as the format family carries them.

**The family ages on `SCREEN_PARSER_VERSION`** (`crates/contracts/src/shot.rs`, whose ledger says
what each version is), never on `ANALYSIS_VERSION`, and carries it at the top level as
`screen_parser_version`. Version 0 is frozen Python's parser. Version 1 is the Rust parser as M34
shipped it, with the tie rule, the forked profile's `Impact Position V` tile, `fields_present`, the
`parser_version` stamp and ranges for M32's new fields. `provenance` carries `kind: "screen"`,
`oracle`, a `note` and a `source`. Where Python recorded the vector it also carries
`python_version`, and on the photo vectors `paddleocr_version`.

**How it came to be at version 1.** M34 built it record, port, then change:

- **Recorded once.** `regenerate --screen-once` recorded it from frozen Python, once. It refuses for
  as long as any screen vector exists (§4).
- **Ported faithfully.** `crates/screen` passed every recorded vector before anything changed.
- **Changed, against the hand-worked vectors.** The faithful rules stood behind a `frozen` module
  until the change was done, so the Python-recorded vectors kept gating the code both parsers shared.
- **Re-recorded.** `golf-core rerecord --declare spec/declarations/screen-v1.json` re-recorded the
  Python-recorded documents through the shipping reader, and the faithful port was deleted.

What moved:

- **Values moved on three documents.** On the two label-fix shots, `2026-08-10-1`'s shot type loses
  the `CENTER` spill, and `2026-08-23-1`'s impact position goes from `HEEL` to `null`. The third is
  `synthetic/duplicate-label`, whose Bounce & Roll had held a spilled Carry.
- **The rest is declared bookkeeping, and every path is in the declaration.** No `needs_review`
  flipped.
- **Where the detail is.** The per-document table is in
  [the M34 plan's P10 findings](plans/m34-screen-reader.md#p10--the-13-shot-re-read-2026-10-02), and
  what the parser now decides is
  [ADR-014's M34 addendum](decisions/014-screen-capture-shot-ingestion.md#addendum-2026-10-02-m34-the-parser-is-rusts-the-profile-is-forked-and-a-tie-is-withheld).

**Every screen document now answers to the shipping reader**, `screen::read` and the parser under
it, and every `units` table to the function it records. The runners are in `crates/screen/tests/`:

- `parse.rs` and `read.rs` run the three recorded document sub-families, and `hand.rs` runs
  `hand/`;
- `units.rs` runs the case tables;
- the verb's own test, `crates/core/tests/parse_screen.rs`, pipes every document through
  `golf-core parse-screen`.

A discovery pin fails on a sub-family that no test runs. Two of the pins are worth knowing before
leaning on a green run:

- **The confidences are held to the bit, not to `RTOL`.** These are `parsed.confidence` (with
  `label_ratio`) and `parse_confidence`, because the edge they exist for is an ulp. With `pyfmt::sum`
  swapped for a left fold, every document still passes under `RTOL`, and the bit tests fail on three
  synthetic ones. A re-record compares under `RTOL`, so a declaration cannot see such a move either.
  The rounded `parse_confidence` moves by at least 0.001 or not at all, so the re-record is safe for
  that one.
- **`units/` stays frozen Python's.** It holds tables, not documents, so the re-record never reads it
  (`SCREEN_UNREAD`). Each `field_for` case carries its own profile, so the fork's V tile cannot move
  it. A parser change that moves a private function's answer needs its own decision about this
  table.

**What reaches no screen vector**, measured and pinned by unit tests instead:

- **Seven paths in the validator and the record.** Among them are an exact binary tie in
  `round(…, 3)`, a printed smash just above the computed one, and a value sitting on a range bound.
- **Float `//` against `floor(a / b)`.** No photo's geometry lands where the two part. Only
  `units/cell_text`'s floor-division case gates the parser's use of it.
- **A direction word that *disagrees* with a printed sign.** `synthetic/word-beats-printed-sign`
  cannot tell the two rules apart, and the frozen Python test it came from has the same gap.

The M34 plan's P5 and P6 findings list each of these, with the mutation that found it.

### Storage (`spec/vectors/storage/`) — 64 vectors, run by `cargo test`

Added by M36. It is the oracle for the flat-file stores and the corpus reader: a tree of files in,
and what each store answers and leaves on disk out. The implementation under test is
`crates/storage`, ported from `storage/`, from `api/state.py`'s tolerant readers and from the shot
store (`launch_monitor/screen/store.py`).

| sub-family | what it is | oracle | version |
|---|---|---|---|
| `corpus/` | `read_corpus` and its narrowings over a sessions tree. Every `ExclusionReason`, both `MishitVerdict`s and the automatic flag, the sort and both duplicate tiebreaks, corrupt manifests, states and analyses. `real.json.gz` is `data/`'s swing directories, each `analysis.json` slimmed to the three keys `read_corpus` reads, and verified at record time to read exactly as `data/` itself does | `python`, with a Rust ledger | 1 |
| `bundle/` | the bundle store: its reads; `assign_from_path` into an empty session, into the newest swing lacking the role, at an explicit `swing_id` (existing and new), and on a duplicate digest; `set_player`, `set_club`, `set_mishit`, `attribute_unlabeled` and `delete_swing` | `python`, with a Rust ledger | 1 |
| `stores/` | the bag, golfer and shot stores, with the bag's validator refusals, and a `slugify` table | `python`, with a Rust ledger | 1 |
| `hand/` | where the corpus's `OUTDATED` rule moved (below): stored versions either side of `comparable_from`, a line at the caller's number rather than the constant, and a mishit median that sees only comparable swings. Each `note` holds its working | `hand` | 1 |

**Two shapes.**

- **A corpus case.** `input` is `{player_id, versions: {installed, comparable_from}, files,
  narrowings}`, where `files` is the sessions directory as raw text, so a half-written state file is
  a case like any other. `files: null` is a root that does not exist, `{}` an empty one, and a key
  ending in `/` an empty directory. `expected` is `{corpus, properties, narrowed}`. `properties`
  holds `CareerCorpus`'s derived counts, which `model_dump` drops and every report prints.
- **An operation case.** `input` is `{files, ops: [{op, args, now}]}`, and `expected` is `{results,
  files}`: each op's `{"returned": …}` or `{"raised": {type, message}}`, then the whole tree after
  the last op. The clock is an argument. Each op carries its own `now`, and one that cannot stamp
  carries `null`. A written JSON file is compared as a value, never as bytes, because key order and
  float spelling are not what frozen Python reads by.

`provenance.recorded_by` names the runner that recorded a case (`_run_corpus` or `_run_ops`), and
`crates/core::storage_family::run_storage` dispatches on it. A `hand/` case names the runner it was
worked against in `provenance.worked_against` instead. A hand case carrying `recorded_by`, or a
Python one carrying `worked_against`, is refused.

**The engine versions are input, not the build's.** `read_corpus` takes `{installed,
comparable_from}` from its caller, and each corpus case records the pair it was read under. So an
`ANALYSIS_VERSION` bump moves neither this family nor the career family. Frozen Python recorded every
case at `{16, 16}`, and the verbs pass `{ANALYSIS_VERSION, COMPARABLE_FROM}`.

### Career (`spec/vectors/career/`) — 56 vectors, run by `cargo test`

Added by M36. It is the oracle for what a golfer's history may say: a corpus and a bag in, and the
four aggregates and the five career scripts' report text out. The implementation under test is the
career half of `crates/analysis` (`baseline`, `dispersion`, `comparison`, `club_profile`, and
`stats`' interval helpers), with `crates/core::reports` for the text.

- **`input`** is `{corpus, bag, display_name, versions, clubs}`. `clubs` is every club in the bag
  profile, then the first club that is in neither the history nor the bag, whose report is the
  "never hit and not in the bag" sentence.
- **`expected`** is `{baseline, dispersion, standing, bag_profile, properties, reports}`.
  `properties` holds what each aggregate derives and `model_dump` drops.
- **`reports`** is each script's own report function, captured: `career_corpus`, `career_baseline`,
  `career_dispersion` and `club_profile`, plain and `_verbose`; `club_profile_<club>` for each of
  `input.clubs`; and `flag_mishit_list`, rendered over the bag profile built with no bag, as the
  script builds it. Recording the text is what keeps it gated after §M29 deletes the scripts.

`synthetic/` crosses every floor at n − 1 and n, the default row and both per-metric override rows,
and the sessions gate at 2 and 3. It reaches every dispersion pattern, standing and caveat form, and
sample sizes past the critical-value tables, so the interval expansions are gated too. Most of its
cases are **adopted**: a synthetic storage corpus case's `expected.corpus`, taken as `input.corpus`
and named in `provenance.source`. `real/aaron.json.gz` is the real corpus, with the real bag.

**An adopted corpus is derived, so the two families cannot drift apart.** `golf-core rerecord` runs
each adopting career vector on its source's answer as that answer will be written (§4), and
`crates/core/tests/career.rs::every_adopted_corpus_is_its_storage_vectors_answer` holds the
committed families equal.

**Both families age on `CAREER_VERSION`** (`crates/contracts/src/career.rs`, whose ledger says what
each version is), one key for both, carried at the top level as `career_version`. Version 0 is
frozen Python's. Version 1 is M36's change to `OUTDATED`. M35's change to `read_corpus` will be
`career-v2`.

**How they came to be at version 1**, in M34's order:

- **Recorded once.** `regenerate --storage-once` and `--career-once` (M36 P2 and P3) recorded them
  from frozen Python. Each now refuses (§4).
- **Ported faithfully.** `crates/storage` and the career half of `crates/analysis` passed every
  recorded vector, `OUTDATED` at the installed engine included.
- **Changed, against the hand-worked vectors.** The corpus excludes a swing as `OUTDATED` when its
  `analysis_version` is older than `versions.comparable_from`, no longer than the installed engine.
  The verbs pass `contracts::swing::COMPARABLE_FROM`, the oldest engine generation whose stored
  numbers today's engine still agrees with. The faithful rule had excluded every swing on disk once
  Rust's `ANALYSIS_VERSION` reached 17, though 16 → 17 moved no number. `is_outdated`, the other
  question (*is there a newer engine to run?*), is unchanged.
  [ADR-024](decisions/024-per-club-shot-history.md)'s M36 addendum is the decision.
- **Re-recorded.** `golf-core rerecord --declare spec/declarations/career-v1.json` moved the
  `OUTDATED` sentence, its adopted copies, and `career_version`, and nothing else. No swing, count or
  aggregate moved, because under `{16, 16}` the two rules exclude the same swings (§4 has the run).

**The runners** are in `crates/core/tests/`, each calling the definition the re-record calls:

- `storage.rs` runs every storage vector through `storage_family::run_storage`;
- `career.rs` builds every aggregate through `career_family::run_career`, and holds a built metric's
  withheld claims to exactly what its floors refuse;
- `reports.rs` holds every report to the recorded text, character for character, and the verbs end
  to end through the binary.

`crates/contracts/tests/{stores,career,aggregates}.rs` read and write back every bag, golfer, corpus
and aggregate either family holds, exactly. `crates/contracts/tests/data/python_tables.json` holds
the dispersion and comparison prose tables word for word, as frozen Python declares them.

Two pins are worth knowing before leaning on a green run:

- **The floats are held to the bit, not to `RTOL`.** With plain folds for the compensated `sum()`s,
  about one baseline float in six moved by a bit, and every one passed the gate. So
  `every_aggregate_float_is_frozen_pythons_to_the_bit` compares every float the four aggregates hold
  by its bits. It runs on Windows with MSVC, where the vectors were recorded, because another libm
  may answer a last bit differently, and CPython over it would too.
- **Float `**` is C `pow`, and no vector can see it.** The product for `x ** 2`, and `sqrt` for
  `** 0.5`, both pass every career vector, the bit pin included. `stats`' and `dispersion`'s own
  bit pins hold CPython's answers on the rows that separate them (§3's third list).

**What reaches no vector**, pinned by unit tests instead: two swings tied on the instant across
sessions, two sessions tied on it, an interval ending exactly on a band edge, a signed-zero tie in a
range, and a shot that fails only a bound. Each is a case no recording reaches, and all but the
last were found by a deliberate divergence that passed the whole family. The M36 plan's P10–P12
findings list them, with the mutation that found each.

**Over `data/` the verbs print what the scripts print.** M36 P16 ran all five against their scripts
with every flag the scripts take: 85 runs, identical bytes, once Windows' `\r\n` and cp1252 console
were taken off the Python side. That was a one-time check, not a gate. `reports.rs` is the gate.

## 3. The rules

Implemented twice, once per side of the freeze, and each is unit-tested against the differences it
must see — because "it passed" from a comparator that cannot see a difference is worth nothing.
`conformance.compare_results` is frozen Python's, tested in `tests/test_conformance.py`.
`golf_core::compare` (`crates/core/src/compare.rs`) is Rust's: it moved out of
`crates/core/tests/engine.rs` into the library at M32 so the re-record could gate on the same
answers the end-to-end test reads, and it reports each difference as a path and a kind (an added
key, a removed key, or a moved value), which is what a declaration is matched against. The two agree
on every rule below. Where they could have parted, they do not: an int and a float that agree are
not a difference in either, because both read an int as exact only when *both* sides are ints.

| Kind | Rule | Why |
|---|---|---|
| `null` | A refusal compares equal to nothing but a refusal | **ADR-010 §2.** A port returning `0.0` where this returns `None` has turned "could not measure" into "measured zero". Reported as a type difference and never tested numerically |
| Structure | Same keys, same list lengths, same order | `checkpoint_scores` and `unscored` are ordered by `CHECKPOINT_REGISTRY`, which a port walks too, so an order difference is a finding |
| `bool` | Exact, and checked **before** `int` | `isinstance(True, int)` is true in Python, so the obvious ordering compares a verdict numerically and lets `1` through for `True` |
| `int` | Exact | Frame indices, `population_n`, `analysis_version` |
| `str` | Exact | Including every `message` and `detail` sentence a golfer is shown |
| `float` | `abs(a - b) <= ATOL + RTOL * abs(expected)`, `RTOL = 1e-9`, `ATOL = 1e-12` | Sized for a different **summation order**, not a different rule: the last bits of an f64 are the same measurement added up differently; a hundredth of a degree is a different measurement |

**A band edge is not protected by the float tolerance and is not meant to be.** `passed` is a bool
and compared exactly, so a port landing a hair the other side of a band fails on the *verdict*
rather than on the number — which is the failure worth being loud about, and why widening `RTOL`
would be the wrong repair for it.

### The known edges a Rust port will hit

Five, and the count moved three times while the port was being written. Two were predicted when
this milestone was written; the third was found by M22 P0 reading the engine for them; the fourth
by M22 P5 writing the sentences that reach a golfer; the fifth by M22 P8 integrating a flight.
All are real in the shipped code, and M22 solves them in one module,
`crates/pyfmt/src/lib.rs`, rather than where each one bites.

**Four of the five land on the strings this section compares exactly, and the fifth lands on a
bool** — which is the more useful way to read the list than by what each one is called. A port can
get every float right to the last bit and still fail on a sentence; it can also get every string
right and have a `>=` fall the other way.

There is a **sixth** divergence that is not here, and the line is worth stating because it is where
someone will look for it: M22 P4 found that Python's `max` returns the *first* maximum where Rust's
returns the last (`min` agrees, which is what hides it), and `phases._top_and_impact` reads a `max`
over frame indices. That is a tie-break in an iterator and not a CPython formatting rule, so it
lives in `phases.rs` and is gated by the stage vectors. See ADR-032's 2026-09-24 addendum.

- **Rounding.** Python's `round()` is half-to-even (banker's); Rust's `f64::round` is
  half-away-from-zero. Forty-six sites in `analysis/`, in two kinds, and the second is the
  dangerous one: thirteen round a value that is then serialized (into `Measurement.value` and
  `CheckpointScore.observed`), and seventeen round a **frame index**, where a one-frame divergence
  changes which frame a checkpoint is measured on and cascades into every score after it.
  `analysis/alignment.py` holds ten of the second kind — it rounds a float frame count in every
  anchor fallback it has, `grep -n 'round(' src/golf_coach/analysis/alignment.py` — and the hop
  size at 44.1 kHz landed on the same split back when `impact.py` was Python.
  `round(x, ndigits)` is a second trap: it is decimal-aware and is **not**
  `(x * 10**n).round() / 10**n`. `benchmarks/joint.py` and `benchmarks/trajectory.py` use the
  two-argument form throughout, and those values reach `percentile`.
- **Float formatting reaches the strings.** `%g`, `.0f` and `.1f` are formatted into `message` and
  `detail`, which §3 compares exactly — `checkpoints/mechanics.py:225`, `:253`, `:269-271`,
  `:276-282` and `:476`, and `engine.py:114`. Rust's `{}` for `f64` is not `%g` and its `{:.1}` is
  not Python's `.1f` at a tie. A port that gets every number right and formats one of them
  differently fails on the sentence. One `%g` is not prose at all —
  `benchmarks/flight_model.py:205` formats an altitude into a **dict key**, so getting that one
  wrong is a lookup miss rather than a wrong sentence.
- **`str(x)` on a float, which an f-string with no format spec reaches.** Three of
  `mechanics.py`'s sentences interpolate a bare band edge — `f"aim under {band.high}"` — and in
  Python 3 `str`, `repr` and `format(v, "")` are one function for a float. Rust's `{}` produces the
  same shortest-round-trip *digits* under different presentation rules: it writes `4` where CPython
  writes `4.0`, and it has no exponent form at all where CPython switches at `decpt <= -4` and
  `decpt > 16`. **Every band edge shipping today formats identically in both languages**, so a port
  using `{}` passes every vector in this repo — which is why this edge is gated by the `repr` table
  above rather than by a swing. Found by M22 P5.
- **Dict insertion order decides which name appears in a sentence.** `benchmarks/joint.py:111` and
  `benchmarks/trajectory.py:168` each build `dict(sorted(shares.items(), key=…))`; `engine.py:100`,
  `:118` and `:181` then take `next(iter(...))` and interpolate the answer into `Measurement.detail`
  as "Largest contributor" and as the interval most of a value falls in. **On a tie the winner is
  decided by Python's stable sort over the dict's original insertion order**, which a Rust `HashMap`
  destroys outright. This is the hardest of them to diagnose, because it produces a correct
  number attached to the wrong label, and only on the swings where two shares happen to tie.
  `engine.py:797` has the same shape on `unscored`: every unregistered name collides on one sort
  key and is therefore ordered by stable-sort-of-input.
- **`math.hypot` with three arguments is a compensated norm, and the ulp reaches a bool.**
  `flight.py` takes the ball's speed as `math.hypot(vx, vy, vz)`. Rust's std has only the
  two-argument form, so a port reaches for a chain or for `(x²+y²+z²).sqrt()` — and both sit within
  **1 ulp** of CPython at every point of a committed flight, five orders inside `RTOL` and
  indistinguishable from nothing. One ulp is the whole difference. `carry_window` computes
  `high_plateau_min_rpm` *analytically*, so the launch spin ratio lands **exactly** on the
  coefficient table's last row: CPython's norm gives the reference shot `40.546527999999995` where
  both approximations give `40.546528`, and `AeroTable.coefficients_for`'s `>=` therefore clamps in
  one language and interpolates in the other. `clamped` is a bool and is compared exactly. So
  `pyfmt` carries CPython's `vector_norm` — Dekker's split, three Neumaier registers and a
  compensated Newton correction — with a 32-case table of `math.hypot`'s own answers beside it. The
  **two**-argument sites in `measure.py` and `pivot.py` were deliberately left on `f64::hypot`:
  every answer they reach is a float compared within `RTOL` and none reaches a bool, so moving them
  would move numbers four green stages already agree on for no gate. Found by M22 P8.

### The screen parser's edges, a list of their own (M34)

`analysis/` reaches none of these, so they are not added to the list above. An engine port that
read them would go looking for a `difflib` it never calls
([ADR-032](decisions/032-the-rust-core.md)'s 2026-09-29 addendum).

- **Where the list came from.** The twelfth addendum predicted six, and M34 found three more by
  building: `strip()`, Unicode `\d`, and the compensated `sum()`.
- **How they are solved.** Each is solved in `crates/pyfmt` or `crates/screen`.
- **How they are gated.** Each is gated **exactly**, before any parser code called it, by a format
  table recorded from CPython (§2) or by a `units` table recorded from the frozen parser.

The parser also reaches two of the engine's edges. `validate_parse`'s messages interpolate `:g` and
`.3f` (`pyfmt::{g, fixed}`), and the record rounds its confidence with banker's `round(…, 3)`
(`pyfmt::round_to`).

- **`difflib.SequenceMatcher.ratio`** scores every OCR label against every field, and Rust's std has
  no counterpart. `crates/screen/src/difflib.rs` reproduces it to the bit:
  - `find_longest_match`'s tie-break, where `("aaa", "aaba")` is 6/7 in CPython and 4/7 with `>=`;
  - `get_matching_blocks`;
  - `autojunk`, which fires only when `len(b) >= 200`.

  It is gated by `format/difflib_ratio`. The table cannot reach the backward extension of a match,
  and a unit test holds CPython's answer for it.
- **`{text!r}` in warnings**, which are compared exactly. `repr` picks its quote by content and
  escapes in its own notation. `pyfmt::str_repr` reproduces it, where `{:?}` differs on 272 of the
  table's 274 cases. Its printable test reads Rust's own Unicode table, which agrees with CPython's
  15.1 on every code point 15.1 assigned. It is gated by `format/str_repr`.
- **Float `//`**, in the line buckets `_Cell.text` sorts value boxes into. CPython's `//` is
  `fmod`-based, not the floor of the quotient: `21.0 // 4.2` is `4.0`, where `21.0 / 4.2` rounds to
  exactly `5.0`. On the integer-pixel grid, 300 of 320,080 pairs differ. `pyfmt::floor_div` is gated
  by `format/floor_div`, and in the parser only by `units/cell_text`, because no photo's geometry
  lands on a differing pair.
- **Unicode case and whitespace.** `str.upper()`, `split()`, `strip()` and `re`'s `\s` share one
  whitespace set. Rust's `char::is_whitespace` is that set minus U+001C–U+001F, and nothing else
  differs, which was checked over every code point. `pyfmt::{upper, split, strip, strip_space,
  is_space}` reproduce it, gated by `format/text_case`. `upper` still differs on 27 code points,
  because Rust 1.87 is on Unicode 16.0 and CPython 3.13 on 15.1. Those are left uncorrected, because
  no launch monitor prints them.
- **`sum()` is compensated** (Neumaier) since CPython 3.12, and the vectors were recorded on 3.13.3.
  The parser's confidence is a mean over OCR confidences, and it reaches `round(…, 3)` and the
  `< min_confidence` bool. `pyfmt::sum` reproduces it, gated by `format/sum`, and by the bit tests in
  §2's screen family.
  - **Where it bites, and where it does not.** It moves no photo's mean, because float32 confidences
    sum exactly in 53 bits, whether PaddleOCR's or VisionKit's. It moves the synthetic screens' mean,
    whose constant `0.95` does not sum exactly.
  - **The engine has this edge too, and it is not fixed there.** `crates/analysis` ports its `sum()`
    sites as left folds, and on 4 of the 21 engine vectors that parts from CPython by 1–2 ulp, inside
    `RTOL`. One site, `pivot.rs:469`'s `sum(deltas) >= 0`, reaches a **branch**, and is unmeasured.
    ADR-032's fifteenth addendum routes the question rather than counting it here.
- **`_THOUSANDS` is a look-behind regex, and `\d` is Unicode.** The `regex` crate has no
  look-around, and is kept out anyway, so `parser::first_number` is a hand scanner. Run over the
  matched text, `\d{3}\b` reduces to "three digits not followed by a digit". `\d` and `float()` read
  every Unicode decimal digit, so the scanner carries Unicode 15.1's 680 as a table
  (`DECIMAL_ZEROS`). It is gated by `units/first_number`, which fails on four cases with ASCII
  digits only.
- **Two tie rules that point opposite ways.** `DeviceProfile.field_for` keeps the *last* field at a
  tie (`>=`). `profile::field_for` reproduces it, gated by `units/field_for`, and `label_ratio`
  still counts through it. `_find_labels` keeps the *first* box (`>`). That one was ported, and gated
  by the Python-recorded vectors, until M34 replaced it with the tie rule
  ([ADR-014's M34 addendum](decisions/014-screen-capture-shot-ingestion.md#addendum-2026-10-02-m34-the-parser-is-rusts-the-profile-is-forked-and-a-tie-is-withheld)).
  That is now a declared change of behaviour, not an edge.

### The many-shot layer's edges, a third list (M36)

Found porting the stores, the corpus reader and the career aggregates, and kept apart from the
engine's six for the screen list's reason: `analyze_swing` reaches none of them, except possibly the
first, whose engine sites are unmeasured (below). Each is solved in `crates/pyfmt`, in
`contracts::time`, or at its call site, and gated by a format table, a family, or a bit pin where
§3's `RTOL` cannot see it. The M36 plan's P3 format scan and the P4, P5, P8, P11 and P12 findings
have each measurement.

- **Float `**` is C `pow`.** `(value - mean) ** 2`, `sd ** 2`, the critical-value expansions' cubes
  and fifth powers, and `_within_session_sd`'s `(squares / degrees) ** 0.5` all call the C library's
  `pow`. On this box's UCRT that is not the correctly rounded product even at an exponent of 2: the
  product missed 12 of 20,000 random squares, and `pow(x, 0.5)` differs from `sqrt` on 524 of
  1,000,000. So `stats::pow` is `x.powf(black_box(y))`. **The `black_box` is load-bearing**: with the
  2.0 visible, LLVM rewrites `pow(x, 2.0)` to `x * x` in a release build and not in a test build, so
  the gate would certify arithmetic the verbs do not run. No career vector can see this. The bit pins
  in `stats` and `dispersion` hold it, on Windows with MSVC. **The engine has `**` sites too**
  (`measure.py`, `phases.py`, `benchmarks/trajectory.py`, `benchmarks/flight_model.py`), and its port
  writes them as `powi(2)` and `sqrt`. Every engine and stage vector passes, so no committed answer
  moved, but nobody has measured whether any engine float differs in its last bit.
  [ADR-032](decisions/032-the-rust-core.md)'s sixteenth addendum routes that, as the fifteenth routed
  `sum()`.
- **The compensated `sum()` reaches the baseline**, in `mean_and_sd` (both sums) and
  `_session_means`, through `pyfmt::sum`. `_within_session_sd` accumulates with a plain `+=`, so it
  must **not** use `pyfmt::sum`. The `RTOL` gate cannot see either choice. §2's career bit pin can.
- **`:.3g`.** The session-drift caveat formats its two spreads at precision 3, and `pyfmt::g` was
  precision 6 only. `pyfmt::general(x, precision)` is CPython's `:.Ng` (a precision of 0 is read as
  1, as CPython reads it), gated by `format/general_precision`.
- **`str.lower()`.** `pyfmt::lower` is `str::to_lowercase`, which agrees with CPython 3.13 on every
  code point CPython lowers, `İ` and `Final_Sigma` included. Rust also lowers 27 code points that
  Unicode 16.0 added and CPython's 15.1 leaves alone. Each lands on a non-ASCII letter, so it can
  only make a name match nothing. Gated by `format/lower`.
- **`str.isalnum()` is not `char::is_alphanumeric`.** `club.py` and `club_spec.py` normalise by
  keeping `isalnum` characters. Measured over every code point, Rust's set is CPython's plus 5,877
  more (combining marks among them), and none less. So `7i` with a combining accent normalises to `7i`
  in Python and matches nothing in Rust. This is **named, not fixed**: every alias is ASCII, so Rust
  can only refuse what Python takes, never the reverse, which is the safe direction under ADR-024
  §5. `club.rs`'s module doc carries the argument.
- **`slugify`'s "combining" is the canonical combining class**, not general category M.
  `unicodedata.combining` keeps U+0903 and U+20DD, which have class 0, so `aःb` slugs to `a-b`. The
  normalization tables differ by Unicode version too: 46 combining marks assigned after 15.1 have a
  class in `unicode-normalization`'s tables and none in CPython's, so Rust can only merge two
  spellings Python keeps apart. No name on disk holds one.
- **Timestamps are pydantic's spelling, and the corpus sorts on CPython's other one.**
  `contracts::time::Timestamp` writes what `model_dump_json` writes: `Z` for UTC, and a fraction
  only when it is non-zero, as six digits. Its `isoformat()` is CPython's (`+00:00` for UTC), because
  `read_corpus` breaks a duplicate tie on that **string**, so 15:00+05:30 loses to 12:00Z, and a
  whole second beats `.500000` because `+` sorts before `.`. Equality and order are by instant;
  `date_ymd()` is `%Y-%m-%d` in the timestamp's own offset. pydantic also reads naive values, bare
  dates, Unix numbers and other lax spellings, and `Timestamp` refuses them all, so a file holding
  one reads as `None` in Rust (`crates/storage`'s crate doc names that divergence). Gated by
  `format/timestamp`. `Timestamp` replaced the lexeme type at `contracts::Timestamp`, and the engine
  and screen families came through byte for byte.
- **`statistics.median`** averages the middle two of an even count, and the mishit floor's
  comparison is a strict `<`.
- **`Path.suffix` is not `Path::extension`.** `content_filename` keeps an upload's suffix, and
  Python gives `clip.` and `.hidden` none where Rust answers `Some("")` for the first, so the port
  takes Python's rule.
- **Swing numbers are ASCII digits.** `_swing_sort_key` uses `str.isdigit()`, which is true for 798
  non-ASCII code points, and some of those make `int()` raise. The store names every swing
  `str(int)`, so only a hand-made directory reaches the difference, and `bundle_store.rs` names it.

## 4. The commands

**Since M32 the oracle is Rust**
([ADR-035 §3](decisions/035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)).
`golf-core rerecord` records the engine and stage families, every re-record gated against the
committed file, and `cargo test` is what certifies the vectors. Since M34 the verb also records the
screen family, and since M36 the storage and career families, each chosen by the declaration's
version key. `conformance.py` is the frozen half: `check` now certifies **the freeze, not the
vectors**, `regenerate` refuses both engine families and the screen, storage and career families,
and `check` retires with `analysis/` in M40. Why each of those moved is
[§M32](plans/m31-m40-shot-first-pivot.md#what-changes-in-python-and-why), and what building it
found is [the M32 plan's findings](plans/m32-shot-contract.md#phase-findings),
[the M34 plan's](plans/m34-screen-reader.md#phase-findings) and
[the M36 plan's](plans/m36-many-shot-layer.md#phase-findings).

```bash
cargo test                                          # certifies every family: audio against
                                                    # crates/trigger, every vector's shapes
                                                    # against crates/contracts, the format table
                                                    # against crates/pyfmt and crates/screen, all
                                                    # seven stages against crates/analysis, the
                                                    # screen family against crates/screen, the
                                                    # storage family against crates/storage, the
                                                    # career family and its report text against
                                                    # crates/analysis and crates/core, and
                                                    # the engine vectors end to end and the stage
                                                    # documents whole against crates/core
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/v17.json --dry-run
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/v17.json
                                                    # re-record the engine and stage families,
                                                    # gated; --dry-run reports and writes nothing
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/screen-v1.json --dry-run
                                                    # the same verb on the screen family, picked
                                                    # by the declaration's screen_parser_version
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/career-v1.json --dry-run
                                                    # the storage and career families together,
                                                    # picked by career_version (M36)
cargo run --bin golf-core -- run < vector.json      # the port's own answer, for the diff below
cargo run --bin golf-core -- parse-screen < spec/vectors/screen/corpus/2026-08-23-2.json
                                                    # the screen reader's answer: a screen vector
                                                    # (or its input) in, ShotData or null out
python scripts/conformance.py check                 # the freeze: frozen Python against every
                                                    # vector, outside the paths Rust re-recorded
python scripts/conformance.py check --id corpus/2026-08-09-2 -v
python scripts/conformance.py list                  # what is committed, and where it came from
python scripts/conformance.py run < vector.json     # frozen Python's answer, on stdout
python scripts/conformance.py regenerate --schemas-only  # the Python-owned schema roots only
python scripts/conformance.py regenerate --format-only   # the format table, from CPython itself
python scripts/conformance.py regenerate --screen-once   # recorded the screen family, once (M34);
                                                         # refuses now, exit 2
python scripts/conformance.py regenerate --storage-once  # recorded the storage family, once (M36);
                                                         # refuses now, exit 2
python scripts/conformance.py regenerate --career-once   # recorded the career family, once (M36);
                                                         # refuses now, exit 2
```

Every one of these needs the base install and `spec/` alone — that is what committing the vectors
buys — and none of them reads `data/`. The exceptions are the three record-once flags, and each
refuses before it reads anything. `--screen-once` needed the `ocr` extra and `data/` for the one run
it was allowed. `--storage-once` and `--career-once` read `data/` for their real cases.

### `rerecord`: how a vector changes now

**A change that moves an engine answer re-records in the same change**, and from M32 that change is
Rust's. `ANALYSIS_VERSION` is `crates/contracts/src/swing.rs`' constant, and frozen Python's stays
behind on purpose: `api/state.py::is_outdated` compares with `<`, so the frozen lab reads a newer
artifact as current. Never "fix" the gap by bumping Python. The steps:

**A change that moves a screen answer does the same, under its own version.** `SCREEN_PARSER_VERSION`
is `crates/contracts/src/shot.rs`' constant, and frozen Python's parser has no version at all: its
vectors say 0. The same verb re-records `spec/vectors/screen/` from a declaration named
`spec/declarations/screen-v<N>.json`. M34's `screen-v1.json` is the worked example.

**A change that moves a storage or career answer does the same, under a third version.**
`CAREER_VERSION` is `crates/contracts/src/career.rs`' constant, frozen Python's families say 0, and
one key covers both families, because a corpus the reader answers is the corpus the aggregates read.
The same verb re-records `spec/vectors/storage/` and `spec/vectors/career/` together from a
declaration named `spec/declarations/career-v<N>.json`. M36's `career-v1.json` is the worked example.

The steps:

1. Write the declaration, `spec/declarations/v<N>.json`, `screen-v<N>.json` or `career-v<N>.json`
   (§2). It holds the version, a note, the paths where the change adds keys, the paths whose values it
   moves, and, on a screen declaration only, the paths whose keys it removes.
2. Run `rerecord --dry-run` and read the report. It lists, per vector, every declared path that
   matched. An undeclared difference anywhere fails the run and names every path. An empty
   declaration's dry run is the quickest way to find what to declare, as long as every path it
   reports is then checked against what the change was meant to do.
3. Run it without `--dry-run`, then once more. **The second run must write nothing.**

The rules are `golf_core::rerecord`'s, unit-tested there and in `crates/core/tests/rerecord.rs`:

- **One family per run, picked by the version key.** A declaration at `analysis_version` walks the
  engine family and then the stage family. A declaration at `screen_parser_version` walks the screen
  documents: `corpus/`, `reference/`, `synthetic/` and `hand/`. `units/` is not read, because it
  holds case tables rather than documents (`SCREEN_UNREAD`). Any other entry under `vectors/screen/`
  is refused before a vector is read. A declaration at `career_version` walks `storage/` (`corpus/`,
  `bundle/`, `stores/` and `hand/`) and then `career/` (`synthetic/` and `real/`), storage first
  because the career family adopts its answers. A run never opens another family's files. There is no
  flag for the family, because a flag that disagreed with the declaration would be a second answer to
  one question.
- **The gate.** Each engine vector's committed input goes through `golf_core::run`, and each stage
  vector's (its engine vector's input, found by `provenance.derived_from`) through
  `stages::run_stages`. Each screen document's input goes through `rerecord::run_screen`, the
  counterpart of `conformance_vectors._run_screen`, which calls the shipping `screen::read`. The
  output is compared with the committed answer through `golf_core::compare`, under §3's rules. A
  difference passes only when the declaration names it: an added key at a declared `added` path, or
  a moved value at a declared `moved` path. Every stage document is also composed onto its engine
  vector's answer, as that answer will be written. Nothing composes onto a screen document, because
  nothing else holds a copy of a screen's answer. Each storage vector goes through
  `storage_family::run_storage`, and each career vector through `career_family::run_career`, which
  builds the four aggregates and renders the five scripts' reports. A script listed in
  `career_family::CARRIED` has its reports copied from the committed vector rather than run. It held
  all five until a renderer existed, and has been empty since M36 P16.
- **An adopted corpus is derived** (M36). A career vector whose `input.corpus` is a storage case's
  `expected.corpus` is run on that case's answer as it will be written, so a storage answer that moved
  moves its copy, and a copy that drifted from an unchanged source is a difference. Either is a
  difference at `input.corpus…` that the declaration must name. That copy is the one `input` a
  re-record can change.
- **A store's written file keeps its committed text where its value agrees.** `run_storage` answers
  each written JSON file in the recorded spelling wherever its value matches under the rules above,
  as an in-tolerance float keeps Python's bits. Without that, files differ from Python's text in
  spelling alone, and the gate would report them.
- **A path through a dotted key is refused by name.** A store's written file is keyed by its path
  (`…/manifest.json`), so a declared `expected.files.<path>` cannot be told from a walk through
  `manifest` and `json`. The run refuses it, naming the path, rather than writing the wrong place.
  Nothing in M36 moves a store's write. M35, the first change that does, builds the way to declare one.
- **A removed key passes only on a screen declaration, path by path.** In an engine declaration a
  removed key never passes, so a port cannot drop a key from the engine's answer and re-record its
  way past the drop. A screen declaration may carry `removed`, added by M34 P10 on the user's call.
  - **Why the screen needs it.** A tile the tie rule withholds has no `raw_fields` key, and that
    absence is the structural mark M35 and M37 grade `misread`. So the screen family's first
    re-record had to drop keys Python recorded.
  - **How it works.** `removed` works as `added` does. Each path is exact and ends in a key, is held
    to the typo guard, is written into the ledger, and is listed in the report. So a key still never
    goes *silently*.
  - **Engine declarations are untouched.** An engine declaration carrying the key at all is refused
    at load, and frozen Python's `frozen_view` and `ledger_covers` learn nothing. A career
    declaration is refused the same way (M36), so a career ledger entry has no `removed` list.
  - **What was rejected.** A `removed` list for both families would have taught frozen Python a case
    no engine change has needed. Keeping the old rule by giving a withheld field a `raw_fields` entry
    would have erased the mark M35 reads.
- **A list is declared at its own path when its length changes, and at each index when it does
  not.** `compare` reports a list of another length as one move of the whole list. So
  `expected.parsed.warnings` declared `moved` matches it, and the whole list is copied across. A list
  of the same length whose entries differ is a move per differing index (`…warnings[2]`), which the
  list's own path does not cover.
- **What is written is the committed document with only the declared paths replaced**, plus
  `provenance.oracle` and one new `rerecords` entry (§2). Every undeclared value keeps the bits
  Python recorded, even where Rust's lands inside the tolerance on other bits, so a port that
  drifts inside `RTOL` cannot launder the drift into the file. Matching is exact, at the
  difference's own path. A value that moved only inside the tolerance is no difference, so it
  matches nothing and the committed value stays.
- **The guards.** A declaration whose version is not its family's constant (`ANALYSIS_VERSION`,
  `SCREEN_PARSER_VERSION` or `CAREER_VERSION`) is refused before any vector is read, so a stale one
  cannot be re-run against a later engine, parser or corpus reader. A declared path that
  matches nothing in any vector fails the run as a typo, unless a committed ledger entry at the
  declaration's version already lists it, which is what lets the second run pass. A declaration
  under `spec/vectors/` is refused, and one outside the repo can drive only a run that writes
  nothing.
- **The run is atomic.** Every vector the run walks is run, gated and composed before a byte is
  written, so one refusal anywhere writes nothing anywhere. Changed files are staged beside their
  targets and renamed over them.
- **The file is written the way `conformance.py::_write_json` wrote it**: through
  `serde_json::Value` (so keys are sorted), indent 2, in the committed file's own line endings (the
  plain synthetic files are CRLF, and so is every screen, storage and career file), and the gzipped
  files (`corpus/`, and the real storage and career cases) with `GzipFile`'s header and mtime 0. The
  deflate stream is not zlib's byte for byte, and nothing reads it.

**The report is what gets reviewed, never `git diff`.** The text churns where nothing moved:
`serde_json` writes a decimal where Python wrote one of the seventy-seven exponent-form floats, and
writes `—` and `°` raw where Python escaped them. There are two worked examples:

- **M32's run** made 150 added keys and 63 moved values on the 42 files, every one of them
  declared, and a second run wrote nothing.
- **M34's run** declared 12 added paths, 19 moved and 5 removed, and wrote 35 screen files. Their
  values moved on three documents, and nothing differed on `hand/`. A second run wrote nothing.
- **M36's run** (`career-v1.json`) added nothing and moved 23 paths: `career_version` on every
  Python-recorded storage and career vector, the `OUTDATED` sentence in the four storage corpus
  cases that hold an outdated swing and in their narrowings, and the same sentence in each one's
  adopted career copy. Then M36 P15 rendered the `career_corpus` report, whose `--verbose` form prints
  that sentence, and the four copies' `expected.reports.career_corpus_verbose` differed. No rule had
  changed: the change had reached a report that was now run. So that path was added to
  `career-v1.json`, in the same uncommitted change, and the declaration was run again. Those four
  files carry two `career-v1` ledger entries. On the second run the typo guard passed the 23 earlier
  paths, which matched nothing any more, because the first entries already ledger them. P16 rendered
  the last two scripts and needed no declaration change. A second run writes nothing.

**`rerecord` cannot create a vector.** It re-records the ones that exist; a new engine vector and
its stages are §M29's Rust vector builder's. A new screen, storage or career case is a hand-worked
vector, written once with its working in its `note`, as M34's `screen/hand/` and M36's
`storage/hand/` were. So a missing `spec/vectors/` is a broken checkout, and
the Rust tests that find one say "restore it from git" rather than naming a command.

### `check`: the freeze held, not the vectors are right

**`check` runs frozen Python against every engine vector through `conformance.frozen_view`.** Every
path any `rerecords` entry lists is taken out of both the committed answer and frozen Python's, and
the rest is compared under `compare_results`. So a pass says frozen Python still reproduces every
value it recorded, which is ADR-035 §4's freeze, checked. It says nothing about whether the vectors
are right; that is `cargo test`'s to say. The line reads `21/21 vectors hold the freeze (frozen
engine v16; 21 re-recorded by Rust)`, and the re-recorded count is the quick check that every
engine vector took a ledger. A vector with no ledger is compared as it stands, which is all `check`
did before M32.

**The version rule is `ledger_covers`, not equality.** A vector at frozen Python's version passes
it. One above it passes only with a `rerecords` entry for each version in between, because those
entries are what say which values the later versions moved. One below it, or above it without the
entries, is STALE. Exit 0 is the freeze holding; 1 is a recorded value frozen Python no longer
reproduces, or a stale vector; 2 is no such vector, or an empty `spec/vectors/`.

**`check` defers the stages to `cargo test` and still reads their version**, under the same rule.
The implementation under test is Rust, so running them here would prove nothing about the port. But
they carry `ANALYSIS_VERSION`, and a stage vector from a retracted engine certifies a port mid-build
against retracted answers, four phases sooner than a stale bundle vector would. The ratio printed at
the end stays the **engine's**, and the stage line is separate, because one number meaning two
things is how a green run gets quoted for something it did not check.

**`check` does not run the audio family and says so**, because this program is not its
implementation: ADR-030 §1 gives strike detection to Rust and M20 deleted the Python detector, so
`cargo test` runs those thirty vectors and `check` reports them as deferred. `conformance.py`
splits the two with `engine_vector_paths()` and `audio_vector_paths()`; `vector_paths()` still
means *everything committed*, because that discovery is what stops a vector existing on disk and
in no index.

**Nor does it run the screen family, and it reads no version there** (M34). `crates/screen` is the
implementation, and since the re-record it is the oracle too, so `check` counts the family through
`screen_vector_paths()` and defers it. It applies no `ANALYSIS_VERSION` staleness test, because the
family ages on `SCREEN_PARSER_VERSION`, which frozen Python does not have. `list` shows the screen
documents at `v1` and `units/` at `v0`.

**Nor the storage and career families** (M36), on the screen family's rule. `check` counts them
through `storage_vector_paths()` and `career_vector_paths()`, defers both to `cargo test`, and applies
no `ANALYSIS_VERSION` test, because both age on `CAREER_VERSION`. `list` shows each vector's own
`career_version`.

### `regenerate` refuses the engine, stage, screen, storage and career families

**The full `regenerate` and `--stages-only` refuse with exit 2, before anything is imported or
written**, schemas included. A rebuild from frozen Python would write its v16 answers over the
Rust-recorded vectors and drop the ledger that says which of their values are Rust's. The refusal
names `golf-core rerecord` and the command line to run instead.

**It follows the audio precedent, which has stood since M20.** Those vectors were recorded by
`audio/impact.py`, which no longer exists, and rebuilding them from the Rust detector would replace
a reference-derived oracle with a self-portrait that passes by construction. If
`AUDIO_DETECTOR_VERSION` ever moves, re-recording is a decision that needs a new oracle named first
— see `conformance_vectors._audio`, which returns nothing and explains why. What keeps
`golf-core rerecord` from being that self-portrait is the gate above: it can change only what a
declaration names.

**`--screen-once` recorded the screen family, and refuses whenever any of it exists** (M34 P4).

- **What it is.** It is the one vector-writing change ADR-035 clause 4 allows the frozen lab, and
  the family's whole Python record: one run of PaddleOCR over the stored photos.
- **What it checked before writing.** Every corpus shot was checked against the stored shot before
  anything was written.
- **Why it can only run once.** It refuses with exit 2, naming `golf-core rerecord`, on **any file**
  under `spec/vectors/screen/`, before the recorder is even imported. So the command named in the
  family's provenance cannot run a second time and overwrite Rust's ledger with frozen Python's
  answers.
- **What stays runnable.** `conformance_vectors.build_screen` stays, on `_audio`'s precedent, as the
  record of how the family was made.

**`--storage-once` and `--career-once` recorded the many-shot layer's families, and refuse the same
way** (M36 P2 and P3). Each was one run of frozen Python's stores, corpus reader, aggregates and
script reports, through the one sanctioned recorder change (ADR-035 clause 4). Each refuses with exit
2 on any file under its family's directory, before the recorder is imported, and names
`golf-core rerecord`. `build_storage` and `build_career` stay as the record, and their `_run_corpus`,
`_run_ops` and `_run_career` are the definitions the Rust runners reproduce.

`regenerate` still does two jobs. `--schemas-only` writes the Python-owned schema roots and never
the three in `RUST_OWNED_SCHEMAS` (§1), and `--format-only` rebuilds the format family from CPython
(§2). `scripts/conformance_vectors.py`'s module doc and `build_stages_from_disk`'s docstring still name
`regenerate` and `--stages-only` as the way in. They are frozen Python, so they stay wrong until
M40 deletes them, and this paragraph is the correction.

### `run`: the cross-language seam

`run` is diffed by a shell pipeline, with no Python in the loop but the reference. **Both ends of it
exist as of M22 P6, and they agreed on all 21 as of P8b**: `golf-core run` is `crates/core`'s
binary, and the two were compared through `compare_results` itself, with no Rust comparator
involved, at zero differences on every vector. Since M32 the two ends answer at different versions
on purpose, and differ by exactly the paths the ledger declares.

**The `diff` is a diagnostic and not the gate**, and the difference is worth keeping straight. A
byte comparison of the two streams is unavailable in either direction: they are 13,569 and 13,221
bytes for one vector, because Python writes `-1.636758133827243e-05` where `serde_json` writes
`-0.00001636758133827243`. So the pipeline shows *where* two implementations disagree once
something has already decided they do, and the deciding is a comparator's: `compare_results` on the
Python side, `golf_core::compare` on the Rust one (§3).

**`cargo test` also reads every engine vector without running one** (M22 P2).
`crates/contracts/tests/round_trip.rs` deserializes each vector's `input` and `expected` into the
ported shapes and writes them back, asking only whether a field survives the crossing. It is
*stricter* than §3 and deliberately so: nothing there computes anything, so a float must come back
with the same bits and a key must come back at all, with the one ledgered allowance for an
`input.shot` key that §2 describes. What it cannot ask for is a byte comparison —
Python writes `-1.636758133827243e-05` where `serde_json` writes `-0.00001636758133827243` for the
identical f64, and seventy-seven distinct floats in the committed vectors take the exponent form. So
every cross-language comparison here is structural, over parsed values, which is what
`compare_results` already does and is why `run`'s stdout seam needs no formatter.

### `parse-screen`: the screen reader's seam, with no Python end

`golf-core parse-screen` is `run`'s shape for `crates/screen` (M34 P6). It reads a screen vector, or
just its `input`, on stdin.

- **What it writes.** On stdout it writes the `ShotData` that `screen::read` makes. It writes `null`
  where the read failed, as frozen Python's `import_screen` returned `failed`, and exits 0, because a
  failed read is an answer.
- **When it fails.** A device no profile names is the caller's mistake: it exits 1, with nothing on
  stdout.
- **Nothing to diff it against.** It has no `conformance.py` counterpart, and none is owed. The
  screen family was recorded once, and it now belongs to `golf-core rerecord`.

So the verb is for a person to see what the reader makes of a vector. `cargo test` already pipes
every screen document through it, and each answer equals the library's with no tolerance.

## 5. What ships in the app, and what stays in the lab

The inventory ADR-030 implies, made explicit — the answer to "does this module get rewritten?"
[ADR-034](decisions/034-shot-first-phone-first.md) (2026-09-29, M31) makes the phone the first
host, and **the phone runs no Python**. That moved three rows: the screen parser now ports, tier 3
is laptop-only, and tier 4's retirement waits on M40.
[ADR-035](decisions/035-rust-everywhere-python-where-required.md) (2026-09-30, M31.5) moved the
rest: **Python stays only where it is required**, for MediaPipe pose and the LLM
([§1](decisions/035-rust-everywhere-python-where-required.md#1-the-rule-and-the-two-exceptions-it-names)
names every file), and everything else ports, the lab included. So tier 3 is §1's list, tier 4 is
[§M29](../ROADMAP.md#m29-the-lab-port--a-rust-lab-cli-the-rmcp-server-and-the-archive-move)'s port
rather than a lab that stays, and retirement happens twice: §M29 deletes what the frozen FastAPI
server does not import, and M40 deletes the rest
([§5](decisions/035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped)). The
rows below say so where they changed, and
[the M31.5 plan's inventory](plans/m31-5-rust-first-replan.md#the-inventory) gives each module its
milestone.

| Tier | Modules | Disposition |
|---|---|---|
| **1 — ported, M22** | `contracts/`, `analysis/`, `feedback/rules.py` | **Done** — `crates/{contracts,analysis,feedback,core}`, all 21 engine vectors conforming through `cargo test` (M22 P2–P8b). **Nothing was deleted**, unlike M20: ADR-032 §7 adds a third clause to the retirement rule — *and nothing that stays Python calls it* — and `analysis/` has 28 callers in the lab, so both implementations stand until M40: [§M29](../ROADMAP.md#m29-the-lab-port--a-rust-lab-cli-the-rmcp-server-and-the-archive-move) ports the lab off it, and the frozen FastAPI server imports it until M40 deletes both (ADR-035 §5). Two parts stayed behind on purpose and are named in ADR-032 §8 and its P7 addendum: `alignment.py`'s render half (220 lines a side-by-side video is drawn from) and 130 lines of `flight_measure.py` that only a script and a page call. No committed vector reaches either, and ADR-035 §2 deletes both rather than porting them. **M36 ported the career half** that M22's criterion did not reach: `analysis/{baseline,dispersion,comparison,club_profile}.py`, `stats.py`'s interval helpers, and the many-shot contracts they read, gated by §2's career family |
| **1 — the rest of the swing loop** | `storage/`, `capture/` | **`storage/` ports to Rust** (ADR-030 §1). **M36 took the stores the many-shot layer reads**: `crates/storage` holds the manifest, the bundle, bag, golfer and shot stores, the corpus reader, and `api/state.py`'s tolerant readers, gated by §2's storage family. So ADR-008's one Python exception, `storage/corpus.py` importing *upward* into `api.state`, has no Rust twin. §M29 takes the rest, except `transcript_store.py`, which stays with the LLM (ADR-035 §1). **`capture/` stays Python**, because the pose worker decodes its frames through `FileVideoSource` (ADR-035 §1; this row said "ports" until M31.5 P1 found the worker's import), and no committed vector crosses it. `crates/capture` (M21) is the *camera edge* rather than a port of this `capture/` |
| **1 — ported** | ~~`audio/impact.py`~~ → `crates/trigger` | **Done, M20.** The Python original is **deleted**, per ADR-030's 2026-09-22 addendum: the vectors are the oracle, not the code that recorded them. `rustfft` is the one numeric library the port needed rather than arithmetic |
| **1 — not yet** | `audio/ffmpeg.py` | **Ports to Rust in §M29**, whose lab CLI runs ffmpeg as a subprocess (ADR-035 §5). Listed here because it was tier 1 by implication and in no table until M20 went looking: it holds the container edit lists, the two `soun` tracks the face-on clips carry and the `video_start_seconds` probe, and it reaches ffmpeg through the `imageio-ffmpeg` wheel rather than a system install. Until it moves, **Python decodes and Rust detects** |
| **2 — shots and clubs** | `launch_monitor/{mock,composite,source}.py`, `clubs/catalogue.py` | **`clubs/catalogue.py` is done, M36**, with the bag (the M31.5 plan's R25): `crates/contracts::catalogue` reads the committed rows by `include_str!` and `crates/storage::catalogue` keys them, while `remember` stays Python's beside `clubs/lookup.py` (§1). M36 also took the shot store (`launch_monitor/screen/store.py`) into `crates/storage`. **`launch_monitor/{mock,composite,source}.py` port in §M29** (R16, whose shot store was M36's). Committed JSON plus stdlib arithmetic already (ADR-022), so the data crosses unchanged. `analysis/{flight,flight_infer,flight_measure,spin_solve,shot_measure}.py` were listed here and are **done** — they are inside `run_vector`'s reach, so M22 P8 and P8b took them with tier 1 rather than after it |
| **2 — the screen reader** | `launch_monitor/screen/{parser,validate,profiles}.py` and `profiles.json`, with `recognizer.py`'s `TextBox` | **Done, M34** (ADR-034 clause 7) — `crates/screen`, with `crates/pyfmt` split out of `analysis` so that it could say what CPython says without reaching the engine. §2's screen family was recorded once from the frozen Python, ported faithfully, then changed in Rust alone: the tie rule, the `Impact Position V` tile in a **forked** `profiles.json`, `fields_present` and the `parser_version` stamp. Rust re-recorded the family after that (ADR-035 §3), and the faithful port was deleted. `screen::read` is the entry point, wired to nothing yet: its first callers are M38's phone and §M29's lab, and `golf-core parse-screen` is its seam. **Nothing Python was deleted**, by ADR-032 §7's rule. The frozen parser and its `profiles.json` stay, because the lab's importer and the frozen FastAPI server's upload path call them, until M40. Until §M29 switches the lab, the two read `2026-08-10-1` and `2026-08-23-1` differently, and the difference is declared. Moved out of tier 4 on 2026-09-29: the phone reads its own photos, so the parser has to run there. `TextBox` crosses because it is the seam, the one shape both recognizers produce. The recognizer and the preprocessing (`paddle.py`, `preprocess.py`, `importer.py`) were to **stay the lab's reader**, with PaddleOCR and OpenCV. ADR-035 §2 ports them too, in §M29, through `ort` running the same Paddle models and gated on the 13 stored bay photos. On the phone, Apple Vision stands where they do. The parser's portability edges are a list of their own, §3's second list, and do not belong to the engine's |
| **3 — the Python sidecar** | `pose/estimator.py`, `pose/worker.py`, `capture/`; `feedback/coach.py`, `feedback/conversation.py`, `storage/transcript_store.py`, `clubs/lookup.py`, and the shapes they own — [ADR-035 §1](decisions/035-rust-everywhere-python-where-required.md#1-the-rule-and-the-two-exceptions-it-names) names every file | **Stays Python, bundled — and laptop-only** (ADR-030 §2, §3; ADR-034 clauses 8 and 10; ADR-035 §1). This is *all* that stays: the two exceptions, each with its import closure — `capture/` because the pose worker decodes through it, and `clubs/lookup.py` because it is an LLM call. **The phone has no Python**: pose there is an in-process iOS `PoseLandmarker` behind M39 P0's gate, and it never reaches this tier. Bundling, which was M26's, is now M40's. Pose is the load-bearing one: `ranges.json` is cut from these landmarks, and MediaPipe is a graph with no mature Rust binding. `pose/worker.py` (M23, ADR-033) is the process `crates/pose` spawns — the sidecar's own entry point, reusing `estimate_pose` unchanged, which is what makes this tier a *process* boundary rather than a library one. The LLM call is the other. `coach.py` sits here rather than in tier 4 because it is bundled with the laptop client rather than lab-only, and it **never reaches the phone**: there is no LLM there (ADR-034 clause 10) |
| **4 — the lab** | `mcp/`, `api/`, the rest of `launch_monitor/screen/`, `pose/{overlay,side_by_side}.py`, `scripts/` | **Not shipped, and ported rather than kept.** [§M29](../ROADMAP.md#m29-the-lab-port--a-rust-lab-cli-the-rmcp-server-and-the-archive-move) is the lab port since [ADR-035 §5](decisions/035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped), after M34 and M36 and no longer blocked on M40: a Rust lab CLI takes over `api/pipeline.py`'s job and the lab scripts, `mcp/` ports on `rmcp`, the screen's recognizer ports through `ort`, and the research scripts move to `archive/`. The overlay tools are deleted rather than ported (ADR-035 §2). **§M29 deletes only what the frozen FastAPI server does not import**; M40 decides `api/` — ported to `axum` or dropped — and deletes the rest, which is what makes tier 1's delete possible at all (ADR-032 §7's rule). Superseded by ADR-035: `api/` retiring into the Flutter shell, the screen's reader staying PaddleOCR, `clubs/lookup.py` as open (it is tier 3), and the question of how ADR-022's fitting reaches a measurement, which is archived rather than run |
| **stub** | `detection/` | Gated on M1.5's no-go. Nothing to port, and §M29 deletes it (ADR-035 §5) |

`feedback/rules.py` is tier 1 and is the reason `run_vector` makes **two** calls rather than one.
`analysis` may not import `feedback` (ADR-008), so `analyze_swing_bundle` leaves
`SwingBundleResult.feedback` as None and `api/pipeline.py:1259` fills it in a line later — meaning
the artifact this repo writes carries ranked tips and a bare engine call does not. Building the
vectors off the engine alone pinned `"feedback": null` on all twenty-one and would have told a port
to ship a results page with no coaching on it. They now carry the real payload, and
`_verify_against_stored` checks it against the archive like everything else.

**That two-call shape is why the port has four crates and not ADR-032 §1's two** (M22 P6). There are
ten in the workspace. M20's `trigger`, M21's `capture` and M23's `pose` are edges rather than
ports. M34 added `pyfmt`, the CPython edges below every crate that needs them, and `screen`, the
tier-2 port. M36 added `storage`, which depends on `contracts` and `pyfmt` and never on `analysis`,
so `crates/core` is the one crate holding the stores, the aggregates and the reports together, and
the career verbs live there.
The rule
is enforced by cargo: `crates/feedback` holds `rules.rs` and depends on `contracts` alone, so it
*cannot* reach `analysis` and `analysis` cannot reach it. Something above both has to make the two
calls, and `crates/core` is it — the counterpart of `api/pipeline.py`, whose job §M29's Rust lab
CLI takes over rather than porting it (ADR-035 §5). See ADR-032's 2026-09-25 addendum for the three cheaper layouts
that were declined, each of which bought a smaller crate count by spending that edge.

`config.py::REPO_ROOT` assumes a source checkout and dies at packaging, which is tier 1's one known
unported assumption; it is in ADR-030's carried open questions.

---

## What this does not cover

Named rather than left to be discovered:

- **`feedback/coach.py`.** The *ranking* is covered; the LLM call is not and will not be — it is
  tier 3 and laptop-only (§5; this bullet said tier 4 until 2026-09-29), it is non-deterministic,
  and `FeedbackPayload.coaching` is `None` on every vector.
- ~~**`audio/impact.py` end to end.**~~ **Closed by M20.** `spec/vectors/audio/` covers it: 30
  vectors, one per stored clip, 4.3 MB. The waveforms were never the obstacle they looked like —
  see §2's audio family for what is shipped instead of them, and why windowing audio is lossy
  where windowing keypoints was not. What remains uncovered is `audio/ffmpeg.py`: the vectors
  begin at a decoded waveform, so a port's *decoder* — edit lists, stream selection, the video
  timebase — is judged by nothing here.
- **Pose itself.** Deliberately, and by a different oracle: ADR-030 §3 pins the sidecar against the
  30 keypoint files already on disk, because its true input is a 4K `.MOV` that cannot be committed
  and a keypoints-only family would be 239 MB before gzip
  ([ADR-033](decisions/033-the-pose-sidecar-protocol.md)'s Consequences). **That comparison has now
  been run** (M23 P7): `scripts/pose_replay.py` re-posed all 30 clips through `golf-pose` and found
  **0 differing values of 5,757,660** over 42,648 frames, key sets included. What it does not reach
  is listed in ADR-033's fourth addendum — no left-handed clip and no live capture beside the pool
  being the two that matter.
- **Anything `EXCLUDED_FROM_RESULT` drops, which is wider than it looks** — found by M22 P6's
  mutation sweep. §4's exclusion is right and stays: round-tripping keypoints would make every
  vector 30x larger and check nothing. But it means the suite cannot see `swing.keypoints` **at
  all**, and one of `analyze_swing_bundle`'s own promises lives there. That function guarantees
  everything comes back in whole-clip coordinates — the face-on phases shifted back by the window
  offset *and* the sliced frames re-attached, so `swing.phases[i].start_frame` is a valid index into
  `swing.keypoints`. `synthetic/windowed.json` gates the shift; **deleting the re-attachment passes
  all 21 vectors in either language.** The same blind spot covers `SwingAnchors` entirely on a
  one-camera bundle, where `alignment` serializes as `null`: inverting `motion_start_detected` —
  an ADR-013 disclosure — is invisible to every committed answer. Both have Rust unit tests standing
  where the vectors cannot; neither has a Python one.
- **Two of `engine.py`'s seven notes, and not the ones you would guess.** A sentence in `notes` is
  compared exactly when it fires, and M22 P6 measured which do. `_tempo_notes`' implausible-ratio
  sentence fires on **nothing in `spec/`, either half** — every collapsed motion-start boundary in
  the corpus is in the *down-the-line* view and that function reads the face-on anchors alone, so
  `aaron-1` (`corpus/2026-08-07-aaron1-1`), the swing it was written for, does not produce it. Nor
  does the "could not be segmented into usable anchors" note. `_anchored_on_strike`'s sentence, four
  lines away, *is* covered — fourteen corpus vectors carry one.
- **A left-handed shot.** Named last because it is the one gap the *outcome* half has, and it
  survived the whole port. The corpus's only left-handed vector is synthetic and carries no shot,
  so `flight_infer.infer_spin_axis`' mirror — the flip ADR-014's addendum had backwards for a
  milestone — reaches no committed answer and ships against the port's unit tests. Its mechanics
  counterpart is narrower and is in §2: the same vector *does* gate the trajectory mirror and
  *cannot* gate the checkpoint one.
- **`engine._tempo_notes`' two sentences, and a flight that is both solved and curved.** The first
  is in the notes bullet above. The second is the only combination in which all six `flight_*`
  measurements record at once, and this repo's data has never produced one: every solved shot here
  is planar and every curved one printed its own spin. One more the *stage* family cannot hold at
  all — `InferredSpinAxis.sign_disagrees` is a method rather than a field, so `run_stages` records
  neither it nor anything computed from it, even though `2026-08-23-11` sets it by deriving a draw
  against a `FADE` on the screen.
- **The screen's recognizer and its preprocessing.** A screen vector begins at boxes. So
  PaddleOCR, the OpenCV rectification, and the rotation and legibility notes they write are judged by
  nothing here, and neither is Apple Vision. M33 measures Vision against the stored shots, and §M29's
  Rust reader is gated on the 13 bay photos against PaddleOCR's parse. The notes reach a vector only
  as recorded input, and on the stored photos that is just the uncropped-frame note.
- **The many-shot layer off this machine.** The bit pins on the career aggregates, `stats::pow` and
  the pooled spread run on Windows with MSVC only, because another libm may answer a last bit
  differently and CPython over it would too. The storage family takes the POSIX answer on case and
  line endings (`crates/storage`'s crate doc), so a case-insensitive filesystem's behaviour, which
  is this box's, is reached by no case.
- **Wall-clock performance.** No vector is timed. The charter's <15 s target remains unmeasured,
  as `ARCHITECTURE.md` §5 says.
