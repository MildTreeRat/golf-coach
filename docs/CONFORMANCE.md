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
`joint_model_v1.json`, `flight_model_v1.json`, `club_catalogue.json`, and OCR's `profiles.json`,
which stays Python). Discovery rather than a listing, so a *new* artifact fails here instead of
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
derive cannot see them, and restating each as an attribute is a second copy that drifts. M36,
which takes the storage roots, is where that is weighed again.

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
pinned twice, so when M36 takes more roots over, both change together.

`crates/contracts/devices.json` (M32, the device capability model) is committed data a Rust crate
reads by `include_str!`, not an artifact opened off disk, so it is no schema root. It is not in the
package-data pin either, which scrapes `src/golf_coach/` and is Python's.

## 2. The vectors

Five families, covering different things. The first two are the **engine** families: `cargo test`
certifies them end to end against `crates/core`, `golf-core rerecord` records them from M32, and
`conformance.py check` runs frozen Python against them to certify the freeze (§4). The other three
name a Rust crate as their implementation and are deferred to `cargo test`.

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
two keys onto all forty-two (21 engine, 21 stage; counted 2026-10-01). `audio` and `format` carry
neither yet; M37's pin, that every family names its oracle, extends `oracle` to them
([ADR-032](decisions/032-the-rust-core.md)'s 2026-09-29 addendum).

- **`provenance.oracle` is `"python"`**, and stays so after a Rust re-record, because every value
  the re-record did not declare is still the one Python recorded, bit for bit. M32 P8 checked that
  in a second language across all forty-two files, comparing every float by `float.hex()` once the
  declared paths were taken out: zero differences.
- **`provenance.rerecords` is a list with one entry appended per re-record**, and it is what says
  which values are Rust's. An entry holds `analysis_version` (the version the re-record moved the
  file to), `by` (`"golf-core rerecord"`), `declaration` (the declaration's repo-relative path, with
  `/`), and `added` and `moved`: the declared paths **that matched in that file**, in the
  declaration's order. So M32's entry on a synthetic engine vector moves `analysis_version` and
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
version it records (`v17.json` is M32's). Each has four required keys — `analysis_version`, a
`note`, `added` and `moved` — and loads or is refused whole: an unknown key, a path declared twice,
or an `added` path ending in an index is refused before any vector is read. They sit outside
`spec/vectors/` on purpose, because `conformance.py::vector_paths` would read a file there as a
vector. The declaration is what a reviewer reads, beside the verb's report (§4), and it is
load-bearing twice: the verb reads it, and `crates/contracts/tests/schemas.rs` reads `v17.json`'s
`added` to know which schema properties must carry a description (§1).

**What reads the ledger.** On the Rust side, nothing but the round-trip allowance above and the
re-record's own second-run rule (§4): the end-to-end gate and the stage tests compare the whole
file, Rust's values and Python's alike, with `analysis_version` equal to `ANALYSIS_VERSION`. On the
Python side, `conformance.frozen_view` takes every ledgered path out of both `expected` and frozen
Python's answer before comparing, and `ledger_covers` accepts a vector above frozen Python's version
only when it carries an entry for every version in between (§4).

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

### Format (`spec/vectors/format/`) — 5 vectors, 3,059 cases, run by `cargo test`

Added by M22 P3 and extended by P5, and the only family whose subject is **not this engine**. It
records what CPython's `round()`, `format()`, `str()` and `sorted()` do, because §3's edges below
are where a Rust port diverges from Python on the *same* arithmetic — and every one of them lands
on the strings §3 compares exactly. The implementation under test is `crates/analysis/src/pyfmt.rs`, ported before anything
calls it, so the edges are solved once rather than at each of the seven call sites downstream.

| vector | what it records | the edge |
|---|---|---|
| `rounding` | `round(x)` and `round(x, n)` | 1 — half-to-even, and decimal-aware at `n` places |
| `fixed` | `f"{x:.Nf}"` at the four precisions the engine interpolates | 2 |
| `general` | `f"{x:g}"` at CPython's default precision of 6 | 2 |
| `repr` | `str(x)` — an f-string with no format spec at all | 4 |
| `ordering` | a stable sort over an insertion-ordered dict, and `engine.py`'s registry rank | 3 |

**It does not age on `ANALYSIS_VERSION`, and it is the only family that does not.** A rounding rule
is the language's, not the engine's, so a version bump leaves every answer in it true; what it
carries instead is `python_version`. `check` therefore reports these without a staleness test, where
it runs one over the stages — pinned in `tests/test_conformance.py`, because the *absence* of the
field is the decision and it reads as an oversight otherwise.

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
engine's oracle to Rust left it where it was.

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
`crates/analysis/src/pyfmt.rs`, rather than where each one bites.

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

## 4. The commands

**Since M32 the oracle is Rust**
([ADR-035 §3](decisions/035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)).
`golf-core rerecord` records the engine and stage families, every re-record gated against the
committed file, and `cargo test` is what certifies the vectors. `conformance.py` is the frozen
half: `check` now certifies **the freeze, not the vectors**, `regenerate` refuses both families,
and `check` retires with `analysis/` in M40. Why each of those moved is
[§M32](plans/m31-m40-shot-first-pivot.md#what-changes-in-python-and-why), and what building it
found is [the M32 plan's findings](plans/m32-shot-contract.md#phase-findings).

```bash
cargo test                                          # certifies every family: audio against
                                                    # crates/trigger, every vector's shapes
                                                    # against crates/contracts, the format table
                                                    # and all seven stages against
                                                    # crates/analysis, and the engine vectors end
                                                    # to end and the stage documents whole
                                                    # against crates/core
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/v17.json --dry-run
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/v17.json
                                                    # re-record the engine and stage families,
                                                    # gated; --dry-run reports and writes nothing
cargo run --bin golf-core -- run < vector.json      # the port's own answer, for the diff below
python scripts/conformance.py check                 # the freeze: frozen Python against every
                                                    # vector, outside the paths Rust re-recorded
python scripts/conformance.py check --id corpus/2026-08-09-2 -v
python scripts/conformance.py list                  # what is committed, and where it came from
python scripts/conformance.py run < vector.json     # frozen Python's answer, on stdout
python scripts/conformance.py regenerate --schemas-only  # the Python-owned schema roots only
python scripts/conformance.py regenerate --format-only   # the format table, from CPython itself
```

Every one of these needs the base install and `spec/` alone — that is what committing the vectors
buys — and none of them reads `data/`.

### `rerecord`: how a vector changes now

**A change that moves an engine answer re-records in the same change**, and from M32 that change is
Rust's. `ANALYSIS_VERSION` is `crates/contracts/src/swing.rs`' constant, and frozen Python's stays
behind on purpose: `api/state.py::is_outdated` compares with `<`, so the frozen lab reads a newer
artifact as current. Never "fix" the gap by bumping Python. The steps:

1. Write the declaration, `spec/declarations/v<N>.json` (§2): the version, a note, the paths where
   the change adds keys, and the paths whose values it moves.
2. Run `rerecord --dry-run` and read the report. It lists, per vector, every declared path that
   matched. An undeclared difference anywhere fails the run and names every path.
3. Run it without `--dry-run`, then once more. **The second run must write nothing.**

The rules are `golf_core::rerecord`'s, unit-tested there and in `crates/core/tests/rerecord.rs`:

- **The gate.** Each engine vector's committed input goes through `golf_core::run`, and each stage
  vector's (its engine vector's input, found by `provenance.derived_from`) through
  `stages::run_stages`. The output is compared with the committed answer through
  `golf_core::compare`, under §3's rules. A difference passes only when the declaration names it:
  an added key at a declared `added` path, or a moved value at a declared `moved` path. A removed
  key never passes. Every stage document is also composed onto its engine vector's answer, as that
  answer will be written.
- **What is written is the committed document with only the declared paths replaced**, plus
  `provenance.oracle` and one new `rerecords` entry (§2). Every undeclared value keeps the bits
  Python recorded, even where Rust's lands inside the tolerance on other bits, so a port that
  drifts inside `RTOL` cannot launder the drift into the file. Matching is exact, at the
  difference's own path. A value that moved only inside the tolerance is no difference, so it
  matches nothing and the committed value stays.
- **The guards.** A declaration whose `analysis_version` is not `ANALYSIS_VERSION` is refused before
  any vector is read, so a stale one cannot be re-run against a later engine. A declared path that
  matches nothing in any vector fails the run as a typo, unless a committed ledger entry at the
  declaration's version already lists it, which is what lets the second run pass. A declaration
  under `spec/vectors/` is refused, and one outside the repo can drive only a run that writes
  nothing.
- **The run is atomic.** Every vector in both families is run, gated and composed before a byte is
  written, so one refusal anywhere writes nothing anywhere. Changed files are staged beside their
  targets and renamed over them.
- **The file is written the way `conformance.py::_write_json` wrote it**: through
  `serde_json::Value` (so keys are sorted), indent 2, in the committed file's own line endings (the
  plain synthetic files are CRLF), and `corpus/`'s gzip with `GzipFile`'s header and mtime 0. The
  deflate stream is not zlib's byte for byte, and nothing reads it.

**The report is what gets reviewed, never `git diff`.** The text churns where nothing moved:
`serde_json` writes a decimal where Python wrote one of the seventy-seven exponent-form floats, and
writes `—` raw where Python escaped it. M32's own run is the worked example: 150 added keys and 63
moved values on the 42 files, every one of them declared, and a second run that wrote nothing.

**`rerecord` cannot create a vector.** It re-records the ones that exist; a new engine vector and
its stages are §M29's Rust vector builder's. So a missing `spec/vectors/` is a broken checkout, and
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

### `regenerate` refuses the engine and stage families

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
| **1 — ported, M22** | `contracts/`, `analysis/`, `feedback/rules.py` | **Done** — `crates/{contracts,analysis,feedback,core}`, all 21 engine vectors conforming through `cargo test` (M22 P2–P8b). **Nothing was deleted**, unlike M20: ADR-032 §7 adds a third clause to the retirement rule — *and nothing that stays Python calls it* — and `analysis/` has 28 callers in the lab, so both implementations stand until M40: [§M29](../ROADMAP.md#m29-the-lab-port--a-rust-lab-cli-the-rmcp-server-and-the-archive-move) ports the lab off it, and the frozen FastAPI server imports it until M40 deletes both (ADR-035 §5). Two parts stayed behind on purpose and are named in ADR-032 §8 and its P7 addendum: `alignment.py`'s render half (220 lines a side-by-side video is drawn from) and 130 lines of `flight_measure.py` that only a script and a page call. No committed vector reaches either, and ADR-035 §2 deletes both rather than porting them |
| **1 — the rest of the swing loop** | `storage/`, `capture/` | **`storage/` ports to Rust** (ADR-030 §1): M36 takes the stores the many-shot layer reads and §M29 the rest, except `transcript_store.py`, which stays with the LLM (ADR-035 §1). **`capture/` stays Python**, because the pose worker decodes its frames through `FileVideoSource` (ADR-035 §1; this row said "ports" until M31.5 P1 found the worker's import). Both are outside M22's criterion, because no committed vector crosses either. `crates/capture` (M21) is the *camera edge* rather than a port of this `capture/` |
| **1 — ported** | ~~`audio/impact.py`~~ → `crates/trigger` | **Done, M20.** The Python original is **deleted**, per ADR-030's 2026-09-22 addendum: the vectors are the oracle, not the code that recorded them. `rustfft` is the one numeric library the port needed rather than arithmetic |
| **1 — not yet** | `audio/ffmpeg.py` | **Ports to Rust in §M29**, whose lab CLI runs ffmpeg as a subprocess (ADR-035 §5). Listed here because it was tier 1 by implication and in no table until M20 went looking: it holds the container edit lists, the two `soun` tracks the face-on clips carry and the `video_start_seconds` probe, and it reaches ffmpeg through the `imageio-ffmpeg` wheel rather than a system install. Until it moves, **Python decodes and Rust detects** |
| **2 — shots and clubs** | `launch_monitor/{mock,composite,source}.py`, `clubs/catalogue.py` | **Ports to Rust in M36**, with the many-shot layer, and §M29 takes what M36 leaves (the M31.5 plan's R16 and R25). Committed JSON plus stdlib arithmetic already (ADR-022), so the data crosses unchanged. `analysis/{flight,flight_infer,flight_measure,spin_solve,shot_measure}.py` were listed here and are **done** — they are inside `run_vector`'s reach, so M22 P8 and P8b took them with tier 1 rather than after it |
| **2 — the screen reader** | `launch_monitor/screen/{parser,validate,profiles}.py` and `profiles.json`, with `recognizer.py`'s `TextBox` | **Ports to Rust, M34** (ADR-034 clause 7), as `crates/screen`, recorded once from the frozen Python and then by Rust (ADR-035 §3), and gated by a screen vector family like every other port. Moved out of tier 4 on 2026-09-29: the phone reads its own photos, so the parser has to run there. `TextBox` crosses because it is the seam, the one shape both recognizers produce. The recognizer and the preprocessing (`paddle.py`, `preprocess.py`, `importer.py`) were to **stay the lab's reader**, with PaddleOCR and OpenCV. ADR-035 §2 ports them too, in §M29, through `ort` running the same Paddle models and gated on the 13 stored bay photos. On the phone, Apple Vision stands where they do. The parser has portability edges of its own, and they are M34's list rather than §3's ([ADR-032](decisions/032-the-rust-core.md)'s 2026-09-29 addendum) |
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

**That two-call shape is why the port has four crates and not ADR-032 §1's two** (M22 P6) — seven
in the workspace, counting M20's `trigger`, M21's `capture` and M23's `pose`, which are edges rather
than ports.
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
- **Wall-clock performance.** No vector is timed. The charter's <15 s target remains unmeasured,
  as `ARCHITECTURE.md` §5 says.
