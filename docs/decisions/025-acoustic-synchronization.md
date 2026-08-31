# ADR-025: Acoustic Synchronization — the Ball Strike as a Shared Clock

## Status
Accepted

## Date
2026-08-29

## Context

ADR-015 built the hand-held two-phone tier on one premise, stated in its §Context as a fact about
the hardware: **"There is no shared clock."** Two phones, two people, two moments of pressing
record, and nothing correlating the wall-clock timestamps in the two containers. Everything that
followed is downstream of it — the normalized `tau` axis, the soft/primary anchor split, the tempo
cross-check, `AlignmentQuality` itself.

The premise was about *timestamps*, and it is still true about timestamps. What it missed is that
the two phones are recording the same room. **Both microphones hear the ball being struck**, and
that is an event in the world rather than a number a phone wrote down.

ADR-015 saw this. Its Option C — audio cross-correlation on the ball strike — was **"not rejected
on merit… the strongest alternative here"**, parked in Consequences with the instruction *"Revisit
only with a concrete need."* This ADR is that revisit, and M10 supplied the need.

### The concrete need is a defect nothing inside one clip can see

M10 repaired the swing *windowing* and handed one defect forward: **the face-on top lands late.**
On the eleven bundles stored from 2026-08-23, the two views disagree about the downswing in the
same direction every time — face-on measures 0.183–0.267 s where down-the-line measures 0.367–0.484
s of the same swing. It costs three `tempo` readings that score and *fail* at 4.92, 6.08 and 6.09:1
against a 4.71 ceiling, and M10 P10's *As built* note says in as many words that those are **not
coaching truth**.

Every one of those failures has one root: **the two views infer the swing independently, from pose,
and nothing external adjudicates.** From inside the face-on clip nothing looks wrong —
`phases.py` reports the boundary as detected and is not wrong to. Two disagreeing durations,
measured against two independently inferred impacts, is two wrong anchors and one gap with no
arbiter.

### And a second defect, larger, found while checking the first

Decoding real clips and cross-correlating the two views' audio (docs/M11_ACOUSTIC_SYNC.md §E4)
showed that on four of the eleven bundles the **impact anchor** — the one this project's own
bake-off calls reliable to a median of one frame — is 5–7 frames out between the views, with
down-the-line consistently early. It is a partly different set of bundles from the late-top four,
and it includes one currently reported at the `full` tier. A container A/V bias, a variable frame
rate and sound travel were each measured and excluded as the cause.

### The one assumption ADR-015 could not check, checked

ADR-015 listed three objections to Option C. The first was that it *"assumes the upload path
preserves the audio track"*, and that was unknowable in 2026-08-07 because nothing had looked.
Measured 2026-08-29 by parsing every stored container directly: **30/30 clips carry `mp4a` audio at
48 kHz.** The assumption holds on this corpus and the upload path has never stripped a track.

Also found, and the reason the decode path needs a real demuxer rather than arithmetic on a sample
count: the audio edit lists are **non-uniform**. Most tracks carry a 2112-sample encoder-priming
offset (44 ms, 2.64 frames at 60 fps) and at least one carries none — so the bias does **not** cancel
between the two views, which is exactly the case a naive reader would get wrong in the direction
that looks correct.

## Options Considered

The alternatives are ADR-015's, re-read against evidence it did not have. Nothing new appears here;
what changed is which objections survive.

### Option A: Leave it — improve `phases.py` until the tops agree
Fix the detector where the bug is, rather than adding a subsystem.
- **Pros**: No dependency, no new tier, no version bump. `_DRAWDOWN_FLOOR` is a real precedent —
  M10 moved a face-on top 10 frames by tuning the descent rule alone.
- **Cons**: **Cannot close this one.** The failure is a golfer hovering at the top, which fragments
  the rising run `_top_and_impact` reads; the later fragment is taken and the downswing shortens.
  Every candidate fix trades one population of clips against another, and there is no held-out
  signal to choose with — which is the whole problem. Tuning a detector against the disagreement it
  produces is fitting to the symptom. Kept as the *right* place for the eventual repair (see
  Consequences), but it cannot supply the evidence to aim it.

### Option B: A shared visual fiducial — clap, LED, countdown
ADR-015's Option B, unchanged.
- **Cons**: Unchanged, and still decisive. A capture ritual that must never be skipped in a garage
  between shots will be skipped, and the first forgotten clap is an unalignable pair.

### Option C: Audio cross-correlation on the ball strike (chosen)
- **Pros**: Automatic, no ritual, no calibration. The strike is a sharp broadband transient and it
  reaches both phones. It composes with Option E rather than replacing it — it improves the tau=2
  anchor and leaves the warp exactly as it was.
- **Cons**: the three ADR-015 raised, now separable — see §Decision.

### Option D: 3D fusion by triangulation
Unreachable by construction, exactly as ADR-015 said. Hand-held phones have no stable extrinsics.
Audio gives a *clock*, not a *calibration*, so nothing here moves this any closer.

## Decision

**Take Option C, as a refinement of Option E rather than a replacement for it.** ADR-015's parked
bullet is hereby taken, on the terms that bullet set: *"it would compose with the chosen option as
a refinement."* That is precisely what was built.

### Which of ADR-015's three objections held

The ADR listed three, and they do not have the same answer. Stating them separately is the point:

- ***"it assumes the upload path preserves the audio track"*** — **did not hold.** Measured 30/30
  (§E1 of docs/M11_ACOUSTIC_SYNC.md). The assumption was correct and is now a checked fact rather
  than a hope; a clip that arrives without a track degrades to the pre-M11 path and says so.
- ***"recovers an offset only, so it does not survive a frame-rate difference"*** — **holds,
  conditionally, and the condition is checkable.** Two cameras filming one event in real time need
  no warp between them; the objection bites only for slo-mo or genuinely different rates. That is a
  per-bundle fact the containers report, not an assumption the design has to make — and the warp is
  still Option E's, which is immune to the difference by construction. What audio contributes is one
  anchor, and an anchor does not need a rate.
- ***"the strike is not the only sharp transient"*** — **holds, and is the hard part.** An indoor
  bay produces four transients per shot, measured in §E5: club–mat and club–ball merged into one
  onset, the ball hitting the screen ~145 ms later, and the simulator's own audio after that. The
  answer is ordering rather than amplitude: **the earliest transient in the window wins**, because
  the ball is the first sound a shot makes. "Take the loudest" takes the impact screen on *every*
  clip measured; "take the nearest to the pose impact" takes it on any clip whose pose impact was
  already right, which is seven of eleven — and no window separates the two, since the correction
  being made runs to 7.5 frames while the gap to reject starts at 5.

### `SYNCHRONIZED` is a new tier, and it sits *above* `full`

`AlignmentQuality`'s four tiers count **inferred** anchors: three pose estimates agree, or two, or
one. A measured tau=2 is not a fourth count — it is a different kind of claim, so it gets its own
name and overwrites whatever the count came to. One anchor that both phones *heard* is better
evidence than three anchors both phones guessed, and §E4 is the measurement that says so: four
bundles sat at `full` or `impact_only` while their down-the-line impact was 5.7–7.5 frames wrong.

The count is not lost when the tier is overwritten — every anchor `align_swings` refuses appends its
own note, so what the tier stops carrying, `SwingAlignment.notes` still says out loud.

`is_degraded` is deliberately **not** the same test as the one gating `caveats.ALIGNMENT_CAVEAT`.
A synchronized pair is not degraded; but it is still interpolated between anchors exactly as `full`
is, so *"synchronized on the ball strike, not on every instant"* remains the right sentence to a
reader.

### The clock is what makes the late top decidable

This is the consequence that reaches a golfer. With tau=2 pinned in both clips to one sound, the two
downswings stop being two independent estimates and become **two measurements of one interval in
real time**. The shorter one is then the late top — and that asymmetry is mechanical rather than
statistical: `_MAJOR_RISE_FRACTION` requires 80% of the largest rise in the clip before a run is a
candidate, so nothing in the rule can move a top *earlier*; only fragmentation can move one later.
Replayed over all eleven stored bundles, face-on is the late view on every one of the seven that
disagree, by 10–17 frames, with no exception.

**A contradicted top withdraws the scores timed from it** (`unscored.CROSS_VIEW_CONTRADICTED`).
That is ADR-010 §2 applied at a seam that did not previously exist: `tempo` is both halves of a
ratio the top defines, so a top the other view contradicts makes the number a comparison between
one instant that is right and one that is not. **Withdrawn rather than restated** — the alignment
can say what the ratio reads on the corrected top, and on bundle 2 that is a passing 2.35:1, but
writing it into a `CheckpointScore` would ship a number whose value came from the alignment and
whose band came from the engine, over frames `segment_phases` never agreed to. No score beats a
wrong one.

`contracts.checkpoints.CONTRADICTED_BY_A_LATE_TOP` is the set, and it holds `tempo` alone.
`hip_shift_at_top` reads the top too and is the obvious second member; it is deliberately not one,
because nothing has yet measured what a ten-frame shift does to a hip position sampled there, and
adding it on the strength of the name would be the guess this paragraph exists to refuse.

### Audio is an I/O-edge adapter, exactly like pose

`audio/` sits where `vision/` sits and obeys the same rule (ADR-008, ADR-007): a decode port behind
an extra, producing a contract shape, with nothing in `analysis/` importing it. The analysis core
receives **frame indices** — `face_on_strikes`, `down_the_line_strikes` — and never a waveform, so
it stays stdlib-only and a base install still runs every analysis test. `imageio-ffmpeg` is the
dependency and it is behind the new `audio` extra; `tests/api/test_pipeline_imports.py` is what
fails if it leaks.

### What this does **not** do

- **It is not calibration.** No down-the-line checkpoint becomes scoreable. The GolfDB corpus behind
  every band is face-on, so a DTL metric still has no reference population. ADR-015's Option D stays
  unreachable.
- **It does not detect the top.** Audio gives impact and only impact. It makes the top *decidable*
  by comparison; it does not find one.
- **It does not replace the `tau` axis.** ADR-015's Option E stands unamended. This improves one
  anchor feeding it.
- **It does not claim sub-frame accuracy.** Two phones at different distances from the ball hear it
  at different times — ~34 cm per millisecond, so a 3 m difference is ~9 ms. Comfortably inside one
  60 fps frame, and comfortably outside the sub-frame precision cross-correlation advertises.
  Frame-accurate sync is the claim; nothing beyond it has been measured.

## Consequences

- **ADR-015's §Context premise now has a documented exception**, recorded in its addendum #2. "There
  is no shared clock" remains true of *timestamps* and false of *events*. Nothing else in ADR-015 is
  revised: the warp, the anchor split and the tier vocabulary all stand.
- **`ANALYSIS_VERSION` 11 → 12, and scores move.** Three things change what gets scored: a
  microphone can overrule the window `select_swing` picked, a corrected impact re-cuts every
  down-the-line quantity resampled onto it, and a contradicted `tempo` is withdrawn so
  `overall_score` becomes a mean over one fewer checkpoint. A version-11 artifact **disagrees**
  with a version-12 one rather than merely lacking a field, which is the `2`/`3`/`10 → 11` shape of
  bump and the reason `is_outdated` has to keep the two out of one `PersonalBaseline`.
- **`tempo` becomes withdrawable, and `feedback` must cope.** A checkpoint that used to be present
  on every readable clip can now be absent on a clip that reads fine, with a reason no capture
  advice answers. `refilming_helps=False` is what carries that: a second clip of the same swing
  reproduces the same pause at the top.
- **A bundle with one microphone is unchanged, and that is the common case.** Half a pair is not a
  shared clock. A pair that heard nothing behaves exactly as it did before M11, on every path.
- **The repair still belongs in `phases.py`.** This ADR makes the late top *visible and decidable*;
  it does not fix the detector. What it buys the eventual fix is the thing Option A could not
  supply — a per-bundle label saying which view was wrong and by how many frames, which is a
  held-out signal to tune against rather than the symptom itself.
- **Sound is now a capture consideration.** Nothing about filming changes, but a bay loud enough to
  mask the strike degrades detection (not the warp — cross-correlation survives what absolute
  detection does not, which is why they are separate mechanisms).

## Addendum (2026-08-30): the second defect was the container after all [M11 P10]

**§Context's "And a second defect, larger, found while checking the first" is wrong about the
cause, and the sentence that is wrong is the one that excluded the container.** That section reports
the down-the-line impact anchor 5–7 frames early on four bundles, and lists "a container A/V bias"
among three explanations "measured and excluded" — worth only ≈22 ms, it says, against ≈110 ms
observed. The 22 ms was the *audio* priming asymmetry. The **video** track's edit list was never
weighed, and it is the whole of it: those four clips carry a leading empty edit of 105–125 ms, which
the audio decode honours and a frame counter does not, so a correct sample index landed 6–7 frames
late in video time. Verified against contact by eye, frame by frame, in both views of five bundles.

**What that changes, and what it does not.** The pose estimate on those clips was not early — it is
right to within a frame, so §Context's reading of §E4 as a *detection* problem does not survive.
The decision does not depend on it: the strike is still a shared clock, `SYNCHRONIZED` still means
both views heard the shot, and every option weighed above is weighed the same way. What the fix
moved was arithmetic one layer below this ADR (`api/pipeline.py` derives a frame from a sample, and
now subtracts the video's own presentation start), not the design it argues for.

**The general lesson is the one §Context already half-drew.** It says the decode path needs "a real
demuxer rather than arithmetic on a sample count", and the reason is stated in terms of the audio
edit lists. It is broader than that: **any two timelines in one container are only comparable once
you know what the decoder did with each of them.** The audio side got that treatment on 2026-08-29
and the video side did not until 2026-08-30, and the gap between those two dates is exactly the
defect. Details and the measurements in
[docs/M11_ACOUSTIC_SYNC.md](../M11_ACOUSTIC_SYNC.md) §Addendum, P10 and P11.

## References
- ADR-015 (hand-held two-phone capture & event-anchored alignment) — the ADR this takes the parked
  Option C of, and whose addendum #2 records the exception to its "no shared clock" premise.
- ADR-011 (camera synchronization & 3D fusion) — the fixed-rig tier, unaffected. Its route to real
  3D is not what this is.
- ADR-010 §2 (no score beats a wrong one) — the rule behind withdrawing a contradicted `tempo`
  rather than restating it.
- ADR-013 (clip-relative detection) — why a strike is converted from seconds into each clip's own
  frames rather than shared as an index.
- ADR-008 (project structure) / ADR-007 (decouple software from hardware) — why `audio/` is an
  adapter behind an extra and `analysis/` never sees a waveform.
- docs/M11_ACOUSTIC_SYNC.md — the phase plan, and §The evidence (§E1–§E5), which is where every
  measurement quoted above was taken.
- docs/M10_ALIGNMENT_ACCURACY.md — the milestone that handed the late top forward, and the source of
  the three failing `tempo` readings.
