"""When a shot's carry is a gross mishit, and the golfer's own verdict on it. [M16 P1]

Career mode pools every tagged shot's carry into one guarded mean (ADR-024). A topped 7 iron that
carries 20 yards is one of those samples, counted exactly like the real ones, and it drags "your 7
iron carries 150" down by a shot the golfer already knows was a mistake. This module is the rule
that spots such a shot and the enum that lets the golfer overrule it either way. ADR-028 is the
why.

**"Miss" is not the word.** `contracts/dispersion.py` uses *miss* for the shape of a golfer's
error distribution — a repeatable miss versus a scattered one. This is `mishit`, and the two never
share a name.

## Why a fraction of the median, and not the flight model

A genuine top prints a low ball speed *and* a short carry, and they agree: nothing in
`analysis/spin_solve.py` or `analysis/flight_infer.py` flags a carry that is physically consistent
with its own launch conditions, however far it is from what the golfer meant. The only thing that
says a 20-yard 7 iron was a mistake is that this golfer's *other* 7 irons carry 150. So the rule is
relative to the club's own history, it uses the median — robust to the very tops it is looking for
— and it is gated at the sample count below which there is no "history" to be an outlier of.

Stdlib + pydantic only (ADR-008). `analysis` and `storage` both sit downstream of this, so the
rule lives on the shared layer — the placement `count_metrics` and `narrowed_to` already have.
"""

from __future__ import annotations

import statistics
from collections.abc import Sequence
from enum import StrEnum


class MishitVerdict(StrEnum):
    """The golfer's own verdict on a shot, set through the per-swing repair route.

    `None` on a `SwingManifest` — the common case — means no verdict, and the automatic rule
    decides. A verdict here overrides that rule in either direction and is never inferred.
    """

    CONFIRMED = "confirmed"  # yes, a mishit — hold it out of the distance averages
    CLEARED = "cleared"  # no, a real shot — count it, whatever the automatic rule thinks


#: The measurements a mishit is withheld from, and the only ones. Ball speed, launch angle,
#: start-line offline, face-to-path and every pose checkpoint still count a mishit shot: the swing
#: was real and its mechanics are a fair sample, only its distance is meaningless. This is the
#: `untagged_swings` lesson (ADR-024) one metric in — an exclusion that is right for carry is not
#: licence to drop the shot everywhere.
MISHIT_EXCLUDED_METRICS: frozenset[str] = frozenset({"carry_distance_yds", "total_distance_yds"})

#: Distinct carry samples a club needs before the automatic rule will flag any of its shots. The
#: CENTER-claim floor from `contracts.baseline.DEFAULT_MINIMUM_N`, reused deliberately: below it a
#: club has no established carry, so there is nothing for a short shot to be an outlier *of*, and
#: the honest output is to flag nothing and let the golfer tag by hand.
MISHIT_MIN_CLEAN_SHOTS = 5

#: How far below a club's own median carry a shot has to fall to be an automatic mishit. 0.50 is a
#: judgment and it is the one number in this milestone a bay session is expected to move — see
#: `MISHIT_CARRY_FRACTION_PROVENANCE`.
MISHIT_CARRY_FRACTION = 0.50

MISHIT_CARRY_FRACTION_PROVENANCE = (
    "judgment, 2026-09-07 (M16 P1): half a club's own median carry is a topped or bladed shot, "
    "not the low edge of normal dispersion - a 7 iron that carries 150 does not carry 75 on a "
    "real strike. Deliberately loose: it removes only shots no average should contain and leaves "
    "heavier contact (a 90-yard chunk off a 150-yard club) for a manual verdict. No "
    "instrument-error evidence stands behind 0.50; revise from a bay session with labelled tops "
    "and duffs"
)


def mishit_carry_floor(carries: Sequence[float]) -> float | None:
    """Carry below which a shot of this club is a gross mishit, or ``None`` if too few samples.

    ``carries`` is the club's pooled carry set — one value per distinct shot photo, the same set
    the baseline means over — so the median here is the median the golfer sees, and the check is
    not circular: one or two tops in the sample barely move a median, which is the whole reason it
    is the median and not the mean.
    """
    if len(carries) < MISHIT_MIN_CLEAN_SHOTS:
        return None
    return MISHIT_CARRY_FRACTION * float(statistics.median(carries))
