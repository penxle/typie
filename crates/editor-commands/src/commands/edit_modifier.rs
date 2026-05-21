use editor_model::{Modifier, ModifierType};
use editor_state::{Position, resolve_modifier_span_at};
use editor_transaction::Transaction;

use crate::helpers::{
    apply_modifier_to_node, collect_text_nodes_in_range, compact_and_restore_selection,
    compact_textblock_preserving_caret, filter_applicable_node_ids, is_text_applicable,
};
use crate::{CommandError, CommandResult};

pub fn edit_modifier(
    tr: &mut Transaction,
    modifier_type: ModifierType,
    modifier: Option<Modifier>,
) -> CommandResult {
    if let Some(m) = &modifier {
        if m.as_type() != modifier_type {
            return Err(CommandError::InvalidArgument(format!(
                "modifier type mismatch: op type {:?}, modifier {:?}",
                modifier_type,
                m.as_type()
            )));
        }
    }
    if !is_text_applicable(modifier_type) {
        return Err(CommandError::InvalidArgument(format!(
            "edit_modifier is only valid for text-applicable modifiers; got {:?}",
            modifier_type
        )));
    }

    let collapsed = tr.selection().is_collapsed();
    if collapsed {
        edit_modifier_collapsed(tr, modifier_type, modifier)
    } else {
        edit_modifier_range(tr, modifier_type, modifier)
    }
}

fn edit_modifier_collapsed(
    tr: &mut Transaction,
    modifier_type: ModifierType,
    modifier: Option<Modifier>,
) -> CommandResult {
    let pos: Position = tr.selection().head;
    let Some(span_ids) = resolve_modifier_span_at(tr.state(), &pos, modifier_type) else {
        return Ok(false);
    };

    for &node_id in &span_ids {
        let doc = tr.doc();
        let node = doc
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        match &modifier {
            Some(m) => {
                apply_modifier_to_node(tr, &node, m)?;
            }
            None => {
                let existing = node
                    .explicit_modifiers()
                    .find(|m| m.as_type() == modifier_type)
                    .cloned();
                if let Some(existing) = existing {
                    tr.remove_modifier(node_id, existing)?;
                }
            }
        }
    }

    compact_textblock_preserving_caret(tr, pos)?;
    Ok(true)
}

fn edit_modifier_range(
    tr: &mut Transaction,
    modifier_type: ModifierType,
    modifier: Option<Modifier>,
) -> CommandResult {
    let selection = tr.selection();
    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());

    let node_ids = collect_text_nodes_in_range(tr, &from, &to)?;
    let applicable_node_ids = filter_applicable_node_ids(&tr.doc(), &node_ids, modifier_type);

    if applicable_node_ids.is_empty() {
        return Ok(false);
    }

    for &node_id in &applicable_node_ids {
        let doc = tr.doc();
        let node = doc
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        match &modifier {
            Some(m) => apply_modifier_to_node(tr, &node, m)?,
            None => {
                let existing = node
                    .explicit_modifiers()
                    .find(|m| m.as_type() == modifier_type)
                    .cloned();
                if let Some(existing) = existing {
                    tr.remove_modifier(node_id, existing)?;
                }
            }
        }
    }

    compact_and_restore_selection(tr, &node_ids)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_inside_link_set_updates_span_value() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://a.com".to_string())] } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://b.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://b.com".to_string())] } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_inside_link_remove_clears_span() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://a.com".to_string())] } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            None,
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Click") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_link_inserts_on_plain_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://a.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [link(href: "https://a.com".to_string())]
            } } }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_link_replaces_existing_value() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [link(href: "https://a.com".to_string())]
            } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://b.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [link(href: "https://b.com".to_string())]
            } } }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_remove_link_clears() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [link(href: "https://a.com".to_string())]
            } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            None,
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_outside_link_set_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("plain") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact_fail!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://a.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("plain") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_text_modifier_rejected() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let err = transact_err!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::LineHeight,
            Some(Modifier::LineHeight { value: 200 })
        ));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn value_modifier_type_mismatch_rejected() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let err = transact_err!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Ruby {
                text: "x".to_string()
            })
        ));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn collapsed_adjacent_different_href_isolates() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [link(href: "https://a.com".to_string())]
                t2: text("World") [link(href: "https://b.com".to_string())]
            } } }
            selection: (t2, 2)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://c.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { paragraph {
                text("Hello") [link(href: "https://a.com".to_string())]
                t2: text("World") [link(href: "https://c.com".to_string())]
            } } }
            selection: (t2, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_paragraph_boundary_not_crossed() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("a") [link(href: "https://a.com".to_string())] }
                paragraph { t2: text("b") [link(href: "https://a.com".to_string())] }
            } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://c.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("a") [link(href: "https://c.com".to_string())] }
                paragraph { text("b") [link(href: "https://a.com".to_string())] }
            } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_inside_ruby_set_updates_span_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Han") [ruby(text: "한".to_string())] } } }
            selection: (t1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Ruby,
            Some(Modifier::Ruby {
                text: "韓".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Han") [ruby(text: "韓".to_string())] } } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn mixed_range_set_overwrites_all() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [link(href: "https://a.com".to_string())]
                t2: text("World") [link(href: "https://b.com".to_string())]
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://c.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { paragraph {
                t1: text("HelloWorld") [link(href: "https://c.com".to_string())]
            } } }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }
}
