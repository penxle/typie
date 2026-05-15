use editor_common::{EdgeInsets, Rect};

use editor_model::{BlockquoteVariant, Doc, Node, NodeRef};

use crate::measure::Measurer;
use crate::measure::container::{PaddedLayoutConfig, layout_padded, layout_vertical};
use crate::measure::{MeasuredBox, MeasuredContent, MeasuredNode};
use crate::style::{Alignment, BorderMode, BoxStyle, Decoration, DecorationData, Direction};
use crate::view_state::ViewState;

const BQ_LINE_WIDTH: f32 = 4.0;
const BQ_CONTENT_PADDING: f32 = 16.0;
const BQ_QUOTE_SIZE: f32 = 16.0;
const BQ_QUOTE_CONTENT_GAP: f32 = 16.0;
const BQ_MESSAGE_PADDING_X: f32 = 14.0;
const BQ_MESSAGE_PADDING_Y: f32 = 8.0;
const BQ_MESSAGE_MAX_WIDTH_RATIO: f32 = 0.8;
const BQ_MESSAGE_MIN_WIDTH: f32 = 40.0;
const BQ_MESSAGE_LAYOUT_GUARD_WIDTH: f32 = 1.0;

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

    match *bq.variant.get() {
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
            let inner_max_width =
                (width * BQ_MESSAGE_MAX_WIDTH_RATIO - BQ_MESSAGE_PADDING_X * 2.0).max(0.0);
            let min_inner_width = BQ_MESSAGE_MIN_WIDTH - BQ_MESSAGE_PADDING_X * 2.0;

            let (children_pass1, height_pass1) =
                layout_vertical(measurer, doc, node, inner_max_width, view_state);

            let intrinsic = children_pass1
                .iter()
                .map(|c| measured_intrinsic_width(c.as_ref()))
                .fold(min_inner_width, f32::max);

            let final_inner_width = (intrinsic + BQ_MESSAGE_LAYOUT_GUARD_WIDTH)
                .min(inner_max_width)
                .max(min_inner_width);

            let (children, total_height) = if final_inner_width >= inner_max_width {
                (children_pass1, height_pass1)
            } else {
                layout_vertical(measurer, doc, node, final_inner_width, view_state)
            };

            let bubble_width = (final_inner_width + BQ_MESSAGE_PADDING_X * 2.0).min(width);
            let bubble_height = total_height + BQ_MESSAGE_PADDING_Y * 2.0;
            let padding = EdgeInsets::symmetric(BQ_MESSAGE_PADDING_X, BQ_MESSAGE_PADDING_Y);
            let alignment = if *bq.variant.get() == BlockquoteVariant::MessageSent {
                Alignment::End
            } else {
                Alignment::Start
            };

            MeasuredNode {
                width: bubble_width,
                height: bubble_height,
                content: MeasuredContent::Box(MeasuredBox {
                    node_id: node.id(),
                    style: BoxStyle {
                        direction: Direction::Vertical,
                        padding,
                        border: EdgeInsets::ZERO,
                        border_mode: BorderMode::Separate,
                        alignment,
                        scope: false,
                        decorations: vec![],
                        monolithic: node.spec().monolithic,
                    },
                    children,
                }),
            }
        }
    }
}

fn measured_intrinsic_width(node: &MeasuredNode) -> f32 {
    match &node.content {
        MeasuredContent::Line(l) => l.glyph_runs.iter().map(|r| r.width).sum(),
        MeasuredContent::Box(b) => {
            let children_max = b
                .children
                .iter()
                .map(|c| measured_intrinsic_width(c.as_ref()))
                .fold(0.0_f32, f32::max);
            children_max
                + b.style.padding.left
                + b.style.padding.right
                + b.style.border.left
                + b.style.border.right
        }
        MeasuredContent::Atom(_) => node.width,
        MeasuredContent::Spacing(_) | MeasuredContent::PageBreak => 0.0,
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
    fn message_sent_uses_intrinsic_width_below_cap() {
        let (doc, bq1) = doc! { root { bq1: blockquote(variant: BlockquoteVariant::MessageSent) { paragraph { text("hi") } } } };

        let node = doc.node(bq1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_blockquote(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert!(
            result.width < 240.0,
            "bubble width should hug content, got {}",
            result.width
        );
        assert!(result.width >= 40.0);
        assert_eq!(b.style.alignment, Alignment::End);
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

    #[test]
    fn message_sent_caps_at_max_ratio_for_long_content() {
        let (doc, bq1) = doc! {
            root {
                bq1: blockquote(variant: BlockquoteVariant::MessageSent) {
                    paragraph {
                        text("The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog.")
                    }
                }
            }
        };

        let node = doc.node(bq1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_blockquote(&mut measurer, &doc, &node, 300.0, &ViewState::new());

        assert!(
            (result.width - 240.0).abs() < 0.01,
            "expected cap at 240.0, got {}",
            result.width
        );
    }

    #[test]
    fn message_sent_uses_max_paragraph_width() {
        let (doc, bq1) = doc! {
            root {
                bq1: blockquote(variant: BlockquoteVariant::MessageSent) {
                    paragraph { text("a") }
                    paragraph { text("longer paragraph here") }
                    paragraph { text("mid") }
                }
            }
        };

        let node = doc.node(bq1).unwrap();
        let mut measurer = Measurer::new_test();
        let result_three = measure_blockquote(&mut measurer, &doc, &node, 300.0, &ViewState::new());

        let (doc2, bq2) = doc! {
            root {
                bq2: blockquote(variant: BlockquoteVariant::MessageSent) {
                    paragraph { text("longer paragraph here") }
                }
            }
        };
        let node2 = doc2.node(bq2).unwrap();
        let mut measurer2 = Measurer::new_test();
        let result_single =
            measure_blockquote(&mut measurer2, &doc2, &node2, 300.0, &ViewState::new());

        assert!(
            (result_three.width - result_single.width).abs() < 0.01,
            "expected width to match longest paragraph: three={}, single={}",
            result_three.width,
            result_single.width
        );
    }

    #[test]
    fn message_received_alignment_start() {
        let (doc, bq1) = doc! { root { bq1: blockquote(variant: BlockquoteVariant::MessageReceived) { paragraph { text("hi") } } } };

        let node = doc.node(bq1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_blockquote(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert_eq!(b.style.alignment, Alignment::Start);
    }
}
