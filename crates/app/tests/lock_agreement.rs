//! The bridge's lock and the root's agree on the crates that decide bytes. [M38 P16]
//!
//! `app/rust` is outside this workspace on purpose (the plan's C1), so it has a lock of its own, and
//! nothing but this test says that lock compiles the facade against the same crates `cargo test`
//! does. The first resolve showed why it is needed: FRB's template lock left the bridge on `serde`
//! 1.0.193 and `serde_json` 1.0.109 against the root's 1.0.229 and 1.0.151, so what `cargo test`
//! certified and what the phone wrote would have come from two JSON writers.
//!
//! No TOML dependency, because a test's dependency is a line in the root lock this is holding still,
//! and a lock's shape is one `name` then one `version` per `[[package]]`.

use std::path::Path;

/// Each writes or hashes what lands on disk, and so what an export carries to the lab: JSON's text
/// (`serde`, `serde_json`), a photo's digest (`sha2`), the archive (`flate2`, `zip`) and a name's
/// normal form (`unicode-normalization`).
const DECIDE_BYTES: [&str; 6] = [
    "serde",
    "serde_json",
    "sha2",
    "flate2",
    "zip",
    "unicode-normalization",
];

/// Every version a lock holds of one crate.
fn versions_of(lock: &str, name: &str) -> Vec<String> {
    let name_line = format!("name = \"{name}\"");
    let mut lines = lock.lines();
    let mut versions = Vec::new();
    while let Some(line) = lines.next() {
        if line == name_line {
            let version = lines
                .next()
                .and_then(|next| next.strip_prefix("version = \""))
                .and_then(|rest| rest.strip_suffix('"'))
                .unwrap_or_else(|| panic!("{name}'s `name` line is not followed by its `version`"));
            versions.push(version.to_string());
        }
    }
    versions
}

/// One sentence per way the two locks fail to hold one version of a crate between them. A crate
/// missing from a lock is one of those ways: the pin on it would otherwise pass while holding nothing.
fn disagreements(root: &str, bridge: &str) -> Vec<String> {
    let mut found = Vec::new();
    for name in DECIDE_BYTES {
        let in_root = versions_of(root, name);
        let in_bridge = versions_of(bridge, name);
        match (in_root.as_slice(), in_bridge.as_slice()) {
            ([one], [other]) if one == other => {}
            ([one], [other]) => found.push(format!(
                "{name}: the root holds {one} and the bridge {other}; \
                 run `cargo update -p {name} --precise {one}` in app/rust"
            )),
            _ => found.push(format!(
                "{name}: wanted one version in each lock, and the root holds {in_root:?} \
                 and the bridge {in_bridge:?}"
            )),
        }
    }
    found
}

fn read_lock(from_repo: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(from_repo);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn the_two_locks_hold_one_version_each_of_what_decides_bytes() {
    let found = disagreements(&read_lock("Cargo.lock"), &read_lock("app/rust/Cargo.lock"));
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// `ort_reach` walks this workspace's graph, and the bridge is outside it, so this is the check on
/// what the bridge resolved. The facade's dev edge to the lab reaches `ort`, and cargo leaves a path
/// dependency's dev edges out of another workspace's lock, which is the reason this holds.
#[test]
fn the_bridge_lock_holds_no_ort() {
    let bridge = read_lock("app/rust/Cargo.lock");
    assert_eq!(versions_of(&bridge, "ort"), Vec::<String>::new());
}

/// Six crates at one version each, so the controls below change one thing at a time.
fn agreeing_lock() -> String {
    DECIDE_BYTES
        .iter()
        .map(|name| format!("[[package]]\nname = \"{name}\"\nversion = \"1.0.0\"\n"))
        .collect()
}

#[test]
fn locks_that_agree_have_no_disagreement() {
    assert_eq!(
        disagreements(&agreeing_lock(), &agreeing_lock()),
        Vec::<String>::new()
    );
}

#[test]
fn another_version_in_the_bridge_is_named_with_its_fix() {
    let bridge = agreeing_lock().replace(
        "name = \"serde_json\"\nversion = \"1.0.0\"",
        "name = \"serde_json\"\nversion = \"1.0.9\"",
    );
    assert_eq!(
        disagreements(&agreeing_lock(), &bridge),
        vec!["serde_json: the root holds 1.0.0 and the bridge 1.0.9; \
             run `cargo update -p serde_json --precise 1.0.0` in app/rust"
            .to_string()]
    );
}

#[test]
fn two_versions_in_one_lock_is_a_disagreement() {
    let bridge = format!(
        "{}[[package]]\nname = \"zip\"\nversion = \"2.0.0\"\n",
        agreeing_lock()
    );
    let found = disagreements(&agreeing_lock(), &bridge);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("zip: wanted one version"), "{found:?}");
}

#[test]
fn a_crate_missing_from_a_lock_is_a_disagreement_and_not_a_pass() {
    let bridge = agreeing_lock().replace("name = \"sha2\"", "name = \"sha3\"");
    let found = disagreements(&agreeing_lock(), &bridge);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("sha2: wanted one version"),
        "{found:?}"
    );
}

/// `serde` must not count `serde_json`'s lines, nor `zip` a `zipper`: the match is the whole name.
#[test]
fn a_name_that_only_starts_the_same_is_another_crate() {
    let lock = "[[package]]\nname = \"serde_json\"\nversion = \"9.9.9\"\n";
    assert_eq!(versions_of(lock, "serde"), Vec::<String>::new());
}
