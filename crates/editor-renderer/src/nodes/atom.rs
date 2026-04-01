use editor_common::Rect;
use editor_model::{Doc, HorizontalRuleVariant, Node};
use editor_view::fragment::AtomFragment;

use crate::icons::ICONS;
use crate::renderer::Renderer;
use crate::sink::RenderSink;
use crate::types::Transform;

pub fn draw(
    renderer: &mut Renderer,
    sink: &mut dyn RenderSink,
    af: &AtomFragment,
    doc: &Doc,
    transform: Transform,
) {
    let t = transform.translate(af.rect.x, af.rect.y);
    let local_rect = Rect {
        x: 0.0,
        y: 0.0,
        width: af.rect.width,
        height: af.rect.height,
    };

    let node = doc.node(af.node_id);

    match node.map(|n| n.node()) {
        Some(Node::HorizontalRule(hr)) => {
            let color = renderer.theme.color("ui.border");
            match hr.variant {
                HorizontalRuleVariant::Line => {
                    let path = ICONS.resolve("hr/line", local_rect);
                    sink.fill_path(&path, color, t);
                }
                HorizontalRuleVariant::DashedLine => {
                    let path = ICONS.resolve("hr/dashed-line", local_rect);
                    sink.fill_path(&path, color, t);
                }
                HorizontalRuleVariant::Circle => {
                    let path = ICONS.resolve("hr/circle", local_rect);
                    sink.fill_path(&path, color, t);
                }
                HorizontalRuleVariant::Diamond => {
                    let path = ICONS.resolve("hr/diamond", local_rect);
                    sink.fill_path(&path, color, t);
                }
                HorizontalRuleVariant::ThreeCircles => {
                    let path = ICONS.resolve("hr/three-circles", local_rect);
                    sink.fill_path(&path, color, t);
                }
                HorizontalRuleVariant::ThreeDiamonds => {
                    let path = ICONS.resolve("hr/three-diamonds", local_rect);
                    sink.fill_path(&path, color, t);
                }
                HorizontalRuleVariant::Zigzag => {
                    let path = ICONS.resolve("hr/zigzag", local_rect);
                    sink.fill_path(&path, color, t);
                }
                HorizontalRuleVariant::CircleLine => {
                    let path = ICONS.resolve("hr/circle-line", local_rect);
                    sink.fill_path(&path, color, t);
                }
                HorizontalRuleVariant::DiamondLine => {
                    let path = ICONS.resolve("hr/diamond-line", local_rect);
                    sink.fill_path(&path, color, t);
                }
            }
        }
        // Image, File, Embed, Archived: external content, skip for now
        Some(Node::Image(_) | Node::File(_) | Node::Embed(_) | Node::Archived(_)) => {}
        _ => {}
    }
}
