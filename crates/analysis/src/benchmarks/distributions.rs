//! Reference distributions behind the benchmark bands. [M22 P5]
//!
//! The port of `benchmarks/distributions.py`. `ranges.json` says *whether* a swing is in band;
//! this says **where in the population it sits** — the same GolfDB-derived data the bands were cut
//! from, kept at full resolution rather than collapsed to two numbers.
//!
//! Deliberately **not** on [`super::store::resolve_range`]'s hot path: scoring reads `ranges.json`
//! and nothing here, which is what lets this file grow richer without touching how a swing is
//! scored (ADR-010 §2).

use std::sync::OnceLock;

use serde::Deserialize;

/// ADR-032 §5, as in [`super::store`]: embedded from the Python package path, one copy on disk.
const GOLFDB_JSON: &str =
    include_str!("../../../../src/golf_coach/analysis/benchmarks/golfdb_v1.json");

const ANY: &str = "all";

/// The observed spread of one metric over one stratum of the reference corpus.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Distribution {
    pub metric: String,
    pub club: String,
    pub sex: String,
    pub view: String,

    pub n: i64,
    pub n_players: i64,
    pub mean: f64,
    pub sd: f64,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,

    /// How *this row* was sampled, when that differs from the file's `dataset` block. Empty for
    /// the rows that block already describes, which is why it defaults rather than being required
    /// — fourteen of the shipped rows carry one.
    #[serde(default)]
    pub provenance: String,
}

impl Distribution {
    /// Roughly where `observed` falls in this population, as a percentile in `[0, 100]`.
    ///
    /// Interpolates between the stored quantiles and **clamps outside p10/p90** — the tails aren't
    /// stored, so a value past them is reported as "at least this extreme" rather than
    /// extrapolated into a precise-looking number the data can't support. `CheckpointScore`'s doc
    /// and the MCP server's instructions both restate that clamp, because a reader who takes a 90
    /// for a rank draws a wrong conclusion from a right number.
    ///
    /// **The bracket walk keeps Python's `zip(points, points[1:])` exactly**, including the case
    /// where two adjacent quantiles are equal: the first bracket that contains `observed` wins and
    /// a zero-width one answers with its *low* percentile. A port that searched for the last
    /// containing bracket, or that skipped the degenerate one, would disagree on a metric whose
    /// population is concentrated — which is a real shape here, not a hypothetical, since these
    /// are normalized travel distances with a floor at zero.
    pub fn percentile_of(&self, observed: f64) -> f64 {
        let points = [
            (10.0, self.p10),
            (25.0, self.p25),
            (50.0, self.p50),
            (75.0, self.p75),
            (90.0, self.p90),
        ];
        if observed <= self.p10 {
            return 10.0;
        }
        if observed >= self.p90 {
            return 90.0;
        }
        for window in points.windows(2) {
            let (low_pct, low_val) = window[0];
            let (high_pct, high_val) = window[1];
            if low_val <= observed && observed <= high_val {
                if high_val == low_val {
                    return low_pct;
                }
                let span = (observed - low_val) / (high_val - low_val);
                return low_pct + span * (high_pct - low_pct);
            }
        }
        // Unreachable for a monotone quantile set, and kept rather than made a panic because the
        // Python has it: a row whose quantiles are out of order would fall through here to 50
        // rather than crash a coaching call.
        50.0
    }
}

#[derive(Debug, Deserialize)]
struct DistributionsFile {
    distributions: Vec<Distribution>,
}

fn distributions() -> &'static [Distribution] {
    static ROWS: OnceLock<Vec<Distribution>> = OnceLock::new();
    ROWS.get_or_init(|| {
        let parsed: DistributionsFile = serde_json::from_str(GOLFDB_JSON)
            .expect("golfdb_v1.json ships in this crate and parses");
        parsed.distributions
    })
}

/// The distribution for `metric`, most-specific stratum first, or `None` if we have none.
///
/// Falls back one axis at a time — `(club, sex, view)`, then each single axis, then the overall
/// population — mirroring [`super::store::resolve_range`]'s specific-to-general walk. Returning
/// `None` rather than a guess keeps a missing stratum from being reported as a confident
/// percentile.
///
/// The corpus stores **marginals**, not the full cross-product, so a fully-specified query will
/// normally land on a single-axis fallback. That is intentional; see `derive_reference.py`.
///
/// Python gives all three axes a default of `"all"` and every ported caller takes all three, so
/// they are required parameters here rather than an options struct: `load_distribution(metric,
/// ANY, ANY, ANY)` reads as the same call and needs no builder to say so.
pub fn load_distribution(
    metric: &str,
    club: &str,
    sex: &str,
    view: &str,
) -> Option<&'static Distribution> {
    load_in(distributions(), metric, club, sex, view)
}

/// The walk itself, over a row set the caller supplies — split out for the reason
/// [`super::store::resolve_range`]'s twin is, and with a sharper version of the same problem.
///
/// `golfdb_v1.json` stores **marginals**, so the generic `("all", "all", "all")` row is matched by
/// the *second* candidate whenever `club` is `"all"` — which every ported caller passes. The fifth
/// candidate is therefore unreachable against the shipped file in every query the engine makes,
/// and a mutation that deleted it passed all 21 vectors and every test here. It is reachable when
/// the club is specific, which is what [`tests::the_last_candidate_is_reachable_for_a_named_club`]
/// asserts over rows it owns.
fn load_in<'a>(
    rows: &'a [Distribution],
    metric: &str,
    club: &str,
    sex: &str,
    view: &str,
) -> Option<&'a Distribution> {
    let candidates = [
        (club, sex, view),
        (club, ANY, ANY),
        (ANY, sex, ANY),
        (ANY, ANY, view),
        (ANY, ANY, ANY),
    ];
    let rows: Vec<&'a Distribution> = rows.iter().filter(|row| row.metric == metric).collect();
    for want in candidates {
        for row in &rows {
            if (row.club.as_str(), row.sex.as_str(), row.view.as_str()) == want {
                return Some(row);
            }
        }
    }
    None
}

/// `load_distribution(metric)` with Python's three defaults, which is how every ported caller
/// reaches it.
///
/// `mechanics.py::_population_placement` queries **the same stratum the band was cut from** —
/// all three axes `"all"` — and that consistency is load-bearing rather than lazy: `tempo_ratio`'s
/// face-on stratum has p90 5.00 against the all-view 4.71, so drawing the percentile from a
/// narrower stratum than the band would let a swing read "inside the band" and "past the 90th
/// percentile" at once.
pub fn load_distribution_any(metric: &str) -> Option<&'static Distribution> {
    load_distribution(metric, ANY, ANY, ANY)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempo() -> &'static Distribution {
        load_distribution_any("tempo_ratio").expect("tempo_ratio ships")
    }

    #[test]
    fn every_registered_checkpoint_metric_has_a_population() {
        for spec in contracts::checkpoints::CHECKPOINT_REGISTRY {
            assert!(
                load_distribution_any(spec.metric).is_some(),
                "no distribution for {}",
                spec.metric
            );
        }
    }

    #[test]
    fn an_unknown_metric_has_none() {
        assert!(load_distribution_any("spine_angle_norm").is_none());
    }

    #[test]
    fn the_tails_clamp_rather_than_extrapolate() {
        let row = tempo();
        assert_eq!(row.percentile_of(row.p10), 10.0);
        assert_eq!(row.percentile_of(row.p10 - 1000.0), 10.0);
        assert_eq!(row.percentile_of(row.p90), 90.0);
        assert_eq!(row.percentile_of(row.p90 + 1000.0), 90.0);
    }

    #[test]
    fn the_stored_quantiles_land_on_their_own_percentiles() {
        let row = tempo();
        assert_eq!(row.percentile_of(row.p25), 25.0);
        assert_eq!(row.percentile_of(row.p50), 50.0);
        assert_eq!(row.percentile_of(row.p75), 75.0);
    }

    #[test]
    fn a_midpoint_interpolates_linearly_within_its_bracket() {
        let row = tempo();
        let middle = (row.p25 + row.p50) / 2.0;
        let expected = 25.0 + 0.5 * (50.0 - 25.0);
        assert!(
            (row.percentile_of(middle) - expected).abs() < 1e-9,
            "{} vs {expected}",
            row.percentile_of(middle)
        );
    }

    /// **Both of `percentile_of`'s defensive arms are unreachable, and this searches for a way in
    /// rather than asserting it.**
    ///
    /// The mutation that made the degenerate bracket answer `high_pct` instead of `low_pct` passed
    /// all 21 vectors *and* every test here, which is the kind of survival worth chasing down
    /// rather than filing. The reason is structural: for bracket `(pI, pJ)` to be degenerate and
    /// selected, `observed` must equal both edges — and then the bracket *before* it, whose high
    /// edge is `pI`, already contains `observed` unless its low edge exceeds it, at which point the
    /// `p10` clamp has answered first. The same argument walks down to the `50.0` fall-through: no
    /// containing bracket means every bracket's low edge is above `observed`, and the first of
    /// those is `p10`.
    ///
    /// So this replicates the walk with the two arms replaced by panics and asserts the two agree
    /// across a grid of quantile arrangements — **including non-monotone ones**, which are the only
    /// shapes that could plausibly get in. Neither arm fires on any of them.
    ///
    /// They stay in the code anyway. Python has them, `docs/CONFORMANCE.md` §3 is about matching an
    /// implementation and not about improving one, and a quantile set is data on disk — the day a
    /// `derive_reference.py` bug ships a non-monotone row, the arms are the difference between a
    /// wrong percentile and a NaN in a coaching sentence.
    #[test]
    fn neither_defensive_arm_in_percentile_of_is_reachable() {
        /// `percentile_of` with the two defensive arms made fatal.
        fn strict(row: &Distribution, observed: f64) -> f64 {
            let points = [
                (10.0, row.p10),
                (25.0, row.p25),
                (50.0, row.p50),
                (75.0, row.p75),
                (90.0, row.p90),
            ];
            if observed <= row.p10 {
                return 10.0;
            }
            if observed >= row.p90 {
                return 90.0;
            }
            for window in points.windows(2) {
                let (low_pct, low_val) = window[0];
                let (high_pct, high_val) = window[1];
                if low_val <= observed && observed <= high_val {
                    assert!(high_val != low_val, "the degenerate arm was reached");
                    let span = (observed - low_val) / (high_val - low_val);
                    return low_pct + span * (high_pct - low_pct);
                }
            }
            panic!("the fall-through was reached");
        }

        let values = [0.5, 1.0, 2.0, 3.0, 4.0];
        let mut checked = 0;
        for &p10 in &values {
            for &p25 in &values {
                for &p50 in &values {
                    for &p75 in &values {
                        for &p90 in &values {
                            let row = Distribution {
                                p10,
                                p25,
                                p50,
                                p75,
                                p90,
                                ..tempo().clone()
                            };
                            for &observed in &values {
                                assert_eq!(strict(&row, observed), row.percentile_of(observed));
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(checked, values.len().pow(6));
    }

    /// The fifth candidate, over rows this test owns. Against `golfdb_v1.json` it is dead code for
    /// every query the engine makes — see [`load_in`] — so this is the only thing that holds it.
    #[test]
    fn the_last_candidate_is_reachable_for_a_named_club() {
        let generic = Distribution {
            club: ANY.to_string(),
            sex: ANY.to_string(),
            view: ANY.to_string(),
            ..tempo().clone()
        };
        let rows = vec![generic];
        let found = load_in(&rows, "tempo_ratio", "driver", "f", "face-on").expect("a fallback");
        assert_eq!(found.club, ANY);
    }

    /// And the precedence between the middle three, which the marginals cannot show either: a club
    /// row must beat a sex row, and a sex row a view row.
    #[test]
    fn the_single_axis_fallbacks_are_walked_club_then_sex_then_view() {
        let row = |club: &str, sex: &str, view: &str, n: i64| Distribution {
            club: club.to_string(),
            sex: sex.to_string(),
            view: view.to_string(),
            n,
            ..tempo().clone()
        };
        let rows = vec![
            row(ANY, ANY, "face-on", 3),
            row(ANY, "f", ANY, 2),
            row("driver", ANY, ANY, 1),
        ];
        assert_eq!(
            load_in(&rows, "tempo_ratio", "driver", "f", "face-on")
                .expect("a row")
                .n,
            1
        );
        assert_eq!(
            load_in(&rows, "tempo_ratio", "wedge", "f", "face-on")
                .expect("a row")
                .n,
            2
        );
        assert_eq!(
            load_in(&rows, "tempo_ratio", "wedge", "m", "face-on")
                .expect("a row")
                .n,
            3
        );
    }

    /// Python's walk takes the **first** containing bracket, so a value sitting exactly on an
    /// interior quantile is answered by the bracket below it and not the one above. The two
    /// disagree by 25 percentile points, which is the size of the error a reversed walk makes.
    #[test]
    fn an_interior_quantile_is_answered_by_the_bracket_below_it() {
        let row = tempo();
        assert_eq!(row.percentile_of(row.p50), 50.0);
        // And the neighbouring brackets are genuinely distinct, so the test above is not vacuous.
        assert!(row.p25 < row.p50 && row.p50 < row.p75);
    }

    /// The numbers `mechanics.py::_population_placement` names as its reason for querying the
    /// all-view stratum, asserted against the shipped file rather than quoted: `tempo_ratio`'s
    /// face-on p90 is 5.00 against the all-view 4.71, so a percentile drawn from the narrower
    /// stratum than the band would let a swing read "inside the band" and "past the 90th
    /// percentile" at once.
    ///
    /// The view key is `"face-on"` with a **hyphen**, which is not the `"face_on"` that
    /// `contracts/placements.py` uses for the same camera. Two vocabularies for one thing, and
    /// this is the one the corpus was derived under.
    #[test]
    fn the_view_stratum_differs_from_the_one_the_band_was_cut_from() {
        let any = load_distribution_any("tempo_ratio").expect("row");
        let face_on = load_distribution("tempo_ratio", ANY, ANY, "face-on").expect("face-on row");
        assert_eq!(any.p90, 4.7106);
        assert_eq!(face_on.p90, 5.0);
    }

    /// The fallback walk itself: an unknown stratum on any axis generalizes to the overall
    /// population rather than missing. Python's candidate list is what decides this, and a port
    /// that dropped an entry from it would return `None` here and lose every percentile.
    #[test]
    fn an_unknown_stratum_generalizes_to_the_overall_population() {
        let any = load_distribution_any("tempo_ratio").expect("row");
        for stratum in [
            load_distribution("tempo_ratio", "sand_wedge", ANY, ANY),
            load_distribution("tempo_ratio", ANY, "x", ANY),
            load_distribution("tempo_ratio", ANY, ANY, "overhead"),
        ] {
            assert_eq!(stratum.expect("a fallback row").p90, any.p90);
        }
    }

    #[test]
    fn a_row_that_carries_provenance_keeps_it() {
        let with_provenance = distributions()
            .iter()
            .filter(|row| !row.provenance.is_empty())
            .count();
        assert!(
            with_provenance > 0,
            "the `provenance` field parsed as empty everywhere"
        );
    }
}
