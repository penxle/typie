pub(crate) fn attrs(view: &editor_model::DocView) -> Option<editor_model::PlainRootNode> {
    match view.node(editor_crdt::Dot::ROOT)?.node().to_plain() {
        editor_model::PlainNode::Root(root) => Some(root),
        _ => None,
    }
}

pub(crate) fn root_default_modifiers(state: &editor_state::State) -> Vec<editor_model::Modifier> {
    state
        .projected
        .block_modifiers()
        .modifiers_of(editor_crdt::Dot::ROOT)
        .into_values()
        .collect()
}
