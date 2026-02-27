use tiny_skia::Pixmap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CacheRect {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

impl CacheRect {
    pub(super) fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Option<Self> {
        if width <= 0.0 || height <= 0.0 {
            return None;
        }

        Some(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub(super) fn from_canvas(width: f32, height: f32) -> Option<Self> {
        Self::from_xywh(0.0, 0.0, width, height)
    }

    pub(super) fn right(self) -> f32 {
        self.x + self.width
    }

    pub(super) fn bottom(self) -> f32 {
        self.y + self.height
    }

    pub(super) fn area(self) -> f32 {
        self.width * self.height
    }

    pub(super) fn intersects(self, other: Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub(super) fn touches_or_intersects(self, other: Self, epsilon: f32) -> bool {
        self.x <= other.right() + epsilon
            && self.right() + epsilon >= other.x
            && self.y <= other.bottom() + epsilon
            && self.bottom() + epsilon >= other.y
    }

    pub(super) fn union(self, other: Self) -> Self {
        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }

    pub(super) fn approx_eq(self, other: Self) -> bool {
        const EPSILON: f32 = 0.1;
        (self.x - other.x).abs() <= EPSILON
            && (self.y - other.y).abs() <= EPSILON
            && (self.width - other.width).abs() <= EPSILON
            && (self.height - other.height).abs() <= EPSILON
    }

    pub(super) fn clamp(self, width: f32, height: f32) -> Option<Self> {
        let left = self.x.max(0.0);
        let top = self.y.max(0.0);
        let right = self.right().min(width);
        let bottom = self.bottom().min(height);
        Self::from_xywh(left, top, right - left, bottom - top)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PixelRect {
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl PixelRect {
    pub(super) fn from_layout_rect(
        rect: CacheRect,
        scale: f32,
        max_width: u32,
        max_height: u32,
    ) -> Option<Self> {
        let x0 = (rect.x * scale).floor().max(0.0).min(max_width as f32);
        let y0 = (rect.y * scale).floor().max(0.0).min(max_height as f32);
        let x1 = (rect.right() * scale).ceil().max(0.0).min(max_width as f32);
        let y1 = (rect.bottom() * scale)
            .ceil()
            .max(0.0)
            .min(max_height as f32);

        if x1 <= x0 || y1 <= y0 {
            return None;
        }

        Some(Self {
            x: x0 as u32,
            y: y0 as u32,
            width: (x1 - x0) as u32,
            height: (y1 - y0) as u32,
        })
    }

    pub(super) fn right(self) -> u32 {
        self.x + self.width
    }

    pub(super) fn bottom(self) -> u32 {
        self.y + self.height
    }

    pub(super) fn to_layout_rect(self, scale: f32) -> CacheRect {
        CacheRect {
            x: self.x as f32 / scale,
            y: self.y as f32 / scale,
            width: self.width as f32 / scale,
            height: self.height as f32 / scale,
        }
    }
}

pub(super) fn merge_and_clamp_rects(
    rects: Vec<CacheRect>,
    canvas_width: f32,
    canvas_height: f32,
    epsilon: f32,
) -> Vec<CacheRect> {
    let mut merged = Vec::new();

    for rect in rects {
        let Some(mut current) = rect.clamp(canvas_width, canvas_height) else {
            continue;
        };

        let mut idx = 0;
        while idx < merged.len() {
            if current.touches_or_intersects(merged[idx], epsilon) {
                current = current.union(merged.swap_remove(idx));
            } else {
                idx += 1;
            }
        }

        merged.push(current);
    }

    merged
}

pub(super) fn collect_non_overlapping_pixel_rects(
    rects: &[CacheRect],
    scale: f32,
    max_width: u32,
    max_height: u32,
) -> Vec<PixelRect> {
    let mut non_overlapping = Vec::new();

    for rect in rects {
        let Some(pixel_rect) = PixelRect::from_layout_rect(*rect, scale, max_width, max_height)
        else {
            continue;
        };

        append_pixel_rect_without_overlap(&mut non_overlapping, pixel_rect);
    }

    non_overlapping
}

fn append_pixel_rect_without_overlap(out: &mut Vec<PixelRect>, rect: PixelRect) {
    let mut pending = vec![rect];

    for existing in out.iter().copied() {
        let mut next = Vec::new();
        for candidate in pending {
            next.extend(subtract_pixel_rect(candidate, existing));
        }
        if next.is_empty() {
            return;
        }
        pending = next;
    }

    out.extend(pending);
}

fn subtract_pixel_rect(rect: PixelRect, overlap: PixelRect) -> Vec<PixelRect> {
    let ix0 = rect.x.max(overlap.x);
    let iy0 = rect.y.max(overlap.y);
    let ix1 = rect.right().min(overlap.right());
    let iy1 = rect.bottom().min(overlap.bottom());

    if ix0 >= ix1 || iy0 >= iy1 {
        return vec![rect];
    }

    let mut parts = Vec::with_capacity(4);

    if rect.y < iy0 {
        parts.push(PixelRect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: iy0 - rect.y,
        });
    }

    if iy1 < rect.bottom() {
        parts.push(PixelRect {
            x: rect.x,
            y: iy1,
            width: rect.width,
            height: rect.bottom() - iy1,
        });
    }

    if rect.x < ix0 {
        parts.push(PixelRect {
            x: rect.x,
            y: iy0,
            width: ix0 - rect.x,
            height: iy1 - iy0,
        });
    }

    if ix1 < rect.right() {
        parts.push(PixelRect {
            x: ix1,
            y: iy0,
            width: rect.right() - ix1,
            height: iy1 - iy0,
        });
    }

    parts
}

pub(super) fn clear_layout_rect(pixmap: &mut Pixmap, rect: CacheRect, scale: f32) {
    let Some(pixel_rect) =
        PixelRect::from_layout_rect(rect, scale, pixmap.width(), pixmap.height())
    else {
        return;
    };

    let row_bytes = pixmap.width() as usize * 4;
    let data = pixmap.data_mut();
    for y in pixel_rect.y..pixel_rect.bottom() {
        let start = y as usize * row_bytes + pixel_rect.x as usize * 4;
        let end = y as usize * row_bytes + pixel_rect.right() as usize * 4;
        data[start..end].fill(0);
    }
}
