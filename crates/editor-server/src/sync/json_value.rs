use serde::{Deserialize, Serialize};

// `serde_json::Value` exposed via tsify emits a bare `Value` reference in the
// generated d.ts with no declaration, breaking direct imports of types that
// reference it. Wrapping in this newtype lets tsify emit `type JsonValue = unknown`.
// `#[ffi]` is not used here because that macro panics on tuple structs, and its
// `custom(...)` form would still emit the dangling `Value` alias.
#[cfg_attr(feature = "wasm", derive(::tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(type = "unknown"))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonValue(pub serde_json::Value);

impl From<serde_json::Value> for JsonValue {
    fn from(v: serde_json::Value) -> Self {
        Self(v)
    }
}
