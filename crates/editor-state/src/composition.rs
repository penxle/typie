use editor_model::NodeId;

#[derive(Clone, Debug, PartialEq)]
pub struct Composition {
    pub node_id: NodeId,
    pub offset: usize,
    pub text: String,
}
