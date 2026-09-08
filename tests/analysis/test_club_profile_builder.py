"""The per-club builder. [M9 P15]

The statistics themselves are pinned in `test_baseline.py` and `test_dispersion.py`, and the
narrowing off disk in `tests/storage/test_corpus.py`. What is tested here is the **assembly**, and
two properties carry nearly all of the weight:

- **Every number comes from the narrowed corpus.** A per-club mean built from the whole bag's `n`
  is the failure mode with no symptom: it looks like a working feature, it clears the five-sample
  floor on a club hit twice, and the sentence it produces — "your 7 iron carries 164 yards" — is
  exactly the sentence M9 exists to make true. So the counts are asserted, not just the club lists.
- **Hit and declared stay two questions.** A club with history that has left the bag, and a club in
  the bag with no history, must both survive the round trip as themselves.

Corpora and bags are built by constructing the contracts directly — no disk, the pattern
`test_baseline.py` states and for its reason: the reader has its own tests, and mixing the two would
make a builder failure look like a reader failure.

**`_builder` is in the filename on purpose — do not "fix" it back to the mirror.** `tests/` holds no
`__init__.py`, so under pytest's default `prepend` import mode every test module is imported under
its bare name and two files called `test_club_profile.py` collide. The collision does not skip one
file, it **interrupts the entire run at collection**, so the day it happens nothing is testable. The
real fix is `--import-mode=importlib`, and it is not a one-line change: ten modules across
`tests/analysis/` and `tests/launch_monitor/` do `from conftest import ...`, which is the same
`prepend` idiom and stops resolving under it. That is its own commit. Until then the mirror is held
by the prefix, and the suffix names what distinguishes this file from
`tests/contracts/test_club_profile.py`: the shape is declared there, the builder is exercised here.
"""

from __future__ import annotations

import ast
from datetime import UTC, datetime
from pathlib import Path

import golf_coach.analysis
from golf_coach.analysis.baseline import build_baseline
from golf_coach.analysis.club_profile import build_bag_profile
from golf_coach.contracts.bag import Bag, BagEntry
from golf_coach.contracts.baseline import BaselineClaim
from golf_coach.contracts.career import CareerCorpus, CorpusSwing
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.intent import ClubCategory
from golf_coach.contracts.mishit import MishitVerdict
from golf_coach.contracts.swing import Measurement

CARRY = "carry_distance_yds"
LM = "launch_monitor:hd_golf"


def _at(day: int) -> datetime:
    return datetime(2026, 8, day, 12, tzinfo=UTC)


def _swing(
    index: int,
    *,
    club: ClubId | None,
    carry: float | None = None,
    session_id: str | None = None,
    face_on: str | None = None,
    shot: str | None = None,
    auto_mishit: bool = False,
    mishit: MishitVerdict | None = None,
) -> CorpusSwing:
    """One distinct swing. Defaults give every swing its own clip, photo and session.

    `auto_mishit` / `mishit` are set directly because `_flag_auto_mishits` lives in
    `storage.corpus.read_corpus` and this file never touches disk — detection is pinned in
    `tests/storage/test_corpus.py`, and what is tested here is that the builder surfaces the flag.
    """
    return CorpusSwing(
        player_id="aaron",
        session_id=session_id or f"2026-08-{index:02d}",
        swing_id=str(index),
        captured_at=_at(index),
        face_on_sha256=face_on or f"clip-{index}",
        shot_sha256=shot or f"photo-{index}",
        club=club,
        auto_mishit=auto_mishit,
        manual_mishit=mishit,
        analyzed=True,
        measurements=(
            []
            if carry is None
            else [Measurement(name=CARRY, value=carry, unit="yards", source=LM, detail="test")]
        ),
    )


def _corpus(*swings: CorpusSwing) -> CareerCorpus:
    return CareerCorpus(player_id="aaron", swings=list(swings))


def _hits(club: ClubId, n: int, *, start: int = 1, carry: float = 150.0) -> list[CorpusSwing]:
    """`n` distinct swings of one club, each its own session, clip and shot photo."""
    return [
        _swing(i, club=club, carry=carry + i) for i in range(start, start + n)
    ]


def _bag(*clubs: ClubId, retired: tuple[ClubId, ...] = (), recorded: int = 1) -> Bag:
    """A declared bag. `clubs` is passed in declaration order on purpose in one test.

    `recorded` is the day every entry was declared. It defaults to 1 — before every swing `_hits`
    builds — so a test that says nothing about the bag's age earns no bag-changed caveat and the
    caveat tests are the only ones that have to think about it.
    """
    when = _at(recorded)
    return Bag(
        player_id="aaron",
        entries={club: BagEntry(club=club, recorded_at=when) for club in clubs},
        retired=tuple(
            BagEntry(club=club, recorded_at=when, retired_at=when) for club in retired
        ),
        updated_at=when,
    )


# --------------------------------------------------------------------- the guard, per club


def test_a_club_with_six_shots_states_a_mean_and_one_with_two_refuses() -> None:
    """The whole milestone in one assertion, and the counts are the half that matters.

    Both clubs sit in one corpus of eight shots — comfortably over the five-sample floor. If the
    builder ever hands `build_baseline` the corpus instead of the narrowing, the driver clears
    CENTER too and this test is the only thing standing between that and a confident mean carry
    for a club hit twice.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 6), *_hits(ClubId.DRIVER, 2, start=20, carry=260.0))

    profile = build_bag_profile(corpus)
    irons = profile.profile_for(ClubId.SEVEN_IRON)
    driver = profile.profile_for(ClubId.DRIVER)
    assert irons is not None and driver is not None

    assert irons.n_swings == 6 and irons.n_shots == 6
    assert BaselineClaim.CENTER in irons.metrics[CARRY].ready
    assert irons.metrics[CARRY].n == 6
    assert irons.metrics[CARRY].mean is not None

    assert driver.n_swings == 2 and driver.n_shots == 2
    assert driver.metrics[CARRY].n == 2
    assert driver.metrics[CARRY].mean is None, "a mean off two shots is the bug this milestone has"

    refusal = next(r for r in driver.metrics[CARRY].withheld if r.claim is BaselineClaim.CENTER)
    assert (refusal.have_n, refusal.need_n) == (2, 5)


def test_the_dispersion_is_built_over_the_same_narrowing_as_the_baseline() -> None:
    """Both halves of a `ClubProfile` must describe the same club's shots.

    Cheap to get wrong and invisible when it is: the two builders take a corpus each, so passing
    the narrowed one to `build_baseline` and the whole corpus to `build_dispersion` type-checks,
    runs, and reports one club's mean beside the whole bag's spread.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 6), *_hits(ClubId.DRIVER, 2, start=20, carry=260.0))

    irons = build_bag_profile(corpus).profile_for(ClubId.SEVEN_IRON)
    assert irons is not None
    assert irons.dispersion[CARRY].n == irons.metrics[CARRY].n == 6
    assert irons.dispersion[CARRY].n_sessions == irons.metrics[CARRY].n_sessions


def test_swings_and_shots_are_counted_separately_and_the_photos_are_the_ceiling() -> None:
    """P14's two counters, becoming real for the first time.

    One 7 iron filmed three times with two shot-screen photos is three swings of history and a
    carry ceiling of two, because every launch-monitor claim dedupes on the photo's hash. A single
    counter would either undercount the history or overstate what a distance statistic can be built
    from — and a carry average is the number nobody audits.
    """
    corpus = _corpus(
        _swing(1, club=ClubId.SEVEN_IRON, carry=150.0, face_on="clip-1", shot="photo-a"),
        _swing(2, club=ClubId.SEVEN_IRON, carry=150.0, face_on="clip-2", shot="photo-a"),
        _swing(3, club=ClubId.SEVEN_IRON, carry=160.0, face_on="clip-3", shot="photo-b"),
    )

    irons = build_bag_profile(corpus).profile_for(ClubId.SEVEN_IRON)
    assert irons is not None
    assert irons.n_swings == 3
    assert irons.n_shots == 2
    assert irons.n_sessions == 3
    assert irons.metrics[CARRY].n == 2, "the carry counts photos, not clips"


# ------------------------------------------------------------------- hit, declared, or both


def test_an_empty_bag_still_profiles_every_club_that_has_shots() -> None:
    """The state every golfer is in today: real history, nothing declared.

    Nothing writes a bag until P19, so a builder that profiled only declared clubs would return an
    empty page for the one golfer on disk.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 2), *_hits(ClubId.PITCHING_WEDGE, 1, start=20))

    for bag in (None, _bag()):
        profile = build_bag_profile(corpus, bag)
        assert [p.club for p in profile.clubs] == [ClubId.SEVEN_IRON, ClubId.PITCHING_WEDGE]
        assert [p.club for p in profile.clubs_used] == [ClubId.SEVEN_IRON, ClubId.PITCHING_WEDGE]
        assert profile.clubs_declared == ()


def test_a_declared_but_unhit_club_appears_with_no_history_and_every_claim_withheld() -> None:
    """"You have not hit your driver yet" is a real answer, and the first one every club has.

    It must reach the page as a club with no numbers rather than as an absence, because the two
    need opposite responses — one is "go hit it", the other is "this club is not in your bag".
    """
    profile = build_bag_profile(_corpus(), _bag(ClubId.DRIVER))

    driver = profile.profile_for(ClubId.DRIVER)
    assert driver is not None
    assert driver.in_bag is True
    assert driver.bag_entry is not None
    assert (driver.n_swings, driver.n_shots, driver.n_sessions) == (0, 0, 0)
    assert driver.metrics == {} and driver.dispersion == {}
    assert profile.clubs_used == ()
    assert [p.club for p in profile.clubs_declared] == [ClubId.DRIVER]


def test_a_club_that_has_left_the_bag_keeps_its_history_and_loses_only_its_loft() -> None:
    """The retired shelf is retention, not versioning — so it is not read here (ADR-024).

    A 7 iron swapped out still hit every shot it hit. Withholding its statistics would delete real
    history to record a bag change, and borrowing the retired entry's loft would attach a
    measurement from one physical club to shots hit with another. Neither: history stays, loft goes.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 2))
    profile = build_bag_profile(corpus, _bag(ClubId.DRIVER, retired=(ClubId.SEVEN_IRON,)))

    irons = profile.profile_for(ClubId.SEVEN_IRON)
    assert irons is not None
    assert irons.in_bag is False
    assert irons.bag_entry is None, "the shelf is not a fallback for the current entry"
    assert irons.n_swings == 2

    assert [p.club for p in profile.clubs_used] == [ClubId.SEVEN_IRON]
    assert [p.club for p in profile.clubs_declared] == [ClubId.DRIVER]


def test_clubs_come_back_in_canonical_bag_order_whatever_order_the_bag_declared_them() -> None:
    """Declaration order is read, not decorative, and sorting is the bug it exists to prevent.

    The four clubs are the set `test_bag.py` uses, because insertion (`pw 3w driver 7i`),
    alphabetical (`3w 7i driver pw`) and bag order (`driver 3w 7i pw`) all differ over it — so a
    builder that got the order from `bag.entries`, or reached for `sorted`, fails visibly rather
    than coincidentally passing.
    """
    bag = _bag(ClubId.PITCHING_WEDGE, ClubId.THREE_WOOD, ClubId.DRIVER, ClubId.SEVEN_IRON)

    profile = build_bag_profile(_corpus(), bag)

    assert [p.club for p in profile.clubs] == [
        ClubId.DRIVER,
        ClubId.THREE_WOOD,
        ClubId.SEVEN_IRON,
        ClubId.PITCHING_WEDGE,
    ]
    assert [p.category for p in profile.clubs] == [
        ClubCategory.DRIVER,
        ClubCategory.WOOD,
        ClubCategory.MID_IRON,
        ClubCategory.WEDGE,
    ]


def test_untagged_swings_reach_no_profile_and_are_reported_once() -> None:
    """The history no club profile can see, kept where a reader will find it.

    Every swing on disk today is one of these, so a bag page that dropped them silently would
    report an empty bag for a golfer with real swings in it and give no hint why.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 2), _swing(9, club=None, carry=140.0))

    profile = build_bag_profile(corpus)

    assert [p.club for p in profile.clubs] == [ClubId.SEVEN_IRON]
    assert profile.profile_for(ClubId.SEVEN_IRON).n_swings == 2  # type: ignore[union-attr]
    assert profile.untagged_swings == 1
    assert build_baseline(corpus).metrics[CARRY].n == 3, "untagged shots still count whole-bag"


def test_no_swings_and_no_bag_is_an_empty_profile_not_an_error() -> None:
    """The first true answer about every golfer, and `read_corpus`'s posture one layer up."""
    profile = build_bag_profile(_corpus())

    assert profile.player_id == "aaron"
    assert profile.clubs == ()
    assert profile.untagged_swings == 0
    assert profile.profile_for(ClubId.DRIVER) is None


# --------------------------------------------------------------------- the bag-changed caveat


def _caveats(corpus: CareerCorpus, bag: Bag | None, club: ClubId) -> list[str]:
    profile = build_bag_profile(corpus, bag).profile_for(club)
    assert profile is not None
    return profile.caveats


def test_an_entry_recorded_before_every_swing_says_nothing() -> None:
    """The clean case, and the one the caveat must not cry wolf on.

    A club declared before it was ever hit has one physical club behind every number, so a caveat
    here would fire on the correct state and teach a reader to skip the sentence in the case where
    it means something.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 3, start=5))

    assert _caveats(corpus, _bag(ClubId.SEVEN_IRON, recorded=2), ClubId.SEVEN_IRON) == []


def test_an_entry_recorded_mid_history_names_the_date_and_how_many_predate_it() -> None:
    """The case the phase exists for, and the count is what pins it to the right club.

    The corpus also holds four driver swings hit before the 7 iron was declared. They are pooled
    under a different name and must not reach this sentence: a builder handing the helper
    `corpus.swings` instead of the narrowing writes a caveat about eight swings onto a club that
    has four, which is a sentence wrong about its own subject while reading like a working one.
    """
    corpus = _corpus(
        *_hits(ClubId.SEVEN_IRON, 4, start=1),
        *_hits(ClubId.DRIVER, 4, start=20, carry=260.0),
    )

    caveats = _caveats(corpus, _bag(ClubId.SEVEN_IRON, recorded=3), ClubId.SEVEN_IRON)

    assert len(caveats) == 1
    assert "2 of the 4 swings" in caveats[0], caveats[0]
    assert "2026-08-03" in caveats[0], caveats[0]
    assert "pool both" in caveats[0], caveats[0]


def test_an_entry_recorded_after_every_swing_doubts_the_club_not_the_pooling() -> None:
    """The common case once P19 ships, and the reason the sentence has two forms.

    A golfer who declares their bag after months of range sessions lands here on every club at
    once. Nothing is being pooled — there is one population of unknown provenance — and telling
    them their carry average mixes two clubs would be both alarming and false. What is actually in
    doubt is whether the make, model and loft on the entry describe the club that hit any of it.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 4, start=1))

    caveats = _caveats(corpus, _bag(ClubId.SEVEN_IRON, recorded=9), ClubId.SEVEN_IRON)

    assert len(caveats) == 1
    assert "All 4 swings" in caveats[0], caveats[0]
    assert "2026-08-09" in caveats[0], caveats[0]
    assert "pool" not in caveats[0], "nothing is pooled when every swing is on the same side"


def test_the_sentence_does_not_say_one_of_these_1_swings() -> None:
    """Plain English on the smallest history there is, which is where the count reads worst.

    A club hit once before its entry was recorded is the state every club passes through, and a
    naive count renders it "1 of these 1 swings were hit" — on a page a golfer reads.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 1, start=1))

    caveat = _caveats(corpus, _bag(ClubId.SEVEN_IRON, recorded=5), ClubId.SEVEN_IRON)[0]

    assert "The single swing behind these numbers predates" in caveat, caveat
    assert "1 swings" not in caveat, caveat


def test_a_swing_hit_in_the_same_instant_the_entry_was_recorded_does_not_predate_it() -> None:
    """The `<` / `<=` boundary, which nothing else in this file would catch.

    An entry recorded at the moment a swing was captured *was* the club that hit it. Off by one
    here, every bag declared during a session caveats the session it was declared in.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 3, start=4))

    assert _caveats(corpus, _bag(ClubId.SEVEN_IRON, recorded=4), ClubId.SEVEN_IRON) == []


def test_a_club_with_no_bag_entry_has_nothing_to_be_inconsistent_with() -> None:
    """No entry, no date, no caveat — for an undeclared club and for a retired one alike.

    The retired half is the one worth writing out: `bag_entry` is deliberately not filled from
    `Bag.retired`, so a club that has left the bag reaches here with `None` and must stay silent
    rather than borrowing the shelf's `recorded_at` to caveat itself.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 3, start=5))

    assert _caveats(corpus, None, ClubId.SEVEN_IRON) == []
    assert _caveats(corpus, _bag(retired=(ClubId.SEVEN_IRON,), recorded=9), ClubId.SEVEN_IRON) == []


def test_a_declared_but_unhit_club_is_pooling_nothing_and_so_says_nothing() -> None:
    """The state every club is in the day it is added to the bag, and the empty-`swings` path.

    There is no history to be two populations of yet. The guard is also what stops the helper
    reaching for the earliest of an empty list.
    """
    assert _caveats(_corpus(), _bag(ClubId.DRIVER, recorded=9), ClubId.DRIVER) == []


def test_the_caveat_qualifies_the_statistics_and_never_withholds_them() -> None:
    """The load-bearing one: this phase must not be able to delete history.

    Six 7-iron shots clear the five-sample floor, and a bag entry recorded midway through them is
    the likelier case of a golfer declaring their bag late — not evidence that the club changed.
    Wiring this judgment to a refusal instead of a sentence would withhold a mean the golfer has
    the shots for, on the strength of when they got round to typing in their bag.
    """
    corpus = _corpus(*_hits(ClubId.SEVEN_IRON, 6))

    profile = build_bag_profile(corpus, _bag(ClubId.SEVEN_IRON, recorded=4)).profile_for(
        ClubId.SEVEN_IRON
    )
    assert profile is not None

    assert len(profile.caveats) == 1
    assert BaselineClaim.CENTER in profile.metrics[CARRY].ready
    assert profile.metrics[CARRY].mean is not None
    assert profile.metrics[CARRY].n == 6, "the caveat must not narrow what the mean is built from"


# --------------------------------------------------------------------- the mishit caveat [M16 P4]


def test_a_mishit_is_counted_named_and_left_out_of_the_carry_mean_only() -> None:
    corpus = _corpus(
        *_hits(ClubId.SEVEN_IRON, 6),  # carries 151..156
        _swing(9, club=ClubId.SEVEN_IRON, carry=20.0, auto_mishit=True),
    )

    irons = build_bag_profile(corpus).profile_for(ClubId.SEVEN_IRON)
    assert irons is not None

    assert irons.n_shots == 7, "the top is still a shot"
    assert irons.mishits == 1
    assert irons.mishit_refs == ["2026-08-09/9"]
    assert irons.mishits_unconfirmed == 1
    assert irons.metrics[CARRY].n == 6, "but not a carry sample"
    assert any("set aside as a mishit" in c for c in irons.caveats)
    assert any("7 shots on this club" in c for c in irons.caveats)


def test_a_confirmed_mishit_reads_as_ruled_on_not_waiting() -> None:
    corpus = _corpus(
        *_hits(ClubId.SEVEN_IRON, 6),
        _swing(9, club=ClubId.SEVEN_IRON, carry=110.0, mishit=MishitVerdict.CONFIRMED),
    )
    irons = build_bag_profile(corpus).profile_for(ClubId.SEVEN_IRON)
    assert irons is not None

    assert irons.mishits == 1
    assert irons.mishits_unconfirmed == 0
    assert irons.metrics[CARRY].n == 6
    assert not any("not yet confirmed" in c for c in irons.caveats)


def test_the_bag_profile_sums_mishits_across_clubs() -> None:
    corpus = _corpus(
        *_hits(ClubId.SEVEN_IRON, 6),
        _swing(9, club=ClubId.SEVEN_IRON, carry=20.0, auto_mishit=True),
        *_hits(ClubId.DRIVER, 6, start=13, carry=250.0),
        _swing(20, club=ClubId.DRIVER, carry=30.0, auto_mishit=True),
    )
    assert build_bag_profile(corpus).mishits_excluded == 2


def test_no_mishit_no_caveat_and_zero_counts() -> None:
    irons = build_bag_profile(_corpus(*_hits(ClubId.SEVEN_IRON, 6))).profile_for(ClubId.SEVEN_IRON)
    assert irons is not None
    assert (irons.mishits, irons.mishit_refs, irons.mishits_unconfirmed) == (0, [], 0)
    assert not any("mishit" in c for c in irons.caveats)


# ------------------------------------------------------------------------- the import boundary


def test_the_builder_imports_no_storage() -> None:
    """ADR-008's dependency rule, pinned for the module with the strongest reason to break it.

    `analysis` depends on `contracts` alone. This builder needs a filter that lived in
    `storage/corpus.py` until M9 P15 moved it onto `CareerCorpus.narrowed_to`, and
    `from golf_coach.storage.corpus import narrow_to` still works, still type-checks and still
    passes every other test in this file. R1 and R2 are `[survey]` rules that `/refactor-review`
    reads and nothing enforces, so without this the move would quietly undo itself.

    **Read statically, for the reason `test_comparison.py` gives**: `analysis/__init__.py` imports
    `engine`, so a subprocess `sys.modules` check would be answering a question about the package
    rather than about this file's own imports.
    """
    path = Path(golf_coach.analysis.__file__).parent / "club_profile.py"
    tree = ast.parse(path.read_text(encoding="utf-8"))

    imported = {
        name
        for node in ast.walk(tree)
        for name in (
            [node.module or ""]
            if isinstance(node, ast.ImportFrom)
            else [alias.name for alias in node.names]
            if isinstance(node, ast.Import)
            else []
        )
    }

    assert not any("storage" in name for name in imported), (
        "analysis.club_profile imports golf_coach.storage — `analysis` depends on `contracts` "
        "alone (ADR-008). Narrow with `CareerCorpus.narrowed_to`, which is the contract-side "
        f"filter `storage.corpus.narrow_to` itself delegates to. Imports found: {sorted(imported)}"
    )
