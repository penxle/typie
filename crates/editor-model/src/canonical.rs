use serde::Serialize;
use serde_json::Value;

/// All xxh3 content-hash inputs MUST go through this function so that hashes
/// are deterministic across processes regardless of `serde_json` map iteration order.
///
/// # Panics
/// Panics if `T::serialize` returns `Err`. All editor-model types use derived
/// `Serialize` and never fail.
pub fn canonical_serialize<T: Serialize>(value: &T) -> Vec<u8> {
    let intermediate: Value = serde_json::to_value(value)
        .expect("canonical_serialize: serde_json::to_value never fails for well-formed input");
    canonical_value_to_bytes(&intermediate)
}

fn canonical_value_to_bytes(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write_canonical(&mut out, value);
    out
}

fn write_canonical(out: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
        Value::Number(n) => out.extend_from_slice(n.to_string().as_bytes()),
        Value::String(s) => {
            let escaped = serde_json::to_string(s).expect("string serialize never fails");
            out.extend_from_slice(escaped.as_bytes());
        }
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_canonical(out, item);
            }
            out.push(b']');
        }
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            out.push(b'{');
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                let escaped = serde_json::to_string(k).expect("key serialize never fails");
                out.extend_from_slice(escaped.as_bytes());
                out.push(b':');
                write_canonical(out, v);
            }
            out.push(b'}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Reordered {
        b: i32,
        a: i32,
    }

    #[derive(Serialize)]
    struct Reordered2 {
        a: i32,
        b: i32,
    }

    #[test]
    fn key_order_independence() {
        let v1 = Reordered { b: 2, a: 1 };
        let v2 = Reordered2 { a: 1, b: 2 };
        assert_eq!(canonical_serialize(&v1), canonical_serialize(&v2));
    }

    #[test]
    fn no_whitespace() {
        let v = serde_json::json!({ "a": 1, "b": [1, 2] });
        let bytes = canonical_value_to_bytes(&v);
        let s = String::from_utf8(bytes).unwrap();
        assert!(!s.contains(' '));
        assert!(!s.contains('\n'));
    }

    #[test]
    fn nested_keys_sorted() {
        let v = serde_json::json!({ "z": { "b": 2, "a": 1 }, "a": null });
        let bytes = canonical_value_to_bytes(&v);
        let s = String::from_utf8(bytes).unwrap();
        assert_eq!(s, r#"{"a":null,"z":{"a":1,"b":2}}"#);
    }

    #[test]
    fn empty_containers() {
        let v = serde_json::json!({});
        assert_eq!(canonical_value_to_bytes(&v), b"{}");
        let v = serde_json::json!([]);
        assert_eq!(canonical_value_to_bytes(&v), b"[]");
    }

    #[test]
    fn primitives() {
        assert_eq!(canonical_value_to_bytes(&serde_json::json!(null)), b"null");
        assert_eq!(canonical_value_to_bytes(&serde_json::json!(true)), b"true");
        assert_eq!(
            canonical_value_to_bytes(&serde_json::json!(false)),
            b"false"
        );
        assert_eq!(canonical_value_to_bytes(&serde_json::json!(0)), b"0");
        assert_eq!(canonical_value_to_bytes(&serde_json::json!(-42)), b"-42");
        assert_eq!(canonical_value_to_bytes(&serde_json::json!(1.5)), b"1.5");
    }

    #[test]
    fn strings_with_special_chars() {
        let v = serde_json::json!("a\"b\\c\nd");
        let bytes = canonical_value_to_bytes(&v);
        let s = String::from_utf8(bytes).unwrap();
        assert_eq!(s, r#""a\"b\\c\nd""#);
    }

    #[test]
    fn public_entry_used() {
        #[derive(serde::Serialize)]
        struct S {
            a: i32,
            b: bool,
        }
        let v = S { a: 1, b: true };
        let via_public = canonical_serialize(&v);
        let s = String::from_utf8(via_public).unwrap();
        assert_eq!(s, r#"{"a":1,"b":true}"#);
    }
}
