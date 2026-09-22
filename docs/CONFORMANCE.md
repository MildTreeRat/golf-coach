# Conformance: the Python core as a specification

> **Tier: AS-BUILT.** The schemas, vectors and runner described here are in the repo and run.
> The Rust core they exist to check is not — that is [M22](../ROADMAP.md).

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

Exported by `pydantic`'s `model_json_schema`, one file per root, `$defs` inlined. The roots are
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
a field added to `SwingResult` without a re-export fails at the commit rather than at the port. The
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

## 2. The vectors

Two families, covering different things.

**Synthetic** (`spec/vectors/synthetic/`) comes from `tests/analysis/conftest.py::make_swing`,
which is deterministic, RNG-free and pure stdlib — so it re-implements in another language exactly
and a port can generate its own inputs rather than trusting ours. Small, uncompressed, readable.
These cover the **code paths**: a window that has to be un-applied, a checkpoint that fails, a
checkpoint that cannot be scored at all, a bundle with no second view. Each vector's
`provenance.note` says which path it is there for; a case that only re-runs a path another case
covers is a vector that costs a regeneration and buys nothing.

**Corpus** (`spec/vectors/corpus/`) is the fifteen real swings on disk, gzipped. These cover what
synthetic input never can: real MediaPipe landmark noise, dropped-visibility frames, two genuinely
unsynchronised cameras, and a launch monitor.

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

**Strikes come from the stored `{role}.audio.json`, not from re-running `audio/impact.py`**, and
deliberately: `tests/audio/test_impact.py` synthesizes its clips from a seeded numpy RNG, so
detection has no reproducible oracle in the suite. The stored artifacts are the golden set, and
they are what the pipeline read. They are taken in file order and not sorted or deduplicated,
because `with_measured_impact` takes the strike nearest the pose impact and a reordering here is a
different tie-break there.

## 3. The rules

Implemented once, in `conformance.compare_results`, and unit-tested in `tests/test_conformance.py`
— because "it passed" from a comparator that cannot see a difference is worth nothing.

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

### Two known edges a Rust port will hit

Both were predicted when this milestone was written, and both are real in the shipped code:

- **Rounding.** Python's `round()` is half-to-even (banker's); Rust's `f64::round` is
  half-away-from-zero. `analysis/alignment.py` rounds a float frame count in every anchor
  fallback it has — `grep -n 'round(' src/golf_coach/analysis/alignment.py` — and `impact.py`'s hop
  size at 44.1 kHz lands on the same split. `round(x, ndigits)` is a second trap: it is
  decimal-aware and is **not** `(x * 10**n).round() / 10**n`. `benchmarks/joint.py` and
  `benchmarks/trajectory.py` use the two-argument form throughout, and those values reach
  `percentile`.
- **Float formatting reaches the strings.** `%g`, `.0f` and `.1f` are formatted into `message` and
  `detail`, which §3 compares exactly — `checkpoints/mechanics.py:225`, `:253`, `:269-271`,
  `:276-282` and `:476`, and `engine.py:114`. Rust's `{}` for `f64` is not `%g` and its `{:.1}` is
  not Python's `.1f` at a tie. A port that gets every number right and formats one of them
  differently fails on the sentence.

## 4. The commands

```bash
python scripts/conformance.py check                 # every committed vector, against this build
python scripts/conformance.py check --id corpus/2026-08-09-2 -v
python scripts/conformance.py list                  # what is committed, and where it came from
python scripts/conformance.py run < vector.json     # vector in, serialized result out
python scripts/conformance.py regenerate            # rewrite spec/ from contracts + data/
```

`check` and `run` need the base install and `spec/` alone — that is what committing the vectors
buys. **`regenerate` needs the capture machine**, because building corpus vectors reads
`data/processed/sessions/`, which is gitignored and exists nowhere else. It refuses to record a
swing whose stored analysis is behind `ANALYSIS_VERSION`; run `scripts/reanalyze.py` first.

`run` is the cross-language seam: a Rust implementation is diffed by a shell pipeline, with no
Python in the loop but the reference.

## 5. What ships in the app, and what stays in the lab

The inventory ADR-030 implies, made explicit — the answer to "does this module get rewritten?"

| Tier | Modules | Disposition |
|---|---|---|
| **1 — the swing loop** | `contracts/`, `analysis/`, `storage/`, `capture/`, `audio/impact.py` | **Ports to Rust** (ADR-030 §1). Gated by this suite. Already stdlib-only bar `impact.py`'s FFT, which is the one place the port needs a numeric library rather than arithmetic |
| **2 — shots, flight and clubs** | `launch_monitor/{mock,composite,source}.py`, `clubs/catalogue.py`, `analysis/flight*.py`, `analysis/spin_solve.py` | **Ports to Rust, after tier 1.** Committed JSON plus stdlib arithmetic already (ADR-022), so the data crosses unchanged |
| **3 — the Python sidecar** | `pose/estimator.py` | **Stays Python, bundled** (ADR-030 §2, §3). The load-bearing decision in the whole plan: `ranges.json` is cut from these landmarks |
| **4 — the lab** | `mcp/`, `api/`, `feedback/coach.py`, `clubs/lookup.py`, `launch_monitor/screen/`, `pose/{overlay,side_by_side}.py`, `scripts/` | **Stays Python, not shipped.** The reference implementation, the corpus tools, the model fitting and this oracle. `api/static/`'s pages stay a lab surface — M5 is superseded in shape by M25 |
| **stub** | `detection/` | Gated on M1.5's no-go. Nothing to port |

`feedback/rules.py` is tier 1 and is the reason `run_vector` makes **two** calls rather than one.
`analysis` may not import `feedback` (ADR-008), so `analyze_swing_bundle` leaves
`SwingBundleResult.feedback` as None and `api/pipeline.py:1259` fills it in a line later — meaning
the artifact this repo writes carries ranked tips and a bare engine call does not. Building the
vectors off the engine alone pinned `"feedback": null` on all twenty-one and would have told a port
to ship a results page with no coaching on it. They now carry the real payload, and
`_verify_against_stored` checks it against the archive like everything else.

`config.py::REPO_ROOT` assumes a source checkout and dies at packaging, which is tier 1's one known
unported assumption; it is in ADR-030's carried open questions.

---

## What this does not cover

Named rather than left to be discovered:

- **`feedback/coach.py`.** The *ranking* is covered; the LLM call is not and will not be — it is
  tier 4, it is non-deterministic, and `FeedbackPayload.coaching` is `None` on every vector.
- **`audio/impact.py` end to end.** The vectors take strike *frames* as an input, so a port's
  detector is unchecked. Checking it wants the waveforms, which are 4.9 GB of video away.
- **Pose itself.** Deliberately: ADR-030 §3 pins the sidecar against the 30 keypoint files already
  on disk, which is a different comparison with a different oracle.
- **Wall-clock performance.** No vector is timed. The charter's <15 s target remains unmeasured,
  as `ARCHITECTURE.md` §5 says.
