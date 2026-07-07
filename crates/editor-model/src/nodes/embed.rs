use editor_crdt::LwwReg;
use editor_macros::NodeAttr;

#[derive(Debug, Clone, PartialEq, Eq, NodeAttr)]
pub struct EmbedNode {
    pub id: LwwReg<Option<String>>,
}
