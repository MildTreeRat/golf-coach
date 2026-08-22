"""One golfer's history, cut by club. [M9 P15]

`contracts/club_profile.py` declared the shape a per-club answer comes back in. This fills it, and
**it is deliberately short** — every statistic below was already sealed by the step that was allowed
to make it. Career mode built the guard (`analysis/baseline.py` will not state a mean without the
samples for one), the discriminator (`analysis/dispersion.py` separates a repeatable miss from a
scattered one) and the honest counting (`storage/corpus.py`); M9 P13 added the club clause. What was
left was the assembly, and if this module ever grows past that, something downstream is being
reimplemented one layer up.

## Narrowing first is the whole correctness argument

Every club's numbers come from `corpus.narrowed_to(club=...)` and never from the corpus itself.
Handing `build_baseline` the whole bag would let a club hit twice clear the five-sample floor on the
*bag's* `n` — a confident mean carry for a 7 iron, built out of eight driver shots. That is exactly
the printed-`n`-describes-a-different-set failure the narrowing recomputes its counts to prevent,
arriving one layer further out, and it is the reason the filter is called per club rather than the
swing list being sliced here.

## Why the filter lives on the contract

`analysis` may import only `contracts` (ADR-008), and the filter began life in `storage/corpus.py`.
Rather than reach upward or grow a second copy, M9 P15 moved it onto `CareerCorpus.narrowed_to` —
`CorpusSwing.artifact_key` is the precedent, a rule both sides need living on the shape both sides
hold. `storage.corpus.narrow_to` stays as the shell-facing spelling and delegates.
`tests/analysis/test_club_profile.py` pins this module's imports statically, because the convenient
one-line `from golf_coach.storage.corpus import narrow_to` is a single keystroke away and would not
fail anything at runtime.

## Declared and hit are two questions, so the union is the answer

A club gets a profile if it has been hit **or** is in the bag. Neither alone is right: profiling
only what was hit loses the new 5 wood with no shots on it, and profiling only the declared bag
loses the club that has since left it and still has real history. `BagProfile` derives
`clubs_used` and `clubs_declared` off the pair for that reason, and this is where the pair is set.

## A bag change is caveated, never split

`BagEntry.recorded_at` is the only thing on disk that can see a club being replaced, and a club
replaced under the same name pools two physical clubs into one carry average. P16 says so in a
sentence and leaves every statistic standing. Splitting the history at that date instead — joining
each stored shot to the stint that hit it — *is* ADR-024's deferred bag entry versioning, which
`contracts/bag.py` is explicit is not being promised: nothing reads `Bag.retired`, and a seam
inferred from one timestamp would silently halve an `n` this repo spent career mode learning to
count honestly. So the loose reading wins, for `SESSION_DRIFT_FACTOR`'s reason one module over: an
entry recorded late for a club that never changed is the likelier case, and being wrong about it
has to cost a sentence rather than a verdict.

Pure functions over contracts, base install only. No I/O — the corpus and the bag both arrive
assembled, from `storage.corpus.read_corpus` and `storage.bag_store.BagStore.get` respectively.
"""

from __future__ import annotations

from collections.abc import Sequence

from golf_coach.analysis.baseline import build_baseline
from golf_coach.analysis.dispersion import build_dispersion
from golf_coach.contracts.bag import Bag, BagEntry
from golf_coach.contracts.career import CareerCorpus, CorpusSwing
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.club_profile import BagProfile, ClubProfile


def build_bag_profile(corpus: CareerCorpus, bag: Bag | None = None) -> BagProfile:
    """Every club this golfer has hit or declared, with what its history supports saying.

    An empty corpus and no bag is an empty `BagProfile` rather than an error — "this golfer has no
    swings and has declared no clubs yet" is a real answer and the first one every golfer has, the
    same posture `read_corpus` and `build_baseline` already take.

    `bag` is optional because a golfer who has never declared one still has shot history worth
    profiling; it costs them the loft and nothing else. Passing `None` is not a degraded mode.
    """
    hit = {swing.club for swing in corpus.swings if swing.club is not None}
    declared = set(bag.entries) if bag is not None else set()

    # Walked off `ClubId`, never sorted and never taken from `bag.entries`' insertion order (R6).
    # Declaration order *is* bag order — `contracts/club.py` says so and `Bag.club_ids` makes the
    # same walk — and `BagProfile._clubs_are_in_bag_order` is what goes red if this is forgotten.
    # Alphabetically a 3 wood lands between the hybrids, which is not a bag anyone recognises.
    clubs = tuple(club for club in ClubId if club in hit or club in declared)

    return BagProfile(
        player_id=corpus.player_id,
        clubs=tuple(_profile_for(corpus, club, bag) for club in clubs),
        # Read off the **whole** corpus. A narrowed one reports 0 by construction, so taking it
        # from anywhere inside the loop would silently erase the one number on this model that
        # describes the history no profile above it can see.
        untagged_swings=corpus.untagged_swings,
    )


# --------------------------------------------------------------------------------------
# Internals
# --------------------------------------------------------------------------------------


def _profile_for(corpus: CareerCorpus, club: ClubId, bag: Bag | None) -> ClubProfile:
    """One club: the physical club, the evidence, and what that evidence supports.

    Private, and should stay that way until something needs one club without the bag around it.
    `BagProfile.profile_for` is the lookup P17's `--club 7i` and P18's `get_club_profile` both use,
    so a second public entry point here would be an abstraction with no caller (R11).

    `caveats` carries one sentence and only ever qualifies — see `_bag_changed_caveats`. Nothing
    here withholds a statistic on the strength of a bag entry's date, which is the whole reason the
    caveat is a string on the profile rather than a `WithheldClaim` inside its metrics.
    """
    narrowed = corpus.narrowed_to(club=club)

    # The live entry only — `Bag.retired` is deliberately not consulted. `contracts/bag.py` states
    # that the shelf is *retention*, not the bag-entry versioning ADR-024 defers, and that nothing
    # reads it; joining stored shots to the stint that hit them is a decision for the ADR rather
    # than something to infer here. So a club that has left the bag keeps every statistic and loses
    # only its loft, which is the split `clubs_used` / `clubs_declared` exists to express.
    entry = bag.entries.get(club) if bag is not None else None

    return ClubProfile(
        club=club,
        bag_entry=entry,
        # Exact today because `Bag._live_and_retired_are_separated` pins `entries` to live clubs
        # only, and P16 did not change that — it reads the live entry's `recorded_at` and nothing
        # off the shelf. It stops being exact the day something hands `bag_entry` a retired stint,
        # which is the divergence `ClubProfile.in_bag` documents, and this line is where it has to
        # be answered rather than left to follow along.
        in_bag=entry is not None,
        n_swings=narrowed.distinct_swings,
        n_shots=narrowed.distinct_shots,
        n_sessions=narrowed.distinct_sessions,
        metrics=build_baseline(narrowed).metrics,
        # This builds a *second* baseline over the same narrowed corpus, and that is not waste to
        # optimise away: `build_dispersion` takes a corpus rather than a baseline on purpose, so
        # that its guarded statistics and its raw per-session samples provably describe the same
        # read. Handing it one built here would be the seam through which one club's spread could
        # be paired with another's sessions. The cost is a second pass over a swing list this repo
        # counts in tens.
        dispersion=build_dispersion(narrowed).metrics,
        # `narrowed.swings`, never `corpus.swings`. The sentence counts *this club's* history, and
        # the module docstring's first section is the same property one layer up: the whole bag's
        # swing list attached to one club produces a caveat that is wrong about its own subject
        # while looking exactly like a working one.
        caveats=_bag_changed_caveats(entry, narrowed.swings),
    )


def _bag_changed_caveats(entry: BagEntry | None, swings: Sequence[CorpusSwing]) -> list[str]:
    """Say when some of these swings predate the bag entry they are being pooled under.

    A club replaced under the same name makes the shots before and after it two different physical
    clubs in one average, and `recorded_at` is the only mark on disk where that seam could be. This
    **adds a sentence and removes nothing** — `contracts/bag.py` and `contracts/club_profile.py`
    both state the posture on the fields themselves, and it is `SESSION_DRIFT_FACTOR`'s: a loose
    judgment that only ever caveats is one whose failure costs a sentence, where the same judgment
    wired to a refusal would delete real history every time an entry was simply recorded late.

    **There are two sentences, because two different things are true.** With swings on both sides
    of the date, one average holds two physical clubs and the mixed form says so. With every swing
    on the early side nothing is pooled at all — there is one population of unknown provenance, and
    what is in doubt is whether the entry's make, model and loft describe the club that hit any of
    it. The second is the *common* case, not the exotic one: nothing writes a bag until M9 P19, so
    the day a golfer first declares one, every club they own lands there at once. A caveat that is
    wrong on the case it fires on most is one people learn to skip on the case where it matters.

    **The mixed form names the proportion as well as the date**, which is one more than the phase
    list asked for. The sentence is permanently true once it fires — the earliest swing never
    moves — so a bare date would read the same on the day a club is declared and a year later. A
    proportion deflates on its own as post-declaration history accumulates, and "2 of 40" is a
    different situation from "2 of 4" in the only way a reader can act on. The all-predate form
    drops it for the obvious reason: there is no proportion when every swing is on one side.

    Strictly `<`: an entry recorded in the same instant as a swing *was* the club that hit it.

    No threshold constant, deliberately. `SESSION_DRIFT_FACTOR` is the precedent for the posture,
    not for the shape — there is no "how much later is suspicious" judgment to make here, only
    "later at all", and a grace window would be a free parameter with no evidence behind it (R11).
    """
    # A club with no declared entry has nothing to be inconsistent with, and a declared club with no
    # swings is pooling nothing yet — the state every club is in the day it is added to the bag.
    if entry is None or not swings:
        return []

    earlier = [swing for swing in swings if swing.captured_at < entry.recorded_at]
    if not earlier:
        return []

    # Both forms below count **swings**, never "shots": `n_shots` on the profile beside them
    # counts distinct shot *photos*, so prose saying "shots" would name a different number than the
    # one the reader can see.
    date = f"{entry.recorded_at:%Y-%m-%d}"

    if len(earlier) == len(swings):
        subject = (
            "The single swing behind these numbers predates"
            if len(swings) == 1
            else f"All {len(swings)} swings behind these numbers predate"
        )
        # The tail carries no pronoun on purpose: "the one that hit them" needs "it" in the
        # singular, and a sentence that has to agree with a count is a sentence that will one day
        # not. Naming the three fields is also the more useful form — they are exactly what is in
        # doubt, and the distance statistics beside them are not.
        return [
            f"{subject} the bag entry recorded on {date}, so the make, model and loft recorded "
            f"there may describe a different club."
        ]

    return [
        f"{len(earlier)} of the {len(swings)} swings behind these numbers were hit before the "
        f"bag entry recorded on {date}. A club replaced under the same name would put two "
        f"different clubs in one average, and these numbers pool both."
    ]
