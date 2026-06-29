# Roadmap: AI Golf Swing Trainer

## Last Updated: 2026-06-28

---

## Software / Hardware Tracks (see ADR-007)

Software development is **decoupled** from hardware acquisition — the two run in
parallel. Most early work needs no final hardware; sample/phone video and simulated
shot data are enough. Hardware is purchased in parallel so it arrives before the
milestones that require it. Each milestone below is tagged with what it needs to
*start*:

| Milestone | Hardware to START | Bootstrap with |
|-----------|-------------------|----------------|
| M1 Capture & Skeleton | None | Phone video / sample swing clips |
| M1.5 Club-Head Detectability Spike | None to start | Phone/sample clips of the impact zone |
| M2 Club & Ball Detection | Global-shutter camera (ADR-003) | Scaffolding + labeling on sample frames |
| M3 Launch Monitor / MCP | Garmin R10 (ADR-004) | MCP server + schema vs. mock `ShotData` |
| M4-PoC Fundamentals Analysis | None | M1 skeleton output (pose only) |
| M4 Analysis Engine | None | Real or simulated merged data |
| M5 Feedback UI | None | — |
| M6 LLM Coaching | None | — |

**Parallel hardware task**: purchase 2× ELP AR0234 cameras + Garmin R10 (used). Not a
blocker for M1.

---

## Milestone 1: Capture & Skeleton (Proof of Concept)
**Goal**: Prove that a consumer camera + MediaPipe can track a golf swing skeleton accurately.
**Hardware to start**: None — bootstrap with phone video or a sample swing clip (ADR-007).

- [ ] Select and acquire camera hardware (see ADR-003)
- [x] Set up Python project: virtual environment, dependencies, folder structure
- [x] Read video frames (OpenCV `FileVideoSource` from `data/raw/`) — camera *recording* awaits hardware
- [x] Run MediaPipe Pose on a swing video (`scripts/run_pose.py`, Tasks API)
- [x] Serialize keypoints to JSON (one record per frame, `data/processed/`)
- [x] Render skeleton overlay on video *(code complete; accuracy review pending a real clip)*
- [ ] Document findings: is 30fps sufficient? Are keypoints stable through full swing?

> **Status (2026-06-28):** pipeline implemented and verified end-to-end on a synthetic clip
> (capture → pose → keypoints JSON + overlay video). Design doc:
> [docs/M1_CAPTURE_FLOW.md](docs/M1_CAPTURE_FLOW.md). Remaining: run on a real swing clip
> dropped at `data/raw/`, review skeleton accuracy, and write up findings.

**Exit Criteria**: Skeleton overlay accurately tracks body through address → follow-through.

---

## Milestone 1.5: Club-Head Detectability Spike (De-Risk Before Investing)
**Goal**: Before sinking time into labeling 200–500 images and training YOLOv8, prove the club head is even *detectable* in our footage — especially through the impact zone. This is a time-boxed investigation (a spike), not production code.
**Hardware to start**: None to begin (use phone/sample video). A fast-shutter capture test is more informative with the global-shutter camera + lighting, but the core visual check can start immediately.
**Why this exists**: Club-head tracking is hard precisely where it matters most (impact). At ~110 mph the head moves ~16 in *between frames* even at 120fps, and motion blur is governed by **exposure time, not shutter type** — a global shutter removes *distortion* (warping) but NOT blur. A sharp club head at impact needs a fast shutter **+ bright light**. We need to see real frames before committing.

- [ ] Capture/collect a handful of swing clips that clearly include the **impact zone** (phone is fine to start)
- [ ] Manual inspection: at impact, is the club head a recognizable object or an unlabelable smear? How many pixels is it? How many usable frames are in the impact window?
- [ ] Lighting/shutter test: does adding bright light + forcing a fast shutter (e.g. 1/2000s) freeze the club head?
- [ ] Quick detectability probe: can an off-the-shelf detector (or just *you*, drawing boxes) reliably localize the club head frame-by-frame?
- [ ] Evaluate the fallback levers and pick a direction:
    - [ ] **Pure ML** — train YOLOv8 on the unmarked club head (most learning, gappiest at impact)
    - [ ] **Marker-assisted** — bright/reflective tape on the club head (reliable; even color-thresholding works)
    - [ ] **Fusion + interpolation** — bridge impact-zone gaps using MediaPipe wrist position + shaft angle + a Kalman tracker, anchored by the R10's `club_path`
- [ ] Write findings into a short ADR (detection strategy) once real frames are seen

**Exit Criteria (go/no-go gate)**: A documented decision on (a) whether camera-based club tracking is viable in our setup, (b) the chosen detection strategy, and (c) the lighting/shutter requirements — *before* any large labeling effort begins. A "no-go on pure-ML" is a valid, useful outcome (fall back to marker or fusion).

---

## Milestone 2: Club & Ball Detection
**Goal**: Detect and track club head and ball through the swing using a fine-tuned model.
**Hardware to start**: Global-shutter camera (ADR-003) — needed for sharp, blur-free club-head frames. Scaffolding and labeling workflow can be built earlier on sample frames.
**Why this exists**: MediaPipe tracks the *body* only (it has no concept of a club). YOLOv8 is what detects the club head + ball, and feeding its detections through a tracker (ByteTrack) produces the visual **club-path arc** — the swing path overlaid on the replay. This is also the one model we train ourselves (ADR-005).
**Gated on M1.5**: Do not start the labeling effort below until the M1.5 spike confirms the detection strategy. The tasks below assume the pure-ML or marker-assisted path; a fusion-heavy outcome reshapes them.

- [ ] Collect training images (~200-500 frames with club head and ball visible)
- [ ] Label images using Label Studio or Roboflow (see ADR-005)
- [ ] Fine-tune YOLOv8 on labeled dataset
- [ ] Evaluate model accuracy (mAP, visual inspection)
- [ ] Integrate detections with pose keypoints into unified per-frame data model
- [ ] Visualize club path overlay on video

**Exit Criteria**: Club head path is tracked continuously from backswing through follow-through.

---

## Milestone 3: Launch Monitor Integration
**Goal**: Ingest real shot data from a launch monitor and expose it via MCP server.
**Hardware to start**: Garmin R10 (ADR-004) for *real* data. The MCP server + `ShotData` schema are built first against **mock/simulated shot data**, then switched to the live R10 feed (ADR-007). The R10's `club_path` metric is the quantitative counterpart to M2's visual club-path arc.

- [ ] Build MCP server against mock/simulated `ShotData` (no hardware required)
- [ ] Select and acquire launch monitor hardware (see ADR-004)
- [ ] Reverse-engineer or use API to extract shot data from device
- [ ] Define `ShotData` schema (club_speed, ball_speed, launch_angle, spin, face_angle, path)
- [ ] Build MCP server with tools: `get_recent_shots`, `get_session_summary`, `get_shot_by_id`, `compare_sessions`
- [ ] Write integration tests: MCP server returns valid data for each tool
- [ ] Connect MCP server to analysis engine data merger

**Exit Criteria**: After a shot, MCP server exposes complete shot metrics; analysis engine can query them.

---

## Milestone 4-PoC: Fundamentals Analysis Proof-of-Concept (pose-only)
**Goal**: Prove the whole analysis spine — phases → checkpoint → score → tip — end-to-end
on **pose data alone**, no club detection and no hardware. First real iteration of the
scoring engine.
**Hardware to start**: None — runs on the M1 skeleton output.
**Decisions behind it**: scoring model in [ADR-009](docs/decisions/009-swing-scoring-model.md);
benchmark ranges in [ADR-010](docs/decisions/010-benchmark-ranges.md).

- [ ] Add `PracticeGoal` intent contract (mode + target shape + club + focus_checkpoint)
- [ ] Extend `SwingResult` with `mechanics_score`, `outcome_score`, and the judged `intent`
- [ ] Add the benchmark store: data file + `resolve_range(checkpoint, club, profile)` with
      fallback; seed **Tour Tempo (~3:1)** as the only range (ADR-010)
- [ ] Implement phase segmentation (address → … → follow-through) from keypoints
- [ ] Implement the **tempo** mechanics checkpoint (backswing:downswing ratio); optionally
      address posture (spine angle)
- [ ] Implement `scoring.py` with the **Fundamentals** policy (mechanics 100%, outcome=None)
- [ ] Implement rule-based tip(s) for the tempo checkpoint (feedback/rules.py)
- [ ] Wire it end-to-end over an M1 sample clip and eyeball the result

**Exit Criteria**: A real `SwingResult` + `FeedbackPayload` produced from a sample swing
clip with a tempo score and a plain-English tip — with the intent/dual-axis seam in place
so M2/M3 add the outcome axis without reworking contracts.

---

## Milestone 4: Swing Analysis Engine (Dual-Axis: Mechanics + Outcome)
**Goal**: Analyze merged pose + detection + shot data and score the swing across both the
**mechanics** and **outcome** axes, combined by an intent-driven scoring policy
(see [ADR-009](docs/decisions/009-swing-scoring-model.md)). Builds out from the M4-PoC.

- [ ] Define swing phase segmentation logic (address, backswing, transition, downswing, impact, follow-through)
- [ ] Add the remaining **practice modes / scoring policies**: shot-shaping, performance, drill
- [ ] Add **outcome checkpoints** (shape, start line, distance, dispersion) parameterized by
      intent; build against `MockShotDataSource` first, then live R10
- [ ] Expand the benchmark store beyond Tour Tempo (TPI kinematic sequence / X-factor;
      TrackMan + Arccos/Shot Scope per-club outcome norms) per ADR-010
- [ ] Define 5-10 swing checkpoints with acceptable ranges:
    - [ ] Address posture (spine angle, knee flex)
    - [ ] Backswing plane (club path relative to target line)
    - [ ] Hip rotation at top of backswing
    - [ ] Transition sequence (lower body leads)
    - [ ] Club face angle at impact (from detection + launch data)
    - [ ] Swing tempo (backswing:downswing ratio)
    - [ ] Follow-through balance
- [ ] Build checkpoint evaluator: input merged data → output per-checkpoint score
- [ ] Build swing scorer: aggregate checkpoint scores → 0-100 overall score
- [ ] Store results in SQLite

**Exit Criteria**: System correctly identifies at least 5 common swing faults on test swings.

---

## Milestone 5: Feedback UI
**Goal**: Present swing analysis to the user in a clear, visual web interface.

- [ ] Set up React project (or Streamlit for rapid prototype)
- [ ] Build video replay component with skeleton + club path overlays
- [ ] Build score dashboard: overall score, per-checkpoint breakdown
- [ ] Build rule-based feedback panel: plain-English tips per checkpoint
- [ ] Build session history view: list of past swings with scores and trends
- [ ] Connect frontend to FastAPI backend

**Exit Criteria**: User swings → sees annotated video, score, and actionable tips within 15 seconds.

---

## Milestone 6: LLM-Powered Coaching
**Goal**: Use Claude API to generate conversational, context-aware coaching advice.

- [ ] Design prompt template: feed swing result data + session history to Claude
- [ ] Implement Claude API call in feedback module
- [ ] Enable Claude to call MCP server tools for shot data context
- [ ] Display LLM coaching alongside rule-based feedback in UI
- [ ] Add ability to ask follow-up questions about the swing

**Exit Criteria**: Claude provides specific, grounded coaching advice referencing actual swing data and shot metrics.

---

## Future (Out of Scope for Now)
- [ ] Second camera angle (down-the-line)
- [ ] Swing comparison overlay (your swing vs. reference pro swing)
- [ ] Drill recommendations based on persistent faults
- [ ] Trained ML model for swing quality regression (replace/augment rules)
- [ ] Mobile companion app
- [ ] Export swing reports as PDF
