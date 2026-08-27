use std::collections::HashSet;

use editor_crdt::Dot;
use editor_model::{
    ChildView, PlainDoc, PlainNode, PlainNodeEntry, PlainParagraphNode, PlainTextNode, Subtree,
};
use editor_state::State;
use editor_transaction::Transaction;
use proptest::prelude::*;

use crate::diff::blocks::carries_nothing;
use crate::error::XmlErrorDetail;
use crate::reader::from_xml;
use crate::test_support::{apply_mutation, arb_mutation, arb_plain_doc, live_heads};
use crate::writer::to_xml;
use crate::{XmlChild, XmlNode, XmlTree, edit};

fn load(doc: &PlainDoc) -> State {
    State::from_plain(doc).expect("generator produces loadable docs")
}

fn peer_paragraph() -> Subtree {
    Subtree {
        node: PlainNode::Paragraph(PlainParagraphNode {}),
        modifiers: Vec::new(),
        carry: Vec::new(),
        children: vec![Subtree {
            node: PlainNode::Text(PlainTextNode {
                text: "PEER".into(),
            }),
            modifiers: Vec::new(),
            carry: Vec::new(),
            children: Vec::new(),
            source_dots: Vec::new(),
        }],
        source_dots: Vec::new(),
    }
}

proptest! {
    // A mutation that finds no landing on the document it drew is a reject, and
    // the structural ones — a table row to drop, a list item to delete — land
    // on roughly a third of the documents the generator makes.
    #![proptest_config(ProptestConfig { cases: 256, max_global_rejects: 4096, ..ProptestConfig::default() })]

    #[test]
    fn round_trip_is_identity(doc in arb_plain_doc()) {
        let state = load(&doc);
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let tree = from_xml(&xml).unwrap();
        prop_assert_eq!(tree.to_plain_doc(), state.to_plain());
        prop_assert_eq!(tree.base, live_heads(&state));
    }

    #[test]
    fn serialization_is_deterministic(doc in arb_plain_doc()) {
        let state = load(&doc);
        let a = to_xml(&state, &live_heads(&state)).unwrap();
        let b = to_xml(&state, &live_heads(&state)).unwrap();
        prop_assert_eq!(a, b);
    }

    #[test]
    fn edit_reaches_target_and_keeps_untouched_dots(doc in arb_plain_doc(), m in arb_mutation()) {
        let state = load(&doc);
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let tree = from_xml(&xml).unwrap();
        let target = apply_mutation(&tree, &m);
        prop_assume!(target.is_some());
        let target = target.unwrap();
        let out = edit(state.clone(), &target).unwrap();
        prop_assert_eq!(out.state.to_plain(), reachable_target(&target, &out.state));

        // What saving writes back has to read as the document it was written
        // from, or the next edit starts from a file the document never held.
        let rewritten = to_xml(&out.state, &live_heads(&out.state));
        prop_assert!(rewritten.is_ok(), "the edited document does not serialize: {:?}", rewritten.err());
        let reread = from_xml(&rewritten.unwrap());
        prop_assert!(reread.is_ok(), "the written file does not read back: {:?}", reread.err());
        prop_assert_eq!(reread.unwrap().to_plain_doc(), out.state.to_plain());

        let before = block_dots(&state);
        let after = block_dots(&out.state);
        let kept = target_dots(&target);
        let view = out.state.view();
        for d in kept.intersection(&before).filter(|d| !d.is_synthetic()) {
            prop_assert!(
                after.contains(d) || view.alias_classes().contains(*d),
                "kept block dot {} vanished",
                d
            );
        }
    }

    #[test]
    fn concurrent_edits_merge(doc in arb_plain_doc(), m in arb_mutation()) {
        let state = load(&doc);
        let heads: hashbrown::HashSet<Dot> = state.graph().current_heads().copied().collect();
        let xml = to_xml(&state, &live_heads(&state)).unwrap();
        let tree = from_xml(&xml).unwrap();
        let target = apply_mutation(&tree, &m);
        prop_assume!(target.is_some());
        let target = target.unwrap();
        let server = edit(state.clone(), &target).unwrap().state;
        let server_css = server.graph().local_changesets_since(&heads).unwrap();

        let peer = State::from_changesets(state.graph().changesets_as_vec(), None).unwrap();
        let root = peer.view().root().unwrap().id();
        let mut tr = Transaction::new(&peer);
        tr.insert_subtree(root, 0, peer_paragraph()).unwrap();
        let peer = tr.commit().0;
        let peer_css = peer.graph().local_changesets_since(&heads).unwrap();

        let mut merged = peer.graph().clone();
        for cs in server_css {
            merged = merged.receive_changeset(cs).unwrap();
        }
        let mut merged_other = server.graph().clone();
        for cs in peer_css {
            merged_other = merged_other.receive_changeset(cs).unwrap();
        }
        let merged_state = State::from_changesets(merged.changesets_as_vec(), None).unwrap();
        let a = merged_state.to_plain();
        let b = State::from_changesets(merged_other.changesets_as_vec(), None).unwrap().to_plain();
        prop_assert_eq!(&a, &b);
        prop_assert!(plain_contains_peer(&a));

        let survived: Vec<PlainNodeEntry> = authored_blocks(&merged_state)
            .into_iter()
            .filter(|entry| !is_peer(entry))
            .collect();
        prop_assert_eq!(survived, authored_blocks(&server));
    }
}

/// The root's authored children, scaffolds left out. A scaffold stands or falls
/// on whether its siblings satisfy the content rule, so the peer's own paragraph
/// can dissolve one the server still needed; the blocks someone wrote are what
/// the merge has to keep.
fn authored_blocks(state: &State) -> Vec<PlainNodeEntry> {
    let view = state.view();
    let Some(root) = view.root() else {
        return Vec::new();
    };
    root.children()
        .filter_map(|child| match child {
            ChildView::Block(block) => (!block.id().is_synthetic()).then(|| block.id()),
            ChildView::Leaf(leaf) => {
                (leaf.node().is_some() && !leaf.dot().is_synthetic()).then(|| leaf.dot())
            }
        })
        .filter_map(|dot| editor_state::to_plain_subtree(state, dot))
        .collect()
}

fn is_peer(entry: &PlainNodeEntry) -> bool {
    matches!(entry.node, PlainNode::Paragraph(_))
        && entry
            .children
            .iter()
            .any(|leaf| matches!(&leaf.node, PlainNode::Text(t) if t.text.contains("PEER")))
}

fn plain_contains_peer(doc: &PlainDoc) -> bool {
    fn walk(entry: &editor_model::PlainNodeEntry) -> bool {
        if let PlainNode::Text(t) = &entry.node
            && t.text.contains("PEER")
        {
            return true;
        }
        entry.children.iter().any(walk)
    }
    walk(&doc.root)
}

/// The target read the way the document can hold it: a scaffold the target
/// names but that stopped existing once its slot was filled is projection-owned
/// and never reaches the document. A scaffold that carries something stays in
/// the oracle even when it is gone, so losing its content fails the property.
fn reachable_target(target: &XmlTree, state: &State) -> PlainDoc {
    fn prune(node: &XmlNode, live: &dyn Fn(Dot) -> bool) -> XmlNode {
        fn dropped(block: &XmlNode, live: &dyn Fn(Dot) -> bool) -> bool {
            block.dot.is_some_and(|d| d.is_synthetic() && !live(d)) && carries_nothing(block)
        }
        let mut out = node.clone();
        out.children = node
            .children
            .iter()
            .filter_map(|child| match child {
                XmlChild::Block(block) => {
                    (!dropped(block, live)).then(|| XmlChild::Block(prune(block, live)))
                }
                XmlChild::Inline(_) => Some(child.clone()),
            })
            .collect();
        out
    }
    let view = state.view();
    let live = |d: Dot| view.node(d).is_some() || view.leaf(d).is_some();
    PlainDoc {
        root: prune(&target.root, &live).to_plain_entry(),
    }
}

fn block_dots(state: &State) -> HashSet<Dot> {
    fn walk(nv: editor_model::NodeView<'_>, out: &mut HashSet<Dot>) {
        out.insert(nv.id());
        for child in nv.children() {
            match child {
                ChildView::Block(block) => walk(block, out),
                ChildView::Leaf(leaf) => {
                    if leaf.node().is_some() {
                        out.insert(leaf.dot());
                    }
                }
            }
        }
    }
    let view = state.view();
    let mut out = HashSet::new();
    if let Some(root) = view.root() {
        walk(root, &mut out);
    }
    out
}

fn target_dots(tree: &XmlTree) -> HashSet<Dot> {
    fn walk(n: &XmlNode, out: &mut HashSet<Dot>) {
        if let Some(d) = n.dot {
            out.insert(d);
        }
        for c in n.block_children() {
            walk(c, out);
        }
    }
    let mut out = HashSet::new();
    walk(&tree.root, &mut out);
    out
}

#[test]
fn negative_cases_by_detail() {
    let base = crate::writer::encode_base(&[]).unwrap();
    let wrap = |inner: &str| {
        format!(
            "<root dot=\"1_0\" base=\"{base}\" attr:layout_mode=\"continuous\" attr:max_width=\"600\">{inner}</root>"
        )
    };
    type DetailCheck = fn(&XmlErrorDetail) -> bool;
    let cases: Vec<(&str, DetailCheck)> = vec![
        (
            "<paragraph><em>x</em></paragraph>",
            |d| matches!(d, XmlErrorDetail::UnknownElement { name, hint } if name == "em" && hint.as_deref() == Some("italic")),
        ),
        (
            "<paragraph x=\"1\"/>",
            |d| matches!(d, XmlErrorDetail::UnknownAttribute { element, attr } if element == "paragraph" && attr == "x"),
        ),
        (
            "<list_item><paragraph/></list_item>",
            |d| matches!(d, XmlErrorDetail::ContentRule { parent, got, .. } if parent == "root" && got == &["list_item"]),
        ),
        (
            "<paragraph mod:font_weight=\"150\">x</paragraph>",
            |d| matches!(d, XmlErrorDetail::ValueOutOfRange { modifier, value } if modifier == "font_weight" && value == "150"),
        ),
        ("<paragraph>a\rb</paragraph>", |d| {
            matches!(d, XmlErrorDetail::NewlineInText)
        }),
        (
            "<archived/>",
            |d| matches!(d, XmlErrorDetail::OpaqueNeedsDot { element } if element == "archived"),
        ),
        (
            "<table><table_row><table_cell><fold><fold_title/><fold_content><table><table_row><table_cell><paragraph/></table_cell></table_row></table></fold_content></fold></table_cell></table_row></table><paragraph/>",
            |d| matches!(d, XmlErrorDetail::ContextNotAllowed { element } if element == "table"),
        ),
        (
            "<fold><fold_content><paragraph/></fold_content><fold_title/></fold><paragraph/>",
            |d| matches!(d, XmlErrorDetail::ContentRule { parent, got, .. } if parent == "fold" && got == &["fold_content", "fold_title"]),
        ),
    ];
    for (inner, detail_is) in cases {
        let err = from_xml(&wrap(inner)).unwrap_err();
        assert!(detail_is(&err.detail), "{inner}: {:?}", err.detail);
    }

    let err = from_xml("<root dot=\"1_0\" base=\"!!\"/>").unwrap_err();
    assert_eq!(*err.detail, XmlErrorDetail::BaseUndecodable);
}
