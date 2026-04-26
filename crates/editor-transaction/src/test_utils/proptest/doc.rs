use std::collections::BTreeMap;

use editor_model::{ContentExpr, Doc, Node, NodeId, NodeType, Schema, Subtree, TextNode, imbl};
use editor_state::{Position, Selection, State};
use proptest::collection::vec as prop_vec;
use proptest::prelude::*;

use crate::test_utils::proptest::text::arb_unicode_text;

#[derive(Debug, Clone, Default)]
pub struct DocIndex {
    pub by_type: BTreeMap<NodeType, Vec<NodeId>>,
}

impl DocIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, ty: NodeType, id: NodeId) {
        self.by_type.entry(ty).or_default().push(id);
    }

    pub fn get(&self, ty: NodeType) -> &[NodeId] {
        self.by_type.get(&ty).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

const MAX_DEPTH: usize = 4;
const MAX_NODES: usize = 30;
const TEXT_MIN: usize = 0;
const TEXT_MAX: usize = 12;
const INLINE_SEQ_MAX: usize = 4;
const BLOCK_SEQ_MAX: usize = 3;

pub fn arb_doc() -> impl Strategy<Value = (State, DocIndex)> {
    arb_subtree(NodeType::Root, 0).prop_map(build_state_from_root)
}

pub(crate) fn arb_subtree(ty: NodeType, depth: usize) -> BoxedStrategy<Subtree> {
    if matches!(ty, NodeType::Text) {
        return arb_unicode_text(TEXT_MIN, TEXT_MAX)
            .prop_map(|s| Subtree::leaf(NodeId::new(), Node::Text(TextNode { text: s })))
            .boxed();
    }

    let spec = Schema::node_spec(ty);
    if spec.is_leaf() {
        return Just(())
            .prop_map(move |()| Subtree::leaf(NodeId::new(), ty.into_node()))
            .boxed();
    }

    let base_cap = if spec.is_textblock() {
        INLINE_SEQ_MAX
    } else {
        BLOCK_SEQ_MAX
    };
    let cap = base_cap.saturating_sub(depth);
    arb_for_content(spec.content.clone(), depth, cap)
        .prop_map(move |children| {
            Subtree::leaf(NodeId::new(), ty.into_node()).with_children(children)
        })
        .boxed()
}

fn arb_for_content(expr: ContentExpr, depth: usize, cap: usize) -> BoxedStrategy<Vec<Subtree>> {
    match expr {
        ContentExpr::Empty => Just(Vec::new()).boxed(),
        ContentExpr::Single(ty) => arb_subtree(ty, depth + 1).prop_map(|s| vec![s]).boxed(),
        ContentExpr::Choice(choices) => arb_one_from_choice(choices, depth)
            .prop_map(|s| vec![s])
            .boxed(),
        ContentExpr::Optional(inner) => {
            if depth >= MAX_DEPTH {
                Just(Vec::new()).boxed()
            } else {
                let inner_owned = *inner;
                prop::option::of(arb_one_from_atom(inner_owned, depth))
                    .prop_map(|opt| opt.map(|s| vec![s]).unwrap_or_default())
                    .boxed()
            }
        }
        ContentExpr::ZeroOrMore(inner) => {
            let max = if depth >= MAX_DEPTH { 0 } else { cap };
            arb_repeated(*inner, depth, 0, max)
        }
        ContentExpr::OneOrMore(inner) => {
            let max = if depth >= MAX_DEPTH { 1 } else { cap.max(1) };
            arb_repeated(*inner, depth, 1, max)
        }
        ContentExpr::Seq(items) => {
            items
                .into_iter()
                .fold(Just(Vec::<Subtree>::new()).boxed(), |acc, sub| {
                    let sub_strategy = arb_for_content(sub, depth, cap);
                    (acc, sub_strategy)
                        .prop_map(|(mut head, tail)| {
                            head.extend(tail);
                            head
                        })
                        .boxed()
                })
        }
    }
}

fn arb_repeated(
    inner: ContentExpr,
    depth: usize,
    min: usize,
    max: usize,
) -> BoxedStrategy<Vec<Subtree>> {
    prop_vec(arb_one_from_atom(inner, depth), min..=max).boxed()
}

fn arb_one_from_atom(atom: ContentExpr, depth: usize) -> BoxedStrategy<Subtree> {
    match atom {
        ContentExpr::Single(ty) => arb_subtree(ty, depth + 1),
        ContentExpr::Choice(choices) => arb_one_from_choice(choices, depth),
        other => panic!("unexpected atom in repeated/optional: {other:?}"),
    }
}

fn arb_one_from_choice(choices: Vec<ContentExpr>, depth: usize) -> BoxedStrategy<Subtree> {
    let types: Vec<NodeType> = choices
        .into_iter()
        .map(|c| match c {
            ContentExpr::Single(ty) => ty,
            other => panic!("Choice expected to contain only Single variants, got {other:?}"),
        })
        .collect();
    let pool: Vec<NodeType> = if depth >= MAX_DEPTH {
        // 재귀 종료 보장: min_required > 0 타입은 제외해 무한 후손 차단.
        let leafy: Vec<_> = types
            .iter()
            .copied()
            .filter(|t| Schema::node_spec(*t).content.min_required() == 0)
            .collect();
        if leafy.is_empty() { types } else { leafy }
    } else {
        types
    };
    proptest::sample::select(pool)
        .prop_flat_map(move |ty| arb_subtree(ty, depth + 1))
        .boxed()
}

fn build_state_from_root(root: Subtree) -> (State, DocIndex) {
    let mut doc = Doc::new_test();
    let mut index = DocIndex::new();
    index.insert(NodeType::Root, NodeId::ROOT);

    let child_ids: imbl::Vector<NodeId> = root.children.iter().map(|c| c.id).collect();
    for child in root.children {
        for (id, entry) in child.into_entries(NodeId::ROOT) {
            index.insert(entry.node.as_type(), id);
            doc = doc.insert_node(id, entry);
        }
    }
    doc = doc.with_node_updated(NodeId::ROOT, |entry| entry.with_children(child_ids));

    let target = index
        .get(NodeType::Text)
        .first()
        .copied()
        .or_else(|| index.get(NodeType::Paragraph).first().copied())
        .unwrap_or(NodeId::ROOT);
    let selection = Selection::collapsed(Position::new(target, 0));

    (State::new(doc, selection), index)
}

#[cfg(test)]
mod sanity {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn arb_doc_produces_valid_shape((state, index) in arb_doc()) {
            prop_assert!(index.get(NodeType::Root).contains(&NodeId::ROOT));
            prop_assert!(index.get(NodeType::Paragraph).len() >= 1);
            let head_id = state.selection.head.node_id;
            prop_assert!(state.doc.get_entry(head_id).is_some());
        }

        #[test]
        fn arb_doc_produces_schema_valid_state((state, _index) in arb_doc()) {
            walk_and_validate(&state.doc, NodeId::ROOT)?;
        }

        #[test]
        fn arb_doc_respects_node_count_limit((_state, index) in arb_doc()) {
            let total: usize = index.by_type.values().map(|v| v.len()).sum();
            prop_assert!(total <= MAX_NODES + 5, "produced {} nodes, max {}", total, MAX_NODES + 5);
        }
    }

    fn walk_and_validate(doc: &Doc, id: NodeId) -> Result<(), TestCaseError> {
        let entry = doc
            .get_entry(id)
            .ok_or_else(|| TestCaseError::fail(format!("node {id:?} missing from doc")))?;
        let parent_type = entry.node.as_type();
        let spec = Schema::node_spec(parent_type);
        let child_types: Vec<NodeType> = entry
            .children
            .iter()
            .filter_map(|cid| doc.get_entry(*cid).map(|e| e.node.as_type()))
            .collect();
        if let Err(e) = spec.content.validate(&child_types) {
            return Err(TestCaseError::fail(format!(
                "schema violation under {parent_type:?}: children={child_types:?} err={e:?}"
            )));
        }
        for cid in &entry.children {
            walk_and_validate(doc, *cid)?;
        }
        Ok(())
    }
}
