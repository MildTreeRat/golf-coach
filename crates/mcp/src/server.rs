//! The server: what `golf-mcp` answers on connect, on `tools/list` and `tools/call`, and on the
//! caveats resource. `mcp/server.py`'s `build_server`. [M29 P9]
//!
//! Thin, as Python's is: each tool's body (P10–P15) calls one reader or report and returns it, and
//! nothing here reads, joins or aggregates. The handler half ([`ServerHandler`]) is protocol only
//! and delegates to plain methods, so a gate can call a tool or read the briefing with no transport
//! (`crate::family`), and the handshake is tested once, over a real one (`tests/surface.rs`).
//!
//! # The golfer registry is optional, and its absence removes tools
//!
//! `build_server(golfers_dir=None)` offers six tools and a briefing without the two per-golfer
//! blocks, because a tool that answers "unknown golfer" to every name is worse than one not
//! offered. [`Dirs::golfers`] `None` is that shape. `run_mcp_server.py` always passes a registry
//! (P8's finding 6), so the launched server always offers the rest; `golf-mcp --no-registry` is
//! the one way to start the other shape, which the recorded surface and five tool cases were built
//! as.
//!
//! # The caveats resource (the plan's D10)
//!
//! `golf-coach://caveats` serves every named caveat sentence as one JSON object of strings, keyed by
//! its `contracts/caveats.py` name. The briefing already carries the long blocks; the resource is
//! for a consumer that composes its own prompt around the short ones (`ONLY_CHECKPOINTS_ARE_JUDGED`
//! in `conversation.py`, `ALIGNMENT_CAVEAT` in `coach.py`), so it can read them here at session
//! start and keep no copy. Python's server has no resources, so nothing recorded is moved by it.

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::model::AnnotateAble;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Implementation, JsonObject, ListResourcesResult,
    ListToolsResult, PaginatedRequestParam, ProtocolVersion, RawResource, ReadResourceRequestParam,
    ReadResourceResult, Resource, ResourceContents, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};

use crate::caveats;
use crate::tools::{ToolSpec, TOOLS};

/// `SERVER_NAME`: what a client lists the server as.
pub const SERVER_NAME: &str = "golf-coach";

/// The caveats resource's address.
pub const CAVEATS_URI: &str = "golf-coach://caveats";

/// The newest protocol revision `rmcp` 0.11.0 models, which is what the server answers a client
/// asking for anything newer with (`rmcp`'s `serve_server` takes the lower of the two). Frozen
/// Python's SDK answered 2025-11-25; the difference is declared in `tests/surface.rs`, and
/// `Cargo.toml`'s `rmcp` pin says why the SDK is not newer.
pub const PROTOCOL: ProtocolVersion = ProtocolVersion::V_2025_06_18;

/// The three directories the server reads, as `build_server` takes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub sessions: PathBuf,
    pub shots: PathBuf,
    /// The golfer registry, and the bags beside it. `None` offers no per-golfer tool.
    pub golfers: Option<PathBuf>,
}

/// What a tool reads "now" from. Only `get_shot_trends` with `days` reads it (P8's finding 8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clock {
    /// The process's clock: a live server.
    System,
    /// A fixed instant, the recorded call's `now`.
    At(String),
    /// No clock at all: a recorded call whose `now` is null, which must not read one.
    Unset,
}

/// The server over one set of directories.
#[derive(Debug, Clone)]
pub struct GolfMcp {
    dirs: Arc<Dirs>,
}

impl GolfMcp {
    pub fn new(dirs: Dirs) -> Self {
        GolfMcp {
            dirs: Arc::new(dirs),
        }
    }

    pub fn dirs(&self) -> &Dirs {
        &self.dirs
    }

    pub fn has_registry(&self) -> bool {
        self.dirs.golfers.is_some()
    }

    /// The rows this server offers, in the table's order.
    pub fn offered(&self) -> impl Iterator<Item = &'static ToolSpec> + '_ {
        TOOLS
            .iter()
            .filter(|spec| !spec.needs_registry || self.has_registry())
    }

    /// What `tools/list` returns.
    pub fn tools(&self) -> Vec<Tool> {
        self.offered()
            .map(|spec| Tool::new(spec.name, spec.description, (spec.schema)()))
            .collect()
    }

    /// What `initialize` answers with, before the protocol revision is negotiated down.
    pub fn info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: PROTOCOL,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            // Python's SDK sends an empty version, and so does this: the surface case pins
            // `serverInfo` whole, and a version string says nothing a client acts on.
            server_info: Implementation {
                name: SERVER_NAME.to_string(),
                title: None,
                version: String::new(),
                icons: None,
                website_url: None,
            },
            instructions: Some(caveats::instructions(self.has_registry())),
        }
    }

    /// One `tools/call`. A tool this server does not offer, or arguments its schema refuses, is a
    /// protocol error (`invalid_params`); everything else is the tool's body's answer.
    pub fn call(
        &self,
        name: &str,
        arguments: &JsonObject,
        clock: &Clock,
    ) -> Result<CallToolResult, ErrorData> {
        let spec = self
            .offered()
            .find(|spec| spec.name == name)
            .ok_or_else(|| ErrorData::invalid_params(format!("unknown tool: {name}"), None))?;
        (spec.parse)(arguments).map_err(|why| {
            ErrorData::invalid_params(format!("{name}: arguments refused: {why}"), None)
        })?;
        (spec.body)(self, arguments, clock)
    }

    /// What `resources/list` returns: the caveats resource alone.
    pub fn resources(&self) -> Vec<Resource> {
        let mut raw = RawResource::new(CAVEATS_URI, "caveats");
        raw.title = Some("The standing caveats".to_string());
        raw.description = Some(
            "Every named caveat sentence, as a JSON object of strings keyed by its \
             contracts/caveats.py name. Read it at session start rather than keeping a copy; \
             ALIGNMENT_CAVEAT carries a {summary} for the alignment tier's own clause."
                .to_string(),
        );
        raw.mime_type = Some("application/json".to_string());
        vec![raw.no_annotation()]
    }

    /// One `resources/read`. Any address but [`CAVEATS_URI`] is not found.
    pub fn read(&self, uri: &str) -> Result<ReadResourceResult, ErrorData> {
        if uri != CAVEATS_URI {
            return Err(ErrorData::resource_not_found(
                format!("no resource at {uri}: golf-mcp serves {CAVEATS_URI} alone"),
                None,
            ));
        }
        let sentences: serde_json::Map<String, serde_json::Value> = caveats::named_sentences()
            .into_iter()
            .map(|(name, text)| (name.to_string(), serde_json::Value::String(text)))
            .collect();
        let text = serde_json::to_string_pretty(&sentences).expect("strings serialize");
        let mut contents = ResourceContents::text(text, CAVEATS_URI);
        if let ResourceContents::TextResourceContents { mime_type, .. } = &mut contents {
            *mime_type = Some("application/json".to_string());
        }
        Ok(ReadResourceResult {
            contents: vec![contents],
        })
    }
}

impl ServerHandler for GolfMcp {
    fn get_info(&self) -> ServerInfo {
        self.info()
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: self.tools(),
            ..ListToolsResult::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let arguments = request.arguments.unwrap_or_default();
        self.call(&request.name, &arguments, &Clock::System)
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult {
            resources: self.resources(),
            ..ListResourcesResult::default()
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        self.read(&request.uri)
    }
}
