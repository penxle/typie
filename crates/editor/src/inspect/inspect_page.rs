use crate::{layout::Page, types::Rect};
use serde::Serialize;

#[derive(Serialize)]
pub struct InspectedElement {
    bounds: Rect,
}

pub fn inspect_page_element(page: &Page, x: f32, y: f32) -> Option<String> {
    let entry = page.spatial_index().locate_at_point(&[x, y])?;

    let pos = entry.pos;
    let element = entry.element();
    let size = element.size();

    let bounds = Rect::new(pos.x, pos.y, size.width, size.height);
    let inspected_element = InspectedElement { bounds };
    serde_json::to_string(&inspected_element).ok()
}
