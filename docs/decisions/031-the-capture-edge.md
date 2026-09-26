# ADR-031: The capture edge — where capture runs, what it holds, and what it writes

## Status
**Accepted** 2026-09-22 — [ROADMAP §M21](../../ROADMAP.md). Applies
[ADR-030](030-app-platform-rust-core-python-sidecar.md) §1 and §6 to the capture edge and **retires
`capture/source.py`'s promise of a `capture/camera.py` adapter**. Amends nothing; ADR-011, ADR-015
and ADR-025 are all unaffected, and §5 below says why.

**Built from P1 on.** `crates/capture` exists and enumerates; §4's bake-off does not, and §7's
"survives a replug" turned out to need a qualifier. Read the **2026-09-22 addendum** below before
either clause — it is where the hardware corrected them.

## Date
2026-09-22

## Context

[ADR-030](030-app-platform-rust-core-python-sidecar.md) decided the platform and M20 built the first
piece of it. What is missing is the edge where footage enters the system: a laptop, one or two
cameras, a microphone, and a strike that has to become a clip on disk. §M21 has stood as a
placeholder since the M18 planning sitting — three numbered sources and a comma-separated task
list — and M18 deliberately left it without an ADR because *"those are M20 and M21 and they have
evidence to gather first"*.

**M20 gathered it.** `crates/trigger` now carries a live detector at recall 30/30 and precision
0.909, a ring buffer, and cut rules derived from `analysis/phases.py::window_around` rather than
chosen. Those rules already reach across the seam this ADR is about: `clip.rs::BUFFER_S` says *"the
video ring multiplies this by its own frame rate"*, and `ring.rs` is generic over its item
specifically because *"ROADMAP §M21 asks for a ring buffer with host-clock timestamps on the video
side"*. The capture edge is no longer speculative — it has a shape, and the shape is written down in
a crate that compiles.

**And §M21, as written, contradicts that.** It says *"`capture/source.py::VideoSource` is already a
`Protocol` … Keep that shape and fill it in"* — a Python `LiveCameraSource` over OpenCV. That is the
same sentence M20 had to re-scope against, and it is wrong for the same reason: ADR-030 §1 gives
Rust *"camera capture, the ring buffer, clip cutting"*, and ADR-030's 2026-09-22 addendum says
**Python keeps only what does not translate** — MediaPipe, and the lab. A camera loop is neither.

Two further forces shape this, and both are about **what capture is allowed to depend on**:

- **The Rust core does not exist yet.** M22 is unstarted. Anything the capture edge hands to "the
  analysis engine" has nowhere to land, so a capture edge designed against M22 cannot be finished or
  tested until M22 is.
- **Cutting is irreversible and analysis is not** — M20 P4's governing asymmetry, one stage earlier.
  The ring buffer is gone once overwritten, so the capture edge's failure modes are all the
  *unrecoverable* kind: a frame not captured, a clip cut short, a timestamp never taken.

## Options Considered

### Option A: Python `LiveCameraSource` first, port in M22+
- **Pros**: `VideoSource` and `FileVideoSource` already exist, OpenCV is already a dependency under
  the `vision` extra, and a webcam loop in Python is an afternoon.
- **Cons**: it is the exact module ADR-030's addendum wants deleted the day it lands, so its cost is
  paid twice and its tests are thrown away. It cannot share `crates/trigger`'s `Ring`, `Cutter` or
  `OnlineDetector`, so the cut rules get a **second implementation** — the silent-drift failure M19
  and the conformance suite exist to prevent. And it would not reach the phone, where ADR-030 §5
  puts the same code.

### Option B: Rust `crates/capture`, writing a swing directory
- **Pros**: consistent with ADR-030 §1; instantiates `Ring<T>` rather than re-writing it, which is
  what `ring.rs` was made generic for; compiles into the phone app later unchanged. Writing a swing
  directory means the consumer is **today's `api/pipeline.py::analyze_swing`**, so M21 gets a real
  end-to-end path with no dependency on M22.
- **Cons**: a camera crate has to be chosen, and it is the first non-numeric third-party dependency
  in the Rust half. Device enumeration is genuinely three platform APIs wearing one hat.

### Option C: Rust capture handing frames straight to a Rust core
- **Pros**: the end state, eventually.
- **Cons**: blocked on M22, which un-blocks nothing and tests nothing. Rejected on ordering alone,
  not on merit — §3 is how Option B reaches the same place early.

## Decision

**Option B.** Eight clauses.

### 1. Capture is Rust, in a new `crates/capture`

Beside `crates/trigger` in the existing workspace, under ADR-030 §1 and its 2026-09-22 addendum. No
Python `LiveCameraSource` is written, now or later.

**`src/golf_coach/capture/source.py`'s docstring is now wrong and this ADR corrects it**: it
advertises `LiveCameraSource (capture/camera.py) — read the ELP USB camera [needs hardware]`, and a
reader who believes it goes looking for a file that will never exist. `capture/file.py` carries the
same claim (*"The live ELP camera adapter (capture/camera.py) will implement the same `VideoSource`
port later"*). Both say **retired to `crates/capture`** instead.

The `VideoSource` port itself **stays** and keeps its one adapter. It is the lab's way into a clip on
disk, and nothing about that changed.

### 2. File and upload need nothing built — and that is a finding, not a gap

ADR-030 §6 ranks file/upload first among capture sources and insists it is *"a first-class source,
not a test fixture"*. Both remain true, and **M21 adds nothing to it**, because it is already built
twice over:

- the lab reads a clip through `capture/file.py::FileVideoSource`, unchanged since M1; and
- per ADR-030 §3 the pose worker's job envelope is `{job_id, clip_path, frame_range, …}` and **the
  worker decodes its own frames**. The Rust core is never handed pixels out of a file, so it never
  needs a file video source.

This is recorded because the obvious reading of ADR-030 §6 is "port `FileVideoSource` to Rust first",
and that phase would produce a decoder nothing calls. The honest statement of the ordering is: source
1 is *done*, and M21 is about source 2.

### 3. What capture hands over is a swing directory on disk

The unit of work is the per-swing directory `storage/manifest.py` already defines — `Role.FACE_ON`,
`Role.DOWN_THE_LINE`, `Role.SHOT_SCREEN`, `EXPECTED_ROLES`, and a manifest whose `status()` is
computed rather than persisted. A triggered strike writes two clips and a manifest into one, and the
consumer is **`api/pipeline.py::analyze_swing` exactly as it is today**.

That single choice is what takes M21 off M22's critical path. It also makes the milestone testable
against fifteen swings of prior art: a directory the capture edge writes is either one this pipeline
reads, or it is wrong, and there is no third answer to argue about.

`Role.SHOT_SCREEN` is **not** capture's to produce. A swing directory written by cameras is
`collecting` until the shot photo arrives by the path ADR-014 already defines, and the manifest says
so with no modification — which is exactly the property `status()` being computed was for.

### 4. One third-party camera crate, chosen in P1 against a named bar

The candidates are `nokhwa` (one API over Media Foundation, V4L2 and AVFoundation), an
`ffmpeg`-based capture path, and the three platform APIs directly. The choice is P1's, made against
hardware rather than against documentation, and the bar is: **enumerate devices with stable
identity, report capabilities, and deliver frames with an arrival timestamp**, on Windows first —
that is the laptop this is built on — without foreclosing the other two.

**The stdlib-only invariant does not forbid this.** `CLAUDE.md` binds the *analysis core* — the
scoring path — to stdlib plus `contracts`, and ADR-030 §8's Consequences extend that to the Rust
core's scoring path. Capture is the I/O edge, where `crates/trigger` already takes `rustfft` and
`audio/ffmpeg.py` already shells out to a binary. The rule worth keeping is the one that was always
meant: **no numeric library between a measurement and a verdict.**

### 5. One host clock, and the camera-to-microphone offset is measured

Frames and audio blocks are stamped **on arrival, from one host clock**, and a device's own
timestamps are recorded beside that rather than trusted as the timeline.

This is a decision and not a detail because the last time this repo assumed the two tracks of one
recording shared a start, it was wrong by a tenth of a second.
`audio/ffmpeg.py::video_start_seconds` exists because of M11 P10: the container's edit list puts the
video **105–125 ms after** the audio on real clips (`docs/M11_ACOUSTIC_SYNC.md` §E4 — 0.105 s on
`2026-08-23/9 down_the_line`, 0.125 s on `2026-08-23/11`), and `api/pipeline.py` subtracts it before
a sample index becomes a frame index. A live capture edge carries the same hazard with none of the
container metadata to recover it from, so it measures the offset per device rather than assuming
zero. That is **P4's whole job**.

**What this does and does not claim about alignment.** `contracts/alignment.py::AlignmentQuality`
already has its answer for two views: `SYNCHRONIZED` is earned by hearing the same ball strike
(ADR-025), not by sharing a clock, and ADR-030 §5 states plainly that two views *"need no shared
clock in flight"*. So this clause is about each view being internally coherent — its own video
against its own audio — and **not** about fusing the two. ADR-011's fiducial-calibration
prerequisite for real 3-D is untouched.

### 6. The ring holds encoded frames, not raw ones

`Ring<T>` is instantiated over encoded frames. It is not re-implemented — `ring.rs` was made generic
for this and says so.

The sizing is already decided, and the arithmetic is decisive. `clip.rs::BUFFER_S` is
`MAX_CLIP_S + SLACK_S` = **31 s**, and its docstring already says *"the video ring multiplies this by
its own frame rate"*. At ADR-030 §5's figure for raw 1080p60 — ~180 MB/s — 31 s is **~5.6 GB per
camera**, so a two-camera session wants ~11 GB resident before a single swing is analysed. The same
31 s at the hardware encoder's ~1–2 MB/s is **~31–62 MB per camera**. Raw is not expensive, it is
impossible, and this is the same measurement ADR-030 §5 used to keep video off Wi-Fi.

The cost is named rather than hidden: a clip cut out of encoded frames has to start at a keyframe, or
carry the frames back to one, so **keyframe interval becomes a capture setting with a correctness
role** rather than a quality knob. P3 owns it.

### 7. A camera has an identity, and a session says so when it loses one

Device enumeration returns a **stable identifier** that survives a replug, not an index. An index is
a position in a list, and another device can take it.

When a session's chosen camera disappears, the session **stops and names the device that is gone**.
It does not fall back to another camera and it does not quietly keep recording one view. This is
ADR-010 §2 at the capture edge: footage from the wrong camera, silently substituted, is a swing
measured against the wrong view's bands — a confident wrong answer, which is the failure mode this
repo refuses everywhere else. `Ring::copy` already returns `None` rather than the nearest thing it
still holds, for exactly this reason.

### 8. What is out of scope, and where it went

- **Phone over Wi-Fi** → **ROADMAP §M28**, blocked on M25. ADR-030 §5 says the phone app comes
  *after* the laptop app works end to end, so it was never an M21 phase; §M21 listing it as "source
  3" was an ordering error this ADR corrects.
- **Pose.** Capture writes clips; ADR-030 §3's worker pool reads them.
- **Live preview rendering.** A Flutter concern (ADR-030 §4), and M25's.
- **`FrameBundle` and multi-view fusion.** ADR-011 and ADR-015 own those, and neither is reopened.

## Consequences

- **M21 is un-blocked from M22, and stops being a "needs no Rust" milestone.** The ROADMAP has said
  since M18 that *"M21's first source needs no Rust"*; §2 shows that source is finished, so what
  remains is Rust from P1 on. The status table and the group header's **Order** paragraph both
  change.
- **`crates/capture` is the second crate**, and the workspace laid out in M20 P0 for exactly this
  reason takes it without restructuring.
- **The Rust half gains its first non-numeric dependency.** `rustfft` was justified as the single
  sanctioned exception for the scoring path; a camera crate is a different kind of dependency in a
  different place, and §4 draws the line where it actually belongs.
- **Two `capture/` docstrings become lies on the commit that lands this** unless they are corrected
  in it. They are, in P0.
- **`storage/manifest.py` gains a writer it was not designed for, and needs no change to take it.**
  If that turns out false, the finding belongs in an addendum here — it would mean the per-swing
  directory is less of a contract than this ADR is betting on.
- **Nothing in `analysis/` moves and `ANALYSIS_VERSION` does not bump.** No band, no checkpoint and
  no vector is touched by anything decided here.

## Deferred, by choice

- **A capture conformance family in `spec/vectors/`.** There is nothing to compare against — capture
  has no Python reference to be a port *of*, which is the one thing that makes M19's oracle pattern
  work. Revisit if a second capture implementation ever exists; the phone's, in M28, would be it.
- **Hardware encoding, host versus camera.** §6 prices the ring in encoded frames; whether the encode
  is the camera's (most UVC devices offer MJPEG, some H.264) or the host's is P2's measurement, not a
  decision here.
- **A frame-accurate two-camera start.** ADR-025 recovers the offset acoustically after the fact, so
  starting the two cameras together is a convenience and not a requirement. Not pursued.

## Addendum, 2026-09-22 — what a replug actually survives, and a bake-off that could not run

**M21 P1 built `crates/capture` and could not finish the phase.** Two clauses above met hardware
and came back changed. Both are recorded here rather than edited into the text, because both are
corrections *by* reality and that is what this section is for.

### §7 is right, and "survives a replug" is doing more work than it can

§7 asks for *"a stable identifier that survives a replug, not an index"*. On Windows that is
Media Foundation's `MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK`, and for the camera
this repo owns it reads

```text
\?\USB#VID_1BCF&PID_28C4&MI_00#6&1da11eb4&2&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}
```

which was read out of `HKLM\SYSTEM\CurrentControlSet\Control\DeviceClasses\{e5323777-…}` while
the camera was **unplugged** — the interface registration outlives the device, which is how a
phase with no hardware got a real string to reason about at all.

**The middle segment is not a serial number.** It is the instance id Windows *generates* from the
hub and the port, which is what it does for a USB device that reports no serial of its own. The
contrast is on the same machine: a device that does report one enumerates as
`USB\VID_0CF2&PID_A100\6243168001` — the serial, verbatim, in that position.

So the honest statement of §7, for this hardware, is: **the identity names the device and the
port.**

- A replug into the same port keeps it. That is the case §7 was written about and it holds.
- A move to a different port changes it, and a session must then say the camera is **gone** —
  which is exactly what §7 already requires, so the *behaviour* needs no amendment. Only the
  sentence explaining it does.
- Two identical cameras are told apart **only** by port.

That last point is why this is a property and not a limitation. ADR-003 buys two of the same
camera, and what makes one of them the face-on view is which tripod it is bolted to — not which
of two indistinguishable serial-less sensors it happens to be. The port is the closest thing the
host has to that fact, and it is the *right* identity for a fixed rig rather than the best
available compromise.

**What it costs, and it is P5's rather than this clause's:** two identical cameras cannot be told
apart before they are first plugged in, so the face-on/down-the-line assignment is a setup step a
human performs once per port and not something capture can infer. `storage/manifest.py`'s `Role`
is where that assignment lands, and it needs no change to take it.

### §4's bake-off did not happen, and the crate choice is provisional

§4 requires the camera crate be chosen *"against hardware rather than against documentation"*.
**There was no hardware.** The machine this repo is built on is a desktop — MSI MS-7D25 — with no
built-in camera, and the only camera Windows holds a record of is the phantom above:
`CM_PROB_PHANTOM`, code 45, not connected. `nokhwa::query` returns zero devices and so does
`crates/capture`. A bake-off with nothing to enumerate decides nothing, so **§4 is still open**
and `crates/capture` calls Media Foundation directly as a provisional choice.

Two facts stand behind that call, and only the first is a measurement:

- **`nokhwa` 0.10.11 does not build under this workspace's `rust-version = "1.87"`.** It pulls
  `image` 0.25.10, which requires 1.88. It builds with `default-features = false` and `image`
  pinned back to 0.25.5 — but its default `decoding` feature pulls in `mozjpeg-sys` and
  `nasm-rs`, a C and NASM build toolchain, in the half of this repo whose whole dependency list
  is four crates. That is a real cost against §4's "first non-numeric third-party dependency"
  framing, and the kind of thing this ADR's Consequences were watching for.
- **`nokhwa`'s Windows backend *is* Media Foundation**, through the same `windows` crate
  `crates/capture` now calls. Going direct gives up one API over three platforms and buys the
  symbolic link unabridged, which is what the clause above is about.

Neither is a verdict, and the second is documentation — the thing §4 forbids deciding on. What
settles it, with a camera attached: whether `nokhwa` reports capabilities this module cannot, or
this module misses a device `nokhwa` sees.

### The ROADMAP's hardware gating was wrong for this machine, and P1–P3 are not desk work

§M21 gated P1–P3 on *"the built-in webcam"* and P4 on *"the USB camera"*, on the premise ADR-030
§5 set — *"the machine is a laptop"*. The machine this is **built** on is not one; the laptop is
the machine the app will eventually **run** on. So the USB camera is wanted three phases earlier
than planned, and §M21 now says so.

A smaller finding than the two above, recorded for the reason M20's cut rules were: a phase that
believes it needs no hardware gets scheduled as desk work, and then spends its session finding
out that it is not.
