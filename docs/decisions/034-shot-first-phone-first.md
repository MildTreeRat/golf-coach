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
