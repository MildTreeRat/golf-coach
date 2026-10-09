//! Golfer identity — who swung, and which way they face. `contracts/golfer.py`. [M22 P2, M36 P5]
//!
//! M22 ported only [`Handedness`], because `analyze_swing_bundle` takes a bare handedness and
//! nothing else from here (ADR-032 §8). M36 brings the rest of the identity the many-shot layer
//! needs: [`Golfer`], the record the golfer store keeps, and [`PLAYER_ID`], the slug rule that
//! record and [`crate::bag::Bag`] both hold an id to.
//!
//! **`slugify` is not here.** It folds a display name through NFD and
//! `unicodedata.combining`, which takes a Unicode normalization table this crate does not carry, so
//! it lands with the golfer store in `crates/storage` (the M36 plan's call 6).
//!
//! Why identity at all, condensed from the Python: every band in this repo describes a tour
//! population, and judging a swing against the golfer's own history is not expressible while
//! sessions are keyed by date. Handedness sits beside it because `head_hip_offset_impact_norm` is
//! signed in camera terms, and which side is which in swing terms is the golfer's handedness — one
//! question asked once, and asked later means asked about footage nobody can re-examine.

use serde::{Deserialize, Serialize};

use crate::{ContractError, Timestamp, Validate};

/// Which side the golfer swings from — the frame of reference for every signed metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Handedness {
    Right,
    Left,
}

impl Handedness {
    /// The wire name, for a sentence rather than for a serializer. [M22 P8b]
    ///
    /// `flight_infer::infer_spin_axis` formats it into the provenance string on a resolved spin axis
    /// — `"+2.5 deg printed, read right-handed"` — which `docs/CONFORMANCE.md` §3 compares exactly.
    /// The same need [`crate::intent::ClubCategory::as_str`] exists for, and the same answer: a
    /// serializer round trip would allocate and quote, and a second hand-written table would drift.
    pub fn as_str(self) -> &'static str {
        match self {
            Handedness::Right => "right",
            Handedness::Left => "left",
        }
    }
}

impl Validate for Handedness {
    fn validate(&self) -> Result<(), crate::ContractError> {
        Ok(())
    }
}

/// A `player_id` is lowercase ASCII alphanumerics and single hyphens — the pattern's source, as
/// Python's `PLAYER_ID.pattern` prints it into a refusal.
///
/// Narrow on purpose: the id becomes a filename in the golfer registry and the bag store, so
/// anything this admits is safe there without a second sanitising step. Rust carries no regex
/// engine, so [`is_player_id`] is the match, hand-written, and `tests` holds it to frozen Python's
/// answers.
pub const PLAYER_ID: &str = r"^[a-z0-9]+(?:-[a-z0-9]+)*$";

/// `PLAYER_ID.match(text)`, **including Python's `$`**, which matches before one trailing newline
/// as well as at the end: `"aaron\n"` is a valid id to frozen Python, and so it is here.
///
/// Ported rather than tightened, because a contract that refuses what the other language accepts
/// reads a file Python wrote as corrupt. Nothing writes such an id — the golfer store's ids come
/// out of `slugify`, which strips — and the bag store takes the id verbatim into a path, which is
/// where the quirk would surface, in both languages alike.
pub fn is_player_id(text: &str) -> bool {
    let body = text.strip_suffix('\n').unwrap_or(text);
    body.split('-').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    })
}

/// The slug check `Golfer` and `Bag` both apply, refusing with Python's own message —
/// `player_id 'Dave Smith' is not a slug (expected …)`, the id written as its `repr` — which the
/// storage vectors record verbatim. `model` names the contract for the error's `field`.
pub fn check_player_id(model: &str, value: &str) -> Result<(), ContractError> {
    if is_player_id(value) {
        return Ok(());
    }
    Err(ContractError {
        field: format!("{model}.player_id"),
        problem: format!(
            "player_id {} is not a slug (expected {PLAYER_ID})",
            pyfmt::str_repr(value)
        ),
    })
}

/// One person, tracked across sessions. The unit a career baseline is cut per.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct Golfer {
    /// Stable id, the slug of the name first used. Never changes once created.
    pub player_id: String,
    /// The name as originally typed, for showing back.
    pub display_name: String,
    pub handedness: Handedness,
    pub created_at: Timestamp,
}

crate::validated!(Golfer);

impl Validate for Golfer {
    fn validate(&self) -> Result<(), ContractError> {
        check_player_id("Golfer", &self.player_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// `bool(PLAYER_ID.match(text))` as frozen Python answered each, measured at M36 P5.
    #[test]
    fn the_slug_rule_answers_as_pythons_regex_does() {
        let cases = [
            ("aaron", true),
            ("aaron\n", true),
            ("aaron\n\n", false),
            ("\naaron", false),
            ("a-b", true),
            ("a--b", false),
            ("-a", false),
            ("a-", false),
            ("", false),
            ("A", false),
            ("\u{e9}", false),
            ("a b", false),
            ("a\r", false),
            ("a\t", false),
            ("0", true),
            ("a-0-b", true),
            ("\u{ff41}", false),
            ("aaron\n-x", false),
            ("a\u{660}", false),
        ];
        for (text, python) in cases {
            assert_eq!(is_player_id(text), python, "{text:?}");
        }
    }

    #[test]
    fn a_non_slug_is_refused_with_pythons_message() {
        let err = check_player_id("Bag", "../aaron").unwrap_err();
        assert_eq!(
            err.problem,
            "player_id '../aaron' is not a slug (expected ^[a-z0-9]+(?:-[a-z0-9]+)*$)"
        );
        let golfer = json!({
            "player_id": "Bad Id",
            "display_name": "Bad",
            "handedness": "right",
            "created_at": "2026-08-12T09:00:00Z",
        });
        let message = serde_json::from_value::<Golfer>(golfer)
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("player_id 'Bad Id' is not a slug"),
            "{message}"
        );
    }

    #[test]
    fn a_golfer_round_trips() {
        let golfer = json!({
            "player_id": "aaron",
            "display_name": "Aaron",
            "handedness": "left",
            "created_at": "2026-08-06T12:00:00.120000Z",
        });
        let parsed: Golfer = serde_json::from_value(golfer.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), golfer);
    }
}
