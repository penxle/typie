use hashbrown::HashSet;

use editor_crdt::wire::{CollectCtx, DecCtx, EncCtx, Wire, WireChangeset, WireError, WireResult};
use editor_crdt::{CrdtError, Dot, ListOp, Op, OpGraph, OpLog};

use crate::{
    DocLogs, ModifierAttrLog, ModifierAttrOp, NodeAttrLog, NodeAttrOp, SeqItem, SpanLog, SpanOp,
};

#[derive(Clone, Debug, PartialEq, Eq, editor_macros::Wire)]
pub enum EditOp {
    #[wire(n(0))]
    Seq(ListOp<SeqItem>),
    #[wire(n(1))]
    Span(SpanOp),
    #[wire(n(2))]
    BlockModifier(ModifierAttrOp),
    #[wire(n(3))]
    NodeAttr(NodeAttrOp),
    #[wire(n(5))]
    NodeCarry(ModifierAttrOp),
}

impl EditOp {
    pub fn is_seq(&self) -> bool {
        matches!(self, EditOp::Seq(_))
    }
}

#[derive(Debug)]
pub enum SplitError {
    Crdt(CrdtError),
}

pub fn seq_parents(graph: &OpGraph<EditOp>, dot: Dot) -> Vec<Dot> {
    let mut out: Vec<Dot> = Vec::new();
    let mut seen: HashSet<Dot> = HashSet::new();
    let start: &Op<EditOp> = graph.get(&dot).expect("op exists");
    let mut stack: Vec<Dot> = start.parents.clone();
    while let Some(p) = stack.pop() {
        if !seen.insert(p) {
            continue;
        }
        let pop = graph.get(&p).expect("parent exists");
        if pop.payload.is_seq() {
            out.push(p);
        } else {
            stack.extend(pop.parents.iter().copied());
        }
    }
    out.sort();
    out
}

pub fn split_logs(graph: &OpGraph<EditOp>) -> Result<DocLogs, SplitError> {
    // Iterate ops in storage order (already ancestry-first) so a full-history
    // load never clones the whole graph through `topo_sort`; the clone-heavy
    // sort only runs for graphs whose storage order is broken (`debug_remove`).
    let owned: Vec<Op<EditOp>>;
    let ordered: Vec<&Op<EditOp>> = match graph.ordered_ops() {
        Some(ops) => ops,
        None => {
            let dots: HashSet<Dot> = graph.iter_all().map(|o| o.id).collect();
            owned = graph.topo_sort(&dots);
            owned.iter().collect()
        }
    };

    // Seq ops seen so far (both orderings above are topological, so a parent
    // is always classified before its children). When every direct parent is
    // a seq op — the entire typing hot path — the op's own normalized parent
    // list IS its seq-parent list and the per-op ancestor walk is skipped.
    let mut seq_dots: HashSet<Dot> = HashSet::with_capacity(ordered.len());

    // Pushed directly in iteration order — already topological, and OpLog
    // linearization order is free (eg-walker replay converges for any
    // topological order; the warm path appends in arrival order the same way).
    let mut seq: OpLog<SeqItem> = OpLog::new();
    let mut spans = SpanLog::new();
    let mut block_modifiers = ModifierAttrLog::new();
    let mut node_attrs = NodeAttrLog::new();
    let mut node_carries = ModifierAttrLog::new();

    for op in ordered {
        match &op.payload {
            EditOp::Seq(list_op) => {
                if op.parents.iter().all(|p| seq_dots.contains(p)) {
                    seq.push_from(op.id, &op.parents, list_op.clone());
                } else {
                    let parents = seq_parents(graph, op.id);
                    seq.push_from(op.id, &parents, list_op.clone());
                }
                seq_dots.insert(op.id);
            }
            EditOp::Span(o) => spans = spans.apply(op.id, o.clone()).map_err(SplitError::Crdt)?,
            EditOp::BlockModifier(o) => {
                block_modifiers = block_modifiers
                    .apply(op.id, o.clone())
                    .map_err(SplitError::Crdt)?
            }
            EditOp::NodeAttr(o) => {
                node_attrs = node_attrs
                    .apply(op.id, o.clone())
                    .map_err(SplitError::Crdt)?
            }
            EditOp::NodeCarry(o) => {
                node_carries = node_carries
                    .apply(op.id, o.clone())
                    .map_err(SplitError::Crdt)?
            }
        }
    }
    Ok(DocLogs {
        seq,
        spans,
        block_modifiers,
        node_attrs,
        node_carries,
    })
}

#[derive(Default)]
pub struct EditOpBundleState;

impl WireChangeset for EditOp {
    type BundleState = EditOpBundleState;

    fn collect_changeset(ops: &[Op<Self>], ctx: &mut CollectCtx) {
        for op in ops {
            ctx.observe(&op.id);
            for p in &op.parents {
                ctx.observe(p);
            }
            <EditOp as Wire>::collect(&op.payload, ctx);
        }
    }

    fn encode_changeset(
        ops: &[Op<Self>],
        _state: &mut Self::BundleState,
        ctx: &EncCtx,
        out: &mut Vec<u8>,
    ) -> WireResult<u32> {
        if ops.is_empty() {
            return Err(WireError::EmptyChangesetOps);
        }
        for (i, op) in ops.iter().enumerate() {
            op.id.encode(ctx, out)?;
            if i > 0 {
                op.parents.encode(ctx, out)?;
            }
            op.payload.encode(ctx, out)?;
        }
        Ok(ops.len() as u32)
    }

    fn decode_changeset(
        _state: &mut Self::BundleState,
        ctx: &DecCtx,
        first_op_parents: Vec<Dot>,
        entry_count: u32,
        input: &mut &[u8],
    ) -> WireResult<Vec<Op<Self>>> {
        if entry_count == 0 {
            return Err(WireError::EmptyChangesetEntries);
        }
        let mut ops = Vec::with_capacity(entry_count as usize);
        for i in 0..entry_count {
            let id = Dot::decode(ctx, input)?;
            let parents = if i == 0 {
                first_op_parents.clone()
            } else {
                <Vec<Dot>>::decode(ctx, input)?
            };
            let payload = EditOp::decode(ctx, input)?;
            ops.push(Op {
                id,
                parents,
                payload,
            });
        }
        Ok(ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Anchor, AtomLeaf, Bias, HorizontalRuleVariant, Modifier, ModifierType, Node, NodeType,
        project_document,
    };
    use editor_crdt::{Changeset, InputEvent};

    /// A leaf's effective modifiers, read from the authoritative segment index.
    fn leaf_eff(
        pd: &crate::ProjectedDoc,
        dot: Dot,
    ) -> std::collections::BTreeMap<crate::ModifierType, Modifier> {
        crate::DocView::new(pd)
            .leaf_state_by_dot_slow(dot)
            .map(|s| s.eff.clone())
            .unwrap_or_default()
    }

    /// Total leaf count across the tree.
    fn leaf_count(pd: &crate::ProjectedDoc) -> usize {
        fn count(tree: &crate::BlockTree, n: &crate::BlockNode) -> usize {
            n.children
                .iter()
                .map(|c| match c {
                    crate::Child::Leaf { .. } => 1,
                    crate::Child::Block(id) => tree.get(*id).map(|b| count(tree, b)).unwrap_or(0),
                })
                .sum()
        }
        pd.tree.root_node().map(|r| count(&pd.tree, r)).unwrap_or(0)
    }

    fn round_trip<T: editor_crdt::wire::Wire>(value: &T) -> editor_crdt::wire::WireResult<T> {
        use editor_crdt::wire::{CollectCtx, DecCtx, EncCtx, WireError};
        let mut cc = CollectCtx::new();
        value.collect(&mut cc);
        let (table, baselines) = cc.finalize();
        let ec = EncCtx::from_table(&table, baselines.clone());
        let dc = DecCtx {
            actor_table: table,
            baselines,
        };
        let mut buf = Vec::new();
        value.encode(&ec, &mut buf)?;
        let mut slice = &buf[..];
        let out = T::decode(&dc, &mut slice)?;
        if !slice.is_empty() {
            return Err(WireError::TrailingBytes {
                remaining: slice.len(),
            });
        }
        Ok(out)
    }

    #[test]
    fn edit_op_wire_round_trip_all_variants() {
        let node = |t: NodeType| t.into_node();
        let img = match node(NodeType::Image) {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        let file = match node(NodeType::File) {
            Node::File(n) => n,
            _ => unreachable!(),
        };
        let embed = match node(NodeType::Embed) {
            Node::Embed(n) => n,
            _ => unreachable!(),
        };
        let arch = match node(NodeType::Archived) {
            Node::Archived(n) => n,
            _ => unreachable!(),
        };
        let atom = |a: AtomLeaf| {
            EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Atom(a),
            })
        };
        let ops = vec![
            EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Char('a'),
            }),
            EditOp::Seq(ListOp::Ins {
                pos: 7,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::new(1, 0)],
                },
            }),
            atom(AtomLeaf::HardBreak),
            atom(AtomLeaf::Tab),
            atom(AtomLeaf::PageBreak),
            atom(AtomLeaf::HorizontalRule {
                variant: HorizontalRuleVariant::default(),
            }),
            atom(AtomLeaf::Image { node: img }),
            atom(AtomLeaf::File { node: file }),
            atom(AtomLeaf::Embed { node: embed }),
            atom(AtomLeaf::Archived { node: arch }),
            EditOp::Seq(ListOp::Del { pos: 2, len: 4 }),
            EditOp::Seq(ListOp::Undel {
                del: Dot::new(2, 5),
            }),
            EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: Dot::new(1, 2),
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: Dot::new(1, 2),
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }),
            EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: Dot::new(1, 1),
                modifier: Modifier::FontSize { value: 1600 },
            }),
            EditOp::NodeAttr(crate::NodeAttrOp {
                target: Dot::new(1, 1),
                attr: crate::NodeAttr::Callout {
                    attr: crate::CalloutNodeAttr::Variant(crate::CalloutVariant::Warning),
                },
            }),
            EditOp::NodeCarry(ModifierAttrOp::SetModifier {
                target: Dot::new(1, 1),
                modifier: Modifier::Bold,
            }),
        ];
        for op in &ops {
            assert_eq!(&round_trip(op).unwrap(), op, "round-trip mismatch: {op:?}");
        }
    }

    fn seq_ins(pos: usize, item: SeqItem) -> EditOp {
        EditOp::Seq(ListOp::Ins { pos, item })
    }
    fn dummy_span(a: Dot) -> EditOp {
        EditOp::Span(SpanOp::AddSpan {
            start: crate::Anchor {
                id: a,
                bias: crate::Bias::Before,
            },
            end: crate::Anchor {
                id: a,
                bias: crate::Bias::After,
            },
            modifier: crate::Modifier::Bold,
        })
    }

    #[test]
    fn is_seq_classifies() {
        assert!(seq_ins(0, SeqItem::Char('a')).is_seq());
        assert!(!dummy_span(Dot::new(1, 0)).is_seq());
    }

    fn build_chained_bundle() -> Vec<Changeset<EditOp>> {
        let mut g: OpGraph<EditOp> = OpGraph::with_actor(1);
        let root = g
            .add_mut(seq_ins(
                0,
                SeqItem::Block {
                    node_type: NodeType::Root,
                    parents: vec![],
                },
            ))
            .unwrap()
            .id;
        let para = g
            .add_mut(seq_ins(
                1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ))
            .unwrap()
            .id;
        g.add_mut(seq_ins(2, SeqItem::Char('a'))).unwrap();
        g.add_mut(EditOp::Span(SpanOp::AddSpan {
            start: Anchor {
                id: para,
                bias: Bias::Before,
            },
            end: Anchor {
                id: para,
                bias: Bias::After,
            },
            modifier: Modifier::Bold,
        }))
        .unwrap();
        g.commit_mut();
        g.add_mut(seq_ins(3, SeqItem::Char('b'))).unwrap();
        g.add_mut(EditOp::NodeCarry(ModifierAttrOp::SetModifier {
            target: para,
            modifier: Modifier::Bold,
        }))
        .unwrap();
        g.commit_mut();
        g.changesets_as_vec()
    }

    #[test]
    fn edit_op_changeset_wire_round_trip() {
        let css = build_chained_bundle();
        assert!(css.len() >= 2);
        let bytes = editor_crdt::wire::encode(&css).unwrap();
        let decoded: Vec<Changeset<EditOp>> = editor_crdt::wire::decode(&bytes).unwrap();
        assert_eq!(decoded, css);
    }

    #[test]
    fn edit_op_encode_changeset_rejects_empty() {
        use editor_crdt::wire::WireError;
        let empty: Vec<Changeset<EditOp>> = vec![Changeset { ops: vec![] }];
        assert!(matches!(
            editor_crdt::wire::encode(&empty),
            Err(WireError::EmptyChangesetOps)
        ));
    }

    #[test]
    fn seq_parents_linear_seq() {
        let mut g: OpGraph<EditOp> = OpGraph::new();
        let a = g.add_mut(seq_ins(0, SeqItem::Char('a'))).unwrap().id;
        let b = g.add_mut(seq_ins(1, SeqItem::Char('b'))).unwrap().id;
        assert_eq!(seq_parents(&g, b), vec![a]);
        assert_eq!(seq_parents(&g, a), Vec::<Dot>::new());
    }

    #[test]
    fn seq_parents_skips_non_seq() {
        let mut g: OpGraph<EditOp> = OpGraph::new();
        let a = g.add_mut(seq_ins(0, SeqItem::Char('a'))).unwrap().id;
        let _x = g.add_mut(dummy_span(a)).unwrap().id;
        let c = g.add_mut(seq_ins(1, SeqItem::Char('c'))).unwrap().id;
        assert_eq!(seq_parents(&g, c), vec![a]);
    }

    #[test]
    fn seq_parents_skips_non_seq_chain() {
        let mut g: OpGraph<EditOp> = OpGraph::new();
        let a = g.add_mut(seq_ins(0, SeqItem::Char('a'))).unwrap().id;
        let x = g.add_mut(dummy_span(a)).unwrap().id;
        let _y = g.add_mut(dummy_span(x)).unwrap().id;
        let c = g.add_mut(seq_ins(1, SeqItem::Char('c'))).unwrap().id;
        assert_eq!(seq_parents(&g, c), vec![a]);
    }

    fn graph_para(text: &str) -> (OpGraph<EditOp>, Dot, Dot) {
        let mut g: OpGraph<EditOp> = OpGraph::new();
        let para = g
            .add_mut(seq_ins(
                0,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ))
            .unwrap()
            .id;
        for (i, ch) in text.chars().enumerate() {
            g.add_mut(seq_ins(1 + i, SeqItem::Char(ch))).unwrap();
        }
        (g, Dot::ROOT, para)
    }

    #[test]
    fn split_pure_text() {
        let (g, _root, _para) = graph_para("ab");
        let logs = split_logs(&g).unwrap();
        let pd = project_document(&logs).unwrap();
        assert_eq!(pd.tree.root_node().iter().count(), 1);
        assert_eq!(pd.tree.root_node().unwrap().node_type, NodeType::Root);
        assert_eq!(leaf_count(&pd), 2);
    }

    #[test]
    fn split_routes_span_overlay() {
        let mut g: OpGraph<EditOp> = OpGraph::new();
        let _para = g
            .add_mut(seq_ins(
                0,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ))
            .unwrap()
            .id;
        let _a = g.add_mut(seq_ins(1, SeqItem::Char('a'))).unwrap().id;
        let _b = g.add_mut(seq_ins(2, SeqItem::Char('b'))).unwrap().id;
        let c = g.add_mut(seq_ins(3, SeqItem::Char('c'))).unwrap().id;
        g.add_mut(EditOp::Span(SpanOp::AddSpan {
            start: crate::Anchor {
                id: c,
                bias: crate::Bias::Before,
            },
            end: crate::Anchor {
                id: c,
                bias: crate::Bias::After,
            },
            modifier: crate::Modifier::Bold,
        }))
        .unwrap();
        let pd = project_document(&split_logs(&g).unwrap()).unwrap();
        assert_eq!(
            leaf_eff(&pd, c).get(&crate::ModifierType::Bold),
            Some(&crate::Modifier::Bold)
        );
    }

    #[test]
    fn split_routes_node_attr() {
        let mut g: OpGraph<EditOp> = OpGraph::new();
        let callout = g
            .add_mut(seq_ins(
                0,
                SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![Dot::ROOT],
                },
            ))
            .unwrap()
            .id;
        let _p = g
            .add_mut(seq_ins(
                1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, callout],
                },
            ))
            .unwrap()
            .id;
        let _x = g.add_mut(seq_ins(2, SeqItem::Char('x'))).unwrap().id;
        g.add_mut(EditOp::NodeAttr(crate::NodeAttrOp {
            target: callout,
            attr: crate::NodeAttr::Callout {
                attr: crate::CalloutNodeAttr::Variant(crate::CalloutVariant::Warning),
            },
        }))
        .unwrap();
        let pd = project_document(&split_logs(&g).unwrap()).unwrap();
        assert!(pd.node_attrs.contains_key(&callout));
    }

    #[test]
    fn split_seq_parent_skip_end_to_end() {
        let mut g: OpGraph<EditOp> = OpGraph::new();
        let _para = g
            .add_mut(seq_ins(
                0,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ))
            .unwrap()
            .id;
        let a = g.add_mut(seq_ins(1, SeqItem::Char('a'))).unwrap().id;
        g.add_mut(EditOp::NodeCarry(ModifierAttrOp::SetModifier {
            target: a,
            modifier: Modifier::Bold,
        }))
        .unwrap();
        let _b = g.add_mut(seq_ins(2, SeqItem::Char('b'))).unwrap().id;
        let pd = project_document(&split_logs(&g).unwrap()).unwrap();
        assert_eq!(leaf_count(&pd), 2);
    }

    #[test]
    fn split_seq_matches_standalone_reference() {
        let mut g: OpGraph<EditOp> = OpGraph::new();
        let root = g
            .add_mut(seq_ins(
                0,
                SeqItem::Block {
                    node_type: NodeType::Root,
                    parents: vec![],
                },
            ))
            .unwrap()
            .id;
        let para = g
            .add_mut(seq_ins(
                1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ))
            .unwrap()
            .id;
        let a = g.add_mut(seq_ins(2, SeqItem::Char('a'))).unwrap().id;
        g.add_mut(EditOp::NodeCarry(ModifierAttrOp::SetModifier {
            target: a,
            modifier: Modifier::Bold,
        }))
        .unwrap();
        let b = g.add_mut(seq_ins(3, SeqItem::Char('b'))).unwrap().id;

        let split = split_logs(&g).unwrap();
        let from_split = editor_crdt::sequence::checkout(&split.seq);

        let std_events = vec![
            InputEvent {
                id: root,
                parents: vec![],
                op: ListOp::Ins {
                    pos: 0,
                    item: SeqItem::Block {
                        node_type: NodeType::Root,
                        parents: vec![],
                    },
                },
            },
            InputEvent {
                id: para,
                parents: vec![root],
                op: ListOp::Ins {
                    pos: 1,
                    item: SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root],
                    },
                },
            },
            InputEvent {
                id: a,
                parents: vec![para],
                op: ListOp::Ins {
                    pos: 2,
                    item: SeqItem::Char('a'),
                },
            },
            InputEvent {
                id: b,
                parents: vec![a],
                op: ListOp::Ins {
                    pos: 3,
                    item: SeqItem::Char('b'),
                },
            },
        ];
        let reference = editor_crdt::sequence::checkout(&editor_crdt::build_oplog(&std_events));
        assert_eq!(from_split, reference);
    }

    #[test]
    fn split_empty_graph_ok() {
        let g: OpGraph<EditOp> = OpGraph::new();
        let pd = project_document(&split_logs(&g).unwrap()).unwrap();
        assert_eq!(pd.tree.root_node().iter().count(), 1);
        assert_eq!(leaf_count(&pd), 0);
    }

    #[test]
    fn split_overlay_only_ok() {
        let mut g: OpGraph<EditOp> = OpGraph::new();
        g.add_mut(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
            target: Dot::ROOT,
            modifier: Modifier::FontSize { value: 1600 },
        }))
        .unwrap();
        let pd = project_document(&split_logs(&g).unwrap()).unwrap();
        assert_eq!(leaf_count(&pd), 0);
    }

    #[test]
    fn from_changesets_compiles_for_edit_op() {
        let cs = Changeset {
            ops: vec![Op {
                id: Dot::new(1, 1),
                parents: vec![],
                payload: seq_ins(
                    0,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![Dot::ROOT],
                    },
                ),
            }],
        };
        let g = OpGraph::from_changesets(vec![cs]).unwrap();
        let pd = project_document(&split_logs(&g).unwrap()).unwrap();
        assert_eq!(pd.tree.root_node().iter().count(), 1);
    }

    fn blk(nt: NodeType, parents: Vec<Dot>) -> SeqItem {
        SeqItem::Block {
            node_type: nt,
            parents,
        }
    }
    fn op_seq(a: u64, c: u64, parents: &[Dot], pos: usize, item: SeqItem) -> Op<EditOp> {
        Op {
            id: Dot::new(a, c),
            parents: parents.to_vec(),
            payload: seq_ins(pos, item),
        }
    }
    fn op_carry(a: u64, c: u64, parents: &[Dot], target: Dot, v: &str) -> Op<EditOp> {
        Op {
            id: Dot::new(a, c),
            parents: parents.to_vec(),
            payload: EditOp::NodeCarry(ModifierAttrOp::SetModifier {
                target,
                modifier: Modifier::TextColor {
                    value: v.to_string(),
                },
            }),
        }
    }
    fn graph(ops: Vec<Op<EditOp>>) -> OpGraph<EditOp> {
        OpGraph::from_changesets(vec![Changeset { ops }]).unwrap()
    }
    fn graphs(css: Vec<Vec<Op<EditOp>>>) -> OpGraph<EditOp> {
        let css: Vec<Changeset<EditOp>> = css
            .into_iter()
            .filter(|ops| !ops.is_empty())
            .map(|ops| Changeset { ops })
            .collect();
        OpGraph::from_changesets(css).unwrap()
    }
    fn checkout_seq(g: &OpGraph<EditOp>) -> Vec<(Dot, SeqItem)> {
        editor_crdt::sequence::checkout(&split_logs(g).unwrap().seq)
    }

    #[test]
    fn ground_truth_linear_interleave() {
        let root = Dot::new(1, 0);
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let ov = Dot::new(2, 0);
        let g_mixed = graph(vec![
            op_seq(1, 0, &[], 0, blk(NodeType::Root, vec![])),
            op_seq(1, 1, &[root], 1, blk(NodeType::Paragraph, vec![root])),
            op_seq(1, 2, &[para], 2, SeqItem::Char('a')),
            op_carry(2, 0, &[a], a, "s"),
            op_seq(1, 3, &[ov], 3, SeqItem::Char('b')),
        ]);
        let g_pure = graph(vec![
            op_seq(1, 0, &[], 0, blk(NodeType::Root, vec![])),
            op_seq(1, 1, &[root], 1, blk(NodeType::Paragraph, vec![root])),
            op_seq(1, 2, &[para], 2, SeqItem::Char('a')),
            op_seq(1, 3, &[a], 3, SeqItem::Char('b')),
        ]);
        assert_eq!(
            checkout_seq(&g_mixed),
            checkout_seq(&g_pure),
            "projection must recover pure-seq result across overlay interleave",
        );
    }

    #[test]
    fn ground_truth_concurrent_insert() {
        let root = Dot::new(1, 0);
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let ov = Dot::new(2, 0);
        let g_mixed = graph(vec![
            op_seq(1, 0, &[], 0, blk(NodeType::Root, vec![])),
            op_seq(1, 1, &[root], 1, blk(NodeType::Paragraph, vec![root])),
            op_seq(1, 2, &[para], 2, SeqItem::Char('a')),
            op_carry(2, 0, &[a], a, "s"),
            op_seq(1, 3, &[ov], 3, SeqItem::Char('x')),
            op_seq(3, 0, &[a], 3, SeqItem::Char('y')),
        ]);
        let g_pure = graph(vec![
            op_seq(1, 0, &[], 0, blk(NodeType::Root, vec![])),
            op_seq(1, 1, &[root], 1, blk(NodeType::Paragraph, vec![root])),
            op_seq(1, 2, &[para], 2, SeqItem::Char('a')),
            op_seq(1, 3, &[a], 3, SeqItem::Char('x')),
            op_seq(3, 0, &[a], 3, SeqItem::Char('y')),
        ]);
        assert_eq!(checkout_seq(&g_mixed), checkout_seq(&g_pure));
    }

    #[test]
    fn cross_replica_merge_order_independent() {
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let cs0 = vec![
            op_seq(1, 1, &[], 0, blk(NodeType::Paragraph, vec![Dot::ROOT])),
            op_seq(1, 2, &[para], 1, SeqItem::Char('a')),
        ];
        let cs1 = vec![op_seq(1, 3, &[a], 2, SeqItem::Char('x'))];
        let cs2 = vec![op_seq(2, 0, &[a], 2, SeqItem::Char('y'))];
        let g_ab = graphs(vec![cs0.clone(), cs1.clone(), cs2.clone()]);
        let g_ba = graphs(vec![cs0, cs2, cs1]);
        assert_eq!(
            project_document(&split_logs(&g_ab).unwrap()).unwrap(),
            project_document(&split_logs(&g_ba).unwrap()).unwrap(),
        );
    }

    #[test]
    fn concurrent_overlay_converges() {
        let para = Dot::new(1, 1);
        let cs0 = vec![
            op_seq(1, 1, &[], 0, blk(NodeType::Paragraph, vec![Dot::ROOT])),
            op_seq(1, 2, &[para], 1, SeqItem::Char('a')),
        ];
        let cs1 = vec![op_carry(1, 3, &[para], para, "s1")];
        let cs2 = vec![op_carry(2, 0, &[para], para, "s2")];
        let g_ab = graphs(vec![cs0.clone(), cs1.clone(), cs2.clone()]);
        let g_ba = graphs(vec![cs0, cs2, cs1]);
        let pd_ab = project_document(&split_logs(&g_ab).unwrap()).unwrap();
        let pd_ba = project_document(&split_logs(&g_ba).unwrap()).unwrap();
        assert_eq!(pd_ab, pd_ba);
        assert!(pd_ab.node_carries.get(&para).is_some());
    }

    #[test]
    fn overlay_heavy_permutation_converges() {
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let cs0 = vec![
            op_seq(1, 1, &[], 0, blk(NodeType::Paragraph, vec![Dot::ROOT])),
            op_seq(1, 2, &[para], 1, SeqItem::Char('a')),
        ];
        let o1 = vec![op_carry(1, 3, &[a], para, "s1")];
        let o2 = vec![op_carry(2, 0, &[a], Dot::ROOT, "s2")];
        let o3 = vec![op_carry(3, 0, &[a], para, "s3")];
        let g1 = graphs(vec![cs0.clone(), o1.clone(), o2.clone(), o3.clone()]);
        let g2 = graphs(vec![cs0.clone(), o3.clone(), o1.clone(), o2.clone()]);
        let g3 = graphs(vec![cs0, o2, o3, o1]);
        let p1 = project_document(&split_logs(&g1).unwrap()).unwrap();
        let p2 = project_document(&split_logs(&g2).unwrap()).unwrap();
        let p3 = project_document(&split_logs(&g3).unwrap()).unwrap();
        assert_eq!(p1, p2);
        assert_eq!(p1, p3);
    }

    fn op_carry_mod(
        a: u64,
        c: u64,
        parents: &[Dot],
        target: Dot,
        modifier: Modifier,
    ) -> Op<EditOp> {
        Op {
            id: Dot::new(a, c),
            parents: parents.to_vec(),
            payload: EditOp::NodeCarry(ModifierAttrOp::SetModifier { target, modifier }),
        }
    }

    fn para_seed() -> (Dot, Dot, Vec<Op<EditOp>>) {
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let base = vec![
            op_seq(1, 1, &[], 0, blk(NodeType::Paragraph, vec![Dot::ROOT])),
            op_seq(1, 2, &[para], 1, SeqItem::Char('a')),
        ];
        (para, a, base)
    }

    #[test]
    fn concurrent_distinct_carry_kinds_both_survive() {
        let (para, a, base) = para_seed();
        let bold = vec![op_carry_mod(2, 0, &[a], para, Modifier::Bold)];
        let size = vec![op_carry_mod(
            3,
            0,
            &[a],
            para,
            Modifier::FontSize { value: 1600 },
        )];
        for order in [
            graphs(vec![base.clone(), bold.clone(), size.clone()]),
            graphs(vec![base.clone(), size.clone(), bold.clone()]),
        ] {
            let pd = project_document(&split_logs(&order).unwrap()).unwrap();
            let c = pd.node_carries.get(&para).expect("carries on paragraph");
            assert_eq!(c.get(&ModifierType::Bold), Some(&Modifier::Bold));
            assert_eq!(
                c.get(&ModifierType::FontSize),
                Some(&Modifier::FontSize { value: 1600 }),
                "distinct-kind concurrent carries both survive per-kind LWW"
            );
        }
    }

    #[test]
    fn same_carry_kind_conflict_higher_dot_wins() {
        let (para, a, base) = para_seed();
        let lo = vec![op_carry_mod(
            2,
            0,
            &[a],
            para,
            Modifier::FontSize { value: 1200 },
        )];
        let hi = vec![op_carry_mod(
            3,
            0,
            &[a],
            para,
            Modifier::FontSize { value: 1600 },
        )];
        let g = graphs(vec![base, lo, hi]);
        let pd = project_document(&split_logs(&g).unwrap()).unwrap();
        assert_eq!(
            pd.node_carries
                .get(&para)
                .and_then(|c| c.get(&ModifierType::FontSize)),
            Some(&Modifier::FontSize { value: 1600 }),
            "same-kind concurrent carries resolve to the higher Dot"
        );
    }

    #[test]
    fn non_carry_kind_carry_op_ignored_in_projection() {
        let (para, a, base) = para_seed();
        let link = vec![op_carry_mod(
            2,
            0,
            &[a],
            para,
            Modifier::Link { href: "x".into() },
        )];
        let g = graphs(vec![base, link]);
        let pd = project_document(&split_logs(&g).unwrap()).unwrap();
        assert!(
            pd.node_carries
                .get(&para)
                .is_none_or(|c| !c.contains_key(&ModifierType::Link)),
            "a non-carry kind never reaches the projected carries"
        );
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn random_two_actor_merge_converges(text in "[a-c]{2,6}") {
            let para = Dot::new(1, 1);
            let base = vec![
                op_seq(1, 1, &[], 0, blk(NodeType::Paragraph, vec![Dot::ROOT])),
            ];
            let mut a1: Vec<Op<EditOp>> = Vec::new();
            let mut a2: Vec<Op<EditOp>> = Vec::new();
            for (i, ch) in text.chars().enumerate() {
                let opv = op_seq(if i % 2 == 0 { 1 } else { 2 }, 10 + i as u64, &[para], 1, SeqItem::Char(ch));
                if i % 2 == 0 { a1.push(opv) } else { a2.push(opv) }
            }
            let g_ab = graphs(vec![base.clone(), a1.clone(), a2.clone()]);
            let g_ba = graphs(vec![base, a2, a1]);
            prop_assert_eq!(
                project_document(&split_logs(&g_ab).unwrap()).unwrap(),
                project_document(&split_logs(&g_ba).unwrap()).unwrap(),
            );
        }

        #[test]
        fn changeset_wire_round_trip_proptest(text in "[a-c]{1,10}") {
            let mut g: OpGraph<EditOp> = OpGraph::with_actor(1);
            let root = g
                .add_mut(seq_ins(
                    0,
                    SeqItem::Block {
                        node_type: NodeType::Root,
                        parents: vec![],
                    },
                ))
                .unwrap()
                .id;
            g.add_mut(seq_ins(
                1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ))
            .unwrap();
            for (i, ch) in text.chars().enumerate() {
                g.add_mut(seq_ins(2 + i, SeqItem::Char(ch))).unwrap();
            }
            let css = g.commit().changesets_as_vec();
            let bytes = editor_crdt::wire::encode(&css).unwrap();
            let decoded: Vec<Changeset<EditOp>> = editor_crdt::wire::decode(&bytes).unwrap();
            prop_assert_eq!(decoded, css);
        }

        #[test]
        fn carry_merge_converges_under_delivery_shuffle(
            kinds in prop::collection::vec(0u8..4, 1..8),
            seed in any::<u64>(),
        ) {
            let (para, a, base) = para_seed();
            let carries: Vec<Vec<Op<EditOp>>> = kinds
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    let modifier = match k {
                        0 => Modifier::Bold,
                        1 => Modifier::Italic,
                        2 => Modifier::FontSize { value: 1200 + i as u32 },
                        _ => Modifier::FontWeight { value: 400 },
                    };
                    vec![op_carry_mod(2 + i as u64, 0, &[a], para, modifier)]
                })
                .collect();

            let mut idx: Vec<(u64, usize)> = (0..carries.len())
                .map(|i| {
                    let mut z = (i as u64).wrapping_add(seed.wrapping_mul(0x9E3779B97F4A7C15));
                    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
                    z ^= z >> 31;
                    (z, i)
                })
                .collect();
            idx.sort_by_key(|(z, _)| *z);

            let mut in_order = vec![base.clone()];
            in_order.extend(carries.clone());
            let mut shuffled = vec![base];
            shuffled.extend(idx.iter().map(|(_, i)| carries[*i].clone()));

            let a_pd = project_document(&split_logs(&graphs(in_order)).unwrap()).unwrap();
            let b_pd = project_document(&split_logs(&graphs(shuffled)).unwrap()).unwrap();
            prop_assert_eq!(a_pd, b_pd);
        }
    }
}
