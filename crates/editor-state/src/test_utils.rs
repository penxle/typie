use std::collections::{BTreeMap, HashMap};

use editor_crdt::{Dot, ListOp, OpGraph};
use editor_model::{
    Anchor, AtomLeaf, Bias, EditOp, Modifier, ModifierAttrOp, ModifierType, NodeAttrOp, PlainDoc,
    PlainNode, PlainNodeEntry, SeqClass, SeqItem, SpanOp, classify,
};

use crate::{ProjectedState, State};

/// Builds a projected [`State`] from a [`PlainDoc`] template (the shape emitted
/// by the `state!` macro). Returns a map from each node's child-index path (from
/// the root; root = empty path) to the projected `Dot`s: block nodes map to their
/// own block `Dot`, text nodes map to their containing block (text has no
/// projected identity).
pub fn build_state_from_plain(plain: PlainDoc) -> (State, HashMap<Vec<usize>, Dot>) {
    build_state_from_plain_with_actor(plain, 1)
}

/// Like [`build_state_from_plain`], but allocates the document's `Dot`s under
/// `actor`. Tests that need two documents with disjoint `Dot` namespaces (e.g.
/// to verify that a stable selection captured from one does not spuriously
/// resolve against the other) build the second with a distinct actor.
pub fn build_state_from_plain_with_actor(
    plain: PlainDoc,
    actor: u64,
) -> (State, HashMap<Vec<usize>, Dot>) {
    let mut graph = OpGraph::<EditOp>::with_actor(actor);
    let mut handles: HashMap<Vec<usize>, Dot> = HashMap::new();
    let mut seq_pos: usize = 0;

    emit_node(
        &plain.root,
        &[],
        &mut Vec::new(),
        &mut graph,
        &mut handles,
        &mut seq_pos,
    );

    graph.commit_mut();
    let projected = ProjectedState::from_graph(graph).expect("template always projects");
    (State::new(projected, None), handles)
}

fn emit_node(
    entry: &PlainNodeEntry,
    parents: &[Dot],
    path: &mut Vec<usize>,
    graph: &mut OpGraph<EditOp>,
    handles: &mut HashMap<Vec<usize>, Dot>,
    seq_pos: &mut usize,
) {
    let node_type = entry.node.as_type();

    match classify(node_type) {
        SeqClass::Block => {
            // The root is implicit (Dot::ROOT): no Block op in the seq, children
            // parent to Dot::ROOT, and its overlays target Dot::ROOT.
            let dot = if matches!(entry.node, PlainNode::Root(_)) {
                Dot::ROOT
            } else {
                let d = graph
                    .add_mut(EditOp::Seq(ListOp::Ins {
                        pos: *seq_pos,
                        item: SeqItem::Block {
                            node_type,
                            parents: parents.to_vec(),
                        },
                    }))
                    .expect("local seq block insert never conflicts")
                    .id;
                *seq_pos += 1;
                d
            };
            handles.insert(path.clone(), dot);

            for modifier in entry.modifiers.values() {
                graph
                    .add_mut(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                        target: dot,
                        modifier: modifier.clone(),
                    }))
                    .expect("local block modifier never conflicts");
            }
            let mut carry_by_type: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
            for m in &entry.carry {
                if m.as_type().is_carry_kind() {
                    carry_by_type.insert(m.as_type(), m.clone());
                }
            }
            for modifier in carry_by_type.into_values() {
                graph
                    .add_mut(EditOp::NodeCarry(ModifierAttrOp::SetModifier {
                        target: dot,
                        modifier,
                    }))
                    .expect("local node carry never conflicts");
            }
            for attr in entry.node.to_attrs() {
                graph
                    .add_mut(EditOp::NodeAttr(NodeAttrOp { target: dot, attr }))
                    .expect("local node attr never conflicts");
            }

            let mut child_parents = parents.to_vec();
            child_parents.push(dot);
            for (i, child) in entry.children.iter().enumerate() {
                path.push(i);
                emit_node(child, &child_parents, path, graph, handles, seq_pos);
                path.pop();
            }
        }
        SeqClass::Text => {
            if let PlainNode::Text(text_node) = &entry.node {
                let mut char_dots = Vec::with_capacity(text_node.text.chars().count());
                for ch in text_node.text.chars() {
                    let d = graph
                        .add_mut(EditOp::Seq(ListOp::Ins {
                            pos: *seq_pos,
                            item: SeqItem::Char(ch),
                        }))
                        .expect("local char insert never conflicts")
                        .id;
                    *seq_pos += 1;
                    char_dots.push(d);
                }
                if let (Some(&first), Some(&last)) = (char_dots.first(), char_dots.last()) {
                    for modifier in entry.modifiers.values() {
                        graph
                            .add_mut(EditOp::Span(SpanOp::AddSpan {
                                start: Anchor {
                                    id: first,
                                    bias: Bias::Before,
                                },
                                end: Anchor {
                                    id: last,
                                    bias: Bias::After,
                                },
                                modifier: modifier.clone(),
                            }))
                            .expect("local span never conflicts");
                    }
                }
            }
            if let Some(parent_dot) = parents.last() {
                handles.insert(path.clone(), *parent_dot);
            }
        }
        SeqClass::Atom => {
            let leaf = AtomLeaf::from_plain_node(&entry.node).expect("atom plain node converts");
            let item = if leaf.is_block_level() {
                SeqItem::BlockAtom {
                    leaf,
                    parents: parents.to_vec(),
                }
            } else {
                SeqItem::Atom(leaf)
            };
            let dot = graph
                .add_mut(EditOp::Seq(ListOp::Ins {
                    pos: *seq_pos,
                    item,
                }))
                .expect("local seq atom insert never conflicts")
                .id;
            *seq_pos += 1;
            handles.insert(path.clone(), dot);

            for modifier in entry.modifiers.values() {
                graph
                    .add_mut(EditOp::Span(SpanOp::AddSpan {
                        start: Anchor {
                            id: dot,
                            bias: Bias::Before,
                        },
                        end: Anchor {
                            id: dot,
                            bias: Bias::After,
                        },
                        modifier: modifier.clone(),
                    }))
                    .expect("local atom span never conflicts");
            }
        }
    }
}

// ── assert_state_eq ──────────────────────────────────────────────────────────
// Structural state equality for tests: compares the projected tree (node types,
// inline content, effective/own modifiers, carries) ignoring the concrete
// `Dot` identities, plus selection-by-path and pending state.

fn block_fingerprint(state: &State, block: &editor_model::NodeView, out: &mut Vec<String>) {
    let carry: Vec<editor_model::Modifier> = state
        .projected
        .carry_modifiers(block.id())
        .into_values()
        .collect();
    let mut mods: Vec<editor_model::Modifier> = block.effective().values().cloned().collect();
    mods.sort_by_key(editor_model::Modifier::as_type);
    out.push(format!(
        "OPEN {:?} carry={:?} mods={:?}",
        block.node_type(),
        carry,
        mods
    ));
    for (slot, child) in block.children().enumerate() {
        match child {
            editor_model::ChildView::Block(b) => block_fingerprint(state, &b, out),
            editor_model::ChildView::Leaf(l) => {
                let mut lmods: Vec<editor_model::Modifier> = block
                    .leaf_state_at(slot)
                    .map(|st| st.own.values().map(|o| o.value.clone()).collect())
                    .unwrap_or_default();
                lmods.sort_by_key(editor_model::Modifier::as_type);
                let content = match l.as_char() {
                    Some(c) => format!("char {c:?}"),
                    None => format!("atom {:?}", l.node_type()),
                };
                out.push(format!("LEAF {content} mods={lmods:?}"));
            }
        }
    }
    out.push("CLOSE".to_string());
}

fn doc_fingerprint(state: &State) -> Vec<String> {
    let view = state.view();
    let mut out = Vec::new();
    if let Some(root) = view.root() {
        block_fingerprint(state, &root, &mut out);
    }
    out
}

fn selection_path(
    view: &editor_model::DocView,
    pos: &crate::Position,
) -> Option<(Vec<usize>, usize)> {
    pos.resolve(view).map(|r| (r.path().to_vec(), pos.offset))
}

pub fn assert_state_eq_impl(actual: &State, expected: &State) {
    assert_eq!(
        doc_fingerprint(actual),
        doc_fingerprint(expected),
        "document structure differs"
    );

    match (&actual.selection, &expected.selection) {
        (None, None) => {}
        (Some(_), None) => panic!("Selection mismatch: actual has Some, expected has None"),
        (None, Some(_)) => panic!("Selection mismatch: actual has None, expected has Some"),
        (Some(s1), Some(s2)) => {
            let av = actual.view();
            let ev = expected.view();
            assert_eq!(
                selection_path(&av, &s1.anchor),
                selection_path(&ev, &s2.anchor),
                "selection anchors differ"
            );
            assert_eq!(
                selection_path(&av, &s1.head),
                selection_path(&ev, &s2.head),
                "selection heads differ"
            );
        }
    }

    assert_eq!(
        actual.pending_modifiers, expected.pending_modifiers,
        "pending modifiers differ"
    );
}

#[macro_export]
macro_rules! assert_state_eq {
    ($actual:expr, $expected:expr) => {
        $crate::test_utils::assert_state_eq_impl(&$actual, &$expected)
    };
}

/// Document-structure-only equality (ignores selection and pending state) — the
/// projected-model replacement for the old `editor_model::assert_doc_eq!`.
pub fn assert_doc_eq_impl(actual: &State, expected: &State) {
    assert_eq!(
        doc_fingerprint(actual),
        doc_fingerprint(expected),
        "document structure differs"
    );
}

#[macro_export]
macro_rules! assert_doc_eq {
    ($actual:expr, $expected:expr) => {
        $crate::test_utils::assert_doc_eq_impl(&$actual, &$expected)
    };
}
