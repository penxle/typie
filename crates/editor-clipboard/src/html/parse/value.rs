pub fn parse_length_to_pt_hundredths(value: &str) -> Option<u32> {
    let v = value.trim();
    if let Some(n) = v.strip_suffix("pt").and_then(|s| s.parse::<f32>().ok()) {
        return Some((n * 100.0) as u32);
    }
    if let Some(n) = v.strip_suffix("px").and_then(|s| s.parse::<f32>().ok()) {
        return Some((n * 75.0) as u32);
    }
    if let Some(n) = v.strip_suffix("em").and_then(|s| s.parse::<f32>().ok()) {
        return Some((n * 1200.0) as u32);
    }
    None
}

pub fn parse_letter_spacing_to_em_hundredths(value: &str) -> Option<i32> {
    let v = value.trim();
    if let Some(n) = v.strip_suffix("em").and_then(|s| s.parse::<f32>().ok()) {
        return Some((n * 100.0) as i32);
    }
    None
}

pub fn parse_font_weight(value: &str) -> Option<u16> {
    let v = value.trim().to_lowercase();
    match v.as_str() {
        "bold" => Some(700),
        "normal" => Some(400),
        "lighter" => Some(300),
        "bolder" => Some(600),
        s => s.parse::<u16>().ok(),
    }
}

pub fn text_decoration_tokens(value: &str) -> Vec<String> {
    value.split_whitespace().map(|s| s.to_lowercase()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn length_pt() {
        assert_eq!(parse_length_to_pt_hundredths("16pt"), Some(1600));
    }
    #[test]
    fn length_px() {
        assert_eq!(parse_length_to_pt_hundredths("16px"), Some(1200));
    }
    #[test]
    fn length_em() {
        assert_eq!(parse_length_to_pt_hundredths("1em"), Some(1200));
        assert_eq!(parse_length_to_pt_hundredths("1.5em"), Some(1800));
    }
    #[test]
    fn length_unsupported() {
        assert_eq!(parse_length_to_pt_hundredths("100%"), None);
        assert_eq!(parse_length_to_pt_hundredths("1rem"), None);
    }
    #[test]
    fn weight_keyword() {
        assert_eq!(parse_font_weight("bold"), Some(700));
    }
    #[test]
    fn weight_numeric() {
        assert_eq!(parse_font_weight("800"), Some(800));
    }
    #[test]
    fn ls() {
        assert_eq!(parse_letter_spacing_to_em_hundredths("0.05em"), Some(5));
    }
    #[test]
    fn td_multi() {
        let t = text_decoration_tokens("underline line-through");
        assert!(t.contains(&"underline".into()));
        assert!(t.contains(&"line-through".into()));
    }
}
