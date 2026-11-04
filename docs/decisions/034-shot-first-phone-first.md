# ADR-034: Shot-first — the launch-monitor screen is the product, the phone is the host

## Status
**Accepted** 2026-09-29 — M31 of the program plan
[m31-m40-shot-first-pivot.md](../plans/m31-m40-shot-first-pivot.md#m31--adr-034-shot-first-the-launch-monitor-screen-is-the-product-the-phone-is-the-host),
whose phase list and interview record are [m31-shot-first-adr.md](../plans/m31-shot-first-adr.md).
**Partially supersedes [ADR-030](030-app-platform-rust-core-python-sidecar.md)**: its §5 ("the phone
is a camera, and its app comes after the laptop's") and the premise its Context states, *the machine
is a laptop*. The rest of ADR-030 stands, and §7 (cloud closed) is reinforced.

**Superseded in part by [ADR-035](035-rust-everywhere-python-where-required.md)** 2026-09-30: §7's
lab reader, §9 in part, and the Consequence that blocked M29 on M40. The addendum at the
foot names each sentence, and the rest of this ADR stands.

It amends ten ADRs, each by an addendum in that ADR rather than by editing it here:
[002](002-pose-estimation-mediapipe.md), [009](009-swing-scoring-model.md),
[010](010-benchmark-ranges.md), [014](014-screen-capture-shot-ingestion.md),
[016](016-local-first-host-and-phone-upload-topology.md), [024](024-per-club-shot-history.md),
[030](030-app-platform-rust-core-python-sidecar.md), [031](031-the-capture-edge.md),
[032](032-the-rust-core.md) and [033](033-the-pose-sidecar-protocol.md). M31 P4–P7 write them, and
each one routes to a numbered clause below instead of restating it.

**Nothing here is built.** The repo still runs the pipeline [ARCHITECTURE §1](../ARCHITECTURE.md)
describes: face-on video → pose → checkpoints, with the shot attached and never scored. M32–M40
build this decision, and the program plan is where each milestone's detail lives. Every
measurement of the stored shots below was taken on 2026-09-29 by M31 P2, and its commands are in
[P2's findings](../plans/m31-shot-first-adr.md#p2--found-2026-09-29).

## Date
2026-09-29

## Context

**What the product was.** Pose analysis of face-on video. The launch-monitor photo is, in
ARCHITECTURE §1's words, *"attached and displayed, never scored"*: `analysis/engine.py` builds its
outcome list empty, and `feedback/rules.py` reads no shot field. The shot side has grown since —
per-club history (ADR-024), a flight model (ADR-027), mishit exclusion (ADR-028), personal
baselines and dispersion — but every piece of it is reached through a swing: the corpus admits a
stored swing only when it has a face-on clip.

ADR-030 planned the app on that product. Its premise was *the machine is a laptop, and a phone is a
camera*: two cameras feed a laptop, which records, cuts a clip at each strike, poses it and scores
it. Its §5 put the phone app after the laptop's, carrying capture and never pose.

**The pivot (the user's, 2026-09-29).** The golfer photographs the launch-monitor screen, many times
in a session. Per-club strengths, weaknesses and grades come from those numbers. Video becomes an
optional visual aid, and the first host is a standalone iPhone app. The laptop comes back later as
a client for what only a laptop can do. The program plan's
[Why](../plans/m31-m40-shot-first-pivot.md#why) and
[Decisions](../plans/m31-m40-shot-first-pivot.md#decisions-the-users-2026-09-29) are the long form.
"Decision N" below means that list's numbering, and its items 6–17, from the M31 interview, are
what clauses 2–5 turn into rules.

**What the HD Golf data supports, measured.** Thirteen shots are stored, all `hd_golf`, all from two
bay sessions: a pair on 2026-08-10 and eleven on 2026-08-23. The two reference photos in
`data/raw/shot_screens/` are not among them.

- **Spin and spin axis are blank on 11 of 13.** Only the 2026-08-10 pair prints them, and both
  sessions used the same layout, so whether spin prints varies by session, not by layout.
- **Smash factor reads 0.76–1.06**, and `ball_speed / club_head_speed` reproduces it to rounding on
  all 13. The simulator prints implausible numbers and the OCR reads them faithfully. That is why
  `analysis/shot_measure.py` already excludes smash factor and club speed.
- **Face impact is text, and horizontal only.** `impact_position` is one of CENTER, HEEL or TOE on
  12 of 13 (7, 4 and 1). The bay screen's `Impact Position V` tile reads `---` on all 13.
- **No tile prints attack angle, dynamic loft or low point**, on either layout. ADR-027 says so for
  attack angle only; the other two rest on the tile inventory.
- **Layouts vary.** The bay and reference layouts differ by one tile (`Bounce & Roll` ↔
  `Impact Position V`), and both carry a `Custom` settings-gear tile. The screen is configurable, so
  what one golfer's device prints is not what the device can print.
- **Face-to-path is computable on 12 of 13.** Classified against `METRIC_TARGETS["face_to_path_deg"]`'s
  tolerance, that is 10 fades, 1 draw and 1 straight. The printed `Shot Type` word disagrees with the
  numbers on one shot: −1.3°, printed `SLIGHT FADE`.
- **No tile prints a landing offline.** `Horizontal Angle` is `launch_direction`, the start
  direction. The curve reaches a flight only through the printed `Spin Axis` tile, so a curved
  landing can be projected for **2 of the 13 shots, and for none of the 11 from 2026-08-23**.

So, for HD Golf, strike quality means the horizontal face-impact grid per club and its centered
rate, ball-speed consistency, and carry, start-line and face-to-path bias against scatter. It never
means low point.

## Options Considered

### Option A: Keep ADR-030's laptop-first video product
The swing stays the unit, and the app is the laptop recording two cameras.
- **Pros**: nothing is thrown away. The Rust core conforms on all 21 vectors, the pose sidecar is
  built, and the bands are earned. No new platform, and no second recognizer.
- **Cons**: the product's value rests on hardware the golfer has to set up at every session: two
  cameras, positioned and verified, and a laptop in the bay. The one thing the golfer produces on
  every shot, the screen, stays unscored. The milestones next on this path, M21's capture edge and
  M24's live session, need a camera, and the development box has none.

### Option B: Shot-first, with the phone as a thin capture client to the laptop
The phone photographs and uploads; the laptop reads, stores and grades, as today.
- **Pros**: OCR stays PaddleOCR in Python, unchanged and already tuned against these photos. No
  parser port, and the phone app is small.
- **Cons**: the laptop still has to be at the bay, or every photo waits until the golfer is home.
  The simplest session needs two devices and a network between them (ADR-016's topology). The laptop
  is still the product, so the pivot changes what is measured and not where anyone uses it.

### Option C: Shot-first on a standalone iPhone *(chosen)*
The phone photographs, reads, stores and grades. The laptop becomes a later client.
- **Pros**: the golfer already has the phone at the bay, and nothing else is required. The Rust core
  exists and conforms, and ADR-030 §4 chose Flutter precisely because it reaches the phone, so the
  core compiles in rather than being ported again. Storage on the device needs no network and no
  account, which is ADR-016's local-first posture in its strongest form.
- **Cons**: OCR has to run on the phone, where PaddleOCR does not, so the reader needs a new
  recognizer and the parser a Rust port. iOS builds need the Mac. Pose on the phone reopens the risk
  ADR-030 §2 closed, because every band was cut from MediaPipe-Python output, so it is gated rather
  than assumed (clause 8). Free signing expires every seven days.

### Sub-decision: OCR on the phone

- **Apple Vision *(chosen)***: on-device, free, and needs no entitlement beyond camera access.
  `VNRecognizeTextRequest` returns text boxes, which is exactly the shape the `TextRecognizer` seam
  in `launch_monitor/screen/recognizer.py` already takes, and VisionKit's document camera does the
  rectification that `preprocess.py` does with OpenCV today. Whether it reads *this* screen well
  enough is the product's biggest unknown, so M33 tests it first, before any app work.
- **PaddleOCR on-device**: the lab's recognizer, so its boxes would be closest to the ones the
  stored shots were read from. It means converting a model and bundling a second inference runtime,
  and OpenCV's preprocessing would have to come along too. The closeness buys less than it seems:
  what the vectors gate is the parser, boxes in and a shot out, and any recognizer behind the seam
  feeds the same parser.
- **ML Kit**: Android-first. Android is deferred (Decision 13), and the seam keeps ML Kit reachable
  when it is not.
- **A vision LLM**: still rejected, on [ADR-014](014-screen-capture-shot-ingestion.md) Option D's
  grounds (a per-shot cost, a network round trip and an API key for basic ingestion), and now also on
  clause 10's: the phone holds no key.

## Decision

**Option C, with Apple Vision.** The numbered clauses are what the addenda cite.

### 1. The unit of the product is the shot

A session of screen photos with no video is first-class and complete. Video is optional: when it
exists, the pose checkpoints are scored exactly as today (ADR-009's mechanics axis), and they sit
in **a separate panel** that never enters a topic grade or a blend (Decision 12). Adding video must
never move a club grade, and a golfer with no video is graded on the same terms as one with it.

### 2. Printed and not printed: the device capability model

**Vocabulary (Decision 6).** Device stats are *printed*; shots are *tracked* (clause 3). A stat the
launch monitor does not print is "not printed", never "not tracked".

- Each device declares every field it can print in `devices.json` (M32), as `analysed` or
  `shown_only`, with a note saying why. HD Golf's club speed and smash factor are `shown_only`,
  because of the 0.76–1.06 above.
- **Printed = declared ∩ `fields_present`**, across the golfer's own shots, because layouts vary per
  golfer. M32's function is `printed_fields(shots)`.
- **Not printed produces nothing**: no measurement, no `unscored` entry, no caveat, no tip and no
  topic. A golfer is never told about a stat their screen does not show.
- **Printed but blank on one shot** goes through the existing `unscored` refusal
  ([ADR-010](010-benchmark-ranges.md) §2): excluded and named, never guessed.
- **Nothing stands in for a stat that is not printed**, whether inferred from other numbers or from
  video. Clause 5.3's projected landing offline is the one exception, and it is named there.

### 3. Tracked shots are derived from intent, and a drill is not tracked

Whether a shot counts reuses [ADR-009](009-swing-scoring-model.md)'s `PracticeGoal`
(`contracts/intent.py`), whose `mode` is set per session and overridable per shot, as ADR-009
§Concepts already says.

- **A `DRILL` shot is not tracked. Every other mode is**, `SHOT_SHAPING` and challenge shots
  included.
- It is **derived from the mode, never stored** as a second flag, so the two cannot disagree.
- An untracked shot is still stored, shown in history and analysed on its own. It never enters club
  or player stats.
- It is **distinct from [ADR-028](028-mishit-exclusion.md)'s mishit rule**. That one is automatic
  and metric-scoped (carry and total only); this one is the golfer's choice and whole-shot.

The user: "Some drills we do not want to add to the player stats as if they were actually swinging
or actually playing."

### 4. Two levels, club and player

Every tracked shot counts toward its **club's** stats and the **player's** overall stats. Each stat
declares its scope:

- **club-only**: carry, ball speed, launch and spin. A driver-plus-wedge carry average means nothing.
- **club-independent**: strike location, face-to-path, start line and the consistency rates.

**Raw player-level averages exist only for club-independent stats.** Topic grades reach the player
level by a different route, pooling judgments rather than raw values (clause 5.4).

### 5. Grades: a grade and a list, per topic

Grading happens at club and player level, over many shots. **ADR-009's per-swing `outcome_score`
stays `None`**: a share of one shot is 0 or 100 and means nothing. ADR-009's single-swing
shot-shaping, performance and drill policies stay unbuilt, and the shape topics below are where
`SHOT_SHAPING` and `target_shape` finally get judged.

#### 5.1 A topic's grade is a share of good shots

The grade is the % of the club's tracked shots meeting the topic's criterion, carried as a rate
with a Wilson interval (`RateEstimate`, M37), and **withheld below the minimum n**. There are no
tour bands: a share is honest without one, and an outlier lowers it naturally, which is what the
user asked for ("how often they hit an average shot, so like a huge outlier would make this score
lower").

#### 5.2 Six topics, each with an availability rule

- **Strike location**: where on the face, as the centered rate and heel/toe. On HD Golf this is
  horizontal only, because `Impact Position V` reads `---`.
- **Consistency**: how often a shot lands near the golfer's typical shot for that club.
- **Low point**: only where the device prints it, so **never for HD Golf**.
- **Fade**, **Draw** and **Straight**: three shape topics, each graded **only over shots declared as
  that shape** (5.3). A shape never declared is absent: not graded, not mentioned, not in a blend.

The criteria for strike location and consistency are M37's, set against hand-worked vectors.

#### 5.3 Shot shapes

**Declared, then classified from the numbers.**

- A shape is declared by `PracticeGoal.target_shape` (`STRAIGHT`, `DRAW` or `FADE`, which already
  exist) with mode `SHOT_SHAPING`. The golfer sets it, per session or per shot, or **challenge mode**
  does: the app calls fade, draw or straight before each shot and records the call as that shot's
  `target_shape`. Challenge shots are shot-shaping shots, not drills, so they **count fully**: they
  feed the shape topics and every other topic.
- A shot is classified from its **face-to-path** (`analysis/shot_measure.py::measure_face_to_path`,
  face − path). Within `METRIC_TARGETS["face_to_path_deg"]`'s tolerance it is straight; the tolerance
  is reused, not a new constant. Outside it, positive is a fade and negative a draw. A shot with no
  face-to-path is unscored for shape.
- **The printed `Shot Type` word is not used to grade.** It stays what it is today: the cross-check
  on the spin-axis sign ([ADR-027](027-ball-flight-simulation.md) §5). Where word and number
  disagree, as they do on one stored shot, the number decides.
- **Handedness-aware, in one precise sense.** The contract signs face, path and spin axis in
  golfer-relative words (open, in-to-out, fade: `contracts/shot.py`, ADR-014 §Sign conventions), so
  the sign that separates a fade from a draw **does not flip for a left-hander**.
  `flight_infer._direction_of` already reads it as a sign, not a conversion. What handedness decides
  is **which side of the target line a shape finishes on**, so it belongs to the on-line projection
  below. A missing handedness withholds the on-line share (`no_handedness`), not the shape class. No
  left-handed shot exists yet, so M37's left-handed vector is what pins this reading.

**Two shares per shape.** "Hit the shape", and "hit the shape and finished on line". **The second is
the topic's grade** and enters the blends. The first shows beside it, so the golfer can see whether
the shaping or the aiming failed. The user: "If I am trying to hit a draw, grade how well I draw and
then land the ball on target. If you don't know don't score it."

**"Finished on line", and the one sanctioned inference.**

- Where the device prints a landing offline, that value is used.
- **HD Golf does not print one.** Where it is not printed, the landing offline is **projected from
  the printed start direction and carry, including the curve**, through ADR-027's flight model
  (`analysis/flight*.py` and `spin_solve.py`, already ported to Rust).
- **This is the one exception to clause 2's "nothing stands in"**: the user's choice, made knowing
  the objection. It is **named as projected** everywhere it appears, following
  [ADR-024](024-per-club-shot-history.md) §3's start-line projection and ADR-027 §6's split between
  measured and simulated quantities.
- **It must include the curve.** A start-line-only projection would mark every well-hit fade
  offline, because a fade starts away from the target on purpose. That is why ADR-024 §3's
  `start_line_offline_yds` cannot stand in for it.
- Where the projection cannot be computed, the on-line share is **withheld and named**.
- The on-line tolerance is `METRIC_TARGETS`' offline row unless M37 finds a reason otherwise. That
  row, `start_line_offline_yds`, is the width of a start-line projection, so M37 confirms it suits a
  landing.
- **Printed replaces projected.** If the bay trip finds that HD Golf's `Custom` tile can print a
  landing offline, the printed value replaces the projection for this device, with no code change.

**What that means at the bay today.** The curve reaches the flight only through a printed spin axis.
Face-to-path gives the curve's direction but not its size, and with no axis the flight is planar,
where `flight_measure._landing_offline` returns `None` rather than repeat the start-line number. So
the projection reaches **2 of the 13 stored shots, and none of the 11 from the 2026-08-23 bay
session**. On HD Golf as seen so far, **the on-line share of every shape topic is withheld, and only
"hit the shape" shows**, until the device prints a spin axis. The 2026-08-10 pair prints one on the
same layout, so it can; the bay trip that enumerates `Custom` should find out how. The user took
this rule knowing the consequence (after P2, 2026-09-29). M37 may still revise the grade, and this
ADR does not reopen it.

#### 5.4 Two blends, with an honest pooling rule

A **club blend** and a **player blend**, each the **equal-weight mean of the graded topics**, with
the three shape topics counting as three topics. The weights are equal because no weighting has
been measured.

At player level, a topic's share **pools every tracked shot across clubs, each shot judged against
its own club's criterion**. Judgments pool honestly where raw values cannot (clause 4), and this is
what lets a player-level consistency exist at all.

#### 5.5 Ungraded is named; impossible is absent

- A topic that is printed but too thin to grade is **excluded from the blend and named**. The
  precedent is `overall_score`: `analysis/engine.py` routes an unscorable checkpoint to
  `SwingResult.unscored` instead of the list `analysis/scoring.py` averages.
- A topic that cannot exist, because it is not printed or its shape was never declared, is simply
  **absent**.
- **A blend with no graded topic is absent, not zero.** `scoring.py`'s mean returns 0.0 for an empty
  list, so a swing with every checkpoint unscored prints an `overall_score` of 0.0. The blends do not
  copy that.

#### 5.6 No tour bands for shot metrics

Shot metrics are judged against the golfer's **personal baseline** plus the targets and tolerances
already in `contracts/dispersion.py::METRIC_TARGETS`. `analysis/benchmarks/ranges.json` is untouched.
The shot rows' tolerances there are judgment rather than measured instrument error, and they say so
in the code.

#### 5.7 The list: what is established

The grade says **how often**. The strengths and weaknesses list beside it says **what is
established**, and it keeps the program plan's rule:

- a **strength** needs an established equivalence claim: the confidence interval of the mean inside
  target ± tolerance, *and* the upper bound of the SD below tolerance;
- a **weakness** is an established bias or scatter, ranked by how far it exceeds tolerance;
- "nothing established" is **never** a strength.

The grades do not feed the list, and the list does not feed the grades.

### 6. The phone is the host

- **A standalone iPhone app.** The Rust core runs on the device through Flutter and
  `flutter_rust_bridge`, which is ADR-030 §4's shell reaching the target it was chosen for. iOS
  builds happen on the Mac, and everything else on the Windows box.
- **Storage is on the device.** Nothing on the phone listens on a port. Export (M38 P4) is how data
  leaves it, and the lab imports it.
- **The laptop is a later client (M40)**: a launch monitor wired in directly (R10 BLE,
  [ADR-004](004-launch-monitor.md)), launch-monitor APIs, and the cameras M21/M24/M25 were building.
- **Android is deferred** (Decision 13). Nothing is designed against it.

### 7. OCR on the phone

- **Apple Vision reads the screen**, behind the `TextRecognizer` boxes seam, with VisionKit's
  document camera doing rectification.
- **The parser and validator port to Rust** (`crates/screen`, M34). They are pure functions of
  boxes and a device profile, so the port is recorded from Python and gated by vectors like every
  other one (clause 9). It carries portability edges of its own, which ADR-032's addendum lists as
  M34's.
- **PaddleOCR and OpenCV stay** as the lab's reader. The two recognizers meet at the seam, so a box
  from either reaches the same parser.
- **The vision LLM stays rejected**, as the sub-decision above says.

### 8. Pose on the phone is reopened behind a conformance gate

ADR-030 deferred pose on the phone, to be revisited "only if the laptop stops being required". It
has stopped being required, so pose on the phone is M39's, behind P0's gate:

- iOS `PoseLandmarker` with **the same `.task` file, pinned by sha256**. The laptop's model is
  fetched today from a `float16/latest` URL (`pose/estimator.py`) with no hash check, so it is
  pinned first.
- VIDEO mode, CPU delegate. The stored face-on clips are re-posed on the phone and scored by
  `golf-core run`.
- **The gate**: every `passed` verdict identical, and every delta within its tolerance.
- **If it fails**, the phone records and mechanics are computed on the laptop (M40).

`ranges.json` is untouched either way. That is what the gate protects, for the reason ADR-030 §2
gave: pose is the calibration of everything downstream.

### 9. The oracle, per vector family

- **A port of existing Python is recorded from Python**, as M22 was.
- **New analysis is Rust first**, against hand-worked vectors with `provenance.oracle: "hand"`. That
  covers M37's strike profile, topic grades and blends. The phone runs only Rust, so the Rust
  implementation is the one that has to exist. A Python one written first would be a module born to
  be retired (ADR-030's 2026-09-22 addendum, and ADR-032 §7 scheduling `analysis/`'s exit), and
  hand-worked vectors keep the oracle independent of both. M37 adds the pin that every vector family
  names its oracle.
- **The lab reaches Rust-only analysis through `golf-core` subcommands**, never through a second
  Python copy.

### 10. No LLM coaching on the phone

It would need the network and an API key, and [ADR-019](019-secret-handling.md)'s key never reaches
the phone. The phone's feedback is the rule-based tips and the profile list. `feedback/coach.py`
stays a laptop and lab feature.

## Consequences

- **Milestones move.** M21, M24, M25 and M26 are paused and re-scoped under M40, with M26 moving
  whole (CI, packaging, signing and distribution; Decision 14). M28 is superseded by M39. M29 is
  blocked on M40 instead of M25, and its job is unchanged. M3's open OCR items fold into M32 and
  M33. "M4 full" is superseded by M37 for the outcome axis. M5's "superseded in shape by" moves from
  M25 to M38. M7, M20, M27 and M30 are untouched, and M27's closure is reinforced: the phone needs no
  network. `ROADMAP.md` records all of this in M31 P8–P9.
- **`ANALYSIS_VERSION` is forced to move in M32** even though no number does. Adding fields to
  `ShotData` changes every corpus vector's key set, because the vectors take `input.shot` from the
  stored `analysis.json`, `compare_results` requires identical key sets, and `crates/contracts`'
  round trip tells an absent key from a null one
  ([program-plan finding 3](../plans/m31-m40-shot-first-pivot.md#what-the-code-says-before-anyone-re-derives-it)).
  The vectors are regenerated in the same change, as the repo's invariant requires.
- **Two places say "OCR stays Python"** (finding 8): `docs/CONFORMANCE.md` §5, whose disposition
  M31 P5 changes, and the `profiles.json` comment in `tests/test_conformance.py::_PACKAGE_DATA`,
  which changes in M34, when the parser ports.
- **A mode other than `FUNDAMENTALS` cannot reach the engine today.** `analysis/scoring.py`'s
  `policy_for` raises for every other mode, and `crates/analysis`' mirror panics. So a `DRILL` or
  `SHOT_SHAPING` intent needs a policy, or needs to travel beside the swing rather than into it.
  M35 decides which, and ADR-009's addendum records it.
- **The corpus becomes shot-keyed.** `read_corpus` excludes a swing with no face-on clip
  (`ExclusionReason.NO_FACE_ON`), so photo-only entries need admitting (M35).
  `CorpusSwing.artifact_key` already keys launch-monitor samples on the photo (`shot:{sha}`).
- **Handedness has no route to a photo-only shot.** It reaches a shot today only through the swing
  manifest that holds the photo, then the player and the golfer store, so a photo with no swing gets
  `None`. M35 needs another route, or every photo-only shot withholds its on-line share.
- **The screen is read by two recognizers**, Vision on the phone and PaddleOCR in the lab, and
  nothing makes their boxes identical. The parser is what they share, and it is what the M34 vectors
  gate; whether Vision's boxes parse as well is M33's question, answered before any app work.
- **A known label hazard ships into the port unless it is fixed first.** OCR drops the `V` from
  `Impact Position V` on 2 of the 13 bay photos, and on one of them the wrong tile then wins the
  field. M32/M34 own the fix and its test, and ADR-014's next addendum corrects the cause its first
  one recorded.
- **A Mac joins the loop** for iOS builds and Vision. The Rust parser, the analysis and the vectors
  still run under `cargo test` on the Windows box, which is most of the product.
- **Unchanged:** `ranges.json`; every number the swing engine produces; ADR-008's import rule; the
  stdlib-only scoring core in both languages; the mechanics checkpoints; and "no score beats a wrong
  one", which clauses 2 and 5.5 extend rather than relax.

## Deferred, by choice

- **Android.** The boxes seam keeps ML Kit reachable, and nothing is designed against it until then.
- **The paid Apple Developer Program.** Free Personal Team signing (seven-day installs) serves until
  M40, which carries packaging, signing and distribution.
- **R10 BLE and launch-monitor APIs** (M40). Each is a `devices.json` entry plus an adapter, and
  clause 2's capability model is what lets a second device arrive without a code change to scoring.
- **Low point for HD Golf.** The device does not print it, so the topic is absent for this device
  and nothing stands in for it. A device that prints it gets the topic with no new rule.
- **User-set blend weights.** Equal until a weighting has been measured, and the blend names the
  topics it averaged, so a later weighting has something to act on.

## Addendum (2026-09-30, M31.5): the oracle moves to Rust, the lab's reader ports, and M35 follows M36

**M31.5 P4**, docs only. [ADR-035](035-rust-everywhere-python-where-required.md) supersedes §7's lab
reader, part of §9, and the Consequence that blocked M29 on M40
([its clause 7](035-rust-everywhere-python-where-required.md#7-what-this-supersedes-sentence-by-sentence)).
The rest of this ADR stands. Nothing here was built, so nothing built changes. The reasons are
ADR-035's. This addendum names the sentences above that stop holding.

### §9: recorded from Python once, and by Rust after that

- **"A port of existing Python is recorded from Python, as M22 was" now means once.**
  - The recording happens before the port moves. After that, Rust re-records under a structural diff
    gate ([clause 3](035-rust-everywhere-python-where-required.md#3-the-oracle-moves-to-rust)).
  - M34's parser and M36's aggregates and stores are recorded from the frozen Python lab. Adding
    those families to the recorder is the one change the frozen lab is allowed
    ([clause 4](035-rust-everywhere-python-where-required.md#4-the-frozen-python-lab)).
- **"New analysis is Rust first, against hand-worked vectors" stands, and it widens to all new
  behaviour.** That includes new behaviour on a port.
  - M34's tie rule and its `Impact Position V` tile are new behaviour on a port. They get hand-worked
    vectors on top of the Python recording
    ([P2 finding 6](../plans/m31-5-rust-first-replan.md#p2--found-2026-09-30)).
  - M37's pin, that every family names its oracle, stands.
- **"The lab reaches Rust-only analysis through `golf-core` subcommands" is superseded.**
  - The frozen lab reaches no new analysis at all. From §M29 the lab is a Rust CLI.
  - The sentence's other half, "never through a second Python copy", stands and now covers
    everything.
- **§9's reason now applies to M35 too.** That reason is that a Python module written first "would
  be a module born to be retired". ADR-035 applies it to M35, which the program plan had planned as
  Python (ADR-035's Option A).

### §7: the lab's reader ports too

- **"PaddleOCR and OpenCV stay as the lab's reader" is superseded.** The reader ports in §M29, using
  `ort` to run the same Paddle models
  ([clauses 2](035-rust-everywhere-python-where-required.md#2-everything-else-ports-including-the-three-things-considered-and-not-kept)
  and [5](035-rust-everywhere-python-where-required.md#5-the-lab-port-is-m29-re-scoped)).
  - Its gate is the 13 stored bay photos. On each photo, the shot parsed from the Rust reader's boxes
    is compared with the shot parsed from PaddleOCR's boxes.
  - The two recognizers still meet at the seam, and the parser is still what the vectors gate.
  - The vision LLM stays rejected.
- **"The port is recorded from Python" means once**, as in §9 above.
- **The Python parser is frozen with its tie hazard** (clause 4). So from M34 until §M29, the phone
  and the lab read the two label-fix shots differently (ADR-035's Consequences).

### Clause 6 and the Consequences

- **Clause 6 says "Export (M38 P4) is how data leaves it, and the lab imports it".** The lab's
  importer becomes a verb of §M29's Rust lab CLI (Q8), so M38 P4 waits on M29.
- **"M29 is blocked on M40 instead of M25, and its job is unchanged" is superseded by clause 5.**
  §M29 is the lab port. It runs after M36, depends on M34 and M36, and is not blocked on M40. M40
  does the deletes.
- **"`ANALYSIS_VERSION` is forced to move in M32 … The vectors are regenerated in the same change"
  still holds, in Rust only.**
  - `golf-core` re-records them, diff-gated. The declared diff is the new `ShotData` keys, plus the
    version number itself.
  - Frozen Python's version stays at 16, by design (clause 3).
- **The Consequences that give work to M35 are still M35's, but in Rust and after M36**
  ([clause 6](035-rust-everywhere-python-where-required.md#6-order-the-phone-path-first)). There are
  three: a policy for a mode other than `FUNDAMENTALS`, photo-only corpus admission, and a
  handedness route for a photo-only shot.
  - M31's program plan had M35 write them in Python, as M36's oracle.
  - Now M36 ports `read_corpus` as it behaves today. M35 then changes it in Rust, with hand-worked
    vectors.
  - `shot_result` moves from M36's list to M35's.
- **"A known label hazard ships into the port unless it is fixed first … M32/M34 own the fix."**
  M34 owns it, in Rust only, and the frozen Python parser keeps it (clause 4).
- **"The screen is read by two recognizers, Vision on the phone and PaddleOCR in the lab."** There
  are still two. From §M29 the lab's recognizer runs Paddle's models through `ort`.
- **"The stdlib-only scoring core in both languages"**, in the list of what is unchanged. The Python
  half is frozen from M32 and deleted with `analysis/` in M40. The Rust half is the one that matters
  from M32.

### What this does not change

- **Clauses 1–5, 8 and 10.** These are the shot as the unit, the capability model, tracked shots,
  the two levels, the grades, pose on the phone behind M39's gate, and no LLM on the phone.
  - Clause 10's `feedback/coach.py` stays a laptop and lab feature.
  - It is one of the two things ADR-035 keeps in Python
    ([clause 1](035-rust-everywhere-python-where-required.md#1-the-rule-and-the-two-exceptions-it-names)).
- **Clause 6's standalone iPhone, and its on-device storage.**
- **`ranges.json`, ADR-008's import rule, and "no score beats a wrong one".**
- **The order this ADR set for M32–M34 and M37–M39.** ADR-035 changes only three things in it:
  - it puts M31.5 before M32;
  - it runs M36 before M35;
  - it moves §M29 from after M40 to before it.

## Addendum (2026-10-10, M35): the Consequences' open questions answered, and the photo-side reasons first emitted

**M35**, in Rust only, in [the M35 plan](../plans/m35-shot-first-sessions.md). The previous
addendum left three Consequences with M35: a policy for a mode other than `FUNDAMENTALS`, photo-only
corpus admission, and a handedness route for a photo-only shot. M35 answered all three, and wrote
clause 2's photo-side refusals for the first time. **The Status's "Nothing here is built" stopped
holding at M32**: M32 built clause 2's capability model, M34 the parser clause 7 needs, M36 the
corpus clause 4 pools over, and M35 the pieces below. None is wired to a caller yet: the first are
M38's phone and §M29's Rust lab CLI.

### The three Consequences M35 owned

- **"A mode other than `FUNDAMENTALS` cannot reach the engine today."** Every mode now reaches it and
  scores mechanics as `FUNDAMENTALS` does, so clause 1's panel is there for a drill too. The mode
  decides only which aggregates a shot feeds, which is clause 3. So neither answer the Consequence
  offered was taken as written: there is no new policy per mode, and the mode does not travel
  beside the swing. It goes into the swing (`SwingResult.intent`) and into the shot
  (`ShotResult.intent`), and the corpus reads it. [ADR-009](009-swing-scoring-model.md)'s M35
  addendum records the decision and its table.
- **"The corpus becomes shot-keyed."** Rust's `read_corpus` admits a *photo entry* (a shot photo,
  no face-on clip), dedupes photo entries by photo hash, makes a photo-only copy of a filmed shot
  the filmed copy's `duplicate` (one ball flight, and the copy with video counts), and leaves a
  `DRILL` out as `untracked`. Clause 3's "an untracked shot is still stored, shown in history and
  analysed on its own" holds: a drill gets its own `ShotResult`, or its `analysis.json` when filmed,
  and reaches no aggregate. [ADR-024](024-per-club-shot-history.md)'s M35 addendum is the decision.
- **"Handedness has no route to a photo-only shot."** It has one, and it was there already. A photo
  entry is a swing directory with a manifest, so its handedness comes from the manifest's golfer
  through the golfer store, as a swing's does. What has no route is a photo in the shot store that
  no session holds, and the corpus never reads one. §M29's import verb files a photo into a session
  with its golfer and club. **A shot needs a handedness only when its screen printed a spin axis**,
  because `+ = fade` is a right-handed reading. So the note naming a missing golfer is appended
  only when the flight refused its landing offline for want of one, where a swing's is
  unconditional, because `head_stays_back` always needs it.

### Clause 2's refusals, first emitted

`printed_blank` and `misread` existed since M32 and were written nowhere. M35 writes them, from
`crates/screen::tile_refusals`, into `ShotResult.unscored` (written as `shot_analysis.json`) and
**never into a `SwingResult`**, so frozen Python, which reads `analysis.json` with a closed enum,
never meets them (ADR-035 clause 4). The rule (the plan's interview decision 3 and call 3), for a
field the device printed and declares `analysed`, that holds no value:

| what the parse recorded under the field's label | reason |
|---|---|
| no `raw_fields` key: located, and withheld by the tie rule | `misread` |
| a blank marker such as `---` | `printed_blank` |
| `""`: nothing under the label | `printed_blank` |
| other text that gave no value, such as `Open` | `misread` |

- **An empty cell is `printed_blank`, not a misread.** On every stored bay photo the blank is a
  `---` that OCR does not see, under Spin, Spin Axis and Impact Position V, so telling the golfer
  to retake would be wrong on most shots. Only `misread` says capturing again helps.
- **A shown-only tile left blank is shown as blank and refused nothing**, and one holding a value
  reaches no measurement. Both follow from clause 2's split, by construction: the engine measures a
  view of the shot with every field outside "printed and `analysed`" set to `None`. So "not printed
  produces nothing" holds with no per-row field list to drift. On every stored shot the view blanks
  only HD Golf's club speed and smash factor, which no measurement reads.
- **No refusal on an unstamped shot or a direct feed.** An unstamped shot's printed set is only the
  fields that hold a value, so it names no blank. Every shot in `data/` is unstamped, so no blank is
  named on a stored shot until §M29's reader re-reads them. A shot with no provenance was never a
  photo, so neither photo-side reason can apply; a direct device's blanks wait for one (M40).
- **The order is the device's declared tile order**, which `devices.json` lists in the profile's
  tile order, and a pin now holds that.

**Clause 1's promise is pinned.** "Adding video must never move a club grade" starts with the shot's
numbers. `analyze_shot` makes the same calls on the same fields as a swing's shot half, and
`crates/analysis/tests/shot.rs` holds its rows to every committed corpus vector's shot and flight
rows.

### What this does not change

- clauses 1–10 as the previous addendum left them, including clause 5's grades, which are M37's,
  and clause 3's "derived from the mode, never stored", which `PracticeMode::is_tracked` is;
- the order the previous addendum set, with M37 after M35.

## Addendum (2026-10-15, M37): clause 5 built, strike location graded only from a measured offset, and the list written to §5.7

**M37**, in Rust only, in [the M37 plan](../plans/m37-strike-profile-and-grades.md), whose interview
decisions, calls and phase findings are the as-built record. Clause 5 is built: each club's strike
profile, the six topics' grades, the club and player blends, and the strengths and weaknesses list,
in `crates/contracts/src/strike.rs` (the shapes), `crates/analysis/src/strike.rs` (the judgments
and the counting), `crates/feedback/src/profile_rules.rs` (the list) and
`crates/core/src/profile.rs::run_profile` (the join), reached through `golf-core profile`. It is the
first analysis written Rust first with no Python behind it at all, as §9 asked, and its oracle is
nine hand-worked cases (`spec/vectors/profile/hand/`). It is wired to nothing yet: its callers are
M38's profile screens and §M29's `rmcp` tool, `get_strike_profile`.

### The criteria clause 5 left to M37, as the interview set them

| Topic | A good shot | Exists for a club when |
|---|---|---|
| Strike location | `hypot(h, v) ≤ 10` mm from face centre, or `\|h\| ≤ 10` where only the horizontal offset is printed (`CENTERED_RADIUS_MM`, provisional) | `impact_offset_h` is printed |
| Consistency | carry within ±10% of the club's median carry, **and** start-line offline within ±10% of that median carry of the club's median offline (`CONSISTENCY_FRACTION`); a mishit is a miss | carry and launch direction are printed |
| Low point | ahead of the ball (`> 0` in) | low point is printed, and the club is an iron, wedge or hybrid |
| Fade, draw, straight | hit the declared shape and finished within ±9 yd of the line | face angle and path are printed, and a tracked shot of the club was declared `shot_shaping` with that shape |

- **Every grade is withheld below 10 judged shots** (`RATE_MINIMUM_N`), and every share is a Wilson
  95% interval: about ±26 points at n = 10 and a 50% share. A withheld grade is named and left out
  of the blend; an absent topic is not named at all (§5.5, as written).
- **The typical shot is the median of the club's non-mishit judged shots**, carry and offline taken
  apart. A median, not a mean, so the outliers the grade punishes do not also move the yardstick,
  and a mishit stays in the judged count and fails, which is what the user asked for: an outlier
  lowers the grade.
- **A topic exists from what the device analyses, not what it shows.** Existence reads
  `capability::analysed_on` over the club's shots. A shot whose input is printed but blank is
  unscored for that topic, counted under its reason (`printed_blank` or `misread`, from the shot's
  stored refusals), and never counted as a miss.
- **Each statistic declares its scope** (`contracts::strike::STAT_SCOPES`). Carry, total, ball
  speed, launch, spin and the start-line offline are club-only, because their yards scale with the
  club. Face-to-path, start line and the impact offsets are club-independent, and only those get a
  raw player-level number. §5.4's pooling is built as written: the player's grade judges every
  tracked shot by its own club's criterion. Untagged shots count toward strike location and the
  shapes, whose criteria need no club, and not toward consistency or low point. On `data/` that is
  why the first grade real data shows is the player's consistency, where no single club reaches the
  floor.

### §5.2's strike location: picture only on HD Golf, and measured from M42

§5.2 said strike location on HD Golf would be the centered rate and heel/toe, "horizontal only".
**That is superseded: HD Golf's printed `CENTER`/`HEEL`/`TOE` word is not graded.** It is a
three-way category with no magnitude, so a share built on it would grade the screen's binning, not
the strike, and the user called it "picture only". Strike location is graded only from a measured
face-impact offset, `ShotData.impact_offset_h` and `impact_offset_v` in millimetres (+ toe, + high,
the fields M32 added). No device in use supplies one. The bay will also run Full Swing's software,
whose impact video shows where the club met the ball, and the iPhone's Live Photo captures that
video beside the still. Reading the offset from it is **M42**, which starts with a feasibility spike
on real Live Photos, adds Full Swing's profile and `devices.json` entry, and revises
`CENTERED_RADIUS_MM` from the reader's measured error. Until then strike location is **absent** on
real data, the stored shots included.

The impact grid is 3 × 3 with per-axis bins at ±10 mm (or one row when the vertical is not
printed), while the centered rate uses the radius. The two differ at the corners of the centre
cell on purpose: the grid is a picture of where, the rate is the judgment.

### §5.3: the landing, the tolerance and the left-hander, as built

- **The 9-yard on-line tolerance is confirmed for a landing** (`METRIC_TARGETS`'
  `start_line_offline_yds` row, reused), knowing it is generous for a wedge and tight for a driver.
- **The shape boundary is inclusive**: straight when `|face_to_path| ≤ 2.0°`, a fade above, a draw
  below, for either hand.
- **Each landing names its basis.** A printed `carry_offline` is used where the device analyses it.
  Otherwise the flight model's `flight_landing_offline_yds` is used, **projected**, and named so in
  the grade (the topic carries how many landings were printed and how many projected) and in the
  report. Otherwise the shot has no landing: it leaves the on-line share's denominator and is
  counted under its reason, read off the shot's stored refusals in order: a blank printed landing's
  own (`printed_blank` or `misread`), then the flight's (`spin_axis_unresolved`, `no_handedness`,
  or the carry's refusal), else `unrecorded`. "Hit the shape" still counts it. On HD Golf as stored, the projection reaches 2 of the 13 shots, as
  §5.3 measured.
- **The sign does not flip for a left-hander**, as §5.3 already said and the program plan's §M37
  had backwards. The profile reads no handedness at all: a left-hander's fade finishes on the other
  side of the line inside the projected landing's own sign. The hand case
  `left-handed-fade-and-draw-projected` is the left-handed vector §5.3 asked for.

### Clause 2's test device, and §5.7's list

- **A `synthetic` device in `devices.json`** declares every `ShotData` reading as `analysed`, so the
  hand cases can reach what no device in use prints: the impact offsets, low point and a printed
  landing offline. `mock` set the precedent of a test device, and
  `crates/contracts/tests/capability.rs` keeps `synthetic` out of every vector family that describes
  a real bay.
- **`rank_profile` is written to §5.7**, and it did not exist before in either language: "unchanged
  in rule" in the program plan meant the rule. A **weakness** is the dispersion's own established
  bias or scatter, ranked by its point estimate's ratio to tolerance (`|offset| / tolerance` for a
  bias, `sd / tolerance` for a scatter), ties in `METRIC_TARGETS` order and a bias before a scatter.
  A **strength** needs the mean's interval inside target ± tolerance (inclusive) **and** the SD's
  upper bound below tolerance (strictly). Strengths follow the weaknesses, ranked smallest first by
  `max(|CI end − target|, SD upper bound) / tolerance`, which is at most 1. "Nothing established"
  lists nothing. Only launch-monitor readings with a stat scope list, so no pose metric
  and no flight projection ever does. The player's list (`rank_player`) reads the whole-corpus
  dispersion, club-independent metrics only. The grades do not feed the list, nor the list the
  grades.

### What real data shows

`golf-core profile --name Aaron` over `data/` (the M37 plan's P10 finding 5): two clubs, each with
consistency alone and withheld at 6 and 5 shots of 10. Strike location, low point and every shape
are absent, because HD Golf prints none of their inputs and no session records a shape. The player
pools 13 shots, 2 untagged, and its consistency is graded at 6 of 11, 95% CI 28% to 79%. The list
names face-to-path's bias and scatter and start line's scatter.

### What this does not change

- clauses 1–4 and 6–10, and §5.1, §5.4, §5.5 and §5.6 as written; this addendum fills in what they
  left to M37 and corrects §5.2's strike-location sentence alone;
- mechanics as a separate panel: no pose measurement enters a grade, a blend or the list, which a
  test in `crates/analysis` and one in `crates/feedback` pin;
- `ranges.json`, which no grade reads.

## Addendum (2026-11-02, M42): Full Swing is the second device, a value off its impact video counts as printed, and the reader has read none of its clips yet

**M42**, in Rust only, in [the M42 plan](../plans/m42-full-swing.md), whose interview decisions, calls
and phase findings are the as-built record. **M41 is folded into it** (decision 2): M41's
"course-play layout" photo, `2026-10-07-1`, is the same Full Swing grid screen as five of the six Live
Photos M42 was planned from, so M41's open questions (the lit L/R and D/F signs, the spin tiles with
no `ShotData` key, the doubled `TOTAL YD`) were answered here, and M41 closes as merged. The screen
reader's half is [ADR-014's M42 addendum](014-screen-capture-shot-ingestion.md#addendum-2026-11-02-m42-the-second-profile-four-opt-in-parser-rules-the-glyphs-colour-and-no-version-bump),
and this one is what clause 2 and §5.2 are told.

### Clause 2, with a second real device

`crates/contracts/devices.json` declares `full_swing` beside `hd_golf`, in its profile's tile order.
`crates/screen/tests/capability.rs` holds every device with a profile equal to that profile's targets
now, not `hd_golf` alone.

- **Club speed and smash factor are `analysed` on Full Swing, and stay `shown_only` on HD Golf**
  (decision 5). HD Golf's club speed does not agree with its own ball speed: the smash the two imply
  sits below 1.0 on most stored shots, which a real strike cannot produce, and the simulator prints
  the smash from those two, so it carries the fault (`crates/analysis/src/shot_measure.rs`, M31 P2's
  finding 1). Full Swing measures the club itself, and the low smashes on its photos (0.85, 1.07,
  1.08) look like real off-centre strikes rather than a broken club speed. The same field is
  analysed on one device and shown on another, which is what a per-device declaration is for.
- **`backspin` and `sidespin` are stored as printed, and no analysis reads them** (decision 4). `ShotData`
  gains both: rpm, `backspin` never negative, `sidespin` + = fade and − = draw, read from which of
  the screen's `D`/`F` is lit, the same polarity as `spin_axis` and never converted by handedness.
  Both are `shown_only`, and `spin_rate` and `spin_axis` stay unset on every Full Swing shot: whether
  a rate and an axis computed from them are worth having is the user's to decide later. There is no
  cross-check of the sidespin's sign against face-to-path, because five of the six photos show `D`
  with the face open to the path, which a toe strike's gear effect produces. Each key is written
  only when set (`shot::WRITTEN_WHEN_SET`, the precedent M37 set for `CorpusSwing`'s two keys), so
  **no committed vector moved and no version key moved**, and every reader of an older shot reads
  `None`. `golf-mcp`'s `NUMERIC_METRICS` lists both.
- **The device stays the caller's choice** (call 10): `--device full_swing` or
  `GOLF_LAUNCH_MONITOR_PROFILE=full_swing`, with `hd_golf` still the default and nothing
  auto-detected. The phone's per-session choice is M38's.

### The question clause 2 left to M42: a value off the impact video counts as printed

**The replay is the device's own output, photographed, so a value read from it counts as printed**
(call 13). A `devices.json` field entry gains an optional `from`, `screen` by default or
`impact_video` (`capability::FieldSource`), and `full_swing` declares `impact_offset_h` `analysed`,
`from: impact_video`, last. **`impact_offset_v` is never declared and never comes from this replay**:
it is shot from overhead, where heel to toe is visible and high to low is not. Read "printed" here
per capture. A still's shot never lists the field, so a still's offset is not printed and nothing
downstream names it, as clause 2 has it for anything not printed. A clip's shot goes one of three
ways, all in `golf_lab::shot::record_impact_outcome`:

| The reader on the clip's frames | `impact_offset_h` | `fields_present` | What else |
|---|---|---|---|
| read | the value | listed | nothing |
| refused | `None` | listed | the warning `impact video: {the refusal's sentence}`, and `tile_refusals` names it `misread`, because retaking the Live Photo during the replay can help, with the detail `screen::IMPACT_VIDEO_MISREAD` |
| could not decode the frames | `None` | not listed | the warning `impact video: could not be decoded: {error}`: a machine's problem, and nothing about the shot |

A clip ffmpeg cannot decode at all fails that clip alone, as an unreadable photo does, and no shot is
made. **A refused replay moves neither `needs_review` nor the exit code**: the flag sets a shot's
numbers aside (the corpus's `shot_needs_review`), and the grid's numbers are as good as they were.
A line under the import's row says what the replay gave, and its last line counts the shots with no
offset. On an entry in a session, strike location then exists for the club and such a shot is
unscored under `misread`, never a miss (§5.5, and the M37 addendum's rule for a tile printed and
holding no value), so a club whose every offset was refused has its grade withheld rather than low.

### A Live Photo is two import methods, neither needing the other

The user's call (decisions 7 and 8), replacing the plan's first design, where a still found its motion
part beside it. "Don't make the video a requirement, just treat the two different pictures we have as
two different methods to import data."

- **The still**, through `golf-lab import-photos`: the grid's numbers and no impact offset.
- **The clip**, through `golf-lab import-video`: the grid's numbers, read off one of its frames by the
  still's own reader (ADR-014's M42 addendum has the frame rule and the clip gate), **and** the impact
  offset or its named refusal off the replay. Why the clip matters, in the user's words: "The graphic
  doesn't accurately show where the ball was hit. The video is the best indicator we can get."

**Nothing pairs a still with its clip, so importing both halves of one Live Photo makes two shots of
one swing**, by their two digests, and both count. The user avoids it by choosing one method per Live
Photo. `import-photos` never looks beside a still and its scan never picks up a `.mov`; `import-video`
never picks up a still. M42 means a Live Photo's 1 to 2 s motion part and nothing else: an ordinary
phone video of the replay is outside it, because none exists to tune on, though nothing inspects a
`.mov` to refuse one. A device declaring no `impact_video` field refuses `import-video`, naming those
that do. How an entry from a clip is stored
is [ARCHITECTURE §4](../ARCHITECTURE.md#4-storage--what-is-actually-persisted).

### The reader is wired in, and has read none of the five clips

`crates/impact` is the reader, and M38's phone will call it too: std only, no Python twin, frames in as
RGB bytes and no video decoded. Per clip it finds the ball, the club head, the frame of contact and the
shaft that tells heel from toe, then projects the ball's centre on the heel-to-toe axis, scaled by the
ball's Rules of Golf diameter, with the head bar's midpoint standing for the face centre. Those two
are stated approximations, and the crate doc says so. **Whether its readings are accurate was not
judged in M42** (decision 3): no face-tape or foot-spray test stands behind any number, in the user's
words "I can take a look at it after."

**It read 0 of the 5 grid clips, at its starting values and at every tuning the plan's brackets allow,
so no constant moved** (the plan's P13 finding has the runs). `IMG_2889`, `IMG_2891` and `IMG_2892`
refuse with `NoShaft`, `IMG_2893` and `IMG_2894` with `NoContact`, and each refusal follows a wrong
ball. The replay's ball is an ellipse wider than tall, 1.35 to 1.47 by its bounding box, which fails
the ball stage's fixed `ASPECT`, and the longest chain of still white discs is always a mark of the
screen's own, a letter of the name banner or a digit of the grid. **It is wired in regardless, with no
go or no-go** (decision 9), so today every clip's shot holds the grid's numbers with its offset
`misread`. A forced-ball diagnostic got no further (0 read, and one five-constant combination reading
one clip at a −47.5 mm nothing should trust): the club head merges with the ball over the contact
frames. A reader that works needs a ball stage that accepts the ellipse and either confines itself to
the replay panel or prefers a chain that ends, and a head stage that survives the merge. Those are new
stages, which M42 recorded and did not build, and `read-impact --review` wrote what the reader saw to
`data/raw/impact-review/` for the user's later look.

**The reader has no version key.** A current stored shot is `cached` on its parser version alone, so
when `crates/impact` is tuned later a shot already stored keeps its old answer until `golf-lab
import-video --force` reads the clip again. Conversely, a re-read after a `SCREEN_PARSER_VERSION` bump
reads the PNG, which holds no replay, so it carries what the older read's replay gave
(`shot::carry_impact_video`) rather than turning a clip's shot into a still's.

What the plan left open, as the crate decided it (each is in the module it names):

- **`frame` and `component`.** Coordinates are continuous, pixel `(i, j)`'s centre at `(i + 0.5, j +
  0.5)`. A component's `length` and `width` are the spread of its pixel centres' projections plus one
  pixel, so a straight run of n pixels measures n. A median of an even count is the mean of the middle
  two. Call 12's fixed numbers are private constants in `ball` (`ASPECT`, `FILL`, `CHAIN_STEP`,
  `SEARCH_SIDE`), and only its tunables are `pub const`s in `lib.rs`.
- **`ball`.** A chain's step is measured against the diameter of the candidate joining it, and of two
  predecessors on equally long chains the nearer continues. `find_ball` and `Ball` are public where
  call 12 said crate-private, because `tests/ball.rs` is an integration test and `read-impact` rings
  what it chose on a refused clip's review image.
- **`shaft`.** Shafts at both head ends are no shaft: taking the nearer join would be a guessed sign,
  so a guessed toe is refused as a guessed letter is. The nearest earlier frame with a head, and only
  that one, is asked when the contact frame has no shaft, and it lends its heel and not its geometry:
  the heel is carried by the sign of the two heads' axis dot product, and axes at right angles carry
  nothing.
- **The rest.** Of two head candidates at one distance, the first a row-major scan meets. The disc
  taken out of the light mask is the pixels whose centres lie within 0.55·D. A distance exactly at
  `CONTACT_MAX_DISTANCE`·D is contact. A rounded −0.0 is written 0.0, an offset of exactly ±60.0 is a
  reading, and `OutOfRange` carries the rounded offset. The refusals' sentences are lower case and unpunctuated, for the warning above and `read-impact`'s
  `<name>: not read - <sentence>`. A frame whose buffer is the wrong size panics, as the caller's bug.

### §5.2 and `CENTERED_RADIUS_MM`

The M37 addendum said M42 "starts with a feasibility spike on real Live Photos" and "revises
`CENTERED_RADIUS_MM` from the reader's measured error". **Neither happened.** No spike ran, since the
user chose to build and try the reader instead. With no ground truth there is no measured error to
revise from (decision 3), so **`CENTERED_RADIUS_MM` stays 10 mm and provisional** (call 16), and its doc
says so. Strike location is still graded only from a measured offset, and with no stored shot holding
one and nothing imported into `data/processed/` by M42 (call 17), it is still absent on real data, as
that addendum's *What real data shows* has it.

### What this does not change

- `hd_golf`'s declaration, its profile and every shot read under it;
- clauses 1 and 3–10, and §5's criteria: a clip's offset, once read, is graded by the M37 rule as it
  stands;
- the 13 stored shots, and `data/processed/`, which M42 wrote nothing into;
- `ANALYSIS_VERSION`, `SCREEN_PARSER_VERSION` and `CAREER_VERSION`, and every committed vector, with
  three hand-worked screen vectors added beside them.

## Addendum (2026-11-04, M33): GO, provisional: pasted copies, and the biggest unknown is smaller and not closed

**M33**, on a Mac, in [the M33 plan](../plans/m33-vision-spike.md), whose decisions and phase findings
are the as-built record. The measurements are [the spike's log](../../spikes/vision-ocr/log.md), and
`probe.py verdict` works its verdict and tables again from the committed boxes and results. Nothing
under `crates/`, `src/`, `scripts/`, `tests/` or `spec/` moved. What Vision's boxes and confidences
mean to the reader is
[ADR-014's M33 addendum](014-screen-capture-shot-ingestion.md#addendum-2026-11-04-m33-vision-behind-the-seam-its-confidences-in-effect-a-constant-and-no-parser-rule),
and this one is what the sub-decision and clause 7 are told.

### The verdict

**GO, provisional: pasted copies. Clips: not measured.** This is not yet the answer on the photos as
captured. M33 is **open**, and whether M38 is unblocked on this evidence is the user's call, which this
addendum does not make.

By the table in [`thresholds.md`](../../spikes/vision-ocr/thresholds.md), committed before Vision read
one real photo and edited by no phase since, stills that pass on a phone path read GO. They passed on one
path, at C0, the first settings, so the verdict carries neither **tuned** nor **tuned, parser**:

| mode | wrong (at most 0) | read, of 72 (at least 66) | lowest field, of 6 (at least 5) | flagged, of 6 (at most 1) | arm |
|---|---|---|---|---|---|
| `raw`, the photo as imported | 0 | 71 | 5: `sidespin` | 0 | **PASS** |
| `vision`, a stand-in for VisionKit's scanner | 0 | **59, miss** | **4: `sidespin`, miss** | 1 | **MISS** |
| `lab`, the lab's rectifier in front of Vision (a diagnostic, and not the lab's reader) | 0 | 68 | 5: four fields | 1 | **PASS** |

- **`raw` has no room on one bar.** It is at T3's limit, so one more lost sign on any photo misses it.
- **On this input `vision` is not an independent arm.** `VNDetectRectanglesRequest` found a rectangle on one
  photo of six, and that one is the replay panel, which holds none of the tiles. The other five fell back
  to `raw`'s exact bytes. So "`raw` or `vision`" is `raw` alone here.

### Full Swing decides, HD Golf is diagnostic, and the parser is Rust's

The plan's interview overturned three things in the program plan's §M33, which was written before M34
existed and asked its question of HD Golf:

- **Full Swing decides the verdict.** In the user's words: "Full swing matters most."
- **HD Golf is diagnostic**: measured and logged, with no bar and no verdict of its own, which the user
  chose over a verdict that could not block. In this run it did not run at all (below).
- **The parser is Rust's**, `golf-core parse-screen`, the code the phone will call, and not frozen
  Python's. So the Consequences' "whether Vision's boxes parse as well" is asked of `crates/screen`.

The gate also gained a coverage floor (T2 and T3) and a review ceiling (T4), because "no unflagged wrong
value" alone passes a recognizer that always loses one field, or one that flags everything.

### Where the sub-decision's "biggest unknown" now stands

**Smaller, and not closed.** Before M33 no real photo had met Vision. Now macOS's Vision, untuned and with
no parser change, has read six copies of the screen that matters most, with no wrong value on any arm, and
on one path it passes the bars set before it ran. That is evidence on "whether it reads *this* screen well enough",
and it is provisional for the reasons below.

**"As well" is not answered.** The lab's reader did not run beside it. M42's 71 of 72 for that reader was
measured on the original files, so it does not stand beside this run's 71, whatever the two counts look
like.

**Clause 7's "with VisionKit's document camera doing rectification" is the part left least settled.** The
arm that passed has no rectifier at all, and the stand-in found the wrong rectangle or none. The likely
reason, by eye and not by test, is that neither side edge of the monitor is in any of the six frames.
`vision`'s miss says nothing about VisionKit's scanner, which needs a device, so M38 measures it. Clause 7
is not amended here.

### What is still unmeasured

- **The original files.** The six stills are JPEGs saved from a chat paste, by the user's decision of
  2026-11-04, when the originals could not reach this Mac. Five of the six were HEICs, so those are
  re-encoded and scaled, and none carries camera metadata.
- **The five clips.** They are not on this Mac, and that absence is this run's reading of the user's
  "proceed", not the user's stated choice. So the clips have no verdict, and nothing here tells M38 to ship
  the still import first.
- **HD Golf's fifteen photos**, by the user's omission ("Omit HD golf for now"). The `hd_golf` path has no
  Vision evidence behind it, and the `Impact Position V` hazard in the Consequences is unanswered for
  Vision.
- **The lab baseline.** The reader's models are not on this Mac, the same reading of "proceed", so there is
  no lab column.
- **C1 to C4.** The ladder never ran, because stills are this run's target by that decision and `raw`
  passed. C0 is the only config measured, so it is not shown to be the best one.

M33 closes when the stills are measured again on the original files and the clips, HD Golf and the lab
baseline have run. Beyond those, what M38 P5's bay session and the plugin test: six stills from one setup
are not a new session, macOS's Vision is not iOS's, a stand-in is not VisionKit, and with no **wrong** cell
on any arm `needs_review` had nothing to catch.

### What this does not change

- Apple Vision stays the chosen recognizer, the sub-decision's other options stay as they were set aside,
  and clauses 1–10 stand;
- every crate and committed vector, and `ANALYSIS_VERSION`, `SCREEN_PARSER_VERSION` and `CAREER_VERSION`;
- `data/`, which M33 only read, and the stored shots.
