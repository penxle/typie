use std::sync::Arc;

use crate::measure::{MeasuredBox, MeasuredContent, MeasuredLine, MeasuredNode};

pub(super) struct FirstLineInfo {
    pub top: f32,
    pub height: f32,
}

pub(super) fn first_line_info(node: &MeasuredNode) -> Option<FirstLineInfo> {
    match &node.content {
        MeasuredContent::Line(_) => Some(FirstLineInfo {
            top: 0.0,
            height: node.height,
        }),
        MeasuredContent::Box(b) => {
            let mut y = b.style.padding.top + b.style.border.top;
            for child in &b.children {
                if let Some(mut info) = first_line_info(child) {
                    info.top += y;
                    return Some(info);
                }
                y += child.height;
            }
            None
        }
        _ => None,
    }
}

pub(super) struct LineStrutExpansion {
    pub ascent: f32,
    pub descent: f32,
    pub min_line_height: f32,
}

pub(super) struct ExpandedFirstLine {
    pub tree: MeasuredNode,
    pub top: f32,
    pub height: f32,
    pub baseline: f32,
}

// Path-only deep clone: ancestor boxes are rebuilt, other branches share Arc.
pub(super) fn expand_first_line(
    node: &MeasuredNode,
    expansion: &LineStrutExpansion,
) -> Option<ExpandedFirstLine> {
    match &node.content {
        MeasuredContent::Line(l) => {
            let new_line = expand_line(l, node.width, node.height, expansion);
            let height = new_line.height;
            let baseline = match &new_line.content {
                MeasuredContent::Line(line) => line.baseline,
                _ => unreachable!(),
            };
            Some(ExpandedFirstLine {
                tree: new_line,
                top: 0.0,
                height,
                baseline,
            })
        }
        MeasuredContent::Box(b) => {
            let mut running_y = b.style.padding.top + b.style.border.top;
            for (i, child) in b.children.iter().enumerate() {
                if let Some(expanded) = expand_first_line(child, expansion) {
                    let mut new_children: Vec<Arc<MeasuredNode>> =
                        Vec::with_capacity(b.children.len());
                    new_children.extend(b.children[..i].iter().cloned());
                    new_children.push(Arc::new(expanded.tree));
                    new_children.extend(b.children[i + 1..].iter().cloned());

                    let delta = expanded.height - child.height;
                    return Some(ExpandedFirstLine {
                        tree: MeasuredNode {
                            width: node.width,
                            height: node.height + delta,
                            content: MeasuredContent::Box(MeasuredBox {
                                node_id: b.node_id,
                                style: b.style.clone(),
                                table_info: b.table_info.clone(),
                                children: new_children,
                            }),
                        },
                        top: running_y + expanded.top,
                        height: expanded.height,
                        baseline: expanded.baseline,
                    });
                }
                running_y += child.height;
            }
            None
        }
        _ => None,
    }
}

fn expand_line(
    line: &MeasuredLine,
    width: f32,
    old_height: f32,
    expansion: &LineStrutExpansion,
) -> MeasuredNode {
    let new_ascent = line.ascent.max(expansion.ascent);
    let new_descent = line.descent.max(expansion.descent);
    let content_height = new_ascent + new_descent;
    let new_height = old_height
        .max(expansion.min_line_height)
        .max(content_height);
    let leading = (new_height - content_height).max(0.0);
    let new_baseline = leading / 2.0 + new_ascent;
    let delta = new_baseline - line.baseline;

    let mut glyph_runs = line.glyph_runs.clone();
    if delta != 0.0 {
        for run in &mut glyph_runs {
            for g in &mut run.glyphs {
                g.y += delta;
            }
        }
    }

    let mut ruby_annotations = line.ruby_annotations.clone();
    if delta != 0.0 {
        for ann in &mut ruby_annotations {
            ann.baseline_y += delta;
            for g in &mut ann.glyphs {
                g.y += delta;
            }
        }
    }

    MeasuredNode {
        width,
        height: new_height,
        content: MeasuredContent::Line(MeasuredLine {
            node_id: line.node_id,
            baseline: new_baseline,
            ascent: new_ascent,
            descent: new_descent,
            cursor_ascent: line.cursor_ascent,
            cursor_descent: line.cursor_descent,
            glyph_runs,
            ruby_annotations,
            empty_caret_x: line.empty_caret_x,
            child_range: line.child_range.clone(),
        }),
    }
}
