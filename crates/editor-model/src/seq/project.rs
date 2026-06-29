use editor_crdt::Dot;

use super::SeqItem;
use crate::nodes::NodeType;
use crate::schema::{ContextExpr, SchemaError};

/// Stable, replica-independent 128-bit content hash (FNV-1a). Deterministic
/// across machines and binary versions, unlike `std`'s randomized hasher.
fn fnv1a_128(bytes: &[u8]) -> u128 {
    const OFFSET: u128 = 0x6c62272e07bb014262b821756295c58d;
    const PRIME: u128 = 0x0000000001000000000000000000013B;
    let mut h = OFFSET;
    for &b in bytes {
        h ^= b as u128;
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// Deterministic synthetic dot for a projection-scaffolded node, addressed by
/// its `(parent, slot, role)`. All replicas compute the same dot from the same
/// real ops; distinct from every real op dot and from sibling/other synthesized
/// nodes. `parent` may itself be synthetic (derived-under-derived chains).
pub fn synthetic_id(parent: Dot, slot: usize, role: NodeType) -> Dot {
    let mut bytes = [0u8; 32];
    bytes[0..8].copy_from_slice(&parent.actor.to_le_bytes());
    bytes[8..16].copy_from_slice(&parent.clock.to_le_bytes());
    bytes[16..24].copy_from_slice(&(slot as u64).to_le_bytes());
    bytes[24..32].copy_from_slice(&(role as u64).to_le_bytes());
    Dot::synthetic(fnv1a_128(&bytes))
}

/// The dot a node can be targeted by (modifiers/attrs/overlays), or `None` for a
/// transient scaffolded node. Real authored ops and the canonical implicit root
/// (`Dot::ROOT`, a permanent anchor) are targetable; other synthetic dots are not.
pub fn anchor_dot(id: Dot) -> Option<Dot> {
    (!id.is_synthetic() || id == Dot::ROOT).then_some(id)
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockTree {
    pub roots: Vec<BlockNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockNode {
    pub id: Dot,
    pub node_type: NodeType,
    pub children: Vec<Child>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Child {
    Leaf { id: Dot, item: SeqItem },
    Block(BlockNode),
}

#[derive(Debug, PartialEq)]
pub enum ProjectError {
    OrphanLeaf { id: Dot },
    AtomClassMismatch { id: Dot, leaf_type: NodeType },
}

impl BlockNode {
    pub fn child_blocks(&self) -> Vec<&BlockNode> {
        self.children
            .iter()
            .filter_map(|c| match c {
                Child::Block(b) => Some(b),
                _ => None,
            })
            .collect()
    }
}

impl Child {
    pub fn as_child_type(&self) -> NodeType {
        match self {
            Child::Leaf { item, .. } => item.as_child_type(),
            Child::Block(b) => b.node_type,
        }
    }
}

enum ChildRef {
    Leaf { id: Dot, item: SeqItem },
    Block(usize),
}

struct BuildNode {
    id: Dot,
    node_type: NodeType,
    children: Vec<ChildRef>,
}

fn chain_mismatch(stack: &[(Dot, usize)], parents: &[Dot]) -> Option<usize> {
    for (i, pid) in parents.iter().enumerate() {
        match stack.get(i) {
            Some((sid, _)) if sid == pid => continue,
            _ => return Some(i),
        }
    }
    None
}

fn descend_stack(stack: &mut Vec<(Dot, usize)>, parents: &[Dot]) -> bool {
    // The implicit root always occupies `stack[0]`, so never truncate below it.
    // On mismatch, keep only the matched valid-ancestor prefix so following inline
    // content attaches to the deepest still-live ancestor (or drops at the root).
    let (keep, descended) = match chain_mismatch(stack, parents) {
        Some(matched) => (matched.max(1), false),
        None => (parents.len().max(1), true),
    };
    if stack.len() > keep {
        stack.truncate(keep);
    }
    descended
}

pub fn project_blocks(items: &[(Dot, SeqItem)]) -> Result<BlockTree, ProjectError> {
    let mut nodes: Vec<BuildNode> = vec![BuildNode {
        id: Dot::ROOT,
        node_type: NodeType::Root,
        children: Vec::new(),
    }];
    let mut stack: Vec<(Dot, usize)> = vec![(Dot::ROOT, 0)];

    for (id, item) in items {
        match item {
            SeqItem::Block { node_type, parents } => {
                if !descend_stack(&mut stack, parents) {
                    continue;
                }
                let idx = nodes.len();
                nodes.push(BuildNode {
                    id: *id,
                    node_type: *node_type,
                    children: Vec::new(),
                });
                let parent_idx = stack.last().expect("root is always present").1;
                nodes[parent_idx].children.push(ChildRef::Block(idx));
                stack.push((*id, idx));
            }
            SeqItem::Char(_) => match stack.last() {
                Some((sid, parent_idx)) if *sid != Dot::ROOT => {
                    nodes[*parent_idx].children.push(ChildRef::Leaf {
                        id: *id,
                        item: item.clone(),
                    });
                }
                _ => {}
            },
            SeqItem::Atom(leaf) => {
                if leaf.is_block_level() {
                    return Err(ProjectError::AtomClassMismatch {
                        id: *id,
                        leaf_type: leaf.node_type(),
                    });
                }
                match stack.last() {
                    Some((sid, parent_idx)) if *sid != Dot::ROOT => {
                        nodes[*parent_idx].children.push(ChildRef::Leaf {
                            id: *id,
                            item: item.clone(),
                        });
                    }
                    _ => {}
                }
            }
            SeqItem::BlockAtom { leaf, parents } => {
                if !leaf.is_block_level() {
                    return Err(ProjectError::AtomClassMismatch {
                        id: *id,
                        leaf_type: leaf.node_type(),
                    });
                }
                if parents.is_empty() {
                    return Err(ProjectError::OrphanLeaf { id: *id });
                }
                if !descend_stack(&mut stack, parents) {
                    continue;
                }
                let parent_idx = stack.last().expect("root is always present").1;
                nodes[parent_idx].children.push(ChildRef::Leaf {
                    id: *id,
                    item: SeqItem::Atom(leaf.clone()),
                });
            }
        }
    }

    let root = assemble(&mut nodes, 0);
    Ok(BlockTree { roots: vec![root] })
}

fn assemble(nodes: &mut [BuildNode], idx: usize) -> BlockNode {
    let id = nodes[idx].id;
    let node_type = nodes[idx].node_type;
    let child_refs = std::mem::take(&mut nodes[idx].children);
    let children = child_refs
        .into_iter()
        .map(|c| match c {
            ChildRef::Leaf { id, item } => Child::Leaf { id, item },
            ChildRef::Block(child_idx) => Child::Block(assemble(nodes, child_idx)),
        })
        .collect();
    BlockNode {
        id,
        node_type,
        children,
    }
}

pub fn flatten(tree: &BlockTree) -> Vec<(Dot, SeqItem)> {
    fn as_dot(id: Dot) -> Dot {
        debug_assert!(
            id.as_op_dot().is_some(),
            "flatten on un-normalized tree (real op only)"
        );
        id
    }

    fn emit_children(children: &[Child], parents: &mut Vec<Dot>, out: &mut Vec<(Dot, SeqItem)>) {
        for c in children {
            match c {
                Child::Leaf { id, item } => {
                    let out_item = match item {
                        SeqItem::Atom(leaf) if leaf.is_block_level() => SeqItem::BlockAtom {
                            leaf: leaf.clone(),
                            parents: parents.clone(),
                        },
                        other => other.clone(),
                    };
                    out.push((as_dot(*id), out_item));
                }
                Child::Block(b) => walk(b, parents, out),
            }
        }
    }

    fn walk(node: &BlockNode, parents: &mut Vec<Dot>, out: &mut Vec<(Dot, SeqItem)>) {
        let id = as_dot(node.id);
        out.push((
            id,
            SeqItem::Block {
                node_type: node.node_type,
                parents: parents.clone(),
            },
        ));
        parents.push(id);
        emit_children(&node.children, parents, out);
        parents.pop();
    }

    let mut out = Vec::new();
    for root in &tree.roots {
        // The implicit root is never a stored op; emit its children under `Dot::ROOT`.
        let mut parents = vec![Dot::ROOT];
        emit_children(&root.children, &mut parents, &mut out);
    }
    out
}

pub fn validate_block_tree(tree: &BlockTree) -> Result<(), SchemaError> {
    fn walk(node: &BlockNode, path: &mut Vec<NodeType>) -> Result<(), SchemaError> {
        path.push(node.node_type);
        let kids: Vec<NodeType> = node.children.iter().map(|c| c.as_child_type()).collect();
        node.node_type.spec().content.validate(&kids)?;
        check_context(node.node_type, path)?;
        for c in &node.children {
            match c {
                Child::Block(b) => walk(b, path)?,
                Child::Leaf { item, .. } => {
                    let lt = item.as_child_type();
                    path.push(lt);
                    check_context(lt, path)?;
                    path.pop();
                }
            }
        }
        path.pop();
        Ok(())
    }

    if !tree.roots.is_empty() {
        let types: Vec<NodeType> = tree.roots.iter().map(|r| r.node_type).collect();
        if types.as_slice() != [NodeType::Root] {
            return Err(SchemaError::RootViolation { roots: types });
        }
    }

    for r in &tree.roots {
        walk(r, &mut Vec::new())?;
    }
    Ok(())
}

fn check_context(t: NodeType, path: &[NodeType]) -> Result<(), SchemaError> {
    let ctx = &t.spec().context;
    if *ctx == ContextExpr::Any || ctx.matches(path) {
        Ok(())
    } else {
        Err(SchemaError::ContextViolation {
            node_type: t,
            path: path.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_nested_blocks() {
        let para = Dot::new(1, 1);
        let bq = Dot::new(1, 4);
        let inner = Dot::new(1, 5);
        let seq = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('i')),
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                inner,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                },
            ),
            (Dot::new(1, 6), SeqItem::Char('y')),
            (Dot::new(1, 7), SeqItem::Char('o')),
        ];
        let tree = project_blocks(&seq).expect("well-formed");
        assert_eq!(tree.roots.len(), 1);
        let root_node = &tree.roots[0];
        assert_eq!(root_node.node_type, NodeType::Root);
        assert_eq!(root_node.id, Dot::ROOT);
        assert_eq!(root_node.child_blocks().len(), 2);
        assert_eq!(
            root_node.child_blocks()[1].child_blocks()[0].node_type,
            NodeType::Paragraph
        );
    }

    #[test]
    fn empty_sequence_is_implicit_root() {
        let tree = project_blocks(&[]).expect("empty ok");
        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0].node_type, NodeType::Root);
        assert_eq!(tree.roots[0].id, Dot::ROOT);
        assert!(tree.roots[0].children.is_empty());
    }

    #[test]
    fn single_block_with_leaf() {
        let para = Dot::new(1, 0);
        let seq = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 1), SeqItem::Char('x')),
        ];
        let tree = project_blocks(&seq).expect("well-formed");
        assert_eq!(tree.roots.len(), 1);
        let para_node = tree.roots[0].child_blocks()[0];
        assert_eq!(para_node.children.len(), 1);
        assert!(
            matches!(&para_node.children[0], Child::Leaf { id, item } if *id == Dot::new(1, 1) && *item == SeqItem::Char('x'))
        );
    }

    #[test]
    fn malformed_parent_is_dropped() {
        let ghost = Dot::new(9, 9);
        let seq = vec![(
            Dot::new(1, 1),
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![ghost],
            },
        )];
        let tree = project_blocks(&seq).unwrap();
        assert!(tree.roots[0].children.is_empty());
    }

    #[test]
    fn corrupted_prefix_parent_chain_drops_block() {
        let bq = Dot::new(1, 4);
        let ghost = Dot::new(9, 9);
        let seq = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                Dot::new(1, 5),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![ghost, bq],
                },
            ),
        ];
        let tree = project_blocks(&seq).unwrap();
        assert_eq!(tree.roots[0].children.len(), 1);
        match &tree.roots[0].children[0] {
            Child::Block(b) => {
                assert_eq!(b.id, bq);
                assert!(b.children.is_empty());
            }
            _ => panic!("expected blockquote block"),
        }
    }

    #[test]
    fn dropped_block_drops_following_inline_not_prior_block() {
        let a = Dot::new(1, 1);
        let ax = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let by = Dot::new(1, 4);
        let ghost = Dot::new(9, 9);
        let seq = vec![
            (
                a,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (ax, SeqItem::Char('x')),
            (
                b,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, ghost],
                },
            ),
            (by, SeqItem::Char('y')),
        ];
        let tree = project_blocks(&seq).unwrap();
        assert_eq!(tree.roots[0].children.len(), 1);
        match &tree.roots[0].children[0] {
            Child::Block(blk) => {
                assert_eq!(blk.id, a);
                assert_eq!(blk.children.len(), 1, "'y' must drop, not adopt into A");
                assert!(matches!(&blk.children[0], Child::Leaf { id, .. } if *id == ax));
            }
            _ => panic!("expected paragraph A"),
        }
    }

    #[test]
    fn dropped_nested_block_promotes_following_inline_to_valid_ancestor() {
        let fold = Dot::new(1, 1);
        let title = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let by = Dot::new(1, 4);
        let ghost = Dot::new(9, 9);
        let seq = vec![
            (
                fold,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![Dot::ROOT],
                },
            ),
            (title, SeqItem::Char('t')),
            (
                b,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, fold, ghost],
                },
            ),
            (by, SeqItem::Char('y')),
        ];
        let tree = project_blocks(&seq).unwrap();
        // B dropped; matched prefix [ROOT, fold] keeps fold open, so 'y' attaches to fold
        // (deepest still-valid ancestor), not dropped at root.
        assert_eq!(tree.roots[0].children.len(), 1);
        match &tree.roots[0].children[0] {
            Child::Block(blk) => {
                assert_eq!(blk.id, fold);
                let leaves: Vec<Dot> = blk
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        Child::Leaf { id, .. } => Some(*id),
                        _ => None,
                    })
                    .collect();
                assert_eq!(leaves, vec![title, by]);
            }
            _ => panic!("expected fold-title block"),
        }
    }

    #[test]
    fn orphan_leaf_is_dropped() {
        let seq = vec![(Dot::new(1, 0), SeqItem::Char('x'))];
        let tree = project_blocks(&seq).unwrap();
        assert!(tree.roots[0].children.is_empty());
    }

    #[test]
    fn orphan_inline_atom_is_dropped() {
        use crate::seq::AtomLeaf;
        let seq = vec![(Dot::new(1, 0), SeqItem::Atom(AtomLeaf::HardBreak))];
        let tree = project_blocks(&seq).unwrap();
        assert!(tree.roots[0].children.is_empty());
    }

    #[test]
    fn orphan_leaf_before_block_dropped_rest_kept() {
        let para = Dot::new(1, 1);
        let seq = vec![
            (Dot::new(1, 0), SeqItem::Char('x')),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('y')),
        ];
        let tree = project_blocks(&seq).unwrap();
        assert_eq!(tree.roots[0].children.len(), 1);
        match &tree.roots[0].children[0] {
            Child::Block(b) => {
                assert_eq!(b.id, para);
                assert_eq!(b.children.len(), 1);
                assert!(matches!(
                    &b.children[0],
                    Child::Leaf { id, .. } if *id == Dot::new(1, 2)
                ));
            }
            _ => panic!("expected paragraph block"),
        }
    }

    #[test]
    fn sibling_after_nesting_pops_and_rematches() {
        let bq = Dot::new(1, 1);
        let para_in = Dot::new(1, 2);
        let para2 = Dot::new(1, 3);
        let seq = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                para_in,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                },
            ),
            (
                para2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
        ];
        let tree = project_blocks(&seq).expect("well-formed");
        let root_node = &tree.roots[0];
        assert_eq!(root_node.child_blocks().len(), 2);
        assert_eq!(root_node.child_blocks()[0].node_type, NodeType::Blockquote);
        assert_eq!(root_node.child_blocks()[1].node_type, NodeType::Paragraph);
        assert_eq!(
            root_node.child_blocks()[0].child_blocks()[0].node_type,
            NodeType::Paragraph
        );
    }

    fn sample_sequence() -> Vec<(Dot, SeqItem)> {
        let para = Dot::new(1, 1);
        let bq = Dot::new(1, 4);
        let inner = Dot::new(1, 5);
        vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('i')),
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                inner,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                },
            ),
            (Dot::new(1, 6), SeqItem::Char('y')),
            (Dot::new(1, 7), SeqItem::Char('o')),
        ]
    }

    #[test]
    fn project_then_flatten_is_identity() {
        let items = sample_sequence();
        let tree = project_blocks(&items).expect("well-formed");
        assert_eq!(flatten(&tree), items);
    }

    #[test]
    fn project_then_flatten_roundtrips_block_atom_after_nested_block() {
        use crate::nodes::HorizontalRuleVariant;
        use crate::seq::AtomLeaf;
        let bq = Dot::new(1, 1);
        let para = Dot::new(1, 2);
        let hr = Dot::new(1, 5);
        let seq = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('a')),
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: HorizontalRuleVariant::default(),
                    },
                    parents: vec![Dot::ROOT],
                },
            ),
        ];
        let tree = project_blocks(&seq).expect("well-formed");
        assert_eq!(flatten(&tree), seq);
    }

    #[test]
    fn validate_accepts_schema_valid_tree() {
        let para = Dot::new(1, 1);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('x')),
            (
                Dot::new(1, 3),
                SeqItem::Atom(crate::seq::AtomLeaf::PageBreak),
            ),
        ];
        let tree = project_blocks(&items).expect("well-formed");
        validate_block_tree(&tree).expect("schema valid");
    }

    #[test]
    fn validate_rejects_content_violation() {
        let bq = Dot::new(1, 1);
        let items = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('x')),
        ];
        let tree = project_blocks(&items).expect("well-formed");
        assert!(matches!(
            validate_block_tree(&tree),
            Err(SchemaError::InvalidContent(_))
        ));
    }

    #[test]
    fn validate_rejects_context_violation() {
        let bq = Dot::new(1, 1);
        let para_in = Dot::new(1, 2);
        let tail = Dot::new(1, 4);
        let items = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                para_in,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                },
            ),
            (
                Dot::new(1, 3),
                SeqItem::Atom(crate::seq::AtomLeaf::PageBreak),
            ),
            (
                tail,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
        ];
        let tree = project_blocks(&items).expect("well-formed");
        assert!(matches!(
            validate_block_tree(&tree),
            Err(SchemaError::ContextViolation {
                node_type: NodeType::PageBreak,
                ..
            })
        ));
    }

    #[test]
    fn validate_empty_tree_is_ok() {
        validate_block_tree(&BlockTree { roots: vec![] }).expect("empty ok");
    }

    #[test]
    fn validate_rejects_non_root_top() {
        let tree = BlockTree {
            roots: vec![BlockNode {
                id: Dot::new(1, 0),
                node_type: NodeType::Paragraph,
                children: vec![],
            }],
        };
        assert!(matches!(
            validate_block_tree(&tree),
            Err(SchemaError::RootViolation { roots }) if roots == [NodeType::Paragraph]
        ));
    }

    #[test]
    fn validate_rejects_multiple_roots() {
        let tree = BlockTree {
            roots: vec![
                BlockNode {
                    id: Dot::ROOT,
                    node_type: NodeType::Root,
                    children: vec![],
                },
                BlockNode {
                    id: Dot::new(1, 1),
                    node_type: NodeType::Root,
                    children: vec![],
                },
            ],
        };
        assert!(matches!(
            validate_block_tree(&tree),
            Err(SchemaError::RootViolation { roots }) if roots == [NodeType::Root, NodeType::Root]
        ));
    }

    #[test]
    fn block_atom_disambiguates_multi_accepting_ancestors() {
        use crate::nodes::HorizontalRuleVariant;
        use crate::seq::AtomLeaf;
        let fold = Dot::new(1, 1);
        let ftitle = Dot::new(1, 2);
        let fcontent = Dot::new(1, 3);
        let bq = Dot::new(1, 4);
        let para = Dot::new(1, 5);
        let img1 = Dot::new(1, 7);
        let img2 = Dot::new(1, 8);
        let hr = |variant| AtomLeaf::HorizontalRule { variant };
        let seq = vec![
            (
                fold,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                ftitle,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![Dot::ROOT, fold],
                },
            ),
            (
                fcontent,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![Dot::ROOT, fold],
                },
            ),
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT, fold, fcontent],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, fold, fcontent, bq],
                },
            ),
            (Dot::new(1, 6), SeqItem::Char('a')),
            (
                img1,
                SeqItem::BlockAtom {
                    leaf: hr(HorizontalRuleVariant::default()),
                    parents: vec![Dot::ROOT, fold, fcontent],
                },
            ),
            (
                img2,
                SeqItem::BlockAtom {
                    leaf: hr(HorizontalRuleVariant::default()),
                    parents: vec![Dot::ROOT],
                },
            ),
        ];
        let tree = project_blocks(&seq).expect("well-formed");
        let root_node = &tree.roots[0];
        assert_eq!(root_node.children.len(), 2);
        assert!(matches!(&root_node.children[1], Child::Leaf { id, .. } if *id == img2));
        let fold_node = root_node.child_blocks()[0];
        let fcontent_node = fold_node.child_blocks()[1];
        assert_eq!(fcontent_node.node_type, NodeType::FoldContent);
        assert!(matches!(
            fcontent_node.children.last().unwrap(),
            Child::Leaf { id, .. } if *id == img1
        ));
    }

    #[test]
    fn block_atom_after_nested_block_binds_to_shallow_parent() {
        use crate::nodes::HorizontalRuleVariant;
        use crate::seq::AtomLeaf;
        let bq = Dot::new(1, 1);
        let para = Dot::new(1, 2);
        let hr = Dot::new(1, 5);
        let seq = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('a')),
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: HorizontalRuleVariant::default(),
                    },
                    parents: vec![Dot::ROOT],
                },
            ),
        ];
        let tree = project_blocks(&seq).expect("well-formed");
        let root_node = &tree.roots[0];
        assert_eq!(root_node.children.len(), 2);
        assert!(
            matches!(&root_node.children[0], Child::Block(b) if b.node_type == NodeType::Blockquote)
        );
        assert!(matches!(
            &root_node.children[1],
            Child::Leaf { id, item }
                if *id == hr
                && matches!(item, SeqItem::Atom(AtomLeaf::HorizontalRule { .. }))
        ));
    }

    #[test]
    fn block_atom_empty_parents_is_orphan() {
        use crate::nodes::HorizontalRuleVariant;
        use crate::seq::AtomLeaf;
        let seq = vec![(
            Dot::new(1, 1),
            SeqItem::BlockAtom {
                leaf: AtomLeaf::HorizontalRule {
                    variant: HorizontalRuleVariant::default(),
                },
                parents: vec![],
            },
        )];
        assert_eq!(
            project_blocks(&seq).unwrap_err(),
            ProjectError::OrphanLeaf { id: Dot::new(1, 1) }
        );
    }

    #[test]
    fn block_atom_unknown_parent_is_dropped() {
        use crate::nodes::HorizontalRuleVariant;
        use crate::seq::AtomLeaf;
        let ghost = Dot::new(9, 9);
        let seq = vec![(
            Dot::new(1, 1),
            SeqItem::BlockAtom {
                leaf: AtomLeaf::HorizontalRule {
                    variant: HorizontalRuleVariant::default(),
                },
                parents: vec![ghost],
            },
        )];
        let tree = project_blocks(&seq).unwrap();
        assert!(tree.roots[0].children.is_empty());
    }

    #[test]
    fn block_level_atom_as_inline_atom_errors() {
        use crate::nodes::HorizontalRuleVariant;
        use crate::seq::AtomLeaf;
        let seq = vec![(
            Dot::new(1, 1),
            SeqItem::Atom(AtomLeaf::HorizontalRule {
                variant: HorizontalRuleVariant::default(),
            }),
        )];
        assert_eq!(
            project_blocks(&seq).unwrap_err(),
            ProjectError::AtomClassMismatch {
                id: Dot::new(1, 1),
                leaf_type: NodeType::HorizontalRule
            }
        );
    }

    #[test]
    fn inline_atom_as_block_atom_errors() {
        use crate::seq::AtomLeaf;
        let para = Dot::new(1, 1);
        let seq = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                Dot::new(1, 2),
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HardBreak,
                    parents: vec![Dot::ROOT, para],
                },
            ),
        ];
        assert_eq!(
            project_blocks(&seq).unwrap_err(),
            ProjectError::AtomClassMismatch {
                id: Dot::new(1, 2),
                leaf_type: NodeType::HardBreak
            }
        );
    }

    mod proptests {
        use super::*;
        use crate::nodes::HorizontalRuleVariant;
        use crate::seq::AtomLeaf;
        use editor_crdt::sequence::checkout;
        use editor_crdt::{InputEvent, ListOp, build_oplog};
        use proptest::prelude::*;

        #[derive(Clone, Debug)]
        enum Shape {
            Leaf(SeqItem),
            BlockAtom(AtomLeaf),
            Block {
                node_type: NodeType,
                children: Vec<Shape>,
            },
        }

        fn arb_leaf() -> impl Strategy<Value = Shape> {
            prop_oneof![
                any::<char>().prop_map(|c| Shape::Leaf(SeqItem::Char(c))),
                Just(Shape::Leaf(SeqItem::Atom(AtomLeaf::HardBreak))),
                Just(Shape::Leaf(SeqItem::Atom(AtomLeaf::Tab))),
                Just(Shape::Leaf(SeqItem::Atom(AtomLeaf::PageBreak))),
            ]
        }

        fn arb_block_atom() -> impl Strategy<Value = Shape> {
            use crate::nodes::ImageNode;
            prop_oneof![
                Just(Shape::BlockAtom(AtomLeaf::HorizontalRule {
                    variant: HorizontalRuleVariant::Line,
                })),
                Just(Shape::BlockAtom(AtomLeaf::Image {
                    node: ImageNode::default(),
                })),
            ]
        }

        fn arb_shape() -> impl Strategy<Value = Shape> {
            let block_types = prop_oneof![
                Just(NodeType::Paragraph),
                Just(NodeType::Blockquote),
                Just(NodeType::BulletList),
                Just(NodeType::ListItem),
                Just(NodeType::Callout),
            ];
            arb_leaf().prop_recursive(4, 32, 4, move |inner| {
                let child = prop_oneof![inner, arb_block_atom()];
                (block_types.clone(), prop::collection::vec(child, 0..4)).prop_map(
                    |(node_type, children)| Shape::Block {
                        node_type,
                        children,
                    },
                )
            })
        }

        fn arb_root() -> impl Strategy<Value = Vec<Shape>> {
            let block = arb_shape().prop_filter("top-level은 블록만", |s| {
                matches!(s, Shape::Block { .. })
            });
            prop::collection::vec(block, 0..4)
        }

        // The implicit root is never serialized; top-level blocks descend from `Dot::ROOT`.
        fn serialize(tops: &[Shape]) -> Vec<(Dot, SeqItem)> {
            fn walk(
                s: &Shape,
                next: &mut u64,
                parents: &mut Vec<Dot>,
                out: &mut Vec<(Dot, SeqItem)>,
            ) {
                let id = Dot::new(1, *next);
                *next += 1;
                match s {
                    Shape::Leaf(item) => out.push((id, item.clone())),
                    Shape::BlockAtom(leaf) => {
                        out.push((
                            id,
                            SeqItem::BlockAtom {
                                leaf: leaf.clone(),
                                parents: parents.clone(),
                            },
                        ));
                    }
                    Shape::Block {
                        node_type,
                        children,
                    } => {
                        out.push((
                            id,
                            SeqItem::Block {
                                node_type: *node_type,
                                parents: parents.clone(),
                            },
                        ));
                        parents.push(id);
                        for c in children {
                            walk(c, next, parents, out);
                        }
                        parents.pop();
                    }
                }
            }
            let mut out = Vec::new();
            let mut next = 0u64;
            let mut parents = vec![Dot::ROOT];
            for c in tops {
                walk(c, &mut next, &mut parents, &mut out);
            }
            out
        }

        fn to_events(items: &[(Dot, SeqItem)]) -> Vec<InputEvent<SeqItem>> {
            let mut out = Vec::new();
            let mut prev: Option<Dot> = None;
            for (i, (id, item)) in items.iter().enumerate() {
                out.push(InputEvent {
                    id: *id,
                    parents: prev.into_iter().collect(),
                    op: ListOp::Ins {
                        pos: i,
                        item: item.clone(),
                    },
                });
                prev = Some(*id);
            }
            out
        }

        proptest! {
            #[test]
            fn wellformed_projects_and_roundtrips(tops in arb_root()) {
                let items = serialize(&tops);
                let log = build_oplog(&to_events(&items));
                let replayed = checkout(&log);
                prop_assert_eq!(&replayed, &items);
                let tree = project_blocks(&replayed).expect("well-formed → Ok");
                prop_assert_eq!(flatten(&tree), replayed);
                let _ = validate_block_tree(&tree);
            }
        }
    }
}
