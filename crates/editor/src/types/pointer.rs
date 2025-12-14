use serde::Serialize;
use tsify::Tsify;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Tsify)]
#[serde(rename_all = "snake_case")]
pub enum PointerStyle {
    Default,
    Text,
    Pointer,
}
