use editor_common::Rect;
use editor_macros::ffi;
use editor_model::{Alignment, Doc, Modifier, ModifierType, Node, NodeRef};
use serde::{Deserialize, Serialize};

use super::layout_index::LayoutIndex;
use crate::measure::resolve::resolve_inherited;
use crate::measure::text::resolve::resolve_paragraph_indent;

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlaceholderMetrics {
    pub page_idx: usize,
    pub rect: Rect,
    pub font_size: Option<u32>,
    pub line_height: Option<u32>,
    pub letter_spacing: Option<i32>,
    pub align: Option<Alignment>,
}

pub(crate) fn placeholder_metrics(
    layout_index: &LayoutIndex,
    doc: &Doc,
) -> Option<PlaceholderMetrics> {
    if !is_single_empty_paragraph(doc) {
        return None;
    }
    let para = doc.root()?.first_child()?;
    let para_id = para.id();

    let entry = layout_index.box_entry(para_id)?;
    let page_rect = layout_index.page_rect(entry.rect)?;
    let indent = placeholder_indent(&para);
    let rect = Rect::from_xywh(
        page_rect.rect.x + indent,
        page_rect.rect.y,
        (page_rect.rect.width - indent).max(0.0),
        page_rect.rect.height,
    );

    Some(PlaceholderMetrics {
        page_idx: page_rect.page_idx,
        rect,
        font_size: resolve_u32(&para, ModifierType::FontSize),
        line_height: resolve_u32(&para, ModifierType::LineHeight),
        letter_spacing: resolve_i32(&para, ModifierType::LetterSpacing),
        align: resolve_align(&para),
    })
}

fn resolve_u32(node: &NodeRef<'_>, ty: ModifierType) -> Option<u32> {
    match resolve_inherited(node, ty) {
        Some(Modifier::FontSize { value }) | Some(Modifier::LineHeight { value }) => Some(*value),
        _ => None,
    }
}

fn resolve_i32(node: &NodeRef<'_>, ty: ModifierType) -> Option<i32> {
    match resolve_inherited(node, ty) {
        Some(Modifier::LetterSpacing { value }) => Some(*value),
        _ => None,
    }
}

fn placeholder_indent(node: &NodeRef<'_>) -> f32 {
    let align = match node.own_modifier(ModifierType::Alignment) {
        Some(Modifier::Alignment { value }) => *value,
        _ => Alignment::default(),
    };
    match align {
        Alignment::Left | Alignment::Justify => resolve_paragraph_indent(node),
        Alignment::Center | Alignment::Right => 0.0,
    }
}

fn resolve_align(node: &NodeRef<'_>) -> Option<Alignment> {
    match resolve_inherited(node, ModifierType::Alignment) {
        Some(Modifier::Alignment { value }) => Some(*value),
        _ => None,
    }
}

pub(crate) fn is_single_empty_paragraph(doc: &Doc) -> bool {
    let Some(root) = doc.root() else {
        return false;
    };
    let mut children = root.children();
    let Some(first) = children.next() else {
        return false;
    };
    if children.next().is_some() {
        return false;
    }
    if !first.spec().is_textblock() {
        return false;
    }
    first
        .children()
        .all(|c| matches!(c.node(), Node::Text(t) if t.text.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::layout_index::LayoutIndex;
    use crate::view::View;
    use editor_macros::doc;

    #[test]
    fn empty_paragraph_is_placeholder_doc() {
        let (doc, _p) = doc! { root { p1: paragraph } };
        assert!(is_single_empty_paragraph(&doc));
    }

    #[test]
    fn paragraph_with_empty_text_is_placeholder_doc() {
        let (doc, _t) = doc! { root { paragraph { t1: text("") } } };
        assert!(is_single_empty_paragraph(&doc));
    }

    #[test]
    fn paragraph_with_text_is_not_placeholder_doc() {
        let (doc, _t) = doc! { root { paragraph { t1: text("hi") } } };
        assert!(!is_single_empty_paragraph(&doc));
    }

    #[test]
    fn two_blocks_is_not_placeholder_doc() {
        let (doc, ..) = doc! { root { paragraph { t1: text("") } paragraph { t2: text("") } } };
        assert!(!is_single_empty_paragraph(&doc));
    }

    #[test]
    fn single_non_textblock_is_not_placeholder_doc() {
        let (doc, _t) = doc! { root { t1: horizontal_rule } };
        assert!(!is_single_empty_paragraph(&doc));
    }

    #[test]
    fn placeholder_metrics_some_for_empty_doc() {
        let (doc, _p) = doc! { root { p1: paragraph } };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        let layout_index = LayoutIndex::new(tree.clone(), pages);

        let m = placeholder_metrics(&layout_index, &doc).expect("empty doc has placeholder");
        assert_eq!(m.page_idx, 0);
        assert!(
            m.rect.width > 0.0,
            "placeholder rect must have content width"
        );
    }

    #[test]
    fn placeholder_metrics_respects_paragraph_indent() {
        use crate::query::cursor::cursor_metrics;
        use editor_state::Position;

        let (doc, p1) = doc! { root [paragraph_indent(200)] { p1: paragraph } };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        let layout_index = LayoutIndex::new(tree.clone(), pages);

        let m = placeholder_metrics(&layout_index, &doc).expect("empty doc placeholder");
        let pos = Position::new(p1, 0);
        let cursor = cursor_metrics(&layout_index, &pos, None).expect("empty paragraph caret");

        assert!(
            (m.rect.x - cursor.caret.x).abs() < 0.01,
            "placeholder x ({}) must coincide with the empty-paragraph caret x ({})",
            m.rect.x,
            cursor.caret.x,
        );
    }

    #[test]
    fn placeholder_metrics_none_for_nonempty_doc() {
        let (doc, _t) = doc! { root { paragraph { t1: text("hi") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        let layout_index = LayoutIndex::new(tree.clone(), pages);

        assert!(placeholder_metrics(&layout_index, &doc).is_none());
    }
}
