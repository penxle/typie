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
        k.front().is_some_and(|c| e.matches(c.as_child_type()))
    };
    match expr {
        ContentExpr::Empty => {}
        ContentExpr::Single(t) => {
            if kids.front().is_some_and(|c| c.as_child_type() == *t) {
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

fn normalize_grid(table: &mut RawNode) {
    let width = table
        .children
        .iter()
        .filter_map(|c| match c {
            RawChild::Block(r) if r.node_type == NodeType::TableRow => Some(
                r.children
                    .iter()
                    .filter(|cc| cc.as_child_type() == NodeType::TableCell)
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
            .filter(|cc| cc.as_child_type() == NodeType::TableCell)
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
        ContentExpr::Empty => unreachable!(),
    }
}

fn drop_context_invalid_here(node: &mut RawNode, path: &[NodeType]) {
    node.children.retain(|c| {
        let t = c.as_child_type();
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
        drop_context_invalid_here(node, &path);
        let types: Vec<NodeType> = node.children.iter().map(|c| c.as_child_type()).collect();
        if node.node_type.spec().content.matches_sequence(&types) {
            break;
        } else if fixed_slot_roles(&node.node_type.spec().content).is_some() {
            reshape_fixed_slots(node);
        } else {
            repair_general(node);
        }
    }
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
        drop_context_invalid_here(node, path);
        let types: Vec<NodeType> = node.children.iter().map(|c| c.as_child_type()).collect();
        if node.node_type.spec().content.matches_sequence(&types) {
            break;
        } else if fixed_slot_roles(&node.node_type.spec().content).is_some() {
            reshape_fixed_slots(node);
        } else {
            repair_general(node);
        }
        recurse_block_children(&mut node.children, path);
    }
    if node.node_type == NodeType::Table {
        normalize_grid(node);
    }
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

    fn fold(kids: Vec<(u64, NodeType)>) -> RawNode {
        RawNode {
            id: Dot::new(1, 0),
            node_type: NodeType::Fold,
            children: kids
                .into_iter()
                .map(|(i, t)| {
                    RawChild::Block(RawNode {
                        id: Dot::new(1, i),
                        node_type: t,
                        children: vec![].into(),
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
            id: Dot::ROOT,
            node_type: NodeType::Root,
            children: vec![RawChild::Block(RawNode {
                id: Dot::new(1, 1),
                node_type: NodeType::Blockquote,
                children: vec![].into(),
            })]
            .into(),
        };
        repair_general(&mut node);

        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[0].as_child_type(), NodeType::Blockquote);
        assert!(matches!(
            &node.children[1],
            RawChild::Block(b) if b.node_type == NodeType::Paragraph && b.id.is_synthetic()
        ));
    }

    #[test]
    fn repair_drops_extra_leaf() {
        let mut node = RawNode {
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
            ]
            .into(),
        };
        repair_general(&mut node);

        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[0].as_child_type(), NodeType::Text);
        assert_eq!(node.children[1].as_child_type(), NodeType::PageBreak);
        assert!(matches!(
            &node.children[1],
            RawChild::Leaf { id, .. } if *id == Dot::new(1, 2)
        ));
    }

    #[test]
    fn repair_promotes_surplus_block_children() {
        let mut node = RawNode {
            id: Dot::ROOT,
            node_type: NodeType::Root,
            children: vec![RawChild::Block(RawNode {
                id: Dot::new(1, 1),
                node_type: NodeType::ListItem,
                children: vec![RawChild::Block(RawNode {
                    id: Dot::new(1, 2),
                    node_type: NodeType::Paragraph,
                    children: vec![].into(),
                })]
                .into(),
            })]
            .into(),
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
                .all(|c| c.as_child_type() == NodeType::Paragraph),
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
                id: Dot::new(1, 0),
                node_type: NodeType::Blockquote,
                children: vec![RawChild::Block(RawNode {
                    id: Dot::new(1, 1),
                    node_type: NodeType::Paragraph,
                    children: vec![RawChild::Leaf {
                        id: Dot::new(1, 2),
                        item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                    }]
                    .into(),
                })]
                .into(),
            }],
        };
        let out = normalize(tree);
        let has_pagebreak = |n: &RawNode| {
            fn walk(n: &RawNode) -> bool {
                n.children.iter().any(|c| match c {
                    RawChild::Leaf { item, .. } => item.as_child_type() == NodeType::PageBreak,
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
                id: Dot::new(1, 0),
                node_type: NodeType::Paragraph,
                children: vec![].into(),
            }],
        };
        let out = normalize(tree);
        assert_eq!(out.roots.len(), 1);
        assert_eq!(out.roots[0].node_type, NodeType::Root);
        assert!(
            out.roots[0]
                .children
                .iter()
                .any(|c| c.as_child_type() == NodeType::Paragraph)
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
                id: Dot::new(1, i),
                node_type: t,
                children: children.into(),
            })
        };
        let tree = RawTree {
            roots: vec![
                RawNode {
                    id: Dot::new(1, 1),
                    node_type: NodeType::Paragraph,
                    children: vec![leaf(2, super::super::SeqItem::Char('a'))].into(),
                },
                RawNode {
                    id: Dot::new(1, 3),
                    node_type: NodeType::Fold,
                    children: vec![].into(),
                },
                RawNode {
                    id: Dot::new(1, 4),
                    node_type: NodeType::Fold,
                    children: vec![
                        blk(5, NodeType::FoldContent, vec![]),
                        blk(6, NodeType::FoldTitle, vec![]),
                    ]
                    .into(),
                },
                RawNode {
                    id: Dot::new(1, 7),
                    node_type: NodeType::Blockquote,
                    children: vec![blk(
                        8,
                        NodeType::Paragraph,
                        vec![leaf(9, super::super::SeqItem::Atom(AtomLeaf::PageBreak))],
                    )]
                    .into(),
                },
                RawNode {
                    id: Dot::new(1, 10),
                    node_type: NodeType::BulletList,
                    children: vec![leaf(11, super::super::SeqItem::Char('z'))].into(),
                },
            ],
        };
        assert!(valid(&normalize(tree.clone())).is_ok());
        assert_eq!(normalize(normalize(tree.clone())), normalize(tree));
    }

    fn tcell(i: u64) -> RawChild {
        RawChild::Block(RawNode {
            id: Dot::new(2, i),
            node_type: NodeType::TableCell,
            children: vec![].into(),
        })
    }

    fn trow(id: u64, cells: Vec<RawChild>) -> RawChild {
        RawChild::Block(RawNode {
            id: Dot::new(2, id),
            node_type: NodeType::TableRow,
            children: cells.into(),
        })
    }

    fn ttable(rows: Vec<RawChild>) -> RawNode {
        RawNode {
            id: Dot::new(2, 0),
            node_type: NodeType::Table,
            children: rows.into(),
        }
    }

    fn cell_count(row: &RawChild) -> usize {
        match row {
            RawChild::Block(b) => b
                .children
                .iter()
                .filter(|c| c.as_child_type() == NodeType::TableCell)
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
                id: Dot::new(2, 100),
                node_type: NodeType::Root,
                children: vec![RawChild::Block(RawNode {
                    id: Dot::new(2, 0),
                    node_type: NodeType::Table,
                    children: rows.into(),
                })]
                .into(),
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
                        .filter(|cc| cc.as_child_type() == NodeType::TableCell)
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
                id: Dot::new(2, i),
                node_type: NodeType::TableCell,
                children: vec![].into(),
            })
        }
        fn trow(id: u64, cells: Vec<RawChild>) -> RawChild {
            RawChild::Block(RawNode {
                id: Dot::new(2, id),
                node_type: NodeType::TableRow,
                children: cells.into(),
            })
        }
        fn root_with_table(rows: Vec<RawChild>) -> RawTree {
            RawTree {
                roots: vec![RawNode {
                    id: Dot::new(2, 100),
                    node_type: NodeType::Root,
                    children: vec![RawChild::Block(RawNode {
                        id: Dot::new(2, 0),
                        node_type: NodeType::Table,
                        children: rows.into(),
                    })]
                    .into(),
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
                            .filter(|cc| cc.as_child_type() == NodeType::TableCell)
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
