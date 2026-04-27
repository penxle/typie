use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Tri<T> {
    Absent,
    Uniform { value: T },
    Mixed,
}

impl<T> Default for Tri<T> {
    fn default() -> Self {
        Tri::Absent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_unit_payload_uniform() {
        let v: Tri<()> = Tri::Uniform { value: () };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#"{"type":"uniform","value":null}"#);
        let parsed: Tri<()> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn serde_value_payload_uniform() {
        let v: Tri<u32> = Tri::Uniform { value: 1600 };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#"{"type":"uniform","value":1600}"#);
    }

    #[test]
    fn serde_absent_and_mixed() {
        let a: Tri<u32> = Tri::Absent;
        let m: Tri<u32> = Tri::Mixed;
        assert_eq!(serde_json::to_string(&a).unwrap(), r#"{"type":"absent"}"#);
        assert_eq!(serde_json::to_string(&m).unwrap(), r#"{"type":"mixed"}"#);
    }

    #[test]
    fn default_is_absent() {
        let d: Tri<u32> = Tri::default();
        assert_eq!(d, Tri::Absent);
    }
}
