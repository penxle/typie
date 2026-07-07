pub(crate) fn build_default_doc(
    root: editor_model::PlainRootNode,
    modifiers: Vec<editor_model::Modifier>,
) -> editor_model::PlainDoc {
    let mut by_type = std::collections::BTreeMap::new();
    for m in modifiers {
        by_type.insert(m.as_type(), m);
    }

    let paragraph = editor_model::PlainNodeEntry {
        node: editor_model::PlainNode::Paragraph(editor_model::PlainParagraphNode {}),
        modifiers: Default::default(),
        carry: Vec::new(),
        children: vec![],
    };
    let root = editor_model::PlainNodeEntry {
        node: editor_model::PlainNode::Root(root),
        modifiers: by_type,
        carry: Vec::new(),
        children: vec![paragraph],
    };

    editor_model::PlainDoc { root }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_default_doc_seeds_root_modifiers() {
        let root = editor_model::PlainRootNode::default();
        let modifiers = vec![
            editor_model::Modifier::FontSize { value: 1200 },
            editor_model::Modifier::BlockGap { value: 100 },
        ];

        let plain = build_default_doc(root, modifiers);

        assert_eq!(
            plain
                .root
                .modifiers
                .get(&editor_model::ModifierType::FontSize),
            Some(&editor_model::Modifier::FontSize { value: 1200 })
        );
        assert_eq!(
            plain
                .root
                .modifiers
                .get(&editor_model::ModifierType::BlockGap),
            Some(&editor_model::Modifier::BlockGap { value: 100 })
        );
    }

    #[test]
    fn default_doc_resolves_root_defaults_at_block_level() {
        let plain = build_default_doc(
            editor_model::PlainRootNode::default(),
            vec![editor_model::Modifier::FontSize { value: 1400 }],
        );
        let state = editor_state::State::from_plain(&plain).unwrap();
        let view = state.view();
        let para = view
            .node(editor_crdt::Dot::ROOT)
            .unwrap()
            .child_blocks()
            .next()
            .unwrap();
        assert_eq!(
            para.effective().get(&editor_model::ModifierType::FontSize),
            Some(&editor_model::Modifier::FontSize { value: 1400 })
        );
    }
}
