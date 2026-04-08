use crate::model::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawTextReplacementRule {
    pub(crate) id: String,
    pub(crate) match_pattern: String,
    pub(crate) substitute: String,
    pub(crate) regex: bool,
}

pub(crate) enum CompiledPattern {
    Plain(String),
    Regex(fancy_regex::Regex),
}

#[allow(dead_code)]
pub(crate) struct TextReplacementRule {
    pub(crate) id: String,
    pub(crate) pattern: CompiledPattern,
    pub(crate) substitute: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplacementUndoState {
    pub(crate) node_id: NodeId,
    pub(crate) offset: usize,
    pub(crate) original_text: String,
    pub(crate) replaced_text: String,
    pub(crate) original_offset_len: usize,
    pub(crate) replaced_offset_len: usize,
}
