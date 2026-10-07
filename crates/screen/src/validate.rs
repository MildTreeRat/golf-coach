//! Physics cross-checks on a parsed screen: `validate.py::validate_parse`, ported faithfully, and
//! then given ranges for the fields M32 added. [M34 P6, P9]
//!
//! OCR does not fail loudly. A dropped digit turns a 128.1-yard carry into 28.1, which is a perfectly
//! well-formed number that will quietly skew every session average it lands in. The defence is that
//! the screen prints *redundant* metrics, and the redundancy is only self-consistent if the parse was
//! right:
//!
//! ```text
//! smash factor  ==  ball speed / club speed
//! shot distance ==  carry + bounce & roll
//! sign(spin axis) agrees with the curvature word in shot type
//! ```
//!
//! The first two hold exactly on real screens (101.5 / 88.6 = 1.15; 128.1 + 23.4 = 151.5), so a
//! mismatch is evidence about the *parse*, not about the shot. The third is the same idea applied to
//! a sign, and it exists because a flipped sign is the one OCR failure the other two cannot see:
//! every magnitude can be perfect and the shot still be reported as curving the wrong way.
//!
//! The ranges are the same idea at coarser grain. They catch an inserted or dropped digit, so they
//! are deliberately loose. Nothing here judges whether a shot was *good*, only whether it was read
//! correctly: an odd-but-consistent reading (a 0.89 smash factor) passes, because the parser
//! faithfully reported what the device displayed.
//!
//! # Where CPython is reproduced
//!
//! Every message reaches `ShotProvenance.warnings` and is compared exactly, so each piece of one is
//! the CPython operation: `{x:g}` is [`pyfmt::g`], `{x:.3f}` is [`pyfmt::fixed`], `{shape!r}` is
//! [`pyfmt::str_repr`], and the shape words are found with [`pyfmt::upper`] and [`pyfmt::split`].
//! `max` and `min` keep their first argument on a tie (`parser::first_max`). The penalty is `+=` left
//! to right, as the Python adds it, so nothing here wants [`pyfmt::sum`]: compensation is a property
//! of `sum()`, not of `+`.
//!
//! # What gates it
//!
//! The synthetic screen vectors (M34 P4's finding 7) carry every path `test_screen_validate.py`
//! covers, and `tests/read.rs` holds each to its `expected.shot`. No photo trips a check: every stored
//! shot is a consistent read. The six ranges frozen Python never had are reached by no vector, since
//! no shipped profile reads their fields, so a unit test below reads them through a profile made up
//! for it.

use crate::parser::{first_max, first_min, FieldValue, ParsedShot};

/// Relative slack on the smash-factor identity. The screen rounds smash to two places, which is
/// worth about 0.4% on a typical shot, and 3% leaves room for that plus the rounding of both speeds.
const SMASH_TOLERANCE: f64 = 0.03;

/// Slack on `carry + bounce == total`, in yards and as a fraction, whichever is larger.
const DISTANCE_TOLERANCE_YARDS: f64 = 1.0;
const DISTANCE_TOLERANCE_RATIO: f64 = 0.015;

/// Deliberately wide: these catch "142.1 read as 1421", not an unusual swing. In the Python's dict
/// order, because that is the order a shot with two out-of-range fields lists its warnings in.
///
/// **The last six are Rust's alone** (M34 P9, the M34 plan's decision 4): the numeric fields M32
/// added to `ShotData`, in its field order, after the twelve frozen Python has. No shipped profile
/// targets any of them, so no photo reaches them yet and the faithful gate cannot see them. They
/// land now because a range is data, and the first device that prints one should not need someone
/// to remember it. `impact_position_v` is words, and has none.
///
/// No shot has ever carried one of the six, so **each bound is argued, not measured**, on the rule
/// the twelve follow: past anything a real strike does, because a range failure costs 0.15 and
/// flags the shot, and a bound a real shot can cross would flag the golfer for being unusual; and
/// near enough that a value ten times a typical one (a lost decimal point, an inserted digit) lands
/// outside. `the_new_fields_are_checked_once_a_profile_reads_them` holds each one to its example.
const PLAUSIBLE_RANGES: [(&str, f64, f64); 18] = [
    ("club_head_speed", 20.0, 200.0),
    ("ball_speed", 20.0, 250.0),
    ("smash_factor", 0.4, 1.7),
    ("launch_angle", -15.0, 70.0),
    ("launch_direction", -45.0, 45.0),
    ("club_path", -30.0, 30.0),
    ("club_face_angle", -30.0, 30.0),
    ("spin_rate", 0.0, 15000.0),
    ("spin_axis", -45.0, 45.0),
    ("carry_distance", 0.0, 450.0),
    ("total_distance", 0.0, 500.0),
    ("bounce_and_roll", 0.0, 150.0),
    // Degrees, + = up. A steep wedge arrives a few degrees down and a teed driver a few up, so a
    // typical -4.5 read as -45 is outside.
    ("attack_angle", -20.0, 20.0),
    // Degrees. A driver delivers about ten and a lob wedge about fifty; below zero needs the shaft
    // leaning further forward than a full swing leans it. A 15.2 read as 152 is outside.
    ("dynamic_loft", -10.0, 80.0),
    // Inches, + = ahead of the ball. A full swing bottoms out within a few inches of the ball, so a
    // 3.5 read as 35 is outside.
    ("low_point", -10.0, 10.0),
    // Millimetres from the face's centre, + = toe and + = high. The widest face, a driver's, is
    // not much over 100 mm heel to toe and is shallower than that, so ±60 is past its edge either
    // way, where a strike is no longer on the face to measure. A 12 read as 120 is outside.
    ("impact_offset_h", -60.0, 60.0),
    ("impact_offset_v", -60.0, 60.0),
    // Yards, + = right. A ball that starts 45° off line (the edge of `launch_direction`'s range)
    // and carries 200 yards lands about 140 off it, so 150 refuses no shot this table admits short
    // of that. A 45.2 read as 452 is outside. A small offline that loses its decimal point (12.5
    // as 125) is not, the blind spot `carry_distance`'s range has for 128.1 read as 28.1: a
    // tighter bound would flag a real slice to close it.
    ("carry_offline", -150.0, 150.0),
];

/// The curvature words HD Golf prints in the Shot Type tile (`SLIGHT FADE`, `DRAW`, `PUSH SLICE`),
/// matched as whole words so `FADE` never matches inside another token.
const FADE_WORDS: [&str; 2] = ["FADE", "SLICE"];
const DRAW_WORDS: [&str; 2] = ["DRAW", "HOOK"];

/// Below this the axis is too flat for the shape word to be evidence about its sign: the device
/// still prints `SLIGHT FADE` for a ball that curved a foot.
const AXIS_DEADBAND: f64 = 1.0;

/// What a failed check costs. A broken identity is the strongest signal available that the parse is
/// wrong, so it outweighs a merely out-of-range value.
const IDENTITY_PENALTY: f64 = 0.25;
const RANGE_PENALTY: f64 = 0.15;

/// Cross-check a parse: its warnings gain one line per failed check, its confidence loses that
/// check's penalty, and it is flagged for review.
///
/// `needs_review` is set when any check fails *or* the confidence lands below `min_confidence`. The
/// two catch different failures, so either alone is enough. Values and raw text are untouched: the
/// validator decides whether to trust a read, never what it read.
pub fn validate_parse(mut parsed: ParsedShot, min_confidence: f64) -> ParsedShot {
    let mut penalty = 0.0;

    for message in identity_failures(&parsed) {
        parsed.warnings.push(message);
        penalty += IDENTITY_PENALTY;
    }
    for message in range_failures(&parsed) {
        parsed.warnings.push(message);
        penalty += RANGE_PENALTY;
    }

    let confidence = first_max(0.0, first_min(1.0, parsed.confidence - penalty));
    parsed.confidence = confidence;
    parsed.needs_review = penalty > 0.0 || confidence < min_confidence;
    parsed
}

/// The checks that only hold when every metric involved was read correctly.
fn identity_failures(parsed: &ParsedShot) -> Vec<String> {
    let mut failures = Vec::new();

    let ball = number(parsed, "ball_speed");
    let club = number(parsed, "club_head_speed");
    let smash = number(parsed, "smash_factor");
    if let (Some(ball), Some(club), Some(smash)) = (ball, club, smash) {
        if club > 0.0 {
            let expected = ball / club;
            let scale = first_max(first_max(expected, smash), 1e-6);
            if (expected - smash).abs() > SMASH_TOLERANCE * scale {
                failures.push(format!(
                    "smash factor {} does not match ball speed / club speed ({} / {} = {}) - one \
                     of the three was misread",
                    pyfmt::g(smash),
                    pyfmt::g(ball),
                    pyfmt::g(club),
                    pyfmt::fixed(expected, 3)
                ));
            }
        }
    }

    let carry = number(parsed, "carry_distance");
    let roll = number(parsed, "bounce_and_roll");
    let total = number(parsed, "total_distance");
    if let (Some(carry), Some(roll), Some(total)) = (carry, roll, total) {
        let expected_total = carry + roll;
        let tolerance = first_max(
            DISTANCE_TOLERANCE_YARDS,
            DISTANCE_TOLERANCE_RATIO * first_max(total.abs(), 1e-6),
        );
        if (expected_total - total).abs() > tolerance {
            failures.push(format!(
                "shot distance {} does not match carry + bounce & roll ({} + {} = {}) - one of \
                 the three was misread",
                pyfmt::g(total),
                pyfmt::g(carry),
                pyfmt::g(roll),
                pyfmt::g(expected_total)
            ));
        }
    }

    failures.extend(spin_axis_disagreement(parsed));
    failures
}

/// Does the axis curve the ball the way the shot-type tile says it curved?
///
/// `spin_axis` is signed `+ = fade` by the contract, and the device names the shape in words a tile
/// away. Disagreement means the digits, the profile's `printed_sign` polarity or the shape word went
/// wrong, and all three are worth a human look, because none of them makes the number *look* wrong.
fn spin_axis_disagreement(parsed: &ParsedShot) -> Option<String> {
    let axis = number(parsed, "spin_axis")?;
    // `isinstance(shape, str)`: a shape that was not read, or read as a number, has nothing to say.
    let Some(FieldValue::Text(shape)) = parsed.values.get("shot_type") else {
        return None;
    };

    let upper = pyfmt::upper(shape);
    let words: Vec<&str> = pyfmt::split(&upper).collect();
    let fade = words.iter().any(|word| FADE_WORDS.contains(word));
    let draw = words.iter().any(|word| DRAW_WORDS.contains(word));
    if fade == draw {
        // Neither named, or a contradictory tile: nothing to check against.
        return None;
    }
    if axis.abs() < AXIS_DEADBAND {
        // A shape word beside a near-zero axis is the device rounding a straight ball into a
        // direction, not evidence about the sign.
        return None;
    }

    if (axis > 0.0) == fade {
        return None;
    }
    let (shape_word, expected) = if fade {
        ("fade", "positive")
    } else {
        ("draw", "negative")
    };
    Some(format!(
        "spin axis {} disagrees with shot type {} - a {shape_word} needs a {expected} axis \
         (+ = fade), so the sign or the shape word was misread",
        pyfmt::g(axis),
        pyfmt::str_repr(shape)
    ))
}

fn range_failures(parsed: &ParsedShot) -> Vec<String> {
    PLAUSIBLE_RANGES
        .iter()
        .filter_map(|&(name, low, high)| {
            let value = number(parsed, name)?;
            // `not low <= value <= high`, which `contains` is, a NaN included (it is outside).
            (!(low..=high).contains(&value)).then(|| {
                format!(
                    "{name} {} is outside the plausible range {}-{} - likely a dropped or extra \
                     digit",
                    pyfmt::g(value),
                    pyfmt::g(low),
                    pyfmt::g(high)
                )
            })
        })
        .collect()
}

/// `_number`: a value read as a number, or `None` for one not read or read as words.
fn number(parsed: &ParsedShot, name: &str) -> Option<f64> {
    match parsed.values.get(name) {
        Some(FieldValue::Number(value)) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_screen;
    use crate::TextBox;

    fn parsed(values: &[(&str, FieldValue)], confidence: f64) -> ParsedShot {
        ParsedShot {
            device: "hd_golf".to_string(),
            values: values
                .iter()
                .map(|(key, value)| (key.to_string(), value.clone()))
                .collect(),
            raw_fields: Default::default(),
            confidence,
            warnings: vec!["a parser warning".to_string()],
            needs_review: false,
            fields_present: Vec::new(),
        }
    }

    /// The clamp at zero, which no vector reaches (the lowest recorded is 0.585): five failed
    /// checks (0.85) on a 0.5 parse go to `0.0`, not below it. Frozen Python's answer for the same
    /// `ParsedShot`, measured, warnings and all, which also pins the identities before the ranges
    /// and the ranges in their table's order.
    #[test]
    fn a_confidence_is_clamped_at_zero() {
        let validated = validate_parse(
            parsed(
                &[
                    ("ball_speed", FieldValue::Number(1015.0)),
                    ("club_head_speed", FieldValue::Number(8.6)),
                    ("smash_factor", FieldValue::Number(9.9)),
                    ("spin_rate", FieldValue::Number(-1.0)),
                ],
                0.5,
            ),
            0.6,
        );
        assert_eq!(validated.confidence.to_bits(), 0.0f64.to_bits());
        assert!(validated.needs_review);
        assert_eq!(
            validated.warnings,
            [
                "a parser warning",
                "smash factor 9.9 does not match ball speed / club speed (1015 / 8.6 = 118.023) - \
                 one of the three was misread",
                "club_head_speed 8.6 is outside the plausible range 20-200 - likely a dropped or \
                 extra digit",
                "ball_speed 1015 is outside the plausible range 20-250 - likely a dropped or extra \
                 digit",
                "smash_factor 9.9 is outside the plausible range 0.4-1.7 - likely a dropped or \
                 extra digit",
                "spin_rate -1 is outside the plausible range 0-15000 - likely a dropped or extra \
                 digit",
            ]
        );
    }

    /// A contradictory tile names both shapes, so it is evidence of neither, whatever the axis. In
    /// lower case, so the words are found only through `upper` as the Python finds them.
    #[test]
    fn a_tile_naming_both_shapes_is_not_checked() {
        let validated = validate_parse(
            parsed(
                &[
                    ("spin_axis", FieldValue::Number(-9.3)),
                    ("shot_type", FieldValue::Text("fade hook".to_string())),
                ],
                0.9,
            ),
            0.6,
        );
        assert_eq!(validated.warnings, ["a parser warning"]);
        assert!(!validated.needs_review);
    }

    /// No club speed above zero, no smash check. The negative case is the one that shows it: at
    /// zero, Rust's `inf` fails no comparison and the guard looks idle (Python would raise), but at
    /// `-88.6` the unguarded check would add a smash warning frozen Python does not write.
    #[test]
    fn a_club_speed_not_above_zero_skips_the_smash_identity() {
        for (club, printed) in [(0.0, "0"), (-88.6, "-88.6")] {
            let validated = validate_parse(
                parsed(
                    &[
                        ("ball_speed", FieldValue::Number(101.5)),
                        ("club_head_speed", FieldValue::Number(club)),
                        ("smash_factor", FieldValue::Number(1.15)),
                    ],
                    0.9,
                ),
                0.6,
            );
            // Only the range check on the club speed fires: frozen Python's 0.75, flagged.
            assert_eq!(
                validated.warnings[1..],
                [format!(
                    "club_head_speed {printed} is outside the plausible range 20-200 - likely a \
                     dropped or extra digit"
                )]
            );
            assert_eq!(validated.confidence, 0.75);
            assert!(validated.needs_review);
        }
    }

    /// Words are matched after `upper`, so a lower-case tile still names its shape. The parser
    /// upper-cases every text tile before this sees it, so only a direct caller reaches the case,
    /// and frozen Python's answer for it is this one.
    #[test]
    fn a_shape_word_counts_in_any_case() {
        let validated = validate_parse(
            parsed(
                &[
                    ("spin_axis", FieldValue::Number(-9.3)),
                    ("shot_type", FieldValue::Text("slight fade".to_string())),
                ],
                0.9,
            ),
            0.6,
        );
        assert_eq!(
            validated.warnings[1..],
            ["spin axis -9.3 disagrees with shot type 'slight fade' - a fade needs a positive axis \
              (+ = fade), so the sign or the shape word was misread"]
        );
        assert_eq!(validated.confidence, 0.65);
    }

    /// Above about 67 yards the slack is the 1.5% and not the yard: 220 against 222 passes, 220
    /// against 224 does not. Frozen Python's answers for both.
    #[test]
    fn a_long_shot_gets_proportional_slack() {
        let with_total = |total: f64| {
            validate_parse(
                parsed(
                    &[
                        ("carry_distance", FieldValue::Number(200.0)),
                        ("bounce_and_roll", FieldValue::Number(20.0)),
                        ("total_distance", FieldValue::Number(total)),
                    ],
                    0.9,
                ),
                0.6,
            )
        };
        assert_eq!(with_total(222.0).warnings, ["a parser warning"]);
        assert_eq!(
            with_total(224.0).warnings[1..],
            [
                "shot distance 224 does not match carry + bounce & roll (200 + 20 = 220) - one of \
              the three was misread"
            ]
        );
    }

    /// The smash slack is 3% of the larger of the computed and the printed smash, so a printed
    /// 1.0305 against a computed 1.0 passes, where 3% of the computed one alone would fail it.
    #[test]
    fn the_smash_slack_scales_with_the_larger_of_the_two() {
        let validated = validate_parse(
            parsed(
                &[
                    ("ball_speed", FieldValue::Number(100.0)),
                    ("club_head_speed", FieldValue::Number(100.0)),
                    ("smash_factor", FieldValue::Number(1.0305)),
                ],
                0.9,
            ),
            0.6,
        );
        assert_eq!(validated.warnings, ["a parser warning"]);
        assert!(!validated.needs_review);
    }

    /// The six ranges M34 P9 added, reached the way a photo would reach them: a profile whose tiles
    /// target M32's fields, laid out as one row of labels with a value under each, parsed, then
    /// validated. No shipped profile targets any of them (`no_shipped_profile_reads_the_new_fields`),
    /// so this made-up one is the only way in.
    ///
    /// Each value is its range comment's example, read once as printed and once with the slip the
    /// comment names. Every slip is caught, in the table's order, and nothing else is; the clean read
    /// passes untouched.
    #[test]
    fn the_new_fields_are_checked_once_a_profile_reads_them() {
        // (label, target, as printed, with the slip). The labels are chosen to sit well below the
        // matcher's 0.8 against each other, so no tie rule is involved: `Impact Offset H` and
        // `Impact Offset V` would score 0.93 and resolve only on the margin.
        const READS: [(&str, &str, &str, &str); 6] = [
            ("Attack Angle", "attack_angle", "-4.5", "-45"),
            ("Dynamic Loft", "dynamic_loft", "15.2", "152"),
            ("Low Point", "low_point", "3.5", "35"),
            ("Toe Heel", "impact_offset_h", "12", "120"),
            ("High Low", "impact_offset_v", "-8.1", "-81"),
            ("Offline", "carry_offline", "45.2", "452"),
        ];
        let fields: Vec<String> = READS
            .iter()
            .map(|(label, target, _, _)| format!(r#"{{"label": "{label}", "target": "{target}"}}"#))
            .collect();
        let profile = crate::profile::parse_profiles(&format!(
            r#"{{"profiles": [{{"device": "t", "title": "T", "fields": [{}]}}]}}"#,
            fields.join(", ")
        ))
        .unwrap()
        .remove("t")
        .unwrap();
        let screen = |slipped: bool| -> Vec<TextBox> {
            READS
                .iter()
                .enumerate()
                .flat_map(|(at, &(label, _, printed, slip))| {
                    let x = 200.0 * at as f64;
                    let value = if slipped { slip } else { printed };
                    [
                        TextBox {
                            text: label.to_string(),
                            x,
                            y: 0.0,
                            width: 100.0,
                            height: 10.0,
                            confidence: 1.0,
                        },
                        TextBox {
                            text: value.to_string(),
                            x: x + 10.0,
                            y: 20.0,
                            width: 60.0,
                            height: 10.0,
                            confidence: 1.0,
                        },
                    ]
                })
                .collect()
        };

        let clean = validate_parse(parse_screen(&screen(false), &profile), 0.6);
        let read: Vec<(&str, f64)> = clean
            .values
            .iter()
            .map(|(target, value)| match value {
                FieldValue::Number(number) => (target, *number),
                FieldValue::Text(text) => panic!("{target} read as words: {text:?}"),
            })
            .collect();
        assert_eq!(
            read,
            [
                ("attack_angle", -4.5),
                ("dynamic_loft", 15.2),
                ("low_point", 3.5),
                ("impact_offset_h", 12.0),
                ("impact_offset_v", -8.1),
                ("carry_offline", 45.2),
            ]
        );
        assert!(clean.warnings.is_empty(), "{:?}", clean.warnings);
        assert!(!clean.needs_review);

        let slipped = validate_parse(parse_screen(&screen(true), &profile), 0.6);
        assert_eq!(
            slipped.warnings,
            [
                "attack_angle -45 is outside the plausible range -20-20 - likely a dropped or extra \
                 digit",
                "dynamic_loft 152 is outside the plausible range -10-80 - likely a dropped or extra \
                 digit",
                "low_point 35 is outside the plausible range -10-10 - likely a dropped or extra \
                 digit",
                "impact_offset_h 120 is outside the plausible range -60-60 - likely a dropped or \
                 extra digit",
                "impact_offset_v -81 is outside the plausible range -60-60 - likely a dropped or \
                 extra digit",
                "carry_offline 452 is outside the plausible range -150-150 - likely a dropped or \
                 extra digit",
            ]
        );
        // Six range failures cost 0.15 each, and nothing else differs between the two reads.
        assert!(slipped.needs_review);
        assert!((clean.confidence - slipped.confidence - 6.0 * RANGE_PENALTY).abs() < 1e-9);
    }

    /// The other half of the claim the ranges' doc makes: no shipped profile targets any of the
    /// fields they were added for, so no photo, and no committed vector, can reach them yet. When a
    /// profile first does, this fails, and the ranges stop being a precaution and start gating reads.
    #[test]
    fn no_shipped_profile_reads_the_new_fields() {
        let added = &PLAUSIBLE_RANGES[12..];
        assert_eq!(
            added[0].0, "attack_angle",
            "the twelve Python has come first"
        );
        for profile in crate::profile::profiles().values() {
            for field in profile.stored_fields() {
                let target = field
                    .target
                    .as_deref()
                    .expect("a stored field has a target");
                assert!(
                    added.iter().all(|&(name, _, _)| name != target),
                    "{} reads {target}, so its range is no longer unreachable: say so in \
                     PLAUSIBLE_RANGES' doc",
                    profile.device
                );
            }
        }
    }

    /// The ranges are closed: a value on either bound is inside.
    #[test]
    fn a_value_on_a_bound_is_inside() {
        let validated = validate_parse(
            parsed(
                &[
                    ("spin_rate", FieldValue::Number(0.0)),
                    ("carry_distance", FieldValue::Number(450.0)),
                    ("launch_angle", FieldValue::Number(-15.0)),
                    ("club_path", FieldValue::Number(30.0)),
                ],
                0.9,
            ),
            0.6,
        );
        assert_eq!(validated.warnings, ["a parser warning"]);
        assert_eq!(validated.confidence, 0.9);
    }
}
