use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FoldContentNode {}
