use editor_common::Rect;
use editor_state::Position;

use crate::page::{LayoutPage, PageRect};
use crate::paginate::types::{LayoutBox, LayoutContent, LayoutLine, LayoutNode};

use super::common::*;
use super::layout_index::{LayoutEntry, LayoutIndex};

pub type CompositionRect = PageRect;

pub(crate) fn composition_rects(
    layout_index: &LayoutIndex,
    from: &Position,
    to: &Position,
) -> Vec<CompositionRect> {
    if from == to {
        return vec![];
    }

    let from_owner = layout_index.entry_for_position(from);
    let to_owner = layout_index.entry_for_position(to);
    let mut phase = Phase::Before;
    let mut rects = Vec::new();

    visit_node(
        &layout_index.tree().root,
        layout_index,
        from,
        to,
        from_owner,
        to_owner,
        &mut phase,
        &mut rects,
        layout_index.pages(),
    );

    rects
}

fn visit_node(
    node: &LayoutNode,
    layout_index: &LayoutIndex,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutEntry>,
    to_owner: Option<&LayoutEntry>,
    phase: &mut Phase,
    rects: &mut Vec<CompositionRect>,
    pages: &[LayoutPage],
) {
    match &node.content {
        LayoutContent::Box(b) => visit_box(
            node,
            b,
            layout_index,
            from,
            to,
            from_owner,
            to_owner,
            phase,
            rects,
            pages,
        ),
        LayoutContent::Line(l) => {
            visit_line(
                node,
                l,
                layout_index,
                from,
                to,
                from_owner,
                to_owner,
                phase,
                rects,
                pages,
            );
        }
        LayoutContent::Atom(_) | LayoutContent::Spacing(_) => {}
    }
}

fn visit_line(
    node: &LayoutNode,
    line: &LayoutLine,
    layout_index: &LayoutIndex,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutEntry>,
    to_owner: Option<&LayoutEntry>,
    phase: &mut Phase,
    rects: &mut Vec<CompositionRect>,
    pages: &[LayoutPage],
) {
    let contains_from = from_owner.is_some_and(|entry| entry.is_node(layout_index, node));
    let contains_to = to_owner.is_some_and(|entry| entry.is_node(layout_index, node));

    let (x_start, x_end) = match (*phase, contains_from, contains_to) {
        (Phase::Before, true, true) => {
            *phase = Phase::After;
            (
                super::grapheme::x_at_offset(line, from),
                super::grapheme::x_at_offset(line, to),
            )
        }
        (Phase::Before, true, false) => {
            *phase = Phase::Inside;
            (super::grapheme::x_at_offset(line, from), line_end_x(line))
        }
        (Phase::Inside, false, false) => (line_start_x(line), line_end_x(line)),
        (Phase::Inside, false, true) => {
            *phase = Phase::After;
            (line_start_x(line), super::grapheme::x_at_offset(line, to))
        }
        _ => return,
    };

    let width = x_end - x_start;
    if width <= 0.0 {
        return;
    }

    if let Some(page_idx) = page_for_y(pages, node.rect.y) {
        let underline_y =
            node.rect.y - pages[page_idx].y_start + line.baseline + line.descent * 0.5;

        rects.push(PageRect::new(
            page_idx,
            Rect::from_xywh(node.rect.x + x_start, underline_y, width, 1.0),
        ));
    }
}

fn visit_box(
    _node: &LayoutNode,
    bx: &LayoutBox,
    layout_index: &LayoutIndex,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutEntry>,
    to_owner: Option<&LayoutEntry>,
    phase: &mut Phase,
    rects: &mut Vec<CompositionRect>,
    pages: &[LayoutPage],
) {
    for child in &bx.children {
        if *phase == Phase::After {
            break;
        }
        visit_node(
            child,
            layout_index,
            from,
            to,
            from_owner,
            to_owner,
            phase,
            rects,
            pages,
        );
    }
}

#[cfg(test)]
mod tests {
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType, SeqItem, SpanLog,
        project_document,
    };
    use editor_resource::Resource;
    use editor_state::Affinity;

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::paginate::paginator::Paginator;

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
            aliases: AliasLog::new(),
        }
    }

    fn build_index(doc: &DocLogs, width: f32) -> LayoutIndex {
        let pd = project_document(doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
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
        LayoutIndex::new(layout.tree, &layout.pages)
    }

    fn para_doc(text: &str, width: f32) -> (Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for (i, ch) in text.chars().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
        }
        let doc = logs(&items);
        let para_id = para;
        let index = build_index(&doc, width);
        (para_id, index)
    }

    fn two_para_doc(text1: &str, text2: &str, width: f32) -> (Dot, Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let para1 = Dot::new(2, 1);
        let base = 2u64;
        let mut items = vec![(
            para1,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        let mut offset = base;
        for ch in text1.chars() {
            items.push((Dot::new(2, offset), SeqItem::Char(ch)));
            offset += 1;
        }
        let para2 = Dot::new(2, offset);
        items.push((
            para2,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        ));
        offset += 1;
        for ch in text2.chars() {
            items.push((Dot::new(2, offset), SeqItem::Char(ch)));
            offset += 1;
        }
        let doc = logs(&items);
        let p1 = para1;
        let p2 = para2;
        let index = build_index(&doc, width);
        (p1, p2, index)
    }

    #[test]
    fn same_position_returns_empty() {
        let (para_id, index) = para_doc("hello", 400.0);
        let pos = Position {
            node: para_id,
            offset: 2,
            affinity: Affinity::default(),
        };
        let rects = composition_rects(&index, &pos, &pos);
        assert!(rects.is_empty());
    }

    #[test]
    fn single_line_composition() {
        let (para_id, index) = para_doc("hello", 400.0);
        let from = Position {
            node: para_id,
            offset: 1,
            affinity: Affinity::default(),
        };
        let to = Position {
            node: para_id,
            offset: 4,
            affinity: Affinity::default(),
        };
        let rects = composition_rects(&index, &from, &to);

        assert_eq!(rects.len(), 1);
        assert!(rects[0].rect.width > 0.0);
        assert_eq!(rects[0].rect.height, 1.0);
    }

    #[test]
    fn multi_paragraph_composition() {
        let (p1, p2, index) = two_para_doc("hello", "world", 400.0);
        let from = Position {
            node: p1,
            offset: 2,
            affinity: Affinity::default(),
        };
        let to = Position {
            node: p2,
            offset: 3,
            affinity: Affinity::default(),
        };
        let rects = composition_rects(&index, &from, &to);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].rect.height, 1.0);
        assert_eq!(rects[1].rect.height, 1.0);
        assert!(rects[0].rect.y < rects[1].rect.y);
    }

    #[test]
    fn soft_wrap_lower_line_emits_rect() {
        let (para_id, index) = para_doc("abcdefgh", 40.0);
        let from = Position {
            node: para_id,
            offset: 6,
            affinity: Affinity::default(),
        };
        let to = Position {
            node: para_id,
            offset: 7,
            affinity: Affinity::default(),
        };
        let rects = composition_rects(&index, &from, &to);

        assert_eq!(rects.len(), 1);
        assert!(rects[0].rect.width > 0.0);
    }
}
