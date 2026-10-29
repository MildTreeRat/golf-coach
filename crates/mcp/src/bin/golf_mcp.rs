//! `golf-mcp`: serve the golf-coach MCP tools over stdio. `scripts/run_mcp_server.py`. [M29 P9]
//!
//! ```text
//! golf-mcp [--sessions-dir D] [--shots-dir D] [--golfers-dir D | --no-registry]
//! golf-mcp answer-case < CASE.json    # one spec/vectors/mcp tool case answered, for the recorder
//! ```
//!
//! # `answer-case` [M29 P15]
//!
//! The family's runner behind stdin and stdout (`golf_mcp::family::answer_case`): one tool case
//! of `spec/vectors/mcp/` in, `{answers, recorded}` out as one line of JSON, the server's answer to
//! every call and the committed answers as this server answers them on purpose. It is how
//! `golf-core rerecord` walks the family under a career declaration, because nothing may depend on
//! this crate and so the recorder runs the binary; and it recorded `rust/`'s cases once, a case
//! with no `expected` answering `recorded: null`. Exit 0 with the answer, 2 with nothing on stdout
//! for input that is not a case. It reads no directory of its own: the tree is the case's, laid out
//! in a scratch directory and removed.
//!
//! **Speaks stdio, not a port**: the client launches this process and talks over the pipe, so a
//! registration is the binary's path and nothing to point a browser at. Read-only: nothing here
//! analyses a swing or imports a shot.
//!
//! The flags are the script's (the M29 plan's C10), resolved as `golf-core`'s career verbs resolve
//! theirs (`golf_core::reports::data_dir`): the flag, else `GOLF_SESSIONS_DIR`, `GOLF_SHOTS_DIR` or
//! `GOLF_GOLFERS_DIR`, else `data/processed/{sessions,shots,golfers}` under the repository.
//! `--no-registry` has no twin flag: the script always passes a registry, and the registry-less
//! shape existed only as `build_server(golfers_dir=None)`, which the recorded surface and five tool
//! cases were built as (the plan's P8 finding 6). A missing directory is a warning, not a failure,
//! as the script's is: it is an empty dataset the tools answer perfectly well, and a client shows a
//! server that exited as broken, which is a worse diagnosis of the same thing.
//!
//! Everything printed goes to stderr, because stdout is the protocol.

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use golf_core::reports::data_dir;
use golf_mcp::family;
use golf_mcp::server::{Dirs, GolfMcp};
use rmcp::ServiceExt;

const USAGE: &str = "usage: golf-mcp [--sessions-dir D] [--shots-dir D] [--golfers-dir D | \
                     --no-registry]\n       golf-mcp answer-case < CASE.json";

/// `answer-case`: one case on stdin, `family::answer_case`'s value on stdout.
fn answer_case() -> ExitCode {
    let mut text = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut text) {
        eprintln!("golf-mcp answer-case: reading stdin: {error}");
        return ExitCode::from(2);
    }
    let answered = serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|e| format!("stdin is not JSON: {e}"))
        .and_then(|case| family::answer_case(&case));
    match answered {
        Ok(answer) => {
            println!("{answer}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("golf-mcp answer-case: {why}");
            ExitCode::from(2)
        }
    }
}

/// The parsed command line, or the sentence that refuses it.
fn parse(args: &[String], env: impl Fn(&str) -> Option<String>) -> Result<Option<Dirs>, String> {
    let (mut sessions, mut shots, mut golfers) = (None, None, None);
    let mut no_registry = false;
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .map(PathBuf::from)
                .ok_or_else(|| format!("{name} needs a directory"))
        };
        match flag.as_str() {
            "-h" | "--help" => return Ok(None),
            "--sessions-dir" => sessions = Some(value("--sessions-dir")?),
            "--shots-dir" => shots = Some(value("--shots-dir")?),
            "--golfers-dir" => golfers = Some(value("--golfers-dir")?),
            "--no-registry" => no_registry = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if no_registry && golfers.is_some() {
        return Err("--golfers-dir and --no-registry name opposite servers".to_string());
    }
    Ok(Some(Dirs {
        sessions: data_dir(sessions, "GOLF_SESSIONS_DIR", "sessions", &env),
        shots: data_dir(shots, "GOLF_SHOTS_DIR", "shots", &env),
        golfers: (!no_registry).then(|| data_dir(golfers, "GOLF_GOLFERS_DIR", "golfers", &env)),
    }))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("answer-case") {
        if args.len() > 1 {
            eprintln!("golf-mcp answer-case takes no argument: the case is stdin\n{USAGE}");
            return ExitCode::from(2);
        }
        return answer_case();
    }
    let dirs = match parse(&args, |var| std::env::var(var).ok()) {
        Ok(Some(dirs)) => dirs,
        Ok(None) => {
            eprintln!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(why) => {
            eprintln!("golf-mcp: {why}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let mut named = vec![("sessions", &dirs.sessions), ("shots", &dirs.shots)];
    if let Some(golfers) = &dirs.golfers {
        named.push(("golfers", golfers));
    }
    for (label, path) in &named {
        if !path.exists() {
            eprintln!(
                "warning: {label} directory does not exist: {}",
                path.display()
            );
        }
    }
    let golfers = dirs
        .golfers
        .as_ref()
        .map_or_else(|| "none".to_string(), |p| p.display().to_string());
    eprintln!(
        "golf-coach MCP server on stdio — sessions={} shots={} golfers={golfers}",
        dirs.sessions.display(),
        dirs.shots.display(),
    );
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("golf-mcp: no async runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    let served = runtime.block_on(async {
        let service = GolfMcp::new(dirs).serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    });
    match served {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("golf-mcp: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn the_flags_win_then_the_environment_then_the_repository() {
        let env = |var: &str| (var == "GOLF_SHOTS_DIR").then(|| "from-env".to_string());
        let dirs = parse(&args("--sessions-dir s"), env).unwrap().unwrap();
        assert_eq!(dirs.sessions, PathBuf::from("s"));
        assert_eq!(dirs.shots, PathBuf::from("from-env"));
        let golfers = dirs.golfers.unwrap();
        assert!(
            golfers.ends_with("data/processed/golfers"),
            "{}",
            golfers.display()
        );
    }

    #[test]
    fn no_registry_is_the_registry_less_server_and_refuses_a_registry() {
        let dirs = parse(&args("--no-registry"), |_| None).unwrap().unwrap();
        assert_eq!(dirs.golfers, None);
        assert!(parse(&args("--no-registry --golfers-dir g"), |_| None).is_err());
        assert!(parse(&args("--shots-dir"), |_| None).is_err());
        assert!(parse(&args("--bogus"), |_| None).is_err());
        assert_eq!(parse(&args("--help"), |_| None), Ok(None));
    }
}
