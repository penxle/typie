use editor_model::{Doc, Node, NodeId};
use editor_state::State;

use crate::{HistoryMeta, StepError, Transaction};

pub fn build_revert_transaction(state: &State, target: &Doc) -> Result<Transaction, StepError> {
    let mut tr = Transaction::new(state);
    tr.update_meta(|m| m.history = HistoryMeta::Skip);
    tr.batch::<_, StepError>(|tr| {
        reconcile_styles(tr, target)?;
        reconcile_node(tr, target, NodeId::ROOT)?;
        Ok(())
    })?;
    Ok(tr)
}

fn reconcile_node(tr: &mut Transaction, target: &Doc, id: NodeId) -> Result<(), StepError> {
    reconcile_attrs(tr, target, id)?;
    reconcile_modifiers(tr, target, id)?;
    reconcile_node_style(tr, target, id)?;
    reconcile_node_marker(tr, target, id)?;
    reconcile_text(tr, target, id)?;
    reconcile_children(tr, target, id)?;
    Ok(())
}

fn reconcile_children(tr: &mut Transaction, target: &Doc, id: NodeId) -> Result<(), StepError> {
    let target_children = children_ids(target, id);
    let target_set: std::collections::HashSet<NodeId> = target_children.iter().copied().collect();

    for cid in current_children(tr, id) {
        if !target_set.contains(&cid) {
            tr.remove_subtree(cid)?;
        }
    }

    for (index, &cid) in target_children.iter().enumerate() {
        let live_somewhere = tr.doc().get_entry(cid).is_some();
        if live_somewhere {
            let cur = current_children(tr, id);
            let cur_index = cur.iter().position(|&x| x == cid);
            if cur_index != Some(index) {
                tr.move_node(cid, id, index)?;
            }
        } else {
            revive_node(tr, target, cid, id, index)?;
        }
        reconcile_node(tr, target, cid)?;
    }
    Ok(())
}

fn revive_node(
    tr: &mut Transaction,
    target: &Doc,
    cid: NodeId,
    parent: NodeId,
    index: usize,
) -> Result<(), StepError> {
    let node_type = target
        .get_entry(cid)
        .expect("revive_node: target must contain cid")
        .node
        .as_type();
    let empty = node_type.into_node().to_plain();
    tr.insert_subtree(parent, index, editor_model::Subtree::leaf(cid, empty))?;
    Ok(())
}

fn children_ids(doc: &Doc, id: NodeId) -> Vec<NodeId> {
    match doc.get_entry(id) {
        Some(e) => e.children.iter().copied().collect(),
        None => Vec::new(),
    }
}

fn current_children(tr: &Transaction, id: NodeId) -> Vec<NodeId> {
    match tr.doc().get_entry(id) {
        Some(e) => e.children.iter().copied().collect(),
        None => Vec::new(),
    }
}

fn reconcile_modifiers(tr: &mut Transaction, target: &Doc, id: NodeId) -> Result<(), StepError> {
    use editor_model::ModifierType;
    use std::collections::BTreeMap;

    let Some(target_entry) = target.get_entry(id) else {
        return Ok(());
    };
    let target_mods: BTreeMap<ModifierType, editor_model::Modifier> = target_entry
        .modifiers
        .iter()
        .map(|(k, m)| (*k, m.clone()))
        .collect();
    let current_mods: BTreeMap<ModifierType, editor_model::Modifier> = {
        let current_doc = tr.doc();
        let Some(current_entry) = current_doc.get_entry(id) else {
            return Ok(());
        };
        current_entry
            .modifiers
            .iter()
            .map(|(k, m)| (*k, m.clone()))
            .collect()
    };

    for (ty, m) in &target_mods {
        if current_mods.get(ty) != Some(m) {
            tr.add_modifier(id, m.clone())?;
        }
    }
    for (ty, m) in &current_mods {
        if !target_mods.contains_key(ty) {
            tr.remove_modifier(id, m.clone())?;
        }
    }
    Ok(())
}

fn node_text(doc: &Doc, id: NodeId) -> Option<String> {
    match &doc.get_entry(id)?.node {
        editor_model::Node::Text(t) => Some(t.text.to_string()),
        _ => None,
    }
}

fn reconcile_text(tr: &mut Transaction, target: &Doc, id: NodeId) -> Result<(), StepError> {
    let cur_doc = tr.doc();
    let (Some(cur), Some(tgt)) = (node_text(&cur_doc, id), node_text(target, id)) else {
        return Ok(());
    };
    if cur == tgt {
        return Ok(());
    }
    let cur_chars: Vec<char> = cur.chars().collect();
    let tgt_chars: Vec<char> = tgt.chars().collect();

    let mut p = 0;
    while p < cur_chars.len() && p < tgt_chars.len() && cur_chars[p] == tgt_chars[p] {
        p += 1;
    }
    let mut s = 0;
    while s < (cur_chars.len() - p)
        && s < (tgt_chars.len() - p)
        && cur_chars[cur_chars.len() - 1 - s] == tgt_chars[tgt_chars.len() - 1 - s]
    {
        s += 1;
    }
    let remove_len = cur_chars.len() - s - p;
    if remove_len > 0 {
        tr.remove_text(id, p, remove_len)?;
    }
    let insert: String = tgt_chars[p..tgt_chars.len() - s].iter().collect();
    if !insert.is_empty() {
        tr.insert_text(id, p, &insert)?;
    }
    Ok(())
}

fn reconcile_attrs(tr: &mut Transaction, target: &Doc, id: NodeId) -> Result<(), StepError> {
    let Some(target_entry) = target.get_entry(id) else {
        return Ok(());
    };
    if matches!(target_entry.node, Node::Text(_)) {
        return Ok(());
    }
    let target_plain = target_entry.node.to_plain();
    let current_plain = match tr.doc().get_entry(id) {
        Some(e) => e.node.to_plain(),
        None => return Ok(()),
    };
    if current_plain != target_plain {
        tr.set_node(id, target_plain)?;
    }
    Ok(())
}

fn to_plain_style(entry: &editor_model::StyleEntry) -> editor_model::PlainStyleEntry {
    editor_model::PlainStyleEntry {
        name: entry.name.get().clone(),
        modifiers: entry.modifiers.iter().cloned().collect(),
    }
}

fn reconcile_styles(tr: &mut Transaction, target: &Doc) -> Result<(), StepError> {
    let target_ids: Vec<String> = target.styles_iter().map(|(id, _)| id.clone()).collect();
    for id in &target_ids {
        let Some(target_entry) = target.style_entry(id) else {
            continue;
        };
        let target_plain = to_plain_style(target_entry);
        // presence를 먼저 확인 — Doc은 style_entry와 presence를 분리 관리.
        let current_plain = if tr.doc().style_present(id) {
            tr.doc().style_entry(id).map(to_plain_style)
        } else {
            None
        };
        if current_plain.as_ref() != Some(&target_plain) {
            tr.set_style(id.clone(), Some(target_plain))?;
        }
    }
    let current_ids: Vec<String> = tr.doc().styles_iter().map(|(id, _)| id.clone()).collect();
    for id in current_ids {
        if !target.style_present(&id) {
            tr.set_style(id, None)?;
        }
    }
    Ok(())
}

fn reconcile_node_style(tr: &mut Transaction, target: &Doc, id: NodeId) -> Result<(), StepError> {
    let Some(target_entry) = target.get_entry(id) else {
        return Ok(());
    };
    let target_style = target_entry.style.get().clone();
    let current_style = match tr.doc().get_entry(id) {
        Some(e) => e.style.get().clone(),
        None => return Ok(()),
    };
    if current_style != target_style {
        tr.set_node_style(id, target_style)?;
    }
    Ok(())
}

fn reconcile_node_marker(tr: &mut Transaction, target: &Doc, id: NodeId) -> Result<(), StepError> {
    let Some(target_entry) = target.get_entry(id) else {
        return Ok(());
    };
    let target_marker = target_entry.marker.get().clone();
    let current_marker = match tr.doc().get_entry(id) {
        Some(e) => e.marker.get().clone(),
        None => return Ok(()),
    };
    if current_marker != target_marker {
        tr.set_marker(id, target_marker)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;
    use editor_model::{CalloutVariant, Node, PlainCalloutNode, PlainNode};

    #[test]
    fn reverts_modifier_change() {
        use editor_model::Modifier;
        let (target_state, t1) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.add_modifier(t1, Modifier::Bold).unwrap();
        let (changed_state, ..) = pre.commit();
        assert!(
            changed_state
                .doc
                .get_entry(t1)
                .unwrap()
                .modifiers
                .iter()
                .count()
                == 1
        );

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(
            reverted.doc.get_entry(t1).unwrap().modifiers.iter().count(),
            0
        );
    }

    #[test]
    fn reverts_modifier_value_change() {
        use editor_model::Modifier;
        let (target_state, t1) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let mut setup = Transaction::new(&target_state);
        setup
            .add_modifier(t1, Modifier::FontSize { value: 1600 })
            .unwrap();
        let (target_state, ..) = setup.commit();

        let mut pre = Transaction::new(&target_state);
        pre.add_modifier(t1, Modifier::FontSize { value: 1200 })
            .unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();

        let mods: Vec<Modifier> = reverted
            .doc
            .get_entry(t1)
            .unwrap()
            .modifiers
            .iter()
            .map(|(_, m)| m.clone())
            .collect();
        assert_eq!(mods, vec![Modifier::FontSize { value: 1600 }]);
    }

    #[test]
    fn reverts_text_change_preserving_common_affixes() {
        let (target_state, t1) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.insert_text(t1, 6, "BRAVE ").unwrap();
        let (changed_state, ..) = pre.commit();
        let cur_text = match &changed_state.doc.get_entry(t1).unwrap().node {
            editor_model::Node::Text(t) => t.text.to_string(),
            _ => unreachable!(),
        };
        assert_eq!(cur_text, "hello BRAVE world");

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        let rev_text = match &reverted.doc.get_entry(t1).unwrap().node {
            editor_model::Node::Text(t) => t.text.to_string(),
            _ => unreachable!(),
        };
        assert_eq!(rev_text, "hello world");
    }

    #[test]
    fn reverts_text_full_replace() {
        let (target_state, t1) = state! {
            doc { root { paragraph { t1: text("abc") } } }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.remove_text(t1, 0, 3).unwrap();
        pre.insert_text(t1, 0, "xyz").unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        let rev = match &reverted.doc.get_entry(t1).unwrap().node {
            editor_model::Node::Text(t) => t.text.to_string(),
            _ => unreachable!(),
        };
        assert_eq!(rev, "abc");
    }

    #[test]
    fn reverts_text_hangul_char_offsets() {
        let (target_state, t1) = state! {
            doc { root { paragraph { t1: text("안녕") } } }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.insert_text(t1, 2, "하세요").unwrap();
        let (changed_state, ..) = pre.commit();
        let cur = match &changed_state.doc.get_entry(t1).unwrap().node {
            editor_model::Node::Text(t) => t.text.to_string(),
            _ => unreachable!(),
        };
        assert_eq!(cur, "안녕하세요");

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        let rev = match &reverted.doc.get_entry(t1).unwrap().node {
            editor_model::Node::Text(t) => t.text.to_string(),
            _ => unreachable!(),
        };
        assert_eq!(rev, "안녕");
    }

    #[test]
    fn reverts_block_deletion() {
        let (target_state, _p1, p2) = state! {
            doc { root { p1: paragraph { text("first") } p2: paragraph { text("second") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.remove_subtree(p2).unwrap();
        let (changed_state, ..) = pre.commit();
        assert_eq!(
            changed_state
                .doc
                .get_entry(NodeId::ROOT)
                .unwrap()
                .children
                .iter()
                .count(),
            1
        );

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(
            editor_model::Doc::from_op_graph(&reverted.graph)
                .unwrap()
                .to_plain(),
            target_state.doc.to_plain()
        );
    }

    #[test]
    fn reverts_block_insertion() {
        let (target_state, _p1) = state! {
            doc { root { p1: paragraph { text("only") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        let newp = NodeId::new();
        pre.insert_subtree(
            NodeId::ROOT,
            1,
            editor_model::Subtree::leaf(
                newp,
                editor_model::PlainNode::Paragraph(editor_model::PlainParagraphNode {}),
            ),
        )
        .unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(
            editor_model::Doc::from_op_graph(&reverted.graph)
                .unwrap()
                .to_plain(),
            target_state.doc.to_plain()
        );
    }

    #[test]
    fn reverts_sibling_reorder() {
        let (target_state, _p1, p2) = state! {
            doc { root { p1: paragraph { text("one") } p2: paragraph { text("two") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.move_node(p2, NodeId::ROOT, 0).unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(
            editor_model::Doc::from_op_graph(&reverted.graph)
                .unwrap()
                .to_plain(),
            target_state.doc.to_plain()
        );
    }

    #[test]
    fn reverts_deletion_then_revival_with_correct_content() {
        let (target_state, _p1, t1, p2) = state! {
            doc { root { p1: paragraph { t1: text("alpha") } p2: paragraph { text("beta") } } }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.insert_text(t1, 5, "X").unwrap();
        pre.remove_subtree(p2).unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        let result = editor_model::Doc::from_op_graph(&reverted.graph).unwrap();
        assert_eq!(result.to_plain(), target_state.doc.to_plain());
        assert_eq!(result.extract_text(), target_state.doc.extract_text());
    }

    #[test]
    fn revert_converges_after_mixed_edits() {
        let (target_state, _p1, t1, p2) = state! {
            doc { root { p1: paragraph { t1: text("alpha") } p2: paragraph { text("beta") } } }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.insert_text(t1, 5, " inserted").unwrap();
        pre.add_modifier(t1, editor_model::Modifier::Bold).unwrap();
        pre.remove_subtree(p2).unwrap();
        let np = NodeId::new();
        pre.insert_subtree(
            NodeId::ROOT,
            1,
            editor_model::Subtree::leaf(
                np,
                editor_model::PlainNode::Paragraph(editor_model::PlainParagraphNode {}),
            ),
        )
        .unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(
            editor_model::Doc::from_op_graph(&reverted.graph)
                .unwrap()
                .to_plain(),
            target_state.doc.to_plain(),
            "revert 후 문서가 대상 시점과 정확히 일치해야 한다"
        );
    }

    #[test]
    fn reverts_cross_parent_move() {
        let (target_state, _bq1, pp, _keep, bq2, _other) = state! {
            doc {
                root {
                    bq1: blockquote { pp: paragraph { text("x") } keep: paragraph { text("k") } }
                    bq2: blockquote { other: paragraph { text("y") } }
                }
            }
            selection: (pp, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.move_node(pp, bq2, 1).unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(
            editor_model::Doc::from_op_graph(&reverted.graph)
                .unwrap()
                .to_plain(),
            target_state.doc.to_plain()
        );
    }

    #[test]
    fn reverts_nested_grandchild_change() {
        let (target_state, _c1, _pp, t1) = state! {
            doc { root { c1: callout { pp: paragraph { t1: text("deep") } } } }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.insert_text(t1, 4, " edit").unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(
            editor_model::Doc::from_op_graph(&reverted.graph)
                .unwrap()
                .to_plain(),
            target_state.doc.to_plain()
        );
    }

    #[test]
    fn reverts_revives_void_image_node_with_attrs() {
        let (target_state, _p1) = state! {
            doc { root { p1: paragraph { text("keep") } } }
            selection: (p1, 0)
        };
        let img = NodeId::new();
        let mut setup = Transaction::new(&target_state);
        setup
            .insert_subtree(
                NodeId::ROOT,
                0,
                editor_model::Subtree::leaf(
                    img,
                    editor_model::PlainNode::Image(editor_model::PlainImageNode {
                        id: Some("img-1".to_string()),
                        proportion: 50,
                    }),
                ),
            )
            .unwrap();
        let (target_state, ..) = setup.commit();

        let mut pre = Transaction::new(&target_state);
        pre.remove_subtree(img).unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(
            editor_model::Doc::from_op_graph(&reverted.graph)
                .unwrap()
                .to_plain(),
            target_state.doc.to_plain()
        );
    }

    #[test]
    fn reverts_revives_deleted_table_subtree() {
        let (target_state, tbl, _after) = state! {
            doc {
                root {
                    tbl: table { table_row { table_cell { paragraph { text("cell") } } } }
                    paragraph { after: text("after") }
                }
            }
            selection: (after, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.remove_subtree(tbl).unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(
            editor_model::Doc::from_op_graph(&reverted.graph)
                .unwrap()
                .to_plain(),
            target_state.doc.to_plain()
        );
    }

    #[test]
    fn reverts_node_attr_change() {
        let (target_state, c1) = state! {
            doc { root { c1: callout { paragraph { text("x") } } } }
            selection: (c1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.set_node(
            c1,
            PlainNode::Callout(PlainCalloutNode {
                variant: CalloutVariant::Warning,
            }),
        )
        .unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, _, _, _, meta) = tr.commit();
        assert!(matches!(meta.history, HistoryMeta::Skip));

        if let Node::Callout(n) = &reverted.doc.get_entry(c1).unwrap().node {
            assert_eq!(*n.variant.get(), CalloutVariant::Info);
        } else {
            panic!("expected callout");
        }
    }

    #[test]
    fn reverts_base_style_modifier_change() {
        use editor_model::{Modifier, PlainStyleEntry};
        let (target_state, _t1) = state! {
            doc {
                styles { base: "기본" [font_size(1600)] }
                root @base [] { paragraph { t1: text("hi") } }
            }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.set_style(
            "base".into(),
            Some(PlainStyleEntry {
                name: "기본".into(),
                modifiers: std::iter::once(Modifier::FontSize { value: 2400 }).collect(),
            }),
        )
        .unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();

        let mods: Vec<Modifier> = reverted
            .doc
            .style_entry("base")
            .unwrap()
            .modifiers
            .iter()
            .cloned()
            .collect();
        assert_eq!(mods, vec![Modifier::FontSize { value: 1600 }]);
    }

    #[test]
    fn reverts_node_style_ref_change() {
        let (target_state, t1) = state! {
            doc {
                styles { s: "s" [bold] }
                root { paragraph { t1: text("hi") } }
            }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.set_node_style(t1, Some("s".into())).unwrap();
        let (changed_state, ..) = pre.commit();

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();

        assert_eq!(
            reverted.doc.node(t1).unwrap().entry().style.get().clone(),
            None
        );
    }

    #[test]
    fn set_marker_and_revert() {
        use editor_model::{Marker, Modifier};
        let (target_state, p1, _t1) = state! {
            doc {
                root { p1: paragraph { t1: text("hi") } }
            }
            selection: (t1, 0)
        };
        let m = Marker {
            modifiers: vec![Modifier::Bold],
            style: None,
        };
        let mut pre = Transaction::new(&target_state);
        pre.set_marker(p1, Some(m.clone())).unwrap();
        let (changed_state, ..) = pre.commit();
        assert_eq!(changed_state.doc.node(p1).unwrap().marker(), Some(&m));

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(reverted.doc.node(p1).unwrap().marker(), None);
    }

    #[test]
    fn reverts_style_creation() {
        use editor_model::{Modifier, PlainStyleEntry};
        let (target_state, _t1) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.set_style(
            "s2".into(),
            Some(PlainStyleEntry {
                name: "s2".into(),
                modifiers: std::iter::once(Modifier::Bold).collect(),
            }),
        )
        .unwrap();
        let (changed_state, ..) = pre.commit();
        assert!(changed_state.doc.style_present("s2"));

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert!(
            !reverted.doc.style_present("s2"),
            "created style must be removed on revert"
        );
    }

    #[test]
    fn reverts_style_deletion() {
        use editor_model::Modifier;
        let (target_state, _t1) = state! {
            doc {
                styles { s: "s" [bold] }
                root { paragraph { t1: text("hi") } }
            }
            selection: (t1, 0)
        };
        let mut pre = Transaction::new(&target_state);
        pre.set_style("s".into(), None).unwrap();
        let (changed_state, ..) = pre.commit();
        assert!(!changed_state.doc.style_present("s"));

        let tr = build_revert_transaction(&changed_state, &target_state.doc).unwrap();
        let (reverted, ..) = tr.commit();
        assert!(
            reverted.doc.style_present("s"),
            "deleted style must be restored on revert"
        );
        let mods: Vec<Modifier> = reverted
            .doc
            .style_entry("s")
            .unwrap()
            .modifiers
            .iter()
            .cloned()
            .collect();
        assert_eq!(mods, vec![Modifier::Bold]);
    }
}
