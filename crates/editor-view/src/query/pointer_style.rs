use editor_macros::ffi;
use editor_model::{Doc, Node};
use serde::{Deserialize, Serialize};

use super::interactive::{InteractiveHit, interactive_hit_test};
use super::layout_index::{LayoutEntry, LayoutIndex};
use crate::paginate::{LayoutContent, LayoutNode};

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerStyle {
    Default,
    Text,
    Pointer,
}

pub(crate) fn pointer_style_at(
    layout_index: &LayoutIndex,
    page_idx: usize,
    doc: &Doc,
    x: f32,
    page_y: f32,
    read_only: bool,
) -> PointerStyle {
    if let Some(hit) = interactive_hit_test(layout_index, doc, page_idx, x, page_y) {
        return match hit {
            InteractiveHit::CalloutIcon { .. } => PointerStyle::Pointer,
            InteractiveHit::FoldTitle { text_rect, .. } => {
                if read_only {
                    PointerStyle::Pointer
                } else if text_rect.is_some_and(|r| r.contains(x, page_y)) {
                    PointerStyle::Text
                } else {
                    PointerStyle::Pointer
                }
            }
        };
    }

    let Some(point) = layout_index.point(page_idx, x, page_y) else {
        return PointerStyle::Text;
    };
    if let Some(hit) = layout_index.exact_entry(point, is_pointer_entry) {
        return match hit.content(layout_index) {
            Some(LayoutContent::Line(_)) => PointerStyle::Text,
            Some(LayoutContent::Atom(atom)) => atom_pointer_style(doc, atom.node_id),
            Some(LayoutContent::Box(b)) => doc
                .node(b.node_id)
                .map(|node| box_pointer_style(node.node()))
                .unwrap_or(PointerStyle::Text),
            Some(LayoutContent::Spacing(_)) | None => PointerStyle::Text,
        };
    }

    PointerStyle::Text
}

fn is_pointer_entry(_entry: &LayoutEntry, node: &LayoutNode) -> bool {
    matches!(
        node.content,
        LayoutContent::Line(_) | LayoutContent::Atom(_) | LayoutContent::Box(_)
    )
}

fn atom_pointer_style(doc: &Doc, id: editor_model::NodeId) -> PointerStyle {
    doc.node(id)
        .map(|node| match node.node() {
            Node::Image(_)
            | Node::File(_)
            | Node::Embed(_)
            | Node::Archived(_)
            | Node::HorizontalRule(_)
            | Node::PageBreak(_) => PointerStyle::Default,
            _ => PointerStyle::Text,
        })
        .unwrap_or(PointerStyle::Text)
}

fn box_pointer_style(node: &Node) -> PointerStyle {
    match node {
        Node::FoldTitle(_) => PointerStyle::Pointer,
        Node::FoldContent(_) => PointerStyle::Default,
        Node::Image(_)
        | Node::File(_)
        | Node::Embed(_)
        | Node::Archived(_)
        | Node::HorizontalRule(_)
        | Node::PageBreak(_) => PointerStyle::Default,
        _ => PointerStyle::Text,
    }
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect, Size};
    use editor_macros::doc;
    use editor_model::NodeId;

    use crate::page::LayoutPage;
    use crate::style::*;

    use super::*;
    use crate::paginate::*;

    fn page(y_start: f32) -> LayoutPage {
        LayoutPage::new(y_start, y_start + 1000.0, Size::new(800.0, 1000.0))
    }

    fn style_at(tree: &LayoutTree, doc: &Doc, x: f32, y: f32, read_only: bool) -> PointerStyle {
        let pages = [page(0.0)];
        let layout_index = LayoutIndex::new(tree.clone(), &pages);
        pointer_style_at(&layout_index, 0, doc, x, y, read_only)
    }

    fn line_node(id: NodeId, x: f32, y: f32, w: f32, h: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(x, y, w, h),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: h,
                ascent: h,
                descent: 0.0,
                cursor_ascent: h,
                cursor_descent: 0.0,
                glyph_runs: vec![],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    fn atom_node(id: NodeId, parent_id: NodeId, x: f32, y: f32, w: f32, h: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(x, y, w, h),
            content: LayoutContent::Atom(LayoutAtom {
                node_id: id,
                parent_id,
                index: 0,
            }),
        }
    }

    fn box_node(
        id: NodeId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        decorations: Vec<Decoration>,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(x, y, w, h),
            content: LayoutContent::Box(LayoutBox {
                node_id: id,
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    decorations,
                    monolithic: false,
                    ..Default::default()
                },
                nav: None,
                children,
            }),
        }
    }

    #[test]
    fn line_returns_text_cursor() {
        let (doc, p1) = doc! { root { p1: paragraph { text("Hello") } } };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                80.0,
                vec![],
                vec![box_node(
                    p1,
                    0.0,
                    0.0,
                    200.0,
                    40.0,
                    vec![],
                    vec![line_node(p1, 20.0, 8.0, 80.0, 20.0)],
                )],
            ),
        };

        assert_eq!(style_at(&tree, &doc, 40.0, 16.0, false), PointerStyle::Text);
    }

    #[test]
    fn atom_returns_default_cursor() {
        let (doc, hr1) = doc! { root { hr1: horizontal_rule } };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                80.0,
                vec![],
                vec![atom_node(hr1, NodeId::ROOT, 0.0, 8.0, 200.0, 24.0)],
            ),
        };

        assert_eq!(
            style_at(&tree, &doc, 40.0, 16.0, false),
            PointerStyle::Default
        );
    }

    #[test]
    fn callout_icon_returns_pointer_cursor() {
        let (doc, c1) = doc! { root { c1: callout { paragraph { text("x") } } } };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                80.0,
                vec![],
                vec![box_node(
                    c1,
                    0.0,
                    0.0,
                    200.0,
                    40.0,
                    vec![Decoration {
                        id: 0,
                        rect: Rect::from_xywh(12.0, 10.0, 20.0, 20.0),
                        data: DecorationData::None,
                    }],
                    vec![line_node(NodeId::new(), 40.0, 8.0, 20.0, 20.0)],
                )],
            ),
        };

        assert_eq!(
            style_at(&tree, &doc, 20.0, 18.0, false),
            PointerStyle::Pointer
        );
    }

    #[test]
    fn fold_title_text_passes_through_to_text_cursor_in_edit_mode() {
        let (doc, f1, ft1) = doc! {
            root {
                f1: fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            }
        };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                80.0,
                vec![],
                vec![box_node(
                    f1,
                    0.0,
                    0.0,
                    200.0,
                    40.0,
                    vec![],
                    vec![box_node(
                        ft1,
                        0.0,
                        0.0,
                        200.0,
                        40.0,
                        vec![],
                        vec![line_node(ft1, 40.0, 8.0, 80.0, 20.0)],
                    )],
                )],
            ),
        };

        assert_eq!(style_at(&tree, &doc, 48.0, 16.0, false), PointerStyle::Text);
        assert_eq!(
            style_at(&tree, &doc, 8.0, 16.0, false),
            PointerStyle::Pointer
        );
        assert_eq!(
            style_at(&tree, &doc, 48.0, 16.0, true),
            PointerStyle::Pointer
        );
    }
}
