use editor_model::{
    CompletionInsertion, Fragment, NodeType, RepairTransition, content_placement, repair_transition,
};
use editor_transaction::minimal_subtree;

use crate::helpers::subtree_to_fragment;

/// Restructuring limit for one untrusted Slice fit. A fit either completes
/// within the work limit or returns `None`; valid ancestry depth is not itself
/// a semantic rejection reason.
const FIT_OP_BUDGET: usize = 4096;

enum DirectFit {
    Ready,
    Hoist(Vec<Fragment>),
}

/// Fit an owned fragment forest to its actual destination parent.
///
/// `parent_path` includes the destination parent. The same shared
/// `content_placement` / `misfit_action` decisions used by projection and
/// fulfillment drive this dot-less executor. An outer SPLIT-HOIST would cross
/// the destination boundary, so it rejects the whole forest. Internal hoists
/// remain lossless and are re-examined by the parent fragment.
pub(crate) fn fit_fragment_forest(
    fragments: Vec<Fragment>,
    parent_path: &[NodeType],
) -> Option<Vec<Fragment>> {
    if parent_path.is_empty() {
        return None;
    }
    let mut budget = FIT_OP_BUDGET;
    let mut path = parent_path.to_vec();
    let mut stack = vec![FragmentFitFrame {
        owner: None,
        children: fragments,
        complete: false,
        next_child: 0,
        entered: false,
    }];

    loop {
        let action = {
            let frame = stack.last_mut()?;
            let container = *path.last()?;
            if !frame.entered {
                if frame
                    .children
                    .iter()
                    .any(|child| child.node.as_type() == NodeType::Unknown)
                {
                    return None;
                }
                match fit_direct_children(
                    &mut frame.children,
                    &path,
                    &mut budget,
                    frame.complete,
                    container,
                    frame.owner.as_ref(),
                )? {
                    DirectFit::Ready => {
                        frame.entered = true;
                        FrameAction::Continue
                    }
                    DirectFit::Hoist(hoist) => FrameAction::Finish(hoist),
                }
            } else if frame.next_child < frame.children.len() {
                let child = frame.children.remove(frame.next_child);
                FrameAction::Descend(child)
            } else {
                let placement = content_placement(container, &fragment_types(&frame.children));
                let valid = if frame.complete {
                    placement.is_valid()
                } else {
                    placement.is_completable()
                };
                if !valid {
                    return None;
                }
                FrameAction::Finish(Vec::new())
            }
        };

        match action {
            FrameAction::Continue => {}
            FrameAction::Descend(mut child) => {
                let owner = fragment_shell(&child);
                path.push(owner.node.as_type());
                stack.push(FragmentFitFrame {
                    owner: Some(owner),
                    children: std::mem::take(&mut child.children),
                    complete: true,
                    next_child: 0,
                    entered: false,
                });
            }
            FrameAction::Finish(mut hoist) => loop {
                let mut completed = stack.pop()?;
                let Some(parent) = stack.last_mut() else {
                    return hoist.is_empty().then_some(completed.children);
                };
                path.pop()?;
                let mut owner = completed.owner.take()?;
                owner.children = completed.children;
                let index = parent.next_child;
                parent.children.insert(index, owner);
                if hoist.is_empty() {
                    parent.next_child += 1;
                    break;
                }
                parent.children.splice(index + 1..index + 1, hoist);
                let container = *path.last()?;
                match fit_direct_children(
                    &mut parent.children,
                    &path,
                    &mut budget,
                    parent.complete,
                    container,
                    parent.owner.as_ref(),
                )? {
                    DirectFit::Ready => {
                        // The retained child is already fitted. Any wrapper
                        // introduced around hoisted content starts after it.
                        parent.next_child += 1;
                        break;
                    }
                    DirectFit::Hoist(next) => hoist = next,
                }
            },
        }
    }
}

struct FragmentFitFrame {
    owner: Option<Fragment>,
    children: Vec<Fragment>,
    complete: bool,
    next_child: usize,
    entered: bool,
}

enum FrameAction {
    Continue,
    Descend(Fragment),
    Finish(Vec<Fragment>),
}

fn fit_direct_children(
    children: &mut Vec<Fragment>,
    path: &[NodeType],
    budget: &mut usize,
    complete: bool,
    container: NodeType,
    container_template: Option<&Fragment>,
) -> Option<DirectFit> {
    loop {
        let types = fragment_types(children);
        match repair_transition(path, &types, |_| true)? {
            RepairTransition::Ready { completion: needed } => {
                if complete {
                    insert_completion_fragments(children, &needed, budget)?;
                }
                return Some(DirectFit::Ready);
            }
            RepairTransition::Wrap { index, chain } => {
                spend(budget)?;
                wrap_fragment(children, index, &chain);
            }
            RepairTransition::SplitHoist {
                index,
                retained_completion,
            } => {
                if editor_model::Schema::node_spec(container).isolating {
                    return None;
                }
                spend(budget)?;
                return Some(DirectFit::Hoist(split_hoist_fragment(
                    container,
                    container_template,
                    children,
                    index,
                    &retained_completion,
                    budget,
                )?));
            }
        }
    }
}

fn spend(budget: &mut usize) -> Option<()> {
    if *budget == 0 {
        return None;
    }
    *budget -= 1;
    Some(())
}

fn insert_completion_fragments(
    children: &mut Vec<Fragment>,
    insertions: &[CompletionInsertion],
    budget: &mut usize,
) -> Option<()> {
    for insertion in insertions {
        spend(budget)?;
        children.insert(
            insertion.index,
            subtree_to_fragment(minimal_subtree(insertion.node_type)),
        );
    }
    Some(())
}

fn fragment_types(fragments: &[Fragment]) -> Vec<NodeType> {
    fragments
        .iter()
        .map(|fragment| fragment.node.as_type())
        .collect()
}

fn fragment_shell(fragment: &Fragment) -> Fragment {
    Fragment {
        node: fragment.node.clone(),
        modifiers: fragment.modifiers.clone(),
        carry: fragment.carry.clone(),
        children: Vec::new(),
    }
}

fn wrap_fragment(children: &mut Vec<Fragment>, i: usize, chain: &[NodeType]) {
    let mut current = children.remove(i);
    for &role in chain.iter().rev() {
        current = Fragment::leaf(role.into_node().to_plain()).with_children(vec![current]);
    }
    children.insert(i, current);
}

fn split_hoist_fragment(
    container: NodeType,
    container_template: Option<&Fragment>,
    children: &mut Vec<Fragment>,
    k: usize,
    retained_completion: &[CompletionInsertion],
    budget: &mut usize,
) -> Option<Vec<Fragment>> {
    let tail = children.split_off(k + 1);
    let promoted = children.pop().expect("k is a valid child index");
    insert_completion_fragments(children, retained_completion, budget)?;
    let mut out = vec![promoted];
    if !tail.is_empty() {
        let mut tail_container = container_template
            .cloned()
            .unwrap_or_else(|| Fragment::leaf(container.into_node().to_plain()));
        tail_container.children = tail;
        out.push(tail_container);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::{
        Alignment, BlockquoteVariant, Modifier, PlainBlockquoteNode, PlainBulletListNode,
        PlainFoldContentNode, PlainHorizontalRuleNode, PlainImageNode, PlainListItemNode,
        PlainNode, PlainPageBreakNode, PlainParagraphNode, PlainTableCellNode, PlainTextNode,
    };

    fn text(t: &str) -> Fragment {
        Fragment::leaf(PlainNode::Text(PlainTextNode { text: t.into() }))
    }

    fn page_break() -> Fragment {
        Fragment::leaf(PlainNode::PageBreak(PlainPageBreakNode::default()))
    }

    fn para(children: Vec<Fragment>) -> Fragment {
        Fragment::leaf(PlainNode::Paragraph(PlainParagraphNode::default())).with_children(children)
    }

    fn list_item(children: Vec<Fragment>) -> Fragment {
        Fragment::leaf(PlainNode::ListItem(PlainListItemNode::default())).with_children(children)
    }

    fn bullet_list(children: Vec<Fragment>) -> Fragment {
        Fragment::leaf(PlainNode::BulletList(PlainBulletListNode::default()))
            .with_children(children)
    }

    fn horizontal_rule() -> Fragment {
        Fragment::leaf(PlainNode::HorizontalRule(PlainHorizontalRuleNode::default()))
    }

    fn table_cell(children: Vec<Fragment>) -> Fragment {
        Fragment::leaf(PlainNode::TableCell(PlainTableCellNode::default())).with_children(children)
    }

    fn fold_content(children: Vec<Fragment>) -> Fragment {
        Fragment::leaf(PlainNode::FoldContent(PlainFoldContentNode::default()))
            .with_children(children)
    }

    fn blockquote(variant: BlockquoteVariant, children: Vec<Fragment>) -> Fragment {
        Fragment::leaf(PlainNode::Blockquote(PlainBlockquoteNode { variant }))
            .with_children(children)
    }

    fn types(fragments: &[Fragment]) -> Vec<NodeType> {
        fragments.iter().map(|f| f.node.as_type()).collect()
    }

    #[test]
    fn keeps_list_item_with_two_paragraphs() {
        let content = vec![bullet_list(vec![list_item(vec![
            para(vec![text("a")]),
            para(vec![text("b")]),
        ])])];

        let content = fit_fragment_forest(content, &[NodeType::Root]).expect("fits");

        assert_eq!(types(&content), vec![NodeType::BulletList]);
        let items = &content[0].children;
        assert_eq!(types(items), vec![NodeType::ListItem]);
        assert_eq!(
            items[0].children,
            vec![para(vec![text("a")]), para(vec![text("b")])]
        );
    }

    #[test]
    fn keeps_list_item_with_three_paragraphs() {
        let content = vec![bullet_list(vec![list_item(vec![
            para(vec![text("a")]),
            para(vec![text("b")]),
            para(vec![text("c")]),
        ])])];

        let content = fit_fragment_forest(content, &[NodeType::Root]).expect("fits");

        let items = &content[0].children;
        assert_eq!(types(items), vec![NodeType::ListItem]);
        assert_eq!(
            items[0].children,
            vec![
                para(vec![text("a")]),
                para(vec![text("b")]),
                para(vec![text("c")]),
            ]
        );
    }

    #[test]
    fn keeps_trailing_paragraph_after_nested_list_in_same_item() {
        let content = vec![bullet_list(vec![list_item(vec![
            para(vec![text("a")]),
            bullet_list(vec![list_item(vec![para(vec![text("x")])])]),
            para(vec![text("b")]),
        ])])];

        let content = fit_fragment_forest(content, &[NodeType::Root]).expect("fits");

        let items = &content[0].children;
        assert_eq!(types(items), vec![NodeType::ListItem]);
        assert_eq!(
            types(&items[0].children),
            vec![
                NodeType::Paragraph,
                NodeType::BulletList,
                NodeType::Paragraph,
            ]
        );
        assert_eq!(items[0].children[2], para(vec![text("b")]));
    }

    #[test]
    fn leaves_valid_list_unchanged() {
        let content = vec![bullet_list(vec![
            list_item(vec![para(vec![text("a")])]),
            list_item(vec![para(vec![text("b")])]),
        ])];
        let before = content.clone();

        let content = fit_fragment_forest(content, &[NodeType::Root]).expect("fits");

        assert_eq!(content, before);
    }

    #[test]
    fn repairs_the_earliest_content_or_context_misfit() {
        let children = vec![
            Fragment::leaf(PlainNode::Image(PlainImageNode::default())),
            page_break(),
        ];
        let path = [NodeType::Root, NodeType::Blockquote, NodeType::Paragraph];
        let transition =
            repair_transition(&path, &fragment_types(&children), |_| true).expect("repair");

        assert!(matches!(
            transition,
            RepairTransition::SplitHoist { index: 0, .. }
        ));
    }

    #[test]
    fn wraps_bare_inline_fragments_for_a_root_destination_without_loss() {
        let content = vec![text("a"), text("b")];

        let content = fit_fragment_forest(content, &[NodeType::Root]).expect("fits");

        assert_eq!(content, vec![para(vec![text("a")]), para(vec![text("b")])]);
    }

    #[test]
    fn keeps_page_break_in_top_level_paragraph() {
        let content = vec![para(vec![text("a"), page_break()])];
        let before = content.clone();

        let content = fit_fragment_forest(content, &[NodeType::Root]).expect("fits");

        assert_eq!(content, before);
    }

    #[test]
    fn task2_split_hoist_completes_the_retained_list_item() {
        let fitted = fit_fragment_forest(
            vec![bullet_list(vec![list_item(vec![horizontal_rule()])])],
            &[NodeType::Root],
        )
        .expect("horizontal rule can hoist without invalidating its retained wrappers");

        assert_eq!(
            types(&fitted),
            vec![NodeType::BulletList, NodeType::HorizontalRule]
        );
        assert_eq!(
            types(&fitted[0].children[0].children),
            vec![NodeType::Paragraph]
        );
        assert!(fitted[0].children[0].children[0].children.is_empty());
    }

    #[test]
    fn split_hoist_preserves_source_wrapper_metadata_on_the_tail() {
        let alignment = Modifier::Alignment {
            value: Alignment::Center,
        };
        let carry = Modifier::Bold;
        let mut source_paragraph = para(vec![text("a"), page_break(), text("b")]);
        source_paragraph.modifiers.push(alignment.clone());
        source_paragraph.carry.push(carry.clone());
        let source_blockquote = blockquote(
            BlockquoteVariant::MessageSent,
            vec![source_paragraph.clone()],
        );

        let fitted = fit_fragment_forest(vec![source_blockquote.clone()], &[NodeType::Root])
            .expect("the PageBreak can be admitted by splitting its wrappers at Root");

        assert_eq!(fitted.len(), 3);
        assert_eq!(fitted[0].node, source_blockquote.node);
        assert_eq!(fitted[2].node, source_blockquote.node);
        assert_eq!(fitted[0].children[0].modifiers, vec![alignment.clone()]);
        assert_eq!(fitted[0].children[0].carry, vec![carry.clone()]);
        assert_eq!(fitted[2].children[0].modifiers, vec![alignment]);
        assert_eq!(fitted[2].children[0].carry, vec![carry]);
    }

    #[test]
    fn split_hoist_does_not_cross_table_cell_isolation() {
        let content = vec![table_cell(vec![para(vec![
            text("a"),
            page_break(),
            text("b"),
        ])])];

        assert!(fit_fragment_forest(content, &[NodeType::Root]).is_none());
    }

    #[test]
    fn split_hoist_does_not_cross_fold_content_isolation() {
        let content = vec![fold_content(vec![para(vec![
            text("a"),
            page_break(),
            text("b"),
        ])])];

        assert!(fit_fragment_forest(content, &[NodeType::Root]).is_none());
    }

    #[test]
    fn wide_split_hoists_do_not_accumulate_same_level_stack_frames() {
        let content = (0..256)
            .map(|_| {
                blockquote(
                    BlockquoteVariant::LeftLine,
                    vec![para(vec![text("a")]), page_break()],
                )
            })
            .collect();
        let fitted = std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(move || fit_fragment_forest(content, &[NodeType::Root]))
            .expect("small-stack fitter thread")
            .join()
            .expect("wide fitting must not overflow its thread stack");

        assert!(fitted.is_some());
    }

    #[test]
    fn deeply_nested_valid_input_is_not_rejected_by_ancestry_depth() {
        let mut inner = para(vec![text("x")]);
        for _ in 0..512 {
            inner = bullet_list(vec![list_item(vec![inner])]);
        }
        let content = vec![inner];
        let fitted = std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(move || {
                let Some(mut fitted) = fit_fragment_forest(content, &[NodeType::Root]) else {
                    return false;
                };
                while let Some(mut fragment) = fitted.pop() {
                    fitted.append(&mut fragment.children);
                }
                true
            })
            .expect("small-stack fitter thread")
            .join()
            .expect("deep fitting must not overflow its thread stack");

        assert!(fitted);
    }
}
