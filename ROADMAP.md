# Roadmap: AI Golf Swing Trainer

## Last Updated: 2026-08-09

Grouped by **state**, not by number, because the numbers no longer run in order: the pose-only
slices (M4-PoC, M4-PoC+, M4-REF, M5-FB) delivered the mechanics half of M4 and the ranking half
of M5 long before either full milestone became reachable. Detail sections keep their original
wording; only the grouping and the M4 checklist have been corrected.

## Status at a glance

| Milestone | State | Needs to start | Detail |
|---|---|---|---|
| **M1** Capture & skeleton | ✅ Done | — | [§M1](#milestone-1-capture--skeleton--done-pose-pipeline) |
| **M4-PoC** Analysis spine | ✅ Done | — | [§M4-PoC](#m4-poc-fundamentals-analysis-proof-of-concept-pose-only-slice-of-m4--done) |
| **M4-PoC+** Hardened panel | ✅ Done | — | [§M4-PoC+](#m4-poc-hardened-fundamentals-panel-pose-only-slice-of-m4--done) |
| **M4-REF** GolfDB validation | ✅ Done | — | [§M4-REF](#m4-ref-golfdb-reference-data-pose-only-slice-of-m4--done) |
| **M5-FB** Ranked coaching | ✅ Done | — | [§M5-FB](#m5-fb-prioritised-coaching-feedback-pose-only-slice-of-m5--done) |
| **M3** Launch monitor / MCP | 🟡 In progress | Nothing — screen OCR done; MCP server left | [§M3](#milestone-3-launch-monitor-integration--in-progress) |
| **M1.5** Detectability spike | ⬜ Next | Nothing — phone clips of the impact zone | [§M1.5](#milestone-15-club-head-detectability-spike-de-risk-before-investing) |
| **M7** Two-phone sim capture | 🟡 In progress, 6/7 phases (3 trimmed) | Nothing — two iPhones + a sim bay | [§M7](#milestone-7-two-phone-sim-capture-no-hardware-purchase) |
| **M4** full (outcome axis) | ⬜ Blocked | The M2 + M3 streams | [§M4 full](#milestone-4-full-swing-analysis-engine--the-outcome-axis) |
| **M5** Feedback UI | ⬜ Not started | M7 Phase 5 gives the host | [§M5](#milestone-5-feedback-ui) |
| **M6** LLM coaching | ⬜ Not started | M5 | [§M6](#milestone-6-llm-powered-coaching) |
| **M2** Club & ball detection | 🔒 Gated | M1.5 go/no-go **+** global-shutter camera | [§M2](#milestone-2-club--ball-detection) |
| Hardware re-validation | 🔒 Gated | Cameras / launch monitor arriving | [§Gate](#hardware-re-validation-gate-revisit-when-cameras--launch-monitor-arrive) |

**The one purchase that actually blocks something:** a global-shutter camera for M2. Everything
else above either needs nothing, or needs work rather than money.

**Oldest unstarted item:** M1.5. It has stayed deferrable because the pose-only spine kept
producing value without it — but M2 and every club-derived metric sit behind it.

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
| M3 Launch Monitor / MCP | None — HD Golf screen captures (ADR-014) | Photos of the SHOT DATA screen; mock `ShotData` |
| M4-PoC Fundamentals Analysis | None | M1 skeleton output (pose only) |
| M4 Analysis Engine | None | Real or simulated merged data |
| M5 Feedback UI | None | — |
| M6 LLM Coaching | None | — |

**Parallel hardware task**: purchase 2× ELP AR0234 cameras + Garmin R10 (used). Not a
blocker for M1.


---

# Done

The pose-only track, complete. All four of these ran on phone video and a public reference corpus — no hardware was used or needed.

---

## Milestone 1: Capture & Skeleton — done (pose pipeline)
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
> [docs/archive/M1_CAPTURE_FLOW.md](docs/archive/M1_CAPTURE_FLOW.md). Remaining: run on a real swing clip
> dropped at `data/raw/`, review skeleton accuracy, and write up findings.

**Exit Criteria**: Skeleton overlay accurately tracks body through address → follow-through.


---

## M4-PoC: Fundamentals Analysis Proof-of-Concept (pose-only slice of M4) — done
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
> Design doc: [docs/archive/M4_ANALYSIS_POC.md](docs/archive/M4_ANALYSIS_POC.md). Finding: phase segmentation
> now anchors on the top of the backswing (not "first motion") so a long pre-swing setup
> isn't mistaken for the backswing; segmentation *accuracy* (smoothing, more checkpoints) is
> the next thing to harden in full M4.

**Exit Criteria**: A real `SwingResult` + `FeedbackPayload` produced from a sample swing
clip with a tempo score and a plain-English tip — with the intent/dual-axis seam in place
so M2/M3 add the outcome axis without reworking contracts.


---

## M4-PoC+: Hardened Fundamentals Panel (pose-only slice of M4) — done
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

## M4-REF: GolfDB reference data (pose-only slice of M4) — done
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

## M5-FB: Prioritised coaching feedback (pose-only slice of M5) — done
**Goal**: Stop reporting three equal-weight pass/fail readouts and start saying *what to work on
first*, grounded in how far off the tour population a swing actually sits. Design doc:
[docs/M5_COACHING_FEEDBACK.md](docs/M5_COACHING_FEEDBACK.md).

- [x] **Wired the reference distributions into production.** `golfdb_v1.json` + `percentile_of()`
      were built and tested in M4-REF and then imported by nothing but their own test. Every
      `CheckpointScore` now carries `percentile` / `population_n` / `one_sided`, and every tip says
      where the swing sits — "a looser finish than at least 90% of 458 tour swings"
- [x] **Ranked the tips, added a headline.** Failures first by `score`, then passes by percentile.
      Two signals because neither works alone: the bands *are* the reference p10/p90 and
      `percentile_of` clamps there, so every failure reports 90; and `_score_within_range` returns
      exactly 1.0 for every pass. The motivating case is `golf_swing-aaron-1`, which scores
      **100/100** while its head sway sits higher than 83% of tour swings
- [x] **Percentiles kept off the scoring path** (ADR-010 addendum) — informational only, drawn from
      the same `(all, all, all)` stratum the bands were cut from, with a test that blinds the
      evaluators to the distributions and asserts `score`/`passed` do not move
- [x] **Unmeasurable checkpoints are named rather than dropped silently** — tempo goes missing on
      ~14% of clips (ADR-013) and `overall_score` is a mean over survivors, so `SwingResult.unscored`
      now carries the names. The score is *not* penalised; the fix is disclosure, not arithmetic
- [x] **Tried to widen the panel 3 → 5, and the gate said no.** GolfDB's mid-backswing/mid-downswing
      are *lead arm parallel to the ground*, a real body pose (unlike `toe_up`, a club event gated on
      M2). New `scripts/golfdb/tune_arm_parallel.py` scored three candidate rules over the 461-clip
      face-on corpus **before** either checkpoint was written: `prior_frac`, which reads no pose
      signal at all, beats every pose rule on every column — frac_err 0.043 vs 0.059 (mid_backswing)
      and 0.038 vs 0.100 (mid_downswing). A checkpoint built on our detection would be worse than a
      constant. Kept runnable, like the six rejected address signals
- [ ] **Reasons, not just names, on `unscored`** — needs the evaluators to return a reason instead of
      `None`, which touches every return site
- [ ] **Per-club percentiles** — the corpus has the strata, but the band has to move in step
      (ADR-010 addendum), so it is one change on two sides, not a percentile-only edit

**Exit Criteria**: a swing report that leads with the one thing to work on and quantifies it against
the tour population — **met**. The panel stayed at three checkpoints, which is a measured result
rather than an omission.


---

# In progress

Shot ingestion landed; the MCP server is the remainder.

---

## Milestone 3: Launch Monitor Integration — in progress
**Goal**: Ingest real shot data from a launch monitor and expose it via MCP server.
**Hardware to start**: None any more. Shot data now comes from photos of the **HD Golf** simulator's `SHOT DATA` screen, parsed by local OCR ([ADR-014](docs/decisions/014-screen-capture-shot-ingestion.md)) — hardware already owned. The Garmin R10 (ADR-004) stays the right answer for real-time streaming and drops into the same port when bought. `club_path` is the quantitative counterpart to M2's visual club-path arc.

- [x] Define `ShotData` schema (club_speed, ball_speed, launch_angle, spin, face_angle, path)
- [x] Extract shot data from the device — screen-capture OCR, since HD Golf has no export
- [x] `CompositeShotDataSource` so screen / mock / R10 feeds mix behind one port
- [x] Parse confidence + physics cross-checks, so a misread digit is flagged not trusted
- [x] `scripts/import_shot_screens.py` — photos in, parsed shots out, content-addressed cache
- [ ] Tune preprocessing against a full range session's photos (not just the 2 reference ones)
- [ ] Build MCP server against a `ShotDataSource` (mock or screen — no hardware required)
- [ ] Build MCP server with tools: `get_recent_shots`, `get_session_summary`, `get_shot_by_id`, `compare_sessions`
- [ ] Write integration tests: MCP server returns valid data for each tool
- [ ] Connect MCP server to analysis engine data merger
- [ ] *(optional, later)* Acquire the Garmin R10 and add its BLE adapter (ADR-004)

> **Status (2026-08-04):** shot ingestion works end-to-end from photos — screen rectification,
> orientation recovery, OCR, geometric tile parsing, sign conventions, physics validation, and a
> parse cache. `ScreenShotDataSource` serves the results on the base install with no OCR stack.
> Remaining: tune preprocessing on a real session's worth of photos, then the MCP server itself.

**Exit Criteria**: After a shot, MCP server exposes complete shot metrics; analysis engine can query them.


---

# Next — nothing blocking

Startable today. M1.5 gates M2, so it is the highest-leverage of the three.

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

## Milestone 7: Two-Phone Sim Capture (no hardware purchase)
**Goal**: Record a swing at an indoor sim on two hand-held iPhones (face-on + down-the-line),
photograph the HD Golf `SHOT DATA` screen, get all three to the desktop, and analyze them
together. Full plan, per-phase detail and planning prompts:
[docs/M7_TWO_PHONE_CAPTURE.md](docs/M7_TWO_PHONE_CAPTURE.md).

This is a **second capture tier** alongside ADR-011's fixed-rig path — it trades 3D away for
zero-setup portability. Triangulation is unreachable with hand-held phones (no calibration is
possible), so down-the-line is **capture + align only**: scoring stays on the three validated
face-on checkpoints. Each phase below is one commit, planned in a fresh session.

> **Status (2026-08-09):** Phases 1–6 built (3 **trimmed**). The whole use case now runs from a
> phone: upload face-on, down-the-line and a photo of the shot screen, and the third file
> triggers a background worker that scores the swing, ranks the tips, aligns the two views and
> attaches the HD Golf numbers — a results page has them by the time you walk back from the bay.
> Verified end to end at 31.8 s for a 30 fps pair; uploads stay instant throughout because the
> pipeline runs in a thread. `SwingManifest.status()` still uses the neutral
> `collecting`/`complete`: analysis state lives in its own `analysis.state.json` sidecar rather
> than in the ingestion manifest. Only the Phase 0 field spike is unstarted.

- [ ] **Phase 0** — Field spike: does `segment_phases()` work on down-the-line footage? Does
      OpenCV decode iPhone HEVC? What does `CAP_PROP_FPS` report for slo-mo? Gate for 1 and 2
- [x] **Phase 1** — Capture layer survives phone footage *(2026-08-07)*: streaming pose and
      overlay passes (peak RSS 971 → 233 MB on the sample clip, and now independent of clip
      length), `camera_id` on `Frame` + `FrameKeypoints` (ADR-011's seam, finally built), clip
      metadata incl. **fps** persisted in the keypoints JSON. The pre-inference downscale was
      **deliberately deferred** — see the phase notes in docs/M7_TWO_PHONE_CAPTURE.md
- [x] **Phase 2** — Video sync / alignment engine *(2026-08-07)*: event-anchored piecewise-linear
      time warp over the phase instants each clip already produces (ADR-011 Option C, standalone),
      in `contracts/alignment.py` + `analysis/alignment.py` + `scripts/align_swings.py`. Immune to
      mismatched fps, clip lengths and iPhone slo-mo by construction. `AlignmentQuality` states how
      much of the swing was actually anchored rather than implying frame accuracy everywhere.
      Added beyond the plan: **multi-swing clip selection** (`phases.candidate_downswings()` +
      `--window`), because real phone clips contain practice swings and the earliest-descent rule
      would otherwise align a practice swing in one view to the real one in the other. Wrote
      **ADR-015**, which also settles the `FrameBundle` question ADR-011 left open
- [x] **Phase 3** — Session & swing bundle store, **trimmed**: role-based swing assignment
      (`storage/bundle_store.py`), content-addressed dedupe, and a `swing_id` repair path for
      the case a naive arrival rule misattributes (documented, not engineered around — see
      `tests/storage/test_bundle_store.py::test_newest_wins_when_two_swings_are_missing_the_same_role`).
      First real code in `storage/`. Status is derived, not a persisted `pending → ready →
      analyzed` machine — nothing downstream consumes `ready` yet
- [x] **Phase 4** — Bundle analysis + launch-monitor join *(2026-08-08)*: `analyze_swing_bundle()`
      in `analysis/engine.py` scores the face-on view through `analyze_swing()` **unchanged** and
      uses down-the-line for alignment anchors only; `SwingResult.shot` is populated at last, from
      a cache-first join on the photo's sha256 (an already-imported shot attaches with no `ocr`
      extra at all). New `scripts/analyze_bundle.py` is the one command; new `SwingBundleResult`
      serializes to `analysis.json` beside an `aligned.mp4`. Added beyond the plan:
      **`phases.select_swing()`**, because the window is not framing — it decides which frames get
      *scored*, and unaided segmentation picks a setup move on all four real bay clips.
      Shot numbers are attached and displayed; *scoring* them stays M4 per ADR-009
- [ ] **Tempo is untrustworthy on real footage and needs its own look.** `phases._motion_start`
      walks back from the top for a quiet stretch, and a golfer who pauses at the top hands it one
      immediately — so the boundary collapses onto the top and the backswing measures shorter than
      the downswing (0.43:1 on `aaron-1`, an impossibility). `analyze_swing` scores it anyway and
      the ranked tips lead with "work on tempo first", which is wrong. Phase 4 makes the
      contradiction impossible to miss (a note, printed above the tips) and deliberately changes
      no scoring. A fix belongs either in `_motion_start` — reject a quiet stretch sitting within
      a downswing's length of the top — or in `evaluate_tempo`, applying the plausibility floor
      `analysis/alignment.py` already has as `MIN_PLAUSIBLE_TEMPO`. Measure against GolfDB first:
      that corpus is where the tempo band came from
- [x] **Phase 5** — Local server + phone upload page *(ingestion 2026-08-06; worker and results
      page 2026-08-09, completing the phase)*: `POST /api/uploads` streams to disk, a static page
      (`api/static/`) with a role picker sticky in `localStorage` and a live status panel.
      Loopback only — `run_server.py` defaults to `127.0.0.1` and refuses a non-loopback `--host`
      with no token set; Phase 6 puts Tailscale in front rather than widening it (ADR-016).
      **The loop is now closed**: the orchestration moved out of `scripts/analyze_bundle.py` into
      `api/pipeline.py` (which imports no fastapi, so the CLI still runs without the `api` extra),
      `api/worker.py` runs it on an asyncio queue at concurrency 1 via `asyncio.to_thread`, and
      `api/static/results.html` renders score, checkpoints, tips, shot numbers and the aligned
      video. State lives in an `analysis.state.json` sidecar keyed on the role→sha256 map, so a
      re-uploaded clip invalidates its own result. **Analysis triggers only on a complete
      bundle** — a partial one waits for an explicit "Analyze anyway", because no timeout guesses
      right about whether the second phone is still walking back from the bay
- [x] **Phase 6** — Tailscale exposure, **and it did not land as planned**: rather than binding the
      tailnet IP, the bind stays on `127.0.0.1` and `tailscale serve` proxies to it over real TLS.
      Because a helper's phone can't join the tailnet, `tailscale funnel` covers guest devices —
      which makes tailnet membership insufficient as the only access control, so `/api/` is gated
      on `GOLF_UPLOAD_TOKEN` (header or one-time `?t=` link). Default port 8080 → 3000 (Windows
      reserves 8069–8168). Wrote **ADR-016**

**Exit Criteria**: one bay session where every swing assembles from the right two clips, the
aligned side-by-side video's IMPACT banners land together in both panels, and shot data attaches
to the correct swing. That session also collects Phase 0's footage — preflight, phone settings,
timings and failure modes are in
[docs/BAY_SESSION_RUNBOOK.md](docs/BAY_SESSION_RUNBOOK.md).


---

# Blocked on earlier milestones

These need a data stream or a host that does not exist yet.

---

## Milestone 4 (full): Swing Analysis Engine — the outcome axis
**Goal**: Analyze merged pose + detection + shot data and score the swing across both the
**mechanics** and **outcome** axes, combined by an intent-driven scoring policy
(see [ADR-009](docs/decisions/009-swing-scoring-model.md)).
**Blocked on**: the M2 (club) and M3 (shot) streams. The mechanics axis is already live via the
M4-PoC → M4-REF slices above; what remains here is genuinely everything that needs a second
data stream.

**Already delivered by the pose-only slices** — these were the original M4 checklist and are
done, listed here so the remaining work below is not misread as a fresh start:

- [x] Swing phase segmentation → `analysis/phases.py` (M4-PoC), validated against 461
      hand-annotated clips in M4-REF: median error 2 frames (top), 1 (impact), 7 (address)
- [x] Swing tempo checkpoint → `evaluate_tempo`, band re-sourced from 1,399 tour swings
- [x] Follow-through balance → `evaluate_finish_balance` (M4-PoC+), band = p90 of 458 tour swings
- [x] Head sway → `evaluate_head_sway` (M4-PoC+) — not on the original list, added because
      face-on 2D pose measures it well
- [x] Checkpoint evaluator → `analysis/checkpoints/mechanics.py`
- [x] Swing scorer → `analysis/scoring.py`, `FundamentalsPolicy`, 0–100 overall
- [x] Benchmark store with provenance → `benchmarks/ranges.json` + `golfdb_v1.json`.
      *Note the original item read "expand beyond Tour Tempo" — Tour Tempo was **replaced**, not
      expanded (ADR-012), so that framing no longer applies*

**Remaining — all of it needs a stream that does not exist yet:**

- [ ] Add `merge.py` — align keypoints + detections + shot data on one timeline. Deliberately
      not built while pose-only is a single stream (YAGNI)
- [ ] Add the remaining **practice modes / scoring policies**: shot-shaping, performance, drill
      (`policy_for` currently raises `NotImplementedError` for all three)
- [ ] Add **outcome checkpoints** (shape, start line, distance, dispersion) parameterized by
      intent; build against `MockShotDataSource` first, then live data
- [ ] Populate `SwingResult.shot` — the field exists and has never been set. *M7 Phase 4 does
      the attach-and-display; **scoring** those numbers is this milestone (ADR-009)*
- [ ] Expand the benchmark store with **outcome** norms (TrackMan + Arccos/Shot Scope per-club)
      and the mechanics ranges that need 3D (TPI kinematic sequence / X-factor), per ADR-010
- [ ] Add the checkpoints that need a second view or club detection:
    - [ ] Address posture (spine angle, knee flex) — needs down-the-line (ADR-011)
    - [ ] Backswing plane (club path relative to target line) — needs M2
    - [ ] Hip rotation at top of backswing — needs 3D (ADR-011)
    - [ ] Transition sequence (lower body leads) — needs 3D (ADR-011)
    - [ ] Club face angle at impact — needs M2 detection + launch data
- [ ] Store results in SQLite (M7 Phase 3 builds the store; this persists `SwingResult` into it)

**Exit Criteria**: System correctly identifies at least 5 common swing faults on test swings,
scoring both axes.

---

## Milestone 5: Feedback UI
**Goal**: Present swing analysis to the user in a clear, visual web interface.

- [ ] Set up React project (or Streamlit for rapid prototype)
- [ ] Build video replay component with skeleton + club path overlays
- [ ] Build score dashboard: overall score, per-checkpoint breakdown *(the payload is ready:
      `CheckpointScore` carries the band **and** the tour percentile, so a bar can show both)*
- [ ] Build rule-based feedback panel: plain-English tips per checkpoint *(the ranking, severity and
      headline landed in M5-FB; what's left is rendering `FeedbackPayload`)*
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

# Gated on hardware

Needs a purchase, or needs hardware in hand to re-check a provisional choice.

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

## Future (Out of Scope for Now)
- [ ] **Multi-view 3D fusion on the fixed rig** (ADR-011 Phases 2–3) — triangulated spine angle,
      hip rotation, X-factor, kinematic sequence. Needs the ELP cameras, intrinsics/extrinsics
      calibration, and eventually a hardware trigger. **Not** reachable from M7's hand-held
      phones, which cannot be calibrated — M7 delivers the 2D-alignment tier instead
- [ ] Down-the-line **metrics** (spine tilt, swing plane) — M7 captures and aligns the DTL view
      but scores nothing from it, because the GolfDB corpus the benchmark bands were derived from
      is face-on, so DTL metrics have no reference population yet (ADR-010, ADR-012)
- [ ] Club tracking from the down-the-line view (YOLOv8, M2) — ADR-003 addendum 2026-07-02b
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
