use editor_model::{
    Alignment, Doc, DocumentAttrs, Modifier, Node, NodeId, NodeType, Schema, Subtree,
};
use editor_state::State;
use proptest::prelude::*;
use proptest::strategy::Union;

use crate::Step;
use crate::test_utils::proptest::doc::{DocIndex, arb_subtree};
use crate::test_utils::proptest::text::arb_unicode_text;

fn arb_insert_text(state: State, index: DocIndex) -> Option<BoxedStrategy<Step>> {
    let text_ids: Vec<NodeId> = index.get(NodeType::Text).to_vec();
    if text_ids.is_empty() {
        return None;
    }
    Some(
        proptest::sample::select(text_ids)
            .prop_flat_map(move |id| {
                let len = text_node_char_len(&state, id).unwrap_or(0);
                (0usize..=len, arb_unicode_text(1, 4)).prop_map(move |(offset, text)| {
                    Step::InsertText {
                        node_id: id,
                        offset,
                        text,
                    }
                })
            })
            .boxed(),
    )
}

fn arb_remove_text(state: State, index: DocIndex) -> Option<BoxedStrategy<Step>> {
    // 빈 Text 노드는 RemoveText 대상에서 제외 (schema가 빈 Text leaf 허용). select 후 reject 대신 풀에서 미리 제거해 reject rate 0.
    let text_ids: Vec<NodeId> = index
        .get(NodeType::Text)
        .iter()
        .copied()
        .filter(|id| text_node_char_len(&state, *id).unwrap_or(0) > 0)
        .collect();
    if text_ids.is_empty() {
        return None;
    }
    let state_for_slice = state.clone();
    Some(
        proptest::sample::select(text_ids)
            .prop_flat_map(move |id| {
                let len = text_node_char_len(&state_for_slice, id).unwrap_or(0);
                let max_remove = len.min(4);
                let state2 = state_for_slice.clone();
                (0usize..len, 1usize..=max_remove).prop_filter_map(
                    "offset+len <= text_len",
                    move |(offset, remove_len)| {
                        if offset + remove_len > len {
                            return None;
                        }
                        // char 단위 슬라이스로 transform과 동일한 단위 보장.
                        let text = text_node_chars_slice(&state2, id, offset, remove_len)?;
                        Some(Step::RemoveText {
                            node_id: id,
                            offset,
                            text,
                        })
                    },
                )
            })
            .boxed(),
    )
}

fn text_node_char_len(state: &State, id: NodeId) -> Option<usize> {
    let entry = state.doc.get_entry(id)?;
    if let Node::Text(t) = &entry.node {
        Some(t.text.chars().count())
    } else {
        None
    }
}

fn text_node_chars_slice(state: &State, id: NodeId, offset: usize, len: usize) -> Option<String> {
    let entry = state.doc.get_entry(id)?;
    if let Node::Text(t) = &entry.node {
        Some(t.text.chars().skip(offset).take(len).collect())
    } else {
        None
    }
}

fn arb_modifier() -> impl Strategy<Value = Modifier> {
    let variants: Vec<Modifier> = vec![
        Modifier::Bold,
        Modifier::Italic,
        Modifier::Underline,
        Modifier::Strikethrough,
        Modifier::FontSize { value: 1600 },
        Modifier::FontFamily {
            value: "Pretendard".into(),
        },
        Modifier::FontWeight { value: 400 },
        Modifier::TextColor {
            value: "#000000".into(),
        },
        Modifier::BackgroundColor {
            value: "#ffffff".into(),
        },
        Modifier::LetterSpacing { value: 0 },
        Modifier::Link {
            href: "https://example.com".into(),
        },
        Modifier::Ruby {
            text: "ruby".into(),
        },
        Modifier::LineHeight { value: 160 },
        Modifier::BlockGap { value: 100 },
        Modifier::ParagraphIndent { value: 0 },
        Modifier::Alignment {
            value: Alignment::Left,
        },
    ];
    proptest::sample::select(variants)
}

fn collect_modifier_targets(index: &DocIndex) -> Vec<NodeId> {
    index
        .by_type
        .iter()
        .filter(|(ty, _)| !matches!(**ty, NodeType::Root))
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect()
}

fn arb_add_modifier(_state: State, index: DocIndex) -> Option<BoxedStrategy<Step>> {
    let target_ids = collect_modifier_targets(&index);
    if target_ids.is_empty() {
        return None;
    }
    Some(
        (proptest::sample::select(target_ids), arb_modifier())
            .prop_map(|(node_id, modifier)| Step::AddModifier { node_id, modifier })
            .boxed(),
    )
}

fn arb_remove_modifier(_state: State, index: DocIndex) -> Option<BoxedStrategy<Step>> {
    let target_ids = collect_modifier_targets(&index);
    if target_ids.is_empty() {
        return None;
    }
    Some(
        (proptest::sample::select(target_ids), arb_modifier())
            .prop_map(|(node_id, modifier)| Step::RemoveModifier { node_id, modifier })
            .boxed(),
    )
}

fn arb_set_modifiers(state: State, index: DocIndex) -> Option<BoxedStrategy<Step>> {
    let target_ids = collect_modifier_targets(&index);
    if target_ids.is_empty() {
        return None;
    }
    Some(
        proptest::sample::select(target_ids)
            .prop_flat_map(move |node_id| {
                let state = state.clone();
                proptest::collection::vec(arb_modifier(), 0..=3).prop_map(move |new_mods| {
                    let old = state
                        .doc
                        .get_entry(node_id)
                        .map(|e| e.modifiers.clone())
                        .unwrap_or_default();
                    Step::SetModifiers {
                        node_id,
                        old_modifiers: old,
                        new_modifiers: new_mods,
                    }
                })
            })
            .boxed(),
    )
}

fn arb_set_node(state: State, index: DocIndex) -> Option<BoxedStrategy<Step>> {
    let target_ids: Vec<NodeId> = index
        .by_type
        .iter()
        .filter(|(ty, _)| !matches!(**ty, NodeType::Root | NodeType::Text))
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    if target_ids.is_empty() {
        return None;
    }
    Some(
        proptest::sample::select(target_ids)
            .prop_filter_map("entry must exist", move |id| {
                let entry = state.doc.get_entry(id)?;
                let ty = entry.node.as_type();
                Some(Step::SetNode {
                    node_id: id,
                    old_node: entry.node.clone(),
                    new_node: ty.into_node(),
                })
            })
            .boxed(),
    )
}

fn arb_set_document_attrs(state: State, _index: DocIndex) -> BoxedStrategy<Step> {
    let old = state.doc.attrs.clone();
    Just(Step::SetDocumentAttrs {
        old,
        new: DocumentAttrs::default(),
    })
    .boxed()
}

fn arb_insert_subtree(state: State, index: DocIndex) -> Option<BoxedStrategy<Step>> {
    let parent_candidates: Vec<NodeId> = index
        .by_type
        .iter()
        .filter(|(ty, _)| !Schema::node_spec(**ty).is_leaf())
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    if parent_candidates.is_empty() {
        return None;
    }

    Some(
        proptest::sample::select(parent_candidates)
            .prop_filter_map(
                "parent must accept at least one child type",
                move |parent_id| {
                    let entry = state.doc.get_entry(parent_id)?;
                    let parent_type = entry.node.as_type();
                    let allowed = Schema::node_spec(parent_type).content.allowed_types();
                    if allowed.is_empty() {
                        return None;
                    }
                    let current_len = entry.children.len();
                    Some((parent_id, allowed, current_len))
                },
            )
            .prop_flat_map(|(parent_id, allowed, current_len)| {
                proptest::sample::select(allowed).prop_flat_map(move |child_ty| {
                    (
                        Just(parent_id),
                        arb_subtree(child_ty, 1),
                        0usize..=current_len,
                    )
                })
            })
            .prop_map(|(parent_id, subtree, idx)| Step::InsertSubtree {
                parent_id,
                index: idx,
                subtree,
            })
            .boxed(),
    )
}

fn arb_remove_subtree(state: State, index: DocIndex) -> Option<BoxedStrategy<Step>> {
    // structural=true children belong to SplitNode/MergeNode/MoveNode territory — exclude here.
    let candidates: Vec<(NodeId, usize, NodeId)> = index
        .by_type
        .values()
        .flatten()
        .copied()
        .filter_map(|parent_id| {
            let entry = state.doc.get_entry(parent_id)?;
            entry
                .children
                .iter()
                .enumerate()
                .filter_map(|(i, cid)| {
                    let cty = state.doc.get_entry(*cid)?.node.as_type();
                    if Schema::node_spec(cty).structural {
                        return None;
                    }
                    Some((parent_id, i, *cid))
                })
                .next()
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let state_for_capture = state.clone();
    Some(
        proptest::sample::select(candidates)
            .prop_filter_map(
                "subtree capture must succeed",
                move |(parent_id, idx, cid)| {
                    let subtree = Subtree::capture(&state_for_capture.doc, cid)?;
                    Some(Step::RemoveSubtree {
                        parent_id,
                        index: idx,
                        subtree,
                    })
                },
            )
            .boxed(),
    )
}

fn arb_split_node(state: State, index: DocIndex) -> Option<BoxedStrategy<Step>> {
    let candidates: Vec<NodeId> = index
        .by_type
        .iter()
        .filter(|(ty, _)| Schema::node_spec(**ty).is_textblock())
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    if candidates.is_empty() {
        return None;
    }
    Some(
        proptest::sample::select(candidates)
            .prop_flat_map(move |node_id| {
                let upper = state
                    .doc
                    .get_entry(node_id)
                    .map(|e| e.children.len())
                    .unwrap_or(0);
                (Just(node_id), 0usize..=upper).prop_map(|(node_id, offset)| Step::SplitNode {
                    node_id,
                    offset,
                    new_node_id: NodeId::new(),
                })
            })
            .boxed(),
    )
}

fn arb_merge_node(state: State, _index: DocIndex) -> Option<BoxedStrategy<Step>> {
    let pairs = collect_adjacent_mergeable_pairs(&state);
    if pairs.is_empty() {
        return None;
    }
    Some(
        proptest::sample::select(pairs)
            .prop_map(|(node_id, target_id, offset)| Step::MergeNode {
                node_id,
                target_id,
                offset,
            })
            .boxed(),
    )
}

fn arb_move_node(state: State, _index: DocIndex) -> Option<BoxedStrategy<Step>> {
    // Strategy 구성 시점에 valid (node, old_parent, old_index, new_parent, new_index) 5-tuple만 수집해 reject 0.
    let all_ids = walk_node_ids(&state.doc);
    let mut ready: Vec<(NodeId, NodeId, usize, NodeId, usize)> = Vec::new();
    for p_id in &all_ids {
        let entry = match state.doc.get_entry(*p_id) {
            Some(e) => e,
            None => continue,
        };
        for (i, cid) in entry.children.iter().copied().enumerate() {
            let cty = match state.doc.get_entry(cid) {
                Some(e) => e.node.as_type(),
                None => continue,
            };
            if Schema::node_spec(cty).structural {
                continue;
            }
            let forbidden = collect_subtree_ids(&state.doc, cid);
            for np in &all_ids {
                if *np == *p_id || forbidden.contains(np) {
                    continue;
                }
                let np_entry = match state.doc.get_entry(*np) {
                    Some(e) => e,
                    None => continue,
                };
                if !Schema::node_spec(np_entry.node.as_type())
                    .content
                    .matches(cty)
                {
                    continue;
                }
                ready.push((cid, *p_id, i, *np, np_entry.children.len()));
            }
        }
    }

    if ready.is_empty() {
        return None;
    }

    Some(
        proptest::sample::select(ready)
            .prop_map(
                |(node_id, old_parent, old_index, new_parent, new_index)| Step::MoveNode {
                    node_id,
                    old_parent,
                    old_index,
                    new_parent,
                    new_index,
                },
            )
            .boxed(),
    )
}

pub(crate) fn collect_adjacent_mergeable_pairs(state: &State) -> Vec<(NodeId, NodeId, usize)> {
    let mut out = Vec::new();
    for parent_id in walk_node_ids(&state.doc) {
        let entry = match state.doc.get_entry(parent_id) {
            Some(e) => e,
            None => continue,
        };
        for window in entry
            .children
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .windows(2)
        {
            let (a, b) = (window[0], window[1]);
            let a_ty = state.doc.get_entry(a).map(|e| e.node.as_type());
            let b_ty = state.doc.get_entry(b).map(|e| e.node.as_type());
            if a_ty == b_ty
                && a_ty
                    .map(|t| Schema::node_spec(t).is_textblock())
                    .unwrap_or(false)
            {
                // offset = target(a)의 merge 이전 자식 수. inverse SplitNode가 정확히 이 child-index에서 다시 분리됨.
                let offset = state
                    .doc
                    .get_entry(a)
                    .map(|e| e.children.len())
                    .unwrap_or(0);
                out.push((b, a, offset));
            }
        }
    }
    out
}

pub(crate) fn walk_node_ids(doc: &Doc) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack = vec![NodeId::ROOT];
    while let Some(id) = stack.pop() {
        out.push(id);
        if let Some(e) = doc.get_entry(id) {
            for c in &e.children {
                stack.push(*c);
            }
        }
    }
    out
}

fn collect_subtree_ids(doc: &Doc, root: NodeId) -> std::collections::HashSet<NodeId> {
    let mut out = std::collections::HashSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !out.insert(id) {
            continue;
        }
        if let Some(e) = doc.get_entry(id) {
            for c in &e.children {
                stack.push(*c);
            }
        }
    }
    out
}

pub(crate) fn arb_step_for_anchor(state: State, anchor: NodeId) -> BoxedStrategy<Step> {
    let entry = match state.doc.get_entry(anchor) {
        Some(e) => e,
        None => {
            return Just(Step::AddModifier {
                node_id: anchor,
                modifier: Modifier::Bold,
            })
            .boxed();
        }
    };
    if let Node::Text(t) = &entry.node {
        let len = t.text.chars().count();
        return (0usize..=len, arb_unicode_text(1, 3))
            .prop_map(move |(offset, text)| Step::InsertText {
                node_id: anchor,
                offset,
                text,
            })
            .boxed();
    }
    arb_modifier()
        .prop_map(move |m| Step::AddModifier {
            node_id: anchor,
            modifier: m,
        })
        .boxed()
}

pub fn arb_syncable_step(state: State, index: DocIndex) -> BoxedStrategy<Step> {
    let mut branches: Vec<(u32, BoxedStrategy<Step>)> = Vec::new();
    if let Some(s) = arb_insert_text(state.clone(), index.clone()) {
        branches.push((175, s));
    }
    if let Some(s) = arb_remove_text(state.clone(), index.clone()) {
        branches.push((175, s));
    }
    if let Some(s) = arb_add_modifier(state.clone(), index.clone()) {
        branches.push((63, s));
    }
    if let Some(s) = arb_remove_modifier(state.clone(), index.clone()) {
        branches.push((63, s));
    }
    if let Some(s) = arb_set_modifiers(state.clone(), index.clone()) {
        branches.push((63, s));
    }
    if let Some(s) = arb_set_node(state.clone(), index.clone()) {
        branches.push((63, s));
    }
    if let Some(s) = arb_insert_subtree(state.clone(), index.clone()) {
        branches.push((75, s));
    }
    if let Some(s) = arb_remove_subtree(state.clone(), index.clone()) {
        branches.push((75, s));
    }
    if let Some(s) = arb_split_node(state.clone(), index.clone()) {
        branches.push((50, s));
    }
    if let Some(s) = arb_merge_node(state.clone(), index.clone()) {
        branches.push((50, s));
    }
    if let Some(s) = arb_move_node(state.clone(), index.clone()) {
        branches.push((50, s));
    }
    branches.push((100, arb_set_document_attrs(state.clone(), index.clone())));
    if branches.is_empty() {
        // SetDocumentAttrs branch always pushes, so this fallback is defensive — kept for safety if generators evolve.
        return Just(Step::InsertText {
            node_id: NodeId::ROOT,
            offset: 0,
            text: String::new(),
        })
        .boxed();
    }
    Union::new_weighted(branches).boxed()
}

#[cfg(test)]
mod sanity {
    use super::*;
    use crate::test_utils::proptest::doc::arb_doc;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn step_applies_to_state(((state, _index), step) in arb_doc().prop_flat_map(|(s, i)| {
            let s2 = s.clone();
            let i2 = i.clone();
            (Just((s, i)), arb_syncable_step(s2, i2))
        })) {
            // 모든 분기가 비어있을 때만 발생하는 fallback (현재 generator로는 도달 불가) — apply 불가하므로 skip.
            prop_assume!(!matches!(
                &step,
                Step::InsertText { node_id, text, .. }
                    if *node_id == NodeId::ROOT && text.is_empty()
            ));
            prop_assert!(step.apply(&state).is_ok(), "step must apply: step={step:?}");
        }
    }

    #[test]
    fn full_distribution_covers_all_groups() {
        use proptest::strategy::ValueTree;
        use proptest::test_runner::{Config, TestRunner};
        use std::collections::HashSet;

        let mut runner = TestRunner::new(Config::with_cases(512));
        let strategy = arb_doc().prop_flat_map(|(s, i)| {
            let s2 = s.clone();
            let i2 = i.clone();
            (Just((s, i)), arb_syncable_step(s2, i2))
        });
        let mut seen: HashSet<&'static str> = HashSet::new();
        for _ in 0..512 {
            let value = strategy.new_tree(&mut runner).unwrap().current();
            let (_, step) = value;
            seen.insert(step_group(&step));
        }
        for g in [
            "text", "modifier", "subtree", "split", "merge", "move", "document",
        ] {
            assert!(seen.contains(g), "group {g} never produced; seen={seen:?}");
        }
    }

    fn step_group(step: &Step) -> &'static str {
        match step {
            Step::InsertText { .. } | Step::RemoveText { .. } => "text",
            Step::AddModifier { .. }
            | Step::RemoveModifier { .. }
            | Step::SetModifiers { .. }
            | Step::SetNode { .. } => "modifier",
            Step::InsertSubtree { .. } | Step::RemoveSubtree { .. } => "subtree",
            Step::SplitNode { .. } => "split",
            Step::MergeNode { .. } => "merge",
            Step::MoveNode { .. } => "move",
            Step::SetDocumentAttrs { .. } => "document",
            _ => "other",
        }
    }
}
