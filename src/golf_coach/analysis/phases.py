"""Swing phase segmentation — pure, stdlib only. [M4-PoC]

Splits a `FrameKeypoints` timeline into the six `SwingPhase` spans by tracking the
**lead wrist** (`LEFT_WRIST`) vertical position in a face-on view (the canonical pose-camera
placement — see M1 findings / ADR-003 addendum). In image coordinates `y` grows *downward*,
so during the swing the wrist traces: rest (high `y`) → rises through the backswing (`y`
falls to a minimum at the top) → falls back through the downswing (`y` climbs) → impact near
address height → follow-through (rises again).

We only need three instants for the tempo checkpoint — **motion start**, **top of backswing**, and
**impact**.

**Top and impact are found together, as the two ends of the downswing.** The downswing is the
longest sustained stretch of the hands *coming down* (rising `y`), so we scan for near-monotone
rising runs and take the **earliest** one within half the size of the largest. Ordering carries the
decision: a full finish leaves the hands higher than they were at the top, and lost tracking after
the finish adds spurious excursions, so "the biggest descent" can easily be something that is not
the downswing — but nothing in a golf swing descends before the downswing does. That is also why
the rule needs no address baseline, no fps assumption, and no per-camera threshold.

An earlier version anchored on the global minimum of wrist `y` ("highest hands"). Validated against
GolfDB's hand-annotated events it mislocated the top by a **median of 26 frames**, always late,
failing on 80% of tour clips — it was finding the finish. The rule here reduces that to a median of
**2 frames** with a 7% failure rate; the residual cases are clips containing a practice swing before
the real one, which no amount of pose accuracy resolves. See docs/M4_POSE_BAKEOFF.md.

**Motion start** is then found *relative* to the top, by 2D wrist speed: walk backward and take the
takeaway to begin just after the last sustained *quiet* stretch. Using speed rather than wrist
height is what makes tempo believable — the early takeaway is near-horizontal, so a height rule
missed it, landed motion-start mid-takeaway, and collapsed tempo toward ~1:1 (M4 findings).

How long that quiet stretch must be is expressed as a fraction of the clip's own downswing
duration rather than as a frame count, which is what lets the same rule read a real-time clip and a
4x slow-motion one (M4-REF Phase B6, ADR-013). Motion start is by a wide margin the least accurate
of the three instants — median 7 frames against 2 and 1 — so it reports whether it actually found
anything: `_motion_start` returns a `detected` flag that rides on the ADDRESS and BACKSWING
segments, and `evaluate_tempo` drops its score rather than divide by an estimate.

Counting the downswing to ball contact matches how Tour Tempo defines it (ADR-010), so the
benchmark stays calibrated. Tempo depends only on start/top/impact *timing*, never on posture.
No numpy, no MediaPipe — just lists, so it runs on the base install (ADR-008).

**Expects smoothed input.** `engine.analyze_swing` runs `smoothing.smooth_keypoints` before
calling this, so the wrist `y` series is already denoised and the top/impact instants are
stable frame-to-frame — this is the real robustness win over the raw-landmark first pass
(which misread a jittery clip as ~1.1:1). It still works on raw keypoints, just noisier.

**Rejected (M4-REF, 2026-08-01): sub-frame parabola refinement of the top.** The textbook remedy
for a flat extremum is to fit a parabola near the `argmin` and take its vertex, on the theory that
the neighbouring samples pin the vertex better than the single lowest one does. It does not apply
here. The top of a golf swing is not a *symmetric* flat extremum — it is an asymmetric reversal,
and a symmetric-window fit is pulled toward whichever side has the shallower slope. Measured on our
two clips the lead wrist approaches the top at ~-0.006/frame and leaves it at ~+0.002/frame, and
the fit moved the top **+1 frame later**; on a synthetic swing with the asymmetry reversed it moved
it *earlier*. A correction whose sign depends on the local shape of the trajectory is a
data-dependent bias, not noise reduction — and it would differ between broadcast reference footage
and phone clips, which is precisely where we need the two to stay comparable. The raw `argmin` on
the smoothed series is left in place; how well it actually locates the top is measured against
GolfDB's ground-truth event labels rather than guessed at (see docs/M4_POSE_BAKEOFF.md).

HARDWARE-REVALIDATE: top/impact are pose-only *proxies* for ball contact. When club/ball
detection (M2) and launch-monitor timing (M3) land, validate them against real impact timing
and against the annotated overlay (see docs/M4_FUNDAMENTALS_PANEL.md).
"""

from __future__ import annotations

from typing import NamedTuple

from golf_coach.contracts.keypoints import FrameKeypoints, PoseLandmark
from golf_coach.contracts.swing import PhaseSegment, SwingPhase

#: The wrist to track, per camera view. Measured, not assumed — see M4_POSE_BAKEOFF §Phase F.
#:
#: A face-on camera sees the lead wrist for most of the swing (tracked in 83% of frames against
#: the trail wrist's 97%, but the *rule* does better on it: 9%/7% failure against 17%/10%). From
#: down-the-line the lead wrist is the **far** arm, occluded by the torso through the top, and is
#: tracked in only **39%** of frames — where the shipped rule fails on 30% of clips for top and
#: 35% for impact. On the trail wrist, which is nearer that camera, the same rule reaches
#: 7% and 2% — better than face-on manages.
#:
#: This reverses §Phase B7's "no view-aware landmark selection is warranted", which was measured on
#: one bay swing. Over 1,045 labelled GolfDB clips it is warranted, and it is genuinely per-view
#: rather than "the trail wrist is better".
#:
#: Both halves are public, and the lead one only recently: it is the default everywhere in this
#: module, so nothing in here has to name it. Outside it, `api.pipeline._auto_window` and
#: `scripts/align_swings.py` choose per view and so name both — and a pair where only one half can
#: be said by name is a pair whose other half gets written as a literal `PoseLandmark.LEFT_WRIST`
#: somewhere this module cannot see it drift.
LEAD_WRIST = PoseLandmark.LEFT_WRIST
TRAIL_WRIST = PoseLandmark.RIGHT_WRIST

# Frames whose lead-wrist visibility is below this are treated as unreliable; we hold the
# last good `y` rather than trust a low-confidence jump (MediaPipe convention).
_MIN_VISIBILITY = 0.5

# A rising run tolerates a dip of this fraction of the rise it has already accumulated before it
# is considered over. A real downswing is not perfectly monotone in a 2D track; testing for strict
# monotonicity shatters it into fragments and ends up scoring noise instead of the swing.
_DRAWDOWN_TOLERANCE = 0.25

# ...but a fraction of the run's *own accumulated* rise is near zero at the start of a run, so in
# the first few frames any tracking noise clears the bar and ends the run. That is not hypothetical:
# it is what made the top read late on real bay footage. A golfer who hovers at the top gives the
# lead wrist a long, nearly-flat stretch, and one 0.002 wobble upward mid-stretch was enough to
# split the descent in two. `_top_and_impact` then took the second fragment and put the top 10
# frames late — a 14-frame downswing where the truth was 24 (session 2026-08-09, swing 2). Every
# consumer inherits that: tempo read 0.43:1, and the alignment warp squeezed the other view 1.69x
# to make the two disagreeing downswings meet.
#
# So the drawdown must also clear an absolute floor before it is believed. The floor is a fraction
# of the clip's own vertical wrist range, which keeps it resolution-, framing- and fps-invariant the
# way ADR-013 requires — it is a floor on the *test*, not a change to the tolerance, and it is inert
# wherever the signal is already clean.
#
# Swept against the 461-clip GolfDB face-on corpus (`tune_phases.py`, Phase B7) paired per-clip
# against the floor-0 baseline. **Impact is untouched at every floor tried** — 0 clips move — which
# is what you would expect from a rule that can only change where a run *starts*. Top:
#
#     floor    med   mean   >10   changed  better  worse
#     0        2.0   10.56   46         0       0      0
#     0.010    2.0   10.58   46        21      10     11
#     0.012    2.0   10.58   46        24      12     12
#     0.014    2.0   10.63   47        27      13     14
#     0.020    2.0   10.66   47        32      14     18
#
# 0.012 is the largest floor that leaves the >10 tail count unmoved and the wins and losses evenly
# matched; past it the trade turns negative. The bay swing flips to the correct top at 0.010, so
# this clears that by a comfortable margin while costing 0.02 frames of mean on the corpus — well
# inside the noise of a distribution whose mean is 10.6 against a median of 2.
_DRAWDOWN_FLOOR = 0.012

# A rising run counts as a candidate downswing at this fraction of the largest rise in the clip.
# Tuned against GolfDB ground truth over the full 461-clip face-on corpus (see
# `docs/M4_POSE_BAKEOFF.md`): mean top error is flat at ~10.5 frames across 0.75-0.85 and rises on
# both sides, so this is the centre of a plateau rather than an argmin. An earlier 97-clip sweep
# put the optimum at 0.50; that was sample size, not signal.
_MAJOR_RISE_FRACTION = 0.80

# Motion start is velocity-anchored. The lead wrist is *still* at setup (and momentarily between
# waggle bobs) but moves continuously once the takeaway begins — including the early, near-
# horizontal part a wrist-*height* rule misses. So we measure 2D wrist speed and, walking back
# from the top, take the takeaway to begin just after the last sustained *quiet* stretch: a run of
# frames slower than `_MOTION_QUIET_FRAC` of the swing's peak wrist speed.
#
# `_MOTION_QUIET_FRAC` is a fraction of the clip's own peak, so it is already scale- and
# fps-invariant; the 461-clip sweep puts it on a plateau across 0.04-0.05.
#
# **The run length is not a frame count** (M4-REF Phase B6, ADR-013). It used to be
# `_MOTION_STALL_FRAMES = 4` — the one fps-dependent absolute left anywhere in the address path,
# and the reason this instant failed the way it did. A downswing is ~8 frames in a real-time clip
# and ~30 in a broadcast slow-motion one, so "4 quiet frames" meant something four times different
# across the two halves of the corpus, and a slow takeaway spends long stretches below any
# fraction of its own peak. The walk therefore stopped *mid-takeaway*: signed error median +4 with
# 66% of clips late, and 17 frames of median error on slow-motion clips against 5 on real-time.
# Expressing the run as a fraction of `(impact - top)` — the clip's own time base, measured from
# the two instants we locate accurately — cut median error 9 -> 8 and mean 27.2 -> 24.1.
_MOTION_QUIET_FRAC = 0.05
_MOTION_STALL_FRACTION = 0.25

# Floor for the quiet run. Two frames is the shortest stretch that can distinguish a dwell from a
# single smoothed sample; it binds only on very short real-time clips.
_MOTION_STALL_MIN_FRAMES = 2

# When the wrist never settles — continuous fidgeting through setup, or a golfer still walking in —
# there is no quiet run to find. This used to return frame 0, which is not a neutral answer: GolfDB
# clips carry a median 59 frames of pre-roll (p90 254), so it fired on 11% of clips and cost them a
# median of 31 frames. Falling back to the tour-median tempo ratio instead bounds the damage
# (median 9 -> 7, mean 27.2 -> 22.9 overall). The value is GolfDB's own median over 1,399 clips
# (ADR-012), not a book number.
#
# The segment is marked `detected=False` when this fires, because the estimate is circular for
# anything that divides by it: `evaluate_tempo` would report ~3.5:1 by construction. Posture
# checkpoints still use it, since they only need it to place a sampling window (ADR-013).
_FALLBACK_TEMPO_RATIO = 3.5

# Address is still the weakest instant by a wide margin (median 7 frames, 40% of clips over 10)
# and this does not fix that. It is intrinsically hard — GolfDB's own SwingNet reaches only 31.7%
# PCE here — because the takeaway onset is a gradual departure from stillness rather than a
# direction change like the top or a contact event like impact. Six alternative signal families
# were tried and all lost to lead-wrist speed; `scripts/golfdb/tune_address.py` keeps them
# runnable and docs/M4_POSE_BAKEOFF.md Phase B6 records why each fails. The honest ceiling marker:
# a rule using *no pose signal at all* scores a median of 11 frames, so what the wrist contributes
# here is real but small.

# Half-widths (in frames) of the transition window straddling the top of the backswing and
# of the impact window straddling the return to address height. Small, symmetric, heuristic.
_TRANSITION_HALF_FRAMES = 3
_IMPACT_HALF_FRAMES = 2

# Below this many frames there is no swing to segment.
_MIN_FRAMES = 6


def _lead_wrist_xy(
    keypoints: list[FrameKeypoints], wrist: PoseLandmark = LEAD_WRIST
) -> list[tuple[float, float]]:
    """Tracked-wrist `(x, y)` per frame, holding the last confident value through dim frames."""
    points: list[tuple[float, float]] = []
    last_good: tuple[float, float] | None = None
    for frame in keypoints:
        wrist_lm = frame.landmark(wrist)
        if wrist_lm.visibility >= _MIN_VISIBILITY or last_good is None:
            last_good = (wrist_lm.x, wrist_lm.y)
        points.append(last_good)
    return points


def _wrist_confident(
    keypoints: list[FrameKeypoints], wrist: PoseLandmark = LEAD_WRIST
) -> list[bool]:
    """Per-frame mask: was the tracked wrist actually seen, or is `_lead_wrist_xy` holding?

    `_lead_wrist_xy` carries the last confident position through dim frames, which is right for a
    continuous series to smooth but wrong as evidence of where the hands went. Held frames are
    excluded from run detection so a stretch of lost tracking cannot bound a descent — worth about
    1.5 frames of mean top error on the GolfDB face-on set (docs/M4_POSE_BAKEOFF.md).

    This read `LEAD_WRIST` in its body while taking a `wrist` argument, so a `TRAIL_WRIST` caller
    got trail-wrist positions masked by *lead*-wrist visibility — precisely inverted from behind,
    where the lead wrist is the occluded far arm. `segment_phases` has passed `wrist` here since
    the down-the-line view moved to the trail wrist; the effect was to mask a well-tracked series
    with the visibility of the arm it was chosen to avoid.
    """
    return [frame.landmark(wrist).visibility >= _MIN_VISIBILITY for frame in keypoints]


def _rising_runs(ys: list[float], confident: list[bool]) -> list[tuple[float, int, int]]:
    """Near-monotone stretches of *falling hands* as `(rise, start_frame, end_frame)`.

    `y` grows downward, so a rising `y` is the hands coming down — a descent of the club. The
    downswing is the largest such stretch in a normal swing; the takeaway and the follow-through
    run the other way. Each run ends when `y` gives back more than `_DRAWDOWN_TOLERANCE` of the
    rise it has accumulated **and** more than `_DRAWDOWN_FLOOR` of the clip's whole wrist range —
    the floor is what stops noise from ending a run in its first few frames, where the relative
    test is vacuous. Only confidently-tracked frames participate.
    """
    runs: list[tuple[float, int, int]] = []
    start = peak_at = -1
    peak = 0.0

    tracked = [y for y, ok in zip(ys, confident, strict=False) if ok]
    floor = _DRAWDOWN_FLOOR * (max(tracked) - min(tracked)) if tracked else 0.0

    for index, y in enumerate(ys):
        if not confident[index]:
            continue
        if start < 0:
            start, peak, peak_at = index, y, index
            continue
        if y >= peak:
            peak, peak_at = y, index
            continue

        rise = peak - ys[start]
        if rise > 0.0 and (peak - y) > max(_DRAWDOWN_TOLERANCE * rise, floor):
            runs.append((rise, start, peak_at))
            start, peak, peak_at = index, y, index
        elif y < ys[start]:
            # Still descending toward a lower turning point — restart from here.
            start, peak, peak_at = index, y, index

    if start >= 0 and peak > ys[start]:
        runs.append((peak - ys[start], start, peak_at))
    return runs


def _top_and_impact(ys: list[float], confident: list[bool], n: int) -> tuple[int, int]:
    """Locate the top of the backswing and impact as the ends of the downswing.

    Takes the **earliest** rising run within `_MAJOR_RISE_FRACTION` of the largest one, rather than
    the largest outright. That one word is what makes this correct on real swings: a full finish
    puts the hands *higher* than they were at the top, and after the finish tracking often degrades
    into large spurious excursions — either can produce a rise that rivals the true downswing. What
    they cannot do is happen *before* it. Ordering is the one piece of structure every golf swing
    has, and unlike a threshold it does not need calibrating per camera, per player, or per fps.

    Falls back to the old global-argmin behaviour only when no run is found at all (a clip with no
    detectable descent), where any answer is a guess anyway.
    """
    runs = _rising_runs(ys, confident)
    if not runs:
        top = min(range(n), key=ys.__getitem__)
        return top, max(range(top, n), key=ys.__getitem__)

    largest = max(rise for rise, _, _ in runs)
    major = [run for run in runs if run[0] >= _MAJOR_RISE_FRACTION * largest]
    _, top, impact = min(major, key=lambda run: run[1])
    return top, impact


class Downswing(NamedTuple):
    """One candidate downswing: the hands descending from `top` to `impact`."""

    top: int
    impact: int
    rise: float  # how far the tracked wrist fell, in normalized image units


def candidate_downswings(
    keypoints: list[FrameKeypoints],
    *,
    min_fraction: float = _MAJOR_RISE_FRACTION,
    wrist: PoseLandmark = LEAD_WRIST,
) -> list[Downswing]:
    """Every descent of the hands in the clip, earliest first. [M7 Phase 2]

    `segment_phases` locates *one* swing and is right to: it was validated against 461 GolfDB
    clips, each of which contains exactly one. A phone clip does not. Someone takes two practice
    swings, settles, and then hits — and `_top_and_impact`'s "earliest major run" rule, which
    exists to stop a *finish* being read as the top, will happily return the first **practice**
    swing instead. That failure is already documented at the top of this module as the residual 7%
    of GolfDB clips; on hand-held phone footage it stops being an edge case.

    This does not change that rule. It exposes the runs `_rising_runs` already finds so a caller
    can *see* how many swings are in the clip and choose one, instead of discovering by eye that
    the wrong one was picked. `analysis.alignment` uses it to warn when two clips of "the same"
    swing disagree, and `scripts/align_swings.py --list-swings` prints it.

    `min_fraction` is the share of the largest descent a run must reach to be listed. It defaults
    to the same `_MAJOR_RISE_FRACTION` `segment_phases` itself applies, so the *first* entry of the
    default listing is exactly the swing `segment_phases` would have chosen. Lower it to see the
    near-misses — a lazy practice swing often descends less far than the real one.

    `wrist` is the camera's question, exactly as it is in `segment_phases` — same name, same
    default, same meaning. It is here so a down-the-line caller can read the *same* landmark for
    the window and for the anchors: the window this listing feeds decides which frames get scored,
    and a window chosen on one wrist while the phases are segmented on the other describes two
    different swings (M10 §A1).
    """
    n = len(keypoints)
    if n < _MIN_FRAMES:
        return []

    xy = _lead_wrist_xy(keypoints, wrist)
    ys = [y for _, y in xy]
    runs = _rising_runs(ys, _wrist_confident(keypoints, wrist))
    if not runs:
        return []

    largest = max(rise for rise, _, _ in runs)
    threshold = min_fraction * largest
    return [
        Downswing(top=start, impact=peak, rise=rise)
        for rise, start, peak in runs
        if rise >= threshold
    ]


# How long a real downswing takes, in seconds — the discriminator that separates a golf swing
# from everything else a phone clip contains. This is a **selection** aid, not a measurement, and
# it is the one rule in this module expressed in seconds rather than in the clip's own time base
# (ADR-013): a downswing is ~0.2-0.3 s for every golfer at every frame rate, which is exactly what
# makes it usable to tell swings apart from setup moves.
#
# Measured on the four real bay clips (`--list-swings`, M7 Phase 2 footage):
#
#   real swings    0.23  0.38  0.40  0.42        <- one per clip, ground-truth confirmed on aaron-1
#   setup moves    0.48  0.50  0.50  0.50  0.53  <- the hands being lowered into address
#   rehearsals     1.57  3.10  3.37  6.60  ...   <- practice swings and idle motion
#   tracking junk  0.08                          <- 5 frames at the very end of a clip
#
# The upper bound sits in a 0.06 s gap (0.42 real against 0.48 decoy), which is *thin*. That is why
# `select_swing` always reports what it chose, always yields to an explicit window, and declines
# rather than guesses when nothing lands in the band.
#
# These numbers were read while the face-on top was landing late (see `_DRAWDOWN_FLOOR`), and the
# note that used to sit here blamed the two views' disagreement on the down-the-line lead wrist
# being the far, occluded arm. **That was backwards.** Measured on 2026-08-09 swing 2, the two
# wrists agree with each other on down-the-line (24 frames and 25) and it was face-on that read 14;
# with the floor in place both views land on 24. So the 0.45 ceiling is not accommodating a DTL
# bias — it is accommodating genuinely slow amateur downswings, which is what 0.40 s is.
#
# **Every one of those numbers came from a 60 fps clip.** A 30 fps recording of the same swing
# brackets the descent more coarsely and reads *longer* — the 2026-08-07 bundle's face-on view
# measures 0.60 s for its only swing. Widening the band to admit that would swallow the whole
# setup-move cluster at 60 fps, so the band stays where the evidence put it and the
# single-candidate case is handled separately instead (see `select_swing`).
_PLAUSIBLE_DOWNSWING_S = (0.15, 0.45)

# How far a descent may sit from the swing another camera already found, in seconds — the tolerance
# `select_matching_swing` matches on. It does two jobs: it admits a descent the band above rejects
# when the *other view* vouches for it, and it is the distance "nearest the reference" is measured
# with.
#
# Sized by sweeping it over the 15 stored bundles and watching which down-the-line descent each
# value picks, against the real swing recorded in M10 §A1 and P5's As-built table:
#
#   below 0.07    session 6 and 2026-08-10/2 take a post-impact descent, or decline
#   below 0.085   session 3 does too — it needs 0.084 (a 0.467s descent, 0.384s reference)
#   0.085-0.25    every bundle picks the same swing; the corpus cannot tell these apart
#   above 0.25    session 5 comes in, but only because a window that wide has stopped asserting
#                 anything: its two views measure the same swing 0.284s apart
#
# 0.12 sits inside that plateau, ~2 frames at 60 fps above the binding case. The lower edge is the
# hard one — miss it and the rule picks the wrong swing confidently rather than declining.
#
# Rejected: a *relative* tolerance, a share of the reference. The three requirements are 16-22% of
# their own references, so a 25% rule also fits this corpus. But what is being absorbed is
# `_top_and_impact` bracketing one descent a few frames early or late — 4 to 5 frames on these
# clips, whatever the swing lasts — which is a frame count and not a proportion of the swing.
_MATCH_TOLERANCE_S = 0.12

# How far a descent's impact may sit from a transient heard in the same clip and still count as
# ending at it, in seconds. This is the whole tolerance of the strike rule, and it is loose on
# purpose: the rule is not being asked which transient was the ball, only whether *something* was
# hit where this descent ended.
#
# Sized from the measurements in docs/M11_ACOUSTIC_SYNC.md that bound the error:
#
#   +/-0.125 s   the worst pose-impact error the corpus holds - down-the-line impact runs 5.7-7.5
#                frames early on four bundles (E4), and nothing says the sign cannot reverse on
#                footage not yet on disk
#   +0.145 s     the ball-to-screen gap (E5), so the nearest transient to a *correct* impact may
#                be the screen strike rather than the ball
#   +0.018 s     the detector's own onset convention, late by that much on average
#                (`audio/impact.py`)
#
# 0.20 s clears the worst of those with room to spare, and nothing it must reject is anywhere near
# it: the post-impact descents this rule exists to exclude sit 15-24 s past the swing on a
# down-the-line clip (M10 A2). There is no value between "covers the measurement error" and
# "admits a decoy" on this corpus, which is why this band can be loose where
# `_PLAUSIBLE_DOWNSWING_S` has to be tight.
#
# Rejected: an asymmetric window - a transient may only land *after* the impact, which is truer to
# the physics, since the crack cannot precede the contact. It is wrong about the *measurement*:
# E4's four offenders have the pose impact landing ~0.1 s after the audio, and an asymmetric rule
# would refuse exactly the bundles this milestone exists to repair.
_STRIKE_TOLERANCE_S = 0.20

# How much clip to keep around the chosen swing, in downswing-lengths before the top and after
# impact — the clip's own time base, so this reads a 30 fps clip and a 240 fps one alike
# (ADR-013). Shared with `scripts/align_swings.py --list-swings` so the window the listing prints
# and the window `select_swing` picks are the same arithmetic.
#
# The lead is sized from `_FALLBACK_TEMPO_RATIO`: a tour backswing runs ~3.5 downswings, and
# `_motion_start` then needs a stretch of *quiet* address before that to walk back to. 2 was
# enough to frame a swing for viewing (which is all Phase 2 asked of it) but not to measure one
# — at 2 the window lands inside the backswing and motion start goes undetected on two of the
# six real clips, which drops the tempo checkpoint and degrades the alignment to top-and-impact.
# Measured across leads 2-8 on all six: 5 is the smallest value that detects motion start on
# every clip while leaving top and impact exactly where they are without a window at all.
_WINDOW_LEAD = 5
_WINDOW_TRAIL = 3

# A floor under that lead, because `_WINDOW_LEAD` alone shrinks it exactly when it must not.
#
# The loop this closes: a short measured downswing shrinks the window, which removes the address,
# which loses the motion start, which sends the alignment to the `_FALLBACK_TEMPO_RATIO` estimate
# that multiplies that same short downswing by 3.5 (M10 §A4).
#
# **This one is in seconds and `_WINDOW_LEAD` is not, and that is the finding.** Measured over the
# 14 stored face-on clips: the lead a window needs before `_motion_start` can find a quiet run is
# 0.92-1.27 s — near-constant in real time, but 3x to 7x in downswing-lengths. It tracks the
# *backswing* (0.83-1.22 s on these clips), and an amateur's backswing runs ~0.8-1.2 s whatever
# their downswing does. So `_WINDOW_LEAD * downswing` under-reaches precisely on the fast
# downswings: sessions 4, 5 and 2 get 1.00, 0.92 and 1.08 s and go undetected, while session 11
# gets 1.67 s for a requirement of 0.92. The quantity is not expressible in the clip's own time
# base, so it is expressed in seconds and converted through the clip's own fps — the same
# deviation, for the same kind of reason, that `_PLAUSIBLE_DOWNSWING_S` above already carries.
#
# 1.5 s sits 0.23 s above the worst observed requirement (1.267 s) and 0.28 s above the longest
# measured backswing (1.217 s). It binds on 5 of the 14 clips and leaves top and impact where they
# were on all of them.
#
# Rejected: raising `_WINDOW_LEAD` to 7, which also fixes all three failures here with no top
# movement. It gives the *failing* clips the thinnest margins (1.28-1.52 s against a 1.17-1.27 s
# requirement — the multiplier shrinks the lead where it is needed most) while widening every
# already-working clip to 2.7-2.9 s of lead for nothing, which is that much more room for a
# rehearsal to enter the window on footage not yet on disk. That is the risk the block above was
# tuned against.
_MIN_ADDRESS_LEAD_S = 1.5

# How far a descent must fall, as a share of the clip's largest, to be considered a swing at all.
# Deliberately looser than `_MAJOR_RISE_FRACTION`, and it is not a nicety: on `aaron-1-back` the
# largest descent is a *bystander* at 0.417, so the default 0.80 threshold puts the cut at 0.334
# and the real swing (0.257) is not even a candidate. Duration is what identifies the swing here,
# so this rule's job is only to avoid discarding it before duration gets to look. Shared with
# `scripts/align_swings.py --list-swings` so the set a human chooses from and the set
# `select_swing` chooses from are identical.
CANDIDATE_MIN_RISE = 0.45


class SwingChoice(NamedTuple):
    """Which descent in a multi-swing clip is the actual swing, and why. [M7 Phase 4]"""

    window: tuple[int, int]
    downswing: Downswing
    candidates: list[Downswing]
    reason: str


def window_around(downswing: Downswing, *, fps: float | None = None) -> tuple[int, int]:
    """The `[start, end)` frame window holding one swing plus its takeaway, address and finish.

    With `fps`, the lead is floored at `_MIN_ADDRESS_LEAD_S` so a fast downswing cannot squeeze the
    address out of the window and take motion-start detection with it. Without it the lead is
    `_WINDOW_LEAD` downswings exactly as before: a clip whose frame rate was never recorded degrades
    to the old arithmetic rather than raising (ADR-013), the same way `alignment`'s `TOP_IMPACT`
    fallback keeps its no-fps path.
    """
    frames = max(1, downswing.impact - downswing.top)
    lead = _WINDOW_LEAD * frames
    if fps is not None and fps > 0.0:
        lead = max(lead, round(_MIN_ADDRESS_LEAD_S * fps))
    return max(0, downswing.top - lead), downswing.impact + _WINDOW_TRAIL * frames


def _struck(
    candidates: list[Downswing], strike_frames: list[int] | None, *, fps: float
) -> list[Downswing]:
    """The candidates whose impact lands on a transient heard in that same clip. [M11 P5]

    Evidence of a different kind from everything else in this module: every other rule here reads
    the pose stream and asks whether a motion *looks* like a swing, and this one asks whether a
    ball was hit. A practice swing has a whoosh and no crack.

    Frames, not samples, and frames in this clip's own numbering — which is why no clip-to-clip
    offset appears anywhere in this file. Each view is filtered against the transients heard in its
    own footage; putting the two clips on one clock is a separate question and a separate phase
    (M11 P6).

    **Empty means "audio has nothing to say", never "no swing".** `None` (nobody listened) and `[]`
    (the detector ran and heard nothing) both come back empty here, and both leave the remaining
    rules to judge the clip exactly as they did before audio existed. Telling those two apart
    matters, but one level up: a clip that was listened to and made no crack is worth a note
    (`api/pipeline.py`), while refusing every candidate on silence would throw a window away over a
    microphone.
    """
    if not strike_frames:
        return []
    tolerance = max(1, round(_STRIKE_TOLERANCE_S * fps))
    return [
        swing
        for swing in candidates
        if any(abs(swing.impact - frame) <= tolerance for frame in strike_frames)
    ]


def _lone_candidate_choice(
    pool: list[Downswing],
    *,
    fps: float,
    candidates: list[Downswing] | None = None,
    struck: bool = False,
) -> SwingChoice | None:
    """The "one candidate wins on its own" escape, shared by both selection rules.

    `None` unless exactly one descent survived. Both `select_swing` and `select_matching_swing`
    arrive here having filtered everything away — one on duration, the other on duration *and* a
    reference — and both then owe the same answer, so the branch lives once rather than twice.

    `select_matching_swing` needs it more, not less: a reference can reject a lone descent the band
    would have kept. On `2026-08-23/4` the only down-the-line descent measures 0.484s against a
    0.200s face-on reference, so neither rule admits it, and declining would throw away the window
    P5 won that bundle.

    `pool` is what survived; `candidates` is every descent found, and defaults to `pool` because
    until M11 P5 those were the same list. They part company when a ball strike has narrowed the
    field: the choice is then made among the struck descents while `SwingChoice.candidates` still
    lists all of them, because that listing exists for a human checking whether the right descent
    was taken and a filtered listing cannot answer that question.
    """
    if len(pool) != 1:
        return None
    only = pool[0]
    low, high = _PLAUSIBLE_DOWNSWING_S
    # Two different justifications, and the reason has to say which one it is. Without audio this
    # is "the band is a tie-break with no tie to break"; with it, the band has been *overruled* by
    # a measurement of the physical event the band only ever stood in for.
    lead, why = (
        ("1 descent ends at a ball strike, and its", "since a ball was struck there")
        if struck
        else ("1 descent, and its", "since there is nothing to choose between")
    )
    return SwingChoice(
        window=window_around(only, fps=fps),
        downswing=only,
        candidates=pool if candidates is None else candidates,
        reason=(
            f"{lead} downswing measures "
            f"{(only.impact - only.top) / fps:.2f}s rather than the usual "
            f"{low:g}-{high:g}s — taking it anyway, {why} "
            f"(top {only.top}, impact {only.impact})"
        ),
    )


def select_swing(
    keypoints: list[FrameKeypoints],
    *,
    fps: float | None,
    wrist: PoseLandmark = LEAD_WRIST,
    strike_frames: list[int] | None = None,
) -> SwingChoice | None:
    """Pick the real swing out of a clip that contains several. [M7 Phase 4]

    `segment_phases` takes the **earliest** major descent, which is right for the single-swing
    GolfDB corpus it was validated against and wrong for a bay clip: on all four real clips it
    picks a *setup move*, and that is not merely a framing problem — the window decides which
    frames get scored, so an unwindowed clip is graded on the wrong swing (whole-clip
    `aaron-1-front` scores 58/100 with tempo unscored; the actual swing scores 67/100).

    Four rules, in order:

    0. **A ball was struck here** (`strike_frames`, M11 P5). When the clip's audio has been
       listened to and a transient lands where a descent ends, the rules below judge *those*
       descents and no others. It runs first because it is the only rule here reading the
       physical event; every other one reads a proxy for it. It is also the only rule that can be
       absent: with `strike_frames` of `None` — no audio extra, no audio track, nobody asked —
       behaviour from rule 1 down is identical to what it was before audio existed, the same way
       `window_around` keeps its no-fps path.
    1. **Duration.** Keep only candidates whose downswing lasts a plausible time
       (`_PLAUSIBLE_DOWNSWING_S`). This is what does the work — it is uniquely correct on all four
       multi-swing bay clips, because setup moves cluster tightly at ~0.5 s and rehearsals run
       whole seconds.
    2. **Last, not first.** Among survivors take the latest: nobody takes a practice swing *after*
       hitting the ball. Applied on its own this rule is wrong on both down-the-line clips (the
       DTL phone keeps rolling 15-24 s past impact, on the busy side of the bay), which is why it
       runs second rather than first.
    3. **One candidate wins on its own.** If exactly one descent survived, take it whatever it
       measures. The duration band exists to *choose between* candidates; with nothing to
       choose between it is only a filter with no job, and applying it anyway throws away a
       perfectly good window — which is how a 30 fps single-swing clip (0.60 s, above the band
       derived from 60 fps footage) ended up scored over its whole length including dead air.
       This can never be worse than declining: `segment_phases` would pick that same lone descent
       regardless, so the choice is only whether to measure it in isolation or with the rest of
       the clip mixed in.

    Returns None — never a guess — when it has no pick to offer: no fps to read durations with
    (legacy keypoints files record `clip=None`, and a duration in seconds is meaningless without
    one), no descent at all, or several descents and none plausible. A caller that gets None
    falls back to `segment_phases`' own choice and should show the candidate listing, which
    prints the same durations this rule judged on.

    Note what this deliberately does **not** claim: a plausible downswing duration is evidence
    about *which descent is a swing*, not evidence that two clips show the *same* swing. Two other
    things speak to that. `alignment.align_swings`' tempo cross-check does, after the fact, and it
    does not fire in every case — it is skipped once the soft anchor has already been refused for
    another reason. `select_matching_swing` below does it at selection time instead: given one
    view's downswing it takes the descent in the other view that matches, which makes it the only
    rule here whose evidence is a *pair* of clips.

    `wrist` mirrors `segment_phases`: the default is the lead wrist and is what every stored
    window was picked with, and a down-the-line caller passes `TRAIL_WRIST` so the window and the
    anchors read one landmark. Choosing the swing on the far, occluded arm is not a near miss —
    on session 10 the lead wrist measures a 9.7 s "downswing" and windows the whole clip, while
    the trail wrist finds the swing at 0.40 s (M10 §A1).
    """
    if fps is None or fps <= 0.0:
        return None

    candidates = candidate_downswings(keypoints, min_fraction=CANDIDATE_MIN_RISE, wrist=wrist)
    struck = _struck(candidates, strike_frames, fps=fps)
    # `or candidates`, not `if strike_frames is not None`: a strike list that lands near no
    # descent leaves the duration rules judging the whole field. A transient nobody's swing ends
    # at is evidence about the detector or the framing; it is not evidence against every
    # candidate at once, and treating it as such would decline a clip over a stray noise.
    pool = struck or candidates
    low, high = _PLAUSIBLE_DOWNSWING_S
    plausible = [swing for swing in pool if low <= (swing.impact - swing.top) / fps <= high]
    if not plausible:
        return _lone_candidate_choice(pool, fps=fps, candidates=candidates, struck=bool(struck))

    chosen = plausible[-1]
    duration = (chosen.impact - chosen.top) / fps
    found = f"{len(candidates)} descent(s)"
    if struck:
        found += f", {len(struck)} ending at a ball strike"
    if len(plausible) == 1:
        reason = (
            f"{found}, 1 with a plausible downswing "
            f"({duration:.2f}s) — top {chosen.top}, impact {chosen.impact}"
        )
    else:
        reason = (
            f"{found}, {len(plausible)} with a plausible downswing; took "
            f"the last ({duration:.2f}s, top {chosen.top}, impact {chosen.impact}). The clip "
            "holds more than one real-looking swing — confirm with --list-swings"
        )
    return SwingChoice(
        window=window_around(chosen, fps=fps),
        downswing=chosen,
        candidates=candidates,
        reason=reason,
    )


def select_matching_swing(
    keypoints: list[FrameKeypoints],
    *,
    fps: float | None,
    reference_downswing_s: float,
    wrist: PoseLandmark = LEAD_WRIST,
    strike_frames: list[int] | None = None,
) -> SwingChoice | None:
    """Pick the descent that matches a swing already chosen in the *other* view. [M10 P7]

    `select_swing` judges a clip alone, and on a down-the-line clip that is not enough: the phone on
    the busy side of the bay keeps rolling 15-24 s past impact, so "the last plausible descent" is
    routinely something that happened after the ball was gone. Face-on is the reliable view — it
    picked a sane swing on 11 of 11 stored clips — so once *its* swing is chosen the other view's is
    no longer a free choice. Sessions 3 and 6 and the 2026-08-07 clip are this shape exactly:
    post-impact descents of 0.233, 0.183 and 0.267 s standing in for real swings of 0.467, 0.484 and
    0.417 s (M10 §A2).

    Four rules, in order, and rule 0 is `select_swing`'s:

    0. **A ball was struck here** (`strike_frames`, M11 P5). It is worth more on this side than on
       the one it was written for: what makes a down-the-line clip hard is the descents that
       happened *after* the ball was gone, and what separates them from the swing is exactly that
       the ball had already been hit.
    1. **Plausible, or vouched for.** Keep a candidate whose downswing lands in
       `_PLAUSIBLE_DOWNSWING_S` **or** within `_MATCH_TOLERANCE_S` of the reference. The second
       branch is not a wider band, it is a different kind of evidence — and it is what admits the
       0.450 s and 0.467 s descents on `2026-08-10/2` and session 3, which miss the band by 0.0003 s
       and 0.017 s while another camera is looking straight at the same swing (M10 §A3).
    2. **Nearest the reference, not last.** `select_swing`'s rule 2 is the one that fails on these
       clips; its docstring above records why "last" is right when a clip is judged alone. Here that
       rule survives only as the tie-break, for candidates equally far from the reference.
    3. **Decline rather than guess** (ADR-013) when nothing survives — except that a lone candidate
       still wins, for the reason `_lone_candidate_choice` records.

    Returns `None` without a frame rate, and without a positive reference: a duration in seconds is
    meaningless without the first and this rule has no question to ask without the second. A caller
    that gets `None` falls back to `select_swing`, which judges this view on its own — which is also
    what it should do when the reference view itself declined, since a reference nobody measured is
    not one to match against.
    """
    if fps is None or fps <= 0.0 or reference_downswing_s <= 0.0:
        return None

    candidates = candidate_downswings(keypoints, min_fraction=CANDIDATE_MIN_RISE, wrist=wrist)
    struck = _struck(candidates, strike_frames, fps=fps)
    pool = struck or candidates
    seconds = [(swing.impact - swing.top) / fps for swing in pool]
    low, high = _PLAUSIBLE_DOWNSWING_S
    matched = [
        (swing, duration)
        for swing, duration in zip(pool, seconds, strict=True)
        if low <= duration <= high
        or abs(duration - reference_downswing_s) <= _MATCH_TOLERANCE_S
    ]
    if not matched:
        return _lone_candidate_choice(pool, fps=fps, candidates=candidates, struck=bool(struck))

    # `min` keeps the first of equals, so reversing hands a tie to the *later* candidate — rule 2
    # of `select_swing`, demoted to the tie-break it is safe as.
    chosen, duration = min(
        reversed(matched), key=lambda pair: abs(pair[1] - reference_downswing_s)
    )
    admitted = (
        "in band" if low <= duration <= high else f"outside the usual {low:g}-{high:g}s band"
    )
    found = f"{len(candidates)} descent(s)"
    if struck:
        found += f", {len(struck)} ending at a ball strike"
    return SwingChoice(
        window=window_around(chosen, fps=fps),
        downswing=chosen,
        candidates=candidates,
        reason=(
            f"{found}, {len(matched)} matching the "
            f"{reference_downswing_s:.2f}s reference from the other view; took the nearest "
            f"({duration:.2f}s, {admitted}, off by "
            f"{abs(duration - reference_downswing_s):.2f}s — top {chosen.top}, "
            f"impact {chosen.impact})"
        ),
    )


def _wrist_speed(xy: list[tuple[float, float]]) -> list[float]:
    """Per-frame 2D lead-wrist speed (frame-to-frame displacement); `0.0` at the first frame."""
    speeds = [0.0]
    for (x0, y0), (x1, y1) in zip(xy, xy[1:], strict=False):
        speeds.append(((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5)
    return speeds


def _motion_start(xy: list[tuple[float, float]], top: int, impact: int) -> tuple[int, bool]:
    """Motion start: the frame the sustained takeaway begins, walking back from the top.

    Returns `(frame, detected)`. `detected` is False when the wrist never settles and the frame is
    the bounded estimate described at `_FALLBACK_TEMPO_RATIO` rather than something found in the
    signal — callers that divide by this boundary must drop their result instead of using it.

    The lead wrist is *still* at setup (and momentarily between waggle bobs) but moves continuously
    once the takeaway begins. We measure 2D wrist speed and walk backward from the top; the
    takeaway begins just after the last **quiet** stretch — a run of frames slower than
    `_MOTION_QUIET_FRAC` of the swing's peak wrist speed. Requiring a *run* keeps a single slow
    smoothed frame mid-takeaway from ending it early, and using speed (not wrist height) catches
    the near-horizontal early takeaway an earlier height rule missed.

    **The run length scales with the clip, not with the frame rate.** It is
    `_MOTION_STALL_FRACTION` of the downswing duration `(impact - top)` — the clip's own time base,
    taken from the two instants we locate to within 2 and 1 frames. That is what makes this rule
    read a 240fps phone clip and a broadcast slow-motion replay the same way; see ADR-013.

    Needs `top > 0` and reads cleanly because `engine` smooths first.
    """
    if top <= 0:
        return 0, True
    speeds = _wrist_speed(xy)
    peak = max(speeds[1 : top + 1], default=0.0)
    if peak <= 0.0:
        return 0, True
    quiet_threshold = peak * _MOTION_QUIET_FRAC
    downswing = max(impact - top, 1)
    stall = max(_MOTION_STALL_MIN_FRAMES, round(_MOTION_STALL_FRACTION * downswing))

    quiet = 0
    for i in range(top, -1, -1):
        if speeds[i] < quiet_threshold:
            quiet += 1
            if quiet >= stall:
                # First moving frame above the quiet run = takeaway start. Clamped to the top,
                # which a long stall near the top can otherwise overshoot.
                return min(i + quiet, top), True
        else:
            quiet = 0

    return max(0, top - round(_FALLBACK_TEMPO_RATIO * downswing)), False


def _segment(
    phase: SwingPhase, start: int, end: int, ts: list[float], detected: bool = True
) -> PhaseSegment:
    return PhaseSegment(
        phase=phase,
        start_frame=start,
        end_frame=end,
        start_ms=ts[start],
        end_ms=ts[end],
        detected=detected,
    )


def segment_phases(
    keypoints: list[FrameKeypoints], wrist: PoseLandmark = LEAD_WRIST
) -> list[PhaseSegment]:
    """Segment a keypoint timeline into the six swing phases (in canonical order).

    Returns an empty list for a clip too short to contain a swing. The returned segments are
    contiguous and their frame indices are monotonic non-decreasing, so a consumer can read
    phase timings straight off the boundaries.

    **`wrist` is the camera's question, not the golfer's.** It defaults to the lead wrist, which is
    right for face-on and is what every band and every stored analysis was produced with. A
    down-the-line caller should pass `TRAIL_WRIST`: from behind, the lead wrist is the far arm and
    is tracked in 39% of frames, where this rule fails on a third of clips. See `TRAIL_WRIST`.
    """
    n = len(keypoints)
    if n < _MIN_FRAMES:
        return []

    ts = [frame.timestamp_ms for frame in keypoints]
    xy = _lead_wrist_xy(keypoints, wrist)
    ys = [y for _, y in xy]
    confident = _wrist_confident(keypoints, wrist)

    # Top and impact are found together, as the two ends of the downswing (see `_top_and_impact`).
    top, impact = _top_and_impact(ys, confident, n)

    # Motion start: the frame the sustained takeaway begins (see `_motion_start`) — anchored on 2D
    # wrist speed so the near-horizontal early takeaway isn't missed and a waggle isn't mistaken
    # for the backswing. `found` is False when the wrist never settled and the boundary is an
    # estimate; it rides on the two segments that boundary defines.
    motion_start, found = _motion_start(xy, top, impact)

    # Bracket a small symmetric window around the top (transition) and after impact, then
    # clamp everything into a monotonic, non-overlapping boundary chain.
    b0 = 0
    b1 = motion_start
    b2 = max(b1, top - _TRANSITION_HALF_FRAMES)
    b3 = min(impact, top + _TRANSITION_HALF_FRAMES)
    b3 = max(b3, b2)
    b4 = max(impact, b3)
    b5 = min(n - 1, b4 + _IMPACT_HALF_FRAMES)
    b6 = n - 1
    b5 = max(b5, b4)

    return [
        _segment(SwingPhase.ADDRESS, b0, b1, ts, found),
        _segment(SwingPhase.BACKSWING, b1, b2, ts, found),
        _segment(SwingPhase.TRANSITION, b2, b3, ts),
        _segment(SwingPhase.DOWNSWING, b3, b4, ts),
        _segment(SwingPhase.IMPACT, b4, b5, ts),
        _segment(SwingPhase.FOLLOW_THROUGH, b5, b6, ts),
    ]
