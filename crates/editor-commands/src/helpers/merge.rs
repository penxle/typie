use editor_crdt::Dot;
use editor_model::{ChildView, NodeView};
use editor_transaction::Transaction;

use crate::CommandError;
use crate::helpers::{capture_charlike_slots, insert_charlike_slots};

pub(crate) struct ContainerMerge {
    pub appended_at: usize,
}

/// Selection-free authority for moving every real block child from `source`
/// to the end of `target`, then removing the emptied source container.
pub(crate) fn merge_block_container_into(
    tr: &mut Transaction,
    target: Dot,
    source: Dot,
) -> Result<ContainerMerge, CommandError> {
    let mut merged = None;
    tr.batch::<_, CommandError>(|tr| {
        materialize_content_owning_direct_scaffolds(tr, target)?;
        materialize_content_owning_direct_scaffolds(tr, source)?;
        let (appended_at, items) = {
            let view = tr.view();
            let target_node = view
                .node(target)
                .ok_or(CommandError::NodeNotFound(target))?;
            let source_node = view
                .node(source)
                .ok_or(CommandError::NodeNotFound(source))?;
            if source_node.children().any(
                |child| matches!(child, ChildView::Leaf(leaf) if leaf.dot().as_op_dot().is_some()),
            ) {
                return Err(CommandError::Corrupted(
                    "block-container merge received a real direct leaf".into(),
                ));
            }
            let appended_at = target_node
                .child_blocks()
                .filter(|child| child.id().as_op_dot().is_some())
                .count();
            let items = source_node
                .child_blocks()
                .filter(|child| child.id().as_op_dot().is_some())
                .map(|child| child.id())
                .collect::<Vec<_>>();
            (appended_at, items)
        };
        if !items.is_empty() {
            tr.move_nodes_consecutive(&items, target, appended_at)?;
        }
        let authored_residue = tr
            .view()
            .node(source)
            .is_some_and(|source| has_authored_descendant(&source));
        if authored_residue {
            return Err(CommandError::Corrupted(
                "block-container merge would discard authored descendants".into(),
            ));
        }
        crate::helpers::remove_subtree_full(tr, source)?;
        merged = Some(ContainerMerge { appended_at });
        Ok(())
    })?;
    merged.ok_or_else(|| CommandError::Corrupted("block-container merge produced no result".into()))
}

fn materialize_content_owning_direct_scaffolds(
    tr: &mut Transaction,
    container: Dot,
) -> Result<(), CommandError> {
    loop {
        let scaffold = {
            let view = tr.view();
            let container = view
                .node(container)
                .ok_or(CommandError::NodeNotFound(container))?;
            container
                .child_blocks()
                .find(|child| child.id().is_synthetic() && has_authored_descendant(child))
                .map(|child| child.id())
        };
        let Some(scaffold) = scaffold else {
            return Ok(());
        };
        editor_transaction::materialize_repair_target(tr, scaffold)?;
    }
}

fn has_authored_descendant(node: &NodeView<'_>) -> bool {
    node.descendants().any(|child| match child {
        ChildView::Block(block) => block.id().as_op_dot().is_some(),
        ChildView::Leaf(leaf) => leaf.dot().as_op_dot().is_some(),
    })
}

fn next_block_sibling_id(parent: &NodeView, target_id: Dot) -> Option<Dot> {
    let idx = parent.child_blocks().position(|b| b.id() == target_id)?;
    parent.child_blocks().nth(idx + 1).map(|b| b.id())
}

/// Merge `source`'s inline content into `target` (appended at the end) and
/// remove `source`.
///
/// When `target`'s container accepts `source` as an adjacent sibling (e.g. a
/// `Root`/`Blockquote`/`Callout` that allows several paragraphs), `source` is
/// moved to sit right after `target` and folded in with `merge_node`, which
/// keeps the inline leaves (and their span formatting) intact.
///
/// Containers that reject `source` as an adjacent sibling cause projection
/// normalization to drop the move again. In that case `source`'s inline content
/// (chars and formatting-bearing atoms) is appended to `target` and the
/// `source` subtree is removed.
pub(crate) fn merge_element_cross_parent(
    tr: &mut Transaction,
    source_id: Dot,
    target_id: Dot,
) -> Result<(), CommandError> {
    let (target_parent, target_index, orig_next) = {
        let view = tr.state().view();
        let target = view
            .node(target_id)
            .ok_or(CommandError::NodeNotFound(target_id))?;
        let parent = target.parent().ok_or(CommandError::NoParent(target_id))?;
        let parent_id = parent.id();
        let index = target
            .index()
            .ok_or_else(|| CommandError::orphan_child(target_id, parent_id))?;
        let next = next_block_sibling_id(&parent, target_id);
        (parent_id, index, next)
    };

    let sp = tr.savepoint();
    tr.move_node(source_id, target_parent, target_index + 1)?;

    let new_next = {
        let view = tr.state().view();
        view.node(target_parent)
            .and_then(|p| next_block_sibling_id(&p, target_id))
    };

    if new_next.is_some() && new_next != orig_next {
        tr.merge_node(target_id)?;
        return Ok(());
    }

    tr.rollback(sp);

    let (slots, target_len) = {
        let state = tr.state();
        let view = state.view();
        let source = view
            .node(source_id)
            .ok_or(CommandError::NodeNotFound(source_id))?;
        let source_len = source.children().count();
        if source
            .children()
            .any(|child| !matches!(child, ChildView::Leaf(leaf) if leaf.is_charlike()))
        {
            return Err(CommandError::Corrupted(
                "cross-parent merge cannot preserve non-charlike source content".into(),
            ));
        }
        let slots = capture_charlike_slots(&state.projected, &source, 0, source_len)?;
        let target_len = view
            .node(target_id)
            .map(|t| t.children().count())
            .ok_or(CommandError::NodeNotFound(target_id))?;
        (slots, target_len)
    };
    insert_charlike_slots(tr, target_id, target_len, &slots)?;
    tr.remove_subtree(source_id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_crdt::{ListOp, sequence::Bias as SeqBias};
    use editor_macros::state;
    use editor_model::{EditOp, NodeType, SeqItem};

    use super::*;

    #[test]
    fn cross_parent_merge_preserves_a_source_page_break_when_the_move_is_valid() {
        let (initial, t1, src) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t1: paragraph { text("A") } }
                    }
                    src: paragraph { text("B") page_break }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let page_break = initial
            .view()
            .node(src)
            .expect("source paragraph")
            .children()
            .find_map(|child| match child {
                ChildView::Leaf(leaf) if leaf.node_type() == NodeType::PageBreak => {
                    Some(leaf.dot())
                }
                _ => None,
            })
            .expect("source PageBreak");
        let mut tr = Transaction::new(&initial);
        merge_element_cross_parent(&mut tr, src, t1).unwrap();
        let (actual, ..) = tr.commit();

        let view = actual.view();
        let paragraph = view.node(t1).expect("target paragraph survives");
        assert_eq!(paragraph.inline_text(), "AB");
        let page_break = view
            .alias_classes()
            .members_of(page_break)
            .into_iter()
            .flatten()
            .copied()
            .find(|dot| view.leaf(*dot).is_some())
            .unwrap_or(page_break);
        assert!(
            view.leaf(page_break)
                .is_some_and(|leaf| leaf.node_type() == NodeType::PageBreak),
            "the source PageBreak stays reachable while projection places it schema-validly"
        );
        assert!(view.node(src).is_none(), "the source paragraph is merged");
    }

    #[test]
    fn cross_parent_merge_rejects_page_break_before_a_lossy_fallback() {
        let (initial, title, source) = state! {
            doc { root {
                fold {
                    title: fold_title { text("A") }
                    fold_content { paragraph { text("inside") } }
                }
                source: paragraph { text("B") page_break }
                paragraph {}
            } }
            selection: (title, 0)
        };
        let mut tr = Transaction::new(&initial);
        assert!(
            merge_element_cross_parent(&mut tr, source, title).is_err(),
            "an incompatible PageBreak must block the copy-and-delete fallback"
        );
        let (actual, steps, ..) = tr.commit();

        editor_state::assert_state_eq!(&actual, &initial);
        assert!(steps.is_empty(), "the rejected merge is atomic");
    }

    #[test]
    fn block_container_merge_preserves_content_owned_by_a_synthetic_wrapper() {
        let (mut initial, earlier, tail) = state! {
            doc { root {
                earlier: ordered_list {
                    list_item { paragraph { text("A") } }
                }
                tail: paragraph {}
            } }
            selection: none
        };
        let pos = initial
            .projected
            .seq_boundary_pos(tail, SeqBias::Before)
            .expect("root insertion boundary");
        let later = initial
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos,
                item: SeqItem::Block {
                    node_type: NodeType::BulletList,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }))
            .unwrap()
            .id;
        initial
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: pos + 1,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, later],
                    attrs: vec![],
                },
            }))
            .unwrap();
        initial
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: pos + 2,
                item: SeqItem::Char('B'),
            }))
            .unwrap();
        {
            let view = initial.view();
            let later = view.node(later).expect("later list");
            let item = later.child_blocks().next().expect("projected list item");
            assert_eq!(item.node_type(), NodeType::ListItem);
            assert!(
                item.id().is_synthetic(),
                "the raw Paragraph is owned by a synthetic ListItem repair wrapper"
            );
        }

        let mut tr = Transaction::new(&initial);
        merge_block_container_into(&mut tr, earlier, later).unwrap();
        let (actual, ..) = tr.commit();

        let view = actual.view();
        let earlier = view.node(earlier).expect("earlier list survives");
        let mut texts = Vec::new();
        for item in earlier.child_blocks() {
            texts.extend(item.child_blocks().map(|paragraph| paragraph.inline_text()));
        }
        assert_eq!(texts, ["A", "B"]);
        assert!(view.node(later).is_none(), "the later container is merged");
    }
}
