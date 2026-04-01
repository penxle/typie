use editor_common::Rect;
use editor_model::NodeId;

#[derive(Debug, Clone)]
pub struct AtomFragment {
    pub node_id: NodeId,
    pub parent_id: NodeId,
    pub index: usize,
    pub rect: Rect,
}
