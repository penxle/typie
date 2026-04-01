use skrifa::outline::OutlinePen;
use zeno::Point;

use super::outline::Outline;

pub struct OutlineWriter<'a>(pub &'a mut Outline);

impl OutlinePen for OutlineWriter<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(Point::new(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(Point::new(x, y));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.quad_to(Point::new(cx0, cy0), Point::new(x, y));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0
            .curve_to(Point::new(cx0, cy0), Point::new(cx1, cy1), Point::new(x, y));
    }

    fn close(&mut self) {
        self.0.close();
    }
}
