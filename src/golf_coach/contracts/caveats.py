"""The standing caveats every consumer of a `SwingResult` has to carry. [M6]

Not a data shape, which makes this the one odd member of `contracts/`. It lives here anyway
because it is *about* the shapes in here, and because two modules need the identical text:
`mcp/server.py` hands it to an MCP client, `feedback/coach.py` hands it to the coaching call.
ADR-008 forbids either importing the other, so the alternative was two copies of load-bearing
prose — and prose that exists twice is prose that goes stale in one place. This repo has been
bitten by that three times (a planning doc that outlived its phase, a document count that went
stale twice); the fix each time was one source of truth.

Why the text exists at all: an LLM presents whatever it is handed as fact. Every number this
repo knows to be provisional — a clamped percentile, an unmeasurable checkpoint, an OCR'd shot,
an interpolated alignment — has to survive the trip out with its uncertainty attached, or it
arrives at a reader as a confident measurement of something nobody measured.

The field names quoted below are the vocabulary the shapes actually use, so anything rendering
a `SwingResult` for a model should label its output with the same words (`feedback/coach.py`
does). A caveat about `unscored` is useless next to a brief that calls it something else.
"""

from __future__ import annotations

#: What the two scores mean and why they are not interchangeable (ADR-009).
TWO_AXES = """\
Two independent axes, and they answer different questions. **Mechanics** come from pose
analysis of a face-on video: five checkpoints (tempo, head sway, finish balance, hip sway, hip
shift at top) scored against bands derived from hand-annotated tour swings, plus the percentile
the swing sits at in that population. **Outcome** comes from the launch monitor: club and ball
speed, launch, spin, carry. A swing can score well on one and badly on the other; say which you
are talking about."""

#: Attached to any alignment tier below FULL, which is the only tier that anchored on all three
#: instants and so the only one a reader may treat as synchronized throughout (ADR-015). Takes
#: the tier's own `summary` clause, so the sentence names *which* anchors held.
ALIGNMENT_CAVEAT = (
    "The two camera views were {summary}, not on every instant. Frame correspondence is exact at "
    "those anchors and interpolated between them, so do not describe the side-by-side video as "
    "synchronized throughout, and do not read fine timing differences off it."
)

#: The provisional-data warnings, one per way this repo can be honestly wrong.
READING_THIS_DATA_HONESTLY = """\
Reading this data honestly:

- `unscored` lists checkpoints that could not be measured, not ones that failed. They are
  excluded from `overall_score` rather than counted as zero, so a swing with an unscored
  checkpoint was judged on fewer fundamentals than one without. Say so if it comes up.
- `percentile` is informational and clamps at the band edges, so a failing checkpoint reports
  about 90 (or about 10, when it missed on the low side of a two-sided metric) whether it missed
  narrowly or hugely. A low percentile is therefore not by itself good news — read `passed` for
  the verdict and rank severity by `score`, never by percentile.
- **Less is not better on every checkpoint.** `tempo`, `hip_sway` and `head_stays_back` are
  two-sided (`one_sided` is false): too little hip travel fails just as too much does, because
  some lateral hip movement is the weight shift a swing needs, and a head that hangs back through
  impact fails as a head that drifts forward with the hips does. On the one-sided checkpoints —
  head sway, finish balance, hip shift at top — below the band is the good side. Read `one_sided`
  before describing a low number as a strength.
- `head_stays_back` is recorded as a **negative** number, and that is a frame of reference rather
  than a fault: its band lives in a right-handed camera frame where more negative means more
  head-behind-hips separation. Never read its sign as bad news, and never quote the raw value to a
  golfer — the checkpoint's own `message` states the magnitude in the right direction. This is also
  the one checkpoint that can be missing because nobody said who swung: when a swing has no golfer
  attributed, it appears in `unscored` rather than being scored on a guess.
- `needs_review` on a shot means its numbers were read off a photograph by OCR and the parse
  was flagged. Do not quote a flagged shot's figures as fact; say the reading is uncertain.
- `alignment_caveat`, when present, means the two camera views were anchored on some swing
  instants and interpolated between them. Do not describe the side-by-side video as
  synchronized throughout, and do not read fine timing differences off it.
- Only six fundamentals are measured, all from the face-on view, and all of them are lateral
  (side-to-side) or timing quantities. Spine angle, hip **rotation**, swing plane and club path
  are NOT measured here — they need a second calibrated view or club detection, neither of which
  exists yet. The hip checkpoints measure how far the hips travel sideways, never how far they
  turn, and never in which direction. Do not infer them from what is available.

Only a handful of sessions and shots are recorded so far. Do not describe a trend or a pattern
from the numbers in front of you — there is not enough of them for one, and any statement about
change over time has to come from something that counted the swings behind it."""

#: The discipline the career tools need, and **only** they do. [Career mode, step 6]
#:
#: Kept out of `READING_THIS_DATA_HONESTLY` rather than appended to it, because that block goes to
#: the coaching call as well, and `feedback/coach.py` writes about one swing. Instructions about
#: refusing a cross-session claim have nothing to act on there, and prompt text that cannot apply
#: is text that teaches a reader to skim the rest.
#:
#: **Why this exists at all.** Everything else in this file warns that a number is provisional.
#: This warns about the opposite failure: a number that is *absent on purpose*. Career mode's guard
#: withholds any statistic the sample size cannot support, and a withheld statistic is `None` —
#: while the evidence behind the refusal (`n`, `n_sessions`, the per-session counts) stays
#: populated, because that is what makes a refusal actionable instead of merely silent.
#:
#: That combination is exactly what an eager reader defeats. Handed "session A: 4 swings, session
#: B: 5 swings" and a withheld mean, a model can average them itself and narrate the trend the
#: guard just declined to make — the arithmetic is trivial and the numbers are right there. Nothing
#: in a payload can stop it. Saying so is the only control that exists.
READING_A_PERSONAL_HISTORY = """\
Reading one golfer's own history:

- A figure the sample size cannot support is **absent, not flagged**. `null` here means the guard
  refused the claim, and `withheld` says what it is waiting for. Report the refusal in words. Do
  NOT reconstruct the missing figure from the per-session counts sitting next to it — those counts
  are the evidence for the refusal, not a series to average.
- `not_established` is not "no" and not "you are fine". It means there is enough data to ask and
  the data does not settle it. An interval straddling a threshold is consistent with a real effect
  and with none.
- `bias` and `scatter` are two separate findings and the contrast between them is the whole point:
  a miss that repeats at the same size points at something set before the swing, a miss that moves
  points at timing and release. Never collapse the pair into one verdict, and never name a specific
  cause — grip, wrist angle, release and clubface are not measured here.
- A tour band describes a population of professionals, and `population_players` says how large it
  is for that metric — it is not the same for all of them. `outside` is a distance from that
  population, NOT a fault: holding an amateur to a tour p10-p90 fails everyone forever, which is
  why this system compares a golfer to themselves first. Read `outside_by` for the direction; on a
  one-sided magnitude like head sway or finish balance, below the band is the good side.
- `n` counts distinct swings, not swing directories. A session can score perfectly well and still
  contribute no samples, because its clips were re-uploads of a swing already counted elsewhere."""
