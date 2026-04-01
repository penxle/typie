use zeno::Verb;

use super::outline::Outline;
use crate::types::{Path, PathElement};

pub fn outline_to_path(outline: &Outline, offset_x: f32, offset_y: f32) -> Path {
    let points = outline.points();
    let verbs = outline.verbs();
    let mut elements = Vec::new();
    let mut point_idx = 0usize;

    for verb in verbs {
        match verb {
            Verb::MoveTo => {
                let p = points[point_idx];
                point_idx += 1;
                elements.push(PathElement::MoveTo {
                    x: offset_x + p.x,
                    y: offset_y - p.y,
                });
            }
            Verb::LineTo => {
                let p = points[point_idx];
                point_idx += 1;
                elements.push(PathElement::LineTo {
                    x: offset_x + p.x,
                    y: offset_y - p.y,
                });
            }
            Verb::QuadTo => {
                let ctrl = points[point_idx];
                let p = points[point_idx + 1];
                point_idx += 2;
                elements.push(PathElement::QuadTo {
                    x1: offset_x + ctrl.x,
                    y1: offset_y - ctrl.y,
                    x: offset_x + p.x,
                    y: offset_y - p.y,
                });
            }
            Verb::CurveTo => {
                let c1 = points[point_idx];
                let c2 = points[point_idx + 1];
                let p = points[point_idx + 2];
                point_idx += 3;
                elements.push(PathElement::CurveTo {
                    x1: offset_x + c1.x,
                    y1: offset_y - c1.y,
                    x2: offset_x + c2.x,
                    y2: offset_y - c2.y,
                    x: offset_x + p.x,
                    y: offset_y - p.y,
                });
            }
            Verb::Close => {
                elements.push(PathElement::Close);
            }
        }
    }

    Path { elements }
}
