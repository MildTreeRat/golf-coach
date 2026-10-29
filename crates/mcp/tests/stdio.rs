//! The binary is a server: `golf-mcp` spawned as a client spawns it, spoken to over its stdin and
//! stdout. [M29 P15]
//!
//! `tests/surface.rs` connects `rmcp`'s own client over an in-memory pipe, which proves the handler
//! and not the process. This spawns the built binary with `--sessions-dir`, `--shots-dir` and
//! `--golfers-dir` over a recorded case's tree and speaks the protocol by hand, one JSON-RPC message
//! per line, as MCP's stdio transport frames it: what Claude Code and P16's Python client will do.
//! Hand-written rather than through `rmcp`'s child-process transport, which is a feature this
//! workspace does not enable and would bring a crate of its own, and because a client that is not
//! `rmcp`'s is the better check that the framing is the protocol's and not one SDK's habit.
//!
//! One session, in the order a client goes: initialize (asking for Python's SDK's revision), the
//! initialized notification, `tools/list` (twelve, the strike profile last), one call answered, the
//! caveats resource read, a miss answered as a result rather than a protocol error, and stdin
//! closed, on which the server exits 0. Every read waits at most [`PATIENCE`], so a server that
//! hangs fails the test rather than hanging it.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use golf_mcp::family::{calls, materialise, Scratch};
use golf_mcp::server::CAVEATS_URI;
use golf_mcp::tools::TOOLS;
use serde_json::{json, Value};

/// How long one answer may take. A debug build answers every recorded call in about two seconds
/// all told (M29 P15), so this is a hang detector, not a performance claim.
const PATIENCE: Duration = Duration::from_secs(60);

/// A recorded case with a registry, whose tree the server is started over.
const CASE: &str = "tools/career-speaking.json";

struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

impl Session {
    fn send(&mut self, message: Value) {
        let stdin = self.stdin.as_mut().expect("stdin is open");
        writeln!(stdin, "{message}").expect("write a message");
        stdin.flush().expect("flush");
    }

    /// The response to request `id`, skipping any notification the server sends first.
    fn answer(&mut self, id: i64) -> Value {
        loop {
            let line = self
                .lines
                .recv_timeout(PATIENCE)
                .unwrap_or_else(|e| panic!("no answer to request {id}: {e}"));
            let message: Value = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("a line that is not JSON-RPC ({e}): {line}"));
            assert_eq!(message["jsonrpc"], "2.0", "{line}");
            if message.get("id") == Some(&json!(id)) {
                assert!(
                    message.get("error").is_none(),
                    "request {id} failed: {line}"
                );
                return message["result"].clone();
            }
            assert!(
                message.get("id").is_none(),
                "an answer to another request: {line}"
            );
        }
    }

    fn call(&mut self, id: i64, tool: &str, arguments: Value) -> Value {
        self.send(json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": tool, "arguments": arguments},
        }));
        self.answer(id)
    }
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_binary_speaks_mcp_over_stdio_and_exits_when_stdin_closes() {
    let path = repo().join("spec/vectors/mcp").join(CASE);
    let case: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the case")).expect("JSON");
    let scratch = Scratch::new();
    materialise(&case["input"]["files"], scratch.path());
    let root = |key: &str| {
        scratch
            .path()
            .join(case["input"]["roots"][key].as_str().unwrap())
    };
    // A golfer the recorded case asks about, so the call below finds one.
    let player = calls(&case["input"])
        .into_iter()
        .find_map(|call| call.arguments.get("player")?.as_str().map(str::to_string))
        .expect("a call naming a player");

    let mut child = Command::new(env!("CARGO_BIN_EXE_golf-mcp"))
        .arg("--sessions-dir")
        .arg(root("sessions"))
        .arg("--shots-dir")
        .arg(root("shots"))
        .arg("--golfers-dir")
        .arg(root("golfers"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("golf-mcp starts");
    let stdout = child.stdout.take().expect("stdout");
    let (sender, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let stdin = child.stdin.take();
    let mut session = Session {
        child,
        stdin,
        lines,
    };

    session.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "golf-mcp stdio test", "version": "0"},
        },
    }));
    let init = session.answer(1);
    assert_eq!(init["protocolVersion"], "2025-06-18", "{init}");
    assert_eq!(init["serverInfo"]["name"], "golf-coach");
    assert!(init["instructions"]
        .as_str()
        .is_some_and(|text| !text.is_empty()));
    session.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));

    session.send(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let listed = session.answer(2);
    let names: Vec<&str> = listed["tools"]
        .as_array()
        .expect("a tool list")
        .iter()
        .map(|tool| tool["name"].as_str().expect("a name"))
        .collect();
    let table: Vec<&str> = TOOLS.iter().map(|spec| spec.name).collect();
    assert_eq!(
        names, table,
        "a registry offers every tool, in the table's order"
    );
    assert_eq!(names.len(), 12);
    assert_eq!(names.last(), Some(&"get_strike_profile"));

    let found = session.call(3, "get_strike_profile", json!({"player": player}));
    assert_eq!(found["isError"], false, "{found}");
    let profile = &found["structuredContent"]["result"];
    assert!(profile["display_name"].is_string(), "{found}");
    assert!(profile["profile"]["player_id"].is_string(), "{found}");
    let block: Value = serde_json::from_str(found["content"][0]["text"].as_str().unwrap())
        .expect("a text block is the view's JSON");
    assert_eq!(&block, profile);

    session.send(json!({
        "jsonrpc": "2.0", "id": 4, "method": "resources/read",
        "params": {"uri": CAVEATS_URI},
    }));
    let read = session.answer(4);
    let caveats: Value = serde_json::from_str(read["contents"][0]["text"].as_str().unwrap())
        .expect("the caveats are a JSON object");
    assert!(caveats["READING_THIS_DATA_HONESTLY"].is_string(), "{read}");

    let miss = session.call(
        5,
        "get_swing",
        json!({"session_id": "no-such-session", "swing_id": "1"}),
    );
    assert_eq!(miss["isError"], false, "a miss is an answer: {miss}");
    assert_eq!(
        miss["structuredContent"]["result"]["found"], false,
        "{miss}"
    );

    drop(session.stdin.take());
    let status = session.child.wait().expect("golf-mcp exits");
    assert!(
        status.success(),
        "golf-mcp exited {status} when stdin closed"
    );
}
