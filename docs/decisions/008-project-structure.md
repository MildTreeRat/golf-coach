# ADR-008: Project Structure & Decoupling Strategy

## Status
Accepted

## Date
2026-06-20

## Context
Before writing real code we need a project layout that (a) follows good design
principles and (b) keeps the six architecture modules (Capture, Pose, Detection,
Launch Monitor, Analysis, Feedback + UI) decoupled enough to be built and tested
independently. Decoupling isn't just tidiness here — it's what makes ADR-007 work:
we need to build most of the system against sample video and simulated shot data
before any hardware is bought.

The README's original sketch was a flat `src/{capture,analysis,feedback,mcp_server,ui}`
with the React UI nested under `src/` and no shared data layer. That invites modules
to import each other and mixes Python and JS toolchains.

## Options Considered

### Option A: Flat module folders, modules import each other directly
- **Pros**: Fewer files. Obvious.
- **Cons**: Modules become coupled (pose imports analysis imports feedback...). Can't
  work on or test one module without the others. No clean place to swap real vs. mock
  data sources. Cycles creep in.

### Option B: Shared `contracts` package + ports/adapters + functional core (chosen)
- **Pros**: A single `contracts` package holds the cross-module data shapes; every
  module depends on *it*, never on each other. I/O boundaries (camera, launch monitor,
  Claude) are defined as Protocol "ports" with swappable real/mock "adapters". The
  analysis engine is a pure functional core (data in, data out, no I/O). Each section is
  independently buildable and testable; mock data is a first-class citizen. Directly
  enables ADR-006/007.
- **Cons**: More upfront structure and a few more files. Slight indirection.

### Option C: Multi-package monorepo (one installable package per module)
- **Pros**: Hardest possible boundaries; each module versioned separately.
- **Cons**: Overkill for a solo project. Heavy packaging/release ceremony for little gain.

## Decision
**Option B.** Single installable package `golf_coach` under an `src/` layout, organized as:

```
src/golf_coach/
  contracts/      # ⭐ the seam: Pydantic data shapes; depends only on pydantic+stdlib
  capture/        # I/O edge: VideoSource port + file/camera adapters
  pose/           # frames -> FrameKeypoints (MediaPipe)
  detection/      # frames -> FrameDetections + tracker (YOLOv8)
  launch_monitor/ # ShotDataSource port + mock/r10 adapters + MCP server
  analysis/       # ⭐ pure functional core: merge -> phases -> checkpoints -> score
  feedback/       # rules + Claude coach + overlay render
  storage/        # SQLite repositories
  api/            # FastAPI orchestrator (imperative shell)
  config.py       # the only place that reads the environment
frontend/         # React UI — separate toolchain, talks to api/ over HTTP
tests/            # mirrors the package; runs on the base install alone
spikes/           # throwaway exploration (e.g. the M1.5 detectability spike)
scripts/          # dev CLI entrypoints
data/             # gitignored: raw/, processed/, models/, db
```

### Principles enforced
1. **Contracts are the only shared dependency.** Modules import `golf_coach.contracts`,
   never each other. Data flows producer -> contract -> consumer.
2. **Ports & adapters at every external boundary.** `VideoSource` and `ShotDataSource`
   are Protocols; real (camera/R10) and mock (file/synthetic) adapters are interchangeable.
3. **Functional core, imperative shell.** `analysis` and the rule engine are pure; all
   I/O lives at the edges (`capture`, `launch_monitor`, `api`).
4. **Dependency-level decoupling.** Heavy libs are optional extras (`vision`, `api`,
   `llm`, `hardware`); the base install is tiny so any module — and the whole test suite
   for contracts — runs without the ML stack.

### Dependency rule
`contracts` ← every module. `analysis` depends only on `contracts`. `api` orchestrates
the modules. `frontend` talks to `api` over HTTP. No import cycles, ever.

## Consequences
- Analysis (M4), feedback (M5/M6), and the MCP server (M3) can be built and tested against
  mock/simulated data before hardware exists — the seam makes "mock mode" trivial.
- Swapping hardware (mock -> R10, file -> live camera) changes one adapter, nothing else.
- Tests for the seam run with only `pip install -e .` (no MediaPipe/YOLO needed).
- A little more boilerplate (ports, contracts) up front; worth it for independent workstreams.
- Tooling standardized in `pyproject.toml` (ruff, pytest, mypy); `src/` layout avoids
  import-path foot-guns.
