//! `golf-core` — the CLI end of `docs/CONFORMANCE.md` §4's cross-language seam. [M22 P6]
//!
//! ```text
//! golf-core run < vector.json          # vector in, serialized result out
//! ```
//!
//! The counterpart of `python scripts/conformance.py run < vector.json`, and the two are meant to be
//! diffed against each other with no Python in the loop but the reference:
//!
//! ```text
//! diff <(python scripts/conformance.py run < v.json) <(cargo run -q --bin golf-core -- run < v.json)
//! ```
//!
//! **That diff is not the gate**, and the difference matters. A byte comparison of the two is
//! unavailable in either direction — Python writes `-1.636758133827243e-05` where `serde_json` writes
//! `-0.00001636758133827243` for the same f64, and seventy-seven distinct floats in the committed
//! vectors take the exponent form (M22 P2). So the diff above shows *where* two implementations
//! disagree once you have already decided they do; `crates/core/tests/engine.rs` is what decides,
//! under §3's structural rules.
//!
//! # Why a subcommand at all, for one verb
//!
//! Because `conformance.py` has five and a port is asked to match one of them. Spelling `run`
//! explicitly is what makes `golf-core check` and `golf-core list` additions rather than a redesign —
//! and ADR-032 §7 already names a `golf-core` subprocess seam as a candidate for how ADR-022's
//! fitting scripts reach a measurement once `analysis/measure.py` is gone, which would be a second
//! verb here.
//!
//! `regenerate` is deliberately not a candidate: the vectors are the oracle and a port that can
//! rewrite them is a port that passes by construction, which is the whole argument
//! `conformance_vectors._audio` makes about the family M20 deleted.

use std::io::{self, Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("run") if args.next().is_none() => run(),
        Some(other) => usage(&format!("unknown command {other:?}")),
        None => usage("no command given"),
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("golf-core: {problem}");
    eprintln!("usage: golf-core run < vector.json");
    ExitCode::FAILURE
}

/// Read one vector from stdin, run it, write the serialized result to stdout.
///
/// **A read to EOF rather than a line-oriented parse**, because a committed vector is pretty-printed
/// across thousands of lines. And one vector per invocation rather than a stream, because that is the
/// contract `conformance.py run` has and a batching mode nobody asked for would make the two
/// commands' stdin mean different things.
fn run() -> ExitCode {
    let mut text = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut text) {
        eprintln!("golf-core: reading stdin: {e}");
        return ExitCode::FAILURE;
    }

    // Accept either a whole vector file or a bare `input` object, the way a caller piping a slice of
    // one would expect. `conformance.py run` takes the whole file; the bare form costs one fallback
    // and saves anyone diffing a hand-built input from having to wrap it.
    let input = match serde_json::from_str::<golf_core::Vector>(&text) {
        Ok(vector) => vector.input,
        Err(whole) => match serde_json::from_str::<golf_core::VectorInput>(&text) {
            Ok(input) => input,
            Err(bare) => {
                eprintln!("golf-core: stdin is neither a vector nor a vector input");
                eprintln!("  as a vector:       {whole}");
                eprintln!("  as a vector input: {bare}");
                return ExitCode::FAILURE;
            }
        },
    };

    let result = golf_core::run(&input);
    // Pretty-printed, because the thing on the other side of this pipe is usually a human reading a
    // diff. `compare_results` parses either way.
    let mut out = io::stdout().lock();
    let written = serde_json::to_writer_pretty(&mut out, &result)
        .map_err(|e| e.to_string())
        .and_then(|()| {
            out.write_all(b"\n")
                .and_then(|()| out.flush())
                .map_err(|e| e.to_string())
        });
    match written {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // A broken pipe lands here too, which is what `golf-core run < v.json | head` does — so
            // the message names the write rather than claiming the analysis failed.
            eprintln!("golf-core: writing stdout: {e}");
            ExitCode::FAILURE
        }
    }
}
