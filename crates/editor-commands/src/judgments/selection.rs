use editor_model::DocView;
use editor_resource::Resource;
use editor_state::{
    Selection, document_content_selection, resolve_paragraph_selection_expansion,
    resolve_sentence_selection_expansion, resolve_word_selection_expansion,
};

use crate::types::Verdict;

/// `Change(sel)` ⇒ 실행 시 선택이 `sel`로 바뀌는 관측 가능한 변화 보장.
/// `AbsorbOnly` ⇒ 해석 결과가 현재 선택과 동일 — 실행은 no-op.
pub fn judge_expand_word(
    view: &DocView,
    selection: Option<Selection>,
    resource: &Resource,
) -> Verdict<Selection> {
    let Some(sel) = selection else {
        return Verdict::NotApplicable;
    };
    verdict_from(
        resolve_word_selection_expansion(&sel, view, resource),
        selection,
    )
}

/// `Change(sel)` ⇒ 실행 시 선택이 `sel`로 바뀌는 관측 가능한 변화 보장.
/// `AbsorbOnly` ⇒ 해석 결과가 현재 선택과 동일 — 실행은 no-op.
pub fn judge_expand_sentence(
    view: &DocView,
    selection: Option<Selection>,
    resource: &Resource,
) -> Verdict<Selection> {
    let Some(sel) = selection else {
        return Verdict::NotApplicable;
    };
    verdict_from(
        resolve_sentence_selection_expansion(&sel, view, resource),
        selection,
    )
}

/// `Change(sel)` ⇒ 실행 시 선택이 `sel`로 바뀌는 관측 가능한 변화 보장.
/// `AbsorbOnly` ⇒ 해석 결과가 현재 선택과 동일 — 실행은 no-op.
pub fn judge_expand_paragraph(view: &DocView, selection: Option<Selection>) -> Verdict<Selection> {
    let Some(sel) = selection else {
        return Verdict::NotApplicable;
    };
    verdict_from(resolve_paragraph_selection_expansion(&sel, view), selection)
}

/// `Change(sel)` ⇒ 실행 시 선택이 `sel`로 바뀌는 관측 가능한 변화 보장.
/// `AbsorbOnly` ⇒ 해석 결과가 현재(`current`) 선택과 동일 — 실행은 no-op.
pub fn judge_expand_all(view: &DocView, current: Option<Selection>) -> Verdict<Selection> {
    verdict_from(
        document_content_selection(view).and_then(|sel| sel.normalize(view)),
        current,
    )
}

fn verdict_from(resolved: Option<Selection>, current: Option<Selection>) -> Verdict<Selection> {
    match resolved {
        None => Verdict::NotApplicable,
        Some(resolved) if current == Some(resolved) => Verdict::AbsorbOnly,
        Some(resolved) => Verdict::Change(resolved),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_resource::Resource;

    use super::*;
    use crate::types::Verdict;

    #[test]
    fn word_expansion_from_caret_is_change() {
        let resource = Resource::new_test();
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 2)
        };
        let view = state.view();
        assert!(matches!(
            judge_expand_word(&view, state.selection, &resource),
            Verdict::Change(_)
        ));
    }

    #[test]
    fn word_expansion_when_word_already_selected_is_absorb_only() {
        let resource = Resource::new_test();
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 2)
        };
        let view = state.view();
        let Verdict::Change(expanded) = judge_expand_word(&view, state.selection, &resource) else {
            panic!("expected Change");
        };
        assert!(matches!(
            judge_expand_word(&view, Some(expanded), &resource),
            Verdict::AbsorbOnly
        ));
    }

    #[test]
    fn sentence_expansion_from_caret_is_change() {
        let resource = Resource::new_test();
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("First sentence. Second sentence.") } } }
            selection: (p1, 3)
        };
        let view = state.view();
        assert!(matches!(
            judge_expand_sentence(&view, state.selection, &resource),
            Verdict::Change(_)
        ));
    }

    #[test]
    fn non_all_units_without_selection_are_not_applicable() {
        let resource = Resource::new_test();
        let (state, ..) = state! {
            doc { root { paragraph { text("hi") } } }
            selection: none
        };
        let view = state.view();
        assert!(matches!(
            judge_expand_word(&view, None, &resource),
            Verdict::NotApplicable
        ));
        assert!(matches!(
            judge_expand_sentence(&view, None, &resource),
            Verdict::NotApplicable
        ));
        assert!(matches!(
            judge_expand_paragraph(&view, None),
            Verdict::NotApplicable
        ));
    }

    #[test]
    fn all_without_selection_is_change() {
        let (state, ..) = state! {
            doc { root { paragraph { text("hi") } } }
            selection: none
        };
        let view = state.view();
        assert!(matches!(judge_expand_all(&view, None), Verdict::Change(_)));
    }

    #[test]
    fn all_when_canonical_selection_current_is_absorb_only() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        let Verdict::Change(all_sel) = judge_expand_all(&view, state.selection) else {
            panic!("expected Change");
        };
        assert!(matches!(
            judge_expand_all(&view, Some(all_sel)),
            Verdict::AbsorbOnly
        ));
    }
}
