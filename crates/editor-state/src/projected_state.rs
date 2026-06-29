use editor_crdt::{Changeset, CrdtError, Dot, ListOp, Op, OpGraph, OpLog};
use editor_model::{
    DocLogs, DocView, EditOp, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeStyleLog, NodeType,
    ProjectedDoc, ProjectionError, SeqItem, SpanLog, SplitError, StyleLog, project_document,
    split_logs,
};

#[derive(Debug)]
pub enum SpineError {
    Crdt(CrdtError),
    Split(SplitError),
    Projection(ProjectionError),
}

impl From<CrdtError> for SpineError {
    fn from(e: CrdtError) -> Self {
        SpineError::Crdt(e)
    }
}
impl From<SplitError> for SpineError {
    fn from(e: SplitError) -> Self {
        SpineError::Split(e)
    }
}
impl From<ProjectionError> for SpineError {
    fn from(e: ProjectionError) -> Self {
        SpineError::Projection(e)
    }
}

#[derive(Clone, Debug)]
pub struct ProjectedState {
    graph: OpGraph<EditOp>,
    logs: DocLogs,
    projected: ProjectedDoc,
}

impl ProjectedState {
    fn derive(graph: &OpGraph<EditOp>) -> Result<(DocLogs, ProjectedDoc), SpineError> {
        let logs = split_logs(graph)?;
        let projected = project_document(&logs)?;
        Ok((logs, projected))
    }

    pub fn from_graph(graph: OpGraph<EditOp>) -> Result<Self, SpineError> {
        let (logs, projected) = Self::derive(&graph)?;
        Ok(Self {
            graph,
            logs,
            projected,
        })
    }

    pub fn empty() -> Self {
        let mut graph = OpGraph::<EditOp>::with_actor(1);
        graph
            .add_mut(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            }))
            .expect("seed paragraph never conflicts");
        Self::from_graph(graph).expect("seed paragraph always projects")
    }

    pub fn apply(&mut self, payload: EditOp) -> Result<Op<EditOp>, SpineError> {
        let op = self.graph.add_mut(payload)?;
        let (logs, projected) = Self::derive(&self.graph)?;
        self.logs = logs;
        self.projected = projected;
        Ok(op)
    }

    pub fn apply_batch(&mut self, payloads: Vec<EditOp>) -> Result<Vec<Op<EditOp>>, SpineError> {
        let mut ops = Vec::with_capacity(payloads.len());
        for payload in payloads {
            match self.graph.add_mut(payload) {
                Ok(op) => ops.push(op),
                Err(e) => {
                    let (logs, projected) = Self::derive(&self.graph)?;
                    self.logs = logs;
                    self.projected = projected;
                    return Err(e.into());
                }
            }
        }
        let (logs, projected) = Self::derive(&self.graph)?;
        self.logs = logs;
        self.projected = projected;
        Ok(ops)
    }

    pub fn commit(&mut self) {
        self.graph.commit_mut();
    }

    pub fn receive_changeset(&self, cs: Changeset<EditOp>) -> Result<Self, SpineError> {
        let graph = self.graph.receive_changeset(cs)?;
        Self::from_graph(graph)
    }

    pub fn view(&self) -> DocView<'_> {
        DocView::new(&self.projected)
    }

    pub fn projected(&self) -> &ProjectedDoc {
        &self.projected
    }

    pub fn graph(&self) -> &OpGraph<EditOp> {
        &self.graph
    }

    pub fn block_modifiers(&self) -> &ModifierAttrLog {
        &self.logs.block_modifiers
    }

    pub fn seq(&self) -> &OpLog<SeqItem> {
        &self.logs.seq
    }

    pub fn node_attrs(&self) -> &NodeAttrLog {
        &self.logs.node_attrs
    }

    pub fn node_styles(&self) -> &NodeStyleLog {
        &self.logs.node_styles
    }

    pub fn node_markers(&self) -> &NodeMarkerLog {
        &self.logs.node_markers
    }

    pub fn styles(&self) -> &StyleLog {
        &self.logs.styles
    }

    pub fn spans(&self) -> &SpanLog {
        &self.logs.spans
    }

    pub fn seq_flat_pos(&self, dot: editor_crdt::Dot) -> Option<usize> {
        let (_, resolver) = editor_crdt::sequence::checkout_with_resolver(&self.logs.seq);
        resolver
            .resolve_boundary(dot, editor_crdt::sequence::Bias::Before)
            .map(|b| b.position)
    }

    /// `(position, len)` of the elements deleted by deletion op `del`, resolved
    /// against the current tree (used to invert an `Undel` for redo).
    pub fn del_target_span(&self, del: editor_crdt::Dot) -> Option<(usize, usize)> {
        let (_, resolver) = editor_crdt::sequence::checkout_with_resolver(&self.logs.seq);
        resolver.del_target_span(del)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, LwwRegOp, OrMapOp, OrSetOp};
    use editor_model::{
        Anchor, Bias, CalloutNodeAttr, CalloutVariant, Marker, Modifier, ModifierAttrOp,
        ModifierType, Node, NodeAttr, NodeAttrOp, NodeLwwOp, SpanOp, StyleOp, StyleRegOp,
    };

    fn seq_block(pos: usize, node_type: NodeType, parents: Vec<Dot>) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Block { node_type, parents },
        })
    }

    fn seq_char(pos: usize, c: char) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Char(c),
        })
    }

    #[test]
    fn from_graph_projects_a_paragraph() {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let para = g
            .add_mut(seq_block(0, NodeType::Paragraph, vec![Dot::ROOT]))
            .unwrap()
            .id;
        g.add_mut(seq_char(1, 'H')).unwrap();
        g.add_mut(seq_char(2, 'i')).unwrap();

        let state = ProjectedState::from_graph(g).expect("projects");
        let view = state.view();
        let p = view.node(para).expect("paragraph present");
        assert_eq!(p.node_type(), NodeType::Paragraph);
        assert_eq!(p.inline_text(), "Hi");
        assert!(state.block_modifiers().modifiers_of(Dot::ROOT).is_empty());
    }

    #[test]
    fn empty_seeds_implicit_root_and_paragraph() {
        let state = ProjectedState::empty();
        let view = state.view();
        let root = view.root().expect("root present");
        assert_eq!(root.node_type(), NodeType::Root);
        assert_eq!(root.id(), Dot::ROOT);
        let para = root.child_blocks().next().expect("seeded paragraph");
        assert_eq!(para.node_type(), NodeType::Paragraph);
        assert!(para.id().as_op_dot().is_some());
    }

    #[test]
    fn apply_builds_paragraph_and_returns_op_dots() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let h = state.apply(seq_char(1, 'H')).unwrap();
        let _i = state.apply(seq_char(2, 'i')).unwrap();
        let view = state.view();
        assert_eq!(view.leaf(h.id).and_then(|l| l.as_char()), Some('H'));
        let p = view.node(para).unwrap();
        assert_eq!(p.inline_text(), "Hi");
    }

    #[test]
    fn apply_nested_blocks() {
        let mut state = ProjectedState::empty();
        let root = state.view().root().unwrap().dot().unwrap();
        let bq = state
            .apply(seq_block(1, NodeType::Blockquote, vec![root]))
            .unwrap()
            .id;
        let bqp = state
            .apply(seq_block(2, NodeType::Paragraph, vec![root, bq]))
            .unwrap()
            .id;
        let _x = state.apply(seq_char(3, 'x')).unwrap();
        let view = state.view();
        let bqp_view = view.node(bqp).unwrap();
        assert_eq!(bqp_view.inline_text(), "x");
        assert_eq!(bqp_view.parent().unwrap().node_type(), NodeType::Blockquote);
    }

    #[test]
    fn apply_span_enriches_effective() {
        let mut state = ProjectedState::empty();
        let x = state.apply(seq_char(1, 'x')).unwrap().id;
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: x,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: x,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap();
        let view = state.view();
        assert_eq!(
            view.leaf(x).unwrap().effective().get(&ModifierType::Bold),
            Some(&Modifier::Bold)
        );
    }

    #[test]
    fn apply_block_modifier_lands_in_log_and_projection() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: para,
                modifier: Modifier::FontSize { value: 1600 },
            }))
            .unwrap();
        assert_eq!(
            state
                .block_modifiers()
                .modifiers_of(para)
                .get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
        let c = state.apply(seq_char(1, 'a')).unwrap().id;
        assert_eq!(
            state
                .view()
                .leaf(c)
                .unwrap()
                .effective()
                .get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn apply_node_style_and_style_log() {
        let mut state = ProjectedState::empty();
        let x = state.apply(seq_char(1, 'x')).unwrap().id;
        state
            .apply(EditOp::NodeStyle(NodeLwwOp {
                target: x,
                op: LwwRegOp::Set {
                    value: Some("s".to_string()),
                },
            }))
            .unwrap();
        state
            .apply(EditOp::Style(StyleRegOp {
                style_id: "s".to_string(),
                op: StyleOp::Presence(OrMapOp::Set {
                    key: "s".to_string(),
                    value: (),
                }),
            }))
            .unwrap();
        state
            .apply(EditOp::Style(StyleRegOp {
                style_id: "s".to_string(),
                op: StyleOp::Modifiers(OrSetOp::Add {
                    elem: Modifier::Italic,
                }),
            }))
            .unwrap();
        assert_eq!(
            state
                .view()
                .leaf(x)
                .unwrap()
                .effective()
                .get(&ModifierType::Italic),
            Some(&Modifier::Italic)
        );
    }

    #[test]
    fn apply_node_marker_projects() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        state
            .apply(EditOp::NodeMarker(NodeLwwOp {
                target: para,
                op: LwwRegOp::Set {
                    value: Some(Marker {
                        modifiers: vec![Modifier::Bold],
                        style: None,
                    }),
                },
            }))
            .unwrap();
        assert!(state.projected().node_markers.get(&para).is_some());
    }

    #[test]
    fn apply_delete_and_undel() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let a = state.apply(seq_char(1, 'a')).unwrap().id;
        let _b = state.apply(seq_char(2, 'b')).unwrap();
        assert_eq!(state.view().node(para).unwrap().inline_text(), "ab");
        let del = state
            .apply(EditOp::Seq(ListOp::Del { pos: 1, len: 1 }))
            .unwrap()
            .id;
        assert_eq!(state.view().node(para).unwrap().inline_text(), "b");
        let _ = a;
        state.apply(EditOp::Seq(ListOp::Undel { del })).unwrap();
        assert_eq!(state.view().node(para).unwrap().inline_text(), "ab");
    }

    #[test]
    fn apply_leaf_typed_block_errors() {
        let mut state = ProjectedState::empty();
        let root = state.view().root().unwrap().dot().unwrap();
        let err = state.apply(seq_block(1, NodeType::Text, vec![root]));
        assert!(matches!(
            err,
            Err(SpineError::Projection(
                ProjectionError::LeafTypedBlock { .. }
            ))
        ));
    }

    #[test]
    fn apply_node_attr_projects() {
        let mut state = ProjectedState::empty();
        let root = state.view().root().unwrap().dot().unwrap();
        let callout = state
            .apply(seq_block(1, NodeType::Callout, vec![root]))
            .unwrap()
            .id;
        let _cp = state
            .apply(seq_block(2, NodeType::Paragraph, vec![root, callout]))
            .unwrap();
        state
            .apply(EditOp::NodeAttr(NodeAttrOp {
                target: callout,
                attr: NodeAttr::Callout {
                    attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
                },
            }))
            .unwrap();
        assert!(state.projected().node_attrs.contains_key(&callout));
        assert!(matches!(
            state.view().node(callout).unwrap().node(),
            Node::Callout(_)
        ));
    }

    fn arb_chars() -> impl proptest::strategy::Strategy<Value = Vec<char>> {
        use proptest::prelude::*;
        proptest::collection::vec(prop::sample::select(vec!['a', 'b', 'c']), 0..8)
    }

    proptest::proptest! {
        #[test]
        fn apply_char_sequence_never_panics_and_text_matches(chars in arb_chars()) {
            let mut state = ProjectedState::empty();
            let para = state
                .view()
                .root()
                .unwrap()
                .child_blocks()
                .next()
                .unwrap()
                .dot()
                .unwrap();
            for (i, c) in chars.iter().enumerate() {
                state.apply(seq_char(1 + i, *c)).expect("char applies");
            }
            let expected: String = chars.iter().collect();
            let got = state.view().node(para).unwrap().inline_text();
            proptest::prop_assert_eq!(got, expected);
        }
    }

    #[test]
    fn accessor_smoke_node_style_block_modifier_span() {
        use editor_crdt::LwwRegOp;
        use editor_model::{Anchor, Bias, ModifierAttrOp, NodeLwwOp, SpanOp};

        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let x = state.apply(seq_char(1, 'x')).unwrap().id;

        state
            .apply(EditOp::NodeStyle(NodeLwwOp {
                target: x,
                op: LwwRegOp::Set {
                    value: Some("mystyle".to_string()),
                },
            }))
            .unwrap();
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: para,
                modifier: Modifier::FontSize { value: 1400 },
            }))
            .unwrap();
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: x,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: x,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap();

        assert_eq!(state.node_styles().value_of(x), Some("mystyle".to_string()));
        assert_eq!(
            state
                .block_modifiers()
                .modifiers_of(para)
                .get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1400 })
        );
        assert!(state.spans().iter().count() > 0);
    }

    #[test]
    fn seq_flat_pos_identifies_char_and_del_removes_it() {
        use editor_crdt::ListOp;

        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        let a_dot = state.apply(seq_char(1, 'a')).unwrap().id;
        state.apply(seq_char(2, 'b')).unwrap();

        assert_eq!(state.view().node(para).unwrap().inline_text(), "ab");

        let pos = state.seq_flat_pos(a_dot).expect("dot exists in seq");
        state
            .apply(EditOp::Seq(ListOp::Del { pos, len: 1 }))
            .unwrap();
        assert_eq!(state.view().node(para).unwrap().inline_text(), "b");
    }

    #[test]
    fn apply_batch_equivalent_to_per_op() {
        let mut batched = ProjectedState::empty();
        let para = batched
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let batch_ops = batched
            .apply_batch(vec![seq_char(1, 'a'), seq_char(2, 'b'), seq_char(3, 'c')])
            .unwrap();
        assert_eq!(batch_ops.len(), 3);
        assert_eq!(batched.view().node(para).unwrap().inline_text(), "abc");

        let mut distinct: std::collections::HashSet<Dot> = std::collections::HashSet::new();
        for op in &batch_ops {
            assert!(distinct.insert(op.id), "returned dots must be distinct");
        }

        let mut per_op = ProjectedState::empty();
        let per_para = per_op
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let a = per_op.apply(seq_char(1, 'a')).unwrap();
        let b = per_op.apply(seq_char(2, 'b')).unwrap();
        let c = per_op.apply(seq_char(3, 'c')).unwrap();

        assert_eq!(per_para, para);
        assert_eq!(
            per_op.view().node(per_para).unwrap().inline_text(),
            batched.view().node(para).unwrap().inline_text()
        );

        assert_eq!(batch_ops[0].id, a.id);
        assert_eq!(batch_ops[1].id, b.id);
        assert_eq!(batch_ops[2].id, c.id);
    }

    #[test]
    fn apply_batch_returned_dots_resolve_in_seq() {
        let mut state = ProjectedState::empty();
        let ops = state
            .apply_batch(vec![seq_char(1, 'a'), seq_char(2, 'b'), seq_char(3, 'c')])
            .unwrap();
        let positions: Vec<usize> = ops
            .iter()
            .map(|op| {
                state
                    .seq_flat_pos(op.id)
                    .expect("returned dot resolves in seq")
            })
            .collect();
        assert_eq!(positions, vec![1, 2, 3]);
    }
}
