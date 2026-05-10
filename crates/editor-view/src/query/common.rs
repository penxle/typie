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
    line.glyph_runs
        .first()
        .map(|r| r.x)
        .unwrap_or(line.text_indent)
}

pub fn line_end_x(line: &LayoutLine) -> f32 {
    line.glyph_runs
        .last()
        .map(|r| r.x + r.width)
        .unwrap_or(line.text_indent)
}
