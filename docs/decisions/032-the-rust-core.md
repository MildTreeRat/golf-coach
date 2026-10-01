# ADR-032: The Rust core — how the analysis engine is ported, and what gates each step

## Status
**Accepted** 2026-09-23 — [ROADMAP §M22](../../ROADMAP.md). Applies
[ADR-030](030-app-platform-rust-core-python-sidecar.md) §1 and §8 to the analysis engine, and
**amends ADR-030's 2026-09-22 addendum** by adding a third clause to its retirement rule, which
reads as a schedule rather than an exemption (§7 below).
[ADR-008](008-project-structure.md)'s import rule is unchanged and becomes compile-time rather
than conventional. Nothing else is amended;
[ADR-022](022-learned-artifacts-as-committed-data.md) is honoured rather than reopened (§5).

**Built, and §9's exit criterion is met** (M22 P8b, 2026-09-25). §2's stage-vector family is
committed (P1) and **all seven stages run green on all 21 vectors**; `crates/contracts` holds the
ported shapes, all three registries and the `unscored` prose table (P2, P5, P5b, P6);
`crates/analysis` holds §3's `pyfmt` (P3) and the whole engine — the geometry (P4), the judging
(P5), the population placements and rotation numbers (P5b), `analyze_swing_bundle` itself (P6), the
second view (P7), the ball flight (P8) and the join from a stored shot to it (P8b). **All 21 engine
vectors conform through `cargo test`**, and `golf-core run` diffs against `conformance.py run`
through Python's own `compare_results` at zero differences on every one of them. **§1's "two
crates" is amended to four** by the seventh addendum — `crates/feedback` and `crates/core`, because
the `run` seam needs both halves of the engine and ADR-008 forbids either half reaching the other;
the workspace holds six, counting M20's `trigger` and M21's `capture`. **No `ANALYSIS_VERSION` bump
and no vector moved**, which §9 predicted and the eleventh addendum qualifies.

**Nothing is deleted.** §7's third clause is why, and it reads as a schedule rather than an
exemption: `analysis/` has a conforming port and committed vectors that prove it, and it stays
until the 28 lab callers that reach it have gone. **§7's schedule is superseded by
[ADR-035](035-rust-everywhere-python-where-required.md)** 2026-09-30, and its rule is not. §M29
now ports the lab, M40 deletes `analysis/`, and two implementations stand until then. Rust
re-records the vectors from M32, under a structural diff gate, rather than after §M29.

Read §3's edges — **six now, and three of them were found by building** — and §7 before starting
a phase, and the **thirteen addenda** below for what building corrected: the first records seven stages, not the eight §2 names; the second corrects §4's
counts and records what a round trip cannot be asked for; the third records that two of §3's three
edges are Rust's own formatter, and which assurance the format table cannot give; the fourth adds a
**fourth portability edge** §3 does not name, and measures how much of a green stage is actually
covered; the fifth adds a **fifth edge**, splits P5 in two, and records the one left-handed vector
that cannot gate the checkpoint whose whole point is handedness; the sixth records that the fifteen
*real* swings never reach the trajectory basis, so its gate is the six synthetic ones, and that this
phase's four surviving divergences are all provable equivalences while its three genuine gaps were
invisible to every vector; the seventh amends §1's crate count, confirms the `run` seam against
Python's own comparator at zero differences, and records that **`EXCLUDED_FROM_RESULT` makes one of
this milestone's own promises structurally unverifiable**; the eighth records that **the corpus no
longer contains a pair `_arbitrate_tops` can decide** — the defect M11 built it for was repaired
upstream — that all fifteen two-camera pairs report `SYNCHRONIZED` so three of the five tiers are
ungated, and that the *second* trajectory basis is reached by four corpus vectors where the first is
reached by none; the ninth adds a **sixth portability edge** that reaches a *bool* rather than a
sentence — CPython's three-argument `math.hypot` is a compensated norm and both Rust stand-ins are
1 ulp out, exactly where a clamp is constructed to land on a table row — and records that
`serde_json` was reading the oracle a ulp wrong until `float_roundtrip` was turned on; the tenth
closes the port, records that the one gate whose input no stage could supply is the one that made
`resolved.launch` an answer rather than a shared assumption, and corrects three things earlier
phases recorded as true; the eleventh is the closing one the Consequences section asked for —
which edges actually fired, what the families cost, that **no vector moved**, and the coverage the
whole port does *not* have, gathered into one list; and the twelfth (M31, ADR-034) records that
**every vector family will name its oracle**, because Python-recorded, hand-worked and
Rust-recorded families are about to stand side by side, that **the screen parser's six
portability edges are M34's list and not additions to §3**, and the planned `crates/pyfmt` split,
pending the refactor ledger; and the thirteenth (M31.5, ADR-035) records that **Rust
records the vectors from M32**, under a structural diff gate, that §7's rule stands while
its schedule becomes §M29 to port and M40 to delete, and that a family Python recorded
and Rust re-records is a kind of oracle M32 has to name.

## Date
2026-09-23

## Context

[ADR-030](030-app-platform-rust-core-python-sidecar.md) §1 gives Rust *"phase segmentation,
measurement, checkpoint scoring, storage"*, and §8 says the port is *"gated by a conformance suite,
not by review"*. M19 built that suite and M20 proved the pattern on a small module. M22 is the
large one: the swing loop itself, which is the reason the platform decision exists at all.

**§M22 has stood since the M18 planning sitting and four of its claims did not survive M19 and
M20.** They are corrected in the ROADMAP by the same change that accepts this ADR, and are listed
here because each one changes the work:

1. It names `api/pipeline.py::analyze_swing`. No such function: the shell entry is
   `api/pipeline.py::analyze_swing_dir`, which calls `analysis/engine.py::analyze_swing_bundle`
   exactly once; `analyze_swing` is one level below that, in `engine.py`.
2. It says to *"prune the import graph"* because `analysis/benchmarks/__init__.py` re-exports
   models *"the checkpoint path never reaches"*. True only of the six checkpoint evaluators, which
   import `load_distribution` and `resolve_range` and nothing else. `engine.py` itself calls
   `benchmarks/joint.py::placement_for` and both `benchmarks/trajectory.py` entry points directly,
   and `flight_measure.py` reaches `benchmarks/flight_model.py` on every swing carrying a shot.
   `benchmarks/__init__.py` imports all five submodules eagerly in any case. **There is nothing to
   prune**, and a session that trusts this line will discover it four stages in.
3. It says `detect_strikes` needs numpy's `hanning` reproduced and is *"the one piece with no
   deterministic oracle"*. M20 did both: the window is in `crates/trigger/src/flux.rs` and the
   oracle is `spec/vectors/audio/`, thirty vectors run by `crates/trigger/tests/conformance.rs`.
   Strike detection is not in this milestone.
4. It calls the core *"stdlib + `contracts` only"* and estimates *"roughly a thousand lines of
   executing logic"*. All five benchmark loaders import pydantic, and the surface reachable from
   `conformance.py::run_vector` is an order of magnitude past a thousand lines — the ROADMAP now
   carries the per-group figures rather than an estimate.

**And one gap that would have defeated the milestone's own rule.** §8 and §M22 both say each stage
is gated by the conformance suite. The committed vectors gate the **whole bundle** and nothing
smaller: there is no committed answer for "phases alone" or "measure alone". Every phase between
the first line of Rust and a complete engine would therefore have been gated by review — the exact
thing §8 exists to forbid — and the first honest signal would have arrived after several thousand
lines, which is the worst possible place to learn that a frame index rounds the other way.

## Options Considered

### Option A: One `crates/core`, gated by the whole-bundle vectors
- **Pros**: fewest moving parts; no cross-crate plumbing; matches the "the Rust core" of ADR-030's
  title; needs no new artifacts in `spec/`.
- **Cons**: no gate until the engine is complete, which is §8 violated in practice while quoting
  it. ADR-008's import rule stays a convention a reviewer has to enforce, in a language that could
  have enforced it for free.

### Option B: `crates/contracts` + `crates/analysis`, gated by a committed stage-vector family *(chosen)*
- **Pros**: the Python package boundary becomes a cargo dependency edge, so ADR-008's *"modules
  never import each other; everything imports `contracts/`"* is checked by the compiler. Every
  phase gets a real gate, and `cargo test` alone runs it — no Python in the loop, which is the
  property M20's audio family already has and the reason it survived the delete. A future port, or
  a rewrite of this one, inherits the same ladder.
- **Cons**: a fourth artifact family in `spec/`, with M19's regenerate and version rules to extend;
  two more workspace members; the stage boundaries are a choice this ADR has to make and defend.

### Option C: Two crates, gated by a live Python differential harness
- **Pros**: no new committed artifacts at all. `compare_results` already exists and would be reused
  as-is; the Python core is present throughout M22, unlike M20's audio, so a live oracle is
  available in a way it was not there.
- **Cons**: the gate needs a working Python install and a subprocess per stage, so `cargo test`
  stops being self-contained; and it leaves nothing behind. The evidence that stage four was
  correct on the day would exist only in a terminal. Rejected on the second point, not the first.

## Decision

### 1. Two crates, mirroring the Python packages

`crates/contracts` and `crates/analysis` join `crates/trigger` and `crates/capture` in the
workspace. `crates/analysis` depends on `crates/contracts`; `crates/contracts` depends on nothing
of ours. That is [ADR-008](008-project-structure.md)'s rule — *"modules never import each other;
everything imports `contracts/`"* — expressed as a dependency edge cargo will not let anyone
violate, which is strictly better than the same rule enforced by reading a diff.

The `analysis/` package's module names carry over one for one, so `measure.py` ↔ `measure.rs`. A
port that renames things while also translating them cannot be diffed against its source by eye,
and every phase of M22 is going to be diffed against its source by eye at least once.

`crates/contracts` holds only what is **reachable** from `SwingBundleResult` and from
`conformance.py::run_vector`'s inputs. A Rust `Bag` with no Rust caller is a second copy that
drifts, which is the argument ADR-030's addendum already makes about implementations.

### 2. Every stage is gated by a committed vector

`spec/vectors/` gains a **stages** family: one file per existing engine vector, carrying the
intermediate answers Python produces for that vector's input, keyed by stage name. The input is
held **by reference** — the stage vector names the engine vector it was derived from and duplicates
none of its 8.6 MB.

The stages are the function boundaries the engine already has, in dependency order: `smoothed`,
`phases`, `measure`, `measurements`, `checkpoints`, `alignment`, `flight`, `feedback`. They are not
invented seams; each is a call `analyze_swing_bundle` already makes, which is what makes them cheap
to record and meaningful to fail on.

Three properties this family has, and all three are why it is worth its bytes:

- **`cargo test` runs it with no Python.** The gate is the same shape as
  `crates/trigger/tests/conformance.rs` — read the directory, sort, deserialize, accumulate every
  difference, one assertion at the end — and that file is the template rather than a thing to
  reinvent.
- **It regenerates without the capture machine.** `regenerate` needs `data/processed/sessions/` to
  build the corpus family, and that directory is gitignored and exists on one machine. Stage
  vectors derive from the **committed** engine vectors, so `regenerate --stages-only` runs
  anywhere. `docs/CONFORMANCE.md` §4 says so.
- **It carries `ANALYSIS_VERSION`** and is stale the moment that constant moves, exactly like the
  engine family. The standing obligation in `CLAUDE.md` — *"an `ANALYSIS_VERSION` bump regenerates
  `spec/vectors/`, in the same change"* — now covers one more family and needs no new words.

**Unlike the audio family, this one stays regenerable.** `conformance_vectors._audio` refuses
because the implementation that recorded it was deleted; the stage builder does not refuse, because
§7 keeps the Python that records it. The contrast is the whole of the difference between M20 and
M22 and is worth reading twice.

### 3. The three portability edges live in one module

`docs/CONFORMANCE.md` §3 predicted two edges. There are three, and all three reach the **strings**
the suite compares exactly rather than the floats it compares within tolerance — which is why they
are one problem and get one module, `crates/analysis/src/pyfmt.rs`, ported before anything calls it.

1. **Python's `round()` is half-to-even and Rust's `f64::round` is half-away-from-zero.** Forty-six
   sites in `analysis/`, in two kinds. Thirteen round a value that is then serialized — into
   `Measurement.value` and `CheckpointScore.observed` — where being wrong costs a failed
   comparison. Seventeen round a **frame index**, where being wrong changes which frame a
   checkpoint is measured on and the error cascades into every score downstream. The second kind is
   the dangerous one, and `analysis/alignment.py` holds ten of them. `round(x, ndigits)` is
   decimal-aware and is **not** `(x * 10^n).round() / 10^n`.
2. **Float formatting reaches the compared strings.** Eighteen `:g` sites and seventy-three `:.Nf`
   sites are interpolated into `CheckpointScore.message` and `Measurement.detail`. Rust's `{}` for
   `f64` is not `%g`, and `{:.1}` is not Python's `.1f` at a tie. One `%g` in
   `benchmarks/flight_model.py` formats a **dict key**, not prose, so getting that one wrong is a
   lookup miss rather than a wrong sentence.
3. **Dict insertion order decides which name appears in a sentence.** `benchmarks/joint.py` and
   `benchmarks/trajectory.py` each build an ordered dict of contribution shares; `engine.py` then
   takes `next(iter(...))` and writes the result into `Measurement.detail` as "Largest contributor".
   On a tie the winner is decided by Python's stable sort over the dict's **original insertion
   order**, so the source iteration order is load-bearing and a Rust `HashMap` destroys it. The
   ported types are ordered maps, explicitly, and the insertion order is part of what is ported.

This third edge is new to §3 and is the one that would have been hardest to diagnose: it produces a
correct number attached to the wrong name, on a swing where two contributions happen to tie.

**This heading's count is wrong from P4 onward, and so is its framing.** Three more edges were
found by building: `str()` on a bare band edge with no format spec (2026-09-24b), `max` returning
the *first* maximum where Rust's returns the last — not a formatting rule, so it lives in
`phases.rs` rather than in `pyfmt.rs` (2026-09-24) — and CPython's three-argument `math.hypot`,
which is a compensated norm whose last bit decides a **bool** (2026-09-26). That last one breaks
*"all three reach the strings"* outright: the list is not the formatting edges, it is the places
CPython computes something Rust computes differently. The one-module decision survived all three.
`docs/CONFORMANCE.md` §3 carries the current list and is the copy to read.

### 4. Pydantic constraints and validators are behaviour

`contracts/` carries seventy `Field(ge=…)`-style bounds and eleven validators. They are **runtime
checks**, not annotations: a port that renders them as documentation silently accepts inputs the
Python rejects, and the two implementations then disagree on malformed data rather than on good
data — which is worse, because no vector exercises it.

They become constructor-checked Rust types. Three deserve naming because they are not bounds:

- `contracts/alignment.py`'s ordering validator is strict on one side and not the other
  (`motion_start <= top`, `impact > top`).
- `contracts/alignment.py`'s `quality_summary` is a `@computed_field` and is **serialized into the
  payload**, so it is output, not a convenience.
- `contracts/swing.py`'s `unscored` validator is a **coercion**, not a check: it accepts a bare
  string and expands it into an `UnscoredCheckpoint` with reason `UNRECORDED`. It exists so
  artifacts written before 2026-08-19 still load. A port that rejects the bare form cannot read the
  older half of `data/processed/`.

### 5. The benchmark data crosses by `include_str!`, with one copy on disk

`ranges.json`, `golfdb_v1.json`, `joint_model_v1.json` and both trajectory models live in
`src/golf_coach/analysis/benchmarks/` and are read there by Python. `crates/analysis` embeds them
at compile time from **that same path**. There is no copy under `crates/`, and there must not be:
[ADR-022](022-learned-artifacts-as-committed-data.md) ships models as provenanced JSON precisely so
there is one authority for a band, and a second copy in a second language is the failure that
invariant exists to prevent.

It also sidesteps, for M22 only, the `config.py::REPO_ROOT` source-checkout assumption that
`docs/CONFORMANCE.md` §5 calls *"tier 1's one known unported assumption"*. Embedding is not a fix
for it; packaging is, and that is M26's.

### 6. `serde` is the dependency, and the numeric-library ban is untouched

`crates/contracts` and `crates/analysis` depend on `serde` and `serde_json`. ADR-030 §1's rule —
inherited by the Rust core as *"no numeric library in the scoring path"* — is about **arithmetic**,
not about JSON. `serde` computes nothing; it is the Rust counterpart of the pydantic the Python
side already carries in `contracts/` and in all five benchmark loaders, and refusing it would mean
hand-writing a JSON parser, which is more code that can be wrong about a float.

`crates/trigger`'s `rustfft` remains the one sanctioned *numeric* exception, for the reason M20
recorded. Nothing in `crates/analysis` gets one.

### 7. Nothing is deleted in M22, and `analysis/` is retired after it

ADR-030's 2026-09-22 addendum retires a module once a conforming Rust implementation exists *and*
the golden vectors that prove it are committed, and names M22 as where that question arrives *"at
far greater scope"*. Both clauses are satisfied at the end of M22. The delete still does not happen
**in** M22, and the rule needs one more clause to say why without saying "never":

> **…and nothing that stays Python calls it** — where *stays* means the sidecar, not *has not been
> ported yet*.

`audio/impact.py` had one production caller and M20 P5 swapped it to a subprocess before deleting
it. `analysis/` has **twenty-eight** importers outside itself: seven in `src/golf_coach/` (`api/`
and `mcp/`) and twenty-one in `scripts/`. **None of them is a sidecar module**, and all of them are
scheduled to go — which is what makes this a sequencing constraint rather than a refusal. The
schedule is [§M29](../../ROADMAP.md):

- **`api/` retires into the Flutter shell.** ADR-030 §4 already replaces the five hand-written
  pages; M25 is where they stop existing. `api/pipeline.py` is the exception — it is orchestration,
  and M24 is where the Rust core takes it over.
- **`mcp/` is ported to Rust.** 3,191 lines against the Python `mcp.server` SDK; the surface
  survives, the language does not. That takes `contracts/caveats.py` and
  `contracts/tool_descriptions.py` with it — §8 lists them as unported **by M22**, not as permanent
  Python.
- **`scripts/` is the blocker still open.** [ADR-022](022-learned-artifacts-as-committed-data.md)
  requires the fitting half to be Python — numpy and scikit-learn, offline, shipping provenanced
  JSON — so some of `scripts/` is permanent by an existing decision. What is *not* settled is how it
  reaches a measurement once `analysis/measure.py` is gone: a `golf-core` subprocess seam, or those
  entry points becoming Rust binaries. §M29 opens with that question, and nothing else in this ADR
  depends on the answer.

**What the delete costs, and it is accepted rather than avoided.**
`scripts/conformance.py::run_vector` **is** the Python core, so deleting `analysis/` retires the
*specification* along with the implementation and `spec/vectors/` stops being regenerable from a
reference. After M29, an `ANALYSIS_VERSION` bump re-records the engine family **from the Rust
core**.

That is the self-portrait `conformance_vectors._audio` refuses in three sentences, and the
difference is what the vectors are *for* at each moment:

- **Before a port conforms** they are a cross-language oracle, and re-recording them from the
  implementation under test proves nothing. That is `_audio`'s case and it still holds — the audio
  family stays frozen, and so does the engine family until M22 closes.
- **After it conforms** the cross-language question is answered and a version bump cannot re-open
  it. What the engine family does from then on is make a deliberate behaviour change **visible**: a
  bump re-records, a reviewer reads what moved, and the vectors are a changelog plus a regression
  suite rather than an oracle.

Freezing the engine family the way audio is frozen was the alternative, and it costs more than it
buys. `AUDIO_DETECTOR_VERSION` has moved twice. `ANALYSIS_VERSION` is at 16 and is the mechanism by
which a new measurement reaches the corpus at all — M15 P13's whole finding was that four new
metrics are exactly what a bump is for. Freezing it freezes the product.

So M22 ends with two implementations of the swing loop, **temporarily**. ADR-030's second
Consequence — *"the Python one stays"* — is not restored here; its addendum stands, and §M29 is when
it fires.

### 8. What is ported, and what is explicitly not

**Ported**: everything reachable from `conformance.py::run_vector` — the geometry (`smoothing`,
`stats`, `measure`, `phases`), the judging (`scoring`, `pivot`, `trajectory`, `checkpoints/`, four
of the five `benchmarks/` loaders), the assembly (`engine.py` and `feedback/rules.py`), the second
view (`alignment.py`), and the outcome (`shot_measure`, `flight_measure`, `flight`, `flight_infer`,
`spin_solve`, `benchmarks/flight_model.py`).

**Not ported, and recorded here so it is not re-derived**:

- `contracts/caveats.py` and `contracts/tool_descriptions.py`. Both are prose builders for the MCP
  surface, and — checked rather than assumed — **nothing in `analysis/` or `feedback/rules.py`
  imports either**. Every mention is a comment explaining why the registry they derive from lives
  where it does. They are out of **M22's** scope, not permanent Python: they follow `mcp/` into Rust
  in §M29 (§7).
- `analysis/flight_caveats.py`, `baseline.py`, `comparison.py`, `dispersion.py`, `club_profile.py`
  and `tempo_trainer.py` — entered from `api/` and `mcp/`, never from the engine. They go where
  their callers go (§7): with the Flutter shell, or with the Rust MCP port.
- `feedback/coach.py` and `feedback/conversation.py`. **These are sidecar**, not deferred: the LLM
  call is one of the two things ADR-030 keeps in Python on purpose, alongside MediaPipe pose.
  `docs/CONFORMANCE.md` already says the prose is unpinned.
- The `contracts/` shapes those modules own.

### 9. The exit criterion is 21/21, and the port is wired to nothing

M22 is done when all **21** engine vectors conform through `cargo test` — six synthetic and fifteen
corpus. The corpus half is not optional and cannot be deferred: every corpus vector carries a shot,
a loft, an intent, a down-the-line clip and both strike lists, and was recorded with them, so the
flight modules are inside the exit criterion whether or not a phase list puts them last.

Unlike M20 P5, **no Python swaps over in M22**. `api/pipeline.py` keeps calling
`analysis/engine.py`, because the port has to be proven before anything depends on it and because
§7's callers have not gone yet. The Rust core gets its first real caller in M24; until then its only
caller is its own test harness. That is a deliberate non-action, stated because the M20 shape would
otherwise imply a swap phase inside this milestone that should not be built — the swap is M24's and
the delete is §M29's.

## Consequences

- **`spec/` gains two families and `conformance.py` gains a kind.** `stages` (§2) and a small
  `format` family for §3's `pyfmt` table. `check` defers both to `cargo test`, exactly as it
  already defers audio, and `vector_paths()` still means everything committed.
- **The workspace goes from two crates to four**, and `crates/analysis` is the first Rust here with
  a Python counterpart that is staying. Drift between them is a real risk, and the committed
  vectors are the only thing that catches it — which is an argument for regenerating them on every
  bump, not a reason to keep the two in sync by hand.
- **ADR-008 is enforced by cargo on the Rust side and by review on the Python side.** The asymmetry
  is worth knowing about; it is not worth restructuring Python to fix.
- **Two implementations exist at the end of M22, and that is temporary.** ADR-030's addendum is
  not narrowed by this ADR — it gains a clause that reads as a *schedule* (§7). `analysis/` is
  deleted in §M29, once `api/` has retired into the shell and `mcp/` has been ported; the one
  question left open is how `scripts/` reaches a measurement afterwards.
- **`docs/CONFORMANCE.md` §3 goes from two known edges to three**, and §5's tier table moves its
  tier-1 and flight rows to ported at the *end* of the milestone rather than at its start.
- **Nothing here is measured yet.** Every number in this ADR is a count of the existing Python —
  sites, importers, lines. The port's own numbers — which edges actually fired, what the stage
  family costs in bytes, whether any vector moved — belong in an addendum written at the end, the
  way ADR-031's was.

## Deferred, by choice

- **`crates/trigger` on `crates/contracts`.** The detector writes an `AudioFile` that already has a
  committed schema and will shortly have a Rust type. Unifying them is obvious and is not M22's: it
  would put a second milestone's crate on this milestone's critical path for no gate.
- **`audio/ffmpeg.py`.** Tier 1, unported, and named as next by ADR-030's addendum. Python still
  decodes and Rust detects; M22 begins at a decoded waveform and changes nothing about that seam.
- **`make_swing` in Rust.** ADR-030 §8 offers it as a way for a port to generate its own inputs. It
  is needed by no gate here — every vector carries its input serialized — so it is a convenience
  for writing *new* Rust tests, and can wait until something wants one.
- **Performance.** No vector is timed and ADR-030 §3's 15-second target is unmeasured in either
  language. M22 is about agreement; speed is M24's, where there is a queue to be late for.

## Addendum, 2026-09-23 — seven stages, not eight, and `source` is not a grouping

M22 P1 built §2's stages family. Three things it found, in the order they cost something.

**`feedback` is not a stage, and the rule that excludes it is worth stating.** §2 listed eight
stages and the built family has seven. The rule is that a port must be able to run a stage **in
isolation from the vector's input** — that is what makes a stage a gate rather than a note.
`build_feedback` takes the assembled `SwingResult`, which no stage produces and the stages file
does not hold, so a port cannot reach it until the engine is finished; and at that moment
`expected.feedback` on the engine vector already gates it, which is what §M22 P6's end-to-end gate
is. Recording it would have been a second copy of an answer committed a few hundred bytes away —
the thing this repo's rule about copied values exists to prevent. The absence is pinned by
`test_feedback_is_not_a_stage_and_that_is_deliberate`, because an absence reads as an oversight to
everyone who did not make it.

**Grouping `measurements` by `Measurement.source` is the obvious implementation and is wrong.** It
was caught producing eighteen `pose` rows against thirteen pose measurements: `engine._PIVOT_VIEWS`
gives the face-on pivot rows `source="pose:face_on"` as well, and `_placements` and
`_dtl_placements` both emit `population:golfdb`. `source` answers *which instrument read this*,
which is a different question from *which function appended it*, and only the second maps onto a
port's phases. The groups are a **positional partition** of the one list `_measurements` returns,
asserted to cover it exactly. This matters beyond P1: §3's third edge is about a name reaching a
sentence from an ordered dict, and this is the same class of mistake one level up — a field that
looks like a key and is a description.

**The bytes went where §2 did not predict, and one of the two was reducible.** The family is
**3.2 MB** across 21 vectors, taking `spec/vectors/` from 15 MB to 18 MB, and two stages are 97% of
it. `smoothed` records `x`/`y` alone: `smooth_keypoints` copies `z`, `visibility`, `frame_index`
and `timestamp_ms` through untouched, so recording them costs 2.3 MB gzipped to test a copy, and
the pass-through is asserted at build time instead. `flight` is the other and is **not** reduced —
most of it is the integrated path at `DEFAULT_STEP_S`, ~919 points on a flown shot, and that path
is a product surface (M15 P15 and P19 draw it) that the Flutter shell inherits when `api/` retires,
not an intermediate. A third, smaller finding: `smooth_keypoints` **drops `camera_id`**, which
nothing reads back and no vector would ever have shown, so the build guard asserts it rather than
leaving a port to carry the field through and be right by accident.

**And one thing §2 got exactly right, which is worth recording because it was the whole point.**
The family regenerates from the committed engine vectors on a machine with no captures — the
`--stages-only` path was built and run that way. What §2 did not have is the guard that makes it
evidence: `run_stages` calls the engine's own functions but *re-orchestrates* them, and that
orchestration is a second copy of `analyze_swing_bundle`'s. `_verify_stages_compose` refuses to
build a vector whose parts do not add back up to the committed bundle answer, which is the corpus
family's `_verify_against_stored` argument applied one level down. It runs in the test suite as
well as at build time, because a build-time-only guard stops running the moment nobody
regenerates.

## Addendum, 2026-09-23b — what §4 counts, and what a round trip cannot be asked for

M22 P2 built `crates/contracts`: nine modules, gated by reading every committed vector's `input`
and `expected` into the ported shapes and writing them back. Four things it found.

**§4's "seventy bounds and eleven validators" is a count of the whole Python package, not of the
ported surface, and the difference matters to a reader sizing the phase.** The payload shapes —
what §1 scopes this crate to — carry **47 bounds and 3 validators**. The other 23 bounds sit in
`audio.py`, `baseline.py`, `dispersion.py`, `reference.py` and `tempo.py`, and the other validators
in `bag.py`, `club_profile.py`, `club_spec.py`, `dispersion.py`, `tempo.py` and `golfer.py`'s name
coercion — every one of them in a module §8 does not port. What is worth keeping is the coincidence
underneath: the three reachable validators are **exactly** the three §4 singles out as *"not
bounds"*, so on the ported surface that footnote is the whole list rather than a sample of it.

**A byte-for-byte round trip is not available in either direction, and no tolerance repairs it.**
Python writes `-1.636758133827243e-05`; `serde_json` writes `-0.00001636758133827243`. Both are
shortest-round-trip forms of one f64 and both parse back to identical bits — they disagree only
about when to reach for an exponent, and **77 distinct floats in the committed vectors take the
exponent form**, all of them landmark `z`. So every cross-language comparison in this port is
structural, over parsed values, which is what `conformance.compare_results` already does and what
makes §9's `run` seam need no formatter. It is worth stating as a *consequence* rather than a
detail: anything that wanted to hash an artifact, or diff one by bytes, across the two languages
would need a Python-compatible float formatter, and that is a different piece of work from §3's
`pyfmt` — which is about `%g` and `.Nf` reaching sentences, not about JSON.

**A vector distinguishes an absent key from a null one, and the obvious harness shape does not.**
Ten corpus vectors carry `"loft_deg": null` while `windowed.json` simply has no `shot` key;
`run_vector` reads both with `.get()` and cannot tell them apart, so nothing before P2 had to.
A round trip can, and the first build flattened the two — rewriting ten files' `null` as no key and
reporting itself as eleven differences. The fix is a two-deep `Option` with a `deserialize_with`
that fires only on a present key. Recorded because the gate found its own defect on its first run,
which is the argument for building it strict: a looser comparator would have passed and the
distinction would have surfaced at P6 as a vector the Rust `run` could not reproduce.

**§1's reachability rule leaves three registries outside the crate, and they go with their
walkers.** `contracts/` holds `CHECKPOINT_REGISTRY`, `PIVOT_MEASUREMENT_REGISTRY` and
`UNSCORED_REASONS` — tables the engine walks rather than shapes the payload carries. They belong in
`crates/contracts` under ADR-008 and they are not in it yet, because **P2's gate cannot see a
table**: round-tripping a vector proves a field survives and says nothing about a checkpoint spec.
The first two land with P5 and the prose table with P6, which keeps §2's *a gate per stage* true
rather than shipping data one phase ahead of the thing that would catch it being wrong.


## Addendum, 2026-09-23c — two of the three edges are Rust's own formatter, and one assurance the table cannot give

What M22 P3 found building §3's `pyfmt` and the `spec/vectors/format/` family that gates it. §3
stands as written — there are three edges, they reach the compared strings, and they belong in one
module — and what changes is the *cost* of two of them and the honesty of the third's gate.

**`core::fmt` already rounds the way CPython does, so two of the three edges reduce to it.** Rust's
`{:.N}` and `{:.Ne}` are not approximations: the exact path in `core::num::flt2dec` generates digits
from the f64's **exact** binary value and breaks a tie **to even**, which is the rule CPython applies
through `_Py_dg_dtoa`. So `round(x, ndigits)` ports as `format!("{x:.n$}")` parsed back, and `%g`
takes its mantissa and exponent from `{:.5e}` and only then picks a form. Neither needed the decimal
arithmetic §3 implied, and the one-argument `round()` is `f64::round_ties_even` in a single call.

Which makes the *scaled* form — `(x * 10f64.powi(n)).round() / 10f64.powi(n)` — the whole of edge 1's
risk, and it is worse than a corner case: over 200,000 three-decimal values in `[0, 10)` it
disagrees with CPython on **4.6%** of them at two places, even when the scaling is given ties-to-even
as well. The multiply is what breaks it. `0.215` is exactly `0.21499999999999999667`, so it rounds
*down*, but `0.215 * 100.0` is `21.5` on the nose and rounds up.

**The values where the two rounding rules differ are enumerable, which is why the table is 266 KB
and not a fuzz harness.** A tie at `n` decimal places needs `x * 10**n == j + 0.5` to hold with no
representation error; the `5**n` in the denominator has to cancel, which forces `x` to be an odd
multiple of `2**-(n+1)`. Half-to-even and half-away-from-zero differ on those values and **nowhere
else** — so they can be listed completely, and a uniform random sweep hits *none* of them. A
rounding gate built only from sampled values would pass a port with the wrong rule, which is the
trap this note exists to prevent the next time someone regenerates the family.

**The one-argument and two-argument `round` need different comparison rules, and the strict gate
found it on the first run.** `round(-0.49999999999999994)` is the `int` `0` in Python, which has no
sign to carry, where `round_ties_even` legitimately answers `-0.0`; comparing bits there asserts a
sign the specification does not have. But `round(-0.04, 1)` **is** `-0.0`, a float, and a port that
normalizes it serializes a different number into `Measurement.value`. So the float form of the
one-argument rounding is compared by value and everything else by bits — the only relaxed comparison
in the family, and not a tolerance.

**The format family is the first that does not age on `ANALYSIS_VERSION`.** Its subject is CPython's
`round`, `format` and `sorted` rather than this engine's answers, so a version bump leaves every
answer in it true. It carries `python_version` instead, and `check` reports it with no staleness
test — pinned in `tests/test_conformance.py`, because the absence of the field is a decision and
reads as an oversight otherwise.

**One mutation survived, and the table says so rather than claiming the coverage.** Swapping
`sort_by` for `sort_unstable_by` passes all 10 ordering cases: Rust's unstable sort is an insertion
sort below 20 elements, and the share maps hold three to six entries, so at the sizes the engine
really uses the two calls give the same answer. That assurance therefore comes from the documented
stability of `sort_by` and not from the gate. A second survivor was a genuine gap and was closed: a
registry rank of `MAX - len(name)` passed both original cases — one because two unregistered names
tied on length and one because they all did — until unregistered names of **distinct** lengths in
neither alphabetical nor length order were added. Both are recorded because the pattern is the
useful part: an ordering gate covers what it can *see*, and a case without a collision in it covers
nothing at all.

---

## Addendum, 2026-09-24 — a fourth portability edge, and how much of a green stage is covered

M22 P4 ported the geometry — `analysis/{smoothing,phases,measure}.py` and `stats.percentile` — and
the `smoothed`, `phases` and `measure` stages run green on all 21 vectors. Two things it found are
worth more than the port.

**§3 names three edges and there is a fourth: `max` and `min` break ties in opposite directions.**
Python's `max` returns the **first** maximum; Rust's `Iterator::max_by` returns the **last**. `min`
agrees in both languages, which is what makes this easy to miss — half the pattern is safe.
`phases._top_and_impact`'s no-runs fallback is `max(range(top, n), key=ys.__getitem__)`, so on a
clip whose wrist `y` repeats, the two languages disagree about **which frame impact is** — and that
is the same failure §3's rounding edge is about, arriving through a different door: seventeen sites
round a frame index, and this one does not round anything. It belongs with the three: it produces a
plausible number on a real clip, it lands on a frame index rather than a float, and it is invisible
to review. `phases::first_argmax` is it solved once, beside `pyfmt`'s three.

Whether it belongs *in* `pyfmt` was considered and declined. `pyfmt` is "CPython's formatting and
rounding rules"; a tie-break in `max` is a rule about an *iterator*, and folding it in would make
that module's subject "things Python does differently", which is not a boundary anyone can hold.

**A green stage is not a covered stage, and the gap is measurable.** Twelve deliberate divergences
were introduced one at a time and run against both gates. Six were caught by the vectors. **Five
survived all 21 vectors and were caught only by the port's own unit tests** — and the reasons are
facts about this corpus rather than about the code:

| the divergence | why 21 vectors cannot see it |
|---|---|
| `round` half-away-from-zero for the motion-start stall | 3 of 21 vectors **do** land the downswing on the exact tie (d = 18, 10, 10), but on none of them does a 2-frame stall against a 3-frame one move the answer — the quiet run there is longer than either. Reached, not load-bearing |
| `max_by` for the first-maximum fallback | **0 of 21** clips have no rising run at all, so the fallback branch never executes |
| hips gated at `MIN_VISIBILITY` rather than `MIN_HIP_VISIBILITY` | **0 frames** anywhere in the 21 carry a hip landmark in `[0.5, 0.7)`, so the harder gate never rejects a frame the ordinary one accepts |
| take the largest descent, not the earliest major one | on **21 of 21** the earliest major run *is* the largest. The rule exists for the practice-swing and tracking-junk case, and this corpus has none |
| a midpoint from one confident landmark and one guessed | 660 half-confident pair-frames exist across the 21 — and **0** of them fall in a (pair, window) combination any measurement reads. The wrists are read only at address; the ears and hips only where they are tracked cleanly |

The refusal paths are the same shape and larger: **not one of the 21 vectors refuses a single
measurement**, so the `measure` stage gates thirteen happy paths and none of `measure.py`'s
twenty-odd refusal branches — every one of which carries a `detail` sentence
[docs/CONFORMANCE.md](../CONFORMANCE.md) §3 compares **exactly** and a golfer reads under
`SwingResult.unscored`. A port that got every one of those sentences wrong passes this phase's gate.

That is not an argument against the stage family — it caught the other six, including the window
seam and the `.Nf` interpolation, and without it the geometry would have been gated by review. It
is a correction to what "P4 is green" licenses. **The committed vectors gate the path this corpus
takes; the branches it does not take are the port's unit tests to gate**, which makes those tests
load-bearing rather than decorative and makes a phase that lands with nothing but a green vector run
an unfinished phase. `crates/analysis/tests/geometry.rs::every_vector_measures_everything` pins the
refusal hole so it stays a recorded fact, and **fails the day a vector refuses something** — which
is the good failure, and should be answered by extending the stage comparison rather than relaxing
the test.

**Three further notes for the phases after this one.** `smooth_keypoints` **drops `camera_id`** —
the Python builds its `FrameKeypoints` without one — so the smoothed timeline has no camera on it
even where the input did, which is why `run_stages` reads the camera off the *unsmoothed* frames;
P7 reads that field and needs the `None`. Python's `math.hypot` and `(dx**2 + dy**2) ** 0.5` are
**not** the same function — `hypot` is scaled and correctly rounded — and `measure.py` uses both, so
the port matches each site rather than normalizing them. And `analysis/phases.py`'s clip-*choosing*
half (`candidate_downswings`, `select_swing`, `select_matching_swing`, `window_around`) is reached
only from `api/pipeline.py` and `scripts/`, so it has no committed gate and stays Python under the
same rule P2 applied to the three `contracts/` registries: code lands with the phase that can prove
it. `alignment.py` reaches only `segment_phases` and three constants, which is P7's.

## Addendum, 2026-09-24b — a fifth edge in the sentences, and the one vector that cannot gate the mirror

*(M22 P5, the judging.)* `benchmarks/{store,distributions}`, `checkpoints/mechanics` and
`contracts/checkpoints.py`'s registry are ported and green on the `checkpoints` stage across all 21
vectors. The phase was written as the whole judging half — the four benchmark loaders, `pivot`,
`trajectory`, `scoring` and the second registry with it — and **it was split rather than absorbed**,
under §M22's own rule. The written scope measured at roughly 2,900 lines of Rust against P4's 1,591,
and the seam is not arbitrary: the two gates it names are independent. The `checkpoints` stage needs
`store`, `distributions` and `mechanics`; the `measurements` stage needs `joint`, the trajectory
model, `analysis/trajectory`, `pivot` and `contracts/pivots`, and neither half reads the other. So
P5 is the first and **P5b** the second, and the phase count is eleven.

**ADR-032 §3 names three edges, P4 found a fourth and this is a fifth.** `f"aim under {band.high}"`
— three of `mechanics.py`'s sentences — interpolates a float with **no format spec at all**, and in
Python 3 `str`, `repr` and `format(v, "")` are one function for a float. Rust's `{}` produces the
same shortest-round-trip digits and presents them under different rules: `4.0` against `4`
(CPython's `Py_DTSF_ADD_DOT_0`), and an exponent form at `decpt <= -4 || decpt > 16` that Rust's
`{}` does not have at all. It belongs in `pyfmt` — it is a CPython formatting rule, which is that
module's whole subject, where P4's `max` tie-break deliberately is not — and it arrived with a
table: `spec/vectors/format/repr.json`, **362 cases**, taking the family to five vectors and 3,059.

The reason it is worth an addendum rather than a line is what the mutation run found. **Every band
edge in `ranges.json` today formats identically in both languages**, so replacing `pyfmt::repr` with
Rust's `{}` passes all 21 vectors, every unit test in the crate, and the entire Python suite. The
same is true of the `:g` sites, for the same reason. This edge is real and currently invisible, and
the only thing standing between it and a future band of `0.5` becoming `0.5` in one language and
`0.5` in the other — or of `4.0` becoming `4` — is the table.

**The finding worth more than the port: the corpus's one left-handed vector cannot gate the
mirror.** `evaluate_head_stays_back` re-signs its observation into the right-handed camera frame the
band was cut in, which is the *entire reason* that checkpoint takes a `Handedness` and the
difference between reading a left-handed swing and calling an ordinary impact position a gross
fault. Of the 21 committed vectors, 19 are right-handed, one has no golfer, and the one left-handed
vector has a `head_hip_gain_norm` of **exactly `0.0`** — so `-raw == raw` and **deleting the mirror
outright passes every vector**. The first version of P5's gate asserted only that both handednesses
appear in the corpus, which is true, and is the kind of reassurance that stops someone looking;
`tests/judging.rs::the_left_handed_vector_cannot_gate_the_mirror` now asserts the hole instead, and
fails the day a left-handed vector with a non-zero gain is captured.

**The stage's coverage, measured the way P4 measured `measure`'s.** 21 vectors times six checkpoints
is 126 evaluations and **exactly one refusal**, a single `no_handedness`. So the stage gates 125
happy paths and one refusal branch; `NO_BAND` is never reached, because `resolve_range` is asked for
`club=all` or the default on every committed vector and the per-club fallback that produces it in
production is never walked. Seventeen deliberate divergences were run against both gates. Three were
caught by the vectors alone or by both — the registry order, the `at least` qualifier at the
percentile rail, a quantile dropped from the interpolation walk — and after the port's own tests were
strengthened in response, seven more were caught by unit tests only. **Five survive everything, and
all five are now explained rather than open:**

- Three are *provably equivalent* mutations, which is dead defensive code faithfully ported.
  `_score_within_range`'s zero-width guard is load-bearing in Python, where `1.0 - distance / 0.0`
  raises, and redundant in Rust, where it is `-inf` and the `max` flattens it to the same `0.0`.
  `evaluate_head_stays_back`'s `observed > band.high` is the same test as `>=`, because that arm is
  only reached when `passed` is false and `observed == band.high` makes it true. And
  `Distribution.percentile_of`'s `high_val == low_val` arm — and the `50.0` fall-through under it —
  are unreachable for **any** quantile arrangement, monotone or not: a degenerate bracket selected
  at `observed` means the bracket before it already contained `observed`, and at the ends the
  `p10`/`p90` clamps answer first. `neither_defensive_arm_in_percentile_of_is_reachable` searches
  15,625 arrangements for a way in and finds none. They stay, because §3 is about matching an
  implementation rather than improving one, and because a `derive_reference.py` bug that ships a
  non-monotone row makes them the difference between a wrong percentile and a NaN in a sentence.
- Two are the formatting call sites above: `pyfmt::g` and `pyfmt::repr` replaced by Rust's `{}`.
  Gated by the format table, not by a swing, and that is the honest reach.

**Two smaller things the split settled.** `PlayerProfile` crosses into `crates/contracts` even
though no vector carries one and P2's round-trip gate therefore could not see it: the alternative
was to widen `resolve_range` to a bare `&str` skill level and move the `None` → `"all"` fallback to
the call site, which is a different function rather than a translated one. Every ported caller
passes `None`, so the `checkpoints` stage proves that branch and nothing proves the other. And
`analysis/scoring.py` did **not** come with this phase: no stage records `mechanics_score`, so its
gate is the engine vector and it lands with the assembly in P6 — the rule P2 set for the registries
and P4 applied to `stats`' career-mode half, applied once more.

## Addendum, 2026-09-24c — a corpus that never reaches the basis, and four survivors that are all equivalences

*(M22 P5b, the placements and the rotation numbers.)* `benchmarks/{joint,trajectory}`,
`analysis/trajectory`, `analysis/pivot` and the `measurements` assembly's `pose`, `placements` and
`pivot_face_on` groups are ported and green across all 21 vectors — 2,131 lines of Rust before their
tests against 1,619 of Python, with 75 unit tests and 6 gate tests beside them. Five of the seven
stages now have a runner. `contracts/placements.py` and `contracts/pivots.py` came with them, so
**all three registries §1 kept out of P2 are now in `crates/contracts`, and each arrived beside its
walker** — which is the rule holding rather than a coincidence.

**`engine.rs` arrives half-written, deliberately.** Each of the seven `measurements` groups belongs
to a different phase — three here, two in P7, two in P8 — so a module that waited for all of them
would have no gate until the end, which is what §2 exists to prevent. The alternative was a gate
test that rebuilt `_measurements`' grouping itself, and `conformance.py::run_stages`' own docstring
already refuses to write that second copy. P6 brings `analyze_swing_bundle` and the rest.

**The finding worth more than the port: the fifteen real swings never reach the trajectory basis.**
`build_trajectory` refuses a swing whose landmark is missing more than `MAX_MISSING` (40%) of its
resampled timeline, and face-on the trail elbow and trail wrist always are — **45-70% across the
corpus**, because the trail arm crosses the torso for most of the swing. So every corpus vector
records `tour_joint_distance` alone, and `tour_trajectory_t2`, `tour_trajectory_q`, the `%g`
percentile in their `detail` sentences and the interval name in Q's are gated by the **six synthetic
vectors** — the ones with perfect visibility by construction. "21 vectors, all green" reads as full
coverage of this stage and is the opposite of it: the real footage gates the *refusal* and the
synthetic footage gates the answer. `tests/measurements.rs::the_corpus_never_reaches_the_trajectory_basis`
asserts the shape of that hole and fails the day a corpus vector reaches the basis.

**The counterpart to the previous addendum's mirror, and it goes the other way.** P5 found that the
corpus's one left-handed vector cannot gate `head_stays_back`'s mirror. The *trajectory* mirror is
genuinely gated: `synthetic/face-on-only` is left-handed **and** one of the six that reach the
basis, so `build_trajectory`'s left/right swap and its `x` negation both sit on the tested path, and
deleting either half moves that vector's T² and Q. The two cases look identical from outside — "one
left-handed vector out of twenty-one" — and are not, which is why each is measured rather than
assumed.

**Forty-five deliberate divergences: four survive, and every one is a provable equivalence rather
than an open hole.** This is a different result from P5's five, where two were genuine reach limits:

- `sample_positions`' `span * i / (steps - 1)` against `span * (i / (steps - 1))` is
  **bit-identical** while `span` is a power of two, and it is 2 at every call site in the crate
  (three anchors, both resamplers) — scaling by a power of two is exact in IEEE 754, so the
  reassociation cannot move a bit. It stops being true on a four-anchor model. The Rust docstring
  first claimed the opposite and was corrected by the mutation.
- A *different fixed* pivot origin cancels out of all five rotation checks: `axis_drift` subtracts
  its own address sample, `hip_path_jitter` is a second difference, and the two reversals read
  orientations — every one is invariant under a constant shift. A **per-sample** origin, which is
  what "hip-relative" means in `build_trajectory`, is caught immediately. So the address hip is
  load-bearing for the overlay and for the first signed pivot metric, and is not a number any check
  can see today.
- `math.hypot` against `(dx**2 + dy**2) ** 0.5` — P4's wart again, one layer up. The two agree to
  within an ulp on every magnitude a normalized landmark pair produces, and the only consumer
  divides by the result. Kept because §3 is about matching an implementation.
- `pyfmt::g` replaced by Rust's `{}` at the two percentile call sites is **exact on the whole
  reachable domain**: a percentile is `round(x, 1)` clamped into `[10, 90]`, so it carries at most
  three significant digits and never reaches either boundary where `%g` switches to an exponent.
  This is the previous addendum's fifth edge recurring in a second module, invisible for a *second
  independent reason* — and that is the strongest argument yet for `spec/vectors/format/` existing,
  because two unrelated call sites have now each been safe by accident.

**Three real gaps the vectors could not see, closed by unit tests.** The shoulder-width ruler is the
`len/2` **median**: the mean and the lower median are each one character away, every synthetic swing
has a constant shoulder width so all three rules agree on them, and the corpus never reaches the
line — so nothing in 21 vectors could distinguish them. And a resampled sample needs **both** its
bracketing frames confident: `max` for `min` reads every sample in this corpus and passes
everything. *Where a stage cannot see a branch, the port's own unit tests are the gate* is §2's
consequence, and this phase is the sharpest case of it so far — two of the three gaps are in the
same function.

**What did not come, and why.** The **down-the-line trajectory artifact**: its reader is
`_dtl_placements`, which is P7's, and no committed vector reaches it before then.
`load_trajectory_model` refuses that view by name and says where it lands, so the second
`include_str!` is a one-line addition rather than a plumbing change. `contracts/career.py`'s
`POSE_DTL_SOURCE` did not come for the same reason, which is why `_PIVOT_VIEWS` has one row here
instead of two — that absence is a *module* boundary, unlike `CALIBRATED_3D`'s, which is a refusal
`pivot_observations` raises on. And neither `JointDatasetInfo` nor `TrajectoryDatasetInfo` crossed:
`run_vector` reads no provenance block, the same cut P5 made on `DatasetInfo`.

## Addendum, 2026-09-25 — four crates, not two; and the comparison cannot see what the contract promises

M22 P6 ported the assembly — `engine.py`'s face-on path, `analysis/scoring.py`, the face-on slice of
`analysis/alignment.py`, `feedback/rules.py` and the `golf-core run` binary — and the six synthetic
vectors are green **end to end**, which is the first whole-bundle conformance in this repo and the
first gate here that is not a stage. Four things building it corrected or established, and the first
is an amendment to §1.

### §1 says two crates and the port needs four

`crates/feedback` and `crates/core` join `crates/contracts` and `crates/analysis`. The reason is
§1's own rule taken seriously rather than an exception to it.

ADR-008 forbids `analysis` and `feedback` importing each other, and §1 asks cargo to enforce that as
a dependency edge. But that separation is *why* `conformance.py::run_vector` makes two calls:
`analyze_swing_bundle` leaves `SwingBundleResult.feedback` as `None` and the caller fills it in, so
the artifact this repo writes has ranked tips and a bare engine call does not. Something above both
halves has to make those two calls, and it cannot be either half. In Python that something is `api/`
and `scripts/`; here it is `crates/core`, which holds the `run` seam, the `golf-core` binary and the
end-to-end gate.

The three alternatives were each worse in the same way — they would have bought a smaller crate
count by spending the edge §1 exists to install. Putting `rules.rs` inside `crates/analysis` makes
ADR-008's analysis/feedback split conventional again, enforced by a reviewer's eye. Putting the
binary in `crates/feedback` with `analysis` as a dependency, or in `crates/analysis` with `feedback`
as one, creates the forbidden edge outright and points it in a direction Python does not have.

So §1 stands as written about the **port** — two crates hold it, and the module names still carry
over one for one — and the count in it should be read as naming those two rather than bounding the
workspace. `crates/core` is not part of the port at all: it is the counterpart of `api/pipeline.py`,
which §7 retires into the Flutter shell rather than porting, and it is where M24's first real caller
will attach.

### The `run` seam works, and it confirms §3's no-byte-comparison rule where it was predicted

`golf-core run < vector.json` was diffed against `python scripts/conformance.py run < vector.json`
on all six synthetic vectors, through `conformance.compare_results` itself — Python's own
comparator, with no Rust comparator in the loop. **Zero differences.**

And the two stdout streams are 13,569 and 13,221 bytes for one vector, which is P2's finding
standing exactly where §4 said it would: Python writes `-1.636758133827243e-05` where `serde_json`
writes `-0.00001636758133827243` for the identical f64. The seam needs no formatter because nothing
compares bytes — a `diff` of the two files shows *where* two implementations disagree once you have
already decided they do, and is not the gate.

### The finding worth more than the port: `EXCLUDED_FROM_RESULT` makes a milestone promise unverifiable

`analyze_swing_bundle`'s contract is that **everything comes back in whole-clip coordinates**: a
window is a search restriction, not a coordinate system, so the face-on phases are shifted back by
the window offset *and* the sliced frames are re-attached, which is what makes
`swing.phases[i].start_frame` a valid index into `swing.keypoints`.

The comparison cannot check the second half. `EXCLUDED_FROM_RESULT` drops `keypoints` and
`detections` before a vector is written, so `expected` holds no frame list at all — and mutating the
re-attachment to a no-op **passes all 21 vectors, in either language**. The one committed vector with
a window, `synthetic/windowed.json`, gates the shift and is structurally incapable of gating the
frames those shifted indices address.

That exclusion is right — round-tripping keypoints would make every vector 30x larger and check
nothing — so this is not an argument for removing it. It is an argument for knowing what the suite
is silent about, and the silence is wider than one field: on a one-camera bundle `alignment`
serializes as `null`, so **no `SwingAnchors` field reaches a compared payload at all**. Hard-coding
`motion_start_detected` to `true` — which inverts an ADR-013 disclosure — passes everything P6 can
run. Both now have unit tests standing where the vectors cannot.

### `_tempo_notes`' sentence is reached by nothing in `spec/vectors/`, and not because the six are clean

The sentence is *"the backswing measures N downswings, which no golf swing does"*, and P5b's shape
recurs here with a sharper reason than "the synthetic vectors are well-behaved". Every collapsed
motion-start boundary in the **corpus** is in the *down-the-line* view, and `_tempo_notes` reads the
face-on anchors alone. The corpus notes that look like this one —
`alignment: down_the_line: backswing measures 0.05 downswings…` — are `align_swings`' own string,
which is a different function and P7's. `aaron-1`, the swing the sentence was written for and which
reads 0.43:1, is on disk as `corpus/2026-08-07-aaron1-1`, and its *face-on* view is fine.

`_anchored_on_strike`'s sentence, by contrast, **is** gated — at P7: fourteen of the fifteen corpus
vectors carry one and six of those are face-on. The two sentences sit four lines apart in
`engine.py` and their coverage is nothing alike, which is the kind of thing only a per-branch sweep
finds.

### Sixty-two divergences, nine survivors with one root cause, and two provable equivalences

The sweep changed the tests far more than it changed the code, which is the pattern P4 set and P5
repeated. The first pass caught 31 and sixteen survived — and **nine of those were one root cause**:
the gate runs six clean single-camera swings, so six of the seven sentences `engine.rs` can append
are unreachable through it and `_windowed`'s three guards (an empty window ignored, a negative start
clamped, an end past the clip clamped) are never exercised. Twelve unit tests closed them and the
second pass catches 60.

The two that survive are equivalences with a stated domain, not gaps:

- **`round(0.20 * fps)` never ties.** A tie needs `0.20 * fps == n + 0.5`, so `fps == 5n + 2.5` — a
  frame rate with a half in it, which no camera reports. `pyfmt::round_index` is used because the
  *site* is one of the seventeen that round a frame index, not because this multiplication needs it.
- **`sort_unstable_by` passes everything, for the third time.** Rust's unstable sort is an insertion
  sort below 20 elements and `CHECKPOINT_REGISTRY` holds six, so it *is* stable at every length the
  engine reaches. P3 recorded this about `OrderedMap::sorted_by` and P5 about the same call; the
  assurance comes from the documented stability of `sort_by` and from no table, and that stays true
  until a seventh checkpoint becomes a twenty-first.

### What stayed in Python, on the rule §2 and P2 set

`align_swings` and the rest of `alignment.py` are P7's, gated by the `alignment` stage on the fifteen
corpus vectors — so `analyze_swing_bundle` **panics** on a bundle carrying a second clip rather than
answering for one camera. (P7 landed it and the panic is gone; the next addendum is that phase's.) That is the choice `engine.rs`'s `pivot_view` already made for the
down-the-line pivot source, and for the same reason: a bundle that quietly dropped its second clip
would return a *plausible* result with no alignment and no `_dtl` rows, and nothing in the workspace
could tell that from a bundle that never had one.

`_without_contradicted_scores` did not come either, and that is the same rule rather than a second
one: it is unreachable without `align_swings`, so it would ship with no gate a phase early.
`MEASUREMENT_REASONS` stayed too — its only reader is `tests/analysis/test_measure.py`, so a port of
it would be a table nothing in this workspace walks.

Two additions to `pyfmt` are worth recording because they are *not* new tables. `signed_fixed`
(`:+.Nf`, one site) and `percent` (`:.N%`, one site) are **compositions of the already-gated
`fixed`**: CPython renders the digits first and inserts the sign or appends the `%` after, so
`spec/vectors/format/fixed.json` gates every digit either can produce and a second family would
regenerate for no new answer. The three cases where that composition is not obvious — `-0.0` must
not gain a `+`, a NaN is `+nan` because CPython signs the word, and `:.0%` multiplies **before** it
rounds — are pinned as unit tests instead.

## Addendum, 2026-09-25b — the arbiter the corpus can no longer reach, and a second basis the corpus can

**M22 P7**, the second view. `alignment.py`'s reachable half, the down-the-line placements and pivot
rows, and `_without_contradicted_scores` — **1,197 lines of Rust before their tests against 1,070 of
Python**, green on the `alignment` stage and on the `measurements` stage's last two groups across all
21 vectors. `crates/analysis/tests/alignment.rs` is the sixth of seven stage runners; only `flight`
is left, and it is P8's.

The port itself held no surprises — 41 of 48 deliberate divergences were caught, and all seven
survivors are accounted for below. What the phase found is about the **corpus**, and it is the
sharpest instance yet of the thing §2's addenda keep restating.

### The corpus no longer contains a pair `_arbitrate_tops` can decide, and that is a fix, not a gap

M11 P7 installed an arbiter for a specific defect: the face-on top landing late on five of the eleven
bundles then on disk, measuring 0.183-0.267 s of downswing where down-the-line measured 0.367-0.484 s
of the same swing. It fires when the two views' downswing durations disagree by more than
`_DOWNSWING_AGREEMENT`'s 30%, and then names the shorter one as the late top.

Measured across the fifteen committed corpus vectors, **the widest disagreement is 27.6%**. On the
*unpinned* anchors it is 21.7%, and pinning tau=2 to the heard ball strike moves it to 27.6% — still
under the threshold. So `_arbitrate_tops` returns `None` on every committed vector, which takes with
it `_shared_tops`, `_top_at`, `_tempo_restated`, the `IMPACT_ONLY` tier, `ClipAlignment.top_late_by`
and **`engine._without_contradicted_scores`** — the only place in the engine where a cross-view
finding re-opens a face-on score.

That is a repair landing upstream rather than a hole: M10's windowing and M11 P10/P11's strike
selection between them removed the case the arbiter was written for. But it means a whole branch of
the second view ships against unit tests alone, and it means the *threshold itself* is now the thing
holding four functions off the gated path. `tests/alignment.rs::
no_committed_pair_disagrees_enough_about_the_downswing_to_be_arbitrated` records the two margins
rather than asserting the outcome, so the day a vector gets close it fails **with a number** and the
answer is to note that the ladder has become gated.

### Every corpus pair reports `SYNCHRONIZED`, so three of the five tiers are ungated

`_synchronized` runs last and overwrites whatever the anchor count came to, and all fifteen pairs
heard the strike in both clips. So `FULL`, `TOP_IMPACT` and `IMPACT_ONLY` never reach a committed
answer, and returning the constant `SYNCHRONIZED` from `align_swings` passes the whole family.

The decision *underneath* the tier is gated, which is the distinction worth keeping: the soft anchor
is accepted on seven pairs and refused on eight, and that reaches the payload through
`warp_motion_start` and through the notes. Four of the module's ten sentences are therefore compared
byte for byte on real footage. The other six are not, and `_synchronized`'s half-pair note is among
them — one clip hearing the strike and the other not is the ordinary case for a phone across the bay,
and the corpus does not contain it.

### The second trajectory basis is reached by four corpus vectors, where the first is reached by none

The exact mirror image of the 2026-09-24c addendum, and the reason both are worth stating rather than
one standing for both. Face-on, `build_trajectory` refuses all fifteen because the **trail** arm
crosses the torso past `MAX_MISSING`. From behind it is the **lead** arm that hides — and the
down-the-line fit drops it, adding both ankles instead. So the same footage that cannot be placed
face-on is placed from the rear on four of the fifteen, and those four are the only committed evidence
for that artifact's arithmetic; the other eleven gate the refusal.

Two smaller absences beside it, each pinned: **no two-camera vector is left-handed**, so the rear
basis's mirror runs on no committed swing (P5b's face-on counterpart *was* gated, by
`synthetic/face-on-only`); and **no vector carries a `down_the_line_window`**, so
`anchors_from_keypoints`' window branch has none either — which matters because that function
*refuses* an empty window where `engine._windowed` ignores one, a one-line difference between two
functions that look interchangeable.

### Forty-eight divergences, seven survivors, and the recurrences are the interesting half

41 caught. Of the seven survivors, **two are provable equivalences**: `frame >= pivot` against
`frame > pivot` in `tau_of_frame`, where both arms answer tau=1 at the pivot and the contract's own
validator makes both divisors non-zero; and `_relative_gap`'s zero guard, which is dead because both
arguments are strictly positive at all four call sites — and would be equivalent anyway, since a NaN
fails the same `>` the guard's `0.0` fails.

**Three are exact-equality boundaries on the three agreement thresholds** — `>` against `>=` at
0.35, at 0.167 and at 0.30. Each needs a float landing exactly on the constant, which no vector and
no constructible fixture here does; three corpus vectors sit at 0.1666… against 0.167, which is the
closest anything comes. This is the same class as the 2026-09-24b addendum's `observed > band.high`.

The last two are recurrences, and both are worth counting:

- **`{}` for `%g` survives for the fourth time**, now in `engine._dtl_placements`' two sentences. The
  edge is real (2026-09-24b), and it is invisible for the same reason in every module it appears in:
  a percentile is `round(x, 1)` clamped into `[10, 90]`, so it never reaches the exponent boundary.
  `spec/vectors/format/` is the only thing standing there, in four places now rather than one.
- **`sort_unstable_by` survives for the fourth time**, in `_without_contradicted_scores`' re-sort of
  `unscored`. Rust's unstable sort is an insertion sort below 20 elements and the list holds at most
  six, so the stability assurance comes from the documented call and not from any table — exactly as
  P3 recorded it.

Two survivors from the first sweep were **real gaps and were closed**: nothing checked that
`align_swings`' notes reach the bundle under the `alignment: ` prefix (the test's loop was vacuous,
because its fixture's two views agreed and produced no notes at all — the failure mode
`testing::a_swinging_body` already carries a warning about, met a second time), and nothing checked
that `analyze_swing_bundle` builds the `_dtl` rows from the **smoothed** rear clip and the **pinned**
anchors, because `run_stages` — and therefore the gate — does that setup itself before calling the two
group functions. Both now have tests, and the second is the shape of thing a per-stage gate is
structurally bad at seeing: the stage records the *inputs to* a call and the *output of* it, never the
two lines of the caller in between.

### What stayed in Python, and it is the render rather than a phase boundary

`frame_of_tau`, `map_frame`, `warp_speeds`, `_segment_rates`, `pair_frames`, `DEFAULT_TAU_RANGE` and
`_MAX_WARP_SPEED_ERROR` — 220 lines, the schedule a side-by-side video is drawn from. Their only
callers are `api/pipeline.py`, `scripts/` and `pose/side_by_side.py`, so
`conformance.py::run_vector` never reaches them and **no committed vector holds an answer for one**.
That is the rule P4 applied to `phases.py`'s clip-choosing half, and unlike P4's case it is not
waiting on a later phase: §7 retires those callers into the Flutter shell, which is where a render
schedule belongs.

It leaves one asymmetry that reads as an omission and is not: `tau_of_frame` is ported and its exact
inverse is not, because `align_swings` calls the first and only a renderer calls the second.

## Addendum, 2026-09-26 — the ball flight: a sixth edge, and it reaches a bool rather than a sentence

**M22 P8.** `benchmarks/flight_model.py`, `flight.py` and `spin_solve.py` in Rust — the published
constants, the RK4 integrator in three dimensions, and the spin recovered backwards out of a
printed carry. 1,570 lines before their tests against 1,598 of Python, and
`crates/analysis/tests/flight.rs` is the **seventh and last stage runner**: every stage in
`spec/vectors/stages/` now has one.

Written as the whole outcome and split under §M22's own rule, which takes the milestone to twelve
phases. `spin_solve.py`'s docstring argues for the same line — the forward model and the inverse
problem over it stay apart so the forward model's property, that every number is re-flown rather
than remembered, stays obvious — but the reason that decides it is P5's: the two halves have
**independent committed gates**.

### §3 is an edge short for the fourth time, and this one is not a string

`flight.py` takes the ball's speed as `math.hypot(vx, vy, vz)`. Rust's std has only the
two-argument form, so a port reaches for a chain or for `(x²+y²+z²).sqrt()` — and both sit within
**1 ulp** of CPython at every point of a committed flight. That is five orders inside `RTOL` and
looks like nothing.

One ulp is the whole difference. `carry_window` computes `high_plateau_min_rpm` *analytically* so
that the launch spin ratio lands **exactly** on the coefficient table's last row; CPython's
compensated norm gives the reference shot `40.546527999999995` where both approximations give
`40.546528`, and `AeroTable.coefficients_for`'s `>=` therefore clamps in one language and
interpolates in the other. `clamped` is a bool and §3 compares bools exactly.

So `pyfmt` gained CPython's `vector_norm` — Dekker's split, three Neumaier registers and a
compensated Newton correction — with a 32-case table of `math.hypot`'s own answers beside it.
**Where P3's three edges reach a sentence a golfer is shown, this one reaches a branch**, which is
the generalisation worth carrying: §3's list is not "the formatting edges", it is "the places
CPython computes something Rust computes differently", and a port that reads it as the first will
look for this one in the wrong file.

The **two**-argument sites in `measure.py` and `pivot.py` were deliberately left on `f64::hypot`.
This weakens the 2026-09-24c addendum's "provable equivalence" to "within a ulp" — but every one of
those answers reaches a float compared within `RTOL` and none reaches a bool, and moving them would
move numbers four green stages already agree on, for no gate.

### The oracle itself was being read a ulp wrong

Not the port: `serde_json`'s fast decimal path returns `0.03` for `"0.030000000000000002"` and
`14.46448727807716` for `"14.464487278077161"`, both of which `str::parse` and CPython get right.
The committed vectors are full of 17-significant-digit floats, so **9.2% of the values one flight
was compared against were the wrong f64** — absorbed in silence by a tolerance six orders above
them. `Cargo.toml` now takes serde_json's `float_roundtrip` feature and carries the measurement as
a comment. The correction applies to all six earlier stage runners as much as to this one, and
**none of them moved**, which is the only reason it reads as a near miss rather than a retraction.

### The census, and why a tolerance was not enough

With both fixed, **all 41,287 floats in the five committed flights come back bit-identical** — not
merely inside `RTOL`. That is a stronger statement than the phase set out to make, so the gate
keeps §3's tolerance *and* adds a census beside it with a 99% floor. A tolerance six orders above a
ulp is structurally unable to tell a converged integration from a systematically wrong primitive,
and the census is what caught the `hypot` edge. The floor is 99% rather than 100% because `exp`,
`cos`, `sin` and `atan2` are the platform's libm on both sides and another target may legitimately
move a last bit.

This is the general form of the 2026-09-24 addendum's finding, one layer down: a green gate
licenses less than it looks like, and the repair is a second measurement of the same run rather
than a tighter threshold.

### Three gates inside one stage, and coverage narrower than the count

The `flight` stage records `flown.resolved.launch` *and* `flown.flight`, and
`resolved.spin.solution.window.launch` *and* the whole `CarryWindow` — so the integrator is gated
**without the inference chain above it** and the solve **without the loft prior that reads it**.
It is the first gate here that reads a committed *input* beside its output, which is the exact
inverse of the 2026-09-25b addendum's complaint that a stage never sees the caller between two
calls.

What that buys is narrower than "21 vectors" suggests, again. **Ten of the fifteen corpus vectors
refuse before the integrator is reached** — `above_peak`, `between_plateaus` or `no_club_loft` — so
they gate the solve and never the flight. Of the five that fly, four are fully clamped for their
whole path and one, `2026-08-23-4`, is the only flight in `spec/` that reads an interpolated
coefficient row; nothing designed that, and it is pinned before a re-recorded corpus loses it.
**Three of the seven `SpinSolveCase`s reach a committed answer** (`above_peak`,
`between_plateaus`, `two_branches`); the other four ship against unit tests, and `below_floor` is
the one to know about, because it is the case ADR-027 §Decision 4 did not know existed.

The ported surface of a 416-line artifact reader turns out to be **seven numbers and eight rows**.
Everything else in `flight_model.py` is per-block provenance or the altitude what-if, whose only
caller is `scripts/simulate_flight.py --altitude`.

### One Python docstring is overstated, and was pinned as measured rather than corrected

`FlightResult.curvature_m` promises "exactly zero whenever the spin axis is zero, however far
offline the shot started". It is exactly zero only when the launch *direction* is zero too; with a
start line on it, `z*cos - x*sin` leaves the rounding residue of a rotation —
`-1.9539925233402755e-14` m on the reference shot, in **both languages to the bit**. The guarantee
a caller can rely on is the planar case, and the test says so rather than the docstring being
quietly rewritten, because the number is the evidence.

## Addendum, 2026-09-26b — the join, and the first gate that had to read the engine vector

**M22 P8b.** `shot_measure.py`, `flight_infer.py` and `flight_measure.py` — the shot's seven derived
tiles, ADR-027 §Decisions 3 and 5's resolution of the two launch conditions the screen did not
print, and the layer that decides which of the six flight numbers becomes a `Measurement`. 1,219
lines of Rust before their tests against 1,494 of Python, plus `engine`'s last two `measurements`
groups and the `fly_shot` call whose refusals extend `unscored`.

**§9's exit criterion is met**, twice over: all 21 engine vectors conform through `cargo test`, and
separately `golf-core run` was diffed against `conformance.py run` through Python's *own*
`compare_results` on all 21 at **zero differences**. Five of the twenty-one differ in output size by
one to three bytes, which is the 2026-09-23b addendum's no-byte-comparison finding standing exactly
where it was predicted.

### The stage vector cannot supply this phase's input, and that is what made it worth something

Each of P8's three gates hands a committed *intermediate* to the function under it, and its module
doc called that the phase's luck. **P8b is that luck running out.** `fly_shot` *is* the caller, so
its input is not in the stage at all, and `tests/flight.rs` had to read the engine vector for the
first time in five gates.

That is worth more than the inconvenience it caused. Until the two ends met, `resolved.launch` was
an assumption *shared* by the port and the gate — a port that built the wrong `UnspunLaunch` would
have passed all three of P8's cleanly, because the wrong value never had to travel anywhere. The
general shape: a per-stage family is a set of hypotheses about where the seams are, and it becomes
evidence only at the points where two of them are forced to agree.

### Three things earlier phases recorded that are false

Corrected here rather than left standing, because each was written down with confidence.

- **The loft prior is gated after all.** `flight_infer.py`'s docstring says no shot on disk carries
  a club, and `conformance_vectors._identity` has since resolved one from the swing manifest:
  **five** of the fifteen carry `loft_deg: 30.5` and two reach the comparison. What is still
  ungated is the floor's *value* — all five sit seventeen degrees above it, so moving
  `LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG` anywhere in `(0, 30.5)` passes all 21.
- **One corpus shot refuses a `shot` measurement.** `2026-08-23-3` printed a club path and no face
  angle, so the group is fourteen face-to-path readings against fifteen shots. That single vector
  is the whole of what stands between a port reading `and` for `or` in `measure_face_to_path` and a
  number it invents.
- **The 2026-09-25 addendum's "no vector moves an impact onto a strike" is now false.** Nineteen of
  the twenty-one produce that note. It was true of the six synthetic vectors P6 was gated by, and
  the corpus half landing turns it over — which is the shape of claim that goes stale when a gate
  widens, so `crates/core/tests/engine.rs` now *counts* it rather than asserting its absence.

### Twenty-seven divergences, twenty-six caught

The survivor is a provable equivalence: `solve_spin_from_carry` sets *both* branches to `peak_rpm`
at `AtPeak`, so returning the rising one is the same number. Two were caught only by unit tests
written in response to the sweep, and both are the kind a vector structurally cannot see — the
`between_plateaus` loft comparison is `>=` and only a club built to exactly 13.0° can tell `>` from
it, and a refused axis reaching the integrator through `or(shot.spin_axis)` passes everything
because every printed axis in `spec/` arrives with a golfer attached.

### What reaches no committed answer, measured and pinned

A **left-handed shot**: the one left-handed vector is synthetic and carries none, so
`infer_spin_axis`' mirror — the flip ADR-014's addendum had backwards for a milestone — ships
against unit tests. `engine::tempo_notes`' two sentences (the four committed notes mentioning tempo
are `alignment.py`'s). And a flight that is **both solved and curved**, the only combination in
which all six `flight_*` rows record at once, which this repo's data has never produced.

One more the stage family cannot hold at all: `2026-08-23-11` derives a draw against a `FADE` on the
screen, so `InferredSpinAxis::sign_disagrees` is true on a committed swing — and it is a *method*
rather than a field, so `run_stages` records neither it nor anything computed from it.

**Not ported**, per §8: `honest_test`, `gate_ordering` and M15 P16's `compare_to_printed` layer —
130 of `flight_measure.py`'s 475 lines — whose callers are `scripts/simulate_flight.py` and the
flight page, and which `run_vector` reaches not at all.

## Addendum, 2026-09-26c — the port is done, and what the whole of it measured

**M22 P9**, the docs cascade, and the addendum the Consequences section asked for: *"the port's own
numbers — which edges actually fired, what the stage family costs in bytes, whether any vector
moved — belong in an addendum written at the end."* Answering those, in order.

**Which edges fired.** §3 named three and the port found three more. The two predicted edges that
matter both fired: banker's rounding is real at forty-six sites, and `%g`/`.Nf` reaching a golfer's
sentence is real. The third predicted one — dict insertion order deciding a name — **never fired on
a committed vector**, because no two shares tie on this corpus; it is gated by unit tests and stays
in the list because a tie is data-dependent and a corpus is not a proof. The three that were not
predicted are the reason the list exists at all: `str()` on a bare band edge (P5), `max` returning
the first maximum rather than the last (P4 — and it lives in `phases.rs` rather than `pyfmt.rs`
because it is an iterator tie-break, not a formatting rule), and three-argument `math.hypot`
reaching a bool (P8). **Two of the three predicted edges collapsed into Rust's own exact
formatter** (P3), which is the cheapest correction in the milestone and the one most likely to be
re-derived by the next port that reads §3 without the addenda.

**What the families cost.** `spec/vectors/stages/` is **3.2 MB** across 21 files, 97% of it
`smoothed` and `flight`; `spec/vectors/format/` is **304 KB** for 3,059 cases and is the only family
that does not age on `ANALYSIS_VERSION`. `spec/vectors/` is **18 MB** in total. The stage family
paid for itself in the sense §2 argued it would — every phase after P3 landed against a committed
answer rather than a review — and the 2026-09-25b and 2026-09-26b addenda are the two places it did
not, both of them at a *caller*, which is the one shape a per-stage gate cannot see.

**Whether any vector moved.** **None.** `ANALYSIS_VERSION` is still 16, no committed vector was
re-recorded, and nothing under `src/golf_coach/` changed in P2 through P8b. §9 said a moved vector
would mean the port had found a Python bug — and that is a claim about the defects a port *can*
see. A port judged against an implementation cannot see a misunderstanding the two share, and the
two overstated docstrings P8 and P8b found — `FlightResult.curvature_m`'s "exactly zero" and
`flight_infer.py`'s "no shot on disk carries a club" — are what that looks like when it does
surface. Both were pinned as measured rather than rewritten.

**The crate count.** §1 said two, the seventh addendum amended it to four, and the workspace holds
**six**: `contracts`, `analysis`, `feedback` and `core` are the port, and `trigger` (M20) and
`capture` (M21) are edges that were already here. ADR-008 is a cargo edge for all of them.

**The size of it.** 9,969 lines of Rust across P4–P8b before their tests, against the 9,705-line
reachable Python surface §M22's table measured before anything was written. The two are not quite
counting the same boundary, which is rather the point: there is no ratio worth quoting, and the
estimate the *old* §M22 carried — "roughly a thousand lines" — was wrong by an order of magnitude
in the direction that costs a schedule. `crates/contracts` is 3,427 lines on top of that, and
`cargo test` runs **507 tests** across the six crates.

**The coverage, stated once in one place**, because it is spread over seven addenda above and
someone will want it as a list. Green on all 21 and gated by nothing but the port's own unit tests:
the trajectory basis on real swings (P5b); `head_stays_back`'s handedness mirror (P5);
`_arbitrate_tops`, `_top_at`, `_tempo_restated`, the `IMPACT_ONLY` tier and
`_without_contradicted_scores` (P7); three of the five sync tiers (P7); the rear basis's mirror and
`anchors_from_keypoints`' window branch (P7); four of the seven `SpinSolveCase`s (P8); the
spin-axis mirror on a left-handed shot (P8b); `_tempo_notes`' sentences (P6, P8b); `measure.py`'s
twenty-odd refusal branches (P4); and everything `EXCLUDED_FROM_RESULT` drops, which includes one of
`analyze_swing_bundle`'s own documented promises (P6). **The port is conforming, not covered**, and
the difference was measured in every phase rather than assumed in any of them.

**What this changes for §7.** Nothing yet, which is the point of its third clause. `analysis/` now
has a conforming Rust implementation and committed vectors that prove it — the first two clauses of
ADR-030's retirement rule — and it is not deleted, because 28 callers that stay Python still reach
it. The schedule holds: M24 gives the core its first real caller, M25 retires `api/` into the
shell, §M29 ports `mcp/` and does the delete. Until then two implementations stand, and the vectors
are what stops them drifting.

## Addendum, 2026-09-29 — who records each family, and the screen parser's edges are M34's

**M31 P5**, docs only. [ADR-034](034-shot-first-phone-first.md) moved the product onto the phone,
and the phone runs only Rust ([clause 6](034-shot-first-phone-first.md#6-the-phone-is-the-host)).
[ADR-030's 2026-09-29 addendum](030-app-platform-rust-core-python-sidecar.md#addendum-2026-09-29--the-machine-is-the-phone-what-adr-034-superseded-here-and-what-it-left)
left it to this ADR to say how the new work is gated. Three things follow, and none of them reopens
§1–§9: no vector moves, `ANALYSIS_VERSION` stays where it is, and §3's list stays at six.

### Every family names its oracle, because there are about to be three kinds

Until now there was one kind, because CPython recorded every family. The engine families
(`synthetic`, `corpus`, `stages`) come from `analysis/` through `conformance.py`, and `format` comes
from CPython's own formatter. `audio` comes from a detector M20 deleted, which is why `regenerate`
refuses to rebuild it. **None of the five says so.** Their `provenance` carries `kind`, a `note`
and what the input was cut from, and no `oracle` key (read 2026-09-29). The oracle is implied by
which runner wrote the file.

[ADR-034 clause 9](034-shot-first-phone-first.md#9-the-oracle-per-vector-family) makes it a choice
per family:

- **A port of existing Python is recorded from Python**, as M22 was. M34's screen parser and M36's
  many-shot layer are ports, so §2 and §7 apply to them as written.
- **New analysis is Rust first, against hand-worked vectors** (`provenance.oracle: "hand"`). That
  covers M37's strike profile, topic grades and blends. There is deliberately no Python to record
  from: one written first would be born to be retired on §7's own schedule. An answer worked by hand
  from the stated rule is independent of both languages.
- **M37's corpus vectors are recorded from the Rust core**, and that is the third kind. §7 says the
  engine family becomes a changelog and a regression suite after §M29, rather than an oracle. This
  family is that from the start, because it never had a Python reference.

So the kind has to be written down, because a regenerate means something different for each:

- re-recording a Python family from Python is §7's changelog;
- re-recording a hand family from Rust is the self-portrait `conformance_vectors._audio` refuses;
- a Rust-recorded family proves only that nothing moved.

**M37 adds the pin that every family names its oracle**, and the pin covers the five that exist.

**The lab reaches Rust-only analysis through `golf-core` subcommands**, never through a second
Python copy (clause 9). That is the `run` seam's shape (the seventh addendum), and it is the
subprocess M20 P5 put between `api/pipeline.py` and `crates/trigger`. For analysis that only Rust
has, this settles §7's open question in favour of the subprocess seam. For the scripts that reach
`analysis/` today, the question is still §M29's.

### The screen parser has edges of its own, and they are M34's list, not §3's

[Clause 7](034-shot-first-phone-first.md#7-ocr-on-the-phone) ports `launch_monitor/screen/`'s
parser and validator to a new `crates/screen` (M34). They are pure functions of OCR boxes and a
device profile, so the port is recorded from Python and gated the way M22's was. The program plan
found six places where CPython decides the answer
([finding 5](../plans/m31-m40-shot-first-pivot.md#what-the-code-says-before-anyone-re-derives-it)).
Each was re-read against the code on 2026-09-29, and the Rust side of 3, 4 and 6 was checked with
`rustc`:

1. **`difflib.SequenceMatcher.ratio`** scores every OCR label against every field
   (`ProfileField.matches`, `profiles.py`). Rust's standard library has no counterpart. The score is
   compared against a threshold and against the other fields' scores, so it must be reproduced
   exactly or labels land on different fields. `Impact Position` scores 0.9375 against
   `Impact Position V`.
2. **`_THOUSANDS` is a look-behind regex** (`parser.py`). The `regex` crate has no look-around, and
   the program plan keeps that crate out anyway, so this is hand-written.
3. **Float floor division.** `int(b.center_y // bucket)` sorts value boxes into lines. CPython's
   `//` is not the floor of the quotient: `1.0 // 0.1` is `9.0`, where `(1.0 / 0.1).floor()` and
   `f64::div_euclid` both give `10`.
4. **`{text!r}` in warnings**, which are compared exactly. `repr` picks its quote by content
   (`'Open'`, but `"it's"`). Rust's `{:?}` always writes double quotes, and it escapes in its own
   notation (`\u{1c}` where CPython writes `\x1c`).
5. **Two tie rules that point opposite ways.** `DeviceProfile.field_for` keeps the *last* field at
   a tie (`>=`), and `_find_labels` keeps the *first* box (`>`).
   - The second has fired on a real photo
     ([M31 P2 finding 2](../plans/m31-shot-first-adr.md#p2--found-2026-09-29)). OCR drops the
     `V` from `Impact Position V`, so two boxes score 1.0 for one field, and the first box wins it.
   - A port with the other tie-break passes every screen where nothing ties.
6. **Unicode `upper()` and `split()`.** Both depend on Unicode tables that CPython applies its own
   way. For example, a bare `split()` splits on U+001C–U+001F, and Rust's `split_whitespace` does
   not, because those four characters are not Unicode `White_Space`.

**They are not added to §3.** §3 lists the places where CPython computes something differently
*for the engine*, and [`docs/CONFORMANCE.md`](../CONFORMANCE.md) §3 is the copy an engine port
reads. `analysis/` reaches none of these six. Adding them there would send the next reader of the
engine looking for a `difflib` it never calls. They belong to the crate that hits them, and M34
records them beside its own vectors and the format tables it generates from CPython.

The parser also reaches two of §3's six edges. `validate_parse`'s warnings interpolate `:g` and
`.3f`, and `to_shot_data` calls `round(parsed.confidence, 3)`. The next section follows from that.

**What the screen family gates is the parser**: boxes in, `ShotData` out. The phone reads the
screen with Apple Vision, the lab reads it with PaddleOCR, and nothing makes their boxes identical.
Whether Vision's boxes parse as well is M33's question, which it answers by diffing each field
against the stored shots. No vector here answers it.

### `pyfmt` moves out of `analysis`, pending the ledger

`crates/screen` needs `pyfmt`, for the reason above, and it may not depend on `analysis`. The two
are siblings over `contracts`, which is ADR-008 as a cargo edge. So M34 plans to move
`crates/analysis/src/pyfmt.rs` into its own `crates/pyfmt`.

The cost of leaving it where it is already shows in the tree. `crates/feedback` cannot import
`analysis::pyfmt` either, so it re-spells `percent` for its one fallback call, and
`the_percent_fallback_agrees_with_pyfmt` holds the two spellings to one rule. One call can afford
that. A parser's worth of `%g`, `.Nf` and banker's rounding cannot, because a second copy of §3's
module is exactly the drift §3 exists to prevent.

**It is planned, not decided.** [`docs/REFACTOR_LEDGER.md`](../REFACTOR_LEDGER.md) has no row for
it (read 2026-09-29), and a structural change is checked against the ledger first. M34 either lands
it with a `Done` row or records why it did not. The move changes no function, so the format family
gates it without being re-recorded. It adds a crate to §1's count, which the seventh addendum has
already moved once.

### What this does not change

- **§3's six edges**, §2's rule that every stage is gated by a committed vector, and §9's criterion
  for the engine.
- **§7's retirement rule and its third clause.** The schedule holds under ADR-034's milestone names.
  - M24's first caller and M25's retirement of `api/` are re-scoped under M40.
  - §M29 is blocked on M40 rather than M25.
  - The core gains callers on the phone (M38) before it gains one on the laptop. Neither moves §7's
    clock, because the clause counts the Python callers that stay, and the phone has none.
- **The committed vectors.** No family moves in M31. The `profiles.json` comment in
  `tests/test_conformance.py` ("OCR stays Python") changes in M34, when the parser ports.

## Addendum, 2026-09-30 — Rust records from M32, and §7's schedule is §M29 to port and M40 to delete

**M31.5 P4**, docs only. [ADR-035](035-rust-everywhere-python-where-required.md) moves the oracle to
Rust and re-scopes §M29 as the port of the Python lab.
[Its clause 7](035-rust-everywhere-python-where-required.md#7-what-this-supersedes-sentence-by-sentence)
supersedes §7's schedule, and keeps §7's rule and §7's argument. The reasons are ADR-035's. This
addendum names the sentences above that stop holding. No vector moves, `ANALYSIS_VERSION` stays at
16, and §3's list stays at six.

### §7's third clause stands, and *stays* now means clause 1's list

**"…and nothing that stays Python calls it" is still the rule, and it still reads as a schedule.**
What changes is what *stays* covers.

- **§7 wrote "the sidecar", and ADR-030's first addendum spelled that as three files.**
  [Clause 1](035-rust-everywhere-python-where-required.md#1-the-rule-and-the-two-exceptions-it-names)
  now names every file. It covers the pose worker with its imports, and the LLM with its transcript
  store, its club lookup and the three shapes it owns.
- **None of those files imports `analysis/` directly** (read 2026-09-30).
- **The LLM's tools reach it indirectly, and §M29 replaces that route.** `scripts/ask_swing.py`
  builds them from Python `mcp/`, which calls `analysis/`, and hands them to `conversation.py`. The
  stdio route to the Rust MCP server replaces them (Q9 in
  [M31.5 P2's findings](../plans/m31-5-rust-first-replan.md#p2--found-2026-09-30)).
- **What `coach.py` imports from the ported half of `contracts/` goes too.** Under Q14, `coach.py`
  reads its swing and shot as JSON from the Rust MCP server instead.

### §7's schedule: §M29 ports, and M40 deletes

[Clause 5](035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped) supersedes
§7's bullet list:

- **"`api/` retires into the Flutter shell … `api/pipeline.py` is the exception … M24 is where the
  Rust core takes it over."** `api/pipeline.py`'s job goes to §M29's Rust lab CLI. `api/` itself is
  ported to `axum` or dropped in M40
  ([clause 2](035-rust-everywhere-python-where-required.md#2-everything-else-ports-including-the-three-things-considered-and-not-kept)).
  Until then the frozen `api/pipeline.py` keeps running as the FastAPI worker's pipeline.
- **"`mcp/` is ported to Rust" stands.** It happens in §M29, on `rmcp`, and `contracts/caveats.py` and
  `contracts/tool_descriptions.py` still go with it.
- **"`scripts/` is the blocker still open … some of `scripts/` is permanent by an existing decision"
  is dissolved.**
  - The fitting scripts are archived in §M29 rather than kept running, and the lab's entry points
    become the Rust lab CLI.
  - So the question §M29 was to open with has nobody left to ask it. That question was how fitting
    reaches a measurement once `analysis/measure.py` is gone.
  - An archived `golfdb/` runs until M40 deletes `analysis/`, and is a record after that
    ([P1 finding 11](../plans/m31-5-rust-first-replan.md#p1--found-2026-09-30)).
- **"§M29 is when it fires" becomes M40.** §M29 replaces the lab, and deletes only what the frozen
  FastAPI server does not import. M40 deletes the rest, `analysis/` included (Q17).
  - Three sentences above now mean M40 for the delete: §7's closing "§M29 is when it fires", the
    Consequences' "`analysis/` is deleted in §M29", and the eleventh addendum's "§M29 ports `mcp/`
    and does the delete". The Status block says so itself.
  - The twelfth addendum's "§M29 is blocked on M40" is superseded. §M29 now runs after M36 and
    before M40.

### Rust records from M32, not after §M29

**§7's "After M29, an `ANALYSIS_VERSION` bump re-records the engine family from the Rust core" is
brought forward to M32**
([clause 3](035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)).

- **§7's argument for it is unchanged, and clause 3 rests on it.** Once a port conforms, a re-record
  is a changelog and a regression suite, not an oracle.
- **Clause 3 makes mechanical the part §7 left to a person.** In §7, "a reviewer reads what moved".
  Under clause 3, a structural diff gate does. Every re-recorded output may differ from the committed
  one only by the keys or values the change declares. Anything else fails the re-record.
- **The gate is structural, never a text diff.** The second addendum found the reason: Python and
  `serde_json` disagree about exponents on 77 floats, so the text churns when nothing has changed.
- **What M32 builds for it** (P1 finding 3):
  - The engine family needs a thin re-record verb around `golf-core run`. `crates/core/tests/engine.rs`'s
    structural comparison moves into library code with it.
  - The stage family needs a Rust port of `conformance.py::run_stages` and its compose check first.
    §2's stage tests recompute each stage and compare it; no Rust code assembles a stage document.
  - `format` and `audio` need nothing.
- **`golf-core`'s module doc says the opposite**, and M32 rewrites it with the verb. It reads:
  "`regenerate` is deliberately not a candidate: the vectors are the oracle and a port that can
  rewrite them is a port that passes by construction". That was §7 before a port conformed.

### The twelfth addendum's kinds of oracle

- **"A port of existing Python is recorded from Python" now means once.** M34's parser and M36's
  aggregates and stores are recorded from the frozen Python before the port moves (Q7). Rust
  re-records them after that, diff-gated. Adding such a family to `scripts/conformance_vectors.py` is
  the one change the frozen lab is allowed
  ([clause 4](035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)).
- **The engine family joins the Rust-recorded kind at M32.** The twelfth addendum said M37's corpus
  vectors are that kind "from the start", and that the engine family becomes one after §M29. It now
  becomes one at M32.
- **"The lab reaches Rust-only analysis through `golf-core` subcommands" is superseded** (clause 7).
  The frozen lab reaches no new analysis at all, and from §M29 the lab is the Rust CLI. The twelfth
  addendum left one question with §M29: how the scripts that reach `analysis/` today reach it. The
  answer is that they are ported or archived.
- **One question the taxonomy does not answer, and M32 has to.** A family recorded by Python and then
  re-recorded by Rust under the gate holds Python's values, except where a change declared
  otherwise. So it is neither of the twelfth addendum's kinds as written. M37's pin will read its
  `provenance.oracle`, and M32 does the first such re-record, so M32's plan names what that field
  says.

### What this does not change

- **§1–§6, §8 and §9.** That includes §3's six edges and §2's rule that every stage is gated by a
  committed vector.
- **§5's one copy on disk.**
  - `crates/analysis` keeps reading the benchmark JSON by `include_str!` from
    `src/golf_coach/analysis/benchmarks/`.
  - M40 moves that JSON crates-side, in the change that deletes `analysis/` (clause 5). §5's rule
    survives the move; only the path changes.
- **§6's numeric-library ban in the scoring path.** `ort`, §M29's OCR runtime, sits outside it, as
  `rustfft` does.
- **The committed vectors.** They stay as the record of what Python said, because the gate lets a
  re-record change only what the change names (clause 3). The audio family stays frozen.
- **The twelfth addendum's list of M34's parser edges, and the planned `crates/pyfmt` split.**
- **The Consequences' "Drift between them is a real risk"**, which stays true in a new form. From M32
  the frozen Python and the Rust core disagree on purpose. Freezing is what keeps that disagreement
  to the declared diffs (ADR-035's Consequences).
