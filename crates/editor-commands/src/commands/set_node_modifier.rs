use editor_crdt::Dot;
use editor_model::Modifier;
use editor_transaction::Transaction;

use crate::helpers::{
    apply_modifier_to_node, is_table_justify, is_unit_variant, matches_modifier_context,
};
use crate::{CommandError, CommandResult};

pub fn set_node_modifier(tr: &mut Transaction, id: Dot, modifier: Modifier) -> CommandResult {
    if is_unit_variant(&modifier) {
        return Err(CommandError::InvalidArgument(format!(
            "{:?} is a unit modifier, use toggle_modifier instead",
            modifier.as_type()
        )));
    }
    if !modifier.is_valid() {
        return Ok(false);
    }

    let skip = {
        let view = tr.view();
        !matches_modifier_context(&view, id, modifier.as_type())
            || is_table_justify(&view, id, &modifier)
    };
    if skip {
        return Ok(false);
    }

    apply_modifier_to_node(tr, id, &modifier)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn sets_font_size_on_root_as_document_default() {
        let (initial, r, ..) = state! {
            doc {
                r: root [font_size(1600)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_node_modifier(
            &mut tr,
            r,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(2400)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn missing_node_id_errors() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let err = transact_err!(initial, |tr| set_node_modifier(
            &mut tr,
            Dot::new(u64::MAX, 1),
            Modifier::FontSize { value: 2400 },
        ));
        assert!(matches!(err, CommandError::NodeNotFound(_)));
    }

    #[test]
    fn rejects_unit_modifier() {
        let (initial, r, ..) = state! {
            doc { r: root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let err = transact_err!(initial, |tr| set_node_modifier(&mut tr, r, Modifier::Bold,));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn rejects_out_of_range_value_as_noop() {
        let (initial, r, ..) = state! {
            doc { r: root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| set_node_modifier(
            &mut tr,
            r,
            Modifier::FontSize { value: 399 }
        ));
    }

    #[test]
    fn rejects_context_mismatched_target_as_noop() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| set_node_modifier(
            &mut tr,
            p1,
            Modifier::BlockGap { value: 100 }
        ));
    }

    #[test]
    fn rejects_table_justify_as_noop() {
        use editor_model::Alignment;
        let (initial, table, ..) = state! {
            doc { root {
                table: table {
                    table_row {
                        table_cell { paragraph { text("x") } }
                    }
                }
            } }
            selection: (table, 0)
        };
        transact_fail!(initial, |tr| set_node_modifier(
            &mut tr,
            table,
            Modifier::Alignment {
                value: Alignment::Justify
            }
        ));
    }

    #[test]
    fn sets_line_height_on_root_reaches_all_paragraphs_without_own_record() {
        use editor_model::ModifierType;
        let (initial, r, p1, p2) = state! {
            doc { r: root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("World") }
            } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_node_modifier(
            &mut tr,
            r,
            Modifier::LineHeight { value: 200 }
        ));
        let view = actual.view();
        for p in [p1, p2] {
            assert_eq!(
                view.node(p)
                    .unwrap()
                    .effective()
                    .get(&ModifierType::LineHeight),
                Some(&Modifier::LineHeight { value: 200 }),
                "a single root edit reaches every paragraph lacking its own record"
            );
        }
    }

    #[test]
    fn sets_block_gap_on_root_as_document_default() {
        use editor_model::ModifierType;
        let (initial, r, ..) = state! {
            doc { r: root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_node_modifier(
            &mut tr,
            r,
            Modifier::BlockGap { value: 150 }
        ));
        assert_eq!(
            actual
                .view()
                .node(r)
                .unwrap()
                .block_modifier(ModifierType::BlockGap),
            Some(&Modifier::BlockGap { value: 150 }),
            "BlockGap is root-only with no selection target, but SetOnNode(ROOT) still records it via the context check (document-settings path)"
        );
    }

    #[test]
    fn sets_paragraph_indent_and_alignment_on_root_as_document_defaults() {
        use editor_model::{Alignment, ModifierType};
        let (initial, r, ..) = state! {
            doc { r: root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_node_modifier(
            &mut tr,
            r,
            Modifier::ParagraphIndent { value: 200 }
        ));
        assert_eq!(
            actual
                .view()
                .node(r)
                .unwrap()
                .block_modifier(ModifierType::ParagraphIndent),
            Some(&Modifier::ParagraphIndent { value: 200 })
        );
        let (actual2, ..) = transact!(actual, |tr| set_node_modifier(
            &mut tr,
            r,
            Modifier::Alignment {
                value: Alignment::Center
            }
        ));
        assert_eq!(
            actual2
                .view()
                .node(r)
                .unwrap()
                .block_modifier(ModifierType::Alignment),
            Some(&Modifier::Alignment {
                value: Alignment::Center
            }),
            "Root is a legal Alignment placement (context), so the document-default alignment records on ROOT"
        );
    }
}
