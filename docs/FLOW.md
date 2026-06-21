# Project Flow (PROPOSED)

> ⚠️ **PROPOSED FLOW — not yet built.** These diagrams describe the *intended* end-to-end
> design and build order as of **2026-06-20**, incorporating the decoupling decisions in
> ADR-007 and the structure in ADR-008. Today only the scaffolding + the `contracts/` seam
> + the `MockShotDataSource` actually exist (marked ✅ below); everything else is a stub
> (`NotImplementedError`). Treat this as the map, not the territory. Update it as milestones
> land and reality diverges from the plan.

This document complements [ARCHITECTURE.md](ARCHITECTURE.md), which holds the original
detailed component/sequence diagrams. This one focuses on the **runtime flow**, the
**decoupling seam**, and the **build order** as they stand after the latest decisions.

---

## 1. Runtime data flow — one swing through the pipeline (PROPOSED)

How data transforms from a swing to feedback. Each labeled arrow is a **contract** (a typed
data shape in `golf_coach.contracts`). Input sources are **swappable adapters** — mock today,
real hardware later — so the rest of the pipeline is identical either way.

```mermaid
flowchart TD
    User([Golfer swings])

    subgraph CAP["Capture — I/O edge"]
        FSRC["FileVideoSource — sample/phone clip (today)"]
        CSRC["LiveCameraSource — ELP camera (later)"]
    end

    subgraph LM["Launch Monitor — I/O edge"]
        MOCK["MockShotDataSource (today) ✅"]
        R10["R10Source — Garmin R10 BLE (later)"]
    end

    POSE["Pose — MediaPipe"]
    DET["Detection — YOLOv8 + tracker"]
    ANA["Analysis — merge to phases to checkpoints to score (pure core)"]
    FB["Feedback — rules + Claude coach + overlay render"]
    UI["Web UI"]
    DB[(SQLite)]

    User --> CAP
    User --> LM
    CAP -->|frames| POSE
    CAP -->|frames| DET
    POSE -->|FrameKeypoints| ANA
    DET -->|FrameDetections + club path| ANA
    LM -->|ShotData| ANA
    ANA -->|SwingResult| FB
    ANA -->|SwingResult| DB
    FB -->|FeedbackPayload| UI
    DB -->|history / trends| UI

    classDef built fill:#d4edda,stroke:#28a745,color:#155724;
    class MOCK built;
```

**Reading it:** the two camera adapters (`File`/`Live`) and two launch-monitor adapters
(`Mock`/`R10`) are interchangeable behind their ports. Pose and Detection run in parallel
on the same frames. Analysis is the convergence point and the only place the streams meet.
✅ = exists today.

---

## 2. The decoupling seam — who depends on what (PROPOSED)

The rule that keeps modules independent: **everything depends on `contracts/`, and modules
never import each other.** The `api` module is the only orchestrator; the React `frontend`
talks to it over HTTP. No import cycles, ever (ADR-008).

```mermaid
flowchart TD
    C["contracts/ — shared data shapes ✅"]

    CAP[capture] --> C
    POSE[pose] --> C
    DET[detection] --> C
    LMM[launch_monitor] --> C
    ANA[analysis] --> C
    FB[feedback] --> C
    STO[storage] --> C
    API[api] --> C

    API --> CAP
    API --> POSE
    API --> DET
    API --> LMM
    API --> ANA
    API --> FB
    API --> STO

    FE["frontend (React)"] -->|HTTP| API

    classDef built fill:#d4edda,stroke:#28a745,color:#155724;
    class C built;
```

**Why it matters:** because consumers depend on the contract (not the producer), you can
build `analysis` against *mock* `FrameKeypoints` before `pose` exists, and run the MCP
server against `MockShotDataSource` before buying the R10.

---

## 3. Build order & hardware gates (PROPOSED)

The milestone sequence from ROADMAP.md. 🔧 = needs hardware to *fully* complete; everything
else (most of the project) runs on sample video + simulated data. Hardware is purchased on
a **parallel track** so it arrives before the milestones that need it (ADR-007).

```mermaid
flowchart LR
    M0["M0 — Scaffold + contracts ✅"] --> M1["M1 — Capture + MediaPipe pose"]
    M1 --> M15["M1.5 — Detectability spike (go/no-go)"]
    M15 --> M2["M2 — YOLOv8 club/ball 🔧"]
    M1 --> M4["M4 — Analysis engine"]
    M2 --> M4
    M3["M3 — MCP server: mock first, then R10 🔧"] --> M4
    M4 --> M5["M5 — Feedback UI"]
    M5 --> M6["M6 — LLM coaching"]

    HW["Hardware purchase (parallel track): 2x ELP cameras + Garmin R10"]
    HW -.->|enables| M2
    HW -.->|enables real data| M3

    classDef built fill:#d4edda,stroke:#28a745,color:#155724;
    class M0 built;
```

**Critical path note:** M3 starts *now* against mock `ShotData` and only the real-data
swap needs the R10. The only milestone truly blocked on hardware is M2 (sharp club-head
frames), and even its scaffolding/labeling workflow can be prepared earlier.

---

## 4. "Swing path" comes from two sources (PROPOSED)

A recurring point of confusion: the swing/club path is represented **two ways**, and they
cross-check each other (see ROADMAP M2/M3).

```mermaid
flowchart LR
    subgraph Visual["Visual path — from camera"]
        Y["YOLOv8 club-head detections"] --> T["tracker links frames"] --> ARC["club-path arc (overlay on replay)"]
    end
    subgraph Numeric["Numeric path — from launch monitor"]
        R["Garmin R10"] --> CP["club_path degrees, at impact"]
    end
    ARC --> ANA["Analysis"]
    CP --> ANA
    ANA --> X["cross-validate: arc shape vs. measured angle"]
```

**MediaPipe's contribution:** the wrist landmarks give the *hand* path every frame, so when
the club-head detection drops out at impact (the hard zone), hand position + shaft angle can
help bridge the gap — that's the "fusion" fallback the M1.5 spike will evaluate.
