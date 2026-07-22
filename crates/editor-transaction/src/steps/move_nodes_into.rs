use editor_crdt::Dot;
use editor_model::{AliasOp, EditOp, NodeType, PlainNode, SeqClass, Subtree, classify};
use editor_state::{BatchedState, ProjectedState};

use crate::StepError;
use crate::steps::move_node::MovedNode;
use crate::steps::support;

/// Where a `MoveNodesInto`/`MoveNodesBack` composite step lands its items:
/// consecutive slots in an already-live parent, or the children of a
/// brand-new container subtree being inserted for the first time.
#[derive(Clone, Debug, PartialEq)]
pub enum MoveDest {
    Existing {
        parent: Dot,
        base_index: usize,
    },
    Fresh {
        parent: Dot,
        index: usize,
        container: Subtree,
    },
}

/// One item of a composite move: its captured subtree — so a replay never
/// needs to re-resolve a live dot for the moved content itself — plus the
/// pre-move `(old_parent, old_index)` position `MoveNodesBack` restores it to.
#[derive(Clone, Debug, PartialEq)]
pub struct MovedItem {
    pub subtree: Subtree,
    pub old_parent: Dot,
    pub old_index: usize,
}

/// Captures `items` (in the given order) from the current projected state —
/// one clean read ahead of any mutation. Shared by `Transaction`'s composite
/// facades; the per-item fallback of `move_nodes_consecutive` reuses this
/// same payload.
pub(crate) fn capture_items(
    ps: &ProjectedState,
    items: &[Dot],
) -> Result<Vec<MovedItem>, StepError> {
    items
        .iter()
        .map(|&item| {
            if support::subtree_has_unknown(ps, item) {
                return Err(StepError::UnknownBearingMove { block: item });
            }
            let subtree =
                support::capture_subtree(ps, item).ok_or(StepError::NodeNotFound(item))?;
            let old_parent = ps.parent_of(item).ok_or(StepError::NodeNotFound(item))?;
            let old_index = support::child_elem_ids(ps, old_parent)
                .iter()
                .position(|&d| d == item)
                .ok_or(StepError::NodeNotFound(item))?;
            Ok(MovedItem {
                subtree,
                old_parent,
                old_index,
            })
        })
        .collect()
}

fn collect_source_dots(subtree: &Subtree, out: &mut Vec<Dot>) {
    out.extend(subtree.source_dots.iter().copied());
    for child in &subtree.children {
        collect_source_dots(child, out);
    }
}

/// `Step::MoveNodesInto`'s forward apply, and the fast path of
/// `Transaction::move_nodes_consecutive`/`insert_subtree_with_moved`: one
/// global descending delete of every item's pre-move dots, one cursor-threaded
/// consecutive emission, one combined alias op. Every projected read (the
/// destination's insert slot, tree parents, and the pre-delete flat positions
/// used for the cursor correction below) happens before the first delete op —
/// `items` is already a clean-snapshot capture from before this call.
///
/// Assumes every item's `old_parent` differs from `dest`'s parent — the same
/// reason `move_node`'s fast path requires `new_parent != old_parent` (a
/// same-parent slot's pre-delete and post-delete index mean different
/// things). `move_nodes_consecutive` routes that case through a per-item
/// fallback instead of this function; this function's own generic replay
/// (`Step::MoveNodesInto`'s dispatch) does not detect or handle it.
pub(crate) fn apply_forward(
    batched: &mut BatchedState,
    dest: &MoveDest,
    items: &[MovedItem],
) -> Result<(Option<Dot>, Vec<MovedNode>), StepError> {
    match dest {
        MoveDest::Existing { parent, base_index } => {
            if items.is_empty() {
                return Ok((None, Vec::new()));
            }
            let mut all_dots: Vec<Dot> = Vec::new();
            for it in items {
                collect_source_dots(&it.subtree, &mut all_dots);
            }
            let (raw_pos, parents, host, positions, del_ops) = {
                let ps = &batched.projected;
                let probe_type = items[0].subtree.node.as_type();
                let raw_pos = support::child_seq_insert_pos(ps, *parent, *base_index, probe_type)?;
                let parents = support::self_inclusive_parents(ps, *parent)
                    .ok_or(StepError::NodeNotFound(*parent))?;
                let host = support::parent_host_type(ps, &parents);
                let positions: Vec<usize> = all_dots
                    .iter()
                    .filter_map(|&d| ps.seq_flat_pos(d))
                    .collect();
                let del_ops = support::delete_dots_ops(ps, &all_dots);
                (raw_pos, parents, host, positions, del_ops)
            };
            for op in del_ops {
                batched.apply(op)?;
            }
            let before = positions.iter().filter(|&&p| p < raw_pos).count();
            let mut seq_pos = raw_pos - before;

            let mut all_pairs: Vec<(Dot, Dot)> = Vec::new();
            let mut moved = Vec::with_capacity(items.len());
            for it in items {
                let mut pairs = Vec::new();
                let root = support::emit_subtree(
                    batched,
                    &it.subtree,
                    &parents,
                    host,
                    &mut seq_pos,
                    &mut pairs,
                )?
                .ok_or(StepError::NodeNotFound(it.old_parent))?;
                all_pairs.extend(pairs.iter().copied());
                moved.push(MovedNode { root, pairs });
            }
            if !all_pairs.is_empty() {
                batched.apply(EditOp::Alias(AliasOp {
                    pairs: support::compress_alias_pairs(&all_pairs),
                }))?;
            }
            Ok((None, moved))
        }
        MoveDest::Fresh {
            parent,
            index,
            container,
        } => {
            let mut all_dots: Vec<Dot> = Vec::new();
            for it in items {
                collect_source_dots(&it.subtree, &mut all_dots);
            }
            let (raw_pos, parents, host, positions, del_ops) = {
                let ps = &batched.projected;
                let raw_pos =
                    support::child_seq_insert_pos(ps, *parent, *index, container.node.as_type())?;
                let parents = support::self_inclusive_parents(ps, *parent)
                    .ok_or(StepError::NodeNotFound(*parent))?;
                let host = support::parent_host_type(ps, &parents);
                let positions: Vec<usize> = all_dots
                    .iter()
                    .filter_map(|&d| ps.seq_flat_pos(d))
                    .collect();
                let del_ops = support::delete_dots_ops(ps, &all_dots);
                (raw_pos, parents, host, positions, del_ops)
            };
            for op in del_ops {
                batched.apply(op)?;
            }
            let before = positions.iter().filter(|&&p| p < raw_pos).count();
            let mut seq_pos = raw_pos - before;

            // The container and its grafted items form one connected emission
            // unit (no read in between): the container is emitted first under
            // `parents`, then every item is emitted as if it were one of the
            // container's own children — same op sequence a genuinely merged
            // `Subtree` would produce, but keeping each item's own root/pairs
            // separately addressable for the returned `Vec<MovedNode>`.
            let mut container_pairs = Vec::new();
            let container_dot = support::emit_subtree(
                batched,
                container,
                &parents,
                host,
                &mut seq_pos,
                &mut container_pairs,
            )?
            .ok_or(StepError::NodeNotFound(*parent))?;

            let mut child_parents = parents;
            child_parents.push(container_dot);
            let container_type = container.node.as_type();

            let mut all_pairs = container_pairs;
            let mut moved = Vec::with_capacity(items.len());
            for it in items {
                let mut pairs = Vec::new();
                let root = support::emit_subtree(
                    batched,
                    &it.subtree,
                    &child_parents,
                    Some(container_type),
                    &mut seq_pos,
                    &mut pairs,
                )?
                .ok_or(StepError::NodeNotFound(it.old_parent))?;
                all_pairs.extend(pairs.iter().copied());
                moved.push(MovedNode { root, pairs });
            }
            if !all_pairs.is_empty() {
                batched.apply(EditOp::Alias(AliasOp {
                    pairs: support::compress_alias_pairs(&all_pairs),
                }))?;
            }
            Ok((Some(container_dot), moved))
        }
    }
}

/// `Step::MoveNodesBack`'s apply: the inverse of [`apply_forward`]. Removes
/// `dest`'s moved-in content purely by position — never by a dot minted
/// during the forward apply, which this step never observes — then restores
/// each item at its `(old_parent, old_index)`, ascending by `old_index` so
/// every earlier restore's position stays valid for the ones that follow.
pub(crate) fn apply_backward(
    batched: &mut BatchedState,
    dest: &MoveDest,
    items: &[MovedItem],
) -> Result<(), StepError> {
    match dest {
        MoveDest::Existing { parent, base_index } => {
            if !items.is_empty() {
                let del_ops = {
                    let ps = &batched.projected;
                    let children = support::child_elem_ids(ps, *parent);
                    let end = base_index + items.len();
                    if end > children.len() {
                        return Err(StepError::IndexOutOfBounds {
                            parent: *parent,
                            index: end,
                            len: children.len(),
                        });
                    }
                    let mut dots = Vec::new();
                    for &elem in &children[*base_index..end] {
                        if ps.is_block(elem) {
                            dots.extend(
                                support::subtree_dots(ps, elem)
                                    .ok_or(StepError::NodeNotFound(elem))?,
                            );
                        } else if let Some(d) = elem.as_op_dot() {
                            dots.push(d.dot());
                        }
                    }
                    support::delete_dots_ops(ps, &dots)
                };
                for op in del_ops {
                    batched.apply(op)?;
                }
            }
        }
        MoveDest::Fresh { parent, index, .. } => {
            let del_ops = {
                let ps = &batched.projected;
                let children = support::child_elem_ids(ps, *parent);
                let elem = *children.get(*index).ok_or(StepError::IndexOutOfBounds {
                    parent: *parent,
                    index: *index,
                    len: children.len(),
                })?;
                let dots = if ps.is_block(elem) {
                    support::subtree_dots(ps, elem).ok_or(StepError::NodeNotFound(elem))?
                } else {
                    match elem.as_op_dot() {
                        Some(d) => vec![d.dot()],
                        None => Vec::new(),
                    }
                };
                support::delete_dots_ops(ps, &dots)
            };
            for op in del_ops {
                batched.apply(op)?;
            }
        }
    }

    let mut ordered: Vec<&MovedItem> = items.iter().collect();
    ordered.sort_by_key(|it| it.old_index);

    let mut all_pairs: Vec<(Dot, Dot)> = Vec::new();
    for it in ordered {
        let (pos, parents, host) = {
            let ps = &batched.projected;
            let pos = support::child_seq_insert_pos(
                ps,
                it.old_parent,
                it.old_index,
                it.subtree.node.as_type(),
            )?;
            let parents = support::self_inclusive_parents(ps, it.old_parent)
                .ok_or(StepError::NodeNotFound(it.old_parent))?;
            let host = support::parent_host_type(ps, &parents);
            (pos, parents, host)
        };
        let mut seq_pos = pos;
        let mut pairs = Vec::new();
        support::emit_subtree(
            batched,
            &it.subtree,
            &parents,
            host,
            &mut seq_pos,
            &mut pairs,
        )?;
        all_pairs.extend(pairs);
    }
    if !all_pairs.is_empty() {
        batched.apply(EditOp::Alias(AliasOp {
            pairs: support::compress_alias_pairs(&all_pairs),
        }))?;
    }
    Ok(())
}

/// Precondition for both composite facades: `items` must be pairwise distinct
/// and form an antichain (no item is an ancestor of another) — a duplicate or
/// nested pair would double-alias the same old dot once its ancestor's whole
/// subtree is captured, tripping the overlapping-range reject the alias log
/// enforces (`alias.rs:73`).
pub(crate) fn validate_items_antichain(
    ps: &ProjectedState,
    items: &[Dot],
) -> Result<(), StepError> {
    for (i, &a) in items.iter().enumerate() {
        for &b in &items[i + 1..] {
            if a == b {
                return Err(StepError::DuplicateMoveItem { item: a });
            }
            if ps.ancestor_real_dots(b, false).contains(&a) {
                return Err(StepError::NonAntichainMoveItems {
                    ancestor: a,
                    descendant: b,
                });
            }
            if ps.ancestor_real_dots(a, false).contains(&b) {
                return Err(StepError::NonAntichainMoveItems {
                    ancestor: b,
                    descendant: a,
                });
            }
        }
    }
    Ok(())
}

/// `dest` must lie outside every item's own subtree (self included) — moving
/// a forest into one of its own members isn't a coherent destination.
pub(crate) fn validate_dest_outside_forest(
    ps: &ProjectedState,
    items: &[Dot],
    dest: Dot,
) -> Result<(), StepError> {
    for &item in items {
        let dots = support::subtree_dots(ps, item).ok_or(StepError::NodeNotFound(item))?;
        if dots.contains(&dest) {
            return Err(StepError::MoveDestinationInsideForest { item, dest });
        }
    }
    Ok(())
}

/// A fresh composite container must itself be a real, addressable block or
/// atom root (never `Text`, `Unknown`, or `Root`) — `emit_subtree` mints it a
/// dot the grafted items are then addressed beneath.
pub(crate) fn validate_container_shape(container: &Subtree) -> Result<(), StepError> {
    if container.node == PlainNode::Unknown {
        return Err(StepError::UnknownSubtree);
    }
    let t = container.node.as_type();
    if t == NodeType::Root {
        return Err(StepError::RootSubtree);
    }
    match classify(t) {
        SeqClass::Block | SeqClass::Atom => Ok(()),
        SeqClass::Text => Err(StepError::InvalidMoveContainer { node_type: t }),
    }
}
