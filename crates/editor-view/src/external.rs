use editor_common::Rect;
use editor_crdt::Dot;
use editor_macros::ffi;
use editor_model::{DocView, LayoutMode, Node};
use editor_state::{Affinity, Position, ResolvedSelection, Selection};
use serde::{Deserialize, Serialize};

use crate::paginate::types::LayoutContent;
use crate::query::layout_index::LayoutIndex;
use crate::style::BorderMode;

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExternalElementData {
    Image {
        id: Option<String>,
        proportion: u32,
        /// Full-page image height limit, independent of its current position.
        #[serde(default)]
        max_height: Option<f32>,
    },
    File {
        id: Option<String>,
    },
    Embed {
        id: Option<String>,
    },
    Archived {
        id: Option<String>,
    },
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
    let page_content_height = match view.root().map(|root| root.node()) {
        Some(Node::Root(root)) => match *root.layout_mode.get() {
            LayoutMode::Paginated {
                page_height,
                page_margin_top,
                page_margin_bottom,
                ..
            } => Some(page_height as f32 - page_margin_top as f32 - page_margin_bottom as f32),
            LayoutMode::Continuous { .. } => None,
        },
        _ => None,
    };
    let mut elements = Vec::new();
    for entry in layout_index.entries_on_page(page_idx) {
        let Some(LayoutContent::Atom(atom)) = entry.content(layout_index) else {
            continue;
        };
        let Some(mut data) = external_element_data(view, &atom.node) else {
            continue;
        };
        if let ExternalElementData::Image { max_height, .. } = &mut data {
            *max_height = page_content_height.map(|height| {
                let mut height = height;
                for ancestor in entry.ancestors() {
                    if let Some(LayoutContent::Box(b)) = layout_index
                        .box_entry(ancestor)
                        .and_then(|entry| entry.content(layout_index))
                    {
                        height -= b.style.padding.top + b.style.padding.bottom;
                        if b.style.border_mode != BorderMode::Collapse {
                            height -= b.style.border.top + b.style.border.bottom;
                        }
                    }
                }
                height.max(1.0)
            });
        }
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
            max_height: None,
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
        AliasLog, AtomLeaf, DocLogs, DocView, ImageNodeAttr, ModifierAttrLog, Node, NodeAttr,
        NodeAttrLog, NodeAttrOp, NodeType, SeqItem, SpanLog, project_document,
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
                    attrs: vec![],
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
                proportion: 100,
                max_height: None,
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
                proportion: 150,
                max_height: None,
            }
        );
    }

    #[test]
    fn image_height_limit_uses_full_page_and_container_padding() {
        use editor_model::{LayoutMode, RootNodeAttr};
        use hashbrown::HashMap;

        for (wrappers, expected_height) in [
            (vec![], 800.0),
            (
                vec![NodeType::Table, NodeType::TableRow, NodeType::TableCell],
                782.0,
            ),
            (vec![NodeType::Fold, NodeType::FoldContent], 766.0),
        ] {
            let mut parents = vec![Dot::ROOT];
            let mut fold_states = HashMap::new();
            let mut items = vec![
                (
                    Dot::new(1, 1),
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: parents.clone(),
                        attrs: vec![],
                    },
                ),
                (Dot::new(1, 2), SeqItem::Char('x')),
            ];
            for node_type in wrappers {
                let id = Dot::new(1, items.len() as u64 + 1);
                if node_type == NodeType::Fold {
                    fold_states.insert(id, true);
                }
                items.push((
                    id,
                    SeqItem::Block {
                        node_type,
                        parents: parents.clone(),
                        attrs: vec![],
                    },
                ));
                parents.push(id);
            }
            let image = Dot::new(1, items.len() as u64 + 1);
            let Node::Image(node) = NodeType::Image.into_node() else {
                unreachable!()
            };
            items.push((
                image,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node },
                    parents,
                },
            ));
            let mut doc = logs(&items);
            doc.node_attrs = doc
                .node_attrs
                .apply(
                    Dot::new(2, 1),
                    NodeAttrOp {
                        target: Dot::ROOT,
                        attr: NodeAttr::Root {
                            attr: RootNodeAttr::LayoutMode(LayoutMode::Paginated {
                                page_width: 800,
                                page_height: 1000,
                                page_margin_top: 100,
                                page_margin_bottom: 100,
                                page_margin_left: 100,
                                page_margin_right: 100,
                            }),
                        },
                    },
                )
                .unwrap();
            let projected = project_document(&doc).unwrap();
            let view = DocView::new(&projected);
            let make_index = |height| {
                let measured = measure_node(
                    &mut crate::measure::Measurer::new(),
                    &view.root().unwrap(),
                    600.0,
                    &MeasureContext {
                        external_heights: HashMap::from([(image, height)]),
                        fold_states: fold_states.clone(),
                        ..Default::default()
                    },
                    &mut Resource::new_test(),
                );
                let layout = Paginator::paginated(800.0, 1000.0, EdgeInsets::all(100.0))
                    .paginate(MeasuredTree { root: measured });
                LayoutIndex::new(layout.tree, &layout.pages)
            };
            // The initial placeholder follows text on page one. Once the host
            // reports the fitted height, it moves intact to the next page.
            for height in [1.0, expected_height] {
                let index = make_index(height);
                let elements = external_elements(&index, &view, None);
                assert_eq!(elements.len(), 1);
                let el = &elements[0];
                assert!(
                    matches!(el.data, ExternalElementData::Image { max_height: Some(h), .. } if h == expected_height),
                    "expected {expected_height}: {:?}",
                    el.data
                );
                assert_eq!(el.page_idx, usize::from(height > 1.0));
                assert!(el.bounds.bottom() <= 900.001);
            }
        }
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
