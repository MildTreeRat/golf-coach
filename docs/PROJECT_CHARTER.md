# Project Charter: AI Golf Swing Trainer

> **Tier: FOUNDING.** The intent the project started from. Scope and risks still hold; specific
> numbers and milestone shapes have been sharpened by later ADRs — read the intent, not the
> specifics.

## Project Name
Home Lab AI Golf Swing Trainer

## Date Created
2026-03-16

## Last Updated
2026-08-05 — reviewed, **purpose / goals / scope unchanged since 2026-03-16**. Only the
"How the charter has been refined" section below is new; everything else is the original text.

---

## 0. How the charter has been refined

The charter has held up well — the purpose, the four goals, and the in/out-of-scope lines all
still describe the project. Three things have been *sharpened* by later decisions, and the
success criteria in §4 read as pre-decision as a result:

- **Scoring is dual-axis, not a single score.** [ADR-009](decisions/009-swing-scoring-model.md)
  separates `mechanics_score` from `outcome_score`, combined by a policy chosen from the
  golfer's practice intent. §4's "scores swing quality" is therefore two numbers and a policy,
  not one number.
- **"Known fundamentals" now means a measured population, not a book.**
  [ADR-012](decisions/012-golfdb-reference-data.md) replaced eyeballed and book-sourced bands
  with p10–p90 ranges derived from 1,399 hand-annotated tour swings. §1's "against known
  fundamentals" is now traceable to an inspectable distribution.
- **Two capture tiers, not one camera count.** §3 says "single-camera (expandable to two)".
  That is still right, but the two are now *different tiers*: a fixed ELP rig that could
  eventually support 3D fusion ([ADR-011](decisions/011-camera-synchronization.md)), and a
  zero-setup hand-held two-phone tier that trades 3D away for portability (M7). The second was
  not foreseen here.

Two out-of-scope lines worth re-reading in light of what happened:

- **"Real-time 3D modeling / digital twin"** stays out of scope, and ADR-011's addendum explains
  why it is genuinely unreachable for hand-held capture rather than merely deferred.
- **"Integration with third-party golf sim software (e.g. E6, GSPro)"** stays out of scope, and
  still is — [ADR-014](decisions/014-screen-capture-shot-ingestion.md) reads the HD Golf
  simulator's `SHOT DATA` screen *optically*, precisely **because** there is no integration and
  no data export. Photographing a screen is the opposite of integrating with it.

The risk register in §6 is unchanged and still current — the club-head detectability risk is
live and unretired, since the M1.5 spike that would settle it has not been run.

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
| Club head not reliably detectable (motion blur + small/fast object at impact, not just framerate) | High | High | De-risk in the M1.5 spike *before* labeling: lighting + fast-shutter test, then choose pure-ML / marker-assisted / fusion+interpolation. Global shutter fixes distortion, not blur (see ADR-003 addendum). |
| Launch monitor data locked in vendor app | High | Medium | Research export options before purchasing; choose open-friendly hardware |
| Pose estimation inaccurate for golf-specific positions | Low | Medium | MediaPipe is well-tested; supplement with custom training if needed |
| Scope creep | High | Medium | Stick to charter; new features go to a "Future" section in ROADMAP |

## 7. Stakeholder

You. This is a solo learning project. All decisions are yours, documented via ADRs.
