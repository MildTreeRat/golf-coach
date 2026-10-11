//! Practice-intent contracts. `contracts/intent.py`. [M22 P2]
//!
//! Per [ADR-009](../../../docs/decisions/009-swing-scoring-model.md): judging a swing is
//! meaningless without knowing what the golfer was *trying* to do. [`PracticeGoal`] carries that
//! intent into the engine, where `mode` selects a scoring policy.
//!
//! Like its Python counterpart this module imports nothing else from `contracts` — intent is an
//! input, so keeping it dependency-free means `swing` can hold a [`PracticeGoal`] without a cycle.
//!
//! `PlayerProfile` is not ported: it is `analysis/scoring.py`'s benchmark key, reachable from the
//! engine's code but not from any payload, so it lands with P5's benchmark loaders.

use serde::{Deserialize, Serialize};

use crate::{ContractError, Validate};

/// What the golfer is practicing — selects a scoring policy (ADR-009), and since ADR-034 whether a
/// shot is tracked at all.
///
/// **Every mode is scored by the one mechanics-only policy** (M35 P2). ADR-009 named a single-swing
/// policy per mode and only `Fundamentals`'s was built, "until full M4"; ADR-034 §5 retired the
/// wait, because a shot is graded per club over many shots (M35, M37), never one swing at a time.
/// So ADR-009's shot-shaping, performance and drill policies stay unbuilt,
/// `analysis::scoring::policy_for` answers every mode with the mechanics blend instead of refusing
/// the other three as it did until M35, and the per-swing `outcome_score` stays `None`. Frozen
/// Python still refuses them, and never asks. What each mode means now is on its variant.
///
/// **What a mode decides above the swing** is which aggregates a shot feeds, and
/// [`PracticeMode::is_tracked`] is that rule (the M35 plan's decision 1). Every mode is analysed
/// on its own, mechanics and shot alike; the variants below say what each feeds besides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PracticeMode {
    /// Grade mechanics only; outcome informational. **Tracked**: feeds club and player stats and
    /// the mechanics baseline, and declares no shape, so no shape topic grades it. A stored swing
    /// with no recorded intent reads as this mode, which is every swing frozen Python wrote.
    Fundamentals,
    /// Trying to shape the ball. Judged in M37's shape topics — over the shots declared as one
    /// [`TargetShape`], each classified by its face-to-path — rather than per swing (ADR-034 §5.3).
    /// **Tracked**: feeds club and player stats and the mechanics baseline like any tracked shot,
    /// and is the one mode a shape topic grades, for its declared `target_shape` alone.
    /// Challenge-mode shots are this mode.
    ShotShaping,
    /// **Tracked**, and graded per club over many shots like any tracked shot (ADR-034 §5): club
    /// and player stats and the mechanics baseline, and no shape topic. Its single-swing policy
    /// stays unbuilt, so its swings are scored as `Fundamentals`' are, and no shot metric is banded
    /// against tour benchmarks (§5.6).
    Performance,
    /// **Not tracked** (ADR-034 §3, M35): a drill shot is stored, shown and analysed on its own,
    /// mechanics included, and never enters club or player stats, the mechanics baseline or a shape
    /// topic. Tracked-ness is derived from this mode and never stored as a second flag, so the two
    /// cannot disagree.
    Drill,
}

impl PracticeMode {
    /// Whether a shot practised in this mode enters club and player stats, the mechanics baseline
    /// and, for [`PracticeMode::ShotShaping`], a shape topic (ADR-034 §3, the M35 plan's decision 1).
    /// Only a drill does not. [M35 P1]
    ///
    /// **Derived, never stored.** A manifest records the mode and nothing else, so no file can hold
    /// a drill that claims to be tracked. And it is a `match` with an arm per mode rather than
    /// `self != Drill`, so a fifth mode is a compile error here until someone decides whether it
    /// counts, instead of counting by default.
    pub fn is_tracked(self) -> bool {
        match self {
            PracticeMode::Fundamentals | PracticeMode::ShotShaping | PracticeMode::Performance => {
                true
            }
            PracticeMode::Drill => false,
        }
    }
}

/// Intended ball flight. With [`PracticeMode::ShotShaping`] it chooses which of M37's shape topics
/// a shot is graded under (ADR-034 §5.3); a shape never declared is not graded or mentioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetShape {
    Straight,
    Draw,
    Fade,
}

/// Club family used to key benchmark rows (ADR-010 §3).
///
/// `All` is the club-independent fallback — tempo and posture key on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClubCategory {
    Driver,
    Wood,
    Hybrid,
    LongIron,
    MidIron,
    ShortIron,
    Wedge,
    Putter,
    All,
}

impl ClubCategory {
    /// The wire name, for a sentence rather than for a serializer. [M22 P5]
    ///
    /// `mechanics.py`'s `NO_BAND` refusal interpolates `club.value` into a `detail` a golfer reads
    /// and `docs/CONFORMANCE.md` §3 compares exactly, which is the same need
    /// [`crate::swing::SwingPhase::as_str`] exists for — and it gets the same answer, for the
    /// reasons recorded there: a serializer round trip would allocate and quote, and a second
    /// hand-written table would drift. The wire-name test reads this rather than its own copy.
    pub fn as_str(self) -> &'static str {
        match self {
            ClubCategory::Driver => "driver",
            ClubCategory::Wood => "wood",
            ClubCategory::Hybrid => "hybrid",
            ClubCategory::LongIron => "long_iron",
            ClubCategory::MidIron => "mid_iron",
            ClubCategory::ShortIron => "short_iron",
            ClubCategory::Wedge => "wedge",
            ClubCategory::Putter => "putter",
            ClubCategory::All => "all",
        }
    }
}

/// The golfer's intent for one swing (ADR-009 §Concepts).
///
/// Selected at session level, overridable per shot. [`Default`] describes the PoC case:
/// Fundamentals mode, no declared shape, club-agnostic — the same defaults the pydantic model
/// carries, so a vector that omits `intent` and one that spells out the defaults agree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PracticeGoal {
    #[serde(default = "PracticeGoal::default_mode")]
    pub mode: PracticeMode,
    #[serde(default)]
    pub target_shape: Option<TargetShape>,
    #[serde(default = "PracticeGoal::default_club")]
    pub club: ClubCategory,
    #[serde(default)]
    pub focus_checkpoint: Option<String>,
}

impl PracticeGoal {
    fn default_mode() -> PracticeMode {
        PracticeMode::Fundamentals
    }

    fn default_club() -> ClubCategory {
        ClubCategory::All
    }
}

impl Default for PracticeGoal {
    fn default() -> Self {
        Self {
            mode: Self::default_mode(),
            target_shape: None,
            club: Self::default_club(),
            focus_checkpoint: None,
        }
    }
}

impl Validate for PracticeGoal {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// Who is swinging — keys skill-parameterized benchmark rows (ADR-010 §3). [M22 P5]
///
/// Deliberately thin, as the Python is: only `skill_level`, with height and physical limits
/// deferred to future mechanics ranges.
///
/// **No vector carries one, and it is ported anyway.** P2 left it out for exactly that reason —
/// its gate is a round trip, and a shape nothing on disk contains cannot be round-tripped. What
/// changed in P5 is that it acquired a caller: `benchmarks::store::resolve_range` takes one, and
/// the alternative was to widen that signature to a bare `&str` skill level and let the
/// `None` → `"all"` fallback live at the call site instead of inside the resolver where Python
/// keeps it. That is a different function, not a translated one — so the one-field struct comes
/// across and the resolver reads the same way in both languages. Every ported caller passes
/// `None` today; the `checkpoints` stage proves the `None` branch and nothing proves the other,
/// which is a hole this records rather than hides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerProfile {
    #[serde(default = "PlayerProfile::default_skill_level")]
    pub skill_level: String,
}

impl PlayerProfile {
    fn default_skill_level() -> String {
        "all".to_string()
    }
}

impl Default for PlayerProfile {
    fn default() -> Self {
        Self {
            skill_level: Self::default_skill_level(),
        }
    }
}

impl Validate for PlayerProfile {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One table, read two ways: the `as_str` word and the serde wire name must be the same
    /// string, because one reaches a golfer's sentence and the other reaches disk.
    #[test]
    fn the_club_word_is_the_wire_name() {
        let every = [
            ClubCategory::Driver,
            ClubCategory::Wood,
            ClubCategory::Hybrid,
            ClubCategory::LongIron,
            ClubCategory::MidIron,
            ClubCategory::ShortIron,
            ClubCategory::Wedge,
            ClubCategory::Putter,
            ClubCategory::All,
        ];
        for club in every {
            let wire = serde_json::to_string(&club).expect("a club serializes");
            assert_eq!(wire, format!("\"{}\"", club.as_str()));
        }
    }

    /// Every mode, from its wire name, with whether it is tracked. Hand-listed because Rust has no
    /// reflection over an enum, so the names are read through serde: a member renamed or removed
    /// fails to parse here. A member *added* is what [`PracticeMode::is_tracked`]'s exhaustive
    /// `match` catches, at compile time.
    const EVERY_MODE: [(&str, bool); 4] = [
        ("fundamentals", true),
        ("shot_shaping", true),
        ("performance", true),
        ("drill", false),
    ];

    /// Decision 1's table, as the one bit it comes down to: of every mode, only a drill stays out
    /// of the aggregates. [M35 P1]
    #[test]
    fn only_a_drill_is_untracked() {
        for (wire, tracked) in EVERY_MODE {
            let mode: PracticeMode =
                serde_json::from_str(&format!("\"{wire}\"")).expect("a practice mode's wire name");
            assert_eq!(mode.is_tracked(), tracked, "{wire}");
        }
        let untracked: Vec<&str> = EVERY_MODE
            .iter()
            .filter(|(_, tracked)| !tracked)
            .map(|(wire, _)| *wire)
            .collect();
        assert_eq!(untracked, ["drill"]);
    }

    /// The default goal is tracked, and that is load-bearing: every manifest frozen Python wrote
    /// has no intent, reads as the default, and must go on counting.
    #[test]
    fn the_default_goal_is_tracked() {
        assert!(PracticeGoal::default().mode.is_tracked());
        let parsed: PracticeGoal = serde_json::from_str("{}").expect("an empty goal parses");
        assert!(parsed.mode.is_tracked());
    }

    #[test]
    fn a_profile_defaults_to_the_club_independent_skill_level() {
        assert_eq!(PlayerProfile::default().skill_level, "all");
        let parsed: PlayerProfile = serde_json::from_str("{}").expect("an empty profile parses");
        assert_eq!(parsed, PlayerProfile::default());
    }
}
