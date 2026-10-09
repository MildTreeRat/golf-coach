//! The club vocabulary — which club hit a shot, as a closed set. `contracts/club.py`. [M36 P5]
//!
//! [ADR-024](../../../docs/decisions/024-per-club-shot-history.md) is the why. Three of the Python
//! module's reasons travel with it, condensed:
//!
//! - **A closed enum, not free text.** "7i", "7 iron" and "seven" splitting one club's carry
//!   average three ways is `slugify`'s failure (one golfer split in two) moved to clubs, and just
//!   as invisible. So: a closed set inside, one tolerant parser at the boundary, and `None` rather
//!   than a guess when text does not resolve.
//! - **A specific club, not [`ClubCategory`].** The category is the right grain for *bands* and the
//!   wrong one for *history*: "mid iron" pools a 6 and an 8, and "my 7 iron" is the question. The
//!   category is therefore derived from the club ([`category_of`]), one table rather than two
//!   vocabularies free to disagree.
//! - **`pw`/`gw`/`sw`/`lw`, not `52`/`56`/`60`.** A loft inside an identifier renames the club at
//!   the first re-grind and orphans every shot it hit. `7i` is a **slot**; the loft belongs to the
//!   physical club in it, which is a [`crate::bag::BagEntry`] (ADR-024 §2).
//!
//! And one thing it must not be wired into: `PracticeGoal.club` stays [`ClubCategory::All`].
//! `ranges.json` holds club-agnostic rows only, so handing the resolver a real category makes every
//! checkpoint resolve no band and the fundamentals panel goes dark.
//!
//! # `isalnum` is this module's one edge, and it is a named divergence
//!
//! `_normalize` keeps `ch for ch in text.lower() if ch.isalnum()`. Rust has no `str.isalnum()`, and
//! [`char::is_alphanumeric`] is a different set: measured over every code point (M36 P4 finding 7)
//! it holds nothing CPython's lacks and **5,877 code points CPython's does not** — the
//! `Other_Alphabetic` marks (U+0345, the combining Latin letters U+0363–U+036F, Hebrew points from
//! U+05B0, Indic vowel signs) and Unicode 16.0's additions. Reproducing CPython exactly needs a
//! general-category table, which `pyfmt` does not carry and std does not expose.
//!
//! It is not needed, because every alias is ASCII. A key can only match when everything kept is an
//! ASCII letter or digit, so the only question a non-ASCII character asks is *kept or dropped*,
//! and the two sets disagree only on characters Rust keeps and CPython drops. Such a character
//! therefore makes Rust **refuse** text CPython would accept — `7i` followed by U+0345 is a 7 iron
//! in Python and `None` here — and never the reverse. That is the direction ADR-024 §5's asymmetry
//! asks for: a refused tag costs one retype at the bay, and a wrong one pools a wedge's carries into
//! a 7 iron's average. A plain combining accent (U+0301) is *not* in the set: both languages drop
//! it, so `7i` plus U+0301 is a 7 iron in both. `tests` pins both sides, each answer measured
//! against frozen Python. `pyfmt::lower`'s own gap (27 Unicode 16.0 capitals Rust lowers and CPython
//! leaves, M36 P4 finding 6) cannot open the other direction: each lowers to a non-ASCII letter,
//! which Rust keeps, so it too can only refuse. The rejected alternatives
//! were a 5,877-entry exception table with no recorded gate behind it, or a Unicode dependency in
//! a crate whose one third-party dependency is `serde`.

use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::intent::ClubCategory;
use crate::{ContractError, Validate};

/// One slot in a bag. **Declaration order is canonical bag order** and is read, not decorative.
///
/// `Bag::club_ids` presents clubs in this order, and the derived [`Ord`] is it, so a golfer reads
/// their bag the way it sits in the bag rather than alphabetically, where `3w` would land between
/// `2h` and `5h`. Sorting anywhere downstream by the wire name is the bug this ordering exists to
/// prevent. `tests/python_schemas.rs` holds the order to frozen Python's, through the `ClubId` enum
/// pydantic exported into `spec/schemas/bag.schema.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ClubId {
    #[serde(rename = "driver")]
    Driver,

    #[serde(rename = "3w")]
    ThreeWood,
    #[serde(rename = "5w")]
    FiveWood,
    #[serde(rename = "7w")]
    SevenWood,

    #[serde(rename = "2h")]
    TwoHybrid,
    #[serde(rename = "3h")]
    ThreeHybrid,
    #[serde(rename = "4h")]
    FourHybrid,
    #[serde(rename = "5h")]
    FiveHybrid,

    #[serde(rename = "1i")]
    OneIron,
    #[serde(rename = "2i")]
    TwoIron,
    #[serde(rename = "3i")]
    ThreeIron,
    #[serde(rename = "4i")]
    FourIron,
    #[serde(rename = "5i")]
    FiveIron,
    #[serde(rename = "6i")]
    SixIron,
    #[serde(rename = "7i")]
    SevenIron,
    #[serde(rename = "8i")]
    EightIron,
    #[serde(rename = "9i")]
    NineIron,

    #[serde(rename = "pw")]
    PitchingWedge,
    #[serde(rename = "gw")]
    GapWedge,
    #[serde(rename = "sw")]
    SandWedge,
    #[serde(rename = "lw")]
    LobWedge,

    #[serde(rename = "putter")]
    Putter,
}

impl ClubId {
    /// Every club, in declaration order — Python's `list(ClubId)`.
    pub const ALL: [ClubId; 22] = [
        ClubId::Driver,
        ClubId::ThreeWood,
        ClubId::FiveWood,
        ClubId::SevenWood,
        ClubId::TwoHybrid,
        ClubId::ThreeHybrid,
        ClubId::FourHybrid,
        ClubId::FiveHybrid,
        ClubId::OneIron,
        ClubId::TwoIron,
        ClubId::ThreeIron,
        ClubId::FourIron,
        ClubId::FiveIron,
        ClubId::SixIron,
        ClubId::SevenIron,
        ClubId::EightIron,
        ClubId::NineIron,
        ClubId::PitchingWedge,
        ClubId::GapWedge,
        ClubId::SandWedge,
        ClubId::LobWedge,
        ClubId::Putter,
    ];

    /// The wire name (`club.value`), for a sentence rather than a serializer — the same need
    /// [`ClubCategory::as_str`] exists for. `tests` holds it to the serde name.
    pub const fn as_str(self) -> &'static str {
        match self {
            ClubId::Driver => "driver",
            ClubId::ThreeWood => "3w",
            ClubId::FiveWood => "5w",
            ClubId::SevenWood => "7w",
            ClubId::TwoHybrid => "2h",
            ClubId::ThreeHybrid => "3h",
            ClubId::FourHybrid => "4h",
            ClubId::FiveHybrid => "5h",
            ClubId::OneIron => "1i",
            ClubId::TwoIron => "2i",
            ClubId::ThreeIron => "3i",
            ClubId::FourIron => "4i",
            ClubId::FiveIron => "5i",
            ClubId::SixIron => "6i",
            ClubId::SevenIron => "7i",
            ClubId::EightIron => "8i",
            ClubId::NineIron => "9i",
            ClubId::PitchingWedge => "pw",
            ClubId::GapWedge => "gw",
            ClubId::SandWedge => "sw",
            ClubId::LobWedge => "lw",
            ClubId::Putter => "putter",
        }
    }

    /// The Python member's name (`club.name`), which `_build_aliases` reads a spoken spelling off:
    /// `SEVEN_IRON` is how "seven iron" folds. Spelled out because `3w` is not an identifier in
    /// either language, and `tests` holds the aliases it derives to frozen Python's table.
    pub const fn member_name(self) -> &'static str {
        match self {
            ClubId::Driver => "DRIVER",
            ClubId::ThreeWood => "THREE_WOOD",
            ClubId::FiveWood => "FIVE_WOOD",
            ClubId::SevenWood => "SEVEN_WOOD",
            ClubId::TwoHybrid => "TWO_HYBRID",
            ClubId::ThreeHybrid => "THREE_HYBRID",
            ClubId::FourHybrid => "FOUR_HYBRID",
            ClubId::FiveHybrid => "FIVE_HYBRID",
            ClubId::OneIron => "ONE_IRON",
            ClubId::TwoIron => "TWO_IRON",
            ClubId::ThreeIron => "THREE_IRON",
            ClubId::FourIron => "FOUR_IRON",
            ClubId::FiveIron => "FIVE_IRON",
            ClubId::SixIron => "SIX_IRON",
            ClubId::SevenIron => "SEVEN_IRON",
            ClubId::EightIron => "EIGHT_IRON",
            ClubId::NineIron => "NINE_IRON",
            ClubId::PitchingWedge => "PITCHING_WEDGE",
            ClubId::GapWedge => "GAP_WEDGE",
            ClubId::SandWedge => "SAND_WEDGE",
            ClubId::LobWedge => "LOB_WEDGE",
            ClubId::Putter => "PUTTER",
        }
    }
}

/// `f"{club}"` on a `StrEnum` is its value, so this is what every sentence naming a club prints.
impl fmt::Display for ClubId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Validate for ClubId {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// Club -> the benchmark-row family it belongs to. **The one mapping**, so a club's category
/// cannot be two things in two places.
///
/// Written out rather than derived from the digit, because the iron boundaries are the judgement
/// call: 1i–4i long, 5i–7i mid, 8i–9i short is a convention, not arithmetic, and a table can be
/// read and argued with. **No club maps to [`ClubCategory::All`]**, the club-agnostic fallback the
/// shipped bands are keyed on; a club resolving to it would re-point per-club work at the all-club
/// row and still call the answer per-club.
///
/// In declaration order, and the `const` block below holds it there, which is what lets
/// [`category_of`] index it bare by discriminant — the Rust form of Python's exhaustiveness test.
pub const CLUB_CATEGORY: [(ClubId, ClubCategory); 22] = [
    (ClubId::Driver, ClubCategory::Driver),
    (ClubId::ThreeWood, ClubCategory::Wood),
    (ClubId::FiveWood, ClubCategory::Wood),
    (ClubId::SevenWood, ClubCategory::Wood),
    (ClubId::TwoHybrid, ClubCategory::Hybrid),
    (ClubId::ThreeHybrid, ClubCategory::Hybrid),
    (ClubId::FourHybrid, ClubCategory::Hybrid),
    (ClubId::FiveHybrid, ClubCategory::Hybrid),
    (ClubId::OneIron, ClubCategory::LongIron),
    (ClubId::TwoIron, ClubCategory::LongIron),
    (ClubId::ThreeIron, ClubCategory::LongIron),
    (ClubId::FourIron, ClubCategory::LongIron),
    (ClubId::FiveIron, ClubCategory::MidIron),
    (ClubId::SixIron, ClubCategory::MidIron),
    (ClubId::SevenIron, ClubCategory::MidIron),
    (ClubId::EightIron, ClubCategory::ShortIron),
    (ClubId::NineIron, ClubCategory::ShortIron),
    (ClubId::PitchingWedge, ClubCategory::Wedge),
    (ClubId::GapWedge, ClubCategory::Wedge),
    (ClubId::SandWedge, ClubCategory::Wedge),
    (ClubId::LobWedge, ClubCategory::Wedge),
    (ClubId::Putter, ClubCategory::Putter),
];

// Row `i` of the table and of `ClubId::ALL` is the club whose discriminant is `i`. A row added out
// of place fails the build here rather than answering another club's category.
const _: () = {
    let mut i = 0;
    while i < ClubId::ALL.len() {
        assert!(ClubId::ALL[i] as usize == i);
        assert!(CLUB_CATEGORY[i].0 as usize == i);
        i += 1;
    }
};

/// The benchmark-row family a club belongs to. Indexes [`CLUB_CATEGORY`] bare, as Python indexes
/// its dict with no `.get` default: a fallback would mislabel a new club as some other family
/// instead of failing where it was added.
pub fn category_of(club: ClubId) -> ClubCategory {
    CLUB_CATEGORY[club as usize].1
}

/// The trailing letter of a numeric club id, spelled the way a golfer says it (`_SUFFIX_WORDS`).
///
/// A panic for an unrecognised suffix is Python's `KeyError` at import: a `5x` added to [`ClubId`]
/// fails the first time the alias table is built, which `tests` does, rather than parsing as
/// nothing forever.
fn suffix_word(suffix: &str) -> &'static str {
    match suffix {
        "w" => "wood",
        "h" => "hybrid",
        "i" => "iron",
        other => panic!("club suffix {other:?} has no spoken word"),
    }
}

/// Fold text to its comparison key: lowercase, alphanumerics only (`_normalize`).
///
/// Dropping every non-alphanumeric rather than just spaces and hyphens is what makes "7-iron",
/// "7 iron" and "7iron" one key. [`char::is_alphanumeric`] stands in for `str.isalnum()`; the
/// module doc says where the two differ and why the difference only ever refuses.
fn normalize(text: &str) -> String {
    pyfmt::lower(text)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Every accepted spelling -> its club, derived from the declaration rather than listed beside it.
///
/// Three forms per club: the value (`7i`), the member name (`seveniron`, which is how "seven
/// iron" folds), and for numeric clubs the digit with its suffix spoken (`7iron`). A club added to
/// [`ClubId`] is then parseable in all three without touching this. **No bare-number aliases**:
/// "7" is a 7 iron or a 7 wood, and unrecognised text is `None`'s job, not a guess's.
///
/// Panics on a collision, where Python raises at import: two clubs claiming one spelling would
/// make the parser answer whichever was declared last, silently pooling a 3 wood's carries into a
/// 3 hybrid's.
fn build_aliases() -> HashMap<String, ClubId> {
    let mut aliases: HashMap<String, ClubId> = HashMap::new();
    for club in ClubId::ALL {
        let value = club.as_str();
        let mut forms = vec![value.to_string(), club.member_name().replace('_', "")];
        if value.starts_with(|c: char| c.is_ascii_digit()) {
            forms.push(format!("{}{}", &value[..1], suffix_word(&value[1..])));
        }
        for form in forms {
            let key = normalize(&form);
            let claimed = *aliases.entry(key.clone()).or_insert(club);
            assert!(
                claimed == club,
                "alias {} is claimed by both {claimed} and {club}",
                pyfmt::str_repr(&key)
            );
        }
    }
    aliases
}

fn aliases() -> &'static HashMap<String, ClubId> {
    static ALIASES: OnceLock<HashMap<String, ClubId>> = OnceLock::new();
    ALIASES.get_or_init(build_aliases)
}

/// Free text to a club, or `None`. **The only place text becomes a [`ClubId`].**
///
/// Accepts the canonical id, the spoken name and the mixed form — "7i", "7 iron", "7-iron",
/// "Seven Iron", " Driver ", "pitching wedge" — and returns `None` for anything else, including
/// text that looks close. "wedge" and "iron" name a category, not a club; "10i" is not in the bag.
/// Neither is nudged toward a nearest match, because the cost is asymmetric: a refused tag costs
/// one retype at the bay, and a wrong one pools a wedge's carries into a 7 iron's average where
/// nothing downstream will ever flag it (ADR-024 §5). Callers reject `None` rather than substitute
/// a default.
pub fn parse_club(text: &str) -> Option<ClubId> {
    aliases().get(&normalize(text)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `club._ALIASES` as frozen Python built it, extracted by hand at M36 P5 (sorted by key).
    /// Held equal both ways, so a derived form Rust adds or loses is a failure naming itself.
    const PYTHON_ALIASES: [(&str, &str); 58] = [
        ("1i", "1i"),
        ("1iron", "1i"),
        ("2h", "2h"),
        ("2hybrid", "2h"),
        ("2i", "2i"),
        ("2iron", "2i"),
        ("3h", "3h"),
        ("3hybrid", "3h"),
        ("3i", "3i"),
        ("3iron", "3i"),
        ("3w", "3w"),
        ("3wood", "3w"),
        ("4h", "4h"),
        ("4hybrid", "4h"),
        ("4i", "4i"),
        ("4iron", "4i"),
        ("5h", "5h"),
        ("5hybrid", "5h"),
        ("5i", "5i"),
        ("5iron", "5i"),
        ("5w", "5w"),
        ("5wood", "5w"),
        ("6i", "6i"),
        ("6iron", "6i"),
        ("7i", "7i"),
        ("7iron", "7i"),
        ("7w", "7w"),
        ("7wood", "7w"),
        ("8i", "8i"),
        ("8iron", "8i"),
        ("9i", "9i"),
        ("9iron", "9i"),
        ("driver", "driver"),
        ("eightiron", "8i"),
        ("fivehybrid", "5h"),
        ("fiveiron", "5i"),
        ("fivewood", "5w"),
        ("fourhybrid", "4h"),
        ("fouriron", "4i"),
        ("gapwedge", "gw"),
        ("gw", "gw"),
        ("lobwedge", "lw"),
        ("lw", "lw"),
        ("nineiron", "9i"),
        ("oneiron", "1i"),
        ("pitchingwedge", "pw"),
        ("putter", "putter"),
        ("pw", "pw"),
        ("sandwedge", "sw"),
        ("seveniron", "7i"),
        ("sevenwood", "7w"),
        ("sixiron", "6i"),
        ("sw", "sw"),
        ("threehybrid", "3h"),
        ("threeiron", "3i"),
        ("threewood", "3w"),
        ("twohybrid", "2h"),
        ("twoiron", "2i"),
    ];

    #[test]
    fn the_alias_table_is_frozen_pythons() {
        let mut rust: Vec<(String, &str)> = aliases()
            .iter()
            .map(|(key, club)| (key.clone(), club.as_str()))
            .collect();
        rust.sort();
        let python: Vec<(String, &str)> = PYTHON_ALIASES
            .iter()
            .map(|(key, club)| (key.to_string(), *club))
            .collect();
        assert_eq!(rust, python);
    }

    #[test]
    fn every_alias_parses_to_its_club() {
        for (alias, club) in PYTHON_ALIASES {
            assert_eq!(parse_club(alias).map(ClubId::as_str), Some(club), "{alias}");
        }
    }

    #[test]
    fn the_wire_name_is_the_value() {
        for club in ClubId::ALL {
            let wire = serde_json::to_string(&club).expect("a club serializes");
            assert_eq!(wire, format!("\"{}\"", club.as_str()));
            assert_eq!(club.to_string(), club.as_str());
        }
    }

    #[test]
    fn no_club_resolves_to_the_club_agnostic_category() {
        for club in ClubId::ALL {
            assert_ne!(category_of(club), ClubCategory::All, "{club}");
        }
    }

    #[test]
    fn parse_club_round_trips_every_canonical_id() {
        for club in ClubId::ALL {
            assert_eq!(parse_club(club.as_str()), Some(club));
        }
    }

    #[test]
    fn parse_club_accepts_the_ways_a_golfer_writes_it() {
        assert_eq!(parse_club("7 iron"), Some(ClubId::SevenIron));
        assert_eq!(parse_club("7-iron"), Some(ClubId::SevenIron));
        assert_eq!(parse_club("7I"), Some(ClubId::SevenIron));
        assert_eq!(parse_club("Seven Iron"), Some(ClubId::SevenIron));
        assert_eq!(parse_club(" Driver "), Some(ClubId::Driver));
        assert_eq!(parse_club("pitching wedge"), Some(ClubId::PitchingWedge));
        assert_eq!(parse_club("3 wood"), Some(ClubId::ThreeWood));
        assert_eq!(parse_club("4 hybrid"), Some(ClubId::FourHybrid));
    }

    #[test]
    fn parse_club_refuses_rather_than_guessing() {
        for junk in [
            "", "   ", "banana", "10i", "wedge", "iron", "wood", "7", "seven", "0i", "6w",
        ] {
            assert_eq!(parse_club(junk), None, "{junk:?}");
        }
    }

    /// Non-ASCII text, each with frozen Python's answer as measured at M36 P5. The first block
    /// agrees: a character both languages drop, one both keep, and `lower`'s own edges (`İ` lowers
    /// to `i` and a combining dot, which both drop). The second is the named divergence: an
    /// `Other_Alphabetic` mark CPython's `isalnum` drops and [`char::is_alphanumeric`] keeps, so
    /// Rust refuses what Python accepts — never the reverse.
    #[test]
    fn non_ascii_text_refuses_where_it_cannot_agree() {
        let agreeing = [
            ("7i\u{301}", Some(ClubId::SevenIron)),
            ("7\u{130}", Some(ClubId::SevenIron)),
            ("seven\u{a0}iron", Some(ClubId::SevenIron)),
            ("7\u{e9} iron", None),
            ("\u{ff17}\u{ff49}", None),
            ("7\u{131}", None),
            ("\u{212a}w", None),
            ("\u{24b9}river", None),
        ];
        for (text, python) in agreeing {
            assert_eq!(parse_club(text), python, "{text:?}");
        }
        let python_accepts_and_rust_refuses = ["7i\u{345}", "7i\u{363}", "7 iron\u{5b0}"];
        for text in python_accepts_and_rust_refuses {
            assert_eq!(parse_club(text), None, "{text:?}");
        }
    }

    #[test]
    fn declaration_order_is_bag_order() {
        let clubs = ClubId::ALL;
        assert_eq!(clubs[0], ClubId::Driver);
        assert_eq!(clubs[clubs.len() - 1], ClubId::Putter);
        let mut sorted = clubs;
        sorted.sort();
        assert_eq!(sorted, clubs, "the derived Ord is declaration order");

        let irons: Vec<&str> = clubs
            .iter()
            .map(|c| c.as_str())
            .filter(|v| v.ends_with('i'))
            .collect();
        let mut alphabetical = irons.clone();
        alphabetical.sort();
        assert_eq!(irons, alphabetical, "irons are out of order");

        let first = |category: ClubCategory| {
            clubs
                .iter()
                .position(|c| category_of(*c) == category)
                .expect("a club in the category")
        };
        assert!(first(ClubCategory::Wood) < first(ClubCategory::Hybrid));
        assert!(first(ClubCategory::Hybrid) < first(ClubCategory::LongIron));
        assert!(first(ClubCategory::LongIron) < first(ClubCategory::Wedge));
    }
}
