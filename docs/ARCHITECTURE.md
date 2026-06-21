# Architecture Document: AI Golf Swing Trainer

## Last Updated: 2026-06-20

> ⚠️ **The diagrams below describe the PROPOSED / target design — not yet built.** As of
> 2026-06-20 only the scaffolding + `contracts/` seam + `MockShotDataSource` exist. For the
> current proposed runtime flow, decoupling seam, and build order (incorporating ADR-007 and
> ADR-008), see [FLOW.md](FLOW.md). Update both as milestones land.

---

## 1. Component Diagram

Shows the major modules, what each owns, and the interfaces between them.

```mermaid
graph TB
    subgraph Capture ["Module: Capture"]
        CAM[Camera Input]
        VID[Video Recorder]
        FRAME[Frame Extractor]
    end

    subgraph Pose ["Module: Pose Estimation"]
        MP[MediaPipe Pose]
        KP[Keypoint Serializer]
    end

    subgraph Detection ["Module: Club/Ball Detection"]
        YOLO[YOLOv8 Model]
        TRACK[Object Tracker]
    end

    subgraph LaunchMonitor ["Module: Launch Monitor"]
        LM_HW[Launch Monitor Hardware]
        LM_PARSE[Data Parser]
        MCP[MCP Server]
    end

    subgraph Analysis ["Module: Analysis Engine"]
        MERGE[Data Merger]
        PHASE[Swing Phase Segmenter]
        CHECK[Checkpoint Evaluator]
        SCORE[Swing Scorer]
    end

    subgraph Feedback ["Module: Feedback"]
        RULE[Rule-Based Feedback]
        LLM[Claude API - LLM Coaching]
        OVERLAY[Visual Overlay Generator]
    end

    subgraph UI ["Module: Web UI"]
        DASH[Dashboard]
        REPLAY[Video Replay]
        HISTORY[Session History]
    end

    subgraph Storage ["Storage"]
        DB[(SQLite)]
        FS[File System - Videos/Models]
    end

    CAM --> VID --> FRAME
    FRAME --> MP --> KP
    FRAME --> YOLO --> TRACK
    LM_HW --> LM_PARSE --> MCP

    KP --> MERGE
    TRACK --> MERGE
    MCP --> MERGE

    MERGE --> PHASE --> CHECK --> SCORE

    SCORE --> RULE --> DASH
    SCORE --> LLM --> DASH
    SCORE --> OVERLAY --> REPLAY
    SCORE --> DB
    DB --> HISTORY --> DASH
```

### Interface Contracts (key data shapes)

| Interface | From → To | Data Shape |
|-----------|-----------|------------|
| Keypoints | Pose → Analysis | `List[FrameKeypoints]` — 33 landmarks per frame with x, y, z, visibility |
| Detections | Detection → Analysis | `List[FrameDetections]` — bounding boxes + class (club_head, ball) per frame |
| Shot Data | MCP → Analysis | `ShotData` — club_speed, ball_speed, launch_angle, spin_rate, club_face_angle, club_path, smash_factor |
| Swing Result | Analysis → Feedback | `SwingResult` — phase_timestamps, checkpoint_scores, overall_score, merged keypoint+detection+shot data |
| Feedback | Feedback → UI | `FeedbackPayload` — score, list of tips (text), overlay frames, LLM coaching response |

---

## 2. Data Flow Diagram

Traces how data transforms from raw input to user-facing output.

```mermaid
flowchart LR
    subgraph Input
        A[Raw Video Frames]
        B[Launch Monitor Raw Data]
    end

    subgraph Transform
        C[Body Keypoints\n33 landmarks/frame]
        D[Club/Ball Detections\nbbox + class/frame]
        E[Normalized Shot Metrics\nstandardized units]
    end

    subgraph Analyze
        F[Merged Timeline\nkeypoints + detections + shot\naligned by timestamp]
        G[Swing Phases\naddress → backswing → transition\n→ downswing → impact → follow-through]
        H[Checkpoint Scores\nper-checkpoint pass/fail/score]
        I[Overall Swing Score\n0-100]
    end

    subgraph Output
        J[Rule-Based Tips\nstructured text]
        K[LLM Coaching\nconversational text]
        L[Annotated Video\noverlay frames]
    end

    A --> C
    A --> D
    B --> E
    C --> F
    D --> F
    E --> F
    F --> G --> H --> I
    I --> J
    I --> K
    H --> L
```

### Storage Points

| What Gets Stored | Where | Why |
|-----------------|-------|-----|
| Raw video files | `data/raw/sessions/{session_id}/` | Replay, reprocessing |
| Keypoint data | SQLite `keypoints` table | Avoid recomputing pose |
| Shot data | SQLite `shots` table | History, trends |
| Swing results | SQLite `swing_results` table | Progress tracking |
| Trained models | `data/models/` | Versioned model weights |

---

## 3. Sequence Diagram

Shows the order of operations for a single swing event.

```mermaid
sequenceDiagram
    actor User
    participant Cam as Camera
    participant Pose as Pose Estimation
    participant Det as Club Detection
    participant LM as Launch Monitor
    participant MCP as MCP Server
    participant Eng as Analysis Engine
    participant FB as Feedback Generator
    participant UI as Web UI

    User->>Cam: Swings club
    
    par Video Capture
        Cam->>Cam: Record frames (2-4 seconds)
    and Launch Monitor
        LM->>LM: Capture shot data
    end

    Cam->>Pose: Send frames
    Cam->>Det: Send frames (parallel)
    LM->>MCP: Push shot data

    par Processing (parallel)
        Pose->>Pose: Extract keypoints per frame
        Det->>Det: Detect club/ball per frame
    end

    Pose->>Eng: Keypoint timeline
    Det->>Eng: Detection timeline
    MCP->>Eng: Shot metrics

    Eng->>Eng: Merge data streams
    Eng->>Eng: Segment swing phases
    Eng->>Eng: Evaluate checkpoints
    Eng->>Eng: Compute score

    par Feedback Generation
        Eng->>FB: Swing result
        FB->>FB: Generate rule-based tips
        FB->>FB: Call Claude API for coaching
        FB->>FB: Render video overlays
    end

    FB->>UI: FeedbackPayload
    UI->>User: Display score, tips, annotated video
```

### Timing Expectations

| Step | Expected Duration |
|------|------------------|
| Video capture | 2-4 seconds (swing duration) |
| Pose estimation | 1-3 seconds (depends on frame count + GPU) |
| Club detection | 1-2 seconds |
| Analysis | <0.5 seconds |
| Rule-based feedback | <0.1 seconds |
| LLM coaching call | 2-5 seconds |
| **Total latency** | **~5-15 seconds from swing to feedback** |

---

## 4. Deployment / Physical View

What runs where and how to start it.

```mermaid
graph TB
    subgraph HomeLab ["Home Lab Machine"]
        subgraph Services ["Long-Running Services"]
            MCP_SRV["MCP Server\n(Python, port 8081)\nExposes shot data tools"]
            WEB["Web UI\n(React dev server, port 3000)\nDashboard + replay"]
            API["Backend API\n(FastAPI, port 8080)\nOrchestrates pipeline"]
        end

        subgraph Scripts ["Run-on-Demand"]
            TRAIN["Training Scripts\nYOLOv8 fine-tuning\nRun manually when retraining"]
            LABEL["Labeling Tools\nLabel Studio (port 8090)\nRun when labeling data"]
        end

        subgraph HW ["Connected Hardware"]
            CAM_HW["USB Camera\n/dev/video0"]
            LM_HW["Launch Monitor\nBluetooth/USB/WiFi"]
        end

        DB_FILE["SQLite DB\ndata/golf_trainer.db"]
        MODEL_FILES["Model Weights\ndata/models/"]
    end

    CAM_HW --> API
    LM_HW --> MCP_SRV
    MCP_SRV --> API
    API --> DB_FILE
    API --> MODEL_FILES
    API --> WEB
```

### Startup Sequence

```bash
# 1. Start MCP server (launch monitor data)
cd src/mcp_server && python server.py

# 2. Start backend API (orchestrates everything)
cd src/ && uvicorn api:app --port 8080

# 3. Start web UI
cd src/ui && npm start

# 4. (Optional) Start Label Studio for data labeling
label-studio start --port 8090
```
