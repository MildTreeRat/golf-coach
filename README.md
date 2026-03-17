# AI Golf Swing Trainer

A home-lab AI-powered golf swing analysis system that captures your swing via camera, integrates launch monitor data, analyzes mechanics, and delivers coaching feedback.

## Project Status

**Phase**: Planning complete, ready for Milestone 1

See [ROADMAP.md](ROADMAP.md) for current progress and [WORKLOG.md](WORKLOG.md) for session-by-session notes.

## Architecture

The system has six modules with clean interfaces between them:

- **Capture** — camera input, video recording, frame extraction
- **Pose Estimation** — MediaPipe body keypoint extraction (33 landmarks/frame)
- **Club/Ball Detection** — YOLOv8 fine-tuned for club head and ball tracking
- **Launch Monitor (MCP Server)** — ingests shot data from hardware, exposes it via MCP tools
- **Analysis Engine** — merges all data streams, segments swing phases, evaluates checkpoints, scores the swing
- **Feedback** — rule-based tips, Claude API coaching, visual overlays displayed in a web UI

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for full diagrams and interface contracts.

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

```
golf-swing-ai/
├── README.md
├── ROADMAP.md
├── WORKLOG.md
├── docs/
│   ├── PROJECT_CHARTER.md
│   ├── ARCHITECTURE.md
│   └── decisions/           # Architecture Decision Records
│       ├── 000-template.md
│       ├── 001-language-python.md
│       ├── 002-pose-estimation-mediapipe.md
│       ├── 003-camera-hardware.md
│       ├── 004-launch-monitor.md
│       ├── 005-object-detection-yolov8.md
│       └── 006-mcp-server.md
├── src/
│   ├── capture/             # Camera + video recording
│   ├── analysis/            # Swing phase segmentation + checkpoint scoring
│   ├── feedback/            # Rule engine + LLM coaching + overlays
│   ├── mcp_server/          # Launch monitor MCP server
│   └── ui/                  # React web dashboard
├── tests/
└── data/
    ├── raw/                 # Raw video files
    ├── processed/           # Extracted keypoints, labeled images
    └── models/              # Trained model weights
```

## Getting Started

*Coming in Milestone 1 — setup instructions will be added once the first working code exists.*

## Decision Log

All architectural and technology decisions are documented as ADRs in `docs/decisions/`. See [000-template.md](docs/decisions/000-template.md) for the format.
