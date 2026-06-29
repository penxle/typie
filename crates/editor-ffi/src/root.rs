pub(crate) fn attrs(view: &editor_model::DocView) -> Option<editor_model::PlainRootNode> {
    match view.node(editor_crdt::Dot::ROOT)?.node().to_plain() {
        editor_model::PlainNode::Root(root) => Some(root),
        _ => None,
    }
}

pub(crate) fn base_style_modifiers(state: &editor_state::State) -> Vec<editor_model::Modifier> {
    let Some(style_id) = state
        .projected
        .node_styles()
        .value_of(editor_crdt::Dot::ROOT)
    else {
        return Vec::new();
    };
    let styles = state.projected.styles();
    if !styles.registered(&style_id) {
        return Vec::new();
    }
    let Some(style) = styles.style_entry(&style_id) else {
        return Vec::new();
    };
    style.modifiers.iter().cloned().collect()
}
