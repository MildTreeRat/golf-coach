"""The cross-session corpus reader — the honest-`n` counter. [Career mode, step 2]

Every property here has the same failure signature: it is invisible. A corpus that counts one
swing three times does not raise, does not warn, and produces a *more* confident baseline than a
correct one — the repeated measurement drives the variance toward zero, and a tight spread is
exactly what step 5 reads as "static cause, check your grip". So the counting rules are pinned
rather than trusted, and so is every path by which a swing leaves the count.

Builders arrive as fixtures (`swing`, `metric`, `analysis`, `shot`) from `conftest.py`.
"""

from __future__ import annotations

from datetime import UTC, datetime

from golf_coach.analysis.baseline import build_baseline
from golf_coach.analysis.comparison import build_standing
from golf_coach.analysis.dispersion import build_dispersion
from golf_coach.contracts.career import ExclusionReason
from golf_coach.contracts.club import ClubId
from golf_coach.contracts.swing import ANALYSIS_VERSION
from golf_coach.storage.corpus import narrow_to, read_corpus

LM = "launch_monitor:hd_golf"
#: The two provenances M15 P12 registered. Spelled out rather than imported from the modules that
#: emit them, so a source string renamed in `analysis` fails here as a corpus-counting change
#: rather than being silently followed.
POPULATION = "population:golfdb"
FLIGHT = "model:flight_v1"


def _at(day: int) -> datetime:
    return datetime(2026, 8, day, 12, tzinfo=UTC)


# --------------------------------------------------------------------------- dedupe


def test_re_uploads_of_one_clip_collapse_to_one_swing(corpus_dir, swing, pose_analysis) -> None:
    """The shape actually on disk: three directories, one swing, three identical numbers."""
    for session_id, swing_id, day in (
        ("2026-08-07", "1", 7),
        ("2026-08-09", "2", 9),
        ("2026-08-10", "1", 10),
    ):
        swing(corpus_dir, session_id, swing_id, face_on="clip-a", created_at=_at(day),
              analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.swing_dirs_seen == 3
    assert corpus.distinct_swings == 1
    assert corpus.metric_counts == {"head_sway_norm": 1}


def test_the_earliest_arrival_survives_and_names_what_it_absorbed(
    corpus_dir, swing, pose_analysis
) -> None:
    """A re-upload's timestamp dates the upload, so the latest must not become `captured_at`."""
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", created_at=_at(10),
          analysis=pose_analysis)
    swing(corpus_dir, "2026-08-07", "1", face_on="clip-a", created_at=_at(7),
          analysis=pose_analysis)

    kept = read_corpus(corpus_dir, "aaron").swings[0]

    assert kept.ref == "2026-08-07/1"
    assert kept.captured_at == _at(7)
    assert kept.duplicates == ["2026-08-10/1"]


def test_a_duplicate_is_reported_not_absorbed(corpus_dir, swing, pose_analysis) -> None:
    swing(corpus_dir, "2026-08-07", "1", face_on="clip-a", created_at=_at(7),
          analysis=pose_analysis)
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", created_at=_at(10),
          analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.duplicates_collapsed == 1
    duplicate = next(e for e in corpus.excluded if e.reason is ExclusionReason.DUPLICATE)
    assert duplicate.ref == "2026-08-10/1"
    assert "2026-08-07/1" in duplicate.detail


def test_different_clips_are_different_swings(corpus_dir, swing, pose_analysis) -> None:
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", shot_screen="shot-a",
          analysis=pose_analysis)
    swing(corpus_dir, "2026-08-10", "2", face_on="clip-b", shot_screen="shot-b",
          analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.distinct_swings == 2
    assert corpus.metric_counts == {"head_sway_norm": 2}


# --------------------------------------------------------------- the two provenance keys


def test_one_clip_with_two_shot_photos_is_a_conflict_not_a_second_reading(
    corpus_dir, swing, analysis, metric
) -> None:
    """One physical swing has one ball flight, so the second photo is misattached, not data.

    Counting it would put a `face_to_path_deg` into the dispersion that no swing ever produced —
    so it is reported for repair and the sample count stays at one on both axes.
    """
    both = analysis([
        metric("head_sway_norm", 0.25),
        metric("face_to_path_deg", 13.2, source=LM, unit="degrees"),
    ])
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", shot_screen="shot-a",
          created_at=_at(10), analysis=both)
    swing(corpus_dir, "2026-08-11", "1", face_on="clip-a", shot_screen="shot-b",
          created_at=_at(11), analysis=both)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.distinct_swings == 1
    assert corpus.metric_counts["head_sway_norm"] == 1
    assert corpus.metric_counts["face_to_path_deg"] == 1
    assert corpus.swings[0].conflicting_shots == ["shot-b"]
    assert corpus.shot_conflicts == 1


def test_matching_shot_photos_across_re_uploads_are_no_conflict(
    corpus_dir, swing, pose_analysis
) -> None:
    """The shape actually on disk — the three re-uploads carry the same photo, so nothing is wrong
    beyond the duplication itself."""
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", shot_screen="shot-a",
          created_at=_at(10), analysis=pose_analysis)
    swing(corpus_dir, "2026-08-11", "1", face_on="clip-a", shot_screen="shot-a",
          created_at=_at(11), analysis=pose_analysis)

    assert read_corpus(corpus_dir, "aaron").shot_conflicts == 0


def test_two_clips_sharing_a_shot_photo_is_two_pose_samples_and_one_shot_sample(
    corpus_dir, swing, analysis, metric
) -> None:
    """The mirror image: two real swings, but one launch-monitor reading attached to both."""
    both = analysis([
        metric("head_sway_norm", 0.25),
        metric("face_to_path_deg", 13.2, source=LM, unit="degrees"),
    ])
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", shot_screen="shot-a", analysis=both)
    swing(corpus_dir, "2026-08-10", "2", face_on="clip-b", shot_screen="shot-a", analysis=both)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.distinct_swings == 2
    assert corpus.distinct_shots == 1
    assert corpus.metric_counts["head_sway_norm"] == 2
    assert corpus.metric_counts["face_to_path_deg"] == 1


def test_an_unrecognised_source_counts_per_swing_and_says_so(
    corpus_dir, swing, analysis, metric
) -> None:
    """A new provenance must not silently inherit a dedupe key that does not fit it."""
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a",
          analysis=analysis([metric("club_lag_deg", 4.0, source="radar:trackman")]))

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.unknown_sources == ["radar:trackman"]
    assert corpus.metric_counts == {"club_lag_deg": 1}


def test_a_placement_is_still_an_unknown_source_and_that_is_the_deferral(
    corpus_dir, swing, analysis, metric
) -> None:
    """M8's placements stay unregistered, and this is the pin that says so out loud.

    M15 P12 registered `model:` and very nearly took this prefix with it, because doing so looks
    free: `read_corpus` groups *by* `face_on_sha256`, so `CorpusSwing` is one-to-one with it and
    the `swing:{ref}` fallback partitions a corpus exactly as `pose:` would. No count moves and
    nothing goes red — which is the hazard, not the reassurance. ADR-022's fourth addendum defers
    whether a distance-from-a-tour-population is a personal quantity at all, and notes that the two
    down-the-line placements are read off a clip `CorpusSwing` carries no hash for. Registering the
    prefix would answer both by accident.
    """
    placement = analysis([metric("tour_joint_distance", 2.4, source=POPULATION, unit="sd")])
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=placement)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.unknown_sources == [POPULATION], (
        "if this now passes empty, ADR-022's fourth addendum has been decided — update it, and "
        "give the two `_dtl` placements a dedupe key that is not the face-on clip's"
    )


def test_a_simulated_flight_dedupes_on_the_shot_photo_and_not_the_clip(
    corpus_dir, swing, analysis, metric, shot
) -> None:
    """ADR-027's flight is a reading of the tile, not of the swing that produced it.

    Two genuinely different swings, one shot photo attached to both — the shape
    `bundle_store`'s "newest swing missing this role" rule can produce. The integrator is fed the
    tile's launch conditions and nothing the body did, so flying it twice is one flight, however
    much the two clips differ. Under `swing:{ref}` this counted 2 and reported a dispersion over a
    single set of launch conditions.
    """
    both = analysis(
        [
            metric("head_sway_norm", 0.25),
            metric("flight_carry_yds", 141.2, source=FLIGHT, unit="yards"),
        ],
        shot=shot(needs_review=False),
    )
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", shot_screen="shot-a", analysis=both)
    swing(corpus_dir, "2026-08-10", "2", face_on="clip-b", shot_screen="shot-a", analysis=both)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.metric_counts["head_sway_norm"] == 2
    assert corpus.metric_counts["flight_carry_yds"] == 1


# --------------------------------------------------------------------------- attribution


def test_another_golfers_swings_are_counted_but_never_included(
    corpus_dir, swing, pose_analysis
) -> None:
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=pose_analysis)
    swing(corpus_dir, "2026-08-10", "2", player_id="dave", face_on="clip-b",
          analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.distinct_swings == 1
    assert corpus.other_golfers == 1
    assert corpus.excluded == []


def test_unattributed_swings_are_itemised_because_they_are_repairable(
    corpus_dir, swing, pose_analysis
) -> None:
    """Unlike another golfer's swing, one naming nobody may still be this golfer's."""
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=pose_analysis)
    swing(corpus_dir, "2026-08-10", "2", player_id=None, face_on="clip-b",
          analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.distinct_swings == 1
    assert corpus.unattributed_swings == 1
    unattributed = next(e for e in corpus.excluded if e.reason is ExclusionReason.UNATTRIBUTED)
    assert unattributed.ref == "2026-08-10/2"


def test_a_swing_with_no_face_on_clip_can_never_carry_a_pose_measurement(
    corpus_dir, swing, pose_analysis
) -> None:
    swing(corpus_dir, "2026-08-10", "1", face_on=None, analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.distinct_swings == 0
    assert corpus.excluded[0].reason is ExclusionReason.NO_FACE_ON


# ----------------------------------------------------------------------------- the club


def test_a_tagged_manifest_carries_its_club_into_the_corpus(
    corpus_dir, swing, pose_analysis
) -> None:
    """The read itself. [M9 P12]"""
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", club=ClubId.SEVEN_IRON,
          analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.swings[0].club is ClubId.SEVEN_IRON
    assert corpus.untagged_swings == 0


def test_a_manifest_written_before_the_club_existed_reads_as_none(
    corpus_dir, swing, pose_analysis
) -> None:
    """Every swing on disk today looks like this, and not one of them is a fault to report.

    `None` is the pre-M9 state and the only thing it means, because the upload route has refused an
    untagged swing since P6. It is counted so a per-club refusal can say how much history it cannot
    see, and never itemised in `excluded`, which is reserved for swings contributing no sample.
    """
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.swings[0].club is None
    assert corpus.untagged_swings == 1
    assert corpus.excluded == []


def test_an_untagged_swing_still_contributes_to_every_metric_count(
    corpus_dir, swing, analysis, metric
) -> None:
    """The design note as an assertion. [ADR-024, Consequences]

    Excluding an untagged swing would shrink the mechanics `n` to punish a missing tag mechanics
    never needed — the club is not an input to measuring head sway. So the tagged swing and the
    untagged one pool into a single count of 2, and `build_baseline` must average both values
    behind it: a count that included the untagged swing while the pooling did not would be the
    printed-`n`-disagrees-with-the-values failure `artifact_key` exists to prevent, arriving by a
    new route.
    """
    swing(corpus_dir, "2026-08-07", "1", face_on="clip-a", club=ClubId.SEVEN_IRON,
          created_at=_at(7), analysis=analysis([metric("head_sway_norm", 0.22)]))
    swing(corpus_dir, "2026-08-08", "1", face_on="clip-b", created_at=_at(8),
          analysis=analysis([metric("head_sway_norm", 0.26)]))

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.metric_counts == {"head_sway_norm": 2}
    assert corpus.untagged_swings == 1
    assert corpus.excluded == []
    assert build_baseline(corpus).metrics["head_sway_norm"].n == 2


def test_the_survivor_names_the_club_not_the_re_upload_whose_cursor_moved_on(
    corpus_dir, swing, pose_analysis
) -> None:
    """One clip, two directories, two clubs — and the earliest arrival is the one that was there.

    Every upload stamps the session cursor as it stood when *that* file arrived, so a clip re-sent
    after the golfer moved on to a wedge carries a wedge. It is the same swing, so the later tag is
    a stale reading rather than a competing one, and taking it would rename a shot already recorded
    correctly. Deliberately not surfaced as a conflict the way a second *shot photo* is: that names
    a repair on a swing being scored, this names a directory that contributes nothing.
    """
    swing(corpus_dir, "2026-08-07", "1", face_on="clip-a", club=ClubId.SEVEN_IRON,
          created_at=_at(7), analysis=pose_analysis)
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", club=ClubId.SAND_WEDGE,
          created_at=_at(10), analysis=pose_analysis)

    kept = read_corpus(corpus_dir, "aaron").swings[0]

    assert kept.ref == "2026-08-07/1"
    assert kept.club is ClubId.SEVEN_IRON
    assert kept.duplicates == ["2026-08-10/1"]


def test_an_untagged_survivor_does_not_borrow_a_re_uploads_club(
    corpus_dir, swing, pose_analysis
) -> None:
    """The other direction, written out because a different assertion catches it. [P4's lesson]

    A pre-M9 clip re-sent today arrives with whatever club is selected now, which is evidence about
    the cursor and not about the swing. Filling the survivor's `None` from it would be exactly the
    guess `parse_club` refuses at the boundary (R7); the repair route is per swing and human.
    """
    swing(corpus_dir, "2026-08-07", "1", face_on="clip-a", created_at=_at(7),
          analysis=pose_analysis)
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", club=ClubId.SAND_WEDGE,
          created_at=_at(10), analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.swings[0].club is None
    assert corpus.untagged_swings == 1


def test_untagged_swings_counts_distinct_swings_not_directories(
    corpus_dir, swing, pose_analysis
) -> None:
    """The property's shape, and where it parts company with `unattributed_swings`.

    That counter is tallied over manifests during the scan, because an unattributed one never
    becomes a `CorpusSwing` at all. This one is derived from `swings`, so two untagged directories
    holding one untagged swing count 1 — the same collapse `distinct_swings` performs, which is
    what lets the two be printed as "1 of 2" without the sentence being a lie.
    """
    swing(corpus_dir, "2026-08-07", "1", face_on="clip-a", club=ClubId.SEVEN_IRON,
          created_at=_at(7), analysis=pose_analysis)
    for session_id, day in (("2026-08-08", 8), ("2026-08-09", 9)):
        swing(corpus_dir, session_id, "1", face_on="clip-b", created_at=_at(day),
              analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.swing_dirs_seen == 3
    assert corpus.distinct_swings == 2
    assert corpus.untagged_swings == 1


# --------------------------------------------------------------------- trust and staleness


def test_a_stale_analysis_is_a_real_swing_that_contributes_nothing(
    corpus_dir, swing, pose_analysis
) -> None:
    """Its numbers describe bytes that are no longer on disk, so they are not a sample of it."""
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=pose_analysis, stale=True)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.distinct_swings == 1
    assert corpus.swings[0].stale is True
    assert corpus.metric_counts == {}
    assert corpus.excluded[0].reason is ExclusionReason.STALE


def test_an_unanalyzed_swing_is_counted_as_a_swing_but_not_as_a_sample(
    corpus_dir, swing
) -> None:
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=None)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.distinct_swings == 1
    assert corpus.swings[0].analyzed is False
    assert corpus.metric_counts == {}
    assert corpus.excluded[0].reason is ExclusionReason.NOT_ANALYZED


def test_a_flagged_shot_contributes_to_no_launch_monitor_count(
    corpus_dir, swing, analysis, metric, shot
) -> None:
    """Same rule `get_session_summary` applies: a flagged parse must not move a number (ADR-014)."""
    flagged = analysis(
        [
            metric("head_sway_norm", 0.25),
            metric("face_to_path_deg", 13.2, source=LM, unit="degrees"),
        ],
        shot=shot(),
    )
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=flagged)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.swings[0].shot_needs_review is True
    assert corpus.metric_counts == {"head_sway_norm": 1}


def test_a_trusted_shot_does_contribute(corpus_dir, swing, analysis, metric, shot) -> None:
    trusted = analysis(
        [metric("face_to_path_deg", 13.2, source=LM, unit="degrees")],
        shot=shot(needs_review=False),
    )
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=trusted)

    assert read_corpus(corpus_dir, "aaron").metric_counts == {"face_to_path_deg": 1}


def test_a_flagged_parse_takes_the_simulated_flight_with_it(
    corpus_dir, swing, analysis, metric, shot
) -> None:
    """The half of M15 P12 that moves a number, and the reason `model:` keys on the shot photo.

    A flight is integrated *from* the flagged tile's launch conditions, so it is exactly as suspect
    as the tile — more so, since the model's own error rides on top. Before P12 it fell through to
    `swing:{ref}`, which has no flagged-parse refusal in it, and a simulated carry counted as a
    sample while the `carry_distance_yds` printed beside it on that same screen did not.
    """
    flagged = analysis(
        [
            metric("head_sway_norm", 0.25),
            metric("flight_carry_yds", 141.2, source=FLIGHT, unit="yards"),
        ],
        shot=shot(),
    )
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=flagged)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.swings[0].shot_needs_review is True
    assert corpus.metric_counts == {"head_sway_norm": 1}


# ------------------------------------------------------------------ the engine-generation axis


def test_an_artifact_from_an_older_engine_is_a_real_swing_that_contributes_nothing(
    corpus_dir, swing, analysis, metric
) -> None:
    """The case nothing could see before the version stamp.

    Its inputs are untouched, so `AnalysisState.matches` passes and it is *not* stale — yet the
    numbers were produced by code that has since moved, and pooling them with a swing analyzed
    today would put a difference in the spread that no golfer ever swung.
    """
    old = analysis([metric("head_sway_norm", 0.25)], version=None)
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=old)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.distinct_swings == 1
    assert corpus.swings[0].stale is False
    assert corpus.swings[0].outdated is True
    assert corpus.swings[0].analysis_version == 0
    assert corpus.outdated_swings == 1
    assert corpus.metric_counts == {}
    assert corpus.excluded[0].reason is ExclusionReason.OUTDATED


def test_a_superseded_version_is_outdated_the_same_way_an_unstamped_one_is(
    corpus_dir, swing, analysis, metric
) -> None:
    """`version=None` and `version=ANALYSIS_VERSION - 1` are the same finding by different routes:
    one predates the stamp, one was superseded by a bump. Only the second can exist after today."""
    superseded = analysis([metric("head_sway_norm", 0.25)], version=ANALYSIS_VERSION - 1)
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=superseded)

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.swings[0].outdated is True
    assert corpus.metric_counts == {}


def test_a_current_artifact_with_no_measurements_is_reported_not_reconstructed(
    corpus_dir, swing, analysis
) -> None:
    """Measurement failed, rather than the engine being old — a different problem, reported apart
    from `outdated_swings`. `checkpoint_scores[0].observed` is 2.35 and must stay out of the
    corpus either way: reading it would mix two derivation paths under one metric name."""
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=analysis(None))

    corpus = read_corpus(corpus_dir, "aaron")

    assert corpus.distinct_swings == 1
    assert corpus.outdated_swings == 0
    assert corpus.analyzed_without_measurements == 1
    assert corpus.metric_counts == {}
    assert corpus.swings[0].measurements == []


# --------------------------------------------------------------------------- empty cases


def test_an_unknown_golfer_reads_as_an_empty_corpus(corpus_dir, swing, pose_analysis) -> None:
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-a", analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "nobody")

    assert corpus.distinct_swings == 0
    assert corpus.other_golfers == 1


def test_a_missing_sessions_directory_is_not_an_error(tmp_path) -> None:
    corpus = read_corpus(tmp_path / "nothing-here", "aaron")

    assert corpus.distinct_swings == 0
    assert corpus.swing_dirs_seen == 0
    assert corpus.sessions_scanned == 0


# --------------------------------------------------------------------------- ordering


def test_swings_are_ordered_oldest_first(corpus_dir, swing, pose_analysis) -> None:
    """A history read out of order is a trend line drawn backwards."""
    swing(corpus_dir, "2026-08-11", "1", face_on="clip-c", created_at=_at(11),
          analysis=pose_analysis)
    swing(corpus_dir, "2026-08-07", "1", face_on="clip-a", created_at=_at(7),
          analysis=pose_analysis)
    swing(corpus_dir, "2026-08-09", "1", face_on="clip-b", created_at=_at(9),
          analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")

    assert [s.ref for s in corpus.swings] == ["2026-08-07/1", "2026-08-09/1", "2026-08-11/1"]


# --------------------------------------------------------------------------- the step-4 seam


def test_the_printed_n_is_the_number_of_values_the_baseline_pools(
    corpus_dir, swing, analysis, metric, shot
) -> None:
    """`metric_counts` and `PersonalBaseline` must key on the same artifacts. [Career mode, step 4]

    The reader counts samples and `analysis.baseline` pools the values behind those counts. They
    used to be able to disagree — the keying rule lived privately in `_count_metrics` and returned
    counts only — and a disagreement would surface exactly and only in the cases the rule exists
    for, printing an `n` next to a mean taken over a different number of values.

    Both now call `CorpusSwing.artifact_key`, so this is a regression test on that seam rather
    than on either side. It lives here because the disk builders do; `tests/` has no `__init__.py`,
    so a sibling test package cannot reach these fixtures.

    The corpus below is every divergence at once: a re-upload, one photo across two real swings,
    and a flagged parse.
    """
    both = [metric("head_sway_norm", 0.21), metric("face_to_path_deg", 10.9, source=LM,
                                                   unit="degrees")]

    swing(corpus_dir, "2026-08-07", "1", face_on="clip-a", shot_screen="photo-1",
          created_at=_at(7), analysis=analysis(both, shot=shot(needs_review=False)))
    # A re-upload of clip-a: one swing, not two.
    swing(corpus_dir, "2026-08-08", "1", face_on="clip-a", shot_screen="photo-1",
          created_at=_at(8), analysis=analysis(both, shot=shot(needs_review=False)))
    # A different swing carrying the *same* photo: two pose readings, one launch-monitor reading.
    swing(corpus_dir, "2026-08-09", "1", face_on="clip-b", shot_screen="photo-1",
          created_at=_at(9), analysis=analysis(both, shot=shot(needs_review=False)))
    # A third swing whose OCR was flagged: its pose counts, its shot numbers do not.
    swing(corpus_dir, "2026-08-10", "1", face_on="clip-c", shot_screen="photo-2",
          created_at=_at(10), analysis=analysis(both, shot=shot(needs_review=True)))

    corpus = read_corpus(corpus_dir, "aaron")
    baseline = build_baseline(corpus)
    dispersion = build_dispersion(corpus)
    standing = build_standing(corpus)

    assert corpus.metric_counts == {"face_to_path_deg": 1, "head_sway_norm": 3}
    assert set(baseline.metrics) == set(corpus.metric_counts)
    for name, count in corpus.metric_counts.items():
        assert baseline.metrics[name].n == count, f"{name}: printed {count}, pooled a different n"
        # Steps 5 and 6 consume the baseline rather than re-deriving from the corpus, so their
        # agreeing is structural — and that is exactly why it is worth pinning. The day one of
        # them reaches for `pooled_samples` directly to get at something the guard withheld, this
        # is the assertion that notices.
        assert dispersion.metrics[name].n == count, f"{name}: dispersion counted a different n"
        assert standing.metrics[name].n == count, f"{name}: the tour join counted a different n"


# ------------------------------------------------------------------------- narrowing


def test_narrowing_recomputes_the_counts_it_leaves_behind(
    corpus_dir, swing, analysis, metric
) -> None:
    """A window or a session filter must move `metric_counts` with `swings`. [Career mode, step 6]

    `get_shot_trends` narrows by date and `compare_sessions` narrows by session id, and both hand
    the result straight to `build_baseline`. A filtered swing list beside the unfiltered counts is
    a corpus whose printed `n` describes a different set of swings than its values do — the same
    invisible disagreement the test above exists to prevent, arriving by a different route.
    """
    for day in (7, 8, 9):
        swing(corpus_dir, f"2026-08-{day:02d}", "1", face_on=f"clip-{day}",
              created_at=_at(day), analysis=analysis([metric("head_sway_norm", 0.2 + day / 100)]))

    corpus = read_corpus(corpus_dir, "aaron")
    since = narrow_to(corpus, since=_at(8))
    one_session = narrow_to(corpus, sessions={"2026-08-07"})

    assert corpus.metric_counts == {"head_sway_norm": 3}
    assert since.metric_counts == {"head_sway_norm": 2}
    assert one_session.metric_counts == {"head_sway_norm": 1}
    assert build_baseline(since).metrics["head_sway_norm"].n == 2
    assert [s.session_id for s in one_session.swings] == ["2026-08-07"]


def test_narrowing_leaves_the_scan_counters_describing_the_whole_read(
    corpus_dir, swing, analysis, metric
) -> None:
    """`swing_dirs_seen` and `excluded` are facts about what was on disk, which a window does not
    change. Documented rather than filtered, because nothing downstream of a narrowed corpus reads
    them — and silently halving a count of directories nobody re-scanned would be its own lie."""
    for day in (7, 8):
        swing(corpus_dir, f"2026-08-{day:02d}", "1", face_on=f"clip-{day}",
              created_at=_at(day), analysis=analysis([metric("head_sway_norm", 0.25)]))

    corpus = read_corpus(corpus_dir, "aaron")
    narrowed = narrow_to(corpus, sessions={"2026-08-07"})

    assert narrowed.distinct_swings == 1
    assert narrowed.swing_dirs_seen == corpus.swing_dirs_seen == 2


def test_narrowing_to_a_club_recomputes_the_counts_with_the_swings(
    corpus_dir, swing, analysis, metric
) -> None:
    """The per-club `n` has to be the club's, not the corpus's. [M9 P13]

    This is the same failure the two tests above pin, arriving by the narrowing M9 added — and it
    is worse here, because the whole point of a per-club view is to answer "how far do you hit
    this club". A filtered swing list beside the whole bag's counts would let `build_baseline`
    clear the five-sample floor on a club that has been hit twice.
    """
    for day, club in ((7, ClubId.SEVEN_IRON), (8, ClubId.SEVEN_IRON), (9, ClubId.SAND_WEDGE)):
        swing(corpus_dir, f"2026-08-{day:02d}", "1", face_on=f"clip-{day}",
              shot_screen=f"shot-{day}", club=club, created_at=_at(day),
              analysis=analysis([metric("carry_distance_yds", 150.0 + day, source=LM,
                                        unit="yards")]))

    corpus = read_corpus(corpus_dir, "aaron")
    irons = narrow_to(corpus, club=ClubId.SEVEN_IRON)

    assert corpus.metric_counts == {"carry_distance_yds": 3}
    assert irons.metric_counts == {"carry_distance_yds": 2}
    assert [s.session_id for s in irons.swings] == ["2026-08-07", "2026-08-08"]
    assert build_baseline(irons).metrics["carry_distance_yds"].n == 2


def test_narrowing_to_an_unhit_club_is_an_empty_corpus_not_an_error(
    corpus_dir, swing, pose_analysis
) -> None:
    """"You have not hit your driver yet" is a real answer, and the first one every club has.

    Same call `read_corpus` already makes for an unknown `player_id`. A raise here would make the
    bag page's empty state an exception path rather than a sentence.
    """
    swing(corpus_dir, "2026-08-07", "1", face_on="clip-a", club=ClubId.SEVEN_IRON,
          created_at=_at(7), analysis=pose_analysis)

    empty = narrow_to(read_corpus(corpus_dir, "aaron"), club=ClubId.DRIVER)

    assert empty.swings == []
    assert empty.metric_counts == {}
    assert empty.distinct_swings == 0


def test_club_and_since_compose(corpus_dir, swing, analysis, metric) -> None:
    """"Your 7 iron over the last month" is one call, not two passes with a list in between."""
    for day, club in ((7, ClubId.SEVEN_IRON), (9, ClubId.SEVEN_IRON), (9, ClubId.SAND_WEDGE)):
        swing(corpus_dir, f"2026-08-{day:02d}", club.value, face_on=f"clip-{day}-{club.value}",
              club=club, created_at=_at(day),
              analysis=analysis([metric("head_sway_norm", 0.25)]))

    both = narrow_to(read_corpus(corpus_dir, "aaron"), since=_at(8), club=ClubId.SEVEN_IRON)

    assert [s.club for s in both.swings] == [ClubId.SEVEN_IRON]
    assert [s.captured_at for s in both.swings] == [_at(9)]
    assert both.metric_counts == {"head_sway_norm": 1}


def test_a_club_narrowing_drops_untagged_swings_and_untagged_swings_follows(
    corpus_dir, swing, pose_analysis
) -> None:
    """The asymmetry ADR-024 argues for, pinned from both ends. [M9 P13]

    `read_corpus` keeps an untagged swing — the club was never an input to measuring head sway —
    so it is here, in the one view where the tag is load-bearing, that it drops out. And
    `untagged_swings` has to follow it: a narrowed corpus reporting the whole read's figure beside
    a filtered swing list is the printed-`n`-describes-a-different-set failure `narrow_to` exists
    to prevent. It follows for free only because the counter is derived rather than stored, which
    is what this assertion is really pinning — convert it to a field and this goes red.
    """
    swing(corpus_dir, "2026-08-07", "1", face_on="clip-a", club=ClubId.SEVEN_IRON,
          created_at=_at(7), analysis=pose_analysis)
    swing(corpus_dir, "2026-08-08", "1", face_on="clip-b", created_at=_at(8),
          analysis=pose_analysis)

    corpus = read_corpus(corpus_dir, "aaron")
    irons = narrow_to(corpus, club=ClubId.SEVEN_IRON)

    assert corpus.untagged_swings == 1
    assert corpus.metric_counts == {"head_sway_norm": 2}
    assert irons.untagged_swings == 0
    assert irons.metric_counts == {"head_sway_norm": 1}
