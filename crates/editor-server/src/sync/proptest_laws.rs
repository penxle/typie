use std::collections::HashSet;

use editor_model::{
    Doc, Modifier, Node, NodeEntry, NodeId, ParagraphNode, RootNode, TextNode, imbl,
};
use icu_segmenter::GraphemeClusterSegmenter;
use proptest::prelude::*;

use crate::sync::test_helpers::assert_doc_consistent;
use crate::sync::{ConflictRecord, ConflictTarget, merge};

fn segmenter() -> GraphemeClusterSegmenter {
    GraphemeClusterSegmenter::new().static_to_owned()
}

fn arb_modifier() -> impl Strategy<Value = Modifier> {
    use editor_model::Alignment;
    prop_oneof![
        Just(Modifier::Bold),
        Just(Modifier::Italic),
        Just(Modifier::Underline),
        Just(Modifier::Strikethrough),
        (1u32..=10000u32).prop_map(|v| Modifier::FontSize { value: v }),
        "[a-zA-Z]{1,12}".prop_map(|v| Modifier::FontFamily { value: v }),
        (100u16..=900u16).prop_map(|v| Modifier::FontWeight { value: v }),
        "[a-fA-F0-9]{6}".prop_map(|v| Modifier::TextColor { value: v }),
        "[a-fA-F0-9]{6}".prop_map(|v| Modifier::BackgroundColor { value: v }),
        (-200i32..=200i32).prop_map(|v| Modifier::LetterSpacing { value: v }),
        "https?://[a-z]{1,12}\\.[a-z]{1,3}".prop_map(|v| Modifier::Link { href: v }),
        "[a-z가-힣]{1,4}".prop_map(|v| Modifier::Ruby { text: v }),
        (50u32..=300u32).prop_map(|v| Modifier::LineHeight { value: v }),
        (0u32..=300u32).prop_map(|v| Modifier::BlockGap { value: v }),
        (0u32..=300u32).prop_map(|v| Modifier::ParagraphIndent { value: v }),
        prop_oneof![
            Just(Modifier::Alignment {
                value: Alignment::Left
            }),
            Just(Modifier::Alignment {
                value: Alignment::Center
            }),
            Just(Modifier::Alignment {
                value: Alignment::Right
            }),
            Just(Modifier::Alignment {
                value: Alignment::Justify
            }),
        ],
    ]
}

fn arb_modifier_vec() -> impl Strategy<Value = Vec<Modifier>> {
    // merge_modifiers canonicalizes to sorted-by-type, unique-by-type — match that invariant so
    // identity and side-id laws can compare against generator output directly.
    prop::collection::vec(arb_modifier(), 0..=3).prop_map(|mut v| {
        v.sort_by_key(|m| m.as_type());
        v.dedup_by_key(|m| m.as_type());
        v
    })
}

fn arb_text() -> impl Strategy<Value = String> {
    // Mix of ASCII, Korean, whitespace, and longer strings to surface bugs in
    // grapheme tokenization and Myers diff under varied input.
    prop_oneof![
        Just(String::new()),
        "[a-z가-힣]{1,12}",
        "[a-z A-Z0-9가-힣]{1,20}",
        " *[a-z가-힣]{1,8} *",
        "[a-zA-Z]{20,40}",
    ]
}

struct ParagraphSpec {
    p_id: NodeId,
    t_id: NodeId,
    text: String,
    p_modifiers: Vec<Modifier>,
    t_modifiers: Vec<Modifier>,
}

fn build_doc(p1: ParagraphSpec, p2: ParagraphSpec) -> Doc {
    let root = NodeEntry {
        node: Node::Root(RootNode {}),
        parent: None,
        children: imbl::Vector::from(vec![p1.p_id, p2.p_id]),
        modifiers: vec![],
    };
    let p1_entry = NodeEntry {
        node: Node::Paragraph(ParagraphNode::default()),
        parent: Some(NodeId::ROOT),
        children: imbl::Vector::from(vec![p1.t_id]),
        modifiers: p1.p_modifiers,
    };
    let t1_entry = NodeEntry {
        node: Node::Text(TextNode { text: p1.text }),
        parent: Some(p1.p_id),
        children: imbl::Vector::new(),
        modifiers: p1.t_modifiers,
    };
    let p2_entry = NodeEntry {
        node: Node::Paragraph(ParagraphNode::default()),
        parent: Some(NodeId::ROOT),
        children: imbl::Vector::from(vec![p2.t_id]),
        modifiers: p2.p_modifiers,
    };
    let t2_entry = NodeEntry {
        node: Node::Text(TextNode { text: p2.text }),
        parent: Some(p2.p_id),
        children: imbl::Vector::new(),
        modifiers: p2.t_modifiers,
    };
    Doc {
        nodes: imbl::hashmap! {
            NodeId::ROOT => root,
            p1.p_id => p1_entry,
            p1.t_id => t1_entry,
            p2.p_id => p2_entry,
            p2.t_id => t2_entry,
        },
        attrs: editor_model::DocumentAttrs::default(),
    }
}

#[derive(Debug, Clone)]
enum Mutation {
    None,
    AddParagraph,
    DeleteFirstParagraph,
    MoveTextAcrossParagraphs,
}

fn apply_mutation(
    doc: Doc,
    p1_id: NodeId,
    t1_id: NodeId,
    p2_id: NodeId,
    mutation: Mutation,
) -> Doc {
    match mutation {
        Mutation::None => doc,
        Mutation::AddParagraph => {
            let new_p = NodeId::new();
            let new_t = NodeId::new();
            let new_t_entry = NodeEntry {
                node: Node::Text(TextNode { text: "new".into() }),
                parent: Some(new_p),
                children: imbl::Vector::new(),
                modifiers: vec![],
            };
            let new_p_entry = NodeEntry {
                node: Node::Paragraph(ParagraphNode::default()),
                parent: Some(NodeId::ROOT),
                children: imbl::Vector::from(vec![new_t]),
                modifiers: vec![],
            };
            doc.insert_node(new_t, new_t_entry)
                .insert_node(new_p, new_p_entry)
                .with_node_updated(NodeId::ROOT, |mut e| {
                    e.children.push_back(new_p);
                    e
                })
        }
        Mutation::DeleteFirstParagraph => doc
            .remove_node(t1_id)
            .remove_node(p1_id)
            .with_node_updated(NodeId::ROOT, |mut e| {
                e.children.retain(|c| c != &p1_id);
                e
            }),
        Mutation::MoveTextAcrossParagraphs => doc
            .with_node_updated(t1_id, |mut e| {
                e.parent = Some(p2_id);
                e
            })
            .with_node_updated(p1_id, |mut e| {
                e.children.retain(|c| c != &t1_id);
                e
            })
            .with_node_updated(p2_id, |mut e| {
                e.children.push_back(t1_id);
                e
            }),
    }
}

fn arb_mutation() -> impl Strategy<Value = Mutation> {
    prop_oneof![
        Just(Mutation::None),
        Just(Mutation::AddParagraph),
        Just(Mutation::DeleteFirstParagraph),
        Just(Mutation::MoveTextAcrossParagraphs),
    ]
}

fn arb_doc_triple() -> impl Strategy<Value = (Doc, Doc, Doc)> {
    (
        arb_text(),
        arb_modifier_vec(),
        arb_modifier_vec(),
        arb_text(),
        arb_modifier_vec(),
        arb_modifier_vec(),
        arb_mutation(),
        arb_mutation(),
    )
        .prop_map(|(bt1, bp1m, bt1m, bt2, bp2m, bt2m, ours_mut, theirs_mut)| {
            let p1_id = NodeId::new();
            let t1_id = NodeId::new();
            let p2_id = NodeId::new();
            let t2_id = NodeId::new();
            let base = build_doc(
                ParagraphSpec {
                    p_id: p1_id,
                    t_id: t1_id,
                    text: bt1,
                    p_modifiers: bp1m,
                    t_modifiers: bt1m,
                },
                ParagraphSpec {
                    p_id: p2_id,
                    t_id: t2_id,
                    text: bt2,
                    p_modifiers: bp2m,
                    t_modifiers: bt2m,
                },
            );
            let ours = apply_mutation(base.clone(), p1_id, t1_id, p2_id, ours_mut);
            let theirs = apply_mutation(base.clone(), p1_id, t1_id, p2_id, theirs_mut);
            (base, ours, theirs)
        })
}

fn arb_doc_pair() -> impl Strategy<Value = (Doc, Doc)> {
    (
        arb_text(),
        arb_modifier_vec(),
        arb_modifier_vec(),
        arb_text(),
        arb_modifier_vec(),
        arb_modifier_vec(),
        arb_mutation(),
    )
        .prop_map(|(bt1, bp1m, bt1m, bt2, bp2m, bt2m, other_mut)| {
            let p1_id = NodeId::new();
            let t1_id = NodeId::new();
            let p2_id = NodeId::new();
            let t2_id = NodeId::new();
            let base = build_doc(
                ParagraphSpec {
                    p_id: p1_id,
                    t_id: t1_id,
                    text: bt1,
                    p_modifiers: bp1m,
                    t_modifiers: bt1m,
                },
                ParagraphSpec {
                    p_id: p2_id,
                    t_id: t2_id,
                    text: bt2,
                    p_modifiers: bp2m,
                    t_modifiers: bt2m,
                },
            );
            let other = apply_mutation(base.clone(), p1_id, t1_id, p2_id, other_mut);
            (base, other)
        })
}

proptest! {
    #[test]
    fn law_identity(
        (t1, p1m, t1m, t2, p2m, t2m) in (arb_text(), arb_modifier_vec(), arb_modifier_vec(), arb_text(), arb_modifier_vec(), arb_modifier_vec())
    ) {
        let p1_id = NodeId::new();
        let t1_id = NodeId::new();
        let p2_id = NodeId::new();
        let t2_id = NodeId::new();
        let doc = build_doc(
            ParagraphSpec { p_id: p1_id, t_id: t1_id, text: t1, p_modifiers: p1m, t_modifiers: t1m },
            ParagraphSpec { p_id: p2_id, t_id: t2_id, text: t2, p_modifiers: p2m, t_modifiers: t2m },
        );
        let seg = segmenter();
        let (m, c) = merge(&seg, &doc, &doc, &doc);
        prop_assert_eq!(&m, &doc);
        prop_assert!(c.is_empty());
        assert_doc_consistent(&m);
    }

    #[test]
    fn law_left_side_id((base, ours) in arb_doc_pair()) {
        let seg = segmenter();
        let (m, c) = merge(&seg, &base, &ours, &base);
        prop_assert_eq!(&m, &ours);
        prop_assert!(c.is_empty());
        assert_doc_consistent(&m);
    }

    #[test]
    fn law_right_side_id((base, theirs) in arb_doc_pair()) {
        let seg = segmenter();
        let (m, c) = merge(&seg, &base, &base, &theirs);
        prop_assert_eq!(&m, &theirs);
        prop_assert!(c.is_empty());
        assert_doc_consistent(&m);
    }

    #[test]
    fn law_symmetry_conflicts((base, ours, theirs) in arb_doc_triple()) {
        let seg = segmenter();
        let (m1, c1) = merge(&seg, &base, &ours, &theirs);
        let (m2, c2) = merge(&seg, &base, &theirs, &ours);
        let to_set = |v: Vec<ConflictRecord>| -> HashSet<ConflictTarget> {
            v.into_iter().map(|r| r.target).collect()
        };
        prop_assert_eq!(to_set(c1), to_set(c2));
        assert_doc_consistent(&m1);
        assert_doc_consistent(&m2);
    }

    #[test]
    fn law_determinism((base, ours, theirs) in arb_doc_triple()) {
        let seg = segmenter();
        let (m1, c1) = merge(&seg, &base, &ours, &theirs);
        let (m2, c2) = merge(&seg, &base, &ours, &theirs);
        prop_assert_eq!(&m1, &m2);
        // Conflict ordering depends on HashMap iteration order (v1 limitation); compare as sets.
        let to_set = |v: Vec<ConflictRecord>| -> HashSet<ConflictTarget> {
            v.into_iter().map(|r| r.target).collect()
        };
        prop_assert_eq!(to_set(c1), to_set(c2));
        assert_doc_consistent(&m1);
        assert_doc_consistent(&m2);
    }
}

#[test]
fn apply_mutation_produces_consistent_docs() {
    let p1_id = NodeId::new();
    let t1_id = NodeId::new();
    let p2_id = NodeId::new();
    let t2_id = NodeId::new();
    let base = build_doc(
        ParagraphSpec {
            p_id: p1_id,
            t_id: t1_id,
            text: "hello".into(),
            p_modifiers: vec![],
            t_modifiers: vec![],
        },
        ParagraphSpec {
            p_id: p2_id,
            t_id: t2_id,
            text: "world".into(),
            p_modifiers: vec![],
            t_modifiers: vec![],
        },
    );
    assert_doc_consistent(&base);
    for mutation in [
        Mutation::None,
        Mutation::AddParagraph,
        Mutation::DeleteFirstParagraph,
        Mutation::MoveTextAcrossParagraphs,
    ] {
        let mutated = apply_mutation(base.clone(), p1_id, t1_id, p2_id, mutation);
        assert_doc_consistent(&mutated);
    }
}
