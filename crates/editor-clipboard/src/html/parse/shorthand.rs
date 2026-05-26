use crate::html::parse::stylesheet::Declaration;
use cssparser::{Parser, ParserInput, Token};

pub fn expand_shorthands(decls: &mut Vec<Declaration>) {
    expand_background(decls);
    expand_font(decls);
}

fn expand_background(decls: &mut Vec<Declaration>) {
    let Some(idx) = decls.iter().position(|d| d.property == "background") else {
        return;
    };
    let bg = decls[idx].clone();
    let color = extract_color_token(&bg.value);
    decls.remove(idx);
    if let Some(c) = color
        && !decls.iter().any(|d| d.property == "background-color")
    {
        decls.push(Declaration {
            property: "background-color".into(),
            value: c,
            important: bg.important,
        });
    }
}

fn expand_font(decls: &mut Vec<Declaration>) {
    let Some(idx) = decls.iter().position(|d| d.property == "font") else {
        return;
    };
    let font = decls[idx].clone();
    decls.remove(idx);

    let Some(parsed) = parse_font_shorthand(&font.value) else {
        return;
    };

    push_if_absent(decls, "font-style", parsed.style, font.important);
    push_if_absent(decls, "font-weight", parsed.weight, font.important);
    push_if_absent(decls, "font-size", Some(parsed.size), font.important);
    push_if_absent(decls, "font-family", Some(parsed.family), font.important);
}

struct ParsedFont {
    style: Option<String>,
    weight: Option<String>,
    size: String,
    family: String,
}

fn parse_font_shorthand(value: &str) -> Option<ParsedFont> {
    let trimmed = value.trim().to_lowercase();
    if matches!(
        trimmed.as_str(),
        "caption" | "icon" | "menu" | "message-box" | "small-caption" | "status-bar"
    ) {
        return None;
    }

    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);

    let mut tokens: Vec<TokenInfo> = Vec::new();
    while let Ok(token) = parser.next_including_whitespace() {
        let token_str = match token {
            Token::Ident(s) => TokenInfo::Ident(s.to_string()),
            Token::Number { value, .. } => TokenInfo::Number(*value),
            Token::Dimension { value, unit, .. } => {
                TokenInfo::Dimension(format!("{}{}", *value, unit.as_ref()))
            }
            Token::Percentage { unit_value, .. } => {
                TokenInfo::Percentage(format!("{}%", *unit_value * 100.0))
            }
            Token::QuotedString(s) => TokenInfo::QuotedString(s.to_string()),
            Token::Delim('/') => TokenInfo::Slash,
            Token::Comma => TokenInfo::Comma,
            Token::WhiteSpace(_) => continue,
            _ => return None,
        };
        tokens.push(token_str);
    }

    let size_idx = tokens
        .iter()
        .position(|t| matches!(t, TokenInfo::Dimension(_) | TokenInfo::Percentage(_)))?;

    let family_start =
        if size_idx + 2 < tokens.len() && matches!(tokens[size_idx + 1], TokenInfo::Slash) {
            size_idx + 3
        } else {
            size_idx + 1
        };

    let size = match &tokens[size_idx] {
        TokenInfo::Dimension(s) => s.clone(),
        TokenInfo::Percentage(s) => s.clone(),
        _ => return None,
    };

    let family_tokens = &tokens[family_start..];
    if family_tokens.is_empty() {
        return None;
    }
    let mut family_parts: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for t in family_tokens {
        match t {
            TokenInfo::Ident(s) => current.push(s.clone()),
            TokenInfo::QuotedString(s) => current.push(format!(r#""{}""#, s)),
            TokenInfo::Comma => {
                if current.is_empty() {
                    return None;
                }
                family_parts.push(current.join(" "));
                current.clear();
            }
            _ => return None,
        }
    }
    if !current.is_empty() {
        family_parts.push(current.join(" "));
    }
    if family_parts.is_empty() {
        return None;
    }
    let family = family_parts.join(", ");

    let head_end = size_idx;
    let mut style: Option<String> = None;
    let mut weight: Option<String> = None;
    for t in &tokens[..head_end] {
        match t {
            TokenInfo::Ident(s) => {
                let lower = s.to_lowercase();
                match lower.as_str() {
                    "normal" => {}
                    "italic" | "oblique" => {
                        if style.is_some() {
                            return None;
                        }
                        style = Some(lower);
                    }
                    "bold" | "bolder" | "lighter" => {
                        if weight.is_some() {
                            return None;
                        }
                        weight = Some(lower);
                    }
                    "small-caps" => {}
                    "ultra-condensed" | "extra-condensed" | "condensed" | "semi-condensed"
                    | "semi-expanded" | "expanded" | "extra-expanded" | "ultra-expanded" => {}
                    _ => return None,
                }
            }
            TokenInfo::Number(n) => {
                if weight.is_some() {
                    return None;
                }
                weight = Some(format!("{}", *n as u16));
            }
            _ => return None,
        }
    }

    Some(ParsedFont {
        style,
        weight,
        size,
        family,
    })
}

#[derive(Debug)]
enum TokenInfo {
    Ident(String),
    Number(f32),
    Dimension(String),
    Percentage(String),
    QuotedString(String),
    Slash,
    Comma,
}

fn push_if_absent(
    decls: &mut Vec<Declaration>,
    property: &str,
    value: Option<String>,
    important: bool,
) {
    let Some(v) = value else { return };
    if decls.iter().any(|d| d.property == property) {
        return;
    }
    decls.push(Declaration {
        property: property.into(),
        value: v,
        important,
    });
}

fn extract_color_token(value: &str) -> Option<String> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    loop {
        match parser.next_including_whitespace_and_comments() {
            Ok(Token::Hash(s)) | Ok(Token::IDHash(s)) => return Some(format!("#{}", s.as_ref())),
            Ok(Token::Ident(s)) if is_color_keyword(s.as_ref()) => return Some(s.to_string()),
            Ok(Token::Function(name))
                if matches!(name.as_ref(), "rgb" | "rgba" | "hsl" | "hsla") =>
            {
                let fname = name.to_string();
                let inner = parser
                    .parse_nested_block::<_, _, ()>(|p| {
                        let start = p.position();
                        while p.next_including_whitespace_and_comments().is_ok() {}
                        Ok(p.slice_from(start).to_string())
                    })
                    .ok()?;
                return Some(format!("{}({})", fname, inner));
            }
            Ok(_) => continue,
            Err(_) => return None,
        }
    }
}

fn is_color_keyword(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "red"
            | "blue"
            | "green"
            | "yellow"
            | "orange"
            | "purple"
            | "pink"
            | "black"
            | "white"
            | "gray"
            | "grey"
            | "brown"
            | "cyan"
            | "magenta"
            | "lime"
            | "navy"
            | "teal"
            | "olive"
            | "maroon"
            | "silver"
            | "gold"
            | "transparent"
            | "currentcolor"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dec(p: &str, v: &str) -> Declaration {
        Declaration {
            property: p.into(),
            value: v.into(),
            important: false,
        }
    }
    #[test]
    fn keyword_only() {
        let mut d = vec![dec("background", "yellow")];
        expand_shorthands(&mut d);
        assert!(
            d.iter()
                .any(|x| x.property == "background-color" && x.value == "yellow")
        );
        assert!(!d.iter().any(|x| x.property == "background"));
    }
    #[test]
    fn image_and_color() {
        let mut d = vec![dec("background", "url(x.png) yellow no-repeat")];
        expand_shorthands(&mut d);
        assert!(
            d.iter()
                .any(|x| x.property == "background-color" && x.value == "yellow")
        );
    }
    #[test]
    fn no_color() {
        let mut d = vec![dec("background", "url(x.png) no-repeat")];
        expand_shorthands(&mut d);
        assert!(!d.iter().any(|x| x.property == "background-color"));
    }
    #[test]
    fn existing_bgcolor_wins() {
        let mut d = vec![dec("background-color", "red"), dec("background", "yellow")];
        expand_shorthands(&mut d);
        let bg = d.iter().find(|x| x.property == "background-color").unwrap();
        assert_eq!(bg.value, "red");
    }
    #[test]
    fn hex_color() {
        let mut d = vec![dec("background", "#fff")];
        expand_shorthands(&mut d);
        assert!(
            d.iter()
                .any(|x| x.property == "background-color" && x.value == "#fff")
        );
    }
    #[test]
    fn rgb_function() {
        let mut d = vec![dec("background", "rgb(255, 0, 0)")];
        expand_shorthands(&mut d);
        let bg = d
            .iter()
            .find(|x| x.property == "background-color")
            .expect("background-color expected");
        assert!(bg.value.starts_with("rgb("), "got: {:?}", bg.value);
    }
    #[test]
    fn rgba_function() {
        let mut d = vec![dec("background", "rgba(0,0,0,0.5)")];
        expand_shorthands(&mut d);
        let bg = d
            .iter()
            .find(|x| x.property == "background-color")
            .expect("background-color expected");
        assert!(bg.value.starts_with("rgba("), "got: {:?}", bg.value);
    }
    #[test]
    fn hsl_function() {
        let mut d = vec![dec("background", "hsl(0, 100%, 50%)")];
        expand_shorthands(&mut d);
        let bg = d
            .iter()
            .find(|x| x.property == "background-color")
            .expect("background-color expected");
        assert!(bg.value.starts_with("hsl("), "got: {:?}", bg.value);
    }

    #[test]
    fn font_full_with_style_weight_size_family() {
        let mut d = vec![dec("font", r#"italic bold 16px/24px "Arial", sans-serif"#)];
        expand_shorthands(&mut d);
        assert!(
            d.iter()
                .any(|x| x.property == "font-style" && x.value == "italic"),
            "font-style longhand expected"
        );
        assert!(
            d.iter()
                .any(|x| x.property == "font-weight" && x.value == "bold"),
            "font-weight longhand expected"
        );
        assert!(
            d.iter()
                .any(|x| x.property == "font-size" && x.value == "16px"),
            "font-size longhand expected"
        );
        assert!(
            d.iter()
                .any(|x| x.property == "font-family" && x.value.contains("Arial")),
            "font-family longhand expected"
        );
        assert!(
            !d.iter().any(|x| x.property == "font"),
            "font shorthand must be removed"
        );
        assert!(
            !d.iter().any(|x| x.property == "line-height"),
            "line-height must be ignored"
        );
    }

    #[test]
    fn font_minimal_size_family_only() {
        let mut d = vec![dec("font", r#"12pt "Times New Roman""#)];
        expand_shorthands(&mut d);
        assert!(
            d.iter()
                .any(|x| x.property == "font-size" && x.value == "12pt")
        );
        assert!(
            d.iter()
                .any(|x| x.property == "font-family" && x.value.contains("Times New Roman"))
        );
        assert!(!d.iter().any(|x| x.property == "font-style"));
        assert!(!d.iter().any(|x| x.property == "font-weight"));
    }

    #[test]
    fn font_system_keyword_dropped() {
        let mut d = vec![dec("font", "caption")];
        expand_shorthands(&mut d);
        assert!(!d.iter().any(|x| x.property == "font"));
        assert!(!d.iter().any(|x| x.property == "font-size"));
        assert!(!d.iter().any(|x| x.property == "font-family"));
    }

    #[test]
    fn font_existing_longhand_preserved() {
        let mut d = vec![dec("font-weight", "300"), dec("font", "italic 16pt Arial")];
        expand_shorthands(&mut d);
        let fw = d
            .iter()
            .find(|x| x.property == "font-weight")
            .expect("font-weight expected");
        assert_eq!(fw.value, "300");
        assert!(
            d.iter()
                .any(|x| x.property == "font-style" && x.value == "italic")
        );
        assert!(
            d.iter()
                .any(|x| x.property == "font-size" && x.value == "16pt")
        );
    }

    #[test]
    fn font_invalid_dropped_entirely() {
        let mut d = vec![dec("font", "Arial")];
        expand_shorthands(&mut d);
        assert!(!d.iter().any(|x| x.property == "font"));
        assert!(!d.iter().any(|x| x.property == "font-family"));
    }

    #[test]
    fn font_keyword_size_dropped() {
        let mut d = vec![dec("font", "medium Arial")];
        expand_shorthands(&mut d);
        assert!(!d.iter().any(|x| x.property == "font"));
        assert!(!d.iter().any(|x| x.property == "font-size"));
        assert!(!d.iter().any(|x| x.property == "font-family"));
    }

    #[test]
    fn font_with_line_height_normal_ignored() {
        let mut d = vec![dec("font", "italic 16px/normal Arial")];
        expand_shorthands(&mut d);
        assert!(
            d.iter()
                .any(|x| x.property == "font-style" && x.value == "italic")
        );
        assert!(
            d.iter()
                .any(|x| x.property == "font-size" && x.value == "16px")
        );
        assert!(
            d.iter()
                .any(|x| x.property == "font-family" && x.value.contains("Arial"))
        );
        assert!(!d.iter().any(|x| x.property == "line-height"));
    }
}
