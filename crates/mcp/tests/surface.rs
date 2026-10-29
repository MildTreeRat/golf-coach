//! The surface gate: what a client sees on connecting to `golf-mcp` is what frozen Python's server
//! showed one, `spec/vectors/mcp/surface/server.json`, outside five declared differences. [M29 P9]
//!
//! Connected over a real handshake (an in-memory pipe, `rmcp`'s client on the other end, asking
//! for the revision Python's SDK asked for), so what is compared is what the wire carried and not
//! the handler's fields. Both server shapes, with a golfer registry and without, as recorded.
//!
//! # What is declared, and why each is not a regression
//!
//! 1. **`protocolVersion`**: recorded `2025-11-25`, answered `2025-06-18`, the newest revision
//!    `rmcp` 0.11.0 models (`golf_mcp::server::PROTOCOL`; `Cargo.toml`'s pin says why the SDK is
//!    no newer). 2025-06-18 is the revision that brought `structuredContent`, which every recorded
//!    answer carries, so nothing a tool returns needs the later one.
//! 2. **`instructions`, in one bullet**: the `unscored` reasons bullet is reworded (the plan's C13,
//!    `golf_mcp::caveats`' module doc). Declared as narrowly as that: the recorded briefing must be
//!    `instructions_worded(_, Wording::Frozen)` exactly, which is every other line of Python's
//!    derived from Rust's registries and wrapped by Rust's `fill`, and Rust's must be that text
//!    with the one bullet replaced.
//! 3. **`tools`, one added**: `get_strike_profile` (M37's graded profile, no Python twin) is the
//!    last tool on the registry shape. Its description and its body are M29 P15's
//!    (`golf_mcp::descriptions::GET_STRIKE_PROFILE`, `golf_mcp::strike`).
//! 4. **`get_club_profile`'s description, in one sentence** [M29 P15]: Python's "`n_swings` counts
//!    clips" is false under Rust's corpus, where a photo-only entry is one of a club's swings with
//!    no clip (ADR-024's M35 addendum, the plan's P13 finding 5). Declared as narrowly as the
//!    briefing's bullet: the recorded description must hold
//!    `descriptions::CLUB_PROFILE_N_SWINGS_FROZEN` once, and Rust's must be that text with it
//!    replaced by `CLUB_PROFILE_N_SWINGS`. Every other description is Python's exactly.
//! 5. **`instructions`' bag block, in one bullet** [M29 P34]: the mishit bullet of
//!    `READING_A_BAG` names `golf-core flag-mishit --clear`, where Python's names
//!    `flag_mishit.py --clear`, the script P34 deleted (`golf_mcp::caveats`' module doc). Declared
//!    as narrowly as item 2: the recorded registry-shape briefing must hold
//!    `mishit_exclusion_bag_bullet(Wording::Frozen)` once, and the registry-less one not at all,
//!    because the bag block ships only with a registry.
//!
//! # The input-schema normalization (the plan's P9)
//!
//! pydantic and `schemars` write the same contract differently: pydantic titles the object and
//! every property (`list_sessionsArguments`, `Limit`) and spells a nullable integer
//! `anyOf [integer, null]`; `schemars`, through `rmcp`'s `schema_for_type`, adds `$schema`, titles
//! the object with the Rust struct's name, adds `format: int64`, and spells it `nullable: true`.
//! What a client must agree on is kept, and only that: the object's type, each property's name,
//! the set of JSON types it accepts (`type`, an `anyOf` of types, and `nullable` folded into one
//! set), its default where either side states one, `required` as a set, and any `description` on
//! the object or a property, because a model reads those (pydantic writes none; `schemars` would
//! copy an argument struct's doc comment into one, which is why `golf_mcp::tools` uses plain
//! comments there). Titles, formats and `$schema` are dropped.

use std::collections::BTreeSet;
use std::path::PathBuf;

use golf_mcp::caveats::{
    instructions, instructions_worded, mishit_exclusion_bag_bullet, named_sentences,
    reading_this_data_honestly, unscored_reasons_bullet, Wording,
};
use golf_mcp::descriptions::{CLUB_PROFILE_N_SWINGS, CLUB_PROFILE_N_SWINGS_FROZEN};
use golf_mcp::server::{Dirs, GolfMcp, CAVEATS_URI};
use rmcp::model::{ClientInfo, ReadResourceRequestParam, ResourceContents};
use rmcp::ServiceExt;
use serde_json::{json, Map, Value};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn recorded() -> Value {
    let path = repo().join("spec/vectors/mcp/surface/server.json");
    let text = std::fs::read_to_string(&path).expect("the surface case");
    serde_json::from_str(&text).expect("the surface case is JSON")
}

/// The server a recorded shape names. No tool is called, so the directories need not exist.
fn dirs(shape: &Value) -> Dirs {
    Dirs {
        sessions: PathBuf::from(shape["sessions"].as_str().unwrap()),
        shots: PathBuf::from(shape["shots"].as_str().unwrap()),
        golfers: shape["golfers"].as_str().map(PathBuf::from),
    }
}

/// What a client sees: the initialize answer and the tool list in the recorded case's keys, plus
/// the resources it lists and the caveats resource's text.
async fn connect(dirs: Dirs) -> (Value, Value, String) {
    let (server_end, client_end) = tokio::io::duplex(1 << 20);
    let server = tokio::spawn(async move {
        let running = GolfMcp::new(dirs)
            .serve(server_end)
            .await
            .expect("the server starts");
        running.waiting().await.expect("the server stops cleanly");
    });
    let asking: ClientInfo = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {},
        "clientInfo": {"name": "golf-mcp surface gate", "version": "0"},
    }))
    .expect("a client's initialize");
    let client = asking.serve(client_end).await.expect("the handshake");
    let info = serde_json::to_value(client.peer_info().expect("the server's answer")).unwrap();
    let tools = client.list_all_tools().await.expect("tools/list");
    let resources = client.list_all_resources().await.expect("resources/list");
    let read = client
        .read_resource(ReadResourceRequestParam {
            uri: CAVEATS_URI.to_string(),
        })
        .await
        .expect("resources/read");
    client.cancel().await.expect("the client closes");
    server.await.expect("the server task");

    let tools: Vec<Value> = tools
        .iter()
        .map(|tool| {
            let wire = serde_json::to_value(tool).unwrap();
            json!({
                "name": wire["name"],
                "description": wire["description"],
                "inputSchema": wire["inputSchema"],
            })
        })
        .collect();
    let surface = json!({
        "protocolVersion": info["protocolVersion"],
        "serverInfo": info["serverInfo"],
        "instructions": info["instructions"],
        "tools": tools,
    });
    let [ResourceContents::TextResourceContents {
        text, mime_type, ..
    }] = read.contents.as_slice()
    else {
        panic!("one text block, not {:?}", read.contents)
    };
    assert_eq!(mime_type.as_deref(), Some("application/json"));
    (
        surface,
        serde_json::to_value(resources).unwrap(),
        text.clone(),
    )
}

/// The types a property accepts, as one set: `type` (one or a list), an `anyOf` of types, and
/// `nullable`.
fn types(property: &Map<String, Value>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut add = |value: &Value| match value {
        Value::String(t) => {
            out.insert(t.clone());
        }
        Value::Array(ts) => out.extend(ts.iter().map(|t| t.as_str().unwrap().to_string())),
        other => panic!("a type is a name or a list of names, not {other}"),
    };
    if let Some(t) = property.get("type") {
        add(t);
    }
    for branch in property
        .get("anyOf")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        add(&branch["type"]);
    }
    if property.get("nullable") == Some(&Value::Bool(true)) {
        out.insert("null".to_string());
    }
    out
}

/// The module doc's normalization.
fn normalized(schema: &Value) -> Value {
    let properties: Map<String, Value> = schema["properties"]
        .as_object()
        .map(|properties| {
            properties
                .iter()
                .map(|(name, property)| {
                    let property = property.as_object().unwrap();
                    let mut kept = json!({"types": types(property)});
                    for key in ["default", "description"] {
                        if let Some(value) = property.get(key) {
                            kept[key] = value.clone();
                        }
                    }
                    (name.clone(), kept)
                })
                .collect()
        })
        .unwrap_or_default();
    let required: BTreeSet<&str> = schema["required"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|name| name.as_str().unwrap())
        .collect();
    json!({
        "type": schema["type"],
        "description": schema.get("description"),
        "properties": properties,
        "required": required,
    })
}

/// The declared tool, and the shape it is offered on.
const ADDED_TOOL: &str = "get_strike_profile";

#[tokio::test]
async fn the_surface_is_frozen_pythons_outside_the_five_declared_differences() {
    let case = recorded();
    assert_eq!(case["provenance"]["oracle"], "python");
    for (shape, registry) in [("with_registry", true), ("without_registry", false)] {
        let expected = &case["expected"][shape];
        let (ours, _, _) = connect(dirs(&case["input"]["servers"][shape])).await;

        // 1. The protocol revision, declared.
        assert_eq!(expected["protocolVersion"], "2025-11-25", "{shape}");
        assert_eq!(ours["protocolVersion"], "2025-06-18", "{shape}");
        assert_eq!(ours["serverInfo"], expected["serverInfo"], "{shape}");

        // 2. and 5. The briefing: Python's exactly under Python's wording, and Rust's one bullet
        // apart, or two on the registry shape, which alone carries the bag block.
        let frozen = expected["instructions"].as_str().unwrap();
        assert_eq!(
            frozen,
            instructions_worded(registry, Wording::Frozen),
            "{shape}"
        );
        let (old, new) = (
            unscored_reasons_bullet(Wording::Frozen),
            unscored_reasons_bullet(Wording::Rust),
        );
        assert_eq!(frozen.matches(old.as_str()).count(), 1, "{shape}");
        assert_ne!(old, new);
        let (old_bag, new_bag) = (
            mishit_exclusion_bag_bullet(Wording::Frozen),
            mishit_exclusion_bag_bullet(Wording::Rust),
        );
        assert_eq!(
            frozen.matches(old_bag.as_str()).count(),
            usize::from(registry),
            "{shape}"
        );
        assert_ne!(old_bag, new_bag);
        assert_eq!(
            ours["instructions"],
            frozen.replace(&old, &new).replace(&old_bag, &new_bag),
            "{shape}"
        );
        assert_eq!(ours["instructions"], instructions(registry), "{shape}");

        // 3. The tools: Python's, in Python's order, and the declared twelfth.
        let theirs = expected["tools"].as_array().unwrap();
        let mine = ours["tools"].as_array().unwrap();
        let names = |tools: &[Value]| -> Vec<String> {
            tools
                .iter()
                .map(|t| t["name"].as_str().unwrap().to_string())
                .collect()
        };
        let mut want = names(theirs);
        if registry {
            want.push(ADDED_TOOL.to_string());
        }
        assert_eq!(names(mine), want, "{shape}");
        for (python, rust) in theirs.iter().zip(mine) {
            let name = &python["name"];
            // 4. One sentence of one description, declared; every other is Python's exactly.
            let recorded = python["description"].as_str().unwrap();
            let want = if name == "get_club_profile" {
                assert_eq!(
                    recorded.matches(CLUB_PROFILE_N_SWINGS_FROZEN).count(),
                    1,
                    "{shape} {name}"
                );
                recorded.replace(CLUB_PROFILE_N_SWINGS_FROZEN, CLUB_PROFILE_N_SWINGS)
            } else {
                recorded.to_string()
            };
            assert_eq!(rust["description"], want, "{shape} {name}");
            assert_eq!(
                normalized(&rust["inputSchema"]),
                normalized(&python["inputSchema"]),
                "{shape} {name}: {}",
                rust["inputSchema"]
            );
        }
    }
}

#[tokio::test]
async fn the_caveats_resource_serves_every_named_sentence_the_briefing_agrees_with() {
    let case = recorded();
    let (ours, resources, text) = connect(dirs(&case["input"]["servers"]["with_registry"])).await;
    assert_eq!(resources.as_array().unwrap().len(), 1);
    assert_eq!(resources[0]["uri"], CAVEATS_URI);
    let served: Map<String, Value> = serde_json::from_str(&text).expect("a JSON object");
    let named = named_sentences();
    assert_eq!(served.len(), named.len());
    for (name, sentence) in named {
        assert_eq!(served[name], sentence, "{name}");
    }
    let briefing = ours["instructions"].as_str().unwrap();
    assert!(briefing.contains(&reading_this_data_honestly()));
    assert!(served["ALIGNMENT_CAVEAT"]
        .as_str()
        .unwrap()
        .contains("{summary}"));
}
