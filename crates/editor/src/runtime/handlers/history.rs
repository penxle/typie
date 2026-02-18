use crate::runtime::{Effect, Runtime};
use crate::transaction::compute_styles_at_cursor;

impl Runtime {
    pub(crate) fn handle_undo(&mut self) -> Vec<Effect> {
        if !self.undo_manager.can_undo() {
            return vec![];
        }

        self.redo_selections.push(self.state.selection);

        let _ = self.undo_manager.undo();
        self.state.doc.clear_children_cache();

        if let Some(selection) = self.undo_selections.pop() {
            self.state.selection = self.validate_selection(selection);
            let new_styles = compute_styles_at_cursor(&self.state.doc, &self.state.selection.head);
            self.state.pending_styles = new_styles;
        }

        vec![
            Effect::FullLayoutInvalidation,
            Effect::DocChanged,
            Effect::SelectionChanged,
            Effect::PendingStylesChanged,
        ]
    }

    pub(crate) fn handle_redo(&mut self) -> Vec<Effect> {
        if !self.undo_manager.can_redo() {
            return vec![];
        }

        self.undo_selections.push(self.state.selection);

        let _ = self.undo_manager.redo();
        self.state.doc.clear_children_cache();

        if let Some(selection) = self.redo_selections.pop() {
            self.state.selection = self.validate_selection(selection);
            let new_styles = compute_styles_at_cursor(&self.state.doc, &self.state.selection.head);
            self.state.pending_styles = new_styles;
        }

        vec![
            Effect::FullLayoutInvalidation,
            Effect::DocChanged,
            Effect::SelectionChanged,
            Effect::PendingStylesChanged,
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Message;

    #[test]
    fn test_undo_restores_selection() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "Hello" }
                }
            }
            selection { (p, 5) }
        };

        rt.update(Message::Input {
            text: " World".to_string(),
        });
        rt.flush();

        assert_eq!(
            rt.state().selection.head.offset,
            11,
            "precondition: selection should be at end of inserted text"
        );

        rt.update(Message::Undo);

        assert_eq!(
            rt.state().selection.head.offset,
            5,
            "selection should be restored to position before input"
        );
    }

    #[test]
    fn test_redo_restores_selection() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "Hello" }
                }
            }
            selection { (p, 5) }
        };

        rt.update(Message::Input {
            text: " World".to_string(),
        });
        rt.flush();

        let selection_after_input = rt.state().selection.head.offset;

        rt.update(Message::Undo);

        assert_eq!(
            rt.state().selection.head.offset,
            5,
            "precondition: undo should restore selection"
        );

        rt.update(Message::Redo);

        assert_eq!(
            rt.state().selection.head.offset,
            selection_after_input,
            "redo should restore selection to after input"
        );
    }

    #[test]
    fn test_multiple_undo_redo_restores_to_original() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        rt.update(Message::Input {
            text: "BC".to_string(),
        });
        rt.flush();
        assert_eq!(rt.state().selection.head.offset, 3);

        rt.update(Message::Undo);
        assert_eq!(
            rt.state().selection.head.offset,
            1,
            "undo should restore to original selection"
        );

        rt.update(Message::Redo);
        assert_eq!(
            rt.state().selection.head.offset,
            3,
            "redo should restore to after input"
        );
    }

    #[test]
    fn test_undo_invalidates_layout_cache() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "Hello" }
                }
            }
            selection { (p, 5) }
        };

        rt.layout();
        assert!(
            rt.is_layout_cached(p),
            "precondition: paragraph layout should be cached"
        );

        rt.update(Message::Input {
            text: " World".to_string(),
        });
        rt.flush();

        rt.layout();
        assert!(
            rt.is_layout_cached(p),
            "precondition: paragraph layout should be cached after input"
        );

        rt.update(Message::Undo);

        assert!(
            !rt.is_layout_cached(p),
            "paragraph layout cache should be invalidated after undo"
        );
    }
}
