use std::collections::{HashMap, VecDeque};

use editor_crdt::Dot;

use super::project::{RawChild, RawNode, RawTree, synthetic_id};
use crate::nodes::NodeType;
use crate::schema::{ContentExpr, ContextExpr};

fn fixed_slot_roles(content: &ContentExpr) -> Option<Vec<NodeType>> {
    match content {
        ContentExpr::Seq(es) if es.iter().all(|e| matches!(e, ContentExpr::Single(_))) => Some(
            es.iter()
                .map(|e| match e {
                    ContentExpr::Single(t) => *t,
                    _ => unreachable!(),
                })
                .collect(),
        ),
        _ => None,
    }
}

pub fn reshape_fixed_slots(node: &mut RawNode) {
    let roles = fixed_slot_roles(&node.node_type.spec().content).expect("fixed-slot only");
    let mut by_role: HashMap<NodeType, VecDeque<RawNode>> = HashMap::new();
    for c in std::mem::take(&mut node.children) {
        if let RawChild::Block(b) = c {
            by_role.entry(b.node_type).or_default().push_back(b);
        }
    }
    for role in roles {
        let slot = by_role
            .get_mut(&role)
            .and_then(|q| q.pop_front())
            .unwrap_or_else(|| scaffold_block(role, 0, node.id));
        node.children.push(RawChild::Block(slot));
    }
}

fn match_content(
    expr: &ContentExpr,
    kids: &mut VecDeque<RawChild>,
    parent: Dot,
    out: &mut Vec<RawChild>,
) {
    let front = |k: &VecDeque<RawChild>, e: &ContentExpr| {
        k.front()
            .is_some_and(|c| c.as_child_type().is_some_and(|t| e.matches(t)))
    };
    match expr {
        ContentExpr::Empty | ContentExpr::Any => {}
        ContentExpr::Single(t) => {
            if kids.front().is_some_and(|c| c.as_child_type() == Some(*t)) {
                out.push(kids.pop_front().unwrap());
            } else {
                out.push(RawChild::Block(scaffold_block(*t, 0, parent)));
            }
        }
        ContentExpr::Optional(inner) => {
            if front(kids, inner) {
                out.push(kids.pop_front().unwrap());
            }
        }
        ContentExpr::ZeroOrMore(inner) => {
            while front(kids, inner) {
                out.push(kids.pop_front().unwrap());
            }
        }
        ContentExpr::OneOrMore(inner) => {
            let n = out.len();
            while front(kids, inner) {
                out.push(kids.pop_front().unwrap());
            }
            if out.len() == n {
                out.push(RawChild::Block(scaffold_block(
                    first_type(inner),
                    0,
                    parent,
                )));
            }
        }
        ContentExpr::Choice(cs) => match cs.iter().find(|c| front(kids, c)) {
            Some(c) => match_content(c, kids, parent, out),
            None => out.push(RawChild::Block(scaffold_block(
                first_type(&cs[0]),
                0,
                parent,
            ))),
        },
        ContentExpr::Seq(exprs) => {
            for e in exprs {
                match_content(e, kids, parent, out);
            }
        }
    }
}

pub fn repair_general(node: &mut RawNode) {
    let content = node.node_type.spec().content.clone();
    loop {
        let mut kids: VecDeque<RawChild> = std::mem::take(&mut node.children).into_iter().collect();
        let mut out = Vec::new();
        match_content(&content, &mut kids, node.id, &mut out);
        let promoted: Vec<RawChild> = Vec::from(kids)
            .into_iter()
            .filter_map(|c| match c {
                RawChild::Block(b) => Some(b.children),
                _ => None,
            })
            .flatten()
            .collect();
        node.children = out;
        if promoted.is_empty() {
            return;
        }
        node.children.extend(promoted);
    }
}

fn repair_preserving_unknown(node: &mut RawNode, repair: impl FnOnce(&mut RawNode)) {
    let taken = std::mem::take(&mut node.children);
    let mut known = Vec::with_capacity(taken.len());
    let mut unknowns: Vec<(usize, RawChild)> = Vec::new();
    for c in taken {
        match c.as_child_type() {
            Some(t) if t != NodeType::Unknown => known.push(c),
            _ => unknowns.push((known.len(), c)),
        }
    }
    node.children = known;
    repair(node);
    if !unknowns.is_empty() {
        let mut out = std::mem::take(&mut node.children);
        for (i, (idx, c)) in unknowns.into_iter().enumerate() {
            let at = (idx + i).min(out.len());
            out.insert(at, c);
        }
        node.children = out;
    }
}

fn normalize_grid(table: &mut RawNode) {
    let width = table
        .children
        .iter()
        .filter_map(|c| match c {
            RawChild::Block(r) if r.node_type == NodeType::TableRow => Some(
                r.children
                    .iter()
                    .filter(|cc| cc.as_child_type() == Some(NodeType::TableCell))
                    .count(),
            ),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    for slot in 0..table.children.len() {
        let Some(RawChild::Block(mut row)) = table.children.get(slot).cloned() else {
            continue;
        };
        if row.node_type != NodeType::TableRow {
            continue;
        }
        let mut count = row
            .children
            .iter()
            .filter(|cc| cc.as_child_type() == Some(NodeType::TableCell))
            .count();
        if count >= width {
            continue;
        }
        while count < width {
            let cell = scaffold_block(NodeType::TableCell, count, row.id);
            row.children.push(RawChild::Block(cell));
            count += 1;
        }
        table.children[slot] = RawChild::Block(row);
    }
}

fn scaffold_block(role: NodeType, slot: usize, parent: Dot) -> RawNode {
    let id = synthetic_id(parent, slot, role);
    let mut out = Vec::new();
    match_content(&role.spec().content, &mut VecDeque::new(), id, &mut out);
    RawNode {
        id,
        node_type: role,
        attrs: vec![],
        children: out,
    }
}

fn first_type(e: &ContentExpr) -> NodeType {
    match e {
        ContentExpr::Single(t) => *t,
        ContentExpr::Choice(cs) => first_type(&cs[0]),
        ContentExpr::OneOrMore(i) | ContentExpr::ZeroOrMore(i) | ContentExpr::Optional(i) => {
            first_type(i)
        }
        ContentExpr::Seq(es) => first_type(&es[0]),
        ContentExpr::Empty | ContentExpr::Any => unreachable!(),
    }
}

fn last_known_child(children: &[RawChild]) -> Option<&RawChild> {
    children.iter().rev().find(|child| {
        child
            .as_child_type()
            .is_some_and(|node_type| node_type != NodeType::Unknown)
    })
}

fn complete_root_trailing_editable_paragraph(node: &mut RawNode) {
    if node.node_type != NodeType::Root {
        return;
    }
    let Some(RawChild::Block(paragraph)) = last_known_child(&node.children) else {
        return;
    };
    if paragraph.node_type != NodeType::Paragraph
        || last_known_child(&paragraph.children).and_then(RawChild::as_child_type)
            != Some(NodeType::PageBreak)
    {
        return;
    }
    node.children.push(RawChild::Block(scaffold_block(
        NodeType::Paragraph,
        0,
        node.id,
    )));
}

fn drop_context_invalid_here(node: &mut RawNode, path: &[NodeType]) {
    node.children.retain(|c| {
        let Some(t) = c.as_child_type() else {
            return true;
        };
        let full: Vec<NodeType> = path.iter().copied().chain(std::iter::once(t)).collect();
        let ctx = &t.spec().context;
        *ctx == ContextExpr::Any || ctx.matches(&full)
    });
}

fn fix_roots(tree: &mut RawTree) {
    let mut tops = std::mem::take(&mut tree.roots);
    match tops.iter().position(|r| r.node_type == NodeType::Root) {
        Some(i) => {
            let mut root = tops.remove(i);
            root.children.extend(tops.into_iter().map(RawChild::Block));
            tree.roots = vec![root];
        }
        None => {
            tree.roots = vec![RawNode {
                id: Dot::ROOT,
                node_type: NodeType::Root,
                attrs: vec![],
                children: tops.into_iter().map(RawChild::Block).collect(),
            }];
        }
    }
}

pub fn normalize(mut tree: RawTree) -> RawTree {
    fix_roots(&mut tree);
    for r in &mut tree.roots {
        normalize_node(r, &mut Vec::new());
    }
    tree
}

/// Normalize a single block's subtree in place under the given ancestor types,
/// applying only that block's (and descendants') content rules — NOT the
/// document Root rules. For localized re-projection of one top-level block.
pub fn normalize_subtree(node: &mut RawNode, ancestors: &[NodeType]) {
    let mut path = ancestors.to_vec();
    normalize_node(node, &mut path);
}

/// Apply only `node`'s own schema content rule (repairing its direct children),
/// assuming its children are already individually normalized — no deep
/// recursion. Lets a caller re-establish a container's content invariant (e.g.
/// the Root's required trailing paragraph) by deferring to the schema rather
/// than hardcoding it. Newly scaffolded children are themselves shaped by
/// `match_content`, so they need no further normalization here.
pub fn normalize_content_shallow(node: &mut RawNode, ancestors: &[NodeType]) {
    let mut path = ancestors.to_vec();
    path.push(node.node_type);
    let mut passes = 0;
    loop {
        passes += 1;
        debug_assert!(passes <= 100, "normalize_content_shallow did not converge");
        if node.node_type != NodeType::Unknown {
            drop_context_invalid_here(node, &path);
        }
        let types: Vec<NodeType> = node
            .children
            .iter()
            .filter_map(|c| c.as_child_type())
            .filter(|t| *t != NodeType::Unknown)
            .collect();
        if node.node_type.spec().content.matches_sequence(&types) {
            break;
        } else if fixed_slot_roles(&node.node_type.spec().content).is_some() {
            repair_preserving_unknown(node, reshape_fixed_slots);
        } else {
            repair_preserving_unknown(node, repair_general);
        }
    }
    complete_root_trailing_editable_paragraph(node);
    path.pop();
}

/// Recurse `normalize_node` into each direct block child, writing the normalized
/// child back.
fn recurse_block_children(children: &mut [RawChild], path: &mut Vec<NodeType>) {
    for c in children.iter_mut() {
        if let RawChild::Block(b) = c {
            normalize_node(b, path);
        }
    }
}

fn normalize_node(node: &mut RawNode, path: &mut Vec<NodeType>) {
    path.push(node.node_type);
    recurse_block_children(&mut node.children, path);
    let mut passes = 0;
    loop {
        passes += 1;
        debug_assert!(passes <= 100, "normalize_node did not converge");
        if node.node_type != NodeType::Unknown {
            drop_context_invalid_here(node, path);
        }
        let types: Vec<NodeType> = node
            .children
            .iter()
            .filter_map(|c| c.as_child_type())
            .filter(|t| *t != NodeType::Unknown)
            .collect();
        if node.node_type.spec().content.matches_sequence(&types) {
            break;
        } else if fixed_slot_roles(&node.node_type.spec().content).is_some() {
            repair_preserving_unknown(node, reshape_fixed_slots);
        } else {
            repair_preserving_unknown(node, repair_general);
        }
        recurse_block_children(&mut node.children, path);
    }
    if node.node_type == NodeType::Table {
        normalize_grid(node);
    }
    complete_root_trailing_editable_paragraph(node);
    path.pop();
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;

    use super::*;
    use crate::seq::{AtomLeaf, BlockTree, validate_block_tree as validate_flat};

    fn valid(t: &RawTree) -> Result<(), crate::SchemaError> {
        validate_flat(&BlockTree::from_raw(t))
    }

    fn find_leaf(tree: &RawTree, id: Dot) -> Option<&super::super::SeqItem> {
        fn in_node(node: &RawNode, id: Dot) -> Option<&super::super::SeqItem> {
            node.children.iter().find_map(|child| match child {
                RawChild::Leaf { id: leaf_id, item } if *leaf_id == id => Some(item),
                RawChild::Block(block) => in_node(block, id),
                RawChild::Leaf { .. } => None,
            })
        }

        tree.roots.iter().find_map(|root| in_node(root, id))
    }

    fn fold(kids: Vec<(u64, NodeType)>) -> RawNode {
        RawNode {
            attrs: vec![],
            id: Dot::new(1, 0),
            node_type: NodeType::Fold,
            children: kids
                .into_iter()
                .map(|(i, t)| {
                    RawChild::Block(RawNode {
                        attrs: vec![],
                        id: Dot::new(1, i),
                        node_type: t,
                        children: vec![],
                    })
                })
                .collect(),
        }
    }

    #[test]
    fn reshape_fold_reorders_dedupes_fills() {
        let mut node = fold(vec![
            (1, NodeType::FoldContent),
            (2, NodeType::FoldTitle),
            (3, NodeType::FoldTitle),
        ]);
        reshape_fixed_slots(&mut node);

        assert_eq!(node.children.len(), 2);
        assert!(matches!(
            &node.children[0],
            RawChild::Block(b) if b.node_type == NodeType::FoldTitle && b.id == Dot::new(1, 2)
        ));
        assert!(matches!(
            &node.children[1],
            RawChild::Block(b) if b.node_type == NodeType::FoldContent && b.id == Dot::new(1, 1)
        ));
    }

    #[test]
    fn reshape_missing_slot_is_filled() {
        let mut node = fold(vec![]);
        reshape_fixed_slots(&mut node);

        assert_eq!(node.children.len(), 2);
        assert!(matches!(
            &node.children[0],
            RawChild::Block(b) if b.node_type == NodeType::FoldTitle && b.id.is_synthetic()
        ));
        assert!(matches!(
            &node.children[1],
            RawChild::Block(b) if b.node_type == NodeType::FoldContent
        ));
    }

    #[test]
    fn repair_root_fills_trailing_paragraph() {
        let mut node = RawNode {
            attrs: vec![],
            id: Dot::ROOT,
            node_type: NodeType::Root,
            children: vec![RawChild::Block(RawNode {
                attrs: vec![],
                id: Dot::new(1, 1),
                node_type: NodeType::Blockquote,
                children: vec![],
            })],
        };
        repair_general(&mut node);

        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[0].as_child_type(), Some(NodeType::Blockquote));
        assert!(matches!(
            &node.children[1],
            RawChild::Block(b) if b.node_type == NodeType::Paragraph && b.id.is_synthetic()
        ));
    }

    #[test]
    fn normalize_root_adds_trailing_paragraph_after_page_break() {
        let paragraph_id = Dot::new(1, 1);
        let tree = RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::ROOT,
                node_type: NodeType::Root,
                children: vec![RawChild::Block(RawNode {
                    attrs: vec![],
                    id: paragraph_id,
                    node_type: NodeType::Paragraph,
                    children: vec![RawChild::Leaf {
                        id: Dot::new(1, 2),
                        item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                    }],
                })],
            }],
        };

        let normalized = normalize(tree);
        let root = &normalized.roots[0];

        assert_eq!(root.children.len(), 2);
        assert!(matches!(
            &root.children[0],
            RawChild::Block(paragraph) if paragraph.id == paragraph_id
        ));
        assert!(matches!(
            &root.children[1],
            RawChild::Block(paragraph)
                if paragraph.node_type == NodeType::Paragraph && paragraph.id.is_synthetic()
        ));
    }

    #[test]
    fn normalize_content_shallow_adds_trailing_paragraph_after_page_break() {
        let mut root = RawNode {
            attrs: vec![],
            id: Dot::ROOT,
            node_type: NodeType::Root,
            children: vec![RawChild::Block(RawNode {
                attrs: vec![],
                id: Dot::new(1, 1),
                node_type: NodeType::Paragraph,
                children: vec![RawChild::Leaf {
                    id: Dot::new(1, 2),
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                }],
            })],
        };

        normalize_content_shallow(&mut root, &[]);

        assert!(matches!(
            &root.children[..],
            [RawChild::Block(_), RawChild::Block(paragraph)]
                if paragraph.node_type == NodeType::Paragraph && paragraph.id.is_synthetic()
        ));
    }

    #[test]
    fn normalize_root_ignores_unknown_after_terminal_page_break() {
        let tree = RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::ROOT,
                node_type: NodeType::Root,
                children: vec![RawChild::Block(RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 1),
                    node_type: NodeType::Paragraph,
                    children: vec![
                        RawChild::Leaf {
                            id: Dot::new(1, 2),
                            item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                        },
                        RawChild::Leaf {
                            id: Dot::new(1, 3),
                            item: super::super::SeqItem::Unknown {
                                tag: 999,
                                bytes: vec![],
                            },
                        },
                    ],
                })],
            }],
        };

        let normalized = normalize(tree);

        assert!(matches!(
            &normalized.roots[0].children[..],
            [RawChild::Block(_), RawChild::Block(paragraph)]
                if paragraph.node_type == NodeType::Paragraph && paragraph.id.is_synthetic()
        ));
    }

    #[test]
    fn repair_drops_extra_leaf() {
        let mut node = RawNode {
            attrs: vec![],
            id: Dot::new(1, 0),
            node_type: NodeType::Paragraph,
            children: vec![
                RawChild::Leaf {
                    id: Dot::new(1, 1),
                    item: super::super::SeqItem::Char('x'),
                },
                RawChild::Leaf {
                    id: Dot::new(1, 2),
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
                RawChild::Leaf {
                    id: Dot::new(1, 3),
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
            ],
        };
        repair_general(&mut node);

        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[0].as_child_type(), Some(NodeType::Text));
        assert_eq!(node.children[1].as_child_type(), Some(NodeType::PageBreak));
        assert!(matches!(
            &node.children[1],
            RawChild::Leaf { id, .. } if *id == Dot::new(1, 2)
        ));
    }

    #[test]
    fn repair_promotes_surplus_block_children() {
        let mut node = RawNode {
            attrs: vec![],
            id: Dot::ROOT,
            node_type: NodeType::Root,
            children: vec![RawChild::Block(RawNode {
                attrs: vec![],
                id: Dot::new(1, 1),
                node_type: NodeType::ListItem,
                children: vec![RawChild::Block(RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 2),
                    node_type: NodeType::Paragraph,
                    children: vec![],
                })],
            })],
        };
        repair_general(&mut node);

        assert!(
            node.children
                .iter()
                .any(|c| matches!(c, RawChild::Block(b) if b.id == Dot::new(1, 2))),
            "promoted Paragraph(1,2) must survive"
        );
        assert!(
            node.children
                .iter()
                .all(|c| c.as_child_type() == Some(NodeType::Paragraph)),
            "all Root children must be Paragraph"
        );
        assert!(matches!(
            node.children.last(),
            Some(RawChild::Block(b)) if b.id.is_synthetic() && b.node_type == NodeType::Paragraph
        ));
    }

    #[test]
    fn drop_context_invalid_drops_pagebreak() {
        let tree = RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::new(1, 0),
                node_type: NodeType::Blockquote,
                children: vec![RawChild::Block(RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 1),
                    node_type: NodeType::Paragraph,
                    children: vec![RawChild::Leaf {
                        id: Dot::new(1, 2),
                        item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                    }],
                })],
            }],
        };
        let out = normalize(tree);
        let has_pagebreak = |n: &RawNode| {
            fn walk(n: &RawNode) -> bool {
                n.children.iter().any(|c| match c {
                    RawChild::Leaf { item, .. } => {
                        item.as_child_type() == Some(NodeType::PageBreak)
                    }
                    RawChild::Block(b) => walk(b),
                })
            }
            walk(n)
        };
        assert!(out.roots.iter().all(|r| !has_pagebreak(r)));
    }

    #[test]
    fn fix_roots_wraps_non_root_top() {
        let tree = RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::new(1, 0),
                node_type: NodeType::Paragraph,
                children: vec![],
            }],
        };
        let out = normalize(tree);
        assert_eq!(out.roots.len(), 1);
        assert_eq!(out.roots[0].node_type, NodeType::Root);
        assert!(
            out.roots[0]
                .children
                .iter()
                .any(|c| c.as_child_type() == Some(NodeType::Paragraph))
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_messy_sample_is_valid_and_idempotent() {
        let leaf = |i: u64, item: super::super::SeqItem| RawChild::Leaf {
            id: Dot::new(1, i),
            item,
        };
        let blk = |i: u64, t: NodeType, children: Vec<RawChild>| {
            RawChild::Block(RawNode {
                attrs: vec![],
                id: Dot::new(1, i),
                node_type: t,
                children,
            })
        };
        let tree = RawTree {
            roots: vec![
                RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 1),
                    node_type: NodeType::Paragraph,
                    children: vec![leaf(2, super::super::SeqItem::Char('a'))],
                },
                RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 3),
                    node_type: NodeType::Fold,
                    children: vec![],
                },
                RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 4),
                    node_type: NodeType::Fold,
                    children: vec![
                        blk(5, NodeType::FoldContent, vec![]),
                        blk(6, NodeType::FoldTitle, vec![]),
                    ],
                },
                RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 7),
                    node_type: NodeType::Blockquote,
                    children: vec![blk(
                        8,
                        NodeType::Paragraph,
                        vec![leaf(9, super::super::SeqItem::Atom(AtomLeaf::PageBreak))],
                    )],
                },
                RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 10),
                    node_type: NodeType::BulletList,
                    children: vec![leaf(11, super::super::SeqItem::Char('z'))],
                },
            ],
        };
        assert!(valid(&normalize(tree.clone())).is_ok());
        assert_eq!(normalize(normalize(tree.clone())), normalize(tree));
    }

    fn tcell(i: u64) -> RawChild {
        RawChild::Block(RawNode {
            attrs: vec![],
            id: Dot::new(2, i),
            node_type: NodeType::TableCell,
            children: vec![],
        })
    }

    fn trow(id: u64, cells: Vec<RawChild>) -> RawChild {
        RawChild::Block(RawNode {
            attrs: vec![],
            id: Dot::new(2, id),
            node_type: NodeType::TableRow,
            children: cells,
        })
    }

    fn ttable(rows: Vec<RawChild>) -> RawNode {
        RawNode {
            attrs: vec![],
            id: Dot::new(2, 0),
            node_type: NodeType::Table,
            children: rows,
        }
    }

    fn cell_count(row: &RawChild) -> usize {
        match row {
            RawChild::Block(b) => b
                .children
                .iter()
                .filter(|c| c.as_child_type() == Some(NodeType::TableCell))
                .count(),
            _ => 0,
        }
    }

    #[test]
    fn normalize_grid_pads_short_rows_to_max() {
        let mut t = ttable(vec![
            trow(10, vec![tcell(11), tcell(12)]),
            trow(20, vec![tcell(21), tcell(22), tcell(23)]),
        ]);
        normalize_grid(&mut t);
        assert_eq!(cell_count(&t.children[0]), 3);
        assert_eq!(cell_count(&t.children[1]), 3);
    }

    #[test]
    fn normalize_grid_rectangular_is_noop() {
        let mut t = ttable(vec![
            trow(10, vec![tcell(11), tcell(12)]),
            trow(20, vec![tcell(21), tcell(22)]),
        ]);
        let before = t.clone();
        normalize_grid(&mut t);
        assert_eq!(t, before);
    }

    #[test]
    fn normalize_grid_pad_cells_have_distinct_slots() {
        let mut t = ttable(vec![
            trow(10, vec![tcell(11)]),
            trow(20, vec![tcell(21), tcell(22), tcell(23)]),
        ]);
        normalize_grid(&mut t);
        let r0 = match &t.children[0] {
            RawChild::Block(b) => b,
            _ => panic!("row0 not block"),
        };
        assert_eq!(r0.children.len(), 3);
        let id1 = match &r0.children[1] {
            RawChild::Block(b) => b.id,
            _ => panic!(),
        };
        let id2 = match &r0.children[2] {
            RawChild::Block(b) => b.id,
            _ => panic!(),
        };
        assert_eq!(id1, synthetic_id(Dot::new(2, 10), 1, NodeType::TableCell));
        assert_eq!(id2, synthetic_id(Dot::new(2, 10), 2, NodeType::TableCell));
        assert!(id1.is_synthetic());
        assert!(id2.is_synthetic());
        assert_ne!(id1, id2);
    }

    #[test]
    fn normalize_grid_empty_table_noop() {
        let mut t = ttable(vec![]);
        normalize_grid(&mut t);
        assert!(t.children.is_empty());
    }

    fn root_with_table(rows: Vec<RawChild>) -> RawTree {
        RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::new(2, 100),
                node_type: NodeType::Root,
                children: vec![RawChild::Block(RawNode {
                    attrs: vec![],
                    id: Dot::new(2, 0),
                    node_type: NodeType::Table,
                    children: rows,
                })],
            }],
        }
    }

    fn table_widths(tree: &RawTree) -> Vec<usize> {
        fn find_table(n: &RawNode) -> Option<&RawNode> {
            if n.node_type == NodeType::Table {
                return Some(n);
            }
            n.child_blocks().into_iter().find_map(find_table)
        }
        let table = tree.roots.iter().find_map(find_table).expect("table");
        table
            .children
            .iter()
            .filter_map(|c| match c {
                RawChild::Block(r) if r.node_type == NodeType::TableRow => Some(
                    r.children
                        .iter()
                        .filter(|cc| cc.as_child_type() == Some(NodeType::TableCell))
                        .count(),
                ),
                _ => None,
            })
            .collect()
    }

    fn grid_cell_ids(tree: &RawTree) -> Vec<Vec<Dot>> {
        fn find_table(n: &RawNode) -> Option<&RawNode> {
            if n.node_type == NodeType::Table {
                return Some(n);
            }
            n.child_blocks().into_iter().find_map(find_table)
        }
        let table = tree.roots.iter().find_map(find_table).expect("table");
        table
            .children
            .iter()
            .filter_map(|c| match c {
                RawChild::Block(r) if r.node_type == NodeType::TableRow => Some(
                    r.children
                        .iter()
                        .filter_map(|cc| match cc {
                            RawChild::Block(b) if b.node_type == NodeType::TableCell => Some(b.id),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn normalize_c2_column_plus_row_pads() {
        let t = root_with_table(vec![
            trow(10, vec![tcell(11), tcell(12), tcell(13)]),
            trow(20, vec![tcell(21), tcell(22), tcell(23)]),
            trow(30, vec![tcell(31), tcell(32)]),
        ]);
        let out = normalize(t);
        assert_eq!(table_widths(&out), vec![3, 3, 3]);
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_c4_irregular_paste_rectangular() {
        let t = root_with_table(vec![
            trow(10, vec![tcell(11)]),
            trow(20, vec![tcell(21), tcell(22), tcell(23), tcell(24)]),
            trow(30, vec![tcell(31), tcell(32)]),
        ]);
        let out = normalize(t);
        assert_eq!(table_widths(&out), vec![4, 4, 4]);
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_c6_empty_row_repaired_then_padded() {
        let t = root_with_table(vec![trow(10, vec![tcell(11), tcell(12)]), trow(20, vec![])]);
        let out = normalize(t);
        assert_eq!(table_widths(&out), vec![2, 2]);
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_c9_row_reorder_stays_rectangular() {
        let t = root_with_table(vec![
            trow(30, vec![tcell(31), tcell(32)]),
            trow(10, vec![tcell(11), tcell(12)]),
        ]);
        let out = normalize(t);
        assert_eq!(table_widths(&out), vec![2, 2]);
        assert_eq!(
            grid_cell_ids(&out),
            vec![
                vec![Dot::new(2, 31), Dot::new(2, 32)],
                vec![Dot::new(2, 11), Dot::new(2, 12)],
            ]
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_c7_empty_table_repaired() {
        let t = root_with_table(vec![]);
        let out = normalize(t);
        let widths = table_widths(&out);
        assert!(!widths.is_empty(), "repair 후 ≥1 행이어야");
        assert!(widths.iter().all(|&w| w >= 1), "각 행 ≥1 셀");
        let first = widths[0];
        assert!(widths.iter().all(|&w| w == first), "직사각형");
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_c3_misalign_limitation_grid_noop() {
        let t = root_with_table(vec![
            trow(10, vec![tcell(11), tcell(12), tcell(13), tcell(14)]),
            trow(20, vec![tcell(21), tcell(22), tcell(23), tcell(24)]),
        ]);
        let before = grid_cell_ids(&t);
        let out = normalize(t);
        assert_eq!(table_widths(&out), vec![4, 4]);
        assert_eq!(grid_cell_ids(&out), before);
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_leaf_in_paragraph() {
        let para = Dot::new(1, 1);
        let unknown = Dot::new(1, 2);
        let pb1 = Dot::new(1, 3);
        let pb2 = Dot::new(1, 4);
        let node = RawNode {
            attrs: vec![],
            id: para,
            node_type: NodeType::Paragraph,
            children: vec![
                RawChild::Leaf {
                    id: unknown,
                    item: super::super::SeqItem::Unknown {
                        tag: 999,
                        bytes: vec![0xAA],
                    },
                },
                RawChild::Leaf {
                    id: pb1,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
                RawChild::Leaf {
                    id: pb2,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
            ],
        };
        let out = normalize(RawTree { roots: vec![node] });
        assert!(
            matches!(
                find_leaf(&out, unknown),
                Some(super::super::SeqItem::Unknown { tag: 999, .. })
            ),
            "normalize가 unknown 리프를 드롭/변형하면 안 된다"
        );
        assert!(
            find_leaf(&out, pb1).is_some(),
            "매치되는 첫 PageBreak는 유지"
        );
        assert!(
            find_leaf(&out, pb2).is_none(),
            "unmatched-drop 경로가 실제로 두번째 PageBreak를 드롭해야 이 케이스가 유의미하다"
        );
    }

    #[test]
    fn normalize_preserves_unknown_leaf_in_table_row() {
        let unknown = Dot::new(2, 22);
        let t = root_with_table(vec![
            trow(10, vec![tcell(11), tcell(12), tcell(13)]),
            trow(
                20,
                vec![
                    tcell(21),
                    RawChild::Leaf {
                        id: unknown,
                        item: super::super::SeqItem::Unknown {
                            tag: 999,
                            bytes: vec![0xAA],
                        },
                    },
                    tcell(23),
                ],
            ),
        ]);
        let out = normalize(t);
        assert_eq!(
            table_widths(&out),
            vec![3, 3],
            "unknown은 셀로 집계되지 않는다"
        );
        fn find_table(n: &RawNode) -> Option<&RawNode> {
            if n.node_type == NodeType::Table {
                return Some(n);
            }
            n.child_blocks().into_iter().find_map(find_table)
        }
        let table = out.roots.iter().find_map(find_table).expect("table");
        let row20 = table
            .children
            .iter()
            .find_map(|c| match c {
                RawChild::Block(r) if r.id == Dot::new(2, 20) => Some(r),
                _ => None,
            })
            .expect("row 20");
        assert!(
            row20.children.iter().any(|c| matches!(
                c,
                RawChild::Leaf { id, item: super::super::SeqItem::Unknown { tag: 999, .. } } if *id == unknown
            )),
            "normalize_grid의 padding 경로가 unknown 리프를 건드리면 안 된다"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_leaf_through_promote() {
        let promoted_para = Dot::new(1, 2);
        let unknown = Dot::new(1, 9);
        let node = RawNode {
            attrs: vec![],
            id: Dot::ROOT,
            node_type: NodeType::Root,
            children: vec![
                RawChild::Block(RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 1),
                    node_type: NodeType::ListItem,
                    children: vec![RawChild::Block(RawNode {
                        attrs: vec![],
                        id: promoted_para,
                        node_type: NodeType::Paragraph,
                        children: vec![],
                    })],
                }),
                RawChild::Leaf {
                    id: unknown,
                    item: super::super::SeqItem::Unknown {
                        tag: 777,
                        bytes: vec![0xCC],
                    },
                },
            ],
        };
        let out = normalize(RawTree { roots: vec![node] });
        assert!(
            out.roots[0]
                .children
                .iter()
                .any(|c| matches!(c, RawChild::Block(b) if b.id == promoted_para)),
            "promoted Paragraph must survive"
        );
        assert!(
            out.roots[0].children.iter().any(|c| matches!(
                c,
                RawChild::Leaf { id, item: super::super::SeqItem::Unknown { tag: 777, .. } } if *id == unknown
            )),
            "promote 경로가 unknown 리프를 드롭/변형하면 안 된다"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_block_in_paragraph_unmatched_drop() {
        let para = Dot::new(1, 1);
        let unknown_block = Dot::new(1, 2);
        let unknown_child = Dot::new(1, 3);
        let pb1 = Dot::new(1, 4);
        let pb2 = Dot::new(1, 5);
        let node = RawNode {
            attrs: vec![],
            id: para,
            node_type: NodeType::Paragraph,
            children: vec![
                RawChild::Block(RawNode {
                    attrs: vec![],
                    id: unknown_block,
                    node_type: NodeType::Unknown,
                    children: vec![RawChild::Leaf {
                        id: unknown_child,
                        item: super::super::SeqItem::Char('x'),
                    }],
                }),
                RawChild::Leaf {
                    id: pb1,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
                RawChild::Leaf {
                    id: pb2,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
            ],
        };
        let out = normalize(RawTree { roots: vec![node] });
        // `fix_roots` wraps the bare Paragraph root under a synthesized Root.
        let para_out = out.roots[0]
            .children
            .iter()
            .find_map(|c| match c {
                RawChild::Block(b) if b.id == para => Some(b),
                _ => None,
            })
            .expect("paragraph must survive under the synthesized root");
        let unknown = para_out
            .children
            .iter()
            .find_map(|c| match c {
                RawChild::Block(b) if b.id == unknown_block => Some(b),
                _ => None,
            })
            .expect("unknown block must survive the unmatched-drop repair pass");
        assert_eq!(unknown.node_type, NodeType::Unknown);
        assert!(
            matches!(
                &unknown.children[0],
                RawChild::Leaf { id, item: super::super::SeqItem::Char('x') } if *id == unknown_child
            ),
            "unknown block's own child must attach normally, untouched"
        );
        assert!(
            para_out
                .children
                .iter()
                .any(|c| matches!(c, RawChild::Leaf { id, .. } if *id == pb1)),
            "matching first PageBreak kept"
        );
        assert!(
            !para_out
                .children
                .iter()
                .any(|c| matches!(c, RawChild::Leaf { id, .. } if *id == pb2)),
            "unmatched-drop must still drop the extra PageBreak (proves the case is meaningful)"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_block_through_context_filter() {
        let unknown_block = Dot::new(1, 10);
        let unknown_child = Dot::new(1, 11);
        let pagebreak = Dot::new(1, 2);
        let tree = RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::new(1, 0),
                node_type: NodeType::Blockquote,
                children: vec![
                    RawChild::Block(RawNode {
                        attrs: vec![],
                        id: Dot::new(1, 1),
                        node_type: NodeType::Paragraph,
                        children: vec![RawChild::Leaf {
                            id: pagebreak,
                            item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                        }],
                    }),
                    RawChild::Block(RawNode {
                        attrs: vec![],
                        id: unknown_block,
                        node_type: NodeType::Unknown,
                        children: vec![RawChild::Leaf {
                            id: unknown_child,
                            item: super::super::SeqItem::Char('u'),
                        }],
                    }),
                ],
            }],
        };
        let out = normalize(tree);

        fn find(n: &RawNode, id: Dot) -> Option<&RawNode> {
            if n.id == id {
                return Some(n);
            }
            n.child_blocks().into_iter().find_map(|c| find(c, id))
        }
        fn has_pagebreak(n: &RawNode) -> bool {
            n.children.iter().any(|c| match c {
                RawChild::Leaf { item, .. } => item.as_child_type() == Some(NodeType::PageBreak),
                RawChild::Block(b) => has_pagebreak(b),
            })
        }

        let root = &out.roots[0];
        let unknown =
            find(root, unknown_block).expect("unknown block must survive context filtering");
        assert!(
            matches!(
                &unknown.children[0],
                RawChild::Leaf { id, item: super::super::SeqItem::Char('u') } if *id == unknown_child
            ),
            "unknown block's own child must attach normally, untouched"
        );
        assert!(
            !has_pagebreak(root),
            "the PageBreak nested two levels deep (Blockquote>Paragraph) is context-invalid \
             and must still be dropped — proves drop_context_invalid_here actually ran"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_block_through_promote() {
        let promoted_para = Dot::new(1, 2);
        let unknown_block = Dot::new(1, 9);
        let unknown_child = Dot::new(1, 10);
        let node = RawNode {
            attrs: vec![],
            id: Dot::ROOT,
            node_type: NodeType::Root,
            children: vec![
                RawChild::Block(RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 1),
                    node_type: NodeType::ListItem,
                    children: vec![RawChild::Block(RawNode {
                        attrs: vec![],
                        id: promoted_para,
                        node_type: NodeType::Paragraph,
                        children: vec![],
                    })],
                }),
                RawChild::Block(RawNode {
                    attrs: vec![],
                    id: unknown_block,
                    node_type: NodeType::Unknown,
                    children: vec![RawChild::Leaf {
                        id: unknown_child,
                        item: super::super::SeqItem::Char('z'),
                    }],
                }),
            ],
        };
        let out = normalize(RawTree { roots: vec![node] });
        assert!(
            out.roots[0]
                .children
                .iter()
                .any(|c| matches!(c, RawChild::Block(b) if b.id == promoted_para)),
            "promoted Paragraph must survive"
        );
        let unknown = out.roots[0]
            .children
            .iter()
            .find_map(|c| match c {
                RawChild::Block(b) if b.id == unknown_block => Some(b),
                _ => None,
            })
            .expect("unknown block must survive the promote cascade");
        assert!(
            matches!(
                &unknown.children[0],
                RawChild::Leaf { id, item: super::super::SeqItem::Char('z') } if *id == unknown_child
            ),
            "unknown block's own child must attach normally, untouched"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_atom_leaf_unknown_bearing_leaf() {
        let para = Dot::new(1, 1);
        let block_atom_unknown = Dot::new(1, 2);
        let pb1 = Dot::new(1, 3);
        let pb2 = Dot::new(1, 4);
        let node = RawNode {
            attrs: vec![],
            id: para,
            node_type: NodeType::Paragraph,
            children: vec![
                RawChild::Leaf {
                    id: block_atom_unknown,
                    item: super::super::SeqItem::BlockAtom {
                        leaf: AtomLeaf::Unknown(crate::nodes::UnknownNode),
                        parents: vec![para],
                    },
                },
                RawChild::Leaf {
                    id: pb1,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
                RawChild::Leaf {
                    id: pb2,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
            ],
        };
        let out = normalize(RawTree { roots: vec![node] });
        assert!(
            matches!(
                find_leaf(&out, block_atom_unknown),
                Some(super::super::SeqItem::BlockAtom {
                    leaf: AtomLeaf::Unknown(_),
                    ..
                })
            ),
            "an AtomLeaf::Unknown-bearing leaf must survive normalize unmodified"
        );
        assert!(
            find_leaf(&out, pb1).is_some(),
            "matching first PageBreak kept"
        );
        assert!(
            find_leaf(&out, pb2).is_none(),
            "unmatched-drop must still drop the extra PageBreak"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_wrapped_table_nested_in_table() {
        let unknown_block = Dot::new(1, 1);
        let inner_table = Dot::new(1, 2);
        let node = RawNode {
            attrs: vec![],
            id: Dot::new(1, 0),
            node_type: NodeType::Table,
            children: vec![RawChild::Block(RawNode {
                attrs: vec![],
                id: unknown_block,
                node_type: NodeType::Unknown,
                children: vec![RawChild::Block(RawNode {
                    attrs: vec![],
                    id: inner_table,
                    node_type: NodeType::Table,
                    children: vec![],
                })],
            })],
        };
        let out = normalize(RawTree { roots: vec![node] });

        fn find(n: &RawNode, id: Dot) -> Option<&RawNode> {
            if n.id == id {
                return Some(n);
            }
            n.child_blocks().into_iter().find_map(|c| find(c, id))
        }

        let root = &out.roots[0];
        let unknown = find(root, unknown_block).expect("unknown block must survive normalize");
        assert_eq!(unknown.node_type, NodeType::Unknown);
        let inner = find(unknown, inner_table).expect("nested table inside unknown must survive");
        assert_eq!(inner.node_type, NodeType::Table);
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn project_document_keeps_unknown_leaf_as_one_slot() {
        use crate::projection::{DocLogs, project_document};
        use crate::{AliasLog, ModifierAttrLog, NodeAttrLog, SpanLog};
        use editor_crdt::{InputEvent, ListOp, build_oplog};

        let para = Dot::new(1, 1);
        let unknown = Dot::new(1, 2);
        let ch = Dot::new(1, 3);
        let items = [
            (
                para,
                super::super::SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (
                unknown,
                super::super::SeqItem::Unknown {
                    tag: 999,
                    bytes: vec![0xAA],
                },
            ),
            (ch, super::super::SeqItem::Char('a')),
        ];
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
        let logs = DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
            aliases: AliasLog::new(),
        };
        let pd = project_document(&logs).unwrap();
        let p = pd.tree.get(para).expect("paragraph present");
        assert_eq!(
            p.children.len(),
            2,
            "unknown 리프가 투영 문서에서 1 슬롯을 점유해야 한다"
        );
        assert!(matches!(
            &p.children[0],
            crate::seq::Child::Leaf { id, item: super::super::SeqItem::Unknown { tag: 999, .. } } if *id == unknown
        ));
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;
        use strum::IntoEnumIterator;

        #[derive(Clone, Debug)]
        enum Shape {
            Leaf(super::super::super::SeqItem),
            Block {
                node_type: NodeType,
                children: Vec<Shape>,
            },
        }

        fn arb_leaf() -> impl Strategy<Value = Shape> {
            prop_oneof![
                any::<char>().prop_map(|c| Shape::Leaf(super::super::super::SeqItem::Char(c))),
                Just(Shape::Leaf(super::super::super::SeqItem::Atom(
                    AtomLeaf::HardBreak
                ))),
                Just(Shape::Leaf(super::super::super::SeqItem::Atom(
                    AtomLeaf::Tab
                ))),
                Just(Shape::Leaf(super::super::super::SeqItem::Atom(
                    AtomLeaf::PageBreak
                ))),
            ]
        }

        fn arb_block_type() -> impl Strategy<Value = NodeType> {
            let types: Vec<NodeType> = NodeType::iter()
                .filter(|t| !matches!(t, NodeType::HardBreak | NodeType::Tab | NodeType::PageBreak))
                .collect();
            prop::sample::select(types)
        }

        fn arb_block(depth: u32) -> impl Strategy<Value = Shape> {
            arb_leaf().prop_recursive(depth, 64, 4, move |inner| {
                (arb_block_type(), prop::collection::vec(inner, 0..4)).prop_map(
                    |(node_type, children)| Shape::Block {
                        node_type,
                        children,
                    },
                )
            })
        }

        fn arb_any_block_tree(depth: u32) -> impl Strategy<Value = RawTree> {
            let block = arb_block(depth)
                .prop_filter("roots는 블록만", |s| matches!(s, Shape::Block { .. }));
            prop::collection::vec(block, 0..4).prop_map(|roots| {
                let mut next = 0u64;
                RawTree {
                    roots: roots.iter().map(|s| build(s, &mut next)).collect(),
                }
            })
        }

        fn build(s: &Shape, next: &mut u64) -> RawNode {
            let id = Dot::new(1, *next);
            *next += 1;
            match s {
                Shape::Block {
                    node_type,
                    children,
                } => RawNode {
                    attrs: vec![],
                    id,
                    node_type: *node_type,
                    children: children.iter().map(|c| build_child(c, next)).collect(),
                },
                Shape::Leaf(_) => unreachable!("roots/children filtered/handled elsewhere"),
            }
        }

        fn build_child(s: &Shape, next: &mut u64) -> RawChild {
            match s {
                Shape::Leaf(item) => {
                    let id = Dot::new(1, *next);
                    *next += 1;
                    RawChild::Leaf {
                        id,
                        item: item.clone(),
                    }
                }
                Shape::Block { .. } => RawChild::Block(build(s, next)),
            }
        }

        proptest! {
            #[test]
            fn normalize_makes_any_tree_valid(tree in arb_any_block_tree(6)) {
                let a = normalize(tree.clone());
                prop_assert!(valid(&a).is_ok());
                prop_assert_eq!(normalize(tree), a.clone());
                prop_assert_eq!(normalize(a.clone()), a);
            }
        }

        fn tcell(i: u64) -> RawChild {
            RawChild::Block(RawNode {
                attrs: vec![],
                id: Dot::new(2, i),
                node_type: NodeType::TableCell,
                children: vec![],
            })
        }
        fn trow(id: u64, cells: Vec<RawChild>) -> RawChild {
            RawChild::Block(RawNode {
                attrs: vec![],
                id: Dot::new(2, id),
                node_type: NodeType::TableRow,
                children: cells,
            })
        }
        fn root_with_table(rows: Vec<RawChild>) -> RawTree {
            RawTree {
                roots: vec![RawNode {
                    attrs: vec![],
                    id: Dot::new(2, 100),
                    node_type: NodeType::Root,
                    children: vec![RawChild::Block(RawNode {
                        attrs: vec![],
                        id: Dot::new(2, 0),
                        node_type: NodeType::Table,
                        children: rows,
                    })],
                }],
            }
        }
        fn table_widths(tree: &RawTree) -> Vec<usize> {
            fn find_table(n: &RawNode) -> Option<&RawNode> {
                if n.node_type == NodeType::Table {
                    return Some(n);
                }
                n.child_blocks().into_iter().find_map(find_table)
            }
            let table = tree.roots.iter().find_map(find_table).expect("table");
            table
                .children
                .iter()
                .filter_map(|c| match c {
                    RawChild::Block(r) if r.node_type == NodeType::TableRow => Some(
                        r.children
                            .iter()
                            .filter(|cc| cc.as_child_type() == Some(NodeType::TableCell))
                            .count(),
                    ),
                    _ => None,
                })
                .collect()
        }

        fn arb_table() -> impl Strategy<Value = RawTree> {
            prop::collection::vec(0usize..5, 1..5).prop_map(|row_sizes| {
                let mut next = 10u64;
                let rows: Vec<RawChild> = row_sizes
                    .into_iter()
                    .map(|n| {
                        let row_id = next;
                        next += 1;
                        let cells: Vec<RawChild> = (0..n)
                            .map(|_| {
                                let c = tcell(next);
                                next += 1;
                                c
                            })
                            .collect();
                        trow(row_id, cells)
                    })
                    .collect();
                root_with_table(rows)
            })
        }

        proptest! {
            #[test]
            fn normalize_table_is_rectangular(t in arb_table()) {
                let out = normalize(t);
                let widths = table_widths(&out);
                if let Some(&first) = widths.first() {
                    prop_assert!(widths.iter().all(|&w| w == first), "ragged: {widths:?}");
                }
                prop_assert!(valid(&out).is_ok());
            }

            #[test]
            fn normalize_table_idempotent(t in arb_table()) {
                let once = normalize(t);
                let twice = normalize(once.clone());
                prop_assert_eq!(once, twice);
            }

            #[test]
            fn normalize_width_matches_max_reference(row_sizes in prop::collection::vec(0usize..5, 1..5)) {
                let mut next = 10u64;
                let rows: Vec<RawChild> = row_sizes
                    .iter()
                    .map(|&n| {
                        let row_id = next;
                        next += 1;
                        let cells: Vec<RawChild> = (0..n).map(|_| { let c = tcell(next); next += 1; c }).collect();
                        trow(row_id, cells)
                    })
                    .collect();
                let out = normalize(root_with_table(rows));
                let widths = table_widths(&out);
                let reference = row_sizes.iter().map(|&n| n.max(1)).max().unwrap_or(1);
                prop_assert!(widths.iter().all(|&w| w == reference), "widths {widths:?} != ref {reference}");
            }
        }
    }
}
