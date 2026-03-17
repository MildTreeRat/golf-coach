# Project Charter: AI Golf Swing Trainer

## Project Name
Home Lab AI Golf Swing Trainer

## Date Created
2026-03-16

## Last Updated
2026-03-16

---

## 1. Purpose

Build a home-lab AI-powered golf swing analysis system that captures a golfer's swing via camera, integrates launch monitor data, analyzes swing mechanics against known fundamentals, and delivers actionable coaching feedback in plain language.

## 2. Goals

1. **Learn AI/ML end-to-end** — from data collection and labeling through model training, inference, and integration.
2. **Build a functional swing analyzer** — capable of detecting pose, tracking the club, identifying swing faults, and scoring swing quality.
3. **Integrate real hardware** — camera(s) and a consumer launch monitor feeding real data into the pipeline.
4. **Practice software engineering discipline** — clean architecture, documented decisions, version control, testing.

## 3. Scope

### In Scope
- Single-camera (expandable to two) video capture of golf swings
- Body pose estimation (33 keypoints) using pretrained models
- Club head and ball detection via fine-tuned object detection
- Rule-based swing analysis engine with scoring
- Launch monitor data ingestion via MCP server
- Web-based feedback UI with visual overlays and coaching text
- LLM-powered natural language coaching (Claude API)
- Session history and progress tracking

### Out of Scope (for now)
- Multi-user support
- Mobile app
- Real-time 3D modeling / digital twin
- Commercial deployment or monetization
- Integration with third-party golf sim software (e.g., E6, GSPro)

## 4. Success Criteria

| Milestone | "Done" Looks Like |
|-----------|-------------------|
| Capture & Skeleton | Skeleton overlay tracks body accurately through full swing |
| Club Detection | Club path is tracked from address through follow-through |
| Swing Analysis | System correctly identifies 5+ common swing faults |
| Launch Monitor Integration | MCP server exposes shot data; analysis engine merges it with pose |
| Feedback UI | User swings, waits <15 seconds, sees score + coaching tips |
| LLM Coaching | Claude provides contextual advice grounded in actual swing + shot data |

## 5. Constraints

- **Budget**: Consumer-grade hardware only (webcam/GoPro, sub-$600 launch monitor)
- **Compute**: Local machine (GPU preferred but not required for inference)
- **Time**: Side project — no hard deadlines, milestone-driven
- **Data**: Must collect and label own training data for club/ball detection

## 6. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Camera framerate too low for club tracking | Medium | High | Test early (Milestone 1); upgrade camera if needed |
| Launch monitor data locked in vendor app | High | Medium | Research export options before purchasing; choose open-friendly hardware |
| Pose estimation inaccurate for golf-specific positions | Low | Medium | MediaPipe is well-tested; supplement with custom training if needed |
| Scope creep | High | Medium | Stick to charter; new features go to a "Future" section in ROADMAP |

## 7. Stakeholder

You. This is a solo learning project. All decisions are yours, documented via ADRs.
