# ADR-011: Camera Synchronization & Multi-View 3D Fusion

## Status
Proposed (forward-looking — no multi-camera hardware acquired yet; the seam is designed now,
implemented in phases)

## Date
2026-07-02

## Context

We have settled on **two camera angles** (ADR-003 addendum 2026-07-02b), each carrying
different streams:
- **Face-on (3 o'clock)** — body landmark tracking (knees, hands, sway, weight shift, tempo).
- **Down-the-line** — forward spine tilt, swing plane, and club/ball detection.

Each single 2D view **foreshortens** some measurements. We proved this empirically: the same
address posture measured **~37° of spine tilt from the ~5 o'clock view but only ~2° face-on**
(the forward bend lives in the camera's depth axis and collapses to nearly zero from
face-on). MediaPipe's monocular depth (`z`) is too noisy/uncalibrated to trust as a fix — it
put the same posture anywhere from 54° to 71° depending on the clip.

The measurements we ultimately want for the M4 **mechanics** checkpoints — true spine angle,
hip rotation, **X-factor** (shoulder–hip separation), and the **kinematic sequence** — are
inherently 3D and cannot be read reliably from any single 2D camera. The way to get them is
to **fuse the two views by triangulation**, and triangulation is only valid if we know which
face-on frame corresponds to which down-the-line frame. That correspondence is
**synchronization**, and it is the enabling capability for everything 3D. Hence this ADR.

### Why sync is hard here (the forces)
- The swing is fast. At 120 fps a frame is ~8.3 ms; the club head travels ~16 in *between
  frames* near impact, and body landmarks move meaningfully too.
- **Sync tolerance is phase-dependent.** Static-ish moments (address posture) tolerate a
  loose offset; dynamic moments (transition, top-of-backswing X-factor, impact) need
  frame-accurate or sub-frame alignment or the triangulated 3D angle is simply wrong.
- Two free-running USB cameras drift (independent clocks) and UVC introduces variable host
  latency, so "just use host timestamps" is only coarse.

## Options Considered

### Option A: Hardware trigger sync (external trigger)
Drive both cameras from a common trigger pulse (a microcontroller / Pi GPIO) so they expose
simultaneously. The ELP AR0234 (ADR-003) was chosen partly for its **external + software
trigger** support — this is that payoff.
- **Pros**: True simultaneity, deterministic, sub-millisecond. Correct even at impact. The
  gold standard for fast motion.
- **Cons**: Wiring + a trigger source; more setup. Must confirm the specific ELP model's
  trigger mode is usable over UVC (some trigger features need vendor tooling).

### Option B: Software timestamp sync
Capture both UVC streams and align by host-clock timestamps in software.
- **Pros**: No extra hardware; works with any UVC camera (or phones) today.
- **Cons**: USB/UVC latency + jitter → tens of ms of error, plus inter-camera frame-rate
  drift. Fine for slow phases (address, backswing), not frame-accurate through impact.

### Option C: Event / visual fiducial sync
Align the two recordings post-hoc on a shared event both cameras see — an LED blink / clap,
the ball-strike (impact) frame, or later the Garmin R10 impact event.
- **Pros**: No special hardware; robust; a natural fiducial (impact) both views capture.
  Great for *refining* Option B.
- **Cons**: Anchors one instant; drift between events remains; sub-frame needs interpolation.

### Option D: Genlock / dedicated mocap rig
- **Pros**: Broadcast-grade sync.
- **Cons**: Cost/complexity far beyond a home lab. Out of scope.

## Decision

Adopt a **phased plan**, and — most importantly — **design the capture seam now** so later
phases slot in without reworking contracts (same philosophy as ADR-008).

- **Phase 1 (now, single camera):** No sync required. One face-on camera. But build the
  capture layer so every frame already carries a `camera_id` **and** a reliable
  `timestamp_ms`. This is the seam that makes sync possible later for free.
- **Phase 2 (two cameras, software sync):** Add the down-the-line camera. Start with
  **Option B refined by Option C** — host timestamps for coarse alignment, snapped to a
  shared visual fiducial (LED/clap, then the impact frame, later the R10 impact event). Good
  enough for posture and slow-phase 3D.
- **Phase 3 (frame-accurate, hardware trigger):** Move to **Option A** (external trigger via
  a small microcontroller) once we need trustworthy *dynamic* 3D — X-factor at the top,
  transition sequence, impact kinematics. This is the AR0234 trigger-support payoff.

Calibration is a prerequisite for true 3D and is called out here even though it is
implemented alongside Phase 2: **intrinsics** per camera (checkerboard/ChArUco) and
**extrinsics** (relative pose) from a calibration target visible in both views.

## Architecture (the seam to build now)

- **Capture:** extend `Frame` (capture/source.py) — which already carries `timestamp_ms` —
  with a `camera_id`. Each camera is its own `VideoSource` stream; nothing else changes in
  Phase 1.
- **Synchronize:** a new stage that pairs frames across the per-camera streams into a
  `FrameBundle` (the set of frames sharing one swing-instant, within a tolerance). In Phase 1
  a bundle trivially wraps a single camera; the downstream code is written against bundles
  from day one so multi-view is not a rewrite.
- **Per-view inference:** pose (and detection) run **per camera** as today, producing
  `FrameKeypoints` tagged with `camera_id`.
- **Fusion:** a new module (`fusion/`, or within `analysis/`) takes the paired per-view
  `FrameKeypoints` + camera calibration → **triangulated 3D landmarks**. The 3D mechanics
  checkpoints (spine angle, hip rotation, X-factor, kinematic sequence) consume these instead
  of guessing from monocular `z`.

## Consequences
- Building the `camera_id` + `timestamp_ms` + `FrameBundle` seam now (even single-camera)
  means Phases 2–3 add cameras and a fusion stage **without touching contracts** downstream.
- True 3D unlocks exactly the mechanics that single 2D cannot do reliably — the reason the
  spine-angle investigation was ambiguous (ADR-003 addendum 2026-07-02b).
- Cost/complexity is deferred: the hardware trigger is Phase 3, not needed to begin.
- **Watch-out:** two AR0234 at 120 fps can saturate a shared USB3 controller — plan separate
  USB controllers/ports (or a powered hub on distinct root hubs).
- Does **not** block current milestones (M1 pose, M4-PoC tempo) — those are single-camera,
  Phase 1.

## References
- ADR-003 (camera hardware; trigger support was chosen with this in mind) + its 2026-07-02
  addenda (face-on pose placement; two-camera stream assignment & spine caveat).
- ADR-007 (decouple software from hardware — this ADR keeps sync off the critical path).
- ADR-008 (project structure — the "design the seam now" approach mirrors it).
