use crate::global::font_version;
use crate::layout::cursor::{CursorNavigable, CursorNavigation, NavigationContext};
use crate::model::{NodeId, PreeditDecor, SelectionDecor};
use crate::state::{Position, Selection};
use crate::types::{Affinity, PaintOverflow, Point, Rect, Size};
use crate::utils::{
    build_char_to_byte_offsets, byte_to_char_offset_with_map, char_to_byte_offset,
    compute_grapheme_boundaries, compute_sentence_boundaries, compute_word_boundaries,
    resolve_explicit_break_line_end,
};
use rustc_hash::FxHashSet;
use skrifa::FontRef;
use skrifa::instance::{LocationRef, Size as SkriFaSize};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundSegment {
    pub start_offset: usize,
    pub end_offset: usize,
    pub color_key: String,
}

impl BackgroundSegment {
    pub fn split(&self, line_start: usize, line_end: usize) -> Option<Self> {
        let overlap_start = line_start.max(self.start_offset);
        let overlap_end = line_end.min(self.end_offset);

        if overlap_start >= overlap_end {
            return None;
        }

        Some(Self {
            start_offset: overlap_start,
            end_offset: overlap_end,
            color_key: self.color_key.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RubySegment {
    pub start_offset: usize,
    pub end_offset: usize,
    pub ruby_text: String,
}

impl RubySegment {
    pub fn split(&self, line_start: usize, line_end: usize) -> Option<Self> {
        let overlap_start = line_start.max(self.start_offset);
        let overlap_end = line_end.min(self.end_offset);

        if overlap_start >= overlap_end {
            return None;
        }

        let seg_len = self.end_offset - self.start_offset;
        if seg_len == 0 {
            return None;
        }

        let ruby_chars: Vec<char> = self.ruby_text.chars().collect();
        let ruby_len = ruby_chars.len();

        let start_ratio = (overlap_start - self.start_offset) as f32 / seg_len as f32;
        let end_ratio = (overlap_end - self.start_offset) as f32 / seg_len as f32;

        let ruby_start = (start_ratio * ruby_len as f32).round() as usize;
        let ruby_end = (end_ratio * ruby_len as f32).round() as usize;

        let ruby_start = ruby_start.min(ruby_len);
        let ruby_end = ruby_end.min(ruby_len);

        if ruby_start >= ruby_end {
            return None;
        }

        let line_ruby_text: String = ruby_chars[ruby_start..ruby_end].iter().collect();

        Some(Self {
            start_offset: overlap_start,
            end_offset: overlap_end,
            ruby_text: line_ruby_text,
        })
    }
}

#[derive(Clone)]
pub struct LineElement {
    pub block_id: NodeId,
    pub size: Size,
    pub line_idx: usize,
    pub layout: Rc<parley::Layout<String>>,
    pub metric: LineMetric,
    pub preedit: Option<PreeditDecor>,
    pub is_empty: bool,
    pub text: Rc<str>,
    pub ruby_segments: Vec<RubySegment>,
    pub background_segments: Vec<BackgroundSegment>,
    pub has_page_break: bool,
}

impl PartialEq for LineElement {
    fn eq(&self, other: &Self) -> bool {
        self.block_id == other.block_id
            && self.size == other.size
            && self.line_idx == other.line_idx
            && Rc::ptr_eq(&self.layout, &other.layout)
            && self.metric == other.metric
            && self.preedit == other.preedit
            && self.is_empty == other.is_empty
            && self.text == other.text
            && self.ruby_segments == other.ruby_segments
            && self.background_segments == other.background_segments
            && self.has_page_break == other.has_page_break
    }
}

impl fmt::Debug for LineElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineElement")
            .field("block_id", &self.block_id)
            .field("line_idx", &self.line_idx)
            .field("size", &self.size)
            .finish()
    }
}

impl LineElement {
    pub fn build(
        block_id: NodeId,
        size: Size,
        line_idx: usize,
        layout: Rc<parley::Layout<String>>,
        metric: LineMetric,
        preedit: Option<PreeditDecor>,
        is_empty: bool,
        text: Rc<str>,
        ruby_segments: Vec<RubySegment>,
        background_segments: Vec<BackgroundSegment>,
        has_page_break: bool,
    ) -> Self {
        Self {
            block_id,
            size,
            line_idx,
            layout,
            metric,
            preedit,
            is_empty,
            text,
            ruby_segments,
            background_segments,
            has_page_break,
        }
    }

    pub fn hash_render_cache_signature<H: Hasher>(&self, state: &mut H) {
        self.metric.start_offset.hash(state);
        self.metric.end_offset.hash(state);
        self.hash_text_slice_signature(state);
        self.hash_style_signature(state);
        self.has_page_break.hash(state);
        self.is_empty.hash(state);
        match &self.preedit {
            Some(preedit) => {
                1u8.hash(state);
                preedit.offset.hash(state);
                preedit.text.hash(state);
            }
            None => {
                0u8.hash(state);
            }
        }
        self.ruby_segments.len().hash(state);
        for segment in &self.ruby_segments {
            segment.start_offset.hash(state);
            segment.end_offset.hash(state);
            segment.ruby_text.hash(state);
        }
        self.background_segments.len().hash(state);
        for segment in &self.background_segments {
            segment.start_offset.hash(state);
            segment.end_offset.hash(state);
            segment.color_key.hash(state);
        }
    }

    fn hash_text_slice_signature<H: Hasher>(&self, state: &mut H) {
        let start_byte = char_to_byte_offset(&self.text, self.metric.start_offset);
        let end_byte = char_to_byte_offset(&self.text, self.metric.end_offset);
        if let Some(slice) = self.text.get(start_byte..end_byte) {
            slice.hash(state);
        } else {
            self.text.hash(state);
            self.metric.start_offset.hash(state);
            self.metric.end_offset.hash(state);
        }
    }

    fn hash_style_signature<H: Hasher>(&self, state: &mut H) {
        if let Some(layout_line) = self.layout.get(self.line_idx) {
            layout_line.items().count().hash(state);
            for item in layout_line.items() {
                if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                    let style = glyph_run.style();
                    style.brush.hash(state);
                    style.underline.is_some().hash(state);
                    style.strikethrough.is_some().hash(state);

                    glyph_run.offset().to_bits().hash(state);
                    glyph_run.advance().to_bits().hash(state);
                    glyph_run.baseline().to_bits().hash(state);

                    let run = glyph_run.run();
                    run.font_size().to_bits().hash(state);

                    let synthesis = run.synthesis();
                    synthesis.embolden().hash(state);
                    match synthesis.skew() {
                        Some(skew) => {
                            1u8.hash(state);
                            skew.to_bits().hash(state);
                        }
                        None => {
                            0u8.hash(state);
                        }
                    }

                    let font = run.font();
                    font.index.hash(state);
                    font.data.id().hash(state);
                    font_version(font.data.as_ref().as_ptr()).hash(state);

                    let coords = run.normalized_coords();
                    coords.len().hash(state);
                    for coord in coords {
                        coord.hash(state);
                    }
                }
            }
        }
    }

    const RUBY_TOP_OVERHANG: f32 = 16.0;

    pub fn paint_overflow(&self) -> PaintOverflow {
        let ruby_top = if self.ruby_segments.is_empty() {
            0.0
        } else {
            Self::RUBY_TOP_OVERHANG
        };

        // When metric.top is negative (negative leading), content extends above the element.
        let content_top_overflow = (-self.metric.top).max(0.0);
        // When content area bottom exceeds element height, content extends below.
        let element_height = self.metric.height + self.metric.leading;
        let content_bottom_overflow =
            (self.metric.top + self.metric.height - element_height).max(0.0);

        PaintOverflow {
            top: ruby_top + content_top_overflow + self.metric.ascent_overflow,
            bottom: content_bottom_overflow + self.metric.descent_overflow,
            ..PaintOverflow::default()
        }
    }

    fn is_explicit_break(&self) -> bool {
        self.metric.break_reason == parley::layout::BreakReason::Explicit
    }

    fn affinity_for_offset(&self, target_offset: usize, default: Affinity) -> Affinity {
        if !self.is_explicit_break() {
            return default;
        }

        if target_offset + 1 >= self.metric.end_offset {
            Affinity::Upstream
        } else {
            default
        }
    }

    fn is_soft_wrap_prev(&self) -> bool {
        if self.line_idx > 0 {
            let Some(prev_line) = self.layout.lines().nth(self.line_idx - 1) else {
                return false;
            };
            prev_line.break_reason() != parley::layout::BreakReason::Explicit
        } else {
            false
        }
    }

    fn is_soft_wrap_next(&self) -> bool {
        if self.line_idx < self.layout.len() - 1 {
            self.metric.break_reason != parley::layout::BreakReason::Explicit
        } else {
            false
        }
    }

    fn trailing_whitespace_start_offset(&self) -> usize {
        let start_byte = char_to_byte_offset(&self.text, self.metric.start_offset);
        let end_byte = char_to_byte_offset(&self.text, self.metric.end_offset);
        let line_slice = &self.text[start_byte..end_byte];
        self.metric.start_offset + line_slice.trim_end().chars().count()
    }

    // x가 cluster가 속한 grapheme의 visual mid보다 왼쪽이면 시작, 오른쪽이면 끝을 반환
    fn snap_to_containing_grapheme(
        &self,
        cluster: &ClusterMetric,
        x: f32,
    ) -> Option<(usize, Affinity)> {
        let grapheme_idx = self
            .metric
            .grapheme_offsets
            .partition_point(|&g| g <= cluster.start_offset)
            .saturating_sub(1);

        let grapheme_start = self
            .metric
            .grapheme_offsets
            .get(grapheme_idx)
            .copied()
            .unwrap_or(self.metric.start_offset);

        let grapheme_end = self
            .metric
            .grapheme_offsets
            .get(grapheme_idx + 1)
            .copied()
            .unwrap_or(self.metric.end_offset);

        let grapheme_clusters: Vec<&ClusterMetric> = self
            .metric
            .clusters
            .iter()
            .filter(|c| c.start_offset >= grapheme_start && c.start_offset < grapheme_end)
            .collect();

        if grapheme_clusters.is_empty() {
            return None;
        }

        let first_cluster = grapheme_clusters.first().unwrap();
        let last_cluster = grapheme_clusters.last().unwrap();
        let grapheme_visual_start = first_cluster.x;
        let grapheme_visual_end = last_cluster.x + last_cluster.width;
        let grapheme_visual_mid = (grapheme_visual_start + grapheme_visual_end) / 2.0;

        let target_offset = if x < grapheme_visual_mid {
            grapheme_start
        } else {
            grapheme_end
        };

        let affinity = self.affinity_for_offset(
            target_offset,
            if target_offset == self.metric.end_offset {
                Affinity::Upstream
            } else {
                Affinity::Downstream
            },
        );

        Some((target_offset, affinity))
    }

    fn find_offset_at_x(&self, x: f32) -> (usize, Affinity) {
        if self.metric.clusters.is_empty() {
            return (self.metric.start_offset, Affinity::Downstream);
        }

        // x를 포함하는 visual cluster
        for cluster in &self.metric.clusters {
            let cluster_start_x = cluster.x;
            let cluster_end_x = cluster.x + cluster.width;

            if x >= cluster_start_x && x < cluster_end_x {
                let cluster_graphemes: Vec<usize> = self
                    .metric
                    .grapheme_offsets
                    .iter()
                    .filter(|&&g| g >= cluster.start_offset && g <= cluster.end_offset)
                    .copied()
                    .collect();

                // 이 cluster를 포함하는 grapheme 찾기
                if cluster_graphemes.is_empty() || cluster_graphemes.len() <= 1 {
                    if let Some(result) = self.snap_to_containing_grapheme(cluster, x) {
                        return result;
                    }
                }

                // cluster와 grapheme이 일치
                if cluster_graphemes.len() == 2 {
                    let cluster_mid_x = cluster_start_x + cluster.width / 2.0;
                    let target_offset = if x < cluster_mid_x {
                        cluster_graphemes[0]
                    } else {
                        cluster_graphemes[1]
                    };

                    let affinity = self.affinity_for_offset(
                        target_offset,
                        if target_offset == self.metric.end_offset {
                            Affinity::Upstream
                        } else {
                            Affinity::Downstream
                        },
                    );
                    return (target_offset, affinity);
                }

                // cluster에 여러 grapheme이 있음
                let num_graphemes = cluster_graphemes.len() - 1;
                let grapheme_width = cluster.width / num_graphemes as f32;
                let relative_x = x - cluster_start_x;
                let grapheme_idx =
                    ((relative_x / grapheme_width).floor() as usize).min(num_graphemes - 1);

                let grapheme_left_x = grapheme_idx as f32 * grapheme_width;
                let grapheme_mid_x = grapheme_left_x + grapheme_width / 2.0;

                let target_offset = if relative_x < grapheme_mid_x {
                    cluster_graphemes[grapheme_idx]
                } else {
                    cluster_graphemes[grapheme_idx + 1]
                };

                let affinity = self.affinity_for_offset(
                    target_offset,
                    if target_offset == self.metric.end_offset {
                        Affinity::Upstream
                    } else {
                        Affinity::Downstream
                    },
                );

                return (target_offset, affinity);
            }
        }

        // x가 어떤 cluster 내부에도 없으면 가장 가까운 cluster 경계 찾기
        let mut best_offset = self.metric.start_offset;
        let mut best_distance = (x - self.metric.clusters[0].x).abs();

        for cluster in &self.metric.clusters {
            let start_distance = (x - cluster.x).abs();
            if start_distance < best_distance {
                best_distance = start_distance;
                best_offset = cluster.start_offset;
            }

            let end_x = cluster.x + cluster.width;
            let end_distance = (x - end_x).abs();
            if end_distance < best_distance {
                best_distance = end_distance;
                best_offset = cluster.end_offset;
            }
        }

        // 가장 가까운 grapheme 경계로 스냅
        let snapped_offset = self.snap_to_grapheme_boundary(best_offset);

        let affinity = self.affinity_for_offset(
            snapped_offset,
            if snapped_offset == self.metric.end_offset {
                Affinity::Upstream
            } else {
                Affinity::Downstream
            },
        );

        (snapped_offset, affinity)
    }

    fn position_to_offset(&self, position: &Position) -> Option<usize> {
        if position.node_id != self.block_id {
            return None;
        }

        let offset = position.offset;
        let start_offset = self.metric.start_offset;
        let end_offset = self.metric.end_offset;

        if offset < start_offset || offset > end_offset {
            return None;
        }

        if offset == start_offset && offset == end_offset {
            return Some(offset);
        }

        if offset == start_offset {
            if position.affinity == Affinity::Downstream || self.line_idx == 0 {
                return Some(offset);
            }
            return None;
        }

        if offset == end_offset {
            if position.affinity == Affinity::Upstream || self.layout.len() - 1 == self.line_idx {
                return Some(offset);
            }
            return None;
        }

        Some(offset)
    }

    fn word_boundary_left(&self, offset: usize) -> Option<usize> {
        if offset == 0 {
            return None;
        }

        let boundaries = compute_word_boundaries(&self.text);
        if boundaries.len() < 2 {
            return None;
        }

        let idx = boundaries.partition_point(|&b| b < offset);
        if idx == 0 {
            return None;
        }

        (0..idx)
            .rev()
            .find(|&i| {
                let start = boundaries[i];
                let end = *boundaries.get(i + 1).unwrap_or(&self.metric.end_offset);
                !self.is_whitespace_segment(start, end)
            })
            .map(|i| boundaries[i])
            .or(Some(boundaries[0]))
    }

    fn word_boundary_right(&self, offset: usize) -> Option<usize> {
        let boundaries = compute_word_boundaries(&self.text);
        if boundaries.len() < 2 {
            return None;
        }

        let line_end = self.adjusted_end_offset();
        let idx = boundaries.partition_point(|&b| b <= offset).max(1);

        (idx..boundaries.len())
            .find(|&i| {
                let start = boundaries[i - 1];
                let end = boundaries[i];
                start < line_end && !self.is_whitespace_segment(start, end)
            })
            .map(|i| boundaries[i])
    }

    fn is_whitespace_segment(&self, start: usize, end: usize) -> bool {
        let byte_start = char_to_byte_offset(&self.text, start);
        let byte_end = char_to_byte_offset(&self.text, end);
        let slice = &self.text[byte_start..byte_end];

        slice.chars().all(|ch| ch.is_whitespace())
    }

    fn offset_to_position(&self, offset: usize, affinity: Affinity) -> Option<Position> {
        let start_offset = self.metric.start_offset;
        let end_offset = self.metric.end_offset;

        if offset < start_offset || offset > end_offset {
            return None;
        }

        let clamped = offset.clamp(start_offset, self.adjusted_end_offset());
        Some(Position::new(self.block_id, clamped, affinity))
    }

    fn adjusted_end_offset(&self) -> usize {
        if self.is_empty {
            self.metric.start_offset
        } else {
            self.metric.end_offset
        }
    }

    fn prev_grapheme_offset(&self, offset: usize) -> Option<usize> {
        let offs = &self.metric.grapheme_offsets;
        if offs.is_empty() {
            return None;
        }
        let idx = offs.partition_point(|&g| g < offset);
        if idx == 0 { None } else { Some(offs[idx - 1]) }
    }

    fn next_grapheme_offset(&self, offset: usize) -> Option<usize> {
        let offs = &self.metric.grapheme_offsets;
        if offs.is_empty() {
            return None;
        }
        let idx = offs.partition_point(|&g| g <= offset);
        offs.get(idx).copied()
    }

    fn snap_to_grapheme_boundary(&self, offset: usize) -> usize {
        let grapheme_offsets = &self.metric.grapheme_offsets;

        if grapheme_offsets.is_empty() {
            return offset;
        }

        let idx = grapheme_offsets.partition_point(|&boundary| boundary < offset);

        if idx == 0 {
            grapheme_offsets[0]
        } else if idx >= grapheme_offsets.len() {
            grapheme_offsets[grapheme_offsets.len() - 1]
        } else {
            let left_boundary = grapheme_offsets[idx - 1];
            let right_boundary = grapheme_offsets[idx];

            if offset - left_boundary <= right_boundary - offset {
                left_boundary
            } else {
                right_boundary
            }
        }
    }

    fn cursor_bounds_internal(&self, position: &Position) -> Option<Rect> {
        self.bounds_internal(position, self.metric.top, self.metric.height)
    }

    fn selection_handle_bounds_internal(&self, position: &Position) -> Option<Rect> {
        self.bounds_internal(position, 0.0, self.metric.height + self.metric.leading)
    }

    fn bounds_internal(&self, position: &Position, top: f32, height: f32) -> Option<Rect> {
        let position = if let Some(preedit) = &self.preedit
            && preedit.node_id == self.block_id
        {
            Position::new(
                self.block_id,
                preedit.offset + bytecount::num_chars(preedit.text.as_bytes()),
                Affinity::Upstream,
            )
        } else {
            *position
        };

        let offset = self.position_to_offset(&position)?;

        if self.is_soft_wrap_next() {
            let trailing_start = self.trailing_whitespace_start_offset();
            if trailing_start > self.metric.start_offset
                && trailing_start < self.metric.end_offset
                && offset > trailing_start
            {
                return Some(Rect::new(
                    self.offset_to_x(trailing_start),
                    top,
                    0.0,
                    height,
                ));
            }
        }

        if self.metric.clusters.is_empty() {
            return Some(Rect::new(self.metric.left, top, 0.0, height));
        }

        let last = self.metric.clusters.last().unwrap();
        if offset == last.end_offset {
            return if position.affinity == Affinity::Upstream
                || self.layout.len() - 1 == self.line_idx
            {
                Some(Rect::new(
                    self.metric.left + last.x + last.width,
                    top,
                    0.0,
                    height,
                ))
            } else {
                None
            };
        }

        for cluster in &self.metric.clusters {
            if offset >= cluster.start_offset && offset < cluster.end_offset {
                return Some(Rect::new(self.metric.left + cluster.x, top, 0.0, height));
            }
        }

        None
    }

    pub fn offset_to_x(&self, offset: usize) -> f32 {
        if self.metric.clusters.is_empty() {
            return self.metric.left;
        }

        if let Some(first) = self.metric.clusters.first() {
            if offset <= first.start_offset {
                return self.metric.left + first.x;
            }
        }

        if let Some(last) = self.metric.clusters.last() {
            if offset >= last.end_offset {
                return self.metric.left + last.x + last.width;
            }
        }

        for cluster in &self.metric.clusters {
            if offset < cluster.end_offset {
                return self.metric.left + cluster.x;
            }
        }

        self.metric.left
    }

    pub fn compute_selection_rects(
        &self,
        point: Point,
        selections: &[SelectionDecor],
    ) -> Vec<Rect> {
        const MIN_WIDTH: f32 = 4.0;
        let mut rects = Vec::new();

        let Some(selection) = self.selection_for_node(selections) else {
            return rects;
        };

        if let Some(rect) = self.selection_highlight(selection, MIN_WIDTH, point) {
            rects.push(rect);
        }

        if let Some(rect) = self.explicit_break_marker(selection, MIN_WIDTH, point) {
            rects.push(rect);
        }

        if self.has_page_break {
            if let Some(rect) = self.page_break_indicator(point, selections) {
                rects.push(rect);
            }
        }

        rects
    }

    fn selection_for_node<'a>(
        &self,
        selections: &'a [SelectionDecor],
    ) -> Option<&'a SelectionDecor> {
        selections.iter().find(|s| s.node_id() == self.block_id)
    }

    fn selection_highlight(
        &self,
        selection: &SelectionDecor,
        min_width: f32,
        point: Point,
    ) -> Option<Rect> {
        let (local_start, local_end) = self.intersect_selection_segment(selection)?;
        let line_is_blank = self.metric.clusters.is_empty();

        if self.is_empty {
            if self.has_page_break {
                return None;
            }
            return Some(self.empty_paragraph_rect(point, min_width));
        }

        if line_is_blank {
            return None;
        }

        let clamped_end = if self.is_soft_wrap_next() {
            let trailing_start = self.trailing_whitespace_start_offset();
            if trailing_start > self.metric.start_offset && trailing_start < self.metric.end_offset
            {
                local_end.min(trailing_start)
            } else {
                local_end
            }
        } else {
            local_end
        };

        let start_x = self.offset_to_x(local_start);
        let end_x = self.offset_to_x(clamped_end);
        let width = end_x - start_x;

        if width <= 0.0 {
            return None;
        }

        Some(Rect::new(
            point.x + start_x,
            point.y,
            width,
            self.metric.height + self.metric.leading,
        ))
    }

    fn empty_paragraph_rect(&self, point: Point, min_width: f32) -> Rect {
        Rect::new(
            point.x + self.metric.left,
            point.y,
            min_width,
            self.metric.height + self.metric.leading,
        )
    }

    fn explicit_break_marker(
        &self,
        selection: &SelectionDecor,
        marker_width: f32,
        point: Point,
    ) -> Option<Rect> {
        let (local_start, _) = self.intersect_selection_segment(selection)?;
        let selection_covers_explicit_break = self.metric.break_reason
            == parley::layout::BreakReason::Explicit
            && self.line_idx + 1 < self.layout.len()
            && local_start <= self.metric.end_offset
            && selection.end_offset() >= self.metric.end_offset;

        if !selection_covers_explicit_break {
            return None;
        }

        let break_x = self.offset_to_x(self.metric.end_offset);
        Some(Rect::new(
            point.x + break_x,
            point.y,
            marker_width,
            self.metric.height + self.metric.leading,
        ))
    }

    fn intersect_selection_segment(&self, selection: &SelectionDecor) -> Option<(usize, usize)> {
        if selection.start_offset() >= self.metric.end_offset
            || selection.end_offset() <= self.metric.start_offset
        {
            return None;
        }

        Some((
            selection.start_offset().max(self.metric.start_offset),
            selection.end_offset().min(self.metric.end_offset),
        ))
    }

    pub fn page_break_indicator(
        &self,
        point: Point,
        selections: &[SelectionDecor],
    ) -> Option<Rect> {
        let Some(selection) = self.selection_for_node(selections) else {
            return None;
        };

        if !self.is_empty && selection.end_offset() <= self.metric.end_offset {
            return None;
        }

        let end_x = self.offset_to_x(self.metric.end_offset);
        Some(Rect::new(
            point.x + end_x,
            point.y,
            self.size.width - end_x + 20.0,
            self.metric.height + self.metric.leading,
        ))
    }
}

impl CursorNavigable for LineElement {
    fn cursor_bounds(&self, _ctx: &NavigationContext, position: &Position) -> Option<Rect> {
        self.cursor_bounds_internal(position)
    }

    fn selection_handle_bounds(
        &self,
        _ctx: &NavigationContext,
        position: &Position,
    ) -> Option<Rect> {
        self.selection_handle_bounds_internal(position)
    }

    fn navigate_left(
        &self,
        _ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation> {
        let offset = self.position_to_offset(&position)?;

        if offset == self.metric.start_offset || self.is_empty {
            if self.is_soft_wrap_prev() {
                return Some(CursorNavigation::SoftWrap {
                    offset: self.metric.start_offset,
                });
            }

            let rect = self.cursor_bounds_internal(&position)?;
            return Some(CursorNavigation::Exit {
                preferred_x: rect.x,
                preferred_y,
            });
        }

        let prev = self
            .prev_grapheme_offset(offset)
            .unwrap_or(self.metric.start_offset);

        if self.is_soft_wrap_next() && offset == self.metric.end_offset {
            let trailing_start = self.trailing_whitespace_start_offset();
            if trailing_start > self.metric.start_offset && trailing_start < self.metric.end_offset
            {
                return Some(CursorNavigation::Moved {
                    selection: Selection::collapsed(
                        self.offset_to_position(trailing_start, Affinity::Upstream)?,
                    ),
                });
            }
        }

        let mut affinity = self.affinity_for_offset(prev, Affinity::Downstream);
        if self.is_explicit_break() && prev + 1 >= self.metric.end_offset {
            affinity = Affinity::Upstream;
        }

        Some(CursorNavigation::Moved {
            selection: Selection::collapsed(self.offset_to_position(prev, affinity)?),
        })
    }

    fn navigate_right(
        &self,
        _ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation> {
        let offset = self.position_to_offset(&position)?;

        if offset == self.metric.end_offset || self.is_empty {
            if self.is_soft_wrap_next() {
                return Some(CursorNavigation::SoftWrap {
                    offset: self.metric.end_offset,
                });
            }

            let rect = self.cursor_bounds_internal(&position)?;
            return Some(CursorNavigation::Exit {
                preferred_x: rect.x,
                preferred_y,
            });
        }

        let next = self
            .next_grapheme_offset(offset)
            .unwrap_or(self.metric.end_offset);

        if self.is_soft_wrap_next() {
            let trailing_start = self.trailing_whitespace_start_offset();
            if trailing_start > self.metric.start_offset
                && trailing_start < self.metric.end_offset
                && next > trailing_start
            {
                return Some(CursorNavigation::Moved {
                    selection: Selection::collapsed(
                        self.offset_to_position(self.metric.end_offset, Affinity::Downstream)?,
                    ),
                });
            }
        }

        let mut affinity = self.affinity_for_offset(next, Affinity::Upstream);
        if self.is_explicit_break() && next >= self.metric.end_offset {
            affinity = Affinity::Downstream;
        }

        Some(CursorNavigation::Moved {
            selection: Selection::collapsed(self.offset_to_position(next, affinity)?),
        })
    }

    fn navigate_word_left(
        &self,
        _ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation> {
        let offset = self.position_to_offset(&position)?;

        let exit = || {
            if self.is_soft_wrap_prev() {
                return Some(CursorNavigation::SoftWrap {
                    offset: self.metric.start_offset,
                });
            }

            let rect = self.cursor_bounds_internal(&position)?;
            Some(CursorNavigation::Exit {
                preferred_x: rect.x,
                preferred_y,
            })
        };

        let Some(boundary) = self.word_boundary_left(offset) else {
            return exit();
        };

        if boundary == offset {
            return exit();
        }

        if boundary < self.metric.start_offset {
            return exit();
        }

        let affinity = self.affinity_for_offset(boundary, Affinity::Downstream);
        Some(CursorNavigation::Moved {
            selection: Selection::collapsed(Position::new(self.block_id, boundary, affinity)),
        })
    }

    fn navigate_word_right(
        &self,
        _ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation> {
        let offset = self.position_to_offset(&position)?;

        let exit = || {
            if self.is_soft_wrap_next() {
                return Some(CursorNavigation::SoftWrap {
                    offset: self.metric.end_offset,
                });
            }

            let rect = self.cursor_bounds_internal(&position)?;
            Some(CursorNavigation::Exit {
                preferred_x: rect.x,
                preferred_y,
            })
        };

        let Some(boundary) = self.word_boundary_right(offset) else {
            return exit();
        };

        if boundary == offset {
            return exit();
        };

        if boundary > self.metric.end_offset {
            return exit();
        }

        let affinity = self.affinity_for_offset(boundary, Affinity::Downstream);
        Some(CursorNavigation::Moved {
            selection: Selection::collapsed(Position::new(self.block_id, boundary, affinity)),
        })
    }

    fn navigate_up(
        &self,
        _ctx: &NavigationContext,
        _position: Position,
        preferred_x: f32,
    ) -> Option<CursorNavigation> {
        Some(CursorNavigation::Exit {
            preferred_x,
            preferred_y: 0.0,
        })
    }

    fn navigate_down(
        &self,
        _ctx: &NavigationContext,
        _position: Position,
        preferred_x: f32,
    ) -> Option<CursorNavigation> {
        Some(CursorNavigation::Exit {
            preferred_x,
            preferred_y: self.size.height,
        })
    }

    fn navigate_sentence_up(
        &self,
        _ctx: &NavigationContext,
        position: Position,
        _preferred_y: f32,
    ) -> Option<CursorNavigation> {
        let offset = self.position_to_offset(&position)?;

        let exit = || {
            let rect = self.cursor_bounds_internal(&position)?;
            Some(CursorNavigation::Exit {
                preferred_x: rect.x,
                preferred_y: 0.0,
            })
        };

        if self.is_empty {
            return exit();
        }

        let mut boundaries = vec![0];
        boundaries.extend(compute_sentence_boundaries(&self.text));

        let idx = boundaries.partition_point(|&b| b < offset);
        if idx == 0 {
            // 첫 문장이면 이전 문단으로
            return exit();
        }

        let mut target_idx = idx - 1;
        if boundaries[target_idx] == offset {
            if target_idx > 0 {
                // 이전 문장의 시작으로 이동
                target_idx -= 1;
            } else {
                // 첫 문장이면 이전 문단으로
                return exit();
            }
        }
        // else: 현재 문장의 시작으로 이동

        let boundary = boundaries[target_idx];
        let affinity = self.affinity_for_offset(boundary, Affinity::Downstream);

        Some(CursorNavigation::Moved {
            selection: Selection::collapsed(Position::new(self.block_id, boundary, affinity)),
        })
    }

    fn navigate_sentence_down(
        &self,
        _ctx: &NavigationContext,
        position: Position,
        _preferred_y: f32,
    ) -> Option<CursorNavigation> {
        let offset = self.position_to_offset(&position)?;

        let exit = || {
            let rect = self.cursor_bounds_internal(&position)?;
            Some(CursorNavigation::Exit {
                preferred_x: rect.x,
                preferred_y: self.size.height,
            })
        };

        if self.is_empty {
            return exit();
        }

        let boundaries = compute_sentence_boundaries(&self.text);
        if boundaries.is_empty() {
            return exit();
        }

        let idx = boundaries.partition_point(|&b| b <= offset);
        if idx >= boundaries.len() {
            // 마지막 문장이면 다음 문단으로
            return exit();
        }

        let mut boundary_idx = idx;
        let mut boundary = boundaries[boundary_idx];

        // 뒤쪽 공백 제외한 문장 끝 위치 계산
        let mut byte_offset = char_to_byte_offset(&self.text, boundary);
        if byte_offset > self.text.len() {
            return exit();
        }
        let mut trimmed_boundary = self.text[..byte_offset].trim_end().chars().count();

        if trimmed_boundary <= offset {
            if boundary_idx + 1 < boundaries.len() {
                // 다음 문장의 끝으로 이동 (trimmed)
                boundary_idx += 1;
                boundary = boundaries[boundary_idx];
                byte_offset = char_to_byte_offset(&self.text, boundary);
                if byte_offset > self.text.len() {
                    return exit();
                }
                trimmed_boundary = self.text[..byte_offset].trim_end().chars().count();
            } else {
                // 마지막 문장이면 다음 문단으로
                return exit();
            }
        }
        // else: 현재 문장의 끝으로 이동 (trimmed)

        let final_boundary = trimmed_boundary;
        let affinity = self.affinity_for_offset(final_boundary, Affinity::Upstream);

        Some(CursorNavigation::Moved {
            selection: Selection::collapsed(Position::new(self.block_id, final_boundary, affinity)),
        })
    }

    fn navigate_to_start(
        &self,
        _ctx: &NavigationContext,
        _position: Position,
    ) -> Option<CursorNavigation> {
        Some(CursorNavigation::Moved {
            selection: Selection::collapsed(
                self.offset_to_position(self.metric.start_offset, Affinity::Downstream)?,
            ),
        })
    }

    fn navigate_to_end(
        &self,
        _ctx: &NavigationContext,
        position: Position,
    ) -> Option<CursorNavigation> {
        let is_explicit = self.metric.break_reason == parley::layout::BreakReason::Explicit;

        if is_explicit {
            if let Some((offset, affinity)) = resolve_explicit_break_line_end(
                position.offset,
                self.metric.end_offset,
                position.affinity,
            ) {
                let affinity = if position.offset + 1 >= self.metric.end_offset {
                    position.affinity
                } else {
                    affinity
                };

                return Some(CursorNavigation::Moved {
                    selection: Selection::collapsed(self.offset_to_position(offset, affinity)?),
                });
            }
        }

        if self.is_soft_wrap_next() {
            let trailing_start = self.trailing_whitespace_start_offset();
            if trailing_start > self.metric.start_offset && trailing_start < self.metric.end_offset
            {
                return Some(CursorNavigation::Moved {
                    selection: Selection::collapsed(
                        self.offset_to_position(trailing_start, Affinity::Upstream)?,
                    ),
                });
            }
        }

        let target_affinity = if position.offset == self.metric.end_offset {
            position.affinity
        } else {
            Affinity::Upstream
        };

        Some(CursorNavigation::Moved {
            selection: Selection::collapsed(
                self.offset_to_position(self.metric.end_offset, target_affinity)?,
            ),
        })
    }

    fn find_selection_at_point(
        &self,
        _ctx: &NavigationContext,
        x: f32,
        _y: f32,
    ) -> Option<Selection> {
        let (offset, mut affinity) = self.find_offset_at_x(x - self.metric.left);

        if self.has_page_break && offset == self.adjusted_end_offset() {
            affinity = Affinity::Downstream;
        }

        let position = self.offset_to_position(offset, affinity)?;
        Some(Selection::collapsed(position))
    }

    fn find_drag_target(&self, ctx: &NavigationContext, x: f32, y: f32) -> Option<Selection> {
        if self.has_page_break {
            let last_cluster_end = self
                .metric
                .clusters
                .last()
                .map(|c| c.x + c.width)
                .unwrap_or(0.0);

            if x - self.metric.left > last_cluster_end || y > self.metric.height {
                let page_break_offset = if self.is_empty {
                    0
                } else {
                    self.metric.end_offset
                };
                let start = Position::new(self.block_id, page_break_offset, Affinity::Downstream);
                let end = Position::new(self.block_id, page_break_offset + 1, Affinity::Upstream);
                return Some(Selection::new(start, end));
            }
        }
        self.find_selection_at_point(ctx, x, y)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterMetric {
    pub start_offset: usize,
    pub end_offset: usize,
    pub x: f32,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LineMetric {
    pub top: f32,
    pub left: f32,
    pub height: f32,
    pub leading: f32,
    pub baseline: f32,
    pub ascent: f32,
    pub content_width: f32,
    pub start_offset: usize,
    pub end_offset: usize,
    pub clusters: Vec<ClusterMetric>,
    pub break_reason: parley::layout::BreakReason,
    pub grapheme_offsets: Vec<usize>,
    /// How far glyph outlines may extend beyond the typographic ascent/descent.
    /// Computed from the font's bounding box vs typographic metrics.
    pub ascent_overflow: f32,
    pub descent_overflow: f32,
}

pub fn build_metrics(
    layout: &parley::Layout<String>,
    text: &str,
    scale_factor: f64,
    strut_ascent: f32,
    strut_descent: f32,
    strut_font_size: f32,
    line_height_ratio: f32,
) -> Vec<LineMetric> {
    let mut lines = Vec::new();

    let char_to_byte = build_char_to_byte_offsets(text);
    let global_grapheme_offsets = compute_grapheme_boundaries(text);

    let mut top = 0.0;
    let safe_strut_font_size = if strut_font_size > 0.0 {
        strut_font_size
    } else {
        1.0
    };
    let ascent_ratio = (strut_ascent.max(0.0)) / safe_strut_font_size;
    let descent_ratio = (strut_descent.max(0.0)) / safe_strut_font_size;
    let fallback_ascent = strut_ascent.max(0.0);
    let fallback_descent = strut_descent.max(0.0);
    let safe_line_height_ratio = if line_height_ratio > 0.0 {
        line_height_ratio
    } else {
        1.0
    };

    for line in layout.lines() {
        let line_metrics = line.metrics();

        let mut clusters = Vec::new();
        let mut advance_x: f32 = 0.0;
        let mut inline_prefix = 0.0;
        let mut seen_glyph = false;
        let mut max_run_font_size = 0.0f32;
        let mut min_cluster_x: Option<f32> = None;
        let mut max_cluster_right: Option<f32> = None;
        let mut max_ascent_overflow = 0.0f32;
        let mut max_descent_overflow = 0.0f32;

        let mut indices = FxHashSet::default();

        for item in line.items() {
            match item {
                parley::PositionedLayoutItem::GlyphRun(glyph_run) => {
                    let run = glyph_run.run();
                    let run_offset = glyph_run.offset();
                    max_run_font_size = max_run_font_size.max(run.font_size());

                    if !indices.insert(run.index()) {
                        continue;
                    }

                    let font = run.font();
                    if let Ok(font_ref) = FontRef::from_index(font.data.as_ref(), font.index) {
                        let sm = skrifa::metrics::Metrics::new(
                            &font_ref,
                            SkriFaSize::new(run.font_size()),
                            LocationRef::default(),
                        );
                        if let Some(bounds) = sm.bounds {
                            max_ascent_overflow =
                                max_ascent_overflow.max((bounds.y_max - sm.ascent).max(0.0));
                            // sm.descent is negative; bounds.y_min is negative
                            max_descent_overflow =
                                max_descent_overflow.max((-bounds.y_min + sm.descent).max(0.0));
                        }
                    }

                    let mut run_advance = 0.0;
                    for cluster in run.clusters() {
                        let text_range = cluster.text_range();
                        let cluster_width = cluster.advance();
                        let cluster_x = run_offset + run_advance;

                        clusters.push(ClusterMetric {
                            start_offset: byte_to_char_offset_with_map(
                                &char_to_byte,
                                text_range.start,
                            ),
                            end_offset: byte_to_char_offset_with_map(&char_to_byte, text_range.end),
                            x: cluster_x,
                            width: cluster_width,
                        });

                        min_cluster_x = Some(
                            min_cluster_x
                                .map(|min_x| min_x.min(cluster_x))
                                .unwrap_or(cluster_x),
                        );
                        max_cluster_right = Some(
                            max_cluster_right
                                .map(|max_x| max_x.max(cluster_x + cluster_width))
                                .unwrap_or(cluster_x + cluster_width),
                        );

                        run_advance += cluster_width;
                        seen_glyph = true;
                    }

                    advance_x = advance_x.max(run_offset + run_advance);
                }
                parley::PositionedLayoutItem::InlineBox(inline_box) => {
                    advance_x += inline_box.width;
                    if !seen_glyph {
                        inline_prefix += inline_box.width;
                    }
                }
            }
        }

        let line_font_size = if max_run_font_size > 0.0 {
            max_run_font_size
        } else {
            safe_strut_font_size
        };
        let mut ascent = ascent_ratio * line_font_size;
        let mut descent = descent_ratio * line_font_size;
        if ascent <= 0.0 && descent <= 0.0 {
            ascent = fallback_ascent;
            descent = fallback_descent;
        }
        let mut height = (ascent + descent).max(0.0);
        if height <= 0.0 {
            height = (line_metrics.ascent + line_metrics.descent).max(0.0);
        }
        let mut line_box_height = (line_font_size * safe_line_height_ratio).max(height);
        if !line_box_height.is_finite() || line_box_height <= 0.0 {
            line_box_height = height;
        }
        let leading = (line_box_height - height).max(0.0);
        let line_top = leading * 0.5;
        let baseline = top + line_top + ascent;
        top += line_box_height;

        let text_range = line.text_range();
        let line_start_char = byte_to_char_offset_with_map(&char_to_byte, text_range.start);
        let line_end_char = byte_to_char_offset_with_map(&char_to_byte, text_range.end);

        let mut grapheme_offsets = Vec::new();

        grapheme_offsets.push(line_start_char);

        // NOTE: parley가 라인을 비순차적으로 반환할 수 있어서 이진 탐색으로 grapheme 경계를 찾는다
        let start_idx = global_grapheme_offsets.partition_point(|&g| g <= line_start_char);
        let mut idx = start_idx;
        while idx < global_grapheme_offsets.len() {
            let g = global_grapheme_offsets[idx];
            if g >= line_end_char {
                break;
            }
            if g > line_start_char {
                grapheme_offsets.push(g);
            }
            idx += 1;
        }

        if grapheme_offsets.last() != Some(&line_end_char) {
            grapheme_offsets.push(line_end_char);
        }

        let left = min_cluster_x.unwrap_or(line_metrics.offset + inline_prefix);
        let content_width = match (min_cluster_x, max_cluster_right) {
            (Some(min_x), Some(max_x)) => (max_x - min_x).max(0.0),
            _ => (advance_x - inline_prefix).max(0.0),
        };
        let cluster_origin = min_cluster_x.unwrap_or(0.0);
        for cluster in &mut clusters {
            cluster.x = snap_to_pixel(cluster.x - cluster_origin, scale_factor);
        }

        lines.push(LineMetric {
            top: snap_to_pixel(line_top, scale_factor),
            left: snap_to_pixel(left, scale_factor),
            height,
            leading,
            baseline: snap_to_pixel(baseline, scale_factor),
            ascent: snap_to_pixel(ascent, scale_factor),
            // NOTE: width를 스냅하면 실제 glyph 폭보다 작아져 overflow가 생길 수 있어 원본값 유지
            content_width,
            start_offset: line_start_char,
            end_offset: line_end_char,
            clusters,
            break_reason: line.break_reason(),
            grapheme_offsets,
            ascent_overflow: max_ascent_overflow,
            descent_overflow: max_descent_overflow,
        });
    }

    lines
}

fn snap_to_pixel(logical: f32, scale: f64) -> f32 {
    if (scale * 4.0).fract() == 0.0 {
        (logical * scale as f32).floor() / scale as f32
    } else {
        logical
    }
}
