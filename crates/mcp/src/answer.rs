//! A tool's return value as frozen Python's SDK put it on the wire. [M29 P10]
//!
//! The `mcp` 2.0.0 SDK serializes a tool's return through pydantic, and every tool here returns a
//! model, a union of two, or a list, so the recorded answers have one shape (the M29 plan's P8
//! finding 4): `structuredContent` is `{"result": …}`, `content` is one text block per list item
//! (none for an empty list) or one for a model, each block is pydantic's `indent=2` JSON of its
//! item, and `isError` is `false`. [`CallToolResult::structured`] writes one compact block instead,
//! which is not that shape, so the result is built here by hand.
//!
//! **The text keeps the view's field order**, because a view's fields are serialized as declared,
//! in pydantic's order, and an insertion-ordered map ([`Ordered`]) keeps its own. Nothing compares
//! that order (the gate parses each block), but it is the order a model reads.
//! `structuredContent` is a `serde_json::Value`, whose maps are sorted in this workspace, as the
//! recorder's writer sorted the committed ones.

use rmcp::model::{CallToolResult, Content};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::{json, Value};

/// pydantic's `model_dump_json(indent=2)`, which `serde_json`'s pretty printer matches: two spaces,
/// `": "`, and non-ASCII written raw.
fn text_of<T: Serialize>(item: &T) -> String {
    serde_json::to_string_pretty(item).expect("a view serializes")
}

fn value_of<T: Serialize>(item: &T) -> Value {
    serde_json::to_value(item).expect("a view serializes")
}

fn success(content: Vec<Content>, result: Value) -> CallToolResult {
    let mut answer = CallToolResult::success(content);
    answer.structured_content = Some(json!({ "result": result }));
    answer
}

/// The answer of a tool that returns one model, or either arm of a `View | NotFound` union.
pub fn model<T: Serialize>(item: &T) -> CallToolResult {
    success(vec![Content::text(text_of(item))], value_of(item))
}

/// The answer of a tool that returns a list: one block per item, and none for an empty list.
pub fn list<T: Serialize>(items: &[T]) -> CallToolResult {
    success(
        items
            .iter()
            .map(|item| Content::text(text_of(item)))
            .collect(),
        Value::Array(items.iter().map(value_of).collect()),
    )
}

/// A Python `dict` built by insertion: a new key goes last and a repeated key keeps its place and
/// takes the new value, so a view's map serializes in the order Python's did.
///
/// `serde_json`'s `Map` is sorted here (no `preserve_order`), which is right for the dicts Python
/// builds from `sorted(…)` and wrong for these.
#[derive(Debug, Clone, PartialEq)]
pub struct Ordered<V>(Vec<(String, V)>);

impl<V> Default for Ordered<V> {
    fn default() -> Self {
        Ordered(Vec::new())
    }
}

impl<V> Ordered<V> {
    /// `dict[key] = value`.
    pub fn insert(&mut self, key: &str, value: V) {
        match self.0.iter_mut().find(|(k, _)| k == key) {
            Some((_, slot)) => *slot = value,
            None => self.0.push((key.to_string(), value)),
        }
    }

    /// `dict.items()`, in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<V: Serialize> Serialize for Ordered<V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct View {
        b: f64,
        a: Option<String>,
        m: Ordered<i64>,
    }

    #[test]
    fn a_model_is_one_block_in_field_order_and_its_value_under_result() {
        let mut m = Ordered::default();
        m.insert("z", 1);
        m.insert("y", 2);
        m.insert("z", 3);
        let answer = model(&View {
            b: 94.9,
            a: None,
            m,
        });
        let wire = serde_json::to_value(&answer).unwrap();
        assert_eq!(wire["isError"], false);
        assert_eq!(
            wire["structuredContent"],
            json!({"result": {"a": null, "b": 94.9, "m": {"y": 2, "z": 3}}})
        );
        assert_eq!(
            wire["content"][0]["text"],
            "{\n  \"b\": 94.9,\n  \"a\": null,\n  \"m\": {\n    \"z\": 3,\n    \"y\": 2\n  }\n}"
        );
    }

    #[test]
    fn an_empty_list_has_no_blocks_and_an_empty_result() {
        let wire = serde_json::to_value(list::<View>(&[])).unwrap();
        assert_eq!(wire["content"], json!([]));
        assert_eq!(wire["structuredContent"], json!({"result": []}));
    }
}
