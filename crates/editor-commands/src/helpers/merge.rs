use editor_crdt::Dot;
use editor_model::NodeView;
use editor_transaction::Transaction;

use crate::CommandError;

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
/// Single-slot containers (a `ListItem` holds exactly one paragraph) reject the
/// extra sibling: projection normalization drops it again, so the move cannot
/// land. In that case `source`'s inline text is appended to `target` and the
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

    let text = {
        let view = tr.state().view();
        view.node(source_id)
            .map(|s| s.inline_text())
            .unwrap_or_default()
    };
    let target_len = {
        let view = tr.state().view();
        view.node(target_id)
            .map(|t| t.children().count())
            .ok_or(CommandError::NodeNotFound(target_id))?
    };
    if !text.is_empty() {
        tr.insert_text(target_id, target_len, &text)?;
    }
    tr.remove_subtree(source_id)?;
    Ok(())
}
