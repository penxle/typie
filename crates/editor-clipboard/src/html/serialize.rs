use super::markup::{
    HtmlTarget, NodeMarkup, markup_for_node, write_clipboard_text, write_close_tag, write_open_tag,
};
use crate::slice::Slice;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use editor_model::{Fragment, PlainNode};
use editor_resource::Resource;
use serde::Serialize;

pub(crate) fn serialize_clipboard_slice(slice: &Slice, resource: &Resource) -> String {
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
                write_close_tag(out, tag);
                continue;
            }
        };
        match markup_for_node(&fragment.node, HtmlTarget::Clipboard) {
            NodeMarkup::Text => {
                let PlainNode::Text(text) = &fragment.node else {
                    unreachable!()
                };
                write_clipboard_text(&text.text, &fragment.modifiers, resource, out);
            }
            NodeMarkup::Tab => out.push('\t'),
            NodeMarkup::Children => push_children(&mut tasks, &fragment.children),
            NodeMarkup::Skip => {}
            NodeMarkup::Element { tag, attrs, void } => {
                write_open_tag(out, tag, &attrs, &[]);
                if !void {
                    tasks.push(SerializeTask::Close(tag));
                    push_children(&mut tasks, &fragment.children);
                }
            }
        }
    }
}

fn push_children<'a>(tasks: &mut Vec<SerializeTask<'a>>, children: &'a [Fragment]) {
    tasks.extend(children.iter().rev().map(SerializeTask::Node));
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
        let html = serialize_clipboard_slice(&slice, &Resource::new_test());
        assert!(html.contains("data-slice-v2="));
        assert!(html.contains("data-version=\"1\""));
        assert!(html.contains("<div data-root>"));
        assert!(html.contains("</div>"));
    }

    #[test]
    fn serialize_prepends_charset_meta() {
        let slice = Slice::new(vec![], 0, 0);
        let html = serialize_clipboard_slice(&slice, &Resource::new_test());
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
