pub(crate) const BASE_STYLE_ID: &str = "base";
pub(crate) const BASE_STYLE_NAME: &str = "기본";

pub(crate) fn build_default_doc(
    root: editor_model::PlainRootNode,
    modifiers: Vec<editor_model::Modifier>,
) -> editor_model::PlainDoc {
    let paragraph_id = editor_model::NodeId::new();

    // type당 1개로 정규화. BTreeSet은 완전 동일 값만 dedupe하고 같은 type 다른 값은 둘 다 남기므로 ModifierType 기준으로 정규화.
    let mut by_type = std::collections::BTreeMap::new();
    for m in modifiers {
        by_type.insert(m.as_type(), m);
    }

    let mut styles = std::collections::BTreeMap::new();
    styles.insert(
        BASE_STYLE_ID.to_string(),
        editor_model::PlainStyleEntry {
            name: BASE_STYLE_NAME.to_string(),
            modifiers: by_type.into_values().collect(),
        },
    );

    let mut nodes = std::collections::BTreeMap::new();
    nodes.insert(
        editor_model::NodeId::ROOT,
        editor_model::PlainNodeEntry {
            parent: None,
            children: vec![paragraph_id],
            modifiers: Default::default(),
            style: Some(BASE_STYLE_ID.to_string()),
            node: editor_model::PlainNode::Root(root),
        },
    );
    nodes.insert(
        paragraph_id,
        editor_model::PlainNodeEntry {
            parent: Some(editor_model::NodeId::ROOT),
            children: vec![],
            modifiers: Default::default(),
            style: None,
            node: editor_model::PlainNode::Paragraph(editor_model::PlainParagraphNode {}),
        },
    );

    editor_model::PlainDoc { nodes, styles }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_default_doc_seeds_base_style() {
        let root = editor_model::PlainRootNode::default();
        let modifiers = vec![
            editor_model::Modifier::FontSize { value: 1200 },
            editor_model::Modifier::BlockGap { value: 100 },
        ];

        let plain = build_default_doc(root, modifiers);

        let base = plain.styles.get("base").expect("base style must exist");
        assert!(
            base.modifiers
                .contains(&editor_model::Modifier::FontSize { value: 1200 })
        );
        assert!(
            base.modifiers
                .contains(&editor_model::Modifier::BlockGap { value: 100 })
        );

        let root_entry = plain.nodes.get(&editor_model::NodeId::ROOT).unwrap();
        assert!(
            root_entry.modifiers.is_empty(),
            "root must not carry modifiers"
        );
        assert_eq!(root_entry.style.as_deref(), Some("base"));
        assert_eq!(root_entry.children.len(), 1);
    }
}
