use std::collections::HashMap;

pub fn parse_styles(style: &str) -> HashMap<String, String> {
    style
        .split(';')
        .filter_map(|p| p.split_once(':'))
        .map(|(k, v)| (k.trim().to_lowercase(), v.trim().into()))
        .collect()
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

pub fn parse_length(s: &str) -> Option<f32> {
    let s = s.trim();
    s.strip_suffix("px")
        .and_then(|v| v.trim().parse().ok())
        .or_else(|| {
            s.strip_suffix("pt")
                .and_then(|v| v.trim().parse::<f32>().ok())
                .map(|v| v * 1.333)
        })
        .or_else(|| s.parse().ok())
}
