use editor_common::{EdgeInsets, Rect};

use crate::style::Alignment as LayoutAlignment;
use editor_model::{Alignment, Doc, Node, NodeRef};

use crate::measure::Measurer;
use crate::measure::container::{PaddedLayoutConfig, layout_padded};
use crate::measure::text::measure::measure_inline_text;
use crate::measure::{MeasuredBox, MeasuredContent, MeasuredNode};
use crate::style::{BorderMode, BoxStyle, Decoration, DecorationData, Direction};
use crate::view_state::ViewState;

const FOLD_TITLE_PADDING_X: f32 = 12.0;
const FOLD_TITLE_PADDING_Y: f32 = 8.0;
const FOLD_TITLE_ICON_WIDTH: f32 = 20.0;
const FOLD_TITLE_ICON_GAP: f32 = 8.0;
const FOLD_CONTENT_PADDING_X: f32 = 24.0;
const FOLD_CONTENT_PADDING_Y: f32 = 16.0;
const FOLD_BORDER_WIDTH: f32 = 1.0;

pub fn measure_fold_title(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let padding = EdgeInsets {
        top: FOLD_TITLE_PADDING_Y,
        left: FOLD_TITLE_PADDING_X + FOLD_TITLE_ICON_WIDTH + FOLD_TITLE_ICON_GAP,
        bottom: FOLD_TITLE_PADDING_Y,
        right: FOLD_TITLE_PADDING_X,
    };

    let expanded = node
        .parent()
        .map(|p| view_state.fold_expanded(p.id()))
        .unwrap_or(true);

    // 펼친 상태일 때만 title과 content를 가르는 1px separator를 BoxStyle.border로 표현.
    // 접힌 상태에서는 외곽 Fold border 만으로 충분.
    let border = EdgeInsets {
        top: 0.0,
        left: 0.0,
        right: 0.0,
        bottom: if expanded { FOLD_BORDER_WIDTH } else { 0.0 },
    };

    let inner_width = width - padding.left - padding.right;
    let (children, children_height) = measure_inline_text(
        measurer,
        doc,
        node,
        inner_width,
        Alignment::Left,
        0.0,
        view_state,
    );

    MeasuredNode {
        width,
        height: children_height + padding.top + padding.bottom + border.top + border.bottom,
        content: MeasuredContent::Box(MeasuredBox {
            node_id: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding,
                border,
                border_mode: BorderMode::Separate,
                alignment: LayoutAlignment::Start,
                scope: false,
                decorations: vec![Decoration {
                    id: 0,
                    rect: Rect {
                        x: FOLD_TITLE_PADDING_X,
                        y: FOLD_TITLE_PADDING_Y,
                        width: FOLD_TITLE_ICON_WIDTH,
                        height: FOLD_TITLE_ICON_WIDTH,
                    },
                    data: DecorationData::Bool(expanded),
                }],
            },
            children,
        }),
    }
}

pub fn measure_fold_content(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let padding = EdgeInsets::symmetric(FOLD_CONTENT_PADDING_X, FOLD_CONTENT_PADDING_Y);

    layout_padded(
        measurer,
        doc,
        node,
        width,
        view_state,
        PaddedLayoutConfig {
            padding,
            border: EdgeInsets::ZERO,
            scope: false,
            alignment: LayoutAlignment::Start,
        },
    )
}

pub fn measure_fold(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let border = EdgeInsets::all(FOLD_BORDER_WIDTH);
    let content_width = width - border.left - border.right;
    let expanded = view_state.fold_expanded(node.id());

    let mut children = Vec::new();
    let mut children_height = 0.0f32;

    for child in node.children() {
        if !expanded && matches!(child.node(), Node::FoldContent(_)) {
            continue;
        }
        let m = measurer.measure(doc, child.id(), content_width, view_state);
        children_height += m.height;
        children.push(m);
    }

    let height = border.top + children_height + border.bottom;

    MeasuredNode {
        width,
        height,
        content: MeasuredContent::Box(MeasuredBox {
            node_id: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding: EdgeInsets::ZERO,
                border,
                border_mode: BorderMode::Separate,
                alignment: LayoutAlignment::Start,
                scope: false,
                decorations: vec![],
            },
            children,
        }),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    #[test]
    fn fold_collapsed_excludes_content() {
        let (doc, f1) = doc! {
            root {
                f1: fold {
                    fold_title { text("Title") }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let mut view_state = ViewState::new();
        view_state.fold_states.insert(f1, false);

        let node = doc.node(f1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_fold(&mut measurer, &doc, &node, 300.0, &view_state);

        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        let box_children: Vec<_> = b
            .children
            .iter()
            .filter(|c| matches!(c.content, MeasuredContent::Box(_)))
            .collect();
        assert_eq!(box_children.len(), 1);
        assert_eq!(b.style.border.top, FOLD_BORDER_WIDTH);
        assert_eq!(b.style.border.left, FOLD_BORDER_WIDTH);
    }

    #[test]
    fn fold_expanded_includes_content() {
        let (doc, f1) = doc! {
            root {
                f1: fold {
                    fold_title { text("Title") }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let mut view_state = ViewState::new();
        view_state.fold_states.insert(f1, true);

        let node = doc.node(f1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_fold(&mut measurer, &doc, &node, 300.0, &view_state);

        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        let box_children: Vec<_> = b
            .children
            .iter()
            .filter(|c| matches!(c.content, MeasuredContent::Box(_)))
            .collect();
        assert_eq!(box_children.len(), 2);
    }

    #[test]
    fn fold_title_has_icon_padding() {
        let (doc, ft1) = doc! {
            root {
                fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let node = doc.node(ft1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_fold_title(&mut measurer, &doc, &node, 300.0, &ViewState::new());

        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert_eq!(b.style.padding.left, 40.0);
        assert_eq!(b.style.padding.right, 12.0);
        assert_eq!(b.style.padding.top, 8.0);
        assert_eq!(b.style.padding.bottom, 8.0);
    }

    #[test]
    fn fold_content_has_padding() {
        let (doc, fc1) = doc! {
            root {
                fold {
                    fold_title { text("Title") }
                    fc1: fold_content { paragraph { text("Content") } }
                }
            }
        };

        let node = doc.node(fc1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_fold_content(&mut measurer, &doc, &node, 300.0, &ViewState::new());

        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert_eq!(b.style.padding.left, 24.0);
        assert_eq!(b.style.padding.right, 24.0);
        assert_eq!(b.style.padding.top, 16.0);
        assert_eq!(b.style.padding.bottom, 16.0);
    }
}
