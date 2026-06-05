use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::insert_tab_at_caret;

pub fn insert_tab(tr: &mut Transaction) -> CommandResult {
    insert_tab_at_caret(tr)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| insert_tab(&mut tr));
    }

    #[test]
    fn insert_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("He") tab t2: text("llo") } } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") tab } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_in_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { tab } } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_attaches_carryable_marker() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph [bold] { t1: text("Hello") [bold] tab } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn tab_node_carries_font_size_metric() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hi") [font_size(2400)] } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph [font_size(2400)] { t1: text("Hi") [font_size(2400)] tab [font_size(2400)] } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn tab_into_empty_paragraph_consumes_and_clears_marker_style() {
        use editor_model::PlainStyleEntry;
        use editor_resource::Resource;

        use crate::commands::delete_text_backward;
        let (state, p1, t1) = state! {
            doc { root { p1: paragraph { t1: text("Hi") } } }
            selection: (t1, 2)
        };
        let mut setup = editor_transaction::Transaction::new(&state);
        setup
            .set_style(
                "s1".into(),
                Some(PlainStyleEntry {
                    name: "s".into(),
                    modifiers: Default::default(),
                }),
            )
            .unwrap();
        setup.set_node_style(t1, Some("s1".into())).unwrap();
        let (state, ..) = setup.commit();

        // delete all text → marker lift puts "s1" on the empty paragraph
        let (e1, ..) = transact!(state, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (emptied, ..) = transact!(e1, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        assert_eq!(
            emptied.doc.node(p1).unwrap().entry().style.get().as_deref(),
            Some("s1"),
            "marker present while empty"
        );

        let (actual, ..) = transact!(emptied, |tr| insert_tab(&mut tr));
        let para = actual.doc.node(p1).unwrap();
        assert!(
            para.children()
                .any(|c| matches!(c.node(), editor_model::Node::Tab(_))
                    && c.entry().style.get().as_deref() == Some("s1")),
            "inserted tab gets marker style"
        );
        assert_eq!(
            para.entry().style.get().as_deref(),
            None,
            "marker cleared from paragraph"
        );
    }

    #[test]
    fn insert_tab_with_pending_style_stamps_style_on_new_tab() {
        use editor_model::PlainStyleEntry;
        use editor_state::PendingStyle;
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let mut setup = editor_transaction::Transaction::new(&initial);
        setup
            .set_style(
                "s1".into(),
                Some(PlainStyleEntry {
                    name: "s".into(),
                    modifiers: Default::default(),
                }),
            )
            .unwrap();
        setup
            .set_pending_style(Some(PendingStyle::Set {
                style_id: "s1".into(),
            }))
            .unwrap();
        let (with_pending, ..) = setup.commit();
        let (actual, ..) = transact!(with_pending, |tr| insert_tab(&mut tr));
        let para = actual.doc.root().unwrap().children().next().unwrap();
        let tab_styled = para.children().any(|c| {
            matches!(c.node(), editor_model::Node::Tab(_))
                && c.entry().style.get().as_deref() == Some("s1")
        });
        assert!(tab_styled, "inserted tab must carry pending style");
        assert!(actual.pending_style.is_none());
    }
}
