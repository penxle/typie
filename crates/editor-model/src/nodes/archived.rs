use editor_crdt::LwwReg;
use editor_macros::NodeAttr;

#[derive(Debug, Clone, PartialEq, Eq, NodeAttr)]
pub struct ArchivedNode {
    pub id: LwwReg<Option<String>>,
}
