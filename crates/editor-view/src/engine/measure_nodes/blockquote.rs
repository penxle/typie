use editor_common::{Alignment, EdgeInsets, Rect};
use editor_model::{BlockquoteVariant, Doc, Node, NodeRef};

use super::super::LayoutEngine;
use super::container::measure_padded_container;
use crate::fragment::PlaceholderData;
use crate::measure::*;
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
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let Node::Blockquote(bq) = node.node() else {
        unreachable!()
    };

    match bq.variant {
        BlockquoteVariant::LeftLine => {
            let padding = EdgeInsets {
                left: BQ_LINE_WIDTH + BQ_CONTENT_PADDING,
                ..EdgeInsets::ZERO
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
                        x: 0.0,
                        y: 0.0,
                        width: BQ_LINE_WIDTH,
                        height: measurement.size.height,
                    },
                    data: PlaceholderData::None,
                });
            }
            measurement
        }
        BlockquoteVariant::LeftQuote => {
            let padding = EdgeInsets {
                left: BQ_QUOTE_SIZE + BQ_QUOTE_CONTENT_GAP,
                ..EdgeInsets::ZERO
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
                        x: 0.0,
                        y: 0.0,
                        width: BQ_QUOTE_SIZE,
                        height: BQ_QUOTE_SIZE,
                    },
                    data: PlaceholderData::None,
                });
            }
            measurement
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
            measure_padded_container(
                engine,
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
    use crate::engine::LayoutEngine;

    #[test]
    fn left_line() {
        let (doc, bq1) = doc! { root { bq1: blockquote(variant: BlockquoteVariant::LeftLine) } };

        let node = doc.node(bq1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_blockquote(&mut engine, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Container(ContainerContent { padding, .. }) = &result.content else {
            panic!()
        };

        assert_eq!(padding.left, 20.0);
        assert_eq!(result.alignment, Alignment::Start);
    }

    #[test]
    fn left_quote() {
        let (doc, bq1) = doc! { root { bq1: blockquote(variant: BlockquoteVariant::LeftQuote) } };

        let node = doc.node(bq1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_blockquote(&mut engine, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Container(ContainerContent { padding, .. }) = &result.content else {
            panic!()
        };

        assert_eq!(padding.left, 32.0);
    }

    #[test]
    fn message_sent() {
        let (doc, bq1) = doc! { root { bq1: blockquote(variant: BlockquoteVariant::MessageSent) } };

        let node = doc.node(bq1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_blockquote(&mut engine, &doc, &node, 300.0, &ViewState::new());

        assert_eq!(result.alignment, Alignment::End);
        assert_eq!(result.size.width, 240.0);
        let MeasuredContent::Container(ContainerContent { padding, .. }) = &result.content else {
            panic!()
        };

        assert_eq!(padding.left, 14.0);
        assert_eq!(padding.right, 14.0);
        assert_eq!(padding.top, 8.0);
        assert_eq!(padding.bottom, 8.0);
    }

    #[test]
    fn message_min_width() {
        let (doc, bq1) = doc! { root { bq1: blockquote(variant: BlockquoteVariant::MessageSent) } };

        let node = doc.node(bq1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_blockquote(&mut engine, &doc, &node, 30.0, &ViewState::new());

        assert_eq!(result.size.width, 30.0);
    }
}
