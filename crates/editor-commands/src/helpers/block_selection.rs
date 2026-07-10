use editor_crdt::Dot;
use editor_model::{ChildView, NodeType, PlainNode, Schema, Subtree};
use editor_state::{Position, ResolvedSelection, Selection, StableResolveCtx, StableSelection};
use editor_transaction::{Transaction, fulfill};

use super::{child_node_type, is_list_type, materialize_position_block};
use crate::{CommandError, CommandResult};

pub(crate) struct SelectedBlockRun {
    pub(crate) parent_id: Dot,
    pub(crate) blocks: Vec<SelectedBlock>,
}

#[derive(Clone)]
pub(crate) struct SelectedBlock {
    pub(crate) id: Dot,
    pub(crate) node_type: NodeType,
    list_segment: Option<SelectedListSegment>,
}

impl SelectedBlock {
    pub(crate) fn is_whole(&self) -> bool {
        self.list_segment.is_none()
    }

    fn keeps_before_items(&self) -> bool {
        self.list_segment
            .as_ref()
            .is_some_and(|segment| !segment.before_items.is_empty())
    }

    fn keeps_after_items(&self) -> bool {
        self.list_segment
            .as_ref()
            .is_some_and(|segment| !segment.after_items.is_empty())
    }
}

#[derive(Clone)]
struct SelectedListSegment {
    before_items: Vec<Dot>,
    selected_items: Vec<Dot>,
    after_items: Vec<Dot>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockSelectionPolicy {
    Standard,
    Wrapper(NodeType),
    Fold,
}

pub(crate) fn resolve_selected_block_run(
    tr: &Transaction,
) -> Result<Option<SelectedBlockRun>, CommandError> {
    resolve_selected_block_run_with_policy(tr, BlockSelectionPolicy::Standard)
}

pub(crate) fn resolve_selected_block_run_for_fold(
    tr: &Transaction,
) -> Result<Option<SelectedBlockRun>, CommandError> {
    resolve_selected_block_run_with_policy(tr, BlockSelectionPolicy::Fold)
}

pub(crate) fn resolve_selected_block_run_for_wrapper(
    tr: &Transaction,
    wrapper_type: NodeType,
) -> Result<Option<SelectedBlockRun>, CommandError> {
    resolve_selected_block_run_with_policy(tr, BlockSelectionPolicy::Wrapper(wrapper_type))
}

fn resolve_selected_block_run_with_policy(
    tr: &Transaction,
    policy: BlockSelectionPolicy,
) -> Result<Option<SelectedBlockRun>, CommandError> {
    let Some(selection) = tr.selection() else {
        return Ok(None);
    };
    let view = tr.view();
    let resolved = selection
        .resolve(&view)
        .ok_or_else(|| CommandError::Corrupted("cannot resolve block selection".into()))?;
    if selection.is_collapsed() {
        let current = view.node(selection.head.node).or_else(|| {
            view.leaf(selection.head.node)
                .and_then(|leaf| leaf.parent())
        });
        let Some(current) = current else {
            return Ok(None);
        };
        if current.node_type() == NodeType::Root || current.id().as_op_dot().is_none() {
            return Ok(None);
        }
        let mut target = current;
        if policy == BlockSelectionPolicy::Fold {
            let mut ancestor = Some(current);
            while let Some(node) = ancestor {
                if matches!(node.node_type(), NodeType::Blockquote | NodeType::Callout) {
                    target = node;
                    break;
                }
                ancestor = node.parent();
            }
        }
        let parent = target.parent().ok_or(CommandError::NoParent(target.id()))?;
        return Ok(Some(SelectedBlockRun {
            parent_id: parent.id(),
            blocks: vec![SelectedBlock {
                id: target.id(),
                node_type: target.node_type(),
                list_segment: None,
            }],
        }));
    }
    let Some(mut current) = view.root() else {
        return Ok(None);
    };

    loop {
        if current.spec().is_textblock() {
            let parent = current
                .parent()
                .ok_or(CommandError::NoParent(current.id()))?;
            if current.id().as_op_dot().is_none() {
                return Ok(None);
            }
            return Ok(Some(SelectedBlockRun {
                parent_id: parent.id(),
                blocks: vec![SelectedBlock {
                    id: current.id(),
                    node_type: current.node_type(),
                    list_segment: None,
                }],
            }));
        }

        let intersecting: Vec<_> = current
            .children()
            .enumerate()
            .filter_map(|(slot, child)| match child {
                ChildView::Block(block) if resolved.intersects_subtree(&block) => {
                    Some(ChildView::Block(block))
                }
                ChildView::Leaf(leaf)
                    if leaf.as_atom().is_some_and(|atom| atom.is_block_level())
                        && resolved.contains_leaf_slot(&current, slot) =>
                {
                    Some(ChildView::Leaf(leaf))
                }
                _ => None,
            })
            .collect();

        if intersecting.is_empty() {
            if current.node_type() == NodeType::Root || !resolved.intersects_subtree(&current) {
                return Ok(None);
            }
            let parent = current
                .parent()
                .ok_or(CommandError::NoParent(current.id()))?;
            if current.id().as_op_dot().is_none() {
                return Ok(None);
            }
            return Ok(Some(SelectedBlockRun {
                parent_id: parent.id(),
                blocks: vec![SelectedBlock {
                    id: current.id(),
                    node_type: current.node_type(),
                    list_segment: None,
                }],
            }));
        }

        if let BlockSelectionPolicy::Wrapper(wrapper_type) = policy
            && intersecting.len() == 1
            && let ChildView::Block(block) = &intersecting[0]
            && block.node_type() == wrapper_type
            && resolved.contains_subtree(block)
        {
            current = *block;
            continue;
        }

        let promotes_atomic_container = intersecting.iter().any(|child| {
            let ChildView::Block(block) = child else {
                return false;
            };
            should_promote_atomic_container(&view, &selection, policy, block)
        });

        if !promotes_atomic_container
            && intersecting.len() == 1
            && let ChildView::Block(block) = &intersecting[0]
            && !resolved.contains_subtree(block)
        {
            current = *block;
            continue;
        }

        let in_list_structure =
            is_list_type(current.node_type()) || current.node_type() == NodeType::ListItem;
        let partially_selected_non_list_container = intersecting.iter().any(|child| {
            let ChildView::Block(block) = child else {
                return false;
            };
            if resolved.contains_subtree(block)
                || block.spec().is_textblock()
                || is_list_type(block.node_type())
                || in_list_structure
            {
                return false;
            }
            !should_promote_atomic_container(&view, &selection, policy, block)
        });
        if partially_selected_non_list_container {
            return Ok(None);
        }

        if intersecting
            .iter()
            .any(|child| child_view_id(child).as_op_dot().is_none())
        {
            return Ok(None);
        }
        return Ok(Some(SelectedBlockRun {
            parent_id: current.id(),
            blocks: intersecting
                .into_iter()
                .map(|child| SelectedBlock {
                    id: child_view_id(&child),
                    node_type: child_node_type(&child),
                    list_segment: None,
                })
                .collect(),
        }));
    }
}

fn should_promote_atomic_container(
    view: &editor_model::DocView,
    selection: &Selection,
    policy: BlockSelectionPolicy,
    block: &editor_model::NodeView,
) -> bool {
    match policy {
        BlockSelectionPolicy::Standard => false,
        BlockSelectionPolicy::Wrapper(wrapper_type) => {
            block.node_type() == wrapper_type
                && !(position_is_within(view, selection.anchor.node, block.id())
                    && position_is_within(view, selection.head.node, block.id()))
        }
        BlockSelectionPolicy::Fold => {
            matches!(block.node_type(), NodeType::Blockquote | NodeType::Callout)
        }
    }
}

fn position_is_within(view: &editor_model::DocView, position_node: Dot, ancestor_id: Dot) -> bool {
    let mut current = view
        .node(position_node)
        .or_else(|| view.leaf(position_node).and_then(|leaf| leaf.parent()));
    while let Some(node) = current {
        if node.id() == ancestor_id {
            return true;
        }
        current = node.parent();
    }
    false
}

pub(crate) fn promote_list_run(
    tr: &Transaction,
    mut run: SelectedBlockRun,
) -> Result<SelectedBlockRun, CommandError> {
    let view = tr.view();
    let selection = tr
        .selection()
        .ok_or_else(|| CommandError::Corrupted("missing block selection".into()))?;
    let resolved = selection
        .resolve(&view)
        .ok_or_else(|| CommandError::Corrupted("cannot resolve block selection".into()))?;
    let parent = view
        .node(run.parent_id)
        .ok_or(CommandError::NodeNotFound(run.parent_id))?;

    let mut list = if is_list_type(parent.node_type()) {
        parent
    } else if parent.node_type() == NodeType::ListItem {
        let candidate = parent.parent().ok_or(CommandError::NoParent(parent.id()))?;
        if !is_list_type(candidate.node_type()) {
            return Err(CommandError::Corrupted(
                "list item is not owned by a list".into(),
            ));
        }
        candidate
    } else {
        for block in &mut run.blocks {
            if is_list_type(block.node_type) {
                let list = view
                    .node(block.id)
                    .ok_or(CommandError::NodeNotFound(block.id))?;
                *block = select_list_segment(&resolved, &list)?;
            }
        }
        return Ok(run);
    };

    loop {
        let owner = list.parent().ok_or(CommandError::NoParent(list.id()))?;
        if owner.node_type() != NodeType::ListItem {
            return Ok(SelectedBlockRun {
                parent_id: owner.id(),
                blocks: vec![select_list_segment(&resolved, &list)?],
            });
        }
        let outer = owner.parent().ok_or(CommandError::NoParent(owner.id()))?;
        if !is_list_type(outer.node_type()) {
            return Err(CommandError::Corrupted(
                "nested list item is not owned by a list".into(),
            ));
        }
        list = outer;
    }
}

fn select_list_segment(
    resolved: &ResolvedSelection,
    list: &editor_model::NodeView,
) -> Result<SelectedBlock, CommandError> {
    let items: Vec<_> = list
        .child_blocks()
        .filter(|item| item.id().as_op_dot().is_some())
        .collect();
    if items
        .iter()
        .any(|item| item.node_type() != NodeType::ListItem)
    {
        return Err(CommandError::Corrupted(
            "list contains a non-list-item block".into(),
        ));
    }
    let selected_indices: Vec<_> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| resolved.intersects_subtree(item).then_some(index))
        .collect();
    let Some(&start) = selected_indices.first() else {
        return Err(CommandError::Corrupted(
            "selected list does not intersect any direct item".into(),
        ));
    };
    let end = *selected_indices.last().unwrap();
    if selected_indices.len() != end - start + 1 {
        return Err(CommandError::Corrupted(
            "selected list items are not contiguous".into(),
        ));
    }

    let item_ids: Vec<_> = items.iter().map(|item| item.id()).collect();
    let list_segment = (start != 0 || end + 1 != item_ids.len()).then(|| SelectedListSegment {
        before_items: item_ids[..start].to_vec(),
        selected_items: item_ids[start..=end].to_vec(),
        after_items: item_ids[end + 1..].to_vec(),
    });
    Ok(SelectedBlock {
        id: list.id(),
        node_type: list.node_type(),
        list_segment,
    })
}

pub(crate) fn normalize_selected_block_run(
    tr: &mut Transaction,
    run: SelectedBlockRun,
    wrapper: PlainNode,
) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let wrapper_type = wrapper.as_type();
    let matching_wrapper_count = run
        .blocks
        .iter()
        .filter(|block| block.node_type == wrapper_type)
        .count();
    if matching_wrapper_count == 1 && run.blocks.len() == 1 {
        return Ok(false);
    }
    let Some(insert_index) = validate_selected_block_normalize(&tr.view(), &run, wrapper_type)?
    else {
        return Ok(false);
    };
    let stable_selection = StableSelection::capture(&selection, &tr.view());
    let slot_selection =
        if matching_wrapper_count == 0 && run.blocks.iter().all(SelectedBlock::is_whole) {
            remap_slot_selection(selection, run.parent_id, insert_index, run.blocks.len())
        } else {
            None
        };

    let mut survivor_id = None;
    tr.batch::<_, CommandError>(|tr| {
        let run = materialize_selected_block_run(tr, &run)?;
        let existing_survivor = run
            .blocks
            .iter()
            .find(|block| block.node_type == wrapper_type)
            .map(|block| block.id);
        let survivor = if let Some(existing) = existing_survivor {
            tr.set_node(existing, wrapper.clone())?;
            existing
        } else {
            tr.insert_subtree(run.parent_id, insert_index, Subtree::leaf(wrapper.clone()))?;
            block_child_id_at(tr, run.parent_id, insert_index)?
        };
        survivor_id = Some(survivor);
        let wrapper_ids: Vec<_> = run
            .blocks
            .iter()
            .filter_map(|block| (block.node_type == wrapper_type).then_some(block.id))
            .collect();
        for wrapper_id in wrapper_ids {
            materialize_synthetic_direct_children(tr, wrapper_id)?;
        }
        let sources = {
            let view = tr.view();
            run.blocks
                .iter()
                .map(|block| {
                    if block.node_type != wrapper_type {
                        return Ok((block.id, false, vec![block.id]));
                    }
                    let node = view
                        .node(block.id)
                        .ok_or(CommandError::NodeNotFound(block.id))?;
                    let children = node.children().map(|child| child_view_id(&child)).collect();
                    Ok((block.id, true, children))
                })
                .collect::<Result<Vec<_>, CommandError>>()?
        };

        let mut content_index = 0;
        for (source_id, is_wrapper, child_ids) in sources {
            if source_id == survivor {
                content_index += child_ids.len();
                continue;
            }
            for child_id in child_ids {
                tr.move_node(child_id, survivor, content_index)?;
                content_index += 1;
            }
            if is_wrapper {
                tr.remove_subtree(source_id)?;
            }
        }
        apply_fulfill(tr, &[survivor, run.parent_id])?;
        Ok(())
    })?;

    let survivor =
        survivor_id.ok_or_else(|| CommandError::Corrupted("wrapper was not normalized".into()))?;
    if let Some(selection) = slot_selection {
        tr.set_selection(Some(selection.with_node(survivor)))?;
    } else {
        restore_selection(
            tr,
            stable_selection,
            "cannot restore normalized block selection",
        )?;
    }
    Ok(true)
}

pub(crate) fn materialize_selected_block_run(
    tr: &mut Transaction,
    run: &SelectedBlockRun,
) -> Result<SelectedBlockRun, CommandError> {
    let mut blocks = Vec::with_capacity(run.blocks.len());
    for block in &run.blocks {
        let Some(segment) = &block.list_segment else {
            blocks.push(block.clone());
            continue;
        };
        blocks.push(materialize_list_segment(tr, block, segment)?);
    }
    Ok(SelectedBlockRun {
        parent_id: run.parent_id,
        blocks,
    })
}

pub(crate) fn materialize_synthetic_direct_children(
    tr: &mut Transaction,
    parent_id: Dot,
) -> Result<bool, CommandError> {
    let mut changed = false;
    loop {
        let synthetic_child = {
            let view = tr.view();
            let parent = view
                .node(parent_id)
                .ok_or(CommandError::NodeNotFound(parent_id))?;
            parent.children().find_map(|child| match child {
                ChildView::Block(block) if block.id().as_op_dot().is_none() => Some(block.id()),
                _ => None,
            })
        };
        let Some(synthetic_child) = synthetic_child else {
            return Ok(changed);
        };
        materialize_position_block(tr, Position::new(synthetic_child, 0))?;
        changed = true;
    }
}

fn materialize_list_segment(
    tr: &mut Transaction,
    block: &SelectedBlock,
    segment: &SelectedListSegment,
) -> Result<SelectedBlock, CommandError> {
    let (parent_id, list_index, list_node) = {
        let view = tr.view();
        let list = view
            .node(block.id)
            .ok_or(CommandError::NodeNotFound(block.id))?;
        let parent = list.parent().ok_or(CommandError::NoParent(list.id()))?;
        let index = list
            .index()
            .ok_or_else(|| CommandError::orphan_child(list.id(), parent.id()))?;
        (parent.id(), index, list.node().to_plain())
    };

    let selected_id = if segment.before_items.is_empty() {
        block.id
    } else {
        tr.insert_subtree(parent_id, list_index + 1, Subtree::leaf(list_node.clone()))?;
        let selected_id = block_child_id_at(tr, parent_id, list_index + 1)?;
        for (index, item_id) in segment.selected_items.iter().enumerate() {
            tr.move_node(*item_id, selected_id, index)?;
        }
        selected_id
    };

    if !segment.after_items.is_empty() {
        let selected_index = tr
            .view()
            .node(selected_id)
            .and_then(|selected| selected.index())
            .ok_or_else(|| CommandError::orphan_child(selected_id, parent_id))?;
        tr.insert_subtree(parent_id, selected_index + 1, Subtree::leaf(list_node))?;
        let after_id = block_child_id_at(tr, parent_id, selected_index + 1)?;
        for (index, item_id) in segment.after_items.iter().enumerate() {
            tr.move_node(*item_id, after_id, index)?;
        }
    }

    Ok(SelectedBlock {
        id: selected_id,
        node_type: block.node_type,
        list_segment: None,
    })
}

pub(crate) fn lift_selected_block_run(
    tr: &mut Transaction,
    run: SelectedBlockRun,
) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if !validate_selected_block_lift(&tr.view(), &run)? {
        return Ok(false);
    }
    let stable_selection = StableSelection::capture(&selection, &tr.view());

    tr.batch::<_, CommandError>(|tr| {
        let run = materialize_selected_block_run(tr, &run)?;
        let Some(plan) = build_lift_plan(&tr.view(), &run)? else {
            return Err(CommandError::Corrupted(
                "materialized block selection cannot be lifted".into(),
            ));
        };
        let mut fulfilled = vec![plan.parent_id];
        if plan.before.is_empty() {
            for (index, block) in plan.selected.iter().enumerate() {
                tr.move_node(block.id, plan.parent_id, plan.wrapper_index + index)?;
            }
            if plan.after.is_empty() {
                tr.remove_subtree(plan.wrapper_id)?;
            } else {
                fulfilled.push(plan.wrapper_id);
            }
        } else {
            for (index, block) in plan.selected.iter().enumerate() {
                tr.move_node(block.id, plan.parent_id, plan.wrapper_index + 1 + index)?;
            }
            fulfilled.push(plan.wrapper_id);

            if !plan.after.is_empty() {
                let after_index = plan.wrapper_index + 1 + plan.selected.len();
                tr.insert_subtree(
                    plan.parent_id,
                    after_index,
                    Subtree::leaf(plan.wrapper.clone()),
                )?;
                let after_wrapper_id = block_child_id_at(tr, plan.parent_id, after_index)?;
                for (index, block) in plan.after.iter().enumerate() {
                    tr.move_node(block.id, after_wrapper_id, index)?;
                }
                fulfilled.push(after_wrapper_id);
            }
        }
        apply_fulfill(tr, &fulfilled)?;
        Ok(())
    })?;

    restore_selection(
        tr,
        stable_selection,
        "cannot restore lifted block selection",
    )?;
    Ok(true)
}

struct LiftPlan {
    parent_id: Dot,
    wrapper_id: Dot,
    wrapper_index: usize,
    wrapper: PlainNode,
    before: Vec<SelectedBlock>,
    selected: Vec<SelectedBlock>,
    after: Vec<SelectedBlock>,
}

fn build_lift_plan(
    view: &editor_model::DocView,
    run: &SelectedBlockRun,
) -> Result<Option<LiftPlan>, CommandError> {
    let wrapper = view
        .node(run.parent_id)
        .ok_or(CommandError::NodeNotFound(run.parent_id))?;
    let parent = wrapper
        .parent()
        .ok_or(CommandError::NoParent(wrapper.id()))?;
    let wrapper_index = wrapper
        .index()
        .ok_or_else(|| CommandError::orphan_child(wrapper.id(), parent.id()))?;
    let children: Vec<_> = wrapper
        .child_blocks()
        .filter(|child| child.id().as_op_dot().is_some())
        .map(|child| SelectedBlock {
            id: child.id(),
            node_type: child.node_type(),
            list_segment: None,
        })
        .collect();
    let start = children
        .iter()
        .position(|child| child.id == run.blocks[0].id)
        .ok_or_else(|| CommandError::Corrupted("selected block is outside wrapper".into()))?;
    if children.get(start..start + run.blocks.len()).map(|slice| {
        slice
            .iter()
            .map(|block| block.id)
            .eq(run.blocks.iter().map(|block| block.id))
    }) != Some(true)
    {
        return Err(CommandError::Corrupted(
            "selected wrapper blocks are not contiguous".into(),
        ));
    }
    let end = start + run.blocks.len();
    let before = children[..start].to_vec();
    let selected = children[start..end].to_vec();
    let after = children[end..].to_vec();

    let mut replacement = Vec::new();
    if !before.is_empty() {
        replacement.push(wrapper.node_type());
    }
    replacement.extend(selected.iter().map(|block| block.node_type));
    if !after.is_empty() {
        replacement.push(wrapper.node_type());
    }
    let mut parent_types: Vec<_> = parent
        .children()
        .map(|child| child_node_type(&child))
        .collect();
    parent_types.splice(wrapper_index..=wrapper_index, replacement);
    if !parent.spec().content.matches_sequence(&parent_types) {
        return Ok(None);
    }

    Ok(Some(LiftPlan {
        parent_id: parent.id(),
        wrapper_id: wrapper.id(),
        wrapper_index,
        wrapper: wrapper.node().to_plain(),
        before,
        selected,
        after,
    }))
}

fn validate_selected_block_lift(
    view: &editor_model::DocView,
    run: &SelectedBlockRun,
) -> Result<bool, CommandError> {
    if run.blocks.is_empty() {
        return Err(CommandError::Corrupted("empty block selection".into()));
    }
    validate_segment_positions(run)?;
    let wrapper = view
        .node(run.parent_id)
        .ok_or(CommandError::NodeNotFound(run.parent_id))?;
    let parent = wrapper
        .parent()
        .ok_or(CommandError::NoParent(wrapper.id()))?;
    let wrapper_index = wrapper
        .index()
        .ok_or_else(|| CommandError::orphan_child(wrapper.id(), parent.id()))?;
    let children: Vec<_> = wrapper
        .child_blocks()
        .filter(|child| child.id().as_op_dot().is_some())
        .collect();
    let start = children
        .iter()
        .position(|child| child.id() == run.blocks[0].id)
        .ok_or_else(|| CommandError::Corrupted("selected block is outside wrapper".into()))?;
    if children.get(start..start + run.blocks.len()).map(|slice| {
        slice
            .iter()
            .map(|child| child.id())
            .eq(run.blocks.iter().map(|block| block.id))
    }) != Some(true)
    {
        return Err(CommandError::Corrupted(
            "selected wrapper blocks are not contiguous".into(),
        ));
    }
    let end = start + run.blocks.len();
    let has_before = start != 0 || run.blocks[0].keeps_before_items();
    let has_after = end != children.len() || run.blocks.last().unwrap().keeps_after_items();

    let mut replacement = Vec::new();
    if has_before {
        replacement.push(wrapper.node_type());
    }
    replacement.extend(run.blocks.iter().map(|block| block.node_type));
    if has_after {
        replacement.push(wrapper.node_type());
    }
    let mut parent_types: Vec<_> = parent
        .children()
        .map(|child| child_node_type(&child))
        .collect();
    parent_types.splice(wrapper_index..=wrapper_index, replacement);
    Ok(parent.spec().content.matches_sequence(&parent_types))
}

pub(crate) fn validate_selected_block_wrap(
    view: &editor_model::DocView,
    run: &SelectedBlockRun,
    wrapper_type: NodeType,
    content_type: NodeType,
) -> Result<Option<usize>, CommandError> {
    if run.blocks.is_empty() {
        return Err(CommandError::Corrupted("empty block selection".into()));
    }
    validate_segment_positions(run)?;
    let parent = view
        .node(run.parent_id)
        .ok_or(CommandError::NodeNotFound(run.parent_id))?;
    let child_ids: Vec<_> = parent
        .children()
        .map(|child| child_view_id(&child))
        .collect();
    let indices: Vec<_> = run
        .blocks
        .iter()
        .map(|block| {
            child_ids
                .iter()
                .position(|id| *id == block.id)
                .ok_or_else(|| CommandError::orphan_child(block.id, run.parent_id))
        })
        .collect::<Result<_, _>>()?;
    if indices.windows(2).any(|pair| pair[1] != pair[0] + 1) {
        return Err(CommandError::Corrupted(
            "selected blocks are not contiguous siblings".into(),
        ));
    }

    let content_types: Vec<_> = run.blocks.iter().map(|block| block.node_type).collect();
    if !Schema::node_spec(content_type)
        .content
        .matches_sequence(&content_types)
    {
        return Ok(None);
    }

    let start = indices[0];
    let end = *indices.last().unwrap();
    let mut parent_types: Vec<_> = parent
        .children()
        .map(|child| child_node_type(&child))
        .collect();
    let mut replacement = Vec::new();
    if run.blocks[0].keeps_before_items() {
        replacement.push(run.blocks[0].node_type);
    }
    replacement.push(wrapper_type);
    if run.blocks.last().unwrap().keeps_after_items() {
        replacement.push(run.blocks.last().unwrap().node_type);
    }
    parent_types.splice(start..=end, replacement);
    if !parent.spec().content.matches_sequence(&parent_types)
        && !parent.spec().content.is_repeatable(wrapper_type)
    {
        return Ok(None);
    }
    let insert_index = start + usize::from(run.blocks[0].keeps_before_items());
    Ok(Some(insert_index))
}

fn validate_selected_block_normalize(
    view: &editor_model::DocView,
    run: &SelectedBlockRun,
    wrapper_type: NodeType,
) -> Result<Option<usize>, CommandError> {
    if run.blocks.is_empty() {
        return Err(CommandError::Corrupted("empty block selection".into()));
    }
    validate_segment_positions(run)?;
    let parent = view
        .node(run.parent_id)
        .ok_or(CommandError::NodeNotFound(run.parent_id))?;
    let child_ids: Vec<_> = parent
        .children()
        .map(|child| child_view_id(&child))
        .collect();
    let indices: Vec<_> = run
        .blocks
        .iter()
        .map(|block| {
            child_ids
                .iter()
                .position(|id| *id == block.id)
                .ok_or_else(|| CommandError::orphan_child(block.id, run.parent_id))
        })
        .collect::<Result<_, _>>()?;
    if indices.windows(2).any(|pair| pair[1] != pair[0] + 1) {
        return Err(CommandError::Corrupted(
            "selected blocks are not contiguous siblings".into(),
        ));
    }

    let mut content_types = Vec::new();
    for block in &run.blocks {
        if block.node_type == wrapper_type {
            let wrapper = view
                .node(block.id)
                .ok_or(CommandError::NodeNotFound(block.id))?;
            content_types.extend(wrapper.children().map(|child| child_node_type(&child)));
        } else {
            content_types.push(block.node_type);
        }
    }
    if !Schema::node_spec(wrapper_type)
        .content
        .matches_sequence(&content_types)
    {
        return Ok(None);
    }

    let start = indices[0];
    let end = *indices.last().unwrap();
    let mut parent_types: Vec<_> = parent
        .children()
        .map(|child| child_node_type(&child))
        .collect();
    let mut replacement = Vec::new();
    if run.blocks[0].keeps_before_items() {
        replacement.push(run.blocks[0].node_type);
    }
    replacement.push(wrapper_type);
    if run.blocks.last().unwrap().keeps_after_items() {
        replacement.push(run.blocks.last().unwrap().node_type);
    }
    parent_types.splice(start..=end, replacement);
    if !parent.spec().content.matches_sequence(&parent_types)
        && !parent.spec().content.is_repeatable(wrapper_type)
    {
        return Ok(None);
    }
    Ok(Some(
        start + usize::from(run.blocks[0].keeps_before_items()),
    ))
}

fn validate_segment_positions(run: &SelectedBlockRun) -> Result<(), CommandError> {
    for (index, block) in run.blocks.iter().enumerate() {
        if block.is_whole() {
            continue;
        }
        if block.keeps_before_items() && index != 0 {
            return Err(CommandError::Corrupted(
                "only the first selected list may keep preceding items".into(),
            ));
        }
        if block.keeps_after_items() && index + 1 != run.blocks.len() {
            return Err(CommandError::Corrupted(
                "only the last selected list may keep following items".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn block_child_id_at(
    tr: &Transaction,
    parent_id: Dot,
    index: usize,
) -> Result<Dot, CommandError> {
    match tr
        .view()
        .node(parent_id)
        .and_then(|parent| parent.child_at(index))
    {
        Some(ChildView::Block(block)) => Ok(block.id()),
        _ => Err(CommandError::NodeNotFound(parent_id)),
    }
}

fn child_view_id(child: &ChildView<'_>) -> Dot {
    match child {
        ChildView::Block(block) => block.id(),
        ChildView::Leaf(leaf) => leaf.dot(),
    }
}

pub(crate) fn apply_fulfill(tr: &mut Transaction, ids: &[Dot]) -> Result<(), CommandError> {
    let steps = {
        let view = tr.view();
        let mut steps = Vec::new();
        for id in ids {
            let node = view.node(*id).ok_or(CommandError::NodeNotFound(*id))?;
            steps.extend(fulfill(&node));
        }
        steps
    };
    tr.apply_steps(steps)?;
    Ok(())
}

pub(crate) fn restore_selection(
    tr: &mut Transaction,
    stable_selection: StableSelection,
    message: &'static str,
) -> Result<(), CommandError> {
    let selection = {
        let view = tr.view();
        let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
        stable_selection.resolve(&ctx)
    }
    .ok_or_else(|| CommandError::Corrupted(message.into()))?;
    tr.set_selection(Some(selection))?;
    Ok(())
}

pub(crate) struct RelativeSlotSelection {
    anchor: editor_state::Position,
    head: editor_state::Position,
}

impl RelativeSlotSelection {
    pub(crate) fn with_node(self, node: Dot) -> editor_state::Selection {
        editor_state::Selection::new(
            editor_state::Position {
                node,
                ..self.anchor
            },
            editor_state::Position { node, ..self.head },
        )
    }
}

pub(crate) fn remap_slot_selection(
    selection: editor_state::Selection,
    parent_id: Dot,
    start: usize,
    len: usize,
) -> Option<RelativeSlotSelection> {
    if selection.anchor.node != parent_id || selection.head.node != parent_id {
        return None;
    }
    let end = start + len;
    let relative = |position: editor_state::Position| {
        let offset = match position.offset {
            value if value == start => 0,
            value if value == end => len,
            _ => return None,
        };
        Some(editor_state::Position { offset, ..position })
    };
    Some(RelativeSlotSelection {
        anchor: relative(selection.anchor)?,
        head: relative(selection.head)?,
    })
}
