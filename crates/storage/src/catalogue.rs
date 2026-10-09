//! The club catalogue's key, and the catalogue read by it. `clubs/catalogue.py`'s `catalogue_key`,
//! `key_for`, `load_catalogue` and `lookup`. [M36 P8]
//!
//! `contracts::catalogue` reads the committed rows (M36 P5); this keys them. The split is the
//! dependency's, not the design's: the key folds a make and a model through [`slugify`], which needs
//! `unicode-normalization`, which `contracts` does not take (P5's finding 4). `remember`, the
//! writer, stays Python's — nothing in Rust confirms a club yet, and `clubs/lookup.py`, which does,
//! is an LLM call that stays Python (ADR-035 clause 1).
//!
//! Every refusal here is silent, as in Python, because this is a cache: a lookup that cannot find a
//! row costs an API call, never an error.

use std::sync::OnceLock;

use contracts::catalogue::{load_catalogue as committed_rows, CatalogueRow};
use contracts::club::ClubId;
use contracts::club_spec::ClubSpec;
use pyfmt::OrderedMap;

use crate::golfer_store::slugify;

/// The four identity fields to one key: `("Titleist", "T-150", 2023, 7i)` is
/// `titleist/t150/2023/7i`.
///
/// **Folded through `slugify`, not a second normaliser.** "T150", "T-150" and "t 150" splitting one
/// club into three rows is the failure "Aaron" and "aaron" splitting one golfer is, with the same
/// invisibility: the catalogue just misses, calls the model again and stores a row nobody sees
/// (ADR-026 §7). Separators are then dropped, which `slugify` must not do — a `player_id` is a
/// filename people read, while a model name is never read back out.
///
/// **An unknown year is its own key, not a wildcard**: an empty segment, matching only rows that also
/// failed to establish a year, because makers reuse a model name across generations with different
/// lofts. Blank components are not refused here; refusing is [`lookup`]'s policy.
pub fn catalogue_key(make: &str, model: &str, model_year: Option<i64>, club: ClubId) -> String {
    let year = model_year.map(|year| year.to_string()).unwrap_or_default();
    format!(
        "{}/{}/{year}/{}",
        slugify(make).replace('-', ""),
        slugify(model).replace('-', ""),
        club.as_str()
    )
}

/// [`catalogue_key`] off a spec's own identity fields, so no caller re-lists the four.
pub fn key_for(spec: &ClubSpec) -> String {
    catalogue_key(&spec.make, &spec.model, spec.model_year, spec.club)
}

/// Every committed row, keyed. A later row with an earlier row's key replaces it in the earlier
/// row's place, which is what Python's `rows[key] = row` does to a dict, and so is
/// [`OrderedMap::insert`].
///
/// Parsed once: the rows are compiled in (`contracts::catalogue`), so the file Python's
/// `load_catalogue` re-reads on every lookup cannot change under this process.
pub fn load_catalogue() -> &'static OrderedMap<&'static CatalogueRow> {
    static KEYED: OnceLock<OrderedMap<&'static CatalogueRow>> = OnceLock::new();
    KEYED.get_or_init(|| {
        let mut keyed = OrderedMap::new();
        for row in committed_rows() {
            keyed.insert(key_for(&row.spec), row);
        }
        keyed
    })
}

/// The remembered specification for one slot of one model, or `None` on a miss — the normal case
/// for the first lookup of any club. A make or model that slugs to nothing misses, because no row
/// may be keyed on emptiness.
///
/// The whole row, not the bare spec: the caller needs the provenance to say where the numbers came
/// from, and a spec with its provenance stripped is the "LLM's row mistaken for one typed off the
/// maker's page" ADR-026 §7 asks the catalogue to prevent.
pub fn lookup(
    make: &str,
    model: &str,
    model_year: Option<i64>,
    club: ClubId,
) -> Option<&'static CatalogueRow> {
    if slugify(make).is_empty() || slugify(model).is_empty() {
        return None;
    }
    load_catalogue()
        .get(&catalogue_key(make, model, model_year, club))
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `tests/clubs/test_catalogue.py`'s key cases.
    #[test]
    fn every_spelling_of_a_model_is_one_key() {
        for model in ["T150", "T-150", "t 150", "  t150  ", "T.150"] {
            assert_eq!(
                catalogue_key("Titleist", model, Some(2023), ClubId::SevenIron),
                "titleist/t150/2023/7i",
                "{model:?}"
            );
        }
        for make in ["TaylorMade", "Taylor Made", "taylor-made", "TAYLORMADE"] {
            assert_eq!(
                catalogue_key(make, "Stealth 2", None, ClubId::Driver),
                "taylormade/stealth2//driver",
                "{make:?}"
            );
        }
        let years: std::collections::BTreeSet<String> = [Some(2019), Some(2023), None]
            .into_iter()
            .map(|year| catalogue_key("TaylorMade", "P790", year, ClubId::SevenIron))
            .collect();
        assert_eq!(years.len(), 3, "a year is part of the identity");
        assert_ne!(
            catalogue_key("Titleist", "T150", Some(2023), ClubId::SevenIron),
            catalogue_key("Titleist", "T150", Some(2023), ClubId::EightIron)
        );
    }

    /// `test_the_shipped_catalogue_parses_and_every_row_in_it_validates`'s count, which says no two
    /// committed rows share a key; and every row is found by its own identity.
    #[test]
    fn every_committed_row_is_found_by_its_own_key() {
        let keyed = load_catalogue();
        assert_eq!(keyed.len(), committed_rows().len());
        for row in committed_rows() {
            let spec = &row.spec;
            let found = lookup(&spec.make, &spec.model, spec.model_year, spec.club)
                .unwrap_or_else(|| panic!("{} is not found", key_for(spec)));
            assert_eq!(found, row);
        }
    }

    #[test]
    fn a_lookup_with_no_make_or_model_misses() {
        assert!(lookup("", "T150", Some(2023), ClubId::SevenIron).is_none());
        assert!(lookup("Titleist", "  ", Some(2023), ClubId::SevenIron).is_none());
        assert!(lookup("Titleist", "T150", Some(2023), ClubId::Putter).is_none());
    }
}
