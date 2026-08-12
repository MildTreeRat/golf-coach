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
analysis of a face-on video: three checkpoints (tempo, head sway, finish balance) scored
against p10-p90 bands derived from hand-annotated tour swings, plus the percentile the swing
sits at in that population. **Outcome** comes from the launch monitor: club and ball speed,
launch, spin, carry. A swing can score well on one and badly on the other; say which you are
talking about."""

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
  about 90 (or about 10, when it missed on the low side of a two-sided metric like tempo)
  whether it missed narrowly or hugely. A low percentile is therefore not by itself good news —
  read `passed` for the verdict and rank severity by `score`, never by percentile.
- `needs_review` on a shot means its numbers were read off a photograph by OCR and the parse
  was flagged. Do not quote a flagged shot's figures as fact; say the reading is uncertain.
- `alignment_caveat`, when present, means the two camera views were anchored on some swing
  instants and interpolated between them. Do not describe the side-by-side video as
  synchronized throughout, and do not read fine timing differences off it.
- Only three fundamentals are measured, all from the face-on view. Spine angle, hip rotation,
  swing plane and club path are NOT measured here — they need a second calibrated view or club
  detection, neither of which exists yet. Do not infer them from what is available.

Only a handful of sessions and shots are recorded so far. Do not describe a trend or a pattern
from the numbers in front of you — there is not enough of them for one, and any statement about
change over time has to come from something that counted the swings behind it."""
