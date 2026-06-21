pub(crate) fn attrs(doc: &editor_model::Doc) -> Option<editor_model::PlainRootNode> {
    let entry = doc.get_entry(editor_model::NodeId::ROOT)?;
    match &entry.node.to_plain() {
        editor_model::PlainNode::Root(root) => Some(root.clone()),
        _ => unreachable!("root entry must be Root"),
    }
}

pub(crate) fn base_style_modifiers(doc: &editor_model::Doc) -> Vec<editor_model::Modifier> {
    let Some(root) = doc.root() else {
        return Vec::new();
    };
    let Some(style_id) = root.entry().style.get().as_deref() else {
        return Vec::new();
    };
    if !doc.style_present(style_id) {
        return Vec::new();
    }
    let Some(style) = doc.style_entry(style_id) else {
        return Vec::new();
    };
    style.modifiers.iter().cloned().collect()
}
