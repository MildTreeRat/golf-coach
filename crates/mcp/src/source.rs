//! The shot source the server answers from: `run_mcp_server.py`'s
//! `CompositeShotDataSource([ScreenShotDataSource(shots_dir)])`, one screen-capture source behind
//! the composite. `launch_monitor/composite.py` and `launch_monitor/screen/source.py`, as that one
//! construction uses them. [M29 P11]
//!
//! Python's `ShotDataSource` is a port with several adapters (the mock, the composite, an R10 feed
//! that never landed), and the MCP server is handed whichever one its caller built. `golf-mcp` is
//! only ever launched over a shot directory, so this is that one shape and not the port: the store
//! is `storage::shot_store::ShotStore`, already ported, and what is left is the order the two
//! adapters above it impose. A second source joins when a second kind of shot feed exists.
//!
//! **The order is the composite's, and it is not one rule** (the M29 plan's P8 finding 7, recorded
//! in `spec/vectors/mcp/tools/shots.json`):
//!
//! - [`ShotSource::recent`] is newest first. The screen source answers the first `count` of the
//!   store's newest-first list, and the composite then drops a repeated `shot_id`, keeping the
//!   first, which is the newer. So `count: 3` over three files with two sharing an id answers two
//!   shots, not three: the count is applied before the repeat is dropped. The composite's re-sort of
//!   one source's already-sorted list is stable, so it moves nothing and is not repeated here.
//! - [`ShotSource::stream`] is oldest first, the order the shots were hit in, and drops a repeat
//!   keeping the first it meets, which is the *older*. `get_shot_by_id` scans it, so it finds the
//!   older of two shots sharing an id while `get_recent_shots` shows the newer. Kept as recorded;
//!   two shots under one id are a store that was written wrong, and neither answer is better.
//!
//! The screen source's two filters (`session_id`, `include_needs_review`) are left out: the server
//! builds it with neither, and a flagged shot is shown labelled rather than hidden (ADR-014).

use std::collections::HashSet;
use std::path::Path;

use contracts::shot::ShotData;
use storage::shot_store::ShotStore;

/// One directory of parsed shots, served as the composite serves it.
#[derive(Debug, Clone)]
pub struct ShotSource {
    store: ShotStore,
}

impl ShotSource {
    pub fn new(shots_dir: &Path) -> Self {
        ShotSource {
            store: ShotStore::new(shots_dir),
        }
    }

    /// `recent(count)`: the newest `count` shots, newest first, each `shot_id` once. A count of
    /// zero or less is none, as both of Python's `count <= 0` guards answer.
    pub fn recent(&self, count: i64) -> Vec<ShotData> {
        let Ok(count) = usize::try_from(count) else {
            return Vec::new();
        };
        let mut seen = HashSet::new();
        self.store
            .all()
            .into_iter()
            .take(count)
            .filter(|shot| seen.insert(shot.shot_id.clone()))
            .collect()
    }

    /// `stream()`: every shot, oldest first, each `shot_id` once (the first met, so the older).
    ///
    /// The store's newest-first list reversed, as the screen source's `reversed(self._shots())`
    /// is: two shots at one instant therefore stream in the reverse of their files' order.
    pub fn stream(&self) -> impl Iterator<Item = ShotData> {
        let mut seen = HashSet::new();
        self.store
            .all()
            .into_iter()
            .rev()
            .filter(move |shot| seen.insert(shot.shot_id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn put(store: &ShotStore, shot_id: &str, key: &str, when: &str) {
        let shot: ShotData = serde_json::from_value(json!({
            "shot_id": shot_id, "session_id": "range", "timestamp": when, "source": "screen",
            "carry_distance": 120.0,
            "provenance": {"device": "hd_golf", "parse_confidence": 0.9, "image_sha256": key},
        }))
        .unwrap();
        store.put(&shot).unwrap();
    }

    /// The composite's two orders over one store, with a repeated id and a tie: what
    /// `tools/shots` records, reduced to the rule.
    #[test]
    fn recent_counts_before_it_drops_a_repeat_and_stream_keeps_the_older() {
        let scratch = crate::family::Scratch::new();
        let dir = scratch.path().join("shots");
        let store = ShotStore::new(&dir);
        put(&store, "a", "k1", "2026-08-04T12:00:00Z");
        put(&store, "b", "k2", "2026-08-04T12:00:00Z");
        put(&store, "a", "k3", "2026-08-04T13:00:00Z");
        let source = ShotSource::new(&dir);

        let ids = |shots: Vec<ShotData>| -> Vec<(String, String)> {
            shots
                .into_iter()
                .map(|s| {
                    let key = s.provenance.unwrap().image_sha256.unwrap();
                    (s.shot_id, key)
                })
                .collect()
        };
        let pair = |id: &str, key: &str| (id.to_string(), key.to_string());
        assert_eq!(ids(source.recent(1)), [pair("a", "k3")]);
        // k3 (the newer `a`), then k1 and k2 at one instant in file order; k1 is a repeat of `a`.
        assert_eq!(ids(source.recent(2)), [pair("a", "k3")]);
        assert_eq!(ids(source.recent(3)), [pair("a", "k3"), pair("b", "k2")]);
        assert!(source.recent(0).is_empty() && source.recent(-5).is_empty());
        // Oldest first: the tie comes out reversed (k2 before k1), and the older `a` is kept.
        assert_eq!(
            ids(source.stream().collect()),
            [pair("b", "k2"), pair("a", "k1")]
        );
    }
}
