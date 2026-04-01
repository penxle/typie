use editor_common::Rect;
use editor_model::{Doc, Node};
use editor_view::fragment::ContainerFragment;

use crate::renderer::Renderer;
use crate::sink::RenderSink;
use crate::types::{Path, Transform};

pub fn draw(
    renderer: &mut Renderer,
    sink: &mut dyn RenderSink,
    cf: &ContainerFragment,
    doc: &Doc,
    transform: Transform,
) {
    let t = transform.translate(cf.rect.x, cf.rect.y);
    let local_rect = Rect {
        x: 0.0,
        y: 0.0,
        width: cf.rect.width,
        height: cf.rect.height,
    };

    let node = doc.node(cf.node_id).map(|n| n.node().clone());

    match &node {
        Some(Node::Callout(callout)) => {
            let token = match callout.variant {
                editor_model::CalloutVariant::Info => "ui.callout.info",
                editor_model::CalloutVariant::Success => "ui.callout.success",
                editor_model::CalloutVariant::Warning => "ui.callout.warning",
                editor_model::CalloutVariant::Danger => "ui.callout.danger",
            };
            let color = renderer.theme.color_with_alpha(token, 8);
            sink.fill_rect(local_rect, color, t);
        }
        Some(Node::Fold(_)) => {
            let color = renderer.theme.color("ui.surface.muted");
            sink.fill_rect(local_rect, color, t);
        }
        _ => {}
    }

    let node_ref = node.as_ref();
    for child in &cf.children {
        super::render_fragment(renderer, sink, child, doc, node_ref, transform);
    }

    let b = &cf.border;
    let border_color = match &node {
        Some(Node::Blockquote(_)) => renderer.theme.color("ui.border.default"),
        Some(Node::Fold(_)) => renderer.theme.color("ui.border.default"),
        Some(Node::Table(_)) => renderer.theme.color("ui.border.default"),
        _ => renderer.theme.color("ui.border"),
    };

    if b.left > 0.0 {
        let path = Path::rect(Rect {
            x: 0.0,
            y: 0.0,
            width: b.left,
            height: cf.rect.height,
        });
        sink.fill_path(&path, border_color, t);
    }

    if b.right > 0.0 {
        let path = Path::rect(Rect {
            x: cf.rect.width - b.right,
            y: 0.0,
            width: b.right,
            height: cf.rect.height,
        });
        sink.fill_path(&path, border_color, t);
    }

    if b.top > 0.0 {
        let path = Path::rect(Rect {
            x: 0.0,
            y: 0.0,
            width: cf.rect.width,
            height: b.top,
        });
        sink.fill_path(&path, border_color, t);
    }

    if b.bottom > 0.0 {
        let path = Path::rect(Rect {
            x: 0.0,
            y: cf.rect.height - b.bottom,
            width: cf.rect.width,
            height: b.bottom,
        });
        sink.fill_path(&path, border_color, t);
    }
}
