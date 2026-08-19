//! Deterministic JSON for hashing (JCS-shaped: sorted keys, compact).

use serde_json::Value;

/// Compact JSON with object keys sorted recursively. Good enough until a JCS crate is named.
pub fn canonical_json(value: &Value) -> String {
    serde_json::to_string(&sort_value(value)).unwrap_or_else(|_| "null".into())
}

pub fn sort_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for k in keys {
                out.insert(k.clone(), sort_value(&map[k]));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_value).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_nested_object_keys() {
        let v = json!({"b": 1, "a": {"z": 2, "m": 3}});
        assert_eq!(canonical_json(&v), r#"{"a":{"m":3,"z":2},"b":1}"#);
    }

    #[test]
    fn array_order_preserved() {
        let v = json!({"xs": [2, 1], "a": 0});
        assert_eq!(canonical_json(&v), r#"{"a":0,"xs":[2,1]}"#);
    }
}
