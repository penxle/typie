use std::sync::Arc;

use editor_macros::ffi;
use fancy_regex::{Assertion, Expr};
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTextReplacementRule {
    pub id: String,
    pub match_pattern: String,
    pub substitute: String,
    pub regex: bool,
}

#[derive(Clone)]
pub enum CompiledPattern {
    Plain(String),
    Regex {
        regex: Arc<fancy_regex::Regex>,
        required_last_char: Option<char>,
        /// Needs the backtracking engine, so matching is not linear in the input.
        backtracks: bool,
    },
}

#[derive(Clone)]
pub struct TextReplacementRule {
    pub id: String,
    pub pattern: CompiledPattern,
    pub substitute: String,
}

impl PartialEq for CompiledPattern {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Plain(left), Self::Plain(right)) => left == right,
            (Self::Regex { regex: left, .. }, Self::Regex { regex: right, .. }) => {
                left.as_str() == right.as_str()
            }
            _ => false,
        }
    }
}

impl Eq for CompiledPattern {}

impl PartialEq for TextReplacementRule {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.pattern == other.pattern && self.substitute == other.substitute
    }
}

impl Eq for TextReplacementRule {}

#[derive(Clone)]
pub struct PreparedTextReplacementRules {
    pub(crate) rules: Arc<[TextReplacementRule]>,
}

pub fn prepare_text_replacement_rules(
    raw_rules: Vec<RawTextReplacementRule>,
) -> PreparedTextReplacementRules {
    PreparedTextReplacementRules {
        rules: compile_rules(raw_rules).into(),
    }
}

fn compile_rules(raw_rules: Vec<RawTextReplacementRule>) -> Vec<TextReplacementRule> {
    raw_rules
        .into_iter()
        .filter_map(|r| {
            let pattern = if r.regex {
                compile_regex(&r.match_pattern)?
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

/// Only a match ending at the caret is ever accepted, so the anchor belongs in
/// the pattern rather than in a filter over candidate matches.
fn compile_regex(pattern: &str) -> Option<CompiledPattern> {
    let anchored = format!("(?:{pattern})\\z");
    let regex = fancy_regex::Regex::new(&anchored).ok()?;
    let expr = Expr::parse_tree(&anchored).ok().map(|tree| tree.expr);

    Some(CompiledPattern::Regex {
        regex: Arc::new(regex),
        required_last_char: expr.as_ref().and_then(required_last_char),
        backtracks: expr.as_ref().is_none_or(backtracks),
    })
}

fn backtracks(expr: &Expr) -> bool {
    match expr {
        Expr::Empty | Expr::Any { .. } | Expr::Literal { .. } | Expr::Delegate { .. } => false,
        Expr::Assertion(assertion) => !matches!(
            assertion,
            Assertion::StartText
                | Assertion::EndText
                | Assertion::StartLine { .. }
                | Assertion::EndLine { .. }
        ),
        Expr::Concat(children) | Expr::Alt(children) => children.iter().any(backtracks),
        Expr::Group(child) => backtracks(child),
        Expr::Repeat { child, .. } => backtracks(child),
        _ => true,
    }
}

fn required_last_char(expr: &Expr) -> Option<char> {
    match expr {
        Expr::Literal { val, casei: false } => val.chars().next_back(),
        Expr::Concat(children) => {
            let last = children.iter().rev().find(|child| !is_zero_width(child))?;
            if matches_empty(last) {
                None
            } else {
                required_last_char(last)
            }
        }
        Expr::Alt(children) => {
            let mut candidates = children.iter().map(required_last_char);
            let first = candidates.next()??;
            candidates.all(|c| c == Some(first)).then_some(first)
        }
        Expr::Group(child) => required_last_char(child),
        Expr::Repeat { child, lo, .. } if *lo > 0 => required_last_char(child),
        _ => None,
    }
}

fn is_zero_width(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Empty
            | Expr::Assertion(_)
            | Expr::LookAround(..)
            | Expr::KeepOut
            | Expr::ContinueFromPreviousMatchEnd
    )
}

fn matches_empty(expr: &Expr) -> bool {
    match expr {
        Expr::Any { .. } | Expr::Delegate { .. } | Expr::GeneralNewline { .. } => false,
        Expr::Literal { val, .. } => val.is_empty(),
        Expr::Concat(children) => children.iter().all(matches_empty),
        Expr::Alt(children) => children.iter().any(matches_empty),
        Expr::Group(child) => matches_empty(child),
        Expr::Repeat { child, lo, .. } => *lo == 0 || matches_empty(child),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compiled(pattern: &str) -> (Option<char>, bool) {
        match compile_regex(pattern).expect("pattern must compile") {
            CompiledPattern::Regex {
                required_last_char,
                backtracks,
                ..
            } => (required_last_char, backtracks),
            CompiledPattern::Plain(_) => unreachable!(),
        }
    }

    #[test]
    fn derives_required_last_char() {
        assert_eq!(compiled("(?<=\u{201C}[^\u{201D}]*)\"").0, Some('"'));
        assert_eq!(compiled(r"\d+#").0, Some('#'));
        assert_eq!(compiled("ab(?=c)").0, Some('b'));
        assert_eq!(compiled("x{2,3}").0, Some('x'));
        assert_eq!(compiled("ab|cb").0, Some('b'));
    }

    #[test]
    fn omits_required_last_char_when_not_guaranteed() {
        assert_eq!(compiled("(?i)abc").0, None);
        assert_eq!(compiled("a|b").0, None);
        assert_eq!(compiled(r"^-\s").0, None);
        assert_eq!(compiled(r"ab\d*").0, None);
    }

    #[test]
    fn detects_backtracking_patterns() {
        assert!(compiled("(?<=\u{201C}[^\u{201D}]*)\"").1);
        assert!(compiled("ab(?=c)").1);
        assert!(compiled(r"(a)\1").1);
        assert!(compiled(r"\bword").1);
    }

    #[test]
    fn detects_linear_patterns() {
        assert!(!compiled(r"\d+#").1);
        assert!(!compiled("(?i)abc").1);
        assert!(!compiled(r"^-\s").1);
        assert!(!compiled("(ab|cd)e").1);
    }
}
