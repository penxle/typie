mod hit_test;
mod navigation;
pub mod search;
pub mod segmentation;

pub use hit_test::hit_test;
pub use navigation::resolve_movement;

use editor_common::Rect;
use editor_state::Position;

use crate::page::Page;
use crate::{PageRect, fragment::*};

pub fn cursor_rect(pages: &[Page], pos: &Position) -> Option<PageRect> {
    let (page_idx, line) = search::find_line_at(pages, pos)?;
    let x = x_at_offset(line, pos);

    Some(PageRect::new(
        page_idx,
        Rect::from_xywh(line.rect.x + x, line.rect.y, 1.0, line.rect.height),
    ))
}

pub(crate) fn x_at_offset(line: &LineFragment, pos: &Position) -> f32 {
    for run in &line.glyph_runs {
        if run.node_id != pos.node_id {
            continue;
        }

        let local_offset = pos.offset.saturating_sub(run.offset);
        if local_offset > run.char_advances.len() {
            continue;
        }

        return run.x + run.char_advances[..local_offset].iter().sum::<f32>();
    }

    0.0
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect, Size};
    use editor_model::NodeId;

    use super::*;

    fn make_run(
        node_id: NodeId,
        offset: usize,
        text: &str,
        x: f32,
        advances: Vec<f32>,
    ) -> GlyphRun {
        GlyphRun::make_test_run(node_id, offset, text, x, advances)
    }

    fn single_line_page(id: NodeId) -> Page {
        Page::new(
            Size::new(200.0, 800.0),
            vec![Fragment::Container(ContainerFragment {
                node_id: NodeId::new(),
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 200.0,
                    height: 20.0,
                },
                children: vec![Fragment::Line(LineFragment {
                    node_id: id,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 200.0,
                        height: 20.0,
                    },
                    baseline: 16.0,
                    glyph_runs: vec![make_run(id, 0, "hello", 0.0, vec![10.0; 5])],
                })],
                scope: false,
                breaks: Breaks::default(),
                border: EdgeInsets::default(),
            })],
        )
    }

    #[test]
    fn cursor_rect_at_offset_0() {
        let id = NodeId::new();
        let page = single_line_page(id);
        let pos = Position::new(id, 0);
        let PageRect { page_idx, rect } = cursor_rect(&[page], &pos).unwrap();

        assert_eq!(page_idx, 0);
        assert_eq!(rect.x, 0.0);
        assert_eq!(rect.y, 0.0);
        assert_eq!(rect.height, 20.0);
    }

    #[test]
    fn cursor_rect_at_offset_3() {
        let id = NodeId::new();
        let page = single_line_page(id);
        let pos = Position::new(id, 3);
        let PageRect { rect, .. } = cursor_rect(&[page], &pos).unwrap();

        assert_eq!(rect.x, 30.0);
    }
}
