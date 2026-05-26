use crate::html::parse::stylesheet::Declaration;
use cssparser::{Parser, ParserInput, Token};

pub fn expand_shorthands(decls: &mut Vec<Declaration>) {
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
}
