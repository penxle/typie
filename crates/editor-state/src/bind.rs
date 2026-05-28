use editor_macros::ffi;
use serde::{Deserialize, Serialize};

/// Which side of a CRDT element the cursor sits on.
#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bind {
    Left,
    Right,
}
