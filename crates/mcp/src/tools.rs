//! The tool table: every tool `golf-mcp` offers, its description, its input schema, and its body.
//! [M29 P9]
//!
//! Python declares a tool by decorating a function (`mcp/server.py`), and the SDK derives the input
//! schema from the signature through pydantic. Here each tool's arguments are one struct, and the
//! schema is derived from it by `schemars` through `rmcp`'s own `schema_for_type`, so the schema a
//! model reads and the parse a call goes through are one definition and cannot drift apart.
//!
//! **The order is Python's**: the six every server offers, then the five a golfer registry adds,
//! as `build_server` registers them and as the recorded surface lists them, then
//! `get_strike_profile`, which has no Python twin and is registry-gated for the other five's reason
//! (it starts by resolving a name to a `player_id`).
//!
//! # Every row has a body
//!
//! The table was whole from P9 so the surface was (the plan's P9), and until P15 a row could carry
//! no body yet and answer that it was not ported, naming the phase that would land it, while the
//! family's gate skipped its recorded calls by name. P15 landed the last, so [`ToolSpec::body`] is
//! no longer optional and that answer is gone: a tool listed is a tool answered, by construction.
//!
//! # A body is thin
//!
//! As `mcp/server.py`'s decorated functions are: parse the arguments, call one reader in
//! [`crate::query`], and put its view, or its miss, on the wire through [`crate::answer`]. P10
//! landed the swing and session three, P11 the two shot tools, which read through
//! [`crate::source`], P12 the three career tools, which read through [`crate::career`], P13
//! the two club tools, which read through [`crate::club`], P14 `simulate_flight`, which reads
//! through [`crate::flight`], and P15 `get_strike_profile`, which reads through
//! [`crate::strike`].

use rmcp::handler::server::common::schema_for_type;
use rmcp::model::{CallToolResult, Content, JsonObject};
use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

use crate::career::{self, WindowRefused};
use crate::server::{Clock, GolfMcp};
use crate::source::ShotSource;
use crate::{answer, club, descriptions, flight, query, strike};

/// What a tool's body is: the server, the call's arguments and the clock in, the answer out. An
/// `Err` is a protocol error, which no recorded call produces.
pub type Body = fn(&GolfMcp, &JsonObject, &Clock) -> Result<CallToolResult, rmcp::ErrorData>;

/// One row of the table.
pub struct ToolSpec {
    /// The wire name a model calls.
    pub name: &'static str,
    pub description: &'static str,
    /// Offered only when a golfer registry is configured (`REGISTRY_TOOL_NAMES`, and the twelfth).
    pub needs_registry: bool,
    /// The arguments' JSON Schema.
    pub schema: fn() -> JsonObject,
    /// The arguments parsed, so a call a model could not have made from the schema is refused
    /// before any body runs. `Err` carries serde's sentence.
    pub parse: fn(&JsonObject) -> Result<(), String>,
    pub body: Body,
}

fn parse_as<T: for<'de> Deserialize<'de>>(arguments: &JsonObject) -> Result<(), String> {
    serde_json::from_value::<T>(serde_json::Value::Object(arguments.clone()))
        .map(drop)
        .map_err(|e| e.to_string())
}

// The argument structs, one per signature, each read by its tools' bodies.
//
// **Plain comments, never `///`, on these and their fields**: `schemars` copies a doc comment into
// the schema as its `description`, which a model reads beside the tool's own, and Python's schemas
// carry none. `tests/surface.rs` compares descriptions, so a doc comment here fails the gate.
mod args {
    use super::*;

    fn twenty() -> i64 {
        20
    }

    fn ten() -> i64 {
        10
    }

    // `list_sessions(limit: int = 20)`. Signed, as Python's `int` is: a negative limit is a call
    // the frozen server answers (`tools/sessions` records `-1`), so it is not refused here.
    #[derive(Deserialize, JsonSchema)]
    #[schemars(crate = "rmcp::schemars")]
    pub struct ListSessions {
        #[serde(default = "twenty")]
        pub limit: i64,
    }

    // `get_swing` and `simulate_flight`: `(session_id: str, swing_id: str)`.
    #[derive(Deserialize, JsonSchema)]
    #[schemars(crate = "rmcp::schemars")]
    pub struct SwingRef {
        pub session_id: String,
        pub swing_id: String,
    }

    // `get_session_summary(session_id: str)`.
    #[derive(Deserialize, JsonSchema)]
    #[schemars(crate = "rmcp::schemars")]
    pub struct SessionRef {
        pub session_id: String,
    }

    // `get_recent_shots(count: int = 10)`.
    #[derive(Deserialize, JsonSchema)]
    #[schemars(crate = "rmcp::schemars")]
    pub struct RecentShots {
        #[serde(default = "ten")]
        pub count: i64,
    }

    // `get_shot_by_id(shot_id: str)`.
    #[derive(Deserialize, JsonSchema)]
    #[schemars(crate = "rmcp::schemars")]
    pub struct ShotRef {
        pub shot_id: String,
    }

    // `get_golfer_profile`, `get_bag_profile` and `get_strike_profile`: `(player: str)`.
    #[derive(Deserialize, JsonSchema)]
    #[schemars(crate = "rmcp::schemars")]
    pub struct Player {
        pub player: String,
    }

    // `get_shot_trends(metric: str, player: str, days: int | None = None)`.
    #[derive(Deserialize, JsonSchema)]
    #[schemars(crate = "rmcp::schemars")]
    pub struct ShotTrends {
        pub metric: String,
        pub player: String,
        #[serde(default)]
        pub days: Option<i64>,
    }

    // `compare_sessions(session_a: str, session_b: str, player: str)`.
    #[derive(Deserialize, JsonSchema)]
    #[schemars(crate = "rmcp::schemars")]
    pub struct CompareSessions {
        pub session_a: String,
        pub session_b: String,
        pub player: String,
    }

    // `get_club_profile(player: str, club: str)`. The club is free text, for Python's reason: a
    // model is handed "seven iron" by a user, and `parse_club` is the one place text becomes a club.
    #[derive(Deserialize, JsonSchema)]
    #[schemars(crate = "rmcp::schemars")]
    pub struct ClubProfile {
        pub player: String,
        pub club: String,
    }
}

macro_rules! tool {
    ($name:literal, $description:expr, $registry:expr, $args:ty, $body:expr) => {
        ToolSpec {
            name: $name,
            description: $description,
            needs_registry: $registry,
            schema: schema_for_type::<$args>,
            parse: parse_as::<$args>,
            body: $body,
        }
    };
}

// The bodies, in the table's order.

/// The call's arguments as their struct. [`GolfMcp::call`] has already parsed them once and refused
/// what the schema refuses, so an `Err` here is unreachable from a client; it is still a protocol
/// error rather than a panic.
fn arguments_as<T: for<'de> Deserialize<'de>>(
    arguments: &JsonObject,
) -> Result<T, rmcp::ErrorData> {
    serde_json::from_value(serde_json::Value::Object(arguments.clone()))
        .map_err(|e| rmcp::ErrorData::invalid_params(e.to_string(), None))
}

fn list_sessions(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::ListSessions = arguments_as(arguments)?;
    let sessions = query::list_sessions(&server.dirs().sessions, args.limit);
    Ok(answer::list(&sessions))
}

fn get_swing(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::SwingRef = arguments_as(arguments)?;
    let sessions = &server.dirs().sessions;
    Ok(
        match query::get_swing(sessions, &args.session_id, &args.swing_id) {
            Some(view) => answer::model(&view),
            None => answer::model(&query::missing_swing(&args.session_id, &args.swing_id)),
        },
    )
}

fn get_session_summary(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::SessionRef = arguments_as(arguments)?;
    Ok(
        match query::get_session_summary(&server.dirs().sessions, &args.session_id) {
            Some(detail) => answer::model(&detail),
            None => answer::model(&query::missing_session(&args.session_id)),
        },
    )
}

fn get_recent_shots(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::RecentShots = arguments_as(arguments)?;
    let source = ShotSource::new(&server.dirs().shots);
    Ok(answer::list(&query::recent_shots(&source, args.count)))
}

fn get_shot_by_id(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::ShotRef = arguments_as(arguments)?;
    let source = ShotSource::new(&server.dirs().shots);
    Ok(match query::get_shot(&source, &args.shot_id) {
        Some(view) => answer::model(&view),
        None => answer::model(&query::missing_shot(&args.shot_id)),
    })
}

/// Offered on every shape of the server: the registry, when there is one, supplies the loft and the
/// handedness, and without one the flight still flies on a printed spin. [M29 P14]
fn simulate_flight(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::SwingRef = arguments_as(arguments)?;
    let dirs = server.dirs();
    let source = ShotSource::new(&dirs.shots);
    Ok(
        match flight::flight_for_swing(
            &dirs.sessions,
            &source,
            &args.session_id,
            &args.swing_id,
            dirs.golfers.as_deref(),
        ) {
            Some(view) => answer::model(&view),
            None => answer::model(&flight::missing_swing_flight(
                &args.session_id,
                &args.swing_id,
            )),
        },
    )
}

/// The golfer registry. Only a server with one offers the tools that call this ([`GolfMcp::call`]
/// looks a tool up among those offered), so its absence here is a protocol error no client reaches.
fn registry(server: &GolfMcp) -> Result<&std::path::Path, rmcp::ErrorData> {
    server.dirs().golfers.as_deref().ok_or_else(|| {
        rmcp::ErrorData::internal_error("a per-golfer tool was called with no registry", None)
    })
}

fn get_golfer_profile(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::Player = arguments_as(arguments)?;
    let golfers = registry(server)?;
    Ok(
        match career::golfer_profile(&server.dirs().sessions, golfers, &args.player) {
            Some(view) => answer::model(&view),
            None => answer::model(&career::missing_golfer_profile(&args.player)),
        },
    )
}

/// The one body that reads the clock, and only with `days` and a golfer found. A window CPython's
/// `datetime` cannot start is an error result, as Python's `OverflowError` reached its SDK as one;
/// a recorded call with no clock that reads one is a malformed case, so a protocol error.
fn get_shot_trends(
    server: &GolfMcp,
    arguments: &JsonObject,
    clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::ShotTrends = arguments_as(arguments)?;
    let golfers = registry(server)?;
    let found = career::shot_trends(
        &server.dirs().sessions,
        golfers,
        &args.metric,
        &args.player,
        args.days,
        clock,
    );
    Ok(match found {
        Ok(Some(view)) => answer::model(&view),
        Ok(None) => answer::model(&career::missing_golfer_trend(&args.player)),
        Err(WindowRefused::OutOfRange { days }) => {
            CallToolResult::error(vec![Content::text(format!(
                "get_shot_trends: a window of days={days} would start outside the years 1 to \
                 9999, which no stored swing falls before or after; pass a smaller days, or none \
                 for the whole history"
            ))])
        }
        Err(WindowRefused::NoClock) => {
            return Err(rmcp::ErrorData::internal_error(
                "get_shot_trends read the clock on a call recorded without one",
                None,
            ))
        }
        Err(WindowRefused::UnreadableClock(text)) => {
            return Err(rmcp::ErrorData::internal_error(
                format!("get_shot_trends: the call's clock {text:?} is not a timestamp"),
                None,
            ))
        }
    })
}

fn compare_sessions(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::CompareSessions = arguments_as(arguments)?;
    let golfers = registry(server)?;
    Ok(
        match career::compare_sessions(
            &server.dirs().sessions,
            golfers,
            &args.session_a,
            &args.session_b,
            &args.player,
        ) {
            Some(view) => answer::model(&view),
            None => answer::model(&career::missing_golfer_comparison(&args.player)),
        },
    )
}

fn get_bag_profile(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::Player = arguments_as(arguments)?;
    let golfers = registry(server)?;
    Ok(
        match club::bag_profile(&server.dirs().sessions, golfers, &args.player) {
            Some(view) => answer::model(&view),
            None => answer::model(&club::missing_golfer_bag(&args.player)),
        },
    )
}

/// Two misses on one wire shape: an unknown golfer, whose hint is this tool's, and text that is not
/// a club, which `club::club_profile` answers itself.
fn get_club_profile(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::ClubProfile = arguments_as(arguments)?;
    let golfers = registry(server)?;
    Ok(
        match club::club_profile(&server.dirs().sessions, golfers, &args.player, &args.club) {
            Some(Ok(view)) => answer::model(&view),
            Some(Err(not_a_club)) => answer::model(&not_a_club),
            None => answer::model(&club::missing_golfer_club(&args.player)),
        },
    )
}

/// `golf-core profile --json`'s element for one golfer, or the golfer miss. A profile that fails
/// its own check is an error result naming the field, as the verb names it on stderr and exits 1:
/// a builder bug, which no recorded tree reaches, reported to the client rather than as a protocol
/// failure of the server. [M29 P15]
fn get_strike_profile(
    server: &GolfMcp,
    arguments: &JsonObject,
    _clock: &Clock,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let args: args::Player = arguments_as(arguments)?;
    let golfers = registry(server)?;
    Ok(
        match strike::strike_profile(&server.dirs().sessions, golfers, &args.player) {
            Some(Ok(view)) => answer::model(&view),
            Some(Err(refused)) => CallToolResult::error(vec![Content::text(format!(
                "get_strike_profile: the profile assembled for {} failed its own check, which \
                 is a bug in the builders and not something the data can fix: {refused}",
                pyfmt::str_repr(&args.player)
            ))]),
            None => answer::model(&strike::missing_golfer_strike(&args.player)),
        },
    )
}

/// Every tool, in the order `tools/list` returns them (the module doc).
pub static TOOLS: [ToolSpec; 12] = [
    tool!(
        "list_sessions",
        descriptions::LIST_SESSIONS,
        false,
        args::ListSessions,
        list_sessions
    ),
    tool!(
        "get_swing",
        descriptions::GET_SWING,
        false,
        args::SwingRef,
        get_swing
    ),
    tool!(
        "get_session_summary",
        descriptions::GET_SESSION_SUMMARY,
        false,
        args::SessionRef,
        get_session_summary
    ),
    tool!(
        "get_recent_shots",
        descriptions::GET_RECENT_SHOTS,
        false,
        args::RecentShots,
        get_recent_shots
    ),
    tool!(
        "get_shot_by_id",
        descriptions::GET_SHOT_BY_ID,
        false,
        args::ShotRef,
        get_shot_by_id
    ),
    tool!(
        "simulate_flight",
        descriptions::SIMULATE_FLIGHT,
        false,
        args::SwingRef,
        simulate_flight
    ),
    tool!(
        "get_golfer_profile",
        descriptions::GET_GOLFER_PROFILE,
        true,
        args::Player,
        get_golfer_profile
    ),
    tool!(
        "get_shot_trends",
        descriptions::GET_SHOT_TRENDS,
        true,
        args::ShotTrends,
        get_shot_trends
    ),
    tool!(
        "compare_sessions",
        descriptions::COMPARE_SESSIONS,
        true,
        args::CompareSessions,
        compare_sessions
    ),
    tool!(
        "get_bag_profile",
        descriptions::GET_BAG_PROFILE,
        true,
        args::Player,
        get_bag_profile
    ),
    tool!(
        "get_club_profile",
        descriptions::GET_CLUB_PROFILE,
        true,
        args::ClubProfile,
        get_club_profile
    ),
    tool!(
        "get_strike_profile",
        descriptions::GET_STRIKE_PROFILE,
        true,
        args::Player,
        get_strike_profile
    ),
];

/// The row named `name`, offered or not.
pub fn spec(name: &str) -> Option<&'static ToolSpec> {
    TOOLS.iter().find(|t| t.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_is_unique_and_the_registry_gates_six() {
        let mut names: Vec<&str> = TOOLS.iter().map(|t| t.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), TOOLS.len());
        assert_eq!(TOOLS.iter().filter(|t| t.needs_registry).count(), 6);
    }

    #[test]
    fn a_call_parses_against_its_own_schema() {
        let parse = |name: &str, args: serde_json::Value| {
            (spec(name).unwrap().parse)(args.as_object().unwrap())
        };
        assert!(parse("list_sessions", serde_json::json!({})).is_ok());
        assert!(parse("list_sessions", serde_json::json!({"limit": -1})).is_ok());
        assert!(parse("get_swing", serde_json::json!({"session_id": "s"})).is_err());
        assert!(parse(
            "get_shot_trends",
            serde_json::json!({"metric": "m", "player": "p", "days": null})
        )
        .is_ok());
    }

    /// The twelfth is the per-golfer one with no Python twin, last, and reads one name, as
    /// `get_golfer_profile` and `get_bag_profile` do. [M29 P15]
    #[test]
    fn the_strike_profile_is_the_last_tool_and_takes_a_player_alone() {
        let last = TOOLS.last().unwrap();
        assert_eq!(last.name, "get_strike_profile");
        assert!(last.needs_registry);
        let parse = |args: serde_json::Value| (last.parse)(args.as_object().unwrap());
        assert!(parse(serde_json::json!({"player": "Aaron"})).is_ok());
        assert!(parse(serde_json::json!({})).is_err());
    }
}
