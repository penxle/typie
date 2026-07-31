use std::collections::VecDeque;

use crate::NodeType;

use super::{ContentExpr, ContextExpr, Schema};

/// Schema decisions for an ordered direct-child sequence.
///
/// `consumed` counts schema-known (non-`Unknown`) children in the greedily
/// accepted prefix. `first_residue` indexes the original `children` slice, so
/// callers can retain their own representation-specific child indexing.
/// `completion_insertions` uses sequential physical child indices: each index
/// addresses the sequence after the preceding insertions have been applied;
/// `None` means insertion alone cannot complete the supplied sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentPlacement {
    pub consumed: usize,
    pub first_residue: Option<usize>,
    pub completion_insertions: Option<Vec<CompletionInsertion>>,
}

impl ContentPlacement {
    /// Whether the supplied children are a schema-valid complete sequence.
    pub fn is_valid(&self) -> bool {
        self.first_residue.is_none()
            && self
                .completion_insertions
                .as_ref()
                .is_some_and(Vec::is_empty)
    }

    /// Whether the supplied children can become valid by inserting only
    /// schema-required scaffolds.
    pub fn is_completable(&self) -> bool {
        self.first_residue.is_none() && self.completion_insertions.is_some()
    }
}

/// A required direct child and its sequential physical insertion index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionInsertion {
    pub index: usize,
    pub node_type: NodeType,
}

/// The schema action for a child already known to be a placement misfit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MisfitAction {
    /// Wrap the child in the declaration-order minimal schema chain. The first
    /// role is the outermost wrapper.
    Wrap { chain: Vec<NodeType> },
    /// Split the current container at the child and hoist it one level.
    SplitHoist,
}

/// The next representation-neutral repair transition for one ordered child
/// sequence. Callers own the actual tree mutation and identity policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepairTransition {
    Ready {
        completion: Vec<CompletionInsertion>,
    },
    Wrap {
        index: usize,
        chain: Vec<NodeType>,
    },
    SplitHoist {
        index: usize,
        retained_completion: Vec<CompletionInsertion>,
    },
}

pub fn context_allows(path: &[NodeType], t: NodeType) -> bool {
    let ctx = &Schema::node_spec(t).context;
    if *ctx == ContextExpr::Any {
        return true;
    }
    let full: Vec<NodeType> = path.iter().copied().chain(std::iter::once(t)).collect();
    ctx.matches(&full)
}

/// Analyze `children` against `parent`'s ordered content expression.
///
/// `Unknown` is transparent: it consumes no slot and can never be residue.
/// Matching is deliberately greedy, mirroring projection repair. A sequence
/// that is already valid reports no completion even when a repeatable group
/// could greedily consume a trailing required role.
pub fn content_placement(parent: NodeType, children: &[NodeType]) -> ContentPlacement {
    let known: Vec<(usize, NodeType)> = children
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, child)| *child != NodeType::Unknown)
        .collect();
    let types: Vec<NodeType> = known.iter().map(|(_, child)| *child).collect();
    let content = &Schema::node_spec(parent).content;
    let mut consumed = 0;
    consume_content(content, &types, &mut consumed);
    let completion_insertions = content.completion_insertions(&types).map(|insertions| {
        insertions
            .into_iter()
            .enumerate()
            .map(|(inserted, (filtered_index, node_type))| {
                let original_known_index = filtered_index - inserted;
                let physical_index = known
                    .get(original_known_index)
                    .map(|(index, _)| *index)
                    .unwrap_or(children.len())
                    + inserted;
                CompletionInsertion {
                    index: physical_index,
                    node_type,
                }
            })
            .collect()
    });

    ContentPlacement {
        consumed,
        first_residue: known.get(consumed).map(|(index, _)| *index),
        completion_insertions,
    }
}

fn consume_content(content: &ContentExpr, children: &[NodeType], consumed: &mut usize) {
    let matches_at = |content: &ContentExpr, index: usize| {
        children
            .get(index)
            .is_some_and(|child| content.matches(*child))
    };

    match content {
        ContentExpr::Empty => {}
        ContentExpr::Any => *consumed = children.len(),
        ContentExpr::Single(expected) => {
            if children.get(*consumed) == Some(expected) {
                *consumed += 1;
            }
        }
        ContentExpr::Optional(inner) => {
            if matches_at(inner, *consumed) {
                consume_content(inner, children, consumed);
            }
        }
        ContentExpr::ZeroOrMore(inner) | ContentExpr::OneOrMore(inner) => {
            while matches_at(inner, *consumed) {
                consume_content(inner, children, consumed);
            }
        }
        ContentExpr::Choice(choices) => {
            if let Some(choice) = choices.iter().find(|choice| matches_at(choice, *consumed)) {
                consume_content(choice, children, consumed);
            }
        }
        ContentExpr::Seq(exprs) => {
            for expr in exprs {
                consume_content(expr, children, consumed);
            }
        }
    }
}

// Declared content-expr order, not enum order: the chain tie-break is defined by
// declaration order, which differs from NodeType's enum order (e.g. Root content).
fn accepted_in_declared_order(parent: NodeType) -> Vec<NodeType> {
    fn walk(e: &ContentExpr, out: &mut Vec<NodeType>) {
        match e {
            ContentExpr::Empty | ContentExpr::Any => {}
            ContentExpr::Single(t) => {
                if !out.contains(t)
                    && !matches!(t, NodeType::Unknown)
                    && !Schema::node_spec(*t).inline
                {
                    out.push(*t);
                }
            }
            ContentExpr::Seq(es) | ContentExpr::Choice(es) => es.iter().for_each(|e| walk(e, out)),
            ContentExpr::ZeroOrMore(e) | ContentExpr::OneOrMore(e) | ContentExpr::Optional(e) => {
                walk(e, out)
            }
        }
    }
    let mut out = Vec::new();
    walk(&Schema::node_spec(parent).content, &mut out);
    out
}

pub fn wrap_chain(path: &[NodeType], child: NodeType) -> Option<Vec<NodeType>> {
    let parent = *path.last().expect("path includes parent");
    if Schema::node_spec(parent).content.matches(child) && context_allows(path, child) {
        return Some(vec![]);
    }
    let mut queue: VecDeque<Vec<NodeType>> = VecDeque::from([vec![]]);
    while let Some(chain) = queue.pop_front() {
        if chain.len() >= 4 {
            continue;
        }
        let tip = chain.last().copied().unwrap_or(parent);
        for w in accepted_in_declared_order(tip) {
            if chain.contains(&w) {
                continue;
            }
            let mut p: Vec<NodeType> = path.to_vec();
            p.extend_from_slice(&chain);
            if !context_allows(&p, w) {
                continue;
            }
            let mut next = chain.clone();
            next.push(w);
            p.push(w);
            if Schema::node_spec(w).content.matches(child) && context_allows(&p, child) {
                return Some(next);
            }
            queue.push_back(next);
        }
    }
    None
}

/// Choose the next lossless schema action for a child already identified as a
/// direct-child misfit at `path`.
pub fn misfit_action(path: &[NodeType], child: NodeType) -> MisfitAction {
    match wrap_chain(path, child) {
        Some(chain) if !chain.is_empty() => MisfitAction::Wrap { chain },
        Some(_) | None => MisfitAction::SplitHoist,
    }
}

/// Choose the next lossless repair transition for `children` under `path`.
///
/// `path` includes the destination parent. `can_wrap` lets a representation
/// block a schema-valid WRAP for identity/cycle reasons while leaving the final
/// WRAP-versus-SPLIT decision in this shared policy.
pub fn repair_transition(
    path: &[NodeType],
    children: &[NodeType],
    mut can_wrap: impl FnMut(usize) -> bool,
) -> Option<RepairTransition> {
    let parent = *path.last()?;
    let placement = content_placement(parent, children);
    let context_misfit = (parent != NodeType::Unknown)
        .then(|| {
            children
                .iter()
                .position(|child| *child != NodeType::Unknown && !context_allows(path, *child))
        })
        .flatten();
    let first_misfit = match (context_misfit, placement.first_residue) {
        (Some(context), Some(content)) => Some(context.min(content)),
        (context, content) => context.or(content),
    };
    let Some(index) = first_misfit else {
        return Some(RepairTransition::Ready {
            completion: placement.completion_insertions?,
        });
    };

    let child = *children.get(index)?;
    if let MisfitAction::Wrap { chain } = misfit_action(path, child)
        && !chain.is_empty()
        && can_wrap(index)
    {
        return Some(RepairTransition::Wrap { index, chain });
    }

    let retained_completion = content_placement(parent, &children[..index]).completion_insertions?;
    Some(RepairTransition::SplitHoist {
        index,
        retained_completion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeType;
    use strum::IntoEnumIterator;

    #[test]
    fn wrap_chain_derives_from_schema() {
        let root = &[NodeType::Root][..];
        assert_eq!(
            wrap_chain(&[NodeType::Root, NodeType::Paragraph], NodeType::Text),
            Some(vec![])
        );
        assert_eq!(
            wrap_chain(&[NodeType::Root, NodeType::BulletList], NodeType::Text),
            Some(vec![NodeType::ListItem, NodeType::Paragraph])
        );
        assert_eq!(
            wrap_chain(&[NodeType::Root, NodeType::BulletList], NodeType::Paragraph),
            Some(vec![NodeType::ListItem])
        );
        assert_eq!(
            wrap_chain(root, NodeType::TableCell),
            Some(vec![NodeType::Table, NodeType::TableRow])
        );
        assert_eq!(
            wrap_chain(root, NodeType::FoldTitle),
            Some(vec![NodeType::Fold])
        );
        assert_eq!(
            wrap_chain(&[NodeType::Root, NodeType::BulletList], NodeType::Fold),
            None
        );
        assert_eq!(
            wrap_chain(
                &[
                    NodeType::Root,
                    NodeType::Table,
                    NodeType::TableRow,
                    NodeType::TableCell
                ],
                NodeType::TableRow
            ),
            None
        );
        assert_eq!(
            wrap_chain(&[NodeType::Root, NodeType::Blockquote], NodeType::PageBreak),
            None
        );
        assert_eq!(
            wrap_chain(root, NodeType::PageBreak),
            Some(vec![NodeType::Paragraph])
        );
    }

    #[test]
    fn list_item_consumes_multiple_paragraphs_and_nested_list() {
        assert_eq!(
            content_placement(
                NodeType::ListItem,
                &[
                    NodeType::Paragraph,
                    NodeType::Paragraph,
                    NodeType::BulletList,
                ],
            ),
            ContentPlacement {
                consumed: 3,
                first_residue: None,
                completion_insertions: Some(vec![]),
            }
        );
    }

    #[test]
    fn text_under_list_wraps_in_declaration_order_minimal_chain() {
        assert_eq!(
            misfit_action(&[NodeType::Root, NodeType::BulletList], NodeType::Text),
            MisfitAction::Wrap {
                chain: vec![NodeType::ListItem, NodeType::Paragraph],
            }
        );
    }

    #[test]
    fn page_break_under_root_requires_paragraph_wrapper() {
        assert_eq!(
            misfit_action(&[NodeType::Root], NodeType::PageBreak),
            MisfitAction::Wrap {
                chain: vec![NodeType::Paragraph],
            }
        );
    }

    #[test]
    fn page_break_inside_blockquote_split_hoists() {
        assert_eq!(
            misfit_action(
                &[NodeType::Root, NodeType::Blockquote, NodeType::Paragraph,],
                NodeType::PageBreak,
            ),
            MisfitAction::SplitHoist
        );
    }

    #[test]
    fn task2_split_hoist_reports_required_completion_for_retained_prefix() {
        assert_eq!(
            repair_transition(
                &[NodeType::Root, NodeType::BulletList, NodeType::ListItem],
                &[NodeType::HorizontalRule],
                |_| true,
            ),
            Some(RepairTransition::SplitHoist {
                index: 0,
                retained_completion: vec![CompletionInsertion {
                    index: 0,
                    node_type: NodeType::Paragraph,
                }],
            })
        );
    }

    #[test]
    fn empty_table_requires_direct_row() {
        assert_eq!(
            content_placement(NodeType::Table, &[]),
            ContentPlacement {
                consumed: 0,
                first_residue: None,
                completion_insertions: Some(vec![CompletionInsertion {
                    index: 0,
                    node_type: NodeType::TableRow,
                }]),
            }
        );
    }

    #[test]
    fn valid_sequence_with_unknowns_has_no_residue_or_completion() {
        assert_eq!(
            content_placement(
                NodeType::ListItem,
                &[
                    NodeType::Unknown,
                    NodeType::Paragraph,
                    NodeType::Unknown,
                    NodeType::OrderedList,
                    NodeType::Unknown,
                ],
            ),
            ContentPlacement {
                consumed: 2,
                first_residue: None,
                completion_insertions: Some(vec![]),
            }
        );
    }

    #[test]
    fn fold_completion_maps_filtered_index_around_unknown() {
        assert_eq!(
            content_placement(NodeType::Fold, &[NodeType::Unknown, NodeType::FoldContent],)
                .completion_insertions,
            Some(vec![CompletionInsertion {
                index: 1,
                node_type: NodeType::FoldTitle,
            }])
        );
    }

    #[test]
    fn fold_completion_orders_multiple_insertions_after_unknowns() {
        assert_eq!(
            content_placement(NodeType::Fold, &[]).completion_insertions,
            Some(vec![
                CompletionInsertion {
                    index: 0,
                    node_type: NodeType::FoldTitle,
                },
                CompletionInsertion {
                    index: 1,
                    node_type: NodeType::FoldContent,
                },
            ])
        );
        assert_eq!(
            content_placement(NodeType::Fold, &[NodeType::Unknown]).completion_insertions,
            Some(vec![
                CompletionInsertion {
                    index: 1,
                    node_type: NodeType::FoldTitle,
                },
                CompletionInsertion {
                    index: 2,
                    node_type: NodeType::FoldContent,
                },
            ])
        );
    }

    #[test]
    fn valid_root_repeatable_content_and_trailing_paragraph_needs_no_completion() {
        assert_eq!(
            content_placement(NodeType::Root, &[NodeType::Image, NodeType::Paragraph]),
            ContentPlacement {
                consumed: 2,
                first_residue: None,
                completion_insertions: Some(vec![]),
            }
        );
    }

    #[test]
    fn every_node_type_is_placeable_from_root() {
        for t in NodeType::iter() {
            if matches!(t, NodeType::Root | NodeType::Unknown) {
                continue;
            }
            assert!(
                wrap_chain(&[NodeType::Root], t).is_some(),
                "{t:?}는 Root로부터 context-유효 배치 체인이 없다 — NodeSpec 선언을 확인하라"
            );
        }
    }
}
