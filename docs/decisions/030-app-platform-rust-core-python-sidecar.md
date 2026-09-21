# ADR-030: The app platform — a Rust core, a Python pose sidecar, and a Flutter shell

## Status
**Accepted** 2026-09-21 — [ROADMAP §M18–M27](../../ROADMAP.md). **Supersedes
[ADR-001](001-language-python.md)** on the primary-language question and amends
[ADR-002](002-pose-estimation-mediapipe.md) and
[ADR-016](016-local-first-host-and-phone-upload-topology.md) by addendum. This document *is* the
M18 milestone: the phases it originally carried were five spikes, and the spikes were dropped
when the premise they were sized against changed. Nothing is built yet — M19 onward build it.

## Date
2026-09-21

## Context

Everything in this repo is an offline desk pipeline. Clips and a shot photo arrive by hand, a CLI
runs `analyze_swing`, and five hand-written HTML pages read the artifacts it leaves on disk. The
ask is an installed app: the golfer connects two cameras, verifies and positions them, starts a
session, and the app records continuously, cuts a clip around each ball strike, analyses each pair
in the background and updates their profile.

**The premise this was first planned against has changed.** The M18–M27 group was written on
2026-09-21 assuming *two iPhones must be enough on their own* — one recording, the other recording
**and** analysing. That single assumption drove almost all of its content, because an iPhone cannot
run MediaPipe-Python, OpenCV, pydantic-core or ffmpeg. It forced a Dart-versus-Rust bake-off, a
MediaPipe Tasks iOS spike, and one standing risk larger than either: **a different pose pipeline
means re-fitting every band in `analysis/benchmarks/ranges.json`.**

The premise is now: **the machine is a laptop.** A phone is a camera. Cloud analysis for people
without a capable laptop is not being built. That deletes the pose risk outright and most of the
spike programme with it, and it changes which question is actually hard. The hard question is no
longer *can the analysis run at all*; it is *what is this app written in, given that the analysis
already works and must keep producing identical numbers*.

**Three facts constrain the answer.**

**Pose is the load-bearing dependency, and it is Python-only.** Every band in `ranges.json` was cut
from MediaPipe's landmark output. This repo has already measured what swapping the pose engine
costs: [ADR-002](002-pose-estimation-mediapipe.md)'s 2026-08-02 addendum ran RTMPose through
`onnxruntime` against MediaPipe on 120 clips and it **lost by 24.7pp on event recovery**, with a
lead-wrist trajectory that correlated at a median 0.948 but ran 1.5× noisier at the wrist, 2.4× at
the shoulder and **4.3× at the hip**. That was a different model, not BlazePose-via-ONNX, so it does
not settle the narrower question — but it establishes the magnitude of the risk, and the risk is
paid in the scoring model, which is the part of this project that took the longest to earn. There
is no Rust binding for MediaPipe, official or mature.

**OpenCV does not force the same conclusion.** The `opencv` crate binds the same C++ library that
`cv2` binds. MediaPipe is the only hard Python dependency in the *shipped* path; everything else
Python does here — fitting under the `research` extra, the corpus tools, LLM coaching, screen OCR —
is either offline lab work or replaceable.

**The analysis core was built to be portable and nobody planned it that way.**
[ADR-008](008-project-structure.md) made `analysis/` depend on `contracts/` and the standard library
and nothing else, and [ADR-022](022-learned-artifacts-as-committed-data.md) made every fitted model
ship as provenanced JSON evaluated by stdlib arithmetic. The consequence is that the swing loop is
roughly **1,000 lines of logic that actually executes**, not the ~5,600 the import graph suggests —
`analysis/benchmarks/__init__.py` re-exports the flight, joint and trajectory models that the
checkpoint path never reaches. Two invariants written for other reasons turn out to be what makes a
rewrite affordable.

## Options Considered

### Option A: Python ships as the app's engine
Package the existing code behind a shell; no port at all.
- **Pros**: no second implementation to keep in sync; `ranges.json` keeps meaning exactly what it
  means; M22 collapses to packaging work; the fastest route to a working app by a wide margin.
- **Cons**: the app carries a Python runtime for everything rather than for pose alone; the
  concurrency story for *record while analysing* is the GIL and multiprocessing, which is the
  weakest part of the current `AnalysisWorker` (one job at a time, one process, no timeout, no
  retry, and the UI polls); and the phone app later has no shared core to reuse.

### Option B: A Rust core with a bundled Python sidecar for pose *(chosen)*
Rust owns the app process, capture, the trigger, the analysis engine and storage. A long-lived pool
of Python workers owns pose, and Python also keeps OCR, LLM coaching and the whole lab.
- **Pros**: real concurrency for capture-while-analysing, which is the thing the live session
  actually needs; **the pose engine is untouched, so every band survives the rewrite without
  re-validation**; the core compiles into the phone app later, so capture and strike detection are
  shared rather than written twice; a single binary for everything except the one component that
  genuinely needs an interpreter.
- **Cons**: it is a second implementation of ~1,000 lines of logic and it can drift from the
  Python one — which is why M19 is promoted to a prerequisite rather than a nicety. It ships two
  runtimes. And it supersedes a founding decision, which is a heavier move than this repo usually
  makes.

### Option C: A Rust core with pose in Rust, via ONNX Runtime
Run the BlazePose weights through the `ort` crate; ship no Python at all.
- **Pros**: one runtime, one language, the smallest installer; no process boundary anywhere.
- **Cons**: MediaPipe is not a model, it is a *graph* — detector → region-of-interest → landmark,
  with cross-frame tracking in `RunningMode.VIDEO`, which ADR-002 names as the likely reason
  RTMPose lost. Reimplementing that pipeline puts every band back in question and buys a smaller
  download. The trade is the wrong way round: the bands cost a corpus of 1,399 hand-annotated tour
  swings ([ADR-012](012-golfdb-reference-data.md)), and the interpreter costs disk.

### Option D: A Rust core calling MediaPipe's C API by FFI
Keep the real MediaPipe graph, reach it from Rust without Python.
- **Pros**: identical landmarks and no interpreter — the best of C and B if it works.
- **Cons**: the least certain option on the board. Rust bindings are community-grade at best and
  building MediaPipe means Bazel. It is the right thing to revisit if the sidecar's process
  boundary ever becomes the bottleneck, and the wrong thing to bet a milestone on now. Recorded in
  *Deferred, by choice* rather than rejected.

## Decision

### 1. The backend is Rust

Rust owns the app process, session orchestration, camera capture, the ring buffer, clip cutting,
audio strike detection, phase segmentation, measurement, checkpoint scoring, storage and the IPC
layer. This **supersedes [ADR-001](001-language-python.md)**, which chose "Python for all backend,
ML, and data processing" and rejected Rust as *"Much slower development cycle. ML ecosystem is
secondary. Overkill for a home lab project."*

That rejection was correct for what it judged and does not survive the change of subject. ADR-001
was choosing a language for a **desk pipeline** in March, where development speed dominated and
nothing had to be installed by anyone. It is not a ruling about a shipped application that records
two camera streams continuously while analysing a third thing in the background. What survives of
ADR-001 is its actual finding — **the ML and CV ecosystem is Python's** — and that finding is
precisely why Decision 2 keeps pose there. What does not survive is "all backend".

ADR-001's other clause, *"JavaScript/React for the web UI only"*, never happened: the UI that got
built is five hand-written HTML pages with no framework and no build step, a choice
[REFACTOR_LEDGER](../REFACTOR_LEDGER.md) has twice declined to revisit. Decision 4 replaces the
clause rather than fulfilling it.

### 2. Pose stays MediaPipe, in Python, unchanged

The model files, the variant (`settings.pose_model_variant`), the `.task` bytes, the Tasks API and
`RunningMode.VIDEO` are all exactly what they are today. **What changes is the process that calls
them, and nothing else.** `analysis/benchmarks/ranges.json` therefore needs no re-validation, no
re-fitting and no addendum; the bands are measuring the same instrument they were cut from.

This is the whole reason Option B beat Option C. Pose is not a component here, it is the
calibration of everything downstream.

### 3. Python runs as a long-lived, bundled worker pool

Not spawn-per-job. Loading a `PoseLandmarker` is expensive — the heavy bundle is ~30 MB — so the
pool keeps warm *processes*, each with the interpreter up and the model bytes resident.

**A worker takes one clip start to finish.** `RunningMode.VIDEO` carries cross-frame tracking state
between calls and `pose/estimator.py` forces strictly-increasing timestamps, so clips cannot be
interleaved on one landmarker instance and a worker gets a fresh landmarker between jobs. It is the
process that is warm, not the landmarker. That is still the bulk of the win.

**The job moves a path, not pixels.** The pipeline already cuts each swing to disk, so the envelope
is `{job_id, clip_path, frame_range, camera_id, pose_model_variant}` and the worker decodes its own
frames. The reply is a `KeypointsFile` exactly as `contracts/keypoints.py` already defines it,
`pose_estimator` stamp included — so the sidecar's output is byte-comparable against the 30
keypoint files already on disk, which is how the port gets verified rather than trusted.

**Zero-copy is available and not needed yet.** If live-preview pose ever wants frames that are not
on disk, the answer is shared memory — Rust writes an mmap'd ring buffer and passes
`{shm_name, offset, len, stride, format}`; Python reads the same pages through
`multiprocessing.shared_memory` and `numpy.frombuffer`, which works on Windows and POSIX. Recorded
here so the protocol leaves room for it; not built.

**Throughput is the reason any of this matters.** ADR-002 measured heavy at ~24 fps, so a 10 s
60 fps clip is roughly 25 s of pose and a two-view swing roughly 50 s. The pool and background
analysis exist to hide that number, not to eliminate it.

### 4. The shell is Flutter, over the Rust core

Flutter for the UI, `flutter_rust_bridge` to the Rust core compiled in as a native library. It
targets Windows, macOS and Linux — which .NET MAUI does not, officially — and it is the only choice
on the board that also reaches the phone, which matters for Decision 5 and for nothing else today.

**There is no web UI.** This is not the existing HTML pages in a window, and not a browser. M5's
web feedback UI is superseded in shape, and the five pages in `api/static/` remain what they are: a
lab surface for a desk pipeline, still useful, not the product.

### 5. The phone is a camera, and its app comes after the laptop's

The phone app is real and wanted, and it is **not** in the first build. The laptop app has to work
end to end first — capture, trigger, analyse, profile — because until it does there is nothing for
a phone to feed and no way to test what it sends.

When it lands it carries **capture, recording and strike detection, and never pose**. Those are
already Rust by Decision 1, so the phone app is the same core with a different shell rather than a
port. Pose and analysis stay on the laptop.

**This supersedes ADR-016's "no phone app" clause and nothing else in it.** The rest of ADR-016 is
*reinforced* here: no cloud, no open router port, clips never leaving hardware the golfer owns.

**Video crosses Wi-Fi, encoded and triggered, never raw or continuous.** Because strike detection
runs on the phone, it sends ~10-second clips at roughly 10–20 MB each rather than a stream; raw
1080p60 would be ~180 MB/s and the phone's hardware encoder makes that ~1–2 MB/s for free. The two
views need no shared clock in flight, because [ADR-025](025-acoustic-synchronization.md) already
recovers the offset acoustically after the fact.

### 6. Capture sources are pluggable, and a file is the first one

`capture/source.py::VideoSource` is already a `Protocol` with a `FileVideoSource` adapter built and
a `LiveCameraSource` that has never existed. That shape carries over, and the order is deliberate:

1. **File / upload** — works today, needs no hardware, and is how the whole system stays testable
   before a single camera is bought.
2. **USB / UVC webcam** on the laptop.
3. **Phone over Wi-Fi**, per Decision 5.

**Upload is a first-class source, not a test fixture.** It must still work when the other two exist.
A golfer with footage and no rig is a supported case, and it is also the only source that can
replay the corpus already on disk through the new stack.

### 7. Cloud analysis is not being built

M27 closes. Not deferred, not blocked on demand — closed, with this as its record. Remote analysis
brings accounts, authentication, per-swing compute cost, video retention and a public endpoint, and
every one of those is a project rather than a phase. It also contradicts the local-first posture
[ADR-014](014-screen-capture-shot-ingestion.md) and [ADR-016](016-local-first-host-and-phone-upload-topology.md)
both rest on.

The cheaper lever comes first and is still untried: ADR-002 measured `lite` at roughly four times
`heavy`'s speed with no significant difference on event recovery across twelve paired McNemar tests.
A golfer whose laptop is too slow gets a faster variant and shorter clips before anyone stands up a
server.

### 8. The port is gated by a conformance suite, not by review

**M19 is now a prerequisite, not a parallel track.** A Rust core and a Python core that disagree
silently is the failure mode this whole decision has to survive, and code review does not catch a
0.29-yard drift. M19's JSON Schemas, golden vectors and `scripts/conformance.py` are what M22 is
checked against, clause by clause, and they must exist first.

> **Built, 2026-09-21.** M19 closed the same day this ADR was accepted: `spec/schemas/` (five
> roots), `spec/vectors/` (6 synthetic + 15 corpus, 8.3 MB gzipped) and
> `python scripts/conformance.py check`, with the rules in
> [docs/CONFORMANCE.md](../CONFORMANCE.md). Two things it found are corrections to this section's
> assumptions rather than confirmations of them, and both are M22's to carry: the serialization a
> port must match is an `exclude=` set at a **call site** in `api/pipeline.py` and appears in no
> schema; and `analyze_swing_bundle` alone does **not** produce the artifact this repo writes,
> because ADR-008 forbids it importing `feedback` — so the conformance runner makes two calls, not
> one. §5's inventory promise is [CONFORMANCE.md §5](../CONFORMANCE.md#5-what-ships-in-the-app-and-what-stays-in-the-lab).

Two assets already exist and should be used rather than rebuilt:
`tests/analysis/conftest.py::make_swing` is deterministic, pure-stdlib and RNG-free, so it
re-implements in Rust exactly; and the **15 stored swings with 30 keypoint files and 30 audio
files** are a real-capture golden set that needs no new fixtures.

## Consequences

- **This is the largest-scope decision in the repo and it builds nothing.** Every milestone from
  M19 to M26 is rewritten by it; no line of source changes on the commit that lands it.
- **Two implementations of the analysis core will exist, and the Python one stays.** It is the
  reference, the oracle and the lab — [ADR-022](022-learned-artifacts-as-committed-data.md)'s
  offline fitting is unchanged and still Python under the `research` extra. "Which one is right" is
  answerable only because M19 makes it answerable.
- **`ranges.json` is untouched, and that is the point.** The one thing that could have invalidated
  the scoring model — a different pose pipeline — is the one thing this decision refuses to change.
- **[ADR-001](001-language-python.md) is superseded**, the first superseded ADR in the repo.
- **[ADR-002](002-pose-estimation-mediapipe.md) and
  [ADR-016](016-local-first-host-and-phone-upload-topology.md) gain addenda.** 002 keeps its
  decision entirely and gains a caller; 016 loses one clause of one bullet and keeps everything
  else.
- **The charter's "Mobile app" out-of-scope line moves into scope.** `Multi-user support` does not —
  nothing decided here needs it, and the app is still one golfer's.
- **The stdlib-only invariant becomes a two-language invariant.** `CLAUDE.md` says the analysis core
  is stdlib + `contracts` only; that stays true of the Python core and becomes a rule the Rust core
  inherits (no numpy-equivalent in the scoring path). M22 is where that gets restated properly.
- **`config.py::REPO_ROOT` assumes a source checkout** and every path constant flows from it. That
  assumption dies at packaging time (M26) and the sidecar inherits it in the meantime.

## Deferred, by choice

- **MediaPipe by C FFI from Rust** (Option D). The right answer if the sidecar's process boundary
  ever becomes the bottleneck, and not worth a Bazel build to find out before it is.
- **Pose on the phone.** Only reachable through MediaPipe's native Tasks SDKs, which is the iOS
  spike this milestone dropped. Revisit only if the laptop stops being required.
- **Shared-memory frame handoff.** Specified in Decision 3, unbuilt. Nothing needs it until live
  preview does.
- **Live-streaming video from the phone.** Triggered clips are enough and cost two orders of
  magnitude less bandwidth. Only a real-time overlay would justify revisiting it.
- **Multi-user support.** Stays out of charter scope. The corpus, the profile and the bag are all
  one golfer's today, and nothing above changes that.
- **Cloud analysis** (Decision 7). Closed rather than deferred — it needs its own ADR and a reason,
  and it has neither.
