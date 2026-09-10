use editor_common::Rect;
use editor_crdt::Dot;
use editor_state::Position;

use crate::measure::nodes::line_geometry::first_line_info;
use crate::measure::text::measure::MeasuredLine;
use crate::measure::text::ruby::RUBY_GAP;
use crate::measure::types::{MeasuredBox, MeasuredContent, MeasuredNode};
use crate::style::{BorderMode, BoxStyle, Direction};

use super::types::{
    ChildAttachment, LayoutAtom, LayoutBox, LayoutContent, LayoutLine, LayoutNode, SpacingKind,
};

pub(super) fn line_y(line: &MeasuredLine, y: f32, previous_content_bottom: f32) -> f32 {
    let ruby_top = line
        .ruby_annotations
        .iter()
        .map(|ruby| ruby.baseline_y - ruby.ascent)
        .reduce(f32::min);
    ruby_top.map_or(y, |top| y.max(previous_content_bottom + RUBY_GAP - top))
}

pub(crate) fn box_content_top(style: &BoxStyle, scope: bool, y: f32) -> Option<f32> {
    (scope || style.monolithic || style.padding.top > 0.0 || style.border.top > 0.0)
        .then_some(y + style.border.top)
}

/// The last painted boundary matters even when a line's leading extends below it.
/// Transparent paragraph boxes and spacing do not consume that leading.
pub(crate) fn content_bottom(node: &LayoutNode) -> Option<f32> {
    match &node.content {
        LayoutContent::Line(line) => {
            (!line.is_phantom).then_some(node.rect.y + line.baseline + line.descent)
        }
        LayoutContent::Atom(_) => Some(node.rect.bottom()),
        LayoutContent::Box(b) => {
            let bottom = b
                .children
                .iter()
                .filter_map(content_bottom)
                .reduce(f32::max);
            if box_content_top(&b.style, b.scope, node.rect.y).is_some()
                || b.style.padding.bottom > 0.0
                || b.style.border.bottom > 0.0
            {
                Some(bottom.unwrap_or(node.rect.bottom()).max(node.rect.bottom()))
            } else {
                bottom
            }
        }
        LayoutContent::Spacing(_) => None,
    }
}

pub(super) fn positioned_style(b: &MeasuredBox, children: &[LayoutNode], y: f32) -> BoxStyle {
    fn first_line_y(node: &LayoutNode) -> Option<f32> {
        match &node.content {
            LayoutContent::Line(_) => Some(node.rect.y),
            LayoutContent::Box(b) => b.children.iter().find_map(first_line_y),
            _ => None,
        }
    }
    let mut style = b.style.clone();
    if style.decorations.is_empty() {
        return style;
    }
    let mut top = y + leading_chrome_height(b);
    let measured_top = b.children.iter().find_map(|child| {
        let found = first_line_info(child).map(|line| top + line.top);
        top += child.height;
        found
    });
    if let Some((measured_top, placed_top)) =
        measured_top.zip(children.iter().find_map(first_line_y))
    {
        for decoration in &mut style.decorations {
            decoration.rect.y += placed_top - measured_top;
        }
    }
    style
}

/// Place an indivisible subtree with the same ruby clearance as paginated lines.
pub(super) fn place_node_at(
    node: &MeasuredNode,
    x: f32,
    y: f32,
    parent: Dot,
    child_index: usize,
    previous_content_bottom: &mut f32,
) -> LayoutNode {
    let placed = match &node.content {
        MeasuredContent::Box(b) => {
            if let Some(top) = box_content_top(&b.style, b.scope, y) {
                *previous_content_bottom = previous_content_bottom.max(top);
            }
            let collapse = b.style.border_mode == BorderMode::Collapse;
            let mut child_y = y + leading_chrome_height(b);
            let mut child_x =
                x + b.style.padding.left + if collapse { 0.0 } else { b.style.border.left };
            let start_y = child_y;
            let start_bottom = *previous_content_bottom;
            let mut bottom = child_y;
            let mut previous_border_bottom: f32 = 0.0;
            let mut idx = 0;
            let mut children = Vec::new();
            for child in &b.children {
                if b.style.direction == Direction::Horizontal {
                    child_y = start_y;
                    *previous_content_bottom = start_bottom;
                } else if collapse {
                    child_y -= previous_border_bottom.min(child_border_top(child));
                }
                let placed = place_node_at(
                    child,
                    child_x,
                    child_y,
                    b.node,
                    idx,
                    previous_content_bottom,
                );
                if matches!(
                    child.content,
                    MeasuredContent::Box(_) | MeasuredContent::Atom(_) | MeasuredContent::PageBreak
                ) {
                    idx += 1;
                }
                if b.style.direction == Direction::Horizontal {
                    child_x += child.width;
                    if collapse && let MeasuredContent::Box(child_box) = &child.content {
                        child_x -= child_box.style.border.right;
                    }
                } else {
                    child_y = placed.rect.bottom();
                }
                bottom = bottom.max(placed.rect.bottom());
                previous_border_bottom = child_border_bottom(child).unwrap_or(0.0);
                children.push(placed);
            }
            let height = node.height.max(bottom - y + trailing_chrome_height(b));
            if b.style.direction == Direction::Horizontal {
                for child in &mut children {
                    child.rect.height = bottom - start_y;
                }
            }
            let style = positioned_style(b, &children, y);
            LayoutNode {
                rect: Rect::from_xywh(x, y, node.width, height),
                content: LayoutContent::Box(LayoutBox {
                    node: b.node,
                    style,
                    children: children.into(),
                    attachment: Some(ChildAttachment {
                        parent,
                        index: child_index,
                    }),
                    scope: b.scope,
                }),
            }
        }
        MeasuredContent::Line(line) => LayoutNode {
            rect: Rect::from_xywh(
                x,
                line_y(line, y, *previous_content_bottom),
                node.width,
                node.height,
            ),
            content: LayoutContent::Line(LayoutLine {
                measured: std::sync::Arc::clone(line),
            }),
        },
        MeasuredContent::Atom(a) => LayoutNode {
            rect: Rect::from_xywh(x, y, node.width, node.height),
            content: LayoutContent::Atom(LayoutAtom {
                node: a.node,
                attachment: ChildAttachment {
                    parent,
                    index: child_index,
                },
            }),
        },
        MeasuredContent::Spacing(height) => LayoutNode {
            rect: Rect::from_xywh(x, y, node.width, *height),
            content: LayoutContent::Spacing(SpacingKind::Gap {
                position: Position::new(parent, child_index),
            }),
        },
        MeasuredContent::PageBreak => LayoutNode {
            rect: Rect::from_xywh(x, y, 0.0, 0.0),
            content: LayoutContent::Spacing(SpacingKind::Gap {
                position: Position::new(parent, child_index),
            }),
        },
    };
    if let Some(bottom) = content_bottom(&placed) {
        *previous_content_bottom = previous_content_bottom.max(bottom);
    }
    placed
}

pub(super) fn child_border_top(node: &MeasuredNode) -> f32 {
    match &node.content {
        MeasuredContent::Box(b) => b.style.border.top,
        _ => 0.0,
    }
}

pub(super) fn child_border_bottom(node: &MeasuredNode) -> Option<f32> {
    match &node.content {
        MeasuredContent::Box(b) => Some(b.style.border.bottom),
        _ => None,
    }
}

pub(super) fn leading_chrome_height(b: &MeasuredBox) -> f32 {
    let border_top = if b.style.border_mode == BorderMode::Collapse {
        0.0
    } else {
        b.style.border.top
    };
    border_top + b.style.padding.top
}

pub(super) fn trailing_chrome_height(b: &MeasuredBox) -> f32 {
    let border_bottom = if b.style.border_mode == BorderMode::Collapse {
        0.0
    } else {
        b.style.border.bottom
    };
    b.style.padding.bottom + border_bottom
}
