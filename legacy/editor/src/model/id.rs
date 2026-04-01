use loro::LoroStringValue;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(Uuid);

impl NodeId {
    pub const ROOT: Self = Self(Uuid::nil());

    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(string: &str) -> Option<Self> {
        Some(Self(Uuid::parse_str(string).ok()?))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn to_string(&self) -> String {
        self.0.as_simple().to_string()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for NodeId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

impl PartialEq<LoroStringValue> for NodeId {
    fn eq(&self, other: &LoroStringValue) -> bool {
        self.to_string() == other.to_string()
    }
}
