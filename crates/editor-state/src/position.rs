use std::cmp::Ordering;

use editor_common::StrExt;
use editor_crdt::Dot;
use editor_macros::ffi;
use editor_model::{ChildView, DocView};
use editor_resource::Resource;
use serde::{Deserialize, Serialize};

use crate::affinity::Affinity;
use crate::classify;
use crate::selection::Selection;

#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Position {
    pub node: Dot,
    pub offset: usize,
    pub affinity: Affinity,
}

impl Position {
    pub fn new(node: Dot, offset: usize) -> Self {
        Self {
            node,
            offset,
            affinity: Affinity::default(),
        }
    }
}

pub struct ResolvedPosition<'a> {
    view: &'a DocView<'a>,
    position: Position,
    path: Vec<usize>,
}

impl Position {
    pub fn resolve<'a>(&self, view: &'a DocView<'a>) -> Option<ResolvedPosition<'a>> {
        let node = view.node(self.node)?;
        if self.offset > node.child_count() {
            return None;
        }
        let mut chain: Vec<usize> = node.ancestors().filter_map(|n| n.index()).collect();
        chain.reverse();
        chain.push(self.offset);
        Some(ResolvedPosition {
            view,
            position: *self,
            path: chain,
        })
    }
}

impl From<&ResolvedPosition<'_>> for Position {
    fn from(r: &ResolvedPosition<'_>) -> Self {
        r.position()
    }
}

/// Inline leaf ids (chars/atoms) fully covered by the range `[from, to]`, in
/// document order. The projected model has no text nodes, so this returns the
/// loose leaf ids themselves.
pub fn inline_leaf_dots_in_range(view: &DocView, from: &Position, to: &Position) -> Vec<Dot> {
    let Some(rs) = Selection::new(*from, *to).resolve(view) else {
        return Vec::new();
    };
    let from = rs.from().path();
    let to = rs.to().path();

    let mut blocks = Vec::new();
    if let Some(root) = view.root() {
        blocks.push(root);
        for d in root.descendants() {
            if let ChildView::Block(b) = d {
                blocks.push(b);
            }
        }
    }

    let mut out = Vec::new();
    for block in blocks {
        let mut base: Vec<usize> = block.ancestors().filter_map(|n| n.index()).collect();
        base.reverse();
        for (i, child) in block.children().enumerate() {
            let ChildView::Leaf(l) = child else { continue };
            if crate::traversal::leaf_slot_is_covered(i, &base, from, to) {
                out.push(l.dot());
            }
        }
    }
    out
}

impl<'a> ResolvedPosition<'a> {
    pub fn view(&self) -> &'a DocView<'a> {
        self.view
    }
    pub fn node(&self) -> Dot {
        self.position.node
    }
    pub fn position(&self) -> Position {
        self.position
    }
    pub fn offset(&self) -> usize {
        self.position.offset
    }
    pub fn affinity(&self) -> Affinity {
        self.position.affinity
    }
    pub fn path(&self) -> &[usize] {
        &self.path
    }
    pub fn is_inline_position(&self) -> bool {
        classify::is_inline_position(self)
    }
}

impl<'a> ResolvedPosition<'a> {
    fn grapheme_boundaries(&self, resource: &Resource) -> Vec<usize> {
        let Some(node) = self.view.node(self.position.node) else {
            return vec![0];
        };
        let children: Vec<ChildView<'a>> = node.children().collect();
        let mut boundaries = vec![0usize];
        let mut i = 0usize;
        while i < children.len() {
            let run_start = i;
            let mut run = String::new();
            while i < children.len() {
                if let ChildView::Leaf(l) = &children[i]
                    && let Some(c) = l.as_char()
                {
                    run.push(c);
                    i += 1;
                } else {
                    break;
                }
            }
            if i > run_start {
                for byte_off in resource.segmenters.grapheme.as_borrowed().segment_str(&run) {
                    boundaries.push(run_start + run.nth_byte_char_offset(byte_off));
                }
            } else {
                boundaries.push(i);
                boundaries.push(i + 1);
                i += 1;
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        boundaries
    }

    pub fn snap_to_grapheme(&self, resource: &Resource) -> ResolvedPosition<'a> {
        let boundaries = self.grapheme_boundaries(resource);
        let offset = self.position.offset;
        if boundaries.contains(&offset) {
            return self.position.resolve(self.view).unwrap();
        }
        let snapped = match self.position.affinity {
            Affinity::Upstream => boundaries
                .iter()
                .copied()
                .rfind(|&b| b <= offset)
                .unwrap_or(0),
            Affinity::Downstream => boundaries
                .iter()
                .copied()
                .find(|&b| b >= offset)
                .unwrap_or(*boundaries.last().unwrap_or(&0)),
        };
        Position {
            node: self.position.node,
            offset: snapped,
            affinity: self.position.affinity,
        }
        .resolve(self.view)
        .unwrap()
    }

    pub fn next_grapheme(&self, resource: &Resource) -> Option<ResolvedPosition<'a>> {
        let boundaries = self.grapheme_boundaries(resource);
        let next = boundaries.into_iter().find(|&b| b > self.position.offset)?;
        Position::new(self.position.node, next).resolve(self.view)
    }

    pub fn prev_grapheme(&self, resource: &Resource) -> Option<ResolvedPosition<'a>> {
        if self.position.offset == 0 {
            return None;
        }
        let boundaries = self.grapheme_boundaries(resource);
        let prev = boundaries
            .into_iter()
            .rfind(|&b| b < self.position.offset)?;
        Position::new(self.position.node, prev).resolve(self.view)
    }
}

impl PartialEq for ResolvedPosition<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.position.affinity == other.position.affinity
    }
}
impl Eq for ResolvedPosition<'_> {}
impl PartialOrd for ResolvedPosition<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ResolvedPosition<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path
            .cmp(&other.path)
            .then_with(|| self.position.affinity.cmp(&other.position.affinity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        DocLogs, ModifierAttrLog, NodeAttrLog, NodeType, ProjectedDoc, SeqItem, SpanLog,
        project_document,
    };

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
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
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
        }
    }

    fn two_paras() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 5);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('i')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 6), SeqItem::Char('y')),
        ];
        (project_document(&logs(&items)).unwrap(), p1, p2)
    }

    #[test]
    fn resolve_and_order() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let a = Position::new(p1, 1).resolve(&view).unwrap();
        let b = Position::new(p2, 0).resolve(&view).unwrap();
        assert_eq!(a.offset(), 1);
        assert!(a < b);
    }

    #[test]
    fn resolve_exposes_accessors() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let r = Position::new(p1, 1).resolve(&view).unwrap();
        assert_eq!(r.node(), p1);
        assert_eq!(r.offset(), 1);
        assert_eq!(r.affinity(), Affinity::default());
        assert_eq!(r.position(), Position::new(p1, 1));
        assert_eq!(r.path(), &[0, 1]);
        assert!(r.view().node(p1).is_some());
    }

    #[test]
    fn resolve_accepts_end_boundary() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        assert!(Position::new(p1, 2).resolve(&view).is_some());
    }

    #[test]
    fn resolve_orders_offsets_within_block() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let a = Position::new(p1, 0).resolve(&view);
        let b = Position::new(p1, 1).resolve(&view);
        assert!(a < b);
    }

    #[test]
    fn resolve_orders_affinity_at_same_point() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let up = Position {
            node: p1,
            offset: 1,
            affinity: Affinity::Upstream,
        }
        .resolve(&view)
        .unwrap();
        let down = Position {
            node: p1,
            offset: 1,
            affinity: Affinity::Downstream,
        }
        .resolve(&view)
        .unwrap();
        assert!(up < down);
        assert!(up != down);
    }

    fn para_doc(children: &[SeqItem]) -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for (i, c) in children.iter().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), c.clone()));
        }
        (project_document(&logs(&items)).unwrap(), para)
    }

    #[test]
    fn grapheme_skips_combining_mark() {
        use editor_model::AtomLeaf;
        use editor_resource::Resource;
        let r = Resource::new_test();
        let (pd, para) = para_doc(&[
            SeqItem::Char('a'),
            SeqItem::Char('e'),
            SeqItem::Char('\u{0301}'),
            SeqItem::Atom(AtomLeaf::HardBreak),
            SeqItem::Char('b'),
        ]);
        let view = DocView::new(&pd);
        let at = |off: usize| Position::new(para, off).resolve(&view).unwrap();
        assert_eq!(at(0).next_grapheme(&r).unwrap().offset(), 1);
        assert_eq!(at(1).next_grapheme(&r).unwrap().offset(), 3);
        assert_eq!(at(3).next_grapheme(&r).unwrap().offset(), 4);
        assert_eq!(at(5).prev_grapheme(&r).unwrap().offset(), 4);
        assert_eq!(at(3).prev_grapheme(&r).unwrap().offset(), 1);
    }

    #[test]
    fn next_grapheme_at_block_end_is_none() {
        use editor_resource::Resource;
        let r = Resource::new_test();
        let (pd, para) = para_doc(&[SeqItem::Char('a'), SeqItem::Char('b')]);
        let view = DocView::new(&pd);
        let at = |off: usize| Position::new(para, off).resolve(&view).unwrap();
        assert!(at(2).next_grapheme(&r).is_none());
    }

    #[test]
    fn prev_grapheme_at_offset_zero_is_none() {
        use editor_resource::Resource;
        let r = Resource::new_test();
        let (pd, para) = para_doc(&[SeqItem::Char('a'), SeqItem::Char('b')]);
        let view = DocView::new(&pd);
        let at = |off: usize| Position::new(para, off).resolve(&view).unwrap();
        assert!(at(0).prev_grapheme(&r).is_none());
    }

    #[test]
    fn empty_paragraph_has_no_adjacent_graphemes() {
        use editor_resource::Resource;
        let r = Resource::new_test();
        let (pd, para) = para_doc(&[]);
        let view = DocView::new(&pd);
        let at = |off: usize| Position::new(para, off).resolve(&view).unwrap();
        assert!(at(0).next_grapheme(&r).is_none());
        assert!(at(0).prev_grapheme(&r).is_none());
    }

    #[test]
    fn snap_to_grapheme_combining_mark() {
        use editor_resource::Resource;
        let r = Resource::new_test();
        let (pd, para) = para_doc(&[SeqItem::Char('e'), SeqItem::Char('\u{0301}')]);
        let view = DocView::new(&pd);
        let snap = |off: usize, aff: Affinity| {
            Position {
                node: para,
                offset: off,
                affinity: aff,
            }
            .resolve(&view)
            .unwrap()
            .snap_to_grapheme(&r)
            .offset()
        };
        assert_eq!(snap(1, Affinity::Upstream), 0);
        assert_eq!(snap(1, Affinity::Downstream), 2);
        assert_eq!(snap(0, Affinity::Downstream), 0);
        assert_eq!(snap(2, Affinity::Upstream), 2);
    }

    fn root_then_blockquote() -> ProjectedDoc {
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let para = Dot::new(1, 2);
        let items = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        project_document(&logs(&items)).unwrap()
    }

    #[test]
    fn resolve_derived_block() {
        let pd = root_then_blockquote();
        let view = DocView::new(&pd);
        let derived = view
            .root()
            .unwrap()
            .child_blocks()
            .find(|b| b.id().is_synthetic())
            .map(|b| b.id())
            .expect("normalize must synthesize a derived trailing paragraph");

        let r = Position::new(derived, 0)
            .resolve(&view)
            .expect("derived block resolves");
        assert_eq!(r.node(), derived);
        assert_eq!(r.path(), &[1, 0]);

        let bq = view
            .root()
            .unwrap()
            .child_blocks()
            .find(|b| b.node_type() == NodeType::Blockquote)
            .map(|b| b.id())
            .unwrap();
        let before = Position::new(bq, 0).resolve(&view).unwrap();
        assert!(before < r);
    }

    #[test]
    fn inline_leaf_dots_cover_last_leaf_with_upstream_block_end_head() {
        // Canonical forward selections normalize to an Upstream head; at a block
        // end that head stays Upstream. Leaf coverage is positional, so the final
        // leaf must still be included even though the head leans upstream.
        let (pd, para) = para_doc(&[
            SeqItem::Char('h'),
            SeqItem::Char('e'),
            SeqItem::Char('l'),
            SeqItem::Char('l'),
            SeqItem::Char('o'),
        ]);
        let view = DocView::new(&pd);
        let from = Position {
            node: para,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let to = Position {
            node: para,
            offset: 5,
            affinity: Affinity::Upstream,
        };
        let dots = inline_leaf_dots_in_range(&view, &from, &to);
        let text: String = dots
            .iter()
            .filter_map(|d| view.leaf(*d).and_then(|l| l.as_char()))
            .collect();
        assert_eq!(
            text, "hello",
            "Upstream block-end head must not drop the last leaf"
        );
    }

    #[test]
    fn resolve_rejects_unknown_and_out_of_range() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        assert!(Position::new(Dot::new(9, 9), 0).resolve(&view).is_none());
        assert!(Position::new(p1, 99).resolve(&view).is_none());
    }
}
