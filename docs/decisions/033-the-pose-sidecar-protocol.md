# ADR-033: The pose sidecar protocol — how the Rust core asks Python for landmarks

## Status
**Accepted** 2026-09-26 — [ROADMAP §M23](../../ROADMAP.md#m23-the-pose-sidecar--a-long-lived-python-worker-pool) P1.
Fills in [ADR-030](030-app-platform-rust-core-python-sidecar.md) §3, which fixed the *shape* of the
sidecar — warm processes, one clip per worker, a path rather than pixels — and left the protocol, the
framing, the handshake, restart, timeouts and backpressure open. Amends nothing. **ADR-030 §3's
throughput budget is corrected by measurement**, in that ADR's 2026-09-26 addendum rather than here,
because the number is its sentence and not this one's.

**Built, and nothing above it calls it** — M23 closed 2026-09-27, 10/10 phases. `crates/pose` holds
the message types and the framing (P2), `golf_coach.pose.worker` answers jobs over stdio (P3),
`pose::Worker` drives one child process end to end (P4), `pose::Pool` drives N of them (P5),
`golf-pose` writes the artifact (P6), and the pool's default width is **2**, measured (P8). **The
whole corpus agrees**: P7 replayed all 30 stored clips and found **0 differing values of 5,757,660**.
The first *caller* was to be M24, then M40's, and is now M29's Rust lab CLI; the fifth and sixth
addenda say why.

**Six addenda at the foot, and the fourth is the closing one.** The first settles clause 4's open
field and three things the worker had to decide; the second settles what clause 9 means by "answer"
and fills in clause 6's numbers — **and corrects one sentence of clause 6**, so read it before
trusting that clause's account of what each timeout catches; the third carries the corpus measurement
and narrows one of P6's; the fourth **rebuilds clause 8's reasoning on the right measurement**
(measurement 4 below is wrong about the float repr), records the API clause 7 does not name, the
measured pool size, and the coverage this sidecar does *not* have. The fifth is
[ADR-034](034-shot-first-phone-first.md)'s: the sidecar is **laptop-only**, because the phone has no
Python, and nothing in the protocol changes. The sixth is
[ADR-035](035-rust-everywhere-python-where-required.md)'s: M29's lab CLI is the first caller, the
Python keypoints writer lasts until M40, and nothing in the protocol changes either.
[docs/plans/m23-pose-sidecar.md](../plans/m23-pose-sidecar.md) is the phase order and the per-phase
findings.

## Date
2026-09-26

## Context

[ADR-030](030-app-platform-rust-core-python-sidecar.md) §3 decided that pose stays MediaPipe in
Python and runs in *"a long-lived, bundled worker pool"*. It is unusually specific about three things
and silent about everything else. It fixes:

- **warm processes, not a warm landmarker** — `RunningMode.VIDEO` carries cross-frame tracking state
  and `pose/estimator.py` forces strictly-increasing timestamps, so a worker takes one clip start to
  finish and builds a fresh `PoseLandmarker` inside it;
- **the job moves a path** — `{job_id, clip_path, frame_range, camera_id, pose_model_variant}`, and
  the worker decodes its own frames; and
- **the reply is a `KeypointsFile`** exactly as `contracts/keypoints.py` defines it, `pose_estimator`
  stamp included.

What it does not say is how those bytes cross the boundary, what a worker says when it comes up, what
happens when one dies mid-clip, how long the core waits, or what `submit` does when the queue is
full. Those are this document's.

**Four measurements taken on 2026-09-26, before any of this was designed.** Each one changed a clause
below, and they are recorded here because the alternative is a protocol argued from taste.

1. **Pose is bit-deterministic on this box.** Re-running `mediapipe:heavy` over
   `data/processed/sessions/2026-08-23/11/face_on.*` — 344 frames, 2160×3840 portrait at 59.965 fps —
   reproduced the stored `face_on.keypoints.json` with **0 differing values and a maximum absolute
   delta of 0.0** across 344 × 33 landmarks × 4 fields. That is what makes the milestone's gate a
   zero-tolerance comparison against the 30 files already on disk rather than a tolerance one.
2. **Throughput is 9.2 fps at `heavy` on 4K portrait**, not ADR-002's ~24: 344 frames in 37.3 s.
   `api/worker.py`'s docstring independently says *"about 9.5 fps"* for a 4K clip, so the two Python
   measurements agree and it is ADR-030 §3's budget that is optimistic — by about 2.5× for this
   corpus's footage.
3. **The corpus is 30 keypoint files, 42,648 frames and 239 MB of JSON**, all stamped
   `mediapipe:heavy`, from **344 to 4,837 frames** each. That is ~77 minutes of pose to re-run at
   9.2 fps, and a **14.1× spread** between the shortest clip and the longest. The largest file is
   27.2 MB indented on disk and **16.4 MB as one compact line**, which is the number a framing
   decision has to be made against.
4. **`save_keypoints` rounds nothing and omits absent keys.** `storage/keypoints_io.py` writes
   `model_dump_json(indent=2, exclude_none=True)`, so the stored files carry full CPython float repr
   and no nulls. `crates/contracts`' `KeypointsFile` derives `Serialize` with no
   `skip_serializing_if`, so a Rust writer emits `"camera_id": null` where Python emits nothing.

   > **Half of this is wrong — corrected 2026-09-27 by P6 and P7, in the fourth addendum.** The
   > absent keys are real and clause 8 rests on them. *"Full CPython float repr"* is not:
   > `model_dump_json` serializes inside pydantic-core, which is Rust, so both sides of this
   > boundary have always written `serde_json`'s float form and the 30 stored files hold **zero**
   > exponent-form literals. Read the fourth addendum before quoting this measurement, or clause 8's
   > reasoning, anywhere.

**And one non-measurement, which is just as load-bearing: nothing on disk exercises a
`frame_range`.** `api/pipeline.py` poses the whole clip and windows afterwards — `_auto_windows` reads
`KeypointsFile.clip.fps` off the enveloped result. So the field ADR-030 §3 names has no caller today,
anywhere.

## Options Considered

### Option A: Newline-delimited JSON on stdin/stdout, one process per worker *(chosen)*
- **Pros**: the `golf-trigger` precedent run in reverse — `audio/trigger.py` already drives a Rust
  binary over a pipe with JSON on stdout, so the shape is proven in this repo and in this direction of
  the boundary. No ports, no sockets, no firewall prompt, no port-collision failure mode, and nothing
  listening on a machine ADR-016 keeps closed. **A crash is EOF on the pipe**, which is a liveness
  signal the pool gets for free rather than a heartbeat it has to invent. `serde_json` and `json` are
  both already present on their sides.
- **Cons**: one 16.4 MB line per reply, which means the reader has to be a streaming line reader and
  not `read_to_string`. stdout becomes protocol, so a stray `print` in a Python dependency corrupts
  the channel — real, and the reason stderr is reserved for logging below.

### Option B: A local socket per worker — loopback TCP, or a Unix socket / named pipe
- **Pros**: bidirectional framing without touching stdio, so a library that prints cannot break it; a
  natural place for out-of-band control messages later.
- **Cons**: on Windows the choice is loopback TCP (a listening port on the golfer's machine, which
  contradicts [ADR-016](016-local-first-host-and-phone-upload-topology.md)'s posture even bound to
  `127.0.0.1`) or named pipes (a second, platform-specific code path). Both need a rendezvous step and
  a connection state machine that stdio gives for nothing, and neither buys anything the pipe cannot
  do. Declined on cost, not on merit.

### Option C: PyO3 — embed CPython in the Rust process
- **Pros**: no serialization at all; landmarks cross as objects.
- **Cons**: it puts MediaPipe, OpenCV and pydantic-core **inside the app process**, so a segfault in a
  C++ graph takes the session down with it — and continuous recording is the thing the session cannot
  lose. It also makes every lab `pip install` need a Rust toolchain, which is the exact regression
  M20 P5 declined for `audio/trigger.py` and for the same reason. The GIL would serialize the pool
  ADR-030 §3 exists to parallelize.

### Option D: One long-lived Python service over HTTP or JSON-RPC, threads inside it
- **Pros**: one process to supervise; a standard protocol with libraries on both sides.
- **Cons**: the GIL again — MediaPipe releases it in places and holds it in others, and ADR-030 §3's
  whole reason for *processes* is that the win is a warm interpreter per concurrent clip. It also
  reintroduces the port from Option B and adds a framework to the sidecar's dependency list.

### Option E: Shared memory, `{shm_name, offset, len, stride, format}`
Specified by ADR-030 §3 itself, as the answer if live-preview pose ever wants frames that are not on
disk.
- **Pros**: zero copy; the only option that works for frames that never reach a file.
- **Cons**: **nothing needs it.** Every caller M23 and M24 have cuts the clip to disk first, so the
  path is already the cheapest possible handle. Kept in *Deferred, by choice*, as ADR-030 §3 kept it.

## Decision

**Option A.** Nine clauses.

### 1. One JSON object per line, request on stdin and reply on stdout; stderr is logging and never protocol

The worker reads `\n`-terminated JSON objects on stdin and writes `\n`-terminated JSON objects on
stdout, one per line, in order. **No length prefix**, because JSON cannot contain a bare newline
outside a string and a string cannot contain a literal one — so the line *is* the frame, and a prefix
would add a second thing to get wrong.

This is a deliberate choice against a measurement and not an unexamined default: the largest reply
this corpus produces is **16.4 MB on one line** (4,837 frames), so both sides must treat the channel
as a stream with no line-length ceiling. The pool reads with a growing buffer; the worker writes with
one `write` plus one `flush` and never builds the line twice.

**stderr carries logging, tracebacks and nothing the pool parses.** The pool captures it, keeps the
last few KiB per worker, and attaches it to whatever failure it reports — which is what makes a
`pose_failed` actionable instead of a shrug. A Python library that prints to stdout would corrupt the
channel, so the worker points `sys.stdout` at `sys.stderr` for the duration of the pose call and
writes its replies to the original stream it saved at startup.

### 2. The job envelope is ADR-030 §3's five fields, and `frame_range` is validated whole-clip-only

```json
{"job_id": "…", "clip_path": "…", "frame_range": null, "camera_id": "face_on",
 "pose_model_variant": "heavy"}
```

Kept exactly as ADR-030 §3 wrote it. `job_id` is the pool's, opaque to the worker and echoed on every
reply about it. `clip_path` is absolute. `camera_id` is carried through to `FrameKeypoints.camera_id`
and is free-form by design (`contracts/keypoints.py`). `pose_model_variant` is resolved through
`pose/estimator.py::resolve_variant`, so an unknown one is rejected by the code that already owns that
validation.

**`frame_range` is accepted, validated, and may only ask for the whole clip** — `null`, or
`[0, frame_count]`. Anything else is refused with `frame_range_unsupported` rather than silently posed
in full. The field stays in the envelope because ADR-030 §3 names it and because two milestones would
use it — clip trimming ([§M30](../../ROADMAP.md#m30-clip-trimming--the-corpus-stops-being-eighty-seconds-of-walk-up)),
and live preview, which is Option E's case — but **no caller asks for a range today**:
`api/pipeline.py` poses the whole clip and windows the result afterwards.

Refusing is the point. A worker that accepted a sub-range and posed the whole clip would return
landmarks whose `frame_index` meant something different from what the caller asked for, and
`_auto_windows` would window the wrong frames — a confident wrong answer, which is
[ADR-010](010-benchmark-ranges.md) §2's rule reaching the sidecar.

### 3. A handshake on startup, and a worker that cannot pose says so before it takes a job

The worker's first line is its handshake, written before it reads anything:

```json
{"pose": "ready", "protocol": 1, "estimator": "mediapipe:heavy", "variant": "heavy",
 "model": "…/pose_landmarker_heavy.task"}
```

It **verifies the model file is present and does not download it.** `ensure_pose_model`'s download
stays a lab and setup action; a worker that reaches the network on first use is a worker that stalls a
golfer's first swing for 30 MB. If the bundle is absent the worker writes

```json
{"pose": "unavailable", "reason": "model_absent", "detail": "…"}
```

and exits non-zero. The pool reports that as a pool-level failure at startup, not as a failed job —
the whole point being that a machine which cannot pose says so when the session starts rather than
30 seconds into the first clip.

`protocol` is an integer the pool checks and refuses when it does not recognize it. M26 bundles the
interpreter and the sidecar with the binary so the two versions ship together, but during development
they do not, and a mismatched pair should fail on the handshake line rather than on a missing field
eight minutes into a clip.

### 4. A job gets two replies: an acceptance carrying the clip's shape, then a result

This is the one place this ADR adds a message ADR-030 §3 did not imply, and it exists to make
clause 6's timeout derivable at all.

```json
{"job_id": "…", "pose": "accepted", "frames": 4837, "fps": 59.9651365485183,
 "width": 2160, "height": 3840}
```

written as soon as the container is open and before a single frame is posed; then exactly one of

```json
{"job_id": "…", "pose": "done", "keypoints": { … a KeypointsFile … }}
{"job_id": "…", "pose": "failed", "reason": "clip_unreadable", "detail": "…"}
```

**Why the acceptance is not optional.** The timeout has to scale with the clip — clause 6 — and the
frame count is the only honest scale factor. The Rust core cannot get it: it has a path, no video
decoder, and [ADR-031](031-the-capture-edge.md) §2 records deliberately that it never decodes a file.
The process that *is* about to decode the clip already knows, so it says. That gives the pool two
different deadlines with two different meanings, which clause 6 spends.

The `done` payload is a `KeypointsFile` — `clip`, `frames`, `pose_estimator` — built by the same
`contracts/keypoints.py` shapes the pipeline already writes, with `pose_estimator` from
`pose_estimator_name(variant)`. The worker **writes no file and decides no path**: clause 8.

`clip.source_sha256` is the one field the worker cannot fill from ADR-030 §3's envelope, since the
manifest is what supplies it in the Python pipeline. Either the worker hashes the file it actually
decoded, or it leaves the field for Rust. **This is P3's to settle and to record here by addendum**;
the leaning is the hash, because a checksum of the bytes that were read is a fact the worker is
uniquely positioned to state, and it costs one pass over a file already being decoded.

### 5. Retry once on a fresh worker, then a typed failure — and only for what a retry could fix

EOF on stdout, a non-zero exit, or a `failed` reply is the end of that worker: the pool does not reuse
a process that has died or raised out of a pose call, because `RunningMode.VIDEO`'s tracking state and
MediaPipe's C++ graph are not things a Python `except` can vouch for.

**The job is retried once, on a freshly spawned worker, and only when a retry could change the
answer.** The split is on the failure, never on the phase:

| Reason | Retried | Because |
|---|---|---|
| worker crashed, EOF, non-zero exit | yes, once | a transient fault in a long-lived C++ graph |
| `work_timeout` | yes, once | the box may have been busy; the second attempt runs less contended |
| `pose_failed` | yes, once | an exception out of the graph, same reasoning |
| `clip_unreadable`, `clip_empty` | **no** | a fact about the file; the second read finds the same file |
| `frame_range_unsupported`, `bad_job` | **no** | a caller bug, and retrying it hides it |
| `model_absent` | **no** | clause 3 makes this a pool-startup failure, not a job's |

A second failure is **reported as a typed failure and never as a partial or guessed `KeypointsFile`**
— no empty `frames`, no landmarks carried over from another clip, no zero-filled frame for one that
could not be posed. [ADR-010](010-benchmark-ranges.md) §2 at the sidecar: a checkpoint that cannot be
measured returns `None` and is named, and the same rule one layer out means a clip that could not be
posed produces a named failure the caller can print.

This table is the same distinction `contracts/caveats.py` draws for the golfer with `refilming_helps`
— *would doing it again change anything?* — and it is drawn here for the same reason: a retry that
cannot help is a retry that turns one wrong answer into two.

### 6. Two timeouts, both derived, because the corpus spans 14×

A flat constant cannot serve a corpus whose clips run 344 to 4,837 frames. So:

- **The accept timeout is flat and short.** Opening a container and reading its metadata is fast and
  does not scale with length, so a worker that has not written its `accepted` line within a few
  seconds is wedged, not busy. This is the check that catches a hung `PoseLandmarker` construction and
  a worker blocked on a network path.
- **The work deadline is derived from the acked frame count**:
  `max(FLOOR, frames / fps_estimate * MARGIN)`, with `fps_estimate` the measured **9.2 fps** at
  `heavy` and a margin of **3**. On this corpus that is ~112 s for the shortest clip and ~26 minutes
  for the longest.

The margin is 3 rather than 1.5 because of what the two errors cost. A deadline that fires early
**kills a healthy worker and throws away up to nine minutes of pose**, then pays for it again on the
retry; a deadline that fires late leaves one job stalled while the rest of the pool keeps draining.
The measurement is also a floor on optimism in every direction that matters: it was taken on an idle
desktop, and the machine this ships to is a laptop, possibly thermally throttled, running two or three
workers and two camera captures at once.

`fps_estimate` is configuration, not a constant, and it is **per variant**: ADR-002 measured `lite` at
roughly four times `heavy`'s speed, so a pool running `lite` with `heavy`'s number would carry a 4×
margin it did not ask for.

### 7. Backpressure is a bounded queue, and `submit` says no rather than growing

`Pool::submit` takes a job and returns a job id; results arrive on a channel. The queue has a
**configured bound**, and submitting to a full queue fails immediately with `queue_full` rather than
blocking the caller or growing without limit.

This is the shape M24's *"backlog and thermal budget"* needs and the reason capture never blocks on
pose: a bound that is reached is a fact the session engine can act on — drop the analysis, tell the
golfer the backlog is full, stop cutting clips — and an unbounded queue is that same decision made
silently and badly, one swing at a time, until memory runs out. `Ring::copy` returning `None` rather
than the nearest thing it still holds is the same choice one stage earlier
([ADR-031](031-the-capture-edge.md) §7).

### 8. Rust owns the swing directory; the worker is pure

The worker takes a clip path and returns landmarks. It does not write a file, does not decide an output
path, does not know what a swing directory is, and has no notion of `Role`.

`crates/pose` hands the `KeypointsFile` to its caller, and the caller writes `{role}.keypoints.json` —
the same division `crates/capture` already has, where capture writes the directory and the detector
only detects. M24 needs that writer regardless, and putting it in the worker would mean two
implementations of the artifact layout, which is the drift `CLAUDE.md` exists to prevent.

**The comparison against the 30 stored files is structural, not byte-for-byte.** Measurement 4 is why:
Python omits absent keys and Rust writes nulls for them, and Python's float repr and `serde_json`'s
disagree about when to reach for an exponent — which [ADR-032](032-the-rust-core.md)'s second addendum
already measured on 77 floats in the committed vectors and made the rule for every cross-language
comparison in this repo. Nothing compares these files as text: the pose cache keys on the **clip's**
sha256, never the keypoints file's.

> **The second of those reasons is false and the conclusion is unchanged — see the fourth addendum.**
> The float halves of the two writers agree over all **239,214,827 bytes** of the corpus, because
> `model_dump_json` is pydantic-core and pydantic-core is `serde_json`; the 77-float finding is about
> `conformance.py`'s artifacts and not this one. Structural equality stays the rule for the reason in
> the sentence after it, which is the one that was always load-bearing. Reusing `pyfmt` to reproduce CPython's repr on a write path with no
reader was declined, and
[§M29](../../ROADMAP.md#m29-the-lab-port--a-rust-lab-cli-the-rmcp-server-and-the-archive-move) deletes the Python
writer anyway.

### 9. The interpreter is resolved the way `golf-trigger` is, and bundling is M26's

`crates/pose` finds its Python the way `audio/trigger.py::binary()` finds its binary: an explicit
configuration setting first, then `PATH`, then the known development location —
`.venv/Scripts/python.exe` on Windows, `.venv/bin/python` elsewhere — and a typed error naming what was
looked for if none of them answer. It then runs `python -m golf_coach.pose.worker`.

**Bundling a standalone runtime (`python-build-standalone`) and shipping the model files are
[§M26](../../ROADMAP.md#m26-ship-it--ci-packaging-signing-and-distribution-per-os)'s**, which already
claims both, and M23's ROADMAP entry claimed them too until P1 removed the duplicate. The swap is
invisible to this protocol: M26 changes which interpreter is resolved and where the `.task` file lives,
and not one message shape above. `config.py::REPO_ROOT`'s source-checkout assumption is inherited here
exactly as ADR-030 records it dying at packaging time.

## Consequences

- **`crates/pose` is the seventh crate**, depending on `contracts` alone, so ADR-008's import rule is
  cargo's to enforce as it is for the other six. `crates/core` gains a dependency on it. Putting it
  inside `crates/core` was declined: that crate's doc claims to be *only* the `run` seam, and this is
  not that.
- **The worker lives at `src/golf_coach/pose/worker.py` and reuses `estimate_pose` unchanged.**
  ADR-030 §2: *"what changes is the process that calls them, and nothing else."* A standalone
  `sidecar/` package would fork the landmark mapping (`_to_frame_keypoints`) and the contract shape
  into a second copy now, for a benefit M29 is the right milestone to decide.
- **Clause 4 adds a message, so the protocol types are three reply kinds and not two.** Anything
  written against ADR-030 §3's plain request/response reading of the sidecar needs the acceptance line
  too.
- **The 30 stored keypoint files are the milestone's only oracle, and re-cutting any clip destroys
  them.** They key on frame numbering and on the clip's sha256, so **M23 P7 must run before
  [§M30](../../ROADMAP.md#m30-clip-trimming--the-corpus-stops-being-eighty-seconds-of-walk-up)
  starts.** If trimming lands first there is nothing left to diff against.
- **A conformance vector family is not added, and that is a decision.** The true input to this boundary
  is a 4K `.MOV` that cannot be committed, and a keypoints-only family would be 239 MB before gzip
  against `spec/`'s current 8.3 + 3.2 MB. The gate is a `golf-pose` binary plus a diff harness — the
  shape M22 used, where `golf-core run` was diffed against `conformance.py run`.
- **`ANALYSIS_VERSION` does not move and `spec/vectors/` does not change.** Nothing here alters an
  analysis answer. If a phase finds otherwise that is a finding, and the vectors are regenerated in the
  same change.
- **No Python entry point is added.** `scripts/run_pose.py` and `api/pipeline.py` keep calling
  `estimate_pose` directly; nothing in Python should route through Rust to reach a Python function.
- **The pool has no caller when M23 closes**, exactly as `crates/analysis` had none when M22 closed.
  M24 gives both one at once.

## Deferred, by choice

- **Shared-memory frame handoff** (Option E). ADR-030 §3 specifies it and defers it; every caller
  through M24 cuts its clip to disk first, so the path is already the cheapest handle there is.
- **A progress or keepalive message.** Clause 6's two deadlines cover the cases a heartbeat would — a
  wedged startup and a clip taking too long — without a message whose only consumer would be a UI that
  does not exist. Revisit when M25 wants a per-clip progress bar.
- **Cancellation.** Nothing can cancel a job today: the pool drops the result or kills the worker. A
  session that stops mid-analysis is M24's question and the kill is sufficient for it.
- **A sub-range pose.** Clause 2 refuses it deliberately. §M30 or live preview is where a caller for it
  appears, and it is a one-clause change here when one does.
- **Batching several clips into one job.** A two-view swing is two jobs, which is what lets them run on
  two workers. One job per clip is also what ADR-030 §3's tracking-state constraint requires.

---

## Addendum, 2026-09-26 — the field clause 4 left open, and three the worker had to decide

**M23 P3 built `src/golf_coach/pose/worker.py`**, and building it settled the one field this ADR
left for a phase plus three questions the ADR did not know it was asking. All four are recorded
here rather than in the module, because they are protocol and the other side of the pipe has to
know them.

### `clip.source_sha256`: the worker hashes the file it decoded

Clause 4 named the leaning and assigned the decision to P3. **The leaning is taken**, and the
reason turned out to be stronger than honesty: the alternative would have broken the milestone's
gate.

`api/pipeline.py` fills the field from `SwingManifest.roles[role].content_sha256` — the digest taken
as the upload streamed in — and `bundle_store` then *moves* those same bytes into the swing
directory. So the digest of the file on disk **is** the stored value, and it was checked rather than
assumed: all 30 corpus keypoints files, **30 matches and 0 mismatches**. A worker that left the
field `None` for Rust to fill would have put every reply structurally out of step with the only
oracle P7 has, and the field is the one thing in `ClipMetadata` the sidecar can state better than
its caller.

It costs one pass over a file already being decoded — 38 MB at 1 MiB a read, inside a job that
takes 39 seconds.

### A worker serves exactly the variant it announced

`pose_model_variant` is accepted when it is absent (the worker's configured default) or equal to
the variant in the handshake. **Anything else is `bad_job`**, and the worker does not switch bundle
on request.

Honouring it was declined on two counts, neither of which is the worker's to decide: the model for
that variant may not be on disk, and clause 6's `fps_estimate` is per variant — ADR-002 puts `lite`
at roughly 4x `heavy` — so a pool whose workers changed instrument mid-flight would derive every
work deadline from the wrong number while its handshake went on claiming otherwise. One process,
one instrument, for the life of the process; a pool that wants two variants spawns two workers.

### `Handshake::Unavailable` gained two reasons, which is what its free-form `reason` was for

Clause 3 defines `model_absent` and `crates/pose` keeps the field a string rather than an enum
precisely so a worker can name something the Rust build had not heard of. Two more exist now, and
both are ways a source checkout fails to be able to pose at all:

- **`vision_extra_absent`** — `cv2` or `mediapipe` is not importable. Detected with
  `importlib.util.find_spec`, which locates a module without executing it, so it costs microseconds
  where a real `import mediapipe` would cost seconds on every worker's startup. The detail carries
  `pip install -e '.[vision]'`, the way `TriggerUnavailable` carries `cargo build --release`.
- **`bad_variant`** — `GOLF_POSE_MODEL_VARIANT` names something `resolve_variant` rejects. An
  operator error, and saying so beats reporting it as a missing model file.

Both exit non-zero, as `model_absent` does. Neither is a `FailureReason`, so clause 5's typed
enum is unchanged.

### Two clarifications the code forced, both narrowing what clause 4 already said

- **The acceptance's `frames` is the container's claim, not the decoded count.** It is
  `CAP_PROP_FRAME_COUNT`, the only count that exists before decoding, and it is deliberately not
  the same number as the `frame_count` in the reply's `ClipMetadata` — which is how many frames
  actually *decoded*, the honest number, and the one a container that claims more than it yields
  disagrees with. Clause 2's range check compares against the claim too, because that is what a
  caller computing a range from clip metadata would have had in front of it.
- **stdout is redirected for the whole job, not only for the pose call.** Clause 1 says "for the
  duration of the pose call"; the worker is wider, because opening a container is also a native
  library that can print and the boundary of a job is easier to reason about than the boundary of
  one call. Replies go to the stream saved at startup either way, which is what makes the width
  free.

### The protocol's first real exercise

`python -m golf_coach.pose.worker` driven over pipes as a subprocess, on
`data/processed/sessions/2026-08-23/11/face_on.*` — 344 frames:

- the **acceptance line arrived in 0.09 s**, so clause 6's flat accept timeout has room to be short;
- the `done` line arrived **39.3 s** and **34.1 s** after the job on two runs of the same clip,
  1,215,589 characters on one line each time — **8.8 and 10.1 fps** including the sha256 pass and
  the serialization. Clause 6's 9.2 fps default sits *inside* that spread rather than above or
  below it, which is the most this sample can say; `MARGIN = 3` absorbs either end several times
  over, and P8 is where the default is measured properly;
- and the reply's `KeypointsFile` was **structurally identical to the stored
  `face_on.keypoints.json`** — every frame, every landmark, the clip metadata and the
  `mediapipe:heavy` stamp. That confirms measurement 1's determinism *through the sidecar* rather
  than through the pipeline, on one clip. P7 is still what says it for the other 29.

---

## Addendum, 2026-09-26b — what clause 9 means by "answer", and clause 6's two numbers

**M23 P4 built `crates/pose/src/worker.rs`**, the first Rust that has spoken to a Python worker.
Four things the ADR left to a phase are settled here, and one sentence of clause 6 is corrected.

### Clause 9 resolves a *list*, and "answer" means a handshake

Clause 9 reads as a first hit — config, then `PATH`, then `.venv` — because that is what
`audio/trigger.py::binary()` does. **It cannot be a first hit here, and the difference is the
reason.** Any `golf-trigger` on `PATH` is the right `golf-trigger`; a Python is not interchangeable
that way, because what is needed is not *a* Python but one that can `import golf_coach`.

Measured on this box, which is an ordinary source checkout: `PATH` answers with a Python 3.13 that
raises `ModuleNotFoundError: No module named 'golf_coach'`, and the checkout's `.venv` — the third
candidate — is the one that works. A first-hit reading therefore fails on the development machine
the ADR was written on.

So `Worker::spawn` tries the candidates **in clause 9's order, unchanged**, and takes the first one
that *answers*. The clause's own wording already allows this — *"a typed error naming what was
looked for if none of them answer"* — and what counts as an answer is now stated:

| The candidate | Verdict |
|---|---|
| A handshake this build's `protocol` matches | It is the worker |
| `unavailable`, or a `protocol` this build does not speak | **An answer.** The search stops; a worker spoke and said no |
| No handshake inside the handshake timeout | **An answer.** Two more candidates would spend two more timeouts learning the same thing |
| Cannot be started, dies without speaking, or prints a line that is not a message | Not our worker. The search moves on, carrying what it said |

When every candidate is rejected the error is `PoseError::Spawn`, whose detail lists each candidate
with its exit code and its stderr tail folded onto one line — on this box, the `ModuleNotFoundError`
that explains it. `InterpreterNotFound` is now only for the case where no candidate existed at all.

### Clause 6's two values, and the one claim in it that is wrong

- **`FLOOR` = 30 s.** The clause names the floor and left the number. What it has to cover is the
  per-job cost that does not scale with length: constructing a `PoseLandmarker` over the 30 MB
  `heavy` bundle, the sha256 pass, and serializing the reply. A zero-frame acceptance is why a floor
  is needed at all — `0 / 9.2 * 3` is a deadline that has already expired.
- **The accept timeout is 10 s, not "a couple of seconds".** P3 measured the acceptance line at
  **0.09 s**, which reads as licence to be tight, and it is not: the *first* job on a fresh worker
  also pays for the lazy `import cv2` that `_open_clip` defers. Paying a respawn to save eight
  seconds on a cold import is the wrong trade, and 10 s is still two orders of magnitude over the
  measurement.
- **"This is the check that catches a hung `PoseLandmarker` construction" is false**, and the
  worker as built is why: `_run_job` writes the acceptance *before* calling `_estimate`, and the
  landmarker is constructed inside `estimate_pose`. A hung construction is therefore caught by the
  **work deadline**, not the accept timeout — which means the worst case is `FLOOR`, 30 s, rather
  than 10. That is the right side to err on and it is left as built; the clause's sentence is what
  is wrong, not the code.
- A third timeout exists that the clause does not name: the **handshake timeout**, 60 s, bounding a
  candidate's whole CPython startup. Generous where the accept timeout is tight, because it is paid
  once per worker and covers the interpreter, `golf_coach.config` and the pydantic-settings import
  behind it. Exceeding it is reported as `Spawn` and is **not** retried — a fact about the machine.

### Clause 3's ordering is enforced by the reader, so an out-of-order worker is unattributable

The first line off a candidate is parsed as a `Handshake`. A worker that replied to a job before
handshaking has therefore not sent a protocol message at all as far as Rust can tell, and it lands
on the same path as a program that simply printed — *not our worker*. The report stays honest, since
it names every candidate and the line each one sent, but **it cannot say which of the two went
wrong**. Accepted rather than fixed: distinguishing them means parsing the first line twice, and the
case is a worker bug this repo's own tests would catch first.

### The retry table is unchanged, and P5 still owns the one row that is not in it

P2's finding stands: `AcceptTimeout` is not a row in clause 5's table and `PoseError::retried`
answers `true` on the crash row's reasoning. P4 did not need to decide it — a `Worker` reports, it
does not retry — so **P5 is still where that becomes a decision**.

### The first real pose through Rust

`Worker::spawn` with clause 6's real numbers, on `2026-08-23/11/face_on.*`: the `.venv` resolved
through the fallback above, `mediapipe:heavy` in the handshake, and **344 frames back in 39.9 s**
with the `source_sha256` the previous addendum decided. Held behind `GOLF_POSE_REAL_WORKER` and
skipped by default, because a frame count is not a conformance check — **P6 is the gate**, against
the committed `face_on.keypoints.json`.

---

## Addendum, 2026-09-27 — the corpus agrees: 5,757,660 values, none of them different

**M23 P7 replayed all 30 stored clips through `golf-pose` and diffed each against the keypoints file
the Python pipeline left on disk.** The harness is `scripts/pose_replay.py`, the precedent is
`scripts/trigger_replay.py`, and this is the measurement the "Consequences" section promised in place
of a conformance vector family. Nothing in the protocol changed; one of P6's measurements is
narrowed, and clause 6's default is confirmed conservative.

### The number

**30/30 clips, 42,648 frames, 5,757,660 values compared, 0 differing, worst absolute delta 0.0**, in
**69.8 minutes** of wall clock at **10.2 fps** overall. Key paths: 16 on each side of every clip,
with no set difference, so an absent key never passed for a null. The census is reported rather than
a verdict for M22 P8's reason in reverse — a tolerance far above a ulp cannot tell a converged
computation from a systematically wrong one, and *"0 of 5,757,660"* is a stronger claim than
*"passed"* at the same price.

Read *values* precisely: 135 per frame — `frame_index`, `timestamp_ms`, `camera_id` and 33 landmarks
× `{x, y, z, visibility}` — plus six per file for the envelope (`clip.{fps, width, height,
frame_count, source_sha256}` and `pose_estimator`). An absent field is compared as `None` against
whatever the other side has, so a value that went missing is a *difference* and not a shorter census.

So the plan's **research finding 1 now holds over the corpus and not over one clip**: pose is
bit-deterministic on this box, which is what makes this gate a zero-tolerance comparison rather than
a tolerance one. Per-clip throughput ran **9.5–10.7 fps** across the 30, so clause 6's
`fps_estimate` of **9.2** is below every clip measured and `MARGIN = 3` is untouched. **P8 still owns
the pool-size default**, and this run was `size: 1` throughout — 30 invocations of a one-worker pool,
which is what `golf-pose` builds.

### Two honesty caveats on the word "corpus"

**It is 30 comparisons over 26 distinct containers.** Two of them — `face_on.91b9d32c1afb.MOV` and
`down_the_line.e19f864d8635.MOV`, by `manifest.json`'s `content_sha256` — are each shared by three
swing directories (`2026-08-07-aaron1/1`, `2026-08-09/2`, `2026-08-10/1`), which are the early
fixture sessions. That is worth knowing before quoting 30 as a count of *footage*.

It also buys something the single-clip gate could not: **those two containers were posed three times
each, on three separately spawned worker processes, and the three written files are byte-identical**
(one sha256 apiece). Determinism against the pipeline's stored answer is one claim; determinism
across fresh interpreters inside one run is another, and both now hold.

**And the corpus still exercises no `frame_range`.** All 30 jobs were whole-clip, so the plan's
research finding 5 stands untouched: the field is accepted and validated, and nothing asks for a
range until trimming or live preview does. The two gaps the milestone knows about are also unmoved —
no left-handed clip, and no two-worker contention.

### P6's sorted-line measurement, narrowed — key order moves the commas too

The writer's module doc and P6's finding 2 say that the file `golf-pose` writes and the file
`save_keypoints` writes are *"sorted line-for-line identical"*. Over 30 files that is **false as
stated, and true after one normalisation**, which is worth having exactly right because it is the
evidence behind clause 8's float claim.

Measured: all 30 written files are the **same byte length** as their committed counterparts —
239,214,827 bytes on each side in total — but **0 of 30** match as sorted lines. The whole difference
is the **trailing comma**. Keys come out alphabetical from `serde_json::Map` and in
field-declaration order from pydantic, so a *different key is last* in each object, and the comma
moves with it: `"width": 2160` against `"width": 2160,`. Strip the trailing comma and the sorted
lines are identical on **30/30**.

**So P6's conclusion is confirmed and its wording was one clause short.** pydantic-core serializes
through `serde_json`, both writers spell every float the same way, and the corpus now says so over
every float literal in 239 MB rather than over one clip's 45,408. What differs is key order and
nothing else. Clause 8's structural comparison stays the right call for the reason it always had —
nothing hashes these files, the pose cache keys on the *clip's* sha256 — and **P9 still owns the
rewrite of clause 8's reasoning**, which should quote this measurement rather than the narrower one.

---

## Addendum, 2026-09-27b — the closing numbers: clause 8's reasoning, what the pool added, the measured pool size, and the coverage this does not have

**M23 is built, and this is the addendum written at the end rather than deferred.** The nine clauses
were written before any of them ran; eight phases later four of them have something to say back.
Nothing here reverses a decision. One clause's *reasoning* rested on a measurement that is wrong, two
gained API this document does not name, one was implemented more literally than it reads, and clause
6's open number is now a table.

### Measurement 4 is wrong about the float repr, and clause 8's reasoning is rebuilt on what is true

Measurement 4 says `save_keypoints` writes files that *"carry full CPython float repr"*. **They do
not.** `model_dump_json` serializes inside **pydantic-core, which is Rust and reaches for
`serde_json`** — so the keypoints artifact has had a Rust writer on both sides of this boundary all
along. Measured both ways: pydantic writes `1.8422693756292574e-05` as `0.000018422693756292574`
where `json.dumps` writes the exponent form, and the committed
`2026-08-23/11/face_on.keypoints.json` spells it the first way and holds **zero** exponent-form
literals despite carrying four values small enough for CPython to produce one.

The two measurements that look as though they contradict each other are about **two different
writers**, which is the whole resolution: [ADR-032](032-the-rust-core.md)'s second addendum found 77
exponent-form floats in `spec/vectors/`, and those are written by `conformance.py`'s `json.dumps`.

**What the corpus says, which is more than P6's single clip could.** All 30 files `golf-pose` wrote
are the same byte length as the 30 the Python pipeline wrote — **239,214,827 bytes on each side** —
and after stripping the trailing comma their sorted lines are identical on **30/30**. Both writers
spell every float literal in 239 MB the same way. The comma clause has to be in that sentence: keys
come out alphabetical from `serde_json::Map` and in field-declaration order from pydantic, so a
*different key is last* in each object and the comma moves with it (`"width": 2160` against
`"width": 2160,`).

**So clause 8's conclusion stands and one of its two reasons does not.** The reason that survives is
the one it always had: **nothing compares these files as text** — the pose cache keys on the *clip's*
sha256, never the keypoints file's — so structural equality is what the property actually is. The
reason that does not survive is *"Python's float repr and `serde_json`'s disagree about when to reach
for an exponent"*: true of `conformance.py`'s artifacts, false of this one. And the clause is left at
structural rather than upgraded to a byte comparison now that the bytes agree, because the agreement
rests on **pydantic-core's choice of float formatter**, which a dependency bump can retract without
anything here noticing.

**`serde_json`'s `preserve_order` was declined, by the phase rather than by this ADR.**
`serde_json::Map` is a `BTreeMap` without that feature; with it an `IndexMap` in insertion order *is*
pydantic's declaration order, so one flag would make the two files byte-identical. It is a
workspace-wide change to every `Value`'s map type, made for a property this clause says the repo does
not need, and it would buy agreement for a *second* reason a dependency owns. Left off, and
`scripts/pose_replay.py` compares key *sets* rather than key order for the same reason.

### Clause 7 gained two pieces of API it does not name, and clause 5 was implemented literally

- **`PoseError::ShutDown`**. A `submit` after `shutdown` is not a bound a caller can wait out, and
  answering it with `QueueFull` would say that it is. Clause 7's refusal is backpressure; this is a
  pool that is gone.
- **`Pool::pending()`** — queued jobs not yet started. It exists because the pool's own tests needed
  it: a *queued* job is handed back by `shutdown` where one *in flight* is killed and reported, and
  from outside nothing could tell those apart. It is also the number a session engine reads to act
  *before* the refusal arrives. One consequence for anyone writing a test against the bound: with
  size 1 and bound 2 the refusal lands on the third submission or the fourth depending on whether a
  driver has popped yet, so the honest assertion is on the window `bound+1 .. bound+size+1`.
- **Clause 5's first sentence is implemented as written** — *"EOF on stdout, a non-zero exit, or a
  `failed` reply is the end of that worker"* — so `clip_unreadable` and `bad_job`, which never touch
  the pose graph, retire a warm interpreter too, and whether the *job* is tried again stays the
  separate question `retried()` answers. Narrowing it to *only when the pose call could have run*
  reads better and was declined, because it contradicts an accepted clause. The cost is named rather
  than hidden: **a bad path or a caller bug spends a warm interpreter**, and the next job pays the
  respawn. If M24 measures that churn as a problem, this is the clause to amend.
- **The accept timeout stays a retried fault**, which closes the row clause 5's table never had:
  `PoseError::AcceptTimeout` answers `retried() == true`, because not retrying a wedged startup would
  fail a *clip* for a fault in the *worker* — the wrong side of clause 5's own split.

### The failure taxonomy found a defect in the worker, which is the strongest thing it did

A job naming a clip that does not exist took a `FileNotFoundError` out of the worker's main loop and
exited the process non-zero — so the pool read it as a **crashed worker**, spent clause 5's retry on
a fresh interpreter, and reported `Crashed`. The one failure clause 5 says never to retry was never
sent: `FileVideoSource.__init__` only stores a path, `__enter__` is what refuses one it cannot open,
and the `with` sat outside the guard that maps a bad clip to `clip_unreadable`. Fixed, and pinned.

Recorded here rather than left in the plan because it is the argument for having a taxonomy at all: a
protocol with one failure shape would have reported this correctly and uselessly forever. It is also
[ADR-010](010-benchmark-ranges.md) §2 at a process boundary — a fact about the clip
reported as one — holding by accident until something checked.

### Clause 6's `fps_estimate` is confirmed conservative, and the pool size is a measurement

Per-clip throughput over the whole corpus ran **9.5–10.7 fps**, so the **9.2** clause 6 assumes sits
below every one of the 30 clips and `MARGIN = 3` is untouched.

`DEFAULT_POOL_SIZE` is **2**, measured on 2026-09-27 over the 6 shortest corpus clips — 2,899 frames
of 4K portrait — through one pool at each width (`scripts/pose_replay.py --shortest 6 --sweep 1,2,4`,
9.7 minutes of runs):

| workers | pose | throughput | speedup | aggregate CPU | cores busy | peak RSS |
|---|---|---|---|---|---|---|
| 1 | 288.9 s | 10.0 fps | 1.00x | 470.7 s | 1.63 | 1,344 MB |
| **2** | **168.8 s** | **17.2 fps** | **1.71x** | **505.2 s** | **2.99** | **2,652 MB** |
| 4 | 123.7 s | 23.4 fps | 2.34x | 593.5 s | 4.80 | 5,134 MB |

**The hypothesis it was run against was wrong**, and the sentence belongs to
[ADR-030](030-app-platform-rust-core-python-sidecar.md)'s 2026-09-26 addendum: *"at 9.2 fps pose is
CPU-bound with no headroom to speak of, so a second concurrent worker may buy very little."* It buys
**1.71×**. The *cores busy* column is why — one worker occupies **1.63 cores**, so MediaPipe is
internally threaded and nowhere near saturating this box, and "CPU-bound" was being read as "one core
saturated" when it never was.

**Four is rejected on memory and not on throughput.** The incremental worker is **~1.3 GB** resident,
not the ~30 MB `.task` bundle the pool's own comment used to assume, so four workers is **5.1 GB** —
and clause 6 names the target as a laptop running pose beside two camera captures. Aggregate CPU also
rises with width (**+7%** at two, **+26%** at four), which a thermally-throttled laptop pays twice,
once as heat and once as the clock it costs. Two is the unit of work as well: a two-view swing is two
jobs, so two workers halve a *swing's* latency where four only help a backlog. Clause 6's aside about
*"two or three workers"* competing for one machine is the configuration that shipped.

The speedup is a property of the development desktop (12 physical / 20 logical cores, 34 GB), and M24
measures it again under a session's real load, which is the only place it decides anything.

### The coverage this sidecar does not have

Conforming is not covered, and every phase measured its own gap rather than assuming it. Gathered
here, because one list is the form a reader can act on:

- **No left-handed clip**, anywhere in the 30. Pose itself does not care; nothing that reads its
  output has been exercised on one through this boundary.
- **No `frame_range`.** All 30 corpus jobs were whole-clip, so clause 2's field is validated and
  unused and the non-measurement above stands. §M30's trimming or a live preview is its first caller.
- **No variant but `mediapipe:heavy`.** Clause 6 makes `fps_estimate` per variant precisely because
  ADR-002 puts `lite` at ~4× `heavy`, and that multiplier has never been measured through a worker.
- **Only two of the real worker's outcomes have ever happened**: a clean `done`, and
  `clip_unreadable`. A crash, a hang, garbage on stdout, a non-zero exit, a missing model and every
  `unavailable` reason are exercised by `crates/pose/tests/stub_worker.py`'s 17 modes and by nothing
  with MediaPipe in it.
- **No pool under contention with capture running beside it.** The sweep shows four graphs at once
  change no landmark and mis-attribute no reply — all 6 digests identical at 1, 2 and 4 workers — but
  a session records while it poses, and that is M24's measurement on M21's hardware.
- **No interpreter but this checkout's `.venv`.** Clause 9's list is exercised on one box, where the
  `PATH` candidate answers wrongly and the third candidate wins. M26's bundled runtime changes which
  candidate answers, not what a message looks like.
- **No caller at all**, exactly as `crates/analysis` had none when M22 closed. Every number here is a
  property of a harness rather than of a session, and M24 gives both halves a caller at once.
- **One box, one corpus.** 30 comparisons over **26 distinct containers**, all 4K portrait phone
  video, all posed on one Windows desktop. Determinism holds across fresh interpreters inside a run
  and against files the Python pipeline wrote weeks earlier; it says nothing about another machine,
  and clause 8's structural comparison is what keeps that from being a question this repo has to
  answer.

---

## Addendum, 2026-09-29 — laptop-only: the phone has no Python, and the first caller moves to M40

**[ADR-034](034-shot-first-phone-first.md) made a standalone iPhone the host.** This sidecar is a
Python process that a Rust core spawns, and a phone runs no Python interpreter, so the sidecar cannot
exist there. It is **laptop-only**. None of the nine clauses changes to say so, because the protocol
was never about where it runs. It is about what crosses the pipe.

### Where pose runs now

- **On the phone**, pose is
  [ADR-034 clause 8](034-shot-first-phone-first.md#8-pose-on-the-phone-is-reopened-behind-a-conformance-gate)'s:
  MediaPipe's iOS `PoseLandmarker`, in process, behind M39 P0's gate. What the gate asks of the pose
  decision is in [ADR-002](002-pose-estimation-mediapipe.md)'s 2026-09-29 addendum. That route never
  reaches this protocol: there is no child to spawn and no pipe to frame.
- **On the laptop**, pose is this sidecar, exactly as built.
  [ADR-030](030-app-platform-rust-core-python-sidecar.md) §2 and §3 still hold there, word for word.

### The first caller moves from M24 to M40

M24 is paused and re-scoped under
[M40](../plans/m31-m40-shot-first-pivot.md#m40--the-laptop-client-resumes), with M25 and M26. So the
first caller of `pose::Pool` is M40's desktop app.

One route gives the sidecar a purpose sooner than M40 would. If M39 P0's gate fails, the phone
records and **mechanics are computed on the laptop**, and on the laptop that is this sidecar's job.
A failed gate therefore gives the sidecar a second source of clips, not a new protocol: a clip that
leaves the phone reaches the laptop as a file, and clause 2's envelope moves a path.

Several sentences above name M24 or M26 as the place something is measured or decided. Rather than
editing each one, this addendum is where the move is recorded, and **each now means M40**:

- clause 7's *"backlog and thermal budget"*;
- the fourth addendum's named cost, that a bad path spends a warm interpreter. *"If M24 measures that
  churn as a problem"* is M40's measurement now;
- the pool's width under a session's real load. `DEFAULT_POOL_SIZE` 2 was measured on the
  development desktop, and nowhere else;
- contention with capture running beside the pool, which is the fourth addendum's coverage list, on
  M21's hardware, and M21 moved to M40 as well;
- clause 9's bundling of the interpreter and the `.task` file. It is M26's, and M26 moves to M40
  whole.

### Pinning the model needs nothing from this protocol

ADR-002's addendum has M39 P0 pin the laptop's `.task` by sha256 before the phone is compared to
it. That happens beneath this protocol, in `pose/estimator.py`'s download, and it needs no field
here. The `ready` handshake names an estimator and a variant, not bytes. If M39 P0 wants the
handshake to carry the hash, that is a protocol change, and it belongs in an addendum here.

### What this does not change

- **The nine clauses**, and every number the M23 addenda measured.
- **The fourth addendum's list of the coverage this sidecar does not have.** It is still owed, now
  by M40.

---

## Addendum, 2026-09-30 — the Python writer outlives M29, the lab CLI is the first caller, and the protocol is unchanged

[ADR-035](035-rust-everywhere-python-where-required.md) keeps Python only where it is required, and
MediaPipe pose is the first thing it names
([clause 1](035-rust-everywhere-python-where-required.md#1-the-rule-and-the-two-exceptions-it-names)).
This boundary is how Python stays for it, so **none of the nine clauses changes**. What moves is when
the things around the boundary happen, because ADR-035 re-scopes §M29 as the lab port and splits
retirement into two moments
([clause 5](035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped)). **Nothing
here is built.**

### The Python keypoints writer is deleted in M40, not in §M29

Clause 8's note ends "§M29 deletes the Python writer anyway". That writer is
`storage/keypoints_io.py::save_keypoints`, and `api/pipeline.py` calls it to cache each view's pose
(`:298`, read 2026-09-30). ADR-035 draws the line between M29 and M40 at the frozen FastAPI server's
import closure, and `api/pipeline.py` is inside it. So M29 does not delete the writer. M40 does, with
`storage/`.

Between M29 and M40, two writers produce `{role}.keypoints.json`: the Rust lab's, through
`crates/pose`'s writer, and the frozen pipeline's. Clause 8 already covers that case. The comparison
is structural, nothing compares these files as text, and the pose cache keys on the clip's sha256.
The fourth addendum's reason for declining the `pyfmt` reuse therefore holds for the longer window
too.

### The first caller is M29's lab CLI, not M40's desktop app

The fifth addendum moved the first caller of `crates/pose` from M24 to M40. ADR-035 brings it
forward. M29's Rust lab CLI "calls Python only as a worker, for MediaPipe, through `crates/pose`"
(clause 5), and M40 depends on M29
([clause 6](035-rust-everywhere-python-where-required.md#6-order-the-phone-path-first)). So the lab CLI
is the first caller. Whether it drives `pose::Pool` or a single `pose::Worker` is M29's plan.

**The fifth addendum's list of what M40 measures stays M40's.** Four of its items are about a live
session on the desktop app, or about packaging that app: the backlog and thermal budget, the pool's
width under a session's real load, contention with capture, and the bundling of the interpreter and
the `.task` file. The lab CLI poses stored clips offline, which is what `golf-pose run` did over the
whole corpus in P7, so it measures none of them. The fifth item, the fourth addendum's named cost
that a bad path spends a warm interpreter, is not tied to a session. The lab CLI is the first caller
that could meet it, and whether M29 measures it is M29's plan.

### The two pointers the Consequences hold

- **The standalone `sidecar/` package stays M29's to decide**, and ADR-035's Deferred list keeps it
  there. What such a package would hold has grown. The question was asked of the pose worker alone,
  and ADR-035's survivors are the worker with `capture/` (which the worker decodes frames through),
  and the LLM side as well (clause 1).
- **"No Python entry point is added" holds, and one of its two examples goes.** `scripts/run_pose.py`
  is deleted in M29, because `golf-pose run` already poses a clip (clause 2, Q10). `api/pipeline.py`
  keeps calling `estimate_pose` in process until M40. The rule that nothing in Python routes through
  Rust to reach a Python function stands.

### What this does not change

- **The nine clauses**, and every number the M23 addenda measured.
- **The fifth addendum**: the sidecar is laptop-only, and a failed M39 gate gives it a second source
  of clips rather than a new protocol.
- **The fourth addendum's list of the coverage this sidecar does not have.** It is still owed by M40.
