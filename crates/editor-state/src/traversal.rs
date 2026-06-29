use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeView};

use crate::{Position, selection::ResolvedSelection};

pub struct TextRun {
    pub host: Dot,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

fn node_path(node: &NodeView) -> Vec<usize> {
    let mut p: Vec<usize> = node.ancestors().filter_map(|n| n.index()).collect();
    p.reverse();
    p
}

fn is_prefix_of(prefix: &[usize], full: &[usize]) -> bool {
    full.starts_with(prefix)
}

fn position_before_or_at_node_start(
    pos_path: &[usize],
    pos_offset: usize,
    node_path: &[usize],
) -> bool {
    for (i, &node_idx) in node_path.iter().enumerate() {
        match pos_path.get(i).copied() {
            Some(p) if p < node_idx => return true,
            Some(p) if p > node_idx => return false,
            Some(_) => continue,
            None => return pos_offset <= node_idx,
        }
    }
    // pos_path matched all of node_path
    pos_path.len() == node_path.len() && pos_offset == 0
}

fn position_after_or_at_node_end(
    pos_path: &[usize],
    pos_offset: usize,
    node_path: &[usize],
    node_end_offset: usize,
) -> bool {
    for (i, &node_idx) in node_path.iter().enumerate() {
        match pos_path.get(i).copied() {
            Some(p) if p > node_idx => return true,
            Some(p) if p < node_idx => return false,
            Some(_) => continue,
            None => return pos_offset > node_idx,
        }
    }
    pos_path.len() == node_path.len() && pos_offset >= node_end_offset
}

fn path_intersects_range(node_path: &[usize], from_path: &[usize], to_path: &[usize]) -> bool {
    // Case 1: node is an ancestor of either endpoint (node_path is a prefix of from/to path)
    if is_prefix_of(node_path, from_path) || is_prefix_of(node_path, to_path) {
        return true;
    }

    // Case 2: node is a sibling between the endpoints under a shared parent
    if !node_path.is_empty() {
        let (&node_idx, node_parent) = node_path.split_last().unwrap();
        if is_prefix_of(node_parent, from_path) && is_prefix_of(node_parent, to_path) {
            let from_idx = from_path.get(node_parent.len()).copied().unwrap_or(0);
            let to_idx = to_path.get(node_parent.len()).copied().unwrap_or(0);
            let lo = from_idx.min(to_idx);
            let hi = from_idx.max(to_idx);
            if lo <= node_idx && node_idx <= hi {
                return true;
            }
        }
    }

    // Case 3: node lies strictly between endpoints lexicographically
    if node_path > from_path && node_path < to_path {
        return true;
    }

    false
}

pub fn intersects_subtree(rs: &ResolvedSelection, node: &NodeView) -> bool {
    let from_path = rs.from().path();
    let to_path = rs.to().path();
    let np = node_path(node);
    path_intersects_range(&np, from_path, to_path)
}

pub fn contains_subtree(rs: &ResolvedSelection, node: &NodeView) -> bool {
    let from = rs.from();
    let to = rs.to();

    let view = rs.view();
    let from_node_id = from.node();
    let to_node_id = to.node();

    let from_nv = view.node(from_node_id);
    let to_nv = view.node(to_node_id);

    let from_node_path: Vec<usize> = from_nv.as_ref().map(|n| node_path(n)).unwrap_or_default();
    let to_node_path: Vec<usize> = to_nv.as_ref().map(|n| node_path(n)).unwrap_or_default();

    let np = node_path(node);
    let node_end_offset = node.children().count();

    position_before_or_at_node_start(&from_node_path, from.offset(), &np)
        && position_after_or_at_node_end(&to_node_path, to.offset(), &np, node_end_offset)
}

pub fn blocks_in_range<'a>(rs: &ResolvedSelection<'a>) -> Vec<NodeView<'a>> {
    let mut out = Vec::new();
    if let Some(root) = rs.view().root() {
        walk_blocks(&root, rs, &mut out);
    }
    out
}

fn walk_blocks<'a>(node: &NodeView<'a>, rs: &ResolvedSelection<'a>, out: &mut Vec<NodeView<'a>>) {
    if intersects_subtree(rs, node) {
        out.push(*node);
        for child in node.child_blocks() {
            walk_blocks(&child, rs, out);
        }
    }
}

pub fn leaves_in_range<'a>(rs: &ResolvedSelection<'a>) -> Vec<editor_model::LeafView<'a>> {
    let from = rs.from().path();
    let to = rs.to().path();
    let mut out = Vec::new();
    for b in blocks_in_range(rs) {
        let base = node_path(&b);
        for (i, child) in b.children().enumerate() {
            if let ChildView::Leaf(l) = child {
                // A leaf fills the half-open content slot [i, i+1). It is inside the
                // selection iff its start path >= from and its end path <= to, so the
                // leaf at the exclusive `to` boundary is not over-collected.
                let mut start = base.clone();
                start.push(i);
                let mut end = base.clone();
                end.push(i + 1);
                if from <= start.as_slice() && end.as_slice() <= to {
                    out.push(l);
                }
            }
        }
    }
    out
}

pub fn text_run_around<'a>(pos: &Position, view: &'a DocView<'a>) -> Option<TextRun> {
    let host = view.node(pos.node)?;
    let children: Vec<_> = host.children().collect();
    let offset = pos.offset;

    // Extend left: walk back from offset-1, collecting chars
    let mut start = offset;
    while start > 0 {
        if let Some(ChildView::Leaf(l)) = children.get(start - 1)
            && l.as_char().is_some()
        {
            start -= 1;
            continue;
        }
        break;
    }

    // Extend right: walk forward from offset, collecting chars
    let mut end = offset;
    while end < children.len() {
        if let Some(ChildView::Leaf(l)) = children.get(end)
            && l.as_char().is_some()
        {
            end += 1;
            continue;
        }
        break;
    }

    let text: String = children[start..end]
        .iter()
        .filter_map(|c| match c {
            ChildView::Leaf(l) => l.as_char(),
            _ => None,
        })
        .collect();

    Some(TextRun {
        host: pos.node,
        start,
        end,
        text,
    })
}

pub fn first_cursor_position(node: &NodeView) -> Option<Position> {
    if node.spec().is_textblock() {
        return Some(Position::new(node.id(), 0));
    }
    match node.first_child() {
        Some(ChildView::Block(b)) => first_cursor_position(&b),
        Some(ChildView::Leaf(_)) => Some(Position::new(node.id(), 0)),
        None => Some(Position::new(node.id(), 0)),
    }
}

pub fn last_cursor_position(node: &NodeView) -> Option<Position> {
    if node.spec().is_textblock() {
        return Some(Position::new(node.id(), node.children().count()));
    }
    match node.last_child() {
        Some(ChildView::Block(b)) => last_cursor_position(&b),
        Some(ChildView::Leaf(_)) => Some(Position::new(node.id(), node.children().count())),
        None => Some(Position::new(node.id(), 0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AtomLeaf, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeStyleLog,
        NodeType, ProjectedDoc, SeqItem, SpanLog, StyleLog, project_document,
    };

    use crate::{Position, selection::Selection};

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
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
        }
    }

    fn two_paras() -> (ProjectedDoc, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 5);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('i')),
            (Dot::new(1, 4), SeqItem::Char('!')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 6), SeqItem::Char('y')),
        ];
        (project_document(&logs(&items)).unwrap(), root, p1, p2)
    }

    fn sel<'a>(
        view: &'a DocView<'a>,
        a: (Dot, usize),
        h: (Dot, usize),
    ) -> crate::selection::ResolvedSelection<'a> {
        let ap = Position::new(a.0, a.1);
        let hp = Position::new(h.0, h.1);
        Selection::new(ap, hp).resolve(view).unwrap()
    }

    // §4.2: intersects_subtree
    #[test]
    fn test_2_intersects_subtree() {
        let (pd, _root, p1, p2) = two_paras();
        let view = DocView::new(&pd);

        // selection from p1:0 to p2:1 — p1 and p2 are in range
        let rs = sel(&view, (p1, 0), (p2, 1));
        let root_nv = view.root().unwrap();
        assert!(
            intersects_subtree(&rs, &root_nv),
            "root intersects cross-para selection"
        );

        let p1_nv = view.node(p1).unwrap();
        assert!(intersects_subtree(&rs, &p1_nv), "p1 intersects");

        let p2_nv = view.node(p2).unwrap();
        assert!(intersects_subtree(&rs, &p2_nv), "p2 intersects");

        // selection entirely inside p1 — p2 should not intersect
        let rs_p1 = sel(&view, (p1, 0), (p1, 2));
        assert!(
            !intersects_subtree(&rs_p1, &p2_nv),
            "p2 does not intersect selection inside p1"
        );
        assert!(
            intersects_subtree(&rs_p1, &p1_nv),
            "p1 intersects selection inside it"
        );
        assert!(
            intersects_subtree(&rs_p1, &root_nv),
            "root (ancestor) intersects"
        );
    }

    // §4.3: contains_subtree
    #[test]
    fn test_3_contains_subtree() {
        let (pd, root, p1, p2) = two_paras();
        let view = DocView::new(&pd);

        // selection from start of p1 to end of p2 — both paragraphs are contained
        let root_nv = view.root().unwrap();
        let root_end = root_nv.children().count();

        let p1_nv = view.node(p1).unwrap();
        let p2_nv = view.node(p2).unwrap();

        // Whole-doc selection: p1 at 0 .. p2 at end
        let p2_end = p2_nv.children().count();
        let rs = sel(&view, (p1, 0), (p2, p2_end));
        assert!(
            contains_subtree(&rs, &p1_nv),
            "p1 contained in selection from p1:0 to p2:end"
        );
        assert!(contains_subtree(&rs, &p2_nv), "p2 contained");
        assert!(
            !contains_subtree(&rs, &root_nv),
            "root not contained (selection starts inside p1, not at root:0)"
        );

        // Entire doc selection at root level
        let rs_full = sel(&view, (root, 0), (root, root_end));
        assert!(
            contains_subtree(&rs_full, &root_nv),
            "root contained in full selection"
        );
        assert!(
            contains_subtree(&rs_full, &p1_nv),
            "p1 contained in full selection"
        );

        // Partial selection inside p1 — p1 not fully contained
        let p1_len = p1_nv.children().count();
        let rs_partial = sel(&view, (p1, 1), (p1, p1_len));
        assert!(
            !contains_subtree(&rs_partial, &p1_nv),
            "p1 not contained in partial selection"
        );
    }

    // §4.4: blocks_in_range
    #[test]
    fn test_4_blocks_in_range() {
        let (pd, _root, p1, p2) = two_paras();
        let view = DocView::new(&pd);

        // selection spanning p1 and p2
        let rs = sel(&view, (p1, 0), (p2, 1));
        let blocks = blocks_in_range(&rs);
        let types: Vec<NodeType> = blocks.iter().map(|b| b.node_type()).collect();
        assert!(types.contains(&NodeType::Root), "root in blocks");
        assert!(types.contains(&NodeType::Paragraph), "paragraph in blocks");
        // root comes first (pre-order DFS)
        assert_eq!(blocks[0].node_type(), NodeType::Root);

        // selection inside one paragraph: root is still included
        let rs_p1 = sel(&view, (p1, 0), (p1, 2));
        let blocks_p1 = blocks_in_range(&rs_p1);
        let types_p1: Vec<NodeType> = blocks_p1.iter().map(|b| b.node_type()).collect();
        assert!(
            types_p1.contains(&NodeType::Root),
            "root in blocks even for single-para selection"
        );
        assert!(types_p1.contains(&NodeType::Paragraph));
        let p2_included = blocks_p1.iter().any(|b| b.id() == p2);
        assert!(!p2_included, "p2 not included in single-para selection");
    }

    // §4.5: leaves_in_range — over-collection guard
    fn p1_image_p2_doc() -> (ProjectedDoc, Dot, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let image_dot = Dot::new(1, 10);
        let p2 = Dot::new(1, 11);
        let img_node = match editor_model::NodeType::Image.into_node() {
            editor_model::Node::Image(n) => n,
            _ => unreachable!(),
        };
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Char('c')),
            (
                image_dot,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node },
                    parents: vec![root],
                },
            ),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 12), SeqItem::Char('x')),
        ];
        (
            project_document(&logs(&items)).unwrap(),
            root,
            p1,
            image_dot,
            p2,
        )
    }

    #[test]
    fn test_5_leaves_in_range_basic() {
        let (pd, _root, p1, _image_dot, _p2) = p1_image_p2_doc();
        let view = DocView::new(&pd);

        // selection from char 1 to char 3 of p1 — covers 'b' and 'c' (offsets 1..3)
        let rs = sel(&view, (p1, 1), (p1, 3));
        let leaves = leaves_in_range(&rs);
        let chars: Vec<char> = leaves.iter().filter_map(|l| l.as_char()).collect();
        assert_eq!(chars.len(), 2, "should collect exactly 2 chars");
        assert!(chars.contains(&'b'), "b in range");
        assert!(chars.contains(&'c'), "c in range");
        assert!(!chars.contains(&'a'), "a not in range");
    }

    #[test]
    fn test_5_leaves_in_range_no_image_when_inside_p1() {
        let (pd, _root, p1, image_dot, _p2) = p1_image_p2_doc();
        let view = DocView::new(&pd);

        // selection entirely inside p1 — the image leaf (sibling of p1) must NOT be collected
        let rs = sel(&view, (p1, 0), (p1, 3));
        let leaves = leaves_in_range(&rs);
        let image_found = leaves.iter().any(|l| {
            l.as_atom()
                .is_some_and(|a| matches!(a, AtomLeaf::Image { .. }))
        });
        assert!(
            !image_found,
            "image leaf must NOT be collected when selection is inside p1"
        );
        let _ = image_dot;
    }

    #[test]
    fn test_5_leaves_in_range_image_when_spanning_p1_p2() {
        let (pd, root, p1, _image_dot, p2) = p1_image_p2_doc();
        let view = DocView::new(&pd);

        // selection spanning p1 to p2 (passing through image)
        let rs = sel(&view, (p1, 0), (p2, 1));
        let leaves = leaves_in_range(&rs);
        let image_found = leaves.iter().any(|l| {
            l.as_atom()
                .is_some_and(|a| matches!(a, AtomLeaf::Image { .. }))
        });
        assert!(
            image_found,
            "image leaf IS collected when selection spans p1 to p2"
        );
        let _ = root;
    }

    // §4.7: text_run_around
    fn hello_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('e')),
            (Dot::new(1, 4), SeqItem::Char('l')),
            (Dot::new(1, 5), SeqItem::Char('l')),
            (Dot::new(1, 6), SeqItem::Char('o')),
        ];
        (project_document(&logs(&items)).unwrap(), para)
    }

    fn split_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        // "ab" + HardBreak + "cd"
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Atom(AtomLeaf::HardBreak)),
            (Dot::new(1, 5), SeqItem::Char('c')),
            (Dot::new(1, 6), SeqItem::Char('d')),
        ];
        (project_document(&logs(&items)).unwrap(), para)
    }

    fn empty_para_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        )];
        (project_document(&logs(&items)).unwrap(), para)
    }

    #[test]
    fn test_7_text_run_caret_amid_hello() {
        let (pd, para) = hello_doc();
        let view = DocView::new(&pd);

        let pos = Position::new(para, 2); // between 'e' and 'l'
        let run = text_run_around(&pos, &view).unwrap();
        assert_eq!(run.start, 0);
        assert_eq!(run.end, 5);
        assert_eq!(run.text, "Hello");
        assert_eq!(run.host, para);
    }

    #[test]
    fn test_7_text_run_split_by_hardbreak() {
        let (pd, para) = split_doc();
        let view = DocView::new(&pd);

        // caret before 'c' (offset 3, after HardBreak at index 2)
        let pos_after = Position::new(para, 3);
        let run_after = text_run_around(&pos_after, &view).unwrap();
        assert_eq!(run_after.start, 3);
        assert_eq!(run_after.end, 5);
        assert_eq!(run_after.text, "cd");

        // caret before 'b' (offset 1, before HardBreak)
        let pos_before = Position::new(para, 1);
        let run_before = text_run_around(&pos_before, &view).unwrap();
        assert_eq!(run_before.start, 0);
        assert_eq!(run_before.end, 2);
        assert_eq!(run_before.text, "ab");
    }

    #[test]
    fn test_7_text_run_empty_para() {
        let (pd, para) = empty_para_doc();
        let view = DocView::new(&pd);

        let pos = Position::new(para, 0);
        let run = text_run_around(&pos, &view).unwrap();
        assert_eq!(run.start, 0);
        assert_eq!(run.end, 0);
        assert_eq!(run.text, "");
    }

    #[test]
    fn test_7_text_run_dead_host_is_none() {
        let (pd, _para) = empty_para_doc();
        let view = DocView::new(&pd);
        let dead = Position::new(Dot::new(9, 9), 0);
        assert!(text_run_around(&dead, &view).is_none());
    }

    // §4.8: cursor boundaries
    #[test]
    fn test_8_cursor_boundaries_paragraph() {
        let (pd, _root, p1, _img, _p2) = p1_image_p2_doc();
        let view = DocView::new(&pd);
        let p1_nv = view.node(p1).unwrap();
        let p1_len = p1_nv.children().count();

        let first = first_cursor_position(&p1_nv).unwrap();
        assert_eq!(first.offset, 0);
        assert_eq!(first.node, p1);

        let last = last_cursor_position(&p1_nv).unwrap();
        assert_eq!(last.offset, p1_len);
        assert_eq!(last.node, p1);
    }

    #[test]
    fn test_8_cursor_boundaries_empty_paragraph() {
        let (pd, para) = empty_para_doc();
        let view = DocView::new(&pd);
        let para_nv = view.node(para).unwrap();

        let first = first_cursor_position(&para_nv).unwrap();
        assert_eq!(first.offset, 0);

        let last = last_cursor_position(&para_nv).unwrap();
        assert_eq!(last.offset, 0);
    }

    // §4.9: proptest — invariants
    proptest::proptest! {
        #[test]
        fn test_9_proptest_intersect_contains_consistency(
            a_off in 0usize..=3,
            h_off in 0usize..=1,
        ) {
            let (pd, _root, p1, _img, p2) = p1_image_p2_doc();
            let view = DocView::new(&pd);

            let anchor = Position::new(p1, a_off.min(3));
            let head = Position::new(p2, h_off.min(1));
            let sel_opt = Selection::new(anchor, head).resolve(&view);
            if let Some(rs) = sel_opt {
                let blocks = blocks_in_range(&rs);
                // Every block in blocks_in_range should intersect the selection
                for b in &blocks {
                    proptest::prop_assert!(intersects_subtree(&rs, b), "every collected block intersects");
                }

                // leaves_in_range ⊆ blocks' leaf children
                let leaves = leaves_in_range(&rs);
                for lv in &leaves {
                    let parent = lv.parent();
                    proptest::prop_assert!(parent.is_some(), "leaf has parent");
                    if let Some(p) = parent {
                        let p_in_blocks = blocks.iter().any(|b| b.id() == p.id());
                        proptest::prop_assert!(p_in_blocks, "leaf's parent is in blocks_in_range");
                    }
                }

                // cursor bounds: first <= last
                let root_nv = view.root().unwrap();
                if let (Some(first), Some(last)) = (first_cursor_position(&root_nv), last_cursor_position(&root_nv)) {
                    let first_r = first.resolve(&view);
                    let last_r = last.resolve(&view);
                    if let (Some(fr), Some(lr)) = (first_r, last_r) {
                        proptest::prop_assert!(fr <= lr, "first cursor <= last cursor");
                    }
                }

                // text_run_around never panics and start <= end
                let pos = rs.from().position();
                if let Some(run) = text_run_around(&pos, &view) {
                    proptest::prop_assert!(run.start <= run.end, "run start <= end");
                    let host_nv = view.node(run.host);
                    if let Some(h) = host_nv {
                        proptest::prop_assert!(run.end <= h.children().count(), "run end <= host child count");
                    }
                }
            }
        }
    }
}
