//! A person's review of one shot: D9's forced stop, and C11 from the phone's side. [M38 P7]
//!
//! `golf_core::session::correct_shot` holds what an edit stores and why, so a lab surface could share
//! it, and this is the shape Dart sees and the card it gets back.

use golf_core::session::{ShotDirs, ShotEdit};

use crate::api::record::ShotCard;
use crate::card::card_of;
use crate::record::one_writer;
use crate::{engine_versions, parse_now, Dirs};

/// One row a person changed. **A row they did not touch is not sent**: `CardRow::printed` is six
/// significant digits, so it says what the screen printed and need not be the stored number's whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldEdit {
    /// `CardRow::field`.
    pub field: String,
    /// The number as typed, as the screen prints it, in the unit `CardRow::label` names: `85.2` under
    /// `Apex Ft`. Text, so Rust is the one place that says what a number is, and `8S.2` comes back as
    /// a refusal to show and not as a value cleared. `None` clears the value.
    pub printed: Option<String>,
    /// One of `CardRow::sign_letters`, on a tile that has them. Needed for any number but zero.
    pub letter: Option<String>,
}

/// Save a review: apply `edits`, clear the flag, re-analyse, and answer the card as it now stands.
/// `edits` empty is "reviewed, nothing to change", which is how a flagged shot with nothing wrong in it
/// is released (D9).
///
/// `Err` is a sentence to show beside the field: a refusal, with nothing written, or a write that
/// failed. `now` refuses a time with no offset, as every time crossing from Dart is.
pub fn correct_shot(
    root: String,
    session_id: String,
    swing_id: String,
    edits: Vec<FieldEdit>,
    now: String,
) -> Result<ShotCard, String> {
    let now = parse_now(&now)?;
    let _one_at_a_time = one_writer();
    let dirs = Dirs::of(&root);
    let edits: Vec<ShotEdit> = edits
        .into_iter()
        .map(|edit| ShotEdit {
            field: edit.field,
            printed: edit.printed,
            letter: edit.letter,
        })
        .collect();
    let shot = golf_core::session::correct_shot(
        &ShotDirs {
            sessions: dirs.sessions,
            golfers: dirs.golfers,
            shots: dirs.shots,
        },
        &session_id,
        &swing_id,
        &edits,
        engine_versions(),
        now,
    )?;
    card_of(&shot, &session_id, &swing_id)
}
