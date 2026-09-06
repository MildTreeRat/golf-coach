# ADR-027: Ball-flight simulation — the model the launch angle was recorded for

## Status
**Accepted** 2026-09-04, and **built** — P1 through P18 have landed, so this document is no longer
ahead of its code. The phases are M15, whose section in [ROADMAP.md](../../ROADMAP.md) this
document's P0 wrote alongside it. Read the **fifteen addenda** before any claim in the body: they
are where reality corrected this document, and on the points they touch they are the account that
holds — as are the counts of what is on disk in §Context:

- **2026-09-05 (P2)** corrects §Decision 2 and weakens §Decision 4 — the published coefficient table
  is a driver's and every shot on disk is an iron's.
- **2026-09-05b (P3)** corrects that addendum, rewrites §Decision 4 from three cases to five, and
  reports the *Deferred, by choice* section's own "honest test" as run and failed. **Read it before
  P8, and before trusting §Decision 3.**
- **2026-09-05c (P4)** is the gate, and it records what the percentage hides — the model ranks the
  two validation shots the wrong way round. It corrects §Decision 4 once more (there are *two*
  plateaus, and the lower is the floor). **Its ±2.55% is superseded by the addendum below**; every
  other claim in it stands.
- **2026-09-05d (P5)** is the third dimension, and it re-flew the gate on the launch directions and
  spin axes that were on disk the whole time: **±2.59%**, from a *smaller* mean absolute error. The
  recorded spin axis closes a tenth of the inverted ordering, so P4's reading of the percentage as
  "the size of the missing spin effect" is an over-attribution. It also finds §Decision 5's first
  branch already live. **Read it before quoting the agreement anywhere, and before P8 or P9.**
- **2026-09-05e (P6)** is the altitude what-if, and it takes an option off the table: the inverted
  ordering **widens** with altitude, nearly quadrupling from sea level to 4369 m, so no atmosphere
  is the free parameter that fixes the gate. It also records that the gate is a *sea-level* gate —
  flown at Denver one shot agrees to 0.3% and the other leaves the tolerance.
- **2026-09-05f (P8)** is §Decision 3 as code, and it is a count rather than a physics finding:
  the solve names a spin for **4 of the 11** spin-less shots and refuses 7. It also empties the OCR
  consistency check §Consequences claimed — the one printed carry that survived the margin is
  inside the range once the floor is the *low* plateau — and qualifies 2026-09-05c's blanket
  refusal of the unique band. **Read it before P9.**
- **2026-09-05g (P9)** is the loft prior and the axis resolution, and it corrects §Decision 5: the
  face-to-path fallback resolves the **sign** of the curve and not the magnitude of the axis, so
  the second branch collapses into the third and the flight is drawn planar. It also records that
  the prior needs no loft-to-spin table — the measured peak sits at driver spin — and that the
  whole inference **refuses all thirteen shots on disk**, because none of them carries a club.
  **That last count is corrected by the addendum below.**
- **2026-09-05h (P10)** reads the corpus, and corrects 2026-09-05g's headline: the club is on the
  **swing**, not on the shot, and eleven of the thirteen shots are attached to a swing that carries
  one. The inference names **one spin on the corpus** and refuses ten; two of those refusals are a
  3 wood nobody has declared in the bag, which is a repair on the bag page rather than a bay
  session. It also checks P4's hand-typed gate constant against disk for the first time (it
  matches), and finds that a **planar flight's landing offline is `start_line_offline_yds` by a
  longer route** — which §Decision 6 has to obey in P11.
- **2026-09-05i (P11)** is §Decision 6 as code, and it corrects that section twice: `flight_spin_rpm`
  records only a **solved** spin, because a printed one is the launch monitor's reading and not a
  model output; and the planar-offline identity 2026-09-05h measured is **not structural** — it
  holds where the spin was solved from the carry and breaks by 0.29 yd where the spin was measured,
  which argues for withholding rather than against it. It also found that two `feedback` consumers
  were about to tell a golfer a flight measurement had been excluded from a score it was never in.
  **Read it before any surface quotes a `flight_*` number.**
- **2026-09-06 (P12)** corrects §Decision 6's account of what registering the prefix buys: the
  fallback was under-refusing, not merely over-counting, and `population:golfdb` stays unregistered
  on purpose.
- **2026-09-06b (P13)** is the corpus on the new engine — every score byte-identical, **five**
  flights and ten refused ones on disk — and it corrects §Decision 6 once more: the re-analysis
  this section says will *prove* the prefix registration **cannot**, because no two distinct
  swings share a shot photo. It also records that the bump swept in four M14 measurements no
  stored artifact carried, which is a finding about when a version is owed rather than about
  ball flight.
- **2026-09-06c (P14)** is the HTTP route, and it corrects §Consequences: a flight's inputs are
  not all on the swing. The loft and the handedness live in **editable** artifacts, so a route
  that re-flies and an `analysis.json` that stored the answer can disagree about one shot, with
  nothing on disk able to see that they do.
- **2026-09-06d (P15)** is the page, and it is a drawing finding rather than a physics one: the
  plan view's offline axis has to be **stretched by the smallest factor that makes the curve
  readable, not the largest that fits**, because every flight on this corpus drifts a third to a
  half of its own apex and "largest that fits" drew the lateral miss taller than the height of
  the shot. It also found this Status block itself a phase behind, which is why
  `tests/test_docs_truth.py` now pins an ADR's self-stated addendum count.
- **2026-09-06e (P16)** is the viewer-honesty pass, and it corrects the addendum above: setting a
  simulated number beside a printed one is **not** a rendering decision, because *which* printed
  number and *whether the gap is an error* are registry questions. On this corpus neither of the
  two pairs is a check — one is the spin solve's own input read back, the other is where the ball
  started against where it finished — and that second gap decomposes rather than being "the
  curve". It also found `sign_disagrees` had never been rendered to a golfer at all.
- **2026-09-06f (P17)** is the eleventh MCP tool, and it found the surface that had been quietest:
  the six numbers reached a **coaching model** as bare floats under a field description calling
  them measured, because `mcp/query.py` flattens `measurements` to name -> value and §Decision 6's
  provenance lives in the two fields it drops. It also found that M15 P10's join runs the wrong
  way for this question — three swings on this corpus share one shot photo — and it is the first
  surface anywhere that can **see** the 2026-09-06c seam rather than only re-fly past it.

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
`contracts/career.py::CorpusSwing.artifact_key` is the single definition of the corpus dedupe rule,
and an unrecognised prefix falls through to `swing:{ref}` while printing *"Unrecognised measurement
sources"* — a warning that has already fired unread once in this repo, for `population:golfdb`.
Registering the prefix there is a step of its own, and it lands before the `ANALYSIS_VERSION` bump so
that the re-analysis is what proves it. ⚠️ P12 landed it and corrected two things this paragraph got
wrong: the path (`contracts/`, not `storage/`, fixed above) and the *reason it matters* — the
fallback's cost is not the over-count this paragraph implies but a missing refusal. See the
2026-09-06 addendum. ⚠️ **P13 ran that re-analysis, and it proves nothing about the key**: no two
distinct swings on this corpus share a shot photo, so `model:{photo}` and the `swing:{ref}` fallback
partition it identically and no count would move if the registration were reverted. The sentence
above is a plan that the corpus cannot carry out; see the 2026-09-06b addendum.

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

## Addendum (2026-09-05, M15 P2): the published table is a driver's, and every shot on disk is an iron's

§Decision 2 said the coefficients would ship as committed data with citations, and they have —
`analysis/benchmarks/flight_model_v1.json` and `flight_model.py`. What it did not anticipate is
what sourcing them turned up, and the finding changes how P4 must be read.

**The coefficient data is a driver's flight.** The best public source is Table 3 of
US 7,156,757 B2 (Acushnet, 2007): eight measured `Cl`/`Cd` points in two seam orientations, for a
1.68 in / 1.61 oz ball. Its eight rows are not an independent grid — each spin ratio is paired
with one Reynolds number, running 0.085/230,000 to **0.284/69,000**, because they are points along
a driver's trajectory where the ball slows as its spin ratio rises. Reading against spin ratio
alone, which §Decision 1 chose, therefore imports that pairing. The Reynolds column is stored
anyway so a later two-dimensional read does not have to re-source the table.

**Both validation shots start above the table's last row and climb from there.** A 7 iron at
90.7 mph and 5991 rpm launches at `S = 0.330`; at 90.5 mph and 8100 rpm, `S = 0.447`. The ceiling
is 0.284. And `S = wR/v` *rises* through a flight, because the ball sheds speed faster than 4%/s
sheds spin. So neither reference shot ever reads an interpolated row: both fly the entire way on
the clamped end row, and so will every iron in this repo's corpus.

**Clamping is the choice, and it is still an extrapolation.** Smits and Smith (1994) measured out
to `S = 1.4` and report lift saturating rather than continuing to rise, so holding the last row is
closer to the physics than extending its slope and can never return an unbounded coefficient.
`AeroCoefficients.clamped` reports it every time, and
`tests/analysis/test_flight_model.py::test_both_validation_shots_launch_above_the_published_table`
fails if the table is ever extended to cover iron spin ratios — at which point this addendum is
what has to be rewritten.

**What it does to §Decision 3 and §Decision 4, measured rather than argued.** A throwaway RK4 over
exactly these constants returned **122.4 yd against HD Golf's 125.6** and **124.1 yd against its
121.0** — about 2.5% out on each, and with *opposite* signs. That is the signature of a flattened
coefficient: with `Cl` and `Cd` pinned at one row, the only route spin still has into carry is the
Magnus term's own `w`, so the model cannot fully tell 5991 rpm from 8100 rpm. Two consequences
follow and both are for later phases to carry rather than to discover:

- **P4 must not report agreement without reporting the clamp.** A tolerance pinned at ~2.5% would
  otherwise read as a validation of a table that was never consulted.
- **§Decision 4's unimodality is weakened, not removed.** Carry still rises and falls with spin
  through the Magnus term, so the two-branch structure and the refusal survive; but the peak is
  flatter than it would be with live coefficients, which makes the solve less well-conditioned near
  it and makes case 3 — the knife-edge — wider than the ADR imagined. The loft prior of §Decision 3
  is doing more work than it looks like it is.

**What would actually fix it** is coefficient data at iron spin ratios, which nobody appears to
have published in the open. This is not the milestone's blocker and should not become one: the
agreement above is good enough to draw a defensible path, which is what §Decision 3 says the model
is for. It is recorded here so that the day such data arrives, the reader knows exactly which
number it improves.

## Addendum (2026-09-05b, M15 P3): above the clamp, spin does not reach the flight at all — and the spin solve fails the only test it can be given

The integrator landed (`analysis/flight.py`) and reproduces P2's scratch numbers to the digit. Running
it corrected the first addendum, rewrote §Decision 4, and let the *Deferred, by choice* section's
"honest test" be run a milestone earlier than that section expected — because the shot it was waiting
for is already on disk. **Read this before P8, and read it before trusting §Decision 3.**
§Decision 1, §Decision 2 and §Decision 5 are untouched, and so is the plan for P4.

### The correction: there is no free ω

The 2026-09-05 addendum said that with `Cl` and `Cd` pinned at one row, *"the only route spin still
has into carry is the Magnus term's own ω"*. There is no such route. Lift is written with a
coefficient — `½ρA·Cl·v²` — so `ω` reaches the flight only through `S = ωR/v`, and `S` only through
`Cl` and `Cd`. Hold those two and spin has left the problem.

The measurement: at 90.6 mph and 22°, carry is **bit-identical at 5,200 rpm and at 30,000 rpm** —
123.2108933755 yd at every value in between. Swapping the two reference shots' spin rates between them
changes neither flight by a last digit; they differ from each other because their *launch angles*
differ. `tests/analysis/test_flight.py::test_above_the_clamp_spin_stops_reaching_the_flight_at_all`
pins it, and its sibling pins the other half — below the threshold, spin still moves the carry, so the
plateau is the data's and not the physics'.

`S` first reaches the table's ceiling of 0.284 at `ω = 0.284·v/R`: about 5,150 rpm at 90.6 mph,
scaling with ball speed from 4,484 rpm at the slowest shot on disk to 6,524 rpm at the fastest. Above
that the flight is clamped from its first step.

### §Decision 4 had three cases; there are five

Carry against spin rises to a peak, falls, and then **floors on a plateau** rather than continuing to
fall. The achievable range is a closed window `[floor, peak]`:

1. **Inside the window** — two solutions, one each side of the peak; the loft prior picks one. *(Unchanged, but see the cap.)*
2. **Above the peak** — no spin reproduces it. *(Unchanged.)*
3. **At the peak** — one solution, on a knife edge. *(Unchanged.)*
4. **On the plateau** — every spin above the threshold reproduces it. Not two answers and not none: infinitely many. **New.**
5. **Below the floor** — no spin reproduces it, for the opposite reason to case 2. The ADR assumed carry falls without bound past the peak. It does not. **New.**

Across the eleven spin-less shots the peak sits at **2,400–3,500 rpm** (4,800 on the one shallow
outlier) and the window is **1.3–10.6 yd wide**. The solve is not merely two-branched; it is badly
conditioned across nearly all of its range. And because carry stops responding above the threshold,
**every inferred spin is capped there** — at 4,500–6,500 rpm for the ball speeds on disk.

### The honest test was available all along, and §Decision 3 fails it

*Deferred, by choice* says: *"the inferred spin on a shot that later reports a real one is the honest
test of this ADR's weakest claim, and it should be run the day such a shot exists."* Two such shots
have been on disk since 2026-08-10. Run against them:

| shot | measured spin | plateau threshold | what the solve does |
|---|---|---|---|
| `2026-08-10-1` | 5,991 rpm | 5,154 rpm | returns **3,201 rpm** — 47% low |
| `2026-08-10-2` | 8,100 rpm | 5,143 rpm | **refuses**: HD's 121.0 yd is below the model's 124.1 yd floor |

Both true spins sit above their own plateau, which is precisely where carry carries no information
about spin. The first shot's carry does fall inside the window, so the solver returns a number
confidently — and it is wrong by nearly half. **One refusal and one 47% error is the whole of the
evidence, and it is all the evidence there is.**

This does not make §Decision 3 dishonest; it makes it *smaller* than it read. Its own words — *"the
spin our integrator needs in order to agree with the simulator"*, *"not a measurement of spin"*,
*"never described to a golfer as their spin rate"* — turn out to be the literal and complete truth
rather than a caution attached to a mostly-good number. What survives is what §Decision 3 said the
value was for: **drawing a path whose carry matches the measured carry.** That still works, and it is
what the milestone was asked for.

**P9 must report the cap and this error beside every inferred spin**, and P16's viewer-honesty phase
now has a specific thing to be honest about. A `flight_spin_rpm` shown without them would read as a
low-spin diagnosis of a swing, which is the one reading §Decision 3 forbids.

### The OCR consistency check does not survive the model's own error

§Consequences claimed a by-product: a printed carry no spin can produce is an independent check on
numbers read off a photograph. Measured, **seven of the eleven printed carries sit above the
achievable peak — by +0.56% to +3.45%, median +0.97%**. The model's own disagreement with HD Golf is
~2.5% on the two shots where it can be checked. Nearly every one of those seven is inside the model's
own error, so refusing them would report the model's bias as a fault in the OCR.

One survives the margin, and it is worth looking at: 83.8 mph at **3.4°** with a printed carry of
33.6 yd, against a floor of 44.8 — **25% below** anything the model can fly. A near-horizontal launch
is exactly where a printed "carry" is most likely to be measuring something else.

### What would fix it

Coefficient data at iron spin ratios, which nobody appears to have published in the open — the same
single fix the first addendum named. Everything above is a consequence of its absence and not of the
integrator, whose own numerical error is four orders of magnitude smaller: the carry is identical to
1e-4 yd at every step from 0.02 s to 0.0005 s.

**If the labelling around §Decision 3 ever erodes, §Alternatives' first entry — refuse without
measured spin — is now backed by a measurement rather than by an argument.**

## Addendum (2026-09-05c, M15 P4): the gate passes at ±2.55%, and the percentage hides that the model ranks the two shots backwards

The gate ran. **Both validation shots land inside 2.55% of HD Golf's own carry**, and that figure is
now `analysis/flight.py::GATE_AGREEMENT_FRACTION` with `tests/analysis/test_flight_validation.py`
pinning it. §Decision 1, §Decision 2, §Decision 3 and §Decision 5 are untouched; §Decision 4 takes a
second correction, smaller than the 2026-09-05b one and in the same direction.

| shot | launch | measured spin | HD Golf | this model | error |
|---|---|---|---|---|---|
| `2026-08-10-1` | 90.7 mph, 20.9° | 5,991 rpm | 125.6 yd | **122.4110 yd** | **−3.189 yd, −2.539%** |
| `2026-08-10-2` | 90.5 mph, 23.5° | 8,100 rpm | 121.0 yd | **124.0747 yd** | **+3.075 yd, +2.541%** |

`ROADMAP.md` required the tolerance to be measured rather than chosen, and *"not a number to widen
until it passes"*. It is pinned at 0.0255 — the achieved worst case of 2.5411% rounded up in the
fourth decimal — and a test asserts the pin is that tight, so the constant has no room in it for a
regression to sit unnoticed.

### The three things that have to be said beside the number

**The flights never read the table.** `FlightResult.fully_clamped` is true on both, as the
2026-09-05 addendum predicted: every coefficient in both flights is the held end row of a driver's
table. ±2.55% is the agreement between HD Golf and a *constant* `Cl`/`Cd` pair. It is not evidence
about the patent's measurements, which were not consulted.

**The disagreement is not the printed precision.** HD Golf prints one decimal place, so ball speed
and launch angle each carry ±0.05 of rounding. Sweeping both across their full rounding interval
moves the carry by **0.31 yd and 0.26 yd** — an order of magnitude below the ~3.1 yd errors. The gap
is the model's, and it is not an artifact of reading numbers off a photograph.

**The errors cancel, and that is the least honest way to state this.** −3.189 and +3.075 average to
**−0.057 yd**, about 0.05%. A bias correction fitted to that would be fitting noise at `n = 2`; two
errors of nearly equal size and opposite sign are the signature of a model that cannot *separate*
the two shots, not of one that is unbiased. There is a test whose only job is to argue against that
correction before someone writes it.

### The finding: the gate passes per-shot while the ordering inverts

HD Golf has `2026-08-10-1` — the lower launch angle, the lower spin — flying **4.6 yd further** than
`2026-08-10-2`. This model has it flying **1.7 yd shorter**. The sign of the difference between the
only two shots that can be checked is wrong, and no per-shot tolerance can see it, because each shot
passes on its own.

The mechanism is the 2026-09-05b addendum's, restated as a consequence. Above the clamp the only
difference the model can see between these two shots is 2.6° of launch angle and 0.2 mph of ball
speed; the 2,109 rpm separating them reaches the flight nowhere at all. Reproducing HD's ordering
needs 6.26 yd of separation out of a term that has no route into the answer.

**This is what ±2.55% actually means, and P9 must carry it rather than the percentage alone.** The
model is not 2.5% away from HD Golf in the way a calibration error is 2.5% away. It is missing the
whole of the spin effect, and 2.5% is the size of that effect at these launch conditions.

### What the gate says about P8, measured rather than assumed

Sweeping spin at each shot's own launch conditions bounds the set of carries the model can produce
there:

| shot | achievable carry | HD Golf's carry | the solve |
|---|---|---|---|
| `2026-08-10-1` | 118.02 – 127.45 yd | 125.6 yd | **inside** — reachable at 3,201 rpm, against a recorded 5,991 |
| `2026-08-10-2` | 123.01 – 130.65 yd | 121.0 yd | **below the floor by 2.0 yd** — no spin reproduces it |

One fabricated answer and one refusal, on the two shots where the true spin is known. That is the
whole of P8's evidence base and it agrees with the 2026-09-05b addendum, with one number corrected:
the floor that refuses `2026-08-10-2` is **123.0 yd, not 124.1**, because the true floor is not
where that addendum put it.

### §Decision 4 again: there are two plateaus, and the lower one is the floor

The 2026-09-05b addendum found the high-spin plateau — above `S = 0.284` the last row is held and
carry stops responding — and treated it as the bottom of the achievable range. It is not. **The
clamp has a low end too**, and it behaves identically: below `S = 0.085` the *first* row is held
(`Cl = 0.141`, `Cd = 0.218`), so spin has no route into the flight there either. At 800 rpm the
whole flight runs `S = 0.044` to `0.060` and never enters the table; carry is constant at 118.02 yd
for every launch spin from 0 up to about 1,130 rpm.

So carry against spin is: a **low plateau**, a rise, the peak, a fall, and a **high plateau** — and
because the fall stops at the high plateau rather than continuing, the two plateau values cut the
range into bands with different solution counts. For `2026-08-10-1`'s launch conditions:

| target carry | solutions |
|---|---|
| below 118.02 yd | none |
| exactly 118.02 yd | infinitely many — the low plateau, 0 to ~1,130 rpm |
| 118.02 – 122.41 yd | **exactly one**, on the rising branch — the falling branch never reaches down here |
| exactly 122.41 yd | infinitely many — the high plateau, ~5,150 rpm up |
| 122.41 – 127.45 yd | two, one each side of the peak |
| exactly 127.45 yd | one, at the peak (2,550 rpm) |
| above 127.45 yd | none |

The unique-solution band is new and is not the good news it looks like: it sits between roughly
1,130 and 1,600 rpm, which is not a spin a 7 iron produces. A solve landing there is unique and
implausible at the same time, and P8 should treat a unique answer below the low plateau's shoulder
as a refusal rather than as its best case.

### Where the number lives, and where it deliberately does not

`GATE_AGREEMENT_FRACTION`, `VALIDATION_SHOTS` and `gate_comparisons()` are in `analysis/flight.py`.
`gate_comparisons()` re-flies the shots rather than returning stored results, so a re-sourced
coefficient table moves the gate instead of leaving a quoted number behind.

They are **not** in `flight_model_v1.json`, which was the first instinct and is wrong: that file
states of itself that every number in it is a published measurement and that nothing in it was
derived from this repo's corpus. A model output over two shots on disk is neither. Keeping one
number tidy is not worth making the artifact's central claim false.

## Addendum (2026-09-05d, M15 P5): the gate had been flying a planar approximation of a shot that curved

P5 gave the integrator its third dimension — launch direction, spin axis, curvature and
`landing_offline`, in a right-handed frame with `x` downrange, `y` up and `z` right. The physics
was the easy half and it changed nothing it should not have: every planar number P3 and P4 measured
survives to the last bit, because the lateral fields default to zero and a zero axis contributes
exactly zero rather than a rounding error.

Then it found that **both validation shots have a launch direction and a spin axis on disk**, and
had all along. `2026-08-10-1` started 5.3° left with a 2.5° axis; `2026-08-10-2` started 4.0° right
with a 9.3° axis. The gate had been flying neither.

### One of the two cannot move the carry, and that is a definition rather than luck

`FlightResult.carry_m` is the landing point projected onto the **launch azimuth**, not onto the
target line. Turning a whole flight about the vertical cannot change how far the ball flew, so
carry is *identical* across every launch direction on disk — bit-identical to a nanometre, the
residual being the sine and cosine round trip. `curvature_m` is the perpendicular coordinate in
that same frame and `landing_offline_m` is the deviation from the target line, so the three are one
landing point in two frames and the identity between them is pinned.

The alternative — carry along the target line, which is the more obvious reading of "carry" —
would have shortened it by `cos(direction)`: **0.43% on `2026-08-10-1` and 0.24% on
`2026-08-10-2`**, a sixth of the gate's own tolerance, and it would have entered §Decision 3's spin
solve as a bias shaped like the start line. The launch direction sets where the ball finished and
nothing else.

### The other one moves the gate, and the gate has been re-pinned

Lift is one vector. What it spends bending the ball it does not spend holding it up, so a tilted
axis costs carry — and the two shots have very different axes.

| shot | HD Golf | P4, planar | P5, as recorded | error, planar | error, as recorded |
|---|---|---|---|---|---|
| `2026-08-10-1` | 125.6 yd | 122.4110 yd | **122.3567 yd** | −2.539% | **−2.582%** |
| `2026-08-10-2` | 121.0 yd | 124.0747 yd | **123.3868 yd** | +2.541% | **+1.973%** |

`GATE_AGREEMENT_FRACTION` moves **0.0255 → 0.0259**, the achieved worst case rounded up in the
fourth decimal exactly as before, and `test_flight_validation.py` still pins that the tightness has
no slack in it.

**This is not the widening `ROADMAP.md` forbids.** That rule is about a tolerance stretched until a
failing model passes; here the *inputs* became complete, the mean absolute error **fell** from 3.13
to 2.82 yd, and the pin rose 0.0004 as an arithmetic consequence of one shot getting slightly worse
while the other got markedly better. The gate now compares against the shot HD Golf actually
measured. Flying a planar approximation of a shot that finished 6.6 yd right of its start line, and
reporting the agreement as the model's accuracy, was the quieter error.

### The finding: the spin axis closes a tenth of the inversion, so P4's attribution was too generous

The 2026-09-05c addendum's central result stands — **the model still ranks the two shots
backwards**. HD Golf has the lower-spin shot 4.6 yd further; this model has it 1.0 yd shorter.

But P4 read ±2.5% as *"the size of the missing spin effect"*, and that sentence cannot be repeated.
Flying the recorded axes narrows the model's gap from +1.664 yd to +1.030 yd, which is **0.63 yd of
the 6.26 yd inversion, a tenth of it, closed by a launch condition that is not the spin rate**. Nine
tenths remain and the conclusion survives; the attribution does not. **M15 P9 must report the
residual and not the whole.**

### P4's cancellation warning has become a measurement

P4 pinned a mean error of −0.057 yd as a warning: two near-equal errors of opposite sign at `n = 2`
are the signature of a model that cannot separate two shots, not of one that is unbiased, and there
was a test whose only job was to argue against a bias correction fitted to them.

The mean is now **−0.428 yd**, and the cancellation is gone. It was partly an artefact of flying
both shots planar. The test survives inverted — it now pins that the errors *do not* cancel — and
the argument against fitting a correction at `n = 2` is unchanged, but nobody can reach for the mean
as evidence that there is no bias to correct.

### §Decision 4's five cases survive, and its numbers move by less than a twentieth of a yard

The plateau structure is the clamp's and the clamp does not care about the axis, so on
`2026-08-10-1`'s full launch conditions the low plateau is **117.99 yd** (was 118.02) with its
shoulder at 1,129 rpm, the peak **127.40 yd** at 2,550 rpm (was 127.45), and the high plateau
**122.36 yd** from 5,154 rpm up (was 122.41). Every band in that addendum's table keeps its solution
count.

§Decision 3's honest test also survives, barely moved: the spin that reproduces
`2026-08-10-1`'s printed carry on the falling branch is **3,185 rpm against a recorded 5,991**, and
`2026-08-10-2` is still refused outright. The third dimension changes the verdict on neither shot,
which is worth knowing before P8 hopes it might.

### §Decision 5's first branch was already live, and one sentence in the repo has not noticed

§Decision 5 orders the axis resolution as *measured `spin_axis` first, else face-to-path, else
refuse*, on the premise from [ADR-014](014-screen-capture-shot-ingestion.md)'s addendum that the
stored axis is sign-inverted. **It is not, and has not been since `screen/profiles.json` gained
`printed_sign: -1` for that tile.** HD Golf prints `-9.3` and the parser stores `+9.3`; both shots
on disk now carry a positive axis beside a `Shot Type` of `FADE`, which agrees with
`contracts/shot.py`'s `+ = fade`, and `screen/validate.py`'s cross-check fires on neither.

`analysis/shot_measure.py`'s exclusion note still says *"both fades on disk carry a negative
value"*. That describes the **printed** sign, not the stored one, and reads as a statement about
`ShotData`. It is left alone here because the axis is P9's field and P9 is where §Decision 5's
ordering is implemented — but it is the first thing P9 should read and correct.

### What handedness can and cannot reach

`LaunchConditions.spin_axis_deg` is **geometric**: positive is a right-hand rotation of the spin
axis about the direction of flight, which curves the ball right. `ShotData.spin_axis` is
`+ = fade`, and a fade is right for a right-handed golfer and left for a left-handed one, so the
two agree only once handedness is known. That mapping is deliberately not in `analysis/flight.py`,
which flies a ball, and a ball has no handedness.

It cannot reach the gate: **carry is even in the spin axis**, so reading both validation shots as a
left-handed golfer's leaves every number above unchanged. Handedness decides only which side the
ball finishes, which is `landing_offline`'s sign and P9's to resolve. `contracts/dispersion.py` is
the precedent for taking that seriously — a camera-relative sign meeting a mixed-handedness corpus
read every left-handed golfer as a gross fault.

### The one modelling choice P5 made, and the alternative it declined

The Magnus term is `0.5*rho*A*Cl*|v|^2 * (w_hat x v_hat)` with the cross product **not**
renormalised. Only the spin perpendicular to the velocity makes a Magnus force, so the `sin(theta)`
the cross product carries is the physics rather than an artefact to divide out. At launch the axis
is square to the velocity by construction and the factor is exactly 1 — which is why the planar
pins survive — and on a 21° launch with a 15° axis it has fallen only to 0.94 by landing.

Renormalising was the alternative and it is wrong at the limit: it would hold full lift on a ball
spinning about its own line of flight, which makes none at all.

## Addendum (2026-09-05e, M15 P6): the altitude what-if, and the inversion grows with the air

P6 landed the last of Stage B: a fifth block in `flight_model_v1.json` (`atmosphere_profile`, ISO
2533's lowest layer), `FlightModel.at_altitude()`, and the subprocess pin §Decision 1 asked for by
name — `analysis.flight` and its constants loader import neither numpy nor scipy.

`simulate_flight` learned **no new argument**. The air is a block of the model, the model is
already a parameter, so the what-if is `simulate_flight(launch, model=load_flight_model()
.at_altitude(1609))`. An `altitude_m` keyword beside `model` would have been a second way to say
one thing and the two would have drifted.

### The profile is written as ratios, and the reason is a defect in the committed row

The standard atmosphere is normally quoted as three absolute formulas. Written that way, evaluating
the profile at zero altitude does **not** return the sea-level block it is supposed to extend — and
the interesting part is that this is only half true, in a way that argues for ratios more strongly
than a uniform disagreement would have.

| quantity | committed | absolute ISA at 0 m | relative |
|---|---|---|---|
| density | 1.225 | 1.22500002 | 1.5e-8 |
| kinematic viscosity | 1.4607e-5 | 1.4607186e-5 | 1.3e-5 |

Density is essentially exact because ISO 2533 chose its gas constant to make that sea-level triple
self-consistent. Viscosity is not, because the standard publishes it rounded to five figures. So
the committed row is exact in one quantity and rounded in another, and *which* one depends on how
the artifact was transcribed rather than on any physics.

Ratios do not care. `at_altitude(0.0)` returns all four numbers to the bit, and a flight flown
through it is point-for-point identical to the default — by construction, not by the committed
numbers happening to agree with each other. Only `name` and `source` differ, deliberately: a
generated air state should say which law generated it rather than pass itself off as the published
row. The bounds (−2000 m to 11000 m) are the standard's own layer and not a judgement about golf;
they contain every course on earth, and an altitude outside them raises rather than extrapolating.

### Thin air is not simply longer, and the second half is the coaching-relevant one

Density scales lift and drag together, so altitude takes away the force holding the ball up as
well as the one slowing it down.

| altitude | `2026-08-10-1` | `2026-08-10-2` |
|---|---|---|
| −390 m (Dead Sea) | 121.34 yd (−0.83%) | 122.16 yd (−0.99%) |
| 0 m | 122.36 yd | 123.39 yd |
| 1609 m (Denver) | **125.99 yd (+2.97%)** | **127.95 yd (+3.70%)** |
| 4369 m (Tactu) | 129.99 yd (+6.24%) | 133.78 yd (+8.42%) |

About 3% for a 7 iron at a mile up, not the ~10% the driver rule of thumb is quoted at — a lofted
shot spends much of its carry on lift, and it loses that at the same rate it loses drag. And the
same shot arrives **flatter**: at Denver `2026-08-10-2` drops from a 21.30 yd apex to 20.59, from
4.74 s to 4.63, from a 40.1° descent to 37.7°, and its curve shrinks from 6.58 yd to 5.96. A shot
that holds a green at sea level runs through it at altitude, and the curve a golfer plays for gets
smaller with the air.

### Altitude cannot escape the clamp, which bounds what this phase can be blamed for

The spin ratio at launch is `wR/v` — launch conditions only, no air in it — so no atmosphere moves
it, and `S` only climbs from there. Both shots launch above the published ceiling at every
altitude, stay `fully_clamped`, and altitude therefore acts on them as a pure scaling of two held
coefficients. Thinner air does hold the ball's speed up, so `S` climbs *less* by landing (0.544 →
0.453 on `2026-08-10-1` at Tactu), but the whole range stays above 0.284 and nothing is read from
the table either way.

### The finding: altitude widens the inverted ordering rather than closing it

The 2026-09-05c addendum's central result is that this model ranks the two validation shots
backwards. P6 measured what the air does to that.

| altitude | model's gap (shot 1 − shot 2) | HD Golf's gap |
|---|---|---|
| −390 m | −0.82 yd | +4.6 yd |
| 0 m | −1.03 yd | +4.6 yd |
| 1609 m | −1.97 yd | +4.6 yd |
| 4369 m | −3.78 yd | +4.6 yd |

**The gap grows monotonically with altitude, and by far more than the carry does** — it nearly
quadruples across a range over which carry moves 6%. That is a diagnosis rather than a new fault.
Above the clamp the only lever this model has between these two shots is 2.6° of launch angle, and
thin air amplifies exactly that lever; a mistyped launch condition would have scaled with the
carry instead. It also closes off a repair that would otherwise have looked available: **no
atmosphere is the free parameter that fixes the gate.** Every direction of air makes the ordering
worse or leaves it alone.

### And the gate is a sea-level gate — flying it higher is numerology

`gate_comparisons()` takes a model, so it will happily fly the validation shots at Denver, where
the first of them agrees with HD Golf to **0.3%** instead of 2.6%. That agreement means nothing:
HD Golf printed a carry for a ball hit indoors near sea level, so the only air the gate can be run
in is the air the shot was hit in, and tuning an altitude until the agreement improved would be
fitting the atmosphere to the residual of a clamped coefficient table. There is a test whose only
job is to say so — the same shape as the one P4 wrote against the bias correction — and it pins
that the altitude which flatters shot one pushes shot two *outside* the tolerance entirely.

## Addendum (2026-09-05f, M15 P8): the solve is built, and it names a spin for four of the eleven shots it was written for

`analysis/spin_solve.py` is §Decision 3 as code. Nothing about the inversion moved — this phase ran
the solve over every shot on disk rather than changing any physics — and what it found is a count:
**the solve produces a number for 4 of the 11 spin-less shots, and 7 of them it refuses outright.**
§Decision 3 reads as though the inference is the ordinary path and the refusal the exception. On
this corpus it is the other way round.

### What each of the eleven does

| case | shots | what comes back |
|---|---|---|
| above the peak | **7** | nothing — no spin flies the ball that far |
| two branches | 3 | two spins; §Decision 3's loft prior has to choose (M15 P9) |
| between the plateaus | 1 | one spin, at 2,307 rpm |

The seven refusals are the 2026-09-05b addendum's own measurement, unchanged by P5's third
dimension: they clear the peak by **+0.56% to +3.45%, median +0.97%**, against a model that
disagrees with HD Golf by ~2.6% on the two shots where it can be checked. Nearly every one of them
is inside the model's own error, which is why they are reported as *this model cannot fly that
carry* and never as an OCR fault.

The three two-branch answers are 2,849, 2,924 and 3,200 rpm on the falling branch, each against its
own cap of 5,103–5,734. They are the same shape as the answer the honest test already knows to be
47% low, so the count above is the *optimistic* reading: four shots get a number, and there is no
shot on disk where a number of this kind has been shown to be right.

### The honest test, re-run in three dimensions

| shot | measured spin | what the solve does | cap |
|---|---|---|---|
| `2026-08-10-1` | 5,991 rpm | returns **3,185 rpm** — **46.8% low** | 5,154 rpm |
| `2026-08-10-2` | 8,100 rpm | **refuses**: 121.0 yd printed against a floor of 122.62 | 5,143 rpm |

Both numbers moved slightly from the 2026-09-05b addendum's (3,201 rpm, and a floor 2.0 yd above
the printed carry) because P5's recorded spin axes are now flown, and neither moved enough to
change a word of what that addendum concluded. Both true spins remain above their own cap, which
is the mechanism: at 5,991 rpm the carry has stopped depending on spin at all, so no solve of any
quality could have recovered it.

### The correction: the OCR consistency check has no survivors left

The 2026-09-05b addendum found one printed carry that its own margin could not explain —
`2026-08-23-2`, 83.8 mph at **3.4°** with a printed 33.6 yd, *"against a floor of 44.8 — 25% below
anything the model can fly"*. That floor was the high plateau, measured before P4 found the low one
underneath it. The real floor for those launch conditions is **26.89 yd**, and 33.6 yd is inside
the range after all: the solve returns 2,307 rpm.

So the by-product §Consequences claimed — a printed carry no spin can produce is an independent
check on numbers read off a photograph — is now empty on this corpus. Every printed carry on disk
is either reachable or misses by less than the model's own disagreement with HD Golf. The check is
not wrong in principle; it is unusable while the model's error is larger than the discrepancies it
would be detecting.

### Two things about that shallow shot that P9 has to know

**It has no falling branch.** Its carry rises from the low plateau straight to the high one and
never falls — the peak *is* the high plateau, to the last bit. So there is no two-solution band on
it at all, and a loft prior has nothing to choose between. It is also the reason `carry_window`
takes the best of three known points rather than trusting the interior search: a golden section on
a monotone interval walks to the edge, which is the right answer here and would be an artefact
anywhere else.

**And it qualifies this addendum's predecessor.** The 2026-09-05c addendum asks P8 to treat a
unique answer between the plateaus as a refusal, on the ground that the band sits at spins no
7 iron produces — on `2026-08-10-1` it runs **1,129–1,538 rpm**, which is exactly right. But on
this shot the same band runs **1,323–4,572 rpm** and the answer lands at 2,307, an ordinary iron
number. The refusal is still the right call more often than not; it just has to be argued from
where the answer sits rather than from the case's name, and `CarryWindow` carries the band edges so
that P9 can.

### Where the seven cases came from, and why they are not five

§Decision 4's five cases are the vocabulary; `SpinSolveCase` has seven because two of the five
split in ways the addenda measured. *On the plateau* is two different plateaus — one at spins below
anything a golfer produces and one at the cap, and they mean opposite things about the shot.
*Inside the window* is two bands, because between the plateau values only the rising branch reaches
and the answer is unique rather than paired. Nothing was added beyond what P3 and P4 found; the
enum is those findings written where code can read them.

### One more number worth recording, because a later phase will want it

The window quoted in the 2026-09-05b addendum — *"1.3–10.6 yd wide"* — is the **two-branch band**,
peak down to the high plateau, and it was measured before the low plateau was known. Re-measured
across the eleven it is 1.3–10.7 yd, unchanged. The *whole* achievable range, floor to peak, is
**6.8–23.6 yd**. Both are true and they answer different questions: the first is how much room a
loft prior has to work in, the second is how far a printed carry can be wrong before the model
stops being able to fly it at all.

## Addendum (2026-09-05g, M15 P9): the branch rule is built, and every shot on disk refuses it

`analysis/flight_infer.py` is §Decision 3's loft prior and §Decision 5's axis resolution, and it is
the first phase of M15 whose headline is about what is *not* on disk rather than about the model.
Run over the thirteen stored shots it produces **no spin at all and two spin axes**:

| what came back | shots | why |
|---|---|---|
| `carry_unreachable` | 8 | no spin flies that carry — P8's seven refusals, plus `2026-08-10-2` |
| `no_club_loft` | 5 | two spins do, or one implausible one, and **no shot on disk carries a club** |
| a spin | **0** | — |

`spin_axis` resolves on the two shots that printed one and `spin_axis_unresolved` on the other
eleven. Nothing here is a new disagreement with HD Golf; it is the same corpus meeting the two
inputs the inference needs and finding neither.

**The club tag is the whole of the spin half.** M9 closed in the same state and for the same
reason — every club-narrowed answer refuses because no swing on disk is tagged — and one bay
session with the club cursor set on the upload page turns both on at once.
`BAY_SESSION_RUNBOOK.md` already says so for M9's sake; it is now true twice.

### The loft prior needs no loft-to-spin model, and that is the phase's one design decision

The branch looks like it wants a published spin table and does not. It is one bit — **does this
club spin the ball faster than the peak-carry spin?** — and the peak is *measured per shot* by
`carry_window` rather than assumed. On the three two-branch shots on disk it sits at 2,402, 2,555
and 2,781 rpm, which is a driver's own spin rate. Every club with more loft than a driver spins
harder than that, so the falling branch is theirs; a driver is the one club whose two candidates
straddle its own spin, and there the prior refuses instead of guessing.

So the module holds a single constant, `LOFT_FLOOR_FOR_THE_FALLING_BRANCH_DEG`, placed in the gap
between a driver's ~12° and a fairway wood's ~15° — a gap no club occupies — and
`tests/analysis/test_flight_infer.py` pins that the branch chosen is identical for every loft
above the gap and refused for every loft below it. That invariance is the honest form of "this
prior is coarse": it does not have to be accurate, it has to land on the right side of a peak it
clears by a factor of two, and the test says so rather than a comment claiming it.

It is also **unexercised on real data**, for the reason the table above gives.

### The correction: §Decision 5's second branch resolves the sign and not the magnitude

§Decision 5 reads as though face-to-path stands in for the axis: *"the measured `spin_axis` where
one exists; else `measure_face_to_path`… else the curve is refused."* Face-to-path is in degrees
and sits on the same screen as an axis in degrees, so a fallback that simply returned it would look
entirely reasonable. The two shots where both quantities are on disk are what rule it out:

| shot | face-to-path | measured axis | tilt per degree | backspin |
|---|---|---|---|---|
| `2026-08-10-1` | +10.9° | 2.5° | 0.23 | 5,991 rpm |
| `2026-08-10-2` | +13.2° | 9.3° | 0.70 | 8,100 rpm |

Three times the tilt per degree of face-to-path, at `n = 2`. **And the direction of the
disagreement is what settles it**: the second shot spins *more*, and more backspin under the same
sidespin tilts an axis *less*, so the two readings are not merely scattered — they are ordered the
wrong way round for any monotone relation between the two quantities. A magnitude fitted to two
points that point the wrong way is an invented number.

So the second branch resolves the **direction of the curve** and stops there, and the outcome is
§Decision 5's *third* branch: the flight is simulated in the vertical plane and `landing_offline`
goes unscored with `spin_axis_unresolved`. `InferredSpinAxis` carries the direction anyway, because
a consumer drawing a straight flight still wants to be able to say *"this was a fade and the model
is not drawing the curve."*

### The sign check runs, and one shot on disk fails it

Face-to-path agrees with the `Shot Type` tile on **11 of the 12** shots that carry both. The
twelfth is the 114.8 mph shot: the screen printed `SLIGHT FADE` and face-to-path reads **−1.3°**,
a draw. It is flagged (`InferredSpinAxis.sign_disagrees`) and nothing is overwritten, which is
§Decision 5's own instruction — resolving a disagreement quietly is how ADR-014's original sign
inversion survived as long as it did. At 1.3° it is as likely to be a `Shot Type` threshold as a
parse fault, and the flagged reading is what would let anyone find out.

Both *measured* axes agree with the word beside them, which is 2026-09-05d's finding that
§Decision 5's first branch is already live, re-checked. That addendum also noted that
`analysis/shot_measure.py`'s module docstring still describes the axis as sign-inverted — *"both
fades on disk carry a negative value"*, which has not been true since `screen/profiles.json` gained
its `printed_sign`. P9 owns §Decision 5, so P9 corrected the sentence.

### Four reasons over seven cases, and the split criterion is what a reader must *do*

`contracts/unscored.py` gains `CARRY_UNREACHABLE`, `NO_CLUB_LOFT`, `SPIN_NOT_RECOVERABLE` and
`SPIN_AXIS_UNRESOLVED`, all with `refilming_helps` false, and they are collected as
`INFERENCE_REASONS` — the vocabulary's third family beside the measurement and judging halves. The
partition test in `tests/contracts/test_unscored.py` was written as a *complement* of
`MEASUREMENT_REASONS` and would have absorbed a new family silently; it is now three-way.

Four rather than seven because the criterion here is what the reader does, and for three of the
four the answer is nothing. `NO_CLUB_LOFT` is the one that is not: it is cleared by tagging the
swing with a club, from the results page, without re-filming. `spin_solve.SpinSolveCase` already
names all seven shapes and travels beside the result on `InferredSpin.solution`, so a second copy
of that taxonomy in `contracts/` would be a second thing to keep in step.

### The unique band is refused against the peak, as 2026-09-05f asked

That addendum found the blanket refusal of `BETWEEN_PLATEAUS` unarguable from the case's name, and
this is the argument that replaced it. A unique answer there is on the *rising* branch by
construction — the falling branch never comes down that far, which is what made it unique — so it
sits below `peak_rpm`, while the prior says a lofted club spins above `peak_rpm`. Both cannot be
right, and the prior is the one built on a club rather than on a printed carry. It refuses in every
case on this corpus, and it refuses for a stated comparison that a driver, or a differently-shaped
curve, would not satisfy.

### What every inferred spin has to be reported beside, measured rather than quoted

**The cap.** `CarryWindow.high_plateau_min_rpm`, 5,103–5,734 rpm across the shots on disk. A
`flight_spin_rpm` shown without it reads as a low-spin diagnosis of the golfer, when it is the
point past which carry stopped depending on spin at all.

**The gate as an ordering, not as a percentage.** `gate_ordering()` re-flies both shots twice — as
recorded, and with the lateral fields stripped — so nothing here is a stored number:

| | shot one minus shot two |
|---|---|
| HD Golf | **+4.60 yd** |
| this model, as recorded | −1.03 yd |
| this model, flown planar (M15 P4) | −1.66 yd |

The residual is **5.63 yd against a 4.60 yd effect** — larger than the difference it is failing to
reproduce — and the recorded spin axis closes **10.1%** of it. Reporting ±2.59% as "accurate to
2.6%" says the opposite of what the gate found, which is why the percentage is not the number this
module hands to a caveat.

**And the honest test, re-run through the prior rather than through the raw solve.** Given the
7 iron's 30.5°, `2026-08-10-1` takes the falling branch at **3,185 rpm against a measured 5,991 —
46.8% low** — and `2026-08-10-2` refuses, its printed 121.0 yd sitting 1.62 yd below the floor.
Both true spins are above their own cap. Nothing moved from 2026-09-05f; what is new is that the
prior is now the thing choosing, and it chooses the branch that is wrong by half.

## Addendum (2026-09-05h, M15 P10): the club was on the swing all along, and the corpus names one spin

P10 pointed the CLI at the shots on disk, through `storage/flight_inputs.py` — a join, not a new
reader — and the first thing it found was that the previous addendum's headline is wrong in a way
that matters.

### The correction: "no shot carries a club" is a statement about `ShotData`, not about the corpus

2026-09-05g reported that the inference produces nothing on any of the thirteen stored shots
because none of them carries a `club`. That is true of the *shot* and always will be: a launch
monitor reports a ball. The **swing** the shot arrived with carries the club, and eleven of the
thirteen shots are attached to one that does — five `7i` and six `3w`, tagged at capture on
2026-08-23, months before this milestone opened. Nothing had to be re-tagged; the tag was simply
on the other side of a join nobody had performed.

The join costs one dictionary. `ShotStore` files a parse under `ShotProvenance.image_sha256` and
`SwingManifest.roles[SHOT_SCREEN].content_sha256` is the same digest, taken as the upload streamed
in — the content addressing `api/pipeline.py::_shot_for` already uses in the other direction. No
new identifier, nothing re-hashed, and no second tolerant reader: `ShotStore.all()` skips a file it
cannot parse and `read_flight_inputs` reports what it could not resolve.

**What is actually missing is one field, and it is not in the bay.** Only the 7 iron has a declared
loft (30.5°, P1's bag correction); no 3 wood has ever been declared at all. So `LoftGap` splits the
one `None` into the four repairs behind it — no swing, no club tag, no bag entry, no declared loft
— because they are not the same errand. `NO_CLUB_LOFT`'s own docstring said the repair was tagging
the swing; on this corpus the swings are tagged and the repair is the bag page.

### What the inference produces run end to end, which is one spin and ten refusals

| | count | which |
|---|---|---|
| flown on a measured spin | 2 | the 2026-08-10 pair, the only shots whose screen printed one |
| flown on a solved spin | **1** | `2026-08-23-4`, **2,924 rpm** under a 5,103 rpm cap |
| `carry_unreachable` | 7 | printed carries 0.6–3.6 yd above the peak of what their conditions fly |
| `no_club_loft` | 2 | both `3w`, both two-branch: 2,161/2,849 and 2,450/3,200 rpm |
| `spin_not_recoverable` | 1 | `2026-08-23-2`, the shallow one, unique answer below the peak |

The four solvable shots are 2026-09-05f's same four; what P9's prior does with them is name one,
refuse two for an undeclared 3 wood, and refuse the shallow one against the peak. **So the two
`no_club_loft` refusals are the whole of what a bag entry would buy** — look the 3 wood up and the
inference names three spins instead of one, with no bay session and no new physics.

### The gate's constant has been unverifiable since P4, and it checks out

`VALIDATION_SHOTS` was typed by hand off two screens and is the only external reference this model
has. The same two screens were parsed into the shot store, so resolving `2026-08-10-1` through the
ordinary path lands on the constant **to the digit, both lateral fields included** — a check that
costs nothing the moment anything reads the corpus, and one that a mistyped digit in either place
would have failed. It is pinned in `tests/analysis/test_flight_infer.py` against the typed tiles,
and `--shots` re-runs it against disk.

### A planar flight's landing offline is a number this repo already stores

With the axis unresolved the flight is drawn in the vertical plane (§Decision 5's third branch),
and its `landing_offline_yds` then comes out as `carry * sin(start line)` — to 1e-14 structurally,
and to 2.6e-5 yd on the real shot, where the only difference is the solve's own root tolerance.
That is `shot_measure.measure_start_line_offline`, recorded since M9, reached by forty flights
instead of one sine.

**This is an instruction to P11.** §Decision 6 forbids a simulated quantity sharing a name with a
measured one because `baseline.py::pooled_samples` groups by name; the hazard arrives here from the
direction that section did not anticipate — a *different* name for an identical number, on eleven
of the thirteen shots. `flight_landing_offline_yds` must not be recorded when the axis is
unresolved. `ShotFlight.curve_is_drawn` is the bit that says when.

### The one shot the model can invert is the only flight in this corpus that reads the table

`2026-08-23-4` flies its whole path inside the published coefficient rows — spin ratio 0.163 to
0.260 against a table covering 0.085 to 0.284, **0 of 919 points held** — while every other shot
on disk, both validation shots included, is clamped end to end.

That is structural rather than lucky, and it is the most useful thing this phase found about the
model. A carry the solve can invert is a carry that still depends on spin; spin reaches carry only
through the coefficients; so a solvable shot is by construction a shot flown on measured
aerodynamics. The corollary is worth stating in the other direction: **every shot this model flies
on a *measured* spin is a shot it flies on a held constant pair**, and the one flight that reads
the table is the one whose spin was invented. P16 draws both, and it must not present the clamped
ones as the trustworthy half.

### A fifth inference reason, which fires on nothing

`NO_LAUNCH_CONDITIONS` joins `INFERENCE_REASONS`: no ball speed, no launch angle, or a launch angle
at or below the horizontal — a ball that rolls, and roll is out of scope. Nothing on disk produces
it. It exists because the OCR drops tiles one at a time rather than all at once (one stored shot is
already missing its face angle), and because `simulate_flight` names its launch-angle guard as the
*caller's* to own: without a reason to return, that guard reaches a corpus read as a traceback.

## Addendum (2026-09-05i, M15 P11): the six measurements land, and two of them record conditionally

P11 put the flight into `analyze_swing`. `analysis/flight_measure.py` is §Decision 6 as code — the
six names, the `model:flight_v1` source, and the two `unscored` entries that stand in when there is
no flight — and `api/pipeline.py::_loft_for` is the shell read that feeds it, beside the handedness
read M6.5 named. The phase found three things this section did not have.

**⚠️ §Decision 6 named `flight_spin_rpm` without splitting the two spins that can fill it, and this
phase splits them.** A spin the screen printed is a reading of a ball; a solved one is *the spin
this integrator needs in order to agree with HD Golf's carry*, which `SpinSource` exists to say and
`spin_solve.py` says in exactly those words. Recording both under one name is the pooling hazard
this section was written to prevent, one layer in: `baseline.pooled_samples` groups by name, so on
the corpus today it would average a measured 5,991 rpm with a solved 2,924 and print the result as
one golfer's spin rate. **So `flight_spin_rpm` records only an inferred spin.** The measured one is
not lost — it is on `ShotData.spin_rate`, where the launch monitor put it, and `ShotFlight.spin_rpm`
is what P14–P16 read to say what the line was drawn with. It is simply not a *model* output and
does not enter under a `model:` source. The other four record on every flight, because the model
produced them however the spin arrived.

**⚠️ The planar-offline identity the 2026-09-05h addendum measured is not structural, and the
correction argues the same rule harder.** That addendum read 2.6e-5 yd between a planar flight's
`landing_offline_yds` and `measure_start_line_offline`, on `2026-08-23-4`. The two are
`simulated carry × sin(start line)` and `printed carry × sin(start line)`, so they agree only where
the two carries do — which is exactly the case that shot is: its spin was solved *from* the printed
carry, so the model reproduces it by construction. On a shot whose spin was **measured** the two
carries differ by the gate's own ±2.59%, and the two offlines differ with them: **0.29 yd apart on
`2026-08-10-1`**, with nothing available to say which is right. Two names for one quantity that
quietly disagree is worse than a duplicate, so `flight_landing_offline_yds` stays withheld wherever
the curve was not drawn — which is eleven of the thirteen shots — and the rule now covers both
halves of the corpus for two different reasons.

**⚠️ A refused flight was about to become a coaching tip, and `unscored` was the wrong shape by
half.** §Decision 6 says the flight gets no `CHECKPOINT_REGISTRY` entry and M15 P9 put its reasons
in `contracts/unscored.py` anyway, which is right — that module is where this repo names an absence.
What neither noticed is that two consumers of `SwingResult.unscored` assume every entry is a
checkpoint that *would have been* in `overall_score`: `feedback/rules.py` renders "could not be
scored on this swing, so it is not included in the score", and `feedback/coach.py` puts the same
claim in the brief a coaching model reads. Both sentences are false of a `flight_*` measurement,
which was never in the score to be excluded from. P11 corrects both — the tips skip
`INFERENCE_REASONS` entirely, and the brief renders the flight under its own heading — rather than
leaving a model to explain a score that never moved.

Three smaller things, recorded because each is a thing the next phase would otherwise re-derive:

- **A refused flight is one `unscored` entry, not five.** It is filed under `flight_carry_yds`, the
  quantity the model exists to produce, with the cause in `detail`. `mcp.query.get_session_summary`
  counts unscored entries by name, and one missing flight counted five times would report a session
  as five times more broken than it is. The axis is the second entry and is genuinely separate,
  because a refused axis does not stop the flight.
- **`simulate_flight`'s guards needed a catcher.** `flight_for_shot` owns two of the five — a
  missing ball speed or launch angle, and a launch angle at or below the horizontal — and the other
  three can still arrive from a parse. Reaching `analyze_swing` they would take the whole swing
  down, discarding pose that had already run and every checkpoint that had already scored, which is
  the failure `api/pipeline._shot_for` was widened to prevent one layer out. `fly_shot` catches
  `ValueError` and returns `NO_LAUNCH_CONDITIONS`.
- **The six get no `METRIC_TARGETS` row, and that is a decision rather than an omission.** Five of
  them are a deterministic function of measurements already in that table, so a scatter finding
  would re-report `carry_distance_yds`'s own spread with a model's error folded in; the sixth is
  solved rather than measured, and its error floor is the gate's inversion rather than an
  instrument's. `contracts/dispersion.py` carries the argument beside the table and
  `test_dispersion.py` pins the absence, because that file's own pin was written as
  `POSE | SHOT == METRIC_TARGETS` and a third registry would have slipped past it in silence.

**One path correction.** This section and the M15 roadmap both put `artifact_key` in
`storage/corpus.py`. It is `CorpusSwing.artifact_key` in `contracts/career.py`, and P12 — which
registers the `model:` prefix there — is where that matters.

## Addendum (2026-09-06, M15 P12): the fallback was not merely conservative

P12 registered `model:` in `contracts/career.py::CorpusSwing.artifact_key`, mapping it onto the
**shot photo's** identity — a flight is integrated from that tile's launch conditions and nothing
the golfer's body did, so two swings sharing one photo are one flight however many times it is
flown. `KNOWN_SOURCE_PREFIXES` is new beside it, because the membership test had been written out
twice and P12 would have made that three times.

**⚠️ The argument for letting P11 ship ahead of this was wrong, and the direction matters.** P11's
note in `analysis/flight_measure.py` said the `swing:{ref}` fallback "can only over-count against a
real artifact key, never under-count", which is true of the *dedupe* and silent about the
*refusal*. `artifact_key` returns `None` for a launch-monitor metric whose parse was flagged under
ADR-014, and the fallback branch has no such rule in it. So between P11 and P12 a flight simulated
off a tile flagged for review counted as a sample while the `carry_distance_yds` printed beside it
on that same screen did not — the model's number trusted where the reading it was computed from was
not. It moved nothing on disk (no stored parse is flagged), and it would have moved something on
the first bay session, which is the only kind of bug this repo's `n` can currently hide.

**`population:golfdb` stays in `unknown_sources`, and that is now written down where it is
reachable.** P12 nearly registered it in passing: the counts do not move — `read_corpus` groups
swings *by* `face_on_sha256`, so `CorpusSwing` is one-to-one with that hash and `swing:{ref}`
partitions any corpus exactly as `pose:` would — and every test stays green, which is the hazard
rather than the reassurance. [ADR-022](022-learned-artifacts-as-committed-data.md)'s fourth addendum
defers two questions a one-line registration would have answered by accident: whether a
distance-from-a-tour-population is a personal quantity at all (the honest return may be `None`, not
a key), and, if it is, that the two down-the-line placements are read off a clip `CorpusSwing`
carries no hash for. `tests/storage/test_corpus.py
::test_a_placement_is_still_an_unknown_source_and_that_is_the_deferral` now pins the absence with
that reasoning, so the next reader tidying the warning away meets the argument first.

**One correction to `no_population_reason`.** `contracts/comparison.py` dispatches its refusal
sentence on the same prefix, and an unregistered `model:` metric was getting the generic *"no
reference distribution is stored for it"* — which tells a reader of a simulated carry that the
repair is to go and cut one. It is not: no population of model outputs is a population of swings,
and acquiring the launch-monitor reference §Deferred asks for would sharpen the measured carry
beside it and leave the simulated carry exactly as unplaceable.

## Addendum (2026-09-06b, M15 P13): the corpus is on the new engine, and the re-run cannot prove the prefix

`ANALYSIS_VERSION` moved 14 → 15 and `reanalyze.py --all` put all fifteen stored swing directories
on it, 15 of 15 clean. §Consequences predicted the shape of this and it holds exactly: **every
`overall_score` is byte-identical**, and so is every `score`, `observed` and `passed` in every
`checkpoint_scores` entry — the panel neither changed size nor moved a digit. What is new on disk is
the `flight_*` family, and on the swings that could not be flown an `unscored` entry instead:
**five artifacts carry a flight and ten carry a refused one** — seven `carry_unreachable`, two
`no_club_loft`, one `spin_not_recoverable` — which is 2026-09-05h's refusals arriving unchanged.
An eleventh `unscored` entry sits on a swing that *did* fly: `2026-08-23/4`, whose spin was solved
and whose axis was not, carries `flight_landing_offline_yds` / `spin_axis_unresolved` while its
other five numbers record. That is 2026-09-05i's asymmetry doing its job on disk for the first
time. Two of the refusals also put a **new sentence in
front of the golfer**, the 3 wood's *"add the club on the bag page"* — the one repair on this corpus
that needs no bay session.

### The correction: this re-analysis cannot prove the prefix, and the argument is P12's own

§Decision 6 says the prefix registration "lands before the `ANALYSIS_VERSION` bump so that the
re-analysis is what proves it". It does not. `flight_carry_yds` counts **n = 3** against the five
artifacts carrying one, which is the right answer — but `model:` is not what made it right: three of
those five are re-uploads of a single face-on clip, and `read_corpus` had already collapsed them
into one `CorpusSwing` before any prefix was consulted. Fifteen directories hold **13 distinct shot
photos**, and the only repeated photo is that same triple, so across the corpus's 13 distinct swings
the photo key and the `swing:{ref}` fallback partition **identically** — reverting the registration
would move no count in the report.

That is the argument P12 made *against* registering `population:golfdb`, arriving now at the
registration it made *for*. The other half of P12 — `artifact_key` returning `None` for a parse
flagged under ADR-014, which the fallback has no rule for — is likewise unexercised, because no
stored parse is flagged. **Both halves are correct and both are unproven**, and the first bay
session where one photo backs two swings, or one tile is flagged, is the first evidence either way.
The honest reading of `n = 3` today is *three flights over thirteen swings*, and not *the dedupe
key works*.

### The bump carried four M14 measurements, and that is a finding about when a version is owed

`hand_separation_norm`, `hand_height_norm`, `hand_offset_from_hips_norm` and `trail_hand_roll_deg`
shipped on 2026-09-03 under "no checkpoint, no band, and no `ANALYSIS_VERSION` bump" — the first two
clauses right and the third wrong by this repo's own rule. `ANALYSIS_VERSION`'s comment block opens
*"bump it whenever the engine's output changes **meaning** — a new measurement"*, and `3 -> 4`,
`6 -> 7`, `7 -> 8` and `9 -> 10` are four precedents that bumped for measurements nothing judges.
Because M14 did not, `is_outdated` — a version comparison and nothing else — could not see that
every stored artifact was missing four quantities the installed engine produces: their honest `n`
was **0** for three days while the milestone that added them read as complete, and this bump is what
wrote them. It is 13 now, swept in as a passenger.

**The rule the two milestones bracket.** A band and a version answer different questions: *does
anything judge this* and *does a stored artifact still mean what it says*. A measurement nothing
judges still moves the version, and a phase that adds one either bumps or runs `--all` itself — M13
changed a sentence rather than a measurement, correctly did not bump, and ran `--all`; M14 added
four measurements and did neither. Nothing in the suite catches this, because no test compares a
stored artifact's measurement names against the registry that produces them — only a re-run does,
and a re-run is exactly what a missing bump suppresses.

## Addendum (2026-09-06c, M15 P14): the flight has a route, and it can disagree with the artifact beside it

`GET /api/sessions/{id}/swings/{id}/flight` is built. It resolves the swing's shot through the
photo's sha256, borrows the loft and the handedness the way P10's join does, flies what it can, and
serves the path, the six measurements, the printed numbers beside them and the caveats. The
derivation is `api/flight_view.py`; the route in `app.py` is the manifest reads and nothing else.

**It re-flies rather than reading `analysis.json`, and the first reason is trivial.** The stored
artifact holds the six numbers and no path, a path is the whole of what a viewer draws, and
re-integrating one costs about ten milliseconds. Storing nine hundred points per swing to avoid
that would be storing a derivation.

**⚠️ The finding is the second reason, and it is a seam this ADR did not have.** The flight's
inputs are not all on the swing. Two of them — the club's loft and the golfer's handedness — live
in *editable* artifacts that the swing merely points at, so **the same shot has two answers on this
repo at once**: what the engine resolved when it ran, which is what `analysis.json` holds and what
`career_baseline` counts, and what resolves now, which is what the route draws. Declare the 3
wood's loft on the bag page and the route flies a shot the corpus still counts as
`no_club_loft` — no re-analysis, no version change, and nothing on disk that can see the
difference: `is_outdated` is a version comparison and nothing else (P13's finding, arriving from
the other side), and `AnalysisState.inputs` hashes the *uploads*, which did not move.

The shape is not new — re-attributing a swing to a left-handed golfer leaves `head_stays_back`
scored under the old sign in exactly the same way, and has since M6.5 — but the size is. Every
earlier case moves one number; a loft flips **five** measurements between recorded and withheld,
plus the flight's presence in `flight_carry_yds`'s `n`. §Consequences anticipated a simulated carry
disagreeing with a measured one on the same screen. It did not anticipate a simulated carry
disagreeing with *itself*, one page apart, and that is the disagreement a reader is least equipped
to interpret because both numbers wear the same name and the same `model:flight_v1` source.

**Left as a finding rather than repaired here, and the repair is not obvious.** Re-analysing on
every bag edit would re-run pose over every swing that club ever hit; invalidating on it would mark
fifteen artifacts stale for a field five of them use. The cheap half — the route reporting the
stored value beside the live one, so a page can say "recorded as refused, flies now" — is a payload
question and belongs with the viewer-honesty phase that would render it.

**Three smaller things this phase settled.**

**A refusal is a 200 and a 404 is something else.** Ten of the thirteen shots on disk cannot be
flown, so a route that treated a refusal as an error would report this repo as broken every time it
was read honestly. The 404 is reserved for a swing with no shot-screen photo, or one whose photo has
never been parsed — there is no flight resource and no sentence about physics to offer. No OCR
runs in the request: the store is keyed on the photo's digest, so an imported screen attaches
without opening the image, and an unread one says which two things read it.

**The four caveats moved out of `scripts/simulate_flight.py`** into `analysis/flight_caveats.py`,
because P7 wrote them when a dev CLI was the only surface and P14 is the second reader.
`caveats_for` composes them, so *which* caveats a flight owes is one rule rather than two — the
argument `contracts/caveats.py` makes about its own prose, one layer up. The path-sampling rule
moved the same way and for the same reason, onto `FlightResult.sample`, where the landing the
integrator solved for is guaranteed to survive the sample.

**The loft remedy is offered only once the flight has asked for a loft and gone without.** It is
`api/pipeline.py`'s rule reaching a second shell, and the reason it is a rule: a shot whose screen
printed a spin never reaches for a loft, so telling that golfer to go and fill in the bag page is a
repair for a problem they do not have. `pipeline.loft_remedy` is now the one definition of those
four sentences.

**And the route is the only sync handler in `app.py`.** Measured on this corpus: every other route
answers in 3-24 ms, this one in 15 ms where the screen printed a spin and up to ~960 ms where it
did not, because `spin_solve.carry_window` measures the whole carry-against-spin curve in about
forty flights. FastAPI runs a `def` handler in a threadpool and an `async def` one on the event
loop, so the keyword is what stops a one-second solve stalling the upload page's 5 s status poll
in another tab.

## Deferred, by choice

**The impact model — club delivery to launch conditions.** The other half of the physics, and the
half that would let loft and lie mean something predictive. It needs an attack angle and a
trustworthy club speed, and this bay currently supplies neither.

**Gapping, club fitting and swing efficiency.** ADR-024 and ADR-026 defer these and they stay
deferred. A flight model is a prerequisite for the first two and does not on its own deliver either;
in particular, a per-club carry expectation needs a population this repo does not have, which is
ADR-010 §4's outstanding TrackMan/Arccos acquisition.

**Wind, and any lie other than a flat one.** The atmosphere is **built** as of P6 — see the
2026-09-05e addendum — because air density was nearly free once the loop existed and is the
difference between a bay in a garage and a course at altitude; it is a what-if rather than an input
to any stored flight, since every shot on disk was hit indoors near sea level. Wind is a vector
field and a different conversation, and stays deferred.

**Roll-out.** `total_distance` minus carry is the ground rather than the swing, as
`analysis/shot_measure.py` already argues in refusing it a target. Simulating it would mean modelling
a surface nobody has described.

**A spin measurement.** The real repair for §Context 2 is the open Milestone 3 item — tuning the
screen profile against the bay's actual tile layout, which is already known to differ from the two
reference photographs. If that lands and spin starts arriving, §Decision 3's inference becomes a
fallback rather than the main path, and the two can be compared: **the inferred spin on a shot that
later reports a real one is the honest test of this ADR's weakest claim**, and it should be run the
day such a shot exists.

> **That day was 2026-08-10, and this paragraph did not notice.** The two reference shots in
> §Context 2 carry a measured spin *and* a printed carry, which is everything the test needs. The
> 2026-09-05b addendum runs it: one refusal, and one answer 47% low. The deferral above still stands
> as the real repair — a screen profile that prints spin is worth more than any inference — but it is
> no longer the thing that unblocks the comparison, because the comparison has already been made.

## Addendum (2026-09-06d, M15 P15): the page, and the axis it is allowed to stretch

P15 built `api/static/flight.html` — vanilla JS on a canvas, no framework and no CDN, which is
the *Alternatives* section's own call arriving as code. Two projections of one polyline rather
than a perspective view of it: a side elevation for height and a plan view for offline. A single
3D view would need a camera, and every camera angle makes one of those two questions unreadable
while still looking authoritative about both.

**The finding is about the second panel's vertical axis, and it inverts what the first version
did.** Down range is drawn at the same scale in both panels so the views stay column-aligned, and
the side view is 1:1 with it — the panel's height falls out of the scale rather than being chosen,
so a flat shot draws flat. Offline cannot be, or a two-yard drift across a hundred-and-twenty-yard
carry is a line one pixel thick. So the plan view stretches its offline axis by a round factor and
prints the factor in the caption at full opacity.

The first version picked the **largest** factor that fit the panel's budget. Measured against the
two flights on disk at a 600px canvas, that is wrong and visibly so:

| shot | carry | apex | offline reach | largest-that-fits | smallest-that-reads |
|---|---|---|---|---|---|
| `2026-08-10-1` | 122.0 yd | 18.5 yd | 9.60 yd | ×2 — plan panel 214px against a 117px side view | ×1 — 120px |
| `2026-08-23-4` | 125.9 yd | 19.2 yd | 6.16 yd | ×2 — 143px | ×1 — 84px |

Every flight this corpus holds drifts a third to a half of its own apex, so "largest that fits"
magnified all of them, and the page drew the lateral miss **taller than the height of the shot** —
on a page whose whole purpose is to be read literally, next to a panel captioned 1:1. The rule is
now the smallest factor that lifts the drawn curve over a legibility floor: a drift already
readable at 1:1 is drawn at 1:1, and only a line too thin to see is stretched. On this corpus the
stretch never fires; it exists for the shot hit dead straight, where at 122 yd of carry a 0.4 yd
drift needs ×10 to be a curve at all.

**What the page draws that the six numbers do not carry**, and each of them is a rule this ADR
already argued somewhere else:

- **The clamped part of the path is dashed**, per point rather than per flight, because the spin
  ratio climbs as the ball sheds speed faster than it sheds spin and a flight can leave the table
  it started inside. The 2026-09-05b addendum is why that matters more here than anywhere: above
  the clamp, spin does not reach the carry at all, so drawing the whole line with one confident
  pen would present the extrapolation as the measured half. `2026-08-10-1` is dashed end to end
  (905 of 905 steps); `2026-08-23-4`, the one shot on disk that reads the table, is solid for all
  919 — and it is the one whose spin was invented, which is the pairing that addendum predicted.
- **The spin's provenance rides on the line carrying the value**, not in a footnote. §Decision 3's
  labelling is what makes the weaker claim safe to make, and a number shipped bare for one phase
  is the erosion that section names as the trigger for falling back to the alternative.
- **A refusal is content, not an error state.** Ten of the thirteen shots cannot be flown, so the
  page prints the reason, the remedy where there is one, and which of the seven solve cases the
  carry landed on — in a neutral banner, matching the route's own choice to answer 200.

**The link into it is offered on every swing whose screen has been read**, including the ten that
refuse. Hiding it on those would hide exactly the shots with something to say, and one of them —
the 3 wood — carries a repair the golfer can make from the bag page.

Not built here, and deliberately: the measured landing point beside the simulated one, the
`caveats_for` prose, and the visible planar treatment. Those are P16, and the payload already
carries all three (`measured`, `caveats`, `curve_is_drawn`) so that it stays a rendering phase.

**And a doc finding, found by writing this addendum.** The Status block above was a phase behind:
it read "eleven addenda" over twelve of them, still said P14 was unbuilt after P14 had shipped its
own addendum, and repeated the *three flights* count `docs/README.md` had already been corrected
to five. `tests/test_docs_truth.py` pinned the map's per-row count and not an ADR's account of
itself, so the header of the longest ADR in the repo — the part a reader of it actually reads —
was the one place the corrections could go stale unwatched. There is a pin now.

## Addendum (2026-09-06e, M15 P16): the comparison that is not one, and a flag nothing rendered

**M15 P16.** The phase the addendum above deferred three things to — the measured landing point
beside the simulated one, the `caveats_for` prose, and the visible planar treatment. It shipped all
three, and it corrects that deferral's own framing: only two of them were rendering.

### Setting a printed number beside a simulated one is a registry decision

2026-09-06d says the payload "already carries all three (`measured`, `caveats`, `curve_is_drawn`)
so that it stays a rendering phase". `measured` is the shot's printed numbers — all seven of them,
through `SHOT_MEASUREMENTS` — and putting one of them beside a simulated one means answering two
questions the payload did not carry: **which** printed name is the same quantity as which simulated
one, and **whether the difference between them is an error**. That is the question the `flight_`
prefix answers from the other side (ADR-022's 2026-08-17a addendum; §Decision 6 here), so it
belongs where the registries are: `analysis/flight_measure.compare_to_printed`, read by the flight
route as `comparison` and by `scripts/simulate_flight.py --shot` as a block. A page pairing
`flight_carry_yds` with `carry_distance_yds` in JavaScript would have been the third copy of two
registries, on the one page `tests/api/test_flight_page.py` exists to keep free of them.

**Two pairs, and there is no third.** `flight_apex_yds` has `ShotData.apex_height` opposite it and
**no shot on disk carries one**, so this model's apex is unfalsifiable on this corpus;
`flight_descent_angle_deg` and `flight_time_s` have no tile at all; and `flight_spin_rpm` is
structurally unpairable, because it records only a *solved* spin and a shot with a printed spin to
check it against is exactly a shot where it is not recorded (2026-09-05i).

### ⚠️ The finding: neither pair is a check, and for two different reasons

| shot | simulated | printed | apart | what the gap is |
|---|---|---|---|---|
| `2026-08-10-1` carry | 122.36 yd | 125.60 yd | −3.24 yd | the model's error, −2.58% |
| `2026-08-10-2` carry | 123.39 yd | 121.00 yd | +2.39 yd | the model's error, +1.97% |
| `2026-08-23-4` carry | 126.101 yd | 126.100 yd | +0.001 yd | **the solve's own input, read back** |
| `2026-08-10-1` offline | −9.60 yd | −11.60 yd | +2.01 yd | +1.71 bend, +0.30 lean |
| `2026-08-10-2` offline | 15.17 yd | 8.44 yd | +6.73 yd | +6.57 bend, +0.17 lean |

**The carry is a check on two of the three flights this corpus produces and a tautology on the
third.** Where the screen printed a spin, the flight was flown on it and the printed carry is an
independent measurement — that is the gate, per shot. Where the spin was *solved*, the printed
carry is the solve's input: the flown carry reproduces it to the root search's own tolerance, and
`+0.001 yd` is the most convincing pair of numbers on the page and evidence of nothing. The CLI has
said so since M15 P7, in a paragraph under its `--shots` table; that paragraph is now
`circular_carry_note`, which composes both the column form and the per-row one, because P16 was
about to write the same claim a second time on a web page.

**The offline pair is not the same quantity twice, and its gap is not "the curve" either.**
`start_line_offline_yds` is where the ball *started*, projected out to the printed carry;
`flight_landing_offline_yds` is where the model has it finishing. The difference is therefore
`curvature * cos(direction)` **plus** the carry row's own disagreement leaning on the start line —
the identity `FlightResult.landing_offline_m` is written from, inverted — and on `2026-08-10-1` the
lean is 15% of the gap. Attributing all of it to the bend would have quietly re-reported the carry
error as curve, on the one row whose whole job is to say that a number was never measured.

### ⚠️ And a flag that had never reached a golfer

§Decision 5 says the screen's own shape word and the direction derived from face-to-path are
**flagged and never overwritten** when they disagree, and `InferredSpinAxis.sign_disagrees` has
carried that since P9. The CLI prints it. Nothing else ever did — and when this page's version was
first written inside the launch-conditions block, where a fact about the spin axis naturally
belongs, it still did not: **the one shot on disk that sets the flag is a `carry_unreachable`
refusal**, so it has no launch conditions, so that block does not render for it. A flag whose only
renderer is a block the flagged shot never reaches is not flagged. It is rendered from `render()`
now, beside the refusal banner, with both readings shown and neither picked.

### What else the drawing had to stop asserting

- **A planar flight's line no longer ends in a landing mark.** With the axis unresolved the flight
  is flown in the vertical plane and §Decision 6 withholds `flight_landing_offline_yds` for exactly
  that reason (2026-09-05i) — so a filled landing dot on the plan view was the page asserting the
  number the pipeline had refused to record. The line still draws, because the start line projected
  out is a true thing to draw; the panel now says in the refusal's own words that its end is not a
  landing point.
- **The printed carry is drawn as a rule across the plan view, never as a point on it.** The screen
  prints no offline tile at all, so the launch monitor knows how far this ball went and nothing
  whatever about where it finished sideways; a ring on the target line would have invented a
  landing. The ring the plan view does get sits on that rule at the *start line projected out*,
  which is the only lateral number the screen supports.
- **The printed carry sets the down-range span when it is the longer of the two**, which it is on
  both measured-spin shots by about three yards. Scaled to the flight alone, the mark for the
  measurement — the one this phase exists to show — is the single thing drawn off the edge of the
  canvas.

## Addendum (2026-09-06f, M15 P17): the tool, the direction of a join, and six floats that said nothing

The eleventh MCP tool, `simulate_flight(session_id, swing_id)`. It flies the shot a stored swing
arrived with and returns the six numbers with their provenance, the refusal where there is one, the
launch conditions, the spin's source and cap, the axis and its disagreement flag, P16's printed
pairing, and `caveats_for`'s sentences. No path: a model cannot look at a polyline, so the only
surface-shaped thing this one drops is the drawing.

**It is not registry-gated**, unlike the career and club tools, and that is a decision rather than
an oversight: those five begin by resolving a name to a `player_id` and can do nothing without one,
while a shot whose screen printed its own spin borrows nothing from a bag. Without a golfer registry
the loft resolves to `NO_BAG_ENTRY` and the axis refuses, which is a flat flight rather than no
flight — so gating it would remove answers instead of protecting them.

### ⚠️ The finding: the surface that talks had been saying nothing

§Decision 6 records `model:flight_v1` on every one of the six, `engine.py` writes it, `analysis.json`
carries it, and the page has said SIMULATED on every row since P15. **The MCP payload dropped it.**
`mcp/query.py::_measurements` flattens `SwingResult.measurements` to name -> value — right for a pose
metric, whose unit and detail are provenance for a derivation step — so what reached a coaching model
was `flight_carry_yds: 122.36` and `flight_spin_rpm: 2923.86`, bare, under a field description that
opens *"quantities measured off this swing"*. A solved spin and a printed one were, on that surface,
the same kind of number under the same heading: precisely the pooling this section exists to prevent,
on the one surface whose output is sentences spoken to a golfer.

The repair is `PlacementView`'s, arriving from the other direction. There, what a bare float loses is
the *meaning*; here it is the fact that nothing measured it. `SimulatedView` keeps the unit, the
source and the detail, and membership is decided by `Measurement.source` through
`contracts.career.MODEL_SOURCE_PREFIX` rather than by the `flight_` name prefix — the same choice
`_measurements` already records for placements, so a second model's numbers land there the day they
exist. `contracts/caveats.py`'s `READING_A_SIMULATED_FLIGHT` is the standing rule beside it, and it
ships on **every** shape of the server, because the split reaches a model through `get_swing` whether
or not the new tool is ever called.

### ⚠️ A join has a direction, and P10's is the other one

`storage/flight_inputs.read_flight_inputs` answers *"which swing was this shot hit on"*, so where one
photo is attached to several swings it names a single survivor — earliest arrival wins, which is the
right rule for counting a shot once (2026-09-06, P12). Asked from this side the same rule is wrong,
and the corpus proves it: **one screen photo is attached to three swings** — `2026-08-07-aaron1/1`,
`2026-08-09/2` and `2026-08-10/1`, whose face-on, down-the-line *and* shot-screen digests are all
identical, one physical swing uploaded three times while the upload path was being tested. Built on
that join, this tool told two of the three that their screen had never been read, while `get_swing`
returned their recorded flight in the same conversation. It resolves the photo the manifest names
instead, which is `api/app.py::swing_flight`'s own resolution and the half of P10 that
`loft_for_club` exists to expose.

This does **not** correct P13's reading of the same fact. That one is about `CorpusSwing`, which
groups by the face-on clip: those three swings share that too, so they are one corpus swing and the
photo key and the `swing:{ref}` fallback still partition this corpus identically.

### The 2026-09-06c seam, visible for the first time

P14 found that two of a flight's inputs live in **editable** artifacts and that nothing on disk can
see when one of them moves: `is_outdated` compares engine versions and `AnalysisState.inputs` hashes
the uploads, so declaring a 3 wood's loft changes what a stored shot flies and no check anywhere
fires. Every surface since has re-flown, which shows today's answer and hides the disagreement.

This tool reads both. `differs_from_recorded` sets the stored `flight_*` rows — read back through
`query.get_swing`, not a second reader of the artifact — beside the flown ones, at the four decimals
`engine.py` rounds to, and `recorded_reading` splits the two causes on `analysis_version`: an artifact
behind the current engine differs *because of the engine* and re-analysis is the repair; a **current**
artifact that differs is an input edited outside it, and saying "you swung differently" would be the
one reading that is certainly false. On the corpus today it is empty on all thirteen shots, which is
what it should be until someone edits a bag.
