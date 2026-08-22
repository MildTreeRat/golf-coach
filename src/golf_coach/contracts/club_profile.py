"""Everything one golfer's history says about one club — and where it refuses. [M9 P14]

Career mode built a guard: `analysis/baseline.py` will not report a mean until it has the samples
for one, `analysis/dispersion.py` separates a repeatable miss from a scattered one, and
`storage/corpus.py` counts the samples honestly. M9 P13 added the last missing piece,
`narrow_to(club=)`, so all of that machinery now runs over *one club's* swings with its counts
recomputed. **What was missing was the shape the answer comes back in**, and that is this module.

It is a declaration, not a computation. Nothing here averages, gates or judges anything — every
statistic arrives already sealed by the step that was allowed to make it, and this module's job is
to say which club it belongs to and what evidence stands behind it. `analysis/club_profile.py`
(P15) builds these; `scripts/club_profile.py` (P17), the MCP tools (P18) and the bag page (P19)
render them.

## Two counters, because per club they diverge

`CareerCorpus` already splits `distinct_swings` from `distinct_shots`, and the split matters far
more once a club is in hand. A 7 iron hit six times on video with two shot-screen photos is six
swings of history and a **carry ceiling of two** — every launch-monitor claim (carry, total,
offline, ball speed, launch angle) dedupes on the shot photo's hash, so two is what a distance
statistic can ever be built from however many clips exist. One counter would either undercount the
history or overstate the distance evidence, and both are the kind of wrong number nobody audits.

Both are always populated, never gated. They are facts about how much data exists rather than
claims about the golfer — the same reason `MetricBaseline.n` and `n_sessions` sit outside the
guard, and the same reason a refusal is actionable at all: "you have 2 shots on this club, a mean
needs 5" tells someone to book a bay hour where a bare `None` tells them nothing.

## A withheld statistic is absent, not zero

Nothing here re-states that rule, because `contracts/baseline.py` already carries it in full — read
*"Withheld means absent, not flagged"* there. It applies unchanged one level down: a `None` inside
a `MetricBaseline` or a `MetricDispersion` reached through a `ClubProfile` is a refusal itemised in
that model's own `withheld`, and any surface rendering it must show the reason rather than a blank
cell. Per club the refusals are simply more common, because five 7-iron shots is a higher bar than
five shots.

## Declared and hit are different questions

`in_bag` and `n_swings > 0` are kept apart, and `BagProfile` derives a list from each. ADR-024 §2
and `contracts/bag.py` both say why: a club in the bag you have not hit has no statistics and is
not an error, and a club you have hit that has left the bag still has real history. Collapsing them
into one list loses which is which — and the two need opposite responses, one being "go hit it" and
the other "this is history, not your current bag".

Stdlib + pydantic only (ADR-008). Composes `MetricBaseline`, `MetricDispersion` and `BagEntry`
rather than restating their fields, exactly as `contracts/dispersion.py` composes `Interval` and
`WithheldClaim` (R5).
"""

from __future__ import annotations

from typing import Self

from pydantic import BaseModel, Field, model_validator

from golf_coach.contracts.bag import BagEntry
from golf_coach.contracts.baseline import MetricBaseline
from golf_coach.contracts.club import ClubId, category_of
from golf_coach.contracts.dispersion import MetricDispersion
from golf_coach.contracts.intent import ClubCategory


class ClubProfile(BaseModel):
    """One club of one golfer's: the physical club, the evidence, and what it supports saying."""

    club: ClubId

    bag_entry: BagEntry | None = Field(
        default=None,
        description=(
            "The physical club declared in this slot, or None when the golfer has not declared "
            "one. None is not a gap to be filled with a catalogue default — it is what makes a "
            "loft or fitting question refuse for this club while its distance statistics stay "
            "perfectly readable, the same per-input refusal `SwingResult.unscored` performs "
            "(ADR-024 §2)."
        ),
    )
    in_bag: bool = Field(
        default=False,
        description=(
            "Is this club in the bag *now*. Deliberately not implied by `n_swings` and not the "
            "same question as `bag_entry is not None` will be once P16 reads a retired stint: a "
            "retired club has history and is not in the bag, a newly declared one is in the bag "
            "with no history. See the module docstring."
        ),
    )

    # --- evidence, never gated --------------------------------------------------------
    n_swings: int = Field(
        default=0,
        description=(
            "Distinct swings tagged with this club — how many times it was hit, as far as this "
            "repo can see. Never swing directories: re-uploads were folded together upstream by "
            "`CorpusSwing`."
        ),
    )
    n_shots: int = Field(
        default=0,
        description=(
            "Distinct shot photos among those swings, and so **the ceiling on every "
            "launch-monitor claim** — carry, total, offline, ball speed and launch angle all "
            "dedupe on the photo's hash. Lower than `n_swings` whenever a clip was filmed without "
            "the screen being photographed; the module docstring says why one counter would not do."
        ),
    )
    n_sessions: int = Field(
        default=0,
        description="Distinct sessions those swings came from. The axis a TREND claim gates on.",
    )

    # --- what the evidence supports ---------------------------------------------------
    metrics: dict[str, MetricBaseline] = Field(
        default_factory=dict,
        description="Metric name -> this club's baseline, sorted by name. Refusals included.",
    )
    dispersion: dict[str, MetricDispersion] = Field(
        default_factory=dict,
        description="Metric name -> this club's miss-shape, sorted by name. Refusals included.",
    )

    caveats: list[str] = Field(
        default_factory=list,
        description=(
            "Things true of this whole profile that qualify it, in the voice a golfer reads. P16 "
            "appends the bag-changed sentence here: it names a date rather than withholding a "
            "statistic, because an entry recorded late for a club that never changed is the more "
            "likely case and the cost of being wrong is one sentence."
        ),
    )

    @property
    def category(self) -> ClubCategory:
        """The benchmark-row family this club belongs to.

        Derived through `category_of` rather than stored, because `CLUB_CATEGORY` is *the* one
        mapping and a copy carried on every profile is a second home free to disagree with it (R4).
        Not serialized, which is the deliberate half: the API and MCP surfaces build their own view
        models (`mcp/career.py`), and a consumer holding the profile holds the club id this is
        derived from.
        """
        return category_of(self.club)


class BagProfile(BaseModel):
    """Every club one golfer has hit or declared, in the order they sit in the bag."""

    player_id: str

    clubs: tuple[ClubProfile, ...] = Field(
        default=(),
        description=(
            "One profile per club hit or declared, in **canonical bag order** — the declaration "
            "order of `ClubId`, pinned below. A flat tuple and not a dict keyed by club, because "
            "`ClubProfile.club` already names the slot and a second key is a second thing that "
            "can disagree with the entry it holds (the bug `Bag._keys_match_entries` exists to "
            "catch)."
        ),
    )

    untagged_swings: int = Field(
        default=0,
        description=(
            "Swings this golfer has hit that name no club, carried whole from "
            "`CareerCorpus.untagged_swings` and deliberately not renamed on the way through: one "
            "number with two spellings is two things that can be reported differently. This is "
            "the history no profile above can see, and it belongs beside them — every swing on "
            "disk today is in here, because they all predate the tag."
        ),
    )

    @model_validator(mode="after")
    def _clubs_are_in_bag_order(self) -> Self:
        """Canonical order, and each club at most once.

        `contracts/club.py` states that `ClubId` declaration order is read and not decorative, and
        that sorting anywhere downstream is the bug the ordering exists to prevent. A validator is
        what makes that enforceable rather than a convention the builder can forget — an
        alphabetised bag puts `3w` between `2h` and `5h`, which a golfer has no way to tell apart
        from a bug in their bag.

        Duplicates are rejected in the same pass because they are the same class of failure caught
        at the same moment: two profiles for one club means one of them is being silently dropped
        by whichever consumer looks the club up first, and `profile_for` below is one.
        """
        order = {club: index for index, club in enumerate(ClubId)}
        seen: set[ClubId] = set()
        previous = -1
        for profile in self.clubs:
            if profile.club in seen:
                raise ValueError(f"{profile.club} appears twice in clubs")
            seen.add(profile.club)
            index = order[profile.club]
            if index < previous:
                raise ValueError(f"clubs are not in bag order: {profile.club} came too late")
            previous = index
        return self

    @property
    def clubs_used(self) -> tuple[ClubProfile, ...]:
        """Clubs with at least one swing behind them, whether or not they are still in the bag."""
        return tuple(profile for profile in self.clubs if profile.n_swings > 0)

    @property
    def clubs_declared(self) -> tuple[ClubProfile, ...]:
        """Clubs currently in the bag, whether or not anything has been hit with them."""
        return tuple(profile for profile in self.clubs if profile.in_bag)

    def profile_for(self, club: ClubId) -> ClubProfile | None:
        """This club's profile, or None if it has neither been hit nor declared.

        One home for the lookup, for the reason `Bag.retired_for` gives: P17's `--club 7i` and
        P18's `get_club_profile` are both scans over `clubs`, and two hand-rolled ones are two
        places to get it wrong. None rather than a raise, because "no history and not in the bag"
        is a real and common answer a caller renders, not an error it recovers from.
        """
        return next((profile for profile in self.clubs if profile.club is club), None)
