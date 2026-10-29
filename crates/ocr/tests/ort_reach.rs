//! `ort` stays in `crates/ocr` (M29 P1; the M29 plan's D13 and its rule on cargo edges).
//!
//! ONNX Runtime is a native library whose build downloads it, and M38's phone links `contracts`,
//! `pyfmt`, `analysis`, `feedback`, `screen` and `storage` through `core`. So none of those seven
//! may reach `ort`, by any path of shipped dependencies: normal and build, never dev, because a
//! dev-dependency is never linked into what ships (`screen`'s on `golf-core` is one).
//!
//! The walk reads every `crates/*/Cargo.toml` by hand rather than asking `cargo metadata`: the
//! manifests here are simple, a TOML crate would be a dependency for one test, and spawning cargo
//! from inside `cargo test` is a lock to reason about for no gain. What it does not handle (an
//! inline table spread over lines, `ort` arriving through a crates.io dependency) does not occur,
//! and the positive controls at the bottom fail if the parse ever stops seeing the edges it is
//! meant to.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

/// The crates on the phone's path, by directory under `crates/`.
const GUARDED: [&str; 7] = [
    "contracts",
    "pyfmt",
    "analysis",
    "feedback",
    "screen",
    "storage",
    "core",
];

/// What may not be reached. `ort-sys` too, in case anything ever takes the raw bindings.
const FORBIDDEN: [&str; 2] = ["ort", "ort-sys"];

/// One crate's shipped dependencies, as its manifest declares them.
#[derive(Default)]
struct Declared {
    /// Every dependency's package name.
    packages: BTreeSet<String>,
    /// The `crates/` directory of each path dependency.
    paths: BTreeSet<String>,
}

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every crate in `crates/`, by directory.
fn workspace() -> BTreeMap<String, Declared> {
    let root = fs::read_to_string(crates_dir().join("..").join("Cargo.toml")).unwrap();
    let renames = workspace_renames(&root);
    let mut crates = BTreeMap::new();
    for entry in fs::read_dir(crates_dir()).unwrap() {
        let dir = entry.unwrap().path();
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            let name = dir.file_name().unwrap().to_string_lossy().into_owned();
            crates.insert(
                name,
                declared(&fs::read_to_string(manifest).unwrap(), &renames),
            );
        }
    }
    crates
}

/// `[workspace.dependencies]` keys to package names, so `foo.workspace = true` resolves to what
/// the root says `foo` is, `package = "..."` renames included.
fn workspace_renames(root: &str) -> BTreeMap<String, String> {
    let mut renames = BTreeMap::new();
    for (section, key, text) in entries(root) {
        if section == "workspace.dependencies" {
            let package = string_value(&text, "package").unwrap_or_else(|| key.clone());
            renames.insert(key, package);
        }
    }
    renames
}

fn declared(manifest: &str, renames: &BTreeMap<String, String>) -> Declared {
    let mut declared = Declared::default();
    for (section, key, text) in entries(manifest) {
        let Some((ships, table_name)) = dependency_section(&section) else {
            continue;
        };
        if !ships {
            continue;
        }
        // `[dependencies.foo]` names its dependency in the header, and its keys are its fields.
        let key = table_name.unwrap_or(key);
        let (key, inherited) = match key.strip_suffix(".workspace") {
            Some(base) => (base.to_owned(), true),
            None => (key, text.contains("workspace = true")),
        };
        let package = match string_value(&text, "package") {
            Some(package) => package,
            None if inherited => renames.get(&key).cloned().unwrap_or(key),
            None => key,
        };
        declared.packages.insert(package);
        if let Some(path) = string_value(&text, "path") {
            let dir = path
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap()
                .to_owned();
            declared.paths.insert(dir);
        }
    }
    declared
}

/// `(section, key, text)` for every top-level key, where `text` is the key's line after the `=`
/// plus any continuation lines (a multi-line array). A `[dependencies.foo]` table yields one entry
/// holding all its lines, with an empty key.
fn entries(manifest: &str) -> Vec<(String, String, String)> {
    let mut entries: Vec<(String, String, String)> = Vec::new();
    let mut section = String::new();
    let mut in_table = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let at_margin = !line.starts_with(char::is_whitespace);
        if at_margin && trimmed.starts_with('[') {
            section = trimmed
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_owned();
            in_table = matches!(dependency_section(&section), Some((_, Some(_))));
            if in_table {
                entries.push((section.clone(), String::new(), String::new()));
            }
            continue;
        }
        match line.split_once('=') {
            Some((key, rest)) if at_margin && !in_table && !key.contains(['"', '{', '[']) => {
                entries.push((section.clone(), key.trim().to_owned(), rest.to_owned()));
            }
            _ => {
                if let Some(last) = entries.last_mut() {
                    last.2.push('\n');
                    last.2.push_str(line);
                }
            }
        }
    }
    entries
}

/// Whether a section declares dependencies, whether they ship, and the dependency a
/// `[dependencies.foo]` header names. `None` for every other section.
fn dependency_section(section: &str) -> Option<(bool, Option<String>)> {
    let rest = match section.strip_prefix("target.") {
        // Skip the cfg: quoted (`'cfg(windows)'`, which may hold dots) or a bare triple.
        Some(target) => match target.chars().next() {
            Some(quote @ ('\'' | '"')) => {
                let close = target[1..].find(quote)? + 1;
                target[close + 1..].strip_prefix('.')?
            }
            _ => target.split_once('.')?.1,
        },
        None => section,
    };
    let (kind, name) = match rest.split_once('.') {
        Some((kind, name)) => (kind, Some(name.to_owned())),
        None => (rest, None),
    };
    match kind {
        "dependencies" | "build-dependencies" => Some((true, name)),
        "dev-dependencies" => Some((false, name)),
        _ => None,
    }
}

/// The string assigned to `key` in `text` (`key = "value"`), if any.
fn string_value(text: &str, key: &str) -> Option<String> {
    let mut search = text;
    while let Some(at) = search.find(key) {
        let before = search[..at].chars().next_back();
        let after = search[at + key.len()..].trim_start();
        let standalone = !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-');
        if let (true, Some(value)) = (standalone, after.strip_prefix('=')) {
            let value = value.trim_start().strip_prefix('"')?;
            return Some(value[..value.find('"')?].to_owned());
        }
        search = &search[at + key.len()..];
    }
    None
}

/// Every crate `start` reaches through shipped path dependencies, itself included, each with the
/// chain that reached it.
fn reach(crates: &BTreeMap<String, Declared>, start: &str) -> BTreeMap<String, Vec<String>> {
    let mut seen = BTreeMap::from([(start.to_owned(), vec![start.to_owned()])]);
    let mut queue = VecDeque::from([start.to_owned()]);
    while let Some(name) = queue.pop_front() {
        let chain = seen[&name].clone();
        for next in &crates[&name].paths {
            if !seen.contains_key(next) {
                let mut longer = chain.clone();
                longer.push(next.clone());
                seen.insert(next.clone(), longer);
                queue.push_back(next.clone());
            }
        }
    }
    seen
}

#[test]
fn no_crate_on_the_phone_path_reaches_ort() {
    let crates = workspace();
    let mut violations = Vec::new();
    for guarded in GUARDED {
        assert!(
            crates.contains_key(guarded),
            "crates/{guarded} is gone; update GUARDED"
        );
        for (name, chain) in reach(&crates, guarded) {
            for forbidden in FORBIDDEN {
                if crates[&name].packages.contains(forbidden) {
                    violations.push(format!("{} -> {forbidden}", chain.join(" -> ")));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "ort is reachable from the phone's path:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ocr_is_the_only_crate_that_declares_ort() {
    let crates = workspace();
    let declaring: Vec<&str> = crates
        .iter()
        .filter(|(_, declared)| FORBIDDEN.iter().any(|f| declared.packages.contains(*f)))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(declaring, ["ocr"]);
}

#[test]
fn the_walk_sees_the_edges_it_is_meant_to() {
    // Positive controls, so the two tests above cannot pass because the parse saw nothing: `ort`'s
    // own entry spans several lines, `core` reaches the engine and the screen reader through path
    // dependencies, a `.workspace = true` key resolves, and `screen`'s dev-dependency on `golf-core`
    // is not followed.
    let crates = workspace();
    assert!(crates["ocr"].packages.contains("ort"));
    assert!(crates["ocr"].packages.contains("serde_json"));
    let from_core = reach(&crates, "core");
    for reached in [
        "analysis",
        "feedback",
        "screen",
        "storage",
        "contracts",
        "pyfmt",
    ] {
        assert!(
            from_core.contains_key(reached),
            "core does not reach {reached}"
        );
    }
    assert!(
        !reach(&crates, "screen").contains_key("core"),
        "a dev-dependency was followed"
    );
}
