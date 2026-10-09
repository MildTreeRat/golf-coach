//! What a club *is* — one slot of one manufacturer's model, as published. `contracts/club_spec.py`.
//! [M36 P5]
//!
//! [ADR-026](../../../docs/decisions/026-club-specification-lookup.md) is the decision. A Titleist
//! T150 7 iron and a TaylorMade Stealth 7 iron differ in loft, lie, length, offset, bounce and
//! shaft, and every one of those moves launch, spin and start line, so the unit is a **slot of a
//! model** (`(make, model, model_year, club)`), not a model: a set has a different loft in every
//! slot.
//!
//! **Every field but `club` is optional, and blank renders blank** — `None` for a number, `""` for
//! free text. Nothing is defaulted, interpolated between slots or guessed, which is ADR-010 §2
//! unchanged; what ADR-026 §1 changed is only which value counts as known (the published loft, not
//! "measured or nothing").
//!
//! **Two closed vocabularies, and none for `head_type`.** `shaft_material` and `shaft_flex` are
//! enums with tolerant parsers for ADR-024 §1's reason — "S", "stiff" and "Stiff Flex" are one
//! flex. `head_type`, `set_composition`, `grind`, `shaft_kick_point` and `swing_weight` stay
//! strings: marketing vocabulary with no agreed boundary, where a closed set would be this module
//! inventing a taxonomy. `swing_weight` is a string because "D2" is a letter-and-digit scale, not a
//! number to do arithmetic on.
//!
//! The parsers share [`crate::club`]'s fold and its divergence: `str.isalnum()` is
//! [`char::is_alphanumeric`] here, which can only make Rust refuse a spelling CPython would take
//! (every alias is ASCII), and `club`'s module doc carries the measurement.
//!
//! **Pydantic's lax coercions are not ported** (the M36 plan's call 9): pydantic takes `"2023"` or
//! `2023.0` for `model_year`, `"9.5"` for a loft, `1` or `"true"` for `adjustable_hosel`, and
//! `NaN` for a float; this refuses each, and nothing here writes any of them.

use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::club::ClubId;
use crate::{ContractError, Timestamp, Validate};

/// What the shaft is made of. Three answers, because that is how shafts are actually sold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaftMaterial {
    #[serde(rename = "steel")]
    Steel,
    #[serde(rename = "graphite")]
    Graphite,
    #[serde(rename = "multi_material")]
    MultiMaterial,
}

impl ShaftMaterial {
    /// Every member, in declaration order.
    pub const ALL: [ShaftMaterial; 3] = [
        ShaftMaterial::Steel,
        ShaftMaterial::Graphite,
        ShaftMaterial::MultiMaterial,
    ];

    /// The wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            ShaftMaterial::Steel => "steel",
            ShaftMaterial::Graphite => "graphite",
            ShaftMaterial::MultiMaterial => "multi_material",
        }
    }

    /// The Python member's name, which the alias table derives a spelling from.
    const fn member_name(self) -> &'static str {
        match self {
            ShaftMaterial::Steel => "STEEL",
            ShaftMaterial::Graphite => "GRAPHITE",
            ShaftMaterial::MultiMaterial => "MULTI_MATERIAL",
        }
    }
}

impl fmt::Display for ShaftMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How much the shaft bends. **Declaration order is softest to stiffest** and is read.
///
/// A flex scale has a direction, and a picker listing it alphabetically would put ladies between
/// extra-stiff and regular. The scale is not linear and not comparable across makers, so this is a
/// label: nothing should subtract two of them, and no `Ord` is derived to invite it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShaftFlex {
    #[serde(rename = "ladies")]
    Ladies,
    #[serde(rename = "senior")]
    Senior,
    #[serde(rename = "regular")]
    Regular,
    #[serde(rename = "stiff")]
    Stiff,
    #[serde(rename = "x_stiff")]
    XStiff,
    #[serde(rename = "xx_stiff")]
    XxStiff,
}

impl ShaftFlex {
    /// Every member, softest to stiffest.
    pub const ALL: [ShaftFlex; 6] = [
        ShaftFlex::Ladies,
        ShaftFlex::Senior,
        ShaftFlex::Regular,
        ShaftFlex::Stiff,
        ShaftFlex::XStiff,
        ShaftFlex::XxStiff,
    ];

    /// The wire name.
    pub const fn as_str(self) -> &'static str {
        match self {
            ShaftFlex::Ladies => "ladies",
            ShaftFlex::Senior => "senior",
            ShaftFlex::Regular => "regular",
            ShaftFlex::Stiff => "stiff",
            ShaftFlex::XStiff => "x_stiff",
            ShaftFlex::XxStiff => "xx_stiff",
        }
    }

    /// The Python member's name.
    const fn member_name(self) -> &'static str {
        match self {
            ShaftFlex::Ladies => "LADIES",
            ShaftFlex::Senior => "SENIOR",
            ShaftFlex::Regular => "REGULAR",
            ShaftFlex::Stiff => "STIFF",
            ShaftFlex::XStiff => "X_STIFF",
            ShaftFlex::XxStiff => "XX_STIFF",
        }
    }
}

impl fmt::Display for ShaftFlex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Fold to the form the alias tables are keyed in: lowercase, alphanumerics only, then a trailing
/// "flex" dropped (`_normalize`).
///
/// The same fold as [`crate::club`]'s, spelled again rather than shared, for Python's reason: these
/// are two vocabularies, and tying them together would let a change made for club spellings change
/// how a flex parses. Dropping "flex" is what turns "Stiff Flex", "R Flex" and "Regular Flex" into
/// aliases already in the table; nothing else is stripped, because recognising less is the point.
fn normalize(text: &str) -> String {
    let folded: String = pyfmt::lower(text)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    match folded.strip_suffix("flex") {
        Some(stem) if !stem.is_empty() => stem.to_string(),
        _ => folded,
    }
}

/// Every accepted spelling -> its member: the enum's own forms (the value, and the member name
/// with spaces), plus the ones no rule derives — the letter codes printed on a shaft band.
///
/// Panics on a collision, where Python raises at import: two members claiming one spelling would
/// make the parser answer whichever was declared last, and a graphite shaft recorded as steel is a
/// wrong number nothing downstream can detect.
fn build_aliases<M: Copy + PartialEq + fmt::Display>(
    members: &[(M, &str, &str)],
    extra: &[(&str, M)],
) -> HashMap<String, M> {
    let mut aliases: HashMap<String, M> = HashMap::new();
    let derived = members.iter().flat_map(|(member, value, name)| {
        [
            (value.to_string(), *member),
            (name.replace('_', " "), *member),
        ]
    });
    let extra = extra
        .iter()
        .map(|(spelling, member)| (spelling.to_string(), *member));
    for (form, member) in derived.chain(extra) {
        let key = normalize(&form);
        let claimed = *aliases.entry(key.clone()).or_insert(member);
        assert!(
            claimed == member,
            "alias {} is claimed by both {claimed} and {member}",
            pyfmt::str_repr(&key)
        );
    }
    aliases
}

/// Spellings no rule derives from the declaration (`_MATERIAL_EXTRA`).
///
/// **"composite" and "hybrid" are deliberately absent.** Makers use "composite" for both a graphite
/// and a genuinely multi-material shaft, so it names two members and may pick neither; "hybrid"
/// already names a club, and a material table answering a club-type word is the near-miss
/// `parse_club` refuses on principle.
const MATERIAL_EXTRA: [(&str, ShaftMaterial); 4] = [
    ("carbon", ShaftMaterial::Graphite),
    ("carbon fiber", ShaftMaterial::Graphite),
    ("carbon fibre", ShaftMaterial::Graphite),
    ("multi material", ShaftMaterial::MultiMaterial),
];

/// The letter codes and words a shaft band, a retailer and a golfer write (`_FLEX_EXTRA`). Neither
/// "A" nor "R" is derivable from `SENIOR` or `REGULAR` by any rule.
const FLEX_EXTRA: [(&str, ShaftFlex); 11] = [
    ("l", ShaftFlex::Ladies),
    ("w", ShaftFlex::Ladies),
    ("a", ShaftFlex::Senior),
    ("m", ShaftFlex::Senior),
    ("r", ShaftFlex::Regular),
    ("reg", ShaftFlex::Regular),
    ("s", ShaftFlex::Stiff),
    ("x", ShaftFlex::XStiff),
    ("extra stiff", ShaftFlex::XStiff),
    ("xx", ShaftFlex::XxStiff),
    ("double extra stiff", ShaftFlex::XxStiff),
];

fn material_aliases() -> &'static HashMap<String, ShaftMaterial> {
    static ALIASES: OnceLock<HashMap<String, ShaftMaterial>> = OnceLock::new();
    ALIASES.get_or_init(|| {
        let members: Vec<_> = ShaftMaterial::ALL
            .iter()
            .map(|m| (*m, m.as_str(), m.member_name()))
            .collect();
        build_aliases(&members, &MATERIAL_EXTRA)
    })
}

fn flex_aliases() -> &'static HashMap<String, ShaftFlex> {
    static ALIASES: OnceLock<HashMap<String, ShaftFlex>> = OnceLock::new();
    ALIASES.get_or_init(|| {
        let members: Vec<_> = ShaftFlex::ALL
            .iter()
            .map(|m| (*m, m.as_str(), m.member_name()))
            .collect();
        build_aliases(&members, &FLEX_EXTRA)
    })
}

/// Free text to a shaft material, or `None`. **The only place text becomes a [`ShaftMaterial`].**
///
/// `parse_club`'s posture and asymmetry: a refused value costs one correction in a form the golfer
/// is already looking at, and a nudged one puts a graphite shaft's weight under a steel label.
pub fn parse_shaft_material(text: &str) -> Option<ShaftMaterial> {
    material_aliases().get(&normalize(text)).copied()
}

/// Free text to a shaft flex, or `None`. **The only place text becomes a [`ShaftFlex`].**
///
/// "S", "stiff", "Stiff Flex" and "  STIFF  " are one flex; "regular-ish" is `None` and not
/// `Regular`, because a hedge is not a flex and recording it as one loses the hedge.
pub fn parse_shaft_flex(text: &str) -> Option<ShaftFlex> {
    flex_aliases().get(&normalize(text)).copied()
}

/// Where a specification came from, so a looked-up number and a typed one stay distinguishable.
///
/// A block of its own rather than three fields on [`ClubSpec`] because it is **not part of club
/// identity**: `BagEntry::same_club_as` must exclude it whole (ADR-026 §4), and nesting makes that
/// one name instead of three chances to forget a fourth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct SpecProvenance {
    /// What produced these values — `"llm:claude-opus-5"`, `"catalogue"` or `"typed"`. A string,
    /// because the model id is part of the answer.
    pub source: String,
    /// When it was retrieved, which dates the *retrieval* and not the club.
    pub retrieved_at: Timestamp,
    /// Anything the source said about its own confidence, verbatim. Empty is the normal case.
    #[serde(default)]
    pub notes: String,
}

crate::validated!(SpecProvenance);

impl Validate for SpecProvenance {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// One slot of one model, as published. The field list `BagEntry` inherits.
///
/// Grouped as identity, head, shaft, assembly and head performance, as Python declares them. Every
/// field but `club` is `None` or `""` when unknown, which is ADR-010 §2 applied to inputs: a blank
/// lie angle is a lie angle nobody knows, where `0.0` would be a club nobody could play.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ClubSpec {
    // --- Identity: which slot of whose model this is. The catalogue keys four of these.
    /// The slot of the manufacturer's set this describes, and the one field never blank.
    pub club: ClubId,
    #[serde(default)]
    pub make: String,
    #[serde(default)]
    pub model: String,
    /// Part of the key, not decoration: makers reuse a model name across generations with
    /// different lofts. `None` is a year nobody established, never "the newest".
    #[serde(default)]
    pub model_year: Option<i64>,
    #[serde(default)]
    pub head_type: String,
    #[serde(default)]
    pub set_composition: String,

    // --- Head.
    /// The published loft — ADR-026 §1's reversal of ADR-024 §2's "measured, never a catalogue
    /// default", because the alternative a home golfer has is nothing, not a measurement.
    #[serde(default)]
    pub loft_deg: Option<f64>,
    #[serde(default)]
    pub lie_deg: Option<f64>,
    #[serde(default)]
    pub bounce_deg: Option<f64>,
    #[serde(default)]
    pub grind: String,
    #[serde(default)]
    pub offset_mm: Option<f64>,
    #[serde(default)]
    pub face_angle_deg: Option<f64>,
    #[serde(default)]
    pub head_weight_g: Option<f64>,
    /// `None` is not `false`: an unknown hosel and a fixed one are different states.
    #[serde(default)]
    pub adjustable_hosel: Option<bool>,
    /// The adjustable range as `(min, max)`; `loft_deg` stays the setting the club is in.
    #[serde(default)]
    pub loft_range_deg: Option<(f64, f64)>,

    // --- Shaft: the six facts `BagEntry.shaft` was once one string for (ADR-026 §3).
    #[serde(default)]
    pub shaft_model: String,
    #[serde(default)]
    pub shaft_material: Option<ShaftMaterial>,
    #[serde(default)]
    pub shaft_flex: Option<ShaftFlex>,
    #[serde(default)]
    pub shaft_weight_g: Option<f64>,
    #[serde(default)]
    pub shaft_torque_deg: Option<f64>,
    #[serde(default)]
    pub shaft_kick_point: String,

    // --- Assembly.
    #[serde(default)]
    pub length_in: Option<f64>,
    /// `"D2"`: D9 to E0 is one point as D2 to D3 is, so nothing may do arithmetic on it.
    #[serde(default)]
    pub swing_weight: String,
    #[serde(default)]
    pub total_weight_g: Option<f64>,
    #[serde(default)]
    pub grip: String,

    // --- Head performance: published, rarely known, never inferred from the fields above.
    #[serde(default)]
    pub cor: Option<f64>,
    #[serde(default)]
    pub moi_g_cm2: Option<f64>,
    #[serde(default)]
    pub usga_conforming: Option<bool>,
}

crate::validated!(ClubSpec);

impl ClubSpec {
    /// `ClubSpec(club=club)`: a slot and nothing anyone did not supply.
    pub fn blank(club: ClubId) -> Self {
        Self {
            club,
            make: String::new(),
            model: String::new(),
            model_year: None,
            head_type: String::new(),
            set_composition: String::new(),
            loft_deg: None,
            lie_deg: None,
            bounce_deg: None,
            grind: String::new(),
            offset_mm: None,
            face_angle_deg: None,
            head_weight_g: None,
            adjustable_hosel: None,
            loft_range_deg: None,
            shaft_model: String::new(),
            shaft_material: None,
            shaft_flex: None,
            shaft_weight_g: None,
            shaft_torque_deg: None,
            shaft_kick_point: String::new(),
            length_in: None,
            swing_weight: String::new(),
            total_weight_g: None,
            grip: String::new(),
            cor: None,
            moi_g_cm2: None,
            usga_conforming: None,
        }
    }
}

/// `_loft_range_is_ordered`: an adjustable range must run low to high.
///
/// Raises rather than sorting: a reversed pair is a caller bug (for M12, an LLM that returned two
/// numbers in the order it said them), and every use of a range is a containment comparison that a
/// reversed pair answers `false` for every loft, the one the club is set to included. An
/// equal-ended range is a hosel with one setting and passes. The message is Python's, the tuple
/// written as its `repr`.
impl Validate for ClubSpec {
    fn validate(&self) -> Result<(), ContractError> {
        match self.loft_range_deg {
            Some((low, high)) if low > high => Err(ContractError {
                field: "ClubSpec.loft_range_deg".to_string(),
                problem: format!(
                    "loft_range_deg ({}, {}) runs high to low",
                    pyfmt::repr(low),
                    pyfmt::repr(high)
                ),
            }),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// `_MATERIAL_ALIASES` and `_FLEX_ALIASES` as frozen Python built them, extracted by hand at
    /// M36 P5 (sorted by key).
    const PYTHON_MATERIAL_ALIASES: [(&str, &str); 6] = [
        ("carbon", "graphite"),
        ("carbonfiber", "graphite"),
        ("carbonfibre", "graphite"),
        ("graphite", "graphite"),
        ("multimaterial", "multi_material"),
        ("steel", "steel"),
    ];
    const PYTHON_FLEX_ALIASES: [(&str, &str); 17] = [
        ("a", "senior"),
        ("doubleextrastiff", "xx_stiff"),
        ("extrastiff", "x_stiff"),
        ("l", "ladies"),
        ("ladies", "ladies"),
        ("m", "senior"),
        ("r", "regular"),
        ("reg", "regular"),
        ("regular", "regular"),
        ("s", "stiff"),
        ("senior", "senior"),
        ("stiff", "stiff"),
        ("w", "ladies"),
        ("x", "x_stiff"),
        ("xstiff", "x_stiff"),
        ("xx", "xx_stiff"),
        ("xxstiff", "xx_stiff"),
    ];

    fn sorted<M: fmt::Display>(aliases: &HashMap<String, M>) -> Vec<(String, String)> {
        let mut out: Vec<_> = aliases
            .iter()
            .map(|(key, member)| (key.clone(), member.to_string()))
            .collect();
        out.sort();
        out
    }

    fn owned(table: &[(&str, &str)]) -> Vec<(String, String)> {
        table
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn the_alias_tables_are_frozen_pythons() {
        assert_eq!(sorted(material_aliases()), owned(&PYTHON_MATERIAL_ALIASES));
        assert_eq!(sorted(flex_aliases()), owned(&PYTHON_FLEX_ALIASES));
    }

    #[test]
    fn every_spelling_of_stiff_is_one_flex() {
        for text in [
            "S",
            "s",
            "stiff",
            "Stiff",
            "Stiff Flex",
            "  STIFF  ",
            "stiff-flex",
        ] {
            assert_eq!(parse_shaft_flex(text), Some(ShaftFlex::Stiff), "{text:?}");
        }
    }

    #[test]
    fn letter_codes_and_words_reach_the_same_members() {
        let cases = [
            ("L", ShaftFlex::Ladies),
            ("A", ShaftFlex::Senior),
            ("senior", ShaftFlex::Senior),
            ("R", ShaftFlex::Regular),
            ("Reg", ShaftFlex::Regular),
            ("R Flex", ShaftFlex::Regular),
            ("X", ShaftFlex::XStiff),
            ("Extra Stiff", ShaftFlex::XStiff),
            ("x_stiff", ShaftFlex::XStiff),
            ("XX", ShaftFlex::XxStiff),
        ];
        for (text, expected) in cases {
            assert_eq!(parse_shaft_flex(text), Some(expected), "{text:?}");
        }
    }

    #[test]
    fn shaft_materials_parse_from_their_spoken_spellings() {
        let cases = [
            ("steel", ShaftMaterial::Steel),
            ("Steel", ShaftMaterial::Steel),
            ("graphite", ShaftMaterial::Graphite),
            ("Carbon Fiber", ShaftMaterial::Graphite),
            ("carbon fibre", ShaftMaterial::Graphite),
            ("multi-material", ShaftMaterial::MultiMaterial),
        ];
        for (text, expected) in cases {
            assert_eq!(parse_shaft_material(text), Some(expected), "{text:?}");
        }
    }

    #[test]
    fn a_flex_that_is_not_one_refuses_rather_than_nudging() {
        for text in [
            "regular-ish",
            "fairly stiff",
            "flex",
            "medium",
            "",
            "   ",
            "7i",
        ] {
            assert_eq!(parse_shaft_flex(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_material_that_names_two_answers_or_none_refuses() {
        for text in ["composite", "hybrid", "titanium", "aluminium", ""] {
            assert_eq!(parse_shaft_material(text), None, "{text:?}");
        }
    }

    #[test]
    fn every_member_parses_from_its_own_value() {
        for member in ShaftFlex::ALL {
            assert_eq!(parse_shaft_flex(member.as_str()), Some(member));
        }
        for member in ShaftMaterial::ALL {
            assert_eq!(parse_shaft_material(member.as_str()), Some(member));
        }
    }

    /// The named divergence, on the flex: U+0345 is an `Other_Alphabetic` mark that CPython's
    /// `isalnum` drops (so Python reads `stiff`) and Rust keeps (so this refuses).
    #[test]
    fn an_other_alphabetic_mark_refuses_where_python_reads_through_it() {
        assert_eq!(parse_shaft_flex("stiff\u{345} flex"), None);
        assert_eq!(
            parse_shaft_flex("stiff\u{301} flex"),
            Some(ShaftFlex::Stiff)
        );
    }

    /// Order is read, not decorative: a picker lists the members in it, and ladies between
    /// extra-stiff and regular is a bug a golfer cannot tell from one in their own bag.
    /// `tests/python_schemas.rs` holds the same order to the enum pydantic exported.
    #[test]
    fn flex_declaration_order_runs_softest_to_stiffest() {
        let order: Vec<&str> = ShaftFlex::ALL.iter().map(|f| f.as_str()).collect();
        assert_eq!(
            order,
            ["ladies", "senior", "regular", "stiff", "x_stiff", "xx_stiff"]
        );
    }

    #[test]
    fn the_wire_names_are_the_python_values() {
        for member in ShaftFlex::ALL {
            let wire = serde_json::to_string(&member).unwrap();
            assert_eq!(wire, format!("\"{}\"", member.as_str()));
        }
        for member in ShaftMaterial::ALL {
            let wire = serde_json::to_string(&member).unwrap();
            assert_eq!(wire, format!("\"{}\"", member.as_str()));
        }
    }

    #[test]
    fn a_spec_with_only_a_club_is_valid_and_fills_nothing_in() {
        let spec: ClubSpec = serde_json::from_value(json!({"club": "7i"})).unwrap();
        assert_eq!(spec, ClubSpec::blank(ClubId::SevenIron));
        let written = serde_json::to_value(&spec).unwrap();
        let filled: Vec<&String> = written
            .as_object()
            .unwrap()
            .iter()
            .filter(|(key, value)| *key != "club" && !value.is_null() && value.as_str() != Some(""))
            .map(|(key, _)| key)
            .collect();
        assert!(filled.is_empty(), "{filled:?}");
    }

    #[test]
    fn a_spec_round_trips_through_json() {
        let spec = ClubSpec {
            make: "Titleist".into(),
            model: "TSR3".into(),
            model_year: Some(2022),
            loft_deg: Some(9.0),
            adjustable_hosel: Some(true),
            loft_range_deg: Some((7.25, 10.75)),
            shaft_material: Some(ShaftMaterial::Graphite),
            shaft_flex: Some(ShaftFlex::Stiff),
            swing_weight: "D3".into(),
            ..ClubSpec::blank(ClubId::Driver)
        };
        let text = serde_json::to_string(&spec).unwrap();
        let restored: ClubSpec = serde_json::from_str(&text).unwrap();
        assert_eq!(restored, spec);
        assert_eq!(restored.loft_range_deg, Some((7.25, 10.75)));
    }

    #[test]
    fn an_adjustable_range_that_runs_backwards_raises_with_pythons_message() {
        let spec = ClubSpec {
            loft_range_deg: Some((10.75, 7.25)),
            ..ClubSpec::blank(ClubId::Driver)
        };
        let err = spec.validate().unwrap_err();
        assert_eq!(err.problem, "loft_range_deg (10.75, 7.25) runs high to low");
        let parsed = serde_json::from_value::<ClubSpec>(
            json!({"club": "driver", "loft_range_deg": [10, 7]}),
        );
        let message = parsed.unwrap_err().to_string();
        assert!(
            message.contains("loft_range_deg (10.0, 7.0) runs high to low"),
            "{message}"
        );
    }

    #[test]
    fn an_equal_ended_range_is_accepted() {
        let spec = ClubSpec {
            loft_range_deg: Some((9.0, 9.0)),
            ..ClubSpec::blank(ClubId::Driver)
        };
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn provenance_carries_its_source_and_defaults_its_notes_empty() {
        let provenance: SpecProvenance = serde_json::from_value(json!({
            "source": "llm:claude-opus-5",
            "retrieved_at": "2026-08-31T12:00:00Z",
        }))
        .unwrap();
        assert_eq!(provenance.source, "llm:claude-opus-5");
        assert_eq!(provenance.retrieved_at.to_string(), "2026-08-31T12:00:00Z");
        assert_eq!(provenance.notes, "");
    }

    /// Call 9's divergence, each with pydantic's answer measured at M36 P5: pydantic takes all of
    /// these, and this refuses them.
    #[test]
    fn pydantics_lax_coercions_are_refused() {
        for spec in [
            json!({"club": "driver", "model_year": 2023.0}),
            json!({"club": "driver", "model_year": "2023"}),
            json!({"club": "driver", "loft_deg": "9.5"}),
            json!({"club": "driver", "adjustable_hosel": "true"}),
            json!({"club": "driver", "adjustable_hosel": 1}),
        ] {
            assert!(
                serde_json::from_value::<ClubSpec>(spec.clone()).is_err(),
                "{spec}"
            );
        }
        // And what pydantic refuses too: a three-tuple, a capitalised club, a null string.
        for spec in [
            json!({"club": "driver", "loft_range_deg": [1, 2, 3]}),
            json!({"club": "Driver"}),
            json!({"club": "driver", "make": null}),
        ] {
            assert!(
                serde_json::from_value::<ClubSpec>(spec.clone()).is_err(),
                "{spec}"
            );
        }
    }
}
