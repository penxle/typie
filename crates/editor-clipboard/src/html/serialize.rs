use crate::slice::Slice;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use editor_model::{Fragment, Modifier, PlainNode};
use editor_resource::Resource;

pub fn to_html(slice: &Slice, resource: &Resource) -> String {
    let mut out = String::new();
    out.push_str(r#"<meta charset="utf-8">"#);
    let meta_json = serde_json::to_string(slice).expect("Slice serde");
    let meta_b64 = STANDARD.encode(meta_json.as_bytes());
    out.push_str(&format!(
        r#"<meta data-slice-v2="{meta_b64}" data-version="1">"#,
    ));
    out.push_str("<div data-root>");
    for fragment in &slice.content {
        serialize_node(fragment, resource, &mut out);
    }
    out.push_str("</div>");
    out
}

fn serialize_node(fragment: &Fragment, resource: &Resource, out: &mut String) {
    match &fragment.node {
        PlainNode::Text(t) => serialize_text(&t.text, &fragment.modifiers, resource, out),
        PlainNode::HardBreak(_) => out.push_str("<br>"),
        PlainNode::Tab(_) => out.push('\t'),
        PlainNode::Paragraph(_) => wrap("p", fragment, resource, out, None),
        PlainNode::BulletList(_) => wrap("ul", fragment, resource, out, None),
        PlainNode::OrderedList(_) => wrap("ol", fragment, resource, out, None),
        PlainNode::ListItem(_) => wrap("li", fragment, resource, out, None),
        PlainNode::Blockquote(b) => wrap(
            "blockquote",
            fragment,
            resource,
            out,
            Some(format!(r#"data-variant="{}""#, variant_str(&b.variant))),
        ),
        PlainNode::Callout(c) => wrap(
            "aside",
            fragment,
            resource,
            out,
            Some(format!(
                r#"data-callout data-variant="{}""#,
                variant_str(&c.variant)
            )),
        ),
        PlainNode::Fold(_) => wrap("details", fragment, resource, out, None),
        PlainNode::FoldTitle(_) => wrap("summary", fragment, resource, out, None),
        PlainNode::FoldContent(_) => {
            for child in &fragment.children {
                serialize_node(child, resource, out);
            }
        }
        PlainNode::Table(t) => wrap(
            "table",
            fragment,
            resource,
            out,
            Some(format!(
                r#"data-border-style="{}" data-proportion="{}""#,
                variant_str(&t.border_style),
                t.proportion,
            )),
        ),
        PlainNode::TableRow(_) => wrap("tr", fragment, resource, out, None),
        PlainNode::TableCell(c) => {
            let attrs = c.col_width.map(|w| format!(r#"data-col-width="{w}""#));
            wrap("td", fragment, resource, out, attrs);
        }
        PlainNode::Image(i) => {
            out.push_str(&format!(
                r#"<img data-id="{}" data-proportion="{}">"#,
                html_escape(i.id.as_deref().unwrap_or("")),
                i.proportion,
            ));
        }
        PlainNode::Embed(e) => {
            out.push_str(&format!(
                r#"<a data-embed data-id="{}"></a>"#,
                html_escape(e.id.as_deref().unwrap_or("")),
            ));
        }
        PlainNode::File(f) => {
            out.push_str(&format!(
                r#"<a data-file data-id="{}"></a>"#,
                html_escape(f.id.as_deref().unwrap_or("")),
            ));
        }
        PlainNode::Archived(_) => {}
        PlainNode::PageBreak(_) => out.push_str(r#"<div style="page-break-after:always"></div>"#),
        PlainNode::HorizontalRule(_) => out.push_str("<hr>"),
        PlainNode::Root(_) => {
            for child in &fragment.children {
                serialize_node(child, resource, out);
            }
        }
        PlainNode::Unknown => {}
    }
}

fn wrap(
    tag: &str,
    fragment: &Fragment,
    resource: &Resource,
    out: &mut String,
    attrs: Option<String>,
) {
    match attrs {
        Some(a) => out.push_str(&format!("<{tag} {a}>")),
        None => out.push_str(&format!("<{tag}>")),
    }
    for c in &fragment.children {
        serialize_node(c, resource, out);
    }
    out.push_str(&format!("</{tag}>"));
}

// 변형 enum 들은 #[serde(rename_all = "snake_case")] 의 plain string 직렬화를 가정
fn variant_str<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|val| val.as_str().map(String::from))
        .unwrap_or_default()
}

fn serialize_text(text: &str, modifiers: &[Modifier], resource: &Resource, out: &mut String) {
    let escaped = html_escape(text);

    let (structural, style_pairs) = split_modifiers(modifiers, resource);
    let mut open_tags: Vec<String> = Vec::new();
    let mut close_tags: Vec<String> = Vec::new();

    for m in &structural {
        let (open, close) = open_close_for(m);
        open_tags.push(open);
        close_tags.push(close);
    }

    for t in &open_tags {
        out.push_str(t);
    }
    if !style_pairs.is_empty() {
        out.push_str(&format!(r#"<span style="{}">"#, style_pairs.join(";")));
    }
    out.push_str(&escaped);
    if !style_pairs.is_empty() {
        out.push_str("</span>");
    }
    for t in close_tags.iter().rev() {
        out.push_str(t);
    }
}

fn structural_order(m: &Modifier) -> u8 {
    match m {
        Modifier::Bold => 0,
        Modifier::Italic => 1,
        Modifier::Underline => 2,
        Modifier::Strikethrough => 3,
        Modifier::Link { .. } => 4,
        _ => u8::MAX,
    }
}

fn css_color(value: &str, token_prefix: &str, resource: &Resource) -> String {
    if value == "none" {
        return "transparent".to_string();
    }
    match resource.theme.try_color(&format!("{token_prefix}.{value}")) {
        Some(c) => format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
        None => value.to_string(),
    }
}

fn split_modifiers<'m>(
    mods: &'m [Modifier],
    resource: &Resource,
) -> (Vec<&'m Modifier>, Vec<String>) {
    let mut structural: Vec<&Modifier> = vec![];
    let mut style: Vec<String> = vec![];
    for m in mods {
        match m {
            Modifier::Bold
            | Modifier::Italic
            | Modifier::Underline
            | Modifier::Strikethrough
            | Modifier::Link { .. } => structural.push(m),
            Modifier::FontSize { value } => {
                style.push(format!("font-size:{}pt", *value as f32 / 100.0))
            }
            Modifier::FontFamily { value } => style.push(format!("font-family:{value}")),
            Modifier::FontWeight { value } => style.push(format!("font-weight:{value}")),
            Modifier::TextColor { value } => {
                style.push(format!("color:{}", css_color(value, "text", resource)))
            }
            Modifier::BackgroundColor { value } => style.push(format!(
                "background-color:{}",
                css_color(value, "bg", resource)
            )),
            Modifier::LetterSpacing { value } => {
                style.push(format!("letter-spacing:{}em", *value as f32 / 100.0))
            }
            _ => {}
        }
    }
    structural.sort_by_key(|m| structural_order(m));
    (structural, style)
}

fn open_close_for(m: &Modifier) -> (String, String) {
    match m {
        Modifier::Bold => ("<strong>".into(), "</strong>".into()),
        Modifier::Italic => ("<em>".into(), "</em>".into()),
        Modifier::Underline => ("<u>".into(), "</u>".into()),
        Modifier::Strikethrough => ("<s>".into(), "</s>".into()),
        Modifier::Link { href } => (
            format!(r#"<a href="{}">"#, html_escape(href)),
            "</a>".into(),
        ),
        _ => (String::new(), String::new()),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::Slice;
    use crate::test_doc::DocBuilder;
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::{
        Fragment, Modifier, NodeType, PlainNode, PlainParagraphNode, PlainTextNode,
    };
    use editor_state::{Position, Selection};

    #[test]
    fn serialize_empty_slice_with_meta() {
        let slice = Slice::new(vec![], 0, 0);
        let html = to_html(&slice, &Resource::new_test());
        assert!(html.contains("data-slice-v2="));
        assert!(html.contains("data-version=\"1\""));
        assert!(html.contains("<div data-root>"));
        assert!(html.contains("</div>"));
    }

    #[test]
    fn serialize_prepends_charset_meta() {
        let slice = Slice {
            fragment: Fragment::leaf(PlainNode::Root(PlainRootNode::default())),
            open_start: 0,
            open_end: 0,
        };
        let html = to_html(&slice, &Resource::new_test());
        assert!(html.starts_with(r#"<meta charset="utf-8">"#));
    }

    #[test]
    fn serialize_paragraph_with_text() {
        let (s, ..) = state! {
            doc { r: root { paragraph { text("Hello") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let slice = Slice::extract(&s).unwrap();
        let html = slice.to_html(&Resource::new_test());
        assert!(html.contains("<p>Hello</p>"));
    }

    #[test]
    fn serialize_text_with_bold_and_italic() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: "bold italic".into(),
                    }))
                    .with_modifiers(vec![Modifier::Bold, Modifier::Italic]),
                ],
            }],
            open_start: 0,
            open_end: 0,
        };
        let html = slice.to_html(&Resource::new_test());
        assert!(html.contains("<strong><em>bold italic</em></strong>"));
    }

    #[test]
    fn serialize_text_with_style_modifiers() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: "styled".into(),
                    }))
                    .with_modifiers(vec![
                        Modifier::FontSize { value: 1600 },
                        Modifier::TextColor {
                            value: "#ff0000".into(),
                        },
                    ]),
                ],
            }],
            open_start: 0,
            open_end: 0,
        };
        let html = slice.to_html(&Resource::new_test());
        assert!(
            html.contains(r#"<span style="font-size:16pt;color:#ff0000">styled</span>"#),
            "actual: {html}"
        );
    }

    #[test]
    fn serialize_palette_keys_resolve_to_theme_hex() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: "colored".into(),
                    }))
                    .with_modifiers(vec![
                        Modifier::TextColor {
                            value: "red".into(),
                        },
                        Modifier::BackgroundColor {
                            value: "yellow".into(),
                        },
                    ]),
                ],
            }],
            open_start: 0,
            open_end: 0,
        };
        let html = slice.to_html(&Resource::new_test());
        assert!(
            html.contains(r#"<span style="color:#ef4444;background-color:#fef3c7">colored</span>"#),
            "actual: {html}"
        );
    }

    #[test]
    fn serialize_background_none_resolves_to_transparent() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: "plain".into(),
                    }))
                    .with_modifiers(vec![Modifier::BackgroundColor {
                        value: "none".into(),
                    }]),
                ],
            }],
            open_start: 0,
            open_end: 0,
        };
        let html = slice.to_html(&Resource::new_test());
        assert!(
            html.contains(r#"background-color:transparent"#),
            "actual: {html}"
        );
    }

    #[test]
    fn serialize_text_with_link() {
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: "click".into(),
                    }))
                    .with_modifiers(vec![Modifier::Link {
                        href: "https://example.com".into(),
                    }]),
                ],
            }],
            open_start: 0,
            open_end: 0,
        };
        assert!(
            slice
                .to_html(&Resource::new_test())
                .contains(r#"<a href="https://example.com">click</a>"#)
        );
    }

    #[test]
    fn serialize_bullet_list() {
        let (s, ..) = state! {
            doc { r: root {
                bullet_list {
                    list_item { paragraph { text("a") } }
                    list_item { paragraph { text("b") } }
                }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let slice = Slice::extract(&s).unwrap();
        let html = slice.to_html(&Resource::new_test());
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>"));
        assert!(html.contains("<p>a</p>"));
    }

    #[test]
    fn serialize_table() {
        let (s, ..) = state! {
            doc { r: root {
                table {
                    table_row {
                        table_cell { paragraph { text("a") } }
                        table_cell { paragraph { text("b") } }
                    }
                }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let html = Slice::extract(&s).unwrap().to_html(&Resource::new_test());
        assert!(html.contains("<table"));
        assert!(html.contains("<tr>"));
        assert!(html.contains("<td"));
        assert!(
            html.contains(r#"data-border-style="solid""#),
            "actual: {html}"
        );
    }

    #[test]
    fn serialize_image() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        b.image(&[root]);
        let s = b.finish(Some(Selection::new(
            Position::new(root, 0),
            Position::new(root, 1),
        )));
        let html = Slice::extract(&s).unwrap().to_html(&Resource::new_test());
        assert!(html.contains("<img data-id"));
    }

    #[test]
    fn serialize_horizontal_rule() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        let _p1 = b.block(NodeType::Paragraph, &[root]);
        b.text("a");
        b.horizontal_rule(&[root]);
        let _p2 = b.block(NodeType::Paragraph, &[root]);
        b.text("b");
        let s = b.finish(Some(Selection::new(
            Position::new(root, 1),
            Position::new(root, 2),
        )));
        let html = Slice::extract(&s).unwrap().to_html(&Resource::new_test());
        assert!(html.contains("<hr>"));
    }
}
