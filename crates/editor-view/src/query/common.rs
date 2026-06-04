use crate::page::LayoutPage;
use crate::paginate::LayoutLine;

#[derive(Clone, Copy, PartialEq)]
pub enum Phase {
    Before,
    Inside,
    After,
}

pub fn page_for_y(pages: &[LayoutPage], y: f32) -> Option<usize> {
    pages.iter().position(|p| y >= p.y_start && y < p.y_end)
}

pub fn line_start_x(line: &LayoutLine) -> f32 {
    let glyph_start = line.glyph_runs.first().map(|r| r.x);
    let gap_start = line.tab_gaps.first().map(|g| g.x);
    match (glyph_start, gap_start) {
        (Some(g), Some(t)) => g.min(t),
        (Some(g), None) => g,
        (None, Some(t)) => t,
        (None, None) => line.empty_caret_x,
    }
}

pub fn line_end_x(line: &LayoutLine) -> f32 {
    let clamp = line.content_edge_x;
    let glyph_end = line.glyph_runs.last().map(|r| {
        let raw = r.x + r.width;
        match clamp {
            Some(c) if raw > c => c,
            _ => raw,
        }
    });
    let gap_end = line.tab_gaps.last().map(|g| g.x + g.width);
    match (glyph_end, gap_end) {
        (Some(g), Some(t)) => g.max(t),
        (Some(g), None) => g,
        (None, Some(t)) => t,
        (None, None) => line.empty_caret_x,
    }
}
