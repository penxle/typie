use editor_model::{Doc, NodeId};

use crate::{Position, State};

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathPosition {
    path: Vec<usize>,
    offset: usize,
}

fn node_path(doc: &Doc, node_id: NodeId) -> Vec<usize> {
    let mut path = Vec::new();
    let mut current = node_id;
    while let Some(entry) = doc.get_entry(current) {
        if let Some(parent_id) = *entry.parent.get() {
            if let Some(parent_entry) = doc.get_entry(parent_id)
                && let Some(idx) = parent_entry
                    .children
                    .iter()
                    .copied()
                    .position(|id| id == current)
            {
                path.push(idx);
            }
            current = parent_id;
        } else {
            break;
        }
    }
    path.reverse();
    path
}

fn position_to_path_position(doc: &Doc, pos: &Position) -> PathPosition {
    PathPosition {
        path: node_path(doc, pos.node_id),
        offset: pos.offset,
    }
}

pub fn assert_state_eq_impl(actual: &State, expected: &State) {
    editor_model::assert_doc_eq_impl(&actual.doc, &expected.doc);

    match (&actual.selection, &expected.selection) {
        (None, None) => {}
        (Some(_), None) => panic!("Selection mismatch: actual has Some, expected has None"),
        (None, Some(_)) => panic!("Selection mismatch: actual has None, expected has Some"),
        (Some(sel1), Some(sel2)) => {
            let anchor1 = position_to_path_position(&actual.doc, &sel1.anchor);
            let anchor2 = position_to_path_position(&expected.doc, &sel2.anchor);
            assert_eq!(
                anchor1, anchor2,
                "Selection anchors differ: {:?} vs {:?}",
                anchor1, anchor2,
            );

            let head1 = position_to_path_position(&actual.doc, &sel1.head);
            let head2 = position_to_path_position(&expected.doc, &sel2.head);
            assert_eq!(
                head1, head2,
                "Selection heads differ: {:?} vs {:?}",
                head1, head2,
            );
        }
    }

    assert_eq!(
        actual.pending_modifiers, expected.pending_modifiers,
        "Pending modifiers differ",
    );

    assert_eq!(
        actual.pending_style, expected.pending_style,
        "Pending style ref differs",
    );
}

#[macro_export]
macro_rules! assert_state_eq {
    ($actual:expr, $expected:expr) => {
        $crate::assert_state_eq_impl(&$actual, &$expected)
    };
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    #[test]
    fn assert_state_eq_identical_states() {
        let (state1, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 5)
        };
        let (state2, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 5)
        };
        crate::assert_state_eq!(&state1, &state2);
    }

    #[test]
    #[should_panic(expected = "Node at index")]
    fn assert_state_eq_different_text() {
        let (state1, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (state2, ..) = state! {
            doc { root { paragraph { t1: text("World") } } }
            selection: (t1, 0)
        };
        crate::assert_state_eq!(&state1, &state2);
    }

    #[test]
    #[should_panic(expected = "Selection anchors differ")]
    fn assert_state_eq_different_selection() {
        let (state1, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (state2, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        crate::assert_state_eq!(&state1, &state2);
    }
}
