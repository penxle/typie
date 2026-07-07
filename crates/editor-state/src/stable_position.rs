use editor_crdt::sequence::{
    Bias, Boundary, BoundaryResolver, SeqCheckout, checkout_with_resolver,
};
use editor_crdt::{Dot, OpLog};
use editor_macros::ffi;
use editor_model::{ChildView, DocView, NodeView, SeqItem};
use serde::{Deserialize, Serialize};

use crate::Position;
use crate::affinity::Affinity;
use crate::bind::Bind;

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StablePositionBinding {
    Adjacent { anchor: Dot, bind: Bind },
    ContainerStart,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StablePosition {
    chain: Vec<Dot>,
    binding: StablePositionBinding,
    affinity: Affinity,
}

fn child_elem_id(child: &ChildView) -> Dot {
    match child {
        ChildView::Leaf(l) => l.dot(),
        ChildView::Block(b) => b.id(),
    }
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
        let mut chain: Vec<Dot> = host.ancestors().map(|n| n.id()).collect();
        chain.reverse();
        // O(log) child lookups instead of collecting every child of the host block —
        // this runs on the per-keystroke selection-capture path, so a linear scan makes
        // it `O(block)` inside a large paragraph.
        let child_count = host.child_count();
        let binding = child_binding(child_count).map_or(
            StablePositionBinding::ContainerStart,
            |(offset, bind)| StablePositionBinding::Adjacent {
                anchor: child_elem_id(
                    &host
                        .child_at(offset)
                        .expect("child binding offset must be live"),
                ),
                bind,
            },
        );
        StablePosition {
            chain,
            binding,
            affinity: pos.affinity,
        }
    }
}

/// How a `StableResolveCtx` looks up a dot's sequence position. The `Live` variant
/// borrows the projected state's already-materialized checkout, so restoring a
/// selection after a remote edit costs `O(anchors · log N)` tree lookups instead of
/// a fresh `O(N)` whole-sequence checkout + rank map on every changeset.
enum StableResolver<'a> {
    Owned(BoundaryResolver),
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
            resolver: StableResolver::Owned(resolver),
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
}

fn index_of(host: &NodeView, anchor: Dot) -> Option<usize> {
    host.children().position(|c| child_elem_id(&c) == anchor)
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

impl StablePosition {
    pub fn resolve(&self, ctx: &StableResolveCtx) -> Option<Position> {
        if self.chain.is_empty() {
            return None;
        }
        let mut k = 0usize;
        let mut found = false;
        for (i, id) in self.chain.iter().enumerate() {
            if ctx.view.node(*id).is_some() {
                k = i;
                found = true;
            } else {
                break;
            }
        }
        if !found {
            return None;
        }
        let host = ctx.view.node(self.chain[k]).unwrap();
        let offset = if k == self.chain.len() - 1 {
            match &self.binding {
                StablePositionBinding::ContainerStart => 0,
                StablePositionBinding::Adjacent { anchor, bind } => {
                    match index_of(&host, *anchor) {
                        Some(j) => j + usize::from(*bind == Bind::Right),
                        None => offset_within(&host, *anchor, ctx),
                    }
                }
            }
        } else {
            offset_within(&host, self.chain[k + 1], ctx)
        };
        Some(Position {
            node: host.id(),
            offset,
            affinity: self.affinity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::Dot;
    use editor_crdt::{InputEvent, ListOp, build_oplog};
    use editor_model::{
        DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeType, ProjectedDoc,
        SeqItem, SpanLog, project_document,
    };

    use crate::Position;

    fn doclogs(ev: &[InputEvent<SeqItem>]) -> DocLogs {
        DocLogs {
            seq: build_oplog(ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_markers: NodeMarkerLog::new(),
        }
    }

    fn block(node_type: NodeType, parents: Vec<Dot>) -> SeqItem {
        SeqItem::Block { node_type, parents }
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
        assert!(matches!(sp.binding, StablePositionBinding::ContainerStart));
        assert_eq!(sp.chain.last(), Some(&para));
        assert_eq!(sp.chain.first(), view.root().map(|r| r.id()).as_ref());
    }

    #[test]
    fn capture_offset_zero_is_container_start() {
        let (pd, para) = para_with(&[SeqItem::Char('a'), SeqItem::Char('b')]);
        let view = DocView::new(&pd);
        let sp = StablePosition::capture(&Position::new(para, 0), &view);
        assert!(matches!(sp.binding, StablePositionBinding::ContainerStart));
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
            sp.binding,
            StablePositionBinding::Adjacent {
                anchor: Dot::new(1, 3),
                bind: Bind::Left,
            }
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
            sp.binding,
            StablePositionBinding::Adjacent {
                anchor: Dot::new(1, 3),
                bind: Bind::Right,
            }
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
            sp.binding,
            StablePositionBinding::Adjacent {
                anchor: Dot::new(1, 3),
                bind: Bind::Right,
            }
        );
    }

    #[test]
    fn types_construct_and_compare() {
        let sp = StablePosition {
            chain: vec![Dot::ROOT, Dot::new(1, 1)],
            binding: StablePositionBinding::Adjacent {
                anchor: Dot::new(1, 2),
                bind: Bind::Left,
            },
            affinity: Affinity::Downstream,
        };
        assert_eq!(sp.clone(), sp);
        assert!(matches!(sp.binding, StablePositionBinding::Adjacent { .. }));
        let cs = StablePosition {
            chain: vec![Dot::ROOT],
            binding: StablePositionBinding::ContainerStart,
            affinity: Affinity::Upstream,
        };
        assert_ne!(sp, cs);
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
        assert!(matches!(sp.binding, StablePositionBinding::ContainerStart));
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
        assert_eq!(sp.chain.last(), Some(&derived));
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
        assert_eq!(sp.chain.last(), Some(&p1));
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
            chain: vec![view.root().unwrap().id(), para],
            binding: StablePositionBinding::Adjacent {
                anchor: Dot::new(9, 9),
                bind: Bind::Left,
            },
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
    fn normalization_dropped_anchor_resolves_in_range() {
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
        assert!(view.leaf(pb).is_none(), "PageBreak dropped from DocView");
        let para_view = view.node(para).expect("paragraph survives normalize");
        let sp = StablePosition {
            chain: vec![view.root().unwrap().id(), bq, para],
            binding: StablePositionBinding::Adjacent {
                anchor: pb,
                bind: Bind::Left,
            },
            affinity: Affinity::Downstream,
        };
        let r = sp.resolve(&ctx).unwrap();
        assert_eq!(r.node, para);
        assert!(r.offset <= para_view.children().count());
        assert_eq!(r.offset, 0, "empty surviving paragraph -> offset 0");
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
