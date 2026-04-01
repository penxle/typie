use editor_common::Ffi;
use editor_macros::ffi;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[ffi(custom(String))]
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u64);

impl NodeId {
    pub const ROOT: Self = Self(0);

    pub fn new() -> Self {
        let mut buf = [0u8; 8];
        getrandom::fill(&mut buf).expect("failed to generate random bytes");
        Self(u64::from_le_bytes(buf))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid NodeId")]
pub struct ParseNodeIdError;

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base62::encode_fmt(self.0))
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base62::encode_fmt(self.0))
    }
}

impl FromStr for NodeId {
    type Err = ParseNodeIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let n = base62::decode(s).map_err(|_| ParseNodeIdError)?;
        u64::try_from(n).map(Self).map_err(|_| ParseNodeIdError)
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for NodeId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&base62::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for NodeId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let n = base62::decode(&s).map_err(serde::de::Error::custom)?;
        u64::try_from(n)
            .map(Self)
            .map_err(|_| serde::de::Error::custom("NodeId overflow"))
    }
}

impl Ffi for NodeId {
    type Target = String;
    type Error = ParseNodeIdError;

    fn to_ffi(&self) -> String {
        self.to_string()
    }

    fn from_ffi(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_is_zero() {
        assert_eq!(NodeId::ROOT.to_string(), "0");
    }

    #[test]
    fn new_generates_unique_ids() {
        let a = NodeId::new();
        let b = NodeId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn from_str_roundtrip() {
        let id = NodeId::new();
        let s = id.to_string();
        let parsed = NodeId::from_str(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn from_str_invalid_returns_err() {
        assert!(NodeId::from_str("!!!invalid").is_err());
    }

    #[test]
    fn from_str_overflow_returns_err() {
        // u64::MAX + 1 in base62
        assert!(NodeId::from_str("LygHa16AHYG").is_err());
    }

    #[test]
    fn copy_semantics() {
        let a = NodeId::new();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn serde_roundtrip() {
        let id = NodeId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }
}
