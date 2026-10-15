//! The profile family's runner: one case under `spec/vectors/profile/` in, the `GradedProfile`
//! [`run_profile`] answers from its input out, in `expected`'s shape. [M37 P9; the corpus cases P11]
//!
//! **No Python twin, and no Python oracle.** The family has two sub-families, and their oracles
//! differ:
//!
//! - **`hand/`** (M37 P5, `provenance.oracle: "hand"`): worked by hand from the M37 plan's decisions
//!   and calls, before the code that must meet it existed; each `note` shows its working. These are
//!   what the grades' correctness rests on.
//! - **`corpus/`** (M37 P11, `provenance.oracle: "rust"`, the plan's call 18 and ADR-032's twelfth
//!   addendum's third kind): one per career case whose corpus holds a shot, its `input` adopted from
//!   that case ([`ADOPTED`]) and its `expected` written once by this runner. Nothing independent
//!   stands behind them, so they pin **stability**: real and recorded corpora through every builder,
//!   and the report's text, which no hand case carries.
//!
//! The family ages on `CAREER_VERSION` (the plan's call 1), as the storage and career families do.
//!
//! # One definition, two callers
//!
//! `tests/profile.rs` is the family's gate, and from M37 P11 `golf-core rerecord` re-records the
//! family through this same function after the career family, as [`crate::career_family::run_career`]
//! serves the career gate and the re-record. So a re-record writes what the gate checks.
//!
//! # The case, and what is read of it
//!
//! `{career_version, id, input, expected, provenance: {kind, note, oracle, source}}`.
//!
//! - **A hand case**: `input` is `{corpus, bag}` and `expected` is `{profile}`.
//! - **A corpus case**: `input` also carries `display_name`, and `expected` also carries `report`:
//!   the `profile` verb's text for the whole profile (`reports::profile::profile` with no club),
//!   less the verb's trailing blank line, the split `run_career` makes. The report is answered
//!   exactly when `input.display_name` is, so the hand cases, which carry neither, keep their
//!   shape.
//!
//! `career_version` is the gate's to check and the re-record's to move, so the runner does not
//! read it. A profile case carries no `recorded_by` (no script recorded it), so the runner refuses
//! a vector of another family by its `provenance.kind` instead, which every family writes.
//!
//! # An adopted input is another vector's
//!
//! A corpus case's `provenance.source` begins with the career case's path, as a career twin's
//! begins with its storage case's, and [`adopted_from`] reads it. Its [`ADOPTED`] inputs are that
//! career case's, copied whole, and `golf-core rerecord` derives them from the career case *as the
//! run will write it*, so a career input that moves (an adopted storage corpus that moved first)
//! moves the profile case's copy in the same run, or the gate refuses it there. `tests/profile.rs`
//! holds the committed families equal. The chain is storage → career → profile, and the profile
//! case reads the bag and the name from the career case because the storage case has neither.

use serde_json::{json, Map, Value};

use contracts::bag::Bag;
use contracts::career::CareerCorpus;

use crate::compare::compare;
use crate::profile::run_profile;
use crate::reports::profile::profile as profile_report;

/// `provenance.kind` on every profile case.
pub const KIND: &str = "profile";

/// The `input` keys a corpus case copies whole from the career case it adopts ([`adopted_from`]),
/// which are every key it has: the corpus, the bag the career case graded clubs against, and the
/// name the report prints. [M37 P11]
pub const ADOPTED: [&str; 3] = ["bag", "corpus", "display_name"];

/// Where an adopted input's source is named: `provenance.source` begins with the career case's path
/// under the repository, as a career twin's begins with its storage case's.
const ADOPTED_PREFIX: &str = "spec/vectors/";

/// **The runner**: the `GradedProfile` [`run_profile`] answers from one profile case's input, as
/// `{"profile": …}`, and beside it `"report"`, the `profile` verb's text for the whole profile,
/// exactly when the input carries a `display_name` to print (a corpus case; the module doc). `Err`
/// for a vector of another family, an input the shapes refuse, a `display_name` that is not a
/// string, or a profile that fails its own `Validate`.
pub fn run_profile_case(vector: &Value) -> Result<Value, String> {
    let id = vector["id"].as_str().unwrap_or("<a vector with no id>");
    if vector["provenance"]["kind"].as_str() != Some(KIND) {
        return Err(format!(
            "{id}: provenance.kind is {:?}, and the profile family's runner answers only {KIND:?}",
            vector["provenance"]["kind"]
        ));
    }
    let corpus: CareerCorpus = serde_json::from_value(vector["input"]["corpus"].clone())
        .map_err(|e| format!("{id}: input.corpus: {e}"))?;
    let bag: Option<Bag> = serde_json::from_value(vector["input"]["bag"].clone())
        .map_err(|e| format!("{id}: input.bag: {e}"))?;
    let profile = run_profile(&corpus, bag.as_ref())
        .map_err(|e| format!("{id}: the profile run_profile built does not validate: {e}"))?;
    let mut answer = Map::new();
    if let Some(name) = vector["input"].get("display_name") {
        let name = name
            .as_str()
            .ok_or_else(|| format!("{id}: input.display_name is not a string"))?;
        answer.insert(
            "report".to_string(),
            json!(profile_report(&profile, name, None)),
        );
    }
    answer.insert("profile".to_string(), json!(profile));
    Ok(Value::Object(answer))
}

/// Every difference between what a case expects and what [`run_profile_case`] answers, under
/// `docs/CONFORMANCE.md` §3's rules through [`compare`], every path rooted at `expected`: the
/// profile's under `profile.`, and a corpus case's report text at `report`, where a string differs
/// unless it is the same string. §3's `RTOL` is sized for another summation order, so a last bit is
/// not a difference here; the gate holds the bits separately, on the target the cases were worked
/// on.
pub fn differences(expected: &Value, built: &Value) -> Vec<String> {
    compare(expected, built)
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// The id of the career case whose [`ADOPTED`] inputs this profile case's are (the module doc), or
/// `None` for a case whose input was built by hand.
///
/// Read from `provenance.source`, which every corpus case begins with the career case's path, as
/// `career_family::adopted_from` reads a career twin's: `spec/vectors/career/real/aaron.json.gz,
/// input.corpus, input.bag and input.display_name`. So a source that begins with a vector path is
/// one, and the path up to the first `,` or space, less `spec/vectors/` and its suffix, is the
/// career case's id. `Err` for a source that begins with the prefix and names no career case,
/// which is a vector this rule would misread.
pub fn adopted_from(vector: &Value) -> Result<Option<String>, String> {
    let id = vector["id"].as_str().unwrap_or("<a vector with no id>");
    let Some(rest) = vector["provenance"]["source"]
        .as_str()
        .and_then(|source| source.strip_prefix(ADOPTED_PREFIX))
    else {
        return Ok(None);
    };
    let path = rest.split([',', ' ']).next().unwrap_or_default();
    let source = path
        .strip_suffix(".json.gz")
        .or_else(|| path.strip_suffix(".json"))
        .filter(|source| source.starts_with("career/"))
        .ok_or_else(|| {
            format!(
                "{id}: provenance.source names {ADOPTED_PREFIX}{path}, which is not a career \
                 case's file"
            )
        })?;
    Ok(Some(source.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A career vector, or one with no kind, is refused by name rather than read as a profile case
    /// whose corpus happens to parse.
    #[test]
    fn a_vector_of_another_family_is_an_err() {
        for kind in [json!("career"), Value::Null] {
            let vector = json!({
                "id": "career/x",
                "input": {"corpus": {"player_id": "p"}, "bag": null},
                "provenance": {"kind": kind},
            });
            let refused = run_profile_case(&vector).expect_err("not a profile case");
            assert!(
                refused.starts_with("career/x: provenance.kind"),
                "{refused}"
            );
        }
    }

    /// The answer is `expected`'s shape: the profile under its own key, and the report beside it
    /// exactly when the input names who to print it for, so a hand case's answer keys the profile
    /// alone. A name that is not a string is refused rather than printed as `null`.
    #[test]
    fn the_answer_keys_the_profile_and_the_report_only_with_a_name() {
        let mut vector = json!({
            "id": "profile/hand/x",
            "input": {"corpus": {"player_id": "p"}, "bag": null},
            "provenance": {"kind": KIND},
        });
        let keys = |built: &Value| -> Vec<String> {
            built
                .as_object()
                .expect("an object")
                .keys()
                .cloned()
                .collect()
        };
        let built = run_profile_case(&vector).expect("an empty corpus answers");
        assert_eq!(keys(&built), ["profile"]);
        assert_eq!(built["profile"]["player_id"], json!("p"));
        assert!(differences(&built, &built).is_empty());

        vector["input"]["display_name"] = json!("Aaron");
        let named = run_profile_case(&vector).expect("an empty corpus answers");
        assert_eq!(keys(&named), ["profile", "report"]);
        assert_eq!(named["profile"], built["profile"]);
        let report = named["report"].as_str().expect("the report is text");
        assert!(report.contains("Aaron"), "{report}");
        assert_eq!(
            differences(&named, &built),
            ["report: key missing from the port"]
        );

        vector["input"]["display_name"] = json!(7);
        let refused = run_profile_case(&vector).expect_err("a name that is not text");
        assert!(refused.contains("display_name"), "{refused}");
    }

    /// Both spellings a corpus case's source may take, a hand case's prose, and a path that is not a
    /// career case's.
    #[test]
    fn an_adopted_input_names_its_career_case() {
        let sourced =
            |source: &str| json!({"id": "profile/corpus/x", "provenance": {"source": source}});
        assert_eq!(
            adopted_from(&sourced(
                "spec/vectors/career/real/aaron.json.gz, input.corpus, input.bag and \
                 input.display_name"
            )),
            Ok(Some("career/real/aaron".to_string()))
        );
        assert_eq!(
            adopted_from(&sourced(
                "spec/vectors/career/synthetic/storage-every-source.json input"
            )),
            Ok(Some("career/synthetic/storage-every-source".to_string()))
        );
        assert_eq!(
            adopted_from(&sourced("Worked by hand from the M37 plan's decisions")),
            Ok(None)
        );
        assert_eq!(adopted_from(&json!({"provenance": {}})), Ok(None));
        for wrong in [
            "spec/vectors/storage/corpus/real.json.gz, expected.corpus",
            "spec/vectors/profile/hand/x.json",
            "spec/vectors/career/real/aaron.txt",
        ] {
            let refused = adopted_from(&sourced(wrong)).expect_err(wrong);
            assert!(refused.contains("not a career case"), "{refused}");
        }
    }
}
