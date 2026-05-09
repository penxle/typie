use editor_macros::ffi;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Encode, Decode,
)]
#[cbor(index_only)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    #[default]
    #[n(0)]
    Left,
    #[n(1)]
    Center,
    #[n(2)]
    Right,
    #[n(3)]
    Justify,
}
