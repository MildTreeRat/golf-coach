//! The strengths and weaknesses list, ADR-034 §5.7, against hand-built dispersions. [M37 P8]
//!
//! Every fixture is a `MetricDispersion` decided the way `analysis::dispersion` decides one, so no
//! test leans on a finding the builder could never produce. That rule is restated in [`decided`]
//! rather than called, because this crate may not reach `analysis` (the last test pins that edge).
//! The hand-worked profile cases (`spec/vectors/profile/hand/`) carry the lists as worked; they are
//! compared against the whole `run_profile` answer in `crates/core`'s gate (M37 P9), the one crate
//! that holds the dispersion builder and this rule together.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use contracts::baseline::Interval;
use contracts::club::ClubId;
use contracts::club_profile::ClubProfile;
use contracts::dispersion::{
    target_for, DispersionPattern, Finding, GolferDispersion, MetricDispersion,
};
use contracts::strike::{is_club_independent, FindingKind, ProfileFinding};
use contracts::Validate;
use feedback::profile_rules::{rank_player, rank_profile};

const LM: &str = "launch_monitor:synthetic";

fn interval(low: f64, high: f64) -> Interval {
    Interval {
        low,
        high,
        confidence: Interval::DEFAULT_CONFIDENCE,
    }
}

/// One row's dispersion, decided as `analysis::dispersion::dispersion_for` decides it: tolerance and
/// target from `METRIC_TARGETS`, a bias only against a target and only when the mean's interval
/// clears the band, scatter when the SD's lower bound clears the tolerance, and a pattern only when
/// both were answered. `center` is `(mean, low, high)` and `spread` is `(sd, low, high)`; `None` is a
/// claim withheld for want of `n`.
fn decided(
    name: &str,
    source: &str,
    center: Option<(f64, f64, f64)>,
    spread: Option<(f64, f64, f64)>,
) -> MetricDispersion {
    let declared = target_for(name);
    let mut metric = MetricDispersion {
        name: name.to_string(),
        unit: String::new(),
        source: source.to_string(),
        n: 20,
        n_sessions: 2,
        target: declared.and_then(|row| row.target),
        tolerance: declared.map(|row| row.tolerance),
        bias: Finding::Withheld,
        center: None,
        center_ci: None,
        offset: None,
        scatter: Finding::Withheld,
        sd: None,
        sd_ci: None,
        within_session_sd: None,
        pattern: None,
        points_at: None,
        caveats: Vec::new(),
        withheld: Vec::new(),
        unavailable: Vec::new(),
    };
    let Some(declared) = declared else {
        return metric;
    };
    let tolerance = declared.tolerance;
    if let (Some(target), Some((mean, low, high))) = (declared.target, center) {
        metric.center = Some(mean);
        metric.center_ci = Some(interval(low, high));
        metric.offset = Some(mean - target);
        metric.bias = if low > target + tolerance || high < target - tolerance {
            Finding::Established
        } else {
            Finding::NotEstablished
        };
    }
    if let Some((sd, low, high)) = spread {
        metric.sd = Some(sd);
        metric.sd_ci = Some(interval(low, high));
        metric.scatter = if low > tolerance {
            Finding::Established
        } else {
            Finding::NotEstablished
        };
    }
    if metric.bias != Finding::Withheld && metric.scatter != Finding::Withheld {
        metric.pattern = Some(
            match (
                metric.bias == Finding::Established,
                metric.scatter == Finding::Established,
            ) {
                (true, true) => DispersionPattern::BiasedAndScattered,
                (true, false) => DispersionPattern::Biased,
                (false, true) => DispersionPattern::Scattered,
                (false, false) => DispersionPattern::NothingEstablished,
            },
        );
    }
    metric
}

/// A launch-monitor row.
fn shot(
    name: &str,
    center: Option<(f64, f64, f64)>,
    spread: Option<(f64, f64, f64)>,
) -> MetricDispersion {
    decided(name, LM, center, spread)
}

fn by_name(metrics: Vec<MetricDispersion>) -> BTreeMap<String, MetricDispersion> {
    metrics
        .into_iter()
        .map(|metric| (metric.name.clone(), metric))
        .collect()
}

fn club(metrics: Vec<MetricDispersion>) -> ClubProfile {
    ClubProfile {
        club: ClubId::SevenIron,
        bag_entry: None,
        in_bag: true,
        n_swings: 20,
        n_shots: 20,
        n_sessions: 2,
        mishits: 0,
        mishit_refs: Vec::new(),
        mishits_unconfirmed: 0,
        metrics: BTreeMap::new(),
        dispersion: by_name(metrics),
        caveats: Vec::new(),
    }
}

fn golfer(metrics: Vec<MetricDispersion>) -> GolferDispersion {
    GolferDispersion {
        player_id: "hand".to_string(),
        metrics: by_name(metrics),
        built_from_swings: 20,
        built_from_sessions: 2,
    }
}

/// Every list holds what `contracts::strike`'s check holds a list to: each finding valid, and the
/// weaknesses ahead of the strengths.
fn checked(list: Vec<ProfileFinding>) -> Vec<ProfileFinding> {
    for finding in &list {
        finding
            .validate()
            .unwrap_or_else(|e| panic!("{}: {e:?}", finding.metric));
    }
    assert!(
        list.windows(2)
            .all(|pair| pair[0].kind.is_weakness() || !pair[1].kind.is_weakness()),
        "a weakness listed after a strength: {list:?}"
    );
    list
}

fn listed(list: &[ProfileFinding]) -> Vec<(&str, FindingKind, f64)> {
    list.iter()
        .map(|finding| (finding.metric.as_str(), finding.kind, finding.ratio))
        .collect()
}

/// Weaknesses rank largest ratio first, each ratio the point estimate over the tolerance, whatever
/// the metric's unit: 3 degrees of face (tolerance 2.0) outranks 12 yards of carry (5.0) only by
/// what each exceeds its own instrument's error by.
#[test]
fn a_bias_and_a_scatter_rank_by_their_ratio_to_tolerance() {
    let list = checked(rank_profile(&club(vec![
        // A 5° open face to the path, established: ratio 5.0 / 2.0.
        shot(
            "face_to_path_deg",
            Some((5.0, 4.0, 6.0)),
            Some((1.0, 0.5, 1.5)),
        ),
        // Carry scatter, no target: sd 15 over 5 yards.
        shot("carry_distance_yds", None, Some((15.0, 11.0, 20.0))),
        // Start-line scatter: sd 3.0 over 2.0.
        shot(
            "start_line_deg",
            Some((0.0, -1.0, 1.0)),
            Some((3.0, 2.5, 4.0)),
        ),
    ])));
    assert_eq!(
        listed(&list),
        vec![
            ("carry_distance_yds", FindingKind::Scatter, 3.0),
            ("face_to_path_deg", FindingKind::Bias, 2.5),
            ("start_line_deg", FindingKind::Scatter, 1.5),
        ]
    );
}

/// A bias's ratio is its offset's size: a draw bias ranks as a fade bias of the same magnitude.
#[test]
fn a_bias_ranks_by_its_size_whichever_side_it_misses() {
    let list = checked(rank_profile(&club(vec![
        shot(
            "face_to_path_deg",
            Some((-6.0, -7.0, -5.0)),
            Some((1.0, 0.5, 1.5)),
        ),
        shot(
            "start_line_deg",
            Some((5.0, 4.0, 6.0)),
            Some((1.0, 0.5, 1.5)),
        ),
    ])));
    assert_eq!(
        listed(&list),
        vec![
            ("face_to_path_deg", FindingKind::Bias, 3.0),
            ("start_line_deg", FindingKind::Bias, 2.5),
        ]
    );
}

/// At an equal ratio, `METRIC_TARGETS` order decides and then bias before scatter. The map is
/// alphabetical, and `carry_distance_yds` sorts ahead of `start_line_offline_yds` there while the
/// table puts it after, so the map's order would answer differently.
#[test]
fn a_tie_falls_to_metric_targets_order_then_bias_before_scatter() {
    let list = checked(rank_profile(&club(vec![
        shot("carry_distance_yds", None, Some((10.0, 8.0, 13.0))),
        shot(
            "start_line_offline_yds",
            Some((0.0, -2.0, 2.0)),
            Some((18.0, 14.0, 24.0)),
        ),
        // Both kinds on one metric at one ratio: offset 4 and sd 4, over 2.0.
        shot(
            "face_to_path_deg",
            Some((4.0, 3.0, 5.0)),
            Some((4.0, 3.0, 5.5)),
        ),
    ])));
    assert_eq!(
        listed(&list),
        vec![
            ("face_to_path_deg", FindingKind::Bias, 2.0),
            ("face_to_path_deg", FindingKind::Scatter, 2.0),
            ("start_line_offline_yds", FindingKind::Scatter, 2.0),
            ("carry_distance_yds", FindingKind::Scatter, 2.0),
        ]
    );
}

/// A strength needs the mean's interval inside target ± tolerance *and* the SD's upper bound below
/// the tolerance; either half alone lists nothing, and neither half is a weakness either.
#[test]
fn a_strength_needs_both_halves() {
    let both = shot(
        "start_line_deg",
        Some((0.2, -0.5, 0.9)),
        Some((1.0, 0.8, 1.4)),
    );
    let list = checked(rank_profile(&club(vec![both])));
    assert_eq!(
        listed(&list),
        vec![("start_line_deg", FindingKind::Strength, 1.4 / 2.0)]
    );

    // Centered and tight, but the SD's interval reaches past the tolerance: not shown to be inside
    // measurement error, and not shown to be outside it.
    let centered_only = shot(
        "start_line_deg",
        Some((0.2, -0.5, 0.9)),
        Some((1.6, 1.2, 2.6)),
    );
    // Tight, but the mean's interval straddles the band's edge.
    let tight_only = shot(
        "start_line_deg",
        Some((1.5, 0.6, 2.4)),
        Some((1.0, 0.8, 1.4)),
    );
    for metric in [centered_only, tight_only] {
        assert_eq!(
            (metric.bias, metric.scatter),
            (Finding::NotEstablished, Finding::NotEstablished)
        );
        assert!(rank_profile(&club(vec![metric])).is_empty());
    }
}

/// The edges as worked by hand (the M37 plan's P5 finding 6): the mean's interval may touch the
/// band's edge, and the SD's upper bound may not touch the tolerance.
#[test]
fn the_strength_edges_are_inclusive_for_the_mean_and_strict_for_the_sd() {
    // An interval ending exactly on both edges is inside, and its ratio is exactly 1.
    let on_the_edge = shot(
        "start_line_deg",
        Some((0.0, -2.0, 2.0)),
        Some((1.0, 0.8, 1.4)),
    );
    assert_eq!(
        listed(&checked(rank_profile(&club(vec![on_the_edge])))),
        vec![("start_line_deg", FindingKind::Strength, 1.0)]
    );

    let past_the_edge = shot(
        "start_line_deg",
        Some((0.0, -2.0, 2.0f64.next_up())),
        Some((1.0, 0.8, 1.4)),
    );
    let sd_at_the_tolerance = shot(
        "start_line_deg",
        Some((0.0, -0.5, 0.5)),
        Some((1.5, 1.0, 2.0)),
    );
    let sd_just_below = shot(
        "start_line_deg",
        Some((0.0, -0.5, 0.5)),
        Some((1.5, 1.0, 2.0f64.next_down())),
    );
    assert!(rank_profile(&club(vec![past_the_edge])).is_empty());
    assert!(rank_profile(&club(vec![sd_at_the_tolerance])).is_empty());
    assert_eq!(
        listed(&checked(rank_profile(&club(vec![sd_just_below])))),
        vec![(
            "start_line_deg",
            FindingKind::Strength,
            2.0f64.next_down() / 2.0
        )]
    );
}

/// A strength's ratio is whichever bound is nearest to failing, and strengths list smallest first,
/// ties in `METRIC_TARGETS` order.
#[test]
fn a_strengths_ratio_is_its_bound_nearest_to_failing() {
    let list = checked(rank_profile(&club(vec![
        // The low end of the mean's interval is furthest out: 1.2 of 2.0.
        shot(
            "face_to_path_deg",
            Some((-0.4, -1.2, 0.4)),
            Some((0.6, 0.4, 0.8)),
        ),
        // The high end: 0.9 of 2.0.
        shot(
            "start_line_deg",
            Some((0.3, -0.3, 0.9)),
            Some((0.5, 0.3, 0.7)),
        ),
        // The SD's upper bound: 6.3 of 9.0.
        shot(
            "start_line_offline_yds",
            Some((0.5, -1.0, 2.0)),
            Some((4.0, 3.0, 6.3)),
        ),
    ])));
    assert_eq!(
        listed(&list),
        vec![
            ("start_line_deg", FindingKind::Strength, 0.9 / 2.0),
            ("face_to_path_deg", FindingKind::Strength, 1.2 / 2.0),
            ("start_line_offline_yds", FindingKind::Strength, 6.3 / 9.0),
        ]
    );

    // An exact tie lists in `METRIC_TARGETS` order. For the three metrics that can be a strength
    // today (the launch-monitor rows with a target) that is also the map's alphabetical order, so
    // this pins the outcome and cannot tell which rule produced it; the weaknesses' tie above can.
    let tied = checked(rank_profile(&club(vec![
        shot(
            "start_line_deg",
            Some((0.0, -1.0, 1.0)),
            Some((0.5, 0.3, 0.7)),
        ),
        shot(
            "face_to_path_deg",
            Some((0.0, -1.0, 1.0)),
            Some((0.5, 0.3, 0.7)),
        ),
    ])));
    assert_eq!(
        listed(&tied),
        vec![
            ("face_to_path_deg", FindingKind::Strength, 0.5),
            ("start_line_deg", FindingKind::Strength, 0.5),
        ]
    );
}

/// "Nothing established" is never a strength (ADR-034 §5.7): intervals that straddle the tolerance
/// list nothing, a metric withheld for want of `n` lists nothing, and a metric with no target cannot
/// be close to one however tight it is.
#[test]
fn nothing_established_is_never_a_strength() {
    let straddling = shot(
        "face_to_path_deg",
        Some((1.0, -0.5, 2.5)),
        Some((1.8, 1.2, 2.9)),
    );
    assert_eq!(
        straddling.pattern,
        Some(DispersionPattern::NothingEstablished)
    );
    let withheld = shot("start_line_deg", None, None);
    assert_eq!(
        (withheld.bias, withheld.scatter),
        (Finding::Withheld, Finding::Withheld)
    );
    // Carry has a tolerance and no target, so its mean is never tested and its spread, however
    // tight, is not closeness to anything.
    let no_target = shot("carry_distance_yds", None, Some((1.0, 0.5, 1.5)));
    assert_eq!(no_target.target, None);
    let list = rank_profile(&club(vec![straddling, withheld, no_target]));
    assert!(list.is_empty(), "{list:?}");
}

/// Mechanics never lists (Decision 12). Pose rows that establish a bias and a scatter as clearly as
/// any shot row are left off, by having no scope (a pose metric mislabelled with a launch-monitor
/// source included); and a flight projection, which has a scope, by its source and by having no
/// tolerance. The shot row beside them lists, so the filter is not simply refusing everything.
#[test]
fn no_pose_metric_and_no_projection_ever_lists() {
    let pose = |name: &str, source: &str| {
        let tolerance = target_for(name)
            .expect("a registered pose metric")
            .tolerance;
        decided(
            name,
            source,
            Some((10.0 * tolerance, 9.0 * tolerance, 11.0 * tolerance)),
            Some((5.0 * tolerance, 4.0 * tolerance, 6.0 * tolerance)),
        )
    };
    let established = pose("head_sway_norm", "pose:face_on");
    assert_eq!(
        established.pattern,
        Some(DispersionPattern::BiasedAndScattered)
    );
    let metrics = vec![
        established,
        pose("finish_balance_norm", "pose:face_on"),
        pose("tempo_ratio", "pose:face_on"),
        pose("hip_sway_norm", LM),
        decided(
            "flight_landing_offline_yds",
            "model:flight_v1",
            Some((20.0, 15.0, 25.0)),
            Some((30.0, 25.0, 40.0)),
        ),
        shot("carry_distance_yds", None, Some((15.0, 11.0, 20.0))),
    ];

    let list = checked(rank_profile(&club(metrics.clone())));
    assert_eq!(
        listed(&list),
        vec![("carry_distance_yds", FindingKind::Scatter, 3.0)]
    );
    assert!(checked(rank_player(&golfer(metrics))).is_empty());
}

/// A shot number's name read by another instrument does not list: the launch-monitor tolerances are
/// judgments about the OCR path, so a start line measured off video, or a pose metric that gained a
/// scope, would be ranked against an error term nobody derived for it. Every row here has a scope,
/// a target and a tolerance, and only the source tells them apart.
#[test]
fn only_a_launch_monitor_reading_lists() {
    let established = |source: &str| {
        decided(
            "start_line_deg",
            source,
            Some((5.0, 4.0, 6.0)),
            Some((3.0, 2.5, 4.0)),
        )
    };
    for source in ["pose:face_on", "model:flight_v1", "video:down_the_line", ""] {
        let metric = established(source);
        assert_eq!(metric.pattern, Some(DispersionPattern::BiasedAndScattered));
        assert!(
            rank_profile(&club(vec![metric.clone()])).is_empty(),
            "{source}"
        );
        assert!(rank_player(&golfer(vec![metric])).is_empty(), "{source}");
    }
    assert_eq!(
        listed(&checked(rank_player(&golfer(vec![established(LM)])))),
        vec![
            ("start_line_deg", FindingKind::Bias, 2.5),
            ("start_line_deg", FindingKind::Scatter, 1.5),
        ]
    );
}

/// The player's list reads club-independent metrics only: a carry or ball-speed spread pooled across
/// a bag is the bag. The same dispersion read as one club's lists all of them.
#[test]
fn the_players_list_is_club_independent() {
    let metrics = vec![
        shot("carry_distance_yds", None, Some((40.0, 35.0, 46.0))),
        shot("ball_speed_mph", None, Some((20.0, 17.0, 24.0))),
        shot(
            "start_line_offline_yds",
            Some((0.5, -1.0, 2.0)),
            Some((4.0, 3.0, 5.4)),
        ),
        shot(
            "face_to_path_deg",
            Some((5.0, 4.0, 6.0)),
            Some((1.0, 0.5, 1.5)),
        ),
        shot(
            "start_line_deg",
            Some((0.3, -0.3, 0.9)),
            Some((0.5, 0.3, 0.7)),
        ),
    ];
    let player = checked(rank_player(&golfer(metrics.clone())));
    assert!(player
        .iter()
        .all(|finding| is_club_independent(&finding.metric)));
    assert_eq!(
        listed(&player),
        vec![
            ("face_to_path_deg", FindingKind::Bias, 2.5),
            ("start_line_deg", FindingKind::Strength, 0.9 / 2.0),
        ]
    );

    let club_list = checked(rank_profile(&club(metrics)));
    let names: BTreeSet<&str> = club_list
        .iter()
        .map(|finding| finding.metric.as_str())
        .collect();
    assert_eq!(
        names,
        BTreeSet::from([
            "ball_speed_mph",
            "carry_distance_yds",
            "face_to_path_deg",
            "start_line_deg",
            "start_line_offline_yds",
        ])
    );
}

/// Nothing at all is an empty list, which `GradedClub` reads as "nothing established either way".
#[test]
fn an_empty_dispersion_lists_nothing() {
    assert!(rank_profile(&club(Vec::new())).is_empty());
    assert!(rank_player(&golfer(Vec::new())).is_empty());
}

/// ADR-008 as a cargo edge: `feedback` reaches `contracts` and `pyfmt` among this workspace's
/// crates and nothing else, so it can never read a grade or a pose number through `analysis`, nor
/// the disk through `storage`. Read off every member's manifest, so a crate added tomorrow is
/// covered without editing this test.
#[test]
fn feedback_depends_on_no_crate_of_ours_but_contracts_and_pyfmt() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates = here.parent().expect("crates/");
    let mut ours = BTreeSet::new();
    for entry in fs::read_dir(crates).expect("crates/ lists") {
        let manifest = entry.expect("an entry").path().join("Cargo.toml");
        if let Ok(text) = fs::read_to_string(&manifest) {
            ours.insert(package_name(&text).expect("a package name"));
        }
    }
    assert!(
        ours.contains("analysis") && ours.contains("storage") && ours.contains("golf-core"),
        "the walk found the workspace: {ours:?}"
    );

    let own = fs::read_to_string(here.join("Cargo.toml")).expect("feedback's manifest");
    let reached: BTreeSet<String> = dependency_names(&own)
        .into_iter()
        .filter(|name| ours.contains(name))
        .collect();
    assert_eq!(
        reached,
        BTreeSet::from(["contracts".to_string(), "pyfmt".to_string()])
    );
}

/// The `name` under `[package]`.
fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "name" {
                    return Some(value.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

/// Every key of every dependency table (`[dependencies]`, `[dev-dependencies]`,
/// `[build-dependencies]`, any `[target.….dependencies]`) and every `[dependencies.<name>]` header.
fn dependency_names(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_table = false;
    for line in manifest.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(header) = line.strip_prefix('[') {
            let header = header.trim_end_matches(']');
            in_table = header.ends_with("dependencies");
            if let Some((_, name)) = header.rsplit_once("dependencies.") {
                names.push(name.to_string());
            }
            continue;
        }
        if in_table {
            if let Some((key, _)) = line.split_once('=') {
                let key = key.trim();
                names.push(key.split('.').next().unwrap_or(key).to_string());
            }
        }
    }
    names
}
