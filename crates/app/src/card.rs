//! A stored shot as the card every read shows (D9). [M38 P6]
//!
//! Outside `api` because the codegen binds what is `pub` there (rule 7). `api::record` holds the shape
//! Dart sees, and this holds how a shot becomes it, for each surface that shows a card: a read, a
//! duplicate's held shot, and from P7 and P9 a review and a detail screen.
//!
//! **The card is built from the shot as stored, and from nothing the read left behind**, so a held
//! shot, a corrected one and a fresh one are one path. Dart formats nothing (rule 8), so each row brings
//! its text.
//!
//! # What a row says
//!
//! - **`None` is `NotRead`, with no display and never a 0** (ADR-010 §2). A person's correction outranks
//!   both, so a value someone cleared is `Corrected` and not `NotRead`: they have looked at it.
//! - **`value` is `ShotData`'s and `display` is the screen's.** They part twice, because a card is held
//!   against the photo beside it. Full Swing prints ball direction and sidespin unsigned and lights a
//!   letter, so a stored `-0.8` shows as `0.8 L`. It prints its apex in feet and `apex_height` holds
//!   yards, so a stored `28.4` shows as `85.2`, under a label that says `Apex Ft`.
//! - **A lit tile showing zero shows no letter**, since the screen needs none and the parser reads none
//!   (`parser::read_text`). [M38 P7]
//! - **`printed` and `letter` are `display` in two pieces**, so a review can fill its number field and
//!   its sign control with no splitting of a string in Dart (rule 8), and what they hold is what
//!   `correct_shot` takes back. [M38 P7]
//! - **`pyfmt::g` and no table of decimals.** The screen is not uniform about them (`95` beside `86.9`
//!   in Club Speed and `19°` beside `24.5°` in Launch Angle, on M33's captures), so a table would be a
//!   second opinion about what it printed, and could hide a digit a person typed. So `-2.0°` on the
//!   screen shows as `-2`, the same number. Six significant digits hold anything this screen prints,
//!   and it is the rule the validator's own warnings use.

use contracts::capability::{capability_for, FieldSource};
use contracts::shot::ShotData;
use screen::golfer_warnings;
use screen::profile::{load_profile, ProfileField};

use crate::api::record::{CardRow, RowState, ShotCard};
use crate::DEVICE;

/// One stored shot as `session_id/swing_id`'s card: one row per field the device declares `from:
/// screen`, in `devices.json`'s order, which is the screen's tile order.
///
/// `impact_offset_h` has no row, since it has no tile and no reader on the phone (D4).
///
/// A shot with no provenance is refused, since the card's flag and confidence would have nothing to
/// say and a 0 would be a false one. Every shot this phone stores has one (`screen::read` writes it).
pub(crate) fn card_of(
    shot: &ShotData,
    session_id: &str,
    swing_id: &str,
) -> Result<ShotCard, String> {
    let provenance = shot.provenance.as_ref().ok_or_else(|| {
        format!(
            "shot {} holds no provenance, so no screen read it",
            shot.shot_id
        )
    })?;
    let declared = capability_for(DEVICE).map_err(|refused| refused.to_string())?;
    let written = serde_json::to_value(shot).map_err(|failed| failed.to_string())?;

    let rows = declared
        .fields
        .iter()
        .filter(|field| field.from == FieldSource::Screen)
        .map(|declared| {
            let field = declared.field.as_str();
            let tile = tile_for(field)?;
            // A number crosses `serde_json::Value` with its bits, as `to_shot_data` relies on.
            let value = written.get(field).and_then(|held| held.as_f64());
            Ok(row_of(
                field,
                tile,
                value,
                provenance.corrections.contains_key(field),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(ShotCard {
        session_id: session_id.to_string(),
        swing_id: swing_id.to_string(),
        flagged: provenance.needs_review,
        confidence: provenance.parse_confidence,
        warnings: golfer_warnings(&provenance.warnings),
        rows,
    })
}

/// The tile holding a `ShotData` key on this phone's device. The profile is parsed once per process, so
/// asking again costs a search. [M38 P9]
pub(crate) fn tile_for(field: &str) -> Result<&'static ProfileField, String> {
    let profile = load_profile(DEVICE).map_err(|refused| refused.to_string())?;
    profile
        .fields
        .iter()
        .find(|tile| tile.target.as_deref() == Some(field))
        .ok_or_else(|| format!("{DEVICE}'s profile has no tile for {field}"))
}

/// One stored number as a card's row shows it, for a surface showing the number and not the row. [M38 P9]
pub(crate) fn display_of(value: f64, tile: &ProfileField) -> String {
    let (printed, letter) = printed_of(value, tile);
    joined(&printed, letter.as_deref())
}

fn joined(printed: &str, letter: Option<&str>) -> String {
    match letter {
        Some(letter) => format!("{printed} {letter}"),
        None => printed.to_string(),
    }
}

fn row_of(field: &str, tile: &ProfileField, value: Option<f64>, corrected: bool) -> CardRow {
    let state = match (corrected, value) {
        (true, _) => RowState::Corrected,
        (false, Some(_)) => RowState::Read,
        (false, None) => RowState::NotRead,
    };
    let (printed, letter) = value.map_or((None, None), |value| {
        let (printed, letter) = printed_of(value, tile);
        (Some(printed), letter)
    });
    CardRow {
        field: field.to_string(),
        label: tile.label.clone(),
        value,
        display: printed
            .as_deref()
            .map(|printed| joined(printed, letter.as_deref())),
        printed,
        letter,
        state,
        sign_letters: sign_letters(tile),
    }
}

/// A stored number as the screen printed it, and the letter lit beside it (the module doc).
fn printed_of(value: f64, tile: &ProfileField) -> (String, Option<String>) {
    // Back through `to_yards` rather than a `* 3.0`, so the factor keeps one home, in `crates/screen`.
    let printed = tile
        .printed_unit
        .map_or(value, |unit| value / unit.to_yards(1.0));
    if tile.lit_sign.is_empty() {
        return (pyfmt::g(printed), None);
    }
    let sign = if value.is_sign_negative() { -1 } else { 1 };
    let letter = tile
        .lit_sign
        .iter()
        .find(|lit| lit.sign == sign && value != 0.0)
        .map(|lit| lit.token.clone());
    (pyfmt::g(printed.abs()), letter)
}

/// The two letters of a tile signed by a lit one: the negative's first, then the positive's. `None` for
/// a tile printing its own sign. `DeviceProfile::check` holds a `lit_sign` to one letter of each sign.
fn sign_letters(tile: &ProfileField) -> Option<(String, String)> {
    let letter = |sign: i64| {
        tile.lit_sign
            .iter()
            .find(|letter| letter.sign == sign)
            .map(|letter| letter.token.clone())
    };
    Some((letter(-1)?, letter(1)?))
}
