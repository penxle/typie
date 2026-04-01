use editor_common::{Alignment, Size};
use editor_model::{Node, NodeRef};

use crate::measure::*;
use crate::view_state::ViewState;

const HORIZONTAL_RULE_HEIGHT: f32 = 24.0;

pub fn measure_atom(node: &NodeRef<'_>, width: f32, view_state: &ViewState) -> Measurement {
    let node_id = node.id();
    let parent_id = node.parent().expect("atom must have parent").id();
    let index = node.index().expect("atom must have index");

    let (w, h) = match node.node() {
        Node::Image(img) => {
            let w = img.proportion * width;
            let h = view_state.external_height(node_id).unwrap_or(0.0);
            (w, h)
        }
        Node::HorizontalRule(_) => (width, HORIZONTAL_RULE_HEIGHT),
        _ => {
            // File, Embed, Archived
            let h = view_state.external_height(node_id).unwrap_or(0.0);
            (width, h)
        }
    };

    Measurement {
        size: Size {
            width: w,
            height: h,
        },
        gap_after: 0.0,
        content: MeasuredContent::Atom { parent_id, index },
        alignment: Alignment::Start,
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::*;

    use super::*;

    #[test]
    fn horizontal_rule() {
        let (doc, hr1) = doc! { root { hr1: horizontal_rule } };

        let node = doc.node(hr1).unwrap();
        let result = measure_atom(&node, 300.0, &ViewState::new());

        assert_eq!(result.size.width, 300.0);
        assert_eq!(result.size.height, 24.0);
        assert!(
            matches!(result.content, MeasuredContent::Atom { parent_id, index } if parent_id == NodeId::ROOT && index == 0)
        );
    }

    #[test]
    fn image_with_external_height() {
        let (doc, i1) = doc! { root { i1: image(proportion: 0.5) } };

        let node = doc.node(i1).unwrap();
        let mut vs = ViewState::new();
        vs.external_heights.insert(i1, 200.0);
        let result = measure_atom(&node, 400.0, &vs);

        assert_eq!(result.size.width, 200.0);
        assert_eq!(result.size.height, 200.0);
    }

    #[test]
    fn image_without_external_height() {
        let (doc, i1) = doc! { root { i1: image(proportion: 0.8) } };

        let node = doc.node(i1).unwrap();
        let result = measure_atom(&node, 400.0, &ViewState::new());

        assert_eq!(result.size.width, 320.0);
        assert_eq!(result.size.height, 0.0);
    }

    #[test]
    fn file_with_external_height() {
        let (doc, f1) = doc! { root { f1: file } };

        let node = doc.node(f1).unwrap();
        let mut vs = ViewState::new();
        vs.external_heights.insert(f1, 48.0);
        let result = measure_atom(&node, 300.0, &vs);

        assert_eq!(result.size.width, 300.0);
        assert_eq!(result.size.height, 48.0);
    }
}
