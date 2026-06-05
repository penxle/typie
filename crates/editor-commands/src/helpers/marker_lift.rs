use editor_model::{Doc, Expand, Modifier, Node, NodeId, NodeRef, Schema};
use editor_transaction::Transaction;

use crate::CommandError;

pub(crate) struct CapturedFirstTextMarker {
    paragraph_id: NodeId,
    had_text: bool,
    first_text_carryable: Vec<Modifier>,
    first_text_style: Option<String>,
}

pub(crate) fn capture_first_text_marker(
    doc: &Doc,
    paragraph_id: NodeId,
) -> Option<CapturedFirstTextMarker> {
    let paragraph = doc.node(paragraph_id)?;
    if !matches!(paragraph.node(), Node::Paragraph(_)) {
        return None;
    }
    let first_text = paragraph
        .children()
        .find(|c| matches!(c.node(), Node::Text(_)));
    let (had_text, first_text_carryable, first_text_style) = match first_text {
        Some(t) => (true, collect_carryable(&t), t.entry().style.get().clone()),
        None => (false, Vec::new(), None),
    };
    Some(CapturedFirstTextMarker {
        paragraph_id,
        had_text,
        first_text_carryable,
        first_text_style,
    })
}

pub(crate) fn apply_first_text_marker_lift(
    tr: &mut Transaction,
    captured: &CapturedFirstTextMarker,
) -> Result<(), CommandError> {
    if !captured.had_text
        || (captured.first_text_carryable.is_empty() && captured.first_text_style.is_none())
    {
        return Ok(());
    }
    let doc = tr.doc();
    let Some(paragraph) = doc.node(captured.paragraph_id) else {
        return Ok(());
    };
    if !matches!(paragraph.node(), Node::Paragraph(_)) {
        return Ok(());
    }
    let still_has_text = paragraph
        .children()
        .any(|c| matches!(c.node(), Node::Text(_)));
    if still_has_text {
        return Ok(());
    }
    for m in &captured.first_text_carryable {
        tr.add_modifier(captured.paragraph_id, m.clone())?;
    }
    if let Some(style_id) = &captured.first_text_style {
        tr.set_node_style(captured.paragraph_id, Some(style_id.clone()))?;
    }
    Ok(())
}

fn collect_carryable(text_node: &NodeRef) -> Vec<Modifier> {
    text_node
        .modifiers()
        .filter(|m| {
            matches!(
                Schema::modifier_spec(m.as_type()).expand,
                Expand::After | Expand::Both
            )
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_transaction::Transaction;

    use super::*;

    #[test]
    fn capture_returns_none_for_non_paragraph() {
        let (state, ft1) = state! {
            doc { root { fold { ft1: fold_title { text("T") } fold_content { paragraph {} } } } }
            selection: (ft1, 0)
        };
        assert!(capture_first_text_marker(&state.doc, ft1).is_none());
    }

    #[test]
    fn capture_returns_some_with_no_text_for_empty_paragraph() {
        let (state, p1) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let captured = capture_first_text_marker(&state.doc, p1).unwrap();
        assert!(!captured.had_text);
        assert!(captured.first_text_carryable.is_empty());
    }

    #[test]
    fn capture_extracts_carryable_from_first_text() {
        let (state, p1, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hi") [bold, font_weight(700)]
                        text("There") [italic]
                    }
                }
            }
            selection: (p1, 0)
        };
        let captured = capture_first_text_marker(&state.doc, p1).unwrap();
        assert!(captured.had_text);
        assert!(
            captured
                .first_text_carryable
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
        assert!(
            captured
                .first_text_carryable
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 }))
        );
        assert!(
            !captured
                .first_text_carryable
                .iter()
                .any(|m| matches!(m, Modifier::Italic))
        );
    }

    #[test]
    fn lift_no_op_when_text_remains() {
        let (state, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hi") [bold] } } }
            selection: (p1, 0)
        };
        let captured = capture_first_text_marker(&state.doc, p1).unwrap();
        let mut tr = Transaction::new(&state);
        apply_first_text_marker_lift(&mut tr, &captured).unwrap();
        let (new_state, _, _, _, _) = tr.commit();
        let p = new_state.doc.node(p1).unwrap();
        assert_eq!(p.modifiers().count(), 0);
    }

    #[test]
    fn lift_attaches_marker_after_text_removal() {
        let (state, p1, t1) = state! {
            doc { root { p1: paragraph { t1: text("Hi") [bold, font_weight(700)] } } }
            selection: (t1, 0)
        };
        let captured = capture_first_text_marker(&state.doc, p1).unwrap();
        let mut tr = Transaction::new(&state);
        tr.remove_subtree(t1).unwrap();
        apply_first_text_marker_lift(&mut tr, &captured).unwrap();
        let (new_state, _, _, _, _) = tr.commit();
        let p = new_state.doc.node(p1).unwrap();
        let mods: Vec<_> = p.modifiers().cloned().collect();
        assert!(mods.iter().any(|m| matches!(m, Modifier::Bold)));
        assert!(
            mods.iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 }))
        );
    }

    #[test]
    fn lift_attaches_style_marker_after_text_removal() {
        use editor_model::PlainStyleEntry;
        let (state, p1, t1) = state! {
            doc { root { p1: paragraph { t1: text("Hi") } } }
            selection: (t1, 0)
        };
        let mut setup = Transaction::new(&state);
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

        let captured = capture_first_text_marker(&state.doc, p1).unwrap();
        let mut tr = Transaction::new(&state);
        tr.remove_subtree(t1).unwrap();
        apply_first_text_marker_lift(&mut tr, &captured).unwrap();
        let (new_state, ..) = tr.commit();
        assert_eq!(
            new_state
                .doc
                .node(p1)
                .unwrap()
                .entry()
                .style
                .get()
                .as_deref(),
            Some("s1")
        );
    }
}
