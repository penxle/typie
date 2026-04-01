use crate::runtime::{Effect, Runtime};
use crate::transaction::compute_styles_at_cursor;

impl Runtime {
    pub fn handle_undo(&mut self) -> Vec<Effect> {
        self.flush();

        if !self.undo_manager.can_undo() {
            return vec![];
        }

        let settings_before_undo = self.doc().settings();
        let selection_before_undo = self.capture_history_selection(self.state.selection);
        let top_undo_marker_before = self.top_undo_marker();
        self.clear_history_pop_events();
        let undo_executed = self.undo_manager.undo().unwrap_or(false);
        let mut popped_markers = self.take_history_pop_markers(loro::UndoOrRedo::Undo);

        if popped_markers.is_empty() {
            if let Some(top_undo_marker_before) = top_undo_marker_before {
                popped_markers.push(top_undo_marker_before);
            }
        }

        let mut restored_snapshot = None;
        for marker in popped_markers {
            if let Some(snapshot) = self.history_take_undo_snapshot(marker) {
                restored_snapshot = Some(snapshot);
            }
        }

        if undo_executed && let Some(redo_marker) = self.top_redo_marker() {
            self.history_record_redo_snapshot(redo_marker, selection_before_undo);
        }

        self.sync_history_selection_state();
        self.state.doc.clear_children_cache();

        if !undo_executed {
            return vec![];
        }

        self.state.selection = restored_snapshot
            .as_ref()
            .map(|snapshot| self.resolve_history_selection(snapshot))
            .unwrap_or_else(|| self.validate_selection(self.state.selection));
        let new_styles = compute_styles_at_cursor(&self.state.doc, &self.state.selection.head);
        self.state.pending_styles = new_styles;
        let settings_after_undo = self.doc().settings();
        let settings_changed = settings_after_undo != settings_before_undo;

        let mut effects = vec![
            Effect::FullLayoutInvalidation,
            Effect::DocChanged,
            Effect::SelectionChanged,
            Effect::PendingStylesChanged,
        ];
        if settings_changed {
            effects.push(Effect::SettingsChanged);
        }
        effects
    }

    pub fn handle_redo(&mut self) -> Vec<Effect> {
        self.flush();

        if !self.undo_manager.can_redo() {
            return vec![];
        }

        let settings_before_redo = self.doc().settings();
        let selection_before_redo = self.capture_history_selection(self.state.selection);
        let top_redo_marker_before = self.top_redo_marker();
        self.clear_history_pop_events();
        let redo_executed = self.undo_manager.redo().unwrap_or(false);
        let mut popped_markers = self.take_history_pop_markers(loro::UndoOrRedo::Redo);

        if popped_markers.is_empty() {
            if let Some(top_redo_marker_before) = top_redo_marker_before {
                popped_markers.push(top_redo_marker_before);
            }
        }

        let mut restored_snapshot = None;
        for marker in popped_markers {
            if let Some(snapshot) = self.history_take_redo_snapshot(marker) {
                restored_snapshot = Some(snapshot);
            }
        }

        if redo_executed && let Some(undo_marker) = self.top_undo_marker() {
            self.history_record_undo_snapshot(undo_marker, selection_before_redo);
        }

        self.sync_history_selection_state();
        self.state.doc.clear_children_cache();

        if !redo_executed {
            return vec![];
        }

        self.state.selection = restored_snapshot
            .as_ref()
            .map(|snapshot| self.resolve_history_selection(snapshot))
            .unwrap_or_else(|| self.validate_selection(self.state.selection));
        let new_styles = compute_styles_at_cursor(&self.state.doc, &self.state.selection.head);
        self.state.pending_styles = new_styles;
        let settings_after_redo = self.doc().settings();
        let settings_changed = settings_after_redo != settings_before_redo;

        let mut effects = vec![
            Effect::FullLayoutInvalidation,
            Effect::DocChanged,
            Effect::SelectionChanged,
            Effect::PendingStylesChanged,
        ];
        if settings_changed {
            effects.push(Effect::SettingsChanged);
        }
        effects
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
