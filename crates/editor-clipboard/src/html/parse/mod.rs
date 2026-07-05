pub mod inheritance;
pub mod normalize;
pub mod resolve_weight;
pub mod rules;
pub mod schema_normalize;
pub mod shorthand;
pub mod stylesheet;
pub mod value;
pub mod walker;

use crate::slice::Slice;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use editor_model::{Fragment, PlainNode, PlainRootNode};
use editor_resource::Resource;
use scraper::{Html, Selector};
use std::sync::OnceLock;

fn body_selector() -> &'static Selector {
    static S: OnceLock<Selector> = OnceLock::new();
    S.get_or_init(|| Selector::parse("body").expect("body selector"))
}

fn meta_data_slice_selector() -> &'static Selector {
    static S: OnceLock<Selector> = OnceLock::new();
    S.get_or_init(|| Selector::parse("meta[data-slice]").expect("meta selector"))
}

pub fn from_html(html: &str, resource: &Resource) -> Slice {
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
    let doc = Html::parse_fragment(input);
    if let Some(slice) = extract_meta_slice(&doc) {
        return slice;
    }
    fallback_body_parse(&doc, resource)
}

fn extract_meta_slice(doc: &Html) -> Option<Slice> {
    let meta = doc.select(meta_data_slice_selector()).next()?;
    let b64 = meta.value().attr("data-slice")?;
    let bytes = STANDARD.decode(b64.as_bytes()).ok()?;
    let s = std::str::from_utf8(&bytes).ok()?;
    serde_json::from_str(s).ok()
}

fn fallback_body_parse(doc: &Html, resource: &Resource) -> Slice {
    let sheet = crate::html::parse::stylesheet::ComputedStylesheet::from_html(doc);
    let body = doc
        .select(body_selector())
        .next()
        .unwrap_or_else(|| doc.root_element());
    let mut children = vec![];
    for child in body.children() {
        crate::html::parse::walker::walk(child, &mut children, &[], &[], &sheet, resource);
    }
    Slice {
        fragment: Fragment {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: vec![],
            style: None,
            children: crate::html::parse::schema_normalize::normalize(children),
        },
        open_start: 0,
        open_end: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_doc::DocBuilder;
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::{AtomLeaf, Modifier, NodeType, PlainParagraphNode, PlainTextNode};
    use editor_resource::{FontFamily, FontFamilySource, FontWeight, Resource, ThemeVariant};
    use editor_state::{Position, Selection};

    #[test]
    fn from_html_round_trip_via_meta() {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("World") }
            } }
            selection: (p1, 1) -> (p2, 3)
        };
        let original = Slice::extract(&s).unwrap();
        let html = original.to_html();
        let resource = Resource::new_test();
        let parsed = Slice::from_html(&html, &resource);
        assert_eq!(parsed, original);
    }

    #[test]
    fn from_html_body_paragraph() {
        let html = "<p>Hello</p>";
        let slice = Slice::from_html(html, &Resource::new_test());
        assert!(matches!(slice.fragment.node, PlainNode::Root(_)));
        assert_eq!(slice.fragment.children.len(), 1);
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::Paragraph(_)
        ));
    }

    #[test]
    fn from_html_ignores_apple_interchange_newline_break() {
        let html = r#"<p>Hello</p><br class="Apple-interchange-newline">"#;
        let slice = Slice::from_html(html, &Resource::new_test());
        assert_eq!(slice.fragment.children.len(), 1);
        let p = &slice.fragment.children[0];
        assert!(matches!(p.node, PlainNode::Paragraph(_)));
        assert_eq!(p.children.len(), 1);
        assert!(matches!(&p.children[0].node, PlainNode::Text(t) if t.text == "Hello"));
    }

    #[test]
    fn from_html_text_newline_is_ignored() {
        let html = "<p style=\"white-space:break-spaces\"><span>a\nb</span></p>";
        let slice = Slice::from_html(html, &Resource::new_test());
        let p = &slice.fragment.children[0];
        assert_eq!(p.children.len(), 1);
        assert!(matches!(&p.children[0].node, PlainNode::Text(t) if t.text == "ab"));
    }

    #[test]
    fn from_html_trailing_text_newline_is_ignored() {
        let html = "<p style=\"white-space:break-spaces\"><span>a\n</span></p>";
        let slice = Slice::from_html(html, &Resource::new_test());
        let p = &slice.fragment.children[0];
        assert_eq!(p.children.len(), 1);
        assert!(matches!(&p.children[0].node, PlainNode::Text(t) if t.text == "a"));
    }

    #[test]
    fn from_html_text_crlf_is_ignored() {
        let html = "<p><span>a\r\nb\rc</span></p>";
        let slice = Slice::from_html(html, &Resource::new_test());
        let p = &slice.fragment.children[0];
        assert_eq!(p.children.len(), 1);
        assert!(matches!(&p.children[0].node, PlainNode::Text(t) if t.text == "abc"));
    }

    #[test]
    fn from_html_br_still_becomes_hard_break() {
        let html = "<p>a<br>b</p>";
        let slice = Slice::from_html(html, &Resource::new_test());
        let p = &slice.fragment.children[0];
        assert_eq!(p.children.len(), 3);
        assert!(matches!(&p.children[0].node, PlainNode::Text(t) if t.text == "a"));
        assert!(matches!(p.children[1].node, PlainNode::HardBreak(_)));
        assert!(matches!(&p.children[2].node, PlainNode::Text(t) if t.text == "b"));
    }

    #[test]
    fn from_html_bold_italic_modifiers() {
        let html = "<p><strong><em>hi</em></strong></p>";
        let slice = Slice::from_html(html, &Resource::new_test());
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
        let slice = Slice::from_html(html, &Resource::new_test());
        assert_eq!(slice.fragment.children.len(), 1);
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::Paragraph(_)
        ));
    }

    #[test]
    fn from_html_orphan_li_wrapped_in_ul() {
        let html = "<li>a</li>";
        let slice = Slice::from_html(html, &Resource::new_test());
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::BulletList(_)
        ));
    }

    #[test]
    fn from_html_orphan_tr_wrapped_in_table() {
        let html = "<tr><td>a</td></tr>";
        let slice = Slice::from_html(html, &Resource::new_test());
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::Table(_)
        ));
    }

    #[test]
    fn from_html_invalid_meta_falls_back_to_body() {
        let html = r#"<meta data-slice="!!!notbase64!!!" data-version="1"><div data-root><p>hello</p></div>"#;
        let slice = Slice::from_html(html, &Resource::new_test());
        assert_eq!(slice.fragment.children.len(), 1);
        assert!(matches!(
            slice.fragment.children[0].node,
            PlainNode::Paragraph(_)
        ));
    }

    #[test]
    fn from_html_callout_variant_restored() {
        let html = r#"<aside data-callout data-variant="warning"><p>warn</p></aside>"#;
        let slice = Slice::from_html(html, &Resource::new_test());
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
        let slice = Slice::from_html(html, &Resource::new_test());
        let text_frag = &slice.fragment.children[0].children[0];
        let mods: Vec<_> = text_frag.modifiers.iter().collect();
        assert!(
            mods.iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 }))
        );
        assert!(mods.iter().any(|m| matches!(m, Modifier::Underline)));
        assert!(
            mods.iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "red"))
        );
    }

    #[test]
    fn parse_strong_span_font_weight_does_not_duplicate_bold() {
        let html = r#"<p><strong><span style="font-weight:700">x</span></strong></p>"#;
        let slice = Slice::from_html(html, &Resource::new_test());
        let text_frag = &slice.fragment.children[0].children[0];
        let bold_count = text_frag
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::Bold))
            .count();
        let fw_count = text_frag
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::FontWeight { value: 700 }))
            .count();
        assert_eq!(
            bold_count, 0,
            "inherited Bold suppressed by child's raw font-weight"
        );
        assert_eq!(fw_count, 1, "FontWeight{{700}} preserved exactly once");
    }

    #[test]
    fn parse_em_span_font_style_does_not_duplicate_italic() {
        let html = r#"<p><em><span style="font-style:italic">x</span></em></p>"#;
        let slice = Slice::from_html(html, &Resource::new_test());
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
        let slice = Slice::from_html(html, &Resource::new_test());
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
        let slice = Slice::from_html(html, &Resource::new_test());
        let text_frag = &slice.fragment.children[0].children[0];
        let strike_count = text_frag
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::Strikethrough))
            .count();
        assert_eq!(strike_count, 1);
    }

    #[test]
    fn font_weight_700_with_registered_700_uses_font_weight_only() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font-family:Pretendard;font-weight:700">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let text_frag = &slice.fragment.children[0].children[0];
        assert!(
            text_frag
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 })),
            "FontWeight{{700}} must be present"
        );
        assert!(
            !text_frag
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "no synthetic Bold when family has 700"
        );
    }

    #[test]
    fn font_weight_700_without_heavier_registered_uses_synthetic_bold() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![FontWeight {
                value: 400,
                hash: "h_p400".into(),
                chunks: vec![],
            }],
        }]);
        let html = r#"<p><span style="font-family:Pretendard;font-weight:700">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let text_frag = &slice.fragment.children[0].children[0];
        assert!(
            text_frag
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "synthetic Bold when family lacks 700"
        );
        assert!(
            !text_frag
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { .. })),
            "no FontWeight when synthetic Bold is used"
        );
    }

    #[test]
    fn roundtrip_bold_with_font_weight_does_not_accumulate() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let original = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![Fragment {
                    node: PlainNode::Paragraph(PlainParagraphNode::default()),
                    modifiers: vec![],
                    style: None,
                    children: vec![
                        Fragment::leaf(PlainNode::Text(PlainTextNode { text: "x".into() }))
                            .with_modifiers(vec![
                                Modifier::FontFamily {
                                    value: "Pretendard".into(),
                                },
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
        let parsed = Slice::from_html(body_only, &resource);
        let text_frag = &parsed.fragment.children[0].children[0];
        let bold_count = text_frag
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::Bold))
            .count();
        let fw_700 = text_frag
            .modifiers
            .iter()
            .any(|m| matches!(m, Modifier::FontWeight { value: 700 }));
        assert_eq!(bold_count, 0, "no synthetic Bold when 700 is registered");
        assert!(fw_700, "FontWeight{{700}} preserved through roundtrip");
    }

    #[test]
    fn font_shorthand_in_style_attribute_round_trip() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Arial".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_a400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_a700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font:italic bold 16px Arial">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(t.modifiers.iter().any(|m| matches!(m, Modifier::Italic)));
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 }))
        );
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 1200 })),
            "16px → 12pt × 100"
        );
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontFamily { value } if value == "Arial"))
        );
    }

    fn find_text(root: &Fragment) -> Option<&Fragment> {
        fn rec(f: &Fragment) -> Option<&Fragment> {
            if matches!(f.node, PlainNode::Text(_)) {
                return Some(f);
            }
            for c in &f.children {
                if let Some(t) = rec(c) {
                    return Some(t);
                }
            }
            None
        }
        rec(root)
    }

    #[test]
    fn inline_block_style_inherits() {
        let s = Slice::from_html(
            r#"<div style="color:red"><p>x</p></div>"#,
            &Resource::new_test(),
        );
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "red"))
        );
    }

    #[test]
    fn inner_wins_color_e2e() {
        let s = Slice::from_html(
            r#"<div style="color:red"><p style="color:blue">x</p></div>"#,
            &Resource::new_test(),
        );
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "indigo"))
        );
        assert!(
            !t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "red"))
        );
    }

    #[test]
    fn background_shorthand_inherits() {
        let s = Slice::from_html(
            r#"<div style="background:yellow"><p>x</p></div>"#,
            &Resource::new_test(),
        );
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::BackgroundColor { value } if value == "yellow"))
        );
    }

    #[test]
    fn stylesheet_class_inherits() {
        let s = Slice::from_html(
            r#"<style>.a { color: red; }</style><div class="a"><p>x</p></div>"#,
            &Resource::new_test(),
        );
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "red"))
        );
    }

    #[test]
    fn inline_overrides_stylesheet() {
        let s = Slice::from_html(
            r#"<style>p { color: red; }</style><p style="color:blue">x</p>"#,
            &Resource::new_test(),
        );
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "indigo"))
        );
    }

    #[test]
    fn specificity_class_beats_tag_e2e() {
        let s = Slice::from_html(
            r#"<style>p { color: blue; } .c { color: red; }</style><p class="c">x</p>"#,
            &Resource::new_test(),
        );
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "red"))
        );
    }

    #[test]
    fn link_nested_inherits() {
        let s = Slice::from_html(
            r#"<a href="https://a.com"><span>nested</span></a>"#,
            &Resource::new_test(),
        );
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Link { href } if href == "https://a.com"))
        );
    }

    #[test]
    fn link_does_not_leak_to_sibling() {
        let s = Slice::from_html(
            r#"<p><a href="https://a.com">a</a><span>b</span></p>"#,
            &Resource::new_test(),
        );
        let p = &s.fragment.children[0];
        let mut ta: Option<&Fragment> = None;
        let mut tb: Option<&Fragment> = None;
        fn collect<'a>(
            f: &'a Fragment,
            a: &mut Option<&'a Fragment>,
            b: &mut Option<&'a Fragment>,
        ) {
            if let PlainNode::Text(t) = &f.node {
                if t.text == "a" {
                    *a = Some(f);
                } else if t.text == "b" {
                    *b = Some(f);
                }
            }
            for c in &f.children {
                collect(c, a, b);
            }
        }
        collect(p, &mut ta, &mut tb);
        assert!(
            ta.unwrap()
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Link { .. }))
        );
        assert!(
            !tb.unwrap()
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Link { .. }))
        );
    }

    #[test]
    fn alignment_lands_on_block() {
        let s = Slice::from_html(
            r#"<div style="text-align:center"><p>x</p></div>"#,
            &Resource::new_test(),
        );
        fn find_block(f: &Fragment) -> Option<&Fragment> {
            if matches!(f.node, PlainNode::Paragraph(_)) {
                return Some(f);
            }
            for c in &f.children {
                if let Some(t) = find_block(c) {
                    return Some(t);
                }
            }
            None
        }
        let p = find_block(&s.fragment).unwrap();
        assert!(p.modifiers.iter().any(|m| matches!(
            m,
            Modifier::Alignment {
                value: editor_model::Alignment::Center
            }
        )));
        let t = find_text(&s.fragment).unwrap();
        assert!(
            !t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Alignment { .. }))
        );
    }

    #[test]
    fn important_ignored_inline() {
        let s = Slice::from_html(
            r#"<p style="color:red !important">x</p>"#,
            &Resource::new_test(),
        );
        let t = find_text(&s.fragment).unwrap();
        assert!(
            !t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { .. }))
        );
    }

    #[test]
    fn important_ignored_stylesheet() {
        let s = Slice::from_html(
            r#"<style>p { color: red !important; }</style><p>x</p>"#,
            &Resource::new_test(),
        );
        let t = find_text(&s.fragment).unwrap();
        assert!(
            !t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { .. }))
        );
    }

    #[test]
    fn orphan_tr_wraps() {
        let s = Slice::from_html("<tr><td>a</td></tr>", &Resource::new_test());
        assert!(matches!(s.fragment.children[0].node, PlainNode::Table(_)));
    }

    fn register_test_family(resource: &mut Resource, name: &str) {
        resource.set_fonts(vec![FontFamily {
            name: name.to_string(),
            source: FontFamilySource::User,
            weights: vec![FontWeight {
                value: 400,
                hash: format!("h_{name}"),
                chunks: vec![],
            }],
        }]);
    }

    #[test]
    fn paste_word_red_text_snaps_to_palette() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="color:rgb(192,0,0)">red text</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "red")),
            "Word-style red rgb(192,0,0) must snap to 'red' palette key"
        );
    }

    #[test]
    fn paste_arial_falls_back_to_registered_family() {
        let mut resource = Resource::new_test();
        register_test_family(&mut resource, "Pretendard");
        let html = r#"<p><span style="font-family:'Arial', Pretendard, sans-serif">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontFamily { value } if value == "Pretendard")),
            "fallback list must produce first registered family"
        );
    }

    #[test]
    fn paste_unregistered_family_drops_modifier() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="font-family:Calibri">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            !t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontFamily { .. })),
            "unregistered family must be dropped"
        );
    }

    #[test]
    fn paste_arbitrary_weight_snaps_to_nearest_hundred() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font-family:Pretendard;font-weight:350">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 400 })),
            "350 must snap to 400 (round-half-up) and match family weight 400"
        );
    }

    #[test]
    fn paste_px_font_size_converts_to_pt_hundredths() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="font-size:16px">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 1200 })),
            "16px must convert to 1200 (12pt × 100)"
        );
    }

    #[test]
    fn paste_rem_font_size_converts() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="font-size:1.5rem">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 1800 })),
            "1.5rem must convert to 1800 (18pt × 100)"
        );
    }

    #[test]
    fn paste_dark_theme_white_does_not_snap_to_bright() {
        let mut resource = Resource::new_test();
        resource.theme.set_variant(ThemeVariant::DarkBlack);
        let html = r#"<p><span style="color:#ffffff">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        let key = t
            .modifiers
            .iter()
            .find_map(|m| match m {
                Modifier::TextColor { value } => Some(value.clone()),
                _ => None,
            })
            .expect("TextColor modifier present");
        assert_ne!(
            key, "bright",
            "dark theme #ffffff paste must not snap to 'bright' (denied palette key)"
        );
    }

    #[test]
    fn paste_transparent_background_normalizes_to_none() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="background-color:transparent">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::BackgroundColor { value } if value == "none")),
            "transparent background must become 'none' palette value"
        );
    }

    #[test]
    fn paste_letter_spacing_normal_normalizes_to_zero() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="letter-spacing:normal">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::LetterSpacing { value: 0 })),
            "letter-spacing:normal must become 0 (not dropped, per spec)"
        );
    }

    #[test]
    fn paste_letter_spacing_arbitrary_snaps_to_palette() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="letter-spacing:0.07em">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::LetterSpacing { value: 5 })),
            "0.07em (= 7) must snap to nearest palette value 5"
        );
    }

    #[test]
    fn strong_with_registered_700_uses_font_weight_only() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font-family:Pretendard"><strong>x</strong></span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        let bold_count = t
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::Bold))
            .count();
        let fw_700 = t
            .modifiers
            .iter()
            .any(|m| matches!(m, Modifier::FontWeight { value: 700 }));
        assert_eq!(
            bold_count, 0,
            "Bold must not be emitted when family has 700"
        );
        assert!(fw_700, "FontWeight{{700}} must be emitted");
    }

    #[test]
    fn strong_without_heavier_weight_uses_synthetic_bold() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![FontWeight {
                value: 400,
                hash: "h_p400".into(),
                chunks: vec![],
            }],
        }]);
        let html = r#"<p><span style="font-family:Pretendard"><strong>x</strong></span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        let bold_count = t
            .modifiers
            .iter()
            .filter(|m| matches!(m, Modifier::Bold))
            .count();
        let has_fw = t
            .modifiers
            .iter()
            .any(|m| matches!(m, Modifier::FontWeight { .. }));
        assert_eq!(
            bold_count, 1,
            "synthetic Bold required when no heavier weight"
        );
        assert!(
            !has_fw,
            "no FontWeight modifier when synthetic Bold is used"
        );
    }

    #[test]
    fn strong_without_registered_family_keeps_bold() {
        let resource = Resource::new_test();
        let html = r#"<p><strong>x</strong></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(t.modifiers.iter().any(|m| matches!(m, Modifier::Bold)));
        assert!(
            !t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { .. }))
        );
    }

    #[test]
    fn font_weight_800_snaps_to_700_when_700_registered() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font-family:Pretendard;font-weight:800">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 })),
            "800 + [400,700] → match_weight returns 700 (heavier preferred above 500)"
        );
    }

    #[test]
    fn font_weight_900_not_downgraded_when_900_registered() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 900,
                    hash: "h_p900".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font-family:Pretendard;font-weight:900">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 900 })),
            "900 must not be downgraded to 700"
        );
    }

    #[test]
    fn font_weight_600_snaps_to_700_when_only_400_and_700_registered() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font-family:Pretendard;font-weight:600">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 })),
            "600 with [400,700] → match_weight returns 700 (heavier preferred above 500)"
        );
    }

    #[test]
    fn font_weight_300_snaps_to_400_when_no_lighter_registered() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font-family:Pretendard;font-weight:300">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 400 })),
            "300 with [400,700] → match_weight returns 400 (closest lighter not present, falls back to 400)"
        );
    }

    #[test]
    fn font_weight_800_unknown_family_preserves_value() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="font-family:Calibri;font-weight:800">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 800 })),
            "value must be preserved when family is unknown"
        );
        assert!(
            !t.modifiers.iter().any(|m| matches!(m, Modifier::Bold)),
            "no Bold added (이중 굵음 방지)"
        );
    }

    #[test]
    fn font_weight_300_unknown_family_preserves_value() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="font-family:Calibri;font-weight:300">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 300 })),
            "value must be preserved when family is unknown"
        );
        assert!(!t.modifiers.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn font_weight_normal_reset_under_synth_bold_parent() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![FontWeight {
                value: 400,
                hash: "h_p400".into(),
                chunks: vec![],
            }],
        }]);
        let html = r#"<p><span style="font-family:Pretendard"><strong><span style="font-weight:normal">x</span></strong></span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 400 })),
            "child's normal (400) must be applied"
        );
        assert!(
            !t.modifiers.iter().any(|m| matches!(m, Modifier::Bold)),
            "parent's synth Bold must be suppressed by child's declared font-weight"
        );
    }

    #[test]
    fn font_weight_normal_reset_with_unregistered_child_family_suppresses_parent_bold() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![FontWeight {
                value: 400,
                hash: "h_p400".into(),
                chunks: vec![],
            }],
        }]);
        let html = r#"<p><span style="font-family:Pretendard"><strong><span style="font-family:Calibri;font-weight:normal">x</span></strong></span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 400 })),
            "child's normal preserved (Calibri drop → fallback to inherited Pretendard)"
        );
        assert!(
            !t.modifiers.iter().any(|m| matches!(m, Modifier::Bold)),
            "parent's synth Bold must be suppressed by child's raw font-weight declaration"
        );
    }

    #[test]
    fn inline_font_weight_overrides_strong_tag() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font-family:Pretendard"><strong style="font-weight:400">x</strong></span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 400 }))
        );
        assert!(!t.modifiers.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn font_weight_bolder_resolved_against_parent_400() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font-family:Pretendard"><span style="font-weight:bolder">x</span></span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 })),
            "bolder against parent 400 must resolve to FontWeight{{700}}"
        );
    }

    #[test]
    fn font_weight_lighter_resolved_against_parent_700() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![FontFamily {
            name: "Pretendard".into(),
            source: FontFamilySource::User,
            weights: vec![
                FontWeight {
                    value: 300,
                    hash: "h_p300".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 400,
                    hash: "h_p400".into(),
                    chunks: vec![],
                },
                FontWeight {
                    value: 700,
                    hash: "h_p700".into(),
                    chunks: vec![],
                },
            ],
        }]);
        let html = r#"<p><span style="font-family:Pretendard;font-weight:700"><span style="font-weight:lighter">x</span></span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 400 })),
            "lighter against parent 700 must resolve to FontWeight{{400}}"
        );
    }

    #[test]
    fn font_weight_bolder_against_synthetic_bold_parent() {
        let mut resource = Resource::new_test();
        resource.set_fonts(vec![
            FontFamily {
                name: "OnlyLight".into(),
                source: FontFamilySource::User,
                weights: vec![FontWeight {
                    value: 400,
                    hash: "h_l400".into(),
                    chunks: vec![],
                }],
            },
            FontFamily {
                name: "Heavy".into(),
                source: FontFamilySource::User,
                weights: vec![
                    FontWeight {
                        value: 400,
                        hash: "h_h400".into(),
                        chunks: vec![],
                    },
                    FontWeight {
                        value: 700,
                        hash: "h_h700".into(),
                        chunks: vec![],
                    },
                    FontWeight {
                        value: 900,
                        hash: "h_h900".into(),
                        chunks: vec![],
                    },
                ],
            },
        ]);
        let html = r#"<p><span style="font-family:OnlyLight"><strong><span style="font-family:Heavy;font-weight:bolder">x</span></strong></span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 900 })),
            "bolder against synthetic Bold parent (effective 700) must resolve to 900"
        );
    }

    #[test]
    fn html_tab_roundtrips() {
        let mut b = DocBuilder::new();
        let root = Dot::ROOT;
        let para = b.block(NodeType::Paragraph, &[root]);
        b.text("a");
        b.atom(AtomLeaf::Tab, &[]);
        b.text("b");
        let s = b.finish(Some(Selection::new(
            Position::new(para, 0),
            Position::new(para, 3),
        )));
        let slice = Slice::extract(&s).unwrap();
        let html = slice.to_html();
        assert!(
            html.contains('\t'),
            "serialized HTML must contain a tab char: {html}"
        );
        let resource = Resource::new_test();
        let meta_end = html.find('>').expect("meta tag closes") + 1;
        let body_only = &html[meta_end..];
        let parsed = Slice::from_html(body_only, &resource);
        fn has_tab(f: &editor_model::Fragment) -> bool {
            matches!(f.node, editor_model::PlainNode::Tab(_)) || f.children.iter().any(has_tab)
        }
        assert!(
            has_tab(&parsed.fragment),
            "parsed HTML must contain a Tab node"
        );
    }
}
