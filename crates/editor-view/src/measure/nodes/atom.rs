use editor_model::{LeafView, NodeType};

use crate::measure::context::MeasureContext;
use crate::measure::types::{MeasuredAtom, MeasuredContent, MeasuredNode};

const HORIZONTAL_RULE_HEIGHT: f32 = 24.0;
const DEFAULT_EXTERNAL_HEIGHT: f32 = 1.0;

pub(crate) fn measure_atom(leaf: &LeafView, width: f32, ctx: &MeasureContext) -> MeasuredNode {
    let height = match leaf.node_type() {
        NodeType::HorizontalRule => HORIZONTAL_RULE_HEIGHT,
        _ => ctx
            .external_height(&leaf.dot())
            .unwrap_or(DEFAULT_EXTERNAL_HEIGHT),
    };
    MeasuredNode {
        width,
        height,
        content: MeasuredContent::Atom(MeasuredAtom { node: leaf.dot() }),
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, AtomLeaf, ChildView, DocLogs, DocView, HorizontalRuleVariant, ModifierAttrLog,
        Node, NodeAttrLog, NodeType, SeqItem, SpanLog, project_document,
    };

    use crate::measure::context::MeasureContext;

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

    fn build_doc_with_hr() -> (DocLogs, Dot) {
        let root = Dot::ROOT;
        let hr = Dot::new(1, 1);
        let items = vec![(
            hr,
            SeqItem::BlockAtom {
                leaf: AtomLeaf::HorizontalRule {
                    variant: HorizontalRuleVariant::default(),
                },
                parents: vec![root],
            },
        )];
        (logs(&items), hr)
    }

    fn build_doc_with_image() -> (DocLogs, Dot) {
        let root = Dot::ROOT;
        let img = Dot::new(1, 1);
        let img_node = match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        let items = vec![(
            img,
            SeqItem::BlockAtom {
                leaf: AtomLeaf::Image { node: img_node },
                parents: vec![root],
            },
        )];
        (logs(&items), img)
    }

    #[test]
    fn horizontal_rule_is_24() {
        let (doc, hr_dot) = build_doc_with_hr();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();

        let lv = root
            .children()
            .find_map(|c| match c {
                ChildView::Leaf(lv) => Some(lv),
                _ => None,
            })
            .unwrap();

        let result = measure_atom(&lv, 300.0, &MeasureContext::default());
        assert_eq!(result.width, 300.0);
        assert_eq!(result.height, 24.0);
        assert!(matches!(&result.content, MeasuredContent::Atom(a) if a.node == hr_dot));
    }

    #[test]
    fn image_is_placeholder_height() {
        let (doc, img_dot) = build_doc_with_image();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();

        let lv = root
            .children()
            .find_map(|c| match c {
                ChildView::Leaf(lv) => Some(lv),
                _ => None,
            })
            .unwrap();

        let result = measure_atom(&lv, 300.0, &MeasureContext::default());
        assert_eq!(result.width, 300.0);
        assert_eq!(result.height, 1.0);
        assert!(matches!(&result.content, MeasuredContent::Atom(a) if a.node == img_dot));
    }

    #[test]
    fn atom_external_height_from_ctx() {
        use hashbrown::HashMap;

        let (doc, img_dot) = build_doc_with_image();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();
        let lv = root
            .children()
            .find_map(|c| match c {
                ChildView::Leaf(lv) => Some(lv),
                _ => None,
            })
            .unwrap();
        let image_elem_id = lv.dot();

        let ctx_with_height = MeasureContext {
            external_heights: HashMap::from([(image_elem_id, 200.0)]),
            ..Default::default()
        };
        let result = measure_atom(&lv, 300.0, &ctx_with_height);
        assert_eq!(result.height, 200.0);

        let result_default = measure_atom(&lv, 300.0, &MeasureContext::default());
        assert_eq!(result_default.height, 1.0);

        let (hr_doc, _) = build_doc_with_hr();
        let hr_pd = project_document(&hr_doc).unwrap();
        let hr_view = DocView::new(&hr_pd);
        let hr_root = hr_view.root().unwrap();
        let hr_lv = hr_root
            .children()
            .find_map(|c| match c {
                ChildView::Leaf(lv) => Some(lv),
                _ => None,
            })
            .unwrap();
        let result_hr = measure_atom(&hr_lv, 300.0, &ctx_with_height);
        assert_eq!(result_hr.height, 24.0);

        assert!(matches!(&result.content, MeasuredContent::Atom(a) if a.node == img_dot));
    }
}
