//! The committed club catalogue, read. `clubs/catalogue.py`'s reader. [M36 P5]
//!
//! A specification is stable for the life of a model, so asking a model for one twice buys the
//! same answer again — slowly, at a cost, and possibly *different*. The catalogue makes the answer
//! a fact of this repo: committed JSON with provenance per row, ADR-022's division applied to a
//! lookup, and [ADR-026](../../../docs/decisions/026-club-specification-lookup.md) §7's decision.
//! Rows arrive when a golfer **confirms** a specification, so every row is a club somebody owns and
//! the long tail stays *missing* rather than wrong.
//!
//! # A two-language file: Python writes it, Rust reads it
//!
//! `clubs/lookup.py` is an LLM call and stays Python (ADR-035 clause 1), so frozen Python's
//! `remember` goes on writing `src/golf_coach/clubs/club_catalogue.json`, and this reads it by
//! `include_str!` — one copy on disk (ADR-032 §5), as the benchmarks cross. The consequence to know:
//! **a row Python remembers reaches Rust at the next build**, not the next lookup. That is clause 4
//! reversed, so `tests/catalogue.rs` pins the direction that matters — every committed row parses,
//! and every value `contracts/club_spec.py` can write (each material and flex, every optional key
//! absent, since the file is written with `exclude_defaults=True`) reads back.
//!
//! # What is not here, and where it goes
//!
//! **The key.** `catalogue_key` folds make and model through `slugify`, which needs a Unicode
//! normalization table this crate does not take (the M36 plan's call 6), so `catalogue_key`,
//! `key_for` and the keyed `lookup` land beside `slugify` in `crates/storage`. Python's
//! `load_catalogue` answers a dict keyed by it, where a later row with an earlier row's key wins;
//! [`read_rows`] answers the rows in file order, and the keying (with its last-wins) is the keyed
//! reader's to apply. `remember`, the writer, stays Python's: nothing in Rust confirms a club yet.
//!
//! # Every refusal is silent, because this is a cache
//!
//! The bag store raises when it cannot read a bag, because the bag is the record. A lookup that
//! cannot read the catalogue must cost an API call, never an error, so an unreadable file is *no
//! catalogue*, and a row that fails validation is skipped while the rest survive — one hand-edited
//! row with a typo is not a reason to stop serving forty checked ones.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::club_spec::{ClubSpec, SpecProvenance};
use crate::{nested, ContractError, Validate};

/// ADR-032 §5: one copy on disk, the package data frozen Python writes, read at compile time.
const CATALOGUE_JSON: &str = include_str!("../../../src/golf_coach/clubs/club_catalogue.json");

/// Bumped when the row shape changes incompatibly. A file declaring anything else is read as *no*
/// catalogue: a newer file misread under this reader would hand a golfer a specification assembled
/// out of fields that moved.
pub const SCHEMA_VERSION: i64 = 1;

/// One remembered specification and where it came from.
///
/// Provenance is **required** here where `BagEntry.provenance` is optional: a row only exists
/// because something produced it, and a row that cannot say what makes ADR-026 §7's claim — an LLM's
/// row is never mistaken for one typed off the maker's page — unverifiable. It travels with the spec
/// rather than being rewritten to `"catalogue"`: the catalogue is where an answer was kept, not a
/// source. The key is **not** a field; it is derived from the spec, so there is one place a row's
/// identity is written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct CatalogueRow {
    pub spec: ClubSpec,
    pub provenance: SpecProvenance,
}

crate::validated!(CatalogueRow);

impl Validate for CatalogueRow {
    fn validate(&self) -> Result<(), ContractError> {
        nested("CatalogueRow.spec", Some(&self.spec))?;
        nested("CatalogueRow.provenance", Some(&self.provenance))
    }
}

/// `_read_rows`: every row of a catalogue's text, in file order, or `None` for a file that cannot
/// be trusted — not JSON, not an object, a `schema_version` other than [`SCHEMA_VERSION`], or
/// `clubs` not a list. A row that does not validate is skipped and the rest survive.
///
/// The version test is Python's `payload.get("schema_version") != 1`, ported as it compares:
/// `1.0` and `true` both equal `1` in Python, so both pass here. Nothing writes either.
///
/// Each row is read the way `CatalogueRow.model_validate` reads a parsed dict, less pydantic's lax
/// coercions (a numeric string for a number, and the rest of the M36 plan's call 9), which refuse
/// the row here where Python would keep it. `remember` writes none of them.
pub fn read_rows(text: &str) -> Option<Vec<CatalogueRow>> {
    let payload: Value = serde_json::from_str(text).ok()?;
    let payload = payload.as_object()?;
    if !equals_one(payload.get("schema_version")) {
        return None;
    }
    let raw = payload.get("clubs")?.as_array()?;
    Some(
        raw.iter()
            .filter_map(|entry| serde_json::from_value(entry.clone()).ok())
            .collect(),
    )
}

/// Python's `value == 1` on a parsed JSON value: an integer or float equal to one, or `True`.
fn equals_one(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => number.as_f64() == Some(1.0),
        _ => false,
    }
}

/// `load_catalogue`: every committed row, parsed once, in file order. An unreadable or
/// unknown-version file is an empty catalogue, as Python's is; `tests/catalogue.rs` holds the
/// committed one to parsing whole, so empty here would be a failed test before it was a quiet miss.
pub fn load_catalogue() -> &'static [CatalogueRow] {
    static ROWS: OnceLock<Vec<CatalogueRow>> = OnceLock::new();
    ROWS.get_or_init(|| read_rows(CATALOGUE_JSON).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(club: &str) -> Value {
        json!({
            "spec": {"club": club, "make": "Titleist", "model": "T150"},
            "provenance": {"source": "typed", "retrieved_at": "2026-09-01T13:51:39.545201Z"},
        })
    }

    #[test]
    fn an_untrusted_file_is_no_catalogue() {
        for text in [
            "{not json",
            "[]",
            r#"{"clubs": []}"#,
            r#"{"schema_version": 2, "clubs": []}"#,
            r#"{"schema_version": "1", "clubs": []}"#,
            r#"{"schema_version": 1, "clubs": {}}"#,
            r#"{"schema_version": 1}"#,
        ] {
            assert_eq!(read_rows(text), None, "{text}");
        }
    }

    #[test]
    fn a_version_python_reads_as_one_is_one() {
        for version in [json!(1), json!(1.0), json!(true)] {
            let text = json!({"schema_version": version, "clubs": []}).to_string();
            assert_eq!(read_rows(&text), Some(vec![]), "{version}");
        }
    }

    #[test]
    fn a_bad_row_is_skipped_and_the_rest_survive() {
        let text = json!({
            "schema_version": 1,
            "clubs": [
                row("7i"),
                {"spec": {"club": "7i"}},
                {"spec": {"club": "10i"}, "provenance": row("7i")["provenance"]},
                "not a row",
                row("8i"),
            ],
        })
        .to_string();
        let rows = read_rows(&text).unwrap();
        let clubs: Vec<&str> = rows.iter().map(|r| r.spec.club.as_str()).collect();
        assert_eq!(clubs, ["7i", "8i"]);
    }
}
