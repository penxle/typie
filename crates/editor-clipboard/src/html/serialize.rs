use crate::slice::Slice;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use editor_model::{Fragment, Modifier, PlainNode};
use editor_resource::Resource;
use serde::Serialize;

pub fn to_html(slice: &Slice, resource: &Resource) -> String {
    let mut out = String::new();
    out.push_str(r#"<meta charset="utf-8">"#);
    let mut meta_json = Vec::new();
    let mut serializer = serde_json::Serializer::new(&mut meta_json);
    slice
        .serialize(serde_stacker::Serializer::new(&mut serializer))
        .expect("Slice serde");
    let meta_b64 = STANDARD.encode(meta_json);
    out.push_str(&format!(
        r#"<meta data-slice-v2="{meta_b64}" data-version="1">"#,
    ));
    out.push_str("<div data-root>");
    serialize_forest(&slice.content, resource, &mut out);
    out.push_str("</div>");
    out
}

enum SerializeTask<'a> {
    Node(&'a Fragment),
    Close(&'static str),
}

fn serialize_forest(fragments: &[Fragment], resource: &Resource, out: &mut String) {
    let mut tasks = Vec::new();
    push_children(&mut tasks, fragments);
    while let Some(task) = tasks.pop() {
        let fragment = match task {
            SerializeTask::Node(fragment) => fragment,
            SerializeTask::Close(tag) => {
                out.push_str(tag);
                continue;
            }
        };
        match &fragment.node {
            PlainNode::Text(t) => serialize_text(&t.text, &fragment.modifiers, resource, out),
            PlainNode::HardBreak(_) => out.push_str("<br>"),
            PlainNode::Tab(_) => out.push('\t'),
            PlainNode::Paragraph(_) => open_container("<p>", "</p>", fragment, &mut tasks, out),
            PlainNode::BulletList(_) => open_container("<ul>", "</ul>", fragment, &mut tasks, out),
            PlainNode::OrderedList(_) => open_container("<ol>", "</ol>", fragment, &mut tasks, out),
            PlainNode::ListItem(_) => open_container("<li>", "</li>", fragment, &mut tasks, out),
            PlainNode::Blockquote(b) => open_container(
                &format!(r#"<blockquote data-variant="{}">"#, variant_str(&b.variant)),
                "</blockquote>",
                fragment,
                &mut tasks,
                out,
            ),
            PlainNode::Callout(c) => open_container(
                &format!(
                    r#"<aside data-callout data-variant="{}">"#,
                    variant_str(&c.variant)
                ),
                "</aside>",
                fragment,
                &mut tasks,
                out,
            ),
            PlainNode::Fold(_) => {
                open_container("<details>", "</details>", fragment, &mut tasks, out)
            }
            PlainNode::FoldTitle(_) => {
                open_container("<summary>", "</summary>", fragment, &mut tasks, out)
            }
            PlainNode::FoldContent(_) => push_children(&mut tasks, &fragment.children),
            PlainNode::Table(t) => open_container(
                &format!(
                    r#"<table data-border-style="{}" data-proportion="{}">"#,
                    variant_str(&t.border_style),
                    t.proportion,
                ),
                "</table>",
                fragment,
                &mut tasks,
                out,
            ),
            PlainNode::TableRow(_) => open_container("<tr>", "</tr>", fragment, &mut tasks, out),
            PlainNode::TableCell(c) => {
                let open = match c.col_width {
                    Some(width) => format!(r#"<td data-col-width="{width}">"#),
                    None => "<td>".to_string(),
                };
                open_container(&open, "</td>", fragment, &mut tasks, out);
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
            PlainNode::PageBreak(_) => {
                out.push_str(r#"<div style="page-break-after:always"></div>"#)
            }
            PlainNode::HorizontalRule(_) => out.push_str("<hr>"),
            PlainNode::Root(_) => push_children(&mut tasks, &fragment.children),
            PlainNode::Unknown => {}
        }
    }
}

fn open_container<'a>(
    open: &str,
    close: &'static str,
    fragment: &'a Fragment,
    tasks: &mut Vec<SerializeTask<'a>>,
    out: &mut String,
) {
    out.push_str(open);
    tasks.push(SerializeTask::Close(close));
    push_children(tasks, &fragment.children);
}

fn push_children<'a>(tasks: &mut Vec<SerializeTask<'a>>, children: &'a [Fragment]) {
    tasks.extend(children.iter().rev().map(SerializeTask::Node));
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
    match resource
        .theme()
        .try_color(&format!("{token_prefix}.{value}"))
    {
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
        let slice = Slice::new(vec![], 0, 0);
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
    fn serialize_multi_paragraph_list_item_uses_one_li() {
        let (s, ..) = state! {
            doc { r: root {
                bullet_list {
                    list_item {
                        paragraph { text("a") }
                        paragraph { text("b") }
                    }
                }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        let html = Slice::extract(&s).unwrap().to_html(&Resource::new_test());

        assert_eq!(html.matches("<li>").count(), 1);
        assert!(html.contains("<p>a</p>"));
        assert!(html.contains("<p>b</p>"));
        assert!(html.find("<p>a</p>").unwrap() < html.find("<p>b</p>").unwrap());
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
