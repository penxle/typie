use editor_common::Rect;

use crate::types::Color;
use crate::types::{Image, Path, Stroke, Transform};

pub trait RenderSink {
    fn pixel_size(&self) -> (u32, u32);
    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform);
    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform);
    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform);
    fn draw_image(&mut self, image: &Image, rect: Rect, transform: Transform);
}
