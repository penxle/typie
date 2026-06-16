use editor_model::NodeId;
use std::ops::Range;
use std::sync::Arc;

use crate::glyph_run::{GlyphRun, RubyAnnotation};
use crate::style::BoxStyle;

#[derive(Debug)]
pub struct MeasuredTree {
    pub root: MeasuredNode,
}

/// A container's measured children as an order-statistics sum tree keyed by each
/// child's height. The container height is the root aggregate (`O(1)`), and a
/// single re-measured child is swapped in `O(log N)` (incremental measure),
/// instead of rebuilding the whole `Vec` every keystroke. `block_slots` locates
/// a child by its `NodeId` so the swap needs no scan.
#[derive(Debug, Clone, Default)]
pub struct MeasuredChildren {
    tree: editor_common::SumTree<Arc<MeasuredNode>, f32>,
    block_slots: Arc<hashbrown::HashMap<NodeId, usize>>,
}

fn block_node_id(node: &MeasuredNode) -> Option<NodeId> {
    match &node.content {
        MeasuredContent::Box(b) => Some(b.node_id),
        MeasuredContent::Line(l) => Some(l.node_id),
        MeasuredContent::Atom(a) => Some(a.node_id),
        MeasuredContent::Spacing(_) | MeasuredContent::PageBreak => None,
    }
}

impl MeasuredChildren {
    pub fn from_blocks(blocks: Vec<Arc<MeasuredNode>>) -> Self {
        let mut block_slots = hashbrown::HashMap::with_capacity(blocks.len());
        let items = blocks
            .into_iter()
            .enumerate()
            .map(|(slot, b)| {
                if let Some(id) = block_node_id(&b) {
                    block_slots.insert(id, slot);
                }
                let height = b.height;
                (b, height)
            })
            .collect();
        Self {
            tree: editor_common::SumTree::from_items(items),
            block_slots: Arc::new(block_slots),
        }
    }

    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Sum of child heights — `O(1)`.
    pub fn total_height(&self) -> f32 {
        self.tree.total_size()
    }

    pub fn get(&self, index: usize) -> Option<&Arc<MeasuredNode>> {
        self.tree.get(index)
    }

    pub fn first(&self) -> Option<&Arc<MeasuredNode>> {
        self.tree.get(0)
    }

    pub fn iter(&self) -> editor_common::Iter<'_, Arc<MeasuredNode>, f32> {
        self.tree.iter()
    }

    /// Swaps a re-measured child in place (item + height) — `O(log N)`. The
    /// caller guarantees `node` keeps the same `NodeId`, so `block_slots` stays
    /// valid.
    pub fn set(&mut self, index: usize, node: Arc<MeasuredNode>) -> bool {
        let height = node.height;
        self.tree.set(index, node, height)
    }

    /// Swaps a re-measured block child located by `NodeId` — `O(log N)`.
    /// Returns `false` when `node_id` is not a current block child, signalling
    /// the caller to fall back to a full rebuild.
    pub fn set_block(&mut self, node_id: NodeId, node: Arc<MeasuredNode>) -> bool {
        match self.block_slots.get(&node_id) {
            Some(&slot) => self.set(slot, node),
            None => false,
        }
    }
}

impl std::ops::Index<usize> for MeasuredChildren {
    type Output = Arc<MeasuredNode>;
    fn index(&self, index: usize) -> &Arc<MeasuredNode> {
        self.tree
            .get(index)
            .expect("MeasuredChildren: index out of bounds")
    }
}

impl<'a> IntoIterator for &'a MeasuredChildren {
    type Item = &'a Arc<MeasuredNode>;
    type IntoIter = editor_common::Iter<'a, Arc<MeasuredNode>, f32>;
    fn into_iter(self) -> Self::IntoIter {
        self.tree.iter()
    }
}

impl FromIterator<Arc<MeasuredNode>> for MeasuredChildren {
    fn from_iter<I: IntoIterator<Item = Arc<MeasuredNode>>>(iter: I) -> Self {
        Self::from_blocks(iter.into_iter().collect())
    }
}

#[derive(Debug, Clone)]
pub struct MeasuredNode {
    pub width: f32,
    pub height: f32,
    pub content: MeasuredContent,
}

#[derive(Debug, Clone)]
pub enum MeasuredContent {
    Box(MeasuredBox),
    Line(MeasuredLine),
    Atom(MeasuredAtom),
    Spacing(f32),
    PageBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageBreakPolicy {
    #[default]
    Auto,
    Avoid,
}

impl MeasuredNode {
    pub(crate) fn page_break_policy(&self) -> PageBreakPolicy {
        match &self.content {
            MeasuredContent::Box(b) => b.page_break_policy,
            MeasuredContent::Line(_) | MeasuredContent::Atom(_) => PageBreakPolicy::Avoid,
            MeasuredContent::Spacing(_) | MeasuredContent::PageBreak => PageBreakPolicy::Auto,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeasuredBox {
    pub node_id: NodeId,
    pub style: BoxStyle,
    pub children: MeasuredChildren,
    pub page_break_policy: PageBreakPolicy,
}

#[derive(Debug, Clone)]
pub struct MeasuredLine {
    pub node_id: NodeId,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub cursor_ascent: f32,
    pub cursor_descent: f32,
    pub glyph_runs: Vec<GlyphRun>,
    pub ruby_annotations: Vec<RubyAnnotation>,
    pub empty_caret_x: f32,
    /// Paragraph child-offset interval this visual line owns for matching
    /// container-anchored cursor positions. Matching is inclusive of both
    /// endpoints (`start <= offset && offset <= end`, not `Range::contains`).
    /// `None` for soft-wrap interior lines of a multi-line text segment —
    /// those lines own no paragraph boundary.
    pub child_range: Option<Range<usize>>,
    pub tab_gaps: Vec<TabGap>,
    pub is_phantom: bool,
    pub content_edge_x: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct TabGap {
    pub node_id: NodeId,
    pub child_index: usize,
    pub x: f32,
    pub width: f32,
}

#[derive(Debug, Clone)]
pub struct MeasuredAtom {
    pub node_id: NodeId,
}
