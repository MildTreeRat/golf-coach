# Roadmap: AI Golf Swing Trainer

## Last Updated: 2026-03-16

---

## Milestone 1: Capture & Skeleton (Proof of Concept)
**Goal**: Prove that a consumer camera + MediaPipe can track a golf swing skeleton accurately.

- [ ] Select and acquire camera hardware (see ADR-003)
- [ ] Set up Python project: virtual environment, dependencies, folder structure
- [ ] Write video capture script (OpenCV, save to `data/raw/`)
- [ ] Run MediaPipe Pose on recorded swing video
- [ ] Serialize keypoints to JSON (one record per frame)
- [ ] Render skeleton overlay on video and review accuracy
- [ ] Document findings: is 30fps sufficient? Are keypoints stable through full swing?

**Exit Criteria**: Skeleton overlay accurately tracks body through address → follow-through.

---

## Milestone 2: Club & Ball Detection
**Goal**: Detect and track club head and ball through the swing using a fine-tuned model.

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

- [ ] Select and acquire launch monitor hardware (see ADR-004)
- [ ] Reverse-engineer or use API to extract shot data from device
- [ ] Define `ShotData` schema (club_speed, ball_speed, launch_angle, spin, face_angle, path)
- [ ] Build MCP server with tools: `get_recent_shots`, `get_session_summary`, `get_shot_by_id`, `compare_sessions`
- [ ] Write integration tests: MCP server returns valid data for each tool
- [ ] Connect MCP server to analysis engine data merger

**Exit Criteria**: After a shot, MCP server exposes complete shot metrics; analysis engine can query them.

---

## Milestone 4: Swing Analysis Engine (Rule-Based)
**Goal**: Analyze merged pose + detection + shot data and score the swing.

- [ ] Define swing phase segmentation logic (address, backswing, transition, downswing, impact, follow-through)
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
