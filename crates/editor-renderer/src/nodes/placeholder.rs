use editor_common::Rect;
use editor_model::Node;
use editor_view::fragment::{PlaceholderData, PlaceholderFragment};

use crate::icons::ICONS;
use crate::renderer::Renderer;
use crate::sink::RenderSink;
use crate::types::Transform;

pub fn draw(
    renderer: &mut Renderer,
    sink: &mut dyn RenderSink,
    pf: &PlaceholderFragment,
    parent_node: Option<&Node>,
    transform: Transform,
) {
    let t = transform.translate(pf.rect.x, pf.rect.y);
    let local_rect = Rect {
        x: 0.0,
        y: 0.0,
        width: pf.rect.width,
        height: pf.rect.height,
    };

    match (parent_node, pf.id) {
        (Some(Node::Callout(callout)), 0) => {
            let icon_name = match callout.variant {
                editor_model::CalloutVariant::Info => "lucide/info",
                editor_model::CalloutVariant::Success => "lucide/circle-check",
                editor_model::CalloutVariant::Warning => "lucide/circle-alert",
                editor_model::CalloutVariant::Danger => "lucide/triangle-alert",
            };
            let color = renderer.theme.color("ui.text.muted");
            let path = ICONS.resolve(icon_name, local_rect);
            sink.fill_path(&path, color, t);
        }

        (Some(Node::Blockquote(bq)), 0)
            if bq.variant == editor_model::BlockquoteVariant::LeftQuote =>
        {
            let color = renderer.theme.color("ui.text.muted");
            let path = ICONS.resolve("typie/blockquote-quote", local_rect);
            sink.fill_path(&path, color, t);
        }

        (Some(Node::Blockquote(bq)), 0)
            if bq.variant == editor_model::BlockquoteVariant::LeftLine =>
        {
            let color = renderer.theme.color("ui.border.default");
            sink.fill_rect(local_rect, color, t);
        }

        (Some(Node::Fold(_)), 0) => {
            let expanded = matches!(&pf.data, PlaceholderData::Bool(true));
            let icon_name = if expanded {
                "lucide/chevron-up"
            } else {
                "lucide/chevron-down"
            };
            let color = renderer.theme.color("ui.text.muted");
            let path = ICONS.resolve(icon_name, local_rect);
            sink.fill_path(&path, color, t);
        }

        (Some(Node::ListItem(_)), 0) => match &pf.data {
            PlaceholderData::Text(label) => {
                let _ = label;
                let color = renderer.theme.color("ui.text.muted");
                let path = ICONS.resolve("list/ordered", local_rect);
                sink.fill_path(&path, color, t);
            }
            _ => {
                let color = renderer.theme.color("ui.text");
                sink.fill_rect(local_rect, color, t);
            }
        },

        (Some(Node::BulletList(_)), 0) => {
            let color = renderer.theme.color("ui.text");
            sink.fill_rect(local_rect, color, t);
        }

        _ => {}
    }
}
