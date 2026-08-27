use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{ChildView, Modifier, ModifierType, PlainNode, Subtree};
use strum::IntoEnumIterator;

use super::Diff;
use crate::error::XmlError;
use crate::lcs::{Edit, MAX_EDIT_DISTANCE, diff_bounded};
use crate::tree::{InlineLeaf, XmlNode};

type Own = BTreeMap<ModifierType, Modifier>;
type InlineRow = (InlineLeaf, Own);

struct BaseLeaf {
    dot: Dot,
    leaf: InlineLeaf,
    own: Own,
}

pub(crate) fn reconcile_textblock(
    diff: &mut Diff<'_>,
    block: Dot,
    target: &XmlNode,
) -> Result<(), XmlError> {
    let base = base_rows(diff, block)?;
    let wanted: Vec<InlineRow> = target
        .inline_items()
        .map(|item| (item.leaf.clone(), item.own.clone()))
        .collect();
    if base
        .iter()
        .map(|row| (&row.leaf, &row.own))
        .eq(wanted.iter().map(|(leaf, own)| (leaf, own)))
    {
        return Ok(());
    }

    let base_keys: Vec<&InlineLeaf> = base.iter().map(|row| &row.leaf).collect();
    let target_keys: Vec<&InlineLeaf> = wanted.iter().map(|(leaf, _)| leaf).collect();
    match diff_bounded(&base_keys, &target_keys, MAX_EDIT_DISTANCE) {
        Some(script) => apply_script(diff, block, &base, &wanted, &script)?,
        None => replace_all(diff, block, &base, &wanted)?,
    }

    settle_own(diff, block, &wanted)
}

fn apply_script(
    diff: &mut Diff<'_>,
    block: Dot,
    base: &[BaseLeaf],
    wanted: &[InlineRow],
    script: &[Edit],
) -> Result<(), XmlError> {
    let mut removals: Vec<(usize, usize)> = Vec::new();
    for edit in script {
        if let Edit::Delete { a } = edit {
            match removals.last_mut() {
                Some((_, end)) if *end == *a => *end = a + 1,
                _ => removals.push((*a, a + 1)),
            }
        }
    }
    for (from, to) in removals.iter().rev() {
        remove_slots(diff, block, *from, *to, &base[*from..*to])?;
    }

    let mut cursor = 0usize;
    let mut pending = String::new();
    for edit in script {
        match edit {
            Edit::Keep { .. } => {
                flush_text(diff, block, &mut cursor, &mut pending)?;
                cursor += 1;
            }
            Edit::Delete { .. } => {}
            Edit::Insert { b } => {
                insert_leaf(diff, block, &wanted[*b].0, &mut cursor, &mut pending)?;
            }
        }
    }
    flush_text(diff, block, &mut cursor, &mut pending)
}

fn replace_all(
    diff: &mut Diff<'_>,
    block: Dot,
    base: &[BaseLeaf],
    wanted: &[InlineRow],
) -> Result<(), XmlError> {
    if !base.is_empty() {
        remove_slots(diff, block, 0, base.len(), base)?;
    }
    let mut cursor = 0usize;
    let mut pending = String::new();
    for (leaf, _) in wanted {
        insert_leaf(diff, block, leaf, &mut cursor, &mut pending)?;
    }
    flush_text(diff, block, &mut cursor, &mut pending)
}

fn remove_slots(
    diff: &mut Diff<'_>,
    block: Dot,
    from: usize,
    to: usize,
    removed: &[BaseLeaf],
) -> Result<(), XmlError> {
    diff.tr
        .remove_child_slots(block, from, to)
        .map_err(|e| XmlError::internal(format!("remove inline slots: {e}")))?;
    diff.counts.chars_deleted += removed
        .iter()
        .filter(|row| matches!(row.leaf, InlineLeaf::Char(_)))
        .count() as u32;
    Ok(())
}

fn insert_leaf(
    diff: &mut Diff<'_>,
    block: Dot,
    leaf: &InlineLeaf,
    cursor: &mut usize,
    pending: &mut String,
) -> Result<(), XmlError> {
    match leaf {
        InlineLeaf::Char(ch) => {
            pending.push(*ch);
            Ok(())
        }
        InlineLeaf::Atom(node) => {
            flush_text(diff, block, cursor, pending)?;
            let subtree = Subtree {
                node: node.clone(),
                modifiers: Vec::new(),
                carry: Vec::new(),
                children: Vec::new(),
                source_dots: Vec::new(),
            };
            diff.tr
                .insert_subtree(block, *cursor, subtree)
                .map_err(|e| XmlError::internal(format!("insert inline atom: {e}")))?;
            *cursor += 1;
            Ok(())
        }
    }
}

fn flush_text(
    diff: &mut Diff<'_>,
    block: Dot,
    cursor: &mut usize,
    pending: &mut String,
) -> Result<(), XmlError> {
    if pending.is_empty() {
        return Ok(());
    }
    diff.tr
        .insert_text(block, *cursor, pending)
        .map_err(|e| XmlError::internal(format!("insert inline text: {e}")))?;
    let count = pending.chars().count();
    *cursor += count;
    diff.counts.chars_inserted += count as u32;
    pending.clear();
    Ok(())
}

fn settle_own(diff: &mut Diff<'_>, block: Dot, wanted: &[InlineRow]) -> Result<(), XmlError> {
    let now = base_rows(diff, block)?;
    if now.len() != wanted.len() {
        return Err(XmlError::internal(format!(
            "inline length mismatch: {} vs {}",
            now.len(),
            wanted.len()
        )));
    }
    for ty in ModifierType::iter() {
        let mut i = 0;
        while i < now.len() {
            let cur = now[i].own.get(&ty).cloned();
            let want = wanted[i].1.get(&ty).cloned();
            if cur == want {
                i += 1;
                continue;
            }
            let mut j = i;
            while j + 1 < now.len()
                && now[j + 1].own.get(&ty).cloned() == cur
                && wanted[j + 1].1.get(&ty).cloned() == want
            {
                j += 1;
            }
            let (first, last) = (now[i].dot, now[j].dot);
            if let Some(current) = cur {
                diff.tr
                    .remove_span_modifier(first, last, current)
                    .map_err(|e| XmlError::internal(format!("remove span modifier: {e}")))?;
            }
            if let Some(target) = want {
                diff.tr
                    .add_span_modifier(first, last, target)
                    .map_err(|e| XmlError::internal(format!("add span modifier: {e}")))?;
            }
            i = j + 1;
        }
    }
    Ok(())
}

fn base_rows(diff: &Diff<'_>, block: Dot) -> Result<Vec<BaseLeaf>, XmlError> {
    let view = diff.tr.view();
    let nv = view
        .node(block)
        .ok_or_else(|| XmlError::internal(format!("block not found: {block}")))?;
    let mut rows = Vec::new();
    for (slot, child) in nv.children().enumerate() {
        let ChildView::Leaf(leaf) = child else {
            return Err(XmlError::internal("block inside textblock"));
        };
        let own: Own = nv
            .leaf_state_at(slot)
            .map(|s| s.own.iter().map(|(t, o)| (*t, o.value.clone())).collect())
            .unwrap_or_default();
        let entry = match leaf.as_char() {
            Some(ch) => InlineLeaf::Char(ch),
            None => InlineLeaf::Atom(
                leaf.node()
                    .map(|node| node.to_plain())
                    .unwrap_or(PlainNode::Unknown),
            ),
        };
        rows.push(BaseLeaf {
            dot: leaf.dot(),
            leaf: entry,
            own,
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::PlainTextNode;
    use editor_transaction::Transaction;

    use super::super::ChangeCounts;
    use super::*;
    use crate::reader::from_xml;
    use crate::test_support::live_heads;
    use crate::writer::to_xml;

    fn apply(state: &editor_state::State, xml: &str) -> (editor_state::State, ChangeCounts) {
        let tree = from_xml(xml).expect("target parses");
        let mut tr = Transaction::new(state);
        let root = state.view().root().expect("root").id();
        let counts = {
            let mut diff = Diff::new(&mut tr, &tree);
            diff.reconcile_node(root, &tree.root, &[])
                .expect("reconcile");
            diff.finish().expect("finish").counts
        };
        let (next, ..) = tr.commit();
        (next, counts)
    }

    fn leaf_dots(state: &editor_state::State, block: Dot) -> Vec<Dot> {
        let view = state.view();
        view.node(block)
            .expect("block")
            .children()
            .filter_map(|c| match c {
                ChildView::Leaf(l) => Some(l.dot()),
                ChildView::Block(_) => None,
            })
            .collect()
    }

    fn paragraph_of(text: &str) -> (editor_state::State, Dot) {
        let (state, p) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.insert_text(p, 0, text).expect("insert");
        let (next, ..) = tr.commit();
        (next, p)
    }

    fn paragraph_with_unknown_leaf() -> (editor_state::State, Dot) {
        use editor_crdt::ListOp;
        use editor_model::{EditOp, SeqItem};

        let (mut state, p) = state! {
            doc { root { p: paragraph { text("ab") } } }
            selection: (p, 0)
        };
        state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::Unknown {
                    tag: 999,
                    bytes: vec![0xAA],
                },
            }))
            .unwrap();
        (state, p)
    }

    #[test]
    fn an_unknown_inline_leaf_the_target_repeats_keeps_its_dot() {
        let (state, p) = paragraph_with_unknown_leaf();
        let before = leaf_dots(&state, p);
        assert_eq!(before.len(), 3);

        let xml = to_xml(&state, &live_heads(&state))
            .unwrap()
            .replace("a<unknown/>b", "aZ<unknown/>b");
        let (next, counts) = apply(&state, &xml);

        let after = leaf_dots(&next, p);
        assert_eq!(after.len(), 4);
        assert_eq!(after[0], before[0]);
        assert_eq!(after[2], before[1]);
        assert_eq!(after[3], before[2]);
        assert_eq!((counts.chars_inserted, counts.chars_deleted), (1, 0));
    }

    #[test]
    fn an_unknown_inline_leaf_the_target_drops_is_removed() {
        let (state, p) = paragraph_with_unknown_leaf();
        let before = leaf_dots(&state, p);

        let xml = to_xml(&state, &live_heads(&state))
            .unwrap()
            .replace("a<unknown/>b", "aZb");
        let (next, counts) = apply(&state, &xml);

        let after = leaf_dots(&next, p);
        assert_eq!(after.len(), 3);
        assert_eq!(after[0], before[0]);
        assert_eq!(after[2], before[2]);
        assert!(!after.contains(&before[1]));
        assert_eq!((counts.chars_inserted, counts.chars_deleted), (1, 0));
        assert_eq!(
            next.to_plain().root.children[0].children[0].node,
            PlainNode::Text(PlainTextNode { text: "aZb".into() })
        );
    }

    #[test]
    fn typo_fix_keeps_untouched_leaf_dots() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("안녕하세요 세상") } } }
            selection: (p, 0)
        };
        let before = leaf_dots(&state, p);
        assert_eq!(before.len(), 8);

        let xml = to_xml(&state, &live_heads(&state))
            .unwrap()
            .replace("안녕하세요 세상", "안녕하세요, 세상아");
        let (next, counts) = apply(&state, &xml);

        assert_eq!(
            next.to_plain().root.children[0].children[0].node,
            PlainNode::Text(PlainTextNode {
                text: "안녕하세요, 세상아".into()
            })
        );

        let after = leaf_dots(&next, p);
        assert_eq!(after.len(), 10);
        assert_eq!(&after[..5], &before[..5]);
        assert_eq!(&after[6..9], &before[5..8]);
        assert!(!before.contains(&after[5]));
        assert!(!before.contains(&after[9]));
        assert_eq!((counts.chars_inserted, counts.chars_deleted), (2, 0));
    }

    #[test]
    fn formatting_only_change_keeps_every_leaf_dot() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("가나다") } } }
            selection: (p, 0)
        };
        let before = leaf_dots(&state, p);
        let xml = to_xml(&state, &live_heads(&state))
            .unwrap()
            .replace("가나다", "가<bold>나</bold>다");
        let (next, counts) = apply(&state, &xml);

        assert_eq!(leaf_dots(&next, p), before);
        let (expected, _) = state! {
            doc { root { p2: paragraph { text("가") text("나") [bold] text("다") } } }
            selection: (p2, 0)
        };
        assert_eq!(next.to_plain(), expected.to_plain());
        assert_eq!((counts.chars_inserted, counts.chars_deleted), (0, 0));
    }

    #[test]
    fn inserted_text_inside_bold_span_gets_target_formatting() {
        let (state, _p) = state! {
            doc { root { p: paragraph { text("ab") [bold] } } }
            selection: (p, 0)
        };
        let xml = to_xml(&state, &live_heads(&state)).unwrap().replace(
            "<bold>ab</bold>",
            "<bold>a</bold>X<bold>b</bold><hard_break/>",
        );
        let (next, counts) = apply(&state, &xml);

        let (expected, _) = state! {
            doc { root { p2: paragraph { text("a") [bold] text("X") text("b") [bold] hard_break } } }
            selection: (p2, 0)
        };
        assert_eq!(next.to_plain(), expected.to_plain());
        assert_eq!((counts.chars_inserted, counts.chars_deleted), (1, 0));
    }

    #[test]
    fn deletions_run_back_to_front_and_keep_surviving_dots() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("abcdefgh") } } }
            selection: (p, 0)
        };
        let before = leaf_dots(&state, p);
        assert_eq!(before.len(), 8);

        let xml = to_xml(&state, &live_heads(&state))
            .unwrap()
            .replace("abcdefgh", "acdXfgh");
        let (next, counts) = apply(&state, &xml);

        assert_eq!(
            next.to_plain().root.children[0].children[0].node,
            PlainNode::Text(PlainTextNode {
                text: "acdXfgh".into()
            })
        );
        assert_eq!((counts.chars_inserted, counts.chars_deleted), (1, 2));

        let after = leaf_dots(&next, p);
        assert_eq!(after.len(), 7);
        assert_eq!(after[0], before[0]);
        assert_eq!(&after[1..3], &before[2..4]);
        assert_eq!(&after[4..7], &before[5..8]);
        assert!(!before.contains(&after[3]));
    }

    #[test]
    fn deleting_an_inline_atom_costs_no_deleted_chars() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("ab") hard_break text("cd") } } }
            selection: (p, 0)
        };
        let before = leaf_dots(&state, p);
        assert_eq!(before.len(), 5);

        let xml = to_xml(&state, &live_heads(&state))
            .unwrap()
            .replace("<hard_break/>", "");
        let (next, counts) = apply(&state, &xml);

        let (expected, _) = state! {
            doc { root { p2: paragraph { text("abcd") } } }
            selection: (p2, 0)
        };
        assert_eq!(next.to_plain(), expected.to_plain());
        assert_eq!((counts.chars_inserted, counts.chars_deleted), (0, 0));

        let after = leaf_dots(&next, p);
        assert_eq!(after, [before[0], before[1], before[3], before[4]]);
    }

    #[test]
    fn one_removal_range_may_mix_chars_and_atoms() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("ab") hard_break text("cd") } } }
            selection: (p, 0)
        };
        let before = leaf_dots(&state, p);
        assert_eq!(before.len(), 5);

        let xml = to_xml(&state, &live_heads(&state))
            .unwrap()
            .replace("ab<hard_break/>cd", "ad");
        let (next, counts) = apply(&state, &xml);

        assert_eq!(
            next.to_plain().root.children[0].children[0].node,
            PlainNode::Text(PlainTextNode { text: "ad".into() })
        );
        assert_eq!((counts.chars_inserted, counts.chars_deleted), (0, 2));
        assert_eq!(leaf_dots(&next, p), [before[0], before[4]]);
    }

    #[test]
    fn a_rewrite_past_the_edit_bound_replaces_the_whole_line() {
        let base_text: String = std::iter::once('Z')
            .chain((0..3000u32).map(|i| char::from_u32(0xac00 + i).unwrap()))
            .collect();
        let target_text: String = std::iter::once('Z')
            .chain((0..3000u32).map(|i| char::from_u32(0xac00 + 3000 + i).unwrap()))
            .collect();

        let (state, p) = paragraph_of(&base_text);
        let before = leaf_dots(&state, p);
        assert_eq!(before.len(), 3001);

        let xml = to_xml(&state, &live_heads(&state))
            .unwrap()
            .replace(&base_text, &target_text);
        let (next, counts) = apply(&state, &xml);

        let (expected, _) = paragraph_of(&target_text);
        assert_eq!(next.to_plain(), expected.to_plain());
        assert_eq!((counts.chars_inserted, counts.chars_deleted), (3001, 3001));

        let after = leaf_dots(&next, p);
        assert_eq!(after.len(), 3001);
        assert_ne!(
            after[0], before[0],
            "whole-line replacement rewrites even the shared leading char"
        );
        assert!(after.iter().all(|d| !before.contains(d)));
    }
}
