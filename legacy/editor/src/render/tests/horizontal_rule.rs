use super::*;
use crate::layout::query::find_node_bounds;

fn render_page_pixels(
    runtime: &mut crate::runtime::Runtime,
    page_idx: usize,
) -> (Vec<u8>, usize, usize) {
    let rendered = runtime.render_page(page_idx).expect("page should render");
    let pixels = unsafe { std::slice::from_raw_parts(rendered.ptr, rendered.len) }.to_vec();
    (pixels, rendered.width as usize, rendered.height as usize)
}

#[test]
fn selected_horizontal_rule_paints_selection_overlay() {
    let mut before_plain = id!();
    let mut hr_plain = id!();
    let mut plain_runtime = runtime! {
        viewport { continuous { width: 800 } }
        doc {
            @before_plain paragraph { text { "before" } }
            @hr_plain horizontal_rule {}
            paragraph { text { "after" } }
        }
        selection { (before_plain, 0) }
    };
    plain_runtime.layout();

    let plain_bounds = find_node_bounds(plain_runtime.doc(), plain_runtime.pages(), hr_plain)
        .expect("horizontal rule bounds should exist");
    let plain_sample_x = (plain_bounds.x + plain_bounds.width * 0.5).floor() as usize;
    let plain_sample_y = (plain_bounds.y + 2.0).floor() as usize;

    let (plain_pixels, plain_width, plain_height) =
        render_page_pixels(&mut plain_runtime, plain_bounds.page_idx);
    let plain_rgba = rgba_at(
        &plain_pixels,
        plain_width,
        plain_sample_x.min(plain_width.saturating_sub(1)),
        plain_sample_y.min(plain_height.saturating_sub(1)),
    );

    let mut hr_selected = id!();
    let mut selected_runtime = runtime! {
        viewport { continuous { width: 800 } }
        doc {
            paragraph { text { "before" } }
            @hr_selected horizontal_rule {}
            paragraph { text { "after" } }
        }
        selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
    };
    selected_runtime.layout();

    let selected_bounds = find_node_bounds(
        selected_runtime.doc(),
        selected_runtime.pages(),
        hr_selected,
    )
    .expect("selected horizontal rule bounds should exist");
    let selected_sample_x = (selected_bounds.x + selected_bounds.width * 0.5).floor() as usize;
    let selected_sample_y = (selected_bounds.y + 2.0).floor() as usize;

    let (selected_pixels, selected_width, selected_height) =
        render_page_pixels(&mut selected_runtime, selected_bounds.page_idx);
    let selected_rgba = rgba_at(
        &selected_pixels,
        selected_width,
        selected_sample_x.min(selected_width.saturating_sub(1)),
        selected_sample_y.min(selected_height.saturating_sub(1)),
    );

    assert_ne!(
        plain_rgba, selected_rgba,
        "horizontal rule가 선택되면 selection overlay가 실제 픽셀에 반영되어야 함"
    );
}
