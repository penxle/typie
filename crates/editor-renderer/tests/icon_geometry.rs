use editor_renderer::icons::ICONS;
use editor_renderer::types::{IconElement, PathElement};

#[test]
fn circle_icon_arc_midpoints_stay_on_circle() {
    for name in ["lucide/info", "lucide/circle-check", "lucide/circle-alert"] {
        let icon = ICONS.resolve(name).unwrap();
        let IconElement::Stroke { path, .. } = icon.elements[0] else {
            panic!("{name}: first element should be the circle stroke");
        };

        let (mut px, mut py) = (0.0f32, 0.0f32);
        let mut curves = 0;
        for el in path {
            match *el {
                PathElement::MoveTo { x, y } => (px, py) = (x, y),
                PathElement::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    let mx = (px + 3.0 * x1 + 3.0 * x2 + x) / 8.0;
                    let my = (py + 3.0 * y1 + 3.0 * y2 + y) / 8.0;
                    let r = ((mx - 12.0).powi(2) + (my - 12.0).powi(2)).sqrt();
                    assert!(
                        (r - 10.0).abs() < 0.05,
                        "{name}: curve midpoint radius {r} deviates from 10",
                    );
                    (px, py) = (x, y);
                    curves += 1;
                }
                _ => {}
            }
        }
        assert!(curves >= 4, "{name}: expected at least 4 curve segments");
    }
}
