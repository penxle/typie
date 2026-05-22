use std::sync::Arc;

use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
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
    Regex(Arc<fancy_regex::Regex>),
}

pub struct TextReplacementRule {
    pub id: String,
    pub pattern: CompiledPattern,
    pub substitute: String,
}

pub fn compile_rules(raw_rules: Vec<RawTextReplacementRule>) -> Vec<TextReplacementRule> {
    raw_rules
        .into_iter()
        .filter_map(|r| {
            let pattern = if r.regex {
                match fancy_regex::Regex::new(&r.match_pattern) {
                    Ok(re) => CompiledPattern::Regex(Arc::new(re)),
                    Err(_) => return None,
                }
            } else {
                CompiledPattern::Plain(r.match_pattern)
            };
            Some(TextReplacementRule {
                id: r.id,
                pattern,
                substitute: r.substitute,
            })
        })
        .collect()
}
