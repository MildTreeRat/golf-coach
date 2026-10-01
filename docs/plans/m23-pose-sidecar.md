# Plan: M23 — the pose sidecar

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-09-26. **Milestone**: [ROADMAP §M23](../../ROADMAP.md#m23-the-pose-sidecar--a-long-lived-python-worker-pool).
**Governing decisions**: [ADR-030](../decisions/030-app-platform-rust-core-python-sidecar.md) §2 and §3
(the sidecar's shape), [ADR-033](../decisions/033-the-pose-sidecar-protocol.md) (its protocol —
written by P1, with the field it left open settled by P3's addendum).

---

## Status checklist

**Stop after every phase. Every time.** Update this table when a phase is done, in the same change
as the phase, and append what the phase *found* to its section below — the findings are what the
next phase is planned against, and every M20/M22 phase produced one that changed the phase after it.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document | ✅ Done *(2026-09-26)* |
| **P1** | ADR-033, and §M23's four corrections | ✅ Done *(2026-09-26)* |
| **P2** | `crates/pose` skeleton and the protocol types | ✅ Done *(2026-09-26)* |
| **P3** | The Python worker — `golf_coach.pose.worker` | ✅ Done *(2026-09-26)* |
| **P4** | `Worker`: one child process, one thread, one job | ✅ Done *(2026-09-26)* |
| **P5** | `Pool`: N workers, bounded queue, retry, shutdown | ✅ Done *(2026-09-27)* |
| **P6** | `golf-pose`, and the keypoints writer | ✅ Done *(2026-09-27)* |
| **P7** | The corpus run: all 30 clips, diffed | ✅ Done *(2026-09-27)* |
| **P8** | Pool size, measured rather than picked | ✅ Done *(2026-09-27)* — the sweep ran; the default is **2** |
| **P9** | Docs cascade, and close the milestone | ✅ Done *(2026-09-27)* — **M23 is closed** |

---

## What this milestone is

The Rust↔Python boundary ADR-030 §3 specifies, built: a pool of warm Python worker processes, each
with the interpreter up and the `.task` bundle resident, driven by a job protocol, so that pose runs
off the app's thread and the app never imports MediaPipe.

**It has no caller, and that is expected.** M22 built a whole analysis engine wired to nothing;
M23 builds the other half of what M24 needs, and M24 is the milestone that gives both a caller at
once. So M23's gate is a binary plus a harness, not a pipeline swap — exactly the shape M22 used
(`golf-core run` diffed against `conformance.py run`).

### The five research findings this plan was written against

Measured on 2026-09-26, on this box, before any phase was planned. Each one changed a decision.

1. **Pose is bit-deterministic here.** Re-running `mediapipe:heavy` over
   `data/processed/sessions/2026-08-23/11/face_on.*` (344 frames) reproduced the stored
   `face_on.keypoints.json` with **0 differing values, max absolute delta 0.0** across all 344 × 33
   landmarks × 4 fields. This is what makes the gate a zero-tolerance comparison rather than a
   tolerance one, and it is worth re-confirming in P7 rather than assumed — the corpus is 30 clips
   and this was one.

2. **Throughput is 9.2 fps, not ADR-002's ~24.** 344 frames in 37.3 s, on 2160×3840 portrait 4K.
   `api/worker.py`'s docstring already says "about 9.5 fps" for a 4K clip, so the two Python
   measurements agree and it is ADR-030 §3's budget that is optimistic: its "a 10 s 60 fps clip is
   ~25 s of pose, a two-view swing ~50 s" is ~2.5× low for this corpus's footage. P1 records the
   correction.

3. **The corpus is 42,648 frames — about 77 minutes of pose.** 30 keypoint files under
   `data/processed/sessions/`, all stamped `mediapipe:heavy`, 344 to 4,837 frames each, 239 MB of
   JSON with the largest single file at 27.2 MB. They are **whole uploaded clips** — walk-up, swing,
   walk-back — which is why the number is so large and why the clip-trimming milestone below is
   worth having.

4. **`save_keypoints` rounds nothing.** `storage/keypoints_io.py` writes
   `model_dump_json(indent=2, exclude_none=True)`, so the stored files carry full CPython float
   repr and omit absent keys entirely. Rust's `contracts::KeypointsFile` derives `Serialize` but has
   no `skip_serializing_if`, so it would write `"camera_id": null` where Python writes nothing.
   Decision: **structural equality, not byte equality** — see "Decisions taken" below.

5. **Nothing on disk exercises a `frame_range`.** `api/pipeline.py` poses the whole clip and windows
   afterwards (`_auto_windows` reads `KeypointsFile.clip.fps`). The envelope keeps the field because
   ADR-030 §3 names it, and the worker rejects anything but the whole clip.

---

## Decisions taken, and why

Each of these was asked and answered before the plan was written. A phase that wants to revisit one
needs a reason the interview did not have, and should record it here rather than quietly diverge.

| Decision | Chosen | Why |
|---|---|---|
| **An ADR?** | Yes — ADR-033, in P1 | ADR-030 §3 fixes the *shape* (warm processes, one clip per worker, a path not pixels) and leaves the protocol, framing, restart policy, timeouts and backpressure open. M20 opened with ADR-031 and M22 with ADR-032, and in both cases the ADR changed the milestone's shape before code was written. |
| **The gate** | A `golf-pose` binary plus a diff harness | Mirrors `golf-core run`. A vector family was declined: its true input is a 4K `.MOV` that cannot be committed, and a keypoints-only family would be 239 MB before gzip. |
| **Bundling** (`python-build-standalone`, shipped models) | **M26 owns both**; M23 leaves a seam | §M23 and §M26 both claimed them. M23 resolves the interpreter the way `audio/trigger.py::binary()` resolves `golf-trigger` — config, then `PATH`, then the known build location — and runs against `.venv` on this box. M26 swaps in a bundled runtime with no protocol change. P1 removes the duplicate claim from §M23. |
| **Transport** | Newline-delimited JSON on stdin/stdout, one process per worker | The `golf-trigger` precedent reversed. No ports, no sockets, no shared memory. A crash is EOF on the pipe, which is a signal the pool gets for free. Sockets were declined for the Windows cost; shared memory is ADR-030 §3's own deferral and nothing needs it. |
| **Crate** | A new `crates/pose` — the seventh | Depends on `contracts` alone, so ADR-008's edge is cargo's to enforce; `crates/core` gains a dependency on it. Declined putting it in `crates/core`, whose crate doc's claim to be *only* the seam would have had to be rewritten. |
| **Worker location** | `src/golf_coach/pose/worker.py`, reusing `estimate_pose` and `KeypointsFile` unchanged | ADR-030 §2: "what changes is the process that calls them, and nothing else." A standalone `sidecar/` package was declined because it forks the landmark mapping and the contract shape into a second copy *now*, which is the drift `CLAUDE.md` exists to prevent. M29 is where that endpoint gets reconsidered. |
| **Who writes the artifact** | Rust | The worker stays pure: a clip path in, landmarks out, no output path. Rust owns the swing directory, as `crates/capture` already does, and M24 needs a writer regardless. |
| **Byte-identical output?** | **No** — structural equality, exact parsed values | Nothing compares these files as text: the pose cache keys on the *clip's* sha256, never the keypoints file's. Reusing `pyfmt` to reproduce CPython's repr on the write path was declined as work with no reader, and M29 deletes the Python writer anyway. Recorded as M22 P2's finding standing where it was predicted. |
| **Failure policy** | Retry once on a fresh worker, then a typed failure | One retry covers a transient crash; a second failure is a fact about the clip and is reported as one, never as a partial or guessed `KeypointsFile` — ADR-010 §2's rule reaching the sidecar. Timeout is derived from `frames ÷ measured fps × margin`, not a flat constant, because a 344-frame clip and a 4,837-frame clip differ by 14×. |
| **Models** | The worker verifies the model at **startup** and refuses jobs without it | A worker that cannot pose says so when the pool starts it, not 30 seconds into a golfer's first swing. `ensure_pose_model`'s download stays a lab and setup action; M26 replaces it with a shipped file. |
| **Concurrency** | std threads and channels; `Pool` + `Worker` as the thread-managing types | The workload is 2–4 long CPU-bound child processes, not thousands of sockets. `recv_timeout` gives timeouts with no runtime. tokio was declined on two counts: it makes `crates/pose`'s whole API async (so `golf-pose` and every test need a runtime), and CPU-bound blocking work lands on `spawn_blocking` anyway — tokio's complexity with threads' behaviour. M24 can wrap this if the Flutter bridge wants async. |
| **Pool API** | `submit` → job id, bounded queue, results on a channel | The shape M24's "backlog and thermal budget" needs, and what lets capture never block on pose. Backpressure is the bound, explicit and configurable. |
| **Pool size** | **Measured**, in P8 | Pose is CPU-bound at 9.2 fps, so a second worker may buy nothing. M20 P3 measured recall and precision rather than asserting them; this is the same move. |
| **Lab access** | No new Python entry point | `scripts/run_pose.py` and `api/pipeline.py` keep calling `estimate_pose` directly. Nothing in Python should route through Rust to reach a Python function. |

### Explicit non-goals

- **No caller.** `api/pipeline.py` is not touched. M24 gives the pool its first caller.
- **No shared-memory frame handoff.** ADR-030 §3 specifies it and defers it; nothing needs it.
- **No bundled interpreter, no shipped models.** M26.
- **No clip trimming.** A separate milestone — see below.
- **No `ANALYSIS_VERSION` bump.** Nothing here changes an analysis answer, so `spec/vectors/` does
  not move. If a phase finds otherwise, that is a finding and the vectors are regenerated in the
  same change (`CLAUDE.md`'s invariant).

---

## The clip-trimming milestone (not M23)

Raised during the interview and deliberately kept out of this milestone. P1 adds a ROADMAP entry for
it; it gets its own `/plan-phases` session afterwards.

**The ask**: automatically trim over-long stored clips and keep the trimmed ones, so the corpus stops
being 80-second uploads of mostly walk-up and walk-back.

**What a trimmed clip keeps**: `phases.py::window_around`'s `[start, end)` — address, takeaway, swing
and finish — plus **1 second either side**. A typical 80 s clip becomes ~8 s.

**The correction that scoping needs.** "One second before and after the swing" is safe only if *the
swing* means address-through-finish. Measured from the ball strike it is destructive:
`crates/trigger/src/clip.rs::MIN_LEAD_S` is `0.80 × (1 + 5) + PEAK_HOLD_S` = **4.85 s before
impact**, worst case, because `window_around`'s lead is five downswing-lengths floored at
`_MIN_ADDRESS_LEAD_S = 1.5 s` and the window has to contain the address for motion-start detection
to work at all. The module doc is explicit that its 5 s pre-roll "is not a comfortable choice, it is
very nearly the minimum". A 1 s cut around impact would take the address out of every clip.

**The ordering constraint, which is the reason this is recorded here.** Trimming re-cuts the clips,
which changes their frame numbering and their sha256 — and therefore invalidates the 30 stored
keypoints files that are M23's *only* oracle. **M23 P7 must run before the trimming milestone
starts.** If trimming lands first, M23's gate has nothing to diff against and the stored files have
to be re-recorded by an engine nobody has verified yet.

---

## Phases

Each phase is sized to be picked up cold by a session that has read `CLAUDE.md`, this plan, and the
three-to-six files the phase names. **Stop after every phase.**

### P0 — this plan document ✅ *(2026-09-26)*

**Goal.** The handoff exists, so no later session re-derives the interview.

**Files.** `docs/plans/m23-pose-sidecar.md` (this file). Nothing else.

**Done.** The file exists with the checklist above.

**A gotcha for whoever commits it.** `tests/test_docs_truth.py::test_the_documentation_map_counts_the_documents_correctly`
counts `git ls-files "*.md" ":!.claude"` and compares it against the number `docs/README.md:3`
claims. Both are **71** right now. This file is untracked, so the test is green — but the commit that
tracks it must bump that count to 72 **in the same commit**, and should add a line to the map's
breakdown for the new `docs/plans/` directory. `_living_docs()` is a non-recursive
`DOCS.glob("*.md")`, so this file needs no tier banner and no map routing row of its own; the banner
at the top is house style, not a test requirement.

---

### P1 — ADR-033, and §M23's four corrections ✅ *(2026-09-26)*

**Goal.** The protocol is decided before anything is built against it, and the roadmap stops saying
four things that research showed are false.

**ADR-033: the pose sidecar protocol.** What it has to decide, because ADR-030 §3 does not:

- **The job envelope.** `job_id`, `clip_path`, `frame_range`, `camera_id`, `pose_model_variant` —
  ADR-030 §3's five fields, kept as written. Say that `frame_range` is accepted and validated as
  whole-clip-only, that no caller asks for a range today, and which milestone would (trimming, or
  live preview).
- **Framing.** One JSON object per line, `\n`-terminated, request on stdin and reply on stdout;
  stderr is logging and never protocol. Say why a length prefix was not needed (JSON cannot contain
  a bare newline) and record the largest reply the corpus produces — **27.2 MB** — as the number
  that makes this a deliberate choice rather than an unexamined one.
- **The handshake.** What a worker says when it comes up, and what it says when the model is absent.
  This is where "verified at startup, refuses jobs without it" becomes a message shape.
- **Failure and restart.** Retry once on a fresh worker, then a typed failure. What EOF means. What
  a non-zero exit means. Why a partial `KeypointsFile` is never emitted (ADR-010 §2).
- **Timeouts.** Derived from `frames ÷ measured fps × margin`, with the 9.2 fps measurement and the
  14× spread between the corpus's shortest and longest clip as the reason a flat constant was
  declined.
- **Backpressure.** The bounded queue, and what `submit` does when the bound is reached.
- **Interpreter resolution.** The `audio/trigger.py::binary()` pattern, and the explicit handoff of
  bundling to M26.

**The four ROADMAP §M23 corrections.** Cite the measurement each time; do not paraphrase it.

1. Throughput is **9.2 fps on 4K portrait**, so a two-view swing is ~2 minutes of pose, not 50 s.
   §M23's "Budget from ADR-002's ~24 fps" paragraph is wrong for this corpus's footage. This also
   wants an addendum on ADR-030 §3, whose numbers it corrects.
2. **Bundling and model shipping move to §M26**, which already claims both. Remove them from §M23's
   task list and say where they went.
3. **Verification is not free.** §M23 says "Verification is free and should be used" — the diff is
   free, the 77-minute re-pose is not. Correct the sentence and name the number.
4. A new milestone entry for **clip trimming**, with the ordering constraint above.

**Files.** `docs/decisions/033-<slug>.md` (new), `docs/decisions/030-app-platform-rust-core-python-sidecar.md`
(addendum, §3's throughput numbers), `ROADMAP.md` (§M23, the status table row, and the new
trimming milestone section plus its status-table row), `docs/README.md` (the map: route to ADR-033,
its tier, the doc count, the addendum count, and the ADR-033 row's own addendum count).

**Reuse.** ADR-031 and ADR-032 are the two closest models for an ADR that opens a milestone — read
ADR-031 first, it is the shorter of the two. `docs/decisions/000-template.md` is the contract.

**Verify.**
```
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py
```
Expect the addendum-count, doc-count, tier and map-routing tests to be the ones that fail first if
anything is missed. `test_each_adr_row_states_that_adrs_own_addendum_count` and
`test_an_adr_that_counts_its_own_addenda_counts_them_correctly` both fire on a new ADR.

**Done.** ADR-033 is accepted, §M23 says four true things where it said four false ones, the trimming
milestone has an entry with its ordering constraint, and the doc-truth suite is green.

#### What P1 built

[ADR-033](../decisions/033-the-pose-sidecar-protocol.md), nine clauses, Accepted. ADR-030 gained its
**second addendum** carrying the throughput correction, because the wrong number is that ADR's
sentence and not §M23's. §M23 is now *in progress, 2/10 phases* and carries three inline corrections
with the measurement in each; clip trimming is
**[§M30](../../ROADMAP.md#m30-clip-trimming--the-corpus-stops-being-eighty-seconds-of-walk-up)** with
its own status-table row. `docs/README.md`: 71 → **73** documents, 71 → **72** addenda, 32 → **33**
decisions, an ADR-033 row, and a `plans/` line in the breakdown.

#### What P1 found — four things, and the first two change P2 and P4

1. **ADR-033 adds a message ADR-030 §3 did not imply, and P2's `Reply` is therefore three kinds
   rather than two.** A job gets an **acceptance** line — `{job_id, pose: "accepted", frames, fps,
   width, height}` — written as soon as the container is open and before a single frame is posed, then
   exactly one of `done` or `failed`.

   The reason is the timeout, and it is not a nicety. The work deadline has to scale with the clip —
   344 to 4,837 frames is **14.1×** — and the frame count is the only honest scale factor. **The Rust
   core cannot get it**: it has a path, no video decoder, and ADR-031 §2 records deliberately that it
   never decodes a file. The process that is about to decode the clip already knows, so it says. That
   also splits the timeout in two with two different meanings, which is what P4 implements: a **flat,
   short accept timeout** (opening a container does not scale with length, so a worker that has not
   acked is wedged, not busy) and a **derived work deadline**,
   `max(FLOOR, frames / fps_estimate * MARGIN)` with `fps_estimate` = the measured **9.2 fps** and
   `MARGIN` = **3** — ~112 s for the shortest corpus clip, ~26 minutes for the longest. `fps_estimate`
   is configuration and **per variant**, because ADR-002 puts `lite` at ~4× `heavy`; P8's measurement
   may revise its default.

   *P3 inherits it too*: the worker writes the acceptance line, so it opens the clip, reads its
   metadata and replies **before** calling `estimate_pose`.

2. **The largest reply is 16.4 MB on one line, not the 27.2 MB the plan's research finding names.**
   27.2 MB is `down_the_line.keypoints.json` on disk with `indent=2`; the same payload compact on a
   single NDJSON line is **16.4 MB** (4,837 frames). The plan's number is not wrong, it is the wrong
   number *for a framing decision*, and ADR-033 clause 1 records the wire figure. Consequence for P2
   and P4: the pool reads lines with a growing buffer and never `read_to_string`, and neither side
   sets a line-length ceiling.

3. **ADR-033 leaves exactly one field open, and P3 closes it by addendum.** `clip.source_sha256` has
   no source in ADR-030 §3's envelope — the manifest supplies it in the Python pipeline. The ADR
   states the leaning (the worker hashes the file it actually decoded) and assigns the decision to P3.
   So **P3's change touches `docs/decisions/033-*.md` and the ADR-033 row in `docs/README.md`**, whose
   addendum count is currently 0. Two smaller obligations P3 also inherits from the ADR: the worker
   points `sys.stdout` at `sys.stderr` for the duration of the pose call and writes replies to the
   stream it saved at startup (a library that prints would otherwise corrupt the channel), and the
   handshake **verifies** the model without downloading it.

4. **The doc-count test reads the git index, so a new doc has to be `git add`ed before the count can
   honestly be bumped.** `test_the_documentation_map_counts_the_documents_correctly` runs
   `git ls-files`, which lists staged files and ignores untracked ones — so a new doc left untracked
   makes the test pass by not existing. Both new files are staged, which is what makes 73 true now
   rather than at the milestone commit. And one format trap, which cost the first run: in
   `docs/README.md`'s ADR table the addendum **count goes before the first addendum's text** in the
   last cell, because `test_each_adr_row_states_that_adrs_own_addendum_count` takes the *first*
   `**N**` it finds there — appending a second `**2**` after the first addendum leaves the row
   claiming 1.

---

### P2 — `crates/pose` skeleton and the protocol types ✅ *(2026-09-26)*

**Goal.** The seventh crate exists and can serialize a job and parse a reply. No child processes, no
threads, no MediaPipe.

> **Read P1's findings 1 and 2 first.** `Reply` is **three** kinds, not two — ADR-033 clause 4 adds an
> acceptance line — and the wire figure that sizes the reader is **16.4 MB**, not 27.2 MB.

**What it contains.**
- `Job` — the five envelope fields, with `frame_range` present and validated whole-clip-only.
- `Handshake` — `ready` (with `protocol`, `estimator`, `variant`, `model`) or `unavailable`.
- `Reply` — the **acceptance** (`frames`, `fps`, `width`, `height`), the success case carrying a
  `contracts::KeypointsFile`, and the failure cases as a typed enum (model absent, clip unreadable,
  clip empty, `frame_range_unsupported`, `bad_job`, pose failed — ADR-033 clause 5's table is the set).
- The NDJSON framing: write one line, read one line. Nothing about *who* is on the other end.
- `PoseError` for the pool's own failures (spawn failed, timed out, crashed, queue full).

**Files.** `crates/pose/Cargo.toml`, `crates/pose/src/lib.rs`, `crates/pose/src/protocol.rs`, and
`Cargo.toml` at the repo root (add `crates/pose` to `members`).

**Reuse.** `crates/capture/Cargo.toml` is the simplest manifest in the workspace and the right
template. `contracts::keypoints::KeypointsFile` already derives `Deserialize`, so parsing a reply
needs no new shape — this is the first phase that benefits from M22 P2 having ported it.

**Verify.** `cargo test -p pose && cargo clippy --all-targets && cargo fmt --check`. The gate is a
serde round-trip per message type plus one golden envelope written out as a literal in the test, so
a field rename is caught by a test and not by P4.

**Done.** `cargo build` succeeds with seven crates, and the protocol types round-trip.

#### What P2 built

`crates/pose`, the seventh crate, depending on `contracts` and serde and nothing else. `src/lib.rs`
holds the crate doc and `PoseError`; `src/protocol.rs` holds `PROTOCOL_VERSION`, `Job`, `Handshake`,
`FailureReason`, `Reply`, and the framing pair `write_line` / `Lines`. 20 tests, `cargo clippy
--all-targets` and `cargo fmt --check` clean, and the whole `cargo test` still green across all seven
crates. `tests/test_docs_truth.py` passes unchanged — no doc counts crates, so P9 is still where the
docs cascade.

#### What P2 found — five things, two of which are P3's and one of which is an ADR gap

1. **ADR-033 clause 5's table has no row for the accept timeout, and this build answered it.**
   `PoseError::AcceptTimeout` returns `retried() == true`, on the crash row's reasoning: a process
   that never acked is killed and replaced, which is a transient fault in a long-lived C++ graph by
   another name. **P5 implements the retry and owns the decision** — if it disagrees, ADR-033 gains
   an addendum rather than the comment gaining an exception. The comment in `PoseError::retried`
   already says this, so the gap is not silent.

2. **The retry table now lives in code as two methods, and P5 should read them rather than
   re-derive it.** `FailureReason::retried` is the worker-reported half (clause 5's lower rows) and
   `PoseError::retried` is the pool's own (crash, EOF, non-zero exit, `work_timeout`). Each is pinned
   by a test transcribed from the ADR row for row — `clause_fives_retry_table_verbatim` and
   `the_pools_half_of_clause_fives_retry_table`.

3. **The wire figure is confirmed exactly, and the framing cost measured rather than argued.** The
   corpus's largest reply is **16,424,762 bytes on one line** (`2026-08-23/8/down_the_line`, 4,837
   frames) — so P1's finding 2 is right to the byte. Parsing it costs **0.33 s** as a bare
   `KeypointsFile`, **0.44 s** as the internally-tagged `Reply`, **0.48 s** with a
   `{job_id, #[serde(flatten)] body}` wrapper, plus **4 ms** to validate. Two consequences: *the
   "flatten builds it twice" folklore is wrong by measurement* — the tag buffers through serde's
   `Content` too and the difference is 9%, so `job_id` is repeated per variant for plainness as much
   as for cost; and **0.44 s of parsing per reply is not zero**, which is one more reason P4 reads on
   a thread. Against 8.8 minutes of posing that clip at 9.2 fps, none of it matters.

4. **Validation sits on the two framing functions, not on `contracts::validated!` — and that binds
   P3.** `write_line` takes `T: Serialize + Validate` and refuses to emit a bad message; `Lines::read`
   takes `T: DeserializeOwned + Validate` and refuses to return one. The macro was tried first and
   declined: wiring bounds into `Deserialize` leaves the *outbound* direction unchecked while looking
   as though the question had been answered, and a protocol message is only ever built by those two
   functions. So **an out-of-bounds reply is reported as `PoseError::Protocol`, not as a parsed
   value** — P4's stub workers get that for free and P3's real worker must satisfy it.

5. **`Reply::Accepted` carries `ClipMetadata`'s bounds, and the corpus says that is safe**: `frames`
   ≥ 0 (zero allowed, exactly as `ClipMetadata.frame_count` allows it — clause 6's `FLOOR` is what
   keeps a zero-frame ack from deriving a zero-length deadline), and `fps`, `width`, `height` all
   **> 0 and required**. Checked against all **30/30** stored keypoints files: every one carries
   `clip.fps`, `width`, `height` and `frame_count`, all present and positive. So P3's worker can
   always fill the ack from `FileVideoSource` — but if it ever meets a container that reports
   `CAP_PROP_FPS` as 0, **it must report `clip_unreadable` rather than ack with a zero**, because the
   Rust reader will refuse that line as out of bounds. That is P3's to confirm on real containers.

---

### P3 — the Python worker ✅ *(2026-09-26)*

**Goal.** `python -m golf_coach.pose.worker` reads job lines on stdin and writes reply lines on
stdout, wrapping today's `estimate_pose` with nothing changed about it.

**What it does, in order.** Resolve the variant and **verify the model is present** (do not
download); emit the handshake; then loop: read a line, parse a job, open a `FileVideoSource`, run
`estimate_pose(source.frames())`, build a `KeypointsFile`, write one reply line. A fresh
`PoseLandmarker` per job is already what `estimate_pose` does — it creates one inside the `with`
block — so ADR-030 §3's "the process is warm, not the landmarker" needs no new code, only a comment
saying that is why the call is not hoisted.

**What it must not do.** Write any file. Decide any output path. Download a model. Know what a swing
directory is.

**The `clip.source_sha256` question.** Python's pipeline fills it from the manifest. The worker has
only a clip path, so either it hashes the file it decoded or the field is left for Rust to fill.
**Decide it in this phase and record the choice in ADR-033** — hashing the file it actually read is
the more honest of the two, and it costs one pass over a file already being decoded.

**Files.** `src/golf_coach/pose/worker.py`, `tests/pose/test_worker.py`.

**Reuse.** `golf_coach.pose.estimator.{estimate_pose, resolve_variant, pose_estimator_name,
model_filename}`, `golf_coach.capture.file.FileVideoSource`, `golf_coach.contracts.keypoints.{KeypointsFile,
ClipMetadata}`, `golf_coach.config.settings`. Do not re-implement the landmark mapping;
`_to_frame_keypoints` is where it lives and it stays there.

**Verify.**
```
.venv/Scripts/python.exe -m pytest tests/pose/test_worker.py
.venv/Scripts/python.exe -m ruff check src tests
.venv/Scripts/python.exe -m mypy src
```
The tests drive the worker over real pipes with `estimate_pose` monkeypatched, so they need neither
MediaPipe nor a 4K clip and run in seconds. Cover: the handshake; a good job; a missing model at
startup; an unreadable clip; a malformed job line; a `frame_range` that is not the whole clip; and
stdin closing (clean exit).

**Also pin.** `tests/api/test_pipeline_imports.py` is the file that fails on a wrong architectural
assumption — check it still passes, and consider whether the worker deserves a line in it. The
worker *may* import MediaPipe, unlike everything that file guards, so if it gains an assertion it is
the inverse one.

**Done.** The worker answers jobs correctly over pipes with no MediaPipe in the test run, and the
`source_sha256` decision is recorded in ADR-033.

#### What P3 built

`src/golf_coach/pose/worker.py` — the handshake, the job loop, the two replies per job, the six
failure reasons — and `tests/pose/test_pose_worker.py`, 28 tests driving `main` over a real
`os.pipe()` pair on a thread with `_open_clip` and `_estimate` faked, so `cargo`'s and pytest's runs
both stay free of MediaPipe. `estimate_pose`, `FileVideoSource`, `_to_frame_keypoints` and
`KeypointsFile` are reused untouched, as ADR-030 §2 requires. ADR-033 gained its **first addendum**
and `docs/README.md` its matching count (72 → **73** addenda); `tests/api/test_pipeline_imports.py`
gained a fourth pose module. `ruff`, `mypy` and `tests/test_docs_truth.py` clean.

#### What P3 found — six things, and the first two are what P4 and P7 are planned against

1. **The worker already reproduces a stored keypoints file exactly, end to end, before any Rust
   exists.** `python -m golf_coach.pose.worker` driven as a subprocess over pipes on
   `2026-08-23/11/face_on` returned a `KeypointsFile` **structurally identical** to the stored
   `face_on.keypoints.json` — all 344 frames, the clip metadata including `source_sha256`, and the
   `mediapipe:heavy` stamp. So P7's comparison is known to be satisfiable on 1 of 30 clips, and
   measurement 1's determinism is confirmed *through the sidecar* rather than through the pipeline.
   The timings: the acceptance line at **0.09 s**, and the `done` line at **39.3 s** and **34.1 s**
   on two runs of the same clip — **8.8 and 10.1 fps** including the sha256 pass and serialization,
   so clause 6's 9.2 fps default sits inside the spread rather than outside it, and `MARGIN = 3`
   absorbs either end several times over. **1,215,589 characters** on the reply line for 344 frames,
   which scales to the 16.4 MB P2 measured for 4,837.

2. **Four things P4's state machine has to be built for**, all of them true of the worker as built:
   - **A failure can arrive with no acceptance before it.** `clip_unreadable`, `bad_job` and
     `frame_range_unsupported` are all decided before or instead of opening a container, so P4 must
     treat `Reply::Failed` as a legal first reply and not wait for an ack it will never get.
   - **A `bad_job` from an unparseable line carries an empty `job_id`.** There is nothing to echo.
     Attribute it to the job in flight — the worker owns one at a time — rather than refusing it as
     unattributable, which would turn a readable failure back into a silent one.
   - **The accept timeout can be genuinely short.** 0.09 s to open a 4K container and read its
     metadata, so a couple of seconds is already generous and a worker past it is wedged, not busy.
   - **The handshake can say two `unavailable` reasons ADR-033 does not define** —
     `vision_extra_absent` and `bad_variant` — both exiting non-zero. P2 already keeps that field a
     `String` rather than an enum, so this costs nothing; it is listed because a later tidy-up that
     types it would break a worker whose install is simply missing the extra.

3. **A worker serves only the variant it announced.** A job naming another `pose_model_variant` is
   `bad_job`, recorded in ADR-033's addendum with the reasoning: the model may not be on disk, and
   clause 6's `fps_estimate` is per variant, so a worker that switched bundle mid-flight would make
   every derived deadline wrong while its handshake went on claiming otherwise. **P5's pool spawns a
   worker per variant**, and a pool running two variants is two sets of workers.

4. **The `vision` extra is checked with `importlib.util.find_spec`, and that is load-bearing for the
   handshake.** Both heavy imports stay inside `_open_clip` and `_estimate` so an absent extra is an
   `unavailable` line carrying `pip install -e '.[vision]'` instead of an ImportError traceback and
   an empty channel — which the pool could only report as a worker that died. Pinned by
   `test_the_pose_modules_import_without_the_vision_stack`, which is where P3 answered the plan's
   "consider whether the worker deserves a line in it": it did, and it is the same assertion as the
   other three pose modules for a different reason.

5. **The test module is `tests/pose/test_pose_worker.py`, not the mirrored `test_worker.py`** — the
   first basename collision in this repo (`tests/api/test_worker.py` already exists, and with no
   `__init__.py` under `tests/` pytest refuses two test modules sharing a basename). Adding
   `--import-mode=importlib` to `addopts` would have kept the mirror and was declined as a
   whole-suite harness change made for one file. **Do not "fix" the name back.**

6. **`ClipMetadata.fps` keeps `FileVideoSource`'s 30 fps fallback rather than refusing a container
   that reports none**, deliberately, because `api/pipeline.py` has it and that is the code that
   wrote every file P7 diffs against. What the worker *does* refuse is a container reporting a zero
   width or height (`clip_unreadable`), because `Reply::Accepted` bounds both at `> 0` and acking a
   zero would be reported as a protocol violation — the wrong thing about the wrong side of the pipe.
   P2's finding 5 asked for exactly this and it is now what happens; no corpus container needs it.

**One thing P3 deliberately did not touch.** `ROADMAP.md` still says §M23 is **2/10 phases** and
names only P0 and P1, so it is now **two** phases behind. That follows P2, which left it alone for
the same reason: the plan routes the docs cascade to **P9**, and ADR-033 plus `docs/README.md`'s
ADR-033 row were in scope here only because P1's finding 3 assigned them to this phase by name.

---

### P4 — `Worker`: one child process, one thread, one job ✅ *(2026-09-26)*

**Goal.** Rust can start a Python worker, hand it one job, get one reply, and survive the worker
dying or hanging.

**What it contains.** A `Worker` type that owns one child process and the thread that talks to it:
spawn (with interpreter resolution), read the handshake, `submit`, read the reply, `kill`. Plus the
timeout, derived per job from frame count and the measured fps rather than a constant. EOF on stdout
is a crash; a reply that does not parse is a protocol violation and also a crash.

**Interpreter resolution.** The `audio/trigger.py::binary()` pattern in the other direction: an
explicit config value, then `PATH`, then the known `.venv` location for a source checkout. A missing
interpreter is a typed unavailability with the fix in the message, the way `TriggerUnavailable`
carries "run `cargo build --release`" — not a panic.

**Files.** `crates/pose/src/worker.rs`, `crates/pose/tests/worker.rs`, plus a small stub script
under `crates/pose/tests/` that speaks the protocol.

**Reuse.** `std::process::Command`, `std::thread`, `std::sync::mpsc::Receiver::recv_timeout`.
`crates/trigger/src/testing.rs` is the precedent for test-only helpers living in the crate.

**Verify.** `cargo test -p pose`. The gate is the **stub worker**: a few dozen lines of Python that
answers correctly, plus variants that exit mid-job, that print garbage, that hang forever, and that
report a missing model. No MediaPipe anywhere in `cargo test`. Add one test behind an env guard that
runs the *real* worker on the 344-frame clip, skipped by default — it is ~40 s and P6 is where the
real path is gated properly.

**Done.** A stub worker is spawned, answers a job, and each of crash, garbage, hang and
model-absent produces the right typed error instead of a hang or a panic.

#### What P4 built

`crates/pose/src/worker.rs` — `Config`, `Outcome`, `Worker`, clause 9's `interpreter_candidates`
and clause 6's `Config::work_deadline` — plus `crates/pose/tests/worker.rs` (23 tests) and
`crates/pose/tests/stub_worker.py`, a stdlib-only worker with **17 modes**, one per way a child
process can behave. `cargo test` is green across all seven crates with no MediaPipe, no 4K clip and
no clip longer than a second anywhere in it; `cargo clippy --all-targets` and `cargo fmt --check`
clean; `ruff`, `mypy`, the 1,990 Python tests and `tests/test_docs_truth.py` unchanged and green.
ADR-033 gained its **second addendum** and `docs/README.md` its matching count (73 → **74**).

**The real worker ran end to end through Rust**: `Worker::spawn` resolved the checkout's `.venv`,
handshook `mediapipe:heavy`, and returned **344 frames in 39.9 s** with the `source_sha256` P3's
addendum decided. It is a test behind `GOLF_POSE_REAL_WORKER` and skipped by default — a frame
count is not a conformance check, and **P6 is the gate**.

#### What P4 found — six things, and the first is a correction to ADR-033

1. **Clause 9's resolution cannot be a first hit, and on this box the first hit is the wrong
   Python.** `PATH` answers with a system Python 3.13 that raises `ModuleNotFoundError: No module
   named 'golf_coach'`; the checkout's `.venv` is the *third* candidate and the one that works. Any
   `golf-trigger` on `PATH` is the right `golf-trigger` — a Python is not interchangeable that way.
   So `Worker::spawn` tries the candidates in **clause 9's order, unchanged**, and takes the first
   that *answers*, which the clause's own *"if none of them answer"* already allows. ADR-033's
   second addendum has the table of what counts as an answer. **P5 should resolve once and pass the
   winner as `Config::python` to every worker after the first** — otherwise a pool of N workers pays
   the two dead candidate spawns N times.

2. **One sentence of clause 6 is wrong, and the code is right.** The clause says the flat accept
   timeout *"is the check that catches a hung `PoseLandmarker` construction"*. It is not:
   `_run_job` writes the acceptance **before** calling `_estimate`, and the landmarker is built
   inside `estimate_pose`. A hung construction is caught by the **work deadline**, so the worst case
   is `FLOOR` — 30 s — and not the accept timeout's 10. Left as built, since that is the safe side;
   the addendum corrects the sentence. The three values P4 chose: `FLOOR` **30 s**, accept **10 s**
   (not the two the 0.09 s measurement invites — the first job on a fresh worker also pays for the
   lazy `import cv2`), and a **third** timeout the clause does not name, the **handshake timeout**
   at 60 s, bounding a whole CPython startup.

3. **A `Worker` reports and never retries, which fixes the shape of clause 5's retry for P5.** Every
   `Err` out of `run` has already killed the child, so the pool's retry is necessarily *spawn a
   fresh worker and resubmit*, never *ask this one again* — which is what clause 5 says, now true by
   construction rather than by discipline. `Outcome` is the sum the results channel carries:
   `Done(KeypointsFile)` and `Failed { reason, detail }`, with a `PoseError` `Err` beside them, so
   P5 branches on `FailureReason::retried` and `PoseError::retried` and nothing else. **P2's open
   row is still open**: `AcceptTimeout` is not in the ADR's table, `PoseError::retried` answers
   `true`, and P5 is where that becomes a decision.

4. **Clause 3's ordering is enforced by the reader, so an out-of-order worker is unattributable.**
   The first line off a candidate is parsed as a `Handshake`, so a worker that answered a job before
   handshaking has not sent a protocol message at all as far as Rust can tell, and it lands on the
   same *not our worker* path as a program that printed. The report still names every candidate and
   the line each sent; it just cannot say which of the two went wrong. Accepted rather than fixed —
   the alternative is parsing every first line twice.

5. **`Lines::read`'s protocol error carried no evidence of what corrupted the channel**, and P2's
   gate could not have caught it: serde's message for a line of human text is *"expected value at
   line 1 column 1"*, which names a position and never the content, so *"the channel is corrupt"*
   arrived with nothing attached. P4 added `protocol::excerpt` — the line's first 120 chars,
   truncated on a `char` boundary and bounded because a legitimate message here reaches 16.4 MB.
   The lesson for P5 is the general one: **an error path that has only been unit-tested has not been
   read.**

6. **Three threads per worker, and one second of teardown each.** A `Worker` owns a stdout reader
   and a stderr tail-keeper; P5's pool adds a driving thread per worker, so 2–4 workers is 6–12
   threads. `EXIT_GRACE` is **1 s**: teardown closes stdin, waits up to a second for a clean exit so
   the child's own exit code survives into the report, then kills. That gives P5's orderly shutdown
   its guarantee for free — but **reaping N workers in a loop is N seconds**, so shutdown should
   close every stdin first and reap afterwards.

**One thing P4 deliberately did not touch.** `ROADMAP.md` still says §M23 is **2/10 phases** and
names only P0 and P1 — now **three** behind, following P2 and P3 for the same reason: the plan routes
the docs cascade to **P9**, and ADR-033 plus its `docs/README.md` row were in scope here only because
the addendum contradicts a clause, which is the one thing that cannot wait for P9.

---

### P5 — `Pool`: N workers, bounded queue, retry, shutdown ✅ *(2026-09-27)*

**Goal.** The thread-managing type M24 will hold: `submit` a job, drain results, and never block
capture.

**What it contains.** A `Pool` owning N `Worker`s, a bounded job queue, a results channel carrying
successes and typed failures, **retry once on a fresh worker then give up**, and an orderly
`shutdown` that closes stdin on every child and reaps them. `submit` returns a job id, or refuses
when the queue is at its bound — backpressure is the bound, and the refusal is a value, not a block.

**What it must not do.** Guess. A job that fails twice produces a typed failure naming the clip and
the reason; it never produces a partial `KeypointsFile` and never a zero-filled one. This is
ADR-010 §2 at the process boundary and the comment should say so.

**Files.** `crates/pose/src/pool.rs`, `crates/pose/tests/pool.rs`, and the stub variants from P4.

**Verify.** `cargo test -p pose`. Cover: N jobs across N workers; a worker that crashes on its first
job and succeeds on the retry; one that crashes both times; a hang that hits the timeout and is
killed; the queue at its bound; shutdown with jobs in flight; and shutdown with a hung child (it must
still terminate). Assert results are attributable — a reply must match the job that asked for it,
which is what `job_id` is for and the kind of thing that only breaks under concurrency.

**Done.** All of the above pass with stub workers, and `cargo test` still needs no MediaPipe.

#### What P5 built

`crates/pose/src/pool.rs` — `PoolConfig`, `Completed`, the `Shared` queue and `Pool` itself
(`submit`, `results`, `pending`, `shutdown`, `Drop`), plus the driver thread with clause 5's retry —
and `crates/pose/tests/pool.rs`, 14 tests. `crates/pose/src/worker.rs` gained a `Handle` (finding 2),
`stub_worker.py` two modes and an echo, and `PoseError` one variant. **`cargo test` is green across
all seven crates** (28 binaries, 0 failures) with no MediaPipe, no 4K clip and nothing in it slower
than four seconds; `cargo clippy --all-targets` and `cargo fmt --check` clean. The Python half was
touched once and not cosmetically — see finding 1 — so `ruff`, `mypy`, all **1,991** Python tests and
`tests/test_docs_truth.py` were re-run and are green. **No doc changes**: nothing here contradicts a
clause of ADR-033, so the additions are recorded below for P9's addendum rather than written now.

#### What P5 found — eight things, and the first is a bug in shipped code

1. **The pool found a real defect in P3's worker, and the retry count is what surfaced it.**
   `FileVideoSource.__init__` only stores a path — `__enter__` is what refuses one it cannot open —
   and `_run_job`'s `with` sat *outside* the `try` that maps a bad clip to `clip_unreadable`. So a job
   naming a missing file took a `FileNotFoundError` traceback out of `main`, exited the process
   non-zero, and the pool correctly read that as a **crashed worker**: retried on a fresh interpreter,
   reported as `Crashed`, and the one failure clause 5 says never to retry was never sent. Fixed with
   an `ExitStack` so the container is *entered* inside the guard, and pinned by
   `test_a_container_that_refuses_on_enter_is_unreadable_and_not_a_crash`. **P3's 28 tests could not
   have caught it**: they fake `_open_clip`, and a fake raises from its constructor — P4's finding 5
   recurring in the other language, *an error path that has only been unit-tested has not been read*.
   Two consequences: **P7 inherits the fix** (30 real paths, and a typo in one of them now costs a
   reply rather than two interpreters), and the real-worker smoke test below is the shape that found
   it, so it is committed behind `GOLF_POSE_REAL_WORKER` rather than run once and deleted.

2. **`Worker` had to gain a `Handle`, and without it `shutdown` cannot be bounded at all.** A `Worker`
   is owned by one driver thread and `run` blocks inside it, so nothing else can reach the child — and
   shutdown *must*, because a wedged child would otherwise hold the join for the whole derived work
   deadline, **up to 26 minutes** on this corpus. `Proc`'s `child` and `stdin` now sit behind one
   `Arc<Mutex<Takedown>>` and `Worker::handle()` hands out a cloneable view of it that can close stdin
   and kill, and can do nothing to a *job*. P4's 23 tests were the regression gate for that surgery
   and stayed green unchanged. Shutdown is then exactly P4's finding 6: close **every** stdin, spend
   one shared `EXIT_GRACE`, kill what is left, join — about a second for any N rather than N seconds.

3. **The queue, the closed flag and the live handles share one lock, and that is a decision.** A
   driver replacing a dead worker publishes its handle while `shutdown` takes the list; under separate
   locks those interleave so a fresh child starts *after* shutdown read the list, which is a leaked
   process and a join that waits out a full deadline. `Shared::publish` therefore returns `false` when
   the pool is closing and the driver kills the worker it just started. This is the only race in the
   phase that a test could not have found — all 14 pass with or without the check — which is why it is
   written down here as reasoning rather than left to a comment.

4. **Clause 5's first sentence is implemented literally: anything but landmarks retires the worker.**
   *"EOF on stdout, a non-zero exit, or a `failed` reply is the end of that worker"* — so `retire` is
   one line (`!matches!(outcome, Ok(Outcome::Done(_)))`) and whether the **job** is tried again is the
   separate question `retried()` answers. Narrowing it to *only when the pose call could have run* was
   considered and declined: it reads better for `clip_unreadable` and `bad_job`, which never touch the
   graph, but it contradicts an accepted clause, and a phase is the wrong place to do that quietly.
   The cost is real and named: **a bad path or a caller bug spends a warm interpreter**, and the next
   job pays the respawn. If M24 measures that churn as a problem, the fix is an ADR addendum.

5. **P2's open row is closed: `AcceptTimeout` stays retried.** The pool branches on
   `PoseError::retried` and `FailureReason::retried` and nothing else, and not retrying a wedged
   startup would fail a *clip* for a fault in the *worker* — the wrong side of clause 5's own split.
   `PoseError::retried`'s comment now records that P5 kept it rather than that P5 will decide it.

6. **Two API additions clause 7 does not name, both for P9's addendum.** `PoseError::ShutDown`,
   because a `submit` after shutdown is not a bound a caller can wait out and answering it with
   `QueueFull` would say that it is; and `Pool::pending()`, which **P5's own tests needed** — three of
   them failed on their first run because a *queued* job is handed back by shutdown where one *in
   flight* is killed and reported, and nothing could tell the two apart from outside. It is also the
   number that lets a session engine act *before* the refusal. The bound test found the related
   surprise: with size 1 and bound 2 the refusal lands on the third submission or the fourth depending
   on whether a driver has popped yet, so **the assertion is on the window `bound+1 .. bound+size+1`**
   and a test that asserted a fixed submission would flake.

7. **A driver never exits when it cannot respawn**; it reports the spawn error for that job and tries
   again on the next one. Exiting is the tidier code and the worse behaviour: it leaves the pool short
   a worker, and the *last* driver exiting leaves submitted jobs in a queue nothing drains — silence,
   where reporting a spawn failure per job at least names what is wrong with the machine.

8. **The real worker answers through the real pool**, behind `GOLF_POSE_REAL_WORKER` and skipped by
   default: a handshake through `Pool::new`, a job line, `clip_unreadable` at **attempts 1** for a
   path that does not exist, and a clean shutdown of a real interpreter — about a second, and it poses
   nothing on purpose. **P6 is still the gate** for a reply that carries landmarks.

**Two constants, and one of them is deliberately provisional.** `DEFAULT_QUEUE_BOUND` is **4** with
its reasoning in its doc comment (two two-view swings; the bound is about latency, not memory).
`DEFAULT_POOL_SIZE` is **1** and says so in the comment P8 is expected to replace — 1 is the
configuration whose cost is known, and a default guessing upward would be a measurement claimed
rather than made.

**One thing P5 deliberately did not touch.** `ROADMAP.md` still says §M23 is **2/10 phases** and names
only P0 and P1 — now **four** behind, following P2, P3 and P4 for the reason the plan gives: the docs
cascade is **P9**, and nothing here contradicts a clause, which is the only thing that cannot wait.

---

### P6 — `golf-pose`, and the keypoints writer ✅ *(2026-09-27)*

**Goal.** The first real pose through the real pool, written to a real file, diffed against a
committed one. This is the phase that can break M22.

**What it contains.**
- `golf-pose` — a binary taking a clip path (and the usual options) that runs one job through the
  pool and writes `{camera_id}.keypoints.json`. The `golf-core run` and `golf-capture list`
  precedents: small, one job, prints what it did.
- The writer: `serde_json::to_writer_pretty` over `contracts::KeypointsFile`, plus
  `skip_serializing_if = "Option::is_none"` on the nullable fields of
  `crates/contracts/src/keypoints.rs` so an absent key stays absent.

**The risk, stated plainly.** Those structs carry `#[serde(remote = "Self")]` — the `validated!`
pattern — and M22 P2 found that **a vector distinguishes an absent key from an explicit `null`**, a
distinction its round-trip harness was caught flattening on the first run. Adding
`skip_serializing_if` changes what serialization emits for exactly that case. **Run the full
`cargo test`, not `-p pose`**, and if a `contracts` round-trip breaks, the fix belongs in the
writer rather than in the shared struct.

**Files.** `crates/pose/src/bin/golf_pose.rs`, `crates/pose/src/writer.rs` (or wherever ADR-033 put
it), `crates/contracts/src/keypoints.rs`, `crates/pose/Cargo.toml` (the `[[bin]]` stanza).

**Reuse.** `crates/core/src/bin/golf_core.rs` for the binary's shape;
`crates/capture/Cargo.toml` for the `[[bin]]` stanza; `crates/core/Cargo.toml`'s comment on why the
package is named `golf-core` — check whether `pose` shadows anything in the sysroot before naming the
lib target (it does not, but the check is cheap and the precedent is there).

**Verify.**
```
cargo build --release
cargo test                       # ALL crates, not -p pose
./target/release/golf-pose <the 344-frame face_on clip>
```
Then a structural diff of the written file against
`data/processed/sessions/2026-08-23/11/face_on.keypoints.json`: every float exactly equal, key
presence equal, `pose_estimator` equal. Expect zero differences — research finding 1 measured it.

**Done.** `golf-pose` writes a file that matches the committed one value for value, and the whole
`cargo test` is green.

#### What P6 built

`crates/pose/src/writer.rs` (clause 8's caller half — `KEYPOINTS_SUFFIX`, `keypoints_path`, `to_json`,
`write`), `crates/pose/src/bin/golf_pose.rs` (`golf-pose run <clip> --out <dir>`), the `[[bin]]`
stanza, and `crates/pose/tests/writer.rs`. **`crates/contracts/src/keypoints.rs` is unchanged** — see
finding 1. `cargo test` is green across all seven crates: **30 binaries, 582 tests, 0 failures**, with
no MediaPipe in any of them; `cargo clippy --all-targets` and `cargo fmt --check` clean. No Python was
touched, and `tests/test_docs_truth.py` plus `tests/api/test_pipeline_imports.py` were re-run anyway
(102 passed).

**The gate passed with zero differences.** `golf-pose` posed
`2026-08-23/11/face_on.21831919bc67.MOV` into the scratchpad and the written file was diffed against
the committed `face_on.keypoints.json`: **46,446 values compared, 0 differing, worst |delta| 0.0**,
**16 key paths on each side with no set difference**, `mediapipe:heavy` both sides, 344 frames both
sides. Research finding 1's determinism now holds through the writer as well as through the worker.
The run: handshake **0.35 s**, then **344 frames in 34.2 s = 10.1 fps** — the top of P3's 8.8–10.1
spread, so clause 6's 9.2 fps default still sits inside it and `MARGIN = 3` still absorbs it several
times over.

#### What P6 found — six things, and the second corrects an ADR-033 measurement

1. **The plan's `skip_serializing_if` on the shared struct breaks the `contracts` round trip, measured:
   397 dropped keys.** 391 `camera_id` and 6 `clip.source_sha256`, all in the six *synthetic* vectors —
   they are the only committed keypoints carrying those two spelled `null`, and
   `crates/contracts/tests/round_trip.rs` distinguishes an absent key from an explicit one on purpose
   (M22 P2 caught its own harness flattening the two). So the two demands are genuinely opposed: the
   vector round trip needs `null` in and `null` out, the writer needs `None` to vanish. **The plan
   pre-authorised the fix and it was taken**: the null-dropping is a recursive walk over a
   `serde_json::Value` inside `writer::to_json`, which is pydantic's `exclude_none=True` exactly, and
   the shared struct was left alone. A mirror struct per shape was the alternative and was declined —
   three structs and eleven field names copied out of `contracts`, which a field added upstream would
   be silently dropped by, where the `Value` walk works from whatever `Serialize` emits.

2. **ADR-033 measurement 4 is wrong about the float repr, and the correction is worth more than the
   writer is.** It says the stored keypoints files *"carry full CPython float repr"*. They do not:
   `model_dump_json` serializes inside **pydantic-core, which is Rust and uses `serde_json`**, so
   `save_keypoints` has been writing ryu's float form all along. Measured both ways — pydantic writes
   `1.8422693756292574e-05` as `0.000018422693756292574` where `json.dumps` writes the exponent form,
   and the committed `face_on.keypoints.json` spells it the first way and contains **zero**
   exponent-form literals despite holding four values small enough for CPython to produce one.

   The two measurements that look like they disagree are about **two different writers**, which is the
   whole resolution: ADR-032's second addendum found 77 exponent-form floats in `spec/vectors/`, and
   those are written by `conformance.py`'s `json.dumps`. The keypoints artifact has had a Rust writer
   on both sides of the boundary all along.

   **The consequence, measured on the 344-frame clip**: the file `golf-pose` wrote and the file the
   Python pipeline wrote are the same **1,981,680 bytes** over the same **70,532 lines**, and sorted
   line-for-line they are **identical** — all 45,408 float literals included. The only difference is
   key *order*. **P9 must correct clause 8's reasoning while keeping its conclusion**: a structural
   comparison is still right, because nothing hashes these files and a byte claim resting on
   pydantic-core's choice of float formatter is one a dependency bump can retract.

3. **Key order is the last divergence, and `preserve_order` was declined by a phase on purpose.**
   `serde_json::Map` is a `BTreeMap` without that feature, so keys come out alphabetical where
   pydantic's come out in field-declaration order. An `IndexMap` in insertion order *is* the
   declaration order, so one feature flag would make the two files byte-identical — but it is a
   workspace-wide swap of every `Value`'s map type made for a property clause 8 says this repo does not
   need. **Left to an addendum rather than taken here**; the argument belongs beside finding 2's
   measurement.

4. **`--out` is required and an existing keypoints file is refused without `--force`, and P7 inherits
   both.** A clip lives *in* its swing directory beside the `face_on.keypoints.json` that is M23's only
   oracle, so the obvious default — write beside the clip — would make the natural first invocation
   overwrite the answer it is being checked against. `scripts/pose_replay.py` must therefore pass
   `--out <scratchpad>`, which the plan already wants. Two more things P7 should know: the binary poses
   **one clip per invocation** (30 invocations, 0.35 s of interpreter startup each — 10 s against 77
   minutes, so do not add a batch mode), and the artifact is named from `--camera-id`, falling back to
   the clip file name's leading dot-component (`face_on.21831919bc67.MOV` → `face_on`). That fallback
   names the *file* only; the `camera_id` that reaches every `FrameKeypoints.camera_id` is
   `--camera-id` and nothing else, so **a replay that omits it will differ from the committed file on
   344 `camera_id` values**.

5. **Open question 3 answered: the writer is `crates/pose/src/writer.rs`, and it is expected to move.**
   `keypoints_path` and `KEYPOINTS_SUFFIX` are re-exported at the crate root; `writer::write` and
   `writer::to_json` are deliberately left qualified, because `pose::write` reads as a verb about
   posing rather than about a file. It takes a **`camera_id`, not a `Role`** — the field is free-form by
   design in `contracts::keypoints`, and a `Role` enum belongs to whoever owns the swing directory,
   which under clause 8 is M24. That is also the milestone that should move this module, and the
   comment says so. Not written to a temp file and renamed, matching `save_keypoints`: a half-written
   file is invalid JSON, which `load_keypoints` refuses loudly, and atomic publish would have to cover
   the clip and the analysis too.

6. **Two small things a later reader would otherwise re-derive.** `crates/pose` is the **first Rust in
   this repo that writes a file at all** — `golf-core run` writes to stdout and nothing else writes
   anywhere — so there was no precedent to follow and `save_keypoints` was the specification. And
   `FailureReason` has no `Display`, so `golf-pose` prints `{reason:?}`; growing P2's protocol surface
   for one error line in a dev binary was the wrong way round, and the variant name is one
   underscore-transform from clause 5's wire spelling.

**Why the committed gate reads `spec/vectors/synthetic/` and not the corpus.** `data/processed/` is
**not in git**, so a test that diffed the 344-frame clip would only pass on this box; the real diff is
the phase's own verification, recorded above. What is committed is the six synthetic vectors' `face_on`
— 391 frames — and they are also the only committed keypoints carrying an explicit `null`, which is
the property the writer can get wrong. The 15 corpus vectors are gzipped and reading them would cost
this crate a `flate2` dev-dependency for keypoints `crates/contracts/tests/round_trip.rs` already
round-trips.

**One thing P6 deliberately did not touch.** `ROADMAP.md` still says §M23 is **2/10 phases** and names
only P0 and P1 — now **five** behind, following P2 through P5 for the reason the plan gives: the docs
cascade is **P9**. Nothing here contradicts a clause, so finding 2's correction to ADR-033 is recorded
for P9's addendum rather than written into the ADR now — but it is the one item on that list that
changes a *measurement* and not a status, so it should not be the last thing P9 does.

---

### P7 — the corpus run ✅ *(2026-09-27)*

**Goal.** The claim stops being "one clip agrees" and becomes "the corpus agrees".

**What it contains.** `scripts/pose_replay.py` — re-pose all 30 clips through the pool and diff each
result against the committed keypoints file, reporting per clip and in total. Run it once,
unattended, ~77 minutes. Record the result as a measurement in ADR-033 and in ROADMAP §M23, the way
M20 recorded "recall 30/30 and precision 0.909 over 11.9 minutes of bay audio".

**What to report, not just pass/fail.** Clips compared, frames compared, values compared, values
differing, worst absolute delta, and wall-clock. A census beside the verdict — M22 P8's finding, that
a tolerance far above a ulp cannot tell a converged computation from a systematically wrong one, and
here the same logic says "0 differences out of N" is a stronger claim than "passed".

**Files.** `scripts/pose_replay.py`. It writes nothing into `data/` — compare in memory, or into the
scratchpad.

**Reuse.** `scripts/trigger_replay.py` is the exact precedent: an opt-in harness that replays the
corpus through a Rust binary and reports. Read it first. `golf_coach.storage.keypoints_io.load_keypoints`
is the tolerant reader for the committed side — do not write a second one (`CLAUDE.md`'s rule about
tolerant readers that drift).

**Ordering.** This run must happen **before** the clip-trimming milestone. Trimming re-cuts the
clips and invalidates every file this diffs against.

**Done.** All 30 clips report zero differing values, and the numbers are in ADR-033 and ROADMAP §M23.
If any clip differs, that is the finding, and it is recorded before anything is changed to make it
pass.

#### What P7 built

`scripts/pose_replay.py` — the 30 stored clips re-posed through `golf-pose` and diffed against the
keypoints files the Python pipeline left on disk, per clip and in total. `scripts/trigger_replay.py`
is the model down to `--id`; `storage/keypoints_io.load_keypoints` is the reader for both sides, so
there is no second tolerant reader. It writes into a temporary directory it removes, or into `--out`,
and never into `data/`. ADR-033 gained its **third addendum** and `docs/README.md` its matching counts
(74 → **75** addenda, the ADR-033 row 2 → **3**); ROADMAP §M23 carries the measurement and §M30's
gate is marked discharged in all three places it was stated. `ruff` clean, `tests/test_docs_truth.py`
92 passed. **No `src/` or Rust was touched**, so `mypy`, `cargo test` and the Python suite are exactly
P6's.

**The gate passed on the corpus. 30/30 clips, 42,648 frames, 5,757,660 values compared, 0 differing,
worst absolute delta 0.0**, in **69.8 minutes** at 10.2 fps overall; 16 key paths on each side of
every clip with no set difference. Research finding 1's determinism now holds over the corpus rather
than over one clip, which is what makes the zero-tolerance comparison the right gate. The harness was
checked against a deliberately corrupted file before the real run — a float moved by **1e-12**, one
dropped key and one extra frame were all three reported — so *"0 differing"* is a measurement and not
a comparator that cannot see.

#### What P7 found — five things, and the second narrows a P6 measurement

1. **"30 clips" is 30 comparisons over 26 distinct containers, and the duplication buys a second
   determinism claim.** `face_on.91b9d32c1afb.MOV` and `down_the_line.e19f864d8635.MOV` are each
   shared by three swing directories — `2026-08-07-aaron1/1`, `2026-08-09/2`, `2026-08-10/1`, the early
   fixture sessions — by `manifest.json`'s `content_sha256`. Worth knowing before quoting 30 as a
   count of *footage*, and worth having because those two containers were posed **three times each on
   three separately spawned worker processes and wrote byte-identical files** (one sha256 apiece).
   Determinism against the pipeline's stored answer and determinism across fresh interpreters inside
   one run are two claims, and P6 could only make the first.

2. **P6's "sorted line-for-line identical" is false as stated over 30 files and true after one
   normalisation, and the difference is the trailing comma.** All 30 written files are the **same byte
   length** as their committed counterparts — 239,214,827 bytes on each side in total — but **0 of 30**
   match as sorted lines. Key order alone is why: `serde_json::Map` is alphabetical and pydantic is
   field-declaration order, so a *different key is last* in each object and the comma moves with it
   (`"width": 2160` against `"width": 2160,`). Strip the trailing comma and **30/30** match. P6's
   *conclusion* stands and is now much better evidenced — both writers spell every float identically
   across 239 MB rather than one clip's 45,408 — but the sentence needed the clause. Recorded in
   ADR-033's third addendum because it changes a *measurement*; **P9 still owns clause 8's reasoning
   rewrite** and should quote the corpus figure rather than the single-clip one.

3. **The per-clip throughput is 9.5–10.7 fps and clause 6's 9.2 is below all thirty**, so the derived
   deadline was never close and `MARGIN = 3` is untouched. The run was `size: 1` throughout — 30
   invocations of a one-worker pool, which is all `golf-pose` builds — so it measures the *serial*
   rate and says nothing about contention. **P8 still owns the pool-size default**, and it needs a way
   to run more than one worker that `golf-pose` does not have: the binary hardcodes `size: 1` with a
   comment saying why, so P8's "flag to sweep worker counts" is either a `--workers` flag on the
   binary or a second binary/test, not a flag on this script alone.

4. **Nothing exercised a `frame_range`, and the two known coverage gaps are unmoved.** All 30 jobs were
   whole-clip, so research finding 5 stands: the field is accepted and validated and no caller asks
   for a range until §M30 or live preview does. Still unexercised by the corpus, and still the list
   P9 is asked to write in one place: **no left-handed clip**, and **no two-worker contention**.

5. **The census is worth more than the verdict, and one detail of it is load-bearing.** *Values* means
   135 per frame — `frame_index`, `timestamp_ms`, `camera_id`, and 33 landmarks × `{x, y, z,
   visibility}` — plus six per file for the envelope. Both sides are walked **together** rather than
   zipped from two independent walks, so an absent field or a short landmark list is compared as `None`
   against a value and counts as a *difference* rather than shortening the census: a diff harness that
   can be made to agree by dropping data is the failure M22 P2 caught in its own round-trip harness,
   and this is the same trap one language over. The key-path half (indices collapsed, 16 paths) is
   there because `load_keypoints` cannot answer it — an absent key and an explicit `null` both parse
   to `None`, and keeping those apart is the writer's whole job.

**One thing P7 deliberately did not touch.** `ROADMAP.md`'s status-table row still says §M23 is
**2/10 phases** and names only P0 and P1 — now **six** behind, following P2 through P6 for the reason
the plan gives: the docs cascade is **P9**. §M23's *section* was in scope here because P7's own done
criterion names it, and §M30's gate because this phase is what discharges it.
`docs/ARCHITECTURE.md` §1 lists `scripts/trigger_replay.py` and should gain `scripts/pose_replay.py`
beside the `golf-pose` line P9 is already adding.

---

### P8 — pool size, measured ✅ *(2026-09-27)*

**Goal.** The default pool size is a measurement with its numbers beside it, not a guess.

**What it contains.** Run the 6 shortest clips at 1, 2 and 4 workers; record wall-clock, aggregate
CPU and peak RSS for each. Roughly 15 minutes of runs. Pose is CPU-bound at 9.2 fps, so the live
hypothesis is that parallelism buys little and costs memory — **record what actually happened**,
including if it contradicts that. Each worker holds a ~30 MB model bundle plus an interpreter, so
peak RSS is a real constraint on a laptop and is part of the answer.

**Files.** `crates/pose/src/pool.rs` (the constant and its doc comment), `scripts/pose_replay.py`
(a flag to sweep worker counts), ROADMAP §M23.

**House style, which matters here.** The constant carries its measurement, the way
`crates/trigger/src/clip.rs`'s constants carry theirs — three numbers per configuration in the doc
comment, and the rejected alternative if one configuration was nearly as good.

**Verify.** `cargo test` still green; the constant's doc comment states the measured numbers; ROADMAP
§M23 carries them.

**Done.** The default is chosen from data, and a reader can see the data without running it again.

**What P8 found — and why this row went back to ⬜ *(2026-09-27)*.** Only the *tooling* landed:
`golf-pose sweep --workers N` and `pose_replay.py --sweep`. The measurement itself was never taken —
`crates/pose/src/pool.rs`'s `DEFAULT_POOL_SIZE` still carries the "provisional, P8 replaces this
comment with numbers" doc comment, and ROADMAP §M23 has no sweep numbers. P9 found this while
checking its own inputs (it reconciles docs "against P7's and P8's numbers") and stopped rather than
close the milestone over a constant that says its measurement is pending.

**Decision, from the user, 2026-09-27: run the sweep — `run-p8`.** Do the measurement before P9.
Run `pose_replay.py --shortest 6 --sweep 1,2,4`, record the three numbers per configuration (wall
clock, aggregate CPU, peak RSS) in `DEFAULT_POOL_SIZE`'s doc comment *replacing* the provisional
sentence, and put the same numbers in ROADMAP §M23. Then P8's Verify above must actually pass before
P9 runs. The alternative offered — closing M23 at 9/10 with the default labelled unmeasured — was
declined.

#### What P8 built

The sweep ran, unattended, on 2026-09-27: `.venv/Scripts/python.exe scripts/pose_replay.py --shortest
6 --sweep 1,2,4`, **9.7 minutes** of runs over 2,899 frames at each of three widths.
`DEFAULT_POOL_SIZE` is **2** and its doc comment is now the table below plus the reasoning and the
rejected alternative, in `crates/trigger/src/stream.rs::MIN_TRIGGER_Z`'s shape down to the *what this
cannot say* paragraph. ROADMAP §M23 carries the same table. `golf-pose run`'s `size: 1` comment lost
its "P8 measures what more than one buys" future tense and gained the 1.3 GB number. **No new
tooling** — `--sweep` and `golf-pose sweep` were already there, which is exactly why this row
re-opened. `cargo test` **30 binaries, 582 tests, 0 failures** (P6's and P7's figures unchanged),
`cargo clippy --all-targets` and `cargo fmt --check` clean, `ruff` clean, `tests/test_docs_truth.py`
92 passed. No Python source and no other Rust was touched, so `mypy`, the Python suite and
`conformance.py` are exactly P7's.

| workers | pose | throughput | speedup | aggregate CPU | cores busy | peak RSS |
|---|---|---|---|---|---|---|
| 1 | 288.9 s | 10.0 fps | 1.00x | 470.7 s | 1.63 | 1,344 MB |
| **2** | **168.8 s** | **17.2 fps** | **1.71x** | **505.2 s** | **2.99** | **2,652 MB** |
| 4 | 123.7 s | 23.4 fps | 2.34x | 593.5 s | 4.80 | 5,134 MB |

#### What P8 found — five things, and the first is the plan's own hypothesis being wrong

1. **"Pose is CPU-bound at 9.2 fps, so a second worker may buy nothing" is false, and the *cores
   busy* column is why.** A second worker buys **1.71×** and a fourth **2.34×**. One worker occupies
   only **1.63 cores** — MediaPipe is internally threaded but nowhere near saturating this box — so
   "CPU-bound" was being read as "one core saturated" when it never was. This is the phase doing what
   it was for: the plan said *record what actually happened, including if it contradicts that*, and it
   did contradict it. `DEFAULT_POOL_SIZE` moved **1 → 2** as a result, which is the only code
   behaviour this phase changes.

2. **Memory is what caps the default, and the old comment was wrong about it by 40×.** The comment
   this phase replaced costed a second worker at "a second interpreter and a second ~30 MB model
   bundle". The measured incremental worker is **~1.3 GB** (1,344 → 2,652 → 5,134 MB: 1,308 MB for the
   second worker, 2,482 MB for the third and fourth together). MediaPipe `heavy` over 2160×3840 frames holds far more than its
   weights. So 4 workers is **5.1 GB resident**, which is what rejects it — not its throughput, which
   is the best measured — because ADR-033 clause 6 names the target as a laptop running pose beside
   two camera captures.

3. **Aggregate CPU rises with width and is the second reason to stop at 2**: 470.7 s → 505.2 s
   (**+7%**) → 593.5 s (**+26%**). Two workers buy 1.71× for 7% more CPU; four buy 1.36× more than
   two for another 18%. On a thermally-throttled laptop that surcharge is paid twice — once as heat,
   once as the clock it costs.

4. **One of P7's two coverage gaps is closed, and by the cheap half of this phase.** P7 finding 4
   listed *no two-worker contention* as unexercised. All **6 digests were identical across all three
   configurations**, so running four MediaPipe graphs at once changed no landmark and mis-attributed
   no reply to the wrong job. **P9's coverage list is therefore one item shorter**: what remains
   unexercised is **no left-handed clip**, and — new here — **no pool under contention *with capture
   running beside it***, which is M24's to measure.

5. **The per-clip rate under one worker is 10.04 fps, inside P7's 9.5–10.7 band, so the sweep's
   baseline is the same machine P7 measured** and the speedups are not an artifact of a faster run.
   Two smaller numbers worth keeping: the handshake scales with width (0.49 s → 0.58 s → 1.05 s for
   1/2/4 interpreters — they handshake in parallel, so the second worker costs 0.09 s and the third
   and fourth 0.24 s each rather than a full startup apiece), and `--shortest 6`'s file-size
   proxy picked clips of 344–578 frames, all six from the 2026-08-23 session.

**One thing P8 deliberately did not touch.** `ROADMAP.md`'s status-table row still says §M23 is
**2/10 phases** and names only P0 and P1 — now **seven** behind, following P2 through P7 for the
reason the plan gives: the docs cascade is **P9**. §M23's *section* was in scope here because this
phase's Files list names it. ADR-033 is untouched: nothing here contradicts a clause, and no clause
fixes a pool size — clause 6 only assumes *"two or three workers"* when it argues for `MARGIN = 3`,
which 2 sits inside.

---

### P9 — docs cascade, and close the milestone ✅ *(2026-09-27)*

**Goal.** Everything that describes this repo describes seven crates and a working sidecar.

**What to update.**
- `ROADMAP.md` — §M23 to ✅ with its findings, the status-table row, and M24's "Needs to start"
  (M23 is no longer the thing it waits on; M21 still is).
- `CLAUDE.md` — the "Where is the Rust, and what is in it?" row says *six crates*; it is seven. The
  commands block gains nothing new (`cargo test` covers it), but the "There is Rust here now" block
  says exactly one crate has a Python caller — still true, and `crates/pose` is the first one with a
  Python *callee*, which is worth a clause.
- `docs/ARCHITECTURE.md` §1 "The commands, precisely" — `golf-pose`.
- `docs/README.md` — the map: doc count, addendum count, ADR-033's tier.
- ADR-033 — the addenda for whatever P3 through P8 found, written now rather than deferred. ADR-032's
  P9 is the model: its closing addendum was worth more than the status flips.
- ADR-030 §3 — confirm P1's throughput addendum still reads true against P7's and P8's numbers.

**Verify.** Everything, in one pass:
```
.venv/Scripts/python.exe -m pytest
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
.venv/Scripts/python.exe scripts/conformance.py check
cargo test
cargo clippy --all-targets && cargo fmt --check
```

**Done.** Six commands green, the milestone is ✅, and the coverage the sidecar does **not** have is
one list in one place — M22 measured that gap in every phase rather than assuming it, and the two
known gaps here are already visible: no left-handed clip and no two-worker contention are exercised
by the corpus in the way a live session will exercise them.

#### What P9 built

**Nine documents and one module doc reconciled with a sidecar that finished yesterday, and no code
behaviour changed.** `CLAUDE.md` (the crate count, and `crates/pose` named as the first crate with a
Python *callee* rather than a caller), `ROADMAP.md` (§M23 to ✅ 10/10 with a *What shipped* list, the
status-table row rewritten phase by phase — it was **seven phases behind**, exactly as P2 through P8
each recorded — the M18–M28 overview's milestone states, and M24's blocker note in both places it is
stated), `docs/ARCHITECTURE.md` (§1's command block gains `golf-pose run`, `golf-pose sweep` and
`scripts/pose_replay.py` with their flags and the reason `--out` is required; §2 gains the sidecar
worker and the diagram node says so; §5 gains the gate-without-a-vector-family bullet and the
latency bullet now names the one term in it that *is* measured), `docs/CONFORMANCE.md` (see finding
1), `docs/README.md` (the addendum total, ADR-033's row and its own count, ADR-030's row), ADR-030
(its Status block, and finding 4's inline correction), **ADR-033's fourth and closing addendum**, and
`crates/pose/src/writer.rs`'s module doc, whose single-clip claim P7 had narrowed and left for here.

**The closing addendum is the phase's substance.** It rebuilds clause 8's reasoning on the corrected
measurement rather than restating its conclusion, records the API clause 7 does not name
(`PoseError::ShutDown`, `Pool::pending()`), clause 5 implemented literally and what that costs, the
defect the failure taxonomy found in shipped Python, clause 6's confirmation and the measured pool
width — and the coverage this sidecar does **not** have, as one list in one place, which is the
milestone's own Done criterion.

**All six verify commands green**, run after the edits rather than assumed: `pytest` **1,991**,
`ruff` clean, `mypy` 118 files clean, `conformance.py check` **21/21 at v16**, `cargo test` **30
binaries, 582 tests, 0 failures**, `cargo clippy --all-targets` and `cargo fmt --check` clean.
`ANALYSIS_VERSION` is still 16 and no vector moved, as the plan's non-goal predicted.

#### What P9 found — five things, and the first two are for M24 to inherit

1. **The plan's update list was one document short, and it is the one that describes verification.**
   `docs/CONFORMANCE.md` §5's tier-3 row — *"the Python sidecar … and this is **all** that stays"* —
   did not list `pose/worker.py`, which is now the process that *is* the sidecar; its crate sentence
   said **six** in the workspace; and its "What this does not cover" bullet on pose said the
   comparison against the 30 stored files *would be* a different oracle, where M23 P7 has since run
   it at 0 of 5,757,660. M22 P9 found the same shape in the same document, which is the useful part:
   a milestone's plan lists the docs that describe *what it built* and forgets the one that describes
   *how this repo knows things work*.

2. **`pose/worker.py` holds the first runtime edge out of `pose/` into another module, and ADR-008's
   addendum ends with *"anything else is a finding"*.** `_open_clip` imports
   `capture.file.FileVideoSource` at first use — lazily, so an absent `vision` extra becomes a
   handshake failure rather than a traceback, which is the right behaviour and not the question. The
   type-only `capture.source.Frame` edge that addendum already records is unchanged; this one is at
   runtime. **Recorded in `docs/ARCHITECTURE.md` §2 as what it is**: the worker is a process entry
   point, so it is a *shell* depending downward in the sense ADR-008's addendum already grants `api`
   and `mcp`, and the graph stays acyclic with `analysis/` untouched. It is written as an as-built
   observation rather than as an amendment, because **an ADR clause is not a docs phase's to write**
   — if M24 reads it the other way, ADR-008 wants a clause and the only alternative is for
   `crates/pose` to hand the worker frames instead of a path, which contradicts ADR-030 §3.

3. **M22 P9's verification trap, met again in the same shape.** `cargo test 2>&1 | tail -5` reports
   **tail's** exit status, so the first two cargo runs of this phase said *exit 0* about nothing at
   all. Redone unpiped, with the output to a file and the status read on its own: 30 binaries, 582
   tests. A pipeline is a verification command with its own opinion, and `cargo fmt --check` was the
   one command that had never been piped.

4. **One sentence of ADR-030's *addendum* is what P8 falsified, so it is corrected in place rather
   than by a third addendum.** *"At 9.2 fps pose is CPU-bound with no headroom to speak of, so a
   second concurrent worker may buy very little"* — measured at **1.71×**, one worker occupying 1.63
   cores. P1's precedent says the ADR that owns a wrong sentence owns its correction, and the
   inline-blockquote form (which ADR-030 already uses at its 2026-09-22 amendment) keeps that ADR's
   addendum count at **two**, which `docs/README.md` pins. The measurement itself lives in ADR-033's
   fourth addendum and in §M23, because that is where a reader of *this* milestone looks.

5. **The coverage list is pointed at from three places and copied into none of them.** `CLAUDE.md`'s
   own rule — nothing here is a count, a status or a band value — applies to a gap list too: §M23's
   row, `docs/ARCHITECTURE.md` §5 and `docs/CONFORMANCE.md` each name the two headline gaps and route
   to ADR-033's fourth addendum for the rest. The seven entries on it are all *unexercised*, not
   *broken*, and four of them close only when something calls the pool, which is M24.

**One thing P9 deliberately did not do.** `docs/CONFORMANCE.md` §3 counts **five** portability edges
where `CLAUDE.md` and §M22's blockquote count **six** — §3's five are the ones `pyfmt.rs` solves,
and the sixth (`max` returning the *first* maximum) lands in `analysis` rather than in a formatter.
Both numbers are internally consistent and neither is about this milestone, so it is left as a
finding rather than reconciled here.

---

## Open questions a phase must close

Not blockers, but each one is a decision a phase makes and should record rather than leave implicit.

1. **`clip.source_sha256`** — the worker hashes what it decoded, or Rust fills it. **P3 decides**,
   ADR-033 records.
2. **Does `crates/core` depend on `crates/pose`?** Nothing in `core` needs pose today — it runs a
   vector whose input is already keypoints. The edge probably belongs to M24's session engine rather
   than to `core`. **P2 decides**, and leaving it out is the defensible answer.
3. **Where the writer lives.** `crates/pose` for now, because an eighth `crates/storage` is premature
   and ADR-032 §8 kept storage in Python for M22. M24 may move it. **P6 records the choice and the
   expectation that it moves.** — **Closed by P6**: `crates/pose/src/writer.rs`, keyed on `camera_id`
   rather than a `Role`, with M24 named as the milestone that moves it. See P6's finding 5.
4. **What the pool logs, and where.** stderr is not protocol, so a worker's stderr has to go
   somewhere a reader can find. **P4 decides**; M24 owns surfacing it in the UI.
