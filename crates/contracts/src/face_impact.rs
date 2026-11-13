//! Where on the face the ball was struck, once for every method that can say. [M46 P5]
//!
//! **One question, many methods, and each method declares its axes** (ADR-037 clause 1, the M46
//! plan's C2). M42's overhead replay (`crates/impact`'s `read_impact`) sees heel to toe and cannot
//! see high to low. M46's STRIKEPRO label (`impact::tape`) sees both. Before this module that
//! difference was one reader's habit: the replay simply never wrote `impact_offset_v`. Here it is a
//! rule. [`ImpactMethod::axes`] says what a method can see, and [`FaceImpact`]'s [`Validate`] refuses a
//! value on an axis its method cannot, so a reading that claims more than its method saw is refused
//! at the boundary rather than believed.
//!
//! **No Python twin, and none is coming**, as with [`crate::strike`]: the question was asked after
//! the pivot (ADR-034) and frozen Python never gains it (ADR-035 clause 4).
//!
//! # What does not change
//!
//! [`ShotData`](crate::shot::ShotData) keeps its `impact_offset_h` and `impact_offset_v`, and the
//! units here are theirs: millimetres from face centre, **+ toward the toe** and **+ high**. Nothing
//! converts a [`FaceImpact`] into a shot in M46 (the plan's rule 5, ADR-037 clause 6): a drill's
//! strikes are untracked (ADR-034 §3), and the replay's reader still fills `impact_offset_h` directly,
//! as it has since M42. The conversion, and the record of the method it would carry, are a later
//! milestone's (ADR-037's Consequences).
//!
//! # A method later is a new variant
//!
//! [`ImpactMethod::axes`] is a `match` with one arm per variant and no wildcard, so a third method
//! (foot spray, a launch monitor's printed offsets) does not compile until its axes are declared. That
//! is the reason it is a function and not a field a caller fills in.

use serde::{Deserialize, Serialize};

use crate::{ContractError, Validate};

/// The tape reader's version: which generation of `impact::tape::read_tape` made a reading.
/// [M46 P5]
///
/// Every stored drill strike carries it (the M46 plan's C8, `reader_version`), as a stored shot
/// carries [`SCREEN_PARSER_VERSION`](crate::shot::SCREEN_PARSER_VERSION). **Bump it whenever a
/// re-read of the same photo could produce a different reading**: a moved mark, a different
/// outcome (read, pick or not read), a new refusal, a changed crop or a changed label geometry.
/// Then a stored strike can say whether re-reading its crop would change it, which is what makes
/// re-reading the kept crops later cheap (Not in M46). The rule is the plan's rule 4: a phase that
/// changes what the reader produces **after P13** bumps it in the same change. Before P13 the reader
/// is being built, and no strike is stored before P18, so nothing it stamps can go stale.
///
/// A ledger of its own, beside [`crate::swing::ANALYSIS_VERSION`],
/// [`SCREEN_PARSER_VERSION`](crate::shot::SCREEN_PARSER_VERSION) and
/// [`CAREER_VERSION`](crate::career::CAREER_VERSION), because it ages a different record: a drill
/// strike's position, which none of the other three reads and which moves none of them.
///
/// 0 is not an entry, as it is not one on the screen's ledger: the ledger starts at the first
/// reader, so a strike can never name a reader that did not exist.
///
/// 1 (M46): the STRIKEPRO label reader as M46 ships it, gated on synthetic photos at P13, with
///   no real strike read before P29 (the plan's P3 finding 10).
pub const TAPE_READER_VERSION: i64 = 1;

/// How a reading of where the face met the ball was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactMethod {
    /// Full Swing's overhead impact replay, in a Live Photo's motion part: `crates/impact`'s
    /// `read_impact` (M42). From above, high to low cannot be seen.
    OverheadReplay,
    /// A STRIKEPRO impact label on the face, photographed after every strike of a drill:
    /// `impact::tape` (M46). The label is the frame, its printed centre the face centre (the plan's
    /// D2), and it shows both axes.
    StrikeTape,
}

impl ImpactMethod {
    /// The axes this method can see. A new variant fails to compile here until it says (the
    /// module doc).
    pub fn axes(self) -> ImpactAxes {
        match self {
            ImpactMethod::OverheadReplay => ImpactAxes {
                heel_toe: true,
                high_low: false,
            },
            ImpactMethod::StrikeTape => ImpactAxes {
                heel_toe: true,
                high_low: true,
            },
        }
    }

    /// The wire name, for a refusal's message: one spelling with the serialized form.
    fn as_str(self) -> &'static str {
        match self {
            ImpactMethod::OverheadReplay => "overhead_replay",
            ImpactMethod::StrikeTape => "strike_tape",
        }
    }
}

/// Which of the face's two axes a method can see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactAxes {
    /// Heel to toe: [`FaceImpact::offset_h_mm`], `ShotData.impact_offset_h`.
    pub heel_toe: bool,
    /// High to low: [`FaceImpact::offset_v_mm`], `ShotData.impact_offset_v`.
    pub high_low: bool,
}

/// One reading of where on the face the ball was struck, by one method.
///
/// **An axis the method did not read is `None`, never 0** (ADR-010 §2): 0 is face centre, a claim.
/// On the wire an absent value is an absent key, as in [`crate::strike`], and a `null` that arrives
/// still reads as absent.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct FaceImpact {
    /// Heel to toe, millimetres from face centre, + toward the toe: `ShotData.impact_offset_h`'s
    /// units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_h_mm: Option<f64>,
    /// High to low, millimetres from face centre, + high: `ShotData.impact_offset_v`'s units.
    /// Refused on a method whose [`ImpactAxes::high_low`] is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_v_mm: Option<f64>,
    pub method: ImpactMethod,
    /// The version of the reader that made this: [`TAPE_READER_VERSION`] for
    /// [`ImpactMethod::StrikeTape`].
    ///
    /// **For [`ImpactMethod::OverheadReplay`] it has nothing to hold.** M42's replay reader has no
    /// version key (ADR-034's M42 addendum), and inventing one here would be improving that reader,
    /// which M46 does not do (the plan's *Not in M46*). Nothing in M46 makes a `FaceImpact` from a
    /// replay, so the gap is left to the first milestone that does, which ADR-037 clause 1 names
    /// so that it finds it. Until then nothing checks this field against its method.
    pub method_version: i64,
}

crate::validated!(FaceImpact);

impl Validate for FaceImpact {
    fn validate(&self) -> Result<(), ContractError> {
        let axes = self.method.axes();
        axis(
            "FaceImpact.offset_h_mm",
            self.offset_h_mm,
            axes.heel_toe,
            self.method,
            "heel to toe",
        )?;
        axis(
            "FaceImpact.offset_v_mm",
            self.offset_v_mm,
            axes.high_low,
            self.method,
            "high to low",
        )
    }
}

/// One axis's value: absent, or seen by its method and a finite number of millimetres.
///
/// Finiteness is checked here rather than through a bound, because no bound applies: an offset is
/// signed and the label's size, not this shape, limits it. A NaN or an infinity is not a position
/// on a face, and `serde_json` cannot carry either, so one reaching this check was assembled in code.
fn axis(
    field: &str,
    value: Option<f64>,
    seen: bool,
    method: ImpactMethod,
    name: &str,
) -> Result<(), ContractError> {
    let Some(value) = value else {
        return Ok(());
    };
    if !seen {
        return Err(ContractError {
            field: field.to_string(),
            problem: format!(
                "{} cannot see {name}, so it holds no value there (got {value})",
                method.as_str()
            ),
        });
    }
    if !value.is_finite() {
        return Err(ContractError {
            field: field.to_string(),
            problem: format!("{value} is not a finite number of millimetres"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A refusal names the method by its wire name, so the two spellings must be one.
    #[test]
    fn a_refusal_spells_the_method_as_the_wire_does() {
        for method in [ImpactMethod::OverheadReplay, ImpactMethod::StrikeTape] {
            assert_eq!(
                serde_json::to_value(method).unwrap(),
                serde_json::Value::from(method.as_str())
            );
        }
    }
}
