"""The bag, in the shapes an MCP tool returns. [M9 P18]

`mcp/career.py`'s sibling, and the same job one cut further in: that module answers "what does this
golfer usually do", this one answers "what does this golfer do **with this club**". Everything
underneath is already built and already sealed — `analysis/club_profile.py` narrows the corpus per
club and hands each slice to career mode's guard, so nothing here averages, gates or judges. What
is here is the wire shape, the two misses, and the notes that say what a silence means.

Neither this nor `career.py` imports the MCP SDK; that stays in `server.py`, which is what lets the
interesting half be tested on a base install.

## Five silences, and they need different answers

This is the phase's whole prose problem, and M9 P17 found the first of them by rendering the real
corpus rather than by reasoning about it. **The last two were found the same way** — by driving a
populated bag and reading what it said — which is why there are five and not the three this module
was first written with.

  1. **No club profiles at all.** Nothing on disk names a club. A bay session does not fix this —
     the swings are already hit — so the answer is "tag them", and `NOTHING_TAGGED` says so.
  2. **A club whose every figure is withheld.** The guard wants more shots on that club. This one
     *is* fixed by hitting balls, and `NOT_ENOUGH_ON_THIS_CLUB` says so.
  3. **A club never hit and not in the bag.** Nothing was refused; there is no record. An answer,
     not a miss — the posture `shot_trends` takes toward a metric a golfer has never recorded.
  4. **A club in the bag that has never been hit.** `IN_BAG_NEVER_HIT`. The state every club is in
     the day it is declared, and describing it as "withheld" would report a refusal of a claim
     nobody made — the same fabricated refusal the tour block is left out to avoid, one field over.
  5. **A club whose swings carry no measurement.** `NO_MEASUREMENTS`. Not about the club at all:
     unanalyzed swings, or swings analyzed by an older engine.

From a distance all five read as "no numbers", and a model handed the wrong one gives a confident
wrong instruction. `contracts.caveats.READING_A_BAG` states the distinction once at the top of the
conversation and the notes below repeat it at the point of use, which is where it gets ignored.

## Why `ClubMetric` is not `career.MetricProfile`

`MetricProfile` carries the tour join — `tour_standing`, `tour_percentile`, the band edges — and
there is no per-club tour join to put in it. ADR-024's second trap says why: `ranges.json` holds
`club_category: "all"` rows only, so the club is not an axis this panel varies on, and ADR-010
gated per-club bands and cut none.

Reusing the shape and leaving those fields at their defaults would ship `tour_standing: "withheld"`
on every metric of every club, forever. That reads as "the guard refused a claim it could otherwise
make", which is false, and this repo's whole posture is that a refusal has to be true — a fabricated
one is worse than an absent field, because it invites "so what would it take?" and nothing answers.
So the baseline and dispersion halves are mirrored field for field and the tour block is simply not
here. The duplication is the price of not lying, and it is bounded: one shape, one module.

Base install only — no MCP SDK, no vision, no OCR.
"""

from __future__ import annotations

from pathlib import Path

from pydantic import BaseModel, Field

from golf_coach.analysis.club_profile import build_bag_profile
from golf_coach.contracts.bag import BagEntry
from golf_coach.contracts.baseline import MetricBaseline
from golf_coach.contracts.club import ClubId, category_of, parse_club
from golf_coach.contracts.club_profile import BagProfile, ClubProfile
from golf_coach.contracts.dispersion import MetricDispersion
from golf_coach.mcp import career, query
from golf_coach.storage.bag_store import BagStore
from golf_coach.storage.corpus import read_corpus

#: Silence (1). The one sentence that has to survive contact with a model that has been told, five
#: bullets earlier, that a missing figure means more swings are needed. Here it does not: the swings
#: exist and are on disk, and what is missing is a tag on them. Repeated in the payload rather than
#: left to `caveats.READING_A_BAG`, because a briefing is read once on connect and a note is read at
#: the moment the answer is being written.
NOTHING_TAGGED = (
    "No swing on record names a club, so there is nothing to profile per club yet. This is NOT a "
    "sample-size refusal and a bay session does not fix it on its own — the club tag is newer than "
    "these swings, and they stay uncountable per club until each one is tagged. Do not answer this "
    "by telling the golfer to hit more balls. New swings get a club at upload; a swing already "
    "stored is retagged one at a time, from the change control on its row in the upload page's "
    "swing list or with POST /api/sessions/<session>/swings/<swing>/club."
)

#: Silence (2), for one club. `career.THE_UNBLOCK` carries the instruction unchanged — the floors
#: are career mode's own, not a second set — and the sentence in front of it is the part that is
#: only true per club: one club's history is a fraction of the bag's, so a corpus that speaks about
#: tempo can refuse everything about a 7 iron with nothing wrong anywhere.
NOT_ENOUGH_ON_THIS_CLUB = (
    "Every claim about this club is withheld. The floors are the same ones the whole-bag history "
    "faces, applied to this club's share of it, so a bag hit across a dozen clubs reaches them far "
    "more slowly than the bag as a whole does. " + career.THE_UNBLOCK
)

#: Silence (3). Deliberately not a `NotFound`: the club is real, the golfer is real, and "you have
#: no record of hitting it" is the answer. `BagProfile.profile_for` returning None is what this
#: reads off, and P14 put that lookup on the contract so this and `scripts/club_profile.py --club`
#: could not disagree about what "no such club" means.
NEVER_HIT = (
    "No swing on record was hit with this club and it is not in the declared bag, so there is no "
    "history to report and nothing has been refused. That is an answer rather than a miss. If the "
    "golfer does hit this club, tagging the swing is what puts it here."
)

#: Silence (4), and the reason it is not (2). A club declared today has no statistics and has
#: refused nothing — no claim was made and none was declined. Saying "every claim is withheld"
#: would be a refusal this module invented, which is the failure the whole file is arranged to
#: avoid. `scripts/club_profile.py` draws the same distinction on screen (`_no_metrics_reason`),
#: and a payload less honest than the dev CLI is the wrong way round.
IN_BAG_NEVER_HIT = (
    "This club is in the bag and nothing has been hit with it yet, so there are no statistics and "
    "nothing has been refused. That is the state every club is in the day it is declared, and the "
    "profile exists so the club is visible before it has any history."
)

#: Silence (5). Swings exist and none of them contributed a value, which is not a fact about the
#: club: the commonest causes are a swing that was never analyzed and one analyzed by an engine
#: older than the one installed. Points at `get_golfer_profile` rather than at the dev CLI the same
#: sentence names in `scripts/club_profile.py`, because a model has the tool and not the shell.
NO_MEASUREMENTS = (
    "These swings carry no measurement at all, so there is nothing here to refuse and nothing to "
    "report. That is not about the club: the commonest causes are swings that were never analyzed "
    "and swings analyzed by an older engine. Call get_golfer_profile to see which metrics have "
    "samples at all."
)

#: Appended whenever untagged swings sit beside a bag that does have clubs in it. `NOTHING_TAGGED`
#: covers the all-or-nothing case; this is the partial one, and it is what `scripts/club_profile.py`
#: prints last on its populated path for the same reason — a bag looks complete when most of a
#: golfer's swings are simply missing from it.
SOME_SWINGS_UNTAGGED = (
    "Some swings on record name no club (see `untagged_swings`), so nothing above counts them. "
    "They are not lost — every whole-bag figure get_golfer_profile reports still includes them — "
    "but no per-club number can be cut from them until each one is tagged."
)


class ClubMetric(BaseModel):
    """One metric for one club: what its history supports, and every refusal in the gaps.

    Mirrors the baseline and dispersion halves of `career.MetricProfile` and carries no tour block
    — see the module docstring for why that absence is deliberate rather than unfinished.
    """

    name: str
    unit: str
    source: str
    n: int = Field(
        description="Distinct contributing artifacts for THIS club — never swing directories, and "
        "never the whole bag's count. A launch-monitor metric counts shot photographs, so its `n` "
        "stops at `n_shots` however many clips exist."
    )
    n_sessions: int

    # --- what this club's own history says (contracts.baseline) ----------------------------
    center: float | None = None
    center_ci_low: float | None = None
    center_ci_high: float | None = None
    median: float | None = None
    sd: float | None = None
    sd_ci_low: float | None = None
    sd_ci_high: float | None = None
    minimum: float | None = None
    maximum: float | None = None

    # --- what the shape of the miss points at (contracts.dispersion) ------------------------
    bias: str = Field(
        default="withheld",
        description=(
            "established | not_established | withheld. The center sits further from the target "
            "than measurement error explains. 'not_established' means the data cannot tell, NOT "
            "that there is no bias."
        ),
    )
    scatter: str = Field(
        default="withheld", description="Same three answers, for the shot-to-shot spread."
    )
    pattern: str | None = Field(
        default=None,
        description="Present only when BOTH findings were answerable — a pattern is a statement "
        "about the pair and half of it cannot imply the other.",
    )
    points_at: str | None = Field(
        default=None, description="What to check, phrased as a check and never as a diagnosis."
    )
    target: float | None = Field(
        default=None,
        description=(
            "What this metric is aiming at, when anything is. The distance metrics carry None on "
            "purpose: how far a golfer should hit a given club is not a number this repo has, so "
            "they never earn a bias finding and `unavailable` says so. Repeatability is still "
            "answerable, because that needs nobody to declare what good is."
        ),
    )
    tolerance: float | None = Field(
        default=None, description="This pipeline's own measurement error, in the metric's units."
    )
    within_session_sd: float | None = Field(
        default=None,
        description="Pooled within-session spread, when it can be formed. Beside `sd`, which "
        "pools across occasions and so mixes shot-to-shot scatter with drift between them.",
    )

    sessions: list[career.SessionPoint] = Field(default_factory=list)
    caveats: list[str] = Field(default_factory=list)
    withheld: list[career.Refusal] = Field(
        default_factory=list,
        description="Refused for want of data. More shots on THIS club fix these.",
    )
    unavailable: list[str] = Field(
        default_factory=list,
        description="Refused for want of a reference. More shots do NOT fix these.",
    )


class ClubView(BaseModel):
    """One club of one golfer's: the physical club, the evidence, and what it supports saying."""

    player_id: str
    club: str = Field(description="Canonical club id, e.g. '7i', 'driver', 'pw'.")
    category: str = Field(
        description="Which family of clubs this is — derived from the club id, never stored "
        "beside it."
    )

    in_bag: bool = Field(
        description="Is this club in the bag now. Not the same question as whether it has history: "
        "a club that has left the bag keeps every statistic it earned, and a newly declared one "
        "has none yet."
    )
    bag_entry: BagEntry | None = Field(
        default=None,
        description=(
            "The physical club declared in this slot, or None when the golfer has not declared "
            "one. None costs the loft and nothing else — the distance statistics beside it are "
            "unaffected — but a loft, gapping-by-loft or fitting question about this club must "
            "refuse rather than take a catalogue value for a loft nobody measured."
        ),
    )

    n_swings: int = Field(
        description="Distinct swings tagged with this club. Clips, deduplicated on their contents."
    )
    n_shots: int = Field(
        description="Distinct shot-screen photographs among those swings, and so the CEILING on "
        "every distance and launch figure below — carry, total, offline, ball speed and launch "
        "angle are all read off that screen. Lower than `n_swings` whenever a clip was filmed "
        "without the screen being photographed."
    )
    n_sessions: int = Field(description="Distinct sessions those swings came from.")

    metrics: list[ClubMetric] = Field(default_factory=list)
    caveats: list[str] = Field(
        default_factory=list,
        description="Qualifications on this whole club, in the voice a golfer reads. A club whose "
        "bag entry was recorded after some of these swings says so here: replaced under the same "
        "name, two physical clubs land in one average.",
    )

    nothing_sayable: bool = Field(
        default=False, description="True when the guard refused every claim on every metric."
    )
    note: str = Field(
        default="",
        description="What the silence above means and what to do about it. Read it before "
        "describing this club as having no data — the three silences need different answers.",
    )


class BagView(BaseModel):
    """Every club one golfer has hit or declared, in the order they sit in the bag."""

    player_id: str
    display_name: str

    clubs: list[ClubView] = Field(
        default_factory=list,
        description="One entry per club hit or declared, in bag order — driver through putter, "
        "never alphabetical. A club with neither history nor a bag entry gets no entry at all, "
        "which is why an empty list is a statement about the tags rather than about the golfer.",
    )
    clubs_used: int = Field(
        default=0, description="How many of them have at least one swing behind them."
    )
    clubs_declared: int = Field(
        default=0, description="How many of them are in the bag now, hit or not."
    )

    untagged_swings: int = Field(
        default=0,
        description="Swings this golfer has hit that name no club. This is the history no entry "
        "above can see, and it is not lost — it counts toward every whole-bag figure "
        "get_golfer_profile reports. It is only invisible per club.",
    )

    nothing_tagged: bool = Field(
        default=False,
        description="True when no club has been hit or declared at all. NOT a sample-size "
        "refusal — read `note`.",
    )
    note: str = Field(default="", description="What to do about it, when nothing is sayable.")


# --------------------------------------------------------------------------------------
# The tools
# --------------------------------------------------------------------------------------


def bag_profile(sessions_dir: Path, golfers_dir: Path, player: str) -> BagView | None:
    """One golfer's whole bag, club by club. None if the golfer is unknown.

    Every club is narrowed out of one `read_corpus` by `build_bag_profile`, so each club's counts
    provably describe that club's swings — the property `CareerCorpus.narrowed_to` recomputes its
    counts to hold.
    """
    golfer = career.resolve_golfer(golfers_dir, player)
    if golfer is None:
        return None

    profile = _bag(sessions_dir, golfers_dir, golfer.player_id)
    clubs = [_club_view(golfer.player_id, one) for one in profile.clubs]

    return BagView(
        player_id=golfer.player_id,
        display_name=golfer.display_name,
        clubs=clubs,
        clubs_used=len(profile.clubs_used),
        clubs_declared=len(profile.clubs_declared),
        untagged_swings=profile.untagged_swings,
        nothing_tagged=not clubs,
        note=_bag_note(clubs, profile.untagged_swings),
    )


def club_profile(
    sessions_dir: Path, golfers_dir: Path, player: str, club: str
) -> ClubView | query.NotFound | None:
    """One club of one golfer's. Two different misses, and they are different return types.

    `None` is an unknown **golfer**, and the adapter turns it into a `NotFound` — the hint on that
    miss differs per tool, which is why `career.py` keeps them in `missing_golfer_*` helpers rather
    than building one here. A `NotFound` returned from inside is an unknown **club**, whose hint is
    the same in both adapters and so is built once, here.

    A club that parses but has neither history nor a bag entry is neither of those: it is an answer,
    and it comes back as a `ClubView` carrying `NEVER_HIT`.
    """
    golfer = career.resolve_golfer(golfers_dir, player)
    if golfer is None:
        return None

    # `parse_club` refuses rather than nudging toward a nearest match, and this inherits that whole
    # argument: a retype costs one line, and a wrong club pools a wedge's carries into a 7 iron's
    # average where nothing downstream will ever flag it (ADR-024 §5).
    parsed = parse_club(club)
    if parsed is None:
        return unknown_club(club)

    profile = _bag(sessions_dir, golfers_dir, golfer.player_id)
    one = profile.profile_for(parsed)
    if one is None:
        return _never_hit(golfer.player_id, parsed)
    return _club_view(golfer.player_id, one)


# --------------------------------------------------------------------------------------
# Misses
# --------------------------------------------------------------------------------------

# The golfer misses mirror `career.missing_golfer_*` exactly — same shape, same reason, a different
# hint each time because what to do next differs. The club miss is this module's own.


def missing_golfer_bag(player: str) -> query.NotFound:
    return query._missing(
        f"No golfer matching {player!r} is registered.",
        "Names are matched loosely (case and punctuation are folded), so this means nobody by "
        "that name has swings on file yet. Call get_golfer_profile to see who is.",
    )


def missing_golfer_club(player: str) -> query.NotFound:
    return query._missing(
        f"No golfer matching {player!r} is registered.",
        "Call get_bag_profile with the name to see whether they have a bag on file at all.",
    )


def unknown_club(text: str) -> query.NotFound:
    """Not a club. Refused rather than resolved to the nearest thing — see `parse_club`."""
    return query._missing(
        f"{text!r} is not a club.",
        "Clubs are named like '7i', '7 iron', 'seven iron', 'driver' or 'pw'. Note that 'wedge' "
        "and 'iron' name a category rather than a club, so neither is accepted: ask which one, or "
        "call get_bag_profile for every club this golfer has.",
    )


# --------------------------------------------------------------------------------------
# Internals
# --------------------------------------------------------------------------------------


def _bag(sessions_dir: Path, golfers_dir: Path, player_id: str) -> BagProfile:
    """The corpus, the bag and the builder — the same three lines `scripts/club_profile.py` runs.

    The bag lives in the golfer directory (`data/processed/golfers/<player_id>.bag.json`), so there
    is no second path to inject. `BagStore.get` is tolerant: a golfer with no declared bag reads as
    None, and None is not a degraded mode — it costs the loft and nothing else.
    """
    corpus = read_corpus(sessions_dir, player_id)
    return build_bag_profile(corpus, BagStore(golfers_dir).get(player_id))


def _club_view(player_id: str, profile: ClubProfile) -> ClubView:
    metrics = [
        _metric(baseline, profile.dispersion.get(name))
        for name, baseline in sorted(profile.metrics.items())
    ]
    # `claims_ready == 0`, which is `PersonalBaseline.nothing_sayable`'s own rule applied to this
    # club's metrics. Derived rather than restated so a change to what "ready" means cannot leave
    # one surface saying a golfer has nothing to hear while another says they do.
    nothing_sayable = not any(baseline.ready for baseline in profile.metrics.values())

    return ClubView(
        player_id=player_id,
        club=profile.club.value,
        category=profile.category.value,
        in_bag=profile.in_bag,
        bag_entry=profile.bag_entry,
        n_swings=profile.n_swings,
        n_shots=profile.n_shots,
        n_sessions=profile.n_sessions,
        metrics=metrics,
        caveats=list(profile.caveats),
        nothing_sayable=nothing_sayable,
        note=_club_note(profile, nothing_sayable),
    )


def _club_note(profile: ClubProfile, nothing_sayable: bool) -> str:
    """Which silence this club is in, or none of them.

    The order is the load-bearing part. "No swings" is asked first, because a club with none has
    refused nothing — `nothing_sayable` is true there and means something else entirely, and
    reading it first is how a newly declared club gets reported as a refusal. A club with no swings
    is always one that was declared: `build_bag_profile` gives a row to a club that was hit **or**
    declared, so the other combination has no row to be in.
    """
    if profile.n_swings == 0:
        return IN_BAG_NEVER_HIT
    if not profile.metrics:
        return NO_MEASUREMENTS
    return NOT_ENOUGH_ON_THIS_CLUB if nothing_sayable else ""


def _bag_note(clubs: list[ClubView], untagged: int) -> str:
    """The bag-level silence, and the reason there is more than one of them.

    An empty list and a full list of refusals are the same *shape* and opposite *instructions*.
    Deciding between them here rather than leaving a model to infer it from `nothing_tagged` is the
    point of the field.

    The two can also both apply, which is the case a populated drive found: a bag with one declared
    club and a pile of untagged swings needs the golfer to hit that club *and* tag the history, and
    a note carrying only the first describes half the situation as though it were the whole of it.
    """
    if not clubs:
        return NOTHING_TAGGED

    note = career.THE_UNBLOCK if all(one.nothing_sayable for one in clubs) else ""
    if untagged:
        note = f"{note} {SOME_SWINGS_UNTAGGED}" if note else SOME_SWINGS_UNTAGGED
    return note


def _never_hit(player_id: str, club: ClubId) -> ClubView:
    """A real club with no record of it. An answer, not a miss — see `NEVER_HIT`."""
    return ClubView(
        player_id=player_id,
        club=club.value,
        # `category_of`, the one mapping, rather than a second copy here. `ClubProfile.category`
        # derives it the same way; there is simply no profile to ask.
        category=category_of(club).value,
        in_bag=False,
        n_swings=0,
        n_shots=0,
        n_sessions=0,
        nothing_sayable=True,
        note=NEVER_HIT,
    )


def _metric(baseline: MetricBaseline, dispersion: MetricDispersion | None) -> ClubMetric:
    """Two layers about one metric, flattened into one view.

    `career._profile`'s shape minus the tour join, and its reasoning for merging the refusals holds
    unchanged: a caller asking "why is there no number here" wants the reasons, not which module
    produced each. `dispersion.withheld` is deliberately not merged in — `analysis/dispersion.py`
    builds it by *filtering* the baseline's own, so every sentence in it is already in `withheld`
    below. P17 verified that containment in a REPL rather than reading it off the source, and
    carrying both would say each thing twice in a payload that is mostly refusals.
    """
    metric = ClubMetric(
        name=baseline.name,
        unit=baseline.unit,
        source=baseline.source,
        n=baseline.n,
        n_sessions=baseline.n_sessions,
        center=baseline.mean,
        center_ci_low=career._low(baseline.mean_ci),
        center_ci_high=career._high(baseline.mean_ci),
        median=baseline.median,
        sd=baseline.sd,
        sd_ci_low=career._low(baseline.sd_ci),
        sd_ci_high=career._high(baseline.sd_ci),
        minimum=baseline.minimum,
        maximum=baseline.maximum,
        sessions=career._points(baseline),
        withheld=[career._refusal(refusal) for refusal in baseline.withheld],
    )

    if dispersion is not None:
        metric.bias = dispersion.bias.value
        metric.scatter = dispersion.scatter.value
        metric.pattern = dispersion.pattern.value if dispersion.pattern else None
        metric.points_at = dispersion.points_at
        metric.target = dispersion.target
        metric.tolerance = dispersion.tolerance
        metric.within_session_sd = dispersion.within_session_sd
        metric.caveats = list(dispersion.caveats)
        metric.unavailable = list(dict.fromkeys(dispersion.unavailable))

    return metric
