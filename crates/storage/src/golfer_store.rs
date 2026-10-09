//! The golfer registry — one JSON file per golfer, keyed by `player_id`. `storage/golfer_store.py`,
//! with `contracts/golfer.py::slugify`. [M36 P8]
//!
//! Flat files, as the shot store is: no shared index, no read-modify-write across golfers. At a
//! home lab's scale a directory listing *is* the index, and a bad record is fixed by deleting one
//! file.
//!
//! The load-bearing behaviour is [`GolferStore::get_or_create`], and it is deliberately
//! **read-biased**: a name that already resolves to a known golfer returns the stored record
//! untouched, handedness included. A career baseline is only as good as the join that assembles it,
//! and the two ways this could go wrong are both silent — a retyped name creating a second golfer
//! (one history split into two half-length ones), or a stray form submission flipping a stored
//! handedness (inverting the sign of every `head_hip_offset_impact_norm` comparison made against
//! that baseline). Refusing to write over an existing record makes both impossible rather than
//! unlikely.
//!
//! # `slugify` is here and not in `contracts`
//!
//! It needs Unicode's decomposition and combining-class tables, which `contracts` does not carry
//! (the M36 plan's call 6), and the golfer store is its first caller. The catalogue's key, which
//! folds a make and a model through it, is [`crate::catalogue`].

use std::fs;
use std::path::{Path, PathBuf};

use contracts::golfer::{Golfer, Handedness};
use contracts::{Timestamp, Validate};
use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;

use crate::StoreError;

/// `_SUFFIX`.
pub const SUFFIX: &str = ".golfer.json";

/// A display name to its stable `player_id`: `"Aaron "` → `"aaron"`.
///
/// **A collision is the feature.** Typing "Aaron" today and "aaron" next week has to land on the same
/// golfer, or a career baseline silently splits in two and each half reports a confident trend over
/// half the data. Accents are stripped rather than replaced for the same reason: without the NFD pass
/// "María" folds to `mar-a` and "Maria" to `maria`, one golfer who is inconsistent about the accent
/// key becomes two, and nothing downstream can detect it. The empty string means nothing survived,
/// and callers refuse it rather than invent a default.
///
/// Python's steps, each through the edge that reproduces it: `str.strip()` by `str.isspace`
/// ([`pyfmt::strip`]: NBSP, EM SPACE and the separators `\x1c`–`\x1f` go too); `str.lower()`
/// ([`pyfmt::lower`]: `İ` becomes `i` plus U+0307, and the Kelvin sign `k`); NFD; drop each character
/// `unicodedata.combining` calls combining, which is a **non-zero canonical combining class**, not
/// general category M — U+0903 and U+20DD are marks of class 0, so Python keeps them and the next step
/// makes `aःb` into `a-b` (P2's finding 4); then every run outside `[a-z0-9]` becomes one `-`, and `-`
/// is stripped from both ends.
///
/// **The Unicode versions differ, and that is a named divergence**: CPython 3.13's `unicodedata` is
/// 15.1, `unicode-normalization`'s tables are 17.0, and `pyfmt::lower` is std's. Measured at M36 P8
/// over every code point, alone and between two letters: the only differences are 46 combining marks
/// assigned after 15.1 (U+0897, U+1ACF–U+1AEB, and others in the supplementary planes), which have a
/// non-zero class here and none in Python, so `x` + mark + `y` is `xy` here and `x-y` there. No NFD
/// decomposition differs, and `lower`'s 27 newer mappings (M36 P4) all land outside `[a-z0-9]`, so
/// they fold to `-` in both. No name on disk or in a vector holds one of the 46.
pub fn slugify(name: &str) -> String {
    let folded = pyfmt::lower(pyfmt::strip(name));
    let mut slug = String::with_capacity(folded.len());
    let mut in_run = false;
    for ch in folded
        .nfd()
        .filter(|&ch| canonical_combining_class(ch) == 0)
    {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            slug.push(ch);
            in_run = false;
        } else if !in_run {
            slug.push('-');
            in_run = true;
        }
    }
    slug.trim_matches('-').to_string()
}

/// A directory of golfers, one JSON file each.
#[derive(Debug, Clone)]
pub struct GolferStore {
    root: PathBuf,
}

impl GolferStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        GolferStore { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/<player_id>.golfer.json`, with the id taken as given: [`Self::get`] reads whatever
    /// that names, as Python's does.
    pub fn path_for(&self, player_id: &str) -> PathBuf {
        self.root.join(format!("{player_id}{SUFFIX}"))
    }

    /// One golfer by id, or `None` if unknown.
    ///
    /// Tolerant, as the manifest is: an unreadable or older-schema record reads as absent. A corrupt
    /// golfer file should cost re-entering a name at the bay, not an error on the page someone is
    /// holding between swings. **It answers the record the file holds**, so `mismatch.golfer.json`
    /// holding golfer `aaron` answers `aaron` (the storage family's `golfer-reads`).
    pub fn get(&self, player_id: &str) -> Option<Golfer> {
        let bytes = fs::read(self.path_for(player_id)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Every known golfer, by display name. Unreadable records are skipped, not fatal.
    ///
    /// The files are `*.golfer.json` in sorted-name order, so a bag beside a golfer is never seen;
    /// then a **stable** sort on `display_name.lower()` by code point, which leaves names equal once
    /// lowered in the files' order and sorts `İnci` (lowered to `i` + U+0307) after `Inga`.
    pub fn list_all(&self) -> Vec<Golfer> {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut ids: Vec<String> = entries
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter_map(|name| name.strip_suffix(SUFFIX).map(str::to_string))
            .collect();
        ids.sort();
        let mut golfers: Vec<Golfer> = ids.iter().filter_map(|id| self.get(id)).collect();
        golfers.sort_by_key(|golfer| pyfmt::lower(&golfer.display_name));
        golfers
    }

    /// Resolve a typed name to a golfer, creating one only if its slug is genuinely new.
    ///
    /// `handedness` is used **only** when creating; for a known golfer the stored value wins (the
    /// module doc says why that direction is the safe one). The display name is `name.strip()`.
    /// A name that slugs to nothing is refused, with Python's message and the name as its `repr`,
    /// rather than given a placeholder id that would quietly collect every unnamed swing into one
    /// fictional golfer.
    ///
    /// A record that exists but cannot be read is not a known golfer, so it is written over (the
    /// storage family's `golfer-reads`), through a `.tmp` beside it and a rename.
    pub fn get_or_create(
        &self,
        name: &str,
        handedness: Handedness,
        now: Timestamp,
    ) -> Result<Golfer, StoreError> {
        let player_id = slugify(name);
        if player_id.is_empty() {
            return Err(StoreError::Refused(format!(
                "name {} has no usable characters for an id",
                pyfmt::str_repr(name)
            )));
        }
        if let Some(existing) = self.get(&player_id) {
            return Ok(existing);
        }

        let golfer = Golfer {
            player_id,
            display_name: pyfmt::strip(name).to_string(),
            handedness,
            created_at: now,
        };
        // A slug is always a valid id, so this cannot refuse; it is the constructor's check, kept so
        // that a change to either rule fails here rather than writing a record nothing can read.
        golfer.validate().map_err(StoreError::Invalid)?;
        fs::create_dir_all(&self.root)?;
        crate::write_atomically(&golfer, &self.path_for(&golfer.player_id))?;
        Ok(golfer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Timestamp {
        "2026-08-12T00:00:00Z".parse().unwrap()
    }

    /// `test_slugify_folds_names_to_a_stable_id`'s table, in its order.
    #[test]
    fn slugify_folds_names_to_a_stable_id() {
        let cases = [
            ("Aaron", "aaron"),
            ("  Aaron  ", "aaron"),
            ("AARON", "aaron"),
            ("Aaron Sierra", "aaron-sierra"),
            ("Aaron  Sierra", "aaron-sierra"),
            ("O'Brien", "o-brien"),
            ("Player 2", "player-2"),
            ("!!!", ""),
            ("", ""),
            ("Mar\u{ed}a", "maria"),
            ("Maria", "maria"),
            ("Bj\u{f6}rn", "bjorn"),
            ("\u{c9}amon", "eamon"),
        ];
        for (name, slug) in cases {
            assert_eq!(slugify(name), slug, "{name:?}");
        }
    }

    #[test]
    fn a_combining_mark_of_class_zero_is_kept_for_the_separator_step() {
        assert_eq!(slugify("a\u{903}b"), "a-b");
        assert_eq!(slugify("a\u{20dd}b"), "a-b");
        assert_eq!(slugify("a\u{301}b"), "ab");
    }

    #[test]
    fn get_or_create_creates_then_returns_the_same_golfer() {
        let dir = tempfile::tempdir().unwrap();
        let golfers = GolferStore::new(dir.path().join("golfers"));
        let created = golfers
            .get_or_create("Aaron", Handedness::Right, now())
            .unwrap();
        let again = golfers
            .get_or_create("  aaron ", Handedness::Left, now())
            .unwrap();
        assert_eq!(created.player_id, "aaron");
        assert_eq!(again, created, "read-biased: handedness and name kept");
        assert_eq!(golfers.list_all().len(), 1);
        assert!(!golfers.path_for("aaron").with_extension("tmp").exists());
    }

    #[test]
    fn accented_and_unaccented_spellings_are_the_same_golfer() {
        let dir = tempfile::tempdir().unwrap();
        let golfers = GolferStore::new(dir.path());
        let first = golfers
            .get_or_create("Mar\u{ed}a", Handedness::Right, now())
            .unwrap();
        let second = golfers
            .get_or_create("Maria", Handedness::Right, now())
            .unwrap();
        assert_eq!(second.player_id, first.player_id);
        assert_eq!(second.display_name, "Mar\u{ed}a");
    }

    #[test]
    fn a_nameless_golfer_is_refused_with_pythons_message() {
        let dir = tempfile::tempdir().unwrap();
        let golfers = GolferStore::new(dir.path().join("golfers"));
        for (name, message) in [
            ("!!!", "name '!!!' has no usable characters for an id"),
            (
                "\u{200b}",
                "name '\\u200b' has no usable characters for an id",
            ),
            ("'", "name \"'\" has no usable characters for an id"),
        ] {
            match golfers.get_or_create(name, Handedness::Right, now()) {
                Err(StoreError::Refused(refused)) => assert_eq!(refused, message),
                other => panic!("{name:?}: {other:?}"),
            }
        }
        assert!(golfers.list_all().is_empty());
        assert!(!golfers.root().exists(), "a refusal writes nothing");
    }

    #[test]
    fn unknown_and_corrupt_records_read_as_absent() {
        let dir = tempfile::tempdir().unwrap();
        let golfers = GolferStore::new(dir.path());
        assert_eq!(golfers.get("nobody"), None);
        golfers
            .get_or_create("Aaron", Handedness::Right, now())
            .unwrap();
        fs::write(golfers.path_for("aaron"), "{not json").unwrap();
        assert_eq!(golfers.get("aaron"), None);
        assert!(golfers.list_all().is_empty());
    }

    #[test]
    fn list_all_is_sorted_by_lowered_display_name() {
        let dir = tempfile::tempdir().unwrap();
        let golfers = GolferStore::new(dir.path());
        golfers
            .get_or_create("Zoe", Handedness::Left, now())
            .unwrap();
        golfers
            .get_or_create("aaron", Handedness::Right, now())
            .unwrap();
        let names: Vec<String> = golfers
            .list_all()
            .into_iter()
            .map(|g| g.display_name)
            .collect();
        assert_eq!(names, ["aaron", "Zoe"]);
    }
}
