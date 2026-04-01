use editor_common::{Alignment, EdgeInsets, Rect};
use editor_model::{Doc, NodeRef};

use super::super::LayoutEngine;
use super::container::measure_padded_container;
use crate::fragment::PlaceholderData;
use crate::measure::*;
use crate::view_state::ViewState;

const CALLOUT_PADDING_X: f32 = 12.0;
const CALLOUT_PADDING_Y: f32 = 16.0;
const CALLOUT_ICON_WIDTH: f32 = 20.0;
const CALLOUT_ICON_CONTENT_GAP: f32 = 8.0;

pub fn measure_callout(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let padding = EdgeInsets {
        top: CALLOUT_PADDING_Y,
        left: CALLOUT_PADDING_X + CALLOUT_ICON_WIDTH + CALLOUT_ICON_CONTENT_GAP,
        bottom: CALLOUT_PADDING_Y,
        right: CALLOUT_PADDING_X,
    };

    let mut measurement = measure_padded_container(
        engine,
        doc,
        node,
        width,
        view_state,
        padding,
        EdgeInsets::ZERO,
        false,
        Alignment::Start,
    );

    if let MeasuredContent::Container(ref mut content) = measurement.content {
        content.placeholders.push(MeasuredPlaceholder {
            id: 0,
            rect: Rect {
                x: CALLOUT_PADDING_X,
                y: CALLOUT_PADDING_Y,
                width: CALLOUT_ICON_WIDTH,
                height: CALLOUT_ICON_WIDTH,
            },
            data: PlaceholderData::None,
        });
    }

    measurement
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;
    use crate::engine::LayoutEngine;

    #[test]
    fn padding() {
        let (doc, c1) = doc! { root { c1: callout } };

        let node = doc.node(c1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_callout(&mut engine, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Container(ContainerContent { padding, .. }) = &result.content else {
            panic!()
        };

        assert_eq!(padding.top, 16.0);
        assert_eq!(padding.left, 40.0);
        assert_eq!(padding.bottom, 16.0);
        assert_eq!(padding.right, 12.0);
        assert_eq!(result.size.height, 32.0);
    }
}
