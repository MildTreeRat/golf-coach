//! The documents state the real MCP tool count. [M29 P15, moved from `tests/test_docs_truth.py`]
//!
//! M9 P18 added two MCP tools and left six sentences saying eight, and Python's pin
//! (`test_the_docs_state_the_real_mcp_tool_count`, M9 P20) was the answer. It counted
//! `TOOL_DESCRIPTIONS`, frozen Python's eleven; the server is `golf-mcp` now, whose table holds a
//! twelfth Python never gains, so the pin moved here with its count read off that table (the plan's
//! C9) and the Python one was removed in the same change.
//!
//! **Pinned on one phrase shape**, "the same N tools", rather than on every "N tools" in the repo,
//! because a subset count is a legitimate thing to write (`club.rs` really does hold two). What is
//! not legitimate is a sentence claiming to describe the *whole* surface with a stale number, and
//! that sentence has one recognisable form wherever it appears. The documents read are Python's
//! list unchanged: `docs/ARCHITECTURE.md`, `ROADMAP.md`, `scripts/ask_swing.py` and every ADR.
//! `WORKLOG.md` is deliberately not read: it is append-only history, and a dated record of what a
//! handshake advertised in August is not a claim about today.
//!
//! Whitespace is collapsed first, as Python's `_flat` collapses it, so a phrase broken across a
//! wrapped line still matches.

use std::fs;
use std::path::{Path, PathBuf};

use golf_mcp::caveats::count_word;
use golf_mcp::tools::TOOLS;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every word `w` in `text` that stands in "the same w tools", whitespace collapsed: Python's
/// `the same (\w+) tools`, so "tools" may run on into punctuation that ends the sentence.
fn same_n_tools(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    words
        .windows(4)
        .filter(|w| w[0] == "the" && w[1] == "same" && w[3].starts_with("tools"))
        .filter(|w| w[2].chars().all(|c| c.is_alphanumeric() || c == '_'))
        .map(|w| w[2].to_string())
        .collect()
}

#[test]
fn the_phrase_is_found_across_a_wrapped_line_and_not_in_a_subset_count() {
    assert_eq!(
        same_n_tools("looks up through the same\n   twelve tools, over stdio"),
        ["twelve"]
    );
    assert!(same_n_tools("`club.rs` holds two tools").is_empty());
    assert_eq!(
        same_n_tools("through the same eleven tools. Next"),
        ["eleven"]
    );
}

#[test]
fn the_docs_state_the_real_mcp_tool_count() {
    let expected = count_word(TOOLS.len());
    let decisions = repo().join("docs/decisions");
    let mut adrs: Vec<PathBuf> = fs::read_dir(&decisions)
        .expect("docs/decisions/")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "md"))
        .collect();
    adrs.sort();
    let mut sources: Vec<PathBuf> = ["docs/ARCHITECTURE.md", "ROADMAP.md", "scripts/ask_swing.py"]
        .iter()
        .map(|relative| repo().join(relative))
        .collect();
    sources.extend(adrs);

    let shown = |path: &Path| {
        path.strip_prefix(repo())
            .unwrap_or(path)
            .display()
            .to_string()
    };
    let mut found = Vec::new();
    let mut wrong = Vec::new();
    for path in &sources {
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for word in same_n_tools(&text) {
            if word != expected {
                wrong.push(format!(
                    "{} says 'the same {word} tools'; golf-mcp's table holds {}",
                    shown(path),
                    TOOLS.len()
                ));
            }
            found.push(shown(path));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(
        !found.is_empty(),
        "no document states the MCP tool count in the pinned form any more: either the phrasing \
         moved, in which case reword this test with it, or the claim was dropped and this pin is \
         now guarding nothing"
    );
}
