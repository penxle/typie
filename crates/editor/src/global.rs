use crate::font::LazyFont;
use crate::runtime::text_replacement::{
    CompiledPattern, RawTextReplacementRule, TextReplacementRule,
};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    pub static GLOBALS: RefCell<Globals> = RefCell::new(Globals::new());
}

pub struct Globals {
    pub parley_layout_context: RefCell<parley::LayoutContext<String>>,
    pub parley_font_context: RefCell<parley::FontContext>,
    pub text_replacement_rules: RefCell<Vec<TextReplacementRule>>,
    pub lazy_fonts: RefCell<HashMap<(String, u16), LazyFont>>,
}

impl Globals {
    pub fn new() -> Self {
        Self {
            parley_layout_context: RefCell::new(parley::LayoutContext::new()),
            parley_font_context: RefCell::new(parley::FontContext::new()),
            lazy_fonts: RefCell::new(HashMap::new()),
            text_replacement_rules: RefCell::new(Vec::new()),
        }
    }
}

pub fn set_text_replacement_rules(raw_rules: Vec<RawTextReplacementRule>) {
    let compiled: Vec<TextReplacementRule> = raw_rules
        .into_iter()
        .filter(|r| !r.match_pattern.is_empty())
        .filter(|r| !r.substitute.is_empty())
        .filter(|r| r.match_pattern != r.substitute)
        .filter_map(|r| {
            let pattern = if r.regex {
                let anchored = format!("(?:{})$", r.match_pattern);
                match fancy_regex::Regex::new(&anchored) {
                    Ok(re) => CompiledPattern::Regex(re),
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
        .collect();

    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        *globals.text_replacement_rules.borrow_mut() = compiled;
    });
}

pub fn with_text_replacement_rules<F, R>(f: F) -> R
where
    F: FnOnce(&[TextReplacementRule]) -> R,
{
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        let rules = globals.text_replacement_rules.borrow();
        f(&rules)
    })
}

pub fn clear_text_replacement_rules() {
    GLOBALS.with(|globals| {
        let globals = globals.borrow();
        globals.text_replacement_rules.borrow_mut().clear();
    });
}
