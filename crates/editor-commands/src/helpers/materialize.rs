use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeType};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use super::materialize_position_block;
use crate::CommandError;

/// `true` for a scaffold dot that `materialize_position_block` can turn into a
/// real op dot. Excludes [`Dot::ROOT`]: it is synthetic too, but it is a
/// permanent implicit anchor, never a materializable scaffold.
pub(crate) fn is_materializable_synthetic(node: Dot) -> bool {
    node != Dot::ROOT && node.as_op_dot().is_none()
}

#[derive(Clone)]
pub(crate) struct PlannedSelection {
    anchor: PlannedEndpoint,
    head: PlannedEndpoint,
}

#[derive(Clone)]
pub(crate) enum PlannedEndpoint {
    Authored {
        position: Position,
        node_type: NodeType,
        child_types: Vec<NodeType>,
        ancestry: Vec<PlannedAncestorSlot>,
    },
    ProjectedSlot {
        authored_ancestor: Dot,
        ancestor_type: NodeType,
        ancestor_ancestry: Vec<PlannedAncestorSlot>,
        path: Vec<PlannedChildSlot>,
        offset: usize,
        affinity: Affinity,
    },
}

#[derive(Clone)]
pub(crate) struct PlannedChildSlot {
    index: usize,
    node_type: NodeType,
}

#[derive(Clone)]
pub(crate) struct PlannedAncestorSlot {
    authored_parent: Option<Dot>,
    parent_type: NodeType,
    index: usize,
}

impl PlannedSelection {
    pub(crate) fn capture(view: &DocView, selection: Selection) -> Option<Self> {
        Some(Self {
            anchor: PlannedEndpoint::capture(view, selection.anchor)?,
            head: PlannedEndpoint::capture(view, selection.head)?,
        })
    }

    pub(crate) fn resolve(&self, view: &DocView) -> Option<Selection> {
        Some(Selection::new(
            self.anchor.resolve(view)?,
            self.head.resolve(view)?,
        ))
    }
}

impl PlannedEndpoint {
    pub(crate) fn capture(view: &DocView, position: Position) -> Option<Self> {
        let node = view.node(position.node)?;
        if position.offset > node.children().count() {
            return None;
        }
        if position.node.as_op_dot().is_some() || position.node == Dot::ROOT {
            return Some(Self::Authored {
                position,
                node_type: node.node_type(),
                child_types: node
                    .children()
                    .map(|child| child_node_type(&child))
                    .collect(),
                ancestry: capture_ancestry(&node)?,
            });
        }

        let mut current = node;
        let mut path = Vec::new();
        while current.id().as_op_dot().is_none() && current.id() != Dot::ROOT {
            path.push(PlannedChildSlot {
                index: current.index()?,
                node_type: current.node_type(),
            });
            current = current.parent()?;
        }
        path.reverse();
        Some(Self::ProjectedSlot {
            authored_ancestor: current.id(),
            ancestor_type: current.node_type(),
            ancestor_ancestry: capture_ancestry(&current)?,
            path,
            offset: position.offset,
            affinity: position.affinity,
        })
    }

    pub(crate) fn resolve(&self, view: &DocView) -> Option<Position> {
        match self {
            Self::Authored {
                position,
                node_type,
                child_types,
                ancestry,
            } => {
                let live = resolve_planned_node(view, position.node)?;
                let node = view.node(live)?;
                (node.node_type() == *node_type
                    && position.offset <= node.children().count()
                    && ancestry_matches(view, &node, ancestry)
                    && node
                        .children()
                        .map(|child| child_node_type(&child))
                        .eq(child_types.iter().copied()))
                .then_some(Position {
                    node: live,
                    ..*position
                })
            }
            Self::ProjectedSlot {
                authored_ancestor,
                ancestor_type,
                ancestor_ancestry,
                path,
                offset,
                affinity,
            } => {
                let mut current = view.node(resolve_planned_node(view, *authored_ancestor)?)?;
                if current.node_type() != *ancestor_type
                    || !ancestry_matches(view, &current, ancestor_ancestry)
                {
                    return None;
                }
                for slot in path {
                    let ChildView::Block(child) = current.child_at(slot.index)? else {
                        return None;
                    };
                    if child.node_type() != slot.node_type {
                        return None;
                    }
                    current = view.node(child.id())?;
                }
                (*offset <= current.children().count()).then_some(Position {
                    node: current.id(),
                    offset: *offset,
                    affinity: *affinity,
                })
            }
        }
    }
}

fn capture_ancestry(node: &editor_model::NodeView<'_>) -> Option<Vec<PlannedAncestorSlot>> {
    let mut current = node.clone();
    let mut ancestry = Vec::new();
    while let Some(parent) = current.parent() {
        let parent_id = parent.id();
        ancestry.push(PlannedAncestorSlot {
            authored_parent: (parent_id == Dot::ROOT || parent_id.as_op_dot().is_some())
                .then_some(parent_id),
            parent_type: parent.node_type(),
            index: current.index()?,
        });
        current = parent;
    }
    Some(ancestry)
}

fn resolve_planned_node(view: &DocView, planned: Dot) -> Option<Dot> {
    if view.node(planned).is_some() {
        return Some(planned);
    }
    view.alias_classes()
        .members_of(planned)
        .into_iter()
        .flatten()
        .copied()
        .find(|candidate| view.node(*candidate).is_some())
}

fn ancestry_matches(
    view: &DocView,
    node: &editor_model::NodeView<'_>,
    ancestry: &[PlannedAncestorSlot],
) -> bool {
    let mut current = node.clone();
    for slot in ancestry {
        let Some(parent) = current.parent() else {
            return false;
        };
        let same_parent = slot
            .authored_parent
            .is_none_or(|planned| resolve_planned_node(view, planned) == Some(parent.id()));
        if !same_parent
            || parent.node_type() != slot.parent_type
            || current.index() != Some(slot.index)
        {
            return false;
        }
        current = parent;
    }
    current.parent().is_none()
}

pub(crate) fn resolve_planned_selection(
    tr: &Transaction,
    selection: &PlannedSelection,
) -> Result<Selection, CommandError> {
    selection.resolve(&tr.view()).ok_or_else(|| {
        CommandError::Corrupted("fitted Slice endpoints changed before execution".into())
    })
}

pub(crate) fn materialize_planned_selection(
    tr: &mut Transaction,
    selection: &PlannedSelection,
) -> Result<Selection, CommandError> {
    let projected = install_planned_selection(tr, selection)?;
    let anchor_precedes_head = projected
        .resolve(&tr.view())
        .is_some_and(|resolved| resolved.anchor() <= resolved.head());
    for anchor in if anchor_precedes_head {
        [true, false]
    } else {
        [false, true]
    } {
        let current = tr.selection().ok_or_else(|| {
            CommandError::Corrupted("fitted Slice lost its selection while materializing".into())
        })?;
        let target = if anchor {
            current.anchor.node
        } else {
            current.head.node
        };
        materialize_target(tr, target)?;
    }
    let materialized = tr.selection().ok_or_else(|| {
        CommandError::Corrupted("fitted Slice produced no materialized selection".into())
    })?;
    let expected = selection.resolve(&tr.view()).ok_or_else(|| {
        CommandError::Corrupted(
            "fitted Slice endpoints changed while materializing their repair scaffold".into(),
        )
    })?;
    if materialized != expected {
        return Err(CommandError::Corrupted(
            "fitted Slice endpoint remap does not match its planned structural slots".into(),
        ));
    }
    Ok(materialized)
}

pub(crate) fn install_planned_selection(
    tr: &mut Transaction,
    selection: &PlannedSelection,
) -> Result<Selection, CommandError> {
    let projected = resolve_planned_selection(tr, selection)?;
    if tr.selection() != Some(projected) {
        tr.set_selection(Some(projected))?;
    }
    Ok(projected)
}

pub(crate) fn materialize_planned_endpoint(
    tr: &mut Transaction,
    endpoint: &PlannedEndpoint,
) -> Result<Position, CommandError> {
    let projected = endpoint.resolve(&tr.view()).ok_or_else(|| {
        CommandError::Corrupted("fitted Slice structural endpoint changed before execution".into())
    })?;
    let materialized = materialize_repair_position(tr, projected)?;
    let expected = endpoint.resolve(&tr.view()).ok_or_else(|| {
        CommandError::Corrupted(
            "fitted Slice structural endpoint changed while materializing".into(),
        )
    })?;
    if materialized != expected {
        return Err(CommandError::Corrupted(
            "fitted Slice structural endpoint remap changed its planned slot".into(),
        ));
    }
    Ok(materialized)
}

/// Materialize the complete projection-repair scaffold that owns `position`.
/// Unlike the filler-specific `materialize_position_block`, this preserves and
/// aliases authored descendants already owned by a synthetic wrapper.
pub(crate) fn materialize_repair_position(
    tr: &mut Transaction,
    position: Position,
) -> Result<Position, CommandError> {
    let node = materialize_target(tr, position.node)?;
    Ok(Position { node, ..position })
}

fn child_node_type(child: &ChildView<'_>) -> NodeType {
    match child {
        ChildView::Block(block) => block.node_type(),
        ChildView::Leaf(leaf) => leaf.node_type(),
    }
}

/// Materialize any synthetic scaffold block holding a selection endpoint so
/// every downstream step targets real dots. Returns the remapped selection, or
/// `None` when both endpoints are already real. Endpoints are materialized in
/// document order: filler scaffold ids are keyed by parent slot, so the earlier
/// insertion must not invalidate the later endpoint's identity.
pub(crate) fn materialize_selection_endpoints(
    tr: &mut Transaction,
    selection: Selection,
) -> Result<Option<Selection>, CommandError> {
    let is_synthetic = is_materializable_synthetic;
    if !is_synthetic(selection.anchor.node) && !is_synthetic(selection.head.node) {
        return Ok(None);
    }

    let (anchor, head) = if selection.anchor.node == selection.head.node {
        let materialized = materialize_position_block(tr, selection.anchor)?;
        (
            materialized,
            Position {
                node: materialized.node,
                ..selection.head
            },
        )
    } else if is_synthetic(selection.anchor.node) && is_synthetic(selection.head.node) {
        let anchor_precedes_head = {
            let view = tr.view();
            let resolved = selection.resolve(&view).ok_or_else(|| {
                CommandError::Corrupted("cannot resolve synthetic selection".into())
            })?;
            resolved.anchor() < resolved.head()
        };
        if anchor_precedes_head {
            let anchor = materialize_position_block(tr, selection.anchor)?;
            let head = materialize_position_block(tr, selection.head)?;
            (anchor, head)
        } else {
            let head = materialize_position_block(tr, selection.head)?;
            let anchor = materialize_position_block(tr, selection.anchor)?;
            (anchor, head)
        }
    } else if is_synthetic(selection.anchor.node) {
        (
            materialize_position_block(tr, selection.anchor)?,
            selection.head,
        )
    } else {
        (
            selection.anchor,
            materialize_position_block(tr, selection.head)?,
        )
    };
    Ok(Some(Selection::new(anchor, head)))
}

pub(crate) fn materialize_target(tr: &mut Transaction, target: Dot) -> Result<Dot, CommandError> {
    let before = tr.selection();
    let remapped = editor_transaction::materialize_repair_target(tr, target)?;
    if remapped == target {
        return Ok(remapped);
    }
    if let Some(before) = before {
        let selection = {
            let view = tr.view();
            let remap = |pos: Position| {
                let node = if pos.node == target {
                    remapped
                } else {
                    view.alias_classes().resolve_with(pos.node, |d| {
                        view.node(d).is_some() || view.leaf(d).is_some()
                    })
                };
                Position { node, ..pos }
            };
            Selection::new(remap(before.anchor), remap(before.head))
        };
        tr.set_selection(Some(selection))?;
    }
    Ok(remapped)
}
