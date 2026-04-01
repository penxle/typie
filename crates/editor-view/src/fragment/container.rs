use editor_common::{EdgeInsets, Rect};
use editor_model::NodeId;

use super::Fragment;

#[derive(Debug, Clone, Copy, Default)]
pub struct Breaks {
    pub top: bool,
    pub bottom: bool,
}

#[derive(Debug, Clone)]
pub struct ContainerFragment {
    pub node_id: NodeId,
    pub rect: Rect,
    pub children: Vec<Fragment>,
    pub scope: bool,
    pub breaks: Breaks,
    pub border: EdgeInsets,
}
