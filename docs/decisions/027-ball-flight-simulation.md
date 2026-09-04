# ADR-027: Ball-flight simulation — the model the launch angle was recorded for

## Status
**Accepted** 2026-09-04. The phases are M15, whose section in [ROADMAP.md](../../ROADMAP.md) this
document's P0 wrote alongside it. **This ADR is ahead of its code** — P1–P18 are unbuilt, so every
claim below is a design intention rather than a measurement, with one exception: the counts of what
is on disk in §Context are read off the artifacts and are true as written. Read the rest as a plan,
and expect the corrections to arrive here as addenda.

## Date
2026-09-04

## Context

This ADR lifts a deferral. [ADR-024](024-per-club-shot-history.md)'s *Deferred, by choice* named a
ball-flight model as downstream work, [ADR-026](026-club-specification-lookup.md) kept it deferred,
and `ROADMAP.md`'s M12 section states it flatly: *"Deliberately out of scope: ball trajectory, swing
efficiency, gapping and club fitting."* Three of those four stay deferred. This one does not, and
the reason is that the thing it was waiting on has arrived — M12 landed the club specification, M9
landed the launch conditions, and what remains missing is smaller and more specific than the
deferral assumed.

Two facts decide the shape of everything below.

### 1. Loft and lie are not inputs to ball flight

The question that prompted this milestone was whether the ball's path could be predicted from
launch-monitor numbers *plus the loft and lie of the club*. The launch-monitor half is right and the
club half is not, and the distinction is worth stating precisely because it is the one most likely
to be re-introduced by a later reader who assumes a more detailed club model must produce a more
accurate flight.

Once the ball separates from the face, its path is determined by its **launch conditions** — ball
speed, vertical launch angle, horizontal launch direction, spin rate, spin axis — together with the
air it flies through and its own mass and diameter. Loft and lie are nowhere in that list. They are
inputs to a different model, the one that maps *club delivery* to *launch conditions*, and that
model is not this one and is not being built here.

This is not a technicality. It means a club bent 2° strong flies exactly as its launch conditions
say it flies, and the flight model neither knows nor needs to know that the bend happened. It also
means **M12's specification work does not unlock this milestone**, and the correction to the bag
entry that M15 P1 makes — the 7 iron is recorded as a Titleist T150 at 32° and the club is a T250 —
changes nothing about any simulated flight. The two land in the same milestone and are not cause and
effect; §Decision 3 gives loft the one narrow job it does have here.

### 2. Spin is missing on eleven of the thirteen shots on disk

`data/processed/shots/` holds thirteen `.shot.json` artifacts. Two of them — both from 2026-08-10,
both reference photographs rather than bay sessions — carry `spin_rate` and `spin_axis`. The other
eleven, which are the whole of the real 2026-08-23 session, carry neither, and each one records why
in its own provenance: `Spin: no value text under the label`. The OCR is not failing. The tile is on
the screen and the value beneath it is not, which is the same class of finding as the `Bounce &
Roll` tile that `ROADMAP.md`'s Milestone 3 checklist already has an open item for.

So the blocker on ball flight is **spin, and not loft**. That is a narrower blocker than the M12
sentence assumed when it wrote that the flight model *"sits behind `spin_axis`"*, and it is narrower
in a way that turns out to matter twice: §Decision 4 recovers the rate, and §Decision 5 recovers the
sign of the axis from a tile nobody thought to read for it.

It also leaves the milestone in an unusually good position for something being built from nothing.
Two shots with a full launch-condition set **and** the simulator's own carry printed beside them are
a validation set — small, but real, and independent of anything this repo computes:

| shot | ball speed | launch | spin | axis | carry, per HD Golf |
|---|---|---|---|---|---|
| `2026-08-10-1` | 90.7 mph | 20.9° | 5991 rpm | +2.5° | 125.6 yd |
| `2026-08-10-2` | 90.5 mph | 23.5° | 8100 rpm | +9.3° | 121.0 yd |

Neither is club-tagged, which is fine — club is irrelevant to flight — but it does mean the loft
prior of §Decision 3 cannot be exercised on the only two shots that can check the physics.

### Why ADR-021's negative result does not close this

[ADR-021](021-caddieset-paired-reference-data.md) ran a study and got a negative: face-on pose does
not predict ball flight, at a carry R² of **−0.205**. `ROADMAP.md` records the conclusion as *"the
club sets the ball and a face-on camera pointed at a body does not see the club"*, and that
conclusion stands unqualified.

It is a different claim from this one. ADR-021 closed **body → ball**. This is **launch conditions →
flight**: the club has already done its work, the ball is in the air, and what remains is Newtonian
mechanics on a sphere whose initial conditions were measured by an instrument. The reason ADR-021
failed is precisely the reason this succeeds — the club sets the ball, and the launch monitor
reports the result of the club having set it. Anything that reads ADR-021 as evidence against a
flight integrator has read it as being about ball flight when it is about pose.

### What the repo has already said it is waiting for

Two places in the code describe this model in the negative, as an absence:

- `analysis/shot_measure.py:26-32` records ball speed and launch angle as *"fitting inputs… for a
  model that does not exist yet"*, and notes that the bay does not print them twice — *"the screen
  clears for the next shot, so a launch angle nobody recorded is gone."*
- The same module's refusal of a launch-angle target reads: *"optimal launch is per club, per ball
  speed and per spin rate, so a single number for it would be wrong for every club in the bag. Same
  missing model as ball speed and the same deferral."*

This is that model. The measure-now-judge-later ordering M6.5 established is what makes the
milestone possible at all, and the two shots above are what it bought.

### The name

`trajectory` is taken twice in this package and both times it means the **swing** as a path through
time — `analysis/trajectory.py` builds the landmark feature vector, and
`analysis/benchmarks/trajectory.py` projects it onto the fitted tour basis. A third meaning of the
word in one package would be a real cost to every future reader. Ball flight is **`flight`**
throughout: module, artifact, metric prefix, CLI, route.

## Decision

### 1. The flight is integrated from launch conditions, and the club is not among them

`analysis/flight.py` integrates a golf ball from its launch conditions under gravity, quadratic
drag, and the Magnus force, with the drag and lift coefficients read against **spin ratio**
`S = ωR/v` rather than against speed. Spin decays exponentially. The integrator is RK4 at a fixed
step, and it terminates on the `y = 0` crossing by interpolating within the final step rather than
by taking the last whole one — a ball is moving fast enough at landing that a whole-step
termination is a visible error in the carry, and it is the kind of error that would be absorbed
silently into a tolerance.

Its inputs are ball speed, launch angle, launch direction, spin rate, spin axis, and an atmosphere.
**Loft is not an input**, per §Context 1.

It lives in `analysis/`, which means it is stdlib and `contracts` only, per
[ADR-008](008-project-structure.md) and `docs/CODE_STANDARDS.md` R2. A physics integrator is the
single most likely module in this repo to attract a numpy rewrite, so it gets its own subprocess pin
in `tests/api/test_pipeline_imports.py` alongside the existing ones rather than relying on the
pipeline's.

### 2. The aerodynamic constants ship as committed data — ADR-022 applied to a lookup

`analysis/benchmarks/flight_model_v1.json` holds the `Cd`/`Cl` tables, the ball's mass and diameter,
the spin-decay constant and a default atmosphere, with a `dataset` block carrying the citations in
the same shape `ranges.json`, `golfdb_v1.json` and `joint_model_v1.json` already use.
`analysis/benchmarks/flight_model.py` evaluates it, following `joint.py`'s convention exactly:
a `_MODEL_FILE` constant, an `lru_cache`d loader over `importlib.resources`, validation into pydantic
models, and two public accessors — one for the provenance and one for the model.

**Nothing is fitted here**, and that is the difference from [ADR-022](022-learned-artifacts-as-committed-data.md)'s
own instances. These are published wind-tunnel measurements of a golf ball, not parameters derived
from this repo's corpus. So there is no `scripts/` fitting stage under the `research` extra, and none
should be written; the precedent is `clubs/catalogue.py`, which describes itself as *"ADR-022's
division applied to a lookup"* — data committed as JSON with provenance per row, and what runs inside
the package is a dictionary read. ADR-022's §1 table still holds with its first column struck out.

The artifact ships **before** the integrator, in an earlier phase. An integrator written first would
have to carry a constant to run at all, and a constant that works is a constant nobody removes.

### 3. Where spin is absent it is solved from the printed carry — and what that number is not

For the eleven shots with no spin, the integrator is run backwards: given ball speed, launch angle
and the carry the simulator printed, find the spin rate that reproduces that carry.

**This is stated on the record rather than left in a docstring, because it is the weakest claim in
the milestone.** HD Golf's printed carry is itself the output of HD Golf's own flight model. Solving
against it fits our model to *their* model, not to reality. A spin recovered this way is *"the spin
our integrator needs in order to agree with the simulator"* and it is **not a measurement of spin**.
It is reported under its own name, from its own source, labelled as inferred everywhere it is shown,
and it must never be described to a golfer as their spin rate.

What it is good for is the thing that was asked for: drawing the path. A flight whose carry matches
the measured carry and whose shape is physically consistent is a defensible picture of the shot even
when one of its inputs was solved rather than read. What it is not good for is any claim about spin
itself — gapping, fitting, or a spin-rate trend over sessions — and none of those is in scope.

**Loft's one job.** §Decision 4 explains why the solve can return two answers. Loft is what chooses
between them: a 30.5° iron at ~90 mph of ball speed sits on the high-spin branch, and the book loft
M12 records is enough to say so. That is the whole of loft's involvement in ball flight, and it is
an involvement in the *inference* rather than in the *flight* — which is why a club bent 2° strong
changes the inferred spin and does not change how the ball flies.

Where there is no loft on record, there is no branch rule, and the answer is `None` with a reason.
Never a default.

### 4. Carry is unimodal in spin, so the solve is allowed to refuse

Carry does not increase monotonically with spin. It rises to a peak and then falls, because past a
point the extra lift costs more in drag and in a steeper descent than it returns in hang time. A
solver written on the assumption of monotonicity will bisect happily and return a confident wrong
answer, and this is recorded here because that assumption is the natural one to make and nothing
downstream would catch it.

Three cases, and only one produces a number:

1. **The measured carry sits below the peak** — two solutions exist, one on each branch. §Decision 3's
   loft prior picks one; with no loft, neither is picked.
2. **The measured carry exceeds the achievable peak** — no spin reproduces it. The printed numbers
   are inconsistent with the model, which means an OCR misread, a mishit, or a real disagreement
   with HD Golf. Return `None` with a reason. **This is a finding and not a failure**, and it is one
   of the more useful things the milestone produces: it is an independent consistency check on
   numbers that are currently read off a photograph and trusted.
3. **The measured carry sits at the peak** — one solution, on a knife edge, and it gets its own test.

This is [ADR-010](010-benchmark-ranges.md) §2 applied to a quantity that is inferred rather than
scored: no number beats a wrong one.

### 5. The spin-axis sign ADR-014 could not resolve is printed on the same screen, in words

[ADR-014](014-screen-capture-shot-ingestion.md)'s addendum records that `spin_axis` was stored
sign-inverted and that the screen prints the tile *"with no direction word"* — and `ROADMAP.md`'s M12
section names exactly this as the blocker the flight model sits behind. `shot_measure.py` excludes
`spin_axis` for the same reason and routes the question to face-to-path instead.

The direction word is on the screen. It is in the **`Shot Type`** tile, which prints `FADE`, `DRAW`,
`SLIGHT FADE` and `HARD FADE` in English, which every one of the thirteen stored shots carries, and
which `analysis/shot_measure.py:207`'s `normalize_shot_shape` already parses into a `TargetShape`.
The blocker was that no *numeric* tile carries the sign; the resolution is that a *text* tile does.

So the axis resolves in this order: the measured `spin_axis` where one exists; else
`measure_face_to_path`, which has agreed with the simulator's own shape verdict on every shot it has
seen; else the curve is refused, the flight is simulated in-plane, and `landing_offline` goes
unscored with its reason. Where an inferred sign disagrees with `Shot Type`, that is a **warning and
never a silent overwrite** — a disagreement is information about the parse, and resolving it quietly
is how ADR-014's original inversion survived as long as it did.

Every refusal above reuses `contracts/unscored.py`. **`refilming_helps` is `false` for all of them**:
a spin field the screen never printed is not a camera problem, and a golfer must never be told to
re-film a swing because a launch monitor withheld a number.

### 6. The simulated quantities are named apart from the measured ones

`flight_carry_yds`, `flight_apex_yds`, `flight_descent_angle_deg`, `flight_time_s`,
`flight_landing_offline_yds` and `flight_spin_rpm`, under a new measurement source `model:flight_v1`
— the fourth, after `pose:face_on`, `launch_monitor:*` and `population:golfdb`.

Distinct names rather than a variant flag on the existing ones, for the reason ADR-022's 2026-08-17a
addendum gave the down-the-line trajectory model its own names: `baseline.py::pooled_samples` groups
by name, so a predicted carry sharing a name with the measured `carry_distance_yds` would pool the
two into one distribution and produce a personal mean over a mixture of a measurement and a model
output. There is no flag that prevents that, because nothing downstream reads one.

The new source prefix has a consequence that is easy to miss and expensive to find:
`storage/corpus.py::CorpusSwing.artifact_key` is the single definition of the corpus dedupe rule,
and an unrecognised prefix falls through to `swing:{ref}` while printing *"Unrecognised measurement
sources"* — a warning that has already fired unread once in this repo, for `population:golfdb`.
Registering the prefix there is a step of its own, and it lands before the `ANALYSIS_VERSION` bump so
that the re-analysis is what proves it.

These are measured-and-not-judged, exactly as carry is: **no band, no `ranges.json` row, no
`CHECKPOINT_REGISTRY` entry**. The panel does not change size and `overall_score` does not change
meaning.

## Consequences

**A number that was recorded for nothing now has a reader.** Ball speed and launch angle have been
stored since M9 against a model nobody had scheduled. Every shot captured since then becomes
retrospectively more useful, which is the payoff of the ordering M6.5 chose and the first time it has
been collected.

**The eleven spin-less shots become partially usable rather than unusable** — with an inference
whose limits are stated, and which is refused outright where the arithmetic does not support it.

**A consistency check on the OCR arrives as a by-product.** §Decision 4's second case flags a printed
carry that no spin can produce. Nothing else in the repo can currently say that a set of printed
numbers is internally impossible; `launch_monitor/screen/validate.py` checks that the tiles agree
with each other, which is a weaker claim than that they agree with physics.

**`ANALYSIS_VERSION` moves 14 → 15 and the corpus is re-analysed.** New measurements enter
`analysis.json`, so the artifacts on disk are stale until `reanalyze.py --all` has run. No existing
score changes — nothing here touches the checkpoint panel — so this is a re-run for completeness of
measurement and not a re-scoring.

**The MCP tool count goes ten → eleven**, which moves a phrase pinned by `tests/test_docs_truth.py`
across `ROADMAP.md`, `docs/ARCHITECTURE.md`, `scripts/ask_swing.py` and every ADR that carries it.

**A simulated carry will sometimes disagree with the measured one, visibly, on the same screen.**
That is intended. The viewer draws both, and a disagreement is a fact about the two models rather
than a defect in either — but it does mean the page has to be built so that a reader cannot mistake
the simulated landing point for a measurement, which is what M15's viewer-honesty phase is for.

## Alternatives considered

**Refuse without measured spin, and simulate only the two shots that have it.** The purest reading of
ADR-010 §2, and it was the first option costed. Rejected because it produces nothing at all from the
only real bay session on disk, and because the refusal is not actually the honest one: the carry *is*
measured, and declining to use it is discarding information rather than declining to invent any.
§Decision 3's labelling is what makes the weaker claim safe to make, and §Decision 4 is what stops it
being made when it cannot be supported. **If the labelling ever erodes, this is the alternative to
fall back to.**

**Estimate spin from club delivery — club speed, loft and attack angle.** The textbook route, and it
is blocked twice over by this bay. `shot_measure.py` already excludes `club_head_speed` on the
evidence that the simulator prints smash factors of 0.89–1.06, which no real strike produces, and
records the specific pair of shots that point at the club-speed reading rather than the ball-speed
one. And HD Golf prints no attack angle at all. Building a spin model on two numbers this repo has
already declined to record would be a wrong number with a provenance string attached.

**Fit the aerodynamic coefficients to this repo's own shots.** Thirteen shots, of which two have
spin. There is no fit there, and ADR-022 §2's argument for interpretable estimators when `n` is small
applies with more force at `n = 2` than anywhere it has been applied before.

**Name the module `trajectory.py`.** Rejected on §Context's grounds — the word already has one
meaning in this package and it is a different one.

**Render the path with a plotting library or a 3D framework.** Rejected to keep the base install
small and the host offline-first ([ADR-016](016-local-first-host-and-phone-upload-topology.md)).
The four existing static pages are vanilla JS with no bundler and no CDN, and a canvas projection of
a polyline is small enough that matching them costs less than the dependency would.

## Deferred, by choice

**The impact model — club delivery to launch conditions.** The other half of the physics, and the
half that would let loft and lie mean something predictive. It needs an attack angle and a
trustworthy club speed, and this bay currently supplies neither.

**Gapping, club fitting and swing efficiency.** ADR-024 and ADR-026 defer these and they stay
deferred. A flight model is a prerequisite for the first two and does not on its own deliver either;
in particular, a per-club carry expectation needs a population this repo does not have, which is
ADR-010 §4's outstanding TrackMan/Arccos acquisition.

**Wind, and any lie other than a flat one.** The integrator takes an atmosphere because air density
is nearly free once the loop exists and is the difference between a bay in a garage and a course at
altitude. Wind is a vector field and a different conversation, and every shot on disk was hit
indoors.

**Roll-out.** `total_distance` minus carry is the ground rather than the swing, as
`analysis/shot_measure.py` already argues in refusing it a target. Simulating it would mean modelling
a surface nobody has described.

**A spin measurement.** The real repair for §Context 2 is the open Milestone 3 item — tuning the
screen profile against the bay's actual tile layout, which is already known to differ from the two
reference photographs. If that lands and spin starts arriving, §Decision 3's inference becomes a
fallback rather than the main path, and the two can be compared: **the inferred spin on a shot that
later reports a real one is the honest test of this ADR's weakest claim**, and it should be run the
day such a shot exists.
