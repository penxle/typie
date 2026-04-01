use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

pub use crate::utils::{LengthUnit, convert_length};

const STYLE_CACHE_LIMIT: usize = 512;

thread_local! {
    static STYLE_CACHE: RefCell<HashMap<String, Arc<HashMap<String, String>>>> = RefCell::new(HashMap::new());
}

pub fn parse_styles(style: &str) -> Arc<HashMap<String, String>> {
    STYLE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(parsed) = cache.get(style) {
            return Arc::clone(parsed);
        }

        let parsed = Arc::new(
            style
                .split(';')
                .filter_map(|p| p.split_once(':'))
                .map(|(k, v)| (k.trim().to_lowercase(), v.trim().into()))
                .collect(),
        );

        if cache.len() >= STYLE_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(style.to_string(), Arc::clone(&parsed));
        parsed
    })
}

pub fn parse_font_size(s: &str) -> Option<f32> {
    let s = s.trim();
    s.strip_suffix("pt")
        .and_then(|v| v.trim().parse().ok())
        .or_else(|| {
            s.strip_suffix("px")
                .and_then(|v| v.trim().parse::<f32>().ok())
                .map(|v| v * 0.75)
        })
        .or_else(|| s.parse().ok())
}

pub fn parse_as(s: &str, target: LengthUnit) -> Option<f32> {
    let s = s.trim();

    let (value, source) = if let Some(v) = s.strip_suffix("px") {
        (v.trim().parse::<f32>().ok()?, LengthUnit::Px)
    } else if let Some(v) = s.strip_suffix("pt") {
        (v.trim().parse::<f32>().ok()?, LengthUnit::Pt)
    } else if let Some(v) = s.strip_suffix("em") {
        (v.trim().parse::<f32>().ok()?, LengthUnit::Em)
    } else {
        return s.parse().ok();
    };

    Some(convert_length(value, source, target))
}
