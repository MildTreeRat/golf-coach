# ADR-015: Hand-Held Two-Phone Capture & Event-Anchored Alignment

## Status
Accepted

## Date
2026-08-07

## Context

M7 records one swing on two hand-held iPhones — one face-on, one down-the-line — and needs to know
which frame of one clip shows the same swing instant as which frame of the other. Without that
there is no side-by-side replay, no way to put a down-the-line measurement beside a face-on one,
and no honest way to show a golfer their swing from two angles at once.

ADR-011 answered this question for the **fixed ELP rig** and its 2026-08-05 addendum explicitly
deferred the hand-held case to this ADR. Three facts about hand-held phones constrain the answer,
and all three are consequences of the capture setup rather than of any software choice:

**There is no shared clock.** Two phones, two people, two moments of pressing record. Wall-clock
timestamps in the containers are set by each phone independently, and nothing correlates them.

**There is no shared frame rate.** The clips measured for this decision report 59.959 and 59.916
fps — nominally identical phones, already disagreeing. iPhone slo-mo is far worse: it stores 120 or
240 fps capture behind a stretched playback rate, so `CAP_PROP_FPS` may not describe real time at
all (docs/M7_TWO_PHONE_SPIKE.md, Q3). Two people configuring two phones will not match settings.

**There is no calibration, ever.** Triangulation needs intrinsics *and* extrinsics — the cameras'
relative pose. Two phones held by two people, placed differently on every swing, have no stable
extrinsics to solve for. This is not a "later" item; it is unreachable by construction.

Clip lengths are also wildly unequal in practice. The first real pair recorded for this work runs
15.4 s face-on against 41.2 s down-the-line, and the longer clip contains practice swings the
shorter one does not.

## Options Considered

### Option A: Container / host timestamps (ADR-011's Option B)
Align by the timestamps each phone wrote.
- **Pros**: Free; no signal processing; already in the file.
- **Cons**: Solves nothing here. The two clocks are unrelated, so the offset is unknown; and
  because the *rates* also differ, even a known offset would drift. Slo-mo defeats it outright.

### Option B: A shared visual fiducial — clap, LED, or a countdown
Both cameras see one event; align on it.
- **Pros**: Genuinely accurate. Standard practice in multi-camera work.
- **Cons**: Requires a deliberate act at the start of **every** swing by two people who are also
  trying to play golf. The first forgotten clap produces an unalignable pair with no recovery. It
  buys an offset but still not a rate, so slo-mo remains broken. Rejected on ergonomics: this is a
  garage tool used between shots, and a capture ritual that must never be skipped will be skipped.

### Option C: Audio cross-correlation on the ball strike
Both phones record audio; the strike is a sharp broadband transient. Cross-correlate to recover the
offset.
- **Pros**: Automatic, needs no ritual, and genuinely precise — sub-frame in principle.
- **Cons**: Recovers an **offset only**, so it does not survive a frame-rate difference and does
  nothing for slo-mo — the very cases this ADR exists to handle. It also needs an audio decoding
  path the project does not have (OpenCV decodes no audio; this means ffmpeg or librosa, a new
  dependency in a repo that keeps heavy deps behind extras), and it assumes the upload path
  preserves the audio track. In an indoor bay the strike is not the only sharp transient — the ball
  hitting the screen a moment later is another. **Not rejected on merit**: it is the strongest
  alternative here and would compose with the chosen option as a refinement, which is where it is
  parked (Consequences, below).

### Option D: 3D fusion by triangulation (ADR-011's Phases 2–3)
- **Pros**: The measurements we actually want long-term — true spine angle, hip rotation, X-factor,
  kinematic sequence.
- **Cons**: Needs calibration that hand-held phones cannot provide. Unreachable, not deferred.

### Option E: Event-anchored piecewise-linear time warp (chosen)
Segment each clip **independently** with the existing `segment_phases()`, then align on the swing
instants each already produces. Define a normalized swing-time axis:

    tau = 0  motion start        tau = 1  top of backswing        tau = 2  impact

and map `tau -> frame` per clip by piecewise-linear interpolation, extrapolating past impact at the
downswing rate.
- **Pros**: Needs no clock, no ritual, no calibration, and no new dependency. Immune **by
  construction** to different frame rates, different clip lengths, different start moments and
  slo-mo, because every quantity is expressed in the swing's own time base rather than in seconds.
  Uses a detector already validated against 461 GolfDB clips. Purely offline and pure-functional.
- **Cons**: Only as good as the anchors. It inherits `segment_phases()`' error — and its failure
  modes, including the practice-swing problem. It is an alignment, not a synchronization: between
  anchors it assumes the two views progress through the swing proportionally, which is exactly true
  only if the phase instants are exactly right.

## Decision

**Option E.** Event-anchored piecewise-linear alignment on a normalized swing-time axis,
implemented in `analysis/alignment.py` over contracts in `contracts/alignment.py`. This is
ADR-011's Option C used **standalone** rather than as a refinement of Option B — the refinement
relationship the original ADR imagined presumed a clock that this capture tier does not have.

### Not all anchors are equal, and the code says so
From this repo's own bake-off (docs/M4_POSE_BAKEOFF.md): impact lands within a median of 1 frame,
top within 2, motion start within 7 — with 40% of clips over 10 frames and an outright fallback on
~14%. So **top and impact are primary anchors and motion start is a soft one**, used only when both
clips detected it independently *and* the two clips agree on the swing's tempo. Otherwise both
clips fall back to the same tour-median estimate, degrading the pre-top region symmetrically rather
than in one panel only.

`AlignmentQuality` carries this outward — `full`, `top_impact`, `impact_only`, `unaligned` — so a
consumer can say *"aligned on impact only"* instead of rendering two panels side by side and
letting the viewer infer a precision nobody measured.

### Tempo agreement is the cross-check
Frame rate cancels out of a backswing:downswing ratio, so two views of one swing must agree on it
however differently the phones were configured. When they do not, the alignment refuses the soft
anchor and says why. This is the guard against the failure mode that would otherwise be invisible:
the two clips locking onto **different swings** and producing a confident, plausible, wrong video.

### Multi-swing clips are handled by selection, not by cleverness
A phone clip usually contains practice swings; `segment_phases()` takes the earliest major descent
and will pick a practice one if it comes first. That is correct behaviour for the single-swing
corpus it was validated on and cannot be fixed by tuning. So `phases.candidate_downswings()`
exposes every descent it finds, and alignment takes an optional frame `window` per clip. The user
chooses; the tool does not guess. Note that N swings produce roughly N+1 descents — the hands
coming back down to address between swings is a descent like any other.

### `FrameBundle` and this alignment stay two different things
ADR-011's addendum left this open. They do not converge. `FrameBundle` pairs frames *within a
millisecond tolerance*, which presumes a common clock; the phone tier has none, and its
correspondence is a **continuous map**, not a set of paired frames. Building one type to serve both
would smuggle a clock assumption into the one tier that specifically lacks a clock. The fixed rig
gets `FrameBundle` if and when it is built; the phone tier gets `SwingAlignment`.

### This does not supersede ADR-011
ADR-011 remains correct for the fixed, calibrated ELP rig it was written about, including the route
to real 3D through software sync and then a hardware trigger. These are **two capture tiers that
coexist**: one trades 3D away for zero-setup portability, the other keeps 3D and pays for it in
setup. Reading this ADR as a replacement would quietly delete the only path the project has to
X-factor and kinematic sequence.

## Consequences

- **Down-the-line stays capture-and-align only.** Alignment gives frame correspondence, not depth.
  The three validated checkpoints remain face-on, scored by the unchanged `analyze_swing()`; the
  GolfDB corpus behind the benchmark bands is face-on, so DTL metrics have no reference population
  to be judged against regardless.
- **Alignment quality is a first-class output**, not an implementation detail. Any UI that renders
  the aligned video must render the quality with it.
- **fps is almost entirely unused**, which is the point. It appears only in the degenerate
  `impact_only` case, which has no second anchor to derive a rate from, and there it is flagged.
  A Q3 finding that "the reported fps is a lie" is therefore a *confirmation* of this design rather
  than a threat to it.
- **The alignment is only as good as `segment_phases()`.** Improving the detector improves the
  alignment for free; a detector failure on down-the-line footage (spike Q1, still open) degrades
  to the manual path rather than producing nonsense. Manual anchors are a **parameter** of
  `align_swings()`, not a branch inside it, so the fallback path exercises the same code.
- **Audio cross-correlation (Option C) is parked, not killed.** If sub-frame accuracy is ever
  needed *and* the clips are known to be same-rate, it composes cleanly: it would refine the
  offset within this warp rather than replace it. Revisit only with a concrete need — it costs a
  dependency and an assumption this design currently does not make.
- **Nothing here needs a server, storage, or a network.** Two JSON files in, an alignment and an
  MP4 out, on the base install plus `vision` to render.

## References
- ADR-011 (camera synchronization & 3D fusion) + its 2026-08-05 addendum, which deferred exactly
  this decision and asked the `FrameBundle` question answered above.
- ADR-013 (clip-relative detection) — the normalized axis is the same principle applied to time.
- ADR-008 (project structure) — contracts as the seam, analysis as a pure functional core.
- docs/M4_POSE_BAKEOFF.md — the anchor error figures the soft/primary split rests on.
- docs/M7_TWO_PHONE_SPIKE.md — Q1 (down-the-line anchors) and Q3 (fps) as they stand.
