# M7: Two-Phone Sim Capture — multi-angle ingestion, alignment, and a host

> **Tier: TARGET.** The live plan — most of it is built, but this document describes the intended
> design and its planning prompts are historical. Verify against code before trusting a detail.

**Status**: 2026-08-09 — **six of seven phases built** (3 trimmed). Only the Phase 0 field spike
is unstarted; it needs a bay session, not code. Per-phase detail below records what actually
shipped where it diverged from the plan.
**Decisions**: [ADR-011](decisions/011-camera-synchronization.md) + its 2026-08-05 addendum
(this is a second capture tier, not a replacement), [ADR-014](decisions/014-screen-capture-shot-ingestion.md)
(shot ingestion, offline-first stance), [ADR-003](decisions/003-camera-hardware.md) addendum 2026-07-02b
(face-on vs down-the-line stream assignment). [ADR-015](decisions/015-handheld-two-phone-capture-and-event-anchored-alignment.md)
and [ADR-016](decisions/016-local-first-host-and-phone-upload-topology.md) were written during
phases 2 and 6, as planned, and are now Accepted.
**Taking it to a bay**: [BAY_SESSION_RUNBOOK.md](BAY_SESSION_RUNBOOK.md).

## How to use this doc

M7 is a **ladder of seven phases**. Each one is independently valuable, ends in a single commit, and
depends only on the phases before it. **Six are built — start at [the ladder](#the-ladder) to see
which.**

Each phase was planned in a fresh session using a self-contained prompt at the bottom of this doc.
That was deliberate: these phases span capture internals, a pure-functional alignment algorithm, a
storage state machine, and a web server, and plan quality drops noticeably when one context is
stretched across all of them.

> ⚠️ **The prompts are now historical, except Phase 0's.** They describe what was believed before
> each phase was built, and several phases deliberately diverged from their own prompt. **Phase 6's
> prompt is actively wrong** and carries a banner saying so. Check the ladder before copying
> anything from that section — the per-phase sections above it are the record of what shipped.

Prefix the Phase 0 prompt with *"Read docs/M7_TWO_PHONE_CAPTURE.md for project context, then plan
Phase 0."* The standalone context inside it is belt-and-braces for anyone working without this doc.

## The ask

At an indoor golf simulator: one person records **face-on** with an iPhone, a second records
**down-the-line** with another iPhone, someone photographs the HD Golf `SHOT DATA` screen, all three
reach the program, and it reports on the swing.

## The problem: this is not supported today

The investigation that produced this doc asked five questions. Everything *downstream* of "a video
file is already sitting on the desktop" works, and works well. Everything *upstream* is missing.

| Question | What the repo actually has |
|---|---|
| How are the two videos synced? | **No sync of any kind.** `camera_id` and `FrameBundle` are prescribed by ADR-011's Architecture section but exist **only in the doc** — grep finds them nowhere in `src/`. `analyze_swing()` takes exactly one keypoints list (`analysis/engine.py:30-37`). No structure anywhere holds two views of one swing. |
| Where do the phone videos get sent? | **Nowhere.** The only ingestion path is a positional CLI argument → `cv2.VideoCapture` (`scripts/run_pose.py:28-38`). No upload, no watcher, no queue. |
| How does the program know to "wait" for a video? | **It doesn't.** There is no session or swing registry. The filename stem is the entire notion of swing identity (`scripts/analyze_swing.py:164`). `src/golf_coach/storage/` is a 4-line docstring. |
| What is hosting the program? | **Nothing.** `src/golf_coach/api/__init__.py` is a 6-line docstring — no FastAPI app, no routes. The `api` extra (fastapi, uvicorn) is declared in `pyproject.toml` and entirely unused. |
| Does the desktop compute the data? | Yes, and that is the right call — MediaPipe lite and PaddleOCR are both CPU-friendly and run comfortably faster than the session. **But** `run_pose.py:36` did `frames = list(source.frames())`, materializing every decoded BGR frame: a 10-second 4K60 iPhone clip is 600 frames × ~24.9 MB ≈ **15 GB**. It would have OOM'd on the first real phone video. *Fixed in Phase 1.* |

Roughly 60% of the work is already done. The remaining 40% is ingestion, identity, and alignment —
which is the harder 40%.

### What is reused unchanged

Pose extraction (`pose/estimator.py`, MediaPipe Tasks), phase segmentation (`analysis/phases.py` —
top median 2 frames, impact median 1 frame across 461 GolfDB clips), the three face-on checkpoints
(`analysis/checkpoints/mechanics.py`), scoring, `feedback/rules.py`, and the whole HD Golf OCR
pipeline (`launch_monitor/screen/`, ADR-014). No phase below rewrites any of it.

## Confirmed decisions

- **Host**: the desktop stays at home; the two phones reach it over **Tailscale**. Not cloud — the
  clips are large, and ADR-014 already established the offline-first stance for exactly this reason.
- **Down-the-line scope**: **capture and align only.** Scoring stays on the three validated face-on
  checkpoints. No new DTL metrics (spine tilt, swing plane) in M7 — the GolfDB corpus the benchmark
  bands were derived from is face-on, so DTL metrics have no reference population to be judged against.
- **Launch monitor**: HD Golf. The existing `hd_golf` profile in
  `launch_monitor/screen/profiles.json` applies as-is.

## Out of scope, permanently, for this capture setup

**3D triangulation — true spine angle, hip rotation, X-factor, kinematic sequence** (ADR-011
Phases 2–3). Triangulation requires cameras that are *fixed* with known intrinsics **and** extrinsics.
Two hand-held iPhones, placed differently on every swing by two different people, cannot be
calibrated — so this is not a "later" item for M7, it is unreachable by construction.

This is the single most likely thing to be re-litigated by someone who reads ADR-011 and concludes
that fusion is the sanctioned path to multi-camera work. **ADR-011 remains correct** — for the fixed
ELP AR0234 rig it was written about. M7 is a *different capture tier* that trades 3D away for
zero-setup portability, and the two coexist. See the 2026-08-05 addendum on ADR-011.

## The ladder

| # | Phase | Ships | Done |
|---|---|---|---|
| D | Land this plan in the repo | This doc + roadmap / ADR / worklog updates | [x] |
| 0 | Field spike | `docs/M7_TWO_PHONE_SPIKE.md`. Gate: decides scope of 1 and 2. | [ ] |
| 1 | Capture layer survives phone footage | Streaming pose, `camera_id`, clip metadata w/ fps | [x] |
| 2 | **Video sync / alignment engine** | `analysis/alignment.py` + side-by-side CLI | [x] |
| 3 | Session & swing bundle store | `storage/` manifest + arrival state machine | [x] **trimmed** |
| 4 | Bundle analysis + launch-monitor join | `analyze_swing_bundle()`, `SwingResult.shot` populated | [x] |
| 5 | Local server + phone upload page | FastAPI on localhost, browser upload, worker | [x] |
| 6 | Tailscale exposure | Phone-to-desktop over tailnet, on-site validation | [x] *(not as specified — see below)* |

Phases 3–6 landed out of order: 6 shipped before 4 and 5 completed. On-site validation is still
outstanding on all of it — that is the bay session, and Phase 0's footage comes from the same trip.
See [BAY_SESSION_RUNBOOK.md](BAY_SESSION_RUNBOOK.md).

---

### Phase 0 — Field spike (no production code)

**Goal**: answer three questions with real footage before designing anything around assumptions.

1. **Does `segment_phases()` find sane top/impact on a down-the-line clip?** It reads `LEFT_WRIST` y
   (`analysis/phases.py:71`) and was tuned on 461 **face-on** clips. From down-the-line, for a
   right-handed golfer, the lead wrist is the *far* arm and is occluded by the torso through parts of
   the swing. **This is the single biggest risk in M7** — Phase 2's whole alignment design rests on
   both clips producing usable anchors.
2. **Does OpenCV decode the iPhone `.mov`?** iPhones write HEVC/h.265 by default and OpenCV's bundled
   FFmpeg is not guaranteed to handle it.
3. **What does `CAP_PROP_FPS` report, normal vs slo-mo?** Slo-mo stores 120/240fps capture with a
   stretched playback rate, so the reported fps may not describe real time.

**Gate**: if (1) fails → Phase 2 gains a manual impact-frame nudge fallback. If (2) fails → Phase 5
gains an ffmpeg transcode step on upload.

---

### Phase 1 — Capture layer survives phone footage

**Built 2026-08-07.** Real phone clips run through pose without OOM, and every clip records which
camera it came from plus its fps.

- **The `list(source.frames())` OOM is fixed** — both in `scripts/run_pose.py` and in
  `scripts/analyze_swing.py`'s `--overlay` path, which had the identical bug and is the command
  most likely to be pointed at phone footage. Pose and overlay are two streaming passes over the
  file; the overlay needs the pixels a second time and a `VideoSource` is one-shot, so it simply
  decodes twice (a few percent of the run — decode is ~675 fps against MediaPipe's ~106).
  Measured on the 480×854 sample: **peak RSS 971 MB → 233 MB**, and the new figure no longer
  scales with clip length or resolution.
- **`camera_id`** on `Frame` (`capture/source.py`) and carried onto `FrameKeypoints` — ADR-011's
  Phase 1 seam, built at last. Optional, defaulting to `None`, which is what every clip recorded
  before today honestly is. `run_pose.py --camera-id face_on`.
- **Clip metadata** (`fps`, `width`, `height`, `frame_count`, `source_sha256`) now rides in the
  keypoints JSON, which grew an envelope: `{"clip": {...}, "frames": [...]}`.
  `storage/keypoints_io.py` reads that *and* the legacy bare array, so nothing was migrated —
  the 461-clip GolfDB cache loads untouched and reports `clip=None`, which is true rather than a
  gap. `frame_count` records what was actually **decoded**, and a decode that comes up short of
  the container's claim now warns instead of passing silently (spike Q2's PARTIAL case).

**The pre-inference downscale was deliberately dropped**, and it is the one item here that did not
ship. The argument for it — "MediaPipe resizes to its own input internally, so 4K buys zero
accuracy" — is sound about the *ceiling* but skips a step: downscaling first makes it a two-step
resample (`INTER_AREA` → MediaPipe's own resize) rather than one, and the difference between those
has never been measured in this repo. It is a throughput optimisation, not a correctness fix, and
the streaming change is what actually makes 4K survivable. Revisit once Phase 0 footage exists and
there is a real decode-throughput number to optimise against; the 480×854 sample would have been
resized by a 720px cap for almost no saving while perturbing the pinned 400/423 baseline the spike's
pre-flight asserts against.

**Verified** by re-running pose on the committed sample and diffing: all 656 × 33 × 4 landmark
values **bit-identical** to the previous output, `TOP @ 400 / IMPACT @ 423` unmoved, and the
spike's pre-flight still exits 0. Dropping the downscale is what makes that an exact identity
rather than a judgement call.

---

### Phase 2 — Video sync / alignment engine

**Built 2026-08-07.** Two keypoints files of one swing go in; a frame correspondence and a
side-by-side MP4 come out, with no shared clock anywhere in the path. Recorded as **ADR-015**.

**The design is as specified**: event-anchored piecewise-linear time warp on a normalized swing-time
axis `τ` (0 = motion start, 1 = top, 2 = impact, extrapolated past impact at the downswing rate),
ADR-011's Option C used standalone. Anchor reliability from the repo's own bakeoff
(`docs/M4_POSE_BAKEOFF.md`, ADR-013) is what splits the anchors into primary and soft:

| Anchor | Error | Use |
|---|---|---|
| `impact` | median 1 frame | primary |
| `top` | median 2 frames | primary |
| `motion_start` | median 7 frames; 40% of clips >10 frames off; falls back entirely on ~14% | soft — only when **both** clips detected it *and* their tempo ratios agree |

- **`contracts/alignment.py`** — `SwingAnchors`, `ClipAlignment`, `SwingAlignment`, and
  `AlignmentQuality` (`full` / `top_impact` / `impact_only` / `unaligned`), each with a
  human-readable summary so a UI can say *"aligned on impact only"* rather than rendering two panels
  and letting the viewer infer precision nobody measured.
- **`analysis/alignment.py`** — pure, stdlib + pydantic. Anchor extraction, `frame_of_tau` /
  `tau_of_frame` (exact inverses), `align_swings`, `map_frame`.
- **`scripts/align_swings.py`** — the text report needs no video at all; `--out` renders the
  side-by-side. Both clips **stream**, since the warp is monotone; buffering two 4K clips would have
  undone Phase 1's whole point.

**Two things were added that the phase plan did not anticipate**, both from meeting real footage:

1. **Multi-swing clip selection.** Real phone clips contain practice swings, and `segment_phases`
   takes the *earliest* major descent — so it will pick a practice swing if one comes first. That is
   correct behaviour for the single-swing GolfDB corpus it was validated on and cannot be tuned
   away. `phases.candidate_downswings()` (a pure addition; `segment_phases` is untouched) lists every
   descent, `--list-swings` prints them, and `--window START:END` selects one. Worth knowing: **N
   swings produce about N+1 descents**, because the hands coming back down to address between swings
   is a descent like any other.
2. **A tempo cross-check.** Frame rate cancels out of a backswing:downswing ratio, so two views of
   one swing must agree on it. When they disagree the alignment refuses the soft anchor and says
   why — this is the guard against the two clips locking onto *different* swings, which is the one
   failure mode that produces a confident, plausible, completely wrong video.

**Separable because** it is purely offline and pure-functional: two JSON files in, an alignment and an
MP4 out. No server, no storage, no upload.

---

### Phase 3 — Session & swing bundle store

**Goal**: three files arriving at different times get grouped into one swing, and the program knows
when a swing is complete — without ever blocking.

An **arrival-driven state machine** over a per-swing manifest: `pending` → `ready` (all expected roles
present, analysis fires) → `analyzed`. A staleness timeout flips `pending` → `partial`. A face-on-only
swing still scores all three existing checkpoints, so `partial` is a useful result, not an error state.

**Swing identity is assigned by the store, not the uploader.** Two people holding two phones cannot be
trusted to type matching swing numbers. Each upload declares only its **role** (`face_on` /
`down_the_line` / `shot_screen`); the store slots it into the newest swing in the session lacking that
role, opening a new swing if none does. Content-addressed dedupe — reusing the `hash_image` pattern
from `launch_monitor/screen/store.py` — so a double-tapped upload does not create a phantom swing.

**Separable because** it is CLI-driven with no HTTP. The server in Phase 5 calls into it.

---

### Phase 4 — Bundle analysis + launch-monitor join — **built 2026-08-08**

**Goal**: one command takes an assembled swing bundle and produces the complete result — score,
checkpoints, ranked tips, aligned side-by-side video, and the HD Golf numbers attached.

**Built as specified, plus one thing the spec did not anticipate.** `analyze_swing_bundle()` and
`scripts/analyze_bundle.py` are as described below; `SwingBundleResult` (new, in
`contracts/swing.py`) serializes to `analysis.json` beside an `aligned.mp4` and the per-view
keypoints. The shot join is **cache-first on the photo's sha256**, which the manifest already
records and `ShotStore` is already keyed on — so a shot imported earlier attaches with no `ocr`
extra installed at all, and OCR runs only on a genuinely new photo.

The addition is **`phases.select_swing()`**. The plan treated picking the swing out of a
multi-swing clip as a Phase 2 concern already solved by `--window`. It is not, because the window
does not merely frame the video — *it decides which frames get scored*. Unaided, `segment_phases`
picks a setup move on all four real bay clips; scoring `aaron-1-front` whole-clip gives 58/100
with tempo unscored and finish balance a 0.53 MISS, and scoring the actual swing gives 67/100
with both in band. So selection had to become automatic and pure. The rule, and the evidence
behind every number in it, is documented on `_PLAUSIBLE_DOWNSWING_S` and `_WINDOW_LEAD`.

**Not built, and now a named follow-up:** the face-on tempo checkpoint is untrustworthy on real
footage — see the ROADMAP item under this milestone. Phase 4 surfaces the contradiction and
changes no scoring.

`analyze_swing_bundle()` scores the **face-on** view via the existing `analyze_swing()` unchanged (no
regression risk to the validated three checkpoints) and segments the down-the-line view for alignment
anchors only. It populates `SwingResult.shot` (`contracts/swing.py:129` — the field exists and nothing
has ever set it).

**Deliberately excluded**: outcome checkpoints. `engine.py:70-71` hardcodes `outcome = []`, and scoring
shot data needs per-club benchmark bands that `ranges.json` does not have — it holds three `(all, all)`
rows. M7 **attaches and displays** the launch monitor numbers; scoring them is M4 proper, per ADR-009.

**Separable because** it is the last piece that runs entirely offline. After Phase 4 the full use case
works end to end via CLI; the remaining phases only change *how the files arrive*.

---

### Phase 5 — Local server + phone upload page

**Goal**: upload the three files from a phone browser and get a results page back. **Localhost only** —
no tunneling in this phase.

Fills the `src/golf_coach/api/` seam. A static upload page with a role picker sticky in `localStorage`
(so each phone is configured once) and a plain `<input type="file" accept="video/*,image/*">` — iOS
Safari handles this natively, so there is no app, no App Store, and no signing. `POST /api/uploads`
streams the body to disk rather than buffering a 200 MB clip in memory, then hands off to Phase 3's
store and returns the assigned `swing_id`. An in-process asyncio worker at concurrency 1 is sufficient
— one swing per couple of minutes does not justify Celery or Redis.

**Separable because** proving upload → assemble → analyze → display works locally is independent of
proving the network path works. Debugging both at once is the trap.

---

### Phase 6 — Tailscale exposure

**Goal**: both phones at the sim reach the desktop at home — including a phone that isn't mine.

**Built (2026-08-06), and it diverges from what this section originally specified.** The original plan
was to bind uvicorn to the desktop's Tailscale IP (`100.x.y.z`) over plain HTTP. Two things changed it:

1. **The bind never widens at all.** `tailscale serve --bg 3000` terminates real Let's Encrypt TLS at
   `https://<machine>.<tailnet>.ts.net` and proxies to `127.0.0.1:3000`. uvicorn stays on loopback,
   which is strictly safer than binding the tailnet IP, and HTTPS gives a secure context — so a future
   page can use `getUserMedia` instead of the camera roll.
2. **A helper's phone can't join the tailnet.** This is a two-person capture; the down-the-line phone
   often belongs to whoever is at the bay. `tailscale funnel --bg 3000` publishes the same URL
   publicly, on demand, no app required — and that makes tailnet membership insufficient as the only
   access control.

So `/api/` routes are gated on `GOLF_UPLOAD_TOKEN`, enforced only when set, accepted as an
`X-Upload-Token` header or a `?t=` query param. A phone is set up once by opening `/?t=<token>`; the
page stores it beside the role and strips it from the URL. `scripts/run_server.py` refuses to start on
a non-loopback `--host` with no token set. See **ADR-016**.

Cloudflare Tunnel was considered and rejected: the free and Pro plans hard-cap request bodies at
100 MB with an edge-side HTTP 413, which would need client-side chunking to work around.

Upload time over the sim's uplink is the dominant latency in the whole system — not compute. Record
1080p60, not 4K. Funnel is relayed and rate-limited, so guest-phone uploads are slower than tailnet
ones; turn Funnel off when the session ends.

Also fixed here: the default port moved 8080 → 3000, because Windows reserves 8069–8168 on this
machine and 8080 could never bind (`WinError 10013`).

**Separable because** it is a networking and security concern touching config and docs, not application
logic, and it is verified independently (phone on cellular with WiFi off) before a bay session depends
on it.

---

## New ADRs

Written *during* the phase that implements them, not up front. Both are Accepted:

- **[ADR-015](decisions/015-handheld-two-phone-capture-and-event-anchored-alignment.md) — Handheld
  two-phone capture & event-anchored alignment** (Phase 2). Records that for
  hand-held phones, alignment is event-anchored 2D and 3D fusion is unreachable. Does **not** supersede
  ADR-011; the two coexist as different capture tiers.
- **[ADR-016](decisions/016-local-first-host-and-phone-upload-topology.md) — Local-first host &
  phone upload topology** (Phase 6). A **loopback-bound** FastAPI with Tailscale Serve/Funnel in
  front of it — the bind never widens — plus a `GOLF_UPLOAD_TOKEN` gate on `/api/`, because a
  helper's phone can't join the tailnet. Browser upload, no phone app, no cloud. Consistent with
  ADR-014's offline-first reasoning.

---

## Planning prompts

> ⚠️ **Only the Phase 0 prompt is live. Phases 1–6 are built — their prompts are kept as the
> record of what was believed before each phase, not as instructions.** They are preserved rather
> than deleted because several phases diverged from their own prompt for reasons worth keeping
> (see the per-phase sections above), but a cold session that copies one will plan work that is
> already done, against constraints that no longer hold. **Check the ladder table before copying
> anything from here.**

Copy the matching block into a fresh session.

### Phase 0

```
Repo: C:\Users\aaron\Desktop\repo\golf-coach (Python 3.11, ADR-driven home-lab golf swing
analysis; read README.md and docs/decisions/ first).

I'm building toward: two people record one golf swing at an indoor sim on two iPhones (one
face-on, one down-the-line), plus a photo of the HD Golf SHOT DATA screen, all analyzed together.
This is a de-risking spike BEFORE any implementation. Produce no production code.

Plan a field spike that answers three questions using real footage I'll record:

1. Does the existing phase detector work on DOWN-THE-LINE video? `segment_phases()` in
   src/golf_coach/analysis/phases.py reads LEFT_WRIST y-position and was tuned on 461 FACE-ON
   GolfDB clips (top median 2 frames, impact median 1 frame). From down-the-line the lead wrist is
   the far arm and is occluded by the torso. This is the biggest risk in the whole project.
2. Can OpenCV decode iPhone .mov files? iPhones default to HEVC/h.265.
3. What does cv2.CAP_PROP_FPS report for a normal clip vs a slo-mo clip? Slo-mo stores 120/240fps
   with a stretched playback rate, so reported fps may not describe real time.

The plan should cover: exactly what to record and how (angles, framing, phone settings, how many
swings, whether to record matched normal/slo-mo pairs), what commands to run against the footage
using the existing scripts/run_pose.py and scripts/analyze_swing.py, how to judge pass/fail on
each question objectively rather than by vibes, and a throwaway probe script if one helps.

Deliverable is a findings doc at docs/M7_TWO_PHONE_SPIKE.md written in the style of the existing
docs/M4_*.md files. State what each outcome implies for the design.
```

### Phase 1

```
Repo: C:\Users\aaron\Desktop\repo\golf-coach (Python 3.11, ADR-driven; read README.md,
docs/decisions/008-project-structure.md and docs/decisions/011-camera-synchronization.md first).

Goal: make the capture + pose layer handle real iPhone footage and record which camera each clip
came from. This is one commit, standalone. Read docs/M7_TWO_PHONE_SPIKE.md for field findings.

Four changes:

1. FIX AN OOM BUG. scripts/run_pose.py:36 does `frames = list(source.frames())`, materializing
   every decoded BGR frame. A 10-second 4K60 iPhone clip is 600 frames x ~24.9 MB = ~15 GB. It has
   only survived because the committed sample is small. Stream instead. Note the overlay writer
   needs the frames a second time — a second FileVideoSource pass is probably simplest, but weigh it.
2. Downscale to ~720px long edge before MediaPipe inference in src/golf_coach/pose/estimator.py.
   MediaPipe resizes to its own input internally, so 4K buys zero accuracy and costs decode time.
   Landmarks are normalized [0,1] so nothing downstream should change — verify that claim.
3. Add `camera_id: str` to the `Frame` dataclass in src/golf_coach/capture/source.py and carry it
   onto FrameKeypoints. ADR-011's Architecture section prescribes exactly this and it was never done.
4. Persist clip metadata (fps, width, height, frame_count, source_sha256) in the keypoints JSON.
   fps is currently never persisted anywhere — it lives only inside FileVideoSource — and the next
   phase needs it.

Hard constraints:
- Both new fields must be optional with defaults. The existing data/processed/*.keypoints.json AND
  the 461-clip GolfDB reference cache under data/reference/golfdb/keypoints/ must still load. This
  is the main regression risk — check how contracts/reference.py and the benchmark path read them.
- `.venv/Scripts/python.exe -m pytest -q` must stay green.
- Match the existing code style: pure functional core, ports/adapters, heavy deps behind extras.
```

### Phase 2

```
Repo: C:\Users\aaron\Desktop\repo\golf-coach (Python 3.11, ADR-driven; read README.md,
docs/decisions/011-camera-synchronization.md and src/golf_coach/analysis/phases.py first).

Goal: given two pose-keypoint files of the SAME golf swing recorded from different angles by two
un-synchronized hand-held iPhones, determine which frame in each corresponds to the same swing
instant — and prove it with a side-by-side video. One commit, fully offline.

THE APPROACH IS ALREADY DECIDED — plan how to build it well, not whether:

Event-anchored piecewise-linear time warp. Run the existing `segment_phases()`
(src/golf_coach/analysis/phases.py) on each clip INDEPENDENTLY; each yields motion_start, top and
impact frame indices. Define a normalized swing-time axis tau: 0 = motion start, 1 = top,
2 = impact, extrapolated linearly past impact at the downswing rate. Map tau -> frame index per
clip by piecewise-linear interpolation. This is ADR-011's Option C used standalone.

Why this and not timestamps: it needs no shared clock, no clapper, no calibration, and is immune to
different frame rates, different clip lengths, different start/stop moments, and iPhone slo-mo
(which stores 120/240fps with a stretched playback rate). Two phones will not be configured
identically. 3D triangulation is permanently off the table for hand-held phones — no calibration
is possible — so alignment is only ever for per-view 2D metrics and side-by-side viewing.

Anchor reliability, from the repo's own bakeoff (docs/M4_POSE_BAKEOFF.md, ADR-013):
- top: median 2 frames error. impact: median 1 frame. Use these as primary anchors.
- motion_start: median 7 frames, 40% of clips >10 frames off, falls back entirely on ~14% of clips.
  Use only as a soft anchor when PhaseSegment.detected is True.

Plan should cover:
- A new pure-functional src/golf_coach/analysis/alignment.py: anchor extraction from segment_phases
  output, warp construction, tau<->frame mapping both directions, and an alignment-quality value so
  the UI can say "aligned on impact only" rather than implying more precision than exists.
- A CLI that takes two keypoints files and writes a side-by-side MP4. Reuse draw_skeleton() and
  annotate_frame() from src/golf_coach/pose/overlay.py unchanged. The visible proof of correctness
  is that the ADDRESS/TOP/IMPACT banners land on both panels simultaneously by construction.
- Tests mirroring tests/analysis/test_phases.py: synthetic anchor pairs at 60 vs 240 fps, a clip
  with motion_start.detected=False, reversed clip ordering, degenerate/too-short clips.
- Draft ADR-015 recording this decision. It does NOT supersede ADR-011 — that stays correct for the
  fixed calibrated ELP rig. These are two capture tiers that coexist. Match the existing ADR style.

Check docs/M7_TWO_PHONE_SPIKE.md first: if down-the-line anchor detection proved unreliable there,
include a manual impact-frame nudge as a fallback path.
```

### Phase 3

```
Repo: C:\Users\aaron\Desktop\repo\golf-coach (Python 3.11, ADR-driven; read README.md and
src/golf_coach/launch_monitor/screen/store.py first — its conventions are the model to follow).

Goal: three files arriving at different times (face-on video, down-the-line video, launch-monitor
screen photo) get grouped into one swing, and the program knows when a swing is complete. One
commit, CLI-driven, NO HTTP — the server comes later and will call this.

Design constraints, already settled:

- The program never blocks waiting. It's an arrival-driven state machine over a per-swing manifest:
  pending -> ready (all expected roles present, analysis fires) -> analyzed. A staleness timeout
  (default ~10 min) flips pending -> partial. A face-on-only swing still scores all three existing
  checkpoints, so `partial` is a useful result, not an error state.
- SWING IDENTITY IS ASSIGNED BY THE STORE, NOT THE UPLOADER. Two people holding two phones cannot be
  trusted to type matching swing numbers. Each upload declares only its ROLE (face_on /
  down_the_line / shot_screen); the store slots it into the newest swing in the session lacking that
  role, opening a new swing if none does. Think hard about the failure modes here — someone
  re-records a swing, someone uploads out of order, one phone dies mid-session — and design the
  repair path.
- Content-addressed dedupe so a double-tapped upload doesn't create a phantom swing. Reuse the
  hash_image / ShotStore pattern rather than inventing a second convention.

Proposed layout (challenge it if there's better):
  data/processed/sessions/<session_id>/<swing_id>/
      manifest.json, face_on.mov, down_the_line.mov, *.keypoints.json, shot.json, result.json

This is the first real code in src/golf_coach/storage/ (currently a 4-line docstring). Note
config.py has a db_path pointing at SQLite that nothing writes — decide explicitly whether this is
flat files or SQLite and justify it; flat files match how everything else in the repo works.

Include tests for arrival ordering (each role first), duplicate upload, and timeout -> partial.
Keep it dependency-free so it imports on the base install (ADR-008).
```

### Phase 4

```
Repo: C:\Users\aaron\Desktop\repo\golf-coach (Python 3.11, ADR-driven; read README.md,
src/golf_coach/analysis/engine.py and docs/decisions/009-swing-scoring-model.md first).

Goal: one command takes an assembled swing bundle (two videos + a shot photo, grouped by the
storage layer) and produces the complete result — score, checkpoints, ranked tips, aligned
side-by-side video, and the HD Golf numbers attached. After this the full use case works end to
end via CLI; the remaining phases only change how files arrive. One commit.

Scope:
- `analyze_swing_bundle()` alongside the existing `analyze_swing()` (analysis/engine.py:30). It
  scores the FACE-ON view via the existing analyze_swing() UNCHANGED — no regression risk to the
  validated three checkpoints — and segments the down-the-line view for alignment anchors only.
- Populate SwingResult.shot (contracts/swing.py:129). The field exists and nothing has ever set it.
  Wire in the OCR path by reusing _import_one from scripts/import_shot_screens.py:104 rather than
  duplicating it.
- Serialize the result. Note analyze_swing.py currently never serializes SwingResult at all — it
  only prints. The results page in the next phase needs JSON.
- Emit the aligned side-by-side MP4 using the Phase 2 alignment module.

DELIBERATELY EXCLUDED — do not add these:
- Outcome checkpoints. engine.py:70-71 hardcodes `outcome = []`. Scoring shot data needs per-club
  benchmark bands that ranges.json does not have (it holds three (all, all) rows). v1 ATTACHES AND
  DISPLAYS the launch monitor numbers; scoring them is M4 proper per ADR-009.
- New down-the-line checkpoints (spine tilt, swing plane). The GolfDB corpus the benchmark ranges
  were derived from is face-on, so DTL metrics have no reference data. Out of scope by decision.

Surface the shot's parse_confidence and needs_review flag in the output — ADR-014 says consumers
must handle low-confidence shots, and a silently-wrong OCR number is the failure mode it was
designed against.
```

### Phase 5

```
Repo: C:\Users\aaron\Desktop\repo\golf-coach (Python 3.11, ADR-driven; read README.md,
docs/decisions/007-decouple-software-hardware.md and docs/decisions/008-project-structure.md first).

Goal: upload the three files from a phone browser and get a results page back. LOCALHOST ONLY —
no tunneling, no Tailscale in this phase. Proving upload -> assemble -> analyze -> display works
locally is deliberately separated from proving the network path works. One commit.

Fills the src/golf_coach/api/ seam (currently a 6-line docstring). fastapi and uvicorn are ALREADY
declared in the `api` extra in pyproject.toml and unused.

Scope:
- Static upload page: a role picker (face-on / down-the-line / shot screen) sticky in localStorage
  so each phone is configured once, and a plain <input type="file" accept="video/*,image/*">. iOS
  Safari handles this natively — no app, no App Store, no signing. Keep it one page; this is a
  garage tool, not a product.
- POST /api/uploads — must STREAM the body to disk, never buffer a 200 MB clip in memory. Hash it,
  hand it to the Phase 3 store, return the assigned swing_id so the phone can show "-> swing 7,
  waiting on down-the-line."
- Background worker: on ready/partial, run pose per view, OCR the shot screen, then the Phase 4
  bundle analysis. An in-process asyncio task queue at concurrency 1 is sufficient — one swing per
  couple of minutes does not justify Celery or Redis. Uploads must not block on analysis.
- GET /api/sessions/<id> plus a results page: status per swing, score, checkpoints with bands and
  tour percentiles, ranked tips (already produced by feedback/rules.py — it just needs rendering),
  the HD Golf numbers, and the aligned video.

Constraints:
- Bind to 127.0.0.1 in this phase. There is no authentication; exposure comes next phase, deliberately.
- Keep heavy deps behind the extras boundary (ADR-008) — the API layer shouldn't drag OpenCV/OCR
  into the base install path.
- Check docs/M7_TWO_PHONE_SPIKE.md: if OpenCV couldn't decode iPhone HEVC, add a transcode step here.
```

### Phase 6

> 🛑 **SUPERSEDED — do not plan from this block.** Unlike the others, this prompt is not merely
> historical: what shipped on 2026-08-06 **contradicts its "settled decisions" point by point**,
> and it has already sent one cold session down the wrong path.
>
> | This prompt says | What the repo actually does |
> |---|---|
> | Bind uvicorn to the tailnet IP `100.x.y.z` | The bind never widens — `127.0.0.1` always; `tailscale serve` terminates TLS and proxies inward |
> | Add the bind host to `config.py` | Deliberately *not* a config field; `run_server.py` refuses a non-loopback `--host` with no token |
> | `api_port: int = 8080` | `api_port: int = 3000` — Windows reserves 8069–8168, so 8080 could never bind |
> | No auth; tailnet membership is the access control | `/api/` is gated on `GOLF_UPLOAD_TOKEN`, because a helper's phone can't join the tailnet |
> | Draft ADR-016 | [ADR-016](decisions/016-local-first-host-and-phone-upload-topology.md) is written and Accepted |
> | Validate on cellular, WiFi off | Done 2026-08-06 — three-file bundle in 30 s |
>
> Building this as written would reverse an Accepted ADR and delete a guard that has a regression
> test. Read the **Phase 6** section above and ADR-016 instead. Kept only as the record of what
> was believed before the phase was built.

```
Repo: C:\Users\aaron\Desktop\repo\golf-coach (Python 3.11, ADR-driven; read README.md and
docs/decisions/014-screen-capture-shot-ingestion.md for the project's offline-first stance).

Goal: two iPhones at an indoor golf simulator reach the FastAPI server running on my desktop at
home, over Tailscale. Small phase — config, docs, and validation, not application logic. One commit.

Settled decisions:
- Bind uvicorn to the desktop's Tailscale IP (100.x.y.z), NOT 0.0.0.0. There is no authentication on
  the upload endpoint; tailnet membership IS the access control. Binding wide would expose an
  unauthenticated file-upload endpoint to the home LAN. Add the bind host to src/golf_coach/config.py
  next to the existing api_port: int = 8080.
- Phones run the Tailscale iOS app and hit the MagicDNS name in Safari. No router ports opened, no
  public exposure, no cloud, no dynamic DNS.

Plan should cover:
- The config change and how the bind address is discovered/configured without hardcoding a machine-
  specific IP into the repo.
- Setup documentation: tailnet join for both phones, MagicDNS name, what to check when it doesn't work.
- A validation procedure that does NOT require driving to the sim: phone on cellular with WiFi off,
  reaching the desktop, uploading a real clip. Do this before relying on it in a bay session.
- Realistic expectations on upload time — each 1080p60 clip crosses the sim's uplink to reach the
  tailnet, and that is the dominant latency in the whole system, not compute. Include the guidance
  that phones should record 1080p60 rather than 4K, and why.
- Draft ADR-016 (local-first host & phone upload topology) in the existing ADR style, referencing
  ADR-014's offline-first reasoning.
- What to check on the first real bay session, and the failure modes to expect there.
```
