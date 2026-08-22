"""The two club tools, on the three silences and on a club that speaks. [M9 P18]

`test_club_profile_builder.py` pins the builder over contracts built in memory; what is tested here
is the trip **out** — that a per-club refusal reaches a model as absent rather than as a number, and
that the three ways this payload can be quiet arrive as three different things.

**That last property is the phase.** An empty bag, a bag of refusals and a club nobody has hit all
render as "no numbers", and the correct response differs each time: tag your swings, hit some balls,
nothing is wrong. A model handed the wrong one gives a confident wrong instruction, so the notes are
asserted against each other and not merely for being non-empty.

The MCP SDK is not imported here. These call `mcp.club` directly, the boundary `test_query.py` and
`test_career_tools.py` both hold, and what keeps them running on the base install.
"""

from __future__ import annotations

from datetime import UTC, datetime
from pathlib import Path

import pytest

from golf_coach.contracts.bag import BagEntry
from golf_coach.contracts.club import ClubId
from golf_coach.mcp import club
from golf_coach.storage.bag_store import BagStore
from golf_coach.storage.manifest import Role

#: `BagEntry.recorded_at` is required and has no default — the contract stays dumb and `BagStore`
#: stamps the real one, discarding whatever is passed here (`set_entry`). So this value never
#: reaches disk; it exists because the model will not build without it.
_UNUSED = datetime(2020, 1, 1, tzinfo=UTC)

#: Two pose metrics and the five launch-monitor ones, so a club's payload shows both provenances at
#: once — the pair whose `n` diverges the moment a clip is filmed without a screen photograph.
_CENTERS = {
    "head_sway_norm": (0.200, 0.015),
    "tempo_ratio": (3.000, 0.100),
    "carry_distance_yds": (154.000, 3.000),
    "total_distance_yds": (168.000, 4.000),
    "start_line_offline_yds": (2.000, 1.500),
    "ball_speed_mph": (118.000, 2.000),
    "launch_angle_deg": (18.000, 1.000),
}

#: 18 swings over 3 sessions clears every floor in the panel, exactly as `test_career_tools.py`
#: chose them — and here they are 18 swings **on one club**, which is the whole point: the same
#: corpus split across a bag would clear nothing.
_SWINGS = 18
_SESSIONS = 3


def _values(i: int) -> dict[str, float]:
    """Alternating about each center, so a spread is exact and no test flakes on a seed."""
    return {
        name: center + (half if i % 2 == 0 else -half)
        for name, (center, half) in _CENTERS.items()
    }


@pytest.fixture
def untagged_dir(tmp_path: Path, swing_writer, career_writer) -> Path:
    """Two analyzed swings naming no club — **the shape actually on disk today.**"""
    root = tmp_path / "sessions"
    for i in range(2):
        swing_writer(
            root,
            f"2026-08-{i + 1:02d}",
            "1",
            player_id="aaron",
            created_at=datetime(2026, 8, i + 1, 12, tzinfo=UTC),
            analysis=career_writer(_values(i)),
        )
    return root


@pytest.fixture
def refusing_dir(tmp_path: Path, swing_writer, career_writer) -> Path:
    """The same two swings, tagged. Every claim about the club is still withheld."""
    root = tmp_path / "sessions"
    for i in range(2):
        swing_writer(
            root,
            f"2026-08-{i + 1:02d}",
            "1",
            player_id="aaron",
            club=ClubId.SEVEN_IRON,
            created_at=datetime(2026, 8, i + 1, 12, tzinfo=UTC),
            analysis=career_writer(_values(i)),
        )
    return root


@pytest.fixture
def speaking_dir(tmp_path: Path, swing_writer, career_writer) -> Path:
    """18 tagged swings across 3 sessions — one club with enough history to be described."""
    root = tmp_path / "sessions"
    for i in range(_SWINGS):
        swing_writer(
            root,
            f"2026-09-{(i % _SESSIONS) + 1:02d}",
            str(i),
            player_id="aaron",
            club=ClubId.SEVEN_IRON,
            created_at=datetime(2026, 9, (i % _SESSIONS) + 1, 12, i, tzinfo=UTC),
            analysis=career_writer(_values(i)),
        )
    return root


def _club(view, name: str = "7i"):
    return next(one for one in view.clubs if one.club == name)


def _metric(view, name: str):
    return next(metric for metric in view.metrics if metric.name == name)


# ------------------------------------------------------------------ resolving the golfer


def test_an_unregistered_name_is_a_miss_on_both_tools(refusing_dir, golfers_dir) -> None:
    """None, so the adapter can turn it into a `NotFound` that says what to do next.

    An empty bag for an unknown name would have the model narrate "you have not tagged any swings"
    at a golfer whose only problem is a typo — and that is the one sentence this phase most needs
    to be true when it is said.
    """
    assert club.bag_profile(refusing_dir, golfers_dir, "Nobody") is None
    assert club.club_profile(refusing_dir, golfers_dir, "Nobody", "7i") is None


def test_a_typed_name_and_a_stored_id_reach_the_same_bag(refusing_dir, golfers_dir) -> None:
    typed = club.bag_profile(refusing_dir, golfers_dir, "Aaron")
    stored = club.bag_profile(refusing_dir, golfers_dir, "aaron")

    assert typed is not None and stored is not None
    assert typed.player_id == stored.player_id == "aaron"


# ------------------------------------------------------------------ the three silences


def test_an_untagged_corpus_is_not_a_refusal_and_does_not_say_hit_more_balls(
    untagged_dir, golfers_dir
) -> None:
    """M9 P17's finding, in the channel a model reads.

    Nothing on disk names a club, so there is no club to refuse *about*. The counters still have to
    say the history exists — those swings are real — and the note has to name the fix, which is
    tagging and not practice. A model given `THE_UNBLOCK` here would tell a golfer to go and hit
    balls they have already hit.
    """
    view = club.bag_profile(untagged_dir, golfers_dir, "Aaron")
    assert view is not None

    assert view.nothing_tagged
    assert not view.clubs
    assert view.untagged_swings == 2
    assert view.note == club.NOTHING_TAGGED
    assert "hit more balls" in view.note
    assert view.note != club.NOT_ENOUGH_ON_THIS_CLUB


def test_a_tagged_corpus_at_the_n_on_disk_refuses_and_asks_for_shots(
    refusing_dir, golfers_dir
) -> None:
    """The other silence, and it must not read like the one above.

    Same two swings, now tagged: a club appears, every claim about it is withheld, and the fix is
    a bay session. `withheld` carries the evidence for the refusal, which is what makes it
    actionable rather than merely silent.
    """
    view = club.bag_profile(refusing_dir, golfers_dir, "Aaron")
    assert view is not None

    assert not view.nothing_tagged
    assert view.untagged_swings == 0
    assert view.note != club.NOTHING_TAGGED

    seven = _club(view)
    assert seven.nothing_sayable
    assert seven.n_swings == 2 and seven.n_sessions == 2

    for metric in seven.metrics:
        assert metric.center is None and metric.sd is None
        assert metric.bias == "withheld" and metric.scatter == "withheld"
        assert metric.withheld, f"{metric.name} refuses without saying what it waits for"
        for refusal in metric.withheld:
            assert refusal.have_n < refusal.need_n
            assert refusal.reason


def test_a_club_never_hit_and_not_in_the_bag_is_an_answer_not_a_miss(
    refusing_dir, golfers_dir
) -> None:
    """The third silence. Nothing was refused — there is simply no record of the club.

    Same posture `get_shot_trends` takes toward a metric a golfer has never recorded: returning a
    `NotFound` here would say the *club* does not exist, which is a different and false statement.
    """
    view = club.club_profile(refusing_dir, golfers_dir, "Aaron", "driver")

    assert isinstance(view, club.ClubView)
    assert view.club == "driver" and view.category == "driver"
    assert view.n_swings == 0 and not view.in_bag and not view.metrics
    assert view.note == club.NEVER_HIT
    assert view.note not in (club.NOTHING_TAGGED, club.NOT_ENOUGH_ON_THIS_CLUB)


def test_every_silence_is_its_own_sentence() -> None:
    """Asserted directly, because every test above depends on it and none of them proves it.

    They are the deliverable of this phase: five states that look alike in a payload and need
    different answers. Two of them collapsing into one string would leave every assertion above
    green and the tools wrong.
    """
    notes = {
        club.NOTHING_TAGGED,
        club.NOT_ENOUGH_ON_THIS_CLUB,
        club.NEVER_HIT,
        club.IN_BAG_NEVER_HIT,
        club.NO_MEASUREMENTS,
        club.SOME_SWINGS_UNTAGGED,
    }
    assert len(notes) == 6


# ------------------------------------------------------------------ the unknown club


def test_text_that_is_not_a_club_is_refused_rather_than_guessed(refusing_dir, golfers_dir) -> None:
    """`parse_club`'s posture, carried to the wire.

    A category is the case worth naming: "wedge" is a word a golfer says, it names four clubs, and
    resolving it to one of them would pool a lob wedge's carries into a pitching wedge's average
    where nothing downstream would ever flag it.
    """
    miss = club.club_profile(refusing_dir, golfers_dir, "Aaron", "wedge")

    assert not isinstance(miss, club.ClubView)
    assert miss is not None and not miss.found
    assert "category" in miss.message
    assert "get_bag_profile" in miss.message


def test_every_spelling_of_one_club_reaches_the_same_profile(speaking_dir, golfers_dir) -> None:
    canonical = club.club_profile(speaking_dir, golfers_dir, "Aaron", "7i")
    spoken = club.club_profile(speaking_dir, golfers_dir, "Aaron", "seven iron")

    assert isinstance(canonical, club.ClubView) and isinstance(spoken, club.ClubView)
    assert canonical.club == spoken.club == "7i"
    assert canonical.n_swings == spoken.n_swings


# ------------------------------------------------------------------ the speaking path


def test_with_enough_swings_on_one_club_it_speaks(speaking_dir, golfers_dir) -> None:
    view = club.club_profile(speaking_dir, golfers_dir, "Aaron", "7i")
    assert isinstance(view, club.ClubView)

    assert not view.nothing_sayable and not view.note
    assert view.n_swings == _SWINGS and view.n_sessions == _SESSIONS

    carry = _metric(view, "carry_distance_yds")
    assert carry.n == _SWINGS
    assert carry.center == pytest.approx(154.0)
    assert carry.center_ci_low is not None and carry.center_ci_high is not None
    assert carry.sd is not None
    assert carry.unit == "yards"


def test_a_distance_earns_a_repeatability_finding_and_never_a_bias_one(
    speaking_dir, golfers_dir
) -> None:
    """No target exists for how far a golfer should hit a club, and none is invented.

    `contracts/dispersion.py` registers carry with a tolerance and no target for exactly this: the
    scatter finding needs nobody to declare what good is, and the bias finding needs someone to.
    `unavailable` is where that reason has to arrive, because more swings never fix it.
    """
    view = club.club_profile(speaking_dir, golfers_dir, "Aaron", "7i")
    assert isinstance(view, club.ClubView)

    carry = _metric(view, "carry_distance_yds")
    assert carry.target is None
    assert carry.bias == "withheld"
    assert carry.scatter != "withheld"
    assert carry.unavailable, "carry refuses a bias finding without saying why"


def test_no_metric_carries_a_tour_placement(speaking_dir, golfers_dir) -> None:
    """The shape decision, pinned. `ranges.json` holds whole-bag rows only (ADR-024's trap).

    Reusing `career.MetricProfile` here would ship `tour_standing: "withheld"` on every metric of
    every club forever — a refusal of a claim nothing ever intended to make, which reads to a model
    as "ask again with more data".
    """
    view = club.club_profile(speaking_dir, golfers_dir, "Aaron", "7i")
    assert isinstance(view, club.ClubView)

    fields = set(club.ClubMetric.model_fields)
    assert not {name for name in fields if name.startswith("tour_")}
    assert "outside_by" not in fields


# ------------------------------------------------------------------ the two counters


def test_the_shot_photo_is_the_ceiling_on_a_distance(
    tmp_path, golfers_dir, swing_writer, career_writer
) -> None:
    """A club filmed more often than it is photographed, which is the ordinary bay case.

    Six swings, two with the simulator screen photographed. The pose metrics see six readings and
    every launch-monitor metric sees two, because `CorpusSwing.artifact_key` keys those on the
    photo's hash. One counter would either undercount the history or overstate the distance
    evidence, and `n_shots` sitting beside `n_swings` is what lets a reader tell which.
    """
    root = tmp_path / "sessions"
    for i in range(6):
        photographed = i < 2
        swing_writer(
            root,
            "2026-09-01",
            str(i),
            player_id="aaron",
            club=ClubId.SEVEN_IRON,
            created_at=datetime(2026, 9, 1, 12, i, tzinfo=UTC),
            roles=(Role.FACE_ON, Role.SHOT_SCREEN) if photographed else (Role.FACE_ON,),
            analysis=career_writer(_values(i)),
        )

    view = club.club_profile(root, golfers_dir, "Aaron", "7i")
    assert isinstance(view, club.ClubView)

    assert view.n_swings == 6
    assert view.n_shots == 2
    assert _metric(view, "head_sway_norm").n == 6
    assert _metric(view, "carry_distance_yds").n == 2


def test_one_club_is_narrowed_out_of_a_mixed_bag(
    tmp_path, golfers_dir, swing_writer, career_writer
) -> None:
    """The correctness argument the builder rests on, asserted through the wire shape.

    Twelve swings, split between two clubs. Neither club may borrow the other's samples to clear a
    floor — a confident mean carry for a 7 iron built out of driver shots is exactly the failure
    `narrowed_to` recomputes its counts to prevent.
    """
    root = tmp_path / "sessions"
    for i in range(12):
        swing_writer(
            root,
            "2026-09-01",
            str(i),
            player_id="aaron",
            club=ClubId.SEVEN_IRON if i % 2 == 0 else ClubId.DRIVER,
            created_at=datetime(2026, 9, 1, 12, i, tzinfo=UTC),
            analysis=career_writer(_values(i)),
        )

    view = club.bag_profile(root, golfers_dir, "Aaron")
    assert view is not None

    assert [one.club for one in view.clubs] == ["driver", "7i"], "bag order, not alphabetical"
    assert _club(view, "driver").n_swings == 6
    assert _club(view, "7i").n_swings == 6
    assert _metric(_club(view, "7i"), "carry_distance_yds").n == 6


# ------------------------------------------------------------------ the declared bag


def test_a_declared_club_appears_with_its_loft_and_no_history(
    untagged_dir, golfers_dir
) -> None:
    """A club in the bag nobody has hit is not an error — it is the state every club starts in.

    And the two lists stay apart: `clubs_declared` counts it, `clubs_used` does not. Note the bag
    profile is no longer `nothing_tagged` even though no swing names a club, which is correct: a
    declared bag is something to say.
    """
    BagStore(golfers_dir).set_entry(
        "aaron",
        BagEntry(club=ClubId.SEVEN_IRON, make="Titleist", loft_deg=34.0, recorded_at=_UNUSED),
    )

    view = club.bag_profile(untagged_dir, golfers_dir, "Aaron")
    assert view is not None

    assert view.clubs_declared == 1 and view.clubs_used == 0
    assert view.untagged_swings == 2, "the untagged history is still counted beside the bag"
    # Both things are true at once and the note has to carry both: hit this club, and tag the
    # swings that are already on disk. Either sentence alone describes half the situation.
    assert club.SOME_SWINGS_UNTAGGED in view.note

    seven = _club(view)
    assert seven.in_bag
    assert seven.bag_entry is not None and seven.bag_entry.loft_deg == 34.0
    assert not seven.metrics
    # NOT the withheld sentence: nothing was claimed about this club, so nothing was refused.
    assert seven.note == club.IN_BAG_NEVER_HIT
    assert seven.note != club.NOT_ENOUGH_ON_THIS_CLUB


def test_a_club_with_no_bag_entry_reports_no_loft_rather_than_a_default(
    refusing_dir, golfers_dir
) -> None:
    """The per-input refusal, on the wire: statistics readable, loft absent.

    `bag_entry` of None is what makes a fitting question about this club refuse, and substituting a
    catalogue loft would make that refusal impossible to see.
    """
    view = club.club_profile(refusing_dir, golfers_dir, "Aaron", "7i")

    assert isinstance(view, club.ClubView)
    assert view.bag_entry is None and not view.in_bag
    assert view.n_swings == 2, "the history is unaffected by an undeclared club"


def test_a_bag_entry_recorded_after_the_swings_carries_its_caveat(
    refusing_dir, golfers_dir
) -> None:
    """P16's sentence, reaching a model.

    `BagStore.set_entry` stamps `recorded_at` now and these swings are dated in the past, so this
    is the ordinary case rather than a contrived one: the day a golfer first declares a bag, every
    club they own lands there at once and every swing predates it.
    """
    BagStore(golfers_dir).set_entry(
        "aaron", BagEntry(club=ClubId.SEVEN_IRON, loft_deg=34.0, recorded_at=_UNUSED)
    )

    view = club.club_profile(refusing_dir, golfers_dir, "Aaron", "7i")

    assert isinstance(view, club.ClubView)
    assert view.caveats, "a bag entry younger than its swings said nothing about it"
    assert "predate" in view.caveats[0]


def test_swings_that_carry_no_measurement_say_so_rather_than_refusing(
    tmp_path, golfers_dir, swing_writer, career_writer
) -> None:
    """The fifth silence, and it is not about the club.

    A tagged swing that contributed no value leaves a club with real `n_swings` and no metrics at
    all. Reporting that as a withheld claim would send a golfer to the bay to fix a swing that was
    never analyzed — so the note names the actual causes and points at the tool that shows which
    metrics have samples.
    """
    root = tmp_path / "sessions"
    for i in range(2):
        swing_writer(
            root,
            "2026-09-01",
            str(i),
            player_id="aaron",
            club=ClubId.SEVEN_IRON,
            created_at=datetime(2026, 9, 1, 12, i, tzinfo=UTC),
            analysis=career_writer({}),
        )

    view = club.club_profile(root, golfers_dir, "Aaron", "7i")

    assert isinstance(view, club.ClubView)
    assert view.n_swings == 2 and not view.metrics
    assert view.note == club.NO_MEASUREMENTS
    assert "get_golfer_profile" in view.note
