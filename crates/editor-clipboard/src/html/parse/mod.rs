pub mod inheritance;
pub mod normalize;
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
            children: crate::html::parse::schema_normalize::normalize(children),
        },
        open_start: 0,
        open_end: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;
    use editor_model::{Modifier, PlainParagraphNode, PlainTextNode};
    use editor_resource::{FontFamily, FontFamilySource, FontWeight, Resource, ThemeVariant};

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
        assert!(mods.iter().any(|m| matches!(m, Modifier::Bold)));
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
        assert_eq!(bold_count, 1, "Bold modifier should appear exactly once");
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
    fn parse_font_weight_still_added_when_no_structural_bold() {
        let html = r#"<p><span style="font-weight:700">x</span></p>"#;
        let slice = Slice::from_html(html, &Resource::new_test());
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
        let resource = Resource::new_test();
        let parsed = Slice::from_html(body_only, &resource);
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

    fn find_text(root: &Fragment) -> Option<&Fragment> {
        fn rec<'a>(f: &'a Fragment) -> Option<&'a Fragment> {
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
        let s = Slice::from_html(r#"<div style="color:red"><p>x</p></div>"#, &Resource::new_test());
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "red"))
        );
    }

    #[test]
    fn inner_wins_color_e2e() {
        let s = Slice::from_html(r#"<div style="color:red"><p style="color:blue">x</p></div>"#, &Resource::new_test());
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
        let s = Slice::from_html(r#"<div style="background:yellow"><p>x</p></div>"#, &Resource::new_test());
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::BackgroundColor { value } if value == "yellow"))
        );
    }

    #[test]
    fn stylesheet_class_inherits() {
        let s =
            Slice::from_html(r#"<style>.a { color: red; }</style><div class="a"><p>x</p></div>"#, &Resource::new_test());
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { value } if value == "red"))
        );
    }

    #[test]
    fn inline_overrides_stylesheet() {
        let s = Slice::from_html(r#"<style>p { color: red; }</style><p style="color:blue">x</p>"#, &Resource::new_test());
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
        let s = Slice::from_html(r#"<a href="https://a.com"><span>nested</span></a>"#, &Resource::new_test());
        let t = find_text(&s.fragment).unwrap();
        assert!(
            t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Link { href } if href == "https://a.com"))
        );
    }

    #[test]
    fn link_does_not_leak_to_sibling() {
        let s = Slice::from_html(r#"<p><a href="https://a.com">a</a><span>b</span></p>"#, &Resource::new_test());
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
        let s = Slice::from_html(r#"<div style="text-align:center"><p>x</p></div>"#, &Resource::new_test());
        fn find_block<'a>(f: &'a Fragment) -> Option<&'a Fragment> {
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
        let s = Slice::from_html(r#"<p style="color:red !important">x</p>"#, &Resource::new_test());
        let t = find_text(&s.fragment).unwrap();
        assert!(
            !t.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { .. }))
        );
    }

    #[test]
    fn important_ignored_stylesheet() {
        let s = Slice::from_html(r#"<style>p { color: red !important; }</style><p>x</p>"#, &Resource::new_test());
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
            t.modifiers.iter().any(|m|
                matches!(m, Modifier::TextColor { value } if value == "red")),
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
            t.modifiers.iter().any(|m|
                matches!(m, Modifier::FontFamily { value } if value == "Pretendard")),
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
            !t.modifiers.iter().any(|m| matches!(m, Modifier::FontFamily { .. })),
            "unregistered family must be dropped"
        );
    }

    #[test]
    fn paste_arbitrary_weight_snaps_to_nearest_hundred() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="font-weight:350">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers.iter().any(|m|
                matches!(m, Modifier::FontWeight { value: 400 })),
            "350 must snap to 400 (round-half-up)"
        );
    }

    #[test]
    fn paste_px_font_size_converts_to_pt_hundredths() {
        let resource = Resource::new_test();
        let html = r#"<p><span style="font-size:16px">x</span></p>"#;
        let slice = Slice::from_html(html, &resource);
        let t = find_text(&slice.fragment).unwrap();
        assert!(
            t.modifiers.iter().any(|m|
                matches!(m, Modifier::FontSize { value: 1200 })),
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
            t.modifiers.iter().any(|m|
                matches!(m, Modifier::FontSize { value: 1800 })),
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
            t.modifiers.iter().any(|m|
                matches!(m, Modifier::BackgroundColor { value } if value == "none")),
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
            t.modifiers.iter().any(|m|
                matches!(m, Modifier::LetterSpacing { value: 0 })),
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
            t.modifiers.iter().any(|m|
                matches!(m, Modifier::LetterSpacing { value: 5 })),
            "0.07em (= 7) must snap to nearest palette value 5"
        );
    }
}
