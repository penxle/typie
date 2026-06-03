use editor_common::{EdgeInsets, Rect};

use crate::style::Alignment as LayoutAlignment;
use editor_model::{Alignment, Doc, Node, NodeRef};

use crate::measure::Measurer;
use crate::measure::container::{PaddedLayoutConfig, layout_padded};
use crate::measure::text::measure::measure_inline_text;
use crate::measure::{MeasuredBox, MeasuredContent, MeasuredNode, PageBreakPolicy};
use crate::style::{BorderMode, BoxStyle, Decoration, DecorationData, Direction};
use crate::view_state::ViewState;

use super::line_geometry::first_line_info;

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

    let mut measured = MeasuredNode {
        width,
        height: children_height + padding.top + padding.bottom,
        content: MeasuredContent::Box(MeasuredBox {
            node_id: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding,
                border: EdgeInsets::ZERO,
                border_mode: BorderMode::Separate,
                alignment: LayoutAlignment::Start,
                scope: false,
                decorations: vec![],
                monolithic: node.spec().monolithic,
            },
            children,
            page_break_policy: PageBreakPolicy::Avoid,
        }),
    };

    let icon_y = first_line_info(&measured)
        .map(|info| info.top + (info.height - FOLD_TITLE_ICON_WIDTH) / 2.0)
        .unwrap_or(FOLD_TITLE_PADDING_Y);

    if let MeasuredContent::Box(ref mut b) = measured.content {
        b.style.decorations.push(Decoration {
            id: 0,
            rect: Rect {
                x: FOLD_TITLE_PADDING_X,
                y: icon_y,
                width: FOLD_TITLE_ICON_WIDTH,
                height: FOLD_TITLE_ICON_WIDTH,
            },
            data: DecorationData::Bool(expanded),
        });
    }

    measured
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
            page_break_policy: PageBreakPolicy::Auto,
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
                monolithic: node.spec().monolithic,
            },
            children,
            page_break_policy: PageBreakPolicy::Auto,
        }),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;
    use crate::glyph_run::GlyphRun;

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
    fn fold_title_icon_centered_on_first_line() {
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

        let first_line_height = b.children[0].height;
        let icon = b.style.decorations.first().expect("icon decoration");
        let icon_center = icon.rect.y + icon.rect.height / 2.0;
        let first_line_center = FOLD_TITLE_PADDING_Y + first_line_height / 2.0;

        assert!(
            (icon_center - first_line_center).abs() < 0.01,
            "icon center {icon_center} should match first line center {first_line_center}",
        );
    }

    #[test]
    fn fold_title_has_no_separator_border_when_expanded() {
        let (doc, ft1) = doc! {
            root {
                fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let mut view_state = ViewState::new();
        // The removed separator was only ever added in the expanded branch, so
        // the test must expand the fold to exercise that path.
        if let Some(p) = doc.node(ft1).unwrap().parent() {
            view_state.fold_states.insert(p.id(), true);
        }

        let node = doc.node(ft1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_fold_title(&mut measurer, &doc, &node, 300.0, &view_state);

        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert_eq!(b.style.border, EdgeInsets::ZERO);
    }

    fn first_line_glyph_run(result: &MeasuredNode) -> &GlyphRun {
        let MeasuredContent::Box(b) = &result.content else {
            panic!("expected box")
        };
        let first = b
            .children
            .iter()
            .find_map(|c| match &c.content {
                MeasuredContent::Line(l) if !l.glyph_runs.is_empty() => Some(l),
                _ => None,
            })
            .expect("a line with glyph runs");
        &first.glyph_runs[0]
    }

    #[test]
    fn fold_title_text_uses_implicit_style() {
        let (doc, ft1) = doc! {
            root {
                fold {
                    ft1: fold_title { text("1234") }
                    fold_content { paragraph { text("c") } }
                }
            }
        };

        let node = doc.node(ft1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_fold_title(&mut measurer, &doc, &node, 300.0, &ViewState::new());

        let gr = first_line_glyph_run(&result);
        // FoldTitle's implicit FontSize(1050) resolves to 14px.
        assert!(
            (gr.font_size - 14.0).abs() < 0.5,
            "font_size = {}",
            gr.font_size
        );
        assert_eq!(gr.color, "text.gray");
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
