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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SnapshotNodeKey {
    Ptr(usize),
    RenderLine {
        block_id: NodeId,
        line_idx: usize,
        layout_ptr: usize,
        scope_id: Option<NodeId>,
        default_text_color_hash: Option<u64>,
    },
    RenderStableElement {
        kind: Discriminant<Element>,
        node_id: NodeId,
        signature: u64,
    },
}

impl PageRenderSnapshot {
    pub(super) fn from_page(page: &Page) -> Self {
        let mut nodes = FxHashMap::default();
        collect_snapshot(&page.root, Point::zero(), &mut nodes);
        Self { nodes }
    }

    pub(super) fn dirty_rects(&self, next: &Self) -> Vec<CacheRect> {
        dirty_rects_between(&self.nodes, &next.nodes)
    }
}

impl SnapshotNodeKey {
    fn for_positioned(positioned: &PositionedNode) -> Option<Self> {
        let element = positioned.node.element.as_ref()?;
        element.as_render()?;

        if matches!(element, Element::TableCell(_)) {
            return None;
        }

        if let Element::Line(line) = element {
            return Some(Self::RenderLine {
                block_id: line.block_id,
                line_idx: line.line_idx,
                layout_ptr: Rc::as_ptr(&line.layout) as usize,
                scope_id: positioned.node.scope_id,
                default_text_color_hash: positioned
                    .node
                    .render_hints
                    .default_text_color
                    .as_ref()
                    .map(|value| hash_str(value.as_str())),
            });
        }

        if element.as_wrapper().is_some()
            && let Some(node_id) = element.block_id()
        {
            return Some(Self::RenderStableElement {
                kind: std::mem::discriminant(element),
                node_id,
                signature: stable_element_signature(element),
            });
        }

        Some(Self::Ptr(Rc::as_ptr(&positioned.node) as usize))
    }
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

fn collect_snapshot(
    positioned: &PositionedNode,
    offset: Point,
    out: &mut FxHashMap<SnapshotNodeKey, CacheRect>,
) {
    let pos = Point::new(
        offset.x + positioned.position.x,
        offset.y + positioned.position.y,
    );

    if let Some(key) = SnapshotNodeKey::for_positioned(positioned)
        && let Some(bounds) = CacheRect::from_xywh(
            pos.x,
            pos.y,
            positioned.node.size.width,
            positioned.node.size.height,
        )
    {
        out.insert(key, bounds);
    }

    if let Some(children) = &positioned.node.children {
        for child in children {
            collect_snapshot(child, pos, out);
        }
    }
}

fn dirty_rects_between<K: Eq + Hash>(
    prev: &FxHashMap<K, CacheRect>,
    next: &FxHashMap<K, CacheRect>,
) -> Vec<CacheRect> {
    let mut dirty_rects = Vec::new();

    for (key, prev_bounds) in prev {
        match next.get(key) {
            Some(next_bounds) if prev_bounds.approx_eq(*next_bounds) => {}
            Some(next_bounds) => {
                dirty_rects.push(*prev_bounds);
                dirty_rects.push(*next_bounds);
            }
            None => dirty_rects.push(*prev_bounds),
        }
    }

    for (key, next_bounds) in next {
        if !prev.contains_key(key) {
            dirty_rects.push(*next_bounds);
        }
    }

    dirty_rects
}

pub(super) struct PageRenderCache {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) scale_factor: f64,
    pub(super) snapshot: PageRenderSnapshot,
    pub(super) snapshot_initialized: bool,
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
            base_pixmap,
        }
    }
}
