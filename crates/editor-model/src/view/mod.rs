use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::LazyLock;

use editor_crdt::Dot;

use crate::projection::{BlockPaths, ProjectedDoc};
use crate::schema::Schema;
use crate::seq::{BlockNode, Child, anchor_dot};
use crate::{AtomLeaf, Modifier, ModifierType, Node, NodeType, OwnModifier, SeqItem};

/// A read-only view over a `ProjectedDoc`. The flat tree gives O(1) node access by
/// `Dot` (node type, children, leaf items read straight from it), and the
/// incrementally-maintained `BlockPaths` gives O(1) parent links — so a `DocView` is
/// O(1) to construct, with no per-call structural rebuild.
pub struct DocView<'a> {
    doc: &'a ProjectedDoc,
    paths: std::borrow::Cow<'a, BlockPaths>,
}

impl<'a> DocView<'a> {
    /// Build a standalone view, deriving the parent index from the tree (`O(n)`). For
    /// callers that hold only a `ProjectedDoc` (a snapshot, tests). The editor's hot
    /// path uses `with_paths` to reuse the already-maintained index.
    pub fn new(doc: &'a ProjectedDoc) -> Self {
        Self {
            doc,
            paths: std::borrow::Cow::Owned(BlockPaths::from_tree(&doc.tree)),
        }
    }

    /// Build a view reusing an already-maintained parent index (`O(1)`). `paths` must
    /// match `doc`'s current tree.
    pub fn with_paths(doc: &'a ProjectedDoc, paths: &'a BlockPaths) -> Self {
        Self {
            doc,
            paths: std::borrow::Cow::Borrowed(paths),
        }
    }

    pub fn alias_classes(&self) -> &crate::AliasClasses {
        &self.doc.alias_classes
    }

    pub fn block_of(&self, leaf: Dot) -> Option<Dot> {
        self.paths.block_of(leaf)
    }

    pub fn parent_of(&self, block: Dot) -> Option<Dot> {
        self.paths.parent_of(block)
    }
}

#[derive(Clone, Copy)]
pub struct NodeView<'a> {
    view: &'a DocView<'a>,
    id: Dot,
}

#[derive(Clone, Copy)]
pub struct LeafView<'a> {
    view: &'a DocView<'a>,
    dot: Dot,
    item: &'a SeqItem,
    block: Dot,
}

pub enum ChildView<'a> {
    Block(NodeView<'a>),
    Leaf(LeafView<'a>),
}

impl<'a> DocView<'a> {
    pub fn roots(&'a self) -> impl Iterator<Item = NodeView<'a>> {
        self.doc
            .tree
            .root_node()
            .map(|r| NodeView {
                view: self,
                id: r.id,
            })
            .into_iter()
    }
    pub fn root(&'a self) -> Option<NodeView<'a>> {
        let r = self.doc.tree.root_node()?;
        (r.node_type == NodeType::Root).then_some(NodeView {
            view: self,
            id: r.id,
        })
    }
    pub fn node(&'a self, id: Dot) -> Option<NodeView<'a>> {
        self.doc.tree.get(id).map(|_| NodeView { view: self, id })
    }
    pub fn leaf(&'a self, id: Dot) -> Option<LeafView<'a>> {
        let block = self.paths.block_of(id)?;
        let item = self
            .doc
            .tree
            .get(block)?
            .children
            .iter()
            .find_map(|c| match c {
                Child::Leaf { id: lid, item } if *lid == id => Some(item),
                _ => None,
            })?;
        Some(LeafView {
            view: self,
            dot: id,
            item,
            block,
        })
    }
    /// The leaf's derived state located by dot: `block_of` plus a linear child
    /// scan for the slot. Cold paths only — hot readers hold the slot and use
    /// [`NodeView::leaf_state_at`].
    pub fn leaf_state_by_dot_slow(&'a self, dot: Dot) -> Option<LeafStateRef<'a>> {
        let block = self.paths.block_of(dot)?;
        let slot = self
            .doc
            .tree
            .get(block)?
            .children
            .iter()
            .position(|c| matches!(c, Child::Leaf { id, .. } if *id == dot))?;
        self.node(block)?.leaf_state_at(slot)
    }
    pub fn leaf_own_modifiers_by_dot_slow(&'a self, dot: Dot) -> Vec<Modifier> {
        self.leaf_state_by_dot_slow(dot)
            .map(|s| s.own_modifiers())
            .unwrap_or_default()
    }
}

impl<'a> NodeView<'a> {
    fn tree_node(&self) -> Option<&'a BlockNode> {
        self.view.doc.tree.get(self.id)
    }
    pub fn id(&self) -> Dot {
        self.id
    }
    pub fn child_count(&self) -> usize {
        self.tree_node().map_or(0, |n| n.children.len())
    }
    /// Number of direct *leaf* children (excludes nested blocks). `O(1)` via the
    /// `ChildList` order-statistics summary.
    pub fn leaf_child_count(&self) -> usize {
        self.tree_node().map_or(0, |n| n.children.leaf_count())
    }
    pub fn dot(&self) -> Option<Dot> {
        anchor_dot(self.id)
    }
    pub fn node_type(&self) -> NodeType {
        self.tree_node()
            .map(|n| n.node_type)
            .unwrap_or(NodeType::Root)
    }
    pub fn spec(&self) -> &'static crate::schema::NodeSpec {
        Schema::node_spec(self.node_type())
    }
    pub fn node(&self) -> Node {
        self.dot()
            .and_then(|d| self.view.doc.node_attrs.get(&d).cloned())
            .unwrap_or_else(|| self.node_type().into_node())
    }
    pub fn parent(&self) -> Option<NodeView<'a>> {
        self.view
            .paths
            .parent_of(self.id)
            .and_then(|p| self.view.node(p))
    }
    pub fn children(&self) -> impl Iterator<Item = ChildView<'a>> {
        let view = self.view;
        let block = self.id;
        self.tree_node()
            .into_iter()
            .flat_map(|n| n.children.iter())
            .map(move |c| match c {
                Child::Block(id) => ChildView::Block(NodeView { view, id: *id }),
                Child::Leaf { id, item } => ChildView::Leaf(LeafView {
                    view,
                    dot: *id,
                    item,
                    block,
                }),
            })
    }
    pub fn child_blocks(&self) -> impl Iterator<Item = NodeView<'a>> {
        let view = self.view;
        self.tree_node()
            .into_iter()
            .flat_map(|n| n.children.iter())
            .filter_map(move |c| match c {
                Child::Block(id) => Some(NodeView { view, id: *id }),
                Child::Leaf { .. } => None,
            })
    }
    pub fn first_child(&self) -> Option<ChildView<'a>> {
        self.children().next()
    }
    pub fn last_child(&self) -> Option<ChildView<'a>> {
        self.children().last()
    }
    pub fn child_at(&self, index: usize) -> Option<ChildView<'a>> {
        // O(log K) via the `ChildList` order-statistics tree, not `O(index)` via
        // `children().nth`. On a large block (a paragraph with thousands of inline
        // leaves) the linear form makes every caret-local operation — selection
        // capture, modifier resolution, inserted-leaf lookup — `O(block)`.
        let view = self.view;
        let block = self.id;
        let c = self.tree_node()?.children.get(index)?;
        Some(match c {
            Child::Block(id) => ChildView::Block(NodeView { view, id: *id }),
            Child::Leaf { id, item } => ChildView::Leaf(LeafView {
                view,
                dot: *id,
                item,
                block,
            }),
        })
    }
    pub fn ancestors(&self) -> impl Iterator<Item = NodeView<'a>> {
        let mut cur = Some(*self);
        std::iter::from_fn(move || {
            let node = cur?;
            cur = node.parent();
            Some(node)
        })
    }
    pub fn descendants(&self) -> impl Iterator<Item = ChildView<'a>> {
        let mut stack: Vec<ChildView<'a>> = self.children().collect();
        stack.reverse();
        std::iter::from_fn(move || {
            let next = stack.pop()?;
            if let ChildView::Block(b) = &next {
                let mut kids: Vec<ChildView<'a>> = b.children().collect();
                kids.reverse();
                stack.extend(kids);
            }
            Some(next)
        })
    }
    pub fn index(&self) -> Option<usize> {
        let parent = self.parent()?;
        parent.children().position(|c| match c {
            ChildView::Block(b) => b.id == self.id,
            ChildView::Leaf(_) => false,
        })
    }
}

impl<'a> LeafView<'a> {
    pub fn dot(&self) -> Dot {
        self.dot
    }
    pub fn node_type(&self) -> NodeType {
        self.item().as_child_type().unwrap_or(NodeType::Unknown)
    }
    pub fn item(&self) -> &'a SeqItem {
        self.item
    }
    pub fn as_char(&self) -> Option<char> {
        match self.item {
            SeqItem::Char(c) => Some(*c),
            _ => None,
        }
    }
    pub fn as_atom(&self) -> Option<&'a AtomLeaf> {
        match self.item {
            SeqItem::Atom(a) => Some(a),
            _ => None,
        }
    }
    pub fn node(&self) -> Option<Node> {
        let atom = self.as_atom()?.clone();
        Some(
            self.view
                .doc
                .node_attrs
                .get(&self.dot)
                .cloned()
                .unwrap_or_else(|| atom.into_node()),
        )
    }

    pub fn is_charlike(&self) -> bool {
        self.as_char().is_some() || matches!(self.node_type(), NodeType::Tab | NodeType::HardBreak)
    }
    pub fn parent(&self) -> Option<NodeView<'a>> {
        self.view.node(self.block)
    }
}

static EMPTY_EFF: LazyLock<BTreeMap<ModifierType, Modifier>> = LazyLock::new(BTreeMap::new);
static EMPTY_OWN: LazyLock<BTreeMap<ModifierType, OwnModifier>> = LazyLock::new(BTreeMap::new);

pub enum InlineKind {
    Char {
        byte_range: Range<usize>,
        char_index: usize,
    },
    Atom(NodeType),
}

pub struct InlineItem<'a> {
    pub dot: Dot,
    pub kind: InlineKind,
    pub effective: &'a BTreeMap<ModifierType, Modifier>,
    pub own_modifiers: &'a BTreeMap<ModifierType, OwnModifier>,
}

/// A leaf's derived state read straight from its owning run segment: the
/// effective and own modifier maps, shared by every leaf of the segment.
#[derive(Clone, Copy)]
pub struct LeafStateRef<'a> {
    pub eff: &'a BTreeMap<ModifierType, Modifier>,
    pub own: &'a BTreeMap<ModifierType, OwnModifier>,
}

impl LeafStateRef<'_> {
    pub fn own_modifiers(&self) -> Vec<Modifier> {
        self.own.values().map(|o| o.value.clone()).collect()
    }
}

impl<'a> NodeView<'a> {
    pub fn effective(&self) -> &'a BTreeMap<ModifierType, Modifier> {
        self.view
            .doc
            .block_effective
            .get(&self.id)
            .unwrap_or(&EMPTY_EFF)
    }
    pub fn block_modifier(&self, ty: ModifierType) -> Option<&'a Modifier> {
        let dot = anchor_dot(self.id)?;
        self.view.doc.block_modifiers.get(&dot)?.get(&ty)
    }
    pub fn carry_modifiers(&self) -> BTreeMap<ModifierType, Modifier> {
        self.view.doc.carry_modifiers(self.id)
    }
    /// The node's run segments as `(effective modifiers, leaf count)` groups in
    /// leaf order, straight from the authoritative segment index — segments split
    /// finer than effective runs (by style/covering), but consumers that aggregate
    /// per leaf are unaffected.
    pub fn run_groups(
        &self,
    ) -> impl Iterator<Item = (&'a BTreeMap<ModifierType, Modifier>, usize)> + 'a {
        self.view
            .doc
            .seg_index
            .group_iter(self.id)
            .map(|s| (s.eff.as_ref(), s.count))
    }
    pub fn inline_text(&self) -> String {
        self.tree_node()
            .into_iter()
            .flat_map(|n| n.children.iter())
            .filter_map(|c| match c {
                Child::Leaf {
                    item: SeqItem::Char(ch),
                    ..
                } => Some(*ch),
                _ => None,
            })
            .collect()
    }
    /// The run segment groups of this block in leaf order, straight from the
    /// maintained segment index — the segment-key equivalent of [`run_groups`].
    /// [`run_groups`]: NodeView::run_groups
    pub fn seg_groups(&self) -> impl Iterator<Item = &'a crate::span::Seg> + 'a {
        self.view.doc.seg_index.group_iter(self.id)
    }
    /// The derived state of the leaf at direct child slot `child_slot`, or `None`
    /// when the slot holds a block or is out of range. `O(log K + log segs)`.
    pub fn leaf_state_at(&self, child_slot: usize) -> Option<LeafStateRef<'a>> {
        let node = self.tree_node()?;
        // A block slot's leaf ordinal aliases the next leaf, so confirm the slot
        // actually holds a leaf before trusting the ordinal.
        match node.children.get(child_slot)? {
            Child::Leaf { .. } => {}
            Child::Block(_) => return None,
        }
        let ordinal = node.children.leaf_ordinal_at(child_slot);
        let (seg, _) = self.view.doc.seg_index.seg_at(self.id, ordinal)?;
        Some(LeafStateRef {
            eff: seg.eff.as_ref(),
            own: seg.own.as_ref(),
        })
    }
    pub fn leaf_own_modifiers_at(&self, child_slot: usize) -> Vec<Modifier> {
        self.leaf_state_at(child_slot)
            .map(|s| s.own_modifiers())
            .unwrap_or_default()
    }
    pub fn inline(&self) -> Vec<InlineItem<'a>> {
        let doc = self.view.doc;
        let Some(node) = self.tree_node() else {
            return Vec::new();
        };
        // Serve effective/own from the authoritative run segments, expanding each seg
        // by its leaf count.
        let mut segs = doc.seg_index.group_iter(self.id);
        let mut cur = segs.next().map(|s| (s, s.count));
        let mut out = Vec::new();
        let mut byte = 0usize;
        let mut char_index = 0usize;
        for c in node.children.iter() {
            let Child::Leaf { id: d, item } = c else {
                continue;
            };
            let (effective, own_modifiers) = {
                while matches!(cur, Some((_, 0))) {
                    cur = segs.next().map(|s| (s, s.count));
                }
                match &mut cur {
                    Some((seg, rem)) => {
                        *rem -= 1;
                        (seg.eff.as_ref(), seg.own.as_ref())
                    }
                    None => (&*EMPTY_EFF, &*EMPTY_OWN),
                }
            };
            let kind = match item {
                SeqItem::Char(ch) => {
                    let len = ch.len_utf8();
                    let k = InlineKind::Char {
                        byte_range: byte..byte + len,
                        char_index,
                    };
                    byte += len;
                    char_index += 1;
                    k
                }
                _ => InlineKind::Atom(item.as_child_type().unwrap_or(NodeType::Unknown)),
            };
            out.push(InlineItem {
                dot: *d,
                kind,
                effective,
                own_modifiers,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::{DocLogs, project_document};
    use crate::{
        AliasLog, Anchor, Bias, Modifier, ModifierAttrLog, ModifierAttrOp, ModifierType, NodeAttr,
        NodeAttrLog, NodeAttrOp, SpanLog, SpanOp, TableNodeAttr,
    };
    use editor_crdt::{InputEvent, ListOp, build_oplog};

    fn events(items: &[(Dot, SeqItem)]) -> Vec<InputEvent<SeqItem>> {
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

    fn logs_of(items: &[(Dot, SeqItem)]) -> DocLogs {
        DocLogs {
            seq: build_oplog(&events(items)),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
            aliases: AliasLog::new(),
        }
    }

    /// A leaf's effective modifiers via the segment index (cold dot lookup).
    fn leaf_eff<'a>(view: &'a DocView<'a>, dot: Dot) -> &'a BTreeMap<ModifierType, Modifier> {
        view.leaf_state_by_dot_slow(dot)
            .map(|s| s.eff)
            .unwrap_or(&EMPTY_EFF)
    }
    /// A leaf's own modifiers via the segment index (cold dot lookup).
    fn leaf_own<'a>(view: &'a DocView<'a>, dot: Dot) -> &'a BTreeMap<ModifierType, OwnModifier> {
        view.leaf_state_by_dot_slow(dot)
            .map(|s| s.own)
            .unwrap_or(&EMPTY_OWN)
    }
    fn own_val<'a>(view: &'a DocView<'a>, dot: Dot, ty: ModifierType) -> Option<&'a Modifier> {
        leaf_own(view, dot).get(&ty).map(|o| &o.value)
    }

    fn nested_doc() -> ProjectedDoc {
        let para = Dot::new(1, 1);
        let bq = Dot::new(1, 4);
        let bq_para = Dot::new(1, 5);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('i')),
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (
                bq_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 6), SeqItem::Char('y')),
        ];
        project_document(&logs_of(&elems)).unwrap()
    }

    fn doc_with_table_and_image() -> ProjectedDoc {
        let image = Dot::new(1, 1);
        let hr = Dot::new(1, 2);
        let table = Dot::new(1, 3);
        let row = Dot::new(1, 4);
        let cell = Dot::new(1, 5);
        let cell_para = Dot::new(1, 6);
        let img_node = match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        let elems = vec![
            (
                image,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node },
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: crate::nodes::HorizontalRuleVariant::default(),
                    },
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (
                row,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![Dot::ROOT, table],
                    attrs: vec![],
                },
            ),
            (
                cell,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![Dot::ROOT, table, row],
                    attrs: vec![],
                },
            ),
            (
                cell_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, table, row, cell],
                    attrs: vec![],
                },
            ),
            (
                Dot::new(1, 7),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
        ];
        let mut l = logs_of(&elems);
        l.node_attrs = NodeAttrLog::new()
            .apply(
                Dot::new(2, 0),
                NodeAttrOp {
                    target: table,
                    attr: NodeAttr::Table {
                        attr: TableNodeAttr::Proportion(80),
                    },
                },
            )
            .unwrap();
        project_document(&l).unwrap()
    }

    #[test]
    fn root_children_and_leaf_nav() {
        let pd = nested_doc();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();
        assert_eq!(root.node_type(), NodeType::Root);
        let blocks: Vec<NodeType> = root.child_blocks().map(|b| b.node_type()).collect();
        assert_eq!(
            blocks,
            vec![
                NodeType::Paragraph,
                NodeType::Blockquote,
                NodeType::Paragraph
            ]
        );
        let h = view.leaf(Dot::new(1, 2)).unwrap();
        assert_eq!(h.as_char(), Some('H'));
        let para = h.parent().unwrap();
        assert_eq!(para.node_type(), NodeType::Paragraph);
        let anc: Vec<NodeType> = para.ancestors().map(|n| n.node_type()).collect();
        assert_eq!(anc, vec![NodeType::Paragraph, NodeType::Root]);
    }

    #[test]
    fn typed_block_node_and_atom() {
        let pd = doc_with_table_and_image();
        let view = DocView::new(&pd);
        let table = view
            .roots()
            .next()
            .unwrap()
            .child_blocks()
            .find(|b| b.node_type() == NodeType::Table)
            .unwrap();
        if let Node::Table(t) = table.node() {
            assert_eq!(*t.proportion.get(), 80);
        } else {
            panic!()
        }
        let hr = view.leaf(Dot::new(1, 2)).unwrap();
        assert!(matches!(
            hr.as_atom(),
            Some(AtomLeaf::HorizontalRule { .. })
        ));
    }

    /// An unknown leaf's resolved `node_type()` must be the real `NodeType::Unknown`
    /// placeholder, never the pre-Task-1b `NodeType::Root` sentinel (which would
    /// wrongly imply a dangling block reference).
    #[test]
    fn unknown_leaf_node_type_is_unknown_not_root_sentinel() {
        let para = Dot::new(1, 1);
        let unknown = Dot::new(1, 2);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (
                unknown,
                SeqItem::Unknown {
                    tag: 999,
                    bytes: vec![0xAA],
                },
            ),
        ];
        let pd = project_document(&logs_of(&elems)).unwrap();
        let view = DocView::new(&pd);
        let leaf = view.leaf(unknown).unwrap();
        assert_eq!(leaf.node_type(), NodeType::Unknown);
        assert_ne!(leaf.node_type(), NodeType::Root);
    }

    #[test]
    fn derived_block_node_default_and_dot_none() {
        let pd = nested_doc();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();
        let derived = root.child_blocks().last().unwrap();
        assert_eq!(derived.node_type(), NodeType::Paragraph);
        assert_eq!(derived.dot(), None);
        assert_eq!(derived.node(), NodeType::Paragraph.into_node());
    }

    #[test]
    fn child_at_index_first_last() {
        let pd = nested_doc();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();
        let para = root
            .child_blocks()
            .find(|b| b.dot() == Some(Dot::new(1, 1)))
            .unwrap();

        let first = match para.first_child().unwrap() {
            ChildView::Leaf(l) => l,
            ChildView::Block(_) => panic!(),
        };
        assert_eq!(first.dot(), Dot::new(1, 2));
        assert_eq!(first.as_char(), Some('H'));

        let last = match para.last_child().unwrap() {
            ChildView::Leaf(l) => l,
            ChildView::Block(_) => panic!(),
        };
        assert_eq!(last.dot(), Dot::new(1, 3));
        assert_eq!(last.as_char(), Some('i'));

        let at1 = match para.child_at(1).unwrap() {
            ChildView::Leaf(l) => l,
            ChildView::Block(_) => panic!(),
        };
        assert_eq!(at1.dot(), Dot::new(1, 3));
        assert_eq!(at1.as_char(), Some('i'));

        let bq = root
            .child_blocks()
            .find(|b| b.node_type() == NodeType::Blockquote)
            .unwrap();
        assert_eq!(bq.index(), Some(1));
    }

    fn doc_spanned_xy() -> ProjectedDoc {
        let para = Dot::new(1, 1);
        let x = Dot::new(1, 2);
        let y = Dot::new(1, 3);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (x, SeqItem::Char('x')),
            (y, SeqItem::Char('y')),
        ];
        let mut l = logs_of(&elems);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: x,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: x,
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                },
            )
            .unwrap()
            .apply(
                Dot::new(3, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: y,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: y,
                        bias: Bias::After,
                    },
                    modifier: Modifier::Italic,
                },
            )
            .unwrap();
        project_document(&l).unwrap()
    }

    fn doc_inline_mixed() -> ProjectedDoc {
        let para = Dot::new(1, 1);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('가')),
            (Dot::new(1, 4), SeqItem::Char('b')),
            (Dot::new(1, 5), SeqItem::Atom(AtomLeaf::HardBreak)),
            (Dot::new(1, 6), SeqItem::Char('c')),
        ];
        project_document(&logs_of(&elems)).unwrap()
    }

    fn doc_empty_paragraph() -> ProjectedDoc {
        let para = Dot::new(1, 1);
        let elems = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![Dot::ROOT],
                attrs: vec![],
            },
        )];
        project_document(&logs_of(&elems)).unwrap()
    }

    #[test]
    fn empty_paragraph_inline_empty() {
        let pd = doc_empty_paragraph();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();
        let para = root.child_blocks().next().unwrap();
        assert_eq!(para.node_type(), NodeType::Paragraph);
        assert!(para.inline().is_empty());
        assert_eq!(para.inline_text(), "");
    }

    #[test]
    fn no_modifier_leaf_empty_fallback() {
        let pd = nested_doc();
        let view = DocView::new(&pd);
        let h = Dot::new(1, 2);
        assert_eq!(view.leaf(h).unwrap().as_char(), Some('H'));
        assert!(leaf_eff(&view, h).is_empty());
        assert!(leaf_own(&view, h).is_empty());
        assert_eq!(own_val(&view, h, ModifierType::Bold), None);

        let i = Dot::new(1, 3);
        assert_eq!(view.leaf(i).unwrap().as_char(), Some('i'));
        assert!(leaf_eff(&view, i).is_empty());
        assert!(leaf_own(&view, i).is_empty());
    }

    #[test]
    fn leaf_modifier_accessors() {
        let pd = doc_spanned_xy();
        let view = DocView::new(&pd);
        let x = Dot::new(1, 2);
        assert_eq!(own_val(&view, x, ModifierType::Bold), Some(&Modifier::Bold));
        let y = Dot::new(1, 3);
        assert_eq!(
            own_val(&view, y, ModifierType::Italic),
            Some(&Modifier::Italic)
        );
        assert!(!leaf_eff(&view, x).is_empty());
    }

    #[test]
    fn inline_text_offsets_and_runs() {
        let pd = doc_inline_mixed();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let text = para.inline_text();
        let items = para.inline();
        let chars: Vec<char> = items
            .iter()
            .filter_map(|it| match it.kind {
                InlineKind::Char { .. } => view.leaf(it.dot).unwrap().as_char(),
                InlineKind::Atom(_) => None,
            })
            .collect();
        assert_eq!(text.chars().count(), chars.len());
        assert!(para.run_groups().count() >= 1);

        let mut last_char_index: Option<usize> = None;
        let mut prev_byte_end = 0usize;
        for it in &items {
            match &it.kind {
                InlineKind::Char {
                    byte_range,
                    char_index,
                } => {
                    if let Some(prev) = last_char_index {
                        assert!(*char_index > prev);
                    }
                    last_char_index = Some(*char_index);
                    assert_eq!(byte_range.start, prev_byte_end);
                    let ch = view.leaf(it.dot).unwrap().as_char().unwrap();
                    assert_eq!(byte_range.len(), ch.len_utf8());
                    prev_byte_end = byte_range.end;
                }
                InlineKind::Atom(ty) => {
                    assert_eq!(*ty, NodeType::HardBreak);
                }
            }
        }
        assert_eq!(text.len(), prev_byte_end);
    }

    #[test]
    fn inline_matches_leaf_state_at() {
        let pd = doc_spanned_xy();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let items = para.inline();
        assert_eq!(items.len(), 2);
        // Every child of this paragraph is a leaf, so the inline index is the
        // child slot.
        for (slot, it) in items.iter().enumerate() {
            let st = para.leaf_state_at(slot).expect("leaf slot resolves");
            assert_eq!(it.effective, st.eff);
            assert_eq!(it.own_modifiers, st.own);
            let by_dot = view.leaf_state_by_dot_slow(it.dot).expect("dot resolves");
            assert_eq!(by_dot.eff, st.eff);
            assert_eq!(by_dot.own, st.own);
        }
        assert!(para.leaf_state_at(items.len()).is_none());
    }

    #[test]
    fn inline_equals_dot_keyed_state() {
        let pd = doc_spanned_xy();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        for it in para.inline() {
            assert_eq!(it.effective, leaf_eff(&view, it.dot));
            assert_eq!(it.own_modifiers, leaf_own(&view, it.dot));
        }
    }

    #[test]
    fn measure_access_pattern_smoke() {
        let pd = nested_doc();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();
        let mut blocks_seen = 0usize;
        for block in root.descendants() {
            if let ChildView::Block(b) = block {
                blocks_seen += 1;
                let _ = b.effective();
                let _ = b.spec();
                let last_is_pagebreak = matches!(
                    b.last_child(),
                    Some(ChildView::Leaf(l)) if l.node_type() == NodeType::PageBreak
                );
                let _ = last_is_pagebreak;
                for it in b.inline() {
                    let _ = (it.dot, &it.effective, &it.own_modifiers);
                }
            }
        }
        assert!(blocks_seen >= 3);
    }

    fn arb_projected_doc() -> impl proptest::strategy::Strategy<Value = ProjectedDoc> {
        use proptest::prelude::*;
        let para_strat = ("[a-c]{0,4}", proptest::bool::ANY);
        proptest::collection::vec(para_strat, 1..=2).prop_map(|paras| {
            let mut elems: Vec<(Dot, SeqItem)> = vec![];
            let mut next: u64 = 1;
            let mut bold_leaf: Option<Dot> = None;
            for (s, want_bold) in &paras {
                let para = Dot::new(1, next);
                next += 1;
                elems.push((
                    para,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![Dot::ROOT],
                        attrs: vec![],
                    },
                ));
                for ch in s.chars() {
                    let leaf = Dot::new(1, next);
                    next += 1;
                    elems.push((leaf, SeqItem::Char(ch)));
                    if *want_bold && bold_leaf.is_none() {
                        bold_leaf = Some(leaf);
                    }
                }
            }
            let mut l = logs_of(&elems);
            if let Some(d) = bold_leaf {
                l.spans = l
                    .spans
                    .apply(
                        Dot::new(3, 0),
                        SpanOp::AddSpan {
                            start: Anchor {
                                id: d,
                                bias: Bias::Before,
                            },
                            end: Anchor {
                                id: d,
                                bias: Bias::After,
                            },
                            modifier: Modifier::Bold,
                        },
                    )
                    .unwrap();
            }
            project_document(&l).unwrap()
        })
    }

    proptest::proptest! {
        #[test]
        fn every_real_block_and_leaf_reachable_once(doc in arb_projected_doc()) {
            let view = DocView::new(&doc);
            let mut ids = std::collections::HashSet::new();
            fn count(
                tree: &crate::seq::BlockTree,
                b: &crate::seq::BlockNode,
                ids: &mut std::collections::HashSet<Dot>,
            ) -> bool {
                let fresh = ids.insert(b.id);
                let mut ok = fresh;
                for c in &b.children {
                    if let crate::seq::Child::Block(cid) = c
                        && let Some(cb) = tree.get(*cid)
                    {
                        ok &= count(tree, cb, ids);
                    }
                }
                ok
            }
            let mut unique = true;
            if let Some(r) = doc.tree.root_node() {
                unique &= count(&doc.tree, r, &mut ids);
            }
            proptest::prop_assert!(unique, "duplicate block ElemId");

            // Every block's run-segment groups partition its leaves, so summing the
            // groups' leaf counts across all blocks recovers the document's leaf count.
            let mut reconstructed = 0usize;
            let mut total_leaves = 0usize;
            for id in ids.iter() {
                if let Some(nv) = view.node(*id) {
                    let bytes: usize = nv
                        .inline()
                        .iter()
                        .filter_map(|it| match &it.kind {
                            InlineKind::Char { byte_range, .. } => Some(byte_range.len()),
                            _ => None,
                        })
                        .sum();
                    proptest::prop_assert_eq!(nv.inline_text().len(), bytes);
                    reconstructed += nv.run_groups().map(|(_, n)| n).sum::<usize>();
                }
                if let Some(b) = doc.tree.get(*id) {
                    total_leaves += b
                        .children
                        .iter()
                        .filter(|c| matches!(c, crate::seq::Child::Leaf { .. }))
                        .count();
                }
            }
            proptest::prop_assert_eq!(reconstructed, total_leaves);
        }
    }

    #[test]
    fn block_modifier_explicit_only_not_inherited() {
        let table = Dot::new(1, 1);
        let row = Dot::new(1, 2);
        let cell_with_bg = Dot::new(1, 3);
        let cell_without_bg = Dot::new(1, 4);
        let para_a = Dot::new(1, 5);
        let para_b = Dot::new(1, 6);
        let elems = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (
                row,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![Dot::ROOT, table],
                    attrs: vec![],
                },
            ),
            (
                cell_with_bg,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![Dot::ROOT, table, row],
                    attrs: vec![],
                },
            ),
            (
                para_a,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, table, row, cell_with_bg],
                    attrs: vec![],
                },
            ),
            (
                cell_without_bg,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![Dot::ROOT, table, row],
                    attrs: vec![],
                },
            ),
            (
                para_b,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, table, row, cell_without_bg],
                    attrs: vec![],
                },
            ),
        ];
        let mut l = logs_of(&elems);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target: cell_with_bg,
                    modifier: Modifier::BackgroundColor {
                        value: "#fff".into(),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);

        let cell_a = view.node(cell_with_bg).unwrap();
        assert_eq!(
            cell_a.block_modifier(ModifierType::BackgroundColor),
            Some(&Modifier::BackgroundColor {
                value: "#fff".into()
            }),
        );

        let cell_b = view.node(cell_without_bg).unwrap();
        assert_eq!(cell_b.block_modifier(ModifierType::BackgroundColor), None,);
    }
}
