use editor_crdt::Dot;
use editor_macros::ffi;
use editor_state::{Selection, StableResolveCtx, StableSelection, State};
use serde::{Deserialize, Serialize};

use super::cursor::cursor_metrics;
use super::hit_test::hit_test;
use super::layout_index::LayoutIndex;
use super::selection::{selection_endpoint_for_position, selection_endpoints};
use crate::page::PageRect;
use crate::paginate::types::LayoutContent;

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ViewportAnchorPoint {
    pub page_idx: usize,
    pub x: f32,
    pub y: f32,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedViewportAnchor {
    pub point: ViewportAnchorPoint,
    pub rect: Option<PageRect>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewportAnchorResolution {
    Resolved { geometry: ResolvedViewportAnchor },
    Unavailable,
    Deleted,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewportAnchorPositionGeometry {
    CursorLine,
    SelectionEndpoint,
    Point { offset_x: f32, offset_y: f32 },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViewportAnchor {
    Position {
        stable: StableSelection,
        original_node: Dot,
        geometry: ViewportAnchorPositionGeometry,
    },
    Node {
        node: Dot,
        offset_x: f32,
        offset_y: f32,
    },
}

pub struct ViewportAnchorPresentation {
    state: State,
    layout_index: LayoutIndex,
}

impl ViewportAnchorPresentation {
    pub(crate) fn new(state: State, layout_index: LayoutIndex) -> Self {
        Self {
            state,
            layout_index,
        }
    }

    pub fn capture_selection_head(&self) -> Option<ViewportAnchor> {
        capture_selection_head(&self.state, &self.layout_index)
    }

    pub fn capture_page_point(&self, point: ViewportAnchorPoint) -> Option<ViewportAnchor> {
        capture_page_point(&self.state, &self.layout_index, point)
    }

    pub fn resolve(&self, anchor: &ViewportAnchor) -> ViewportAnchorResolution {
        resolve(&self.state, &self.layout_index, anchor)
    }
}

pub(crate) fn capture_selection_head(
    state: &State,
    layout_index: &LayoutIndex,
) -> Option<ViewportAnchor> {
    let selection = state.selection?;
    let doc = state.view();
    let resolved = selection.resolve(&doc)?;
    let geometry = if selection.is_collapsed() {
        ViewportAnchorPositionGeometry::CursorLine
    } else {
        let endpoints = selection_endpoints(layout_index, &resolved)?;
        if selection.head != endpoints.from_position && selection.head != endpoints.to_position {
            return None;
        }
        ViewportAnchorPositionGeometry::SelectionEndpoint
    };
    let stable = StableSelection::capture(&Selection::collapsed(selection.head), &doc);
    Some(ViewportAnchor::Position {
        stable,
        original_node: selection.head.node,
        geometry,
    })
}

pub(crate) fn capture_page_point(
    state: &State,
    layout_index: &LayoutIndex,
    point: ViewportAnchorPoint,
) -> Option<ViewportAnchor> {
    let layout_point = layout_index.point(point.page_idx, point.x, point.y)?;
    if let Some((entry, node)) =
        layout_index.exact_entry_with(layout_point, |_entry, layout_node| {
            match &layout_node.content {
                LayoutContent::Atom(atom) => Some(atom.node),
                LayoutContent::Box(layout_box) if layout_box.style.monolithic => {
                    Some(layout_box.node)
                }
                LayoutContent::Box(_) | LayoutContent::Line(_) | LayoutContent::Spacing(_) => None,
            }
        })
    {
        return Some(ViewportAnchor::Node {
            node,
            offset_x: layout_point.x - entry.rect.x,
            offset_y: layout_point.y - entry.rect.y,
        });
    }

    let selection = hit_test(layout_index, point.page_idx, point.x, point.y)?;
    let position = selection.head;
    let base = position_point(layout_index, &position)?;
    let stable = StableSelection::capture(&Selection::collapsed(position), &state.view());
    Some(ViewportAnchor::Position {
        stable,
        original_node: position.node,
        geometry: ViewportAnchorPositionGeometry::Point {
            offset_x: point.x - base.x,
            offset_y: point.y - base.y,
        },
    })
}

pub(crate) fn resolve(
    state: &State,
    layout_index: &LayoutIndex,
    anchor: &ViewportAnchor,
) -> ViewportAnchorResolution {
    match anchor {
        ViewportAnchor::Position {
            stable,
            original_node,
            geometry,
        } => resolve_position(state, layout_index, stable, *original_node, geometry),
        ViewportAnchor::Node {
            node,
            offset_x,
            offset_y,
        } => resolve_node(state, layout_index, *node, *offset_x, *offset_y),
    }
}

fn resolve_node(
    state: &State,
    layout_index: &LayoutIndex,
    node: Dot,
    offset_x: f32,
    offset_y: f32,
) -> ViewportAnchorResolution {
    let doc = state.view();
    if doc.node(node).is_none() && doc.leaf(node).is_none() {
        return ViewportAnchorResolution::Deleted;
    }
    let Some(entry) = layout_index.entry_for_content_node(&node) else {
        return ViewportAnchorResolution::Unavailable;
    };
    let y = entry.rect.y + offset_y;
    let Some(page_idx) = layout_index.page_idx_for_y(y) else {
        return ViewportAnchorResolution::Unavailable;
    };
    let Some(page_y_start) = layout_index.page_y_start(page_idx) else {
        return ViewportAnchorResolution::Unavailable;
    };
    ViewportAnchorResolution::Resolved {
        geometry: ResolvedViewportAnchor {
            point: ViewportAnchorPoint {
                page_idx,
                x: entry.rect.x + offset_x,
                y: y - page_y_start,
            },
            rect: None,
        },
    }
}

fn resolve_position(
    state: &State,
    layout_index: &LayoutIndex,
    stable: &StableSelection,
    original_node: Dot,
    geometry: &ViewportAnchorPositionGeometry,
) -> ViewportAnchorResolution {
    let doc = state.view();
    if doc.node(original_node).is_none() {
        return ViewportAnchorResolution::Deleted;
    }
    let ctx = StableResolveCtx::from_live(&doc, state.projected.seq_checkout());
    let Some(position) = stable.resolve(&ctx).map(|selection| selection.head) else {
        return ViewportAnchorResolution::Deleted;
    };

    let resolved = match geometry {
        ViewportAnchorPositionGeometry::CursorLine => {
            let Some(cursor) = cursor_metrics(layout_index, &position, None) else {
                return ViewportAnchorResolution::Unavailable;
            };
            ResolvedViewportAnchor {
                point: ViewportAnchorPoint {
                    page_idx: cursor.page_idx,
                    x: cursor.caret.x,
                    y: cursor.line.y + cursor.line.height / 2.0,
                },
                rect: Some(PageRect::new(cursor.page_idx, cursor.line)),
            }
        }
        ViewportAnchorPositionGeometry::SelectionEndpoint => {
            let Some(rect) = selection_endpoint_for_position(layout_index, &position) else {
                return ViewportAnchorResolution::Unavailable;
            };
            ResolvedViewportAnchor {
                point: ViewportAnchorPoint {
                    page_idx: rect.page_idx,
                    x: rect.rect.x,
                    y: rect.rect.y + rect.rect.height / 2.0,
                },
                rect: Some(rect),
            }
        }
        ViewportAnchorPositionGeometry::Point { offset_x, offset_y } => {
            let Some(base) = position_point(layout_index, &position) else {
                return ViewportAnchorResolution::Unavailable;
            };
            ResolvedViewportAnchor {
                point: ViewportAnchorPoint {
                    page_idx: base.page_idx,
                    x: base.x + offset_x,
                    y: base.y + offset_y,
                },
                rect: None,
            }
        }
    };
    ViewportAnchorResolution::Resolved { geometry: resolved }
}

fn position_point(
    layout_index: &LayoutIndex,
    position: &editor_state::Position,
) -> Option<ViewportAnchorPoint> {
    if let Some(cursor) = cursor_metrics(layout_index, position, None) {
        return Some(ViewportAnchorPoint {
            page_idx: cursor.page_idx,
            x: cursor.caret.x,
            y: cursor.line.y + cursor.line.height / 2.0,
        });
    }
    let rect = selection_endpoint_for_position(layout_index, position)?;
    Some(ViewportAnchorPoint {
        page_idx: rect.page_idx,
        x: rect.rect.x,
        y: rect.rect.y + rect.rect.height / 2.0,
    })
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;
    use editor_crdt::ListOp;
    use editor_model::{AtomLeaf, EditOp, Node, NodeType, SeqItem};
    use editor_state::{Affinity, Position, ProjectedState, Selection, State};

    use crate::{PageRect, View};

    fn expect_resolved(
        resolution: super::ViewportAnchorResolution,
    ) -> super::ResolvedViewportAnchor {
        match resolution {
            super::ViewportAnchorResolution::Resolved { geometry } => geometry,
            other => panic!("expected resolved anchor geometry, got {other:?}"),
        }
    }

    #[test]
    fn collapsed_selection_anchor_resolves_cursor_line_and_midpoint() {
        let mut projected = ProjectedState::empty();
        projected.commit();
        let paragraph = projected
            .view()
            .root()
            .expect("empty document must have a root")
            .child_blocks()
            .next()
            .expect("empty document must have a paragraph")
            .id();
        let position = Position::new(paragraph, 0);
        let state = State::new(projected, Some(Selection::collapsed(position)));
        let mut view = View::new_test();
        view.layout(&state);

        let presentation = view
            .capture_viewport_anchor_presentation()
            .expect("laid out view must produce an anchor presentation");
        let anchor = presentation
            .capture_selection_head()
            .expect("collapsed selection must produce an anchor");
        let resolved = expect_resolved(presentation.resolve(&anchor));
        let cursor = view
            .cursor_metrics(&state, &position)
            .expect("collapsed selection must have cursor metrics");

        assert_eq!(
            resolved.rect,
            Some(PageRect::new(cursor.page_idx, cursor.line))
        );
        assert_eq!(resolved.point.page_idx, cursor.page_idx);
        assert_eq!(resolved.point.x, cursor.caret.x);
        assert_eq!(resolved.point.y, cursor.line.y + cursor.line.height / 2.0);
    }

    #[test]
    fn page_point_in_text_resolves_to_the_exact_captured_point() {
        let mut projected = ProjectedState::empty();
        projected.commit();
        projected
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('x'),
            }))
            .expect("text insert must apply");
        let state = State::new(projected, None);
        let mut view = View::new_test();
        view.layout(&state);
        let presentation = view
            .capture_viewport_anchor_presentation()
            .expect("laid out view must produce an anchor presentation");
        let point = super::ViewportAnchorPoint {
            page_idx: 0,
            x: 173.0,
            y: 117.0,
        };

        let anchor = presentation
            .capture_page_point(point.clone())
            .expect("a point near laid out text must produce an anchor");
        let resolved = expect_resolved(presentation.resolve(&anchor));

        assert_eq!(resolved.point, point);
        assert_eq!(resolved.rect, None);
    }

    #[test]
    fn selection_range_anchor_uses_the_head_endpoint_rect_and_midpoint() {
        let mut projected = ProjectedState::empty();
        projected.commit();
        projected
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('x'),
            }))
            .expect("text insert must apply");
        let paragraph = projected
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .id();
        let selection = Selection::new(Position::new(paragraph, 0), Position::new(paragraph, 1));
        let state = State::new(projected, Some(selection));
        let mut view = View::new_test();
        view.layout(&state);
        let presentation = view.capture_viewport_anchor_presentation().unwrap();
        let anchor = presentation.capture_selection_head().unwrap();

        let resolved = expect_resolved(presentation.resolve(&anchor));
        let expected =
            super::selection_endpoint_for_position(&presentation.layout_index, &selection.head)
                .expect("selection head must have endpoint geometry");

        assert_eq!(resolved.rect, Some(expected.clone()));
        assert_eq!(resolved.point.page_idx, expected.page_idx);
        assert_eq!(resolved.point.x, expected.rect.x);
        assert_eq!(
            resolved.point.y,
            expected.rect.y + expected.rect.height / 2.0
        );
    }

    #[test]
    fn text_point_moves_by_the_same_geometry_delta_as_its_stable_position() {
        let mut projected = ProjectedState::empty();
        projected.commit();
        projected
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('x'),
            }))
            .expect("text insert must apply");
        let paragraph = projected
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .id();
        let position = Position::new(paragraph, 0);
        let state = State::new(projected, None);
        let mut before_view = View::new_test();
        before_view.layout(&state);
        let before_cursor = before_view.cursor_metrics(&state, &position).unwrap();
        let point = super::ViewportAnchorPoint {
            page_idx: before_cursor.page_idx,
            x: before_cursor.caret.x + 5.0,
            y: before_cursor.line.y + before_cursor.line.height / 2.0,
        };
        let anchor = before_view
            .capture_viewport_anchor_presentation()
            .unwrap()
            .capture_page_point(point.clone())
            .unwrap();

        let (after_state, _) = state
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: Vec::new(),
                },
            }))
            .expect("paragraph insert above must apply");
        let mut after_view = View::new_test();
        after_view.layout(&after_state);
        let after_cursor = after_view.cursor_metrics(&after_state, &position).unwrap();
        let resolved = expect_resolved(
            after_view
                .capture_viewport_anchor_presentation()
                .unwrap()
                .resolve(&anchor),
        );

        assert_eq!(
            resolved.point.y - point.y,
            after_cursor.line.y - before_cursor.line.y
        );
    }

    #[test]
    fn point_inside_resized_external_node_keeps_its_absolute_local_offset() {
        let mut projected = ProjectedState::empty();
        projected.commit();
        let image_node = match NodeType::Image.into_node() {
            Node::Image(node) => node,
            _ => unreachable!(),
        };
        let image = projected
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: image_node },
                    parents: vec![Dot::ROOT],
                },
            }))
            .expect("image insert must apply")
            .id;
        let state = State::new(projected, None);
        let mut view = View::new_test();
        view.layout(&state);
        assert!(view.set_external_height(&state, image, 1_000.0));
        let before = view
            .external_elements(&state, None)
            .into_iter()
            .find(|element| element.node == image)
            .expect("image must be laid out");
        let captured_point = super::ViewportAnchorPoint {
            page_idx: before.page_idx,
            x: before.bounds.x + before.bounds.width / 2.0,
            y: before.bounds.y + 150.0,
        };
        let anchor = view
            .capture_viewport_anchor_presentation()
            .unwrap()
            .capture_page_point(captured_point.clone())
            .expect("point inside the image must produce an anchor");

        assert!(view.set_external_height(&state, image, 1_400.0));
        let resolved = expect_resolved(
            view.capture_viewport_anchor_presentation()
                .unwrap()
                .resolve(&anchor),
        );

        assert_eq!(resolved.point, captured_point);
        assert_eq!(resolved.rect, None);
    }

    #[test]
    fn external_selection_anchor_resolves_the_measured_endpoint_rect() {
        let mut projected = ProjectedState::empty();
        projected.commit();
        let image_node = match NodeType::Image.into_node() {
            Node::Image(node) => node,
            _ => unreachable!(),
        };
        let image = projected
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: image_node },
                    parents: vec![Dot::ROOT],
                },
            }))
            .expect("image insert must apply")
            .id;
        let selection = Selection::new(
            Position {
                node: Dot::ROOT,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: Dot::ROOT,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        let state = State::new(projected, Some(selection));
        let mut view = View::new_test();
        view.layout(&state);
        let presentation = view.capture_viewport_anchor_presentation().unwrap();
        let anchor = presentation.capture_selection_head().unwrap();
        let provisional = expect_resolved(presentation.resolve(&anchor));

        assert!(view.set_external_height(&state, image, 400.0));
        let measured = expect_resolved(
            view.capture_viewport_anchor_presentation()
                .unwrap()
                .resolve(&anchor),
        );

        assert!(
            measured.rect.as_ref().unwrap().rect.height
                > provisional.rect.as_ref().unwrap().rect.height
        );
        assert!(measured.point.y > provisional.point.y);
    }

    #[test]
    fn deleted_anchor_identity_is_distinct_from_unavailable_geometry() {
        let mut projected = ProjectedState::empty();
        projected.commit();
        let paragraph = projected
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .id();
        let position = Position::new(paragraph, 0);
        let state = State::new(projected, Some(Selection::collapsed(position)));
        let mut view = View::new_test();
        view.layout(&state);
        let anchor = view
            .capture_viewport_anchor_presentation()
            .unwrap()
            .capture_selection_head()
            .unwrap();

        let (deleted, _) = state
            .apply(EditOp::Seq(ListOp::Del { pos: 0, len: 1 }))
            .expect("paragraph delete must apply");
        view.layout(&deleted);

        assert_eq!(
            view.capture_viewport_anchor_presentation()
                .unwrap()
                .resolve(&anchor),
            super::ViewportAnchorResolution::Deleted,
        );
    }
}
