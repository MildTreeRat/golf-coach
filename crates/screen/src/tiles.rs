//! A tile the screen printed and the shot holds no value for, named `printed_blank` or `misread`.
//! [M35 P3]
//!
//! **No Python twin.** Frozen Python has neither reason (`contracts/unscored.py` is closed, ADR-035
//! clause 4), so nothing in it asks this question. It lives here, not in `contracts` or `analysis`,
//! because telling the two apart needs the profile: a `ShotData` field has to be mapped to the label
//! its tile carries, and the cell's text checked against the device's blank markers, and only this
//! crate knows either. `analysis::engine::analyze_shot` takes the answer as its `tiles` argument and
//! copies it to the front of `ShotResult.unscored`. `crates/core` is the shell that passes it across,
//! so `analysis` never sees `screen` (ADR-008 as a cargo edge).
//!
//! # Which fields
//!
//! A field the shot's screen printed, that its device declares
//! [`Analysed`](contracts::capability::FieldUse::Analysed), holding no value: [`analysed_on`] less the
//! fields that hold one. That is ADR-034 §2's declared, printed and blank, applied in that order.
//!
//! - A field the layout does not print is never blank, and is named nowhere.
//! - A [`ShownOnly`](contracts::capability::FieldUse::ShownOnly) tile left blank is shown blank and
//!   refuses nothing, because nothing was going to grade it.
//! - A field holding a value is never refused, whatever its tile held. That is also why
//!   `corrections` is not read: a field a person corrected (M38) holds the corrected value.
//!
//! # The rule, from what the parse recorded under the tile's label
//!
//! | `provenance.raw_fields[label]` | reason |
//! |---|---|
//! | no key: located, then withheld by the tie rule | `misread` |
//! | `""`: nothing under the label | `printed_blank` |
//! | a match for the profile's `blank_markers` (`---`) | `printed_blank` |
//! | any other text, which gave no value | `misread` |
//!
//! - **No key is `misread`**, by the M34 plan's P8 finding 3. Every label the parser *read* has a
//!   `raw_fields` key, `""` included, so a field in `fields_present` without one was located and
//!   then withheld, because its label tied with another tile's. Something was printed there, and a
//!   photo whose labels read cleanly may recover it.
//! - **`""` is `printed_blank`** (the M35 plan's decision 3), though the parser's own warning for it
//!   is `no value text under the label`. On every bay photo the screen vectors hold, the empty cells
//!   are Spin, Spin Axis and Impact Position V, where the screen prints a `---` the OCR does not see.
//!   Telling the golfer to retake would be wrong on most shots, since a retake reads the same
//!   nothing.
//! - **Other text is `misread`**: the tile printed something and no value came of it. On the
//!   committed photos that is a number tile whose text holds no number (`2026-08-23-3`'s `Open`
//!   under Club Face Angle). Validation never drops a value it doubts, it flags the shot, so this is
//!   the parser's refusal and nothing else.
//!
//! # None, where a shot cannot say
//!
//! - **A direct feed** (no provenance) was never a photo, and both reasons are about one. A direct
//!   device's blanks wait for a direct device (M40).
//! - **An unstamped shot** (`fields_present: None`, every shot frozen Python stored) cannot tell a
//!   withheld tile from one its layout lacks: neither has a `raw_fields` key. Its printed set is only
//!   the fields holding a value (`printed_on`'s conservative reading), so it names no blank either,
//!   and none is named until M29 re-reads the stored photos through this crate. ADR-010 §2 puts
//!   under-reporting a blank before inventing one.
//!
//! The device is checked before either case. A shot naming a device `devices.json` does not
//! declare, or one no profile here reads, is refused rather than answered with nothing, as
//! `printed_on` refuses one.

use contracts::capability::{analysed_on, capability_for, fields_holding_a_value};
use contracts::shot::ShotData;
use contracts::unscored::{UnscoredCheckpoint, UnscoredReason};
use contracts::ContractError;

use crate::parser::is_blank;
use crate::profile::{load_profile, DeviceProfile, ProfileField};

/// Every printed, analysed tile this shot holds no value for, as an `unscored` entry, in the order
/// its device declares its fields (the profile's tile order, which `tests/capability.rs` pins).
///
/// `name` is the `ShotData` field (`spin_rate`), the vocabulary `devices.json` and `fields_present`
/// speak. `detail` names the tile's label and what the parse recorded under it, in the parser's
/// warning style (`Spin: nothing under the label`), so two blanks on one shot read apart.
///
/// Empty for a direct feed and for an unstamped shot (the module doc says why). Refuses a device
/// `devices.json` does not declare and one no profile reads. It also refuses a `fields_present`
/// naming a declared field the device's profile has no tile for, which no read writes: the parser
/// lists only its own profile's targets, and `tests/capability.rs` holds `hd_golf`'s targets equal
/// to its declaration.
pub fn tile_refusals(shot: &ShotData) -> Result<Vec<UnscoredCheckpoint>, ContractError> {
    let Some(provenance) = shot.provenance.as_ref() else {
        return Ok(Vec::new());
    };
    let capability = capability_for(&provenance.device)?;
    let profile = load_profile(&provenance.device).map_err(|refusal| ContractError {
        field: "ShotProvenance.device".to_string(),
        problem: refusal.to_string(),
    })?;
    if provenance.fields_present.is_none() {
        return Ok(Vec::new());
    }

    let analysed = analysed_on(shot)?;
    let held = fields_holding_a_value(shot);
    let mut refusals = Vec::new();
    for field in capability.declared() {
        if !analysed.contains(field) || held.contains(field) {
            continue;
        }
        let tile = tile_for(profile, field).ok_or_else(|| ContractError {
            field: "ShotProvenance.fields_present".to_string(),
            problem: format!(
                "{} names a tile the {} profile does not have",
                pyfmt::str_repr(field),
                profile.device
            ),
        })?;
        let raw = provenance.raw_fields.get(&tile.label).map(String::as_str);
        let (reason, detail) = judged(&tile.label, raw, &profile.blank_markers);
        refusals.push(UnscoredCheckpoint {
            name: field.to_string(),
            reason,
            detail,
        });
    }
    Ok(refusals)
}

/// The tile that fills `field`. Labels and targets are one-to-one in every shipped profile
/// (`parser`'s `only_the_impact_position_pair_overlaps` pins the labels distinct, and
/// `tests/capability.rs` the targets), which is what lets `raw_fields`, keyed by label, answer for
/// a field.
fn tile_for<'a>(profile: &'a DeviceProfile, field: &str) -> Option<&'a ProfileField> {
    profile
        .fields
        .iter()
        .find(|tile| tile.target.as_deref() == Some(field))
}

/// The module doc's table, one row per arm, and the `detail` each row writes.
///
/// `""` is tested before the markers, as `read_cell` tests it, so an empty cell reads as one even
/// on a profile whose markers include one that is all whitespace. Text is quoted with
/// [`pyfmt::str_repr`], as every parser warning quotes it, so `detail` and `provenance.warnings`
/// show the same text the same way.
fn judged(label: &str, raw: Option<&str>, blank_markers: &[String]) -> (UnscoredReason, String) {
    match raw {
        None => (
            UnscoredReason::Misread,
            format!("{label}: located, but its label tied with another tile's, so it was not read"),
        ),
        Some("") => (
            UnscoredReason::PrintedBlank,
            format!("{label}: nothing under the label"),
        ),
        Some(text) if is_blank(text, blank_markers) => (
            UnscoredReason::PrintedBlank,
            format!("{label}: {}, the screen's blank", pyfmt::str_repr(text)),
        ),
        Some(text) => (
            UnscoredReason::Misread,
            format!(
                "{label}: {} under the label, and no value could be read from it",
                pyfmt::str_repr(text)
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markers() -> Vec<String> {
        load_profile("hd_golf")
            .expect("the shipped profile")
            .blank_markers
            .clone()
    }

    /// Each row of the table, in its order, with the sentence it writes. `- - -` is a marker once
    /// the whitespace is out (`is_blank`), and `--` is one of `hd_golf`'s own.
    #[test]
    fn each_row_of_the_table_gives_its_reason() {
        let markers = markers();
        let cases = [
            (
                None,
                UnscoredReason::Misread,
                "Spin: located, but its label tied with another tile's, so it was not read",
            ),
            (
                Some(""),
                UnscoredReason::PrintedBlank,
                "Spin: nothing under the label",
            ),
            (
                Some("---"),
                UnscoredReason::PrintedBlank,
                "Spin: '---', the screen's blank",
            ),
            (
                Some("- - -"),
                UnscoredReason::PrintedBlank,
                "Spin: '- - -', the screen's blank",
            ),
            (
                Some("--"),
                UnscoredReason::PrintedBlank,
                "Spin: '--', the screen's blank",
            ),
            (
                Some("Open"),
                UnscoredReason::Misread,
                "Spin: 'Open' under the label, and no value could be read from it",
            ),
        ];
        for (raw, reason, detail) in cases {
            assert_eq!(
                judged("Spin", raw, &markers),
                (reason, detail.to_string()),
                "{raw:?}"
            );
        }
    }

    /// An empty cell is blank before the markers are asked, so a profile whose only marker is
    /// whitespace (which `is_blank` would also match on `""`) says the same thing for it.
    #[test]
    fn an_empty_cell_is_blank_whatever_the_markers() {
        for markers in [vec![], vec![" ".to_string()]] {
            assert_eq!(
                judged("Spin", Some(""), &markers).0,
                UnscoredReason::PrintedBlank
            );
        }
    }

    /// The label is how `raw_fields` is keyed, so each stored field has to find exactly its own
    /// tile; `Custom`, which fills nothing, is no field's.
    #[test]
    fn every_declared_field_has_its_tile() {
        let profile = load_profile("hd_golf").expect("the shipped profile");
        let capability = capability_for("hd_golf").expect("declared");
        for field in capability.declared() {
            let tile = tile_for(profile, field).unwrap_or_else(|| panic!("{field}"));
            assert_eq!(tile.target.as_deref(), Some(field));
        }
        assert!(tile_for(profile, "apex_height").is_none());
    }
}
