use editor_crdt::Dot;

use super::SeqItem;
use crate::nodes::{NodeAttr, NodeType};
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

/// The projected document tree, addressed by `Dot`. Every block lives in `nodes`
/// keyed by its id, so locating a block is `O(1)` (a hash lookup) instead of an
/// `O(N)` descent — the property the per-character paste path needs. A block's
/// ordered children live in its `ChildList`: inline leaves are stored by value,
/// child blocks are referenced by `Dot` (resolved through `nodes`). `nodes` is a
/// persistent `imbl::HashMap` and `ChildList` a persistent `SumTree`, so cloning a
/// whole tree is `O(1)` (structural sharing) — cloning a projection per
/// transaction stays cheap.
#[derive(Clone, Debug)]
pub struct BlockTree {
    pub nodes: imbl::HashMap<Dot, BlockNode>,
    pub root: Dot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockNode {
    pub id: Dot,
    pub node_type: NodeType,
    pub attrs: Vec<NodeAttr>,
    pub children: ChildList,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Child {
    Leaf { id: Dot, item: SeqItem },
    Block(Dot),
}

impl PartialEq for BlockTree {
    fn eq(&self, o: &Self) -> bool {
        self.root == o.root && self.nodes == o.nodes
    }
}

impl BlockTree {
    pub fn get(&self, id: Dot) -> Option<&BlockNode> {
        self.nodes.get(&id)
    }
    pub fn get_mut(&mut self, id: Dot) -> Option<&mut BlockNode> {
        self.nodes.get_mut(&id)
    }
    pub fn root_node(&self) -> Option<&BlockNode> {
        self.nodes.get(&self.root)
    }
    /// The `BlockNode` a `Child::Block` refers to (`None` for a leaf or a dangling
    /// reference).
    pub fn child_block(&self, c: &Child) -> Option<&BlockNode> {
        match c {
            Child::Block(id) => self.nodes.get(id),
            Child::Leaf { .. } => None,
        }
    }
    /// The child's node type — a leaf's item type, or a block's `node_type`
    /// (resolved through `nodes`). `Root` for a dangling reference (never happens
    /// for a well-formed tree). `None` for an unknown leaf (no schema-checkable
    /// type).
    pub fn child_type(&self, c: &Child) -> Option<NodeType> {
        match c {
            Child::Leaf { item, .. } => item.as_child_type(),
            Child::Block(id) => Some(
                self.nodes
                    .get(id)
                    .map(|b| b.node_type)
                    .unwrap_or(NodeType::Root),
            ),
        }
    }

    /// Build the flat tree from a freshly projected/normalized nested scratch tree.
    /// `O(N)`.
    pub fn from_raw(raw: &RawTree) -> Self {
        fn add(raw: &RawNode, nodes: &mut imbl::HashMap<Dot, BlockNode>) {
            let children: ChildList = raw
                .children
                .iter()
                .map(|c| match c {
                    RawChild::Leaf { id, item } => Child::Leaf {
                        id: *id,
                        item: item.clone(),
                    },
                    RawChild::Block(b) => {
                        add(b, nodes);
                        Child::Block(b.id)
                    }
                })
                .collect();
            nodes.insert(
                raw.id,
                BlockNode {
                    id: raw.id,
                    node_type: raw.node_type,
                    attrs: raw.attrs.clone(),
                    children,
                },
            );
        }
        let mut nodes = imbl::HashMap::new();
        let root = raw.roots.first().map(|r| r.id).unwrap_or(Dot::ROOT);
        // Only the canonical (first) root is reachable; `normalize` guarantees a
        // single `Root`, so this never orphans real content.
        if let Some(r) = raw.roots.first() {
            add(r, &mut nodes);
        }
        BlockTree { nodes, root }
    }

    /// Insert a nested scratch block (and its whole subtree) into `nodes`, returning
    /// its id. Used to graft a freshly re-projected window subtree into the live tree.
    pub fn insert_block_subtree(&mut self, raw: &RawNode) -> Dot {
        fn add(raw: &RawNode, nodes: &mut imbl::HashMap<Dot, BlockNode>) {
            let children: ChildList = raw
                .children
                .iter()
                .map(|c| match c {
                    RawChild::Leaf { id, item } => Child::Leaf {
                        id: *id,
                        item: item.clone(),
                    },
                    RawChild::Block(b) => {
                        add(b, nodes);
                        Child::Block(b.id)
                    }
                })
                .collect();
            nodes.insert(
                raw.id,
                BlockNode {
                    id: raw.id,
                    node_type: raw.node_type,
                    attrs: raw.attrs.clone(),
                    children,
                },
            );
        }
        add(raw, &mut self.nodes);
        raw.id
    }

    /// Remove `block` and its whole block subtree from `nodes`. Leaf children are
    /// stored inline so they need no separate removal.
    pub fn remove_block_subtree(&mut self, block: Dot) {
        let Some(node) = self.nodes.remove(&block) else {
            return;
        };
        for c in &node.children {
            if let Child::Block(id) = c {
                self.remove_block_subtree(*id);
            }
        }
    }

    /// Run `f` on `block`'s children if it exists, returning whether it did.
    /// `O(log N)` (a single map lookup, copy-on-write).
    pub fn with_block_children(&mut self, block: Dot, f: impl FnOnce(&mut ChildList)) -> bool {
        match self.nodes.get_mut(&block) {
            Some(node) => {
                f(&mut node.children);
                true
            }
            None => false,
        }
    }
}

// ===== Nested scratch tree =====
//
// `project_blocks` + `normalize` + `validate_block_tree` + `flatten` operate on
// this owned, physically-nested form — it suits recursive structural surgery
// (scaffolding, grandchild promotion, grid repair) where dropping a child just
// drops its owned subtree. It is converted to the flat `BlockTree` once per
// re-projection via `BlockTree::from_raw`.

#[derive(Clone, Debug, PartialEq)]
pub struct RawTree {
    pub roots: Vec<RawNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawNode {
    pub id: Dot,
    pub node_type: NodeType,
    pub attrs: Vec<NodeAttr>,
    pub children: Vec<RawChild>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RawChild {
    Leaf { id: Dot, item: SeqItem },
    Block(RawNode),
}

impl RawNode {
    pub fn child_blocks(&self) -> Vec<&RawNode> {
        self.children
            .iter()
            .filter_map(|c| match c {
                RawChild::Block(b) => Some(b),
                _ => None,
            })
            .collect()
    }
}

impl RawChild {
    pub fn as_child_type(&self) -> Option<NodeType> {
        match self {
            RawChild::Leaf { item, .. } => item.as_child_type(),
            RawChild::Block(b) => Some(b.node_type),
        }
    }
}

/// A block's ordered children, backed by a persistent order-statistics tree
/// (`SumTree`) summed by direct-leaf count. This makes positional edits
/// (`insert`/`remove` at a slot, and `leaf_slot` mapping a leaf offset to a child
/// slot) `O(log K)` instead of the `Vec`'s `O(K)`, and — being persistent — keeps
/// `clone` `O(1)`. The API mirrors `Vec<Child>` (iter/len/get/push/insert/remove/
/// index) so call sites that only read or append are unaffected.
#[derive(Clone, Debug, Default)]
pub struct ChildList {
    tree: editor_common::SumTree<Child, u64>,
}

fn child_leaf_size(c: &Child) -> u64 {
    match c {
        Child::Leaf { .. } => 1,
        Child::Block(_) => 0,
    }
}

impl ChildList {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.tree.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }
    pub fn iter(&self) -> editor_common::Iter<'_, Child, u64> {
        self.tree.iter()
    }
    pub fn get(&self, i: usize) -> Option<&Child> {
        self.tree.get(i)
    }
    pub fn first(&self) -> Option<&Child> {
        self.tree.get(0)
    }
    pub fn last(&self) -> Option<&Child> {
        self.len().checked_sub(1).and_then(|i| self.tree.get(i))
    }
    pub fn push(&mut self, c: Child) {
        let s = child_leaf_size(&c);
        self.tree.push(c, s);
    }
    /// Insert at a child *slot* index (counting blocks and leaves alike).
    pub fn insert(&mut self, slot: usize, c: Child) {
        let s = child_leaf_size(&c);
        self.tree.insert(slot, c, s);
    }
    /// Remove the child at a slot index, returning it if in range.
    pub fn remove(&mut self, slot: usize) -> Option<Child> {
        self.tree.remove(slot).map(|(c, _)| c)
    }
    /// The child *slot* holding the `leaf_offset`-th direct leaf child, or `len()`
    /// when the offset is at/after the last leaf — i.e. where a new leaf inserted at
    /// that leaf offset belongs. `O(log K)`.
    pub fn leaf_slot(&self, leaf_offset: usize) -> usize {
        self.tree
            .find_by_offset(leaf_offset as u64)
            .map(|(slot, _)| slot)
            .unwrap_or_else(|| self.len())
    }
    /// Number of direct leaf children — `O(1)`.
    pub fn leaf_count(&self) -> usize {
        self.tree.total_size() as usize
    }
    /// Number of direct leaf children in slots `[0, slot)` — the leaf ordinal a
    /// leaf at `slot` has inside the block's segment tree. `O(log K)`.
    pub fn leaf_ordinal_at(&self, slot: usize) -> usize {
        self.tree.offset_before(slot) as usize
    }
    /// Replace the child at `slot`. `O(log K)`.
    pub fn set(&mut self, slot: usize, c: Child) {
        let s = child_leaf_size(&c);
        self.tree.set(slot, c, s);
    }
    pub fn to_vec(&self) -> Vec<Child> {
        self.iter().cloned().collect()
    }
    pub fn extend(&mut self, iter: impl IntoIterator<Item = Child>) {
        for c in iter {
            self.push(c);
        }
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&Child) -> bool) {
        let kept: Vec<Child> = self.iter().filter(|c| keep(c)).cloned().collect();
        *self = ChildList::from_iter(kept);
    }
    /// Replace the children in the inclusive slot range with `replacement`, like
    /// `Vec::splice` (but returns nothing). `O((removed + inserted) · log K)`.
    pub fn splice(
        &mut self,
        range: std::ops::RangeInclusive<usize>,
        replacement: impl IntoIterator<Item = Child>,
    ) {
        let start = *range.start();
        let count = (*range.end() + 1).saturating_sub(start);
        for _ in 0..count {
            if start < self.len() {
                self.remove(start);
            } else {
                break;
            }
        }
        for (at, c) in (start..).zip(replacement) {
            self.insert(at, c);
        }
    }
    /// Split off children at `[at, len)`, leaving `[0, at)` in `self`.
    pub fn split_off(&mut self, at: usize) -> ChildList {
        let all = self.to_vec();
        let tail = all[at.min(all.len())..].to_vec();
        *self = ChildList::from_iter(all[..at.min(all.len())].iter().cloned());
        ChildList::from_iter(tail)
    }
}

impl From<Vec<Child>> for ChildList {
    fn from(v: Vec<Child>) -> Self {
        ChildList::from_iter(v)
    }
}

impl PartialEq for ChildList {
    fn eq(&self, o: &Self) -> bool {
        self.len() == o.len() && self.iter().eq(o.iter())
    }
}

impl<'a> IntoIterator for &'a ChildList {
    type Item = &'a Child;
    type IntoIter = editor_common::Iter<'a, Child, u64>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for ChildList {
    type Item = Child;
    type IntoIter = std::vec::IntoIter<Child>;
    fn into_iter(self) -> Self::IntoIter {
        self.tree.iter().cloned().collect::<Vec<_>>().into_iter()
    }
}

impl FromIterator<Child> for ChildList {
    fn from_iter<I: IntoIterator<Item = Child>>(iter: I) -> Self {
        // `O(n)` balanced build, so converting a `Vec<Child>` (normalize's rebuild
        // passes) costs the same as the `Vec` did — not `O(n log n)` of push-each.
        let items: Vec<(Child, u64)> = iter
            .into_iter()
            .map(|c| {
                let s = child_leaf_size(&c);
                (c, s)
            })
            .collect();
        Self {
            tree: editor_common::SumTree::from_items(items),
        }
    }
}

impl std::ops::Index<usize> for ChildList {
    type Output = Child;
    fn index(&self, i: usize) -> &Child {
        self.get(i).expect("child slot index out of bounds")
    }
}

#[derive(Debug, PartialEq)]
pub enum ProjectError {
    OrphanLeaf { id: Dot },
    AtomClassMismatch { id: Dot, leaf_type: NodeType },
}

enum ChildRef {
    Leaf { id: Dot, item: SeqItem },
    Block(usize),
}

struct BuildNode {
    id: Dot,
    node_type: NodeType,
    attrs: Vec<NodeAttr>,
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

pub fn project_blocks(items: &[(Dot, SeqItem)]) -> Result<RawTree, ProjectError> {
    let mut nodes: Vec<BuildNode> = vec![BuildNode {
        id: Dot::ROOT,
        node_type: NodeType::Root,
        attrs: vec![],
        children: Vec::new(),
    }];
    let mut stack: Vec<(Dot, usize)> = vec![(Dot::ROOT, 0)];

    for (id, item) in items {
        match item {
            SeqItem::Block {
                node_type,
                parents,
                attrs,
            } => {
                if !descend_stack(&mut stack, parents) {
                    continue;
                }
                let idx = nodes.len();
                nodes.push(BuildNode {
                    id: *id,
                    node_type: *node_type,
                    attrs: attrs.clone(),
                    children: Vec::new(),
                });
                let parent_idx = stack.last().expect("root is always present").1;
                nodes[parent_idx].children.push(ChildRef::Block(idx));
                stack.push((*id, idx));
            }
            SeqItem::Char(_) | SeqItem::Unknown { .. } => match stack.last() {
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
    Ok(RawTree { roots: vec![root] })
}

fn assemble(nodes: &mut [BuildNode], idx: usize) -> RawNode {
    let id = nodes[idx].id;
    let node_type = nodes[idx].node_type;
    let attrs = std::mem::take(&mut nodes[idx].attrs);
    let child_refs = std::mem::take(&mut nodes[idx].children);
    let children = child_refs
        .into_iter()
        .map(|c| match c {
            ChildRef::Leaf { id, item } => RawChild::Leaf { id, item },
            ChildRef::Block(child_idx) => RawChild::Block(assemble(nodes, child_idx)),
        })
        .collect();
    RawNode {
        id,
        node_type,
        attrs,
        children,
    }
}

pub fn flatten(tree: &RawTree) -> Vec<(Dot, SeqItem)> {
    fn as_dot(id: Dot) -> Dot {
        debug_assert!(
            id.as_op_dot().is_some(),
            "flatten on un-normalized tree (real op only)"
        );
        id
    }

    fn emit_children(children: &[RawChild], parents: &mut Vec<Dot>, out: &mut Vec<(Dot, SeqItem)>) {
        for c in children {
            match c {
                RawChild::Leaf { id, item } => {
                    let out_item = match item {
                        SeqItem::Atom(leaf) if leaf.is_block_level() => SeqItem::BlockAtom {
                            leaf: leaf.clone(),
                            parents: parents.clone(),
                        },
                        other => other.clone(),
                    };
                    out.push((as_dot(*id), out_item));
                }
                RawChild::Block(b) => walk(b, parents, out),
            }
        }
    }

    fn walk(node: &RawNode, parents: &mut Vec<Dot>, out: &mut Vec<(Dot, SeqItem)>) {
        let id = as_dot(node.id);
        out.push((
            id,
            SeqItem::Block {
                node_type: node.node_type,
                parents: parents.clone(),
                attrs: node.attrs.clone(),
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
    fn walk(
        tree: &BlockTree,
        node: &BlockNode,
        path: &mut Vec<NodeType>,
        check_own_context: bool,
    ) -> Result<(), SchemaError> {
        path.push(node.node_type);
        let kids: Vec<NodeType> = node
            .children
            .iter()
            .filter_map(|c| tree.child_type(c))
            .filter(|t| *t != NodeType::Unknown)
            .collect();
        node.node_type.spec().content.validate(&kids)?;
        if check_own_context {
            check_context(node.node_type, path)?;
        }
        // An `Unknown` placeholder's own children are opaque — no confidence in
        // their true schema, so their positional/context legality is never
        // checked here, for both leaves and block children (nested known
        // blocks still validate their own content and deeper descendants via
        // the `Child::Block` recursion below).
        for c in &node.children {
            match c {
                Child::Block(id) => {
                    if let Some(b) = tree.get(*id) {
                        walk(tree, b, path, node.node_type != NodeType::Unknown)?;
                    }
                }
                Child::Leaf { item, .. } => {
                    if node.node_type != NodeType::Unknown
                        && let Some(lt) = item.as_child_type()
                    {
                        path.push(lt);
                        check_context(lt, path)?;
                        path.pop();
                    }
                }
            }
        }
        path.pop();
        Ok(())
    }

    // An empty tree (no root node) is vacuously valid.
    let Some(root) = tree.root_node() else {
        return Ok(());
    };
    if root.node_type != NodeType::Root {
        return Err(SchemaError::RootViolation {
            roots: vec![root.node_type],
        });
    }
    walk(tree, root, &mut Vec::new(), true)
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

    fn valid(t: &RawTree) -> Result<(), SchemaError> {
        validate_block_tree(&BlockTree::from_raw(t))
    }

    #[test]
    fn leaf_ordinal_at_counts_leaves_before_slot() {
        let leaf = |c: char| Child::Leaf {
            id: Dot::new(1, 0),
            item: SeqItem::Char(c),
        };
        let children: ChildList = vec![
            leaf('a'),
            Child::Block(Dot::new(1, 1)),
            leaf('b'),
            leaf('c'),
        ]
        .into_iter()
        .collect();
        let got: Vec<usize> = (0..=children.len())
            .map(|slot| children.leaf_ordinal_at(slot))
            .collect();
        assert_eq!(got, vec![0, 1, 1, 2, 3]);
    }

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
                inner,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                    attrs: vec![],
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
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 1), SeqItem::Char('x')),
        ];
        let tree = project_blocks(&seq).expect("well-formed");
        assert_eq!(tree.roots.len(), 1);
        let para_node = tree.roots[0].child_blocks()[0];
        assert_eq!(para_node.children.len(), 1);
        assert!(
            matches!(&para_node.children[0], RawChild::Leaf { id, item } if *id == Dot::new(1, 1) && *item == SeqItem::Char('x'))
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
                attrs: vec![],
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
                    attrs: vec![],
                },
            ),
            (
                Dot::new(1, 5),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![ghost, bq],
                    attrs: vec![],
                },
            ),
        ];
        let tree = project_blocks(&seq).unwrap();
        assert_eq!(tree.roots[0].children.len(), 1);
        match &tree.roots[0].children[0] {
            RawChild::Block(b) => {
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
                    attrs: vec![],
                },
            ),
            (ax, SeqItem::Char('x')),
            (
                b,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, ghost],
                    attrs: vec![],
                },
            ),
            (by, SeqItem::Char('y')),
        ];
        let tree = project_blocks(&seq).unwrap();
        assert_eq!(tree.roots[0].children.len(), 1);
        match &tree.roots[0].children[0] {
            RawChild::Block(blk) => {
                assert_eq!(blk.id, a);
                assert_eq!(blk.children.len(), 1, "'y' must drop, not adopt into A");
                assert!(matches!(&blk.children[0], RawChild::Leaf { id, .. } if *id == ax));
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
                    attrs: vec![],
                },
            ),
            (title, SeqItem::Char('t')),
            (
                b,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, fold, ghost],
                    attrs: vec![],
                },
            ),
            (by, SeqItem::Char('y')),
        ];
        let tree = project_blocks(&seq).unwrap();
        // B dropped; matched prefix [ROOT, fold] keeps fold open, so 'y' attaches to fold
        // (deepest still-valid ancestor), not dropped at root.
        assert_eq!(tree.roots[0].children.len(), 1);
        match &tree.roots[0].children[0] {
            RawChild::Block(blk) => {
                assert_eq!(blk.id, fold);
                let leaves: Vec<Dot> = blk
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        RawChild::Leaf { id, .. } => Some(*id),
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
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('y')),
        ];
        let tree = project_blocks(&seq).unwrap();
        assert_eq!(tree.roots[0].children.len(), 1);
        match &tree.roots[0].children[0] {
            RawChild::Block(b) => {
                assert_eq!(b.id, para);
                assert_eq!(b.children.len(), 1);
                assert!(matches!(
                    &b.children[0],
                    RawChild::Leaf { id, .. } if *id == Dot::new(1, 2)
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
                    attrs: vec![],
                },
            ),
            (
                para_in,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                    attrs: vec![],
                },
            ),
            (
                para2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
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
                inner,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                    attrs: vec![],
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
    fn flatten_round_trips_block_attrs() {
        let callout = Dot::new(1, 1);
        let items = vec![(
            callout,
            SeqItem::Block {
                node_type: NodeType::Callout,
                parents: vec![Dot::ROOT],
                attrs: vec![crate::NodeAttr::Callout {
                    attr: crate::CalloutNodeAttr::Variant(crate::CalloutVariant::Warning),
                }],
            },
        )];
        let tree = project_blocks(&items).unwrap();
        let flat = flatten(&tree);
        let round = flat
            .iter()
            .find(|(d, _)| *d == callout)
            .map(|(_, item)| item.clone())
            .expect("callout present after flatten");
        assert_eq!(
            round, items[0].1,
            "flatten이 init attrs를 무손실 왕복해야 한다"
        );
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
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                    attrs: vec![],
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
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('x')),
            (
                Dot::new(1, 3),
                SeqItem::Atom(crate::seq::AtomLeaf::PageBreak),
            ),
        ];
        let tree = project_blocks(&items).expect("well-formed");
        valid(&tree).expect("schema valid");
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
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('x')),
        ];
        let tree = project_blocks(&items).expect("well-formed");
        assert!(matches!(valid(&tree), Err(SchemaError::InvalidContent(_))));
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
                    attrs: vec![],
                },
            ),
            (
                para_in,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                    attrs: vec![],
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
                    attrs: vec![],
                },
            ),
        ];
        let tree = project_blocks(&items).expect("well-formed");
        assert!(matches!(
            valid(&tree),
            Err(SchemaError::ContextViolation {
                node_type: NodeType::PageBreak,
                ..
            })
        ));
    }

    #[test]
    fn validate_empty_tree_is_ok() {
        valid(&RawTree { roots: vec![] }).expect("empty ok");
    }

    #[test]
    fn validate_rejects_non_root_top() {
        let tree = RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::new(1, 0),
                node_type: NodeType::Paragraph,
                children: vec![],
            }],
        };
        assert!(matches!(
            valid(&tree),
            Err(SchemaError::RootViolation { roots }) if roots == [NodeType::Paragraph]
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
                    attrs: vec![],
                },
            ),
            (
                ftitle,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![Dot::ROOT, fold],
                    attrs: vec![],
                },
            ),
            (
                fcontent,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![Dot::ROOT, fold],
                    attrs: vec![],
                },
            ),
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT, fold, fcontent],
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, fold, fcontent, bq],
                    attrs: vec![],
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
        assert!(matches!(&root_node.children[1], RawChild::Leaf { id, .. } if *id == img2));
        let fold_node = root_node.child_blocks()[0];
        let fcontent_node = fold_node.child_blocks()[1];
        assert_eq!(fcontent_node.node_type, NodeType::FoldContent);
        assert!(matches!(
            fcontent_node.children.last().unwrap(),
            RawChild::Leaf { id, .. } if *id == img1
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
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                    attrs: vec![],
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
            matches!(&root_node.children[0], RawChild::Block(b) if b.node_type == NodeType::Blockquote)
        );
        assert!(matches!(
            &root_node.children[1],
            RawChild::Leaf { id, item }
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
                    attrs: vec![],
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

    #[test]
    fn unknown_item_occupies_one_slot_as_leaf() {
        let para = Dot::new(1, 1);
        let unknown = Dot::new(1, 2);
        let ch = Dot::new(1, 3);
        let items = vec![
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
                    bytes: vec![0xAA, 0xBB],
                },
            ),
            (ch, SeqItem::Char('a')),
        ];
        let tree = project_blocks(&items).unwrap();
        let root = &tree.roots[0];
        let RawChild::Block(p) = &root.children[0] else {
            panic!("paragraph block expected");
        };
        assert_eq!(
            p.children.len(),
            2,
            "unknown 리프가 정확히 1 슬롯을 점유해야 한다"
        );
        assert!(
            matches!(&p.children[0], RawChild::Leaf { id, item: SeqItem::Unknown { tag: 999, .. } } if *id == unknown)
        );
        assert!(
            matches!(&p.children[1], RawChild::Leaf { id, item: SeqItem::Char('a') } if *id == ch)
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
                                attrs: vec![],
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
                let _ = valid(&tree);
            }
        }
    }
}
