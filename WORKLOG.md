# Work Log

Write a short entry every time you sit down to work. Reverse chronological (newest first).
This is your "pick up where I left off" document.

---

## 2026-07-02 (cont. 2) — M4-PoC implementation plan written up (no code yet)

**Duration**: ~0.5 hour
**What I did**: Reviewed the M4-PoC scope (ROADMAP + ADR-009/010), re-verified the current
contracts/stubs against the plan, and documented the agreed implementation plan into the repo
as **[docs/M4_POC_PLAN.md](docs/M4_POC_PLAN.md)** so it survives outside the local scratch
plan. Confirmed nothing in the plan is stale: `analysis/engine.py` and `feedback/rules.py` are
still `NotImplementedError` stubs; `contracts/{intent}.py` and
`analysis/{benchmarks,phases,checkpoints,scoring}` don't exist yet (all new); hatchling packages
all of `src/golf_coach`, so a `benchmarks/ranges.json` ships with no pyproject change.
**Plan in one line**: build the pose-only Fundamentals spine
(`FrameKeypoints + PracticeGoal → analyze_swing → SwingResult → build_feedback → FeedbackPayload`)
with a single **tempo** checkpoint, the dual-axis/intent seam in place, benchmark ranges as
JSON-with-provenance (Tour Tempo ~3:1 seeded), all on the stdlib base install. Full breakdown of
the 10 change-sets (new modules, contract extensions, tests, verification) is in the plan doc.
**Key decisions (all captured in the plan doc)**:
- **Scope = M4-PoC only, one tempo checkpoint.** No `merge.py`, no outcome checkpoints, no extra
  scoring policies, no SQLite — those are full-M4 and left as named seams.
- **Analysis core stays pure-Python/stdlib** (no numpy/MediaPipe) so the spine + tests run on
  `pip install -e .`; benchmark store ships as **JSON not YAML** to keep base deps tiny.
**Where I left off**: Plan is documented and approved; **no analysis code written yet**. Next
session: implement change-sets 1→9 in `docs/M4_POC_PLAN.md` (contracts → benchmarks → phases →
checkpoint → scoring → engine → feedback → tests), then write the `docs/M4_ANALYSIS_POC.md`
milestone flow doc and check off the ROADMAP boxes.
**Blockers**: None — pose-only, no hardware.
**Notes**: Real-clip eyeball can reuse the existing face-on keypoints JSON in `data/processed/`
(from `aaron-swing-2.mov`) for the exit-criterion sanity check.

---

## 2026-07-02 — M1 angle re-shoot: face-on confirmed as canonical pose placement

**Duration**: ~0.5 hour
**What I did**: Re-shot the swing from a **face-on** angle (`data/raw/aaron-swing-2.mov`,
480×854, 58.9 fps, 656 frames) to test the self-occlusion hypothesis from the 06-28 review,
ran it through `run_pose.py`, and computed the same metrics on both clips for a
side-by-side (detection, per-group visibility, knee-by-decile, jitter).
**Findings** (full write-up:
[M1_CAPTURE_FLOW.md → angle comparison](docs/M1_CAPTURE_FLOW.md#m1-findings-angle-comparison-2026-07-02)):
- Face-on wins on **every** body metric. **Knees 0.71 → 0.88 (+24%)**, lower body
  0.70 → 0.83, overall visibility 0.78 → 0.89. Leg jitter also dropped.
- The big one: knee confidence now stays 0.85–0.95 *through the bent swing posture* where it
  used to collapse to ~0.60. Confirms the occlusion diagnosis — at ~5 o'clock the legs
  stacked front-to-back; face-on separates them.
- Honest caveat: all-landmark jitter rose slightly, but that's genuine (larger arm arc in
  face-on), not noise. Temporal smoothing still worth doing.
**Key decision**:
- **Canonical pose-camera placement = face-on / 3 o'clock** (ball flies to 12; RH golfer,
  mirror to 9 for LH). Recorded in **ADR-003 addendum (2026-07-02)**, which also reconciles
  this with the ADR's original "down-the-line first" note: down-the-line is for the *club*
  stream (M2/YOLOv8), face-on is for the *pose* stream.
**Where I left off**: M1 pose setup is now solid. The "re-record & re-review lower body" open
refinement is **done**; only the optional temporal-smoothing pass remains. Ready to start
**M4-PoC** (tempo) on the new keypoints JSON whenever we choose.
**Blockers**: None.
**Notes**: old raw clip (`golf_swing-aaron-1.mov`) no longer in `data/raw`, but its
keypoints JSON remains in `data/processed` and was used for the comparison.

---

## 2026-07-02 (cont.) — Spine-angle investigation → camera topology & sync plan

**Duration**: ~0.5 hour
**What I did**: Investigated a hunch that shot 2's spine looked "very vertical." Estimated
spine tilt (shoulder-mid → hip-mid vs vertical) on both clips, in 2D and depth-aware 3D.
Used the result to settle the camera topology and to plan synchronization.
**Findings**:
- Shot 1 (~5 o'clock): ~37° spine tilt at address (believable). Shot 2 (face-on): ~2° in the
  image — looks dead vertical, but that's a **projection artifact**: forward tilt lives in the
  camera's depth axis and foreshortens to ≈0 face-on. `z`-based 3D put shot 2 at ~66–71° (if
  anything more bent), but MediaPipe `z` is unreliable. **Can't** conclude a more upright
  stance — the "vertical" look is the camera angle. Rule: forward spine tilt = down-the-line
  measurement, not face-on.
**Key decisions (documented)**:
- **Two cameras, not three.** The spine/hip stream and the club stream share the
  down-the-line viewpoint; one global-shutter camera there runs both MediaPipe + YOLOv8. →
  **ADR-003 addendum (2026-07-02b)** with the stream→camera assignment table + spine caveat.
- **ADR-011 (Proposed): Camera Synchronization & Multi-View 3D Fusion** — phased plan
  (Phase 1 single-cam now, build `camera_id`+timestamp+`FrameBundle` seam; Phase 2 two-cam
  software/event sync + calibration; Phase 3 hardware trigger for frame-accurate dynamic 3D).
  Unlocks true spine angle / hip rotation / X-factor / kinematic sequence for M4 mechanics.
- Also noted the spine caveat in M1_CAPTURE_FLOW.md and added both items to ROADMAP Future.
**Where I left off**: Camera plan is now on record end-to-end (angle, count, stream
assignment, sync roadmap). No code change — ADR-011's capture-seam work (`camera_id` on
`Frame`, `FrameBundle`) is a small future task, not needed for M1/M4-PoC.
**Blockers**: None.
**Notes**: ADR-011 is Proposed/forward-looking — no multi-cam hardware yet; seam designed now.

---

## 2026-06-28 (cont. 2) — M1 accuracy review & documentation

**Duration**: ~1 hour
**What I did**: Ran the M1 pipeline on the first real swing clip
(`data/raw/golf_swing-aaron-1.mov`, 480×854, 58.9 fps, 674 frames) and did the accuracy
review. Then documented the model + this step thoroughly so it's easy to pick back up.
**Findings** (full write-up in
[docs/M1_CAPTURE_FLOW.md → M1 findings](docs/M1_CAPTURE_FLOW.md#m1-findings-accuracy-review-2026-06-28)):
- 100% detection, avg visibility 0.78; **upper body tracks well**; **59 fps is plenty**.
- **Knees/lower body weak during the bent swing posture**, fine once standing; plus some
  **jitter**.
- Ran a **lite-vs-heavy model experiment on the same clip** — heavy did NOT fix the knees
  (and is 3–5× slower). Inspecting matched frames, the cause is the **picture** (dim light,
  dark shorts on dark floor = no leg contrast, cluttered background), not model size.
**Key decisions**:
- **Keep the lite model** (`pose_landmarker_lite.task`). Documented the whole model story —
  Tasks API vs legacy Solutions API, the lite/full/heavy variants, the download URL/location
  (`data/models/`, via `_ensure_model`) — in **ADR-002 addendum** and the M1 flow doc's new
  **"Pose model (reference)"** section.
- The lower-body fix is a **better recording**, not code: more/even lower-body lighting,
  declutter, contrast the legs, tripod framing. Captured in the findings.
**Where I left off**: M1 is effectively done and fully documented. Pose-setup refinement
continues **later**: (1) re-record with the lighting/background fixes, (2) optionally add a
temporal-smoothing pass for jitter. Otherwise ready to start **M4-PoC** (tempo) on the
keypoints JSON whenever we choose.
**Blockers**: None.
**Notes**: heavy model + heavy overlay were generated as a one-off experiment (gitignored
under data/); committed default remains lite.

---

## 2026-06-28 (cont.) — M1 implementation

**Duration**: ~1.5 hours
**What I did**: Implemented Milestone 1 (Capture & Skeleton). Wrote `FileVideoSource`
(`capture/file.py`, a `VideoSource` adapter over `cv2.VideoCapture`), implemented
`estimate_pose` (`pose/estimator.py`) with the raw→`FrameKeypoints` mapping isolated in a
pure `_to_frame_keypoints` helper, added `draw_skeleton` (`pose/overlay.py`) working purely
off our contract, and wired the lot in `scripts/run_pose.py` (capture → pose → keypoints
JSON + skeleton overlay mp4). Added tests for the pure mapping (no ML deps) and for
`FileVideoSource` (guarded by `importorskip` so the base suite still runs). Set up a `.venv`,
installed `.[vision,dev]`, and verified end-to-end on a synthetic clip. Wrote the M1 design
doc `docs/M1_CAPTURE_FLOW.md` (with mermaid data-flow + sequence diagrams).
**Key decisions / surprises**:
- **MediaPipe Tasks API, not the legacy Solutions API.** The installed mediapipe (0.10.35
  on Python 3.13) has *removed* `mp.solutions` — only `Image`/`ImageFormat`/`tasks` remain.
  Rewrote the estimator to use `PoseLandmarker` (VIDEO mode); it needs a `.task` model
  bundle, so `_ensure_model()` downloads `pose_landmarker_lite.task` (~5 MB) into
  `data/models/` on first run. Corrected `docs/M1_CAPTURE_FLOW.md` accordingly.
- Undetected frames emit 33 placeholder landmarks at visibility 0 (one record per frame) so
  the timeline stays aligned for M4-PoC.
**Verification**: `ruff check` clean; `pytest -q` → 9 passed (4 contract + 3 mapping + 2
capture); `python scripts/run_pose.py <synthetic.mp4>` produced a 15-frame keypoints JSON
(33 landmarks each) and an overlay mp4. Synthetic artifacts cleaned up afterward.
**Where I left off**: M1 code is complete and verified mechanically. **Next: the human
accuracy review** — drop a real swing clip at `data/raw/swing.mp4`, run `run_pose.py`, and
confirm the skeleton tracks address→follow-through; then document findings (30fps enough?
keypoints stable?) to close M1. After that, M4-PoC (tempo) can consume the keypoints JSON.
**Blockers**: None — needs a real swing clip for the accuracy review (no hardware required).
**Notes**: `.venv` created locally; `data/models/` and `data/processed/` are gitignored.

---

## 2026-06-28

**Duration**: ~1 hour
**What I did**: Design session on the central question — *how does the system decide what a
good swing is, distinguish good from bad, and pick the correction?* Surveyed where real
benchmark data comes from (outcome: TrackMan tour/amateur averages, Arccos/Shot Scope,
FlightScope, ShotLink; mechanics: TPI kinematic sequence + X-factor, GEARS/AMM3D/K-Vest,
academic biomechanics, Tour Tempo). Concluded the industry moat is *owning the data*, so v1
leans on published norms stored as cited data and migrates to our own captured data later.
Made four decisions and wrote them up as ADR-009 and ADR-010, then updated ROADMAP.
**Key decisions**:
- ADR-009 — **dual-axis scoring**: separate `mechanics_score` and `outcome_score`, combined
  by a **scoring policy chosen by the user's practice mode** (fundamentals / shot-shaping /
  performance / drill). Intent (`PracticeGoal`) parameterizes outcome ranges, so "good fade
  when I wanted straight = bad" and "don't care where it went, grade my fundamentals" both
  fall out naturally. `SwingResult` + `analyze_swing` carry intent and two sub-scores from
  day one. Supersedes the original single-score M4 framing.
- ADR-010 — **benchmark ranges as versioned data with provenance** (not hardcoded), resolved
  via `resolve_range(checkpoint, club, profile)` with most-specific→`all` fallback;
  parameterized by `ClubCategory` and `PlayerProfile`. v1 seeds **Tour Tempo (~3:1)** only;
  expand to TPI / TrackMan / Arccos as M2/M3 land, then to our own calibration data.
- PoC scope — **M4-PoC: pose-only Fundamentals analysis**. Prove phases→checkpoint→score→tip
  end-to-end on the M1 skeleton with the tempo checkpoint, one scoring policy, the intent +
  dual-axis seam in place but only Fundamentals implemented. No club detection, no hardware.
**Where I left off**: Decisions captured in ADR-009/010; ROADMAP now has M4-PoC before the
full M4. No analysis code written yet — `analysis/engine.py` and `feedback/rules.py` are
still stubs. M1 (capture + pose) remains the prerequisite for M4-PoC.
**Blockers**: None. M4-PoC depends on M1 producing keypoints first.
**Notes**: M1 is still the next *code* step; M4-PoC is the first analysis iteration that
follows it.

---

## 2026-06-20

**Duration**: ~1.5 hours
**What I did**: Picked the project back up after the planning session. Reviewed all docs/ADRs to re-orient. Decided to decouple software development from hardware acquisition and wrote ADR-007 to capture it; annotated ROADMAP milestones with explicit hardware dependencies. Discussed the role of YOLOv8 and how swing path is captured. Assessed how hard reliable club-head tracking really is and decided to de-risk it before investing — added **Milestone 1.5: Club-Head Detectability Spike** with a go/no-go gate. Then designed and scaffolded the whole project per ADR-008: `src/golf_coach/` package, the shared `contracts/` seam (fully implemented Pydantic models), ports + a working mock shot source, module stubs, pyproject with optional extras, tests, scripts, spikes/, frontend/, data/. Verified: `pip install -e '.[dev]'` works, `pytest` green (4 contract tests), `ruff` clean. Added `docs/FLOW.md` with PROPOSED-flow mermaid diagrams (runtime data flow, decoupling seam, build order + hardware gates, two-source swing path) and flagged ARCHITECTURE.md's diagrams as proposed too.
**Key decisions**:
- ADR-007 — software and hardware tracks run in parallel. M1 (capture + MediaPipe pose) starts now with phone/sample video, no purchase needed. MCP server (M3) gets a mock `ShotData` mode from the start. Hardware purchase is a parallel task, not a blocker.
- Added M1.5 detectability spike: prove the club head is detectable (especially through impact) BEFORE labeling 200–500 images. Strategy options to choose from once we see real frames: pure-ML / marker-assisted / fusion+interpolation. M2 labeling is now gated on this spike. Charter risk register and ADR-003 updated accordingly.
- ADR-008 — project structure & decoupling: a shared `contracts/` package is the seam (modules never import each other), ports+adapters at I/O boundaries (real vs mock), analysis is a pure functional core, heavy deps are optional extras so any module runs without the ML stack.
**Clarified**: YOLOv8 detects the club head + ball (MediaPipe only tracks the body); its detections through a tracker produce the visual club-path arc. The Garmin R10's `club_path` is the numeric counterpart. YOLOv8 is also the train-it-yourself ML exercise (ADR-005) and the most cuttable piece if scope tightens. Important correction: **global shutter removes distortion, not motion blur** — sharp impact frames need fast shutter + bright light (ADR-003 addendum).
**Where I left off**: Project is scaffolded and the contracts seam is real (tests green). Next concrete step is implementing **M1**: install the `vision` extra, write `FileVideoSource` (capture/file.py), implement `estimate_pose` (pose/estimator.py) over a sample/phone clip, and render a skeleton overlay via `scripts/run_pose.py`. In parallel, the M1.5 spike — grab a few swing clips with the impact zone and eyeball club-head detectability before any labeling.
**Blockers**: None for M0/M1/M1.5. Hardware purchase (cameras + R10) pending but no longer blocking.
**Notes**: —

---

## 2026-03-16

**Duration**: ~1 hour
**What I did**: Project planning session. Created project charter, architecture doc, roadmap, ADR templates, and decision log. Defined tech stack and milestone sequence.
**Key decisions**: Python stack, MediaPipe for pose, YOLOv8 for detection, MCP server for launch monitor, Claude API for coaching. See `docs/decisions/` for full ADRs.
**Where I left off**: No code yet. Next step is Milestone 1 — acquire/set up camera, record a test swing, run MediaPipe.
**Blockers**: Need to decide on and purchase camera hardware (ADR-003) and launch monitor (ADR-004).
**Notes**: —

---

<!--
TEMPLATE — copy this for each session:

## YYYY-MM-DD

**Duration**: 
**What I did**: 
**Key decisions**: 
**Where I left off**: 
**Blockers**: 
**Notes**: 

-->
