use crate::model::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTextReplacementRule {
    pub id: String,
    pub match_pattern: String,
    pub substitute: String,
    pub regex: bool,
}

pub enum CompiledPattern {
    Plain(String),
    Regex(fancy_regex::Regex),
}

#[allow(dead_code)]
pub struct TextReplacementRule {
    pub id: String,
    pub pattern: CompiledPattern,
    pub substitute: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplacementUndoState {
    pub node_id: NodeId,
    pub offset: usize,
    pub original_text: String,
    pub replaced_text: String,
    pub original_offset_len: usize,
    pub replaced_offset_len: usize,
}
