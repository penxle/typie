use editor_common::{Alignment, EdgeInsets, Rect, Size};
use editor_model::{Doc, Node, NodeRef};

use super::super::LayoutEngine;
use super::super::resolve::resolve_gap_after;
use super::container::measure_padded_container;
use crate::fragment::PlaceholderData;
use crate::measure::*;
use crate::view_state::ViewState;

const FOLD_TITLE_PADDING_X: f32 = 12.0;
const FOLD_TITLE_PADDING_Y: f32 = 8.0;
const FOLD_TITLE_ICON_WIDTH: f32 = 20.0;
const FOLD_TITLE_ICON_GAP: f32 = 8.0;
const FOLD_CONTENT_PADDING_X: f32 = 24.0;
const FOLD_CONTENT_PADDING_Y: f32 = 16.0;
const FOLD_BORDER_WIDTH: f32 = 1.0;

pub fn measure_fold_title(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let padding = EdgeInsets {
        top: FOLD_TITLE_PADDING_Y,
        left: FOLD_TITLE_PADDING_X + FOLD_TITLE_ICON_WIDTH + FOLD_TITLE_ICON_GAP,
        bottom: FOLD_TITLE_PADDING_Y,
        right: FOLD_TITLE_PADDING_X,
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

    let expanded = node
        .parent()
        .map(|p| view_state.fold_expanded(p.id()))
        .unwrap_or(true);

    if let MeasuredContent::Container(ref mut content) = measurement.content {
        content.placeholders.push(MeasuredPlaceholder {
            id: 0,
            rect: Rect {
                x: FOLD_TITLE_PADDING_X,
                y: FOLD_TITLE_PADDING_Y,
                width: FOLD_TITLE_ICON_WIDTH,
                height: FOLD_TITLE_ICON_WIDTH,
            },
            data: PlaceholderData::Bool(expanded),
        });
    }

    measurement
}

pub fn measure_fold_content(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let padding = EdgeInsets::symmetric(FOLD_CONTENT_PADDING_X, FOLD_CONTENT_PADDING_Y);

    measure_padded_container(
        engine,
        doc,
        node,
        width,
        view_state,
        padding,
        EdgeInsets::ZERO,
        false,
        Alignment::Start,
    )
}

pub fn measure_fold(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let border = EdgeInsets::all(FOLD_BORDER_WIDTH);
    let content_width = width - border.left - border.right;
    let expanded = view_state.fold_expanded(node.id());

    let children: Vec<ChildMeasurement> = node
        .children()
        .filter(|child| {
            if expanded {
                true
            } else {
                !matches!(child.node(), Node::FoldContent(_))
            }
        })
        .map(|child| {
            let m = engine.measure(doc, child.id(), content_width, view_state);
            ChildMeasurement {
                node_id: child.id(),
                measurement: m,
            }
        })
        .collect();

    let children_height: f32 = children.iter().map(|c| c.measurement.size.height).sum();
    let height = border.top + children_height + border.bottom;

    Measurement {
        size: Size { width, height },
        gap_after: resolve_gap_after(node),
        content: MeasuredContent::Container(ContainerContent {
            children,
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
            border,
            border_mode: BorderMode::Separate,
            placeholders: vec![],
        }),
        alignment: Alignment::Start,
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;
    use crate::engine::LayoutEngine;

    #[test]
    fn fold_collapsed_excludes_content() {
        let (doc, f1) = doc! {
            root {
                f1: fold {
                    fold_title { paragraph { text("Title") } }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let mut view_state = ViewState::new();
        view_state.fold_states.insert(f1, false);

        let node = doc.node(f1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_fold(&mut engine, &doc, &node, 300.0, &view_state);

        let MeasuredContent::Container(ContainerContent {
            children, border, ..
        }) = &result.content
        else {
            panic!()
        };

        assert_eq!(children.len(), 1);
        assert_eq!(border.top, FOLD_BORDER_WIDTH);
        assert_eq!(border.left, FOLD_BORDER_WIDTH);
    }

    #[test]
    fn fold_expanded_includes_content() {
        let (doc, f1) = doc! {
            root {
                f1: fold {
                    fold_title { paragraph { text("Title") } }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let mut view_state = ViewState::new();
        view_state.fold_states.insert(f1, true);

        let node = doc.node(f1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_fold(&mut engine, &doc, &node, 300.0, &view_state);

        let MeasuredContent::Container(ContainerContent { children, .. }) = &result.content else {
            panic!()
        };

        assert_eq!(children.len(), 2);
    }

    #[test]
    fn fold_title_has_icon_padding() {
        let (doc, ft1) = doc! {
            root {
                fold {
                    ft1: fold_title { paragraph { text("Title") } }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let node = doc.node(ft1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_fold_title(&mut engine, &doc, &node, 300.0, &ViewState::new());

        let MeasuredContent::Container(ContainerContent { padding, .. }) = &result.content else {
            panic!()
        };

        assert_eq!(padding.left, 40.0);
        assert_eq!(padding.right, 12.0);
        assert_eq!(padding.top, 8.0);
        assert_eq!(padding.bottom, 8.0);
    }

    #[test]
    fn fold_content_has_padding() {
        let (doc, fc1) = doc! {
            root {
                fold {
                    fold_title { paragraph { text("Title") } }
                    fc1: fold_content { paragraph { text("Content") } }
                }
            }
        };

        let node = doc.node(fc1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_fold_content(&mut engine, &doc, &node, 300.0, &ViewState::new());

        let MeasuredContent::Container(ContainerContent { padding, .. }) = &result.content else {
            panic!()
        };

        assert_eq!(padding.left, 24.0);
        assert_eq!(padding.right, 24.0);
        assert_eq!(padding.top, 16.0);
        assert_eq!(padding.bottom, 16.0);
    }
}
