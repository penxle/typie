use std::sync::Arc;

use crate::measure::text::measure::{LineStrutExpansion, MeasuredLine};
use crate::measure::types::{MeasuredBox, MeasuredContent, MeasuredNode};

pub(crate) struct FirstLineInfo {
    pub top: f32,
    pub height: f32,
}

pub(crate) fn first_line_info(node: &MeasuredNode) -> Option<FirstLineInfo> {
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

pub(crate) struct ExpandedFirstLine {
    pub tree: MeasuredNode,
    pub top: f32,
    pub height: f32,
    pub baseline: f32,
}

pub(crate) fn expand_first_line(
    node: &MeasuredNode,
    expansion: &LineStrutExpansion,
) -> Option<ExpandedFirstLine> {
    match &node.content {
        MeasuredContent::Line(l) => {
            let new_node = expand_line(l, node.width, node.height, expansion);
            let height = new_node.height;
            let baseline = match &new_node.content {
                MeasuredContent::Line(line) => line.baseline,
                _ => unreachable!(),
            };
            Some(ExpandedFirstLine {
                tree: new_node,
                top: 0.0,
                height,
                baseline,
            })
        }
        MeasuredContent::Box(b) => {
            let mut running_y = b.style.padding.top + b.style.border.top;
            for (i, child) in b.children.iter().enumerate() {
                if let Some(expanded) = expand_first_line(child, expansion) {
                    let delta = expanded.tree.height - child.height;
                    let mut new_children = b.children.clone();
                    new_children.set(i, Arc::new(expanded.tree));
                    return Some(ExpandedFirstLine {
                        tree: MeasuredNode {
                            width: node.width,
                            height: node.height + delta,
                            content: MeasuredContent::Box(MeasuredBox {
                                node: b.node,
                                style: b.style.clone(),
                                children: new_children,
                                page_break_policy: b.page_break_policy,
                                scope: b.scope,
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
    if line.is_phantom {
        // Phantom wrappers stay zero-height; `from_line` would copy the line's
        // intrinsic height into the layout tree.
        return MeasuredNode {
            width,
            height: 0.0,
            content: MeasuredContent::Line(std::sync::Arc::new(line.clone())),
        };
    }

    let new_ascent = line.ascent.max(expansion.ascent);
    let new_descent = line.descent.max(expansion.descent);
    let new_height = old_height.max(expansion.min_line_height);
    MeasuredNode::from_line(
        width,
        line.with_vertical_metrics(new_ascent, new_descent, new_height),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use editor_crdt::Dot;

    use super::*;
    use crate::measure::PageBreakPolicy;
    use crate::measure::text::measure::MeasuredLine;
    use crate::measure::types::{MeasuredBox, MeasuredChildren, MeasuredContent, MeasuredNode};
    use crate::style::BoxStyle;

    fn make_line(n: u64, height: f32, ascent: f32, descent: f32, is_phantom: bool) -> MeasuredLine {
        let baseline = ascent;
        MeasuredLine {
            node: Dot::new(1, n),
            height,
            baseline,
            ascent,
            descent,
            cursor_ascent: ascent,
            cursor_descent: descent,
            glyph_runs: vec![],
            ruby_annotations: vec![],
            empty_caret_x: 0.0,
            offset_range: None,
            tab_gaps: vec![],
            is_phantom,
            content_edge_x: None,
        }
    }

    fn box_with_line(line: MeasuredLine, padding_top: f32, width: f32) -> MeasuredNode {
        let line_node = Arc::new(MeasuredNode::from_line(width, line));
        let line_height = line_node.height;
        let children = MeasuredChildren::from_blocks(vec![line_node]);
        let box_height = padding_top + line_height;
        let mut style = BoxStyle::default();
        style.padding.top = padding_top;
        MeasuredNode {
            width,
            height: box_height,
            content: MeasuredContent::Box(MeasuredBox {
                node: Dot::new(1, 99),
                style,
                children,
                page_break_policy: PageBreakPolicy::Auto,
                scope: false,
            }),
        }
    }

    #[test]
    fn first_line_info_finds_line_under_padding() {
        let line = make_line(1, 10.0, 8.0, 2.0, false);
        let node = box_with_line(line, 4.0, 100.0);
        let info = first_line_info(&node).unwrap();
        assert_eq!(info.top, 4.0);
        assert_eq!(info.height, 10.0);

        let spacing = MeasuredNode {
            width: 100.0,
            height: 5.0,
            content: MeasuredContent::Spacing(5.0),
        };
        let children = MeasuredChildren::from_blocks(vec![Arc::new(spacing)]);
        let only_spacing = MeasuredNode {
            width: 100.0,
            height: 5.0,
            content: MeasuredContent::Box(MeasuredBox {
                node: Dot::new(1, 98),
                style: BoxStyle::default(),
                children,
                page_break_policy: PageBreakPolicy::Auto,
                scope: false,
            }),
        };
        assert!(first_line_info(&only_spacing).is_none());
    }

    #[test]
    fn expand_first_line_grows_line_and_box_uniformly() {
        let line = make_line(2, 8.0, 5.0, 3.0, false);
        let node = box_with_line(line, 0.0, 100.0);
        let original_height = node.height;

        let expansion = LineStrutExpansion {
            ascent: 12.0,
            descent: 3.0,
            min_line_height: 15.0,
        };
        let result = expand_first_line(&node, &expansion).unwrap();

        assert!(result.height > 8.0, "expanded line height must grow");
        let delta = result.height - 8.0;
        assert!(delta > 0.0);
        assert_eq!(result.tree.height, original_height + delta);

        let line_wrapper = match &result.tree.content {
            MeasuredContent::Box(b) => b.children.get(0).unwrap().clone(),
            _ => panic!("expected box"),
        };
        let inner_line_height = match &line_wrapper.content {
            MeasuredContent::Line(l) => l.height,
            _ => panic!("expected line"),
        };
        assert_eq!(
            line_wrapper.height, inner_line_height,
            "H1: wrapper height must equal line.height"
        );
    }

    #[test]
    fn expand_first_line_nested_box_keeps_following_lines() {
        let line1 = make_line(4, 10.0, 8.0, 2.0, false);
        let line2 = make_line(5, 10.0, 8.0, 2.0, false);
        let inner_children = MeasuredChildren::from_blocks(vec![
            Arc::new(MeasuredNode::from_line(100.0, line1)),
            Arc::new(MeasuredNode::from_line(100.0, line2)),
        ]);
        let inner = MeasuredNode {
            width: 100.0,
            height: 20.0,
            content: MeasuredContent::Box(MeasuredBox {
                node: Dot::new(1, 97),
                style: BoxStyle::default(),
                children: inner_children,
                page_break_policy: PageBreakPolicy::Auto,
                scope: false,
            }),
        };
        let outer = MeasuredNode {
            width: 100.0,
            height: 20.0,
            content: MeasuredContent::Box(MeasuredBox {
                node: Dot::new(1, 96),
                style: BoxStyle::default(),
                children: MeasuredChildren::from_blocks(vec![Arc::new(inner)]),
                page_break_policy: PageBreakPolicy::Auto,
                scope: false,
            }),
        };

        let expansion = LineStrutExpansion {
            ascent: 12.0,
            descent: 3.0,
            min_line_height: 15.0,
        };
        let result = expand_first_line(&outer, &expansion).unwrap();

        let first_line_delta = result.height - 10.0;
        assert!(first_line_delta > 0.0, "first line must grow");
        assert_eq!(
            result.tree.height,
            20.0 + first_line_delta,
            "outer box must keep the second line's height and grow only by the first line's delta"
        );
        let MeasuredContent::Box(ref ob) = result.tree.content else {
            panic!("expected outer box");
        };
        assert_eq!(
            ob.children[0].height,
            20.0 + first_line_delta,
            "inner box must keep the second line's height"
        );
    }

    fn shrunken_line(height: f32, ascent: f32, descent: f32) -> MeasuredLine {
        let content = ascent + descent;
        MeasuredLine {
            node: Dot::new(1, 7),
            height,
            baseline: (height - content) / 2.0 + ascent,
            ascent,
            descent,
            cursor_ascent: ascent,
            cursor_descent: descent,
            glyph_runs: vec![],
            ruby_annotations: vec![],
            empty_caret_x: 0.0,
            offset_range: None,
            tab_gaps: vec![],
            is_phantom: false,
            content_edge_x: None,
        }
    }

    #[test]
    fn expand_does_not_reinflate_shrunken_line() {
        // line-height 80%: box(12.8) < content(16). A same-strut marker expansion
        // must be a no-op, not re-clamp the line back to its content height.
        let line = shrunken_line(12.8, 8.0, 8.0);
        let node = box_with_line(line, 0.0, 100.0);
        let expansion = LineStrutExpansion {
            ascent: 8.0,
            descent: 8.0,
            min_line_height: 12.8,
        };
        let result = expand_first_line(&node, &expansion).unwrap();
        assert!(
            (result.height - 12.8).abs() < 1e-3,
            "same-strut expansion must keep the shrunken box (got {})",
            result.height
        );
        assert!(
            (result.baseline - 6.4).abs() < 1e-3,
            "baseline must stay centered (got {})",
            result.baseline
        );
    }

    #[test]
    fn expand_grows_to_marker_line_box_not_content() {
        // Bigger marker (content 24, line-height 80% → box 19.2) on a small text
        // line: the line grows to the marker's line box, not its content height.
        let line = shrunken_line(12.8, 8.0, 8.0);
        let node = box_with_line(line, 0.0, 100.0);
        let expansion = LineStrutExpansion {
            ascent: 12.0,
            descent: 12.0,
            min_line_height: 19.2,
        };
        let result = expand_first_line(&node, &expansion).unwrap();
        assert!(
            (result.height - 19.2).abs() < 1e-3,
            "expansion must grow to min_line_height, not content height (got {})",
            result.height
        );
        assert!(
            (result.baseline - ((19.2 - 24.0) / 2.0 + 12.0)).abs() < 1e-3,
            "baseline must center the marker content in the line box (got {})",
            result.baseline
        );
    }

    #[test]
    fn expand_phantom_line_is_zero_height() {
        let line = make_line(3, 20.0, 16.0, 4.0, true);
        let node = box_with_line(line, 0.0, 100.0);

        let expansion = LineStrutExpansion {
            ascent: 20.0,
            descent: 5.0,
            min_line_height: 0.0,
        };
        let result = expand_first_line(&node, &expansion).unwrap();

        let line_wrapper = match &result.tree.content {
            MeasuredContent::Box(b) => b.children.get(0).unwrap().clone(),
            _ => panic!("expected box"),
        };
        assert_eq!(
            line_wrapper.height, 0.0,
            "phantom wrapper must be zero-height"
        );
    }
}
