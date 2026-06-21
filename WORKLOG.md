# Work Log

Write a short entry every time you sit down to work. Reverse chronological (newest first).
This is your "pick up where I left off" document.

---

## 2026-06-20

**Duration**: ~1.5 hours
**What I did**: Picked the project back up after the planning session. Reviewed all docs/ADRs to re-orient. Decided to decouple software development from hardware acquisition and wrote ADR-007 to capture it; annotated ROADMAP milestones with explicit hardware dependencies. Discussed the role of YOLOv8 and how swing path is captured. Assessed how hard reliable club-head tracking really is and decided to de-risk it before investing — added **Milestone 1.5: Club-Head Detectability Spike** with a go/no-go gate. Then designed and scaffolded the whole project per ADR-008: `src/golf_coach/` package, the shared `contracts/` seam (fully implemented Pydantic models), ports + a working mock shot source, module stubs, pyproject with optional extras, tests, scripts, spikes/, frontend/, data/. Verified: `pip install -e '.[dev]'` works, `pytest` green (4 contract tests), `ruff` clean. Added `docs/FLOW.md` with PROPOSED-flow mermaid diagrams (runtime data flow, decoupling seam, build order + hardware gates, two-source swing path) and flagged ARCHITECTURE.md's diagrams as proposed too.
**Key decisions**:
- ADR-007 — software and hardware tracks run in parallel. M1 (capture + MediaPipe pose) starts now with phone/sample video, no purchase needed. MCP server (M3) gets a mock `ShotData` mode from the start. Hardware purchase is a parallel task, not a blocker.
- Added M1.5 detectability spike: prove the club head is detectable (especially through impact) BEFORE labeling 200–500 images. Strategy options to choose from once we see real frames: pure-ML / marker-assisted / fusion+interpolation. M2 labeling is now gated on this spike. Charter risk register and ADR-003 updated accordingly.
- ADR-008 — project structure & decoupling: a shared `contracts/` package is the seam (modules never import each other), ports+adapters at I/O boundaries (real vs mock), analysis is a pure functional core, heavy deps are optional extras so any module runs without the ML stack.
**Clarified**: YOLOv8 detects the club head + ball (MediaPipe only tracks the body); its detections through a tracker produce the visual club-path arc. The Garmin R10's `club_path` is the numeric counterpart. YOLOv8 is also the train-it-yourself ML exercise (ADR-005) and the most cuttable piece if scope tightens. Important correction: **global shutter removes distortion, not motion blur** — sharp impact frames need fast shutter + bright light (ADR-003 addendum).
**Where I left off**: Project is scaffolded and the contracts seam is real (tests green). Next concrete step is implementing **M1**: install the `vision` extra, write `FileVideoSource` (capture/file.py), implement `estimate_pose` (pose/estimator.py) over a sample/phone clip, and render a skeleton overlay via `scripts/run_pose.py`. In parallel, the M1.5 spike — grab a few swing clips with the impact zone and eyeball club-head detectability before any labeling.
**Blockers**: None for M0/M1/M1.5. Hardware purchase (cameras + R10) pending but no longer blocking.
**Notes**: —

---

## 2026-03-16

**Duration**: ~1 hour
**What I did**: Project planning session. Created project charter, architecture doc, roadmap, ADR templates, and decision log. Defined tech stack and milestone sequence.
**Key decisions**: Python stack, MediaPipe for pose, YOLOv8 for detection, MCP server for launch monitor, Claude API for coaching. See `docs/decisions/` for full ADRs.
**Where I left off**: No code yet. Next step is Milestone 1 — acquire/set up camera, record a test swing, run MediaPipe.
**Blockers**: Need to decide on and purchase camera hardware (ADR-003) and launch monitor (ADR-004).
**Notes**: —

---

<!--
TEMPLATE — copy this for each session:

## YYYY-MM-DD

**Duration**: 
**What I did**: 
**Key decisions**: 
**Where I left off**: 
**Blockers**: 
**Notes**: 

-->
