# Plan: M31–M40 — shot-first, phone-first

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Findings are appended as milestones close, and a finding contradicting
the plan wins.

**Planned**: 2026-09-29. **Milestones**: M31–M40, written into `ROADMAP.md` by M31 (closed
2026-09-30). **Governing decisions**: [ADR-034](../decisions/034-shot-first-phone-first.md) (written
by M31), amending
[ADR-002](../decisions/002-pose-estimation-mediapipe.md), [ADR-009](../decisions/009-swing-scoring-model.md),
[ADR-010](../decisions/010-benchmark-ranges.md), [ADR-014](../decisions/014-screen-capture-shot-ingestion.md),
[ADR-016](../decisions/016-local-first-host-and-phone-upload-topology.md), [ADR-024](../decisions/024-per-club-shot-history.md),
[ADR-030](../decisions/030-app-platform-rust-core-python-sidecar.md), [ADR-031](../decisions/031-the-capture-edge.md),
[ADR-032](../decisions/032-the-rust-core.md) and [ADR-033](../decisions/033-the-pose-sidecar-protocol.md).
**Re-planned by** [ADR-035](../decisions/035-rust-everywhere-python-where-required.md) (M31.5, closed
2026-09-30), which the next section summarises.

This is a program plan, not one milestone's phase list: it covers ten milestones, and details only
the first one that writes code (M32). Each later milestone gets its own `docs/plans/mNN-*.md` when it
starts, as M23 did.

---

## Re-planned by M31.5 (2026-09-30)

The user redirected the program on 2026-09-30, before M32 wrote any code: **everything is Rust, and
Python stays only where it is required.** [ADR-035](../decisions/035-rust-everywhere-python-where-required.md)
is the decision, and M31.5's plan, [m31-5-rust-first-replan.md](m31-5-rust-first-replan.md), holds the
inventory it rests on (rows R1–R44) and the user's answers (Q1–Q17). This section routes; the clauses
are ADR-035's.

- **The rule** ([clause 1](../decisions/035-rust-everywhere-python-where-required.md#1-the-rule-and-the-two-exceptions-it-names)).
  Two things stay Python: MediaPipe pose, behind `crates/pose`, and the LLM. Clause 1 names every file
  that survives with them. No milestone here plans new Python beyond those two. M31.5 P6b brought
  every section below into line, and where one still reads otherwise, ADR-035 and this section win.
- **The oracle moves to Rust from M32**
  ([clause 3](../decisions/035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)).
  `golf-core` re-records, and every re-record is diff-gated. A port of behaviour the frozen Python
  already has is recorded from Python once, before the port moves. New behaviour gets hand-worked
  vectors.
- **The Python lab is frozen from M32 to M40**
  ([clause 4](../decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)). It
  keeps working, gains nothing, and must keep reading what Rust writes.
- **What moved:**
  - **M32** is Rust-only: the contract, the capability model, the two photo-side reasons and the
    first Rust re-record. All screen-parser work leaves it for M34, as the M32 interview had already
    decided (the M31.5 plan's carried decisions 1–4).
  - **M34** gains that parser work and the 13-shot re-read.
  - **M36 runs before M35** (Q4). M36 ports the many-shot layer faithfully, and M35 then changes it
    in Rust. `shot_result` moves from M36's list to M35's.
  - **M29 is re-scoped as the lab port** (Q2, Q3,
    [clause 5](../decisions/035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped)).
    It comes after M34 and M36 and is no longer blocked on M40. It carries a Rust lab CLI, the `rmcp`
    server, OCR through `ort`, the phone-export verb that M38 P4 now waits on, and the archive move.
  - **M40** decides `api/` (`axum` or drop) and deletes the frozen Python (Q17).
  - **M33 is unchanged**: it measures Apple Vision against the frozen parser, which is using the lab
    rather than changing it.
- **What still stands:** Decisions 1–17 below, in full. The re-plan changes the language things are
  written in and the order they land in, not what the product does. The findings in "What the code
  says" stand too, with the M31.5 errata at their foot.

---

## Status checklist

| Milestone | What | Where it runs | Depends on | State |
|---|---|---|---|---|
| **M31** | The pivot decided (ADR-034, docs only) | desk | – | ✅ Done *(2026-09-30)*, 13/13 phases — [m31-shot-first-adr.md](m31-shot-first-adr.md) |
| **M31.5** | The Rust re-plan: Python only where required ([ADR-035](../decisions/035-rust-everywhere-python-where-required.md), docs only) | desk | M31 | ✅ Done *(2026-09-30)*, 12/12 phases — [m31-5-rust-first-replan.md](m31-5-rust-first-replan.md) |
| **M32** | Wider shot contract and device capability, in Rust; the first Rust re-record | this box | M31.5 | ✅ Done *(2026-10-01)*, 13/13 phases — [m32-shot-contract.md](m32-shot-contract.md). Re-detailed Rust-only below (M31.5 P6a) |
| **M33** | Apple Vision spike: can the phone read the screen? | Mac | M31.5 | ⬜ Not started |
| **M34** | The screen reader in Rust (`crates/screen`): the parser port, the V tile and tie rule, the 13-shot re-read | this box | M32 | ⬜ Not started — unblocked by M32 *(2026-10-01)* |
| **M35** | Shot-first sessions in Rust: photo-only shots, `ShotResult` and `analyze_shot` | this box | M36 | ⬜ Not started |
| **M36** | The many-shot layer and its stores ported to Rust, recorded once from frozen Python | this box | M32 | ⬜ Not started — unblocked by M32 *(2026-10-01)*; runs before M35 |
| **M37** | Strike profile, topic grades and strengths/weaknesses (Rust first) | desk, then bay | M35 | ⬜ Not started |
| **M38** | The iPhone app | Mac, then bay | M34 (skeleton), M37 (profile screens), M29 (P4, the export import) | ⬜ Not started |
| **M39** | Optional video on the phone | Mac, then bay | M38 | ⬜ Not started |
| **M40** | The laptop client resumes (M21/M24/M25/M26 re-scoped, R10/API adapters); `api/` ported or dropped, and the frozen Python deleted | later | M38, M29 | ⬜ Not started |
| **M29** | The lab port: a Rust lab CLI, the `rmcp` server, OCR through `ort`, and the archive move | this box | M34, M36 | ⬜ Not started — re-scoped by ADR-035, number kept (Q2) |

**The order, since two rows are out of numeric order.** M32 comes first. M34 and M36 follow it,
then M35, then M37. M29 needs M34 and M36 and runs on this box beside M37 and M38. M38 P4 waits on
M29, and M40 comes last. M33 needs nothing but M31.5, because the frozen parser it measures is one
M32 no longer touches.

**M33 runs early on purpose.** Whether Apple Vision reads the HD Golf screen well enough is the
product's biggest unknown. It can be tested before any Rust port, by feeding Vision's boxes to the
frozen Python parser. If it fails, this plan changes before any app work starts.

The iPhone app skeleton (M38 P2: photo → read → store → history) can start once M34 lands, in
parallel with M36, M35, M37 and M29.

---

## Why

Today the product is pose analysis of face-on video. The shot photo is "attached and displayed
but never scored" (`docs/ARCHITECTURE.md` §1): `analysis/engine.py` sets `outcome = []`, and
`feedback/rules.py` reads no shot field.

The pivot turns that around:
- The golfer photographs the launch-monitor screen, many times.
- Strengths and weaknesses per club come from those numbers: strike quality (face impact, and low
  point where the device prints it) and distance/direction consistency.
- Video is optional. It is a visual aid ("on this shot your low point was here (red); it should be
  here (green)"), and the pose checkpoints are computed only when video exists.
- The first host is a standalone iPhone app. The laptop stays as a later client.

## Decisions (the user's, 2026-09-29)

1. **HD Golf now, others later.** A second device should be a `profiles.json` entry plus a
   `devices.json` entry, with no code change.
2. **A stat the launch monitor doesn't print is *not printed*, and nothing stands in for it.**
   Nothing is inferred, whether from other numbers or from video. The one sanctioned exception is
   the projected landing offline of [Decision 17 §4](#decision-17--shot-shapes-in-full). This gives
   a *device capability model*:
   - A field that is **not printed** produces no measurement, no `unscored` entry, no caveat and
     no tip.
   - A field that is **printed but blank on one shot** goes through the existing `unscored`
     refusal (ADR-010 §2).
3. **The pose checkpoints stay, but are optional.** They are computed only when video was
   supplied, and a session with no video is first-class.
4. **Keep the laptop, phone first.** The laptop comes back in M40 for:
   - a launch monitor wired in directly (R10 BLE, ADR-004);
   - a launch-monitor API;
   - camera orchestration (M21/M24).
5. **Standalone on the iPhone.** The Rust core runs on-device through Flutter and
   `flutter_rust_bridge`. The user has a Mac, so iOS builds happen there and everything else on
   the Windows box.

### Added in the M31 interview (2026-09-29)

These add to 1–5 rather than replacing them. ADR-034 (M31 P3) turns them into numbered clauses, and
[M31's plan](m31-shot-first-adr.md#decisions-taken-in-the-m31-interview-and-why) keeps the
interview's record of each. A milestone that wants to revisit one needs a reason the interview did
not have.

6. **Vocabulary: shots are *tracked*, device stats are *printed*.** A stat the launch monitor does
   not print is "not printed", never "not tracked". So M32's function is `printed_fields(shots)`,
   and printed = declared ∩ `fields_present`. "Tracked" had come to mean two things, and the user's
   own use of it is about shots, so the device side gets the other word.
7. **Which shots count is derived from intent.** It reuses ADR-009's `PracticeGoal`
   (`contracts/intent.py`), whose `mode` is set per session and overridable per shot, exactly as
   ADR-009 §Concepts already says.
   - A `DRILL` shot is not tracked. Every other mode is, `SHOT_SHAPING` included.
   - It is derived from the mode, never stored as a second flag.
   - An untracked shot is still stored, shown in history and analysed on its own. It never enters
     club or player stats.
   - It is distinct from ADR-028's mishit rule, which is automatic and metric-scoped. This one is
     the golfer's choice and whole-shot.
   - The user: "Some drills we do not want to add to the player stats as if they were actually
     swinging or actually playing."
8. **Two levels, club and player.** Every tracked shot counts toward its **club's** stats and the
   **player's** overall stats.
   - **Raw player-level averages exist only for club-independent stats**: strike location,
     face-to-path, start line, consistency rates.
   - Carry, ball speed, launch and spin are per club only. A driver-plus-wedge carry average means
     nothing.
   - Each stat declares its scope.
9. **Grades: a grade plus a list, per topic.**
   - A topic's grade is the **share of good shots**: the % of the club's tracked shots meeting the
     topic's criterion.
   - It is carried as a rate with a Wilson interval (`RateEstimate`, M37) and **withheld below the
     minimum n**.
   - No tour bands. A share is honest without one, and an outlier lowers a grade naturally, which
     is what the user asked for: "how often they hit an average shot (so like a huge outlier would
     make this score lower)".
10. **Six topics.**
    - **Strike location**: where on the face, as the centered rate and heel/toe.
    - **Consistency**: how often a shot lands near the golfer's typical shot for that club.
    - **Low point**: only where the device prints it, so **never for HD Golf**.
    - **Fade**, **Draw** and **Straight**: three shot-shape topics, each graded **only over shots
      declared as that shape** (Decision 17).
    - **A shape never declared is absent**: not graded, not mentioned, not in a blend.
    - The exact criterion for strike location and consistency is M37's, set against hand-worked
      vectors.
11. **Two blends, one per club and one for the player.** Each is the **equal-weight mean of the
    graded topics**, and the three shape topics count as three topics. The weights are equal
    because no weighting has been measured.
    - At player level, a topic's share pools every tracked shot across clubs, with **each shot
      judged against its own club's criterion**. That is how shares pool honestly where raw
      averages cannot, and it is what lets a player-level consistency exist at all.
    - A topic that is printed but ungraded (n too small) is excluded from the blend **and named**,
      as `unscored` checkpoints are today.
    - A topic that cannot exist (not printed, or a shape never declared) is simply absent.
12. **Mechanics is a separate panel.** When video exists, the pose checkpoints are scored as today
    (ADR-009's mechanics axis) and **never enter a topic grade or a blend**. Adding video must never
    move a club grade, and a golfer with no video is graded on the same terms.
13. **Android is deferred.** iPhone only for M31–M40. The `TextRecognizer` boxes seam keeps ML Kit
    reachable later, and nothing is designed against Android.
14. **M26 (Ship it) is paused and re-scoped under M40 whole**, like M21, M24 and M25. M38's free
    Personal Team signing is the iOS path until then.
15. **M31 brings more docs in line** than its file list first named: `docs/PROJECT_CHARTER.md` §0,
    the root `README.md`, `docs/FLOW.md` (§1 and §4 redrawn, §2 and §3 bannered), and addenda to
    ADR-002, 010, 031 and 033. Each says something the pivot made false: the charter says the phone
    is only a camera, and FLOW draws iPhones → Tailscale → laptop.
16. **`ROADMAP.md`'s M31–M40 sections are short and route here**: status, the ask, depends-on,
    where it runs, exit, and a link. The detail lives in one place.
17. **Shot shapes are graded only when declared, from the numbers.** In full below.

### Decision 17 — shot shapes, in full

1. **Graded only when a shape was declared.** The declaration is `PracticeGoal.target_shape`
   (`STRAIGHT`/`DRAW`/`FADE`, which already exist in `contracts/intent.py`), with mode
   `SHOT_SHAPING`. It is set in one of two ways:
   - **by the golfer**, per session or per shot;
   - **by a challenge mode**, where the app calls fade, draw or straight before each shot and
     records the call as that shot's `target_shape`.

   Challenge shots are shot-shaping shots, not drills, so they **count fully**: they feed the shape
   topics and every other topic.
2. **Classified from numbers, not the screen's word.**
   - A shot is **straight** when its face-to-path is within `METRIC_TARGETS["face_to_path_deg"]`'s
     tolerance (`contracts/dispersion.py`; reused, not a new constant).
   - Otherwise it is a **fade** or a **draw** by the sign, which flips for a left-hander.
   - Missing face-to-path on a shot → that shot is unscored for shape. Missing handedness →
     `no_handedness`.
   - HD Golf's `Shot Type` word is **not** used to grade. It stays what it is today: the
     cross-check on the spin-axis sign.
   - *Preliminary, from planning (2026-09-29):* face-to-path is computable on 12 of the 13 stored
     shots, and the word and the numbers disagree on one of them (−1.3°, printed `SLIGHT FADE`).
     M31 P2 confirms or corrects both.
3. **Two shares per shape.**
   - **"Hit the shape"**;
   - **"hit the shape and finished on line"**.

   **The second is the topic's grade** in the blends. The first shows beside it, to say whether the
   shaping or the aiming failed.
4. **"Finished on line"** uses the device's printed landing offline where it prints one. **HD Golf
   (as seen so far) does not**: its `Horizontal Angle` is the start direction, not the finish.
   - Where the offline is not printed, it is **projected from the printed start direction and
     carry, including the curve** (ADR-027's flight model).
   - It is **named as projected** everywhere it appears, following ADR-024 §3's start-line
     projection and ADR-027 §6's measured/simulated split.
   - Where the projection cannot be computed, the on-line share is withheld and named.
   - **This is the one sanctioned exception to Decision 2 ("never inferred")**: the user's choice,
     made knowing the objection.
   - It must include the curve, because a start-line-only projection would mark every well-hit fade
     offline (a fade starts left on purpose).
   - The on-line tolerance comes from `METRIC_TARGETS`' offline row unless M37 finds a reason
     otherwise.
   - If the bay trip finds that HD Golf's `Custom` tile can print a landing offline, the printed
     value replaces the projection for this device, with no code change.

The user: "If I am trying to hit a draw, grade how well I draw and then land the ball on target. If
you don't know don't score it." Then: "how often they can actually hit a fade when told to hit a
fade… for a fade, draw, and straight shot."

### How the grades relate to what already exists

- **ADR-009's per-swing `outcome_score` stays `None`.** A share of one shot is 0 or 100 and means
  nothing, so grading happens at club and player level, over many shots. ADR-009's
  shot-shaping/performance/drill *policies* for a single swing stay unbuilt. The three shape topics
  (Decisions 10 and 17) are where `SHOT_SHAPING` and `target_shape` finally get judged.
- **The strengths/weaknesses list keeps its rule** (M31's "The ADR records" §5, below).
  - A strength needs an established equivalence claim: the CI of the mean inside target ±
    tolerance, *and* the SD's upper bound below tolerance.
  - A weakness is an established bias or scatter, ranked by how far it exceeds tolerance.
  - "Nothing established" is never a strength.
  - The grade and the list sit side by side: the grade says how often, and the list says what is
    established.
- **"No score beats a wrong one" gains two neighbours** (ADR-010 §2):
  - *not printed* produces nothing at all: no measurement, no `unscored`, no caveat, no tip, no
    topic;
  - *not declared* (a shape never attempted) produces no topic.

  A topic that *is* printed but has too few shots is withheld and named.
- **One sanctioned inference**: the projected landing offline of Decision 17 §4. It is named as
  projected wherever it appears, and ADR-034 records it as the only exception to "never inferred".

## What the HD Golf data can and can't support

- **It prints no attack angle, dynamic loft or low point** (ADR-027 says so too), so low point is
  never printed for HD Golf, and the low-point topic (Decision 10) is absent for it.
- **Face impact is text only.**
  - Horizontal: `impact_position` is one of {CENTER, HEEL, TOE} on every stored shot.
  - Vertical: the bay's screen shows an `Impact Position V` tile the profile doesn't read (ROADMAP
    §M3, ADR-014 addendum). It read `---` on the bay photos checked.
  - The screen layout is configurable (there is a `Custom` gear tile), so a bay trip should list
    which tiles can be enabled.
- **Spin is blank on 11 of the 13 stored shots.** Smash reads 0.89–1.00, which is physically
  implausible, so club speed and smash become `shown_only`. `shot_measure.py`'s docstring already
  excludes them for this reason.
- **So for HD Golf, "strike quality" is:**
  - the face-impact grid per club, and the centered-strike rate;
  - ball-speed consistency;
  - carry, start-line and face-to-path bias against scatter.

## What the code says, before anyone re-derives it

These are findings from the planning session's read of the code (2026-09-29). Verify each one
before leaning on it.

1. **The capability model needs a per-golfer intersection, because layouts vary per user.** Bay
   photos have `Custom` and `Impact Position V` where the two reference photos in
   `data/raw/shot_screens/` have `Bounce & Roll`. So:
   - each device declares the fields it can print;
   - printed = declared ∩ the tiles actually seen on that user's photos.
2. **The corpus already keys launch-monitor samples on the photo:** `CorpusSwing.artifact_key`
   returns `shot:{sha}`. What is swing-bound:
   - Admission: `read_corpus` excludes a manifest with no face-on clip
     (`ExclusionReason.NO_FACE_ON`, `storage/corpus.py:97`).
   - Measurements: they come only from `analysis.json`.
3. **Adding fields to `ShotData` forces a vector re-record even though no number moves:**
   - Corpus vectors take `input.shot` from the stored `analysis.json`.
   - `compare_results` requires identical key sets.
   - `crates/contracts`' `round_trip.rs` distinguishes an absent key from a null one.
4. **`mcp/query.py::_METRIC_FIELDS` / `_NON_METRIC_FIELDS`** are checked against `ShotData`
   exhaustively by `tests/mcp/test_query.py`.
5. **The screen parser has Python-specific behaviour that a Rust port must reproduce exactly.**
   None of it is on ADR-032 §3's list of edges:
   - `difflib.SequenceMatcher.ratio` (`profiles.py:76`);
   - the look-behind regex `_THOUSANDS` (`parser.py`), where the Rust `regex` crate has no
     look-around;
   - float floor division `center_y // bucket`;
   - `{text!r}` inside warnings, which are compared exactly;
   - tie rules: `field_for` uses `>=` (the last field wins), `_find_labels` uses `>` (the first
     box wins);
   - Unicode behaviour of `str.upper()` and `split()`.
6. **`Impact Position` and `Impact Position V` score 0.9375 against each other**, against a
   threshold of 0.8.
   - Exact text resolves correctly.
   - An OCR misread such as `Impact Position Y` could land on the wrong field. It fails as
     "missing", never as a wrong value, but it needs a test.
7. **What pose runs today:**
   - MediaPipe Tasks `PoseLandmarker`, `RunningMode.VIDEO` (`pose/estimator.py`).
   - The model comes from a `float16/latest` URL (`estimator.py:45`), so **the model bytes are
     unpinned**.
   - iOS has the same Tasks API and `.task` format, so conforming is plausible, not guaranteed.
     Decoder, delegate, version skew and the unpinned model all differ.
8. **Two places say "OCR stays Python":** `docs/CONFORMANCE.md` §5, and the `profiles.json`
   comment in `tests/test_conformance.py::_PACKAGE_DATA`. Both change once the parser ports.
9. **`bundle_store` attaches an upload to "the newest swing lacking this role."**
   - Photo-only sessions are fine: each photo opens a new entry.
   - Mixed sessions misattach. The phone's storage API should take explicit targets.

**Errata, M31 P2 (2026-09-29).** These were measured on this box. The detail and the commands are
in [M31's P2 findings](m31-shot-first-adr.md#p2--found-2026-09-29), and where this plan disagrees
with them, they win.

- **"What the HD Golf data…" section.**
  - Smash reads **0.76–1.06** over the 13 stored shots, not 0.89–1.00. That is still implausible,
    so the exclusion stands.
  - `impact_position` is populated on **12 of 13** shots, not on every one.
- **ADR-027 says HD Golf prints no attack angle, and nothing more.** It is silent on dynamic loft
  and low point, so their absence rests on the tile inventory instead.
- **Finding 1.**
  - **`Custom` is on both layouts**: it is a settings gear. The two layouts differ by one tile,
    `Bounce & Roll` ↔ `Impact Position V`.
  - The bay photos number 13 and are every stored shot. The two reference photos are none of
    them.
- **Finding 6's hazard has been observed on real photos.**
  - On 2 of the 13 bay photos, the OCR drops the `V`.
  - On `2026-08-10-1` the V tile then won the `Impact Position` field. That blanked the field and
    pushed `CENTER` into `Shot Type`.
  - This is the actual cause of the bug ADR-014's addendum blames on an uncropped cell boundary.
- **Decision 17 §2.** The face-to-path sign that separates a fade from a draw **does not flip for a
  left-hander**, because the contract signs face, path and spin axis golfer-relative
  (`flight_infer._direction_of`). Handedness decides which *side* of the line a shape finishes on,
  so it belongs to §4's projection.
- **Decision 17 §4.** The curved projection reaches **2 of 13** stored shots, and **none** of the 11
  from the 2026-08-23 bay session, because the curve needs a printed spin axis. At the bay as seen,
  the on-line share is withheld on every shot.

**Errata, M31.5 (2026-09-30).** These come from the session that set out to plan M32 and was
redirected. They are the M31.5 plan's
[planning findings 1–3](m31-5-rust-first-replan.md#what-the-planning-read-found-2026-09-30), and all
three now land in M34, where the parser work went.

- **The old §M32 sequence's step 4 was wrong.** `scripts/import_shot_screens.py --force` on
  `data/raw/shot_screens/` parses `IMG_2738.jpeg` and `IMG_2739.jpeg`, which are not stored shots
  (M31 P2 finding 2), and would *add* them to `data/processed/shots/`.
  - The 13 stored shots are re-read through `api/pipeline.py::_shot_for`, which looks the shot up in
    `ShotStore` by the manifest's photo sha256. A stale-cache check under `scripts/reanalyze.py`
    would drive that path.
  - **The lesson carries to M34's Rust re-read: re-read through the bundle, never through the bulk
    importer.**
- **Adding a tile to a profile moves `parse_confidence`.** `parser._score` divides by
  `len(profile.stored_fields)`, so adding `Impact Position V` moves every bay shot by about −0.02: the
  V cell reads empty, so value coverage drops. All 13 stored shots sit at 0.897–0.948 against
  `validate_parse`'s 0.6, so nothing flips. M34's exit allows the move by name.
- **The V tile will add `Impact Position V: no value text under the label` to every bay shot**, as
  Spin and Spin Axis already do, because the OCR never sees `---` on those photos.

## Reuse, don't rebuild

Each entry names its Rust home, or the milestone that builds one. The Python beside it is frozen
([ADR-035 clause 4](../decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)):
read it to learn the behaviour a port must reproduce, and never extend it.

- **Photo → shot.** Frozen Python's chain is `launch_monitor/screen/`: `import_screen` →
  `prepare_screen` → `TextRecognizer` (PaddleOCR) → `parse_screen` → `validate_parse` →
  `to_shot_data` → `ShotStore.put`, with `scripts/import_shot_screens.py` as its bulk CLI.
  - The parser and the validator: `crates/screen` (M34).
  - The recognizer: Apple Vision on the phone (M33, M38), and `ort` running the same Paddle models in
    the lab (M29). The preprocessing goes with each: VisionKit on the phone, a Rust port in the lab.
  - `ShotStore`: `crates/storage`'s shot store (M36).
  - The import and its bulk CLI: verbs of the Rust lab CLI (M29).
- **Per-shot numbers:** `crates/analysis`'s `shot_measure`, `flight`, `flight_infer`,
  `flight_measure` and `spin_solve`, ported and conforming since M22. Frozen Python's
  `analysis/shot_measure.py`, `flight*.py` and `spin_solve.py` are what their vectors were recorded
  from.
- **The many-shot layer** has no Rust twin yet. M36 ports each piece faithfully:
  - `analysis/baseline.py::build_baseline` → `crates/analysis`;
  - `analysis/dispersion.py::build_dispersion` → `crates/analysis`, with `METRIC_TARGETS` and
    `PATTERN_READING` (both in `contracts/dispersion.py`) → `crates/contracts`;
  - `analysis/club_profile.py::build_bag_profile` → `crates/analysis`;
  - `contracts/mishit.py::mishit_carry_floor` → `crates/contracts`;
  - `storage/corpus.py::read_corpus` → `crates/storage`. M35 then changes it.
- **Club specs:** `clubs/` (ADR-026). The catalogue ports with the bag (M36), and Rust reads the JSON
  `clubs/lookup.py` writes. The lookup itself stays Python, because it is an LLM call
  ([clause 1](../decisions/035-rust-everywhere-python-where-required.md#1-the-rule-and-the-two-exceptions-it-names)).
  **Per-club tagging:** ADR-024.
- **Video:**
  - impact frame: `crates/analysis`'s `phases` and `alignment::with_measured_impact`, plus
    `crates/trigger`, all ported already;
  - drawing: `pose/overlay.py::annotate_frame` is **not ported** (the M31.5 plan's Q10). It is
    deleted in M40, and the phone draws its own overlay (M39 P2);
  - pose: `crates/pose` + `pose/worker.py` on the laptop, which is the MediaPipe exception.

---

## M31 — ADR-034, "Shot-first: the launch-monitor screen is the product, the phone is the host"

**Phases:** [m31-shot-first-adr.md](m31-shot-first-adr.md) holds M31's phase list, the interview's
record of Decisions 6–17, and what each phase found.

**Files:**
- `docs/decisions/034-shot-first-phone-first.md`
- Addenda to ADR-002, 009, 010, 014, 016, 024, 030, 031, 032 and 033.
- `docs/README.md`: the ADR row, plus the addendum counts. Both are pinned by
  `tests/test_docs_truth.py`.
- `ROADMAP.md`: status-table rows M31–M40 and the section for each.
- `CLAUDE.md`: the product sentence at the top, and the "What is the app written in" row.
- `docs/CONFORMANCE.md` §5.
- `docs/PROJECT_CHARTER.md` §0, and the root `README.md` (Decision 15).
- `docs/FLOW.md`: §1 and §4 redrawn, §2 and §3 bannered (Decision 15).

**Roadmap changes:**
- M21, M24, M25 and M26 are paused and re-scoped under M40. M26 moves whole: CI, packaging,
  signing and distribution (Decision 14).
- M28 is superseded by M39.
- M29 is blocked on M40 instead of M25. Its job is unchanged.
- M3's open OCR items fold into M32 and M33.
- "M4 full" is superseded by M37.
- M5's "superseded in shape by" moves from M25 to M38.

**The ADR records:**
1. **The unit of the product is the shot.** Video is optional, and mechanics are a separate panel
   (Decision 12). It also records the capability model:
   - each device declares every field it prints as `analysed` or `shown_only`;
   - printed = declared ∩ `fields_present`, across the golfer's shots.
2. **ADR-030:**
   - §5 and the "the machine is a laptop" premise are superseded;
   - §1 is amended: storage is on-device too;
   - pose on the phone is reopened, behind a conformance gate (M39 P0).
3. **OCR on the phone:**
   - Apple Vision sits behind the `TextRecognizer` boxes seam, with VisionKit doing rectification;
   - the parser and validator port to Rust;
   - PaddleOCR and OpenCV stay as the lab's reader;
   - a vision LLM stays rejected, on ADR-014's grounds.
4. **The oracle for each vector family:**
   - ports of existing Python are recorded from Python;
   - *new* analysis is Rust first, against hand-worked vectors (`provenance.oracle: "hand"`);
   - the lab reaches Rust-only analysis through `golf-core` subcommands.
5. **Bands:**
   - Shot metrics get no tour bands; they are judged against the personal baseline plus the
     targets already in `METRIC_TARGETS`.
   - A **strength** needs an established equivalence claim: the CI of the mean lies inside
     target ± tolerance, *and* the upper bound of the SD is below tolerance.
   - "Nothing established" is never a strength.
6. **No LLM coaching on the phone,** since it would need the network and an API key.
7. **The interview's Decisions 6–17:** the vocabulary, tracked shots, the two levels, the grades,
   topics and blends, mechanics as a separate panel, and shot shapes with the one sanctioned
   inference. [M31's P3](m31-shot-first-adr.md#p3--write-adr-034) sets out ADR-034's clause order.

**Exit:** `pytest tests/test_docs_truth.py` passes.

## M32 — Wider shot contract and device capability (detailed)

**As built (2026-10-01):** this section is the design, and [the M32 plan](m32-shot-contract.md) is
what built it, phase by phase. Its "Phase findings" carry what building found, and where a finding
contradicts this section the finding wins. Two corrections were made before building, as that
plan's calls 1 and 2:
1. **The order.** The Python tooling landed *before* the bump and the re-record, not after them as
   "Sequence" below has it, so that no phase ended with `pytest` red. The frozen view is a no-op on
   a vector with no ledger, so it was proved on fixtures first.
2. **A gap in "What changes in Python".** `tests/test_conformance.py::test_a_vector_round_trips_through_the_stdin_seam`
   also compares frozen Python's output with a synthetic vector's `expected`, so it joined the
   frozen view with the two tests named below.

One more was found while building: the typo guard and "a second run writes nothing" contradict each
other as written below, and the ledger reconciles them
([ADR-032](../decisions/032-the-rust-core.md)'s 2026-10-01 addendum).

**Rust only.** M31.5 P6a re-detailed this section under ADR-035's clauses 3, 4 and 6. It is L3:
`crates/contracts`, `ANALYSIS_VERSION` and the committed vectors. It is also where the oracle moves.
M32 does the first Rust re-record, so the gate that keeps a re-record honest is built here and used
here first.

**What does not change:**
- Frozen Python's contracts, parser and engine, and its `ANALYSIS_VERSION` of 16.
- `data/`: nothing under it is read or written.

The one Python change is to the conformance tooling, because that tool's job is what moves ("What
changes in Python, and why", below).

**What left this section when M31.5 re-planned it** (carried decisions 1–4 of the
[M31.5 plan](m31-5-rust-first-replan.md#decisions-carried-to-m32-and-m34-from-the-m32-interview-2026-09-30)):
- **To M34, in Rust against `crates/screen`:**
  - everything in the old "Screen package" list: `profiles.json`'s V tile, `ProfileField.scale`,
    `ParsedShot.fields_present`, the scale in `_read_cell`, the stamping in `to_shot_data`, the
    ranges in `validate.py`, and the stale-cache miss in `import_screen`;
  - the stale-cache check in `api/pipeline.py::_shot_for`. M34 lands the stamping it reads, and its
    caller is the lab's shot lookup, which M29 ports (§M34 says why);
  - the old sequence, and the re-read in the old exit.
- **To M29: `mcp/query.py::_METRIC_FIELDS`.** The frozen MCP server never sees the new keys. The
  Rust tool surface is the `rmcp` server's, so that is where the new fields join a metric list. The
  M31.5 plan's P6a findings say why this item did not go to M34.
- **Dropped:**
  - `contracts/capability.py`, and the new fields on Python's `ShotData`;
  - `devices.json` joining `tests/test_conformance.py::_PACKAGE_DATA`. The file lives in `crates/`,
    and that pin only scrapes `src/golf_coach/`.

### The shot contract — `crates/contracts/src/shot.rs`

`ShotData` gains seven fields. Each is an `Option` with `#[serde(default)]`, and none is bounded, for
the reason the struct's own doc gives:

| Field | Units / meaning |
|---|---|
| `attack_angle` | deg, + = up |
| `dynamic_loft` | deg |
| `low_point` | in, + = ahead of the ball |
| `impact_offset_h` | mm, + = toe |
| `impact_offset_v` | mm, + = high |
| `impact_position_v` | text |
| `carry_offline` | yd, + = right |

`ShotProvenance` gains three, each `#[serde(default)]`:
- **`parser_version: i64`, default 0.** It records which parser produced the shot. 0 means
  unstamped, which is every shot the frozen Python parser wrote. `Validate` refuses a negative.
- **`fields_present: Option<Vec<String>>`.** It lists the tiles the parse located, blank or not.
  `None` means not recorded, and every stored shot reads `None` until M29's Rust lab re-reads it.
  M34's re-read runs over committed boxes and writes no `data/` (§M34).
- **`corrections: BTreeMap<String, String>`**, for M38's review and edit. It is added now so that
  M38 does not force a second re-record, and it is a `BTreeMap` for the reason `raw_fields` gives.

Also:
- **`SCREEN_PARSER_VERSION`**, with a ledger comment in the form `ANALYSIS_VERSION`'s has
  (`crates/contracts/src/swing.rs:39`).
  - Its first entry is 1, which is M34's Rust parser. Nothing stamps it before M34.
  - Whether M34's faithful port and its tie rule are one version or two is M34's call.
- **`parse_is_current(shot) -> bool`.** It returns `provenance.parser_version >=
  SCREEN_PARSER_VERSION`, and `true` for a shot with no provenance, because a direct feed has no parse
  to redo. It has no caller until a Rust shot lookup exists: the lab's in M29, or the phone's store in
  M38, whichever lands first (§M34).

### The capability model — `crates/contracts/src/capability.rs` and `crates/contracts/devices.json`

- **`FieldUse { Analysed, ShownOnly }`**, with a `note` on every `ShownOnly` field that says why.
- **`DeviceCapability`**, loaded from `devices.json` by `include_str!`, which keeps ADR-032 §5's one
  copy on disk. The file sits in `crates/` because nothing Python reads it. Its entries:
  - **`hd_golf`** declares the `target`s of its profile in `launch_monitor/screen/profiles.json`,
    plus `impact_position_v` from the bay layout.
    - `club_head_speed` and `smash_factor` are `shown_only`. Their note cites the smash measurement,
      which the stale-comment fix below moves into `crates/analysis/src/shot_measure.rs`.
    - `attack_angle`, `dynamic_loft`, `low_point`, both `impact_offset_*` fields and `carry_offline`
      are not declared.
    - The pin that holds `hd_golf` equal to its profile is M34's, because the profile gains its V
      tile there.
  - **`mock`** declares what `launch_monitor/mock.py` fills in.
- **`capability_for(device)`** refuses an unknown device with a `ContractError` rather than guessing a
  default.
- **`device_of(shot)`** returns `provenance.device`, and falls back to `ShotSource::as_str`.
  - That rule exists today inside `crates/analysis/src/engine.rs::shot_measurements`, which builds
    `launch_monitor:{device}` from it.
  - M32 moves the rule here and has `shot_measurements` call it, so there is one definition.
- **`printed_on(shot)` and `printed_fields(shots)`.** The M31.5 plan left their meaning to P6a, and it
  is settled here:
  - **`printed_on(shot)` is declared ∩ `fields_present`, for one shot.** So a shot from a layout
    that lacks a tile is excluded from that field, and never counted as blank on it.
  - **`printed_fields(shots)` is the union of `printed_on` over the golfer's shots.** That is ADR-034
    §2's "declared ∩ `fields_present`, across the golfer's own shots".
  - **A direct feed** (a shot with no provenance) prints everything its device declares.
  - **An unstamped screen shot** (`fields_present: None`) is read conservatively, as declared ∩ the
    fields it holds a value for.
    - So a blank on such a shot is excluded rather than named `printed_blank`.
    - That under-reports blanks on the 13 stored shots until M29's re-read ends it (§M34 says why
      it is not M34's).
    - It never invents one, which is the order ADR-010 §2 sets.

### Two photo-side reasons — `crates/contracts/src/unscored.rs`

`PrintedBlank` (`printed_blank`) and `Misread` (`misread`) each get a row in `UNSCORED_REASONS`. This
is the M31.5 plan's carried decision 5.

- **`printed_blank`**: the tile was found, and nothing readable sat under it, whether `---` or empty.
  `refilming_helps` is **false**.
- **`misread`**: text sat under the tile, and no value could be read from it. `refilming_helps` is
  **true**, because retaking the photo may fix it.

Neither is an inference reason, and nothing emits either in M32. M35 and M37 are the first to write
them, into `shot_analysis.json`. They must never reach `analysis.json`, because frozen Python's
`UnscoredReason` is a closed enum and would refuse them (ADR-035 clause 4).

### `ANALYSIS_VERSION` 16 → 17, in Rust only

- **The change is in `crates/contracts/src/swing.rs:39`.** The ledger entry says it is shape only:
  `ShotData` and `ShotProvenance` gained keys, and no number moved.
- **"What the code says" finding 3 forces the bump**, even though no value changes.
- **`contracts/swing.py` stays at 16** (ADR-035 clause 3).
  - `api/state.py::is_outdated` compares with `<`, so frozen Python reads a v17 artifact as current.
  - Nobody should "fix" the gap by bumping Python.

### The re-record — `golf-core rerecord`, in `crates/core`

**The comparator becomes library code.** `crates/core/tests/engine.rs::compare` implements
`docs/CONFORMANCE.md` §3's rules over `serde_json::Value`.
- It moves to `crates/core/src/compare.rs`, together with the test that exercises it,
  `the_comparator_sees_the_differences_it_exists_to_see`.
- Each difference it reports carries a path and a kind: an added key, a removed key, or a moved value.

**The stage document gets a Rust producer: `crates/core/src/stages.rs`.** It ports two functions:
- `scripts/conformance.py::run_stages` (`:341`), which assembles the seven stages;
- `scripts/conformance_vectors.py::_verify_stages_compose` (`:454`), which checks that they compose
  onto the bundle's answer.

It lives in `core` rather than `analysis` because it is vector plumbing, and `analysis` stays a
library that knows nothing about `spec/`. The stage tests in `crates/analysis/tests/` keep their own
orchestration: they gate the port one stage at a time, and the compose check is what catches this new
copy drifting.

**The verb.** `golf-core rerecord --declare <file>` walks the engine family (`synthetic/` and
`corpus/`) and the stage family. For each vector it does five things:
1. It runs Rust on the committed input: `run` for an engine vector, and `stages.rs` for a stage
   vector.
2. It compares that output with the committed output, through `compare.rs`.
3. It fails the whole run, and names every path, if any difference is not covered by the
   declaration. The declaration covers an added key at a declared pattern, and a moved value at a
   declared path.
4. It writes the **committed document, with only the declared paths replaced**.
   - So every undeclared value stays exactly as Python recorded it, bit for bit. That is ADR-035
     clause 3's "every value Python recorded survives in the file".
   - The gate compares within §3's tolerance, but the file never takes Rust's in-tolerance floats.
5. It writes nothing if nothing differs, so a second run is a no-op.

**How the file is written.** It matches `conformance.py::_write_json`:
- through `serde_json::Value`, which sorts keys because the workspace does not enable
  `preserve_order`;
- with an indent of 2;
- with `corpus/`'s gzip at level 9 and mtime 0. `flate2` moves from `crates/core`'s dev-dependencies
  to its dependencies.

The text still churns, because `serde_json` spells 77 exponent-form floats as decimals and does not
`\u`-escape. So the verb's report is what gets reviewed, never `git diff`.

**What a re-recorded vector says about itself.** ADR-032's thirteenth addendum left this question to
M32:
- **`provenance.oracle` stays `"python"`**, because every value M32 does not declare is still
  Python's.
- **A new `provenance.rerecords` list gains one entry per re-record.** The entry holds the version it
  moved to, `"by": "golf-core rerecord"`, and the declaration: the added-key patterns and the moved
  paths.
- So each file says which of its values are Rust's.
- M32 writes `oracle` onto the 42 files it re-records. M37's pin, that every family names its oracle,
  extends the key to `format` and `audio`. It can also require every `rerecords` entry to carry a
  declaration.

**M32's declaration, in full:**
- **Added**: `expected.swing.shot.{attack_angle, dynamic_loft, low_point, impact_offset_h,
  impact_offset_v, impact_position_v, carry_offline}` and
  `expected.swing.shot.provenance.{parser_version, fields_present, corrections}`.
- **Moved**: `analysis_version` and `expected.analysis_version`, 16 → 17.
  - That is two paths, not one (the M31.5 plan's P4 finding 5).
  - A stage vector has only the first.
- **The synthetic vectors carry no shot**, so for them the declaration is the version move alone.
- **Inputs are not rewritten.** `input.shot` keeps the shape Python wrote. The frozen lab goes on
  writing exactly that shape until M40, and Rust has to read it.

**`golf-core`'s module doc is rewritten along with the verb.** Its "`regenerate` is deliberately not a
candidate" paragraph was ADR-032 §7's position before a port conformed. It now says why a gated
re-record is not a self-portrait (ADR-035 clause 3).

### Schemas: split ownership, with the Rust half hand-maintained

ADR-035 left this choice to M32, from the M31.5 plan's P1 finding 2, and it is settled here. The
answer is candidate (c)'s split, using candidate (b)'s mechanism for the Rust half.

- **Three roots become Rust-owned in M32**: `shot_data`, `swing_result` and `swing_bundle_result`.
  They are the roots whose shape the new keys move.
- **The other seven stay generated** by frozen Python's `export_schemas`. That includes
  `keypoints_file`, because the pose worker survives and writes it.
- **The three files are edited by hand.** The edit adds the seven `ShotData` properties and the three
  `ShotProvenance` ones to each file, spelled the way pydantic spells their neighbours. The last two
  files embed both types in `$defs`.
- **Why not `schemars`** (candidate (a)):
  - The three files carry 78 bound keywords (`minimum`, `maximum` and their exclusive forms), counted
    on 2026-09-30. All of them come from pydantic `Field`s.
  - Rust holds those bounds in its `Validate` impls, where `schemars` cannot see them. So its first
    run would drop them.
  - Keeping them would mean stating every bound a second time, in `#[schemars(range)]`, and a second
    copy drifts.
  - It would also add a dependency to the crate the phone compiles, for a file no Rust code reads.
  - M36 should revisit this if hand-maintenance proves the larger cost once it takes the storage
    roots.
- **The pin is a new `crates/contracts/tests/schemas.rs`.**
  - In each Rust-owned file, the property sets of `ShotData` and `ShotProvenance` must equal the key
    sets the Rust structs serialize.
  - So a key added to either struct without a schema edit fails `cargo test`.
  - The pin extends shape by shape as later milestones take roots over.

### What changes in Python, and why

**This is the one place M32 touches Python.** The lab itself is frozen and stays untouched. The
oracle's tooling moves because the oracle does. Left alone, two things would break (the M31.5 plan's
P1 finding 3):
- `conformance.py check` would report all 42 re-recorded vectors STALE, and exit 1;
- in `tests/test_conformance.py`, the engine-conformance pin and both version pins would fail, and so
  would the schema-freshness pin once three schema files change.

**`scripts/conformance.py`:**
- **A frozen view**, which both `check` and the tests use.
  - It removes every path a `rerecords` entry declared, from `expected` and from frozen Python's
    output alike, and compares what is left.
  - That certifies that frozen Python still reproduces every value it recorded, which is ADR-035
    clause 4's freeze, checked.
  - It no longer certifies the vectors. From M32, `cargo test` does that.
- **`cmd_check`** compares a ledgered vector in the frozen view, and skips its version test. A vector
  without a ledger is judged as it is today.
- **`regenerate`** refuses the engine and stage families, on the precedent of
  `conformance_vectors._audio`.
  - A Python rebuild would overwrite the Rust-recorded v17 vectors with v16 ones, and drop the
    ledger.
  - The refusal covers the full `regenerate` and `--stages-only`.
  - `--schemas-only` writes only the Python-owned roots, through a new `RUST_OWNED_SCHEMAS` set.
  - `SCHEMA_ROOTS` keeps all ten, so `test_every_on_disk_artifact_has_a_schema` still maps every
    artifact to a root.
  - `--format-only` is unchanged.

**`tests/test_conformance.py`:**
- **`test_every_committed_schema_is_what_contracts_exports_today`** (`:52`) skips
  `RUST_OWNED_SCHEMAS`.
- **`test_each_vector_still_conforms`** (`:163`) and **`test_each_stage_vector_is_what_this_build_produces`**
  (`:420`) compare in the frozen view.
  - Without the view, the first would fail on every re-recorded vector.
  - The second passes either way in M32, because the declaration reaches no stage.
- **`test_each_vector_was_recorded_at_the_current_engine_version`** (`:178`) and its stage twin
  (`:403`) are re-pointed.
  - The new rule: a vector recorded above frozen Python's version carries a `rerecords` entry for
    each version above it.
  - The "recorded at the current version" assertion already lives against Rust's constant, in
    `crates/core/tests/engine.rs` and in four stage test files.
- **`test_each_stage_vector_still_composes_onto_its_bundle_answer`** (`:440`) is unchanged.
  - It composes phases, measurements, checkpoints and `unscored`, and M32's declaration touches none
    of them.
  - A later declaration that does touch them re-points it the same way.

**`conformance.py check` leaves "Verify, every milestone" here.** The pytest pins above carry the
freeze. The command still runs, but it now means "the freeze held" rather than "the vectors are
right".

### The stale comments, fixed in the Rust mirrors

This is the M31.5 plan's carried decision 6. The Python copies stay frozen (ADR-035 clause 4), and
stay wrong until M40 deletes them.

- **`crates/analysis/src/shot_measure.rs`:**
  - Its module doc (near `:16–18`) and `measure_ball_speed` (near `:111–113`) point to "the Python
    module docstring" for the smash measurement, and M40 deletes that docstring.
  - So the measurement moves into the Rust doc, with M31's numbers: smash reads 0.76–1.06 over the 13
    stored shots (M31 P2 finding 1).
  - "The two shots on disk" is restated as what it was: the 2026-08-10 pair.
  - Python's "three shots" (`shot_measure.py:102`) and "one shot per session" (`:61`) have no Rust
    mirror, so there is nothing to fix for them.
- **`crates/analysis/src/engine.rs:478–479`**, the doc on `BundleRequest.shot`:
  - It says "outcome checkpoints need per-club benchmark bands `ranges.json` does not have … full
    M4".
  - It is rewritten to ADR-034 §5's position. A shot metric is never banded. It is graded per club
    over many shots (M35, M37), and the per-swing `outcome_score` stays `None`.
- **`crates/contracts/src/intent.rs`** says "full M4" five times: in the `PracticeMode` doc, and on
  `ShotShaping`, `Performance`, `Drill` and `TargetShape`. Each is rewritten to ADR-034's position:
  - the single-swing policies stay unbuilt (§5);
  - `SHOT_SHAPING` with a `target_shape` is judged in M37's shape topics;
  - a `DRILL` shot is not tracked (§3, M35).

### Sequence

Run this on this box. It reads only committed vectors, so, unlike the old sequence, it needs neither
`data/` nor the `ocr` extra.

1. **Land `compare.rs`, `stages.rs` and `rerecord` with `ANALYSIS_VERSION` still at 16.** An empty
   declaration over all 42 vectors must find no difference and write nothing. That proves the gate
   and the stage port against the Python record before anything moves.
2. **Land the contract changes**: the capability model, the two reasons, and `ANALYSIS_VERSION` 17.
3. **Run `golf-core rerecord --declare` with M32's declaration.** Any undeclared difference stops the
   milestone here.
4. **Edit the three schemas, and make the Python tooling changes above.**
5. **Run the full verify suite, then `rerecord` once more.** It must write nothing.

### Tests and new pins

- **`crates/contracts/src/shot.rs`** (unit tests):
  - A Python-shaped shot, with none of the ten new keys, reads with every new field at its default.
    That is P1 finding 1 of the M31.5 plan, in the reverse direction. It has to hold until M40,
    because the frozen lab keeps writing such shots.
  - A shot carrying all ten new keys round-trips.
  - `parse_is_current` holds for a stamped shot, an unstamped one, and a direct feed.
- **`crates/contracts/tests/capability.rs`** (new):
  - every declared field is a `ShotData` key;
  - every `shown_only` field has a non-empty note;
  - printed = declared ∩ present per shot, and the union is taken across shots;
  - a layout that lacks a tile excludes that field rather than blanking it;
  - an unstamped shot and a direct feed each follow their rule;
  - `hd_golf` never prints `low_point` or `attack_angle`;
  - an unknown device is refused;
  - `device_of` returns `hd_golf` on all 15 corpus shots. The test reads their `input.shot`, and it
    lives here because `contracts` cannot see `analysis`. `shot_measurements` calls `device_of`
    rather than restating the rule, so the corpus `measurements` stage keeps gating the source string.
- **`crates/contracts/src/unscored.rs`:**
  - `every_reason_has_a_row` and the wire-name table gain the two new members. The wire-name test
    says it is pinned against `contracts/unscored.py`, and those two members have no Python twin, by
    design.
  - `refilming_helps_exactly_where_the_clip_is_the_problem` gains `Misread`, and its doc changes to
    say "the clip or the photo".
  - `the_inference_family_is_five_rows_of_this_table` is unchanged.
- **`crates/contracts/tests/round_trip.rs`:** an input key that a vector's `rerecords` declared may
  come out at its default where it went in absent, and nothing else may. Inputs keep Python's shape,
  and this is the one allowance that shape needs.
- **`crates/contracts/tests/schemas.rs`** (new), as described above.
- **`crates/core`:**
  - `compare.rs`'s own tests.
  - For `rerecord`:
    - an undeclared difference fails, and names its path;
    - a declared added key and a declared moved value land;
    - every undeclared value in the written file is the committed one, bit for bit;
    - a file read back gives the same `Value`;
    - a second run writes nothing.
  - A new `crates/core/tests/stages.rs`: `stages.rs` reproduces all 21 committed stage documents
    under §3's rules, and each one composes onto its engine vector's `expected`.
- **Existing pins that pass unchanged after the re-record:** the `analysis_version ==
  ANALYSIS_VERSION` assertions in `crates/core/tests/engine.rs`, in
  `crates/analysis/tests/{geometry,measurements,judging,alignment}.rs`, and in `round_trip.rs:249`.
- **Python:**
  - the `tests/test_conformance.py` changes above;
  - `tests/api/test_pipeline_imports.py` and `tests/test_docs_truth.py` stay green.

### Docs

- **`docs/CONFORMANCE.md`:**
  - §1: the three Rust-owned schemas, and who edits them;
  - §2: `provenance.oracle`, and the `rerecords` ledger;
  - §4: the `rerecord` verb, and what `check` means now.
- **`docs/ARCHITECTURE.md` §1**, "The commands, precisely": `golf-core rerecord`, and the new meaning
  of `check`'s line.
- **An addendum to ADR-032**, recording the first Rust re-record and the ledger's form. It answers
  the question the thirteenth addendum asked.

### Exit

- Every suite is green, and `conformance.py check` passes in the frozen view.
- On each of the 42 files, the re-record's report lists only the declared differences, and a second
  run writes nothing.
- Every undeclared value in those files is the committed Python value, and each file names M32's
  declaration in `provenance.rerecords`.
- Frozen Python is untouched outside `scripts/conformance.py` and `tests/test_conformance.py`, and
  `contracts/swing.py` still says 16.
- **The old exit's three items are now M34's**: the diff of the re-parsed shots, `fields_present`
  telling the two layouts apart, and the ADR-014 addendum's count of photos with a V tile.

## M33 — Apple Vision spike (Mac)

**Unchanged in substance by M31.5.** The spike feeds Vision's boxes to the frozen Python parser, which
is using the lab, not changing it
([ADR-035 clause 4](../decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)).
It depends on M31.5 alone, because M32 no longer touches that parser. Two things follow:
- **The harness is spike code, not a lab module.** `TextRecognizer` is a `Protocol`, so the harness
  satisfies it from outside `src/golf_coach/`, kept in `spikes/` with the spike's record. The package
  gains nothing.
- **The frozen parser carries the `Impact Position V` tie**, which M34 fixes in Rust only. The stored
  shots were parsed with it, so on `2026-08-10-1` and `2026-08-23-1` a field that differs may be the
  tie's doing rather than Vision's. Read the gate's diffs on those two shots against the tie first.

**The recognizer:** a Swift CLI, `tools/vision-ocr/`.
- It runs `VNRecognizeTextRequest` in `.accurate` mode, with `usesLanguageCorrection=false` and
  `customWords` = the profile labels.
- It flips Vision's bottom-left origin to top-left and emits boxes as JSON.

**The harness:** a `JsonBoxesRecognizer` implementing `TextRecognizer`, in the spike's own directory.
- Vision's boxes go through today's `parse_screen` / `validate_parse`.
- The resulting shots are diffed per field against the stored ones.

**Two modes:**
1. the OpenCV-rectified images;
2. raw photos, rectified with `VNDetectRectanglesRequest` + `CIPerspectiveCorrection`.

**Gate:**
- no field value differs unless it is flagged `needs_review`;
- `min_confidence` is calibrated for this recognizer. Vision's confidences are coarse, so the 30%
  OCR weight in `parse_confidence` and the 0.6 review threshold need re-checking. A change they need
  lands in `crates/screen` (M34), not in the frozen parser.

## M34 — The screen reader in Rust

**Re-detailed by M31.5 P6b.** All the screen-parser work is here now, in Rust only (the M31.5 plan's
[carried decisions 1–4](m31-5-rust-first-replan.md#decisions-carried-to-m32-and-m34-from-the-m32-interview-2026-09-30)),
and the frozen Python parser is not touched
([ADR-035 clause 4](../decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)).
It is a port with new behaviour on top, so its vectors are recorded from frozen Python for what
exists and hand-worked for what is new
([clause 3](../decisions/035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)).
It depends on M32 and runs on this box, where `data/` and the `ocr` extra are.

**New `crates/screen`,** depending on `contracts` only, with no `regex` crate:
- `profile.rs`: `include_str!` of the device profiles (which copy is settled below);
- `difflib.rs`: CPython's `SequenceMatcher.ratio`, reproduced exactly;
- `parser.rs`, `validate.rs`, and `orient.rs` (`label_ratio`).

**Crate split:** move `crates/analysis/src/pyfmt.rs` into its own `crates/pyfmt`, so that `screen`
does not depend on `analysis` (ADR-008 as a cargo edge). Check `docs/REFACTOR_LEDGER.md` first.

**Entry point:** `golf-core parse-screen`, boxes in, `ShotData` out.

### The order: record, port, then change

This is the M31.5 plan's Q4 and Q7, applied to the parser.

1. **Record today's parser from frozen Python, once.** Add a `screen` family to
   `scripts/conformance_vectors.py`, which is the one change the frozen lab allows (clause 4).
   - `spec/vectors/screen/corpus/`: PaddleOCR's boxes for the 13 stored bay photos, and frozen
     Python's parse of each, **including the `CENTER` spill on `2026-08-10-1`**. The recorder finds
     each photo through its bundle, by the manifest's photo sha256 as `api/pipeline.py::_shot_for`
     does, and never through the bulk importer (the M31.5 errata above).
   - The two reference photos in `data/raw/shot_screens/` (`IMG_2738`, `IMG_2739`) carry the other
     layout. Their boxes are recorded too, read directly, and written to no store.
   - `spec/vectors/screen/synthetic/`: hand-built boxes for the edges.
   - Format-family tables for every edge in "What the code says" finding 5, generated from CPython.
   - Once recorded, `regenerate` refuses the family, on `conformance_vectors._audio`'s precedent, as
     §M32 makes it refuse the engine and stage families.
2. **Port it faithfully.** `crates/screen` reproduces every recorded vector under
   `docs/CONFORMANCE.md` §3's rules before anything changes. The faithful port is a gate step and
   never ships.
3. **Change it in Rust, with hand-worked vectors** (`provenance.oracle: "hand"`):
   - **The tie rule** (carried decision 2). Boxes are assigned to labels one-to-one by best
     similarity. Where two boxes could swap fields at equal total score, neither field is read, a
     warning names the ambiguity, and both boxes still bound their neighbours' columns. An exact
     `Impact Position` beside `Impact Position V` resolves uniquely, and so does an OCR-damaged
     `Impact Position Y`; each gets a hand-worked vector.
   - **The old §M32 "Screen package" list**: the `Impact Position V` tile, `ProfileField.scale` and
     the scale applied when a cell is read, `fields_present`, `parser_version` stamping, and ranges in
     `validate.rs` for the new `ShotData` fields.
4. **Re-record the corpus family, diff-gated.** `golf-core rerecord` (§M32) is extended to the screen
   family. This is the 13-shot re-read, and its declaration is the exit below.

### `SCREEN_PARSER_VERSION`

- **Ledger entry 1 is the Rust parser as M34 ships it**, tie rule and V tile included. §M32 left the
  entry's meaning here.
- **The faithful port gets no version of its own**, because it never stamps a shot. A stored shot is
  either frozen Python's (0, unstamped) or 1. M34's plan splits them only if the faithful port ever
  ships alone.
- The screen family ages on this constant as the engine family ages on `ANALYSIS_VERSION`. So the
  re-record moves each corpus vector's `parser_version` from 0 to 1, as a declared value.

### The 13-shot re-read writes no `data/`

It runs over the boxes the recorder committed. The lab's Rust OCR is M29's (`ort`), so a Rust parser
in M34 has nothing else to read.

- **It does not rewrite `data/processed/shots/`.** ADR-035's
  [Consequences](../decisions/035-rust-everywhere-python-where-required.md#consequences) and ADR-014's
  M31.5 addendum both say the lab goes on reading the two label-fix shots the old way until M29
  switches it to Rust. A Rust write here would bring that forward, and would leave each
  `analysis.json` echoing the old shot.
- **So the stored shots stay unstamped until M29's lab re-reads them** (§M29). The stale-cache check
  §M32 routed here from `api/pipeline.py::_shot_for` is a check inside the lab's shot lookup, and that
  lookup is ported with the lab. M34 lands the stamping it reads, and `parse_is_current` (§M32) is
  the check.
- **From M34 until M29 the phone and the lab read `2026-08-10-1` and `2026-08-23-1` differently.**
  The difference is declared, not a regression.

### `profiles.json`: fork it or share it

ADR-014's M31.5 addendum leaves this to M34, and ADR-035 decides neither.

- The frozen parser reads `launch_monitor/screen/profiles.json` as package data.
- **Sharing it** means the V tile reaches the frozen parser. Every bay shot gains a warning, and its
  `parse_confidence` drops by about 0.02 (the M31.5 errata). That is new behaviour in the frozen lab.
- **Forking it** means a Rust copy under `crates/screen` that gains the tile, beside a frozen copy
  that does not. It gives up ADR-032 §5's one copy on disk for this file until M40 deletes the Python
  one.
- Clause 4 points at the fork. M34 records the choice where ADR-014's addendum left the question.
  ADR-034's Consequences also hand M34 the `_PACKAGE_DATA` comment in `tests/test_conformance.py`
  that calls the file "OCR stays Python", and the choice decides what that comment should say.

### The capability pin and the golfer-facing filter

- **`hd_golf` equals its profile.** §M32's `devices.json` declares `hd_golf`'s fields. M34 pins them
  equal to the targets of the profile `crates/screen` reads, which include `impact_position_v` from
  here on. The pin lives in `crates/screen/tests/`, because `contracts` cannot see `screen`.
- **`no tile found for '<label>'` never reaches the golfer** (carried decision 3). It stays in
  `provenance.warnings`.
  - M34 adds one filter beside the parser that writes the warning, so that the producer and the
    filter share one spelling.
  - Every Rust surface that shows warnings calls it: M38's review screen, and the `rmcp` shot view
    (M29). `coach.py`'s JSON entry (M29) reads its shot through that view.
  - Frozen Python keeps showing the warning until M40, in `mcp/query.py`'s shot view and in the brief
    that `api/pipeline.py` builds through `coach.py`'s pydantic entry. Both are frozen (clause 4).

### Exit

- `cargo test` passes on every screen vector: the Python-recorded family, as the faithful port's
  gate, and the hand-worked ones.
- The re-record's report on the 13 corpus vectors is carried decision 4's exit:
  - every shot number is identical except on the two label-fix shots. On `2026-08-10-1`, `CENTER`
    stops spilling into `Shot Type` and `impact_position` stays `None`. On `2026-08-23-1`, `HEEL`,
    right today by luck, becomes `None`;
  - parse bookkeeping (`parse_confidence`, `warnings`, `raw_fields`) may change, and each change is
    listed and explained. The V tile's warning and the confidence move are the expected ones;
  - no `needs_review` flips;
  - a second run writes nothing.
- `fields_present` tells the two layouts apart, and the report counts the photos with a V tile
  (ADR-014's M31 addendum). These were §M32's old exit items.
- The `hd_golf` pin and the filter's tests pass, and nothing under `data/` has changed.

## M35 — Shot-first sessions in Rust

**Rust only, and after M36**
([ADR-035 clause 6](../decisions/035-rust-everywhere-python-where-required.md#6-order-the-phone-path-first)).
M31's plan had M35 write Python, as M36's oracle. Now M36 ports the many-shot layer first, faithfully,
and M35 changes it in Rust with hand-worked vectors. It depends on M36, whose `crates/storage` holds
the corpus it changes (the M31.5 plan's P1 finding 7). Frozen Python gains none of it.

**Engine:**
- `crates/contracts/src/shot_result.rs::ShotResult`, written as `shot_analysis.json`. It moved here
  from M36's list, because it is new (the M31.5 plan's P2 finding 5).
- `crates/analysis/src/engine.rs::analyze_shot`, built from `shot_measurements` and
  `flight_measurements`. The Rust engine already keeps those apart from the swing's measurements, so
  the old §M35's extraction was Python's alone. The result is filtered by §M32's `printed_on`.
- `golf-core run-shot`.
- **The two photo-side reasons are first written here**, into `ShotResult.unscored`: `printed_blank`
  and `misread` (§M32). They go to `shot_analysis.json` and never to `analysis.json`, which frozen
  Python reads with a closed enum
  ([clause 4](../decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)).

**Intent, and which shots count (Decision 7):**
- Record a `PracticeGoal` per shot: a session default with a per-shot override.
  `crates/contracts/src/intent.rs` already mirrors ADR-009's shape. `SwingResult.intent` carries one
  per swing, and `ShotResult` carries it for a photo-only shot.
- Add one rule to `intent.rs` for whether a mode counts toward stats: `DRILL` does not, and every
  other mode does. It is a function of the mode, not a stored flag.
- **A mode other than `FUNDAMENTALS` cannot reach the engine today.**
  `crates/analysis/src/scoring.rs::policy_for` panics for every other mode, as Python's raises. M35
  decides whether a `DRILL` or `SHOT_SHAPING` intent gets a policy or travels beside the swing, and
  ADR-009's addendum records it (ADR-034's Consequences).
- `read_corpus` and the aggregates built on it (`build_baseline`, `build_dispersion`,
  `build_bag_profile`, all M36's ports) leave untracked shots out, and a pin says so. An untracked
  shot is still stored, listed in history and given its own `ShotResult`.
- **The precedent for "excluded and named"** is `overall_score`. The engine routes an unscorable
  checkpoint to `SwingResult.unscored` instead of the list scoring averages, in both
  implementations, so it is excluded from the score and named, never counted as zero. The pivot
  needs that rule twice: here, where an untracked shot leaves the aggregates but stays stored and
  listed, and in M37, where an ungraded topic leaves a blend but is named (Decision 11).

**Storage, in M36's `crates/storage`:**
- `read_corpus`:
  - admits photo-only entries;
  - dedupes them by photo hash;
  - no longer lets `NO_FACE_ON` exclude them.
- `career.rs`: `face_on_sha256` becomes optional.
- A tolerant reader for `shot_analysis.json`, beside the ones M36 moved in from `api/state.py`.
- **A route for handedness to reach a photo-only shot.** Today it arrives only through the swing
  manifest that holds the photo, then the player, then the golfer store, so a photo with no swing gets
  `None`, and every photo-only shot would withhold its on-line share (ADR-034's Consequences).

**What the old §M35 gave Python goes elsewhere**
([clause 7](../decisions/035-rust-everywhere-python-where-required.md#7-what-this-supersedes-sentence-by-sentence)):
- `api/pipeline.py`'s photo-only branch and `api/state.py::load_shot_analysis`: the phone writes
  `shot_analysis.json` on the device (M38), and the Rust lab CLI writes and reads it in the lab (M29).
- `scripts/import_shot_screens.py --player --club --into-session`, and `scripts/reanalyze.py` over
  photo-only directories: verbs of M29's lab CLI.

**Frozen Python disagrees with this on purpose, until M40.**
- Frozen `read_corpus` goes on excluding a photo-only manifest as `NO_FACE_ON`
  (`storage/corpus.py:97`). The frozen server's career view and Python `mcp/`'s club and career tools
  read through it.
- So once photo-only entries reach `data/` (M29's import verb, or the frozen upload path while it
  stays open), frozen Python pools fewer shots per club than Rust does. ADR-024's M31.5 addendum
  records it.
- Frozen Python never learns the `DRILL` rule either, so it pools a Rust-written `DRILL` swing that
  has a face-on clip.

**Vectors:**
- `spec/vectors/shot/`: hand-worked (`provenance.oracle: "hand"`), since there is no Python to record.
- M35's change to `read_corpus` re-records M36's `storage/` and `career/` families wherever it moves
  them, diff-gated, declaring each admitted photo-only entry. The new cases (the dedupe, mixed
  sessions, a `DRILL` shot) are hand-worked.
- **The swing output is unchanged**: `golf-core rerecord` with an empty declaration over the engine
  and stage families writes nothing.

**Tests:**
- `crates/storage/tests/corpus.rs`: photo-only, the dedupe, mixed sessions, and a `DRILL` shot that is
  stored but never reaches an aggregate.
- `crates/contracts`' intent tests: only `DRILL` fails to count, and a per-shot override beats the
  session default.
- The unchanged-swing pin above.

## M36 — The many-shot layer in Rust

**Runs before M35, and depends on M32**
([ADR-035 clause 6](../decisions/035-rust-everywhere-python-where-required.md#6-order-the-phone-path-first)).
It ports the many-shot layer and its stores **faithfully**, `read_corpus` included, exactly as it
behaves today, `NO_FACE_ON` exclusion and all. M35 then changes it.

**Recorded once from frozen Python**
([clause 3](../decisions/035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust),
the M31.5 plan's Q7). M36 adds `career/` and `storage/` families to `scripts/conformance_vectors.py`,
which is the one change the frozen lab allows
([clause 4](../decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)),
records them, and only then ports. That recording is the port's one independent reference. After it,
`regenerate` refuses both families, on `_audio`'s precedent, and Rust re-records them, diff-gated.

**Ports:**
- **To `crates/contracts`**: career, baseline, dispersion (with `METRIC_TARGETS` and
  `PATTERN_READING`), club_profile, mishit, bag, club, club_spec and comparison. `shot_result` has
  left this list for M35's (P2 finding 5).
- **To `crates/analysis`**: `baseline`, `dispersion`, `club_profile` and `comparison`. `stats.rs`
  gains the CI helpers.
- **New `crates/storage`**: the manifest, the bundle store, `corpus` and the shot store (the M31.5
  plan's R13 and R16).
  - It takes the tolerant readers `read_corpus` calls from `api/state.py` (R9) inside the crate. So
    ADR-008's one Python exception, `storage/corpus.py` importing *upward* into `api.state`, does not
    carry over to Rust.
  - The bundle store gains an explicit target beside the faithful "newest swing lacking this role"
    ("What the code says" finding 9). That is new behaviour, with hand-worked vectors.
- **The catalogue** (`clubs/catalogue.py` and `club_catalogue.json`, R25) ports with the bag.
  - The JSON becomes a two-language file: `clubs/lookup.py` writes it, and it stays Python because it
    is an LLM call. Rust reads it.
  - That is clause 4's rule reversed, so M36 pins that Rust reads every committed entry and accepts
    every value `contracts/club_spec.py` can write (P2 finding 4).
- **R29's five CLIs** (`career_baseline`, `career_corpus`, `career_dispersion`, `club_profile` and
  `flag_mishit`) get Rust verbs here. M29 deletes the Python scripts with the other lab scripts. They
  read through `GolferStore` and `BagStore`, so those two stores port with them.

**`read_corpus`'s `OUTDATED` rule meets the version split.** M36's plan decides what to do about it.
- `read_corpus` excludes an analysis older than the installed `ANALYSIS_VERSION`
  (`storage/corpus.py:252`), because its numbers "are not comparable with a swing analyzed today".
- Rust's version is 17 from M32, and every `analysis.json` in `data/` is frozen Python's 16. So a
  faithful port run over `data/` pools nothing until M29's Rust lab re-analyses it, although M32's
  bump moved no number.
- The vectors are unaffected if each fixture's version is set relative to the installed one. What
  the verbs do over `data/` before M29 is the open part, for instance whether a shape-only bump makes
  an artifact outdated at all.

**Also:**
- **Schemas.** Each schema root whose shape ports here becomes Rust-owned, by §M32's split. M36
  revisits `schemars` if hand-maintenance has proved the larger cost.
- **Writes.** Anything M36's verbs write under `data/` is bound by clause 4: they may add keys, and
  may never write a value a frozen enum or bound refuses.

**Vectors:**
- `spec/vectors/career/`: synthetic corpora crossing every n-gate (4/5/9/10/11/12) and the mishit
  floor.
- `spec/vectors/storage/`.
- Both are Python-recorded (`provenance.oracle: "python"`), except the explicit-target cases, which
  are hand-worked.

## M37 — Strike profile, topic grades and strengths/weaknesses (Rust first)

**Contract:** `crates/contracts/src/strike.rs`, holding `ImpactCell`, `StrikeProfile` and
`RateEstimate`. `RateEstimate` is a Wilson interval using stdlib `sqrt` only. Beside
`StrikeProfile`, the contract gains a **topic grade** (the topic, its `RateEstimate` or the reason it
is withheld, and for a shape topic both shares) and a **blend** (its mean, the topics it averaged,
and the topics it excluded, named).

**`crates/analysis/src/strike.rs`, per club:**
- the impact grid and the centered-strike rate;
- heel/toe asymmetry;
- ball-speed SD;
- carry, start line and face-to-path, each through the existing `dispersion`;
- low point, only when printed;
- everything gated by `minimum_n`.

**Scope, per stat (Decision 8):** each stat declares whether it is **club-only** (carry, ball speed,
launch, spin) or **club-independent** (strike location, face-to-path, start line, consistency
rates). Only a club-independent stat gets a raw player-level average.

**Topic grades (Decisions 9 and 10):**
- **Six topics:** strike location, consistency, low point, fade, draw and straight.
- A topic's grade is the **share of good shots**, the % of the club's tracked shots meeting the
  topic's criterion, as a `RateEstimate`. It is **withheld below the minimum n**. No tour bands.
- The criteria for strike location and consistency are set here, against hand-worked vectors.
- Low point exists only where printed. A shape topic exists only where that shape was declared.

**Shot shapes (Decision 17, whole):**
- A shot declared `SHOT_SHAPING` with a `target_shape`, by the golfer or by challenge mode, is
  classified from its **face-to-path** against `METRIC_TARGETS["face_to_path_deg"]`'s tolerance:
  inside it is straight, outside it is a fade or a draw by the sign, and the sign flips for a
  left-hander. The printed `Shot Type` word is not used. Missing face-to-path leaves the shot
  unscored for shape; missing handedness is `no_handedness`.
- **Two shares per shape:** "hit the shape", and "hit the shape and finished on line". The second is
  the topic's grade; the first shows beside it.
- **On line** reads the printed landing offline where the device prints one. Otherwise it is
  **projected from the printed start direction and carry, including the curve**, through ADR-027's
  flight model (`crates/analysis`'s `flight`, `flight_infer` and `spin_solve`), and **named as
  projected** everywhere it appears. Where the projection cannot be computed, the on-line share is
  withheld and named. The tolerance is `METRIC_TARGETS`' offline row unless this milestone finds a
  reason otherwise.
- **Hand-worked vectors** cross the straight tolerance's edge from both sides, and include a
  left-handed shot.

**Blends (Decision 11):**
- **A club blend and a player blend**, each the equal-weight mean of the graded topics, with the
  three shapes as three topics.
- At player level, a topic's share pools every tracked shot across clubs, each shot judged against
  its own club's criterion.
- **Ungraded is named, impossible is absent:** a printed topic too thin to grade is excluded from
  the blend and named; a topic that is not printed, or a shape never declared, does not appear.

**Mechanics is a separate panel (Decision 12):** the pose checkpoints never enter a topic grade or a
blend.

**`crates/feedback/src/profile_rules.rs::rank_profile`** is unchanged in rule; the grades sit beside
its list, they do not feed it:
- Weaknesses are established bias or scatter, ranked by how far they exceed tolerance.
- Strengths are equivalence claims only.

**Vectors:**
- hand-worked synthetic vectors, plus Rust-recorded corpus vectors. The corpus shots can come from
  the committed corpus vectors, which already carry each one as `input.shot`. The builder that reads
  `data/` is M29's
  ([ADR-035 clause 3](../decisions/035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust));
- a new pin: every vector family names its oracle. It can also require a declaration on every
  `rerecords` entry (§M32).

**Reaching it from the lab:**
- `golf-core profile`;
- `get_strike_profile` is a tool of the `rmcp` server, not of Python `mcp/`, which is frozen
  ([clause 4](../decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)).
  M37 adds it if M29 has landed, and otherwise M29 carries it. The docs-truth pin on the MCP tool
  count is the one §M29 re-points.

## M38 — The iPhone app (Mac; P5 at the bay)

- **P0 — toolchain.** Install the `aarch64-apple-ios` and `-sim` targets, Flutter and FRB. Get
  `cargo test` green on macOS; it has never run on anything but Windows.
- **P1 — facade crate `crates/app`.** It exposes `parse_screen`, the session cursor,
  `record_shot`, `correct_shot`, `bag_profile`, `strike_profiles`, the topic grades and blends,
  `profile_feedback` and `export_sessions`. It does not depend on `capture` or `pose`, nor on M29's
  `ort` reader, because the phone reads the screen with Vision.
- **P2 — the Flutter `app/` skeleton.** The flow:
  1. pick golfer and bag, then club, then the session's intent (mode and target shape, Decision 7);
  2. VisionKit document camera;
  3. Vision OCR (a Swift plugin);
  4. Rust parse;
  5. review/edit when the shot `needs_review`, including a per-shot override of the intent. Its
     warnings go through M34's golfer-facing filter;
  6. history.
  - **Challenge mode:** the app calls fade, draw or straight before each shot and records the call
    as that shot's `target_shape` (Decision 17 §1). Challenge shots count fully.
- **P3 — profile screens:**
  - the per-club profile, strengths and weaknesses;
  - per topic, the grade beside the list, with both shares for each shape topic;
  - the club blend and the player blend, each naming the topics it excluded;
  - a projected on-line share is labelled as projected.
- **P4 — export:** the phone's `export_sessions` (P1), imported into `data/` by a verb of M29's Rust
  lab CLI (the M31.5 plan's Q8), so P4 waits on M29. The frozen server reads what that verb writes, so
  [ADR-035 clause 4](../decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)
  binds it.
- **P5 — bay session:** ≥30 photos across ≥2 clubs. Record the success rate, the review rate and
  latency.

**Exit:**
- At the bay, with no laptop, photos become stored, validated shots and the per-club profile
  renders.
- The lab's Rust reader (`crates/storage`'s `read_corpus`) agrees with the phone on the exported
  data. Frozen Python's `read_corpus` is not the reference (Q8), and it excludes photo-only entries
  by design (§M35).

### Testing on the iPhone without the App Store

- **Day to day:**
  - Plug the phone into the Mac and run `flutter run`. Wi-Fi works after the first pairing.
  - Turn on Developer Mode on the phone (Settings → Privacy & Security).
  - Sign with a free Apple ID "Personal Team" in Xcode.
  - The install expires after 7 days (re-run to re-sign), and a free account allows at most 3
    sideloaded apps.
  - Camera, Vision and VisionKit need no paid entitlement, only `NSCameraUsageDescription`.
- **The iOS Simulator** covers the photo → OCR → parse loop, using the stored bay photos from the
  photo library. It has no camera, so the document scanner is device-only.
- **Most of the product is tested without the Mac.** The Rust parser, strike analysis and feedback
  run under `cargo test` and the vectors on the Windows box. The Mac is only needed for the
  Flutter shell and Apple Vision.
- **The paid Apple Developer Program ($99/yr) is optional, later.** It gives 1-year signing, and
  TestFlight internal testing needs no App Review.

## M39 — Optional video on the phone (Mac, then bay)

- **P0 — pose conformance gate:**
  - iOS `PoseLandmarker` with the same `.task` file, pinned by sha256. Pin the laptop's `latest`
    URL first (`pose/estimator.py`, which is inside the MediaPipe exception).
  - VIDEO mode, CPU delegate.
  - Re-pose the stored face-on clips, then run `golf-core run`.
  - **Gate:** every `passed` verdict is identical, and every delta is within its tolerance.
  - **If it fails:** the phone records, and mechanics are computed on the laptop (M40).
- **P1 — capture:**
  - record face-on;
  - detect the strike with `crates/trigger` on-device;
  - attach the clip to the current shot explicitly.
- **P2 — overlay:**
  - the impact frame, carrying that shot's launch-monitor facts;
  - the red/green low-point marker only when the device prints low point, so never for HD Golf.

## M40 — The laptop client resumes

- M21, M24 and M25 are re-scoped: the same Flutter app as a desktop target.
- M26 moves here whole (Decision 14): CI, packaging, signing and distribution. Until then, M38's
  free Personal Team signing is the iOS path.
- An R10 BLE adapter (`btleplug`), plus its `devices.json` entry.
- Launch-monitor API adapters, in Rust.
- The laptop's OCR recognizer is M29's `ort` reader, behind the same boxes seam.
- `crates/pose`'s live-session measurements (ADR-033's fifth addendum) stay here. M29 is the crate's
  first caller, and it poses offline.
- **`api/`: port it to `axum`, or drop it**
  ([ADR-035's Deferred list](../decisions/035-rust-everywhere-python-where-required.md#deferred-by-choice)).
  - A port keeps or declines ADR-016's rules: the loopback bind, the refused non-loopback start, and
    the token as a route dependency (ADR-016's M31.5 addendum).
  - The same decision settles the caller of `clubs/lookup.py` (the M31.5 plan's Q13), and
    `tempo_trainer.py` with the web UI (Q12).
  - If M29 left the frozen server's upload path open, it closes here.
- **M40 deletes the frozen Python**, which is
  [clause 5](../decisions/035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped)'s
  second moment. That is everything in the frozen FastAPI server's import closure, and clause 5 lists
  it. None of it goes before `api/` is decided, because the frozen server imports all of it (Q17).
  - Every file a crate reads by `include_str!` from `src/golf_coach/` moves crates-side in the same
    change: the benchmark JSON, and `profiles.json` unless M34 forked it.
  - `config.py` is cut to what the two workers read, and `contracts/` to `keypoints.py` and the LLM's
    own shapes
    ([clause 1](../decisions/035-rust-everywhere-python-where-required.md#1-the-rule-and-the-two-exceptions-it-names)).
  - The `api`, `ocr` and `audio` extras go with the code they served, and each test directory goes
    with its area (the M31.5 plan's R40).

## M29 — The lab port: a Rust lab CLI, the `rmcp` server, and the archive move

**Re-scoped by ADR-035, number kept**
([clause 5](../decisions/035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped),
the M31.5 plan's Q2). ROADMAP's §M29 was "the last Python": reduce Python to the sidecar, delete the
rest, and wait on M40 to do it. The ask stands, and the job is now to **port** the lab. M29 depends on
M34 and M36, runs on this box (where `data/` lives) beside M37 and M38, and is no longer blocked on
M40 (Q3). M38 P4 waits on it, and M40 depends on it.

It gets its own `docs/plans/m29-*.md` when it starts. This section is its scope, and the questions it
inherits, each with where it was found.

### The Rust lab CLI

One CLI runs the lab end to end. It replaces `api/pipeline.py` as the lab's pipeline, and the lab's
entry points (R28 in the M31.5 plan's inventory):
- **strike detection** through `crates/trigger` directly, where `audio/trigger.py` pipes PCM to
  `golf-trigger`;
- **strike audio** through ffmpeg as a subprocess, replacing `audio/{ffmpeg,source}.py`;
- **OCR** through `ort` (below), and **the parse** through `crates/screen` (M34);
- **the engine** through `crates/core`;
- **the storage M36 did not port**: the rest of `storage/`, except `transcript_store.py`, which stays
  with the LLM (R13). `contracts/audio.py` ports with the audio store (R7);
- **pose** through `crates/pose`, which spawns `pose/worker.py`. The lab CLI is the crate's first
  caller (the M31.5 plan's P5b finding 3, ADR-033's M31.5 addendum). Whether M29 measures
  warm-interpreter churn is its own plan's call.

**Its verbs** cover R28: a bundle, a keypoints file, re-analysis, photo import and the flight.
- They take on what the old §M35 gave Python: photo import with `--player --club --into-session`, and
  re-analysis over photo-only directories.
- The flight verb runs over `crates/analysis`'s flight model. The render-only helpers in
  `flight_measure.py` are not ported (Q10).

**The 13 stored shots are re-read into `data/` here.** M34's re-read ran over committed boxes and
wrote no `data/`.
- The lab's shot lookup, the Rust twin of `api/pipeline.py::_shot_for`, calls `parse_is_current`
  (§M32). It finds all 13 unstamped, and re-reads each through the Rust reader and parser, finding the
  photo through its bundle.
- So this is where `fields_present` reaches the stored shots. It is also where the lab's reading of
  the two label-fix shots changes, which is when ADR-035's
  [Consequences](../decisions/035-rust-everywhere-python-where-required.md#consequences) date it.

**The frozen server reads what the Rust lab writes, until M40**
([clause 4](../decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)). The
lab may add keys, and may never write a value a frozen enum or bound refuses. So `analysis.json`
goes on carrying only the reasons Python's `UnscoredReason` knows.

**The lab's OCR, through `ort`:**
- It runs the same Paddle models, with the preprocessing that goes with them. What replaces OpenCV's
  rectification in Rust is M29's plan (ADR-014's M31.5 addendum).
- **The gate.** On each of the 13 stored bay photos, the shot parsed from the Rust reader's boxes is
  compared with the shot parsed from PaddleOCR's boxes. M29's plan sets what may differ (the M31.5
  plan's P3 finding 5).
- PaddleOCR stops being the lab's reader when the gate passes. It is deleted in M40, because the
  frozen server's upload path imports it lazily (P5a finding 7).
- **`ort` is a second numeric library outside the scoring path**, beside `crates/trigger`'s
  `rustfft`. It stays out of `crates/{contracts,analysis,feedback,screen}`, and out of the phone's
  facade, `crates/app`.

### The `rmcp` server, and the LLM's route to it

- **The `rmcp` server replaces Python `mcp/` for MCP clients** (Q5), and its binary replaces
  `scripts/run_mcp_server.py`.
  - The surface to serve is Python's whole tool list today. That is wider than ADR-006's first table,
    because `mcp/club.py` and `mcp/flight.py` added tools since (P5a finding 3).
  - It takes with it the flight caveats (`analysis/flight_caveats.py`), `contracts/caveats.py` and
    `contracts/tool_descriptions.py` (clause 5).
- **The new `ShotData` fields join a metric list here** (§M32). Python's
  `mcp/query.py::_METRIC_FIELDS` never sees them.
- **The shot view goes through M34's golfer-facing filter.**
- **The reason prose.**
  - `contracts/caveats.py` builds the "not capture problems … the footage is not what went wrong"
    sentence from Python's reason table.
  - The Rust copy derives it from `UNSCORED_REASONS`, so `printed_blank` joins that list by itself.
  - `misread` is a *photo* worth retaking, which "re-film" does not say. M29 rewords the sentence,
    unless the first surface to emit `misread` got there first (P6a finding 5).
- **The docs-truth pin on the tool count** (`test_the_docs_state_the_real_mcp_tool_count`) reads
  Python's `TOOL_DESCRIPTIONS`. It is re-pointed at the Rust list in the change that moves the
  descriptions.
- **`conversation.py` drives the server over stdio** (Q9). That is ADR-020's Option B, through the
  installed SDK's `anthropic.lib.tools.mcp`.
  - `mcp_tool` is synchronous and calls the async `ClientSession` through `anyio.from_thread.run`, so
    the session must live on an event loop that the runner's thread can call into (P5a finding 4).
  - Each tool's description and input schema then come from the server's `list_tools`, so ADR-020's
    "two tool definitions over one implementation" ends with `mcp/runner_tools.py`.
  - `scripts/ask_swing.py` switches to this route.
  - `api/app.py` keeps handing in-process tools to the frozen server's conversation until M40. M29
    adds the new route beside the old one, and does not rewire the frozen server (P2 finding 2).
- **`coach.py` gains a JSON entry** (Q14). It reads its swing and shot from the `rmcp` server, as
  `conversation.py` does, while `api/pipeline.py` keeps calling the pydantic entry until M40.
- **That is the only Python M29 writes**, and it sits inside clause 1's LLM exception.
- **Still open, and M29's to decide**
  ([ADR-035's Deferred list](../decisions/035-rust-everywhere-python-where-required.md#deferred-by-choice)):
  - how the caveat prose reaches Python: through the `rmcp` server's descriptions and results, or
    through a kept copy, which would be a fourth surviving shape for M29 to name (P2 finding 4);
  - whether a standalone `sidecar/` package holds the surviving Python. It would now hold the LLM side
    as well as the pose worker (ADR-033; P5b finding 7).

### The vector builder, the phone export, and the upload path

- **The vector builder that reads `data/` becomes Rust**
  ([clause 3](../decisions/035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)).
  It is a port of `conformance_vectors.build_all` over M36's storage readers. From here on, a new
  vector built from `data/` is Rust's.
- **The phone-export import is a verb of the lab CLI** (Q8). It writes the stored layout the frozen
  server reads, so clause 4 binds it, and M38 P4's exit compares against the Rust reader.
- **Whether the frozen server's upload path stays open until M40** is M29's to decide.
  `api/worker.py` → `api/pipeline.py` still writes `data/` with Python, and a Python write drops the
  Rust-only keys without a word.

### The archive move, and what M29 deletes

- **The archive move** (clause 5, interview decision 3). `scripts/golfdb/`, `scripts/caddieset/`,
  `scripts/hand_landmark_reliability.py`, `scripts/{trigger_replay,pose_replay}.py` and
  `contracts/reference.py` move to `archive/`, and the `research` extra goes with them.
  - The JSON they produced stays in `src/golf_coach/analysis/benchmarks/`, where `crates/analysis`
    reads it, until M40.
  - Archived `golfdb/` runs until M40 deletes `analysis/`, and is a record after that (P1 finding 11).
  - It can re-derive nothing Rust has changed, because its metric columns come from Python's
    `analysis.measure`. The first change that needs a re-derive needs a decision too (ADR-022's M31.5
    addendum; P5a finding 6).
  - The doc paths that name the moved scripts are rewritten in the same change. P1 finding 14 counted
    them; re-count first.
  - `spikes/` stays where it is.
- **M29 deletes only what the frozen server does not import** (clause 5's first moment, Q17): the lab
  scripts (R28–R30, R32 and R34), `detection/` and `frontend/`, each with its tests. The lab scripts
  go rather than stay frozen, because a Python re-import of a Rust-written shot drops its Rust-only
  keys (P2 finding 2).
- **Extras.** `research` goes with the archive. `vision` loses `ultralytics` with `detection/`, its
  only user. `hardware` goes too: nothing imports `bleak`, and M40's R10 adapter is `btleplug` (P1
  finding 13). The `api`, `ocr` and `audio` extras stay until M40.
- **What stays** is clause 1's list, plus everything the frozen FastAPI server imports, until M40.

### Exit

M29's own plan refines these.
- The OCR gate passes on the 13 stored bay photos.
- The Rust lab CLI re-runs every stored bundle in `data/`, and the frozen server still starts and
  reads every artifact it wrote.
- The 13 stored shots are stamped, and on disk only the two label-fix shots change a field value, as
  M34's declared diff said they would.
- `scripts/ask_swing.py` answers through the `rmcp` server over stdio, and the tool-count pin reads
  the Rust list.
- The archive move and the deletions are done, with the doc paths rewritten in the same change.

---

## Invariants every milestone keeps

- **ADR-008, as cargo edges:** `crates/analysis` depends only on `crates/contracts`, and capability
  data lives in `crates/contracts` (§M32). `crates/screen` must not depend on `crates/analysis`, which
  is why `pyfmt` splits out, and `crates/feedback` does not depend on it either. Python's half of the
  rule freezes with the lab
  ([ADR-035 clause 7](../decisions/035-rust-everywhere-python-where-required.md#7-what-this-supersedes-sentence-by-sentence)).
- **No numeric library in the scoring path, now stated for Rust alone:** no `regex` crate, and Wilson
  intervals written by hand. Python's half froze with the lab. The two numeric libraries outside the
  path are named rather than assumed: `crates/trigger`'s `rustfft`, and M29's `ort`.
- **"No score beats a wrong one," extended:** not printed is not unscored. Pin that a field the
  device does not print appears nowhere. A shape never declared produces no topic, and a topic too
  thin to grade is withheld and named (Decisions 10, 11 and 17).
- **A `DRILL` shot never reaches an aggregate** (Decision 7). Pin it wherever Rust aggregates shots.
  Frozen Python never learns the rule, and §M35 names what that costs.
- **No new Python beyond ADR-035 clause 1's two exceptions**, the pose worker and the LLM. The one
  change the frozen lab takes is a recorder family in `scripts/conformance_vectors.py`
  ([clause 4](../decisions/035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)).
- **The frozen lab keeps reading what Rust writes, until M40** (clause 4). Rust may add keys to
  anything frozen Python reads, and never writes a value a frozen enum or bound refuses.
- **An `ANALYSIS_VERSION` bump re-records `spec/vectors/` in the same change**, through
  `golf-core rerecord`, diff-gated (§M32).
  - The re-record reads only committed vectors, so it runs wherever `cargo` does.
  - "Only on the Windows box, where `data/` lives" is now about building *new* vectors from `data/`:
    frozen Python's builder does that until M29, and the Rust one after it.
  - Frozen Python's version stays where it is
    ([clause 3](../decisions/035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)).
- **These must stay green:** `tests/api/test_pipeline_imports.py`, until M40 retires it with
  `api/pipeline.py`; `tests/test_docs_truth.py`; and, from M32, `cargo test`, which is what certifies
  the vectors.

## Verify, every milestone

```bash
.venv/Scripts/python.exe -m pytest
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
cargo test
cargo clippy --all-targets && cargo fmt --check
```

**`scripts/conformance.py check` left this list at M32** (§M32, "What changes in Python, and why").
M32's exit runs it once, in the frozen view. After that, its pytest pins carry the freeze, and
`cargo test` certifies the vectors. The Python lines stay until M40 deletes what they check.

Run `cargo test` unpiped, or check `PIPESTATUS`: `cargo test | tail` reports `tail`'s exit status
(WORKLOG, M23 P9). Don't run it at the same time as pytest either. The timing test
`tests/api/test_worker.py::test_failure_is_recorded_and_the_consumer_survives` failed once under
that load on 2026-09-29, and passed alone.
