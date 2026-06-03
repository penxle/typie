use editor_resource::Resource;

use super::resolve::ResolvedTextStyle;
use super::text_run::TextRun;

pub const TAB_STOP_SPACES: f32 = 8.0;

pub fn tab_px(style: &ResolvedTextStyle, resource: &mut Resource) -> f32 {
    let space = " ";
    let runs = vec![TextRun {
        node_id: editor_model::NodeId::new(),
        byte_range: 0..space.len(),
        style: style.clone(),
    }];
    let style_runs =
        super::style_run::resolve_style_runs(space, &runs, &mut resource.font_registry);
    let layout = super::layout::build_layout(
        space,
        &style_runs,
        editor_model::Alignment::Left,
        0.0,
        1.0e6,
        resource,
        &[],
    );
    let mut advance = 0.0_f32;
    for line in layout.lines() {
        for item in line.items() {
            if let parley::PositionedLayoutItem::GlyphRun(gr) = item {
                for g in gr.glyphs() {
                    advance += g.advance;
                }
            }
        }
    }
    (advance * TAB_STOP_SPACES).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::Measurer;

    #[test]
    fn tab_px_scales_with_font_size() {
        let measurer = Measurer::new_test();
        let mut resource = measurer.resource.lock().unwrap();
        let small = ResolvedTextStyle {
            font_family: String::new(),
            font_weight: 400,
            font_size: 16.0,
            letter_spacing: 0.0,
            line_height: 1.6,
        };
        let big = ResolvedTextStyle {
            font_size: 32.0,
            ..small.clone()
        };
        let a = tab_px(&small, &mut resource);
        let b = tab_px(&big, &mut resource);
        assert!(b > a, "bigger font → bigger tab_px (a={a}, b={b})");
    }
}
