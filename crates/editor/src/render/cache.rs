use crate::layout::{Element, Page, PositionedNode};
use crate::model::NodeId;
use crate::render::geometry::CacheRect;
use crate::types::Point;
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};
use std::mem::Discriminant;
use std::rc::Rc;
use tiny_skia::Pixmap;

#[derive(Default, Clone)]
pub(super) struct PageRenderSnapshot {
    nodes: FxHashMap<SnapshotNodeKey, CacheRect>,
}

#[derive(Default, Clone)]
pub(super) struct PageLayoutSnapshot {
    nodes: FxHashMap<usize, CacheRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SnapshotNodeKey {
    Ptr(usize),
    LineElement {
        block_id: NodeId,
        line_idx: usize,
        layout_ptr: usize,
        scope_id: Option<NodeId>,
        default_text_color_hash: Option<u64>,
    },
    StableElement {
        kind: Discriminant<Element>,
        node_id: NodeId,
        signature: u64,
    },
}

impl PageRenderSnapshot {
    pub(super) fn from_page(page: &Page) -> Self {
        let mut nodes = FxHashMap::default();
        Self::collect(&page.root, Point::zero(), &mut nodes);
        Self { nodes }
    }

    pub(super) fn dirty_rects(&self, next: &Self) -> Vec<CacheRect> {
        let mut dirty_rects = Vec::new();

        for (ptr, prev_bounds) in &self.nodes {
            match next.nodes.get(ptr) {
                Some(next_bounds) if prev_bounds.approx_eq(*next_bounds) => {}
                Some(next_bounds) => {
                    dirty_rects.push(*prev_bounds);
                    dirty_rects.push(*next_bounds);
                }
                None => dirty_rects.push(*prev_bounds),
            }
        }

        for (ptr, next_bounds) in &next.nodes {
            if !self.nodes.contains_key(ptr) {
                dirty_rects.push(*next_bounds);
            }
        }

        dirty_rects
    }

    fn collect(
        positioned: &PositionedNode,
        offset: Point,
        out: &mut FxHashMap<SnapshotNodeKey, CacheRect>,
    ) {
        let pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if let Some(key) = Self::snapshot_key(positioned) {
            if let Some(bounds) = CacheRect::from_xywh(
                pos.x,
                pos.y,
                positioned.node.size.width,
                positioned.node.size.height,
            ) {
                out.insert(key, bounds);
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::collect(child, pos, out);
            }
        }
    }

    fn snapshot_key(positioned: &PositionedNode) -> Option<SnapshotNodeKey> {
        let element = positioned.node.element.as_ref()?;
        element.as_render()?;
        if matches!(element, Element::TableCell(_)) {
            return None;
        }
        if let Element::Line(line) = element {
            return Some(SnapshotNodeKey::LineElement {
                block_id: line.block_id,
                line_idx: line.line_idx,
                layout_ptr: Rc::as_ptr(&line.layout) as usize,
                scope_id: positioned.node.scope_id,
                default_text_color_hash: positioned
                    .node
                    .render_hints
                    .default_text_color
                    .as_ref()
                    .map(|value| Self::hash_str(value.as_str())),
            });
        }

        if element.as_wrapper().is_some() {
            if let Some(node_id) = element.block_id() {
                return Some(SnapshotNodeKey::StableElement {
                    kind: std::mem::discriminant(element),
                    node_id,
                    signature: Self::stable_element_signature(element),
                });
            }
        }

        Some(SnapshotNodeKey::Ptr(Rc::as_ptr(&positioned.node) as usize))
    }

    fn hash_str(value: &str) -> u64 {
        let mut hasher = FxHasher::default();
        value.hash(&mut hasher);
        hasher.finish()
    }

    fn stable_element_signature(element: &Element) -> u64 {
        let mut hasher = FxHasher::default();
        let hashed = element.hash_render_cache_signature(&mut hasher);
        debug_assert!(
            hashed,
            "stable_element_signature called for non-wrapper element: {:?}",
            element
        );

        hasher.finish()
    }
}

impl PageLayoutSnapshot {
    pub(super) fn from_page(page: &Page) -> Self {
        let mut nodes = FxHashMap::default();
        Self::collect(&page.root, Point::zero(), &mut nodes);
        Self { nodes }
    }

    pub(super) fn dirty_rects(&self, next: &Self) -> Vec<CacheRect> {
        let mut dirty_rects = Vec::new();

        for (ptr, prev_bounds) in &self.nodes {
            match next.nodes.get(ptr) {
                Some(next_bounds) if prev_bounds.approx_eq(*next_bounds) => {}
                Some(next_bounds) => {
                    dirty_rects.push(*prev_bounds);
                    dirty_rects.push(*next_bounds);
                }
                None => dirty_rects.push(*prev_bounds),
            }
        }

        for (ptr, next_bounds) in &next.nodes {
            if !self.nodes.contains_key(ptr) {
                dirty_rects.push(*next_bounds);
            }
        }

        dirty_rects
    }

    fn collect(positioned: &PositionedNode, offset: Point, out: &mut FxHashMap<usize, CacheRect>) {
        let pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if positioned.node.element.is_some() {
            if let Some(bounds) = CacheRect::from_xywh(
                pos.x,
                pos.y,
                positioned.node.size.width,
                positioned.node.size.height,
            ) {
                out.insert(Rc::as_ptr(&positioned.node) as usize, bounds);
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::collect(child, pos, out);
            }
        }
    }
}

pub(super) struct PageRenderCache {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) scale_factor: f64,
    pub(super) snapshot: PageRenderSnapshot,
    pub(super) snapshot_initialized: bool,
    pub(super) layout_snapshot: PageLayoutSnapshot,
    pub(super) layout_snapshot_initialized: bool,
    pub(super) base_pixmap: Pixmap,
}

impl PageRenderCache {
    pub(super) fn new(width: u32, height: u32, scale_factor: f64) -> Self {
        let base_pixmap = Pixmap::new(width.max(1), height.max(1)).unwrap();
        Self {
            width,
            height,
            scale_factor,
            snapshot: PageRenderSnapshot::default(),
            snapshot_initialized: false,
            layout_snapshot: PageLayoutSnapshot::default(),
            layout_snapshot_initialized: false,
            base_pixmap,
        }
    }
}

#[derive(Default)]
pub(super) struct CacheDebugFrame {
    pub(super) render_rects: Vec<CacheRect>,
    pub(super) full_repaint: bool,
    pub(super) cache_reused: bool,
    pub(super) layout_rects: Vec<CacheRect>,
    pub(super) full_relayout: bool,
    pub(super) layout_reused: bool,
}
