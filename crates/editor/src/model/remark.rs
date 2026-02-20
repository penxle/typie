use serde::{Deserialize, Serialize};

pub type RemarkId = super::NodeId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct Remark {
    pub id: RemarkId,
    pub user_id: String,
    pub text: String,
    pub created_at: i64,
}
