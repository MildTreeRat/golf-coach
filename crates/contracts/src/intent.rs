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
/// **Only `Fundamentals` has a single-swing policy, and the other three stay without one.** That
/// was once "until full M4"; ADR-034 §5 retired the wait. A shot is graded per club over many shots
/// (M35, M37), never one swing at a time, so ADR-009's single-swing shot-shaping, performance and
/// drill policies stay unbuilt, `analysis::scoring::policy_for` keeps refusing them, and the
/// per-swing `outcome_score` stays `None`. What each mode means now is on its variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PracticeMode {
    /// Grade mechanics only; outcome informational.
    Fundamentals,
    /// Trying to shape the ball. Judged in M37's shape topics — over the shots declared as one
    /// [`TargetShape`], each classified by its face-to-path — rather than per swing (ADR-034 §5.3).
    /// Tracked, and challenge-mode shots are this mode.
    ShotShaping,
    /// Tracked, and graded per club over many shots like any tracked shot (ADR-034 §5). Its
    /// single-swing policy stays unbuilt, and no shot metric is banded against tour benchmarks
    /// (§5.6).
    Performance,
    /// **Not tracked** (ADR-034 §3, M35): a drill shot is stored, shown and analysed on its own, and
    /// never enters club or player stats. Tracked-ness is derived from this mode and never stored as
    /// a second flag, so the two cannot disagree.
    Drill,
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

    #[test]
    fn a_profile_defaults_to_the_club_independent_skill_level() {
        assert_eq!(PlayerProfile::default().skill_level, "all");
        let parsed: PlayerProfile = serde_json::from_str("{}").expect("an empty profile parses");
        assert_eq!(parsed, PlayerProfile::default());
    }
}
