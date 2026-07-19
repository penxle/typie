use editor_crdt::sequence::{
    Bias, Boundary, BoundaryResolver, SeqCheckout, checkout_with_resolver,
};
use editor_crdt::{Dot, OpLog};
use editor_macros::ffi;
use editor_model::{ChildView, DocView, NodeType, NodeView, SeqItem};
use serde::{Deserialize, Serialize};

use crate::Position;
use crate::affinity::Affinity;
use crate::bind::Bind;
use crate::selection::Selection;
use crate::stable_selection::StableSelection;
use crate::state::State;

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StablePositionChild {
    pub dot: Dot,
    pub bind: Bind,
}

/// A step in a `StablePosition`'s ancestor chain. A real ancestor stores its
/// authored dot. A projection-synthesized scaffold cannot: its id is a hash that
/// folds in the content it wraps, so it is reissued the moment the document
/// reprojects and a stored id would strand the anchor. A synthetic step instead
/// stores what it can be rediscovered by — the first real dot it owns, its role,
/// and its depth in the chain — so resolution re-anchors through the owner.
#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChainSegment {
    Real {
        dot: Dot,
    },
    Synthetic {
        owner: Dot,
        role: NodeType,
        depth: u32,
    },
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StablePosition {
    pub chain: Vec<ChainSegment>,
    pub child: Option<StablePositionChild>,
    pub affinity: Affinity,
}

fn child_elem_id(child: &ChildView) -> Dot {
    match child {
        ChildView::Leaf(l) => l.dot(),
        ChildView::Block(b) => b.id(),
    }
}

/// The chain step for `node` at `depth`. Root and authored ancestors keep their
/// dot; a projection-synthesized scaffold records the real content it owns so a
/// reprojected replica can rediscover it. Root is synthetic-bit-set but a stable
/// canonical anchor, so it stays a `Real` step.
fn chain_segment(node: &NodeView, depth: u32) -> ChainSegment {
    let id = node.id();
    if id == Dot::ROOT || !id.is_synthetic() {
        ChainSegment::Real { dot: id }
    } else {
        ChainSegment::Synthetic {
            owner: first_real_descendant(node).unwrap_or(id),
            role: node.node_type(),
            depth,
        }
    }
}

/// The first authored (non-synthetic) dot in `node`'s preorder subtree — the
/// stable content a synthetic scaffold wraps, mirroring the scaffold's own
/// `wrap_cause`. `None` for an empty filler scaffold that owns no real content.
fn first_real_descendant(node: &NodeView) -> Option<Dot> {
    node.descendants().find_map(|c| {
        let dot = child_elem_id(&c);
        (!dot.is_synthetic()).then_some(dot)
    })
}

impl StablePosition {
    pub fn capture(pos: &Position, view: &DocView) -> StablePosition {
        Self::capture_with_child_binding(pos, view, |child_count| {
            if child_count == 0 || pos.offset == 0 {
                None
            } else if pos.affinity == Affinity::Downstream && pos.offset < child_count {
                Some((pos.offset, Bind::Left))
            } else {
                Some((pos.offset - 1, Bind::Right))
            }
        })
    }

    /// Captures the lower endpoint of a non-collapsed range. Text inserted at
    /// this boundary stays outside the range, so offset 0 can bind to the first
    /// child when the container is non-empty.
    pub(crate) fn capture_range_start(pos: &Position, view: &DocView) -> StablePosition {
        Self::capture_with_child_binding(pos, view, |child_count| {
            if child_count == 0 {
                None
            } else if pos.offset < child_count {
                Some((pos.offset, Bind::Left))
            } else {
                Some((pos.offset - 1, Bind::Right))
            }
        })
    }

    /// Captures the upper endpoint of a non-collapsed range. Text inserted at
    /// this boundary stays outside the range by binding to the preceding child.
    pub(crate) fn capture_range_end(pos: &Position, view: &DocView) -> StablePosition {
        Self::capture_with_child_binding(pos, view, |child_count| {
            if child_count == 0 || pos.offset == 0 {
                None
            } else {
                Some((pos.offset - 1, Bind::Right))
            }
        })
    }

    fn capture_with_child_binding(
        pos: &Position,
        view: &DocView,
        child_binding: impl FnOnce(usize) -> Option<(usize, Bind)>,
    ) -> StablePosition {
        let host = view
            .node(pos.node)
            .expect("StablePosition::capture: position node must be a live block");
        let mut nodes: Vec<NodeView> = host.ancestors().collect();
        nodes.reverse();
        let chain: Vec<ChainSegment> = nodes
            .iter()
            .enumerate()
            .map(|(depth, n)| chain_segment(n, depth as u32))
            .collect();
        // O(log) child lookups instead of collecting every child of the host block —
        // this runs on the per-keystroke selection-capture path, so a linear scan makes
        // it `O(block)` inside a large paragraph.
        let child_count = host.child_count();
        let child = child_binding(child_count).map(|(offset, bind)| StablePositionChild {
            dot: child_elem_id(
                &host
                    .child_at(offset)
                    .expect("child binding offset must be live"),
            ),
            bind,
        });
        StablePosition {
            chain,
            child,
            affinity: pos.affinity,
        }
    }
}

/// How a `StableResolveCtx` looks up a dot's sequence position. The `Live` variant
/// borrows the projected state's already-materialized checkout, so restoring a
/// selection after a remote edit costs `O(anchors · log N)` tree lookups instead of
/// a fresh `O(N)` whole-sequence checkout + rank map on every changeset.
enum StableResolver<'a> {
    Owned(Box<BoundaryResolver>),
    Live(&'a SeqCheckout),
}

impl StableResolver<'_> {
    fn boundary(&self, d: Dot) -> Option<Boundary> {
        match self {
            StableResolver::Owned(r) => r.resolve_boundary(d, Bias::Before),
            StableResolver::Live(c) => c.resolve_boundary(d, Bias::Before),
        }
    }

    /// Sequence position for any dot, visible or deleted — the ordering key used
    /// for a target anchor whose element may have been concurrently removed.
    fn position(&self, d: Dot) -> Option<usize> {
        self.boundary(d).map(|b| b.position)
    }

    /// Position of a dot only when it is still visible; `None` for tombstones.
    /// This mirrors the old visible-only `rank` map used to order a node's live
    /// children.
    fn visible_position(&self, d: Dot) -> Option<usize> {
        self.boundary(d).filter(|b| b.visible).map(|b| b.position)
    }
}

pub struct StableResolveCtx<'a> {
    view: &'a DocView<'a>,
    resolver: StableResolver<'a>,
}

impl<'a> StableResolveCtx<'a> {
    pub fn new(view: &'a DocView<'a>, seq: &OpLog<SeqItem>) -> StableResolveCtx<'a> {
        let (_elems, resolver) = checkout_with_resolver(seq);
        StableResolveCtx {
            view,
            resolver: StableResolver::Owned(Box::new(resolver)),
        }
    }

    /// Build a resolve context over the projected state's live sequence checkout,
    /// avoiding the `O(N)` rebuild that `new` pays when it only has the raw oplog.
    pub fn from_live(view: &'a DocView<'a>, seq: &'a SeqCheckout) -> StableResolveCtx<'a> {
        StableResolveCtx {
            view,
            resolver: StableResolver::Live(seq),
        }
    }

    fn alias(&self, d: Dot) -> Dot {
        self.view.alias_classes().resolve_with(d, |m| {
            self.view.node(m).is_some() || self.view.block_of(m).is_some()
        })
    }
}

fn index_of(host: &NodeView, dot: Dot) -> Option<usize> {
    host.children().position(|c| child_elem_id(&c) == dot)
}

fn direct_child_containing(host: &NodeView, target: Dot, ctx: &StableResolveCtx) -> Option<usize> {
    let host_id = host.id();
    let mut cursor = if ctx.view.node(target).is_some() {
        Some(target)
    } else {
        ctx.view.block_of(target)
    };

    while let Some(id) = cursor {
        let parent = ctx.view.parent_of(id)?;
        if parent == host_id {
            return index_of(host, id);
        }
        cursor = Some(parent);
    }
    None
}

fn is_inline_dot(dot: Dot, ctx: &StableResolveCtx) -> bool {
    let dot = ctx.alias(dot);
    ctx.view
        .leaf(dot)
        .is_some_and(|leaf| leaf.node_type().spec().inline)
}

fn resolves_via_child_parent(host: &NodeView, dot: Dot, ctx: &StableResolveCtx) -> bool {
    is_inline_dot(dot, ctx) || host.spec().is_textblock()
}

fn offset_within(c: &NodeView, target: Dot, ctx: &StableResolveCtx) -> usize {
    let Some(op) = target.as_op_dot() else {
        return 0;
    };
    let d = op.dot();
    let Some(r) = ctx.resolver.position(d) else {
        return 0;
    };
    let mut offset = 0usize;
    let mut prev_real: Option<usize> = None;
    for child in c.children() {
        let key = match &child {
            ChildView::Leaf(l) => {
                let k = ctx.resolver.visible_position(l.dot());
                if k.is_some() {
                    prev_real = k;
                }
                k
            }
            ChildView::Block(b) => match b.dot() {
                Some(d) => {
                    let k = ctx.resolver.visible_position(d);
                    if k.is_some() {
                        prev_real = k;
                    }
                    k
                }
                None => prev_real,
            },
        };
        if key.is_none_or(|k| k < r) {
            offset += 1;
        }
    }
    offset
}

/// Resolves a chain step to a live node in the current projection. A real step
/// resolves by its aliased dot. A synthetic step re-anchors: it walks up from the
/// live node that now holds the owner's real content until it reaches an ancestor
/// of the recorded role — the rediscovered scaffold — so a rehashed id never
/// strands the anchor.
fn resolve_segment<'a>(seg: &ChainSegment, ctx: &'a StableResolveCtx<'a>) -> Option<NodeView<'a>> {
    match seg {
        ChainSegment::Real { dot } => ctx.view.node(ctx.alias(*dot)),
        ChainSegment::Synthetic { owner, role, .. } => {
            let owner = ctx.alias(*owner);
            let mut cur = if ctx.view.node(owner).is_some() {
                Some(owner)
            } else {
                ctx.view.block_of(owner)
            };
            while let Some(id) = cur {
                let node = ctx.view.node(id)?;
                if node.node_type() == *role {
                    return Some(node);
                }
                cur = ctx.view.parent_of(id);
            }
            None
        }
    }
}

/// The dot a chain step is positioned by when it sits just below a resolved host:
/// a real step's own dot, or a synthetic step's owner (a real dot), so the offset
/// is computed against live content instead of collapsing to 0 on a synthetic id.
fn segment_target_dot(seg: &ChainSegment) -> Dot {
    match seg {
        ChainSegment::Real { dot } => *dot,
        ChainSegment::Synthetic { owner, .. } => *owner,
    }
}

impl StablePosition {
    pub fn resolve(&self, ctx: &StableResolveCtx) -> Option<Position> {
        if let Some(child) = &self.child
            && is_inline_dot(child.dot, ctx)
            && let Some(pos) = self.resolve_child_parent_boundary(ctx, child.dot, child.bind)
        {
            return Some(pos);
        }

        let (host, next_child) = self.resolve_chain_host(ctx)?;
        if next_child.is_none()
            && let Some(child) = &self.child
            && host.spec().is_textblock()
            && let Some(pos) = self.resolve_child_parent_boundary(ctx, child.dot, child.bind)
        {
            return Some(pos);
        }
        Some(self.resolve_in_host(ctx, host, next_child))
    }

    fn resolve_child_parent_boundary(
        &self,
        ctx: &StableResolveCtx,
        dot: Dot,
        bind: Bind,
    ) -> Option<Position> {
        let dot = ctx.alias(dot);
        let host_dot = if ctx.view.node(dot).is_some() {
            ctx.view.parent_of(dot)
        } else {
            ctx.view.block_of(dot)
        };
        let host = ctx.view.node(host_dot?)?;
        let offset = index_of(&host, dot)? + usize::from(bind == Bind::Right);
        Some(Position {
            node: host.id(),
            offset,
            affinity: self.affinity,
        })
    }

    fn resolve_chain_host<'a>(
        &self,
        ctx: &'a StableResolveCtx<'a>,
    ) -> Option<(NodeView<'a>, Option<Dot>)> {
        // The deepest chain step that still resolves to a live node — real steps
        // by their (aliased) dot, synthetic steps by re-anchoring through the real
        // content they own. Taking the deepest match (not the longest live prefix)
        // recovers a reparented or rehashed-scaffold host even when an intermediate
        // step died, replacing the old first-mismatch truncation and its offset-0
        // collapse with an owner re-anchor.
        let mut host: Option<(usize, NodeView<'a>)> = None;
        for (i, seg) in self.chain.iter().enumerate() {
            if let Some(n) = resolve_segment(seg, ctx) {
                host = Some((i, n));
            }
        }
        let (k, host) = host?;
        let next_child = self
            .chain
            .get(k + 1)
            .map(|seg| ctx.alias(segment_target_dot(seg)));
        Some((host, next_child))
    }

    fn resolve_in_host(
        &self,
        ctx: &StableResolveCtx,
        host: NodeView<'_>,
        next_child: Option<Dot>,
    ) -> Position {
        let offset = if let Some(next_child) = next_child {
            offset_within(&host, next_child, ctx)
        } else {
            match &self.child {
                None => 0,
                Some(StablePositionChild { dot, bind }) => {
                    let aliased = ctx.alias(*dot);
                    match index_of(&host, aliased).or_else(|| {
                        (!resolves_via_child_parent(&host, aliased, ctx))
                            .then(|| direct_child_containing(&host, aliased, ctx))
                            .flatten()
                    }) {
                        Some(j) => j + usize::from(*bind == Bind::Right),
                        None => offset_within(&host, aliased, ctx),
                    }
                }
            }
        };
        Position {
            node: host.id(),
            offset,
            affinity: self.affinity,
        }
    }
}

// ── Migration-only v1 anchor resolution ─────────────────────────────────────
//
// Production never reads v1: every client is force-updated and server-stored v1
// is cleared by a one-shot migration. But that migration must resolve each stored
// v1 anchor exactly as the shipping v1 reader would — including its offset-0
// collapse — before re-capturing it as v2, so a degraded row lands where the old
// client put it. This is a faithful copy of the v1 resolve, wired to report
// whether it fell back to that collapse.

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StablePositionV1 {
    pub chain: Vec<Dot>,
    pub child: Option<StablePositionChild>,
    pub affinity: Affinity,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StableSelectionV1 {
    pub anchor: StablePositionV1,
    pub head: StablePositionV1,
}

impl StablePositionV1 {
    fn resolve(&self, ctx: &StableResolveCtx, degraded: &mut bool) -> Option<Position> {
        if let Some(child) = &self.child
            && is_inline_dot(child.dot, ctx)
            && let Some(pos) = v1_child_parent_boundary(ctx, child.dot, child.bind, self.affinity)
        {
            return Some(pos);
        }

        let (host, next_child) = self.resolve_chain_host(ctx)?;
        if next_child.is_none()
            && let Some(child) = &self.child
            && host.spec().is_textblock()
            && let Some(pos) = v1_child_parent_boundary(ctx, child.dot, child.bind, self.affinity)
        {
            return Some(pos);
        }
        Some(self.resolve_in_host(ctx, host, next_child, degraded))
    }

    fn resolve_chain_host<'a>(
        &self,
        ctx: &'a StableResolveCtx<'a>,
    ) -> Option<(NodeView<'a>, Option<Dot>)> {
        if self.chain.is_empty() {
            return None;
        }
        let mut k = 0usize;
        let mut found = false;
        for (i, id) in self.chain.iter().enumerate() {
            if ctx.view.node(ctx.alias(*id)).is_some() {
                k = i;
                found = true;
            } else {
                break;
            }
        }
        if !found {
            return None;
        }
        let host = ctx.view.node(ctx.alias(self.chain[k])).unwrap();
        let next_child = (k < self.chain.len() - 1).then(|| ctx.alias(self.chain[k + 1]));
        Some((host, next_child))
    }

    fn resolve_in_host(
        &self,
        ctx: &StableResolveCtx,
        host: NodeView<'_>,
        next_child: Option<Dot>,
        degraded: &mut bool,
    ) -> Position {
        let offset = if let Some(next_child) = next_child {
            v1_offset_within(&host, next_child, ctx, degraded)
        } else {
            match &self.child {
                None => 0,
                Some(StablePositionChild { dot, bind }) => {
                    let aliased = ctx.alias(*dot);
                    match index_of(&host, aliased).or_else(|| {
                        (!resolves_via_child_parent(&host, aliased, ctx))
                            .then(|| direct_child_containing(&host, aliased, ctx))
                            .flatten()
                    }) {
                        Some(j) => j + usize::from(*bind == Bind::Right),
                        None => v1_offset_within(&host, aliased, ctx, degraded),
                    }
                }
            }
        };
        Position {
            node: host.id(),
            offset,
            affinity: self.affinity,
        }
    }
}

fn v1_child_parent_boundary(
    ctx: &StableResolveCtx,
    dot: Dot,
    bind: Bind,
    affinity: Affinity,
) -> Option<Position> {
    let dot = ctx.alias(dot);
    let host_dot = if ctx.view.node(dot).is_some() {
        ctx.view.parent_of(dot)
    } else {
        ctx.view.block_of(dot)
    };
    let host = ctx.view.node(host_dot?)?;
    let offset = index_of(&host, dot)? + usize::from(bind == Bind::Right);
    Some(Position {
        node: host.id(),
        offset,
        affinity,
    })
}

fn v1_offset_within(
    c: &NodeView,
    target: Dot,
    ctx: &StableResolveCtx,
    degraded: &mut bool,
) -> usize {
    let Some(op) = target.as_op_dot() else {
        *degraded = true;
        return 0;
    };
    let d = op.dot();
    let Some(r) = ctx.resolver.position(d) else {
        *degraded = true;
        return 0;
    };
    let mut offset = 0usize;
    let mut prev_real: Option<usize> = None;
    for child in c.children() {
        let key = match &child {
            ChildView::Leaf(l) => {
                let k = ctx.resolver.visible_position(l.dot());
                if k.is_some() {
                    prev_real = k;
                }
                k
            }
            ChildView::Block(b) => match b.dot() {
                Some(d) => {
                    let k = ctx.resolver.visible_position(d);
                    if k.is_some() {
                        prev_real = k;
                    }
                    k
                }
                None => prev_real,
            },
        };
        if key.is_none_or(|k| k < r) {
            offset += 1;
        }
    }
    offset
}

/// Resolves a normalized v1 selection against the current projection exactly as
/// the shipping v1 reader would, then re-captures it as a v2 [`StableSelection`].
/// Returns the v2 selection and whether resolution degraded (fell back to the
/// offset-0 collapse on either endpoint). `Err` means the anchor could not be
/// located at all — the migration must surface, not silently drop, such a row.
pub fn resolve_v1_selection(
    state: &State,
    v1: &StableSelectionV1,
) -> Result<(StableSelection, bool), String> {
    let view = state.view();
    let ctx = StableResolveCtx::from_live(&view, state.projected.seq_checkout());
    let mut degraded = false;
    let anchor = v1
        .anchor
        .resolve(&ctx, &mut degraded)
        .ok_or_else(|| "v1 anchor did not resolve against the graph".to_string())?;
    let head = v1
        .head
        .resolve(&ctx, &mut degraded)
        .ok_or_else(|| "v1 head did not resolve against the graph".to_string())?;
    let v2 = StableSelection::capture(&Selection { anchor, head }, &view);
    Ok((v2, degraded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::Dot;
    use editor_crdt::{InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, AliasOp, AliasRun, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType,
        ProjectedDoc, SeqItem, SpanLog, project_document,
    };

    use crate::Position;

    fn doclogs(ev: &[InputEvent<SeqItem>]) -> DocLogs {
        DocLogs {
            seq: build_oplog(ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
            aliases: AliasLog::new(),
        }
    }

    fn block(node_type: NodeType, parents: Vec<Dot>) -> SeqItem {
        SeqItem::Block {
            node_type,
            parents,
            attrs: vec![],
        }
    }

    fn ins_only(items: &[(Dot, SeqItem)]) -> Vec<InputEvent<SeqItem>> {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        ev
    }

    fn para_with(leaves: &[SeqItem]) -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let mut items = vec![(para, block(NodeType::Paragraph, vec![root]))];
        for (i, l) in leaves.iter().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), l.clone()));
        }
        (project_document(&doclogs(&ins_only(&items))).unwrap(), para)
    }

    #[test]
    fn capture_empty_block_is_container_start() {
        let (pd, para) = para_with(&[]);
        let view = DocView::new(&pd);
        let sp = StablePosition::capture(&Position::new(para, 0), &view);
        assert!(sp.child.is_none());
        assert_eq!(sp.chain.last(), Some(&ChainSegment::Real { dot: para }));
        assert_eq!(
            sp.chain.first(),
            Some(&ChainSegment::Real { dot: Dot::ROOT })
        );
    }

    #[test]
    fn capture_offset_zero_is_container_start() {
        let (pd, para) = para_with(&[SeqItem::Char('a'), SeqItem::Char('b')]);
        let view = DocView::new(&pd);
        let sp = StablePosition::capture(&Position::new(para, 0), &view);
        assert!(sp.child.is_none());
    }

    #[test]
    fn capture_downstream_interior_binds_following_left() {
        let (pd, para) = para_with(&[SeqItem::Char('a'), SeqItem::Char('b'), SeqItem::Char('c')]);
        let view = DocView::new(&pd);
        let pos = Position {
            node: para,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let sp = StablePosition::capture(&pos, &view);
        assert_eq!(
            sp.child,
            Some(StablePositionChild {
                dot: Dot::new(1, 3),
                bind: Bind::Left,
            })
        );
    }

    #[test]
    fn capture_upstream_interior_binds_preceding_right() {
        let (pd, para) = para_with(&[SeqItem::Char('a'), SeqItem::Char('b'), SeqItem::Char('c')]);
        let view = DocView::new(&pd);
        let pos = Position {
            node: para,
            offset: 2,
            affinity: Affinity::Upstream,
        };
        let sp = StablePosition::capture(&pos, &view);
        assert_eq!(
            sp.child,
            Some(StablePositionChild {
                dot: Dot::new(1, 3),
                bind: Bind::Right,
            })
        );
    }

    #[test]
    fn capture_end_boundary_binds_last_right() {
        let (pd, para) = para_with(&[SeqItem::Char('a'), SeqItem::Char('b')]);
        let view = DocView::new(&pd);
        let pos = Position {
            node: para,
            offset: 2,
            affinity: Affinity::Downstream,
        };
        let sp = StablePosition::capture(&pos, &view);
        assert_eq!(
            sp.child,
            Some(StablePositionChild {
                dot: Dot::new(1, 3),
                bind: Bind::Right,
            })
        );
    }

    #[test]
    fn types_construct_and_compare() {
        let sp = StablePosition {
            chain: vec![
                ChainSegment::Real { dot: Dot::ROOT },
                ChainSegment::Real {
                    dot: Dot::new(1, 1),
                },
            ],
            child: Some(StablePositionChild {
                dot: Dot::new(1, 2),
                bind: Bind::Left,
            }),
            affinity: Affinity::Downstream,
        };
        assert_eq!(sp.clone(), sp);
        assert!(sp.child.is_some());
        let cs = StablePosition {
            chain: vec![ChainSegment::Real { dot: Dot::ROOT }],
            child: None,
            affinity: Affinity::Upstream,
        };
        assert_ne!(sp, cs);
    }

    #[test]
    fn old_binding_field_is_rejected() {
        let value = serde_json::json!({
            "chain": [Dot::ROOT],
            "binding": { "type": "container_start" },
            "affinity": Affinity::Downstream,
        });
        assert!(serde_json::from_value::<StablePosition>(value).is_err());
    }

    #[test]
    fn live_roundtrip_char_both_affinities() {
        let (pd, para) = para_with(&[SeqItem::Char('a'), SeqItem::Char('b'), SeqItem::Char('c')]);
        let view = DocView::new(&pd);
        let logs = doclogs(&ins_only(&[
            (Dot::new(1, 1), block(NodeType::Paragraph, vec![Dot::ROOT])),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Char('c')),
        ]));
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        for affinity in [Affinity::Downstream, Affinity::Upstream] {
            for offset in 0..=3 {
                let pos = Position {
                    node: para,
                    offset,
                    affinity,
                };
                let sp = StablePosition::capture(&pos, &view);
                assert_eq!(
                    sp.resolve(&ctx),
                    Some(pos),
                    "offset {offset} aff {affinity:?}"
                );
            }
        }
    }

    #[test]
    fn live_roundtrip_empty_block() {
        let (pd, para) = para_with(&[]);
        let view = DocView::new(&pd);
        let logs = doclogs(&ins_only(&[(
            Dot::new(1, 1),
            block(NodeType::Paragraph, vec![Dot::ROOT]),
        )]));
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        let pos = Position::new(para, 0);
        let sp = StablePosition::capture(&pos, &view);
        assert!(sp.child.is_none());
        assert_eq!(sp.resolve(&ctx), Some(pos));
    }

    #[test]
    fn live_roundtrip_atom_boundary() {
        use editor_model::AtomLeaf;
        let (pd, para) = para_with(&[
            SeqItem::Char('a'),
            SeqItem::Atom(AtomLeaf::HardBreak),
            SeqItem::Char('b'),
        ]);
        let view = DocView::new(&pd);
        let logs = doclogs(&ins_only(&[
            (Dot::new(1, 1), block(NodeType::Paragraph, vec![Dot::ROOT])),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Atom(AtomLeaf::HardBreak)),
            (Dot::new(1, 4), SeqItem::Char('b')),
        ]));
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        for offset in 0..=3 {
            let pos = Position::new(para, offset);
            let sp = StablePosition::capture(&pos, &view);
            assert_eq!(sp.resolve(&ctx), Some(pos), "offset {offset}");
        }
    }

    fn root_two_paras() -> (ProjectedDoc, DocLogs) {
        let items = vec![
            (Dot::new(1, 1), block(NodeType::Paragraph, vec![Dot::ROOT])),
            (Dot::new(1, 2), SeqItem::Char('x')),
            (Dot::new(1, 3), block(NodeType::Paragraph, vec![Dot::ROOT])),
            (Dot::new(1, 4), SeqItem::Char('y')),
        ];
        let logs = doclogs(&ins_only(&items));
        (project_document(&logs).unwrap(), logs)
    }

    #[test]
    fn live_roundtrip_block_container_host() {
        let (pd, logs) = root_two_paras();
        let view = DocView::new(&pd);
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        let root = view.root().unwrap().id();
        for offset in 0..=2 {
            let pos = Position::new(root, offset);
            let sp = StablePosition::capture(&pos, &view);
            assert_eq!(sp.resolve(&ctx), Some(pos), "root offset {offset}");
        }
    }

    #[test]
    fn block_container_boundary_stays_in_captured_host_when_anchor_moves_inside_wrapper() {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 2);
        let pre_items = vec![
            (p1, block(NodeType::Paragraph, vec![root])),
            (p2, block(NodeType::Paragraph, vec![root])),
        ];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let pre_view = DocView::new(&pre);
        let root_id = pre_view.root().unwrap().id();
        let captured = StablePosition::capture(
            &Position {
                node: root_id,
                offset: 1,
                affinity: Affinity::Upstream,
            },
            &pre_view,
        );

        let list = Dot::new(2, 1);
        let item = Dot::new(2, 2);
        let moved_p1 = Dot::new(2, 3);
        let post_items = vec![
            (list, block(NodeType::BulletList, vec![root])),
            (item, block(NodeType::ListItem, vec![root, list])),
            (moved_p1, block(NodeType::Paragraph, vec![root, list, item])),
            (p2, block(NodeType::Paragraph, vec![root])),
        ];
        let mut post_logs = doclogs(&ins_only(&post_items));
        post_logs.aliases.apply(AliasOp {
            pairs: vec![AliasRun {
                old_start: p1,
                len: 1,
                new_start: moved_p1,
            }],
        });
        let post = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);

        let resolved = captured.resolve(&ctx).unwrap();
        assert_eq!(resolved.node, post_view.root().unwrap().id());
        assert_eq!(resolved.offset, 1);
    }

    fn pre_post_delete_b() -> (ProjectedDoc, ProjectedDoc, DocLogs, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let pre_items = vec![
            (para, block(NodeType::Paragraph, vec![root])),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Char('c')),
        ];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let mut post_ev = ins_only(&pre_items);
        post_ev.push(InputEvent {
            id: Dot::new(1, 5),
            parents: vec![Dot::new(1, 4)],
            op: ListOp::Del { pos: 2, len: 1 },
        });
        let post_logs = doclogs(&post_ev);
        let post = project_document(&post_logs).unwrap();
        (pre, post, post_logs, para)
    }

    #[test]
    fn deleted_mid_char_resolves_within_block_bind_independent() {
        let (pre, post, post_logs, para) = pre_post_delete_b();
        let pre_view = DocView::new(&pre);
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);
        let down = StablePosition::capture(
            &Position {
                node: para,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            &pre_view,
        );
        let up = StablePosition::capture(
            &Position {
                node: para,
                offset: 2,
                affinity: Affinity::Upstream,
            },
            &pre_view,
        );
        assert_eq!(down.resolve(&ctx).unwrap().offset, 1);
        assert_eq!(up.resolve(&ctx).unwrap().offset, 1);
        assert_eq!(down.resolve(&ctx).unwrap().node, para);
    }

    #[test]
    fn deleted_first_char_resolves_to_zero() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let pre_items = vec![
            (para, block(NodeType::Paragraph, vec![root])),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
        ];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let pre_view = DocView::new(&pre);
        let sp = StablePosition::capture(
            &Position {
                node: para,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            &pre_view,
        );
        let mut post_ev = ins_only(&pre_items);
        post_ev.push(InputEvent {
            id: Dot::new(1, 4),
            parents: vec![Dot::new(1, 3)],
            op: ListOp::Del { pos: 1, len: 1 },
        });
        let post_logs = doclogs(&post_ev);
        let post = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);
        assert_eq!(sp.resolve(&ctx).unwrap().offset, 0);
    }

    #[test]
    fn deleted_last_char_resolves_to_len() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let pre_items = vec![
            (para, block(NodeType::Paragraph, vec![root])),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
        ];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let pre_view = DocView::new(&pre);
        let sp = StablePosition::capture(
            &Position {
                node: para,
                offset: 2,
                affinity: Affinity::Upstream,
            },
            &pre_view,
        );
        let mut post_ev = ins_only(&pre_items);
        post_ev.push(InputEvent {
            id: Dot::new(1, 4),
            parents: vec![Dot::new(1, 3)],
            op: ListOp::Del { pos: 2, len: 1 },
        });
        let post_logs = doclogs(&post_ev);
        let post = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);
        assert_eq!(sp.resolve(&ctx).unwrap().offset, 1);
    }

    fn root_blockquote_para() -> (ProjectedDoc, DocLogs) {
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let para = Dot::new(1, 2);
        let items = vec![
            (bq, block(NodeType::Blockquote, vec![root])),
            (para, block(NodeType::Paragraph, vec![root, bq])),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let logs = doclogs(&ins_only(&items));
        (project_document(&logs).unwrap(), logs)
    }

    #[test]
    fn derived_block_host_roundtrips() {
        let (pd, logs) = root_blockquote_para();
        let view = DocView::new(&pd);
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        let derived = view
            .root()
            .unwrap()
            .child_blocks()
            .find(|b| b.id().is_synthetic())
            .map(|b| b.id())
            .expect("normalize synthesizes a derived trailing paragraph");
        let pos = Position::new(derived, 0);
        let sp = StablePosition::capture(&pos, &view);
        assert!(matches!(
            sp.chain.last(),
            Some(ChainSegment::Synthetic {
                role: NodeType::Paragraph,
                ..
            })
        ));
        assert_eq!(sp.resolve(&ctx), Some(pos));
    }

    #[test]
    fn derived_block_as_anchor_roundtrips() {
        let (pd, logs) = root_blockquote_para();
        let view = DocView::new(&pd);
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        let root = view.root().unwrap().id();
        let n = view.root().unwrap().children().count();
        let pos = Position::new(root, n);
        let sp = StablePosition::capture(&pos, &view);
        assert_eq!(sp.resolve(&ctx), Some(pos));
    }

    #[test]
    fn deleted_host_block_walks_to_live_ancestor() {
        let root = Dot::ROOT;
        let p0 = Dot::new(1, 1);
        let p1 = Dot::new(1, 3);
        let p2 = Dot::new(1, 5);
        let pre_items = vec![
            (p0, block(NodeType::Paragraph, vec![root])),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (p1, block(NodeType::Paragraph, vec![root])),
            (Dot::new(1, 4), SeqItem::Char('b')),
            (p2, block(NodeType::Paragraph, vec![root])),
            (Dot::new(1, 6), SeqItem::Char('c')),
        ];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let pre_view = DocView::new(&pre);
        let sp = StablePosition::capture(&Position::new(p1, 0), &pre_view);
        assert_eq!(sp.chain.last(), Some(&ChainSegment::Real { dot: p1 }));
        let mut post_ev = ins_only(&pre_items);
        post_ev.push(InputEvent {
            id: Dot::new(1, 7),
            parents: vec![Dot::new(1, 6)],
            op: ListOp::Del { pos: 2, len: 2 },
        });
        let post_logs = doclogs(&post_ev);
        let post = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);
        let r = sp.resolve(&ctx).unwrap();
        let root_id = post_view.root().unwrap().id();
        assert_eq!(r.node, root_id);
        assert_eq!(r.offset, 1);
    }

    #[test]
    fn unknown_anchor_resolves_to_host_start() {
        let (pd, para) = para_with(&[SeqItem::Char('a')]);
        let view = DocView::new(&pd);
        let logs = doclogs(&ins_only(&[
            (Dot::new(1, 1), block(NodeType::Paragraph, vec![Dot::ROOT])),
            (Dot::new(1, 2), SeqItem::Char('a')),
        ]));
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        let sp = StablePosition {
            chain: vec![
                ChainSegment::Real {
                    dot: view.root().unwrap().id(),
                },
                ChainSegment::Real { dot: para },
            ],
            child: Some(StablePositionChild {
                dot: Dot::new(9, 9),
                bind: Bind::Left,
            }),
            affinity: Affinity::Downstream,
        };
        let r = sp.resolve(&ctx).unwrap();
        assert_eq!(r.node, para);
        assert_eq!(r.offset, 0);
    }

    #[test]
    fn reordered_fold_host_is_in_range_no_panic() {
        let root = Dot::ROOT;
        let fold = Dot::new(1, 1);
        let content = Dot::new(1, 2);
        let title = Dot::new(1, 3);
        let pre_items = vec![
            (fold, block(NodeType::Fold, vec![root])),
            (content, block(NodeType::FoldContent, vec![root, fold])),
            (title, block(NodeType::FoldTitle, vec![root, fold])),
        ];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let pre_view = DocView::new(&pre);
        let fold_view = pre_view.node(fold).unwrap();
        let n = fold_view.children().count();
        let sp = StablePosition::capture(&Position::new(fold, n), &pre_view);
        let mut post_ev = ins_only(&pre_items);
        post_ev.push(InputEvent {
            id: Dot::new(1, 4),
            parents: vec![Dot::new(1, 3)],
            op: ListOp::Del { pos: 1, len: 1 },
        });
        let post_logs = doclogs(&post_ev);
        let post = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);
        let r = sp.resolve(&ctx).unwrap();
        if let Some(host) = post_view.node(r.node) {
            assert!(r.offset <= host.children().count(), "offset in range");
        } else {
            panic!("resolved host must exist");
        }
    }

    #[test]
    fn non_reordered_block_container_deleted_sibling_exact() {
        let root = Dot::ROOT;
        let p0 = Dot::new(1, 1);
        let p1 = Dot::new(1, 2);
        let p2 = Dot::new(1, 3);
        let pre_items = vec![
            (p0, block(NodeType::Paragraph, vec![root])),
            (p1, block(NodeType::Paragraph, vec![root])),
            (p2, block(NodeType::Paragraph, vec![root])),
        ];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let pre_view = DocView::new(&pre);
        let root_id = pre_view.root().unwrap().id();
        let sp = StablePosition::capture(
            &Position {
                node: root_id,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            &pre_view,
        );
        let mut post_ev = ins_only(&pre_items);
        post_ev.push(InputEvent {
            id: Dot::new(1, 4),
            parents: vec![Dot::new(1, 3)],
            op: ListOp::Del { pos: 1, len: 1 },
        });
        let post_logs = doclogs(&post_ev);
        let post = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);
        let r = sp.resolve(&ctx).unwrap();
        assert_eq!(r.node, post_view.root().unwrap().id());
        assert_eq!(r.offset, 1);
    }

    #[test]
    fn normalization_hoisted_anchor_resolves_in_range() {
        use editor_model::AtomLeaf;
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let para = Dot::new(1, 2);
        let pb = Dot::new(1, 3);
        let items = vec![
            (bq, block(NodeType::Blockquote, vec![root])),
            (para, block(NodeType::Paragraph, vec![root, bq])),
            (pb, SeqItem::Atom(AtomLeaf::PageBreak)),
        ];
        let logs = doclogs(&ins_only(&items));
        let pd = project_document(&logs).unwrap();
        let view = DocView::new(&pd);
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        // Under total projection the context-invalid PageBreak (Blockquote>Paragraph) is
        // SPLIT-HOISTed to a Root Paragraph and preserved — no longer dropped.
        assert!(
            view.leaf(pb).is_some(),
            "the anchored PageBreak survives (hoisted, not dropped)"
        );
        let sp = StablePosition {
            chain: vec![
                ChainSegment::Real {
                    dot: view.root().unwrap().id(),
                },
                ChainSegment::Real { dot: bq },
                ChainSegment::Real { dot: para },
            ],
            child: Some(StablePositionChild {
                dot: pb,
                bind: Bind::Left,
            }),
            affinity: Affinity::Downstream,
        };
        let r = sp.resolve(&ctx).unwrap();
        // The anchor resolves to a live block with an in-range offset (its real,
        // hoisted location rather than a dropped-anchor fallback).
        let target = view.node(r.node).expect("resolves to a live block");
        assert!(r.offset <= target.children().count(), "offset in range");
    }

    #[test]
    fn leading_derived_sibling_offset_in_range() {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let pre_items = vec![
            (p1, block(NodeType::Paragraph, vec![root])),
            (Dot::new(1, 2), SeqItem::Char('a')),
        ];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let pre_view = DocView::new(&pre);
        let root_id = pre_view.root().unwrap().id();
        let sp = StablePosition::capture(
            &Position {
                node: root_id,
                offset: 1,
                affinity: Affinity::Upstream,
            },
            &pre_view,
        );
        let mut post_ev = ins_only(&pre_items);
        post_ev.push(InputEvent {
            id: Dot::new(1, 3),
            parents: vec![Dot::new(1, 2)],
            op: ListOp::Del { pos: 0, len: 2 },
        });
        let post_logs = doclogs(&post_ev);
        let post = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);
        let new_root = post_view.root().unwrap();
        assert!(
            new_root.child_blocks().all(|b| b.id().is_synthetic()),
            "all surviving Root children are derived (leading)"
        );
        let r = sp.resolve(&ctx).unwrap();
        assert_eq!(r.node, new_root.id());
        assert!(r.offset <= new_root.children().count());
    }

    #[test]
    fn undel_restores_original_position() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let b = Dot::new(1, 3);
        let base = vec![
            (para, block(NodeType::Paragraph, vec![root])),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (b, SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Char('c')),
        ];
        let pre = project_document(&doclogs(&ins_only(&base))).unwrap();
        let pre_view = DocView::new(&pre);
        let pos = Position {
            node: para,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let sp = StablePosition::capture(&pos, &pre_view);

        let mut ev = ins_only(&base);
        // `ListOp::Undel { del }` takes the DELETE op's Dot (it looks up del_targets[del_lv]),
        // NOT the inserted element's Dot. Passing `b` would un-delete nothing and the test
        // would pass via the dead-anchor fallback without exercising live-restore.
        let del_op = Dot::new(1, 5);
        ev.push(InputEvent {
            id: del_op,
            parents: vec![Dot::new(1, 4)],
            op: ListOp::Del { pos: 2, len: 1 },
        });
        ev.push(InputEvent {
            id: Dot::new(1, 6),
            parents: vec![del_op],
            op: ListOp::Undel { del: del_op },
        });
        let logs = doclogs(&ev);
        let post = project_document(&logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &logs.seq);
        assert!(post_view.leaf(b).is_some(), "Undel must restore 'b' live");
        assert_eq!(sp.resolve(&ctx), Some(pos));
    }

    #[test]
    fn resolve_follows_alias_after_simulated_move() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let succ = Dot::new(5, 0);
        let pre_items = vec![
            (para, block(NodeType::Paragraph, vec![root])),
            (a, SeqItem::Char('a')),
            (b, SeqItem::Char('b')),
        ];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let pre_view = DocView::new(&pre);
        let pos = Position {
            node: para,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let sp = StablePosition::capture(&pos, &pre_view);
        assert_eq!(
            sp.child,
            Some(StablePositionChild {
                dot: a,
                bind: Bind::Right,
            })
        );

        let mut post_ev = ins_only(&pre_items);
        post_ev.push(InputEvent {
            id: succ,
            parents: vec![b],
            op: ListOp::Ins {
                pos: 3,
                item: SeqItem::Char('a'),
            },
        });
        let del_op = Dot::new(1, 4);
        post_ev.push(InputEvent {
            id: del_op,
            parents: vec![succ],
            op: ListOp::Del { pos: 1, len: 1 },
        });
        let mut post_logs = doclogs(&post_ev);
        post_logs.aliases.apply(AliasOp {
            pairs: vec![AliasRun {
                old_start: a,
                len: 1,
                new_start: succ,
            }],
        });
        let post = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);

        assert!(post_view.leaf(a).is_none(), "'a' was deleted, not restored");
        assert!(post_view.leaf(succ).is_some(), "successor is live");
        let r = sp.resolve(&ctx).unwrap();
        assert_eq!(r.node, para);
        assert_eq!(
            r.offset, 2,
            "must land on succ's own slot, not a's stale tombstone rank"
        );
    }

    #[test]
    fn resolve_after_undo_returns_to_gen1_self() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let gen0 = Dot::new(9, 9);
        let gen1 = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let pre_items = vec![
            (para, block(NodeType::Paragraph, vec![root])),
            (gen1, SeqItem::Char('a')),
            (b, SeqItem::Char('b')),
        ];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let pre_view = DocView::new(&pre);
        let pos = Position {
            node: para,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let sp = StablePosition::capture(&pos, &pre_view);
        assert_eq!(
            sp.child,
            Some(StablePositionChild {
                dot: gen1,
                bind: Bind::Right,
            })
        );

        let mut ev = ins_only(&pre_items);
        let del_op = Dot::new(1, 4);
        ev.push(InputEvent {
            id: del_op,
            parents: vec![b],
            op: ListOp::Del { pos: 1, len: 1 },
        });
        ev.push(InputEvent {
            id: Dot::new(1, 5),
            parents: vec![del_op],
            op: ListOp::Undel { del: del_op },
        });
        let mut logs = doclogs(&ev);
        logs.aliases.apply(AliasOp {
            pairs: vec![AliasRun {
                old_start: gen0,
                len: 1,
                new_start: gen1,
            }],
        });
        let post = project_document(&logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &logs.seq);
        assert!(post_view.leaf(gen1).is_some(), "Undel restores gen1 live");
        assert_eq!(sp.resolve(&ctx), Some(pos));
    }

    #[test]
    fn resolve_gen2_anchor_maps_to_gen1_after_move_undo() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let gen1 = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let gen2 = Dot::new(2, 1);
        let base_items = vec![
            (para, block(NodeType::Paragraph, vec![root])),
            (gen1, SeqItem::Char('a')),
            (b, SeqItem::Char('b')),
            (gen2, SeqItem::Char('a')),
        ];
        let mut mid_ev = ins_only(&base_items);
        let del1 = Dot::new(1, 4);
        mid_ev.push(InputEvent {
            id: del1,
            parents: vec![gen2],
            op: ListOp::Del { pos: 1, len: 1 },
        });
        let mut mid_logs = doclogs(&mid_ev);
        mid_logs.aliases.apply(AliasOp {
            pairs: vec![AliasRun {
                old_start: gen1,
                len: 1,
                new_start: gen2,
            }],
        });
        let mid = project_document(&mid_logs).unwrap();
        let mid_view = DocView::new(&mid);
        let pos = Position {
            node: para,
            offset: 2,
            affinity: Affinity::Upstream,
        };
        let sp = StablePosition::capture(&pos, &mid_view);
        assert_eq!(
            sp.child,
            Some(StablePositionChild {
                dot: gen2,
                bind: Bind::Right,
            })
        );

        let mut post_ev = mid_ev.clone();
        let del2 = Dot::new(2, 2);
        post_ev.push(InputEvent {
            id: del2,
            parents: vec![del1],
            op: ListOp::Del { pos: 2, len: 1 },
        });
        post_ev.push(InputEvent {
            id: Dot::new(1, 5),
            parents: vec![del2],
            op: ListOp::Undel { del: del1 },
        });
        let mut post_logs = doclogs(&post_ev);
        post_logs.aliases.apply(AliasOp {
            pairs: vec![AliasRun {
                old_start: gen1,
                len: 1,
                new_start: gen2,
            }],
        });
        let post = project_document(&post_logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);
        assert!(post_view.leaf(gen1).is_some(), "gen1 restored by move undo");
        assert!(post_view.leaf(gen2).is_none(), "gen2 dead again");

        let r = sp.resolve(&ctx).unwrap();
        assert_eq!(r.node, para);
        assert_eq!(r.offset, 1, "gen2 anchor must map through to live gen1");
    }

    #[test]
    fn resolve_container_start_follows_block_move_alias() {
        let root = Dot::ROOT;
        let empty_para = Dot::new(1, 1);
        let new_para = Dot::new(5, 0);
        let pre_items = vec![(empty_para, block(NodeType::Paragraph, vec![root]))];
        let pre = project_document(&doclogs(&ins_only(&pre_items))).unwrap();
        let pre_view = DocView::new(&pre);
        let pos = Position::new(empty_para, 0);
        let sp = StablePosition::capture(&pos, &pre_view);
        assert!(sp.child.is_none());
        assert_eq!(
            sp.chain.last(),
            Some(&ChainSegment::Real { dot: empty_para })
        );

        let mut ev = ins_only(&pre_items);
        ev.push(InputEvent {
            id: new_para,
            parents: vec![empty_para],
            op: ListOp::Ins {
                pos: 1,
                item: block(NodeType::Paragraph, vec![root]),
            },
        });
        let del_op = Dot::new(1, 2);
        ev.push(InputEvent {
            id: del_op,
            parents: vec![new_para],
            op: ListOp::Del { pos: 0, len: 1 },
        });
        let mut logs = doclogs(&ev);
        logs.aliases.apply(AliasOp {
            pairs: vec![AliasRun {
                old_start: empty_para,
                len: 1,
                new_start: new_para,
            }],
        });
        let post = project_document(&logs).unwrap();
        let post_view = DocView::new(&post);
        let ctx = StableResolveCtx::new(&post_view, &logs.seq);

        assert!(
            post_view.node(empty_para).is_none(),
            "original block is gone"
        );
        let moved = post_view.node(new_para).expect("moved block is live");
        assert_eq!(
            moved.child_count(),
            0,
            "destination is empty, like the source"
        );

        let r = sp.resolve(&ctx).unwrap();
        assert_eq!(r.node, new_para);
        assert_eq!(r.offset, 0);
    }

    #[test]
    fn adjacent_anchor_with_no_parent_falls_back_to_chain() {
        let (pd, _para) = para_with(&[SeqItem::Char('a')]);
        let view = DocView::new(&pd);
        let logs = doclogs(&ins_only(&[
            (Dot::new(1, 1), block(NodeType::Paragraph, vec![Dot::ROOT])),
            (Dot::new(1, 2), SeqItem::Char('a')),
        ]));
        let ctx = StableResolveCtx::new(&view, &logs.seq);
        let root_id = view.root().unwrap().id();
        let sp = StablePosition {
            chain: vec![ChainSegment::Real { dot: root_id }],
            child: Some(StablePositionChild {
                dot: root_id,
                bind: Bind::Left,
            }),
            affinity: Affinity::Downstream,
        };
        let r = sp.resolve(&ctx).unwrap();
        assert_eq!(r.node, root_id);
        assert_eq!(r.offset, 0);
    }

    fn bare_table_cell_state() -> (State, Dot, Dot) {
        use editor_crdt::{Changeset, Op};
        use editor_model::EditOp;
        // A bare TableCell under Root is wrapped back into synthetic Table > TableRow
        // scaffolds (and its inline content into a synthetic Paragraph) to satisfy the
        // schema — a content-owning scaffold chain over a real cell and char.
        let cell = Dot::new(1, 0);
        let a = Dot::new(1, 1);
        let css = vec![Changeset {
            ops: vec![
                Op {
                    id: cell,
                    parents: vec![],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 0,
                        item: SeqItem::Block {
                            node_type: NodeType::TableCell,
                            parents: vec![Dot::ROOT],
                            attrs: vec![],
                        },
                    }),
                },
                Op {
                    id: a,
                    parents: vec![cell],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 1,
                        item: SeqItem::Char('a'),
                    }),
                },
            ],
        }];
        (State::from_changesets(css, None).unwrap(), cell, a)
    }

    #[test]
    fn capture_stores_synthetic_scaffolds_by_the_real_content_they_own() {
        let (state, _cell, a) = bare_table_cell_state();
        let view = state.view();
        let host = view.block_of(a).expect("'a' lives in a block");
        let pos = Position::new(host, 0);
        let sp = StablePosition::capture(&pos, &view);

        // The synthetic Table / TableRow steps store a real owner dot, never their
        // own reprojection-unstable hashed id.
        let table = sp
            .chain
            .iter()
            .find(|s| {
                matches!(
                    s,
                    ChainSegment::Synthetic {
                        role: NodeType::Table,
                        ..
                    }
                )
            })
            .expect("chain crosses a synthetic Table scaffold");
        assert!(matches!(
            table,
            ChainSegment::Synthetic { owner, .. } if !owner.is_synthetic()
        ));
        assert!(sp.chain.iter().any(|s| matches!(
            s,
            ChainSegment::Synthetic {
                role: NodeType::TableRow,
                owner,
                ..
            } if !owner.is_synthetic()
        )));

        let ctx = StableResolveCtx::from_live(&view, state.projected.seq_checkout());
        assert_eq!(sp.resolve(&ctx), Some(pos));
    }

    #[test]
    fn synthetic_segments_resolve_by_owner_ignoring_stored_depth() {
        let (state, _cell, a) = bare_table_cell_state();
        let view = state.view();
        let host = view.block_of(a).expect("'a' lives in a block");
        let sp = StablePosition::capture(&Position::new(host, 0), &view);
        let ctx = StableResolveCtx::from_live(&view, state.projected.seq_checkout());
        let resolved = sp.resolve(&ctx).expect("captured anchor resolves");

        // Rewrite every synthetic step's depth to a bogus value: resolution
        // re-anchors through the owner, so the recorded depth cannot move it.
        let mut mangled = sp.clone();
        for seg in &mut mangled.chain {
            if let ChainSegment::Synthetic { depth, .. } = seg {
                *depth = 9999;
            }
        }
        assert_eq!(mangled.resolve(&ctx), Some(resolved));
    }

    #[test]
    fn resolve_v1_selection_migrates_a_resolvable_anchor_without_degrading() {
        let (state, _cell, a) = bare_table_cell_state();
        let v1 = StableSelectionV1 {
            anchor: StablePositionV1 {
                chain: vec![Dot::ROOT],
                child: Some(StablePositionChild {
                    dot: a,
                    bind: Bind::Right,
                }),
                affinity: Affinity::Upstream,
            },
            head: StablePositionV1 {
                chain: vec![Dot::ROOT],
                child: Some(StablePositionChild {
                    dot: a,
                    bind: Bind::Right,
                }),
                affinity: Affinity::Upstream,
            },
        };
        let (v2, degraded) = resolve_v1_selection(&state, &v1).unwrap();
        assert!(!degraded, "a resolvable v1 anchor must not degrade");
        // The migrated v2 anchor re-encodes the synthetic Table scaffold by owner.
        assert!(v2.anchor.chain.iter().any(|s| matches!(
            s,
            ChainSegment::Synthetic {
                role: NodeType::Table,
                ..
            }
        )));
        let view = state.view();
        let ctx = StableResolveCtx::from_live(&view, state.projected.seq_checkout());
        assert!(v2.resolve(&ctx).expect("v2 resolves").is_collapsed());
    }

    #[test]
    fn resolve_v1_selection_flags_degraded_and_matches_the_v1_offset_zero_fallback() {
        let (state, _cell, _a) = bare_table_cell_state();
        // An anchor whose child dot is nowhere in the sequence: the shipping v1
        // reader collapses it to offset 0 at the host. Migration must reproduce
        // that fallback and mark the row degraded.
        let unknown = Dot::new(9, 9);
        let v1 = StableSelectionV1 {
            anchor: StablePositionV1 {
                chain: vec![Dot::ROOT],
                child: Some(StablePositionChild {
                    dot: unknown,
                    bind: Bind::Left,
                }),
                affinity: Affinity::Downstream,
            },
            head: StablePositionV1 {
                chain: vec![Dot::ROOT],
                child: Some(StablePositionChild {
                    dot: unknown,
                    bind: Bind::Left,
                }),
                affinity: Affinity::Downstream,
            },
        };
        let (v2, degraded) = resolve_v1_selection(&state, &v1).unwrap();
        assert!(
            degraded,
            "an unresolvable child collapses to the offset-0 fallback"
        );

        let view = state.view();
        let ctx = StableResolveCtx::from_live(&view, state.projected.seq_checkout());
        let sel = v2.resolve(&ctx).expect("degraded v2 still resolves");
        assert_eq!(sel.anchor.node, Dot::ROOT);
        assert_eq!(sel.anchor.offset, 0);
    }

    fn arb_para_chars() -> impl proptest::strategy::Strategy<Value = Vec<char>> {
        use proptest::prelude::*;
        proptest::collection::vec(prop::sample::select(vec!['a', 'b', 'c', 'd']), 0..6)
    }

    proptest::proptest! {
        #[test]
        fn live_captures_roundtrip_and_resolve_never_panics(chars in arb_para_chars(), del in 0usize..6) {
            let root = Dot::ROOT;
            let para = Dot::new(1, 1);
            let mut items = vec![
                (para, block(NodeType::Paragraph, vec![root])),
            ];
            for (i, c) in chars.iter().enumerate() {
                items.push((Dot::new(1, 2 + i as u64), SeqItem::Char(*c)));
            }
            let pre = project_document(&doclogs(&ins_only(&items))).unwrap();
            let pre_view = DocView::new(&pre);

            let live_logs = doclogs(&ins_only(&items));
            let live_ctx = StableResolveCtx::new(&pre_view, &live_logs.seq);
            for offset in 0..=chars.len() {
                let pos = Position::new(para, offset);
                let sp = StablePosition::capture(&pos, &pre_view);
                proptest::prop_assert_eq!(sp.resolve(&live_ctx), Some(pos));
            }

            let captured: Vec<StablePosition> = (0..=chars.len())
                .map(|offset| StablePosition::capture(&Position::new(para, offset), &pre_view))
                .collect();
            if !chars.is_empty() {
                let visible_index = 1 + (del % chars.len());
                let mut ev = ins_only(&items);
                let last = items.last().unwrap().0;
                ev.push(InputEvent {
                    id: Dot::new(1, 100),
                    parents: vec![last],
                    op: ListOp::Del { pos: visible_index, len: 1 },
                });
                let post_logs = doclogs(&ev);
                let post = project_document(&post_logs).unwrap();
                let post_view = DocView::new(&post);
                let ctx = StableResolveCtx::new(&post_view, &post_logs.seq);
                for sp in &captured {
                    let r = sp.resolve(&ctx).expect("resolve returns a position");
                    let host = post_view.node(r.node).expect("host exists");
                    proptest::prop_assert!(r.offset <= host.children().count());
                }
            }
        }
    }
}
