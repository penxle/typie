use std::collections::HashSet;

use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeType, Schema, content_placement};
use editor_state::Position;

use super::{LinearDeletionKind, LinearDeletionPlan, LinearJoinExecution};
use crate::CommandError;
use crate::helpers::{SliceInsertionTarget, SliceInsertionTargetNode, is_list_type};

pub(crate) struct JoinedReplacementFrontier {
    pub(crate) target: SliceInsertionTarget,
    pub(crate) join: Option<LinearJoinExecution>,
}

pub(crate) struct UnjoinedReplacementFrontiers {
    pub(crate) left: SliceInsertionTarget,
    pub(crate) right: SliceInsertionTarget,
}

#[derive(Clone)]
struct VirtualChild {
    id: Option<Dot>,
    node_type: NodeType,
    node: Option<Box<VirtualNode>>,
}

#[derive(Clone)]
struct VirtualNode {
    id: Dot,
    node_type: NodeType,
    children: Vec<VirtualChild>,
}

impl VirtualChild {
    fn contributes_authored_content(&self) -> bool {
        self.id.is_some_and(|id| id.as_op_dot().is_some())
            || self.node.as_deref().is_some_and(|node| {
                node.children
                    .iter()
                    .any(VirtualChild::contributes_authored_content)
            })
    }
}

impl VirtualNode {
    fn capture(view: &DocView, id: Dot, expanded: &HashSet<Dot>) -> Option<Self> {
        let node = view.node(id)?;
        let children = node
            .children()
            .map(|child| match child {
                ChildView::Block(block) => VirtualChild {
                    id: Some(block.id()),
                    node_type: block.node_type(),
                    node: expanded
                        .contains(&block.id())
                        .then(|| Self::capture(view, block.id(), expanded))
                        .flatten()
                        .map(Box::new),
                },
                ChildView::Leaf(leaf) => VirtualChild {
                    id: Some(leaf.dot()),
                    node_type: leaf.node_type(),
                    node: None,
                },
            })
            .collect();
        Some(Self {
            id,
            node_type: node.node_type(),
            children,
        })
    }

    fn find(&self, id: Dot) -> Option<&Self> {
        if self.id == id {
            return Some(self);
        }
        self.children
            .iter()
            .filter_map(|child| child.node.as_deref())
            .find_map(|child| child.find(id))
    }

    fn find_mut(&mut self, id: Dot) -> Option<&mut Self> {
        if self.id == id {
            return Some(self);
        }
        self.children
            .iter_mut()
            .filter_map(|child| child.node.as_deref_mut())
            .find_map(|child| child.find_mut(id))
    }

    fn parent_and_index(&self, id: Dot) -> Option<(Dot, usize)> {
        for (index, child) in self.children.iter().enumerate() {
            if child.id == Some(id) {
                return Some((self.id, index));
            }
            if let Some(found) = child
                .node
                .as_deref()
                .and_then(|node| node.parent_and_index(id))
            {
                return Some(found);
            }
        }
        None
    }

    fn remove_child(&mut self, id: Dot) -> Option<VirtualChild> {
        if let Some(index) = self.children.iter().position(|child| child.id == Some(id)) {
            return Some(self.children.remove(index));
        }
        self.children
            .iter_mut()
            .filter_map(|child| child.node.as_deref_mut())
            .find_map(|node| node.remove_child(id))
    }

    fn path_to(&self, id: Dot, out: &mut Vec<SliceInsertionTargetNode>) -> bool {
        out.push(SliceInsertionTargetNode {
            id: self.id,
            node_type: self.node_type,
            child_types: self.children.iter().map(|child| child.node_type).collect(),
            child_ids: self.children.iter().map(|child| child.id).collect(),
            index: None,
        });
        if self.id == id {
            return true;
        }
        for (index, child) in self.children.iter().enumerate() {
            let Some(node) = child.node.as_deref() else {
                continue;
            };
            let before = out.len();
            if node.path_to(id, out) {
                if let Some(path_node) = out.get_mut(before) {
                    path_node.index = Some(index);
                }
                return true;
            }
            out.truncate(before);
        }
        out.pop();
        false
    }

    fn complete_expanded(&mut self) -> bool {
        for child in &mut self.children {
            if let Some(node) = child.node.as_deref_mut()
                && !node.complete_expanded()
            {
                return false;
            }
        }
        let types = self
            .children
            .iter()
            .map(|child| child.node_type)
            .collect::<Vec<_>>();
        let placement = content_placement(self.node_type, &types);
        let Some(completion) = placement.completion_insertions else {
            return false;
        };
        if placement.first_residue.is_some() {
            return false;
        }
        for insertion in completion {
            self.children.insert(
                insertion.index,
                VirtualChild {
                    id: None,
                    node_type: insertion.node_type,
                    node: None,
                },
            );
        }
        true
    }
}

pub(crate) fn plan_joined_replacement_frontier(
    view: &DocView,
    deletion: &LinearDeletionPlan,
) -> Result<Option<JoinedReplacementFrontier>, CommandError> {
    let Some((mut virtual_root, join)) = simulate_linear_deletion(view, deletion)? else {
        return Ok(None);
    };
    if join
        .as_ref()
        .is_some_and(|join| join.trailing_page_break.is_some())
    {
        return Ok(None);
    }
    if !virtual_root.complete_expanded() {
        return Ok(None);
    }

    let target_id = deletion.from.node;
    if virtual_root.find(target_id).is_none() {
        return Ok(None);
    }
    let mut path = Vec::new();
    if !virtual_root.path_to(target_id, &mut path) {
        return Ok(None);
    }
    let position = Position {
        node: target_id,
        offset: deletion.from.offset,
        affinity: deletion.from.affinity,
    };
    Ok(SliceInsertionTarget::from_path(position, path)
        .map(|target| JoinedReplacementFrontier { target, join }))
}

pub(crate) fn plan_unjoined_replacement_frontiers(
    view: &DocView,
    deletion: &LinearDeletionPlan,
) -> Result<Option<UnjoinedReplacementFrontiers>, CommandError> {
    let Some(mut virtual_root) = simulate_unjoined_deletion(view, deletion)? else {
        return Ok(None);
    };
    if !virtual_root.complete_expanded() {
        return Ok(None);
    }

    let Some(left) = target_in_virtual_tree(&virtual_root, deletion.from) else {
        return Ok(None);
    };
    let right_position = if deletion.from.node == deletion.to.node {
        deletion.from
    } else {
        Position {
            node: deletion.to.node,
            offset: 0,
            affinity: deletion.to.affinity,
        }
    };
    let Some(right) = target_in_virtual_tree(&virtual_root, right_position) else {
        return Ok(None);
    };
    Ok(Some(UnjoinedReplacementFrontiers { left, right }))
}

fn target_in_virtual_tree(
    virtual_root: &VirtualNode,
    position: Position,
) -> Option<SliceInsertionTarget> {
    let target = virtual_root.find(position.node)?;
    if position.offset > target.children.len() {
        return None;
    }
    let mut path = Vec::new();
    if !virtual_root.path_to(position.node, &mut path) {
        return None;
    }
    SliceInsertionTarget::from_path(position, path)
}

pub(crate) fn plan_linear_join(
    view: &DocView,
    deletion: &LinearDeletionPlan,
) -> Result<Option<LinearJoinExecution>, CommandError> {
    if !matches!(
        deletion.kind,
        LinearDeletionKind::Cross {
            merge_textblocks: true,
            ..
        }
    ) {
        return Ok(None);
    }
    let Some((_, join)) = simulate_linear_deletion(view, deletion)? else {
        return Err(CommandError::Corrupted(
            "linear deletion could not simulate its planned join".into(),
        ));
    };
    Ok(Some(join.ok_or_else(|| {
        CommandError::Corrupted("linear deletion lost its planned join".into())
    })?))
}

fn simulate_linear_deletion(
    view: &DocView,
    deletion: &LinearDeletionPlan,
) -> Result<Option<(VirtualNode, Option<LinearJoinExecution>)>, CommandError> {
    let Some(mut virtual_root) = simulate_unjoined_deletion(view, deletion)? else {
        return Ok(None);
    };
    let join = if let LinearDeletionKind::Cross {
        geometry,
        from_textblock: Some(from_textblock),
        to_textblock: Some(to_textblock),
        merge_textblocks: true,
    } = &deletion.kind
    {
        Some(virtual_merge_after_delete(
            &mut virtual_root,
            *from_textblock,
            *to_textblock,
            geometry.lca_id,
            affected_nodes(view, deletion),
        )?)
    } else {
        None
    };
    Ok(Some((virtual_root, join)))
}

fn simulate_unjoined_deletion(
    view: &DocView,
    deletion: &LinearDeletionPlan,
) -> Result<Option<VirtualNode>, CommandError> {
    let root = view
        .root()
        .ok_or_else(|| CommandError::Corrupted("linear deletion has no root".into()))?;
    let mut expanded = HashSet::new();
    for endpoint in [deletion.from.node, deletion.to.node] {
        let node = view
            .node(endpoint)
            .ok_or(CommandError::NodeNotFound(endpoint))?;
        let ancestors = node.ancestors().collect::<Vec<_>>();
        expanded.extend(ancestors.iter().map(|ancestor| ancestor.id()));
        // A container join can consume the first or last surviving sibling at
        // the splice seam even when that sibling is not itself an endpoint
        // ancestor (notably trailing/leading nested lists in ListItems).
        // Expand those direct children so the planner can preserve and move
        // their real children without capturing the whole document.
        expanded.extend(ancestors.iter().flat_map(|ancestor| {
            ancestor.children().filter_map(|child| match child {
                ChildView::Block(block) => Some(block.id()),
                ChildView::Leaf(_) => None,
            })
        }));
    }
    let mut virtual_root = VirtualNode::capture(view, root.id(), &expanded)
        .ok_or_else(|| CommandError::Corrupted("cannot capture linear deletion target".into()))?;
    if !virtual_delete_range(&mut virtual_root, &deletion.from_path, &deletion.to_path) {
        return Ok(None);
    }
    Ok(Some(virtual_root))
}

fn affected_nodes(view: &DocView, deletion: &LinearDeletionPlan) -> Vec<Dot> {
    let mut affected = Vec::new();
    for endpoint in [deletion.from.node, deletion.to.node] {
        let mut current = view.node(endpoint);
        while let Some(node) = current {
            if !affected.contains(&node.id()) {
                affected.push(node.id());
            }
            current = node.parent();
        }
    }
    affected
}

fn virtual_delete_range(node: &mut VirtualNode, from: &[usize], to: &[usize]) -> bool {
    let Some((&from_index, from_rest)) = from.split_first() else {
        return false;
    };
    let Some((&to_index, to_rest)) = to.split_first() else {
        return false;
    };

    if from_index == to_index {
        return match (from_rest.is_empty(), to_rest.is_empty()) {
            (true, true) => virtual_remove_slots(node, from_index, to_index),
            (true, false) => node
                .children
                .get_mut(from_index)
                .and_then(|child| child.node.as_deref_mut())
                .is_some_and(|child| virtual_delete_to(child, to_rest)),
            (false, true) => node
                .children
                .get_mut(from_index)
                .and_then(|child| child.node.as_deref_mut())
                .is_some_and(|child| virtual_delete_from(child, from_rest)),
            (false, false) => node
                .children
                .get_mut(from_index)
                .and_then(|child| child.node.as_deref_mut())
                .is_some_and(|child| virtual_delete_range(child, from_rest, to_rest)),
        };
    }

    let from_child_id = (!from_rest.is_empty())
        .then(|| node.children.get(from_index)?.id)
        .flatten();
    let to_child_id = (!to_rest.is_empty())
        .then(|| node.children.get(to_index)?.id)
        .flatten();

    if let Some(from_child_id) = from_child_id {
        let Some(from_child) = node
            .children
            .iter_mut()
            .find(|child| child.id == Some(from_child_id))
            .and_then(|child| child.node.as_deref_mut())
        else {
            return false;
        };
        if !virtual_delete_from(from_child, from_rest) {
            return false;
        }
    }

    let fully_from = if from_rest.is_empty() {
        from_index
    } else {
        from_index + 1
    };
    if !virtual_remove_slots(node, fully_from, to_index) {
        return false;
    }

    if let Some(to_child_id) = to_child_id {
        let Some(to_child) = node
            .children
            .iter_mut()
            .find(|child| child.id == Some(to_child_id))
            .and_then(|child| child.node.as_deref_mut())
        else {
            return false;
        };
        if !virtual_delete_to(to_child, to_rest) {
            return false;
        }
    }
    true
}

fn virtual_delete_from(node: &mut VirtualNode, path: &[usize]) -> bool {
    let Some((&index, rest)) = path.split_first() else {
        return false;
    };
    if rest.is_empty() {
        return virtual_remove_slots(node, index, node.children.len());
    }
    let Some(child_id) = node.children.get(index).and_then(|child| child.id) else {
        return false;
    };
    if !virtual_remove_slots(node, index + 1, node.children.len()) {
        return false;
    }
    node.children
        .iter_mut()
        .find(|child| child.id == Some(child_id))
        .and_then(|child| child.node.as_deref_mut())
        .is_some_and(|child| virtual_delete_from(child, rest))
}

fn virtual_delete_to(node: &mut VirtualNode, path: &[usize]) -> bool {
    let Some((&index, rest)) = path.split_first() else {
        return false;
    };
    if rest.is_empty() {
        return virtual_remove_slots(node, 0, index);
    }
    let Some(child_id) = node.children.get(index).and_then(|child| child.id) else {
        return false;
    };
    if !virtual_remove_slots(node, 0, index) {
        return false;
    }
    node.children
        .iter_mut()
        .find(|child| child.id == Some(child_id))
        .and_then(|child| child.node.as_deref_mut())
        .is_some_and(|child| virtual_delete_to(child, rest))
}

fn virtual_remove_slots(node: &mut VirtualNode, from: usize, to: usize) -> bool {
    if from > to || to > node.children.len() {
        return false;
    }
    if node.children[from..to]
        .iter()
        .any(|child| Schema::node_spec(child.node_type).structural)
    {
        return false;
    }
    node.children.drain(from..to);
    true
}

fn virtual_merge_after_delete(
    root: &mut VirtualNode,
    from_textblock: Dot,
    to_textblock: Dot,
    lca: Dot,
    affected: Vec<Dot>,
) -> Result<LinearJoinExecution, CommandError> {
    let to_parent = root
        .parent_and_index(to_textblock)
        .map(|(parent, _)| parent);
    let mut moved = {
        let to = root.find_mut(to_textblock).ok_or_else(|| {
            CommandError::Corrupted("virtual deletion lost its right textblock".into())
        })?;
        std::mem::take(&mut to.children)
            .into_iter()
            .filter(VirtualChild::contributes_authored_content)
            .collect::<Vec<_>>()
    };
    let trailing_page_break = {
        let from = root.find_mut(from_textblock).ok_or_else(|| {
            CommandError::Corrupted("virtual deletion lost its left textblock".into())
        })?;
        let trailing = from
            .children
            .last()
            .is_some_and(|child| child.node_type == NodeType::PageBreak)
            .then(|| from.children.last().and_then(|child| child.id))
            .flatten();
        if trailing.is_some() {
            from.children.pop();
        }
        from.children.append(&mut moved);
        trailing
    };
    root.remove_child(to_textblock).ok_or_else(|| {
        CommandError::Corrupted("virtual deletion could not remove right textblock".into())
    })?;

    let mut prune = Vec::new();
    if let Some(mut current) = to_parent {
        loop {
            if current == lca {
                break;
            }
            let Some(node) = root.find(current) else {
                break;
            };
            let real_children = node
                .children
                .iter()
                .filter(|child| child.contributes_authored_content())
                .count();
            let spec = Schema::node_spec(node.node_type);
            if real_children != 0 || spec.content.min_required() == 0 || spec.structural {
                break;
            }
            let Some((parent, _)) = root.parent_and_index(current) else {
                break;
            };
            root.remove_child(current);
            prune.push(current);
            current = parent;
        }
    }

    let mut container_merges = Vec::new();
    let mut current = root
        .parent_and_index(from_textblock)
        .map(|(parent, _)| parent);
    while let Some(current_id) = current {
        if current_id == lca {
            break;
        }
        let Some((parent_id, current_index)) = root.parent_and_index(current_id) else {
            break;
        };
        let next = root
            .find(parent_id)
            .and_then(|parent| parent.children.get(current_index + 1))
            .and_then(|child| child.id)
            .filter(|id| root.find(*id).is_some());
        if let Some(next_id) = next {
            let current_type = root
                .find(current_id)
                .map(|node| node.node_type)
                .ok_or_else(|| {
                    CommandError::Corrupted("virtual deletion lost its left container".into())
                })?;
            let next_type = root
                .find(next_id)
                .map(|node| node.node_type)
                .ok_or_else(|| {
                    CommandError::Corrupted("virtual deletion lost its right container".into())
                })?;
            if current_type == next_type || (is_list_type(current_type) && is_list_type(next_type))
            {
                if current_type == NodeType::ListItem && next_type == NodeType::ListItem {
                    let seam = {
                        let left = root.find(current_id).and_then(|node| {
                            node.children
                                .iter()
                                .rev()
                                .find(|child| child.contributes_authored_content())
                                .and_then(|child| child.id)
                        });
                        let right = root.find(next_id).and_then(|node| {
                            node.children
                                .iter()
                                .find(|child| child.contributes_authored_content())
                                .and_then(|child| child.id)
                        });
                        left.zip(right).filter(|(left, right)| {
                            root.find(*left)
                                .is_some_and(|node| is_list_type(node.node_type))
                                && root
                                    .find(*right)
                                    .is_some_and(|node| is_list_type(node.node_type))
                        })
                    };
                    if let Some((left, right)) = seam {
                        container_merges.push((left, right));
                        virtual_merge_containers(root, left, right)?;
                    }
                }
                container_merges.push((current_id, next_id));
                virtual_merge_containers(root, current_id, next_id)?;
            }
        }
        current = Some(parent_id);
    }
    Ok(LinearJoinExecution {
        from_textblock,
        to_textblock,
        trailing_page_break,
        prune,
        container_merges,
        affected,
    })
}

fn virtual_merge_containers(
    root: &mut VirtualNode,
    target: Dot,
    source: Dot,
) -> Result<(), CommandError> {
    let source = root
        .remove_child(source)
        .ok_or_else(|| CommandError::Corrupted("virtual deletion lost merge source".into()))?;
    let mut moving = source
        .node
        .map(|node| node.children)
        .unwrap_or_default()
        .into_iter()
        .filter(VirtualChild::contributes_authored_content)
        .collect::<Vec<_>>();
    root.find_mut(target)
        .ok_or_else(|| CommandError::Corrupted("virtual deletion lost merge target".into()))?
        .children
        .append(&mut moving);
    Ok(())
}
