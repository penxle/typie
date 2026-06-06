use editor_model::{Modifier, ModifierType, NodeRef};

pub fn resolve_inherited<'a>(
    node: &NodeRef<'a>,
    modifier_type: ModifierType,
) -> Option<&'a Modifier> {
    node.modifiers_with_style()
        .find(|m| ModifierType::from(*m) == modifier_type)
        .or_else(|| {
            node.parent()
                .and_then(|p| resolve_inherited(&p, modifier_type))
        })
}

#[cfg(test)]
mod tests {
    use editor_macros::{doc, state};
    use editor_model::*;
    use editor_transaction::Transaction;

    use super::*;

    #[test]
    fn resolve_inherited_finds_on_self() {
        let (doc, p1) = doc! { root { p1: paragraph [block_gap(200)] } };

        let node = doc.node(p1).unwrap();
        let result = resolve_inherited(&node, ModifierType::BlockGap);

        assert!(matches!(result, Some(Modifier::BlockGap { value: 200 })));
    }

    #[test]
    fn resolve_inherited_walks_up_to_ancestor() {
        let (doc, t1) = doc! {
            root [block_gap(150)] {
                paragraph { t1: text("hi") }
            }
        };

        let node = doc.node(t1).unwrap();
        let result = resolve_inherited(&node, ModifierType::BlockGap);

        assert!(matches!(result, Some(Modifier::BlockGap { value: 150 })));
    }

    #[test]
    fn resolve_inherited_returns_none_when_absent() {
        let (doc, p1) = doc! { root [] { p1: paragraph } };

        let node = doc.node(p1).unwrap();
        let result = resolve_inherited(&node, ModifierType::BlockGap);

        assert!(result.is_none());
    }

    #[test]
    fn resolve_inherited_picks_up_style_modifier_on_run() {
        let (initial, _p1, t1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hi") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "h1".into(),
            Some(PlainStyleEntry {
                name: "Heading".into(),
                modifiers: vec![
                    Modifier::FontSize { value: 1800 },
                    Modifier::FontWeight { value: 700 },
                    Modifier::TextColor {
                        value: "#0000ff".into(),
                    },
                ]
                .into_iter()
                .collect(),
            }),
        )
        .unwrap();
        tr.set_node_style(t1, Some("h1".into())).unwrap();
        let (next, ..) = tr.commit();

        let text = next.doc.node(t1).unwrap();

        assert!(matches!(
            resolve_inherited(&text, ModifierType::FontSize),
            Some(Modifier::FontSize { value: 1800 })
        ));
        assert!(matches!(
            resolve_inherited(&text, ModifierType::FontWeight),
            Some(Modifier::FontWeight { value: 700 })
        ));
        assert!(matches!(
            resolve_inherited(&text, ModifierType::TextColor),
            Some(Modifier::TextColor { value }) if value == "#0000ff"
        ));
    }

    #[test]
    fn resolve_inherited_walks_up_to_base_style() {
        let (doc, t1) = doc! {
            styles { base: "기본" [block_gap(150)] }
            root @base [] { paragraph { t1: text("hi") } }
        };
        let node = doc.node(t1).unwrap();
        let result = resolve_inherited(&node, ModifierType::BlockGap);
        assert!(matches!(result, Some(Modifier::BlockGap { value: 150 })));
    }

    #[test]
    fn resolve_inherited_node_own_modifier_overrides_style() {
        let (initial, _p1, t1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hi") [font_size(1200)] } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "h1".into(),
            Some(PlainStyleEntry {
                name: "Heading".into(),
                modifiers: vec![Modifier::FontSize { value: 1800 }]
                    .into_iter()
                    .collect(),
            }),
        )
        .unwrap();
        tr.set_node_style(t1, Some("h1".into())).unwrap();
        let (next, ..) = tr.commit();

        let text = next.doc.node(t1).unwrap();

        assert!(matches!(
            resolve_inherited(&text, ModifierType::FontSize),
            Some(Modifier::FontSize { value: 1200 })
        ));
    }
}
