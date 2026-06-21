# ADR-007: Decouple Software Development from Hardware Acquisition

## Status
Accepted

## Date
2026-06-20

## Context
The original milestone plan (ROADMAP.md) and worklog treated hardware as a gate
on all progress. The first worklog entry lists "purchase camera hardware (ADR-003)"
and "purchase launch monitor (ADR-004)" as blockers, and Milestone 1 is written as
if the ELP AR0234 camera must be in hand before any capture/pose work can begin.

Re-examining this 3 months later: the hardware *decisions* are already made (ADR-003,
ADR-004 are Accepted) — what's missing is the *purchase*, plus shipping/setup time.
Meanwhile the repo has zero code. Blocking all software work on a hardware purchase
means the most significant technical risk (does the pose/detection/analysis pipeline
actually work?) stays unretired while we wait on shipping.

Crucially, most early software work does not need the final hardware:
- **MediaPipe pose estimation** runs on *any* video — a phone clip or a YouTube swing
  is enough to validate that pose tracking holds up through a full golf swing.
- **Project scaffolding, data schemas, the analysis engine skeleton, and the MCP
  server** can all be built and tested against sample or simulated data.
- The **global-shutter camera (ADR-003)** only becomes essential at Milestone 2,
  where club-head motion blur actually breaks YOLOv8 training/detection.
- The **Garmin R10 (ADR-004)** only becomes essential at Milestone 3; until then the
  MCP server can serve mock/simulated `ShotData` (Option F from ADR-004).

## Options Considered

### Option A: Hardware-gated (original implicit plan)
- **Pros**: Every component is tested against real, final-quality data from day one.
- **Cons**: All software progress blocked on purchasing + shipping + setup. Technical
  risk in the software pipeline stays unretired during the wait. Wastes the "free"
  time we have before hardware is needed.

### Option B: Decouple software track from hardware track (run in parallel)
- **Pros**: Software risk gets retired immediately using sample/phone video and
  simulated shot data. Hardware is purchased in parallel so it's ready exactly when
  the milestones that *need* it (M2, M3) arrive. No idle waiting. Matches the project's
  low-coupling architecture — swapping a data *source* (mock → real) shouldn't require
  rewriting consumers.
- **Cons**: Some early validation happens against lower-quality input (phone/rolling-
  shutter video, mock shot data), so a few findings must be re-confirmed once real
  hardware arrives. Two parallel tracks to manage.

### Option C: Simulated data only, defer all hardware indefinitely
- **Pros**: Zero spend, never blocked.
- **Cons**: Never validates against real swings/shots — defeats the project's purpose
  (integrate real hardware, hit real balls, close the feedback loop).

## Decision
**Option B — decouple the two tracks.** Software development proceeds now against
sample/phone video and simulated `ShotData`; hardware (ELP AR0234 cameras, Garmin R10)
is purchased in parallel so it lands before the milestones that require it.

This makes the *hardware-dependency* of each milestone explicit rather than implicit:

| Milestone | Hardware needed to START | Can begin now with |
|-----------|--------------------------|--------------------|
| M1 Capture & Skeleton | None | Phone video / sample swing clips |
| M2 Club & Ball Detection | Global-shutter camera (ADR-003) | Scaffolding + labeling workflow on sample frames |
| M3 Launch Monitor / MCP | Garmin R10 (ADR-004) | MCP server + schema against mock `ShotData` |
| M4 Analysis Engine | None (uses merged data) | Real or simulated merged data |
| M5 Feedback UI | None | — |
| M6 LLM Coaching | None | — |

## Consequences
- Milestone 1 starts immediately with no purchase required.
- The MCP server (M3) must support a **mock/simulated `ShotData` mode** from the start,
  toggling to real R10 data later — this is a feature, not a workaround, and reinforces
  the low-coupling goal (ADR-006: swapping hardware only changes the parser).
- A small amount of early validation (pose stability, club-head detectability) is done
  on lower-quality video and must be **re-confirmed** once the global-shutter camera
  arrives. Treat early findings as provisional until then.
- Hardware purchase becomes its own tracked task, run in parallel — not a blocker.
- ROADMAP.md is updated to annotate each milestone's hardware dependency explicitly.
