# Documentation map

31 documents. This page says which one to read, and — just as importantly — which ones are
records of the past rather than descriptions of the present.

**Every doc declares a tier in its first lines:**

| Tier | Means | Trust its numbers? |
|---|---|---|
| **AS-BUILT** | Describes what exists and runs today | Yes |
| **TARGET** | Describes the intended design; much is unbuilt | It's a plan, not a measurement |
| **REFERENCE** | Current design, but records historical numbers by design | Read the design, not the numbers |
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
5. **[../WORKLOG.md](../WORKLOG.md)** — **read the top entry.** This is the best
   "pick up where I left off" document in the repo: every session records what was done, what
   surprised, where it was left, and what's blocked. If you read only one doc, read this one.

Coming back after a break and wanting to *change* something? Read the ADR for the area first —
several encode decisions that look wrong until you see the measurement behind them (the
estimator choice, the tempo band, why the panel is three checkpoints and not five).

---

## Living docs

| Doc | Tier | Answers |
|---|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | AS-BUILT | What runs today? What's a stub? What's actually persisted? How does one `analyze_swing` call work? |
| [FLOW.md](FLOW.md) | TARGET | Where is this going? What's blocked on what? The milestone status map, component and deployment views. |
| [PROJECT_CHARTER.md](PROJECT_CHARTER.md) | Founding | Why does this project exist, what's in and out of scope, what are the risks? Plus how later ADRs sharpened it. |
| [M4_ADDRESS_DETECTION.md](M4_ADDRESS_DETECTION.md) | AS-BUILT | How is the address instant found, why is it the weak one (7 frames vs 2 and 1), and why is tempo *dropped* rather than guessed on 14% of clips? |
| [M5_COACHING_FEEDBACK.md](M5_COACHING_FEEDBACK.md) | AS-BUILT | How are tips ranked and why does ranking need two different signals? Why did the panel stay at three checkpoints? |
| [M4_FUNDAMENTALS_PANEL.md](M4_FUNDAMENTALS_PANEL.md) | REFERENCE | What can face-on 2D pose measure, and what is deferred to a second view / detection / launch monitor? *(Its Findings numbers are superseded.)* |
| [M4_POSE_BAKEOFF.md](M4_POSE_BAKEOFF.md) | REFERENCE | Has this been tried already? The estimator bake-off, six rejected address signals, the arm-parallel no-go. **782 lines — grep it, don't read it.** |
| [M7_TWO_PHONE_CAPTURE.md](M7_TWO_PHONE_CAPTURE.md) | TARGET | The current live plan: two phones at a sim, seven phases, with a self-contained planning prompt per phase. |
| [../data/README.md](../data/README.md) | AS-BUILT | The data layout, the three-tier reference cache, and how to rebuild the GolfDB corpus. |

`pose_bakeoff_v1.json` is the machine-readable companion to M4_POSE_BAKEOFF, written by
`scripts/golfdb/bakeoff.py`, which hard-codes the path — leave it in `docs/`.

---

## Decisions (ADRs)

14 decisions, 11 addenda between them. **The addenda are where reality corrected the original
call**, so a doc's original Decision section is not always the final word — the counts below
exist so you don't miss them.

| ADR | Decision | Status | Addenda |
|---|---|---|---|
| [001](decisions/001-language-python.md) | Python as the primary language | Accepted | — |
| [002](decisions/002-pose-estimation-mediapipe.md) | MediaPipe for pose estimation | Accepted | **2** — Tasks API replaced the removed Solutions API; **lite kept over full/heavy, measured** (12 McNemar tests, none significant) |
| [003](decisions/003-camera-hardware.md) | Camera hardware | Accepted | **3** — global shutter ≠ no motion blur; **pose camera goes face-on (3 o'clock)**; two cameras not three + the spine caveat |
| [004](decisions/004-launch-monitor.md) | Garmin R10 | Accepted, **not the near-term path** | — (superseded in practice by ADR-014) |
| [005](decisions/005-object-detection-yolov8.md) | YOLOv8 for club/ball detection | Accepted | — (not built; gated on M1.5) |
| [006](decisions/006-mcp-server.md) | MCP server for shot data | Accepted | **1** — parsing moved out of the server, behind the `ShotDataSource` port |
| [007](decisions/007-decouple-software-from-hardware.md) | Software and hardware tracks run in parallel | Accepted | — |
| [008](decisions/008-project-structure.md) | Project structure & the `contracts/` seam | Accepted | — |
| [009](decisions/009-swing-scoring-model.md) | Dual-axis scoring with intent-driven policies | Accepted | — |
| [010](decisions/010-benchmark-ranges.md) | Benchmark ranges as versioned data with provenance | Accepted | **4** — JSON not YAML; two provisional rows; tempo re-sourced from GolfDB; **percentiles ride on `CheckpointScore` but never on the scoring path** |
| [011](decisions/011-camera-synchronization.md) | Camera sync & multi-view 3D fusion | **Partially accepted** | **1** — a second capture tier: hand-held phones can be *aligned* but never *fused* |
| [012](decisions/012-golfdb-reference-data.md) | GolfDB as a reference-swing source | Accepted | — |
| [013](decisions/013-clip-relative-detection.md) | Clip-relative detection windows; explicit detection confidence | Accepted | — |
| [014](decisions/014-screen-capture-shot-ingestion.md) | Shot data by OCR of the simulator screen | Accepted | — |

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
