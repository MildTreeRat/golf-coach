//! Benchmark range store — versioned data with provenance. [M22 P5]
//!
//! The port of `benchmarks/store.py`. Per
//! [ADR-010](../../../../docs/decisions/010-benchmark-ranges.md): the "correct" range for a
//! checkpoint is *data*, not a magic constant in code. Every row records its `source` so each
//! threshold is auditable and swappable, and rows are keyed by `(checkpoint, club, skill)` so they
//! can be parameterized later without touching the resolver.
//!
//! [`resolve_range`] falls back most-specific → least-specific so the store can stay sparse, and
//! returns `None` when nothing matches — a missing benchmark yields *no* score for that checkpoint
//! rather than a wrong one (ADR-010 §2).

use std::sync::OnceLock;

use contracts::intent::{ClubCategory, PlayerProfile};
use serde::Deserialize;

/// ADR-032 §5: one copy on disk, read from the Python package path at compile time.
const RANGES_JSON: &str =
    include_str!("../../../../src/golf_coach/analysis/benchmarks/ranges.json");

/// The skill level a row keyed to no particular one carries, and the fallback a profile-less
/// caller resolves at. `"all"` in both `ranges.json` and `golfdb_v1.json`.
const ANY: &str = "all";

/// One seeded benchmark row (internal — not a cross-module contract).
///
/// `source_date` and `added` are on disk and not here: no caller on the ported surface reads them,
/// and serde ignores unknown keys, so leaving them out costs nothing and states the reach
/// honestly. `source` stays because [`ResolvedRange`] carries it outward.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BenchmarkRange {
    checkpoint: String,
    club_category: ClubCategory,
    skill_level: String,
    low: f64,
    high: f64,
    source: String,
}

#[derive(Debug, Deserialize)]
struct RangesFile {
    ranges: Vec<BenchmarkRange>,
}

/// The resolved band for a checkpoint, carrying its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRange {
    pub low: f64,
    pub high: f64,
    /// Where this threshold came from. Nothing in the engine reads it — it is carried so that a
    /// band quoted anywhere can be traced back, which is the whole of ADR-010's argument.
    pub source: String,
}

/// Python's `@lru_cache(maxsize=1)` on `_load_ranges`, which here is a parse that happens once.
///
/// [`OnceLock`] rather than `lazy_static` or `once_cell`: it is std, and ADR-030's stdlib-only
/// invariant is about arithmetic rather than about this, but reaching for a crate when the
/// standard library has the exact type would still be a dependency nobody needs.
fn ranges() -> &'static [BenchmarkRange] {
    static RANGES: OnceLock<Vec<BenchmarkRange>> = OnceLock::new();
    RANGES.get_or_init(|| {
        let parsed: RangesFile =
            serde_json::from_str(RANGES_JSON).expect("ranges.json ships in this crate and parses");
        parsed.ranges
    })
}

/// Resolve the expected band for a checkpoint, most-specific → least-specific.
///
/// Fallback order (ADR-010 §2): the requested `(club, skill)` first, then progressively
/// generalized to `(club, all)`, `(all, skill)`, and finally `(all, all)`. Returns the first
/// match, or `None` if the store has no row for this checkpoint at any specificity — the caller
/// then produces no score for it (never a wrong one).
///
/// **The two loops are nested in this order and that is not arbitrary.** The outer walk is over
/// specificity and the inner over the rows, so a less specific row can never beat a more specific
/// one that appears later in the file. Swapping them would make the answer depend on row order in
/// `ranges.json`, which nothing promises.
pub fn resolve_range(
    checkpoint: &str,
    club: ClubCategory,
    profile: Option<&PlayerProfile>,
) -> Option<ResolvedRange> {
    resolve_in(ranges(), checkpoint, club, profile)
}

/// The walk itself, over a row set the caller supplies.
///
/// **Split out so the precedence is testable, and that is not a hypothetical need.** `ranges.json`
/// ships exactly one row per checkpoint, so against the real store every ordering of this walk
/// gives the same answer — a mutation that swapped the two loops passed the whole suite and all 21
/// vectors. The claim in [`resolve_range`]'s doc is about what happens when the store *stops*
/// being sparse, which is the direction ADR-010 §3 points it, and this is the seam that lets that
/// claim be asserted today rather than discovered the day a per-club row lands.
fn resolve_in(
    rows: &[BenchmarkRange],
    checkpoint: &str,
    club: ClubCategory,
    profile: Option<&PlayerProfile>,
) -> Option<ResolvedRange> {
    let skill = profile.map_or(ANY, |p| p.skill_level.as_str());
    let candidates = [
        (club, skill),
        (club, ANY),
        (ClubCategory::All, skill),
        (ClubCategory::All, ANY),
    ];
    let rows: Vec<&BenchmarkRange> = rows
        .iter()
        .filter(|row| row.checkpoint == checkpoint)
        .collect();
    for (want_club, want_skill) in candidates {
        for row in &rows {
            if row.club_category == want_club && row.skill_level == want_skill {
                return Some(ResolvedRange {
                    low: row.low,
                    high: row.high,
                    source: row.source.clone(),
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_file_parses_and_is_not_empty() {
        assert!(!ranges().is_empty());
    }

    #[test]
    fn every_registered_checkpoint_metric_resolves_a_band() {
        // The gate the `checkpoints` stage cannot give on its own: a metric with no row refuses
        // with `NO_BAND`, which is a *valid* answer, so a vector recorded against a store missing
        // a row would look like a correct refusal rather than like a missing band.
        for spec in contracts::checkpoints::CHECKPOINT_REGISTRY {
            assert!(
                resolve_range(spec.metric, ClubCategory::All, None).is_some(),
                "no band for {}",
                spec.metric
            );
        }
    }

    #[test]
    fn an_unknown_checkpoint_resolves_nothing() {
        assert_eq!(
            resolve_range("spine_angle_norm", ClubCategory::All, None),
            None
        );
    }

    #[test]
    fn a_club_with_no_row_falls_back_to_the_club_independent_one() {
        // Every row shipping today is `(all, all)`, so a per-club query must land on it rather
        // than miss. This is the fallback that keeps the store sparse.
        let specific = resolve_range("tempo_ratio", ClubCategory::Driver, None).expect("band");
        let general = resolve_range("tempo_ratio", ClubCategory::All, None).expect("band");
        assert_eq!(specific, general);
    }

    #[test]
    fn an_unknown_skill_level_falls_back_to_all() {
        let profile = PlayerProfile {
            skill_level: "scratch".to_string(),
        };
        assert_eq!(
            resolve_range("tempo_ratio", ClubCategory::All, Some(&profile)),
            resolve_range("tempo_ratio", ClubCategory::All, None),
        );
    }

    /// A store with four rows at four specificities, deliberately written **least specific
    /// first** so that a walk driven by row order rather than by the candidate list answers with
    /// the wrong one.
    fn a_sparse_store() -> Vec<BenchmarkRange> {
        let row = |club: ClubCategory, skill: &str, low: f64| BenchmarkRange {
            checkpoint: "tempo_ratio".to_string(),
            club_category: club,
            skill_level: skill.to_string(),
            low,
            high: low + 1.0,
            source: format!("{}/{skill}", club.as_str()),
        };
        vec![
            row(ClubCategory::All, ANY, 1.0),
            row(ClubCategory::All, "scratch", 2.0),
            row(ClubCategory::Driver, ANY, 3.0),
            row(ClubCategory::Driver, "scratch", 4.0),
        ]
    }

    /// The precedence the shipped store cannot exercise: most specific first, then club, then
    /// skill, then the club-and-skill-independent row. Each arm names a *different* row, so an
    /// ordering mistake is visible rather than absorbed.
    #[test]
    fn the_walk_is_most_specific_first_whatever_order_the_rows_are_in() {
        let store = a_sparse_store();
        let scratch = PlayerProfile {
            skill_level: "scratch".to_string(),
        };
        let cases = [
            (ClubCategory::Driver, Some(&scratch), "driver/scratch"),
            (ClubCategory::Driver, None, "driver/all"),
            (ClubCategory::MidIron, Some(&scratch), "all/scratch"),
            (ClubCategory::MidIron, None, "all/all"),
        ];
        for (club, profile, expected) in cases {
            let band = resolve_in(&store, "tempo_ratio", club, profile).expect("band");
            assert_eq!(band.source, expected, "club={club:?} profile={profile:?}");
        }
    }

    /// And the last candidate is reachable: a checkpoint whose only row is the generic one still
    /// resolves for a specific club and skill. Dropping `(All, ANY)` from the candidate list would
    /// fail here and nowhere else.
    #[test]
    fn the_generic_row_is_the_last_resort_and_is_reached() {
        let store = vec![BenchmarkRange {
            checkpoint: "tempo_ratio".to_string(),
            club_category: ClubCategory::All,
            skill_level: ANY.to_string(),
            low: 1.0,
            high: 2.0,
            source: "generic".to_string(),
        }];
        let scratch = PlayerProfile {
            skill_level: "scratch".to_string(),
        };
        let band =
            resolve_in(&store, "tempo_ratio", ClubCategory::Wedge, Some(&scratch)).expect("band");
        assert_eq!(band.source, "generic");
    }

    #[test]
    fn the_band_carries_its_provenance() {
        let band = resolve_range("tempo_ratio", ClubCategory::All, None).expect("band");
        assert!(band.source.contains("GolfDB"), "{}", band.source);
        assert!(band.low < band.high);
    }
}
