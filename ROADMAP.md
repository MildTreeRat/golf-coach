# Roadmap: AI Golf Swing Trainer

## Last Updated: 2026-07-16

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

- [x] Add `PracticeGoal` intent contract (mode + target shape + club + focus_checkpoint)
- [x] Extend `SwingResult` with `mechanics_score`, `outcome_score`, and the judged `intent`
- [x] Add the benchmark store: data file + `resolve_range(checkpoint, club, profile)` with
      fallback; seed **Tour Tempo (~3:1)** as the only range (ADR-010)
- [x] Implement phase segmentation (address → … → follow-through) from keypoints
- [x] Implement the **tempo** mechanics checkpoint (backswing:downswing ratio) *(address
      posture deferred — needs down-the-line/3D, ADR-011)*
- [x] Implement `scoring.py` with the **Fundamentals** policy (mechanics 100%, outcome=None)
- [x] Implement rule-based tip(s) for the tempo checkpoint (feedback/rules.py)
- [x] Wire it end-to-end over an M1 sample clip and eyeball the result

> **Status (2026-07-03):** implemented and verified end-to-end (27 tests on the base install,
> `ruff`/`mypy` clean) plus a real-clip eyeball on the face-on `aaron-swing-2` keypoints.
> Design doc: [docs/M4_ANALYSIS_POC.md](docs/M4_ANALYSIS_POC.md). Finding: phase segmentation
> now anchors on the top of the backswing (not "first motion") so a long pre-swing setup
> isn't mistaken for the backswing; segmentation *accuracy* (smoothing, more checkpoints) is
> the next thing to harden in full M4.

**Exit Criteria**: A real `SwingResult` + `FeedbackPayload` produced from a sample swing
clip with a tempo score and a plain-English tip — with the intent/dual-axis seam in place
so M2/M3 add the outcome axis without reworking contracts.

---

## Milestone 4-PoC+: Hardened Fundamentals Panel (pose-only)
**Goal**: Make the pose analysis *trustworthy without hardware* — precision via a landmark
smoothing pass, visual verification via an annotated overlay — and widen the panel with the
checkpoints face-on 2D pose measures well. Design doc:
[docs/M4_FUNDAMENTALS_PANEL.md](docs/M4_FUNDAMENTALS_PANEL.md).

- [x] Add a temporal **smoothing** pass (`analysis/smoothing.py`), applied once in the engine
- [x] Feed smoothed keypoints to phase segmentation so top/impact are stable frame-to-frame
- [x] Add **head sway** checkpoint (lateral head travel, shoulder-width normalized)
- [x] Add **finish balance** checkpoint (post-impact settle, shoulder-width normalized)
- [x] Seed provisional benchmark rows (labelled `PROVISIONAL / UNCALIBRATED`, ADR-010 addendum)
- [x] Add `scripts/analyze_swing.py` — text report **+ annotated overlay** (ADDRESS/TOP/IMPACT
      markers + score HUD) as the no-hardware accuracy check
- [x] Verify: 39 tests (base install), ruff/mypy clean, real-clip run + overlay eyeball

> **Status (2026-07-16):** done & verified. The overlay localized the remaining error — top &
> impact detect correctly, but **motion-start lands mid-takeaway**, deflating tempo to ~1:1.
> Hardening motion-start is the next segmentation task; sway/balance bands await calibration.
>
> **Update (2026-08-01):** motion-start **hardened** — the wrist-height rule missed the near-
> horizontal early takeaway, so `phases.py` now anchors it on **2D wrist speed** (last quiet frame
> before the takeaway). On `aaron-swing-2` the ADDRESS instant moved to the true onset (hands still
> at the ball) and tempo reads an honest **1.53:1** (a genuinely quick swing), up from the
> under-counted 1.05:1; TOP/IMPACT unchanged. 41 tests, ruff/mypy clean, overlay re-verified. Only
> the **Hardware Re-Validation Gate** items remain for the pose-only panel.

**Exit Criteria**: A real swing scored on three pose-only checkpoints with an annotated overlay
that lets a human verify the detected instants — met.

---

## M4-REF: GolfDB reference data (no hardware) — in progress
**Goal**: Replace eyeballed benchmark bands with ranges derived from a real population of tour
swings, and validate our instruments against ground truth — both without buying hardware. See
[ADR-012](docs/decisions/012-golfdb-reference-data.md) and the change ledger in
[docs/M4_POSE_BAKEOFF.md](docs/M4_POSE_BAKEOFF.md).

- [x] **Metric definitions v2** — `finish_balance` `max` → p90 (one bad frame no longer sets the
      score), `head_sway` `NOSE` → ear-midpoint (the nose rides on a rotating head), stricter hip
      visibility gate. Landed *before* band derivation, since a band only means something against
      the definition it was cut from
- [x] **`tempo_ratio` re-sourced** — 2.7–3.3 (a book) → **2.72–4.71**, the p10–p90 of 1,399
      hand-annotated clips from 246 tour golfers. Novosel's floor was right; his ceiling captured
      only the lower third of the real distribution
- [x] **Fixed top-of-backswing detection** — ground truth showed "top = highest hands" was a
      median of **26 frames late on 80%** of tour clips: a full finish puts the hands higher than
      the top, so it was finding the finish. Rebuilt around the earliest major descent; median
      error now **2 frames** (top) and **1** (impact) over the full 461-clip corpus, against 21
      and 35 for the rule it replaced
- [x] **Estimator bake-off** — MediaPipe lite/full/heavy vs RTMPose-m on identical clips, scored on
      event recovery against ground truth. **Kept lite**: no MediaPipe variant differs significantly
      from another (12 paired McNemar tests, none p < 0.05), heavy costs 4.4x for nothing, and
      RTMPose lost by 24.7pp. full's apparent +9.1pp edge at n=120 vanished to +1.9pp at n=461 —
      the same sample-size trap that had mis-tuned the descent threshold. See
      [ADR-002 addendum](docs/decisions/002-pose-estimation-mediapipe.md)
- [x] **Recalibrated `head_sway_norm` / `finish_balance_norm`** — 0.5 → **0.42** and 0.6 → **0.28**,
      the p90 of 458 face-on swings from 122 tour golfers, measured at GolfDB's *annotated* instants
      rather than our own segmentation. Both eyeballed bands were loose; `finish_balance` by over 2x.
      `ranges.json` now has **no `PROVISIONAL / UNCALIBRATED` rows left**
- [x] **Calibrated the address constants** — `_MOTION_QUIET_FRAC` / `_MOTION_STALL_FRAMES` were set
      from one clip's speed profile; swept against all 461, they move 0.08/3 → **0.05/4** (the
      centre of a plateau, not the grid-edge argmin), cutting median address error **13 → 9 frames**
- [x] **Improved address detection** — the fixed 4-frame stall was the one fps-dependent absolute in
      the path, and the corpus is ~47% slow-motion (median error 17 frames there against 5
      real-time). Expressed as **0.25 of the clip's own downswing duration**, with the frame-0
      fallback replaced by a bounded estimate that marks itself `detected=False`: median **9 → 7**
      frames, mean 27.2 → 22.9, PCE 14.3% → 15.8%. Six alternative signal families were tried and
      all lost to lead-wrist speed. Separately — and worth more — posture checkpoints now sample a
      short window *ending at* the boundary instead of averaging the whole ADDRESS phase from frame
      0, which corrected a false 1.21-shoulder-width sway fault on `golf_swing-aaron-1` down to
      0.36. Bands re-derived under metric definitions **v3**. See
      [docs/M4_ADDRESS_DETECTION.md](docs/M4_ADDRESS_DETECTION.md),
      [ADR-013](docs/decisions/013-clip-relative-detection.md) and M4_POSE_BAKEOFF Phase B6
- [ ] **Track `med_norm` and the slow-mo split, not the pooled frame median** — `bakeoff.py` already
      calls `med_norm` "the headline number" and now emits `med_err_frames_by_speed`, but the
      figures quoted around this repo are still raw frames. A pooled frame median over a corpus that
      is 47% broadcast slow-motion measures the corpus mix as much as the rule
- [ ] Address is *still* the weakest instant (median 7 frames, 40% over 10) and the remaining
      headroom looks like a learned-model problem, not a heuristic one: a rule using **no pose
      signal at all** scores 11 frames, and GolfDB's own SwingNet reaches 31.7% PCE against our
      15.8%. Would need its own ADR — ADR-008 keeps the analysis core stdlib-only

**Exit Criteria**: every band in `ranges.json` traceable to an inspectable distribution, and phase
instants validated against hand-annotated ground truth — **met**. Address remains the weakest of the
three instants, but it is now measured, improved against a different rule, and no longer able to
fail silently.

---

## Hardware Re-Validation Gate (revisit when cameras / launch monitor arrive)
**Why this exists**: several M4-PoC/PoC+ choices are the best we can do from a single face-on
camera with no ground truth. They are deliberately provisional and must be re-checked — not
silently trusted — once hardware (down-the-line camera per ADR-011, Garmin R10 per ADR-004)
lands. Everything flagged here is greppable in-code via `HARDWARE-REVALIDATE:` comments and via
the `PROVISIONAL / UNCALIBRATED` provenance strings in `ranges.json`.

- [x] **Recalibrate provisional bands** — **done without hardware** (M4-REF Phase B): both are now
      p90 of 458 face-on tour swings (122 golfers), and the `PROVISIONAL / UNCALIBRATED` provenance
      strings are gone. *Still worth re-checking against our own captured swings — a tour population
      says what good looks like, not what this camera measures; and the same estimator processing
      both sides is what makes the comparison fair, so common-mode bias is cancelled, not removed.*
- [x] **Validate phase instants** — **done without hardware** (M4-REF): validated against GolfDB's
      461 hand-annotated face-on clips, which found and fixed a systematic top-detection defect.
      Median error now 2 frames (top), 1 (impact), **7 (address)**. Real impact timing from M2/M3 is
      still the stronger check for *impact specifically*, but the pose-only instants are no longer
      unvalidated guesses tuned on one clip
- [ ] **Revisit deferred checkpoints** — spine tilt, hip rotation, X-factor, swing plane become
      measurable with the down-the-line view / 3D fusion (ADR-011); add them to the panel
- [ ] **Re-tune smoothing** — window / weighting were set by eye on ~60fps phone clips; global
      shutter + higher fps may want different values

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
- [ ] Second camera angle (down-the-line) — carries spine/plane pose **and** club tracking
      (ADR-003 addendum 2026-07-02b)
- [ ] **Camera synchronization + multi-view 3D fusion** (ADR-011) — phased: software/event
      sync first, hardware trigger later; unlocks true 3D spine angle, hip rotation, X-factor
- [ ] Swing comparison overlay (your swing vs. reference pro swing) — **most of the groundwork
      exists**: M4-REF's Tier 1 cache holds 461 face-on tour swings as keypoints in the *exact*
      `FrameKeypoints` serialization `analyze_swing()` already loads, so a reference swing replays
      through the existing pipeline unchanged. What is missing is selection (which pro, matched on
      club/sex/build?) and spatial normalization, not extraction. Note the corpus is gitignored for
      licensing (ADR-012), so shipping this needs a redistribution story — aggregate percentiles are
      committable, per-clip keypoints of named tour players are not
- [ ] Drill recommendations based on persistent faults
- [ ] Trained ML model for swing quality regression (replace/augment rules)
- [ ] Mobile companion app
- [ ] Export swing reports as PDF
