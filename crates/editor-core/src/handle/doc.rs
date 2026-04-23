use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_doc_op(editor: &mut Editor, op: DocOp) -> Result<(), EditorError> {
    editor.transact(|tr| match op {
        DocOp::SetAttrs { attrs } => {
            tr.set_document_attrs(attrs)?;
            Ok(())
        }
    })
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{DocumentAttrs, LayoutMode};

    use crate::editor::Editor;
    use crate::event::EditorEvent;
    use crate::message::*;

    fn paginated_attrs(page_width: f32) -> DocumentAttrs {
        DocumentAttrs {
            layout_mode: LayoutMode::Paginated {
                page_width,
                page_height: 600.0,
                page_margin_top: 20.0,
                page_margin_bottom: 20.0,
                page_margin_left: 20.0,
                page_margin_right: 20.0,
            },
        }
    }

    #[test]
    fn doc_set_attrs_emits_render_invalidated_once() {
        let (state, _t1) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("hello") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });

        let events = editor.apply(Message::Doc {
            op: DocOp::SetAttrs {
                attrs: paginated_attrs(600.0),
            },
        });

        let render_invalidated_count = events
            .iter()
            .filter(|e| matches!(e, EditorEvent::RenderInvalidated))
            .count();
        assert_eq!(
            render_invalidated_count, 1,
            "RenderInvalidated must be emitted exactly once"
        );
    }

    #[test]
    fn doc_set_attrs_twice_preserves_page_width() {
        let attrs = paginated_attrs(400.0);
        let (state, _t1) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    paragraph { t1: text("hello") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Doc {
            op: DocOp::SetAttrs {
                attrs: attrs.clone(),
            },
        });

        editor.apply(Message::Doc {
            op: DocOp::SetAttrs {
                attrs: attrs.clone(),
            },
        });

        let pages = editor.view.pages();
        assert!(!pages.is_empty());
        assert_eq!(pages[0].size.width, 400.0);
    }
}
