use super::super::{Effect, Runtime};
use crate::layout::cursor::{Cursor, NavigationContext};
use crate::state::Selection;

impl Runtime {
    pub(crate) fn handle_delete_backward(&mut self) -> Vec<Effect> {
        self.transact(|tr| {
            if tr.delete_selection()? {
                return Ok(true);
            }
            if tr.delete_text_backward()? {
                return Ok(true);
            }
            if tr.join_backward()? {
                return Ok(true);
            }
            if tr.merge_list_item_backward()? {
                return Ok(true);
            }
            if tr.lift_list_item()? {
                return Ok(true);
            }
            tr.lift()
        })
    }

    pub(crate) fn handle_delete_forward(&mut self) -> Vec<Effect> {
        self.transact(|tr| {
            if tr.delete_selection()? {
                return Ok(true);
            }
            if tr.delete_text_forward()? {
                return Ok(true);
            }
            if tr.merge_list_item_forward()? {
                return Ok(true);
            }
            tr.join_forward()
        })
    }

    pub(crate) fn handle_delete_word_backward(&mut self) -> Vec<Effect> {
        if !self.state.selection.is_collapsed() {
            return self.transact(|tr| tr.delete_selection());
        }

        let ctx = NavigationContext::new(&self.state.doc);
        let Some((_, rect)) = Cursor::bounds(&ctx, &self.pages, self.state.selection.head) else {
            return vec![];
        };
        let preferred_y = rect.y;

        let Some(end_selection) =
            Cursor::move_word_left(&ctx, &self.pages, self.state.selection.head, preferred_y)
        else {
            return vec![];
        };
        let end_position = end_selection.head;

        if end_position.node_id != self.state.selection.head.node_id {
            return vec![];
        }

        let selection = self.state.selection;
        self.transact(move |tr| {
            tr.set_selection(Selection::new(end_position, selection.head));
            tr.delete_selection()
        })
    }

    pub(crate) fn handle_delete_word_forward(&mut self) -> Vec<Effect> {
        if !self.state.selection.is_collapsed() {
            return self.transact(|tr| tr.delete_selection());
        }

        let ctx = NavigationContext::new(&self.state.doc);
        let Some((_, rect)) = Cursor::bounds(&ctx, &self.pages, self.state.selection.head) else {
            return vec![];
        };
        let preferred_y = rect.y + rect.height;

        let Some(end_selection) =
            Cursor::move_word_right(&ctx, &self.pages, self.state.selection.head, preferred_y)
        else {
            return vec![];
        };
        let end_position = end_selection.head;

        let selection = self.state.selection;
        self.transact(move |tr| {
            tr.set_selection(Selection::new(selection.head, end_position));
            tr.delete_selection()
        })
    }

    pub(crate) fn handle_delete_to_line_start(&mut self) -> Vec<Effect> {
        if !self.state.selection.is_collapsed() {
            return self.transact(|tr| tr.delete_selection());
        }

        let ctx = NavigationContext::new(&self.state.doc);
        let Some(line_start_selection) =
            Cursor::move_to_line_start(&ctx, &self.pages, self.state.selection.head)
        else {
            return vec![];
        };
        let line_start = line_start_selection.head;

        let selection = self.state.selection;
        self.transact(move |tr| {
            tr.set_selection(Selection::new(line_start, selection.head));
            tr.delete_selection()
        })
    }

    pub(crate) fn handle_delete_node(&mut self, node_id: String) -> Vec<Effect> {
        use crate::model::{Node, NodeId};

        let Some(node_id) = NodeId::from_string(&node_id) else {
            return vec![];
        };

        // TODO: 다른 external 추가되면 수정
        let is_image = self
            .doc()
            .node(node_id)
            .map(|n| matches!(n.node(), Node::Image(_)))
            .unwrap_or(false);

        self.transact(move |tr| {
            if is_image {
                // TODO: delete_node_recursive 에서 처리하는 게 맞나..
                tr.push_effect(Effect::ExternalElementChanged);
            }

            tr.delete_node_with_selection_adjustment(node_id)?;

            Ok(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        model::CalloutType,
        runtime::{Effect, Message},
    };

    #[test]
    fn test_delete_word_backward_after_horizontal_rule_does_nothing() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                horizontal_rule {}
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) }
        };

        rt.layout();
        rt.update(Message::DeleteWordBackward);

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, p);
        assert_eq!(selection.anchor.offset, 0);
        assert_eq!(selection.head.node_id, p);
        assert_eq!(selection.head.offset, 0);
    }

    #[test]
    fn test_delete_backward_at_start_of_first_paragraph_in_fold_does_nothing() {
        let mut n1 = id!();
        let mut fold_id = id!();

        let initial = state! {
            doc {
                paragraph {}
                @fold_id fold {
                    fold_title {
                        text { "title" }
                    }
                    fold_content {
                        @n1 paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (n1, 0) }
        };

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph {}
                @fold_id fold {
                    fold_title {
                        text { "title" }
                    }
                    fold_content {
                        @n1 paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (n1, 0) }
        };

        rt.layout();
        rt.update(Message::DeleteBackward);
        rt.tick();

        assert_state_eq!(*rt.state(), initial);
    }

    #[test]
    fn test_delete_selection_across_isolating_boundary_does_nothing() {
        let mut n1 = id!();
        let mut n2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @n1 paragraph {}
                fold {
                    @n2 fold_title {}
                    fold_content {
                        paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (n1, 0) -> (n2, 0) }
        };

        rt.layout();
        rt.update(Message::DeleteBackward);
        rt.tick();

        let expected = state! {
            doc {
                @n1 paragraph {}
                fold {
                    @n2 fold_title {}
                    fold_content {
                        paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (n1, 0) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn test_delete_selection_across_isolating_boundaries() {
        let mut n1 = id!();
        let mut n2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @n1 paragraph { text { "11" } }
                fold {
                    fold_title { text { "22" } }
                    fold_content {
                        @n2 paragraph { text { "33" } }
                    }
                }
                paragraph { text { "44" } }
            }
            selection { (n1, 1) -> (n2, 1) }
        };

        rt.layout();
        rt.update(Message::DeleteBackward);
        rt.tick();

        let expected = state! {
            doc {
                @n1 paragraph { text { "1" } }
                fold {
                    fold_title { }
                    fold_content {
                        paragraph { text { "3" } }
                    }
                }
                paragraph { text { "44" } }
            }
            selection { (n1, 1) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn test_delete_selection_across_isolating_boundaries_2() {
        let mut n1 = id!();
        let mut n2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph { text { "11" } }
                fold {
                    @n1 fold_title { text { "22" } }
                    fold_content {
                        paragraph { text { "33" } }
                    }
                }
                @n2 paragraph { text { "44" } }
            }
            selection { (n1, 1) -> (n2, 1) }
        };

        rt.layout();
        rt.update(Message::DeleteBackward);
        rt.tick();

        let expected = state! {
            doc {
                paragraph { text { "11" } }
                fold {
                    @n1 fold_title { text { "2" } }
                    fold_content {
                        paragraph {}
                    }
                }
                @n2 paragraph { text { "4" } }
            }
            selection { (n1, 1) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn test_delete_selection_across_isolating_end_boundary_with_whole_fold_content() {
        let mut n1 = id!();
        let mut n2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph { text { "11" } }
                fold {
                    fold_title { text { "22" } }
                    fold_content {
                        @n1 paragraph { text { "33" } }
                    }
                }
                @n2 paragraph { text { "44" } }
            }
            selection { (n1, 0) -> (n2, 1) }
        };

        rt.layout();
        rt.update(Message::DeleteBackward);
        rt.tick();

        let expected = state! {
            doc {
                paragraph { text { "11" } }
                fold {
                    fold_title { text { "22" } }
                    fold_content {
                        @n1 paragraph {}
                    }
                }
                @n2 paragraph { text { "4" } }
            }
            selection { (n1, 0) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn test_delete_selection_containing_whole_fold() {
        let mut n1 = id!();
        let mut n2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @n1 paragraph { text { "11" } }
                fold {
                    fold_title { text { "22" } }
                    fold_content {
                        paragraph { text { "33" } }
                    }
                }
                @n2 paragraph { text { "44" } }
            }
            selection { (n1, 1) -> (n2, 1) }
        };

        rt.layout();
        rt.update(Message::DeleteBackward);
        rt.tick();

        let expected = state! {
            doc {
                @n1 paragraph { text { "14" } }
            }
            selection { (n1, 1) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn test_delete_selection_containing_sub_fold() {
        let mut n1 = id!();
        let mut n2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph { text { "11" } }
                fold {
                    fold_title { text { "22" } }
                    fold_content {
                        @n1 paragraph { text { "33" } }
                        fold {
                            fold_title { text { "44" } }
                            fold_content {
                                paragraph { text { "55" } }
                            }
                        }
                    }
                }
                @n2 paragraph { text { "66" } }
            }
            selection { (n1, 1) -> (n2, 1) }
        };

        rt.layout();
        rt.update(Message::DeleteBackward);
        rt.tick();

        let expected = state! {
            doc {
                paragraph { text { "11" } }
                fold {
                    fold_title { text { "22" } }
                    fold_content {
                        @n1 paragraph { text { "3" } }
                    }
                }
                paragraph { text { "6" } }
            }
            selection { (n1, 1) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn test_delete_selection_fold_with_non_textblock() {
        let mut n1 = id!();
        let mut n2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                fold {
                    @n1 fold_title { text { "11" } }
                    fold_content {
                        paragraph { text { "22" } }
                        bullet_list {
                            list_item {
                                @n2 paragraph { text { "33" } }
                            }
                        }
                    }
                }
            }
            selection { (n1, 1) -> (n2, 1) }
        };

        rt.layout();
        rt.update(Message::DeleteBackward);
        rt.tick();

        let expected = state! {
            doc {
                fold {
                    @n1 fold_title { text { "1" } }
                    fold_content {
                        @n2 paragraph { text { "3" } }
                    }
                }
                paragraph {}
            }
            selection { (n1, 1) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn test_delete_selection_fold_with_nested_non_textblock() {
        let mut n1 = id!();
        let mut n2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                fold {
                    @n1 fold_title { text { "11" } }
                    fold_content {
                        bullet_list {
                            list_item {
                                paragraph { text { "33" } }
                                ordered_list {
                                    list_item {
                                        @n2 paragraph { text { "44" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            selection { (n1, 1) -> (n2, 1) }
        };

        rt.layout();
        rt.update(Message::DeleteBackward);
        rt.tick();

        let expected = state! {
            doc {
                fold {
                    @n1 fold_title { text { "1" } }
                    fold_content {
                        @n2 paragraph { text { "4" } }
                    }
                }
                paragraph {}
            }
            selection { (n1, 1) }
        };

        assert_state_eq!(*rt.state(), expected);
    }

    #[test]
    fn test_delete_selection_callout_with_two_lines() {
        let mut n1 = id!();
        let mut n2 = id!();
        let mut callout = id!();

        let initial = state! {
            doc {
                @n1 paragraph {
                    text { "outside" }
                }
                @callout callout(callout_type: CalloutType::Success,) {
                    @n2 paragraph {
                        text { "line 1" }
                    }
                    paragraph {
                        text { "line 2" }
                    }
                }
                paragraph {}
            }
            selection { (n1, 3) -> (n2, 5) }
        };

        let (actual, effects) = transact_with_effect!(initial, |tr| tr.delete_selection().unwrap());

        let has_item_changed = effects
            .iter()
            .any(|e| matches!(e, Effect::NodeChanged { node_id } if *node_id == callout));

        assert!(
            has_item_changed,
            "Effect::NodeChanged should be emitted for the callout"
        );

        let has_item_changed_2 = effects
            .iter()
            .any(|e| matches!(e, Effect::NodeChanged { node_id } if *node_id == n1));

        assert!(
            has_item_changed_2,
            "Effect::NodeChanged should be emitted for the paragraph"
        );

        let has_item_changed_3 = effects
            .iter()
            .any(|e| matches!(e, Effect::NodeChanged { node_id } if *node_id == n2));

        assert!(
            has_item_changed_3,
            "Effect::NodeChanged should be emitted for the paragraph"
        );

        let expected = state! {
            doc {
                @n1 paragraph {
                    text { "out1" }
                }
                callout(callout_type: CalloutType::Success,) {
                    paragraph {
                        text { "line 2" }
                    }
                }
                paragraph {}
            }
            selection { (n1, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn test_delete_selected_image_adjusts_selection() {
        use crate::model::NodeId;

        let mut p1 = id!();
        let mut img = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "before" } }
                @img image(src: Some("test.png".to_string()), width: Some(100.0), height: Some(100.0),) {}
                @p2 paragraph { text { "after" } }
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };

        rt.layout();

        let doc = rt.doc();
        assert!(doc.node(img).is_some());

        rt.handle_delete_node(img.to_string());

        let doc = rt.doc();
        assert!(doc.node(img).is_none());

        let selection = rt.selection();

        assert_eq!(selection.anchor.node_id, p2);
        assert_eq!(selection.anchor.offset, 0);
        assert!(selection.is_collapsed());
    }
}
