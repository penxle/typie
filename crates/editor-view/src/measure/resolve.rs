use editor_model::{Modifier, ModifierType, NodeRef};

pub fn resolve_inherited<'a>(
    node: &NodeRef<'a>,
    modifier_type: ModifierType,
) -> Option<&'a Modifier> {
    node.modifiers()
        .iter()
        .find(|m| ModifierType::from(*m) == modifier_type)
        .or_else(|| {
            node.parent()
                .and_then(|p| resolve_inherited(&p, modifier_type))
        })
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::*;

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
}
