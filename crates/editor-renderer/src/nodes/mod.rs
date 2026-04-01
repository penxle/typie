mod atom;
mod container;
mod line;
mod placeholder;

use editor_model::{Doc, Node};
use editor_view::fragment::Fragment;

use crate::renderer::Renderer;
use crate::sink::RenderSink;
use crate::types::Transform;

pub fn render_fragment(
    renderer: &mut Renderer,
    sink: &mut dyn RenderSink,
    fragment: &Fragment,
    doc: &Doc,
    parent_node: Option<&Node>,
    transform: Transform,
) {
    match fragment {
        Fragment::Container(cf) => container::draw(renderer, sink, cf, doc, transform),
        Fragment::Line(lf) => line::draw(renderer, sink, lf, transform),
        Fragment::Atom(af) => atom::draw(renderer, sink, af, doc, transform),
        Fragment::Placeholder(pf) => placeholder::draw(renderer, sink, pf, parent_node, transform),
    }
}
