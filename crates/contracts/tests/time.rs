//! `contracts::time::Timestamp` against `spec/vectors/format/timestamp.json`. [M36 P4]
//!
//! The format family's one table whose answer is pydantic's rather than the interpreter's, and the
//! one `crates/pyfmt/tests/format.rs` lists as this crate's (`provenance.implemented_by`). Each row
//! is a string and what pydantic made of it: refused, or read and then written back, with the
//! instant, CPython's `isoformat()` and `%Y-%m-%d` in the value's own offset. Compared exactly — a
//! spelling is the same spelling or it is not.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use contracts::time::Timestamp;
use serde::Deserialize;

#[derive(Deserialize)]
struct Table {
    id: String,
    provenance: Provenance,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Provenance {
    implemented_by: String,
    pydantic_version: String,
}

#[derive(Deserialize)]
struct Case {
    kind: String,
    value: String,
    pydantic: Option<String>,
    isoformat: Option<String>,
    date: Option<String>,
    epoch_us: Option<i64>,
}

fn table() -> Table {
    let path: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/format/timestamp.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

/// Every row: refused where pydantic refuses, and otherwise read to the same instant and written
/// back in pydantic's spelling, through `parse` and through serde alike.
#[test]
fn every_row_reads_and_writes_as_pydantic_does() {
    let table = table();
    assert_eq!(table.provenance.implemented_by, "contracts");
    let mut kinds = [0usize; 3];
    for case in &table.cases {
        let parsed = Timestamp::parse(&case.value);
        let Some(pydantic) = &case.pydantic else {
            assert_eq!(case.kind, "refused", "{:?}", case.value);
            assert!(
                parsed.is_err(),
                "{:?} reads as {parsed:?}, and pydantic refuses it",
                case.value
            );
            assert!(serde_json::from_value::<Timestamp>(serde_json::json!(case.value)).is_err());
            kinds[2] += 1;
            continue;
        };
        let parsed = parsed.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            &parsed.to_string(),
            pydantic,
            "{:?} written back",
            case.value
        );
        assert_eq!(
            Some(parsed.utc_micros()),
            case.epoch_us,
            "{:?}'s instant",
            case.value
        );
        assert_eq!(
            Some(parsed.isoformat()),
            case.isoformat,
            "{:?}.isoformat()",
            case.value
        );
        if let Some(date) = &case.date {
            assert_eq!(&parsed.date_ymd(), date, "{:?} as %Y-%m-%d", case.value);
        }
        // What pydantic wrote reads back to itself, so a stored value round-trips byte for byte.
        let again = Timestamp::parse(pydantic).expect("pydantic's own spelling reads");
        assert_eq!(&again.to_string(), pydantic);
        assert_eq!(again.offset_minutes(), parsed.offset_minutes());
        // And serde is the same two calls, as a JSON string.
        let through_serde: Timestamp =
            serde_json::from_value(serde_json::json!(case.value)).expect("serde reads it");
        assert_eq!(
            serde_json::to_value(through_serde).unwrap(),
            serde_json::json!(pydantic)
        );
        match case.kind.as_str() {
            "written" => {
                assert_eq!(&case.value, pydantic);
                kinds[0] += 1;
            }
            "respelled" => {
                assert_ne!(&case.value, pydantic);
                kinds[1] += 1;
            }
            other => panic!("{:?} is a {other:?} row with an answer", case.value),
        }
    }
    assert!(
        kinds.iter().all(|&n| n > 0),
        "every kind of row is present: {kinds:?}"
    );
    println!(
        "{}: {} written, {} respelled, {} refused (pydantic {})",
        table.id, kinds[0], kinds[1], kinds[2], table.provenance.pydantic_version
    );
}

/// Equality, order and hash are the instant's, as Python's are for two aware datetimes: every pair
/// of rows compares as their `epoch_us` do, so `12:00Z` equals `17:30+05:30` and hashes alike.
#[test]
fn equality_order_and_hash_are_the_instants() {
    let rows: Vec<(Timestamp, i64)> = table()
        .cases
        .iter()
        .filter_map(|case| Some((Timestamp::parse(&case.value).ok()?, case.epoch_us?)))
        .collect();
    let hash = |t: &Timestamp| {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        hasher.finish()
    };
    let mut equal_across_offsets = 0;
    for (a, a_us) in &rows {
        for (b, b_us) in &rows {
            assert_eq!(a.cmp(b), a_us.cmp(b_us), "{a:?} against {b:?}");
            assert_eq!(a == b, a_us == b_us, "{a:?} == {b:?}");
            if a == b {
                assert_eq!(hash(a), hash(b), "{a:?} and {b:?} hash apart");
                if a.offset_minutes() != b.offset_minutes() {
                    equal_across_offsets += 1;
                }
            }
        }
    }
    assert!(
        equal_across_offsets > 0,
        "no two rows share an instant in different offsets, so equality-by-instant is untested"
    );
}

/// What pydantic reads and this refuses — the M36 plan's call 9, named rather than left to be
/// found. Each comment is pydantic 2.13's answer, measured when this was written; the format table
/// holds only what the two agree on, so these are pinned here instead.
#[test]
fn the_lax_spellings_pydantic_reads_are_refused() {
    for (text, pydantic_reads_it_as) in [
        (
            "2026-08-04T12:00:00",
            "naive 2026-08-04T12:00:00, with no offset at all",
        ),
        ("2026-08-04", "naive midnight"),
        ("1700000000", "2023-11-14T22:13:20Z, a Unix number"),
        (
            "2026-08-04 12:00:00Z",
            "2026-08-04T12:00:00Z, a space for the T",
        ),
        ("2026-08-04t12:00:00z", "2026-08-04T12:00:00Z, lowercase"),
        (
            "2026-08-04T12:00:00+0530",
            "2026-08-04T12:00:00+05:30, no colon",
        ),
        ("2026-08-04T12:00Z", "2026-08-04T12:00:00Z, no seconds"),
        (
            "2026-08-04T12:00:00.1234567Z",
            "2026-08-04T12:00:00.123456Z, truncated",
        ),
        (
            "2026-08-04T12:00:00,5Z",
            "2026-08-04T12:00:00.500000Z, a comma",
        ),
    ] {
        assert!(
            Timestamp::parse(text).is_err(),
            "{text:?} reads, where only pydantic should ({pydantic_reads_it_as})"
        );
    }
}

/// The clock: UTC, written with `Z`, and no finer than a microsecond, so what a store stamps
/// spells as pydantic would have stamped it.
#[test]
fn now_is_utc_at_microsecond_resolution() {
    let before = Timestamp::now_utc();
    let now = Timestamp::now_utc();
    assert_eq!(now.offset_minutes(), 0);
    assert!(now >= before);
    let written = now.to_string();
    assert!(written.ends_with('Z'), "{written}");
    assert_eq!(
        Timestamp::parse(&written).unwrap().utc_micros(),
        now.utc_micros()
    );
}

/// `Debug` shows the spelling, which is what a failing assertion on a contract needs to show.
#[test]
fn debug_shows_the_lexeme() {
    let t: Timestamp = "2026-08-20T23:30:00-05:00".parse().unwrap();
    assert_eq!(format!("{t:?}"), "Timestamp(\"2026-08-20T23:30:00-05:00\")");
    assert_eq!(t.date_ymd(), "2026-08-20");
    assert_eq!(t.isoformat(), "2026-08-20T23:30:00-05:00");
}
