# AI Golf Swing Trainer

A home-lab AI-powered golf swing analysis system that captures your swing via camera, integrates launch monitor data, analyzes mechanics, and delivers coaching feedback.

## Project Status

**Phase**: Project scaffolded (structure + shared contracts in place). Starting Milestone 1.

See [ROADMAP.md](ROADMAP.md) for current progress and [WORKLOG.md](WORKLOG.md) for session-by-session notes.

## Architecture

The system has six modules with clean interfaces between them:

- **Capture** — camera input, video recording, frame extraction
- **Pose Estimation** — MediaPipe body keypoint extraction (33 landmarks/frame)
- **Club/Ball Detection** — YOLOv8 fine-tuned for club head and ball tracking
- **Launch Monitor (MCP Server)** — ingests shot data from hardware, exposes it via MCP tools
- **Analysis Engine** — merges all data streams, segments swing phases, evaluates checkpoints, scores the swing
- **Feedback** — rule-based tips, Claude API coaching, visual overlays displayed in a web UI

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for full diagrams and interface contracts,
and [docs/FLOW.md](docs/FLOW.md) for the proposed end-to-end flow, decoupling seam, and build order (mermaid diagrams).

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Python 3.11+ (backend/ML), JavaScript/React (UI) |
| Pose Estimation | MediaPipe Pose |
| Object Detection | YOLOv8 (Ultralytics) |
| Video Processing | OpenCV |
| ML Framework | PyTorch |
| Backend API | FastAPI |
| MCP Server | Python (MCP SDK) |
| Database | SQLite |
| LLM | Claude API (Anthropic) |
| Frontend | React |

## Project Structure

The layout and the reasoning behind it are documented in [ADR-008](docs/decisions/008-project-structure.md).
The core idea: a shared `contracts/` package is the seam every module depends on (modules
never import each other), I/O boundaries use swappable real/mock adapters, and the analysis
engine is a pure functional core. This is what lets sections be built independently and run
on simulated data before any hardware exists.

```
golf-coach/
├── README.md  ROADMAP.md  WORKLOG.md
├── pyproject.toml           # deps (base + vision/api/llm/hardware extras) + tooling
├── docs/
│   ├── PROJECT_CHARTER.md
│   ├── ARCHITECTURE.md
│   └── decisions/           # ADRs 000–010
├── src/
│   └── golf_coach/
│       ├── contracts/       # ⭐ shared data shapes (Pydantic) — the decoupling seam
│       ├── capture/         # VideoSource port + file/camera adapters
│       ├── pose/            # MediaPipe → FrameKeypoints
│       ├── detection/       # YOLOv8 → FrameDetections + club-path tracker
│       ├── launch_monitor/  # ShotDataSource port + mock/r10 adapters + MCP server
│       ├── analysis/        # ⭐ pure functional core: merge→phases→checkpoints→score
│       ├── feedback/        # rules + Claude coaching + overlays
│       ├── storage/         # SQLite repositories
│       ├── api/             # FastAPI orchestrator
│       └── config.py        # settings (the only env reader)
├── frontend/                # React UI (M5) — separate toolchain, talks to api/ over HTTP
├── tests/                   # mirrors the package; seam tests run on the base install
├── spikes/                  # throwaway exploration (e.g. the M1.5 detectability spike)
├── scripts/                 # dev CLI entrypoints (run_pose.py, run_mcp_server.py, …)
└── data/                    # gitignored: raw/ processed/ models/ + SQLite db
```

## Setup

```bash
python -m venv .venv && source .venv/bin/activate
pip install -e '.[dev]'            # base + dev tools; contracts/tests need nothing heavier
pip install -e '.[vision,dev]'     # add when starting M1 (MediaPipe/OpenCV/YOLOv8)
pytest                             # runs the contracts seam tests
```

## Getting Started

*Coming in Milestone 1 — setup instructions will be added once the first working code exists.*

## Decision Log

All architectural and technology decisions are documented as ADRs in `docs/decisions/`. See [000-template.md](docs/decisions/000-template.md) for the format.
