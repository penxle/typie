use editor_common::Rect;
use editor_macros::ffi;
use editor_model::{Doc, Node, NodeId};
use editor_state::Selection;
use serde::{Deserialize, Serialize};

use crate::paginate::LayoutContent;
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
    pub node_id: NodeId,
    pub bounds: Rect,
    pub data: ExternalElementData,
    pub is_selected: bool,
}

pub(crate) fn external_elements(
    layout_index: &LayoutIndex,
    doc: &Doc,
    selection: Option<&Selection>,
) -> Vec<ExternalElement> {
    let selection = selection.and_then(|s| s.resolve(doc));
    let mut elements = Vec::new();
    for (page_idx, page) in layout_index.pages().iter().enumerate() {
        for entry in layout_index.entries_on_page(page_idx) {
            let Some(LayoutContent::Atom(atom)) = entry.content(layout_index) else {
                continue;
            };
            let Some(data) = external_element_data(doc, atom.node_id) else {
                continue;
            };
            let is_selected = selection.as_ref().is_some_and(|sel| {
                doc.node(atom.node_id)
                    .is_some_and(|node_ref| sel.contains_subtree(&node_ref))
            });
            elements.push(ExternalElement {
                page_idx,
                node_id: atom.node_id,
                bounds: Rect::from_xywh(
                    entry.rect.x,
                    entry.rect.y - page.y_start,
                    entry.rect.width,
                    entry.rect.height,
                ),
                data,
                is_selected,
            });
        }
    }
    elements
}

fn external_element_data(doc: &Doc, node_id: NodeId) -> Option<ExternalElementData> {
    match doc.node(node_id)?.node() {
        Node::Image(img) => Some(ExternalElementData::Image {
            id: img.id.get().clone(),
            proportion: *img.proportion.get(),
        }),
        Node::File(file) => Some(ExternalElementData::File {
            id: file.id.get().clone(),
        }),
        Node::Embed(embed) => Some(ExternalElementData::Embed {
            id: embed.id.get().clone(),
        }),
        Node::Archived(archived) => Some(ExternalElementData::Archived {
            id: archived.id.get().clone(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::{doc, state};
    use editor_state::{Position, Selection};

    use crate::View;

    #[test]
    fn lists_external_atoms_with_page_local_bounds() {
        let (doc, img, file, embed, archived) = doc! {
            root {
                img: image(id: Some("img-1".to_string()), proportion: 50)
                file: file(id: Some("file-1".to_string()))
                embed: embed(id: Some("embed-1".to_string()))
                archived: archived(id: Some("archived-1".to_string()))
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let elements =
            view.external_elements(&doc, Some(&Selection::collapsed(Position::new(img, 0))));

        assert_eq!(elements.len(), 4);
        assert_eq!(elements[0].node_id, img);
        assert_eq!(elements[0].page_idx, 0);
        assert_eq!(elements[0].bounds.width, elements[1].bounds.width);
        assert_eq!(elements[0].bounds.height, 1.0);
        assert_eq!(
            elements.iter().map(|el| el.node_id).collect::<Vec<_>>(),
            vec![img, file, embed, archived]
        );
    }

    #[test]
    fn marks_bracketed_external_node_as_selected() {
        let (state, _root, img) = state! {
            doc { r: root { img: image paragraph } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let mut view = View::new_test();
        view.layout(&state.doc);

        let elements = view.external_elements(&state.doc, state.selection.as_ref());

        let image = elements
            .iter()
            .find(|element| element.node_id == img)
            .expect("image external element");
        assert!(image.is_selected);
    }
}
