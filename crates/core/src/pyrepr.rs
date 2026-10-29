//! CPython's `repr()` of a value `json.loads` produced. [M29 P27]
//!
//! Written for `golf-mcp`'s tolerant readers (M29 P10, as `golf_mcp::query::py_str`'s leaf) and
//! moved here at P27, as [`crate::textwrap`] was at P26, when `golf-lab reanalyze`'s one-line diff
//! needed it for the windows and anchors it prints: the two shells cannot depend on each other, and
//! both depend on this one.
//!
//! **An object's keys come out sorted**, because `serde_json`'s maps are, where Python's keep the
//! file's order. Neither caller prints an object a writer here produces.

use serde_json::Value;

/// Python's `repr()` of a parsed JSON value: `None`, `True` and `False`, an integer in decimal, a
/// float as its shortest round-trip `repr` ([`pyfmt::repr`]), a string quoted as Python quotes it
/// ([`pyfmt::str_repr`]), and a list or dict as its repr.
pub fn py_repr(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Number(n) if n.is_i64() || n.is_u64() => n.to_string(),
        Value::Number(n) => pyfmt::repr(n.as_f64().expect("a JSON number is an f64")),
        Value::String(text) => pyfmt::str_repr(text),
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(py_repr).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", pyfmt::str_repr(k), py_repr(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn repr_of_a_parsed_value_is_pythons() {
        for (value, want) in [
            (json!("tempo"), "'tempo'"),
            (json!(5), "5"),
            (json!(-3), "-3"),
            (json!(1.0), "1.0"),
            (json!(1e-5), "1e-05"),
            (json!(true), "True"),
            (json!(null), "None"),
            (json!(["a", 1, null, "it's"]), "['a', 1, None, \"it's\"]"),
            (json!({"b": 1, "a": [false]}), "{'a': [False], 'b': 1}"),
        ] {
            assert_eq!(py_repr(&value), want, "{value}");
        }
    }
}
