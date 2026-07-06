use editor_common::Rect;
use editor_crdt::Dot;
use editor_macros::ffi;
use editor_model::{DocView, Node};
use editor_state::{Affinity, Position, ResolvedSelection, Selection};
use serde::{Deserialize, Serialize};

use crate::paginate::types::LayoutContent;
use crate::query::layout_index::LayoutIndex;

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExternalElementData {
    Image { id: Option<String>, proportion: u32 },
    File { id: Option<String> },
    Embed { id: Option<String> },
    Archived { id: Option<String> },
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExternalElement {
    pub page_idx: usize,
    pub node: Dot,
    pub bounds: Rect,
    pub is_selected: bool,
    pub data: ExternalElementData,
}

pub(crate) fn page_external_elements(
    layout_index: &LayoutIndex,
    view: &DocView,
    page_idx: usize,
    selection: Option<&ResolvedSelection>,
) -> Vec<ExternalElement> {
    let Some(page) = layout_index.pages().get(page_idx) else {
        return Vec::new();
    };
    let mut elements = Vec::new();
    for entry in layout_index.entries_on_page(page_idx) {
        let Some(LayoutContent::Atom(atom)) = entry.content(layout_index) else {
            continue;
        };
        let Some(data) = external_element_data(view, &atom.node) else {
            continue;
        };
        let slot = Selection::new(
            Position {
                node: atom.attachment.parent,
                offset: atom.attachment.index,
                affinity: Affinity::Downstream,
            },
            Position {
                node: atom.attachment.parent,
                offset: atom.attachment.index + 1,
                affinity: Affinity::Upstream,
            },
        );
        let is_selected = selection.is_some_and(|sel| sel.contains_range(slot));
        elements.push(ExternalElement {
            page_idx,
            node: atom.node,
            bounds: Rect::from_xywh(
                entry.rect.x,
                entry.rect.y - page.y_start,
                entry.rect.width,
                entry.rect.height,
            ),
            is_selected,
            data,
        });
    }
    elements
}

pub(crate) fn external_elements(
    layout_index: &LayoutIndex,
    view: &DocView,
    selection: Option<&ResolvedSelection>,
) -> Vec<ExternalElement> {
    let mut elements = Vec::new();
    for page_idx in 0..layout_index.pages().len() {
        elements.extend(page_external_elements(
            layout_index,
            view,
            page_idx,
            selection,
        ));
    }
    elements
}

fn external_element_data(view: &DocView, id: &Dot) -> Option<ExternalElementData> {
    let dot = id;
    match view.leaf(*dot)?.node()? {
        Node::Image(node) => Some(ExternalElementData::Image {
            id: node.id.get().clone(),
            proportion: *node.proportion.get(),
        }),
        Node::File(node) => Some(ExternalElementData::File {
            id: node.id.get().clone(),
        }),
        Node::Embed(node) => Some(ExternalElementData::Embed {
            id: node.id.get().clone(),
        }),
        Node::Archived(node) => Some(ExternalElementData::Archived {
            id: node.id.get().clone(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AtomLeaf, DocLogs, DocView, ImageNodeAttr, ModifierAttrLog, Node, NodeAttr, NodeAttrLog,
        NodeAttrOp, NodeMarkerLog, NodeType, SeqItem, SpanLog, project_document,
    };
    use editor_resource::Resource;
    use editor_state::{Position, Selection};

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::paginate::paginator::Paginator;
    use crate::query::layout_index::LayoutIndex;

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
            node_markers: NodeMarkerLog::new(),
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

    fn image_doc() -> (DocLogs, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let img_dot = Dot::new(10, 1);
        let para = Dot::new(10, 2);
        let img_node = match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        let items = vec![
            (
                img_dot,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node },
                    parents: vec![root],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(10, 3), SeqItem::Char('x')),
        ];
        (logs(&items), root, img_dot, para)
    }

    #[test]
    fn image_external_element_bounds_and_data() {
        let (doc, _root, img_dot, _para) = image_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let index = build_index(&doc, 400.0);

        let elements = page_external_elements(&index, &view, 0, None);

        assert_eq!(elements.len(), 1, "expected one external element");
        let el = &elements[0];
        assert_eq!(el.node, img_dot);
        assert_eq!(el.page_idx, 0);
        assert!(el.bounds.width > 0.0 || el.bounds.height >= 0.0);
        assert_eq!(
            el.data,
            ExternalElementData::Image {
                id: None,
                proportion: 100
            },
            "image data must use the current projected node"
        );
        assert!(!el.is_selected);
    }

    #[test]
    fn image_external_element_data_reflects_node_attrs() {
        let (mut doc, _root, img_dot, _para) = image_doc();
        doc.node_attrs = NodeAttrLog::new()
            .apply(
                Dot::new(20, 0),
                NodeAttrOp {
                    target: img_dot,
                    attr: NodeAttr::Image {
                        attr: ImageNodeAttr::Id(Some("asset-1".to_string())),
                    },
                },
            )
            .unwrap()
            .apply(
                Dot::new(20, 1),
                NodeAttrOp {
                    target: img_dot,
                    attr: NodeAttr::Image {
                        attr: ImageNodeAttr::Proportion(150),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let index = build_index(&doc, 400.0);

        let elements = page_external_elements(&index, &view, 0, None);

        assert_eq!(
            elements[0].data,
            ExternalElementData::Image {
                id: Some("asset-1".to_string()),
                proportion: 150
            }
        );
    }

    #[test]
    fn is_selected_trap2_covering_selection_true_collapsed_false() {
        let (doc, root, img_dot, _para) = image_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let index = build_index(&doc, 400.0);

        let root_id = root;
        let img_id = img_dot;

        let elements_no_sel = page_external_elements(&index, &view, 0, None);
        assert_eq!(elements_no_sel.len(), 1);
        let atom_index = {
            let entry = index
                .entries_on_page(0)
                .into_iter()
                .find(|e| {
                    e.content(&index)
                        .is_some_and(|c| matches!(c, LayoutContent::Atom(a) if a.node == img_id))
                })
                .expect("image atom entry");
            match entry.content(&index).unwrap() {
                LayoutContent::Atom(atom) => atom.attachment.index,
                _ => unreachable!(),
            }
        };

        let covering = Selection::new(
            Position::new(root_id, atom_index),
            Position::new(root_id, atom_index + 1),
        );
        let covering_resolved = covering
            .resolve(&view)
            .expect("covering selection must resolve");
        let elements_covered = page_external_elements(&index, &view, 0, Some(&covering_resolved));
        assert_eq!(elements_covered.len(), 1);
        assert!(
            elements_covered[0].is_selected,
            "Trap-2: selection covering atom slot must yield is_selected=true"
        );

        let covering_upstream_head = Selection::new(
            Position::new(root_id, atom_index),
            Position {
                node: root_id,
                offset: atom_index + 1,
                affinity: editor_state::Affinity::Upstream,
            },
        );
        let covering_upstream_resolved = covering_upstream_head
            .resolve(&view)
            .expect("covering selection with upstream head must resolve");
        let elements_upstream =
            page_external_elements(&index, &view, 0, Some(&covering_upstream_resolved));
        assert_eq!(elements_upstream.len(), 1);
        assert!(
            elements_upstream[0].is_selected,
            "Trap-3: canonical forward selection (Downstream anchor, Upstream head at block end) \
             covering the atom slot must yield is_selected=true"
        );

        let collapsed = Selection::new(
            Position::new(root_id, atom_index),
            Position::new(root_id, atom_index),
        );
        let collapsed_resolved = collapsed
            .resolve(&view)
            .expect("collapsed selection must resolve");
        let elements_collapsed =
            page_external_elements(&index, &view, 0, Some(&collapsed_resolved));
        assert_eq!(elements_collapsed.len(), 1);
        assert!(
            !elements_collapsed[0].is_selected,
            "Trap-2: collapsed selection at atom start must yield is_selected=false"
        );
    }
}
