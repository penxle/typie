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
    assert!(matches!(root.node, Node::Root(RootNode {})));
    assert!(root.parent.is_none());
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0], p);

    let p_entry = doc.get_entry(p).unwrap();
    assert!(matches!(p_entry.node, Node::Paragraph(_)));
    assert_eq!(p_entry.parent, Some(NodeId::ROOT));
    assert_eq!(p_entry.children.len(), 1);
    assert_eq!(p_entry.children[0], t);

    let t_entry = doc.get_entry(t).unwrap();
    if let Node::Text(ref text_node) = t_entry.node {
        assert_eq!(text_node.text, "Hello World");
    } else {
        panic!("expected Text node");
    }
    assert_eq!(t_entry.parent, Some(p));
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
        assert_eq!(text_node.text, "Hello");
    } else {
        panic!("expected Text node");
    }
    assert_eq!(t_entry.modifiers.len(), 2);
    assert!(matches!(t_entry.modifiers[0], Modifier::Bold));
    assert!(matches!(t_entry.modifiers[1], Modifier::Italic));
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
        assert_eq!(text_node.text, "Hello ");
    } else {
        panic!("expected Text node");
    }
    assert_eq!(t1_entry.modifiers.len(), 1);
    assert!(matches!(t1_entry.modifiers[0], Modifier::Bold));

    let t2_entry = doc.get_entry(t2).unwrap();
    if let Node::Text(ref text_node) = t2_entry.node {
        assert_eq!(text_node.text, "World");
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
        assert_eq!(text_node.text, "Click");
    } else {
        panic!("expected Text node");
    }
    assert_eq!(t_entry.modifiers.len(), 2);
    assert!(matches!(t_entry.modifiers[0], Modifier::Bold));
    if let Modifier::Link { ref href } = t_entry.modifiers[1] {
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
    assert!(p_entry.modifiers.contains(&Modifier::Alignment {
        value: Alignment::Center,
    }));
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
    assert_eq!(hr_entry.parent, Some(NodeId::ROOT));
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
        assert_eq!(text_node.text, "Hello");
    } else {
        panic!("expected Text node");
    }
    let root = doc.get_entry(NodeId::ROOT).unwrap();
    assert_eq!(root.children.len(), 1);
    let p_id = root.children[0];
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
    assert!(matches!(p_entry.modifiers[0], Modifier::Bold));
}
