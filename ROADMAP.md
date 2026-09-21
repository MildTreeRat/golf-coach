# Roadmap: AI Golf Swing Trainer

## Last Updated: 2026-09-21

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
| **M9** Player tracking (per-club) | ✅ Done *(2026-08-22)*, 20/20 phases | — (built and **silent by design**: every club-narrowed answer refuses, because no swing on disk is tagged yet. A bay session, and the retag control, are what make it speak) | [§M9](#m9-player-tracking-per-club-shot-history--done-and-waiting-on-a-bay-session) |
| **M10** Alignment accuracy | ✅ Done *(2026-08-26)*, 10/10 phases | — (the corpus has been re-analysed against the fix; the face-on top it left landing late was closed by M11 on 2026-08-29) | [§M10](#m10-alignment-accuracy--the-two-panels-leave-address-together--done) |
| **M11** Acoustic sync | ✅ Done *(2026-08-30)*, 12/12 phases | — (P10 and P11 closed the addendum's residual: the anchor now matches contact by eye to within a frame on all ten clips read frame by frame) | [§M11](#m11-acoustic-sync--the-ball-strike-is-the-clock--done) |
| **M12** Club specs | ✅ Done *(2026-09-01)*, 8/8 phases | — (desk work, and it produced the first `.bag.json` this repo has ever held; only the form's **layout on a phone** is unverified) | [§M12](#m12-club-specs--the-golfer-names-a-club-the-program-determines-what-it-is--done) |
| **M13** Downswing tempo | ✅ Done *(2026-09-02)*, 8/8 phases | — (desk work; all 15 stored swings re-analysed onto the new sentence with no score moved. Only the two pages' **layout** is unverified) | [§M13](#m13-downswing-tempo--the-downswing-is-what-you-feel-the-backswing-is-what-you-change--done) |
| **M14** Hand landmarks | ✅ Done *(2026-09-03)*, 6/6 phases | — (desk work on artifacts already on disk; it re-opened a closed question and answered it, and shipped four metrics with **no band, no checkpoint and no `ANALYSIS_VERSION` bump**. The overlay change is unverified in a browser) | [§M14](#m14-hand-landmarks--six-points-nobody-reads-and-the-head-dots-nobody-measures--done) |
| **Hands spike** MediaPipe Hands on a grip | ✅ Done *(2026-09-03, **no-go**)* | — (ran on the fifteen face-on swings already on disk; M14's successor milestone does not open, and only the ADR is left to write) | [§Hands spike](#hands-spike-mediapipe-hands-on-a-golf-grip--m14s-successor-and-a-no-go) |
| **M15** Ball flight | ✅ Done *(2026-09-06)*, 20/20 phases. **The flight is measured, stored, served, drawn as a golfer sees it, said and answerable — and every surface names the numbers nothing measured** | — (P0–P9: ADR-027, this table, one bag entry corrected, the published constants committed, the RK4 integrator built in three dimensions, **the gate passed at ±2.59%**, the altitude what-if, a CLI over all of it, **the flight solved backwards**, **P9 the loft prior and the axis**, and **P10 the corpus, read through a join**. Read the agreement with its five caveats before quoting it: both validation shots fly the whole way above the only public coefficient table; above that clamp **spin does not reach the flight at all**; the gate passes per shot while the model ranks the two shots backwards, an inversion the recorded spin axis closes only a tenth of and altitude *widens*; and the spin solve **names a number for 4 of the 11 spin-less shots and refuses 7**. P9 added a sixth about the corpus rather than the physics — run end to end the inference produced no spin at all — and **P10 corrected it**: the club is on the *swing*, not on the shot, eleven of the thirteen shots are attached to a swing that carries one, and the inference names **one spin** (`2026-08-23-4`, 2,924 rpm under a 5,103 rpm cap) and refuses ten. Two of those refusals are a 3 wood nobody has declared in the bag — the bag page, not the bay. P10 also checked P4's hand-typed gate constant against disk for the first time, and it matches to the digit. **P11 landed the six measurements** and corrected §Decision 6 twice: `flight_spin_rpm` records only a *solved* spin, because pooling it with a printed one is the hazard that section exists to prevent; and P10's planar-offline identity is not structural — it holds where the spin was solved from the carry and breaks by 0.29 yd where it was measured. **P12 registered the `model:` prefix** in `contracts/career.py::CorpusSwing.artifact_key` — not `storage/corpus.py`, where the ADR and this roadmap both had it — keying a flight on the **shot photo** it was flown from, and found P11's reason for shipping ahead of it inverted: the `swing:{ref}` fallback can only over-count on the dedupe but carries **no flagged-parse refusal**, so a flight built on a tile flagged under ADR-014 counted as a sample while the carry printed beside it did not. `population:golfdb` stays unregistered on purpose — it moves no count and would decide ADR-022's fourth addendum by accident. **P13 bumped `ANALYSIS_VERSION` 14 → 15** and re-analysed all fifteen stored swing directories onto it: every score byte-identical, five artifacts carrying a flight and ten a refused one — and the finding is that this re-run **cannot prove P12's registration**, since no two distinct swings on this corpus share a shot photo, so the photo key and the `swing:{ref}` fallback partition it identically. It also swept in four M14 measurements no stored artifact had ever carried, which is the milestone that read the band question and the version question as one question. **P14 put the flight behind `GET .../flight`**, which re-flies at read time because the artifact stores the six numbers and no path — and found the seam twelve earlier corrections had not: two of a flight's inputs live in *editable* artifacts, so the route and the stored artifact can disagree about one shot. Declare the 3 wood's loft and the page draws a flight the corpus still counts as refused, with nothing on disk able to see the difference. **P15 drew it** — a canvas, vanilla JS, two projections of one polyline rather than a camera on it — and the finding is the plan view's vertical axis: it must be stretched by the **smallest factor that makes the curve readable, not the largest that fits**, because every flight on this corpus drifts a third to a half of its own apex and the first rule drew the lateral miss *taller than the height of the shot* beside a panel captioned 1:1. On this corpus the stretch now never fires. P15 also found ADR-027's own Status block a phase behind — eleven addenda over twelve, P14 still listed as unbuilt, and the *three flights* count `docs/README.md` had already been corrected to five — because the doc-truth suite pinned the map's row and not an ADR's account of itself. **P16 put the printed numbers beside the simulated ones** — the carry as a hollow ring on the ground line, the start line as a rule across the plan view, the four `caveats_for` sentences rendered for the first time — and the finding is that **neither of the two available pairs is a check**: where the spin was solved the printed carry is the solve's own *input*, reproduced to +0.001 yd, which is the most convincing pair of numbers on the page and evidence of nothing; and the two offlines are where the ball started against where it finished, a gap that splits exactly into bend **plus** the carry error leaning on the start line rather than being "the curve". The pairing itself turned out to be a registry decision rather than a rendering one, correcting P15's framing of this phase. P16 also found `sign_disagrees` had **never been rendered to a golfer**: the one shot on disk that sets it is a refusal, so the launch-conditions block the sentence was first written into never renders for it. **P17 added the eleventh MCP tool** — `simulate_flight(session_id, swing_id)`, offered without a golfer registry because a printed spin borrows nothing from a bag — and found the surface that had been quietest: `mcp/query.py` flattens `measurements` to name -> value, so the six reached a **coaching model as bare floats** under a field description calling them measured, and a solved spin was indistinguishable from a printed one exactly where the output is sentences spoken to a golfer. `SwingView.simulated` is the split, keyed on `Measurement.source` and never on the name. P17 also found P10's join runs the **wrong way** for this question — one screen photo here is attached to three swings, and a shot-to-swing join names one survivor — and built `differs_from_recorded`, the first thing anywhere that can *see* P14's seam instead of re-flying past it. **P18 cascaded the docs**: the tool count ten → eleven, ADR-027's Status block to built with its fifteenth addendum, and this section to done. **P19 added the view a golfer means by seeing a shot** — a perspective tracer from behind the ball, down the target line, red and animated, with the ball's own track on the turf under it because a curve seen end-on is mostly foreshortened away. It corrects P15's rationale without overturning it: a camera really does foreshorten both questions at once, so the tracer is a **third** panel captioned `perspective — nothing measurable` rather than a replacement for the two that can be read, and it asserts exactly what the plan view asserts — no landing ring and no offline sentence on a planar flight. Its two findings are geometry: the top of a perspective frame is **not the apex**, because a camera sees angular elevation and that peaks a good forty yards earlier than the flight does (fitting on the apex put the tracer nine pixels off the top of the canvas); and a camera at literal eye height is the literal answer and a bad picture, compressing everything past 25 yards into about **seventeen pixels** above the horizon. **The only unverified thing in the milestone is still the page's layout** — no browser has ever been driven) | [§M15](#m15-ball-flight--the-model-the-launch-angle-was-recorded-for--done) |
| **M16** Mishits | ✅ Done *(2026-09-08)*, 9/9 phases (P0–P8) | — (desk work over the corpus already on disk; building corrected nothing in ADR-028. The rule went live in P3 and found its first real subject unprompted in P5 — aaron's 7 iron `2026-08-23/2` carried 33.6 yd against a 121.8 yd median. Holding it out drops that club's clean carries to four, below the CENTER floor, so `get_club_profile` now *withholds* its carry mean where it used to print a top-dragged ≈101.6 yd — no number beats a wrong one. No `ANALYSIS_VERSION` bump — the aggregates are live) | [§M16](#m16-mishits--the-topped-seven-iron-that-is-not-your-seven-iron--done) |
| **M17** Pivot points | ✅ Done *(2026-09-10)*, 9/9 phases | — (desk work over the corpus already on disk; the overlay and the rule checks needed no bay session) | [§M17](#m17-pivot-points--the-shoulder-line-the-hip-line-and-the-three-points-they-turn-about) |
| **M18** The platform decided | ✅ Done *(2026-09-21)*. Docs only — the spikes were dropped when the two-iPhone premise was, and **the ADR became the milestone**. Rust core, MediaPipe pose in a bundled Python sidecar pool, Flutter shell, phone-as-camera later, cloud closed. **ADR-030 supersedes ADR-001**, the first superseded ADR here. The load-bearing choice was not the language or the shell but *where pose runs*: `ranges.json` is cut from MediaPipe's landmarks, so keeping pose in Python is what lets everything else be rewritten without reopening the scoring model | — (desk work; the decisions were taken directly) | [§M18](#m18-the-platform-decided--a-rust-core-a-python-pose-sidecar-and-a-flutter-shell) |
| **M19** The core as a specification | ✅ Done *(2026-09-21)*, 5/5 phases. **M22 is unblocked** — `python scripts/conformance.py check` diffs any implementation against this one, 21 vectors at v16 in ~7 s. Five schemas in `spec/schemas/`, 6 synthetic vectors and the 15 real swings gzipped to 8.3 MB in `spec/vectors/`, the rules in [docs/CONFORMANCE.md](docs/CONFORMANCE.md). Two findings a port would otherwise have inherited: the `exclude=` set that makes `analysis.json` what it is lives at a **call site** and in no schema; and a bare engine call leaves `feedback` None, so the first build pinned `"feedback": null` on all 21 vectors — a spec telling a port to ship no coaching | — (desk work; done) | [§M19](#m19-the-python-core-becomes-a-specification--schemas-golden-vectors-and-a-conformance-runner) |
| **M20** The trigger | ⬜ Not started | Nothing for the algorithm (replay over the stored clips); one bay trip for false positives | [§M20](#m20-the-trigger--hearing-the-ball-strike-live-and-cutting-the-clip) |
| **M21** Capture edge | ⬜ Not started — **the file source needs nothing**; the webcam and phone sources need M20's spec | Nothing for source 1; cameras for 2 and 3 | [§M21](#m21-capture-edge--a-file-then-a-webcam-then-a-phone) |
| **M22** The Rust core | ⬜ Not started, **unblocked 2026-09-21** | Nothing — M19 shipped the oracle; read [docs/CONFORMANCE.md](docs/CONFORMANCE.md) first | [§M22](#m22-the-rust-core--the-analysis-engine-passing-the-conformance-suite) |
| **M23** The pose sidecar | 🔒 Blocked | M22 (something has to send the jobs) | [§M23](#m23-the-pose-sidecar--a-long-lived-python-worker-pool) |
| **M24** Session engine | 🔒 Blocked | M21 and M22 | [§M24](#m24-session-engine--start-a-session-and-swings-flow-through-to-the-profile) |
| **M25** The app | 🔒 Blocked | M24 | [§M25](#m25-the-app--the-flutter-shell-and-the-setup-wizard) |
| **M26** Ship it | 🔒 Blocked | M25 (CI can start as soon as there is a `Cargo.toml`) | [§M26](#m26-ship-it--ci-packaging-signing-and-distribution-per-os) |
| **M27** Remote worker | ❌ **Closed** *(2026-09-21)* by ADR-030 §7 — not deferred. Kept as the record of a decision | — | [§M27](#m27-remote-worker--closed-not-deferred) |
| **M5** Feedback UI | ⬜ Not started, **superseded in shape by M25** (no web UI) | M7 Phase 5 gives the host | [§M5](#milestone-5-feedback-ui) |
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
`initialize` negotiated protocol `2025-11-25`, advertised every tool it then held with their
schemas, and
served live `call_tool` requests including the not-found path. It is registered with Claude Code
(`claude mcp add`, per the README) and reports `✔ Connected`, which is a second client completing
the same handshake independently.

**NEXT ACTION — one bay session.** M9 closed on 2026-08-22 with P20 and M10 closed on
2026-08-26, so this board is again empty of desk work and everything left wants the bay or wants
`n` — one trip serves both: M7 Phase 0's field spike, M3's remaining OCR work and M2's
lighting test all want the screen and the bay in front of you, while career mode, the dispersion
discriminator, the tour join and every per-club answer are built, correct and refusing at `n = 2`.
20–30 tagged swings in one session turns all of them on at once.
[BAY_SESSION_RUNBOOK.md](docs/BAY_SESSION_RUNBOOK.md) sequences the trip; **set the club cursor on
the upload page before the first swing**, because a session hit without club tags produces data
that can never be split by club afterwards.

**M10 and M11 are why the trip is worth taking now rather than a fortnight ago.** A bay session
recorded against the windowing bug would have produced 20–30 more bundles scored on the wrong
frames; the code was fixed on 2026-08-26 and the corpus re-analysed the same day (M10 P10). What
M10 left behind — the face-on top landing late — is **closed** as of 2026-08-29:
[§M11](#m11-acoustic-sync--the-ball-strike-is-the-clock--done) took ADR-015's parked Option C, the
ball strike both phones already record, as the shared clock that decides which view's top is wrong
rather than merely doubting both. Every bundle on disk now reads `synchronized`, and a `tempo` timed
from a contradicted top is withdrawn instead of shipped. So a session filmed today is scored on
frames two views agree about. **M11's residual closed on 2026-08-30**, and it was two errors rather
than one: the anchor was taking a quiet transient 2-3 frames ahead of the ball, and on the four
down-the-line clips whose container presents its video 105-125 ms after its audio it was landing
6-7 frames late on top of that. They ran in opposite directions and partly cancelled, which is why
neither showed up in a score. P10 measures the video's own presentation start and subtracts it; P11
lands the candidate floor P9 specified and had to revert. Ten clips have now been read frame by
frame in both views and the anchor agrees with contact on every one of them, so `tau=2` is a
measured frame rather than an estimate — which is what the second panel of every render is drawn
against.

**M9's own last blocker went in the session before P20**: `index.html`'s swing list now has a
per-swing **change** control beside the golfer one, so a swing already on disk is retagged from a phone in one tap over `POST
/api/sessions/{session}/swings/{swing}/club` — a route that had worked and been untested by any UI
since P6. It is the club's only repair path and deliberately one swing at a time; a session has
many clubs, so there is no bulk backfill and there is not meant to be one (ADR-024 §5). **Driven on
a copy of the real corpus**: with the two stored swings retagged, `scripts/club_profile.py` leaves
the empty state and prints a `7i` row — 2 swings, 2 shot photos, 2 sessions, every claim honestly
withheld against the 5-, 10- and 12-sample floors — and `mcp/club.py` moves from `NEVER_HIT` to
`NOT_ENOUGH_ON_THIS_CLUB`. **The real `data/` was left alone**: only the person who hit those two
swings knows what hit them, which is ADR-024 §5's own argument, so that tap is theirs to make.
Until 2026-08-20 this section read *"nothing on this board is desk work any more"*, and
[M9](#m9-player-tracking-per-club-shot-history--done-and-waiting-on-a-bay-session) is what
stopped that being true — it was the one substantial item needing **neither a bay session nor an
`n`**, because no shot on disk recorded which club hit it and adding that tag was pure desk work
that makes the *next* bay session's data worth more than the last one's. **As of 2026-08-22 that
sentence is true again**: M9 is closed, and everything left on this board wants the bay or wants
`n`.

**It went false again on 2026-08-31, for the same reason it was in M9's week, and
[M12](#m12-club-specs--the-golfer-names-a-club-the-program-determines-what-it-is--done) closed it
on 2026-09-01.** M9 built the bag and nobody had ever filled it in, so `data/processed/golfers/`
held a golfer record and no `.bag.json` at all until M12 P6 wrote the first one. A five-field form
asking for a lie angle is what produced that, and the fix was that the golfer names a club and the
program determines its specification. It needed neither the bay nor an `n` — and, like M9, it makes
the bay session that follows it worth more, because a club's specification is another input that is
unrecoverable after the fact. **With it closed, the sentence above is true once more: everything
left on this board wants the bay or wants `n`.**

**The ingest spine is closed.** P1–P7 all landed 2026-08-21, and a swing can no longer reach disk
untagged: `contracts/club.py` holds the taxonomy, `contracts/bag.py` the declared bag,
`storage/bag_store.py` puts a bag on disk at `data/processed/golfers/<player_id>.bag.json` beside
the golfer's own record, and **P4 carried the club into two things that already run**:
`SwingManifest.club` and a second session cursor, `SessionMeta.club`. P5 wrote the field — threading
the club through `bundle_store` at swing creation, plus a per-swing repair route — P6 put the
requirement at the boundary, and P7 gave the phone a one-tap way to satisfy it. The design is
[ADR-024](docs/decisions/024-per-club-shot-history.md), whose addendum records the one call P3 added
to it — a club that leaves the bag is kept rather than overwritten.

P4 also fixed a latent bug it had to: `set_current_player` replaced the whole `session.json`, which
was correct with one cursor and would have cleared the club with two.

**P9 closed the measurements track, and closed P11 with it.** `start_line_offline_yds` answers
"how many yards right or left" in yards instead of degrees: `carry * sin(start line)`, exact
trigonometry over two printed numbers with no physics invented. It is honest about what it is not —
the screen prints **no offline tile**, so this is where the ball *would* have landed if it never
curved, and the curve is a separate reading in `face_to_path_deg`. A golfer who starts it straight
and slices reads about 0 here, which is why any prose over it must say *started* and never
*finished*. `ANALYSIS_VERSION` went 8 → 9 and all four stored swings moved onto it with every
`overall_score` byte-identical; the two real shots on disk read −11.6 yd and +8.4 yd, opposite
directions, which is the sign working on real data. It also carried P11's last row — target `0.0` by
geometry, tolerance 9.0 yards, which is `_JUDGED_DEGREES`'s 2° evaluated at a 250-yard driver, the
widest club in the bag. **P11 is therefore done, having never run as a phase**: the strict-equality
pin means a target row cannot lag its metric by even one commit, so P8 took two thirds of it and P9
took the rest.

**P12 opened the corpus track** (2026-08-21). `CorpusSwing` now reads `manifest.club` off the
**survivor** of a duplicate group — the earliest arrival, so a clip re-sent after the session cursor
moved on cannot rename the swing it duplicates — and `CareerCorpus.untagged_swings` counts what
per-club work cannot see. That counter is a **derived property**, not the stored field the phase
list specified: an unattributed manifest never becomes a `CorpusSwing` and so must be tallied during
the scan, whereas an untagged one is a real swing in `swings`, and deriving it is what stops P13's
`narrow_to` reporting the whole read's figure beside a filtered swing list. An untagged swing is
**counted, never excluded** — the club was never an input to measuring head sway, so excluding it
would shrink the mechanics `n` to punish a missing tag mechanics never needed.

**P13 closed the filter, and it is three lines.** `narrow_to(club=)` (2026-08-22) takes a `ClubId`
and adds one clause to a comprehension; everything that makes it *honest* was already there, because
the function recomputes `metric_counts` inside the filter rather than trusting a caller. So a
per-club mean carry refuses at exactly the thresholds a whole-corpus metric refuses at — "your 7
iron carries 164 yards" needs five distinct **7-iron** shots, not five shots, and nothing new had
to learn the guard. It is also where an untagged swing finally drops out: `read_corpus` keeps it
on purpose, since the club was never an input to measuring head sway, and a per-club view is the
one place the tag is load-bearing. `untagged_swings` follows to 0 for free, which is what P12's
derived-property shape was for. On the real corpus every club narrowing is honestly empty — both
swings on disk predate the tag.

**P10 was not skipped after all — it shipped on 2026-08-21 and its paperwork landed on the 22nd.**
`ball_speed_mph` and `launch_angle_deg` are recorded as fitting inputs with no target and no band,
and they carried their two `METRIC_TARGETS` rows with them because P11's parity pin leaves no
choice. **It also bumped `ANALYSIS_VERSION` 9 → 10 without running `scripts/reanalyze.py`**, so for
one day every stored swing read `OUTDATED` and `Honest n per metric` printed *(none)*. Repaired
alongside P13: all four swings are on version 10, every `overall_score` byte-identical, and 21
metrics are back in the counts.

**P14 opened the profile track.** `contracts/club_profile.py` declares `ClubProfile` and
`BagProfile` — the shape a per-club answer comes back in, composing `MetricBaseline`,
`MetricDispersion` and `BagEntry` rather than restating them, so the minimum-`n` guard and the
bias/scatter discriminator arrive per club with nothing new learning the rules. It carries **two
evidence counters, not one**: a club filmed six times with two screen photos is six swings of
history and a carry ceiling of two, since every launch-monitor claim dedupes on the photo's hash.

**P15 filled those shapes and answered the ADR-008 question the same day.** The pure half of
`narrow_to` moved onto `CareerCorpus.narrowed_to` — taking `count_metrics` with it, since a filter
that does not recompute its counts is the bug the filter exists to prevent — and
`storage.corpus.narrow_to` now delegates in one line, so `analysis/club_profile.py` narrows per club
while `analysis` still imports `contracts` alone. The builder is 30 lines of assembly over
`build_baseline` and `build_dispersion`, which is the size it should be: everything it reports was
sealed by the step allowed to seal it. A static test pins the module's imports, because the
convenient `from golf_coach.storage.corpus import narrow_to` is one keystroke away and breaks
nothing at runtime.

**P16 closed the profile track.** A club whose bag entry was recorded after some of its swings
now says so in a sentence and keeps every statistic — and the sentence has two forms, because when
*every* swing predates the entry nothing is pooled at all and only the make, model and loft are in
doubt. That is the common case rather than the exotic one: it is what every club looks like the day
a golfer first declares a bag.

P17 built `scripts/club_profile.py` on top of that — the CLI that reads the numbers without a
browser or an MCP client, and the phase that proved the spine works end to end. It was the first
surface to render one of these caveats for real.

**P8 opened the measurements track and cashed the tag in.** `carry_distance_yds` and
`total_distance_yds` are measured, which they could not honestly be before: a carry pooled across
clubs is meaningless, so the club tag is what makes distance poolable at all. Both are *measured and
judged by nothing* — no target, because how far a golfer should hit a club is not a number this repo
has and every distribution here is cut from GolfDB, which contains no ball flight. `ANALYSIS_VERSION`
went 7 → 8 and all four stored swings were re-analyzed onto it with every `overall_score`
byte-identical. P8 also had to ship its two target rows, because an unregistered production metric
fails `test_every_production_metric_has_a_tolerance` rather than going quietly silent — which is the
rule P9 then inherited.

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

> **Addendum (2026-09-03, M14 P3):** **the word "grip" in that list was narrowed by measurement,
> and the rest of the list stands.** The evidence behind "invisible" is
> [M4_POSE_BAKEOFF.md](docs/M4_POSE_BAKEOFF.md) §Phase G, and §Phase G screened hands
> **down-the-line**: the *lead* thumb / index / pinky come in at 0.37 / 0.39 / 0.40 against its
> 0.60 exclusion floor. Asked of the other camera — same screen, same floor, over the address
> window — all six hand landmarks track in **1.00 of frames on 15 of 15 stored swings**, the
> weakest of them (lead thumb) at p10 0.67 against a 0.50 visibility gate. The control is what
> makes it a finding: the same script over the same swings in the same window, one flag apart,
> puts the lead hand at **0.25** down-the-line and clears the floor on 4 of 15. **The variable is
> the camera, not the landmark** — and mechanics are scored from the face-on one. Four hand
> metrics now measure over that window; see
> [docs/M14_HAND_LANDMARKS.md](docs/M14_HAND_LANDMARKS.md) §P3–§P5 and [§M14](#m14-hand-landmarks--six-points-nobody-reads-and-the-head-dots-nobody-measures--done).
>
> **What does not change, and it is most of it.** There is no grip verdict: a visibility screen is
> an exclusion floor and not a relevance ranking, so a landmark at 1.00 is *measurable* and not yet
> *useful*, and none of the four metrics has a band. It is an **address** result and not a swing
> one — face-on over the whole clip the same hands fall to 0.63–0.68. Grip *strength*, the V's and
> the knuckle count, needs MediaPipe Hands, which this repo has never evaluated. **Wrists and
> clubface are untouched**: a visible wrist landmark is not a lead-wrist *angle*, and the club
> still needs M2. And the argument built on this sentence is unaffected — dispersion is still the
> discriminator, and it is still `n` that blocks it.

> **Addendum (2026-09-03, MediaPipe Hands spike):** the sentence above says grip strength "needs
> MediaPipe Hands, which this repo has never evaluated." **It has now been evaluated, and the
> answer is no-go** — so *grip strength* returns to the invisible list, this time measured rather
> than inherited from a screen run in the wrong view.
>
> The model does not fail to find a hand. Over the same fifteen face-on swings and the same address
> window, at the best pose-guided crop, it returns a hand on **179 of 179 frames**, places it on
> the correct wrist (1.00), labels its handedness correctly (1.00 trail) and puts its index knuckle
> **1.6–3.6 cm** from the pose model's own. It fails to find **two**: a golf grip reads to a model
> trained on open, gesturing hands as a single hand, and two are resolved on **0.15** of frames
> against a 0.60 floor. The hand it keeps is the **trail** hand (1.00); the lead hand, which is the
> one grip strength is read on, comes back at **0.14**. Dropping the detection threshold to 0.2
> takes that to 0.35 — the second hand is partly suppressed as an overlapping duplicate and partly
> never proposed, and neither half reaches the floor.
>
> **What this does not change.** M14's four hand metrics are untouched: they come from the *pose*
> model's coarse hand points, which still track at 1.00 at address, and where the hands *are*
> remains measurable. The narrowing above stands. What closes is the successor milestone — there is
> no route from this instrument to the V's or the knuckle count, and the next one would be training
> a hand model on interlocked hands on a club, which is M1.5's declined labelling effort in a
> different costume. Findings: [spikes/mediapipe-hands/log.md](spikes/mediapipe-hands/log.md).

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

  > **Addendum (2026-09-03, M14 P3):** "grip … invisible to this instrument" is narrowed — the
  > hands are measurable face-on **at address**, at 1.00 tracked frames on 15 of 15 stored swings,
  > and the screen that said otherwise was run down-the-line. The full correction, and the four
  > things it does *not* license, sit with the other statement of this claim — the **Biggest
  > constraint on coaching** paragraph in the status prose at the top of this file, which is
  > unlinkable because it lives above the first heading.
  > **The argument on this line survives it intact**, and this is the place to notice why: what is
  > measurable is where the hands *are*, not whether the grip is *good* — there is no band and no
  > verdict — so the cause is still unknowable from a single swing and the variance is still what
  > splits it. Lead-wrist angle and release timing are untouched.

  > **Addendum (2026-09-03, MediaPipe Hands spike):** grip *strength* — the static cause this
  > bullet would most like to see — is now measured as **out of reach**, not merely unbuilt. The
  > full correction sits with the other statement of this claim, in the **Biggest constraint on
  > coaching** paragraph above; the short version is that a golf grip reads to `HandLandmarker` as
  > one hand, and the one it keeps is the trail hand. **This bullet's argument is again unaffected**
  > — it never needed to see the grip, only to split static causes from timing ones by variance.
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

## M9: Player tracking, per-club shot history — done, and waiting on a bay session

**Design**: [ADR-024](docs/decisions/024-per-club-shot-history.md).
**Phase list**: [docs/M9_PLAYER_TRACKING.md](docs/M9_PLAYER_TRACKING.md) — 20 phases, each
independently commit-ready. **P1–P12 landed 2026-08-21, P13–P20 on 2026-08-22. All 20 are in.**
P1–P7 are the whole ingest spine: the vocabulary, the bag shape, the bag on disk, the
club on `SwingManifest` and on a second session cursor, the writer that stamps it, the 409 that
refuses an untagged upload, and the one-tap picker that satisfies it. A swing can no longer reach
disk without a club. **P8 opened the measurements track** and put carry and total distance into
`measurements` — the thing the tag was for, since a carry pooled across clubs describes nobody's
shot. Both are measured and judged by nothing: no target exists for how far a golfer should hit a
club, and none is invented. **P10 took the optional phase too**, recording ball speed and launch
angle as fitting inputs judged by nothing. **P9 closed that track** with `start_line_offline_yds`,
the start line
projected out to the carry — where the ball *started*, in yards, which is the only lateral number
this screen can honestly produce because it prints no offline tile. It takes a target of `0.0`,
because zero is straight by geometry rather than by a population. P11 was absorbed into P8 and P9
rather than run: the parity pin means a target row cannot lag its metric by a commit.

**The gap M9 closed, in one sentence.** This repo could say how a swing compares to a tour
population and how it compares to the golfer's own history. It could not say how far you hit your
7 iron, because **no shot on disk recorded which club hit it.** A swing can no longer reach disk
without that field, and every reader of it is built — so what is left is not code, it is swings.

**Why it is mostly wiring.** Career mode already built everything downstream of that field: the
corpus reader that counts an honest `n`, the baseline with its minimum-`n` guard, the bias/scatter
discriminator, and — the load-bearing one — `storage.corpus.narrow_to`, which filters a corpus
*and recomputes its metric counts*. **P13 added that `club=` clause on 2026-08-22**, and the whole
career pipeline now produces per-club answers with nothing new having learned the rules. The
statistics are written and validated; what is still missing is tagged swings — every swing on disk
predates the field, so every per-club narrowing is currently, and honestly, empty until the retag
control or a bay session fills it.

**Why it ran before the bay session rather than after.** It was the one substantial item on this
board needing neither a bay nor an `n`, and the ordering mattered in one direction only: a session
hit *without* club tags produces data that can never be split by club afterwards. Tagging was the
cheapest thing here and the only one unrecoverable if skipped — which is why **setting the club
cursor is now a preflight step**, not an optional one.

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
under — and P8 is the first place it is observable: `scripts/career_dispersion.py` now prints both
distances at `n = 2 over 2 sessions`, both claims waiting on their sample floors, with the reason no
target exists printed rather than the metric going quietly absent.

**P20 closed the milestone** on 2026-08-22 by reconciling the documentation with fifteen phases of
code — and its finding was that `tests/test_docs_truth.py` was fully green the whole time, so
`ARCHITECTURE.md` §4 stayed wrong for fifteen consecutive phases with a note on each one saying
so. Three pins went in with the prose fix: the MCP tool count derived from `TOOL_DESCRIPTIONS`
(P18 added two tools and left six sites claiming eight), every route in `api/app.py` appearing in
a route table that did not previously exist anywhere in the repo, and this file's phase count
agreeing with the documentation map's.

**P14–P18 built the per-club answer and both readers of it.** P14 is the shape (`BagProfile` /
`ClubProfile`, with `n_swings` and `n_shots` kept apart because a clip filmed without a screen photo
is history that cannot carry a distance), P15 the builder, P16 the sentence a club whose bag entry
postdates its swings carries, **P17 the CLI** (`python scripts/club_profile.py`) and **P18 the MCP
tools** — `get_bag_profile` and `get_club_profile`, registry-gated beside the career three. Their
output today is one step behind even a refusal, and correctly so — with no swing tagged, no club
gets a row to refuse in, so both readers report the untagged count and where a tag comes from
instead. The refusal table turns on with the first tagged swing and the numbers with the fifth.

**P18's own finding is that "no numbers" has five spellings**, and they need different answers: no
club tagged at all (tag them), a club whose figures are withheld (hit it), a club never hit and not
in the bag (nothing is wrong), a club declared today (nothing was claimed, so nothing was refused)
and swings that carried no measurement (not about the club). The first cut gave four of them the
refusal sentence, which is how a golfer gets sent to the bay to fix a swing that was never
analyzed. `mcp/club.py` names one constant per silence and `caveats.READING_A_BAG` teaches the
distinction once at the top of the conversation.

---

## M10: Alignment accuracy — the two panels leave address together — done

**Design**: [ADR-015](docs/decisions/015-handheld-two-phone-capture-and-event-anchored-alignment.md).
**Phase list**: [docs/M10_ALIGNMENT_ACCURACY.md](docs/M10_ALIGNMENT_ACCURACY.md) — 10 phases, each
independently commit-ready. **All 10 built**: the whole Group B track (P1-P3), so the degraded fallback
and the cross-check now both work in seconds, and the whole Group A track (P4-P8) — the
down-the-line view is selected on the trail wrist its anchors were always segmented on, the window
always keeps a quiet address in front of the takeaway, and the two views no longer choose their
swings independently. Measured over the 15 stored bundles, six down-the-line windows move onto the
real swing and no seventh moves at all. P9 closed the paperwork: ADR-015's first addendum, and
`ANALYSIS_VERSION` 10 → 11.

**P10 ran on 2026-08-26 and the corpus is on the fix.** All 15 bundles were re-analysed and
re-rendered in one pass; every stored result is current, and `career_corpus.py` counts 13 distinct
swings over 21 metrics with nothing excluded as `OUTDATED`. Over the eleven bundles from
2026-08-23: windows that pointed at no swing at all went 4 → 1, face-on motion starts that were
never detected went 3 → 0, and the worst tau=0 disagreement went 0.300 s → 0.167 s. Four scores
moved and eleven did not.

**The tier count got slightly worse, and that is the point.** Sessions 9 and 6 were shipping 0.233 s
and 0.200 s of visible drift while reporting `full`; they now degrade to `impact_only` and
`top_impact` and name the reason in their notes, while session 10 — the worst offender, windowed on
the whole clip — is the one that came *up* to `full`. No bundle claims an alignment it does not
have, which is what this milestone set out to buy.

**The gap that started it, in one sentence.** The side-by-side `aligned.mp4` opened with the
down-the-line panel already into its takeaway while the face-on panel was still standing at
address — visibly, on the bundles on disk, and by an amount worth a quarter of a second. Both
renders nominated for an eyeball check now leave address together, and the phase list records the
frames.

**What M10 left behind, for whoever opens the next milestone.** On four bundles the two views
disagree about the downswing in the same direction every time — face-on measures 0.183-0.267 s
where down-the-line measures 0.384-0.484 s of the same swing — so the face-on top is landing late,
which is the class `_DRAWDOWN_FLOOR` was fitted against and did not finish. It costs three `tempo`
readings that now score and fail at 4.92, 6.08 and 6.09:1, and it is why one bundle still cannot be
windowed down-the-line. Those three failures are **not** coaching truth and the phase list says so
beside them.

**That handoff was taken up and closed on 2026-08-29** by
[§M11](#m11-acoustic-sync--the-ball-strike-is-the-clock--done), which found the disagreement on
*seven* bundles rather than four once a measured impact was under it, withdrew every `tempo` timed
from a contradicted top, and gave `2026-08-23/5` the down-the-line window this paragraph says it
could not have. The numbers above are M10's and are left as they were measured on 2026-08-26; read
M11 P9's *As built* for what is on disk now.

**It was not one bug, and the obvious suspect was innocent.** `analysis/alignment.py` was largely
right: it detected the trouble, refused the soft anchor and wrote an accurate note. The defects
divided in two. **Group A** was upstream in `analysis/phases.py` — the down-the-line swing was
selected with the *lead* wrist while its anchors were measured on the *trail* wrist, and the two
views chose their swings independently, so a clip could be windowed on a practice swing or not
windowed at all. **Group B** was inside the alignment itself — both the degraded fallback and the
tempo cross-check compared **ratios** where they had to compare **durations**, and a ratio divides
out the very quantity that is wrong. Group B is why bundles reported `full` were misaligned too,
which is the part a reader would otherwise not expect.

**Why the fix cost a re-analysis, not just a re-render.** The window decides which frames get
*scored*, so correcting it moved checkpoint scores on the stored bundles as well as the video.
That was intended — a bundle scored on a practice swing was scored on the wrong thing — and it
landed on four of the fifteen: three fell because `tempo` stopped being unscored and started
failing, and one rose because the bundle that had been scored over its whole clip finally got a
window. P10 records both directions.

---

## M11: Acoustic sync — the ball strike is the clock — done

**Design**: [ADR-025](docs/decisions/025-acoustic-synchronization.md), accepted 2026-08-29. It
takes [ADR-015](docs/decisions/015-handheld-two-phone-capture-and-event-anchored-alignment.md)'s
**Option C**, which that ADR parked rather than rejected — *"the strongest alternative here…
Revisit only with a concrete need."* Of its three objections, one **did not hold** (the upload path
preserves audio, measured 30/30) and two did; ADR-015 now carries a second addendum recording that
its "there is no shared clock" premise has an exception.
**Phase list**: [docs/M11_ACOUSTIC_SYNC.md](docs/M11_ACOUSTIC_SYNC.md) — 10 phases, tier REFERENCE.
**10/10 built**, closed 2026-08-29 — P9, the corpus re-run, whose *As built* is the before/after
table and the honest account of what is left; P8, ADR-025 and `ANALYSIS_VERSION` 12, which took
P7's deferred question with it: a `tempo` timed from a top the other view contradicts is **withdrawn** into `unscored` with a
new `CROSS_VIEW_CONTRADICTED` reason, rather than shipped as the failing score M10 P10 called not
coaching truth; P7, arbitrating the late top on the shared clock; P6, `SYNCHRONIZED` and impact as a measured anchor; P5, the ball strike in swing selection; P4, `audio_for`, the cached per-view read; P3, strike detection and the clip-to-clip offset; P2, the `AudioFile` contract and its storage reader; P1, the `audio` extra and the decode port; P0, the verified baseline.

**The concrete need is what M10 handed over.** Both views infer the swing independently, from pose,
and nothing external adjudicates — so on four bundles the face-on top lands late, three `tempo`
readings score and fail on a denominator the other view contradicts, and one bundle still cannot be
windowed down-the-line. ADR-015 built the normalized `tau` axis on the premise that "there is no
shared clock", and M10's addendum then recorded that fps had entered the design in five places
anyway, because *the anchors need real time even though the warp does not*.

**There is a shared clock and it has been on disk all along.** Both phones record the ball strike.
Verified 2026-08-29 by parsing the containers directly: **30/30 stored clips carry `mp4a` audio at
48 kHz**, which settles the one assumption ADR-015 could not check. Also found, and the reason the
decode path needs a real demuxer: the audio edit lists are **non-uniform** — most tracks carry a
2112-sample encoder-priming offset (44 ms, 2.64 frames at 60 fps) and at least one carries none, so
the bias does not cancel between the two views.

**P0 then found a second defect, larger than the first.** Decoding real clips and cross-correlating
the two views' audio shows that on four of the eleven 2026-08-23 bundles the **impact anchor** — the
one alignment treats as reliable — is 5–7 frames out between the views, with down-the-line
consistently early. It is a partly different set of bundles from the late-top four (only one is in
both), it includes a bundle currently reported at the `full` tier, and a container A/V bias, a
variable frame rate and sound travel were each measured and excluded as the cause. Details in
[§E4](docs/M11_ACOUSTIC_SYNC.md); P6 and P7 were written before it was known.

**Three things it buys, in order of confidence.** *Selection* — a rehearsal makes no crack, which is
a far better discriminator than the duration band whose own comment calls its margin thin.
*Synchronization* — a measured impact in both views, and a new `SYNCHRONIZED` tier above `full`,
because a shared clock is different in kind from three inferred anchors. *Arbitration* — with one
clock the two views' tops become comparable, which is what finally closes the late-top defect.

**What it does not do.** Audio gives impact, not the top; it makes the top *decidable*, not
detected. It is not calibration, so no down-the-line checkpoint becomes scoreable and ADR-015's
Option D stays unreachable. And the `tau` axis stays — M10's addendum already drew the line this
milestone works along.

**Costs, paid**: one new extra (`audio`, on `imageio-ffmpeg`), an `ANALYSIS_VERSION` bump 11 → 12,
and a corpus re-analysis that moved scores — the same shape of change M10 P10 was, and for the same
reason.

**What the re-run found**, 2026-08-29, 15/15 bundles re-analysed and re-rendered: **every bundle
reads `synchronized`** and 30/30 clips are pinned to a strike they heard, four of them shot weeks
before this was designed. The late face-on top is on **seven** of the eleven 2026-08-23 bundles
rather than the three M10 handed over — a measured impact *widens* the disagreement, which is how
1, 7, 9 and 11 joined it — so seven `tempo` scores are withdrawn into `unscored`, including the two
that were *passing* at 4.27 and 4.06:1 on the same short downswing that made the others fail.
`2026-08-23/4` reads 100.00 where it read 88.50 because a wrong score left, not because the swing
improved; session 5 got the down-the-line window it never had; and the false *"the two clips are
showing DIFFERENT swings"* note is gone corpus-wide, replaced by *"one swing filmed twice"*.

**The residual P9 handed forward is closed, and it was two errors rather than one** *(2026-08-30,
P10 and P11)*. §Verification's "watch two renders by eye" was finally run and `9` failed it — its
two panels struck about four output frames apart. Underneath were an anchor taking a quiet transient
2–3 frames ahead of the ball on *every* clip, and, on the four down-the-line clips carrying §E2's
video edit list, a presentation offset of 105–125 ms that the audio decode honours and a frame
counter does not. They ran in opposite directions and partly cancelled, which is why neither showed
up in a score and why P9's floor made those four renders *worse* when it was tried alone. **P10**
measures the video's own presentation start through the demuxer and subtracts it before a sample
index becomes a frame; **P11** lands the floor. Ten clips have now been read frame by frame in both
views — including `1` and `/11`, which nobody had checked — and the anchor lands on contact in every
one. The corpus re-run (`ANALYSIS_VERSION` 13, `AUDIO_DETECTOR_VERSION` 2) moved an anchor on eight
of fifteen bundles and no window at all, and it **returned** two withdrawn `tempo` readings: on `1`
and `/11` the cross-view contradiction was the mis-registered anchor, not the swing, and both now
score as honest failures (2.61 and 2.50:1). Bundle `9`'s panels strike on one output frame. Frame
numbers in §Addendum, P10 and P11 of the phase doc.

---

## M12: Club specs — the golfer names a club, the program determines what it is — done

**Design**: [ADR-026](docs/decisions/026-club-specification-lookup.md), accepted 2026-08-31. It
**reverses** [ADR-024](docs/decisions/024-per-club-shot-history.md) §2's *"never a catalogue
default"* rule for loft, and ADR-024 carries a second addendum saying so — read that rather than §2,
which is now history.
**Phase list**: [docs/M12_CLUB_SPECS.md](docs/M12_CLUB_SPECS.md) — 8 phases, tier REFERENCE.
**8/8 built, closed 2026-09-01**: P0, which wrote ADR-026, ADR-024's addendum and the milestone
doc; P1, which landed `contracts/club_spec.py` — the field list, the two shaft enums and their
refusing parsers; P2, which made `BagEntry` inherit it, so **the bag entry is the whole
specification** rather than five fields beside one; P3, `clubs/catalogue.py` — the committed
catalogue keyed so that "T150", "T-150" and "t 150" are one club, which is what makes the second
lookup of an accepted club offline; P4, `clubs/lookup.py` — the model call, with its schema walked
off `ClubSpec` rather than written beside it; P5, `POST /api/clubs/lookup` plus a `BagEntryRequest`
carrying the whole spec, which composes all three; P6, the bag page's search, confirm and set
fan-out; and P7, the docs — `clubs/` in `docs/ARCHITECTURE.md` §2's import map, the catalogue in
§4's storage table, and this milestone's tier flipped from TARGET to REFERENCE.

**The path has been walked, and there is a bag on disk.** `data/processed/golfers/aaron.bag.json`
holds seven irons declared through the page on 2026-09-01 — the first bag this repo has ever held —
and `club_catalogue.json` holds the seven rows they taught it. Through the page: **20.1 s** for one
club, **0.3 s** for the same club again from the catalogue, **49.3 s** for a 4i–PW set in one call.
What is still unverified is layout — P6 was driven headlessly, so nobody has yet looked at a
twenty-seven-field form on a phone.

**P4 and P5 measured the thing this milestone was built on, and the second reading corrects the
first.** P4's four lookups had the model fill a **driver's** loft (10.5°, with a real 8.5–12.5
adjustable range) and a **wedge's** (56°) while refusing every iron loft, and concluded it will not
recite an iron spec chart. P5's two calls through the route show something narrower: asked for a
single T150 7 iron it answers **32.0°**, hedged in its own note as a recollection *"worth confirming
against Titleist's published chart"*; asked for 4i–PW in one call it refuses all seven, *"will not
interpolate"*. **It declines to invent a progression and will hedge a single number.**

So ADR-026 §1's *book loft versus nothing* holds better than P4 recorded, and §6 is working in both
directions — a hallucinated seven-slot loft chart is exactly what it exists to stop. It also makes
the confirm step worth more, not less: the number arrives with a written request to check it. What
the lookup fills besides loft is lie, length, head type, set composition, shaft model and material,
grip and `usga_conforming`, with the set call's lie and length progressions internally consistent.
Two lookups of one club have now disagreed on `lie_deg` twice, which is the argument for the
catalogue. Details, including the four undocumented schema limits P4 paid for, are in both phases'
*As built*.

**The bag was a blank form and it stayed blank for a milestone.** M9 built the whole path —
`contracts/bag.py`, `storage/bag_store.py`, the write route, the row form on the career page — and
it works. It had simply never been used: `data/processed/golfers/` held `aaron.golfer.json` and
**no `.bag.json` at all** until P6. That was not a defect in any of those parts. It is what a
five-field form asking a golfer for numbers they do not have to hand produces.

**And five fields are not what a club is.** `BagEntry` carries loft, make, model, shaft and length,
where `shaft` is one free-text string standing in for six independent facts. A bag filled in
perfectly, exactly as the shape allows, still cannot say whether a 7 iron is a 30.5° players iron on
a 120 g steel shaft or a 27° game-improvement iron on 60 g graphite — two clubs that produce
different ball flights from the same swing.

**The move is to stop asking and start looking up.** A club's *specification*, unlike the club *tag*
ADR-024 had to ask a human for, is a published property of a manufactured object — nothing in this
bay can measure it, and nothing needs to. The golfer types "Titleist T150, 7 iron", the program
determines the rest, and the golfer confirms it. Looking up and saving stay two acts, because what
comes back is a model's proposal and confirming it is what makes it a declaration.

**Loft reverses, and that is the one thing here most likely to be implemented backwards.**
`bag.py:74` says *"Measured loft. None means unmeasured, and never a catalogue default"* and ADR-024
§2 argues for it. Both are retired. The original call applied ADR-010 §2 correctly but compared the
wrong pair: it assumed *book loft versus measured loft*, and the choice a golfer with no loft machine
actually faces is **book loft versus nothing** — which has won for a full milestone. ADR-010 §2 is
not weakened; a spec the model will not commit to still comes back `None`, and blank renders blank
rather than zero.

**What it costs, on the record**: a club bent 2° strong reads its book loft and nothing downstream
knows. Accepted, not solved — the field is editable, and a separate measured-loft field is deferred
to the day someone wants the difference modelled.

**No `ANALYSIS_VERSION` bump and no corpus re-run.** Nothing here changes how a swing is scored, so
unlike M10 and M11 this milestone owes no re-analysis. It also does not turn anything on: every
per-club statistic stays silent at the same floors, because no swing on disk is tagged yet. What it
buys is that the **next** bay session's data is worth more than the last one's — the same argument
M9 was built on, applied to the club's specification instead of its name.

**Deliberately out of scope**: ball trajectory, swing efficiency, gapping and club fitting. ADR-024
defers all four and ADR-026 keeps them deferred; the flight model additionally sits behind
`spin_axis`, which the HD Golf screen prints with no direction word (ADR-014's addendum records two
fades stored as draws). M12 lands the inputs, which are the half that cannot be recovered later.

> **Addendum (2026-09-04, M15):** **ball trajectory is now in scope** —
> [ADR-027](docs/decisions/027-ball-flight-simulation.md) lifts that one deferral and leaves the
> other three exactly where this paragraph put them. Two corrections to the sentence above, and the
> second is the useful one.
>
> **The `spin_axis` blocker is answered rather than waived.** The screen prints no direction word on
> the *numeric* tile, which is true and is what ADR-014's addendum records. It prints one on the
> **`Shot Type`** tile — `FADE`, `DRAW`, `SLIGHT FADE` — on every one of the thirteen stored shots,
> and `analysis/shot_measure.py`'s `normalize_shot_shape` has been parsing it since M6.5. The sign
> was on the screen the whole time, in a tile nobody had thought to read for it.
>
> **And the blocker was the wrong field anyway.** `spin_axis` is missing from 11 of the 13 stored
> shots, but so is `spin_rate`, and the rate is the one that decides the carry — the whole
> 2026-08-23 bay session records `Spin: no value text under the label`. So what actually blocked the
> flight model was **spin, not the axis and not loft**, and ADR-027 §3 recovers the rate by solving
> the integrator backwards against the printed carry, on the record that this fits our model to HD
> Golf's rather than to reality.
>
> **What this paragraph got right, and it is the load-bearing half**: M12 landed the inputs. Ball
> speed and launch angle have been stored since M9 against a model nobody had scheduled, and M15 is
> the milestone that collects on that ordering. **What it implied and should not have**: that the
> club specification is what the flight model was waiting for. It is not — loft and lie are inputs
> to the *impact* model, not to ball flight, so M15's correction of the 7 iron from a T150 to a T250
> changes no simulated flight. ADR-027 §Context 1 spends its words on that, because it is the
> assumption most likely to be re-introduced by the next reader.

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
      5k-character briefing delivered), `list_tools` returned every tool then registered with its
      schema, and
      `call_tool` served `list_sessions`, `get_swing`, `get_session_summary`, `get_recent_shots`
      and a deliberate miss — the `NotFound` shape survives the wire as a normal result rather
      than a protocol error, which is what it was designed to do. Registered with Claude Code via
      `claude mcp add`; `claude mcp list` reports `✔ Connected`
- [x] **Ask follow-up questions about a swing** *(2026-08-15, [ADR-020](docs/decisions/020-conversational-followups.md))* —
      the SDK tool runner over `mcp/query.py`'s functions directly, exactly as this line predicted;
      the stdio round trip stays for *external* clients. A conversation seeds from the swing's
      stored brief and looks everything else up through the same eleven tools. Transcripts live in
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

# The app — from an offline pipeline to a live product (M18–M27)

Planned 2026-09-21 in one sitting and **re-decided the same day**, so that each milestone below can
be pinned down in its own fresh session. **The ask:** an installed app for Windows, Linux, macOS
and — later — a phone. The golfer connects two cameras, verifies them, positions them and starts a
session; the cameras record continuously, a ball strike triggers a ±5 s clip from each, every pair
is analysed in the background while recording carries on, and each result updates the golfer's
profile. (The numbering is M18 because M14–M17 were already taken.)

This group is kept in one place although its members are in different states — M18 is done, M19–M21
are startable, M22–M26 wait on them — because reading it in pieces across *Next* and *Blocked*
would lose the order. The state is in each header and in the status table.

**The premise changed once, and it changed everything below it.** The first version of this group
assumed *two iPhones must be enough on their own*, one recording and the other recording **and**
analysing. That single assumption drove most of its content, because an iPhone cannot run
MediaPipe-Python, OpenCV, pydantic-core or ffmpeg: it forced a Dart-versus-Rust bake-off, a
MediaPipe Tasks iOS spike, and the standing risk that a different pose pipeline would mean
**re-fitting every band in `ranges.json`**. The premise is now that the machine is a **laptop** and
the phone is a **camera**. That deleted the pose risk and the spike programme with it.

**Decided, in [ADR-030](docs/decisions/030-app-platform-rust-core-python-sidecar.md).** Read the ADR
for the reasoning; the short form is:

- **The backend is Rust** — app process, capture, trigger, analysis engine, storage.
  **Supersedes [ADR-001](docs/decisions/001-language-python.md)**, the first superseded ADR here.
- **Pose stays MediaPipe-in-Python, unchanged**, driven by a long-lived **worker pool** the Rust
  core sends jobs to. Jobs carry a clip path and a frame range, never pixels. `ranges.json` is
  therefore untouched and needs no re-validation — that is the entire point of the split.
- **Python also keeps** OCR, LLM coaching and the whole lab: fitting under the `research` extra
  ([ADR-022](docs/decisions/022-learned-artifacts-as-committed-data.md) is unaffected), the corpus
  tools and the conformance oracle. It ships as a **bundled sidecar**, not as the app.
- **The shell is Flutter** over the Rust core via `flutter_rust_bridge`. **No web UI** — M5 is
  superseded in shape and `api/static/`'s five pages stay a lab surface.
- **The phone app comes after the laptop app works**, and carries capture and strike detection but
  never pose. Supersedes [ADR-016](docs/decisions/016-local-first-host-and-phone-upload-topology.md)'s
  "no phone app" clause and nothing else in it.
- **Capture sources are pluggable and file upload is the first one** — it works today with no
  hardware, and stays first-class after cameras exist.
- **Cloud analysis is closed**, not deferred. M27 records why.

**What the code already gives.** `capture/source.py::VideoSource` and `audio/source.py::AudioSource`
are ports whose docstrings name a live adapter that was never built. `api/pipeline.py` already picks
the swing out of a longer clip (`select_swing`, `_auto_windows`). `analysis/` is stdlib-only over
stored JSON and every band and model ships as JSON, which is the seam ADR-008 and ADR-022 drew and
**the reason the Rust port is affordable at all** — the swing loop is roughly a thousand lines of
logic that actually executes, not the ~5,600 the import graph suggests. The per-swing directory (two
clips and a manifest) is the unit of work, and the golfer's profile is recomputed on read from
`storage/corpus.py::read_corpus`, so a new result shows up without an invalidation step.

**What it does not.** No live camera, microphone, ring buffer or device enumeration; a "session" is
the UTC date. `audio/impact.py::detect_strikes` is offline over a whole decoded clip, tuned on one
golfer, one bay and iPhone audio, and a bay makes about four transients per shot. `AnalysisWorker`
runs one job at a time with no timeout or retry, in one process, and the UI polls. A bundle needs a
shot photo and a club cursor before it analyses, and both cameras must target one `swing_id`. There
is no packaging and no CI, `config.py::REPO_ROOT` assumes a source checkout, and calibration for
real 3-D (the printed fiducial squares) is still only a seam in `contracts/pivots.py`.

**Order.** M19, M20 and M21's first source need no Rust and no hardware, so they start now. M22 is
gated on M19 — a Rust core and a Python core that disagree silently is the failure mode this whole
plan has to survive, and only a conformance suite catches it. M23–M26 follow in a line.

> **M19 closed 2026-09-21**, so **M22 is open** and the gate it was waiting for exists:
> `python scripts/conformance.py check`. M20 and M21's file source are still startable in
> parallel and still need no Rust. Read [docs/CONFORMANCE.md](docs/CONFORMANCE.md) before M22 —
> in particular §3's two Rust-specific edges, which are the difference between a numeric failure
> that is a bug and one that is a language.

---

## M18: The platform decided — a Rust core, a Python pose sidecar and a Flutter shell

**Status**: ✅ Done *(2026-09-21)*. Docs only — no source, no tests.

**What happened.** This milestone was five spikes ending in an ADR. The spikes were sized against
the two-iPhone premise; when that premise was dropped for a laptop, the questions they existed to
answer stopped being open. The decisions were taken directly and the milestone became its own
deliverable: [ADR-030](docs/decisions/030-app-platform-rust-core-python-sidecar.md), an addendum
each on [ADR-002](docs/decisions/002-pose-estimation-mediapipe.md) and
[ADR-016](docs/decisions/016-local-first-host-and-phone-upload-topology.md),
[ADR-001](docs/decisions/001-language-python.md) marked superseded, the charter's "Mobile app" line
moved into scope, and this group rewritten.

**The finding worth carrying forward** is which decision was actually load-bearing. It was not the
language and not the shell — it was **where pose runs**. `ranges.json` is cut from MediaPipe's
landmark output, and ADR-002 had already measured what a different pose pipeline costs: RTMPose
lost by 24.7pp on event recovery and ran 4.3x noisier at the hip. Keeping pose in Python is what
lets everything else be rewritten without putting the scoring model back in question. Every other
choice here is reversible; that one would not have been.

**Not done, on purpose.** No spike report doc — there are no measurements to hold, and a new living
doc would pull in a tier banner and a map row for prose the ADR already carries. No ADR for the
trigger or the capture edge: those are M20 and M21 and they have evidence to gather first. No
ADR-011 addendum — camera topology is genuinely unaffected, the same disclaimer ADR-016 made.

---

## M19: The Python core becomes a specification — schemas, golden vectors and a conformance runner

**Status**: ✅ Done *(2026-09-21)*, 5/5 phases. Desk work over artifacts already on disk. **M22 is
unblocked.** The document is [docs/CONFORMANCE.md](docs/CONFORMANCE.md); the artifacts are
[`spec/`](spec/README.md); the runner is `scripts/conformance.py`.

**The ask.** Make the existing implementation something a port can be checked against with one
command. Today the only oracle is the Python test suite, which cannot be run by a different
language — and ADR-030 has committed to a second implementation of the swing loop.

**Promoted from "useful" to "first".** A Rust core and a Python core that disagree silently is the
failure mode the whole platform decision has to survive, and review does not catch a drift of a
fraction of a unit. This must exist before M22 starts, not alongside it.

**The phases, as built.**
- **P1 schemas** — five roots exported to `spec/schemas/` from
  `conformance.py::SCHEMA_ROOTS`: `KeypointsFile`, `AudioFile`, `ShotData`, `SwingResult`,
  `SwingBundleResult`. The rule for adding one is that *something other than Python parses it*,
  which is why the list is shorter than `contracts/`. `SwingManifest` was in the original list and
  is not exported: nothing outside Python reads it, and a schema is a promise to hold a shape
  still.
- **P2 golden vectors** — 21 in `spec/vectors/`: **6 synthetic** from
  `tests/analysis/conftest.py::make_swing` (uncompressed and readable, each one there for a code
  path) and **15 corpus**, gzipped, one per stored swing. The storage question the plan left open
  was settled by measurement: the full keypoint set is 239 MB, sliced to the scored window 25.6 MB,
  gzipped **8.3 MB**, which commits. A further cut to 3.0 MB — `analysis/` names 15 of the 33
  landmarks and reads `z` on none — was declined, because a reduced file is no longer a
  `KeypointsFile` a port can parse with the shipped schema.
- **P3 tolerance rules** — `compare_results`, and both predicted edges confirmed in shipped code.
  Floats admit `RTOL = 1e-9` (a different summation order, nothing wider); everything else is
  exact, `bool` **before** `int` because `isinstance(True, int)` is true in Python, and a `None` is
  compared to nothing but a `None`.
- **P4 `scripts/conformance.py`** — `check`, `run` (stdin/stdout, the cross-language seam), `list`,
  `regenerate`. `check` runs the whole suite in ~7 s off a base install and `spec/` alone.
- **P5 inventory** — [CONFORMANCE.md §5](docs/CONFORMANCE.md#5-what-ships-in-the-app-and-what-stays-in-the-lab),
  four tiers plus the stub, module by module.

**The two findings worth carrying to M22.**

1. **The serialization a port must match is not in any schema.** `api/pipeline.py` writes
   `analysis.json` as `model_dump_json(exclude={"swing": {"keypoints", "detections"}})` — the
   exclusion is a *call-site* decision, so a port that implemented `SwingBundleResult` exactly as
   the schema describes it would emit the whole keypoint list and differ on a field nobody meant to
   compare. `conformance.EXCLUDED_FROM_RESULT` is the one copy, pinned against the literal in
   `pipeline.py` by a test that reads it back out of the source.
2. **A bare engine call does not produce the artifact this repo writes.** `analysis` may not import
   `feedback` (ADR-008), so `analyze_swing_bundle` leaves `SwingBundleResult.feedback` as None and
   `api/pipeline.py` fills it a line later. The first build of the vectors went through the engine
   alone and pinned `"feedback": null` on all twenty-one — a specification instructing a port to
   ship a results page with no coaching on it. `run_vector` now makes both calls, and every vector
   reproduces the archive's ranked tips.

The build also caught its own first reconstruction error, which is the reason
`_verify_against_stored` exists: recovering a swing's declared loft from its own result works only
where the flight *succeeded*, because a refusal leaves no `club_loft_deg` measurement behind — so
`2026-08-23/2` silently rebuilt as `None` and produced a refusal sentence blaming a missing loft
the original run had. Both arguments `analysis` is forbidden to fetch for itself now come from the
registry and the bag, the way the shell fetches them.

**Reused, not rebuilt:** `make_swing`, the stored `*.audio.json` strike frames (`detect_strikes`
has no reproducible oracle in the suite — `tests/audio/test_impact.py` synthesizes from a seeded
numpy RNG), `scripts/reanalyze.py` as the precondition, and `api/pipeline.py`'s own resolution of
handedness and loft.

**What it does not cover**, named rather than left to be found: `feedback/coach.py`'s LLM call
(non-deterministic and pinned *absent*), `audio/impact.py` end to end (the vectors take strike
frames as an input), pose itself (ADR-030 §3 pins the sidecar against the 30 keypoint files
instead), and wall-clock performance.

**The standing obligation this creates:** an `ANALYSIS_VERSION` bump must regenerate the vectors in
the same change. `tests/test_conformance.py` fails until it does, and `CLAUDE.md` carries it as an
invariant.

**Exit Criteria**: A port can be diffed against the Python reference with one command, and the
schemas and vectors are in the repo. ✅ — `python scripts/conformance.py check` → 21/21 at v16.

---

## M20: The trigger — hearing the ball strike live and cutting the clip

**Status**: ⬜ Not started. The algorithm is startable now: it is replayed over the stored clips.
The false-positive check needs one bay trip.

**The ask.** Turn "the cameras always record and the program notices the ball being hit" into a
specified, measured algorithm, in Python first, so the code that implements it implements a spec and
not a hunch.

**Doubly load-bearing now.** Under ADR-030 this spec is implemented twice over: it is the Rust
core's trigger on the laptop, **and** it is what runs on the phone so that a phone sends triggered
clips instead of a video stream. Getting it wrong is a bandwidth decision as well as a correctness
one.

**The phases.**
- **P1 an online detector** — the spec and a Python reference: block-wise spectral flux, a rolling
  robust-z floor (ADR-013's clip-relative principle applied to a moving window), a refractory
  period that covers the ball → mat → screen burst (85–145 ms apart) and the simulator's own
  audio, and the earliest-wins rule from `analysis/alignment.py::with_measured_impact`.
- **P2 a replay harness** — stream the stored clips' audio through it, and concatenations of them
  with silence and voices in between; report precision, recall and how late the decision comes.
- **P3 ring-buffer and cutting rules** — five seconds either side against what the pipeline needs
  (`window_around` wants at least 1.5 s of quiet address and three downswing-lengths after impact),
  two swings closer than ten seconds, a practice swing with no strike, and a maximum clip.
- **P4 a continuous recording** — 30–60 minutes at a bay with phones just recording, for false
  positives; fold it into `docs/BAY_SESSION_RUNBOOK.md`.

**Reuses.** `audio/impact.py`, `contracts/audio.py::AudioStrike`, `AUDIO_DETECTOR_VERSION`.
**Risks:** voices, a neighbouring bay, simulator sound; the thresholds were tuned on one golfer, one
bay and iPhone-AAC audio, and "voice" was only synthetic modulated noise; a USB webcam's microphone
may not hear the strike at all. The detector is also numerically fussy — clip-relative by ADR-013,
dependent on numpy's `hanning` specifically, and carrying a calibrated `+1` sample convention — so
the spec must state what a reimplementation has to match and to what tolerance.

**Exit Criteria**: Measured precision and recall over the stored clips and one continuous
recording, and a written spec the capture edge implements.

---

## M21: Capture edge — a file, then a webcam, then a phone

**Status**: ⬜ Not started. **The first source is startable now and needs no hardware and no Rust.**

**The ask.** `capture/source.py::VideoSource` is already a `Protocol` with `FileVideoSource` built
and `LiveCameraSource` never written. Keep that shape and fill it in, in this order:

1. **File / upload** — works today, needs nothing bought, and is the only source that can replay the
   stored corpus through the new stack. **First-class, not a test fixture**: it must still work once
   the other two exist, because a golfer with footage and no rig is a supported case.
2. **USB / UVC webcam** on the laptop — Media Foundation on Windows, V4L2 on Linux, AVFoundation on
   macOS.
3. **Phone over Wi-Fi**, sending encoded clips its own detector triggered (ADR-030 §5), not a
   stream.

**Tasks.** Enumerate and verify devices; preview; microphone; a ring buffer with host-clock
timestamps; the camera-to-microphone offset (M11 P10's `video_start_s` was a 105–125 ms surprise on
four clips, so it is measured here, not assumed); dropped frames; thermal and storage budgets. The
`SYNCHRONIZED` alignment tier needs each view's audio and video presented against one clock.

**Questions for its own chat:** whether a USB webcam's microphone is usable or a host microphone is
muxed into both clips; what continuous background recording costs an iPhone thermally.

**Exit Criteria**: A stored clip replays end to end through the new stack from the file source, and
two webcams record continuously with a trigger from M20's spec cutting two clips a bundle can be
built from.

---

## M22: The Rust core — the analysis engine passing the conformance suite

**Status**: ⬜ Not started, **unblocked 2026-09-21**. The language is settled (ADR-030) and the
oracle exists: read [docs/CONFORMANCE.md](docs/CONFORMANCE.md) before writing a line of Rust, and
§3's two known edges — banker's rounding, and `%g` formatting reaching the sentences a golfer is
shown — before assuming a numeric failure is a numeric bug. `spec/vectors/synthetic/` is where to
start; `make_swing` re-implements in Rust exactly, so a port can generate its own inputs.

**The ask.** Port the engine to Rust, in dependency order — contracts → measure → phases →
checkpoints and scoring → alignment → audio → benchmark loaders → career and profile → storage —
**each stage gated by M19's conformance suite**, not by review. The swing loop is the first tier;
shots, flight and clubs the second; LLM coaching stays Python (third tier, via the sidecar).

**What makes it affordable.** ADR-008 made `analysis/` stdlib + `contracts` only and ADR-022 made
every model ship as JSON, so the semantic closure is roughly a thousand lines of executing logic.
`ranges.json` and `golfdb_v1.json` port as data with no reimplementation. Prune the import graph
rather than following it: `analysis/benchmarks/__init__.py` re-exports flight, joint and trajectory
models that the checkpoint path never reaches.

**Where it bites.** `contracts/` is pydantic, and its `Field(ge=…, le=…)` constraints are *runtime
validators* — a port that treats them as annotations silently accepts values Python rejects.
`detect_strikes` needs numpy's `hanning` reproduced exactly and is the one piece with no
deterministic oracle in the test suite. The stdlib-only invariant carries over as a rule the Rust
core inherits: no numeric library in the scoring path.

**Open, and for its own chat:** shot-screen OCR stays Python in the sidecar (it was PaddleOCR);
the MCP server stays a lab tool unless something asks for it in the app.

**Exit Criteria**: Tier 1 passes M19's suite byte-for-byte on the exact outputs and within tolerance
on the floats.

---

## M23: The pose sidecar — a long-lived Python worker pool

**Status**: 🔒 Blocked on M22 (something has to send the jobs).

**The ask.** The Rust↔Python boundary ADR-030 §3 specifies, built: a pool of warm worker processes,
each with the interpreter up and the `.task` bundle resident, driven by a job protocol.

**Tasks.** The job envelope (`job_id`, `clip_path`, `frame_range`, `camera_id`,
`pose_model_variant`) and the `KeypointsFile` reply; pool sizing and backpressure; worker lifecycle,
crash detection and restart; timeouts and retry, which `AnalysisWorker` has neither of today;
bundling a standalone Python runtime per OS (`python-build-standalone`); and the model files, which
must ship rather than download on first run.

**The constraint that shapes it:** `RunningMode.VIDEO` carries cross-frame tracking state, so a
worker takes **one clip start to finish** and gets a fresh `PoseLandmarker` between jobs. The warm
thing is the process, not the landmarker. Budget from ADR-002's ~24 fps at `heavy`: a 10 s 60 fps
clip is ~25 s of pose, a two-view swing ~50 s.

**Verification is free and should be used**: the sidecar's output is diffed against the 30 keypoint
files already on disk, which were produced by the same estimator at `mediapipe:heavy`.

**Deferred here**: the shared-memory frame handoff for live preview (ADR-030 §3 specifies it,
nothing needs it yet).

---

## M24: Session engine — start a session and swings flow through to the profile

**Status**: 🔒 Blocked on M21 and M22.

**The ask.** An explicit start and stop in place of the UTC-date session; capture isolated from
analysis so frames are never dropped while pose runs; a queue with a backlog and a thermal budget;
swing ids assigned for both cameras; a bundle that does not need a shot photo; results pushed to
the UI instead of polled; the profile updated as each result returns (it is already recomputed on
read, so this is a notification and not an invalidation).

**Open, and for its own chat:** how shot data arrives in a live flow, since today it is a photo of
the simulator screen taken by hand.

---

## M25: The app — the Flutter shell and the setup wizard

**Status**: 🔒 Blocked on M24.

**The ask.** The workflow the golfer described: connect cameras → verify → position → start
session, then results, history, career and ball-flight screens. Flutter over the Rust core via
`flutter_rust_bridge` (ADR-030 §4). Positioning uses pose-landmark visibility as the "golfer fully
in frame" check, ADR-003's placement guidance, and the fiducial seam.

**Not a port of the HTML pages.** `api/static/`'s five pages stay what they are — a lab surface for
the desk pipeline. **M5's web UI is superseded in shape by this milestone**, not revived by it.

---

## M26: Ship it — CI, packaging, signing and distribution per OS

**Status**: 🔒 Blocked on M25. CI can start as soon as there is a `Cargo.toml` to run it against.

**Tasks.** There is no CI today. Per-user data directories (`config.py::REPO_ROOT` assumes a source
checkout, and every path constant flows from it); the bundled Python sidecar and the pose models
shipped rather than downloaded on first run; signing; MSIX, AppImage and a `.dmg`; an update path;
crash and privacy handling. TestFlight and Google Play only once the phone app exists.

---

## M27: Remote worker — closed, not deferred

**Status**: ❌ **Closed 2026-09-21** by [ADR-030](docs/decisions/030-app-platform-rust-core-python-sidecar.md) §7.
Kept here as the record of a decision, not as a backlog item.

**Why.** Remote analysis for people without a capable laptop brings accounts, authentication,
per-swing compute cost, video privacy and retention, and the exposure of a public endpoint — each a
project rather than a phase — and it contradicts the local-first posture ADR-014 and ADR-016 both
rest on. Reopening it needs a new ADR and a reason.

**The cheaper lever, still untried.** ADR-002 measured the `lite` pose model at about four times
the speed of `heavy` with no significant difference on event recovery across twelve paired McNemar
tests. A slow laptop gets a faster variant and shorter clips; that is the answer until someone
shows it is not enough.

---

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

## M13: Downswing tempo — the downswing is what you feel, the backswing is what you change — done

**Status**: ✅ Done 2026-09-02, 8/8 phases. ADR-023's third addendum records the reversal where a
reader of the 2026-08-20 one will meet it, `TempoPlan` carries the anchor the flip needs, the
trainer reads the downswing and prescribes the backswing, the pace slider now spans the whole range
a measurable downswing can fit to, so the 144% on the swing on disk opens rather than clamping to
140, the career scope selects on the same half it fits to, both pages name the downswing they were
fitted to and offer the snap when it sits outside the tour range, and the tempo verdict names a
backswing target in milliseconds. All desk work — no bay session, no `n`.

**The corpus is on the new sentence and no number moved.** `reanalyze.py --all` re-ran all 15
stored swings (`--dry-run` reported nothing to do, because `ANALYSIS_VERSION` deliberately did not
move and staleness cannot see a changed sentence), and `score`, `passed`, `observed`,
`expected_low`, `expected_high` and `overall_score` came back byte-identical on every one while
every message changed. **What is unverified is layout** — P5's pins run under node, so nobody has
looked at the notice and the snap button in a browser.

**Goal**: The tempo trainer anchors its target to the golfer's **backswing** and derives the
downswing from it at the tour ratio ([ADR-023](docs/decisions/023-tempo-training-and-absolute-swing-durations.md)'s
2026-08-20 addendum). That is the wrong half to hold fixed: the downswing is what a golfer *feels*
— it is how hard they swung — and the backswing is what they can deliberately change. Flip it.
Read the downswing, multiply by the tour ratio, prescribe the backswing, and re-word the tempo
verdict from an abstract ratio into a backswing duration in milliseconds.

On the swing on disk (901 ms back / 384 ms down, 2.35:1) the target moves from *901 back / 267
down* to **1296 back / 384 down**, and the verdict from *"aim for the tour range 2.72–4.71:1"* to
*"your downswing was 384 ms; at the tour ratio that wants a 1044–1808 ms backswing and yours was
901."*

**What it deliberately does not do**: no band on `downswing_ms` — ADR-023 §1 stands, the panel
stays at six checkpoints, `overall_score` is byte-identical and `ANALYSIS_VERSION` does not move.
The whole milestone is one divisor, one deleted guard, three contract fields and some prose.

**The detail most likely to be implemented backwards**: the anchor guard is *deleted*, not flipped.
Keeping the p10–p90 rule and merely swapping the metric refuses this golfer's 384 ms downswing and
hands them the tour median — a milestone that changes nothing observable on either swing on file.
What replaces it is a notice that the downswing sits outside the tour range, plus an opt-in snap to
the nearest in-range value.

Phases, files, tests and the arithmetic are in
[docs/M13_DOWNSWING_TEMPO.md](docs/M13_DOWNSWING_TEMPO.md). P0 wrote ADR-023's third addendum,
P1 landed the contract fields, P2 flipped the anchor, P3 widened the pace control to reach it,
P4 took the career scope with it and P5 — the first phase a golfer can see — put the notice, the
snap and the three anchor sentences on the two pages.

**Exit criteria**, all met: the pace slider opens at the fitted 144% rather than clamping to 140,
the beat strip plays the golfer's own downswing interval, and the tempo checkpoint names a
backswing target in milliseconds — with no row added to `ranges.json`. On the stored swing it reads
*"your downswing was 384 ms; at the tour ratio that wants a 1044-1808 ms backswing, and yours was
868"*.

---

## M14: Hand landmarks — six points nobody reads, and the head dots nobody measures — done

**Status**: ✅ Done 2026-09-03, 6/6 phases. All desk work, on artifacts already on disk — no bay
session, no `n`, no re-capture. P0 wrote the milestone document; P1 made `pose/overlay.py` derive
its dot set from its bone list; P2 placed landmarks 17–22 in the synthetic fixture; **P3 was the
gate and it passed**; P4 and P5 added four measured, unjudged metrics; P6 reconciled the docs.
**No checkpoint, no band, and no `ANALYSIS_VERSION` bump** — so no stored score moved, and nothing
needed re-analysing.

**It began as a rendering complaint and turned into a reopened claim.** The question was "can the
plot points move to better spots — fewer in the head, more in the hands". They cannot: BlazePose's
33-point topology is trained weights, and `pose_model_variant` picks a capacity, not a layout. But
the question decomposed into two real findings. The overlay drew **all 33** landmarks and connected
**twelve**, so eleven head dots and two foot dots were rendered that no bone touched and no
checkpoint measures — a rendering defect, fixed in P1, moving no numbers. And landmarks 17–22 —
the pinkies, index knuckles and thumbs — have been written into **every `.keypoints.json` this repo
has ever stored** and read by nothing. That is what made the screen cheap.

**P3 is the result worth carrying**, and it is the one that corrects this document: see the
addendum under [§Biggest constraint on coaching](#career-mode-one-golfer-tracked-over-time--done-built-and-silent).
Face-on over the address window, all six hand landmarks track in 1.00 of frames on 15 of 15 stored
swings; down-the-line, same script and same swings, the lead hand reads 0.25 and clears the floor
on 4 of 15. §Phase G's finding was about the camera, not the landmark. The three limits stated
alongside it are load-bearing: it is an exclusion floor and not a relevance ranking, the corpus is
15 clips from one golfer against §Phase G's 584 from 166, and face-on over the *whole* clip the
hands fall to 0.63–0.68 — which is why every metric is scoped to address and none is scoped wider.

**Four metrics, all `signal`, none judged.** `hand_separation_norm` and `hand_height_norm` (P4),
`hand_offset_from_hips_norm` and the labelled proxy `trail_hand_roll_deg` (P5), each with a
tolerance in `contracts/dispersion.py` and a `no_target_reason` in place of a target, so each can
carry a **scatter** finding in career mode and none can carry a **bias** one. Declaring a target is
declaring what good is, and this repo does that in exactly one place — a band with a derivation
behind it (ADR-010 §2). The spread/error harness re-run over all 461 face-on GolfDB clips
reproduced every pre-existing tolerance to the digit, three runs running, which is the only
available check that a number derived weeks apart came off the same instrument.

**P5 found the thing nobody went looking for.** `hand_offset_from_hips_norm` came back **bimodal —
26% of 455 clips negative** — which is the camera-relative-sign-over-mixed-handedness warning
`measure.py` has carried since metric definitions v2, firing for the first time. The irony is that
the metric the warning was written for, `head_hip_offset_impact_norm`, was then measured and found
clear of it. A band cut across both modes would sit in the empty middle and read every left-handed
golfer as a gross fault, so that metric has a measured tolerance and no target until handedness is
resolved.

Phases, tables and the three design calls behind the rotation proxy are in
[docs/M14_HAND_LANDMARKS.md](docs/M14_HAND_LANDMARKS.md).

**Exit criteria**, all met: the overlay draws only what a bone connects, the fixture places the
hand landmarks against their own wrist, the reliability screen ran and published its table either
way, and any metric that shipped did so measured and unjudged. **What is unverified is the
overlay in a browser** — stored `aligned.mp4` clips still carry the old dots until re-rendered, and
nobody has looked at a freshly rendered one.

**What this deliberately did not do, and what it sets up.** It is not MediaPipe Hands: the
21-landmark-per-hand model is the only thing that could deliver grip *strength* — the V's, the
knuckle count — and it has never been evaluated here. It needs a `spikes/` probe first on the
`spikes/club-head-detectability/` + ADR-017 pattern, because two interlocked hands wrapped around
a shaft are out of distribution for a model trained on open gesturing hands, and it would force an
**L3** change: a parallel optional field on `FrameKeypoints`, never an extension of the flat
33-list. That is the natural next milestone. Grip *consistency* — whether one golfer sets the hands
the same way twice — now falls out for free once these numbers are stored, and belongs with the
dispersion work rather than here.

> **Addendum (2026-09-03, MediaPipe Hands spike):** "that is the natural next milestone" — the
> probe ran on the day M14 closed, and **it is a no-go, so the milestone does not exist.** The
> prediction in the sentence above is half right and the half it got wrong is the interesting one.
> Two interlocked hands *are* out of distribution, but not because the model cannot see them: at
> the best pose-guided crop it returns a hand on 179 of 179 address frames, on the right wrist,
> with its index knuckle 1.6–3.6 cm from the pose model's own. It returns **one**. Two hands come
> back on 0.15 of frames against a 0.60 floor, and the hand it keeps is the **trail** one (1.00
> against the lead's 0.14) — the wrong one, since grip strength is read on the lead hand. Grip
> *consistency* is unaffected and still falls out of the stored pose metrics for free.
> [spikes/mediapipe-hands/log.md](spikes/mediapipe-hands/log.md).

---

## Hands spike: MediaPipe Hands on a golf grip — M14's successor, and a no-go

**Goal**: before an L3 change — a parallel hand field on `FrameKeypoints`, a `pose_estimator` stamp
bump, every cached pose in `data/processed/` re-run — prove the 21-point hand model resolves a golf
grip at all. Time-boxed, on the M1.5 pattern, and it needed no new footage.

> **Status (2026-09-03): done, and the answer is no-go.** Thresholds were committed before the
> model was run over a frame. Findings:
> [spikes/mediapipe-hands/log.md](spikes/mediapipe-hands/log.md).
>
> **The model does not fail to find a hand; it fails to find two.** Fifteen face-on swings, 179
> frames inside the same address window M14 P3 screened. At the best pose-guided crop it returns a
> hand on **179 of 179** frames, places it on the correct wrist (1.00), gets the handedness label
> right (1.00) and puts the index MCP **1.6–3.6 cm** from the pose model's own index landmark. Two
> hands come back on **0.15** of frames against a 0.60 floor — a golf grip reads as a single hand
> to a model trained on open, gesturing ones — and the hand it keeps is the **trail** hand (1.00)
> while the **lead** hand, the one grip strength is read on, comes back at **0.14**.

- [x] Commit the pass/fail bar first — [thresholds.md](spikes/mediapipe-hands/thresholds.md),
      written before the model ran: two-hand detection ≥ 0.60 (§Phase G's floor, reused),
      placement and distinctness ≥ 0.90 against the pose wrists, jitter ≤ 2.0x raw pose
- [x] Full-frame arm as the control — **failed as predicted**, 0.16 detection and 0.00 two-hand at
      4K, which is what says the crop arm is doing real work
- [x] Pose-guided crop arm, `k` swept over {1.0, 1.5, 2.0, 3.0} — best at **k=2.0**, and no crop
      scale gets two-hand detection above 0.15
- [x] Eyeball the landmarks — `probe.py frames` wrote one annotated crop per swing; **one** hand on
      all fifteen
- [x] Rule out the cheap follow-ups: the **glove** is not the cause (gloved and bare-handed swings
      behave identically), and lowering the detection threshold to 0.2 only reaches 0.35
- [x] Controls: **down-the-line** is worse in every column (0.17 two-hand, 0.62 placement), and
      whole-clip face-on collapses to 0.08 as M14 predicted it would
- [ ] Write the finding into an ADR — **open.** The M1.5 pattern ends in one, and the decision this
      would record is "no camera-only route to grip strength; the V's stay out of scope until a
      hand model is trained on hands holding a club"

**Exit criteria (go/no-go gate)**: ✅ met. The L3 change is **not** authorised, the successor
milestone M14 named does not open, and the residual question is a training effort of the same shape
M1.5 declined for the club head — not a re-run and not a bay session.

---

## M15: Ball flight — the model the launch angle was recorded for — done

**Status**: ✅ Done *(2026-09-06)*, 20/20 phases. **The six measurements are in `analyze_swing`,
the corpus counts them honestly, every stored swing is on engine version 15, the flight is served
over HTTP, a page draws it three ways — including the one a golfer means, from behind the ball —
the page says what each number is and is not, and a model can ask for one and is told what it may
say about it.** Only the page's **layout** is unverified — no browser
was driven at any point, and every payload was replayed against the page's own JavaScript instead.
P0 wrote [ADR-027](docs/decisions/027-ball-flight-simulation.md), this section, the addendum on
M12's out-of-scope paragraph and the map's counts; P1 corrected one bag entry; P2 landed the
constants; P3 landed the integrator; P4 passed the gate; **P5 gave it a third dimension and re-flew
the gate at ±2.59%**; P6 added the altitude what-if and the no-numpy pin; P7 put the first human
surface over them; **P8 solved the flight backwards for its missing spin**; **P9 built the
branch rule and the axis resolution, and found that both refuse every shot on disk**; **P10 pointed
it at the corpus and found that they do not** — the club was on the swing all along; **P11 put the
six measurements into `analyze_swing`**; **P12 registered the prefix that makes the corpus count
them, and found the fallback had been trusting a flight built on a parse it distrusts**; **P13
moved `ANALYSIS_VERSION` to 15 and put every stored swing on it**; **P14 put it behind a route**;
**P15 drew it**; **P16 put the printed numbers beside the simulated ones and found that neither
pair is a check**; **P17 gave a model the eleventh tool and found that the surface which talks had
been shipping the flight as bare floats**; P18 cascaded the docs; **P19 drew the shot the way a golfer sees it**. **All fifteen
phases that ran the model found something
the ADR did not have**, and each is an addendum on it: P2, that the only published coefficient table anyone has put
in the open covers a *driver's* spin ratios while both validation shots are irons flying the whole
way above it — P3, that above that clamp **spin does not reach the flight at all**, which turns the
spin solve from a two-branch problem into a five-case one and makes §Decision 3's inferred spin a
path-drawing device rather than a number — P4, that the gate **passes on each shot while ranking
the two of them backwards**, which is what the percentage conceals — P5, that the gate had been
flying a **planar approximation of a shot that curved**, which moved the pin to 0.0259 and showed
that a tenth of the inversion was never spin's to explain — and P6, that the inversion **widens
with altitude**, which rules the atmosphere out as the thing that would close it and makes the gate
explicitly a *sea-level* gate — and P8, that the solve those five addenda were arguing about
**refuses seven of the eleven shots it was written for** and empties the OCR consistency check
§Consequences claimed, because the one printed carry that survived the margin turns out to be
reachable once the floor is the *low* plateau.
— and P9, that the inference it has all been building toward **produces nothing on this
corpus**: no stored shot carries a club, so no loft resolves, so the branch is never chosen, and
§Decision 5's face-to-path fallback turns out to resolve the *sign* of the curve and not the
magnitude of the axis — and P10, that **that last count was a fact about `ShotData` and not about
the corpus**: the swing carries the club, eleven of the thirteen shots are attached to one that
does, and once the join exists the inference names a spin for one shot, refuses two for an
undeclared 3 wood and seven for a carry it cannot fly — and P11, that §Decision 6 named
`flight_spin_rpm` without splitting a **printed** spin from a **solved** one, and that P10's own
planar-offline identity holds only where the spin was solved from the carry.
— and P12, that the `swing:{ref}` fallback P11 shipped against was **under-refusing rather than
over-counting**: it has no flagged-parse rule in it, so a flight simulated off a tile flagged under
ADR-014 counted as a sample while the carry printed beside it did not.
— and P13, that the re-analysis ADR-027 §Decision 6 says will **prove** that registration cannot:
no two distinct swings on this corpus share a shot photo, so the photo key and the fallback
partition it identically and both halves of P12 are correct and unexercised. P13 also found the
bump carrying **four M14 measurements** no stored artifact had ever held.
— and P14, that a flight's inputs are not all on the swing: the loft and the handedness live in
**editable** artifacts, so the route that re-flies and the `analysis.json` that stored the answer
can disagree about one shot, and nothing on disk can see that they do.
— and P15, the first finding about the *drawing* rather than the model: the plan view's offline
axis must be stretched by the **smallest** factor that makes the curve readable rather than the
largest that fits, because every flight on this corpus drifts a third to a half of its own apex
and the obvious rule drew the lateral miss taller than the height of the shot.
— and P16, that a simulated number set beside a printed one is a **registry** decision and not a
rendering one, and that on this corpus neither of the two available pairs is a check: the carry is
the spin solve's own input read back wherever the spin was solved (+0.001 yd, and evidence of
nothing), and the two offlines are where the ball started against where it finished, a gap that
decomposes into bend *plus* the carry error leaning on the start line. It also found
`sign_disagrees` had never been rendered to a golfer — the one shot on disk that sets it is a
refusal, so the block the sentence was first written into never renders for it.
— and P17, the first finding about a surface that says nothing rather than one that says too much:
the six numbers reached a **coaching model as bare floats**, because `mcp/query.py` flattens
`measurements` to name -> value and §Decision 6's provenance lives in the two fields it drops, so a
solved spin and a printed one were the same kind of number exactly where the output is sentences
spoken to a golfer. It also found that P10's join runs the wrong way for "what did this swing fly"
— one photo on this corpus is attached to three swings, and the join names one — and that
`differs_from_recorded` makes P14's seam visible for the first time.
**Read the fourth addendum before quoting the agreement anywhere.**

**Goal**: simulate the ball's path from the launch conditions the simulator prints, draw it, and be
honest about which of its inputs were measured and which were solved for.

**Why now.** Two sentences in the code have been describing this model in the negative for a
milestone. `analysis/shot_measure.py` records ball speed and launch angle as *"fitting inputs… for a
model that does not exist yet"*, and the club profile refuses a launch-angle target because
*"optimal launch is per club, per ball speed and per spin rate… the same missing model as ball speed
and the same deferral."* This is that model, and the measure-now-judge-later ordering M6.5 chose is
what makes it buildable from artifacts already on disk.

**The two facts that shape it**, both from ADR-027 §Context and both worth knowing before reading
any phase below:

- **Loft and lie are not inputs to ball flight.** They belong to the *impact* model — club delivery
  to launch conditions — which is not being built. So P1's bag correction and the simulator are not
  cause and effect, and a club bent 2° strong flies exactly as its launch conditions say it does.
- **Spin is missing on 11 of the 13 shots on disk.** The whole 2026-08-23 bay session records
  `Spin: no value text under the label`; only the two 2026-08-10 reference shots carry it. Spin, not
  loft, is what was blocking this.

**The validation set is two shots, and that is the gate.** `2026-08-10-1` (90.7 mph, 20.9°, 5991 rpm
→ 125.6 yd) and `2026-08-10-2` (90.5 mph, 23.5°, 8100 rpm → 121.0 yd) carry a full launch-condition
set *and* the simulator's own carry beside it. P4 is where the integrator meets them. **The tolerance
is not chosen in advance** — it is measured, recorded in ADR-027 as a finding, and pinned at what was
achieved. If the agreement is poor, that is a disagreement between two models to investigate, and not
a number to widen until it passes: P9 bakes this error into every inferred spin.

### Stage A — decide it, record it, fix the data

- [x] **P0** — ADR-027, this section, M12's addendum, `docs/README.md`'s counts *(2026-09-04)*
- [x] **P1** — the bag correction *(2026-09-05)*. `aaron.bag.json`'s 7 iron read `Titleist T150,
      loft_deg 32.0, lie_deg null`; the club is a **2025 T250**, and the published 30.5° / 63° / 37"
      are now on the entry, provenanced `typed`. Applied through `BagStore.set_entry` and
      `catalogue.remember` — the two writes `POST /api/golfers/{id}/bag/{club}` performs — so no
      second bag writer entered the repo, the T150 went to `Bag.retired` with its 32.0° intact, and
      re-serialising dropped the stale pre-M12 `shaft` key from **every** entry. Book loft, not
      measured, per M12's retired-reversal note.
      **Only the 7 iron was corrected, and that is a decision rather than an omission**: the other
      five slots and the retired 6 iron still read T150, so the bag now records a mixed set on
      purpose. **The bag-changed caveat was already firing and still is** — every 7 iron swing
      predates the entry, so `_bag_changed_caveats` moved its date from 2026-09-01 to 2026-09-05 and
      added no new sentence. `club_catalogue.json` gained a `titleist/t250/2025/7i` row, which is the
      one part of this phase that is committed rather than local

### Stage B — the physics (`analysis/`, stdlib only)

- [x] **P2** — the constants *(2026-09-05)*. `benchmarks/flight_model_v1.json` +
      `benchmarks/flight_model.py`, on `joint.py`'s convention — `_MODEL_FILE`, an `lru_cache`d
      loader over `importlib.resources`, pydantic validation, `flight_dataset_info()` and
      `load_flight_model()`. **Not a fit, so no `scripts/` stage exists and none should be
      written**; provenance is per *block*, because the four blocks have four sources: the ball
      from the R&A/USGA Equipment Rules, the atmosphere from ISO 2533 sea level, the 4%/s spin
      decay from Lyu et al. 2018 (CC BY), and the eight `Cl`/`Cd` rows from **Table 3 of
      US 7,156,757 B2** — measured, both seam orientations kept apart and averaged on read by the
      patent family's own stated rule.
      **The table stops at a spin ratio of 0.284 and both validation shots start above it.** Its
      eight rows are points along a *driver's* flight, where the spin ratio rises as the ball
      slows, and a 7 iron launches at `S = 0.330` (5991 rpm) and `S = 0.447` (8100 rpm) — climbing
      from there. So the end row is *held* rather than extended (Smits & Smith measured lift
      saturating, so holding beats extending), `AeroCoefficients.clamped` says when that happened,
      and a test pins that both reference shots are clamped for their whole flight so the claim
      cannot rot. A throwaway RK4 over exactly these constants returned **122.4 yd vs HD Golf's
      125.6 and 124.1 yd vs its 121.0** — ~2.5% out each way, with *opposite* signs, which is what
      a flattened coefficient looks like. Encouraging, and produced by a constant pair rather than
      by the table; **P4 has to say both**
- [x] **P3** — the integrator *(2026-09-05)*. `analysis/flight.py` + `tests/analysis/test_flight.py`:
      `LaunchConditions` → `FlightResult`, RK4 at a fixed step, gravity + drag + Magnus, coefficients
      against **spin ratio** and not speed, spin decayed against *absolute* time so each RK4 stage
      reads its own. Landing is **solved** inside the crossing step — linear seed, then Newton on
      `y(θ)` re-running the integrator over the short step — which lands the reference shot to
      3.6e-12 m against the 9.5 cm a whole-step termination would have been long by. `FlightPoint`
      carries the spin ratio and the clamp flag per point, because neither is constant along a
      flight and P16 has to be able to draw which part was extrapolated. It reproduces P2's scratch
      numbers to the digit: **122.4110 yd and 124.0747 yd**.
      **The step is not where the error is** — carry is identical to 1e-4 yd at every step from
      0.02 s to 0.0005 s, so `DEFAULT_STEP_S` is 0.005 s (four times finer than the coarsest step
      already measured to converge) and the ~2.5% against HD Golf is the clamp's, not the
      integrator's.
      ⚠️ **And it found the thing that rewrites P8 and P9** — see ADR-027's second addendum. Lift is
      written with a coefficient, so `ω` reaches the flight only through `S`, and `S` only through
      `Cl`/`Cd`: above the clamp **spin has no route into the answer at all**. Carry is bit-identical
      at 5,200 and 30,000 rpm. Carry-against-spin therefore rises, peaks, falls and *floors on a
      plateau*, which makes §Decision 4 five cases rather than three
- [x] **P4** — ⚠️ **the gate, passed** *(2026-09-05)*. `tests/analysis/test_flight_validation.py`,
      plus `VALIDATION_SHOTS`, `GATE_AGREEMENT_FRACTION` and `gate_comparisons()` at the foot of
      `analysis/flight.py`. **Every number in this bullet was measured on a planar flight, and P5
      re-flew it** — the tolerance is now 0.0259 and the two carries are 122.3567 and 123.3868 yd.
      The findings below survive the re-fly; the arithmetic does not, and P5's bullet carries the
      current figures. As measured here: **122.4110 yd against HD Golf's 125.6 and 124.0747 against
      its 121.0 — −2.539% and +2.541%.** The tolerance was measured and then pinned at 0.0255, the achieved
      worst case rounded up in the fourth decimal, and a test asserts the pin is that tight so it
      cannot absorb a regression. `gate_comparisons()` re-flies the shots rather than storing them,
      so a re-sourced table moves the gate; it is **not** in `flight_model_v1.json`, whose own
      `license_note` says every number in it is a published measurement.
      **Two things checked and ruled out.** The disagreement is not the printed precision — the
      tiles round to 1 dp and sweeping that interval moves the carry 0.31 yd against a 3.19 yd
      error. And the errors cancel to −0.057 yd, which is the *least* honest way to state this: two
      near-equal errors of opposite sign at `n = 2` mean the model cannot separate the two shots,
      not that it is unbiased. There is a test whose only job is to argue against that calibration.
      ⚠️ **The finding: the gate passes per shot while the ordering inverts.** HD Golf has the
      lower-spin shot flying **4.6 yd further**; this model has it **1.7 yd shorter**, because above
      the clamp the only difference it can see between them is 2.6° of launch angle. No per-shot
      tolerance can catch that. §Decision 4 also takes a second correction — there are **two**
      plateaus, the low-spin clamp being a floor at 118.0 yd, which puts a *unique*-solution band
      between them at spins no 7 iron produces. See ADR-027's 2026-09-05c addendum
- [x] **P5** — the third dimension, and it re-flew the gate *(2026-09-05)*. `LaunchConditions` gains
      `launch_direction_deg` and `spin_axis_deg`, both defaulting to zero; the state is
      `(x, y, z, vx, vy, vz)` in a right-handed frame with `x` downrange, `y` up and `z` right;
      `FlightResult` gains `curvature_m` and `landing_offline_m`. The Magnus term is
      `Cl * (w_hat x v_hat)` with the axis fixed in space and the cross product **not**
      renormalised — only spin perpendicular to the velocity makes a force, and renormalising
      would hold full lift on a ball spinning about its own line of flight.
      **Every planar number P3 and P4 measured survives to the last bit**, because a zero axis
      contributes exactly zero rather than a rounding error, and that is asserted rather than
      assumed.
      **`carry_m` is projected onto the launch azimuth, not the target line** — the phase's one
      real decision. Turning a flight about the vertical cannot change how far the ball flew, so
      carry is invariant to the launch direction; the target-line reading would have shortened it
      by 0.43% and 0.24% on the two validation shots and put a start-line-shaped bias into P8.
      `(carry, curvature)` are the landing point's coordinates in the launch-line frame and
      `landing_offline` is its deviation from the target line, so the three are one point in two
      frames and the identity is pinned.
      ⚠️ **And it found that the gate had been flying a planar approximation of a shot that
      curved.** Both validation shots carry a launch direction (−5.3°, +4.0°) *and* a spin axis
      (2.5°, 9.3°), and had all along. The direction cannot move the carry by construction; the
      axis does, because lift is one vector. Flown as recorded: **122.3567 yd (−2.582%) and
      123.3868 yd (+1.973%)**, so `GATE_AGREEMENT_FRACTION` is re-pinned **0.0255 → 0.0259** — one
      shot slightly worse, the other markedly better, and the *mean absolute* error down from 3.13
      to 2.82 yd. Not a tolerance widened to pass; the same rule applied to the shot HD Golf
      measured. See ADR-027's 2026-09-05d addendum, which also records that the recorded axis
      closes **a tenth of the inverted ordering** — so P4's "±2.5% is the size of the missing spin
      effect" is an over-attribution — that P4's error cancellation was partly an artefact of
      flying planar (mean −0.057 → −0.428 yd), and that §Decision 5's first branch is already live
      while `shot_measure.py`'s sentence about it has not noticed
- [x] **P6** — atmosphere, and the no-numpy pin *(2026-09-05)*. `flight_model_v1.json` gains a
      fifth block — `atmosphere_profile`, ISO 2533's lowest layer — read into an
      `AtmosphereProfile` with `FlightModel.at_altitude()` over it. **`simulate_flight` learned no
      new argument**: the air is a block of the model and the model is already a parameter, so the
      what-if is `simulate_flight(launch, model=load_flight_model().at_altitude(1609))` and an
      `altitude_m` keyword beside `model` would have been a second way to say one thing.
      **The profile is written as ratios against the committed sea-level row, not as the
      standard's absolute formulas**, and the reason is a defect in that row: density is
      self-consistent to 1.5e-8 because ISO 2533 chose its gas constant to make it so, while the
      kinematic viscosity is published rounded and disagrees at 1.3e-5. Exact in one, rounded in
      the other, and which is which depends on the transcription. As ratios, `at_altitude(0.0)`
      returns all four numbers to the bit and a flight through it is point-for-point identical to
      the default. Bounds are the layer's own (−2000 to 11000 m) and outside them it raises.
      **Thin air is not simply longer.** Density scales lift and drag together, so at Denver the
      two reference shots carry **+2.97% and +3.70%** — not the ~10% the driver rule of thumb is
      quoted at — while arriving lower, sooner, shallower and *straighter*. Altitude also cannot
      escape the clamp: the launch spin ratio has no air in it, so both shots stay `fully_clamped`
      at every altitude.
      ⚠️ **The finding: altitude widens the inverted ordering.** The model's gap between the two
      validation shots goes −1.03 yd at sea level → −1.97 at Denver → −3.78 at Tactu, against HD
      Golf's constant +4.6 the other way. It nearly quadruples over a range that moves carry 6%,
      which says the inversion is an aerodynamic term rather than a mistyped launch condition —
      and **rules the atmosphere out as the free parameter that would fix the gate**. The gate is
      therefore a *sea-level* gate, and there is a test saying so: flown at Denver, shot one
      agrees to 0.3% while shot two leaves the tolerance. See ADR-027's 2026-09-05e addendum.
      The pin ADR-027 §Decision 1 asked for by name is in `tests/api/test_pipeline_imports.py` and
      covers `scipy` beside `numpy` — the shortcut is not really an array, it is `solve_ivp`,
      which would take the solved landing and the per-point clamp flag with it

### Stage C — inference, and the CLI

- [x] **P7** — the CLI, and it reads nothing *(2026-09-05)*. `scripts/simulate_flight.py`:
      `--ball-speed / --launch-angle / --spin`, the two lateral fields, `--altitude`, `--step`,
      `--points N` to sample the path and `--verbose` for the provenance block by block. **All the
      logic it has is formatting** — the physics, the gate and the atmosphere were already
      `analysis/flight.py`'s and `benchmarks/flight_model.py`'s, so this phase added no arithmetic
      and no test file: `scripts/` has never had one here, and a formatter is the wrong place to
      start.
      **Explicit launch conditions only is the phase boundary, not an omission.** P10 is what
      teaches it the tolerant readers, and until P8 exists a CLI pointed at the corpus would have
      to invent a spin for 11 of the 13 shots on disk. `--gate` is the same rule rather than an
      exception — `VALIDATION_SHOTS` is a constant, not an artifact — and it is the first place a
      human can see the ordering inversion instead of reading ±2.59% and stopping: it prints the
      per-shot table *and* the sentence that HD Golf has shot one 4.6 yd further while the model
      has it 1.03 yd shorter, both measured at run time.
      **Every number in a caveat is derived, never typed** (`docs/CODE_STANDARDS.md` R4): the clamp
      threshold comes off `AeroTable`, the percentage off `GATE_AGREEMENT_FRACTION`, the ordering
      off a re-flown `gate_comparisons()`. A re-sourced table moves all three and the prose follows.
      **Exit 0 flew, exit 2 did not, and there is deliberately no exit 1** for `analyze_bundle.py`'s
      "flew but flagged" — every iron in this corpus is clamped end to end, so a flagged exit would
      fire on essentially every real shot and mean nothing by the second run. The clamp is said in
      words each time instead. Also fixed a stale count in `docs/README.md`'s ADR-027 row, which
      said "three corrections" over a list of five
- [x] **P8** — ⚠️ solve spin from carry *(2026-09-05)*. `analysis/spin_solve.py` +
      `tests/analysis/test_spin_solve.py` (+15): `UnspunLaunch` (launch conditions with the one
      missing field left out, rather than a `LaunchConditions` whose spin is silently ignored),
      `carry_window` — which measures the whole carry-against-spin curve in about forty flights —
      and `solve_spin_from_carry` over it. **Seven cases, being ADR-027 §Decision 4's five with two
      of them split**: the plateau case is two plateaus meaning opposite things, and the
      inside-the-window case is two bands, because between the plateau values only the rising
      branch reaches. The `scipy` half of `test_pipeline_imports.py`'s pin now covers this module
      too, and it is the sharpest of the three — `brentq` and `minimize_scalar` are a one-line
      substitution for each search here.
      **The physics did not move; the phase is a count.** Run over the corpus, the solve **names a
      spin for 4 of the 11 spin-less shots and refuses 7** — so §Decision 3's inference is the
      exception rather than the path, which is not how that section reads. The seven refusals clear
      the peak by +0.56% to +3.45% against a model that is ~2.6% out where it can be checked, so
      they are reported as *this model cannot fly that carry* and never as an OCR fault.
      **The honest test re-run in three dimensions**: 3,185 rpm against a recorded 5,991 (46.8%
      low) on one shot and a refusal on the other, 121.0 yd printed against a floor of 122.62.
      Both true spins are above their own cap, where carry has stopped depending on spin at all.
      ⚠️ **And it corrected two things.** The OCR consistency check §Consequences claimed **has no
      survivors left** — `2026-08-23-2`'s printed 33.6 yd was called 25% below anything the model
      could fly, against a floor that was the *high* plateau, and the real floor is 26.89 yd, so it
      solves at 2,307 rpm. That same shot has **no falling branch at all** — its peak is its high
      plateau to the last bit — so a loft prior has nothing to choose on it, and its unique band
      runs 1,323–4,572 rpm rather than the 1,129–1,538 the reference shot's does. The blanket
      "treat a unique answer as a refusal" therefore has to be argued from where the answer sits;
      `CarryWindow` carries the band edges so P9 can. See ADR-027's 2026-09-05f addendum
- [x] **P9** — ⚠️ the loft prior and the axis, and **both refuse the whole corpus** *(2026-09-05)*.
      `analysis/flight_infer.py` + `tests/analysis/test_flight_infer.py` (+21): `infer_spin` puts
      the branch rule over P8's seven cases, `infer_spin_axis` resolves ADR-027 §Decision 5 in its
      stated order, and `honest_test()` / `gate_ordering()` are the two things a caller must print
      beside any number either produces. `contracts/unscored.py` gains four reasons —
      `CARRY_UNREACHABLE`, `NO_CLUB_LOFT`, `SPIN_NOT_RECOVERABLE`, `SPIN_AXIS_UNRESOLVED`, all with
      `refilming_helps` false — collected as `INFERENCE_REASONS`, the vocabulary's **third family**,
      which turned that module's two-way partition test into a three-way one so a future family
      cannot be absorbed silently.
      ⚠️ **The finding is a count and it is zero.** Over the thirteen stored shots the inference
      returns **no spin at all**: eight `carry_unreachable` and five `no_club_loft`, because not one
      shot on disk carries a `club` and loft is the whole of the branch rule. The spin axis resolves
      on the two shots that printed one and refuses the other eleven. Built and silent, exactly as
      M9 is and for the same missing tag — **the bay session with the club cursor set turns both on
      at once**.
      **The prior needs no loft-to-spin table**, which is the phase's one design decision. The
      branch is one bit — does this club spin faster than the peak-carry spin — and `carry_window`
      *measures* that peak per shot: 2,402, 2,555 and 2,781 rpm on the three two-branch shots, which
      is a driver's own spin. So everything more lofted than a driver takes the falling branch and a
      driver refuses, and the single constant sits in the 12–15° gap no club occupies. A test pins
      that the answer is identical for every loft above the gap, which is the honest form of "coarse
      prior, enormous margin"
      ⚠️ **And it corrected §Decision 5.** The face-to-path fallback resolves the **sign** of the
      curve and not the magnitude of the axis: on the two shots carrying both, 10.9° of face-to-path
      sits against a 2.5° axis and 13.2° against 9.3° — three times the tilt per degree, and ordered
      the wrong way for any monotone relation, since the shot with *more* backspin should tilt
      *less*. So the second branch collapses into the third, the flight is drawn planar and
      `landing_offline` goes unscored. The sign check still runs and **one shot on disk fails it**
      (114.8 mph, printed `SLIGHT FADE`, face-to-path −1.3°): flagged, never overwritten. Also
      corrected `shot_measure.py`'s stale sentence calling the stored axis sign-inverted — the
      2026-09-05d addendum spotted it and §Decision 5's owner is this phase. See ADR-027's
      2026-09-05g addendum
- [x] **P10** — ⚠️ the CLI reads real shots, and the club was on the swing all along
      *(2026-09-05)*. `storage/flight_inputs.py` + `tests/storage/test_flight_inputs.py` (+9) is
      the join — shot photo sha256 to `SwingManifest`, and through it to the bag's loft and the
      golfer's handedness; `analysis/flight_infer.py` gains `flight_for_shot` and `ShotFlight`
      (+12 tests), which is the unit **P11 measures and P14 serves**; `scripts/simulate_flight.py`
      gains `--shots` and `--shot ID`. **No new reader**: `ShotStore.all()` and the bundle, bag and
      golfer stores already existed, and `storage/` may not import `launch_monitor/` (ADR-008), so
      the join takes its shots as an argument and `scripts/` is where the two meet.
      ⚠️ **The finding is a correction to P9's headline.** *"No shot on disk carries a club"* is
      true of `ShotData` and false of the corpus: **eleven of the thirteen shots are attached to a
      swing that carries a club tag** — five `7i`, six `3w`, tagged at capture on 2026-08-23. What
      is actually missing is a *declared loft*: only the 7 iron has one. So `LoftGap` splits the
      one `None` into its four repairs, and the six 3 wood shots are the **bag page, not the bay**.
      Run end to end the inference now **names one spin and refuses ten** — `2026-08-23-4` at
      2,924 rpm under a 5,103 rpm cap; seven `carry_unreachable` (0.6–3.6 yd above their own peak),
      two `no_club_loft` (both 3 woods, both two-branch, 2,161/2,849 and 2,450/3,200 rpm), one
      `spin_not_recoverable`. Looking the 3 wood up turns two of those into answers with no bay
      session and no new physics.
      **Three more things it found, each pinned.** `VALIDATION_SHOTS` — hand-typed in P4 and never
      checked — resolves off disk **to the digit, both lateral fields included**, which is the
      first external check that constant has ever had. A **planar flight's `landing_offline_yds`
      is `carry * sin(start line)`**, which is `shot_measure`'s `start_line_offline_yds` by a
      longer route (1e-14 structurally, 2.6e-5 yd on the real shot) — so P11 must not record
      `flight_landing_offline_yds` on a shot whose axis is unresolved, which is eleven of the
      thirteen. And the one shot that resolves is **the only flight in this corpus that reads the
      published table at every point** (0 of 919 held), which is structural rather than lucky: a
      carry the solve can invert is a carry still responding to spin, and that is the same regime
      as reading the coefficients rather than holding them.
      `contracts/unscored.py` gains a fifth inference reason, `NO_LAUNCH_CONDITIONS`, which fires
      on nothing today and exists because the OCR drops tiles one at a time and `simulate_flight`
      names its launch-angle guard as the caller's to own. See ADR-027's 2026-09-05h addendum

### Stage D — into the pipeline

- [x] **P11** — the six `flight_*` measurements, in the pipeline *(2026-09-05)*.
      `analysis/flight_measure.py` + `tests/analysis/test_flight_measure.py`: `fly_shot` resolves
      and flies one stored shot, `FLIGHT_MEASUREMENTS` reads six numbers off the one flight (the
      `SHOT_MEASUREMENTS` shape would have re-integrated it six times), and `flight_unscored` names
      what it could not draw. Source `model:flight_v1`, the fourth provenance and the first that is
      not a reading of anything. `analyze_swing` gains `loft_deg`, fed by
      `api/pipeline.py::_loft_for` through a new `storage/flight_inputs.loft_for_club` — the bag
      half of P10's join, with the photo-sha256 match skipped because the shell is already holding
      the manifest.
      **Two of the six record conditionally, and both conditions are one rule — one provenance per
      name.** ⚠️ `flight_spin_rpm` records only a **solved** spin: a printed one is the launch
      monitor's reading, and pooling the two would average a measured 5,991 rpm with a solved 2,924
      under one name. That is a correction to ADR-027 §Decision 6. And
      `flight_landing_offline_yds` is withheld wherever the curve was not drawn — which is where
      ⚠️ P10's identity turns out **not to be structural**: it holds to 2.6e-5 yd where the spin was
      solved *from* the printed carry and breaks by **0.29 yd** where the spin was measured, so the
      case for withholding is stronger on that half of the corpus rather than absent.
      ⚠️ **And it found a refused flight about to become a coaching tip.** `feedback/rules.py` and
      `feedback/coach.py` both render `unscored` as "could not be scored, so it is not included in
      the score", which is false of a measurement that was never in `overall_score`; the tips now
      skip `INFERENCE_REASONS` and the brief gives the flight its own heading. A refused flight is
      **one** entry (under `flight_carry_yds`) and not five, because `get_session_summary` counts by
      name. `fly_shot` catches `simulate_flight`'s `ValueError` so an unparseable tile cannot take a
      whole swing down. No `METRIC_TARGETS` rows, argued in `contracts/dispersion.py` and pinned —
      the dispersion test's `POSE | SHOT` equality would not have seen a third registry
- [x] **P12** — the `model:flight_v1` source prefix, registered *(2026-09-06)*. In
      `contracts/career.py::CorpusSwing.artifact_key` and not `storage/corpus.py`, which is where
      this bullet and ADR-027 §Decision 6 both had it. `model:` keys on the **shot photo**, beside
      `launch_monitor:`: the integrator is fed that tile's launch conditions and nothing the body
      did. New `KNOWN_SOURCE_PREFIXES` because `count_metrics` had the membership test written out
      a second time. ⚠️ **P11's reason for shipping ahead of this was wrong in the direction that
      matters** — the `swing:{ref}` fallback can only over-count on the *dedupe*, but it carries no
      flagged-parse refusal, so a flight simulated off a tile flagged under ADR-014 counted as a
      sample while the `carry_distance_yds` printed beside it did not. Nothing on disk is flagged;
      the first bay session would have been. ⚠️ **`population:golfdb` stays unregistered on
      purpose**: it moves no count (`CorpusSwing` is one-to-one with `face_on_sha256`, so the
      fallback already partitions as `pose:` would) and would decide ADR-022's fourth addendum by
      accident, including for two down-the-line placements read off a clip `CorpusSwing` has no
      hash for. Also one refusal sentence: `no_population_reason` was telling a reader of a
      simulated carry to go and cut a distribution for it
- [x] **P13** — `ANALYSIS_VERSION` 14 → 15, and the corpus is on it *(2026-09-06)*.
      `contracts/swing.py` (the constant and its ledger entry) plus `reanalyze.py --all` over all
      fifteen stored swing directories, 15/15 clean and exit 0. **The prediction held exactly**:
      every `overall_score` byte-identical, and every `score`, `observed` and `passed` in every
      `checkpoint_scores` entry with it — no window moved, no anchor moved, no `aligned.mp4` went
      stale. What is new on disk is the `flight_*` family: **five artifacts carry a flight, ten
      carry a refused one** (seven `carry_unreachable`, two `no_club_loft`, one
      `spin_not_recoverable`), and the one flight that flew without an axis carries the eleventh
      entry, `flight_landing_offline_yds` / `spin_axis_unresolved`. Two swings gained a **sentence
      the golfer sees** for the first time — the 3 wood's *"add the club on the bag page"*.
      One new pin, in `tests/test_docs_truth.py`: the ledger must document the installed version,
      because a bump without its entry is the one part of this phase nothing else would catch.
      ⚠️ **The finding: the re-analysis ADR-027 §Decision 6 says will *prove* the prefix
      registration cannot.** `flight_carry_yds` reports `n = 3` against five artifacts, which is
      right — but three of those five are re-uploads of one face-on clip that `read_corpus` had
      already collapsed. Fifteen directories hold 13 distinct shot photos and the only repeat is
      that same triple, so `model:{photo}` and `swing:{ref}` partition this corpus **identically**;
      P12's flagged-parse refusal is unexercised too, since no stored parse is flagged. Correct,
      and waiting on a bay session for its first evidence — which is the argument P12 used
      *against* registering `population:golfdb`, arriving at the registration it made *for*.
      ⚠️ **And the bump carried four M14 measurements onto disk.** `hand_separation_norm`,
      `hand_height_norm`, `hand_offset_from_hips_norm` and `trail_hand_roll_deg` shipped under
      "no checkpoint, no band, and no `ANALYSIS_VERSION` bump" — the third clause wrong by the
      ledger's own rule (*"a new measurement"*) and against four precedents that bumped for
      measurements nothing judges. `is_outdated` is a version comparison and nothing else, so
      nothing could see that no stored artifact held them: their honest `n` was **0** for three
      days and is 13 now. A band and a version answer different questions, and M14 read them as
      one. See ADR-027's 2026-09-06b addendum, and M14's own document, which now says so

### Stage E — the surfaces

- [x] **P14** — ⚠️ the flight route, and the answer it can disagree with *(2026-09-06)*.
      `GET /api/sessions/{session_id}/swings/{swing_id}/flight`, with its `ARCHITECTURE.md` §1
      row in the same commit — which is what `test_architecture_lists_every_api_route` makes
      unskippable. `api/flight_view.py` is the derivation (no `fastapi`, so it is testable on a
      base install) and the route is the manifest reads: shot photo sha256 → `ShotStore`, then
      `loft_for_club` and the golfer's handedness off the stores `create_app` already holds.
      **It re-flies rather than reading `analysis.json`**, first because the artifact stores the
      six numbers and no path and a path is what a viewer draws, and second for the finding below.
      The payload is everything **P16** has to render — the six measurements through
      `FLIGHT_MEASUREMENTS`, the printed numbers through `SHOT_MEASUREMENTS`, the spin's source,
      the axis's refusal, `curve_is_drawn`, and the caveats — so that phase stays a rendering
      phase rather than becoming a second route change.
      **A refusal is a 200 and a 404 is something else**: ten of the thirteen shots on disk cannot
      be flown, so a route that erred on a refusal would report the repo as broken every honest
      read. The 404 is for a swing with no shot screen, or one never parsed — and **no OCR runs
      in the request**, the store being keyed on the photo's digest.
      ⚠️ **The finding: two of a flight's inputs live in editable artifacts.** Declare the 3
      wood's loft on the bag page and the route flies a shot `analysis.json` still records as
      `no_club_loft` and `career_baseline` still counts as refused — no re-analysis, no version
      change, and nothing on disk that can see it: `is_outdated` compares versions and
      `AnalysisState.inputs` hashes the *uploads*. The shape is not new (re-attributing a swing
      leaves `head_stays_back` scored under the old handedness) but the size is: a loft flips
      **five** measurements between recorded and withheld. See ADR-027's 2026-09-06c addendum.
      **Two moves out of `scripts/`, both because P7 wrote for one surface and there are now two**:
      the four caveats to `analysis/flight_caveats.py` (with `caveats_for`, so *which* caveats a
      flight owes is one rule) and the path sampling to `FlightResult.sample`, which guarantees the
      solved landing survives. `pipeline.loft_remedy` is public for the same reason, and offers
      its sentence only once a flight has asked for a loft and gone without.
      **The only sync handler in `app.py`**, measured rather than assumed: every other route
      answers in 3-24 ms and this one in 15 ms with a printed spin, ~960 ms without, because the
      solve flies about forty flights. `def` puts that in a threadpool instead of on the event
      loop
- [x] **P15** — ⚠️ the page, and the axis it is allowed to stretch *(2026-09-06)*.
      `api/static/flight.html`: canvas, vanilla JS, **no framework, no bundler, no CDN**, which is
      ADR-027's *Alternatives* call arriving as code. **Two projections of one polyline rather
      than a camera on it** — a side elevation for height, a plan view for offline — because every
      camera angle makes one of those two questions unreadable while looking authoritative about
      both. Reached from `results.html`'s shot block on every swing whose screen has been read,
      **including the ten that refuse**: hiding the link there would hide exactly the shots with
      something to say, one of which carries a repair the golfer can make from the bag page.
      ⚠️ **The finding, and it inverts the first version.** Down range is drawn at one scale in
      both panels and the side view is 1:1 with it, so the panel's height falls out of the scale
      and a flat shot draws flat. Offline cannot be — at 1:1 a two-yard drift across a
      hundred-and-twenty-yard carry is a line one pixel thick — so the plan view stretches it by a
      round, printed factor. Written first as *the largest factor that fits*, which measured
      against both flights on disk drew a 9.6 yd drift at ×2 into a **214px panel above a 117px
      side view**: the lateral miss taller than the height of the shot, beside a caption reading
      1:1. Every flight on this corpus drifts a third to a half of its own apex, so that rule
      magnified all of them. It is now the *smallest* factor that lifts the curve over a
      legibility floor, so on this corpus the stretch never fires and a straight shot (0.4 yd of
      drift) still gets its ×10.
      **The clamped part of the path is dashed, per point**, which is the 2026-09-05b addendum at
      a surface: `2026-08-10-1` is dashed end to end (905 of 905 steps) and `2026-08-23-4` — the
      one shot that reads the table, and the one whose spin was invented — is solid for all 919.
      The spin's provenance rides on the line carrying the value rather than waiting for P16,
      because a number shipped bare for one phase is the erosion §Decision 3 names.
      **Pinned by `tests/api/test_flight_page.py`** (+8): the page holds no second copy of
      `FLIGHT_MEASUREMENTS`, `FLIGHT_SOURCE`, `INFERENCE_REASONS` or `SpinSolveCase`, decides its
      re-filming sentence on `refilming_helps` and not on a reason's name, and reports all four
      failure paths. **Layout on a phone is unverified** — no browser was driven; the projection
      arithmetic was checked against the live payloads at 600px and 320px instead
- [x] **P16** — ⚠️ viewer honesty, and the comparison that is not one *(2026-09-06)*.
      `analysis/flight_measure.compare_to_printed` + `PrintedComparison` + `circular_carry_note`,
      served as `comparison` by `api/flight_view.py`, rendered by `api/static/flight.html` and
      printed by `scripts/simulate_flight.py --shot`. The printed carry is now **drawn**: a hollow
      ring on the side view's ground line, and a rule across the plan view — a rule and not a
      point, because the screen prints no offline tile at all and a ring on the target line would
      have invented a landing. `caveats_for`'s four sentences render for the first time, the spin's
      cap rides with a solved number, and the two conditional measurements' refusals are said in
      the panel rather than only in `unscored`.
      ⚠️ **The finding, and it corrects P15's framing of this phase.** P15 left this "a rendering
      phase, the payload already carries all three" — but pairing a printed number with a simulated
      one is a *registry* decision (which two names are one quantity, and whether their difference
      is an error), so it lives in `flight_measure` beside `FLIGHT_MEASUREMENTS` and both surfaces
      read it. And on this corpus **neither of the two pairs is a check**: where the spin was
      solved, the printed carry is the solve's own input and the flight reproduces it to
      **+0.001 yd** — the most convincing pair of numbers on the page, and evidence of nothing;
      and the two offlines are where the ball *started* against where it finished, whose gap is
      `curvature * cos(direction)` **plus** the carry error leaning on the start line (15% of it on
      `2026-08-10-1`), so calling it "the curve" would re-report the carry error as bend. The other
      four measurements have no printed counterpart at all — **no shot on disk carries an apex** —
      so five of the six are unfalsifiable here and the sixth is only half checkable.
      ⚠️ **And a flag that had never reached a golfer.** `sign_disagrees` has been carried since P9
      and printed by the CLI; written into this page's launch-conditions block it *still* would not
      have shown, because the one shot on disk that sets it is a `carry_unreachable` refusal with
      no launch conditions. It renders from `render()` now, beside the refusal. A planar flight's
      line also stopped ending in a landing mark — that would be the page asserting the number
      `FLIGHT_MEASUREMENTS` withholds — and the printed carry now sets the down-range span, since
      it is the longer of the two on both measured-spin shots and was being drawn off the canvas.
      **Pinned by 15 new tests** across `tests/analysis/test_flight_measure.py`,
      `tests/api/test_flight_route.py` and `tests/api/test_flight_page.py`. **Layout is still
      unverified** — the projection arithmetic and every payload on disk were replayed against the
      page's own JavaScript at 600px and 320px, but no browser was driven
- [x] **P17** — ⚠️ the eleventh tool, and the surface that had been quietest *(2026-09-06)*.
      `simulate_flight(session_id, swing_id)` in `mcp/flight.py`, registered by both adapters —
      `mcp/server.py` and `mcp/runner_tools.py` — from one `SIMULATE_FLIGHT` description in
      `contracts/tool_descriptions.py`. It flies the shot a stored swing arrived with and returns
      the six numbers with their provenance, the refusal where there is one, the launch conditions,
      the spin's source and cap, the axis and `sign_disagrees`, P16's printed pairing and
      `caveats_for`'s sentences. **No path** — a model cannot look at a polyline — which is the one
      thing this surface drops. Not registry-gated, unlike the career and club five: a shot whose
      screen printed its own spin borrows nothing from a bag, so gating it would remove answers
      rather than protect them.
      ⚠️ **The finding: `get_swing` had been shipping the flight as bare floats.** §Decision 6's
      `model:flight_v1` is on every one of the six, the engine writes it, the artifact carries it
      and the page has said SIMULATED since P15 — but `mcp/query.py::_measurements` flattens
      `measurements` to name -> value, so a **coaching model** received `flight_carry_yds: 122.36`
      and `flight_spin_rpm: 2923.86` under a field description opening *"quantities measured off
      this swing"*. A solved spin and a printed one were the same kind of number on the one surface
      whose output is sentences spoken to a golfer. `SwingView.simulated` is `PlacementView`'s
      split made for the opposite reason — there a float loses the meaning, here it loses the fact
      that nothing measured it — with membership on `Measurement.source` through
      `MODEL_SOURCE_PREFIX`, never the `flight_` name. `caveats.READING_A_SIMULATED_FLIGHT` is the
      standing rule beside it and ships on **every** shape of the server, because the split reaches
      a model through `get_swing` whether or not the tool is called.
      ⚠️ **And a join has a direction.** `read_flight_inputs` answers "which swing was this shot hit
      on" and names one survivor per photo; **one screen photo on this corpus is attached to three
      swings** (`2026-08-07-aaron1/1`, `2026-08-09/2`, `2026-08-10/1` — all three roles identical,
      one physical swing uploaded three times), so built on that join the tool told two of them
      their screen had never been read while `get_swing` returned their recorded flight in the same
      conversation. It resolves the manifest's photo instead, which is the route's own resolution.
      This does not correct P13: `CorpusSwing` groups by the face-on clip, which those three also
      share, so they remain one corpus swing.
      **It is also the first surface that can see P14's seam** rather than re-fly past it:
      `differs_from_recorded` sets the stored rows beside the flown ones at the engine's own
      rounding, and `recorded_reading` splits the two causes on `analysis_version` — an older
      artifact differs because of the engine, a current one because an input outside it was edited.
      Empty on all thirteen shots today, which is what it should be until someone edits a bag.
      **Pinned by 20 new tests** across `tests/mcp/test_flight_tool.py` (14), `tests/mcp/
      test_query.py` (3), `tests/mcp/test_server.py` (1) and the two count pins that moved
- [x] **P18** — the doc-truth cascade *(2026-09-06)*. Tool count **ten → eleven** in
      `scripts/ask_swing.py`, `docs/ARCHITECTURE.md` §1 and this file; ADR-027's Status block flips
      to built with its **fifteenth** addendum; `docs/README.md`'s ADR row, addendum totals and
      §Conventions counts follow; `docs/ARCHITECTURE.md` §2 gains the `simulated` split beside the
      placements one it mirrors; this section flips to done
- [x] **P19** — ⚠️ the shot as a golfer sees it *(2026-09-06)*. A third panel at the top of
      `api/static/flight.html`: a perspective tracer from behind the ball looking down the target
      line, red, animated along the path, with a Replay button and the ball's own track drawn on
      the turf beneath it. No payload change and no route change — `api/flight_view.py` has served
      the polyline in three dimensions since P14, so this is rendering only. **It does not
      overturn P15's argument, it scopes it**: a camera does make both "how high" and "how far
      offline" foreshortened at once, which is why the two orthographic panels stay and why this
      one carries `perspective — nothing measurable` in the slot they use for their scale. It
      asserts exactly what the plan view asserts and no more: the printed carry is a rule across
      the strip rather than a mark on it, and `tracerLanding` returns on `curve_is_drawn` before
      it draws a ring or names an offline, so a planar flight gets neither.
      ⚠️ **Two findings, both geometry.** The top of a perspective frame is **not the apex** — what
      a camera sees is angular elevation, `(height - eye) / depth`, which on every flight here
      peaks around 30 yards out rather than 70, where the ball is lower but much closer. Fitting
      the frame on the highest point of the *flight* put the tracer nine pixels off the top of the
      canvas, and only at wide layouts, where there is least padding in hand. And a camera at
      **literal eye height** — 1.9 yd up, at the ball, which is what "from my perspective" means
      taken literally — compresses everything between 25 yards and the landing into about
      **seventeen pixels** just above the horizon while the turf nearer than 25 yards fills half
      the panel. Six yards up and fourteen back spreads the same 125 yards over ~200px. Both were
      found by replaying the five payload shapes through the page's own JavaScript under a stubbed
      DOM at 600px and 320px; the browser is still undriven

**Deliberately out of scope**: the impact model, gapping, club fitting, swing efficiency, wind,
roll-out. ADR-027's *Deferred, by choice* records each with what it is actually waiting on. The one
worth watching is a **spin measurement** — Milestone 3's open profile-tuning item is the real repair.
It was also going to be what made the honest test of ADR-027 §3 possible, and **P3 found that the
test was already runnable**: the two 2026-08-10 shots carry a measured spin *and* a printed carry,
which is all it needs. It has been run, and §3 fails it — one refusal, one answer 47% low. The
profile-tuning item is still the repair; it is no longer the blocker on knowing.

---

## M16: Mishits — the topped seven iron that is not your seven iron — done

**Status**: ✅ Done *(2026-09-08)*, 9/9 phases (P0–P8). Desk work over the corpus already on disk —
no bay session, no `n`, no re-capture. Built on branch `GOLF-6`, one commit per phase, the full
suite / `ruff` / `mypy` green at every one. The *why* is
[ADR-028](docs/decisions/028-mishit-exclusion.md), and building corrected nothing in it — the
chokepoint, the 0.50 floor, the manual override and the metric scope all shipped as designed.

**The complaint.** The per-club profile pools every tagged shot's carry and takes a guarded mean. A
topped 7 iron that carries 20 yards is one of the five shots that mean is built from, counted
exactly like the four real ones. "How far do I hit my 7 iron" comes back six yards short because of
a shot the golfer already knows was a mistake.

**Why the flight model cannot catch it.** A genuine top prints a low ball speed *and* a short
carry, and they agree — so `analysis/spin_solve.py`'s `BELOW_FLOOR` and the `carry_unreachable`
refusal, which fire when a carry is impossible *for its launch conditions*, stay quiet. The signals
that a 20-yard 7 iron was a mistake are that it is far shorter than that golfer's own 7 irons, that
the simulator's `Impact Position` tile said so, or that the golfer says so. M16 uses the first and
the third; the tile is deferred until a bay session shows what it prints.

**The rule.** A shot is a mishit when its carry is below **half** that club's own median carry,
gated on five clean carry samples for the club (the CENTER-claim floor from `contracts/baseline.py`,
reused). Auto-flagged in `read_corpus` before the count is taken, so the printed `n` and the pooled
`n` never disagree. The `0.50` is a loose, documented judgment expected to be tuned from labelled
bay data — it catches tops and blades and leaves heavier contact for the golfer to confirm by hand.

**The override.** `SwingManifest.mishit` — `confirmed` or `cleared`, set through
`POST /api/sessions/{session_id}/swings/{swing_id}/mishit` or `scripts/flag_mishit.py`, the `club`
field's pattern exactly. The manual verdict always wins over the auto flag.

**Scoped to distance, and reported.** Only `carry_distance_yds` and `total_distance_yds` drop the
shot; ball speed, launch, offline and every pose checkpoint still count it. Every held-out shot is
counted (`mishits`), named (`mishit_refs`) and caveated in the voice the golfer reads. No
`ANALYSIS_VERSION` bump — the aggregates are computed live and nothing in `analysis.json` changes.

**Phases, as built.** P0 docs (this section and ADR-028). P1 `contracts/mishit.py` and the manifest
field. P2 the corpus flag (`CorpusSwing.auto_mishit` / `manual_mishit` / `is_mishit`) and the
`artifact_key` chokepoint. P3 auto-detection wired into `read_corpus` via `_flag_auto_mishits`,
grouped by club, before `count_metrics` — behaviour went live here. P4 the `ClubProfile` /
`BagProfile` fields and `_mishit_caveats`. P5 the write route, `bundle_store.set_mishit`, and
`scripts/flag_mishit.py`. P6 the per-club MCP surfaces. P7 the swing and session surfaces and the
two derived caveat bullets (`READING_A_BAG`, `READING_A_PERSONAL_HISTORY`). P8 docs reconciliation
and the merge.

**Exit criteria — met on real data, and sharper than the fixture.** The exit test was "a club with
five clean shots and one ~20-yard top reports a mean that excludes the top, `mishits = 1`,
`mishit_refs` naming the swing, a caveat sentence, and `ball_speed_mph`'s `n` unchanged." P5's
`flag_mishit.py --list` over the corpus on disk found the case unflagged by anyone: aaron's 7 iron
has five tagged carries — 121.8, **33.6**, 124.7, 126.1, 101.9 — median 121.8, floor 60.9, so
`2026-08-23/2` auto-flags, `mishits = 1`, `mishit_refs` names it, and the caveat prints. What the
corpus does *not* match is the fixture's shape: five total, not five-plus-a-top, so the exclusion
leaves **four** clean carries — below the CENTER-claim floor of five. `get_club_profile` for that
club therefore stops reporting `carry_distance_yds` and `total_distance_yds` at all ("a typical
value needs 5 samples; there are 4"), where before it printed a confident ≈101.6 yd mean dragged
down by the top. That is the ADR-010 posture landing exactly where it should — no number beats a
wrong one. `ball_speed_mph` and `backswing_ms` still count all five. `--clear` on the shot restores
the five-sample carry mean; `--confirm` on a heavier miss above the floor removes one. The full
suite, `ruff` and `mypy` stayed green at every phase.

---

## M17: Pivot points — the shoulder line, the hip line, and the three points they turn about

**Status**: ✅ Done, 9/9 phases (P0–P8), closed 2026-09-10. Desk work over the corpus already on
disk — no bay session, the overlay and the rule checks needed none. Built on branch `GOLF-5`,
one commit per phase. The *why* is [ADR-029](docs/decisions/029-pivot-points.md); the phase
record is [docs/M17_PIVOT_POINTS.md](docs/M17_PIVOT_POINTS.md).

**The ask.** Track the pivot points a golfer watches to judge a turn — the shoulder-line
midpoint, the hip-line midpoint and the hands — through the swing; draw them and the two
rotation lines on `aligned.mp4`; and flag when the turn is clean versus "wonky".

**Why it is an interim instrument.** [ADR-011](docs/decisions/011-camera-synchronization.md)
established that true rotation — hip turn, shoulder turn, X-factor — is 3-D, and its 2026-08-05
addendum established that the two hand-held phones "can be aligned but never fused": no stable
extrinsics, so triangulation is unreachable and one 2-D camera foreshortens the turn (37°
down-the-line vs 2° face-on for the same posture). The golfer is printing QR-code fiducial
markers for the hitting area, which gives real calibration and a 3-D source later. So M17's
rotation numbers are an **explicit interim 2-D-per-view instrument**: unjudged, per-camera,
never blended, each carrying the sentence saying why it is provisional — built behind a
`contracts/pivots.py` seam so the calibrated source drops in as a second producer of one shape.

**Not a checkpoint.** A 2-D image-plane line angle carries the frame's pixel aspect (the
`trail_hand_roll_deg` precedent — it ships unjudged) and, measured through the swing, an
unknown amount of foreshortening. There is no band to earn yet and no calibrated instrument to
earn it with, so the pivot numbers ride on `measurements`, never `checkpoint_scores`
([ADR-010](docs/decisions/010-benchmark-ranges.md) §2). Wonky detection is stdlib rule checks
on the pivot paths — path smoothness, no against-the-turn reversal, the hip and shoulder
centres staying near a stable vertical axis.

**The phases.** P0 the docs (this section, ADR-029, the ADR-011 addendum) — **built 2026-09-09**.
P1 the overlay — `pose/overlay.py` gained the guide lines and the pivot markers, reaching both
`aligned.mp4` panels unchanged; **built 2026-09-09**, stored renders show the old overlay until
P7. P2 `contracts/pivots.py` — `FrameOfReference`,
`PivotObservation`, `PIVOT_SAMPLES` (41, odd so the top of the backswing lands on a sample) and
the ten-row per-view registry; **built 2026-09-09**. P3 `analysis/pivot.py::pivot_observations` —
one producer taking the three swing anchors, serving both cameras; **built 2026-09-09**, and it
moved two calls the plan had left to a phrase: the ruler is a **median** per-sample shoulder width
(a mean over address → impact shrinks with the turn, which would score the biggest turns wonkiest)
and the origin is the hip centre **at address** rather than per sample (a per-sample hip origin
zeroes the very travel two of the five checks measure). P4 the five rule checks over its output —
two drifts, a roughness and a reversal per half of the swing, keyed by `PivotMeasurementSpec.check`
so one implementation serves both views; **built 2026-09-09**, and every one of them is unsigned, so
a mirrored swing scores identically and handedness is not an argument. P5 the
engine wiring, a `career.artifact_key` branch for the second camera's rows, and
`ANALYSIS_VERSION` 15 → 16; **built 2026-09-09** — the `_dtl` rows carry `POSE_DTL_SOURCE`, the
first `pose:` source that dedupes on nothing, because `CorpusSwing` holds no hash for the rear clip,
so those five are reported and pool into no personal baseline while their face-on partners pool
normally. **P5's corpus run is what found P3's veto**: the hands share the shoulder and hip
interpolation gate, they are the worst-tracked of the three face-on, and the one point nothing
measures was refusing the whole face-on view on all fifteen swings — two gates now, and
`PivotObservation.hands` is optional. All fifteen re-analysed, ten pivot rows each, every score
byte-identical. P6 the derived caveat, `api/state.py::resolve_pivots` and its three
channels (results page, coaching prompt, `SwingView.rotation`), all saying the numbers are
interim; **built 2026-09-10** — `contracts/caveats.py` names all ten rows and gained
`PIVOTS_ARE_INTERIM`, `resolve_pivots` mirrors `resolve_placements` field-for-field except
`interim_reason` standing in for `calibrated`, and the results page renders a "Your rotation
(interim)" block below the tour population one, reusing its `.plc` styling. P7 re-render every
stored `aligned.mp4`; **built 2026-09-10** — all fifteen swings re-rendered, every score
byte-identical, every video's decoded frame count matched what was written, and a direct frame
read of one fresh render confirmed both panels' pivot markers and guide lines, the down-the-line
line visibly opening from a near-point at address to a long diagonal by follow-through. Browser
playback itself is still unverified. P8 reconciled the docs — this section, ADR-029's Status and
its second addendum, `docs/README.md` and `WORKLOG.md`; **built 2026-09-10**.

**P2–P6 were rewritten after P0 shipped.** Reading the first phase list back against the modules
it named found six places it could not be built as written — most importantly a rule-check type
(`measure.MeasureFn`) that takes `FrameKeypoints` and would have closed the very seam the
milestone exists to build, and phases passed to a second camera that never has any. The
decisions did not move; the plan had drifted off them. ADR-029's 2026-09-09b addendum is the
record.

**Exit criteria — met.** A two-view swing's `aligned.mp4` shows the three pivot markers and the
two rotation lines on both panels, the down-the-line lines visibly turning; `get_swing` returns a
`rotation` block of per-view numbers each labelled interim; a swing that sways or reverses
scores its `pivot_*` values high against a clean one; and `reanalyze.py --all` reported every
`overall_score` byte-identical across the corpus. The full suite, `ruff` and `mypy` were green at
every phase.

---

## Milestone 5: Feedback UI
**Goal**: Present swing analysis to the user in a clear, visual web interface.

- [ ] Set up React project (or Streamlit for rapid prototype)
- [ ] Build video replay component with skeleton + club path overlays
- [ ] Build score dashboard: overall score, per-checkpoint breakdown *(the payload is ready:
      `CheckpointScore` carries the band **and** the tour percentile, so a bar can show both)*
- [ ] Build rule-based feedback panel: plain-English tips per checkpoint *(the ranking, severity and
      headline landed in M5-FB; what's left is rendering `FeedbackPayload`)*
- [x] Build session history view *(shipped as a static page rather than waiting on React:
      `api/static/library.html` over `GET /api/sessions` — every session newest-first, each
      swing's score, headline, club and golfer, filterable by golfer, with the aligned render
      playable inline. Trends stayed on the career page, which already reads a golfer against
      their own history; this one answers "which swing was that", not "how am I doing")*
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
- [ ] ~~Mobile companion app~~ — **superseded 2026-09-21 by the app milestones, [M18](#m18-the-platform-decided--a-rust-core-a-python-pose-sidecar-and-a-flutter-shell)–M26**:
      a phone is a **camera** that hears the strike and sends the clip; the machine that analyses
      is a laptop (ADR-030). The charter's "Mobile app" out-of-scope line moved with it
- [ ] Export swing reports as PDF
