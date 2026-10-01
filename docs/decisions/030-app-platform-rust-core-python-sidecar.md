# ADR-030: The app platform — a Rust core, a Python pose sidecar, and a Flutter shell

## Status
**Accepted** 2026-09-21 — [ROADMAP §M18–M27](../../ROADMAP.md). **Supersedes
[ADR-001](001-language-python.md)** on the primary-language question and amends
[ADR-002](002-pose-estimation-mediapipe.md) and
[ADR-016](016-local-first-host-and-phone-upload-topology.md) by addendum. This document *is* the
M18 milestone: the phases it originally carried were five spikes, and the spikes were dropped
when the premise they were sized against changed. **Four of the milestones that build it are
done**: M19 made the Python core a specification, M20 ported the strike detector and deleted its
Python (which is what the first addendum below is about), **M22 ported the whole analysis
engine** — all 21 committed vectors conforming, and wired to nothing until M24 — and **M23 built
§3's worker pool**, whose protocol is [ADR-033](033-the-pose-sidecar-protocol.md). §1's Rust core
and §3's boundary to the sidecar therefore exist, both with no caller above them; §4's session
engine and §5's shell do not. M21 is part-built.

**Partially superseded by [ADR-034](034-shot-first-phone-first.md)** 2026-09-29: §5, and the
premise the Context states, *the machine is a laptop*. The phone is now the host and the laptop a
later client, so M21 and M24–M26 are paused and re-scoped under M40. The rest of this ADR stands,
and §7 is reinforced.

**Superseded in part by [ADR-035](035-rust-everywhere-python-where-required.md)** 2026-09-30: the
first addendum's rule, the survivor list it carries, and the schedule on which the lab
retires. Python stays only where it is required, which means MediaPipe and the LLM. The
lab is ported in §M29 rather than kept, and Rust records the vectors from M32. The
Decision section stands as ADR-034 left it.

**Four addenda, and read the second before budgeting anything against §3.** M23 P1 measured pose on
this repo's own footage and it runs at **9.2 fps, not ~24** — so §3's "a two-view swing is roughly
50 s" is about 2.5× optimistic for 4K portrait phone video. The protocol §3 left open is
[ADR-033](033-the-pose-sidecar-protocol.md). **Read the third before building on §1, §2, §5 or §6**:
it says clause by clause what ADR-034 superseded, amended, reopened and left alone. **Read
the fourth before relying on the first**: it names the sentences of the first and the
third that ADR-035 replaced.

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

  > **Amended 2026-09-22 by the addendum below.** A ported module's Python original is *deleted*,
  > not kept, once the vectors that prove the port exist. The oracle survives as `spec/`, which is
  > the artifact, rather than as the code that recorded it.
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

---

## Addendum, 2026-09-22 — the retirement rule, and the first module to meet it

**M20 moved ball-strike detection to Rust and deleted the Python that had done it.** Decision 1
already gave Rust "audio strike detection" and [CONFORMANCE.md](../CONFORMANCE.md) §5 already
filed `audio/impact.py` as tier 1, so *what* moved is not new. What is new is that the original
was removed rather than kept beside it, which amends this ADR's second Consequence, and the rule
that made that safe.

### The rule

> **Python keeps only what does not translate.** A module is retired from Python once a conforming
> Rust implementation exists *and* the golden vectors that prove it are committed. The vectors are
> the oracle, not the code that recorded them.

"Does not translate" is narrower than it first reads, and the narrowness is the point. It means
**MediaPipe** — a graph, not a model, with no mature Rust binding, and the instrument every band in
`ranges.json` was cut from (Decision 2). It does not mean "uses numpy". `impact.py` was numpy, and
a spectral flux detector is portable arithmetic plus one FFT; `rustfft` reproduced it to a worst
relative difference of **7.7e-15**, six orders of magnitude inside `CONFORMANCE.md` §3's tolerance.
The lab stays Python too — tier 4 — because it is not shipped, not because it could not be ported.

> **Amended 2026-09-23 by [ADR-032](032-the-rust-core.md) §7.** "Stays" was doing more work than it
> can. The lab is not shipped *and is not permanent*: §M29 retires `api/` into the Flutter shell and
> ports `mcp/` to Rust, because the rule above cannot fire on `analysis/` while twenty-eight lab
> modules import it. The clause the rule was missing is **…and nothing that stays Python calls it**,
> where *stays* means the sidecar — pose and the LLM — and not *not ported yet*. What survives is
> `pose/estimator.py`, `feedback/coach.py` and `feedback/conversation.py`, and nothing else.

### Why deleting is safe, and what it costs

Keeping both implementations sounds strictly safer and is not. A second copy nothing runs is a
second copy that **drifts**: a constant changed in one and not the other is invisible until a
number moves, which is exactly the silent disagreement §8 exists to prevent. One implementation
plus a committed oracle has fewer places to be wrong than two implementations and a hope.

It costs two things, and both are load-bearing:

1. **Coverage has to be right before the delete, because there is no second chance.** Once the
   Python is gone, nothing can regenerate what the vectors failed to capture. M20 built the oracle
   first (P0) and deleted last (P6), and it covered *both* halves of `impact.py` — `detect_strikes`
   on thirty clips and `offset_between` on the fifteen two-view swings — even though the second
   has no caller in this repo at all, precisely because "no caller today" is not "no caller ever".
2. **The vectors stop being regenerable, and that is now enforced.**
   `conformance_vectors._audio` returns nothing and explains why. Rebuilding them from the Rust
   detector would replace a reference-derived oracle with a **self-portrait** — it would pass by
   construction and detect nothing. If `AUDIO_DETECTOR_VERSION` ever moves, re-recording is a
   decision needing a new oracle named first, not a script anybody can run.

### What this does not change

`regenerate` still builds the synthetic and corpus families from Python, because the engine has
not been ported yet — M22 is when that same question arrives for `analysis/`, at far greater
scope. The rule above is what M22 inherits, and the order it implies is the order M20 used:
vectors, port, conform, then delete.

### The seam M20 drew, and the one it did not

Python still **decodes** and Rust **detects**. `audio/ffmpeg.py` holds what M11 paid for — container
edit lists, the two `soun` tracks the face-on clips carry, the `video_start_seconds` probe that was
a 105–125 ms surprise on four clips — and it reaches ffmpeg through the binary the `imageio-ffmpeg`
wheel ships rather than a system install. It is tier 1 and unported, and naming it here is the
point: **`CONFORMANCE.md` §5's inventory listed `audio/impact.py` and not `audio/ffmpeg.py`**, so
the decoder was tier 1 by implication and in no table. It is in one now.

The call is a **subprocess**, not a native extension: `audio/ffmpeg.py` already shells out in that
same module, and a PyO3 build would make `pip install -e '.[audio]'` require a Rust toolchain,
which is a real regression for a lab install that mostly reads artifacts. `config.py::REPO_ROOT`'s
source-checkout assumption is inherited by the binary lookup and dies at packaging with everything
else (M26).

---

## Addendum, 2026-09-26 — §3's throughput budget is 2.5x optimistic, measured

**M23 P1 measured pose on this repo's own footage before writing
[ADR-033](033-the-pose-sidecar-protocol.md), and §3's last paragraph does not survive it.** That
paragraph says:

> Throughput is the reason any of this matters. ADR-002 measured heavy at ~24 fps, so a 10 s 60 fps
> clip is roughly 25 s of pose and a two-view swing roughly 50 s.

The **conclusion is reinforced** and only the arithmetic changes — the pool and background analysis
exist to hide this number, and there is more of it to hide than §3 thought.

### What was measured

`mediapipe:heavy` over `data/processed/sessions/2026-08-23/11/face_on.mov` — 344 frames at
2160x3840 portrait, 59.965 fps — took **37.3 s, or 9.2 fps**. So a 10 s 60 fps clip is roughly
**65 s** of pose and a two-view swing roughly **two minutes**, not 50 s.

Two things make this a correction rather than one contradictory sample:

- **`api/worker.py`'s docstring already said so**, independently: *"about 9.5 fps"* for a 4K clip,
  written when the worker was built and never reconciled with this section. Two Python measurements
  agree with each other and disagree with the ADR.
- **ADR-002's ~24 fps is not wrong, it is about different footage.** The bake-off ran GolfDB
  reference clips, which are small; this corpus is 4K portrait phone video, which is roughly nine
  times the pixels per frame. The number that belongs in a *budget* is the one measured on the
  footage the app will actually be handed.

### What it changes

**Nothing in the decision.** Option B still beats Option C on §2's grounds — the bands, not the
speed — and §7's cheaper lever is if anything more attractive: ADR-002 measured `lite` at roughly
four times `heavy`, and four times 9.2 fps is the difference between a two-minute swing and a
thirty-second one.

What it does change is every number downstream that was sized against 25 s per clip:

- **The corpus is ~77 minutes of pose to re-run**, not ~30. Thirty stored keypoint files, 42,648
  frames, 344 to 4,837 each. §M23's claim that *"verification is free"* was true of the diff and
  false of the re-pose, and M23 P1 corrected that sentence too.
- **A flat timeout cannot serve this.** The 14.1x spread between the shortest clip and the longest
  is why [ADR-033](033-the-pose-sidecar-protocol.md) clause 6 derives the deadline from the clip's
  frame count, and why the worker has to report that count before posing — which is the one message
  ADR-033 adds beyond the request/reply pair §3 implies.
- **Pool sizing is a measurement, not a pick.** At 9.2 fps pose is CPU-bound with no headroom to
  speak of, so a second concurrent worker may buy very little; M23 P8 measures it rather than
  choosing a number here.

  > **Measured 2026-09-27 by M23 P8, and this bullet's guess was wrong** — which is the argument for
  > having made it a measurement. A second worker buys **1.71×** and a fourth **2.34×**, because one
  > worker occupies only **1.63 cores**: MediaPipe is internally threaded and never saturated this
  > box, so "CPU-bound" was being read as "one core busy" when it never was. The default is **2**,
  > capped by memory rather than by throughput — the incremental worker is **~1.3 GB** resident, so
  > four is 5.1 GB against a laptop that also has two captures running. The table is in
  > [ADR-033](033-the-pose-sidecar-protocol.md)'s fourth addendum and in
  > [§M23](../../ROADMAP.md#m23-the-pose-sidecar--a-long-lived-python-worker-pool); the re-pose of
  > the whole corpus ran at **9.5–10.7 fps** per clip, so this addendum's 9.2 is conservative and
  > everything it concludes about the budget stands.

`audio/trigger.py`'s docstring says the pipeline *"already spends ~25 s per clip in pose"*, which
inherited this figure. It is left as written: it is an argument about a subprocess spawn being cheap
by comparison, and it gets stronger, not weaker, at 65 s.

---

## Addendum, 2026-09-29 — the machine is the phone: what ADR-034 superseded here, and what it left

**[ADR-034](034-shot-first-phone-first.md) moved the product from the swing video to the
launch-monitor shot, and the host from a laptop to a standalone iPhone.** It is the second time this
ADR's premise has moved, and the Context above records the first: M18–M27 were planned against *two
iPhones on their own*, and this ADR was written when that became *the machine is a laptop*. ADR-034
does not go back to the first premise. What made that one expensive was asking a phone to run
MediaPipe-Python, OpenCV, pydantic-core and ffmpeg, and the phone is now asked to run the **Rust
core** — the thing §1 built, and the thing ADR-008 and ADR-022 had made portable before anyone
planned to port it. The reasons are ADR-034's and are not restated here; this addendum says which
sentences above no longer hold.

### Superseded: §5, and the premise that the machine is a laptop

§5, *"the phone is a camera, and its app comes after the laptop's"*, is superseded whole. The phone
app comes **first** and is the product ([ADR-034 clause 6](034-shot-first-phone-first.md#6-the-phone-is-the-host)),
and the laptop is a later client
([M40](../plans/m31-m40-shot-first-pivot.md#m40--the-laptop-client-resumes)). §5's reason for its
order was that until the laptop app worked *"there is nothing for a phone to feed"*. That was true
of a video product whose analysis ran on the laptop. A shot-first phone feeds nothing: the photo is
read, analysed and stored on the phone that took it.

The Context's *"The premise is now: the machine is a laptop"* goes with it. What that premise
deleted was the pose risk, and it comes back **scoped to optional video** rather than to the
product, behind a gate (below).

Two parts of §5 outlive it, for reasons of their own:

- **Its Wi-Fi arithmetic** (triggered clips of 10–20 MB, not a raw stream at ~180 MB/s) has no
  caller now. M28, which was to build phone-over-Wi-Fi, is superseded by M39, where the phone keeps
  its own clip. The measurement is still right, and [ADR-031](031-the-capture-edge.md) §6 still
  sizes the laptop's ring by it.
- **"This supersedes ADR-016's 'no phone app' clause and nothing else in it"** stays true of *this*
  ADR. What ADR-034 changes in ADR-016 is recorded in ADR-016.

### Amended: §1 — storage is on the device too

§1 gives Rust *"… storage and the IPC layer"*. On the phone, "storage" is the phone's own: nothing
on it listens on a port, and export is how data leaves it (ADR-034 clause 6). The IPC layer §1
names is the sidecar's ([ADR-033](033-the-pose-sidecar-protocol.md)), and the phone has **none**,
because it has no Python. `flutter_rust_bridge` is an in-process call. The rest of §1 stands.

### Reopened, for the phone only: §2, behind M39's gate

§2, *"Pose stays MediaPipe, in Python, unchanged"*, **still holds on the laptop**, word for word.
It cannot hold on the phone, which has no interpreter.
[ADR-034 clause 8](034-shot-first-phone-first.md#8-pose-on-the-phone-is-reopened-behind-a-conformance-gate)
reopens pose there on §2's own terms:

- it is **MediaPipe's graph**, through iOS's Tasks `PoseLandmarker` and the same pinned `.task`
  bytes, not a reimplementation of it — so Option C's objection is met rather than overruled;
- the gate is **M39 P0**, and it protects §2's reason: pose is the calibration of everything
  downstream, so `ranges.json` is untouched whether the gate passes or fails;
- **if it fails**, the phone records, and mechanics are computed on the laptop (M40), through the
  sidecar §3 built.

[ADR-002](002-pose-estimation-mediapipe.md)'s 2026-09-29 addendum records what the gate asks of the
pose decision itself.

*Deferred, by choice*'s **"Pose on the phone … Revisit only if the laptop stops being required"**
has had its condition met. It is
[M39](../plans/m31-m40-shot-first-pivot.md#m39--optional-video-on-the-phone-mac-then-bay) now.

### Re-scoped: the milestones that were building this ADR

- **M21, M24, M25 and M26 are paused** and re-scoped under M40, as the desktop target of the same
  Flutter app. M26 moves whole, and the last Consequence's `config.py::REPO_ROOT` goes with it: that
  assumption now dies at M40's packaging.
- **M28 is superseded by M39.**
- **M29 is blocked on M40**, not M25. Its job is unchanged: it retires `api/` and `mcp/`, so that
  [ADR-032](032-the-rust-core.md) §7's clause can fire on `analysis/`.

`ROADMAP.md` records each of them (M31 P8–P9).

§4 reaches the target it was chosen for. It said the phone *"matters for Decision 5 and for nothing
else today"*. The phone is now the first target rather than the last, and the desktop targets are
M40's.

§6's list of capture sources is the laptop's, and it splits three ways:

- **File/upload** stays first-class, for §6's reason: it is the only source that replays the corpus.
- **The USB/UVC webcam** moves with M21 to M40.
- **Phone over Wi-Fi** is superseded with M28.

On the phone, the shot photo is the first source and a video clip is optional (ADR-034 clause 1).

### Reinforced: §7

Cloud analysis stays closed, and the pivot strengthens the case: **the phone needs no network at
all**. There is no upload, no account, and no LLM on the phone
([ADR-034 clause 10](034-shot-first-phone-first.md#10-no-llm-coaching-on-the-phone)), so
[ADR-019](019-secret-handling.md)'s key never reaches it.

### What this does not change

- **§1's choice of Rust, and the retirement rule** in the first addendum.
  [ADR-034 clause 9](034-shot-first-phone-first.md#9-the-oracle-per-vector-family) *extends* the
  rule rather than amending it: new analysis is written in Rust first against hand-worked vectors,
  because a Python one written first would be born to be retired.
- **§3.** The sidecar is unchanged and becomes laptop-only. ADR-033's 2026-09-29 addendum says
  what that moves.
- **§8.** The port is still gated by the conformance suite, not by review. How the screen
  parser's port (M34) is gated is [ADR-032](032-the-rust-core.md)'s to record, by its own addendum.
- **The second addendum's numbers.** They were measured on this desktop, and they say nothing about
  a phone. M39 P0 is where the phone's are measured.
- **`ranges.json`, ADR-001's supersession, and the two-language stdlib-only invariant.**

---

## Addendum, 2026-09-30 — what is required replaces what does not translate, and the lab is ported rather than kept

**M31.5 P4**, docs only. [ADR-035](035-rust-everywhere-python-where-required.md) writes down the
user's directive of 2026-09-30: everything is Rust, and Python stays only where it is required. It
replaces the rule the first addendum wrote, keeps the reasoning that rule rested on, and moves the
schedule that [ADR-032](032-the-rust-core.md) §7 hung from it. The reasons are ADR-035's and are not
restated here. This addendum names the sentences above that no longer hold. Nothing is built, no
vector moves, and `ANALYSIS_VERSION` stays at 16.

### Replaced: the first addendum's rule, and who it left standing

- **"Python keeps only what does not translate"** is replaced by
  [clause 1](035-rust-everywhere-python-where-required.md#1-the-rule-and-the-two-exceptions-it-names):
  Python stays only where the project depends on a library that has no alternative the user would
  take today. The two rules agree about MediaPipe. They part on the sentence after it, *"The lab
  stays Python too — tier 4 — because it is not shipped, not because it could not be ported."* The
  lab is ported now, in §M29
  ([clause 5](035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped)), for the
  reason this ADR's own first addendum gave for deleting: a second copy is a second thing that
  drifts.
- **The survivor list is replaced by clause 1's, which names each file.** The inline amendment's
  list, *"`pose/estimator.py`, `feedback/coach.py` and `feedback/conversation.py`, and nothing
  else"*, was short in two ways. It missed the pose worker's own imports
  ([P1 finding 10](../plans/m31-5-rust-first-replan.md#p1--found-2026-09-30), 2026-09-30), and it
  missed the club lookup, which is an LLM call (P1 finding 8).
- **§3's sidecar carries MediaPipe and nothing else.** The inline amendment read *stays* as "the
  sidecar — pose and the LLM". The LLM stays Python, but it is not a worker in §3's pool.
  - Once `mcp/` is Rust, `conversation.py` drives the Rust MCP server over stdio. That is ADR-035's
    sub-decision, and [ADR-020](020-conversational-followups.md)'s Option B.
  - How the Rust lab reaches `coach.py` is §M29's to build (clause 5's JSON entry).
- **"The vectors are the oracle, not the code that recorded them" stands**, and so does M20's
  delete
  ([clause 7](035-rust-everywhere-python-where-required.md#7-what-this-supersedes-sentence-by-sentence)).
  What changes is who records. From M32 `golf-core` does, under a structural diff gate
  ([clause 3](035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)). So the first
  addendum's *"`regenerate` still builds the synthetic and corpus families from Python"* holds only
  until M32's first Rust re-record.

### Replaced: the schedule

- **The inline amendment's "§M29 retires `api/` into the Flutter shell and ports `mcp/` to Rust"
  splits.** `mcp/` still ports in §M29, on `rmcp`. `api/` is ported to `axum` or dropped, in M40
  ([clause 2](035-rust-everywhere-python-where-required.md#2-everything-else-ports-including-the-three-things-considered-and-not-kept)).
- **The third addendum's "M29 is blocked on M40 … Its job is unchanged" is superseded by clause 5.**
  §M29 *is* the lab port. It runs after M36, depends on M34 and M36, and is not blocked on M40.
- **Retirement happens at two moments.** §M29 switches the lab's entry points to Rust, and deletes
  only what the frozen FastAPI server does not import. M40 deletes the rest, `analysis/` included
  (Q17 in [P2's findings](../plans/m31-5-rust-first-replan.md#p2--found-2026-09-30)).
- **The third addendum's "What this does not change" was right about ADR-034 and is now incomplete.**
  It said that [ADR-034 §9](034-shot-first-phone-first.md#9-the-oracle-per-vector-family) *extends*
  the retirement rule rather than amending it. ADR-035 does amend it, in the two places above.

### Superseded: the second Consequence, as the first addendum left it

The Consequence reads: *"Two implementations of the analysis core will exist, and the Python one
stays. It is the reference, the oracle and the lab — ADR-022's offline fitting is unchanged and
still Python under the `research` extra."* The first addendum already retracted *stays*. The rest
moves as follows:

- **The oracle is Rust from M32** (clause 3). The exception is a port of behaviour that frozen Python
  already has. That is recorded from Python once, before the port moves.
- **The lab is frozen from M32 and ported in §M29**
  ([clauses 4](035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab) and 5), and the
  frozen copy is deleted in M40.
  - Two implementations therefore stand until M40.
  - From M32 they disagree on purpose. Rust's `ANALYSIS_VERSION` moves, and frozen Python's stays at
    16.
- **ADR-022's fitting is archived** with `scripts/golfdb/` and `scripts/caddieset/` in §M29, and the
  `research` extra goes with it. The artifacts it produced stay, and Rust evaluates them (clause 5).

### Moved to §M29: the seam M20 drew

**"Python still decodes and Rust detects" holds until §M29.** Then the Rust lab reaches ffmpeg as a
subprocess itself (clause 5).

- **The first addendum's case for a subprocess** over a native extension carries over unchanged.
- **Its case against PyO3 goes away with the Python caller.** That case was that a lab install would
  need a Rust toolchain.
- **`audio/ffmpeg.py` is deleted in M40**, because the frozen server reaches it until then.

### What this does not change

- **§1.** ADR-035 extends it rather than amending it. The lab's orchestration becomes Rust too.
- **§2, word for word on the laptop.** MediaPipe stays in Python (clause 1). ADR-035's Option C
  declined pose through ONNX Runtime on §2's own grounds.
- **§3 and [ADR-033](033-the-pose-sidecar-protocol.md)'s protocol.** The pool, the job that moves a
  path rather than pixels, and the worker that decodes its own frames all stay as built. The last of
  these is why `capture/` survives (P1 finding 10).
- **§7, §8 and the second addendum's numbers.** §7 keeps cloud analysis closed. §8's gate is still
  the conformance suite, and from M32 the suite includes the diff-gated re-record.
- **"Why deleting is safe."** ADR-035 chose delete over archive, on this ADR's reasoning, for any
  retired Python whose Rust twin conforms (Q1, clause 2). Only the research record goes to
  `archive/`.
- **The audio family stays frozen**, and `regenerate` still refuses it (clause 3).
- **The two-language stdlib-only invariant, in the scoring path.**
  - `ort` is a second numeric library, and it sits outside that path, beside `rustfft`. Clause 5
    brings it into the lab's OCR reader.
  - The invariant's Python half is deleted with `analysis/` in M40.
- **`ranges.json`, and ADR-001's supersession.** ADR-035 clause 7 routes what ADR-001's Status still
  claims for Python.
