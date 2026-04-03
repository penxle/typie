use editor_common::{Alignment, EdgeInsets, Rect};
use editor_model::{BlockquoteVariant, Doc, Node, NodeRef};

use crate::measure::Measurer;
use crate::measure::container::layout_padded;
use crate::measure::{MeasuredContent, MeasuredNode};
use crate::style::{Decoration, DecorationData};
use crate::view_state::ViewState;

const BQ_LINE_WIDTH: f32 = 4.0;
const BQ_CONTENT_PADDING: f32 = 16.0;
const BQ_QUOTE_SIZE: f32 = 16.0;
const BQ_QUOTE_CONTENT_GAP: f32 = 16.0;
const BQ_MESSAGE_PADDING_X: f32 = 14.0;
const BQ_MESSAGE_PADDING_Y: f32 = 8.0;
const BQ_MESSAGE_MAX_WIDTH_RATIO: f32 = 0.8;
const BQ_MESSAGE_MIN_WIDTH: f32 = 40.0;

pub fn measure_blockquote(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let Node::Blockquote(bq) = node.node() else {
        unreachable!()
    };

    match bq.variant {
        BlockquoteVariant::LeftLine => {
            let padding = EdgeInsets {
                left: BQ_LINE_WIDTH + BQ_CONTENT_PADDING,
                ..EdgeInsets::ZERO
            };
            let mut measured = layout_padded(
                measurer,
                doc,
                node,
                width,
                view_state,
                padding,
                EdgeInsets::ZERO,
                false,
                Alignment::Start,
            );
            if let MeasuredContent::Box(ref mut b) = measured.content {
                b.style.decorations.push(Decoration {
                    id: 0,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: BQ_LINE_WIDTH,
                        height: measured.height,
                    },
                    data: DecorationData::None,
                });
            }
            measured
        }
        BlockquoteVariant::LeftQuote => {
            let padding = EdgeInsets {
                left: BQ_QUOTE_SIZE + BQ_QUOTE_CONTENT_GAP,
                ..EdgeInsets::ZERO
            };
            let mut measured = layout_padded(
                measurer,
                doc,
                node,
                width,
                view_state,
                padding,
                EdgeInsets::ZERO,
                false,
                Alignment::Start,
            );
            if let MeasuredContent::Box(ref mut b) = measured.content {
                b.style.decorations.push(Decoration {
                    id: 0,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: BQ_QUOTE_SIZE,
                        height: BQ_QUOTE_SIZE,
                    },
                    data: DecorationData::None,
                });
            }
            measured
        }
        BlockquoteVariant::MessageSent | BlockquoteVariant::MessageReceived => {
            let bubble_width = (width * BQ_MESSAGE_MAX_WIDTH_RATIO)
                .max(BQ_MESSAGE_MIN_WIDTH)
                .min(width);
            let padding = EdgeInsets::symmetric(BQ_MESSAGE_PADDING_X, BQ_MESSAGE_PADDING_Y);
            let alignment = if bq.variant == BlockquoteVariant::MessageSent {
                Alignment::End
            } else {
                Alignment::Start
            };
            layout_padded(
                measurer,
                doc,
                node,
                bubble_width,
                view_state,
                padding,
                EdgeInsets::ZERO,
                false,
                alignment,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    #[test]
    fn left_line() {
        let (doc, bq1) = doc! { root { bq1: blockquote(variant: BlockquoteVariant::LeftLine) } };

        let node = doc.node(bq1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_blockquote(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert_eq!(b.style.padding.left, 20.0);
        assert_eq!(b.style.alignment, Alignment::Start);
    }

    #[test]
    fn left_quote() {
        let (doc, bq1) = doc! { root { bq1: blockquote(variant: BlockquoteVariant::LeftQuote) } };

        let node = doc.node(bq1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_blockquote(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert_eq!(b.style.padding.left, 32.0);
    }

    #[test]
    fn message_sent() {
        let (doc, bq1) = doc! { root { bq1: blockquote(variant: BlockquoteVariant::MessageSent) } };

        let node = doc.node(bq1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_blockquote(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert_eq!(b.style.alignment, Alignment::End);
        assert_eq!(result.width, 240.0);
        assert_eq!(b.style.padding.left, 14.0);
        assert_eq!(b.style.padding.right, 14.0);
        assert_eq!(b.style.padding.top, 8.0);
        assert_eq!(b.style.padding.bottom, 8.0);
    }

    #[test]
    fn message_min_width() {
        let (doc, bq1) = doc! { root { bq1: blockquote(variant: BlockquoteVariant::MessageSent) } };

        let node = doc.node(bq1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_blockquote(&mut measurer, &doc, &node, 30.0, &ViewState::new());

        assert_eq!(result.width, 30.0);
    }
}
