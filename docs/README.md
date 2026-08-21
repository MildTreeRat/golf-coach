# Documentation map

52 markdown documents: 42 in `docs/` — 14 here at the top level (including this map), 24 ADRs,
3 archived, 1 in `proposals/` — plus 10 outside it (the four at the repo root, and one each in
`data/` and `frontend/`, four in `spikes/`). This page says which one to read, and — just as
importantly — which ones are records of the past rather than descriptions of the present.

*(The breakdown is spelled out so the count can be checked rather than trusted — `git ls-files
'*.md' ':!.claude'` — because a bare number here has gone stale twice. It is now pinned by
`tests/test_docs_truth.py` rather than left to discipline. `.claude/` is excluded because the
agent and slash-command definitions in it are harness configuration, not documentation — nothing
here routes to them.)*

**Start with [../CLAUDE.md](../CLAUDE.md)**, at the repo root: the invariants, which section of
which document answers which question, and how much to read for a given kind of change. It is the
short one, and it is auto-loaded into every Claude session.

**Every doc declares a tier in its first lines:**

| Tier | Means | Trust its numbers? |
|---|---|---|
| **AS-BUILT** | Describes what exists and runs today | Yes |
| **TARGET** | Describes the intended design; much is unbuilt | It's a plan, not a measurement |
| **REFERENCE** | Current design, but records historical numbers by design | Read the design, not the numbers |
| **FOUNDING** | The charter the project started from | Read the intent, not the specifics |
| **SUPERSEDED** | Historical record, in `archive/` | No — see the banner for what replaced it |

The one rule that matters: **benchmark bands live in
`src/golf_coach/analysis/benchmarks/ranges.json`, never in a doc.** They have been re-derived
three times and each row carries its own provenance. Any band quoted in prose is a snapshot.

---

## Start here

Picking the project up cold, in order:

1. **[../README.md](../README.md)** — what this is, what state it's in, and the three commands
   that work today.
2. **This page** — the map.
3. **[ARCHITECTURE.md](ARCHITECTURE.md)** *(AS-BUILT)* — what actually runs. Start with §1's
   pipeline diagram; it is the only diagram in the repo that describes reality rather than intent.
4. **[../ROADMAP.md](../ROADMAP.md)** — read just the *Status at a glance* table, then the
   section for whatever you're working on.
5. **[../WORKLOG.md](../WORKLOG.md)** — **read the top entry, and only the top entry.** This is
   the best "pick up where I left off" document in the repo: every session records what was done,
   what surprised, where it was left, and what's blocked. It is also append-only across 30-odd
   sessions and by far the longest file here, so reading it whole costs more than everything else
   on this list put together and mostly buys history. Top entry, then stop.

Coming back after a break and wanting to *change* something? Read the ADR for the area first —
several encode decisions that look wrong until you see the measurement behind them (the
estimator choice, the tempo band, why a band edge is asserted only where it clears the
instrument).

---

## Living docs

| Doc | Tier | Answers |
|---|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | AS-BUILT | What runs today? What's a stub? What's actually persisted? How does one `analyze_swing` call work? |
| [FLOW.md](FLOW.md) | TARGET | Where is this going? What's blocked on what? The milestone status map, component and deployment views. |
| [PROJECT_CHARTER.md](PROJECT_CHARTER.md) | FOUNDING | Why does this project exist, what's in and out of scope, what are the risks? Plus how later ADRs sharpened it. |
| [CODE_STANDARDS.md](CODE_STANDARDS.md) | AS-BUILT | What rules is code held to here, and — just as importantly — which deliberate choices are *not* violations? Every rule cites a repo precedent or states plainly that it has none yet. |
| [REFACTOR_LEDGER.md](REFACTOR_LEDGER.md) | AS-BUILT | Has this refactor already been considered and declined? Append-only, one line per decision. Read before proposing a structural change, so the same idea isn't re-litigated. |
| [M4_ADDRESS_DETECTION.md](M4_ADDRESS_DETECTION.md) | AS-BUILT | How is the address instant found, why is it the weak one (7 frames vs 2 and 1), and why is tempo *dropped* rather than guessed on 14% of clips? |
| [M5_COACHING_FEEDBACK.md](M5_COACHING_FEEDBACK.md) | AS-BUILT | How are tips ranked and why does ranking need two different signals? Why was the panel held at three checkpoints for so long, and what unblocked it? |
| [M4_FUNDAMENTALS_PANEL.md](M4_FUNDAMENTALS_PANEL.md) | REFERENCE | What can face-on 2D pose measure, and what is deferred to a second view / detection / launch monitor? *(Its Findings numbers are superseded.)* |
| [M4_POSE_BAKEOFF.md](M4_POSE_BAKEOFF.md) | REFERENCE | Has this been tried already? The estimator bake-off, six rejected address signals, the arm-parallel no-go. **The longest doc here — grep it, don't read it.** |
| [M7_TWO_PHONE_CAPTURE.md](M7_TWO_PHONE_CAPTURE.md) | TARGET | The current live plan: two phones at a sim, seven phases (six built). **Its planning prompts are historical — Phase 6's is actively wrong and bannered.** |
| [M9_PLAYER_TRACKING.md](M9_PLAYER_TRACKING.md) | TARGET | The 20-phase plan for per-club shot history: how far do I hit my 7 iron, and where does it go? Nothing built yet — start at P1. The *why* is ADR-024. |
| [BAY_SESSION_RUNBOOK.md](BAY_SESSION_RUNBOOK.md) | AS-BUILT | Taking the two-phone capture to a real sim: preflight at home, phone settings and why 1080p60, measured timings, and what to check when it doesn't work. *(Failure modes are predicted until the first bay session.)* |
| [M7_TWO_PHONE_SPIKE.md](M7_TWO_PHONE_SPIKE.md) | REFERENCE | Does phase detection survive down-the-line, does OpenCV decode iPhone HEVC, and does `CAP_PROP_FPS` mean anything on slo-mo? Thresholds committed 2026-08-07; **results pending footage**. |
| [../data/README.md](../data/README.md) | AS-BUILT | The data layout, the three-tier reference cache, and how to rebuild the GolfDB corpus. |

`pose_bakeoff_v1.json` is the machine-readable companion to M4_POSE_BAKEOFF, written by
`scripts/golfdb/bakeoff.py`, which hard-codes the path — leave it in `docs/`.

[`proposals/`](proposals/README.md) is deliberately **not** in the table above. It holds drafts
written by the `/design` agent, unreviewed until someone reads them; a proposal earns a row here
only when it is promoted into `docs/` by hand. Nothing on this page routes to a draft, because
everything on this page is supposed to be trustworthy.

---

## Decisions (ADRs)

24 decisions, 29 addenda between them (`grep -c '^#\+ *Addendum' docs/decisions/*.md` — the
stated total had drifted to 11, then to 13, and is now pinned by `tests/test_docs_truth.py`
along with every per-ADR count in the last column).
**The addenda are where reality corrected the original call**, so a doc's original Decision
section is not always the final word — the counts below exist so you don't miss them.

| ADR | Decision | Status | Addenda |
|---|---|---|---|
| [001](decisions/001-language-python.md) | Python as the primary language | Accepted | — |
| [002](decisions/002-pose-estimation-mediapipe.md) | MediaPipe for pose estimation | Accepted | **2** — Tasks API replaced the removed Solutions API; **lite kept over full/heavy, measured** (12 McNemar tests, none significant) |
| [003](decisions/003-camera-hardware.md) | Camera hardware | Accepted | **4** — global shutter ≠ no motion blur; **pose camera goes face-on (3 o'clock)**; two cameras not three + the spine caveat; **the number behind the blur note — ~1/2000 s, so buy light not shutter type** |
| [004](decisions/004-launch-monitor.md) | Garmin R10 | Accepted, **not the near-term path** | — (superseded in practice by ADR-014) |
| [005](decisions/005-object-detection-yolov8.md) | YOLOv8 for club/ball detection | Accepted, **not yet startable** | **1** — M1.5 ran and deferred the labelling on evidence; the blocker is exposure time, not the detector |
| [006](decisions/006-mcp-server.md) | MCP server for shot data | Accepted | **2** — parsing moved out behind the `ShotDataSource` port; tool list re-scoped from shot-only to swings + shots |
| [007](decisions/007-decouple-software-from-hardware.md) | Software and hardware tracks run in parallel | Accepted | — |
| [008](decisions/008-project-structure.md) | Project structure & the `contracts/` seam | Accepted | **2** — two modules import *upward* into `api.state` for the tolerant artifact readers, knowingly; `mcp` named as a second imperative shell alongside `api`, and the `pose`/`detection` → `Frame` edge made type-only |
| [009](decisions/009-swing-scoring-model.md) | Dual-axis scoring with intent-driven policies | Accepted | — |
| [010](decisions/010-benchmark-ranges.md) | Benchmark ranges as versioned data with provenance | Accepted | **8** — JSON not YAML; two provisional rows; tempo re-sourced from GolfDB; **percentiles ride on `CheckpointScore` but never on the scoring path**; **two hip checkpoints promoted, and a rule for which band edges may be asserted**; **`hip_sway_norm`'s lower edge revisited and kept — the rule gains a second axis, resolution vs. placement**; **per-club bands gated and none cut — the club is not an axis this panel varies on**; **`unscored` carries the reason, not just the name — and `refilming_helps` is the bit that decides what a golfer is told** |
| [011](decisions/011-camera-synchronization.md) | Camera sync & multi-view 3D fusion | **Partially accepted** | **1** — a second capture tier: hand-held phones can be *aligned* but never *fused* |
| [012](decisions/012-golfdb-reference-data.md) | GolfDB as a reference-swing source | Accepted | — |
| [013](decisions/013-clip-relative-detection.md) | Clip-relative detection windows; explicit detection confidence | Accepted | — |
| [014](decisions/014-screen-capture-shot-ingestion.md) | Shot data by OCR of the simulator screen | Accepted | **1** — `spin_axis` was stored sign-inverted; the sign table gains the one tile the device prints already signed |
| [015](decisions/015-handheld-two-phone-capture-and-event-anchored-alignment.md) | Hand-held two-phone capture & event-anchored alignment | Accepted | — (settles the `FrameBundle` question ADR-011 left open) |
| [016](decisions/016-local-first-host-and-phone-upload-topology.md) | Local-first host & phone upload topology | Accepted | **1** — how the token is *held* is ADR-019's question, not this one's; the startup line prints a prefix, not the token |
| [017](decisions/017-club-head-detection-strategy.md) | Club-head detection strategy — and the constraint that actually binds | Accepted | **1** — why the spike's own threshold table did not decide it |
| [018](decisions/018-bay-lighting.md) | Bay lighting — buying the exposure ADR-017 asked for | Accepted | — |
| [019](decisions/019-secret-handling.md) | Secret handling — masked in memory, plaintext at rest | Accepted | **1** — a fourth unwrap site (`scripts/ask_swing.py`), recorded because the pin caught it and the decision says widening the surface must be deliberate |
| [020](decisions/020-conversational-followups.md) | Conversational follow-ups — a transcript store, and the tool runner over `query.py` | Accepted | — (the stdio round trip is for *external* clients; in-app calls go direct) |
| [021](decisions/021-caddieset-paired-reference-data.md) | CaddieSet as a paired mechanics/outcome source | Accepted as a corpus, **and its study returned a negative result** | — (face-on pose does not predict ball flight; the club sets the ball and the club is not in the picture) |
| [022](decisions/022-learned-artifacts-as-committed-data.md) | Learned artifacts as committed data — and the tour joint-distribution model | Accepted, **three models surfaced and spoken** | **4** — a second artifact (the trajectory model) under the same rule; `z` lost its A/B, the anchor set nearly made it unusable, and a pixel-aspect bug was worth 7 points of variance; then a down-the-line model, whose placement is reported beside the face-on one and never blended with it; then the policy for *saying* a placement, since a band was the wrong instrument for a corpus made entirely of swings that work; then what career mode may say about a *history* of one, deferred with its trigger and its seam |
| [023](decisions/023-tempo-training-and-absolute-swing-durations.md) | Tempo training, and absolute swing durations as reference data | Accepted, built and surfaced | **1** — durations enter `golfdb_v1.json` as distributions and never as a band, because tempo is scored once already; **two beat patterns, since no one pulse marks both the top and impact**; then the correction — **club was the wrong speed axis and the ADR over-claimed from it**: a real speed cohort (LPGA vs PGA) is 167 ms apart on the backswing, so the target now anchors to the golfer's own |
| [024](decisions/024-per-club-shot-history.md) | Per-club shot history — the tag that makes distance measurable | **Proposed**, not started | — |

Format: [000-template.md](decisions/000-template.md).

**The four most load-bearing, if you're short on time:** ADR-008 (why modules never import each
other, and why the analysis core is stdlib-only), ADR-009 (why there are two scores),
ADR-010 (why a missing benchmark yields *no* score rather than a wrong one), ADR-012 (why the
bands are trustworthy without hardware).

---

## Archive

`archive/` holds superseded milestone docs. Nothing is deleted — each carries a banner naming
what replaced it and why it was kept.

| Doc | Kept for |
|---|---|
| [archive/M1_CAPTURE_FLOW.md](archive/M1_CAPTURE_FLOW.md) | The face-on vs down-the-line **angle comparison** (+24% knee confidence) that settled canonical camera placement, and the **pose model reference table** — the only place the model variant, download URL and API choice are written down. |
| [archive/M4_ANALYSIS_POC.md](archive/M4_ANALYSIS_POC.md) | The correction that shaped everything after it: phase segmentation must anchor on the **top of the backswing**, not "first motion". |
| [archive/M4_POC_PLAN.md](archive/M4_POC_PLAN.md) | The agreed design and its **anti-over-engineering guardrails** — the explicit reasoning for not building `merge.py`, outcome checkpoints or SQLite at PoC stage. Those still hold. |

---

## Conventions

- **WORKLOG is append-only, reverse-chronological.** New entry per session; template at the
  bottom of the file.
- **A decision gets an ADR; a measurement gets a findings section.** ADRs record *why*;
  M4_POSE_BAKEOFF records *what the number was before and after*.
- **Rejected alternatives stay runnable.** `scripts/golfdb/tune_*.py` keep every losing
  candidate as a named row, including deliberate no-pose baselines any future candidate must
  beat. Don't delete the evidence — it's why the same idea doesn't get re-litigated.
- **When a doc's numbers go stale, add a banner rather than silently editing history.** That is
  the whole reason this map exists.
