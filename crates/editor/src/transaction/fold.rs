use crate::model::{FoldContentNode, FoldNode, FoldTitleNode, Node, NodeId, NodeType};
use crate::state::position_helpers::leaf_block_start;
use crate::state::{
    Position, Selection, collect_top_level_blocks_in_range, selected_single_block_id,
};
use crate::transaction::Transaction;
use crate::types::Affinity;
use anyhow::{Context, Result};

impl Transaction {
    pub fn insert_fold(&mut self) -> Result<Option<NodeId>> {
        let original_selection = self.selection().clone();

        let target = self.expand_selection_until(|parent, blocks| {
            let parent_type = parent.as_type();
            let parent_spec = self.doc().schema().node_spec(parent_type);
            let fold_type = NodeType::Fold;

            if !parent_spec.content.matches(fold_type) {
                return false;
            }

            let fold_content_type = NodeType::FoldContent;
            let fold_content_spec = self.doc().schema().node_spec(fold_content_type);
            let block_types: Vec<NodeType> = blocks
                .iter()
                .filter_map(|id| self.node(*id).and_then(|n| n.node().map(|n| n.as_type())))
                .collect();

            block_types
                .iter()
                .all(|t| fold_content_spec.content.matches(*t))
        })?;

        if let Some(target) = target {
            self.set_selection(target);
            if let Some(fold_id) = self.wrap_in_fold()? {
                return Ok(Some(fold_id));
            }
            self.set_selection(original_selection);
        }

        Ok(None)
    }

    fn wrap_in_fold(&mut self) -> Result<Option<NodeId>> {
        let selection = self.selection().clone();
        let (from, to) = selection.as_sorted(self.doc())?;

        let block_ids = collect_top_level_blocks_in_range(self.doc(), from, to)?;
        if block_ids.is_empty() {
            return Ok(None);
        }

        let first_block = self.node(block_ids[0]).context("First block not found")?;
        let parent = first_block.parent().context("Block has no parent")?;
        let parent_id = parent.node_id();

        for &block_id in &block_ids[1..] {
            let block = self.node(block_id).context("Block not found")?;
            if block.parent().map(|p| p.node_id()) != Some(parent_id) {
                return Ok(None);
            }
        }

        let parent_spec = parent.spec().context("Parent spec not found")?;
        let fold_type = NodeType::Fold;
        if !parent_spec.content.matches(fold_type) {
            return Ok(None);
        }

        let first_block_index = first_block.index().context("First block has no index")?;

        let fold_id = NodeId::new();
        let fold_title_id = NodeId::new();
        let fold_content_id = NodeId::new();

        let parent_mut = self.node_mut(parent_id).context("Parent not found")?;
        parent_mut.as_mut().insert_child_with_id(
            first_block_index,
            fold_id,
            Node::Fold(FoldNode::default()),
        )?;

        let fold = self.node_mut(fold_id).context("Fold not found")?;
        fold.as_mut().insert_child_with_id(
            0,
            fold_title_id,
            Node::FoldTitle(FoldTitleNode::default()),
        )?;
        fold.as_mut().insert_child_with_id(
            1,
            fold_content_id,
            Node::FoldContent(FoldContentNode::default()),
        )?;

        self.move_node_range(
            block_ids[0],
            *block_ids.last().unwrap(),
            Some(fold_content_id),
            None,
            None,
        )?;

        self.set_selection(Selection::collapsed(Position::new(
            fold_title_id,
            0,
            Affinity::Downstream,
        )));

        for block_id in &block_ids {
            self.mark_attr_mutation(*block_id);
        }
        self.mark_attr_mutation(fold_id);

        Ok(Some(fold_id))
    }

    pub fn unwrap_fold(&mut self) -> Result<bool> {
        let Some(fold_id) = self.selected_or_ancestor_fold_id() else {
            return Ok(false);
        };

        let fold = self.node(fold_id).context("Fold not found")?;
        let parent = fold.parent().context("Fold parent not found")?;
        let parent_id = parent.node_id();
        let parent_prev = fold.prev_sibling().map(|n| n.node_id());

        let fold_content_id = fold
            .children()
            .find(|child| matches!(child.node(), Some(Node::FoldContent(_))))
            .map(|child| child.node_id())
            .context("FoldContent not found")?;

        let content_child_ids: Vec<NodeId> = self
            .node(fold_content_id)
            .context("FoldContent node not found")?
            .children()
            .map(|child| child.node_id())
            .collect();

        if let (Some(first), Some(last)) = (
            content_child_ids.first().copied(),
            content_child_ids.last().copied(),
        ) {
            self.move_node_range(first, last, Some(parent_id), parent_prev, Some(fold_id))?;
        }

        self.delete_node_recursive(fold_id)?;

        let new_selection = if let Some(first_unwrapped) = content_child_ids.first().copied() {
            let first_node = self
                .node(first_unwrapped)
                .context("First unwrapped node not found")?;
            Selection::collapsed(
                leaf_block_start(&first_node).context("Cannot find leaf block start")?,
            )
        } else {
            let fallback_offset = parent_prev
                .and_then(|prev_id| self.node(prev_id).and_then(|n| n.index()))
                .map(|idx| idx + 1)
                .unwrap_or(0);

            Selection::collapsed(Position::new(
                parent_id,
                fallback_offset,
                Affinity::Downstream,
            ))
        };

        self.set_selection(new_selection);
        Ok(true)
    }

    fn selected_or_ancestor_fold_id(&self) -> Option<NodeId> {
        let selection = self.selection().clone();

        if let Some(node_id) = selected_single_block_id(self.doc(), &selection) {
            if self
                .node(node_id)
                .map(|node| matches!(node.node(), Some(Node::Fold(_))))
                .unwrap_or(false)
            {
                return Some(node_id);
            }
        }

        let head_fold = self.node(selection.head.node_id).and_then(|node| {
            node.ancestors()
                .find(|ancestor| matches!(ancestor.node(), Some(Node::Fold(_))))
                .map(|ancestor| ancestor.node_id())
        });

        if selection.is_collapsed() {
            return head_fold;
        }

        let anchor_fold = self.node(selection.anchor.node_id).and_then(|node| {
            node.ancestors()
                .find(|ancestor| matches!(ancestor.node(), Some(Node::Fold(_))))
                .map(|ancestor| ancestor.node_id())
        });

        if head_fold.is_some() && head_fold == anchor_fold {
            head_fold
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn insert_fold_wraps_paragraph() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                paragraph { text { "world" } }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial, |tr| tr.insert_fold().unwrap());

        let mut fold_title_id = id!();
        let expected = state! {
            doc {
                fold {
                    @fold_title_id fold_title {}
                    fold_content {
                        @p1 paragraph { text { "hello" } }
                    }
                }
                paragraph { text { "world" } }
            }
            selection { (fold_title_id, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_fold_wraps_multiple_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                @p2 paragraph { text { "world" } }
                paragraph { text { "!" } }
            }
            selection { (p1, 0) -> (p2, 5) }
        };

        let actual = transact!(initial, |tr| tr.insert_fold().unwrap());

        let mut fold_title_id = id!();
        let expected = state! {
            doc {
                fold {
                    @fold_title_id fold_title {}
                    fold_content {
                        @p1 paragraph { text { "hello" } }
                        @p2 paragraph { text { "world" } }
                    }
                }
                paragraph { text { "!" } }
            }
            selection { (fold_title_id, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_fold_wraps_in_list() {
        let mut n1 = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        paragraph { text { "hello" } }
                        bullet_list {
                            list_item {
                                @n1 paragraph { text { "world" } }
                            }
                        }
                    }
                }
                paragraph { text { "!" } }
            }
            selection { (n1, 0) }
        };

        let actual = transact!(initial, |tr| tr.insert_fold().unwrap());

        let expected = state! {
            doc {
                fold {
                    @n1 fold_title { }
                    fold_content {
                        bullet_list {
                            list_item {
                                paragraph { text { "hello" } }
                                bullet_list {
                                    list_item {
                                        paragraph { text { "world" } }
                                    }
                                }
                            }
                        }
                    }
                }
                paragraph { text { "!" } }
            }
            selection { (n1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn unwrap_fold_from_fold_title_selection() {
        let mut title = id!();
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                paragraph { text { "before" } }
                fold {
                    @title fold_title {}
                    fold_content {
                        @p1 paragraph { text { "hello" } }
                        @p2 paragraph { text { "world" } }
                    }
                }
                paragraph { text { "after" } }
            }
            selection { (title, 0) }
        };

        let actual = transact!(initial, |tr| tr.unwrap_fold().unwrap());

        let expected = state! {
            doc {
                paragraph { text { "before" } }
                @p1 paragraph { text { "hello" } }
                @p2 paragraph { text { "world" } }
                paragraph { text { "after" } }
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn unwrap_fold_from_selected_fold_node() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                paragraph { text { "before" } }
                fold {
                    fold_title {}
                    fold_content {
                        @p1 paragraph { text { "hello" } }
                        @p2 paragraph { text { "world" } }
                    }
                }
                paragraph { text { "after" } }
            }
            selection { (p1, 0) -> (p1, 0) }
        };

        let actual = transact!(initial, |tr| tr.unwrap_fold().unwrap());

        let expected = state! {
            doc {
                paragraph { text { "before" } }
                @p1 paragraph { text { "hello" } }
                @p2 paragraph { text { "world" } }
                paragraph { text { "after" } }
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }
}
