use editor_clipboard::Slice;
use editor_model::{DocView, Fragment};
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::CommandError;
use crate::helpers::{
    block_boundary_fragments, build_inline_mode, can_splice_textblock, find_ancestor_textblock,
    fit_slice_for_textblock_parent, fragments_are_inline, fragments_fit_parent,
    insert_blocks_at_block_boundary, insert_blocks_in_textblock_at_position,
    insert_content_as_inline_at_position, is_insertable_inline_fragment,
    materialize_position_block, open_inline_content_for_textblock_insert, position_in_textblock,
    prepare_page_breaks_for_position, repair_slice_fragments, splice_emits_change,
    top_level_fragments,
};
use crate::types::SliceProvenance;

pub enum SliceInsertionPlan {
    DirectInline { fragments: Vec<Fragment> },
    SpliceBlocks { candidate: Slice },
    OpenInline { fragments: Vec<Fragment> },
    BlockBoundary { blocks: Vec<Fragment> },
}

/// 삽입 후보 판정의 3-state 결과. 카스케이드의 "실패"에는 두 가지 서로 다른
/// 원인이 있어 이를 구별한다: 슬라이스 자체가 비어 콘텐츠가 없는 경우(`Empty`)와,
/// 슬라이스는 비어있지 않지만 이 위치에 들어맞는 삽입 형태가 없는 경우(`NoFit`).
/// 호출부(`insert_slice_at_position`)는 `NoFit`에서만 노드 소멸 여부를 확인해
/// `NodeNotFound`를 방출할 수 있다 — `Empty`는 노드 상태와 무관하게 항상 no-op.
pub(crate) enum SliceInsertionOutcome {
    Plan(SliceInsertionPlan),
    Empty,
    NoFit,
}

pub(crate) fn resolve_slice_insertion_outcome(
    view: &DocView,
    position: Position,
    slice: Slice,
) -> SliceInsertionOutcome {
    if slice.is_empty() {
        return SliceInsertionOutcome::Empty;
    }
    let mut slice = prepare_page_breaks_for_position(view, &position, slice);
    if slice.is_empty() {
        return SliceInsertionOutcome::Empty;
    }
    repair_slice_fragments(&mut slice.content);

    if position_in_textblock(view, &position) {
        let top_level = top_level_fragments(&slice);
        let direct_inline = find_ancestor_textblock(view, position.node)
            .and_then(|id| view.node(id))
            .is_some_and(|textblock| {
                fragments_are_inline(&top_level)
                    && fragments_fit_parent(textblock.node_type(), &top_level)
            });
        if direct_inline {
            let fragments: Vec<Fragment> = top_level.into_iter().cloned().collect();
            if !fragments.iter().any(is_insertable_inline_fragment) {
                return SliceInsertionOutcome::NoFit;
            }
            return SliceInsertionOutcome::Plan(SliceInsertionPlan::DirectInline { fragments });
        }

        let parent_fitted = fit_slice_for_textblock_parent(view, &position, &slice);
        if let Some(candidate) = parent_fitted.as_ref()
            && can_splice_textblock(view, &position, candidate)
            && splice_emits_change(view, &position, candidate)
        {
            return SliceInsertionOutcome::Plan(SliceInsertionPlan::SpliceBlocks {
                candidate: candidate.clone(),
            });
        }

        let candidate = parent_fitted.as_ref().unwrap_or(&slice);
        if let Some(fragments) =
            open_inline_content_for_textblock_insert(view, &position, candidate)
        {
            let fragments: Vec<Fragment> = fragments.into_iter().cloned().collect();
            if !fragments.iter().any(is_insertable_inline_fragment) {
                return SliceInsertionOutcome::NoFit;
            }
            return SliceInsertionOutcome::Plan(SliceInsertionPlan::OpenInline { fragments });
        }
        SliceInsertionOutcome::NoFit
    } else {
        let Some(container) = view.node(position.node) else {
            return SliceInsertionOutcome::NoFit;
        };
        let Some(blocks) = block_boundary_fragments(&slice, container.node_type()) else {
            return SliceInsertionOutcome::NoFit;
        };
        SliceInsertionOutcome::Plan(SliceInsertionPlan::BlockBoundary { blocks })
    }
}

/// `Some(plan)` ⇒ `insert_slice_at_position`이 이 plan으로 삽입 op를 방출하는
/// 관측 가능한 변화 보장. plan은 변환(페이지브레이크 정리·수선) 완료된 콘텐츠를
/// 소유한다.
pub fn resolve_slice_insertion(
    view: &DocView,
    position: Position,
    slice: Slice,
) -> Option<SliceInsertionPlan> {
    match resolve_slice_insertion_outcome(view, position, slice) {
        SliceInsertionOutcome::Plan(plan) => Some(plan),
        SliceInsertionOutcome::Empty | SliceInsertionOutcome::NoFit => None,
    }
}

pub(crate) fn insert_slice_at_position(
    tr: &mut Transaction,
    position: Position,
    slice: Slice,
    provenance: SliceProvenance,
) -> Result<Option<Selection>, CommandError> {
    let outcome = {
        let view = tr.state().view();
        resolve_slice_insertion_outcome(&view, position, slice)
    };
    let plan = match outcome {
        SliceInsertionOutcome::Plan(plan) => plan,
        SliceInsertionOutcome::Empty => return Ok(None),
        SliceInsertionOutcome::NoFit => {
            let view = tr.state().view();
            if !position_in_textblock(&view, &position) && view.node(position.node).is_none() {
                return Err(CommandError::NodeNotFound(position.node));
            }
            return Ok(None);
        }
    };
    match plan {
        SliceInsertionPlan::DirectInline { fragments } => {
            let position = materialize_position_block(tr, position)?;
            let mode = build_inline_mode(tr, &position, provenance)?;
            insert_content_as_inline_at_position(tr, position, fragments, &mode)
        }
        SliceInsertionPlan::SpliceBlocks { candidate } => {
            let mut inserted = None;
            tr.batch::<_, CommandError>(|tr| {
                let position = materialize_position_block(tr, position)?;
                let mode = build_inline_mode(tr, &position, provenance)?;
                inserted = insert_blocks_in_textblock_at_position(tr, position, &candidate, &mode)?;
                Ok(())
            })?;
            Ok(inserted)
        }
        SliceInsertionPlan::OpenInline { fragments } => {
            let position = materialize_position_block(tr, position)?;
            let mode = build_inline_mode(tr, &position, provenance)?;
            insert_content_as_inline_at_position(tr, position, fragments, &mode)
        }
        SliceInsertionPlan::BlockBoundary { blocks } => {
            let position = materialize_position_block(tr, position)?;
            insert_blocks_at_block_boundary(tr, position, blocks)
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_macros::state;
    use editor_model::{Fragment, PlainNode, PlainParagraphNode, PlainTextNode};
    use editor_state::{Affinity, Position};

    use super::*;

    fn text_slice(text: &str) -> Slice {
        Slice {
            content: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: text.into(),
            }))],
            open_start: 0,
            open_end: 0,
        }
    }

    fn paragraph_slice(text: &str) -> Slice {
        Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: text.into(),
                }))],
            }],
            open_start: 0,
            open_end: 0,
        }
    }

    #[test]
    fn inline_text_into_paragraph_is_direct_inline() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let view = state.view();
        let position = Position::new(p1, 2);
        let plan = resolve_slice_insertion(&view, position, text_slice("x"));
        assert!(matches!(
            plan,
            Some(SliceInsertionPlan::DirectInline { .. })
        ));
    }

    #[test]
    fn empty_slice_is_none() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let view = state.view();
        let plan = resolve_slice_insertion(
            &view,
            Position::new(p1, 2),
            Slice {
                content: vec![],
                open_start: 0,
                open_end: 0,
            },
        );
        assert!(plan.is_none());
    }

    #[test]
    fn block_slice_at_root_boundary_is_block_boundary() {
        let (state, r, ..) = state! {
            doc { r: root { paragraph { text("a") } paragraph { text("b") } } }
            selection: none
        };
        let view = state.view();
        let position = Position {
            node: r,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let plan = resolve_slice_insertion(&view, position, paragraph_slice("x"));
        assert!(matches!(
            plan,
            Some(SliceInsertionPlan::BlockBoundary { .. })
        ));
    }

    #[test]
    fn inline_text_into_fold_title_is_direct_inline_or_none_consistent_with_schema() {
        let (state, t) = state! {
            doc { root {
                fold {
                    t: fold_title { text("title") }
                    fold_content { paragraph { text("c") } }
                }
                paragraph {}
            } }
            selection: (t, 1)
        };
        let view = state.view();
        let inline = resolve_slice_insertion(&view, Position::new(t, 1), text_slice("x"));
        let block = resolve_slice_insertion(&view, Position::new(t, 1), paragraph_slice("x"));
        assert!(inline.is_some());
        assert!(matches!(
            block,
            None | Some(SliceInsertionPlan::OpenInline { .. })
        ));
    }

    fn state_under_distinct_actor(source: &editor_state::State, actor: u64) -> editor_state::State {
        let plain = editor_state::to_plain(source.projected.projected());
        let (state, _) = editor_state::test_utils::build_state_from_plain_with_actor(plain, actor);
        state
    }

    #[test]
    fn missing_node_with_empty_slice_is_noop_not_error() {
        let (_foreign, f1) = state! {
            doc { root { f1: paragraph { text("zz") } } }
            selection: none
        };
        let (state_actor1, ..) = state! {
            doc { root { paragraph { text("a") } } }
            selection: none
        };
        let state = state_under_distinct_actor(&state_actor1, 2);

        let mut tr = editor_transaction::Transaction::new(&state);
        let result = insert_slice_at_position(
            &mut tr,
            Position::new(f1, 0),
            Slice {
                content: vec![],
                open_start: 0,
                open_end: 0,
            },
            crate::types::SliceProvenance::Plain,
        );
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn missing_node_with_content_slice_propagates_node_not_found() {
        let (_foreign, f1) = state! {
            doc { root { f1: paragraph { text("zz") } } }
            selection: none
        };
        let (state_actor1, ..) = state! {
            doc { root { paragraph { text("a") } } }
            selection: none
        };
        let state = state_under_distinct_actor(&state_actor1, 2);

        let mut tr = editor_transaction::Transaction::new(&state);
        let result = insert_slice_at_position(
            &mut tr,
            Position::new(f1, 0),
            paragraph_slice("x"),
            crate::types::SliceProvenance::Plain,
        );
        assert!(matches!(result, Err(crate::CommandError::NodeNotFound(_))));
    }
}
