use crate::error::{Pos, XmlError, XmlErrorDetail};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Open {
        name: String,
        attrs: Vec<(String, String, Pos)>,
        self_closing: bool,
        pos: Pos,
    },
    Close {
        name: String,
        pos: Pos,
    },
    Text {
        text: String,
        pos: Pos,
    },
}

struct Cursor<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: u32,
    column: u32,
}

impl Cursor<'_> {
    fn pos(&self) -> Pos {
        Pos {
            line: self.line,
            column: self.column,
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.chars.next()?;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n' | '\r' | '\t')) {
            self.bump();
        }
    }
}

fn is_name_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == ':'
}

fn is_name_char(ch: char) -> bool {
    is_name_start(ch) || ch.is_ascii_digit() || ch == '-' || ch == '.'
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, XmlError> {
    let mut cur = Cursor {
        chars: input.chars().peekable(),
        line: 1,
        column: 1,
    };
    let mut out = Vec::new();
    let mut open_names: Vec<String> = Vec::new();
    loop {
        let pos = cur.pos();
        match cur.peek() {
            None => break,
            Some('<') => {
                cur.bump();
                match cur.peek() {
                    Some('?') => return Err(XmlError::at(pos, XmlErrorDetail::Declaration)),
                    Some('!') => return Err(XmlError::at(pos, XmlErrorDetail::CommentOrDtd)),
                    Some('/') => {
                        cur.bump();
                        let name = read_name(&mut cur)?;
                        cur.skip_ws();
                        if !cur.eat('>') {
                            return Err(XmlError::at(
                                cur.pos(),
                                XmlErrorDetail::CloseTagUnterminated { name },
                            ));
                        }
                        match open_names.pop() {
                            Some(open) if open == name => {}
                            open => {
                                return Err(XmlError::at(
                                    pos,
                                    XmlErrorDetail::CloseWithoutOpen { name, open },
                                ));
                            }
                        }
                        out.push(Token::Close { name, pos });
                    }
                    _ => {
                        let name = read_name(&mut cur)?;
                        let mut attrs: Vec<(String, String, Pos)> = Vec::new();
                        loop {
                            cur.skip_ws();
                            match cur.peek() {
                                Some('/') => {
                                    cur.bump();
                                    if !cur.eat('>') {
                                        return Err(XmlError::at(
                                            cur.pos(),
                                            XmlErrorDetail::SelfCloseUnterminated,
                                        ));
                                    }
                                    out.push(Token::Open {
                                        name,
                                        attrs,
                                        self_closing: true,
                                        pos,
                                    });
                                    break;
                                }
                                Some('>') => {
                                    cur.bump();
                                    open_names.push(name.clone());
                                    out.push(Token::Open {
                                        name,
                                        attrs,
                                        self_closing: false,
                                        pos,
                                    });
                                    break;
                                }
                                Some(ch) if is_name_start(ch) => {
                                    let apos = cur.pos();
                                    let key = read_name(&mut cur)?;
                                    cur.skip_ws();
                                    if !cur.eat('=') {
                                        return Err(XmlError::at(
                                            cur.pos(),
                                            XmlErrorDetail::AttrMissingEquals { attr: key },
                                        ));
                                    }
                                    cur.skip_ws();
                                    let quote = match cur.bump() {
                                        Some(q @ ('"' | '\'')) => q,
                                        _ => {
                                            return Err(XmlError::at(
                                                cur.pos(),
                                                XmlErrorDetail::AttrUnquoted { attr: key },
                                            ));
                                        }
                                    };
                                    let value = read_until(&mut cur, quote, true)?;
                                    if attrs.iter().any(|(k, _, _)| *k == key) {
                                        return Err(XmlError::at(
                                            apos,
                                            XmlErrorDetail::AttrDuplicate { attr: key },
                                        ));
                                    }
                                    attrs.push((key, value, apos));
                                }
                                Some(_) => {
                                    return Err(XmlError::at(
                                        cur.pos(),
                                        XmlErrorDetail::IllegalCharInTag,
                                    ));
                                }
                                None => {
                                    return Err(XmlError::at(
                                        cur.pos(),
                                        XmlErrorDetail::TagUnterminated { name },
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Some(_) => {
                let text = read_until(&mut cur, '<', false)?;
                out.push(Token::Text { text, pos });
            }
        }
    }
    if let Some(name) = open_names.pop() {
        return Err(XmlError::at(
            cur.pos(),
            XmlErrorDetail::ElementUnclosed { name },
        ));
    }
    Ok(out)
}

fn read_name(cur: &mut Cursor<'_>) -> Result<String, XmlError> {
    let pos = cur.pos();
    let mut name = String::new();
    match cur.peek() {
        Some(ch) if is_name_start(ch) => {}
        _ => return Err(XmlError::at(pos, XmlErrorDetail::NameExpected)),
    }
    while let Some(ch) = cur.peek() {
        if !is_name_char(ch) {
            break;
        }
        name.push(ch);
        cur.bump();
    }
    Ok(name)
}

fn read_until(cur: &mut Cursor<'_>, stop: char, consume_stop: bool) -> Result<String, XmlError> {
    let mut out = String::new();
    loop {
        match cur.peek() {
            None if consume_stop => {
                return Err(XmlError::at(cur.pos(), XmlErrorDetail::UnterminatedQuote));
            }
            None => return Ok(out),
            Some(ch) if ch == stop => {
                if consume_stop {
                    cur.bump();
                }
                return Ok(out);
            }
            Some('&') => {
                let pos = cur.pos();
                cur.bump();
                out.push(read_entity(cur, pos)?);
            }
            Some('<') if consume_stop => {
                return Err(XmlError::at(cur.pos(), XmlErrorDetail::LtInAttrValue));
            }
            Some(ch) => {
                if is_forbidden(ch) {
                    return Err(XmlError::at(
                        cur.pos(),
                        XmlErrorDetail::ForbiddenControlChar {
                            codepoint: ch as u32,
                        },
                    ));
                }
                out.push(ch);
                cur.bump();
            }
        }
    }
}

fn read_entity(cur: &mut Cursor<'_>, pos: Pos) -> Result<char, XmlError> {
    let mut body = String::new();
    loop {
        match cur.bump() {
            Some(';') => break,
            Some(ch) if body.len() < 12 && (ch.is_ascii_alphanumeric() || ch == '#') => {
                body.push(ch)
            }
            _ => return Err(XmlError::at(pos, XmlErrorDetail::UnknownEntity)),
        }
    }
    let ch = match body.as_str() {
        "lt" => '<',
        "gt" => '>',
        "amp" => '&',
        "quot" => '"',
        "apos" => '\'',
        b if b.starts_with("#x") => u32::from_str_radix(&b[2..], 16)
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| XmlError::at(pos, XmlErrorDetail::BadNumericReference))?,
        b if b.starts_with('#') => b[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| XmlError::at(pos, XmlErrorDetail::BadNumericReference))?,
        _ => return Err(XmlError::at(pos, XmlErrorDetail::UnknownEntity)),
    };
    if is_forbidden(ch) {
        return Err(XmlError::at(
            pos,
            XmlErrorDetail::ForbiddenControlChar {
                codepoint: ch as u32,
            },
        ));
    }
    Ok(ch)
}

pub(crate) fn is_forbidden(ch: char) -> bool {
    let c = ch as u32;
    (c < 0x20 && c != 0x09 && c != 0x0A && c != 0x0D) || c == 0xFFFE || c == 0xFFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(input: &str) -> Vec<String> {
        tokenize(input)
            .unwrap()
            .into_iter()
            .map(|t| match t {
                Token::Open {
                    name,
                    self_closing,
                    attrs,
                    ..
                } => format!("open:{name}:{}:{}", attrs.len(), self_closing),
                Token::Close { name, .. } => format!("close:{name}"),
                Token::Text { text, .. } => format!("text:{text}"),
            })
            .collect()
    }

    #[test]
    fn tokenizes_elements_attributes_entities_and_positions() {
        let toks = tokenize("<a x=\"1&amp;2\" attr:y='q'>t&lt;\n</a><b/>").unwrap();
        let Token::Open { attrs, pos, .. } = &toks[0] else {
            panic!()
        };
        assert_eq!(
            attrs[0],
            ("x".into(), "1&2".into(), Pos { line: 1, column: 4 })
        );
        assert_eq!(attrs[1].0, "attr:y");
        assert_eq!(*pos, Pos { line: 1, column: 1 });
        assert_eq!(
            kinds("<a x=\"1&amp;2\" attr:y='q'>t&lt;\n</a><b/>"),
            vec!["open:a:2:false", "text:t<\n", "close:a", "open:b:0:true"]
        );
        let Token::Close { pos, .. } = &toks[2] else {
            panic!()
        };
        assert_eq!(*pos, Pos { line: 2, column: 1 });
    }

    #[test]
    fn rejects_declaration_comment_cdata_pi_dtd_and_unknown_entity() {
        let cases = [
            ("<?xml version=\"1.0\"?><a/>", XmlErrorDetail::Declaration),
            ("<!-- c --><a/>", XmlErrorDetail::CommentOrDtd),
            ("<a><![CDATA[x]]></a>", XmlErrorDetail::CommentOrDtd),
            ("<!DOCTYPE a><a/>", XmlErrorDetail::CommentOrDtd),
            ("<a>&nbsp;</a>", XmlErrorDetail::UnknownEntity),
            ("<a>&#xD800;</a>", XmlErrorDetail::BadNumericReference),
            (
                "<a>x</b>",
                XmlErrorDetail::CloseWithoutOpen {
                    name: "b".into(),
                    open: Some("a".into()),
                },
            ),
            (
                "<a x=1/>",
                XmlErrorDetail::AttrUnquoted { attr: "x".into() },
            ),
            (
                "<a x=\"1\" x=\"2\"/>",
                XmlErrorDetail::AttrDuplicate { attr: "x".into() },
            ),
            ("<a>", XmlErrorDetail::ElementUnclosed { name: "a".into() }),
        ];
        for (bad, detail) in cases {
            let err = tokenize(bad).unwrap_err();
            assert!(err.pos.is_some(), "{bad}");
            assert_eq!(*err.detail, detail, "{bad}");
        }
    }

    #[test]
    fn rejects_malformed_tags_attributes_and_control_characters() {
        let cases = [
            (
                "<a x/>",
                XmlErrorDetail::AttrMissingEquals { attr: "x".into() },
            ),
            ("<a x=\"1\"/ >", XmlErrorDetail::SelfCloseUnterminated),
            (
                "<a>x</a",
                XmlErrorDetail::CloseTagUnterminated { name: "a".into() },
            ),
            ("<a", XmlErrorDetail::TagUnterminated { name: "a".into() }),
            ("<1a/>", XmlErrorDetail::NameExpected),
            ("<a x=\"1/>", XmlErrorDetail::UnterminatedQuote),
            ("<a x=\"<\"/>", XmlErrorDetail::LtInAttrValue),
            ("<a @b=\"1\"/>", XmlErrorDetail::IllegalCharInTag),
            (
                "<a>\u{0}</a>",
                XmlErrorDetail::ForbiddenControlChar { codepoint: 0 },
            ),
            (
                "<a>&#0;</a>",
                XmlErrorDetail::ForbiddenControlChar { codepoint: 0 },
            ),
        ];
        for (bad, detail) in cases {
            let err = tokenize(bad).unwrap_err();
            assert!(err.pos.is_some(), "{bad:?}");
            assert_eq!(*err.detail, detail, "{bad:?}");
        }
    }

    #[test]
    fn numeric_references_and_bare_gt_and_quotes_in_text() {
        assert_eq!(
            kinds("<a>&#44032;&#xAC00;>\"'</a>"),
            vec!["open:a:0:false", "text:가가>\"'", "close:a"]
        );
    }
}
