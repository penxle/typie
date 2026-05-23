use editor_clipboard::Slice;
use editor_model::{
    Fragment, Modifier, Node, NodeId, NodeType, PlainNode, PlainParagraphNode, PlainRootNode,
    PlainTextNode, Schema, Subtree,
};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{insert_hard_break_at_caret, insert_text_at_caret};
use crate::{CommandError, CommandResult};

pub fn insert_slice(tr: &mut Transaction, slice: Slice) -> CommandResult {
    if is_empty_slice(&slice) {
        return Ok(false);
    }

    // Mirror `insert_text` / `insert_hard_break`: callers compose
    // `delete_selection` ahead of this command when they want a non-collapsed
    // selection replaced.
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let slice = coerce_slice_for_caret(tr, slice);
    if is_empty_slice(&slice) {
        return Ok(false);
    }

    let inline_only = is_inline_only(&slice);
    let in_textblock = caret_in_textblock(tr);
    match (inline_only, in_textblock) {
        (true, true) => insert_inline_at_caret(tr, &slice),
        (false, true) => insert_blocks_in_textblock(tr, &slice),
        (true, false) => insert_inline_at_block_boundary(tr, &slice),
        (false, false) => insert_blocks_at_block_boundary(tr, &slice),
    }
}

// Coerce slice's top-level block children to types the caret container allows.
// Disallowed types are unwrapped recursively until either an allowed type or
// an inline leaf is reached.
fn coerce_slice_for_caret(tr: &Transaction, slice: Slice) -> Slice {
    let container_type = match container_type_for_caret(tr) {
        Some(t) => t,
        None => return slice,
    };

    let Slice {
        fragment,
        open_start,
        open_end,
    } = slice;

    match fragment.node {
        PlainNode::Root(_) => {
            let coerced: Vec<Fragment> = fragment
                .children
                .into_iter()
                .flat_map(|c| coerce_fragment_for_parent(c, container_type))
                .collect();
            Slice {
                fragment: Fragment {
                    node: fragment.node,
                    modifiers: fragment.modifiers,
                    children: coerced,
                },
                open_start,
                open_end,
            }
        }
        _ => {
            let coerced = coerce_fragment_for_parent(
                Fragment {
                    node: fragment.node,
                    modifiers: fragment.modifiers,
                    children: fragment.children,
                },
                container_type,
            );
            let wrapped = Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: coerced,
            };
            Slice {
                fragment: wrapped,
                open_start,
                open_end,
            }
        }
    }
}

fn container_type_for_caret(tr: &Transaction) -> Option<NodeType> {
    let state = tr.state();
    let rs = state
        .selection
        .as_ref()
        .and_then(|s| s.resolve(&state.doc))?;
    let head = rs.head();
    let node = state.doc.node(head.node_id())?;
    // Coerce only against the textblock the caret sits inside — at block
    // boundaries (Case C/D candidates) we want the slice's blocks to land
    // as siblings of the existing blocks, not be unwrapped against the
    // boundary container's schema.
    match node.node() {
        Node::Text(_) => node.parent().map(|p| p.as_type()),
        _ => None,
    }
}

fn coerce_fragment_for_parent(f: Fragment, parent_type: NodeType) -> Vec<Fragment> {
    let f_type = f.node.as_type();
    let f_spec = Schema::node_spec(f_type);
    let parent_spec = Schema::node_spec(parent_type);

    // Textblock-in-textblock keeps its boundary so Case B can split the
    // surrounding textblock around it instead of flattening to inline.
    if parent_spec.is_textblock() && f_spec.is_textblock() {
        return vec![f];
    }
    // Inline content reaches the inline insertion path as-is.
    if f_spec.inline {
        return vec![f];
    }
    if child_allowed(parent_type, f_type) {
        return vec![f];
    }
    let mut out = vec![];
    for child in f.children {
        out.extend(coerce_fragment_for_parent(child, parent_type));
    }
    out
}

fn child_allowed(parent_type: NodeType, child_type: NodeType) -> bool {
    let spec = Schema::node_spec(parent_type);
    spec.content.allowed_types().contains(&child_type)
}

fn is_empty_slice(slice: &Slice) -> bool {
    slice.fragment.children.is_empty()
        && !matches!(
            slice.fragment.node,
            PlainNode::Text(_) | PlainNode::HardBreak(_)
        )
}

// An inline-only slice represents pasteable content that fits inside a single
// textblock — either bare inline (Text/HardBreak) or a single textblock wrapper
// (Paragraph) around inline. A Root with multiple block children is a
// block-sequence even if every block happens to be inline-compatible.
fn is_inline_only(slice: &Slice) -> bool {
    fn is_textblock_wrapper(n: &PlainNode) -> bool {
        matches!(n, PlainNode::Paragraph(_))
    }
    fn is_inline_leaf(n: &PlainNode) -> bool {
        matches!(n, PlainNode::Text(_) | PlainNode::HardBreak(_))
    }

    let frag = &slice.fragment;
    match &frag.node {
        n if is_inline_leaf(n) => true,
        n if is_textblock_wrapper(n) => frag.children.iter().all(|c| is_inline_leaf(&c.node)),
        PlainNode::Root(_) => {
            let block_kids: Vec<&Fragment> = frag
                .children
                .iter()
                .filter(|c| !is_inline_leaf(&c.node))
                .collect();
            match block_kids.len() {
                0 => true,
                1 if is_textblock_wrapper(&block_kids[0].node) => block_kids[0]
                    .children
                    .iter()
                    .all(|c| is_inline_leaf(&c.node)),
                _ => false,
            }
        }
        _ => false,
    }
}

fn caret_in_textblock(tr: &Transaction) -> bool {
    let state = tr.state();
    let Some(sel) = state.selection.as_ref().and_then(|s| s.resolve(&state.doc)) else {
        return false;
    };
    sel.head().is_inline_position()
}

fn collect_inline(f: &Fragment) -> Vec<&Fragment> {
    fn walk<'a>(f: &'a Fragment, out: &mut Vec<&'a Fragment>) {
        match &f.node {
            PlainNode::Text(_) | PlainNode::HardBreak(_) => out.push(f),
            _ => {
                for c in &f.children {
                    walk(c, out);
                }
            }
        }
    }
    let mut out = vec![];
    walk(f, &mut out);
    out
}

fn insert_inline_at_caret(tr: &mut Transaction, slice: &Slice) -> CommandResult {
    let fragments: Vec<Fragment> = collect_inline(&slice.fragment)
        .into_iter()
        .cloned()
        .collect();
    insert_inline_fragments(tr, fragments)
}

fn insert_inline_fragments(tr: &mut Transaction, fragments: Vec<Fragment>) -> CommandResult {
    let mut any_change = false;
    for f in fragments {
        match f.node {
            PlainNode::Text(t) if !t.text.is_empty() => {
                if f.modifiers.is_empty() {
                    insert_text_at_caret(tr, &t.text)?;
                } else {
                    insert_modifier_text(tr, &t.text, f.modifiers)?;
                }
                any_change = true;
            }
            PlainNode::HardBreak(_) => {
                insert_hard_break_at_caret(tr)?;
                any_change = true;
            }
            _ => {}
        }
    }
    Ok(any_change)
}

fn insert_modifier_text(
    tr: &mut Transaction,
    text: &str,
    modifiers: Vec<Modifier>,
) -> CommandResult {
    let pos = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;
    let (parent_id, child_index) = textblock_insert_point(tr, pos)?;
    let id = NodeId::new();
    let subtree =
        Subtree::leaf(id, PlainNode::Text(PlainTextNode::default())).with_modifiers(modifiers);
    tr.insert_subtree(parent_id, child_index, subtree)?;
    tr.insert_text(id, 0, text)?;
    let len = text.chars().count();
    tr.set_selection(Some(Selection::collapsed(Position {
        node_id: id,
        offset: len,
        affinity: Affinity::Upstream,
    })))?;
    Ok(true)
}

fn textblock_insert_point(
    tr: &mut Transaction,
    pos: Position,
) -> Result<(NodeId, usize), CommandError> {
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;
    match node.node() {
        Node::Text(text_node) => {
            let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
            let parent_id = parent.id();
            let text_index = node
                .index()
                .ok_or(CommandError::orphan_child(pos.node_id, parent_id))?;
            let text_len = text_node.text.len();
            let index = if pos.offset == 0 {
                text_index
            } else if pos.offset == text_len {
                text_index + 1
            } else {
                drop(doc);
                let split_id = NodeId::new();
                tr.split_node(pos.node_id, pos.offset, split_id)?;
                text_index + 1
            };
            Ok((parent_id, index))
        }
        _ => Ok((pos.node_id, pos.offset)),
    }
}

fn insert_blocks_in_textblock(tr: &mut Transaction, slice: &Slice) -> CommandResult {
    let head = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;

    // Resolve textblock id + split index, splitting any straddling text node first.
    let (textblock_id, split_index_in_textblock) = {
        let doc = tr.doc();
        let head_node = doc
            .node(head.node_id)
            .ok_or(CommandError::NodeNotFound(head.node_id))?;
        match head_node.node() {
            Node::Text(text_node) => {
                let parent = head_node
                    .parent()
                    .ok_or(CommandError::NoParent(head.node_id))?;
                let textblock_id = parent.id();
                let text_index = head_node
                    .index()
                    .ok_or(CommandError::orphan_child(head.node_id, textblock_id))?;
                let text_len = text_node.text.len();
                let index = if head.offset == 0 {
                    text_index
                } else if head.offset == text_len {
                    text_index + 1
                } else {
                    drop(doc);
                    let split_text_id = NodeId::new();
                    tr.split_node(head.node_id, head.offset, split_text_id)?;
                    text_index + 1
                };
                (textblock_id, index)
            }
            _ => (head.node_id, head.offset),
        }
    };

    let (container_id, textblock_index) = {
        let doc = tr.doc();
        let tb = doc
            .node(textblock_id)
            .ok_or(CommandError::NodeNotFound(textblock_id))?;
        let parent = tb.parent().ok_or(CommandError::NoParent(textblock_id))?;
        let textblock_index = tb
            .index()
            .ok_or(CommandError::orphan_child(textblock_id, parent.id()))?;
        (parent.id(), textblock_index)
    };

    // Split the textblock at the resolved child index. p2_id becomes the right half.
    let p2_id = NodeId::new();
    tr.split_node(textblock_id, split_index_in_textblock, p2_id)?;

    let blocks: Vec<&Fragment> = match &slice.fragment.node {
        PlainNode::Root(_) => slice.fragment.children.iter().collect(),
        _ => vec![&slice.fragment],
    };

    let merge_start = slice.open_start > 0
        && blocks
            .first()
            .is_some_and(|b| same_textblock_type(&b.node, textblock_id, tr));
    let merge_end = slice.open_end > 0
        && blocks
            .last()
            .is_some_and(|b| same_textblock_type(&b.node, p2_id, tr));

    let middle_start = if merge_start { 1 } else { 0 };
    let middle_end = if merge_end {
        blocks.len().saturating_sub(1)
    } else {
        blocks.len()
    };
    // When the same block participates as both first and last (single-block slice with
    // both ends open and matching textblocks), only merge into the start to avoid
    // double-applying its inline content.
    let merge_end = merge_end && middle_end >= middle_start;

    let mut last_caret: Option<Position> = None;

    if merge_start {
        let first = blocks[0];
        let inline = first.children.to_vec();
        position_caret_at_textblock_end(tr, textblock_id)?;
        insert_inline_fragments(tr, inline)?;
        last_caret = Some(
            tr.selection()
                .expect("selection preserved through mutations")
                .head,
        );
    }

    for (insert_at, block) in
        (textblock_index + 1..).zip(blocks.iter().take(middle_end).skip(middle_start))
    {
        let subtree = (*block).clone().into_subtree();
        let inserted_id = subtree.id;
        tr.insert_subtree(container_id, insert_at, subtree)?;
        last_caret = Some(position_at_end_of_block(tr, inserted_id));
    }

    if merge_end {
        let last = blocks.last().unwrap();
        let inline = last.children.to_vec();
        position_caret_at_textblock_start(tr, p2_id)?;
        insert_inline_fragments(tr, inline)?;
        // After inserting at the start of p2, the caret naturally lands between
        // the merged-in inline and p2's original inline content.
        last_caret = Some(
            tr.selection()
                .expect("selection preserved through mutations")
                .head,
        );
    }

    let textblock_empty_after = tr
        .doc()
        .node(textblock_id)
        .map(|n| n.children().count() == 0)
        .unwrap_or(false);
    if textblock_empty_after {
        tr.remove_subtree(textblock_id)?;
    }
    let p2_empty_after = tr
        .doc()
        .node(p2_id)
        .map(|n| n.children().count() == 0)
        .unwrap_or(false);
    if p2_empty_after {
        tr.remove_subtree(p2_id)?;
    }

    let final_pos = match last_caret {
        Some(p) => p,
        None => Position {
            node_id: p2_id,
            offset: 0,
            affinity: Affinity::Upstream,
        },
    };
    tr.set_selection(Some(Selection::collapsed(final_pos)))?;

    Ok(true)
}

fn position_caret_at_textblock_end(
    tr: &mut Transaction,
    textblock_id: NodeId,
) -> Result<(), CommandError> {
    let doc = tr.doc();
    let tb = doc
        .node(textblock_id)
        .ok_or(CommandError::NodeNotFound(textblock_id))?;
    let pos = match tb.last_child() {
        Some(c) => match c.node() {
            Node::Text(t) => Position {
                node_id: c.id(),
                offset: t.text.len(),
                affinity: Affinity::Upstream,
            },
            _ => {
                let child_count = tb.children().count();
                Position {
                    node_id: textblock_id,
                    offset: child_count,
                    affinity: Affinity::Upstream,
                }
            }
        },
        None => Position {
            node_id: textblock_id,
            offset: 0,
            affinity: Affinity::Upstream,
        },
    };
    drop(doc);
    tr.set_selection(Some(Selection::collapsed(pos)))?;
    Ok(())
}

fn position_caret_at_textblock_start(
    tr: &mut Transaction,
    textblock_id: NodeId,
) -> Result<(), CommandError> {
    let doc = tr.doc();
    let tb = doc
        .node(textblock_id)
        .ok_or(CommandError::NodeNotFound(textblock_id))?;
    let pos = match tb.first_child() {
        Some(c) => match c.node() {
            Node::Text(_) => Position {
                node_id: c.id(),
                offset: 0,
                affinity: Affinity::Downstream,
            },
            _ => Position {
                node_id: textblock_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
        },
        None => Position {
            node_id: textblock_id,
            offset: 0,
            affinity: Affinity::Downstream,
        },
    };
    drop(doc);
    tr.set_selection(Some(Selection::collapsed(pos)))?;
    Ok(())
}

fn position_at_end_of_block(tr: &Transaction, block_id: NodeId) -> Position {
    let doc = tr.doc();
    let block = doc.node(block_id).expect("inserted block exists");
    match block.last_child() {
        Some(c) => match c.node() {
            Node::Text(t) => Position {
                node_id: c.id(),
                offset: t.text.len(),
                affinity: Affinity::Upstream,
            },
            _ => {
                let child_count = block.children().count();
                Position {
                    node_id: block_id,
                    offset: child_count,
                    affinity: Affinity::Upstream,
                }
            }
        },
        None => Position {
            node_id: block_id,
            offset: 0,
            affinity: Affinity::Upstream,
        },
    }
}

fn insert_inline_at_block_boundary(tr: &mut Transaction, slice: &Slice) -> CommandResult {
    let head = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;
    let container_id = head.node_id;
    let index = head.offset;

    let inline_clones: Vec<Fragment> = collect_inline(&slice.fragment)
        .into_iter()
        .cloned()
        .collect();
    if inline_clones.is_empty() {
        return Ok(false);
    }

    let new_para_id = NodeId::new();
    let para_subtree = Subtree::leaf(
        new_para_id,
        PlainNode::Paragraph(PlainParagraphNode::default()),
    );
    tr.insert_subtree(container_id, index, para_subtree)?;

    position_caret_at_textblock_start(tr, new_para_id)?;
    insert_inline_fragments(tr, inline_clones)?;
    Ok(true)
}

fn insert_blocks_at_block_boundary(tr: &mut Transaction, slice: &Slice) -> CommandResult {
    let head = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;
    let container_id = head.node_id;
    let base_index = head.offset;

    let blocks: Vec<&Fragment> = match &slice.fragment.node {
        PlainNode::Root(_) => slice.fragment.children.iter().collect(),
        _ => vec![&slice.fragment],
    };
    if blocks.is_empty() {
        return Ok(false);
    }

    let mut last_inserted: Option<NodeId> = None;
    for (offset, block) in blocks.iter().enumerate() {
        let subtree = (*block).clone().into_subtree();
        let inserted_id = subtree.id;
        tr.insert_subtree(container_id, base_index + offset, subtree)?;
        last_inserted = Some(inserted_id);
    }

    if let Some(id) = last_inserted {
        let final_pos = position_at_end_of_block(tr, id);
        tr.set_selection(Some(Selection::collapsed(final_pos)))?;
    }

    Ok(true)
}

fn same_textblock_type(slice_node: &PlainNode, doc_node_id: NodeId, tr: &Transaction) -> bool {
    let doc = tr.doc();
    let Some(doc_node) = doc.node(doc_node_id) else {
        return false;
    };
    matches!(
        (slice_node, doc_node.node()),
        (PlainNode::Paragraph(_), Node::Paragraph(_))
    )
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    fn root_with_paragraph(text: &str) -> Slice {
        Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![Fragment {
                    node: PlainNode::Paragraph(PlainParagraphNode::default()),
                    modifiers: vec![],
                    children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: text.into(),
                    }))],
                }],
            },
            open_start: 2,
            open_end: 2,
        }
    }

    fn paragraph_fragment(text: &str) -> Fragment {
        Fragment {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: vec![],
            children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: text.into(),
            }))],
        }
    }

    #[test]
    fn insert_empty_slice_no_op() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let empty = Slice {
            fragment: Fragment::leaf(PlainNode::Root(PlainRootNode::default())),
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact_fail!(initial.clone(), |tr| insert_slice(&mut tr, empty));
        assert_state_eq!(&actual, &initial);
    }

    #[test]
    fn empty_slice_helper_recognises_bare_container() {
        let empty = Slice {
            fragment: Fragment::leaf(PlainNode::Root(PlainRootNode::default())),
            open_start: 0,
            open_end: 0,
        };
        assert!(is_empty_slice(&empty));
    }

    #[test]
    fn is_inline_only_classifies_single_paragraph_slice() {
        let slice = root_with_paragraph("XY");
        assert!(is_inline_only(&slice));
    }

    #[test]
    fn is_inline_only_classifies_multi_paragraph_slice() {
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![paragraph_fragment("a"), paragraph_fragment("b")],
            },
            open_start: 2,
            open_end: 2,
        };
        assert!(!is_inline_only(&slice));
    }

    #[test]
    fn insert_inline_only_into_paragraph_middle() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let slice = root_with_paragraph("XY");
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("HeXYllo") } } }
            selection: (t1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_inline_at_block_boundary_wraps_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                paragraph { text("b") }
            } }
            selection: (r, 1, >)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: "X".into(),
                }))],
            },
            open_start: 1,
            open_end: 1,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                paragraph { t2: text("X") }
                paragraph { text("b") }
            } }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_disallowed_block_unwraps_children() {
        use editor_model::{PlainBulletListNode, PlainListItemNode};
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![Fragment {
                    node: PlainNode::BulletList(PlainBulletListNode::default()),
                    modifiers: vec![],
                    children: vec![Fragment {
                        node: PlainNode::ListItem(PlainListItemNode::default()),
                        modifiers: vec![],
                        children: vec![paragraph_fragment("X")],
                    }],
                }],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root { paragraph { t: text("HelloX") } } }
            selection: (t, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let slice = Slice::from_text("X");
        transact_fail!(initial, |tr| insert_slice(&mut tr, slice));
    }

    #[test]
    fn insert_blocks_at_block_boundary() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                paragraph { text("b") }
            } }
            selection: (r, 1, >)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![paragraph_fragment("X"), paragraph_fragment("Y")],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                paragraph { text("X") }
                paragraph { t3: text("Y") }
                paragraph { text("b") }
            } }
            selection: (t3, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_blocks_into_empty_paragraph_replaces_without_extra_empties() {
        use editor_model::PlainCalloutNode;
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![
                    Fragment {
                        node: PlainNode::Callout(PlainCalloutNode::default()),
                        modifiers: vec![],
                        children: vec![paragraph_fragment("1")],
                    },
                    Fragment {
                        node: PlainNode::Paragraph(PlainParagraphNode::default()),
                        modifiers: vec![],
                        children: vec![],
                    },
                ],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root {
                callout { paragraph { text("1") } }
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_blocks_into_paragraph_middle_splits_and_merges() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 5)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![paragraph_fragment("first"), paragraph_fragment("second")],
            },
            open_start: 2,
            open_end: 2,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Hellofirst") }
                paragraph { t2: text("second World") }
            } }
            selection: (t2, 6)
        };
        assert_state_eq!(&actual, &expected);
    }
}
