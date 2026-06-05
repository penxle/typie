use editor_macros::doc;
use editor_model::*;

#[test]
fn doc_basic_tree() {
    let (doc, p, t) = doc! {
        root {
            p: paragraph {
                t: text("Hello World")
            }
        }
    };

    let root = doc.get_entry(NodeId::ROOT).unwrap();
    assert!(matches!(root.node, Node::Root(_)));
    assert!(root.parent.get().is_none());
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children.iter().next().copied().unwrap(), p);

    let p_entry = doc.get_entry(p).unwrap();
    assert!(matches!(p_entry.node, Node::Paragraph(_)));
    assert_eq!(p_entry.parent.get(), &Some(NodeId::ROOT));
    assert_eq!(p_entry.children.len(), 1);
    assert_eq!(p_entry.children.iter().next().copied().unwrap(), t);

    let t_entry = doc.get_entry(t).unwrap();
    if let Node::Text(ref text_node) = t_entry.node {
        assert_eq!(text_node.text.to_string(), "Hello World");
    } else {
        panic!("expected Text node");
    }
    assert_eq!(t_entry.parent.get(), &Some(p));
    assert!(t_entry.children.is_empty());
    assert!(t_entry.modifiers.is_empty());
}

#[test]
fn doc_styled_text() {
    let (doc, t) = doc! {
        root {
            paragraph {
                t: text("Hello") [bold, italic]
            }
        }
    };
    let t_entry = doc.get_entry(t).unwrap();
    if let Node::Text(ref text_node) = t_entry.node {
        assert_eq!(text_node.text.to_string(), "Hello");
    } else {
        panic!("expected Text node");
    }
    assert_eq!(t_entry.modifiers.len(), 2);
    let mod_keys: std::collections::HashSet<ModifierType> =
        t_entry.modifiers.iter().map(|(k, _)| *k).collect();
    assert!(mod_keys.contains(&ModifierType::Bold));
    assert!(mod_keys.contains(&ModifierType::Italic));
}

#[test]
fn doc_multi_node_text() {
    let (doc, t1, t2) = doc! {
        root {
            paragraph {
                t1: text("Hello ") [bold]
                t2: text("World")
            }
        }
    };
    let t1_entry = doc.get_entry(t1).unwrap();
    if let Node::Text(ref text_node) = t1_entry.node {
        assert_eq!(text_node.text.to_string(), "Hello ");
    } else {
        panic!("expected Text node");
    }
    assert_eq!(t1_entry.modifiers.len(), 1);
    assert!(t1_entry.modifiers.contains_key(&ModifierType::Bold));

    let t2_entry = doc.get_entry(t2).unwrap();
    if let Node::Text(ref text_node) = t2_entry.node {
        assert_eq!(text_node.text.to_string(), "World");
    } else {
        panic!("expected Text node");
    }
    assert!(t2_entry.modifiers.is_empty());
}

#[test]
fn doc_link_modifier() {
    let (doc, t) = doc! {
        root {
            paragraph {
                t: text("Click") [bold, link(href: "https://example.com".into())]
            }
        }
    };
    let t_entry = doc.get_entry(t).unwrap();
    if let Node::Text(ref text_node) = t_entry.node {
        assert_eq!(text_node.text.to_string(), "Click");
    } else {
        panic!("expected Text node");
    }
    assert_eq!(t_entry.modifiers.len(), 2);
    assert!(t_entry.modifiers.contains_key(&ModifierType::Bold));
    if let Some(Modifier::Link { href }) = t_entry.modifiers.get(&ModifierType::Link).cloned() {
        assert_eq!(href, "https://example.com");
    } else {
        panic!("expected Link modifier");
    }
}

#[test]
fn doc_node_params() {
    let (doc, p) = doc! {
        root {
            p: paragraph [alignment(Alignment::Center)] {
                text("Hello")
            }
        }
    };
    let p_entry = doc.get_entry(p).unwrap();
    assert_eq!(
        p_entry.modifiers.get(&ModifierType::Alignment),
        Some(&Modifier::Alignment {
            value: Alignment::Center
        })
    );
}

#[test]
fn doc_leaf_node() {
    let (doc, hr) = doc! {
        root {
            hr: horizontal_rule
        }
    };
    let hr_entry = doc.get_entry(hr).unwrap();
    assert!(matches!(hr_entry.node, Node::HorizontalRule(_)));
    assert_eq!(hr_entry.parent.get(), &Some(NodeId::ROOT));
    assert!(hr_entry.children.is_empty());
}

#[test]
fn doc_unnamed_node() {
    let (doc, t) = doc! {
        root {
            paragraph {
                t: text("Hello")
            }
        }
    };
    let t_entry = doc.get_entry(t).unwrap();
    if let Node::Text(ref text_node) = t_entry.node {
        assert_eq!(text_node.text.to_string(), "Hello");
    } else {
        panic!("expected Text node");
    }
    let root = doc.get_entry(NodeId::ROOT).unwrap();
    assert_eq!(root.children.len(), 1);
    let p_id = root.children.iter().next().copied().unwrap();
    let p_entry = doc.get_entry(p_id).unwrap();
    assert!(matches!(p_entry.node, Node::Paragraph(_)));
}

#[test]
fn doc_modifier_shorthand_on_block() {
    let (doc, p) = doc! {
        root {
            p: paragraph [bold] {
                text("Hello")
            }
        }
    };
    let p_entry = doc.get_entry(p).unwrap();
    assert_eq!(p_entry.modifiers.len(), 1);
    assert!(p_entry.modifiers.contains_key(&ModifierType::Bold));
}

#[test]
fn doc_root_with_layout_mode_param() {
    let (doc, ..) = doc! {
        root (
            layout_mode: LayoutMode::Paginated {
                page_width: 595,
                page_height: 842,
                page_margin_top: 50,
                page_margin_bottom: 50,
                page_margin_left: 50,
                page_margin_right: 50,
            }
        ) {
            paragraph { text("hi") }
        }
    };
    let root = doc.get_entry(NodeId::ROOT).unwrap();
    match &root.node {
        Node::Root(r) => assert!(matches!(*r.layout_mode.get(), LayoutMode::Paginated { .. })),
        _ => panic!("expected Root"),
    }
}

#[test]
fn doc_styles_declaration() {
    let (doc, ..) = doc! {
        styles {
            heading: "제목 1" [bold, font_size(2400)]
            body: [italic]
        }
        root {
            paragraph { text("hi") }
        }
    };

    assert!(doc.style_present("heading"));
    assert!(doc.style_present("body"));

    let heading = doc.style_entry("heading").unwrap();
    assert_eq!(heading.name.get(), "제목 1");
    assert!(
        heading
            .modifiers
            .iter()
            .any(|m| matches!(m, Modifier::Bold))
    );
    assert!(
        heading
            .modifiers
            .iter()
            .any(|m| matches!(m, Modifier::FontSize { value: 2400 }))
    );

    let body = doc.style_entry("body").unwrap();
    assert_eq!(body.name.get(), "body");
    assert!(body.modifiers.iter().any(|m| matches!(m, Modifier::Italic)));
}

#[test]
fn doc_node_style_reference() {
    let (doc, p) = doc! {
        styles {
            heading: "제목" [bold]
        }
        root {
            p: paragraph @heading { text("hi") }
        }
    };

    let p_entry = doc.get_entry(p).unwrap();
    assert_eq!(p_entry.style.get().as_deref(), Some("heading"));

    let p_ref = doc.node(p).unwrap();
    assert!(
        p_ref
            .modifiers_with_style()
            .any(|m| matches!(m, Modifier::Bold))
    );
}

#[test]
fn doc_style_on_text_and_leaf() {
    let (doc, t, hr) = doc! {
        styles {
            emph: [italic]
            rule: [bold]
        }
        root {
            paragraph {
                t: text("hi") @emph
            }
            hr: horizontal_rule @rule
        }
    };

    assert_eq!(
        doc.get_entry(t).unwrap().style.get().as_deref(),
        Some("emph")
    );
    assert_eq!(
        doc.get_entry(hr).unwrap().style.get().as_deref(),
        Some("rule")
    );
}

#[test]
fn doc_style_and_explicit_modifier_coexist() {
    let (doc, p) = doc! {
        styles {
            heading: [bold]
        }
        root {
            p: paragraph @heading [italic] {
                text("hi")
            }
        }
    };

    let entry = doc.get_entry(p).unwrap();
    assert_eq!(entry.style.get().as_deref(), Some("heading"));
    assert!(entry.modifiers.contains_key(&ModifierType::Italic));

    let p_ref = doc.node(p).unwrap();
    assert!(
        p_ref
            .modifiers_with_style()
            .any(|m| matches!(m, Modifier::Bold))
    );
    assert!(
        p_ref
            .modifiers_with_style()
            .any(|m| matches!(m, Modifier::Italic))
    );
}

// `doc!`/`state!`의 styles 검증은 컴파일 타임 에러다 (trybuild 미도입 — 아래는 계약 문서).
//
// 1. 미선언 style 참조:
//        root { paragraph @nope { text("x") } }
//    => error: unknown style `nope`
//
// 2. styles 블록 내 중복 id:
//        styles { a: [bold]  a: [italic] }  root { paragraph { text("x") } }
//    => error: duplicate style `a`
//
// 3. 빈 style def (표시명·modifiers 둘 다 없음):
//        styles { a: }  root { paragraph { text("x") } }
//    => error: style definition must have a display name or modifiers
