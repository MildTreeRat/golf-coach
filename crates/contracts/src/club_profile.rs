//! Everything one golfer's history says about one club — and where it refuses.
//! `contracts/club_profile.py`. [M36 P7]
//!
//! Career mode built a guard (the baseline will not report a mean until it has the samples for one),
//! a dispersion that separates a repeatable miss from a scattered one, and a corpus that counts
//! samples honestly; `CareerCorpus::narrowed_to` runs all of it over *one club's* swings. This is
//! the shape that answer comes back in.
//!
//! **A declaration, not a computation.** Nothing here averages, gates or judges — every statistic
//! arrives sealed by the step allowed to make it, and this module says which club it belongs to and
//! what evidence stands behind it. `analysis::club_profile` (M36 P12) builds these; the
//! `club-profile` verb renders them (M36 P16).
//!
//! # Two counters, because per club they diverge
//!
//! A 7 iron hit six times on video with two shot-screen photos is six swings of history and a
//! **carry ceiling of two** — every launch-monitor claim dedupes on the photo's hash. One counter
//! would undercount the history or overstate the distance evidence. Both are always populated,
//! never gated: they are facts about how much data exists, and what makes a refusal actionable.
//!
//! # A withheld statistic is absent, not zero
//!
//! [`crate::baseline`]'s rule, one level down: a `None` inside a [`MetricBaseline`] or a
//! [`MetricDispersion`] reached through a [`ClubProfile`] is a refusal itemised in that model's own
//! `withheld`, and a surface rendering it shows the reason rather than a blank cell.
//!
//! # Declared and hit are different questions
//!
//! `in_bag` and `n_swings > 0` stay apart, and [`BagProfile`] derives a list from each (ADR-024
//! §2): a club in the bag you have not hit has no statistics and is not an error, and a club you
//! have hit that has left the bag still has real history. The two need opposite responses — "go hit
//! it" and "this is history, not your current bag".

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bag::BagEntry;
use crate::baseline::MetricBaseline;
use crate::club::{category_of, ClubId};
use crate::dispersion::MetricDispersion;
use crate::intent::ClubCategory;
use crate::{each, nested, ContractError, Validate};

/// One club of one golfer's: the physical club, the evidence, and what it supports saying.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ClubProfile {
    pub club: ClubId,

    /// The physical club declared in this slot, or `None` when the golfer has not declared one —
    /// not a gap to fill from anywhere: it is what makes a loft or fitting question refuse for this
    /// club while its distance statistics stay readable. A declared loft is the **published** one
    /// (ADR-026 §1), so it describes the model rather than this club if the club has been bent.
    #[serde(default)]
    pub bag_entry: Option<BagEntry>,
    /// Is this club in the bag *now*. Not implied by `n_swings`, and not `bag_entry.is_some()`: a
    /// retired club has history and is not in the bag, a newly declared one is in it with none.
    #[serde(default)]
    pub in_bag: bool,

    // --- evidence, never gated --------------------------------------------------------------
    /// Distinct swings tagged with this club. Never swing directories: re-uploads were folded
    /// together upstream.
    #[serde(default)]
    pub n_swings: i64,
    /// Distinct shot photos among those swings, and so **the ceiling on every launch-monitor
    /// claim**. Lower than `n_swings` whenever a clip was filmed without the screen photographed.
    #[serde(default)]
    pub n_shots: i64,
    /// Distinct sessions those swings came from. The axis a TREND claim gates on.
    #[serde(default)]
    pub n_sessions: i64,

    /// Distinct shot photos held out of the **carry and total-distance averages only** as mishits
    /// (`contracts::mishit`, ADR-028). `n_shots` still counts them, and so does every other metric.
    #[serde(default)]
    pub mishits: i64,
    /// `session/swing` of every shot in `mishits`, sorted, so a held-out sample is something a
    /// golfer can go and look at.
    #[serde(default)]
    pub mishit_refs: Vec<String>,
    /// Of `mishits`, how many the automatic rule flagged and the golfer has not yet confirmed or
    /// cleared.
    #[serde(default)]
    pub mishits_unconfirmed: i64,

    // --- what the evidence supports ---------------------------------------------------------
    /// Metric name → this club's baseline, sorted by name. Refusals included.
    #[serde(default)]
    pub metrics: BTreeMap<String, MetricBaseline>,
    /// Metric name → this club's miss-shape, sorted by name. Refusals included.
    #[serde(default)]
    pub dispersion: BTreeMap<String, MetricDispersion>,

    /// Things true of this whole profile that qualify it, in the voice a golfer reads — the
    /// bag-changed and mishit sentences. A caveat names a date rather than withholding a
    /// statistic, because an entry recorded late for a club that never changed is the likelier case.
    #[serde(default)]
    pub caveats: Vec<String>,
}

crate::validated!(ClubProfile);

impl Validate for ClubProfile {
    fn validate(&self) -> Result<(), ContractError> {
        nested("ClubProfile.bag_entry", self.bag_entry.as_ref())?;
        for (name, metric) in &self.metrics {
            nested(&format!("ClubProfile.metrics[{name:?}]"), Some(metric))?;
        }
        for (name, metric) in &self.dispersion {
            nested(&format!("ClubProfile.dispersion[{name:?}]"), Some(metric))?;
        }
        Ok(())
    }
}

impl ClubProfile {
    /// The benchmark-row family this club belongs to, derived through [`category_of`] rather than
    /// stored, because `CLUB_CATEGORY` is *the* one mapping and a copy on every profile is a second
    /// home free to disagree with it. Not serialized, as Python's property is not.
    pub fn category(&self) -> ClubCategory {
        category_of(self.club)
    }
}

/// Every club one golfer has hit or declared, in the order they sit in the bag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct BagProfile {
    pub player_id: String,

    /// One profile per club hit or declared, in **canonical bag order** — [`ClubId`]'s declaration
    /// order, which [`Validate`] holds. A flat list and not a map keyed by club, because
    /// `ClubProfile.club` already names the slot and a second key could disagree with it.
    #[serde(default)]
    pub clubs: Vec<ClubProfile>,

    /// Swings this golfer hit that name no club, carried whole from
    /// `CareerCorpus::untagged_swings` and not renamed on the way through. The history no profile
    /// above can see.
    #[serde(default)]
    pub untagged_swings: i64,

    /// Shot photos across the whole bag held out of a club's carry and total-distance average as
    /// mishits, carried whole from `CareerCorpus::mishit_shots` (ADR-028).
    #[serde(default)]
    pub mishits_excluded: i64,
}

crate::validated!(BagProfile);

/// Each club's profile, then `_clubs_are_in_bag_order` with Python's messages.
///
/// `ClubId` declaration order is read, not decorative, and sorting anywhere downstream is the bug
/// the order exists to prevent: an alphabetised bag puts `3w` between `2h` and `5h`, which a golfer
/// cannot tell apart from a bug in their bag. Duplicates are refused in the same pass, because two
/// profiles for one club means [`BagProfile::profile_for`] silently drops one. A duplicate is
/// reported before an order fault on the same club, as Python checks it first.
impl Validate for BagProfile {
    fn validate(&self) -> Result<(), ContractError> {
        each("BagProfile.clubs", &self.clubs)?;
        let mut seen = [false; ClubId::ALL.len()];
        let mut previous = None;
        for profile in &self.clubs {
            let index = profile.club as usize;
            if seen[index] {
                return Err(refusal(format!("{} appears twice in clubs", profile.club)));
            }
            seen[index] = true;
            if previous.is_some_and(|previous| index < previous) {
                return Err(refusal(format!(
                    "clubs are not in bag order: {} came too late",
                    profile.club
                )));
            }
            previous = Some(index);
        }
        Ok(())
    }
}

fn refusal(problem: String) -> ContractError {
    ContractError {
        field: "BagProfile".to_string(),
        problem,
    }
}

impl BagProfile {
    /// Clubs with at least one swing behind them, whether or not they are still in the bag.
    pub fn clubs_used(&self) -> Vec<&ClubProfile> {
        self.clubs
            .iter()
            .filter(|profile| profile.n_swings > 0)
            .collect()
    }

    /// Clubs currently in the bag, whether or not anything has been hit with them.
    pub fn clubs_declared(&self) -> Vec<&ClubProfile> {
        self.clubs.iter().filter(|profile| profile.in_bag).collect()
    }

    /// This club's profile, or `None` if it has neither been hit nor declared — a real and common
    /// answer a caller renders, not an error. One home for the lookup, so `--club 7i` and the MCP
    /// tool cannot scan differently.
    pub fn profile_for(&self, club: ClubId) -> Option<&ClubProfile> {
        self.clubs.iter().find(|profile| profile.club == club)
    }
}

/// `tests/contracts/test_club_profile.py`, case for case.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::{BaselineClaim, Interval, WithheldClaim};
    use crate::club_spec::ClubSpec;
    use crate::dispersion::Finding;
    use serde_json::json;

    fn profile(club: ClubId) -> ClubProfile {
        ClubProfile {
            club,
            bag_entry: None,
            in_bag: false,
            n_swings: 0,
            n_shots: 0,
            n_sessions: 0,
            mishits: 0,
            mishit_refs: Vec::new(),
            mishits_unconfirmed: 0,
            metrics: BTreeMap::new(),
            dispersion: BTreeMap::new(),
            caveats: Vec::new(),
        }
    }

    fn bag(clubs: Vec<ClubProfile>) -> BagProfile {
        BagProfile {
            player_id: "aaron".into(),
            clubs,
            untagged_swings: 0,
            mishits_excluded: 0,
        }
    }

    fn round_trip<T: Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
        serde_json::from_str(&serde_json::to_string(value).unwrap()).unwrap()
    }

    fn clubs(of: &[&ClubProfile]) -> Vec<ClubId> {
        of.iter().map(|profile| profile.club).collect()
    }

    /// The composition is the phase, so the nesting is what has to survive: a baseline carrying a
    /// claim *and* a refusal, and a dispersion carrying a finding.
    #[test]
    fn a_populated_profile_round_trips_through_json() {
        let baseline: MetricBaseline = serde_json::from_value(json!({
            "name": "carry_distance_yds", "unit": "yards", "source": "launch_monitor:hd_golf",
            "n": 6, "n_sessions": 2, "mean": 164.2, "mean_ci": {"low": 158.0, "high": 170.4},
            "ready": ["center"],
        }))
        .unwrap();
        let baseline = MetricBaseline {
            withheld: vec![WithheldClaim {
                claim: BaselineClaim::Spread,
                have_n: 6,
                need_n: 10,
                have_sessions: 2,
                need_sessions: 1,
                reason: "6 shots on this club; a spread claim needs 10".into(),
            }],
            ..baseline
        };
        let dispersion: MetricDispersion = serde_json::from_value(json!({
            "name": "carry_distance_yds", "unit": "yards", "source": "launch_monitor:hd_golf",
            "n": 6, "n_sessions": 2, "tolerance": 5.0, "scatter": "not_established",
            "unavailable": ["no target for carry_distance_yds"],
        }))
        .unwrap();
        let mut spec = ClubSpec::blank(ClubId::SevenIron);
        spec.loft_deg = Some(34.0);
        let entry = BagEntry {
            spec,
            recorded_at: "2026-08-22T12:00:00Z".parse().unwrap(),
            retired_at: None,
            provenance: None,
        };
        let seven = ClubProfile {
            bag_entry: Some(entry),
            in_bag: true,
            n_swings: 6,
            n_shots: 6,
            n_sessions: 2,
            metrics: BTreeMap::from([("carry_distance_yds".into(), baseline)]),
            dispersion: BTreeMap::from([("carry_distance_yds".into(), dispersion)]),
            caveats: vec!["the bag entry was recorded after some of these shots".into()],
            ..profile(ClubId::SevenIron)
        };
        let original = BagProfile {
            untagged_swings: 2,
            ..bag(vec![seven])
        };

        let restored = round_trip(&original);
        assert_eq!(restored, original);
        let club = &restored.clubs[0];
        let carry = &club.metrics["carry_distance_yds"];
        assert_eq!(carry.mean, Some(164.2));
        assert_eq!(
            carry.mean_ci,
            Some(Interval {
                low: 158.0,
                high: 170.4,
                confidence: 0.95
            })
        );
        assert_eq!(carry.withheld[0].need_n, 10, "the refusal survived");
        assert_eq!(
            club.dispersion["carry_distance_yds"].scatter,
            Finding::NotEstablished
        );
        assert_eq!(club.bag_entry.as_ref().unwrap().spec.loft_deg, Some(34.0));
        assert_eq!(restored.untagged_swings, 2);
    }

    #[test]
    fn category_is_derived_for_every_club() {
        for club in ClubId::ALL {
            assert_eq!(profile(club).category(), category_of(club));
        }
        let written = serde_json::to_value(profile(ClubId::Driver)).unwrap();
        assert!(
            written.get("category").is_none(),
            "a derived value is not stored"
        );
    }

    /// The distinction ADR-024 §2 turns on, asserted from both lists so a club cannot hide.
    #[test]
    fn declared_and_hit_are_separate_lists_in_all_four_combinations() {
        let declared_and_hit = ClubProfile {
            in_bag: true,
            n_swings: 8,
            ..profile(ClubId::Driver)
        };
        let declared_unhit = ClubProfile {
            in_bag: true,
            ..profile(ClubId::ThreeWood)
        };
        let retired_but_hit = ClubProfile {
            n_swings: 5,
            ..profile(ClubId::SevenIron)
        };
        let neither = profile(ClubId::LobWedge);
        let bag = bag(vec![
            declared_and_hit,
            declared_unhit,
            retired_but_hit,
            neither,
        ]);
        assert_eq!(bag.validate(), Ok(()));

        assert_eq!(
            clubs(&bag.clubs_used()),
            [ClubId::Driver, ClubId::SevenIron]
        );
        assert_eq!(
            clubs(&bag.clubs_declared()),
            [ClubId::Driver, ClubId::ThreeWood]
        );
    }

    /// The carry ceiling. Six clips with two screen photos is two distance samples, not six.
    #[test]
    fn swings_and_shots_are_independent_counts() {
        let seven = ClubProfile {
            n_swings: 6,
            n_shots: 2,
            n_sessions: 2,
            ..profile(ClubId::SevenIron)
        };
        let restored = round_trip(&seven);
        assert_eq!((restored.n_swings, restored.n_shots), (6, 2));
    }

    #[test]
    fn the_mishit_fields_default_to_nothing_and_survive_a_round_trip() {
        let bare: ClubProfile = serde_json::from_value(json!({"club": "7i"})).unwrap();
        assert_eq!(bare, profile(ClubId::SevenIron));
        let empty: BagProfile = serde_json::from_value(json!({"player_id": "aaron"})).unwrap();
        assert_eq!(empty.mishits_excluded, 0);

        let populated = ClubProfile {
            mishits: 2,
            mishit_refs: vec!["2026-09-01/3".into(), "2026-09-02/1".into()],
            mishits_unconfirmed: 1,
            ..profile(ClubId::SevenIron)
        };
        let original = BagProfile {
            mishits_excluded: 2,
            ..bag(vec![populated])
        };
        let restored = round_trip(&original);
        assert_eq!(restored, original);
        assert_eq!(
            restored.clubs[0].mishit_refs,
            ["2026-09-01/3", "2026-09-02/1"]
        );
    }

    /// Inserted pw, 3w, driver, 7i; alphabetically 3w, 7i, driver, pw. Only bag order puts the
    /// driver first and the wedge last, the order a golfer looks down at.
    #[test]
    fn clubs_must_be_in_bag_order() {
        let inserted = [
            ClubId::PitchingWedge,
            ClubId::ThreeWood,
            ClubId::Driver,
            ClubId::SevenIron,
        ];
        let refuse = |order: &[ClubId]| {
            let given = json!({
                "player_id": "aaron",
                "clubs": order.iter().map(|club| json!({"club": club})).collect::<Vec<_>>(),
            });
            serde_json::from_value::<BagProfile>(given)
                .unwrap_err()
                .to_string()
        };
        assert!(
            refuse(&inserted)
                .starts_with("BagProfile: clubs are not in bag order: 3w came too late"),
            "{}",
            refuse(&inserted)
        );
        let mut alphabetical = inserted;
        alphabetical.sort_by_key(|club| club.as_str());
        assert!(refuse(&alphabetical).contains("clubs are not in bag order: driver came too late"));

        let ordered: Vec<ClubId> = ClubId::ALL
            .into_iter()
            .filter(|club| inserted.contains(club))
            .collect();
        assert_eq!(
            ordered,
            [
                ClubId::Driver,
                ClubId::ThreeWood,
                ClubId::SevenIron,
                ClubId::PitchingWedge
            ]
        );
        assert_eq!(
            bag(ordered.into_iter().map(profile).collect()).validate(),
            Ok(())
        );
    }

    /// The order is the declaration's, so the whole taxonomy passes unmodified.
    #[test]
    fn a_full_bag_in_declaration_order_is_accepted() {
        let full = bag(ClubId::ALL.into_iter().map(profile).collect());
        assert_eq!(full.validate(), Ok(()));
        assert_eq!(round_trip(&full), full);
    }

    #[test]
    fn a_club_may_not_appear_twice() {
        let twice = bag(vec![
            ClubProfile {
                n_swings: 5,
                ..profile(ClubId::SevenIron)
            },
            profile(ClubId::SevenIron),
        ]);
        assert_eq!(
            twice.validate().unwrap_err().to_string(),
            "BagProfile: 7i appears twice in clubs"
        );
    }

    #[test]
    fn profile_for_finds_a_club_or_returns_none() {
        let bag = bag(vec![
            ClubProfile {
                n_swings: 3,
                ..profile(ClubId::Driver)
            },
            ClubProfile {
                n_swings: 5,
                ..profile(ClubId::SevenIron)
            },
        ]);
        assert_eq!(
            bag.profile_for(ClubId::SevenIron).map(|p| p.n_swings),
            Some(5)
        );
        assert!(bag.profile_for(ClubId::SandWedge).is_none());
    }

    /// Every stored swing predates the tag, so nothing profiles — and the swings are still counted.
    #[test]
    fn an_empty_bag_profile_is_empty_rather_than_an_error() {
        let empty = BagProfile {
            untagged_swings: 2,
            ..bag(Vec::new())
        };
        assert_eq!(empty.validate(), Ok(()));
        assert!(empty.clubs_used().is_empty() && empty.clubs_declared().is_empty());
        assert!(empty.profile_for(ClubId::SevenIron).is_none());
        assert_eq!(empty.untagged_swings, 2);
    }

    /// A refusal deep in a club's dispersion, assembled in code, names its route from the root.
    #[test]
    fn a_nested_refusal_names_the_club_and_the_metric() {
        let mut bag: BagProfile = serde_json::from_value(json!({
            "player_id": "aaron",
            "clubs": [{"club": "7i", "dispersion": {"carry_distance_yds": {
                "name": "carry_distance_yds", "unit": "yards", "source": "launch_monitor:hd_golf",
                "n": 6, "n_sessions": 2, "center_ci": {"low": 1.0, "high": 2.0},
            }}}],
        }))
        .unwrap();
        let metric = bag.clubs[0]
            .dispersion
            .get_mut("carry_distance_yds")
            .unwrap();
        metric.center_ci.as_mut().unwrap().confidence = 1.0;
        assert_eq!(
            bag.validate().unwrap_err().field,
            "BagProfile.clubs[0] -> ClubProfile.dispersion[\"carry_distance_yds\"] -> \
             MetricDispersion.center_ci -> Interval.confidence"
        );
    }
}
