use editor_macros::ffi;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum StateField {
    Doc,
    Selection,
    Cursor,
    PageSizes,
    Modifiers,
}
