use editor_commands as commands;
use editor_model::{Node, PlainImageNode, PlainNode};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_node_op(editor: &mut Editor, op: NodeOp) -> Result<(), EditorError> {
    editor.transact(|tr| match op {
        NodeOp::SetAttrs { id, attrs } => {
            tr.set_node(id, attrs)?;
            Ok(())
        }
        NodeOp::SetImageId { id, image_id } => {
            let doc = tr.doc();
            let Some(node_ref) = doc.node(id) else {
                return Err(EditorError::General {
                    msg: format!("node {id} not found"),
                });
            };
            let Node::Image(image) = node_ref.node() else {
                return Err(EditorError::General {
                    msg: format!("node {id} is not an image"),
                });
            };
            let proportion = *image.proportion.get();
            tr.set_node(
                id,
                PlainNode::Image(PlainImageNode {
                    id: Some(image_id),
                    upload_id: None,
                    proportion,
                }),
            )?;
            Ok(())
        }
        NodeOp::SetImageProportion { id, proportion } => {
            let doc = tr.doc();
            let Some(node_ref) = doc.node(id) else {
                return Err(EditorError::General {
                    msg: format!("node {id} not found"),
                });
            };
            let Node::Image(image) = node_ref.node() else {
                return Err(EditorError::General {
                    msg: format!("node {id} is not an image"),
                });
            };
            let image_id = image.id.get().clone();
            let upload_id = image.upload_id.get().clone();
            tr.set_node(
                id,
                PlainNode::Image(PlainImageNode {
                    id: image_id,
                    upload_id,
                    proportion,
                }),
            )?;
            Ok(())
        }
        NodeOp::Delete { id } => {
            commands::delete_node(tr, id)?;
            Ok(())
        }
        NodeOp::Table { .. } => Ok(()),
    })
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Node;
    use editor_state::assert_state_eq;

    use super::*;

    #[test]
    fn delete_node_removes_selected_external_block_and_records_history() {
        let (initial, _root, _t1, img, ..) = state! {
            doc { r: root {
                paragraph { t1: text("Before") }
                img: image
                paragraph { t2: text("After") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(initial.clone());

        editor.apply(Message::Node {
            op: NodeOp::Delete { id: img },
        });

        let (deleted, ..) = state! {
            doc { root {
                paragraph { t1: text("Before") }
                paragraph { t2: text("After") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(editor.state(), &deleted);
        assert!(editor.history.can_undo());

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
        assert_state_eq!(editor.state(), &deleted);
    }

    #[test]
    fn set_image_proportion_preserves_image_id() {
        let (initial, img) = state! {
            doc { root {
                img: image(id: Some("asset-1".to_string()), proportion: 100)
            } }
            selection: (img, 0)
        };
        let mut editor = Editor::new_test(initial);

        editor.apply(Message::Node {
            op: NodeOp::SetImageProportion {
                id: img,
                proportion: 40,
            },
        });

        let node_ref = editor.state().doc.node(img).expect("image node exists");
        let Node::Image(image) = node_ref.node() else {
            panic!("expected image node");
        };
        assert_eq!(image.id.get().as_deref(), Some("asset-1"));
        assert_eq!(*image.proportion.get(), 40);
    }

    #[test]
    fn set_image_id_clears_upload_id_and_preserves_proportion() {
        let (initial, img) = state! {
            doc { root {
                img: image(upload_id: Some("upload-1".to_string()), proportion: 55)
            } }
            selection: (img, 0)
        };
        let mut editor = Editor::new_test(initial);

        editor.apply(Message::Node {
            op: NodeOp::SetImageId {
                id: img,
                image_id: "asset-1".to_string(),
            },
        });

        let node_ref = editor.state().doc.node(img).expect("image node exists");
        let Node::Image(image) = node_ref.node() else {
            panic!("expected image node");
        };
        assert_eq!(image.id.get().as_deref(), Some("asset-1"));
        assert_eq!(image.upload_id.get(), &None);
        assert_eq!(*image.proportion.get(), 55);
    }
}
