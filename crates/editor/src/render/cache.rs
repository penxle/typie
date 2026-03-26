use crate::layout::elements::LineElement;
use crate::layout::{Element, Page, PositionedNode};
use crate::model::NodeId;
use crate::render::backend::cpu::pixel_buf::PixelBuf;
use crate::render::geometry::LayoutRect;
use crate::types::Point;
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};
use std::mem::Discriminant;
use std::rc::Rc;

const SCALE_FACTOR_MATCH_EPSILON: f64 = 1e-6;

#[derive(Default, Clone, PartialEq)]
pub(crate) struct PageSnapshot {
    nodes: FxHashMap<SnapshotNodeKey, LayoutRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SnapshotNodeKey {
    Ptr(usize),
    RenderLine {
        block_id: NodeId,
        line_idx: usize,
        signature: u64,
    },
    RenderStableElement {
        kind: Discriminant<Element>,
        node_id: NodeId,
        signature: u64,
    },
}

impl PageSnapshot {
    pub(crate) fn from_page(page: &Page) -> Self {
        let mut nodes = FxHashMap::default();
        collect_snapshot(&page.root, Point::zero(), &mut nodes);
        Self { nodes }
    }

    pub(crate) fn dirty_rects(&self, next: &Self) -> Vec<LayoutRect> {
        dirty_rects_between(&self.nodes, &next.nodes)
    }

    /// snapshot 비교 기반 dirty rect 계산 (CPU/GPU 공용).
    ///
    /// `snapshot_initialized`가 false이면 전체 캔버스를 dirty로 처리한다.
    /// 반환: (normalized_dirty_rects, should_full_repaint)
    pub(crate) fn compute_dirty_rects(
        prev: Option<(&Self, bool)>,
        next: &Self,
        canvas_width: f32,
        canvas_height: f32,
    ) -> (Vec<LayoutRect>, bool) {
        use crate::render::renderer::{normalize_dirty_rects, should_promote_full_repaint};

        let dirty_rects = match prev {
            Some((prev_snapshot, true)) => prev_snapshot.dirty_rects(next),
            _ => LayoutRect::from_canvas(canvas_width, canvas_height)
                .map(|r| vec![r])
                .unwrap_or_default(),
        };

        if dirty_rects.is_empty() {
            return (Vec::new(), false);
        }

        let normalized = normalize_dirty_rects(dirty_rects, canvas_width, canvas_height);
        let full_repaint = should_promote_full_repaint(&normalized, canvas_width, canvas_height);
        (normalized, full_repaint)
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
                signature: render_line_signature(positioned, line),
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

fn render_line_signature(positioned: &PositionedNode, line: &LineElement) -> u64 {
    let mut hasher = FxHasher::default();
    line.hash_render_cache_signature(&mut hasher);
    positioned.node.scope_id.hash(&mut hasher);
    positioned
        .node
        .render_hints
        .default_text_color
        .as_ref()
        .map(|value| hash_str(value.as_str()))
        .hash(&mut hasher);
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
    out: &mut FxHashMap<SnapshotNodeKey, LayoutRect>,
) {
    let pos = Point::new(
        offset.x + positioned.position.x,
        offset.y + positioned.position.y,
    );

    if let Some(key) = SnapshotNodeKey::for_positioned(positioned)
        && let Some(bounds) = node_paint_bounds(positioned, pos)
    {
        out.insert(key, bounds);
    }

    if let Some(children) = &positioned.node.children {
        for child in children {
            collect_snapshot(child, pos, out);
        }
    }
}

pub(super) fn node_paint_bounds(positioned: &PositionedNode, pos: Point) -> Option<LayoutRect> {
    let mut x = pos.x;
    let mut y = pos.y;
    let mut width = positioned.node.size.width;
    let mut height = positioned.node.size.height;

    if let Some(element) = positioned.node.element.as_ref() {
        if let Element::TableBorder(table) = element {
            // NOTE: 테이블은 LayoutNode 폭보다 더 넓게 그려질 수 있음
            x += table.x_offset;
            width = table.size.width;
            height = table.size.height;
        }

        let overflow = element.paint_overflow();
        x -= overflow.left;
        y -= overflow.top;
        width += overflow.left + overflow.right;
        height += overflow.top + overflow.bottom;
    }

    LayoutRect::from_xywh(x, y, width, height)
}

fn dirty_rects_between<K: Eq + Hash>(
    prev: &FxHashMap<K, LayoutRect>,
    next: &FxHashMap<K, LayoutRect>,
) -> Vec<LayoutRect> {
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

pub(super) struct PageCache {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) scale_factor: f64,
    pub(super) snapshot: PageSnapshot,
    pub(super) snapshot_initialized: bool,
    pub(super) background: PixelBuf,
    pub(super) content: PixelBuf,
}

impl PageCache {
    pub(super) fn new(width: u32, height: u32, scale_factor: f64) -> Self {
        let background = PixelBuf::new(width.max(1), height.max(1)).unwrap();
        let content = PixelBuf::new(width.max(1), height.max(1)).unwrap();
        Self {
            width,
            height,
            scale_factor,
            snapshot: PageSnapshot::default(),
            snapshot_initialized: false,
            background,
            content,
        }
    }

    pub(super) fn matches(&self, width: u32, height: u32, scale_factor: f64) -> bool {
        self.width == width
            && self.height == height
            && same_scale_factor(self.scale_factor, scale_factor)
    }

    pub(super) fn matches_for_height_resize(&self, width: u32, scale_factor: f64) -> bool {
        self.width == width && same_scale_factor(self.scale_factor, scale_factor)
    }

    pub(super) fn resize_preserving_overlap(
        self,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Self {
        let mut resized = Self::new(width, height, scale_factor);
        copy_pixelbuf_overlap(&self.background, &mut resized.background);
        copy_pixelbuf_overlap(&self.content, &mut resized.content);
        resized.snapshot = self.snapshot;
        resized.snapshot_initialized = self.snapshot_initialized;
        resized
    }

    pub(super) fn exposed_rects_on_resize(
        &self,
        new_width_px: u32,
        new_height_px: u32,
        scale: f32,
    ) -> Vec<LayoutRect> {
        exposed_rects_for_resize(self.width, self.height, new_width_px, new_height_px, scale)
    }
}

pub(super) fn same_scale_factor(a: f64, b: f64) -> bool {
    (a - b).abs() <= SCALE_FACTOR_MATCH_EPSILON
}

fn copy_pixelbuf_overlap(src: &PixelBuf, dst: &mut PixelBuf) {
    let copy_w = src.width().min(dst.width()) as usize;
    let copy_h = src.height().min(dst.height()) as usize;
    if copy_w == 0 || copy_h == 0 {
        return;
    }

    let src_stride = src.width() as usize * 4;
    let dst_stride = dst.width() as usize * 4;
    let row_bytes = copy_w * 4;

    let src_data = src.data();
    let dst_data = dst.data_mut();
    for row in 0..copy_h {
        let src_off = row * src_stride;
        let dst_off = row * dst_stride;
        dst_data[dst_off..dst_off + row_bytes]
            .copy_from_slice(&src_data[src_off..src_off + row_bytes]);
    }
}

fn exposed_rects_for_resize(
    old_width_px: u32,
    old_height_px: u32,
    new_width_px: u32,
    new_height_px: u32,
    scale: f32,
) -> Vec<LayoutRect> {
    if scale <= 0.0 {
        return Vec::new();
    }

    let mut rects = Vec::new();

    if new_width_px > old_width_px {
        let x = old_width_px as f32 / scale;
        let width = (new_width_px - old_width_px) as f32 / scale;
        let height = new_height_px as f32 / scale;
        if let Some(rect) = LayoutRect::from_xywh(x, 0.0, width, height) {
            rects.push(rect);
        }
    }

    if new_height_px > old_height_px {
        let y = old_height_px as f32 / scale;
        let width = new_width_px as f32 / scale;
        let height = (new_height_px - old_height_px) as f32 / scale;
        if let Some(rect) = LayoutRect::from_xywh(0.0, y, width, height) {
            rects.push(rect);
        }
    }

    rects
}
