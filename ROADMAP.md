# Roadmap: AI Golf Swing Trainer

## Last Updated: 2026-08-21

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
| **M3** Launch monitor / MCP | 🟡 In progress | Nothing — ingestion + MCP server done; OCR tuning left | [§M3](#milestone-3-launch-monitor-integration--in-progress) |
| **M1.5** Detectability spike | ✅ Done *(2026-08-14, **no-go**)* | — (ran on footage already on disk) | [§M1.5](#milestone-15-club-head-detectability-spike-de-risk-before-investing) |
| **M7** Two-phone sim capture | 🟡 In progress, 6/7 phases (3 trimmed) | Nothing — two iPhones + a sim bay | [§M7](#milestone-7-two-phone-sim-capture-no-hardware-purchase) |
| **M4** full (outcome axis) | ⬜ Blocked | The M2 + M3 streams | [§M4 full](#milestone-4-full-swing-analysis-engine--the-outcome-axis) |
| **M6** LLM coaching | ✅ Done *(2026-08-15)* | — (live coaching, the MCP handshake and follow-up questions are all proven) | [§M6](#milestone-6-llm-powered-coaching--done) |
| **M6.5** Measure now, judge later | ✅ Done | — (9 recorded, **6 scored**; the handedness seam landed and the last candidate was settled) | [§M6.5](#m65-measure-now-judge-later--done) |
| **Career mode** One golfer over time | ✅ Done, 6/6 steps | — (built and silent; a bay session gives it the `n` to speak) | [§Career](#career-mode-one-golfer-tracked-over-time--done-built-and-silent) |
| **M8** Learning what "good" means | ✅ Done *(2026-08-17)* | — (three models fitted, validated, surfaced **and spoken**, with a policy rather than a band) | [§M8](#m8-learning-what-good-means--gates-run-model-fitted) |
| **M9** Player tracking (per-club) | 🟡 In progress, 7/20 phases | Nothing — the ingest half is desk work | [§M9](#m9-player-tracking-per-club-shot-history--in-progress) |
| **M5** Feedback UI | ⬜ Not started | M7 Phase 5 gives the host | [§M5](#milestone-5-feedback-ui) |
| **M2** Club & ball detection | 🔒 Gated, **and M1.5 said no-go** | Bay lighting for a ~1/2000 s exposure — *not* a global-shutter camera | [§M2](#milestone-2-club--ball-detection) |
| Hardware re-validation | 🔒 Gated | Cameras / launch monitor arriving | [§Gate](#hardware-re-validation-gate-revisit-when-cameras--launch-monitor-arrive) |

**M6's live path is proven** as of 2026-08-14. A real key is configured and
`python scripts/analyze_bundle.py 2026-08-10/2 --no-video` returned a paragraph written by
`claude-opus-5`, attributed in `analysis.json` (`feedback.coaching`) with a sha256 of the brief. It
led on tempo — the one checkpoint outside its band — stayed inside the brief, and volunteered both
caveats that applied: the launch-monitor numbers are attached but unscored, and the second view was
anchored only at top and impact. No spine angle, hip rotation, swing plane or club path. How the key
is *held* is [ADR-019](docs/decisions/019-secret-handling.md); it is a `SecretStr` in `.env`, and
`.env` is the only place it exists.

**The MCP handshake is proven too**, same day. The server was driven over stdio by a real client:
`initialize` negotiated protocol `2025-11-25`, advertised all **8** tools with their schemas, and
served live `call_tool` requests including the not-found path. It is registered with Claude Code
(`claude mcp add`, per the README) and reports `✔ Connected`, which is a second client completing
the same handshake independently.

**NEXT ACTION — do this first: M9 P8.** Until 2026-08-20 this section read *"nothing on this
board is desk work any more"*, and [M9](#m9-player-tracking-per-club-shot-history--in-progress)
is what stopped that being true. It is the one substantial item that needs **neither a bay session
nor an `n`**: no shot on disk records which club hit it, and adding that tag is pure desk work that
makes the *next* bay session's data worth more than the last one's.

**The ingest spine is closed.** P1–P7 all landed 2026-08-21, and a swing can no longer reach disk
untagged: `contracts/club.py` holds the taxonomy, `contracts/bag.py` the declared bag,
`storage/bag_store.py` puts a bag on disk at `data/processed/golfers/<player_id>.bag.json` beside
the golfer's own record, and **P4 carried the club into two things that already run**:
`SwingManifest.club` and a second session cursor, `SessionMeta.club`. P5 wrote the field — threading
the club through `bundle_store` at swing creation, plus a per-swing repair route — P6 put the
requirement at the boundary, and P7 gave the phone a one-tap way to satisfy it. Continue at
[M9 P8](docs/M9_PLAYER_TRACKING.md), which opens the measurements track and is independent of
everything above it; the design is
[ADR-024](docs/decisions/024-per-club-shot-history.md), whose addendum records the one call P3 added
to it — a club that leaves the bag is kept rather than overwritten.

P4 also fixed a latent bug it had to: `set_current_player` replaced the whole `session.json`, which
was correct with one cursor and would have cleared the club with two.

**P6 required the club and P7 made it pickable, and the order was deliberate.** `POST /api/uploads`
reads the session's club cursor before it streams a byte and answers 409 when nothing is selected,
which is the asymmetry ADR-024 §5 argues for (an untagged golfer is repairable later; nothing but
memory can say which club hit swing 3). For the length of one phase that left the upload page unable
to upload. P7 closed it with an always-open chip grid backed by a new `GET /api/clubs` — the picker
**derives** its list from `ClubId` rather than inlining it, which was the one design question P6
deferred, because a second copy of the taxonomy in a static file nothing tests would let a new club
parse at the API while being invisible at the bay.

Everything else still divides in two, and both halves want the same trip:

- **Needs a bay session**: M7 Phase 0's field spike, M3's remaining OCR work (the profile
  describes a screen layout the simulator can be configured out of — enumerating that needs the
  screen in front of you), and M2's lighting/shutter test, which the ADR-018 light was bought for
  and [BAY_SESSION_RUNBOOK.md §8](docs/BAY_SESSION_RUNBOOK.md) writes up.
- **Needs `n`**: career mode is complete and silent at n=2, and every surface built on it — the
  baseline, the dispersion discriminator, the tour join, and now the follow-up conversation —
  refuses rather than guesses. 20–30 swings in one session is what turns all of them on at once.

So the highest-value *trip* is still **one bay session**, and it unblocks more than any other
single thing. The runbook already sequences it. But M9's ingest half should land before that trip
rather than after it — a session hit without club tags produces data that can never be split by
club afterwards, which is the one kind of loss no amount of re-analysis repairs.

*(The smaller item that used to sit here — every shot carrying `screen title 'SHOT DATA' not
found` — was done on 2026-08-14, and was hiding a wrong number: `spin_axis` was stored with its
sign inverted, so both shots on disk were fades recorded as draws. See the
[ADR-014 addendum](docs/decisions/014-screen-capture-shot-ingestion.md). What is left of M3's OCR
work genuinely needs the bay: the profile describes a screen layout the simulator can be
configured out of, and enumerating those needs the screen in front of you.)*

**The one purchase that actually blocks something — and M1.5 changed what it is.** It is not a
global-shutter camera. The spike measured the club head smearing across 600–980 px at impact at
the bay's 1/60 s exposure, and put the requirement at **~1/2000 s**, which is on the order of
**30x more light** than the bay currently gives. Shutter *type* does not enter that calculation;
exposure *duration* is all of it. So the M2 purchase is **lighting first**, evaluated against
minimum exposure and lux — a global-shutter camera in the current light still records a smear.
See [ADR-017](docs/decisions/017-club-head-detection-strategy.md), and
[ADR-018](docs/decisions/018-bay-lighting.md) for what was bought and why: one 65 W COB rated
flicker-free at 1/2000 s, ~$200, to answer the question rather than to build a rig. The test it
buys is written up in [BAY_SESSION_RUNBOOK.md §8](docs/BAY_SESSION_RUNBOOK.md). Everything else
above either needs nothing, or needs work rather than money.

**Oldest unstarted item:** M7 Phase 0's field spike — and it needs a bay, not a decision.
(M1.5 held this slot until 2026-08-14, when it turned out to be answerable from footage already
on disk.)

**Biggest constraint on *coaching*, as opposed to measuring: n.** Causal coaching ("why is your
face open") is unreachable with this instrument — grip, wrists and clubface are all invisible to
it. What *is* reachable is per-golfer dispersion, which splits static causes from timing causes
without seeing the body, and needs 20–30 shots in one session rather than the one-per-session on
disk. See [Career mode](#career-mode-one-golfer-tracked-over-time--done-built-and-silent). No
purchase unblocks this, and as of 2026-08-12 **nothing to build does either**: all six steps are
done — golfer identity at capture; the corpus reader that counts the `n`; the backfill that brought
every stored analysis up to the current engine; the baseline plus the minimum-N guard that reads
it; the discriminator that turns a mean and a spread into which *family* of cause to investigate;
and the surfacing, which joins a personal center to the tour band and puts all of it behind three
MCP tools and a career page. The counter prints **n = 2 for every metric**, so every one of those
surfaces refuses. Every swing on disk is measured, `career_baseline.py` refuses all 27 claims over
those measurements, `career_dispersion.py` refuses both findings on all nine metrics, and the tour
join refuses all nine placements. The mechanism is complete and silent; only `n` is missing.

*(Counts read nine rather than eight as of 2026-08-13: M6.5's `head_hip_gain_norm` is picked up by
the corpus reader with no career-mode change at all, which is the registry-driven derivation doing
its job.)*

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

The pose-only track, complete. The first four ran on phone video and a public reference corpus —
no hardware was used or needed. **Career mode** joins them as of 2026-08-12 and is the odd one out
in a way worth stating: it is finished and it currently says nothing. Its deliverable is a
mechanism *plus the guard that keeps it quiet*, so refusing every claim over the two swings on disk
is the feature working, not the milestone being unfinished. What it waits for is `n`, and no code
supplies that.

**M6.5** closes the group on 2026-08-13, and its last decision is the one worth remembering: a
candidate metric was rejected not for being noisy but for **not surviving our camera**. Everything
this repo scores differences one landmark across time, where a camera bias is common-mode and
cancels; the one candidate that instead compared two body parts at a single instant disagreed with
the reference population by a third of a shoulder-width *before the swing started*. The panel is
six checkpoints, and the second question — *does this band transfer?* — now has a harness
(`check_metric_transfer.py`) beside the one that asks whether a metric is signal at all.

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
      *(2026-08-11: that refusal was about **instant detection**, and does not generalize —
      see [§M6.5](#m65-measure-now-judge-later--done). Spatial metrics measured at the
      already-validated instants pass the equivalent gate comfortably.)*
- [x] **Reasons, not just names, on `unscored`** *(2026-08-19)* — the evaluators return a
      `CheckpointOutcome` and `measure.py` a `MeasureOutcome`, so the reason is decided where the
      condition failed rather than inferred downstream. Eight causes in `contracts/unscored.py`;
      the load-bearing field is `refilming_helps`, which is what `feedback/rules.py` was
      *guessing* from whether the metric had survived into `measurements`. That heuristic is gone,
      and with it the two checkpoint names `feedback` had to retype because it may not import
      `analysis`. No number moved, so `ANALYSIS_VERSION` did not — see the ADR-010 addendum
- [ ] **Per-club percentiles** — the corpus has the strata, but the band has to move in step
      (ADR-010 addendum), so it is one change on two sides, not a percentile-only edit

**Exit Criteria**: a swing report that leads with the one thing to work on and quantifies it against
the tour population — **met**. The panel stayed at three checkpoints, which is a measured result
rather than an omission.


---

## Career mode: one golfer, tracked over time — done (built and silent)
**The idea**: everything today judges a swing against a *tour population*. Career mode judges it
against **the golfer's own history** — their baseline, their spread, their trend. It is the
missing half of the scoring model: `golfdb_v1.json` says what good looks like across 122 tour
players, and nothing yet says what *normal* looks like for the person actually swinging.

**Why it is worth its own milestone rather than a feature**: it changes what questions are
answerable, not just what is displayed.

- **Dispersion becomes a cause discriminator.** This is the strongest single argument for it. We
  can measure that a club face is open; we cannot see *why*, because grip, lead-wrist angle and
  release timing are all invisible to this instrument (wrists jitter 6x more than hips and 14.5%
  of frames fail the visibility gate; the club needs M2). But the **variance** of face angle across
  a session splits the causes without seeing the body at all: consistently open by a similar amount
  points at a *static* cause (grip, setup — checkable before you swing), while a wide spread points
  at *timing/release*. Those have completely different fixes. Three shots read +8.6, +2.8, -5.0 —
  suggestive of timing, and nowhere near enough to say so.
- **A fix becomes testable.** Since the cause is unknowable from this data, the honest method is
  empirical: baseline the golfer's face-to-path, change one thing, measure whether it moved toward
  zero. That needs a per-golfer baseline to move *from*.
- **Personal bands beat tour bands for feedback.** A 15-handicap held to a tour p10-p90 fails
  everything forever. Their own p50 is the useful comparison, with the tour band as the horizon.
- It subsumes the two MCP tools deliberately deferred in M3 (`get_shot_trends`, `compare_sessions`),
  which were held back for exactly this reason: *"a trend tool over n=3 reports noise in a
  confident voice."*

**What blocks it, and it is almost only one thing: n.** Nothing about this is hard to build — it
is a store, a few aggregates, and a minimum-N guard that refuses to speak below threshold. It is
deferred because building it now would produce a confident-looking trend line over three points,
which is the failure mode this repo exists to avoid. **The unblock is a bay session that captures
20-30 swings with shots attached.**

Two corrections to that framing, both found by looking at the disk rather than the roadmap:

- **`n` is 2, not 3.** Three of the four swings on disk are byte-identical re-uploads of one clip
  (`face_on.91b9d32c1afb` in `2026-08-07-aaron1/1`, `2026-08-09/2` and `2026-08-10/1`, with the
  same shot photo attached to all three). Since shots join by photo *hash*, a cross-session
  aggregate written naively would count one swing — and one shot — three times. The corpus reader
  has to dedupe on content hash, and report the duplicates rather than absorb them.
- **"not any code" was wrong about exactly one thing: capture-time metadata.** Everything else is
  derivable after the fact, so it can be built whenever. Who swung and which way they face are
  recorded or lost, so they had to land *before* the bay session, not after it.

**Step 1 is done (2026-08-11): golfer identity + handedness at capture.** `contracts/golfer.py`
(`Golfer`, `Handedness`, `slugify`), a flat-file registry in `storage/golfer_store.py`, a
per-session cursor in `storage/session_meta.py`, `player_id` stamped write-once onto
`SwingManifest`, a golfer bar on the upload page, and `scripts/backfill_golfer.py` (already run —
all four existing swings are `aaron`, right-handed). Uploads are deliberately never blocked on it;
setting a golfer adopts the swings that arrived unlabeled, and each swing row has a repair link.

**Step 2 is done (2026-08-11): the cross-session corpus reader.** `contracts/career.py`
(`CareerCorpus`, `CorpusSwing`, `ExclusionReason`), `storage/corpus.py` (`read_corpus`), and
`scripts/career_corpus.py`, which prints the honest `n`. Against the four swings on disk:

```
4 swing directories across 4 sessions  ->  2 distinct swings, 2 distinct shots
  face_on 91b9d32c1afb  kept 2026-08-07-aaron1/1  <- 2026-08-09/2, 2026-08-10/1
  n = 1 for all eight metrics; 1 swing analyzed pre-M6.5 with `measurements: []`
```

Two things the plan did not anticipate, both found by writing the test:

- **The two dedupe keys diverge in only one direction.** Pose metrics count distinct face-on clips,
  launch-monitor metrics distinct shot photos, and the real case is one photo attached to two
  different swings by `bundle_store`'s arrival rule — two pose samples, one shot sample. The
  reverse (one clip, two photos) looked like the stronger argument for two keys and is not a
  sample at all: one physical swing produced one ball flight, so a second photo is misattached.
  It is reported as `conflicting_shots` for repair rather than counted, because counting it would
  put a `face_to_path_deg` into the dispersion that no swing ever produced.
- **`excluded` means "contributes no sample", not "not in the corpus".** An unanalyzed or stale
  swing is a real distinct swing of this golfer's carrying no usable numbers *yet* — both are
  repaired by re-running the pipeline, so they are reported as work rather than as absence.

**Step 3 is done (2026-08-12): the backfill, and a second axis of staleness.** All four
`analysis.json` on disk are re-analyzed and carry all eight measurements; **`n = 2` for every
metric**. The re-run moved nothing else — scores, checkpoints, phases, windows and alignment
anchors are byte-identical to what was stored, which is the check that says the backfill added
data rather than changing history. New `scripts/reanalyze.py` is the repeatable form of it.

Two things found by looking at the disk, neither of which was in the plan:

- **Nothing could tell that a stored analysis was older than the engine.** It worked this once by
  accident: M6.5 happened to *add a field*, so "pre-M6.5" read as `measurements: []`. The
  2026-08-09 `_DRAWDOWN_FLOOR` fix is the counterexample — it moved a stored tempo from 0.43 to
  2.42 without changing the shape of anything. So `SwingBundleResult` now carries
  `analysis_version` (`contracts/swing.ANALYSIS_VERSION`, defaulting to **0** so a legacy artifact
  reads as older-than-current rather than claiming to be current), `read_corpus` reports
  `ExclusionReason.OUTDATED` and keeps those swings out of the counts, and `reanalyze.py` targets
  them by default. This matters for step 4 specifically: a `PersonalBaseline` reads *spread*, and
  pooling two engine generations manufactures variance out of a code change — the mirror image of
  the duplicate-counting error step 2 was built to refuse.
- **`analysis.state.json` could silently contradict `analysis.json`, and did.** `2026-08-09/2`'s
  sidecar read **66.67** with the pre-fix "Tempo too quick - 0.4:1" headline while its own
  analysis read **94.92** / "2.4:1", so the upload page showed 67/100 for a swing whose results
  page showed 95/100. `analyze_swing_dir` wrote the analysis and only `api/worker.py` wrote the
  sidecar, so every CLI run was a way to desync them — and `AnalysisState.matches` could never
  catch it, because it compares *inputs* and a re-analysis does not change the inputs. The
  terminal state is now written by `pipeline.record_state` as part of writing `analysis.json`;
  the worker keeps `queued`/`running`/crash, the three states no analysis on disk corresponds to.

**Step 4 is done (2026-08-12): the baseline, and the guard that keeps it quiet.**
`contracts/baseline.py` (`PersonalBaseline`, `MetricBaseline`, `BaselineClaim`, `WithheldClaim`,
the threshold table), `analysis/baseline.py` (`build_baseline`, `pooled_samples`), `mean_ci` /
`sd_ci` in `analysis/stats.py`, and `scripts/career_baseline.py`. Against the two swings on disk it
refuses **all 24 claims** — eight metrics × three claims — and each refusal names what it is waiting
for, so the output doubles as a worklist.

The gate is per **(metric, claim)**, not per metric, because the three claims have genuinely
different appetites for `n`: `CENTER` (a typical value), `SPREAD` (repeatability — the claim this
milestone exists for), `TREND` (movement, which additionally needs ≥3 *sessions*, since twelve
swings hit in one bay hour are one occasion). Every statistic carries a 95% CI, so a mean over 6
swings cannot print the same way as one over 60.

Four things found by building it:

- **The dedupe rule was private, and pooling could have disagreed with counting.** Step 2's keying
  lived inside `storage/corpus.py::_count_metrics`, which returned *counts and not values*. A
  baseline that pooled `swing.measurements` naively would have averaged a different number of
  values than the `n` printed beside it — and only in the cases the rule exists for (a re-uploaded
  clip; one shot photo across two real swings), which is to say only where it is invisible. The
  rule is now `CorpusSwing.artifact_key`, called by both sides, with the equality pinned end-to-end
  in `tests/storage/test_corpus.py`.
- **M6.5's spread/error ratios cannot set these thresholds**, which is the obvious place to reach
  for them. That ratio is *population* spread over *instrument* error, while what binds a personal
  baseline is the golfer's own shot-to-shot variability — unmeasured, and much larger. Deriving
  from tempo's r = 2.4 yields a usefully-resolved personal mean at **n ≈ 3**. So the floors are
  judgment, documented as judgment, and the CI is what makes that safe: a floor set too low shows
  up as a visibly wide interval rather than as a confident wrong number.
- **Withheld had to mean *absent*, not flagged.** A statistic shipped beside a `ready: false` is one
  forgotten conditional away from being rendered anyway. Gated fields are `None`, which is
  `Measurement`'s "structurally incapable of reading as a verdict" one level up. What stays
  populated is the *evidence* — `n`, `n_sessions`, the per-session counts — because those are facts
  about how much data exists rather than claims about the golfer, and they are what makes a refusal
  actionable.
- **`head_hip_offset_impact_norm` is readable here and nowhere else.** M6.5 blocked it from becoming
  a checkpoint because its sign is camera-relative and a band cut from GolfDB's mixed-handedness
  population would be meaningless. A personal corpus is single-handed by construction, so the sign
  is consistent without consulting `Golfer.handedness` at all — the one metric a personal baseline
  can interpret that a tour band cannot.

**Step 5 is done (2026-08-12): the cause discriminator, and a reading written for one metric.**
`contracts/dispersion.py` (`Finding`, `DispersionPattern`, `MetricTarget`, the `METRIC_TARGETS`
table, `MetricDispersion`, `GolferDispersion`), `analysis/dispersion.py` (`build_dispersion`,
`dispersion_for`), and `scripts/career_dispersion.py`. Two findings rather than one verdict —
**bias** (the center sits further from the target than measurement error explains) and **scatter**
(the spread is larger than measurement error explains) — because a golfer can have both and the
*contrast* is the entire signal: the same 6° average miss means opposite things at sd 1 and at sd 9.
Both are decided by an interval and never a point estimate, so a tolerance set wrong surfaces as an
honest "cannot tell" instead of a confident wrong pattern. Against the disk it refuses both findings
on all eight metrics.

Five things found by building it:

- **The reading was written for one metric and printed for all eight.** The first cut of the
  `BIASED` text named the things to check outright — "grip, alignment, ball position, face at
  address" — which is right under `face_to_path_deg` and is nonsense under `head_sway_norm`, where
  it printed unchanged. A golfer would have been sent to check their grip about a head that moves.
  One reading serves every metric, so it may name only the *class* of cause; the specific check
  needs per-metric vocabulary and belongs in `feedback`.
- **Six of the eight tolerances did not have to be invented — they were already measured.**
  `tune_spatial_metric.py` computes `noise` (two estimators at identical labelled instants) plus
  `bound` (labelled against detected segmentation), which is exactly "the smallest difference
  distinguishable from this pipeline's own error". Re-run over the 461-clip face-on corpus: 0.024
  (`finish_balance_norm`) to 0.943 (`tempo_ratio`). Tempo's `noise` is **0.000** and that is
  structural rather than lucky — it reads phase instants only, and both estimators were handed
  GolfDB's labelled ones, so its whole error term is our own address detection. The two
  launch-monitor metrics have no analogue for a photographed screen, so their 2.0° is judgment,
  documented as judgment and flagged as the first thing a bay session should revise.
- **Four of the eight can carry a scatter finding and must not carry a bias one.** Declaring a
  target is declaring what *good* is, which this repo does in exactly one place — a band with a
  derivation behind it (ADR-010 §2). `hip_sway_norm` and `hip_shift_at_top_norm` have no band and
  "less is better" is not established for either (some lateral hip travel is a weight shift);
  `head_hip_offset_impact_norm` has a readable sign but no known right amount; and `tempo_ratio`'s
  target *is* a band, so reading it here would import `benchmarks` into the personal-baseline path
  — the boundary step 4 held on purpose. Each refusal carries its reason rather than being absent.
- **A bias on a one-sided magnitude asserts less than it reads.** Zero head sway is not attainable
  by a human, so "the center is distinguishable from 0" is established for essentially every
  golfer. What it actually says is *a consistent amount, above measurement error* — which is
  precisely the half of the contrast this step needs — and **not** that the amount is too much.
  That question is the tour band's, and it is step 6.
- **The guard did not need re-deriving, it needed consuming.** `MetricDispersion` is built from the
  `MetricBaseline` step 4 already sealed, so when `CENTER` was refused there is no mean in the
  input to test a bias against — absent rather than ignored. Only `_refuse` had to become public
  (`analysis.baseline.refuse`), so both steps build refusals from one definition; a second copy is
  how two floors drift apart with the looser one deciding what gets said.

**Step 6 is done (2026-08-12): the surfacing, and the join that closes the scoring model.**
`contracts/comparison.py` + `analysis/comparison.py` (the personal-vs-tour join), `mcp/career.py`
plus three MCP tools, `GET /api/golfers/{id}/career`, a new `static/career.html`, an "Against your
own history" block on the swing page, and the `vs tour` row in `scripts/career_baseline.py`.
`storage.corpus.narrow_to` is the one new reader. 39 new tests, ruff and mypy clean. (The running
total lives in `WORKLOG.md`'s top entry — a repo-wide count written into a milestone section is
stale by the next session.)

The join answers the question the tour band was always going to be asked: whether the golfer's own
center sits where tour swings sit. It is read off the mean's **95% CI**, never the mean, so a
center a hair past p90 with an interval crossing it reports `straddles` — unresolved — rather than
a placement that flips on the next swing. Against the disk it refuses all eight.

**The four target-less metrics did not need targets after all.** The plan going in was that a bias
finding for `tempo_ratio` was "a one-line edit to `METRIC_TARGETS` once a band exists". It is, and
it would have been wrong: the band *is* the answer, and asking whether the center's CI sits inside
p10–p90 is the band's own question asked directly. Inventing a point target — the midpoint of
2.72–4.71 — would have declared 3.7 to be what *good* is, which is a claim nobody derived.
`METRIC_TARGETS` is unchanged, and the join answers what the missing targets were wanted for.

Five things found by building it:

- **The one metric a personal baseline can read is the one metric the tour band cannot.** Step 4
  established that `head_hip_offset_impact_norm`'s sign is readable personally, because a personal
  corpus is single-handed by construction. The stored distribution is not: it is cut from GolfDB's
  mixed-handedness population, which is exactly why M6.5 blocked the metric as a checkpoint. So the
  join has to refuse the one metric that has both a center and a distribution — a stored row
  existing is not a stored row meaning something, and "did we get a row back" would have passed.
- **`sd` against `sd` is a category error and looks like a free second finding.** `Distribution`
  carries one, one field from the p10 the join already reads. But the tour `sd` is *between-player*
  variation (458 clips over 122 players, under four each) while a personal `sd` is *within-player*
  repeatability. "Your spread is tighter than the tour's" compares one golfer's consistency against
  how much a field differs from itself, and comes out flattering for everyone alive. Recorded in
  `unavailable` rather than computed.
- **"Outside" is a verdict for half the panel and the opposite for the other half.** Found by
  rendering the speaking path: `finish_balance_norm` sits *below* p10 on a one-sided `[0, high]`
  band, which is better balance than the tour population — and the page printed "outside tour
  range" in amber, identically to a center above p90. Two fixes, both structural: every standing
  pill is now neutral (the contract carries no `score` and no `passed` precisely so it cannot read
  as a verdict, and colour was re-adding one), and `outside` always names its side. This is step
  5's own defect repeating one milestone later — a shared reading that is correct for some metrics
  and misleading for others — and again only a render caught it.
- **A withheld claim keeps its evidence, and an LLM can do arithmetic.** The refusal ships `n`,
  `n_sessions` and the per-session counts, because that is what makes it actionable. Handed
  "session A: 4 swings, session B: 5", a model can average them and narrate the trend the guard
  just refused; nothing in a payload prevents it. So `contracts/caveats.py` gained
  `READING_A_PERSONAL_HISTORY`, kept *separate* from the block the coaching call gets — that one
  writes about a single swing, and rules for tools it does not have teach a reader to skim.
- **The boundary moved out one layer instead of dissolving.** `analysis.baseline` and
  `analysis.dispersion` still import no `benchmarks`; the join is the only module that imports
  both. Pinned by a test that reads the two files' **source**, because the obvious runtime version
  cannot work: `analysis/__init__.py` imports `engine`, so importing anything under
  `golf_coach.analysis` pulls the bands in before the module body runs. A `sys.modules` check there
  passes for the wrong reason forever.

`TREND` is defined and gated but exposes only the per-session breakdown; the inferential statistic
(a slope, and whether it differs from zero) is deferred until there is a corpus to test it against.
`get_shot_trends` keeps ADR-006's name though it now covers the six pose metrics too.

**Design notes** (recorded while the reasoning was fresh; steps 1–4 have since built all three):
- The minimum-N guard is the load-bearing part, and it should refuse per-metric rather than
  globally: face-angle dispersion needs far fewer shots to be meaningful than a carry-distance
  trend does. *(Built in step 4, and refined one notch finer — the gate is per **(metric, claim)**,
  because `SPREAD` needs more `n` than `CENTER` on the same metric.)*
- `Measurement` (M6.5) is the right input. Every analyzed swing already records eight of them with
  no band attached, which is exactly the shape a personal baseline is cut from — and why recording
  them before they could be judged was worth doing. *(Confirmed: `build_baseline` reads nothing
  else.)*
- Handedness is now on the `Golfer` record, which is what finally lets
  `head_hip_offset_impact_norm`'s camera-relative sign be interpreted — it stays unscored until a
  checkpoint reads that field, since the GolfDB band behind it is cut from a mostly right-handed
  population. *(Step 4 found this is a constraint on the **tour band only**: a personal corpus is
  single-handed by construction, so a personal baseline reads the sign without the field.)*

**Open on the bay session, and it is the one thing here that is not merely waiting for `n`.** M8's
five population placements pool through `build_baseline` as if they were personal metrics, and the
two down-the-line ones cannot be deduplicated because `CorpusSwing` carries no down-the-line hash.
Nothing false is said at n=2 — every claim is withheld, and dispersion and the tour join refuse
them outright — but the default `CENTER` floor is 5, so **the first bay session is also the moment
a center appears for `tour_trajectory_q_dtl`**, a quantity that ships labelled *NOT calibrated*.
The decision was deferred rather than defaulted; the argument, the trigger and the one-line seam
are [ADR-022's fourth addendum](docs/decisions/022-learned-artifacts-as-committed-data.md). Read it
**before** analysing that session's swings, not after.


## M6.5: Measure now, judge later — done
**Goal**: Record every quantity this system *can* measure from data already on disk, without
scoring any of it — so bands can be derived from a real population later, and so a swing captured
today is still worth re-reading once they exist.

**Why it exists**: measurement and judgment were fused. `evaluate_head_sway` measured, resolved a
band, scored, and returned `None` if *any* step failed — so a metric with no band could not be
measured, while bands are derived from populations of measurements. That circle, not the difficulty
of any metric, is why the panel sat at three checkpoints for two milestones.

- [x] **Split measure from judge** *(2026-08-11)* — `analysis/measure.py` holds the measuring half
      (pure, no `resolve_range`, `None` means *could not measure*); `checkpoints/mechanics.py` keeps
      the judging half. The three evaluators are unchanged in behaviour, pinned by the existing
      `tests/analysis` suite passing with no assertion moved
- [x] **`Measurement` contract** — no band, no `passed`, no `score`. ADR-010 §2 expressed as a type
      rather than a convention: a measurement is structurally incapable of reading as a verdict
- [x] **Three new face-on pose metrics** — `hip_sway_norm`, `hip_shift_at_top_norm`,
      `head_hip_offset_impact_norm`. All hips/shoulders/ears, all windowed, all `x`-over-`x` and so
      immune to the 16:9 pixel-aspect assumption. Vertical and shoulder-tilt metrics were left out
      for being aspect-*sensitive*; ankles and knees for having zero recorded reliability evidence
- [x] **Two launch-monitor metrics** — `face_to_path_deg` (the observable ADR-009 §Concepts names
      for shape) and `start_line_deg`. `smash_factor` / `club_head_speed` are deliberately excluded:
      every shot on disk reads smash 0.89-1.00, i.e. ball speed *below* club speed, and the OCR is
      faithful, so the simulator itself is printing a physically impossible number. `spin_axis` too
      — its sign contradicts the contract and the parser warns it stored an uninterpreted magnitude
- [x] **`scripts/golfdb/tune_spatial_metric.py`** — the gate the repo did not have. It had three
      harnesses for *temporal* rules and none for spatial quantities, so "should we add this
      checkpoint?" was answerable only by argument. Scores population spread against measurement
      error, where error is estimator disagreement (`lite` vs `full`, both already cached for all
      461 face-on clips, no new extraction) plus segmentation error (labelled vs detected instants)
- [x] **Registry-driven derivation** — `derive_pose_metrics.py` iterates
      `measure.POSE_MEASUREMENTS` instead of hardcoding two metric names in three places, so a
      candidate metric is one line. `derive_reference.py` needed no change; it auto-discovers
- [x] **Decide what to promote** *(2026-08-12)* — **two of the five, and the panel is now 3 → 5.**
      `hip_sway_norm` (`0.14-0.50`) and `hip_shift_at_top_norm` (`0.0-0.21`) are scored checkpoints;
      the bands were already derived from the same 458 face-on GolfDB swings and needed no new data.
      The "wants more than one golfer's swings" note above turned out to be aimed at the wrong
      thing: the bands come from 122 tour golfers, not from ours, and what actually had to be
      decided was **band shape**. `derive_reference.py`'s one-sided `[0, p90]` default encodes *less
      is better*, which career mode step 5 had already established is **not** true of hip travel —
      some of it is the weight shift a swing needs. So `hip_sway_norm` is two-sided (its p10 of 0.14
      sits 2.8x the metric's 0.050 measurement error above zero, so "too little" is a real
      distinction) while `hip_shift_at_top_norm` is one-sided for the opposite reason — its p10 of
      0.015 sits *below* a 0.053 error floor, so a lower edge would split golfers this pipeline
      cannot tell apart. Rule recorded in the [ADR-010 addendum
      (2026-08-12)](docs/decisions/010-benchmark-ranges.md): assert a band edge only where it clears
      the instrument. `ANALYSIS_VERSION` 1 → 2, since `overall_score` is a mean over five now
- [x] **The handedness seam** *(2026-08-13)* — `analyze_swing(..., handedness=)`, resolved from the
      manifest's `player_id` by `api/pipeline.py` and never by `analysis`, which stays pure and
      imports no registry. `None` costs the swing that one checkpoint and says so in `unscored`;
      guessing right-handed would read a left-handed golfer's ordinary impact position as a gross
      fault, which is the failure that blocked this metric for two milestones
- [x] **`head_hip_offset_impact_norm` was rejected, and its *delta* promoted instead**
      *(2026-08-13)* — the panel is 3 → 5 → **6**. The absolute offset failed a transfer check that
      had never been run: `scripts/golfdb/check_metric_transfer.py` measures the same quantity **at
      address**, where the body is square and no swing has happened, and found our bay clips sit
      **0.32 shoulder-widths** from the corpus there — 55% of the whole gap at impact and ~4x the
      metric's own error. Scoring it would have made "not staying behind the ball" the top tip on
      every swing on disk, roughly half of it camera. `head_hip_gain_norm` (impact minus address,
      one shared address-window ruler) removes that static term by construction, and costs almost
      nothing to do so: ratio **7.1** against the absolute's 7.6. Band two-sided `[-0.67, -0.14]`,
      both edges 3.4x the 0.080 error. `ANALYSIS_VERSION` 2 → 3

> **Status (2026-08-11):** all six pose metrics clear the gate over the full 461-clip face-on
> corpus (ratio = spread / error): `finish_balance` 8.2, `head_hip_offset_impact` 7.6,
> `hip_sway` 7.1, `head_sway` 6.7, `hip_shift_at_top` 3.6, `tempo` 2.4. The harness validates
> itself three ways — it reproduces `head_sway_norm`'s shipped band (p10-p90 **0.029-0.430** against
> `ranges.json`'s 0.0-0.43), `finish_balance_norm`'s p90 (**0.287** against 0.29), and the face-on
> `tempo_ratio` p90 of **5.000** that `mechanics.py` documents against the all-view 4.71.
>
> Two things found by running it. Normalizing the simulator's shape text matched `CENTER` before
> `FADE`, classifying a real recorded `"CENTER SLIGHT FADE"` as **straight** — curvature words now
> beat centering words. And re-deriving showed the corpus had been storing `CheckpointScore.observed`,
> which is **rounded to 2dp**; measurements now carry full precision, which moves 42 distribution
> rows by under 0.005 and leaves every shipped band unchanged at the precision it quotes.
>
> `head_hip_offset_impact_norm` is signed and camera-relative. It is empirically *not* bimodal on
> this corpus (p10 -0.88 to p90 -0.33, consistently head-behind-hips) so a band is derivable — but
> that is a fact about GolfDB's handedness mix, not a guarantee, and handedness must be resolved
> before it becomes a checkpoint. `derive_reference.py`'s recommended band for it is also garbage
> (`low=0.00 high=-0.33`): its one-sided heuristic assumes non-negative values.

> **Status (2026-08-13): done.** The handedness seam is built and the last candidate is settled —
> against, in the form it was proposed, and for in a form that survives our own camera. What the
> transfer check added to this repo is a second question to ask of any band: not only *is this
> metric signal rather than jitter* (`tune_spatial_metric.py`) but *does the population it was cut
> from project the way ours does*. Every shipped checkpoint differences one landmark across time,
> where a camera bias is common-mode and cancels; the absolute head-hip offset was the first
> candidate that did not, and it is the first that failed.
>
> Two things found by running it rather than reasoning about it. Classifying handedness
> per-metric gave `TOBY KEITH` opposite labels on the two signed metrics from medians of -0.110
> and +0.015, both inside measurement error — so handedness is now resolved once per subject from
> the metric with the widest separation. And the stored `sd` for `head_hip_offset_impact_norm` was
> **half artifact** (0.504 against 0.249): two clips of one Stacy Lewis driver swing read 5.44 and
> 6.13 shoulder-widths, a collapsed `shoulder_width` denominator rather than a body. Quantiles were
> robust to it so no shipped band moved, which is exactly why nothing had caught it.

**Exit Criteria**: every measurable quantity recorded on every analyzed swing, with a harness that
says which of them a band is worth deriving from — met, and the promotion decisions that were
deliberately held separate have now all been taken. The panel is **6 checkpoints**. The one
candidate this milestone rejected is still measured on every swing, so a later camera-geometry fix
can revisit it without re-capturing anything.


---

## M8: Learning what "good" means — gates run, model fitted

**The idea**: everything before this judges a swing one number at a time. A coach does not. Most of
what a coach knows that an average golfer does not is *conditional* — a wide hip slide is fine if
the head stays back and a fault if it does not — and six independent bands have no way to hold an
"if". This milestone asked what could be learned from data instead of asserted, and ran two gates
to find out.

**Gate 1 — can mechanics predict the ball? No.** [ADR-021](docs/decisions/021-caddieset-paired-reference-data.md).
CaddieSet (MIT, 924 face-on shots, 8 golfers of mixed skill) is the first corpus here with mechanics
and ball flight on the same row. Leave-one-golfer-out: spin axis **0.532** against **0.572** for
knowing only which club was hit; carry **R² = -0.205**, worse than predicting that golfer's average.
Start direction cleared its baseline marginally (0.594 vs 0.535) and a regularisation sweep moved
nothing. Centering each feature on the golfer's own mean made it *worse* — the little signal there
was lived between the eight golfers, not inside any of them.

That is what ball-flight physics predicts and what [§Career](#career-mode-one-golfer-tracked-over-time--done-built-and-silent)
already suspected: the club sets the ball and a face-on camera pointed at a body does not see the
club. It is also the first empirical support [ADR-009](docs/decisions/009-swing-scoring-model.md)'s
two-axis split has had.

**Gate 2 — is there joint structure the bands are missing? Yes.**
`scripts/golfdb/tune_joint_structure.py` over the 458 face-on clips: `head_sway_norm` ×
`hip_shift_at_top_norm` at **+0.441**, × `head_hip_gain_norm` at **-0.385**, two more past 0.2,
correlation-matrix condition number 4.8.

**What shipped**: a robust center, scale and inverse correlation of the tour population, fitted
offline under the `research` extra and committed as ~50 numbers that stdlib arithmetic evaluates —
the pattern `ranges.json` already is ([ADR-022](docs/decisions/022-learned-artifacts-as-committed-data.md)).
Leave-one-*player*-out exceedance 11.1% against a 10% target, so the shape transfers to golfers the
fit never saw. On `2026-08-10/2` it places a swing scoring **96.9** at the **73rd percentile** of
unusualness, with `head_hip_gain_norm` contributing 38.6% of the departure *while passing its band*.

- [x] `scripts/caddieset/{fetch,ingest,study_panel}.py` and the corpus decision
- [x] `scripts/golfdb/tune_joint_structure.py` — the gate, plus player-clustered band intervals
- [x] `scripts/golfdb/derive_joint_model.py` → `analysis/benchmarks/joint_model_v1.json`
- [x] `analysis/benchmarks/joint.py` + 12 pins in `tests/analysis/test_joint.py`
- [x] **Surface it.** Landed in two halves, and the gap between them is the lesson. M8.1 did the
      data half — registered as a `Measurement`, `ANALYSIS_VERSION` 3 → 4, `reanalyze.py` re-run —
      and **M8.3 did the prose half**, which had been left undone: `caveats.py` named none of the
      five placements while all five shipped, so an MCP client received them as bare floats under a
      field description promising there was no percentile
- [x] **Revisited `hip_sway_norm`'s lower edge — kept at 0.14, 2026-08-18.** The box was opened by
      the player-clustered bootstrap putting p10's 95% interval down at **0.0801** (458 clips from
      122 golfers are not 458 independent samples), described here and in WORKLOG as "1.6× the
      0.050 error floor rather than 2.8×". **That phrasing mixed two different measurements**, and
      separating them is what settled it: 2.8× is a claim about *resolution* — can the pipeline
      tell 0.14 from zero — and clustering does not touch it; the interval is a claim about
      *placement*, where the tour p10 sits. Only the second widened, so the finding is reduced
      confidence in the edge's location rather than an unmeasurable edge, and
      `hip_shift_at_top_norm` (p10 *below* its floor) is not the precedent it resembles. The edge
      stays, `ranges.json` now carries the interval beside the 2.8×, and no score moved — all four
      stored swings pass at 1.0. [ADR-010 addendum
      2026-08-18](docs/decisions/010-benchmark-ranges.md)
- [x] **Per-club bands gated, and none is cut — 2026-08-18.** Costed was not gated: the counts
      below say a stratum is big enough to cut a band from, not that the band differs from the one
      shipped. `scripts/golfdb/tune_per_club_bands.py` screens each per-club p90 against the
      all-club p90 in units of the metric's own error, then puts a player-clustered bootstrap on
      whatever clears. **Two of nineteen strata survive** (`head_sway_norm` x iron, 2.85x;
      `finish_balance_norm` x fairway, 2.56x) and they do not make a panel. **Driver never differs
      at all** (0.17–0.56x) because it *is* 341 of the 458 face-on clips. Pooling every non-driver
      club onto 42 golfers leaves exactly one real effect — head sway is lower with a shorter club,
      2.20x — and there is no `ClubCategory` for "not a driver". **Tempo is on the wrong axis
      entirely**: the golfer effect is 4.7x the club effect, and the club effect is 5.8x *smaller*
      than our own tempo error, so tempo belongs to `PersonalBaseline` and not to a band here.
      [ADR-010 addendum 2026-08-18](docs/decisions/010-benchmark-ranges.md) has the three blockers
      that would have to clear before any of it ships

**Exit criteria**: a swing can be told its combination is unusual, with the metric responsible
named — **met**, on a `SwingResult` since M8.1 and in words a golfer reads since M8.3.

### M8.1 — the trajectory model (NEXT ACTION, agreed 2026-08-16)

**Why**: the shipped model reads six scalars at four instants. The Tier 1 cache holds **461 face-on
clips as full 33-landmark time series**, so the model currently ignores most of the signal already
on disk. A trajectory model also unlocks the one thing the scalar model structurally cannot do —
saying **when** in the swing the departure happens, which is the sentence a coach actually gives.

Steps, and **the ordering matters — step 1 gates step 3**:

1. - [x] **Gate the `z` channel — PASSED with a caveat, 2026-08-16.**
        `scripts/golfdb/tune_z_channel.py`, screening every axis on `tune_spatial_metric.py`'s bar
        (spread ÷ noise ≥ 2.0) over the 461 face-on clips that have both estimators cached.
        Median ratio **`x` 9.86, `y` 18.09, `z` 2.69**, all 12 landmarks clearing 2.0 at n = 3,521.
        So `z` is not pure noise — but it carries only ~22% of the planar signal-to-noise, **and the
        screen flatters it**: lite and full are one architecture at two sizes, so they make
        *correlated* monocular-depth mistakes and agree with each other while both guessing.
        **2.69 is an upper bound on `z`, not an estimate.** Decision: carry `z` into step 3 and
        settle it there by fitting with and without it. Writeup: `docs/M4_POSE_BAKEOFF.md` §Phase D,
        which also records the two silent bugs found on the way (event indices need rebasing on
        `start`; spread must be hip-relative).
2. - [x] **Down-the-line keypoints extracted, 2026-08-16.** All **584** DTL clips, 0 missing, in
        1,501 s at 109 fps. The Tier 1 cache now holds **1,045** clips — 461 face-on and 584
        down-the-line — where before it held only the face-on half. (GolfDB's remaining 354 clips
        are view `other` and are not worth extracting: they are neither of our camera positions.)
        Nothing consumes the DTL half yet; it is the raw material for the item below.
3. - [x] **Fitted and validated, 2026-08-16** — `scripts/golfdb/derive_trajectory_model.py` →
        `analysis/benchmarks/trajectory_model_v1.json` (98 KB). 12 landmarks, **x/y**, 40 timesteps
        on **3 detected anchors**, PCA to **10 components**, 73.6% variance, over 415 clips from
        116 golfers. Leave-one-player-out exceedance **T² 8.9%** against a 10% target.
        Three findings, all in `docs/M4_POSE_BAKEOFF.md` §Phase E:
        **`z` lost** — it lowered variance explained at equal component count and worsened both
        calibrations, confirming §Phase D's warning that its gate score was an upper bound.
        **The anchor set nearly shipped unusable** — four of GolfDB's eight annotated events are
        ones `segment_phases()` cannot produce, so a model anchored on them could never score a
        real swing; the three we do detect validate better anyway.
        **`Q` is not calibrated** (14% against 10%) and that is a property — a golfer the basis
        never saw has idiosyncrasies that land in the residual by construction. Both exceedance
        figures ship inside the artifact so a consumer can see how far to trust each.
   - [x] **Loadable from `analysis/`, 2026-08-16.** `analysis/trajectory.py` is the **single**
        feature builder — stdlib, in the package — and `derive_trajectory_model.py` imports it
        rather than keeping a numpy copy, so a vector cannot be built one way at fit time and
        another at scoring time. `analysis/benchmarks/trajectory.py` projects onto the basis;
        18 pins in `tests/analysis/test_trajectory.py`.
   - [x] **A pixel-aspect bug, found on the way.** `videos_160` squashes a non-square crop square,
        so x and y sit on different scales per clip (ADR-012). Metrics built from x-ratios cancel
        it; this model mixes axes and did not. Correcting it moved variance explained
        **73.6% → 80.3%** and changed the optimal component count from 10 to **6**.
4. - [x] **Surfaced, 2026-08-16.** `ANALYSIS_VERSION` 3 → 4, one `reanalyze.py` run,
        `measurements` 9 → 12 on all four stored swings and **every `overall_score` identical** —
        the placements ride on `measurements`, never `checkpoint_scores`, so they cannot move a
        score. New `source` value `population:golfdb`, alongside `pose:face_on` and
        `launch_monitor:*`, marks them as population-relative rather than measured off the body.

**Two things settled in discussion, recorded so they are not re-litigated:**

- **There is no 3D here, and that is structural rather than pending.** The reference corpus is
  single-view: of 580 source videos only 60 contain more than one view, and just **14 cross-view
  clip pairs overlap in time** — nowhere near enough to fit anything. And on our own capture,
  [ADR-011](docs/decisions/011-camera-synchronization.md)'s addendum already ruled that hand-held
  phones "can be aligned but never fused… unreachable by construction", because two people holding
  phones differently every swing have no stable extrinsics.
- **The second camera's value is a second 2D model, not depth.** Down-the-line sees what face-on
  cannot — spine tilt, swing plane — which is what step 2 is for. Two per-view models is exactly
  what "aligned but never fused" implies architecturally.

**More reference swings are *not* the bottleneck right now.** 458 clips against ~21 fitted
parameters is comfortable; another 500 tour clips would barely move the scalar model. That flips at
step 3, where hundreds of dimensions make n=458 start to pinch — so extract more from the clips on
hand *first*, and only then go looking for more clips.

### M8.2 — a down-the-line model (not started)

584 DTL tour swings are now cached and nothing reads them. The face-on models are blind to
everything that lives in the other plane — spine tilt, swing plane, the arm-parallel positions M5's
gate rejected on face-on evidence — and this is the corpus for it. Two per-view models, never a
fused 3D one, is exactly what [ADR-011](docs/decisions/011-camera-synchronization.md)'s addendum
implies: hand-held phones can be aligned but never fused.

What it needs, and none of it is a rerun of M8.1:

- [x] **Its own anchor set — answered and actioned, 2026-08-17.** On the lead wrist the shipped
  rule misses the top on **30%** of down-the-line clips and impact on **35%**. On the **trail**
  wrist — nearer that camera, tracked in 70% of frames against the lead wrist's 39% — the same rule
  reaches **7%** and **2%**, better than face-on manages. `segment_phases` now takes the landmark
  and the bundle path passes `TRAIL_WRIST` for the DTL clip. This also answers
  [M7 Spike](docs/M7_TWO_PHONE_SPIKE.md) Q1, the biggest unmeasured risk under the two-phone
  ladder, and reverses M4_POSE_BAKEOFF §Phase B7's "no view-aware landmark selection is warranted"
  — that was one bay swing; this is 1,045 labelled clips.
- [x] **Its own landmark list — measured 2026-08-17.** `scripts/golfdb/tune_landmarks.py` over all
  584 DTL clips: **the whole lead arm is gone**, not just the wrist. Lead elbow 0.46, wrist 0.47,
  thumb/index/pinky 0.37-0.40 tracked, against 0.84-0.87 on the trail side. Shoulders, hips, knees
  and ankles are fine on both sides — it is specifically the arm that swings across the body and
  is hidden by the torso. Two of the face-on twelve are among the five failures, so that list
  cannot be reused. Proposed DTL twelve: **ears, shoulders, hips, knees, ankles, plus the trail
  elbow and trail wrist**. See M4_POSE_BAKEOFF §Phase G.
- [x] **Fitted and settled empirically, 2026-08-17.** `derive_trajectory_model.py` gained `--view`
  and `--landmarks`, and writes one artifact per view. `trajectory_model_dtl_v1.json` (63 KB): 12
  landmarks, x/y, 40 steps on 3 detected anchors, PCA to 6 components, **510 clips from 166
  golfers** — broader than the face-on model's 415/116 — with leave-one-player-out T² exceedance
  **10.2%** against a 10% target.
  **The landmark list is worth far more than the screen suggested**: handing the face-on twelve to
  a down-the-line fit skips **441 of 584 clips (72%)**, because the lead arm is missing too much of
  its timeline, and what survives is the biased remnant where it happened to stay visible
  (Q calibration 20.3% against 12.2%). M4_POSE_BAKEOFF §Phase H.
  ⚠️ **93.7% variance explained is not a boast.** Down-the-line the swing runs toward and away from
  the camera, so the features are more redundant and fewer directions describe them. This model is
  well-calibrated and probably sees *less* of the swing than the face-on one.
- [x] **Surfaced, 2026-08-17.** Per-view loading in `benchmarks/trajectory.py`, and
  `analyze_swing_bundle` records `tour_trajectory_t2_dtl` / `_q_dtl` beside the face-on pair.
  `ANALYSIS_VERSION` 5 → 6; face-on untouched, every stored score identical, `measurements` 12 → 14
  on a two-view bundle.
  **The design answer: the two are never blended.** Two cameras answering the same question about
  different planes; a mean of them answers neither, and blending would be the mistake ADR-009
  avoided by keeping mechanics and outcome apart. Disagreement is a *finding* — a swing ordinary
  face-on and unusual from behind departed in the plane face-on cannot see. Anchors are reused from
  the alignment pass rather than recomputed, and the two implementations of "read three instants off
  a phase chain" are now pinned to each other.
- **Its own aspect handling.** ADR-012's `videos_160` distortion applies here too, and M8.1 showed
  it is worth ~7 points of explained variance when a model mixes axes.

### M8.3 — saying it (done, 2026-08-17)

Three models fitted, five placements on every swing, and **nothing said any of it to a golfer**.
That was M6.5's ordering working as designed — inspectable before spoken — but it had also left a
live gap: `contracts/caveats.py` named none of the five, so `mcp/query.py` shipped
`tour_trajectory_q_dtl: 11.06` as a bare float under a field description promising there was no
percentile. On three of four stored swings that number is a mis-detected down-the-line anchor, and
it is the largest figure on the swing.

This needed **a band or a policy**. It is a policy — a band would put a placement on the scoring
path, and there is nothing to cut one from, since every clip behind these models is a tour
professional and "far from the tour population" covers both the golfer doing something wrong and
the tour player with an unusual action. [ADR-022's third addendum](docs/decisions/022-learned-artifacts-as-committed-data.md)
states it in full.

- [x] **`contracts/placements.py`** — `POPULATION_PLACEMENT_REGISTRY`, `checkpoints.py`'s argument
  repeated: prose that has to name a set must derive it. `engine.py` takes each name and unit from
  it, `benchmarks/trajectory.py` takes its two view strings from it, and `caveats.py` builds two
  new bullets out of it — including the uncalibrated split, filtered on `PlacementSpec.calibrated`.
- [x] **The coaching brief renders them** (`feedback/coach.py`), which it never did — `build_brief`
  excluded `measurements` entirely. Calibration rides on the line carrying the value, not only in
  the caveat block, for the reason `_checkpoint_line` repeats `one_sided`.
- [x] **The MCP channel got its own shape**: `SwingView.population`, the one part of `measurements`
  that keeps its `detail`, because for a placement that string is not provenance but meaning.
- [x] **No score moved and `ANALYSIS_VERSION` did not bump.** Nothing about the engine's output
  changed meaning; re-analysing `2026-08-10/2` reproduced its stored `analysis.json` byte for byte.
  `feedback/rules.py` was deliberately left alone — a rule-based tip about a placement would be a
  verdict, and there is no band to earn one.

**Exit criteria**: a golfer hears what the population models found, with its uncertainty attached
and never as a fault — **met**.


---

# In progress

Shot ingestion and the MCP server both landed; tuning OCR on a real session's photos is the
remainder of M3. M6 joins them: Claude now writes the per-swing verdict, and what is left there
is the client handshake and conversational follow-up.

---

## M9: Player tracking, per-club shot history — in progress

**Design**: [ADR-024](docs/decisions/024-per-club-shot-history.md).
**Phase list**: [docs/M9_PLAYER_TRACKING.md](docs/M9_PLAYER_TRACKING.md) — 20 phases, each
independently commit-ready. **P1–P7 landed 2026-08-21; start at P8.** That is the whole ingest
spine: the vocabulary, the bag shape, the bag on disk, the club on `SwingManifest` and on a second
session cursor, the writer that stamps it, the 409 that refuses an untagged upload, and the one-tap
picker that satisfies it. A swing can no longer reach disk without a club. P8 opens the
measurements track, which is independent of everything above it.

**The gap, in one sentence.** This repo can say how a swing compares to a tour population and how
it compares to the golfer's own history. It cannot say how far you hit your 7 iron, because **no
shot on disk records which club hit it.**

**Why it is mostly wiring.** Career mode already built everything downstream of that field: the
corpus reader that counts an honest `n`, the baseline with its minimum-`n` guard, the bias/scatter
discriminator, and — the load-bearing one — `storage.corpus.narrow_to`, which filters a corpus
*and recomputes its metric counts*. Adding a `club=` clause to that filter makes the whole career
pipeline produce per-club answers with nothing new learning the rules. The statistics are written
and validated; what is missing is the tag.

**Why this is the next action rather than a bay session.** It is the one substantial item on this
board that needs neither a bay nor an `n`. And the ordering matters in one direction only: a
session hit *without* club tags produces data that can never be split by club afterwards. Tagging
is the cheapest thing here and the only one that is unrecoverable if skipped.

**What it delivers.** A bag page: every club with its average carry, its spread, its start-line
bias, and its loft — each with an honest `n` or an explicit refusal. Plus a declared bag carrying
per-club loft, which is the anchor club fitting will need and which cannot be reconstructed later.

**What it deliberately does not deliver**, all recorded in ADR-024's *Deferred*:

- **True landing offset.** The simulator prints no offline tile, so lateral miss ships as
  `start_line_offline_yds` — carry times the sine of the start line, i.e. where the ball *would*
  have landed if it never curved. Curve stays in degrees on `face_to_path_deg`. A real flight
  model is blocked on `spin_axis`, whose sign already stored two fades as draws
  ([ADR-014 addendum](docs/decisions/014-screen-capture-shot-ingestion.md)).
- **Club fitting.** The reason loft, ball speed and launch angle get recorded now. The models need
  data nobody has; the inputs are unrecoverable after the fact, so the inputs land and the models
  wait.
- **Per-club benchmark bands.** [ADR-010](docs/decisions/010-benchmark-ranges.md) already gated
  these and cut none — the club is not an axis this panel varies on, and
  [ADR-023](docs/decisions/023-tempo-training-and-absolute-swing-durations.md)'s addendum reached
  the same conclusion from the other direction.

**Two traps, written down because both look like oversights.** Do not wire the club into
`resolve_range` / `PracticeGoal.club` — `ranges.json` holds `club_category: "all"` rows only, so a
real category makes every checkpoint resolve no band and the panel goes dark. And do not give club
an `attribute_unlabeled` equivalent: reaching backwards over a session's untagged swings is safe
for golfer (usually one per session) and destructive for club (many per session).

**Expect refusals.** Every swing currently on disk is untagged, and the guard needs five shots per
club before it will state a mean. The correct output at every stage of M9 is a refusal with a
correct `n`; a number appearing early is the bug. Same acceptance criterion career mode shipped
under.

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
      — *and enumerate the bay screen's actual tile layout while you are there.* The profile was
      written from the two reference photos; the bay's screen shows `Impact Position V` where they
      show `Bounce & Roll`, so "no tile found" fires on every real shot and is telling the truth
- [x] **Sign conventions audited against a real screen** *(2026-08-14)* — `spin_axis` was stored
      inverted (HD Golf prints it signed, opposite the contract's `+ = fade`); both shots on disk
      were fades recorded as draws. Fixed as profile data (`printed_sign`), and cross-checked on
      every parse against the `Shot Type` tile, which is the only way to catch a flipped sign —
      the arithmetic identities cannot see one. Three false warnings retired with it
- [x] Build MCP server against a `ShotDataSource` (mock or screen — no hardware required)
      *(2026-08-10)* — `src/golf_coach/mcp/`, stdio transport, `scripts/run_mcp_server.py`
- [x] **Build MCP server with tools — and the tool list changed.** ADR-006's five were all
      shot-only, written in March before M4/M5/M7 existed. Shipped instead: `list_sessions`,
      `get_swing`, `get_session_summary`, `get_recent_shots`, `get_shot_by_id` — both axes of
      ADR-009's model, because the shot numbers are the *less* differentiated half (they are
      HD Golf's own readout, photographed) and a shot-only server would leave Claude unable to
      say where a swing sits against the tour population. See the ADR-006 2026-08-10 addendum
- [x] Write integration tests: 27 tests, and the query layer is pinned to import no MCP SDK
      (same extras boundary `api/pipeline.py` holds against fastapi)
- [x] `get_shot_trends` / `compare_sessions` *(2026-08-12, career mode step 6)* — deferred, then
      built without the sample size changing. The guard is what made them safe: a statistic the
      `n` cannot support is `None`, so the confident voice has nothing to say. A third tool,
      `get_golfer_profile`, carries the bias/scatter discriminator the original table had no
      place for
- [ ] Connect MCP server to analysis engine data merger
- [ ] *(optional, later)* Acquire the Garmin R10 and add its BLE adapter (ADR-004)

> **Status (2026-08-10):** the server is built and answers over stdio against real bay data —
> `get_swing("2026-08-10", "1")` returns 94.9 with all three checkpoints, their tour percentiles
> and the joined shot. It is a thin adapter: `mcp/query.py` does the reading and imports no SDK,
> `mcp/server.py` declares the tools and delegates. Anything the repo knows to be provisional is
> carried out rather than flattened — `needs_review` on an OCR'd shot, the alignment tier and its
> caveat, and `unscored` checkpoint names — because an LLM will otherwise present all of it as
> fact. The only M3 item left that needs no hardware is OCR preprocessing tuning.

**Exit Criteria**: After a shot, MCP server exposes complete shot metrics; analysis engine can query them.


---

## Milestone 6: LLM-Powered Coaching — done
**Goal**: Use Claude API to generate conversational, context-aware coaching advice.

- [x] **Design prompt template** — `feedback/coach.py`. The brief is *rendered*, not dumped:
      every value is labelled with the vocabulary the caveats warn about (`unscored`,
      `percentile`, `needs_review`, `alignment_caveat`), so a warning about `unscored` lands
      beside a line that says `unscored`. Keypoints and phases are excluded — several hundred
      frames of landmarks that no coach reasons from and that would dominate the prompt
- [x] **Implement the Claude API call in the feedback module** *(2026-08-11)* — `claude-opus-5`,
      adaptive thinking at `effort: low`, and it **never raises for an expected failure**: no key,
      no `llm` extra, a rate limit, a refusal, a truncation each return a reason that becomes a
      note on the result. Coaching is the last thing that happens to a swing and the least
      important; it must never be able to cost a golfer their score
- [x] **One source of truth for the caveats** — the standing warnings moved to
      `contracts/caveats.py`, composed by *both* `mcp/server.py` and `feedback/coach.py`. ADR-008
      forbids either importing the other, and the alternative was two copies of load-bearing prose
      drifting apart, which this repo has been bitten by three times
- [x] **Display LLM coaching alongside rule-based feedback in UI** — `results.html` renders it as
      a tinted card, never as a fourth `.tip`, with the model named *above* the prose. Gated on
      `CoachingProvenance` as well as text: unattributed prose on a page of measurements is
      indistinguishable from a measurement
- [x] **Add an API key and verify the live call** — done 2026-08-14 against `2026-08-10/2`.
      `claude-opus-5` wrote the verdict; `analysis.json` carries the model, the timestamp and a
      sha256 of the brief. It led on tempo, asserted nothing the brief did not contain, and named
      the two caveats that applied on its own. No spine angle, hip rotation, swing plane or club
      path. The key is a `SecretStr` read from `.env`
      ([ADR-019](docs/decisions/019-secret-handling.md))
- [x] **Register the MCP server with a real client and exercise the handshake** — done 2026-08-14.
      A stdio client completed `initialize` (protocol `2025-11-25`, capabilities negotiated, the
      5k-character briefing delivered), `list_tools` returned all **8** with their schemas, and
      `call_tool` served `list_sessions`, `get_swing`, `get_session_summary`, `get_recent_shots`
      and a deliberate miss — the `NotFound` shape survives the wire as a normal result rather
      than a protocol error, which is what it was designed to do. Registered with Claude Code via
      `claude mcp add`; `claude mcp list` reports `✔ Connected`
- [x] **Ask follow-up questions about a swing** *(2026-08-15, [ADR-020](docs/decisions/020-conversational-followups.md))* —
      the SDK tool runner over `mcp/query.py`'s functions directly, exactly as this line predicted;
      the stdio round trip stays for *external* clients. A conversation seeds from the swing's
      stored brief and looks everything else up through the same eight tools. Transcripts live in
      `data/processed/conversations/`, holding the model's own content blocks **verbatim** —
      thinking blocks are only replayable unchanged, and only into the model that produced them.
      Two entry points: `scripts/ask_swing.py` and a chat panel on the results page.
      **Proven live on 2026-08-10/2**: three turns, and the one that matters is the second —
      asked whether tempo was worse than last session, it reported the refusal ("the comparison
      tool withheld every per-metric mean… it needs 8 swings in a session") instead of averaging
      the per-session counts sitting beside it
- [ ] *Enable Claude to call MCP server tools for shot data context* — **reframed, not dropped.**
      The in-app call needs no tools: the pipeline already holds the whole `SwingBundleResult` in
      memory and hands it over directly. Tools become the right answer only for the follow-up
      case above

> **Status (2026-08-11):** a swing analyzed with a key configured carries `feedback.coaching_text`
> and `feedback.coaching` (model + timestamp + a sha256 of the brief, so a stored result can tell
> when the numbers moved underneath it). Found by running it: the first cut trimmed trailing zeros
> unconditionally and rendered a percentile of 90 as **9** and 10 as **1** — plausible, wrong, and
> aimed straight at a model that would have repeated it as fact. That also exposed a real
> inaccuracy in the caveat text itself, which claimed every failing checkpoint reports "about 90";
> a two-sided metric that misses *low* clamps to 10, as tempo does on `2026-08-10/2`. Both fixed,
> the second one for the MCP server too.

**Exit Criteria**: Claude provides specific, grounded coaching advice referencing actual swing data and shot metrics.

# Next — nothing blocking

Startable today. **M1.5 has now run** (2026-08-14) and closed as a no-go on pure-ML club
detection; it is kept in this section as the record of what it decided and what it left open.
The two below need a bay, not a decision.

---

## Milestone 1.5: Club-Head Detectability Spike (De-Risk Before Investing)
**Goal**: Before sinking time into labeling 200–500 images and training YOLOv8, prove the club head is even *detectable* in our footage — especially through the impact zone. This is a time-boxed investigation (a spike), not production code.
**Hardware to start**: None to begin (use phone/sample video). A fast-shutter capture test is more informative with the global-shutter camera + lighting, but the core visual check can start immediately.
**Why this exists**: Club-head tracking is hard precisely where it matters most (impact). At ~110 mph the head moves ~16 in *between frames* even at 120fps, and motion blur is governed by **exposure time, not shutter type** — a global shutter removes *distortion* (warping) but NOT blur. A sharp club head at impact needs a fast shutter **+ bright light**. We need to see real frames before committing.

> **Status (2026-08-14): done, and the answer is no-go.** It needed no new footage — the four bay
> clips already on disk carried the impact zone at 4K/60. Thresholds were committed before any
> frame was extracted. Findings:
> [spikes/club-head-detectability/log.md](spikes/club-head-detectability/log.md). Decision:
> [ADR-017](docs/decisions/017-club-head-detection-strategy.md).
>
> **The club head is a good detection target at rest and is destroyed by exposure, not by the
> detector.** 42 px across at 4K, crisp, behind a crisp ball — then a translucent 600–980 px band
> at impact, 14x to 23x its own size, present in the impact zone for about three frames of the
> sixty in that second and boundable in none of them. Pure-ML has nothing to label there, and a
> marker does not help because it raises *contrast* while the thing destroying the head is
> *exposure*. The requirement, consistent across both swings and the full plausible club-speed
> bracket, is **~1/2000 s** — about 30x the bay's current light.

- [x] Capture/collect a handful of swing clips that clearly include the **impact zone** — **not
      needed**: `data/raw/aaron-{1,2}` already held two swings × two views at 2160×3840/60fps,
      with impact frames the pipeline had already computed
- [x] Manual inspection: recognizable object or unlabelable smear? **Unlabelable band.** Head is
      **42 px** short-axis at rest; **zero** usable frames in the impact window, out of ~3 in
      which the club is in the zone at all
- [ ] Lighting/shutter test: does bright light + a forced fast shutter freeze the head? —
      **still open, and it is the only open question.** No fast-shutter clip exists, so 1/2000 s
      is a *specification derived from measurement*, not an observation. One clip settles it
- [x] Quick detectability probe — done by measurement plus direct inspection of native-resolution
      crops; `probe.py measure` prints the table and re-runs in a minute
- [x] Evaluate the fallback levers and pick a direction:
    - [x] **Pure ML** — **no-go at current capture.** Nothing to label through impact
    - [x] **Marker-assisted** — **no-go as a standalone fix.** Solves contrast; the problem is
          exposure. A marker at 1/60 s smears across the same 600–980 px
    - [x] **Fusion + interpolation** — **chosen** as the only path that produces a club path from
          what we can actually record. Explicitly a *modelled* path, and must be surfaced as one
- [x] Write findings into a short ADR — [ADR-017](docs/decisions/017-club-head-detection-strategy.md),
      plus addenda on [ADR-005](docs/decisions/005-object-detection-yolov8.md) (labelling deferred
      on evidence) and [ADR-003](docs/decisions/003-camera-hardware.md) (the number behind "global
      shutter ≠ no motion blur")

**Exit Criteria (go/no-go gate)**: ✅ met. (a) Camera-based club tracking is **not** viable at
current capture; (b) the strategy is fusion + interpolation until the capture changes; (c) the
lighting/shutter requirement is ~1/2000 s and ~30x present light. The no-go arrived before any
labeling effort began, which is what this spike existed to achieve.


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
- [x] **Tempo is untrustworthy on real footage and needs its own look.** *(fixed 2026-08-09 —
      and neither of the two fixes predicted here was the right one.)* The diagnosis above was
      wrong: `_motion_start` was not collapsing onto the top, it was doing its job on a **top that
      was ten frames late**. `_rising_runs` ends a descent when the drawdown exceeds a quarter of
      the run's *own accumulated* rise, which is near zero in a run's first frames — so a golfer
      hovering at the top produced a 0.002 wobble that split the descent, and the second fragment
      was taken as the top. The downswing measured 14 frames where the truth was 24, and tempo
      fell out at 0.43:1. Fixed by flooring the drawdown test at `phases._DRAWDOWN_FLOOR` (0.012
      of the clip's own wrist range); tempo on `aaron-1` now reads 2.42:1. Validated paired
      per-clip against the 461-clip GolfDB corpus: impact unmoved (0 clips change), top median
      unchanged with 12 better against 12 worse, address mean 22.9 → 22.8. The same fix removed
      two symptoms nobody had connected to it — the down-the-line panel replaying at 1.69x and the
      two panels starting a second out of step — because both came from the two views disagreeing
      about the downswing length
- [ ] **The down-the-line motion start reads late on real footage.** With the top fixed, the two
      views agree exactly on the downswing (24 frames each) but still disagree on *tempo* (2.42
      face-on against 1.50 down-the-line), because DTL's `motion_start` lands ~22 frames late. So
      alignment stays at the `top_impact` tier rather than `full`. Harmless today — the fallback
      anchor is symmetric and derived from downswings that now agree, so the render is correct
      either way — but it is the last soft-anchor weakness on real bay footage. Note that GolfDB
      is a **face-on** corpus, so `tune_address.py` cannot measure a DTL-specific rule; this needs
      down-the-line ground truth before anyone re-tunes for it
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



---

# Gated on hardware

Needs a purchase, or needs hardware in hand to re-check a provisional choice.

---

## Milestone 2: Club & Ball Detection
**Goal**: Detect and track club head and ball through the swing using a fine-tuned model.
**Hardware to start**: **lighting, not a camera** — M1.5 measured the requirement at ~1/2000 s, which is 5 stops below the bay's present exposure and needs roughly 4–10x more light *on the ball* (less than ADR-017's "30x", which held ISO constant). A global-shutter camera in the current light still records a smear (ADR-003's 2026-08-14 addendum). The light chosen and why: [ADR-018](docs/decisions/018-bay-lighting.md). Scaffolding and labeling workflow can be built earlier on sample frames.
**Why this exists**: MediaPipe tracks the *body* only (it has no concept of a club). YOLOv8 is what detects the club head + ball, and feeding its detections through a tracker (ByteTrack) produces the visual **club-path arc** — the swing path overlaid on the replay. This is also the one model we train ourselves (ADR-005).
**Gated on M1.5 — which has now reported, and the answer reshapes this milestone.** The spike ran 2026-08-14 and returned **no-go on pure ML and no-go on marker-assisted**, because the club head is destroyed by exposure time rather than by anything a detector or a marker addresses ([ADR-017](docs/decisions/017-club-head-detection-strategy.md)). **Do not start the labeling effort below.** The tasks as written assume the pure-ML path; the chosen interim direction is fusion + interpolation, which needs none of them and produces a *modelled* club path that must be surfaced as modelled. The labelling tasks become startable again only if a fast-shutter capture test passes — one clip settles it.

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
      committable, per-clip keypoints of named tour players are not. **M8 found the way round
      this**: a *basis* fitted over 122 pros is an aggregate, and a basis is not a pro
- [ ] Drill recommendations based on persistent faults — **one of them shipped** (2026-08-20,
      [ADR-023](docs/decisions/023-tempo-training-and-absolute-swing-durations.md)): the tempo
      trainer, a metronome built from tour absolute durations and played on the results page when
      the tempo checkpoint fails. Not "persistent" yet — it reads one swing, and a fault is only
      persistent against a `PersonalBaseline`, which is still gated on `n`. Two things it added
      that the rest of this row can build on: `backswing_ms` / `downswing_ms` now reach
      `measurements` (so *which half* is off becomes answerable once `n` exists), and the requested
      mph-indexed version was measured and refused — club moves tour downswing duration by 6.9 ms
      against 47.0 ms between golfers
- [x] ~~Trained ML model for swing quality regression (replace/augment rules)~~ — **attempted and
      redirected**, see [§M8](#m8-learning-what-good-means--gates-run-model-fitted). Regression
      against *outcome* is closed (ADR-021: face-on pose does not predict ball flight). What shipped
      instead is a normative model of the tour population's joint distribution (ADR-022)
- [ ] Mobile companion app
- [ ] Export swing reports as PDF
