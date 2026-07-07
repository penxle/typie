use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CursorMetrics {
    pub page_idx: usize,
    pub caret: Rect,
    pub line: Rect,
}

use editor_common::Rect;
use editor_state::Position;

use super::grapheme;
use super::layout_index::LayoutIndex;
use crate::paginate::types::{LayoutContent, LayoutLine};

pub(crate) fn cursor_metrics(
    layout_index: &LayoutIndex,
    pos: &Position,
    metrics_override: Option<(f32, f32)>,
) -> Option<CursorMetrics> {
    let entry = layout_index.entry_for_position(pos)?;
    let page_rect = layout_index.page_rect(entry.rect)?;

    match entry.content(layout_index)? {
        LayoutContent::Line(l) => {
            let x = x_at_offset(l, pos);
            let (cursor_ascent, cursor_descent) = if let Some(ov) = metrics_override {
                ov
            } else {
                let run = if pos.offset > 0 {
                    l.glyph_runs
                        .iter()
                        .find(|r| r.offset_range.contains(&(pos.offset - 1)))
                } else {
                    None
                }
                .or_else(|| l.glyph_runs.first());
                match run {
                    Some(r) => (r.cursor_ascent, r.cursor_descent),
                    None => (l.cursor_ascent, l.cursor_descent),
                }
            };
            let cursor_height = cursor_ascent + cursor_descent;
            let caret = Rect::from_xywh(
                entry.rect.x + x,
                page_rect.rect.y + l.baseline - cursor_ascent,
                1.0,
                cursor_height,
            );
            Some(CursorMetrics {
                page_idx: page_rect.page_idx,
                caret,
                line: page_rect.rect,
            })
        }
        LayoutContent::Atom(_) => None,
        _ => None,
    }
}

pub(crate) fn x_at_offset(line: &LayoutLine, pos: &Position) -> f32 {
    grapheme::x_at_offset(line, pos)
}

#[cfg(test)]
mod tests {
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AtomLeaf, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType, SeqItem, SpanLog,
        project_document,
    };
    use editor_resource::Resource;
    use editor_state::Affinity;

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::paginate::paginator::Paginator;
    use crate::paginate::types::LayoutContent;
    use crate::query::grapheme;

    use super::*;

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
        }
    }

    fn build_index(doc: &DocLogs, width: f32) -> (editor_crdt::Dot, LayoutIndex) {
        let pd = project_document(doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let root_id = root_node.id();
        let mut res = Resource::new_test();
        let measured = measure_node(
            &mut crate::measure::Measurer::new(),
            &root_node,
            width,
            &MeasureContext::default(),
            &mut res,
        );
        let layout = Paginator::continuous(width, 100_000.0, EdgeInsets::all(0.0))
            .paginate(MeasuredTree { root: measured });
        let index = LayoutIndex::new(layout.tree, &layout.pages);
        (root_id, index)
    }

    fn para_doc(text: &str, width: f32) -> (editor_crdt::Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        )];
        for (i, ch) in text.chars().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
        }
        let doc = logs(&items);
        let para_id = para;
        let (_, index) = build_index(&doc, width);
        (para_id, index)
    }

    fn para_items_doc(children: Vec<SeqItem>, width: f32) -> (editor_crdt::Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let para = Dot::new(10, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        )];
        for (i, child) in children.into_iter().enumerate() {
            items.push((Dot::new(10, 2 + i as u64), child));
        }
        let doc = logs(&items);
        let para_id = para;
        let (_, index) = build_index(&doc, width);
        (para_id, index)
    }

    #[test]
    fn cursor_metrics_line_returns_some() {
        let (para_id, index) = para_doc("Hi", 400.0);
        let pos = Position {
            node: para_id,
            offset: 1,
            affinity: Affinity::default(),
        };

        let result = cursor_metrics(&index, &pos, None);
        assert!(
            result.is_some(),
            "cursor_metrics must return Some for a line position"
        );

        let cm = result.unwrap();

        let entry = index.entry_for_position(&pos).unwrap();
        let node = entry.node(&index).unwrap();
        let LayoutContent::Line(l) = &node.content else {
            panic!("entry must be a line");
        };

        let expected_x = entry.rect.x + grapheme::x_at_offset(l, &pos);
        assert_eq!(
            cm.caret.x, expected_x,
            "caret.x must equal entry.rect.x + x_at_offset"
        );
        assert_eq!(
            cm.caret.height,
            l.cursor_ascent + l.cursor_descent,
            "caret.height must equal cursor_ascent + cursor_descent"
        );
        assert_eq!(cm.caret.width, 1.0, "caret.width must be 1.0");
        assert_eq!(cm.page_idx, 0, "page_idx must be 0");
    }

    #[test]
    fn cursor_metrics_after_tab_only_line_returns_visible_caret() {
        let (para_id, index) = para_items_doc(
            vec![SeqItem::Atom(AtomLeaf::Tab), SeqItem::Atom(AtomLeaf::Tab)],
            400.0,
        );
        let pos = Position {
            node: para_id,
            offset: 2,
            affinity: Affinity::Upstream,
        };

        let cm =
            cursor_metrics(&index, &pos, None).expect("cursor after tab-only line must resolve");

        let entry = index.entry_for_position(&pos).unwrap();
        let node = entry.node(&index).unwrap();
        let LayoutContent::Line(line) = &node.content else {
            panic!("entry must be a line");
        };
        assert_eq!(cm.caret.x, entry.rect.x + grapheme::x_at_offset(line, &pos));
        assert!(cm.caret.height > 0.0, "caret must be visible");
    }

    #[test]
    fn cursor_metrics_atom_returns_none() {
        use editor_model::{AtomLeaf, HorizontalRuleVariant};

        let root = Dot::ROOT;
        let hr = Dot::new(1, 1);
        let p = Dot::new(1, 2);
        let items = vec![
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: HorizontalRuleVariant::default(),
                    },
                    parents: vec![root],
                },
            ),
            (
                p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let doc = logs(&items);
        let root_id = root;
        let (_, index) = build_index(&doc, 400.0);

        let pos = Position {
            node: root_id,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        assert!(
            cursor_metrics(&index, &pos, None).is_none(),
            "cursor_metrics must return None for an atom position"
        );
    }
}
