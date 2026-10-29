//! `golf-mcp`: the MCP server in Rust. `src/golf_coach/mcp/` with `scripts/run_mcp_server.py` in
//! front of it. [M29 P9]
//!
//! It exposes analyzed swings, launch-monitor shots and one golfer's history to an LLM client as
//! MCP tools over stdio, as the frozen Python server does (ADR-006), and is held to that server's
//! answers by `spec/vectors/mcp/`, recorded once from it (the M29 plan's D9). A shell like
//! `golf-core` (D13): it may hold any library crate, and nothing depends on it.
//!
//! - [`caveats`] — the standing caveats, derived from `contracts`' registries as Python's are, and
//!   the one place they now reach a model from (D10). [`textwrap`] is CPython's `fill`, for them,
//!   re-exported from `golf_core::textwrap` since P26 moved it there for `golf-lab`'s report.
//! - [`descriptions`] — what each tool is for, in the words a model picks it by.
//! - [`tools`] — the table: name, description, input schema, and the phase that lands each body.
//! - [`query`] — `mcp/query.py`: the swing, session and shot views, and the readers that fill them.
//! - [`source`] — the shot source those readers draw shots from, in the composite's two orders.
//! - [`career`] — `mcp/career.py`: one golfer's profile, a metric's trend and two sessions held
//!   against each other, over Rust's corpus.
//! - [`club`] — `mcp/club.py`: one golfer's bag and one club of it, over the same corpus, with a
//!   note naming which of five silences an empty answer is.
//! - [`flight`] — `mcp/flight.py`: one swing's shot flown by `crates/analysis`, with its caveats,
//!   the printed comparison, and where today's flight and the stored one disagree.
//! - [`strike`] — no Python twin: one golfer's graded strike profile, `golf-core profile --json`'s
//!   shape, through `golf_core::profile::run_profile`.
//! - [`answer`] — a view on the wire, as frozen Python's SDK put it there.
//! - [`server`] — the handler: the briefing, `tools/list`, `tools/call` and the caveats resource.
//! - [`family`] — the family's runner, one recorded case in, the server's answers out.
//!
//! **Built a phase at a time.** P9 made the surface whole, with every body answering that it was not
//! ported yet. P10 landed `list_sessions`, `get_swing` and `get_session_summary`, P11
//! `get_recent_shots` and `get_shot_by_id`, P12 `get_golfer_profile`, `get_shot_trends` and
//! `compare_sessions`, P13 `get_bag_profile` and `get_club_profile`, P14 `simulate_flight`, and
//! P15 `get_strike_profile`, the last, after which a row's body stopped being optional.
//!
//! **Two binaries' worth of seams.** `golf-mcp` serves over stdio, and `golf-mcp answer-case` is
//! the family's runner as a process ([`family::answer_case`]), which is how `golf-core rerecord`
//! walks this family: nothing may depend on this crate, so the recorder runs the binary.

pub mod answer;
pub mod career;
pub mod caveats;
pub mod club;
pub mod descriptions;
pub mod family;
pub mod flight;
pub mod query;
pub mod server;
pub mod source;
pub mod strike;
pub use golf_core::textwrap;
pub mod tools;
