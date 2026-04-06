use editor_common::{Alignment, EdgeInsets, Rect};
use editor_model::{Doc, NodeRef};

use crate::measure::Measurer;
use crate::measure::container::{PaddedLayoutConfig, layout_padded};
use crate::measure::{MeasuredContent, MeasuredNode};
use crate::style::{Decoration, DecorationData};
use crate::view_state::ViewState;

const CALLOUT_PADDING_X: f32 = 12.0;
const CALLOUT_PADDING_Y: f32 = 16.0;
const CALLOUT_ICON_WIDTH: f32 = 20.0;
const CALLOUT_ICON_CONTENT_GAP: f32 = 8.0;

pub fn measure_callout(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let padding = EdgeInsets {
        top: CALLOUT_PADDING_Y,
        left: CALLOUT_PADDING_X + CALLOUT_ICON_WIDTH + CALLOUT_ICON_CONTENT_GAP,
        bottom: CALLOUT_PADDING_Y,
        right: CALLOUT_PADDING_X,
    };

    let mut measured = layout_padded(
        measurer,
        doc,
        node,
        width,
        view_state,
        PaddedLayoutConfig {
            padding,
            border: EdgeInsets::ZERO,
            scope: false,
            alignment: Alignment::Start,
        },
    );

    if let MeasuredContent::Box(ref mut b) = measured.content {
        b.style.decorations.push(Decoration {
            id: 0,
            rect: Rect {
                x: CALLOUT_PADDING_X,
                y: CALLOUT_PADDING_Y,
                width: CALLOUT_ICON_WIDTH,
                height: CALLOUT_ICON_WIDTH,
            },
            data: DecorationData::None,
        });
    }

    measured
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    #[test]
    fn padding() {
        let (doc, c1) = doc! { root { c1: callout } };

        let node = doc.node(c1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_callout(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert_eq!(b.style.padding.top, 16.0);
        assert_eq!(b.style.padding.left, 40.0);
        assert_eq!(b.style.padding.bottom, 16.0);
        assert_eq!(b.style.padding.right, 12.0);
        assert_eq!(result.height, 32.0);
    }
}
