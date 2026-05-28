use std::sync::Arc;

use editor_common::EdgeInsets;

use crate::style::Alignment;
use editor_model::{Doc, Modifier, ModifierType, Node, NodeRef};

use crate::measure::Measurer;
use crate::measure::resolve::resolve_inherited;
use crate::measure::text::measure::build_strut_only_line;
use crate::measure::text::resolve::resolve_text_style;
use crate::measure::{MeasuredBox, MeasuredContent, MeasuredNode};
use crate::style::{BorderMode, BoxStyle, Direction};
use crate::view_state::ViewState;

const BLOCK_GAP_BASE_PX: f32 = 16.0;

pub fn resolve_gap_after(node: &NodeRef<'_>) -> f32 {
    match resolve_inherited(node, ModifierType::BlockGap) {
        Some(Modifier::BlockGap { value }) => *value as f32 / 100.0 * BLOCK_GAP_BASE_PX,
        _ => 0.0,
    }
}

const PHANTOM_INDENT_BASE_PX: f32 = 16.0;

/// ParagraphIndent applies to a paragraph only when its parent is Root
/// (`resolve_paragraph_indent`). The phantom's parent is `node`, so it
/// applies iff `node` is Root.
fn phantom_indent(node: &NodeRef<'_>) -> f32 {
    if !matches!(node.node(), Node::Root(_)) {
        return 0.0;
    }
    match resolve_inherited(node, ModifierType::ParagraphIndent) {
        Some(Modifier::ParagraphIndent { value }) => *value as f32 / 100.0 * PHANTOM_INDENT_BASE_PX,
        _ => 0.0,
    }
}

/// `(measured_phantom_line, gap_after_px)`. The phantom carries no
/// modifiers of its own, so its inherited font style and trailing
/// BlockGap equal what a real `paragraph {}` materialized at this slot
/// would resolve — guaranteeing jump-free transition. Keyed by the
/// container so the gap position `(node, index)` resolves to it via the
/// container-anchored cursor path (`collect_lines`).
fn make_gap_phantom_block(
    measurer: &mut Measurer,
    node: &NodeRef<'_>,
    width: f32,
    index: usize,
) -> (Arc<MeasuredNode>, f32) {
    let base_style = resolve_text_style(node);
    let indent = phantom_indent(node);
    let line = build_strut_only_line(
        measurer,
        node.id(),
        &base_style,
        width,
        editor_model::Alignment::Left,
        indent,
        index..index,
    );
    (line, resolve_gap_after(node))
}

pub fn layout_vertical(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> (Vec<Arc<MeasuredNode>>, f32) {
    let children_refs: Vec<_> = node.children().collect();
    let phantom_index: Option<usize> = view_state
        .gap_phantom
        .filter(|gp| gp.parent == node.id())
        .map(|gp| gp.index);

    let mut blocks: Vec<(Arc<MeasuredNode>, f32)> = Vec::with_capacity(children_refs.len() + 1);
    for (i, child) in children_refs.iter().enumerate() {
        if phantom_index == Some(i) {
            blocks.push(make_gap_phantom_block(measurer, node, width, i));
        }
        let m = measurer.measure(doc, child.id(), width, view_state);
        let child_node = doc.node(child.id()).unwrap();
        blocks.push((m, resolve_gap_after(&child_node)));
    }
    if phantom_index == Some(children_refs.len()) {
        blocks.push(make_gap_phantom_block(
            measurer,
            node,
            width,
            children_refs.len(),
        ));
    }

    let mut result = Vec::with_capacity(blocks.len() * 2);
    let mut total_height = 0.0;
    let n = blocks.len();
    for (idx, (mnode, gap_after)) in blocks.into_iter().enumerate() {
        total_height += mnode.height;
        result.push(mnode);
        if idx + 1 < n && gap_after > 0.0 {
            result.push(Arc::new(MeasuredNode {
                width,
                height: gap_after,
                content: MeasuredContent::Spacing(gap_after),
            }));
            total_height += gap_after;
        }
    }

    (result, total_height)
}

pub struct PaddedLayoutConfig {
    pub padding: EdgeInsets,
    pub border: EdgeInsets,
    pub scope: bool,
    pub alignment: Alignment,
}

pub fn layout_padded(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
    config: PaddedLayoutConfig,
) -> MeasuredNode {
    let PaddedLayoutConfig {
        padding,
        border,
        scope,
        alignment,
    } = config;
    let inner_width = width - padding.left - padding.right - border.left - border.right;
    let (children, children_height) = layout_vertical(measurer, doc, node, inner_width, view_state);
    let total_height = children_height + padding.top + padding.bottom + border.top + border.bottom;

    MeasuredNode {
        width,
        height: total_height,
        content: MeasuredContent::Box(MeasuredBox {
            node_id: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding,
                border,
                border_mode: BorderMode::Separate,
                alignment,
                scope,
                decorations: vec![],
                monolithic: node.spec().monolithic,
            },
            table_info: None,
            children,
        }),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::NodeId;

    use super::*;

    #[test]
    fn sums_children() {
        let (doc, p1) = doc! {
            root {
                p1: paragraph { text("hello") }
            }
        };

        let node = doc.node(p1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = layout_padded(
            &mut measurer,
            &doc,
            &node,
            300.0,
            &ViewState::new(),
            PaddedLayoutConfig {
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::ZERO,
                scope: false,
                alignment: Alignment::Start,
            },
        );

        assert!(matches!(result.content, MeasuredContent::Box(_)));
        assert_eq!(result.width, 300.0);
    }

    #[test]
    fn inserts_gap_as_spacing() {
        let (doc,) = doc! {
            root [block_gap(200)] {
                paragraph { text("a") }
                paragraph { text("b") }
            }
        };

        let node = doc.node(NodeId::ROOT).unwrap();
        let mut measurer = Measurer::new_test();
        let (children, _) = layout_vertical(&mut measurer, &doc, &node, 300.0, &ViewState::new());

        assert_eq!(children.len(), 3);
        assert!(matches!(children[1].content, MeasuredContent::Spacing(_)));
    }

    #[test]
    fn resolve_gap_after_converts_block_gap() {
        let (doc, p1) = doc! { root [block_gap(100)] { p1: paragraph } };
        let node = doc.node(p1).unwrap();
        assert_eq!(resolve_gap_after(&node), 16.0);
    }

    #[test]
    fn resolve_gap_after_returns_zero_when_no_block_gap() {
        let (doc, p1) = doc! { root [] { p1: paragraph } };
        let node = doc.node(p1).unwrap();
        assert_eq!(resolve_gap_after(&node), 0.0);
    }

    fn count_lines_with(
        children: &[std::sync::Arc<MeasuredNode>],
        id: NodeId,
        range: std::ops::Range<usize>,
    ) -> usize {
        children
            .iter()
            .filter(|n| {
                matches!(&n.content,
            crate::measure::MeasuredContent::Line(l)
                if l.node_id == id && l.child_range == Some(range.clone()))
            })
            .count()
    }

    #[test]
    fn gap_phantom_matches_real_paragraph_layout_jump_free() {
        use crate::measure::Measurer;
        use crate::view_state::{GapPhantom, ViewState};
        let (doc_gap,) = doc! {
            root {
                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                paragraph {}
            }
        };
        let (doc_real,) = doc! {
            root {
                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                paragraph {}
                fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                paragraph {}
            }
        };
        let mut m = Measurer::new_test();
        let mut vs = ViewState::new();
        vs.gap_phantom = Some(GapPhantom {
            parent: NodeId::ROOT,
            index: 1,
        });
        let (cg, hg) = layout_vertical(
            &mut m,
            &doc_gap,
            &doc_gap.node(NodeId::ROOT).unwrap(),
            300.0,
            &vs,
        );
        let (cr, hr) = layout_vertical(
            &mut m,
            &doc_real,
            &doc_real.node(NodeId::ROOT).unwrap(),
            300.0,
            &ViewState::new(),
        );
        assert!(
            (hg - hr).abs() < 0.5,
            "phantom height {hg} must match real-paragraph height {hr}"
        );
        assert_eq!(count_lines_with(&cg, NodeId::ROOT, 1..1), 1);
        let sp = |c: &[std::sync::Arc<MeasuredNode>]| {
            c.iter()
                .filter(|n| matches!(n.content, crate::measure::MeasuredContent::Spacing(_)))
                .count()
        };
        assert_eq!(
            sp(&cg),
            sp(&cr),
            "gap spacing count must match the real-paragraph case"
        );
    }

    #[test]
    fn gap_phantom_index_zero_leading() {
        use crate::measure::Measurer;
        use crate::view_state::{GapPhantom, ViewState};
        let (doc,) = doc! { root { image paragraph { text("b") } } };
        let mut m = Measurer::new_test();
        let mut vs = ViewState::new();
        vs.gap_phantom = Some(GapPhantom {
            parent: NodeId::ROOT,
            index: 0,
        });
        let (c, h0) = layout_vertical(&mut m, &doc, &doc.node(NodeId::ROOT).unwrap(), 300.0, &vs);
        let (_c2, h1) = layout_vertical(
            &mut m,
            &doc,
            &doc.node(NodeId::ROOT).unwrap(),
            300.0,
            &ViewState::new(),
        );
        assert!(h0 > h1, "leading phantom must add height");
        assert_eq!(count_lines_with(&c, NodeId::ROOT, 0..0), 1);
    }

    #[test]
    fn no_phantom_when_parent_mismatch_or_none() {
        use crate::measure::Measurer;
        use crate::view_state::{GapPhantom, ViewState};
        let (doc, p1) = doc! { root { p1: paragraph { text("hello") } } };
        let mut m = Measurer::new_test();
        let node = doc.node(p1).unwrap();
        let base = layout_vertical(&mut m, &doc, &node, 300.0, &ViewState::new()).1;
        let mut vs = ViewState::new();
        vs.gap_phantom = Some(GapPhantom {
            parent: NodeId::new(),
            index: 0,
        });
        let mismatch = layout_vertical(&mut m, &doc, &node, 300.0, &vs).1;
        assert_eq!(
            base, mismatch,
            "mismatched parent descriptor must have no effect"
        );
    }
}
