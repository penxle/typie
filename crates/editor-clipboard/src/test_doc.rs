use editor_crdt::{Dot, ListOp, OpGraph};
use editor_model::{Anchor, AtomLeaf, Bias, EditOp, Modifier, Node, NodeType, SeqItem, SpanOp};
use editor_state::Selection;
use editor_state::{ProjectedState, State};

pub(crate) struct DocBuilder {
    graph: OpGraph<EditOp>,
    pos: usize,
}

impl DocBuilder {
    pub(crate) fn new() -> Self {
        Self {
            graph: OpGraph::<EditOp>::with_actor(1),
            pos: 0,
        }
    }

    fn ins(&mut self, item: SeqItem) -> Dot {
        let dot = self
            .graph
            .add_mut(EditOp::Seq(ListOp::Ins {
                pos: self.pos,
                item,
            }))
            .expect("local insert never conflicts")
            .id;
        self.pos += 1;
        dot
    }

    pub(crate) fn block(&mut self, node_type: NodeType, parents: &[Dot]) -> Dot {
        self.ins(SeqItem::Block {
            node_type,
            parents: parents.to_vec(),
            attrs: vec![],
        })
    }

    pub(crate) fn text(&mut self, s: &str) -> Vec<Dot> {
        s.chars().map(|c| self.ins(SeqItem::Char(c))).collect()
    }

    pub(crate) fn atom(&mut self, leaf: AtomLeaf, parents: &[Dot]) -> Dot {
        let item = if leaf.is_block_level() {
            SeqItem::BlockAtom {
                leaf,
                parents: parents.to_vec(),
            }
        } else {
            SeqItem::Atom(leaf)
        };
        self.ins(item)
    }

    pub(crate) fn image(&mut self, parents: &[Dot]) -> Dot {
        let node = match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        self.atom(AtomLeaf::Image { node }, parents)
    }

    pub(crate) fn horizontal_rule(&mut self, parents: &[Dot]) -> Dot {
        self.atom(
            AtomLeaf::HorizontalRule {
                variant: editor_model::HorizontalRuleVariant::default(),
            },
            parents,
        )
    }

    pub(crate) fn span(&mut self, first: Dot, last: Dot, modifier: Modifier) {
        self.graph
            .add_mut(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: first,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: last,
                    bias: Bias::After,
                },
                modifier,
            }))
            .expect("local span never conflicts");
    }

    pub(crate) fn projected(mut self) -> ProjectedState {
        self.graph.commit_mut();
        ProjectedState::from_graph(self.graph).expect("template always projects")
    }

    pub(crate) fn finish(self, selection: Option<Selection>) -> State {
        State::new(self.projected(), selection)
    }
}
