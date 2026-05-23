use crate::slice::Slice;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use editor_model::{
    Fragment, Modifier, PlainBlockquoteNode, PlainBulletListNode, PlainCalloutNode, PlainFoldNode,
    PlainFoldTitleNode, PlainHardBreakNode, PlainHorizontalRuleNode, PlainListItemNode, PlainNode,
    PlainOrderedListNode, PlainParagraphNode, PlainRootNode, PlainTableCellNode, PlainTableNode,
    PlainTableRowNode, PlainTextNode,
};
use html5ever::tendril::TendrilSink;
use html5ever::{ParseOpts, QualName, namespace_url, ns};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

pub fn from_html(html: &str) -> Slice {
    let trimmed = html.trim_start();
    let wrapped;
    let input = if trimmed.starts_with("<tr")
        || trimmed.starts_with("<td")
        || trimmed.starts_with("<th")
        || trimmed.starts_with("<tbody")
        || trimmed.starts_with("<thead")
        || trimmed.starts_with("<tfoot")
    {
        wrapped = format!("<table>{html}</table>");
        wrapped.as_str()
    } else {
        html
    };

    let opts = ParseOpts::default();
    let dom = html5ever::driver::parse_fragment(
        RcDom::default(),
        opts,
        QualName::new(None, ns!(html), "body".into()),
        vec![],
    )
    .one(input);

    if let Some(slice) = extract_meta_slice(&dom.document) {
        return slice;
    }
    fallback_body_parse(&dom.document)
}

fn extract_meta_slice(node: &Handle) -> Option<Slice> {
    if let NodeData::Element { name, attrs, .. } = &node.data
        && &*name.local == "meta"
    {
        for attr in attrs.borrow().iter() {
            if &*attr.name.local == "data-slice" {
                let b64 = attr.value.to_string();
                let bytes = STANDARD.decode(b64.as_bytes()).ok()?;
                let s = std::str::from_utf8(&bytes).ok()?;
                return serde_json::from_str(s).ok();
            }
        }
    }
    for child in node.children.borrow().iter() {
        if let Some(slice) = extract_meta_slice(child) {
            return Some(slice);
        }
    }
    None
}

fn fallback_body_parse(root: &Handle) -> Slice {
    let mut children = vec![];
    walk_into_fragments(root, &mut children, &[]);
    Slice {
        fragment: Fragment {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: vec![],
            children: normalize_schema(children),
        },
        open_start: 0,
        open_end: 0,
    }
}

fn walk_into_fragments(node: &Handle, out: &mut Vec<Fragment>, mods: &[Modifier]) {
    match &node.data {
        NodeData::Document => {
            for c in node.children.borrow().iter() {
                walk_into_fragments(c, out, mods);
            }
        }
        NodeData::Element { name, attrs, .. } => {
            let local = name.local.to_string();
            match local.as_str() {
                "p" => out.push(wrap_block(
                    node,
                    PlainNode::Paragraph(PlainParagraphNode::default()),
                    mods,
                )),
                "blockquote" => out.push(wrap_block(
                    node,
                    PlainNode::Blockquote(PlainBlockquoteNode::default()),
                    mods,
                )),
                "ul" => out.push(wrap_block(
                    node,
                    PlainNode::BulletList(PlainBulletListNode::default()),
                    mods,
                )),
                "ol" => out.push(wrap_block(
                    node,
                    PlainNode::OrderedList(PlainOrderedListNode::default()),
                    mods,
                )),
                "li" => out.push(wrap_block(
                    node,
                    PlainNode::ListItem(PlainListItemNode::default()),
                    mods,
                )),
                "table" => out.push(wrap_block(
                    node,
                    PlainNode::Table(PlainTableNode::default()),
                    mods,
                )),
                "tr" => out.push(wrap_block(
                    node,
                    PlainNode::TableRow(PlainTableRowNode::default()),
                    mods,
                )),
                "td" => out.push(wrap_block(
                    node,
                    PlainNode::TableCell(PlainTableCellNode::default()),
                    mods,
                )),
                "hr" => out.push(Fragment::leaf(PlainNode::HorizontalRule(
                    PlainHorizontalRuleNode::default(),
                ))),
                "br" => out.push(Fragment::leaf(PlainNode::HardBreak(
                    PlainHardBreakNode::default(),
                ))),
                "details" => out.push(wrap_block(
                    node,
                    PlainNode::Fold(PlainFoldNode::default()),
                    mods,
                )),
                "summary" => out.push(wrap_block(
                    node,
                    PlainNode::FoldTitle(PlainFoldTitleNode::default()),
                    mods,
                )),
                "strong" | "b" => walk_inline_with_mod(node, out, mods, Modifier::Bold),
                "em" | "i" => walk_inline_with_mod(node, out, mods, Modifier::Italic),
                "u" => walk_inline_with_mod(node, out, mods, Modifier::Underline),
                "s" | "strike" | "del" => {
                    walk_inline_with_mod(node, out, mods, Modifier::Strikethrough)
                }
                "a" => {
                    let href = attrs
                        .borrow()
                        .iter()
                        .find(|a| &*a.name.local == "href")
                        .map(|a| a.value.to_string())
                        .unwrap_or_default();
                    walk_inline_with_mod(node, out, mods, Modifier::Link { href });
                }
                "aside" => {
                    let attrs_borrow = attrs.borrow();
                    let is_typie_callout = attrs_borrow
                        .iter()
                        .any(|a| &*a.name.local == "data-callout");
                    if is_typie_callout {
                        let variant_s = attrs_borrow
                            .iter()
                            .find(|a| &*a.name.local == "data-variant")
                            .map(|a| a.value.to_string())
                            .unwrap_or_else(|| "info".into());
                        let variant = parse_variant::<editor_model::CalloutVariant>(&variant_s);
                        drop(attrs_borrow);
                        out.push(wrap_block(
                            node,
                            PlainNode::Callout(PlainCalloutNode { variant }),
                            mods,
                        ));
                    } else {
                        drop(attrs_borrow);
                        for c in node.children.borrow().iter() {
                            walk_into_fragments(c, out, mods);
                        }
                    }
                }
                "span" | "font" => {
                    let style = attrs
                        .borrow()
                        .iter()
                        .find(|a| &*a.name.local == "style")
                        .map(|a| a.value.to_string())
                        .unwrap_or_default();
                    let mut new_mods = mods.to_vec();
                    parse_style_into_mods(&style, &mut new_mods);
                    for c in node.children.borrow().iter() {
                        walk_into_fragments(c, out, &new_mods);
                    }
                }
                "div" | "section" | "article" | "header" | "footer" | "main" | "html" | "body"
                | "head" | "meta" => {
                    for c in node.children.borrow().iter() {
                        walk_into_fragments(c, out, mods);
                    }
                }
                "script" | "style" => {}
                _ => {
                    for c in node.children.borrow().iter() {
                        walk_into_fragments(c, out, mods);
                    }
                }
            }
        }
        NodeData::Text { contents } => {
            let text = contents.borrow().to_string();
            if !text.is_empty() {
                out.push(
                    Fragment::leaf(PlainNode::Text(PlainTextNode { text }))
                        .with_modifiers(mods.to_vec()),
                );
            }
        }
        _ => {}
    }
}

fn parse_variant<T: serde::de::DeserializeOwned + Default>(s: &str) -> T {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or_default()
}

fn push_unique(mods: &mut Vec<Modifier>, m: Modifier) {
    if !mods.contains(&m) {
        mods.push(m);
    }
}

fn parse_style_into_mods(style: &str, mods: &mut Vec<Modifier>) {
    for decl in style.split(';') {
        let mut parts = decl.splitn(2, ':');
        let key = parts.next().unwrap_or("").trim().to_lowercase();
        let value = parts.next().unwrap_or("").trim();
        match key.as_str() {
            "font-weight" => {
                let weight: u16 = if value == "bold" {
                    700
                } else {
                    value.parse().unwrap_or(400)
                };
                if weight >= 600 {
                    push_unique(mods, Modifier::Bold);
                }
                mods.push(Modifier::FontWeight { value: weight });
            }
            "font-style" if value == "italic" => push_unique(mods, Modifier::Italic),
            "text-decoration" => {
                if value.contains("underline") {
                    push_unique(mods, Modifier::Underline);
                }
                if value.contains("line-through") {
                    push_unique(mods, Modifier::Strikethrough);
                }
            }
            "color" => mods.push(Modifier::TextColor {
                value: value.to_string(),
            }),
            "background-color" => mods.push(Modifier::BackgroundColor {
                value: value.to_string(),
            }),
            "font-size" => {
                if let Some(pt) = value.strip_suffix("pt").and_then(|s| s.parse::<f32>().ok()) {
                    mods.push(Modifier::FontSize {
                        value: (pt * 100.0) as u32,
                    });
                }
            }
            "font-family" => mods.push(Modifier::FontFamily {
                value: value.to_string(),
            }),
            "letter-spacing" => {
                if let Some(em) = value.strip_suffix("em").and_then(|s| s.parse::<f32>().ok()) {
                    mods.push(Modifier::LetterSpacing {
                        value: (em * 100.0) as i32,
                    });
                }
            }
            _ => {}
        }
    }
}

fn walk_inline_with_mod(
    node: &Handle,
    out: &mut Vec<Fragment>,
    parent_mods: &[Modifier],
    add: Modifier,
) {
    let mut new_mods = parent_mods.to_vec();
    new_mods.push(add);
    for c in node.children.borrow().iter() {
        walk_into_fragments(c, out, &new_mods);
    }
}

fn wrap_block(node: &Handle, plain_node: PlainNode, mods: &[Modifier]) -> Fragment {
    let mut kids = vec![];
    for c in node.children.borrow().iter() {
        walk_into_fragments(c, &mut kids, mods);
    }
    Fragment {
        node: plain_node,
        modifiers: vec![],
        children: kids,
    }
}

fn normalize_schema(children: Vec<Fragment>) -> Vec<Fragment> {
    let mut result: Vec<Fragment> = vec![];
    let mut inline_run: Vec<Fragment> = vec![];

    fn flush_inline(inline_run: &mut Vec<Fragment>, result: &mut Vec<Fragment>) {
        if !inline_run.is_empty() {
            result.push(Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                children: std::mem::take(inline_run),
            });
        }
    }

    for child in children {
        match &child.node {
            PlainNode::Text(_) | PlainNode::HardBreak(_) => inline_run.push(child),
            PlainNode::ListItem(_) => {
                flush_inline(&mut inline_run, &mut result);
                result.push(Fragment {
                    node: PlainNode::BulletList(PlainBulletListNode::default()),
                    modifiers: vec![],
                    children: vec![child],
                });
            }
            PlainNode::TableRow(_) => {
                flush_inline(&mut inline_run, &mut result);
                result.push(Fragment {
                    node: PlainNode::Table(PlainTableNode::default()),
                    modifiers: vec![],
                    children: vec![child],
                });
            }
            PlainNode::TableCell(_) => {
                flush_inline(&mut inline_run, &mut result);
                result.push(Fragment {
                    node: PlainNode::Table(PlainTableNode::default()),
                    modifiers: vec![],
                    children: vec![Fragment {
                        node: PlainNode::TableRow(PlainTableRowNode::default()),
                        modifiers: vec![],
                        children: vec![child],
                    }],
                });
            }
            _ => {
                flush_inline(&mut inline_run, &mut result);
                result.push(child);
            }
        }
    }
    flush_inline(&mut inline_run, &mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    #[test]
    fn from_html_round_trip_via_meta() {
        let (s, ..) = state! {
            doc { root {
                paragraph { t1: text("Hello") }
                paragraph { t2: text("World") }
            } }
            selection: (t1, 1) -> (t2, 3)
        };
        let original = Slice::extract(&s).unwrap();
        let html = original.to_html();
        let parsed = Slice::from_html(&html);
        assert_eq!(parsed, original);
    }

    #[test]
    fn from_html_body_paragraph() {
        let html = "<p>Hello</p>";
        let slice = Slice::from_html(html);
        assert!(matches!(slice.fragment.node, PlainNode::Root(_)));
        assert_eq!(slice.fragment.children.len(), 1);
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::Paragraph(_)
        ));
    }

    #[test]
    fn from_html_bold_italic_modifiers() {
        let html = "<p><strong><em>hi</em></strong></p>";
        let slice = Slice::from_html(html);
        let p = &slice.fragment.children[0];
        let text_frag = &p.children[0];
        assert!(matches!(text_frag.node, PlainNode::Text(_)));
        let mods: std::collections::HashSet<_> = text_frag.modifiers.iter().collect();
        assert!(mods.contains(&Modifier::Bold));
        assert!(mods.contains(&Modifier::Italic));
    }

    #[test]
    fn from_html_text_in_root_wrapped_in_paragraph() {
        let html = "hello";
        let slice = Slice::from_html(html);
        assert_eq!(slice.fragment.children.len(), 1);
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::Paragraph(_)
        ));
    }

    #[test]
    fn from_html_orphan_li_wrapped_in_ul() {
        let html = "<li>a</li>";
        let slice = Slice::from_html(html);
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::BulletList(_)
        ));
    }

    #[test]
    fn from_html_orphan_tr_wrapped_in_table() {
        let html = "<tr><td>a</td></tr>";
        let slice = Slice::from_html(html);
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::Table(_)
        ));
    }

    #[test]
    fn from_html_invalid_meta_falls_back_to_body() {
        let html = r#"<meta data-slice="!!!notbase64!!!" data-version="1"><div data-root><p>hello</p></div>"#;
        let slice = Slice::from_html(html);
        assert_eq!(slice.fragment.children.len(), 1);
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::Paragraph(_)
        ));
    }

    #[test]
    fn from_html_callout_variant_restored() {
        let html = r#"<aside data-callout data-variant="warning"><p>warn</p></aside>"#;
        let slice = Slice::from_html(html);
        let aside = &slice.fragment.children[0];
        if let PlainNode::Callout(c) = &aside.node {
            assert!(matches!(c.variant, editor_model::CalloutVariant::Warning));
        } else {
            panic!("expected Callout");
        }
    }

    #[test]
    fn from_html_inline_style_to_modifiers() {
        let html = r#"<p><span style="font-weight:700;color:#ff0000;text-decoration:underline">x</span></p>"#;
        let slice = Slice::from_html(html);
        let text_frag = &slice.fragment.children[0].children[0];
        let mods: Vec<_> = text_frag.modifiers.iter().collect();
        assert!(mods.iter().any(|m| matches!(m, Modifier::Bold)));
        assert!(mods.iter().any(|m| matches!(m, Modifier::Underline)));
        assert!(
            mods.iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "#ff0000"))
        );
    }

    #[test]
    fn parse_strong_span_font_weight_does_not_duplicate_bold() {
        let html = r#"<p><strong><span style="font-weight:700">x</span></strong></p>"#;
        let slice = Slice::from_html(html);
        let text_frag = &slice.fragment.children[0].children[0];
        let bold_count = text_frag
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::Bold))
            .count();
        assert_eq!(bold_count, 1, "Bold modifier should appear exactly once");
    }

    #[test]
    fn parse_em_span_font_style_does_not_duplicate_italic() {
        let html = r#"<p><em><span style="font-style:italic">x</span></em></p>"#;
        let slice = Slice::from_html(html);
        let text_frag = &slice.fragment.children[0].children[0];
        let italic_count = text_frag
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::Italic))
            .count();
        assert_eq!(italic_count, 1);
    }

    #[test]
    fn parse_u_span_text_decoration_does_not_duplicate_underline() {
        let html = r#"<p><u><span style="text-decoration:underline">x</span></u></p>"#;
        let slice = Slice::from_html(html);
        let text_frag = &slice.fragment.children[0].children[0];
        let underline_count = text_frag
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::Underline))
            .count();
        assert_eq!(underline_count, 1);
    }

    #[test]
    fn parse_s_span_text_decoration_does_not_duplicate_strikethrough() {
        let html = r#"<p><s><span style="text-decoration:line-through">x</span></s></p>"#;
        let slice = Slice::from_html(html);
        let text_frag = &slice.fragment.children[0].children[0];
        let strike_count = text_frag
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::Strikethrough))
            .count();
        assert_eq!(strike_count, 1);
    }

    #[test]
    fn parse_font_weight_still_added_when_no_structural_bold() {
        let html = r#"<p><span style="font-weight:700">x</span></p>"#;
        let slice = Slice::from_html(html);
        let text_frag = &slice.fragment.children[0].children[0];
        assert!(
            text_frag
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
        assert!(
            text_frag
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 }))
        );
    }

    #[test]
    fn roundtrip_bold_with_font_weight_does_not_accumulate() {
        let original = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![Fragment {
                    node: PlainNode::Paragraph(PlainParagraphNode::default()),
                    modifiers: vec![],
                    children: vec![
                        Fragment::leaf(PlainNode::Text(PlainTextNode { text: "x".into() }))
                            .with_modifiers(vec![
                                Modifier::Bold,
                                Modifier::FontWeight { value: 700 },
                            ]),
                    ],
                }],
            },
            open_start: 0,
            open_end: 0,
        };
        let html = original.to_html();
        let meta_end = html.find('>').expect("meta tag closes") + 1;
        let body_only = &html[meta_end..];
        let parsed = Slice::from_html(body_only);
        let text_frag = &parsed.fragment.children[0].children[0];
        let bold_count = text_frag
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::Bold))
            .count();
        assert_eq!(
            bold_count, 1,
            "Bold should not duplicate after fallback roundtrip"
        );
    }
}
