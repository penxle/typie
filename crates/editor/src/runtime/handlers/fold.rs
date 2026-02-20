use crate::model::{Node, NodeId};
use crate::runtime::{Effect, Runtime};
use crate::state::{Selection, leaf_block_end};

impl Runtime {
    fn remap_selection_out_of_fold_content(&self, fold_id: NodeId) -> Option<Selection> {
        let fold = self.doc().node(fold_id)?;
        let mut fold_title_id = None;
        let mut fold_content_id = None;
        for child in fold.children() {
            match child.node() {
                Node::FoldTitle(_) => fold_title_id = Some(child.node_id()),
                Node::FoldContent(_) => fold_content_id = Some(child.node_id()),
                _ => {}
            }
        }

        let fold_title_id = fold_title_id?;
        let fold_content_id = fold_content_id?;

        let selection = self.state.selection;
        let fold_title = self.doc().node(fold_title_id)?;
        let target = leaf_block_end(&fold_title);

        let is_in_fold_content = |node_id: NodeId| {
            node_id == fold_content_id || self.doc().is_ancestor(fold_content_id, node_id)
        };
        let anchor_in_content = is_in_fold_content(selection.anchor.node_id);
        let head_in_content = is_in_fold_content(selection.head.node_id);

        if !anchor_in_content && !head_in_content {
            return None;
        }

        let anchor = if anchor_in_content {
            target
        } else {
            selection.anchor
        };
        let head = if head_in_content {
            target
        } else {
            selection.head
        };

        Some(Selection::new(anchor, head))
    }

    pub(crate) fn toggle_view_state(&mut self, node_id: NodeId) -> Vec<Effect> {
        let current_expanded = self.layout_engine.fold_expanded(node_id);

        let mut effects = if current_expanded {
            self.remap_selection_out_of_fold_content(node_id)
                .map_or_else(Vec::new, |next_selection| {
                    self.transact(move |tr| {
                        tr.set_selection(next_selection);
                        tr.set_preferred_x(None);
                        Ok(true)
                    })
                })
        } else {
            Vec::new()
        };

        self.layout_engine
            .set_fold_state(node_id, !current_expanded);

        effects.push(Effect::SubtreeChanged { node_id });
        effects
    }

    pub(crate) fn handle_insert_fold(&mut self) -> Vec<Effect> {
        let mut created_fold_id = None;
        let mut effects = self.transact(|tr| {
            let fold_id = tr.insert_fold()?;
            created_fold_id = fold_id;
            Ok(fold_id.is_some())
        });

        if let Some(fold_id) = created_fold_id {
            effects.extend(self.toggle_view_state(fold_id));
        }

        effects
    }

    pub(crate) fn handle_unwrap_fold(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.unwrap_fold())
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::click_fold_toggle;

    #[test]
    fn close_fold_moves_selection_from_content_to_fold_title_end() {
        let mut fold = id!();
        let mut title = id!();
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @fold fold {
                    @title fold_title {
                        text { "Title" }
                    }
                    fold_content {
                        @p paragraph {
                            text { "Hello" }
                        }
                    }
                }
            }
            selection { (p, 5) }
        };

        click_fold_toggle(&mut rt, fold);
        click_fold_toggle(&mut rt, fold);

        let selection = rt.selection();
        assert!(selection.is_collapsed());
        assert_eq!(selection.head.node_id, title);
        assert_eq!(selection.head.offset, 5);
    }

    #[test]
    fn close_fold_keeps_selection_when_outside_content() {
        let mut p1 = id!();
        let mut fold = id!();
        let mut title = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "Outside" }
                }
                @fold fold {
                    @title fold_title {
                        text { "Title" }
                    }
                    fold_content {
                        paragraph {
                            text { "Inside" }
                        }
                    }
                }
            }
            selection { (p1, 7) }
        };

        click_fold_toggle(&mut rt, fold);
        click_fold_toggle(&mut rt, fold);

        let selection = rt.selection();
        assert!(selection.is_collapsed());
        assert_eq!(selection.head.node_id, p1);
        assert_eq!(selection.head.offset, 7);
        assert_ne!(selection.head.node_id, title);
    }

    #[test]
    fn close_fold_moves_only_endpoint_inside_content() {
        let mut p1 = id!();
        let mut fold = id!();
        let mut title = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "Outside" }
                }
                @fold fold {
                    @title fold_title {
                        text { "Title" }
                    }
                    fold_content {
                        @p2 paragraph {
                            text { "Inside" }
                        }
                    }
                }
            }
            selection { (p1, 3) -> (p2, 2) }
        };

        click_fold_toggle(&mut rt, fold);
        click_fold_toggle(&mut rt, fold);

        let selection = rt.selection();
        assert!(!selection.is_collapsed());
        assert_eq!(selection.anchor.node_id, p1);
        assert_eq!(selection.anchor.offset, 3);
        assert_eq!(selection.head.node_id, title);
        assert_eq!(selection.head.offset, 5);
    }
}
