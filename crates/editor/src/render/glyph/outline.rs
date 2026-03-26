use crate::render::glyph::scale::outline::Outline;
use kurbo::BezPath;
use zeno::Verb;

/// 힌티드 아웃라인을 kurbo BezPath로 변환한다.
/// offset_x, offset_y: 글리프의 화면 좌표 (snapped).
/// 아웃라인의 y축은 위가 양수이므로 origin_y - point.y로 변환한다.
pub fn outline_to_bezpath(outline: &Outline, offset_x: f32, offset_y: f32) -> BezPath {
    let mut path = BezPath::new();
    for layer_idx in 0..outline.len() {
        let Some(layer) = outline.get(layer_idx) else {
            continue;
        };
        append_layer_to_bezpath(&mut path, layer.points(), layer.verbs(), offset_x, offset_y);
    }
    path
}

fn append_layer_to_bezpath(
    path: &mut BezPath,
    points: &[zeno::Point],
    verbs: &[Verb],
    origin_x: f32,
    origin_y: f32,
) {
    let mut point_idx = 0usize;
    for verb in verbs {
        match verb {
            Verb::MoveTo => {
                let Some(p) = points.get(point_idx) else {
                    return;
                };
                point_idx += 1;
                path.move_to(((origin_x + p.x) as f64, (origin_y - p.y) as f64));
            }
            Verb::LineTo => {
                let Some(p) = points.get(point_idx) else {
                    return;
                };
                point_idx += 1;
                path.line_to(((origin_x + p.x) as f64, (origin_y - p.y) as f64));
            }
            Verb::QuadTo => {
                let Some(ctrl) = points.get(point_idx) else {
                    return;
                };
                let Some(p) = points.get(point_idx + 1) else {
                    return;
                };
                point_idx += 2;
                path.quad_to(
                    ((origin_x + ctrl.x) as f64, (origin_y - ctrl.y) as f64),
                    ((origin_x + p.x) as f64, (origin_y - p.y) as f64),
                );
            }
            Verb::CurveTo => {
                let Some(c1) = points.get(point_idx) else {
                    return;
                };
                let Some(c2) = points.get(point_idx + 1) else {
                    return;
                };
                let Some(p) = points.get(point_idx + 2) else {
                    return;
                };
                point_idx += 3;
                path.curve_to(
                    ((origin_x + c1.x) as f64, (origin_y - c1.y) as f64),
                    ((origin_x + c2.x) as f64, (origin_y - c2.y) as f64),
                    ((origin_x + p.x) as f64, (origin_y - p.y) as f64),
                );
            }
            Verb::Close => {
                path.close_path();
            }
        }
    }
}
